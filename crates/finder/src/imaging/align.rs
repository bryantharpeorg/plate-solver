//! The finder alignment (050): imaging frames under the imaging camera's
//! lease, each solved and paired with the finder camera's live solve nearest
//! its middle, and where the imaging camera's centre lies from the finder
//! camera's centre, as the finder camera sees it, off its frame too. It moves
//! nothing. Every solve is offered to 032 through `Imaging::note_solve`.

use std::{
    path::Path,
    sync::{Arc, Weak},
};

use math_core::{angular_distance, UnitVector};
use serde::{Deserialize, Serialize};
use tokio::{sync::Notify, task::JoinHandle};

use super::{
    relation::FinderFix,
    solver::{HintSource, ImagingSolver, SolveFrame},
    solving, Held, Imaging, ImagingView, Lease, Refusal, RoiEdit, SettingsEdit,
};

/// The name the finder alignment holds the imaging camera's lease under.
pub const HOLDER: &str = "the finder alignment";
/// The purpose label of every imaging frame it takes.
pub const PURPOSE: &str = "align";
/// Its exposure when the request names none: with Track off a star at the
/// celestial equator trails about 7 px of ASTAP's prepared copy in it.
pub const ALIGN_EXPOSURE_MS: f64 = 500.0;
/// Pair with the nearest finder fix no more than this many seconds from the
/// middle of the imaging exposure.
pub const PAIR_S: f64 = 5.0;
/// Every hint names the paired fix's centre, with this radius.
pub const SEARCH_RADIUS_DEG: f64 = 10.0;

/// The middle of the finder camera's frame: within this share of its width,
/// and of its height, of its centre.
pub const MIDDLE_SHARE: f64 = 0.25;
/// Done once this many frames in a row put the centre in the middle.
pub const DONE_FRAMES: u64 = 2;
/// The words of a start refused, and of a run ended, while Track is on.
const TRACK_ON: &str = "Track is on; turn it off while the finder camera's screws are turned";

/// Where the imaging camera's centre lies in the finder camera's frame.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Centre {
    /// The finder camera's pixels on the pinhole model, outside the frame
    /// when it falls outside.
    pub x: f64,
    pub y: f64,
    pub finder_width: f64,
    pub finder_height: f64,
    /// Right of the frame's centre along its rows (left when negative), and up
    /// along its columns (down when negative).
    pub right_deg: f64,
    pub up_deg: f64,
    /// The two centres' separation on the sky.
    pub separation_deg: f64,
    /// 032's own test (its FR-020): the place, distortion included, inside the frame.
    pub on_frame: bool,
    /// On the frame and within `MIDDLE_SHARE` of its width and height of its centre.
    pub middle: bool,
}

/// The imaging centre `ra_deg`, `dec_deg` in the fix's frame; `None` behind
/// the finder camera.
pub fn centre_of(fix: &FinderFix, ra_deg: f64, dec_deg: f64) -> Option<Centre> {
    let centre = UnitVector::from_radec(ra_deg.to_radians(), dec_deg.to_radians());
    let frame = &fix.frame;
    let (x, y) = frame.project_pinhole(centre)?;
    let (w, h) = frame.size();
    let focal = (w / 2.0) / (frame.fov_deg().to_radians() / 2.0).tan();
    let on_frame = frame
        .project(centre)
        .is_some_and(|place| frame.contains(place, 0.0));
    Some(Centre {
        x,
        y,
        finder_width: w,
        finder_height: h,
        right_deg: ((x - w / 2.0) / focal).atan().to_degrees(),
        up_deg: ((h / 2.0 - y) / focal).atan().to_degrees(),
        separation_deg: angular_distance(frame.center, centre).to_degrees(),
        on_frame,
        middle: on_frame
            && (x - w / 2.0).abs() <= MIDDLE_SHARE * w
            && (y - h / 2.0).abs() <= MIDDLE_SHARE * h,
    })
}

/// The body of `POST /api/imaging/align`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlignAsk {
    #[serde(default)]
    pub exposure_ms: Option<f64>,
}

/// The run's last imaging frame.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AlignLast {
    pub frame_id: u64,
    /// The middle of the exposure, console Unix seconds.
    pub at: f64,
    /// `measured`, `not_solved` or `no_live_solve`.
    pub outcome: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub centre: Option<Centre>,
    /// For a frame that did not solve: how many frames in a row, this one
    /// included, did not solve.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unsolved: Option<u64>,
    /// 032's `relation.last.outcome` for this frame.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aim: Option<String>,
    /// This frame and the `DONE_FRAMES` - 1 before it each put the centre in
    /// the middle.
    pub done: bool,
}

