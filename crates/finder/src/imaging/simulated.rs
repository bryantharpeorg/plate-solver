//! The simulated solver adapter (032): answers from the simulator's truth, so
//! a desk console solves its imaging frames without a star database. It reads
//! no file (FR-008).

use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use math_core::{angular_distance, UnitVector};
use simulator::{
    imaging::{deep_field, ImagingOptics, SimStar, FIELD_SEED, FOCUS_SIGMA_PX, PSF_SIGMA_PX},
    Simulator,
};

use super::solver::{
    ImagingSolve, ImagingSolver, MeasureFuture, Region, SolveError, SolveFrame, SolveFuture,
    SolveHint, StarMeasure,
};

/// ASTAP's page: "a minimum about 30 stars".
pub const MIN_STARS: usize = 30;
/// ASTAP's page: the height "within preferably 5% accuracy".
pub const FOV_TOLERANCE: f64 = 0.05;
/// How far past the sensor the seeded field is read for a star count, in
/// degrees.
const FIELD_SLACK_DEG: f64 = 0.05;

/// The simulated solver adapter: the truth of the frame's exposure is the
/// answer, and a hint is accepted only within its own rules.
pub struct SimulatedSolver {
    simulator: Simulator,
    delay: Duration,
    /// The seed and fraction of the measure noise (033 FR-005), off until
    /// `with_measure_noise` is called.
    noise: Option<(u64, f64)>,
    /// The star measures this solver has answered, for the noise stream: a call
    /// that answers `TooFewStars` takes none.
    measures: AtomicU64,
    /// The sharpest point of the sensor, as fractions of its width and
    /// height, and the growth away from it (033 FR-005), off until
    /// `with_sharpest_at` is called.
    sharpest: Option<(f64, f64, f64)>,
}

impl SimulatedSolver {
    /// Answers from this simulator's truth.
    pub fn new(simulator: Simulator) -> Self {
        Self {
            simulator,
            delay: Duration::ZERO,
            noise: None,
            measures: AtomicU64::new(0),
            sharpest: None,
        }
    }

    /// Wait this long on tokio time before answering.
    pub fn with_delay(self, delay: Duration) -> Self {
        Self { delay, ..self }
    }

    /// Multiply each star measure by a seeded factor from `1 − fraction` to
    /// `1 + fraction`. The n-th star measure this solver answers takes the
    /// n-th number of the stream, so two solvers built alike answer alike.
    pub fn with_measure_noise(self, seed: u64, fraction: f64) -> Self {
        Self {
            noise: Some((seed, fraction)),
            ..self
        }
    }

    /// Stars are smallest at (x, y), given as fractions of the sensor's width
    /// and height, and grow away from it: `growth` sensor pixels of width,
    /// added in quadrature, at half the sensor's diagonal from that point.
    pub fn with_sharpest_at(self, x: f64, y: f64, growth: f64) -> Self {
        Self {
            sharpest: Some((x, y, growth)),
            ..self
        }
    }
}

impl ImagingSolver for SimulatedSolver {
    fn name(&self) -> &'static str {
        "simulated"
    }

    fn solve(&self, frame: SolveFrame, hint: SolveHint) -> SolveFuture<'_> {
        let took_ms = self.delay.as_secs_f64() * 1000.0;
        Box::pin(async move {
            tokio::time::sleep(self.delay).await;
            answer(&self.simulator, &frame, hint, took_ms)
        })
    }

    fn measure(&self, frame: SolveFrame, region: Option<Region>) -> MeasureFuture<'_> {
        Box::pin(async move {
            tokio::time::sleep(self.delay).await;
            measure(&self.simulator, self, &frame, region)
        })
    }
}

