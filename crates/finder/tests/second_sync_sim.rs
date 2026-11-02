//! Oracle: the simulator's true direction and exact 1° base tilt toward 130°.
//! Azimuth separation is at least 90°; both fitted tilt coordinates are within 1′.
#[path = "goto_api.rs"]
mod api;
use api::Rig;
use scope_node::mount::{Axis, Mount};
const START: f64 = 1_768_528_800.0;

// Inverse Rodrigues rotation, independent of the pointing model, puts the first
// sync at true azimuth 120°, altitude 50°. Both syncs use the upward backlash flank.
fn initial_zero() -> [f64; 2] {
    let (a, e, t) = (
        120_f64.to_radians(),
        50_f64.to_radians(),
        130_f64.to_radians(),
    );
    let v = [e.cos() * a.cos(), e.cos() * a.sin(), e.sin()];
    let k = [-t.sin(), t.cos(), 0.0];
    let (s, c) = (-1_f64).to_radians().sin_cos();
    let dot = k[0] * v[0] + k[1] * v[1];
    let cross = [k[1] * v[2], -k[0] * v[2], k[0] * v[1] - k[1] * v[0]];
    let v = [0, 1, 2].map(|i| v[i] * c + cross[i] * s + k[i] * dot * (1.0 - c));
    [
        v[1].atan2(v[0]).to_degrees() - 0.85,
        v[2].asin().to_degrees() - 0.85,
    ]
}
#[tokio::test(start_paused = true)]
async fn one_tap_slews_takes_a_sync_and_fits_the_base_tilt_without_corrections() {
    let zero = initial_zero();
    let rig = Rig::new(START, zero[1], zero[0], true).await;
    rig.move_to([4500, 4500]).await;
    for _ in 0..15 {
        rig.tick().await;
    }
    rig.state.session().reset_model(false, rig.now()).unwrap();
    rig.sync().await;
    let first = rig.state.session().model(rig.now());
    assert_eq!(first.syncs.len(), 1);
    assert!((first.syncs[0].az_deg - 120.0).abs() * 3600.0 < 30.0);
    assert!(first.base_tilt.is_none());
    assert!((rig.mount.true_direction().az - 120.0).abs() < 0.001);
    let target = rig.view().await["target"].clone();
    let (status, body) = rig.call("/api/model/second-sync", false).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["started"], true);
    let progress = rig.finish(Some(15.0), true).await;
    assert_eq!(progress.last().unwrap()["state"], "done", "{progress:?}");
    assert!(progress.iter().all(|p| p["correction"] == 0));
    assert!(progress.iter().any(|p| p["state"] == "slewing"));
    assert!(progress.iter().any(|p| p["state"] == "solving"));
    let direction = rig.mount.true_direction();
    let difference = (direction.az - 120.0 + 180.0).rem_euclid(360.0) - 180.0;
    assert!(difference.abs() >= 90.0, "true direction: {direction:?}");
    let model = rig.state.session().model(rig.now());
    let second = model.syncs.last().unwrap();
    assert!(model.syncs.len() >= 2);
    assert!(second.time > first.syncs[0].time);
    for (i, axis) in [Axis::Azimuth, Axis::Altitude].into_iter().enumerate() {
        assert!((second.axis[i] - f64::from(rig.mount.state(axis).await.position)).abs() <= 1.0);
    }
    let base = model.base_tilt.expect("second sync must fit the base tilt");
    assert_eq!(model.epochs.len(), 1);
    assert!((base.deg() - 1.0).abs() * 60.0 < 1.0, "{base:?}");
    assert!((base.toward_deg() - 130.0).abs() * 60.0 < 1.0, "{base:?}");
    assert_eq!(rig.view().await["target"], target);
}

