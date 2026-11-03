//! US6 oracles: the simulator's true eyepiece (60 arcsec), recorded mount
//! commands (0.1 s), and route JSON/refusals (exact). The unmodified support
//! rig solves rendered centroids every 0.5 s; only the live loop feeds Track.
#[path = "mcp/support.rs"]
mod support;

use axum::{body::Body, http::Request, Router};
use finder::{model::separation, targets};
use scope_node::mount::{Axis, Mount};
use serde_json::{json, Value};
use std::{
    io::Write,
    sync::{Arc, Mutex},
    time::Instant,
};
use support::{
    sim::{MountCommand, Rig, START},
    *,
};

const LIMITED: &str = "the rate limit of 2 moving calls a second was reached";
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

async fn tool(app: &Router, name: &str, args: Value) -> Value {
    let response = send(app, call(name, args)).await;
    assert_eq!(response.status(), 200);
    let rpc = body(response).await;
    assert!(rpc.get("error").is_none(), "{rpc}");
    let result = rpc["result"].clone();
    assert!(!result.to_string().contains("test-token"));
    result
}
fn success(result: &Value) -> &Value {
    assert_ne!(result["isError"], true, "{result}");
    let value = &result["structuredContent"];
    assert!(value.is_object(), "{result}");
    assert_eq!(
        serde_json::from_str::<Value>(result["content"][0]["text"].as_str().unwrap()).unwrap(),
        *value
    );
    value
}
fn refused(result: Value, reason: &str) {
    assert_eq!(result["isError"], true, "{result}");
    assert_eq!(result["content"], json!([{"type":"text","text":reason}]));
}
async fn tracking(rig: &Rig, on: bool) -> Value {
    let result = tool(&rig.app, "set_tracking", json!({"on":on})).await;
    success(&result).clone()
}
async fn ticks(rig: &Rig, count: usize) {
    for _ in 0..count {
        rig.tick().await;
    }
}
async fn rig(connected: bool) -> (Rig, Log) {
    let mut rig = Rig::new(START, 50.0, 120.0, connected).await;
    let log = Log::default();
    rig.state = rig.state.with_call_log(log.clone());
    rig.app = app(rig.state.clone(), "127.0.0.1");
    if connected {
        rig.sync().await;
        assert_eq!(session(&rig.app).await["mount"]["connected"], true);
    }
    (rig, log)
}
fn source(view: &Value, since: &Value) {
    assert_eq!(view["motion"]["by"], "mcp", "{view}");
    assert_eq!(view["motion"]["tool"], "set_tracking");
    assert_eq!(&view["motion"]["since"], since);
}
fn error_arcsec(rig: &Rig) -> f64 {
    let target = targets::find("M42").unwrap();
    let (alt, az) =
        simulator::sky::horizontal(target.vector(), simulator::Site::default(), rig.now());
    let truth = rig.mount.true_direction();
    separation(az, alt, truth.az, truth.alt) * 3600.0
}
async fn wait_tracking(rig: &Rig, since: &Value) {
    for _ in 0..5000 {
        let view = session(&rig.app).await;
        source(&view, since);
        if view["track"]["state"] == "tracking" {
            return;
        }
        assert_eq!(view["track"]["state"], "armed", "{view}");
        assert!(
            matches!(
                view["goto"]["state"].as_str(),
                Some("slewing" | "solving" | "correcting" | "done")
            ),
            "{view}"
        );
        rig.tick().await;
    }
    panic!("Track never started");
}
fn stop_within(rig: &Rig, from: f64) {
    let commands = rig.commands.lock().unwrap();
    let stop = commands
        .iter()
        .find(|c| c.at >= from && c.action == MountCommand::StopAll)
        .expect("both axes must be commanded to stop");
    assert!(stop.at - from <= 0.1, "{stop:?}, requested at {from}");
}
async fn quiet(rig: &Rig) {
    let first = rig.commands.lock().unwrap().len();
    ticks(rig, 50).await;
    assert!(rig.commands.lock().unwrap()[first..]
        .iter()
        .all(|c| !matches!(
            c.action,
            MountCommand::Rate { .. } | MountCommand::GoTo { .. }
        )));
    for axis in [Axis::Azimuth, Axis::Altitude] {
        assert!(!rig.mount.state(axis).await.moving);
    }
}
async fn move_axis(rig: &Rig, duration: f64) -> f64 {
    let result = tool(
        &rig.app,
        "move_axis",
        json!({
            "axis":"altitude", "deg_per_s":0.5, "duration_s":duration
        }),
    )
    .await;
    assert_eq!(success(&result), &json!({"started":true}));
    let commands = rig.commands.lock().unwrap();
    commands
        .iter()
        .rev()
        .find(|c| {
            matches!(
                c.action,
                MountCommand::Rate {
                    axis: Axis::Altitude,
                    deg_per_s: 0.5
                }
            )
        })
        .expect("accepted direct move must command the connected mount")
        .at
}

