//! US2's scenarios, one test each. Oracles: plan.md § The simulated imaging
//! camera, the worked examples of spec.md's scenarios, the simulator's own
//! truth and sky functions, and the seeded field's own rules, with every
//! expected value computed here, not read from the camera.
use math_core::{angular_distance, UnitVector};
use pattern_database::PatternDatabase;
use scope_node::{
    imaging::{BayerPattern, Exposure, ImagingCamera, ImagingError, ImagingFrame, ImagingSettings, Roi},
    mount::{Axis, Mount},
};
use simulator::{
    imaging::{deep_field, FIELD_SEED},
    mount::SimMount,
    sky, ImagingOptics, ImagingTruth, Optics, SimImagingCamera, SimStar, Simulator, Site,
    VirtualClock,
};
use std::{
    f64::consts::PI,
    path::Path,
    sync::{Arc, LazyLock},
    time::Duration,
};

static DB: LazyLock<Arc<PatternDatabase>> = LazyLock::new(|| {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../reference-solutions/cedar-solve/tetra3/data/default_database.npz");
    Arc::new(pattern_database::load_from_path(&path).expect("bundled cedar database"))
});

/// The sim rig of spec.md § User Scenarios: the test database, 40 N 75 W, a
/// virtual clock from Unix 1768528800 and the tube pushed to alt 50, az 120.
fn rig() -> Simulator {
    let sim = Simulator::new(DB.clone(), Site::default(), VirtualClock::new(1_768_528_800.0));
    sim.push_to(50.0, 120.0);
    sim
}

/// The 16-bit samples of a frame, decoded little-endian, row by row.
fn samples(frame: &ImagingFrame) -> Vec<f64> {
    frame
        .samples
        .chunks_exact(2)
        .map(|pair| f64::from(u16::from_le_bytes([pair[0], pair[1]])))
        .collect()
}

/// The brightest sample whose centre lies within `scan` pixels of `near`.
fn brightest(values: &[f64], width: usize, near: (f64, f64), scan: f64) -> (usize, usize) {
    let height = values.len() / width;
    let bounds = |low: f64, high: f64, count: usize| {
        let lo = low.ceil().max(0.0) as usize;
        let hi = high.floor().min(count as f64 - 1.0) as usize;
        lo..=hi
    };
    let columns = bounds(near.0 - scan - 0.5, near.0 + scan - 0.5, width);
    let rows = bounds(near.1 - scan - 0.5, near.1 + scan - 0.5, height);
    let mut best = (0, 0);
    let mut best_value = f64::NEG_INFINITY;
    for row in rows {
        for column in columns.clone() {
            let value = values[row * width + column];
            if value > best_value {
                best_value = value;
                best = (column, row);
            }
        }
    }
    assert!(best_value > f64::NEG_INFINITY, "no sample within {scan} px of {near:?}");
    best
}

/// The centroid and the second moments [Sxx, Sxy, Syy] of the (2·half + 1)²
/// samples about a sample, weighted by (sample − background), as spec.md
/// § User Scenarios defines them.
fn moments(
    values: &[f64],
    width: usize,
    at: (usize, usize),
    half: usize,
    background: f64,
) -> ((f64, f64), [f64; 3]) {
    let height = values.len() / width;
    let window = |centre: usize, count: usize| {
        centre.saturating_sub(half)..=(centre + half).min(count - 1)
    };
    let mut total = 0.0;
    let mut place = [0.0; 2];
    for row in window(at.1, height) {
        for column in window(at.0, width) {
            let weight = values[row * width + column] - background;
            total += weight;
            place[0] += weight * (column as f64 + 0.5);
            place[1] += weight * (row as f64 + 0.5);
        }
    }
    assert!(total > 0.0, "nothing above the background about {at:?}");
    place[0] /= total;
    place[1] /= total;
    let mut second = [0.0; 3];
    for row in window(at.1, height) {
        for column in window(at.0, width) {
            let weight = values[row * width + column] - background;
            let dx = column as f64 + 0.5 - place[0];
            let dy = row as f64 + 0.5 - place[1];
            second[0] += weight * dx * dx;
            second[1] += weight * dx * dy;
            second[2] += weight * dy * dy;
        }
    }
    (
        (place[0], place[1]),
        [second[0] / total, second[1] / total, second[2] / total],
    )
}

