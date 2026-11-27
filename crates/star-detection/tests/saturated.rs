//! US1-S1 to US1-S5: drawn shapes and places are the independent oracles.
//! Places allow 0.1 px per axis; the independently summed flux allows 1e-6.

use star_detection::{
    detect_stars,
    saturated::{saturated_stars, with_saturated_stars, SetAside},
    star::Star,
};

const WIDTH: usize = 640;
const HEIGHT: usize = 480;

fn draw(light: impl Fn(f64, f64) -> f64) -> Vec<u8> {
    (0..WIDTH * HEIGHT)
        .map(|index| {
            let x = (index % WIDTH) as f64 + 0.5;
            let y = (index / WIDTH) as f64 + 0.5;
            (10.0 + light(x, y)).round().clamp(0.0, 255.0) as u8
        })
        .collect()
}

fn gaussian(x: f64, y: f64, cx: f64, cy: f64, sigma: f64, peak: f64) -> f64 {
    peak * (-((x - cx).powi(2) + (y - cy).powi(2)) / (2.0 * sigma * sigma)).exp()
}

fn pair(distance: f64, second_peak: f64, halo: f64) -> Vec<u8> {
    draw(|x, y| {
        gaussian(x, y, 300.5 - distance / 2.0, 240.5, 3.0, 3000.0)
            + gaussian(x, y, 300.5 + distance / 2.0, 240.5, 3.0, second_peak)
            + gaussian(x, y, 300.5, 240.5, 40.0, halo)
    })
}

fn cedar(image: &[u8]) -> Vec<Star> {
    detect_stars(image, WIDTH, HEIGHT, 8.0, 2, true, true)
}

fn near(star: &Star, x: f64, y: f64, tolerance: f64) -> bool {
    (star.x - x).abs() <= tolerance && (star.y - y).abs() <= tolerance
}

fn rejected(image: &[u8], reasons: &[SetAside]) -> Vec<Star> {
    let stars = cedar(image);
    let (kept, set_aside) = saturated_stars(image, WIDTH, HEIGHT, &stars);
    assert!(kept.is_empty(), "unexpected stars: {kept:?}");
    assert_eq!(set_aside, reasons);
    assert_eq!(
        with_saturated_stars(image, WIDTH, HEIGHT, stars.clone()),
        stars
    );
    stars
}

// An independent rectangle sum, using the drawn frame's saturated pixels to
// locate the box. The ring is three pixels out; the summed inset is two out.
fn ring_subtracted_flux(image: &[u8]) -> f64 {
    let pixels: Vec<_> = image
        .iter()
        .enumerate()
        .filter(|&(_, &value)| value >= 250)
        .map(|(index, _)| (index % WIDTH, index / WIDTH))
        .collect();
    let left = pixels.iter().map(|p| p.0).min().unwrap();
    let right = pixels.iter().map(|p| p.0).max().unwrap();
    let top = pixels.iter().map(|p| p.1).min().unwrap();
    let bottom = pixels.iter().map(|p| p.1).max().unwrap();
    let mut ring = Vec::new();
    for y in top - 3..=bottom + 3 {
        for x in left - 3..=right + 3 {
            if x == left - 3 || x == right + 3 || y == top - 3 || y == bottom + 3 {
                ring.push(f64::from(image[y * WIDTH + x]));
            }
        }
    }
    let background = ring.iter().sum::<f64>() / ring.len() as f64;
    let mut flux = 0.0;
    for y in top - 2..=bottom + 2 {
        for x in left - 2..=right + 2 {
            flux += f64::from(image[y * WIDTH + x]) - background;
        }
    }
    flux
}

