//! A 16-bit Bayer camera along the main telescope's axis, with its own seeded sky.
use crate::{noise::SplitMix64, Simulator};
use math_core::{attitude::RotationMatrix, PinholeCamera, UnitVector};
use scope_node::imaging::{
    check_exposure, check_settings, BayerPattern, Exposure, ImagingCamera, ImagingError,
    ImagingFrame, ImagingFuture, ImagingInfo, ImagingSettings, ImagingStatus, Range, Roi,
};
use std::{
    borrow::Cow,
    f64::consts::{PI, TAU},
    sync::Mutex,
    time::Duration,
};
use tokio::sync::Notify;

/// The seed of the simulated imaging camera's own field.
pub const FIELD_SEED: u64 = 20_261_115;
/// Stars a square degree brighter than [`MAG_FAINT`].
pub const STARS_PER_SQ_DEG: f64 = 320.0;
/// The brightest magnitude the field draws.
pub const MAG_BRIGHT: f64 = 8.0;
/// The faintest magnitude the field draws.
pub const MAG_FAINT: f64 = 16.0;
/// Log10 of the star count, per magnitude.
pub const MAG_SLOPE: f64 = 0.33;
/// Electrons a magnitude-10 star delivers a second.
pub const ELECTRONS_MAG10_PER_S: f64 = 400_000.0;
/// Electrons of sky a sample collects a second.
pub const SKY_E_PER_S: f64 = 20.0;
/// Read noise, in electrons.
pub const READ_NOISE_E: f64 = 3.0;
/// The star profile's width in focus, in pixels.
pub const PSF_SIGMA_PX: f64 = 1.2;
/// Added in quadrature per unit of focus error, in pixels.
pub const FOCUS_SIGMA_PX: f64 = 1.0;
/// Counts an electron makes at gain 0, times 10 for every 200 of gain.
pub const ADU_PER_E_AT_GAIN_0: f64 = 0.25;
/// Counts a unit of offset makes.
pub const ADU_PER_OFFSET: f64 = 16.0;

/// The imaging optics. Pixels are (x, y); the field is horizontal, in degrees.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImagingOptics {
    pub width: u32,
    pub height: u32,
    pub fov_deg: f64,
    /// The imaging angle: positive turns frame-up toward the finder frame's
    /// left.
    pub angle_deg: f64,
    pub focus_error: f64,
    pub pixel_um: f64,
}

impl Default for ImagingOptics {
    fn default() -> Self {
        Self {
            width: 1236,
            height: 822,
            fov_deg: 0.75,
            angle_deg: 17.0,
            focus_error: 0.0,
            pixel_um: 19.12,
        }
    }
}

/// Where an imaging frame points, independent of any solve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImagingTruth {
    /// J2000 degrees of the frame's centre.
    pub ra_deg: f64,
    pub dec_deg: f64,
    /// The angle of frame-up east of celestial north, 0 to 360.
    pub up_east_of_north_deg: f64,
    pub arcsec_per_px: f64,
}

/// A star of the simulated field.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SimStar {
    pub ra_deg: f64,
    pub dec_deg: f64,
    pub magnitude: f64,
    /// Relative response in red, green and blue.
    pub colour: [f64; 3],
}

/// A Dec band's height, in degrees.
const BAND_DEG: f64 = 0.25;

/// The cells of a Dec band: the cells the plan's field cuts the sky into.
fn cells(band: i64) -> u32 {
    let middle = -90.0 + BAND_DEG * band as f64 + BAND_DEG / 2.0;
    (360.0 * middle.to_radians().cos() / BAND_DEG)
        .round()
        .max(1.0) as u32
}

