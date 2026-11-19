//! Calibrating a sub on its mosaic (034): dark, flat, bad pixels, in that order, before
//! any colour. Pure functions; nothing here reads a file or a clock.

use std::{collections::HashSet, fmt};

use serde::Serialize;

pub mod library;

/// A hot sample stands this many noises above its largest same-colour neighbour.
pub const HOT_SIGMAS: f32 = 8.0;
/// Its height above its colour's median exceeds this times its neighbour's height.
pub const HOT_RATIO: f32 = 5.0;
/// At most this many touching samples may be lit around a hot sample.
pub const HOT_LIT_NEIGHBOURS: usize = 1;
/// A lit touching sample exceeds this many of its own colour's noises.
pub const ADJACENT_SIGMAS: f32 = 3.0;
/// It also exceeds this share of the candidate's height above its colour's median.
pub const ADJACENT_SHARE: f32 = 0.1;
/// A flat under this is not divided by; its sample is repaired instead.
pub const FLAT_FLOOR: f32 = 0.1;
/// The least a noise or a spread may be: one count, the samples' own step.
pub const NOISE_FLOOR: f32 = 1.0;

/// One frame's samples as real numbers, row by row from the top-left.
#[derive(Debug, Clone, PartialEq)]
pub struct Mosaic {
    pub width: u32,
    pub height: u32,
    /// `"RGGB"`, `"BGGR"`, `"GRBG"` or `"GBRG"`, or `None` for a mono sensor.
    pub bayer: Option<String>,
    pub samples: Vec<f32>,
}

impl Mosaic {
    /// From little-endian samples of 8 or 16 bits. `None` when `bits` is neither,
    /// or the length is not width × height × bits / 8.
    pub fn from_samples(
        samples: &[u8],
        width: u32,
        height: u32,
        bits: u32,
        bayer: Option<&str>,
    ) -> Option<Self> {
        let bytes = match bits {
            8 => 1,
            16 => 2,
            _ => return None,
        };
        let need = (width as usize)
            .checked_mul(height as usize)?
            .checked_mul(bytes)?;
        if samples.len() != need {
            return None;
        }
        let samples = if bits == 8 {
            samples.iter().map(|&v| f32::from(v)).collect()
        } else {
            samples
                .as_chunks::<2>()
                .0
                .iter()
                .map(|&v| f32::from(u16::from_le_bytes(v)))
                .collect()
        };
        Some(Self {
            width,
            height,
            bayer: bayer.map(str::to_owned),
            samples,
        })
    }

    /// 0 red, 1 green, 2 blue, from the pattern's top-left 2 × 2 cell.
    /// Both greens are one colour; a mono sample is always 0.
    pub fn colour(&self, x: u32, y: u32) -> usize {
        match self.bayer.as_deref() {
            Some(pattern) => match pattern.as_bytes()[((y % 2) * 2 + x % 2) as usize] {
                b'G' => 1,
                b'B' => 2,
                _ => 0,
            },
            None => 0,
        }
    }

    /// 3 for a mosaic, 1 for mono.
    pub fn colours(&self) -> usize {
        if self.bayer.is_some() {
            3
        } else {
            1
        }
    }

    /// Distance along a row or column to the nearest same-colour sample.
    pub fn step(&self) -> u32 {
        if self.bayer.is_some() {
            2
        } else {
            1
        }
    }

    fn sample(&self, x: u32, y: u32) -> f32 {
        self.samples[y as usize * self.width as usize + x as usize]
    }

    fn colour_samples(&self) -> Vec<Vec<f32>> {
        let mut colours = vec![Vec::new(); self.colours()];
        for y in 0..self.height {
            for x in 0..self.width {
                colours[self.colour(x, y)].push(self.sample(x, y));
            }
        }
        colours
    }
}

/// The value at index ⌊p·(n − 1)⌋ of the values sorted with `f32::total_cmp`,
/// with p from 0 to 1. `None` for no values.
pub fn percentile(values: &[f32], p: f64) -> Option<f32> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable_by(f32::total_cmp);
    Some(sorted[(p * (sorted.len() - 1) as f64).floor() as usize])
}

