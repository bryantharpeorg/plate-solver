//! US4 oracles: injected wire bytes, recorded exchanges and positions set on the
//! emulator, with exact counts and strings (zero tolerance). Paused Tokio time
//! also supplies NodeClock to both the board and link.
use scope_node::{
    clock::NodeClock,
    mount::{
        skywatcher::{
            emulator::{Emulator, EmulatorConfig, EmulatorHandle, Exchange},
            mount_link::{GotoPath, MountHealth},
            MountLink,
        },
        Axis, AxisState, Mount,
    },
};
use std::{sync::Arc, time::Duration};

struct Clock(tokio::time::Instant);
impl NodeClock for Clock {
    fn now(&self) -> f64 {
        self.0.elapsed().as_secs_f64()
    }
}

async fn setup(config: EmulatorConfig) -> (Arc<MountLink<Emulator>>, EmulatorHandle) {
    let clock: Arc<dyn NodeClock> = Arc::new(Clock(tokio::time::Instant::now()));
    let (_, handle) = Emulator::new(config, clock.clone());
    let board = handle.clone();
    let reopen_clock = clock.clone();
    let link = MountLink::start(
        Box::new(move || Ok(board.reopen(reopen_clock.clone()))),
        clock,
    );
    tokio::time::sleep(Duration::from_secs(3)).await;
    assert!(link.connected());
    assert_eq!(link.resets(), 0);
    assert_eq!(link.health(), MountHealth::default());
    (link, handle)
}

async fn poll(link: &MountLink<Emulator>) -> [AxisState; 2] {
    [
        link.state(Axis::Azimuth).await,
        link.state(Axis::Altitude).await,
    ]
}

fn count_command(events: &[Exchange], command: &str) -> usize {
    events.iter().filter(|e| e.command == command).count()
}

#[tokio::test(start_paused = true)]
async fn short_status_is_retried_and_kept_escaped() {
    let (link, handle) = setup(EmulatorConfig::default()).await;
    let injected = handle.exchanges().len();
    handle.inject_reply(b'f', 1, "=10\r", 1);
    let states = poll(&link).await;
    assert!(!states[0].moving);
    assert_eq!(link.health().invalid_replies, 1);
    assert_eq!(link.health().last_invalid.as_deref(), Some("=10\\r"));
    assert_eq!(count_command(&handle.exchanges()[injected..], ":f1\r"), 2);
}

#[tokio::test(start_paused = true)]
async fn five_short_statuses_exhaust_exactly_five_transmissions() {
    let (link, handle) = setup(EmulatorConfig::default()).await;
    let injected = handle.exchanges().len();
    handle.inject_reply(b'f', 1, "=10\r", 5);
    let states = poll(&link).await;
    assert!(states[0].moving, "lost telemetry must never look like rest");
    assert_eq!(link.health().invalid_replies, 5);
    assert_eq!(link.health().last_invalid.as_deref(), Some("=10\\r"));
    assert_eq!(count_command(&handle.exchanges()[injected..], ":f1\r"), 5);
}

#[tokio::test(start_paused = true)]
async fn short_position_is_retried_from_the_boards_counts() {
    let (link, handle) = setup(EmulatorConfig::default()).await;
    handle.set_position(2, 123_456);
    let injected = handle.exchanges().len();
    handle.inject_reply(b'j', 2, "=12\r", 1);
    let states = poll(&link).await;
    assert_eq!(states[1].position, 123_456);
    assert!(!states[1].moving);
    let events = handle.exchanges();
    let replies: Vec<_> = events[injected..]
        .iter()
        .filter(|e| e.command == ":j2\r")
        .map(|e| e.reply.as_deref())
        .collect();
    // Sky-Watcher byte pairs: 123456 + 0x800000 = 0x81E240.
    assert_eq!(replies, [Some("=12\r"), Some("=40E281\r")]);
    assert_eq!(link.health().invalid_replies, 1);
    assert_eq!(link.health().last_invalid.as_deref(), Some("=12\\r"));
    assert_eq!(link.resets(), 0, "this is the first position baseline");
}

