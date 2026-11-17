//! US1-S7–S9: separation agrees with the independent haversine formula to 1e-6°.
//! The console clock is fixed; live freshness includes 5.0 s and excludes 5.1 s.

use crate::support::live as support;
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use finder::{
    router,
    samples::Samples,
    session::{FixedClock, Session},
    sky::Frame,
    AppState,
};
use serde_json::{json, Value};
use std::{path::Path, sync::Arc};
use tower::ServiceExt;

async fn read(app: &Router) -> Value {
    let (status, view) = support::request(app, "GET", "/api/session", json!(null)).await;
    assert_eq!(status, StatusCode::OK);
    view
}

async fn edit(app: &Router, body: Value) {
    let (status, _) = support::request(app, "PUT", "/api/session", body).await;
    assert_eq!(status, StatusCode::OK);
}

fn app(samples: Samples, session: Session, time: f64) -> Router {
    router(
        AppState::shared(support::DB.clone(), samples)
            .with_session(session)
            .with_clock(Arc::new(FixedClock(time))),
    )
}

async fn solved() -> Router {
    let samples = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../reference-solutions/cedar-solve/examples/data/medium_fov");
    let app = app(
        Samples::scan(&samples).unwrap(),
        Session::default(),
        support::START,
    );
    edit(
        &app,
        json!({"site": {"lat_deg": 40, "lon_deg": -75}, "target": "M78"}),
    )
    .await;
    let response = app.clone().oneshot(Request::post("/api/solve")
        .header("content-type", "multipart/form-data; boundary=separation-test")
        .body(Body::from("--separation-test\r\nContent-Disposition: form-data; name=\"sample\"\r\n\r\norion2.jpg\r\n--separation-test--\r\n")).unwrap()).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let report: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(report["status"], "match_found");
    app
}

fn assert_separation(view: &Value) {
    let pointing = &view["pointing"];
    let target = &view["target"];
    let radians = |position: &Value, field: &str| position[field].as_f64().unwrap().to_radians();
    let dec_a = radians(pointing, "dec_deg");
    let dec_b = radians(target, "dec_deg");
    let delta_ra = radians(target, "ra_deg") - radians(pointing, "ra_deg");
    // Haversine on the read's own J2000 coordinates, independent of angular_distance.
    let haversine = ((dec_b - dec_a) / 2.0).sin().powi(2)
        + dec_a.cos() * dec_b.cos() * (delta_ra / 2.0).sin().powi(2);
    let expected = (2.0 * haversine.sqrt().asin()).to_degrees();
    let guidance = &view["guidance"];
    assert_eq!(guidance["class"], "pointable");
    assert!(guidance["turn_deg"].is_number());
    assert!(guidance["tilt_deg"].is_number());
    let actual = guidance["separation_deg"]
        .as_f64()
        .expect("pointable guidance with directions carries separation_deg");
    assert!((actual - expected).abs() < 1e-6, "{actual} != {expected}");
}

#[tokio::test]
async fn solved_m78_separation_matches_the_reads_haversine_angle() {
    let app = solved().await;
    assert_separation(&read(&app).await);
}

#[tokio::test]
async fn separation_requires_directions_and_returns_when_the_target_is_pointable() {
    let app = solved().await;
    assert_separation(&read(&app).await);
    edit(&app, json!({"target": "M8"})).await;
    let view = read(&app).await;
    let guidance = &view["guidance"];
    assert_eq!(guidance["class"], "below_horizon");
    for field in ["turn_deg", "tilt_deg", "separation_deg"] {
        assert!(guidance.get(field).is_none(), "{field} on a down target");
    }
    edit(&app, json!({"target": "M78"})).await;
    assert_separation(&read(&app).await);

    let empty = self::app(Samples::default(), Session::default(), support::START);
    edit(
        &empty,
        json!({"site": {"lat_deg": 40, "lon_deg": -75}, "target": "M78"}),
    )
    .await;
    let view = read(&empty).await;
    assert!(view["pointing"].is_null());
    assert_eq!(view["guidance"]["class"], "pointable");
    for field in ["turn_deg", "tilt_deg", "separation_deg"] {
        assert!(
            view["guidance"].get(field).is_none(),
            "{field} without a pointing"
        );
    }
}

#[tokio::test]
async fn live_separation_is_present_at_five_seconds_and_cleared_after_it() {
    let frame = Frame::from_solution(&plate_solver::Solution {
        ra: Some(1.0),
        dec: Some(0.1),
        roll: Some(0.0),
        camera: Some(math_core::PinholeCamera::new(
            1456.0,
            1088.0,
            12_f64.to_radians(),
        )),
        ..Default::default()
    })
    .unwrap();
    let now = support::START + 10.0;
    let state = AppState::shared(support::DB.clone(), Samples::default())
        .with_clock(Arc::new(FixedClock(now)));
    let session = state.session();
    let app = router(state);
    edit(
        &app,
        json!({"site": {"lat_deg": 40, "lon_deg": -75}, "target": "M78"}),
    )
    .await;
    session.record_live_pointing(frame, now - 5.0, 1).unwrap();
    let view = read(&app).await;
    assert_eq!(view["pointing"]["source"], "live");
    assert!(view["guidance"].get("stale").is_none());
    assert_separation(&view);

    session.record_live_pointing(frame, now - 5.1, 2).unwrap();
    let view = read(&app).await;
    assert_eq!(view["guidance"]["stale"], true);
    for field in ["turn_deg", "tilt_deg", "separation_deg"] {
        assert!(view["guidance"].get(field).is_none(), "stale {field}");
    }
}
