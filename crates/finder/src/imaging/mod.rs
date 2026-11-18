//! The imaging camera as the console holds it (031): a second connection to the
//! paired node for `ScopeNodeImaging` alone, the state it keeps and the routes
//! that read and change it.

pub mod routes;

use std::{
    future::Future,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::http::StatusCode;
use scope_link::proto;
use serde::{Deserialize, Serialize};
use tokio::{sync::watch, task::JoinHandle, time};

use crate::{
    link::{ImagingClient, LinkManager},
    session::Clock,
};

/// How often the camera's status is read while `ready`.
pub const STATUS_PERIOD: Duration = Duration::from_secs(2);
/// How often the camera is asked for again in `no_service`, `no_camera` and
/// `error`; never sooner because a heartbeat was answered.
pub const RETRY_PERIOD: Duration = Duration::from_secs(5);
/// The console's exposure, in milliseconds, until set.
pub const DEFAULT_EXPOSURE_MS: f64 = 1000.0;
/// The limit on any call the imaging task makes itself.
pub const CALL_WAIT: Duration = Duration::from_secs(5);
/// The limit on the call a settings change waits for.
pub const SET_WAIT: Duration = Duration::from_secs(30);

/// The words of a call that ran out of `CALL_WAIT`.
const NO_ANSWER: &str = "the scope node did not answer within 5 s";

/// `GET /api/imaging`, and `imaging` in the session. Non-exhaustive: later specs
/// add parts.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct ImagingView {
    /// `off`, `no_link`, `no_service`, `no_camera`, `error` or `ready`.
    pub state: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camera: Option<CameraView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<StatusView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<SettingsView>,
}

impl ImagingView {
    /// The view of a console that takes no imaging frames.
    pub fn off() -> Self {
        Self {
            state: "off",
            error: None,
            camera: None,
            status: None,
            settings: None,
        }
    }
}

/// A camera control's inclusive range.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RangeView {
    pub min: f64,
    pub max: f64,
}

/// A region of the sensor, in unbinned pixels.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct RoiView {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl From<proto::ImagingRoi> for RoiView {
    fn from(roi: proto::ImagingRoi) -> Self {
        Self {
            x: roi.x,
            y: roi.y,
            width: roi.width,
            height: roi.height,
        }
    }
}

/// The camera's description, as the view serializes it: `null` for an absent
/// serial, Bayer pattern, gain or offset.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CameraView {
    pub model: String,
    pub serial: Option<String>,
    pub width: u32,
    pub height: u32,
    pub pixel_um: f64,
    pub bits: u32,
    pub bayer: Option<String>,
    pub exposure_ms: RangeView,
    pub gain: Option<RangeView>,
    pub offset: Option<RangeView>,
    pub binnings: Vec<u32>,
    pub cooler: bool,
}

impl From<proto::ImagingInfo> for CameraView {
    fn from(info: proto::ImagingInfo) -> Self {
        Self {
            model: info.model,
            serial: info.serial,
            width: info.width,
            height: info.height,
            pixel_um: info.pixel_um,
            bits: info.bits,
            bayer: (!info.bayer.is_empty()).then_some(info.bayer),
            exposure_ms: RangeView {
                min: info.exposure_ms.map(|r| r.min).unwrap_or_default(),
                max: info.exposure_ms.map(|r| r.max).unwrap_or_default(),
            },
            gain: info.gain.map(|r| RangeView {
                min: r.min,
                max: r.max,
            }),
            offset: info.offset.map(|r| RangeView {
                min: r.min,
                max: r.max,
            }),
            binnings: info.binnings,
            cooler: info.cooler,
        }
    }
}

/// The camera's sensor status, as the view serializes it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StatusView {
    pub temp_c: Option<f64>,
    pub cooler_power: Option<f64>,
    pub set_temp_c: Option<f64>,
    pub exposing: bool,
}

impl From<proto::ImagingStatusReply> for StatusView {
    fn from(reply: proto::ImagingStatusReply) -> Self {
        Self {
            temp_c: reply.temp_c,
            cooler_power: reply.cooler_power,
            set_temp_c: reply.set_temp_c,
            exposing: reply.exposing,
        }
    }
}

