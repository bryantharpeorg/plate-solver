//! From the repository root, copy to crates/finder/tests/us8_capture.rs, run
//! cargo test -p finder --locked --test us8_capture record_tracking -- --nocapture,
//! then remove that temporary test. Captures the real router in paused time,
//! using 014's catalog-rendering/solver/in-process simulated mount harness.
#[path = "goto_api.rs"]
mod api;
use api::Rig;
use axum::{
    body::{to_bytes, Body},
    http::Request,
};
use serde_json::{json, Value};
use tower::ServiceExt;
async fn write(rig: &Rig, method: &str, path: &str, body: Value) -> Value {
    let response = rig
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("host", "localhost")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap()
}
#[tokio::test(start_paused = true)]
async fn record_tracking() {
    let rig = Rig::new(1_768_528_800.0, 50.0, 120.0, true).await;
    rig.sync().await;
    let armed = write(&rig, "POST", "/api/track", json!({"on":true})).await;
    assert_eq!(armed["track"]["state"], "armed");
    for _ in 0..5000 {
        if rig.view().await["track"]["state"] == "tracking" {
            break;
        }
        rig.tick().await;
    }
    assert_eq!(rig.view().await["track"]["state"], "tracking");
    for _ in 0..750 {
        rig.tick().await;
    }
    let no_camera = rig.view().await;
    let tracking = write(
        &rig,
        "PUT",
        "/api/session",
        json!({"imaging_camera":{
        "width_mm":23.5,"height_mm":15.6,"pitch_um":3.76}}),
    )
    .await;
    // At the pole cos(latitude)=0, so field rotation has no finite smear limit.
    let no_limit = write(
        &rig,
        "PUT",
        "/api/session",
        json!({"site":{"lat_deg":-90,"lon_deg":-75}}),
    )
    .await;
    assert_eq!(
        no_limit["track"]["readout"]["longest_exposure_s"],
        "no limit"
    );
    // A meridian target 1° from the zenith enters US6's keyhole wait.
    write(&rig, "POST", "/api/track", json!({"on":false})).await;
    rig.site_for_altitude("M42", 89.0);
    write(&rig, "POST", "/api/track", json!({"on":true})).await;
    let mut keyhole = Value::Null;
    for _ in 0..100 {
        rig.tick().await;
        let view = rig.view().await;
        if view["track"]["state"] == "keyhole_wait" {
            keyhole = view;
            break;
        }
    }
    assert_eq!(keyhole["track"]["state"], "keyhole_wait");
    write(&rig, "POST", "/api/track", json!({"on":false})).await;
    let target = finder::targets::find("M42").unwrap();
    let (alt, _) = finder::sky::alt_az(
        target.ra_deg,
        target.dec_deg,
        &finder::sky::Observer {
            lat: 40.0,
            lon: 15.0,
            time: rig.now(),
        },
    );
    assert!((1.0..80.0).contains(&alt));
    write(
        &rig,
        "PUT",
        "/api/session",
        json!({"site":{"lat_deg":40,"lon_deg":15},"min_alt_deg":alt-0.0001}),
    )
    .await;
    write(&rig, "POST", "/api/track", json!({"on":true})).await;
    for _ in 0..20 {
        rig.tick().await;
        if rig.view().await["track"]["state"] == "off" {
            break;
        }
    }
    let too_low = rig.view().await;
    assert_eq!(too_low["track"]["ended_reason"], "too_low");
    let captures = json!({"armed":armed,"tracking":tracking,"no_camera":no_camera,
        "no_limit":no_limit,"keyhole_wait":keyhole,"too_low":too_low});
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("web/src/test/fixtures/session-track.json");
    std::fs::write(
        path,
        serde_json::to_string_pretty(&captures).unwrap() + "\n",
    )
    .unwrap();
}
