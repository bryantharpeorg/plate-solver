//! The imaging camera as the console holds it (031): a second connection to the
//! paired node for `ScopeNodeImaging` alone, the state it keeps and the routes
//! that read and change it.

pub mod astap;
pub mod calibration;
pub mod fits;
pub mod focus;
pub mod frames;
pub mod preview;
pub mod relation;
pub mod routes;
pub mod simulated;
pub mod solver;
pub mod solving;
pub mod stack;

use std::{
    future::Future,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock, Weak},
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

use calibration::service::Calibration;
use frames::FrameView;

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
    /// The latest imaging frame, from the first one asked for on (031 US2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame: Option<FrameView>,
    /// The latest solve of an imaging frame, from the first accepted request
    /// on (032 FR-010).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub solve: Option<solving::SolveView>,
    /// What the console keeps between the two cameras, from the first solved
    /// imaging frame or the first focal length on (032 US3, FR-017).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relation: Option<relation::RelationView>,
    /// The focus region, the focus record and the focus aid (033 US5, FR-029),
    /// only while the camera is `ready` and the console has a solver.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focus: Option<focus::run::FocusView>,
    /// The calibration library and what matches the latest saved imaging frame (034).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calibration: Option<calibration::service::CalibrationView>,
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
            frame: None,
            solve: None,
            relation: None,
            focus: None,
            calibration: None,
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
    /// The latest imaging frame, and the last one that ended (031 US2).
    frame: Option<FrameView>,
    ended: Option<FrameView>,
    /// The frame in flight's task and the client to abort it with (US5),
    /// cleared when the frame ends.
    frame_task: Option<(JoinHandle<()>, ImagingClient)>,
    /// The console's own frame count, and the UTC date it was last read for
    /// (FR-011).
    count: u64,
    count_date: Option<String>,
    /// The lease out, its grant and its holder's name, and the grant to give
    /// the next one (US9, FR-028).
    lease: Option<(u64, &'static str)>,
    next_grant: u64,
    /// The JPEG previews of the last `PREVIEWS_KEPT` saved frames, by id
    /// (FR-018).
    previews: Vec<(u64, Vec<u8>)>,
    /// The console's imaging solver, and the latest solve (032 US2).
    solver: Option<solving::HeldSolver>,
    solve: Option<solving::SolveView>,
    /// The session whose pointing and target hint a solve (032 US3).
    session: Option<solving::HeldSession>,
    /// The centre of the last imaging solve since the console started, with
    /// the session's target of that moment: the `last_solve` hint's place,
    /// which stops counting once the target changes (032 US3, FR-014).
    last_solve: Option<LastSolve>,
    /// What the console keeps between the two cameras (032 US3, US4).
    relation: relation::Relation,
    /// The task that fills the history of the finder camera's live solves,
    /// started by `set_solver` and aborted with the console's own task
    /// (032 US4, FR-019).
    history_task: Option<JoinHandle<()>>,
    /// The focus state: what the console keeps about focus for one camera, and
    /// the latest run of the focus aid (033 US5, FR-029).
    focus: focus::run::State,
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
            frame: None,
            ended: None,
            frame_task: None,
            count: 0,
            count_date: None,
            lease: None,
            next_grant: 1,
            previews: Vec::new(),
            solver: None,
            solve: None,
            session: None,
            last_solve: None,
            relation: relation::Relation::new(PathBuf::new()),
            history_task: None,
            focus: focus::run::State::new(PathBuf::new()),
        }
    }
}

/// The centre of the last solved imaging frame, with the session's target of
/// that moment: the `last_solve` hint's place and the target it was solved
/// under (032 US3, FR-014).
type LastSolve = ((f64, f64), Option<(f64, f64)>);

/// The paired node's imaging camera as the console holds it: the task that
/// follows the link and the state it keeps. Dropping the `Imaging` ends the task.
pub struct Imaging {
    state: Arc<Mutex<Held>>,
    /// A count that rises with every change of the latest frame, which
    /// `ended` waits on.
    changes: watch::Sender<u64>,
    task: JoinHandle<()>,
    /// Where imaging frames are saved, the console's clock, and the link whose
    /// clock offset and client the frame tasks read.
    root: PathBuf,
    clock: Arc<dyn Clock>,
    link: Arc<LinkManager>,
    /// Weak because calibration owns this imaging state (034).
    calibration: OnceLock<Weak<Calibration>>,
}