#[test]
fn us1_s1_round_saturated_stars_keep_their_places_and_brightness_order() {
    let mut frames = Vec::new();
    for p in [200.5, 200.75, 201.0] {
        for (sigma, peak) in [(1.5, 3000.0), (3.0, 3000.0), (3.0, 30000.0), (5.0, 3000.0)] {
            frames.push((
                draw(|x, y| {
                    gaussian(x, y, p, p, sigma, peak) + gaussian(x, y, 400.5, 300.5, 1.5, 100.0)
                }),
                p,
                p == 200.5 && sigma == 3.0 && peak == 3000.0,
            ));
        }
    }
    for beta in [2.5, 1.8] {
        frames.push((
            draw(|x, y| {
                let radius_squared = (x - 200.5).powi(2) + (y - 200.5).powi(2);
                25500.0 * (1.0 + radius_squared / 4.0).powf(-beta)
                    + gaussian(x, y, 400.5, 300.5, 1.5, 100.0)
            }),
            200.5,
            false,
        ));
    }
    let small = draw(|x, y| {
        gaussian(x, y, 200.5, 200.5, 1.5, 340.0) + gaussian(x, y, 400.5, 300.5, 1.5, 100.0)
    });
    assert_eq!(small.iter().filter(|&&v| v >= 250).count(), 5);
    frames.push((small, 200.5, false));

    for (image, p, check_flux) in frames {
        let stars = cedar(&image);
        assert!(stars.iter().any(|s| near(s, 400.5, 300.5, 0.5)));
        assert!(!stars.iter().any(|s| (s.x - p).hypot(s.y - p) <= 3.0));
        let (kept, set_aside) = saturated_stars(&image, WIDTH, HEIGHT, &stars);
        assert!(set_aside.is_empty(), "{set_aside:?}");
        assert_eq!(kept.len(), 1);
        let star = kept[0];
        assert!(near(&star, p, p, 0.1), "{p}: {star:?}");
        assert_eq!(star.peak_value, 255);
        assert_eq!(
            star.num_saturated,
            image.iter().filter(|&&v| v >= 250).count()
        );
        if check_flux {
            assert!((star.brightness - ring_subtracted_flux(&image)).abs() <= 1e-6);
        }
        let mut expected = vec![star];
        expected.extend_from_slice(&stars);
        assert_eq!(with_saturated_stars(&image, WIDTH, HEIGHT, stars), expected);

        let bright = Star::new(50.5, 50.5, 255, 1e6, 0);
        let faint = Star::new(600.5, 50.5, 20, 10.0, 0);
        assert_eq!(
            with_saturated_stars(&image, WIDTH, HEIGHT, vec![bright, faint]),
            vec![bright, star, faint]
        );
    }
}

#[test]
fn us1_s2_trails_doubles_nebulae_edges_and_hot_pixels_stay_out() {
    use SetAside::{Edge, Elongated, Extended, Small};
    for radius in [1.0, 2.0, 3.0] {
        let image = draw(|x, y| {
            let t = (((x - 150.0) * 300.0 + (y - 100.0) * 240.0)
                / (300.0_f64.powi(2) + 240.0_f64.powi(2)))
            .clamp(0.0, 1.0);
            if (x - (150.0 + t * 300.0)).hypot(y - (100.0 + t * 240.0)) <= radius {
                400.0
            } else {
                0.0
            }
        });
        rejected(&image, &[Elongated]);
    }
    for (distance, second_peak) in [
        (8.0, 3000.0),
        (10.0, 3000.0),
        (12.0, 3000.0),
        (8.0, 800.0),
        (12.0, 800.0),
    ] {
        rejected(&pair(distance, second_peak, 0.0), &[Elongated]);
    }
    // Shape takes precedence over the bright ring when no frame trail stands.
    rejected(&pair(12.0, 3000.0, 200.0), &[Elongated]);
    for (halo, sigma, peak) in [
        (200.0, 8.0, 400.0),
        (60.0, 8.0, 400.0),
        (0.0, 25.0, 400.0),
        (150.0, 3.0, 3000.0),
    ] {
        let image = draw(|x, y| {
            gaussian(x, y, 320.5, 240.5, 40.0, halo) + gaussian(x, y, 320.5, 240.5, sigma, peak)
        });
        rejected(&image, &[Extended]);
    }
    let lobes = draw(|x, y| {
        gaussian(x, y, 300.5, 230.5, 30.0, 180.0)
            + gaussian(x, y, 330.5, 250.5, 25.0, 180.0)
            + gaussian(x, y, 315.5, 270.5, 20.0, 150.0)
    });
    rejected(&lobes, &[Extended]);
    rejected(
        &draw(|x, y| gaussian(x, y, 5.5, 240.5, 3.0, 3000.0)),
        &[Edge],
    );
    let cedar_top = draw(|x, y| gaussian(x, y, 200.5, 200.5, 1.0, 1000.0));
    assert_eq!(cedar_top.iter().filter(|&&v| v >= 250).count(), 9);
    let stars = rejected(&cedar_top, &[]);
    assert!(stars.iter().any(|s| near(s, 200.5, 200.5, 0.5)));

    for (width, height, reasons) in [(1, 1, vec![]), (2, 2, vec![Small]), (4, 1, vec![Small])] {
        let mut image = vec![10; WIDTH * HEIGHT];
        for y in 240..240 + height {
            for x in 320..320 + width {
                image[y * WIDTH + x] = 255;
            }
        }
        let (kept, set_aside) = saturated_stars(&image, WIDTH, HEIGHT, &[]);
        assert!(kept.is_empty());
        assert_eq!(set_aside, reasons);
    }
    let mut touching = vec![10; WIDTH * HEIGHT];
    for (left, top) in [(300, 240), (303, 243)] {
        for y in top..top + 3 {
            for x in left..left + 3 {
                touching[y * WIDTH + x] = 255;
            }
        }
    }
    assert_eq!(
        saturated_stars(&touching, WIDTH, HEIGHT, &[]),
        (vec![], vec![Elongated])
    );
}

