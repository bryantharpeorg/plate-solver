//! Byte oracles: Sky-Watcher motor-controller command set §§4–5 and the
//! first-test sequence in docs/research/2026-10-21-synscan-mount-control.md.
use scope_node::{
    clock::NodeClock,
    mount::{
        skywatcher::{
            emulator::{Emulator, EmulatorConfig, EmulatorHandle},
            probe,
            transport::Transport,
            Error, SkyWatcher,
        },
        Axis, Mount,
    },
};
use std::{sync::Arc, time::Duration};
use Axis::{Altitude as Alt, Azimuth as Az};

struct Clock(tokio::time::Instant);
impl NodeClock for Clock {
    fn now(&self) -> f64 {
        self.0.elapsed().as_secs_f64()
    }
}
async fn advance(seconds: u64) {
    tokio::time::sleep(Duration::from_secs(seconds)).await;
}
fn stream(config: EmulatorConfig) -> (Emulator, EmulatorHandle, Arc<dyn NodeClock>) {
    let clock: Arc<dyn NodeClock> = Arc::new(Clock(tokio::time::Instant::now()));
    let (stream, handle) = Emulator::new(config, clock.clone());
    (stream, handle, clock)
}
async fn connect(config: EmulatorConfig) -> (SkyWatcher<Emulator>, EmulatorHandle) {
    let (stream, handle, clock) = stream(config);
    (SkyWatcher::connect(stream, clock).await.unwrap(), handle)
}
fn commands(handle: &EmulatorHandle) -> Vec<String> {
    handle.exchanges().into_iter().map(|e| e.command).collect()
}

#[tokio::test(start_paused = true)]
async fn absolute_goto_bytes_and_arrival() {
    let (mount, handle) = connect(EmulatorConfig::default()).await;
    mount.goto(Az, 0x123).await;
    assert!(commands(&handle).contains(&":S1230180\r".into()));
    assert!(mount.state(Az).await.moving);
    advance(2).await;
    let state = mount.state(Az).await;
    assert_eq!(state.position, 0x123);
    assert!(!state.moving);
    assert_eq!(mount.axes()[0].counts_per_revolution, 1_620_000);
}

#[tokio::test(start_paused = true)]
async fn relative_fallback_and_arrival() {
    let (mount, handle) = connect(EmulatorConfig {
        refuse_absolute: true,
        ..Default::default()
    })
    .await;
    for (axis, position, target) in [(Az, 0x123, ":H1230100\r"), (Alt, -0x123, ":H2230100\r")] {
        mount.goto(axis, position).await;
        let wire = commands(&handle);
        assert!(wire.contains(&target.into()));
        assert!(wire
            .iter()
            .any(|s| s.starts_with(&format!(":M{}", axis.index() + 1))));
        advance(2).await;
        assert_eq!(mount.state(axis).await.position, position);
        assert!(!mount.state(axis).await.moving);
    }
}

fn assert_stop_wait_between(handle: &EmulatorHandle, first: usize, last: usize) {
    let events = handle.exchanges();
    let stopped = &events[first + 1..last];
    let stop = stopped.iter().position(|e| e.command == ":K1\r").unwrap();
    let polls = &stopped[stop + 1..];
    let running: Vec<_> = polls
        .iter()
        .filter(|e| e.command == ":f1\r")
        .map(|e| e.reply.as_ref().unwrap().as_bytes()[2])
        .collect();
    assert!(running.contains(&b'1'), "must observe deceleration");
    assert_eq!(
        running.last(),
        Some(&b'0'),
        "must finish with confirmed rest"
    );
}

#[tokio::test(start_paused = true)]
async fn motor_refusal_stops_waits_and_retries_once() {
    let (mount, handle) = connect(EmulatorConfig::default()).await;
    mount.rate(Az, 0.01).await;
    advance(1).await;
    handle.inject_refusal(b'I', 1, 2, 1);
    mount.rate(Az, 0.02).await;
    let wire = commands(&handle);
    let attempts: Vec<_> = wire
        .iter()
        .enumerate()
        .filter(|(_, c)| c.starts_with(":I1"))
        .map(|(i, _)| i)
        .collect();
    assert_eq!(attempts.len(), 3); // initial rate, refusal, one resend
    assert_stop_wait_between(&handle, attempts[1], attempts[2]);
    assert_eq!(wire[attempts[1]], wire[attempts[2]]);
    handle.inject_refusal(b'G', 2, 2, 2);
    mount.rate(Alt, 1.0).await;
    assert!(!mount.state(Alt).await.moving);
    assert_eq!(
        commands(&handle)
            .iter()
            .filter(|c| c.starts_with(":G2"))
            .count(),
        2
    );
}