/// The eigenvalues of [[Sxx, Sxy], [Sxy, Syy]], larger first, and the angle in
/// radians of the larger one's axis.
fn eigen([sxx, sxy, syy]: [f64; 3]) -> ((f64, f64), f64) {
    let half = (sxx + syy) / 2.0;
    let radius = ((sxx - syy).powi(2) / 4.0 + sxy * sxy).sqrt();
    let large = half + radius;
    ((large, half - radius), sxy.atan2(large - sxx))
}

/// The J2000 vector of a place, and its separation in degrees from another.
fn vector(ra_deg: f64, dec_deg: f64) -> UnitVector {
    UnitVector::from_radec(ra_deg.to_radians(), dec_deg.to_radians())
}
fn separation(a: (f64, f64), b: (f64, f64)) -> f64 {
    angular_distance(vector(a.0, a.1), vector(b.0, b.1)).to_degrees()
}

/// The position angle east of north from one place to another, in degrees.
fn position_angle(ra0: f64, dec0: f64, ra1: f64, dec1: f64) -> f64 {
    let (ra0, dec0, ra1, dec1) = (
        ra0.to_radians(),
        dec0.to_radians(),
        ra1.to_radians(),
        dec1.to_radians(),
    );
    let delta = ra1 - ra0;
    (delta.sin() * dec1.cos())
        .atan2(dec0.cos() * dec1.sin() - dec0.sin() * dec1.cos() * delta.cos())
        .to_degrees()
        .rem_euclid(360.0)
}

/// A star `rho_deg` from a place, toward position angle `pa_deg` east of north.
fn offset_star(
    ra_deg: f64,
    dec_deg: f64,
    rho_deg: f64,
    pa_deg: f64,
    magnitude: f64,
    colour: [f64; 3],
) -> SimStar {
    let (ra, dec, rho, pa) = (
        ra_deg.to_radians(),
        dec_deg.to_radians(),
        rho_deg.to_radians(),
        pa_deg.to_radians(),
    );
    let dec1 = (dec.sin() * rho.cos() + dec.cos() * rho.sin() * pa.cos()).asin();
    let ra1 = ra + (pa.sin() * rho.sin() * dec.cos()).atan2(rho.cos() - dec.sin() * dec1.sin());
    SimStar {
        ra_deg: ra1.to_degrees().rem_euclid(360.0),
        dec_deg: dec1.to_degrees(),
        magnitude,
        colour,
    }
}

/// A star's place in the frame whose truth is given, by the tangent-plane
/// geometry of the truth's centre, frame angle and scale.
fn place_by_truth(star: &SimStar, truth: &ImagingTruth, centre: (f64, f64)) -> (f64, f64) {
    let rho = separation((star.ra_deg, star.dec_deg), (truth.ra_deg, truth.dec_deg)) * 3600.0;
    let pa = position_angle(truth.ra_deg, truth.dec_deg, star.ra_deg, star.dec_deg);
    let distance = rho / truth.arcsec_per_px;
    let from_up = (pa - truth.up_east_of_north_deg).to_radians();
    (
        centre.0 + distance * from_up.sin(),
        centre.1 - distance * from_up.cos(),
    )
}

/// A camera on the rig with `stars`, noise off, one 20 ms frame at `gain` 0,
/// its stars placed at the truth for the middle of its own exposure.
async fn quiet_frame(sim: &Simulator, stars: Vec<SimStar>) -> ImagingFrame {
    let camera = SimImagingCamera::new(sim.clone())
        .with_noise(false)
        .with_stars(stars);
    camera
        .apply(&ImagingSettings {
            gain: Some(0.0),
            ..ImagingSettings::default()
        })
        .await
        .unwrap();
    camera
        .expose(&Exposure {
            exposure_ms: 20.0,
            purpose: "frame".into(),
        })
        .await
        .unwrap()
}

