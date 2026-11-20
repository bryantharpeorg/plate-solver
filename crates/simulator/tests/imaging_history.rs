//! 036 US1: live telescope truth, a live twin and worked count/trail geometry
//! are the oracles. All time comes from the injected, paused virtual clock.
use math_core::{angular_distance, UnitVector};
use pattern_database::PatternDatabase;
use scope_node::{
    imaging::{Exposure, ImagingCamera, ImagingFrame, ImagingSettings},
    mount::{Axis, Mount},
};
use simulator::{
    mount::{MountConfig, SimMount},
    ImagingTruth, Optics, SimImagingCamera, SimStar, Simulator, Site, TrueDirection, VirtualClock,
};
use std::{
    path::Path,
    sync::{Arc, LazyLock},
    time::Duration,
};

static DB: LazyLock<Arc<PatternDatabase>> = LazyLock::new(|| {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../reference-solutions/cedar-solve/tetra3/data/default_database.npz");
    Arc::new(pattern_database::load_from_path(&path).expect("bundled cedar database"))
});

fn clock() -> VirtualClock {
    VirtualClock::new(1_768_528_800.0)
}

fn ideal_mount(clock: VirtualClock) -> SimMount {
    SimMount::with_config(
        clock,
        MountConfig {
            base_tilt_deg: 0.0,
            backlash_deg: 0.0,
            zero_points_deg: [0.0; 2],
            ..MountConfig::default()
        },
    )
}

fn still_simulator() -> Simulator {
    let sim = Simulator::new(
        DB.clone(),
        Site {
            lat_deg: 40.0,
            lon_deg: -75.0,
        },
        clock(),
    );
    sim.push_to(50.0, 120.0);
    sim
}

async fn moving(az: f64, both_axes: bool) -> (Simulator, SimMount) {
    let sim = still_simulator();
    let mount = ideal_mount(sim.clock().clone());
    sim.connect_mount(mount.clone());
    sim.push_to(50.0, az);
    // Rate errors affect only the commands that follow them.
    mount.set_rate_error(0.25);
    mount.rate(Axis::Azimuth, 0.02).await;
    if both_axes {
        mount.rate(Axis::Altitude, 0.01).await;
    }
    tokio::time::sleep(Duration::from_secs(5)).await;
    (sim, mount)
}

fn separation_arcsec(a: (f64, f64), b: (f64, f64)) -> f64 {
    let vector = |(ra, dec): (f64, f64)| UnitVector::from_radec(ra.to_radians(), dec.to_radians());
    angular_distance(vector(a), vector(b)).to_degrees() * 3600.0
}

fn centre(truth: ImagingTruth) -> (f64, f64) {
    (truth.ra_deg, truth.dec_deg)
}

fn angle_difference(a: f64, b: f64) -> f64 {
    ((a - b + 180.0).rem_euclid(360.0) - 180.0).abs()
}

struct Live {
    time: f64,
    truth: ImagingTruth,
    eyepiece: TrueDirection,
}

fn live(sim: &Simulator) -> Live {
    Live {
        time: sim.clock().now(),
        truth: sim.imaging_truth(),
        eyepiece: sim.telescope().eyepiece,
    }
}

fn assert_live(actual: ImagingTruth, expected: &Live) {
    let distance = separation_arcsec(
        centre(actual),
        (expected.eyepiece.ra, expected.eyepiece.dec),
    );
    assert!(
        distance <= 0.02,
        "centre differs from live eyepiece by {distance} arcsec"
    );
    let angle = angle_difference(
        actual.up_east_of_north_deg,
        expected.truth.up_east_of_north_deg,
    );
    assert!(
        angle <= 0.001,
        "angle differs from live truth by {angle} degrees"
    );
}

