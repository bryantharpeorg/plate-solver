//! The calibration library on disk (034): masters, their index, and which match a frame.

use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
    sync::Mutex,
};

use serde::{Deserialize, Serialize};

use super::{self as calibration, Masters, Mosaic};
use crate::imaging::{
    fits::{self, CardValue, RealImage},
    ImagingView,
};

pub const FORMAT: &str = "finder-calibration/1";
pub const BAD_PIXELS_FORMAT: &str = "finder-bad-pixels/1";
pub const SET_TEMP_TOLERANCE_C: f64 = 1.0;
pub const SENSOR_TEMP_TOLERANCE_C: f64 = 8.0;
pub const EXPOSURE_TOLERANCE_MS: f64 = 1.0;
pub const SETTING_TOLERANCE: f64 = 0.001;
pub const FLAT_ANGLE_WARN_DEG: f64 = 1.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Dark,
    Flat,
}

impl Kind {
    fn name(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Flat => "flat",
        }
    }

    fn folder(self) -> &'static str {
        match self {
            Self::Dark => "darks",
            Self::Flat => "flats",
        }
    }
}

/// The index's persistent entry. Every key is written, including absent values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MasterEntry {
    pub id: String,
    pub kind: Kind,
    /// Console Unix seconds when the master was made.
    pub made_at: f64,
    pub frames: u32,
    /// Whether the dark test judged the frames behind this master.
    pub checked: bool,
    pub model: String,
    pub serial: Option<String>,
    pub width: u32,
    pub height: u32,
    /// Origin on the sensor in binned pixels.
    pub origin_x: u32,
    pub origin_y: u32,
    pub bits: u32,
    pub bayer: Option<String>,
    pub gain: Option<f64>,
    pub offset: Option<f64>,
    pub binning: u32,
    pub set_temp_c: Option<f64>,
    /// Mean sensor temperature, absent when any contributing frame lacked it.
    pub temp_c: Option<f64>,
    pub exposure_ms: f64,
    pub level: Option<f64>,
    pub bad_pixels: Option<u64>,
    pub label: Option<String>,
    pub imaging_angle_deg: Option<f64>,
    pub level_share: Option<f64>,
    pub dark_frames: Option<u32>,
    /// Path relative to this library.
    pub file: String,
}

/// What a frame is matched by.
#[derive(Debug, Clone, PartialEq)]
pub struct FrameKey {
    pub model: String,
    pub serial: Option<String>,
    pub width: u32,
    pub height: u32,
    pub origin_x: u32,
    pub origin_y: u32,
    pub bits: u32,
    pub bayer: Option<String>,
    pub gain: Option<f64>,
    pub offset: Option<f64>,
    pub binning: u32,
    pub set_temp_c: Option<f64>,
    pub temp_c: Option<f64>,
    pub exposure_ms: f64,
}

impl From<&MasterEntry> for FrameKey {
    fn from(entry: &MasterEntry) -> Self {
        Self {
            model: entry.model.clone(),
            serial: entry.serial.clone(),
            width: entry.width,
            height: entry.height,
            origin_x: entry.origin_x,
            origin_y: entry.origin_y,
            bits: entry.bits,
            bayer: entry.bayer.clone(),
            gain: entry.gain,
            offset: entry.offset,
            binning: entry.binning,
            set_temp_c: entry.set_temp_c,
            temp_c: entry.temp_c,
            exposure_ms: entry.exposure_ms,
        }
    }
}

fn same(a: Option<f64>, b: Option<f64>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => (a - b).abs() <= SETTING_TOLERANCE,
        _ => false,
    }
}

