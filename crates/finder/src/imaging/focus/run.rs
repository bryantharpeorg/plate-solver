//! The focus aid and the focus check on the console (033 US5, US8): the focus
//! state the imaging state holds, the task that runs the aid under 031's
//! lease, and `focus` in the imaging view. US9 adds the tile read for the
//! time being and a frame kept for a solve.

use std::{
    path::{Path, PathBuf},
    sync::{Arc, Weak},
    time::Duration,
};

use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use tokio::{sync::Notify, task::JoinHandle};

use super::{
    aid::{Aid, Sample},
    judge, FocusReading, Kept, KeptRegion, Mark, Of, Read, Source, Tile, Unmeasurable, Verdict,
};
use crate::imaging::{
    frames::FrameView,
    relation::CameraId,
    solver::{ImagingSolver, Region, SolveFrame},
    CameraView, Held, Imaging, ImagingView, Lease, Refusal, RoiEdit, RoiView, SettingsEdit,
};

/// The purpose label of every imaging frame the aid takes (FR-020).
pub const PURPOSE: &str = "focus";
/// The name the aid holds the imaging camera's lease under (FR-021).
pub const HOLDER: &str = "the focus aid";
/// The focus exposure when the request names none (FR-021).
pub const FOCUS_EXPOSURE_MS: f64 = 1000.0;
/// A focus check ends after this many imaging frames at the latest (FR-026).
pub const CHECK_FRAMES: u64 = 9;
/// A put-back the scope node refuses while the camera is `ready` is tried
/// again after this long (054), as the scope node waits after its own aborts.
pub const RESTORE_PAUSE: Duration = Duration::from_secs(1);
/// No put-back is tried later than this after the first (054).
pub const RESTORE_WAIT: Duration = Duration::from_secs(5);
/// The words of a search that found no tile to read (FR-022).
const NO_REGION: &str = "no tile of the imaging frame had five stars to measure";

/// The body of `POST /api/imaging/focus` (FR-020, FR-026). Non-exhaustive: US8
/// adds `check`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct FocusAsk {
    /// With `check` the aid runs a focus check, which stops by itself with a
    /// verdict and notes nothing in the record (FR-026).
    #[serde(default)]
    pub check: bool,
    #[serde(default)]
    pub find: bool,
    #[serde(default)]
    pub tile: Option<TileAsk>,
    #[serde(default)]
    pub exposure_ms: Option<f64>,
}

/// A tile fixed by hand in the body of `POST /api/imaging/focus` (FR-020).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TileAsk {
    pub col: u32,
    pub row: u32,
}

/// `focus` in the imaging view (FR-029). Every size is in `unit`. Every
/// `Option` is skipped while `None`. Non-exhaustive: US8 adds `check`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct FocusView {
    /// `arcsec` when the console knows the scale, else `px`: sensor pixels.
    pub unit: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arcsec_per_px: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<RegionOut>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub record: Option<RecordOut>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aid: Option<AidView>,
}

/// The focus region in the view (FR-029): a tile's rectangle in sensor pixels.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct RegionOut {
    pub col: u32,
    pub row: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    /// `found` or `fixed` for the kept focus region; `provisional` for a tile
    /// a running aid reads for the time being (US9).
    pub source: &'static str,
}

/// The focus record in the view (FR-029).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct RecordOut {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tonight: Option<SizeAt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub earlier: Option<SizeAt>,
}

/// A size and when it was read (FR-029).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct SizeAt {
    pub value: f64,
    /// Console Unix seconds.
    pub at: f64,
}

/// The aid's part of the focus view (FR-029): the latest run, running or ended,
/// absent before the first start of this console's run. Non-exhaustive: US8
/// adds `check`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct AidView {
    /// `starting`, `finding`, `running` or `ended`.
    pub state: &'static str,
    /// Whether this run is a focus check (FR-026).
    pub check: bool,
    /// Console Unix seconds at the start.
    pub since: f64,
    pub exposure_ms: f64,
    /// Samples so far.
    pub readings: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cadence_ms: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last: Option<LastView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub median: Option<MedianView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best: Option<f64>,
    /// `better`, `worse` or `same`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trend: Option<&'static str>,
    pub found: bool,
    /// `good`, `slightly_soft`, `clearly_out`, `unmeasurable` or `no_record`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verdict: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<ReferenceView>,
    /// The last 120 samples, oldest first; `null` for a frame with too few
    /// stars (FR-019).
    pub history: Vec<Option<f64>>,
    /// `stopped`, `done`, `failed` or `no_region` (FR-026, FR-027).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_reason: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restore_error: Option<String>,
}

/// The last sample in the view (FR-025): whatever the frame was.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct LastView {
    pub frame_id: u64,
    /// The middle of the exposure, console Unix seconds.
    pub at: f64,
    /// `reading`, `settling`, `too_few_stars` or `far_out`.
    pub state: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stars: Option<u32>,
}

/// The median of the readings held in the view (FR-025).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct MedianView {
    pub value: f64,
    pub readings: usize,
}

