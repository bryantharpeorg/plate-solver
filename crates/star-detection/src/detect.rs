//! Star detection pipeline entry point.
//!
//! Implements the full cedar-detect algorithm on top of the crate's
//! noise-estimation, binning, centroiding, and star-type helpers.

use crate::binning::{build_cascade, BinnedImage};
use crate::centroid::{measure_star, measure_star_scaled};
use crate::noise::{estimate_noise, NOISE_FLOOR};
use crate::star::Star;

/// Detect stars in an 8-bit grayscale image.
///
/// `image` is row-major with `width` columns. `sigma` is the detection
/// significance threshold, `binning` is one of 1/2/4/8, `normalize_rows`
/// optionally shifts each row's dark level before binning (ignored when
/// `binning == 1`, as in cedar-detect), and `detect_hot_pixels` enables
/// hot-pixel rejection against the full-resolution image.
///
/// Returned stars are sorted by background-subtracted brightness descending;
/// equal brightnesses keep detection order.
pub fn detect_stars(
    image: &[u8],
    width: usize,
    height: usize,
    sigma: f64,
    binning: usize,
    normalize_rows: bool,
    detect_hot_pixels: bool,
) -> Vec<Star> {
    detect_stars_with_noise_floor(
        image,
        width,
        height,
        sigma,
        binning,
        normalize_rows,
        detect_hot_pixels,
        NOISE_FLOOR,
    )
}