impl FrameKey {
    /// Read required cards in the declared order, with the file's own exposure.
    pub fn from_image(image: &fits::FitsImage) -> Result<Self, String> {
        let card = |name| image.cards.iter().find(|(k, _)| k == name).map(|(_, v)| v);
        let text = |name| match card(name) {
            Some(CardValue::Text(v)) => Some(v.clone()),
            _ => None,
        };
        let number = |name| match card(name) {
            Some(CardValue::Real(v)) => Some(*v),
            Some(CardValue::Int(v)) => Some(*v as f64),
            _ => None,
        };
        let missing = |name| format!("the imaging frame's file has no {name} card");
        let model = text("INSTRUME").ok_or_else(|| missing("INSTRUME"))?;
        let exposure_ms = number("EXPTIME").ok_or_else(|| missing("EXPTIME"))? * 1000.0;
        let binning = number("XBINNING").ok_or_else(|| missing("XBINNING"))? as u32;
        let origin_x = number("XORGSUBF").ok_or_else(|| missing("XORGSUBF"))? as u32;
        let origin_y = number("YORGSUBF").ok_or_else(|| missing("YORGSUBF"))? as u32;
        Ok(Self {
            model,
            serial: text("SERIALNO"),
            width: image.width,
            height: image.height,
            origin_x,
            origin_y,
            bits: image.bits,
            bayer: text("BAYERPAT"),
            gain: number("GAIN"),
            offset: number("OFFSET"),
            binning,
            set_temp_c: number("SET-TEMP"),
            temp_c: number("CCD-TEMP"),
            exposure_ms,
        })
    }

    /// Use the saved frame's settings, including its own set temperature.
    pub fn from_view(view: &ImagingView) -> Option<Self> {
        let camera = view.camera.as_ref()?;
        let frame = view.frame.as_ref().filter(|f| f.state == "saved")?;
        let binning = frame.binning?;
        Some(Self {
            model: camera.model.clone(),
            serial: camera.serial.clone(),
            width: frame.width?,
            height: frame.height?,
            bits: frame.bits?,
            origin_x: frame.roi.x.checked_div(binning)?,
            origin_y: frame.roi.y.checked_div(binning)?,
            bayer: frame.bayer.clone(),
            gain: frame.gain,
            offset: frame.offset,
            binning,
            set_temp_c: frame.set_temp_c,
            temp_c: frame.temp_c,
            exposure_ms: frame.exposure_ms,
        })
    }

    fn fits(&self, other: &Self) -> bool {
        self.model == other.model
            && self.serial == other.serial
            && self.width == other.width
            && self.height == other.height
            && self.origin_x == other.origin_x
            && self.origin_y == other.origin_y
            && self.bits == other.bits
            && self.bayer == other.bayer
            && self.binning == other.binning
    }

    /// Hold a set's frames to its first settings and to the requested exposure.
    /// Sensor temperature may drift and is not part of agreement.
    pub fn agrees(&self, first: &FrameKey, exposure_ms: f64) -> bool {
        self.fits(first)
            && same(self.gain, first.gain)
            && same(self.offset, first.offset)
            && same(self.set_temp_c, first.set_temp_c)
            && (self.exposure_ms - exposure_ms).abs() <= EXPOSURE_TOLERANCE_MS
    }

    fn dark_distance(&self, dark: &Self) -> Option<f64> {
        let exposure_matches = (self.exposure_ms - dark.exposure_ms).abs() <= EXPOSURE_TOLERANCE_MS;
        if !self.fits(dark)
            || !same(self.gain, dark.gain)
            || !same(self.offset, dark.offset)
            || !exposure_matches
        {
            return None;
        }
        match (self.set_temp_c, dark.set_temp_c) {
            (Some(a), Some(b)) => ((a - b).abs() <= SET_TEMP_TOLERANCE_C).then_some((a - b).abs()),
            (None, None) => match (self.temp_c, dark.temp_c) {
                (Some(a), Some(b)) => {
                    ((a - b).abs() <= SENSOR_TEMP_TOLERANCE_C).then_some((a - b).abs())
                }
                _ => Some(0.0),
            },
            _ => None,
        }
    }
}

/// The masters selected for a frame, in the public serialization order.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Selection {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame: Option<u64>,
    pub dark: Option<String>,
    pub flat: Option<String>,
    pub bad_pixels: Option<u64>,
    pub missing: Vec<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flat_turned_deg: Option<f64>,
}

