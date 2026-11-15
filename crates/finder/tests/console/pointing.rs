//! Latest-pointing contract on cedar's samples. RA/Dec oracle: the story's cedar
//! answer (1e-4 degrees); horizontal coordinates: sky::alt_az (1e-9 degrees).
//! The fixed clock, sequence linkage and retained fields have zero tolerance.

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock};

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use axum::Router;
use finder::samples::Samples;
use finder::session::{Clock, FixedClock};
use finder::sky::{alt_az, Observer};
use finder::{router, AppState};
use pattern_database::PatternDatabase;
use serde_json::{json, Value};
use tower::ServiceExt;

const TIME: f64 = 1768528800.0; // 2026-01-16 02:00 UT

static DB: LazyLock<Arc<PatternDatabase>> = LazyLock::new(|| {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../reference-solutions/cedar-solve/tetra3/data/default_database.npz");
    Arc::new(pattern_database::load_from_path(&path).unwrap())
});

fn app(clock: Arc<dyn Clock>) -> Router {
    let samples = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../reference-solutions/cedar-solve/examples/data/medium_fov");
    router(AppState::shared(DB.clone(), Samples::scan(&samples).unwrap()).with_clock(clock))
}

async fn send(app: &Router, request: Request<Body>) -> Value {
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(status, StatusCode::OK, "{body}");
    body
}