/// The settings in force, the camera's and the console's own three together.
/// Non-exhaustive: later specs add parts.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct SettingsView {
    pub exposure_ms: f64,
    pub gain: Option<f64>,
    pub offset: Option<f64>,
    pub binning: u32,
    pub roi: RoiView,
    pub cooler_on: bool,
    pub set_temp_c: Option<f64>,
    pub forward_kib_per_s: Option<u32>,
    pub save_at_node: bool,
}

/// The region of interest in a settings change, in unbinned sensor pixels: the
/// body's `{"x":…,"y":…,"width":…,"height":…}`. All four are required; another
/// key is refused.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoiEdit {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// The body of `PUT /api/imaging/settings`. Absent is unchanged; null is
/// refused but for `roi` (the whole sensor) and `forward_kib_per_s` (no limit).
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingsEdit {
    #[serde(default, deserialize_with = "present_value")]
    pub exposure_ms: Option<f64>,
    #[serde(default, deserialize_with = "present_value")]
    pub gain: Option<f64>,
    #[serde(default, deserialize_with = "present_value")]
    pub offset: Option<f64>,
    #[serde(default, deserialize_with = "present_value")]
    pub binning: Option<u32>,
    #[serde(default, deserialize_with = "present_nullable")]
    pub roi: Option<Option<RoiEdit>>,
    #[serde(default, deserialize_with = "present_value")]
    pub cooler_on: Option<bool>,
    #[serde(default, deserialize_with = "present_value")]
    pub set_temp_c: Option<f64>,
    #[serde(default, deserialize_with = "present_nullable")]
    pub forward_kib_per_s: Option<Option<f64>>,
    #[serde(default, deserialize_with = "present_value")]
    pub save_at_node: Option<bool>,
}

/// A value that must be present: a JSON null is an error, not "absent".
fn present_value<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

/// A value that may be cleared: present-and-null is `Some(None)`.
fn present_nullable<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

/// Why a request was not carried out: 400, 409 or 502, and the words.
#[derive(Debug, Clone, PartialEq)]
pub struct Refusal {
    pub status: StatusCode,
    pub message: String,
}

impl Refusal {
    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    fn conflict(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            message: message.into(),
        }
    }
}

/// What the imaging task holds: the state the view reads, the client it talks
/// over, and the console's own three settings, which last for a run and survive
/// a dropped link.
#[derive(Debug)]
struct Held {
    state: &'static str,
    error: Option<String>,
    camera: Option<CameraView>,
    status: Option<StatusView>,
    settings: Option<SettingsView>,
    client: Option<ImagingClient>,
    exposure_ms: f64,
    forward_kib_per_s: Option<u32>,
    save_at_node: bool,
}

impl Default for Held {
    fn default() -> Self {
        Self {
            state: "no_link",
            error: None,
            camera: None,
            status: None,
            settings: None,
            client: None,
            exposure_ms: DEFAULT_EXPOSURE_MS,
            forward_kib_per_s: None,
            save_at_node: false,
        }
    }
}

/// The paired node's imaging camera as the console holds it: the task that
/// follows the link and the state it keeps. Dropping the `Imaging` ends the task.
pub struct Imaging {
    state: Arc<Mutex<Held>>,
    task: JoinHandle<()>,
    /// Where imaging frames are saved, and the console's clock; US2's frame
    /// task reads both.
    #[allow(dead_code)]
    for_frames: (PathBuf, Arc<dyn Clock>),
}

impl Drop for Imaging {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl Imaging {
    /// Start the task that follows the link. Dropping the `Imaging` ends it.
    pub fn start(root: PathBuf, link: Arc<LinkManager>, clock: Arc<dyn Clock>) -> Arc<Self> {
        let state = Arc::new(Mutex::new(Held::default()));
        let task = tokio::spawn(follow(link, state.clone()));
        Arc::new(Self {
            state,
            task,
            for_frames: (root, clock),
        })
    }

    /// `GET /api/imaging`, and `imaging` in the session. Outside `ready` the view
    /// holds none of `camera`, `status` and `settings`.
    pub fn view(&self) -> ImagingView {
        let held = self.state.lock().expect("imaging state");
        let ready = held.state == "ready";
        ImagingView {
            state: held.state,
            error: if held.state == "error" {
                held.error.clone()
            } else {
                None
            },
            camera: ready.then(|| held.camera.clone()).flatten(),
            status: ready.then(|| held.status.clone()).flatten(),
            settings: ready.then(|| held.settings.clone()).flatten(),
        }
    }

