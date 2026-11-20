//! Tracking errors from subs (036 US2). Oracles: the spec's worked examples,
//! sensor geometry, the rendered star's own samples, and FR-005's defaults.

use finder::imaging::{
    solver::{ImagingSolve, Region},
    steering::{set_point_in_frame, tracking_error, GapFuture, Steered, Stepped, TrackingError},
};
use scope_node::imaging::{Exposure, ImagingCamera, ImagingSettings};
use simulator::{SimImagingCamera, SimStar};

use crate::support::live;

const TARGET: (f64, f64) = (83.81867, -5.38967);
const CENTRE: (f64, f64) = (618.0, 411.0);
const TIME: f64 = 1_768_528_802.5;

fn worked_solve() -> ImagingSolve {
    ImagingSolve::from_cd(
        83.8221,
        -5.3911,
        [
            [-5.196152422706632e-4, -3.0e-4],
            [3.0e-4, -5.196152422706632e-4],
        ],
        1236,
        822,
    )
}

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{actual} differs from {expected} by more than {tolerance}"
    );
}

#[tokio::test(start_paused = true)]
async fn worked_centre_error_has_the_given_time_and_sky_signs() {
    // US2-S1: 0.00343 degrees east in RA at Dec -5.39, 0.00143 south.
    let error = tracking_error(&worked_solve(), CENTRE, TARGET, TIME).unwrap();
    assert_eq!(error.time, TIME);
    near(error.east_arcsec, 12.2934, 0.001);
    near(error.north_arcsec, -5.1480, 0.001);
    let value = serde_json::to_value(error).unwrap();
    assert_eq!(value["time"], TIME);
    assert_eq!(value["east_arcsec"], error.east_arcsec);
    assert_eq!(value["north_arcsec"], error.north_arcsec);
}

#[tokio::test(start_paused = true)]
async fn worked_named_set_point_applies_the_whole_matrix() {
    // US2-S2: add 3600 * (-0.0369615, 0.0559808) degrees to US2-S1.
    let error = tracking_error(&worked_solve(), (718.0, 361.0), TARGET, TIME).unwrap();
    assert_eq!(error.time, TIME);
    near(error.east_arcsec, -120.768, 0.001);
    near(error.north_arcsec, 196.383, 0.001);
}

#[tokio::test(start_paused = true)]
async fn sensor_set_point_becomes_frame_pixels_without_rounding_the_centre() {
    // US2-S3: sensor centre, translated by the region then divided by binning.
    let whole = Region {
        x: 0,
        y: 0,
        width: 1236,
        height: 822,
    };
    assert_eq!(set_point_in_frame((1236, 822), None, whole, 1), CENTRE);
    let region = Region {
        x: 400,
        y: 200,
        width: 400,
        height: 400,
    };
    assert_eq!(
        set_point_in_frame((1236, 822), None, region, 2),
        (109.0, 105.5)
    );
    assert_eq!(
        set_point_in_frame((1236, 822), Some((718.0, 361.0)), whole, 1),
        (718.0, 361.0)
    );
    assert_eq!(
        set_point_in_frame(
            (1237, 823),
            None,
            Region {
                width: 1237,
                height: 823,
                ..whole
            },
            1
        ),
        (618.5, 411.5)
    );
    assert_eq!(
        set_point_in_frame((1236, 822), Some((718.0, 361.0)), region, 2),
        (159.0, 80.5)
    );
}

