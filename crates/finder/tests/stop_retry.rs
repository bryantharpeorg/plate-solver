//! US1 oracles: gate receipts and recorded StopAlls, simulator axis state, and
//! the specified HTTP/MCP JSON. Timing uses paused tokio time, never wall time.
#[path = "stop/support.rs"]
mod support;

use std::{sync::atomic::Ordering, time::Duration};

use axum::body::to_bytes;
use scope_node::mount::{Axis, Mount};
use serde_json::{json, Value};
use support::*;
use tokio::time::{sleep, timeout, Instant};

const UNANSWERED: &str = r#"{"stopped":true,"acknowledged":false}"#;

async fn moving_rig(mode: Mode) -> Rig {
    let rig = Rig::new().await;
    assert!(rig.gate.log().is_empty());
    assert!(rig.commands.lock().unwrap().is_empty());
    // Bypass console motion: exactly one Stop caller owns each round below.
    rig.mount.rate(Axis::Azimuth, 1.0).await;
    rig.tick().await;
    assert!(rig.mount.state(Axis::Azimuth).await.moving);
    assert!(rig.state.goto().unwrap().progress().is_none());
    assert!(rig.state.session().tracker().is_none());
    rig.gate.set(mode);
    rig
}
fn stops(rig: &Rig) -> Vec<f64> {
    rig.commands
        .lock()
        .unwrap()
        .iter()
        .filter(|c| c.action == MountCommand::StopAll)
        .map(|c| c.at)
        .collect()
}
fn attempts(rig: &Rig, from: f64, count: usize) {
    let log = rig.gate.log();
    assert_eq!(log.len(), count, "{log:?}");
    for (i, at) in log.iter().enumerate() {
        // The contract specifies 200 ms per attempt plus a 50 ms pause.
        assert!(
            (at - from - i as f64 * 0.25).abs() < 0.005,
            "{log:?}, from {from}"
        );
    }
}
async fn rests_by(rig: &Rig, at: f64) {
    while rig.now() <= at {
        if !rig.mount.state(Axis::Azimuth).await.moving
            && !rig.mount.state(Axis::Altitude).await.moving
        {
            return;
        }
        rig.tick().await;
    }
    panic!("axes did not rest by {at}");
}
async fn exact_stop(rig: &Rig) {
    let reply = send(&rig.app, stop()).await;
    assert_eq!(reply.status(), 200);
    assert_eq!(
        to_bytes(reply.into_body(), usize::MAX).await.unwrap(),
        r#"{"stopped":true}"#
    );
}

#[tokio::test(start_paused = true)]
async fn passing_gate_sends_one_immediate_stop_and_axes_rest() {
    let rig = moving_rig(Mode::Pass).await;
    let from = rig.now();
    exact_stop(&rig).await;
    assert!(rig.now() - from <= 0.1);
    attempts(&rig, from, 1);
    assert_eq!(stops(&rig).len(), 1);
    assert!(stops(&rig)[0] - from <= 0.1);
    rests_by(&rig, from + 2.0).await;
    rig.tick().await;
    assert_eq!(rig.gate.log().len(), 1, "OK must end the round");
}

#[tokio::test(start_paused = true)]
async fn two_held_attempts_then_ok_at_half_a_second() {
    let rig = moving_rig(Mode::Hold(2)).await;
    let from = rig.now();
    exact_stop(&rig).await;
    attempts(&rig, from, 3);
    let commands = stops(&rig);
    assert_eq!(commands.len(), 1);
    assert!((commands[0] - from - 0.5).abs() < 0.005);
    rests_by(&rig, commands[0] + 2.0).await;
    rig.tick().await;
    attempts(&rig, from, 3);
}

#[tokio::test(start_paused = true)]
async fn held_round_has_eight_attempts_and_reports_no_ack_at_1_95_seconds() {
    let rig = moving_rig(Mode::HoldAll).await;
    let from = rig.now();
    let reply = send(&rig.app, stop()).await;
    assert_eq!(reply.status(), 200);
    assert_eq!(
        body(reply).await,
        serde_json::from_str::<Value>(UNANSWERED).unwrap()
    );
    assert!((rig.now() - from - 1.95).abs() <= 0.1);
    attempts(&rig, from, 8);
    assert!(stops(&rig).is_empty());
    assert!(
        rig.mount.state(Axis::Azimuth).await.moving,
        "the same running mechanics stop when the gate passes in the previous test"
    );
    rig.tick().await;
    attempts(&rig, from, 8);
}

