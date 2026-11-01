//! One capture task shared by all authenticated scope-link clients.
use crate::{
    camera::check_range,
    capture::CaptureLoop,
    clock::NodeClock,
    detect::DetectSettings,
    exposure::Controller,
    mount::{queue::MountQueue, watchdog::Watchdog, Axis, Mount},
    preview::{self, PreviewSchedule},
    Camera, CameraInfo, RangeError, Settings,
};
use scope_link::{
    proto::{
        self,
        scope_node_mount_server::{ScopeNodeMount, ScopeNodeMountServer},
        scope_node_server::{ScopeNode, ScopeNodeServer},
        scope_node_stop_server::{ScopeNodeStop, ScopeNodeStopServer},
    },
    TokenInterceptor,
};
use std::{
    pin::Pin,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    sync::{broadcast, watch},
    task::JoinHandle,
};
use tokio_stream::{wrappers::BroadcastStream, Stream, StreamExt};
use tonic::{Request, Response, Status};

#[derive(Clone, Copy)]
struct ControlState {
    running: bool,
    unresponsive: bool,
    interval: Duration,
}

/// Simulation controls affect the real capture task and service, including existing streams.
#[derive(Clone)]
pub struct NodeControls {
    state: watch::Sender<ControlState>,
}
impl Default for NodeControls {
    fn default() -> Self {
        Self {
            state: watch::channel(ControlState {
                running: true,
                unresponsive: false,
                interval: Duration::from_millis(200),
            })
            .0,
        }
    }
}
impl NodeControls {
    pub fn stop_frames(&self) {
        self.state.send_modify(|s| s.running = false);
    }
    pub fn start_frames(&self) {
        self.state.send_modify(|s| s.running = true);
    }
    pub fn set_unresponsive(&self, value: bool) {
        self.state.send_modify(|s| s.unresponsive = value);
    }
    pub fn set_frame_interval(&self, interval: Duration) {
        assert!(!interval.is_zero(), "frame interval must be positive");
        self.state.send_modify(|s| s.interval = interval);
    }
    async fn ready(&self, frames: bool) {
        let mut changes = self.state.subscribe();
        let _ = changes
            .wait_for(|s| !s.unresponsive && (!frames || s.running))
            .await;
    }
}

struct Commands {
    pending: Option<Settings>,
    preview: bool,
}
struct Shared {
    info: CameraInfo,
    clock: Arc<dyn NodeClock>,
    commands: Mutex<Commands>,
    frames: broadcast::Sender<proto::StarFrame>,
    controls: NodeControls,
    mount: Option<MountQueue>,
    watchdog: Option<Watchdog>,
}
struct Runtime {
    shared: Arc<Shared>,
    capture: JoinHandle<()>,
    token: TokenInterceptor,
}
impl Drop for Runtime {
    fn drop(&mut self) {
        self.capture.abort();
    }
}

#[derive(Clone)]
pub struct NodeService {
    runtime: Arc<Runtime>,
}
impl NodeService {
    pub fn new<C: Camera + 'static>(
        camera: C,
        clock: Arc<dyn NodeClock>,
        token: impl AsRef<[u8]>,
        initial: Settings,
    ) -> Result<Self, RangeError> {
        Self::with_controls(camera, clock, token, initial, NodeControls::default())
    }
    pub fn with_controls<C: Camera + 'static>(
        camera: C,
        clock: Arc<dyn NodeClock>,
        token: impl AsRef<[u8]>,
        initial: Settings,
        controls: NodeControls,
    ) -> Result<Self, RangeError> {
        Self::with_mount(camera, clock, token, initial, controls, None)
    }
    /// Install an optional mount before starting capture and the heartbeat watchdog.
    pub fn with_mount<C: Camera + 'static>(
        camera: C,
        clock: Arc<dyn NodeClock>,
        token: impl AsRef<[u8]>,
        initial: Settings,
        controls: NodeControls,
        mount: Option<Arc<dyn Mount>>,
    ) -> Result<Self, RangeError> {
        let info = camera.info();
        let mount = mount.map(MountQueue::new);
        let watchdog = mount
            .as_ref()
            .map(|m| Watchdog::new(clock.clone(), m.clone()));
        let capture = CaptureLoop::new(camera, initial.clone())?.with_mount(mount.clone());
        let shared = Arc::new(Shared {
            info,
            clock,
            commands: Mutex::new(Commands {
                pending: None,
                preview: false,
            }),
            frames: broadcast::channel(16).0,
            controls,
            mount,
            watchdog,
        });
        let task = tokio::spawn(run(capture, initial, shared.clone()));
        Ok(Self {
            runtime: Arc::new(Runtime {
                shared,
                capture: task,
                token: TokenInterceptor::new(token),
            }),
        })
    }
    pub fn controls(&self) -> NodeControls {
        self.runtime.shared.controls.clone()
    }
    /// All three services share one channel. Only StopAll omits authentication.
    pub fn into_routes(self) -> tonic::service::Routes {
        let token = self.runtime.token.clone();
        tonic::service::Routes::new(self.clone().into_server())
            .add_service(ScopeNodeMountServer::with_interceptor(self.clone(), token))
            .add_service(ScopeNodeStopServer::new(self))
    }
    fn mount(&self) -> Result<&MountQueue, Status> {
        self.runtime
            .shared
            .mount
            .as_ref()
            .ok_or_else(|| Status::failed_precondition("no mount connected"))
    }
    /// Every method, including Describe and Heartbeat, uses the pairing interceptor.
    pub fn into_server(
        self,
    ) -> tonic::service::interceptor::InterceptedService<ScopeNodeServer<Self>, TokenInterceptor>
    {
        let token = self.runtime.token.clone();
        ScopeNodeServer::with_interceptor(self, token)
    }
}