/// One cell's stars: a Poisson count of the plan's density over the cell's
/// exact area, then a place, a magnitude and a colour from four uniforms.
fn cell_stars(seed: u64, band: i64, cell: u32) -> Vec<SimStar> {
    let count = cells(band);
    let delta = 360.0 / f64::from(count);
    let dec_lo = -90.0 + BAND_DEG * band as f64;
    let (dec_lo_r, dec_hi_r) = (dec_lo.to_radians(), (dec_lo + BAND_DEG).to_radians());
    let area = delta.to_radians() * (dec_hi_r.sin() - dec_lo_r.sin()) * (180.0 / PI).powi(2);
    // The plan names the cell's frame as j·2²⁰ + i, but SplitMix64's frames
    // sit one draw apart on one orbit, and its Poisson draws come in pairs, so
    // neighbouring cells of a band would draw the same stars and the field
    // would hold repeated magnitudes (US2-S8). Mixing the frame through the
    // golden constant spreads the cells; a cell still depends only on the
    // seed and its place on the sky.
    let frame = (band as u64 * (1_u64 << 20) + u64::from(cell)).wrapping_mul(0x9e3779b97f4a7c15);
    let mut random = SplitMix64::new(seed, frame);
    let mut stars = Vec::new();
    for _ in 0..random.poisson(STARS_PER_SQ_DEG * area) as usize {
        let ra = f64::from(cell) * delta + delta * random.uniform();
        let dec = dec_lo + BAND_DEG * random.uniform();
        let uniform = random.uniform();
        let magnitude =
            8.0 + (1.0 + uniform * (10f64.powf(MAG_SLOPE * 8.0) - 1.0)).log10() / MAG_SLOPE;
        let colour_uniform = random.uniform();
        stars.push(SimStar {
            ra_deg: ra,
            dec_deg: dec,
            magnitude,
            colour: [0.6 + 0.8 * colour_uniform, 1.0, 1.4 - 0.8 * colour_uniform],
        });
    }
    stars
}

/// The seeded field's stars within `radius_deg` of a place, brightest first.
/// The field depends only on the seed and the place on the sky.
pub fn deep_field(seed: u64, ra_deg: f64, dec_deg: f64, radius_deg: f64) -> Vec<SimStar> {
    assert!(ra_deg.is_finite() && dec_deg.is_finite() && dec_deg.abs() <= 90.0);
    assert!(radius_deg.is_finite() && radius_deg >= 0.0);
    let centre = UnitVector::from_radec(ra_deg.to_radians(), dec_deg.to_radians());
    let dec_lo = (dec_deg - radius_deg).max(-90.0);
    let dec_hi = (dec_deg + radius_deg).min(90.0);
    if dec_lo > dec_hi {
        return Vec::new();
    }
    // The RA reach of the circle, as far as the meridian from the pole that
    // touches it: asin(sin r / cos dec), or the whole sky at the pole.
    let (sin_r, cos_dec) = (radius_deg.to_radians().sin(), dec_deg.to_radians().cos());
    let half = if sin_r >= cos_dec {
        180.0
    } else {
        (sin_r / cos_dec).asin().to_degrees()
    };
    let mut stars = Vec::new();
    let band_lo = ((dec_lo + 90.0) / BAND_DEG).floor() as i64;
    let band_hi = (((dec_hi + 90.0) / BAND_DEG).floor() as i64).min(719);
    for band in band_lo..=band_hi {
        let count = cells(band);
        let delta = 360.0 / f64::from(count);
        let first = ((ra_deg - half) / delta).floor() as i64;
        let last = ((ra_deg + half) / delta).floor() as i64;
        // A reach that spans the band holds each cell once, not more.
        let touched: Box<dyn Iterator<Item = i64>> = if last - first + 1 >= count as i64 {
            Box::new(0..count as i64)
        } else {
            Box::new(first..=last)
        };
        for cell in touched.map(|index| index.rem_euclid(count as i64) as u32) {
            for star in cell_stars(seed, band, cell) {
                let place =
                    UnitVector::from_radec(star.ra_deg.to_radians(), star.dec_deg.to_radians());
                if math_core::angular_distance(centre, place).to_degrees() <= radius_deg {
                    stars.push(star);
                }
            }
        }
    }
    stars.sort_by(|a, b| {
        a.magnitude
            .total_cmp(&b.magnitude)
            .then_with(|| a.ra_deg.total_cmp(&b.ra_deg))
    });
    stars
}

/// The settings in force, whether an exposure runs, how many aborts have been
/// asked for and how many frames this camera has delivered.
struct State {
    settings: ImagingSettings,
    exposing: bool,
    aborts: u64,
    frames: u64,
}

/// Clears the exposing mark on every way out, a dropped future included.
struct ExposingGuard<'a>(&'a Mutex<State>);
impl Drop for ExposingGuard<'_> {
    fn drop(&mut self) {
        self.0.lock().unwrap().exposing = false;
    }
}

/// One exposure's fixed inputs: the settings, optics and axes in force when
/// it began, its length and the frame number its noise comes from.
struct Shot {
    settings: ImagingSettings,
    optics: ImagingOptics,
    rotation0: RotationMatrix,
    rotation1: RotationMatrix,
    seconds: f64,
    exposure_ms: f64,
    start_s: f64,
    frame_number: u64,
}

