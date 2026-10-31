//! One solve, end to end: decode a frame, detect stars, solve, and build the report the
//! page draws, with the matched stars and the deep-sky objects placed on the frame.

use std::collections::HashSet;
use std::time::Instant;

use math_core::{angular_distance, distort_centroids};
use pattern_database::{CatalogId, PatternDatabase, StarId};
use plate_solver::{solve_from_centroids, DetectParams, Solution, SolveStatus};
use serde::Serialize;
use star_detection::{detect_stars, gray};

use crate::catalog::{self, DeepSkyObject};
use crate::constellation::{self, Constellation};
use crate::grid::{graticule, GridLine};
use crate::sky::{format_dec, format_ra, Frame};
use crate::stars;

/// Largest image side accepted, in pixels: far above any finder camera, low enough to
/// refuse a small file that claims an enormous image.
pub const MAX_IMAGE_SIDE: u32 = 20_000;
/// Most memory the decoder may allocate for one frame.
const MAX_DECODE_BYTES: u64 = 256 * 1024 * 1024;
/// The solver's match settings, as in the `solve_image` example.
const MATCH_RADIUS: f64 = 0.01;
const MATCH_THRESHOLD: f64 = 1e-5;
const MATCH_MAX_ERROR: f64 = 0.002;
/// Detected stars reported, brightest first.
const MAX_DETECTED: usize = 500;
/// Unmatched catalog stars drawn as "expected here", brightest first.
const MAX_EXPECTED_STARS: usize = 25;
/// How far beyond the frame's corners a deep-sky object counts as nearby, in degrees.
const NEARBY_DEG: f64 = 15.0;
/// Nearby objects reported, nearest first.
const MAX_NEARBY: usize = 8;

/// How to detect and solve; the defaults are the `solve_image` example's. Detection itself
/// runs as cedar's does ([`DetectParams::cedar`]), so cedar's frames solve to cedar's answers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SolveOptions {
    /// Horizontal field of view estimate in degrees.
    pub fov: Option<f64>,
    /// How far the true field of view may be from `fov`, in degrees.
    pub fov_max_error: f64,
    /// Radial distortion `k` to start from (negative is barrel).
    pub distortion: f64,
    /// Binning before detection: 1, 2, 4 or 8.
    pub binning: usize,
    /// Detection significance threshold.
    pub sigma: f64,
    /// How many of the brightest detected stars go to the solver.
    pub stars: usize,
    /// Solve timeout in milliseconds.
    pub timeout_ms: u64,
}

impl Default for SolveOptions {
    fn default() -> Self {
        let cedar = DetectParams::cedar();
        Self {
            fov: None,
            fov_max_error: 10.0,
            distortion: 0.0,
            binning: cedar.binning,
            sigma: cedar.sigma,
            stars: 30,
            timeout_ms: 5000,
        }
    }
}

/// An 8-bit grayscale frame, row-major.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrayImage {
    /// Pixels, `width * height` of them.
    pub pixels: Vec<u8>,
    /// Width in pixels.
    pub width: usize,
    /// Height in pixels.
    pub height: usize,
}

/// Why a frame could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// Larger than [`MAX_IMAGE_SIDE`] or the decoder's memory budget.
    TooLarge(String),
    /// Not an image this build reads (PNG, JPEG, BMP, PNM), or damaged.
    Unreadable(String),
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooLarge(e) => write!(f, "the image is too large: {e}"),
            Self::Unreadable(e) => write!(f, "the file is not a readable image: {e}"),
        }
    }
}

fn decode(bytes: &[u8]) -> Result<image::DynamicImage, DecodeError> {
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| DecodeError::Unreadable(e.to_string()))?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_IMAGE_SIDE);
    limits.max_image_height = Some(MAX_IMAGE_SIDE);
    limits.max_alloc = Some(MAX_DECODE_BYTES);
    reader.limits(limits);
    reader.decode().map_err(|e| match e {
        image::ImageError::Limits(e) => DecodeError::TooLarge(e.to_string()),
        e => DecodeError::Unreadable(e.to_string()),
    })
}

