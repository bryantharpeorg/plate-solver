#![allow(dead_code)]
//! Oracles: rendered catalog stars, simulator mechanics and true eyepiece,
//! sampled every 0.1 s in paused time; angular acceptance is strictly 60 arcseconds.
#[path = "support/goto.rs"]
mod api;
use api::*;
use finder::{api::AppState, ephemeris, samples::Samples, session::Aim, sky};
use scope_node::mount::{Axis, Mount};
use serde_json::json;
use std::{sync::Arc, time::Duration};

// Every negative motion assertion first proves the connected mount can move.
async fn accepted_control(rig: &Rig) {
    rig.sync().await;
    assert!(rig.view().await["mount"]["connected"].as_bool().unwrap());
    assert_eq!(rig.call("/api/goto", false).await.0, 200);
    for _ in 0..100 {
        rig.tick().await;
        if !rig.stationary().await {
            break;
        }
    }
    assert!(!rig.stationary().await);
    assert_eq!(rig.call("/api/stop", false).await.0, 200);
    for _ in 0..15 {
        rig.tick().await;
    }
    assert!(rig.stationary().await);
}
async fn refused(rig: &Rig, status: u16, reason: &str, foreign: bool) {
    let before = [
        rig.mount.state(Axis::Azimuth).await.position,
        rig.mount.state(Axis::Altitude).await.position,
    ];
    let (answer, body) = rig.call("/api/goto", foreign).await;
    assert_eq!(answer, status, "{body}");
    assert!(body["error"].as_str().unwrap().contains(reason), "{body}");
    for _ in 0..15 {
        rig.tick().await;
        assert!(rig.stationary().await);
    }
    assert_eq!(
        [
            rig.mount.state(Axis::Azimuth).await.position,
            rig.mount.state(Axis::Altitude).await.position
        ],
        before
    );
}
#[tokio::test(start_paused = true)]
async fn approach_to_fifteen_point_two_degrees_is_refused_without_motion() {
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    accepted_control(&rig).await;
    rig.site_for_altitude("M42", 15.2);
    for _ in 0..15 {
        rig.tick().await;
    }
    refused(
        &rig,
        409,
        "approach would go below the minimum altitude",
        false,
    )
    .await;
}
#[tokio::test(start_paused = true)]
async fn sun_five_degrees_up_is_refused_without_motion() {
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    accepted_control(&rig).await;
    let (ra, dec) = ephemeris::sun(sky::julian_date(rig.now()));
    let lon = (ra - sky::gmst_deg(sky::julian_date(rig.now())) + 180.0).rem_euclid(360.0) - 180.0;
    rig.state
        .session()
        .edit(
            serde_json::from_value(json!({"site":{"lat_deg":dec+85.0,"lon_deg":lon}})).unwrap(),
            rig.now(),
        )
        .unwrap();
    assert!((sun_truth(dec + 85.0, lon, rig.now()).0 - 5.0).abs() < 1e-6);
    refused(&rig, 409, "Sun is up", false).await;
}
#[tokio::test(start_paused = true)]
async fn m8_below_horizon_is_refused_without_motion() {
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    accepted_control(&rig).await;
    rig.target("M8");
    refused(&rig, 409, "M8 is below the horizon", false).await;
}
#[tokio::test(start_paused = true)]
async fn absent_mount_and_missing_sync_each_name_the_reason() {
    let rig = Rig::new(START, 50.0, 120.0, false).await;
    refused(&rig, 409, "no mount", false).await;
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    accepted_control(&rig).await;
    rig.controls.stop_frames();
    for _ in 0..5 {
        rig.tick().await;
    }
    rig.state.session().reset_model(false, rig.now()).unwrap();
    assert!(rig.state.session().model(rig.now()).syncs.is_empty());
    refused(&rig, 409, "no sync", false).await;
}
#[tokio::test(start_paused = true)]
async fn foreign_goto_is_forbidden_without_motion() {
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    accepted_control(&rig).await;
    refused(&rig, 403, "origin", true).await;
}

#[tokio::test(start_paused = true)]
async fn simulate_startup_connects_the_default_mount_and_serves_motion_and_stop() {
    let state = AppState::shared(DB.clone(), Samples::default())
        .with_simulation(START, finder::simulate::SimulationTransport::Memory)
        .await
        .unwrap();
    let sim = state.simulator().unwrap();
    let mount = sim.mount().expect("simulate mode must connect a mount");
    // The startup wiring uses US1's default mechanics, not a special test mount.
    assert_eq!(mount.axes()[0].counts_per_revolution, 1_620_000);
    sim.push_to(50.0, 120.0);
    state
        .session()
        .edit(
            serde_json::from_value(json!({
                "site":{"lat_deg":40,"lon_deg":-75}, "target":"M42",
                "camera_settings":{"exposure_ms":200,"gain":1}
            }))
            .unwrap(),
            START,
        )
        .unwrap();
    state
        .session()
        .set_aim(Some(Aim { x: 700.0, y: 420.0 }), START)
        .unwrap();
    let state = state.with_live_solver(Arc::new(Stars));
    for _ in 0..100 {
        if !state.session().model(sim.clock().now()).syncs.is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(!state.session().model(sim.clock().now()).syncs.is_empty());
    let mut client = state.link().unwrap().mount().unwrap();
    assert_eq!(client.describe_mount().await.unwrap().axes.len(), 2);
    state.goto().unwrap().start().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(mount.state(Axis::Azimuth).await.moving || mount.state(Axis::Altitude).await.moving);
    state.goto().unwrap().stop().await;
    tokio::time::sleep(Duration::from_millis(1500)).await;
    assert!(!mount.state(Axis::Azimuth).await.moving);
    assert!(!mount.state(Axis::Altitude).await.moving);
}