#[tokio::test(start_paused = true)]
async fn unavailable_is_paced_and_bounded() {
    let rig = moving_rig(Mode::Refuse).await;
    let from = rig.now();
    let reply = send(&rig.app, stop()).await;
    assert_eq!(reply.status(), 200);
    assert_eq!(
        body(reply).await,
        serde_json::from_str::<Value>(UNANSWERED).unwrap()
    );
    assert!(rig.now() - from <= 2.05);
    let log = rig.gate.log();
    assert!(!log.is_empty() && log.len() <= 41, "{log:?}");
    assert!(log[0] - from <= 0.1);
    for pair in log.windows(2) {
        assert!(pair[1] - pair[0] >= 0.045, "{log:?}");
    }
    assert!(stops(&rig).is_empty());
    assert!(rig.mount.state(Axis::Azimuth).await.moving);
}

#[tokio::test(start_paused = true)]
async fn dropping_http_future_does_not_drop_the_stop_round() {
    let rig = moving_rig(Mode::HoldAll).await;
    let app = rig.app.clone();
    let from = rig.now();
    let request = tokio::spawn(async move { send(&app, stop()).await });
    sleep(Duration::from_millis(300)).await;
    assert!(!request.is_finished());
    attempts(&rig, from, 2);
    request.abort();
    assert!(request.await.unwrap_err().is_cancelled());
    let before = rig.gate.log().len();
    rig.gate.set(Mode::Pass);
    let released = rig.now();
    while stops(&rig).is_empty() && rig.now() - released < 0.5 {
        rig.tick().await;
    }
    assert!(rig.gate.log().len() > before);
    assert!(rig.gate.log()[before] - released <= 0.5);
    assert_eq!(stops(&rig).len(), 1);
    rests_by(&rig, rig.now() + 2.0).await;
}

#[tokio::test(start_paused = true)]
async fn silent_heartbeats_report_unacknowledged_within_the_window() {
    let rig = moving_rig(Mode::HoldAll).await;
    let link = rig.state.link().unwrap();
    assert!(link.status().borrow().connected);
    rig.controls.set_unresponsive(true);
    for _ in 0..20 {
        if !link.status().borrow().connected {
            break;
        }
        rig.tick().await;
    }
    assert!(!link.status().borrow().connected);
    let from = rig.now();
    let reply = send(&rig.app, stop()).await;
    assert_eq!(reply.status(), 200);
    assert_eq!(
        body(reply).await,
        serde_json::from_str::<Value>(UNANSWERED).unwrap()
    );
    assert!(rig.now() - from <= 2.05);
}

#[tokio::test(start_paused = true)]
async fn no_scope_link_keeps_the_exact_reply_without_advancing_time() {
    let app = app(state());
    let from = Instant::now();
    let reply = send(&app, stop()).await;
    assert_eq!(reply.status(), 200);
    assert_eq!(
        to_bytes(reply.into_body(), usize::MAX).await.unwrap(),
        r#"{"stopped":true}"#
    );
    assert_eq!(Instant::now(), from);
}

#[tokio::test(start_paused = true)]
async fn retained_tracker_cannot_delay_goto_cancellation_or_first_attempt() {
    let rig = Rig::new().await;
    rig.sync().await;
    let reply = send(&rig.app, post("/api/track", json!({"on":true}))).await;
    assert_eq!(reply.status(), 200, "{}", body(reply).await);
    for _ in 0..5000 {
        if session(&rig.app).await["track"]["state"] == "tracking" {
            break;
        }
        rig.tick().await;
    }
    assert_eq!(session(&rig.app).await["track"]["state"], "tracking");
    let tracker = rig.state.session().tracker().unwrap();
    exact_stop(&rig).await;
    assert_eq!(tracker.state().state, "off");
    assert!(std::sync::Arc::ptr_eq(
        &tracker,
        &rig.state.session().tracker().unwrap()
    ));
    rests_by(&rig, rig.now() + 2.0).await;
    // M45 is a different pointable target on this winter night.
    rig.state
        .session()
        .edit(
            serde_json::from_value(json!({"target":"M45"})).unwrap(),
            rig.now(),
        )
        .unwrap();
    let reply = send(&rig.app, post("/api/goto", Value::Null)).await;
    assert_eq!(reply.status(), 200, "{}", body(reply).await);
    for _ in 0..20 {
        if rig.mount.state(Axis::Azimuth).await.moving {
            break;
        }
        rig.tick().await;
    }
    assert!(rig.mount.state(Axis::Azimuth).await.moving);
    assert_eq!(session(&rig.app).await["goto"]["state"], "slewing");
    rig.gate.set(Mode::HoldAll);
    let before = rig.gate.log().len();
    let from = rig.now();
    let app = rig.app.clone();
    let request = tokio::spawn(async move { send(&app, stop()).await });
    sleep(Duration::from_millis(100)).await;
    let view = session(&rig.app).await;
    assert!(rig.now() - from <= 0.1 + 1e-5);
    assert_eq!(view["goto"]["state"], "halted", "{view}");
    assert_eq!(view["goto"]["reason"], "the observer stopped it");
    let log = rig.gate.log();
    assert!(log.len() > before);
    assert!(log[before] - from < 0.1);
    assert!(!request.is_finished(), "the HTTP reply waits for the round");
    assert_eq!(request.await.unwrap().status(), 200);
}