#[tokio::test(start_paused = true)]
async fn track_first_goto_keeps_mcp_source_and_holds_m42_for_ten_minutes() {
    let wall = Instant::now();
    let (rig, log) = rig(true).await;
    // This sync is away from M42, so an immediate Track would not prove GoTo.
    assert!(error_arcsec(&rig) > 60.0);
    let called = rig.now();
    let initial = tracking(&rig, true).await;
    assert_eq!(initial["track"]["state"], "armed");
    assert!(matches!(
        initial["goto"]["state"].as_str(),
        Some("slewing" | "solving" | "correcting")
    ));
    let since = initial["motion"]["since"].clone();
    assert!(since.as_f64().unwrap() >= called);
    source(&initial, &since);
    wait_tracking(&rig, &since).await;
    let started = rig.now();
    // Include t=0, then every second through t=600, timed from tracking.
    for second in 0..=600 {
        assert!((rig.now() - started - f64::from(second)).abs() <= 1e-5);
        let error = error_arcsec(&rig);
        assert!(error <= 60.0, "t={second}s: M42 error {error} arcsec");
        let view = session(&rig.app).await;
        assert_eq!(view["track"]["state"], "tracking", "{view}");
        source(&view, &since);
        if second < 600 {
            ticks(&rig, 10).await;
        }
    }
    // Nonempty, recent live errors prove the ordinary live loop fed Track.
    let tracker = rig.state.session().tracker().unwrap();
    let samples = tracker.state().samples;
    assert!(samples.len() > 30);
    assert!(samples.last().unwrap().time >= rig.now() - 1.0);
    let at = rig.now();
    let mcp = tool(&rig.app, "get_session", json!({})).await;
    let route = session(&rig.app).await;
    assert_eq!(rig.now(), at, "compare at one virtual instant");
    let rms = &success(&mcp)["track"]["readout"]["rms_arcsec"];
    assert!(rms.is_number());
    assert_eq!(*rms, route["track"]["readout"]["rms_arcsec"]);
    tracking(&rig, false).await;
    assert!(!log.text().contains("test-token"));
    eprintln!(
        "ten-minute live-loop hold took {:?} wall time",
        wall.elapsed()
    );
}

#[tokio::test(start_paused = true)]
async fn no_mount_matches_track_route_and_never_exposes_token() {
    // Positive control uses the same tool, headers and a connected simulator.
    let (connected, _) = rig(true).await;
    assert_eq!(tracking(&connected, true).await["track"]["wanted"], true);
    tracking(&connected, false).await;
    let (rig, log) = rig(false).await;
    let response = send(
        &rig.app,
        Request::post("/api/track")
            .header("host", "localhost:8080")
            .header("content-type", "application/json")
            .body(Body::from(json!({"on":true}).to_string()))
            .unwrap(),
    )
    .await;
    assert_eq!(response.status(), 409);
    let route = body(response).await;
    let reason = route["error"].as_str().unwrap();
    assert_eq!(reason, "no mount is connected");
    refused(
        tool(&rig.app, "set_tracking", json!({"on":true})).await,
        reason,
    );
    assert_eq!(
        log.text(),
        format!("mcp set_tracking {{\"on\":true}} -> error: {reason}\n")
    );
    assert!(!log.text().contains("test-token"));
    assert!(rig.commands.lock().unwrap().is_empty());
    assert_ne!(session(&rig.app).await["track"]["wanted"], true);
    // Off succeeds even without a mount and returns the route's session shape.
    let off = tracking(&rig, false).await;
    assert_eq!(off["track"], session(&rig.app).await["track"]);
}

