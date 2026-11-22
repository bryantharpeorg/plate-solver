//! US1-S3/S4/S5: seeded skies, with drawn positions and camera direction as oracles.
mod dark;

use dark::{drawn, field, floor, frame, render, solve, true_stars, CITY, DARK};
use scope_node::detect::{detect, DetectSettings};

#[test]
fn dark_sky_frame_without_a_floor_fills_the_limit_mostly_with_noise() {
    let field = field();
    let pixels = render(&field, DARK, 1.0, 13);
    let zeros = pixels.iter().filter(|&&pixel| pixel == 0).count();
    assert!(
        zeros as f64 / pixels.len() as f64 >= 0.8,
        "{zeros} zero pixels"
    );

    let centroids = detect(&frame(pixels), &DetectSettings::cedar());
    let stars = drawn(&field, DARK, 1.0);
    assert_eq!(centroids.len(), 200);
    let all_true = true_stars(&centroids, &stars);
    assert!(all_true <= 15, "{all_true} true detections");
    let brightest_true = true_stars(&centroids[..30], &stars);
    assert!(
        brightest_true <= 10,
        "{brightest_true} true among the brightest 30"
    );
}

#[test]
fn dark_sky_frame_with_a_floor_keeps_stars_and_removes_noise_points() {
    let field = field();
    let centroids = detect(&frame(render(&field, DARK, 1.0, 13)), &floor(1.0));
    assert!(centroids.len() >= 4, "{} detections", centroids.len());
    assert_eq!(
        true_stars(&centroids, &drawn(&field, DARK, 1.0)),
        centroids.len()
    );
}

#[test]
fn city_frame_solves_and_both_floors_preserve_every_centroid() {
    let field = field();
    let frame = frame(render(&field, CITY, 1.0, 13));
    let centroids = detect(&frame, &DetectSettings::cedar());
    let error = solve(&field, &centroids).expect("city frame solves with the console's settings");
    assert!(error <= 30.0, "camera direction error {error} arcsec");
    for value in [0.5, 1.0] {
        assert_eq!(detect(&frame, &floor(value)), centroids, "floor {value}");
    }
}
