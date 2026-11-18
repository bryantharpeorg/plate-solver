//! What the console keeps between the finder camera and the imaging camera
//! (032): the scale a solved imaging frame measured, the focal length that
//! sizes a solve's field until one is kept, the misses that turn the hint's
//! place and its field height from one source to the next, the history of the
//! finder camera's live solves, and the aim readings an imaging solve takes
//! from it — the last five held, smoothed into medians, with the outliers set
//! aside.

use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
};

use math_core::UnitVector;
use serde::{Deserialize, Serialize};

use super::{solver::ImagingSolve, solving::ARCSEC_PER_RADIAN, CameraView};

/// The format line of `relation.json` (FR-017).
pub const FORMAT: &str = "imaging-relation/1";

/// After this many more solves have ended `not_solved` or `timed_out` with no
/// solved solve between them, the field height's next source is used, and the
/// hint's place moves one source down (FR-014, FR-016).
pub const SCALE_MISSES: u32 = 3;

/// The history of live solves keeps at least this many seconds (FR-019).
pub const HISTORY_MIN_S: f64 = 120.0;
/// The most fixes the history holds over its span: a fix that follows the
/// newest one kept by less than `keep_s / HISTORY_MAX as f64` seconds is not
/// kept (FR-019). It is a density, not a queue length: no fix is ever dropped
/// for being one too many.
pub const HISTORY_MAX: usize = 2000;
/// The fit reaches this far before the exposure's start and after its end, or
/// the exposure's own length if that is longer (FR-020).
pub const FIT_REACH_S: f64 = 10.0;
/// Fewer fixes than this in the fit take no aim reading (FR-020).
pub const FIT_MIN: usize = 3;
/// The line's travel during the exposure, beyond its standard error, that
/// makes the telescope moving, in arcseconds on the sky (FR-020).
pub const MAX_TRAVEL_ARCSEC: f64 = 60.0;
/// The scatter about the line that makes the fixes unsteady, in arcseconds on
/// the sky (FR-020).
pub const MAX_SCATTER_ARCSEC: f64 = 90.0;

/// How many aim readings the window holds (FR-024).
pub const WINDOW: usize = 5;
/// A reading this far on the sky from the median aim point is an outlier
/// (FR-025).
pub const OUTLIER_ARCSEC: f64 = 120.0;
/// A reading this far in angle from the median imaging angle is an outlier
/// (FR-025).
pub const OUTLIER_ANGLE_DEG: f64 = 1.0;
/// This many outliers in a row replace what the window holds with themselves
/// (FR-025).
pub const REPLACE_AFTER: usize = 3;
/// The medians must be this far on the sky from the aim point in force before
/// the session's aim point moves (FR-026).
pub const AIM_STEP_ARCSEC: f64 = 10.0;
/// The narrow radius a hint takes once the aim point is measured is never
/// under this (FR-028).
pub const NARROW_MIN_DEG: f64 = 0.2;

/// The camera a kept scale belongs to: the file names its model, serial, size
/// and pixel size, and a file made for another camera is ignored (FR-017).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CameraId {
    pub model: String,
    pub serial: Option<String>,
    pub width: u32,
    pub height: u32,
    pub pixel_um: f64,
}

/// The camera a kept scale belongs to, from the camera's description.
impl From<&CameraView> for CameraId {
    fn from(camera: &CameraView) -> Self {
        Self {
            model: camera.model.clone(),
            serial: camera.serial.clone(),
            width: camera.width,
            height: camera.height,
            pixel_um: camera.pixel_um,
        }
    }
}

/// The kept scale: arcseconds a sensor pixel, and the solve it came from
/// (FR-017).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Scale {
    pub arcsec_per_px: f64,
    pub frame_id: u64,
    /// Console Unix seconds when the solve ended.
    pub at: f64,
}

