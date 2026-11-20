//! US3 oracles: the emulator's wire replies and counts, a separate fault-free
//! control, and the spec's fixed command counts and travel bounds. Comparisons
//! are exact except for the emulator's one-count rounding of position readings.
use crate::support::{
    live::{ConsoleClock, START},
    tracker::apparent,
};
use finder::{
    link::{LinkManager, Transport},
    model::Model,
    session::{Session, Site},
    targets,
    tracking::Tracker,
};
use scope_link::{memory, proto};
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
};
use simulator::VirtualClock;
use std::{sync::Arc, time::Duration};

struct NodeTime(VirtualClock);
impl NodeClock for NodeTime {
    fn now(&self) -> f64 {
        self.0.node_now()
    }
}

struct Rig {
    node_link: Arc<MountLink<Line>>,
    link: Arc<LinkManager>,
    handle: EmulatorHandle,
    line: LineHandle,
    tracker: Tracker,
}
impl Rig {
    async fn new() -> Self {
        let clock = VirtualClock::new(START);
        let node_clock: Arc<dyn NodeClock> = Arc::new(NodeTime(clock.clone()));
        let (_, handle) = Emulator::new(EmulatorConfig::default(), node_clock.clone());
        let line = LineHandle::default();
        let board = handle.clone();
        let faults = line.clone();
        let reopen_clock = node_clock.clone();
        let node_link = MountLink::start(
            Box::new(move || {
                Ok(Line::new(
                    board.reopen(reopen_clock.clone()),
                    faults.clone(),
                ))
            }),
            node_clock.clone(),
        );
        tokio::time::sleep(Duration::from_secs(1)).await;
        assert!(node_link.connected());
        assert_eq!(node_link.resets(), 0);
        assert_eq!(node_link.line_health(), LineHealth::default());

        let mut model = Model::default();
        model.load_known_terms(1.0, 130.0, [0.0; 2]);
        let (h, az) = apparent(targets::find("M42").unwrap(), clock.now());
        let zeros = model.inverse(az, h, [360; 2]).unwrap();
        model.load_known_terms(1.0, 130.0, zeros);
        // Only the tracker and explicit test reads may use the serial line.
        let controls = NodeControls::default();
        controls.stop_frames();
        let node = NodeService::with_controls_and_mount(
            SyntheticCamera::new(node_clock.clone()),
            node_clock,
            "test-token",
            Default::default(),
            controls,
            Some(node_link.clone()),
        )
        .unwrap();
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
        let session = Arc::new(Session::default());
        session
            .edit(
                serde_json::from_value(serde_json::json!({
                    "node":{"address":"scope.local:1","token":"test-token"}
                }))
                .unwrap(),
                clock.now(),
            )
            .unwrap();
        let console = Arc::new(ConsoleClock(clock));
        let link = Arc::new(LinkManager::start(session, console.clone(), transport));
        for _ in 0..100 {
            if link.status().borrow().connected {
                break;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        assert!(link.status().borrow().connected);
        let tracker = Tracker::new(
            console,
            link.clone(),
            model,
            Site {
                lat_deg: 40.0,
                lon_deg: -75.0,
            },
            15.0,
        );
        Self {
            node_link,
            link,
            handle,
            line,
            tracker,
        }
    }

    async fn track_m42(&self) {
        self.tracker
            .start(targets::find("M42").unwrap().clone())
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(5_500)).await;
        self.assert_tracking();
    }

    fn assert_tracking(&self) {
        let state = self.tracker.state();
        assert_eq!(state.state, "tracking");
        assert!(state.ended_reason.is_none(), "{state:?}");
    }

    async fn counts(&self) -> [i32; 2] {
        [
            self.node_link.state(Axis::Azimuth).await.position,
            self.node_link.state(Axis::Altitude).await.position,
        ]
    }

    fn mark(&self) -> usize {
        self.handle.exchanges().len()
    }

    fn since(&self, mark: usize) -> Vec<Exchange> {
        self.handle.exchanges()[mark..].to_vec()
    }
}

fn assert_no_stop(events: &[Exchange]) {
    assert!(
        events
            .iter()
            .all(|e| !e.command.starts_with(":K") && !e.command.starts_with(":L")),
        "{events:?}"
    );
}

fn assert_board_moving(handle: &EmulatorHandle, axis: u8) {
    let command = format!(":f{axis}");
    let events = handle.exchanges();
    let reply = events
        .iter()
        .rev()
        .filter(|e| e.command.starts_with(&command))
        .filter_map(|e| e.reply.as_deref())
        .find(|r| {
            r.len() == 5
                && r.starts_with('=')
                && r.ends_with('\r')
                && r.as_bytes()[1..4].iter().all(u8::is_ascii_digit)
        })
        .expect("the board must have recorded a three-digit status reply");
    assert_eq!(reply.as_bytes()[2], b'1', "axis {axis}: {reply:?}");
}

// Each invocation owns a new board, node and console, including the control.
// No expected position comes from the controller's rate arithmetic.
async fn start_run(axis: u8, fault: Option<bool>) -> [[i32; 2]; 2] {
    let rig = Rig::new().await;
    rig.track_m42().await;
    match fault {
        Some(true) => rig.handle.inject_reply(b'J', axis, "=0\r", 1),
        Some(false) => rig.handle.drop_replies(b'J', axis, 1),
        None => {}
    }
    let mark = rig.mark();
    tokio::time::sleep(Duration::from_millis(4_500)).await;
    let events = rig.since(mark);
    rig.assert_tracking();
    let early = rig.counts().await;
    tokio::time::sleep(Duration::from_secs(10)).await;
    let late = rig.counts().await;

    let start = format!(":J{axis}");
    assert!(events.iter().any(|e| e.command.starts_with(&start)));
    assert_eq!(
        events
            .iter()
            .filter(|e| e.command.starts_with(&start) && e.reply.as_deref() != Some("=\r"))
            .count(),
        usize::from(fault.is_some()),
        "{events:?}"
    );
    assert_no_stop(&events);
    let health = rig.node_link.line_health();
    assert_eq!(health.unacknowledged_starts, u64::from(fault.is_some()));
    assert_eq!(health.halted_moves, 0);
    assert!(rig.link.status().borrow().connected);
    rig.tracker.stop("stopped").await;
    [early, late]
}

async fn compare_starts(axis: u8) {
    let control = start_run(axis, None).await;
    for fault in [true, false] {
        let counts = start_run(axis, Some(fault)).await;
        for (actual, expected) in counts
            .into_iter()
            .flatten()
            .zip(control.into_iter().flatten())
        {
            assert!(
                actual.abs_diff(expected) <= 1,
                "axis {axis}, injected reply {fault}: {actual} vs control {expected}"
            );
        }
    }
}

#[tokio::test(start_paused = true)]
async fn s1_azimuth_start_faults_preserve_track_and_control_counts() {
    compare_starts(1).await;
}

#[tokio::test(start_paused = true)]
async fn s2_altitude_start_faults_preserve_track_and_control_counts() {
    compare_starts(2).await;
}

#[tokio::test(start_paused = true)]
async fn s3_late_status_keeps_track_moving_for_thirty_seconds() {
    let rig = Rig::new().await;
    rig.track_m42().await;
    rig.line.delay_next(":f1", Duration::from_millis(400));
    // Start and end away from the tracker's whole-second rate refreshes.
    tokio::time::sleep(Duration::from_millis(2_250)).await;
    let before = rig.counts().await;
    let mark = rig.mark();
    tokio::time::sleep(Duration::from_secs(30)).await;
    let events = rig.since(mark);
    let after = rig.counts().await;

    rig.assert_tracking();
    for command in [":J", ":I"] {
        assert_eq!(
            events
                .iter()
                .filter(|e| e.command.starts_with(command))
                .count(),
            60,
            "{command}: {events:?}"
        );
    }
    assert_no_stop(&events);
    assert!(after[0] - before[0] >= 700, "{before:?} -> {after:?}");
    assert!(after[1] - before[1] >= 100, "{before:?} -> {after:?}");
    for axis in [1, 2] {
        assert_board_moving(&rig.handle, axis);
    }
    assert_eq!(rig.node_link.health().invalid_replies, 0);
    assert_eq!(rig.node_link.line_health().stale_replies, 1);
    assert_eq!(rig.node_link.resets(), 0);
    assert!(rig.node_link.connected());
    assert!(rig.link.status().borrow().connected);
    rig.tracker.stop("stopped").await;
}

#[tokio::test(start_paused = true)]
async fn s4_console_client_timeout_leaves_the_next_axis_reply_in_step() {
    let rig = Rig::new().await;
    assert_eq!(rig.tracker.state().state, "off");
    let mut client = rig.link.mount().expect("console's paired mount client");
    client
        .goto_axis(proto::Axis::Altitude, 90_000)
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert!(
        client
            .axis_state(proto::Axis::Altitude)
            .await
            .unwrap()
            .moving
    );

    rig.line.delay_next(":f1", Duration::from_millis(250));
    let result = tokio::time::timeout(
        Duration::from_millis(200),
        client.axis_state(proto::Axis::Azimuth),
    )
    .await;
    assert!(
        result.is_err(),
        "the console call must time out: {result:?}"
    );
    tokio::time::sleep(Duration::from_millis(100)).await;
    let altitude = client.axis_state(proto::Axis::Altitude).await.unwrap();
    assert!(altitude.moving);
    assert_board_moving(&rig.handle, 2);
    assert_eq!(rig.node_link.line_health().stale_replies, 1);
}
