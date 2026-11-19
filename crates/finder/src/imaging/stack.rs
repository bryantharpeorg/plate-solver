//! Stacking arithmetic (035): colour from a mosaic, where a sub falls on the
//! reference grid, and the sums that make a stack. Pure functions; nothing here
//! reads a file, a clock, the mount or the site.

use super::{calibration::Mosaic, solver::ImagingSolve};

/// At each corner of the reference grid the placing map may lie this many of the
/// sub's pixels from where the two imaging solves put that corner (owner decision 3).
pub const CORNER_SLACK_PX: f64 = 0.5;
/// A sample point this close to the edge of a sub's pixel centres counts as on it.
pub const EDGE_SLACK: f64 = 1e-6;

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
        }
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
        let mut added = Added {
            covered: 0,
            added: 0,
            rejected: 0,
            offsets: [0.0; 3],
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
                    *value = interpolated as f32;
                }
                if values.iter().any(|value| !value.is_finite()) {
                    added.rejected += 1;
                    continue;
                }
                let index = y as usize * self.width as usize + x as usize;
                for (channel, value) in values.into_iter().enumerate() {
                    self.sum[channel * size + index] += value;
                }
                self.weight[index] += 1.0;
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