/// `relation.json` (FR-017, FR-027): the scale kept, the aim readings held
/// and those set aside, each with the size of the finder frame it was made
/// in. The two lists default to empty when their key is absent and are left
/// out while empty.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stored {
    pub format: String,
    pub camera: CameraId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<Scale>,
    /// The aim readings held, oldest first (FR-027).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub readings: Vec<AimReading>,
    /// The aim readings set aside, oldest first (FR-027).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub set_aside: Vec<AimReading>,
}

/// `None` when the file is missing, unreadable, of another format or of
/// another camera (FR-017).
pub fn load(path: &Path, camera: &CameraId) -> Option<Stored> {
    let bytes = std::fs::read(path).ok()?;
    let stored: Stored = serde_json::from_slice(&bytes).ok()?;
    (stored.format == FORMAT && stored.camera == *camera).then_some(stored)
}

/// Write the file whole under a `.part` name and rename it over any earlier
/// one (FR-017).
pub fn save(path: &Path, stored: &Stored) -> std::io::Result<()> {
    let bytes = serde_json::to_vec(stored).expect("the relation's numbers are finite");
    let part = path.with_extension("json.part");
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder)?;
    }
    std::fs::write(&part, bytes)?;
    std::fs::rename(&part, path)
}

/// The aim point as `relation` in the imaging view shows it, in finder pixels
/// (FR-023).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Xy {
    pub x: f64,
    pub y: f64,
}

/// How the last solved whole-sensor frame fared (FR-023). In JSON:
/// `measured`, `replaced`, `set_aside`, `no_live_pointing`, `moving` or
/// `off_frame`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LastOutcome {
    Measured,
    Replaced,
    SetAside,
    NoLivePointing,
    Moving,
    OffFrame,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Last {
    pub frame_id: u64,
    pub outcome: LastOutcome,
}

/// The imaging sensor's size on the sky (FR-023).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Field {
    pub width_arcsec: f64,
    pub height_arcsec: f64,
}

/// `relation` in the imaging view: the kept scale and the focal length, and
/// the aim reading an imaging solve took. Non-exhaustive: later stories add
/// parts.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct RelationView {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<Scale>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focal_length_mm: Option<f64>,
    /// The aim point measured, in finder pixels (FR-023).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aim: Option<Xy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub angle_deg: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mirrored: Option<bool>,
    /// How many aim readings are held (FR-023).
    pub readings: usize,
    /// How many aim readings are set aside (FR-025).
    pub set_aside: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last: Option<Last>,
    /// The sensor's size times the kept scale (FR-023).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<Field>,
    /// A measured aim point is held back while Track is on (032, FR-036).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub held: bool,
}

/// One live solve of the finder camera: a finder fix (FR-019).
#[derive(Debug, Clone, Copy)]
pub struct FinderFix {
    /// Mid-exposure time on the console's clock, Unix seconds.
    pub time: f64,
    /// The solve's projection.
    pub frame: crate::sky::Frame,
    /// The node exposure the solve is of.
    pub frame_id: u64,
}

/// The history of the finder camera's live solves (FR-019): bounded by its
/// span alone — a fix that follows the newest one kept by less than the span
/// over `HISTORY_MAX` is not kept, and only fixes older than the span are
/// dropped.
#[derive(Debug, Default)]
pub struct FinderHistory {
    /// Oldest first.
    fixes: VecDeque<FinderFix>,
}

impl FinderHistory {
    /// Keeps the fix unless one is held and the fix's time is less than
    /// `keep_s / HISTORY_MAX as f64` after the newest one's; then drops every
    /// fix whose time is more than `keep_s` before the newest one's. Nothing
    /// else is dropped.
    pub fn push(&mut self, fix: FinderFix, keep_s: f64) {
        let spacing = keep_s / HISTORY_MAX as f64;
        let keep = self
            .fixes
            .back()
            .is_none_or(|newest| fix.time - newest.time >= spacing);
        if keep {
            self.fixes.push_back(fix);
            let keep_from = self.fixes.back().expect("the fix just pushed").time - keep_s;
            self.fixes.retain(|fix| fix.time >= keep_from);
        }
    }

    /// The fixes held, oldest first.
    pub fn fixes(&self) -> Vec<FinderFix> {
        self.fixes.iter().copied().collect()
    }
}

