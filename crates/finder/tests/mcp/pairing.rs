//! 056 US3-S1/S2: exact protocol refusals and route/session/log oracles; no tolerance.
use std::io::{self, Write};
use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::Request;
use axum::response::Response;
use axum::Router;
use serde_json::{json, Value};

use crate::support::mcp::*;
use crate::support::pairing::{clock, gated, get, COOKIE, HOST, LIMITED, PHONE, TOKEN, WRONG};

const MISSING: &str = "MCP from another device needs the console token as a bearer token";

fn authenticated(mut request: Request<Body>, header: (&str, &str)) -> Request<Body> {
    request.headers_mut().insert(
        axum::http::HeaderName::from_bytes(header.0.as_bytes()).unwrap(),
        header.1.parse().unwrap(),
    );
    request
}

async fn refusal(response: Response, status: u16, message: &str) {
    assert_eq!(response.status(), status);
    assert_eq!(
        response.headers()["www-authenticate"],
        "Bearer realm=\"finder console\""
    );
    assert_eq!(
        body(response).await,
        json!({"jsonrpc":"2.0", "error":{"code":-32001, "message":message}})
    );
}

async fn local_session(local: &Router) -> Value {
    let response = send(local, get("/api/session", Some("localhost:8080"), None)).await;
    assert_eq!(response.status(), 200);
    body(response).await
}

#[tokio::test]
async fn us3_s1_mcp_requires_the_console_token_after_the_lan_checks() {
    let state = gated(clock());
    let phone = app(state.clone(), PHONE);
    let local = app(state.clone(), "127.0.0.1");
    let before = local_session(&local).await;
    for (name, arguments) in [
        ("get_session", json!({})),
        ("set_target", json!({"target":"M42"})),
    ] {
        refusal(
            send(&phone, call(name, arguments.clone())).await,
            401,
            MISSING,
        )
        .await;
        refusal(
            send(
                &phone,
                authenticated(call(name, arguments), ("authorization", "Bearer NOPE-NOPE")),
            )
            .await,
            401,
            WRONG,
        )
        .await;
    }
    assert_eq!(local_session(&local).await, before);

    let bearer = format!("Bearer {TOKEN}");
    let response = send(
        &phone,
        authenticated(call("get_session", json!({})), ("authorization", &bearer)),
    )
    .await;
    assert_eq!(response.status(), 200);
    assert_eq!(body(response).await["result"]["structuredContent"], before);
    let response = send(
        &phone,
        authenticated(call("get_session", json!({})), ("cookie", COOKIE)),
    )
    .await;
    assert_eq!(response.status(), 200);
    let response = send(
        &phone,
        authenticated(
            call("set_target", json!({"target":"M42"})),
            ("authorization", &bearer),
        ),
    )
    .await;
    assert_eq!(response.status(), 200);
    assert_eq!(local_session(&local).await["target"]["messier"], "M42");
    assert_eq!(
        send(&local, call("get_session", json!({}))).await.status(),
        200
    );

    let public = app(state, "203.0.113.7");
    assert_eq!(
        send(
            &public,
            authenticated(call("get_session", json!({})), ("authorization", &bearer))
        )
        .await
        .status(),
        403
    );
    assert_eq!(
        send(&public, call("get_session", json!({}))).await.status(),
        403
    );
    assert_eq!(
        send(
            &app(crate::support::mcp::state(), PHONE),
            call("get_session", json!({}))
        )
        .await
        .status(),
        200
    );
}

#[derive(Clone, Default)]
struct Log(Arc<Mutex<Vec<u8>>>);
impl Write for Log {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl Log {
    fn text(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }
}

#[tokio::test]
async fn us3_s2_stop_runs_before_refusal_and_wrong_tokens_share_the_route_limit() {
    let log = Log::default();
    let phone = app(gated(clock()).with_call_log(log.clone()), PHONE);
    refusal(send(&phone, call("stop", json!({}))).await, 401, MISSING).await;
    assert_eq!(log.text(), "mcp stop {} -> {\"stopped\":true}\n");
    for _ in 0..10 {
        refusal(
            send(
                &phone,
                authenticated(
                    call("get_session", json!({})),
                    ("authorization", "Bearer NOPE-NOPE"),
                ),
            )
            .await,
            401,
            WRONG,
        )
        .await;
    }
    let bearer = format!("Bearer {TOKEN}");
    refusal(
        send(
            &phone,
            authenticated(call("get_session", json!({})), ("authorization", &bearer)),
        )
        .await,
        429,
        LIMITED,
    )
    .await;
    let response = send(
        &phone,
        get("/api/session", Some(HOST), Some(("cookie", COOKIE))),
    )
    .await;
    assert_eq!(response.status(), 429);
    assert_eq!(body(response).await, json!({"error":LIMITED}));
    assert_eq!(log.text(), "mcp stop {} -> {\"stopped\":true}\n");
    assert!(!log.text().to_lowercase().contains(TOKEN));
}
