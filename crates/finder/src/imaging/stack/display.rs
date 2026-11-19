//! The stack as a picture (035): reduced, flattened, balanced, stretched. Display only;
//! nothing here changes what is saved as FITS.

use super::{Colour, Stack};
use crate::imaging::{calibration::percentile, preview};

pub const DISPLAY_WIDTH: u32 = 1920;
pub const MCP_WIDTH: u32 = 1024;
pub const TILES: u32 = 8;
pub const DEFAULT_STRETCH: f64 = 10.0;
/// White is this percentile of what is left above the background.
pub const WHITE_PERCENTILE: f64 = 0.999;
pub const NOISE_SPANS: f64 = 30.0;

/// A stack reduced by whole blocks to a mean, and which pixels any sub reached.
#[derive(Debug, Clone, PartialEq)]
pub struct Reduced {
    pub colour: Colour,
    pub covered: Vec<bool>,
}

/// A background level of `a·x + b·y + c` at position (x, y).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plane {
    pub a: f64,
    pub b: f64,
    pub c: f64,
}

impl Plane {
    fn flat(c: f64) -> Self {
        Self { a: 0.0, b: 0.0, c }
    }

    fn at(self, x: f64, y: f64) -> f64 {
        self.a * x + self.b * y + self.c
    }
}

/// Reduce the stack's sums and weights directly, without a full-size mean.
/// Each whole block contributes its summed sums divided by its summed weights.
pub fn reduce(sum: &[f32], weight: &[f32], width: u32, height: u32, max_width: u32) -> Reduced {
    let k = block_size(width, max_width);
    let mut colour = Colour::new(width / k, height / k);
    let mut covered = Vec::with_capacity((colour.width * colour.height) as usize);
    let size = width as usize * height as usize;
    for y in 0..colour.height {
        for x in 0..colour.width {
            let mut sums = [0.0_f64; 3];
            let mut weights = 0.0;
            for by in 0..k {
                for bx in 0..k {
                    let i = ((k * y + by) * width + k * x + bx) as usize;
                    weights += f64::from(weight[i]);
                    for (channel, total) in sums.iter_mut().enumerate() {
                        *total += f64::from(sum[channel * size + i]);
                    }
                }
            }
            covered.push(weights > 0.0);
            if weights > 0.0 {
                for (channel, total) in sums.into_iter().enumerate() {
                    colour.set(channel, x, y, (total / weights) as f32);
                }
            }
        }
    }
    Reduced { colour, covered }
}

fn block_size(width: u32, max_width: u32) -> u32 {
    assert!(max_width > 0, "a picture's maximum width must be positive");
    width.div_ceil(max_width).max(1)
}

/// Fit a plane through the darker half of an 8 × 8 grid's lower medians.
pub fn background(reduced: &Reduced) -> [Plane; 3] {
    let colour = &reduced.colour;
    let (tw, th) = (colour.width / TILES, colour.height / TILES);
    std::array::from_fn(|channel| {
        let mut tiles = Vec::new();
        if tw > 0 && th > 0 {
            for q in 0..TILES {
                for p in 0..TILES {
                    let mut values = Vec::new();
                    for y in q * th..(q + 1) * th {
                        for x in p * tw..(p + 1) * tw {
                            if reduced.covered[(y * colour.width + x) as usize] {
                                values.push(colour.at(channel, x, y));
                            }
                        }
                    }
                    if values.len() * 2 >= (tw * th) as usize {
                        tiles.push((
                            (f64::from(p) + 0.5) * f64::from(tw),
                            (f64::from(q) + 0.5) * f64::from(th),
                            percentile(&values, 0.5).expect("a half-covered tile"),
                        ));
                    }
                }
            }
        }
        let levels: Vec<_> = tiles.iter().map(|tile| tile.2).collect();
        let Some(median) = percentile(&levels, 0.5) else {
            let values: Vec<_> = colour
                .plane(channel)
                .iter()
                .zip(&reduced.covered)
                .filter_map(|(&value, &covered)| covered.then_some(value))
                .collect();
            return Plane::flat(f64::from(percentile(&values, 0.5).unwrap_or(0.0)));
        };
        tiles.retain(|tile| tile.2 <= median);
        fit(&tiles).unwrap_or_else(|| {
            let levels: Vec<_> = tiles.iter().map(|tile| tile.2).collect();
            Plane::flat(f64::from(percentile(&levels, 0.5).expect("kept tiles")))
        })
    })
}

