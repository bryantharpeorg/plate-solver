//! US3's scenarios over the simulator's node and the sim rig of spec.md §
//! User Scenarios. Oracles: the worked examples of the scenarios, the pattern
//! camera's own rule and the mount's own state, computed here, never read from
//! the node or its service.
use pattern_database::PatternDatabase;
use scope_link::{memory, proto, BearerToken};
use scope_node::{
    clock::NodeClock,
    imaging::{
        BayerPattern, Exposure, ImagingCamera, ImagingFrame, ImagingFuture, ImagingInfo,
        ImagingSettings, ImagingStatus, Range, Roi,
    },
    service::NodeService,
    synthetic::SyntheticCamera,
    Settings,
};
use simulator::{
    mount::SimMount, SimCamera, SimImagingCamera, SimNode, Simulator, Site, VirtualClock,
};
use std::{
    path::Path,
    sync::{Arc, LazyLock, Mutex},
    time::Duration,
};
use tonic::{service::interceptor::InterceptedService, transport::Channel, Streaming};

type ImagingClient = proto::scope_node_imaging_client::ScopeNodeImagingClient<
    InterceptedService<Channel, BearerToken>,
>;
type CameraClient =
    proto::scope_node_client::ScopeNodeClient<InterceptedService<Channel, BearerToken>>;
type MountClient =
    proto::scope_node_mount_client::ScopeNodeMountClient<InterceptedService<Channel, BearerToken>>;

static DB: LazyLock<Arc<PatternDatabase>> = LazyLock::new(|| {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../reference-solutions/cedar-solve/tetra3/data/default_database.npz");
    Arc::new(pattern_database::load_from_path(&path).expect("bundled cedar database"))
});

/// The sim rig of spec.md § User Scenarios: the test database, 40 N 75 W, a
/// virtual clock from Unix 1768528800 and the tube pushed to alt 50, az 120.
fn rig() -> Simulator {
    let sim = Simulator::new(
        DB.clone(),
        Site::default(),
        VirtualClock::new(1_768_528_800.0),
    );
    sim.push_to(50.0, 120.0);
    sim
}

struct Clock(VirtualClock);
impl NodeClock for Clock {
    fn now(&self) -> f64 {
        self.0.node_now()
    }
}

fn imaging_client(channel: &Channel) -> ImagingClient {
    proto::scope_node_imaging_client::ScopeNodeImagingClient::with_interceptor(
        channel.clone(),
        BearerToken::new("test-token").unwrap(),
    )
}

fn camera_client(channel: &Channel) -> CameraClient {
    proto::scope_node_client::ScopeNodeClient::with_interceptor(
        channel.clone(),
        BearerToken::new("test-token").unwrap(),
    )
}

fn mount_client(channel: &Channel) -> MountClient {
    proto::scope_node_mount_client::ScopeNodeMountClient::with_interceptor(
        channel.clone(),
        BearerToken::new("test-token").unwrap(),
    )
}

/// One connection to a service's full set of routes.
async fn connect(service: &NodeService) -> Channel {
    let (connect, io) = memory::channel_pair();
    let routes = service.routes();
    tokio::spawn(async move {
        memory::serve_routes_on(io, routes).await.unwrap();
    });
    connect.await.unwrap()
}

/// One stream's header, then its chunks, then its end.
async fn header_and_chunks(
    stream: &mut Streaming<proto::ExposeEvent>,
) -> (proto::ImagingFrameHeader, Vec<Vec<u8>>) {
    let header = match stream.message().await.unwrap() {
        Some(proto::ExposeEvent {
            event: Some(proto::expose_event::Event::Header(header)),
        }) => header,
        _ => panic!("the stream's first message is not a header"),
    };
    let mut chunks = Vec::new();
    while let Some(item) = stream.message().await.unwrap() {
        match item {
            proto::ExposeEvent {
                event: Some(proto::expose_event::Event::Chunk(bytes)),
            } => chunks.push(bytes),
            _ => panic!("a message after the header is not a chunk"),
        }
    }
    (header, chunks)
}

fn request(exposure_ms: f64, purpose: &str) -> proto::ExposeRequest {
    proto::ExposeRequest {
        exposure_ms,
        purpose: purpose.into(),
        ..Default::default()
    }
}

