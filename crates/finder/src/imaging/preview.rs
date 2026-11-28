//! The colour preview of a saved imaging frame (031 US6): superpixel colour
//! for a mosaic and gray for mono, box-averaged by a whole factor until the
//! longer side is at most 1280 pixels, stretched over each channel's noise,
//! and encoded as a JPEG of quality 85. A preview frame also gets a crop around
//! its brightest box, one pixel a cell (058).

use image::{codecs::jpeg::JpegEncoder, ExtendedColorType, ImageEncoder};

/// The preview's longest side, in pixels (FR-017).
pub const PREVIEW_SIDE: u32 = 1280;
/// The JPEG's quality (FR-017).
pub const PREVIEW_QUALITY: u8 = 85;
/// How many saved frames' previews the console keeps (FR-018).
pub const PREVIEWS_KEPT: usize = 8;
/// The stretch's asinh factor (FR-017).
pub const STRETCH: f64 = 10.0;
/// The least span the stretch reads, in units of a channel's noise (FR-017).
pub const NOISE_FLOOR: f64 = 30.0;
/// The crop's side, in cells: one picture pixel a cell (058 FR-006).
pub const CROP_SIDE: u32 = 200;
/// The side, in samples, of the boxes the crop's centre is found among (058 FR-006).
const BOX_SIDE: u32 = 32;