/// One star at the truth's centre for the middle of the next 20 ms frame.
fn centre_star(sim: &Simulator, magnitude: f64, colour: [f64; 3]) -> Vec<SimStar> {
    let truth = sim.imaging_truth_at(sim.clock().now() + 0.010);
    vec![SimStar {
        ra_deg: truth.ra_deg,
        dec_deg: truth.dec_deg,
        magnitude,
        colour,
    }]
}

#[tokio::test(start_paused = true)]
async fn the_description_starting_settings_and_default_optics_are_the_plans() {
    let sim = rig();
    let camera = SimImagingCamera::new(sim.clone());
    let info = camera.describe().await.unwrap();
    assert_eq!(info.model, "simulated imaging camera");
    assert_eq!(info.serial.as_deref(), Some("SIM-0001"));
    assert_eq!((info.width, info.height), (1236, 822));
    assert_eq!(info.pixel_um, 19.12);
    assert_eq!(info.bits, 16);
    assert_eq!(info.bayer, Some(BayerPattern::Rggb));
    assert_eq!(info.exposure_ms.min, 1.0);
    assert_eq!(info.exposure_ms.max, 3_600_000.0);
    assert_eq!(info.gain.unwrap().max, 300.0);
    assert_eq!(info.offset.unwrap().max, 100.0);
    assert_eq!(info.binnings, vec![1, 2]);
    assert!(info.cooler);
    // An empty apply answers the settings in force.
    let settings = camera.apply(&ImagingSettings::default()).await.unwrap();
    assert_eq!(settings.gain, Some(120.0));
    assert_eq!(settings.offset, Some(10.0));
    assert_eq!(settings.binning, Some(1));
    assert_eq!(
        settings.roi,
        Some(Roi {
            x: 0,
            y: 0,
            width: 1236,
            height: 822
        })
    );
    assert_eq!(settings.cooler_on, Some(false));
    assert_eq!(settings.set_temp_c, None);
    let optics = ImagingOptics::default();
    assert_eq!((optics.width, optics.height), (1236, 822));
    assert_eq!(optics.fov_deg, 0.75);
    assert_eq!(optics.angle_deg, 17.0);
    assert_eq!(optics.focus_error, 0.0);
    assert_eq!(optics.pixel_um, 19.12);
}

#[tokio::test(start_paused = true)]
async fn the_truth_answers_scale_centre_and_frame_angle() {
    let sim = rig();
    let t = sim.clock().now();
    assert_eq!(t, 1_768_528_800.0);
    let truth = sim.imaging_truth_at(t);
    // Worked example: 2·tan(0.375°)/1236 radians a pixel.
    let radians = 2.0 * (0.75_f64 / 2.0).to_radians().tan() / 1236.0;
    assert!((truth.arcsec_per_px - radians * 180.0 / PI * 3600.0).abs() < 1e-9);
    assert!((truth.arcsec_per_px - 2.18450).abs() <= 0.0001);
    // The centre is the eyepiece's existing truth.
    let directions = sim.directions_at(t);
    assert!((truth.ra_deg - directions.eyepiece.ra).abs() < 1e-9);
    assert!((truth.dec_deg - directions.eyepiece.dec).abs() < 1e-9);
    // With the eyepiece at the finder frame's centre and the imaging angle 0,
    // frame-up is the position angle of increasing altitude.
    sim.set_optics(Optics {
        eyepiece_px: (728.0, 544.0),
        ..Optics::default()
    });
    sim.set_imaging_optics(ImagingOptics {
        angle_deg: 0.0,
        ..ImagingOptics::default()
    });
    let flat = sim.imaging_truth_at(t);
    let site = Site::default();
    let p0 = sky::to_j2000(sky::horizontal_to_date(50.0, 120.0, site, t), sky::julian_date(t));
    let p1 = sky::to_j2000(sky::horizontal_to_date(50.01, 120.0, site, t), sky::julian_date(t));
    let (ra0, dec0) = p0.to_radec();
    let (ra1, dec1) = p1.to_radec();
    let pa = position_angle(ra0.to_degrees(), dec0.to_degrees(), ra1.to_degrees(), dec1.to_degrees());
    assert!((flat.up_east_of_north_deg - pa).abs() <= 0.01, "{} vs {pa}", flat.up_east_of_north_deg);
    // The imaging angle turns it by its own amount, east of north.
    sim.set_imaging_optics(ImagingOptics::default());
    let turned = sim.imaging_truth_at(t);
    let expected = (pa + 17.0).rem_euclid(360.0);
    let difference = (turned.up_east_of_north_deg - expected + 180.0).rem_euclid(360.0) - 180.0;
    assert!(difference.abs() <= 0.01, "{} vs {expected}", turned.up_east_of_north_deg);
}

