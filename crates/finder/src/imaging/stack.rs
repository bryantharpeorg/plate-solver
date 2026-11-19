//! Stacking arithmetic (035): colour from a mosaic, where a sub falls on the
//! reference grid, and the sums that make a stack. Pure functions; nothing here
//! reads a file, a clock, the mount or the site.

pub mod display;
pub mod service;

use super::{
    calibration::{percentile, Mosaic},
    solver::{ImagingSolve, Region},
};

/// At each corner of the reference grid the placing map may lie this many of the
/// sub's pixels from where the two imaging solves put that corner (owner decision 3).
pub const CORNER_SLACK_PX: f64 = 0.5;
/// A sample point this close to the edge of a sub's pixel centres counts as on it.
pub const EDGE_SLACK: f64 = 1e-6;
/// A pixel is tested once it holds this many subs.
pub const LEARNING_SUBS: u32 = 8;
pub const REJECT_SIGMAS: f32 = 3.0;
/// The least a deviation may be: one count.
pub const SIGMA_FLOOR: f32 = 1.0;
/// A level is read on a grid of about this many pixels a side.
pub const LEVEL_GRID: u32 = 512;
/// A sub's centre may move this share of the reference's shorter side.
pub const OFF_FIELD_SHARE: f64 = 0.5;
/// The placing map's area scale may differ from one by this much.
pub const AREA_TOLERANCE: f64 = 0.02;
/// Tolerance for equality of two gain or offset settings.
pub const GAIN_TOLERANCE: f64 = 0.001;
/// Tolerance for equality of two exposure settings, in milliseconds.
pub const EXPOSURE_TOLERANCE_MS: f64 = 1.0;

/// Whether a sub so placed belongs in a stack. Check the solve before drift,
/// so a mirror is always `solve_differs`, whatever its shift.
pub fn judge(
    placement: &Placement,
    reference: (u32, u32),
    sub: (u32, u32),
) -> Result<(), &'static str> {
    let area = placement.area_scale();
    if !area.is_finite() || (area - 1.0).abs() > AREA_TOLERANCE {
        return Err("solve_differs");
    }
    let (x, y) = placement
        .centre_shift(reference, sub)
        .ok_or("solve_differs")?;
    if x.hypot(y) > OFF_FIELD_SHARE * f64::from(reference.0.min(reference.1)) {
        return Err("off_field");
    }
    Ok(())
}

/// What a sub must share with the reference sub to be stacked with it.
#[derive(Debug, Clone, PartialEq)]
pub struct SubSettings {
    pub width: u32,
    pub height: u32,
    pub bits: u32,
    /// The Bayer pattern, or None for mono.
    pub bayer: Option<String>,
    pub binning: u32,
    /// The sub's region in sensor pixels.
    pub roi: Region,
    pub gain: Option<f64>,
    pub offset: Option<f64>,
    /// The console's exposure setting when the sub was asked for.
    pub exposure_ms: f64,
}

/// Compare each sub with the reference sub, including absent gain and offset.
pub fn same_settings(reference: &SubSettings, sub: &SubSettings) -> bool {
    fn same_number(a: Option<f64>, b: Option<f64>) -> bool {
        match (a, b) {
            (None, None) => true,
            (Some(a), Some(b)) => (a - b).abs() <= GAIN_TOLERANCE,
            _ => false,
        }
    }
    reference.width == sub.width
        && reference.height == sub.height
        && reference.bits == sub.bits
        && reference.bayer == sub.bayer
        && reference.binning == sub.binning
        && reference.roi == sub.roi
        && same_number(reference.gain, sub.gain)
        && same_number(reference.offset, sub.offset)
        && (reference.exposure_ms - sub.exposure_ms).abs() <= EXPOSURE_TOLERANCE_MS
}

/// Three planes of 32-bit reals, red, green and blue, each row by row from the top.
#[derive(Debug, Clone, PartialEq)]
pub struct Colour {
    pub width: u32,
    pub height: u32,
    /// Channel c at (x, y) is at `c·width·height + y·width + x`.
    pub planes: Vec<f32>,
}

