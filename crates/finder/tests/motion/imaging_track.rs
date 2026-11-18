//! An aim reading taken while Track is on is held back and applied at the
//! first live solve after Track off, so 015's tracker never chases it (032
//! US4, FR-036). Oracles: the simulator's true aim point, the true eyepiece
//! direction sampled every 0.1 s in paused time with the test's own
//! separation, and 038's `slip_pending`, which an aim point applied at once
//! would mark.

use std::{path::Path, sync::Arc, time::Duration};

use axum::{http::StatusCode, Router};
use finder::{imaging, samples::Samples, session::Session, AppState};
use math_core::{angular_distance, UnitVector};
use scope_link::memory;
use serde_json::{json, Value};
use simulator::{mount::SimMount, SimNode, Simulator, Site, VirtualClock};

use crate::support::live as support;

/// The tracking rig's simulator (the shared setup): the mount connected first
/// and pushed to alt 50°, az 120° after.
fn sim() -> Simulator {
    let sim = Simulator::new(
        support::DB.clone(),
        Site::default(),
        VirtualClock::new(support::START),
    );
    sim.connect_mount(SimMount::new(sim.clock().clone()));
    sim.push_to(50.0, 120.0);
    sim
}

/// The node's routes over a fresh duplex for each connection.
fn duplex(node: &Arc<SimNode>) -> finder::link::Transport {
    let node = node.clone();
    finder::link::Transport::connector(move || {
        let node = node.clone();
        async move {
            let (client, server) = tokio::io::duplex(64 * 1024);
            let service = node.service();
            tokio::spawn(async move {
                memory::serve_routes_on(server, service.routes())
                    .await
                    .unwrap();
            });
            Ok::<_, std::io::Error>(client)
        }
    })
}

/// The tracking rig: the solving rig with two changes — the finder camera's
/// solves stay on the paused runtime through `goto::Stars` so GoTo and Track
/// run in virtual time, and the session has the target M42 — still with no
/// aim point.
struct Rig {
    app: Router,
    sim: Simulator,
    session: Arc<Session>,
}

fn rig(root: &Path) -> Rig {
    let sim = sim();
    let node = Arc::new(
        SimNode::new(sim.clone(), "test-token", Default::default())
            .unwrap()
            .with_frame_interval(Duration::from_millis(500)),
    );
    let session = Session::default();
    session
        .edit(
            serde_json::from_value(json!({
                "node": {"address": "scope.local:1", "token": "test-token"},
                "site": {"lat_deg": 40, "lon_deg": -75},
                "target": "M42"
            }))
            .unwrap(),
            support::START,
        )
        .unwrap();
    let state = AppState::shared(support::DB.clone(), Samples::default())
        .with_session(session)
        .with_clock(Arc::new(support::ConsoleClock(sim.clock().clone())))
        .with_link(duplex(&node))
        .with_live_solver(Arc::new(crate::support::goto::Stars))
        .with_imaging(root)
        .with_imaging_solver(Arc::new(imaging::simulated::SimulatedSolver::new(
            sim.clone(),
        )));
    Rig {
        app: finder::router(state.clone()),
        sim: sim.clone(),
        session: state.session(),
    }
}

async fn get(app: &Router, path: &str) -> Value {
    let (_, value) = support::request(app, "GET", path, json!(null)).await;
    value
}

