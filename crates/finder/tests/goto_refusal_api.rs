//! Oracle: the real GoTo route's 409 error, with exact simulator axis counts.
//! Every refusal first proves an accepted GoTo moves the connected mount.
#[path = "goto_api.rs"]
mod api;
use api::Rig;
use scope_node::mount::{Axis, Mount};
use serde_json::json;

const START: f64 = 1_768_528_800.0;

async fn positions(rig: &Rig) -> [i32; 2] {
    [
        rig.mount.state(Axis::Azimuth).await.position,
        rig.mount.state(Axis::Altitude).await.position,
    ]
}

async fn accepted(rig: &Rig) {
    rig.sync().await;
    let before = positions(rig).await;
    let session = rig.view().await;
    assert_eq!(session["mount"]["connected"], true);
    assert!(session.get("goto_refusal").is_none(), "{session}");
    assert_eq!(positions(rig).await, before, "reading must not move axes");
    for _ in 0..15 {
        rig.tick().await;
        assert!(rig.stationary().await);
        assert_eq!(positions(rig).await, before);
    }
    assert_eq!(rig.call("/api/goto", false).await.0, 200);
    for _ in 0..100 {
        rig.tick().await;
        if !rig.stationary().await {
            break;
        }
    }
    assert!(!rig.stationary().await);
    let running = rig.view().await;
    assert_eq!(running["goto"]["state"], "slewing");
    assert!(running.get("goto_refusal").is_none());
    assert_eq!(rig.call("/api/stop", false).await.0, 200);
    for _ in 0..15 {
        rig.tick().await;
    }
    assert!(rig.stationary().await);
}

async fn refused(rig: &Rig, reason: &str) {
    let before = positions(rig).await;
    let session = rig.view().await;
    let (status, body) = rig.call("/api/goto", false).await;
    assert_eq!(status, 409, "{body}");
    assert!(body["error"].as_str().unwrap().contains(reason), "{body}");
    assert_eq!(session["goto_refusal"], body["error"]);
    for _ in 0..15 {
        rig.tick().await;
        assert!(rig.stationary().await);
        assert_eq!(positions(rig).await, before);
    }
}

#[tokio::test(start_paused = true)]
async fn below_horizon_and_low_approach_match_the_route_without_moving() {
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    accepted(&rig).await;
    rig.target("M8");
    refused(&rig, "M8 is below the horizon").await;
    rig.site_for_altitude("M42", 15.2);
    refused(&rig, "approach would go below the minimum altitude").await;
}

#[tokio::test(start_paused = true)]
async fn sun_up_matches_the_route_even_when_target_guidance_is_pointable() {
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    accepted(&rig).await;
    let (ra, dec) = finder::ephemeris::sun(finder::sky::julian_date(rig.now()));
    let lon = (ra - finder::sky::gmst_deg(finder::sky::julian_date(rig.now())) + 180.0)
        .rem_euclid(360.0)
        - 180.0;
    rig.state
        .session()
        .edit(
            serde_json::from_value(
                json!({"site":{"lat_deg":dec+85.0,"lon_deg":lon},"target":"M31"}),
            )
            .unwrap(),
            rig.now(),
        )
        .unwrap();
    assert!((api::sun_truth(dec + 85.0, lon, rig.now()).0 - 5.0).abs() < 1e-6);
    assert_eq!(rig.view().await["guidance"]["class"], "pointable");
    refused(&rig, "Sun is up").await;
}

#[tokio::test(start_paused = true)]
async fn no_sync_matches_the_route_without_moving() {
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    accepted(&rig).await;
    rig.controls.stop_frames();
    for _ in 0..5 {
        rig.tick().await;
    }
    rig.state.session().reset_model(false, rig.now()).unwrap();
    assert!(rig.state.session().model(rig.now()).syncs.is_empty());
    refused(&rig, "no sync yet").await;
    rig.state
        .session()
        .edit(
            serde_json::from_value(json!({"target":null})).unwrap(),
            rig.now(),
        )
        .unwrap();
    refused(&rig, "no target is selected").await;
}

#[tokio::test(start_paused = true)]
async fn no_mount_omits_refusal_and_second_sync_omits_it_while_running() {
    let rig = Rig::new(START, 50.0, 120.0, false).await;
    assert!(rig.view().await.get("goto_refusal").is_none());
    assert_eq!(rig.call("/api/goto", false).await.0, 409);
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    rig.sync().await;
    assert_eq!(rig.call("/api/model/second-sync", false).await.0, 200);
    let session = rig.view().await;
    assert_eq!(session["goto"]["state"], "slewing");
    assert!(session.get("goto_refusal").is_none());
    rig.call("/api/stop", false).await;
}