#[tokio::test(start_paused = true)]
async fn tracking_shares_move_limit_but_off_stops_within_the_same_second() {
    let (rig, log) = rig(true).await;
    let start = move_axis(&rig, 10.0).await;
    refused(
        tool(&rig.app, "set_tracking", json!({"on":true})).await,
        DIRECT,
    );
    let before = rig.commands.lock().unwrap().clone();
    refused(
        tool(&rig.app, "set_tracking", json!({"on":true})).await,
        LIMITED,
    );
    assert_eq!(
        *rig.commands.lock().unwrap(),
        before,
        "limited call moves nothing"
    );
    assert_eq!(session(&rig.app).await["axis_move"]["state"], "moving");
    let stopped = rig.now();
    tracking(&rig, false).await;
    assert!(rig.now() - start < 1.0, "all four calls are in one second");
    stop_within(&rig, stopped);
    ticks(&rig, 1).await;
    assert_eq!(session(&rig.app).await["axis_move"]["state"], "halted");
    quiet(&rig).await;
    let text = log.text();
    let lines: Vec<_> = text.lines().collect();
    assert_eq!(lines.len(), 4);
    assert_eq!(
        lines[1],
        format!("mcp set_tracking {{\"on\":true}} -> error: {DIRECT}")
    );
    assert_eq!(
        lines[2],
        format!("mcp set_tracking {{\"on\":true}} -> error: {LIMITED}")
    );
    let result = lines[3]
        .strip_prefix("mcp set_tracking {\"on\":false} -> ")
        .unwrap();
    assert!(serde_json::from_str::<Value>(result).unwrap().is_object());
    assert!(!text.contains("test-token"));
}

#[tokio::test(start_paused = true)]
async fn mcp_stop_ends_running_track_and_no_rate_restarts_it() {
    let (rig, _) = rig(true).await;
    let view = tracking(&rig, true).await;
    wait_tracking(&rig, &view["motion"]["since"]).await;
    ticks(&rig, 10).await;
    assert!(
        rig.commands.lock().unwrap().iter().any(|c| matches!(
            c.action, MountCommand::Rate { deg_per_s, .. } if deg_per_s != 0.0
        )),
        "Track must command rates before Stop"
    );
    assert_eq!(session(&rig.app).await["track"]["state"], "tracking");
    let stopped = rig.now();
    assert_eq!(
        success(&tool(&rig.app, "stop", json!({})).await),
        &json!({"stopped":true})
    );
    stop_within(&rig, stopped);
    let view = session(&rig.app).await;
    assert_eq!(view["track"]["state"], "off");
    assert_eq!(view["track"]["wanted"], false);
    assert_eq!(view["track"]["ended_reason"], "stopped");
    assert!(view.get("motion").is_none());
    quiet(&rig).await;
}

#[tokio::test(start_paused = true)]
async fn direct_move_refuses_track_even_when_centred_and_still_stops_on_time() {
    let (rig, _) = rig(true).await;
    let target = targets::find("M42").unwrap();
    rig.sim.point_eyepiece_at(target.ra_deg, target.dec_deg);
    ticks(&rig, 20).await;
    assert!(
        error_arcsec(&rig) < 60.0,
        "Track could otherwise start without a GoTo"
    );
    let started = move_axis(&rig, 2.0).await;
    let before = rig.commands.lock().unwrap().clone();
    refused(
        tool(&rig.app, "set_tracking", json!({"on":true})).await,
        DIRECT,
    );
    assert_eq!(*rig.commands.lock().unwrap(), before);
    let view = session(&rig.app).await;
    assert_ne!(view["track"]["wanted"], true);
    assert_eq!(view["axis_move"]["state"], "moving");
    assert_eq!(view["motion"]["tool"], "move_axis");
    while rig.now() - started < 2.2 {
        rig.tick().await;
    }
    stop_within(&rig, started + 2.0);
    let view = session(&rig.app).await;
    assert_eq!(view["axis_move"]["state"], "done", "{view}");
    assert_ne!(view["track"]["wanted"], true);
    quiet(&rig).await;
}

#[tokio::test]
async fn tracking_tool_schema_and_off_result_match_the_page_session() {
    let app = app(state(), "127.0.0.1");
    let rpc = body(
        send(
            &app,
            request(json!({"jsonrpc":"2.0","id":1,"method":"tools/list"})),
        )
        .await,
    )
    .await;
    let descriptor = rpc["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "set_tracking")
        .expect("set_tracking listed");
    assert!(descriptor["description"]
        .as_str()
        .unwrap()
        .contains("real telescope"));
    assert_eq!(
        descriptor["inputSchema"]["properties"]["on"]["type"],
        "boolean"
    );
    assert_eq!(descriptor["inputSchema"]["required"], json!(["on"]));
    assert_eq!(descriptor["outputSchema"], json!({"type":"object"}));
    let off = tool(&app, "set_tracking", json!({"on":false})).await;
    assert_eq!(*success(&off), session(&app).await);
}