/// One colour picture: RGB, 3 bytes a pixel, row by row from the top-left.
#[derive(Debug, Clone, PartialEq)]
pub struct Rgb {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

/// The sensor's mosaic, as `bayer` names it.
enum Mosaic {
    Rggb,
    Bggr,
    Grbg,
    Gbrg,
}

/// The mosaic a Bayer pattern names, or none for a mono frame: `rgb` reads
/// any other pattern as mono (FR-017).
fn mosaic(bayer: Option<&str>) -> Option<Mosaic> {
    match bayer {
        Some("RGGB") => Some(Mosaic::Rggb),
        Some("BGGR") => Some(Mosaic::Bggr),
        Some("GRBG") => Some(Mosaic::Grbg),
        Some("GBRG") => Some(Mosaic::Gbrg),
        _ => None,
    }
}

/// The picture of one frame: each whole 2 × 2 cell of a mosaic becomes one
/// pixel, red and blue their samples and green the mean of its two, a mono
/// frame one pixel a sample in all three channels; then each k × k block
/// becomes its mean with k = ⌈longer side / 1280⌉, and each channel is
/// stretched from its median over the largest of its 99.9th percentile's
/// distance and thirty times its noise (FR-017).
pub fn rgb(samples: &[u8], width: u32, height: u32, bits: u32, bayer: Option<&str>) -> Rgb {
    let bytes = if bits == 8 { 1 } else { 2 };
    let read = |x: u32, y: u32| -> f64 {
        let index = (y as usize * width as usize + x as usize) * bytes;
        if bytes == 1 {
            f64::from(samples[index])
        } else {
            f64::from(u16::from_le_bytes([samples[index], samples[index + 1]]))
        }
    };
    // The colour and size steps together: the mean of a k × k block of
    // superpixels is the mean of the samples it covers, so the block is read
    // straight from the frame and no whole-frame copy is built. Trailing rows
    // and columns that do not fill a block are dropped.
    let mut channels: [Vec<f64>; 3];
    let picture: (u32, u32);
    match mosaic(bayer) {
        None => {
            let k = width.max(height).div_ceil(PREVIEW_SIDE).max(1);
            let (w, h) = (width / k, height / k);
            let mut values = Vec::with_capacity((w * h) as usize);
            for y in 0..h {
                for x in 0..w {
                    let mut sum = 0.0;
                    for block_y in 0..k {
                        for block_x in 0..k {
                            sum += read(k * x + block_x, k * y + block_y);
                        }
                    }
                    values.push(sum / f64::from(k * k));
                }
            }
            channels = [values.clone(), values.clone(), values];
            picture = (w, h);
        }
        Some(pattern) => {
            let (super_w, super_h) = (width / 2, height / 2);
            let k = super_w.max(super_h).div_ceil(PREVIEW_SIDE).max(1);
            let (w, h) = (super_w / k, super_h / k);
            channels = std::array::from_fn(|_| Vec::with_capacity((w * h) as usize));
            for y in 0..h {
                for x in 0..w {
                    let mut sums = [0.0f64; 3];
                    for block_y in 0..k {
                        for block_x in 0..k {
                            let (sx, sy) = (k * x + block_x, k * y + block_y);
                            let (s00, s01, s10, s11) = (
                                read(2 * sx, 2 * sy),
                                read(2 * sx + 1, 2 * sy),
                                read(2 * sx, 2 * sy + 1),
                                read(2 * sx + 1, 2 * sy + 1),
                            );
                            let (red, green, blue) = match pattern {
                                Mosaic::Rggb => (s00, (s01 + s10) / 2.0, s11),
                                Mosaic::Bggr => (s11, (s01 + s10) / 2.0, s00),
                                Mosaic::Grbg => (s01, (s00 + s11) / 2.0, s10),
                                Mosaic::Gbrg => (s10, (s00 + s11) / 2.0, s01),
                            };
                            sums[0] += red;
                            sums[1] += green;
                            sums[2] += blue;
                        }
                    }
                    let cells = f64::from(k * k);
                    for (channel, sum) in channels.iter_mut().zip(sums) {
                        channel.push(sum / cells);
                    }
                }
            }
            picture = (w, h);
        }
    }
    // The stretch: each channel's black is its median and its noise that less
    // its 16th percentile; one span, the largest over the three channels of
    // the 99.9th percentile's distance and thirty times the noise, floors at
    // 1 (FR-017).
    let sorted: [Vec<f64>; 3] = std::array::from_fn(|channel| {
        let mut values = channels[channel].clone();
        values.sort_by(f64::total_cmp);
        values
    });
    let percentile = |channel: usize, part: f64| -> f64 {
        let values = &sorted[channel];
        if values.is_empty() {
            return 0.0;
        }
        values[(part * (values.len() - 1) as f64).floor() as usize]
    };
    let black: [f64; 3] = std::array::from_fn(|channel| percentile(channel, 0.5));
    let mut span: f64 = 1.0;
    for (channel, black) in black.iter().enumerate() {
        let noise = black - percentile(channel, 0.16);
        span = span.max(percentile(channel, 0.999) - black);
        span = span.max(NOISE_FLOOR * noise);
    }
    let scale = 255.0 / STRETCH.asinh();
    for (channel, values) in channels.iter_mut().enumerate() {
        for value in values.iter_mut() {
            let t = ((*value - black[channel]) / span).clamp(0.0, 1.0);
            *value = ((STRETCH * t).asinh() * scale).round();
        }
    }
    let (w, h) = picture;
    let mut pixels = Vec::with_capacity((w * h) as usize * 3);
    for index in 0..(w * h) as usize {
        pixels.extend(channels.iter().map(|channel| channel[index] as u8));
    }
    Rgb {
        width: w,
        height: h,
        pixels,
    }
}

/// The picture as a JPEG of quality 85 (FR-017). `None` when the picture has
/// no pixels, which `image`'s encoder refuses.
pub fn jpeg(picture: &Rgb) -> Option<Vec<u8>> {
    if picture.width == 0 || picture.height == 0 {
        return None;
    }
    let mut out = std::io::Cursor::new(Vec::new());
    JpegEncoder::new_with_quality(&mut out, PREVIEW_QUALITY)
        .write_image(
            &picture.pixels,
            picture.width,
            picture.height,
            ExtendedColorType::Rgb8,
        )
        .ok()
        .map(|_| out.into_inner())
}

/// The centre, in samples, of the frame's brightest box (058 FR-006): of the
/// whole `BOX_SIDE` × `BOX_SIDE` boxes cut from the frame's top-left, the one
/// whose samples sum the highest, the first in row order on a tie; the frame's
/// own centre when it holds no whole box. Samples past the last whole box of a
/// row or a column belong to none.
fn brightest(samples: &[u8], width: u32, height: u32, bits: u32) -> (u32, u32) {
    let bytes = if bits == 8 { 1 } else { 2 };
    let (boxes_across, boxes_down) = (width / BOX_SIDE, height / BOX_SIDE);
    let mut sums = vec![0u64; boxes_across as usize];
    let mut best: Option<(u64, u32, u32)> = None;
    for band in 0..boxes_down {
        // One band of rows at a time: each box's samples are summed as they
        // pass, and the sums are reset for the next band.
        sums.fill(0);
        for row in band * BOX_SIDE..(band + 1) * BOX_SIDE {
            for column in 0..boxes_across * BOX_SIDE {
                let index = (row * width + column) as usize * bytes;
                let sample = if bytes == 1 {
                    u64::from(samples[index])
                } else {
                    u64::from(u16::from_le_bytes([samples[index], samples[index + 1]]))
                };
                sums[(column / BOX_SIDE) as usize] += sample;
            }
        }
        for (across, &sum) in sums.iter().enumerate() {
            if best.is_none_or(|(top, _, _)| sum > top) {
                let x = across as u32 * BOX_SIDE + BOX_SIDE / 2;
                let y = band * BOX_SIDE + BOX_SIDE / 2;
                best = Some((sum, x, y));
            }
        }
    }
    best.map_or((width / 2, height / 2), |(_, x, y)| (x, y))
}

/// The crop of a preview frame (058 FR-006): `rgb` of the `CROP_SIDE` ×
/// `CROP_SIDE` cells around the frame's brightest box, one picture pixel a cell
/// and stretched over its own samples, never box-averaged. A mosaic's cell is
/// 2 × 2 samples and a mono frame's one sample; the crop is moved in to lie
/// within the frame's whole cells, and its first column and row fall on a whole
/// cell, so a mosaic keeps its pattern. A frame with no whole cell row gives a
/// picture with no pixels, which `jpeg` gives no JPEG for.
pub fn crop(samples: &[u8], width: u32, height: u32, bits: u32, bayer: Option<&str>) -> Rgb {
    let cell = if mosaic(bayer).is_some() { 2 } else { 1 };
    let bytes = if bits == 8 { 1 } else { 2 };
    let (whole_width, whole_height) = (width / cell * cell, height / cell * cell);
    let w = (CROP_SIDE * cell).min(whole_width);
    let h = (CROP_SIDE * cell).min(whole_height);
    let (x, y) = brightest(samples, width, height, bits);
    let x0 = x.saturating_sub(w / 2).min(whole_width - w) / cell * cell;
    let y0 = y.saturating_sub(h / 2).min(whole_height - h) / cell * cell;
    let row_bytes = width as usize * bytes;
    let span = w as usize * bytes;
    let mut part = Vec::with_capacity(span * h as usize);
    for row in y0..y0 + h {
        let start = row as usize * row_bytes + x0 as usize * bytes;
        part.extend_from_slice(&samples[start..start + span]);
    }
    rgb(&part, w, h, bits, bayer)
}
