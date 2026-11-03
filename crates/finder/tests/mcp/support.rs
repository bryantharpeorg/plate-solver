//! Router-only MCP requests. All expectations come from routes or the MCP contract.
use std::net::SocketAddr;
use std::path::Path;
use std::sync::{Arc, LazyLock};

use axum::body::{to_bytes, Body};
use axum::extract::connect_info::MockConnectInfo;
use axum::http::Request;
use axum::response::Response;
use axum::Router;
use finder::samples::Samples;
use finder::session::FixedClock;
use finder::{router, AppState};
use pattern_database::PatternDatabase;
use serde_json::{json, Value};
use tower::ServiceExt;

static DB: LazyLock<Arc<PatternDatabase>> = LazyLock::new(|| {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../reference-solutions/cedar-solve/tetra3/data/default_database.npz");
    Arc::new(pattern_database::load_from_path(&path).unwrap())
});

pub fn state() -> AppState {
    AppState::shared(DB.clone(), Samples::default()).with_clock(Arc::new(FixedClock(1768528800.0)))
}

pub fn app(state: AppState, peer: &str) -> Router {
    router(state).layer(MockConnectInfo(SocketAddr::new(
        peer.parse().unwrap(),
        12345,
    )))
}

pub fn request(message: Value) -> Request<Body> {
    Request::post("/mcp")
        .header("host", "localhost:8080")
        .header("content-type", "application/json")
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-11-25")
        .body(Body::from(message.to_string()))
        .unwrap()
}

pub fn call(name: &str, arguments: Value) -> Request<Body> {
    request(json!({"jsonrpc":"2.0", "id":1, "method":"tools/call",
        "params":{"name":name, "arguments":arguments}}))
}

pub async fn send(app: &Router, request: Request<Body>) -> Response {
    app.clone().oneshot(request).await.unwrap()
}

pub async fn body(response: Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

pub async fn session(app: &Router) -> Value {
    let response = send(
        app,
        Request::get("/api/session").body(Body::empty()).unwrap(),
    )
    .await;
    assert_eq!(response.status(), 200);
    body(response).await
}

#[allow(dead_code)]
pub fn state_with_samples(samples: Samples) -> AppState {
    AppState::shared(DB.clone(), samples).with_clock(Arc::new(FixedClock(1768528800.0)))
}

/// In-memory scope link and real centroid solves, shared by the motion stories.
pub mod sim {
    #![allow(dead_code)]

    use super::*;
    use finder::{
        link::Transport,
        live::{SolveFuture, Solver},
        session::{Aim, Clock, Session},
        solve::{self, SolveOptions},
    };
    use scope_link::{memory, proto};
    use scope_node::{
        clock::NodeClock,
        mount::{Axis, AxisInfo, AxisState, Mount, MountFuture},
        service::{NodeControls, NodeService},
    };
    use simulator::{
        mount::{MountConfig, SimMount},
        SimCamera, Simulator, VirtualClock,
    };
    use std::{sync::Mutex, time::Duration};

    pub const START: f64 = 1_768_528_800.0;

    pub struct ConsoleClock(pub VirtualClock);
    impl Clock for ConsoleClock {
        fn now(&self) -> f64 {
            self.0.now()
        }
    }
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
            let recorded = Recorded {
                mount: mount.clone(),
                clock: clock.clone(),
                commands: commands.clone(),
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
            let app = app(state.clone(), "127.0.0.1");
            let rig = Self {
                sim,
                mount,
                state,
                app,
                commands,
                controls,
            };
            for _ in 0..100 {
                let view = session_view(&rig).await;
                if view["node"]["connected"] == true
                    && (!connected || view["mount"]["connected"] == true)
                {
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
}
