//! US2: worked examples from spec 034, exact unless a tolerance is stated.
//! Simulator masters are built here from its declared sensor and light formulas.
use finder::imaging::calibration::{
    calibrate, find_hot_pixels, levels, lit_samples, normalise_flat, percentile, repair, Level,
    Masters, Mosaic, Report, SizeMismatch,
};
use scope_node::imaging::{Exposure, ImagingCamera, ImagingSettings};
use simulator::{
    DustShadow, HotPixel, LightPath, SensorPattern, SimImagingCamera, SimStar, Simulator, Site,
    VirtualClock,
};

use crate::support::live as support;

fn uniform(width: u32, height: u32, value: f32) -> Mosaic {
    Mosaic {
        width,
        height,
        bayer: Some("RGGB".into()),
        samples: vec![value; width as usize * height as usize],
    }
}

fn background(size: u32) -> Mosaic {
    let mut mosaic = uniform(size, size, 200.0);
    for y in 0..size {
        for x in 0..size {
            // RGGB, independently of Mosaic::colour.
            let value = match (x % 2, y % 2) {
                (0, 0) => 100.0,
                (1, 1) => 300.0,
                _ => 200.0,
            };
            put(&mut mosaic, x, y, value);
        }
    }
    mosaic
}

fn at(mosaic: &Mosaic, x: u32, y: u32) -> f32 {
    mosaic.samples[(y * mosaic.width + x) as usize]
}

fn put(mosaic: &mut Mosaic, x: u32, y: u32, value: f32) {
    let index = (y * mosaic.width + x) as usize;
    mosaic.samples[index] = value;
}

fn report(
    dark: &'static str,
    flat: &'static str,
    bad_pixels: &'static str,
    repaired: u64,
) -> Report {
    Report {
        dark,
        flat,
        bad_pixels,
        repaired,
        not_repaired: 0,
        low_flat: 0,
    }
}

#[test]
fn samples_colours_and_floored_percentiles() {
    let bytes = [0xA0, 0x00, 0x88, 0x13, 0xFF, 0xFF, 0x00, 0x00];
    for (pattern, colours) in [
        ("RGGB", [0, 1, 1, 2]),
        ("GRBG", [1, 0, 2, 1]),
        ("BGGR", [2, 1, 1, 0]),
        ("GBRG", [1, 2, 0, 1]),
    ] {
        let mosaic = Mosaic::from_samples(&bytes, 2, 2, 16, Some(pattern)).unwrap();
        assert_eq!(mosaic.samples, [160.0, 5000.0, 65535.0, 0.0]);
        assert_eq!((mosaic.width, mosaic.height), (2, 2));
        assert_eq!(mosaic.bayer.as_deref(), Some(pattern));
        assert_eq!(mosaic.colours(), 3);
        assert_eq!(mosaic.step(), 2);
        for (i, expected) in colours.into_iter().enumerate() {
            assert_eq!(mosaic.colour(i as u32 % 2, i as u32 / 2), expected);
            assert_eq!(mosaic.colour(i as u32 % 2 + 2, i as u32 / 2 + 2), expected);
        }
    }
    let mono = Mosaic::from_samples(&[0x00, 0x7F, 0xFF, 0x10], 2, 2, 8, None).unwrap();
    assert_eq!(mono.samples, [0.0, 127.0, 255.0, 16.0]);
    assert_eq!(mono.bayer, None);
    assert_eq!((mono.colours(), mono.step()), (1, 1));
    for y in 0..2 {
        for x in 0..2 {
            assert_eq!(mono.colour(x, y), 0);
        }
    }
    assert_eq!(
        Mosaic::from_samples(&bytes[..7], 2, 2, 16, Some("RGGB")),
        None
    );
    assert_eq!(Mosaic::from_samples(&bytes, 2, 2, 12, None), None);
    assert_eq!(
        Mosaic::from_samples(&bytes, u32::MAX, u32::MAX, 16, None),
        None
    );
    assert_eq!(percentile(&[5.0, 1.0, 4.0, 2.0, 3.0], 0.5), Some(3.0));
    assert_eq!(percentile(&[1.0, 2.0, 3.0, 4.0], 0.5), Some(2.0));
    assert_eq!(percentile(&[1.0, 2.0, 3.0, 4.0], 0.16), Some(1.0));
    assert_eq!(percentile(&[4.0, 1.0, 3.0, 2.0], 0.0), Some(1.0));
    assert_eq!(percentile(&[4.0, 1.0, 3.0, 2.0], 1.0), Some(4.0));
    assert_eq!(percentile(&[], 0.5), None);
}