#[tokio::test(start_paused = true)]
async fn two_stars_land_by_the_truth() {
    let sim = rig();
    let truth = sim.imaging_truth_at(sim.clock().now() + 0.010);
    let a = SimStar {
        ra_deg: truth.ra_deg,
        dec_deg: truth.dec_deg,
        magnitude: 5.0,
        colour: [1.0, 1.0, 1.0],
    };
    let b = SimStar {
        ra_deg: truth.ra_deg,
        dec_deg: truth.dec_deg + 300.0 / 3600.0,
        magnitude: 5.0,
        colour: [1.0, 1.0, 1.0],
    };
    let frame = quiet_frame(&sim, vec![a, b]).await;
    assert_eq!((frame.width, frame.height), (1236, 822));
    assert_eq!(frame.samples.len(), 2 * 1236 * 822);
    let values = samples(&frame);
    let (column, row) = brightest(&values, 1236, (618.0, 411.0), 4.0);
    let ((ax, ay), _) = moments(&values, 1236, (column, row), 12, 160.0);
    assert!((ax - 618.0).abs() <= 0.3 && (ay - 411.0).abs() <= 0.3, "({ax}, {ay})");
    // B is 300 arcsec north of A: (d·sin θ, −d·cos θ) with θ and s from the
    // truth, d = 300/s.
    let theta = truth.up_east_of_north_deg.to_radians();
    let d = 300.0 / truth.arcsec_per_px;
    let expected = (618.0 + d * theta.sin(), 411.0 - d * theta.cos());
    let (column, row) = brightest(&values, 1236, expected, 4.0);
    let ((bx, by), _) = moments(&values, 1236, (column, row), 12, 160.0);
    assert!(
        (bx - ax - d * theta.sin()).abs() <= 0.3 && (by - ay + d * theta.cos()).abs() <= 0.3,
        "({bx}, {by}) less ({ax}, {ay})"
    );
}

#[tokio::test(start_paused = true)]
async fn frames_repeat_by_seed_and_the_noise_matches_the_worked_example() {
    let sim = rig();
    let one = SimImagingCamera::new(sim.clone()).with_stars(vec![]);
    let two = SimImagingCamera::new(sim.clone()).with_stars(vec![]);
    for camera in [&one, &two] {
        camera
            .apply(&ImagingSettings {
                gain: Some(0.0),
                ..ImagingSettings::default()
            })
            .await
            .unwrap();
    }
    let take = |camera: &SimImagingCamera| {
        let camera = camera.clone();
        async move {
            camera
                .expose(&Exposure {
                    exposure_ms: 1000.0,
                    purpose: "dark".into(),
                })
                .await
                .unwrap()
        }
    };
    let first = take(&one).await;
    let second = take(&two).await;
    let third = take(&one).await;
    assert_eq!(first.samples, second.samples);
    assert_ne!(first.samples, third.samples);
    // Worked example: offset 10 gives 160; 20 electrons of sky at 0.25 a count
    // give 5; the noise is √(20 + 9) electrons, 1.346 counts, 1.377 rounded.
    let values = samples(&first);
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    assert!((mean - 165.0).abs() <= 0.05, "{mean}");
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
    let deviation = variance.sqrt();
    assert!((1.31..=1.45).contains(&deviation), "{deviation}");
}

