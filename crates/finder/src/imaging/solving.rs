//! Solving an imaging frame (032): the request, the solve, its view in the
//! imaging view, and the solve sidecar written beside the frame's file.

use std::{path::Path, sync::Arc};

use serde::Serialize;
use tokio::sync::{broadcast, watch};

use super::{
    relation,
    solver::{HintSource, ImagingSolve, ImagingSolver, SolveError, SolveFrame, SolveHint},
    Held, Imaging, ImagingView, Refusal,
};
use crate::session::{Aim, AimView, Clock, PointingSource, Session};

/// The format line of a solve sidecar (FR-011).
pub const SIDECAR_FORMAT: &str = "imaging-solve/1";

/// How old a live pointing may be to hint a solve: no more than this many
/// seconds before the exposure's start or after its end (FR-014).
pub const HINT_AGE_S: f64 = 30.0;
/// The radius of a hint that names a place (FR-015).
pub const WIDE_RADIUS_DEG: f64 = 3.0;
/// The radius of a hint that names no place: a search of the whole sky
/// (FR-015).
pub const WHOLE_SKY_DEG: f64 = 180.0;
/// Arcseconds in a radian, for the focal length's scale (FR-016).
pub const ARCSEC_PER_RADIAN: f64 = 206_264.806;

/// One saved imaging frame to solve.
#[derive(Debug, Clone)]
pub struct Ask<'a> {
    /// The imaging folder, and the frame's file relative to it, as 031's
    /// `FrameView::file` gives it: `frames/<UTC date>/<id>.fits`.
    pub root: &'a Path,
    pub file: &'a str,
    pub frame_id: u64,
    pub frame: SolveFrame,
    pub hint: SolveHint,
    /// Console Unix seconds when the solve was asked for.
    pub requested_at: f64,
}

/// One ended solve.
#[derive(Debug, Clone, PartialEq)]
pub struct Ended {
    pub outcome: Result<ImagingSolve, SolveError>,
    /// The solve's own figure for a solved solve; for a failed one, the
    /// console clock's time from `requested_at` to the answer, in milliseconds.
    pub took_ms: f64,
    /// The solve sidecar, relative to the imaging folder; `None` when it could
    /// not be written.
    pub file: Option<String>,
}

/// The latest solve as the imaging view shows it (FR-010). Every `Option` is
/// skipped while `None`. Non-exhaustive: later stories add parts.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct SolveView {
    pub frame_id: u64,
    /// `solving`, `solved` or `failed`.
    pub state: &'static str,
    /// Console Unix seconds.
    pub requested_at: f64,
    pub solver: &'static str,
    pub hint: SolveHint,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ra_deg: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dec_deg: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub up_east_of_north_deg: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arcsec_per_px: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mirrored: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cd_deg: Option<[[f64; 2]; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stars: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub took_ms: Option<f64>,
    /// `frames/<UTC date>/<id>.solve.json`, relative to the imaging folder.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// What a hint is built from (032 US3). The tests build one by literal, so
/// every field is here from US3 on and no later story or spec adds one
/// without planning it.
pub struct HintInputs<'a> {
    /// The frame about to be solved.
    pub frame: &'a SolveFrame,
    /// The sensor's width and height in pixels, from the camera's description.
    pub sensor: (u32, u32),
    /// A live pointing: RA, Dec, time (FR-014).
    pub pointing: Option<(f64, f64, f64)>,
    /// The centre of the last imaging solve since the console started.
    pub last_solve: Option<(f64, f64)>,
    /// The session's target.
    pub target: Option<(f64, f64)>,
    /// Arcseconds a sensor pixel, and the focal length in millimetres:
    /// `Relation::height_inputs()` (FR-016).
    pub scale: Option<f64>,
    pub pixel_um: f64,
    pub focal_length_mm: Option<f64>,
    /// Whether the session's aim point is a measured one. US3 passes false.
    pub aim_measured: bool,
    /// How many places down the list of sources that exist the hint goes:
    /// `Relation::hint_skip()` (FR-014).
    pub skip: usize,
}