pub struct Applied {
    pub selection: Selection,
    pub report: calibration::Report,
    pub problems: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct Index {
    format: String,
    masters: Vec<MasterEntry>,
}

struct Held {
    index: Index,
    error: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct BadPixelFile {
    format: String,
    width: u32,
    height: u32,
    level: f32,
    spread: f32,
    sigmas: f32,
    pixels: Vec<(u32, u32)>,
}

#[derive(Default)]
struct ReadMasters {
    dark: Option<(String, RealImage)>,
    flat: Option<(String, RealImage)>,
    bad_pixels: Option<(String, BadPixelFile)>,
}

pub struct Library {
    root: PathBuf,
    held: Mutex<Held>,
    read: Mutex<ReadMasters>,
}

impl Library {
    /// A missing file is an empty library; an unreadable one is kept for recovery.
    pub fn open(root: PathBuf) -> Self {
        let loaded = match fs::read(root.join("index.json")) {
            Ok(bytes) => (|| {
                // Inspect the format before parsing entries of a future format.
                let value: serde_json::Value =
                    serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
                if let Some(format) = value.get("format").and_then(|v| v.as_str()) {
                    if format != FORMAT {
                        return Err(format!("its format is {format}"));
                    }
                }
                serde_json::from_value::<Index>(value)
                    .map(Some)
                    .map_err(|e| e.to_string())
            })(),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.to_string()),
        };
        let (index, error) = match loaded {
            Ok(Some(index)) => (index, None),
            result => (
                Index {
                    format: FORMAT.into(),
                    masters: Vec::new(),
                },
                result
                    .err()
                    .map(|why| format!("the calibration index could not be read: {why}")),
            ),
        };
        Self {
            root,
            held: Mutex::new(Held { index, error }),
            read: Mutex::new(ReadMasters::default()),
        }
    }

    pub fn error(&self) -> Option<String> {
        self.held.lock().expect("calibration index").error.clone()
    }

    /// Oldest first, with id as the stable tie break.
    pub fn masters(&self) -> Vec<MasterEntry> {
        let mut masters = self
            .held
            .lock()
            .expect("calibration index")
            .index
            .masters
            .clone();
        masters.sort_by(|a, b| a.made_at.total_cmp(&b.made_at).then(a.id.cmp(&b.id)));
        masters
    }

    pub fn new_id(&self, kind: Kind, unix_s: f64) -> String {
        let held = self.held.lock().expect("calibration index");
        let (_, date) = fits::utc(unix_s.floor());
        let second = date
            .split('.')
            .next()
            .unwrap_or(&date)
            .replace(['-', ':'], "");
        let base = format!("{}-{second}Z", kind.name());
        let mut id = base.clone();
        let mut suffix = 2;
        while held.index.masters.iter().any(|m| m.id == id)
            || self
                .root
                .join(kind.folder())
                .join(format!("{id}.fits"))
                .exists()
        {
            id = format!("{base}-{suffix}");
            suffix += 1;
        }
        id
    }