impl Drop for Imaging {
    fn drop(&mut self) {
        self.task.abort();
        let mut held = self.state.lock().expect("imaging state");
        // The history's task ends with the console's own (FR-019), and so does
        // the focus aid's (033 US5, FR-029).
        if let Some(task) = held.history_task.take() {
            task.abort();
        }
        held.focus.abort_task();
    }
}

impl Imaging {
    /// Start the task that follows the link. Dropping the `Imaging` ends it.
    pub fn start(root: PathBuf, link: Arc<LinkManager>, clock: Arc<dyn Clock>) -> Arc<Self> {
        // The relation lives in `relation.json` in the imaging folder (FR-017),
        // and the focus state in `focus.json` beside it (033 FR-029).
        let relation = relation::Relation::new(root.join("relation.json"));
        let focus = focus::run::State::new(root.join("focus.json"));
        let state = Arc::new(Mutex::new(Held {
            relation,
            focus,
            ..Held::default()
        }));
        let task = tokio::spawn(follow(link.clone(), state.clone()));
        let (changes, _) = watch::channel(0);
        Arc::new(Self {
            state,
            changes,
            task,
            root,
            clock,
            link,
            calibration: OnceLock::new(),
        })
    }

    /// The imaging folder this state saves under (031).
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Let every ready view carry the calibration object (034). The first call wins.
    pub(crate) fn attach_calibration(&self, calibration: &Arc<Calibration>) {
        let _ = self.calibration.set(Arc::downgrade(calibration));
    }

    /// `GET /api/imaging`, and `imaging` in the session. Outside `ready` the view
    /// holds none of `camera`, `status`, `settings`, `frame` and `solve`. The
    /// aim point in force is read before the lock is taken: no lock is held
    /// across a call into the session, and every answer — this one, the take,
    /// the settings edit, the abort and `Imaging::solve`'s — measures the
    /// relation against it (FR-023).
    pub fn view(&self) -> ImagingView {
        let now = self.clock.now();
        let (in_force, aim_held) = {
            let session = self.state.lock().expect("imaging state").session.clone();
            session
                .map(|session| solving::aim_in_force(&session.0, now))
                .unwrap_or((None, false))
        };
        let held = self.state.lock().expect("imaging state");
        let ready = held.state == "ready";
        // `status.exposing` is the console's own: true while its latest frame
        // is `exposing`, whatever the camera's status, read every 2 s, last
        // said (FR-009).
        let mut status = ready.then(|| held.status.clone()).flatten();
        if let Some(status) = status.as_mut() {
            status.exposing = held
                .frame
                .as_ref()
                .is_some_and(|frame| frame.state == "exposing");
        }
        let mut view = ImagingView {
            state: held.state,
            error: if held.state == "error" {
                held.error.clone()
            } else {
                None
            },
            camera: ready.then(|| held.camera.clone()).flatten(),
            status,
            settings: ready.then(|| held.settings.clone()).flatten(),
            frame: ready.then(|| held.frame.clone()).flatten(),
            solve: ready.then(|| held.solve.clone()).flatten(),
            relation: ready
                .then(|| held.relation.view(in_force, aim_held))
                .flatten(),
            // The focus view only while `ready` with a solver (033 FR-029).
            focus: (ready && held.solver.is_some()).then(|| focus::run::view(&held, now)),
            calibration: None,
        };
        drop(held);
        // The finished view includes the relation's angle; no imaging lock is
        // held when calibration is called, since it also calls into Imaging.
        if ready {
            if let Some(calibration) = self.calibration.get().and_then(Weak::upgrade) {
                view.calibration = Some(calibration.view(Some(&view)));
            }
        }
        view
    }

    /// Apply a settings change (FR-003): the console keeps `exposure_ms`,
    /// `forward_kib_per_s` and `save_at_node`, sends the rest to the node, and
    /// answers the view of the settings then in force. While a lease is out,
    /// anyone but the holder answers its 409 (FR-029).
    pub async fn set(&self, edit: SettingsEdit) -> Result<ImagingView, Refusal> {
        self.set_as(None, edit).await
    }

