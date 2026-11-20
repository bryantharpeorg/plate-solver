//! US1 oracles: the spec's exact paused-time deadlines and the emulator's own
//! wire record, statuses and final counts. All comparisons have zero tolerance.
use scope_link::proto::{
    self, scope_node_mount_server::ScopeNodeMount, scope_node_server::ScopeNode,
    scope_node_stop_server::ScopeNodeStop,
};
use scope_node::{
    clock::NodeClock,
    mount::{
        skywatcher::{
            emulator::{Emulator, EmulatorConfig, EmulatorHandle, Exchange, Line, LineHandle},
            mount_link::LineHealth,
            MountLink,
        },
        Axis, Mount,
    },
    service::{NodeControls, NodeService},
    synthetic::SyntheticCamera,
    Settings,
};
use std::{sync::Arc, time::Duration};
use tokio::{task::JoinHandle, time::Instant};
use tonic::{Code, Request};

const AZ: proto::Axis = proto::Axis::Azimuth;
const ALT: proto::Axis = proto::Axis::Altitude;

struct Clock(Instant);
impl NodeClock for Clock {
    fn now(&self) -> f64 {
        self.0.elapsed().as_secs_f64()
    }
}

struct Rig {
    service: NodeService,
    link: Arc<MountLink<Line>>,
    handle: EmulatorHandle,
    line: LineHandle,
    heartbeat: JoinHandle<()>,
}
impl Drop for Rig {
    fn drop(&mut self) {
        self.heartbeat.abort();
    }
}
impl Rig {
    async fn new() -> Self {
        let clock: Arc<dyn NodeClock> = Arc::new(Clock(Instant::now()));
        let (_, handle) = Emulator::new(EmulatorConfig::default(), clock.clone());
        let line = LineHandle::default();
        let board = handle.clone();
        let faults = line.clone();
        let reopen_clock = clock.clone();
        let link = MountLink::start(
            Box::new(move || {
                Ok(Line::new(
                    board.reopen(reopen_clock.clone()),
                    faults.clone(),
                ))
            }),
            clock.clone(),
        );
        tokio::time::sleep(Duration::from_secs(1)).await;
        assert!(link.connected());
        assert_eq!(link.resets(), 0);
        assert_eq!(link.line_health(), LineHealth::default());
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
        let beating = service.clone();
        let heartbeat = tokio::spawn(async move {
            loop {
                beating
                    .heartbeat(Request::new(proto::HeartbeatRequest { t0: 0.0 }))
                    .await
                    .unwrap();
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        });
        tokio::time::sleep(Duration::from_millis(10)).await;
        Self {
            service,
            link,
            handle,
            line,
            heartbeat,
        }
    }
    async fn read(&self, axis: proto::Axis) -> proto::AxisState {
        let state = self
            .service
            .axis_state(Request::new(proto::AxisStateRequest { axis: axis as i32 }))
            .await
            .unwrap()
            .into_inner();
        assert!(self.link.connected());
        assert_eq!(state.mount_resets, 0);
        state
    }
    async fn both(&self) -> [proto::AxisState; 2] {
        [self.read(AZ).await, self.read(ALT).await]
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
    async fn goto(&self, axis: proto::Axis, position: i32) {
        self.service
            .goto_axis(Request::new(proto::GotoAxisRequest {
                axis: axis as i32,
                position,
            }))
            .await
            .unwrap();
    }
    async fn tracking(&self) {
        self.rate(AZ, 0.004).await;
        self.rate(ALT, -0.003).await;
        tokio::time::sleep(Duration::from_secs(2)).await;
        assert_moving(self.both().await, [true, true]);
    }
    async fn slewing_altitude(&self) {
        self.goto(ALT, 90_000).await;
        tokio::time::sleep(Duration::from_secs(2)).await;
        assert_moving(self.both().await, [false, true]);
        self.line.delay_next(":f1", Duration::from_millis(250));
    }
    fn mark(&self) -> usize {
        self.handle.exchanges().len()
    }
    fn since(&self, mark: usize) -> Vec<Exchange> {
        self.handle.exchanges()[mark..].to_vec()
    }
    fn health(&self, stale: u64, last: Option<&str>) {
        assert_eq!(self.link.health().invalid_replies, 0);
        let health = self.link.line_health();
        assert_eq!(health.stale_replies, stale);
        assert_eq!(health.last_stale.as_deref(), last);
    }
    async fn read_rounds(&self, rounds: usize, moving: [bool; 2]) {
        for _ in 0..rounds {
            tokio::time::sleep(Duration::from_millis(300)).await;
            let mark = self.mark();
            assert_moving(self.both().await, moving);
            assert_wire(&self.since(mark), &[":f1", ":j1", ":f2", ":j2"]);
            assert_eq!(self.link.health().invalid_replies, 0);
            assert_eq!(self.link.line_health().stale_replies, 1);
        }
    }
    async fn rate_rounds(&self, azimuth: f64) {
        let mark = self.mark();
        for _ in 0..5 {
            tokio::time::sleep(Duration::from_secs(1)).await;
            self.rate(AZ, azimuth).await;
            self.rate(ALT, -0.003).await;
            tokio::time::sleep(Duration::from_millis(100)).await;
            assert_moving(self.both().await, [true, true]);
        }
        let events = self.since(mark);
        assert_eq!(
            events
                .iter()
                .filter(|e| e.command.starts_with(":J"))
                .count(),
            10
        );
        assert_no_stop(&events);
    }
    async fn rest(&self) {
        let states = tokio::time::timeout(Duration::from_secs(60), async {
            loop {
                tokio::time::sleep(Duration::from_millis(100)).await;
                let states = self.both().await;
                if states.iter().all(|state| !state.moving) {
                    break states;
                }
            }
        })
        .await
        .expect("both slews must finish within 60 s");
        assert_eq!(states[0].position, 90_000);
        assert_eq!(states[1].position, 45_000);
    }
}

fn assert_moving(states: [proto::AxisState; 2], expected: [bool; 2]) {
    assert_eq!(states.map(|state| state.moving), expected);
}
fn assert_wire(events: &[Exchange], expected: &[&str]) {
    assert_eq!(
        events
            .iter()
            .map(|event| &event.command[..3])
            .collect::<Vec<_>>(),
        expected
    );
}
fn assert_no_stop(events: &[Exchange]) {
    assert!(
        events
            .iter()
            .all(|e| !e.command.starts_with(":K") && !e.command.starts_with(":L")),
        "{events:?}"
    );
}
fn assert_stop(events: &[Exchange]) {
    assert_wire(events, &[":K1", ":K2"]);
    assert!(events
        .iter()
        .all(|event| event.reply.as_deref() == Some("=\r")));
}
fn two_delays(line: &LineHandle) {
    line.delay_next(":f1", Duration::from_millis(400));
    line.delay_next(":f1", Duration::from_millis(150));
}

#[tokio::test(start_paused = true)]
async fn s1_late_read_leaves_tracking_and_later_rates_in_step() {
    let rig = Rig::new().await;
    rig.tracking().await;
    rig.line.delay_next(":f1", Duration::from_millis(400));
    let mark = rig.mark();
    let asked = Instant::now();
    assert!(rig.read(AZ).await.moving);
    assert_eq!(asked.elapsed(), Duration::from_millis(400));
    assert_wire(&rig.since(mark), &[":f1", ":f1", ":j1"]);
    rig.health(1, Some("=111\\r"));
    rig.read_rounds(10, [true, true]).await;
    let mark = rig.mark();
    rig.rate(AZ, 0.0041).await;
    assert_wire(&rig.since(mark), &[":f1", ":I1", ":J1"]);
    rig.rate_rounds(0.0041).await;
    rig.health(1, Some("=111\\r"));
}

#[tokio::test(start_paused = true)]
async fn s2_five_timeouts_preserve_motion_and_axis_identity_without_reset() {
    let rig = Rig::new().await;
    rig.rate(AZ, 0.004).await;
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert_moving(rig.both().await, [true, false]);
    rig.line.delay_next(":f1", Duration::from_millis(1600));
    let mark = rig.mark();
    let asked = Instant::now();
    assert!(
        rig.read(AZ).await.moving,
        "lost reading must never look like rest"
    );
    assert_eq!(asked.elapsed(), Duration::from_millis(1500));
    assert_wire(&rig.since(mark), &[":f1", ":f1", ":f1", ":f1", ":f1"]);
    tokio::time::sleep(Duration::from_millis(200)).await;
    let altitude = rig.read(ALT).await;
    assert!(!altitude.moving);
    assert_eq!(altitude.rate_deg_per_s, 0.0);
    assert_eq!(altitude.position, 0);
    assert!(rig.read(AZ).await.moving);
    tokio::time::sleep(Duration::from_secs(5)).await;
    assert!(rig.read(AZ).await.moving);
    assert_eq!(rig.link.resets(), 0);
    rig.health(1, Some(&"=111\\r".repeat(5)));
}

#[tokio::test(start_paused = true)]
async fn s3_abandoned_read_discards_arrived_status_before_other_axis() {
    let rig = Rig::new().await;
    rig.slewing_altitude().await;
    assert!(
        tokio::time::timeout(Duration::from_millis(200), rig.read(AZ))
            .await
            .is_err()
    );
    tokio::time::sleep(Duration::from_millis(100)).await;
    for _ in 0..2 {
        assert!(rig.read(ALT).await.moving);
    }
    let events = rig.handle.exchanges();
    let status = events.iter().rev().find(|e| e.command == ":f2\r").unwrap();
    assert_eq!(status.reply.as_deref().unwrap().as_bytes()[2], b'1');
    rig.health(1, Some("=001\\r"));
}

#[tokio::test(start_paused = true)]
async fn s4_abandoned_read_waits_only_until_original_reply_arrives() {
    let rig = Rig::new().await;
    rig.slewing_altitude().await;
    assert!(
        tokio::time::timeout(Duration::from_millis(200), rig.read(AZ))
            .await
            .is_err()
    );
    let asked = Instant::now();
    assert!(rig.read(ALT).await.moving);
    assert_eq!(asked.elapsed(), Duration::from_millis(50));
    rig.read_rounds(5, [false, true]).await;
    rig.health(1, Some("=001\\r"));
}

#[tokio::test(start_paused = true)]
async fn s5_stop_cancels_rate_reads_out_its_reply_and_allows_tracking_again() {
    let rig = Rig::new().await;
    rig.tracking().await;
    rig.line.delay_next(":f1", Duration::from_millis(50));
    let service = rig.service.clone();
    let moving = tokio::spawn(async move {
        service
            .rate_axis(Request::new(proto::RateAxisRequest {
                axis: AZ as i32,
                deg_per_s: 0.0041,
            }))
            .await
    });
    tokio::time::sleep(Duration::from_millis(20)).await;
    let mark = rig.mark();
    let asked = Instant::now();
    rig.service
        .stop_all(Request::new(proto::StopAllRequest {}))
        .await
        .unwrap();
    assert_eq!(asked.elapsed(), Duration::from_millis(30));
    let error = moving.await.unwrap().unwrap_err();
    assert_eq!(error.code(), Code::Aborted);
    assert_eq!(error.message(), "mount command discarded by Stop");
    assert_stop(&rig.since(mark));
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_moving(rig.both().await, [false, false]);
    rig.rate_rounds(0.004).await;
    rig.health(1, Some("=111\\r"));
}

#[tokio::test(start_paused = true)]
async fn s6_stop_behind_lost_reply_waits_to_abandoned_frames_deadline() {
    let rig = Rig::new().await;
    rig.tracking().await;
    rig.handle.drop_replies(b'f', 1, 1);
    let service = rig.service.clone();
    let sent = Instant::now();
    let moving = tokio::spawn(async move {
        service
            .rate_axis(Request::new(proto::RateAxisRequest {
                axis: AZ as i32,
                deg_per_s: 0.0041,
            }))
            .await
    });
    tokio::time::sleep(Duration::from_millis(20)).await;
    let mark = rig.mark();
    let asked = Instant::now();
    rig.service
        .stop_all(Request::new(proto::StopAllRequest {}))
        .await
        .unwrap();
    assert_eq!(asked.elapsed(), Duration::from_millis(280));
    assert_eq!(sent.elapsed(), Duration::from_millis(300));
    assert_stop(&rig.since(mark));
    assert_eq!(moving.await.unwrap().unwrap_err().code(), Code::Aborted);
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_moving(rig.both().await, [false, false]);
    rig.health(0, None);
}

#[tokio::test(start_paused = true)]
async fn s7_gotos_after_abandoned_read_reach_both_targets_without_stops() {
    let rig = Rig::new().await;
    assert_moving(rig.both().await, [false, false]);
    rig.line.delay_next(":f1", Duration::from_millis(250));
    let mark = rig.mark();
    assert!(
        tokio::time::timeout(Duration::from_millis(200), rig.read(AZ))
            .await
            .is_err()
    );
    let asked = Instant::now();
    rig.goto(AZ, 90_000).await;
    rig.goto(ALT, 45_000).await;
    assert_eq!(asked.elapsed(), Duration::from_millis(50));
    assert_wire(
        &rig.since(mark),
        &[
            ":f1", ":f1", ":j1", ":G1", ":S1", ":J1", ":f2", ":j2", ":G2", ":S2", ":J2",
        ],
    );
    rig.rest().await;
    assert_no_stop(&rig.since(mark));
    rig.health(1, Some("=001\\r"));
}

#[tokio::test(start_paused = true)]
async fn s9_overlong_ack_tail_is_discarded_before_its_retry() {
    let rig = Rig::new().await;
    rig.tracking().await;
    rig.handle
        .inject_reply(b'I', 1, "=0123456789ABCDEF0123\r", 1);
    let mark = rig.mark();
    rig.rate(AZ, 0.0041).await;
    assert_wire(&rig.since(mark), &[":f1", ":I1", ":I1", ":J1"]);
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_moving(rig.both().await, [true, true]);
    rig.health(1, Some("0123\\r"));
}

#[tokio::test(start_paused = true)]
async fn s10_bare_emulator_discard_does_not_record_a_baud_change() {
    let clock: Arc<dyn NodeClock> = Arc::new(Clock(Instant::now()));
    let (_, handle) = Emulator::new(EmulatorConfig::default(), clock.clone());
    let board = handle.clone();
    let reopen_clock = clock.clone();
    let link = MountLink::start(
        Box::new(move || Ok(board.reopen(reopen_clock.clone()))),
        clock,
    );
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert!(link.connected());
    handle.inject_reply(b'f', 1, "=0123456789ABCDEF0123\r", 1);
    let mark = handle.exchanges().len();
    let bauds = handle.baud_attempts();
    assert!(!link.state(Axis::Azimuth).await.moving);
    assert_wire(&handle.exchanges()[mark..], &[":f1", ":f1", ":j1"]);
    assert_eq!(link.health().invalid_replies, 0);
    assert_eq!(link.line_health().stale_replies, 1);
    assert_eq!(link.line_health().last_stale.as_deref(), Some("0123\\r"));
    assert_eq!(handle.baud_attempts(), bauds);
}

#[tokio::test(start_paused = true)]
async fn s11_cancelling_read_out_keeps_original_exchange_and_deadline() {
    let rig = Rig::new().await;
    rig.slewing_altitude().await;
    let sent = Instant::now();
    assert!(
        tokio::time::timeout(Duration::from_millis(100), rig.read(AZ))
            .await
            .is_err()
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(50), rig.read(ALT))
            .await
            .is_err()
    );
    let mark = rig.mark();
    let asked = Instant::now();
    assert!(rig.read(ALT).await.moving);
    assert_eq!(asked.elapsed(), Duration::from_millis(100));
    assert_eq!(sent.elapsed(), Duration::from_millis(250));
    assert_wire(&rig.since(mark), &[":f2", ":j2"]);
    assert!(!rig.read(AZ).await.moving);
    rig.health(1, Some("=001\\r"));
}

#[tokio::test(start_paused = true)]
async fn s12_retry_reads_own_delayed_reply_before_position_transmission() {
    let rig = Rig::new().await;
    rig.tracking().await;
    two_delays(&rig.line);
    let mark = rig.mark();
    let asked = Instant::now();
    assert!(rig.read(AZ).await.moving);
    assert_eq!(asked.elapsed(), Duration::from_millis(450));
    assert_wire(&rig.since(mark), &[":f1", ":f1", ":j1"]);
    rig.health(1, Some("=111\\r"));
    rig.read_rounds(3, [true, true]).await;
}

#[tokio::test(start_paused = true)]
async fn s13_retry_reads_own_delayed_reply_before_rate_acknowledgement() {
    let rig = Rig::new().await;
    rig.tracking().await;
    two_delays(&rig.line);
    let mark = rig.mark();
    let asked = Instant::now();
    rig.rate(AZ, 0.0041).await;
    assert_eq!(asked.elapsed(), Duration::from_millis(450));
    assert_wire(&rig.since(mark), &[":f1", ":f1", ":I1", ":J1"]);
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_moving(rig.both().await, [true, true]);
    rig.health(1, Some("=111\\r"));
}

#[tokio::test(start_paused = true)]
async fn s14_retry_reads_own_delayed_reply_before_goto_position() {
    let rig = Rig::new().await;
    let states = rig.both().await;
    assert_eq!([states[0].position, states[1].position], [0, 0]);
    assert_moving(states, [false, false]);
    two_delays(&rig.line);
    let mark = rig.mark();
    let asked = Instant::now();
    rig.goto(AZ, 90_000).await;
    rig.goto(ALT, 45_000).await;
    assert_eq!(asked.elapsed(), Duration::from_millis(450));
    assert_wire(
        &rig.since(mark),
        &[
            ":f1", ":f1", ":j1", ":G1", ":S1", ":J1", ":f2", ":j2", ":G2", ":S2", ":J2",
        ],
    );
    rig.rest().await;
    assert_no_stop(&rig.since(mark));
    rig.health(1, Some("=001\\r"));
}

#[tokio::test(start_paused = true)]
async fn s15_lost_reply_pays_one_extra_bounded_wait_after_answered_retry() {
    let rig = Rig::new().await;
    rig.tracking().await;
    rig.handle.drop_replies(b'f', 1, 1);
    let mark = rig.mark();
    let asked = Instant::now();
    assert!(rig.read(AZ).await.moving);
    assert_eq!(asked.elapsed(), Duration::from_millis(600));
    assert_eq!(
        asked.elapsed() - Duration::from_millis(300),
        Duration::from_millis(300)
    );
    assert_wire(&rig.since(mark), &[":f1", ":f1", ":j1"]);
    rig.health(0, None);
}
