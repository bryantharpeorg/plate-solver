//! US1 oracles: emulator board state and literal wire replies, plus the real binary.
//! Counts, identity and bytes are exact; waits use the scenario's supplied clock.
use scope_link::proto::{
    self, scope_node_mount_server::ScopeNodeMount, scope_node_server::ScopeNode,
    scope_node_stop_server::ScopeNodeStop,
};
use scope_node::{
    clock::NodeClock,
    mount::{
        skywatcher::{
            emulator::{Emulator, EmulatorConfig, EmulatorHandle},
            transport::Transport,
            MountLink, Opener, SkyWatcher,
        },
        Axis, Mount, MountIdentity,
    },
    service::NodeService,
    synthetic::SyntheticCamera,
    Settings,
};
use std::{net::TcpListener, path::PathBuf, process::Stdio, sync::Arc, time::Duration};
use tokio_stream::StreamExt;
use tonic::{Code, Request};

struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

// Same clock as skywatcher.rs: paused Tokio time is the independent test clock.
struct Clock(tokio::time::Instant);
impl NodeClock for Clock {
    fn now(&self) -> f64 {
        self.0.elapsed().as_secs_f64()
    }
}
fn clock() -> Arc<dyn NodeClock> {
    Arc::new(Clock(tokio::time::Instant::now()))
}
fn powered_off_link() -> (Arc<MountLink<Emulator>>, EmulatorHandle, Arc<dyn NodeClock>) {
    let clock = clock();
    let (_, handle) = Emulator::new(EmulatorConfig::default(), clock.clone());
    handle.power_off();
    let board = handle.clone();
    let reopen_clock = clock.clone();
    let open: Opener<Emulator> = Box::new(move || Ok(board.reopen(reopen_clock.clone())));
    (MountLink::start(open, clock.clone()), handle, clock)
}

#[tokio::test(start_paused = true)]
async fn starts_without_mount_and_connects_after_power_on() {
    let (link, handle, _) = powered_off_link();
    tokio::time::sleep(Duration::from_secs(10)).await;
    assert!(!link.connected());
    assert_eq!(link.identity(), None);
    let exchanges = handle.exchanges();
    assert!(!exchanges.is_empty());
    assert!(exchanges.iter().all(|e| e.reply.is_none()));

    let powered_on = exchanges.len();
    handle.power_on();
    tokio::time::sleep(Duration::from_secs(5)).await;
    assert!(link.connected());
    assert_eq!(link.axes().map(|a| a.counts_per_revolution), [1_620_000; 2]);
    let commands: Vec<_> = handle.exchanges()[powered_on..]
        .iter()
        .map(|e| e.command.clone())
        .collect();
    let indices = [":F3\r", ":W1050000\r", ":W2050000\r"]
        .map(|command| commands.iter().position(|c| c == command).unwrap());
    assert!(indices[0] < indices[1] && indices[1] < indices[2]);
    assert!(!handle.auxiliary_enabled(1));
    assert!(!handle.auxiliary_enabled(2));
    assert_eq!(
        link.identity(),
        Some(MountIdentity {
            firmware: "3.40".into(),
            board: "0x90".into(),
        })
    );
    assert_eq!(link.resets(), 0);
}

#[tokio::test(start_paused = true)]
async fn disconnected_service_refuses_moves_stops_at_once_and_serves_frames_without_counts() {
    let (link, handle, clock) = powered_off_link();
    let service = NodeService::with_mount(
        SyntheticCamera::new(clock.clone()),
        clock,
        "test-token",
        Settings::default(),
        link,
    )
    .unwrap();
    // The watchdog has tripped by now; disconnected refusals must take precedence.
    tokio::time::sleep(Duration::from_secs(5)).await;
    assert!(service
        .describe_mount(Request::new(proto::DescribeMountRequest {}))
        .await
        .unwrap()
        .into_inner()
        .axes
        .is_empty());
    let axis = proto::Axis::Azimuth as i32;
    let errors = [
        service
            .axis_state(Request::new(proto::AxisStateRequest { axis }))
            .await
            .unwrap_err(),
        service
            .goto_axis(Request::new(proto::GotoAxisRequest {
                axis,
                position: 50_000,
            }))
            .await
            .unwrap_err(),
        service
            .rate_axis(Request::new(proto::RateAxisRequest {
                axis,
                deg_per_s: 0.5,
            }))
            .await
            .unwrap_err(),
    ];
    for error in errors {
        assert_eq!(error.code(), Code::FailedPrecondition);
        assert_eq!(error.message(), "mount not connected");
    }
    let stopped = tokio::time::timeout(
        Duration::from_millis(10),
        service.stop_all(Request::new(proto::StopAllRequest {})),
    )
    .await;
    assert!(stopped.is_ok(), "Stop must not wait for the first connect");
    let error = stopped.unwrap().unwrap_err();
    assert_eq!(error.code(), Code::Unavailable);
    assert_eq!(error.message(), "the mount did not confirm the stop");

    let mut frames = service
        .frames(Request::new(proto::FramesRequest {}))
        .await
        .unwrap()
        .into_inner();
    let frame = frames.next().await.unwrap().unwrap();
    assert!(frame.axis_start.is_none());
    assert!(frame.axis_end.is_none());
    let exchanges = handle.exchanges();
    assert!(!exchanges.is_empty());
    // Only discovery/initialisation may reach the wire: no move, telemetry or Stop.
    assert!(exchanges.iter().all(|e| matches!(
        e.command.as_bytes()[1],
        b'e' | b'F' | b'a' | b'b' | b'g' | b'q' | b'W'
    )));
    assert!(exchanges.iter().all(|e| e.reply.is_none()));
}