/// What a reading was judged against (FR-025).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct ReferenceView {
    pub value: f64,
    /// `tonight` or `earlier`.
    pub of: &'static str,
}

/// The focus state the imaging state holds: what the console keeps about focus
/// for one camera — the `Kept` of `focus.json` — the latest run and the signal
/// its end wakes. Its fields are private; `Imaging::view` reads them through
/// [`view`].
#[derive(Debug)]
pub(in crate::imaging) struct State {
    kept: Kept,
    run: Option<Run>,
    /// Woken once the run's lifecycle is `ended`, after the lease is dropped.
    ended: Arc<Notify>,
    /// The region an aid could not take back, and its put-back's words (054):
    /// kept here, not with the run, so a later run does not lose it.
    left: Option<(RoiView, String)>,
}

impl State {
    /// Nothing known yet; the `Kept` of `path`, nothing read.
    pub(in crate::imaging) fn new(path: PathBuf) -> Self {
        Self {
            kept: Kept::new(path),
            run: None,
            ended: Arc::new(Notify::new()),
            left: None,
        }
    }

    /// The camera was described: the `Kept` reads `focus.json` for it (FR-029),
    /// and a region an aid left is no longer named (054).
    pub(in crate::imaging) fn describe(&mut self, camera: CameraId) {
        self.kept.describe(camera);
        self.left = None;
    }

    /// The observer changed the settings (054): a region an aid left is now theirs.
    pub(in crate::imaging) fn forget_left(&mut self) {
        self.left = None;
    }

    /// The words of the region an aid could not take back, while that region is
    /// still the one in force (054).
    fn left_in(&self, roi: RoiView) -> Option<&str> {
        self.left
            .as_ref()
            .filter(|(left, _)| *left == roi)
            .map(|(_, words)| words.as_str())
    }

    /// End the run's task, for `Drop` (the run's lifecycle stays as it is).
    pub(in crate::imaging) fn abort_task(&mut self) {
        if let Some(task) = self.run.as_mut().and_then(|run| run.task.take()) {
            task.abort();
        }
    }

    /// Whether an aid or a check is running (FR-029).
    fn running(&self) -> bool {
        self.run.as_ref().is_some_and(Run::running)
    }

    /// The signal the run's end wakes, to make a `Notified` of before the
    /// lifecycle is read.
    fn end_signal(&self) -> Arc<Notify> {
        self.ended.clone()
    }

    fn begin(&mut self, run: Run) {
        self.run = Some(run);
    }

    fn set_task(&mut self, task: JoinHandle<()>) {
        if let Some(run) = self.run.as_mut() {
            run.task = Some(task);
        }
    }

    /// Mark the stop; answers whether a run was running to be stopped.
    fn mark_stop(&mut self) -> bool {
        match self.run.as_mut().filter(|run| run.running()) {
            Some(run) => {
                run.stop = true;
                true
            }
            None => false,
        }
    }
}

/// One run of the aid, starting until ended. Everything the task reports lives
/// here, under the imaging state's lock; nothing is awaited while it is held.
#[derive(Debug)]
struct Run {
    /// `starting`, `finding`, `running` or `ended`.
    lifecycle: &'static str,
    /// Whether this run is a focus check (FR-026).
    check: bool,
    /// Console Unix seconds at the start.
    since: f64,
    exposure_ms: f64,
    aid: Aid,
    /// A stop was asked for (FR-027).
    stop: bool,
    /// The run's task; taken by `abort_task`.
    task: Option<JoinHandle<()>>,
    /// When the last sample was pushed, console Unix seconds.
    sampled_at: Option<f64>,
    cadence_ms: Option<f64>,
    last: Option<LastView>,
    verdict: Option<Verdict>,
    /// The reference the verdict was judged against, as it stood before the
    /// reading was noted (FR-025).
    reference: Option<(f64, Of)>,
    ended_reason: Option<&'static str>,
    error: Option<String>,
    restore_error: Option<String>,
    /// The last frame saved, whose file goes when the next is saved (FR-024).
    previous: Option<(u64, Option<String>)>,
    /// The tile the run reads for the time being, while no tile could be
    /// measured and the far-out one with the smallest size stands in, kept as
    /// no region (FR-037); `None` once a region is kept. The view shows it as
    /// the region while the run is not `ended`.
    provisional: Option<Tile>,
}

impl Run {
    fn new(since: f64, exposure_ms: f64, check: bool) -> Self {
        Self {
            lifecycle: "starting",
            check,
            since,
            exposure_ms,
            aid: Aid::new(),
            stop: false,
            task: None,
            sampled_at: None,
            cadence_ms: None,
            last: None,
            verdict: None,
            reference: None,
            ended_reason: None,
            error: None,
            restore_error: None,
            previous: None,
            provisional: None,
        }
    }

    fn running(&self) -> bool {
        matches!(self.lifecycle, "starting" | "finding" | "running")
    }
}

/// The words of each state that is not `ready`, as 031's own `set` refuses
/// them (FR-020); written out here because 031's are private to its modules.
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

