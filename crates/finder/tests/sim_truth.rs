//! Oracle: the simulator's true eyepiece direction, independently rotated in
//! simulator::mount. US2-S3 permits at most one arcsecond of separation.
use finder::model::{BaseTilt, Epoch, Model};
use scope_node::mount::{Axis, Mount};
use simulator::{
    mount::{MountConfig, SimMount},
    VirtualClock,
};
use std::time::Duration;

#[tokio::test(start_paused = true)]
async fn known_tilt_and_zero_points_predict_the_true_eyepiece_direction() {
    let mount = SimMount::with_config(
        VirtualClock::new(1_768_528_800.0),
        MountConfig {
            base_tilt_deg: 1.0,
            base_tilt_toward_deg: 130.0,
            zero_points_deg: [17.0, -2.0],
            backlash_deg: 0.0,
        },
    );
    let truth = mount.truth();
    assert_eq!(truth.base_tilt_deg, 1.0);
    assert_eq!(truth.base_tilt_toward_deg, 130.0);
    assert_eq!(truth.zero_points_deg, [17.0, -2.0]);

    mount.goto(Axis::Altitude, 45 * 4500).await;
    mount.goto(Axis::Azimuth, 120 * 4500).await;
    tokio::time::advance(Duration::from_secs(60)).await;
    let positions = [
        mount.state(Axis::Azimuth).await,
        mount.state(Axis::Altitude).await,
    ];
    assert!(positions.iter().all(|axis| !axis.moving));
    assert_eq!(positions.map(|axis| axis.position), [120 * 4500, 45 * 4500]);
    let axes = positions.map(|axis| f64::from(axis.position));
    let counts = mount.axes().map(|axis| axis.counts_per_revolution);
    let mut model = Model::default();
    assert!(model.predict(axes, counts).is_none());
    model.load_known_terms(
        truth.base_tilt_deg,
        truth.base_tilt_toward_deg,
        truth.zero_points_deg,
    );
    // No fitted observations: the prediction must come from the loaded terms.
    assert!(model.syncs.is_empty());
    assert_eq!(
        model.epochs,
        [Epoch {
            ia: -17.0,
            ie: -2.0
        }]
    );
    let base = model.base_tilt.unwrap();
    assert!((base.deg() - 1.0).abs() < 1e-12);
    assert!((base.toward_deg() - 130.0).abs() < 1e-12);
    let actual = mount.true_direction();
    let predicted = model.predict(axes, counts).unwrap();
    assert!(distance(predicted.az_deg, predicted.alt_deg, actual.az, actual.alt) <= 1.0);

    // Negative controls: either missing tilt or missing zero points must fail
    // this oracle, so a degenerate prediction/comparison cannot pass the test.
    model.base_tilt = Some(BaseTilt::default());
    let no_tilt = model.predict(axes, counts).unwrap();
    assert!(distance(no_tilt.az_deg, no_tilt.alt_deg, actual.az, actual.alt) > 60.0);
    model.load_known_terms(truth.base_tilt_deg, truth.base_tilt_toward_deg, [0.0; 2]);
    let no_zeros = model.predict(axes, counts).unwrap();
    assert!(distance(no_zeros.az_deg, no_zeros.alt_deg, actual.az, actual.alt) > 3600.0);
}

fn distance(az: f64, alt: f64, truth_az: f64, truth_alt: f64) -> f64 {
    let ray = |az: f64, alt: f64| {
        let (a, e) = (az.to_radians(), alt.to_radians());
        [e.cos() * a.cos(), e.cos() * a.sin(), e.sin()]
    };
    let (a, b) = (ray(az, alt), ray(truth_az, truth_alt));
    let chord = (0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f64>().sqrt();
    2.0 * (chord / 2.0).clamp(0.0, 1.0).asin().to_degrees() * 3600.0
}