/// The hint for a solve (FR-014, FR-015, FR-016, FR-028): among the sources
/// that exist — `pointing` when its time is within `HINT_AGE_S` of the
/// exposure, `last_solve`, `target`, and `none`, which always exists — the
/// one `skip` places down, never past `none`; the radius 3°, narrowed for
/// `pointing` to the sensor's diagonal on the sky once the session's aim
/// point is measured and a scale sizes the field, never under 0.2° or over
/// 3°, or the whole sky for `none`; and the field height, the frame's rows
/// times its binning times the scale, over 3600, from the scale or from the
/// focal length, or absent.
pub fn hint_for(inputs: &HintInputs<'_>) -> SolveHint {
    // The sources that exist, in order (FR-014).
    let pointing = inputs.pointing.filter(|&(_, _, time)| {
        time >= inputs.frame.start - HINT_AGE_S
            && time <= inputs.frame.start + inputs.frame.exposure_s + HINT_AGE_S
    });
    let mut sources: Vec<(HintSource, (f64, f64))> = Vec::new();
    if let Some((ra_deg, dec_deg, _)) = pointing {
        sources.push((HintSource::Pointing, (ra_deg, dec_deg)));
    }
    if let Some(place) = inputs.last_solve {
        sources.push((HintSource::LastSolve, place));
    }
    if let Some(place) = inputs.target {
        sources.push((HintSource::Target, place));
    }
    // `none` always exists and holds the last place: no miss can lock solving
    // out of the whole sky (FR-014).
    let index = inputs.skip.min(sources.len());
    let (source, position) = if index < sources.len() {
        let (source, place) = sources[index];
        (source, Some(place))
    } else {
        (HintSource::None, None)
    };
    let radius_deg = if source == HintSource::None {
        WHOLE_SKY_DEG
    } else if source == HintSource::Pointing && inputs.aim_measured {
        // Once the session's aim point is the measured one, and while the kept
        // scale is the field height's source, a hint from a live pointing is
        // narrowed to the sensor's diagonal on the sky (FR-028) — the camera's
        // whole width and height, whatever region the frame covers, since the
        // place of such a hint is the sensor's centre, and a frame of a region
        // may lie anywhere on the sensor — never under 0.2° or over 3°.
        inputs
            .scale
            .map(|scale| {
                (f64::from(inputs.sensor.0).hypot(f64::from(inputs.sensor.1)) * scale / 3600.0)
                    .clamp(relation::NARROW_MIN_DEG, WIDE_RADIUS_DEG)
            })
            .unwrap_or(WIDE_RADIUS_DEG)
    } else {
        WIDE_RADIUS_DEG
    };
    // The field height (FR-016): the frame's rows times its binning times the
    // scale, over 3600 — the kept scale's, or the focal length's.
    let rows = f64::from(inputs.frame.height) * f64::from(inputs.frame.binning);
    let fov_deg = match inputs.scale {
        Some(scale) => Some(rows * scale / 3600.0),
        None => inputs
            .focal_length_mm
            .map(|focal| rows * ARCSEC_PER_RADIAN * (inputs.pixel_um / 1000.0) / focal / 3600.0),
    };
    SolveHint {
        source,
        position,
        radius_deg,
        fov_deg,
    }
}

/// The imaging solver as the imaging state holds it: wrapped so `Held` stays
/// `Debug`, printed by its name.
#[derive(Clone)]
pub(crate) struct HeldSolver(pub Arc<dyn ImagingSolver>);

impl std::fmt::Debug for HeldSolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.name())
    }
}

/// The imaging session as the imaging state holds it (032 US3): wrapped so
/// `Held` stays `Debug`, printed as `session`.
#[derive(Clone)]
pub(crate) struct HeldSession(pub Arc<crate::session::Session>);

