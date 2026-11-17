//! The session's chart rotation, through the router in process. Oracles are
//! hand-computed quarter turns (US1-S1) and the JSON grammar of a number (US1-S2).

use std::path::Path;
use std::sync::{Arc, LazyLock};

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use axum::Router;
use finder::samples::Samples;
use finder::session::{FixedClock, Session};
use finder::{router, AppState};
use pattern_database::PatternDatabase;
use serde_json::{json, Value};
use tower::ServiceExt;

static DB: LazyLock<Arc<PatternDatabase>> = LazyLock::new(|| {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../reference-solutions/cedar-solve/tetra3/data/default_database.npz");
    Arc::new(pattern_database::load_from_path(&path).unwrap())
});

fn app() -> Router {
    router(
        AppState::shared(DB.clone(), Samples::default())
            .with_session(Session::default())
            .with_clock(Arc::new(FixedClock(1768528800.0))),
    )
}

async fn request(app: &Router, edit: Option<Value>) -> (StatusCode, Value) {
    let req = match edit {
        Some(edit) => Request::put("/api/session")
            .header("content-type", "application/json")
            .body(Body::from(edit.to_string()))
            .unwrap(),
        None => Request::get("/api/session").body(Body::empty()).unwrap(),
    };
    let response = app.clone().oneshot(req).await.unwrap();
    let status = response.status();
    assert_eq!(response.headers()["content-type"], "application/json");
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

async fn put(app: &Router, edit: Value) -> Value {
    let (status, body) = request(app, Some(edit)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body
}

#[tokio::test]
async fn rotations_are_stored_normalised_within_the_circle() {
    let app = app();
    // Oracle: hand-computed quarter turns, and rem_euclid's two zero edges.
    for (edit, expected) in [
        (450.0, 90.0),
        (-90.0, 270.0),
        (0.0, 0.0),
        (-1e-20, 0.0),
        (-0.0, 0.0),
    ] {
        let body = put(&app, json!({"chart_rotation_deg": edit})).await;
        assert_eq!(body["chart_rotation_deg"], expected, "{edit} -> {body}");
        let (status, read) = request(&app, None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(read["chart_rotation_deg"], expected, "{edit} -> {read}");
        // A negative zero would keep its sign through JSON: the stored zero is positive.
        if expected == 0.0 {
            assert_eq!(
                serde_json::to_string(&read["chart_rotation_deg"]).unwrap(),
                "0.0",
                "{edit} -> {read}"
            );
        }
    }
}

#[tokio::test]
async fn a_rotation_must_be_a_number_or_null_and_bad_edits_change_nothing() {
    let console = app();
    let before = put(&console, json!({"chart_rotation_deg": 30})).await;
    for bad in [json!("x"), json!(true), json!([90]), json!("90")] {
        let (status, error) = request(&console, Some(json!({"chart_rotation_deg": bad}))).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{bad} -> {error}");
        assert!(error["error"].as_str().is_some(), "{bad} -> {error}");
        let (status, read) = request(&console, None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(read, before, "{bad} changed the session");
    }
    // A console that has never been told an angle reads without the key.
    let (status, read) = request(&app(), None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        !read.as_object().unwrap().contains_key("chart_rotation_deg"),
        "{read}"
    );
}
