//! Solving an imaging frame (032): the request, the solve, its view in the
//! imaging view, and the solve sidecar written beside the frame's file.

use std::{path::Path, sync::Arc};

use serde::Serialize;
use tokio::sync::watch;

use super::{
    solver::{ImagingSolve, ImagingSolver, SolveError, SolveFrame, SolveHint},
    Held, Imaging, ImagingView, Refusal,
};
use crate::session::Clock;

/// The format line of a solve sidecar (FR-011).
pub const SIDECAR_FORMAT: &str = "imaging-solve/1";

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

/// The imaging solver as the imaging state holds it: wrapped so `Held` stays
/// `Debug`, printed by its name.
#[derive(Clone)]
pub(crate) struct HeldSolver(pub Arc<dyn ImagingSolver>);

impl std::fmt::Debug for HeldSolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.name())
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

impl Imaging {
    /// Give the console its imaging solver (032), with the session that hints
    /// it and the live loop whose solves it remembers. Called by
    /// `AppState::with_imaging_solver`; the session and the live loop are
    /// later stories' to read.
    pub fn set_solver(
        &self,
        solver: Arc<dyn ImagingSolver>,
        _session: Arc<crate::session::Session>,
        _live: Option<Arc<crate::live::Live>>,
    ) {
        self.state.lock().expect("imaging state").solver = Some(HeldSolver(solver));
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

    /// Solve the latest imaging frame (FR-009): the checks and the solve's
    /// start under one acquisition of the state's lock, a task in flight, and
    /// the view at once. Nothing here retries a failed solve (FR-012).
    pub async fn solve(&self) -> Result<ImagingView, Refusal> {
        let task = {
            let mut held = self.state.lock().expect("imaging state");
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
            let hint = SolveHint::whole_sky();
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
    solver: Arc<dyn ImagingSolver>,
    root: std::path::PathBuf,
    frame_id: u64,
    file: String,
    frame: SolveFrame,
    hint: SolveHint,
    requested_at: f64,
}

/// Follow one solve and show its answer (FR-009, FR-010, FR-011).
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
    let view = ended_view(&task, ended);
    task.held.lock().expect("imaging state").solve = Some(view);
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
