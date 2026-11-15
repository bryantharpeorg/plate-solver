//! US2 oracles: the emulator's recorded commands, acknowledgements and board state.
//! Counts and reset totals are exact. All deadlines use paused Tokio time, also
//! supplied to the board and link through NodeClock; no wall clock is involved.
use scope_link::proto::{
    self, scope_node_mount_server::ScopeNodeMount, scope_node_stop_server::ScopeNodeStop,
};
use scope_node::{
    clock::NodeClock,
    mount::{
        skywatcher::{
            emulator::{Emulator, EmulatorConfig, EmulatorHandle, Exchange},
            MountLink,
        },
        Axis, Mount,
    },
    service::NodeService,
    synthetic::SyntheticCamera,
    Settings,
};
use std::{sync::Arc, time::Duration};
use tonic::Request;

struct Clock(tokio::time::Instant);
impl NodeClock for Clock {
    fn now(&self) -> f64 {
        self.0.elapsed().as_secs_f64()
    }
}

async fn setup() -> (Arc<MountLink<Emulator>>, EmulatorHandle, Arc<dyn NodeClock>) {
    let clock: Arc<dyn NodeClock> = Arc::new(Clock(tokio::time::Instant::now()));
    let (_, handle) = Emulator::new(EmulatorConfig::default(), clock.clone());
    let board = handle.clone();
    let reopen_clock = clock.clone();
    let link = MountLink::start(
        Box::new(move || Ok(board.reopen(reopen_clock.clone()))),
        clock.clone(),
    );
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert!(link.connected());
    assert_eq!(link.resets(), 0, "the first connect is not a reset");
    (link, handle, clock)
}

async fn poll(link: &MountLink<Emulator>) {
    for axis in [Axis::Azimuth, Axis::Altitude] {
        link.state(axis).await;
    }
}

async fn poll_seconds(link: &MountLink<Emulator>, seconds: usize) {
    for _ in 0..seconds {
        tokio::time::sleep(Duration::from_secs(1)).await;
        poll(link).await;
    }
}

fn assert_no_move(events: &[Exchange]) {
    assert!(
        events.iter().all(|e| !matches!(
            e.command.as_bytes()[1],
            b'G' | b'I' | b'S' | b'H' | b'M' | b'J'
        )),
        "unexpected move: {events:?}"
    );
}

fn count_command(events: &[Exchange], command: &str) -> usize {
    events.iter().filter(|e| e.command == command).count()
}

#[tokio::test(start_paused = true)]
async fn power_cycle_initialises_and_disables_encoders_before_any_move() {
    let (link, handle, _) = setup().await;
    poll(&link).await;
    let cycled = handle.exchanges().len();
    handle.power_cycle();
    poll_seconds(&link, 1).await;
    let events = handle.exchanges();
    for command in [":F3\r", ":W1050000\r", ":W2050000\r"] {
        assert_eq!(count_command(&events[cycled..], command), 1);
    }
    assert_eq!(link.resets(), 1);
    assert!(!handle.auxiliary_enabled(1));
    assert!(!handle.auxiliary_enabled(2));
    assert_no_move(&events[cycled..]);
}

#[tokio::test(start_paused = true)]
async fn lost_link_after_a_move_reconnects_and_counts_one_reset() {
    let (link, handle, _) = setup().await;
    link.goto(Axis::Azimuth, 100_000).await;
    handle.power_off();
    poll_seconds(&link, 5).await;
    assert!(!link.connected());
    handle.power_on();
    poll_seconds(&link, 6).await;
    assert!(link.connected());
    assert_eq!(link.resets(), 1);
}

#[tokio::test(start_paused = true)]
async fn counts_jump_reinitialises_the_board() {
    let (link, handle, _) = setup().await;
    poll(&link).await;
    let jumped = handle.exchanges().len();
    handle.set_position(1, 500_000);
    poll_seconds(&link, 1).await;
    assert_eq!(link.resets(), 1);
    assert_eq!(count_command(&handle.exchanges()[jumped..], ":F3\r"), 1);
}

#[tokio::test(start_paused = true)]
async fn real_goto_within_top_rate_is_not_a_jump() {
    let (link, handle, _) = setup().await;
    poll(&link).await;
    link.goto(Axis::Azimuth, 16_200).await;
    let state = tokio::time::timeout(Duration::from_secs(60), async {
        loop {
            tokio::time::sleep(Duration::from_millis(100)).await;
            let state = link.state(Axis::Azimuth).await;
            if !state.moving {
                break state;
            }
        }
    })
    .await
    .expect("the emulator must finish this slew within 60 s");
    assert_eq!(state.position, 16_200);
    assert_eq!(count_command(&handle.exchanges(), ":J1\r"), 1);
    assert_eq!(link.resets(), 0);
}

