//! Stop tests use real simulator mechanics and the node's token-free Stop service.
use std::{
    io::Write,
    net::SocketAddr,
    path::Path,
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        Arc, LazyLock, Mutex,
    },
    time::Duration,
};

use axum::{
    body::{to_bytes, Body},
    extract::connect_info::MockConnectInfo,
    http::Request,
    response::Response,
    Router,
};
use finder::{
    link::Transport,
    live::{SolveFuture, Solver},
    samples::Samples,
    session::{Aim, Clock, Session},
    solve::{self, SolveOptions},
    AppState,
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
    SimCamera, Simulator, VirtualClock,
};
use tower::ServiceExt;

pub const START: f64 = 1_768_528_800.0;
static DB: LazyLock<Arc<PatternDatabase>> = LazyLock::new(|| {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../reference-solutions/cedar-solve/tetra3/data/default_database.npz");
    Arc::new(pattern_database::load_from_path(&path).unwrap())
});

pub struct ConsoleClock(pub VirtualClock);
impl Clock for ConsoleClock {
    fn now(&self) -> f64 {
        self.0.now()
    }
}
struct NodeTime {
    clock: VirtualClock,
    offset_ms: Arc<AtomicU64>,
}
impl NodeClock for NodeTime {
    fn now(&self) -> f64 {
        self.clock.node_now() + self.offset_ms.load(Ordering::SeqCst) as f64 / 1000.0
    }
}
// Solve on the paused runtime so a blocking worker cannot be outrun by time.
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

