//! US3: the scenario's literal words and response shapes are the MCP oracle;
//! FR-011 supplies the README's words and order. All waits use bounded, paused time.

use std::{fs, path::Path, sync::Arc, time::Duration};

use axum::{body::Body, http::Request, Router};
use finder::{link::Transport, samples::Samples, session::Session, AppState};
use scope_link::memory;
use serde_json::{json, Value};
use simulator::{mount::SimMount, SimNode};

use crate::support::{live, mcp::*};

fn rig() -> Router {
    let sim = live::sim();
    sim.connect_mount(SimMount::new(sim.clock().clone()));
    let node = Arc::new(
        SimNode::new(sim.clone(), "test-token", Default::default())
            .unwrap()
            .with_frame_interval(Duration::from_millis(500)),
    );
    let transport = Transport::connector(move || {
        let node = node.clone();
        async move {
            let (client, server) = tokio::io::duplex(64 * 1024);
            tokio::spawn(async move {
                memory::serve_routes_on(server, node.service().routes())
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
                "site": {"lat_deg": 40, "lon_deg": -75},
                "camera_settings": {"exposure_ms": 500, "gain": 1}
            }))
            .unwrap(),
            live::START,
        )
        .unwrap();
    let state = AppState::shared(live::DB.clone(), Samples::default())
        .with_session(session)
        .with_clock(Arc::new(live::ConsoleClock(sim.clock().clone())))
        .with_link(transport)
        .with_live();
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

fn focus(result: &Value) -> &Value {
    assert_ne!(result["isError"], true, "{result}");
    let content = &result["structuredContent"];
    assert_eq!(content.as_object().unwrap().len(), 1, "{result}");
    content.get("focus").expect("the answer holds focus")
}

async fn ready(app: &Router) {
    for _ in 0..1200 {
        if session(app).await["pointing"]["source"] == "live" {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("the finder camera did not solve live within 120 s");
}

/// US3-S1: the tools use the shared aid, validate arguments in order, carry
/// its exact refusal words and the session's settings, and advertise the read.
#[tokio::test(start_paused = true)]
async fn tools_run_the_finder_camera_focus_aid_and_keep_its_refusal_words() {
    let bare = app(state(), "127.0.0.1");
    assert_eq!(
        error(&tool(&bare, "set_finder_focus", json!({"on": true})).await),
        "the scope node is not connected"
    );

    let app = rig();
    let initial = tool(&app, "get_finder_focus", json!({})).await;
    assert!(focus(&initial).is_null(), "{initial}");
    assert_eq!(initial["structuredContent"], json!({"focus": null}));
    for (name, arguments, expected) in [
        ("set_finder_focus", json!({}), "on must be true or false"),
        (
            "set_finder_focus",
            json!({"x": 1}),
            "on must be true or false",
        ),
        (
            "set_finder_focus",
            json!({"on": true, "x": 1}),
            "set_finder_focus takes on and nothing else",
        ),
        (
            "get_finder_focus",
            json!({"x": 1}),
            "get_finder_focus takes no arguments",
        ),
        (
            "set_finder_focus",
            json!({"on": "true"}),
            "on must be true or false",
        ),
        (
            "set_finder_focus",
            json!({"on": false, "x": 1}),
            "set_finder_focus takes on and nothing else",
        ),
    ] {
        assert_eq!(error(&tool(&app, name, arguments).await), expected);
    }

    ready(&app).await;
    let started = tool(&app, "set_finder_focus", json!({"on": true})).await;
    assert_eq!(focus(&started)["state"], "running");
    assert_eq!(
        error(&tool(&app, "set_finder_focus", json!({"on": true})).await),
        "the finder camera's focus aid is already running"
    );

    let mut settled = false;
    for _ in 0..1200 {
        let result = tool(&app, "get_finder_focus", json!({})).await;
        let view = focus(&result);
        assert_eq!(view["state"], "running", "{view}");
        if view["median"]["readings"] == 3 {
            assert_eq!(view["last"]["state"], "reading");
            assert!(view["median"]["value"].as_f64().unwrap() > 0.0);
            settled = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(settled, "no settled focus reading within 120 s");

    let stopped = tool(&app, "set_finder_focus", json!({"on": false})).await;
    assert_eq!(focus(&stopped)["state"], "ended");
    assert_eq!(focus(&stopped)["ended"], "stopped");

    let response = send(
        &app,
        Request::put("/api/session")
            .header("host", "localhost:8080")
            .header("content-type", "application/json")
            .body(Body::from(
                json!({"camera_settings": {"gain": "auto"}}).to_string(),
            ))
            .unwrap(),
    )
    .await;
    assert_eq!(response.status(), 200);
    assert_eq!(body(response).await["camera_settings"]["gain"], "auto");
    assert_eq!(
        error(&tool(&app, "set_finder_focus", json!({"on": true})).await),
        "the finder camera's exposure or gain is automatic: fix both on Setup"
    );

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
        .find(|tool| tool["name"] == "get_finder_focus")
        .expect("get_finder_focus is listed");
    assert_eq!(reading["annotations"]["readOnlyHint"], true);
    let setting = tools
        .iter()
        .find(|tool| tool["name"] == "set_finder_focus")
        .expect("set_finder_focus is listed");
    assert_eq!(setting["inputSchema"]["required"], json!(["on"]));
    assert_eq!(
        setting["inputSchema"]["properties"]["on"]["type"],
        "boolean"
    );
}

/// US3-S2: the README retains the imaging count and introduces the finder
/// camera's tools and instructions in FR-011's specified places.
#[test]
fn readme_names_the_finder_camera_focus_aid_and_its_tools() {
    let readme = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("README.md"))
        .expect("finder's committed README must be readable");
    assert!(readme.contains("The imaging camera has sixteen tools of its own"));
    let alignment = readme
        .find("The Tools page's finder alignment has two tools:\n")
        .unwrap();
    let tools = readme
        .find("The Tools page's focus aid for the finder camera has two tools:\n")
        .expect("the finder camera's focus aid has its own tool paragraph");
    let wsl = readme.find("### MCP under WSL2").unwrap();
    assert!(alignment < tools && tools < wsl);
    for name in ["get_finder_focus", "set_finder_focus"] {
        assert!(
            readme[tools..wsl].contains(&format!("\n- `{name}`")),
            "missing list entry for {name}"
        );
    }
    let alignment = readme.find("\n## Finder alignment\n").unwrap();
    let focusing = readme.find("\n## Focusing the finder camera\n").unwrap();
    let calibration = readme.find("\n## Calibration frames\n").unwrap();
    assert!(alignment < focusing && focusing < calibration);
    assert!(readme[focusing..calibration]
        .contains("Set a fixed exposure and gain for the finder camera on Setup first."));
}
