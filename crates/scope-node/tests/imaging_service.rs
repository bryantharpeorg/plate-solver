//! US3's scenarios, one test each. Oracles: the stand-in camera's sample rule
//! (plan.md § The stand-in camera), computed here rather than read from the
//! camera, and the exact texts of plan.md § The node's service.
use scope_link::{memory, proto, BearerToken};
use scope_node::{
    clock::NodeClock,
    imaging::{
        synthetic::SyntheticImagingCamera, BayerPattern, Exposure, ImagingCamera, ImagingError,
        ImagingFrame, ImagingFuture, ImagingInfo, ImagingSettings, ImagingStatus, Range, Roi,
    },
    service::{NodeControls, NodeService},
    synthetic::SyntheticCamera,
    Settings,
};
use std::{sync::Arc, time::Duration};
use tokio::{task::JoinHandle, time};
use tonic::{service::interceptor::InterceptedService, transport::Channel, Code};

type ImagingClient = proto::scope_node_imaging_client::ScopeNodeImagingClient<
    InterceptedService<Channel, BearerToken>,
>;

// The test clock of spec.md § User Scenarios: 1000.0 plus the tokio time
// elapsed since it was built, so paused time advances it exactly.
struct Clock(tokio::time::Instant);
impl NodeClock for Clock {
    fn now(&self) -> f64 {
        1000.0 + self.0.elapsed().as_secs_f64()
    }
}

struct Rig {
    channel: Channel,
    clock: Arc<dyn NodeClock>,
    controls: NodeControls,
    server: JoinHandle<()>,
}
impl Drop for Rig {
    fn drop(&mut self) {
        self.server.abort();
    }
}
impl Rig {
    fn node(clock: Arc<dyn NodeClock>) -> NodeService {
        NodeService::new(
            SyntheticCamera::new(clock.clone()),
            clock,
            "test-token",
            Settings::default(),
        )
        .unwrap()
    }
    async fn serve(service: NodeService, clock: Arc<dyn NodeClock>) -> Self {
        let controls = service.controls();
        let (connect, io) = memory::channel_pair();
        let routes = service.routes();
        let server = tokio::spawn(async move {
            memory::serve_routes_on(io, routes).await.unwrap();
        });
        Self {
            channel: connect.await.unwrap(),
            clock,
            controls,
            server,
        }
    }
    /// The node rig: the stand-in finder camera with the stand-in imaging camera.
    async fn new() -> Self {
        let clock: Arc<dyn NodeClock> = Arc::new(Clock(tokio::time::Instant::now()));
        let imaging = SyntheticImagingCamera::new(clock.clone());
        Self::serve(Self::node(clock.clone()).with_imaging(imaging), clock).await
    }
    /// The node rig built without `with_imaging`.
    async fn without_imaging() -> Self {
        let clock: Arc<dyn NodeClock> = Arc::new(Clock(tokio::time::Instant::now()));
        Self::serve(Self::node(clock.clone()), clock).await
    }
    /// The node rig with an imaging camera that fails every exposure.
    async fn failing() -> Self {
        let clock: Arc<dyn NodeClock> = Arc::new(Clock(tokio::time::Instant::now()));
        Self::serve(Self::node(clock.clone()).with_imaging(FallenCamera), clock).await
    }
    fn imaging(&self, token: &str) -> ImagingClient {
        proto::scope_node_imaging_client::ScopeNodeImagingClient::with_interceptor(
            self.channel.clone(),
            BearerToken::new(token).unwrap(),
        )
    }
    async fn header(
        stream: &mut tonic::Streaming<proto::ExposeEvent>,
    ) -> proto::ImagingFrameHeader {
        match stream.message().await.unwrap().unwrap().event {
            Some(proto::expose_event::Event::Header(header)) => header,
            other => panic!("the first item is not a header: {other:?}"),
        }
    }
    async fn chunks(stream: &mut tonic::Streaming<proto::ExposeEvent>) -> Vec<Vec<u8>> {
        let mut chunks = Vec::new();
        while let Some(event) = stream.message().await.unwrap() {
            match event.event {
                Some(proto::expose_event::Event::Chunk(bytes)) => chunks.push(bytes),
                other => panic!("a later item is not a chunk: {other:?}"),
            }
        }
        chunks
    }
    async fn expose(
        client: &mut ImagingClient,
        exposure_ms: f64,
        purpose: &str,
    ) -> tonic::Streaming<proto::ExposeEvent> {
        client
            .expose(proto::ExposeRequest {
                exposure_ms,
                purpose: purpose.into(),
                ..Default::default()
            })
            .await
            .unwrap()
            .into_inner()
    }
}

