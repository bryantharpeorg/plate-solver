//! Oracles: the page's route JSON (exact), generated PGM dimensions (exact),
//! and simulator truth at mid-exposure (1 arcminute). Link age tolerance: 0.1 s.
#[path = "mcp/support.rs"]
mod support;

use axum::{body::Body, http::Request, Router};
use base64::prelude::{Engine as _, BASE64_STANDARD};
use finder::api::SOLVE_BODY_LIMIT;
use math_core::{angular_distance, UnitVector};
use serde_json::{json, Value};
use std::{path::Path, time::Duration};
use support::*;

fn orion() -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../reference-solutions/cedar-solve/examples/data/medium_fov/orion2.jpg"),
    )
    .unwrap()
}

async fn tool(app: &Router, name: &str, arguments: Value) -> Value {
    let response = send(app, call(name, arguments)).await;
    assert_eq!(response.status(), 200);
    let rpc = body(response).await;
    assert!(rpc.get("error").is_none(), "{rpc}");
    rpc["result"].clone()
}

fn assert_json(result: &Value, structured: Value, text: &Value) {
    assert_ne!(result["isError"], true, "{result}");
    assert_eq!(result["structuredContent"], structured);
    assert_eq!(result["content"].as_array().unwrap().len(), 1);
    assert_eq!(result["content"][0]["type"], "text");
    assert_eq!(
        serde_json::from_str::<Value>(result["content"][0]["text"].as_str().unwrap()).unwrap(),
        *text
    );
}

async fn solve_frame(app: &Router, bytes: &[u8], name: Option<&str>) -> Value {
    let mut args = json!({"image":BASE64_STANDARD.encode(bytes)});
    if let Some(name) = name {
        args["name"] = json!(name);
    }
    tool(app, "solve_frame", args).await
}

async fn upload(app: &Router, bytes: &[u8]) -> (u16, Value) {
    let mut form = b"--mcp-frame\r\nContent-Disposition: form-data; name=\"image\"; filename=\"orion2.jpg\"\r\nContent-Type: image/jpeg\r\n\r\n".to_vec();
    form.extend_from_slice(bytes);
    form.extend_from_slice(b"\r\n--mcp-frame--\r\n");
    let response = send(
        app,
        Request::post("/api/solve")
            .header("content-type", "multipart/form-data; boundary=mcp-frame")
            .body(Body::from(form))
            .unwrap(),
    )
    .await;
    (response.status().as_u16(), body(response).await)
}

#[tokio::test]
async fn saved_frame_matches_the_page_and_sets_the_same_latest_pointing() {
    let page = app(state(), "127.0.0.1");
    let app = app(state(), "127.0.0.1");
    let bytes = orion();
    let (status, expected) = upload(&page, &bytes).await;
    assert_eq!(status, 200);
    assert_eq!(expected["status"], "match_found");
    let result = solve_frame(&app, &bytes, Some("orion2.jpg")).await;
    assert_ne!(result["isError"], true, "{result}");
    let report = &result["structuredContent"];
    assert_eq!(report["status"], "match_found");
    for field in ["ra_deg", "dec_deg", "roll_deg"] {
        assert!(expected["pointing"][field].is_number());
        assert_eq!(report["pointing"][field], expected["pointing"][field]);
    }
    assert_eq!(report["name"], "orion2.jpg");
    assert_json(&result, report.clone(), report);
    let view = session(&app).await;
    assert!(view["pointing"].is_object());
    assert_eq!(view["pointing"], session(&page).await["pointing"]);
    assert_json(
        &tool(&app, "get_pointing", json!({})).await,
        json!({"pointing":view["pointing"]}),
        &view["pointing"],
    );
}

// P5's comment is padding, not pixel data: even a 32 MiB file decodes to a
// tiny, valid grayscale image. No generated image is stored in the repository.
fn pgm(length: usize) -> Vec<u8> {
    let mut bytes = b"P5\n#".to_vec();
    let suffix = b"\n16 12\n255\n";
    bytes.resize(length - suffix.len() - 16 * 12, b'x');
    bytes.extend_from_slice(suffix);
    bytes.resize(length, 0);
    assert_eq!(bytes.len(), length);
    bytes
}

#[tokio::test]
async fn decoded_limit_accepts_exactly_32_mib_and_refuses_one_more_byte_atomically() {
    let app = app(state(), "127.0.0.1");
    // A non-null pointing makes the unchanged assertion detect accidental clearing.
    let seeded = solve_frame(&app, &orion(), None).await;
    assert_eq!(seeded["structuredContent"]["status"], "match_found");
    let before = session(&app).await["pointing"].clone();
    assert!(before.is_object());

    let accepted = solve_frame(&app, &pgm(SOLVE_BODY_LIMIT), None).await;
    assert_ne!(accepted["isError"], true, "{accepted}");
    let report = &accepted["structuredContent"];
    assert_eq!(report["width"], 16);
    assert_eq!(report["height"], 12);
    assert_eq!(report["status"], "too_few");
    assert_eq!(report["name"], Value::Null);
    assert_json(&accepted, report.clone(), report);
    assert_eq!(session(&app).await["pointing"], before);

    let refused = solve_frame(&app, &pgm(SOLVE_BODY_LIMIT + 1), Some("large.pgm")).await;
    assert_eq!(refused["isError"], true, "{refused}");
    assert_eq!(
        refused["content"],
        json!([{"type":"text", "text":"the image is larger than POST /api/solve's 32 MiB limit"}])
    );
    assert_eq!(session(&app).await["pointing"], before);
}