/// A decoded frame in 8-bit gray. Color frames are converted the way cedar converts them
/// ([`star_detection::gray`], PIL's weights), not with `image`'s Rec. 709 weights: those move
/// faint stars, and then a color frame no longer solves to cedar's answer.
fn to_gray(image: &image::DynamicImage) -> image::GrayImage {
    if !image.color().has_color() {
        return image.to_luma8();
    }
    let (width, height) = (image.width(), image.height());
    let pixels = match image {
        image::DynamicImage::ImageRgb8(rgb) => gray::from_rgb8(rgb.as_raw()),
        other => gray::from_rgb8(other.to_rgb8().as_raw()),
    };
    image::GrayImage::from_raw(width, height, pixels).expect("one gray pixel per pixel")
}

/// Decode a frame to 8-bit grayscale, refusing oversized or unreadable files.
pub fn decode_gray(bytes: &[u8]) -> Result<GrayImage, DecodeError> {
    let gray = to_gray(&decode(bytes)?);
    let (width, height) = gray.dimensions();
    Ok(GrayImage {
        pixels: gray.into_raw(),
        width: width as usize,
        height: height as usize,
    })
}

/// A JPEG preview of a frame, at most `max_side` pixels on its longer side, stretched so
/// its stars show at that size.
pub fn thumbnail_jpeg(bytes: &[u8], max_side: u32) -> Result<Vec<u8>, DecodeError> {
    let gray = to_gray(&decode(bytes)?);
    let (width, height) = gray.dimensions();
    // Shrink by keeping each block's brightest pixel: averaging would dim the stars into
    // the sky, which is all a small preview of a star field has to show.
    let block = width.max(height).div_ceil(max_side.max(1)).max(1);
    let mut small = image::GrayImage::new(width.div_ceil(block), height.div_ceil(block));
    for (x, y, p) in gray.enumerate_pixels() {
        let q = small.get_pixel_mut(x / block, y / block);
        q.0[0] = q.0[0].max(p.0[0]);
    }
    stretch(&mut small);
    let mut out = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageLuma8(small)
        .write_to(&mut out, image::ImageFormat::Jpeg)
        .map_err(|e| DecodeError::Unreadable(e.to_string()))?;
    Ok(out.into_inner())
}

/// Stretch a star field for a preview: from just above the sky background (the 60th
/// percentile) to the brightest stars (the 99.9th), midtones lifted. The range never drops
/// below 24 levels, so a frame with no stars keeps its noise dark instead of blown up.
fn stretch(image: &mut image::GrayImage) {
    let mut levels = image.as_raw().clone();
    if levels.is_empty() {
        return;
    }
    levels.sort_unstable();
    let at = |q: f64| f64::from(levels[((levels.len() - 1) as f64 * q) as usize]);
    let lo = at(0.6);
    let hi = at(0.999).max(lo + 24.0);
    for p in image.pixels_mut() {
        let v = ((f64::from(p.0[0]) - lo) / (hi - lo))
            .clamp(0.0, 1.0)
            .powf(0.7);
        p.0[0] = (v * 255.0).round() as u8;
    }
}

/// How a solve ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// Solved and verified.
    MatchFound,
    /// No pattern in the frame matched the database.
    NoMatch,
    /// Out of time.
    Timeout,
    /// Stopped by the caller.
    Cancelled,
    /// Too few stars to try.
    TooFew,
}

impl Status {
    fn of(status: Option<SolveStatus>) -> Self {
        match status {
            Some(SolveStatus::MatchFound) => Self::MatchFound,
            Some(SolveStatus::Timeout) => Self::Timeout,
            Some(SolveStatus::Cancelled) => Self::Cancelled,
            Some(SolveStatus::TooFew) => Self::TooFew,
            Some(SolveStatus::NoMatch) | None => Self::NoMatch,
        }
    }

