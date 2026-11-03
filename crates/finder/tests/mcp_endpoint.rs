//! Exact route JSON is the oracle for tools; MCP HTTP statuses and the address
//! policy are specified examples. Every comparison has zero tolerance.
#[path = "mcp/support.rs"]
mod support;

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use finder::mcp::{AddressSource, McpConfig};
use finder::router;
use finder::session::Session;
use serde_json::{json, Value};
use support::*;

#[tokio::test]
async fn initialize_and_list_are_json_without_authorization_or_session_id() {
    let app = app(state(), "127.0.0.1");
    let req = request(json!({"jsonrpc":"2.0", "id":1, "method":"initialize",
        "params":{"protocolVersion":"2025-11-25", "capabilities":{},
            "clientInfo":{"name":"endpoint-test", "version":"1"}}}));
    assert!(!req.headers().contains_key("authorization"));
    let response = send(&app, req).await;
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers()["content-type"], "application/json");
    assert!(!response.headers().contains_key("mcp-session-id"));
    let init = body(response).await;
    assert_eq!(init["result"]["serverInfo"]["name"], "finder");
    assert!(init["result"]["capabilities"]["tools"].is_object());
    let req = request(json!({"jsonrpc":"2.0", "id":2, "method":"tools/list"}));
    assert!(!req.headers().contains_key("mcp-session-id"));
    let response = send(&app, req).await;
    assert_eq!(response.status(), 200);
    let list = body(response).await;
    let tools = list["result"]["tools"].as_array().unwrap();
    for (name, read_only) in [("get_session", true), ("set_target", false)] {
        let tool = tools.iter().find(|tool| tool["name"] == name).unwrap();
        assert_eq!(
            tool["annotations"]["readOnlyHint"]
                .as_bool()
                .unwrap_or(false),
            read_only
        );
        assert_eq!(tool["outputSchema"], json!({"type":"object"}));
    }
}

#[tokio::test]
async fn get_is_405_and_preserves_the_state_file_bytes() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../../target/mcp-state-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("session.json");
    let app = app(
        state().with_session(Session::with_state_file(&path).unwrap()),
        "127.0.0.1",
    );
    let response = send(&app, call("set_target", json!({"target":"M42"}))).await;
    assert_eq!(response.status(), 200);
    assert_ne!(body(response).await["result"]["isError"], true);
    let before = std::fs::read(&path).unwrap();
    let mut req = request(json!({}));
    *req.method_mut() = Method::GET;
    let response = send(&app, req).await;
    assert_eq!(response.status(), 405);
    assert_eq!(response.headers()["allow"], "POST");
    assert_eq!(std::fs::read(&path).unwrap(), before);
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn disabled_mcp_uses_the_page_fallback_and_runs_no_tool() {
    let app = app(
        state().with_mcp(McpConfig {
            enabled: false,
            ..Default::default()
        }),
        "127.0.0.1",
    );
    let before = session(&app).await;
    let initialize = json!({"jsonrpc":"2.0", "id":1, "method":"initialize",
        "params":{"protocolVersion":"2025-11-25", "capabilities":{},
        "clientInfo":{"name":"test", "version":"1"}}});
    let mut missing = request(initialize.clone());
    *missing.uri_mut() = "/no-such-path".parse().unwrap();
    let fallback = send(&app, missing).await.status();
    let response = send(&app, request(initialize)).await;
    assert_eq!(response.status(), fallback);
    assert_eq!(
        send(&app, call("set_target", json!({"target":"M42"})))
            .await
            .status(),
        fallback
    );
    assert_eq!(session(&app).await, before);
}

#[tokio::test]
async fn unsupported_version_refuses_and_notifications_are_accepted() {
    let app = app(state(), "127.0.0.1");
    let before = session(&app).await;
    for name in ["get_session", "set_target"] {
        let mut req = call(name, json!({"target":"M42"}));
        req.headers_mut()
            .insert("mcp-protocol-version", "1999-01-01".parse().unwrap());
        assert_eq!(send(&app, req).await.status(), 400);
        assert_eq!(session(&app).await, before);
    }
    let response = send(
        &app,
        request(json!({"jsonrpc":"2.0", "method":"notifications/initialized"})),
    )
    .await;
    assert_eq!(response.status(), 202);
}