#[tokio::test]
async fn malformed_base64_and_unreadable_images_are_tool_errors() {
    let app = app(state(), "127.0.0.1");
    let seeded = solve_frame(&app, &orion(), None).await;
    assert_eq!(seeded["structuredContent"]["status"], "match_found");
    let before = session(&app).await["pointing"].clone();
    let invalid = tool(&app, "solve_frame", json!({"image":"%%%"})).await;
    assert_eq!(invalid["isError"], true, "{invalid}");
    assert!(!invalid["content"][0]["text"].as_str().unwrap().is_empty());
    let (status, expected) = upload(&app, b"not an image").await;
    assert_eq!(status, 415);
    let invalid = solve_frame(&app, b"not an image", None).await;
    assert_eq!(invalid["isError"], true, "{invalid}");
    assert_eq!(invalid["content"][0]["text"], expected["error"]);
    assert_eq!(session(&app).await["pointing"], before);
}

#[tokio::test]
async fn empty_pointing_and_link_keep_the_sessions_null_fields() {
    let app = app(state(), "127.0.0.1");
    let view = session(&app).await;
    for (name, field) in [("get_pointing", "pointing"), ("get_link", "node")] {
        assert!(view[field].is_null());
        assert_json(
            &tool(&app, name, json!({})).await,
            json!({field:view[field]}),
            &view[field],
        );
    }
}

#[tokio::test(start_paused = true)]
async fn live_pointings_follow_the_true_eyepiece_at_mid_exposure() {
    let rig = sim::Rig::new(sim::START, 35.0, 150.0, true).await;
    rig.sync().await;
    let live = rig.state.live().unwrap();
    let mut events = live.subscribe();
    let mut ids = Vec::new();
    for _ in 0..5 {
        let event = tokio::time::timeout(Duration::from_secs(5), events.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(event.report.status, finder::solve::Status::MatchFound);
        let result = tool(&rig.app, "get_pointing", json!({})).await;
        let p = &result["structuredContent"]["pointing"];
        assert_eq!(p["source"], "live");
        assert_eq!(p["frame_id"], event.frame_id);
        assert_eq!(p["time"], event.time);
        let frame = rig.state.link().unwrap().frames().borrow().clone().unwrap();
        assert_eq!(frame.id, event.frame_id);
        // Independent clock mapping: the simulator's node starts at 1000 s,
        // while its console starts at START. Use the exposure, not receipt time.
        let mid_time = (frame.start_s + frame.end_s) / 2.0 + (sim::START - 1000.0);
        assert!((p["time"].as_f64().unwrap() - mid_time).abs() < 1e-6);
        let truth = rig.sim.directions_at(mid_time).eyepiece;
        let error = angular_distance(
            UnitVector::from_radec(
                p["ra_deg"].as_f64().unwrap().to_radians(),
                p["dec_deg"].as_f64().unwrap().to_radians(),
            ),
            UnitVector::from_radec(truth.ra.to_radians(), truth.dec.to_radians()),
        )
        .to_degrees()
            * 60.0;
        assert!(error < 1.0, "frame {}: {error} arcmin", event.frame_id);
        assert_json(
            &result,
            json!({"pointing":session(&rig.app).await["pointing"]}),
            p,
        );
        ids.push(event.frame_id);
    }
    assert!(ids.windows(2).all(|pair| pair[1] > pair[0]));
}

#[tokio::test(start_paused = true)]
async fn link_reports_disconnection_two_seconds_after_the_node_stops_answering() {
    let rig = sim::Rig::new(sim::START, 35.0, 150.0, true).await;
    rig.sync().await;
    let connected = tool(&rig.app, "get_link", json!({})).await;
    assert_eq!(connected["structuredContent"]["node"]["connected"], true);
    assert!(
        connected["structuredContent"]["node"]["stars"]
            .as_u64()
            .unwrap()
            > 0
    );
    // Rig exposes the NodeService's controls, as opposed to an unrelated
    // simulator control handle: this stops heartbeats as well as frames.
    rig.controls.set_unresponsive(true);
    let stopped = rig.now();
    for _ in 0..20 {
        rig.tick().await;
    }
    assert!((rig.now() - stopped - 2.0).abs() <= 0.1);
    let result = tool(&rig.app, "get_link", json!({})).await;
    assert_eq!(result["structuredContent"]["node"]["connected"], false);
    let mut actual = result["structuredContent"]["node"].clone();
    let mut expected = session(&rig.app).await["node"].clone();
    let actual_age = actual
        .as_object_mut()
        .unwrap()
        .remove("last_frame_age_s")
        .unwrap();
    let expected_age = expected
        .as_object_mut()
        .unwrap()
        .remove("last_frame_age_s")
        .unwrap();
    assert!((actual_age.as_f64().unwrap() - expected_age.as_f64().unwrap()).abs() <= 0.1);
    assert_eq!(actual, expected);
    assert_json(
        &result,
        result["structuredContent"].clone(),
        &result["structuredContent"]["node"],
    );
}

#[tokio::test]
async fn tools_list_includes_frames_pointing_and_link_with_read_only_hints() {
    let app = app(state(), "127.0.0.1");
    let response = send(
        &app,
        request(json!({"jsonrpc":"2.0", "id":2, "method":"tools/list"})),
    )
    .await;
    assert_eq!(response.status(), 200);
    let list = body(response).await;
    let tools = list["result"]["tools"].as_array().unwrap();
    for name in ["solve_frame", "get_pointing", "get_link"] {
        let tool = tools.iter().find(|tool| tool["name"] == name).unwrap();
        assert_eq!(tool["outputSchema"], json!({"type":"object"}));
        if name != "solve_frame" {
            assert_eq!(tool["annotations"]["readOnlyHint"], true);
        }
    }
}