#[tokio::test(start_paused = true)]
async fn reversal_waits_for_confirmed_rest() {
    let (mount, handle) = connect(EmulatorConfig::default()).await;
    mount.rate(Az, 1.0).await;
    advance(1).await;
    assert!(mount.state(Az).await.rate_deg_per_s > 0.0);
    mount.rate(Az, -1.0).await;
    let wire = commands(&handle);
    let first = wire.iter().position(|c| c == ":G130\r").unwrap();
    let reverse = wire.iter().position(|c| c == ":G131\r").unwrap();
    assert_stop_wait_between(&handle, first, reverse);
    advance(1).await;
    assert!(mount.state(Az).await.rate_deg_per_s < 0.0);
}

#[tokio::test(start_paused = true)]
async fn baud_discovery_initialization_and_auxiliary_encoders() {
    let (mount, handle) = connect(EmulatorConfig {
        baud: 115200,
        ..Default::default()
    })
    .await;
    assert_eq!(handle.baud_attempts(), [9600, 115200]);
    assert_eq!(mount.report().baud, 115200);
    assert_eq!(mount.report().firmware.to_string(), "3.40 (board 0x90)");
    let events = handle.exchanges();
    assert!(events
        .iter()
        .any(|e| e.baud == 9600 && e.command == ":e1\r" && e.reply.is_none()));
    let wire = commands(&handle);
    assert!(wire.contains(&":F3\r".into()));
    for axis in [1, 2] {
        for command in [
            format!(":a{axis}\r"),
            format!(":b{axis}\r"),
            format!(":g{axis}\r"),
            format!(":q{axis}010000\r"),
            format!(":W{axis}050000\r"),
        ] {
            assert!(wire.contains(&command), "missing {command:?}");
        }
        assert!(!handle.auxiliary_enabled(axis));
    }
    assert!(mount.report().warnings.is_empty());
}

#[tokio::test(start_paused = true)]
async fn echo_fragmentation_and_old_firmware() {
    let (mount, handle) = connect(EmulatorConfig {
        echo: true,
        firmware: [3, 38, 0x90],
        ..Default::default()
    })
    .await;
    assert_eq!(mount.report().firmware.to_string(), "3.38 (board 0x90)");
    assert!(mount
        .report()
        .warnings
        .iter()
        .any(|w| w.contains("smooth variable-rate tracking")));
    mount.goto(Alt, -1).await;
    advance(1).await;
    assert_eq!(mount.state(Alt).await.position, -1);
    assert!(commands(&handle).contains(&":S2FFFF7F\r".into()));
}

#[tokio::test(start_paused = true)]
async fn step_period_modes_and_measured_rates() {
    let (mount, handle) = connect(EmulatorConfig::default()).await;
    // Command-set formula, independent constants: 16 MHz, 1,620,000 CPR, ratio 16.
    for (axis, rate, fast, mode) in [(Az, 1.0, true, ":G130\r"), (Alt, 0.01, false, ":G210\r")] {
        mount.rate(axis, rate).await;
        let period = (16_000_000.0_f64 * 360.0 / rate / 1_620_000.0 * if fast { 16.0 } else { 1.0 })
            .round() as u32;
        assert_eq!(handle.step_period(axis.index() as u8 + 1), period);
        assert!(commands(&handle).contains(&mode.into()));
        advance(2).await;
        let before = mount.state(axis).await.position;
        advance(100).await;
        let after = mount.state(axis).await.position;
        let measured = f64::from(after - before) * 360.0 / 1_620_000.0 / 100.0;
        assert!(
            (measured / rate - 1.0).abs() < 0.001,
            "{measured} vs {rate}"
        );
    }
    // INDI's threshold is 128 times a sidereal revolution, 86164.0905 seconds.
    let threshold = 128.0 * 360.0 / 86164.0905;
    for (rate, mode) in [(threshold, ":G110\r"), (threshold * 1.0001, ":G130\r")] {
        mount.rate(Az, rate).await;
        assert_eq!(
            commands(&handle)
                .iter()
                .rev()
                .find(|c| c.starts_with(":G1"))
                .unwrap(),
            mode
        );
    }
}

