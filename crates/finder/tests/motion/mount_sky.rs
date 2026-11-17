//! US1 oracles (026): the filter draws each frame's stars at the pointing the
//! encoders give, so syncs, GoTo and every guard run unchanged, and a board reset
//! stops the drawing for good. Truth is computed here from counts and the
//! simulator's sky, never read back from the filter.
use crate::support::mcp as support;
use axum::{body::Body, http::Request, Router};
use finder::{
    ephemeris,
    link::{FrameFilter, Transport},
    live::{SolveFuture, Solver},
    mount_sky::MountSky,
    samples::Samples,
    session::Session,
    solve::{self, SolveOptions},
    targets, AppState,
};
use pattern_database::PatternDatabase;
use scope_link::{memory, proto};
use scope_node::{
    clock::NodeClock,
    mount::{Axis, AxisInfo, AxisState, Mount, MountFuture},
    service::{NodeControls, NodeService},
};
use serde_json::{json, Value};
use simulator::{
    mount::{MountConfig, SimMount},
    sky, SimCamera, Simulator, Site, VirtualClock,
};
use std::{
    path::Path,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, LazyLock, Mutex,
    },
    time::Duration,
};
use support::sim::{Command, ConsoleClock, MountCommand, START};

static DB: LazyLock<Arc<PatternDatabase>> = LazyLock::new(|| {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../reference-solutions/cedar-solve/tetra3/data/default_database.npz");
    Arc::new(pattern_database::load_from_path(&path).unwrap())
});

/// The drawn sky's tube altitude, as the console was started with.
const TUBE_ALT: f64 = 30.0;
/// The `SimMount` fixture's counts per revolution (the real board reports 1,980,000).
const COUNTS: [u32; 2] = [1_620_000, 1_620_000];
const SITE: Site = Site {
    lat_deg: 40.0,
    lon_deg: -75.0,
};
/// A console time with the Sun up at the site, when the sky still solves.
const DAY: f64 = START + 15.0 * 3600.0;

struct NodeTime(VirtualClock);
impl NodeClock for NodeTime {
    fn now(&self) -> f64 {
        self.0.node_now()
    }
}
// Solve rendered stars on the paused runtime, so virtual time cannot race a
// blocking worker and outrun the exposure that supplied its centroids.
struct Stars;
impl Solver for Stars {
    fn solve(&self, frame: proto::StarFrame, options: SolveOptions) -> SolveFuture<'_> {
        Box::pin(async move {
            solve::solve_centroids(
                &DB,
                &frame.centroids,
                (frame.width as usize, frame.height as usize),
                &SolveOptions {
                    timeout_ms: u64::MAX,
                    ..options
                },
            )
        })
    }
}

#[derive(Clone)]
struct Recorded {
    mount: SimMount,
    clock: VirtualClock,
    commands: Arc<Mutex<Vec<Command>>>,
    resets: Arc<AtomicU64>,
}
impl Recorded {
    fn record(&self, action: MountCommand) {
        self.commands.lock().unwrap().push(Command {
            at: self.clock.now(),
            action,
        });
    }
}
impl Mount for Recorded {
    fn resets(&self) -> u64 {
        self.resets.load(Ordering::SeqCst)
    }
    fn axes(&self) -> [AxisInfo; 2] {
        self.mount.axes()
    }
    fn state(&self, axis: Axis) -> MountFuture<'_, AxisState> {
        self.mount.state(axis)
    }
    fn goto(&self, axis: Axis, position: i32) -> MountFuture<'_, ()> {
        Box::pin(async move {
            self.record(MountCommand::GoTo { axis, position });
            self.mount.goto(axis, position).await;
        })
    }
    fn rate(&self, axis: Axis, deg_per_s: f64) -> MountFuture<'_, ()> {
        Box::pin(async move {
            self.record(MountCommand::Rate { axis, deg_per_s });
            self.mount.rate(axis, deg_per_s).await;
        })
    }
    fn stop_all(&self) -> MountFuture<'_, ()> {
        Box::pin(async move {
            self.record(MountCommand::StopAll);
            self.mount.stop_all().await;
        })
    }
}