/// A 16-bit colour camera that looks where the main telescope looks, seeded
/// and repeatable, with the sky, its noise and the seeded field of
/// [`deep_field`].
pub struct SimImagingCamera {
    simulator: Simulator,
    seed: u64,
    stars: Option<Vec<SimStar>>,
    noise: bool,
    state: Mutex<State>,
    abort: Notify,
}

impl SimImagingCamera {
    /// The rig's camera: the deep field, noise on, seed 13.
    pub fn new(simulator: Simulator) -> Self {
        let (width, height) = {
            let optics = simulator.imaging_optics();
            (optics.width, optics.height)
        };
        Self {
            simulator,
            seed: 13,
            stars: None,
            noise: true,
            state: Mutex::new(State {
                settings: ImagingSettings {
                    gain: Some(120.0),
                    offset: Some(10.0),
                    binning: Some(1),
                    roi: Some(Roi {
                        x: 0,
                        y: 0,
                        width,
                        height,
                    }),
                    cooler_on: Some(false),
                    set_temp_c: None,
                },
                exposing: false,
                aborts: 0,
                frames: 0,
            }),
            abort: Notify::new(),
        }
    }

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Draw these stars and no others.
    pub fn with_stars(mut self, stars: Vec<SimStar>) -> Self {
        self.stars = Some(stars);
        self
    }

    pub fn with_noise(mut self, on: bool) -> Self {
        self.noise = on;
        self
    }

    /// How many frames this camera has delivered. An aborted exposure takes
    /// none, so the frame number counts the frames delivered.
    pub fn frames_taken(&self) -> u64 {
        self.state.lock().unwrap().frames
    }

    /// The description: the optics in force and this camera's own constants.
    fn info(&self) -> ImagingInfo {
        let optics = self.simulator.imaging_optics();
        ImagingInfo {
            model: "simulated imaging camera".into(),
            serial: Some("SIM-0001".into()),
            width: optics.width,
            height: optics.height,
            pixel_um: optics.pixel_um,
            bits: 16,
            bayer: Some(BayerPattern::Rggb),
            exposure_ms: Range {
                min: 1.0,
                max: 3_600_000.0,
            },
            gain: Some(Range {
                min: 0.0,
                max: 300.0,
            }),
            offset: Some(Range {
                min: 0.0,
                max: 100.0,
            }),
            binnings: vec![1, 2],
            cooler: true,
        }
    }