impl Colour {
    /// All zero.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            planes: vec![0.0; 3 * width as usize * height as usize],
        }
    }

    pub fn at(&self, channel: usize, x: u32, y: u32) -> f32 {
        self.planes[self.index(channel, x, y)]
    }

    pub fn set(&mut self, channel: usize, x: u32, y: u32, value: f32) {
        let index = self.index(channel, x, y);
        self.planes[index] = value;
    }

    pub fn plane(&self, channel: usize) -> &[f32] {
        let size = self.width as usize * self.height as usize;
        &self.planes[channel * size..(channel + 1) * size]
    }

    fn index(&self, channel: usize, x: u32, y: u32) -> usize {
        (channel * self.height as usize + y as usize) * self.width as usize + x as usize
    }
}

/// Keep each pixel's own sample and interpolate its other colours from touching
/// samples inside the mosaic. Mono, or a grid under two wide or high, is copied
/// to all three planes.
pub fn colour(mosaic: &Mosaic) -> Colour {
    let mut picture = Colour::new(mosaic.width, mosaic.height);
    if mosaic.bayer.is_none() || mosaic.width < 2 || mosaic.height < 2 {
        let size = mosaic.samples.len();
        for channel in 0..3 {
            picture.planes[channel * size..(channel + 1) * size].copy_from_slice(&mosaic.samples);
        }
        return picture;
    }

    for y in 0..mosaic.height {
        for x in 0..mosaic.width {
            let own = mosaic.colour(x, y);
            let sample = mosaic.samples[y as usize * mosaic.width as usize + x as usize];
            let mut sums = [0.0_f64; 3];
            let mut counts = [0_u32; 3];
            for ny in y.saturating_sub(1)..=(y + 1).min(mosaic.height - 1) {
                for nx in x.saturating_sub(1)..=(x + 1).min(mosaic.width - 1) {
                    if nx == x && ny == y {
                        continue;
                    }
                    let channel = mosaic.colour(nx, ny);
                    sums[channel] += f64::from(
                        mosaic.samples[ny as usize * mosaic.width as usize + nx as usize],
                    );
                    counts[channel] += 1;
                }
            }
            for (channel, (sum, count)) in sums.into_iter().zip(counts).enumerate() {
                let value = if channel == own {
                    sample
                } else {
                    // Every other colour touches a site on a Bayer grid at least
                    // two wide and high, including its edges and corners.
                    (sum / f64::from(count)) as f32
                };
                picture.set(channel, x, y, value);
            }
        }
    }
    picture
}

/// Each plane's lower median on the grid whose x and y are multiples of
/// ceil(longer side / `LEVEL_GRID`), at least one. Empty planes have level zero.
pub fn levels(picture: &Colour) -> [f32; 3] {
    let step = picture
        .width
        .max(picture.height)
        .div_ceil(LEVEL_GRID)
        .max(1) as usize;
    std::array::from_fn(|channel| {
        let mut samples = Vec::new();
        for y in (0..picture.height).step_by(step) {
            for x in (0..picture.width).step_by(step) {
                samples.push(picture.at(channel, x, y));
            }
        }
        percentile(&samples, 0.5).unwrap_or(0.0)
    })
}

/// The reference grid's position p falls at `m·p + t` in the sub.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    pub m: [[f64; 2]; 2],
    pub t: [f64; 2],
}

impl Placement {
    pub fn identity() -> Self {
        Self {
            m: [[1.0, 0.0], [0.0, 1.0]],
            t: [0.0, 0.0],
        }
    }

    /// `None` when a point it reads has no place in the sub, or when the map
    /// strays more than `CORNER_SLACK_PX` at a corner of the reference grid.
    pub fn between(reference: &ImagingSolve, sub: &ImagingSolve) -> Option<Self> {
        let width = f64::from(reference.width);
        let height = f64::from(reference.height);
        let c = (width / 2.0, height / 2.0);
        let q = width.min(height) / 4.0;
        if q == 0.0 {
            return None;
        }
        let project = |p| {
            let (ra, dec) = reference.sky_at(p);
            sub.pixel_of(ra, dec)
        };
        let fc = project(c)?;
        let fxp = project((c.0 + q, c.1))?;
        let fxm = project((c.0 - q, c.1))?;
        let fyp = project((c.0, c.1 + q))?;
        let fym = project((c.0, c.1 - q))?;
        let m = [
            [(fxp.0 - fxm.0) / (2.0 * q), (fyp.0 - fym.0) / (2.0 * q)],
            [(fxp.1 - fxm.1) / (2.0 * q), (fyp.1 - fym.1) / (2.0 * q)],
        ];
        let placement = Self {
            m,
            t: [
                fc.0 - m[0][0] * c.0 - m[0][1] * c.1,
                fc.1 - m[1][0] * c.0 - m[1][1] * c.1,
            ],
        };
        // Two tangent planes have a projective mapping. Bound the linear
        // approximation's error at every corner before it can place a sub.
        for corner in [(0.0, 0.0), (width, 0.0), (0.0, height), (width, height)] {
            let exact = project(corner)?;
            let mapped = placement.apply(corner);
            if (exact.0 - mapped.0).hypot(exact.1 - mapped.1) > CORNER_SLACK_PX {
                return None;
            }
        }
        Some(placement)
    }

