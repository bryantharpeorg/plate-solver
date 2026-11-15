//! US3 oracles: emulator identity, literal API contracts and the simulator's
//! independent sky and mechanics. Identity, counts, errors and command order are
//! exact; reset endings are bounded by 2 s (Track) or 3 s (GoTo) on paused time.
use crate::support::mcp as support;
use axum::{body::Body, http::Request, Router};
use finder::{
    link::Transport,
    live::{SolveFuture, Solver},
    samples::Samples,
    session::{Aim, Session},
    solve::{self, SolveOptions},
    AppState,
};
use pattern_database::PatternDatabase;
use scope_link::proto::scope_node_mount_server::ScopeNodeMount;
use scope_link::{memory, proto};
use scope_node::{
    clock::NodeClock,
    mount::{
        skywatcher::{
            emulator::{Emulator, EmulatorConfig},
            MountLink,
        },
        Axis, AxisInfo, AxisState, Mount, MountFuture, MountIdentity,
    },
    service::{NodeControls, NodeService},
    synthetic::SyntheticCamera,
};
use serde_json::{json, Value};
use simulator::{
    mount::{MountConfig, SimMount},
    SimCamera, Simulator, VirtualClock,
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
                &options,
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
    fn identity(&self) -> Option<MountIdentity> {
        Some(MountIdentity {
            firmware: "3.40".into(),
            board: "0x90".into(),
        })
    }
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
    pub mount: SimMount,
    pub state: AppState,
    pub app: Router,
    pub commands: Arc<Mutex<Vec<Command>>>,
    pub controls: NodeControls,
    pub resets: Arc<AtomicU64>,
}
impl Rig {
    pub async fn new(start: f64, alt: f64, az: f64, connected: bool) -> Self {
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
        session
            .set_aim(Some(Aim { x: 700.0, y: 420.0 }), start)
            .unwrap();
        let state = AppState::shared(DB.clone(), Samples::default())
            .with_session(session)
            .with_clock(Arc::new(ConsoleClock(clock)))
            .with_link(transport)
            .with_live_solver(Arc::new(Stars));
        let app = support::app(state.clone(), "127.0.0.1");
        let rig = Self {
            sim,
            mount,
            state,
            app,
            commands,
            controls,
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
}

#[tokio::test(start_paused = true)]
async fn service_describes_the_emulator_and_unknown_identity_is_empty() {
    let clock: Arc<dyn NodeClock> = Arc::new(NodeTime(VirtualClock::new(START)));
    let (_, handle) = Emulator::new(EmulatorConfig::default(), clock.clone());
    let reopen_clock = clock.clone();
    let link = MountLink::start(
        Box::new(move || Ok(handle.reopen(reopen_clock.clone()))),
        clock.clone(),
    );
    let service = NodeService::with_mount(
        SyntheticCamera::new(clock.clone()),
        clock.clone(),
        "test-token",
        Default::default(),
        link.clone(),
    )
    .unwrap();
    tokio::time::sleep(Duration::from_secs(3)).await;
    assert!(link.connected());
    let reply = service
        .describe_mount(tonic::Request::new(proto::DescribeMountRequest {}))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(reply.firmware, "3.40");
    assert_eq!(reply.board, "0x90");
    let absent = NodeService::new(
        SyntheticCamera::new(clock.clone()),
        clock,
        "test-token",
        Default::default(),
    )
    .unwrap();
    let reply = absent
        .describe_mount(tonic::Request::new(proto::DescribeMountRequest {}))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(reply.firmware, "");
    assert_eq!(reply.board, "");
}

#[tokio::test(start_paused = true)]
async fn session_reports_identity_and_omits_zero_resets() {
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    let view = support::session(&rig.app).await;
    assert_eq!(view["mount"]["firmware"], "3.40");
    assert_eq!(view["mount"]["board"], "0x90");
    assert!(!view["mount"].as_object().unwrap().contains_key("resets"));
}

const POWER_CYCLED: &str = "The mount was power-cycled: sync again first";
const STEP: Duration = Duration::from_millis(10);

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
impl Rig {
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
            tokio::time::sleep(STEP).await;
        }
    }
    async fn stop_frames_after_sync(&self) {
        self.sync().await;
        self.controls.stop_frames();
        // Drain any exposure already in flight before staging the reset.
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    fn reset(&self) -> usize {
        let after = self.commands.lock().unwrap().len();
        self.resets.store(1, Ordering::SeqCst);
        after
    }
    async fn stopped_after(&self, after: usize, seconds: f64) {
        let deadline = self.now() + seconds;
        while !self.commands.lock().unwrap()[after..]
            .iter()
            .any(|c| c.action == MountCommand::StopAll)
        {
            assert!(self.now() < deadline, "no StopAll after reset");
            tokio::time::sleep(STEP).await;
        }
    }
    fn no_goto_after(&self, after: usize) {
        assert!(!self.commands.lock().unwrap()[after..]
            .iter()
            .any(|c| matches!(c.action, MountCommand::GoTo { .. })));
    }
    fn tracker(&self) -> finder::tracking::Tracker {
        finder::tracking::Tracker::new(
            Arc::new(ConsoleClock(self.sim.clock().clone())),
            self.state.link().unwrap(),
            self.state.session().model(self.now()),
            finder::session::Site {
                lat_deg: 40.0,
                lon_deg: -75.0,
            },
            15.0,
        )
    }
}

#[tokio::test(start_paused = true)]
async fn reset_refuses_goto_and_second_sync_without_sending_a_slew() {
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    rig.stop_frames_after_sync().await;
    let after = rig.reset();
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(rig.view().await["mount"]["resets"], 1);
    for path in ["/api/goto", "/api/model/second-sync"] {
        let (status, body) = rig.call(path, None).await;
        assert_eq!(status, 409, "{path}: {body}");
        assert_eq!(body["error"], POWER_CYCLED);
    }
    rig.no_goto_after(after);
}

#[tokio::test(start_paused = true)]
async fn a_new_frame_sync_allows_goto_again() {
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    rig.stop_frames_after_sync().await;
    rig.reset();
    tokio::time::sleep(Duration::from_millis(500)).await;
    let (status, body) = rig.call("/api/goto", None).await;
    assert_eq!(status, 409);
    assert_eq!(body["error"], POWER_CYCLED);
    let refused = rig.now();
    rig.controls.start_frames();
    let deadline = rig.now() + 10.0;
    while !rig
        .state
        .session()
        .model(rig.now())
        .syncs
        .iter()
        .any(|s| s.time > refused)
    {
        assert!(rig.now() < deadline, "no sync after frames restarted");
        rig.tick().await;
    }
    assert_eq!(
        rig.call("/api/goto", None).await,
        (200, json!({"started":true}))
    );
}

#[tokio::test(start_paused = true)]
async fn reset_aborts_a_slewing_goto_and_sends_stop() {
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    rig.sync().await;
    assert_eq!(rig.call("/api/goto", None).await.0, 200);
    rig.until(3.0, |v| v["goto"]["state"] == "slewing").await;
    // Observe actual motor motion before changing the node's count.
    for _ in 0..100 {
        if rig.mount.state(Axis::Azimuth).await.moving
            || rig.mount.state(Axis::Altitude).await.moving
        {
            break;
        }
        tokio::time::sleep(STEP).await;
    }
    assert!(
        rig.mount.state(Axis::Azimuth).await.moving || rig.mount.state(Axis::Altitude).await.moving
    );
    let changed = rig.now();
    let after = rig.reset();
    let view = rig.until(3.0, |v| v["goto"]["state"] == "failed").await;
    assert!(view["goto"]["reason"]
        .as_str()
        .unwrap()
        .contains("power-cycled"));
    rig.stopped_after(after, 3.0 - (rig.now() - changed)).await;
}

#[tokio::test(start_paused = true)]
async fn reset_ends_tracking_and_refuses_track_on_until_sync() {
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    rig.sync().await;
    assert_eq!(
        rig.call("/api/track", Some(json!({"on":true}))).await.0,
        200
    );
    rig.until(180.0, |v| v["track"]["state"] == "tracking")
        .await;
    rig.controls.stop_frames();
    tokio::time::sleep(Duration::from_millis(500)).await;
    let changed = rig.now();
    let after = rig.reset();
    let view = rig
        .until(2.0, |v| {
            v["track"]["state"] == "off" && v["mount"]["resets"] == 1
        })
        .await;
    assert!(view["track"]["ended_reason"]
        .as_str()
        .unwrap()
        .contains("power-cycled"));
    rig.stopped_after(after, 2.0 - (rig.now() - changed)).await;
    let (status, body) = rig.call("/api/track", Some(json!({"on":true}))).await;
    assert_eq!(status, 409);
    assert_eq!(body["error"], POWER_CYCLED);
}

#[tokio::test(start_paused = true)]
async fn reset_during_correction_stops_before_the_next_slew_pair() {
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    rig.sync().await;
    assert_eq!(rig.call("/api/goto", None).await.0, 200);
    rig.until(180.0, |v| v["goto"]["state"] == "solving").await;
    rig.mount.push_by(0.1, 0.0);
    rig.until(10.0, |v| v["goto"]["state"] == "correcting")
        .await;
    let changed = rig.now();
    let after = rig.reset();
    let view = rig.until(3.0, |v| v["goto"]["state"] == "failed").await;
    assert!(view["goto"]["reason"]
        .as_str()
        .unwrap()
        .contains("power-cycled"));
    rig.stopped_after(after, 3.0 - (rig.now() - changed)).await;
    tokio::time::sleep(Duration::from_secs(1)).await;
    rig.no_goto_after(after);
}

#[tokio::test(start_paused = true)]
async fn reset_during_keyhole_slew_stops_without_a_further_waypoint() {
    let mut target = finder::targets::find("M42").unwrap().clone();
    target.dec_deg = 39.0;
    let start = (1..360)
        .map(|n| START + f64::from(n) * 60.0)
        .find(|t| {
            simulator::sky::horizontal(target.vector(), simulator::Site::default(), *t).0 >= 88.5
        })
        .unwrap();
    let rig = Rig::new(start, 50.0, 120.0, true).await;
    rig.sync().await;
    assert!(
        simulator::sky::horizontal(target.vector(), simulator::Site::default(), rig.now()).0
            >= 88.5
    );
    let tracker = rig.tracker();
    tracker.start(target).await.unwrap();
    let deadline = rig.now() + 2.0;
    loop {
        let gotos = rig
            .commands
            .lock()
            .unwrap()
            .iter()
            .filter(|c| matches!(c.action, MountCommand::GoTo { .. }))
            .count();
        if tracker.state().state == "keyhole_wait" && gotos >= 2 {
            break;
        }
        assert!(
            rig.now() < deadline,
            "keyhole slew did not start: {:?}",
            tracker.state()
        );
        tokio::time::sleep(STEP).await;
    }
    assert!(
        rig.mount.state(Axis::Azimuth).await.moving || rig.mount.state(Axis::Altitude).await.moving
    );
    let changed = rig.now();
    let after = rig.reset();
    while tracker.state().state != "off" {
        assert!(rig.now() - changed < 2.0, "keyhole did not end");
        tokio::time::sleep(STEP).await;
    }
    assert!(tracker
        .state()
        .ended_reason
        .unwrap()
        .contains("power-cycled"));
    rig.stopped_after(after, 2.0 - (rig.now() - changed)).await;
    tokio::time::sleep(Duration::from_secs(1)).await;
    rig.no_goto_after(after);
}

#[tokio::test(start_paused = true)]
async fn tracking_first_read_uses_the_sessions_last_seen_reset_count() {
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    rig.sync().await;
    assert!(!rig.view().await["mount"]
        .as_object()
        .unwrap()
        .contains_key("resets"));
    let tracker = rig.tracker();
    let changed = rig.now();
    let after = rig.reset();
    tracker
        .start_with_resets(finder::targets::find("M42").unwrap().clone(), Some(0))
        .await
        .unwrap();
    while tracker.state().state != "off" {
        assert!(
            rig.now() - changed < 2.0,
            "tracking did not end at its first read"
        );
        tokio::time::sleep(STEP).await;
    }
    assert!(tracker
        .state()
        .ended_reason
        .unwrap()
        .contains("power-cycled"));
    rig.stopped_after(after, 2.0 - (rig.now() - changed)).await;
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert!(!rig.commands.lock().unwrap()[after..]
        .iter()
        .any(|c| matches!(c.action, MountCommand::Rate { .. })));
}