#[derive(Debug, Clone, PartialEq)]
pub enum MountCommand {
    GoTo { axis: Axis, position: i32 },
    Rate { axis: Axis, deg_per_s: f64 },
    StopAll,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Command {
    pub at: f64,
    pub action: MountCommand,
}
#[derive(Clone)]
struct Recorded {
    mount: SimMount,
    clock: VirtualClock,
    commands: Arc<Mutex<Vec<Command>>>,
    hold_state: Arc<AtomicBool>,
    held_states: Arc<AtomicUsize>,
    hold_rate: Arc<AtomicBool>,
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
    fn axes(&self) -> [AxisInfo; 2] {
        self.mount.axes()
    }
    fn state(&self, axis: Axis) -> MountFuture<'_, AxisState> {
        Box::pin(async move {
            if self.hold_state.load(Ordering::SeqCst) {
                self.held_states.fetch_add(1, Ordering::SeqCst);
                while self.hold_state.load(Ordering::SeqCst) {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            }
            self.mount.state(axis).await
        })
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
            if self.hold_rate.load(Ordering::SeqCst) {
                std::future::pending::<()>().await;
            }
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

#[derive(Clone, Copy)]
pub enum Mode {
    Pass,
    Hold(usize),
    HoldAll,
    Refuse,
}
#[derive(Clone)]
pub struct StopGate {
    node: NodeService,
    clock: VirtualClock,
    mode: Arc<Mutex<Mode>>,
    attempts: Arc<Mutex<Vec<f64>>>,
}
impl StopGate {
    pub fn set(&self, mode: Mode) {
        *self.mode.lock().unwrap() = mode;
    }
    pub fn log(&self) -> Vec<f64> {
        self.attempts.lock().unwrap().clone()
    }
}
#[tonic::async_trait]
impl proto::scope_node_stop_server::ScopeNodeStop for StopGate {
    async fn stop_all(
        &self,
        request: tonic::Request<proto::StopAllRequest>,
    ) -> Result<tonic::Response<proto::StopAllReply>, tonic::Status> {
        self.attempts.lock().unwrap().push(self.clock.now());
        let mode = {
            let mut mode = self.mode.lock().unwrap();
            let current = *mode;
            if let Mode::Hold(n) = current {
                *mode = Mode::Hold(n.saturating_sub(1));
            }
            current
        };
        match mode {
            Mode::HoldAll | Mode::Hold(1..) => std::future::pending().await,
            Mode::Refuse => Err(tonic::Status::unavailable("stop reply unavailable")),
            Mode::Pass | Mode::Hold(0) => {
                proto::scope_node_stop_server::ScopeNodeStop::stop_all(&self.node, request).await
            }
        }
    }
}

#[derive(Clone, Default)]
pub struct Log(Arc<Mutex<Vec<u8>>>);
impl Write for Log {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl Log {
    pub fn text(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }
}

pub fn state() -> AppState {
    AppState::shared(DB.clone(), Samples::default())
        .with_clock(Arc::new(finder::session::FixedClock(START)))
}
pub fn app(state: AppState) -> Router {
    finder::router(state).layer(MockConnectInfo(SocketAddr::from(([127, 0, 0, 1], 12345))))
}
pub async fn send(app: &Router, request: Request<Body>) -> Response {
    app.clone().oneshot(request).await.unwrap()
}
pub async fn body(response: Response) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap()
}
pub async fn session(app: &Router) -> Value {
    body(
        send(
            app,
            Request::get("/api/session").body(Body::empty()).unwrap(),
        )
        .await,
    )
    .await
}
pub fn post(path: &str, value: Value) -> Request<Body> {
    Request::post(path)
        .header("host", "localhost:8080")
        .header("content-type", "application/json")
        .body(Body::from(value.to_string()))
        .unwrap()
}
pub fn stop() -> Request<Body> {
    // Stop has no token, body, Host or Origin requirement.
    Request::post("/api/stop").body(Body::empty()).unwrap()
}
pub fn rpc(value: Value) -> Request<Body> {
    let mut request = post("/mcp", value);
    request.headers_mut().insert(
        "accept",
        "application/json, text/event-stream".parse().unwrap(),
    );
    request
        .headers_mut()
        .insert("mcp-protocol-version", "2025-11-25".parse().unwrap());
    request
}

pub struct Rig {
    pub sim: Simulator,
    pub mount: SimMount,
    pub state: AppState,
    pub app: Router,
    pub commands: Arc<Mutex<Vec<Command>>>,
    pub controls: NodeControls,
    pub gate: StopGate,
    pub hold_state: Arc<AtomicBool>,
    pub held_states: Arc<AtomicUsize>,
    #[allow(dead_code)] // Used by US2's failed-start scenario.
    pub hold_rate: Arc<AtomicBool>,
    /// US4 advances only the node's clock through this shared test fixture.
    #[allow(dead_code)]
    pub node_offset_ms: Arc<AtomicU64>,
    pub call_log: Log,
}
impl Rig {
    pub async fn new() -> Self {
        let clock = VirtualClock::new(START);
        let sim = Simulator::new(DB.clone(), simulator::Site::default(), clock.clone());
        let mount = SimMount::with_config(
            clock.clone(),
            MountConfig {
                base_tilt_deg: 1.0,
                base_tilt_toward_deg: 130.0,
                zero_points_deg: [120.0, 50.0],
                backlash_deg: 0.3,
            },
        );
        sim.connect_mount(mount.clone());
        let commands = Arc::new(Mutex::new(Vec::new()));
        let hold_state = Arc::new(AtomicBool::new(false));
        let held_states = Arc::new(AtomicUsize::new(0));
        let hold_rate = Arc::new(AtomicBool::new(false));
        let node_offset_ms = Arc::new(AtomicU64::new(0));
        let recorded = Recorded {
            mount: mount.clone(),
            clock: clock.clone(),
            commands: commands.clone(),
            hold_state: hold_state.clone(),
            held_states: held_states.clone(),
            hold_rate: hold_rate.clone(),
        };
        let node = NodeService::with_controls_and_mount(
            SimCamera::new(sim.clone()),
            Arc::new(NodeTime {
                clock: clock.clone(),
                offset_ms: node_offset_ms.clone(),
            }),
            "test-token",
            Default::default(),
            NodeControls::default(),
            Some(Arc::new(recorded)),
        )
        .unwrap();
        let controls = node.controls();
        controls.set_frame_interval(Duration::from_millis(500));
        let gate = StopGate {
            node: node.clone(),
            clock: clock.clone(),
            mode: Arc::new(Mutex::new(Mode::Pass)),
            attempts: Arc::new(Mutex::new(Vec::new())),
        };
        let stop_gate = gate.clone();
        let transport = Transport::connector(move || {
            let node = node.clone();
            let gate = stop_gate.clone();
            async move {
                let (client, server) = tokio::io::duplex(64 * 1024);
                tokio::spawn(async move {
                    let routes = tonic::service::Routes::new(node.clone().into_server())
                        .add_service(
                            proto::scope_node_mount_server::ScopeNodeMountServer::with_interceptor(
                                node,
                                scope_link::TokenInterceptor::new("test-token"),
                            ),
                        )
                        .add_service(proto::scope_node_stop_server::ScopeNodeStopServer::new(
                            gate,
                        ));
                    memory::serve_routes_on(server, routes).await.unwrap();
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
                START,
            )
            .unwrap();
        session
            .set_aim(Some(Aim { x: 700.0, y: 420.0 }), START)
            .unwrap();
        let call_log = Log::default();
        let state = AppState::shared(DB.clone(), Samples::default())
            .with_session(session)
            .with_clock(Arc::new(ConsoleClock(clock)))
            .with_link(transport)
            .with_live_solver(Arc::new(Stars))
            .with_call_log(call_log.clone());
        let app = app(state.clone());
        let rig = Self {
            sim,
            mount,
            state,
            app,
            commands,
            controls,
            gate,
            hold_state,
            held_states,
            hold_rate,
            node_offset_ms,
            call_log,
        };
        for _ in 0..100 {
            let view = session_view(&rig).await;
            if view["node"]["connected"] == true && view["mount"]["connected"] == true {
                return rig;
            }
            rig.tick().await;
        }
        panic!("scope link did not connect: {}", session_view(&rig).await);
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
        panic!("no solved sync: {}", session_view(self).await);
    }
}
async fn session_view(rig: &Rig) -> Value {
    session(&rig.app).await
}
