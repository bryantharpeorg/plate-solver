//! Keep saturated stars that cedar's own detection gates set aside.
//!
//! cedar-detect warns that a bright star bleeding into too many adjacent pixels
//! is rejected (`reference-solutions/cedar-detect/src/algorithm.rs`, Imaging
//! requirements). Its flat-top and size gates can therefore lose the brightest
//! stars. This separate pass leaves cedar's detections unchanged.
//!
//! The labelled regions, minimum area, second-moment axis ratio and pixel-centre
//! convention follow tetra3's spot extraction (`get_centroids_from_image` in
//! `reference-solutions/cedar-solve/tetra3/tetra3.py`). The finder camera's level,
//! 8-connectivity, frame bound, common trail and background ring are the measured
//! rules approved for 052. A region already holding a cedar star is passed over.

use crate::star::Star;
use std::collections::HashSet;

/// The finder camera's one saturation level.
pub const SATURATED: u8 = 250;
/// The smallest saturated region considered a star (tetra3's minimum area).
pub const MIN_PIXELS: usize = 5;
/// More saturated pixels than this set the whole frame aside.
pub const MAX_SATURATED_PIXELS: usize = 24_576;
/// The largest major-to-minor axis ratio considered round.
pub const MAX_AXIS_RATIO: f64 = 1.5;
/// Maximum axial difference from the frame's trail, in degrees.
pub const TRAIL_TOLERANCE_DEG: f64 = 10.0;
/// At least this many long regions must agree on a frame's trail.
pub const TRAIL_MIN_REGIONS: usize = 3;
/// Distance from the saturated box to its one-pixel background ring.
pub const RING_MARGIN: usize = 3;
/// Largest ring mean's share of the way from the frame median to saturation.
pub const MAX_RING_SHARE: f64 = 0.5;

/// Why a saturated region was not kept as a star.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SetAside {
    Small,
    Edge,
    Elongated,
    Extended,
}

/// Add the kept saturated stars before the first fainter star in `stars`.
/// Existing stars retain their order and precede new stars of equal brightness.
pub fn with_saturated_stars(
    image: &[u8],
    width: usize,
    height: usize,
    stars: Vec<Star>,
) -> Vec<Star> {
    let (kept, _) = saturated_stars(image, width, height, &stars);
    if kept.is_empty() {
        return stars;
    }
    let mut combined = Vec::with_capacity(stars.len() + kept.len());
    let mut kept = kept.into_iter().peekable();
    for star in stars {
        while kept.peek().is_some_and(|s| s.brightness > star.brightness) {
            combined.push(kept.next().unwrap());
        }
        combined.push(star);
    }
    combined.extend(kept);
    combined
}

/// Find missing saturated stars, brightest first, and set-aside reasons in the
/// reading order of the regions' first pixels. Lone pixels and regions whose
/// boxes hold a star of `stars` are neither kept nor set aside.
///
/// # Panics
/// Panics when `image.len()` differs from `width * height`.
pub fn saturated_stars(
    image: &[u8],
    width: usize,
    height: usize,
    stars: &[Star],
) -> (Vec<Star>, Vec<SetAside>) {
    assert_eq!(image.len(), width * height);
    // Retain only saturated indices, never a frame-sized visited mask. Stop
    // before following any region in a washed-out frame.
    let mut pixels = Vec::new();
    for (index, &value) in image.iter().enumerate() {
        if value >= SATURATED {
            if pixels.len() == MAX_SATURATED_PIXELS {
                return (Vec::new(), vec![SetAside::Extended]);
            }
            pixels.push(index);
        }
    }
    if pixels.is_empty() {
        return (Vec::new(), Vec::new());
    }
    let median = sampled_median(image);
    let mut remaining: HashSet<usize> = pixels.iter().copied().collect();
    let mut pending = Vec::new();
    let mut regions = Vec::new();
    for first in pixels {
        if !remaining.remove(&first) {
            continue;
        }
        let mut region = Region::new(first % width, first / width);
        pending.push(first);
        while let Some(index) = pending.pop() {
            let (x, y) = (index % width, index / width);
            region.add(x, y, image[index]);
            for ny in y.saturating_sub(1)..=(y + 1).min(height - 1) {
                for nx in x.saturating_sub(1)..=(x + 1).min(width - 1) {
                    let neighbor = ny * width + nx;
                    if remaining.remove(&neighbor) {
                        pending.push(neighbor);
                    }
                }
            }
        }
        if region.count == 1
            || stars.iter().any(|s| {
                s.x >= region.left as f64
                    && s.x <= (region.right + 1) as f64
                    && s.y >= region.top as f64
                    && s.y <= (region.bottom + 1) as f64
            })
        {
            continue;
        }
        if region.count < MIN_PIXELS {
            regions.push(Err(SetAside::Small));
            continue;
        }
        if region.left < RING_MARGIN
            || region.top < RING_MARGIN
            || region.right + RING_MARGIN >= width
            || region.bottom + RING_MARGIN >= height
        {
            regions.push(Err(SetAside::Edge));
            continue;
        }
        let n = region.count as f64;
        let cx = region.sum_x / n;
        let cy = region.sum_y / n;
        let vxx = region.sum_xx / n - cx * cx;
        let vyy = region.sum_yy / n - cy * cy;
        let vxy = region.sum_xy / n - cx * cy;
        let spread = ((vxx - vyy).powi(2) / 4.0 + vxy * vxy).sqrt();
        let major = (vxx + vyy) / 2.0 + spread;
        let minor = (vxx + vyy) / 2.0 - spread;
        if minor <= 0.0 {
            regions.push(Err(SetAside::Elongated));
            continue;
        }
        let axis = ((major / minor).sqrt() > MAX_AXIS_RATIO)
            .then(|| 0.5 * (2.0 * vxy).atan2(vxx - vyy).to_degrees());
        regions.push(Ok(Candidate { region, axis }));
    }

    let axes: Vec<_> = regions
        .iter()
        .filter_map(|r| r.as_ref().ok()?.axis)
        .collect();
    let trail = frame_trail(&axes);
    let mut kept = Vec::new();
    let mut set_aside = Vec::new();
    for candidate in regions {
        let candidate = match candidate {
            Ok(candidate) => candidate,
            Err(reason) => {
                set_aside.push(reason);
                continue;
            }
        };
        if candidate.axis.is_some_and(|axis| {
            !trail.is_some_and(|t| axial_distance(axis, t) <= TRAIL_TOLERANCE_DEG)
        }) {
            set_aside.push(SetAside::Elongated);
            continue;
        }
        let region = candidate.region;
        let (left, right) = (region.left - RING_MARGIN, region.right + RING_MARGIN);
        let (top, bottom) = (region.top - RING_MARGIN, region.bottom + RING_MARGIN);
        let mut ring_sum = 0.0;
        for x in left..=right {
            ring_sum += f64::from(image[top * width + x]);
            ring_sum += f64::from(image[bottom * width + x]);
        }
        for y in top + 1..bottom {
            ring_sum += f64::from(image[y * width + left]);
            ring_sum += f64::from(image[y * width + right]);
        }
        let ring_count = 2 * (right - left + bottom - top);
        let ring_mean = ring_sum / ring_count as f64;
        if ring_mean - median > MAX_RING_SHARE * (f64::from(SATURATED) - median) {
            set_aside.push(SetAside::Extended);
            continue;
        }
        let mut brightness = 0.0;
        for y in top + 1..bottom {
            for x in left + 1..right {
                brightness += f64::from(image[y * width + x]) - ring_mean;
            }
        }
        kept.push(Star::new(
            region.sum_x / region.count as f64 + 0.5,
            region.sum_y / region.count as f64 + 0.5,
            region.peak,
            brightness,
            region.count,
        ));
    }
    kept.sort();
    (kept, set_aside)
}

