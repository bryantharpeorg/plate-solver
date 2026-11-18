//! The solver port (032): how the console asks an imaging solver where a saved
//! imaging frame points, or how large its stars are, without knowing which
//! program answers.

use std::{future::Future, path::PathBuf, pin::Pin};

use serde::{ser::SerializeMap, Deserialize, Serialize, Serializer};

/// A rectangle in pixels: its top-left corner, then its size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Region {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// 031's view of a region of interest has the same four numbers.
impl From<super::RoiView> for Region {
    fn from(roi: super::RoiView) -> Self {
        Self {
            x: roi.x,
            y: roi.y,
            width: roi.width,
            height: roi.height,
        }
    }
}

/// A saved imaging frame as a solver adapter needs it.
#[derive(Debug, Clone, PartialEq)]
pub struct SolveFrame {
    /// The saved FITS file. The simulated adapter does not open it.
    pub path: PathBuf,
    /// Delivered samples across and down.
    pub width: u32,
    pub height: u32,
    pub bits: u32,
    /// `"RGGB"`, `"BGGR"`, `"GRBG"`, `"GBRG"`, or `None` for mono.
    pub bayer: Option<String>,
    pub binning: u32,
    /// The region of the sensor the frame covers, in sensor pixels.
    pub roi: Region,
    /// The exposure's start in console Unix seconds, and its length in seconds.
    pub start: f64,
    pub exposure_s: f64,
}

/// Where the hint's place came from (032 US3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HintSource {
    Pointing,
    LastSolve,
    Target,
    None,
}

/// Where the solver is told to look.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SolveHint {
    pub source: HintSource,
    /// J2000 degrees; `None` for a search of the whole sky.
    pub position: Option<(f64, f64)>,
    pub radius_deg: f64,
    /// The imaging frame's height on the sky, when known.
    pub fov_deg: Option<f64>,
}

impl SolveHint {
    /// No place, 180°, no field height.
    pub fn whole_sky() -> Self {
        Self {
            source: HintSource::None,
            position: None,
            radius_deg: 180.0,
            fov_deg: None,
        }
    }
}

/// Written by hand, not derived: `position` becomes the two keys `ra_deg` and
/// `dec_deg`, and the place and the field height are left out when absent.
impl Serialize for SolveHint {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let entries =
            2 + usize::from(self.position.is_some()) + usize::from(self.fov_deg.is_some());
        let mut map = serializer.serialize_map(Some(entries))?;
        map.serialize_entry("source", &self.source)?;
        if let Some((ra_deg, dec_deg)) = self.position {
            map.serialize_entry("ra_deg", &ra_deg)?;
            map.serialize_entry("dec_deg", &dec_deg)?;
        }
        map.serialize_entry("radius_deg", &self.radius_deg)?;
        if let Some(fov_deg) = self.fov_deg {
            map.serialize_entry("fov_deg", &fov_deg)?;
        }
        map.end()
    }
}

/// What a solver said about a picture, in the picture's own FITS pixels: column
/// and row counted from 1, rows in the file's order. Degrees.
#[derive(Debug, Clone, PartialEq)]
pub struct Answer {
    pub crpix: [f64; 2],
    pub crval: [f64; 2],
    /// `[[CD1_1, CD1_2], [CD2_1, CD2_2]]`.
    pub cd: [[f64; 2]; 2],
    pub warning: Option<String>,
}

/// How a prepared copy sits in its imaging frame: copy pixel (i, j) covers the
/// frame's pixels from (x0 + k·i, y0 + k·j), k on a side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Prepared {
    pub x0: u32,
    pub y0: u32,
    pub k: u32,
    pub width: u32,
    pub height: u32,
}

/// Where an imaging frame points, in the imaging frame's own pixels.
#[derive(Debug, Clone, PartialEq)]
pub struct ImagingSolve {
    /// J2000 degrees of the imaging frame's centre; RA in 0 to 360.
    pub ra_deg: f64,
    pub dec_deg: f64,
    /// The angle of frame-up east of celestial north, 0 to 360.
    pub up_east_of_north_deg: f64,
    /// Per delivered sample.
    pub arcsec_per_px: f64,
    pub mirrored: bool,
    /// Degrees east (row 0) and north (row 1) for one pixel right (column 0)
    /// and one pixel down (column 1).
    pub cd_deg: [[f64; 2]; 2],
    pub width: u32,
    pub height: u32,
    /// Matched stars, when the solver says.
    pub stars: Option<u32>,
    pub took_ms: f64,
    pub warning: Option<String>,
}

