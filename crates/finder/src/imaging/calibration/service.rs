//! The calibration library as the console holds it, and the sets it takes (034).

use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

use super::{
    bad_pixel_map, levels,
    library::{FrameKey, Kind, Library, MasterEntry, Selection, SET_TEMP_TOLERANCE_C},
    lit_samples, normalise_flat, percentile, Mosaic, NOISE_FLOOR,
};
use crate::{
    imaging::{fits, focus::run::PURPOSE, Imaging, ImagingView, Lease, Refusal, SettingsEdit},
    session::Clock,
};

/// The smallest and largest number of darks a set may average.
pub const MIN_SET_FRAMES: u32 = 3;
pub const MAX_SET_FRAMES: u32 = 64;
/// A dark may read this many offset-frame noises above the offset level.
pub const DARK_RISE_SIGMAS: f64 = 2.0;
/// An offset level above this share of full scale means the camera is uncovered.
pub const COVER_SHARE: f64 = 0.2;
/// Allowed share of bright samples that are not hot pixels, with a minimum.
pub const LIT_SHARE: f64 = 0.0001;
pub const LIT_MIN: u64 = 8;
/// The name under which a set holds the imaging camera's lease.
pub const HOLDER: &str = "a set of darks or flats";
pub const FLAT_TARGET: f64 = 0.45;
pub const FLAT_LOW: f64 = 0.40;
pub const FLAT_HIGH: f64 = 0.50;
pub const FLAT_SATURATED: f64 = 0.90;
pub const FLAT_SEARCH_FRAMES: u32 = 8;
pub const FLAT_MAX_EXPOSURE_MS: f64 = 30_000.0;

/// The latest set, retained until the console exits.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct SetView {
    pub id: String,
    pub kind: &'static str,
    pub state: &'static str,
    pub wanted: u32,
    pub taken: u32,
    pub exposure_ms: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset_level: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level_share: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub darks_taken: Option<u32>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub not_dark: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// The body of `POST /api/imaging/calibration/darks`.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DarksRequest {
    pub count: Option<u32>,
    pub anyway: Option<bool>,
}

/// The two steps of `POST /api/imaging/calibration/flats`.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlatsRequest {
    pub count: Option<u32>,
    pub label: Option<String>,
    pub darks: Option<bool>,
    pub anyway: Option<bool>,
}

struct Set {
    view: SetView,
    stop: Arc<AtomicBool>,
    flats: Option<Arc<FlatFrames>>,
}

struct Run {
    id: String,
    count: u32,
    exposure_ms: f64,
    shortest_ms: f64,
    anyway: bool,
    stop: Arc<AtomicBool>,
}

struct FrameSum {
    sum: Vec<u32>,
    key: FrameKey,
    temperature: Option<f64>,
}

struct FlatFrames {
    frames: FrameSum,
    exposure_ms: f64,
    level_share: f64,
    label: String,
}

/// Failures from the camera/read path have a dark's prefix. Set-wide checks
/// (settings, temperature and darkness) carry their own words instead.
enum SetEnd {
    Stopped,
    Failed { message: String, not_dark: bool },
}

impl SetEnd {
    fn failed(message: impl Into<String>) -> Self {
        Self::Failed {
            message: message.into(),
            not_dark: false,
        }
    }
    fn dark(message: impl std::fmt::Display) -> Self {
        Self::frame("dark", message)
    }
    fn frame(purpose: &str, message: impl std::fmt::Display) -> Self {
        Self::failed(format!("a {purpose} failed: {message}"))
    }
    fn not_dark(message: String) -> Self {
        Self::Failed {
            message,
            not_dark: true,
        }
    }
}

struct SetFrame {
    key: FrameKey,
    mosaic: Mosaic,
}

/// `GET /api/imaging/calibration`, and `calibration` in the imaging view.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct CalibrationView {
    pub masters: Vec<MasterEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(rename = "match", skip_serializing_if = "Option::is_none")]
    pub matched: Option<Selection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub set: Option<SetView>,
}

impl CalibrationView {
    /// `{"masters":[]}`: a console that takes no imaging frames.
    pub fn empty() -> Self {
        Self {
            masters: Vec::new(),
            error: None,
            matched: None,
            set: None,
        }
    }
}

