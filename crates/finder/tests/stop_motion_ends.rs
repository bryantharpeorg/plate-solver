//! US2 oracles: specified session/HTTP/MCP JSON, the gate's attempt receipts,
//! recorded mount commands and simulator mechanics. All clocks are virtual;
//! attempt offsets allow 5 ms, and bounded state polling samples every 10 ms.
#[allow(dead_code)]
#[path = "stop/support.rs"]
mod support;

use std::{sync::atomic::Ordering, sync::Arc, time::Duration};

use axum::body::to_bytes;
use finder::{model::Model, session::Site, tracking::Tracker};
use scope_node::mount::{Axis, Mount};
use serde_json::{json, Value};
use support::*;
use tokio::time::{sleep, timeout, Instant};

const SAMPLE: Duration = Duration::from_millis(10);

async fn ready() -> Rig {
    let rig = Rig::new().await;
    rig.sync().await;
    for axis in [Axis::Azimuth, Axis::Altitude] {
        assert!(!rig.mount.state(axis).await.moving);
    }
    rig
}

async fn attempts_by(rig: &Rig, count: usize, limit: Duration) {
    timeout(limit, async {
        while rig.gate.log().len() < count {
            sleep(SAMPLE).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("missing attempt {count}: {:?}", rig.gate.log()));
}

fn stops(rig: &Rig) -> Vec<f64> {
    rig.commands
        .lock()
        .unwrap()
        .iter()
        .filter(|command| command.action == MountCommand::StopAll)
        .map(|command| command.at)
        .collect()
}

async fn at_rest(rig: &Rig) {
    timeout(Duration::from_secs(1), async {
        while rig.mount.state(Axis::Azimuth).await.moving
            || rig.mount.state(Axis::Altitude).await.moving
        {
            sleep(SAMPLE).await;
        }
    })
    .await
    .expect("both simulated axes must come to rest");
}

async fn slewing() -> Rig {
    let rig = ready().await;
    let reply = send(&rig.app, post("/api/goto", Value::Null)).await;
    assert_eq!(reply.status(), 200, "{}", body(reply).await);
    timeout(Duration::from_secs(2), async {
        while !rig.mount.state(Axis::Azimuth).await.moving {
            sleep(SAMPLE).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(session(&rig.app).await["goto"]["state"], "slewing");
    assert!(rig.gate.log().is_empty());
    rig
}

async fn move_axis(rig: &Rig, duration_s: f64) -> Value {
    let reply = send(
        &rig.app,
        rpc(json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
        "params":{"name":"move_axis","arguments":{
            "axis":"altitude","deg_per_s":0.5,"duration_s":duration_s
        }}})),
    )
    .await;
    assert_eq!(reply.status(), 200);
    let reply = body(reply).await;
    assert!(reply.get("error").is_none(), "{reply}");
    reply["result"].clone()
}

async fn moving_axis(rig: &Rig, duration_s: f64) {
    let reply = move_axis(rig, duration_s).await;
    assert_ne!(reply["isError"], true, "{reply}");
    assert_eq!(reply["structuredContent"], json!({"started":true}));
    sleep(Duration::from_millis(100)).await;
    assert!(rig.mount.state(Axis::Altitude).await.moving);
    assert_eq!(session(&rig.app).await["axis_move"]["state"], "moving");
}

async fn axis_ending(rig: &Rig) -> Value {
    timeout(Duration::from_secs(4), async {
        loop {
            let progress = session(&rig.app).await["axis_move"].clone();
            assert_ne!(
                progress["state"], "done",
                "must never report success: {progress}"
            );
            if progress["state"] == "halted" {
                return progress;
            }
            assert_eq!(progress["state"], "moving");
            sleep(SAMPLE).await;
        }
    })
    .await
    .expect("axis move must publish its ending after the bounded stop round")
}

async fn release_and_rest(rig: &Rig) {
    rig.gate.set(Mode::Pass);
    let reply = send(&rig.app, stop()).await;
    assert_eq!(reply.status(), 200);
    assert_eq!(body(reply).await, json!({"stopped":true}));
    at_rest(rig).await;
}

#[tokio::test(start_paused = true)]
async fn lost_link_publishes_goto_halt_before_retry_and_third_attempt_reaches_node() {
    let rig = slewing().await;
    rig.gate.set(Mode::Hold(2));
    rig.controls.set_unresponsive(true);
    attempts_by(&rig, 1, Duration::from_secs(2)).await;
    let view = session(&rig.app).await;
    assert!(
        rig.gate.log().len() < 3,
        "halt must precede the third attempt"
    );
    assert_eq!(view["goto"]["state"], "halted", "{view}");
    assert!(view["goto"]["reason"]
        .as_str()
        .unwrap()
        .contains("lost link"));

    attempts_by(&rig, 3, Duration::from_secs(1)).await;
    let third = rig.gate.log()[2];
    // The watchdog may also stop this mount. Require a recorded stop at the
    // third gate receipt, proving the retained client reached the node too.
    timeout(Duration::from_millis(50), async {
        while !stops(&rig)
            .iter()
            .any(|at| *at >= third && *at - third < 0.05)
        {
            sleep(SAMPLE).await;
        }
    })
    .await
    .expect("the third attempt must pass through the gate to the node");
    at_rest(&rig).await;
}

#[tokio::test(start_paused = true)]
async fn lost_link_goto_remains_busy_while_its_stop_round_runs() {
    let rig = slewing().await;
    rig.gate.set(Mode::HoldAll);
    rig.controls.set_unresponsive(true);
    attempts_by(&rig, 1, Duration::from_secs(2)).await;
    sleep(Duration::from_millis(300)).await;
    let view = session(&rig.app).await;
    assert_eq!(view["goto"]["state"], "halted", "{view}");
    assert!(view["goto"]["reason"]
        .as_str()
        .unwrap()
        .contains("lost link"));
    let reply = send(&rig.app, post("/api/goto", Value::Null)).await;
    assert_eq!(reply.status(), 409);
    assert_eq!(
        body(reply).await,
        json!({"error":"a GoTo is already running"})
    );
    assert!(rig.now() - rig.gate.log()[0] < 1.95);
    attempts_by(&rig, 3, Duration::from_secs(1)).await;
}

#[tokio::test(start_paused = true)]
async fn rate_move_stays_moving_until_third_final_stop_then_reports_done() {
    let rig = ready().await;
    rig.gate.set(Mode::Hold(2));
    let started = rig.now();
    moving_axis(&rig, 1.0).await;
    timeout(Duration::from_secs(2), async {
        loop {
            let progress = session(&rig.app).await["axis_move"].clone();
            // Reading the session can yield to the stop task. Check receipts
            // after that read so a third attempt during it permits completion.
            if rig.gate.log().len() >= 3 {
                break;
            }
            assert_eq!(progress["state"], "moving");
            sleep(SAMPLE).await;
        }
    })
    .await
    .expect("the final stop must retry while progress remains moving");
    timeout(Duration::from_millis(100), async {
        while session(&rig.app).await["axis_move"]["state"] == "moving" {
            sleep(SAMPLE).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        session(&rig.app).await["axis_move"],
        json!({"tool":"move_axis","state":"done"})
    );
    assert!((rig.gate.log()[0] - started - 1.0).abs() <= 0.05);
    assert_eq!(stops(&rig).len(), 1);
    at_rest(&rig).await;
}

#[tokio::test(start_paused = true)]
async fn unanswered_rate_move_never_reports_done_and_gate_keeps_axis_running() {
    let rig = ready().await;
    rig.gate.set(Mode::HoldAll);
    moving_axis(&rig, 1.0).await;
    let ending = axis_ending(&rig).await;
    assert_eq!(
        ending,
        json!({"tool":"move_axis","state":"halted",
        "reason":"the mount did not confirm the stop"})
    );
    assert!(
        rig.now() - rig.gate.log()[0] >= 1.9,
        "wait for the whole round"
    );
    assert!(stops(&rig).is_empty());
    assert!(rig.mount.state(Axis::Altitude).await.moving);
    // The same rig stops when the gate passes, as in the successful move above.
    release_and_rest(&rig).await;
    assert!(!stops(&rig).is_empty());
}

#[tokio::test(start_paused = true)]
async fn observer_stop_keeps_its_reason_when_the_rate_moves_stop_is_unanswered() {
    let rig = ready().await;
    rig.gate.set(Mode::HoldAll);
    moving_axis(&rig, 10.0).await;
    let app = rig.app.clone();
    let request = tokio::spawn(async move { send(&app, stop()).await });
    let ending = axis_ending(&rig).await;
    assert_eq!(
        ending,
        json!({"tool":"move_axis","state":"halted",
        "reason":"the observer stopped it"})
    );
    assert_eq!(
        body(request.await.unwrap()).await,
        json!({"stopped":true,"acknowledged":false})
    );
    release_and_rest(&rig).await;
}

#[tokio::test(start_paused = true)]
async fn unanswered_rate_start_retries_its_stop_before_answering_the_tool() {
    let rig = ready().await;
    rig.hold_rate.store(true, Ordering::SeqCst);
    rig.gate.set(Mode::Hold(1));
    let reply = timeout(Duration::from_secs(2), move_axis(&rig, 1.0))
        .await
        .unwrap();
    assert_eq!(reply["isError"], true, "{reply}");
    assert_eq!(reply["content"][0]["text"], "mount did not answer");
    assert!(rig.commands.lock().unwrap().iter().any(|command| matches!(
        command.action,
        MountCommand::Rate {
            axis: Axis::Altitude,
            ..
        }
    )));
    assert_eq!(rig.gate.log().len(), 2);
    assert_eq!(stops(&rig).len(), 1);
}

async fn tracking() -> Rig {
    let rig = ready().await;
    let reply = send(&rig.app, post("/api/track", json!({"on":true}))).await;
    assert_eq!(reply.status(), 200, "{}", body(reply).await);
    for _ in 0..5000 {
        if session(&rig.app).await["track"]["state"] == "tracking" {
            break;
        }
        rig.tick().await;
    }
    assert_eq!(session(&rig.app).await["track"]["state"], "tracking");
    rig.tick().await;
    assert!(rig.mount.state(Axis::Azimuth).await.moving);
    rig
}

#[tokio::test(start_paused = true)]
async fn track_stop_starts_promptly_and_retries_to_the_exact_acknowledged_body() {
    let rig = tracking().await;
    let before = rig.gate.log().len();
    rig.gate.set(Mode::Hold(2));
    let from = rig.now();
    let reply = send(&rig.app, stop()).await;
    assert_eq!(reply.status(), 200);
    assert_eq!(
        to_bytes(reply.into_body(), usize::MAX).await.unwrap(),
        r#"{"stopped":true}"#
    );
    let log = rig.gate.log();
    assert!(log[before] - from <= 0.1);
    assert!(log.len() - before >= 3, "{log:?}");
    at_rest(&rig).await;
    let view = session(&rig.app).await;
    assert_eq!(view["track"]["state"], "off");
    assert_eq!(view["track"]["ended_reason"], "stopped");
}

#[tokio::test(start_paused = true)]
async fn track_stop_shares_one_deadline_across_both_tracker_calls() {
    let rig = tracking().await;
    rig.gate.set(Mode::HoldAll);
    let from = Instant::now();
    let reply = timeout(Duration::from_millis(2500), send(&rig.app, stop()))
        .await
        .expect("the second tracker stop cannot start another two-second window");
    assert!(from.elapsed() <= Duration::from_millis(2500));
    assert_eq!(reply.status(), 200);
    assert_eq!(
        body(reply).await,
        json!({"stopped":true,"acknowledged":false})
    );
    assert_eq!(session(&rig.app).await["track"]["ended_reason"], "stopped");
    release_and_rest(&rig).await;
}

#[tokio::test(start_paused = true)]
async fn never_started_tracker_retries_at_zero_quarter_and_half_a_second() {
    let rig = Rig::new().await;
    let tracker = Tracker::new(
        Arc::new(ConsoleClock(rig.sim.clock().clone())),
        rig.state.link().unwrap(),
        Model::default(),
        Site {
            lat_deg: 40.0,
            lon_deg: -75.0,
        },
        15.0,
    );
    assert_eq!(tracker.state().state, "off");
    assert!(tracker.state().target.is_none());
    // Only this tracker owns a stop round; no console motion is running.
    rig.mount.rate(Axis::Azimuth, 1.0).await;
    rig.tick().await;
    assert!(rig.mount.state(Axis::Azimuth).await.moving);
    assert!(rig.gate.log().is_empty());
    rig.gate.set(Mode::Hold(2));
    let from = rig.now();
    tracker.stop("stopped").await;
    let log = rig.gate.log();
    assert_eq!(log.len(), 3, "{log:?}");
    for (at, offset) in log.iter().zip([0.0, 0.25, 0.5]) {
        assert!((at - from - offset).abs() < 0.005, "{log:?}, from {from}");
    }
    assert_eq!(stops(&rig).len(), 1);
    at_rest(&rig).await;
}