impl ImagingSolve {
    /// An imaging solve from its centre and its linear part, deriving the angle,
    /// the scale and the mirror sense as FR-002 works them. `stars` and
    /// `warning` are `None` and `took_ms` 0.
    pub fn from_cd(
        ra_deg: f64,
        dec_deg: f64,
        cd_deg: [[f64; 2]; 2],
        width: u32,
        height: u32,
    ) -> Self {
        let det = cd_deg[0][0] * cd_deg[1][1] - cd_deg[0][1] * cd_deg[1][0];
        Self {
            ra_deg: ra_deg.rem_euclid(360.0),
            dec_deg,
            up_east_of_north_deg: (-cd_deg[0][1])
                .atan2(-cd_deg[1][1])
                .to_degrees()
                .rem_euclid(360.0),
            arcsec_per_px: 3600.0 * det.abs().sqrt(),
            mirrored: det < 0.0,
            cd_deg,
            width,
            height,
            stars: None,
            took_ms: 0.0,
            warning: None,
        }
    }

    /// From a solver's answer about a prepared copy of a frame `width` ×
    /// `height`: the linear part for the frame's own pixels, the frame's centre
    /// from the answer's reference pixel, and the answer's warning.
    pub fn from_answer(answer: &Answer, copy: Prepared, width: u32, height: u32) -> Self {
        let k = f64::from(copy.k);
        let cd_deg = [
            [answer.cd[0][0] / k, answer.cd[0][1] / k],
            [answer.cd[1][0] / k, answer.cd[1][1] / k],
        ];
        // The frame's centre in the copy's FITS pixels.
        let x = (f64::from(width) / 2.0 - f64::from(copy.x0)) / k + 0.5;
        let y = (f64::from(height) / 2.0 - f64::from(copy.y0)) / k + 0.5;
        let (east, north) = (
            answer.cd[0][0] * (x - answer.crpix[0]) + answer.cd[0][1] * (y - answer.crpix[1]),
            answer.cd[1][0] * (x - answer.crpix[0]) + answer.cd[1][1] * (y - answer.crpix[1]),
        );
        let (ra, dec) = to_sky(
            answer.crval[0].to_radians(),
            answer.crval[1].to_radians(),
            east.to_radians(),
            north.to_radians(),
        );
        let mut solve = Self::from_cd(
            ra.to_degrees().rem_euclid(360.0),
            dec.to_degrees(),
            cd_deg,
            width,
            height,
        );
        solve.warning = answer.warning.clone();
        solve
    }

    /// J2000 degrees of an imaging-frame position.
    pub fn sky_at(&self, pixel: (f64, f64)) -> (f64, f64) {
        let (dx, dy) = (
            pixel.0 - f64::from(self.width) / 2.0,
            pixel.1 - f64::from(self.height) / 2.0,
        );
        let (east, north) = (
            (self.cd_deg[0][0] * dx + self.cd_deg[0][1] * dy).to_radians(),
            (self.cd_deg[1][0] * dx + self.cd_deg[1][1] * dy).to_radians(),
        );
        let (ra, dec) = to_sky(
            self.ra_deg.to_radians(),
            self.dec_deg.to_radians(),
            east,
            north,
        );
        (ra.to_degrees().rem_euclid(360.0), dec.to_degrees())
    }

    /// The imaging-frame position of a place; `None` a quarter turn or more
    /// away.
    pub fn pixel_of(&self, ra_deg: f64, dec_deg: f64) -> Option<(f64, f64)> {
        let (east, north) = from_sky(
            self.ra_deg.to_radians(),
            self.dec_deg.to_radians(),
            ra_deg.to_radians(),
            dec_deg.to_radians(),
        )?;
        let (east, north) = (east.to_degrees(), north.to_degrees());
        let det = self.cd_deg[0][0] * self.cd_deg[1][1] - self.cd_deg[0][1] * self.cd_deg[1][0];
        let (dx, dy) = (
            (self.cd_deg[1][1] * east - self.cd_deg[0][1] * north) / det,
            (self.cd_deg[0][0] * north - self.cd_deg[1][0] * east) / det,
        );
        Some((
            dx + f64::from(self.width) / 2.0,
            dy + f64::from(self.height) / 2.0,
        ))
    }
}