/// [`detect_stars`], with the detection image's noise estimate raised to at
/// least `noise_floor` 8-bit levels. At or below cedar-detect's own floor,
/// [`NOISE_FLOOR`], it finds exactly what [`detect_stars`] finds.
#[allow(clippy::too_many_arguments)]
pub fn detect_stars_with_noise_floor(
    image: &[u8],
    width: usize,
    height: usize,
    sigma: f64,
    binning: usize,
    normalize_rows: bool,
    detect_hot_pixels: bool,
    noise_floor: f64,
) -> Vec<Star> {
    assert!(
        width * height == image.len(),
        "image length must equal width * height"
    );
    assert!(
        [1, 2, 4, 8].contains(&binning),
        "binning must be 1, 2, 4, or 8"
    );

    if width < 7 || height < 7 {
        return Vec::new();
    }

    let (detection, higher_res) =
        build_cascade(image, width, height, binning, normalize_rows && binning > 1);

    // Noise is estimated on the detection (most-binned) image.
    let noise = estimate_noise(&detection.data, detection.width, detection.height).max(noise_floor);
    let sigma_noise_2 = ((2.0 * sigma * noise + 0.5) as i16).max(2);
    let sigma_noise_3 = ((3.0 * sigma * noise + 0.5) as i16).max(3);

    // 1-D row scan produces candidates on the detection image.
    let candidates = scan_rows(&detection, sigma_noise_2, sigma_noise_3);

    // Optional hot-pixel rejection against the full-resolution input image,
    // un-normalized, at every binning level.
    let mut filtered = candidates;
    if detect_hot_pixels {
        filtered.retain(|c| {
            !all_bright_are_hot(image, width, height, c.x, c.y, binning, sigma_noise_2)
        });
    }

    // Form blobs from vertically adjacent candidates.
    let blobs = form_blobs(filtered, detection.height);

    // 2-D gate and centroid each blob.
    let mut stars = Vec::new();
    for blob in blobs {
        if let Some(star) = gate_and_measure(
            &blob,
            &detection,
            higher_res.as_ref(),
            binning,
            noise,
            sigma,
        ) {
            stars.push(star);
        }
    }

    // Stable sort on brightness alone, as cedar-detect does: ties keep blob order.
    stars.sort_by(|a, b| {
        b.brightness
            .partial_cmp(&a.brightness)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    stars
}

/// A 1-D candidate pixel in detection-image coordinates.
#[derive(Clone, Copy, Debug)]
struct Candidate {
    x: usize,
    y: usize,
}

/// Scan every row of the detection image, applying the 7-pixel 1-D gate.
fn scan_rows(detection: &BinnedImage, sigma_noise_2: i16, sigma_noise_3: i16) -> Vec<Candidate> {
    let mut candidates = Vec::new();
    let width = detection.width;
    let height = detection.height;
    if width < 7 {
        return candidates;
    }

    for y in 0..height {
        let row_start = y * width;
        let row = &detection.data[row_start..row_start + width];

        // Cheap row minimum sampled every 64th pixel.
        let mut row_min = 255u8;
        for i in (0..width).step_by(64) {
            row_min = row_min.min(row[i]);
        }
        // cedar-detect casts before halving; the two only differ above 255.
        let threshold = row_min.saturating_add(sigma_noise_2 as u8 / 2);

        for x in 3..width - 3 {
            let c = row[x];
            if c < threshold {
                continue;
            }
            let gate = &row[x - 3..x + 4];
            if gate_star_1d(gate, sigma_noise_2, sigma_noise_3) {
                candidates.push(Candidate { x, y });
            }
        }
    }

    candidates
}

/// Apply the 7-pixel 1-D gate to a single candidate pixel.
///
/// Gate layout: | lb lm l C r rm rb |
fn gate_star_1d(gate: &[u8], sigma_noise_2: i16, sigma_noise_3: i16) -> bool {
    let lb = gate[0] as i16;
    let lm = gate[1] as i16;
    let l = gate[2] as i16;
    let c = gate[3] as i16;
    let r = gate[4] as i16;
    let rm = gate[5] as i16;
    let rb = gate[6] as i16;

    // Significance: 2*C - (lb+rb) >= sigma_noise_2.
    if 2 * c - (lb + rb) < sigma_noise_2 {
        return false;
    }
    // Center must be a local peak over immediate neighbors.
    if l > c || c < r {
        return false;
    }
    // Center must be strictly higher than margins.
    if lm >= c || c <= rm {
        return false;
    }
    // Flat-top tie-breaks.
    if l == c && lm > r {
        return false;
    }
    if c == r && l <= rm {
        return false;
    }
    // Uniform background borders.
    if (lb - rb).abs() > sigma_noise_3 {
        return false;
    }
    true
}

/// A blob is a set of vertically adjacent 1-D candidates.
#[derive(Debug, Default)]
struct Blob {
    candidates: Vec<Candidate>,
    /// If `Some`, this blob has been merged into another and is empty.
    recipient: Option<usize>,
}

/// Merge candidates into blobs using union-find-style forwarding.
fn form_blobs(candidates: Vec<Candidate>, height: usize) -> Vec<Blob> {
    let mut blobs: Vec<Blob> = Vec::with_capacity(candidates.len());
    let mut by_row: Vec<Vec<(Candidate, usize)>> = vec![Vec::new(); height];

    for (id, c) in candidates.into_iter().enumerate() {
        blobs.push(Blob {
            candidates: vec![c],
            recipient: None,
        });
        by_row[c.y].push((c, id));
    }

    for y in 1..height {
        for &(c, recipient_id) in &by_row[y] {
            for &(prev_c, donor_id) in &by_row[y - 1] {
                if prev_c.x + 3 < c.x {
                    continue;
                }
                if prev_c.x > c.x + 3 {
                    break;
                }
                merge_blobs(&mut blobs, donor_id, recipient_id);
            }
        }
    }

    blobs
        .into_iter()
        .filter(|b| !b.candidates.is_empty())
        .collect()
}

/// Drain `donor` into `recipient`, following forwarding links.
fn merge_blobs(blobs: &mut [Blob], mut donor_id: usize, recipient_id: usize) {
    if donor_id == recipient_id {
        return;
    }
    loop {
        if !blobs[donor_id].candidates.is_empty() {
            let mut donated: Vec<Candidate> = std::mem::take(&mut blobs[donor_id].candidates);
            blobs[donor_id].recipient = Some(recipient_id);
            blobs[recipient_id].candidates.append(&mut donated);
            return;
        }
        let next = blobs[donor_id]
            .recipient
            .expect("empty blob must have recipient");
        if next == recipient_id {
            return;
        }
        donor_id = next;
    }
}

/// Apply the 2-D gate to a blob and, if it passes, measure its centroid.
fn gate_and_measure(
    blob: &Blob,
    detection: &BinnedImage,
    higher_res: Option<&BinnedImage>,
    binning: usize,
    noise: f64,
    sigma: f64,
) -> Option<Star> {
    let (image, higher) = if binning == 1 {
        (&detection.data, &detection.data)
    } else {
        let h = higher_res.expect("binning > 1 requires higher-res image");
        (&detection.data, &h.data)
    };
    let width = detection.width;
    let height = detection.height;

    // Blob bounding box in detection coordinates.
    let mut x_min = usize::MAX;
    let mut x_max = 0usize;
    let mut y_min = usize::MAX;
    let mut y_max = 0usize;
    for c in &blob.candidates {
        x_min = x_min.min(c.x);
        x_max = x_max.max(c.x);
        y_min = y_min.min(c.y);
        y_max = y_max.max(c.y);
    }
    let core_width = x_max - x_min + 1;
    let core_height = y_max - y_min + 1;

    let max_size = detection.max_size;
    if core_width > max_size || core_height > max_size {
        return None;
    }
    if x_min < 3 || x_max + 3 >= width || y_min < 3 || y_max + 3 >= height {
        return None;
    }

    let core_left = x_min;
    let core_top = y_min;
    let neighbors_left = x_min - 1;
    let neighbors_top = y_min - 1;
    let neighbors_width = core_width + 2;
    let neighbors_height = core_height + 2;
    let margin_left = x_min - 2;
    let margin_top = y_min - 2;
    let margin_width = core_width + 4;
    let margin_height = core_height + 4;
    let perimeter_left = x_min - 3;
    let perimeter_top = y_min - 3;
    let perimeter_width = core_width + 6;
    let perimeter_height = core_height + 6;

    let core_mean = mean_in_rect(image, width, core_left, core_top, core_width, core_height);

    // Inner-core brightness check for cores >= 3x3.
    if core_width >= 3 && core_height >= 3 {
        let outer_mean = perimeter_mean(image, width, core_left, core_top, core_width, core_height);
        if core_mean < outer_mean {
            return None;
        }
    }

    // Neighbor mean (corners excluded).
    let neighbor_mean = perimeter_mean_excluding_corners(
        image,
        width,
        neighbors_left,
        neighbors_top,
        neighbors_width,
        neighbors_height,
    );
    if core_mean < neighbor_mean {
        return None;
    }

    // Margin mean.
    let margin_mean = perimeter_mean(
        image,
        width,
        margin_left,
        margin_top,
        margin_width,
        margin_height,
    );
    if core_mean <= margin_mean {
        return None;
    }

    // Perimeter statistics.
    let (perimeter_mean_val, perimeter_min, perimeter_max, perimeter_stddev) = perimeter_stats(
        image,
        width,
        perimeter_left,
        perimeter_top,
        perimeter_width,
        perimeter_height,
    );

    if (perimeter_max as f64 - perimeter_min as f64) > 3.0 * sigma * noise {
        return None;
    }

    let max_noise = noise.max(perimeter_stddev);
    if core_mean - perimeter_mean_val < sigma * max_noise {
        return None;
    }

    // Centroid and brightness.
    if binning == 1 {
        measure_star(
            image,
            width,
            margin_left,
            margin_top,
            margin_width,
            margin_height,
        )
    } else {
        // Centroid on the one-less-binned image. As in cedar-detect, the peak
        // position is scaled by binning/2 before the 0.5 pixel-center offset
        // is added.
        let h_width = higher_res.unwrap().width;
        let left = margin_left * 2;
        let top = margin_top * 2;
        let box_width = (left + margin_width * 2).min(h_width) - left;
        let box_height = (top + margin_height * 2).min(higher_res.unwrap().height) - top;
        let scale = (binning / 2) as f64;
        measure_star_scaled(higher, h_width, left, top, box_width, box_height, scale)
    }
}

/// Mean of all pixels in a rectangle.
fn mean_in_rect(
    image: &[u8],
    width: usize,
    left: usize,
    top: usize,
    rect_width: usize,
    rect_height: usize,
) -> f64 {
    let mut sum = 0u64;
    for y in top..top + rect_height {
        let row_start = y * width;
        for x in left..left + rect_width {
            sum += image[row_start + x] as u64;
        }
    }
    sum as f64 / (rect_width * rect_height) as f64
}

/// Mean of the 1-pixel perimeter of a rectangle.
fn perimeter_mean(
    image: &[u8],
    width: usize,
    left: usize,
    top: usize,
    rect_width: usize,
    rect_height: usize,
) -> f64 {
    let mut sum = 0u64;
    let mut count = 0usize;
    let right = left + rect_width - 1;
    let bottom = top + rect_height - 1;

    for x in left..=right {
        sum += image[top * width + x] as u64;
        sum += image[bottom * width + x] as u64;
        count += 2;
    }
    for y in (top + 1)..bottom {
        sum += image[y * width + left] as u64;
        sum += image[y * width + right] as u64;
        count += 2;
    }

    sum as f64 / count as f64
}

/// Perimeter mean excluding the four corner pixels.
fn perimeter_mean_excluding_corners(
    image: &[u8],
    width: usize,
    left: usize,
    top: usize,
    rect_width: usize,
    rect_height: usize,
) -> f64 {
    let mut sum = 0u64;
    let mut count = 0usize;
    let right = left + rect_width - 1;
    let bottom = top + rect_height - 1;

    // Top and bottom edges, corners skipped.
    for x in (left + 1)..right {
        sum += image[top * width + x] as u64;
        sum += image[bottom * width + x] as u64;
        count += 2;
    }
    for y in (top + 1)..bottom {
        sum += image[y * width + left] as u64;
        sum += image[y * width + right] as u64;
        count += 2;
    }

    sum as f64 / count as f64
}

/// (mean, min, max, stddev) of a rectangle's 1-pixel perimeter.
fn perimeter_stats(
    image: &[u8],
    width: usize,
    left: usize,
    top: usize,
    rect_width: usize,
    rect_height: usize,
) -> (f64, u8, u8, f64) {
    let mut sum = 0u64;
    let mut count = 0usize;
    let mut min = 255u8;
    let mut max = 0u8;
    let right = left + rect_width - 1;
    let bottom = top + rect_height - 1;

    for x in left..=right {
        let top_val = image[top * width + x];
        let bot_val = image[bottom * width + x];
        sum += top_val as u64;
        sum += bot_val as u64;
        count += 2;
        min = min.min(top_val).min(bot_val);
        max = max.max(top_val).max(bot_val);
    }
    for y in (top + 1)..bottom {
        let left_val = image[y * width + left];
        let right_val = image[y * width + right];
        sum += left_val as u64;
        sum += right_val as u64;
        count += 2;
        min = min.min(left_val).min(right_val);
        max = max.max(left_val).max(right_val);
    }

    let mean = sum as f64 / count as f64;
    // Accumulate in cedar-detect's perimeter order (top row, side pixels row
    // by row, bottom row) so the float sum rounds the same way.
    let mut dev2 = 0.0f64;
    let mut add = |value: u8| {
        let d = value as f64 - mean;
        dev2 += d * d;
    };
    for x in left..=right {
        add(image[top * width + x]);
    }
    for y in (top + 1)..bottom {
        add(image[y * width + left]);
        add(image[y * width + right]);
    }
    for x in left..=right {
        add(image[bottom * width + x]);
    }
    let stddev = (dev2 / count as f64).sqrt();

    (mean, min, max, stddev)
}

/// Hot-pixel check for a candidate at `(x, y)` in detection coordinates.
///
/// Returns true when none of the `binning × binning` full-resolution pixels
/// behind the candidate is a neighbor-supported `Bright` pixel.
fn all_bright_are_hot(
    image: &[u8],
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    binning: usize,
    sigma_noise_2: i16,
) -> bool {
    for full_y in y * binning..(y + 1) * binning {
        for full_x in x * binning..(x + 1) * binning {
            if full_y >= height || full_x < 3 || full_x + 3 >= width {
                continue;
            }
            let row_start = full_y * width;
            let gate = &image[row_start + full_x - 3..row_start + full_x + 4];
            if classify_pixel(gate, sigma_noise_2) == PixelType::Bright {
                return false;
            }
        }
    }
    true
}

#[derive(Debug, Eq, PartialEq)]
enum PixelType {
    Dark,
    Bright,
    Hot,
}

fn classify_pixel(gate: &[u8], sigma_noise_2: i16) -> PixelType {
    let lb = gate[0] as i16;
    let c = gate[3] as i16;
    let rb = gate[6] as i16;
    let l = gate[2] as i16;
    let r = gate[4] as i16;

    let est_background_2 = lb + rb;
    let center_minus_background_2 = 2 * c - est_background_2;
    if center_minus_background_2 < sigma_noise_2 {
        return PixelType::Dark;
    }

    let neighbor_sum_minus_background = (l + r) - est_background_2;
    if 4 * neighbor_sum_minus_background <= center_minus_background_2 / 2 {
        return PixelType::Hot;
    }
    PixelType::Bright
}