    /// What to try next, for anything but a match.
    pub fn hint(self) -> Option<&'static str> {
        match self {
            Self::MatchFound => None,
            Self::NoMatch => Some(
                "No star pattern in this frame matched the database. Check that the database \
                 covers this camera's field of view, or lower the detection threshold.",
            ),
            Self::Timeout => Some(
                "The solve ran out of time. Give it a field of view estimate or a longer timeout.",
            ),
            Self::Cancelled => Some("The solve was cancelled."),
            Self::TooFew => Some(
                "Too few stars were found to try a solve; it needs at least four. Lower the \
                 detection threshold or use a longer exposure.",
            ),
        }
    }
}

/// Where the frame points, and how well the solve fits.
#[derive(Debug, Clone, Serialize)]
pub struct Pointing {
    /// J2000 right ascension of the image center, degrees.
    pub ra_deg: f64,
    /// J2000 declination of the image center, degrees.
    pub dec_deg: f64,
    /// Right ascension as `05h 40m 46s`.
    pub ra_text: String,
    /// Declination as `−04° 14′ 46″`.
    pub dec_text: String,
    /// The constellation at the image center.
    pub constellation: Constellation,
    /// Roll, degrees.
    pub roll_deg: f64,
    /// Where celestial north points in the image, degrees clockwise from up.
    pub north_deg: f64,
    /// Horizontal field of view, degrees.
    pub fov_deg: f64,
    /// Vertical field of view, degrees.
    pub fov_vertical_deg: f64,
    /// Image scale at the center, arcseconds per pixel.
    pub arcsec_per_px: f64,
    /// Fitted radial distortion `k` (negative is barrel).
    pub distortion: f64,
    /// Matched catalog stars.
    pub matched: usize,
    /// Residual over the matched stars, arcseconds: RMS, 90th percentile and largest.
    pub rmse_arcsec: Option<f64>,
    /// See `rmse_arcsec`.
    pub p90_arcsec: Option<f64>,
    /// See `rmse_arcsec`.
    pub max_arcsec: Option<f64>,
    /// Probability that a match this good is chance.
    pub false_alarm: Option<f64>,
}

/// A catalog star placed on the frame.
#[derive(Debug, Clone, Serialize)]
pub struct StarMark {
    /// Image position: where it was detected when matched, else where it should be.
    pub x: f64,
    /// See `x`.
    pub y: f64,
    /// Catalog designation, as `"HIP 26727"`.
    pub id: String,
    /// Proper name, as `"Alnitak"`, when it has one.
    pub name: Option<&'static str>,
    /// Catalog magnitude.
    pub mag: f64,
    /// Whether the solve matched it to a detected star.
    pub matched: bool,
    /// For a matched star, pixels between where it was detected and where the solution
    /// puts it: the fit, star by star.
    pub error_px: Option<f64>,
}

/// A deep-sky object on or overlapping the frame.
#[derive(Debug, Clone, Serialize)]
pub struct ObjectMark {
    /// The object.
    #[serde(flatten)]
    pub object: DeepSkyObject,
    /// Image position of its center (it may lie just outside the frame).
    pub x: f64,
    /// See `x`.
    pub y: f64,
    /// Half its major axis, in pixels.
    pub radius_px: f64,
    /// Half its minor axis, in pixels (the major one when the minor is unknown).
    pub minor_px: f64,
    /// Which way its major axis points in the image, degrees clockwise from up.
    pub angle_deg: f64,
    /// Distance from the frame center, degrees.
    pub separation_deg: f64,
    /// Whether its center is inside the frame.
    pub inside: bool,
}

/// A deep-sky object near, but off, the frame.
#[derive(Debug, Clone, Serialize)]
pub struct NearbyMark {
    /// The object.
    #[serde(flatten)]
    pub object: DeepSkyObject,
    /// Distance from the frame center, degrees.
    pub separation_deg: f64,
    /// Which way it lies in the image, degrees clockwise from up.
    pub direction_deg: f64,
}

