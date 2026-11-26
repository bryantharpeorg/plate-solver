//! US3-S8/S9: the Track rig uses the paused runtime for live solves and
//! mount moves. Oracles are the specified refusal, lifecycle and settings;
//! every wait is bounded by 300 s of paused time.

use std::{path::Path, sync::Arc, time::Duration};

use axum::{http::StatusCode, Router};
use finder::{imaging::simulated::SimulatedSolver, samples::Samples, session::Session, AppState};
use scope_link::memory;
use serde_json::{json, Value};
use simulator::{mount::SimMount, SimNode, Simulator, Site, VirtualClock};

use crate::support::live as support;

struct Rig {
    app: Router,
    sim: Simulator,
    session: Arc<Session>,
}

fn rig(root: &Path) -> Rig {
    let sim = Simulator::new(
        support::DB.clone(),
        Site::default(),
        VirtualClock::new(support::START),
    );
    sim.connect_mount(SimMount::new(sim.clock().clone()));
    sim.push_to(50.0, 120.0);
    let node = Arc::new(
        SimNode::new(sim.clone(), "test-token", Default::default())
            .unwrap()
            .with_frame_interval(Duration::from_millis(500)),
    );
    let transport = finder::link::Transport::connector(move || {
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
    });
    let session = Session::default();
    let site = Site::default();
    session
        .edit(
            serde_json::from_value(json!({
                "node": {"address": "scope.local:1", "token": "test-token"},
                "site": {"lat_deg": site.lat_deg, "lon_deg": site.lon_deg},
                "target": "M42"
            }))
            .unwrap(),
            support::START,
        )
        .unwrap();
    let state = AppState::shared(support::DB.clone(), Samples::default())
        .with_session(session)
        .with_clock(Arc::new(support::ConsoleClock(sim.clock().clone())))
        .with_link(transport)
        .with_live_solver(Arc::new(crate::support::goto::Stars))
        .with_imaging(root)
        .with_imaging_solver(Arc::new(SimulatedSolver::new(sim.clone())))
        .with_focal_length_mm(1800.0);
    Rig {
        app: finder::router(state.clone()),
        session: state.session(),
        sim,
    }
}

async fn wait(app: &Router, path: &str, check: impl Fn(&Value) -> bool) -> Value {
    for _ in 0..3000 {
        let (status, view) = support::request(app, "GET", path, json!(null)).await;
        assert_eq!(status, StatusCode::OK);
        if check(&view) {
            return view;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("{path} did not reach the awaited state within 300 s");
}

async fn ready(rig: &Rig) {
    wait(&rig.app, "/api/imaging", |v| v["state"] == "ready").await;
    for _ in 0..3000 {
        if !rig.session.model(rig.sim.clock().now()).syncs.is_empty() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("the pointing model never held a sync");
}

async fn track(app: &Router, on: bool) {
    let (status, answer) = support::request(app, "POST", "/api/track", json!({"on": on})).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
}

#[tokio::test(start_paused = true)]
async fn start_refuses_track_on() {
    let scratch = crate::support::Scratch::new("imaging-align-track-start");
    let rig = rig(&scratch.0.join("imaging"));
    ready(&rig).await;
    track(&rig.app, true).await;
    wait(&rig.app, "/api/session", |v| v["track"]["wanted"] == true).await;
    assert_eq!(
        support::request(&rig.app, "POST", "/api/imaging/align", json!({})).await,
        (
            StatusCode::CONFLICT,
            json!({"error": "Track is on; turn it off while the finder camera's screws are turned"})
        )
    );
}

#[tokio::test(start_paused = true)]
async fn track_turned_on_ends_the_run_and_restores_the_camera() {
    let scratch = crate::support::Scratch::new("imaging-align-track-end");
    let rig = rig(&scratch.0.join("imaging"));
    ready(&rig).await;
    let (_, before) = support::request(&rig.app, "GET", "/api/imaging", json!(null)).await;
    let (status, answer) =
        support::request(&rig.app, "POST", "/api/imaging/align", json!({})).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    wait(&rig.app, "/api/imaging", |v| v["align"]["frames"] == 1).await;
    track(&rig.app, true).await;
    let view = wait(&rig.app, "/api/imaging", |v| v["align"]["state"] == "ended").await;
    assert_eq!(view["align"]["ended"], "failed");
    assert_eq!(
        view["align"]["error"],
        "Track is on; turn it off while the finder camera's screws are turned"
    );
    assert_eq!(view["settings"], before["settings"]);
    track(&rig.app, false).await;
    assert_eq!(
        support::request(&rig.app, "POST", "/api/imaging/frame", json!(null))
            .await
            .0,
        StatusCode::OK
    );
}