/// The library and the latest eligible saved imaging frame seen in this run.
pub struct Calibration {
    imaging: Arc<Imaging>,
    library: Library,
    matched: Mutex<Option<(u64, FrameKey)>>,
    clock: Arc<dyn Clock>,
    set: Mutex<Option<Set>>,
}

impl Calibration {
    /// Open `calibration/` under the imaging folder, with the console's clock.
    pub fn new(imaging: Arc<Imaging>, root: PathBuf, clock: Arc<dyn Clock>) -> Arc<Self> {
        Arc::new(Self {
            imaging,
            library: Library::open(root),
            matched: Mutex::new(None),
            clock,
            set: Mutex::new(None),
        })
    }

    pub fn library(&self) -> &Library {
        &self.library
    }

    /// Match the last saved imaging frame seen whose purpose is neither dark,
    /// flat nor focus. Later exposures and those purposes leave its key intact.
    /// The current relation's angle judges a camera turn, even for a kept key.
    pub fn view(&self, imaging: Option<&ImagingView>) -> CalibrationView {
        let matched = {
            let mut held = self.matched.lock().expect("calibration match");
            if let Some(view) = imaging.filter(|view| view.state == "ready") {
                if let Some(frame) = view.frame.as_ref().filter(|frame| {
                    frame.state == "saved"
                        && !matches!(frame.purpose.as_str(), "dark" | "flat" | PURPOSE)
                        && held.as_ref().is_none_or(|(id, _)| frame.id > *id)
                }) {
                    if let Some(key) = FrameKey::from_view(view) {
                        *held = Some((frame.id, key));
                    }
                }
            }
            held.clone()
        };
        let angle = imaging
            .and_then(|view| view.relation.as_ref())
            .and_then(|relation| relation.angle_deg);
        CalibrationView {
            set: self
                .set
                .lock()
                .expect("latest set")
                .as_ref()
                .map(|set| set.view.clone()),
            masters: self.library.masters(),
            error: self.library.error(),
            matched: matched.map(|(id, key)| {
                let mut selection = self.library.select(&key, None, angle);
                selection.frame = Some(id);
                selection
            }),
        }
    }

    /// Remove a master and its files. Ill-formed ids never reach the disk.
    pub async fn remove(self: &Arc<Self>, id: &str) -> Result<CalibrationView, Refusal> {
        let dropped = {
            let mut held = self.set.lock().expect("latest set");
            if let Some(set) = held
                .as_mut()
                .filter(|set| set.view.id == id && set.view.state == "needs_darks")
            {
                set.flats = None;
                set.view.state = "stopped";
                true
            } else {
                false
            }
        };
        if dropped {
            return Ok(self.current_view());
        }
        let stop = {
            let held = self.set.lock().expect("latest set");
            held.as_ref()
                .filter(|set| set.view.id == id && running(set.view.state))
                .map(|set| set.stop.clone())
        };
        if let Some(stop) = stop {
            stop.store(true, Ordering::Release);
            self.imaging.abort().await;
            return Ok(self.current_view());
        }
        let missing = || Refusal {
            status: StatusCode::NOT_FOUND,
            message: format!("no master or set has the id {id}"),
        };
        if !master_id(id) {
            return Err(missing());
        }
        let calibration = self.clone();
        let owned_id = id.to_string();
        let removed = tokio::task::spawn_blocking(move || calibration.library.remove(&owned_id))
            .await
            .map_err(|error| removal_error(error.to_string()))?
            .map_err(|error| removal_error(error.to_string()))?;
        if !removed {
            return Err(missing());
        }
        Ok(self
            .imaging
            .view()
            .calibration
            .unwrap_or_else(|| self.view(None)))
    }

    fn current_view(&self) -> CalibrationView {
        self.imaging
            .view()
            .calibration
            .unwrap_or_else(|| self.view(None))
    }

    fn update_set(&self, edit: impl FnOnce(&mut SetView)) {
        if let Some(set) = self.set.lock().expect("latest set").as_mut() {
            edit(&mut set.view);
        }
    }