impl std::fmt::Debug for HeldSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("session")
    }
}

/// Solve one saved imaging frame and write its solve sidecar beside it
/// (FR-009, FR-011). It touches neither the imaging view nor the relation;
/// `Imaging::solve` is built on it, and a later spec that solves a frame other
/// than the latest one calls it directly. A failed write turns the outcome into
/// `Failed` and leaves `file` `None`. Nothing here retries.
pub async fn solve_frame(solver: &dyn ImagingSolver, clock: &dyn Clock, ask: Ask<'_>) -> Ended {
    let outcome = solver.solve(ask.frame.clone(), ask.hint).await;
    let took_ms = match &outcome {
        Ok(solve) => solve.took_ms,
        Err(_) => (clock.now() - ask.requested_at) * 1000.0,
    };
    let sidecar = sidecar_name(ask.file);
    let bytes = sidecar_bytes(solver.name(), &ask, &outcome, took_ms);
    let root = ask.root.to_path_buf();
    let name = sidecar.clone();
    let written = tokio::task::spawn_blocking(move || write_sidecar(&root, &name, &bytes))
        .await
        .expect("the solve writer cannot be cancelled");
    match written {
        Ok(()) => Ended {
            outcome,
            took_ms,
            file: Some(sidecar),
        },
        Err(error) => Ended {
            outcome: Err(SolveError::Failed(format!(
                "could not save the imaging solve: {error}"
            ))),
            took_ms,
            file: None,
        },
    }
}

/// The solve sidecar's name, relative to the imaging folder: the frame's file
/// name with `.fits` replaced by `.solve.json` (FR-011).
fn sidecar_name(file: &str) -> String {
    file.strip_suffix(".fits")
        .map(|base| format!("{base}.solve.json"))
        .unwrap_or_else(|| format!("{file}.solve.json"))
}

/// The sidecar's bytes (FR-011): the format, the frame, the solver, the hint,
/// `solved` and `took_ms`, then the solve's numbers or the failure's reason.
fn sidecar_bytes(
    solver: &str,
    ask: &Ask<'_>,
    outcome: &Result<ImagingSolve, SolveError>,
    took_ms: f64,
) -> Vec<u8> {
    let frame = Path::new(ask.file)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| ask.file.to_string());
    let mut value = serde_json::json!({
        "format": SIDECAR_FORMAT,
        "frame_id": ask.frame_id,
        "frame": frame,
        "solver": solver,
        "requested_at": ask.requested_at,
        "hint": ask.hint,
        "solved": outcome.is_ok(),
        "took_ms": took_ms,
    });
    let sidecar = value.as_object_mut().expect("the sidecar's keys");
    match outcome {
        Ok(solve) => {
            sidecar.insert("ra_deg".to_string(), serde_json::json!(solve.ra_deg));
            sidecar.insert("dec_deg".to_string(), serde_json::json!(solve.dec_deg));
            sidecar.insert(
                "up_east_of_north_deg".to_string(),
                serde_json::json!(solve.up_east_of_north_deg),
            );
            sidecar.insert(
                "arcsec_per_px".to_string(),
                serde_json::json!(solve.arcsec_per_px),
            );
            sidecar.insert("mirrored".to_string(), serde_json::json!(solve.mirrored));
            sidecar.insert("cd_deg".to_string(), serde_json::json!(solve.cd_deg));
            sidecar.insert("width".to_string(), serde_json::json!(solve.width));
            sidecar.insert("height".to_string(), serde_json::json!(solve.height));
            if let Some(stars) = solve.stars {
                sidecar.insert("stars".to_string(), serde_json::json!(stars));
            }
            if let Some(warning) = &solve.warning {
                sidecar.insert("warning".to_string(), serde_json::json!(warning));
            }
        }
        Err(error) => {
            sidecar.insert("reason".to_string(), serde_json::json!(error.reason()));
            sidecar.insert("error".to_string(), serde_json::json!(error.message()));
            if let Some(detail) = error.detail() {
                sidecar.insert("detail".to_string(), serde_json::json!(detail));
            }
        }
    }
    serde_json::to_vec(&value).expect("the sidecar's numbers are finite")
}

