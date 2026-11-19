//! US8: the calibration tools through the MCP door. Oracles: the declared
//! tool contract and the page routes' bodies and refusals, compared exactly.
//! The stand-in's samples are independent of exposure, so its darks pass and
//! its flats fail to rise. Every wait uses tokio's paused clock.

use std::{path::Path, sync::Arc, time::Duration};

use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
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

const CALIBRATION: &str = "/api/imaging/calibration";

struct NodeTime(VirtualClock);
impl NodeClock for NodeTime {
    fn now(&self) -> f64 {
        self.0.node_now()
    }
}

/// 031's stand-in console rig, with the MCP door enabled. The console and
/// node share a virtual clock, and the imaging camera delivers 64 × 48 samples.
fn stand_in(root: &Path) -> Router {
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
    app(
        AppState::shared(live::DB.clone(), finder::samples::Samples::default())
            .with_session(session)
            .with_clock(Arc::new(live::ConsoleClock(sim.clock().clone())))
            .with_link(transport)
            .with_live()
            .with_imaging(root),
        "127.0.0.1",
    )
}

async fn tool(app: &Router, name: &str, arguments: Value) -> Value {
    let response = send(app, call(name, arguments)).await;
    assert_eq!(response.status(), StatusCode::OK);
    let rpc = body(response).await;
    assert!(rpc.get("error").is_none(), "{rpc}");
    rpc["result"].clone()
}

fn error_text(result: &Value) -> &str {
    assert_eq!(result["isError"], true, "{result}");
    assert_eq!(result["content"][0]["type"], "text", "{result}");
    result["content"][0]["text"].as_str().unwrap()
}

async fn page(app: &Router, method: &str, path: &str, value: Value) -> (StatusCode, Value) {
    let response = send(
        app,
        Request::builder()
            .method(method)
            .uri(path)
            .header("content-type", "application/json")
            .body(Body::from(value.to_string()))
            .unwrap(),
    )
    .await;
    (response.status(), body(response).await)
}