#[tokio::test]
async fn foreign_origins_rebinding_and_missing_or_malformed_hosts_refuse_edits() {
    let app = app(state(), "127.0.0.1");
    let before = session(&app).await;
    assert!(before["target"].is_null());
    for (host, origin) in [
        (Some("localhost:8080"), Some("http://elsewhere.example")),
        (Some("evil.example:8080"), Some("http://evil.example:8080")),
        (None, None),
        (Some("evil.example:8080"), None),
        (Some("user@localhost:8080"), None),
        (Some("localhost:65536"), None),
        (Some("localhost:invalid"), None),
        (Some("localhost:8080"), Some("https://localhost:8080")),
        (Some("localhost:8080"), Some("http://localhost:8081")),
        (Some("localhost:8080"), Some("null")),
        (Some("localhost:8080"), Some("http://localhost:8080/path")),
    ] {
        let mut req = call("set_target", json!({"target":"M42"}));
        req.headers_mut().remove("host");
        if let Some(host) = host {
            req.headers_mut().insert("host", host.parse().unwrap());
        }
        if let Some(origin) = origin {
            req.headers_mut().insert("origin", origin.parse().unwrap());
        }
        let response = send(&app, req).await;
        assert_eq!(response.status(), 403, "{host:?} {origin:?}");
        let error = body(response).await;
        assert_eq!(error["jsonrpc"], "2.0");
        assert!(error["error"]["message"].is_string());
        assert!(error.get("id").is_none());
        assert_eq!(session(&app).await, before);
    }
    for header in ["host", "origin"] {
        let mut req = call("set_target", json!({"target":"M42"}));
        req.headers_mut()
            .append(header, "localhost:8080".parse().unwrap());
        if header == "origin" {
            req.headers_mut()
                .append("origin", "http://localhost:8080".parse().unwrap());
        }
        assert_eq!(send(&app, req).await.status(), 403);
        assert_eq!(session(&app).await, before);
    }
    // Positive control: the identical tool edit runs through the guarded endpoint.
    let response = send(&app, call("set_target", json!({"target":"M42"}))).await;
    assert_eq!(response.status(), 200);
    assert_eq!(
        body(response).await["result"]["structuredContent"]["target"]["messier"],
        "M42"
    );
}

#[tokio::test]
async fn ip_literals_localhost_and_only_exact_allowed_names_are_accepted() {
    let app = app(
        state().with_allowed_hosts(["surface.local".into()]),
        "127.0.0.1",
    );
    let expected = session(&app).await;
    for (host, origin) in [
        ("192.168.1.20:8080", None),
        ("surface.local:8080", None),
        ("SURFACE.LOCAL:8080", Some("http://surface.local:8080")),
        ("[::1]:8080", Some("http://[::1]:8080")),
        ("localhost:8080", Some("http://localhost:8080")),
    ] {
        let mut req = call("get_session", json!({}));
        req.headers_mut().insert("host", host.parse().unwrap());
        if let Some(origin) = origin {
            req.headers_mut().insert("origin", origin.parse().unwrap());
        }
        let response = send(&app, req).await;
        assert_eq!(response.status(), 200, "{host}");
        assert_eq!(
            body(response).await["result"]["structuredContent"],
            expected
        );
    }
    for host in ["surface.local.evil.example:8080", "sub.surface.local:8080"] {
        let mut req = call("get_session", json!({}));
        req.headers_mut().insert("host", host.parse().unwrap());
        assert_eq!(send(&app, req).await.status(), 403);
    }
}