/// The three normal equations, solved with partial pivoting after checking
/// their determinant for a line (or fewer than three samples).
fn fit(tiles: &[(f64, f64, f32)]) -> Option<Plane> {
    if tiles.len() < 3 {
        return None;
    }
    let mut matrix = [[0.0; 4]; 3];
    for &(x, y, level) in tiles {
        let row = [x, y, 1.0];
        for i in 0..3 {
            for j in 0..3 {
                matrix[i][j] += row[i] * row[j];
            }
            matrix[i][3] += row[i] * f64::from(level);
        }
    }
    let m = &matrix;
    let determinant = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    if determinant.abs() < 1e-9 * m[0][0] * m[1][1] * m[2][2] {
        return None;
    }
    for column in 0..3 {
        let pivot = (column..3)
            .max_by(|&a, &b| matrix[a][column].abs().total_cmp(&matrix[b][column].abs()))?;
        matrix.swap(column, pivot);
        let scale = matrix[column][column];
        for value in &mut matrix[column][column..] {
            *value /= scale;
        }
        let pivot_row = matrix[column];
        for (row, values) in matrix.iter_mut().enumerate() {
            if row != column {
                let scale = values[column];
                for (value, pivot) in values[column..].iter_mut().zip(&pivot_row[column..]) {
                    *value -= scale * pivot;
                }
            }
        }
    }
    Some(Plane {
        a: matrix[0][3],
        b: matrix[1][3],
        c: matrix[2][3],
    })
}

/// Remove each plane, apply gains, and asinh-stretch about the channel medians
/// with one span shared by all three channels. Uncovered pixels stay black.
pub fn tone(reduced: &Reduced, planes: &[Plane; 3], gains: [f64; 3], stretch: f64) -> preview::Rgb {
    let colour = &reduced.colour;
    let value = |channel: usize, i: usize| {
        let x = (i % colour.width as usize) as f64 + 0.5;
        let y = (i / colour.width as usize) as f64 + 0.5;
        gains[channel] * (f64::from(colour.plane(channel)[i]) - planes[channel].at(x, y))
    };
    let mut medians = [0.0; 3];
    let mut span = 1.0_f64;
    for (channel, median) in medians.iter_mut().enumerate() {
        // Percentiles use 034's lower-order statistic, a channel at a time.
        let values: Vec<_> = reduced
            .covered
            .iter()
            .enumerate()
            .filter(|&(_, &covered)| covered)
            .map(|(i, _)| value(channel, i) as f32)
            .collect();
        *median = f64::from(percentile(&values, 0.5).unwrap_or(0.0));
        let noise = *median - f64::from(percentile(&values, 0.16).unwrap_or(0.0));
        let white = f64::from(percentile(&values, WHITE_PERCENTILE).unwrap_or(0.0));
        span = span.max(white - *median).max(NOISE_SPANS * noise);
    }
    let mut pixels = vec![0; reduced.covered.len() * 3];
    let scale = 255.0 / stretch.asinh();
    for (i, &covered) in reduced.covered.iter().enumerate() {
        if covered {
            for channel in 0..3 {
                let t = ((value(channel, i) - medians[channel]) / span).clamp(0.0, 1.0);
                pixels[3 * i + channel] = ((stretch * t).asinh() * scale).round() as u8;
            }
        }
    }
    preview::Rgb {
        width: colour.width,
        height: colour.height,
        pixels,
    }
}

/// Reduce the sums and weights, fit the background and tone the result.
pub fn render(stack: &Stack, gains: [f64; 3], stretch: f64, max_width: u32) -> preview::Rgb {
    let reduced = reduce(
        stack.sum(),
        stack.weight(),
        stack.width(),
        stack.height(),
        max_width,
    );
    tone(&reduced, &background(&reduced), gains, stretch)
}

/// Average each whole block of bytes, rounding halves away from zero.
pub fn shrink(picture: &preview::Rgb, max_width: u32) -> preview::Rgb {
    let k = block_size(picture.width, max_width);
    let (width, height) = (picture.width / k, picture.height / k);
    let mut pixels = Vec::with_capacity((width * height) as usize * 3);
    for y in 0..height {
        for x in 0..width {
            let mut sums = [0.0_f64; 3];
            for by in 0..k {
                for bx in 0..k {
                    let i = (((k * y + by) * picture.width + k * x + bx) * 3) as usize;
                    for (channel, sum) in sums.iter_mut().enumerate() {
                        *sum += f64::from(picture.pixels[i + channel]);
                    }
                }
            }
            pixels.extend(sums.map(|sum| (sum / (f64::from(k) * f64::from(k))).round() as u8));
        }
    }
    preview::Rgb {
        width,
        height,
        pixels,
    }
}
