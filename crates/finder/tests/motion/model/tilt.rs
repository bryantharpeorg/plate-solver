//! Oracle: the mount geometry in 053's User Scenarios, independent of the fit.
//! Zero points, collimation and axis non-perpendicularity precede a Rodrigues
//! rotation of the tube by the base tilt. Distances use the chord half-angle.
use finder::model::{Model, Sync};

const COUNTS: [u32; 2] = [1_620_000; 2];

#[derive(Clone, Copy)]
struct Mount {
    tilt: f64,
    toward: f64,
    zeros: [f64; 2],
    collimation: f64,
    non_perpendicularity: f64,
}

impl Mount {
    fn new(tilt: f64, toward: f64) -> Self {
        Self {
            tilt,
            toward,
            zeros: [17.0, -2.0],
            collimation: 0.0,
            non_perpendicularity: 0.0,
        }
    }

    fn direction(self, axis: [f64; 2]) -> [f64; 2] {
        let a = (axis[0] + self.zeros[0]).to_radians();
        let e = (axis[1] + self.zeros[1]).to_radians();
        let c = self.collimation.to_radians();
        let n = self.non_perpendicularity.to_radians();
        let forward = [a.cos(), a.sin(), 0.0];
        let hinge = [-a.sin() * n.cos(), a.cos() * n.cos(), n.sin()];
        let up = cross(forward, hinge);
        let tube = [0, 1, 2]
            .map(|i| c.cos() * (e.cos() * forward[i] + e.sin() * up[i]) + c.sin() * hinge[i]);
        let w = self.toward.to_radians();
        let rotation_axis = [-w.sin(), w.cos(), 0.0];
        let perpendicular = cross(rotation_axis, tube);
        let parallel: f64 = (0..3).map(|i| rotation_axis[i] * tube[i]).sum();
        let (s, c) = self.tilt.to_radians().sin_cos();
        let ray = [0, 1, 2]
            .map(|i| tube[i] * c + perpendicular[i] * s + rotation_axis[i] * parallel * (1.0 - c));
        [
            ray[1].atan2(ray[0]).to_degrees().rem_euclid(360.0),
            ray[2].atan2(ray[0].hypot(ray[1])).to_degrees(),
        ]
    }

    fn sync(self, axis: [f64; 2], time: f64) -> Sync {
        let [az_deg, alt_deg] = self.direction(axis);
        Sync {
            time,
            axis: counts(axis),
            counts_per_revolution: COUNTS,
            az_deg,
            alt_deg,
            roll_deg: None,
            rms_arcsec: 1.0,
            epoch: 0,
        }
    }

    fn miss(self, model: &Model, axis: [f64; 2]) -> f64 {
        let prediction = model.predict(counts(axis), COUNTS).unwrap();
        distance(
            [prediction.az_deg, prediction.alt_deg],
            self.direction(axis),
        )
    }

    fn worst_miss(self, model: &Model) -> f64 {
        (0..360)
            .step_by(15)
            .flat_map(|az| {
                (22..=82)
                    .step_by(10)
                    .map(move |alt| self.miss(model, [f64::from(az), f64::from(alt)]))
            })
            .fold(0.0, f64::max)
    }
}

