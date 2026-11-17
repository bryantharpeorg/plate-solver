//! US3's link scenarios, one test each. Oracles: the pattern camera's own
//! sample rule, the worked chunk counts and pacing window of plan.md § The
//! node's service, and the simulator camera's constants (US3-S9's 162 is the
//! worked example of spec.md § User Scenarios).
use pattern_database::PatternDatabase;
use scope_link::{memory, proto, BearerToken};
use scope_node::{
    clock::NodeClock,
    imaging::{
        BayerPattern, Exposure, ImagingCamera, ImagingFrame, ImagingFuture, ImagingInfo,
        ImagingSettings, ImagingStatus, Range, Roi,
    },
    mount::Mount,
    service::NodeService,
    synthetic::SyntheticCamera,
    Settings,
};
use simulator::{
    mount::SimMount, SimCamera, SimImagingCamera, SimNode, Simulator, Site, VirtualClock,
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, LazyLock, Mutex,
    },
    time::Duration,
};
use tokio::time::Instant;
use tonic::{service::interceptor::InterceptedService, transport::Channel};

type ImagingClient = proto::scope_node_imaging_client::ScopeNodeImagingClient<
    InterceptedService<Channel, BearerToken>,
>;

static DB: LazyLock<Arc<PatternDatabase>> = LazyLock::new(|| {
    Arc::new(
        pattern_database::load_from_path(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../reference-solutions/cedar-solve/tetra3/data/default_database.npz"),
        )
        .unwrap(),
    )
});

/// The sim rig of spec.md § User Scenarios: the test database, 40 N 75 W, a
/// virtual clock from Unix 1768528800.
fn rig() -> Simulator {
    Simulator::new(
        DB.clone(),
        Site::default(),
        VirtualClock::new(1_768_528_800.0),
    )
}

fn imaging_client(channel: &Channel) -> ImagingClient {
    proto::scope_node_imaging_client::ScopeNodeImagingClient::with_interceptor(
        channel.clone(),
        BearerToken::new("test-token").unwrap(),
    )
}

async fn frame_header(
    stream: &mut tonic::Streaming<proto::ExposeEvent>,
) -> proto::ImagingFrameHeader {
    match stream.message().await.unwrap().unwrap().event {
        Some(proto::expose_event::Event::Header(header)) => header,
        other => panic!("the first item is not a header: {other:?}"),
    }
}

/// The stream's chunks, with the virtual instant the last of them arrived.
async fn stream_chunks(
    stream: &mut tonic::Streaming<proto::ExposeEvent>,
) -> (Vec<Vec<u8>>, Instant) {
    let mut chunks = Vec::new();
    let mut last = Instant::now();
    while let Some(event) = stream.message().await.unwrap() {
        match event.event {
            Some(proto::expose_event::Event::Chunk(bytes)) => {
                chunks.push(bytes);
                last = Instant::now();
            }
            other => panic!("a later item is not a chunk: {other:?}"),
        }
    }
    (chunks, last)
}

// The sim rig's clock as the node's clock, as `SimNode` serves it.
struct RigClock(VirtualClock);
impl NodeClock for RigClock {
    fn now(&self) -> f64 {
        self.0.node_now()
    }
}

/// The pattern camera's samples, computed here: the i-th of `samples` is
/// `i mod 65521`, little-endian.
fn pattern(samples: usize) -> Vec<u8> {
    let mut bytes = vec![0u8; samples * 2];
    for (i, pair) in bytes.as_chunks_mut::<2>().0.iter_mut().enumerate() {
        *pair = ((i % 65_521) as u16).to_le_bytes();
    }
    bytes
}