    /// The field this exposure draws: the stars it was given, or the seeded
    /// field about the truth at its start, out to half the frame's diagonal
    /// plus 0.05 degrees.
    fn stars_for(&self, shot: &Shot) -> Cow<'_, [SimStar]> {
        if let Some(stars) = &self.stars {
            return Cow::Borrowed(stars);
        }
        let boresight = UnitVector {
            x: shot.rotation0.rows[0][0],
            y: shot.rotation0.rows[0][1],
            z: shot.rotation0.rows[0][2],
        };
        let (ra, dec) = boresight.to_radec();
        let diagonal = 0.5
            * (f64::from(shot.optics.width).hypot(f64::from(shot.optics.height)))
            * shot.optics.fov_deg
            / f64::from(shot.optics.width);
        Cow::Owned(deep_field(
            FIELD_SEED,
            ra.to_degrees(),
            dec.to_degrees(),
            diagonal + 0.05,
        ))
    }

    /// One frame: stars, sky and noise as electrons, then counts, binning
    /// and rounding, by plan.md § A frame.
    fn render(&self, shot: &Shot, stars: &[SimStar]) -> ImagingFrame {
        let binning = shot.settings.binning.unwrap_or(1).max(1);
        let roi = shot.settings.roi.unwrap_or_default();
        let width = roi.width as usize;
        let height = roi.height as usize;
        let mut electrons = vec![0.0; width * height];
        let sigma = (PSF_SIGMA_PX * PSF_SIGMA_PX
            + (FOCUS_SIGMA_PX * shot.optics.focus_error).powi(2))
        .sqrt();
        let reach = (sigma.mul_add(4.0, 1.0)).ceil() as i64;
        let camera = PinholeCamera::new(
            f64::from(shot.optics.width),
            f64::from(shot.optics.height),
            shot.optics.fov_deg.to_radians(),
        );
        for star in stars {
            let Some(p0) = place(&camera, &shot.rotation0, star) else {
                continue;
            };
            let Some(p1) = place(&camera, &shot.rotation1, star) else {
                continue;
            };
            // The smear is drawn along the straight line from the star's
            // place at the start to its place at the end.
            let steps = (p1.0 - p0.0).hypot(p1.1 - p0.1).ceil().clamp(1.0, 64.0) as u32;
            let electrons_total =
                ELECTRONS_MAG10_PER_S * 10f64.powf(-0.4 * (star.magnitude - 10.0)) * shot.seconds;
            for step in 0..steps {
                let along = (f64::from(step) + 0.5) / f64::from(steps);
                let (px, py) = (p0.0 + along * (p1.0 - p0.0), p0.1 + along * (p1.1 - p0.1));
                // The samples whose column and row are each within reach of
                // the point, a square box, clipped to the region.
                let columns = (
                    (px - reach as f64 - 0.5).ceil() as i64,
                    (px + reach as f64 - 0.5).floor() as i64,
                );
                let rows = (
                    (py - reach as f64 - 0.5).ceil() as i64,
                    (py + reach as f64 - 0.5).floor() as i64,
                );
                let point = electrons_total / f64::from(steps);
                for row in
                    rows.0.max(i64::from(roi.y))..=rows.1.min(i64::from(roi.y + roi.height) - 1)
                {
                    for column in columns.0.max(i64::from(roi.x))
                        ..=columns.1.min(i64::from(roi.x + roi.width) - 1)
                    {
                        let distance =
                            (column as f64 + 0.5 - px).powi(2) + (row as f64 + 0.5 - py).powi(2);
                        // The sensor's mosaic: red at an even column and row,
                        // blue at odd and odd, green otherwise.
                        let filter = match (column % 2, row % 2) {
                            (0, 0) => 0,
                            (1, 1) => 2,
                            _ => 1,
                        };
                        let index = (row - i64::from(roi.y)) as usize * width
                            + (column - i64::from(roi.x)) as usize;
                        electrons[index] +=
                            point * star.colour[filter] * (-distance / (2.0 * sigma * sigma)).exp()
                                / (TAU * sigma * sigma);
                    }
                }
            }
        }
        // The sky falls on every sample of the region.
        let sky = SKY_E_PER_S * shot.seconds;
        for sample in &mut electrons {
            *sample += sky;
        }
        if self.noise {
            let mut random = SplitMix64::new(self.seed, shot.frame_number);
            for sample in &mut electrons {
                let deviation = (*sample + READ_NOISE_E * READ_NOISE_E).sqrt();
                *sample += deviation * random.normal();
            }
        }
        // Electrons become counts; binning 2 averages samples of the same
        // colour, keeping the mosaic; rounding is half away from zero.
        let counts = ADU_PER_OFFSET * shot.settings.offset.unwrap_or(0.0);
        let scale = ADU_PER_E_AT_GAIN_0 * 10f64.powf(shot.settings.gain.unwrap_or(0.0) / 200.0);
        let delivered = |value: f64| (value.round().clamp(0.0, 65535.0) as u16).to_le_bytes();
        let mut samples = Vec::with_capacity(width * height * 2);
        if binning == 1 {
            for sample in &electrons {
                samples.extend_from_slice(&delivered(counts + sample * scale));
            }
        } else {
            for row in 0..height / 2 {
                for column in 0..width / 2 {
                    // The four same-colour region samples the delivered one
                    // averages, of which those inside the region count.
                    let x0 = 2 * (column - column % 2) + column % 2;
                    let y0 = 2 * (row - row % 2) + row % 2;
                    let mut sum = 0.0;
                    let mut inside = 0.0;
                    for (dx, dy) in [(0, 0), (2, 0), (0, 2), (2, 2)] {
                        let (x, y) = (x0 + dx, y0 + dy);
                        if x < width && y < height {
                            sum += counts + electrons[y * width + x] * scale;
                            inside += 1.0;
                        }
                    }
                    samples.extend_from_slice(&delivered(sum / inside));
                }
            }
        }
        let (temp_c, _) = cooler(&shot.settings);
        ImagingFrame {
            samples,
            width: roi.width / binning,
            height: roi.height / binning,
            bits: 16,
            bayer: Some(BayerPattern::Rggb),
            start_s: shot.start_s,
            end_s: shot.start_s + shot.seconds,
            exposure_ms: shot.exposure_ms,
            gain: shot.settings.gain,
            offset: shot.settings.offset,
            binning,
            temp_c: Some(temp_c),
            roi,
        }
    }
}

