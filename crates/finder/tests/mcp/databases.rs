//! Oracles: US3-S8–S9's exact names, ranges and reasons, and the simulator's
//! 12 degree field (0.1 degree tolerance). MCP must preserve the route's list.

use crate::support::{live, mcp};
use axum::Router;
use base64::prelude::{Engine as _, BASE64_STANDARD};
use finder::{
    databases::{Database, Databases},
    samples::Samples,
    session::Session,
    solve::Status,
    AppState,
};
use serde_json::{json, Value};
use simulator::{Optics, SimNode};
use std::{path::Path, sync::Arc, time::Duration};

fn databases() -> Databases {
    Databases::new(
        ["first", "second"]
            .into_iter()
            .map(|name| Database {
                name: name.into(),
                db: live::DB.clone(),
            })
            .collect(),
    )
    .unwrap()
}

async fn tool(app: &Router, name: &str, arguments: Value) -> Value {
    let response = mcp::send(app, mcp::call(name, arguments)).await;
    assert_eq!(response.status(), 200);
    let rpc = mcp::body(response).await;
    assert!(rpc.get("error").is_none(), "{rpc}");
    assert_ne!(rpc["result"]["isError"], true, "{rpc}");
    rpc["result"]["structuredContent"].clone()
}

#[tokio::test(start_paused = true)]
async fn get_session_names_the_live_database_and_preserves_the_routes_list() {
    let sim = live::sim();
    sim.set_optics(Optics {
        fov_deg: 12.0,
        ..Default::default()
    });
    let session = Session::default();
    session
        .edit(
            serde_json::from_value(json!({
                "node": {"address": "scope.local:50051", "token": "test-token"},
                "camera_settings": {"exposure_ms": 200, "gain": 1}
            }))
            .unwrap(),
            live::START,
        )
        .unwrap();
    let node = Arc::new(
        SimNode::new(sim.clone(), "test-token", Default::default())
            .unwrap()
            .with_frame_interval(Duration::from_millis(500)),
    );
    let state = AppState::shared(live::DB.clone(), Samples::default())
        .with_databases(databases())
        .with_session(session)
        .with_clock(Arc::new(live::ConsoleClock(sim.clock().clone())))
        .with_link(live::transport(node))
        .with_live();
    let mut events = state.live().unwrap().subscribe();
    let app = mcp::app(state, "127.0.0.1");
    let report = live::event(&mut events).await;
    assert_eq!(report.report.status, Status::MatchFound);
    assert_eq!(report.report.database.as_deref(), Some("first"));
    assert!((report.report.pointing.as_ref().unwrap().fov_deg - 12.0).abs() < 0.1);
    let route = mcp::session(&app).await;
    let result = tool(&app, "get_session", json!({})).await;
    assert_eq!(
        result["field"]["database"],
        json!({"name": "first", "min_fov_deg": 10.0, "max_fov_deg": 30.0, "reason": "field"})
    );
    assert_eq!(
        route["field"]["databases"],
        json!([
            {"name": "first", "min_fov_deg": 10.0, "max_fov_deg": 30.0},
            {"name": "second", "min_fov_deg": 10.0, "max_fov_deg": 30.0}
        ])
    );
    assert_eq!(result["field"]["databases"], route["field"]["databases"]);
}

#[tokio::test]
async fn solve_frame_names_the_first_database_that_solves_the_saved_frame() {
    let app = mcp::app(mcp::state().with_databases(databases()), "127.0.0.1");
    let image = std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../reference-solutions/cedar-solve/examples/data/medium_fov/orion2.jpg"),
    )
    .unwrap();
    let result = tool(
        &app,
        "solve_frame",
        json!({"image": BASE64_STANDARD.encode(image)}),
    )
    .await;
    assert_eq!(result["status"], "match_found");
    assert_eq!(result["database"], "first");
}
