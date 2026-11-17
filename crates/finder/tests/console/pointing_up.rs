//! US3-S1–S4: zenith bearing agrees with celestial north less the textbook
//! parallactic angle to 0.3° (J2000 north versus north of date, and projection).
//! Presence, sequence linkage and event/session equality have zero tolerance.

use crate::support::live as support;
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use finder::{
    samples::Samples,
    session::{Aim, FixedClock},
    sky::{self, Frame},
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

async fn edit(app: &Router, edit: Value) {
    let (status, _) = support::request(app, "PUT", "/api/session", edit).await;
    assert_eq!(status, StatusCode::OK);
}

async fn solved(with_site: bool) -> (Router, Value) {
    let samples = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../reference-solutions/cedar-solve/examples/data/medium_fov");
    let app = finder::router(
        AppState::shared(support::DB.clone(), Samples::scan(&samples).unwrap())
            .with_clock(Arc::new(FixedClock(support::START))),
    );
    if with_site {
        edit(&app, json!({"site": {"lat_deg": 40, "lon_deg": -75}})).await;
    }
    let response = app.clone().oneshot(Request::post("/api/solve")
        .header("content-type", "multipart/form-data; boundary=up-test")
        .body(Body::from("--up-test\r\nContent-Disposition: form-data; name=\"sample\"\r\n\r\norion2.jpg\r\n--up-test--\r\n")).unwrap()).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let report: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(report["status"], "match_found");
    (app, report)
}

fn assert_up(view: &Value, north: f64) {
    let p = &view["pointing"];
    let jd = sky::julian_date(p["time"].as_f64().unwrap());
    let (ra, dec) = sky::precess_from_j2000(
        p["ra_deg"].as_f64().unwrap(),
        p["dec_deg"].as_f64().unwrap(),
        jd,
    );
    let lat = view["site"]["lat_deg"].as_f64().unwrap().to_radians();
    let h = (sky::gmst_deg(jd) + view["site"]["lon_deg"].as_f64().unwrap() - ra).to_radians();
    let dec = dec.to_radians();
    // Parallactic angle: atan2(sin H, tan φ cos δ − sin δ cos H).
    let q = h.sin().atan2(lat.tan() * dec.cos() - dec.sin() * h.cos());
    let expected = (north - q.to_degrees()).rem_euclid(360.0);
    let actual = p["up_deg"].as_f64().expect("site supplies up_deg");
    assert!((0.0..360.0).contains(&actual));
    let error = ((actual - expected + 180.0).rem_euclid(360.0) - 180.0).abs();
    assert!(error < 0.3, "{actual} != {expected}, error {error}°");
}

#[tokio::test]
async fn orion_up_is_reported_north_less_the_parallactic_angle() {
    let (app, report) = solved(true).await;
    let view = read(&app).await;
    assert_eq!(view["pointing"]["seq"], report["pointing_seq"]);
    assert_up(&view, report["pointing"]["north_deg"].as_f64().unwrap());
}

#[tokio::test]
async fn up_is_absent_without_a_site_and_appears_when_the_site_is_set() {
    let (app, report) = solved(false).await;
    let without = read(&app).await;
    assert!(without["pointing"].is_object());
    assert!(without["pointing"].get("up_deg").is_none());
    edit(&app, json!({"site": {"lat_deg": 40, "lon_deg": -75}})).await;
    let with = read(&app).await;
    assert_eq!(with["pointing"]["seq"], without["pointing"]["seq"]);
    assert_up(&with, report["pointing"]["north_deg"].as_f64().unwrap());
    edit(&app, json!({"site": null})).await;
    assert!(read(&app).await["pointing"].get("up_deg").is_none());
}

#[tokio::test]
async fn live_up_uses_the_aim_direction_and_exposure_time() {
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
    // Read a day later: up must still describe the pointing's own exposure.
    let state = AppState::shared(support::DB.clone(), Samples::default())
        .with_clock(Arc::new(FixedClock(support::START + 86_400.0)));
    let session = state.session();
    session
        .set_aim(Some(Aim { x: 200.0, y: 150.0 }), support::START)
        .unwrap();
    let app = finder::router(state);
    edit(&app, json!({"site": {"lat_deg": 40, "lon_deg": -75}})).await;
    session
        .record_live_pointing(frame, support::START, 1)
        .unwrap();
    let view = read(&app).await;
    assert_eq!(view["pointing"]["source"], "live");
    assert_eq!(view["pointing"]["time"], support::START);
    let ray = frame.unproject((200.0, 150.0)).unwrap();
    let (ra, dec) = ray.to_radec();
    assert!((view["pointing"]["ra_deg"].as_f64().unwrap() - ra.to_degrees()).abs() < 1e-9);
    assert!((view["pointing"]["dec_deg"].as_f64().unwrap() - dec.to_degrees()).abs() < 1e-9);
    assert_up(&view, frame.north_at(ray).unwrap());
}

#[tokio::test(start_paused = true)]
async fn solved_live_events_keep_the_session_pointings_up_bearing() {
    let sim = support::sim();
    let state = support::state(&sim, None, true);
    let mut reports = state.live().unwrap().subscribe();
    let app = finder::router(state);
    for _ in 0..2 {
        let event = support::event(&mut reports).await;
        assert_eq!(event.report.status, finder::solve::Status::MatchFound);
        let event = serde_json::to_value(event).unwrap();
        let view = read(&app).await;
        assert!(event["pointing_seq"].is_number());
        assert_eq!(event["pointing_seq"], view["pointing"]["seq"]);
        assert!(event["up_deg"].is_number());
        assert_eq!(event["up_deg"], view["pointing"]["up_deg"]);
    }
}
