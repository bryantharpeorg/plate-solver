//! US2-S1–S9. Oracles: literal config values and refusals, child exit and
//! output, the stand-in's properties and requests, and the supplied clock.
//! Comparisons are exact except the wall-clock frame's duration (1e-9 s).
mod finder_script;
mod support;

use finder_script::{
    camera, clock, exposure, finder, fits, flat, gain_step, light_request, ramp, reading, DEVICE,
};
use scope_link::{memory, proto, BearerToken};
use scope_node::{
    config::{Adapter, CameraMode, Config},
    imaging::indi::IndiCamera,
    indi_finder::IndiFinder,
    service::NodeService,
    synthetic::SyntheticCamera,
    Camera, Settings,
};
use std::{
    io,
    net::{SocketAddr, TcpListener},
    path::PathBuf,
    process::{Command, Output, Stdio},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use support::{colour, mono, set_number, Scripts, MONO_DEVICE};
use tokio::{task::JoinHandle, time::timeout};
use tonic::transport::Channel;

const HEAD: &str = "listen = '127.0.0.1:50051'\ntoken = 'test-token'\n";

fn parse(body: &str) -> Result<Config, scope_node::config::ConfigError> {
    Config::parse(&(HEAD.to_string() + body))
}

/// US2-S1: the table is optional; its supplied values and split neutral
/// control must survive parsing without any extra controls.
#[test]
fn s1_indi_config_is_optional_and_preserves_its_controls() {
    let config = parse("adapter = 'indi'\n").unwrap();
    assert_eq!(config.adapter, Adapter::Indi);
    assert!(config.indi.is_none());
    let config = parse(
        "adapter = 'indi'\n[indi]\naddress = '127.0.0.1:7700'\ndevice = 'Finder CCD'\n\
         neutral = { 'CCD_CONTROLS.Offset' = 20 }\n",
    )
    .unwrap();
    let indi = config.indi.unwrap();
    assert_eq!(indi.address(), "127.0.0.1:7700".parse().unwrap());
    assert_eq!(indi.device.as_deref(), Some("Finder CCD"));
    assert_eq!(
        indi.neutral_controls(),
        vec![("CCD_CONTROLS".to_string(), "Offset".to_string(), 20.0)]
    );
}

/// US2-S2: invalid finder-camera tables fail with the specified texts.
#[test]
fn s2_indi_config_refuses_wrong_adapter_empty_device_and_bad_neutral() {
    for (body, expected) in [
        (
            "adapter = 'rpicam'\n[indi]\ndevice = 'Finder CCD'\n",
            "the [indi] table applies only to adapter = \"indi\"",
        ),
        (
            "adapter = 'indi'\n[indi]\ndevice = ''\n",
            "indi.device must not be empty",
        ),
        (
            "adapter = 'indi'\n[indi]\nneutral = { 'Offset' = 20 }\n",
            "indi.neutral key Offset must be PROPERTY.ELEMENT",
        ),
        (
            "adapter = 'indi'\n[indi]\nneutral = { 'CCD_CONTROLS.Offset' = nan }\n",
            "indi.neutral CCD_CONTROLS.Offset must be a finite number",
        ),
    ] {
        assert_eq!(parse(body).err().unwrap().to_string(), expected);
    }
    assert_eq!(
        parse("adapter = 'indi'\n[indi]\nunknown = 1\n")
            .err()
            .unwrap()
            .to_string(),
        "invalid unknown at line 5"
    );
}

/// US2-S3: one server requires two distinct device names; a different
/// address or a simulated imaging camera does not impose that requirement.
#[test]
fn s3_two_cameras_on_one_indi_server_need_their_own_devices() {
    let shared = "the finder camera and the imaging camera share the INDI server at 127.0.0.1:7624: set indi.device and imaging.device";
    for (body, expected) in [
        (
            "[indi]\ndevice = 'Finder CCD'\n[imaging]\nadapter = 'indi'\n",
            shared,
        ),
        (
            "[imaging]\nadapter = 'indi'\ndevice = 'Imaging CCD'\n",
            shared,
        ),
        (
            "[indi]\ndevice = 'Same CCD'\n[imaging]\nadapter = 'indi'\ndevice = 'Same CCD'\n",
            "indi.device and imaging.device both name Same CCD: the finder camera and the imaging camera need a device each",
        ),
    ] {
        assert_eq!(
            parse(&("adapter = 'indi'\n".to_string() + body))
                .err()
                .unwrap()
                .to_string(),
            expected
        );
    }
    for body in [
        "[imaging]\nadapter = 'indi'\naddress = '127.0.0.1:7625'\n",
        "[indi]\ndevice = 'A'\n[imaging]\nadapter = 'indi'\ndevice = 'B'\n",
        "[imaging]\nadapter = 'simulated'\n",
    ] {
        assert!(parse(&("adapter = 'indi'\n".to_string() + body)).is_ok());
    }
}

struct Scratch(PathBuf);
impl Scratch {
    fn config(name: &str, text: &str) -> Self {
        let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("indi-finder-node-{name}-{}.toml", std::process::id()));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn free_address() -> SocketAddr {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
}

/// A bounded child run used by the synchronous config and startup cases.
struct Node {
    child: std::process::Child,
    _config: Scratch,
}
impl Node {
    fn spawn(name: &str, config: &str, check: bool) -> Self {
        let config = Scratch::config(name, config);
        let mut command = Command::new(env!("CARGO_BIN_EXE_scope-node"));
        command.arg("--config").arg(&config.0);
        if check {
            command.arg("--check-config");
        }
        Self {
            child: command
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap(),
            _config: config,
        }
    }

    fn output(mut self, limit: Duration) -> Output {
        let start = Instant::now();
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                use std::io::Read;
                let mut stdout = Vec::new();
                let mut stderr = Vec::new();
                self.child.stdout.take().unwrap().read_to_end(&mut stdout).unwrap();
                self.child.stderr.take().unwrap().read_to_end(&mut stderr).unwrap();
                return Output { status, stdout, stderr };
            }
            assert!(start.elapsed() < limit, "scope-node did not exit within {limit:?}");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
impl Drop for Node {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// US2-S4: check-config prints the default address and neutral controls,
/// masks the token, and does not add an INDI table to simulated output.
#[test]
fn s4_check_config_prints_indi_only_for_the_indi_adapter() {
    let config = HEAD.to_string()
        + "adapter = 'indi'\nexposure_ms = 1000\ngain = 30\n[indi]\ndevice = 'Finder CCD'\n\
           neutral = { 'CCD_CONTROLS.Offset' = 20 }\n";
    let output = Node::spawn("s4-indi", &config, true).output(Duration::from_secs(5));
    assert_eq!(output.status.code(), Some(0), "{:?}", output.stderr);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("adapter = \"indi\""), "{stdout}");
    assert!(stdout.contains("\n[indi]\naddress = \"127.0.0.1:7624\"\ndevice = \"Finder CCD\"\nneutral = { \"CCD_CONTROLS.Offset\" = 20 }\n"), "{stdout}");
    assert!(!stdout.contains("test-token"));
    assert!(stdout.find("[detection]").unwrap() < stdout.find("[indi]").unwrap());
    let output = Node::spawn(
        "s4-simulated",
        &(HEAD.to_string() + "adapter = 'simulated'\n"),
        true,
    )
    .output(Duration::from_secs(5));
    assert!(output.status.success());
    assert!(!String::from_utf8(output.stdout).unwrap().contains("[indi]"));
}

/// US2-S5: discovery fails before serving, using the finder-camera address
/// even when the imaging camera has a different unreachable server.
#[test]
fn s5_an_unreachable_finder_camera_exits_one_with_its_address() {
    // Keep both reservations until their ports have been chosen, so they differ.
    let finder = TcpListener::bind("127.0.0.1:0").unwrap();
    let imaging = TcpListener::bind("127.0.0.1:0").unwrap();
    let finder_address = finder.local_addr().unwrap();
    let imaging_address = imaging.local_addr().unwrap();
    drop((finder, imaging));
    let config = format!(
        "{HEAD}adapter = 'indi'\n[indi]\naddress = '{finder_address}'\n\
         [imaging]\nadapter = 'indi'\naddress = '{imaging_address}'\n"
    );
    let output = Node::spawn("s5-no-server", &config, false).output(Duration::from_secs(5));
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(
        stderr.lines().last().unwrap(),
        format!("scope-node: cannot reach the INDI server at {finder_address}: Connection refused (os error 111)")
    );
}

/// US2-S6: the real binary reaches the named mono camera beside a colour
/// camera, sends neutral controls and serves its description and first star frame.
#[tokio::test]
async fn s6_the_built_node_serves_the_named_indi_finder_camera() {
    let colour_script = colour();
    let colour_first = colour_script
        .split_once("@ send\n").unwrap().1
        .split_once("\n@ ").unwrap().0;
    // mono()'s device text is attached to an XML line, so no device
    // directive is active: naming the device is proved by its description.
    let script = mono()
        .replacen("@ send\n", &format!("@ send\n{colour_first}\n"), 1)
        .replace("@ expect after 200 enableBLOB", "@ expect enableBLOB")
        + "@ expect newNumberVector CCD_OFFSET OFFSET=20\n@ send\n"
        + &set_number(MONO_DEVICE, "CCD_OFFSET", "Ok", None, &[("OFFSET", 20.0)])
        + "\n@ expect newNumberVector CCD_EXPOSURE CCD_EXPOSURE_VALUE=0.1\n"
        + &finder_script::answers(MONO_DEVICE, 0.1, &ramp())
        + "@ expect newNumberVector CCD_EXPOSURE CCD_EXPOSURE_VALUE=0.1\n";
    let scripts = Scripts::new(vec![script]);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let connect = scripts.connect();
    let bridge = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut stand_in = connect().await.unwrap();
        let _ = tokio::io::copy_bidirectional(&mut socket, &mut stand_in).await;
    });
    let listen = free_address();
    let config = Scratch::config("s6-tcp", &format!(
        "listen = '{listen}'\ntoken = 'test-token'\nadapter = 'indi'\nexposure_ms = 100\ngain = 90\n\
         [indi]\naddress = '{address}'\ndevice = 'CCD Simulator'\nneutral = {{ 'CCD_OFFSET.OFFSET' = 20 }}\n"
    ));
    let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_scope-node"))
        .arg("--config").arg(&config.0)
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::piped())
        .kill_on_drop(true).spawn().unwrap();
    let (description, frame) = timeout(Duration::from_secs(10), async {
        let channel = loop {
            assert!(child.try_wait().unwrap().is_none(), "node exited before listening");
            if let Ok(channel) = Channel::from_shared(format!("http://{listen}"))
                .unwrap().connect().await {
                break channel;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        };
        let mut client = proto::scope_node_client::ScopeNodeClient::with_interceptor(
            channel, BearerToken::new("test-token").unwrap(),
        );
        let description = client.describe(proto::DescribeRequest {}).await.unwrap().into_inner();
        let frame = client.frames(proto::FramesRequest {}).await.unwrap()
            .into_inner().message().await.unwrap().unwrap();
        (description, frame)
    }).await.expect("description and first star frame within 10 s");
    assert_eq!(description.model, "CCD Simulator");
    assert_eq!((description.width, description.height), (1280, 1024));
    let mode = proto::Mode { width: 1280, height: 1024, fps: 1.0 };
    assert_eq!(description.modes, vec![mode]);
    assert_eq!(description.active_mode, Some(mode));
    assert_eq!((description.exposure_min_ms, description.exposure_max_ms), (10.0, 3_600_000.0));
    assert_eq!((description.gain_min, description.gain_max), (1.0, 300.0));
    assert_eq!((frame.width, frame.height), (3, 2));
    assert_eq!((frame.exposure_ms, frame.gain), (100.0, 90.0));
    assert!((frame.end_s - frame.start_s - 0.1).abs() < 1e-9);
    tokio::time::sleep(Duration::from_secs(1)).await;
    child.kill().await.unwrap();
    timeout(Duration::from_secs(5), bridge).await.unwrap().unwrap();
    scripts.finish().await;
}

struct ServiceRig {
    frames: tonic::Streaming<proto::StarFrame>,
    server: JoinHandle<()>,
}
impl Drop for ServiceRig {
    fn drop(&mut self) {
        self.server.abort();
    }
}
impl ServiceRig {
    async fn new(camera: IndiFinder<IndiCamera>, settings: Settings) -> Self {
        let service = NodeService::new(camera, clock(), "test-token", settings).unwrap();
        let routes = service.routes();
        let (connect, io) = memory::channel_pair();
        let server = tokio::spawn(async move {
            memory::serve_routes_on(io, routes).await.unwrap();
        });
        let frames = proto::scope_node_client::ScopeNodeClient::with_interceptor(
            connect.await.unwrap(), BearerToken::new("test-token").unwrap(),
        ).frames(proto::FramesRequest {}).await.unwrap().into_inner();
        Self { frames, server }
    }

    async fn frame(&mut self) -> proto::StarFrame {
        timeout(Duration::from_secs(60), self.frames.message())
            .await.expect("star frame within 60 s of paused time").unwrap().unwrap()
    }
}

/// US2-S7: three refused dials recover on the fifth, with ids continuing
/// and the gain resent before the first accepted exposure after recovery.
#[tokio::test(start_paused = true)]
async fn s7_the_service_recovers_after_three_refused_reconnections() {
    let image = flat(7);
    let first = finder(0.0) + &gain_step(30.0) + &exposure(0.5, &image)
        + &light_request(0.5) + "@ close\n";
    let second = finder(0.0) + &gain_step(30.0) + &exposure(0.5, &image) + &light_request(0.5);
    let scripts = Scripts::new(vec![first, second]);
    let dials = Arc::new(AtomicUsize::new(0));
    let count = dials.clone();
    let connect = scripts.connect();
    let camera = IndiCamera::new(
        Arc::new(move || {
            let dial = count.fetch_add(1, Ordering::SeqCst) + 1;
            let connect = connect.clone();
            Box::pin(async move {
                if (2..=4).contains(&dial) {
                    Err(io::Error::new(io::ErrorKind::ConnectionRefused, "server away"))
                } else {
                    connect().await
                }
            })
        }),
        "127.0.0.1:7624".to_string(), Some(DEVICE.to_string()), Vec::new(), clock(),
    );
    let camera = IndiFinder::discover(camera, CameraMode::Auto).await.unwrap();
    let mut rig = ServiceRig::new(camera, Settings {
        exposure_ms: Some(500.0), gain: Some(30.0), ..Settings::default()
    }).await;
    let first = rig.frame().await;
    let second = rig.frame().await;
    assert_eq!((first.id, first.gain, first.start_s), (1, 30.0, reading(400)));
    assert_eq!((second.id, second.gain, second.start_s), (2, 30.0, reading(5300)));
    assert_eq!(dials.load(Ordering::SeqCst), 5);
    tokio::time::sleep(Duration::from_millis(100)).await;
    scripts.finish().await;
}

/// US2-S8: the flat frame drives automatic gain through the literal
/// sequence 1, 4, 16, 64, 100 while the 1 fps mode caps exposure at 1000 ms.
#[tokio::test(start_paused = true)]
async fn s8_automatic_gain_climbs_and_holds_at_the_camera_limit() {
    let image = flat(7);
    let mut script = finder(0.0);
    for gain in [1.0, 4.0, 16.0, 64.0, 100.0] {
        script += &gain_step(gain);
        script += &exposure(1.0, &image);
    }
    script += &light_request(1.0);
    let scripts = Scripts::new(vec![script]);
    let camera = IndiFinder::discover(camera(&scripts, DEVICE), CameraMode::Auto).await.unwrap();
    let mut rig = ServiceRig::new(camera, Settings::default()).await;
    for gain in [1.0, 4.0, 16.0, 64.0, 100.0] {
        let frame = rig.frame().await;
        assert_eq!((frame.exposure_ms, frame.gain), (1000.0, gain));
    }
    tokio::time::sleep(Duration::from_millis(100)).await;
    scripts.finish().await;
}

/// US2-S9: a fresh device's gain-0 retry is discarded; the next star
/// frame retains automatic gain 1 and costs exactly one capture-loop retry.
#[tokio::test(start_paused = true)]
async fn s9_automatic_gain_recovers_from_a_server_restart_over_stars() {
    // Take the simulated star field before the finder-camera clock starts.
    let field = SyntheticCamera::new(clock()).next_frame().await.unwrap();
    let image = fits(field.width, field.height, 8, &field.pixels);
    let first = finder(0.0) + &gain_step(1.0) + &exposure(0.5, &image)
        + &light_request(0.5) + "@ close\n";
    let second = finder(0.0) + &exposure(0.5, &image) + &gain_step(1.0)
        + &exposure(0.5, &image) + &light_request(0.5);
    let scripts = Scripts::new(vec![first, second]);
    let camera = IndiFinder::discover(camera(&scripts, DEVICE), CameraMode::Auto).await.unwrap();
    let mut rig = ServiceRig::new(camera, Settings {
        exposure_ms: Some(500.0), ..Settings::default()
    }).await;
    let first = rig.frame().await;
    let second = rig.frame().await;
    assert!(first.centroids.len() >= 25, "{} centroids", first.centroids.len());
    assert!(second.centroids.len() >= 25, "{} centroids", second.centroids.len());
    assert_eq!((first.id, first.gain, first.start_s), (1, 1.0, reading(400)));
    assert_eq!((second.id, second.gain, second.start_s), (2, 1.0, reading(4000)));
    tokio::time::sleep(Duration::from_millis(100)).await;
    scripts.finish().await;
}
