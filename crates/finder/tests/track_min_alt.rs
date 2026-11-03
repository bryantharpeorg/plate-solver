//! A minimum altitude raised through `PUT /api/session` while M42 is tracked
//! reaches the running tracker: tracking ends as too low and both axes rest.
#[path = "goto_api.rs"]
mod goto_api;
use axum::{
    body::{to_bytes, Body},
    http::Request,
};
use goto_api::Rig;
use serde_json::{json, Value};
use tower::ServiceExt;
const START: f64 = 1_768_528_800.0;

async fn write(rig: &Rig, method: &str, path: &str, body: Value) -> Value {
    let request = Request::builder()
        .method(method)
        .uri(path)
        .header("host", "localhost")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let response = rig.app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), 200);
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test(start_paused = true)]
async fn raised_minimum_altitude_ends_running_tracking_as_too_low() {
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    rig.sync().await;
    write(&rig, "POST", "/api/track", json!({"on": true})).await;
    for _ in 0..5000 {
        if rig.view().await["track"]["state"] == "tracking" {
            break;
        }
        rig.tick().await;
    }
    assert_eq!(rig.view().await["track"]["state"], "tracking");
    for _ in 0..20 {
        rig.tick().await;
    }
    assert!(
        !rig.stationary().await,
        "tracking must move a connected mount"
    );

    // M42 never rises above 45° at latitude 40°, so 80° is above it now.
    let body = write(&rig, "PUT", "/api/session", json!({"min_alt_deg": 80.0})).await;
    assert_eq!(body["min_alt_deg"], 80.0);
    let edited = rig.now();
    while rig.view().await["track"]["state"] != "off" {
        assert!(rig.now() - edited < 1.0, "{}", rig.view().await);
        rig.tick().await;
    }
    while !rig.stationary().await {
        assert!(rig.now() - edited < 1.5);
        rig.tick().await;
    }
    let body = rig.view().await;
    assert_eq!(body["track"]["ended_reason"], "too_low");
    assert_eq!(body["track"]["wanted"], false);
}