/// An imaging camera of the test's own: a 4944 × 3284 16-bit frame whose i-th
/// sample is `i mod 65521`, computed here, never rendered.
const PATTERN_WIDTH: u32 = 4944;
const PATTERN_HEIGHT: u32 = 3284;
struct Pattern {
    clock: VirtualClock,
    exposing: Mutex<bool>,
}
impl Pattern {
    fn new(clock: VirtualClock) -> Self {
        Self {
            clock,
            exposing: Mutex::new(false),
        }
    }
}
impl ImagingCamera for Pattern {
    fn describe(&self) -> ImagingFuture<'_, ImagingInfo> {
        Box::pin(async {
            Ok(ImagingInfo {
                model: "pattern camera".into(),
                serial: None,
                width: PATTERN_WIDTH,
                height: PATTERN_HEIGHT,
                pixel_um: 4.78,
                bits: 16,
                bayer: Some(BayerPattern::Rggb),
                exposure_ms: Range {
                    min: 1.0,
                    max: 3_600_000.0,
                },
                gain: Some(Range {
                    min: 0.0,
                    max: 300.0,
                }),
                offset: Some(Range {
                    min: 0.0,
                    max: 100.0,
                }),
                binnings: vec![1, 2],
                cooler: true,
            })
        })
    }
    fn status(&self) -> ImagingFuture<'_, ImagingStatus> {
        Box::pin(async {
            Ok(ImagingStatus {
                temp_c: Some(20.0),
                cooler_power: Some(0.0),
                set_temp_c: None,
                exposing: *self.exposing.lock().unwrap(),
            })
        })
    }
    fn apply<'a>(&'a self, _: &'a ImagingSettings) -> ImagingFuture<'a, ImagingSettings> {
        Box::pin(async { Ok(ImagingSettings::default()) })
    }
    fn expose<'a>(&'a self, exposure: &'a Exposure) -> ImagingFuture<'a, ImagingFrame> {
        Box::pin(async move {
            let start_s = self.clock.node_now();
            *self.exposing.lock().unwrap() = true;
            tokio::time::sleep(Duration::from_secs_f64(exposure.exposure_ms / 1000.0)).await;
            *self.exposing.lock().unwrap() = false;
            let count = PATTERN_WIDTH as usize * PATTERN_HEIGHT as usize;
            let mut samples = Vec::with_capacity(count * 2);
            for i in 0..count {
                samples.extend_from_slice(&((i % 65521) as u16).to_le_bytes());
            }
            Ok(ImagingFrame {
                samples,
                width: PATTERN_WIDTH,
                height: PATTERN_HEIGHT,
                bits: 16,
                bayer: Some(BayerPattern::Rggb),
                start_s,
                end_s: self.clock.node_now(),
                exposure_ms: exposure.exposure_ms,
                gain: Some(100.0),
                offset: Some(10.0),
                binning: 1,
                temp_c: Some(20.0),
                roi: Roi {
                    x: 0,
                    y: 0,
                    width: PATTERN_WIDTH,
                    height: PATTERN_HEIGHT,
                },
            })
        })
    }
    fn abort(&self) -> ImagingFuture<'_, ()> {
        Box::pin(async { Ok(()) })
    }
}

/// One heartbeat the first connection kept: when it was sent, when its reply
/// arrived, and the reply's `t1`.
#[derive(Clone)]
struct Beat {
    sent: tokio::time::Instant,
    arrived: tokio::time::Instant,
    t1: f64,
}

/// Each stream's chunks: exactly 124, the first 123 of 262,144 bytes and the
/// last of 228,480, whose bytes are the camera's samples.
fn assert_pattern_chunks(chunks: &[(Vec<u8>, tokio::time::Instant)]) {
    assert_eq!(chunks.len(), 124);
    for (index, (bytes, _)) in chunks.iter().enumerate() {
        assert_eq!(
            bytes.len(),
            if index < 123 { 262_144 } else { 228_480 },
            "chunk {index}"
        );
    }
    let mut sample = 0usize;
    for (bytes, _) in chunks {
        for pair in bytes.as_chunks::<2>().0 {
            assert_eq!(u16::from_le_bytes(*pair), (sample % 65521) as u16);
            sample += 1;
        }
    }
    assert_eq!(sample, PATTERN_WIDTH as usize * PATTERN_HEIGHT as usize);
}