/// A colour's median and its noise, floored at one count.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Level {
    pub median: f32,
    pub noise: f32,
}

/// One `Level` a colour, in colour order. Noise is median less 16th percentile.
pub fn levels(mosaic: &Mosaic) -> Vec<Level> {
    mosaic
        .colour_samples()
        .iter()
        .map(|values| {
            let median = percentile(values, 0.5).unwrap_or(0.0);
            Level {
                median,
                noise: (median - percentile(values, 0.16).unwrap_or(0.0)).max(NOISE_FLOOR),
            }
        })
        .collect()
}

/// What a sub is calibrated with. Each slice is row by row, as the sub is.
#[derive(Debug, Clone, Copy, Default)]
pub struct Masters<'a> {
    pub dark: Option<&'a [f32]>,
    /// Already normalised.
    pub flat: Option<&'a [f32]>,
    pub bad_pixels: Option<&'a [(u32, u32)]>,
}

/// What `calibrate` did. Serialized with these six keys, in this order.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Report {
    /// `applied` or `missing`.
    pub dark: &'static str,
    /// `applied` or `missing`.
    pub flat: &'static str,
    /// `map` for a supplied map, `sub` for a search in the sub itself.
    pub bad_pixels: &'static str,
    pub repaired: u64,
    pub not_repaired: u64,
    pub low_flat: u64,
}

/// A master's sample count does not fit the sub. No calibration has run.
#[derive(Debug, Clone, PartialEq)]
pub struct SizeMismatch {
    pub master: &'static str,
    pub have: usize,
    pub need: usize,
}

impl fmt::Display for SizeMismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the master {} holds {} samples and the sub {}",
            self.master, self.have, self.need
        )
    }
}

impl std::error::Error for SizeMismatch {}

/// Subtract the dark, divide by the flat, then repair bad samples.
/// Validate both masters before changing any sample. Negative counts are kept.
pub fn calibrate(sub: &mut Mosaic, masters: &Masters<'_>) -> Result<Report, SizeMismatch> {
    for (master, values) in [("dark", masters.dark), ("flat", masters.flat)] {
        if let Some(values) = values {
            if values.len() != sub.samples.len() {
                return Err(SizeMismatch {
                    master,
                    have: values.len(),
                    need: sub.samples.len(),
                });
            }
        }
    }
    if let Some(dark) = masters.dark {
        for (sample, dark) in sub.samples.iter_mut().zip(dark) {
            *sample -= dark;
        }
    }
    let mut bad = Vec::new();
    if let Some(flat) = masters.flat {
        for (i, (sample, &flat)) in sub.samples.iter_mut().zip(flat).enumerate() {
            if flat >= FLAT_FLOOR {
                *sample /= flat;
            } else {
                bad.push((i as u32 % sub.width, i as u32 / sub.width));
            }
        }
    }
    let low_flat = bad.len() as u64;
    match masters.bad_pixels {
        Some(map) => bad.extend_from_slice(map),
        None => bad.extend(find_hot_pixels(sub)),
    }
    let (repaired, not_repaired) = repair(sub, &bad);
    Ok(Report {
        dark: if masters.dark.is_some() {
            "applied"
        } else {
            "missing"
        },
        flat: if masters.flat.is_some() {
            "applied"
        } else {
            "missing"
        },
        bad_pixels: if masters.bad_pixels.is_some() {
            "map"
        } else {
            "sub"
        },
        repaired,
        not_repaired,
        low_flat,
    })
}

/// The four same-colour row and column neighbours that are inside the frame.
fn neighbours(mosaic: &Mosaic, x: u32, y: u32) -> impl Iterator<Item = (u32, u32)> + '_ {
    let step = mosaic.step();
    [
        x.checked_sub(step).map(|x| (x, y)),
        x.checked_add(step).map(|x| (x, y)),
        y.checked_sub(step).map(|y| (x, y)),
        y.checked_add(step).map(|y| (x, y)),
    ]
    .into_iter()
    .flatten()
    .filter(|&(x, y)| x < mosaic.width && y < mosaic.height)
}