async fn run<C: Camera>(mut capture: CaptureLoop<C>, mut settings: Settings, shared: Arc<Shared>) {
    let mut previews = PreviewSchedule::default();
    let mut actual: Option<(f64, f64)> = None;
    loop {
        shared.controls.ready(true).await;
        let start = tokio::time::Instant::now();
        let interval = shared.controls.state.borrow().interval;
        if let Some(next) = shared.commands.lock().unwrap().pending.take() {
            settings = next;
        }
        let mut camera_settings = settings.clone();
        if let Some((exposure, gain)) = actual {
            camera_settings.exposure_ms = Some(settings.exposure_ms.unwrap_or(exposure));
            camera_settings.gain = Some(settings.gain.unwrap_or(gain));
        }
        if let Err(error) = capture.apply(camera_settings) {
            eprintln!("cannot apply camera settings: {error}");
            tokio::time::sleep(Duration::from_secs(1)).await;
            continue;
        }
        let captured = match capture.next().await {
            Ok(frame) => frame,
            Err(_) => continue,
        };
        let frame = captured.frame;
        let requested = {
            let mut commands = shared.commands.lock().unwrap();
            std::mem::take(&mut commands.preview)
        };
        let preview_jpeg = if previews.due(frame.end_s, requested) {
            match preview::encode(&frame) {
                Ok(jpeg) => Some(jpeg),
                Err(error) => {
                    eprintln!("cannot encode preview: {error}");
                    None
                }
            }
        } else {
            None
        };
        let mode_limit = shared
            .info
            .active_mode
            .filter(|m| m.fps > 0.0)
            .map_or(f64::INFINITY, |m| 1000.0 / m.fps);
        let controller = Controller::new(
            &shared.info,
            (interval.as_secs_f64() * 1000.0).max(mode_limit),
        );
        actual = Some(controller.next(
            frame.exposure_ms,
            frame.gain,
            captured.centroids.len(),
            &settings,
        ));
        if !shared.controls.state.borrow().unresponsive {
            let _ = shared.frames.send(proto::StarFrame {
                id: captured.id,
                start_s: frame.start_s,
                end_s: frame.end_s,
                width: frame.width,
                height: frame.height,
                exposure_ms: frame.exposure_ms,
                gain: frame.gain,
                settings: Some(to_wire(&settings)),
                centroids: captured.centroids,
                preview_jpeg,
                axis_start: captured.axis_start,
                axis_end: captured.axis_end,
            });
        }
        tokio::time::sleep_until(start + interval).await;
    }
}
fn to_wire(settings: &Settings) -> proto::Settings {
    proto::Settings {
        exposure_ms: settings.exposure_ms,
        gain: settings.gain,
        detection: Some(proto::Detection {
            sigma: settings.detection.sigma,
            binning: settings.detection.binning as u32,
            normalize_rows: settings.detection.normalize_rows,
            detect_hot_pixels: settings.detection.detect_hot_pixels,
        }),
    }
}
fn from_wire(settings: proto::Settings) -> Result<Settings, Status> {
    let d = settings.detection.unwrap_or(proto::Detection {
        sigma: 8.0,
        binning: 2,
        normalize_rows: true,
        detect_hot_pixels: true,
    });
    if !d.sigma.is_finite() || !(1.0..=50.0).contains(&d.sigma) {
        return Err(Status::invalid_argument(
            "detection.sigma must be between 1 and 50",
        ));
    }
    if ![1, 2, 4, 8].contains(&d.binning) {
        return Err(Status::invalid_argument(
            "detection.binning must be 1, 2, 4 or 8",
        ));
    }
    Ok(Settings {
        exposure_ms: settings.exposure_ms,
        gain: settings.gain,
        detection: DetectSettings {
            sigma: d.sigma,
            binning: d.binning as usize,
            normalize_rows: d.normalize_rows,
            detect_hot_pixels: d.detect_hot_pixels,
            ..DetectSettings::cedar()
        },
    })
}