/// The settings of one turn of the aid, or of the settings put back: exactly
/// the four keys the aid owns (FR-021).
fn edit(exposure_ms: f64, binning: u32, roi: Option<RoiEdit>, save_at_node: bool) -> SettingsEdit {
    SettingsEdit {
        exposure_ms: Some(exposure_ms),
        binning: Some(binning),
        roi: Some(roi),
        save_at_node: Some(save_at_node),
        ..SettingsEdit::default()
    }
}

/// A file's place under the imaging folder, split for the platform's
/// separators.
fn place(root: &Path, relative: &str) -> PathBuf {
    let mut path = root.to_path_buf();
    for part in relative.split('/') {
        path.push(part);
    }
    path
}

impl Imaging {
    /// Start the focus aid (FR-020, FR-021): the eight refusals in order, then
    /// the lease asked for as [`HOLDER`], a new run in the focus state and the
    /// task that runs it. A refused start has changed nothing.
    pub async fn focus_start(self: &Arc<Self>, ask: FocusAsk) -> Result<ImagingView, Refusal> {
        // Checks 1 to 7 under one acquisition of the state's lock, from its own
        // fields: `Imaging::solver`, `holder` and the view take the lock
        // themselves and are not called while it is held.
        let tile;
        let (solver, camera) = {
            let held = self.state.lock().expect("imaging state");
            // 1. The camera's state, with 031's words (FR-020).
            if held.state != "ready" {
                return Err(state_refusal(held.state, held.error.as_deref()));
            }
            // 2. No solver (FR-020).
            let Some(solver) = held.solver.clone().map(|held| held.0) else {
                return Err(Refusal::conflict("this console has no imaging solver"));
            };
            // 3. An aid is running (FR-020).
            if held.focus.running() {
                return Err(Refusal::conflict("the focus aid is already running"));
            }
            // 4. A frame in flight that no lease guards (FR-020): while a
            //    lease is out this is passed over, whatever the frame's state,
            //    and check 8 answers with the holder's name.
            if held.lease.is_none()
                && held
                    .frame
                    .as_ref()
                    .is_some_and(|frame| matches!(frame.state, "exposing" | "receiving"))
            {
                return Err(Refusal::conflict("an exposure is in progress"));
            }
            // 5. Both (FR-020).
            if ask.find && ask.tile.is_some() {
                return Err(Refusal::bad_request("give find or tile, not both"));
            }
            // 6. A tile the grid does not hold (FR-020).
            tile = match ask.tile {
                Some(tile) => match Tile::new(tile.col, tile.row) {
                    Some(tile) => Some(tile),
                    None => {
                        return Err(Refusal::bad_request(
                            "tile col and row must each be 0, 1 or 2",
                        ))
                    }
                },
                None => None,
            };
            // 7. The exposure against the camera's range, with 031's words
            //    (FR-020).
            if let Some(exposure_ms) = ask.exposure_ms {
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
            }
            (solver, held.camera.clone().expect("a described camera"))
        };
        // 8. The lease, asked for last so a refused start never held the
        //    camera (FR-021); it also keeps two starts apart.
        let lease = self.lease(HOLDER)?;
        let config = Config {
            solver,
            camera,
            check: ask.check,
            find: ask.find,
            tile,
            exposure_ms: ask.exposure_ms.unwrap_or(FOCUS_EXPOSURE_MS),
        };
        {
            let mut held = self.state.lock().expect("imaging state");
            held.focus
                .begin(Run::new(self.clock.now(), config.exposure_ms, config.check));
        }
        let task = tokio::spawn(run(Arc::downgrade(self), lease, config, self.root.clone()));
        self.state
            .lock()
            .expect("imaging state")
            .focus
            .set_task(task);
        Ok(self.view())
    }

    /// Stop the aid and wait until it has ended, so the camera is free and the
    /// settings are back (FR-028). With nothing running: the view, changed
    /// nothing.
    pub async fn focus_stop(&self) -> ImagingView {
        let running = self.state.lock().expect("imaging state").focus.mark_stop();
        if running {
            // The exposure in flight is the aid's own; ending it lets the task
            // see the stop within one frame.
            self.abort().await;
            // The `Notified` is made first and the lifecycle read after, so an
            // end between the two cannot be missed.
            let signal = self.state.lock().expect("imaging state").focus.end_signal();
            loop {
                let notified = signal.notified();
                if !self.focus_running() {
                    break;
                }
                notified.await;
            }
        }
        self.view()
    }

    /// Whether an aid or a check is running, whatever the camera's state
    /// (FR-029). It keeps nobody out: the lease does that.
    pub fn focus_running(&self) -> bool {
        self.state.lock().expect("imaging state").focus.running()
    }

    /// The focus view once no aid or check is running (FR-026), so the camera
    /// is free and the settings are back; `None` when the view has no
    /// `focus`. With nothing running: that view at once.
    pub async fn focus_ended(&self) -> Option<FocusView> {
        // The `Notified` is made first and the lifecycle read after, so an
        // end between the two cannot be missed.
        let signal = self.state.lock().expect("imaging state").focus.end_signal();
        loop {
            let notified = signal.notified();
            if !self.focus_running() {
                break;
            }
            notified.await;
        }
        self.view().focus
    }
}

