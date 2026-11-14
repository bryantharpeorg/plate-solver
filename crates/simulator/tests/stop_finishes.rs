//! Oracles: SimMount's mechanics and the adapter's completion count. Stops take
//! 1 s of injected virtual time; caller deadlines are 200 ms, rest is exactly zero.
use scope_link::{memory, proto, BearerToken};
use scope_node::{
    clock::NodeClock,
    mount::{Axis, AxisInfo, AxisState, Mount, MountFuture},
    service::{NodeControls, NodeService},
    synthetic::SyntheticCamera,
    Settings,
};
use simulator::{mount::SimMount, VirtualClock};
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::{task::JoinHandle, time};
use tonic::{service::interceptor::InterceptedService, transport::Channel};

type MountClient =
    proto::scope_node_mount_client::ScopeNodeMountClient<InterceptedService<Channel, BearerToken>>;
type StopClient = proto::scope_node_stop_client::ScopeNodeStopClient<Channel>;

struct Clock(VirtualClock);
impl NodeClock for Clock {
    fn now(&self) -> f64 {
        self.0.node_now()
    }
}

struct SlowMount {
    mount: SimMount,
    started: AtomicUsize,
    completed: AtomicUsize,
}
impl Mount for SlowMount {
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
            self.started.fetch_add(1, Ordering::SeqCst);
            time::sleep(Duration::from_secs(1)).await;
            self.mount.stop_all().await;
            self.completed.fetch_add(1, Ordering::SeqCst);
        })
    }
}

struct UnconfirmedMount(SimMount);
impl Mount for UnconfirmedMount {
    fn axes(&self) -> [AxisInfo; 2] {
        self.0.axes()
    }
    fn state(&self, axis: Axis) -> MountFuture<'_, AxisState> {
        self.0.state(axis)
    }
    fn goto(&self, axis: Axis, position: i32) -> MountFuture<'_, ()> {
        self.0.goto(axis, position)
    }
    fn rate(&self, axis: Axis, rate: f64) -> MountFuture<'_, ()> {
        self.0.rate(axis, rate)
    }
    fn stop_all(&self) -> MountFuture<'_, ()> {
        self.0.stop_all()
    }
    fn stop_confirmed(&self) -> MountFuture<'_, bool> {
        Box::pin(async move {
            self.stop_all().await;
            false
        })
    }
}

struct Rig {
    mount: MountClient,
    stop: StopClient,
    server: JoinHandle<()>,
    heartbeat: JoinHandle<()>,
}
impl Drop for Rig {
    fn drop(&mut self) {
        self.heartbeat.abort();
        self.server.abort();
    }
}
impl Rig {
    async fn new(clock: VirtualClock, mount: Option<Arc<dyn Mount>>) -> Self {
        let clock: Arc<dyn NodeClock> = Arc::new(Clock(clock));
        let service = NodeService::with_controls_and_mount(
            SyntheticCamera::new(clock.clone()),
            clock,
            "test-token",
            Settings::default(),
            NodeControls::default(),
            mount,
        )
        .unwrap();
        let (connect, io) = memory::channel_pair();
        let routes = service.routes();
        let server = tokio::spawn(async move {
            memory::serve_routes_on(io, routes).await.unwrap();
        });
        let channel = connect.await.unwrap();
        let mut camera = proto::scope_node_client::ScopeNodeClient::with_interceptor(
            channel.clone(),
            BearerToken::new("test-token").unwrap(),
        );
        // Keep the real watchdog healthy, so only the requested stops can finish.
        let heartbeat = tokio::spawn(async move {
            loop {
                camera
                    .heartbeat(proto::HeartbeatRequest { t0: 0.0 })
                    .await
                    .unwrap();
                time::sleep(Duration::from_millis(100)).await;
            }
        });
        Self {
            mount: proto::scope_node_mount_client::ScopeNodeMountClient::with_interceptor(
                channel.clone(),
                BearerToken::new("test-token").unwrap(),
            ),
            // Stop deliberately has no bearer token.
            stop: StopClient::new(channel),
            server,
            heartbeat,
        }
    }
    async fn rate(&mut self, axis: proto::Axis, rate: f64) {
        self.mount
            .rate_axis(proto::RateAxisRequest {
                axis: axis as i32,
                deg_per_s: rate,
            })
            .await
            .unwrap();
    }
    async fn start_axes(&mut self, mount: &SimMount) {
        self.rate(proto::Axis::Azimuth, 1.0).await;
        self.rate(proto::Axis::Altitude, -1.0).await;
        time::sleep(Duration::from_millis(500)).await;
        for axis in [Axis::Azimuth, Axis::Altitude] {
            assert!(mount.state(axis).await.moving, "{axis:?} must be moving");
        }
    }
}

