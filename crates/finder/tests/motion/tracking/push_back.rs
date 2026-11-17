//! Oracles: the simulator's true eyepiece direction and the session's own view,
//! sampled in paused time; angular acceptance is 60 arcseconds. Going back is
//! the GoTo's own done tolerance, read through the track state it arms.
use crate::support::{goto::Rig, tracker};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use finder::{model::separation, targets};
use serde_json::{json, Value};
use std::time::Duration;
use tower::ServiceExt;
const START: f64 = 1_768_528_800.0;
const TICKS_PER_SECOND: usize = 10;

async fn write(app: &Router, method: &str, path: &str, body: Value) -> (StatusCode, Value) {
    let request = Request::builder()
        .method(method)
        .uri(path)
        .header("host", "localhost")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

async fn track(rig: &Rig, on: bool) -> Value {
    let (status, body) = write(&rig.app, "POST", "/api/track", json!({"on":on})).await;
    assert_eq!(status, 200, "{body}");
    body
}

/// The J2000 place the tube points at now, from the simulator's own ray.
fn true_place(rig: &Rig) -> (f64, f64) {
    let direction = rig.mount.true_direction();
    let vector = tracker::equatorial(direction.alt, direction.az, rig.now());
    let (ra, dec) = vector.to_radec();
    (ra.to_degrees(), dec.to_degrees())
}

/// Read the session view every 100 ms of virtual time until the condition holds,
/// for at most 120 s of virtual time.
async fn until(rig: &Rig, mut condition: impl FnMut(&Value) -> bool) -> Value {
    let start = rig.now();
    loop {
        let view = rig.view().await;
        if condition(&view) {
            return view;
        }
        assert!(rig.now() - start < 120.0, "never came true: {view}");
        rig.tick().await;
    }
}

/// The rig on M42 as spec.md § User Scenarios defines it: centred as routes.rs's
/// `centred()`, then 30 s more of tracking so the mount's play is taken up.
async fn on_m42() -> Rig {
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    let target = targets::find("M42").unwrap();
    rig.sim.point_eyepiece_at(target.ra_deg, target.dec_deg);
    rig.sync().await;
    assert_eq!(rig.view().await["mount"]["connected"], true);
    // Track reads the newest solved pointing; wait for one solved on M42.
    for _ in 0..10 {
        let pointing = rig.view().await["pointing"].clone();
        let within = separation(
            pointing["ra_deg"].as_f64().unwrap(),
            pointing["dec_deg"].as_f64().unwrap(),
            target.ra_deg,
            target.dec_deg,
        ) < 1.0 / 60.0;
        if within {
            break;
        }
        rig.tick().await;
    }
    let body = track(&rig, true).await;
    assert_eq!(body["track"]["state"], "tracking", "{body}");
    assert!(
        body.get("goto").is_none(),
        "a centred Track ran a GoTo: {body}"
    );
    assert_eq!(body["track"]["target"]["messier"], "M42", "{body}");
    assert_eq!(body["track"]["wanted"], true, "{body}");
    for _ in 0..20 {
        rig.tick().await;
        if !rig.stationary().await {
            break;
        }
    }
    assert!(
        !rig.stationary().await,
        "accepted Track did not move the mount"
    );
    for _ in 0..30 * TICKS_PER_SECOND {
        rig.tick().await;
    }
    assert_eq!(rig.view().await["track"]["state"], "tracking");
    rig
}

/// Track on with a push held answers armed with a GoTo under way, then tracks
/// the target with no push key and the eyepiece back on it (US2-S1, US2-S2).
async fn goes_back(rig: &Rig, target: &targets::Target) {
    let body = track(rig, true).await;
    assert_eq!(body["track"]["state"], "armed", "{body}");
    assert!(body["track"].get("push").is_none(), "{body}");
    assert!(
        matches!(
            body["goto"]["state"].as_str(),
            Some("slewing" | "solving" | "correcting")
        ),
        "no GoTo under way: {body}"
    );
    let view = until(rig, |view| view["track"]["state"] == "tracking").await;
    assert!(view["track"].get("push").is_none(), "{view}");
    let (ra, dec) = true_place(rig);
    assert!(
        separation(ra, dec, target.ra_deg, target.dec_deg) * 3600.0 <= 60.0,
        "the true place {ra} {dec} is not back on {} {}",
        target.ra_deg,
        target.dec_deg
    );
}

#[tokio::test(start_paused = true)]
async fn track_on_while_following_a_push_goes_back_to_the_target() {
    let rig = on_m42().await;
    let target = targets::find("M42").unwrap();
    rig.mount.push_by(0.0, -2.0);
    until(&rig, |view| {
        view["track"]["push"]["state"].as_str() == Some("following")
    })
    .await;
    goes_back(&rig, target).await;
    track(&rig, false).await;
}

#[tokio::test(start_paused = true)]
async fn track_on_while_moving_by_hand_goes_back_to_the_target() {
    let rig = on_m42().await;
    let target = targets::find("M42").unwrap();
    rig.mount.push_by(0.0, -1.0);
    until(&rig, |view| {
        view["track"]["push"]["state"].as_str() == Some("moving")
    })
    .await;
    goes_back(&rig, target).await;
    track(&rig, false).await;
}

#[tokio::test(start_paused = true)]
async fn track_on_with_no_push_stays_the_no_op_it_is_today() {
    let rig = on_m42().await;
    let before = rig.state.session().tracker().expect("a tracker");
    let body = track(&rig, true).await;
    assert_eq!(body["track"]["state"], "tracking", "{body}");
    assert!(body.get("goto").is_none(), "a GoTo started: {body}");
    assert!(body["track"].get("push").is_none(), "{body}");
    assert!(std::sync::Arc::ptr_eq(
        &before,
        &rig.state.session().tracker().expect("a tracker")
    ));
    for _ in 0..2 * TICKS_PER_SECOND {
        rig.tick().await;
    }
    let view = rig.view().await;
    assert_eq!(view["track"]["state"], "tracking", "{view}");
    assert!(view.get("goto").is_none(), "{view}");
    track(&rig, false).await;
}

#[tokio::test(start_paused = true)]
async fn track_on_for_another_target_goes_to_it() {
    let rig = on_m42().await;
    let target = targets::find("M43").unwrap();
    let (status, body) = write(&rig.app, "PUT", "/api/session", json!({"target": "M43"})).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["target"]["id"], target.id, "{body}");
    let body = track(&rig, true).await;
    assert_eq!(body["track"]["state"], "armed", "{body}");
    assert_eq!(body["track"]["target"]["id"], target.id, "{body}");
    assert!(
        matches!(
            body["goto"]["state"].as_str(),
            Some("slewing" | "solving" | "correcting")
        ),
        "no GoTo under way: {body}"
    );
    until(&rig, |view| view["track"]["state"] == "tracking").await;
    let (ra, dec) = true_place(&rig);
    assert!(
        separation(ra, dec, target.ra_deg, target.dec_deg) * 3600.0 <= 60.0,
        "the true place {ra} {dec} is not on {} {}",
        target.ra_deg,
        target.dec_deg
    );
    track(&rig, false).await;
}

#[tokio::test(start_paused = true)]
async fn track_off_during_the_wait_for_rest_answers_stopped_and_starts_no_goto() {
    let rig = on_m42().await;
    rig.mount.push_by(0.0, -2.0);
    until(&rig, |view| {
        view["track"]["push"]["state"].as_str() == Some("following")
    })
    .await;
    let app = rig.app.clone();
    let first =
        tokio::spawn(async move { write(&app, "POST", "/api/track", json!({"on":true})).await });
    tokio::time::sleep(Duration::from_millis(20)).await;
    let (status, off) = write(&rig.app, "POST", "/api/track", json!({"on":false})).await;
    assert_eq!(status, 200, "{off}");
    let (status, body) = first.await.unwrap();
    assert_eq!(status, 409, "{body}");
    assert_eq!(body["error"], "stopped", "{body}");
    let start = rig.now();
    while rig.now() - start < 5.0 {
        let view = rig.view().await;
        assert!(
            !matches!(
                view["goto"]["state"].as_str(),
                Some("slewing" | "solving" | "correcting")
            ),
            "a GoTo started: {view}"
        );
        rig.tick().await;
    }
    assert!(rig.stationary().await, "the axes did not come to rest");
}