#[tokio::test]
async fn lan_peers_pass_and_public_peers_or_absent_connect_info_fail_closed() {
    let state = state();
    let local = app(state.clone(), "127.0.0.1");
    let before = session(&local).await;
    for peer in [
        "127.0.0.1",
        "192.168.1.30",
        "172.24.0.1",
        "169.254.1.2",
        "::1",
        "fe80::1",
        "fd00::30",
        "::ffff:192.168.1.30",
    ] {
        let app = app(state.clone(), peer);
        let response = send(&app, call("get_session", json!({}))).await;
        assert_eq!(response.status(), 200, "{peer}");
        assert_eq!(body(response).await["result"]["structuredContent"], before);
    }
    for app in [
        app(state.clone(), "203.0.113.7"),
        app(state.clone(), "::ffff:203.0.113.7"),
        router(state),
    ] {
        for name in ["get_session", "set_target"] {
            let mut req = call(name, json!({"target":"M42"}));
            req.headers_mut()
                .insert("host", "192.168.1.20:8080".parse().unwrap());
            assert_eq!(send(&app, req).await.status(), 403);
            assert_eq!(session(&app).await, before);
        }
    }
    assert_eq!(
        send(&local, call("set_target", json!({"target":"M42"})))
            .await
            .status(),
        200
    );
    assert_eq!(session(&local).await["target"]["messier"], "M42");
}

fn assert_tool_json(result: &Value, expected: &Value) {
    assert_ne!(result["isError"], true, "{result}");
    assert_eq!(&result["structuredContent"], expected);
    assert_eq!(result["content"][0]["type"], "text");
    assert_eq!(result["content"][0]["text"], expected.to_string());
}

#[tokio::test]
async fn tools_share_the_routes_json_and_target_edit_parser() {
    let app = app(state(), "127.0.0.1");
    for target in [json!("M42"), Value::Null] {
        let response = send(&app, call("get_session", json!({}))).await;
        assert_eq!(response.status(), 200);
        assert_tool_json(&body(response).await["result"], &session(&app).await);
        let edit = json!({"target":target});
        let route = send(
            &app,
            Request::put("/api/session")
                .header("content-type", "application/json")
                .body(Body::from(edit.to_string()))
                .unwrap(),
        )
        .await;
        assert_eq!(route.status(), 200);
        let expected = body(route).await;
        let response = send(&app, call("set_target", edit)).await;
        assert_eq!(response.status(), 200);
        assert_tool_json(&body(response).await["result"], &expected);
    }
    for edit in [
        json!({"target":42}),
        json!({"target":"NGC 99999"}),
        json!({"sun_guard":false}),
    ] {
        let before = session(&app).await;
        let route = send(
            &app,
            Request::put("/api/session")
                .header("content-type", "application/json")
                .body(Body::from(edit.to_string()))
                .unwrap(),
        )
        .await;
        assert!(route.status().is_client_error());
        let expected = body(route).await;
        let response = send(&app, call("set_target", edit)).await;
        assert_eq!(response.status(), 200);
        let result = body(response).await;
        assert_eq!(result["result"]["isError"], true);
        assert_eq!(result["result"]["content"][0]["text"], expected["error"]);
        assert_eq!(session(&app).await, before);
    }
}

struct Addresses(Vec<IpAddr>);
impl AddressSource for Addresses {
    fn addresses(&self) -> Vec<IpAddr> {
        self.0.clone()
    }
}