/// One reading of the aim point and the imaging angle: an aim reading
/// (FR-020). The tests build one by literal, so every field is here from US4
/// on and no later story or spec adds one without planning it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AimReading {
    /// The aim point, in finder pixels.
    pub x: f64,
    pub y: f64,
    /// The imaging angle, the imaging frame's up east of north less the
    /// finder frame's up seen at the imaging centre, in 0 to 360 (FR-021).
    pub angle_deg: f64,
    pub mirrored: bool,
    /// The middle of the imaging exposure, console Unix seconds.
    pub at: f64,
    pub frame_id: u64,
    /// The finder solve's scale at the aim reading.
    pub finder_arcsec_per_px: f64,
    /// The size of the finder frame `x` and `y` are pixels of.
    pub finder_width: f64,
    pub finder_height: f64,
}

/// What `read_aim` answered for one frame (FR-020).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Outcome {
    Measured(AimReading),
    NoLivePointing,
    Moving,
    OffFrame,
}

/// Take an aim reading from an imaging solve and the history's live solves
/// (FR-020, FR-021): project the solve's centre through the fixes around the
/// exposure, fit a straight line by least squares to those places against
/// time, in each axis of the finder camera's frame, and read the line's value
/// at the middle of the exposure. It reads no axis position and computes no
/// altitude or azimuth.
pub fn read_aim(
    fixes: &[FinderFix],
    solve: &ImagingSolve,
    start: f64,
    exposure_s: f64,
    frame_id: u64,
) -> Outcome {
    let (t0, t1) = (start, start + exposure_s);
    let ti = (t0 + t1) / 2.0;
    let reach = FIT_REACH_S.max(exposure_s);
    let centre = UnitVector::from_radec(solve.ra_deg.to_radians(), solve.dec_deg.to_radians());
    // 1. The fix inside the exposure nearest its middle, the earlier of two
    //    equally near.
    let Some(anchor) = fixes
        .iter()
        .filter(|fix| fix.time >= t0 && fix.time <= t1)
        .min_by(|a, b| (a.time - ti).abs().total_cmp(&(b.time - ti).abs()))
    else {
        return Outcome::NoLivePointing;
    };
    // 2. The centre through the anchor's frame, and the anchor's up there.
    let Some(place) = anchor.frame.project(centre) else {
        return Outcome::OffFrame;
    };
    if !anchor.frame.contains(place, 0.0) {
        return Outcome::OffFrame;
    }
    let Some(up) = anchor.frame.up_east_of_north_at(centre) else {
        return Outcome::OffFrame;
    };
    let scale = anchor.frame.arcsec_per_px();
    // 3. The fitted fixes: those in the reach, of the anchor's size, whose
    //    own centre-through-frame lies inside their own frame.
    let size = anchor.frame.size();
    let mut points: Vec<(f64, f64, f64)> = Vec::new();
    for fix in fixes {
        if !(fix.time >= t0 - reach && fix.time <= t1 + reach) || fix.frame.size() != size {
            continue;
        }
        let Some((x, y)) = fix.frame.project(centre) else {
            continue;
        };
        if !fix.frame.contains((x, y), 0.0) {
            continue;
        }
        points.push((fix.time - ti, x, y));
    }
    if points.len() < FIT_MIN {
        return Outcome::NoLivePointing;
    }
    // 4. The line, by least squares in each axis.
    let n = points.len() as f64;
    let mean = |sum: f64| sum / n;
    let (mean_tau, mean_x, mean_y) = (
        mean(points.iter().map(|p| p.0).sum()),
        mean(points.iter().map(|p| p.1).sum()),
        mean(points.iter().map(|p| p.2).sum()),
    );
    let d = points
        .iter()
        .map(|p| (p.0 - mean_tau) * (p.0 - mean_tau))
        .sum::<f64>();
    if d <= 1e-9 {
        return Outcome::NoLivePointing;
    }
    let bx = points
        .iter()
        .map(|p| (p.0 - mean_tau) * (p.1 - mean_x))
        .sum::<f64>()
        / d;
    let by = points
        .iter()
        .map(|p| (p.0 - mean_tau) * (p.2 - mean_y))
        .sum::<f64>()
        / d;
    let (ax, ay) = (mean_x - bx * mean_tau, mean_y - by * mean_tau);
    // 5. The scatter about the line.
    let scatter =
        |residuals: Vec<f64>| (residuals.iter().map(|r| r * r).sum::<f64>() / (n - 2.0)).sqrt();
    let sx = scatter(points.iter().map(|p| p.1 - (ax + bx * p.0)).collect());
    let sy = scatter(points.iter().map(|p| p.2 - (ay + by * p.0)).collect());
    if sx.hypot(sy) * scale > MAX_SCATTER_ARCSEC {
        return Outcome::Moving;
    }
    // 6. The travel during the exposure, beyond its own standard error.
    let travel = bx.hypot(by) * exposure_s * scale;
    let error = sx.hypot(sy) / d.sqrt() * exposure_s * scale;
    if travel - error > MAX_TRAVEL_ARCSEC {
        return Outcome::Moving;
    }
    // 7. The imaging angle (FR-021).
    let angle_deg = (solve.up_east_of_north_deg - up).rem_euclid(360.0);
    // 8.
    Outcome::Measured(AimReading {
        x: ax,
        y: ay,
        angle_deg,
        mirrored: solve.mirrored,
        at: ti,
        frame_id,
        finder_arcsec_per_px: scale,
        finder_width: size.0,
        finder_height: size.1,
    })
}