/// Weighted position and second moments over the trail-sized square specified
/// by US1-S1. Sample positions are their centres, and background is 170 counts:
/// 160 offset + 2 seconds * 5 sky counts/second (030's worked constants).
fn trail_moments(frame: &ImagingFrame, midpoint: (f64, f64), d: f64) -> ((f64, f64), f64) {
    let width = frame.width as usize;
    let values: Vec<f64> = frame
        .samples
        .as_chunks::<2>()
        .0
        .iter()
        .map(|v| f64::from(u16::from_le_bytes(*v)))
        .collect();
    let window = |index: usize| {
        let x = (index % width) as f64 + 0.5;
        let y = (index / width) as f64 + 0.5;
        ((x - midpoint.0).abs() <= d / 2.0 + 10.0 && (y - midpoint.1).abs() <= d / 2.0 + 10.0)
            .then_some((x, y))
    };
    let (mut total, mut xsum, mut ysum) = (0.0, 0.0, 0.0);
    for (index, value) in values.iter().enumerate() {
        if let Some((x, y)) = window(index) {
            let weight = value - 170.0;
            total += weight;
            xsum += weight * x;
            ysum += weight * y;
        }
    }
    assert!(total > 0.0, "no star above the background");
    let mean = (xsum / total, ysum / total);
    let (mut xx, mut xy, mut yy) = (0.0, 0.0, 0.0);
    for (index, value) in values.iter().enumerate() {
        if let Some((x, y)) = window(index) {
            let weight = value - 170.0;
            let (dx, dy) = (x - mean.0, y - mean.1);
            xx += weight * dx * dx;
            xy += weight * dx * dy;
            yy += weight * dy * dy;
        }
    }
    // Eigenvalue difference of the symmetric second-moment matrix.
    let difference = ((xx - yy).powi(2) + 4.0 * xy * xy).sqrt() / total;
    (mean, difference)
}

#[tokio::test(start_paused = true)]
async fn exposure_history_matches_live_eyepiece_and_the_rendered_trail() {
    let (sim, _mount) = moving(120.0, false).await;
    let initial = sim.imaging_truth();
    let camera = SimImagingCamera::new(sim.clone())
        .with_noise(false)
        .with_stars(vec![SimStar {
            ra_deg: initial.ra_deg,
            dec_deg: initial.dec_deg,
            magnitude: 8.0,
            colour: [1.0; 3],
        }]);
    camera
        .apply(&ImagingSettings {
            gain: Some(0.0),
            ..ImagingSettings::default()
        })
        .await
        .unwrap();
    let shot = tokio::spawn(async move {
        camera
            .expose(&Exposure {
                exposure_ms: 2000.0,
                purpose: "frame".into(),
            })
            .await
            .unwrap()
    });
    // Poll the exposure at this instant, without advancing between its start
    // and the first independent live readings.
    tokio::task::yield_now().await;
    let first = live(&sim);
    tokio::time::advance(Duration::from_secs(1)).await;
    let middle = live(&sim);
    tokio::time::advance(Duration::from_secs(1)).await;
    let last = live(&sim);
    let frame = shot.await.unwrap();
    assert_eq!(
        frame.start_s,
        first.time - sim.clock().start_unix_s() + 1000.0
    );
    // Worked rate: 0.02 degrees/second * 1.25 * 2 seconds = 0.0500 degrees.
    let rise = (last.eyepiece.az - first.eyepiece.az).rem_euclid(360.0);
    assert!(
        (rise - 0.0500).abs() <= 0.0005,
        "azimuth rose {rise} degrees"
    );
    tokio::time::advance(Duration::from_secs(30)).await;
    let answers = [&first, &middle, &last].map(|reading| {
        let answer = sim.imaging_truth_at(reading.time);
        assert_live(answer, reading);
        answer
    });
    let separation = separation_arcsec(centre(answers[0]), centre(answers[2]));
    assert!(separation > 60.0, "centre moved only {separation} arcsec");
    let d = separation / answers[2].arcsec_per_px;
    assert!(d > 20.0, "trail is only {d} pixels long");
    // Position angle from the last centre toward the first, east of north.
    let delta_ra = (answers[0].ra_deg - answers[2].ra_deg).to_radians();
    let dec1 = answers[0].dec_deg.to_radians();
    let dec3 = answers[2].dec_deg.to_radians();
    let a = (delta_ra.sin() * dec1.cos())
        .atan2(dec3.cos() * dec1.sin() - dec3.sin() * dec1.cos() * delta_ra.cos());
    let angle = a - answers[2].up_east_of_north_deg.to_radians();
    let end = (618.0 - d * angle.sin(), 411.0 - d * angle.cos());
    let midpoint = ((618.0 + end.0) / 2.0, (411.0 + end.1) / 2.0);
    let (mean, spread) = trail_moments(&frame, midpoint, d);
    let expected = d * d / 12.0;
    assert!(
        (spread - expected).abs() <= 0.10 * expected,
        "eigenvalue difference {spread} vs trail variance {expected}"
    );
    assert!(
        (mean.0 - midpoint.0).hypot(mean.1 - midpoint.1) <= 1.0,
        "mean {mean:?} vs midpoint {midpoint:?}"
    );
}