/// What the task needs beside the `Imaging` it serves and the lease it owns.
struct Config {
    solver: Arc<dyn ImagingSolver>,
    camera: CameraView,
    /// Whether this run is a focus check (FR-026).
    check: bool,
    find: bool,
    tile: Option<Tile>,
    exposure_ms: f64,
}

/// The settings the aid remembered and puts back (FR-021, FR-027). `roi: None`
/// is the whole sensor.
#[derive(Debug, Clone)]
struct Remembered {
    exposure_ms: f64,
    binning: u32,
    save_at_node: bool,
    roi: Option<RoiEdit>,
}

impl Remembered {
    fn edit(&self) -> SettingsEdit {
        edit(self.exposure_ms, self.binning, self.roi, self.save_at_node)
    }
}

/// How the run ended (FR-027), and what is put back first.
struct Ended {
    reason: &'static str,
    /// The words, for every reason but `stopped`.
    error: Option<String>,
    restore: Option<Remembered>,
}

impl Ended {
    fn failed(error: String) -> Self {
        Self {
            reason: "failed",
            error: Some(error),
            restore: None,
        }
    }

    fn stopped() -> Self {
        Self {
            reason: "stopped",
            error: None,
            restore: None,
        }
    }

    /// A check ended by itself (FR-026).
    fn done() -> Self {
        Self {
            reason: "done",
            error: None,
            restore: None,
        }
    }
}

/// The task that runs the aid (FR-021 to FR-027): it owns the lease from its
/// first line to its last, and holds the `Imaging` weakly, so dropping the last
/// `Arc` ends it.
async fn run(imaging: Weak<Imaging>, lease: Lease, config: Config, root: PathBuf) {
    let Some(imaging) = imaging.upgrade() else {
        return;
    };
    let end = steps(&imaging, &lease, &config, &root).await;
    finish(&imaging, lease, end).await;
}

/// The task's five steps: remember, choose the region, set, one frame after
/// another, end (FR-021 to FR-027).
async fn steps(imaging: &Arc<Imaging>, lease: &Lease, config: &Config, root: &Path) -> Ended {
    // 1. Remember the settings in force (FR-021). A camera that left `ready`
    //    after the start's checks ends the run at once: nothing is remembered,
    //    so nothing is put back.
    let Some(remembered) = remember(imaging) else {
        return state_ended(imaging);
    };
    let sensor = Region {
        x: 0,
        y: 0,
        width: config.camera.width,
        height: config.camera.height,
    };
    let b0 = *config.camera.binnings.iter().min().unwrap_or(&1);
    // 2. Choose the region (FR-021): the tile asked for, kept as `fixed`; else
    //    a search, with `find` or with nothing kept; else the kept one. A
    //    search that no tile could be measured leaves the aid on the far-out
    //    tile with the smallest size, read for the time being and kept as no
    //    region (FR-037).
    let kept = imaging
        .state
        .lock()
        .expect("imaging state")
        .focus
        .kept
        .region()
        .map(|region| region.tile());
    let (mut tile, mut provisional) = match config.tile {
        Some(tile) => {
            keep_tile(
                imaging,
                KeptRegion::fixed(sensor, b0, tile, imaging.clock.now()),
            );
            (tile, false)
        }
        None => match (!config.find, kept) {
            (true, Some(tile)) => (tile, false),
            _ => match search(imaging, lease, config, root, b0).await {
                Ok(Found::Best(best, reading)) => {
                    keep_tile(imaging, KeptRegion::found(sensor, b0, best, &reading));
                    (best, false)
                }
                Ok(Found::FarOut(far_out)) => (far_out, true),
                Ok(Found::None) => {
                    return Ended {
                        reason: "no_region",
                        error: Some(NO_REGION.to_string()),
                        restore: Some(remembered),
                    }
                }
                Err(end) => return with(end, remembered),
            },
        },
    };
    set_provisional(imaging, provisional.then_some(tile));
    // 3. Set the focus exposure, the smallest binning, that region and no node
    //    copy (FR-021). A failed set ends the run with the refusal's words.
    if let Err(refusal) = set_region(lease, config.exposure_ms, sensor, b0, tile).await {
        return with(Ended::failed(refusal.message), remembered);
    }
    set_lifecycle(imaging, "running");
    // 4. One frame, repeated (FR-023 to FR-025).
    loop {
        // a. A stop asked for ends the run (FR-027).
        if stop_asked(imaging) {
            return with(Ended::stopped(), remembered);
        }
        let frame = match one_frame(imaging, lease, root).await {
            Ok(frame) => frame,
            Err(end) => return with(end, remembered),
        };
        // e. The saved frame as the solver needs it, and the reading of the
        //    whole of it (FR-023). The scale is read again at each use, since a
        //    solve can end while the aid runs.
        let (Some(width), Some(height), Some(bits), Some(binning), Some(start), Some(file)) = (
            frame.width,
            frame.height,
            frame.bits,
            frame.binning,
            frame.start,
            frame.file.clone(),
        ) else {
            return with(
                Ended::failed("the imaging frame was not saved".into()),
                remembered,
            );
        };
        let solve_frame = SolveFrame {
            path: place(root, &file),
            width,
            height,
            bits,
            bayer: frame.bayer.clone(),
            binning,
            roi: Region::from(frame.roi),
            start,
            exposure_s: frame.exposure_ms / 1000.0,
        };
        let read = match super::read(
            config.solver.as_ref(),
            solve_frame,
            frame.id,
            None,
            imaging.scale_arcsec_per_px(),
        )
        .await
        {
            Ok(read) => read,
            Err(error) => return with(Ended::failed(error.message()), remembered),
        };
        // f to h: one critical section, no await inside it. `at` is the middle
        // of the exposure.
        let settled = note(imaging, &read, frame.id, start + frame.exposure_ms / 2000.0);
        // i. A check ends by itself (FR-026): at its first settled reading,
        //    with that reading's verdict; at its first unmeasurable frame,
        //    with that frame's verdict; or after `CHECK_FRAMES` imaging frames
        //    with neither, with `unmeasurable`. It never reaches step j.
        if config.check && check_ends(imaging, settled.is_some(), read.is_err()) {
            return with(Ended::done(), remembered);
        }
        // j. While the run's tile is one read for the time being, each settled
        //    reading starts the search again (FR-037). A best tile is kept
        //    and the aid goes on there; without one, whether or not that
        //    search found a far-out tile, the aid goes on with the tile it had
        //    and never ends `no_region`. The `Aid` is not reset: the readings
        //    held carry over, and a reading on the new tile that differs by
        //    more than 15% is a change like any other.
        if provisional && settled.is_some() {
            match search(imaging, lease, config, root, b0).await {
                Ok(Found::Best(best, reading)) => {
                    keep_tile(imaging, KeptRegion::found(sensor, b0, best, &reading));
                    tile = best;
                    provisional = false;
                    set_provisional(imaging, None);
                }
                Ok(_) => {}
                Err(end) => return with(end, remembered),
            }
            if let Err(refusal) = set_region(lease, config.exposure_ms, sensor, b0, tile).await {
                return with(Ended::failed(refusal.message), remembered);
            }
            set_lifecycle(imaging, "running");
        }
    }
}