    /// Judge all checks without awaiting, taking the camera's own lease last.
    pub async fn take_darks(
        self: &Arc<Self>,
        request: DarksRequest,
    ) -> Result<CalibrationView, Refusal> {
        let count = set_count(request.count)?;
        let (view, lease) = self.start_step(false)?;
        let exposure_ms = view.settings.as_ref().expect("ready settings").exposure_ms;
        let shortest_ms = view.camera.as_ref().expect("ready camera").exposure_ms.min;
        let id = self.library.new_id(Kind::Dark, self.clock.now());
        let stop = Arc::new(AtomicBool::new(false));
        let set = SetView {
            id: id.clone(),
            kind: "dark",
            state: "offset",
            wanted: count,
            taken: 0,
            exposure_ms,
            label: None,
            offset_level: None,
            level: None,
            level_share: None,
            darks_taken: None,
            not_dark: false,
            error: None,
        };
        *self.set.lock().expect("latest set") = Some(Set {
            view: set,
            stop: stop.clone(),
            flats: None,
        });
        // Snapshot the accepted offset state before another runtime worker can
        // advance the task. The lease already belongs to this accepted request.
        let answer = self.current_view();
        let calibration = self.clone();
        tokio::spawn(async move {
            let run = Run {
                id,
                count,
                exposure_ms,
                shortest_ms,
                anyway: request.anyway == Some(true),
                stop,
            };
            let result = calibration.run_darks(&lease, &run).await;
            calibration.end_step(lease, exposure_ms).await;
            calibration.update_set(|set| match result {
                Ok(()) => set.state = "done",
                Err(SetEnd::Stopped) => set.state = "stopped",
                Err(SetEnd::Failed { message, not_dark }) => {
                    set.state = "failed";
                    set.error = Some(message);
                    set.not_dark = not_dark;
                }
            });
        });
        Ok(answer)
    }

    /// Camera state, running set, waiting flats, single exposure, then lease.
    /// Never hold our mutex while calling Imaging (its view calls us back).
    fn start_step(&self, flat_darks: bool) -> Result<(ImagingView, Lease), Refusal> {
        let view = self.imaging.view();
        if view.state != "ready" {
            return Err(state_refusal(&view));
        }
        let (taking, waiting) = {
            let held = self.set.lock().expect("latest set");
            (
                held.as_ref().is_some_and(|set| running(set.view.state)),
                held.as_ref()
                    .is_some_and(|set| set.view.kind == "flat" && set.view.state == "needs_darks"),
            )
        };
        if taking {
            return Err(Refusal::conflict(
                "a set of darks or flats is already being taken",
            ));
        }
        if flat_darks && !waiting {
            return Err(Refusal::conflict("no flats are waiting for their darks"));
        }
        if !flat_darks && waiting {
            return Err(Refusal::conflict(
                "flats are waiting for their darks: take those or drop them first",
            ));
        }
        if self.imaging.holder().is_none()
            && view
                .frame
                .as_ref()
                .is_some_and(|frame| matches!(frame.state, "exposing" | "receiving"))
        {
            return Err(Refusal::conflict("an exposure is in progress"));
        }
        let lease = self.imaging.lease(HOLDER)?;
        Ok((view, lease))
    }

    /// No set_frame path leaves an exposure in flight. Restore and release
    /// before any final state is visible, including needs_darks on a retry.
    async fn end_step(&self, lease: Lease, exposure_ms: f64) {
        let _ = lease
            .set(SettingsEdit {
                exposure_ms: Some(exposure_ms),
                ..Default::default()
            })
            .await;
        drop(lease);
    }