struct Candidate {
    region: Region,
    axis: Option<f64>,
}

struct Region {
    count: usize,
    sum_x: f64,
    sum_y: f64,
    sum_xx: f64,
    sum_yy: f64,
    sum_xy: f64,
    left: usize,
    right: usize,
    top: usize,
    bottom: usize,
    peak: u8,
}

impl Region {
    fn new(x: usize, y: usize) -> Self {
        Self {
            count: 0,
            sum_x: 0.0,
            sum_y: 0.0,
            sum_xx: 0.0,
            sum_yy: 0.0,
            sum_xy: 0.0,
            left: x,
            right: x,
            top: y,
            bottom: y,
            peak: 0,
        }
    }

    fn add(&mut self, x: usize, y: usize, value: u8) {
        self.count += 1;
        self.left = self.left.min(x);
        self.right = self.right.max(x);
        self.top = self.top.min(y);
        self.bottom = self.bottom.max(y);
        self.peak = self.peak.max(value);
        let (x, y) = (x as f64, y as f64);
        self.sum_x += x;
        self.sum_y += y;
        self.sum_xx += x * x;
        self.sum_yy += y * y;
        self.sum_xy += x * y;
    }
}

fn sampled_median(image: &[u8]) -> f64 {
    let mut counts = [0; 256];
    for &value in image.iter().step_by(64) {
        counts[value as usize] += 1;
    }
    let mut cumulative = 0;
    for (value, count) in counts.into_iter().enumerate() {
        cumulative += count;
        if cumulative > image.len().div_ceil(64) / 2 {
            return value as f64;
        }
    }
    unreachable!("a frame with a saturated pixel has at least one sample")
}

fn wrapped_degrees(angle: f64) -> f64 {
    (angle + 180.0).rem_euclid(360.0) - 180.0
}

fn axial_distance(a: f64, b: f64) -> f64 {
    wrapped_degrees(2.0 * (a - b)).abs() / 2.0
}

fn frame_trail(axes: &[f64]) -> Option<f64> {
    if axes.len() < TRAIL_MIN_REGIONS {
        return None;
    }
    let (mut sin_sum, mut cos_sum) = (0.0, 0.0);
    for &axis in axes {
        let doubled = (2.0 * axis).to_radians();
        sin_sum += doubled.sin();
        cos_sum += doubled.cos();
    }
    let mean = sin_sum.atan2(cos_sum).to_degrees();
    let mut deviations: Vec<_> = axes
        .iter()
        .map(|axis| wrapped_degrees(2.0 * axis - mean))
        .collect();
    deviations.sort_by(f64::total_cmp);
    let middle = deviations.len() / 2;
    let median = if deviations.len().is_multiple_of(2) {
        (deviations[middle - 1] + deviations[middle]) / 2.0
    } else {
        deviations[middle]
    };
    let trail = (mean + median) / 2.0;
    (axes
        .iter()
        .filter(|&&axis| axial_distance(axis, trail) <= TRAIL_TOLERANCE_DEG)
        .count()
        >= TRAIL_MIN_REGIONS)
        .then_some(trail)
}
