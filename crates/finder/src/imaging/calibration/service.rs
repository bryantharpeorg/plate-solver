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
    bad_pixel_map,
    library::{FrameKey, Kind, Library, MasterEntry, Selection, SET_TEMP_TOLERANCE_C},
    lit_samples, percentile, Mosaic, NOISE_FLOOR,
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
    pub offset_level: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<f64>,
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

struct Set {
    view: SetView,
    stop: Arc<AtomicBool>,
}

struct DarkRun {
    id: String,
    count: u32,
    exposure_ms: f64,
    shortest_ms: f64,
    anyway: bool,
    stop: Arc<AtomicBool>,
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
        Self::failed(format!("a dark failed: {message}"))
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
        let count = request
            .count
            .filter(|n| (MIN_SET_FRAMES..=MAX_SET_FRAMES).contains(n))
            .ok_or_else(|| Refusal::bad_request("count must be a whole number from 3 to 64"))?;
        let view = self.imaging.view();
        if view.state != "ready" {
            return Err(state_refusal(&view));
        }
        let taking = self
            .set
            .lock()
            .expect("latest set")
            .as_ref()
            .is_some_and(|set| running(set.view.state));
        if taking {
            return Err(Refusal::conflict(
                "a set of darks or flats is already being taken",
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
            offset_level: None,
            level: None,
            not_dark: false,
            error: None,
        };
        *self.set.lock().expect("latest set") = Some(Set {
            view: set,
            stop: stop.clone(),
        });
        // Snapshot the accepted offset state before another runtime worker can
        // advance the task. The lease already belongs to this accepted request.
        let answer = self.current_view();
        let calibration = self.clone();
        tokio::spawn(async move {
            let run = DarkRun {
                id,
                count,
                exposure_ms,
                shortest_ms,
                anyway: request.anyway == Some(true),
                stop,
            };
            let result = calibration.run_darks(&lease, &run).await;
            // No path out of set_frame leaves our exposure in flight. Put the
            // exposure back and release the camera before publishing an ending.
            let _ = lease
                .set(SettingsEdit {
                    exposure_ms: Some(exposure_ms),
                    ..Default::default()
                })
                .await;
            drop(lease);
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

    /// One ordinary saved imaging frame. All camera changes and takes go
    /// through the task's lease, while reads, abort and ended stay open.
    async fn set_frame(
        &self,
        lease: &Lease,
        stop: &AtomicBool,
        exposure_ms: f64,
        first: Option<&FrameKey>,
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
            .map_err(|e| SetEnd::dark(e.message))?;
        let view = lease
            .take("dark")
            .await
            .map_err(|e| SetEnd::dark(e.message))?;
        let id = view
            .frame
            .as_ref()
            .ok_or_else(|| SetEnd::dark(state_refusal(&view).message))?
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
            return Err(SetEnd::dark(
                frame.error.as_deref().unwrap_or("the exposure failed"),
            ));
        }
        let file = frame
            .file
            .ok_or_else(|| SetEnd::dark("the imaging frame has no file"))?;
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
        .map_err(|e| SetEnd::dark(format!("could not read {file}: {e}")))?
        .map_err(|e| SetEnd::dark(format!("could not read {file}: {e}")))?;
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

    async fn run_darks(self: &Arc<Self>, lease: &Lease, run: &DarkRun) -> Result<(), SetEnd> {
        let offset = self
            .set_frame(lease, &run.stop, run.shortest_ms, None)
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
        self.update_set(|set| set.state = "frames");
        let mut sum = Vec::<u32>::new();
        let mut master_key = None;
        let mut temperature = Some(0.0);
        for _ in 0..run.count {
            let frame = self
                .set_frame(lease, &run.stop, run.exposure_ms, Some(&first))
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
            sum = tokio::task::spawn_blocking(move || {
                if sum.is_empty() {
                    sum.resize(frame.mosaic.samples.len(), 0);
                }
                for (total, sample) in sum.iter_mut().zip(frame.mosaic.samples) {
                    *total += sample as u32;
                }
                sum
            })
            .await
            .map_err(SetEnd::dark)?;
            self.update_set(|set| set.taken += 1);
        }
        if run.stop.load(Ordering::Acquire) {
            return Err(SetEnd::Stopped);
        }
        let key = master_key.expect("at least three darks");
        let entry = MasterEntry {
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
            temp_c: temperature.map(|t| t / f64::from(run.count)),
            exposure_ms: run.exposure_ms,
            level: None,
            bad_pixels: None,
            label: None,
            imaging_angle_deg: None,
            level_share: None,
            dark_frames: None,
            file: String::new(),
        };
        let count = run.count;
        let calibration = self.clone();
        tokio::task::spawn_blocking(move || -> std::io::Result<()> {
            let samples: Vec<f32> = sum.into_iter().map(|s| s as f32 / count as f32).collect();
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