#[test]
fn dark_then_flat_and_missing_steps_keep_negative_counts() {
    let dark = [160.0; 64];
    let flat = [0.5; 64];
    let mut sub = uniform(8, 8, 560.0);
    let actual = calibrate(
        &mut sub,
        &Masters {
            dark: Some(&dark),
            flat: Some(&flat),
            bad_pixels: Some(&[]),
        },
    )
    .unwrap();
    assert!(sub.samples.iter().all(|&v| v == 800.0));
    assert_eq!(actual, report("applied", "applied", "map", 0));
    assert_eq!(
        serde_json::to_string(&actual).unwrap(),
        r#"{"dark":"applied","flat":"applied","bad_pixels":"map","repaired":0,"not_repaired":0,"low_flat":0}"#
    );
    for (dark, flat, value, expected, dark_step, flat_step) in [
        (
            Some(dark.as_slice()),
            None,
            560.0,
            400.0,
            "applied",
            "missing",
        ),
        (
            None,
            Some(flat.as_slice()),
            560.0,
            1120.0,
            "missing",
            "applied",
        ),
        (None, None, 560.0, 560.0, "missing", "missing"),
        (
            Some(dark.as_slice()),
            None,
            150.0,
            -10.0,
            "applied",
            "missing",
        ),
    ] {
        let mut sub = uniform(8, 8, value);
        let actual = calibrate(
            &mut sub,
            &Masters {
                dark,
                flat,
                bad_pixels: None,
            },
        )
        .unwrap();
        assert!(sub.samples.iter().all(|&v| v == expected));
        assert_eq!(actual, report(dark_step, flat_step, "sub", 0));
    }
}

#[test]
fn flats_are_normalised_by_colour_before_dividing_a_sub() {
    let mut flat = uniform(4, 4, 4000.0);
    let reds = [(0, 0), (2, 0), (0, 2), (2, 2)];
    let blues = [(1, 1), (3, 1), (1, 3), (3, 3)];
    for ((x, y), value) in reds.into_iter().zip([2000.0, 2200.0, 1800.0, 2000.0]) {
        put(&mut flat, x, y, value);
    }
    for ((x, y), value) in blues.into_iter().zip([1000.0, 1000.0, 500.0, 1500.0]) {
        put(&mut flat, x, y, value);
    }
    assert_eq!(normalise_flat(&mut flat), [2000.0, 4000.0, 1000.0]);
    for ((x, y), value) in reds.into_iter().zip([1.0, 1.1, 0.9, 1.0]) {
        assert_eq!(at(&flat, x, y), value);
    }
    for ((x, y), value) in blues.into_iter().zip([1.0, 1.0, 0.5, 1.5]) {
        assert_eq!(at(&flat, x, y), value);
    }
    let mut sub = uniform(4, 4, 1000.0);
    assert_eq!(
        calibrate(
            &mut sub,
            &Masters {
                flat: Some(&flat.samples),
                bad_pixels: Some(&[]),
                ..Default::default()
            }
        )
        .unwrap(),
        report("missing", "applied", "map", 0)
    );
    // Hand divisions rounded to 0.001 count; allow 0.001 for f32 arithmetic.
    for ((x, y), value) in reds.into_iter().zip([1000.0, 909.091, 1111.111, 1000.0]) {
        assert!((at(&sub, x, y) - value).abs() <= 0.001);
    }
    for ((x, y), value) in blues.into_iter().zip([1000.0, 1000.0, 2000.0, 666.667]) {
        assert!((at(&sub, x, y) - value).abs() <= 0.001);
    }
    for y in 0..4 {
        for x in 0..4 {
            if x % 2 != y % 2 {
                assert_eq!(at(&flat, x, y), 1.0);
                assert_eq!(at(&sub, x, y), 1000.0);
            }
        }
    }
    let mut zero = uniform(2, 2, -1.0);
    assert_eq!(normalise_flat(&mut zero), [-1.0; 3]);
    assert_eq!(zero.samples, [0.0; 4]);
}