/// The sample rule, computed here from plan.md § The stand-in camera: with
/// binning `b`, region origin `(x0, y0)` and offset `o` in force, the sample at
/// column `c` and row `r` is `256 + 16·round(o) + ((x0/b + c + 3·(y0/b + r))
/// mod 251)`, little-endian, row by row from the top-left.
fn samples(roi: Roi, binning: u32, offset: f64, width: u32, height: u32) -> Vec<u8> {
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

/// A camera of the test's own whose every exposure fails, for US3-S6.
struct FallenCamera;
impl ImagingCamera for FallenCamera {
    fn describe(&self) -> ImagingFuture<'_, ImagingInfo> {
        Box::pin(async {
            Ok(ImagingInfo {
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
            })
        })
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
    fn apply<'a>(&'a self, settings: &'a ImagingSettings) -> ImagingFuture<'a, ImagingSettings> {
        Box::pin(async move { Ok(settings.clone()) })
    }
    fn expose<'a>(&'a self, _: &'a Exposure) -> ImagingFuture<'a, ImagingFrame> {
        Box::pin(async { Err(ImagingError::Failed("the camera fell over".into())) })
    }
    fn abort(&self) -> ImagingFuture<'_, ()> {
        Box::pin(async { Ok(()) })
    }
}

#[tokio::test(start_paused = true)]
async fn describe_status_and_configure_answer_the_stand_in_camera() {
    let rig = Rig::new().await;
    let mut client = rig.imaging("test-token");
    let info = client
        .describe_imaging(proto::DescribeImagingRequest {})
        .await
        .unwrap()
        .into_inner();
    assert_eq!(info.model, "synthetic imaging camera");
    assert_eq!(info.serial, None);
    assert_eq!((info.width, info.height), (64, 48));
    assert_eq!(info.pixel_um, 4.78);
    assert_eq!(info.bits, 16);
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
    let status = client
        .imaging_status(proto::ImagingStatusRequest {})
        .await
        .unwrap()
        .into_inner();
    assert_eq!(
        (status.temp_c, status.cooler_power, status.set_temp_c),
        (Some(20.0), Some(0.0), None)
    );
    assert!(!status.exposing);
    // An empty ConfigureImaging answers the settings in force.
    let empty = client
        .configure_imaging(proto::ImagingSettings::default())
        .await
        .unwrap()
        .into_inner();
    assert_eq!(
        (empty.gain, empty.offset, empty.binning),
        (Some(100.0), Some(10.0), Some(1))
    );
    assert_eq!(
        empty.roi,
        Some(proto::ImagingRoi {
            x: 0,
            y: 0,
            width: 64,
            height: 48,
        })
    );
    assert_eq!(empty.cooler_on, Some(false));
    // A gain edit changes only the gain.
    let gain = client
        .configure_imaging(proto::ImagingSettings {
            gain: Some(200.0),
            ..Default::default()
        })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(gain.gain, Some(200.0));
    assert_eq!((gain.offset, gain.binning), (Some(10.0), Some(1)));
    assert_eq!(
        gain.roi,
        Some(proto::ImagingRoi {
            x: 0,
            y: 0,
            width: 64,
            height: 48,
        })
    );
    assert_eq!(gain.cooler_on, Some(false));
    // A gain outside the range is refused with the port's text.
    let error = client
        .configure_imaging(proto::ImagingSettings {
            gain: Some(301.0),
            ..Default::default()
        })
        .await
        .unwrap_err();
    assert_eq!(error.code(), Code::InvalidArgument);
    assert_eq!(
        error.message(),
        "gain 301 is outside the camera's range 0–300"
    );
}