/// Milliseconds spent in each step.
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct Timing {
    /// Decoding the file, when the caller measured it.
    pub decode: f64,
    /// Star detection.
    pub detect: f64,
    /// The solve.
    pub solve: f64,
}

/// Everything the page draws for one frame.
#[derive(Debug, Clone, Serialize)]
pub struct SolveReport {
    /// The frame's file name, when known.
    pub name: Option<String>,
    /// How the solve ended.
    pub status: Status,
    /// What to try next, when it did not solve.
    pub hint: Option<&'static str>,
    /// Frame width in pixels.
    pub width: usize,
    /// Frame height in pixels.
    pub height: usize,
    /// Time per step.
    pub timing_ms: Timing,
    /// Detected star centroids `[x, y]`, brightest first.
    pub detected: Vec<[f64; 2]>,
    /// How many of the brightest went to the solver.
    pub used: usize,
    /// Where it points, when it solved.
    pub pointing: Option<Pointing>,
    /// Session sequence assigned when this solve records the latest pointing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pointing_seq: Option<u64>,
    /// Matched catalog stars, then the brightest unmatched ones that should be in frame.
    pub stars: Vec<StarMark>,
    /// Deep-sky objects on or overlapping the frame: those centered in it first, brightest
    /// first.
    pub objects: Vec<ObjectMark>,
    /// Deep-sky objects just off the frame, nearest first.
    pub nearby: Vec<NearbyMark>,
    /// The RA/Dec grid over the frame, when it solved.
    pub grid: Vec<GridLine>,
}

fn ms(since: Instant) -> f64 {
    since.elapsed().as_secs_f64() * 1e3
}

/// Detect, solve, and place what the solution says is on the frame.
pub fn solve_frame(db: &PatternDatabase, image: &GrayImage, options: &SolveOptions) -> SolveReport {
    solve_frame_with_frame(db, image, options).0
}

/// Detect, solve and report, retaining the solved projection for the observing session.
pub fn solve_frame_with_frame(
    db: &PatternDatabase,
    image: &GrayImage,
    options: &SolveOptions,
) -> (SolveReport, Option<Frame>) {
    let cedar = DetectParams::cedar();
    let t = Instant::now();
    let detected = detect_stars(
        &image.pixels,
        image.width,
        image.height,
        options.sigma,
        options.binning,
        cedar.normalize_rows,
        cedar.detect_hot_pixels,
    );
    let detect = ms(t);
    let detected = detected.iter().map(|s| [s.x, s.y]).collect();
    solve_detected(db, detected, (image.width, image.height), options, detect)
}

/// Solve a node's brightest-first centroids, keeping the solved projection.
/// Pixel coordinates on the link are `(x, y)`; the solver takes `(y, x)`.
pub fn solve_centroids(
    db: &PatternDatabase,
    centroids: &[scope_link::proto::Centroid],
    size: (usize, usize),
    options: &SolveOptions,
) -> (SolveReport, Option<Frame>) {
    let detected = centroids.iter().map(|s| [s.x, s.y]).collect();
    solve_detected(db, detected, size, options, 0.0)
}

