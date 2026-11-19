//! The live stack as the console runs it (035): subs taken back to back,
//! solved, gated, calibrated and added until stopped. Registration comes only from
//! imaging solves; this service knows nothing of the mount or the site.

use std::{
    collections::VecDeque,
    path::Path,
    sync::{Arc, Mutex},
    time::Instant,
};

use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use tokio::sync::{watch, Notify};

use super::{Added, Colour, Placement, Stack, SubSettings};
use crate::{
    imaging::{
        calibration::{library::FrameKey, service::Calibration, Mosaic},
        fits, focus,
        frames::FrameView,
        solver::{
            HintSource, ImagingSolve, ImagingSolver, Region, SolveError, SolveFrame, SolveHint,
        },
        solving::{self, Ask},
        Imaging, ImagingView, Lease, Refusal,
    },
    session::Clock,
};

/// The name a stack holds the imaging camera's lease under.
pub const HOLDER: &str = "the stack";
pub const FAILED_IN_A_ROW: u32 = 3;
pub const QUEUE_SUBS: usize = 4;
pub const FOCUS_GATE_MIN: usize = 5;
pub const FOCUS_GATE_EXCESS: f64 = 0.3;
pub const REHINT_AFTER: u32 = 3;

/// Absent is the default, or unchanged on a running stack.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartRequest {
    #[serde(default, deserialize_with = "super::super::present_value")]
    pub reject: Option<bool>,
    #[serde(default, deserialize_with = "super::super::present_value")]
    pub focus_gate: Option<bool>,
    #[serde(default, deserialize_with = "super::super::present_value")]
    pub gains: Option<[f64; 3]>,
    #[serde(default, deserialize_with = "super::super::present_value")]
    pub stretch: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OptionsView {
    pub reject: bool,
    pub focus_gate: bool,
    pub gains: [f64; 3],
    pub stretch: f64,
}

/// Subs not stacked, by reason. Every key is always written, in this order.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Dropped {
    pub not_solved: u32,
    pub too_few_stars: u32,
    pub solver_failed: u32,
    pub soft: u32,
    pub off_field: u32,
    pub solve_differs: u32,
    pub settings: u32,
    pub frame_failed: u32,
    pub unreadable: u32,
    pub behind: u32,
    pub unprocessed: u32,
}

impl Dropped {
    pub fn total(&self) -> u32 {
        self.not_solved
            + self.too_few_stars
            + self.solver_failed
            + self.soft
            + self.off_field
            + self.solve_differs
            + self.settings
            + self.frame_failed
            + self.unreadable
            + self.behind
            + self.unprocessed
    }

