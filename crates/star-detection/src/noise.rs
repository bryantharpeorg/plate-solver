//! Robust RMS noise estimation from an 8-bit grayscale image.
//!
//! The estimator takes three 1-pixel-tall horizontal cuts across the image,
//! removes bright star-like outliers from each cut, and returns the standard
//! deviation of the darkest remaining cut (by mean). A floor of `0.2` is
//! applied so the detection threshold never collapses to zero on black-clipped
//! backgrounds.

/// Minimum returned noise level, in ADU: cedar-detect's own floor.
pub const NOISE_FLOOR: f64 = 0.2;

/// Sigma used for the star-removal cutoff when de-starring each cut's
/// histogram. This matches the upstream cedar-detect implementation.
const DE_STAR_SIGMA: f64 = 8.0;

/// Estimate RMS noise from a grayscale image.
///
/// `image` is an 8-bit grayscale buffer laid out row-major with `width` pixels
/// per row. The estimator:
///
/// 1. Takes three 1-pixel-tall horizontal cuts of width `min(50, width/4)`
///    centered vertically on the image and horizontally at approximately
///    `width/4`, `width/2`, and `3*width/4`.
/// 2. For each cut, builds a histogram, trims the brightest 10% of samples,
///    and then removes any remaining bins whose value exceeds
///    `mean + 8 * max(stddev, 1.0)` (the upstream de-star step).
/// 3. Computes the mean and population standard deviation of the remaining
///    histogram.
/// 4. Returns the standard deviation of the cut with the lowest mean,
///    clamped to at least `0.2`.
///
/// # Panics
///
/// Panics if `image.len()` is not exactly `width * height`.
pub fn estimate_noise(image: &[u8], width: usize, height: usize) -> f64 {
    assert_eq!(
        image.len(),
        width * height,
        "image length must equal width * height"
    );

    if width == 0 || height == 0 {
        return NOISE_FLOOR;
    }

    let cut_width = (width / 4).clamp(1, 50);
    let cut_half = cut_width / 2;
    let mid_y = height / 2;

    let centers_x = [width / 4, width / 2, 3 * width / 4];

    let mut best_stddev = 0.0f64;
    let mut best_mean = f64::INFINITY;

    for &cx in &centers_x {
        let start_x = cx
            .saturating_sub(cut_half)
            .min(width.saturating_sub(cut_width));
        let y = mid_y.min(height - 1);
        let row_offset = y * width;

        let pixels = &image[row_offset + start_x..row_offset + start_x + cut_width];

        // Build histogram of the cut.
        let mut histogram = [0u32; 256];
        for &v in pixels {
            histogram[v as usize] += 1;
        }

        // De-star: first trim the brightest 10% to get a coarse background
        // estimate, then remove any bins above `mean + 8 * max(stddev, 1.0)`.
        let pixel_count: u32 = pixels.len() as u32;
        let mut trimmed = histogram;
        trim_histogram(&mut trimmed, pixel_count * 9 / 10);
        let (mean, stddev) = stats_for_histogram(&trimmed);
        let star_cutoff = (mean + DE_STAR_SIGMA * stddev.max(1.0)) as usize;
        for (h, count) in histogram.iter_mut().enumerate() {
            if h >= star_cutoff {
                *count = 0;
            }
        }

        let (mean, stddev) = stats_for_histogram(&histogram);

        if mean < best_mean {
            best_mean = mean;
            best_stddev = stddev;
        }
    }

    best_stddev.max(NOISE_FLOOR)
}

/// Trim a histogram so that at most `count_to_keep` samples remain,
/// discarding the brightest samples first.
fn trim_histogram(histogram: &mut [u32; 256], count_to_keep: u32) {
    let mut count = 0u32;
    for bin in histogram.iter_mut() {
        let bin_count = *bin;
        if count + bin_count > count_to_keep {
            let excess = count + bin_count - count_to_keep;
            *bin -= excess;
        }
        count += *bin;
    }
}

/// Compute the population mean and standard deviation of a histogram.
fn stats_for_histogram(histogram: &[u32; 256]) -> (f64, f64) {
    let mut count = 0u64;
    let mut first_moment = 0u64;
    for (h, &bin_count) in histogram.iter().enumerate() {
        count += bin_count as u64;
        first_moment += bin_count as u64 * h as u64;
    }
    if count == 0 {
        return (0.0, 0.0);
    }
    let mean = first_moment as f64 / count as f64;
    let mut second_moment = 0.0f64;
    for (h, &bin_count) in histogram.iter().enumerate() {
        let d = h as f64 - mean;
        second_moment += bin_count as f64 * d * d;
    }
    let variance = second_moment / count as f64;
    (mean, variance.sqrt())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn darkest_cut_is_chosen() {
        // 100x10 image. Cut 1 and 3 are dark background (mean ~10). Cut 2 is
        // bright (mean ~100). The estimator must pick a dark cut.
        let width = 100;
        let height = 10;
        let mut image = vec![10u8; width * height];

        // Bright interloper over the middle cut: x in [45, 55], all rows.
        for y in 0..height {
            for x in 45..=55 {
                image[y * width + x] = 200;
            }
        }

        let noise = estimate_noise(&image, width, height);
        // Dark cuts have stddev ~0; floor applies.
        assert!(
            (noise - 0.2).abs() < 1e-9,
            "expected floor 0.2, got {}",
            noise
        );
    }

    #[test]
    fn noise_floor_is_applied() {
        // Completely black image: measured stddev is 0, so floor must apply.
        let image = vec![0u8; 80 * 20];
        let noise = estimate_noise(&image, 80, 20);
        assert!((noise - 0.2).abs() < 1e-9, "expected 0.2, got {}", noise);
    }

    #[test]
    fn noisy_background_returns_above_floor() {
        // 200x20 image with Gaussian-ish noise around 50 ADU.
        let width = 200;
        let height = 20;
        let mut image = vec![50u8; width * height];
        // Add small perturbations to every pixel.
        for (i, p) in image.iter_mut().enumerate() {
            let delta = ((i % 7) as i32) - 3;
            *p = (*p as i32 + delta).clamp(0, 255) as u8;
        }

        let noise = estimate_noise(&image, width, height);
        assert!(noise > 0.2, "expected noise above floor, got {}", noise);
    }
}