/// How `Window::take` fared with one aim reading (FR-024, FR-025).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Taken {
    /// Held: the medians may have moved.
    Measured,
    /// An outlier: set aside, moving nothing.
    SetAside,
    /// The third outlier in a row: it and the two before it now are what is
    /// held.
    Replaced,
}

/// The last aim readings held and those set aside (FR-024, FR-025): the
/// medians of what is held are the aim point and the imaging angle the
/// console answers with.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Window {
    /// Oldest first.
    readings: Vec<AimReading>,
    /// Oldest first.
    aside: Vec<AimReading>,
}

impl Window {
    /// Nothing held.
    pub fn new() -> Self {
        Self::default()
    }

    /// The readings held and those set aside, oldest first, as the file holds
    /// them (FR-027).
    pub fn from_parts(readings: Vec<AimReading>, set_aside: Vec<AimReading>) -> Self {
        Self {
            readings,
            aside: set_aside,
        }
    }

    /// Take one aim reading (FR-024, FR-025). A reading made in a finder
    /// frame of another size than the newest one held or set aside drops
    /// everything first, since its pixels mean something else; then, with
    /// three or more held, a reading more than 120″ on the sky from the
    /// median aim point, more than 1° from the median imaging angle or of the
    /// other mirror sense is set aside, and the third set aside in a row
    /// replaces what is held; a reading that is none of those clears those
    /// set aside, is held, and pushes the oldest out beyond five.
    pub fn take(&mut self, reading: AimReading) -> Taken {
        // The finder frame first: pixels of another size mean something else
        // (FR-027).
        if self
            .readings
            .last()
            .or(self.aside.last())
            .is_some_and(|newest| {
                newest.finder_width != reading.finder_width
                    || newest.finder_height != reading.finder_height
            })
        {
            self.readings.clear();
            self.aside.clear();
        }
        // The outlier test needs three held (FR-025).
        let outlier = self.readings.len() >= REPLACE_AFTER
            && self
                .aim()
                .zip(self.angle_deg())
                .is_some_and(|((x, y), angle)| {
                    let distance =
                        (reading.x - x).hypot(reading.y - y) * reading.finder_arcsec_per_px;
                    let turned =
                        ((reading.angle_deg - angle + 180.0).rem_euclid(360.0) - 180.0).abs();
                    distance > OUTLIER_ARCSEC
                        || turned > OUTLIER_ANGLE_DEG
                        || reading.mirrored != self.mirrored().expect("a reading is held")
                });
        if outlier {
            self.aside.push(reading);
            if self.aside.len() >= REPLACE_AFTER {
                self.readings = std::mem::take(&mut self.aside);
                return Taken::Replaced;
            }
            return Taken::SetAside;
        }
        self.aside.clear();
        self.readings.push(reading);
        if self.readings.len() > WINDOW {
            self.readings.remove(0);
        }
        Taken::Measured
    }

