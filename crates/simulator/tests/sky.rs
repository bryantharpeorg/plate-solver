//! Catalog-rendered frames checked through the real camera port, detector and solver.
use math_core::{angular_distance, UnitVector};
use pattern_database::{PatternDatabase, StarId};
use plate_solver::{solve_from_centroids, SolveStatus};
use scope_node::{
    detect::{self, DetectSettings},
    Camera, Settings,
};
use simulator::{Optics, SimCamera, Simulator, Site, VirtualClock};
use std::{
    path::Path,
    sync::{Arc, LazyLock},
};

static DB: LazyLock<Arc<PatternDatabase>> = LazyLock::new(|| {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../reference-solutions/cedar-solve/tetra3/data/default_database.npz");
    Arc::new(pattern_database::load_from_path(&path).expect("bundled cedar database"))
});

fn simulator() -> Simulator {
    Simulator::new(
        DB.clone(),
        Site::default(),
        VirtualClock::new(1_768_528_800.0),
    )
}

#[tokio::test(start_paused = true)]
async fn adapter_reports_optics_ranges_and_exact_exposure() {
    let sim = simulator();
    let mut camera: Box<dyn Camera> = Box::new(SimCamera::new(sim.clone()));
    let info = camera.info();
    assert_eq!(info.model, "simulator");
    assert_eq!((info.width, info.height), (1456, 1088));
    assert_eq!((info.exposure_min_ms, info.exposure_max_ms), (1.0, 2000.0));
    assert_eq!((info.gain_min, info.gain_max), (1.0, 16.0));
    assert_eq!(info.modes, vec![info.active_mode.unwrap()]);
    camera
        .apply(&Settings {
            exposure_ms: Some(123.456),
            gain: Some(2.0),
            ..Settings::default()
        })
        .unwrap();
    let start = sim.clock().node_now();
    let frame = camera.next_frame().await.unwrap();
    assert_eq!((frame.width, frame.height), (1456, 1088));
    assert_eq!(frame.pixels.len(), 1456 * 1088);
    assert_eq!((frame.exposure_ms, frame.gain), (123.456, 2.0));
    assert_eq!(frame.start_s, start);
    assert!((frame.end_s - frame.start_s - 0.123456).abs() < 1e-6);
    assert!(sim.clock().node_now() >= frame.end_s);
}

#[tokio::test(start_paused = true)]
async fn rendered_sky_solves_to_the_true_camera_direction() {
    let sim = simulator();
    sim.push_to(50.0, 120.0);
    let mut camera = SimCamera::new(sim.clone());
    let frame = camera.next_frame().await.unwrap();
    let detected = detect::detect(&frame, &DetectSettings::cedar());
    assert!(
        !detected.is_empty(),
        "a blank frame must never pass this test"
    );
    // Exactly the console's SolveOptions defaults and match parameters.
    let centroids: Vec<_> = detected.iter().take(30).map(|s| (s.y, s.x)).collect();
    let solution = solve_from_centroids(
        &centroids,
        (1456, 1088),
        None,
        10.0,
        0.01,
        1e-5,
        5000,
        0.0,
        0.002,
        &**DB,
    );
    assert_eq!(
        solution.status,
        Some(SolveStatus::MatchFound),
        "{solution:?}; detected {}",
        detected.len()
    );
    let solved = UnitVector::from_radec(solution.ra.unwrap(), solution.dec.unwrap());
    // True direction at mid-exposure, the instant at which the scene is rendered.
    let truth = sim
        .directions_at(sim.clock().start_unix_s() + (frame.start_s + frame.end_s) / 2.0 - 1000.0)
        .camera;
    assert_eq!((truth.alt, truth.az), (50.0, 120.0));
    let error = angular_distance(
        solved,
        UnitVector::from_radec(truth.ra.to_radians(), truth.dec.to_radians()),
    )
    .to_degrees()
        * 3600.0;
    assert!(error < 30.0, "camera direction error {error} arcsec");
}

#[tokio::test(start_paused = true)]
async fn centring_a_catalog_star_in_the_eyepiece_places_it_on_the_known_pixel() {
    let sim = simulator();
    sim.push_to(50.0, 120.0);
    let star = sim
        .field_stars(200.0, 1.0)
        .first()
        .expect("a star in the field")
        .0;
    let vector = DB.star_vector(StarId(star)).unwrap().normalize().unwrap();
    let (ra, dec) = vector.to_radec();
    sim.point_eyepiece_at(ra.to_degrees(), dec.to_degrees());
    let truth = sim.telescope();
    assert!(
        angular_distance(
            vector,
            UnitVector::from_radec(
                truth.eyepiece.ra.to_radians(),
                truth.eyepiece.dec.to_radians()
            )
        ) < 1e-9
    );
    let frame = SimCamera::new(sim.clone()).next_frame().await.unwrap();
    let stars = detect::detect(&frame, &DetectSettings::cedar());
    let distance = stars
        .iter()
        .map(|s| (s.x - 700.0).hypot(s.y - 420.0))
        .reduce(f64::min)
        .expect("detected stars");
    assert!(
        distance < 0.3,
        "eyepiece star is {distance} pixels from (700, 420)"
    );
    assert_eq!(sim.optics(), Optics::default());
}