    fn count(&mut self, outcome: &str) {
        let count = match outcome {
            "not_solved" => &mut self.not_solved,
            "too_few_stars" => &mut self.too_few_stars,
            "solver_failed" => &mut self.solver_failed,
            "soft" => &mut self.soft,
            "off_field" => &mut self.off_field,
            "solve_differs" => &mut self.solve_differs,
            "settings" => &mut self.settings,
            "unreadable" => &mut self.unreadable,
            "unprocessed" => &mut self.unprocessed,
            _ => unreachable!("an outcome supplied by the worker"),
        };
        *count += 1;
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ReferenceView {
    pub frame_id: u64,
    pub width: u32,
    pub height: u32,
    pub ra_deg: f64,
    pub dec_deg: f64,
    pub up_east_of_north_deg: f64,
    pub arcsec_per_px: f64,
    pub exposure_ms: f64,
}

/// The last sub that reached the solver.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LastView {
    pub frame_id: u64,
    /// `stacked`, or the key of Dropped it was counted under.
    pub outcome: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_deg: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shift_px: Option<[f64; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rejected_px: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focus_arcsec: Option<f64>,
    /// Real milliseconds from taking the sub off the queue to its outcome.
    pub took_ms: f64,
}

/// The latest stack, running or ended. Later stories add display and files.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct StackView {
    /// `stacking` or `ended`.
    pub state: &'static str,
    pub id: String,
    pub started_at: f64,
    pub options: OptionsView,
    pub taken: u32,
    pub stacked: u32,
    pub queued: u32,
    pub dropped: Dropped,
    pub exposure_s: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<ReferenceView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last: Option<LastView>,
    /// What calibration found absent for the last sub calibrated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub missing: Option<Vec<&'static str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focus_median_arcsec: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_reason: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

struct Ending {
    reason: &'static str,
    error: Option<String>,
}

struct Run {
    view: StackView,
    /// Saved subs, oldest first. Kept with the counts so queued is exact.
    waiting: VecDeque<FrameView>,
    stopping: watch::Sender<bool>,
    finished: watch::Sender<bool>,
    ending: Option<Ending>,
}

/// Counts and pixels have separate locks: a view never waits for a pixel pass.
/// Neither lock is held across an await or a call into Imaging.
pub struct Stacker {
    imaging: Arc<Imaging>,
    calibration: Arc<Calibration>,
    clock: Arc<dyn Clock>,
    counts: Mutex<Option<Run>>,
    pixels: Arc<Mutex<Option<Stack>>>,
}

impl Stacker {
    pub fn new(
        imaging: Arc<Imaging>,
        calibration: Arc<Calibration>,
        clock: Arc<dyn Clock>,
    ) -> Arc<Self> {
        Arc::new(Self {
            imaging,
            calibration,
            clock,
            counts: Mutex::new(None),
            pixels: Arc::new(Mutex::new(None)),
        })
    }

    /// Start a stack, or change the gains and stretch of the one running.
    pub async fn start(
        self: &Arc<Self>,
        request: StartRequest,
        target: Option<&str>,
    ) -> Result<StackView, Refusal> {
        if request.gains.is_some_and(|g| {
            g.iter()
                .any(|v| !v.is_finite() || !(0.1..=10.0).contains(v))
        }) {
            return Err(Refusal::bad_request(
                "gains must be three numbers from 0.1 to 10",
            ));
        }
        if request
            .stretch
            .is_some_and(|v| !v.is_finite() || !(1.0..=1000.0).contains(&v))
        {
            return Err(Refusal::bad_request(
                "stretch must be a number from 1 to 1000",
            ));
        }
        {
            let mut counts = self.counts.lock().expect("stack counts");
            if let Some(run) = counts.as_mut().filter(|run| run.view.state == "stacking") {
                if request.reject.is_some() || request.focus_gate.is_some() {
                    return Err(Refusal::conflict(
                        "the stack is running: stop it to change reject or focus_gate",
                    ));
                }
                if let Some(gains) = request.gains {
                    run.view.options.gains = gains;
                }
                if let Some(stretch) = request.stretch {
                    run.view.options.stretch = stretch;
                }
                return Ok(run.view.clone());
            }
        }
        let imaging = self.imaging.view();
        if imaging.state != "ready" {
            return Err(Refusal::conflict(state_words(&imaging)));
        }
        let solver = self
            .imaging
            .solver()
            .ok_or_else(|| Refusal::conflict("this console has no imaging solver"))?;
        if self.imaging.holder().is_none()
            && imaging
                .frame
                .as_ref()
                .is_some_and(|f| matches!(f.state, "exposing" | "receiving"))
        {
            return Err(Refusal::conflict("an exposure is in progress"));
        }
        // The lease alone excludes simultaneous starts and other camera users.
        let lease = self.imaging.lease(HOLDER)?;
        let started_at = self.clock.now();
        let (_, utc) = fits::utc(started_at);
        let mut id = format!("{}Z", utc[..19].replace(':', "-"));
        if let Some(target) = target {
            id.push('-');
            id.push_str(target);
        }
        let root = self.imaging.root().to_path_buf();
        let folder = tokio::task::spawn_blocking(move || make_folder(&root, &id))
            .await
            .expect("the stack folder maker cannot be cancelled");
        let id = match folder {
            Ok(id) => id,
            Err(error) => {
                drop(lease);
                return Err(Refusal {
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                    message: format!("cannot make the stack folder: {error}"),
                });
            }
        };
        lease.save_in(&format!("runs/{id}/subs"));
        let view = StackView {
            state: "stacking",
            id,
            started_at,
            options: OptionsView {
                reject: request.reject.unwrap_or(true),
                focus_gate: request.focus_gate.unwrap_or(true),
                gains: request.gains.unwrap_or([1.0; 3]),
                stretch: request.stretch.unwrap_or(10.0),
            },
            taken: 0,
            stacked: 0,
            queued: 0,
            dropped: Dropped::default(),
            exposure_s: 0.0,
            reference: None,
            last: None,
            missing: None,
            focus_median_arcsec: None,
            ended_at: None,
            ended_reason: None,
            error: None,
        };
        let (stopping, _) = watch::channel(false);
        let (finished, _) = watch::channel(false);
        let taker_stop = stopping.subscribe();
        let worker_stop = stopping.subscribe();
        *self.pixels.lock().expect("stack pixels") = None;
        *self.counts.lock().expect("stack counts") = Some(Run {
            view: view.clone(),
            waiting: VecDeque::with_capacity(QUEUE_SUBS),
            stopping,
            finished,
            ending: None,
        });
        let waiting = Arc::new(Notify::new());
        let notify = waiting.clone();
        let stacker = self.clone();
        let taker = tokio::spawn(async move {
            stacker.take_subs(lease, notify, taker_stop).await;
        });
        let stacker = self.clone();
        let worker = tokio::spawn(async move {
            Worker {
                stacker,
                solver,
                stopping: worker_stop,
                reference: None,
                reference_settings: None,
                last_solve: None,
                misses: 0,
                focus_readings: Vec::new(),
            }
            .run(waiting)
            .await
        });
        let stacker = self.clone();
        // Only this task finalizes the stack. Awaiting the taker releases the
        // camera before the worker's last work and before the ended view.
        tokio::spawn(async move {
            taker.await.expect("the stack taker");
            worker.await.expect("the stack worker");
            let ended_at = stacker.clock.now();
            let mut counts = stacker.counts.lock().expect("stack counts");
            let run = counts.as_mut().expect("the ending stack");
            run.view.dropped.unprocessed += run.waiting.len() as u32;
            run.waiting.clear();
            run.view.queued = 0;
            let ending = run.ending.take().expect("a requested ending");
            run.view.ended_at = Some(ended_at);
            run.view.ended_reason = Some(ending.reason);
            run.view.error = ending.error;
            run.view.state = "ended";
            run.finished.send_replace(true);
        });
        Ok(view)
    }

    /// Stop the running stack and answer once its taker and worker have ended.
    /// None before the first start; otherwise the latest view when already ended.
    pub async fn stop(&self) -> Option<StackView> {
        let mut finished = {
            let counts = self.counts.lock().expect("stack counts");
            let run = counts.as_ref()?;
            if run.view.state == "ended" {
                return Some(run.view.clone());
            }
            run.finished.subscribe()
        };
        if self.end("stopped", None) {
            self.imaging.abort().await;
        }
        let _ = finished.wait_for(|ended| *ended).await;
        self.view()
    }

    pub fn view(&self) -> Option<StackView> {
        self.counts
            .lock()
            .expect("stack counts")
            .as_ref()
            .map(|run| run.view.clone())
    }

    /// The latest stack's mean; None before a sub is stacked.
    pub fn mean(&self) -> Option<Colour> {
        self.pixels
            .lock()
            .expect("stack pixels")
            .as_ref()
            .map(Stack::mean)
    }

    /// First requester chooses the reason. Both tasks observe the same mark.
    fn end(&self, reason: &'static str, error: Option<String>) -> bool {
        let mut counts = self.counts.lock().expect("stack counts");
        let run = counts.as_mut().expect("a started stack");
        if run.view.state == "ended" || run.ending.is_some() {
            return false;
        }
        run.ending = Some(Ending { reason, error });
        run.stopping.send_replace(true);
        true
    }

    async fn take_subs(&self, lease: Lease, waiting: Arc<Notify>, stopping: watch::Receiver<bool>) {
        let mut failed = 0;
        loop {
            if *stopping.borrow() {
                return;
            }
            // The gap before an exposure: 036 can wait here under this lease.
            let view = match lease.take("sub").await {
                Ok(view) => view,
                Err(refusal) => {
                    self.end("camera", Some(refusal.message));
                    return;
                }
            };
            let Some(frame) = view.frame else {
                self.end("camera", Some(state_words(&view)));
                return;
            };
            // Stop's abort may have run just before this take created its frame.
            if *stopping.borrow() {
                self.imaging.abort().await;
            }
            let frame = self.imaging.ended(frame.id).await;
            match frame.state {
                "saved" => {
                    failed = 0;
                    {
                        let mut counts = self.counts.lock().expect("stack counts");
                        let run = counts.as_mut().expect("the taking stack");
                        run.view.taken += 1;
                        if run.waiting.len() == QUEUE_SUBS {
                            run.waiting.pop_front();
                            run.view.dropped.behind += 1;
                        }
                        run.waiting.push_back(frame);
                        run.view.queued = run.waiting.len() as u32;
                    }
                    waiting.notify_one();
                }
                "failed" => {
                    {
                        let mut counts = self.counts.lock().expect("stack counts");
                        let view = &mut counts.as_mut().expect("the taking stack").view;
                        view.taken += 1;
                        view.dropped.frame_failed += 1;
                    }
                    failed += 1;
                    if failed == FAILED_IN_A_ROW {
                        self.end("camera", frame.error);
                        return;
                    }
                }
                "aborted" => {
                    self.end("stopped", None);
                    return;
                }
                _ => unreachable!("Imaging::ended answers an ended frame"),
            }
        }
    }

    async fn next_sub(&self, waiting: &Notify) -> FrameView {
        loop {
            let sub = {
                let mut counts = self.counts.lock().expect("stack counts");
                let run = counts.as_mut().expect("the working stack");
                let sub = run.waiting.pop_front();
                run.view.queued = run.waiting.len() as u32;
                sub
            };
            if let Some(sub) = sub {
                return sub;
            }
            // notify_one retains a permit if a save arrives before this await.
            waiting.notified().await;
        }
    }
}

fn state_words(view: &ImagingView) -> String {
    match view.state {
        "no_link" => "the scope node is not connected".to_string(),
        "no_service" => "this scope node has no imaging service".to_string(),
        "no_camera" => "this scope node has no imaging camera".to_string(),
        _ => format!(
            "the imaging camera is not answering: {}",
            view.error.as_deref().unwrap_or_default()
        ),
    }
}

/// Reserve one folder atomically; another start in the same second takes -2.
fn make_folder(root: &Path, id: &str) -> std::io::Result<String> {
    let runs = root.join("runs");
    std::fs::create_dir_all(&runs)?;
    let mut suffix = 1_u64;
    loop {
        let name = if suffix == 1 {
            id.to_string()
        } else {
            format!("{id}-{suffix}")
        };
        let folder = runs.join(&name);
        match std::fs::create_dir(&folder) {
            Ok(()) => {
                std::fs::create_dir(folder.join("subs"))?;
                return Ok(name);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => suffix += 1,
            Err(error) => return Err(error),
        }
    }
}

struct Worker {
    stacker: Arc<Stacker>,
    solver: Arc<dyn ImagingSolver>,
    stopping: watch::Receiver<bool>,
    reference: Option<ImagingSolve>,
    reference_settings: Option<SubSettings>,
    last_solve: Option<ImagingSolve>,
    misses: u32,
    /// Only accepted subs join this history.
    focus_readings: Vec<f64>,
}

struct Accepted {
    frame: SolveFrame,
    solve: ImagingSolve,
    missing: Vec<&'static str>,
    reference: Option<ReferenceView>,
}

impl Worker {
    async fn run(mut self, waiting: Arc<Notify>) {
        loop {
            let sub = tokio::select! {
                biased;
                _ = self.stopping.wait_for(|stopping| *stopping) => break,
                sub = self.stacker.next_sub(&waiting) => sub,
            };
            let began = Instant::now();
            let mut last = None;
            let result = self.process(&sub, &mut last).await;
            let outcome = match &result {
                Ok(_) => "stacked",
                Err(reason) => *reason,
            };
            if let Some(last) = &mut last {
                last.outcome = outcome;
                last.took_ms = began.elapsed().as_secs_f64() * 1000.0;
            }
            {
                let mut counts = self.stacker.counts.lock().expect("stack counts");
                let view = &mut counts.as_mut().expect("the working stack").view;
                match &result {
                    Ok(accepted) => {
                        view.stacked += 1;
                        view.exposure_s += sub.exposure_ms / 1000.0;
                        view.missing = Some(accepted.missing.clone());
                        view.focus_median_arcsec = median(&self.focus_readings);
                        if let Some(reference) = &accepted.reference {
                            view.reference = Some(reference.clone());
                        }
                    }
                    Err(reason) => view.dropped.count(reason),
                }
                if last.is_some() {
                    view.last = last;
                }
            }
            if let Ok(accepted) = result {
                // Only an accepted sub measures the aim point; page solve and
                // kept scale are untouched. No stack lock is held here.
                self.stacker
                    .imaging
                    .note_solve(&accepted.frame, sub.id, &accepted.solve);
            }
        }
    }

    async fn process(
        &mut self,
        sub: &FrameView,
        last: &mut Option<LastView>,
    ) -> Result<Accepted, &'static str> {
        let (Some(width), Some(height), Some(bits), Some(binning), Some(start), Some(file)) = (
            sub.width,
            sub.height,
            sub.bits,
            sub.binning,
            sub.start,
            sub.file.as_deref(),
        ) else {
            return Err("unreadable");
        };
        let frame = SolveFrame {
            path: self.stacker.imaging.root().join(file),
            width,
            height,
            bits,
            bayer: sub.bayer.clone(),
            binning,
            roi: sub.roi.into(),
            start,
            exposure_s: sub.exposure_ms / 1000.0,
        };
        let settings = SubSettings {
            width,
            height,
            bits,
            bayer: sub.bayer.clone(),
            binning,
            roi: sub.roi.into(),
            gain: sub.gain,
            offset: sub.offset,
            exposure_ms: sub.exposure_ms,
        };
        if self
            .reference_settings
            .as_ref()
            .is_some_and(|reference| !super::same_settings(reference, &settings))
        {
            return Err("settings");
        }
        *last = Some(LastView {
            frame_id: sub.id,
            outcome: "unprocessed",
            turn_deg: None,
            shift_px: None,
            rejected_px: None,
            focus_arcsec: None,
            took_ms: 0.0,
        });
        let mut hint = self.stacker.imaging.hint(&frame);
        if hint.source != HintSource::Pointing
            && self.reference.is_some()
            && self.misses < REHINT_AFTER
        {
            if let Some(solve) = &self.last_solve {
                let scale = solve.arcsec_per_px / 3600.0;
                hint = SolveHint {
                    source: HintSource::LastSolve,
                    position: Some((solve.ra_deg, solve.dec_deg)),
                    radius_deg: f64::from(width).hypot(f64::from(height)) * scale,
                    fov_deg: Some(f64::from(height) * scale),
                };
            }
        }
        let ask = Ask {
            root: self.stacker.imaging.root(),
            file,
            frame_id: sub.id,
            frame: frame.clone(),
            hint,
            requested_at: self.stacker.clock.now(),
        };
        let ended = tokio::select! {
            biased;
            _ = self.stopping.wait_for(|stopping| *stopping) => return Err("unprocessed"),
            ended = solving::solve_frame(self.solver.as_ref(), self.stacker.clock.as_ref(), ask) => ended,
        };
        // Misses change only here: a focus failure after a solve is no miss.
        let solve = match ended.outcome {
            Ok(solve) => {
                self.misses = 0;
                solve
            }
            Err(SolveError::NotSolved { .. }) => {
                self.misses += 1;
                return Err("not_solved");
            }
            Err(SolveError::TooFewStars { .. }) => {
                self.misses += 1;
                return Err("too_few_stars");
            }
            Err(_) => return Err("solver_failed"),
        };
        self.last_solve = Some(solve.clone());
        let options = self.stacker.view().expect("a working stack").options;
        let focus_arcsec = if options.focus_gate {
            let region = self
                .stacker
                .imaging
                .view()
                .focus
                .and_then(|focus| focus.region)
                .and_then(|region| {
                    let roi = frame.roi;
                    (region.x >= roi.x
                        && region.y >= roi.y
                        && u64::from(region.x) + u64::from(region.width)
                            <= u64::from(roi.x) + u64::from(roi.width)
                        && u64::from(region.y) + u64::from(region.height)
                            <= u64::from(roi.y) + u64::from(roi.height))
                    .then(|| Region {
                        x: (region.x - roi.x) / binning,
                        y: (region.y - roi.y) / binning,
                        width: region.width / binning,
                        height: region.height / binning,
                    })
                });
            let read = tokio::select! {
                biased;
                _ = self.stopping.wait_for(|stopping| *stopping) => return Err("unprocessed"),
                read = focus::read(self.solver.as_ref(), frame.clone(), sub.id, region, Some(solve.arcsec_per_px / f64::from(binning))) => read,
            };
            let reading = match read {
                Err(_) => return Err("solver_failed"),
                Ok(Err(focus::Unmeasurable::TooFewStars { .. })) => return Err("too_few_stars"),
                Ok(Err(focus::Unmeasurable::FarOut { .. })) => return Err("soft"),
                Ok(Ok(reading)) => reading.arcsec.expect("the sub's solve supplies a scale"),
            };
            last.as_mut()
                .expect("this sub reached the solve")
                .focus_arcsec = Some(reading);
            if self.focus_readings.len() >= FOCUS_GATE_MIN
                && reading
                    > (1.0 + FOCUS_GATE_EXCESS)
                        * median(&self.focus_readings).expect("five readings")
            {
                return Err("soft");
            }
            Some(reading)
        } else {
            None
        };
        let placement = match &self.reference {
            Some(reference) => {
                let placement = Placement::between(reference, &solve).ok_or("off_field")?;
                super::judge(
                    &placement,
                    (reference.width, reference.height),
                    (width, height),
                )?;
                placement
            }
            None => Placement::identity(),
        };
        let reject = options.reject;
        let angle = self
            .stacker
            .imaging
            .view()
            .relation
            .and_then(|r| r.angle_deg);
        let calibration = self.stacker.calibration.clone();
        let pixels = self.stacker.pixels.clone();
        let path = frame.path.clone();
        let (added, missing) = tokio::task::spawn_blocking(
            move || -> Result<(Added, Vec<&'static str>), &'static str> {
                let bytes = std::fs::read(path).map_err(|_| "unreadable")?;
                let image = fits::decode(&bytes).map_err(|_| "unreadable")?;
                drop(bytes);
                let key = FrameKey::from_image(&image).map_err(|_| "unreadable")?;
                let mut mosaic = Mosaic::from_samples(
                    &image.samples,
                    image.width,
                    image.height,
                    image.bits,
                    key.bayer.as_deref(),
                )
                .ok_or("unreadable")?;
                drop(image);
                let applied = calibration
                    .library()
                    .calibrate(&key, &mut mosaic, None, angle);
                let colour = super::colour(&mosaic);
                let mut pixels = pixels.lock().expect("stack pixels");
                let stack = pixels.get_or_insert_with(|| {
                    let stack = Stack::new(colour.width, colour.height).with_background_match();
                    if reject {
                        stack.with_rejection()
                    } else {
                        stack
                    }
                });
                Ok((stack.add(&colour, &placement), applied.selection.missing))
            },
        )
        .await
        .expect("the stack pixel pass cannot be cancelled")?;
        let reference = if self.reference.is_none() {
            self.reference = Some(solve.clone());
            self.reference_settings = Some(settings);
            Some(ReferenceView {
                frame_id: sub.id,
                width,
                height,
                ra_deg: solve.ra_deg,
                dec_deg: solve.dec_deg,
                up_east_of_north_deg: solve.up_east_of_north_deg,
                arcsec_per_px: solve.arcsec_per_px,
                exposure_ms: sub.exposure_ms,
            })
        } else {
            None
        };
        let reference_size = self
            .reference
            .as_ref()
            .map(|r| (r.width, r.height))
            .expect("the accepted reference sub");
        let last = last.as_mut().expect("this sub reached the solve");
        last.turn_deg = Some(placement.turn_deg());
        last.shift_px = placement
            .centre_shift(reference_size, (width, height))
            .map(|(x, y)| [x, y]);
        last.rejected_px = Some(added.rejected);
        if let Some(reading) = focus_arcsec {
            self.focus_readings.push(reading);
        }
        Ok(Accepted {
            frame,
            solve,
            missing,
            reference,
        })
    }
}

fn median(readings: &[f64]) -> Option<f64> {
    if readings.is_empty() {
        return None;
    }
    let mut sorted = readings.to_vec();
    sorted.sort_by(f64::total_cmp);
    Some(sorted[(sorted.len() - 1) / 2])
}