/// Give the ending its remembered settings (step 1's), whatever made it.
fn with(mut end: Ended, remembered: Remembered) -> Ended {
    if end.restore.is_none() {
        end.restore = Some(remembered);
    }
    end
}

/// The settings in force, remembered for the end (FR-021); `None` when the
/// camera left `ready`. A region an earlier aid could not put back is taken as
/// the whole sensor (054): an aid never keeps another aid's leftovers.
fn remember(imaging: &Arc<Imaging>) -> Option<Remembered> {
    let held = imaging.state.lock().expect("imaging state");
    let settings = held.settings.clone()?;
    let whole = held.focus.left_in(settings.roi).is_some()
        || held.camera.as_ref().is_some_and(|camera| {
            settings.roi.x == 0
                && settings.roi.y == 0
                && settings.roi.width == camera.width
                && settings.roi.height == camera.height
        });
    Some(Remembered {
        exposure_ms: settings.exposure_ms,
        binning: settings.binning,
        save_at_node: settings.save_at_node,
        roi: (!whole).then_some(RoiEdit {
            x: settings.roi.x,
            y: settings.roi.y,
            width: settings.roi.width,
            height: settings.roi.height,
        }),
    })
}

/// The ending for a camera that is not `ready` (FR-027): 031's words, nothing
/// put back.
fn state_ended(imaging: &Arc<Imaging>) -> Ended {
    let held = imaging.state.lock().expect("imaging state");
    Ended::failed(state_refusal(held.state, held.error.as_deref()).message)
}

/// Keep a region and write `focus.json` (FR-021).
fn keep_tile(imaging: &Arc<Imaging>, region: KeptRegion) {
    imaging
        .state
        .lock()
        .expect("imaging state")
        .focus
        .kept
        .keep_region(region);
}

/// The stop mark, read by the task at each step's turn.
fn stop_asked(imaging: &Arc<Imaging>) -> bool {
    imaging
        .state
        .lock()
        .expect("imaging state")
        .focus
        .run
        .as_ref()
        .is_some_and(|run| run.stop)
}

fn set_lifecycle(imaging: &Arc<Imaging>, lifecycle: &'static str) {
    let mut held = imaging.state.lock().expect("imaging state");
    if let Some(run) = held.focus.run.as_mut() {
        run.lifecycle = lifecycle;
    }
}

/// What a search found (FR-022, FR-037): the sharpest tile with its reading;
/// or, with no tile measurable, the far-out tile with the smallest size,
/// which the aid reads for the time being; or no tile and none far out.
enum Found {
    Best(Tile, FocusReading),
    FarOut(Tile),
    None,
}