    /// Apply a settings change (FR-003): the console keeps `exposure_ms`,
    /// `forward_kib_per_s` and `save_at_node`, sends the rest to the node, and
    /// answers the view of the settings then in force.
    pub async fn set(&self, edit: SettingsEdit) -> Result<ImagingView, Refusal> {
        let (mut client, camera, mut settings) = {
            let held = self.state.lock().expect("imaging state");
            match held.state {
                "ready" => (
                    held.client.clone().expect("a ready imaging client"),
                    held.camera.clone().expect("a described camera"),
                    held.settings.clone().expect("the settings in force"),
                ),
                "no_link" => {
                    return Err(Refusal::conflict("the scope node is not connected"));
                }
                "no_service" => {
                    return Err(Refusal::conflict("this scope node has no imaging service"));
                }
                "no_camera" => {
                    return Err(Refusal::conflict("this scope node has no imaging camera"));
                }
                _ => {
                    return Err(Refusal::conflict(format!(
                        "the imaging camera is not answering: {}",
                        held.error.as_deref().unwrap_or_default()
                    )));
                }
            }
        };
        if let Some(exposure_ms) = edit.exposure_ms {
            if !exposure_ms.is_finite()
                || exposure_ms < camera.exposure_ms.min
                || exposure_ms > camera.exposure_ms.max
            {
                return Err(Refusal::bad_request(format!(
                    "exposure_ms {exposure_ms} is outside the camera's range {}–{} ms",
                    camera.exposure_ms.min, camera.exposure_ms.max
                )));
            }
            settings.exposure_ms = exposure_ms;
        }
        if let Some(forward) = edit.forward_kib_per_s.flatten() {
            if forward.fract() != 0.0 || !(64.0..=1_048_576.0).contains(&forward) {
                return Err(Refusal::bad_request(
                    "forward_kib_per_s must be a whole number from 64 to 1048576, or null",
                ));
            }
            settings.forward_kib_per_s = Some(forward as u32);
        } else if edit.forward_kib_per_s.is_some() {
            settings.forward_kib_per_s = None;
        }
        if let Some(save_at_node) = edit.save_at_node {
            settings.save_at_node = save_at_node;
        }
        let mut wire = proto::ImagingSettings::default();
        let mut for_camera = false;
        if let Some(gain) = edit.gain {
            wire.gain = Some(gain);
            for_camera = true;
        }
        if let Some(offset) = edit.offset {
            wire.offset = Some(offset);
            for_camera = true;
        }
        if let Some(binning) = edit.binning {
            wire.binning = Some(binning);
            for_camera = true;
        }
        match edit.roi {
            Some(Some(roi)) => {
                wire.roi = Some(proto::ImagingRoi {
                    x: roi.x,
                    y: roi.y,
                    width: roi.width,
                    height: roi.height,
                });
                for_camera = true;
            }
            Some(None) => {
                wire.roi = Some(proto::ImagingRoi {
                    x: 0,
                    y: 0,
                    width: camera.width,
                    height: camera.height,
                });
                for_camera = true;
            }
            None => {}
        }
        if let Some(cooler_on) = edit.cooler_on {
            wire.cooler_on = Some(cooler_on);
            for_camera = true;
        }
        if let Some(set_temp_c) = edit.set_temp_c {
            wire.set_temp_c = Some(set_temp_c);
            for_camera = true;
        }
        if for_camera {
            let applied = match time::timeout(SET_WAIT, client.configure(wire)).await {
                Err(_) => {
                    return Err(Refusal {
                        status: StatusCode::BAD_GATEWAY,
                        message: "the scope node did not apply the settings: no answer within 30 s"
                            .to_string(),
                    });
                }
                Ok(Err(status)) => {
                    if status.code() == tonic::Code::InvalidArgument {
                        return Err(Refusal::bad_request(status.message()));
                    }
                    return Err(Refusal {
                        status: StatusCode::BAD_GATEWAY,
                        message: format!(
                            "the scope node did not apply the settings: {}",
                            status.message()
                        ),
                    });
                }
                Ok(Ok(applied)) => applied,
            };
            settings.gain = applied.gain.or(settings.gain);
            settings.offset = applied.offset.or(settings.offset);
            settings.binning = applied.binning.unwrap_or(settings.binning);
            settings.roi = applied.roi.map(RoiView::from).unwrap_or(settings.roi);
            settings.cooler_on = applied.cooler_on.unwrap_or(settings.cooler_on);
            settings.set_temp_c = applied.set_temp_c;
        }
        {
            let mut held = self.state.lock().expect("imaging state");
            held.exposure_ms = settings.exposure_ms;
            held.forward_kib_per_s = settings.forward_kib_per_s;
            held.save_at_node = settings.save_at_node;
            if held.state == "ready" {
                held.settings = Some(settings);
            }
        }
        Ok(self.view())
    }
}

/// The imaging task's phase: what it is waiting for besides the link.
enum Phase {
    /// No link: waiting for `connected` to turn true.
    NoLink,
    /// A retry state: ask again at the deadline.
    Retry(time::Instant),
    /// Ready: read the status at the deadline.
    Ready(time::Instant),
}

impl Phase {
    fn deadline(&self) -> Option<time::Instant> {
        match self {
            Self::NoLink => None,
            Self::Retry(at) | Self::Ready(at) => Some(*at),
        }
    }
}

/// Why a call of the task's failed, and the state it leaves.
enum Failure {
    /// `no_service` or `no_camera`: the describe status said which.
    Classified(&'static str),
    /// An error state, with the node's or the timeout's words.
    Error(String),
}

/// Classify a status the way a failed describe is classified.
fn classify_status(status: tonic::Status) -> Failure {
    match status.code() {
        tonic::Code::Unimplemented => Failure::Classified("no_service"),
        tonic::Code::FailedPrecondition => Failure::Classified("no_camera"),
        _ => Failure::Error(status.message().to_string()),
    }
}

/// One call of the task's, within `CALL_WAIT`.
async fn call<T, F>(future: F) -> Result<T, Failure>
where
    F: Future<Output = Result<T, tonic::Status>>,
{
    match time::timeout(CALL_WAIT, future).await {
        Err(_) => Err(Failure::Error(NO_ANSWER.to_string())),
        Ok(Err(status)) => Err(classify_status(status)),
        Ok(Ok(value)) => Ok(value),
    }
}

/// Write the state of a failed call: the client is dropped, and outside `error`
/// the view holds nothing of the camera.
fn classify(held: &Arc<Mutex<Held>>, failure: Failure) {
    let mut held = held.lock().expect("imaging state");
    held.client = None;
    held.camera = None;
    held.status = None;
    held.settings = None;
    match failure {
        Failure::Classified(state) => {
            held.state = state;
            held.error = None;
        }
        Failure::Error(message) => {
            held.state = "error";
            held.error = Some(message);
        }
    }
}

/// Clear everything but the console's own settings, for a dropped link.
fn drop_link(held: &Arc<Mutex<Held>>) {
    let mut held = held.lock().expect("imaging state");
    held.state = "no_link";
    held.error = None;
    held.camera = None;
    held.status = None;
    held.settings = None;
    held.client = None;
}

/// The settings in force, from the node's answer and the console's own three.
fn settings_view(
    applied: &proto::ImagingSettings,
    camera: &CameraView,
    exposure_ms: f64,
    forward_kib_per_s: Option<u32>,
    save_at_node: bool,
) -> SettingsView {
    SettingsView {
        exposure_ms: exposure_ms.clamp(camera.exposure_ms.min, camera.exposure_ms.max),
        gain: applied.gain,
        offset: applied.offset,
        binning: applied.binning.unwrap_or(1),
        roi: applied.roi.map(RoiView::from).unwrap_or(RoiView {
            x: 0,
            y: 0,
            width: camera.width,
            height: camera.height,
        }),
        cooler_on: applied.cooler_on.unwrap_or(false),
        set_temp_c: applied.set_temp_c,
        forward_kib_per_s,
        save_at_node,
    }
}

/// Connect and describe the camera, and on success read the settings in force
/// and the status, storing the ready state. Answers `Ok(())` or the failure.
async fn attempt(link: &LinkManager, held: &Arc<Mutex<Held>>) -> Result<(), Failure> {
    let mut client = call(link.connect_imaging()).await?;
    let info = call(client.describe()).await?;
    let camera = CameraView::from(info);
    let applied = call(client.configure(proto::ImagingSettings::default())).await?;
    let status = call(client.status()).await?;
    let mut held = held.lock().expect("imaging state");
    held.settings = Some(settings_view(
        &applied,
        &camera,
        held.exposure_ms,
        held.forward_kib_per_s,
        held.save_at_node,
    ));
    held.camera = Some(camera);
    held.status = Some(StatusView::from(status));
    held.state = "ready";
    held.error = None;
    held.client = Some(client);
    Ok(())
}

/// Race one attempt against the link dropping. Answers the next phase.
async fn establish(
    link: &Arc<LinkManager>,
    held: &Arc<Mutex<Held>>,
    status: &mut watch::Receiver<crate::link::LinkStatus>,
) -> Option<Phase> {
    let attempt = attempt(link, held);
    tokio::select! {
        outcome = status.wait_for(|s| !s.connected) => {
            outcome.ok()?;
            drop_link(held);
            Some(Phase::NoLink)
        }
        result = attempt => match result {
            Ok(()) => Some(Phase::Ready(time::Instant::now() + STATUS_PERIOD)),
            Err(failure) => {
                classify(held, failure);
                Some(Phase::Retry(time::Instant::now() + RETRY_PERIOD))
            }
        },
    }
}

/// Read the camera's status once, racing the link dropping, and answer the next
/// phase. Answers `None` when the link manager is gone.
async fn poll_status(
    held: &Arc<Mutex<Held>>,
    status: &mut watch::Receiver<crate::link::LinkStatus>,
) -> Option<Phase> {
    let poll = {
        let held = held.clone();
        async move {
            let mut client = held
                .lock()
                .expect("imaging state")
                .client
                .clone()
                .expect("a ready imaging client");
            match time::timeout(CALL_WAIT, client.status()).await {
                Err(_) => Err(Failure::Error(NO_ANSWER.to_string())),
                Ok(Err(status)) => Err(classify_status(status)),
                Ok(Ok(reply)) => Ok(StatusView::from(reply)),
            }
        }
    };
    tokio::select! {
        outcome = status.wait_for(|s| !s.connected) => {
            outcome.ok()?;
            drop_link(held);
            Some(Phase::NoLink)
        }
        result = poll => match result {
            Ok(view) => {
                held.lock().expect("imaging state").status = Some(view);
                Some(Phase::Ready(time::Instant::now() + STATUS_PERIOD))
            }
            Err(failure) => {
                classify(held, failure);
                Some(Phase::Retry(time::Instant::now() + RETRY_PERIOD))
            }
        },
    }
}

/// What woke the task: the connected flag changed, or the phase's deadline came.
#[derive(Clone, Copy)]
enum Wake {
    Link(bool),
    Tick,
}

/// The task that follows the link: it waits on the link's `connected` flag alone,
/// never on a change of the whole status, which every heartbeat changes (031
/// FR-006).
async fn follow(link: Arc<LinkManager>, held: Arc<Mutex<Held>>) {
    let mut status = link.status();
    let mut acted: Option<bool> = None;
    let mut phase = Phase::NoLink;
    loop {
        // Wake on the connected flag differing from the value last acted on, or
        // on the phase's deadline, whichever is first. With no deadline there is
        // nothing to race: waiting on a `None` future would spin.
        let wake = match phase.deadline() {
            None => match status.wait_for(|s| acted != Some(s.connected)).await {
                Ok(value) => Wake::Link(value.connected),
                Err(_) => return,
            },
            Some(deadline) => {
                let flip = status.wait_for(|s| acted != Some(s.connected));
                tokio::select! {
                    outcome = flip => match outcome {
                        Ok(value) => Wake::Link(value.connected),
                        Err(_) => return,
                    },
                    _ = time::sleep_until(deadline) => Wake::Tick,
                }
            }
        };
        let next = match (wake, &phase) {
            (Wake::Link(false), _) => {
                drop_link(&held);
                Some(Phase::NoLink)
            }
            (Wake::Link(true), _) => establish(&link, &held, &mut status).await,
            (Wake::Tick, Phase::Retry(_)) => establish(&link, &held, &mut status).await,
            (Wake::Tick, Phase::Ready(_)) => poll_status(&held, &mut status).await,
            (Wake::Tick, Phase::NoLink) => Some(Phase::NoLink),
        };
        phase = match next {
            Some(phase) => phase,
            None => return,
        };
        if let Wake::Link(connected) = wake {
            acted = Some(connected);
        }
    }
}
