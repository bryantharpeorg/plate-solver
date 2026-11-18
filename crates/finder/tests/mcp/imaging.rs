//! The three imaging MCP tools (031 US8): the listing, and the answers of
//! `get_imaging_camera`, `set_imaging_camera` and `take_imaging_frame` against
//! the routes' own bodies and the stand-in camera's rules. Oracles: the imaging
//! routes' bodies (the tools share their parser and refusals), the stand-in
//! camera's sample and range rules (030 US1), the file and preview of US2 and
//! US6, and tokio's paused clock.

use crate::support::{live, mcp as support};

use std::{path::Path, sync::Arc, time::Duration};

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use base64::prelude::{Engine as _, BASE64_STANDARD};
use finder::{link::Transport, AppState};
use image::GenericImageView;
use scope_link::memory;
use scope_node::{
    clock::NodeClock,
    imaging::{
        synthetic::SyntheticImagingCamera, Exposure, ImagingCamera, ImagingError, ImagingFrame,
        ImagingFuture, ImagingInfo, ImagingSettings, ImagingStatus,
    },
    service::{NodeControls, NodeService},
    Settings,
};
use serde_json::{json, Value};
use simulator::{mount::SimMount, SimCamera, VirtualClock};
use support::*;

/// A node's NodeClock on the simulator's virtual clock, so the stand-in
/// camera's exposures follow the same clock the console reads.
struct NodeTime(VirtualClock);
impl NodeClock for NodeTime {
    fn now(&self) -> f64 {
        self.0.node_now()
    }
}

