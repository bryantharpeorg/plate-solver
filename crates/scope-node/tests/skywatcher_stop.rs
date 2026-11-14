//! Oracles: emulator serial exchanges and Sky-Watcher's K/L stop replies.
//! Confirmation and command order are exact; mechanics get 1 s to decelerate.
use scope_node::{
    clock::NodeClock,
    mount::{
        skywatcher::{
            emulator::{Emulator, EmulatorConfig, EmulatorHandle},
            SkyWatcher,
        },
        Axis, Mount,
    },
};
use std::{sync::Arc, time::Duration};

struct Clock(tokio::time::Instant);
impl NodeClock for Clock {
    fn now(&self) -> f64 {
        self.0.elapsed().as_secs_f64()
    }
}

async fn moving_mount(config: EmulatorConfig) -> (SkyWatcher<Emulator>, EmulatorHandle) {
    let clock: Arc<dyn NodeClock> = Arc::new(Clock(tokio::time::Instant::now()));
    let (stream, handle) = Emulator::new(config, clock.clone());
    let mount = SkyWatcher::connect(stream, clock).await.unwrap();
    mount.rate(Axis::Azimuth, 1.0).await;
    mount.rate(Axis::Altitude, -1.0).await;
    tokio::time::sleep(Duration::from_secs(1)).await;
    for axis in [Axis::Azimuth, Axis::Altitude] {
        assert!(mount.state(axis).await.moving, "{axis:?} must be moving");
    }
    (mount, handle)
}

async fn assert_rest(mount: &SkyWatcher<Emulator>) {
    tokio::time::sleep(Duration::from_secs(1)).await;
    for axis in [Axis::Azimuth, Axis::Altitude] {
        let state = mount.state(axis).await;
        assert!(!state.moving, "{axis:?}: {state:?}");
        assert_eq!(state.rate_deg_per_s, 0.0);
    }
}

#[tokio::test(start_paused = true)]
async fn a_healthy_board_confirms_both_axis_stops() {
    let (mount, handle) = moving_mount(EmulatorConfig::default()).await;
    assert!(mount.stop_confirmed().await);
    let stops: Vec<_> = handle
        .exchanges()
        .into_iter()
        .filter(|e| matches!(e.command.as_bytes().get(1), Some(b'K' | b'L')))
        .map(|e| (e.command, e.reply))
        .collect();
    assert_eq!(
        stops,
        [
            (":K1\r".into(), Some("=\r".into())),
            (":K2\r".into(), Some("=\r".into())),
        ]
    );
    assert_rest(&mount).await;
}

#[tokio::test(start_paused = true)]
async fn a_silent_first_axis_confirms_with_emergency_stop_after_the_second_axis() {
    let (mount, handle) = moving_mount(EmulatorConfig {
        silent_stop_axis: Some(1),
        ..Default::default()
    })
    .await;
    assert!(mount.stop_confirmed().await);
    let events = handle.exchanges();
    let k1 = events.iter().position(|e| e.command == ":K1\r").unwrap();
    let k2 = events.iter().position(|e| e.command == ":K2\r").unwrap();
    let l1 = events.iter().position(|e| e.command == ":L1\r").unwrap();
    assert!(k1 < k2 && k2 < l1);
    assert!(events
        .iter()
        .filter(|e| e.command == ":K1\r")
        .all(|e| e.reply.is_none()));
    assert_eq!(events[k2].reply.as_deref(), Some("=\r"));
    assert_eq!(events[l1].reply.as_deref(), Some("=\r"));
    assert!(!events.iter().any(|e| e.command == ":L2\r"));
    assert_rest(&mount).await;
}

#[tokio::test(start_paused = true)]
async fn missing_normal_and_emergency_replies_on_axis_two_are_not_confirmation() {
    let (mount, handle) = moving_mount(EmulatorConfig::default()).await;
    handle.drop_replies(b'K', 2, 5);
    handle.drop_replies(b'L', 2, 5);
    assert!(!mount.stop_confirmed().await);
    let events = handle.exchanges();
    let k1 = events.iter().position(|e| e.command == ":K1\r").unwrap();
    let k2 = events.iter().position(|e| e.command == ":K2\r").unwrap();
    let l2 = events.iter().position(|e| e.command == ":L2\r").unwrap();
    assert!(k1 < k2 && k2 < l2);
    assert_eq!(events[k1].reply.as_deref(), Some("=\r"));
    for command in [":K2\r", ":L2\r"] {
        let attempts: Vec<_> = events.iter().filter(|e| e.command == command).collect();
        assert_eq!(attempts.len(), 5);
        assert!(attempts.iter().all(|e| e.reply.is_none()));
    }
    // The emulator executes silent commands. Physical rest alone cannot prove
    // receipt of a board acknowledgement.
    assert_rest(&mount).await;
}