#[tokio::test]
async fn binary_keeps_running_with_a_missing_mount_device() {
    let address = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap();
    // The temporary listener was dropped, so the binary can bind this free port.
    let directory = Scratch(
        PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("mount-link-process-{}", std::process::id())),
    );
    std::fs::create_dir_all(&directory.0).unwrap();
    let device = directory.0.join("missing-device");
    assert!(!device.exists());
    let config = directory.0.join("config.toml");
    std::fs::write(
        &config,
        format!(
            "listen = '{address}'\ntoken = 'test-token'\nadapter = 'simulated'\n[ mount ]\ndevice = '{}'\n",
            device.display()
        ),
    )
    .unwrap();
    let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_scope-node"))
        .arg("--config")
        .arg(&config)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    tokio::time::sleep(Duration::from_secs(5)).await;
    let running = child.try_wait().unwrap().is_none();
    if running {
        child.kill().await.unwrap();
    }
    let output = child.wait_with_output().await.unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(running, "scope-node exited: {stderr}");
    assert!(stderr.contains("mount not connected"), "{stderr}");
}

#[tokio::test(start_paused = true)]
async fn emulator_position_and_injected_reply_survive_the_expected_power_cycle() {
    let clock = clock();
    let (_, handle) = Emulator::new(EmulatorConfig::default(), clock.clone());
    let board = SkyWatcher::connect(handle.reopen(clock.clone()), clock)
        .await
        .unwrap();
    handle.set_position(1, 500_000);
    assert_eq!(board.state(Axis::Azimuth).await.position, 500_000);
    assert!(!handle.auxiliary_enabled(1));

    let injected = handle.exchanges().len();
    handle.inject_reply(b'f', 1, "=101\r", 2);
    board.state(Axis::Azimuth).await;
    handle.power_cycle();
    assert!(handle.auxiliary_enabled(1));
    board.state(Axis::Azimuth).await;
    assert_eq!(board.state(Axis::Azimuth).await.position, 0);
    let exchanges = handle.exchanges();
    let statuses: Vec<_> = exchanges[injected..]
        .iter()
        .filter(|e| e.command == ":f1\r")
        .map(|e| e.reply.as_deref())
        .collect();
    assert_eq!(statuses, [Some("=101\r"), Some("=101\r"), Some("=000\r")]);
    assert_eq!(
        exchanges
            .iter()
            .rev()
            .find(|e| e.command == ":j1\r")
            .unwrap()
            .reply
            .as_deref(),
        Some("=000080\r")
    );
}

#[tokio::test(start_paused = true)]
async fn injected_reply_waits_for_power_executes_the_command_and_overrides_its_reply() {
    let clock = clock();
    let (_, handle) = Emulator::new(
        EmulatorConfig {
            silent_stop_axis: Some(1),
            ..Default::default()
        },
        clock.clone(),
    );
    let mut transport = Transport::new(handle.reopen(clock));
    assert!(
        handle.baud_attempts().is_empty(),
        "reopen is not a baud change"
    );
    handle.inject_reply(b'F', 1, "=AB\r", 1);
    handle.power_off();
    assert!(transport.command(b'F', 1, b"", false).await.is_err());
    assert!(handle.exchanges().iter().all(|e| e.reply.is_none()));
    handle.power_on();
    // The powered-off attempt must neither initialise the board nor consume the fault.
    assert_eq!(
        transport.command(b'f', 1, b"", false).await.unwrap(),
        b"000"
    );
    assert_eq!(transport.command(b'F', 1, b"", false).await.unwrap(), b"AB");
    assert_eq!(
        transport.command(b'f', 1, b"", false).await.unwrap(),
        b"001"
    );
    handle.inject_reply(b'K', 1, "=CD\r", 1);
    assert_eq!(transport.command(b'K', 1, b"", false).await.unwrap(), b"CD");
}