    /// Start the exposure search and flats, or finish waiting flats with darks.
    pub async fn take_flats(
        self: &Arc<Self>,
        request: FlatsRequest,
    ) -> Result<CalibrationView, Refusal> {
        let darks = request.darks == Some(true);
        let count = if darks && request.count.is_none() && request.label.is_none() {
            None
        } else if !darks
            && request.anyway != Some(true)
            && request.count.is_some()
            && request.label.is_some()
        {
            let count = set_count(request.count)?;
            let label = request.label.as_deref().expect("label present");
            if !flat_label(label) {
                return Err(Refusal::bad_request("label must be 1 to 32 characters of a to z, 0 to 9 and -, starting with a letter or digit"));
            }
            Some(count)
        } else {
            return Err(Refusal::bad_request(
                "send count and label to take flats, or darks: true to take their darks",
            ));
        };
        let (view, lease) = self.start_step(darks)?;
        let original_ms = view.settings.as_ref().expect("ready settings").exposure_ms;
        let range = &view.camera.as_ref().expect("ready camera").exposure_ms;
        let shortest_ms = range.min;
        let longest_ms = range.max.min(FLAT_MAX_EXPOSURE_MS);
        let calibration = self.clone();
        if let Some(count) = count {
            let stop = Arc::new(AtomicBool::new(false));
            let id = self.library.new_id(Kind::Flat, self.clock.now());
            let label = request.label.expect("flat label");
            *self.set.lock().expect("latest set") = Some(Set {
                view: SetView {
                    id: id.clone(),
                    kind: "flat",
                    state: "search",
                    wanted: count,
                    taken: 0,
                    exposure_ms: original_ms,
                    label: Some(label.clone()),
                    offset_level: None,
                    level: None,
                    level_share: None,
                    darks_taken: None,
                    not_dark: false,
                    error: None,
                },
                stop: stop.clone(),
                flats: None,
            });
            let answer = self.current_view();
            tokio::spawn(async move {
                let run = Run {
                    id,
                    count,
                    exposure_ms: original_ms,
                    shortest_ms,
                    anyway: false,
                    stop,
                };
                let result = calibration.run_flats(&lease, &run, longest_ms, label).await;
                calibration.end_step(lease, original_ms).await;
                let mut held = calibration.set.lock().expect("latest set");
                let set = held.as_mut().expect("running flat set");
                if run.stop.load(Ordering::Acquire) {
                    set.view.state = "stopped";
                } else {
                    match result {
                        Ok(flats) => {
                            set.flats = Some(Arc::new(flats));
                            set.view.state = "needs_darks";
                        }
                        Err(SetEnd::Stopped) => set.view.state = "stopped",
                        Err(SetEnd::Failed { message, not_dark }) => {
                            set.view.state = "failed";
                            set.view.error = Some(message);
                            set.view.not_dark = not_dark;
                        }
                    }
                }
            });
            Ok(answer)
        } else {
            let (flats, run) = {
                let mut held = self.set.lock().expect("latest set");
                // DELETE can drop waiting flats on another runtime worker after
                // start_step checked them. No lock crosses the lease acquisition.
                let set = held
                    .as_mut()
                    .filter(|set| set.view.state == "needs_darks")
                    .ok_or_else(|| Refusal::conflict("no flats are waiting for their darks"))?;
                let flats = set.flats.take().expect("waiting flats");
                set.view.state = "flat_darks";
                set.view.darks_taken = Some(0);
                set.view.error = None;
                set.view.not_dark = false;
                let run = Run {
                    id: set.view.id.clone(),
                    count: set.view.wanted,
                    exposure_ms: flats.exposure_ms,
                    shortest_ms,
                    anyway: request.anyway == Some(true),
                    stop: set.stop.clone(),
                };
                (flats, run)
            };
            let answer = self.current_view();
            tokio::spawn(async move {
                let result = calibration
                    .run_flat_darks(&lease, &run, flats.clone())
                    .await;
                calibration.end_step(lease, original_ms).await;
                let mut held = calibration.set.lock().expect("latest set");
                let set = held.as_mut().expect("running flat darks");
                if run.stop.load(Ordering::Acquire) {
                    set.view.state = "stopped";
                } else {
                    match result {
                        Ok(()) => set.view.state = "done",
                        Err(SetEnd::Stopped) => set.view.state = "stopped",
                        Err(SetEnd::Failed { message, not_dark }) => {
                            set.flats = Some(flats);
                            set.view.state = "needs_darks";
                            set.view.error = Some(message);
                            set.view.not_dark = not_dark;
                        }
                    }
                }
            });
            Ok(answer)
        }
    }