/// Hot samples, row by row: above the largest same-colour neighbour by more
/// than eight noises and five times its height, with at most one lit touching sample.
pub fn find_hot_pixels(mosaic: &Mosaic) -> Vec<(u32, u32)> {
    hot_pixels(mosaic, &levels(mosaic))
}

fn hot_pixels(mosaic: &Mosaic, levels: &[Level]) -> Vec<(u32, u32)> {
    let mut hot = Vec::new();
    for y in 0..mosaic.height {
        for x in 0..mosaic.width {
            let v = mosaic.sample(x, y);
            let level = levels[mosaic.colour(x, y)];
            let Some(n) = neighbours(mosaic, x, y)
                .map(|(x, y)| mosaic.sample(x, y))
                .reduce(f32::max)
            else {
                continue;
            };
            let height = v - level.median;
            if v - n > HOT_SIGMAS * level.noise && height > HOT_RATIO * (n - level.median).max(0.0)
            {
                let mut lit = 0;
                for ny in y.saturating_sub(1)..=y.saturating_add(1).min(mosaic.height - 1) {
                    for nx in x.saturating_sub(1)..=x.saturating_add(1).min(mosaic.width - 1) {
                        if (nx, ny) == (x, y) {
                            continue;
                        }
                        let adjacent = levels[mosaic.colour(nx, ny)];
                        if mosaic.sample(nx, ny) - adjacent.median
                            > (ADJACENT_SIGMAS * adjacent.noise).max(ADJACENT_SHARE * height)
                        {
                            lit += 1;
                        }
                    }
                }
                if lit <= HOT_LIT_NEIGHBOURS {
                    hot.push((x, y));
                }
            }
        }
    }
    hot
}

/// How many samples stand more than `HOT_SIGMAS` noises above their colour's
/// median without being hot pixels: light that reached the sensor.
pub fn lit_samples(mosaic: &Mosaic) -> u64 {
    let levels = levels(mosaic);
    let hot: HashSet<_> = hot_pixels(mosaic, &levels).into_iter().collect();
    let mut count = 0;
    for y in 0..mosaic.height {
        for x in 0..mosaic.width {
            let level = levels[mosaic.colour(x, y)];
            if mosaic.sample(x, y) - level.median > HOT_SIGMAS * level.noise
                && !hot.contains(&(x, y))
            {
                count += 1;
            }
        }
    }
    count
}

/// Replace each distinct listed sample inside the frame with the median of its
/// good same-colour neighbours. Answers (repaired, not repaired).
pub fn repair(mosaic: &mut Mosaic, bad: &[(u32, u32)]) -> (u64, u64) {
    let bad: HashSet<_> = bad
        .iter()
        .copied()
        .filter(|&(x, y)| x < mosaic.width && y < mosaic.height)
        .collect();
    let mut repaired = 0;
    for &(x, y) in &bad {
        let mut good: Vec<_> = neighbours(mosaic, x, y)
            .filter(|point| !bad.contains(point))
            .map(|(x, y)| mosaic.sample(x, y))
            .collect();
        if good.is_empty() {
            continue;
        }
        good.sort_unstable_by(f32::total_cmp);
        let mid = good.len() / 2;
        let value = if good.len().is_multiple_of(2) {
            (good[mid - 1] + good[mid]) / 2.0
        } else {
            good[mid]
        };
        mosaic.samples[y as usize * mosaic.width as usize + x as usize] = value;
        repaired += 1;
    }
    (repaired, bad.len() as u64 - repaired)
}

/// Divide each colour by its own median and answer the medians in colour order.
/// A colour whose median is not above zero becomes zero.
pub fn normalise_flat(flat: &mut Mosaic) -> Vec<f32> {
    let medians: Vec<_> = flat
        .colour_samples()
        .iter()
        .map(|values| percentile(values, 0.5).unwrap_or(0.0))
        .collect();
    for y in 0..flat.height {
        for x in 0..flat.width {
            let median = medians[flat.colour(x, y)];
            let sample = &mut flat.samples[y as usize * flat.width as usize + x as usize];
            *sample = if median > 0.0 { *sample / median } else { 0.0 };
        }
    }
    medians
}
