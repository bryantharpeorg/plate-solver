//! The MCP tools refuse the chart rotation. Oracles: the tools' exact refusal
//! words and an unchanged session read (US1-S4).

use crate::support::mcp as support;

use axum::Router;
use serde_json::{json, Value};
use support::*;

async fn tool(app: &Router, name: &str, arguments: Value) -> Value {
    let response = send(app, call(name, arguments)).await;
    assert_eq!(response.status(), 200);
    let rpc = body(response).await;
    assert!(rpc.get("error").is_none(), "{rpc}");
    rpc["result"].clone()
}

fn assert_refusal(result: &Value, reason: &str) {
    assert_eq!(result["isError"], true, "{result}");
    assert_eq!(result["content"], json!([{"type":"text", "text":reason}]));
}

#[tokio::test]
async fn both_tools_refuse_a_rotation_and_change_nothing() {
    let app = app(state(), "127.0.0.1");
    let before = session(&app).await;
    let arguments = json!({"target": "M42", "chart_rotation_deg": 90});
    assert_refusal(
        &tool(&app, "set_target", arguments.clone()).await,
        "set_target changes only target",
    );
    assert_refusal(
        &tool(&app, "set_session", arguments).await,
        "set_session changes only site, min_alt_deg and target",
    );
    assert_eq!(session(&app).await, before);
}