#[test]
fn repair_uses_only_good_neighbours_of_the_same_colour() {
    for (x, y, bad_value, expected) in [
        (4, 4, 5000.0, 100.0),
        (0, 0, 7000.0, 100.0),
        (3, 4, 900.0, 200.0),
    ] {
        let mut mosaic = background(8);
        put(&mut mosaic, x, y, bad_value);
        assert_eq!(repair(&mut mosaic, &[(x, y)]), (1, 0));
        assert_eq!(at(&mosaic, x, y), expected);
    }
    let mut mosaic = background(8);
    for ((x, y), value) in [(2, 4), (6, 4), (4, 2), (4, 6)]
        .into_iter()
        .zip([140.0, 100.0, 120.0, 180.0])
    {
        put(&mut mosaic, x, y, value);
    }
    let mut single = mosaic.clone();
    assert_eq!(repair(&mut single, &[(4, 4)]), (1, 0));
    assert_eq!(at(&single, 4, 4), 130.0);
    put(&mut mosaic, 6, 4, 9000.0);
    let mut reversed = mosaic.clone();
    assert_eq!(repair(&mut mosaic, &[(4, 4), (6, 4)]), (2, 0));
    assert_eq!(at(&mosaic, 4, 4), 140.0);
    assert_eq!(at(&mosaic, 6, 4), 100.0);
    assert_eq!(repair(&mut reversed, &[(6, 4), (4, 4)]), (2, 0));
    assert_eq!(reversed, mosaic);
    let mut duplicates = background(8);
    assert_eq!(repair(&mut duplicates, &[(4, 4), (4, 4), (99, 99)]), (1, 0));
    let mut blocked = background(4);
    let original = blocked.clone();
    assert_eq!(
        repair(&mut blocked, &[(0, 0), (2, 0), (0, 2), (2, 2)]),
        (0, 4)
    );
    assert_eq!(blocked, original);
    assert_eq!(
        calibrate(
            &mut blocked,
            &Masters {
                bad_pixels: Some(&[(0, 0), (2, 0), (0, 2), (2, 2)]),
                ..Default::default()
            }
        )
        .unwrap()
        .not_repaired,
        4
    );
    let mut mono = uniform(5, 5, 50.0);
    mono.bayer = None;
    put(&mut mono, 2, 2, 900.0);
    for (x, y) in [(2, 0), (0, 2), (4, 2), (2, 4)] {
        put(&mut mono, x, y, 70.0);
    }
    assert_eq!(repair(&mut mono, &[(2, 2)]), (1, 0));
    assert_eq!(at(&mono, 2, 2), 50.0);
}

fn ratio_examples() -> Mosaic {
    let mut mosaic = background(16);
    put(&mut mosaic, 6, 6, 400.0);
    for (x, y, value) in [(10, 10, 4100.0), (10, 2, 6100.0)] {
        put(&mut mosaic, x, y, value);
        for (nx, ny) in [(x - 2, y), (x + 2, y), (x, y - 2), (x, y + 2)] {
            put(&mut mosaic, nx, ny, 1100.0);
        }
    }
    mosaic
}