    /// `set` for one caller (US9): no grant for anyone, a lease's grant for
    /// its holder. The lease's check is made under the state's lock, after the
    /// state's and before the frame-in-flight check.
    async fn set_as(&self, grant: Option<u64>, edit: SettingsEdit) -> Result<ImagingView, Refusal> {
        let (mut client, camera, mut settings) = {
            let held = self.state.lock().expect("imaging state");
            let (client, camera, settings) = match held.state {
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
            };
            // A lease out to someone else answers with its holder's name
            // (FR-029), after the state's check and before the frame's.
            held_by(&held, grant)?;
            // A settings change during an exposure is refused (FR-008).
            if held
                .frame
                .as_ref()
                .is_some_and(|frame| matches!(frame.state, "exposing" | "receiving"))
            {
                return Err(Refusal::conflict("an exposure is in progress"));
            }
            (client, camera, settings)
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

    /// Take one imaging frame (FR-008): the checks and the id under one
    /// acquisition of the state's lock, a task in flight, and the view at once.
    /// Every frame `take` starts is saved to a file. While a lease is out,
    /// anyone but the holder answers its 409 (FR-029).
    pub async fn take(&self, purpose: &str) -> Result<ImagingView, Refusal> {
        self.take_as(None, purpose).await
    }

    /// `take` for one caller (US9), as `set_as`: the lease's check is passed
    /// to `frames::take`, which makes it under its one lock acquisition.
    async fn take_as(&self, grant: Option<u64>, purpose: &str) -> Result<ImagingView, Refusal> {
        let task = frames::take(self, purpose, |held| held_by(held, grant))?;
        let client = task.client.clone();
        let task = tokio::spawn(frames::run(task));
        let mut held = self.state.lock().expect("imaging state");
        held.frame_task = Some((task, client));
        drop(held);
        Ok(self.view())
    }

    /// Take the imaging camera for `holder`, a name that reads well after
    /// "held by" (`"the focus aid"`, `"the stack"`), for one taker at a time
    /// (FR-028). It never waits and looks at nothing but the lease: while
    /// another taker holds the camera it answers a `Refusal` of 409 with
    /// `the imaging camera is held by` and that holder's name.
    pub fn lease(self: &Arc<Self>, holder: &'static str) -> Result<Lease, Refusal> {
        let mut held = self.state.lock().expect("imaging state");
        if let Some((_, who)) = held.lease {
            return Err(Refusal::conflict(format!(
                "the imaging camera is held by {who}"
            )));
        }
        let grant = held.next_grant;
        held.next_grant += 1;
        held.lease = Some((grant, holder));
        Ok(Lease {
            imaging: Arc::downgrade(self),
            grant,
        })
    }

    /// The holder's name while a lease is out (FR-028).
    pub fn holder(&self) -> Option<&'static str> {
        self.state
            .lock()
            .expect("imaging state")
            .lease
            .map(|(_, holder)| holder)
    }

    /// Abort the exposure or the imaging frame in flight (FR-013): the node's
    /// exposure is stopped through `AbortExposure`, and the frame task ends
    /// the frame `aborted` when the node's `Aborted` arrives; a frame still
    /// being received is ended here, its task stopped and its stream dropped.
    /// With nothing in progress nothing changes.
    pub async fn abort(&self) -> ImagingView {
        enum Stop {
            /// The exposure is still running on the node.
            Exposing(Box<ImagingClient>, JoinHandle<()>),
            /// The samples are still arriving.
            Receiving(JoinHandle<()>),
        }
        let stop = {
            let mut held = self.state.lock().expect("imaging state");
            let state = held.frame.as_ref().map(|frame| frame.state);
            match state {
                Some("exposing") => held
                    .frame_task
                    .take()
                    .map(|(task, client)| Stop::Exposing(Box::new(client), task)),
                Some("receiving") => held
                    .frame_task
                    .take()
                    .map(|(task, _)| Stop::Receiving(task)),
                _ => None,
            }
        };
        let Some(stop) = stop else {
            return self.view();
        };
        match stop {
            Stop::Receiving(task) => {
                // The task is ended, dropping its stream, and the frame ends
                // here (FR-013).
                task.abort();
                self.end_aborted();
            }
            Stop::Exposing(mut client, task) => {
                // The task ends the frame when the node's `Aborted` arrives;
                // a failed call ends it here.
                if client.abort().await.is_err() {
                    task.abort();
                    self.end_aborted();
                }
            }
        }
        self.view()
    }

    /// End the latest frame `aborted` if it is still in flight (FR-013).
    fn end_aborted(&self) {
        let mut held = self.state.lock().expect("imaging state");
        if let Some(frame) = held.frame.as_mut() {
            if matches!(frame.state, "exposing" | "receiving") {
                frame.state = "aborted";
                held.ended = Some(frame.clone());
                drop(held);
                self.changes.send_modify(|count| *count += 1);
            }
        }
    }

    /// The frame with this id as it ended: the last ended frame when its id is
    /// `id`, otherwise the latest frame if its id is `id` and it has ended; for
    /// an older id, the last ended frame. Waits while the frame asked for is
    /// still in flight.
    pub async fn ended(&self, id: u64) -> FrameView {
        let mut changes = self.changes.subscribe();
        loop {
            {
                let held = self.state.lock().expect("imaging state");
                if held.ended.as_ref().is_some_and(|frame| frame.id == id) {
                    return held.ended.clone().expect("the frame just found");
                }
                if let Some(frame) = held.frame.clone().filter(|frame| frame.id == id) {
                    if matches!(frame.state, "saved" | "failed" | "aborted") {
                        return frame;
                    }
                } else if let Some(ended) = held.ended.clone() {
                    // An older id: the last ended frame.
                    return ended;
                }
            }
            // The frame asked for is still in flight: wait for it to move on.
            if changes.changed().await.is_err() {
                let held = self.state.lock().expect("imaging state");
                return held
                    .ended
                    .clone()
                    .or_else(|| held.frame.clone())
                    .expect("a frame to answer with");
            }
        }
    }

    /// The JPEG preview of the frame with this id, among the last
    /// `PREVIEWS_KEPT` saved frames, or none (FR-018).
    pub fn preview(&self, id: u64) -> Option<Vec<u8>> {
        let held = self.state.lock().expect("imaging state");
        held.previews
            .iter()
            .find(|(kept, _)| *kept == id)
            .map(|(_, jpeg)| jpeg.clone())
    }
}

/// The imaging camera, held for one taker (031 US9). Dropping it releases the
/// camera at once, also when the task that owns it is aborted. It holds no
/// strong reference to the `Imaging` (FR-032).
#[derive(Debug)]
pub struct Lease {
    /// The `Imaging` the camera belongs to, held weakly: a lease keeps no
    /// console alive.
    imaging: Weak<Imaging>,
    /// Which grant this is; the `Drop` clears the holder only while it is
    /// still this one.
    grant: u64,
}

/// The refusal of a lease whose `Imaging` is gone (FR-032).
fn gone() -> Refusal {
    Refusal::conflict("this console takes no imaging frames")
}

impl Lease {
    /// `Imaging::set` for the holder (FR-030): the same checks, refusals,
    /// words and order, less the lease's own. With the `Imaging` gone: 409
    /// `this console takes no imaging frames`.
    pub async fn set(&self, edit: SettingsEdit) -> Result<ImagingView, Refusal> {
        match self.imaging.upgrade() {
            Some(imaging) => imaging.set_as(Some(self.grant), edit).await,
            None => Err(gone()),
        }
    }

