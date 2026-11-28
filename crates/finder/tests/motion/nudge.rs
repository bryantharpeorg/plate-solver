//! Oracles: the simulator's true tube direction, counts, eyepiece and horizon
//! transform, the steps' sizes and the ways' angles clockwise of the view's up, and
//! the words of spec.md US1, 014 and 016. No expected value is read from the code
//! under test: no test reads `FINE_ARCSEC` or `COARSE_ARCSEC`.
use crate::support::goto::{Rig, DB};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use finder::{samples::Samples, AppState};
use scope_node::mount::{Axis, Mount};
use serde_json::{json, Value};
use simulator::mount::MountDirection;
use std::time::Duration;
use tower::ServiceExt;

/// The start of every nudge scenario, Unix seconds (a night at the site).
pub(super) const START: f64 = 1_768_528_800.0;
/// M42's catalogue place, degrees, as `steering_gap.rs` points the eyepiece at it.
const M42: (f64, f64) = (83.81867, -5.38967);
/// Each way's angle clockwise of the view's up, degrees: up, right, down, left.
pub(super) const WAYS: [(&str, f64); 4] = [
    ("up", 0.0),
    ("right", 90.0),
    ("down", 180.0),
    ("left", 270.0),
];
/// Each step's size on the sky, arcseconds: fine and coarse.
pub(super) const STEPS: [(&str, f64); 2] = [("fine", 30.0), ("coarse", 240.0)];

pub(super) fn wrap(angle: f64) -> f64 {
    (angle + 180.0).rem_euclid(360.0) - 180.0
}

async fn send(app: &Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let value = serde_json::from_slice(&body)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&body).into_owned()));
    (status, value)
}

/// A JSON write from the console's own origin: no `Origin` header, as a script sends it.
pub(super) async fn write(
    app: &Router,
    method: &str,
    path: &str,
    body: Value,
) -> (StatusCode, Value) {
    send(
        app,
        Request::builder()
            .method(method)
            .uri(path)
            .header("host", "localhost:8080")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
    )
    .await
}

/// One press of an arrow, as the page sends it.
pub(super) async fn nudge(
    app: &Router,
    direction: &str,
    step: &str,
    view: &str,
) -> (StatusCode, Value) {
    write(
        app,
        "POST",
        "/api/mount/nudge",
        json!({"direction": direction, "step": step, "view": view}),
    )
    .await
}

/// A press that must be refused with exactly `words`.
pub(super) async fn refused(app: &Router, direction: &str, step: &str, view: &str, words: &str) {
    let (status, body) = nudge(app, direction, step, view).await;
    assert_eq!(status, 409, "{body}");
    assert_eq!(body, json!({"error": words}));
}

async fn positions(rig: &Rig) -> [i32; 2] {
    [
        rig.mount.state(Axis::Azimuth).await.position,
        rig.mount.state(Axis::Altitude).await.position,
    ]
}

/// Reads until the move rests: `axis_move` no longer `moving` and both axes stopped,
/// read every 100 ms within 30 s, then one second more. Returns the session read last.
pub(super) async fn rest(rig: &Rig) -> Value {
    for _ in 0..300 {
        rig.tick().await;
        if rig.view().await["axis_move"]["state"] != "moving" && rig.stationary().await {
            for _ in 0..10 {
                rig.tick().await;
            }
            return rig.view().await;
        }
    }
    panic!("the move did not rest: {}", rig.view().await);
}

/// Waits for the approach's turnaround: both axes at rest after they have moved, while
/// `axis_move` still reads `moving`. Polled every 10 ms.
async fn turnaround(rig: &Rig, start: [i32; 2]) {
    for _ in 0..3000 {
        tokio::time::sleep(Duration::from_millis(10)).await;
        if positions(rig).await != start
            && rig.stationary().await
            && rig.view().await["axis_move"]["state"] == "moving"
        {
            return;
        }
    }
    panic!("the approach has no turnaround: {}", rig.view().await);
}

/// The synced rig at `alt`: built, synced, and pressed fine down and fine left in the
/// mount view, each read until it rests, so both axes rest on the flank a press leaves
/// them on.
pub(super) async fn synced(alt: f64) -> Rig {
    let rig = Rig::new(START, alt, 120.0, true).await;
    rig.sync().await;
    for direction in ["down", "left"] {
        let (status, body) = nudge(&rig.app, direction, "fine", "mount").await;
        assert_eq!(status, 200, "{body}");
        rest(&rig).await;
    }
    rig
}

/// The tube's move between two true directions, `[up, right]` arcseconds: the altitude
/// change, and the azimuth change times the cosine of the altitude before.
pub(super) fn moved(before: MountDirection, after: MountDirection) -> [f64; 2] {
    [
        (after.alt - before.alt) * 3600.0,
        wrap(after.az - before.az) * before.alt.to_radians().cos() * 3600.0,
    ]
}