async fn interpolation_matches_twin(az: f64, wraps: bool) {
    let ((sparse, _), (twin, _)) = tokio::join!(moving(az, true), moving(az, true));
    assert_eq!(
        sparse.clock().now(),
        twin.clock().now(),
        "twins must share their epoch"
    );
    let time = sparse.clock().now();
    let first = sparse.imaging_truth();
    let camera_az0 = sparse.telescope().camera.az;
    let twin_first = live(&twin);
    assert_live(first, &twin_first);
    tokio::time::advance(Duration::from_secs(1)).await;
    let twin_middle = live(&twin);
    tokio::time::advance(Duration::from_secs(1)).await;
    let last = sparse.imaging_truth();
    let camera_az2 = sparse.telescope().camera.az;
    let twin_last = live(&twin);
    assert_live(last, &twin_last);
    assert_eq!(twin_middle.time, time + 1.0);
    assert_live(sparse.imaging_truth_at(time + 1.0), &twin_middle);
    if wraps {
        assert!(
            camera_az0 > 359.0 && camera_az2 < 1.0,
            "camera azimuth did not cross zero: {camera_az0} -> {camera_az2}"
        );
    }
}

#[tokio::test(start_paused = true)]
async fn between_samples_matches_a_live_twin_with_both_axes_turning() {
    interpolation_matches_twin(120.0, false).await;
}

#[tokio::test(start_paused = true)]
async fn between_samples_takes_azimuth_the_short_way_across_zero() {
    interpolation_matches_twin(359.85, true).await;
}

/// 030's unchanged directions_at is the fallback oracle, in degrees.
fn assert_fallback(sim: &Simulator, time: f64) {
    let expected = sim.directions_at(time).eyepiece;
    let actual = sim.imaging_truth_at(time);
    assert!(
        angle_difference(actual.ra_deg, expected.ra) <= 1e-9,
        "fallback RA {} vs {}",
        actual.ra_deg,
        expected.ra
    );
    assert!(
        (actual.dec_deg - expected.dec).abs() <= 1e-9,
        "fallback Dec {} vs {}",
        actual.dec_deg,
        expected.dec
    );
}

#[derive(Clone, Copy, Debug)]
enum Restart {
    PushTo,
    PushBy,
    Optics,
    Connect,
    Eyepiece,
}

async fn restart_discards_history(restart: Restart) {
    // Every restart gets a fresh US1-S1 simulator, after ten seconds turning.
    let (sim, _) = moving(120.0, false).await;
    tokio::time::advance(Duration::from_secs(5)).await;
    let before = live(&sim);
    tokio::time::advance(Duration::from_secs(1)).await;
    match restart {
        Restart::PushTo => sim.push_to(40.0, 200.0),
        Restart::PushBy => sim.push_by(80.0, -10.0),
        Restart::Optics => sim.set_optics(Optics {
            eyepiece_px: (600.0, 500.0),
            ..Optics::default()
        }),
        Restart::Connect => sim.connect_mount(ideal_mount(sim.clock().clone())),
        Restart::Eyepiece => sim.point_eyepiece_at(83.81867, -5.38967),
    }
    assert_fallback(&sim, before.time);
    tokio::time::advance(Duration::from_secs(1)).await;
    sim.imaging_truth();
    tokio::time::advance(Duration::from_secs(1)).await;
    assert_fallback(&sim, before.time);
}

#[tokio::test(start_paused = true)]
async fn simulator_push_to_starts_the_memory_again() {
    restart_discards_history(Restart::PushTo).await;
}

#[tokio::test(start_paused = true)]
async fn simulator_push_by_starts_the_memory_again() {
    restart_discards_history(Restart::PushBy).await;
}

#[tokio::test(start_paused = true)]
async fn setting_optics_starts_the_memory_again() {
    restart_discards_history(Restart::Optics).await;
}

#[tokio::test(start_paused = true)]
async fn connecting_a_mount_starts_the_memory_again() {
    restart_discards_history(Restart::Connect).await;
}

#[tokio::test(start_paused = true)]
async fn pointing_the_eyepiece_starts_the_memory_again() {
    restart_discards_history(Restart::Eyepiece).await;
}