#[tokio::test(start_paused = true)]
async fn focus_error_widens_the_star_as_the_quadrature() {
    for (focus, width, tolerance) in [(0.0, 1.20, 0.05), (3.0, 3.23, 0.12)] {
        let sim = rig();
        sim.set_imaging_optics(ImagingOptics {
            focus_error: focus,
            ..ImagingOptics::default()
        });
        let frame = quiet_frame(&sim, centre_star(&sim, 5.0, [1.0; 3])).await;
        let values = samples(&frame);
        let (column, row) = brightest(&values, 1236, (618.0, 411.0), 6.0);
        let (_, [sxx, _, syy]) = moments(&values, 1236, (column, row), 12, 160.0);
        let measured = ((sxx + syy) / 2.0).sqrt();
        assert!((measured - width).abs() <= tolerance, "{measured} vs {width}");
    }
}

#[tokio::test(start_paused = true)]
async fn the_sky_and_the_mount_smear_a_star_between_its_true_places() {
    // No mount: the sky's own rate over 2 s, about 13 px here.
    {
        let sim = rig();
        let truth = sim.imaging_truth_at(sim.clock().now() + 1.0);
        let star = SimStar {
            ra_deg: truth.ra_deg,
            dec_deg: truth.dec_deg,
            magnitude: 9.0,
            colour: [1.0; 3],
        };
        let camera = SimImagingCamera::new(sim.clone())
            .with_noise(false)
            .with_stars(vec![star]);
        camera
            .apply(&ImagingSettings {
                gain: Some(0.0),
                ..ImagingSettings::default()
            })
            .await
            .unwrap();
        let frame = camera
            .expose(&Exposure {
                exposure_ms: 2000.0,
                purpose: "frame".into(),
            })
            .await
            .unwrap();
        let values = samples(&frame);
        let (column, row) = brightest(&values, 1236, (618.0, 411.0), 10.0);
        let (_, [sxx, sxy, syy]) = moments(&values, 1236, (column, row), 20, 170.0);
        let ((large, small), _) = eigen([sxx, sxy, syy]);
        // Worked example: the sky turns 15.041 arcsec a second at the equator.
        let l = 15.041 * truth.dec_deg.to_radians().cos() * 2.0 / truth.arcsec_per_px;
        let expected = l * l / 12.0;
        assert!(
            (large - small - expected).abs() <= 0.08 * expected,
            "{large} − {small} vs {expected}"
        );
    }
    // A mount in motion smears it between its two true places.
    {
        let sim = rig();
        sim.connect_mount(SimMount::new(sim.clock().clone()));
        sim.push_to(50.0, 120.0);
        sim.mount().unwrap().rate(Axis::Azimuth, 0.03).await;
        tokio::time::advance(Duration::from_secs(60)).await;
        let truth0 = sim.imaging_truth();
        let star = SimStar {
            ra_deg: truth0.ra_deg,
            dec_deg: truth0.dec_deg,
            magnitude: 8.0,
            colour: [1.0; 3],
        };
        let camera = SimImagingCamera::new(sim.clone())
            .with_noise(false)
            .with_stars(vec![star]);
        camera
            .apply(&ImagingSettings {
                gain: Some(0.0),
                ..ImagingSettings::default()
            })
            .await
            .unwrap();
        let frame = camera
            .expose(&Exposure {
                exposure_ms: 500.0,
                purpose: "frame".into(),
            })
            .await
            .unwrap();
        let truth1 = sim.imaging_truth();
        let p0 = (618.0, 411.0);
        let p1 = place_by_truth(&star, &truth1, (618.0, 411.0));
        let distance = (p1.0 - p0.0).hypot(p1.1 - p0.1);
        assert!(distance > 10.0, "only {distance} px apart");
        let mid = ((p0.0 + p1.0) / 2.0, (p0.1 + p1.1) / 2.0);
        let values = samples(&frame);
        let (column, row) = brightest(&values, 1236, mid, 12.0);
        let ((cx, cy), [sxx, sxy, syy]) = moments(&values, 1236, (column, row), 20, 163.0);
        assert!(
            (cx - mid.0).abs() <= 0.2 && (cy - mid.1).abs() <= 0.2,
            "({cx}, {cy}) vs ({}, {})",
            mid.0,
            mid.1
        );
        let ((large, small), axis) = eigen([sxx, sxy, syy]);
        let expected = distance * distance / 12.0;
        assert!(
            (large - small - expected).abs() <= 0.10 * expected,
            "{large} − {small} vs {expected}"
        );
        let line = (p1.1 - p0.1).atan2(p1.0 - p0.0);
        let between = (axis - line).to_degrees().rem_euclid(180.0);
        assert!(between <= 5.0 || 180.0 - between <= 5.0, "{between}° off");
    }
}