    async fn run_flats(
        &self,
        lease: &Lease,
        run: &Run,
        longest_ms: f64,
        label: String,
    ) -> Result<FlatFrames, SetEnd> {
        let mut exposure = flat_exposure(run.exposure_ms, run.shortest_ms, longest_ms);
        let mut tried = Vec::new();
        let mut points = Vec::new();
        let mut first = None;
        let mut found = false;
        let mut last = (exposure, 0.0);
        for _ in 0..FLAT_SEARCH_FRAMES {
            let frame = self
                .set_frame(lease, &run.stop, exposure, first.as_ref(), "flat")
                .await?;
            let (frame, level) = flat_level(frame).await?;
            let full_scale = full_scale(&frame.key);
            first.get_or_insert(frame.key);
            tried.push(exposure);
            let share = level / full_scale;
            self.update_set(|set| set.level = Some(level));
            last = (exposure, 100.0 * share);
            if (FLAT_LOW..=FLAT_HIGH).contains(&share) {
                found = true;
                break;
            }
            let next = if share >= FLAT_SATURATED {
                exposure / 8.0
            } else {
                points.push((exposure, level));
                if points.len() == 1 {
                    if share < FLAT_TARGET {
                        exposure * 2.0
                    } else {
                        exposure / 2.0
                    }
                } else {
                    let (ea, ma) = points[points.len() - 2];
                    let (eb, mb) = points[points.len() - 1];
                    let slope = (mb - ma) / (eb - ea);
                    if slope <= 0.0 || !slope.is_finite() {
                        return Err(SetEnd::failed(format!("the flat's level does not rise with the exposure: {ea} ms and {eb} ms read {ma} and {mb}")));
                    }
                    eb + (FLAT_TARGET * full_scale - mb) / slope
                }
            };
            let next = flat_exposure(next, run.shortest_ms, longest_ms);
            if tried.contains(&next) {
                let p = share * 100.0;
                return Err(SetEnd::failed(if share < FLAT_LOW {
                    format!("the flat is too dim: {exposure} ms reads {p:.1}% of full scale; is the panel lit?")
                } else {
                    format!("the flat is too bright: {exposure} ms reads {p:.1}% of full scale; dim the panel")
                }));
            }
            exposure = next;
        }
        if !found {
            let (e, p) = last;
            return Err(SetEnd::failed(format!("no exposure put the flat between 40 and 50% of full scale in 8 tries; the last, {e} ms, read {p:.1}%")));
        }
        self.update_set(|set| {
            set.exposure_ms = exposure;
            set.state = "frames";
        });
        let mut sum = Vec::new();
        let mut temperature = Some(0.0);
        let mut shares = 0.0;
        for _ in 0..run.count {
            let frame = self
                .set_frame(lease, &run.stop, exposure, first.as_ref(), "flat")
                .await?;
            let (frame, level) = flat_level(frame).await?;
            let share = level / full_scale(&frame.key);
            self.update_set(|set| set.level = Some(level));
            if !(FLAT_LOW..=FLAT_HIGH).contains(&share) {
                let p = 100.0 * share;
                return Err(SetEnd::failed(format!(
                    "a flat reads {p:.1}% of full scale, outside 40 to 50; is the panel steady?"
                )));
            }
            shares += share;
            temperature = temperature
                .zip(frame.key.temp_c)
                .map(|(total, t)| total + t);
            sum = add_samples(sum, frame.mosaic, "flat").await?;
            self.update_set(|set| set.taken += 1);
        }
        if run.stop.load(Ordering::Acquire) {
            return Err(SetEnd::Stopped);
        }
        let level_share = shares / f64::from(run.count);
        self.update_set(|set| set.level_share = Some(level_share));
        Ok(FlatFrames {
            frames: FrameSum {
                sum,
                key: first.expect("search frame"),
                temperature: temperature.map(|t| t / f64::from(run.count)),
            },
            exposure_ms: exposure,
            level_share,
            label,
        })
    }