/// `align` in the imaging view: the latest run, running or ended.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AlignView {
    /// `running` or `ended`.
    pub state: &'static str,
    pub since: f64,
    pub exposure_ms: f64,
    pub frames: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last: Option<AlignLast>,
    /// `stopped` or `failed`, once ended.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// What the imaging state holds of the finder alignment.
#[derive(Debug, Default)]
pub(in crate::imaging) struct State {
    run: Option<Run>,
    ended: Arc<Notify>,
}

impl State {
    pub(in crate::imaging) fn abort_task(&mut self) {
        if let Some(task) = self.run.as_mut().and_then(|run| run.task.take()) {
            task.abort();
        }
    }
}

#[derive(Debug)]
struct Run {
    view: AlignView,
    task: Option<JoinHandle<()>>,
    unsolved: u64,
    middle: u64,
    stop: bool,
    previous: Option<(u64, String)>,
}

pub(in crate::imaging) fn view(held: &Held) -> Option<AlignView> {
    held.align.run.as_ref().map(|run| run.view.clone())
}

/// 031's words for a camera that is not `ready`, from `Imaging::set`.
fn state_refusal(held: &Held) -> Refusal {
    match held.state {
        "no_link" => Refusal::conflict("the scope node is not connected"),
        "no_service" => Refusal::conflict("this scope node has no imaging service"),
        "no_camera" => Refusal::conflict("this scope node has no imaging camera"),
        _ => Refusal::conflict(format!(
            "the imaging camera is not answering: {}",
            held.error.as_deref().unwrap_or_default()
        )),
    }
}

/// Read the session with no imaging lock held: its snapshot reads imaging too.
fn track_on(imaging: &Imaging) -> bool {
    let session = imaging.state.lock().expect("imaging state").session.clone();
    session.is_some_and(|session| {
        session
            .0
            .snapshot(imaging.clock.now())
            .track
            .is_some_and(|track| track.wanted)
    })
}

impl Imaging {
    /// Start the finder alignment: FR-004's refusals in order, the lease
    /// last, then a task taking and solving one imaging frame after another.
    pub async fn align_start(self: &Arc<Self>, ask: AlignAsk) -> Result<ImagingView, Refusal> {
        let track = track_on(self);
        let exposure_ms = ask.exposure_ms.unwrap_or(ALIGN_EXPOSURE_MS);
        let solver = {
            let held = self.state.lock().expect("imaging state");
            if held.state != "ready" {
                return Err(state_refusal(&held));
            }
            let Some(solver) = held.solver.clone().map(|held| held.0) else {
                return Err(Refusal::conflict("this console has no imaging solver"));
            };
            if held
                .align
                .run
                .as_ref()
                .is_some_and(|run| run.view.state == "running")
            {
                return Err(Refusal::conflict("the finder alignment is already running"));
            }
            if track {
                return Err(Refusal::conflict(TRACK_ON));
            }
            if held.relation.height_inputs() == (None, None) {
                return Err(Refusal::conflict("the imaging camera's field height is unknown: start the console with --focal-length-mm, or solve an imaging frame on the Imaging page"));
            }
            if !held
                .relation
                .fixes()
                .last()
                .is_some_and(|fix| (self.clock.now() - fix.time).abs() <= solving::HINT_AGE_S)
            {
                return Err(Refusal::conflict("the finder camera has no live solve"));
            }
            // A lease holder's frame is passed over so the lease answers with
            // its holder's name, as it does for the focus aid.
            if held.lease.is_none()
                && held
                    .frame
                    .as_ref()
                    .is_some_and(|frame| matches!(frame.state, "exposing" | "receiving"))
            {
                return Err(Refusal::conflict("an exposure is in progress"));
            }
            let camera = held.camera.as_ref().expect("a ready camera");
            if !exposure_ms.is_finite()
                || exposure_ms < camera.exposure_ms.min
                || exposure_ms > camera.exposure_ms.max
            {
                return Err(Refusal::bad_request(format!(
                    "exposure_ms {exposure_ms} is outside the camera's range {}–{} ms",
                    camera.exposure_ms.min, camera.exposure_ms.max
                )));
            }
            solver
        };
        let lease = self.lease(HOLDER)?;
        {
            let mut held = self.state.lock().expect("imaging state");
            held.align.run = Some(Run {
                view: AlignView {
                    state: "running",
                    since: self.clock.now(),
                    exposure_ms,
                    frames: 0,
                    last: None,
                    ended: None,
                    error: None,
                },
                task: None,
                unsolved: 0,
                middle: 0,
                stop: false,
                previous: None,
            });
        }
        let task = tokio::spawn(run(Arc::downgrade(self), lease, solver, exposure_ms));
        self.state
            .lock()
            .expect("imaging state")
            .align
            .run
            .as_mut()
            .expect("the run just started")
            .task = Some(task);
        Ok(self.view())
    }