/// The search (FR-022): one whole-sensor frame, whose nine tiles are measured,
/// and the sharpest tile with its reading — or, with no tile measurable, the
/// far-out tile with the smallest size, to read for the time being (FR-037).
async fn search(
    imaging: &Arc<Imaging>,
    lease: &Lease,
    config: &Config,
    root: &Path,
    b0: u32,
) -> Result<Found, Ended> {
    set_lifecycle(imaging, "finding");
    // The whole sensor for the search's own frame (FR-022).
    if let Err(refusal) = lease.set(edit(config.exposure_ms, b0, None, false)).await {
        return Err(Ended::failed(refusal.message));
    }
    let frame = one_frame(imaging, lease, root).await?;
    let (Some(width), Some(height), Some(bits), Some(binning), Some(start), Some(file)) = (
        frame.width,
        frame.height,
        frame.bits,
        frame.binning,
        frame.start,
        frame.file.clone(),
    ) else {
        return Err(Ended::failed("the imaging frame was not saved".into()));
    };
    let solve_frame = SolveFrame {
        path: place(root, &file),
        width,
        height,
        bits,
        bayer: frame.bayer.clone(),
        binning,
        roi: Region::from(frame.roi),
        start,
        exposure_s: frame.exposure_ms / 1000.0,
    };
    let found = match super::find_region(
        config.solver.as_ref(),
        solve_frame,
        frame.id,
        imaging.scale_arcsec_per_px(),
    )
    .await
    {
        Ok(found) => found,
        Err(error) => return Err(Ended::failed(error.message())),
    };
    Ok(
        match found.best.and_then(|best| {
            found
                .tiles
                .iter()
                .find(|tile| tile.tile == best)
                .and_then(|tile| tile.read.ok())
                .map(|reading| (best, reading))
        }) {
            Some((best, reading)) => Found::Best(best, reading),
            None => match found.far_out {
                Some(tile) => Found::FarOut(tile),
                None => Found::None,
            },
        },
    )
}

/// Set the focus exposure, the smallest binning, `tile`'s region and no node
/// copy (FR-021), the region worked out again from the tile so a region kept
/// for another binning cannot be refused.
async fn set_region(
    lease: &Lease,
    exposure_ms: f64,
    sensor: Region,
    b0: u32,
    tile: Tile,
) -> Result<(), Refusal> {
    let region = super::tile_region(sensor, b0, tile);
    lease
        .set(edit(
            exposure_ms,
            b0,
            Some(RoiEdit {
                x: region.x,
                y: region.y,
                width: region.width,
                height: region.height,
            }),
            false,
        ))
        .await
        .map(|_| ())
}

/// Note in the run whether its tile is one read for the time being (FR-037),
/// which the view shows as the region while the run is not `ended`.
fn set_provisional(imaging: &Arc<Imaging>, tile: Option<Tile>) {
    let mut held = imaging.state.lock().expect("imaging state");
    if let Some(run) = held.focus.run.as_mut() {
        run.provisional = tile;
    }
}

/// Steps 4a to 4d: one frame through the lease, awaited until it ended, the
/// previous saved frame's file and sidecar removed (FR-024) and this one kept
/// as the previous. Answers the saved frame.
async fn one_frame(imaging: &Arc<Imaging>, lease: &Lease, root: &Path) -> Result<FrameView, Ended> {
    // b. Take (FR-023). The frame's id is the answered view's `frame.id`.
    let view = match lease.take(PURPOSE).await {
        Ok(view) => view,
        Err(refusal) => return Err(Ended::failed(refusal.message)),
    };
    let Some(frame) = view.frame else {
        // The camera left `ready` between the take and its answer.
        return Err(state_ended(imaging));
    };
    let id = frame.id;
    let frame = imaging.ended(id).await;
    // c. A stop or an abort ends the run `stopped`; a failed frame ends it
    //    `failed` with the frame's error (FR-027).
    if stop_asked(imaging) || frame.state == "aborted" {
        return Err(Ended::stopped());
    }
    if frame.state == "failed" {
        return Err(Ended::failed(frame.error.unwrap_or_default()));
    }
    // The previous saved frame goes now that this one is saved (FR-024), unless
    // an imaging solve is running on it: then it stays in place for good, and
    // so does the solve sidecar its solve will write; the aid forgets it and
    // never comes back to it (FR-039).
    let (previous, solving) = {
        let mut guard = imaging.state.lock().expect("imaging state");
        let held: &mut Held = &mut guard;
        let solving = held
            .solve
            .as_ref()
            .filter(|solve| solve.state == "solving")
            .map(|solve| solve.frame_id);
        let run = held.focus.run.as_mut().expect("the run");
        let previous = run.previous.replace((id, frame.file.clone()));
        let solving = previous
            .as_ref()
            .is_some_and(|(previous, _)| Some(*previous) == solving);
        (previous, solving)
    };
    if !solving {
        remove(root, previous);
    }
    Ok(frame)
}