fn solve_detected(
    db: &PatternDatabase,
    detected: Vec<[f64; 2]>,
    (width, height): (usize, usize),
    options: &SolveOptions,
    detect: f64,
) -> (SolveReport, Option<Frame>) {
    let centroids: Vec<_> = detected
        .iter()
        .take(options.stars)
        .map(|s| (s[1], s[0]))
        .collect();
    let t = Instant::now();
    let solution = solve_from_centroids(
        &centroids,
        (width, height),
        options.fov,
        options.fov_max_error,
        MATCH_RADIUS,
        MATCH_THRESHOLD,
        options.timeout_ms,
        options.distortion,
        MATCH_MAX_ERROR,
        db,
    );
    let solve = ms(t);

    let status = Status::of(solution.status);
    let mut report = SolveReport {
        name: None,
        status,
        hint: status.hint(),
        width,
        height,
        timing_ms: Timing {
            decode: 0.0,
            detect,
            solve,
        },
        detected: detected.into_iter().take(MAX_DETECTED).collect(),
        used: centroids.len(),
        pointing: None,
        pointing_seq: None,
        stars: Vec::new(),
        objects: Vec::new(),
        nearby: Vec::new(),
        grid: Vec::new(),
    };
    let frame = (status == Status::MatchFound)
        .then(|| Frame::from_solution(&solution))
        .flatten();
    if let Some(frame) = &frame {
        report.pointing = Some(pointing(&solution, frame));
        report.stars = star_marks(db, &solution, frame, options.distortion);
        (report.objects, report.nearby) = object_marks(frame);
        report.grid = graticule(frame);
    }
    (report, frame)
}

fn pointing(solution: &Solution, frame: &Frame) -> Pointing {
    let deg = |v: Option<f64>| v.unwrap_or(0.0).to_degrees();
    let (ra_deg, dec_deg) = (deg(solution.ra), deg(solution.dec));
    Pointing {
        ra_deg,
        dec_deg,
        ra_text: format_ra(ra_deg),
        dec_text: format_dec(dec_deg),
        constellation: constellation::containing(ra_deg, dec_deg),
        roll_deg: deg(solution.roll),
        north_deg: frame.north_deg(),
        fov_deg: solution.camera.map_or(0.0, |c| c.fov.to_degrees()),
        fov_vertical_deg: frame.fov_vertical().to_degrees(),
        arcsec_per_px: frame.arcsec_per_px(),
        distortion: solution.distortion.unwrap_or(0.0),
        matched: solution.matched_centroids.len(),
        rmse_arcsec: solution.rmse,
        p90_arcsec: solution.p90e,
        max_arcsec: solution.maxe,
        false_alarm: solution.match_probability,
    }
}

/// A catalog star's designation, proper name and magnitude.
fn describe(db: &PatternDatabase, index: usize) -> (String, Option<&'static str>, f64) {
    let mag = db.star_radec_mag(StarId(index)).map_or(f64::NAN, |s| s.2);
    let (id, name) = match db.star_catalog_ids.get(index) {
        Some(CatalogId::Hip(n)) => (format!("HIP {n}"), stars::name_of(*n)),
        Some(CatalogId::Bsc(n)) => (format!("HR {n}"), None),
        Some(CatalogId::Tyc(a, b, c)) => (format!("TYC {a}-{b}-{c}"), None),
        None => (format!("#{index}"), None),
    };
    (id, name, mag)
}

fn star_marks(
    db: &PatternDatabase,
    solution: &Solution,
    frame: &Frame,
    initial_distortion: f64,
) -> Vec<StarMark> {
    let (width, height) = frame.size();
    // The solver hands back centroids undistorted with the caller's starting `k`;
    // distort them back to where they sit in the image.
    let detected = distort_centroids(
        &solution.matched_centroids,
        width,
        height,
        initial_distortion,
        None,
        None,
    );
    let mut marks = Vec::new();
    let mut matched = HashSet::new();
    for ((&(y, x), &star), &index) in detected
        .iter()
        .zip(&solution.matched_stars)
        .zip(&solution.matched_catalog_ids)
    {
        matched.insert(index);
        let (id, name, mag) = describe(db, index);
        marks.push(StarMark {
            x,
            y,
            id,
            name,
            mag,
            matched: true,
            error_px: frame.project(star).map(|(px, py)| (px - x).hypot(py - y)),
        });
    }
    let expected = db
        .nearby_stars(frame.center, frame.half_diagonal())
        .into_iter()
        .filter(|index| !matched.contains(index))
        .filter_map(|index| {
            let point = frame.project(db.star_vector(StarId(index))?)?;
            frame.contains(point, 0.0).then_some((index, point))
        })
        .take(MAX_EXPECTED_STARS);
    for (index, (x, y)) in expected {
        let (id, name, mag) = describe(db, index);
        marks.push(StarMark {
            x,
            y,
            id,
            name,
            mag,
            matched: false,
            error_px: None,
        });
    }
    marks
}

