//! Oracles are the story's specified mechanics, analytic constant-acceleration
//! kinematics and independent horizontal-vector geometry. All angles are degrees.
use scope_node::mount::{Axis, Mount};
use simulator::{
    mount::{MountConfig, SimMount},
    Simulator, Site, VirtualClock,
};
use std::{sync::Arc, time::Duration};

const COUNTS_PER_DEGREE: f64 = 4500.0; // 1,620,000 / 360, research note §1.
fn counts(degrees: f64) -> i32 {
    (degrees * COUNTS_PER_DEGREE).round() as i32
}
fn clock() -> VirtualClock {
    VirtualClock::new(1_768_528_800.0)
}
fn ideal() -> MountConfig {
    MountConfig {
        base_tilt_deg: 0.0,
        zero_points_deg: [0.0; 2],
        backlash_deg: 0.0,
        ..MountConfig::default()
    }
}
async fn advance(seconds: f64) {
    tokio::time::advance(Duration::from_secs_f64(seconds)).await;
}
async fn arrive(mount: &impl Mount, axis: Axis, degrees: f64) {
    mount.goto(axis, counts(degrees)).await;
    advance(120.0).await;
    let state = mount.state(axis).await;
    assert!(!state.moving, "{state:?}");
    assert!((state.position - counts(degrees)).abs() <= 1);
}
fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{actual} != {expected} +/- {tolerance}"
    );
}
fn turn(from: f64, to: f64) -> f64 {
    (to - from + 180.0).rem_euclid(360.0) - 180.0
}

#[tokio::test(start_paused = true)]
async fn both_axes_honour_capabilities_counts_signed_rates_and_stop_through_the_port() {
    let mount = SimMount::new(clock());
    let port: &dyn Mount = &mount;
    for axis in [Axis::Azimuth, Axis::Altitude] {
        let info = port.axes()[axis.index()];
        assert_eq!(info.counts_per_revolution, 1_620_000);
        assert_eq!(info.top_rate_deg_per_s, 3.4);
        assert!(info.can_goto);
        for target in [-12.0, 0.0, 10.0] {
            arrive(&mount, axis, target).await;
        }
        for requested in [-3.4, -0.01, 0.01, 1.7, 3.4] {
            port.rate(axis, requested).await;
            advance(3.0).await;
            let start = port.state(axis).await;
            assert!(start.moving);
            near(start.rate_deg_per_s, requested, requested.abs() * 0.001);
            advance(10.0).await;
            let end = port.state(axis).await;
            near(
                f64::from(end.position - start.position) / COUNTS_PER_DEGREE / 10.0,
                requested,
                requested.abs() * 0.001,
            );
        }
    }
    port.stop_all().await;
    advance(2.0).await;
    for axis in [Axis::Azimuth, Axis::Altitude] {
        let stopped = port.state(axis).await;
        assert!(!stopped.moving);
        assert_eq!(stopped.rate_deg_per_s, 0.0);
        advance(5.0).await;
        assert_eq!(port.state(axis).await, stopped);
    }
}

#[tokio::test(start_paused = true)]
async fn backlash_changes_truth_by_point_three_at_the_same_axis_reading() {
    for axis in [Axis::Azimuth, Axis::Altitude] {
        let mount = SimMount::with_config(
            clock(),
            MountConfig {
                backlash_deg: 0.3,
                ..ideal()
            },
        );
        arrive(&mount, axis, -1.0).await;
        arrive(&mount, axis, 0.0).await;
        let upward = mount.true_direction();
        assert_eq!(mount.state(axis).await.position, 0);
        arrive(&mount, axis, 1.0).await;
        arrive(&mount, axis, 0.0).await;
        let downward = mount.true_direction();
        assert_eq!(mount.state(axis).await.position, 0);
        let difference = match axis {
            Axis::Azimuth => turn(upward.az, downward.az),
            Axis::Altitude => downward.alt - upward.alt,
        };
        near(difference, 0.3, 0.01);
    }
}

#[tokio::test(start_paused = true)]
async fn ninety_degree_trapezoid_arrives_in_twenty_seven_point_six_seconds() {
    for axis in [Axis::Azimuth, Axis::Altitude] {
        let mount = SimMount::with_config(clock(), ideal());
        mount.goto(axis, counts(90.0)).await;
        let mut elapsed = 0.0;
        loop {
            let state = mount.state(axis).await;
            assert!(state.rate_deg_per_s.abs() <= 3.4);
            if !state.moving {
                break;
            }
            advance(0.01).await;
            elapsed += 0.01;
            assert!(elapsed < 28.0, "slew never stopped");
        }
        // Integral of accelerate/cruise/brake: d/v + v/a.
        near(elapsed, 90.0 / 3.4 + 3.4 / 3.0, 0.01);
        near(elapsed, 27.6, 0.1);
        assert_eq!(mount.state(axis).await.position, counts(90.0));
    }
}

