//! Acceptance tests for the solve loop.
//!
//! Covers these three Requirement sections of the `plate-solver` spec:
//!   - Solve inputs and defaults
//!   - Preparation
//!   - Image-pattern iteration

use pattern_database::{DatabaseProperties, PatternDatabase, StarKdTree};
use plate_solver::{
    preparation,
    solve::{solve_from_centroids, solve_from_image, DetectParams},
    SolveStatus,
};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

/// Build a minimal in-memory database with the requested FOV range and pattern count.
fn test_db(
    min_fov: f32,
    max_fov: f32,
    num_patterns: u32,
    verification_stars_per_fov: u16,
) -> PatternDatabase {
    PatternDatabase {
        star_table: Vec::new(),
        num_stars: 0,
        pattern_catalog: Vec::new(),
        pattern_largest_edge: Vec::new(),
        pattern_key_hashes: Vec::new(),
        star_catalog_ids: Vec::new(),
        properties: DatabaseProperties {
            min_fov,
            max_fov,
            num_patterns,
            verification_stars_per_fov,
            ..DatabaseProperties::default()
        },
        star_kdtree: StarKdTree::new(&[]),
    }
}

#[test]
fn exhausted_search_of_an_empty_pattern_catalog_is_no_match() {
    let db = test_db(10.0, 30.0, 100, 150);
    let bright = vec![
        (10.0, 10.0),
        (20.0, 20.0),
        (30.0, 30.0),
        (40.0, 40.0),
        (50.0, 50.0),
    ];
    let sol = solve_from_centroids(
        &bright,
        (100, 100),
        None,
        5.0,
        0.01,
        1e-5,
        5000,
        0.0,
        0.002,
        db,
    );
    // With no real candidates the loop exhausts and returns NO_MATCH, not TOO_FEW.
    assert_eq!(sol.status, Some(SolveStatus::NoMatch));
}

#[test]
fn default_fov_from_db_range() {
    let db = test_db(10.0, 30.0, 100, 150);
    let sol = solve_from_centroids(
        &[(10.0, 10.0), (20.0, 20.0), (30.0, 30.0), (40.0, 40.0)],
        (100, 100),
        None,
        0.0,
        0.01,
        1e-5,
        5000,
        0.0,
        0.002,
        db,
    );
    // Midpoint of [10, 30] is 20 degrees.
    assert_eq!(sol.fov_used, Some(20.0));
}

#[test]
fn noise_estimated_from_image_never_constant() {
    // Two different images must produce different recorded noise estimates.
    // Use structured images where the middle row (used by the estimator) has
    // differing variance.
    let mut dark = vec![10u8; 64 * 64];
    for x in 0..64 {
        dark[32 * 64 + x] = ((x % 4) * 2) as u8;
    }
    let mut bright = vec![180u8; 64 * 64];
    for x in 0..64 {
        bright[32 * 64 + x] = 180 + ((x % 4) * 10) as u8;
    }
    let db = test_db(10.0, 30.0, 100, 150);

    let params = DetectParams {
        noise_estimate: None,
        ..Default::default()
    };

    let sol_dark = solve_from_image(
        &dark,
        64,
        64,
        None,
        5.0,
        0.01,
        1e-5,
        5000,
        0.0,
        0.002,
        db.clone(),
        params,
    );
    let sol_bright = solve_from_image(
        &bright, 64, 64, None, 5.0, 0.01, 1e-5, 5000, 0.0, 0.002, db, params,
    );

    let noise_dark = sol_dark.noise_estimate.unwrap();
    let noise_bright = sol_bright.noise_estimate.unwrap();
    assert!(
        (noise_dark - noise_bright).abs() > 1e-6,
        "noise estimates should differ: dark={}, bright={}",
        noise_dark,
        noise_bright
    );
    assert_ne!(noise_dark, 1.0, "noise must not be a fixed constant");
    assert_ne!(noise_bright, 1.0, "noise must not be a fixed constant");
}

