//! The preview frame's MCP tool and its words (058 US3): `take_preview_frame`
//! against the routes' own bodies and the stand-in rig's rules, and the README
//! and the glossary naming the preview frame. Oracles: the imaging routes'
//! bodies (the tool shares their parser and refusals), the simulated imaging
//! camera's 1236 × 822 RGGB sensor (`crates/simulator/src/imaging.rs:145`),
//! the folders' own listings, the scenarios' words, and the README's and the
//! glossary's text.

use std::{fs, path::Path, sync::Arc, time::Duration};

use axum::{body::Body, http::Request, Router};
use base64::prelude::{Engine as _, BASE64_STANDARD};
use finder::{imaging::Imaging, link::Transport, samples::Samples, session::Session, AppState};
use image::GenericImageView;
use scope_link::memory;
use serde_json::{json, Value};
use simulator::{mount::SimMount, SimNode};

use crate::support::{live, mcp as support, Scratch};

use support::*;

/// One fresh duplex for each connection, serving the node's own routes.
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

/// The MCP rig: the crop rig's node serving the simulator's imaging camera
/// over its own duplex, a paired session, the imaging folder R and the
/// imaging state, served through the MCP door.
fn rig(root: &Path) -> (Router, Arc<Imaging>) {
    let sim = live::sim();
    sim.connect_mount(SimMount::new(sim.clock().clone()));
    sim.push_to(50.0, 120.0);
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
    let state = AppState::shared(live::DB.clone(), Samples::default())
        .with_session(session)
        .with_clock(Arc::new(live::ConsoleClock(sim.clock().clone())))
        .with_link(duplex(&node))
        .with_live()
        .with_imaging(root);
    let imaging = state.imaging().expect("the rig's imaging state");
    (app(state, "127.0.0.1"), imaging)
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

/// Read `GET /api/imaging` every 100 ms until `check` holds, for at most
/// 120 s of paused time.
async fn wait_until(app: &Router, check: impl Fn(&Value) -> bool) -> Value {
    for _ in 0..1200 {
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

/// US3-S1: the tool refuses arguments, a console with no imaging and a
/// holder's lease, and answers a shown preview frame with the imaging view
/// and its preview and crop as two image blocks, saving nothing (oracles:
/// the folders' own listings and the routes' own words).
#[tokio::test(start_paused = true)]
async fn the_tool_takes_a_preview_frame_and_refuses_as_the_route_does() {
    // The bare console answers its own words at once.
    let bare = app(state(), "127.0.0.1");
    let result = tool(&bare, "take_preview_frame", json!({})).await;
    assert_eq!(error_text(&result), "this console takes no imaging frames");

    let scratch = Scratch::new("mcp-imaging-preview-frames");
    let root = scratch.0.join("imaging");
    let (app, imaging) = rig(&root);
    ready(&app).await;
    let result = tool(&app, "set_imaging_camera", json!({"exposure_ms": 200})).await;
    assert_ne!(result["isError"], true, "{result}");

    // Any argument is refused.
    let result = tool(&app, "take_preview_frame", json!({"x": 1})).await;
    assert_eq!(error_text(&result), "take_preview_frame takes no arguments");

    // The tool waits until the frame is shown and answers the imaging view
    // with its preview and its crop as two image blocks, in that order.
    let result = tool(&app, "take_preview_frame", json!({})).await;
    assert_ne!(result["isError"], true, "{result}");
    let frame = &result["structuredContent"]["frame"];
    assert_eq!(frame["state"], json!("shown"), "{frame}");
    assert_eq!(frame["purpose"], json!("preview"), "{frame}");
    assert!(frame.get("file").is_none(), "{frame}");
    let blocks = result["content"].as_array().unwrap();
    let images: Vec<&Value> = blocks
        .iter()
        .filter(|block| block["type"] == "image")
        .collect();
    assert_eq!(images.len(), 2, "{result}");
    let mut sizes = Vec::new();
    for block in &images {
        assert_eq!(block["mimeType"], "image/jpeg", "{result}");
        let bytes = BASE64_STANDARD
            .decode(block["data"].as_str().unwrap())
            .unwrap();
        sizes.push(image::load_from_memory(&bytes).unwrap().dimensions());
    }
    assert_eq!(sizes, vec![(618, 411), (200, 200)]);

    // Nothing was written: R holds no frames folder.
    assert!(!root.join("frames").exists());

    // A holder's lease is refused with its name.
    let lease = imaging.lease("the stack").unwrap();
    let result = tool(&app, "take_preview_frame", json!({})).await;
    assert_eq!(
        error_text(&result),
        "the imaging camera is held by the stack"
    );
    drop(lease);

    // The listing holds the tool with its schema and description.
    let response = send(
        &app,
        request(json!({"jsonrpc":"2.0", "id":1, "method":"tools/list"})),
    )
    .await;
    let listing = body(response).await;
    let take = listing["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|tool| tool["name"] == "take_preview_frame")
        .expect("the tool is listed");
    assert_eq!(take["outputSchema"], json!({"type": "object"}));
    assert_eq!(
        take["description"],
        json!("Take one preview frame, as POST /api/imaging/preview does: an imaging frame at the settings in force that is shown and never saved, at the console or at the scope node. It waits until the frame is shown and returns it with its colour JPEG preview and its crop, 200 x 200 cells around its brightest part, one pixel a cell.")
    );
}

/// The text with each run of whitespace as one space.
fn one_space(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// US3-S2: the README and the glossary name the preview frame where FR-010
/// and FR-011 place it, with the words the plan gives (oracle: the documents'
/// own text).
#[test]
fn the_readme_and_the_glossary_name_the_preview_frame_in_their_places() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let readme =
        fs::read_to_string(root.join("README.md")).expect("the committed README must be readable");
    let glossary = fs::read_to_string(root.join("../../GLOSSARY.md"))
        .expect("the committed glossary must be readable");
    let text = one_space(&readme);

    assert!(text.contains(
        "The imaging camera has sixteen tools of its own, which answer what the page's \
         Imaging page does (a seventeenth, for its preview frames, follows the Tools \
         page's tools below):"
    ));

    let focus_aid = text
        .find("The Tools page's focus aid for the finder camera has two tools:")
        .expect("the finder camera focus aid's paragraph");
    let paragraph = text
        .find("The Imaging page's preview frames have one tool:")
        .expect("the preview frames' paragraph");
    let wsl = text.find("### MCP under WSL2").expect("the WSL2 heading");
    assert!(focus_aid < paragraph && paragraph < wsl);
    assert!(
        text[paragraph..wsl].contains("- `take_preview_frame`"),
        "the paragraph holds the tool's entry"
    );

    assert!(text.contains(
        "`exposing`, `receiving`, `saved`, `shown` (a preview frame's end, below), \
         `failed` and `aborted`"
    ));

    let imaging_frames = text.find("## Imaging frames").expect("the section");
    let one_holder = text.find("**One holder.**").expect("the paragraph");
    let preview_frames = text.find("## Preview frames").expect("the section");
    let solving = text.find("## Solving imaging frames").expect("the section");
    assert!(imaging_frames < one_holder && one_holder < preview_frames && preview_frames < solving);
    let single = text
        .find("The Imaging page's **Preview frames** are single frames too")
        .expect("the sentence on single frames");
    assert!(one_holder < single && single < preview_frames);
    for words in [
        "nothing of it is saved",
        "Stopping: the last preview frame is still in flight",
    ] {
        let found = text
            .find(words)
            .unwrap_or_else(|| panic!("the section holds {words}"));
        assert!(preview_frames < found && found < solving, "{words}");
    }

    // The glossary: the two entries before **Sub**, each a line of its own,
    // and the purpose label's list ending with `preview`.
    let entry = |head: &str| {
        glossary
            .lines()
            .position(|line| line.trim() == head)
            .unwrap_or_else(|| panic!("the glossary holds {head} as a line of its own"))
    };
    let preview_frame = entry("**Preview frame**:");
    let crop = entry("**Crop** (of a preview frame):");
    let sub = entry("**Sub**:");
    assert!(preview_frame < crop && crop < sub);
    assert!(one_space(&glossary).contains("`sub`, `run`, `align` and `preview`."));
}
