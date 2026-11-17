//! US3's scenarios over the node rig of spec.md § User Scenarios. Oracles:
//! plan.md § The stand-in camera's sample rule and § The port, with every
//! expected value computed here, not read from the camera or the service.
use scope_link::{memory, proto, BearerToken};
use scope_node::{
    clock::NodeClock,
    imaging::{
        synthetic::SyntheticImagingCamera, BayerPattern, Exposure, ImagingCamera, ImagingError,
        ImagingFrame, ImagingFuture, ImagingInfo, ImagingSettings, ImagingStatus, Range, Roi,
    },
    service::NodeService,
    synthetic::SyntheticCamera,
    Settings,
};
use std::{sync::Arc, time::Duration};
use tokio::{task::JoinHandle, time};
use tonic::{service::interceptor::InterceptedService, transport::Channel, Code, Streaming};

type ImagingClient = proto::scope_node_imaging_client::ScopeNodeImagingClient<
    InterceptedService<Channel, BearerToken>,
>;
type CameraClient =
    proto::scope_node_client::ScopeNodeClient<InterceptedService<Channel, BearerToken>>;

// The test clock of spec.md § User Scenarios: 1000.0 plus the tokio time
// elapsed since it was built, so paused time advances it exactly.
struct Clock(tokio::time::Instant);
impl NodeClock for Clock {
    fn now(&self) -> f64 {
        1000.0 + self.0.elapsed().as_secs_f64()
    }
}

/// The sample rule of plan.md § The stand-in camera, computed here: with
/// binning `b`, region origin `(x0, y0)` and offset `o` in force, the sample
/// at column `c` and row `r` is `256 + 16·round(o) + ((x0/b + c + 3·(y0/b + r))
/// mod 251)`, little-endian, row by row from the top-left.
fn sample_rule(roi: Roi, binning: u32, offset: f64, width: u32, height: u32) -> Vec<u8> {
    let base = 256.0 + 16.0 * offset.round();
    let mut bytes = Vec::with_capacity(width as usize * height as usize * 2);
    for row in 0..height {
        for column in 0..width {
            let index = (roi.x / binning + column + 3 * (roi.y / binning + row)) % 251;
            bytes.extend_from_slice(&((base + f64::from(index)) as u16).to_le_bytes());
        }
    }
    bytes
}

fn roi(x: u32, y: u32, width: u32, height: u32) -> Option<proto::ImagingRoi> {
    Some(proto::ImagingRoi {
        x,
        y,
        width,
        height,
    })
}

fn request(exposure_ms: f64, purpose: &str) -> proto::ExposeRequest {
    proto::ExposeRequest {
        exposure_ms,
        purpose: purpose.into(),
        ..Default::default()
    }
}

