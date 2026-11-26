//! The finder camera's focus aid (051): the size of the stars in the finder
//! camera's raw frames, read beside the live solve and judged by 033's own
//! arithmetic (`imaging::focus::aid::Aid`). It changes no setting of the finder
//! camera and takes nothing from the live solve; each reading costs one raw
//! frame on the scope link (025).

use crate::imaging::focus::{aid::Sample, FAR_OUT_PX, MIN_STARS};
use scope_link::proto::Centroid;

/// The brightest unsaturated stars measured in a frame (US1).
pub const STARS_MEASURED: usize = 15;
/// Half the side of the square box each star is measured in, in pixels (US1).
pub const BOX_PX: i64 = 16;
/// A star with a joined pixel at or above this level is not measured (US1).
pub const SATURATED: u8 = 250;
/// A pixel joins a star only this many noise levels above the background (US1).
pub const THRESHOLD_NOISE: f64 = 3.0;
/// Each pixel is summed as this many sub-pixels a side when a star's flux is
/// counted in circles (US1).
pub const SUBPIXELS: u32 = 5;
/// A star's circle reaches this far past its farthest joined pixel, in pixels (US1).
pub const APERTURE_MARGIN_PX: f64 = 1.0;

const SIDE: usize = (2 * BOX_PX + 1) as usize;

/// One raw frame measured at its own star frame's centroids, the node's
/// detection (US1): a reading, or why there is none, as 033's `Sample` names
/// it. The centroids are brightest first; a star is measured when its box lies
/// inside the frame and holds no other centroid, and its own pixels are under
/// `SATURATED` and have flux; the first `STARS_MEASURED` such stars are
/// measured, each as its half-flux diameter in sub-pixel circles.
pub fn measure(pixels: &[u8], width: u32, height: u32, centroids: &[Centroid]) -> Sample {
    assert_eq!(pixels.len(), width as usize * height as usize);
    let mut sizes = Vec::with_capacity(STARS_MEASURED);
    for (index, star) in centroids.iter().enumerate() {
        if !star.x.is_finite() || !star.y.is_finite() {
            continue;
        }
        let (x, y) = (star.x.floor() as i64, star.y.floor() as i64);
        if x < BOX_PX
            || y < BOX_PX
            || x >= i64::from(width) - BOX_PX
            || y >= i64::from(height) - BOX_PX
            || centroids.iter().enumerate().any(|(other_index, other)| {
                index != other_index
                    && (other.x - star.x).abs() <= BOX_PX as f64
                    && (other.y - star.y).abs() <= BOX_PX as f64
            })
        {
            continue;
        }
        if let Some(size) = star_size(pixels, width as usize, x, y) {
            sizes.push(size);
            if sizes.len() == STARS_MEASURED {
                break;
            }
        }
    }
    let stars = sizes.len() as u32;
    if stars < MIN_STARS {
        return Sample::TooFewStars { stars };
    }
    let px = median(&mut sizes);
    if px > FAR_OUT_PX {
        Sample::FarOut { px, stars }
    } else {
        Sample::Reading { px, stars }
    }
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_unstable_by(f64::total_cmp);
    (values[(values.len() - 1) / 2] + values[values.len() / 2]) / 2.0
}

// Offsets from the centre of the box's centre pixel, not the supplied centroid.
fn offset(index: usize) -> (f64, f64) {
    (
        (index % SIDE) as f64 - BOX_PX as f64,
        (index / SIDE) as f64 - BOX_PX as f64,
    )
}

