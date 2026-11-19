//! Stacking arithmetic (035): colour from a mosaic and where a sub falls on the
//! reference grid. Pure functions; nothing here reads a file, a clock, the mount
//! or the site.

use super::{calibration::Mosaic, solver::ImagingSolve};

/// At each corner of the reference grid the placing map may lie this many of the
/// sub's pixels from where the two imaging solves put that corner (owner decision 3).
pub const CORNER_SLACK_PX: f64 = 0.5;

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
