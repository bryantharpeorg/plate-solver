//! Oracles: route JSON and refusal text (exact), simulator mount connectivity,
//! and 012's solved Orion pointing with sky::alt_az / pushto::turn_tilt (1e-9°).
#[path = "mcp/support.rs"]
mod support;

use axum::{body::Body, http::Request, Router};
use finder::{
    samples::Samples,
    sky::{self, Observer},
};
use serde_json::{json, Value};
use std::path::Path;
use support::*;

async fn get(app: &Router, uri: &str) -> Value {
    let response = send(app, Request::get(uri).body(Body::empty()).unwrap()).await;
    assert_eq!(response.status(), 200);
    body(response).await
}

async fn put(app: &Router, edit: &Value) -> (u16, Value) {
    let response = send(
        app,
        Request::put("/api/session")
            .header("content-type", "application/json")
            .body(Body::from(edit.to_string()))
            .unwrap(),
    )
    .await;
    (response.status().as_u16(), body(response).await)
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
    // Compare JSON exactly, without reserializing the route's already parsed
    // floats (serde_json's default parser need not preserve their spelling).
    assert_eq!(
        serde_json::from_str::<Value>(result["content"][0]["text"].as_str().unwrap()).unwrap(),
        *text
    );
}

fn assert_refusal(result: &Value, reason: &str) {
    assert_eq!(result["isError"], true, "{result}");
    assert_eq!(result["content"], json!([{"type":"text", "text":reason}]));
}

