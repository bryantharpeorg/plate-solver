//! Oracles: the pinned extract's common names and literal canonical ids, and the
//! route's JSON for exact parity with the MCP tool's structured and text content.

use crate::support::mcp as support;

use axum::{body::Body, http::Request};
use serde_json::{json, Value};

#[tokio::test]
async fn whole_names_find_the_same_object_through_the_route_and_tool() {
    let app = support::app(support::state(), "127.0.0.1");
    for (query, encoded, expected) in [
        ("Orion Nebula", "Orion+Nebula", "NGC 1976"),
        ("Great Orion Nebula", "Great+Orion+Nebula", "NGC 1976"),
        ("Ring Nebula", "Ring+Nebula", "NGC 6720"),
        ("Whirlpool Galaxy", "Whirlpool+Galaxy", "NGC 5194"),
        ("Double Cluster", "Double+Cluster", "C 14"),
        ("Cat’s Eye", "Cat%E2%80%99s+Eye", "NGC 6543"),
        ("Horse Head", "Horse+Head", "B 33"),
    ] {
        let response = support::send(
            &app,
            Request::get(format!("/api/targets?q={encoded}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(response.status(), 200, "{query}");
        let route = support::body(response).await;
        let ids: Vec<_> = route
            .as_array()
            .unwrap()
            .iter()
            .map(|target| target["id"].as_str().unwrap())
            .collect();
        assert_eq!(ids, [expected], "{query}");

        let response =
            support::send(&app, support::call("search_targets", json!({"q": query}))).await;
        assert_eq!(response.status(), 200, "{query}");
        let rpc = support::body(response).await;
        assert!(rpc.get("error").is_none(), "{query}: {rpc}");
        let result = &rpc["result"];
        assert_ne!(result["isError"], true, "{query}: {rpc}");
        assert_eq!(result["structuredContent"], json!({"targets": route}));
        let content = result["content"].as_array().unwrap();
        assert_eq!(content.len(), 1, "{query}");
        assert_eq!(content[0]["type"], "text");
        assert_eq!(
            serde_json::from_str::<Value>(content[0]["text"].as_str().unwrap()).unwrap(),
            route,
            "{query}"
        );
    }
}