/// Remove a frame's file and its solve sidecar (FR-024), ignoring a missing one
/// and printing any other failure to standard error.
fn remove(root: &Path, previous: Option<(u64, Option<String>)>) {
    let Some((_, Some(file))) = previous else {
        return;
    };
    if let Err(error) = std::fs::remove_file(place(root, &file)) {
        if error.kind() != std::io::ErrorKind::NotFound {
            eprintln!("focus: could not remove {}: {error}", file);
        }
    }
    let Some(base) = file.strip_suffix(".fits") else {
        return;
    };
    let sidecar = format!("{base}.solve.json");
    if let Err(error) = std::fs::remove_file(place(root, &sidecar)) {
        if error.kind() != std::io::ErrorKind::NotFound {
            eprintln!("focus: could not remove {}: {error}", sidecar);
        }
    }
}

/// Steps 4f to 4h (FR-023, FR-025): the sample pushed, the cadence and the
/// last sample set, the reference read before the verdict is set, the verdict
/// kept with it, and a settled reading noted. One critical section; no await
/// falls inside it. Answers the settled reading, if this sample made one.
fn note(imaging: &Arc<Imaging>, read: &Read, id: u64, at: f64) -> Option<f64> {
    let mut guard = imaging.state.lock().expect("imaging state");
    let held: &mut Held = &mut guard;
    let now = imaging.clock.now();
    let run = held.focus.run.as_mut().expect("the run");
    // f. The sample, the cadence and the last sample.
    let step = run.aid.push(Sample::from(read));
    let settled = step.settled;
    run.cadence_ms = run.sampled_at.map(|at| (now - at) * 1000.0);
    run.sampled_at = Some(now);
    let state = run
        .aid
        .phase()
        .map(|phase| phase.as_str())
        .unwrap_or("reading");
    let (value, stars) = match read {
        Ok(reading) => (Some(reading.px), Some(reading.stars)),
        Err(Unmeasurable::TooFewStars { stars }) => (None, Some(*stars)),
        Err(Unmeasurable::FarOut { px, stars }) => (Some(*px), Some(*stars)),
    };
    run.last = Some(LastView {
        frame_id: id,
        at,
        state,
        value,
        stars,
    });
    // g. The reference as it stood before the reading is noted, so a settled
    //    reading is never judged against itself. Only a measured scale reaches
    //    the record (FR-025).
    let measured = held
        .relation
        .view(None, false)
        .and_then(|view| view.scale)
        .map(|scale| scale.arcsec_per_px);
    let reference = held.focus.kept.reference(now, measured);
    match (settled, read) {
        (Some(m), _) => {
            let found = run.aid.found();
            run.verdict = Some(judge(Ok(m), reference.map(|(px, _)| px)));
            run.reference = reference;
            // h. Noted with the aid's `found` of this moment — but never by a
            //    check, which is judged against the record and changes
            //    nothing (FR-025, FR-026).
            if !run.check {
                held.focus.kept.note(m, at, measured, found);
            }
        }
        (None, Err(unmeasurable)) => {
            run.verdict = Some(judge(Err(*unmeasurable), reference.map(|(px, _)| px)));
            run.reference = reference;
        }
        // A measurable frame that did not settle: no verdict and no reference
        // (FR-025).
        _ => {
            run.verdict = None;
            run.reference = None;
        }
    }
    settled
}

/// Step 4i (FR-026): whether a check ends at this frame — at once at its
/// first settled reading or unmeasurable frame, whose verdict the sample
/// already set, and at the count of imaging frames reaching `CHECK_FRAMES`,
/// where nothing settled and nothing was unmeasurable, so the verdict becomes
/// `unmeasurable`.
fn check_ends(imaging: &Arc<Imaging>, settled: bool, unmeasurable: bool) -> bool {
    let mut held = imaging.state.lock().expect("imaging state");
    let run = held.focus.run.as_mut().expect("the run");
    if settled || unmeasurable {
        return true;
    }
    if run.aid.readings() < CHECK_FRAMES {
        return false;
    }
    run.verdict = Some(Verdict::Unmeasurable);
    true
}

impl Imaging {
    /// The words of a put-back that failed, while the region the aid could not
    /// take back is still in force (054); 041's region refusal names it.
    pub(in crate::imaging) fn focus_left(&self) -> Option<String> {
        let held = self.state.lock().expect("imaging state");
        let roi = held.settings.as_ref()?.roi;
        held.focus.left_in(roi).map(str::to_string)
    }
}

/// Put the remembered settings back through the lease (FR-027, 054): the scope
/// node's refusal (502) while the camera is `ready` is followed by another try
/// `RESTORE_PAUSE` later, so long as that try starts within `RESTORE_WAIT` of
/// the first; any other refusal is the last at once. Answers the last refusal's
/// words, or none once a try takes. No lock is held across the pause.
async fn put_back(
    imaging: &Arc<Imaging>,
    lease: &Lease,
    remembered: &Remembered,
) -> Option<String> {
    let first = tokio::time::Instant::now();
    loop {
        let refusal = match lease.set(remembered.edit()).await {
            Ok(_) => return None,
            Err(refusal) => refusal,
        };
        let ready = imaging.state.lock().expect("imaging state").state == "ready";
        if !ready
            || refusal.status != StatusCode::BAD_GATEWAY
            || first.elapsed() + RESTORE_PAUSE > RESTORE_WAIT
        {
            return Some(refusal.message);
        }
        tokio::time::sleep(RESTORE_PAUSE).await;
    }
}

