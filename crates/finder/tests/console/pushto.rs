//! US5 API contract. Oracles: fixed console time, existing sky::alt_az (1e-9°),
//! cedar's orion2 report marks (1e-9 pixels/degrees), and the session minimum.

use std::path::Path;
use std::sync::{Arc, LazyLock};

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use axum::Router;
use finder::samples::Samples;
use finder::session::{FixedClock, Session};
use finder::sky::{alt_az, Observer};
use finder::{router, AppState};
use pattern_database::PatternDatabase;
use serde_json::{json, Value};
use tower::ServiceExt;

const TIME: f64 = 1_768_528_800.0; // 2026-01-16 02:00 UT
static DB: LazyLock<Arc<PatternDatabase>> = LazyLock::new(|| {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    Arc::new(
        pattern_database::load_from_path(
            &repo.join("reference-solutions/cedar-solve/tetra3/data/default_database.npz"),
        )
        .unwrap(),
    )
});

fn state(time: f64) -> AppState {
    let samples = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../reference-solutions/cedar-solve/examples/data/medium_fov");
    AppState::shared(DB.clone(), Samples::scan(&samples).unwrap())
        .with_clock(Arc::new(FixedClock(time)))
}

async fn send(app: &Router, request: Request<Body>) -> Value {
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&body).unwrap()
}

async fn get(app: &Router, uri: &str) -> Value {
    send(app, Request::get(uri).body(Body::empty()).unwrap()).await
}

async fn put(app: &Router, edit: Value) -> Value {
    send(
        app,
        Request::put("/api/session")
            .header("content-type", "application/json")
            .body(Body::from(edit.to_string()))
            .unwrap(),
    )
    .await
}

async fn solved() -> (AppState, Router, Value) {
    let state = state(TIME);
    let app = router(state.clone());
    put(
        &app,
        json!({"site": {"lat_deg": 40, "lon_deg": -75}, "target": "M78"}),
    )
    .await;
    let report = send(&app, Request::post("/api/solve")
        .header("content-type", "multipart/form-data; boundary=pushto-test")
        .body(Body::from("--pushto-test\r\nContent-Disposition: form-data; name=\"sample\"\r\n\r\norion2.jpg\r\n--pushto-test--\r\n")).unwrap()).await;
    assert_eq!(report["status"], "match_found");
    (state, app, report)
}

fn close(actual: &Value, expected: f64) {
    let actual = actual.as_f64().expect("a numeric guidance field");
    assert!((actual - expected).abs() < 1e-9, "{actual} != {expected}");
}

async fn guidance_after(seconds: f64) {
    let (state, app, _) = solved().await;
    let first = get(&app, "/api/session").await;
    // A second router shares the same observing session, reading a later FixedClock.
    let later = router(state.with_clock(Arc::new(FixedClock(TIME + seconds))));
    let view = get(&later, "/api/session").await;
    assert_eq!(view["time"], TIME + seconds);
    assert_eq!(view["pointing"], first["pointing"]);
    let pointing = &first["pointing"];
    let held = alt_az(
        pointing["ra_deg"].as_f64().unwrap(),
        pointing["dec_deg"].as_f64().unwrap(),
        &Observer {
            lat: 40.0,
            lon: -75.0,
            time: TIME,
        },
    );
    let target = finder::targets::find("M78").unwrap();
    let at = |time| {
        alt_az(
            target.ra_deg,
            target.dec_deg,
            &Observer {
                lat: 40.0,
                lon: -75.0,
                time,
            },
        )
    };
    let now = at(TIME + seconds);
    let then = at(TIME);
    let delta = (now.1 - held.1 + 180.0).rem_euclid(360.0) - 180.0;
    let guidance = &view["guidance"];
    assert_eq!(guidance["class"], "pointable");
    close(&guidance["turn_deg"], delta);
    close(&guidance["tilt_deg"], now.0 - held.0);
    close(
        &json!(
            guidance["tilt_deg"].as_f64().unwrap()
                - first["guidance"]["tilt_deg"].as_f64().unwrap()
        ),
        now.0 - then.0,
    );
    close(
        &json!(
            guidance["turn_deg"].as_f64().unwrap()
                - first["guidance"]["turn_deg"].as_f64().unwrap()
        ),
        now.1 - then.1,
    );
}

#[tokio::test]
async fn an_uploaded_pointing_never_goes_stale_an_hour_later() {
    guidance_after(3600.0).await;
}

#[tokio::test]
async fn ten_minutes_later_only_the_target_follows_the_clock() {
    guidance_after(600.0).await;
}