#[tokio::test(start_paused = true)]
async fn goto_records_the_accepted_path_on_each_board() {
    let (relative, _) = setup(EmulatorConfig {
        refuse_absolute: true,
        ..Default::default()
    })
    .await;
    assert_eq!(relative.health().goto_path[0], None);
    relative.goto(Axis::Azimuth, 16_200).await;
    assert_eq!(
        relative.health().goto_path,
        [Some(GotoPath::Relative), None]
    );

    let (absolute, _) = setup(EmulatorConfig::default()).await;
    assert_eq!(absolute.health().goto_path[0], None);
    absolute.goto(Axis::Azimuth, 16_200).await;
    assert_eq!(
        absolute.health().goto_path,
        [Some(GotoPath::Absolute), None]
    );
}

#[tokio::test(start_paused = true)]
async fn motion_start_with_a_lost_acknowledgement_is_sent_once() {
    let (link, handle) = setup(EmulatorConfig::default()).await;
    let injected = handle.exchanges().len();
    handle.drop_replies(b'J', 1, 1);
    link.goto(Axis::Azimuth, 16_200).await;
    assert_eq!(count_command(&handle.exchanges()[injected..], ":J1\r"), 1);
}

#[tokio::test(start_paused = true)]
async fn reset_finding_read_keeps_its_invalid_reply_and_the_old_goto_path() {
    let (link, handle) = setup(EmulatorConfig::default()).await;
    handle.inject_reply(b'f', 1, "=10\r", 1);
    poll(&link).await;
    link.goto(Axis::Azimuth, 16_200).await;
    tokio::time::sleep(Duration::from_secs(2)).await;
    poll(&link).await;
    handle.power_cycle();
    handle.inject_reply(b'f', 1, "=1\r", 1);
    tokio::time::sleep(Duration::from_secs(1)).await;
    poll(&link).await;
    assert_eq!(link.resets(), 1);
    assert_eq!(
        link.health(),
        MountHealth {
            invalid_replies: 2,
            last_invalid: Some("=1\\r".into()),
            goto_path: [Some(GotoPath::Absolute), None],
        }
    );
}

#[tokio::test(start_paused = true)]
async fn timeout_framing_and_length_failures_share_one_read_budget() {
    for (command, axis, short) in [(b'f', 1, "=123456\r"), (b'j', 2, "=\r")] {
        let (link, handle) = setup(EmulatorConfig::default()).await;
        let injected = handle.exchanges().len();
        handle.drop_replies(command, axis, 1);
        handle.inject_reply(command, axis, "?garbled\r", 1);
        handle.inject_reply(command, axis, short, 1);
        handle.drop_replies(command, axis, 1);
        handle.inject_reply(command, axis, short, 1);
        let states = poll(&link).await;
        assert!(states[usize::from(axis - 1)].moving);
        let wire = format!(":{}{axis}\r", char::from(command));
        assert_eq!(count_command(&handle.exchanges()[injected..], &wire), 5);
        assert_eq!(link.health().invalid_replies, 2);
        assert_eq!(
            link.health().last_invalid,
            Some(short.as_bytes().escape_ascii().to_string())
        );
        assert!(!poll(&link).await[usize::from(axis - 1)].moving);
        assert_eq!(link.health().invalid_replies, 2);
    }
}

#[tokio::test(start_paused = true)]
async fn refusal_is_neither_retried_nor_counted_as_a_wrong_length() {
    for (command, axis) in [(b'f', 1), (b'j', 2)] {
        let (link, handle) = setup(EmulatorConfig::default()).await;
        let injected = handle.exchanges().len();
        handle.inject_refusal(command, axis, 0, 1);
        assert!(poll(&link).await[usize::from(axis - 1)].moving);
        let wire = format!(":{}{axis}\r", char::from(command));
        assert_eq!(count_command(&handle.exchanges()[injected..], &wire), 1);
        assert_eq!(link.health(), MountHealth::default());
    }
}

#[tokio::test(start_paused = true)]
async fn refused_motion_start_is_also_sent_once() {
    let (link, handle) = setup(EmulatorConfig::default()).await;
    let injected = handle.exchanges().len();
    handle.inject_refusal(b'J', 1, 2, 1);
    link.goto(Axis::Azimuth, 16_200).await;
    assert_eq!(count_command(&handle.exchanges()[injected..], ":J1\r"), 1);
}

