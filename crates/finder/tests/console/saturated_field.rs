//! 052 US2: catalog projections and simulator camera directions are independent
//! oracles. Bright stars must return within 1.5 px, solves within 30 arcsec
//! (60 arcsec for live reports read after exposure). Cedar alone must not match.

use crate::support::live::{event, request, DB, START};
use axum::http::StatusCode;
use database_generation::{
    build_pattern_catalog, clean_and_limit, enumerate_patterns, serialize_to_path, CatalogEntry,
    CatalogId,
};
use finder::{
    samples::Samples,
    simulate::SimulationTransport,
    solve::{self, SolveOptions, SolveReport, Status},
    AppState,
};
use math_core::{angular_distance, UnitVector};
use pattern_database::{DatabaseProperties, PatternDatabase};
use scope_link::proto::Centroid;
use scope_node::{
    detect::{detect, DetectSettings},
    Camera, GrayFrame, Settings,
};
use serde_json::json;
use simulator::{Optics, SimCamera, Simulator, Site, TrueDirection, VirtualClock};
use std::{io::Cursor, path::Path, sync::Arc, time::Duration};

const FIELDS: [(f64, f64); 2] = [(83.82, -5.39), (56.75, 24.12)];

fn set_field(sim: &Simulator, ra: f64, dec: f64, fov_deg: f64) {
    sim.set_optics(Optics {
        width: 1280,
        height: 960,
        fov_deg,
        eyepiece_px: (640.0, 480.0),
    });
    sim.point_eyepiece_at(ra, dec);
    sim.set_finder_focus_error(3.0);
}

async fn bright_frame(ra: f64, dec: f64) -> (Simulator, GrayFrame) {
    let sim = Simulator::new(DB.clone(), Site::default(), VirtualClock::new(START));
    set_field(&sim, ra, dec, 10.5);
    let mut camera = SimCamera::new(sim.clone());
    camera
        .apply(&Settings {
            exposure_ms: Some(500.0),
            gain: Some(2.0),
            ..Default::default()
        })
        .unwrap();
    let frame = camera.next_frame().await.unwrap();
    (sim, frame)
}

fn cedar(frame: &GrayFrame) -> Vec<Centroid> {
    let params = plate_solver::DetectParams::cedar();
    star_detection::detect_stars(
        &frame.pixels,
        frame.width as usize,
        frame.height as usize,
        params.sigma,
        params.binning,
        params.normalize_rows,
        params.detect_hot_pixels,
    )
    .into_iter()
    .take(200)
    .map(|s| Centroid {
        x: s.x,
        y: s.y,
        brightness: s.brightness,
    })
    .collect()
}

fn solved_near(report: &SolveReport, truth: TrueDirection, tolerance_arcsec: f64) {
    assert_eq!(report.status, Status::MatchFound, "{report:?}");
    let pointing = report.pointing.as_ref().unwrap();
    let actual =
        UnitVector::from_radec(pointing.ra_deg.to_radians(), pointing.dec_deg.to_radians());
    let expected = UnitVector::from_radec(truth.ra.to_radians(), truth.dec.to_radians());
    let error = angular_distance(actual, expected).to_degrees() * 3600.0;
    assert!(error < tolerance_arcsec, "pointing error {error} arcsec");
}

fn compare_solves(db: &PatternDatabase, frame: &GrayFrame, truth: TrueDirection) {
    let size = (frame.width as usize, frame.height as usize);
    let options = SolveOptions::default();
    let baseline = solve::solve_centroids(db, &cedar(frame), size, &options).0;
    assert_eq!(
        baseline.status,
        Status::NoMatch,
        "cedar alone: {baseline:?}"
    );
    let node = detect(frame, &DetectSettings::cedar());
    solved_near(
        &solve::solve_centroids(db, &node, size, &options).0,
        truth,
        30.0,
    );
}