    /// Stop the alignment, abort its exposure and wait for its solve, settings
    /// restoration and lease release. With nothing running, change nothing.
    pub async fn align_stop(&self) -> ImagingView {
        let signal = {
            let mut held = self.state.lock().expect("imaging state");
            match held.align.run.as_mut() {
                Some(run) if run.view.state == "running" => {
                    run.stop = true;
                    Some(held.align.ended.clone())
                }
                _ => None,
            }
        };
        if let Some(signal) = signal {
            self.abort().await;
            loop {
                let notified = signal.notified();
                if !self.align_running() {
                    break;
                }
                notified.await;
            }
        }
        self.view()
    }

    /// Whether the latest alignment run is still running.
    pub fn align_running(&self) -> bool {
        self.state
            .lock()
            .expect("imaging state")
            .align
            .run
            .as_ref()
            .is_some_and(|run| run.view.state == "running")
    }
}

async fn run(
    imaging: Weak<Imaging>,
    lease: Lease,
    solver: Arc<dyn ImagingSolver>,
    exposure_ms: f64,
) {
    let Some(imaging) = imaging.upgrade() else {
        return;
    };
    // Remember exactly the settings this procedure changes. A whole sensor
    // is restored as None so the camera applies its whole-sensor rules.
    let remembered = {
        let held = imaging.state.lock().expect("imaging state");
        held.settings.as_ref().map(|settings| {
            let whole = held.camera.as_ref().is_some_and(|camera| {
                settings.roi.x == 0
                    && settings.roi.y == 0
                    && settings.roi.width == camera.width
                    && settings.roi.height == camera.height
            });
            SettingsEdit {
                exposure_ms: Some(settings.exposure_ms),
                roi: Some((!whole).then_some(RoiEdit {
                    x: settings.roi.x,
                    y: settings.roi.y,
                    width: settings.roi.width,
                    height: settings.roi.height,
                })),
                save_at_node: Some(settings.save_at_node),
                ..SettingsEdit::default()
            }
        })
    };
    let error = if let Some(remembered) = remembered {
        let error = steps(&imaging, &lease, solver.as_ref(), exposure_ms).await;
        let restore_error = lease
            .set(remembered)
            .await
            .err()
            .map(|refusal| refusal.message);
        error.or(restore_error)
    } else {
        Some(state_refusal(&imaging.state.lock().expect("imaging state")).message)
    };
    // Release outside the lock: Lease::drop takes that lock itself.
    drop(lease);
    let mut held = imaging.state.lock().expect("imaging state");
    let run = held.align.run.as_mut().expect("the run that ended");
    run.view.state = "ended";
    run.view.ended = Some(if error.is_some() { "failed" } else { "stopped" });
    run.view.error = error;
    held.align.ended.notify_waiters();
}

fn stop_asked(imaging: &Imaging) -> bool {
    imaging
        .state
        .lock()
        .expect("imaging state")
        .align
        .run
        .as_ref()
        .is_some_and(|run| run.stop)
}