#[test]
fn us1_s3_round_merged_tops_and_separated_stars_are_kept() {
    for distance in [4.0, 6.0, 16.0, 24.0] {
        let image = pair(distance, 3000.0, 0.0);
        let (kept, set_aside) = saturated_stars(&image, WIDTH, HEIGHT, &cedar(&image));
        assert!(set_aside.is_empty(), "d={distance}: {set_aside:?}");
        assert!(kept.iter().all(|s| (s.y - 240.5).abs() <= 0.1));
        if distance <= 6.0 {
            assert_eq!(kept.len(), 1);
            assert!((kept[0].x - 300.5).abs() <= 0.1);
        } else {
            assert_eq!(kept.len(), 2);
            for x in [300.5 - distance / 2.0, 300.5 + distance / 2.0] {
                assert!(kept.iter().any(|s| (s.x - x).abs() <= 0.1));
            }
        }
    }
}

fn trailed(x: f64, y: f64, cx: f64, cy: f64) -> f64 {
    let angle = 20.0_f64.to_radians();
    // Nine pixels, averaged over eighteen evenly spaced centres, before clipping.
    (0..18)
        .map(|i| {
            let offset = 9.0 * (i as f64 / 17.0 - 0.5);
            gaussian(
                x,
                y,
                cx + offset * angle.cos(),
                cy + offset * angle.sin(),
                1.5,
                3000.0,
            )
        })
        .sum::<f64>()
        / 18.0
}

#[test]
fn us1_s4_a_common_trail_keeps_stars_but_not_the_double_or_edge() {
    use SetAside::{Edge, Elongated};
    let places = [
        (100.5, 100.5),
        (300.5, 150.5),
        (500.5, 100.5),
        (150.5, 350.5),
        (450.5, 380.5),
    ];
    for count in [5, 3, 2] {
        let image = draw(|x, y| {
            places[..count]
                .iter()
                .map(|&(cx, cy)| trailed(x, y, cx, cy))
                .sum::<f64>()
                + gaussian(x, y, 295.5, 300.5, 3.0, 3000.0)
                + gaussian(x, y, 305.5, 300.5, 3.0, 3000.0)
        });
        let (kept, set_aside) = saturated_stars(&image, WIDTH, HEIGHT, &cedar(&image));
        if count >= 3 {
            assert_eq!(kept.len(), count);
            for &(cx, cy) in &places[..count] {
                assert!(kept.iter().any(|s| near(s, cx, cy, 0.1)));
            }
            assert_eq!(set_aside, vec![Elongated]);
        } else {
            assert!(kept.is_empty());
            assert_eq!(set_aside, vec![Elongated; 3]);
        }
    }
    let image = draw(|x, y| {
        trailed(x, y, places[0].0, places[0].1)
            + trailed(x, y, places[1].0, places[1].1)
            + trailed(x, y, 5.5, 240.5)
    });
    let (kept, set_aside) = saturated_stars(&image, WIDTH, HEIGHT, &cedar(&image));
    assert!(kept.is_empty());
    assert_eq!(set_aside, vec![Elongated, Elongated, Edge]);
}

#[test]
fn us1_s5_the_frame_bound_rejects_a_large_disc_and_a_white_frame() {
    for radius in [88.0, 89.0] {
        let image = draw(|x, y| {
            if (x - 320.0).hypot(y - 240.0) <= radius {
                1000.0
            } else {
                0.0
            }
        });
        let (kept, set_aside) = saturated_stars(&image, WIDTH, HEIGHT, &[]);
        if radius == 88.0 {
            assert_eq!(kept.len(), 1);
            assert!(near(&kept[0], 320.0, 240.0, 0.1));
            assert!(set_aside.is_empty());
        } else {
            assert!(kept.is_empty());
            assert_eq!(set_aside, vec![SetAside::Extended]);
        }
    }
    let white = vec![255; WIDTH * HEIGHT];
    assert_eq!(
        saturated_stars(&white, WIDTH, HEIGHT, &[]),
        (vec![], vec![SetAside::Extended])
    );
    assert!(with_saturated_stars(&white, WIDTH, HEIGHT, vec![]).is_empty());
}
