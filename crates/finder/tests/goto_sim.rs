//! Paused-time motion scenarios share the mounted route harness and its controls.
#[path = "goto_api.rs"]
mod api;
use api::Rig;
use finder::sky;
use scope_node::mount::{Axis, Mount};
const START: f64 = 1_768_528_800.0;

#[tokio::test(start_paused = true)]
async fn five_catalog_targets_converge_after_two_syncs_and_report_progress() {
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    rig.second().await;
    for name in ["M42", "M31", "M45", "M81", "M1"] {
        rig.target(name);
        assert_eq!(rig.call("/api/goto", false).await.0, 200);
        let mut nudged = false;
        let progress = rig
            .finish_with(Some(15.0), true, |v| {
                if name == "M42" && !nudged && v["state"] == "settling" {
                    // Known initial slip makes the numbered correction observable.
                    rig.mount.push_by(0.1, 0.0);
                    nudged = true;
                }
            })
            .await;
        assert_eq!(progress[0]["state"], "slewing");
        assert!(progress.iter().any(|v| v["state"] == "solving"));
        let last = progress.last().unwrap();
        assert_eq!(last["state"], "done", "{name}: {progress:?}");
        assert!(last["correction"].as_u64().unwrap() <= 5);
        if name == "M42" {
            assert!(last["correction"].as_u64().unwrap() > 0);
        }
        assert!(last["error_arcsec"].as_f64().unwrap() < 60.0);
        assert!(
            rig.error_arcsec(name) < 60.0,
            "{name}: {} arcsec",
            rig.error_arcsec(name)
        );
        let corrections: Vec<_> = progress
            .iter()
            .filter(|v| v["state"] == "correcting")
            .map(|v| v["correction"].as_u64().unwrap())
            .collect();
        assert_eq!(
            corrections,
            (1..=last["correction"].as_u64().unwrap()).collect::<Vec<_>>()
        );
    }
}
#[tokio::test(start_paused = true)]
async fn sixteen_degree_target_never_drops_below_fifteen() {
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    rig.second().await;
    let target = horizontal_target(&rig, 16.0, 190.0);
    rig.state
        .goto()
        .unwrap()
        .start_target(target)
        .await
        .unwrap();
    let progress = rig.finish(Some(15.0), false).await;
    assert_eq!(progress.last().unwrap()["state"], "done", "{progress:?}");
    assert!(rig.mount.true_direction().alt >= 15.0);
}
fn horizontal_target(rig: &Rig, alt: f64, az: f64) -> finder::targets::Target {
    let mut target = finder::targets::find("M42").unwrap().clone();
    let date = simulator::sky::horizontal_to_date(alt, az, simulator::Site::default(), rig.now());
    let (ra, dec) = simulator::sky::to_j2000(date, sky::julian_date(rig.now())).to_radec();
    target.ra_deg = ra.to_degrees();
    target.dec_deg = dec.to_degrees();
    target
}
fn crossing(alt: f64, mut low: f64, mut high: f64, rising: bool) -> f64 {
    for _ in 0..45 {
        let mid = (low + high) / 2.0;
        if (api::sun_truth(40.0, -75.0, mid).0 < alt) == rising {
            low = mid;
        } else {
            high = mid;
        }
    }
    (low + high) / 2.0
}
#[tokio::test(start_paused = true)]
async fn dusk_path_lifts_clear_of_the_sun_and_completes() {
    let start = crossing(-10.0, START - 8.0 * 3600.0, START, false);
    let sun = api::sun_truth(40.0, -75.0, start);
    let rig = Rig::new(start, 3.0, sun.1 - 30.0, true).await;
    rig.mount.push_to(3.0, sun.1 - 30.0);
    rig.sync().await;
    let target = horizontal_target(&rig, 40.0, sun.1 + 30.0);
    rig.state
        .goto()
        .unwrap()
        .start_target(target)
        .await
        .unwrap();
    let progress = rig.finish(None, true).await;
    assert_eq!(progress.last().unwrap()["state"], "done", "{progress:?}");
    assert!(rig.mount.true_direction().alt > 39.0);
}

