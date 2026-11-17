//! The imaging side of the camera port (030): one exposure on request.
use std::{fmt, future::Future, pin::Pin};

pub mod synthetic;

/// A colour sensor's mosaic, named by its top-left 2 × 2 cell row by row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BayerPattern {
    /// Red, green, green, blue.
    Rggb,
    /// Blue, green, green, red.
    Bggr,
    /// Green, red on the first row; green, blue on the second.
    Grbg,
    /// Green, blue on the first row; green, red on the second.
    Gbrg,
}

impl BayerPattern {
    /// `"RGGB"`, `"BGGR"`, `"GRBG"` or `"GBRG"`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Rggb => "RGGB",
            Self::Bggr => "BGGR",
            Self::Grbg => "GRBG",
            Self::Gbrg => "GBRG",
        }
    }

    /// The inverse, ignoring case and surrounding spaces; anything else is `None`.
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_uppercase().as_str() {
            "RGGB" => Some(Self::Rggb),
            "BGGR" => Some(Self::Bggr),
            "GRBG" => Some(Self::Grbg),
            "GBRG" => Some(Self::Gbrg),
            _ => None,
        }
    }
}

/// An inclusive range of a control's values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Range {
    pub min: f64,
    pub max: f64,
}

/// A region of the sensor in unbinned pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Roi {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// What a camera says about itself. Nothing here comes from a model's name.
#[derive(Clone, Debug, PartialEq)]
pub struct ImagingInfo {
    pub model: String,
    pub serial: Option<String>,
    pub width: u32,
    pub height: u32,
    pub pixel_um: f64,
    /// Bits a sample as delivered: 8 or 16.
    pub bits: u32,
    /// `None` for a mono sensor.
    pub bayer: Option<BayerPattern>,
    pub exposure_ms: Range,
    /// `None` when the camera has no such control.
    pub gain: Option<Range>,
    /// `None` when the camera has no such control.
    pub offset: Option<Range>,
    pub binnings: Vec<u32>,
    pub cooler: bool,
}

/// In a request, an absent field leaves that setting alone. In an answer every
/// field the camera has is present; `gain` and `offset` are `None` only without
/// the control, `set_temp_c` only before one was set.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ImagingSettings {
    pub gain: Option<f64>,
    pub offset: Option<f64>,
    pub binning: Option<u32>,
    pub roi: Option<Roi>,
    pub cooler_on: Option<bool>,
    pub set_temp_c: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImagingStatus {
    pub temp_c: Option<f64>,
    /// Percent.
    pub cooler_power: Option<f64>,
    pub set_temp_c: Option<f64>,
    pub exposing: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Exposure {
    pub exposure_ms: f64,
    pub purpose: String,
}

/// One exposure, its samples exactly as the camera delivered them. `Default`
/// exists so that a test's literal ends with `..Default::default()` and a later
/// field breaks none.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ImagingFrame {
    /// Little-endian, `width × height × bits / 8` bytes, row by row from the
    /// top-left.
    pub samples: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub bits: u32,
    pub bayer: Option<BayerPattern>,
    /// Seconds on the node's clock.
    pub start_s: f64,
    pub end_s: f64,
    pub exposure_ms: f64,
    pub gain: Option<f64>,
    pub offset: Option<f64>,
    pub binning: u32,
    pub temp_c: Option<f64>,
    /// The region of interest in force for the exposure, in unbinned sensor
    /// pixels (the plan document's Amendment 6). The frame's own `width` and
    /// `height` are what it delivered; this says where on the sensor it sat.
    pub roi: Roi,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ImagingError {
    /// The request names a value the camera does not accept; nothing changed.
    Invalid(String),
    /// An exposure is in progress. Displays as `an exposure is already in
    /// progress`.
    Busy,
    /// The exposure was aborted on request. Displays as `the exposure was
    /// aborted`.
    Aborted,
    /// The camera or its server failed; the text says how.
    Failed(String),
}

impl fmt::Display for ImagingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(text) | Self::Failed(text) => f.write_str(text),
            Self::Busy => f.write_str("an exposure is already in progress"),
            Self::Aborted => f.write_str("the exposure was aborted"),
        }
    }
}
impl std::error::Error for ImagingError {}

/// Every method answers a boxed future, so any adapter works behind the port.
pub type ImagingFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, ImagingError>> + Send + 'a>>;

/// Every method takes `&self`, so `status` and `abort` work while `expose`
/// waits.
pub trait ImagingCamera: Send + Sync {
    fn describe(&self) -> ImagingFuture<'_, ImagingInfo>;
    fn status(&self) -> ImagingFuture<'_, ImagingStatus>;
    /// Change the settings present; answer all the settings in force.
    fn apply<'a>(&'a self, settings: &'a ImagingSettings) -> ImagingFuture<'a, ImagingSettings>;
    fn expose<'a>(&'a self, exposure: &'a Exposure) -> ImagingFuture<'a, ImagingFrame>;
    fn abort(&self) -> ImagingFuture<'_, ()>;
}