#[tokio::test(start_paused = true)]
async fn binning_keeps_the_mosaic_and_the_roi_shifts_the_star() {
    let sim = rig();
    // A red star at the centre, at both binnings.
    for (binning, size, place) in [
        (1u32, (1236usize, 822usize), (618.0, 411.0)),
        (2, (618, 411), (309.0, 205.5)),
    ] {
        let camera = SimImagingCamera::new(sim.clone())
            .with_noise(false)
            .with_stars(centre_star(&sim, 5.0, [1.0, 0.0, 0.0]));
        camera
            .apply(&ImagingSettings {
                gain: Some(0.0),
                binning: Some(binning),
                ..ImagingSettings::default()
            })
            .await
            .unwrap();
        let frame = camera
            .expose(&Exposure {
                exposure_ms: 20.0,
                purpose: "frame".into(),
            })
            .await
            .unwrap();
        assert_eq!((frame.width, frame.height), size);
        assert_eq!(frame.binning, binning);
        assert_eq!(frame.bayer, Some(BayerPattern::Rggb));
        assert_eq!(
            frame.roi,
            Roi {
                x: 0,
                y: 0,
                width: 1236,
                height: 822
            }
        );
        let values = samples(&frame);
        let (column, row) = brightest(&values, size.0, place, 4.0);
        assert!(column % 2 == 0 && row % 2 == 0, "({column}, {row})");
        assert!(values[row * size.0 + column] > 1000.0, "{}, {}", values[row * size.0 + column], place.0);
        assert!(
            (column as f64 + 0.5 - place.0).abs() <= 2.0
                && (row as f64 + 0.5 - place.1).abs() <= 2.0,
            "({column}, {row}) vs {place:?}"
        );
        for r in row.saturating_sub(9)..=(row + 9).min(size.1 - 1) {
            for c in column.saturating_sub(9)..=(column + 9).min(size.0 - 1) {
                if c % 2 == 1 || r % 2 == 1 {
                    assert_eq!(values[r * size.0 + c], 160.0, "({c}, {r})");
                }
            }
        }
        if binning == 2 {
            // 822 is not a multiple of 4: row 410 is the mean of the two
            // samples of sensor row 820 alone.
            for c in 0..size.0 {
                assert_eq!(values[410 * size.0 + c], 160.0, "({c}, 410)");
            }
        }
    }
    // The star 409.5 px below the centre, on sensor row 820: each red sample
    // enters exactly one mean, and each mean is of two.
    {
        let mut frames = Vec::new();
        for binning in [1u32, 2] {
            let truth = sim.imaging_truth_at(sim.clock().now() + 0.010);
            let rho = 409.5 * truth.arcsec_per_px / 3600.0;
            let star = offset_star(
                truth.ra_deg,
                truth.dec_deg,
                rho,
                truth.up_east_of_north_deg + 180.0,
                5.0,
                [1.0, 0.0, 0.0],
            );
            let camera = SimImagingCamera::new(sim.clone())
                .with_noise(false)
                .with_stars(vec![star]);
            camera
                .apply(&ImagingSettings {
                    gain: Some(0.0),
                    binning: Some(binning),
                    ..ImagingSettings::default()
                })
                .await
                .unwrap();
            frames.push(
                camera
                    .expose(&Exposure {
                        exposure_ms: 20.0,
                        purpose: "frame".into(),
                    })
                    .await
                    .unwrap(),
            );
        }
        let full = samples(&frames[0]);
        let binned = samples(&frames[1]);
        let a: f64 = (0..1236)
            .step_by(2)
            .map(|c| full[820 * 1236 + c] - 160.0)
            .sum();
        assert!(a > 10_000.0, "{a}");
        let b: f64 = (0..618)
            .step_by(2)
            .map(|c| binned[410 * 618 + c] - 160.0)
            .sum();
        assert!((b - a / 2.0).abs() <= 0.05 * a / 2.0, "{b} vs {}", a / 2.0);
        for c in (1..618).step_by(2) {
            assert_eq!(binned[410 * 618 + c], 160.0, "({c}, 410)");
        }
    }
    // A white star at the centre and a region of interest.
    {
        let camera = SimImagingCamera::new(sim.clone())
            .with_noise(false)
            .with_stars(centre_star(&sim, 5.0, [1.0; 3]));
        camera
            .apply(&ImagingSettings {
                gain: Some(0.0),
                roi: Some(Roi {
                    x: 400,
                    y: 200,
                    width: 400,
                    height: 400,
                }),
                ..ImagingSettings::default()
            })
            .await
            .unwrap();
        let frame = camera
            .expose(&Exposure {
                exposure_ms: 20.0,
                purpose: "frame".into(),
            })
            .await
            .unwrap();
        assert_eq!((frame.width, frame.height), (400, 400));
        assert_eq!(
            frame.roi,
            Roi {
                x: 400,
                y: 200,
                width: 400,
                height: 400
            }
        );
        let values = samples(&frame);
        let (column, row) = brightest(&values, 400, (218.0, 211.0), 4.0);
        let ((cx, cy), _) = moments(&values, 400, (column, row), 12, 160.0);
        assert!((cx - 218.0).abs() <= 0.3 && (cy - 211.0).abs() <= 0.3, "({cx}, {cy})");
    }
}

