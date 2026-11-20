//! US2 oracles: the spec's exact paused-time deadlines and the emulator's own
//! wire record, moving flags and final counts. All comparisons have zero tolerance.
use scope_link::proto::{
    self, scope_node_mount_server::ScopeNodeMount, scope_node_server::ScopeNode,
};
use scope_node::{
    clock::NodeClock,
    mount::{
        skywatcher::{
            emulator::{Emulator, EmulatorConfig, EmulatorHandle, Exchange, Line, LineHandle},
            mount_link::LineHealth,
            MountLink,
        },
        Mount,
    },
    service::{NodeControls, NodeService},
    synthetic::SyntheticCamera,
    Settings,
};
use std::{sync::Arc, time::Duration};
use tokio::{task::JoinHandle, time::Instant};
use tonic::Request;

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
        assert_eq!(state.mount_resets, self.link.resets());
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
    fn mark(&self) -> usize {
        self.handle.exchanges().len()
    }
    fn since(&self, mark: usize) -> Vec<Exchange> {
        self.handle.exchanges()[mark..].to_vec()
    }
    fn health(&self, unacknowledged: u64, halted: u64) {
        assert_eq!(self.link.health().invalid_replies, 0);
        let health = self.link.line_health();
        assert_eq!(health.unacknowledged_starts, unacknowledged);
        assert_eq!(health.halted_moves, halted);
    }
    fn stale(&self, count: u64, last: &str) {
        let health = self.link.line_health();
        assert_eq!(health.stale_replies, count);
        assert_eq!(health.last_stale.as_deref(), Some(last));
    }
    async fn rest(&self) -> [proto::AxisState; 2] {
        tokio::time::timeout(Duration::from_secs(60), async {
            loop {
                tokio::time::sleep(Duration::from_millis(100)).await;
                let states = self.both().await;
                if states.iter().all(|state| !state.moving) {
                    break states;
                }
            }
        })
        .await
        .expect("both slews must finish within 60 s")
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

async fn tracking_start(reply: Option<&str>, elapsed: Duration) {
    let rig = Rig::new().await;
    rig.tracking().await;
    match reply {
        Some(reply) => rig.handle.inject_reply(b'J', 1, reply, 1),
        None => rig.handle.drop_replies(b'J', 1, 1),
    }
    let mark = rig.mark();
    let asked = Instant::now();
    rig.rate(AZ, 0.0041).await;
    assert_eq!(asked.elapsed(), elapsed);
    assert_wire(&rig.since(mark), &[":f1", ":I1", ":J1", ":f1"]);
    for _ in 0..60 {
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_moving(rig.both().await, [true, true]);
    }
    assert_no_stop(&rig.since(mark));
    rig.health(1, 0);
}

#[tokio::test(start_paused = true)]
async fn s1_start_acknowledgement_carrying_data_keeps_tracking() {
    tracking_start(Some("=0\r"), Duration::ZERO).await;
}

#[tokio::test(start_paused = true)]
async fn s2_unframeable_start_acknowledgement_keeps_tracking() {
    tracking_start(Some("?\r"), Duration::ZERO).await;
}

#[tokio::test(start_paused = true)]
async fn s3_missing_start_acknowledgement_keeps_tracking_after_300_ms() {
    tracking_start(None, Duration::from_millis(300)).await;
}

#[tokio::test(start_paused = true)]
async fn s4_start_lost_before_the_board_stops_both_axes() {
    let rig = Rig::new().await;
    rig.rate(ALT, -0.003).await;
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert_moving(rig.both().await, [false, true]);
    rig.line.lose_next(":J1");
    let mark = rig.mark();
    let asked = Instant::now();
    rig.rate(AZ, 0.004).await;
    assert_eq!(asked.elapsed(), Duration::from_millis(300));
    assert_wire(
        &rig.since(mark),
        &[":f1", ":G1", ":I1", ":f1", ":K1", ":K2"],
    );
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_moving(rig.both().await, [false, false]);
    rig.health(1, 1);
}

#[tokio::test(start_paused = true)]
async fn s5_garbled_azimuth_start_allows_both_slews_to_finish() {
    let rig = Rig::new().await;
    let initial = rig.both().await;
    assert_eq!(
        initial.map(|state| (state.position, state.moving)),
        [(0, false); 2]
    );
    rig.handle.inject_reply(b'J', 1, "=0\r", 1);
    let mark = rig.mark();
    rig.goto(AZ, 90_000).await;
    rig.goto(ALT, 45_000).await;
    assert_wire(
        &rig.since(mark),
        &[
            ":f1", ":j1", ":G1", ":S1", ":J1", ":f1", ":f2", ":j2", ":G2", ":S2", ":J2",
        ],
    );
    assert_eq!(
        rig.rest().await.map(|state| state.position),
        [90_000, 45_000]
    );
    assert_no_stop(&rig.since(mark));
    rig.health(1, 0);
}

#[tokio::test(start_paused = true)]
async fn s6_missing_altitude_start_acknowledgement_leaves_azimuth_slewing() {
    let rig = Rig::new().await;
    let initial = rig.both().await;
    assert_eq!(
        initial.map(|state| (state.position, state.moving)),
        [(0, false); 2]
    );
    rig.handle.drop_replies(b'J', 2, 1);
    let mark = rig.mark();
    let asked = Instant::now();
    rig.goto(AZ, 90_000).await;
    rig.goto(ALT, 45_000).await;
    assert_eq!(asked.elapsed(), Duration::from_millis(300));
    assert_wire(
        &rig.since(mark),
        &[
            ":f1", ":j1", ":G1", ":S1", ":J1", ":f2", ":j2", ":G2", ":S2", ":J2", ":f2",
        ],
    );
    assert_eq!(
        rig.rest().await.map(|state| state.position),
        [90_000, 45_000]
    );
    assert_no_stop(&rig.since(mark));
    rig.health(1, 0);
}

#[tokio::test(start_paused = true)]
async fn s7_move_finished_before_status_read_still_stops_both_axes() {
    let rig = Rig::new().await;
    rig.goto(ALT, 90_000).await;
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert!(rig.read(ALT).await.moving);
    rig.handle.drop_replies(b'J', 1, 1);
    let mark = rig.mark();
    let asked = Instant::now();
    rig.goto(AZ, 100).await;
    assert_eq!(asked.elapsed(), Duration::from_millis(300));
    assert_wire(
        &rig.since(mark),
        &[":f1", ":j1", ":G1", ":S1", ":J1", ":f1", ":K1", ":K2"],
    );
    let states = rig.rest().await;
    assert_eq!(states[0].position, 100);
    assert!(states[1].position < 45_000);
    rig.health(1, 1);
}

#[tokio::test(start_paused = true)]
async fn s8_refused_start_stops_both_axes_without_a_status_read() {
    let rig = Rig::new().await;
    rig.tracking().await;
    rig.handle.inject_refusal(b'J', 1, 2, 1);
    let mark = rig.mark();
    let asked = Instant::now();
    rig.rate(AZ, 0.0041).await;
    assert_eq!(asked.elapsed(), Duration::ZERO);
    assert_wire(&rig.since(mark), &[":f1", ":I1", ":J1", ":K1", ":K2"]);
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_moving(rig.both().await, [false, false]);
    rig.health(0, 1);
}

#[tokio::test(start_paused = true)]
async fn s9_line_health_survives_reset_and_accumulates_on_the_new_board() {
    let rig = Rig::new().await;
    assert_moving(rig.both().await, [false, false]);
    rig.line.delay_next(":f1", Duration::from_millis(400));
    assert!(!rig.read(AZ).await.moving);
    rig.line.lose_next(":J1");
    rig.rate(AZ, 0.004).await;
    assert_moving(rig.both().await, [false, false]);
    rig.stale(1, "=001\\r");
    rig.health(1, 1);

    rig.handle.power_cycle();
    tokio::time::sleep(Duration::from_secs(1)).await;
    rig.read(AZ).await;
    assert_eq!(rig.link.resets(), 1);
    rig.stale(1, "=001\\r");
    rig.health(1, 1);
    rig.read(ALT).await;
    assert_eq!(rig.link.resets(), 1);
    rig.stale(1, "=001\\r");
    rig.health(1, 1);

    rig.tracking().await;
    rig.line.delay_next(":f1", Duration::from_millis(400));
    assert!(rig.read(AZ).await.moving);
    rig.line.lose_next(":J2");
    rig.rate(ALT, 0.5).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_moving(rig.both().await, [false, false]);
    rig.stale(2, "=111\\r");
    rig.health(2, 2);
}

#[tokio::test(start_paused = true)]
async fn s10_unreadable_start_status_exhausts_five_tries_and_stops_both_axes() {
    let rig = Rig::new().await;
    rig.tracking().await;
    rig.handle.drop_replies(b'J', 1, 1);
    let mark = rig.mark();
    let asked = Instant::now();
    let service = rig.service.clone();
    let rate = tokio::spawn(async move {
        service
            .rate_axis(Request::new(proto::RateAxisRequest {
                axis: AZ as i32,
                deg_per_s: 0.0041,
            }))
            .await
    });
    tokio::time::sleep(Duration::from_millis(100)).await;
    rig.handle.drop_replies(b'f', 1, 5);
    rate.await.unwrap().unwrap();
    assert_eq!(asked.elapsed(), Duration::from_millis(1800));
    assert_wire(
        &rig.since(mark),
        &[
            ":f1", ":I1", ":J1", ":f1", ":f1", ":f1", ":f1", ":f1", ":K1", ":K2",
        ],
    );
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_moving(rig.both().await, [false, false]);
    rig.health(1, 1);
}