#[tokio::test(start_paused = true)]
async fn a_paced_expose_stream_never_holds_the_heartbeat() {
    let sim = rig();
    let clock = sim.clock().clone();
    let service = NodeService::with_mount(
        SyntheticCamera::new(Arc::new(Clock(clock.clone()))),
        Arc::new(Clock(clock.clone())),
        "test-token",
        Settings::default(),
        Arc::new(SimMount::new(clock.clone())) as Arc<dyn scope_node::mount::Mount>,
    )
    .unwrap()
    .with_imaging(Pattern::new(clock.clone()));
    // Two connections: the mount and the heartbeats on the first, the imaging
    // frames on the second.
    let first = connect(&service).await;
    let second = connect(&service).await;
    let mut mount = mount_client(&first);
    // The mount's azimuth rate, set on the first connection.
    mount
        .rate_axis(proto::RateAxisRequest {
            axis: proto::Axis::Azimuth as i32,
            deg_per_s: 0.5,
        })
        .await
        .unwrap();
    // Heartbeats every 250 ms, as crates/finder/src/link.rs:547 paces them,
    // keeping each reply.
    const HEARTBEAT: Duration = Duration::from_millis(250);
    let beats = Arc::new(Mutex::new(Vec::new()));
    let beater = {
        let mut camera = camera_client(&first);
        let beats = beats.clone();
        tokio::spawn(async move {
            let mut next_beat = tokio::time::Instant::now();
            loop {
                tokio::time::sleep_until(next_beat).await;
                let sent = tokio::time::Instant::now();
                let reply = camera
                    .heartbeat(proto::HeartbeatRequest { t0: 0.0 })
                    .await
                    .unwrap()
                    .into_inner();
                let arrived = tokio::time::Instant::now();
                beats.lock().unwrap().push(Beat {
                    sent,
                    arrived,
                    t1: reply.t1,
                });
                next_beat += HEARTBEAT;
            }
        })
    };

    let mut imaging = imaging_client(&second);
    // The limited stream: 4,194,304 bytes a second, at most.
    let mut stream = imaging
        .expose(proto::ExposeRequest {
            exposure_ms: 100.0,
            purpose: "frame".into(),
            max_bytes_per_s: 4_194_304,
            ..Default::default()
        })
        .await
        .unwrap()
        .into_inner();
    let (header, header_at) = match stream.message().await.unwrap() {
        Some(proto::ExposeEvent {
            event: Some(proto::expose_event::Event::Header(header)),
        }) => (header, tokio::time::Instant::now()),
        _ => panic!("the stream's first message is not a header"),
    };
    assert_eq!(header.byte_count, 32_472_192);
    let mut chunks: Vec<(Vec<u8>, tokio::time::Instant)> = Vec::new();
    while let Some(item) = stream.message().await.unwrap() {
        match item {
            proto::ExposeEvent {
                event: Some(proto::expose_event::Event::Chunk(bytes)),
            } => chunks.push((bytes, tokio::time::Instant::now())),
            _ => panic!("a message after the header is not a chunk"),
        }
    }
    // Exactly 124 chunks: 123 of 262,144 bytes and the last of 228,480, whose
    // bytes are the camera's samples.
    assert_pattern_chunks(&chunks);
    // The last chunk arrives 7.6875 s after the first, which tokio's timer
    // rounds up to 7.688 s; counting the chunk about to leave would give
    // 7.742 s.
    let elapsed = chunks.last().unwrap().1 - header_at;
    assert!(
        (Duration::from_secs_f64(7.68)..=Duration::from_secs_f64(7.70)).contains(&elapsed),
        "the last chunk arrived {elapsed:?} after the header"
    );
    // At least 30 heartbeats were answered between the header and the last
    // chunk, every reply's t1 0.250 s ± 1 ms after the one before, and every
    // reply in the virtual instant it was sent.
    let beats = beats.lock().unwrap().clone();
    let last_at = chunks.last().unwrap().1;
    let between = beats
        .iter()
        .filter(|beat| beat.arrived >= header_at && beat.arrived <= last_at)
        .count();
    assert!(
        between >= 30,
        "only {between} heartbeats between header and last chunk"
    );
    for beat in &beats {
        assert_eq!(beat.sent, beat.arrived, "a heartbeat reply was held");
    }
    for pair in beats.windows(2) {
        let gap = pair[1].t1 - pair[0].t1;
        assert!(
            (gap - 0.250).abs() <= 0.001,
            "t1 gap {gap} between two heartbeats"
        );
    }

    // The unlimited stream: no pacing, so its last chunk arrives in the same
    // virtual instant as its header.
    let mut stream = imaging
        .expose(request(100.0, "frame"))
        .await
        .unwrap()
        .into_inner();
    let (header, header_at) = match stream.message().await.unwrap() {
        Some(proto::ExposeEvent {
            event: Some(proto::expose_event::Event::Header(header)),
        }) => (header, tokio::time::Instant::now()),
        _ => panic!("the stream's first message is not a header"),
    };
    assert_eq!(header.byte_count, 32_472_192);
    let mut chunks: Vec<(Vec<u8>, tokio::time::Instant)> = Vec::new();
    while let Some(item) = stream.message().await.unwrap() {
        match item {
            proto::ExposeEvent {
                event: Some(proto::expose_event::Event::Chunk(bytes)),
            } => chunks.push((bytes, tokio::time::Instant::now())),
            _ => panic!("a message after the header is not a chunk"),
        }
    }
    assert_pattern_chunks(&chunks);
    assert_eq!(chunks.last().unwrap().1, header_at);

    // The mount kept moving and the watchdog never stopped it.
    let state = mount
        .axis_state(proto::AxisStateRequest {
            axis: proto::Axis::Azimuth as i32,
        })
        .await
        .unwrap()
        .into_inner();
    assert!(state.moving);
    assert_eq!(state.watchdog_stops, 0);
    beater.abort();
}