#[tokio::test(start_paused = true)]
async fn changed_optics_and_ranges_reach_the_port_and_refusals_are_atomic() {
    let sim = simulator();
    let mut camera = SimCamera::new(sim.clone()).with_seed(17);
    sim.set_optics(Optics {
        width: 800,
        height: 600,
        fov_deg: 20.0,
        eyepiece_px: (350.0, 240.0),
    });
    assert_eq!((camera.info().width, camera.info().height), (800, 600));
    assert_eq!(sim.optics().fov_deg, 20.0);
    assert_eq!(sim.optics().eyepiece_px, (350.0, 240.0));
    camera.set_ranges((10.0, 100.0), (2.0, 4.0)).unwrap();
    let info = camera.info();
    assert_eq!(
        (
            info.exposure_min_ms,
            info.exposure_max_ms,
            info.gain_min,
            info.gain_max
        ),
        (10.0, 100.0, 2.0, 4.0)
    );
    for (exposure, gain, setting) in [
        (101.0, 2.0, "exposure_ms"),
        (50.0, 5.0, "gain"),
        (f64::NAN, 2.0, "exposure_ms"),
    ] {
        let error = camera
            .apply(&Settings {
                exposure_ms: Some(exposure),
                gain: Some(gain),
                ..Settings::default()
            })
            .unwrap_err();
        assert_eq!(error.setting, setting);
        assert!(error.to_string().contains("range"));
        assert_eq!(camera.frames_taken(), 0);
    }
    for invalid in [(0.0, 10.0), (10.0, 1.0), (1.0, f64::INFINITY)] {
        assert!(camera.set_ranges(invalid, (2.0, 4.0)).is_err());
        assert!(camera.set_ranges((10.0, 100.0), invalid).is_err());
        assert_eq!(camera.info(), info);
    }
    // Automatic defaults respect the newly declared ranges.
    let auto = camera.next_frame().await.unwrap();
    assert_eq!((auto.exposure_ms, auto.gain), (100.0, 2.0));
    camera
        .apply(&Settings {
            exposure_ms: Some(50.0),
            gain: Some(3.0),
            ..Settings::default()
        })
        .unwrap();
    assert!(camera.set_ranges((60.0, 100.0), (2.0, 4.0)).is_err());
    assert_eq!(camera.info(), info);
    let fixed = camera.next_frame().await.unwrap();
    assert_eq!(
        (fixed.width, fixed.height, fixed.exposure_ms, fixed.gain),
        (800, 600, 50.0, 3.0)
    );
    assert_eq!(camera.frames_taken(), 2);
}

#[tokio::test(start_paused = true)]
async fn catalog_scene_replays_and_changes_with_time_site_direction_and_exposure() {
    let sim = simulator();
    let first_camera = SimCamera::new(sim.clone());
    let second_camera = SimCamera::new(sim.clone());
    // Concurrent cameras see identical direction, time, exposure and frame id.
    let mut a = first_camera;
    let mut b = second_camera;
    let (first, replay) = tokio::join!(a.next_frame(), b.next_frame());
    assert_eq!(first.unwrap(), replay.unwrap());
    let initial = sim.field_stars(200.0, 1.0);
    assert!(initial.len() >= 4);
    assert!(initial
        .windows(2)
        .all(|p| p[0].1.magnitude <= p[1].1.magnitude));
    let short = sim.field_stars(20.0, 1.0);
    assert!(short.len() < initial.len());
    assert!(short
        .iter()
        .all(|star| initial.iter().any(|s| s.0 == star.0)));
    let truth = sim.telescope().camera;
    assert_eq!((truth.alt, truth.az), (45.0, 180.0));
    tokio::time::advance(std::time::Duration::from_secs(3600)).await;
    assert_ne!(initial, sim.field_stars(200.0, 1.0));
    assert_ne!(truth.ra, sim.telescope().camera.ra);
    sim.push_to(40.0, 200.0);
    assert_ne!(initial, sim.field_stars(200.0, 1.0));
    let other_site = Simulator::new(
        DB.clone(),
        Site {
            lat_deg: -33.0,
            lon_deg: 151.0,
        },
        VirtualClock::new(1_768_528_800.0),
    );
    assert_ne!(initial, other_site.field_stars(200.0, 1.0));
    // The settable eyepiece offset is part of the actual ray geometry too.
    sim.set_optics(Optics {
        eyepiece_px: (728.0, 544.0),
        ..Optics::default()
    });
    let centred = sim.telescope();
    assert!((centred.camera.ra - centred.eyepiece.ra).abs() < 1e-12);
    assert!((centred.camera.dec - centred.eyepiece.dec).abs() < 1e-12);
}

#[tokio::test(start_paused = true)]
async fn default_exposure_has_the_planned_average_star_count() {
    let sim = simulator();
    let mut camera = SimCamera::new(sim.clone());
    let mut counts = Vec::new();
    // Twelve separated fields around the sky; do not force every field to have
    // the same density. The plan's oracle is 25–40 stars in an average field.
    for alt in [30.0, 60.0] {
        for az in [0.0, 60.0, 120.0, 180.0, 240.0, 300.0] {
            sim.push_to(alt, az);
            let frame = camera.next_frame().await.unwrap();
            counts.push(detect::detect(&frame, &DetectSettings::cedar()).len());
        }
    }
    let average = counts.iter().sum::<usize>() as f64 / counts.len() as f64;
    assert!(
        (25.0..=40.0).contains(&average),
        "average {average}; counts {counts:?}"
    );
}