    pub fn apply(&self, p: (f64, f64)) -> (f64, f64) {
        (
            self.m[0][0] * p.0 + self.m[0][1] * p.1 + self.t[0],
            self.m[1][0] * p.0 + self.m[1][1] * p.1 + self.t[1],
        )
    }

    /// How far the sub is turned from the reference, degrees, −180 to 180.
    pub fn turn_deg(&self) -> f64 {
        (self.m[1][0] - self.m[0][1])
            .atan2(self.m[0][0] + self.m[1][1])
            .to_degrees()
    }

    /// The determinant of `m`: 1 for a rigid map, negative for a mirror.
    pub fn area_scale(&self) -> f64 {
        self.m[0][0] * self.m[1][1] - self.m[0][1] * self.m[1][0]
    }

    /// Where the sub's centre falls on the reference grid, less that grid's
    /// centre. `None` when `m` cannot be inverted.
    pub fn centre_shift(&self, reference: (u32, u32), sub: (u32, u32)) -> Option<(f64, f64)> {
        let determinant = self.area_scale();
        if determinant.abs() < 1e-12 {
            return None;
        }
        let x = f64::from(sub.0) / 2.0 - self.t[0];
        let y = f64::from(sub.1) / 2.0 - self.t[1];
        Some((
            (self.m[1][1] * x - self.m[0][1] * y) / determinant - f64::from(reference.0) / 2.0,
            (self.m[0][0] * y - self.m[1][0] * x) / determinant - f64::from(reference.1) / 2.0,
        ))
    }
}

/// What one `add` did.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Added {
    /// Pixels of the grid the sub reached.
    pub covered: u64,
    pub added: u64,
    pub rejected: u64,
    /// What was added to the sub's red, green and blue before anything else.
    pub offsets: [f32; 3],
}

/// Three planes of sums and one per-pixel count on the reference grid.
pub struct Stack {
    width: u32,
    height: u32,
    sum: Vec<f32>,
    weight: Vec<f32>,
    subs: u32,
    /// Running sums of squared deviations, allocated only for rejection.
    squares: Option<Vec<f32>>,
    /// Absent without background matching; the first sub's levels once read.
    levels: Option<Option<[f32; 3]>>,
}

impl Stack {
    /// A stack that neither matches backgrounds nor rejects finite values.
    pub fn new(width: u32, height: u32) -> Self {
        let size = width as usize * height as usize;
        Self {
            width,
            height,
            sum: vec![0.0; 3 * size],
            weight: vec![0.0; size],
            subs: 0,
            squares: None,
            levels: None,
        }
    }

    /// Test each pixel against its running mean and sample deviation after
    /// `LEARNING_SUBS` samples. Any channel's outlier leaves the whole pixel out.
    pub fn with_rejection(mut self) -> Self {
        self.squares = Some(vec![0.0; self.sum.len()]);
        self
    }