#[tokio::test(start_paused = true)]
async fn goto_and_rate_copy_health_including_stop_wait_reads() {
    let (link, handle) = setup(EmulatorConfig::default()).await;
    handle.inject_reply(b'f', 1, "=1\r", 1);
    handle.inject_reply(b'j', 1, "=12\r", 1);
    // The positioning path is accepted before the start, even if its ACK is lost.
    handle.drop_replies(b'J', 1, 1);
    link.goto(Axis::Azimuth, 16_200).await;
    assert_eq!(link.health().invalid_replies, 2);
    assert_eq!(link.health().last_invalid.as_deref(), Some("=12\\r"));
    assert_eq!(link.health().goto_path, [Some(GotoPath::Absolute), None]);
    handle.inject_reply(b'f', 2, "=10\r", 1);
    link.rate(Axis::Altitude, 0.5).await;
    assert_eq!(link.health().invalid_replies, 3);
    handle.inject_reply(b'f', 2, "=\r", 1);
    link.rate(Axis::Altitude, 0.0).await;
    assert_eq!(link.health().invalid_replies, 4);
    assert_eq!(link.health().last_invalid.as_deref(), Some("=\\r"));
}

#[tokio::test(start_paused = true)]
async fn loss_finding_read_is_kept_and_reconnect_does_not_double_count_health() {
    let (link, handle) = setup(EmulatorConfig::default()).await;
    handle.inject_reply(b'f', 1, "=1\r", 10);
    handle.inject_reply(b'f', 2, "=22\r", 5);
    poll(&link).await;
    link.state(Axis::Azimuth).await;
    assert!(!link.connected());
    assert_eq!(link.health().invalid_replies, 15);
    assert_eq!(link.health().last_invalid.as_deref(), Some("=1\\r"));
    tokio::time::sleep(Duration::from_secs(3)).await;
    assert!(link.connected());
    assert_eq!(link.resets(), 1);
    assert_eq!(link.health().invalid_replies, 15);
    handle.inject_reply(b'j', 1, "=12\r", 1);
    poll(&link).await;
    poll(&link).await;
    assert_eq!(link.health().invalid_replies, 16);
    assert_eq!(link.health().last_invalid.as_deref(), Some("=12\\r"));
}

#[tokio::test(start_paused = true)]
async fn successive_boards_fold_goto_paths_per_axis_and_keep_the_latest_bytes() {
    let (link, handle) = setup(EmulatorConfig::default()).await;
    handle.inject_reply(b'f', 1, "=1\r", 1);
    poll(&link).await;
    link.goto(Axis::Azimuth, 16_200).await;
    handle.inject_refusal(b'S', 2, 0, 1);
    link.goto(Axis::Altitude, 16_200).await;
    tokio::time::sleep(Duration::from_secs(2)).await;
    poll(&link).await;
    handle.power_cycle();
    tokio::time::sleep(Duration::from_secs(1)).await;
    poll(&link).await;
    assert_eq!(link.resets(), 1);
    handle.inject_refusal(b'S', 1, 0, 1);
    link.goto(Axis::Azimuth, 16_200).await;
    handle.inject_reply(b'j', 2, "=34\r", 1);
    tokio::time::sleep(Duration::from_secs(2)).await;
    poll(&link).await;
    assert_eq!(link.health().goto_path, [Some(GotoPath::Relative); 2]);
    handle.power_cycle();
    tokio::time::sleep(Duration::from_secs(1)).await;
    poll(&link).await;
    assert_eq!(link.resets(), 2);
    link.goto(Axis::Altitude, 16_200).await;
    assert_eq!(
        link.health(),
        MountHealth {
            invalid_replies: 2,
            last_invalid: Some("=34\\r".into()),
            goto_path: [Some(GotoPath::Relative), Some(GotoPath::Absolute)],
        }
    );
    assert_eq!(GotoPath::Absolute.to_string(), ":S");
    assert_eq!(GotoPath::Relative.to_string(), ":H/:M");
}