#[test]
fn deep_field_is_seeded_dense_sorted_and_stable() {
    let field = deep_field(FIELD_SEED, 83.8, -5.4, 2.0);
    assert_eq!(field, deep_field(FIELD_SEED, 83.8, -5.4, 2.0));
    // Worked example: 320 a square degree over 12.566 square degrees, 4021.
    assert!((3780..=4262).contains(&field.len()), "{}", field.len());
    let bright = field.iter().filter(|s| s.magnitude < 14.0).count();
    assert!((768..=977).contains(&bright), "{bright}");
    for pair in field.windows(2) {
        assert!(pair[0].magnitude < pair[1].magnitude, "not rising, or shared");
    }
    for star in &field {
        assert!((8.0..=16.0).contains(&star.magnitude), "{}", star.magnitude);
        assert!(
            separation((star.ra_deg, star.dec_deg), (83.8, -5.4)) <= 2.0,
            "{} {}",
            star.ra_deg,
            star.dec_deg
        );
    }
    // A star of one field is the same star of the field that also holds it.
    let near = deep_field(FIELD_SEED, 83.8, -5.4, 0.3);
    let shifted = deep_field(FIELD_SEED, 84.0, -5.4, 0.3);
    let shared: Vec<&SimStar> = near
        .iter()
        .filter(|s| separation((s.ra_deg, s.dec_deg), (84.0, -5.4)) <= 0.3)
        .collect();
    assert!(!shared.is_empty(), "the fields do not overlap");
    for star in shared {
        assert!(shifted.contains(star), "{star:?}");
    }
}

