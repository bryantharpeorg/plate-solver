//! Exact HTTP oracles: write-only credentials, atomic rejected edits, and 012's
//! same-origin guard. Camera limits come from the simulator's Describe response.
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use finder::{link::Transport, router, samples::Samples, session::FixedClock, AppState};
use pattern_database::PatternDatabase;
use serde_json::{json, Value};
use simulator::{SimNode, Simulator, Site, VirtualClock};
use std::{
    path::Path,
    sync::{Arc, LazyLock},
    time::Duration,
};
use tower::ServiceExt;
static DB: LazyLock<Arc<PatternDatabase>> = LazyLock::new(|| {
    Arc::new(
        pattern_database::load_from_path(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../reference-solutions/cedar-solve/tetra3/data/default_database.npz"),
        )
        .unwrap(),
    )
});
fn state() -> AppState {
    AppState::shared(DB.clone(), Samples::default())
        .with_clock(Arc::new(FixedClock(1_768_528_800.0)))
}
async fn send(app: &Router, edit: Option<Value>, origin: Option<&str>) -> (StatusCode, Value) {
    let mut req = if edit.is_some() {
        Request::put("/api/session")
    } else {
        Request::get("/api/session")
    }
    .header("host", "localhost:8080")
    .header("content-type", "application/json");
    if let Some(origin) = origin {
        req = req.header("origin", origin);
    }
    let response = app
        .clone()
        .oneshot(
            req.body(edit.map_or(Body::empty(), |v| Body::from(v.to_string())))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}
#[tokio::test]
async fn token_is_write_only_partial_node_edits_and_clear_work() {
    let app = router(state());
    let (status, read) = send(
        &app,
        Some(json!({"node":{"address":"192.168.1.40:50051","token":"test-token"}})),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(read["node"]["address"], "192.168.1.40:50051");
    assert_eq!(read["node"]["token_set"], true);
    assert_eq!(read["node"]["connected"], false);
    assert!(!read.to_string().contains("test-token"));
    assert_eq!(send(&app, None, None).await.1, read);
    let read = send(
        &app,
        Some(json!({"node":{"address":"camera.local:1234"}})),
        None,
    )
    .await
    .1;
    assert_eq!(read["node"]["token_set"], true);
    let read = send(&app, Some(json!({"node":null})), None).await.1;
    assert!(read.get("node").is_none());
    assert!(read.get("camera_settings").is_none());
}
#[tokio::test]
async fn invalid_addresses_tokens_settings_and_unknown_fields_are_atomic_400s() {
    let app = router(state());
    let before = send(&app, None, None).await.1;
    for node in [
        json!({"address":"not an address"}),
        json!({"address":"host:0"}),
        json!({"address":"host:65536"}),
        json!({"address":"http://host:42"}),
        json!({"address":"host:42/path"}),
        json!({"address":"user@host:42"}),
        json!({"address":"host"}),
        json!({"token":""}),
        json!({"token":"bad\ntoken"}),
        json!({"adress":"host:1234"}),
    ] {
        let (status, error) = send(&app, Some(json!({"min_alt_deg":30,"node":node})), None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{error}");
        assert!(error["error"].is_string());
        assert_eq!(send(&app, None, None).await.1, before);
    }
    for settings in [
        json!({"exposure_ms":0}),
        json!({"gain":-1}),
        json!({"sigma":0.9}),
        json!({"sigma":51}),
        json!({"exposure_ms":"manual"}),
        json!({"gain":null}),
        json!({"unknown":1}),
    ] {
        let (status, error) = send(
            &app,
            Some(json!({"min_alt_deg":30,"camera_settings":settings})),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{error}");
        assert!(error["error"].is_string());
        assert_eq!(send(&app, None, None).await.1, before);
    }
}
#[tokio::test]
async fn foreign_origin_cannot_edit_node_or_camera_settings() {
    let app = router(state());
    let before = send(&app, None, None).await.1;
    let (status,error)=send(&app,Some(json!({"node":{"address":"camera.local:50051","token":"test-token"},"camera_settings":{"exposure_ms":200}})),Some("http://elsewhere.example")).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(error["error"].is_string());
    assert_eq!(send(&app, None, None).await.1, before);
}
#[tokio::test(start_paused = true)]
async fn connected_camera_refuses_both_ranges_without_any_edit() {
    let sim = Simulator::new(
        DB.clone(),
        Site::default(),
        VirtualClock::new(1_768_528_800.0),
    );
    let node = Arc::new(SimNode::new(sim, "test-token", Default::default()).unwrap());
    let transport = Transport::connector(move || {
        let node = node.clone();
        async move {
            let (client, server) = tokio::io::duplex(65536);
            tokio::spawn(async move {
                scope_link::memory::serve_on(server, node.service().into_server())
                    .await
                    .unwrap();
            });
            Ok::<_, std::io::Error>(client)
        }
    });
    let app = router(state().with_link(transport));
    assert_eq!(send(&app,Some(json!({"node":{"address":"camera.local:50051","token":"test-token"},"camera_settings":{"exposure_ms":200,"gain":4}})),Some("http://localhost:8080")).await.0,StatusCode::OK);
    let mut ready = false;
    for _ in 0..100 {
        if send(&app, None, None).await.1["node"]["connected"] == true {
            ready = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    assert!(ready);
    // Stop time-varying fields at the fixed clock; no frame has arrived yet.
    for (field, value, range) in [("exposure_ms", 2001, "1–2000"), ("gain", 17, "1–16")] {
        let before = send(&app, None, None).await.1;
        let (status, error) = send(
            &app,
            Some(json!({"min_alt_deg":30,"camera_settings":{field:value}})),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(error["error"].as_str().unwrap().contains(range), "{error}");
        assert_eq!(send(&app, None, None).await.1, before);
    }
}
