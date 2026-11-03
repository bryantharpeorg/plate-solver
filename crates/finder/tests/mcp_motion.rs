//! US4 oracles: simulator axis values (exact), true eyepiece direction (1 arcmin),
//! recording mount commands (Stop within 0.1 s), legacy JSON and the specified log.
#[path = "mcp/support.rs"]
mod support;

use axum::{body::Body, http::Request, Router};
use finder::{model::separation, session::Session, targets};
use scope_node::mount::{Axis, Mount};
use serde_json::{json, Value};
use std::{
    io::Write,
    sync::{Arc, Mutex},
    time::Duration,
};
use support::{
    sim::{MountCommand, Rig, START},
    *,
};

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

async fn tool(app: &Router, name: &str, arguments: Value) -> Value {
    let response = send(app, call(name, arguments)).await;
    assert_eq!(response.status(), 200);
    let rpc = body(response).await;
    assert!(rpc.get("error").is_none(), "{rpc}");
    rpc["result"].clone()
}
fn success(result: &Value, expected: Value) {
    assert_ne!(result["isError"], true, "{result}");
    assert_eq!(result["structuredContent"], expected);
    assert_eq!(
        serde_json::from_str::<Value>(result["content"][0]["text"].as_str().unwrap()).unwrap(),
        expected
    );
}
async fn page(app: &Router, path: &str, value: Value) -> Value {
    let response = send(
        app,
        Request::post(path)
            .header("host", "localhost:8080")
            .header("content-type", "application/json")
            .body(Body::from(value.to_string()))
            .unwrap(),
    )
    .await;
    assert_eq!(response.status(), 200);
    body(response).await
}
async fn ready() -> Rig {
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    rig.sync().await;
    assert_eq!(session(&rig.app).await["mount"]["connected"], true);
    rig
}
async fn stationary(rig: &Rig) -> bool {
    !rig.mount.state(Axis::Azimuth).await.moving && !rig.mount.state(Axis::Altitude).await.moving
}
async fn start(rig: &Rig) {
    success(
        &tool(&rig.app, "goto_target", json!({})).await,
        json!({"started":true}),
    );
    for _ in 0..20 {
        if !stationary(rig).await {
            return;
        }
        rig.tick().await;
    }
    panic!("accepted GoTo never moved the connected mount");
}
fn source(view: &Value, by: &str, tool: &str) {
    assert_eq!(view["motion"]["by"], by, "{view}");
    assert_eq!(view["motion"]["tool"], tool);
    assert!(view["motion"]["since"].as_f64().unwrap() <= view["time"].as_f64().unwrap());
}
async fn complete(rig: &Rig, by: &str, tool: &str) {
    let mut since = None;
    for _ in 0..1000 {
        let view = session(&rig.app).await;
        match view["goto"]["state"].as_str().unwrap() {
            "done" => {
                assert!(since.is_some(), "must have observed the running motion");
                assert!(view.get("motion").is_none(), "{view}");
                let target = targets::find("M42").unwrap();
                let (alt, az) = simulator::sky::horizontal(
                    target.vector(),
                    simulator::Site::default(),
                    rig.now(),
                );
                let truth = rig.mount.true_direction();
                assert!(separation(az, alt, truth.az, truth.alt) * 3600.0 <= 60.0);
                return;
            }
            "slewing" | "solving" | "correcting" => {
                source(&view, by, tool);
                let first = since.get_or_insert(view["motion"]["since"].clone());
                assert_eq!(*first, view["motion"]["since"]);
            }
            _ => panic!("unexpected GoTo: {view}"),
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    panic!("GoTo never finished");
}

#[tokio::test(start_paused = true)]
async fn axes_equal_the_simulator_exactly_and_goto_needs_no_page_confirmation() {
    let rig = ready().await;
    let descriptions = tool(&rig.app, "describe_axes", json!({})).await;
    let states = tool(&rig.app, "get_axis_state", json!({})).await;
    for (i, (axis, name)) in [(Axis::Azimuth, "azimuth"), (Axis::Altitude, "altitude")]
        .into_iter()
        .enumerate()
    {
        let info = rig.mount.axes()[i];
        assert_eq!(
            descriptions["structuredContent"]["axes"][i],
            json!({
                "axis":name, "counts_per_revolution":info.counts_per_revolution,
                "top_rate_deg_per_s":info.top_rate_deg_per_s, "can_goto":info.can_goto
            })
        );
        let expected = rig.mount.state(axis).await;
        assert_eq!(
            states["structuredContent"]["axes"][i],
            json!({
                "axis":name, "position":expected.position, "rate_deg_per_s":expected.rate_deg_per_s, "moving":expected.moving
            })
        );
    }
    let before = rig.now();
    start(&rig).await;
    assert!(session(&rig.app).await["motion"]["since"].as_f64().unwrap() >= before);
    complete(&rig, "mcp", "goto_target").await;
}

#[tokio::test(start_paused = true)]
async fn page_goto_and_track_have_page_attribution() {
    let rig = ready().await;
    assert_eq!(
        page(&rig.app, "/api/goto", json!({})).await,
        json!({"started":true})
    );
    for _ in 0..4 {
        source(&session(&rig.app).await, "page", "goto");
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    page(&rig.app, "/api/stop", json!({})).await;
    for _ in 0..20 {
        rig.tick().await;
    }
    source(
        &page(&rig.app, "/api/track", json!({"on":true})).await,
        "page",
        "track",
    );
    for _ in 0..1000 {
        let view = session(&rig.app).await;
        source(&view, "page", "track");
        if view["track"]["state"] == "tracking" {
            break;
        }
        assert_eq!(view["track"]["state"], "armed", "{view}");
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    assert_eq!(session(&rig.app).await["track"]["state"], "tracking");
    source(&session(&rig.app).await, "page", "track");
    page(&rig.app, "/api/stop", json!({})).await;
    assert!(session(&rig.app).await.get("motion").is_none());
}

#[tokio::test(start_paused = true)]
async fn legacy_session_loads_exact_fields_and_motion_is_never_persisted() {
    let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target")
        .join(format!("mcp-motion-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let path = folder.join("session.json");
    let saved =
        json!({"site":{"lat_deg":40.0,"lon_deg":-75.0},"min_alt_deg":15.0,"target":"NGC 1976"});
    std::fs::write(&path, saved.to_string()).unwrap();
    let session = Session::with_state_file(&path).unwrap();
    let loaded = app(state().with_session(session), "127.0.0.1");
    let view = support::session(&loaded).await;
    assert_eq!(view["site"], saved["site"]);
    assert_eq!(view["min_alt_deg"], saved["min_alt_deg"]);
    assert_eq!(view["target"]["id"], saved["target"]);
    assert!(view.get("motion").is_none());
    drop(loaded);
    let rig = sim::persistent(&path).await;
    rig.sync().await;
    start(&rig).await;
    source(&support::session(&rig.app).await, "mcp", "goto_target");
    assert!(
        serde_json::from_slice::<Value>(&std::fs::read(&path).unwrap())
            .unwrap()
            .get("motion")
            .is_none()
    );
    complete(&rig, "mcp", "goto_target").await;
    assert!(
        serde_json::from_slice::<Value>(&std::fs::read(&path).unwrap())
            .unwrap()
            .get("motion")
            .is_none()
    );
    drop(rig);
    std::fs::remove_dir_all(folder).unwrap();
}

async fn stopped(rig: &Rig, app: &Router, request: Request<Body>, status: Option<u16>) {
    assert!(
        !stationary(rig).await,
        "positive control: GoTo must be moving"
    );
    let index = rig.commands.lock().unwrap().len();
    let at = rig.now();
    let response = send(app, request).await;
    if let Some(status) = status {
        assert_eq!(response.status(), status);
    }
    let commands = rig.commands.lock().unwrap()[index..].to_vec();
    // Mount::stop_all commands BOTH axes; a single-axis stop cannot satisfy this oracle.
    assert!(
        commands
            .iter()
            .any(|c| c.action == MountCommand::StopAll && c.at - at <= 0.1),
        "{commands:?}"
    );
    assert!(rig.now() - at <= 0.1);
    let view = session(&rig.app).await;
    assert_eq!(view["goto"]["state"], "halted");
    assert_eq!(view["goto"]["reason"], "the observer stopped it");
    assert!(view.get("motion").is_none());
    for _ in 0..50 {
        rig.tick().await;
    }
    assert!(stationary(rig).await);
    let az = rig.mount.state(Axis::Azimuth).await;
    let alt = rig.mount.state(Axis::Altitude).await;
    for _ in 0..50 {
        rig.tick().await;
        assert_eq!(rig.mount.state(Axis::Azimuth).await, az);
        assert_eq!(rig.mount.state(Axis::Altitude).await, alt);
    }
    assert!(
        rig.commands.lock().unwrap()[index..]
            .iter()
            .all(|c| c.action == MountCommand::StopAll),
        "no correction may restart motion"
    );
}

#[tokio::test(start_paused = true)]
async fn second_client_stop_and_sdk_rejections_cannot_block_stop() {
    for case in [
        "second client",
        "unsupported version",
        "bare",
        "foreign origin",
    ] {
        let rig = ready().await;
        start(&rig).await;
        let client = app(rig.state.clone(), "192.168.1.30");
        let mut request = call(
            if case == "unsupported version" {
                "stop"
            } else {
                "stop_all"
            },
            json!({}),
        );
        let status = match case {
            "unsupported version" => {
                request
                    .headers_mut()
                    .insert("mcp-protocol-version", "1999-01-01".parse().unwrap());
                Some(400)
            }
            "bare" => {
                request.headers_mut().remove("accept");
                request.headers_mut().remove("mcp-protocol-version");
                None
            }
            "foreign origin" => {
                request
                    .headers_mut()
                    .insert("origin", "http://elsewhere.example".parse().unwrap());
                Some(403)
            }
            _ => Some(200),
        };
        stopped(&rig, &client, request, status).await;
    }
}

#[tokio::test(start_paused = true)]
async fn security_rejections_cannot_block_stop() {
    for case in [
        "rebinding",
        "missing host",
        "outside peer",
        "no connect info",
        "malformed host",
    ] {
        let rig = ready().await;
        start(&rig).await;
        let client = match case {
            "outside peer" => app(rig.state.clone(), "203.0.113.7"),
            "no connect info" => finder::router(rig.state.clone()),
            _ => rig.app.clone(),
        };
        let mut request = call("stop_all", json!({}));
        match case {
            "rebinding" => {
                request
                    .headers_mut()
                    .insert("host", "evil.example:8080".parse().unwrap());
                request
                    .headers_mut()
                    .insert("origin", "http://evil.example:8080".parse().unwrap());
            }
            "missing host" => {
                request.headers_mut().remove("host");
            }
            "malformed host" => {
                request
                    .headers_mut()
                    .insert("host", "bad host".parse().unwrap());
            }
            _ => {}
        }
        stopped(&rig, &client, request, Some(403)).await;
    }
}

#[tokio::test(start_paused = true)]
async fn batches_stop_once_without_smuggling_a_move() {
    let rig = ready().await;
    for foreign in [true, false] {
        assert!(stationary(&rig).await);
        let index = rig.commands.lock().unwrap().len();
        let at = rig.now();
        let mut request = request(json!([
            {"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"stop_all","arguments":{}}},
            {"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"goto_target","arguments":{}}},
            {"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"stop","arguments":{}}}
        ]));
        if foreign {
            request
                .headers_mut()
                .insert("origin", "http://elsewhere.example".parse().unwrap());
        }
        assert_eq!(
            send(&rig.app, request).await.status(),
            if foreign { 403 } else { 415 }
        );
        let commands = rig.commands.lock().unwrap()[index..].to_vec();
        assert_eq!(commands.len(), 1, "exactly one Stop for the whole request");
        assert_eq!(commands[0].action, MountCommand::StopAll);
        assert!(commands[0].at - at <= 0.1);
        for _ in 0..10 {
            rig.tick().await;
        }
        assert_eq!(rig.commands.lock().unwrap().len(), index + 1);
        assert!(stationary(&rig).await);
        let view = session(&rig.app).await;
        assert!(view.get("goto").is_none());
        assert!(view.get("motion").is_none());
    }
}

#[tokio::test(start_paused = true)]
async fn refused_rebinding_and_remote_reads_and_moves_leave_axes_exactly_unchanged() {
    let rig = ready().await;
    start(&rig).await; // positive control before any negative assertion
    page(&rig.app, "/api/stop", json!({})).await;
    for _ in 0..50 {
        rig.tick().await;
    }
    assert!(stationary(&rig).await);
    let before = [
        rig.mount.state(Axis::Azimuth).await,
        rig.mount.state(Axis::Altitude).await,
    ];
    let index = rig.commands.lock().unwrap().len();
    for (name, peer, hostile) in [
        ("goto_target", "127.0.0.1", true),
        ("get_session", "203.0.113.7", false),
        ("goto_target", "203.0.113.7", false),
    ] {
        let client = app(rig.state.clone(), peer);
        let mut request = call(name, json!({}));
        request.headers_mut().insert(
            "host",
            if hostile {
                "evil.example:8080"
            } else {
                "192.168.1.20:8080"
            }
            .parse()
            .unwrap(),
        );
        if hostile {
            request
                .headers_mut()
                .insert("origin", "http://evil.example:8080".parse().unwrap());
        }
        assert_eq!(send(&client, request).await.status(), 403);
        for _ in 0..10 {
            rig.tick().await;
        }
        assert_eq!(
            [
                rig.mount.state(Axis::Azimuth).await,
                rig.mount.state(Axis::Altitude).await
            ],
            before
        );
        assert_eq!(rig.commands.lock().unwrap().len(), index);
    }
}

#[tokio::test]
async fn stops_are_listed_and_succeed_without_a_mount() {
    let app = app(state(), "127.0.0.1");
    for name in ["stop", "stop_all"] {
        success(&tool(&app, name, json!({})).await, json!({"stopped":true}));
    }
    let response = send(
        &app,
        request(json!({"jsonrpc":"2.0","id":2,"method":"tools/list"})),
    )
    .await;
    let listing = body(response).await;
    for name in [
        "stop",
        "stop_all",
        "describe_axes",
        "get_axis_state",
        "goto_target",
    ] {
        let entry = listing["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["name"] == name)
            .unwrap();
        assert_eq!(entry["outputSchema"], json!({"type":"object"}));
        if ["describe_axes", "get_axis_state"].contains(&name) {
            assert_eq!(entry["annotations"]["readOnlyHint"], true);
        }
        if name == "goto_target" {
            assert!(entry["description"]
                .as_str()
                .unwrap()
                .contains("real telescope"));
        }
    }
}

#[tokio::test(start_paused = true)]
async fn call_log_has_exactly_two_lines_in_order_and_refusals_have_reasons() {
    let rig = ready().await;
    let log = Log::default();
    let client = app(rig.state.clone().with_call_log(log.clone()), "127.0.0.1");
    success(
        &tool(&client, "goto_target", json!({})).await,
        json!({"started":true}),
    );
    success(
        &tool(&client, "stop_all", json!({})).await,
        json!({"stopped":true}),
    );
    assert_eq!(
        log.text(),
        "mcp goto_target {} -> {\"started\":true}\nmcp stop_all {} -> {\"stopped\":true}\n"
    );
    let log = Log::default();
    let client = app(state().with_call_log(log.clone()), "127.0.0.1");
    let result = tool(&client, "goto_target", json!({})).await;
    assert_eq!(result["isError"], true);
    assert_eq!(
        log.text(),
        "mcp goto_target {} -> error: no mount is connected\n"
    );
}

#[tokio::test(start_paused = true)]
async fn no_landed_tool_result_or_call_log_contains_the_node_token() {
    let rig = ready().await;
    let log = Log::default();
    let client = app(rig.state.clone().with_call_log(log.clone()), "127.0.0.1");
    assert_eq!(session(&rig.app).await["node"]["token_set"], true);
    let response = send(
        &client,
        request(json!({"jsonrpc":"2.0","id":7,"method":"tools/list"})),
    )
    .await;
    let listing = body(response).await;
    for entry in listing["result"]["tools"].as_array().unwrap() {
        let name = entry["name"].as_str().unwrap();
        let args = match name {
            "set_target" => json!({"target":"M42"}),
            "set_session" => json!({"min_alt_deg":15}),
            "search_targets" => json!({"q":"orion"}),
            "solve_frame" => json!({"image":"UDUKMSAxCjI1NQoA"}), // valid 1x1 PGM
            "goto_axis_position" => json!({"axis":"azimuth","position":0}),
            "move_axis" => json!({"axis":"altitude","deg_per_s":0.1,"duration_s":0.1}),
            "set_tracking" => json!({"on":false}),
            _ => json!({}),
        };
        let result = tool(&client, name, args).await;
        assert!(
            !result.to_string().contains("test-token"),
            "{name}: {result}"
        );
        // Future moving tools cannot overlap, and GoTo calls stay >=1 s apart.
        page(&rig.app, "/api/stop", json!({})).await;
        for _ in 0..20 {
            rig.tick().await;
        }
    }
    assert!(!log.text().contains("test-token"));
}