#[tokio::test(start_paused = true)]
async fn no_mount_keeps_030s_past_truth_after_many_reads() {
    let sim = still_simulator();
    for _ in 0..20 {
        tokio::time::advance(Duration::from_secs(1)).await;
        sim.imaging_truth();
    }
    assert_fallback(&sim, sim.clock().now() - 7.5);
}

async fn slow_axis(mount: &SimMount, axis: Axis) -> Vec<f64> {
    mount.rate(axis, 0.0001).await;
    let mut readings = Vec::new();
    for _ in 0..200 {
        tokio::time::advance(Duration::from_millis(100)).await;
        let direction = mount.true_direction();
        readings.push(match axis {
            Axis::Azimuth => direction.az,
            Axis::Altitude => direction.alt,
        });
    }
    readings
}

fn assert_whole_counts(readings: &[f64]) {
    // Worked example: 360 / 1,620,000 degrees = 0.8 arcsec per count.
    let step = 360.0 / 1_620_000.0;
    for reading in readings {
        let delta = reading - readings[0];
        let counts = (delta / step).round();
        assert!(
            (delta - counts * step).abs() <= 1e-9,
            "travel {delta} is not a whole count of {step} degrees"
        );
    }
    let mut distinct = readings.to_vec();
    distinct.dedup();
    // 0.36 arcsec/second * 20 seconds = 7.2 arcsec, nine counts.
    assert!(
        (8..=11).contains(&distinct.len()),
        "{} distinct values",
        distinct.len()
    );
    // Twenty 100 ms intervals: 0.72 arcsec remains under one 0.8 arcsec count.
    assert_eq!(
        readings[19], readings[0],
        "the tube moved before one whole count"
    );
}

#[tokio::test(start_paused = true)]
async fn count_steps_defaults_off_and_can_quantize_either_axis() {
    let smooth = ideal_mount(clock());
    assert!(!smooth.count_steps());
    let mut values = slow_axis(&smooth, Axis::Azimuth).await;
    values.dedup();
    assert!(values.len() > 100, "only {} smooth values", values.len());
    for axis in [Axis::Azimuth, Axis::Altitude] {
        let stepped = ideal_mount(clock());
        stepped.set_count_steps(true);
        assert!(stepped.count_steps());
        assert_whole_counts(&slow_axis(&stepped, axis).await);
    }
}

#[tokio::test(start_paused = true)]
async fn history_retains_an_hour_and_falls_back_after_expiry() {
    let (sim, _) = moving(120.0, false).await;
    let first = live(&sim);
    for _ in 0..30 {
        tokio::time::advance(Duration::from_secs(100)).await;
        sim.imaging_truth();
    }
    assert_live(sim.imaging_truth_at(first.time), &first);
    for _ in 0..6 {
        tokio::time::advance(Duration::from_secs(100)).await;
        sim.imaging_truth();
    }
    assert_live(sim.imaging_truth_at(first.time), &first);
    tokio::time::advance(Duration::from_secs(100)).await;
    sim.imaging_truth();
    assert_fallback(&sim, first.time);
}

#[tokio::test(start_paused = true)]
async fn a_push_on_the_mount_keeps_earlier_samples() {
    // The mount's push is motion, unlike the simulator's restart controls.
    for push_to in [false, true] {
        let (sim, mount) = moving(120.0, false).await;
        tokio::time::advance(Duration::from_secs(5)).await;
        let before = live(&sim);
        tokio::time::advance(Duration::from_secs(1)).await;
        if push_to {
            let direction = mount.true_direction();
            mount.push_to(direction.alt - 1.0, direction.az);
        } else {
            mount.push_by(0.0, -1.0);
        }
        let after = sim.imaging_truth();
        let remembered = sim.imaging_truth_at(before.time);
        assert_live(remembered, &before);
        assert!(separation_arcsec(centre(remembered), centre(after)) > 1000.0);
    }
}

#[tokio::test(start_paused = true)]
async fn a_second_read_at_the_same_instant_replaces_the_newest_sample() {
    let (sim, mount) = moving(120.0, false).await;
    let before = live(&sim);
    mount.push_by(0.0, -1.0);
    let after = live(&sim);
    assert_eq!(before.time, after.time);
    assert!(separation_arcsec(centre(before.truth), centre(after.truth)) > 1000.0);
    assert_live(sim.imaging_truth_at(after.time), &after);
    // Also check that the replacement is remembered after the mount moves on.
    tokio::time::advance(Duration::from_secs(1)).await;
    sim.imaging_truth();
    assert_live(sim.imaging_truth_at(after.time), &after);
}