/// Write the solve sidecar whole under a `.part` name and rename it over any
/// earlier one (FR-011). Answers the error words of a failed write.
fn write_sidecar(root: &Path, relative: &str, bytes: &[u8]) -> Result<(), String> {
    let file = place(root, relative);
    let part = place(root, &format!("{relative}.part"));
    std::fs::create_dir_all(file.parent().expect("the frames folder"))
        .map_err(|error| error.to_string())?;
    std::fs::write(&part, bytes).map_err(|error| error.to_string())?;
    std::fs::rename(&part, &file).map_err(|error| error.to_string())
}

/// The file's place under the imaging folder, split for the platform's
/// separators.
fn place(root: &Path, relative: &str) -> std::path::PathBuf {
    let mut path = root.to_path_buf();
    for part in relative.split('/') {
        path.push(part);
    }
    path
}

/// The words of each state that is not `ready`, as `set` refuses them (031
/// FR-008).
fn state_refusal(state: &str, error: Option<&str>) -> Refusal {
    match state {
        "no_link" => Refusal::conflict("the scope node is not connected"),
        "no_service" => Refusal::conflict("this scope node has no imaging service"),
        "no_camera" => Refusal::conflict("this scope node has no imaging camera"),
        _ => Refusal::conflict(format!(
            "the imaging camera is not answering: {}",
            error.unwrap_or_default()
        )),
    }
}

/// The aim point the relation is measured against (032 US4, FR-023,
/// FR-036): the session's held one (the session holds it back while Track is
/// on), marked `measured`, when there is one; else the session's `aim`. The
/// `bool` is whether it is held.
pub(crate) fn aim_in_force(session: &Session, now: f64) -> (Option<AimView>, bool) {
    if let Some(aim) = session.held_aim() {
        return (
            Some(AimView {
                x: aim.x,
                y: aim.y,
                source: Some("measured"),
            }),
            true,
        );
    }
    (session.snapshot(now).aim, false)
}

/// The history of the finder camera's live solves (FR-019): one more
/// subscriber of the live broadcast, which pairs each report that solved with
/// the session's live pointing of that solve and pushes a finder fix, kept for
/// the longer of `HISTORY_MIN_S` and four imaging exposures. A lagged receiver
/// carries on; a closed one ends the task. A report whose pointing has
/// already been replaced is skipped.
async fn history(
    live: Arc<crate::live::Live>,
    session: Arc<Session>,
    clock: Arc<dyn Clock>,
    held: std::sync::Arc<std::sync::Mutex<Held>>,
) {
    let mut reports = live.subscribe();
    loop {
        let report = match reports.recv().await {
            Ok(report) => report,
            Err(broadcast::error::RecvError::Lagged(_)) => continue,
            Err(broadcast::error::RecvError::Closed) => return,
        };
        let Some(seq) = report.pointing_seq else {
            continue;
        };
        let fix = session
            .snapshot(clock.now())
            .pointing
            .filter(|pointing| pointing.seq == seq && pointing.source == PointingSource::Live)
            .map(|pointing| relation::FinderFix {
                time: pointing.time,
                frame: pointing.frame,
                frame_id: report.frame_id,
            });
        let Some(fix) = fix else {
            continue;
        };
        let mut held = held.lock().expect("imaging state");
        let keep_s = relation::HISTORY_MIN_S.max(4.0 * held.exposure_ms / 1000.0);
        held.relation.push_fix(fix, keep_s);
    }
}