    async fn run_flat_darks(
        self: &Arc<Self>,
        lease: &Lease,
        run: &Run,
        flats: Arc<FlatFrames>,
    ) -> Result<(), SetEnd> {
        let darks = self
            .collect_darks(lease, run, Some(&flats.frames.key))
            .await?;
        let entry = MasterEntry {
            kind: Kind::Flat,
            label: Some(flats.label.clone()),
            dark_frames: Some(run.count),
            level_share: Some(flats.level_share),
            ..self.master_entry(run, &flats.frames)
        };
        let calibration = self.clone();
        let count = run.count;
        tokio::task::spawn_blocking(move || {
            let mut mosaic = Mosaic {
                width: entry.width,
                height: entry.height,
                bayer: entry.bayer.clone(),
                samples: flats
                    .frames
                    .sum
                    .iter()
                    .zip(darks.sum)
                    .map(|(&flat, dark)| (f64::from(flat) - f64::from(dark)) as f32 / count as f32)
                    .collect(),
            };
            normalise_flat(&mut mosaic);
            for sample in &mut mosaic.samples {
                if !sample.is_finite() || *sample <= 0.0 {
                    *sample = 0.0;
                }
            }
            calibration.library.add(entry, &mosaic.samples)
        })
        .await
        .map_err(master_error)?
        .map_err(master_error)?;
        Ok(())
    }

