//! US2-S1 to US2-S5: the tap on the solved frame, and the refusals that come
//! before the place is known. Oracles: the simulator's imaging truth and
//! push, the solve's own matrix at a pixel (`ImagingSolve::from_cd` and
//! `sky_at`, 032's), the words of spec.md FR-002, FR-006 and FR-007 as this
//! file writes them, and M42's catalogue place. No expected value is read from
//! the code under test: this file has its own copy of PARTS and of the words.
use super::centre::{m42, sep, Cause, Rig, CENTRE};
use super::nudge::write;
use crate::support::live as support;
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use finder::{imaging::solver::ImagingSolve, samples::Samples, AppState};
use serde_json::{json, Value};
use tower::ServiceExt;

const PARTS: &str = "centre takes nothing, to centre the target, or frame_id, x and y together";
const NOT_SOLVED: &str = "the imaging camera has not solved a frame yet";
const TRACK_OFF: &str = "Track is off: turn Track on to centre";
const NEWER: &str = "a newer imaging frame was taken after frame";

/// The imaging frame's size at the rig: the solve's frame, 1236 by 822.
const WIDTH: u32 = 1236;
const HEIGHT: u32 = 822;

async fn send(app: &Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let value = serde_json::from_slice(&body)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&body).into_owned()));
    (status, value)
}

/// A centring that must be refused with exactly `words`, a 409.
async fn refused(app: &Router, body: Value, words: &str) {
    let (status, answer) = write(app, "POST", CENTRE, body).await;
    assert_eq!(status, 409, "{answer}");
    assert_eq!(answer, json!({"error": words}));
}

/// A centring whose ask has one part, or two, of the three: 400 with PARTS.
async fn partial(app: &Router, body: Value) {
    let (status, answer) = write(app, "POST", CENTRE, body).await;
    assert_eq!(status, 400, "{answer}");
    assert_eq!(answer, json!({"error": PARTS}));
}

/// The sky at `pixel` of a solve's frame, by the solve's own matrix (032's).
fn sky_at(solve: &Value, pixel: (f64, f64)) -> (f64, f64) {
    let cd = serde_json::from_value(solve["cd_deg"].clone()).unwrap();
    ImagingSolve::from_cd(
        solve["ra_deg"].as_f64().unwrap(),
        solve["dec_deg"].as_f64().unwrap(),
        cd,
        WIDTH,
        HEIGHT,
    )
    .sky_at(pixel)
}

/// US2-S1: a tap centres its point at each imaging angle, with Track on at the
/// true eyepiece and the star frames stopped.
async fn tap_centres(angle: f64, pixel: (f64, f64)) {
    let rig = Rig::new(angle).await;
    rig.displace(Cause::Push, 0.0, 0.0).await;
    let solve = rig.solve().await;
    let place = sky_at(&solve, pixel);
    let before = rig.off(place);
    let answer = rig
        .call(
            "POST",
            CENTRE,
            json!({"frame_id": solve["frame_id"], "x": pixel.0, "y": pixel.1}),
        )
        .await;
    assert_eq!(answer["frame_id"], solve["frame_id"]);
    assert!(
        (answer["offset_arcsec"].as_f64().unwrap() - before).abs() <= 5.0,
        "{answer}, true offset {before}"
    );
    let after = rig.rests().await;
    assert!(
        rig.off(place) <= 30.0,
        "{} off the tapped point",
        rig.off(place)
    );
    assert!(sep(rig.held(), place) <= 0.01, "held {:?}", rig.held());
    assert_eq!(after["track"]["target"]["id"], "NGC 1976");
}

#[tokio::test(start_paused = true)]
async fn tap_at_imaging_angle_17_centres_its_point() {
    tap_centres(17.0, (300.0, 200.0)).await;
}

#[tokio::test(start_paused = true)]
async fn tap_at_imaging_angle_100_centres_its_point() {
    tap_centres(100.0, (1000.0, 700.0)).await;
}

#[tokio::test(start_paused = true)]
async fn tap_at_imaging_angle_250_centres_its_point() {
    tap_centres(250.0, (618.0, 100.0)).await;
}