/// Take one aim reading from a solved imaging frame (FR-020 to FR-022,
/// FR-036): the solve task's step 2 and the whole of `Imaging::note_solve`.
/// It holds no `&Imaging`, and the imaging state's lock is never held across a
/// call into the session.
fn take_aim_reading(
    held: &std::sync::Arc<std::sync::Mutex<Held>>,
    clock: &dyn Clock,
    session: &Session,
    frame: &SolveFrame,
    frame_id: u64,
    solve: &ImagingSolve,
) {
    // 1. Only a whole-sensor frame takes an aim reading.
    let (fixes, whole) = {
        let held = held.lock().expect("imaging state");
        let whole = held.camera.as_ref().is_some_and(|camera| {
            frame.roi.x == 0
                && frame.roi.y == 0
                && frame.roi.width == camera.width
                && frame.roi.height == camera.height
        });
        (held.relation.fixes(), whole)
    };
    if !whole {
        return;
    }
    // 2. The aim reading from the history's live solves.
    let outcome = relation::read_aim(&fixes, solve, frame.start, frame.exposure_s, frame_id);
    // 3. The aim point in force of now.
    let now = clock.now();
    let (in_force, _) = aim_in_force(session, now);
    let aim = {
        let mut held = held.lock().expect("imaging state");
        held.relation.take(outcome, frame_id, in_force)
    };
    // 4. Applied, or held back while Track is on.
    if let Some((x, y)) = aim {
        if let Err(error) = session.set_measured_aim(Aim { x, y }, now) {
            eprintln!("could not set the measured aim point: {error}");
        }
    }
}

impl Imaging {
    /// Give the console its imaging solver (032), with the session that hints
    /// it and the live loop whose solves it remembers: a task that follows
    /// the live loop's reports fills the history of the finder camera's live
    /// solves (FR-019). Called by `AppState::with_imaging_solver`.
    pub fn set_solver(
        &self,
        solver: Arc<dyn ImagingSolver>,
        session: Arc<Session>,
        live: Option<Arc<crate::live::Live>>,
    ) {
        let source_session = session.clone();
        self.steering()
            .set_source(Arc::new(move || source_session.steered()));
        let history = live.map(|live| {
            tokio::spawn(history(
                live,
                session.clone(),
                self.clock.clone(),
                self.state.clone(),
            ))
        });
        let mut held = self.state.lock().expect("imaging state");
        held.solver = Some(HeldSolver(solver));
        held.session = Some(HeldSession(session));
        if let Some(task) = history {
            if let Some(old) = held.history_task.replace(task) {
                old.abort();
            }
        }
    }

    /// The console's imaging solver, when it has one (FR-009).
    pub fn solver(&self) -> Option<Arc<dyn ImagingSolver>> {
        self.state
            .lock()
            .expect("imaging state")
            .solver
            .clone()
            .map(|held| held.0)
    }

    /// Offer a solve that `solve_frame` made outside `Imaging::solve`
    /// (FR-037, for a later spec that solves frames itself). A whole-sensor
    /// frame takes an aim reading exactly as the task's step 2 does; a frame
    /// of a region does nothing. It changes neither the imaging view's
    /// `solve`, the kept scale nor the misses.
    pub fn note_solve(&self, frame: &SolveFrame, frame_id: u64, solve: &ImagingSolve) {
        let session = self
            .state
            .lock()
            .expect("imaging state")
            .session
            .clone()
            .map(|session| session.0);
        if let Some(session) = session {
            take_aim_reading(
                &self.state,
                self.clock.as_ref(),
                &session,
                frame,
                frame_id,
                solve,
            );
        }
    }