#[test]
fn hot_pixels_require_noise_and_ratio_and_are_searched_only_without_a_map() {
    let mosaic = ratio_examples();
    assert_eq!(
        levels(&mosaic),
        [
            Level {
                median: 100.0,
                noise: 1.0
            },
            Level {
                median: 200.0,
                noise: 1.0
            },
            Level {
                median: 300.0,
                noise: 1.0
            },
        ]
    );
    assert_eq!(find_hot_pixels(&mosaic), [(10, 2), (6, 6)]);
    let mut sub = mosaic.clone();
    assert_eq!(
        calibrate(&mut sub, &Masters::default()).unwrap(),
        report("missing", "missing", "sub", 2)
    );
    assert_eq!(at(&sub, 6, 6), 100.0);
    assert_eq!(at(&sub, 10, 2), 1100.0);
    assert_eq!(at(&sub, 10, 10), 4100.0);
    let mut mapped = mosaic.clone();
    assert_eq!(
        calibrate(
            &mut mapped,
            &Masters {
                bad_pixels: Some(&[]),
                ..Default::default()
            }
        )
        .unwrap(),
        report("missing", "missing", "map", 0)
    );
    assert_eq!(mapped, mosaic);
    let mut noisy = background(16);
    for y in [0, 8] {
        for x in (0..16).step_by(2) {
            put(&mut noisy, x, y, 90.0);
        }
    }
    put(&mut noisy, 8, 4, 160.0);
    assert_eq!(
        levels(&noisy)[0],
        Level {
            median: 100.0,
            noise: 10.0
        }
    );
    assert!(find_hot_pixels(&noisy).is_empty());
}

