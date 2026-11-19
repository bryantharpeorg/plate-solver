//! 035 US2-S1/S2: worked examples of touching-sample means. All values are exact.
//! Patterns and expected planes are written here independently of Mosaic::colour.

use finder::imaging::{
    calibration::Mosaic,
    stack::{colour, Colour},
};

#[test]
fn flat_colours_include_every_edge_and_corner_in_all_four_patterns() {
    for pattern in ["RGGB", "BGGR", "GRBG", "GBRG"] {
        let mut samples = Vec::new();
        for y in 0..4 {
            for x in 0..4 {
                samples.push(match pattern.as_bytes()[2 * (y % 2) + x % 2] {
                    b'R' => 100.0,
                    b'G' => 200.0,
                    b'B' => 300.0,
                    _ => unreachable!(),
                });
            }
        }
        let mosaic = Mosaic {
            width: 4,
            height: 4,
            bayer: Some(pattern.into()),
            samples,
        };
        let picture = colour(&mosaic);
        assert_eq!((picture.width, picture.height), (4, 4));
        assert_eq!(picture.planes.len(), 48);
        for (channel, value) in [100.0, 200.0, 300.0].into_iter().enumerate() {
            assert_eq!(picture.plane(channel), &[value; 16], "{pattern}");
            for y in 0..4 {
                for x in 0..4 {
                    assert_eq!(picture.at(channel, x, y), value, "{pattern}, ({x}, {y})");
                }
            }
        }
    }
}

#[test]
fn mono_and_mosaics_under_two_wide_or_high_copy_samples_to_each_plane() {
    for (width, height, bayer, samples) in [
        (3, 2, None, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]),
        (1, 4, Some("RGGB"), vec![1.0, 2.0, 3.0, 4.0]),
        (4, 1, Some("RGGB"), vec![1.0, 2.0, 3.0, 4.0]),
        (1, 1, Some("BGGR"), vec![7.0]),
    ] {
        let mosaic = Mosaic {
            width,
            height,
            bayer: bayer.map(str::to_owned),
            samples,
        };
        let picture = colour(&mosaic);
        assert_eq!((picture.width, picture.height), (width, height));
        for channel in 0..3 {
            assert_eq!(picture.plane(channel), mosaic.samples);
        }
    }
}

#[test]
fn one_red_sample_spreads_by_halves_and_quarters_only_in_red() {
    let mut mosaic = Mosaic {
        width: 6,
        height: 6,
        bayer: Some("RGGB".into()),
        samples: vec![0.0; 36],
    };
    mosaic.samples[2 * 6 + 2] = 400.0;
    let picture = colour(&mosaic);
    for y in 0..6 {
        for x in 0..6 {
            let expected = match (x, y) {
                (2, 2) => 400.0,
                (1, 2) | (3, 2) | (2, 1) | (2, 3) => 200.0,
                (1, 1) | (3, 1) | (1, 3) | (3, 3) => 100.0,
                _ => 0.0,
            };
            assert_eq!(picture.at(0, x, y), expected, "({x}, {y})");
        }
    }
    assert_eq!(picture.plane(0).iter().sum::<f32>(), 1600.0);
    assert_eq!(picture.plane(1), &[0.0; 36]);
    assert_eq!(picture.plane(2), &[0.0; 36]);
}

#[test]
fn one_corner_red_sample_uses_only_neighbours_inside_the_mosaic() {
    let mut samples = vec![0.0; 16];
    samples[0] = 400.0;
    let picture = colour(&Mosaic {
        width: 4,
        height: 4,
        bayer: Some("RGGB".into()),
        samples,
    });
    for y in 0..4 {
        for x in 0..4 {
            let expected = match (x, y) {
                (0, 0) => 400.0,
                (1, 0) | (0, 1) => 200.0,
                (1, 1) => 100.0,
                _ => 0.0,
            };
            assert_eq!(picture.at(0, x, y), expected, "({x}, {y})");
        }
    }
    assert_eq!(picture.plane(1), &[0.0; 16]);
    assert_eq!(picture.plane(2), &[0.0; 16]);
}

#[test]
fn one_green_sample_spreads_by_quarters_only_in_green() {
    let mut samples = vec![0.0; 36];
    samples[2 * 6 + 3] = 400.0;
    let picture = colour(&Mosaic {
        width: 6,
        height: 6,
        bayer: Some("RGGB".into()),
        samples,
    });
    for y in 0..6 {
        for x in 0..6 {
            let expected = match (x, y) {
                (3, 2) => 400.0,
                (2, 2) | (4, 2) | (3, 1) | (3, 3) => 100.0,
                _ => 0.0,
            };
            assert_eq!(picture.at(1, x, y), expected, "({x}, {y})");
        }
    }
    assert_eq!(picture.plane(1).iter().sum::<f32>(), 800.0);
    assert_eq!(picture.plane(0), &[0.0; 36]);
    assert_eq!(picture.plane(2), &[0.0; 36]);
}

#[test]
fn colour_storage_is_three_row_major_planes_starting_at_zero() {
    let mut picture = Colour::new(3, 2);
    assert_eq!((picture.width, picture.height), (3, 2));
    assert_eq!(picture.planes, vec![0.0; 18]);
    picture.set(0, 2, 1, 10.0);
    picture.set(1, 1, 0, 20.0);
    picture.set(2, 0, 1, 30.0);
    assert_eq!(picture.planes[5], 10.0);
    assert_eq!(picture.planes[7], 20.0);
    assert_eq!(picture.planes[15], 30.0);
    assert_eq!(picture.plane(0), &[0.0, 0.0, 0.0, 0.0, 0.0, 10.0]);
    assert_eq!(picture.plane(1), &[0.0, 20.0, 0.0, 0.0, 0.0, 0.0]);
    assert_eq!(picture.plane(2), &[0.0, 0.0, 0.0, 30.0, 0.0, 0.0]);
    assert_eq!(picture.at(1, 1, 0), 20.0);
}
