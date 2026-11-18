//! The scope link's imaging service (030): five calls under the pairing token,
//! with an exposure as a header and then chunks of at most 256 KiB, and the
//! node's own copy of a frame written before the header is sent (US7).
use crate::imaging::{
    save, BayerPattern, Exposure, ImagingCamera, ImagingError, ImagingFrame, ImagingInfo,
    ImagingSettings, Range, Roi,
};
use crate::service::NodeService;
use scope_link::proto::{
    self, expose_event, scope_node_imaging_server::ScopeNodeImaging, AbortExposureReply,
    AbortExposureRequest, DescribeImagingRequest, ExposeEvent, ExposeRequest, ImagingFrameHeader,
    ImagingRange, ImagingRoi, ImagingStatusReply, ImagingStatusRequest,
};
use std::{
    path::{Path, PathBuf},
    pin::Pin,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, OnceLock,
    },
    time::Duration,
};
use tokio::{sync::mpsc, time::Instant};
use tokio_stream::{wrappers::ReceiverStream, Stream};
use tonic::{Request, Response, Status};

/// The most bytes of samples in one chunk.
pub const CHUNK_BYTES: usize = 262_144;

/// The node's one imaging camera and its frame count. Its fields are private
/// to this module; `crate::service` builds it with `new` and keeps it in the
/// runtime, and only this module's five calls reach the camera.
pub(crate) struct ImagingHost {
    camera: Arc<dyn ImagingCamera>,
    next_id: AtomicU64,
    /// US7: where the node keeps its own copy of a frame asked for with
    /// `save`.
    save_dir: OnceLock<PathBuf>,
}
impl ImagingHost {
    pub(crate) fn new(camera: Arc<dyn ImagingCamera>) -> Self {
        Self {
            camera,
            next_id: AtomicU64::new(1),
            save_dir: OnceLock::new(),
        }
    }
    /// The first call sets the folder; a later one changes nothing.
    pub(crate) fn set_save_dir(&self, dir: PathBuf) {
        let _ = self.save_dir.set(dir);
    }
    /// The save folder, when one was set.
    fn save_dir(&self) -> Option<&Path> {
        self.save_dir.get().map(PathBuf::as_path)
    }
    /// The next frame's id, taken whether or not anyone still listens.
    fn next_frame_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::SeqCst)
    }
}

/// The port's errors as the link's statuses, each with the error's text.
fn error_status(error: ImagingError) -> Status {
    match error {
        ImagingError::Invalid(text) => Status::invalid_argument(text),
        ImagingError::Busy => Status::resource_exhausted("an exposure is already in progress"),
        ImagingError::Aborted => Status::aborted("the exposure was aborted"),
        ImagingError::Failed(text) => Status::internal(text),
    }
}

fn range_to_wire(range: Range) -> ImagingRange {
    ImagingRange {
        min: range.min,
        max: range.max,
    }
}

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
            .map(BayerPattern::as_str)
            .unwrap_or_default()
            .to_string(),
        exposure_ms: Some(range_to_wire(info.exposure_ms)),
        gain: info.gain.map(range_to_wire),
        offset: info.offset.map(range_to_wire),
        binnings: info.binnings.clone(),
        cooler: info.cooler,
    }
}

fn roi_from_wire(roi: ImagingRoi) -> Roi {
    Roi {
        x: roi.x,
        y: roi.y,
        width: roi.width,
        height: roi.height,
    }
}

fn roi_to_wire(roi: Roi) -> ImagingRoi {
    ImagingRoi {
        x: roi.x,
        y: roi.y,
        width: roi.width,
        height: roi.height,
    }
}