    /// Solve the latest imaging frame (FR-009): the checks and the frame under
    /// the state's lock, the hint built with the lock released — it takes the
    /// lock itself and reads the session — and then the solve's start under
    /// one more acquisition, `an imaging solve is already running` checked once
    /// more. A task in flight, and the view at once. Nothing here retries a
    /// failed solve (FR-012).
    pub async fn solve(&self) -> Result<ImagingView, Refusal> {
        let (solver, id, file, frame) = {
            let held = self.state.lock().expect("imaging state");
            if held.state != "ready" {
                return Err(state_refusal(held.state, held.error.as_deref()));
            }
            let Some(solver) = held.solver.clone().map(|held| held.0) else {
                return Err(Refusal::conflict("this console has no imaging solver"));
            };
            if held
                .solve
                .as_ref()
                .is_some_and(|solve| solve.state == "solving")
            {
                return Err(Refusal::conflict("an imaging solve is already running"));
            }
            let Some(frame) = held.frame.clone() else {
                return Err(Refusal::conflict("take an imaging frame first"));
            };
            if matches!(frame.state, "exposing" | "receiving") {
                return Err(Refusal::conflict("the imaging frame is still arriving"));
            }
            if matches!(frame.state, "failed" | "aborted") {
                return Err(Refusal::conflict("the latest imaging frame was not saved"));
            }
            // The saved frame as the solver needs it; a `saved` frame has all
            // of these, and without one of them the frame is not one.
            let (Some(width), Some(height), Some(bits), Some(binning), Some(start), Some(file)) = (
                frame.width,
                frame.height,
                frame.bits,
                frame.binning,
                frame.start,
                frame.file.clone(),
            ) else {
                return Err(Refusal::conflict("the latest imaging frame was not saved"));
            };
            let id = frame.id;
            let frame = SolveFrame {
                path: self.root.join(&file),
                width,
                height,
                bits,
                bayer: frame.bayer.clone(),
                binning,
                roi: super::solver::Region::from(frame.roi),
                start,
                exposure_s: frame.exposure_ms / 1000.0,
            };
            (solver, id, file, frame)
        };
        // The hint the console would give this frame now (FR-014, FR-015,
        // FR-016), built with the lock released.
        let hint = self.hint(&frame);
        let task = {
            let mut held = self.state.lock().expect("imaging state");
            if held
                .solve
                .as_ref()
                .is_some_and(|solve| solve.state == "solving")
            {
                return Err(Refusal::conflict("an imaging solve is already running"));
            }
            let requested_at = self.clock.now();
            held.solve = Some(SolveView {
                frame_id: id,
                state: "solving",
                requested_at,
                solver: solver.name(),
                hint,
                ra_deg: None,
                dec_deg: None,
                up_east_of_north_deg: None,
                arcsec_per_px: None,
                mirrored: None,
                cd_deg: None,
                stars: None,
                warning: None,
                took_ms: None,
                file: None,
                reason: None,
                error: None,
                detail: None,
            });
            SolveTask {
                held: self.state.clone(),
                changes: self.changes.clone(),
                clock: self.clock.clone(),
                session: held.session.as_ref().map(|session| session.0.clone()),
                solver,
                root: self.root.clone(),
                frame_id: id,
                file,
                frame,
                hint,
                requested_at,
            }
        };
        tokio::spawn(run(task));
        Ok(self.view())
    }