/// An imaging camera of the test's own: a 100 ms exposure answers the
/// 4944 × 3284 16-bit pattern of US3-S8, with no render.
struct PatternCamera {
    clock: Arc<dyn NodeClock>,
    exposing: Mutex<bool>,
}
impl ImagingCamera for PatternCamera {
    fn describe(&self) -> ImagingFuture<'_, ImagingInfo> {
        Box::pin(async {
            Ok(ImagingInfo {
                model: "pattern imaging camera".into(),
                serial: None,
                width: 4944,
                height: 3284,
                pixel_um: 4.78,
                bits: 16,
                bayer: Some(BayerPattern::Rggb),
                exposure_ms: Range {
                    min: 1.0,
                    max: 60_000.0,
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
    fn apply<'a>(&'a self, settings: &'a ImagingSettings) -> ImagingFuture<'a, ImagingSettings> {
        Box::pin(async move {
            Ok(ImagingSettings {
                gain: settings.gain.or(Some(100.0)),
                offset: settings.offset.or(Some(10.0)),
                binning: settings.binning.or(Some(1)),
                roi: settings.roi.or(Some(Roi {
                    x: 0,
                    y: 0,
                    width: 4944,
                    height: 3284,
                })),
                cooler_on: settings.cooler_on.or(Some(false)),
                set_temp_c: settings.set_temp_c,
            })
        })
    }
    fn expose<'a>(&'a self, exposure: &'a Exposure) -> ImagingFuture<'a, ImagingFrame> {
        Box::pin(async move {
            *self.exposing.lock().unwrap() = true;
            let start_s = self.clock.now();
            tokio::time::sleep(Duration::from_millis(100)).await;
            *self.exposing.lock().unwrap() = false;
            Ok(ImagingFrame {
                samples: pattern(4944 * 3284),
                width: 4944,
                height: 3284,
                bits: 16,
                bayer: Some(BayerPattern::Rggb),
                start_s,
                end_s: start_s + exposure.exposure_ms / 1000.0,
                exposure_ms: exposure.exposure_ms,
                gain: Some(100.0),
                offset: Some(10.0),
                binning: 1,
                temp_c: Some(20.0),
                roi: Roi {
                    x: 0,
                    y: 0,
                    width: 4944,
                    height: 3284,
                },
            })
        })
    }
    fn abort(&self) -> ImagingFuture<'_, ()> {
        Box::pin(async {
            *self.exposing.lock().unwrap() = false;
            Ok(())
        })
    }
}

/// One answered heartbeat, with the virtual instants it was sent and its
/// reply arrived.
#[derive(Clone)]
struct Beat {
    sent: Instant,
    t1: f64,
    arrived: Instant,
}

#[tokio::test(start_paused = true)]
async fn a_paced_frame_never_holds_a_heartbeat() {
    let clock = VirtualClock::new(1_768_528_800.0);
    let node_clock: Arc<dyn NodeClock> = Arc::new(RigClock(clock.clone()));
    let service = NodeService::with_mount(
        SyntheticCamera::new(node_clock.clone()),
        node_clock.clone(),
        "test-token",
        Settings::default(),
        Arc::new(SimMount::new(clock.clone())) as Arc<dyn Mount>,
    )
    .unwrap()
    .with_imaging(PatternCamera {
        clock: node_clock.clone(),
        exposing: Mutex::new(false),
    });

    // The same service over two connections: heartbeats on the first, frames
    // on the second.
    let mut servers = Vec::new();
    let mut channels = Vec::new();
    for _ in 0..2 {
        let (connect, io) = memory::channel_pair();
        let routes = service.routes();
        servers.push(tokio::spawn(async move {
            memory::serve_routes_on(io, routes).await.unwrap();
        }));
        channels.push(connect.await.unwrap());
    }
    let mut mount_client = proto::scope_node_mount_client::ScopeNodeMountClient::with_interceptor(
        channels[0].clone(),
        BearerToken::new("test-token").unwrap(),
    );
    let mut imaging_client = imaging_client(&channels[1]);

    // A task sending Heartbeat every 250 ms, as the finder paces it, keeping
    // each reply.
    let beats: Arc<Mutex<Vec<Beat>>> = Arc::new(Mutex::new(Vec::new()));
    let stop = Arc::new(AtomicBool::new(false));
    let beats_task = {
        let mut beating = proto::scope_node_client::ScopeNodeClient::with_interceptor(
            channels[0].clone(),
            BearerToken::new("test-token").unwrap(),
        );
        let beats = beats.clone();
        let stop = stop.clone();
        tokio::spawn(async move {
            let mut next = Instant::now();
            while !stop.load(Ordering::SeqCst) {
                tokio::time::sleep_until(next).await;
                next += Duration::from_millis(250);
                let sent = Instant::now();
                let reply = beating
                    .heartbeat(proto::HeartbeatRequest { t0: 0.0 })
                    .await
                    .unwrap()
                    .into_inner();
                beats.lock().unwrap().push(Beat {
                    sent,
                    t1: reply.t1,
                    arrived: Instant::now(),
                });
            }
        })
    };

    // The mount turns at 0.5°/s on the first connection.
    mount_client
        .rate_axis(proto::RateAxisRequest {
            axis: proto::Axis::Azimuth as i32,
            deg_per_s: 0.5,
        })
        .await
        .unwrap();

    // The limited stream: 32,472,192 bytes in 124 chunks at 4 MiB/s.
    let mut limited = imaging_client
        .expose(proto::ExposeRequest {
            exposure_ms: 100.0,
            purpose: "frame".into(),
            max_bytes_per_s: 4_194_304,
            ..Default::default()
        })
        .await
        .unwrap()
        .into_inner();
    let header = frame_header(&mut limited).await;
    assert_eq!(header.byte_count, 32_472_192);
    let header_at = Instant::now();
    let (limited_chunks, limited_last) = stream_chunks(&mut limited).await;
    assert_eq!(limited_chunks.len(), 124);
    for chunk in &limited_chunks[..123] {
        assert_eq!(chunk.len(), 262_144);
    }
    assert_eq!(limited_chunks[123].len(), 228_480);
    // The bytes are the camera's samples.
    let expected = pattern(4944 * 3284);
    let mut offset = 0;
    for chunk in &limited_chunks {
        assert_eq!(chunk, &expected[offset..offset + chunk.len()]);
        offset += chunk.len();
    }
    assert_eq!(offset, 32_472_192);
    // The last chunk leaves when 123 × 262,144 bytes have gone: 7.6875 s
    // after the first, which tokio's timer rounds up to 7.688 s.
    let elapsed = limited_last - header_at;
    assert!(
        elapsed >= Duration::from_secs_f64(7.68) && elapsed <= Duration::from_secs_f64(7.70),
        "the last chunk arrived {elapsed:?} after the header"
    );

    // The unlimited stream crosses in the same virtual instant as its header.
    let mut unlimited = imaging_client
        .expose(proto::ExposeRequest {
            exposure_ms: 100.0,
            purpose: "frame".into(),
            max_bytes_per_s: 0,
            ..Default::default()
        })
        .await
        .unwrap()
        .into_inner();
    let header = frame_header(&mut unlimited).await;
    assert_eq!(header.byte_count, 32_472_192);
    let at_header = Instant::now();
    let (unlimited_chunks, unlimited_last) = stream_chunks(&mut unlimited).await;
    assert_eq!(unlimited_chunks.len(), 124);
    for chunk in &unlimited_chunks[..123] {
        assert_eq!(chunk.len(), 262_144);
    }
    assert_eq!(unlimited_chunks[123].len(), 228_480);
    let mut offset = 0;
    for chunk in &unlimited_chunks {
        assert_eq!(chunk, &expected[offset..offset + chunk.len()]);
        offset += chunk.len();
    }
    assert_eq!(offset, 32_472_192);
    assert_eq!(unlimited_last, at_header);

    stop.store(true, Ordering::SeqCst);
    beats_task.await.unwrap();
    let beats = beats.lock().unwrap().clone();
    // Every reply arrived in the virtual instant it was sent.
    for beat in &beats {
        assert_eq!(beat.sent, beat.arrived);
    }
    // Every reply's t1 is 0.250 s ± 1 ms after the one before.
    for pair in beats.windows(2) {
        let gap = pair[1].t1 - pair[0].t1;
        assert!(
            (gap - 0.25).abs() <= 0.001,
            "heartbeat t1 advanced by {gap} s"
        );
    }
    // At least 30 heartbeats were answered inside the paced stream.
    let between = beats
        .iter()
        .filter(|beat| beat.sent >= header_at && beat.sent <= limited_last)
        .count();
    assert!(between >= 30, "{between} heartbeats answered in the window");

    // The mount kept moving and the watchdog never stopped it.
    let state = mount_client
        .axis_state(proto::AxisStateRequest {
            axis: proto::Axis::Azimuth as i32,
        })
        .await
        .unwrap()
        .into_inner();
    assert!(state.moving);
    assert_eq!(state.watchdog_stops, 0);
    for server in servers {
        server.abort();
    }
}

#[tokio::test(start_paused = true)]
async fn the_sim_node_serves_both_cameras_and_the_callers_own() {
    let simulator = rig();
    let node = SimNode::new(simulator.clone(), "test-token", Settings::default()).unwrap();
    let channel = node.memory_channel().await.unwrap();
    let mut imaging = imaging_client(&channel);
    let mut finder = proto::scope_node_client::ScopeNodeClient::with_interceptor(
        channel.clone(),
        BearerToken::new("test-token").unwrap(),
    );

    let info = imaging
        .describe_imaging(proto::DescribeImagingRequest {})
        .await
        .unwrap()
        .into_inner();
    assert_eq!(info.model, "simulated imaging camera");
    assert_eq!((info.width, info.height), (1236, 822));
    assert_eq!(info.bayer, "RGGB");

    let mut stream = imaging
        .expose(proto::ExposeRequest {
            exposure_ms: 100.0,
            purpose: "frame".into(),
            ..Default::default()
        })
        .await
        .unwrap()
        .into_inner();
    let header = frame_header(&mut stream).await;
    assert_eq!(header.byte_count, 2_031_984);
    let (pieces, _) = stream_chunks(&mut stream).await;
    assert_eq!(pieces.len(), 8);
    assert_eq!(pieces.concat().len(), 2_031_984);

    // The finder camera on the same channel is unchanged.
    let finder_info = finder
        .describe(proto::DescribeRequest {})
        .await
        .unwrap()
        .into_inner();
    assert_eq!(finder_info.model, "simulator");
    assert_eq!((finder_info.width, finder_info.height), (1456, 1088));

    // `from_cameras` attaches the caller's camera: with no star and no noise,
    // every sample is the worked 162.
    let camera = SimImagingCamera::new(simulator.clone())
        .with_stars(vec![])
        .with_noise(false);
    let own = SimNode::from_cameras(
        simulator.clone(),
        SimCamera::new(simulator.clone()),
        camera,
        "test-token",
        Settings::default(),
    )
    .unwrap();
    let channel = own.memory_channel().await.unwrap();
    let mut imaging = imaging_client(&channel);
    let mut stream = imaging
        .expose(proto::ExposeRequest {
            exposure_ms: 100.0,
            purpose: "frame".into(),
            ..Default::default()
        })
        .await
        .unwrap()
        .into_inner();
    let own_header = frame_header(&mut stream).await;
    assert_eq!(own_header.byte_count, 2_031_984);
    let (pieces, _) = stream_chunks(&mut stream).await;
    let bytes = pieces.concat();
    assert_eq!(bytes.len(), 2_031_984);
    assert!(bytes
        .as_chunks::<2>()
        .0
        .iter()
        .all(|pair| u16::from_le_bytes(*pair) == 162));
}
