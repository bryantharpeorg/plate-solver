//! The imaging solve MCP tool (032 US7): the listing, and the answers of
//! `solve_imaging_frame` against the imaging view and the route's refusals.
//! Oracles: the scenario's own words, the MCP contract, the simulator's
//! virtual clock, the route's own view and the token the rig paired with.

use crate::support::{live, mcp as support};

use std::{path::Path, sync::Arc, time::Duration};

use axum::{body::Body, http::StatusCode, Router};
use finder::{
    imaging::{self, solver::ImagingSolver},
    link::Transport,
    samples::Samples,
    session::Session,
    AppState,
};
use scope_link::memory;
use serde_json::{json, Value};
use simulator::{mount::SimMount, SimNode, Simulator, Site, VirtualClock};
use support::*;

/// The solving rig's simulator (the shared setup): the test database at
/// 40° N 75° W on the virtual clock from Unix 1768528800, the mount connected
/// first and pushed to alt 50°, az 120° after.
fn sim() -> Simulator {
    let sim = Simulator::new(
        live::DB.clone(),
        Site::default(),
        VirtualClock::new(live::START),
    );
    sim.connect_mount(SimMount::new(sim.clock().clone()));
    sim.push_to(50.0, 120.0);
    sim
}

/// The node's routes over a fresh duplex for each connection.
fn duplex(node: &Arc<SimNode>) -> Transport {
    let node = node.clone();
    Transport::connector(move || {
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

/// The solving rig with MCP on (the shared setup): its node, a paired session
/// with the site and no target or aim point, the imaging folder `root` when
/// there is one, the solver when there is one, and the focal length in
/// millimetres when one is given, served through the MCP door.
struct Rig {
    app: Router,
    sim: Simulator,
}

fn rig(
    sim: &Simulator,
    root: Option<&Path>,
    solver: Option<Arc<dyn ImagingSolver>>,
    focal_length_mm: Option<f64>,
) -> Rig {
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
                "site": {"lat_deg": 40, "lon_deg": -75}
            }))
            .unwrap(),
            live::START,
        )
        .unwrap();
    let mut state = AppState::shared(live::DB.clone(), Samples::default())
        .with_session(session)
        .with_clock(Arc::new(live::ConsoleClock(sim.clock().clone())))
        .with_link(duplex(&node))
        .with_live();
    if let Some(root) = root {
        state = state.with_imaging(root.to_path_buf());
    }
    if let Some(solver) = solver {
        state = state.with_imaging_solver(solver);
    }
    if let Some(mm) = focal_length_mm {
        state = state.with_focal_length_mm(mm);
    }
    Rig {
        app: app(state, "127.0.0.1"),
        sim: sim.clone(),
    }
}

/// The solving rig with the simulated solver over the simulator's truth.
fn solving_rig(root: &Path) -> Rig {
    let sim = sim();
    let solver = Arc::new(imaging::simulated::SimulatedSolver::new(sim.clone()));
    rig(&sim, Some(root), Some(solver), None)
}

/// Call a tool and answer its result, failing on a JSON-RPC error.
async fn tool(app: &Router, name: &str, arguments: Value) -> Value {
    let response = send(app, call(name, arguments)).await;
    assert_eq!(response.status(), 200);
    let rpc = body(response).await;
    assert!(rpc.get("error").is_none(), "{rpc}");
    rpc["result"].clone()
}

/// The words of a tool error.
fn error_text(result: &Value) -> &str {
    assert_eq!(result["isError"], true, "{result}");
    assert_eq!(result["content"][0]["type"], "text", "{result}");
    result["content"][0]["text"].as_str().unwrap()
}