#[tokio::test(start_paused = true)]
async fn sim_node_serves_the_simulated_imaging_camera_beside_the_finder_camera() {
    let sim = rig();
    let node = SimNode::new(sim, "test-token", Settings::default()).unwrap();
    let channel = node.memory_channel().await.unwrap();
    let mut imaging = imaging_client(&channel);
    let info = imaging
        .describe_imaging(proto::DescribeImagingRequest {})
        .await
        .unwrap()
        .into_inner();
    assert_eq!(info.model, "simulated imaging camera");
    assert_eq!((info.width, info.height), (1236, 822));
    assert_eq!(info.bayer, "RGGB");
    let mut stream = imaging
        .expose(request(100.0, "frame"))
        .await
        .unwrap()
        .into_inner();
    let (header, chunks) = header_and_chunks(&mut stream).await;
    assert_eq!(header.byte_count, 2_031_984);
    assert_eq!(
        chunks.iter().map(|chunk| chunk.len()).sum::<usize>(),
        2_031_984
    );
    assert_eq!(chunks.len(), 8);
    assert!(chunks[..7].iter().all(|chunk| chunk.len() == 262_144));
    assert_eq!(chunks[7].len(), 196_976);
    // Describe on the same channel still answers the finder camera.
    let mut camera = camera_client(&channel);
    let describe = camera
        .describe(proto::DescribeRequest {})
        .await
        .unwrap()
        .into_inner();
    assert_eq!(describe.model, "simulator");
    assert_eq!((describe.width, describe.height), (1456, 1088));
}

#[tokio::test(start_paused = true)]
async fn from_cameras_attaches_the_callers_imaging_camera() {
    let sim = rig();
    let imaging = SimImagingCamera::new(sim.clone())
        .with_stars(vec![])
        .with_noise(false);
    let node = SimNode::from_cameras(
        sim.clone(),
        SimCamera::new(sim.clone()),
        imaging,
        "test-token",
        Settings::default(),
    )
    .unwrap();
    let channel = node.memory_channel().await.unwrap();
    let mut client = imaging_client(&channel);
    let mut stream = client
        .expose(request(100.0, "frame"))
        .await
        .unwrap()
        .into_inner();
    let (header, chunks) = header_and_chunks(&mut stream).await;
    assert_eq!(header.byte_count, 2_031_984);
    let samples = chunks.concat();
    assert_eq!(samples.len(), 2_031_984);
    // With no star and no noise a sample is 160 of offset and 2 electrons of
    // sky at the starting gain of 120, 0.25 × 10^0.6 counts an electron: 161.99
    // (the scenario's worked example).
    assert!(samples
        .as_chunks::<2>()
        .0
        .iter()
        .all(|pair| u16::from_le_bytes(*pair) == 162));
}