#[tokio::test(start_paused = true)]
async fn rendered_target_has_zero_error_at_its_own_centroid() {
    // US2-S4: this rig has no moving mount. Read truth at mid-exposure, and
    // independently construct a destination 100 arcseconds away at PA 70.
    let sim = live::sim();
    let time = sim.clock().now() + 0.010;
    let truth = sim.imaging_truth_at(time);
    let (ra, dec) = (truth.ra_deg.to_radians(), truth.dec_deg.to_radians());
    let distance = (100.0_f64 / 3600.0).to_radians();
    let angle = 70.0_f64.to_radians();
    let target_dec = (dec.sin() * distance.cos() + dec.cos() * distance.sin() * angle.cos()).asin();
    let target_ra = ra
        + (angle.sin() * distance.sin() * dec.cos())
            .atan2(distance.cos() - dec.sin() * target_dec.sin());
    let target = (
        target_ra.to_degrees().rem_euclid(360.0),
        target_dec.to_degrees(),
    );
    let scale = truth.arcsec_per_px / 3600.0;
    let (sin, cos) = truth.up_east_of_north_deg.to_radians().sin_cos();
    let solve = ImagingSolve::from_cd(
        truth.ra_deg,
        truth.dec_deg,
        [[-scale * cos, -scale * sin], [scale * sin, -scale * cos]],
        1236,
        822,
    );
    let camera = SimImagingCamera::new(sim.clone())
        .with_noise(false)
        .with_stars(vec![SimStar {
            ra_deg: target.0,
            dec_deg: target.1,
            magnitude: 5.0,
            colour: [1.0; 3],
        }]);
    camera
        .apply(&ImagingSettings {
            gain: Some(0.0),
            ..Default::default()
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
    assert_eq!((frame.width, frame.height), (1236, 822));
    assert_eq!(frame.bits, 16);
    let samples: Vec<f64> = frame
        .samples
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| f64::from(u16::from_le_bytes(*pair)))
        .collect();
    let width = frame.width as usize;
    let brightest = samples
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .unwrap()
        .0;
    let (column, row) = (brightest % width, brightest / width);
    let (mut weight, mut x, mut y) = (0.0, 0.0, 0.0);
    for r in row.saturating_sub(10)..=(row + 10).min(frame.height as usize - 1) {
        for c in column.saturating_sub(10)..=(column + 10).min(width - 1) {
            let signal = samples[r * width + c] - 160.0;
            weight += signal;
            x += signal * (c as f64 + 0.5);
            y += signal * (r as f64 + 0.5);
        }
    }
    assert!(weight > 0.0, "the rendered star must be above background");
    let centroid = (x / weight, y / weight);
    // The rendered image is the oracle: 0.3 px at about 2.18 arcsec per px.
    let error = tracking_error(&solve, centroid, target, time).unwrap();
    near(error.east_arcsec, 0.0, 0.7);
    near(error.north_arcsec, 0.0, 0.7);
    // Centre less target = -100 * (sin 70, cos 70) arcseconds.
    let error = tracking_error(&solve, CENTRE, target, time).unwrap();
    near(error.east_arcsec, -93.97, 0.05);
    near(error.north_arcsec, -34.20, 0.05);
}

#[tokio::test(start_paused = true)]
async fn unprojectable_targets_and_non_finite_numbers_have_no_error() {
    // US2-S5: the far side has no tangent-plane pixel; bad numbers cannot
    // describe a tracking error. Check both coordinates and both infinities.
    let solve = worked_solve();
    assert_eq!(
        tracking_error(&solve, CENTRE, (263.8221, 5.3911), TIME),
        None
    );
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(tracking_error(&solve, CENTRE, TARGET, invalid), None);
        for set in [(invalid, CENTRE.1), (CENTRE.0, invalid)] {
            assert_eq!(tracking_error(&solve, set, TARGET, TIME), None);
        }
        for target in [(invalid, TARGET.1), (TARGET.0, invalid)] {
            assert_eq!(tracking_error(&solve, CENTRE, target, TIME), None);
        }
    }
    let singular = ImagingSolve::from_cd(83.8221, -5.3911, [[0.0; 2]; 2], 1236, 822);
    assert_eq!(tracking_error(&singular, CENTRE, TARGET, TIME), None);
    // Finite inputs whose result overflows must also be refused.
    assert_eq!(
        tracking_error(&solve, (f64::MAX, f64::MAX), TARGET, TIME),
        None
    );
}

#[tokio::test(start_paused = true)]
async fn steering_source_uses_only_mount_neutral_terms() {
    // US2-S6: rule 1 applies to the complete source, comments included.
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/imaging/steering.rs"),
    )
    .unwrap();
    for forbidden in [
        "ephemeris",
        "alt_az",
        "parallactic",
        "counts_per_revolution",
        "AxisInfo",
        "MountClient",
        "azimuth",
        "altitude",
        "field_rotation",
        "hour_angle",
        "meridian",
        "backlash",
        "keyhole",
    ] {
        assert!(
            !source.contains(forbidden),
            "steering source contains {forbidden}"
        );
    }
}

struct MinimalTracker;

impl Steered for MinimalTracker {
    fn target(&self) -> Option<(f64, f64)> {
        None
    }
    fn steer(&self, _error: TrackingError) {}
    fn gap(&self) -> GapFuture {
        Box::pin(async { Stepped::default() })
    }
    fn release(&self) {}
    fn steered_by_subs(&self) -> bool {
        false
    }
}

#[tokio::test(start_paused = true)]
async fn five_required_methods_leave_the_six_provided_methods_optional() {
    // US2-S7 / FR-005: a tracker that says nothing limits nothing.
    let tracker: &dyn Steered = &MinimalTracker;
    assert_eq!(tracker.longest_exposure_s(0.001), None);
    assert_eq!(tracker.longest_exposure_ahead_s(0.001), None);
    assert!(tracker.steady());
    assert!(tracker.steady_between(0.0, 1.0));
    assert_eq!(tracker.pause(), None);
    let () = tracker.tune(0.5, 0.3);
}