fn star_size(pixels: &[u8], width: usize, x: i64, y: i64) -> Option<f64> {
    let mut patch = [0.0; SIDE * SIDE];
    let mut edge = Vec::with_capacity(4 * (SIDE - 1));
    for row in 0..SIDE {
        for column in 0..SIDE {
            let value = f64::from(
                pixels[(y - BOX_PX + row as i64) as usize * width
                    + (x - BOX_PX + column as i64) as usize],
            );
            patch[row * SIDE + column] = value;
            if row == 0 || column == 0 || row == SIDE - 1 || column == SIDE - 1 {
                edge.push(value);
            }
        }
    }
    let background = median(&mut edge);
    for value in &mut edge {
        *value = (*value - background).abs();
    }
    let noise = (1.4826 * median(&mut edge)).max(0.5);
    let level = background + THRESHOLD_NOISE * noise;

    // A strict comparison preserves the first in reading order on a tie.
    let middle = BOX_PX as usize;
    let mut start = (middle - 2) * SIDE + middle - 2;
    for row in middle - 2..=middle + 2 {
        for column in middle - 2..=middle + 2 {
            let index = row * SIDE + column;
            if patch[index] > patch[start] {
                start = index;
            }
        }
    }
    if patch[start] <= level {
        return None;
    }

    // Only the 8-connected component of the start pixel belongs to the star.
    let mut seen = [false; SIDE * SIDE];
    let mut pending = vec![start];
    let mut own = Vec::new();
    seen[start] = true;
    while let Some(index) = pending.pop() {
        if patch[index] >= f64::from(SATURATED) {
            return None;
        }
        own.push(index);
        let (row, column) = (index / SIDE, index % SIDE);
        for r in row.saturating_sub(1)..=(row + 1).min(SIDE - 1) {
            for c in column.saturating_sub(1)..=(column + 1).min(SIDE - 1) {
                let neighbor = r * SIDE + c;
                if !seen[neighbor] && patch[neighbor] > level {
                    seen[neighbor] = true;
                    pending.push(neighbor);
                }
            }
        }
    }

    let (mut flux, mut cx, mut cy) = (0.0, 0.0, 0.0);
    for &index in &own {
        let value = patch[index] - background;
        let (dx, dy) = offset(index);
        flux += value;
        cx += dx * value;
        cy += dy * value;
    }
    cx /= flux;
    cy /= flux;
    let farthest = own
        .iter()
        .map(|&index| {
            let (dx, dy) = offset(index);
            (dx - cx).hypot(dy - cy)
        })
        .fold(0.0_f64, f64::max);
    let radius = (farthest + APERTURE_MARGIN_PX).min((BOX_PX - 1) as f64);

    // SEP's flux_radius method: sub-pixel circles, then linear interpolation
    // of the half-flux crossing. Written from the description, without SEP source.
    let mut parts = Vec::new();
    for (index, &value) in patch.iter().enumerate() {
        let (dx, dy) = offset(index);
        if (dx - cx).hypot(dy - cy) > radius + 1.0 {
            continue;
        }
        let part_flux = (value - background) / f64::from(SUBPIXELS * SUBPIXELS);
        for b in 0..SUBPIXELS {
            for a in 0..SUBPIXELS {
                let px = dx - 0.5 + (f64::from(a) + 0.5) / f64::from(SUBPIXELS);
                let py = dy - 0.5 + (f64::from(b) + 0.5) / f64::from(SUBPIXELS);
                let distance = (px - cx).hypot(py - cy);
                if distance <= radius {
                    parts.push((distance, part_flux));
                }
            }
        }
    }
    let total: f64 = parts.iter().map(|part| part.1).sum();
    if total <= 0.0 {
        return None;
    }
    parts.sort_unstable_by(|a, b| a.0.total_cmp(&b.0));
    let (mut previous_distance, mut enclosed) = (0.0, 0.0);
    let mut i = 0;
    while i < parts.len() {
        let distance = parts[i].0;
        let mut group_flux = parts[i].1;
        i += 1;
        // Nearby consecutive distances are one radius, including ties split
        // by floating-point rounding. Include the whole radius before crossing.
        while i < parts.len() && parts[i].0 - parts[i - 1].0 <= 1e-9 {
            group_flux += parts[i].1;
            i += 1;
        }
        let next = enclosed + group_flux;
        if next >= total / 2.0 {
            let fraction = (total / 2.0 - enclosed) / (next - enclosed);
            return Some(2.0 * (previous_distance + fraction * (distance - previous_distance)));
        }
        previous_distance = distance;
        enclosed = next;
    }
    None
}