    /// Write a new master's FITS file before listing it. `add` makes no map.
    pub fn add(&self, mut entry: MasterEntry, samples: &[f32]) -> io::Result<MasterEntry> {
        let mut held = self.held.lock().expect("calibration index");
        if held.index.masters.iter().any(|m| m.id == entry.id) {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "a master has that id",
            ));
        }
        // IDs are one filename, even for callers other than the guarded routes.
        if entry.id.is_empty()
            || entry.id.contains(['/', '\\'])
            || entry.id == "."
            || entry.id == ".."
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "a master id must be one filename",
            ));
        }
        if samples.len() as u64 != u64::from(entry.width) * u64::from(entry.height) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "the samples do not fit the master's size",
            ));
        }
        entry.file = format!("{}/{}.fits", entry.kind.folder(), entry.id);
        fs::create_dir_all(self.root.join(entry.kind.folder()))?;
        let file = self.root.join(&entry.file);
        let part = file.with_extension("fits.part");
        let image = RealImage {
            width: entry.width,
            height: entry.height,
            samples: samples.to_vec(),
            cards: master_cards(&entry),
        };
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&part)?;
        let written = (|| {
            output.write_all(&fits::encode_real(&image))?;
            drop(output);
            if file.exists() {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "a master file has that name",
                ));
            }
            fs::rename(&part, &file)
        })();
        if let Err(error) = written {
            let _ = fs::remove_file(&part);
            return Err(error);
        }
        if held.error.is_some() {
            fs::rename(
                self.root.join("index.json"),
                self.root.join("index.json.unreadable"),
            )?;
        }
        held.index.masters.push(entry.clone());
        self.write_index(&held.index)?;
        held.error = None;
        Ok(entry)
    }

    /// Keep a listed dark's map beside it and record its size in the index.
    /// An oversized map leaves the entry and files untouched.
    pub fn attach_bad_pixels(
        &self,
        id: &str,
        map: &calibration::BadPixels,
    ) -> io::Result<MasterEntry> {
        let mut held = self.held.lock().expect("calibration index");
        let entry = held
            .index
            .masters
            .iter_mut()
            .find(|m| m.id == id && m.kind == Kind::Dark)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no master dark has that id"))?;
        if map.too_many {
            return Ok(entry.clone());
        }
        let file = self.root.join(&entry.file).with_extension("badpixels.json");
        let part = file.with_extension("json.part");
        let bytes = serde_json::to_vec(&BadPixelFile {
            format: BAD_PIXELS_FORMAT.into(),
            width: entry.width,
            height: entry.height,
            level: map.level,
            spread: map.spread,
            sigmas: calibration::BAD_PIXEL_SIGMAS,
            pixels: map.pixels.clone(),
        })?;
        let written = fs::write(&part, bytes).and_then(|()| fs::rename(&part, &file));
        if let Err(error) = written {
            let _ = fs::remove_file(&part);
            return Err(error);
        }
        entry.bad_pixels = Some(map.pixels.len() as u64);
        let entry = entry.clone();
        self.read.lock().expect("calibration masters").bad_pixels = None;
        self.write_index(&held.index)?;
        Ok(entry)
    }

    fn write_index(&self, index: &Index) -> io::Result<()> {
        let part = self.root.join("index.json.part");
        fs::write(&part, serde_json::to_vec_pretty(index)?)?;
        fs::rename(part, self.root.join("index.json"))
    }

    /// Unlist, save the index, then delete the master's files and forget the cache.
    pub fn remove(&self, id: &str) -> io::Result<bool> {
        let mut held = self.held.lock().expect("calibration index");
        let Some(position) = held.index.masters.iter().position(|m| m.id == id) else {
            return Ok(false);
        };
        let entry = held.index.masters.remove(position);
        self.write_index(&held.index)?;
        *self.read.lock().expect("calibration masters") = ReadMasters::default();
        for file in [
            self.root.join(&entry.file),
            self.root.join(&entry.file).with_extension("badpixels.json"),
        ] {
            match fs::remove_file(file) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
        Ok(true)
    }

    pub fn select(
        &self,
        key: &FrameKey,
        flat_label: Option<&str>,
        angle_deg: Option<f64>,
    ) -> Selection {
        let held = self.held.lock().expect("calibration index");
        let newest = |a: &MasterEntry, b: &MasterEntry| {
            a.made_at.total_cmp(&b.made_at).then(a.id.cmp(&b.id))
        };
        let dark = held
            .index
            .masters
            .iter()
            .filter(|m| m.kind == Kind::Dark)
            .filter_map(|m| key.dark_distance(&FrameKey::from(m)).map(|d| (m, d)))
            .min_by(|(a, da), (b, db)| da.total_cmp(db).then_with(|| newest(b, a)))
            .map(|(m, _)| m);
        let flat = held
            .index
            .masters
            .iter()
            .filter(|m| {
                m.kind == Kind::Flat
                    && key.fits(&FrameKey::from(*m))
                    && flat_label.is_none_or(|label| m.label.as_deref() == Some(label))
            })
            .max_by(|a, b| newest(a, b));
        let bad_pixels = dark.and_then(|m| m.bad_pixels);
        let flat_turned_deg = flat
            .and_then(|m| m.imaging_angle_deg)
            .zip(angle_deg)
            .map(|(then, now)| ((now - then + 180.0).rem_euclid(360.0) - 180.0).abs())
            .filter(|&d| d > FLAT_ANGLE_WARN_DEG);
        let missing = [
            ("dark", dark.is_none()),
            ("flat", flat.is_none()),
            ("bad_pixels", bad_pixels.is_none()),
        ]
        .into_iter()
        .filter_map(|(name, missing)| missing.then_some(name))
        .collect();
        Selection {
            frame: None,
            dark: dark.map(|m| m.id.clone()),
            flat: flat.map(|m| m.id.clone()),
            bad_pixels,
            missing,
            flat_turned_deg,
        }
    }

    /// Reads files on the caller's thread: runtime callers use the blocking pool.
    /// Only the last successfully read dark, flat and map are retained.
    pub fn calibrate(
        &self,
        key: &FrameKey,
        sub: &mut Mosaic,
        flat_label: Option<&str>,
        angle_deg: Option<f64>,
    ) -> Applied {
        let selection = self.select(key, flat_label, angle_deg);
        let entries = self.masters();
        let mut read = self.read.lock().expect("calibration masters");
        let ReadMasters {
            dark: dark_cache,
            flat: flat_cache,
            bad_pixels: map_cache,
        } = &mut *read;
        let mut problems = Vec::new();
        let dark = self.read_master(
            selection.dark.as_deref(),
            &entries,
            dark_cache,
            sub,
            &mut problems,
        );
        let flat = self.read_master(
            selection.flat.as_deref(),
            &entries,
            flat_cache,
            sub,
            &mut problems,
        );
        let bad_pixels = self.read_bad_pixels(
            selection.dark.as_deref(),
            &entries,
            map_cache,
            sub,
            &mut problems,
        );
        let report = calibration::calibrate(
            sub,
            &Masters {
                dark,
                flat,
                bad_pixels,
            },
        )
        .expect("the library checked the masters' sizes");
        Applied {
            selection,
            report,
            problems,
        }
    }

    fn read_bad_pixels<'a>(
        &self,
        id: Option<&str>,
        entries: &[MasterEntry],
        cache: &'a mut Option<(String, BadPixelFile)>,
        sub: &Mosaic,
        problems: &mut Vec<String>,
    ) -> Option<&'a [(u32, u32)]> {
        let entry = entries
            .iter()
            .find(|m| Some(m.id.as_str()) == id && m.bad_pixels.is_some())?;
        let result = (|| {
            if cache.as_ref().is_none_or(|(id, _)| id != &entry.id) {
                *cache = None;
                let bytes = fs::read(self.root.join(&entry.file).with_extension("badpixels.json"))
                    .map_err(|e| e.to_string())?;
                let value: serde_json::Value =
                    serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
                if let Some(format) = value.get("format").and_then(|v| v.as_str()) {
                    if format != BAD_PIXELS_FORMAT {
                        return Err(format!("its format is {format}"));
                    }
                }
                let map: BadPixelFile = serde_json::from_value(value).map_err(|e| e.to_string())?;
                *cache = Some((entry.id.clone(), map));
            }
            let map = &cache.as_ref().expect("bad-pixel map was read").1;
            if map.width != sub.width || map.height != sub.height {
                return Err(format!(
                    "it is {} × {} and the sub {} × {}",
                    map.width, map.height, sub.width, sub.height
                ));
            }
            Ok(())
        })();
        if let Err(why) = result {
            problems.push(format!(
                "the bad-pixel map of {} could not be used: {why}",
                entry.id
            ));
            return None;
        }
        Some(&cache.as_ref().expect("bad-pixel map was read").1.pixels)
    }

    fn read_master<'a>(
        &self,
        id: Option<&str>,
        entries: &[MasterEntry],
        cache: &'a mut Option<(String, RealImage)>,
        sub: &Mosaic,
        problems: &mut Vec<String>,
    ) -> Option<&'a [f32]> {
        let entry = entries.iter().find(|m| Some(m.id.as_str()) == id)?;
        let result = (|| {
            if cache.as_ref().is_none_or(|(id, _)| id != &entry.id) {
                *cache = None;
                let bytes = fs::read(self.root.join(&entry.file)).map_err(|e| e.to_string())?;
                let image = fits::decode_real(&bytes).map_err(|e| e.to_string())?;
                *cache = Some((entry.id.clone(), image));
            }
            let image = &cache.as_ref().expect("master was read").1;
            if image.width != sub.width || image.height != sub.height {
                return Err(format!(
                    "it is {} × {} and the sub {} × {}",
                    image.width, image.height, sub.width, sub.height
                ));
            }
            if image.samples.len() != sub.samples.len() {
                return Err(format!(
                    "it holds {} samples and the sub {}",
                    image.samples.len(),
                    sub.samples.len()
                ));
            }
            Ok(())
        })();
        if let Err(why) = result {
            problems.push(format!(
                "the master {} {} could not be used: {why}",
                entry.kind.name(),
                entry.id
            ));
            return None;
        }
        Some(&cache.as_ref().expect("master was read").1.samples)
    }
}