#[tonic::async_trait]
impl ScopeNode for NodeService {
    async fn describe(
        &self,
        _: Request<proto::DescribeRequest>,
    ) -> Result<Response<CameraInfo>, Status> {
        self.runtime.shared.controls.ready(false).await;
        Ok(Response::new(self.runtime.shared.info.clone()))
    }
    type FramesStream = Pin<Box<dyn Stream<Item = Result<proto::StarFrame, Status>> + Send>>;
    async fn frames(
        &self,
        _: Request<proto::FramesRequest>,
    ) -> Result<Response<Self::FramesStream>, Status> {
        self.runtime.shared.controls.ready(false).await;
        // A lagging subscriber skips lost exposures; their ids remain observable as gaps.
        let stream = BroadcastStream::new(self.runtime.shared.frames.subscribe())
            .filter_map(|r| r.ok().map(Ok));
        Ok(Response::new(Box::pin(stream)))
    }
    async fn heartbeat(
        &self,
        request: Request<proto::HeartbeatRequest>,
    ) -> Result<Response<proto::HeartbeatReply>, Status> {
        let t1 = self.runtime.shared.clock.now();
        self.runtime.shared.controls.ready(false).await;
        if let Some(watchdog) = &self.runtime.shared.watchdog {
            watchdog.heartbeat();
        }
        Ok(Response::new(proto::HeartbeatReply {
            t0: request.into_inner().t0,
            t1,
            t2: self.runtime.shared.clock.now(),
        }))
    }
    async fn configure(
        &self,
        request: Request<proto::Settings>,
    ) -> Result<Response<proto::Settings>, Status> {
        self.runtime.shared.controls.ready(false).await;
        let settings = from_wire(request.into_inner())?;
        check_range(&self.runtime.shared.info, &settings)
            .map_err(|e| Status::invalid_argument(e.to_string()))?;
        let response = to_wire(&settings);
        self.runtime.shared.commands.lock().unwrap().pending = Some(settings);
        Ok(Response::new(response))
    }
    async fn request_preview(
        &self,
        _: Request<proto::PreviewRequest>,
    ) -> Result<Response<proto::PreviewAck>, Status> {
        self.runtime.shared.controls.ready(false).await;
        self.runtime.shared.commands.lock().unwrap().preview = true;
        Ok(Response::new(proto::PreviewAck {}))
    }
}

fn axis(value: i32) -> Result<Axis, Status> {
    match proto::Axis::try_from(value) {
        Ok(proto::Axis::Azimuth) => Ok(Axis::Azimuth),
        Ok(proto::Axis::Altitude) => Ok(Axis::Altitude),
        _ => Err(Status::invalid_argument("axis must be azimuth or altitude")),
    }
}
#[tonic::async_trait]
impl ScopeNodeMount for NodeService {
    async fn describe_mount(
        &self,
        _: Request<proto::DescribeMountRequest>,
    ) -> Result<Response<proto::DescribeMountReply>, Status> {
        self.runtime.shared.controls.ready(false).await;
        let axes = self
            .mount()?
            .axes()
            .into_iter()
            .map(|info| proto::AxisInfo {
                counts_per_revolution: info.counts_per_revolution,
                top_rate_deg_per_s: info.top_rate_deg_per_s,
                can_goto: info.can_goto,
            })
            .collect();
        Ok(Response::new(proto::DescribeMountReply { axes }))
    }
    async fn axis_state(
        &self,
        request: Request<proto::AxisStateRequest>,
    ) -> Result<Response<proto::AxisStateReply>, Status> {
        let axis = axis(request.into_inner().axis)?;
        self.runtime.shared.controls.ready(false).await;
        let state = self.mount()?.state(axis).await;
        Ok(Response::new(proto::AxisStateReply {
            position: state.position,
            rate_deg_per_s: state.rate_deg_per_s,
            moving: state.moving,
        }))
    }
    async fn goto_axis(
        &self,
        request: Request<proto::GotoAxisRequest>,
    ) -> Result<Response<proto::GotoAxisReply>, Status> {
        let request = request.into_inner();
        let axis = axis(request.axis)?;
        let mount = self.mount()?;
        if !mount.axes()[axis.index()].can_goto {
            return Err(Status::failed_precondition("axis cannot go to a position"));
        }
        mount.goto(axis, request.position).await?;
        Ok(Response::new(proto::GotoAxisReply {}))
    }
    async fn rate_axis(
        &self,
        request: Request<proto::RateAxisRequest>,
    ) -> Result<Response<proto::RateAxisReply>, Status> {
        let request = request.into_inner();
        let axis = axis(request.axis)?;
        if !request.deg_per_s.is_finite() {
            return Err(Status::invalid_argument("axis rate must be finite"));
        }
        self.mount()?.rate(axis, request.deg_per_s).await?;
        Ok(Response::new(proto::RateAxisReply {}))
    }
}
#[tonic::async_trait]
impl ScopeNodeStop for NodeService {
    async fn stop_all(
        &self,
        _: Request<proto::StopAllRequest>,
    ) -> Result<Response<proto::StopAllReply>, Status> {
        if let Some(mount) = &self.runtime.shared.mount {
            mount.stop_all().await;
        }
        Ok(Response::new(proto::StopAllReply {}))
    }
}
