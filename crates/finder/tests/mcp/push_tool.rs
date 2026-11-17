//! Oracle: FR-011's contract — the description before this spec, a space, and
//! its one new sentence, read from the tool list the router itself serves.
use crate::support::mcp::{app, body, request, send, state};
use serde_json::json;

#[tokio::test]
async fn set_tracking_description_gains_the_push_sentence() {
    let app = app(state(), "127.0.0.1");
    let rpc = body(
        send(
            &app,
            request(json!({"jsonrpc":"2.0","id":1,"method":"tools/list"})),
        )
        .await,
    )
    .await;
    let description = rpc["result"]["tools"]
        .as_array()
        .expect("a tool list")
        .iter()
        .find(|tool| tool["name"] == "set_tracking")
        .expect("set_tracking listed")["description"]
        .as_str()
        .expect("a description")
        .to_string();
    assert!(description.contains("real telescope"), "{description}");
    // FR-011: the sentence is appended, and the description is otherwise
    // unchanged. The description before this spec is quoted from it.
    let before = "Move a real telescope by turning Track on for the session's target, \
or stop all motion by turning it off, as the page's Track toggle does.";
    let sentence =
        "Turning Track on while it follows a push by hand sends the telescope back to the target.";
    assert_eq!(description, format!("{before} {sentence}"));
}