#[tokio::test(start_paused = true)]
async fn expose_answers_a_header_then_chunks_and_counts_frames() {
    let rig = Rig::new().await;
    let mut client = rig.imaging("test-token");
    let at_call = rig.clock.now();
    let mut first = Rig::expose(&mut client, 500.0, "frame").await;
    let header = Rig::header(&mut first).await;
    assert_eq!(header.id, 1);
    assert_eq!((header.width, header.height, header.bits), (64, 48, 16));
    assert_eq!(header.bayer, "RGGB");
    assert_eq!(header.exposure_ms, 500.0);
    assert_eq!(header.gain, Some(100.0));
    assert_eq!(header.offset, Some(10.0));
    assert_eq!(header.binning, 1);
    assert_eq!(header.temp_c, Some(20.0));
    assert_eq!(header.purpose, "frame");
    assert_eq!(header.byte_count, 6144);
    assert_eq!(
        header.roi,
        Some(proto::ImagingRoi {
            x: 0,
            y: 0,
            width: 64,
            height: 48,
        })
    );
    assert_eq!(header.start_s, at_call);
    assert_eq!(header.end_s, at_call + 0.5);
    let chunks = Rig::chunks(&mut first).await;
    assert!(chunks.iter().all(|chunk| chunk.len() <= 262_144));
    let bytes: Vec<u8> = chunks.concat();
    assert_eq!(bytes.len(), 6144);
    assert_eq!(
        bytes,
        samples(
            Roi {
                x: 0,
                y: 0,
                width: 64,
                height: 48,
            },
            1,
            10.0,
            64,
            48,
        )
    );
    // The node counts its frames from 1: the next header carries id 2.
    let mut second = Rig::expose(&mut client, 500.0, "frame").await;
    let header = Rig::header(&mut second).await;
    assert_eq!(header.id, 2);
    Rig::chunks(&mut second).await;
    // Binning 2 and a region configured first: the next header carries them
    // (oracle: the stand-in's frame, US1-S4).
    client
        .configure_imaging(proto::ImagingSettings {
            binning: Some(2),
            roi: Some(proto::ImagingRoi {
                x: 8,
                y: 4,
                width: 16,
                height: 8,
            }),
            ..Default::default()
        })
        .await
        .unwrap();
    let mut third = Rig::expose(&mut client, 100.0, "frame").await;
    let header = Rig::header(&mut third).await;
    assert_eq!(
        header.roi,
        Some(proto::ImagingRoi {
            x: 8,
            y: 4,
            width: 16,
            height: 8,
        })
    );
    assert_eq!(header.binning, 2);
    assert_eq!((header.width, header.height), (8, 4));
    let chunks = Rig::chunks(&mut third).await;
    let bytes: Vec<u8> = chunks.concat();
    assert_eq!(bytes.len(), 8 * 4 * 2);
    assert_eq!(
        bytes,
        samples(
            Roi {
                x: 8,
                y: 4,
                width: 16,
                height: 8,
            },
            2,
            10.0,
            8,
            4,
        )
    );
}

#[tokio::test(start_paused = true)]
async fn every_call_requires_the_token() {
    let rig = Rig::new().await;
    let mut client = rig.imaging("wrong-token");
    for error in [
        client
            .describe_imaging(proto::DescribeImagingRequest {})
            .await
            .unwrap_err(),
        client
            .imaging_status(proto::ImagingStatusRequest {})
            .await
            .unwrap_err(),
        client
            .configure_imaging(proto::ImagingSettings::default())
            .await
            .unwrap_err(),
        client
            .expose(proto::ExposeRequest::default())
            .await
            .unwrap_err(),
        client
            .abort_exposure(proto::AbortExposureRequest {})
            .await
            .unwrap_err(),
    ] {
        assert_eq!(error.code(), Code::Unauthenticated);
    }
}

#[tokio::test(start_paused = true)]
async fn without_a_camera_without_the_service_and_while_unresponsive() {
    // A node built without `with_imaging` answers every call with the same
    // precondition, `Expose` as the first item of its stream.
    let rig = Rig::without_imaging().await;
    let mut client = rig.imaging("test-token");
    for error in [
        client
            .describe_imaging(proto::DescribeImagingRequest {})
            .await
            .unwrap_err(),
        client
            .imaging_status(proto::ImagingStatusRequest {})
            .await
            .unwrap_err(),
        client
            .configure_imaging(proto::ImagingSettings::default())
            .await
            .unwrap_err(),
        client
            .abort_exposure(proto::AbortExposureRequest {})
            .await
            .unwrap_err(),
    ] {
        assert_eq!(error.code(), Code::FailedPrecondition);
        assert_eq!(error.message(), "no imaging camera");
    }
    let mut stream = Rig::expose(&mut client, 100.0, "frame").await;
    let error = stream.message().await.unwrap_err();
    assert_eq!(error.code(), Code::FailedPrecondition);
    assert_eq!(error.message(), "no imaging camera");
    assert!(stream.message().await.unwrap().is_none());

    // A node built before this spec serves only its ScopeNode service, so the
    // new service answers `Unimplemented`.
    let clock: Arc<dyn NodeClock> = Arc::new(Clock(tokio::time::Instant::now()));
    let service = Rig::node(clock);
    let (connect, io) = memory::channel_pair();
    let routes = tonic::service::Routes::new(service.into_server());
    let server = tokio::spawn(async move {
        memory::serve_routes_on(io, routes).await.unwrap();
    });
    let mut old = proto::scope_node_imaging_client::ScopeNodeImagingClient::with_interceptor(
        connect.await.unwrap(),
        BearerToken::new("test-token").unwrap(),
    );
    let error = old
        .describe_imaging(proto::DescribeImagingRequest {})
        .await
        .unwrap_err();
    assert_eq!(error.code(), Code::Unimplemented);
    server.abort();

    // A node made unresponsive stays silent here too, as its Describe does.
    let rig = Rig::new().await;
    rig.controls.set_unresponsive(true);
    let mut client = rig.imaging("test-token");
    assert!(time::timeout(
        Duration::from_secs(1),
        client.describe_imaging(proto::DescribeImagingRequest {})
    )
    .await
    .is_err());
}