/// `GET /api/imaging`.
async fn imaging(app: &Router) -> Value {
    let response = send(
        app,
        axum::http::Request::get("/api/imaging")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(response.status(), 200);
    body(response).await
}

/// A write through the page's routes, answering status and body together.
async fn page(app: &Router, method: &str, path: &str, value: Value) -> (StatusCode, Value) {
    let response = send(
        app,
        axum::http::Request::builder()
            .method(method)
            .uri(path)
            .header("content-type", "application/json")
            .body(Body::from(value.to_string()))
            .unwrap(),
    )
    .await;
    let status = response.status();
    (status, body(response).await)
}

/// Read `GET /api/imaging` every 100 ms until `check` holds, for at most 60 s.
async fn wait_until(app: &Router, check: impl Fn(&Value) -> bool) -> Value {
    for _ in 0..600 {
        let view = imaging(app).await;
        if check(&view) {
            return view;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!(
        "imaging never reached the awaited state: {}",
        imaging(app).await
    );
}

async fn ready(app: &Router) -> Value {
    wait_until(app, |view| view["state"] == "ready").await
}

/// A saved frame (the shared setup): one frame asked for, awaited until saved.
async fn saved_frame(app: &Router) -> Value {
    let (status, view) = page(app, "POST", "/api/imaging/frame", json!({})).await;
    assert_eq!(status, StatusCode::OK, "{view}");
    let id = view["frame"]["id"].clone();
    wait_until(app, |view| {
        view["frame"]["id"] == id && view["frame"]["state"] == "saved"
    })
    .await
}

/// US7-S1: the tool is listed with an object output schema, not read-only,
/// with its description (oracle: the stated name, schema and words).
#[tokio::test]
async fn solve_imaging_frame_is_listed_as_specified() {
    let app = app(state(), "127.0.0.1");
    let response = send(
        &app,
        request(json!({"jsonrpc":"2.0", "id":1, "method":"tools/list"})),
    )
    .await;
    let listing = body(response).await;
    let tool = listing["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|tool| tool["name"] == "solve_imaging_frame")
        .unwrap();
    assert_eq!(tool["outputSchema"], json!({"type":"object"}));
    assert_ne!(tool["annotations"]["readOnlyHint"], json!(true));
    assert_eq!(
        tool["description"],
        "Solve the latest imaging frame, as POST /api/imaging/solve does. It waits until the solve ends and returns the imaging camera's state with the solve."
    );
}

/// US7-S2: the tool answers only once a solve delayed by 5 s of virtual time
/// has ended, with the solve the imaging view then shows (oracle: the stated
/// delay on the virtual clock, and the route's own view).
#[tokio::test(start_paused = true)]
async fn solve_imaging_frame_answers_only_once_the_solve_has_ended() {
    let scratch = crate::support::Scratch::new("mcp-imaging-solve-wait");
    let root = scratch.0.join("imaging");
    let sim = sim();
    let solver = Arc::new(
        imaging::simulated::SimulatedSolver::new(sim.clone()).with_delay(Duration::from_secs(5)),
    );
    let rig = rig(&sim, Some(&root), Some(solver), None);
    ready(&rig.app).await;
    let view = saved_frame(&rig.app).await;
    let id = view["frame"]["id"].clone();

    let before = rig.sim.clock().now();
    let result = tool(&rig.app, "solve_imaging_frame", json!({})).await;
    let after = rig.sim.clock().now();
    assert!(
        after - before >= 5.0,
        "the clock moved {} s, not the solve's 5 s",
        after - before
    );

    assert_ne!(result["isError"], true, "{result}");
    let solve = &result["structuredContent"]["solve"];
    assert_eq!(solve["state"], json!("solved"));
    assert_eq!(solve["frame_id"], id);
    assert!(!result.to_string().contains("test-token"), "{result}");

    // The route's own view, read just after, shows the same solve.
    let view = imaging(&rig.app).await;
    for key in ["ra_deg", "dec_deg", "file"] {
        assert_eq!(solve[key], view["solve"][key], "{key}: {solve}");
    }
}

/// US7-S3: arguments, no frame and a failed solve are each a tool error with
/// the route's or the solve's own words (oracle: the stated words).
#[tokio::test(start_paused = true)]
async fn arguments_a_missing_frame_and_a_failed_solve_are_tool_errors() {
    let scratch = crate::support::Scratch::new("mcp-imaging-solve-errors");
    let root = scratch.0.join("imaging");
    let first = solving_rig(&root);
    ready(&first.app).await;

    let result = tool(&first.app, "solve_imaging_frame", json!({"id":1})).await;
    assert_eq!(
        error_text(&result),
        "solve_imaging_frame takes no arguments"
    );

    // Before any imaging frame: the route's refusal.
    let result = tool(&first.app, "solve_imaging_frame", json!({})).await;
    assert_eq!(error_text(&result), "take an imaging frame first");

    // A rig built with `with_focal_length_mm(900.0)`: the focal length sizes
    // the field at twice the truth's height and the solve fails.
    let sim = sim();
    let solver = Arc::new(imaging::simulated::SimulatedSolver::new(sim.clone()));
    let failed = rig(&sim, Some(&root), Some(solver), Some(900.0));
    ready(&failed.app).await;
    saved_frame(&failed.app).await;
    let result = tool(&failed.app, "solve_imaging_frame", json!({})).await;
    assert_eq!(error_text(&result), "the imaging solver found no solution");
    assert!(!result.to_string().contains("test-token"), "{result}");
}

/// US7-S4: without imaging or without a solver the tool answers within 1 s of
/// tokio time with its words, and no result of the tool on the solving rig
/// holds the node's token (oracles: tokio's paused clock, the stated words
/// and the pairing's token).
#[tokio::test(start_paused = true)]
async fn without_imaging_or_a_solver_the_tool_answers_at_once() {
    let app = app(state(), "127.0.0.1");
    let before = tokio::time::Instant::now();
    let result = tool(&app, "solve_imaging_frame", json!({})).await;
    assert!(
        before.elapsed() < Duration::from_secs(1),
        "the tool answered late: {:?}",
        before.elapsed()
    );
    assert_eq!(error_text(&result), "this console takes no imaging frames");

    let scratch = crate::support::Scratch::new("mcp-imaging-solve-no-solver");
    let root = scratch.0.join("imaging");
    let sim = sim();
    let rig = rig(&sim, Some(&root), None, None);
    ready(&rig.app).await;
    saved_frame(&rig.app).await;
    let before = tokio::time::Instant::now();
    let result = tool(&rig.app, "solve_imaging_frame", json!({})).await;
    assert!(
        before.elapsed() < Duration::from_secs(1),
        "the tool answered late: {:?}",
        before.elapsed()
    );
    assert_eq!(error_text(&result), "this console has no imaging solver");
}