/// US2-S2: the bare console, the rig's foreign write, text and unknown key, the
/// partial asks, the point's and frame's refusals in FR-002's order, and the
/// minute and the telescope's move for a point.
#[tokio::test(start_paused = true)]
async fn asks_and_refusals_answer_their_words_in_order() {
    let bare = finder::router(AppState::shared(support::DB.clone(), Samples::default()));
    refused(&bare, json!({}), NOT_SOLVED).await;

    let rig = Rig::new(17.0).await;
    let foreign = Request::post(CENTRE)
        .header("host", "localhost:8080")
        .header("origin", "http://elsewhere.example")
        .header("content-type", "application/json")
        .body(Body::from("{}"))
        .unwrap();
    assert_eq!(send(&rig.app, foreign).await.0, 403);
    let plain = Request::post(CENTRE)
        .header("host", "localhost:8080")
        .header("content-type", "text/plain")
        .body(Body::from("{}"))
        .unwrap();
    assert_eq!(send(&rig.app, plain).await.0, 415);
    let (status, answer) = write(&rig.app, "POST", CENTRE, json!({"z": 1})).await;
    assert_eq!(status, 400, "{answer}");

    let a = rig.solve().await;
    let a_id = a["frame_id"].as_u64().unwrap();
    partial(&rig.app, json!({"frame_id": a_id})).await;
    partial(&rig.app, json!({"frame_id": a_id, "x": 1.0})).await;

    rig.track_on().await;
    rig.sleep(15.0).await;
    let b = rig.solve().await;
    let b_id = b["frame_id"].as_u64().unwrap();
    refused(
        &rig.app,
        json!({"frame_id": b_id, "x": 1237.0, "y": 10.0}),
        &format!("the point is outside imaging frame {b_id}"),
    )
    .await;
    refused(
        &rig.app,
        json!({"frame_id": a_id, "x": 10.0, "y": 10.0}),
        &format!("imaging frame {a_id} is not the latest solved frame"),
    )
    .await;

    let c = rig.call("POST", "/api/imaging/frame", Value::Null).await;
    let c_id = c["frame"]["id"].as_u64().unwrap();
    refused(&rig.app, json!({}), &format!("{NEWER} {b_id}")).await;
    rig.wait("/api/imaging", |v| {
        v["frame"]["id"] == c_id && v["frame"]["state"] == "saved"
    })
    .await;
    rig.call("POST", "/api/imaging/solve", Value::Null).await;
    refused(
        &rig.app,
        json!({}),
        "the imaging camera's solve is still running",
    )
    .await;
    rig.wait("/api/imaging", |v| {
        v["solve"]["frame_id"] == c_id && v["solve"]["state"] == "solved"
    })
    .await;
    rig.call("PUT", "/api/session", json!({"target": null}))
        .await;
    refused(&rig.app, json!({}), "no target is selected").await;

    rig.sleep(61.0).await;
    refused(
        &rig.app,
        json!({"frame_id": c_id, "x": 618.0, "y": 300.0}),
        &format!("imaging frame {c_id} is more than a minute old: take another and solve it"),
    )
    .await;

    let d = rig.solve().await;
    let d_id = d["frame_id"].as_u64().unwrap();
    let point = json!({"frame_id": d_id, "x": 618.0, "y": 300.0});
    let answer = rig.call("POST", CENTRE, point.clone()).await;
    assert_eq!(answer["frame_id"], d_id);
    refused(
        &rig.app,
        point,
        &format!("the telescope moved after imaging frame {d_id}: take another and solve it"),
    )
    .await;
}

/// US2-S3: with a region of interest, the centring brings the target to the
/// sensor's centre (FR-003's set point), not to the frame's centre.
#[tokio::test(start_paused = true)]
async fn centres_a_frame_with_a_region_of_interest() {
    let rig = Rig::new(17.0).await;
    rig.displace(Cause::Push, 0.0, 0.0).await;
    rig.call(
        "PUT",
        "/api/imaging/settings",
        json!({"roi": {"x": 0, "y": 0, "width": 824, "height": 548}}),
    )
    .await;
    rig.solve().await;
    let before = rig.off(m42());
    let answer = rig.call("POST", CENTRE, json!({})).await;
    assert!(
        (answer["offset_arcsec"].as_f64().unwrap() - before).abs() <= 5.0,
        "{answer}, true offset {before}"
    );
    rig.rests().await;
    assert!(rig.off(m42()) <= 25.0, "{} off M42", rig.off(m42()));
}

/// US2-S4: the centring's refusals for Track and for the telescope's motion, in
/// order, with the frame each names.
#[tokio::test(start_paused = true)]
async fn track_and_motion_refusals_answer_in_order() {
    let rig = Rig::new(17.0).await;
    refused(&rig.app, json!({}), NOT_SOLVED).await;

    let first = rig.solve().await;
    let first_id = first["frame_id"].as_u64().unwrap();
    refused(&rig.app, json!({}), TRACK_OFF).await;

    rig.track_on().await;
    refused(
        &rig.app,
        json!({}),
        &format!("the telescope moved after imaging frame {first_id}: take another and solve it"),
    )
    .await;

    let second = rig.solve().await;
    let second_id = second["frame_id"].as_u64().unwrap();
    rig.sleep(15.0).await;
    refused(
        &rig.app,
        json!({}),
        &format!("the telescope moved after imaging frame {second_id}: take another and solve it"),
    )
    .await;

    let third = rig.solve().await;
    let third_id = third["frame_id"].as_u64().unwrap();
    rig.call("POST", CENTRE, json!({})).await;
    refused(
        &rig.app,
        json!({}),
        &format!("the telescope moved after imaging frame {third_id}: take another and solve it"),
    )
    .await;
}

/// US2-S5: a newer frame refuses before the solve's own state is read.
#[tokio::test(start_paused = true)]
async fn newer_frame_refuses_before_the_solve_state() {
    let rig = Rig::new(17.0).await;
    let b = rig.call("POST", "/api/imaging/frame", Value::Null).await;
    let b_id = b["frame"]["id"].as_u64().unwrap();
    rig.wait("/api/imaging", |v| {
        v["frame"]["id"] == b_id && v["frame"]["state"] == "saved"
    })
    .await;
    rig.call("POST", "/api/imaging/solve", Value::Null).await;
    rig.call("POST", "/api/imaging/frame", Value::Null).await;
    refused(&rig.app, json!({}), &format!("{NEWER} {b_id}")).await;
}