/// `edit` laid over `in_force` and checked against `info`, answering the merged
/// settings. The first failing rule, in the order below, answers
/// [`ImagingError::Invalid`] with its text:
///
/// 1. `gain` present: without the control, `this camera has no gain control`;
///    not finite or outside the range, `gain {value} is outside the camera's
///    range {min}–{max}`;
/// 2. `offset`: the same two texts with `offset`;
/// 3. `binning` present and not one of the camera's;
/// 4. when the edit names `roi` or `binning`: the region that will be in
///    force, the edit's or the one in force, against the binning that will be
///    in force. An edit that names neither is never refused for the region in
///    force, which another INDI client may have set;
/// 5. `cooler_on` on or `set_temp_c` present on a camera without a cooler;
/// 6. `set_temp_c` not finite or outside −50 to 50.
pub fn check_settings(
    info: &ImagingInfo,
    in_force: &ImagingSettings,
    edit: &ImagingSettings,
) -> Result<ImagingSettings, ImagingError> {
    let merged = ImagingSettings {
        gain: edit.gain.or(in_force.gain),
        offset: edit.offset.or(in_force.offset),
        binning: edit.binning.or(in_force.binning),
        roi: edit.roi.or(in_force.roi),
        cooler_on: edit.cooler_on.or(in_force.cooler_on),
        set_temp_c: edit.set_temp_c.or(in_force.set_temp_c),
    };
    for (name, value, range) in [
        ("gain", edit.gain, &info.gain),
        ("offset", edit.offset, &info.offset),
    ] {
        if let Some(value) = value {
            match range {
                None => {
                    return Err(ImagingError::Invalid(format!(
                        "this camera has no {name} control"
                    )))
                }
                Some(range) if !value.is_finite() || value < range.min || value > range.max => {
                    return Err(ImagingError::Invalid(format!(
                        "{name} {value} is outside the camera's range {}–{}",
                        range.min, range.max
                    )))
                }
                Some(_) => {}
            }
        }
    }
    if let Some(binning) = edit.binning {
        if !info.binnings.contains(&binning) {
            let list = info
                .binnings
                .iter()
                .map(|binning| binning.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(ImagingError::Invalid(format!(
                "binning {binning} is not one of the camera's binnings {list}"
            )));
        }
    }
    if edit.roi.is_some() || edit.binning.is_some() {
        let binning = merged.binning.unwrap_or(1).max(1);
        if let Some(roi) = merged.roi {
            let step = 2 * binning;
            let inside = roi.x % step == 0
                && roi.y % step == 0
                && roi.width >= binning
                && roi.height >= binning
                && u64::from(roi.x) + u64::from(roi.width) <= u64::from(info.width)
                && u64::from(roi.y) + u64::from(roi.height) <= u64::from(info.height);
            if !inside {
                return Err(ImagingError::Invalid(format!(
                    "roi must lie inside the {} × {} sensor with x and y multiples of {step}",
                    info.width, info.height
                )));
            }
        }
    }
    if !info.cooler && (edit.cooler_on == Some(true) || edit.set_temp_c.is_some()) {
        return Err(ImagingError::Invalid(
            "this camera has no cooler".to_string(),
        ));
    }
    if let Some(temp) = edit.set_temp_c {
        if !temp.is_finite() || !(-50.0..=50.0).contains(&temp) {
            return Err(ImagingError::Invalid(
                "set_temp_c must be between -50 and 50".to_string(),
            ));
        }
    }
    Ok(merged)
}

/// The port's exposure refusals: `exposure_ms` not finite or outside the
/// camera's range, and a purpose that is not 1 to 16 lower-case letters.
pub fn check_exposure(info: &ImagingInfo, exposure: &Exposure) -> Result<(), ImagingError> {
    let ms = exposure.exposure_ms;
    if !ms.is_finite() || ms < info.exposure_ms.min || ms > info.exposure_ms.max {
        return Err(ImagingError::Invalid(format!(
            "exposure_ms {ms} is outside the camera's range {}–{} ms",
            info.exposure_ms.min, info.exposure_ms.max
        )));
    }
    let purpose = exposure.purpose.as_bytes();
    let labelled =
        (1..=16).contains(&purpose.len()) && purpose.iter().all(|byte| byte.is_ascii_lowercase());
    if !labelled {
        return Err(ImagingError::Invalid(
            "purpose must be 1 to 16 lower-case letters".to_string(),
        ));
    }
    Ok(())
}