fn counts(axis: [f64; 2]) -> [f64; 2] {
    [0, 1].map(|i| axis[i] * f64::from(COUNTS[i]) / 360.0)
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn angle_difference(a: f64, b: f64) -> f64 {
    (a - b + 180.0).rem_euclid(360.0) - 180.0
}

fn distance(a: [f64; 2], b: [f64; 2]) -> f64 {
    let ray = |[az, alt]: [f64; 2]| {
        let (az, alt) = (az.to_radians(), alt.to_radians());
        [alt.cos() * az.cos(), alt.cos() * az.sin(), alt.sin()]
    };
    let (a, b) = (ray(a), ray(b));
    let chord = (0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f64>().sqrt();
    (2.0 * (chord / 2.0).clamp(0.0, 1.0).asin()).to_degrees()
}

// Compute the one-sync prediction directly from the first observation's zero
// points, without consulting Model::predict or using the fit as an oracle.
fn one_sync_prediction(first: &Sync, second: &Sync) -> [f64; 2] {
    let first_axis = [0, 1].map(|i| first.axis[i] * 360.0 / f64::from(COUNTS[i]));
    let second_axis = [0, 1].map(|i| second.axis[i] * 360.0 / f64::from(COUNTS[i]));
    [
        second_axis[0] + angle_difference(first.az_deg, first_axis[0]),
        second_axis[1] + first.alt_deg - first_axis[1],
    ]
}

/// US1-S1: tilt and direction within 1 arcminute; prediction within 1 arcsecond.
#[test]
fn quarter_turn_fits_zero_to_five_degrees_in_both_directions() {
    for toward in [195.0, 15.0] {
        for tilt in [0.0, 1.0, 3.0, 5.0] {
            let mount = Mount::new(tilt, toward);
            let first = mount.sync([105.0, 77.0], 0.0);
            let second = mount.sync([0.0, 45.0], 120.0);
            if tilt == 5.0 && toward == 15.0 {
                assert!(angle_difference(first.az_deg, second.az_deg).abs() < 90.0);
            }
            let mut model = Model::default();
            assert!(model.add_sync(first));
            assert!(model.add_sync(second));
            assert_eq!(model.epochs.len(), 1, "{tilt} degrees toward {toward}");
            let base = model.base_tilt.expect("the quarter turn fits base tilt");
            assert!((base.deg() - tilt).abs() < 1.0 / 60.0, "{base:?}");
            if tilt > 0.0 {
                assert!(angle_difference(base.toward_deg(), toward).abs() < 1.0 / 60.0);
            }
            assert!(mount.miss(&model, [250.0, 30.0]) < 1.0 / 3600.0);
        }
    }
}

/// US1-S2: an excessive base tilt opens an epoch fitting the second sync to 1 arcsecond.
#[test]
fn eight_degree_base_tilt_is_taken_as_a_slip() {
    let mount = Mount::new(8.0, 195.0);
    let mut model = Model::default();
    assert!(model.add_sync(mount.sync([105.0, 77.0], 0.0)));
    assert!(model.add_sync(mount.sync([0.0, 45.0], 120.0)));
    assert_eq!(model.epochs.len(), 2);
    assert!(model.base_tilt.is_none());
    assert!(mount.miss(&model, [0.0, 45.0]) < 1.0 / 3600.0);
}

/// US1-S3: the tilt alone decides even when the one-sync miss is below 2.5 degrees.
#[test]
fn excessive_tilt_is_a_slip_even_near_the_one_sync_prediction() {
    let mount = Mount::new(7.0, 0.0);
    let first = mount.sync([120.0, 10.0], 0.0);
    let second = mount.sync([210.0, 10.0], 120.0);
    assert!(
        distance(
            one_sync_prediction(&first, &second),
            mount.direction([210.0, 10.0])
        ) < 2.5
    );
    let mut model = Model::default();
    assert!(model.add_sync(first));
    assert!(model.add_sync(second));
    assert_eq!(model.epochs.len(), 2);
    assert!(model.base_tilt.is_none());
}

/// US1-S4: unmodelled terms leave less than 0.5 degrees worst miss with the
/// lower second sync, and at least 0.5 degrees with the high one; tilt within 0.15 degrees.
#[test]
fn a_lower_second_sync_limits_the_miss_from_unmodelled_terms() {
    for (collimation, non_perpendicularity) in [(0.25, 0.0), (0.0, 0.25)] {
        let mount = Mount {
            collimation,
            non_perpendicularity,
            ..Mount::new(3.0, 195.0)
        };
        for altitude in [47.0, 81.0] {
            let mut model = Model::default();
            assert!(model.add_sync(mount.sync([105.0, 77.0], 0.0)));
            assert!(model.add_sync(mount.sync([0.0, altitude], 120.0)));
            assert_eq!(model.epochs.len(), 1);
            let base = model.base_tilt.unwrap();
            assert!((base.deg() - 3.0).abs() < 0.15, "{base:?}");
            let worst = mount.worst_miss(&model);
            if altitude == 47.0 {
                assert!(worst < 0.5, "worst miss {worst}");
            } else {
                assert!(worst >= 0.5, "worst miss {worst}");
            }
        }
    }
}

/// US1-S5: a 5-degree azimuth slip can be absorbed as base tilt; it then misses
/// by over 2 degrees elsewhere even though both syncs were accepted in one epoch.
#[test]
fn an_unmarked_slip_between_two_syncs_is_absorbed_as_base_tilt() {
    let before = Mount::new(1.0, 130.0);
    let slipped = Mount {
        zeros: [22.0, -2.0],
        ..before
    };
    let first = before.sync([30.0, 40.0], 0.0);
    let second = slipped.sync([140.0, 50.0], 200.0);
    assert!(
        distance(
            one_sync_prediction(&first, &second),
            slipped.direction([140.0, 50.0])
        ) > 2.5
    );
    let mut model = Model::default();
    assert!(model.add_sync(first));
    assert!(model.add_sync(second));
    assert_eq!(model.epochs.len(), 1);
    assert!(model.base_tilt.is_some());
    assert!(slipped.worst_miss(&model) > 2.0);
}

/// US1-S6: an explicit slip mark opens an epoch at a tilt spot too.
#[test]
fn a_marked_slip_opens_an_epoch_before_judging_the_base_tilt() {
    let mount = Mount::new(1.0, 130.0);
    let mut model = Model::default();
    assert!(model.add_sync(mount.sync([30.0, 40.0], 0.0)));
    model.mark_slip();
    assert!(model.add_sync(mount.sync([140.0, 50.0], 200.0)));
    assert_eq!(model.epochs.len(), 2);
    assert!(model.base_tilt.is_none());
}

/// US1-S7: the old epoch's far sync must not make a nearby third sync a tilt spot.
#[test]
fn the_tilt_spot_uses_only_the_current_epoch() {
    let mount = Mount::new(8.0, 195.0);
    let mut model = Model::default();
    assert!(model.add_sync(mount.sync([105.0, 77.0], 0.0)));
    assert!(model.add_sync(mount.sync([0.0, 45.0], 120.0)));
    assert_eq!(model.epochs.len(), 2);
    assert!(model.base_tilt.is_none());
    assert!(model.add_sync(mount.sync([350.0, 45.0], 240.0)));
    assert_eq!(model.epochs.len(), 2);
}

/// US1-S8: across north, a 30-degree axis move does not qualify as a quarter turn.
#[test]
fn the_tilt_spot_takes_the_short_way_across_north() {
    let mount = Mount::new(1.0, 130.0);
    let mut model = Model::default();
    assert!(model.add_sync(mount.sync([350.0, 45.0], 0.0)));
    assert!(model.add_sync(mount.sync([20.0, 45.0], 120.0)));
    assert_eq!(model.epochs.len(), 1);
    assert!(model.base_tilt.is_none());
}