/// The tangent plane about (α0, δ0) to the sky: (ξ, η) east and north in radians
/// become a place.
fn to_sky(ra0: f64, dec0: f64, east: f64, north: f64) -> (f64, f64) {
    let cos = dec0.cos() - north * dec0.sin();
    let ra = ra0 + east.atan2(cos);
    let dec = (dec0.sin() + north * dec0.cos()).atan2(east.hypot(cos));
    (ra, dec)
}

/// The tangent plane about (α0, δ0) from the sky: a place becomes (ξ, η) east
/// and north in radians, `None` a quarter turn or more away.
fn from_sky(ra0: f64, dec0: f64, ra: f64, dec: f64) -> Option<(f64, f64)> {
    let cos = dec0.sin() * dec.sin() + dec0.cos() * dec.cos() * (ra - ra0).cos();
    if cos <= 0.0 {
        return None;
    }
    let east = dec.cos() * (ra - ra0).sin() / cos;
    let north = (dec0.cos() * dec.sin() - dec0.sin() * dec.cos() * (ra - ra0).cos()) / cos;
    Some((east, north))
}

/// The median half-flux diameter in imaging-frame pixels, and the star count.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StarMeasure {
    pub half_flux_px: f64,
    pub stars: u32,
}

/// Why a solve or a star measure did not answer one.
#[derive(Debug, Clone, PartialEq)]
pub enum SolveError {
    NotSolved { detail: Option<String> },
    TooFewStars { detail: Option<String> },
    NoDatabase { detail: Option<String> },
    TimedOut { seconds: u64 },
    Failed(String),
}

impl SolveError {
    /// The reason as the imaging view and the solve sidecar write it.
    pub fn reason(&self) -> &'static str {
        match self {
            Self::NotSolved { .. } => "not_solved",
            Self::TooFewStars { .. } => "too_few_stars",
            Self::NoDatabase { .. } => "no_database",
            Self::TimedOut { .. } => "timed_out",
            Self::Failed(_) => "failed",
        }
    }

    /// The words the page shows.
    pub fn message(&self) -> String {
        match self {
            Self::NotSolved { .. } => "the imaging solver found no solution".to_string(),
            Self::TooFewStars { .. } => {
                "the imaging solver found too few stars in the imaging frame".to_string()
            }
            Self::NoDatabase { .. } => {
                "the imaging solver has no star database it can read".to_string()
            }
            Self::TimedOut { seconds } => {
                format!("the imaging solver gave no answer within {seconds} s")
            }
            Self::Failed(text) => text.clone(),
        }
    }

    /// The solver's own words for what went wrong, when it gave any.
    pub fn detail(&self) -> Option<&str> {
        match self {
            Self::NotSolved { detail }
            | Self::TooFewStars { detail }
            | Self::NoDatabase { detail } => detail.as_deref(),
            Self::TimedOut { .. } | Self::Failed(_) => None,
        }
    }
}

/// One solve in flight.
pub type SolveFuture<'a> =
    Pin<Box<dyn Future<Output = Result<ImagingSolve, SolveError>> + Send + 'a>>;
/// One star measure in flight.
pub type MeasureFuture<'a> =
    Pin<Box<dyn Future<Output = Result<StarMeasure, SolveError>> + Send + 'a>>;

/// The solver port. A solver adapter speaks one imaging solver's own interface,
/// and no type here names a solver, a camera model or a mount.
pub trait ImagingSolver: Send + Sync {
    /// `astap` or `simulated`.
    fn name(&self) -> &'static str;
    fn solve(&self, frame: SolveFrame, hint: SolveHint) -> SolveFuture<'_>;
    /// `region` is in the frame's pixels; `None` is the whole frame.
    fn measure(&self, frame: SolveFrame, region: Option<Region>) -> MeasureFuture<'_>;
}