    /// Shift each later sub to the first sub's levels, independently per channel.
    pub fn with_background_match(mut self) -> Self {
        self.levels = Some(None);
        self
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// Subs added so far, including those that reached no pixel.
    pub fn subs(&self) -> u32 {
        self.subs
    }

    /// Bilinearly sample the sub at each mapped pixel centre and add it in the
    /// same pass. A nonfinite value leaves that pixel out in all three channels.
    pub fn add(&mut self, sub: &Colour, placement: &Placement) -> Added {
        let offsets = if let Some(reference) = &mut self.levels {
            let current = levels(sub);
            match reference {
                Some(first) => std::array::from_fn(|channel| first[channel] - current[channel]),
                None => {
                    *reference = Some(current);
                    [0.0; 3]
                }
            }
        } else {
            [0.0; 3]
        };
        let mut added = Added {
            covered: 0,
            added: 0,
            rejected: 0,
            offsets,
        };
        self.subs += 1;
        if sub.width < 2 || sub.height < 2 {
            return added;
        }
        let max_x = f64::from(sub.width - 1);
        let max_y = f64::from(sub.height - 1);
        let size = self.weight.len();
        for y in 0..self.height {
            for x in 0..self.width {
                let (px, py) = placement.apply((f64::from(x) + 0.5, f64::from(y) + 0.5));
                let (u, v) = (px - 0.5, py - 0.5);
                if !(-EDGE_SLACK..=max_x + EDGE_SLACK).contains(&u)
                    || !(-EDGE_SLACK..=max_y + EDGE_SLACK).contains(&v)
                {
                    continue;
                }
                added.covered += 1;
                let (u, v) = (u.clamp(0.0, max_x), v.clamp(0.0, max_y));
                let x0 = (u.floor() as u32).min(sub.width - 2);
                let y0 = (v.floor() as u32).min(sub.height - 2);
                let (fx, fy) = (u - f64::from(x0), v - f64::from(y0));
                let neighbours = [
                    (x0, y0, (1.0 - fx) * (1.0 - fy)),
                    (x0 + 1, y0, fx * (1.0 - fy)),
                    (x0, y0 + 1, (1.0 - fx) * fy),
                    (x0 + 1, y0 + 1, fx * fy),
                ];
                let mut values = [0.0_f32; 3];
                for (channel, value) in values.iter_mut().enumerate() {
                    let mut interpolated = 0.0_f64;
                    for (nx, ny, share) in neighbours {
                        // Zero times NaN is still NaN: an unused neighbour must
                        // not be read, including on the last row and column.
                        if share != 0.0 {
                            interpolated += share * f64::from(sub.at(channel, nx, ny));
                        }
                    }
                    *value = interpolated as f32 + offsets[channel];
                }
                if values.iter().any(|value| !value.is_finite()) {
                    added.rejected += 1;
                    continue;
                }
                let index = y as usize * self.width as usize + x as usize;
                let weight = self.weight[index];
                if let Some(squares) = &self.squares {
                    if weight >= LEARNING_SUBS as f32
                        && values.iter().enumerate().any(|(channel, value)| {
                            let offset = channel * size + index;
                            let deviation = value - self.sum[offset] / weight;
                            let variance = squares[offset] / (weight - 1.0);
                            deviation * deviation
                                > REJECT_SIGMAS
                                    * REJECT_SIGMAS
                                    * variance.max(SIGMA_FLOOR * SIGMA_FLOOR)
                        })
                    {
                        added.rejected += 1;
                        continue;
                    }
                }
                for (channel, value) in values.into_iter().enumerate() {
                    let offset = channel * size + index;
                    let sum = &mut self.sum[offset];
                    if let Some(squares) = &mut self.squares {
                        // Welford's update avoids subtracting large squared
                        // sums in f32. Rejected pixels never reach this update.
                        let d1 = value - if weight > 0.0 { *sum / weight } else { 0.0 };
                        *sum += value;
                        let d2 = value - *sum / (weight + 1.0);
                        squares[offset] += d1 * d2;
                    } else {
                        *sum += value;
                    }
                }
                self.weight[index] = weight + 1.0;
                added.added += 1;
            }
        }
        added
    }

    /// How many subs were added at each pixel, row by row.
    pub fn weight(&self) -> &[f32] {
        &self.weight
    }

    /// Three planes of sums, laid out as a `Colour`'s.
    pub fn sum(&self) -> &[f32] {
        &self.sum
    }

    /// Sum over weight; 0.0 where the weight is 0.
    pub fn mean(&self) -> Colour {
        let mut mean = Colour::new(self.width, self.height);
        let size = self.weight.len();
        for channel in 0..3 {
            for (index, &weight) in self.weight.iter().enumerate() {
                if weight > 0.0 {
                    let offset = channel * size + index;
                    mean.planes[offset] = self.sum[offset] / weight;
                }
            }
        }
        mean
    }
}