#[tokio::test(start_paused = true)]
async fn the_default_field_shows_a_star_above_the_sky() {
    let sim = rig();
    let truth = sim.imaging_truth_at(sim.clock().now() + 0.5);
    let camera = SimImagingCamera::new(sim.clone()).with_noise(false);
    let frame = camera
        .expose(&Exposure {
            exposure_ms: 1000.0,
            purpose: "frame".into(),
        })
        .await
        .unwrap();
    let field = deep_field(FIELD_SEED, truth.ra_deg, truth.dec_deg, 0.2);
    let star = field.first().expect("stars within 0.2 degrees");
    let (x, y) = place_by_truth(star, &truth, (618.0, 411.0));
    let values = samples(&frame);
    let columns = (x - 10.5).ceil().max(0.0) as usize..=(x + 9.5).floor().min(1235.0) as usize;
    let rows = (y - 10.5).ceil().max(0.0) as usize..=(y + 9.5).floor().min(821.0) as usize;
    let found = rows
        .flat_map(|r| columns.clone().map(move |c| values[r * 1236 + c]))
        .any(|v| v >= 480.0);
    assert!(found, "nothing 300 above 180 within 10 of ({x}, {y})");
}

#[tokio::test(start_paused = true)]
async fn the_camera_honours_the_port_on_the_simulator_clock() {
    let sim = rig();
    let camera = Arc::new(SimImagingCamera::new(sim.clone()).with_noise(false));
    // start_s is the simulator clock's node_now() at the call.
    let before = sim.clock().node_now();
    let frame = camera
        .expose(&Exposure {
            exposure_ms: 100.0,
            purpose: "frame".into(),
        })
        .await
        .unwrap();
    assert_eq!(frame.start_s, before);
    assert_eq!(frame.end_s, frame.start_s + 0.1);
    // The port's refusals, with this camera's own ranges.
    assert_eq!(
        camera
            .apply(&ImagingSettings {
                gain: Some(301.0),
                ..ImagingSettings::default()
            })
            .await,
        Err(ImagingError::Invalid(
            "gain 301 is outside the camera's range 0–300".into()
        ))
    );
    assert_eq!(
        camera
            .apply(&ImagingSettings {
                binning: Some(3),
                ..ImagingSettings::default()
            })
            .await,
        Err(ImagingError::Invalid(
            "binning 3 is not one of the camera's binnings 1, 2".into()
        ))
    );
    // A second expose is Busy, an apply that names a setting is Busy while an
    // empty one answers the settings in force, and abort ends the exposure
    // with Aborted at once. An aborted exposure takes no frame number.
    let first = {
        let camera = camera.clone();
        tokio::spawn(async move {
            camera
                .expose(&Exposure {
                    exposure_ms: 10_000.0,
                    purpose: "frame".into(),
                })
                .await
        })
    };
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert_eq!(
        camera
            .apply(&ImagingSettings {
                gain: Some(150.0),
                ..ImagingSettings::default()
            })
            .await,
        Err(ImagingError::Busy)
    );
    let in_force = camera.apply(&ImagingSettings::default()).await.unwrap();
    assert_eq!(in_force.gain, Some(120.0));
    assert_eq!(
        camera
            .expose(&Exposure {
                exposure_ms: 100.0,
                purpose: "frame".into(),
            })
            .await,
        Err(ImagingError::Busy)
    );
    let at_abort = sim.clock().node_now();
    camera.abort().await.unwrap();
    assert_eq!(first.await.unwrap(), Err(ImagingError::Aborted));
    assert_eq!(sim.clock().node_now(), at_abort);
    assert_eq!(camera.frames_taken(), 1);
    let frame = camera
        .expose(&Exposure {
            exposure_ms: 100.0,
            purpose: "frame".into(),
        })
        .await
        .unwrap();
    assert_eq!((frame.width, frame.height), (1236, 822));
    assert_eq!(camera.frames_taken(), 2);
}
