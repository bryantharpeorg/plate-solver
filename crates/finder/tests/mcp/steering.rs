//! US11: steering through the MCP door. Oracles: the declared tool contract
//! and the steering route's own answers and refusals (036 US5-S5). JSON and
//! error words are exact; the no-imaging refusal advances no paused tokio time.

use std::{path::Path, sync::Arc, time::Duration};

use axum::{http::StatusCode, Router};
use finder::{link::Transport, AppState};
use scope_link::memory;
use scope_node::{
    clock::NodeClock,
    imaging::synthetic::SyntheticImagingCamera,
    service::{NodeControls, NodeService},
    Settings,
};
use serde_json::{json, Value};
use simulator::{mount::SimMount, SimCamera, VirtualClock};

use crate::support::{live, mcp::*, Scratch};

struct NodeTime(VirtualClock);
impl NodeClock for NodeTime {
    fn now(&self) -> f64 {
        self.0.node_now()
    }
}

/// A paired stand-in scope node and ready imaging camera, as in 031's MCP rig.
async fn imaging_console(root: &Path) -> Router {
    let sim = live::sim();
    sim.connect_mount(SimMount::new(sim.clock().clone()));
    sim.push_to(50.0, 120.0);
    let clock: Arc<dyn NodeClock> = Arc::new(NodeTime(sim.clock().clone()));
    let node = NodeService::with_controls(
        SimCamera::new(sim.clone()),
        clock.clone(),
        "test-token",
        Settings::default(),
        NodeControls::default(),
    )
    .unwrap()
    .with_imaging(SyntheticImagingCamera::new(clock));
    node.controls()
        .set_frame_interval(Duration::from_millis(500));
    let session = finder::session::Session::default();
    session
        .edit(
            serde_json::from_value(json!({
                "node":{"address":"scope.local:1","token":"test-token"},
                "site":{"lat_deg":40,"lon_deg":-75}
            }))
            .unwrap(),
            live::START,
        )
        .unwrap();
    let transport = Transport::connector(move || {
        let node = node.clone();
        async move {
            let (client, server) = tokio::io::duplex(64 * 1024);
            tokio::spawn(async move {
                memory::serve_routes_on(server, node.routes())
                    .await
                    .unwrap();
            });
            Ok::<_, std::io::Error>(client)
        }
    });
    let console = AppState::shared(live::DB.clone(), finder::samples::Samples::default())
        .with_session(session)
        .with_clock(Arc::new(live::ConsoleClock(sim.clock().clone())))
        .with_link(transport)
        .with_live()
        .with_imaging(root);
    let app = app(console, "127.0.0.1");
    for _ in 0..600 {
        if view(&tool(&app, "get_imaging_camera", json!({})).await)["state"] == "ready" {
            return app;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("imaging camera never ready");
}

async fn listed_tools(app: &Router) -> Vec<Value> {
    let response = send(
        app,
        request(json!({"jsonrpc":"2.0","id":1,"method":"tools/list"})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let rpc = body(response).await;
    assert!(rpc.get("error").is_none(), "{rpc}");
    rpc["result"]["tools"].as_array().unwrap().clone()
}

async fn tool(app: &Router, name: &str, arguments: Value) -> Value {
    let response = send(app, call(name, arguments)).await;
    assert_eq!(response.status(), StatusCode::OK);
    let rpc = body(response).await;
    assert!(rpc.get("error").is_none(), "{rpc}");
    rpc["result"].clone()
}

fn view(result: &Value) -> &Value {
    assert_ne!(result["isError"], true, "{result}");
    assert!(result["structuredContent"].is_object(), "{result}");
    &result["structuredContent"]
}

fn error_text(result: &Value) -> &str {
    assert_eq!(result["isError"], true, "{result}");
    assert_eq!(result["content"][0]["type"], "text", "{result}");
    result["content"][0]["text"].as_str().unwrap()
}

async fn put(app: &Router, edit: Value) -> (StatusCode, Value) {
    live::request(app, "PUT", "/api/imaging/steering", edit).await
}

/// US11-S1: the tool's description and schema match the contract literally.
#[tokio::test(start_paused = true)]
async fn listing_describes_four_optional_non_nullable_settings() {
    let scratch = Scratch::new("mcp-steering-listing");
    let app = imaging_console(&scratch.0).await;
    let tools = listed_tools(&app).await;
    let tool = tools
        .iter()
        .find(|tool| tool["name"] == "set_steering")
        .expect("set_steering is listed");
    assert_eq!(tool["description"], "Set what steers the tracker during a live stack, as PUT /api/imaging/steering does: on, smear_px, step_share and rate_share, each optional. Read the result in the steering part of get_imaging_camera.");
    assert_eq!(tool["outputSchema"], json!({"type":"object"}));
    assert!(tool["annotations"].get("readOnlyHint").is_none());
    let schema = &tool["inputSchema"];
    assert_eq!(schema["type"], "object");
    assert!(schema
        .get("required")
        .is_none_or(|required| required.as_array().unwrap().is_empty()));
    let properties = schema["properties"].as_object().unwrap();
    let mut keys: Vec<_> = properties.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(keys, ["on", "rate_share", "smear_px", "step_share"]);
    for (name, kind) in [
        ("on", "boolean"),
        ("smear_px", "number"),
        ("step_share", "number"),
        ("rate_share", "number"),
    ] {
        assert_eq!(
            properties[name]["type"], kind,
            "{name}: {}",
            properties[name]
        );
        assert!(properties[name].get("anyOf").is_none());
        assert!(properties[name].get("oneOf").is_none());
    }
}

/// US11-S2: partial edits persist, answer the route's view, and appear in reads.
#[tokio::test(start_paused = true)]
async fn edits_answer_the_imaging_view_and_are_visible_to_session_tools() {
    let scratch = Scratch::new("mcp-steering-edits");
    let app = imaging_console(&scratch.0).await;
    let edit = json!({"smear_px":1.5});
    let result = tool(&app, "set_steering", edit.clone()).await;
    assert_eq!(view(&result)["state"], "ready");
    assert_eq!(view(&result)["steering"]["smear_px"], 1.5);
    assert_eq!(view(&result)["steering"]["on"], true);
    let (status, route) = put(&app, edit).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(view(&result), &route);

    let edit = json!({"on":false,"step_share":0.25,"rate_share":0});
    let result = tool(&app, "set_steering", edit.clone()).await;
    let steering = &view(&result)["steering"];
    assert_eq!(steering["on"], false);
    assert_eq!(steering["step_share"], 0.25);
    assert_eq!(steering["rate_share"], 0.0);
    assert_eq!(steering["smear_px"], 1.5);
    let (status, route) = put(&app, edit).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(view(&result), &route);

    let camera = tool(&app, "get_imaging_camera", json!({})).await;
    assert_eq!(&view(&camera)["steering"], steering);
    let session = tool(&app, "get_session", json!({})).await;
    assert_eq!(&view(&session)["imaging"]["steering"], steering);
    let empty = tool(&app, "set_steering", json!({})).await;
    assert_eq!(view(&empty), view(&camera));
    let (status, route) = put(&app, json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(view(&empty), &route);
}

/// US11-S3: route refusals and strict argument parsing change no setting.
#[tokio::test(start_paused = true)]
async fn refusals_use_the_routes_words_and_leave_every_setting_unchanged() {
    let scratch = Scratch::new("mcp-steering-refusals");
    let app = imaging_console(&scratch.0).await;
    let baseline = tool(&app, "set_steering", json!({"smear_px":1.5})).await;
    let before = view(&baseline)["steering"].clone();
    for (edit, words) in [
        (
            json!({"smear_px":0}),
            Some("the corner smear must be a number from 0.1 to 20"),
        ),
        (
            json!({"step_share":1.5}),
            Some("the step share must be a number from 0 to 1"),
        ),
        (json!({"smear":2}), None),
        (json!({"on":1}), None),
        (
            json!({"rate_share":-0.1}),
            Some("the rate share must be a number from 0 to 1"),
        ),
        (json!({"on":null}), None),
        (json!({"smear_px":null}), None),
        (json!({"step_share":null}), None),
        (json!({"rate_share":null}), None),
        (
            json!({"on":false,"smear_px":2,"step_share":1.5}),
            Some("the step share must be a number from 0 to 1"),
        ),
        (
            json!({"on":false,"smear_px":0,"step_share":1.5,"rate_share":-0.1}),
            Some("the corner smear must be a number from 0.1 to 20"),
        ),
    ] {
        let result = tool(&app, "set_steering", edit.clone()).await;
        let error = error_text(&result);
        if let Some(words) = words {
            assert_eq!(error, words);
        }
        let current = tool(&app, "get_imaging_camera", json!({})).await;
        assert_eq!(view(&current)["steering"], before, "{edit}");
        let (status, route) = put(&app, edit.clone()).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{edit}: {route}");
        if words.is_some() {
            assert_eq!(route["error"], error);
        }
        let current = tool(&app, "get_imaging_camera", json!({})).await;
        assert_eq!(view(&current)["steering"], before, "{edit}");
    }
}

/// US11-S3: shape errors precede missing imaging; neither call passes time.
#[tokio::test(start_paused = true)]
async fn without_imaging_empty_arguments_are_refused_at_once_after_shape_checks() {
    let app = app(state(), "127.0.0.1");
    let before = tokio::time::Instant::now();
    let result = tool(&app, "set_steering", json!({})).await;
    assert_eq!(error_text(&result), "this console takes no imaging frames");
    assert_eq!(tokio::time::Instant::now(), before);
    let result = tool(&app, "set_steering", json!({"smear":2})).await;
    assert_ne!(error_text(&result), "this console takes no imaging frames");
    assert_eq!(tokio::time::Instant::now(), before);
}