/// Step 5 (FR-027): wait out a frame in flight, put the remembered settings
/// back through the lease, drop the lease and only then let the lifecycle say
/// `ended`, so a reader who sees it finds the camera free and the settings as
/// they will stay. A put-back that fails leaves the region in force named as
/// the aid's (054).
async fn finish(imaging: &Arc<Imaging>, lease: Lease, end: Ended) {
    if let Some(id) = {
        let held = imaging.state.lock().expect("imaging state");
        held.frame
            .as_ref()
            .filter(|frame| matches!(frame.state, "exposing" | "receiving"))
            .map(|frame| frame.id)
    } {
        imaging.ended(id).await;
    }
    let remembered = end.restore.is_some();
    let restore_error = match end.restore {
        Some(settings) => put_back(imaging, &lease, &settings).await,
        None => None,
    };
    drop(lease);
    {
        let mut guard = imaging.state.lock().expect("imaging state");
        let held: &mut Held = &mut guard;
        if remembered {
            let roi = held.settings.as_ref().map(|settings| settings.roi);
            held.focus.left = match (restore_error.as_ref(), roi) {
                (Some(words), Some(roi)) => Some((roi, words.clone())),
                _ => None,
            };
        }
        let run = held.focus.run.as_mut().expect("the run");
        run.lifecycle = "ended";
        run.ended_reason = Some(end.reason);
        run.error = end.error;
        run.restore_error = restore_error;
        held.focus.ended.notify_waiters();
    }
}

/// The focus view of one moment (FR-029), from the focus state and the two
/// scales: `shown`, which sizes what the view shows, and `measured`, which the
/// record keeps. Only `Imaging::view` calls it, while it holds the state's
/// lock.
pub(in crate::imaging) fn view(held: &Held, now: f64) -> FocusView {
    let shown = held.relation.scale_arcsec_per_px();
    let s = shown.unwrap_or(1.0);
    let measured = held
        .relation
        .view(None, false)
        .and_then(|view| view.scale)
        .map(|scale| scale.arcsec_per_px);
    let record = held.focus.kept.record(now, measured);
    let sized = |mark: Mark| SizeAt {
        value: mark.px * s,
        at: mark.at,
    };
    // The region is the tile a running aid reads for the time being, shown as
    // `provisional`, in place of the kept one; without one, the kept region
    // with its own source; with neither, nothing (FR-029, FR-037).
    let region = match (
        held.focus
            .run
            .as_ref()
            .filter(|run| run.running())
            .and_then(|run| run.provisional),
        held.camera.as_ref(),
    ) {
        (Some(tile), Some(camera)) => {
            let sensor = Region {
                x: 0,
                y: 0,
                width: camera.width,
                height: camera.height,
            };
            let b0 = *camera.binnings.iter().min().unwrap_or(&1);
            let rect = super::tile_region(sensor, b0, tile);
            Some(RegionOut {
                col: tile.col,
                row: tile.row,
                x: rect.x,
                y: rect.y,
                width: rect.width,
                height: rect.height,
                source: "provisional",
            })
        }
        _ => held.focus.kept.region().map(|region| RegionOut {
            col: region.col,
            row: region.row,
            x: region.x,
            y: region.y,
            width: region.width,
            height: region.height,
            source: match region.source {
                Source::Found => "found",
                Source::Fixed => "fixed",
            },
        }),
    };
    FocusView {
        unit: if shown.is_some() { "arcsec" } else { "px" },
        arcsec_per_px: shown,
        region,
        record: (record.tonight.is_some() || record.earlier.is_some()).then_some(RecordOut {
            tonight: record.tonight.map(sized),
            earlier: record.earlier.map(sized),
        }),
        aid: held.focus.run.as_ref().map(|run| AidView {
            state: run.lifecycle,
            check: run.check,
            since: run.since,
            exposure_ms: run.exposure_ms,
            readings: run.aid.readings(),
            cadence_ms: run.cadence_ms,
            last: run.last.map(|last| LastView {
                value: last.value.map(|value| value * s),
                ..last
            }),
            median: run.aid.median().map(|median| MedianView {
                value: median.px * s,
                readings: median.readings,
            }),
            best: run.aid.best().map(|best| best * s),
            trend: run.aid.trend().map(|trend| trend.as_str()),
            found: run.aid.found(),
            verdict: run.verdict.map(|verdict| verdict.as_str()),
            reference: run.reference.map(|(px, of)| ReferenceView {
                value: px * s,
                of: match of {
                    Of::Tonight => "tonight",
                    Of::Earlier => "earlier",
                },
            }),
            history: run
                .aid
                .history()
                .into_iter()
                .map(|value| value.map(|value| value * s))
                .collect(),
            ended_reason: run.ended_reason,
            error: run.error.clone(),
            restore_error: run.restore_error.clone(),
        }),
    }
}
