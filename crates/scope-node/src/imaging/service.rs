//! The imaging service of the scope link (030): one camera, one exposure at a
//! time, its frames in chunks that never share the heartbeat's connection.
use crate::{
    imaging::{
        BayerPattern, Exposure, ImagingCamera, ImagingError, ImagingFrame, ImagingInfo,
        ImagingSettings, ImagingStatus, Range, Roi,
    },
    service::{NodeControls, NodeService},
};
use scope_link::proto::{self, scope_node_imaging_server::ScopeNodeImaging};
use std::{
    pin::Pin,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::sync::mpsc;
use tokio_stream::{wrappers::ReceiverStream, Stream};
use tonic::{Request, Response, Status};

/// The most bytes of samples in one chunk.
pub const CHUNK_BYTES: usize = 262_144;

/// The node's one imaging camera and its frame count. Its fields are private
/// to this module; `crates/scope-node/src/service.rs` builds it with `new`,
/// keeps it in the runtime, and only this module's five calls reach the
/// camera.
pub(crate) struct ImagingHost {
    camera: Arc<dyn ImagingCamera>,
    next_id: AtomicU64,
}

impl ImagingHost {
    pub(crate) fn new(camera: Arc<dyn ImagingCamera>) -> Self {
        Self {
            camera,
            next_id: AtomicU64::new(1),
        }
    }
}

/// The link's answer for each of the port's errors.
fn error_status(error: ImagingError) -> Status {
    match error {
        ImagingError::Invalid(text) => Status::invalid_argument(text),
        ImagingError::Busy => Status::resource_exhausted("an exposure is already in progress"),
        ImagingError::Aborted => Status::aborted("the exposure was aborted"),
        ImagingError::Failed(text) => Status::internal(text),
    }
}

/// The token check having passed, the status every call without a camera
/// answers.
fn no_camera() -> Status {
    Status::failed_precondition("no imaging camera")
}

fn range_to_wire(range: Range) -> proto::ImagingRange {
    proto::ImagingRange {
        min: range.min,
        max: range.max,
    }
}

fn roi_to_wire(roi: Roi) -> proto::ImagingRoi {
    proto::ImagingRoi {
        x: roi.x,
        y: roi.y,
        width: roi.width,
        height: roi.height,
    }
}

fn roi_from_wire(roi: proto::ImagingRoi) -> Roi {
    Roi {
        x: roi.x,
        y: roi.y,
        width: roi.width,
        height: roi.height,
    }
}

/// On the wire an empty `bayer` is mono, an absent `ImagingRange` no control
/// and an absent `roi` an unchanged region.
fn info_to_wire(info: &ImagingInfo) -> proto::ImagingInfo {
    proto::ImagingInfo {
        model: info.model.clone(),
        serial: info.serial.clone(),
        width: info.width,
        height: info.height,
        pixel_um: info.pixel_um,
        bits: info.bits,
        bayer: info
            .bayer
            .map(|pattern| pattern.as_str().to_string())
            .unwrap_or_default(),
        exposure_ms: Some(range_to_wire(info.exposure_ms)),
        gain: info.gain.map(range_to_wire),
        offset: info.offset.map(range_to_wire),
        binnings: info.binnings.clone(),
        cooler: info.cooler,
    }
}

fn settings_to_wire(settings: &ImagingSettings) -> proto::ImagingSettings {
    proto::ImagingSettings {
        gain: settings.gain,
        offset: settings.offset,
        binning: settings.binning,
        roi: settings.roi.map(roi_to_wire),
        cooler_on: settings.cooler_on,
        set_temp_c: settings.set_temp_c,
    }
}

fn settings_from_wire(edit: proto::ImagingSettings) -> ImagingSettings {
    ImagingSettings {
        gain: edit.gain,
        offset: edit.offset,
        binning: edit.binning,
        roi: edit.roi.map(roi_from_wire),
        cooler_on: edit.cooler_on,
        set_temp_c: edit.set_temp_c,
    }
}

fn status_to_wire(status: &ImagingStatus) -> proto::ImagingStatusReply {
    proto::ImagingStatusReply {
        temp_c: status.temp_c,
        cooler_power: status.cooler_power,
        set_temp_c: status.set_temp_c,
        exposing: status.exposing,
    }
}

/// One exposure's stream, in its own task: the token check aside, every
/// failure is the stream's first and only item, and the exposure runs to its
/// end whether or not anyone still reads (owner decision 11).
async fn send_exposure(
    controls: NodeControls,
    host: Option<Arc<ImagingHost>>,
    request: proto::ExposeRequest,
    sender: mpsc::Sender<Result<proto::ExposeEvent, Status>>,
) {
    controls.ready(false).await;
    let Some(host) = host else {
        let _ = sender.send(Err(no_camera())).await;
        return;
    };
    let frame = match host
        .camera
        .expose(&Exposure {
            exposure_ms: request.exposure_ms,
            purpose: request.purpose.clone(),
        })
        .await
    {
        Ok(frame) => frame,
        Err(error) => {
            let _ = sender.send(Err(error_status(error))).await;
            return;
        }
    };
    // The id counts the node's frames whether or not anyone still listens.
    let id = host.next_id.fetch_add(1, Ordering::SeqCst);
    // US7 writes the node's own copy here; without a folder there is only the
    // reason there is none.
    let (node_file, node_save_error) = if request.save {
        (String::new(), "this scope node has no save_dir".to_string())
    } else {
        (String::new(), String::new())
    };
    let header = frame_header(id, &frame, &request.purpose, node_file, node_save_error);
    if sender
        .send(Ok(proto::ExposeEvent {
            event: Some(proto::expose_event::Event::Header(header)),
        }))
        .await
        .is_err()
    {
        return;
    }
    let mut first: Option<tokio::time::Instant> = None;
    let mut sent = 0;
    for chunk in frame.samples.chunks(CHUNK_BYTES) {
        if request.max_bytes_per_s > 0 {
            // No chunk leaves before the bytes already sent divided by the
            // rate after the first chunk (US3-S8).
            let base = *first.get_or_insert(tokio::time::Instant::now());
            let after = sent as f64 / f64::from(request.max_bytes_per_s);
            if after > 0.0 {
                tokio::time::sleep_until(base + Duration::from_secs_f64(after)).await;
            }
        }
        if sender
            .send(Ok(proto::ExposeEvent {
                event: Some(proto::expose_event::Event::Chunk(chunk.to_vec())),
            }))
            .await
            .is_err()
        {
            return;
        }
        sent += chunk.len();
    }
}

fn frame_header(
    id: u64,
    frame: &ImagingFrame,
    purpose: &str,
    node_file: String,
    node_save_error: String,
) -> proto::ImagingFrameHeader {
    proto::ImagingFrameHeader {
        id,
        width: frame.width,
        height: frame.height,
        bits: frame.bits,
        bayer: frame
            .bayer
            .map(|pattern: BayerPattern| pattern.as_str().to_string())
            .unwrap_or_default(),
        start_s: frame.start_s,
        end_s: frame.end_s,
        exposure_ms: frame.exposure_ms,
        gain: frame.gain,
        offset: frame.offset,
        binning: frame.binning,
        temp_c: frame.temp_c,
        byte_count: frame.samples.len() as u64,
        purpose: purpose.to_string(),
        node_file,
        node_save_error,
        roi: Some(roi_to_wire(frame.roi)),
    }
}

#[tonic::async_trait]
impl ScopeNodeImaging for NodeService {
    async fn describe_imaging(
        &self,
        _: Request<proto::DescribeImagingRequest>,
    ) -> Result<Response<proto::ImagingInfo>, Status> {
        self.controls().ready(false).await;
        let host = self.imaging_host().ok_or_else(no_camera)?;
        let answer = host.camera.describe().await.map_err(error_status)?;
        Ok(Response::new(info_to_wire(&answer)))
    }

    async fn imaging_status(
        &self,
        _: Request<proto::ImagingStatusRequest>,
    ) -> Result<Response<proto::ImagingStatusReply>, Status> {
        self.controls().ready(false).await;
        let host = self.imaging_host().ok_or_else(no_camera)?;
        let answer = host.camera.status().await.map_err(error_status)?;
        Ok(Response::new(status_to_wire(&answer)))
    }

    async fn configure_imaging(
        &self,
        request: Request<proto::ImagingSettings>,
    ) -> Result<Response<proto::ImagingSettings>, Status> {
        self.controls().ready(false).await;
        let host = self.imaging_host().ok_or_else(no_camera)?;
        let edit = settings_from_wire(request.into_inner());
        let answer = host.camera.apply(&edit).await.map_err(error_status)?;
        Ok(Response::new(settings_to_wire(&answer)))
    }

    type ExposeStream = Pin<Box<dyn Stream<Item = Result<proto::ExposeEvent, Status>> + Send>>;

    async fn expose(
        &self,
        request: Request<proto::ExposeRequest>,
    ) -> Result<Response<Self::ExposeStream>, Status> {
        let request = request.into_inner();
        let (sender, receiver) = mpsc::channel(4);
        let controls = self.controls();
        let host = self.imaging_host().cloned();
        tokio::spawn(async move {
            send_exposure(controls, host, request, sender).await;
        });
        Ok(Response::new(Box::pin(ReceiverStream::new(receiver))))
    }

    async fn abort_exposure(
        &self,
        _: Request<proto::AbortExposureRequest>,
    ) -> Result<Response<proto::AbortExposureReply>, Status> {
        self.controls().ready(false).await;
        let host = self.imaging_host().ok_or_else(no_camera)?;
        host.camera.abort().await.map_err(error_status)?;
        Ok(Response::new(proto::AbortExposureReply {}))
    }
}
