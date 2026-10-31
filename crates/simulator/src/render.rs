//! Brightness-ordered catalog stars, Gaussian PSFs, sky, shot and read noise.
use crate::{noise::SplitMix64, Optics};
use math_core::{attitude::RotationMatrix, UnitVector};
use pattern_database::PatternDatabase;

/// At 200 ms and unit gain, magnitude 6.75 gives about 30 stars per 12-degree
/// field on average (the actual count follows the database's sky density).
pub const REFERENCE_EXPOSURE_MS: f64 = 200.0;
pub const REFERENCE_MAGNITUDE: f64 = 6.75;
pub const PSF_SIGMA_PX: f64 = 1.2;

/// A catalog star projected into full-frame (x, y) pixels, before pixel noise.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderStar {
    pub x: f64,
    pub y: f64,
    pub magnitude: f64,
}

pub fn magnitude_limit(exposure_ms: f64, gain: f64) -> f64 {
    assert!(exposure_ms.is_finite() && exposure_ms > 0.0 && gain.is_finite() && gain > 0.0);
    REFERENCE_MAGNITUDE + 2.5 * (exposure_ms * gain / REFERENCE_EXPOSURE_MS).log10()
}

/// Iterate the star table in its existing brightness order; never resample stars
/// or use patterns as a rendering catalog. Stored vectors are f32, maths is f64.
pub fn field_stars(
    db: &PatternDatabase,
    optics: Optics,
    rotation: RotationMatrix,
    exposure_ms: f64,
    gain: f64,
) -> Vec<(usize, RenderStar)> {
    let limit = magnitude_limit(exposure_ms, gain);
    let camera = optics.pinhole();
    let mut stars = Vec::new();
    for (index, row) in db.star_table.as_chunks::<6>().0.iter().enumerate() {
        let magnitude = f64::from(row[5]);
        if magnitude > limit {
            break;
        }
        let v = rotation.rotate(UnitVector {
            x: row[2].into(),
            y: row[3].into(),
            z: row[4].into(),
        });
        let (pixels, inside) = camera.project(&[v]);
        if !inside.is_empty() {
            let (y, x) = pixels[0];
            stars.push((index, RenderStar { x, y, magnitude }));
        }
    }
    stars
}

/// Render one exposure. Inputs fully specify the pixels; no clock or global RNG
/// is consulted here. Sampling at index + 0.5 preserves the detector convention.
pub fn render(
    optics: &Optics,
    stars: &[RenderStar],
    exposure_ms: f64,
    gain: f64,
    seed: u64,
    frame_id: u64,
) -> Vec<u8> {
    let strength = exposure_ms * gain / REFERENCE_EXPOSURE_MS;
    magnitude_limit(exposure_ms, gain);
    let width = optics.width as usize;
    let height = optics.height as usize;
    let mut light = vec![8.0 * strength; width * height];
    for star in stars {
        let peak = 40.0 * strength * 10_f64.powf(-0.4 * (star.magnitude - REFERENCE_MAGNITUDE));
        // Six sigma retains the wings even around saturated bright stars.
        let radius = 8;
        for y in (star.y.floor() as i64 - radius).max(0)
            ..=(star.y.floor() as i64 + radius).min(height as i64 - 1)
        {
            for x in (star.x.floor() as i64 - radius).max(0)
                ..=(star.x.floor() as i64 + radius).min(width as i64 - 1)
            {
                let r2 = (x as f64 + 0.5 - star.x).powi(2) + (y as f64 + 0.5 - star.y).powi(2);
                light[y as usize * width + x as usize] +=
                    peak * (-r2 / (2.0 * PSF_SIGMA_PX.powi(2))).exp();
            }
        }
    }
    let mut random = SplitMix64::new(seed, frame_id);
    // Sixteen electrons per output level keeps faint-star centroids useful while
    // retaining photon statistics; 0.75 levels RMS models independent read noise.
    light
        .into_iter()
        .map(|mean| {
            (random.poisson(mean * 16.0) / 16.0 + 0.75 * random.normal())
                .round()
                .clamp(0.0, 255.0) as u8
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeded_exposures_repeat_but_frame_numbers_change_noise() {
        // A fixture star at boresight, not a value produced by the renderer.
        let optics = Optics {
            width: 64,
            height: 48,
            ..Optics::default()
        };
        let stars = [RenderStar {
            x: 30.25,
            y: 24.75,
            magnitude: 5.0,
        }];
        let draw = |frame| render(&optics, &stars, 200.0, 1.0, 13, frame);
        let a = draw(7);
        assert_eq!(a, draw(7));
        assert_ne!(a, draw(8));
        assert_ne!(a, render(&optics, &stars, 200.0, 1.0, 14, 7));
        assert!(a.iter().any(|&p| p > 100));
    }

    #[test]
    fn exposure_and_gain_follow_the_magnitude_scale() {
        // Pogson's law: ten times the exposure or gain reaches 2.5 mag deeper.
        assert_eq!(magnitude_limit(200.0, 1.0), 6.75);
        assert_eq!(magnitude_limit(2000.0, 1.0), 9.25);
        assert_eq!(magnitude_limit(200.0, 10.0), 9.25);
        assert_eq!(magnitude_limit(20.0, 1.0), 4.25);
    }

    #[test]
    fn gaussian_profile_matches_the_specified_sigma_and_flux() {
        let optics = Optics {
            width: 32,
            height: 32,
            ..Optics::default()
        };
        let stars = [RenderStar {
            x: 16.5,
            y: 16.5,
            magnitude: 6.75,
        }];
        let mut centre = 0.0;
        let mut wing = 0.0;
        for frame in 0..256 {
            let pixels = render(&optics, &stars, 200.0, 1.0, 13, frame);
            centre += f64::from(pixels[16 * 32 + 16]);
            wing += f64::from(pixels[16 * 32 + 18]);
        }
        // Background 8 plus a 40-level peak; at radius 2 px a sigma-1.2
        // Gaussian has mean 8 + 40 exp(-4/2.88) = 17.9741 levels.
        assert!((centre / 256.0 - 48.0).abs() < 0.7);
        assert!((wing / 256.0 - 17.9741).abs() < 0.7);
    }
}