/// The stream's first item, which every failure after the token check is.
async fn refused(stream: &mut Streaming<proto::ExposeEvent>) -> tonic::Status {
    match stream.message().await {
        Err(status) => status,
        Ok(Some(_)) => panic!("the stream's first item is not a failure"),
        Ok(None) => panic!("the stream ended before its first item"),
    }
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

/// The node rig of spec.md § User Scenarios: the node's service over the
/// synthetic camera on the test clock, served over one connection a client.
struct Rig {
    clock: Arc<Clock>,
    service: NodeService,
    channel: Channel,
    servers: Vec<JoinHandle<()>>,
}
impl Drop for Rig {
    fn drop(&mut self) {
        for server in &self.servers {
            server.abort();
        }
    }
}
impl Rig {
    /// The node rig with the stand-in imaging camera.
    async fn new() -> Self {
        let clock: Arc<Clock> = Arc::new(Clock(tokio::time::Instant::now()));
        let service =
            Self::plain(clock.clone()).with_imaging(SyntheticImagingCamera::new(clock.clone()));
        Self::serve(clock, service).await
    }

    /// The node rig over an imaging camera of the test's own.
    async fn with_camera(camera: impl ImagingCamera + 'static) -> Self {
        let clock: Arc<Clock> = Arc::new(Clock(tokio::time::Instant::now()));
        let service = Self::plain(clock.clone()).with_imaging(camera);
        Self::serve(clock, service).await
    }

    /// The node rig built without `with_imaging`.
    async fn without_imaging() -> Self {
        let clock: Arc<Clock> = Arc::new(Clock(tokio::time::Instant::now()));
        let service = Self::plain(clock.clone());
        Self::serve(clock, service).await
    }

    fn plain(clock: Arc<Clock>) -> NodeService {
        NodeService::new(
            SyntheticCamera::new(clock.clone()),
            clock,
            "test-token",
            Settings::default(),
        )
        .unwrap()
    }

    async fn serve(clock: Arc<Clock>, service: NodeService) -> Self {
        let (connect, io) = memory::channel_pair();
        let routes = service.routes();
        let server = tokio::spawn(async move {
            memory::serve_routes_on(io, routes).await.unwrap();
        });
        let channel = connect.await.unwrap();
        Self {
            clock,
            service,
            channel,
            servers: vec![server],
        }
    }

    fn imaging(&self, token: &str) -> ImagingClient {
        proto::scope_node_imaging_client::ScopeNodeImagingClient::with_interceptor(
            self.channel.clone(),
            BearerToken::new(token).unwrap(),
        )
    }

    fn camera(&self) -> CameraClient {
        proto::scope_node_client::ScopeNodeClient::with_interceptor(
            self.channel.clone(),
            BearerToken::new("test-token").unwrap(),
        )
    }

    /// A connection served by `into_server()` alone, as a node built before
    /// this spec.
    async fn legacy(&mut self) -> ImagingClient {
        let service = self.service.clone();
        let (connect, io) = memory::channel_pair();
        let server = tokio::spawn(async move {
            memory::serve_on(io, service.into_server()).await.unwrap();
        });
        self.servers.push(server);
        let channel = connect.await.unwrap();
        proto::scope_node_imaging_client::ScopeNodeImagingClient::with_interceptor(
            channel,
            BearerToken::new("test-token").unwrap(),
        )
    }
}

#[tokio::test(start_paused = true)]
async fn the_three_unary_calls_answer_the_stand_in_camera() {
    let rig = Rig::new().await;
    let mut c = rig.imaging("test-token");
    let info = c
        .describe_imaging(proto::DescribeImagingRequest {})
        .await
        .unwrap()
        .into_inner();
    assert_eq!(info.model, "synthetic imaging camera");
    assert_eq!(info.serial, None);
    assert_eq!(info.pixel_um, 4.78);
    assert_eq!((info.width, info.height, info.bits), (64, 48, 16));
    assert_eq!(info.bayer, "RGGB");
    assert_eq!(
        info.exposure_ms,
        Some(proto::ImagingRange {
            min: 1.0,
            max: 60_000.0,
        })
    );
    assert_eq!(
        info.gain,
        Some(proto::ImagingRange {
            min: 0.0,
            max: 300.0,
        })
    );
    assert_eq!(
        info.offset,
        Some(proto::ImagingRange {
            min: 0.0,
            max: 100.0,
        })
    );
    assert_eq!(info.binnings, [1, 2]);
    assert!(info.cooler);
    let status = c
        .imaging_status(proto::ImagingStatusRequest {})
        .await
        .unwrap()
        .into_inner();
    assert_eq!(
        (
            status.temp_c,
            status.cooler_power,
            status.set_temp_c,
            status.exposing
        ),
        (Some(20.0), Some(0.0), None, false)
    );
    let empty = c
        .configure_imaging(proto::ImagingSettings::default())
        .await
        .unwrap()
        .into_inner();
    assert_eq!(
        (
            empty.gain,
            empty.offset,
            empty.binning,
            empty.roi,
            empty.cooler_on,
            empty.set_temp_c
        ),
        (
            Some(100.0),
            Some(10.0),
            Some(1),
            roi(0, 0, 64, 48),
            Some(false),
            None
        )
    );
    let changed = c
        .configure_imaging(proto::ImagingSettings {
            gain: Some(200.0),
            ..Default::default()
        })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(changed.gain, Some(200.0));
    assert_eq!(
        (
            changed.offset,
            changed.binning,
            changed.roi,
            changed.cooler_on,
            changed.set_temp_c
        ),
        (
            empty.offset,
            empty.binning,
            empty.roi,
            empty.cooler_on,
            None
        )
    );
    let refused = c
        .configure_imaging(proto::ImagingSettings {
            gain: Some(301.0),
            ..Default::default()
        })
        .await
        .unwrap_err();
    assert_eq!(refused.code(), Code::InvalidArgument);
    assert_eq!(
        refused.message(),
        "gain 301 is outside the camera's range 0–300"
    );
}

#[tokio::test(start_paused = true)]
async fn expose_answers_a_header_then_the_samples_and_the_ids_count() {
    let rig = Rig::new().await;
    let mut c = rig.imaging("test-token");
    let whole = Roi {
        x: 0,
        y: 0,
        width: 64,
        height: 48,
    };
    for id in [1, 2] {
        let at_call = rig.clock.now();
        let mut stream = c
            .expose(request(500.0, "frame"))
            .await
            .unwrap()
            .into_inner();
        let (header, chunks) = header_and_chunks(&mut stream).await;
        assert_eq!(header.id, id);
        assert_eq!((header.width, header.height, header.bits), (64, 48, 16));
        assert_eq!(header.bayer, "RGGB");
        assert_eq!(header.exposure_ms, 500.0);
        assert_eq!(header.gain, Some(100.0));
        assert_eq!(header.offset, Some(10.0));
        assert_eq!(header.binning, 1);
        assert_eq!(header.temp_c, Some(20.0));
        assert_eq!(header.purpose, "frame");
        assert_eq!(header.byte_count, 6144);
        assert_eq!(header.roi, roi(0, 0, 64, 48));
        assert_eq!(header.start_s, at_call);
        assert_eq!(header.end_s, at_call + 0.5);
        assert_eq!(
            (header.node_file.as_str(), header.node_save_error.as_str()),
            ("", "")
        );
        assert!(chunks.iter().all(|chunk| chunk.len() <= 262_144));
        let bytes = chunks.concat();
        assert_eq!(bytes.len(), 6144);
        assert_eq!(bytes, sample_rule(whole, 1, 10.0, 64, 48));
    }
}

#[tokio::test(start_paused = true)]
async fn a_configured_region_and_binning_are_the_next_headers_own_roi() {
    let rig = Rig::new().await;
    let mut c = rig.imaging("test-token");
    let answer = c
        .configure_imaging(proto::ImagingSettings {
            binning: Some(2),
            roi: roi(8, 4, 16, 8),
            ..Default::default()
        })
        .await
        .unwrap()
        .into_inner();
    assert_eq!((answer.binning, answer.roi), (Some(2), roi(8, 4, 16, 8)));
    let mut stream = c
        .expose(request(500.0, "frame"))
        .await
        .unwrap()
        .into_inner();
    let (header, chunks) = header_and_chunks(&mut stream).await;
    // The region in force, in unbinned sensor pixels, and the frame's own
    // delivered size: 8 × 4 over binning 2 (US1-S4's oracle).
    assert_eq!(header.roi, roi(8, 4, 16, 8));
    assert_eq!(header.binning, 2);
    assert_eq!((header.width, header.height), (8, 4));
    assert_eq!(header.byte_count, 64);
    let region = Roi {
        x: 8,
        y: 4,
        width: 16,
        height: 8,
    };
    assert_eq!(chunks.concat(), sample_rule(region, 2, 10.0, 8, 4));
}

#[tokio::test(start_paused = true)]
async fn each_call_requires_the_pairing_token() {
    let rig = Rig::new().await;
    let mut c = rig.imaging("wrong-token");
    let describe = c
        .describe_imaging(proto::DescribeImagingRequest {})
        .await
        .unwrap_err();
    let status = c
        .imaging_status(proto::ImagingStatusRequest {})
        .await
        .unwrap_err();
    let configure = c
        .configure_imaging(proto::ImagingSettings::default())
        .await
        .unwrap_err();
    let abort = c
        .abort_exposure(proto::AbortExposureRequest {})
        .await
        .unwrap_err();
    // The token check alone answers as the call's own status.
    let expose = c.expose(request(500.0, "frame")).await.unwrap_err();
    for error in [describe, status, configure, abort, expose] {
        assert_eq!(error.code(), Code::Unauthenticated);
    }
}

#[tokio::test(start_paused = true)]
async fn a_node_without_an_imaging_camera_answers_failed_precondition() {
    let rig = Rig::without_imaging().await;
    let mut c = rig.imaging("test-token");
    for call in [
        c.describe_imaging(proto::DescribeImagingRequest {})
            .await
            .unwrap_err(),
        c.imaging_status(proto::ImagingStatusRequest {})
            .await
            .unwrap_err(),
        c.configure_imaging(proto::ImagingSettings::default())
            .await
            .unwrap_err(),
        c.abort_exposure(proto::AbortExposureRequest {})
            .await
            .unwrap_err(),
    ] {
        assert_eq!(call.code(), Code::FailedPrecondition);
        assert_eq!(call.message(), "no imaging camera");
    }
    let mut stream = c
        .expose(request(500.0, "frame"))
        .await
        .unwrap()
        .into_inner();
    let status = refused(&mut stream).await;
    assert_eq!(status.code(), Code::FailedPrecondition);
    assert_eq!(status.message(), "no imaging camera");
    assert!(stream.message().await.unwrap().is_none());
}

#[tokio::test(start_paused = true)]
async fn a_connection_without_the_imaging_service_answers_unimplemented() {
    let mut rig = Rig::new().await;
    let mut legacy = rig.legacy().await;
    let error = legacy
        .describe_imaging(proto::DescribeImagingRequest {})
        .await
        .unwrap_err();
    assert_eq!(error.code(), Code::Unimplemented);
}

#[tokio::test(start_paused = true)]
async fn an_unresponsive_node_answers_neither_imaging_nor_describe() {
    let rig = Rig::new().await;
    rig.service.controls().set_unresponsive(true);
    let mut c = rig.imaging("test-token");
    assert!(time::timeout(
        Duration::from_secs(1),
        c.describe_imaging(proto::DescribeImagingRequest {})
    )
    .await
    .is_err());
    let mut camera = rig.camera();
    assert!(time::timeout(
        Duration::from_secs(1),
        camera.describe(proto::DescribeRequest {})
    )
    .await
    .is_err());
}

#[tokio::test(start_paused = true)]
async fn a_second_expose_is_refused_and_an_abort_ends_the_first() {
    let rig = Rig::new().await;
    let mut c = rig.imaging("test-token");
    let mut first = c
        .expose(request(10_000.0, "frame"))
        .await
        .unwrap()
        .into_inner();
    time::sleep(Duration::from_secs(1)).await;
    assert!(
        c.imaging_status(proto::ImagingStatusRequest {})
            .await
            .unwrap()
            .into_inner()
            .exposing
    );
    let mut second = c
        .expose(request(100.0, "frame"))
        .await
        .unwrap()
        .into_inner();
    let status = refused(&mut second).await;
    assert_eq!(status.code(), Code::ResourceExhausted);
    assert_eq!(status.message(), "an exposure is already in progress");
    assert!(second.message().await.unwrap().is_none());
    c.abort_exposure(proto::AbortExposureRequest {})
        .await
        .unwrap();
    let status = refused(&mut first).await;
    assert_eq!(status.code(), Code::Aborted);
    assert_eq!(status.message(), "the exposure was aborted");
    assert!(first.message().await.unwrap().is_none());
    assert!(
        !c.imaging_status(proto::ImagingStatusRequest {})
            .await
            .unwrap()
            .into_inner()
            .exposing
    );
    // Abort with nothing exposing answers OK.
    c.abort_exposure(proto::AbortExposureRequest {})
        .await
        .unwrap();
}

#[tokio::test(start_paused = true)]
async fn a_refused_exposure_or_purpose_is_invalid_argument_in_the_stream() {
    let rig = Rig::new().await;
    let mut c = rig.imaging("test-token");
    for (request, text) in [
        (
            request(0.5, "frame"),
            "exposure_ms 0.5 is outside the camera's range 1–60000 ms",
        ),
        (
            request(500.0, "Dark"),
            "purpose must be 1 to 16 lower-case letters",
        ),
    ] {
        let mut stream = c.expose(request).await.unwrap().into_inner();
        let status = refused(&mut stream).await;
        assert_eq!(status.code(), Code::InvalidArgument);
        assert_eq!(status.message(), text);
        assert!(stream.message().await.unwrap().is_none());
    }
}

/// A camera of the test's own whose exposure always fails.
struct Fallen;
fn fallen_info() -> ImagingInfo {
    ImagingInfo {
        model: "fallen camera".into(),
        serial: None,
        width: 64,
        height: 48,
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
    }
}
impl ImagingCamera for Fallen {
    fn describe(&self) -> ImagingFuture<'_, ImagingInfo> {
        Box::pin(async { Ok(fallen_info()) })
    }
    fn status(&self) -> ImagingFuture<'_, ImagingStatus> {
        Box::pin(async {
            Ok(ImagingStatus {
                temp_c: Some(20.0),
                cooler_power: Some(0.0),
                set_temp_c: None,
                exposing: false,
            })
        })
    }
    fn apply<'a>(&'a self, _: &'a ImagingSettings) -> ImagingFuture<'a, ImagingSettings> {
        Box::pin(async { Ok(ImagingSettings::default()) })
    }
    fn expose<'a>(&'a self, _: &'a Exposure) -> ImagingFuture<'a, ImagingFrame> {
        Box::pin(async { Err(ImagingError::Failed("the camera fell over".into())) })
    }
    fn abort(&self) -> ImagingFuture<'_, ()> {
        Box::pin(async { Ok(()) })
    }
}

