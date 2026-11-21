//! US1-S1–S8: the emulator's wire record and axis readings, a stand-in's
//! record, and the spec's literal words and deadlines are the oracles.
//! Paused-time durations, command bytes and replies have zero tolerance.
use scope_link::proto::{
    self, scope_node_mount_server::ScopeNodeMount, scope_node_server::ScopeNode,
    scope_node_stop_server::ScopeNodeStop,
};
use scope_node::{
    clock::NodeClock,
    mount::{
        skywatcher::{
            emulator::{Emulator, EmulatorConfig, EmulatorHandle, Exchange},
            MountLink,
        },
        Axis, AxisInfo, AxisState, Mount, MountFuture,
    },
    service::{NodeControls, NodeService},
    shutdown::Shutdown,
    synthetic::SyntheticCamera,
    Settings,
};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{task::JoinHandle, time::Instant};
use tonic::{Code, Request, Status};

const AZ: proto::Axis = proto::Axis::Azimuth;
const ALT: proto::Axis = proto::Axis::Altitude;

struct Clock(Instant);
impl NodeClock for Clock {
    fn now(&self) -> f64 {
        self.0.elapsed().as_secs_f64()
    }
}
fn clock() -> Arc<dyn NodeClock> {
    Arc::new(Clock(Instant::now()))
}