#[tokio::test(start_paused = true)]
async fn both_stops_precede_emergency_fallback() {
    let (mount, handle) = connect(EmulatorConfig {
        silent_stop_axis: Some(1),
        ..Default::default()
    })
    .await;
    mount.rate(Az, 1.0).await;
    mount.rate(Alt, 1.0).await;
    advance(1).await;
    mount.stop_all().await;
    let wire = commands(&handle);
    let k1 = wire.iter().position(|c| c == ":K1\r").unwrap();
    let k2 = wire.iter().position(|c| c == ":K2\r").unwrap();
    let l1 = wire.iter().position(|c| c == ":L1\r").unwrap();
    assert!(k1 < k2 && k2 < l1);
    assert!(!wire.contains(&":L2\r".into()));
    advance(2).await;
    assert!(!mount.state(Az).await.moving);
    assert!(!mount.state(Alt).await.moving);
}

#[tokio::test(start_paused = true)]
async fn probe_order_and_report() {
    let (stream, handle, _) = stream(EmulatorConfig::default());
    let report = probe(stream).await.unwrap().to_string();
    assert_eq!(
        commands(&handle),
        [":e1\r", ":a1\r", ":b1\r", ":g1\r", ":q1010000\r", ":j1\r"]
    );
    for field in [
        "3.40",
        "0x90",
        "counts per revolution: 1620000",
        "timer frequency: 16000000",
        "high-speed ratio: 16",
        "auxiliary-encoder support: true",
        "axis position: 0",
    ] {
        assert!(report.contains(field), "{field} missing from {report}");
    }
}

#[tokio::test(start_paused = true)]
async fn transport_timeouts_and_replay_safety() {
    let (stream, handle, clock) = stream(EmulatorConfig::default());
    let mut transport = Transport::new(stream);
    handle.drop_replies(b'e', 1, 4);
    let before = clock.now();
    assert_eq!(
        transport.command(b'e', 1, b"", true).await.unwrap(),
        b"032890"
    );
    assert_eq!(commands(&handle).len(), 5);
    assert!((clock.now() - before - 1.2).abs() < 0.01);
    handle.drop_replies(b'a', 1, 5);
    assert!(matches!(
        transport.command(b'a', 1, b"", true).await,
        Err(Error::Timeout)
    ));
    assert_eq!(
        commands(&handle)
            .iter()
            .filter(|c| c.starts_with(":a"))
            .count(),
        5
    );
    handle.drop_replies(b'J', 1, 5);
    assert!(matches!(
        transport.command(b'J', 1, b"", false).await,
        Err(Error::Timeout)
    ));
    assert_eq!(
        commands(&handle)
            .iter()
            .filter(|c| c.starts_with(":J"))
            .count(),
        1
    );
}

#[tokio::test(start_paused = true)]
async fn position_wrap_zero_rate_and_lost_telemetry() {
    let (mount, handle) = connect(EmulatorConfig {
        auxiliary_encoders: false,
        ..Default::default()
    })
    .await;
    assert!(!mount.report().axes[1].auxiliary_encoders);
    assert!(!commands(&handle).iter().any(|c| c.starts_with(":W")));
    mount.goto(Alt, 8_388_600).await;
    advance(2000).await;
    assert_eq!(mount.state(Alt).await.position, 8_388_600);
    mount.goto(Alt, -8_388_600).await;
    advance(1).await;
    assert_eq!(mount.state(Alt).await.position, -8_388_600);
    assert!(commands(&handle).contains(&":S2080000\r".into()));
    mount.rate(Az, 1.0).await;
    advance(1).await;
    assert!(mount.state(Az).await.moving);
    mount.rate(Az, 0.0).await;
    assert!(!mount.state(Az).await.moving);
    handle.drop_replies(b'f', 2, 5);
    assert!(
        mount.state(Alt).await.moving,
        "missing telemetry cannot report rest"
    );
}

#[tokio::test(start_paused = true)]
async fn connection_and_motion_failures() {
    let (bytes, _, clock) = stream(EmulatorConfig {
        baud: 19200,
        ..Default::default()
    });
    assert!(matches!(
        SkyWatcher::connect(bytes, clock).await,
        Err(Error::Timeout)
    ));
    let (bytes, _, clock) = stream(EmulatorConfig {
        timer: [0; 2],
        ..Default::default()
    });
    assert!(matches!(
        SkyWatcher::connect(bytes, clock).await,
        Err(Error::InvalidReply)
    ));
    let (mount, handle) = connect(EmulatorConfig::default()).await;
    mount.rate(Az, 1.0).await;
    advance(1).await;
    assert!(mount.state(Az).await.moving);
    mount.rate(Az, f64::NAN).await;
    advance(2).await;
    assert!(!mount.state(Az).await.moving);
    assert!(commands(&handle).contains(&":K2\r".into()));
}