#[tokio::test(start_paused = true)]
async fn a_camera_failure_is_internal_with_its_text() {
    let rig = Rig::with_camera(Fallen).await;
    let mut c = rig.imaging("test-token");
    let mut stream = c
        .expose(request(500.0, "frame"))
        .await
        .unwrap()
        .into_inner();
    let status = refused(&mut stream).await;
    assert_eq!(status.code(), Code::Internal);
    assert_eq!(status.message(), "the camera fell over");
    assert!(stream.message().await.unwrap().is_none());
}

#[tokio::test(start_paused = true)]
async fn saving_without_a_folder_reports_it_and_answers_the_whole_frame() {
    let rig = Rig::new().await;
    let mut c = rig.imaging("test-token");
    let mut stream = c
        .expose(proto::ExposeRequest {
            exposure_ms: 500.0,
            purpose: "frame".into(),
            save: true,
            ..Default::default()
        })
        .await
        .unwrap()
        .into_inner();
    let (header, chunks) = header_and_chunks(&mut stream).await;
    assert_eq!(header.node_file, "");
    assert_eq!(header.node_save_error, "this scope node has no save_dir");
    assert_eq!(
        chunks.concat(),
        sample_rule(
            Roi {
                x: 0,
                y: 0,
                width: 64,
                height: 48
            },
            1,
            10.0,
            64,
            48
        )
    );
}

#[tokio::test(start_paused = true)]
async fn a_dropped_expose_runs_to_its_end_and_the_frame_still_counts() {
    let rig = Rig::new().await;
    let mut c = rig.imaging("test-token");
    let first = c
        .expose(request(2_000.0, "frame"))
        .await
        .unwrap()
        .into_inner();
    time::sleep(Duration::from_secs(1)).await;
    drop(first);
    // Status read 0.5 s after the drop still says the exposure runs (owner
    // decision 11).
    time::sleep(Duration::from_millis(500)).await;
    assert!(
        c.imaging_status(proto::ImagingStatusRequest {})
            .await
            .unwrap()
            .into_inner()
            .exposing
    );
    time::sleep(Duration::from_secs(2)).await;
    let mut second = c
        .expose(request(100.0, "frame"))
        .await
        .unwrap()
        .into_inner();
    let (header, _) = header_and_chunks(&mut second).await;
    assert_eq!(header.id, 2);
}