struct Heartbeat(JoinHandle<()>);
impl Drop for Heartbeat {
    fn drop(&mut self) {
        self.0.abort();
    }
}
fn beat(service: &NodeService) -> Heartbeat {
    let service = service.clone();
    Heartbeat(tokio::spawn(async move {
        loop {
            service
                .heartbeat(Request::new(proto::HeartbeatRequest { t0: 0.0 }))
                .await
                .unwrap();
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }))
}

struct Rig {
    service: NodeService,
    link: Arc<MountLink<Emulator>>,
    handle: EmulatorHandle,
    _heartbeat: Heartbeat,
}
impl Rig {
    async fn new(powered: bool) -> Self {
        let clock = clock();
        let (_, handle) = Emulator::new(EmulatorConfig::default(), clock.clone());
        if !powered {
            handle.power_off();
        }
        let board = handle.clone();
        let reopen_clock = clock.clone();
        let link = MountLink::start(
            Box::new(move || Ok(board.reopen(reopen_clock.clone()))),
            clock.clone(),
        );
        tokio::time::sleep(Duration::from_secs(1)).await;
        assert_eq!(link.connected(), powered);
        let controls = NodeControls::default();
        controls.stop_frames();
        let service = NodeService::with_controls_and_mount(
            SyntheticCamera::new(clock.clone()),
            clock,
            "test-token",
            Settings::default(),
            controls,
            Some(link.clone()),
        )
        .unwrap();
        let heartbeat = beat(&service);
        tokio::time::sleep(Duration::from_millis(10)).await;
        Self {
            service,
            link,
            handle,
            _heartbeat: heartbeat,
        }
    }
    async fn goto(&self, axis: proto::Axis, position: i32) {
        self.service
            .goto_axis(Request::new(proto::GotoAxisRequest {
                axis: axis as i32,
                position,
            }))
            .await
            .unwrap();
    }
    async fn rate(&self, axis: proto::Axis, deg_per_s: f64) {
        self.service
            .rate_axis(Request::new(proto::RateAxisRequest {
                axis: axis as i32,
                deg_per_s,
            }))
            .await
            .unwrap();
    }
    async fn both_moving(&self) -> [bool; 2] {
        let mut moving = [false; 2];
        for (index, axis) in [AZ, ALT].into_iter().enumerate() {
            moving[index] = self
                .service
                .axis_state(Request::new(proto::AxisStateRequest { axis: axis as i32 }))
                .await
                .unwrap()
                .into_inner()
                .moving;
        }
        moving
    }
    fn mark(&self) -> usize {
        self.handle.exchanges().len()
    }
    fn since(&self, mark: usize) -> Vec<Exchange> {
        self.handle.exchanges()[mark..].to_vec()
    }
    async fn confirm_and_refuse_moves(&self) {
        let mark = self.mark();
        let started = Instant::now();
        assert_eq!(self.service.shut_down().await, Shutdown::Confirmed);
        assert_eq!(started.elapsed(), Duration::from_millis(0));
        assert_exchanges(
            &self.since(mark),
            &[(":K1\r", Some("=\r")), (":K2\r", Some("=\r"))],
        );
        refused_goto(&self.service).await;
        refused_rate(&self.service).await;
        tokio::time::sleep(Duration::from_secs(2)).await;
        assert_eq!(self.both_moving().await, [false, false]);
        assert_no_motion(&self.since(mark));
        assert_eq!(
            Shutdown::Confirmed.to_string(),
            "the mount confirmed the stop"
        );
    }
}

fn assert_exchanges(events: &[Exchange], expected: &[(&str, Option<&str>)]) {
    assert_eq!(
        events
            .iter()
            .map(|e| (e.command.as_str(), e.reply.as_deref()))
            .collect::<Vec<_>>(),
        expected
    );
}
fn assert_no_motion(events: &[Exchange]) {
    assert!(
        events.iter().all(|e| !matches!(
            e.command.as_bytes().get(1),
            Some(b'G' | b'S' | b'H' | b'M' | b'I' | b'J')
        )),
        "{events:?}"
    );
}
fn assert_refused(error: Status) {
    assert_eq!(error.code(), Code::Aborted);
    assert_eq!(error.message(), "mount command discarded by Stop");
}
async fn refused_goto(service: &NodeService) {
    assert_refused(
        service
            .goto_axis(Request::new(proto::GotoAxisRequest {
                axis: ALT as i32,
                position: 50_000,
            }))
            .await
            .unwrap_err(),
    );
}
async fn refused_rate(service: &NodeService) {
    assert_refused(
        service
            .rate_axis(Request::new(proto::RateAxisRequest {
                axis: AZ as i32,
                deg_per_s: 0.5,
            }))
            .await
            .unwrap_err(),
    );
}

#[tokio::test(start_paused = true)]
async fn s1_shutdown_stops_gotos_and_permanently_refuses_moves() {
    let rig = Rig::new(true).await;
    rig.goto(AZ, 1_000_000).await;
    rig.goto(ALT, 300_000).await;
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert_eq!(rig.both_moving().await, [true, true]);
    rig.confirm_and_refuse_moves().await;
}

#[tokio::test(start_paused = true)]
async fn s2_shutdown_stops_rates_and_permanently_refuses_moves() {
    let rig = Rig::new(true).await;
    rig.rate(AZ, 1.0).await;
    rig.rate(ALT, -1.0).await;
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert_eq!(rig.both_moving().await, [true, true]);
    rig.confirm_and_refuse_moves().await;
}

#[tokio::test(start_paused = true)]
async fn s3_shutdown_carries_through_an_existing_stop_then_runs_its_own() {
    let rig = Rig::new(true).await;
    rig.rate(AZ, 1.0).await;
    rig.handle.drop_replies(b'K', 1, 5);
    let mark = rig.mark();
    let started = Instant::now();
    let service = rig.service.clone();
    let stopping = tokio::spawn(async move {
        service
            .stop_all(Request::new(proto::StopAllRequest {}))
            .await
    });
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(rig.service.shut_down().await, Shutdown::Confirmed);
    assert_eq!(started.elapsed(), Duration::from_millis(1_500));
    stopping.await.unwrap().unwrap();
    assert_exchanges(
        &rig.since(mark),
        &[
            (":K1\r", None),
            (":K1\r", None),
            (":K1\r", None),
            (":K1\r", None),
            (":K1\r", None),
            (":K2\r", Some("=\r")),
            (":L1\r", Some("=\r")),
            (":K1\r", Some("=\r")),
            (":K2\r", Some("=\r")),
        ],
    );
}

#[tokio::test(start_paused = true)]
async fn s4_power_loss_during_a_rate_is_not_confirmed_after_all_retries() {
    let rig = Rig::new(true).await;
    rig.rate(AZ, 1.0).await;
    rig.handle.power_off();
    let mark = rig.mark();
    let started = Instant::now();
    assert_eq!(rig.service.shut_down().await, Shutdown::NotConfirmed);
    assert_eq!(started.elapsed(), Duration::from_millis(6_000));
    let expected: Vec<_> = [":K1\r", ":K2\r", ":L1\r", ":L2\r"]
        .into_iter()
        .flat_map(|command| [(command, None); 5])
        .collect();
    assert_exchanges(&rig.since(mark), &expected);
    assert_eq!(
        Shutdown::NotConfirmed.to_string(),
        "the mount did not confirm the stop"
    );
}

#[tokio::test(start_paused = true)]
async fn s5_a_lost_link_tries_each_stop_once_on_its_kept_stream() {
    let rig = Rig::new(true).await;
    rig.rate(AZ, 1.0).await;
    rig.handle.power_off();
    for _ in 0..3 {
        rig.service
            .axis_state(Request::new(proto::AxisStateRequest { axis: AZ as i32 }))
            .await
            .unwrap();
    }
    let error = rig
        .service
        .axis_state(Request::new(proto::AxisStateRequest { axis: AZ as i32 }))
        .await
        .unwrap_err();
    assert_eq!(error.code(), Code::FailedPrecondition);
    assert_eq!(error.message(), "mount not connected");
    assert!(!rig.link.connected());
    let mark = rig.mark();
    let started = Instant::now();
    assert_eq!(rig.service.shut_down().await, Shutdown::NotConfirmed);
    assert_eq!(started.elapsed(), Duration::from_millis(1_200));
    let stops: Vec<_> = rig
        .since(mark)
        .into_iter()
        .filter(|e| e.command.starts_with(":K") || e.command.starts_with(":L"))
        .collect();
    assert_exchanges(
        &stops,
        &[
            (":K1\r", None),
            (":K2\r", None),
            (":L1\r", None),
            (":L2\r", None),
        ],
    );
}

#[derive(Default)]
struct NeverStops(Mutex<Vec<String>>);
impl Mount for NeverStops {
    fn axes(&self) -> [AxisInfo; 2] {
        [AxisInfo {
            counts_per_revolution: 1_620_000,
            top_rate_deg_per_s: 3.4,
            can_goto: true,
        }; 2]
    }
    fn state(&self, _: Axis) -> MountFuture<'_, AxisState> {
        Box::pin(async {
            AxisState {
                position: 0,
                rate_deg_per_s: 0.0,
                moving: false,
            }
        })
    }
    fn goto(&self, axis: Axis, position: i32) -> MountFuture<'_, ()> {
        Box::pin(async move {
            self.0
                .lock()
                .unwrap()
                .push(format!("goto {axis:?} {position}"));
        })
    }
    fn rate(&self, axis: Axis, rate: f64) -> MountFuture<'_, ()> {
        Box::pin(async move {
            self.0.lock().unwrap().push(format!("rate {axis:?} {rate}"));
        })
    }
    fn stop_all(&self) -> MountFuture<'_, ()> {
        Box::pin(std::future::pending())
    }
    fn stop_confirmed(&self) -> MountFuture<'_, bool> {
        Box::pin(std::future::pending())
    }
}