/// One symbol through the same overlap and projection path as the report's objects.
pub(crate) fn object_mark(frame: &Frame, object: &DeepSkyObject) -> Option<ObjectMark> {
    let half_diagonal = frame.half_diagonal().to_degrees();
    let px_per_deg = 3600.0 / frame.arcsec_per_px();
    let separation_deg = angular_distance(frame.center, object.vector()).to_degrees();
    let radius_deg = object.major_arcmin.unwrap_or(0.0) / 120.0;
    let minor_deg = object.minor_arcmin.map_or(radius_deg, |m| m / 120.0);
    if separation_deg <= half_diagonal + radius_deg {
        if let Some(point) = frame.project(object.vector()) {
            let radius_px = radius_deg * px_per_deg;
            if frame.contains(point, radius_px) {
                // Position angle runs from north through east, and east is a quarter
                // turn counterclockwise from north on a frame of the sky.
                let north = frame.north_at(object.vector()).unwrap_or(0.0);
                let angle_deg = (north - object.pa_deg.unwrap_or(0.0)).rem_euclid(360.0);
                return Some(ObjectMark {
                    object: object.clone(),
                    x: point.0,
                    y: point.1,
                    radius_px,
                    minor_px: minor_deg * px_per_deg,
                    angle_deg,
                    separation_deg,
                    inside: frame.contains(point, 0.0),
                });
            }
        }
    }
    None
}