/// How far a move lands from a step, as a fraction of the step's size: the step is
/// `size` arcseconds at `angle_deg` clockwise of the view's up.
pub(super) fn miss(moved: [f64; 2], size: f64, angle_deg: f64) -> f64 {
    let angle = angle_deg.to_radians();
    (moved[0] - size * angle.cos()).hypot(moved[1] - size * angle.sin()) / size
}

// US1-S1: the first press, then every step each way, each read at rest.
#[tokio::test(start_paused = true)]
async fn coarse_right_then_every_step_lands_and_leaves_the_model_alone() {
    let rig = synced(50.0).await;
    let began = rig.now();
    let epoch = rig.view().await["model"]["epoch"].clone();

    let (status, body) = nudge(&rig.app, "right", "coarse", "mount").await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        body,
        json!({"up_arcsec": 0.0, "right_arcsec": 240.0, "moved": "axes"})
    );
    let at_once = rig.view().await;
    assert_eq!(at_once["motion"]["by"], "page");
    assert_eq!(at_once["motion"]["tool"], "nudge");
    rest(&rig).await;

    for (step, size) in STEPS {
        for (way, angle) in WAYS {
            let before = rig.mount.true_direction();
            let (status, body) = nudge(&rig.app, way, step, "mount").await;
            assert_eq!(status, 200, "{way} {step}: {body}");
            assert_eq!(body["moved"], "axes");
            let rested = rest(&rig).await;
            assert_eq!(rested["axis_move"]["state"], "done");
            assert_eq!(rested["axis_move"]["tool"], "nudge");
            let after = rig.mount.true_direction();
            let landed = miss(moved(before, after), size, angle);
            assert!(
                landed < 0.03,
                "{way} {step} missed by {landed:.4} of its step"
            );
        }
    }

    let model = rig.state.session().model(rig.now());
    assert_eq!(rig.view().await["model"]["epoch"], epoch);
    assert!(!model.slip_pending);
    assert!(model.syncs.iter().any(|sync| sync.time > began));
}

// US1-S2: each fine press's moved axis dips below its end and comes up to it.
#[tokio::test(start_paused = true)]
async fn fine_presses_approach_each_moved_axis_from_below() {
    let rig = synced(50.0).await;
    for (way, axis) in [
        ("up", Axis::Altitude),
        ("right", Axis::Azimuth),
        ("down", Axis::Altitude),
        ("left", Axis::Azimuth),
    ] {
        let start = rig.mount.state(axis).await.position;
        let (status, body) = nudge(&rig.app, way, "fine", "mount").await;
        assert_eq!(status, 200, "{way}: {body}");
        let mut lowest = i32::MAX;
        for _ in 0..300 {
            rig.tick().await;
            lowest = lowest.min(rig.mount.state(axis).await.position);
            if rig.view().await["axis_move"]["state"] != "moving" && rig.stationary().await {
                break;
            }
        }
        let end = rig.mount.state(axis).await.position;
        assert!(end - lowest >= 2240, "{way}: end {end}, lowest {lowest}");
        if way == "up" || way == "right" {
            assert!(end > start, "{way}: end {end} not above start {start}");
        } else {
            assert!(end < start, "{way}: end {end} not below start {start}");
        }
    }
}