    /// The hint the console would give this frame now (FR-014, FR-015,
    /// FR-016, FR-035). `Imaging::solve` uses it, and so does a later spec
    /// that calls `solve_frame` itself. It takes the imaging state's lock and
    /// reads the session, so it is never called with the lock held.
    pub fn hint(&self, frame: &SolveFrame) -> SolveHint {
        let now = self.clock.now();
        // The session's pointing, target and aim point, read without the
        // imaging state's lock: no lock is held across a call into the
        // session.
        let session = self
            .state
            .lock()
            .expect("imaging state")
            .session
            .as_ref()
            .map(|session| session.0.clone());
        let (latest, target, aim) = match &session {
            Some(session) => {
                let view = session.snapshot(now);
                (
                    view.pointing
                        .as_ref()
                        .filter(|pointing| pointing.source == PointingSource::Live)
                        .map(|pointing| (pointing.ra_deg, pointing.dec_deg, pointing.time)),
                    view.target.map(|target| (target.ra_deg, target.dec_deg)),
                    view.aim,
                )
            }
            None => (None, None, None),
        };
        let (sensor, pixel_um, scale, focal_length_mm, skip, last_solve, fixes) = {
            let held = self.state.lock().expect("imaging state");
            let (scale, focal_length_mm) = held.relation.height_inputs();
            let skip = held.relation.hint_skip();
            // The last imaging solve stops counting once the target changes
            // (FR-014).
            let last_solve = held
                .last_solve
                .filter(|(_, solved_with)| *solved_with == target)
                .map(|(centre, _)| centre);
            let fixes = held.relation.fixes();
            match &held.camera {
                Some(camera) => (
                    (camera.width, camera.height),
                    camera.pixel_um,
                    scale,
                    focal_length_mm,
                    skip,
                    last_solve,
                    fixes,
                ),
                None => (
                    (frame.roi.width, frame.roi.height),
                    0.0,
                    scale,
                    focal_length_mm,
                    skip,
                    last_solve,
                    fixes,
                ),
            }
        };
        // The `pointing` source is the history's live solve nearest the
        // middle of the exposure, among those no more than `HINT_AGE_S`
        // before its start or after its end, carried through the aim point as
        // a live pointing is; the session's latest pointing only when the
        // history holds none (FR-035).
        let middle = frame.start + frame.exposure_s / 2.0;
        let pointing = fixes
            .iter()
            .filter(|fix| {
                fix.time >= frame.start - HINT_AGE_S
                    && fix.time <= frame.start + frame.exposure_s + HINT_AGE_S
            })
            .min_by(|a, b| (a.time - middle).abs().total_cmp(&(b.time - middle).abs()))
            .and_then(|fix| {
                let direction = match aim {
                    Some(aim) => fix.frame.unproject((aim.x, aim.y)),
                    None => Some(fix.frame.center),
                };
                direction.map(|direction| {
                    let (ra, dec) = direction.to_radec();
                    (ra.to_degrees(), dec.to_degrees(), fix.time)
                })
            })
            .or(latest);
        hint_for(&HintInputs {
            frame,
            sensor,
            pointing,
            last_solve,
            target,
            scale,
            pixel_um,
            focal_length_mm,
            aim_measured: aim.is_some_and(|aim| aim.source == Some("measured")),
            skip,
        })
    }

    /// The focal length, in millimetres, that sizes a solve's field until a
    /// scale is kept (FR-018), for `AppState::with_focal_length_mm`.
    pub fn set_focal_length_mm(&self, mm: f64) {
        self.state
            .lock()
            .expect("imaging state")
            .relation
            .set_focal_length_mm(mm);
    }

    /// Arcseconds a sensor pixel (FR-018): the kept scale, or without one the
    /// focal length's with the camera's pixel size. `None` with neither.
    pub fn scale_arcsec_per_px(&self) -> Option<f64> {
        self.state
            .lock()
            .expect("imaging state")
            .relation
            .scale_arcsec_per_px()
    }

    /// The latest solve once its `frame_id` is above `frame_id`, or is
    /// `frame_id` and its `state` is not `solving`, as 031's `ended` answers a
    /// frame.
    pub async fn solved(&self, frame_id: u64) -> SolveView {
        let mut changes = self.changes.subscribe();
        loop {
            {
                let held = self.state.lock().expect("imaging state");
                if let Some(solve) = held.solve.clone().filter(|solve| {
                    solve.frame_id > frame_id
                        || (solve.frame_id == frame_id && solve.state != "solving")
                }) {
                    return solve;
                }
            }
            if changes.changed().await.is_err() {
                let held = self.state.lock().expect("imaging state");
                return held.solve.clone().expect("a solve to answer with");
            }
        }
    }
}