#[tokio::test]
async fn search_classes_use_the_session_and_console_clock() {
    let app = router(state(TIME));
    put(&app, json!({"site": {"lat_deg": 40, "lon_deg": -75}})).await;
    for (id, class) in [
        ("m42", "pointable"),
        ("m8", "below_horizon"),
        ("m75", "sun_guard"),
    ] {
        let result = get(&app, &format!("/api/targets?q={id}")).await;
        assert_eq!(result[0]["class"], class);
        assert_eq!(result[0]["messier"].as_str().unwrap().to_lowercase(), id);
    }
    put(&app, json!({"min_alt_deg": 50})).await;
    assert_eq!(get(&app, "/api/targets?q=m42").await[0]["class"], "too_low");
    put(&app, json!({"site": null})).await;
    assert_eq!(get(&app, "/api/targets?q=m42").await[0]["class"], "no_site");
    assert_eq!(
        get(&app, "/api/targets?q=m75").await[0]["class"],
        "sun_guard"
    );
}

#[tokio::test]
async fn m78_placement_equals_its_report_mark() {
    let (_, app, report) = solved().await;
    let view = get(&app, "/api/session").await;
    let placement = &view["guidance"]["placement"];
    let mark = report["objects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["id"] == "M78")
        .unwrap();
    assert_eq!(placement["on_frame"], true);
    for field in ["x", "y"] {
        close(&placement[field], mark[field].as_f64().unwrap());
    }
    close(&placement["axes"][0], mark["radius_px"].as_f64().unwrap());
    close(&placement["axes"][1], mark["minor_px"].as_f64().unwrap());
    close(&placement["angle"], mark["angle_deg"].as_f64().unwrap());
}

#[tokio::test]
async fn m79_placement_equals_its_nearby_report_mark() {
    let (_, app, report) = solved().await;
    put(&app, json!({"target": "M79"})).await;
    let view = get(&app, "/api/session").await;
    let placement = &view["guidance"]["placement"];
    let mark = report["nearby"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["id"] == "M79")
        .unwrap();
    assert_eq!(placement["on_frame"], false);
    close(
        &placement["direction_deg"],
        mark["direction_deg"].as_f64().unwrap(),
    );
    close(
        &placement["distance_deg"],
        mark["separation_deg"].as_f64().unwrap(),
    );
}

#[tokio::test]
async fn suggestions_rank_at_the_session_minimum_altitude() {
    let app = router(state(TIME));
    put(
        &app,
        json!({"site": {"lat_deg": 40, "lon_deg": -75}, "min_alt_deg": 50}),
    )
    .await;
    // Existing explicit-site clients and session-site clients use the same minimum.
    for uri in [
        "/api/similar?id=M42&limit=20",
        "/api/similar?id=M42&lat=40&lon=-75&limit=20",
        "/api/similar?id=M1&limit=20",
    ] {
        let report = get(&app, uri).await;
        let suggestions = report["suggestions"].as_array().unwrap();
        if uri.contains("id=M1&") {
            assert!(suggestions.iter().any(|o| o["up"] == true));
        }
        let m78 = suggestions.iter().position(|o| o["id"] == "M78").unwrap();
        assert_eq!(suggestions[m78]["up"], false);
        let mut down = false;
        for suggestion in suggestions {
            let object = finder::catalog::find(suggestion["id"].as_str().unwrap()).unwrap();
            let altitude = alt_az(
                object.ra_deg,
                object.dec_deg,
                &Observer {
                    lat: 40.0,
                    lon: -75.0,
                    time: TIME,
                },
            )
            .0;
            assert_eq!(suggestion["up"], altitude >= 50.0);
            if altitude < 50.0 {
                down = true;
            } else {
                assert!(!down);
            }
        }
    }
}

#[tokio::test]
async fn session_guidance_is_derived_for_the_selected_target_on_each_read() {
    let state = state(TIME).with_session(Session::default());
    let app = router(state.clone());
    assert!(get(&app, "/api/session").await["guidance"].is_null());
    put(
        &app,
        json!({"site": {"lat_deg": 40, "lon_deg": -75}, "target": "M8"}),
    )
    .await;
    let first = get(&app, "/api/session").await;
    let guidance = &first["guidance"];
    assert_eq!(guidance["class"], "below_horizon");
    let rise = guidance["rise"].as_f64().unwrap();
    assert!(TIME < rise && rise < guidance["transit"].as_f64().unwrap());
    assert!(guidance["transit"].as_f64().unwrap() < guidance["set"].as_f64().unwrap());
    assert!(guidance["turn_deg"].is_null() && guidance["tilt_deg"].is_null());
    let later = router(state.with_clock(Arc::new(FixedClock(rise + 60.0))));
    let risen = get(&later, "/api/session").await;
    assert_eq!(risen["target"], first["target"]);
    assert_eq!(risen["guidance"]["class"], "pointable");
    assert!(risen["guidance"]["rise"].is_null());
    put(&app, json!({"min_alt_deg": 30})).await;
    let never = get(&app, "/api/session").await;
    assert_eq!(never["guidance"]["never"], true);
    assert!(never["guidance"]["transit"].is_number());
    assert!(never["guidance"]["rise"].is_null() && never["guidance"]["set"].is_null());
}