/// The cooler's rule: with the cooler off, or on with no set temperature, the
/// sensor reads 20.0 °C at power 0.0; on with a set temperature `T`, it reads
/// `T` at power `clamp(2.5·(20 − T), 0, 100)`.
fn cooler(settings: &ImagingSettings) -> (f64, f64) {
    match (settings.cooler_on, settings.set_temp_c) {
        (Some(true), Some(temp)) => (temp, (2.5 * (20.0 - temp)).clamp(0.0, 100.0)),
        _ => (20.0, 0.0),
    }
}

/// A star's place in the frame, as (x, y), or none behind the boresight.
fn place(camera: &PinholeCamera, rotation: &RotationMatrix, star: &SimStar) -> Option<(f64, f64)> {
    let direction = UnitVector::from_radec(star.ra_deg.to_radians(), star.dec_deg.to_radians());
    let local = rotation.rotate(direction);
    if local.x <= 0.0 {
        return None;
    }
    let (pixels, _) = camera.project(&[local]);
    let (y, x) = pixels[0];
    Some((x, y))
}

impl ImagingCamera for SimImagingCamera {
    fn describe(&self) -> ImagingFuture<'_, ImagingInfo> {
        Box::pin(async move { Ok(self.info()) })
    }

    fn status(&self) -> ImagingFuture<'_, ImagingStatus> {
        Box::pin(async move {
            let state = self.state.lock().unwrap();
            let (temp_c, cooler_power) = cooler(&state.settings);
            Ok(ImagingStatus {
                temp_c: Some(temp_c),
                cooler_power: Some(cooler_power),
                set_temp_c: state.settings.set_temp_c,
                exposing: state.exposing,
            })
        })
    }

    fn apply<'a>(&'a self, settings: &'a ImagingSettings) -> ImagingFuture<'a, ImagingSettings> {
        Box::pin(async move {
            let mut state = self.state.lock().unwrap();
            // While an exposure runs, an empty edit answers the settings in
            // force and any other is Busy before any check, changing nothing.
            if state.exposing {
                if *settings == ImagingSettings::default() {
                    return Ok(state.settings.clone());
                }
                return Err(ImagingError::Busy);
            }
            let merged = check_settings(&self.info(), &state.settings, settings)?;
            state.settings = merged.clone();
            Ok(merged)
        })
    }

    fn expose<'a>(&'a self, exposure: &'a Exposure) -> ImagingFuture<'a, ImagingFrame> {
        Box::pin(async move {
            check_exposure(&self.info(), exposure)?;
            // The settings in force are fixed here, and their region is the
            // frame's roi; the frame number is the next one this camera
            // delivers.
            let (settings, frame_number, aborts) = {
                let mut state = self.state.lock().unwrap();
                if state.exposing {
                    return Err(ImagingError::Busy);
                }
                state.exposing = true;
                (state.settings.clone(), state.frames, state.aborts)
            };
            let _clear = ExposingGuard(&self.state);
            let start_s = self.simulator.clock().node_now();
            let (rotation0, optics) = self
                .simulator
                .imaging_rotation_at(self.simulator.clock().now());
            let wait = tokio::time::sleep(Duration::from_secs_f64(exposure.exposure_ms / 1000.0));
            tokio::pin!(wait);
            loop {
                tokio::select! {
                    _ = &mut wait => break,
                    _ = self.abort.notified() => {
                        // A permit an older exposure left behind is not ours.
                        let asked = self.state.lock().unwrap().aborts;
                        if asked != aborts {
                            return Err(ImagingError::Aborted);
                        }
                    }
                }
            }
            let rotation1 = self
                .simulator
                .imaging_rotation_at(self.simulator.clock().now())
                .0;
            let shot = Shot {
                settings,
                optics,
                rotation0,
                rotation1,
                seconds: exposure.exposure_ms / 1000.0,
                exposure_ms: exposure.exposure_ms,
                start_s,
                frame_number,
            };
            let frame = self.render(&shot, &self.stars_for(&shot));
            self.state.lock().unwrap().frames += 1;
            Ok(frame)
        })
    }

    fn abort(&self) -> ImagingFuture<'_, ()> {
        Box::pin(async move {
            let mut state = self.state.lock().unwrap();
            if state.exposing {
                state.aborts += 1;
                drop(state);
                self.abort.notify_one();
            }
            Ok(())
        })
    }
}