/// The stand-in console rig of the shared setup: the sim rig's Simulator with a
/// connected mount, the node rig whose imaging camera is the one the builder
/// makes on the node's clock, a paired session, and `AppState` with live and
/// imaging, served through the MCP door.
async fn rig<C: ImagingCamera + 'static>(
    root: &Path,
    camera: impl FnOnce(Arc<dyn NodeClock>) -> C,
) -> Router {
    let sim = live::sim();
    sim.connect_mount(SimMount::new(sim.clock().clone()));
    sim.push_to(50.0, 120.0);
    let node = NodeService::with_controls(
        SimCamera::new(sim.clone()),
        Arc::new(NodeTime(sim.clock().clone())),
        "test-token",
        Settings::default(),
        NodeControls::default(),
    )
    .unwrap()
    .with_imaging(camera(Arc::new(NodeTime(sim.clock().clone()))));
    node.controls()
        .set_frame_interval(Duration::from_millis(500));
    let session = finder::session::Session::default();
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
    let transport = Transport::connector(move || {
        let node = node.clone();
        async {
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
        .with_imaging(root.to_path_buf());
    support::app(console, "127.0.0.1")
}

/// The stand-in console rig: the node's imaging camera is the stand-in camera.
async fn stand_in(root: &Path) -> Router {
    rig(root, SyntheticImagingCamera::new).await
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
        Request::get("/api/imaging").body(Body::empty()).unwrap(),
    )
    .await;
    assert_eq!(response.status(), 200);
    body(response).await
}

/// A write through the page's routes, answering status and body together.
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
    let status = response.status();
    (status, body(response).await)
}

/// The bytes of a preview route answer.
async fn preview_bytes(app: &Router, id: u64) -> Vec<u8> {
    let response = send(
        app,
        Request::get(format!("/api/imaging/frames/{id}/preview.jpg"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(response.status(), 200);
    to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap()
        .to_vec()
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

/// US8-S1: the three tools are listed with the stated descriptions, output
/// schemas and input schema (oracle: the MCP contract).
#[tokio::test]
async fn the_three_imaging_tools_are_listed_as_specified() {
    let app = app(state(), "127.0.0.1");
    let response = send(
        &app,
        request(json!({"jsonrpc":"2.0", "id":1, "method":"tools/list"})),
    )
    .await;
    let listing = body(response).await;
    let tools = listing["result"]["tools"].as_array().unwrap();
    let named = |name: &str| tools.iter().find(|tool| tool["name"] == name).unwrap();

    let get = named("get_imaging_camera");
    assert_eq!(get["outputSchema"], json!({"type":"object"}));
    assert_eq!(get["annotations"]["readOnlyHint"], true);
    assert_eq!(
        get["description"],
        "Read the imaging camera's state, description, status, settings and latest imaging frame."
    );

    let set = named("set_imaging_camera");
    assert_eq!(set["outputSchema"], json!({"type":"object"}));
    assert_eq!(set["annotations"]["readOnlyHint"].as_bool(), None);
    assert!(set["description"]
        .as_str()
        .unwrap()
        .contains("as PUT /api/imaging/settings does"));
    let properties = set["inputSchema"]["properties"].as_object().unwrap();
    let mut keys: Vec<&str> = properties.keys().map(|key| key.as_str()).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "binning",
            "cooler_on",
            "exposure_ms",
            "forward_kib_per_s",
            "gain",
            "offset",
            "roi",
            "save_at_node",
            "set_temp_c"
        ]
    );
    assert!(set["inputSchema"]
        .get("required")
        .is_none_or(|list| list.as_array().unwrap().is_empty()));
    assert_eq!(properties["gain"]["type"], json!("number"));
    assert!(
        properties["forward_kib_per_s"]["type"]
            .as_array()
            .unwrap()
            .contains(&json!("null")),
        "forward_kib_per_s must allow null: {}",
        properties["forward_kib_per_s"]
    );

    let take = named("take_imaging_frame");
    assert_eq!(take["outputSchema"], json!({"type":"object"}));
    assert_eq!(take["annotations"]["readOnlyHint"].as_bool(), None);
    let description = take["description"].as_str().unwrap();
    assert!(description.contains("colour JPEG") && description.contains("waits"));
}

/// US8-S2: `get_imaging_camera` and `get_session` carry the imaging view, and
/// `set_imaging_camera` shares the route's answers and refusals (oracle: the
/// routes' own bodies).
#[tokio::test(start_paused = true)]
async fn the_tools_share_the_imaging_routes_bodies() {
    let scratch = crate::support::Scratch::new("mcp-imaging-bodies");
    let root = scratch.0.join("imaging");
    let app = stand_in(&root).await;
    ready(&app).await;

    let result = tool(&app, "get_imaging_camera", json!({})).await;
    assert_ne!(result["isError"], true, "{result}");
    let view = imaging(&app).await;
    assert_eq!(result["structuredContent"], view);
    let session = tool(&app, "get_session", json!({})).await;
    assert_eq!(session["structuredContent"]["imaging"], view);

    let result = tool(
        &app,
        "set_imaging_camera",
        json!({"gain":200, "exposure_ms":100}),
    )
    .await;
    assert_ne!(result["isError"], true, "{result}");
    assert_eq!(
        result["structuredContent"]["settings"]["gain"],
        json!(200.0)
    );
    assert_eq!(
        result["structuredContent"]["settings"]["exposure_ms"],
        json!(100.0)
    );
    let view = imaging(&app).await;
    assert_eq!(view["settings"]["gain"], json!(200.0));
    assert_eq!(view["settings"]["exposure_ms"], json!(100.0));

    let result = tool(&app, "set_imaging_camera", json!({"gain":301})).await;
    assert_eq!(
        error_text(&result),
        "gain 301 is outside the camera's range 0–300"
    );

    // A body the route refuses for its shape: the tool answers the same words.
    let (status, answer) = page(&app, "PUT", "/api/imaging/settings", json!({"gian":1})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let result = tool(&app, "set_imaging_camera", json!({"gian":1})).await;
    assert_eq!(error_text(&result), answer["error"]);
}

/// US8-S3: `take_imaging_frame` waits for the frame, answers it saved with its
/// file, and carries the preview as a JPEG image block (oracles: the file under
/// R, and the preview route's own bytes).
#[tokio::test(start_paused = true)]
async fn take_imaging_frame_answers_the_saved_frame_and_its_preview() {
    let scratch = crate::support::Scratch::new("mcp-imaging-frame");
    let root = scratch.0.join("imaging");
    let app = stand_in(&root).await;
    ready(&app).await;
    tool(&app, "set_imaging_camera", json!({"exposure_ms":100})).await;

    let take = {
        let app = app.clone();
        tokio::spawn(async move { tool(&app, "take_imaging_frame", json!({})).await })
    };
    // Halfway through the 100 ms exposure: no answer and no file yet.
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(!take.is_finished());
    let file = root.join("frames/2026-01-16/000001.fits");
    assert!(!file.exists());

    let result = take.await.unwrap();
    assert_ne!(result["isError"], true, "{result}");
    let frame = &result["structuredContent"]["frame"];
    assert_eq!(frame["state"], json!("saved"));
    assert_eq!(frame["file"], json!("frames/2026-01-16/000001.fits"));
    assert!(file.exists(), "the frame's file must exist under R");
    assert_eq!(
        result["structuredContent"]["frame"]["preview"],
        json!("/api/imaging/frames/1/preview.jpg")
    );

    assert_eq!(result["content"].as_array().unwrap().len(), 2);
    assert_eq!(result["content"][0]["type"], "text");
    assert_eq!(result["content"][1]["type"], "image");
    assert_eq!(result["content"][1]["mimeType"], "image/jpeg");
    let bytes = BASE64_STANDARD
        .decode(result["content"][1]["data"].as_str().unwrap())
        .unwrap();
    let preview = preview_bytes(&app, 1).await;
    assert_eq!(bytes, preview);
    let picture = image::load_from_memory(&bytes).unwrap();
    assert_eq!(picture.dimensions(), (32, 24));
}

/// US8-S4: arguments are refused, a second exposure is refused, an aborted
/// frame answers its words, and so does a failed one.
#[tokio::test(start_paused = true)]
async fn take_imaging_frame_refuses_arguments_and_reports_how_the_frame_ended() {
    let scratch = crate::support::Scratch::new("mcp-imaging-refusals");
    let root = scratch.0.join("imaging");
    let app = stand_in(&root).await;
    ready(&app).await;
    tool(&app, "set_imaging_camera", json!({"exposure_ms":10000})).await;

    let result = tool(&app, "take_imaging_frame", json!({"exposure_ms":5})).await;
    assert_eq!(error_text(&result), "take_imaging_frame takes no arguments");

    // A frame started through the page 1 s before.
    let (status, _) = page(&app, "POST", "/api/imaging/frame", json!({})).await;
    assert_eq!(status, StatusCode::OK);
    tokio::time::sleep(Duration::from_secs(1)).await;
    let result = tool(&app, "take_imaging_frame", json!({})).await;
    assert_eq!(error_text(&result), "an exposure is already in progress");

    // Once that frame is saved, the tool's own frame is aborted 1 s in.
    wait_until(&app, |view| view["frame"]["state"] == "saved").await;
    let take = {
        let app = app.clone();
        tokio::spawn(async move { tool(&app, "take_imaging_frame", json!({})).await })
    };
    tokio::time::sleep(Duration::from_secs(1)).await;
    let (status, _) = page(&app, "POST", "/api/imaging/abort", json!({})).await;
    assert_eq!(status, StatusCode::OK);
    let result = take.await.unwrap();
    assert_eq!(error_text(&result), "the exposure was aborted");
}

/// US8-S4: a camera that fails the exposure answers the frame's own error
/// (oracle: the test's own camera's words).
#[tokio::test(start_paused = true)]
async fn a_failed_exposure_answers_the_frames_own_error() {
    let scratch = crate::support::Scratch::new("mcp-imaging-failure");
    let root = scratch.0.join("imaging");
    let app = rig(&root, |clock| FallsOver {
        inner: SyntheticImagingCamera::new(clock),
    })
    .await;
    ready(&app).await;
    let result = tool(&app, "take_imaging_frame", json!({})).await;
    assert_eq!(
        error_text(&result),
        "the exposure failed: the camera fell over"
    );
}

/// A camera of the test's own that describes itself as the stand-in camera does
/// and answers every exposure with its own failure.
struct FallsOver {
    inner: SyntheticImagingCamera,
}
impl ImagingCamera for FallsOver {
    fn describe(&self) -> ImagingFuture<'_, ImagingInfo> {
        self.inner.describe()
    }
    fn status(&self) -> ImagingFuture<'_, ImagingStatus> {
        self.inner.status()
    }
    fn apply<'a>(&'a self, edit: &'a ImagingSettings) -> ImagingFuture<'a, ImagingSettings> {
        self.inner.apply(edit)
    }
    fn expose<'a>(&'a self, _exposure: &'a Exposure) -> ImagingFuture<'a, ImagingFrame> {
        Box::pin(async { Err(ImagingError::Failed("the camera fell over".into())) })
    }
    fn abort(&self) -> ImagingFuture<'_, ()> {
        self.inner.abort()
    }
}

/// US8-S5: on a console that takes no imaging frames the tools answer at once
/// with the state `off` and the console's refusal (oracle: tokio's paused clock).
#[tokio::test(start_paused = true)]
async fn without_imaging_the_tools_answer_at_once() {
    let app = app(state(), "127.0.0.1");
    let result = tool(&app, "get_imaging_camera", json!({})).await;
    assert_ne!(result["isError"], true, "{result}");
    assert_eq!(result["structuredContent"], json!({"state":"off"}));
    for name in ["set_imaging_camera", "take_imaging_frame"] {
        let before = tokio::time::Instant::now();
        let result = tool(&app, name, json!({})).await;
        assert_eq!(before.elapsed(), Duration::ZERO, "{name} answered late");
        assert_eq!(
            error_text(&result),
            "this console takes no imaging frames",
            "{name}"
        );
    }
}

/// US8-S6: no result of the three tools holds the node's token (oracle: the
/// pairing's token, `test-token`).
#[tokio::test(start_paused = true)]
async fn no_tool_result_holds_the_node_token() {
    let scratch = crate::support::Scratch::new("mcp-imaging-token");
    let root = scratch.0.join("imaging");
    let app = stand_in(&root).await;
    ready(&app).await;
    for name in [
        "get_imaging_camera",
        "set_imaging_camera",
        "take_imaging_frame",
    ] {
        let result = tool(&app, name, json!({})).await;
        assert!(
            !result.to_string().contains("test-token"),
            "{name}: {result}"
        );
    }
}