async fn session(app: &Router) -> Value {
    send(
        app,
        Request::get("/api/session").body(Body::empty()).unwrap(),
    )
    .await
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

async fn solve(app: &Router, sample: &str) -> Value {
    send(
        app,
        Request::post("/api/solve")
            .header("content-type", "multipart/form-data; boundary=pointing-test")
            .body(Body::from(format!("--pointing-test\r\nContent-Disposition: form-data; name=\"sample\"\r\n\r\n{sample}\r\n--pointing-test--\r\n")))
            .unwrap(),
    )
    .await
}

fn orion(pointing: &Value) {
    assert!(pointing.is_object(), "{pointing}");
    assert!((pointing["ra_deg"].as_f64().unwrap() - 85.190964).abs() < 1e-4);
    assert!((pointing["dec_deg"].as_f64().unwrap() - -4.246105).abs() < 1e-4);
    assert_eq!(pointing["time"], TIME);
    assert!(pointing["seq"].as_u64().is_some_and(|seq| seq > 0));
    assert!(pointing["frame"].is_object());
}

fn horizontal(pointing: &Value, lat: f64, lon: f64) {
    let (alt, az) = alt_az(
        pointing["ra_deg"].as_f64().unwrap(),
        pointing["dec_deg"].as_f64().unwrap(),
        &Observer {
            lat,
            lon,
            time: TIME,
        },
    );
    assert!((pointing["alt_deg"].as_f64().unwrap() - alt).abs() < 1e-9);
    assert!((pointing["az_deg"].as_f64().unwrap() - az).abs() < 1e-9);
}

#[tokio::test]
async fn solved_orion_records_its_position_time_horizontal_coordinates_and_report_sequence() {
    let app = app(Arc::new(FixedClock(TIME)));
    put(&app, json!({"site": {"lat_deg": 40, "lon_deg": -75}})).await;
    let report = solve(&app, "orion2.jpg").await;
    assert_eq!(report["status"], "match_found");
    let view = session(&app).await;
    let pointing = &view["pointing"];
    orion(pointing);
    horizontal(pointing, 40.0, -75.0);
    assert_eq!(pointing["ra_deg"], report["pointing"]["ra_deg"]);
    assert_eq!(pointing["dec_deg"], report["pointing"]["dec_deg"]);
    assert_eq!(report["pointing_seq"], pointing["seq"]);
    let frame = &pointing["frame"];
    assert_eq!(
        frame["camera"]["width"],
        report["width"].as_u64().unwrap() as f64
    );
    assert_eq!(
        frame["camera"]["height"],
        report["height"].as_u64().unwrap() as f64
    );
    assert!(
        (frame["camera"]["fov"].as_f64().unwrap()
            - report["pointing"]["fov_deg"].as_f64().unwrap().to_radians())
        .abs()
            < 1e-12
    );
    assert_eq!(frame["distortion"], report["pointing"]["distortion"]);
    // Oracle: the reported J2000 centre converted to a unit vector, 1e-12 tolerance.
    let centre = math_core::UnitVector::from_radec(
        pointing["ra_deg"].as_f64().unwrap().to_radians(),
        pointing["dec_deg"].as_f64().unwrap().to_radians(),
    );
    for (i, expected) in [centre.x, centre.y, centre.z].into_iter().enumerate() {
        assert!((frame["center"][i].as_f64().unwrap() - expected).abs() < 1e-12);
        assert!((frame["rotation"][0][i].as_f64().unwrap() - expected).abs() < 1e-12);
    }
}

#[tokio::test]
async fn unsolved_roof_retains_the_previous_pointing_field_for_field() {
    let app = app(Arc::new(FixedClock(TIME)));
    solve(&app, "orion2.jpg").await;
    let before = session(&app).await;
    orion(&before["pointing"]);
    let roof = solve(&app, "roof.jpg").await;
    assert_ne!(roof["status"], "match_found");
    assert!(roof["pointing_seq"].is_null());
    assert_eq!(session(&app).await, before);
}

#[tokio::test]
async fn solving_without_a_site_has_only_equatorial_coordinates_and_frame_time() {
    let app = app(Arc::new(FixedClock(TIME)));
    let report = solve(&app, "orion2.jpg").await;
    let pointing = session(&app).await["pointing"].clone();
    orion(&pointing);
    assert!(pointing["alt_deg"].is_null());
    assert!(pointing["az_deg"].is_null());
    assert_eq!(report["pointing_seq"], pointing["seq"]);
}

// A clock owned by this test makes edit time differ from frame time without sleeps.
struct TestClock(AtomicU64);
impl Clock for TestClock {
    fn now(&self) -> f64 {
        self.0.load(Ordering::Relaxed) as f64
    }
}

#[tokio::test]
async fn setting_changing_and_clearing_the_site_use_frame_time_and_keep_the_pointing() {
    let clock = Arc::new(TestClock(AtomicU64::new(TIME as u64)));
    let app = app(clock.clone());
    solve(&app, "orion2.jpg").await;
    let original = session(&app).await["pointing"].clone();
    orion(&original);
    clock.0.store(TIME as u64 + 86_400, Ordering::Relaxed);
    for (lat, lon) in [(40.0, -75.0), (-30.0, 150.0)] {
        let view = put(&app, json!({"site": {"lat_deg": lat, "lon_deg": lon}})).await;
        assert_eq!(view["time"], TIME + 86_400.0);
        horizontal(&view["pointing"], lat, lon);
        for field in ["seq", "frame", "ra_deg", "dec_deg", "time"] {
            assert_eq!(view["pointing"][field], original[field], "{field}");
        }
    }
    let cleared = put(&app, json!({"site": null})).await;
    assert_eq!(cleared["pointing"], original);
}

#[tokio::test]
async fn each_successful_solve_advances_the_sequence_and_uses_its_own_clock_time() {
    let clock = Arc::new(TestClock(AtomicU64::new(TIME as u64)));
    let app = app(clock.clone());
    let first = solve(&app, "orion2.jpg").await;
    let first_seq = first["pointing_seq"].as_u64().unwrap();
    clock.0.store(TIME as u64 + 60, Ordering::Relaxed);
    let second = solve(&app, "pleiades.jpg").await;
    let pointing = session(&app).await["pointing"].clone();
    assert_eq!(pointing["seq"], first_seq + 1);
    assert_eq!(second["pointing_seq"], pointing["seq"]);
    assert_eq!(pointing["time"], TIME + 60.0);
    assert_eq!(pointing["ra_deg"], second["pointing"]["ra_deg"]);
    assert_eq!(pointing["dec_deg"], second["pointing"]["dec_deg"]);
}