fn master_cards(entry: &MasterEntry) -> Vec<(String, CardValue)> {
    use CardValue::{Int, Logical, Real, Text};
    let mut cards = vec![
        (
            "IMAGETYP".into(),
            Text(
                match entry.kind {
                    Kind::Dark => "MASTER DARK",
                    Kind::Flat => "MASTER FLAT",
                }
                .into(),
            ),
        ),
        ("CALID".into(), Text(entry.id.clone())),
        ("NFRAMES".into(), Int(i64::from(entry.frames))),
    ];
    if entry.kind == Kind::Flat {
        cards.push((
            "NDARKS".into(),
            Int(i64::from(entry.dark_frames.unwrap_or(0))),
        ));
    }
    cards.extend([
        ("DARKTEST".into(), Logical(entry.checked)),
        ("EXPTIME".into(), Real(entry.exposure_ms / 1000.0)),
    ]);
    for (name, value) in [("GAIN", entry.gain), ("OFFSET", entry.offset)] {
        if let Some(value) = value {
            cards.push((name.into(), Real(value)));
        }
    }
    for (name, value) in [
        ("XBINNING", entry.binning),
        ("YBINNING", entry.binning),
        ("XORGSUBF", entry.origin_x),
        ("YORGSUBF", entry.origin_y),
    ] {
        cards.push((name.into(), Int(i64::from(value))));
    }
    for (name, value) in [("CCD-TEMP", entry.temp_c), ("SET-TEMP", entry.set_temp_c)] {
        if let Some(value) = value {
            cards.push((name.into(), Real(value)));
        }
    }
    if let Some(value) = &entry.bayer {
        cards.push(("BAYERPAT".into(), Text(value.clone())));
    }
    cards.push(("INSTRUME".into(), Text(entry.model.clone())));
    if let Some(value) = &entry.serial {
        cards.push(("SERIALNO".into(), Text(value.clone())));
    }
    cards.extend([
        ("ROWORDER".into(), Text("TOP-DOWN".into())),
        ("DATE".into(), Text(fits::utc(entry.made_at).1)),
    ]);
    if entry.kind == Kind::Flat {
        cards.push((
            "FLATLBL".into(),
            Text(entry.label.clone().unwrap_or_default()),
        ));
        if let Some(value) = entry.imaging_angle_deg {
            cards.push(("IMGANGLE".into(), Real(value)));
        }
    }
    cards
}