// Exact, level model for spot-selection geometry; expectations are circular
// intervals on a compass, with 0.001° tolerance for count rounding.
fn level_model(azimuths: &[f64]) -> finder::model::Model {
    let mut model = finder::model::Model::default();
    model.epochs.push(finder::model::Epoch { ia: 0.0, ie: 0.0 });
    model.base_tilt = Some(Default::default());
    model.syncs = azimuths
        .iter()
        .map(|&az| finder::model::Sync {
            time: START,
            axis: [az * 1000.0, 50_000.0],
            counts_per_revolution: [360_000; 2],
            az_deg: az,
            alt_deg: 50.0,
            roll_deg: None,
            rms_arcsec: 1.0,
            epoch: 0,
        })
        .collect();
    model
}
fn planner(model: &finder::model::Model) -> finder::goto::Planner<'_> {
    finder::goto::Planner {
        model,
        axes: [scope_link::proto::AxisInfo {
            counts_per_revolution: 360_000,
            top_rate_deg_per_s: 3.4,
            can_goto: true,
        }; 2],
        current: [120_000, 50_000],
        wind: Default::default(),
        site: finder::session::Site {
            lat_deg: 40.0,
            lon_deg: -75.0,
        },
        start: START,
        min_alt: 15.0,
    }
}
#[test]
fn nearest_spot_respects_every_nightly_sync_and_wraps_through_north() {
    // From 120°, the closest point >=90° from both 120° and 250° is 30°.
    let model = level_model(&[120.0, 250.0]);
    let plan = planner(&model).second_sync().unwrap();
    let p = model
        .predict(plan.destination.map(f64::from), [360_000; 2])
        .unwrap();
    assert!((p.az_deg - 30.0).abs() < 0.001, "{}", p.az_deg);
    for sync in &model.syncs {
        let difference = (p.az_deg - sync.az_deg + 180.0).rem_euclid(360.0) - 180.0;
        assert!(difference.abs() >= 90.0 - 0.001);
    }
    assert!((p.alt_deg - 50.0).abs() < 0.001);
}
#[test]
fn impossible_azimuth_spread_is_refused() {
    let model = level_model(&[0.0, 120.0, 240.0]);
    assert!(planner(&model)
        .second_sync()
        .unwrap_err()
        .contains("90 degrees"));
}
#[tokio::test(start_paused = true)]
async fn second_sync_stop_and_missing_sync_use_the_existing_motion_guards() {
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    rig.sync().await;
    assert_eq!(rig.call("/api/model/second-sync", false).await.0, 200);
    for _ in 0..100 {
        if !rig.stationary().await {
            break;
        }
        rig.tick().await;
    }
    assert!(!rig.stationary().await);
    let (status, body) = rig.call("/api/model/second-sync", false).await;
    assert_eq!(status, 409, "{body}");
    assert!(body["error"].as_str().unwrap().contains("already running"));
    assert_eq!(rig.call("/api/stop", true).await.0, 200);
    for _ in 0..20 {
        rig.tick().await;
    }
    let progress = rig.finish(None, false).await;
    assert_eq!(progress.last().unwrap()["state"], "halted");
    assert!(rig.stationary().await);
    rig.controls.stop_frames();
    for _ in 0..10 {
        rig.tick().await;
    }
    rig.state.session().reset_model(false, rig.now()).unwrap();
    let before = [
        rig.mount.state(Axis::Azimuth).await.position,
        rig.mount.state(Axis::Altitude).await.position,
    ];
    let (status, body) = rig.call("/api/model/second-sync", false).await;
    assert_eq!(status, 409, "{body}");
    assert!(body["error"].as_str().unwrap().contains("no sync"));
    for _ in 0..15 {
        rig.tick().await;
        assert!(rig.stationary().await);
    }
    assert_eq!(
        before,
        [
            rig.mount.state(Axis::Azimuth).await.position,
            rig.mount.state(Axis::Altitude).await.position
        ]
    );
}

#[test]
fn second_sync_refuses_a_mount_without_goto_and_the_sun_up() {
    let model = level_model(&[120.0]);
    let mut p = planner(&model);
    p.axes[0].can_goto = false;
    assert!(p.second_sync().unwrap_err().contains("no mount"));
    p.axes[0].can_goto = true;
    // On the solar meridian at the equator the January Sun is well above
    // the horizon (independent of the second-sync selection algorithm).
    let (ra, _) = finder::ephemeris::sun(finder::sky::julian_date(START));
    p.site.lat_deg = 0.0;
    p.site.lon_deg = (ra - finder::sky::gmst_deg(finder::sky::julian_date(START)) + 180.0)
        .rem_euclid(360.0)
        - 180.0;
    assert_eq!(p.second_sync().unwrap_err(), "Sun is up");
}