    /// One ordinary saved imaging frame. All camera changes and takes go
    /// through the task's lease, while reads, abort and ended stay open.
    async fn set_frame(
        &self,
        lease: &Lease,
        stop: &AtomicBool,
        exposure_ms: f64,
        first: Option<&FrameKey>,
        purpose: &str,
    ) -> Result<SetFrame, SetEnd> {
        if stop.load(Ordering::Acquire) {
            return Err(SetEnd::Stopped);
        }
        lease
            .set(SettingsEdit {
                exposure_ms: Some(exposure_ms),
                ..Default::default()
            })
            .await
            .map_err(|e| SetEnd::frame(purpose, e.message))?;
        let view = lease
            .take(purpose)
            .await
            .map_err(|e| SetEnd::frame(purpose, e.message))?;
        let id = view
            .frame
            .as_ref()
            .ok_or_else(|| SetEnd::frame(purpose, state_refusal(&view).message))?
            .id;
        // remove's abort can precede the first frame; the flag closes that race.
        if stop.load(Ordering::Acquire) {
            self.imaging.abort().await;
        }
        let frame = self.imaging.ended(id).await;
        if stop.load(Ordering::Acquire) || frame.state == "aborted" {
            return Err(SetEnd::Stopped);
        }
        if frame.state == "failed" {
            return Err(SetEnd::frame(
                purpose,
                frame.error.as_deref().unwrap_or("the exposure failed"),
            ));
        }
        let file = frame
            .file
            .ok_or_else(|| SetEnd::frame(purpose, "the imaging frame has no file"))?;
        let path = self.imaging.root().join(&file);
        let read = tokio::task::spawn_blocking(move || -> Result<SetFrame, String> {
            let image = fits::decode(&std::fs::read(path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            let key = FrameKey::from_image(&image)?;
            let mosaic = Mosaic::from_samples(
                &image.samples,
                image.width,
                image.height,
                image.bits,
                key.bayer.as_deref(),
            )
            .ok_or("its samples do not fit its size")?;
            Ok(SetFrame { key, mosaic })
        })
        .await
        .map_err(|e| SetEnd::frame(purpose, format!("could not read {file}: {e}")))?
        .map_err(|e| SetEnd::frame(purpose, format!("could not read {file}: {e}")))?;
        if !read.key.agrees(first.unwrap_or(&read.key), exposure_ms) {
            return Err(SetEnd::failed(
                "the imaging settings changed during the set",
            ));
        }
        if let (Some(s), Some(t)) = (read.key.set_temp_c, read.key.temp_c) {
            if (t - s).abs() > SET_TEMP_TOLERANCE_C {
                return Err(SetEnd::failed(format!(
                    "the sensor is at {t:.1} °C, not yet at its set temperature {s:.1} °C"
                )));
            }
        }
        Ok(read)
    }

    async fn collect_darks(
        &self,
        lease: &Lease,
        run: &Run,
        flat_key: Option<&FrameKey>,
    ) -> Result<FrameSum, SetEnd> {
        let offset = self
            .set_frame(lease, &run.stop, run.shortest_ms, flat_key, "dark")
            .await?;
        let (first, level, noise, full_scale) = tokio::task::spawn_blocking(move || {
            let level = f64::from(percentile(&offset.mosaic.samples, 0.5).unwrap_or(0.0));
            let noise = f64::from(
                (level as f32 - percentile(&offset.mosaic.samples, 0.16).unwrap_or(0.0))
                    .max(NOISE_FLOOR),
            );
            let full_scale = 2.0f64.powi(offset.key.bits as i32) - 1.0;
            (offset.key, level, noise, full_scale)
        })
        .await
        .map_err(SetEnd::dark)?;
        self.update_set(|set| set.offset_level = Some(level));
        if !run.anyway && level > COVER_SHARE * full_scale {
            return Err(SetEnd::not_dark(format!(
                "the camera is not covered: its shortest exposure reads {level} of {full_scale}"
            )));
        }
        lease
            .set(SettingsEdit {
                exposure_ms: Some(run.exposure_ms),
                ..Default::default()
            })
            .await
            .map_err(|e| SetEnd::dark(e.message))?;
        self.update_set(|set| {
            set.state = if flat_key.is_some() {
                "flat_darks"
            } else {
                "frames"
            }
        });
        let mut sum = Vec::<u32>::new();
        let mut master_key = None;
        let mut temperature = Some(0.0);
        for _ in 0..run.count {
            let frame = self
                .set_frame(
                    lease,
                    &run.stop,
                    run.exposure_ms,
                    Some(flat_key.unwrap_or(&first)),
                    "dark",
                )
                .await?;
            let anyway = run.anyway;
            let (frame, median, lit) = tokio::task::spawn_blocking(move || {
                let median = f64::from(percentile(&frame.mosaic.samples, 0.5).unwrap_or(0.0));
                let lit = if anyway || median > level + DARK_RISE_SIGMAS * noise {
                    0
                } else {
                    lit_samples(&frame.mosaic)
                };
                (frame, median, lit)
            })
            .await
            .map_err(SetEnd::dark)?;
            self.update_set(|set| set.level = Some(median));
            if !run.anyway {
                if median > level + DARK_RISE_SIGMAS * noise {
                    return Err(SetEnd::not_dark(format!("the darks are not dark: one reads {median} against an offset level of {level}; cover the telescope")));
                }
                if lit > LIT_MIN && lit as f64 > LIT_SHARE * frame.mosaic.samples.len() as f64 {
                    return Err(SetEnd::not_dark(format!("the darks are not dark: one holds {lit} bright samples that are not hot pixels; cover the telescope")));
                }
            }
            temperature = temperature.zip(frame.key.temp_c).map(|(sum, t)| sum + t);
            master_key.get_or_insert(frame.key);
            sum = add_samples(sum, frame.mosaic, "dark").await?;
            self.update_set(|set| {
                if let Some(taken) = set.darks_taken.as_mut() {
                    *taken += 1;
                } else {
                    set.taken += 1;
                }
            });
        }
        if run.stop.load(Ordering::Acquire) {
            return Err(SetEnd::Stopped);
        }
        Ok(FrameSum {
            sum,
            key: master_key.expect("at least three darks"),
            temperature: temperature.map(|t| t / f64::from(run.count)),
        })
    }

    fn master_entry(&self, run: &Run, frames: &FrameSum) -> MasterEntry {
        let key = frames.key.clone();
        MasterEntry {
            id: run.id.clone(),
            kind: Kind::Dark,
            made_at: self.clock.now(),
            frames: run.count,
            checked: !run.anyway,
            model: key.model,
            serial: key.serial,
            width: key.width,
            height: key.height,
            origin_x: key.origin_x,
            origin_y: key.origin_y,
            bits: key.bits,
            bayer: key.bayer,
            gain: key.gain,
            offset: key.offset,
            binning: key.binning,
            set_temp_c: key.set_temp_c,
            temp_c: frames.temperature,
            exposure_ms: run.exposure_ms,
            level: None,
            bad_pixels: None,
            label: None,
            imaging_angle_deg: None,
            level_share: None,
            dark_frames: None,
            file: String::new(),
        }
    }

    async fn run_darks(self: &Arc<Self>, lease: &Lease, run: &Run) -> Result<(), SetEnd> {
        let frames = self.collect_darks(lease, run, None).await?;
        let entry = self.master_entry(run, &frames);
        let count = run.count;
        let calibration = self.clone();
        tokio::task::spawn_blocking(move || -> std::io::Result<()> {
            let samples: Vec<f32> = frames
                .sum
                .into_iter()
                .map(|s| s as f32 / count as f32)
                .collect();
            let entry = calibration.library.add(
                MasterEntry {
                    level: percentile(&samples, 0.5).map(f64::from),
                    ..entry
                },
                &samples,
            )?;
            let map = bad_pixel_map(&samples, entry.width, entry.height);
            calibration.library.attach_bad_pixels(&entry.id, &map)?;
            Ok(())
        })
        .await
        .map_err(|e| SetEnd::failed(format!("the master could not be saved: {e}")))?
        .map_err(|e| SetEnd::failed(format!("the master could not be saved: {e}")))?;
        Ok(())
    }
}

fn set_count(count: Option<u32>) -> Result<u32, Refusal> {
    count
        .filter(|n| (MIN_SET_FRAMES..=MAX_SET_FRAMES).contains(n))
        .ok_or_else(|| Refusal::bad_request("count must be a whole number from 3 to 64"))
}

fn flat_label(label: &str) -> bool {
    let letter_or_digit = |b: u8| b.is_ascii_lowercase() || b.is_ascii_digit();
    (1..=32).contains(&label.len())
        && letter_or_digit(label.as_bytes()[0])
        && label.bytes().all(|b| letter_or_digit(b) || b == b'-')
}

fn flat_exposure(exposure: f64, lo: f64, hi: f64) -> f64 {
    (exposure.clamp(lo, hi) * 1000.0).round() / 1000.0
}

fn full_scale(key: &FrameKey) -> f64 {
    2.0f64.powi(key.bits as i32) - 1.0
}

async fn flat_level(frame: SetFrame) -> Result<(SetFrame, f64), SetEnd> {
    tokio::task::spawn_blocking(move || {
        let level = levels(&frame.mosaic)
            .iter()
            .map(|l| l.median)
            .fold(0.0f32, f32::max);
        (frame, f64::from(level))
    })
    .await
    .map_err(|e| SetEnd::frame("flat", e))
}

async fn add_samples(
    mut sum: Vec<u32>,
    mosaic: Mosaic,
    purpose: &'static str,
) -> Result<Vec<u32>, SetEnd> {
    tokio::task::spawn_blocking(move || {
        if sum.is_empty() {
            sum.resize(mosaic.samples.len(), 0);
        }
        for (total, sample) in sum.iter_mut().zip(mosaic.samples) {
            *total += sample as u32;
        }
        sum
    })
    .await
    .map_err(|e| SetEnd::frame(purpose, e))
}

fn master_error(error: impl std::fmt::Display) -> SetEnd {
    SetEnd::failed(format!("the master could not be saved: {error}"))
}

fn removal_error(error: String) -> Refusal {
    Refusal {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        message: format!("could not remove the master: {error}"),
    }
}

/// A kind, eight date digits, T, six time digits, Z and an optional numeric
/// suffix. Check bytes before passing any id to the library.
fn master_id(id: &str) -> bool {
    let Some(stamp) = id
        .strip_prefix("dark-")
        .or_else(|| id.strip_prefix("flat-"))
    else {
        return false;
    };
    let bytes = stamp.as_bytes();
    bytes.len() >= 16
        && bytes[..8].iter().all(u8::is_ascii_digit)
        && bytes[8] == b'T'
        && bytes[9..15].iter().all(u8::is_ascii_digit)
        && bytes[15] == b'Z'
        && (bytes.len() == 16
            || (bytes[16] == b'-'
                && bytes.len() > 17
                && bytes[17..].iter().all(u8::is_ascii_digit)))
}

fn running(state: &str) -> bool {
    matches!(state, "offset" | "frames" | "search" | "flat_darks")
}

fn state_refusal(view: &ImagingView) -> Refusal {
    Refusal::conflict(match view.state {
        "no_link" => "the scope node is not connected".to_owned(),
        "no_service" => "this scope node has no imaging service".to_owned(),
        "no_camera" => "this scope node has no imaging camera".to_owned(),
        _ => format!(
            "the imaging camera is not answering: {}",
            view.error.as_deref().unwrap_or_default()
        ),
    })
}