// US1-S3: the refusals and their words, and the press waiting for the approach's end.
#[tokio::test(start_paused = true)]
async fn refusals_answer_their_words_and_a_press_waits_for_the_move() {
    let bare = finder::router(AppState::shared(DB.clone(), Samples::default()));
    refused(&bare, "up", "fine", "mount", "no mount is connected").await;

    let rig = synced(50.0).await;
    let foreign = Request::post("/api/mount/nudge")
        .header("host", "localhost:8080")
        .header("origin", "http://elsewhere.example")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"direction": "up", "step": "fine", "view": "mount"}).to_string(),
        ))
        .unwrap();
    let (status, body) = send(&rig.app, foreign).await;
    assert_eq!(status, 403, "{body}");
    assert_eq!(
        body,
        json!({"error": "writes require the console's own origin and an allowed host"})
    );
    let plain = Request::post("/api/mount/nudge")
        .header("host", "localhost:8080")
        .header("content-type", "text/plain")
        .body(Body::from(
            json!({"direction": "up", "step": "fine", "view": "mount"}).to_string(),
        ))
        .unwrap();
    assert_eq!(send(&rig.app, plain).await.0, 415);
    let (status, body) = write(
        &rig.app,
        "POST",
        "/api/mount/nudge",
        json!({"direction": "up", "step": "fine", "view": "mount", "x": 1}),
    )
    .await;
    assert_eq!(status, 400, "{body}");
    let (status, body) = write(
        &rig.app,
        "POST",
        "/api/mount/nudge",
        json!({"direction": "north", "step": "fine", "view": "mount"}),
    )
    .await;
    assert_eq!(status, 400, "{body}");

    // Two presses one after the other: the second waits for the first.
    let (status, body) = nudge(&rig.app, "up", "fine", "mount").await;
    assert_eq!(status, 200, "{body}");
    refused(
        &rig.app,
        "up",
        "fine",
        "mount",
        "the mount is already moving",
    )
    .await;
    rest(&rig).await;

    // A press at the approach's turnaround is refused too.
    let start = positions(&rig).await;
    let (status, body) = nudge(&rig.app, "down", "fine", "mount").await;
    assert_eq!(status, 200, "{body}");
    turnaround(&rig, start).await;
    refused(
        &rig.app,
        "up",
        "fine",
        "mount",
        "the mount is already moving",
    )
    .await;
    rest(&rig).await;

    // The end below the minimum altitude, then the approach below it.
    let alt = rig.view().await["pointing"]["alt_deg"].as_f64().unwrap();
    let (status, body) = write(
        &rig.app,
        "PUT",
        "/api/session",
        json!({"min_alt_deg": alt - 0.02}),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    refused(
        &rig.app,
        "down",
        "coarse",
        "mount",
        "the position is below the minimum altitude",
    )
    .await;
    let (status, body) = write(
        &rig.app,
        "PUT",
        "/api/session",
        json!({"min_alt_deg": alt - 0.2}),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    refused(
        &rig.app,
        "down",
        "coarse",
        "mount",
        "approach would go below the minimum altitude",
    )
    .await;
    let (status, body) = nudge(&rig.app, "right", "coarse", "mount").await;
    assert_eq!(status, 200, "{body}");
    rest(&rig).await;

    // No sync: the frames stop, the night is reset, and the press is refused.
    rig.controls.stop_frames();
    tokio::time::sleep(Duration::from_millis(500)).await;
    let (status, body) = write(
        &rig.app,
        "POST",
        "/api/model/reset",
        json!({"what": "night"}),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    refused(&rig.app, "up", "fine", "mount", "the night has no sync yet").await;
}

// US1-S4: Track turned on during a press is refused; Track then takes the eyepiece
// to M42 once the moves have rested.
#[tokio::test(start_paused = true)]
async fn track_on_during_a_press_is_refused_and_track_starts_after_the_moves() {
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    rig.sim.point_eyepiece_at(M42.0, M42.1);
    rig.sync().await;

    let (status, body) = nudge(&rig.app, "down", "coarse", "mount").await;
    assert_eq!(status, 200, "{body}");
    tokio::time::sleep(Duration::from_millis(100)).await;
    let (status, body) = write(&rig.app, "POST", "/api/track", json!({"on": true})).await;
    assert_eq!(status, 409, "{body}");
    assert_eq!(body, json!({"error": "the mount is already moving"}));
    let rested = rest(&rig).await;
    assert!(rested.get("track").is_none() || rested["track"]["state"] == "off");

    let start = positions(&rig).await;
    let (status, body) = nudge(&rig.app, "down", "fine", "mount").await;
    assert_eq!(status, 200, "{body}");
    turnaround(&rig, start).await;
    let (status, body) = write(&rig.app, "POST", "/api/track", json!({"on": true})).await;
    assert_eq!(status, 409, "{body}");
    assert_eq!(body, json!({"error": "the mount is already moving"}));
    let rested = rest(&rig).await;
    assert_eq!(rested["axis_move"]["state"], "done");
    assert!(rested.get("track").is_none() || rested["track"]["state"] == "off");
    assert!(rig.error_arcsec("M42") > 240.0);

    let (status, body) = write(&rig.app, "POST", "/api/track", json!({"on": true})).await;
    assert_eq!(status, 200, "{body}");
    let mut session = rig.view().await;
    for _ in 0..1200 {
        if session["track"]["state"] == "tracking" {
            break;
        }
        rig.tick().await;
        session = rig.view().await;
    }
    assert_eq!(session["track"]["state"], "tracking", "{session}");
    assert!(rig.error_arcsec("M42") <= 30.0);
}

// US1-S5: the step's size near the zenith, and the refusal from 88° altitude up.
#[tokio::test(start_paused = true)]
async fn steps_near_the_zenith_keep_their_size_and_stop_at_88_degrees() {
    let rig = synced(86.0).await;
    let before = rig.mount.true_direction();
    let (status, body) = nudge(&rig.app, "right", "coarse", "mount").await;
    assert_eq!(status, 200, "{body}");
    rest(&rig).await;
    let landed = miss(moved(before, rig.mount.true_direction()), 240.0, 90.0);
    assert!(landed < 0.25, "coarse right missed by {landed:.4}");

    let before = rig.mount.true_direction();
    let (status, body) = nudge(&rig.app, "up", "coarse", "mount").await;
    assert_eq!(status, 200, "{body}");
    rest(&rig).await;
    let landed = miss(moved(before, rig.mount.true_direction()), 240.0, 0.0);
    assert!(landed < 0.04, "coarse up missed by {landed:.4}");

    let rig = Rig::new(START, 89.5, 120.0, true).await;
    rig.sync().await;
    refused(
        &rig.app,
        "up",
        "fine",
        "mount",
        "the telescope is too near the zenith: nudges stop at 88° altitude",
    )
    .await;
}