#[tokio::test(start_paused = true)]
async fn a_second_expose_is_refused_and_abort_ends_the_first() {
    let rig = Rig::new().await;
    let mut client = rig.imaging("test-token");
    let mut first = Rig::expose(&mut client, 10_000.0, "frame").await;
    time::sleep(Duration::from_secs(1)).await;
    let status = client
        .imaging_status(proto::ImagingStatusRequest {})
        .await
        .unwrap()
        .into_inner();
    assert!(status.exposing);
    let mut second = Rig::expose(&mut client, 100.0, "frame").await;
    let error = second.message().await.unwrap_err();
    assert_eq!(error.code(), Code::ResourceExhausted);
    assert_eq!(error.message(), "an exposure is already in progress");
    assert!(second.message().await.unwrap().is_none());
    client
        .abort_exposure(proto::AbortExposureRequest {})
        .await
        .unwrap();
    let error = first.message().await.unwrap_err();
    assert_eq!(error.code(), Code::Aborted);
    assert_eq!(error.message(), "the exposure was aborted");
    assert!(first.message().await.unwrap().is_none());
    let status = client
        .imaging_status(proto::ImagingStatusRequest {})
        .await
        .unwrap()
        .into_inner();
    assert!(!status.exposing);
    // Abort with nothing exposing answers OK.
    client
        .abort_exposure(proto::AbortExposureRequest {})
        .await
        .unwrap();
}

#[tokio::test(start_paused = true)]
async fn refused_exposures_camera_failures_and_save_without_a_folder() {
    let rig = Rig::new().await;
    let mut client = rig.imaging("test-token");
    // An exposure outside the range is the stream's only item.
    let mut stream = Rig::expose(&mut client, 0.5, "frame").await;
    let error = stream.message().await.unwrap_err();
    assert_eq!(error.code(), Code::InvalidArgument);
    assert_eq!(
        error.message(),
        "exposure_ms 0.5 is outside the camera's range 1–60000 ms"
    );
    assert!(stream.message().await.unwrap().is_none());
    // A purpose that is not 1 to 16 lower-case letters, in the same way.
    let mut stream = Rig::expose(&mut client, 500.0, "Dark").await;
    let error = stream.message().await.unwrap_err();
    assert_eq!(error.code(), Code::InvalidArgument);
    assert_eq!(
        error.message(),
        "purpose must be 1 to 16 lower-case letters"
    );
    assert!(stream.message().await.unwrap().is_none());
    // A camera failure reaches the caller as `Internal` with its text.
    let failed = Rig::failing().await;
    let mut falling = failed.imaging("test-token");
    let mut stream = Rig::expose(&mut falling, 500.0, "frame").await;
    let error = stream.message().await.unwrap_err();
    assert_eq!(error.code(), Code::Internal);
    assert_eq!(error.message(), "the camera fell over");
    assert!(stream.message().await.unwrap().is_none());
    // `save` on a node with no save folder: the header says why, and the
    // whole frame still follows.
    let mut stream = client
        .expose(proto::ExposeRequest {
            exposure_ms: 500.0,
            purpose: "frame".into(),
            save: true,
            ..Default::default()
        })
        .await
        .unwrap()
        .into_inner();
    let header = Rig::header(&mut stream).await;
    assert_eq!(header.node_file, "");
    assert_eq!(header.node_save_error, "this scope node has no save_dir");
    let chunks = Rig::chunks(&mut stream).await;
    assert_eq!(chunks.concat().len(), 6144);
}

#[tokio::test(start_paused = true)]
async fn a_dropped_stream_leaves_the_exposure_to_finish() {
    let rig = Rig::new().await;
    let mut client = rig.imaging("test-token");
    let stream = Rig::expose(&mut client, 2_000.0, "frame").await;
    time::sleep(Duration::from_secs(1)).await;
    drop(stream);
    // 0.5 s after the drop the exposure still runs (owner decision 11).
    time::sleep(Duration::from_millis(500)).await;
    let status = client
        .imaging_status(proto::ImagingStatusRequest {})
        .await
        .unwrap()
        .into_inner();
    assert!(status.exposing);
    // The exposure ran to its end with nobody reading; the next frame counts.
    time::sleep(Duration::from_millis(1_500)).await;
    let mut next = Rig::expose(&mut client, 100.0, "frame").await;
    let header = Rig::header(&mut next).await;
    assert_eq!(header.id, 2);
    Rig::chunks(&mut next).await;
}