/// The adapter's answer for one frame (FR-008's five steps).
fn answer(
    simulator: &Simulator,
    frame: &SolveFrame,
    hint: SolveHint,
    took_ms: f64,
) -> Result<ImagingSolve, SolveError> {
    let (truth, optics) = (
        simulator.imaging_truth_at(frame.start + frame.exposure_s / 2.0),
        simulator.imaging_optics(),
    );
    let (answer, stars) = geometry(truth, &optics, frame);
    if stars < MIN_STARS {
        return Err(SolveError::TooFewStars {
            detail: Some(format!("{stars} stars")),
        });
    }
    // A hint's place farther from the answer than its radius is refused, and so
    // is a field height more than 5% from the frame's.
    if let Some((ra_deg, dec_deg)) = hint.position {
        let separation = angular_distance(
            UnitVector::from_radec(answer.ra_deg.to_radians(), answer.dec_deg.to_radians()),
            UnitVector::from_radec(ra_deg.to_radians(), dec_deg.to_radians()),
        )
        .to_degrees();
        if separation > hint.radius_deg {
            return Err(SolveError::NotSolved { detail: None });
        }
    }
    if let Some(fov_deg) = hint.fov_deg {
        let height = f64::from(frame.height) * answer.arcsec_per_px / 3600.0;
        if (fov_deg - height).abs() > FOV_TOLERANCE * height {
            return Err(SolveError::NotSolved { detail: None });
        }
    }
    let mut answer = answer;
    answer.stars = Some(stars as u32);
    answer.took_ms = took_ms;
    Ok(answer)
}

/// The star measure (FR-008): the seeded field's stars in the region, and the
/// point-spread width the simulator's focus error gives, grown away from the
/// solver's sharpest point and multiplied by its seeded noise (033 FR-005).
fn measure(
    simulator: &Simulator,
    solver: &SimulatedSolver,
    frame: &SolveFrame,
    region: Option<Region>,
) -> Result<StarMeasure, SolveError> {
    let (truth, optics) = (
        simulator.imaging_truth_at(frame.start + frame.exposure_s / 2.0),
        simulator.imaging_optics(),
    );
    let (answer, _) = geometry(truth, &optics, frame);
    let (x, y, width, height) = match region {
        Some(region) => (
            f64::from(region.x),
            f64::from(region.y),
            f64::from(region.width),
            f64::from(region.height),
        ),
        None => (0.0, 0.0, f64::from(answer.width), f64::from(answer.height)),
    };
    let stars = seeded_stars(truth, &optics)
        .iter()
        .filter(|star| {
            answer
                .pixel_of(star.ra_deg, star.dec_deg)
                .is_some_and(|(px, py)| px >= x && px < x + width && py >= y && py < y + height)
        })
        .count();
    if stars == 0 {
        return Err(SolveError::TooFewStars {
            detail: Some("0 stars".to_string()),
        });
    }
    // The measured region's centre, in sensor pixels: the region given, or the
    // whole frame without one, at the frame's binning over its region of
    // interest.
    let r = region.unwrap_or(Region {
        x: 0,
        y: 0,
        width: frame.width,
        height: frame.height,
    });
    let b = f64::from(frame.binning);
    let (cx, cy) = (
        f64::from(frame.roi.x) + (f64::from(r.x) + f64::from(r.width) / 2.0) * b,
        f64::from(frame.roi.y) + (f64::from(r.y) + f64::from(r.height) / 2.0) * b,
    );
    // Stars grow away from the sharpest point: `growth` sensor pixels of width,
    // added in quadrature, at half the sensor's diagonal from it.
    let grown = match solver.sharpest {
        Some((x, y, growth)) => {
            let (w, h) = (f64::from(optics.width), f64::from(optics.height));
            growth * (cx - x * w).hypot(cy - y * h) / (w.hypot(h) / 2.0)
        }
        None => 0.0,
    };
    let sigma = (PSF_SIGMA_PX * PSF_SIGMA_PX
        + (FOCUS_SIGMA_PX * optics.focus_error).powi(2)
        + grown * grown)
        .sqrt();
    let mut half_flux_px = 2.0 * (2.0 * std::f64::consts::LN_2).sqrt() * sigma / b;
    if let Some((seed, fraction)) = solver.noise {
        let n = solver.measures.fetch_add(1, Ordering::SeqCst);
        half_flux_px *= 1.0 + fraction * (2.0 * unit(seed, n) - 1.0);
    }
    Ok(StarMeasure {
        half_flux_px,
        stars: stars as u32,
    })
}

