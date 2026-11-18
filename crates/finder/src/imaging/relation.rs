//! What the console keeps between the finder camera and the imaging camera
//! (032): the scale a solved imaging frame measured, the focal length that
//! sizes a solve's field until one is kept, and the misses that turn the hint's
//! place and its field height from one source to the next.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{solving::ARCSEC_PER_RADIAN, CameraView};

/// The format line of `relation.json` (FR-017).
pub const FORMAT: &str = "imaging-relation/1";

/// After this many more solves have ended `not_solved` or `timed_out` with no
/// solved solve between them, the field height's next source is used, and the
/// hint's place moves one source down (FR-014, FR-016).
pub const SCALE_MISSES: u32 = 3;

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

/// `relation.json` (FR-017). Non-exhaustive: a later story adds the aim
/// readings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stored {
    pub format: String,
    pub camera: CameraId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<Scale>,
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

/// `relation` in the imaging view: the kept scale and the focal length.
/// Non-exhaustive: later stories add parts.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct RelationView {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<Scale>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focal_length_mm: Option<f64>,
}

/// What the console keeps between the two cameras while it runs: the kept
/// scale, the focal length and the misses that turn the hint's sources. One
/// value lives in the imaging state; its fields are private.
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
        }
    }

    /// The focal length in millimetres (FR-018). A value that is not finite or
    /// not above 0 is ignored.
    pub fn set_focal_length_mm(&mut self, mm: f64) {
        if mm.is_finite() && mm > 0.0 {
            self.focal_length_mm = Some(mm);
        }
    }

    /// The camera was described (FR-017). For the camera already held this
    /// does nothing; otherwise the file is read for this camera, and what it
    /// holds is taken or the kept scale dropped, and the misses zeroed.
    pub fn describe(&mut self, camera: CameraId) {
        if self.camera.as_ref() == Some(&camera) {
            return;
        }
        self.scale = load(&self.path, &camera).and_then(|stored| stored.scale);
        self.misses = 0;
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

    /// `relation` in the imaging view: `None` while it would hold nothing.
    pub fn view(&self) -> Option<RelationView> {
        (self.scale.is_some() || self.focal_length_mm.is_some()).then_some(RelationView {
            scale: self.scale,
            focal_length_mm: self.focal_length_mm,
        })
    }

    /// Write the file; a failed save is printed to standard error and the
    /// values kept in memory (FR-017). Nothing is written before a camera is
    /// described: the file names its camera.
    fn save(&self) {
        let Some(camera) = self.camera.clone() else {
            return;
        };
        let stored = Stored {
            format: FORMAT.to_string(),
            camera,
            scale: self.scale,
        };
        if let Err(error) = save(&self.path, &stored) {
            eprintln!("could not save the imaging relation: {error}");
        }
    }
}