#[tokio::test(start_paused = true)]
async fn stop_decelerates_at_three_degrees_per_second_squared() {
    let mount = SimMount::with_config(clock(), ideal());
    for axis in [Axis::Azimuth, Axis::Altitude] {
        mount.rate(axis, 3.4).await;
    }
    advance(2.0).await;
    let before = mount.state(Axis::Azimuth).await;
    assert_eq!(before.rate_deg_per_s, 3.4);
    mount.stop_all().await;
    for elapsed in 1..=11 {
        advance(0.1).await;
        let state = mount.state(Axis::Azimuth).await;
        assert!(state.moving);
        near(
            state.rate_deg_per_s,
            3.4 - 3.0 * f64::from(elapsed) / 10.0,
            1e-9,
        );
    }
    advance(0.04).await;
    for axis in [Axis::Azimuth, Axis::Altitude] {
        let state = mount.state(axis).await;
        assert!(!state.moving); // 1.14 s, within 1.13 +/- 0.05 s.
        assert_eq!(state.rate_deg_per_s, 0.0);
        near(
            f64::from(state.position - before.position) / COUNTS_PER_DEGREE,
            3.4 * 3.4 / 6.0,
            1.0 / COUNTS_PER_DEGREE,
        );
    }
}

#[tokio::test(start_paused = true)]
async fn tilted_frame_gives_two_degrees_between_opposite_azimuths() {
    let mount = SimMount::with_config(
        clock(),
        MountConfig {
            base_tilt_deg: 1.0,
            ..ideal()
        },
    );
    arrive(&mount, Axis::Altitude, 45.0).await;
    let north = mount.true_direction();
    arrive(&mount, Axis::Azimuth, 180.0).await;
    let south = mount.true_direction();
    near((north.alt - south.alt).abs(), 2.0, 0.01);
    // Exact rotation also has a measurable sideways component at azimuth 90°.
    arrive(&mount, Axis::Azimuth, 90.0).await;
    let east = mount.true_direction();
    near(
        east.alt,
        (45_f64.to_radians().sin() * 1_f64.to_radians().cos())
            .asin()
            .to_degrees(),
        1e-8,
    );
    assert!(turn(90.0, east.az).abs() > 0.9);
}

#[tokio::test(start_paused = true)]
async fn default_base_tilt_and_zero_points_are_present() {
    let config = MountConfig::default();
    assert_eq!(config.base_tilt_deg, 1.0);
    assert_eq!(config.backlash_deg, 0.3);
    assert!(config.zero_points_deg.iter().all(|angle| *angle != 0.0));
    let mount = SimMount::new(clock());
    let truth = mount.true_direction();
    assert!(truth.az.abs() > 1.0 && truth.alt.abs() > 0.1);
    assert_eq!(mount.state(Axis::Azimuth).await.position, 0);
    assert_eq!(mount.state(Axis::Altitude).await.position, 0);
}

#[tokio::test(start_paused = true)]
async fn hand_push_changes_only_true_direction() {
    let mount = SimMount::new(clock());
    let before = mount.true_direction();
    let positions = [
        mount.state(Axis::Azimuth).await,
        mount.state(Axis::Altitude).await,
    ];
    mount.push_by(3.0, 0.0);
    let after = mount.true_direction();
    near(turn(before.az, after.az), 3.0, 0.01);
    near(after.alt, before.alt, 0.01);
    assert_eq!(mount.state(Axis::Azimuth).await, positions[0]);
    assert_eq!(mount.state(Axis::Altitude).await, positions[1]);
}

#[tokio::test(start_paused = true)]
async fn each_move_loses_the_configured_slip_while_counts_reach_the_target() {
    for axis in [Axis::Azimuth, Axis::Altitude] {
        let mount = SimMount::with_config(clock(), ideal());
        mount.set_slip_per_move(axis, 0.5);
        arrive(&mount, axis, 10.0).await;
        let truth = mount.true_direction();
        near(
            if axis == Axis::Azimuth {
                truth.az
            } else {
                truth.alt
            },
            9.5,
            0.01,
        );
        arrive(&mount, axis, 0.0).await;
        let truth = mount.true_direction();
        near(
            if axis == Axis::Azimuth {
                turn(0.0, truth.az)
            } else {
                truth.alt
            },
            0.0,
            0.01,
        );
    }
}

#[tokio::test(start_paused = true)]
async fn power_cycle_resets_both_counts_without_changing_truth() {
    let mount = SimMount::new(clock());
    arrive(&mount, Axis::Azimuth, 30.0).await;
    arrive(&mount, Axis::Altitude, 40.0).await;
    mount.push_by(3.0, 2.0);
    let before = mount.true_direction();
    mount.power_cycle();
    for axis in [Axis::Azimuth, Axis::Altitude] {
        let state = mount.state(axis).await;
        assert_eq!(state.position, 0);
        assert!(!state.moving);
    }
    assert_eq!(mount.true_direction(), before);
    arrive(&mount, Axis::Azimuth, 10.0).await;
    assert!(turn(before.az, mount.true_direction().az).abs() > 9.0);
}