fn service(link: Arc<MountLink<Emulator>>, clock: Arc<dyn NodeClock>) -> NodeService {
    NodeService::with_mount(
        SyntheticCamera::new(clock.clone()),
        clock,
        "test-token",
        Settings::default(),
        link,
    )
    .unwrap()
}

fn assert_stop_acknowledged(events: &[Exchange]) {
    for command in [":K1\r", ":K2\r"] {
        let replies: Vec<_> = events
            .iter()
            .filter(|e| e.command == command)
            .map(|e| e.reply.as_deref())
            .collect();
        assert_eq!(replies, [Some("=\r")], "{command:?}");
    }
    let k1 = events.iter().position(|e| e.command == ":K1\r").unwrap();
    let k2 = events.iter().position(|e| e.command == ":K2\r").unwrap();
    assert!(k1 < k2);
}

#[tokio::test(start_paused = true)]
async fn axis_state_reports_the_reset_count_on_both_axes() {
    let (link, handle, clock) = setup().await;
    poll(&link).await;
    handle.power_cycle();
    poll_seconds(&link, 1).await;
    let service = service(link, clock);
    for axis in [proto::Axis::Azimuth, proto::Axis::Altitude] {
        let state = service
            .axis_state(Request::new(proto::AxisStateRequest { axis: axis as i32 }))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(state.mount_resets, 1);
        assert_eq!(state.watchdog_stops, 0);
    }
}

#[tokio::test(start_paused = true)]
async fn disconnected_moves_never_reach_the_wire() {
    let (link, handle, _) = setup().await;
    link.goto(Axis::Azimuth, 100_000).await;
    assert_eq!(count_command(&handle.exchanges(), ":J1\r"), 1);
    let powered_off = handle.exchanges().len();
    handle.power_off();
    poll_seconds(&link, 5).await;
    assert!(!link.connected());
    link.goto(Axis::Azimuth, 1000).await;
    link.rate(Axis::Altitude, 0.5).await;
    assert_no_move(&handle.exchanges()[powered_off..]);
}

#[tokio::test(start_paused = true)]
async fn cleared_bit_and_counts_jump_on_one_read_count_once() {
    let (link, handle, _) = setup().await;
    poll(&link).await;
    link.goto(Axis::Azimuth, 100_000).await;
    // Read the completed move, establishing a nonzero jump baseline and set bit.
    poll_seconds(&link, 10).await;
    assert_eq!(link.state(Axis::Azimuth).await.position, 100_000);
    let cycled = handle.exchanges().len();
    handle.power_cycle();
    poll_seconds(&link, 4).await;
    assert_eq!(link.resets(), 1);
    assert_eq!(count_command(&handle.exchanges()[cycled..], ":F3\r"), 1);
}

#[tokio::test(start_paused = true)]
async fn bit_that_never_read_set_does_not_count_a_reset() {
    let (link, handle, _) = setup().await;
    handle.inject_reply(b'f', 1, "=000\r", 5);
    poll_seconds(&link, 5).await;
    assert_eq!(link.resets(), 0);
    assert_eq!(count_command(&handle.exchanges(), ":F3\r"), 1);
}

#[tokio::test(start_paused = true)]
async fn failed_reinitialisation_yields_to_stop_drops_moves_and_eventually_counts_once() {
    let (link, handle, _) = setup().await;
    poll(&link).await;
    handle.drop_replies(b'e', 1, 10);
    let cycled = handle.exchanges().len();
    handle.power_cycle();
    tokio::time::sleep(Duration::from_secs(1)).await;
    let polling_link = link.clone();
    let polling = tokio::spawn(async move { poll(&polling_link).await });
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(
        !polling.is_finished(),
        "re-initialisation must still be running"
    );
    assert!(!link.connected());
    assert!(
        tokio::time::timeout(Duration::from_millis(10), link.stop_confirmed())
            .await
            .expect("Stop must interrupt re-initialisation")
    );
    tokio::time::timeout(Duration::from_millis(10), link.goto(Axis::Azimuth, 1000))
        .await
        .expect("disconnected GoTo must return at once");
    tokio::time::timeout(Duration::from_millis(10), link.rate(Axis::Altitude, 0.5))
        .await
        .expect("disconnected rate must return at once");
    polling.await.unwrap();
    assert_eq!(
        link.resets(),
        0,
        "a failed re-initialisation is not counted yet"
    );
    assert_stop_acknowledged(&handle.exchanges()[cycled..]);
    tokio::time::sleep(Duration::from_secs(10)).await;
    assert!(link.connected());
    assert_eq!(link.resets(), 1);
    assert_no_move(&handle.exchanges()[cycled..]);
}

