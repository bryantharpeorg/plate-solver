//! Router oracles: the 10-pixel snap radius, unchanged durable aim on refusal,
//! and the solved camera centre after reset (1e-10 degrees).
#[path = "live/support.rs"]
mod support;
use axum::{body::Body, http::Request};
use finder::session::Aim;
use serde_json::json;
use support::*;
use tower::ServiceExt;

#[tokio::test(start_paused = true)]
async fn rejected_tap_keeps_aim_and_reset_returns_pointings_to_the_camera_centre() {
    let sim = sim();
    let state = state(&sim, None, true);
    let mut events = state.live().unwrap().subscribe();
    let app = finder::router(state);
    let first = event(&mut events).await;
    let id = first.preview_seq.unwrap();
    // A far-off pixel is independently more than ten pixels from every reported star.
    assert!(first
        .report
        .detected
        .iter()
        .all(|s| (s[0] + 100.0).hypot(s[1] + 100.0) > 10.0));
    let (status, answer) = request(
        &app,
        "POST",
        "/api/aim",
        json!({"frame_id":id,"x":-100,"y":-100}),
    )
    .await;
    assert_eq!(status, 422);
    assert_eq!(
        answer,
        json!({"error":"no star is close enough to the tap"})
    );
    let (_, read) = request(&app, "GET", "/api/session", json!(null)).await;
    assert_eq!(read["aim"], json!({"x":700.0,"y":420.0}));
    let (status, _) = request(
        &app,
        "POST",
        "/api/aim",
        json!({"frame_id":999999,"x":700,"y":420}),
    )
    .await;
    assert_eq!(status, 409);
    let (status, read) = request(&app, "DELETE", "/api/aim", json!(null)).await;
    assert_eq!(status, 200);
    assert!(read.get("aim").is_none());
    event(&mut events).await;
    let (_, read) = request(&app, "GET", "/api/session", json!(null)).await;
    let p = &read["pointing"];
    let center = p["frame"]["center"].as_array().unwrap();
    let (ra, dec) = math_core::UnitVector {
        x: center[0].as_f64().unwrap(),
        y: center[1].as_f64().unwrap(),
        z: center[2].as_f64().unwrap(),
    }
    .to_radec();
    assert!((p["ra_deg"].as_f64().unwrap() - ra.to_degrees()).abs() < 1e-10);
    assert!((p["dec_deg"].as_f64().unwrap() - dec.to_degrees()).abs() < 1e-10);
}
#[tokio::test(start_paused = true)]
async fn aim_writes_enforce_origin_json_and_strict_shape_and_preview_retention_is_eight() {
    let sim = sim();
    let state = state(&sim, Some(std::sync::Arc::new(Fail)), true);
    let live = state.live().unwrap();
    let mut events = live.subscribe();
    let app = finder::router(state);
    let first = event(&mut events).await;
    let id = first.preview_seq.unwrap();
    for method in ["POST", "DELETE"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri("/api/aim")
                    .header("host", "localhost")
                    .header("origin", "http://foreign.example")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({"frame_id":id,"x":700,"y":420}).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), 403);
    }
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/aim",
            json!({"frame_id":id,"x":700,"y":420,"extra":1})
        )
        .await
        .0,
        400
    );
    let response = app
        .clone()
        .oneshot(Request::post("/api/aim").body(Body::from("{}")).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), 415);
    let mut previews = vec![id];
    while previews.len() < 9 {
        let report = event(&mut events).await;
        if let Some(next) = report.preview_seq {
            if previews.last() != Some(&next) {
                previews.push(next);
            }
        }
    }
    assert!(live.preview(id).is_none());
    for id in &previews[1..] {
        assert!(live.preview(*id).is_some());
    }
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/aim",
            json!({"frame_id":id,"x":700,"y":420})
        )
        .await
        .0,
        409
    );
    let session = finder::session::Session::default();
    assert!(session
        .set_aim(
            Some(Aim {
                x: f64::NAN,
                y: 420.0
            }),
            0.0
        )
        .is_err());
}