    /// How many aim readings are held.
    pub fn len(&self) -> usize {
        self.readings.len()
    }

    /// Whether none is held.
    pub fn is_empty(&self) -> bool {
        self.readings.is_empty()
    }

    /// How many aim readings are set aside.
    pub fn set_aside(&self) -> usize {
        self.aside.len()
    }

    /// The medians of the x and of the y of what is held (FR-024).
    pub fn aim(&self) -> Option<(f64, f64)> {
        Some((
            median(self.readings.iter().map(|reading| reading.x).collect())?,
            median(self.readings.iter().map(|reading| reading.y).collect())?,
        ))
    }

    /// The median imaging angle, taken around the circle, 0 to 360 (FR-024).
    pub fn angle_deg(&self) -> Option<f64> {
        let oldest = self.readings.first()?;
        let differences = self
            .readings
            .iter()
            .map(|reading| (reading.angle_deg - oldest.angle_deg + 180.0).rem_euclid(360.0) - 180.0)
            .collect();
        Some((oldest.angle_deg + median(differences)?).rem_euclid(360.0))
    }

    /// The newest aim reading's mirror sense.
    pub fn mirrored(&self) -> Option<bool> {
        self.readings.last().map(|reading| reading.mirrored)
    }

    /// The aim readings held, oldest first.
    pub fn readings(&self) -> &[AimReading] {
        &self.readings
    }

    /// The aim readings set aside, oldest first.
    pub fn aside(&self) -> &[AimReading] {
        &self.aside
    }

    /// Drop everything held and set aside.
    pub fn clear(&mut self) {
        self.readings.clear();
        self.aside.clear();
    }
}

/// The median of the values: the middle one, or the mean of the middle two
/// (FR-024).
fn median(mut values: Vec<f64>) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let n = values.len();
    Some(if n % 2 == 1 {
        values[n / 2]
    } else {
        (values[n / 2 - 1] + values[n / 2]) / 2.0
    })
}

/// Whether the session's aim point should be set from a median (FR-026):
/// with no aim point in force, with one that is not a measured one, and when
/// the median is more than 10″ on the sky from the one in force.
pub fn should_apply(
    in_force: Option<crate::session::AimView>,
    median: (f64, f64),
    finder_arcsec_per_px: f64,
) -> bool {
    let Some(aim) = in_force else {
        return true;
    };
    aim.source != Some("measured")
        || ((median.0 - aim.x).hypot(median.1 - aim.y) * finder_arcsec_per_px > AIM_STEP_ARCSEC)
}

/// What the console keeps between the two cameras while it runs: the kept
/// scale, the focal length, the misses that turn the hint's sources, the
/// history of the finder camera's live solves and the window of aim
/// readings. One value lives in the imaging state; its fields are private.
#[derive(Debug)]
pub struct Relation {
    /// `<imaging folder>/relation.json`.
    path: PathBuf,
    /// The camera the kept scale belongs to; `None` before the camera is
    /// described.
    camera: Option<CameraId>,
    focal_length_mm: Option<f64>,
    scale: Option<Scale>,
    /// Solves ended `not_solved` or `timed_out` since the last solved one.
    misses: u32,
    /// The finder camera's live solves (FR-019).
    history: FinderHistory,
    /// The last aim readings held, and those set aside (FR-024, FR-025).
    window: Window,
    /// How the last solved whole-sensor frame fared (FR-023).
    last: Option<Last>,
}