async fn get(app: &Router, path: &str) -> Value {
    let (status, body) = page(app, "GET", path, Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    body
}

async fn wait_until(app: &Router, path: &str, check: impl Fn(&Value) -> bool) -> Value {
    for _ in 0..600 {
        let view = get(app, path).await;
        if check(&view) {
            return view;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!(
        "the awaited state was not reached: {}",
        get(app, path).await
    );
}

async fn ready(app: &Router) {
    wait_until(app, "/api/imaging", |view| view["state"] == "ready").await;
}

/// US8-S1: exact schemas, keys, descriptions and read-only annotations.
#[tokio::test]
async fn the_three_tools_are_listed_with_their_schemas() {
    let app = app(state(), "127.0.0.1");
    let response = send(
        &app,
        request(json!({"jsonrpc":"2.0","id":1,"method":"tools/list"})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let listing = body(response).await;
    let tools = listing["result"]["tools"].as_array().unwrap();
    let named = |name: &str| {
        tools
            .iter()
            .find(|tool| tool["name"] == name)
            .unwrap_or_else(|| panic!("missing tool {name}: {listing}"))
    };
    for name in ["list_calibration", "take_darks", "take_flats"] {
        let tool = named(name);
        assert_eq!(tool["outputSchema"], json!({"type":"object"}));
        assert_eq!(
            tool["annotations"]["readOnlyHint"] == true,
            name == "list_calibration"
        );
    }
    assert_eq!(named("list_calibration")["description"],
        "List the calibration library's master darks and master flats, the set being taken, and what matches the latest imaging frame.");
    for (name, route, expected) in [
        ("take_darks", "darks", vec!["anyway", "count"]),
        (
            "take_flats",
            "flats",
            vec!["anyway", "count", "darks", "label"],
        ),
    ] {
        let tool = named(name);
        let description = tool["description"].as_str().unwrap();
        assert!(description.contains(&format!("as POST {CALIBRATION}/{route} does")));
        assert!(description.contains("list_calibration"));
        let mut keys: Vec<_> = tool["inputSchema"]["properties"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(keys, expected);
    }
    assert_eq!(
        named("take_darks")["inputSchema"]["required"],
        json!(["count"])
    );
}

/// US8-S2: the route is the exact listing oracle; the stand-in's identical
/// samples at every exposure let three darks finish with one master.
#[tokio::test(start_paused = true)]
async fn the_listing_matches_the_route_and_darks_start_before_they_finish() {
    let scratch = Scratch::new("mcp-calibration-darks");
    let app = stand_in(&scratch.0.join("imaging"));
    ready(&app).await;
    let result = tool(&app, "list_calibration", json!({})).await;
    assert_ne!(result["isError"], true, "{result}");
    assert_eq!(result["structuredContent"], get(&app, CALIBRATION).await);

    let result = tool(&app, "take_darks", json!({"count":3})).await;
    assert_ne!(result["isError"], true, "{result}");
    let set = &result["structuredContent"]["set"];
    assert_eq!(set["kind"], "dark");
    assert_eq!(set["wanted"], 3);
    assert_eq!(set["state"], "offset");
    assert_eq!(set["taken"], 0);
    let running = get(&app, CALIBRATION).await;
    assert!(
        ["offset", "frames"].contains(&running["set"]["state"].as_str().unwrap()),
        "the tool must answer while the darks are still being taken: {running}"
    );

    wait_until(&app, CALIBRATION, |view| view["set"]["state"] == "done").await;
    let result = tool(&app, "list_calibration", json!({})).await;
    assert_ne!(result["isError"], true, "{result}");
    let route = get(&app, CALIBRATION).await;
    assert_eq!(result["structuredContent"], route);
    assert_eq!(route["set"]["state"], "done");
    let masters = route["masters"].as_array().unwrap();
    assert_eq!(masters.len(), 1);
    assert_eq!(masters[0]["frames"], 3);
    assert_eq!(masters[0]["model"], "synthetic imaging camera");
}

/// US8-S3: exact route refusals, malformed bodies, flat search failure and
/// refusal of a second set while the first's ten-second dark is exposing.
#[tokio::test(start_paused = true)]
async fn taking_tools_share_the_routes_refusals_and_start_flats() {
    let scratch = Scratch::new("mcp-calibration-refusals");
    let app = stand_in(&scratch.0.join("imaging"));
    ready(&app).await;
    for args in [json!({}), json!({"count":2})] {
        let result = tool(&app, "take_darks", args).await;
        assert_eq!(
            error_text(&result),
            "count must be a whole number from 3 to 64"
        );
    }
    let result = tool(&app, "take_flats", json!({"count":3})).await;
    assert_eq!(
        error_text(&result),
        "send count and label to take flats, or darks: true to take their darks"
    );
    let result = tool(&app, "take_flats", json!({"darks":true})).await;
    assert_eq!(error_text(&result), "no flats are waiting for their darks");

    for (name, route, args) in [
        ("take_darks", "darks", json!({"count":3,"cover":true})),
        (
            "take_flats",
            "flats",
            json!({"count":"three","label":"desk"}),
        ),
    ] {
        let (status, answer) = page(
            &app,
            "POST",
            &format!("{CALIBRATION}/{route}"),
            args.clone(),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        let result = tool(&app, name, args).await;
        assert_eq!(error_text(&result), answer["error"]);
    }

    let result = tool(&app, "take_flats", json!({"count":3,"label":"desk"})).await;
    assert_ne!(result["isError"], true, "{result}");
    assert_eq!(result["structuredContent"]["set"]["kind"], "flat");
    let failed = wait_until(&app, CALIBRATION, |view| view["set"]["state"] == "failed").await;
    assert!(failed["set"]["error"]
        .as_str()
        .unwrap()
        .starts_with("the flat's level does not rise with the exposure"));

    let (status, _) = page(
        &app,
        "PUT",
        "/api/imaging/settings",
        json!({"exposure_ms":10000}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let result = tool(&app, "take_darks", json!({"count":3})).await;
    assert_ne!(result["isError"], true, "{result}");
    let id = result["structuredContent"]["set"]["id"].as_str().unwrap();
    wait_until(&app, "/api/imaging", |view| {
        view["frame"]["state"] == "exposing"
            && view["frame"]["exposure_ms"].as_f64() == Some(10_000.0)
    })
    .await;
    let refused = tool(&app, "take_darks", json!({"count":3})).await;
    assert_eq!(
        error_text(&refused),
        "a set of darks or flats is already being taken"
    );
    let (status, _) = page(&app, "DELETE", &format!("{CALIBRATION}/{id}"), Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    wait_until(&app, CALIBRATION, |view| view["set"]["state"] == "stopped").await;
}

/// US8-S4: no imaging is judged before arguments, with no virtual time
/// elapsed, and all three tools keep the pairing token out of their results.
#[tokio::test(start_paused = true)]
async fn no_imaging_answers_at_once_and_results_keep_the_token_private() {
    let bare = app(state(), "127.0.0.1");
    let before = tokio::time::Instant::now();
    let result = tool(&bare, "list_calibration", json!({})).await;
    assert_ne!(result["isError"], true, "{result}");
    assert_eq!(result["structuredContent"], json!({"masters":[]}));
    assert_eq!(before.elapsed(), Duration::ZERO);
    for (name, args) in [
        ("take_darks", json!({})),
        ("take_flats", json!({})),
        ("take_darks", json!({"count":"many"})),
        ("take_flats", json!({"label":7})),
    ] {
        let before = tokio::time::Instant::now();
        let result = tool(&bare, name, args).await;
        assert_eq!(error_text(&result), "this console takes no imaging frames");
        assert_eq!(before.elapsed(), Duration::ZERO);
    }

    let scratch = Scratch::new("mcp-calibration-token");
    let app = stand_in(&scratch.0.join("imaging"));
    ready(&app).await;
    for name in ["list_calibration", "take_darks", "take_flats"] {
        let result = tool(&app, name, json!({})).await;
        assert!(
            !result.to_string().contains("test-token"),
            "{name}: {result}"
        );
    }
}