#[tokio::test(start_paused = true)]
async fn us2_s2_bright_fields_keep_the_missing_stars_and_solve() {
    for (ra, dec) in FIELDS {
        let (sim, frame) = bright_frame(ra, dec).await;
        let node = detect(&frame, &DetectSettings::cedar());
        let baseline = cedar(&frame);
        let drawn: Vec<_> = sim
            .field_stars(500.0, 2.0)
            .into_iter()
            .map(|(_, star)| star)
            .filter(|s| s.x >= 20.0 && s.x < 1260.0 && s.y >= 20.0 && s.y < 940.0)
            .take(5)
            .collect();
        assert_eq!(drawn.len(), 5, "exercise five bright stars in each field");
        for star in drawn {
            assert!(
                node.iter()
                    .take(10)
                    .any(|s| (s.x - star.x).hypot(s.y - star.y) < 1.5),
                "field ({ra}, {dec}) missing bright star {star:?} in the node's first ten"
            );
            assert!(baseline
                .iter()
                .all(|s| (s.x - star.x).hypot(s.y - star.y) >= 3.0));
        }
        compare_solves(&DB, &frame, sim.telescope().camera);
    }
}

#[tokio::test(start_paused = true)]
async fn us2_s3_live_loop_solves_three_bright_frames_within_thirty_seconds() {
    let state = AppState::shared(DB.clone(), Samples::default())
        .with_simulation(START, SimulationTransport::Memory)
        .await
        .unwrap()
        .with_live();
    let sim = state.simulator().unwrap();
    sim.stop_frames();
    set_field(&sim, FIELDS[0].0, FIELDS[0].1, 10.5);
    let mut reports = state.live().unwrap().subscribe();
    let app = finder::router(state);
    let (status, body) = request(
        &app,
        "PUT",
        "/api/session",
        json!({"camera_settings": {"exposure_ms": 500, "gain": 2}}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    sim.start_frames();
    tokio::time::timeout(Duration::from_secs(30), async {
        let mut count = 0;
        while count < 3 {
            let report = event(&mut reports).await;
            // An exposure already in flight can still have the old 1456-pixel optics.
            if report.report.width != 1280 {
                continue;
            }
            solved_near(&report.report, sim.telescope().camera, 60.0);
            assert!(sim.clock().now() - START <= 30.0);
            count += 1;
        }
    })
    .await
    .expect("three 1280-wide live reports within thirty seconds");
}

fn narrow_database() -> Arc<PatternDatabase> {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR"));
    std::fs::create_dir_all(folder).unwrap();
    let path = folder.join("052-tycho-pleiades.npz");
    if !path.exists() {
        let fixture = include_str!("../../../../fixtures/finder/tycho-pleiades.csv");
        let mut entries: Vec<_> = fixture
            .lines()
            .skip(2)
            .enumerate()
            .map(|(i, row)| {
                let values: Vec<f64> = row.split(',').map(|v| v.parse().unwrap()).collect();
                assert_eq!(values.len(), 3);
                CatalogEntry {
                    ra: values[0].to_radians(),
                    dec: values[1].to_radians(),
                    mag: values[2],
                    id: CatalogId::Tyc(i as u32 + 1, 1, 1),
                    pm_ra: None,
                    pm_dec: None,
                }
            })
            .collect();
        assert_eq!(entries.len(), 203);
        clean_and_limit(&mut entries, None);
        let patterns = enumerate_patterns(&entries, 1.6, 1.85, 150.0, 20.0, 50);
        let catalog = build_pattern_catalog(&entries, &patterns, 0.001, false);
        let properties = DatabaseProperties {
            pattern_mode: "edge_ratio".into(),
            hash_table_type: "quadratic_probe".into(),
            pattern_size: 4,
            pattern_bins: catalog.pattern_bins as u16,
            pattern_max_error: 0.001,
            min_fov: 1.6,
            max_fov: 1.85,
            star_catalog: "tyc_main".into(),
            epoch_equinox: 2000,
            epoch_proper_motion: 2026.9,
            verification_stars_per_fov: 150,
            star_max_magnitude: entries.last().unwrap().mag as f32,
            num_patterns: catalog.num_patterns as u32,
        };
        let part = folder.join(format!(
            "052-tycho-pleiades.npz.{}.part",
            std::process::id()
        ));
        serialize_to_path(&part, &entries, &catalog, &properties).unwrap();
        std::fs::rename(part, &path).unwrap();
    }
    Arc::new(pattern_database::load_from_path(&path).unwrap())
}

fn narrow_frame(sim: &Simulator, trailed: bool) -> GrayFrame {
    let optics = sim.optics();
    let (width, height) = (optics.width as usize, optics.height as usize);
    let mut pixels = vec![10.0; width * height];
    let reach = if trailed { 16 } else { 64 };
    for (_, star) in sim.field_stars(1e9, 1.0) {
        let peak = 255.0 * 10.0_f64.powf(-0.4 * (star.magnitude - 8.5));
        for y in (star.y.floor() as i64 - reach).max(0)
            ..=(star.y.floor() as i64 + reach).min(height as i64 - 1)
        {
            for x in (star.x.floor() as i64 - reach).max(0)
                ..=(star.x.floor() as i64 + reach).min(width as i64 - 1)
            {
                let dx = x as f64 + 0.5 - star.x;
                let dy = y as f64 + 0.5 - star.y;
                let profile = if trailed {
                    let angle = 20.0_f64.to_radians();
                    (0..18)
                        .map(|i| {
                            let offset = 9.0 * (f64::from(i) / 17.0 - 0.5);
                            let r2 = (dx - offset * angle.cos()).powi(2)
                                + (dy - offset * angle.sin()).powi(2);
                            (-r2 / (2.0 * 1.3_f64.powi(2))).exp()
                        })
                        .sum::<f64>()
                        / 18.0
                } else {
                    (1.0 + (dx * dx + dy * dy) / 2.5_f64.powi(2)).powf(-1.8)
                };
                pixels[y as usize * width + x as usize] += peak * profile;
            }
        }
    }
    GrayFrame {
        width: optics.width,
        height: optics.height,
        pixels: pixels
            .into_iter()
            .map(|v| v.round().clamp(0.0, 255.0) as u8)
            .collect(),
        start_s: 0.0,
        end_s: 0.5,
        exposure_ms: 500.0,
        gain: 1.0,
    }
}

#[test]
fn us2_s4_moffat_and_trailed_pleiades_solve_at_the_finder_cameras_scale() {
    let db = narrow_database();
    let sim = Simulator::new(db.clone(), Site::default(), VirtualClock::new(START));
    set_field(&sim, FIELDS[1].0, FIELDS[1].1, 1.617);
    for trailed in [false, true] {
        let frame = narrow_frame(&sim, trailed);
        compare_solves(&db, &frame, sim.telescope().camera);
    }
}

#[tokio::test(start_paused = true)]
async fn us2_s5_uploaded_pngs_detect_as_the_node_and_solve() {
    for (ra, dec) in FIELDS {
        let (sim, frame) = bright_frame(ra, dec).await;
        let png =
            image::GrayImage::from_raw(frame.width, frame.height, frame.pixels.clone()).unwrap();
        let mut bytes = Cursor::new(Vec::new());
        png.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
        let image = solve::decode_gray(bytes.get_ref()).unwrap();
        let report = solve::solve_frame(&DB, &image, &SolveOptions::default());
        let node = detect(&frame, &DetectSettings::cedar());
        let places: Vec<_> = node.iter().map(|s| [s.x, s.y]).collect();
        assert_eq!(
            report
                .detected
                .iter()
                .take(200)
                .copied()
                .collect::<Vec<_>>(),
            places
        );
        solved_near(&report, sim.telescope().camera, 30.0);
    }
}