#[tokio::test(start_paused = true)]
async fn short_slews_reversals_and_rate_clamps_obey_acceleration() {
    let mount = SimMount::with_config(clock(), ideal());
    mount.goto(Axis::Azimuth, counts(0.75)).await;
    advance(0.5).await;
    near(mount.state(Axis::Azimuth).await.rate_deg_per_s, 1.5, 1e-9);
    advance(0.5).await;
    assert!(!mount.state(Axis::Azimuth).await.moving);
    assert_eq!(mount.state(Axis::Azimuth).await.position, counts(0.75));
    mount.rate(Axis::Azimuth, 100.0).await;
    advance(2.0).await;
    assert_eq!(mount.state(Axis::Azimuth).await.rate_deg_per_s, 3.4);
    mount.rate(Axis::Azimuth, -100.0).await;
    advance(3.4 / 3.0).await;
    near(mount.state(Axis::Azimuth).await.rate_deg_per_s, 0.0, 1e-8);
    advance(2.0).await;
    assert_eq!(mount.state(Axis::Azimuth).await.rate_deg_per_s, -3.4);
    mount.goto(Axis::Azimuth, counts(-5.0)).await;
    advance(20.0).await;
    assert_eq!(mount.state(Axis::Azimuth).await.position, counts(-5.0));
    assert!(!mount.state(Axis::Azimuth).await.moving);
    mount.goto(Axis::Azimuth, counts(-5.0)).await;
    assert!(!mount.state(Axis::Azimuth).await.moving);
    mount.rate(Axis::Azimuth, 0.0).await;
    mount.stop_all().await;
    assert!(!mount.state(Axis::Azimuth).await.moving);
}

fn simulator() -> Simulator {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../reference-solutions/cedar-solve/tetra3/data/default_database.npz");
    Simulator::new(
        Arc::new(pattern_database::load_from_path(&path).unwrap()),
        Site::default(),
        clock(),
    )
}

#[tokio::test(start_paused = true)]
async fn connected_mount_drives_telescope_camera_and_existing_push_controls() {
    use scope_node::Camera;
    use simulator::SimCamera;
    let sim = simulator();
    let mount = SimMount::new(sim.clock().clone());
    sim.connect_mount(mount.clone());
    let truth = mount.true_direction();
    let eyepiece = sim.telescope().eyepiece;
    near(eyepiece.alt, truth.alt, 1e-8);
    near(turn(eyepiece.az, truth.az), 0.0, 1e-8);
    let states = [
        mount.state(Axis::Azimuth).await,
        mount.state(Axis::Altitude).await,
    ];
    let camera = sim.telescope().camera;
    sim.push_by(3.0, 2.0); // Existing API remains camera-centred.
    near(turn(camera.az, sim.telescope().camera.az), 3.0, 0.01);
    near(sim.telescope().camera.alt - camera.alt, 2.0, 0.01);
    sim.push_to(50.0, 120.0);
    near(sim.telescope().camera.alt, 50.0, 1e-8);
    near(sim.telescope().camera.az, 120.0, 1e-8);
    near(
        mount.true_direction().alt,
        sim.telescope().eyepiece.alt,
        1e-8,
    );
    assert_eq!(mount.state(Axis::Azimuth).await, states[0]);
    assert_eq!(mount.state(Axis::Altitude).await, states[1]);
    let initial_stars = sim.field_stars(200.0, 1.0);
    assert!(!initial_stars.is_empty());
    let initial_frame = SimCamera::new(sim.clone()).next_frame().await.unwrap();
    arrive(&mount, Axis::Azimuth, 10.0).await;
    let end = mount.true_direction();
    near(sim.telescope().eyepiece.alt, end.alt, 1e-8);
    near(turn(sim.telescope().eyepiece.az, end.az), 0.0, 1e-8);
    assert_ne!(initial_stars, sim.field_stars(200.0, 1.0));
    let end_frame = SimCamera::new(sim.clone()).next_frame().await.unwrap();
    assert_ne!(initial_frame.pixels, end_frame.pixels);
    let aimed = sim.telescope().eyepiece;
    sim.point_eyepiece_at(aimed.ra + 1.0, aimed.dec);
    near(sim.telescope().eyepiece.ra, aimed.ra + 1.0, 1e-8);
    assert_eq!(mount.state(Axis::Azimuth).await.position, counts(10.0));
    sim.disconnect_mount();
    sim.push_to(35.0, 70.0);
    assert_eq!(sim.telescope().camera.alt, 35.0);
    assert_eq!(sim.telescope().camera.az, 70.0);
}