#[test]
fn detection_parameters_do_not_perturb_solve_math() {
    // Same centroids fed to both paths should see the same preparation/iteration behavior.
    let centroids = vec![
        (10.0, 10.0),
        (20.0, 20.0),
        (30.0, 30.0),
        (40.0, 40.0),
        (50.0, 50.0),
    ];
    let db = test_db(10.0, 30.0, 100, 150);
    let sol_centroids = solve_from_centroids(
        &centroids,
        (100, 100),
        None,
        5.0,
        0.01,
        1e-5,
        5000,
        0.0,
        0.002,
        db.clone(),
    );

    // Build a synthetic image containing exactly those centroids as bright peaks.
    let mut image = vec![0u8; 100 * 100];
    for &(y, x) in &centroids {
        let iy = y as usize;
        let ix = x as usize;
        if iy < 100 && ix < 100 {
            image[iy * 100 + ix] = 255;
        }
    }
    let params = DetectParams {
        sigma: 2.0,
        noise_estimate: Some(0.5),
        binning: 1,
        normalize_rows: false,
        detect_hot_pixels: false,
        return_binned: false,
        use_binned_for_star_candidates: false,
    };
    let sol_image = solve_from_image(
        &image, 100, 100, None, 5.0, 0.01, 1e-5, 5000, 0.0, 0.002, db, params,
    );

    // Both should reach the same terminal status (NO_MATCH with no real candidates).
    assert_eq!(sol_centroids.status, sol_image.status);
    assert_eq!(sol_centroids.fov_used, sol_image.fov_used);
}

#[test]
fn threshold_bonferroni_corrected() {
    let match_threshold = 1e-5;
    let num_patterns = 1000u32;
    let corrected = preparation::bonferroni_threshold(match_threshold, num_patterns);
    assert!((corrected - match_threshold / num_patterns as f64).abs() < 1e-18);
}

#[test]
fn cluster_busting_limits_pattern_centroids() {
    let fov = 20.0_f64.to_radians();
    let n = 100usize;
    let width = 1024.0;
    // A tight cluster of 10 centroids around the center.
    let mut centroids: Vec<(f64, f64)> = (0..10)
        .map(|i| (512.0 + i as f64 * 0.5, 512.0 + i as f64 * 0.5))
        .collect();
    // Plus a few well-separated centroids.
    centroids.push((100.0, 100.0));
    centroids.push((900.0, 900.0));
    centroids.push((100.0, 900.0));
    centroids.push((900.0, 100.0));

    let busted = preparation::cluster_bust(&centroids, fov, n, width);
    // The cluster should be thinned to at most one point.
    let cluster_points: Vec<_> = busted
        .iter()
        .filter(|(y, x)| (*y - 512.0).abs() < 10.0 && (*x - 512.0).abs() < 10.0)
        .collect();
    assert!(
        cluster_points.len() <= 1,
        "cluster-busting should thin the tight cluster, got {} points",
        cluster_points.len()
    );
    // The four separated points should survive.
    assert!(busted.len() >= 4);
}

#[test]
fn too_few_centroids() {
    let db = test_db(10.0, 30.0, 100, 150);
    let sol = solve_from_centroids(
        &[(10.0, 10.0), (20.0, 20.0), (30.0, 30.0)],
        (100, 100),
        None,
        5.0,
        0.01,
        1e-5,
        5000,
        0.0,
        0.002,
        db,
    );
    assert_eq!(sol.status, Some(SolveStatus::TooFew));
}

#[test]
fn timeout_bounds_the_search() {
    let db = test_db(10.0, 30.0, 100, 150);
    // Enough centroids that the inner loops do real work; with a zero timeout the
    // first `should_stop()` check returns TIMEOUT before any candidate is accepted.
    let centroids: Vec<(f64, f64)> = (0..20).map(|i| (i as f64 * 5.0, i as f64 * 5.0)).collect();
    let sol = solve_from_centroids(
        &centroids,
        (200, 200),
        None,
        5.0,
        0.01,
        1e-5,
        0,
        0.0,
        0.002,
        db,
    );
    assert_eq!(sol.status, Some(SolveStatus::Timeout));
}

#[test]
fn solve_context_reports_a_set_cancel_flag() {
    // SolveStatus::Cancelled has no test and no public trigger. This checks only
    // the context flag; timeout_bounds_the_search covers the zero-timeout solve.
    let db = test_db(10.0, 30.0, 100, 150);
    let cancelled = Arc::new(AtomicBool::new(true));
    let ctx = plate_solver::status::SolveContext {
        db,
        props: DatabaseProperties::default(),
        fov_initial: 20.0_f64.to_radians(),
        fov_estimate: None,
        fov_max_error: None,
        match_threshold: 1e-8,
        match_radius: 0.01,
        match_max_error: 0.002,
        distortion: 0.0,
        solve_timeout_ms: 60_000,
        start_instant: std::time::Instant::now(),
        cancelled: cancelled.clone(),
        verification_stars_per_fov: 150,
    };

    assert!(ctx.is_cancelled());
}