#[tokio::test]
async fn catalog_search_passes_through_route_arrays_as_text_and_targets() {
    let app = app(state(), "127.0.0.1");
    assert_eq!(
        put(&app, &json!({"site":{"lat_deg":40,"lon_deg":-75}}))
            .await
            .0,
        200
    );
    for (arguments, query) in [
        (json!({"q":"orion"}), "q=orion"),
        (json!({"q":"nebula", "limit":1}), "q=nebula&limit=1"),
        (json!({"q":"nebula", "limit":0}), "q=nebula&limit=0"),
        (json!({"q":"nebula", "limit":500}), "q=nebula&limit=500"),
        (json!({"q":"qqq"}), "q=qqq"),
    ] {
        let expected = get(&app, &format!("/api/targets?{query}")).await;
        if query == "q=orion" {
            assert!(!expected.as_array().unwrap().is_empty());
        }
        let result = tool(&app, "search_targets", arguments).await;
        assert_json(&result, json!({"targets":expected}), &expected);
    }
    let response = send(
        &app,
        Request::get("/api/targets?q=%20%20")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(response.status(), 400);
    let reason = body(response).await;
    assert_refusal(
        &tool(&app, "search_targets", json!({"q":"  "})).await,
        reason["error"].as_str().unwrap(),
    );
}

#[tokio::test]
async fn checked_session_edits_keep_route_errors_and_are_atomic() {
    let route_app = app(state(), "127.0.0.1");
    let app = app(state(), "127.0.0.1");
    for edit in [json!({"min_alt_deg":85}), json!({"sun_guard":false})] {
        let before = session(&app).await;
        let (status, error) = put(&app, &edit).await;
        assert_eq!(status, 400);
        let result = tool(&app, "set_session", edit).await;
        assert_refusal(&result, error["error"].as_str().unwrap());
        assert_eq!(session(&app).await, before);
    }
    // Positive control for the same tool and parser, also checking exact PUT parity.
    for edit in [
        json!({"site":{"lat_deg":40,"lon_deg":-75}, "min_alt_deg":30, "target":"M42"}),
        json!({"target":"M78"}),
        json!({"site":null, "target":null, "min_alt_deg":0}),
    ] {
        let (status, expected) = put(&route_app, &edit).await;
        assert_eq!(status, 200);
        let result = tool(&app, "set_session", edit).await;
        assert_json(&result, expected.clone(), &expected);
        assert_eq!(session(&app).await, expected);
    }
}

#[tokio::test]
async fn guidance_is_the_sessions_and_the_solved_orion_turn_and_tilt() {
    let samples = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../reference-solutions/cedar-solve/examples/data/medium_fov");
    let app = app(
        state_with_samples(Samples::scan(&samples).unwrap()),
        "127.0.0.1",
    );
    // With no target the one-field object still carries the route's null value.
    assert_json(
        &tool(&app, "get_guidance", json!({})).await,
        json!({"guidance":null}),
        &json!(null),
    );
    assert_eq!(
        put(
            &app,
            &json!({"site":{"lat_deg":40,"lon_deg":-75},"target":"M78"})
        )
        .await
        .0,
        200
    );
    let response = send(&app, Request::post("/api/solve")
        .header("content-type", "multipart/form-data; boundary=mcp-session")
        .body(Body::from("--mcp-session\r\nContent-Disposition: form-data; name=\"sample\"\r\n\r\norion2.jpg\r\n--mcp-session--\r\n")).unwrap()).await;
    assert_eq!(response.status(), 200);
    assert_eq!(body(response).await["status"], "match_found");
    let view = session(&app).await;
    assert_eq!(view["time"], sim::START);
    let result = tool(&app, "get_guidance", json!({})).await;
    let guidance = &result["structuredContent"]["guidance"];
    assert_eq!(guidance["class"], "pointable");
    assert_json(
        &result,
        json!({"guidance":view["guidance"]}),
        &view["guidance"],
    );
    let observer = Observer {
        lat: 40.0,
        lon: -75.0,
        time: sim::START,
    };
    let pointing = &view["pointing"];
    let held = sky::alt_az(
        pointing["ra_deg"].as_f64().unwrap(),
        pointing["dec_deg"].as_f64().unwrap(),
        &observer,
    );
    let target = finder::targets::find("M78").unwrap();
    let at = sky::alt_az(target.ra_deg, target.dec_deg, &observer);
    let (turn, tilt) = finder::pushto::turn_tilt(held, at);
    for (field, expected) in [("turn_deg", turn), ("tilt_deg", tilt)] {
        let actual = guidance[field].as_f64().unwrap();
        assert!(
            (actual - expected).abs() < 1e-9,
            "{field}: {actual} != {expected}"
        );
    }
}

#[tokio::test]
async fn session_tool_cannot_change_node_credentials_or_other_excluded_fields() {
    let app = app(state(), "127.0.0.1");
    let (status, _) = put(
        &app,
        &json!({"node":{"address":"scope.local:1","token":"test-token"}}),
    )
    .await;
    assert_eq!(status, 200);
    let before = session(&app).await;
    assert_eq!(before["node"]["address"], "scope.local:1");
    assert_eq!(before["node"]["token_set"], true);
    for extra in [
        json!({"node":{"address":"other.local:2"}}),
        json!({"node":{"token":"other-token"}}),
        json!({"node":null}),
        json!({"imaging_camera":null}),
        json!({"camera_settings":{}}),
    ] {
        let mut edit = extra;
        edit["target"] = json!("M42");
        assert_refusal(
            &tool(&app, "set_session", edit).await,
            "set_session changes only site, min_alt_deg and target",
        );
        assert_eq!(session(&app).await, before);
    }
}

#[tokio::test(start_paused = true)]
async fn connected_mount_locks_any_site_key_only_for_mcp() {
    let rig = sim::Rig::new(sim::START, 35.0, 150.0, true).await;
    rig.sync().await; // Prove the live centroid solver and aim point are connected too.
    let before = session(&rig.app).await;
    assert_eq!(before["mount"]["connected"], true);
    assert_eq!(before["site"], json!({"lat_deg":40.0,"lon_deg":-75.0}));
    assert_eq!(before["aim"], json!({"x":700.0,"y":420.0}));
    for site in [
        json!({"lat_deg":40,"lon_deg":75}),
        Value::Null,
        before["site"].clone(),
    ] {
        let edit = json!({"site":site, "min_alt_deg":30, "target":"M78"});
        assert_refusal(
            &tool(&rig.app, "set_session", edit).await,
            "the site cannot change while a mount is connected",
        );
        let after = session(&rig.app).await;
        for field in ["site", "min_alt_deg", "target"] {
            assert_eq!(after[field], before[field], "{field}");
        }
    }
    let result = tool(
        &rig.app,
        "set_session",
        json!({"min_alt_deg":20,"target":"M78"}),
    )
    .await;
    assert_ne!(result["isError"], true, "{result}");
    let changed = session(&rig.app).await;
    assert_eq!(changed["site"], before["site"]);
    assert_eq!(changed["min_alt_deg"], 20.0);
    assert_eq!(changed["target"]["messier"], "M78");
    // The page keeps its existing ability to change site even with the mount connected.
    let edit = json!({"site":{"lat_deg":40,"lon_deg":75}});
    let (status, applied) = put(&rig.app, &edit).await;
    assert_eq!(status, 200);
    assert_eq!(applied["site"], json!({"lat_deg":40.0,"lon_deg":75.0}));
    // A connected scope node without a mount is also allowed to edit the site.
    let unmounted = sim::Rig::new(sim::START, 35.0, 150.0, false).await;
    let result = tool(&unmounted.app, "set_session", edit).await;
    assert_ne!(result["isError"], true, "{result}");
    assert_eq!(session(&unmounted.app).await["site"], applied["site"]);
}

#[tokio::test(start_paused = true)]
async fn simulator_rig_records_mount_commands_at_the_virtual_time() {
    use scope_link::proto::Axis;
    use scope_node::mount::{Axis as MountAxis, Mount};
    use sim::{Command, MountCommand};

    let rig = sim::Rig::new(sim::START, 35.0, 150.0, true).await;
    let mut mount = rig.state.link().unwrap().mount().unwrap();
    rig.commands.lock().unwrap().clear();
    let mut expected = Vec::new();
    mount.goto_axis(Axis::Azimuth, 1000).await.unwrap();
    expected.push(Command {
        at: rig.now(),
        action: MountCommand::GoTo {
            axis: MountAxis::Azimuth,
            position: 1000,
        },
    });
    rig.tick().await;
    mount.rate_axis(Axis::Altitude, 0.5).await.unwrap();
    expected.push(Command {
        at: rig.now(),
        action: MountCommand::Rate {
            axis: MountAxis::Altitude,
            deg_per_s: 0.5,
        },
    });
    assert!(rig.mount.state(MountAxis::Altitude).await.moving);
    rig.tick().await;
    mount.stop_all().await.unwrap();
    expected.push(Command {
        at: rig.now(),
        action: MountCommand::StopAll,
    });
    assert_eq!(*rig.commands.lock().unwrap(), expected);
    // The simulator brakes at 3 deg/s²; allow the commanded axes to come to rest.
    for _ in 0..3 {
        rig.tick().await;
    }
    for axis in [MountAxis::Azimuth, MountAxis::Altitude] {
        assert!(!rig.mount.state(axis).await.moving);
    }
}

#[tokio::test]
async fn list_includes_session_catalog_and_guidance_with_read_only_hints() {
    let app = app(state(), "127.0.0.1");
    let response = send(
        &app,
        request(json!({"jsonrpc":"2.0","id":2,"method":"tools/list"})),
    )
    .await;
    assert_eq!(response.status(), 200);
    let list = body(response).await;
    let tools = list["result"]["tools"].as_array().unwrap();
    for (name, read_only) in [
        ("set_session", false),
        ("search_targets", true),
        ("get_guidance", true),
    ] {
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