    /// `Imaging::take` for the holder, as `set` (FR-030).
    pub async fn take(&self, purpose: &str) -> Result<ImagingView, Refusal> {
        match self.imaging.upgrade() {
            Some(imaging) => imaging.take_as(Some(self.grant), purpose).await,
            None => Err(gone()),
        }
    }
}

impl Drop for Lease {
    /// Release the camera at once (FR-032): nothing is awaited, so a
    /// cancelled task's drop of its lease releases the camera in the instant
    /// the task ends. A lease whose `Imaging` is gone does nothing.
    fn drop(&mut self) {
        if let Some(imaging) = self.imaging.upgrade() {
            let mut held = imaging.state.lock().expect("imaging state");
            if held.lease.is_some_and(|(grant, _)| grant == self.grant) {
                held.lease = None;
            }
        }
    }
}

/// The lease's check (FR-029): a caller whose grant is not the one out is
/// refused with its holder's name. Made under the state's lock, after the
/// state's check and before the frame-in-flight check.
fn held_by(held: &Held, grant: Option<u64>) -> Result<(), Refusal> {
    match held.lease {
        Some((out, holder)) if Some(out) != grant => Err(Refusal::conflict(format!(
            "the imaging camera is held by {holder}"
        ))),
        _ => Ok(()),
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
    held.camera = Some(camera.clone());
    // The camera was described: the relation reads `relation.json` for this
    // camera, and a file made for another is ignored (FR-017); the focus state
    // reads `focus.json` beside it (033 FR-029).
    held.relation.describe(relation::CameraId::from(&camera));
    held.focus.describe(relation::CameraId::from(&camera));
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