pub struct Rig {
    pub sim: Simulator,
    pub state: AppState,
    pub app: Router,
    pub commands: Arc<Mutex<Vec<Command>>>,
    pub resets: Arc<AtomicU64>,
}
impl Rig {
    pub async fn new(start: f64, connected: bool) -> Self {
        let clock = VirtualClock::new(start);
        let sim = Simulator::new(DB.clone(), SITE, clock.clone());
        let mount = SimMount::with_config(
            clock.clone(),
            MountConfig {
                base_tilt_deg: 1.0,
                base_tilt_toward_deg: 130.0,
                zero_points_deg: [300.0, 70.0],
                backlash_deg: 0.3,
            },
        );
        if connected {
            sim.connect_mount(mount.clone());
        }
        // The zero counts must be fixed away from the power-on zero (plan trap 7),
        // before any frame can reach the console.
        for (axis, count) in [(Axis::Azimuth, 20_000), (Axis::Altitude, 10_000)] {
            mount.goto(axis, count).await;
        }
        for _ in 0..400 {
            tokio::time::sleep(Duration::from_millis(50)).await;
            if !mount.state(Axis::Azimuth).await.moving && !mount.state(Axis::Altitude).await.moving
            {
                break;
            }
        }
        for axis in [Axis::Azimuth, Axis::Altitude] {
            let state = mount.state(axis).await;
            assert!(!state.moving, "the setup mount did not settle");
            let settled = if axis == Axis::Azimuth {
                20_000
            } else {
                10_000
            };
            assert_eq!(state.position, settled);
        }
        let commands = Arc::new(Mutex::new(Vec::new()));
        let resets = Arc::new(AtomicU64::new(0));
        let recorded = Recorded {
            mount: mount.clone(),
            clock: clock.clone(),
            commands: commands.clone(),
            resets: resets.clone(),
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
        session
            .edit(
                serde_json::from_value(json!({
                    "node":{"address":"scope.local:1","token":"test-token"},
                    "site":{"lat_deg":40,"lon_deg":-75}, "target":"M42",
                    "camera_settings":{"exposure_ms":200,"gain":1}
                }))
                .unwrap(),
                start,
            )
            .unwrap();
        let state = AppState::shared(DB.clone(), Samples::default())
            .with_session(session)
            .with_clock(Arc::new(ConsoleClock(clock)))
            .with_mount_sky(transport, TUBE_ALT)
            .with_live_solver(Arc::new(Stars));
        let app = support::app(state.clone(), "127.0.0.1");
        let rig = Self {
            sim,
            state,
            app,
            commands,
            resets,
        };
        for _ in 0..100 {
            let view = support::session(&rig.app).await;
            if view["node"]["connected"] == true
                && (!connected || view["mount"]["connected"] == true)
            {
                return rig;
            }
            rig.tick().await;
        }
        panic!(
            "scope link did not connect: {}",
            support::session(&rig.app).await
        );
    }
    pub fn now(&self) -> f64 {
        self.sim.clock().now()
    }
    pub async fn tick(&self) {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    pub async fn sync(&self) {
        for _ in 0..100 {
            if !self.state.session().model(self.now()).syncs.is_empty() {
                return;
            }
            self.tick().await;
        }
        panic!("no solved sync: {}", support::session(&self.app).await);
    }
    /// The newest frame the link published.
    fn frame(&self) -> proto::StarFrame {
        self.state
            .link()
            .unwrap()
            .frames()
            .borrow_and_update()
            .clone()
            .expect("no frame published")
    }
    async fn call(&self, path: &str, body: Option<Value>) -> (u16, Value) {
        let response = support::send(&self.app, post(path, body)).await;
        (response.status().as_u16(), support::body(response).await)
    }
    async fn view(&self) -> Value {
        support::session(&self.app).await
    }
    async fn until(&self, seconds: f64, condition: impl Fn(&Value) -> bool) -> Value {
        let deadline = self.now() + seconds;
        loop {
            let view = self.view().await;
            if condition(&view) {
                return view;
            }
            assert!(self.now() < deadline, "condition timed out: {view}");
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
}

fn post(path: &str, body: Option<Value>) -> Request<Body> {
    let request = Request::post(path).header("host", "localhost:8080");
    match body {
        Some(value) => request
            .header("content-type", "application/json")
            .body(Body::from(value.to_string()))
            .unwrap(),
        None => request.body(Body::empty()).unwrap(),
    }
}

/// The mid-exposure counts of a frame carrying both ends.
fn mid_counts(frame: &proto::StarFrame) -> [f64; 2] {
    let (start, end) = (
        frame.axis_start.as_ref().unwrap(),
        frame.axis_end.as_ref().unwrap(),
    );
    [
        (f64::from(start.azimuth) + f64::from(end.azimuth)) / 2.0,
        (f64::from(start.altitude) + f64::from(end.altitude)) / 2.0,
    ]
}

/// The truth of counts (spec § User Scenarios): the zero counts read as azimuth
/// 180° and the tube's altitude, and each revolution is a full turn.
fn truth_of_counts(
    mid: [f64; 2],
    zero: [i32; 2],
    counts: [u32; 2],
    tube_alt_deg: f64,
) -> (f64, f64) {
    (
        (180.0 + (mid[0] - f64::from(zero[0])) * 360.0 / f64::from(counts[0])).rem_euclid(360.0),
        tube_alt_deg + (mid[1] - f64::from(zero[1])) * 360.0 / f64::from(counts[1]),
    )
}

/// A target's apparent horizontal position at a console time, from the
/// simulator's sky and Saemundsson's refraction.
fn apparent_horizontal(target: &targets::Target, site: Site, time: f64) -> (f64, f64) {
    let (alt, az) = sky::horizontal(target.vector(), site, time);
    (ephemeris::apparent_from_true(alt), az)
}

/// Smallest angle between two azimuths, in degrees.
fn az_difference(a: f64, b: f64) -> f64 {
    ((a - b + 180.0).rem_euclid(360.0) - 180.0).abs()
}

/// The session's fixed zero counts, as whole numbers.
fn zero_counts(view: &Value) -> [i32; 2] {
    let zero = &view["mount_sky"]["zero_counts"];
    [
        zero[0].as_i64().unwrap() as i32,
        zero[1].as_i64().unwrap() as i32,
    ]
}

#[tokio::test(start_paused = true)]
async fn the_first_counted_frame_fixes_the_zero_and_draws_the_pointing() {
    let rig = Rig::new(START, true).await;
    let view = rig
        .until(60.0, |v| {
            v["pointing"]["source"] == "live" && !v["mount_sky"]["zero_counts"].is_null()
        })
        .await;
    assert_eq!(view["mount_sky"]["tube_alt_deg"].as_f64(), Some(TUBE_ALT));
    let frame = rig.frame();
    let mid = mid_counts(&frame);
    let zero = zero_counts(&view);
    assert_eq!(zero, [mid[0].round() as i32, mid[1].round() as i32]);
    assert_eq!(zero, [20_000, 10_000]);
    assert!(!frame.centroids.is_empty());
    // The node's own camera points more than 40° away, so the live pointing can
    // only come from the drawn stars, not the camera's (plan trap 7).
    let camera = rig.sim.telescope().camera;
    assert!(
        finder::model::separation(camera.az, camera.alt, 180.0, TUBE_ALT) > 40.0,
        "the camera is not away from the drawn sky: {camera:?}"
    );
    let alt = view["pointing"]["alt_deg"].as_f64().unwrap();
    let az = view["pointing"]["az_deg"].as_f64().unwrap();
    assert!(az_difference(az, 180.0) < 0.05, "azimuth {az}");
    assert!((alt - TUBE_ALT).abs() < 0.05, "altitude {alt}");
}

#[tokio::test(start_paused = true)]
async fn the_filter_keeps_the_frame_and_replaces_its_centroids() {
    let rig = Rig::new(START, true).await;
    let sky = MountSky::new(DB.clone(), rig.state.session().clone(), TUBE_ALT);
    let frame = proto::StarFrame {
        id: 7,
        start_s: 10.0,
        end_s: 10.2,
        width: 1456,
        height: 1088,
        exposure_ms: 200.0,
        gain: 1.0,
        settings: None,
        centroids: vec![
            proto::Centroid {
                x: 1.0,
                y: 2.0,
                brightness: 3.0,
            },
            proto::Centroid {
                x: 4.0,
                y: 5.0,
                brightness: 6.0,
            },
            proto::Centroid {
                x: 7.0,
                y: 8.0,
                brightness: 9.0,
            },
        ],
        preview_jpeg: Some(b"a preview".to_vec()),
        axis_start: Some(proto::AxisPositions {
            azimuth: 1000,
            altitude: 2000,
        }),
        axis_end: Some(proto::AxisPositions {
            azimuth: 1000,
            altitude: 2000,
        }),
    };
    let drawn = sky.filter(frame.clone(), Some(0.0));
    assert_eq!(drawn.id, 7);
    assert_eq!(drawn.start_s, 10.0);
    assert_eq!(drawn.end_s, 10.2);
    assert_eq!(drawn.width, 1456);
    assert_eq!(drawn.height, 1088);
    assert_eq!(drawn.exposure_ms, 200.0);
    assert_eq!(drawn.gain, 1.0);
    assert_eq!(drawn.axis_start, frame.axis_start);
    assert_eq!(drawn.axis_end, frame.axis_end);
    assert!(drawn.preview_jpeg.is_none());
    assert!(
        (20..=200).contains(&drawn.centroids.len()),
        "{} centroids",
        drawn.centroids.len()
    );
    let brightness: Vec<f64> = drawn.centroids.iter().map(|c| c.brightness).collect();
    assert!(
        brightness.windows(2).all(|pair| pair[0] >= pair[1]),
        "not in non-increasing brightness"
    );
    assert_eq!(sky.view().zero_counts, Some([1000, 2000]));
}

#[tokio::test(start_paused = true)]
async fn a_night_goto_lands_the_drawn_pointing_on_m42() {
    let rig = Rig::new(START, true).await;
    rig.sync().await;
    assert_eq!(
        rig.call("/api/goto", None).await,
        (200, json!({"started": true}))
    );
    let done = rig
        .until(600.0, |v| {
            matches!(
                v["goto"]["state"].as_str(),
                Some("done" | "failed" | "halted")
            )
        })
        .await;
    assert_eq!(done["goto"]["state"], "done", "the GoTo did not end done");
    let frame = rig.frame();
    let mid = mid_counts(&frame);
    let zero = zero_counts(&done);
    let offset = rig
        .state
        .link()
        .unwrap()
        .status()
        .borrow()
        .offset_s
        .unwrap();
    let console_time = (frame.start_s + frame.end_s) / 2.0 - offset;
    let (truth_az, truth_alt) = truth_of_counts(mid, zero, COUNTS, TUBE_ALT);
    let (m42_alt, m42_az) = apparent_horizontal(targets::find("M42").unwrap(), SITE, console_time);
    assert!(
        az_difference(truth_az, m42_az) < 0.05,
        "azimuth truth {truth_az} vs M42 {m42_az}"
    );
    assert!(
        (truth_alt - m42_alt).abs() < 0.05,
        "altitude truth {truth_alt} vs M42 {m42_alt}"
    );
}

#[tokio::test(start_paused = true)]
async fn a_daytime_goto_is_refused_and_moves_nothing() {
    assert!(
        finder::goto::sun_horizontal(SITE.lat_deg, SITE.lon_deg, DAY).0 >= -0.83,
        "the day setup has the Sun up"
    );
    let rig = Rig::new(DAY, true).await;
    rig.sync().await;
    let (status, body) = rig.call("/api/goto", None).await;
    assert_eq!(status, 409, "{body}");
    assert_eq!(body["error"], "Sun is up");
    let actions = rig.commands.lock().unwrap();
    assert!(
        !actions.iter().any(|c| matches!(
            c.action,
            MountCommand::GoTo { .. } | MountCommand::Rate { .. }
        )),
        "a motion command reached the mount: {actions:?}"
    );
}

#[tokio::test(start_paused = true)]
async fn a_node_without_a_mount_draws_nothing() {
    let rig = Rig::new(START, false).await;
    let mut frames = rig.state.link().unwrap().frames();
    // Mark whatever the link holds now as seen, so only arriving frames count.
    frames.borrow_and_update();
    for _ in 0..3 {
        frames.changed().await.unwrap();
        let frame = frames
            .borrow_and_update()
            .clone()
            .expect("no frame published");
        assert!(
            frame.centroids.is_empty(),
            "the camera's centroids reached the console"
        );
    }
    let view = rig.view().await;
    assert_eq!(view["pointing"], Value::Null, "a live pointing appeared");
    assert_eq!(
        view["mount_sky"],
        json!({"tube_alt_deg": TUBE_ALT, "zero_counts": Value::Null})
    );
}

#[tokio::test(start_paused = true)]
async fn a_power_cycle_stops_the_drawn_sky_for_good() {
    let rig = Rig::new(START, true).await;
    rig.sync().await;
    let settled = rig
        .until(10.0, |v| !v["mount_sky"]["zero_counts"].is_null())
        .await;
    let zero = zero_counts(&settled);
    let last = rig.frame();
    assert!(
        last.centroids.len() >= 20,
        "{} centroids before the reset",
        last.centroids.len()
    );
    rig.resets.store(1, Ordering::SeqCst);
    rig.until(10.0, |v| v["mount"]["resets"] == 1).await;
    // Eleven frames span more than the node's 5 s preview interval. Mark the
    // last pre-reset frame as seen so only frames arriving now count.
    let mut frames = rig.state.link().unwrap().frames();
    frames.borrow_and_update();
    for _ in 0..11 {
        frames.changed().await.unwrap();
        let frame = frames
            .borrow_and_update()
            .clone()
            .expect("no frame published");
        assert!(frame.centroids.is_empty(), "a frame still has stars");
        assert!(frame.preview_jpeg.is_none(), "a frame kept its preview");
    }
    let view = rig.view().await;
    assert_eq!(view["mount_sky"]["power_cycled"], true);
    assert_eq!(zero_counts(&view), zero);
    let (status, body) = rig.call("/api/goto", None).await;
    assert_eq!(status, 409, "{body}");
    rig.resets.store(0, Ordering::SeqCst);
    rig.until(10.0, |v| {
        !v["mount"].as_object().unwrap().contains_key("resets")
    })
    .await;
    for _ in 0..3 {
        frames.changed().await.unwrap();
        let frame = frames
            .borrow_and_update()
            .clone()
            .expect("no frame published");
        assert!(
            frame.centroids.is_empty(),
            "drawing resumed after the count changed back"
        );
    }
    assert_eq!(rig.view().await["mount_sky"]["power_cycled"], true);
}
