//! US1: literal pinhole geometry on a north-up, east-left 1456 × 1088 frame.
//! Pixel expectations have 1e-6 px tolerance; angles have 1e-9° unless stated.

use finder::{
    imaging::{align::centre_of, relation::FinderFix},
    sky::Frame,
};
use math_core::{angular_distance, PinholeCamera, UnitVector};
use plate_solver::Solution;
use simulator::{Optics, Simulator, Site, VirtualClock};

use crate::support::live::{DB, START};

fn fix(ra_deg: f64, dec_deg: f64, distortion: Option<f64>) -> FinderFix {
    FinderFix {
        time: 0.0,
        frame_id: 1,
        frame: Frame::from_solution(&Solution {
            ra: Some(ra_deg.to_radians()),
            dec: Some(dec_deg.to_radians()),
            roll: Some(0.0),
            camera: Some(PinholeCamera::new(1456.0, 1088.0, 12_f64.to_radians())),
            distortion,
            ..Default::default()
        })
        .unwrap(),
    }
}

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{actual} differs from {expected} by more than {tolerance}"
    );
}

#[test]
fn one_degree_up_is_in_the_middle() {
    // US1-S1: a meridian step is one degree north, with y = cy - f tan(1°).
    let centre = centre_of(&fix(100.0, 20.0, None), 100.0, 21.0).unwrap();
    let focal = 728.0 / 6_f64.to_radians().tan();
    near(centre.x, 728.0, 1e-6);
    near(centre.y, 544.0 - focal * 1_f64.to_radians().tan(), 1e-6);
    near(centre.right_deg, 0.0, 1e-9);
    near(centre.up_deg, 1.0, 1e-9);
    near(centre.separation_deg, 1.0, 1e-9);
    assert!(centre.on_frame);
    assert!(centre.middle);
    assert_eq!(centre.finder_width, 1456.0);
    assert_eq!(centre.finder_height, 1088.0);
}

#[test]
fn five_degrees_up_is_above_the_frame() {
    // US1-S2: the pinhole projection continues above the top edge.
    let centre = centre_of(&fix(100.0, 20.0, None), 100.0, 25.0).unwrap();
    let focal = 728.0 / 6_f64.to_radians().tan();
    near(centre.y, 544.0 - focal * 5_f64.to_radians().tan(), 1e-6);
    assert!(centre.y < 0.0);
    near(centre.up_deg, 5.0, 1e-9);
    assert!(!centre.on_frame);
    assert!(!centre.middle);
}

#[test]
fn east_is_left_and_distortion_does_not_change_the_pinhole_place() {
    // US1-S3: on the equator, RA differences are the frame's horizontal angles.
    let frame = fix(100.0, 0.0, None);
    let focal = 728.0 / 6_f64.to_radians().tan();
    let left = centre_of(&frame, 108.0, 0.0).unwrap();
    near(left.right_deg, -8.0, 1e-9);
    near(left.up_deg, 0.0, 1e-9);
    near(left.x, 728.0 - focal * 8_f64.to_radians().tan(), 1e-6);
    assert!(!left.on_frame);

    let right = centre_of(&frame, 96.0, 0.0).unwrap();
    near(right.right_deg, 4.0, 1e-9);
    near(right.separation_deg, 4.0, 1e-9);
    assert!(right.on_frame);
    assert!(!right.middle);

    let distorted = centre_of(&fix(100.0, 0.0, Some(-0.2)), 108.0, 0.0).unwrap();
    near(distorted.x, 728.0 - focal * 8_f64.to_radians().tan(), 1e-6);
}

#[test]
fn the_antipode_is_behind_the_camera() {
    // US1-S4: RA + 180°, Dec negated is the opposite sky direction.
    assert!(centre_of(&fix(100.0, 20.0, None), 280.0, -20.0).is_none());
}

#[test]
fn simulator_imaging_camera_can_point_off_the_finder_frame() {
    // US1-S5: freeze the external clock so both truth readings share an instant.
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .start_paused(true)
        .build()
        .unwrap();
    let _entered = runtime.enter();
    let sim = Simulator::new(DB.clone(), Site::default(), VirtualClock::new(START));
    sim.push_to(50.0, 120.0);
    sim.set_optics(Optics {
        eyepiece_px: (-300.0, 420.0),
        ..Optics::default()
    });
    let imaging = sim.imaging_truth();
    let finder = sim.telescope().camera;
    let angle = angular_distance(
        UnitVector::from_radec(imaging.ra_deg.to_radians(), imaging.dec_deg.to_radians()),
        UnitVector::from_radec(finder.ra.to_radians(), finder.dec.to_radians()),
    )
    .to_degrees();
    // The pixel is 1028 px left and 124 px up from the finder camera's centre.
    let focal = 728.0 / 6_f64.to_radians().tan();
    near(
        angle,
        (1028_f64.hypot(124.0) / focal).atan().to_degrees(),
        1e-6,
    );
}