impl Relation {
    /// Nothing known yet. `path` is `<imaging folder>/relation.json`; nothing
    /// is read.
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            camera: None,
            focal_length_mm: None,
            scale: None,
            misses: 0,
            history: FinderHistory::default(),
            window: Window::new(),
            last: None,
        }
    }

    /// The focal length in millimetres (FR-018). A value that is not finite or
    /// not above 0 is ignored.
    pub fn set_focal_length_mm(&mut self, mm: f64) {
        if mm.is_finite() && mm > 0.0 {
            self.focal_length_mm = Some(mm);
        }
    }

    /// The camera was described (FR-017, FR-027). For the camera already held
    /// this does nothing; otherwise the file is read for this camera, and
    /// what it holds — the scale and the two lists of aim readings — is taken
    /// or, with none, the kept scale and every aim reading dropped, and the
    /// misses zeroed.
    pub fn describe(&mut self, camera: CameraId) {
        if self.camera.as_ref() == Some(&camera) {
            return;
        }
        let stored = load(&self.path, &camera);
        self.scale = stored.as_ref().and_then(|stored| stored.scale);
        self.window = stored
            .map(|stored| Window::from_parts(stored.readings, stored.set_aside))
            .unwrap_or_default();
        self.misses = 0;
        self.last = None;
        self.camera = Some(camera);
    }

    /// Arcseconds a sensor pixel (FR-018): the kept scale, or without one the
    /// focal length's with the camera's pixel size. `None` with neither.
    pub fn scale_arcsec_per_px(&self) -> Option<f64> {
        if let Some(scale) = self.scale {
            return Some(scale.arcsec_per_px);
        }
        let camera = self.camera.as_ref()?;
        let focal = self.focal_length_mm?;
        Some(ARCSEC_PER_RADIAN * (camera.pixel_um / 1000.0) / focal)
    }

    /// `(scale, focal_length_mm)` for the hint's field height (FR-016): the
    /// sources that exist, in this order the kept scale, the focal length and
    /// none, and among them the one `misses / SCALE_MISSES` whole divisions
    /// down, turned back to the first when all have been used.
    pub fn height_inputs(&self) -> (Option<f64>, Option<f64>) {
        let kept = usize::from(self.scale.is_some());
        let focal = usize::from(self.focal_length_mm.is_some());
        let turn = (self.misses / SCALE_MISSES) as usize % (1 + kept + focal);
        if turn < kept {
            (
                Some(self.scale.expect("a kept scale").arcsec_per_px),
                self.focal_length_mm,
            )
        } else if turn < kept + focal {
            (None, self.focal_length_mm)
        } else {
            (None, None)
        }
    }

    /// A solve was solved: keep its scale in place of the last, zero the
    /// misses, and save (FR-016, FR-017). A failed save is printed to standard
    /// error and the value kept in memory.
    pub fn store_scale(&mut self, arcsec_per_px: f64, frame_id: u64, at: f64) {
        self.scale = Some(Scale {
            arcsec_per_px,
            frame_id,
            at,
        });
        self.misses = 0;
        self.save();
    }

    /// A solve ended `not_solved` or `timed_out`: one more miss (FR-014,
    /// FR-016).
    pub fn miss(&mut self) {
        self.misses += 1;
    }

    /// How many places down the hint's list of sources the next solve goes:
    /// the misses in a row divided by `SCALE_MISSES`, a whole division
    /// (FR-014).
    pub fn hint_skip(&self) -> usize {
        (self.misses / SCALE_MISSES) as usize
    }

    /// One more live solve of the finder camera (FR-019), with the span the
    /// history keeps, in seconds.
    pub fn push_fix(&mut self, fix: FinderFix, keep_s: f64) {
        self.history.push(fix, keep_s);
    }

    /// The history's live solves, oldest first (FR-019).
    pub fn fixes(&self) -> Vec<FinderFix> {
        self.history.fixes()
    }

    /// Record what `read_aim` answered for a frame (FR-020, FR-023, FR-024
    /// to FR-027). `in_force` is the aim point in force of now, from
    /// `solving::aim_in_force`; while one is held it is the held one. It
    /// answers the aim point to hand to `Session::set_measured_aim`, when
    /// there is one to hand over.
    pub fn take(
        &mut self,
        outcome: Outcome,
        frame_id: u64,
        in_force: Option<crate::session::AimView>,
    ) -> Option<(f64, f64)> {
        // An aim point that is not a measured one is replaced at the next
        // reading: the window is cleared first, and its readings with them
        // (FR-023, FR-027).
        let measured = in_force.is_some_and(|aim| aim.source == Some("measured"));
        if !measured {
            self.window.clear();
        }
        let outcome = match outcome {
            Outcome::Measured(reading) => {
                let taken = self.window.take(reading);
                self.last = Some(Last {
                    frame_id,
                    outcome: match taken {
                        Taken::Measured => LastOutcome::Measured,
                        Taken::SetAside => LastOutcome::SetAside,
                        Taken::Replaced => LastOutcome::Replaced,
                    },
                });
                // The medians move the session's aim point only past the
                // 10″ step (FR-026), and a reading set aside moves nothing.
                let answer = match taken {
                    Taken::SetAside => None,
                    Taken::Measured | Taken::Replaced => self.window.aim().and_then(|median| {
                        should_apply(in_force, median, reading.finder_arcsec_per_px)
                            .then_some(median)
                    }),
                };
                self.save();
                return answer;
            }
            Outcome::NoLivePointing => LastOutcome::NoLivePointing,
            Outcome::Moving => LastOutcome::Moving,
            Outcome::OffFrame => LastOutcome::OffFrame,
        };
        self.last = Some(Last { frame_id, outcome });
        None
    }

    /// The imaging sensor's size on the sky (FR-023): the camera's width and
    /// height times the kept scale; `None` without either.
    fn field(&self) -> Option<Field> {
        let camera = self.camera.as_ref()?;
        let scale = self.scale?;
        Some(Field {
            width_arcsec: f64::from(camera.width) * scale.arcsec_per_px,
            height_arcsec: f64::from(camera.height) * scale.arcsec_per_px,
        })
    }

    /// `relation` in the imaging view, measured against the aim point in
    /// force (FR-023, FR-025): `aim`, `angle_deg` and `mirrored` from the
    /// window's medians, and `readings` its count, only while the aim point
    /// in force is a measured one; `set_aside` the window's count; `held`
    /// while a measured aim point is held back by Track. `None` while it
    /// would hold nothing.
    pub fn view(
        &self,
        in_force: Option<crate::session::AimView>,
        held: bool,
    ) -> Option<RelationView> {
        let measured = in_force.is_some_and(|aim| aim.source == Some("measured"));
        let (aim, angle_deg, mirrored, readings) = if measured {
            (
                self.window.aim().map(|(x, y)| Xy { x, y }),
                self.window.angle_deg(),
                self.window.mirrored(),
                self.window.len(),
            )
        } else {
            (None, None, None, 0)
        };
        let set_aside = self.window.set_aside();
        let field = self.field();
        let empty = self.scale.is_none()
            && self.focal_length_mm.is_none()
            && aim.is_none()
            && self.last.is_none()
            && field.is_none()
            && readings == 0
            && set_aside == 0
            && !held;
        (!empty).then_some(RelationView {
            scale: self.scale,
            focal_length_mm: self.focal_length_mm,
            aim,
            angle_deg,
            mirrored,
            readings,
            set_aside,
            last: self.last,
            field,
            held,
        })
    }

    /// Write the file; a failed save is printed to standard error and the
    /// values kept in memory (FR-017, FR-027). Nothing is written before a
    /// camera is described: the file names its camera.
    fn save(&self) {
        let Some(camera) = self.camera.clone() else {
            return;
        };
        let stored = Stored {
            format: FORMAT.to_string(),
            camera,
            scale: self.scale,
            readings: self.window.readings().to_vec(),
            set_aside: self.window.aside().to_vec(),
        };
        if let Err(error) = save(&self.path, &stored) {
            eprintln!("could not save the imaging relation: {error}");
        }
    }
}