#[tokio::test(start_paused = true)]
async fn cancelled_poll_leaves_reinitialisation_running() {
    let (link, handle, _) = setup().await;
    poll(&link).await;
    let cycled = handle.exchanges().len();
    handle.power_cycle();
    handle.drop_replies(b'e', 1, 2);
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert!(
        tokio::time::timeout(Duration::from_millis(200), poll(&link))
            .await
            .is_err()
    );
    tokio::time::sleep(Duration::from_secs(5)).await;
    assert!(link.connected());
    assert_eq!(link.resets(), 1);
    assert_eq!(count_command(&handle.exchanges()[cycled..], ":F3\r"), 1);
}

async fn lose_running_link(link: &MountLink<Emulator>, handle: &EmulatorHandle) -> usize {
    poll(link).await;
    link.rate(Axis::Azimuth, 1.0).await;
    assert_eq!(count_command(&handle.exchanges(), ":J1\r"), 1);
    let injected = handle.exchanges().len();
    handle.drop_replies(b'e', 1, 5);
    handle.drop_replies(b'f', 1, 10);
    handle.drop_replies(b'f', 2, 5);
    poll(link).await;
    assert!(link.connected(), "two failed reads are not loss");
    poll(link).await;
    assert!(!link.connected());
    injected
}

#[tokio::test(start_paused = true)]
async fn stop_reaches_a_lost_powered_board_and_interrupts_connect() {
    let (link, handle, _) = setup().await;
    let injected = lose_running_link(&link, &handle).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(!link.connected());
    assert!(
        tokio::time::timeout(Duration::from_millis(100), link.stop_confirmed())
            .await
            .expect("Stop must make the connect give way")
    );
    assert_stop_acknowledged(&handle.exchanges()[injected..]);
    tokio::time::sleep(Duration::from_secs(10)).await;
    assert!(link.connected());
    assert_eq!(link.resets(), 1);
    let first = link.state(Axis::Azimuth).await;
    tokio::time::sleep(Duration::from_secs(1)).await;
    let second = link.state(Axis::Azimuth).await;
    assert!(!first.moving && !second.moving);
    assert_eq!(first.position, second.position);
}

#[tokio::test(start_paused = true)]
async fn silent_lost_board_gets_each_stop_once_in_order_within_four_timeouts() {
    let (link, handle, _) = setup().await;
    poll(&link).await;
    let powered_off = handle.exchanges().len();
    handle.power_off();
    poll(&link).await;
    poll(&link).await;
    assert!(!link.connected());
    tokio::time::sleep(Duration::from_millis(100)).await;
    let started = tokio::time::Instant::now();
    assert!(
        !tokio::time::timeout(Duration::from_millis(1500), link.stop_confirmed())
            .await
            .expect("silent Stop must finish within 1.5 s")
    );
    assert!(started.elapsed() <= Duration::from_millis(1200));
    let events = handle.exchanges();
    let stops: Vec<_> = events[powered_off..]
        .iter()
        .filter(|e| matches!(e.command.as_bytes()[1], b'K' | b'L'))
        .map(|e| (e.command.as_str(), e.reply.as_deref()))
        .collect();
    assert_eq!(
        stops,
        [
            (":K1\r", None),
            (":K2\r", None),
            (":L1\r", None),
            (":L2\r", None)
        ]
    );
}

#[tokio::test(start_paused = true)]
async fn service_stop_over_a_lost_powered_board_answers_ok() {
    let (link, handle, clock) = setup().await;
    let injected = lose_running_link(&link, &handle).await;
    assert!(!link.connected());
    // Build after loss: an earlier watchdog could stop the still-connected board.
    let service = service(link.clone(), clock);
    assert!(!link.connected());
    tokio::time::timeout(
        Duration::from_millis(500),
        service.stop_all(Request::new(proto::StopAllRequest {})),
    )
    .await
    .expect("service Stop must interrupt connect")
    .expect("acknowledged Stop must answer OK even when the link is lost");
    assert_stop_acknowledged(&handle.exchanges()[injected..]);
}