/// Read `GET /api/imaging` every 100 ms of virtual time until `check` holds,
/// for at most 300 s.
async fn wait_imaging(app: &Router, check: impl Fn(&Value) -> bool) -> Value {
    for _ in 0..3000 {
        let (_, view) = support::request(app, "GET", "/api/imaging", json!(null)).await;
        if check(&view) {
            return view;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("imaging never reached the awaited state");
}

/// Read `GET /api/session` every 100 ms of virtual time until `check` holds,
/// for at most 300 s, as `crates/finder/tests/support/goto.rs` reads a sync.
async fn wait_session(app: &Router, check: impl Fn(&Value) -> bool) -> Value {
    for _ in 0..3000 {
        let (_, session) = support::request(app, "GET", "/api/session", json!(null)).await;
        if check(&session) {
            return session;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("the session never reached the awaited state");
}

/// US4-S8: with Track on and the GoTo's last approach 15 s past, an imaging
/// solve's aim reading is held: the session keeps no aim, the main telescope's
/// true direction moves under 60″ in the next 5 s and the model's slip is
/// never marked, and Track off applies the held aim point within 5 s (oracle:
/// the simulator's true aim point and its eyepiece direction, sampled every
/// 100 ms of virtual time with the shared setup's separation, and the model's
/// `slip_pending`).
#[tokio::test(start_paused = true)]
async fn an_aim_reading_taken_while_track_is_on_waits_for_track_off() {
    let scratch = crate::support::Scratch::new("imaging-track-s8");
    let folder = scratch.0.join("imaging");
    let rig = rig(&folder);
    wait_imaging(&rig.app, |view| view["state"] == "ready").await;

    // A sync in the pointing model, read once a tick as the goto support
    // reads it.
    let mut synced = false;
    for _ in 0..3000 {
        if !rig.session.model(rig.sim.clock().now()).syncs.is_empty() {
            synced = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(synced, "the pointing model never held a sync");

    let (status, answer) =
        support::request(&rig.app, "POST", "/api/track", json!({"on": true})).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    wait_session(&rig.app, |session| session["track"]["state"] == "tracking").await;
    // The fit reaches 10 s before the exposure, and the GoTo's last approach
    // must be outside it.
    tokio::time::sleep(Duration::from_secs(15)).await;

    // One frame, solved.
    let (status, view) =
        support::request(&rig.app, "POST", "/api/imaging/frame", json!(null)).await;
    assert_eq!(status, StatusCode::OK, "{view}");
    let id = view["frame"]["id"].as_u64().unwrap();
    wait_imaging(&rig.app, |view| {
        view["frame"]["id"] == json!(id) && view["frame"]["state"] == "saved"
    })
    .await;
    let (status, answer) =
        support::request(&rig.app, "POST", "/api/imaging/solve", json!(null)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let view = wait_imaging(&rig.app, |view| {
        view["solve"]["frame_id"] == json!(id) && view["solve"]["state"] != "solving"
    })
    .await;

    assert_eq!(view["solve"]["state"], json!("solved"), "{}", view["solve"]);
    assert_eq!(
        view["relation"]["last"]["outcome"],
        json!("measured"),
        "{}",
        view["relation"]
    );
    assert_eq!(
        view["relation"]["held"],
        json!(true),
        "{}",
        view["relation"]
    );
    assert!(
        (view["relation"]["aim"]["x"].as_f64().unwrap() - 700.0).abs() < 1.5,
        "{}",
        view["relation"]
    );
    assert!(
        (view["relation"]["aim"]["y"].as_f64().unwrap() - 420.0).abs() < 1.5,
        "{}",
        view["relation"]
    );
    let held = (
        view["relation"]["aim"]["x"].as_f64().unwrap(),
        view["relation"]["aim"]["y"].as_f64().unwrap(),
    );
    let session = get(&rig.app, "/api/session").await;
    assert!(session.get("aim").is_none(), "{session}");
    assert_eq!(session["track"]["state"], json!("tracking"), "{session}");

    // The main telescope's true direction for the 5 s after the solve ended:
    // never more than 60″ from its first reading, and the model's slip never
    // marked.
    let first = rig.sim.directions_at(rig.sim.clock().now()).eyepiece;
    let first = UnitVector::from_radec(first.ra.to_radians(), first.dec.to_radians());
    for _ in 0..50 {
        let time = rig.sim.clock().now();
        let eyepiece = rig.sim.directions_at(time).eyepiece;
        let direction = UnitVector::from_radec(eyepiece.ra.to_radians(), eyepiece.dec.to_radians());
        let separation = angular_distance(first, direction).to_degrees() * 3600.0;
        assert!(
            separation <= 60.0,
            "the telescope moved {separation} arcsec in the 5 s"
        );
        assert!(
            !rig.session.model(time).slip_pending,
            "the model's slip is marked"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Track off applies the held aim point within 5 s.
    let (status, answer) =
        support::request(&rig.app, "POST", "/api/track", json!({"on": false})).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let mut applied = None;
    for _ in 0..50 {
        let session = get(&rig.app, "/api/session").await;
        if session["aim"]["source"] == json!("measured") {
            applied = Some(session);
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let session = applied.expect("the held aim point was not applied within 5 s of Track off");
    assert!(
        (session["aim"]["x"].as_f64().unwrap() - held.0).abs() < 1e-9
            && (session["aim"]["y"].as_f64().unwrap() - held.1).abs() < 1e-9,
        "the session's aim {session} is not the held one {held:?}"
    );
    let (_, imaging) = support::request(&rig.app, "GET", "/api/imaging", json!(null)).await;
    assert!(
        imaging["relation"].get("held").is_none(),
        "{}",
        imaging["relation"]
    );
}
