//! SSE oracle: exact serialization of each independently received live report;
//! preview bytes equal the node's JPEG, and a decoder confirms its format.
#[path = "live/support.rs"]
mod support;
use axum::{
    body::{to_bytes, Body},
    http::Request,
};
use futures_util::StreamExt;
use serde_json::{json, Value};
use std::time::Duration;
use support::*;
use tower::ServiceExt;

#[tokio::test(start_paused = true)]
async fn two_solves_send_two_live_events_and_the_retained_jpeg_is_served() {
    let sim = sim();
    let state = state(&sim, None, false);
    let live = state.live().unwrap();
    let mut reports = live.subscribe();
    let app = finder::router(state);
    let response = app
        .clone()
        .oneshot(Request::get("/api/live").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers()["content-type"], "text/event-stream");
    let mut body = response.into_body().into_data_stream();
    for _ in 0..2 {
        let expected = event(&mut reports).await;
        assert_eq!(expected.report.status, finder::solve::Status::MatchFound);
        let chunk = tokio::time::timeout(Duration::from_secs(2), body.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let text = std::str::from_utf8(&chunk).unwrap();
        assert!(text.lines().any(|line| line == "event: live"), "{text}");
        let data = text
            .lines()
            .find_map(|line| line.strip_prefix("data: "))
            .unwrap();
        let actual: Value = serde_json::from_str(data).unwrap();
        assert!(
            data == serde_json::to_string(&expected).unwrap(),
            "event differs from live report for frame {}",
            expected.frame_id
        );
        assert_eq!(actual["report"]["name"], Value::Null);
        let seq = expected.preview_seq.unwrap();
        let preview = app
            .clone()
            .oneshot(
                Request::get(format!("/api/live/preview.jpg?seq={seq}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(preview.status(), 200);
        assert_eq!(preview.headers()["content-type"], "image/jpeg");
        let bytes = to_bytes(preview.into_body(), usize::MAX).await.unwrap();
        assert_eq!(bytes.as_ref(), live.preview(seq).unwrap().jpeg);
        assert_eq!(
            image::guess_format(&bytes).unwrap(),
            image::ImageFormat::Jpeg
        );
        image::load_from_memory(&bytes).unwrap();
    }
    assert_eq!(
        request(&app, "GET", "/api/live/preview.jpg?seq=999999", json!(null))
            .await
            .0,
        404
    );
    sim.stop_frames();
    tokio::time::sleep(Duration::from_secs(3)).await;
    let response = app
        .clone()
        .oneshot(Request::get("/api/live").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let mut idle = response.into_body().into_data_stream();
    assert!(tokio::time::timeout(Duration::from_secs(14), idle.next())
        .await
        .is_err());
    let comment = tokio::time::timeout(Duration::from_secs(2), idle.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(comment.starts_with(b":"));
}

#[tokio::test(start_paused = true)]
async fn stream_and_preview_without_a_live_loop_answer_not_found() {
    let app = finder::router(finder::AppState::shared(
        DB.clone(),
        finder::samples::Samples::default(),
    ));
    for route in ["/api/live", "/api/live/preview.jpg?seq=1"] {
        assert_eq!(request(&app, "GET", route, json!(null)).await.0, 404);
    }
}
