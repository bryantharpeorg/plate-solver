//! Oracles: configured 0.3 degree mechanical play and the simulator's independent
//! eyepiece ray. Exact command ordering; 60/30 arcsecond sky bounds in paused time.
use crate::support::tracker;
use finder::{session::Session, targets};
use scope_node::mount::{Axis, Mount};
use tracker::*;

#[tokio::test(start_paused = true)]
async fn first_transit_measures_then_fixed_reset_removes_the_observed_value() {
    let rig = measured(Session::default()).await;
    assert!(rig.body().await["backlash"]["alt_deg"].as_f64().unwrap() > 0.0);
    let (status, body) = support::request(
        &rig.app,
        "POST",
        "/api/model/reset",
        serde_json::json!({"what":"fixed"}),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert!(body.get("backlash").is_none());
    assert!(rig.body().await.get("backlash").is_none());
    rig.tracker.stop("stopped").await;
}

#[tokio::test(start_paused = true)]
async fn first_azimuth_reversal_measures_its_own_dead_band() {
    let target = targets::find("M81").unwrap().clone();
    let turn = reversal(&target, 0);
    let rig = Rig::new(turn - 600.0, target, Session::default()).await;
    let start = rig.now();
    rig.follow(turn + 600.0, 60.0).await;
    assert!(rig.true_error_arcsec() <= 30.0);
    rig.assert_reversal(Axis::Azimuth, start, false);
    let body = rig.body().await;
    let az = body["backlash"]["az_deg"].as_f64().unwrap();
    assert!((az - 0.3).abs() * 3600.0 <= 30.0, "measured {az}");
    assert!(body["backlash"].get("alt_deg").is_none());
    rig.tracker.stop("stopped").await;
}

#[tokio::test(start_paused = true)]
async fn another_target_on_the_same_mount_takes_up_the_measured_altitude_play() {
    let mut rig = measured(Session::default()).await;
    let b = rig.body().await["backlash"]["alt_deg"].as_f64().unwrap();
    rig.tracker.stop("stopped").await;
    let mut next = rig.target.clone();
    next.ra_deg += 5.0;
    let transit = reversal(&next, 1);
    // A GoTo-style final positive approach, on this same mechanical mount.
    let (h, a) = apparent(&next, rig.now() + 10.0);
    let counts = rig.mount.axes().map(|a| a.counts_per_revolution);
    let dest = rig.model.inverse(a, h, counts).unwrap();
    for offset in [-0.5, 0.0] {
        for (i, axis) in AXES.into_iter().enumerate() {
            rig.mount
                .goto(
                    axis,
                    (dest[i] + offset * f64::from(counts[i]) / 360.0).round() as i32,
                )
                .await;
        }
        loop {
            tokio::time::sleep(STEP).await;
            if !rig.mount.state(Axis::Azimuth).await.moving
                && !rig.mount.state(Axis::Altitude).await.moving
            {
                break;
            }
        }
    }
    rig.target = next.clone();
    rig.tracker.start(next).await.unwrap();
    rig.follow(transit - 60.0, 180.0).await;
    let start = rig.now();
    rig.follow(transit + 120.0, 30.0).await;
    // Command duration follows the measured value, within its 30 arcsec tolerance.
    assert!((b - 0.3).abs() < 30.0 / 3600.0);
    rig.assert_reversal(Axis::Altitude, start, true);
    rig.tracker.stop("stopped").await;
}

#[tokio::test(start_paused = true)]
async fn m81_takes_up_azimuth_play_at_greatest_elongation() {
    let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target")
        .join(format!("backlash-az-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let path = folder.join("session.json");
    std::fs::write(
        &path,
        r#"{"site":null,"target":null,"min_alt_deg":15,"backlash":{"az_deg":0.3,"alt_deg":0.3}}"#,
    )
    .unwrap();
    let target = targets::find("M81").unwrap().clone();
    // March puts the western elongation in darkness. Its positive hour angle
    // exercises the negative-to-positive reversal, opposite to a transit.
    let turn = turning_point(&target, 0, START + 60.0 * 86_400.0, false);
    let (h, _) = apparent(&target, turn);
    assert!((h - 43.5).abs() < 0.5, "M81 elongation altitude {h}");
    let (_, az) = apparent(&target, turn);
    let (e, a, p) = (h.to_radians(), az.to_radians(), 40_f64.to_radians());
    let hour_angle = (-a.sin() * e.cos())
        .atan2(e.sin() * p.cos() - e.cos() * p.sin() * a.cos())
        .to_degrees();
    assert!(
        (hour_angle - 71.3).abs() < 0.5,
        "M81 hour angle {hour_angle}"
    );
    let rig = Rig::new(
        turn - 120.0,
        target,
        Session::with_state_file(&path).unwrap(),
    )
    .await;
    rig.follow(rig.now() + 10.0, 30.0).await;
    let start = rig.now();
    rig.follow(turn + 120.0, 30.0).await;
    rig.assert_reversal(Axis::Azimuth, start, true);
    rig.tracker.stop("stopped").await;
    std::fs::remove_dir_all(folder).unwrap();
}

#[tokio::test(start_paused = true)]
async fn descending_syncs_are_added_without_changing_epoch_or_biasing_ie() {
    let target = targets::find("M42").unwrap().clone();
    let transit = reversal(&target, 1);
    let rig = Rig::new(transit - 600.0, target, Session::default()).await;
    let mut up_ie = None;
    let mut down = 0;
    let mut held = 0;
    while rig.now() < transit + 600.0 {
        let e = rig.truth_error();
        rig.tracker.pointing(e[0], e[1]);
        {
            let d = rig.mount.true_direction();
            let counts = rig.mount.axes().map(|a| a.counts_per_revolution);
            let axis = [
                rig.mount.state(Axis::Azimuth).await.position as f64,
                rig.mount.state(Axis::Altitude).await.position as f64,
            ];
            let accepted = rig
                .session
                .record_sync(finder::model::Sync {
                    time: rig.now(),
                    axis,
                    counts_per_revolution: counts,
                    az_deg: d.az,
                    alt_deg: d.alt,
                    roll_deg: None,
                    rms_arcsec: 3.0,
                    epoch: 0,
                })
                .unwrap();
            if rig.now() >= transit && rig.body().await.get("backlash").is_none() {
                if rig.mount.state(Axis::Altitude).await.rate_deg_per_s < 0.0 {
                    assert!(!accepted, "unknown reverse-side sync must be held out");
                    held += 1;
                }
                rig.follow(rig.now() + 2.0, 60.0).await;
                continue;
            }
            assert!(accepted);
            let model = rig.session.model(rig.now());
            if rig.now() < transit {
                up_ie = Some(model.epochs.last().unwrap().ie);
            } else {
                down += 1;
                assert_eq!(model.epochs.len(), 1);
                assert!(model.syncs.iter().any(|s| s.time > transit));
                assert!((model.epochs.last().unwrap().ie - up_ie.unwrap()).abs() * 3600.0 <= 30.0);
            }
        }
        rig.follow(rig.now() + 2.0, 60.0).await;
    }
    assert!(held > 10, "syncs while measuring must actually be held out");
    assert!(down > 10, "descending syncs must actually be added");
    rig.tracker.stop("stopped").await;
}