async fn assert_rest(mount: &SimMount) {
    for axis in [Axis::Azimuth, Axis::Altitude] {
        let state = mount.state(axis).await;
        assert!(!state.moving, "{axis:?}: {state:?}");
        assert_eq!(state.rate_deg_per_s, 0.0);
    }
}

async fn cancelled_stops_finish(count: usize) {
    let clock = VirtualClock::new(1_768_528_800.0);
    let adapter = Arc::new(SlowMount {
        mount: SimMount::new(clock.clone()),
        started: AtomicUsize::new(0),
        completed: AtomicUsize::new(0),
    });
    let mut rig = Rig::new(clock, Some(adapter.clone())).await;
    rig.start_axes(&adapter.mount).await;
    let first = time::Instant::now();
    for attempt in 0..count {
        time::sleep_until(first + Duration::from_millis(250 * attempt as u64)).await;
        assert!(time::timeout(
            Duration::from_millis(200),
            rig.stop.stop_all(proto::StopAllRequest {}),
        )
        .await
        .is_err());
        assert_eq!(adapter.completed.load(Ordering::SeqCst), 0);
    }
    assert_eq!(adapter.started.load(Ordering::SeqCst), 1);
    time::sleep(Duration::from_secs(count as u64 + 1)).await;
    assert_eq!(adapter.started.load(Ordering::SeqCst), count);
    assert_eq!(adapter.completed.load(Ordering::SeqCst), count);
    assert_rest(&adapter.mount).await;
    // This traverses the queue again: a stranded `stopping` flag rejects it.
    rig.rate(proto::Axis::Azimuth, -1.0).await;
    time::sleep(Duration::from_millis(500)).await;
    let state = adapter.mount.state(Axis::Azimuth).await;
    assert!(state.moving);
    assert_eq!(state.rate_deg_per_s, -1.0);
    assert_eq!(adapter.completed.load(Ordering::SeqCst), count);
}

#[tokio::test(start_paused = true)]
async fn a_stop_finishes_after_its_caller_times_out() {
    cancelled_stops_finish(1).await;
}

#[tokio::test(start_paused = true)]
async fn three_overlapping_timed_out_stops_all_finish() {
    cancelled_stops_finish(3).await;
}

#[tokio::test(start_paused = true)]
async fn an_unconfirmed_stop_returns_unavailable_with_the_exact_reason() {
    let clock = VirtualClock::new(1_768_528_800.0);
    let mount = Arc::new(UnconfirmedMount(SimMount::new(clock.clone())));
    let mut rig = Rig::new(clock, Some(mount)).await;
    let error = rig
        .stop
        .stop_all(proto::StopAllRequest {})
        .await
        .unwrap_err();
    assert_eq!(error.code(), tonic::Code::Unavailable);
    assert_eq!(error.message(), "the mount did not confirm the stop");
    rig.rate(proto::Axis::Azimuth, 1.0).await;
}

#[tokio::test(start_paused = true)]
async fn the_default_confirmation_stops_both_axes_and_answers_ok() {
    let clock = VirtualClock::new(1_768_528_800.0);
    let mount = Arc::new(SimMount::new(clock.clone()));
    let mut rig = Rig::new(clock, Some(mount.clone())).await;
    rig.start_axes(&mount).await;
    rig.stop.stop_all(proto::StopAllRequest {}).await.unwrap();
    time::sleep(Duration::from_millis(500)).await;
    assert_rest(&mount).await;
}

#[tokio::test(start_paused = true)]
async fn a_node_without_a_mount_answers_ok() {
    let mut rig = Rig::new(VirtualClock::new(1_768_528_800.0), None).await;
    rig.stop.stop_all(proto::StopAllRequest {}).await.unwrap();
}