#[tokio::test]
async fn setup_reports_lan_urls_local_only_and_disabled_using_the_bound_port() {
    for (bound, enabled, local_only, urls) in [
        (
            "0.0.0.0:8080",
            true,
            false,
            vec![
                "http://localhost:8080/mcp",
                "http://192.168.1.20:8080/mcp",
                "http://169.254.1.2:8080/mcp",
                "http://[fe80::1]:8080/mcp",
                "http://[fd00::30]:8080/mcp",
            ],
        ),
        (
            "127.0.0.1:8080",
            true,
            true,
            vec!["http://localhost:8080/mcp"],
        ),
        (
            "127.0.0.1:43210",
            false,
            true,
            vec!["http://localhost:43210/mcp"],
        ),
    ] {
        let config = McpConfig {
            enabled,
            bound_addr: bound.parse::<SocketAddr>().unwrap(),
            address_source: Arc::new(Addresses(
                [
                    "2001:db8::1",
                    "fd00::30",
                    "127.0.0.1",
                    "::1",
                    "192.168.1.20",
                    "169.254.1.2",
                    "fe80::1",
                ]
                .map(|ip| ip.parse().unwrap())
                .to_vec(),
            )),
        };
        let app = app(
            state()
                .with_allowed_hosts(["surface.local".into()])
                .with_mcp(config),
            "127.0.0.1",
        );
        let before = session(&app).await;
        let response = send(&app, Request::get("/api/mcp").body(Body::empty()).unwrap()).await;
        assert_eq!(response.status(), 200);
        assert_eq!(
            body(response).await,
            json!({"enabled":enabled, "local_only":local_only,
            "urls":urls, "allowed_hosts":["surface.local"]})
        );
        assert_eq!(session(&app).await, before);
    }
}

#[tokio::test]
async fn target_tool_refuses_every_other_session_field_without_an_edit() {
    let app = app(state(), "127.0.0.1");
    let before = session(&app).await;
    assert!(before["target"].is_null());
    assert!(before["site"].is_null());
    for extra in [
        json!({"node":{"token":"x"}}),
        json!({"site":{"lat_deg":40,"lon_deg":-75}}),
        json!({"site":null}),
        json!({"node":null}),
        json!({"imaging_camera":null}),
        json!({"min_alt_deg":30}),
        json!({"camera_settings":{}}),
    ] {
        let mut edit = extra;
        edit["target"] = json!("M42");
        let response = send(&app, call("set_target", edit)).await;
        assert_eq!(response.status(), 200);
        let result = body(response).await;
        assert_eq!(result["result"]["isError"], true);
        assert_eq!(
            result["result"]["content"][0]["text"],
            "set_target changes only target"
        );
        assert_eq!(session(&app).await, before);
    }
}

#[tokio::test]
async fn sdk_owns_accept_content_type_and_batch_refusals() {
    let app = app(state(), "127.0.0.1");
    let before = session(&app).await;
    for (header, value, status) in [
        ("accept", None, StatusCode::NOT_ACCEPTABLE),
        (
            "content-type",
            Some("text/plain"),
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
        ),
    ] {
        let mut req = call("set_target", json!({"target":"M42"}));
        req.headers_mut().remove(header);
        if let Some(value) = value {
            req.headers_mut().insert(header, value.parse().unwrap());
        }
        assert_eq!(send(&app, req).await.status(), status);
        assert_eq!(session(&app).await, before);
    }
    let batch = json!([{"jsonrpc":"2.0", "id":1, "method":"tools/call",
        "params":{"name":"set_target", "arguments":{"target":"M42"}}}]);
    assert_eq!(send(&app, request(batch)).await.status(), 415);
    assert_eq!(session(&app).await, before);
}

#[tokio::test]
async fn forty_four_mib_is_accepted_but_one_more_byte_cannot_edit_the_session() {
    let app = app(state(), "127.0.0.1");
    for (target, length, status) in [
        (json!("M42"), 44 * 1024 * 1024, 200),
        (Value::Null, 44 * 1024 * 1024 + 1, 413),
    ] {
        let before = session(&app).await;
        let mut req = call("set_target", json!({"target":target}));
        let mut bytes = axum::body::to_bytes(std::mem::take(req.body_mut()), usize::MAX)
            .await
            .unwrap()
            .to_vec();
        bytes.resize(length, b' ');
        *req.body_mut() = Body::from(bytes);
        let response = send(&app, req).await;
        assert_eq!(response.status(), status);
        if status == 200 {
            assert_eq!(
                body(response).await["result"]["structuredContent"]["target"]["messier"],
                "M42"
            );
        } else {
            assert_eq!(session(&app).await, before);
        }
    }
}