fn settings_from_wire(settings: proto::ImagingSettings) -> ImagingSettings {
    ImagingSettings {
        gain: settings.gain,
        offset: settings.offset,
        binning: settings.binning,
        roi: settings.roi.map(roi_from_wire),
        cooler_on: settings.cooler_on,
        set_temp_c: settings.set_temp_c,
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

fn status_to_wire(status: crate::imaging::ImagingStatus) -> ImagingStatusReply {
    ImagingStatusReply {
        temp_c: status.temp_c,
        cooler_power: status.cooler_power,
        set_temp_c: status.set_temp_c,
        exposing: status.exposing,
    }
}

/// US7's copy on the node's disk, written before the header is sent
/// (FR-033). The file carries the model the camera reports, so it is asked
/// for here; the frame moves into the blocking write and comes back, so its
/// samples are never copied. A failure to write is reported in the header
/// and never stops the frame being sent (owner decision 9).
async fn save_frame(
    host: &ImagingHost,
    id: u64,
    frame: ImagingFrame,
    purpose: &str,
) -> (ImagingFrame, String, String) {
    let Some(dir) = host.save_dir() else {
        return (
            frame,
            String::new(),
            "this scope node has no save_dir".to_string(),
        );
    };
    let model = match host.camera.describe().await {
        Ok(info) => info.model,
        Err(error) => {
            return (
                frame,
                String::new(),
                format!("could not save the imaging frame: {error}"),
            )
        }
    };
    let dir = dir.to_path_buf();
    let purpose = purpose.to_string();
    let (frame, saved) = tokio::task::spawn_blocking(move || {
        let saved = save::write(&dir, id, &frame, &model, &purpose);
        (frame, saved)
    })
    .await
    .expect("the frame writer cannot be cancelled");
    match saved {
        Ok(name) => (frame, name, String::new()),
        Err(error) => (
            frame,
            String::new(),
            format!("could not save the imaging frame: {error}"),
        ),
    }
}

fn header(
    id: u64,
    frame: &ImagingFrame,
    purpose: &str,
    node_file: String,
    node_save_error: String,
) -> ImagingFrameHeader {
    ImagingFrameHeader {
        id,
        width: frame.width,
        height: frame.height,
        bits: frame.bits,
        bayer: frame
            .bayer
            .map(BayerPattern::as_str)
            .unwrap_or_default()
            .to_string(),
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
        _: Request<DescribeImagingRequest>,
    ) -> Result<Response<proto::ImagingInfo>, Status> {
        self.controls().ready(false).await;
        let host = self
            .imaging_host()
            .ok_or_else(|| Status::failed_precondition("no imaging camera"))?;
        let info = host.camera.describe().await.map_err(error_status)?;
        Ok(Response::new(info_to_wire(&info)))
    }

    async fn imaging_status(
        &self,
        _: Request<ImagingStatusRequest>,
    ) -> Result<Response<ImagingStatusReply>, Status> {
        self.controls().ready(false).await;
        let host = self
            .imaging_host()
            .ok_or_else(|| Status::failed_precondition("no imaging camera"))?;
        let status = host.camera.status().await.map_err(error_status)?;
        Ok(Response::new(status_to_wire(status)))
    }

    async fn configure_imaging(
        &self,
        request: Request<proto::ImagingSettings>,
    ) -> Result<Response<proto::ImagingSettings>, Status> {
        self.controls().ready(false).await;
        let host = self
            .imaging_host()
            .ok_or_else(|| Status::failed_precondition("no imaging camera"))?;
        let edit = settings_from_wire(request.into_inner());
        let settings = host.camera.apply(&edit).await.map_err(error_status)?;
        Ok(Response::new(settings_to_wire(&settings)))
    }

    type ExposeStream = Pin<Box<dyn Stream<Item = Result<ExposeEvent, Status>> + Send>>;
    async fn expose(
        &self,
        request: Request<ExposeRequest>,
    ) -> Result<Response<Self::ExposeStream>, Status> {
        // The interceptor's token check alone answers as the call's own status;
        // after it every failure is the stream's first and only item.
        let request = request.into_inner();
        let (sender, receiver) = mpsc::channel::<Result<ExposeEvent, Status>>(4);
        let controls = self.controls();
        let host = self.imaging_host().cloned();
        tokio::spawn(async move {
            controls.ready(false).await;
            let Some(host) = host else {
                let _ = sender
                    .send(Err(Status::failed_precondition("no imaging camera")))
                    .await;
                return;
            };
            let exposure = Exposure {
                exposure_ms: request.exposure_ms,
                purpose: request.purpose.clone(),
            };
            let frame = match host.camera.expose(&exposure).await {
                Ok(frame) => frame,
                Err(error) => {
                    let _ = sender.send(Err(error_status(error))).await;
                    return;
                }
            };
            // The id is taken whether or not anyone still listens.
            let id = host.next_frame_id();
            // US7: the copy is taken before the header, so a frame lost on
            // the link is not lost (FR-033).
            let (frame, node_file, node_save_error) = if request.save {
                save_frame(&host, id, frame, &request.purpose).await
            } else {
                (frame, String::new(), String::new())
            };
            let sent_header = sender
                .send(Ok(ExposeEvent {
                    event: Some(expose_event::Event::Header(header(
                        id,
                        &frame,
                        &request.purpose,
                        node_file,
                        node_save_error,
                    ))),
                }))
                .await;
            if sent_header.is_err() {
                return;
            }
            let mut first: Option<Instant> = None;
            let mut sent = 0;
            for chunk in frame.samples.chunks(CHUNK_BYTES) {
                // A chunk leaves no sooner than the bytes already sent divided
                // by the rate after the first chunk (US3-S8).
                if let Some(first) = first {
                    if request.max_bytes_per_s > 0 {
                        let rate = f64::from(request.max_bytes_per_s);
                        tokio::time::sleep_until(
                            first + Duration::from_secs_f64(sent as f64 / rate),
                        )
                        .await;
                    }
                }
                if sender
                    .send(Ok(ExposeEvent {
                        event: Some(expose_event::Event::Chunk(chunk.to_vec())),
                    }))
                    .await
                    .is_err()
                {
                    return;
                }
                if first.is_none() {
                    first = Some(Instant::now());
                }
                sent += chunk.len();
            }
        });
        Ok(Response::new(Box::pin(ReceiverStream::new(receiver))))
    }

    async fn abort_exposure(
        &self,
        _: Request<AbortExposureRequest>,
    ) -> Result<Response<AbortExposureReply>, Status> {
        self.controls().ready(false).await;
        let host = self
            .imaging_host()
            .ok_or_else(|| Status::failed_precondition("no imaging camera"))?;
        host.camera.abort().await.map_err(error_status)?;
        Ok(Response::new(AbortExposureReply {}))
    }
}
