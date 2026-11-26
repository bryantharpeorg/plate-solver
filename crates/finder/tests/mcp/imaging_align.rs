//! US4: the MCP contract's literal words and the simulator's eyepiece pixel
//! (700, 420), within 2 px, are independent oracles. The README's words and
//! ordering come from FR-014; all waits use bounded, paused time.

use std::{fs, path::Path, sync::Arc, time::Duration};

use axum::{body::Body, http::Request, Router};
use finder::{
    imaging::simulated::SimulatedSolver, link::Transport, samples::Samples, session::Session,
    AppState,
};
use scope_link::memory;
use serde_json::{json, Value};
use simulator::{mount::SimMount, Optics, SimNode};

use crate::support::{live, mcp::*, Scratch};

fn rig(scratch: &Scratch) -> Router {
    let sim = live::sim();
    sim.connect_mount(SimMount::new(sim.clock().clone()));
    sim.push_to(50.0, 120.0);
    sim.set_optics(Optics {
        eyepiece_px: (700.0, 420.0),
        ..Optics::default()
    });
    let node = Arc::new(
        SimNode::new(sim.clone(), "test-token", Default::default())
            .unwrap()
            .with_frame_interval(Duration::from_millis(500)),
    );
    let transport = Transport::connector(move || {
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
    let state = AppState::shared(live::DB.clone(), Samples::default())
        .with_session(session)
        .with_clock(Arc::new(live::ConsoleClock(sim.clock().clone())))
        .with_link(transport)
        .with_live()
        .with_imaging(scratch.0.join("imaging"))
        .with_imaging_solver(Arc::new(SimulatedSolver::new(sim)))
        .with_focal_length_mm(1800.0);
    app(state, "127.0.0.1")
}

async fn tool(app: &Router, name: &str, arguments: Value) -> Value {
    let response = send(app, call(name, arguments)).await;
    assert_eq!(response.status(), 200);
    let rpc = body(response).await;
    assert!(rpc.get("error").is_none(), "{rpc}");
    rpc["result"].clone()
}

fn error(result: &Value) -> &str {
    assert_eq!(result["isError"], true, "{result}");
    assert_eq!(result["content"].as_array().unwrap().len(), 1);
    assert_eq!(result["content"][0]["type"], "text");
    result["content"][0]["text"].as_str().unwrap()
}

fn alignment(result: &Value) -> &Value {
    assert_ne!(result["isError"], true, "{result}");
    let content = &result["structuredContent"];
    assert_eq!(content.as_object().unwrap().len(), 1, "{result}");
    content.get("align").expect("the answer holds align")
}

async fn ready(app: &Router) {
    for _ in 0..1200 {
        let response = send(
            app,
            Request::get("/api/imaging").body(Body::empty()).unwrap(),
        )
        .await;
        assert_eq!(response.status(), 200);
        let view = body(response).await;
        if view["state"] == "ready" && session(app).await["pointing"]["source"] == "live" {
            tokio::time::sleep(Duration::from_secs(5)).await;
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("the imaging camera and live finder solve were not ready within 120 s");
}

/// US4-S1: argument precedence, null before a start, the shared alignment's
/// start/read/stop, and the tool listing, all through /mcp.
#[tokio::test(start_paused = true)]
async fn tools_run_the_finder_alignment_and_keep_its_refusal_words() {
    let bare = app(state(), "127.0.0.1");
    for (name, arguments) in [
        ("get_finder_alignment", json!({"x": 1})),
        ("set_finder_alignment", json!({})),
    ] {
        assert_eq!(
            error(&tool(&bare, name, arguments).await),
            "this console takes no imaging frames"
        );
    }

    let scratch = Scratch::new("mcp-imaging-align");
    let app = rig(&scratch);
    let result = tool(&app, "get_finder_alignment", json!({})).await;
    assert!(alignment(&result).is_null(), "{result}");
    assert_eq!(result["structuredContent"], json!({"align": null}));
    for (name, arguments, expected) in [
        (
            "set_finder_alignment",
            json!({}),
            "on must be true or false",
        ),
        (
            "set_finder_alignment",
            json!({"on": false, "exposure_ms": 5}),
            "set_finder_alignment with on false takes no other argument",
        ),
        (
            "get_finder_alignment",
            json!({"x": 1}),
            "get_finder_alignment takes no arguments",
        ),
    ] {
        assert_eq!(error(&tool(&app, name, arguments).await), expected);
    }

    ready(&app).await;
    let started = tool(&app, "set_finder_alignment", json!({"on": true})).await;
    assert_eq!(alignment(&started)["state"], "running");
    assert_eq!(
        error(&tool(&app, "set_finder_alignment", json!({"on": true})).await),
        "the finder alignment is already running"
    );

    let mut measured = false;
    for _ in 0..1200 {
        let result = tool(&app, "get_finder_alignment", json!({})).await;
        let align = alignment(&result);
        assert_eq!(align["state"], "running");
        if align["last"]["outcome"] == "measured" {
            let x = align["last"]["centre"]["x"].as_f64().unwrap();
            assert!((x - 700.0).abs() <= 2.0, "{align}");
            measured = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(measured, "no frame was measured within 120 s");
    let stopped = tool(&app, "set_finder_alignment", json!({"on": false})).await;
    assert_eq!(alignment(&stopped)["state"], "ended");
    assert_eq!(alignment(&stopped)["ended"], "stopped");

    let response = send(
        &app,
        request(json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"})),
    )
    .await;
    assert_eq!(response.status(), 200);
    let listing = body(response).await;
    let tools = listing["result"]["tools"].as_array().unwrap();
    let reading = tools
        .iter()
        .find(|tool| tool["name"] == "get_finder_alignment")
        .expect("get_finder_alignment is listed");
    assert_eq!(reading["annotations"]["readOnlyHint"], true);
    assert!(tools
        .iter()
        .any(|tool| tool["name"] == "set_finder_alignment"));
}

/// US4-S2: retain the imaging camera's count and put the alignment's two
/// tools and section in the specified places, with exact words.
#[test]
fn readme_names_the_finder_alignment_and_its_tools() {
    let readme = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("README.md"))
        .expect("finder's committed README must be readable");
    let count = readme
        .find("The imaging camera has sixteen tools of its own")
        .expect("the imaging camera's tool count stays unchanged");
    let tools = readme
        .find("The Tools page's finder alignment has two tools:\n")
        .expect("the alignment has its own tool paragraph");
    let wsl = readme.find("### MCP under WSL2").unwrap();
    assert!(count < tools && tools < wsl);
    for name in ["get_finder_alignment", "set_finder_alignment"] {
        assert!(
            readme[tools..wsl].contains(&format!("\n- `{name}`")),
            "missing list entry for {name}"
        );
    }
    assert!(
        readme.find("\n## Finder alignment\n").unwrap()
            < readme.find("\n## Calibration frames\n").unwrap()
    );
}
