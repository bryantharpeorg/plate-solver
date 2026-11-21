//! US2-S1–S4: the shipped binary opens a pseudo-terminal as its mount device.
//! Oracles are the board's wire record and status replies, the child's status
//! and stderr, and the spec's literal wall-clock bounds (1 s, 1.4–2.5 s, 9 s).
use rustix::{
    fd::OwnedFd,
    fs::{Mode, OFlags},
    pty::{self, OpenptFlags},
    termios::{self, OptionalActions},
};
use scope_link::{
    proto::{
        self, scope_node_client::ScopeNodeClient, scope_node_mount_client::ScopeNodeMountClient,
    },
    BearerToken,
};
use scope_node::{
    clock::SystemClock,
    mount::skywatcher::emulator::{Emulator, EmulatorConfig, EmulatorHandle, Exchange},
};
use std::{
    net::{SocketAddr, TcpListener},
    path::PathBuf,
    process::{ExitStatus, Stdio},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::{Child, Command},
    task::JoinHandle,
};
use tonic::{service::interceptor::InterceptedService, transport::Channel, Code, Streaming};

type MountClient = ScopeNodeMountClient<InterceptedService<Channel, BearerToken>>;

struct Task(JoinHandle<()>);
impl Drop for Task {
    fn drop(&mut self) {
        self.0.abort();
    }
}

struct Terminal {
    device: String,
    handle: EmulatorHandle,
    _slave: OwnedFd,
    _relay: Task,
}
impl Terminal {
    fn new() -> Self {
        let master =
            pty::openpt(OpenptFlags::RDWR | OpenptFlags::NOCTTY | OpenptFlags::CLOEXEC).unwrap();
        pty::grantpt(&master).unwrap();
        pty::unlockpt(&master).unwrap();
        let name = pty::ptsname(&master, Vec::new()).unwrap();
        // Keep the slave open even after the child exits, so the relay survives
        // for the board readings two seconds later.
        let slave = rustix::fs::open(
            &name,
            OFlags::RDWR | OFlags::NOCTTY | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .unwrap();
        let mut settings = termios::tcgetattr(&slave).unwrap();
        settings.make_raw();
        termios::tcsetattr(&slave, OptionalActions::Now, &settings).unwrap();
        rustix::io::ioctl_fionbio(&master, true).unwrap();
        let (mut emulator, handle) =
            Emulator::new(EmulatorConfig::default(), Arc::new(SystemClock));
        let relay = Task(tokio::spawn(async move {
            let mut bytes = [0; 1024];
            loop {
                match rustix::io::read(&master, &mut bytes) {
                    Ok(0) => panic!("terminal master closed while its slave is held open"),
                    Ok(count) => {
                        emulator.write_all(&bytes[..count]).await.unwrap();
                        // Replies must return through the terminal, not remain
                        // unread inside the emulator.
                        let reply = emulator.discard_input();
                        let mut remaining = reply.as_slice();
                        while !remaining.is_empty() {
                            match rustix::io::write(&master, remaining) {
                                Ok(0) => panic!("terminal master accepted no reply bytes"),
                                Ok(count) => remaining = &remaining[count..],
                                Err(rustix::io::Errno::AGAIN) => {
                                    tokio::time::sleep(Duration::from_millis(1)).await;
                                }
                                Err(rustix::io::Errno::INTR) => continue,
                                Err(error) => panic!("writing terminal reply: {error}"),
                            }
                        }
                    }
                    Err(rustix::io::Errno::AGAIN) => {
                        tokio::time::sleep(Duration::from_millis(1)).await;
                    }
                    Err(rustix::io::Errno::INTR) => continue,
                    Err(error) => panic!("reading terminal command: {error}"),
                }
            }
        }));
        Self {
            device: name.to_str().unwrap().to_owned(),
            handle,
            _slave: slave,
            _relay: relay,
        }
    }
}

struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct Node {
    child: Child,
    stderr: JoinHandle<String>,
    _directory: Scratch,
}
impl Node {
    async fn start(name: &str, device: &str) -> (Self, SocketAddr) {
        let address = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap();
        let directory = Scratch(PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
            "exit-stops-the-mount-{}-{name}",
            std::process::id()
        )));
        std::fs::create_dir_all(&directory.0).unwrap();
        let config = directory.0.join("config.toml");
        std::fs::write(
            &config,
            format!(
                "listen = '{address}'\ntoken = 'test-token'\nadapter = 'simulated'\n\
                 [mount]\ndevice = '{device}'\n"
            ),
        )
        .unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_scope-node"))
            .arg("--config")
            .arg(&config)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let mut pipe = child.stderr.take().unwrap();
        let stderr = tokio::spawn(async move {
            let mut text = String::new();
            pipe.read_to_string(&mut text).await.unwrap();
            text
        });
        let mut node = Self {
            child,
            stderr,
            _directory: directory,
        };
        node.wait_for_listener(address).await;
        (node, address)
    }

    async fn wait_for_listener(&mut self, address: SocketAddr) {
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                assert!(
                    self.child.try_wait().unwrap().is_none(),
                    "node exited before listening"
                );
                if tokio::net::TcpStream::connect(address).await.is_ok() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("scope-node must listen within 10 s");
    }

    async fn signal(&self, signal: &str) -> Instant {
        let pid = self.child.id().expect("our child is running");
        let started = Instant::now();
        let sent = Command::new("kill")
            .arg(format!("-{signal}"))
            .arg(pid.to_string())
            .status()
            .await
            .unwrap();
        assert!(sent.success(), "kill must succeed for our own child");
        started
    }

    async fn exit(mut self, started: Instant, wait: Duration) -> (ExitStatus, String, Duration) {
        let status = tokio::time::timeout(wait, self.child.wait())
            .await
            .expect("scope-node must exit before the test's timeout")
            .unwrap();
        let elapsed = started.elapsed();
        let stderr = self.stderr.await.unwrap();
        (status, stderr, elapsed)
    }
}