fn object_marks(frame: &Frame) -> (Vec<ObjectMark>, Vec<NearbyMark>) {
    let half_diagonal = frame.half_diagonal().to_degrees();
    let (mut marks, mut nearby) = (Vec::new(), Vec::new());
    for object in catalog::messier() {
        if let Some(mark) = object_mark(frame, object) {
            marks.push(mark);
            continue;
        }
        let separation_deg = angular_distance(frame.center, object.vector()).to_degrees();
        if separation_deg <= half_diagonal + NEARBY_DEG {
            if let Some(point) = frame.project_pinhole(object.vector()) {
                nearby.push(NearbyMark {
                    object: object.clone(),
                    separation_deg,
                    direction_deg: frame.direction_deg(point),
                });
            }
        }
    }
    let brightness = |m: &ObjectMark| m.object.mag.unwrap_or(f64::INFINITY);
    marks.sort_by(|a, b| {
        b.inside
            .cmp(&a.inside)
            .then(brightness(a).total_cmp(&brightness(b)))
            .then(a.separation_deg.total_cmp(&b.separation_deg))
    });
    nearby.sort_by(|a, b| a.separation_deg.total_cmp(&b.separation_deg));
    nearby.truncate(MAX_NEARBY);
    (marks, nearby)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(width: u32, height: u32) -> Vec<u8> {
        let img = image::GrayImage::new(width, height);
        let mut out = std::io::Cursor::new(Vec::new());
        img.write_to(&mut out, image::ImageFormat::Png).unwrap();
        out.into_inner()
    }

    #[test]
    fn decodes_and_refuses() {
        let image = decode_gray(&png(12, 7)).unwrap();
        assert_eq!((image.width, image.height, image.pixels.len()), (12, 7, 84));
        assert!(matches!(
            decode_gray(&png(MAX_IMAGE_SIDE + 1, 1)),
            Err(DecodeError::TooLarge(_))
        ));
        assert!(matches!(
            decode_gray(b"not an image"),
            Err(DecodeError::Unreadable(_))
        ));
        assert!(DecodeError::TooLarge("x".into())
            .to_string()
            .contains("too large"));
        assert!(DecodeError::Unreadable("x".into())
            .to_string()
            .contains("not a readable"));
    }

    #[test]
    fn color_turns_gray_as_cedar_turns_it() {
        // A green pixel: `to_luma8`'s Rec. 709 weights make it 147; cedar's frames used 601
        // (`star_detection::gray`, which checks PIL's values itself).
        let mut img = image::RgbImage::new(2, 1);
        img.put_pixel(1, 0, image::Rgb([10, 200, 30]));
        let mut png = std::io::Cursor::new(Vec::new());
        img.write_to(&mut png, image::ImageFormat::Png).unwrap();
        let gray = decode_gray(png.get_ref()).unwrap();
        assert_eq!(gray.pixels, [0, gray::pil_luma(10, 200, 30)]);
        assert_eq!(gray.pixels[1], 124);
        let rec709 = image::DynamicImage::ImageRgb8(img).to_luma8();
        assert_eq!(rec709.get_pixel(1, 0).0, [147]);
    }

    #[test]
    fn thumbnails_fit_the_box() {
        let jpeg = thumbnail_jpeg(&png(400, 100), 80).unwrap();
        let thumb = image::load_from_memory(&jpeg).unwrap();
        assert_eq!((thumb.width(), thumb.height()), (80, 20));
        assert!(thumbnail_jpeg(b"nope", 80).is_err());
    }

    #[test]
    fn stretch_takes_the_sky_to_black_and_the_stars_to_white() {
        // A dim sky at 20 with one star at 120 in a thousand pixels.
        let mut img = image::GrayImage::from_pixel(100, 10, image::Luma([20]));
        img.put_pixel(3, 3, image::Luma([120]));
        stretch(&mut img);
        assert_eq!(img.get_pixel(50, 5).0, [0]);
        assert_eq!(img.get_pixel(3, 3).0, [255]);
        // Pure noise keeps a floor under its range: no level reaches white.
        let mut noise = image::GrayImage::from_fn(50, 50, |x, y| {
            image::Luma([20 + ((x * 7 + y * 13) % 5) as u8])
        });
        stretch(&mut noise);
        assert!(noise.pixels().all(|p| p.0[0] < 160));
        stretch(&mut image::GrayImage::new(0, 0));
    }

    #[test]
    fn previews_keep_faint_stars() {
        // One faint star in a big dark frame survives the shrink and the stretch.
        let mut img = image::GrayImage::from_pixel(1600, 1200, image::Luma([10]));
        img.put_pixel(801, 601, image::Luma([60]));
        let mut png_bytes = std::io::Cursor::new(Vec::new());
        img.write_to(&mut png_bytes, image::ImageFormat::Png)
            .unwrap();
        let jpeg = thumbnail_jpeg(png_bytes.get_ref(), 160).unwrap();
        let thumb = image::load_from_memory(&jpeg).unwrap().to_luma8();
        assert_eq!(thumb.dimensions(), (160, 120));
        assert!(
            thumb.pixels().map(|p| p.0[0]).max().unwrap() > 200,
            "the star is gone"
        );
    }

    #[test]
    fn every_failure_has_a_hint() {
        assert_eq!(Status::of(Some(SolveStatus::MatchFound)).hint(), None);
        for s in [
            SolveStatus::NoMatch,
            SolveStatus::Timeout,
            SolveStatus::Cancelled,
            SolveStatus::TooFew,
        ] {
            assert!(Status::of(Some(s)).hint().is_some(), "{s:?}");
        }
        assert_eq!(Status::of(None), Status::NoMatch);
    }

    #[test]
    fn defaults_match_the_solve_image_example() {
        let o = SolveOptions::default();
        assert_eq!(
            (o.fov, o.binning, o.stars, o.timeout_ms),
            (None, 2, 30, 5000)
        );
        assert_eq!((o.fov_max_error, o.distortion, o.sigma), (10.0, 0.0, 8.0));
    }
}