#[test]
fn low_and_nan_flats_are_repaired_instead_of_divided() {
    let mut sub = uniform(8, 8, 1000.0);
    let mut flat = uniform(8, 8, 1.0);
    put(&mut flat, 4, 4, 0.05);
    put(&mut flat, 2, 2, f32::NAN);
    let mut expected = report("missing", "applied", "map", 2);
    expected.low_flat = 2;
    assert_eq!(
        calibrate(
            &mut sub,
            &Masters {
                flat: Some(&flat.samples),
                bad_pixels: Some(&[]),
                ..Default::default()
            }
        )
        .unwrap(),
        expected
    );
    assert!(sub.samples.iter().all(|&v| v == 1000.0));
    // Exactly the floor is divided, and overlap with the map repairs once.
    put(&mut flat, 2, 2, 0.1);
    let mut sub = uniform(8, 8, 1000.0);
    let actual = calibrate(
        &mut sub,
        &Masters {
            flat: Some(&flat.samples),
            bad_pixels: Some(&[(4, 4)]),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!((actual.low_flat, actual.repaired), (1, 1));
    assert_eq!(at(&sub, 2, 2), 10000.0);
}

#[test]
fn master_size_errors_leave_the_sub_untouched_even_after_a_valid_dark() {
    let original = uniform(8, 8, 560.0);
    for (masters, master, have) in [
        (
            Masters {
                dark: Some(&[160.0; 63]),
                ..Default::default()
            },
            "dark",
            63,
        ),
        (
            Masters {
                dark: Some(&[160.0; 64]),
                flat: Some(&[0.5; 65]),
                bad_pixels: None,
            },
            "flat",
            65,
        ),
    ] {
        let mut sub = original.clone();
        let error = calibrate(&mut sub, &masters).unwrap_err();
        assert_eq!(
            error,
            SizeMismatch {
                master,
                have,
                need: 64
            }
        );
        assert_eq!(
            error.to_string(),
            format!("the master {master} holds {have} samples and the sub 64")
        );
        assert_eq!(sub, original);
    }
}

fn sim() -> Simulator {
    let sim = Simulator::new(
        support::DB.clone(),
        Site::default(),
        VirtualClock::new(support::START),
    );
    sim.push_to(50.0, 120.0);
    sim
}

fn hot_pattern() -> SensorPattern {
    SensorPattern {
        hot_pixels: [(100, 50, 4000.0), (101, 51, 2000.0), (600, 400, 40000.0)]
            .into_iter()
            .map(|(x, y, electrons_per_s)| HotPixel {
                x,
                y,
                electrons_per_s,
            })
            .collect(),
        column_amplitude: 0.0,
    }
}

async fn take(camera: &SimImagingCamera, exposure_ms: f64, binning: u32) -> Mosaic {
    camera
        .apply(&ImagingSettings {
            gain: Some(0.0),
            binning: Some(binning),
            ..Default::default()
        })
        .await
        .unwrap();
    let frame = camera
        .expose(&Exposure {
            exposure_ms,
            purpose: "frame".into(),
        })
        .await
        .unwrap();
    assert_eq!(frame.bayer.unwrap().as_str(), "RGGB");
    Mosaic::from_samples(
        &frame.samples,
        frame.width,
        frame.height,
        frame.bits,
        Some("RGGB"),
    )
    .unwrap()
}

// This oracle deliberately sorts directly rather than calling percentile or repair.
fn median(mut values: Vec<f32>) -> f32 {
    values.sort_by(f32::total_cmp);
    let mid = values.len() / 2;
    if values.len().is_multiple_of(2) {
        (values[mid - 1] + values[mid]) / 2.0
    } else {
        values[mid]
    }
}

fn block(mosaic: &Mosaic, x: u32, y: u32, size: u32) -> f32 {
    median(
        (y..y + size)
            .flat_map(|row| (x..x + size).map(move |col| at(mosaic, col, row)))
            .collect(),
    )
}

#[tokio::test(start_paused = true)]
async fn known_simulator_patterns_calibrate_to_fifty_counts_across_the_sensor() {
    let sim = sim();
    let mut pattern = hot_pattern();
    pattern.column_amplitude = 14.0;
    sim.set_sensor_pattern(pattern);
    sim.set_light_path(LightPath {
        corner_light: 0.5,
        dust: vec![DustShadow {
            x: 300.0,
            y: 200.0,
            radius_px: 10.0,
            depth: 0.3,
        }],
        ..Default::default()
    });
    let camera = SimImagingCamera::new(sim)
        .with_stars(vec![])
        .with_noise(true);
    let mut sub = take(&camera, 10000.0, 1).await;
    assert!((block(&sub, 0, 0, 100) - block(&sub, 568, 361, 100)).abs() > 15.0);
    let mut dark = Vec::with_capacity(sub.samples.len());
    let mut flat = Vec::with_capacity(sub.samples.len());
    let (half_w, half_h) = (f64::from(sub.width) / 2.0, f64::from(sub.height) / 2.0);
    for y in 0..sub.height {
        for x in 0..sub.width {
            let (px, py) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
            let v = 1.0
                - 0.5 * ((px - half_w).powi(2) + (py - half_h).powi(2))
                    / (half_w.powi(2) + half_h.powi(2));
            let d = if (px - 300.0).powi(2) + (py - 200.0).powi(2) <= 100.0 {
                0.7
            } else {
                1.0
            };
            flat.push((v * d) as f32);
            let hot = match (x, y) {
                (100, 50) => 10000.0,
                (101, 51) => 5000.0,
                (600, 400) => 100000.0,
                _ => 0.0,
            };
            dark.push(160.0 + 2.0 * (x % 8) as f32 - 7.0 + hot);
        }
    }
    let bad = [(100, 50), (101, 51), (600, 400)];
    assert_eq!(
        calibrate(
            &mut sub,
            &Masters {
                dark: Some(&dark),
                flat: Some(&flat),
                bad_pixels: Some(&bad)
            }
        )
        .unwrap(),
        report("applied", "applied", "map", 3)
    );
    for (x, y, size) in [(0, 0, 100), (568, 361, 100), (295, 195, 10)] {
        let level = block(&sub, x, y, size);
        assert!(
            (level - 50.0).abs() <= 3.0,
            "block at {x},{y} reads {level}"
        );
    }
    for (x, y) in bad {
        let neighbours = vec![
            at(&sub, x - 2, y),
            at(&sub, x + 2, y),
            at(&sub, x, y - 2),
            at(&sub, x, y + 2),
        ];
        assert_eq!(at(&sub, x, y), median(neighbours));
    }
}

fn star(sigma: f64) -> Mosaic {
    let mut mosaic = background(16);
    for y in 0..16 {
        for x in 0..16 {
            let r2 = (f64::from(x) - 8.0).powi(2) + (f64::from(y) - 8.0).powi(2);
            let value = at(&mosaic, x, y) + (2000.0 * (-r2 / (2.0 * sigma * sigma)).exp()) as f32;
            put(&mut mosaic, x, y, value);
        }
    }
    mosaic
}

fn pair() -> Mosaic {
    let mut mosaic = background(16);
    put(&mut mosaic, 4, 4, 5100.0);
    put(&mut mosaic, 5, 5, 2300.0);
    mosaic
}

#[test]
fn touching_light_keeps_sharp_stars_and_distinguishes_pairs_from_triples() {
    for sigma in [0.8, 0.5] {
        let mut mosaic = star(sigma);
        assert!(find_hot_pixels(&mosaic).is_empty(), "star width {sigma}");
        let before = mosaic.clone();
        assert_eq!(
            calibrate(&mut mosaic, &Masters::default())
                .unwrap()
                .repaired,
            0
        );
        assert_eq!(at(&mosaic, 8, 8), 2100.0);
        assert_eq!(mosaic, before);
    }
    assert_eq!(find_hot_pixels(&pair()), [(4, 4), (5, 5)]);
    let mut nebula = background(16);
    for y in 0..16 {
        for x in 0..16 {
            if (f64::from(x) - 8.0).powi(2) + (f64::from(y) - 8.0).powi(2) <= 9.0 {
                let value = at(&nebula, x, y) + 50.0;
                put(&mut nebula, x, y, value);
            }
        }
    }
    put(&mut nebula, 8, 8, 5150.0);
    assert_eq!(find_hot_pixels(&nebula), [(8, 8)]);
    let mut faint = background(16);
    put(&mut faint, 4, 4, 120.0);
    put(&mut faint, 5, 4, 202.5);
    put(&mut faint, 4, 5, 202.5);
    assert_eq!(find_hot_pixels(&faint), [(4, 4)]);
    let mut triple = pair();
    put(&mut triple, 5, 4, 3200.0);
    assert!(find_hot_pixels(&triple).is_empty());
}

#[test]
fn lit_samples_counts_light_and_excludes_hot_samples() {
    assert_eq!(lit_samples(&star(0.8)), 21);
    let mut isolated = background(16);
    put(&mut isolated, 6, 6, 400.0);
    assert_eq!(lit_samples(&isolated), 0);
    assert_eq!(lit_samples(&pair()), 0);
    assert_eq!(lit_samples(&ratio_examples()), 9);
    assert_eq!(lit_samples(&uniform(16, 16, 160.0)), 0);
}

#[tokio::test(start_paused = true)]
async fn binned_simulator_star_survives_repair_of_the_three_hot_pixels() {
    let sim = sim();
    sim.set_sensor_pattern(hot_pattern());
    let truth = sim.imaging_truth_at(sim.clock().now() + 0.5);
    let camera = SimImagingCamera::new(sim)
        .with_noise(true)
        .with_stars(vec![SimStar {
            ra_deg: truth.ra_deg,
            dec_deg: truth.dec_deg,
            magnitude: 9.0,
            colour: [1.0; 3],
        }]);
    let mut sub = take(&camera, 1000.0, 2).await;
    let (index, largest) = sub
        .samples
        .iter()
        .copied()
        .enumerate()
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .unwrap();
    let place = (index as u32 % sub.width, index as u32 / sub.width);
    let bad = [(50, 24), (51, 25), (300, 200)];
    assert!(largest > 5000.0, "largest {largest} at {place:?}");
    assert!(!bad.contains(&place));
    assert_eq!(find_hot_pixels(&sub), bad);
    assert_eq!(
        calibrate(&mut sub, &Masters::default()).unwrap(),
        report("missing", "missing", "sub", 3)
    );
    assert_eq!(sub.samples[index], largest);
}
