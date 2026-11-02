//! Oracles: US2's specified travel, reversible zero-backlash mechanics, and
//! 014's 0.3° full backlash width. Time is injected through the virtual clock.
use scope_node::mount::{Axis, Mount};
use simulator::{
    mount::{MountConfig, SimMount},
    VirtualClock,
};
use std::time::Duration;

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

fn coordinate(mount: &SimMount, axis: Axis) -> f64 {
    let direction = mount.true_direction();
    match axis {
        Axis::Azimuth => (direction.az + 180.0).rem_euclid(360.0) - 180.0,
        Axis::Altitude => direction.alt,
    }
}

async fn arrive(mount: &SimMount, axis: Axis, degrees: f64) {
    // The simulator reports 1,620,000 counts per turn: 4,500 per degree.
    let counts = (degrees * 4500.0).round() as i32;
    mount.goto(axis, counts).await;
    tokio::time::advance(Duration::from_secs(3)).await;
    let state = mount.state(axis).await;
    assert!(!state.moving);
    assert_eq!(state.position, counts);
}

#[tokio::test(start_paused = true)]
async fn one_percent_fast_turns_both_axes_one_point_zero_one_degrees() {
    for axis in [Axis::Azimuth, Axis::Altitude] {
        let mount = SimMount::with_config(clock(), ideal());
        mount.set_rate_error(0.01);
        assert_eq!(mount.rate_error(), 0.01);
        let start = coordinate(&mount, axis);
        mount.rate(axis, 0.01).await;
        tokio::time::advance(Duration::from_secs(100)).await;
        let travel = coordinate(&mount, axis) - start;
        // 0.01°/s × 100 s × 1.01 = 1.01°. The acceleration ramp is
        // shorter than 0.004 s, comfortably inside US2-S1's 0.001° bound.
        assert!((travel - 1.01).abs() <= 0.001, "{axis:?}: {travel}");
        assert!((mount.state(axis).await.rate_deg_per_s - 0.0101).abs() < 1e-12);
    }
}

#[tokio::test(start_paused = true)]
async fn zero_backlash_returns_to_the_start_after_a_one_degree_reversal() {
    for axis in [Axis::Azimuth, Axis::Altitude] {
        let mount = SimMount::with_config(clock(), ideal());
        let start = coordinate(&mount, axis);
        arrive(&mount, axis, 1.0).await;
        assert!((coordinate(&mount, axis) - start - 1.0).abs() <= 1.0 / 3600.0);
        arrive(&mount, axis, 0.0).await;
        assert!((coordinate(&mount, axis) - start).abs() <= 1.0 / 3600.0);
    }
}

#[tokio::test(start_paused = true)]
async fn defaults_keep_zero_rate_error_and_point_three_backlash_on_both_axes() {
    let mount = SimMount::new(clock());
    assert_eq!(mount.rate_error(), 0.0);
    assert_eq!(mount.truth().backlash_deg, 0.3);
    // Check the full backlash width on each axis, with the same default
    // mechanics but no tilt or zero offsets to mix horizontal coordinates.
    for axis in [Axis::Azimuth, Axis::Altitude] {
        let mount = SimMount::with_config(
            clock(),
            MountConfig {
                base_tilt_deg: 0.0,
                zero_points_deg: [0.0; 2],
                ..MountConfig::default()
            },
        );
        arrive(&mount, axis, -1.0).await;
        arrive(&mount, axis, 0.0).await;
        let from_below = coordinate(&mount, axis);
        arrive(&mount, axis, 1.0).await;
        arrive(&mount, axis, 0.0).await;
        let from_above = coordinate(&mount, axis);
        assert!((from_above - from_below - 0.3).abs() <= 1.0 / 3600.0);
    }
}

#[tokio::test(start_paused = true)]
async fn signed_rate_errors_scale_each_new_command_and_still_stop() {
    for axis in [Axis::Azimuth, Axis::Altitude] {
        let mount = SimMount::with_config(clock(), ideal());
        let shared = mount.clone();
        for fraction in [-0.01, 0.01, 0.0] {
            shared.set_rate_error(fraction);
            assert_eq!(mount.rate_error(), fraction);
            for requested in [-0.01, 0.01] {
                mount.rate(axis, requested).await;
                tokio::time::advance(Duration::from_secs(1)).await;
                let start = coordinate(&mount, axis);
                tokio::time::advance(Duration::from_secs(100)).await;
                let travel = coordinate(&mount, axis) - start;
                let expected = requested * (1.0 + fraction) * 100.0;
                assert!((travel - expected).abs() <= 0.001);
                assert!(mount.state(axis).await.moving);
            }
        }
        mount.set_rate_error(0.01);
        mount.stop_all().await;
        tokio::time::advance(Duration::from_secs(1)).await;
        let stopped = mount.state(axis).await;
        assert!(!stopped.moving);
        assert_eq!(stopped.rate_deg_per_s, 0.0);
    }
}
