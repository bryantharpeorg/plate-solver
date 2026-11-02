#![allow(dead_code)]
//! Oracles: rendered catalog stars, simulator mechanics and true eyepiece,
//! sampled every 0.1 s in paused time; angular acceptance is strictly 60 arcseconds.
#[path = "live/support.rs"]
mod support;
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use finder::{
    api::AppState,
    ephemeris,
    link::Transport,
    live::{SolveFuture, Solver},
    samples::Samples,
    session::{Aim, Session},
    sky,
    solve::{self, SolveOptions},
};
use scope_link::{memory, proto};
use scope_node::{
    clock::NodeClock,
    mount::{Axis, AxisInfo, AxisState, Mount, MountFuture},
    service::{NodeControls, NodeService},
};
use serde_json::{json, Value};
use simulator::{
    mount::{MountConfig, SimMount},
    SimCamera, Simulator, VirtualClock,
};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use support::{ConsoleClock, DB, START};
use tower::ServiceExt;

// Real rendered stars and the real solver, kept on the paused runtime so a
// blocking worker cannot let virtual time race ahead of the exposure.
struct Stars;
impl Solver for Stars {
    fn solve(&self, frame: proto::StarFrame, options: SolveOptions) -> SolveFuture<'_> {
        Box::pin(async move {
            solve::solve_centroids(
                &DB,
                &frame.centroids,
                (frame.width as usize, frame.height as usize),
                &options,
            )
        })
    }
}
struct NodeTime(VirtualClock);
impl NodeClock for NodeTime {
    fn now(&self) -> f64 {
        self.0.node_now()
    }
}
#[derive(Clone)]
struct Recorded {
    mount: SimMount,
    clock: VirtualClock,
    stops: Arc<Mutex<Vec<f64>>>,
}
impl Mount for Recorded {
    fn axes(&self) -> [AxisInfo; 2] {
        self.mount.axes()
    }
    fn state(&self, axis: Axis) -> MountFuture<'_, AxisState> {
        self.mount.state(axis)
    }
    fn goto(&self, axis: Axis, position: i32) -> MountFuture<'_, ()> {
        self.mount.goto(axis, position)
    }
    fn rate(&self, axis: Axis, rate: f64) -> MountFuture<'_, ()> {
        self.mount.rate(axis, rate)
    }
    fn stop_all(&self) -> MountFuture<'_, ()> {
        Box::pin(async move {
            self.stops.lock().unwrap().push(self.clock.now());
            self.mount.stop_all().await;
        })
    }
}
// Independent horizon transform from the simulator, not the GoTo guard.
pub(crate) fn sun_truth(lat: f64, lon: f64, time: f64) -> (f64, f64) {
    let jd = sky::julian_date(time);
    let (ra, dec) = ephemeris::sun(jd);
    let date = math_core::UnitVector::from_radec(ra.to_radians(), dec.to_radians());
    simulator::sky::horizontal(
        simulator::sky::to_j2000(date, jd),
        simulator::Site {
            lat_deg: lat,
            lon_deg: lon,
        },
        time,
    )
}
pub(crate) struct Rig {
    pub(crate) sim: Simulator,
    pub(crate) mount: SimMount,
    pub(crate) state: AppState,
    pub(crate) app: Router,
    pub(crate) stops: Arc<Mutex<Vec<f64>>>,
    pub(crate) controls: NodeControls,
}
impl Rig {
    pub(crate) async fn new(start: f64, alt: f64, az: f64, connected: bool) -> Self {
        let clock = VirtualClock::new(start);
        let sim = Simulator::new(DB.clone(), simulator::Site::default(), clock.clone());
        let mount = SimMount::with_config(
            clock.clone(),
            MountConfig {
                base_tilt_deg: 1.0,
                base_tilt_toward_deg: 130.0,
                zero_points_deg: [az, alt],
                backlash_deg: 0.3,
            },
        );
        if connected {
            sim.connect_mount(mount.clone());
        }
        let stops = Arc::new(Mutex::new(Vec::new()));
        let recorded = Recorded {
            mount: mount.clone(),
            clock: clock.clone(),
            stops: stops.clone(),
        };
        let node = NodeService::with_controls_and_mount(
            SimCamera::new(sim.clone()),
            Arc::new(NodeTime(clock.clone())),
            "test-token",
            Default::default(),
            NodeControls::default(),
            connected.then(|| Arc::new(recorded) as Arc<dyn Mount>),
        )
        .unwrap();
        let controls = node.controls();
        controls.set_frame_interval(Duration::from_millis(500));
        let transport = Transport::connector(move || {
            let node = node.clone();
            async move {
                let (client, server) = tokio::io::duplex(64 * 1024);
                tokio::spawn(async move {
                    memory::serve_routes_on(server, node.routes())
                        .await
                        .unwrap();
                });
                Ok::<_, std::io::Error>(client)
            }
        });
        let session = Session::default();
        session.edit(serde_json::from_value(json!({"node":{"address":"scope.local:1","token":"test-token"},"site":{"lat_deg":40,"lon_deg":-75},"target":"M42","camera_settings":{"exposure_ms":200,"gain":1}})).unwrap(), start).unwrap();
        session
            .set_aim(Some(Aim { x: 700.0, y: 420.0 }), start)
            .unwrap();
        let state = AppState::shared(DB.clone(), Samples::default())
            .with_session(session)
            .with_clock(Arc::new(ConsoleClock(clock)))
            .with_link(transport)
            .with_live_solver(Arc::new(Stars));
        let app = finder::router(state.clone());
        let rig = Self {
            sim,
            mount,
            state,
            app,
            stops,
            controls,
        };
        for _ in 0..100 {
            if rig.state.link().unwrap().status().borrow().connected {
                break;
            }
            rig.tick().await;
        }
        assert!(rig.state.link().unwrap().status().borrow().connected);
        rig
    }
    pub(crate) fn now(&self) -> f64 {
        self.sim.clock().now()
    }
    pub(crate) async fn tick(&self) {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    pub(crate) async fn call(&self, path: &str, foreign: bool) -> (StatusCode, Value) {
        let mut request = Request::post(path).header("host", "localhost");
        if foreign {
            request = request.header("origin", "http://foreign.invalid");
        }
        let response = self
            .app
            .clone()
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        (
            status,
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap(),
        )
    }
    pub(crate) async fn view(&self) -> Value {
        let response = self
            .app
            .clone()
            .oneshot(Request::get("/api/session").body(Body::empty()).unwrap())
            .await
            .unwrap();
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap()
    }
    pub(crate) async fn sync(&self) {
        for _ in 0..100 {
            if !self.state.session().model(self.now()).syncs.is_empty() {
                return;
            }
            self.tick().await;
        }
        panic!("no solved sync: {}", self.view().await);
    }
    pub(crate) async fn stationary(&self) -> bool {
        !self.mount.state(Axis::Azimuth).await.moving
            && !self.mount.state(Axis::Altitude).await.moving
    }
    pub(crate) async fn move_to(&self, axis: [i32; 2]) {
        let mut client = self.state.link().unwrap().mount().unwrap();
        for (i, a) in [proto::Axis::Azimuth, proto::Axis::Altitude]
            .into_iter()
            .enumerate()
        {
            client.goto_axis(a, axis[i]).await.unwrap();
        }
        for _ in 0..1600 {
            self.tick().await;
            if self.stationary().await {
                return;
            }
        }
        panic!("mount did not settle");
    }
    pub(crate) async fn second(&self) {
        // Both syncs use the same backlash flank. The two physical directions
        // are >90° apart; the simulator, rather than the fitted model, is truth.
        self.move_to([4500, 4500]).await;
        for _ in 0..15 {
            self.tick().await;
        }
        self.move_to([580_000, 4500]).await;
        for _ in 0..15 {
            self.tick().await;
        }
        let model = self.state.session().model(self.now());
        assert!(model.base_tilt.is_some(), "{:?}", model.view());
        assert_eq!(model.epochs.len(), 1);
    }
    pub(crate) fn target(&self, name: &str) {
        self.state
            .session()
            .edit(
                serde_json::from_value(json!({"target": name})).unwrap(),
                self.now(),
            )
            .unwrap();
    }
    pub(crate) fn site_for_altitude(&self, name: &str, alt: f64) {
        let target = finder::targets::find(name).unwrap();
        let (ra, dec) =
            sky::precess_from_j2000(target.ra_deg, target.dec_deg, sky::julian_date(self.now()));
        // On the meridian, h = 90 - |latitude - declination| (Meeus 13).
        let lat = dec + 90.0 - alt;
        let lon =
            (ra - sky::gmst_deg(sky::julian_date(self.now())) + 180.0).rem_euclid(360.0) - 180.0;
        self.state
            .session()
            .edit(
                serde_json::from_value(json!({"site":{"lat_deg":lat,"lon_deg":lon},"target":name}))
                    .unwrap(),
                self.now(),
            )
            .unwrap();
    }
    pub(crate) async fn finish(&self, min_alt: Option<f64>, sun_distance: bool) -> Vec<Value> {
        self.finish_with(min_alt, sun_distance, |_| {}).await
    }
    pub(crate) async fn finish_with(
        &self,
        min_alt: Option<f64>,
        sun_distance: bool,
        mut sample: impl FnMut(&Value),
    ) -> Vec<Value> {
        let mut progress = Vec::new();
        for _ in 0..5000 {
            let view = self.view().await;
            let value = view["goto"].clone();
            if progress.last() != Some(&value) {
                progress.push(value.clone());
            }
            sample(&value);
            let truth = self.mount.true_direction();
            if let Some(min) = min_alt {
                assert!(truth.alt >= min, "altitude {} < {}", truth.alt, min);
            }
            if sun_distance {
                let sun = sun_truth(
                    view["site"]["lat_deg"].as_f64().unwrap(),
                    view["site"]["lon_deg"].as_f64().unwrap(),
                    self.now(),
                );
                assert!(finder::model::separation(truth.az, truth.alt, sun.1, sun.0) >= 15.0);
            }
            if matches!(value["state"].as_str(), Some("done" | "failed" | "halted")) {
                assert!(self.stationary().await);
                return progress;
            }
            self.tick().await;
        }
        panic!("GoTo did not end: {}", self.view().await);
    }
    pub(crate) fn error_arcsec(&self, name: &str) -> f64 {
        let target = finder::targets::find(name).unwrap();
        let truth = self.sim.telescope().eyepiece;
        finder::model::separation(truth.ra, truth.dec, target.ra_deg, target.dec_deg) * 3600.0
    }
}

// Every negative motion assertion first proves the connected mount can move.
async fn accepted_control(rig: &Rig) {
    rig.sync().await;
    assert!(rig.view().await["mount"]["connected"].as_bool().unwrap());
    assert_eq!(rig.call("/api/goto", false).await.0, 200);
    for _ in 0..100 {
        rig.tick().await;
        if !rig.stationary().await {
            break;
        }
    }
    assert!(!rig.stationary().await);
    assert_eq!(rig.call("/api/stop", false).await.0, 200);
    for _ in 0..15 {
        rig.tick().await;
    }
    assert!(rig.stationary().await);
}
async fn refused(rig: &Rig, status: u16, reason: &str, foreign: bool) {
    let before = [
        rig.mount.state(Axis::Azimuth).await.position,
        rig.mount.state(Axis::Altitude).await.position,
    ];
    let (answer, body) = rig.call("/api/goto", foreign).await;
    assert_eq!(answer, status, "{body}");
    assert!(body["error"].as_str().unwrap().contains(reason), "{body}");
    for _ in 0..15 {
        rig.tick().await;
        assert!(rig.stationary().await);
    }
    assert_eq!(
        [
            rig.mount.state(Axis::Azimuth).await.position,
            rig.mount.state(Axis::Altitude).await.position
        ],
        before
    );
}
#[tokio::test(start_paused = true)]
async fn approach_to_fifteen_point_two_degrees_is_refused_without_motion() {
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    accepted_control(&rig).await;
    rig.site_for_altitude("M42", 15.2);
    for _ in 0..15 {
        rig.tick().await;
    }
    refused(
        &rig,
        409,
        "approach would go below the minimum altitude",
        false,
    )
    .await;
}
#[tokio::test(start_paused = true)]
async fn sun_five_degrees_up_is_refused_without_motion() {
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    accepted_control(&rig).await;
    let (ra, dec) = ephemeris::sun(sky::julian_date(rig.now()));
    let lon = (ra - sky::gmst_deg(sky::julian_date(rig.now())) + 180.0).rem_euclid(360.0) - 180.0;
    rig.state
        .session()
        .edit(
            serde_json::from_value(json!({"site":{"lat_deg":dec+85.0,"lon_deg":lon}})).unwrap(),
            rig.now(),
        )
        .unwrap();
    assert!((sun_truth(dec + 85.0, lon, rig.now()).0 - 5.0).abs() < 1e-6);
    refused(&rig, 409, "Sun is up", false).await;
}
#[tokio::test(start_paused = true)]
async fn m8_below_horizon_is_refused_without_motion() {
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    accepted_control(&rig).await;
    rig.target("M8");
    refused(&rig, 409, "M8 is below the horizon", false).await;
}
#[tokio::test(start_paused = true)]
async fn absent_mount_and_missing_sync_each_name_the_reason() {
    let rig = Rig::new(START, 50.0, 120.0, false).await;
    refused(&rig, 409, "no mount", false).await;
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    accepted_control(&rig).await;
    rig.controls.stop_frames();
    for _ in 0..5 {
        rig.tick().await;
    }
    rig.state.session().reset_model(false, rig.now()).unwrap();
    assert!(rig.state.session().model(rig.now()).syncs.is_empty());
    refused(&rig, 409, "no sync", false).await;
}
#[tokio::test(start_paused = true)]
async fn foreign_goto_is_forbidden_without_motion() {
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    accepted_control(&rig).await;
    refused(&rig, 403, "origin", true).await;
}

#[tokio::test(start_paused = true)]
async fn simulate_startup_connects_the_default_mount_and_serves_motion_and_stop() {
    let state = AppState::shared(DB.clone(), Samples::default())
        .with_simulation(START, finder::simulate::SimulationTransport::Memory)
        .await
        .unwrap();
    let sim = state.simulator().unwrap();
    let mount = sim.mount().expect("simulate mode must connect a mount");
    // The startup wiring uses US1's default mechanics, not a special test mount.
    assert_eq!(mount.axes()[0].counts_per_revolution, 1_620_000);
    sim.push_to(50.0, 120.0);
    state
        .session()
        .edit(
            serde_json::from_value(json!({
                "site":{"lat_deg":40,"lon_deg":-75}, "target":"M42",
                "camera_settings":{"exposure_ms":200,"gain":1}
            }))
            .unwrap(),
            START,
        )
        .unwrap();
    state
        .session()
        .set_aim(Some(Aim { x: 700.0, y: 420.0 }), START)
        .unwrap();
    let state = state.with_live_solver(Arc::new(Stars));
    for _ in 0..100 {
        if !state.session().model(sim.clock().now()).syncs.is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(!state.session().model(sim.clock().now()).syncs.is_empty());
    let mut client = state.link().unwrap().mount().unwrap();
    assert_eq!(client.describe_mount().await.unwrap().axes.len(), 2);
    state.goto().unwrap().start().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(mount.state(Axis::Azimuth).await.moving || mount.state(Axis::Altitude).await.moving);
    state.goto().unwrap().stop().await;
    tokio::time::sleep(Duration::from_millis(1500)).await;
    assert!(!mount.state(Axis::Azimuth).await.moving);
    assert!(!mount.state(Axis::Altitude).await.moving);
}