#[tokio::test(start_paused = true)]
async fn s6_an_unresponsive_adapter_times_out_with_every_later_move_refused() {
    let clock = clock();
    let mount = Arc::new(NeverStops::default());
    let service = NodeService::with_mount(
        SyntheticCamera::new(clock.clone()),
        clock,
        "test-token",
        Settings::default(),
        mount.clone(),
    )
    .unwrap();
    let _heartbeat = beat(&service);
    tokio::time::sleep(Duration::from_millis(10)).await;
    service
        .goto_axis(Request::new(proto::GotoAxisRequest {
            axis: AZ as i32,
            position: 1_000,
        }))
        .await
        .unwrap();
    let started = Instant::now();
    let shutting_down = service.clone();
    let shutdown = tokio::spawn(async move { shutting_down.shut_down().await });
    tokio::time::sleep(Duration::from_millis(100)).await;
    refused_goto(&service).await;
    refused_rate(&service).await;
    assert_eq!(shutdown.await.unwrap(), Shutdown::TimedOut);
    assert_eq!(started.elapsed(), Duration::from_millis(8_000));
    refused_goto(&service).await;
    assert_eq!(*mount.0.lock().unwrap(), ["goto Azimuth 1000"]);
    assert_eq!(
        Shutdown::TimedOut.to_string(),
        "the mount did not confirm the stop within 8 s"
    );
}

#[tokio::test(start_paused = true)]
async fn s7_no_mount_needs_no_stop_or_wait() {
    let clock = clock();
    let service = NodeService::new(
        SyntheticCamera::new(clock.clone()),
        clock,
        "test-token",
        Settings::default(),
    )
    .unwrap();
    let started = Instant::now();
    assert_eq!(service.shut_down().await, Shutdown::NoMount);
    assert_eq!(started.elapsed(), Duration::from_millis(0));
    assert_eq!(Shutdown::NoMount.to_string(), "no mount to stop");
}

#[tokio::test(start_paused = true)]
async fn s8_a_never_connected_mount_answers_at_once_without_sending_a_stop() {
    let rig = Rig::new(false).await;
    let mark = rig.mark();
    let started = Instant::now();
    assert_eq!(rig.service.shut_down().await, Shutdown::NotConfirmed);
    assert_eq!(started.elapsed(), Duration::from_millis(0));
    tokio::time::sleep(Duration::from_secs(3)).await;
    let events = rig.since(mark);
    assert!(events.iter().any(|e| e.command == ":e1\r"), "{events:?}");
    assert!(
        events
            .iter()
            .all(|e| !e.command.starts_with(":K") && !e.command.starts_with(":L")),
        "{events:?}"
    );
}