#[tokio::test(start_paused = true)]
async fn repeated_half_degree_slips_fail_after_five_corrections_with_error() {
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    rig.second().await;
    assert_eq!(rig.call("/api/goto", false).await.0, 200);
    let mut slipped = None;
    // Inject a physical 0.5-degree slip, counts unchanged, once after every
    // arrival. Doing so after the approach avoids two opposite per-axis losses
    // cancelling each other and accidentally testing a mount without a slip.
    let progress = rig
        .finish_with(Some(15.0), true, |value| {
            let number = value["correction"].as_u64();
            if value["state"] == "settling" && slipped != number {
                rig.mount.push_by(0.5, 0.0);
                slipped = number;
            }
        })
        .await;
    let last = progress.last().unwrap();
    assert_eq!(last["state"], "failed", "{progress:?}");
    assert_eq!(last["correction"], 5);
    assert!(last["error_arcsec"].as_f64().unwrap() >= 60.0);
    assert!(last["reason"].as_str().unwrap().contains("5 corrections"));
    assert!(!rig.stops.lock().unwrap().is_empty());
    let corrections: Vec<_> = progress
        .iter()
        .filter(|v| v["state"] == "correcting")
        .map(|v| v["correction"].as_u64().unwrap())
        .collect();
    assert_eq!(corrections, vec![1, 2, 3, 4, 5]);
}
#[tokio::test(start_paused = true)]
async fn hand_push_opens_an_epoch_and_goto_still_converges() {
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    rig.second().await;
    let before = rig.state.session().model(rig.now()).epochs.len();
    rig.mount.push_by(3.0, 0.0);
    for _ in 0..15 {
        rig.tick().await;
    }
    assert_eq!(
        rig.state.session().model(rig.now()).epochs.len(),
        before + 1
    );
    assert_eq!(rig.call("/api/goto", false).await.0, 200);
    let progress = rig.finish(Some(15.0), true).await;
    assert_eq!(progress.last().unwrap()["state"], "done", "{progress:?}");
    assert!(rig.error_arcsec("M42") < 60.0);
}
async fn moving(rig: &Rig) {
    rig.sync().await;
    assert_eq!(rig.call("/api/goto", false).await.0, 200);
    for _ in 0..100 {
        rig.tick().await;
        if !rig.stationary().await {
            return;
        }
    }
    panic!("accepted GoTo never moved");
}
#[tokio::test(start_paused = true)]
async fn foreign_origin_bodiless_stop_commands_both_axes_and_reports_observer() {
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    moving(&rig).await;
    for _ in 0..15 {
        rig.tick().await;
    }
    assert!(!rig.stationary().await);
    let at = rig.now();
    assert_eq!(rig.call("/api/stop", true).await.0, 200);
    let stop = *rig.stops.lock().unwrap().last().unwrap();
    assert!(stop - at <= 0.1);
    for _ in 0..15 {
        rig.tick().await;
    }
    assert!(rig.stationary().await);
    let result = rig.view().await;
    assert_eq!(result["goto"]["state"], "halted");
    assert!(result["goto"]["reason"]
        .as_str()
        .unwrap()
        .contains("observer"));
    // Cancellation must also prevent a delayed settle/solve from restarting motion.
    for _ in 0..20 {
        rig.tick().await;
        assert!(rig.stationary().await);
    }
}
#[tokio::test(start_paused = true)]
async fn silent_heartbeats_stop_at_one_second_and_report_lost_link() {
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    moving(&rig).await;
    let status = rig.state.link().unwrap().status().borrow().clone();
    let age = status.last_heartbeat_at.unwrap().elapsed().as_secs_f64();
    let last_heartbeat = rig.now() - age;
    rig.controls.set_unresponsive(true);
    for _ in 0..30 {
        rig.tick().await;
    }
    let stop = rig.stops.lock().unwrap()[0];
    assert!(
        (stop - last_heartbeat - 1.0).abs() <= 0.1,
        "{stop} versus {last_heartbeat}"
    );
    assert!(rig.stationary().await);
    let result = rig.view().await;
    assert_eq!(result["goto"]["state"], "halted");
    assert!(result["goto"]["reason"]
        .as_str()
        .unwrap()
        .contains("lost link"));
}
#[tokio::test(start_paused = true)]
async fn sunrise_tick_commands_stop_within_a_second_and_reports_sun_guard() {
    let dawn = crossing(-0.83, START + 8.0 * 3600.0, START + 12.0 * 3600.0, true);
    let rig = Rig::new(dawn - 6.0, 50.0, 120.0, true).await;
    rig.target("M81");
    moving(&rig).await;
    for _ in 0..100 {
        rig.tick().await;
        if rig.now() > dawn + 2.0 {
            break;
        }
    }
    let stops = rig.stops.lock().unwrap().clone();
    assert!(
        stops.iter().any(|at| *at >= dawn && *at - dawn <= 1.0),
        "{stops:?} dawn {dawn}"
    );
    assert!(rig.stationary().await);
    let result = rig.view().await;
    assert_eq!(result["goto"]["state"], "halted");
    assert!(result["goto"]["reason"]
        .as_str()
        .unwrap()
        .contains("Sun guard"));
}

#[tokio::test(start_paused = true)]
async fn continuously_observed_cable_wind_survives_real_simulator_power_cycle() {
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    rig.sync().await;
    rig.move_to([350 * 4500, 0]).await;
    for _ in 0..15 {
        rig.tick().await;
    }
    let before = rig.state.session().model(rig.now()).epochs.len();
    rig.mount.power_cycle();
    for _ in 0..15 {
        rig.tick().await;
    }
    assert_eq!(rig.mount.state(Axis::Azimuth).await.position, 0);
    assert_eq!(
        rig.state.session().model(rig.now()).epochs.len(),
        before + 1
    );
    let direction = rig.mount.true_direction();
    let target = horizontal_target(&rig, direction.alt, direction.az + 30.0);
    rig.state
        .goto()
        .unwrap()
        .start_target(target)
        .await
        .unwrap();
    rig.tick().await;
    assert!(rig.mount.state(Axis::Azimuth).await.rate_deg_per_s < 0.0);
    // An erroneous clockwise +30-degree plan would have a positive rate here.
    rig.call("/api/stop", true).await;
    for _ in 0..15 {
        rig.tick().await;
    }
    assert!(rig.stationary().await);
}