#[tokio::test(start_paused = true)]
async fn mcp_stops_reuse_the_doors_real_reply_and_explain_unacknowledged() {
    let rig = moving_rig(Mode::HoldAll).await;
    let expected: Value = serde_json::from_str(UNANSWERED).unwrap();
    for name in ["stop_all", "stop"] {
        let before = rig.gate.log().len();
        let reply = send(
            &rig.app,
            rpc(json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
            "params":{"name":name,"arguments":{}}})),
        )
        .await;
        assert_eq!(reply.status(), 200);
        let reply = body(reply).await;
        let result = &reply["result"];
        assert_ne!(result["isError"], true, "{reply}");
        assert_eq!(result["structuredContent"], expected);
        assert_eq!(
            serde_json::from_str::<Value>(result["content"][0]["text"].as_str().unwrap()).unwrap(),
            expected
        );
        assert_eq!(
            rig.gate.log().len() - before,
            8,
            "door and tool must share a round"
        );
        let log = rig.call_log.text();
        let prefix = format!("mcp {name} {{}} -> ");
        let lines: Vec<_> = log
            .lines()
            .filter_map(|line| line.strip_prefix(&prefix))
            .collect();
        assert_eq!(lines.len(), 1, "{log}");
        assert_eq!(serde_json::from_str::<Value>(lines[0]).unwrap(), expected);
    }
    let listed = body(
        send(
            &rig.app,
            rpc(json!({"jsonrpc":"2.0","id":2,"method":"tools/list"})),
        )
        .await,
    )
    .await;
    let tools = listed["result"]["tools"].as_array().unwrap();
    for name in ["stop", "stop_all"] {
        let description = tools.iter().find(|tool| tool["name"] == name).unwrap()["description"]
            .as_str()
            .unwrap();
        assert!(description.contains("acknowledged: false"), "{description}");
        assert!(description.contains("may still be moving"), "{description}");
    }
}

#[tokio::test(start_paused = true)]
async fn foreign_origin_batch_stops_once_before_denial() {
    let rig = moving_rig(Mode::HoldAll).await;
    let mut request = rpc(json!([
        {"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"stop","arguments":{}}},
        {"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"stop_all","arguments":{}}}
    ]));
    request
        .headers_mut()
        .insert("origin", "http://foreign.example".parse().unwrap());
    let from = rig.now();
    let reply = send(&rig.app, request).await;
    assert_eq!(reply.status(), 403);
    attempts(&rig, from, 8);
    assert!((rig.now() - from - 1.95).abs() <= 0.1);
    assert!(stops(&rig).is_empty());
    assert!(rig.mount.state(Axis::Azimuth).await.moving);
}

#[tokio::test(start_paused = true)]
async fn stop_answers_while_goto_holds_the_motion_lock_waiting_on_state() {
    let rig = Rig::new().await;
    rig.sync().await;
    rig.controls.stop_frames();
    for _ in 0..5 {
        rig.tick().await;
    }
    let before = rig.held_states.load(Ordering::SeqCst);
    rig.hold_state.store(true, Ordering::SeqCst);
    let app = rig.app.clone();
    let goto = tokio::spawn(async move { send(&app, post("/api/goto", Value::Null)).await });
    for _ in 0..20 {
        if rig.held_states.load(Ordering::SeqCst) > before {
            break;
        }
        sleep(Duration::from_millis(1)).await;
    }
    assert!(
        rig.held_states.load(Ordering::SeqCst) > before,
        "mount state call must be held"
    );
    assert!(!goto.is_finished());
    // A session read also waits on the motion lock: prove this is the blocked
    // route, not merely some unrelated mount telemetry task.
    let app = rig.app.clone();
    let read = tokio::spawn(async move { session(&app).await });
    sleep(Duration::from_millis(1)).await;
    assert!(!read.is_finished());
    let from = Instant::now();
    let reply = timeout(Duration::from_millis(100), send(&rig.app, stop()))
        .await
        .unwrap();
    assert_eq!(reply.status(), 200);
    assert_eq!(body(reply).await, json!({"stopped":true}));
    assert!(from.elapsed() <= Duration::from_millis(100));
    assert!(!goto.is_finished(), "Stop must answer before the held GoTo");
    assert_eq!(rig.gate.log().len(), 1);
    rig.hold_state.store(false, Ordering::SeqCst);
    let cancelled = goto.await.unwrap();
    assert_eq!(cancelled.status(), 409);
    assert_eq!(
        body(cancelled).await,
        json!({"error":"the observer stopped it"})
    );
    read.await.unwrap();
    exact_stop(&rig).await;
}
