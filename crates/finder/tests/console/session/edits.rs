//! Shared-session contract through the router. All values are exact (zero tolerance),
//! from FR-009/010, the pinned OpenNGC ids and the fixed Unix-time fixture.

use std::path::Path;
use std::process::Command;
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
async fn fresh_session_and_fixed_clock_are_exact_on_repeated_reads() {
    let app = app();
    let expected = json!({"site": null, "target": null, "pointing": null,
        "min_alt_deg": 15.0, "time": 1768528800.0});
    for _ in 0..2 {
        assert_eq!(
            request(&app, None).await,
            (StatusCode::OK, expected.clone())
        );
    }
}

#[tokio::test]
async fn one_device_writes_and_another_reads_the_same_session() {
    let a = app();
    let b = a.clone();
    let edited = put(
        &a,
        json!({"site": {"lat_deg": 40, "lon_deg": -75}, "target": "M42"}),
    )
    .await;
    assert_eq!(edited["site"], json!({"lat_deg": 40.0, "lon_deg": -75.0}));
    assert_eq!(edited["target"]["id"], "NGC 1976");
    assert_eq!(edited["target"]["messier"], "M42");
    assert_eq!(request(&b, None).await, (StatusCode::OK, edited));
}

#[tokio::test]
async fn invalid_edits_are_json_errors_and_leave_every_field_unchanged() {
    let app = app();
    let before = put(
        &app,
        json!({"site": {"lat_deg": 40, "lon_deg": -75}, "target": "M31"}),
    )
    .await;
    for (edit, field) in [
        (
            json!({"site": {"lat_deg": 95, "lon_deg": 0}, "target": "M42"}),
            "lat_deg",
        ),
        (json!({"site": {"lat_deg": -91, "lon_deg": 0}}), "lat_deg"),
        (json!({"site": {"lat_deg": 0, "lon_deg": 181}}), "lon_deg"),
        (json!({"site": {"lat_deg": 0, "lon_deg": -181}}), "lon_deg"),
        (json!({"min_alt_deg": 85, "target": null}), "min_alt_deg"),
        (json!({"min_alt_deg": -1}), "min_alt_deg"),
        (json!({"sun_guard": false, "min_alt_deg": 30}), "sun_guard"),
        (
            json!({"site": {"lat_deg": 40, "lon_deg": -75, "height": 0}}),
            "height",
        ),
        (json!({"min_alt_deg": null}), "min_alt_deg"),
        (json!({"target": 42}), "target"),
    ] {
        let (status, error) = request(&app, Some(edit)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{error}");
        assert!(error["error"].as_str().unwrap().contains(field), "{error}");
        assert_eq!(request(&app, None).await.1, before);
    }
}

#[tokio::test]
async fn unknown_target_is_a_404_and_does_not_apply_other_edits() {
    let app = app();
    let before = put(&app, json!({"target": "M31"})).await;
    let (status, error) = request(
        &app,
        Some(json!({"target": "NGC 99999", "min_alt_deg": 30})),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(error["error"].as_str().unwrap().contains("target"));
    assert_eq!(request(&app, None).await.1, before);
}

#[tokio::test]
async fn partial_edits_and_null_keep_every_other_field() {
    let app = app();
    let mut expected = put(
        &app,
        json!({"site": {"lat_deg": 40, "lon_deg": -75}, "target": "M42"}),
    )
    .await;
    expected["min_alt_deg"] = json!(30.0);
    assert_eq!(put(&app, json!({"min_alt_deg": 30})).await, expected);
    expected["target"] = Value::Null;
    expected.as_object_mut().unwrap().remove("guidance");
    assert_eq!(put(&app, json!({"target": null})).await, expected);
    expected["site"] = Value::Null;
    assert_eq!(put(&app, json!({"site": null})).await, expected);
    assert_eq!(put(&app, json!({})).await, expected);
}

#[tokio::test]
async fn range_endpoints_are_inclusive() {
    let app = app();
    for (lat, lon, min) in [(-90, -180, 0), (90, 180, 80)] {
        let body = put(
            &app,
            json!({"site": {"lat_deg": lat, "lon_deg": lon}, "min_alt_deg": min}),
        )
        .await;
        assert_eq!(body["site"]["lat_deg"], f64::from(lat));
        assert_eq!(body["site"]["lon_deg"], f64::from(lon));
        assert_eq!(body["min_alt_deg"], f64::from(min));
    }
}

#[tokio::test]
async fn library_without_a_state_file_leaves_home_and_xdg_empty() {
    const CHILD: &str = "FINDER_SESSION_LIBRARY_CHILD";
    if std::env::var_os(CHILD).is_some() {
        let app = app();
        put(&app, json!({"target": "M42", "min_alt_deg": 30})).await;
        return;
    }
    // Child-only environment: parallel tests never mutate the process environment.
    let empty = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target")
        .join(format!("session-library-home-{}", std::process::id()));
    std::fs::create_dir_all(&empty).unwrap();
    assert_eq!(std::fs::read_dir(&empty).unwrap().count(), 0);
    let out = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            &format!(
                "{}::library_without_a_state_file_leaves_home_and_xdg_empty",
                module_path!().split_once("::").unwrap().1
            ),
            "--nocapture",
        ])
        .env(CHILD, "1")
        .env("HOME", &empty)
        .env("XDG_STATE_HOME", &empty)
        .output()
        .unwrap();
    let count = std::fs::read_dir(&empty).unwrap().count();
    std::fs::remove_dir_all(&empty).unwrap();
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("test result: ok. 1 passed"),
        "the child ran no test: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(count, 0);
}

#[tokio::test]
async fn simultaneous_partial_edits_keep_both_devices_changes() {
    let app = app();
    let (site, target) = tokio::join!(
        put(&app, json!({"site": {"lat_deg": 40, "lon_deg": -75}})),
        put(&app, json!({"target": "M42"})),
    );
    assert!(site["site"].is_object());
    assert!(target["target"].is_object());
    let body = request(&app, None).await.1;
    assert_eq!(body["site"], json!({"lat_deg": 40.0, "lon_deg": -75.0}));
    assert_eq!(body["target"]["id"], "NGC 1976");
}

#[tokio::test]
async fn similar_without_explicit_time_uses_the_injected_clock() {
    let app = app();
    let mut bodies = Vec::new();
    for suffix in ["", "&time=1768528800"] {
        let response = app
            .clone()
            .oneshot(
                Request::get(format!("/api/similar?id=M42&lat=40&lon=-75{suffix}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        bodies.push(serde_json::from_slice::<Value>(&bytes).unwrap());
    }
    // Oracle: the existing endpoint with the fixture's explicit time. Zero tolerance.
    assert_eq!(bodies[0], bodies[1]);
}