/// Set only the three settings the alignment owns, then take frames until
/// an exposure is aborted or fails. An unsuccessful solve keeps looking.
async fn steps(
    imaging: &Arc<Imaging>,
    lease: &Lease,
    solver: &dyn ImagingSolver,
    exposure_ms: f64,
) -> Option<String> {
    if let Err(refusal) = lease
        .set(SettingsEdit {
            exposure_ms: Some(exposure_ms),
            roi: Some(None),
            save_at_node: Some(false),
            ..SettingsEdit::default()
        })
        .await
    {
        return Some(refusal.message);
    }
    loop {
        if stop_asked(imaging) {
            return None;
        }
        if track_on(imaging) {
            return Some(TRACK_ON.into());
        }
        let view = match lease.take(PURPOSE).await {
            Ok(view) => view,
            Err(refusal) => return Some(refusal.message),
        };
        let id = view.frame.expect("the frame just taken").id;
        // A stop can arrive while the take request is still being answered.
        if stop_asked(imaging) {
            imaging.abort().await;
        }
        let frame = imaging.ended(id).await;
        if stop_asked(imaging) {
            return None;
        }
        match frame.state {
            "aborted" => return None,
            "failed" => return Some(frame.error.unwrap_or_default()),
            _ => {}
        }
        let file = frame.file.expect("a saved frame's file");
        let solve_frame = SolveFrame {
            path: imaging.root.join(&file),
            width: frame.width.expect("a saved frame's width"),
            height: frame.height.expect("a saved frame's height"),
            bits: frame.bits.expect("a saved frame's bits"),
            bayer: frame.bayer,
            binning: frame.binning.expect("a saved frame's binning"),
            roi: frame.roi.into(),
            start: frame.start.expect("a saved frame's start"),
            exposure_s: frame.exposure_ms / 1000.0,
        };
        let middle = solve_frame.start + solve_frame.exposure_s / 2.0;
        let (fix, previous) = {
            let mut held = imaging.state.lock().expect("imaging state");
            let solving = held
                .solve
                .as_ref()
                .filter(|solve| solve.state == "solving")
                .map(|solve| solve.frame_id);
            let run = held.align.run.as_mut().expect("the running alignment");
            run.view.frames += 1;
            // Forget a frame held by an Imaging-page solve permanently: both
            // files stay, including the sidecar that solve may still write.
            let previous = run
                .previous
                .replace((id, file.clone()))
                .filter(|(id, _)| Some(*id) != solving);
            let fix = held
                .relation
                .fixes()
                .into_iter()
                .filter(|fix| (fix.time - middle).abs() <= PAIR_S)
                .min_by(|a, b| (a.time - middle).abs().total_cmp(&(b.time - middle).abs()));
            (fix, previous)
        };
        if let Some((_, file)) = previous {
            remove(&imaging.root, &file);
        }
        let Some(fix) = fix else {
            note(imaging, id, middle, Err(None));
            continue;
        };
        // Imaging::hint calls into the session: no imaging lock is held.
        let mut hint = imaging.hint(&solve_frame);
        let (ra, dec) = fix.frame.center.to_radec();
        hint.source = HintSource::Pointing;
        hint.position = Some((ra.to_degrees(), dec.to_degrees()));
        hint.radius_deg = SEARCH_RADIUS_DEG;
        let outcome = solving::solve_frame(
            solver,
            imaging.clock.as_ref(),
            solving::Ask {
                root: &imaging.root,
                file: &file,
                frame_id: id,
                frame: solve_frame.clone(),
                hint,
                requested_at: imaging.clock.now(),
            },
        )
        .await
        .outcome;
        match outcome {
            Ok(solve) => {
                imaging.note_solve(&solve_frame, id, &solve);
                note(
                    imaging,
                    id,
                    middle,
                    Ok(centre_of(&fix, solve.ra_deg, solve.dec_deg)),
                );
            }
            Err(error) => note(imaging, id, middle, Err(Some(error.message().to_string()))),
        }
    }
}

/// Remove only this run's previous frame and its solve sidecar.
fn remove(root: &Path, file: &str) {
    let path = root.join(file);
    for path in [path.with_extension("solve.json"), path] {
        if let Err(error) = std::fs::remove_file(&path) {
            if error.kind() != std::io::ErrorKind::NotFound {
                eprintln!(
                    "finder alignment: could not remove {}: {error}",
                    path.display()
                );
            }
        }
    }
}

fn note(
    imaging: &Imaging,
    frame_id: u64,
    at: f64,
    reading: Result<Option<Centre>, Option<String>>,
) {
    let (outcome, centre, error) = match reading {
        Ok(centre) => ("measured", centre, None),
        Err(Some(error)) => ("not_solved", None, Some(error)),
        Err(None) => ("no_live_solve", None, None),
    };
    let mut held = imaging.state.lock().expect("imaging state");
    let aim = held
        .relation
        .view(None, false)
        .and_then(|view| view.last)
        .filter(|last| last.frame_id == frame_id)
        .and_then(|last| serde_json::to_value(last.outcome).ok())
        .and_then(|value| value.as_str().map(str::to_string));
    let run = held.align.run.as_mut().expect("the running alignment");
    run.unsolved = if outcome == "not_solved" {
        run.unsolved + 1
    } else {
        0
    };
    run.middle = if centre.is_some_and(|centre| centre.middle) {
        run.middle + 1
    } else {
        0
    };
    run.view.last = Some(AlignLast {
        frame_id,
        at,
        outcome,
        error,
        centre,
        unsolved: (outcome == "not_solved").then_some(run.unsolved),
        aim,
        done: run.middle >= DONE_FRAMES,
    });
}
