//! US4: the MCP tool `nudge` answers the route's view and its refusals through the same
//! function, and the README and the glossary carry the words. The oracles are the
//! simulator's own counts, the words of FR-014 to FR-016 and plan.md § Interfaces.
use crate::support::mcp as support;

use std::{
    fs,
    io::Write,
    path::Path,
    sync::{Arc, Mutex},
};

use axum::Router;
use scope_node::mount::{Axis, Mount};
use serde_json::{json, Value};
use support::{
    sim::{Rig, START},
    *,
};

const USAGE: &str = "nudge takes direction (up, right, down or left), step (fine or coarse) and view (mount, finder_camera or imaging_camera)";
const MOVING: &str = "the mount is already moving";
const LIMITED: &str = "the rate limit of 2 moving calls a second was reached";
const NO_SYNC: &str = "the night has no sync yet";
const NOT_SOLVED: &str = "the imaging camera has not solved a frame yet";
const DIRECT: &str = "a direct axis move is running";

#[derive(Clone, Default)]
struct Log(Arc<Mutex<Vec<u8>>>);
impl Write for Log {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl Log {
    fn text(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }
}

/// One tool call's result: an MCP answer that never names the node's token.
async fn tool(app: &Router, name: &str, arguments: Value) -> Value {
    let response = send(app, call(name, arguments)).await;
    assert_eq!(response.status(), 200);
    let rpc = body(response).await;
    assert!(rpc.get("error").is_none(), "{rpc}");
    let result = rpc["result"].clone();
    assert!(!result.to_string().contains("test-token"), "{result}");
    result
}

/// The words of a tool error, which is its only content.
fn error(result: &Value) -> &str {
    assert_eq!(result["isError"], true, "{result}");
    assert_eq!(result["content"].as_array().unwrap().len(), 1, "{result}");
    result["content"][0]["text"].as_str().unwrap()
}

async fn positions(rig: &Rig) -> [i32; 2] {
    [
        rig.mount.state(Axis::Azimuth).await.position,
        rig.mount.state(Axis::Altitude).await.position,
    ]
}

/// Reads the session each 100 ms until the last direct move has ended.
async fn until_rest(rig: &Rig, client: &Router) {
    for _ in 0..1200 {
        if session(client).await["axis_move"]["state"] != "moving" {
            return;
        }
        rig.tick().await;
    }
    panic!("the move did not end: {}", session(client).await);
}

/// US4-S1: the tool is listed with its schema, answers the mount view's step as its
/// structured content, moves the mount through the console's guards, holds `motion`
/// for its caller, and logs its call without the token.
#[tokio::test(start_paused = true)]
async fn nudge_is_listed_moves_the_mount_and_holds_the_motion_for_its_caller() {
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    rig.sync().await;
    let log = Log::default();
    let client = app(rig.state.clone().with_call_log(log.clone()), "127.0.0.1");

    let listing = body(
        send(
            &client,
            request(json!({"jsonrpc":"2.0","id":7,"method":"tools/list"})),
        )
        .await,
    )
    .await;
    let entry = listing["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|candidate| candidate["name"] == "nudge")
        .expect("nudge is listed");
    assert!(entry["description"]
        .as_str()
        .unwrap()
        .contains("real telescope"));
    assert_eq!(entry["outputSchema"]["type"], "object");
    for property in ["direction", "step", "view"] {
        assert_eq!(
            entry["inputSchema"]["properties"][property]["type"], "string",
            "{entry}"
        );
    }

    let before = positions(&rig).await;
    let answer = tool(
        &client,
        "nudge",
        json!({"direction": "right", "step": "coarse", "view": "mount"}),
    )
    .await;
    assert_ne!(answer["isError"], true, "{answer}");
    assert_eq!(
        answer["structuredContent"],
        json!({"up_arcsec": 0.0, "right_arcsec": 240.0, "moved": "axes"})
    );
    let read = session(&client).await;
    assert_eq!(read["motion"]["by"], "mcp", "{read}");
    assert_eq!(read["motion"]["tool"], "nudge", "{read}");

    assert_eq!(
        error(&tool(&client, "set_tracking", json!({"on": true})).await),
        DIRECT
    );
    until_rest(&rig, &client).await;
    let after = positions(&rig).await;
    assert!(after[0] > before[0], "azimuth {before:?} -> {after:?}");
    assert_eq!(after[1], before[1], "altitude {before:?} -> {after:?}");

    assert!(
        log.text().lines().any(|line| line.contains("mcp nudge")),
        "{}",
        log.text()
    );
    assert!(!log.text().contains("test-token"));
}

/// US4-S2: before the sync the tool refuses with the route's words; a malformed call is
/// refused before it takes a place in the rate limit; of three coarse presses in a row
/// the second meets the move under way and the third the limit; and the imaging camera
/// view has no frame to read before a solve.
#[tokio::test(start_paused = true)]
async fn nudge_refuses_with_the_route_words_and_shares_the_rate_limit() {
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    let client = app(rig.state.clone(), "127.0.0.1");
    assert_eq!(
        error(
            &tool(
                &client,
                "nudge",
                json!({"direction": "up", "step": "fine", "view": "mount"}),
            )
            .await
        ),
        NO_SYNC
    );

    rig.sync().await;
    for arguments in [
        json!({"direction": "north", "step": "fine", "view": "mount"}),
        json!({"direction": "up", "step": "fine"}),
        json!({"direction": "up", "step": "fine", "view": "mount", "x": 1}),
    ] {
        assert_eq!(error(&tool(&client, "nudge", arguments).await), USAGE);
    }

    for _ in 0..11 {
        rig.tick().await;
    }
    let coarse_up = json!({"direction": "up", "step": "coarse", "view": "mount"});
    let first = tool(&client, "nudge", coarse_up.clone()).await;
    assert_ne!(first["isError"], true, "{first}");
    assert_eq!(
        error(&tool(&client, "nudge", coarse_up.clone()).await),
        MOVING
    );
    assert_eq!(error(&tool(&client, "nudge", coarse_up).await), LIMITED);

    until_rest(&rig, &client).await;
    for _ in 0..10 {
        rig.tick().await;
    }
    assert_eq!(
        error(
            &tool(
                &client,
                "nudge",
                json!({"direction": "up", "step": "fine", "view": "imaging_camera"}),
            )
            .await
        ),
        NOT_SOLVED
    );
}

/// US4-S3: the committed README and glossary name the nudge where FR-015 and FR-016 place
/// it, with the words those requirements and plan.md § Interfaces give.
#[test]
fn the_readme_and_the_glossary_name_the_nudge_in_their_places() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let readme =
        fs::read_to_string(root.join("README.md")).expect("the committed README must be readable");
    let glossary = fs::read_to_string(root.join("../../GLOSSARY.md"))
        .expect("the committed glossary must be readable");
    assert!(readme.contains("The imaging camera has sixteen tools of its own"));

    let tools = readme
        .find("The Tools page's focus aid for the finder camera has two tools:")
        .unwrap();
    let paragraph = readme
        .find("The page's nudge arrows have one tool:")
        .expect("the nudge arrows have their own paragraph");
    let wsl = readme.find("### MCP under WSL2").unwrap();
    assert!(tools < paragraph && paragraph < wsl);
    assert!(readme[paragraph..wsl]
        .lines()
        .any(|line| line.starts_with("- `nudge` takes `direction` ")));

    let api = readme.find("\n## API\n").expect("the API section") + 1;
    let api_section = &readme[api..api + readme[api..].find("\n## ").unwrap()];
    assert!(api_section
        .lines()
        .any(|line| line.starts_with("| `POST /api/mount/nudge` |")));

    let drawn = readme.find("\n## Drawn sky on the real mount\n").unwrap();
    let nudging = readme
        .find("\n## Nudging the telescope\n")
        .expect("the nudging section");
    let recording = readme.find("\n## Recording a night\n").unwrap();
    assert!(drawn < nudging && nudging < recording);
    let section = &readme[nudging..recording];
    for words in [
        "fine is 30″, coarse is 4′",
        "the finder camera has no live pointing yet",
        "is never read as a push and never a sync",
        "Track on always does: centre with Track on",
    ] {
        assert!(section.contains(words), "the section lacks {words:?}");
    }

    let direct = "**Direct axis move**:\nMoving an axis to an axis position, or at a rate for a set time, asked for by axis rather than by target, or both axes by one nudge's step. MCP clients ask for the first two; a nudge with Track off is the third. Each needs a sync and passes the same guards as a GoTo.\n_Avoid_: raw move, manual slew\n";
    assert!(
        glossary.contains(direct),
        "the glossary lacks the direct axis move"
    );
    let nudge = "**Nudge**:\nOne press of a nudge arrow: the telescope moves one step on the sky, fine 30″ or coarse 4′, up, right, down or left in the view the arrows follow: the mount's own (up is higher altitude, right a clockwise turn), the finder camera's live views as the page turns them, or the imaging camera's frame from its latest solved imaging solve. With Track off it is a direct axis move; with Track on the place Track holds moves by the step and Track makes it. A nudge is never a sync, never a slip and never read as a push.\n_Avoid_: jog, slew, manual move\n";
    assert!(glossary.contains(nudge), "the glossary lacks the nudge");
}