/// One number of the seeded noise stream, uniform from 0 to 1: SplitMix64,
/// whose values for the seed 0 are the published stream (033 FR-005). The
/// simulator's own generator is private to its crate, which is why these six
/// lines stand here.
fn unit(seed: u64, n: u64) -> f64 {
    let mut z = seed.wrapping_add(n.wrapping_add(1).wrapping_mul(0x9e37_79b9_7f4a_7c15));
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^= z >> 31;
    ((z >> 11) as f64 + 0.5) / 9_007_199_254_740_992.0
}

/// The answer's geometry (FR-008's first two steps): the truth's place moved
/// to the centre of the frame's region, its up taken there and scaled by the
/// binning, never mirrored; and the seeded field's stars in the frame.
fn geometry(
    truth: simulator::imaging::ImagingTruth,
    optics: &ImagingOptics,
    frame: &SolveFrame,
) -> (ImagingSolve, usize) {
    let theta = truth.up_east_of_north_deg.to_radians();
    // Degrees a sensor pixel.
    let scale = truth.arcsec_per_px / 3600.0;
    // The sensor's solve: the whole sensor, at its centre, the truth's up.
    let sensor = ImagingSolve::from_cd(
        truth.ra_deg,
        truth.dec_deg,
        [
            [-scale * theta.cos(), -scale * theta.sin()],
            [scale * theta.sin(), -scale * theta.cos()],
        ],
        optics.width,
        optics.height,
    );
    // The frame's centre is the centre of its region, and its up is the
    // sensor's up seen there, since north turns across the sensor.
    let (cx, cy) = (
        f64::from(frame.roi.x) + f64::from(frame.roi.width) / 2.0,
        f64::from(frame.roi.y) + f64::from(frame.roi.height) / 2.0,
    );
    let (ra_deg, dec_deg) = sensor.sky_at((cx, cy));
    let (up_ra, up_dec) = sensor.sky_at((cx, cy - 1.0));
    let up_deg = position_angle(ra_deg, dec_deg, up_ra, up_dec).to_radians();
    let scale = scale * f64::from(frame.binning);
    let answer = ImagingSolve::from_cd(
        ra_deg,
        dec_deg,
        [
            [-scale * up_deg.cos(), -scale * up_deg.sin()],
            [scale * up_deg.sin(), -scale * up_deg.cos()],
        ],
        frame.width,
        frame.height,
    );
    let stars = seeded_stars(truth, optics)
        .iter()
        .filter(|star| in_frame(&answer, star))
        .count();
    (answer, stars)
}

/// The simulator's seeded field, read past the sensor's diagonal so every star
/// of the frame is in the list.
fn seeded_stars(truth: simulator::imaging::ImagingTruth, optics: &ImagingOptics) -> Vec<SimStar> {
    let radius = f64::from(optics.width).hypot(f64::from(optics.height))
        * (truth.arcsec_per_px / 3600.0)
        / 2.0
        + FIELD_SLACK_DEG;
    deep_field(FIELD_SEED, truth.ra_deg, truth.dec_deg, radius)
}

/// Whether a star of the seeded field falls inside the answer's frame.
fn in_frame(answer: &ImagingSolve, star: &SimStar) -> bool {
    answer
        .pixel_of(star.ra_deg, star.dec_deg)
        .is_some_and(|(x, y)| {
            x >= 0.0 && x < f64::from(answer.width) && y >= 0.0 && y < f64::from(answer.height)
        })
}

/// The position angle from one place to another, east of north, in degrees.
fn position_angle(ra_from: f64, dec_from: f64, ra_to: f64, dec_to: f64) -> f64 {
    let (ra_from, dec_from, ra_to, dec_to) = (
        ra_from.to_radians(),
        dec_from.to_radians(),
        ra_to.to_radians(),
        dec_to.to_radians(),
    );
    (dec_to.cos() * (ra_to - ra_from).sin())
        .atan2(
            dec_from.cos() * dec_to.sin() - dec_from.sin() * dec_to.cos() * (ra_to - ra_from).cos(),
        )
        .to_degrees()
        .rem_euclid(360.0)
}
