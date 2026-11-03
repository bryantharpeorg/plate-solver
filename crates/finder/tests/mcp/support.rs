//! Router-only MCP requests. All expectations come from routes or the MCP contract.
use std::net::SocketAddr;
use std::path::Path;
use std::sync::{Arc, LazyLock};

use axum::body::{to_bytes, Body};
use axum::extract::connect_info::MockConnectInfo;
use axum::http::Request;
use axum::response::Response;
use axum::Router;
use finder::samples::Samples;
use finder::session::FixedClock;
use finder::{router, AppState};
use pattern_database::PatternDatabase;
use serde_json::{json, Value};
use tower::ServiceExt;

static DB: LazyLock<Arc<PatternDatabase>> = LazyLock::new(|| {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../reference-solutions/cedar-solve/tetra3/data/default_database.npz");
    Arc::new(pattern_database::load_from_path(&path).unwrap())
});

pub fn state() -> AppState {
    AppState::shared(DB.clone(), Samples::default()).with_clock(Arc::new(FixedClock(1768528800.0)))
}

pub fn app(state: AppState, peer: &str) -> Router {
    router(state).layer(MockConnectInfo(SocketAddr::new(
        peer.parse().unwrap(),
        12345,
    )))
}

pub fn request(message: Value) -> Request<Body> {
    Request::post("/mcp")
        .header("host", "localhost:8080")
        .header("content-type", "application/json")
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-11-25")
        .body(Body::from(message.to_string()))
        .unwrap()
}

pub fn call(name: &str, arguments: Value) -> Request<Body> {
    request(json!({"jsonrpc":"2.0", "id":1, "method":"tools/call",
        "params":{"name":name, "arguments":arguments}}))
}

pub async fn send(app: &Router, request: Request<Body>) -> Response {
    app.clone().oneshot(request).await.unwrap()
}

pub async fn body(response: Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

pub async fn session(app: &Router) -> Value {
    let response = send(
        app,
        Request::get("/api/session").body(Body::empty()).unwrap(),
    )
    .await;
    assert_eq!(response.status(), 200);
    body(response).await
}