struct Moving {
    terminal: Terminal,
    client: MountClient,
    _heartbeat: Task,
    // Intentionally unread and alive until the test ends: a graceful server
    // drain must not delay the node's exit behind this stream.
    _frames: Streaming<proto::StarFrame>,
}
impl Moving {
    async fn start(name: &str) -> (Node, Self) {
        let terminal = Terminal::new();
        let (node, address) = Node::start(name, &terminal.device).await;
        let channel = Channel::from_shared(format!("http://{address}"))
            .unwrap()
            .connect()
            .await
            .unwrap();
        let mut client = ScopeNodeMountClient::with_interceptor(
            channel.clone(),
            BearerToken::new("test-token").unwrap(),
        );
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let description = client
                    .describe_mount(proto::DescribeMountRequest {})
                    .await
                    .unwrap()
                    .into_inner();
                if !description.axes.is_empty() {
                    assert_eq!(description.axes.len(), 2);
                    break;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("the mount must connect within 10 s");

        let mut camera =
            ScopeNodeClient::with_interceptor(channel, BearerToken::new("test-token").unwrap());
        let mut beats = camera.clone();
        let (first_beat, ready) = tokio::sync::oneshot::channel();
        let heartbeat = Task(tokio::spawn(async move {
            let mut first_beat = Some(first_beat);
            loop {
                if beats
                    .heartbeat(proto::HeartbeatRequest { t0: 0.0 })
                    .await
                    .is_err()
                {
                    break;
                }
                if let Some(first_beat) = first_beat.take() {
                    let _ = first_beat.send(());
                }
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
        }));
        tokio::time::timeout(Duration::from_secs(5), ready)
            .await
            .expect("first heartbeat must answer")
            .unwrap();
        let frames = camera
            .frames(proto::FramesRequest {})
            .await
            .unwrap()
            .into_inner();
        client
            .goto_axis(proto::GotoAxisRequest {
                axis: proto::Axis::Azimuth as i32,
                position: 1_000_000,
            })
            .await
            .unwrap();
        client
            .rate_axis(proto::RateAxisRequest {
                axis: proto::Axis::Altitude as i32,
                deg_per_s: 1.0,
            })
            .await
            .unwrap();
        let moving = Self {
            terminal,
            client,
            _heartbeat: heartbeat,
            _frames: frames,
        };
        tokio::time::sleep(Duration::from_secs(1)).await;
        assert_eq!(moving.both_moving().await, [true, true]);
        (node, moving)
    }

    async fn both_moving(&self) -> [bool; 2] {
        // Ask the board directly after the process exits too; no node-side
        // state or adapter decoder supplies this independent oracle.
        let mut board = self.terminal.handle.reopen(Arc::new(SystemClock));
        let mut moving = [false; 2];
        for (index, command) in [b":f1\r", b":f2\r"].into_iter().enumerate() {
            board.write_all(command).await.unwrap();
            let mut reply = [0; 5];
            tokio::time::timeout(Duration::from_secs(1), board.read_exact(&mut reply))
                .await
                .expect("the board must answer its status query")
                .unwrap();
            assert_eq!(reply[0], b'=', "{reply:?}");
            assert_eq!(reply[4], b'\r', "{reply:?}");
            assert!(matches!(reply[2], b'0' | b'1'), "{reply:?}");
            moving[index] = reply[2] == b'1';
        }
        moving
    }

    fn mark(&self) -> usize {
        self.terminal.handle.exchanges().len()
    }

    fn since(&self, mark: usize) -> Vec<Exchange> {
        self.terminal.handle.exchanges()[mark..].to_vec()
    }
}

fn stops(events: &[Exchange]) -> Vec<(&str, Option<&str>)> {
    events
        .iter()
        .filter(|e| e.command.starts_with(":K") || e.command.starts_with(":L"))
        .map(|e| (e.command.as_str(), e.reply.as_deref()))
        .collect()
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

async fn healthy_stop(name: &str, signal: &str, last_line: &str) {
    let (node, moving) = Moving::start(name).await;
    let mark = moving.mark();
    let started = node.signal(signal).await;
    let (status, stderr, elapsed) = node.exit(started, Duration::from_secs(5)).await;
    assert!(elapsed < Duration::from_secs(1), "exit took {elapsed:?}");
    assert_eq!(status.code(), Some(0), "{stderr}");
    assert_eq!(stderr.lines().last(), Some(last_line), "{stderr}");
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert_eq!(moving.both_moving().await, [false, false]);
    let events = moving.since(mark);
    assert_eq!(
        stops(&events),
        [(":K1\r", Some("=\r")), (":K2\r", Some("=\r"))]
    );
    assert_no_motion(&events);
}

#[tokio::test]
async fn s1_sigterm_stops_both_axes_and_exits_with_an_open_frame_stream() {
    healthy_stop(
        "term",
        "TERM",
        "scope-node: SIGTERM: the mount confirmed the stop",
    )
    .await;
}

#[tokio::test]
async fn s2_sigint_stops_both_axes_and_exits_with_an_open_frame_stream() {
    healthy_stop(
        "int",
        "INT",
        "scope-node: SIGINT: the mount confirmed the stop",
    )
    .await;
}

#[tokio::test]
async fn s3_second_signal_and_late_move_do_not_interrupt_the_stop() {
    let (node, moving) = Moving::start("second-signal").await;
    moving.terminal.handle.drop_replies(b'K', 1, 5);
    let mark = moving.mark();
    let started = node.signal("TERM").await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    node.signal("INT").await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    let mut client = moving.client.clone();
    let (late_move, (status, stderr, elapsed)) = tokio::join!(
        async {
            tokio::time::timeout(
                Duration::from_secs(5),
                client.goto_axis(proto::GotoAxisRequest {
                    axis: proto::Axis::Altitude as i32,
                    position: 50_000,
                }),
            )
            .await
            .expect("the late call must finish when the node exits")
        },
        node.exit(started, Duration::from_secs(5))
    );
    assert_ne!(late_move.unwrap_err().code(), Code::Aborted);
    assert!(
        (Duration::from_millis(1_400)..=Duration::from_millis(2_500)).contains(&elapsed),
        "exit took {elapsed:?}"
    );
    assert_eq!(status.code(), Some(0), "{stderr}");
    assert_eq!(
        stderr
            .lines()
            .filter(|line| line.starts_with("scope-node: SIG"))
            .collect::<Vec<_>>(),
        ["scope-node: SIGTERM: the mount confirmed the stop"],
        "{stderr}"
    );
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert_eq!(moving.both_moving().await, [false, false]);
    let events = moving.since(mark);
    let stops = stops(&events);
    assert!(stops.len() >= 7, "{stops:?}");
    assert_eq!(
        &stops[..7],
        [
            (":K1\r", None),
            (":K1\r", None),
            (":K1\r", None),
            (":K1\r", None),
            (":K1\r", None),
            (":K2\r", Some("=\r")),
            (":L1\r", Some("=\r")),
        ]
    );
    assert!(stops[7..].iter().all(|(c, _)| c.starts_with(":K")));
    assert_no_motion(&events);
}

#[tokio::test]
async fn s4_powered_off_board_exits_one_without_confirmation_within_nine_seconds() {
    let (node, moving) = Moving::start("power-off").await;
    moving.terminal.handle.power_off();
    let started = node.signal("TERM").await;
    let (status, stderr, elapsed) = node.exit(started, Duration::from_secs(15)).await;
    assert!(elapsed < Duration::from_secs(9), "exit took {elapsed:?}");
    assert_eq!(status.code(), Some(1), "{stderr}");
    assert!(
        stderr.lines().last().is_some_and(
            |line| line.starts_with("scope-node: SIGTERM: the mount did not confirm the stop")
        ),
        "{stderr}"
    );
}
