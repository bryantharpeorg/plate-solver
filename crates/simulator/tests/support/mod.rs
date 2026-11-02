#![allow(dead_code)]
use pattern_database::PatternDatabase;
use scope_link::{memory, proto, BearerToken};
use scope_node::{
    clock::NodeClock,
    mount::{Axis, AxisInfo, AxisState, Mount, MountFuture},
    service::NodeService,
    Settings,
};
use simulator::{mount::SimMount, SimCamera, Simulator, Site, VirtualClock};
use std::{
    path::Path,
    sync::{Arc, LazyLock, Mutex},
    time::Duration,
};
use tokio::sync::Notify;
use tonic::{service::interceptor::InterceptedService, transport::Channel};

pub type CameraClient =
    proto::scope_node_client::ScopeNodeClient<InterceptedService<Channel, BearerToken>>;
pub type MountClient =
    proto::scope_node_mount_client::ScopeNodeMountClient<InterceptedService<Channel, BearerToken>>;
pub type StopClient = proto::scope_node_stop_client::ScopeNodeStopClient<Channel>;
static DB: LazyLock<Arc<PatternDatabase>> = LazyLock::new(|| {
    Arc::new(
        pattern_database::load_from_path(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../reference-solutions/cedar-solve/tetra3/data/default_database.npz"),
        )
        .unwrap(),
    )
});

pub struct Clock(pub VirtualClock);
impl NodeClock for Clock {
    fn now(&self) -> f64 {
        self.0.node_now()
    }
}

// A controllably slow adapter still executes every accepted command on SimMount.
// Stop timestamps are sampled from the same external clock as the node.
pub struct Adapter {
    pub mount: SimMount,
    pub clock: VirtualClock,
    pub entered: Notify,
    pub release: Notify,
    pub gotos: Mutex<Vec<i32>>,
    pub stops: Mutex<Vec<f64>>,
}
impl Mount for Adapter {
    fn axes(&self) -> [AxisInfo; 2] {
        self.mount.axes()
    }
    fn state(&self, axis: Axis) -> MountFuture<'_, AxisState> {
        self.mount.state(axis)
    }
    fn goto(&self, axis: Axis, position: i32) -> MountFuture<'_, ()> {
        Box::pin(async move {
            self.gotos.lock().unwrap().push(position);
            self.mount.goto(axis, position).await;
        })
    }
    fn rate(&self, axis: Axis, rate: f64) -> MountFuture<'_, ()> {
        Box::pin(async move {
            if rate == 0.123 {
                self.entered.notify_one();
                self.release.notified().await;
            }
            self.mount.rate(axis, rate).await;
        })
    }
    fn stop_all(&self) -> MountFuture<'_, ()> {
        Box::pin(async move {
            self.stops.lock().unwrap().push(self.clock.node_now());
            self.mount.stop_all().await;
        })
    }
}

pub struct Rig {
    pub sim: Simulator,
    pub adapter: Arc<Adapter>,
    pub service: NodeService,
    pub camera: CameraClient,
    pub mount: MountClient,
    pub stop: StopClient,
    server: tokio::task::JoinHandle<()>,
}
impl Drop for Rig {
    fn drop(&mut self) {
        self.server.abort();
    }
}
impl Rig {
    pub async fn new() -> Self {
        let clock = VirtualClock::new(1_768_528_800.0);
        let sim = Simulator::new(DB.clone(), Site::default(), clock.clone());
        let mount = SimMount::new(clock.clone());
        sim.connect_mount(mount.clone());
        let adapter = Arc::new(Adapter {
            mount,
            clock: clock.clone(),
            entered: Notify::new(),
            release: Notify::new(),
            gotos: Mutex::new(Vec::new()),
            stops: Mutex::new(Vec::new()),
        });
        let service = NodeService::with_mount(
            SimCamera::new(sim.clone()),
            Arc::new(Clock(clock)),
            "test-token",
            Settings {
                exposure_ms: Some(200.0),
                gain: Some(1.0),
                ..Settings::default()
            },
            adapter.clone(),
        )
        .unwrap();
        let (connect, io) = memory::channel_pair();
        let routes = service.routes();
        let server = tokio::spawn(async move {
            memory::serve_routes_on(io, routes).await.unwrap();
        });
        let channel = connect.await.unwrap();
        Self {
            sim,
            adapter,
            service,
            camera: proto::scope_node_client::ScopeNodeClient::with_interceptor(
                channel.clone(),
                BearerToken::new("test-token").unwrap(),
            ),
            mount: proto::scope_node_mount_client::ScopeNodeMountClient::with_interceptor(
                channel.clone(),
                BearerToken::new("test-token").unwrap(),
            ),
            stop: StopClient::new(channel),
            server,
        }
    }
    pub async fn heartbeat(&mut self) -> f64 {
        self.camera
            .heartbeat(proto::HeartbeatRequest { t0: 0.0 })
            .await
            .unwrap()
            .into_inner()
            .t1
    }
    pub async fn rate(&mut self, axis: proto::Axis, rate: f64) {
        self.mount
            .rate_axis(proto::RateAxisRequest {
                axis: axis as i32,
                deg_per_s: rate,
            })
            .await
            .unwrap();
    }
    pub async fn step(&mut self, seconds: f64, heartbeat: bool) {
        for _ in 0..(seconds * 100.0).round() as usize {
            tokio::time::sleep(Duration::from_millis(10)).await;
            if heartbeat {
                self.heartbeat().await;
            }
        }
    }
    pub async fn at_rest(&self) {
        for axis in [Axis::Azimuth, Axis::Altitude] {
            let state = self.adapter.mount.state(axis).await;
            assert!(!state.moving, "{axis:?}: {state:?}");
            assert_eq!(state.rate_deg_per_s, 0.0);
        }
    }
}