/// One solve in flight: what its task needs beside the state it reports to,
/// as 031's frame task holds its own.
struct SolveTask {
    held: std::sync::Arc<std::sync::Mutex<Held>>,
    changes: watch::Sender<u64>,
    clock: Arc<dyn Clock>,
    /// The session whose target of that moment a solved solve is kept with
    /// (US3).
    session: Option<Arc<crate::session::Session>>,
    solver: Arc<dyn ImagingSolver>,
    root: std::path::PathBuf,
    frame_id: u64,
    file: String,
    frame: SolveFrame,
    hint: SolveHint,
    requested_at: f64,
}

/// Follow one solve and show its answer (FR-009, FR-010, FR-011, FR-016,
/// FR-017).
async fn run(task: SolveTask) {
    let ask = Ask {
        root: &task.root,
        file: &task.file,
        frame_id: task.frame_id,
        frame: task.frame.clone(),
        hint: task.hint,
        requested_at: task.requested_at,
    };
    let ended = solve_frame(task.solver.as_ref(), task.clock.as_ref(), ask).await;
    let view = ended_view(&task, ended.clone());
    // The relation is told how the solve ended, and a solved solve's centre
    // kept, before the view shows the solve's end, so a reader who sees it
    // also sees what it changed. The session is read without the imaging
    // state's lock.
    let now = task.clock.now();
    let target = task
        .session
        .as_ref()
        .and_then(|session| session.snapshot(now).target)
        .map(|target| (target.ra_deg, target.dec_deg));
    // 1. The relation: the scale a solved solve measured, the miss a failed
    //    one counts, the centre kept with the target.
    {
        let mut held = task.held.lock().expect("imaging state");
        match &ended.outcome {
            Ok(solve) => {
                held.relation.store_scale(
                    solve.arcsec_per_px / f64::from(task.frame.binning),
                    task.frame_id,
                    now,
                );
                held.last_solve = Some(((solve.ra_deg, solve.dec_deg), target));
            }
            Err(SolveError::NotSolved { .. } | SolveError::TimedOut { .. }) => {
                held.relation.miss();
            }
            Err(_) => {}
        }
    }
    // 2. A solved solve of a whole-sensor frame takes an aim reading
    //    (FR-020), before the view shows the solve's end.
    if let (Ok(solve), Some(session)) = (&ended.outcome, task.session.as_ref()) {
        take_aim_reading(
            &task.held,
            task.clock.as_ref(),
            session,
            &task.frame,
            task.frame_id,
            solve,
        );
    }
    // 3. The view shows the solve's end.
    {
        let mut held = task.held.lock().expect("imaging state");
        held.solve = Some(view);
    }
    // Wake whoever waits in `solved`, as a frame does when it ends.
    task.changes.send_modify(|count| *count += 1);
}

/// The ended solve's view (FR-010).
fn ended_view(task: &SolveTask, ended: Ended) -> SolveView {
    let mut view = SolveView {
        frame_id: task.frame_id,
        state: "failed",
        requested_at: task.requested_at,
        solver: task.solver.name(),
        hint: task.hint,
        ra_deg: None,
        dec_deg: None,
        up_east_of_north_deg: None,
        arcsec_per_px: None,
        mirrored: None,
        cd_deg: None,
        stars: None,
        warning: None,
        took_ms: Some(ended.took_ms),
        file: ended.file.clone(),
        reason: None,
        error: None,
        detail: None,
    };
    match ended.outcome {
        Ok(solve) => {
            view.state = "solved";
            view.ra_deg = Some(solve.ra_deg);
            view.dec_deg = Some(solve.dec_deg);
            view.up_east_of_north_deg = Some(solve.up_east_of_north_deg);
            view.arcsec_per_px = Some(solve.arcsec_per_px);
            view.mirrored = Some(solve.mirrored);
            view.cd_deg = Some(solve.cd_deg);
            view.stars = solve.stars;
            view.warning = solve.warning;
        }
        Err(error) => {
            view.reason = Some(error.reason());
            view.error = Some(error.message());
            view.detail = error.detail().map(str::to_string);
        }
    }
    view
}
