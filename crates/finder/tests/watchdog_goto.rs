//! Oracles: recorded commands, simulated axes and true eyepiece direction,
//! and the specified session states/reason. Polling is bounded at 10 ms;
//! recovery must finish within 300 s and strictly less than 60 arcseconds.
#[allow(dead_code)]
#[path = "stop/support.rs"]
mod support;

use std::{future::Future, sync::atomic::Ordering, time::Duration};

use scope_link::proto;
use scope_node::mount::{Axis, Mount};
use serde_json::{json, Value};
use support::*;
use tokio::time::{sleep, timeout};

const WATCHDOG_REASON: &str = "halted by the scope node's watchdog";
const SAMPLE: Duration = Duration::from_millis(10);

async fn connected_during<F: Future>(rig: &Rig, work: F) -> F::Output {
    let mut status = rig.state.link().unwrap().status();
    assert!(status.borrow_and_update().connected);
    tokio::pin!(work);
    loop {
        tokio::select! {
            biased;
            changed = status.changed() => {
                changed.expect("scope link status must remain available");
                assert!(status.borrow_and_update().connected, "link disconnected");
            }
            result = &mut work => return result,
        }
    }
}

async fn ready() -> Rig {
    let rig = Rig::new().await;
    rig.sync().await;
    rig
}

async fn start(rig: &Rig, path: &str, value: Value) {
    let reply = send(&rig.app, post(path, value)).await;
    assert_eq!(reply.status(), 200, "{}", body(reply).await);
    timeout(Duration::from_secs(2), async {
        while !rig.mount.state(Axis::Azimuth).await.moving
            || !rig.mount.state(Axis::Altitude).await.moving
        {
            sleep(SAMPLE).await;
        }
    })
    .await
    .expect("accepted GoTo must actually slew");
    // Let both initial GotoAxis replies return before firing mid-slew.
    sleep(SAMPLE).await;
    assert_eq!(session(&rig.app).await["goto"]["state"], "slewing");
    let commands = rig.commands.lock().unwrap();
    assert_eq!(
        commands
            .iter()
            .filter(|c| matches!(c.action, MountCommand::GoTo { .. }))
            .count(),
        2
    );
    assert!(!commands.iter().any(|c| c.action == MountCommand::StopAll));
}

async fn fire_during_slew(rig: &Rig, track: bool) {
    // The console clock and its healthy 250 ms heartbeats keep running normally.
    // Only the node's injected time jumps beyond its one-second watchdog limit.
    let console_now = rig.now();
    rig.node_offset_ms.fetch_add(1200, Ordering::SeqCst);
    assert_eq!(rig.now(), console_now);
    timeout(Duration::from_millis(500), async {
        loop {
            let view = session(&rig.app).await;
            if view["goto"]["state"] == "halted" && (!track || view["track"]["state"] == "off") {
                assert_eq!(view["goto"]["reason"], WATCHDOG_REASON);
                if track {
                    assert_eq!(view["track"]["wanted"], false);
                    assert_eq!(view["track"]["ended_reason"], WATCHDOG_REASON);
                }
                break;
            }
            assert!(
                view["goto"]["state"] == "slewing" || (track && view["goto"]["state"] == "halted"),
                "{view}"
            );
            sleep(SAMPLE).await;
        }
    })
    .await
    .expect("watchdog must halt the GoTo before treating still axes as arrival");

    let watchdog_stop = rig
        .commands
        .lock()
        .unwrap()
        .iter()
        .position(|c| c.action == MountCommand::StopAll)
        .expect("watchdog must command StopAll");
    timeout(Duration::from_secs(1), async {
        while rig.mount.state(Axis::Azimuth).await.moving
            || rig.mount.state(Axis::Altitude).await.moving
        {
            sleep(SAMPLE).await;
        }
    })
    .await
    .expect("watchdog stop must bring both axes to rest after braking");
    let mut client = rig.state.link().unwrap().mount().unwrap();
    for axis in [proto::Axis::Azimuth, proto::Axis::Altitude] {
        let reading = client.axis_state(axis).await.unwrap();
        assert_eq!(reading.watchdog_stops, 1);
        assert!(!reading.moving);
        assert_eq!(reading.rate_deg_per_s, 0.0);
    }
    // Cover settle and the next leg, and prove the mount stays stopped while
    // heartbeats continue. The command log already proved a GoTo was sent.
    for _ in 0..30 {
        rig.tick().await;
        for axis in [Axis::Azimuth, Axis::Altitude] {
            assert!(!rig.mount.state(axis).await.moving);
        }
        let view = session(&rig.app).await;
        assert_eq!(view["goto"]["state"], "halted");
        assert_eq!(view["goto"]["reason"], WATCHDOG_REASON);
        if track {
            assert_eq!(view["track"]["state"], "off");
            assert_eq!(view["track"]["ended_reason"], WATCHDOG_REASON);
        }
    }
    let commands = rig.commands.lock().unwrap();
    assert!(
        commands[watchdog_stop + 1..]
            .iter()
            .all(|c| !matches!(c.action, MountCommand::GoTo { .. })),
        "{commands:?}"
    );
    // The watchdog bypasses the console Stop gate: it is the first stop.
    assert!(rig
        .gate
        .log()
        .first()
        .is_none_or(|at| *at >= commands[watchdog_stop].at));
}

#[tokio::test(start_paused = true)]
async fn node_watchdog_halts_goto_without_losing_console_link_or_sending_another_leg() {
    let rig = ready().await;
    connected_during(&rig, async {
        start(&rig, "/api/goto", Value::Null).await;
        fire_during_slew(&rig, false).await;
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn past_watchdog_firing_is_the_baseline_for_a_successful_later_goto() {
    let rig = ready().await;
    // Real silence changes neither clock offset nor exposure timestamps.
    rig.controls.set_unresponsive(true);
    timeout(Duration::from_secs(2), async {
        while rig.commands.lock().unwrap().is_empty()
            || rig.state.link().unwrap().status().borrow().connected
        {
            sleep(SAMPLE).await;
        }
    })
    .await
    .expect("silent link must fire the real node watchdog");
    assert_eq!(
        rig.commands.lock().unwrap()[0].action,
        MountCommand::StopAll
    );
    assert!(!rig.state.link().unwrap().status().borrow().connected);
    rig.controls.set_unresponsive(false);
    timeout(Duration::from_secs(5), async {
        while !rig.state.link().unwrap().status().borrow().connected {
            sleep(SAMPLE).await;
        }
    })
    .await
    .expect("scope link must recover");
    let mut client = rig.state.link().unwrap().mount().unwrap();
    assert_eq!(
        client
            .axis_state(proto::Axis::Azimuth)
            .await
            .unwrap()
            .watchdog_stops,
        1
    );
    let reply = send(&rig.app, post("/api/goto", Value::Null)).await;
    assert_eq!(reply.status(), 200, "{}", body(reply).await);
    let ending = timeout(Duration::from_secs(300), async {
        loop {
            let view = session(&rig.app).await;
            match view["goto"]["state"].as_str() {
                Some("done" | "failed" | "halted") => break view["goto"].clone(),
                _ => rig.tick().await,
            }
        }
    })
    .await
    .expect("recovered GoTo must finish");
    assert_eq!(ending["state"], "done", "{ending}");
    let truth = rig.sim.telescope().eyepiece;
    let target = finder::targets::find("M42").unwrap();
    let error =
        finder::model::separation(truth.ra, truth.dec, target.ra_deg, target.dec_deg) * 3600.0;
    assert!(error < 60.0, "true eyepiece error: {error} arcseconds");
    assert_eq!(
        client
            .axis_state(proto::Axis::Altitude)
            .await
            .unwrap()
            .watchdog_stops,
        1
    );
    assert!(rig
        .commands
        .lock()
        .unwrap()
        .iter()
        .any(|c| matches!(c.action, MountCommand::GoTo { .. })));
}

#[tokio::test(start_paused = true)]
async fn armed_track_retains_the_gotos_watchdog_halt_reason() {
    let rig = ready().await;
    connected_during(&rig, async {
        start(&rig, "/api/track", json!({"on":true})).await;
        let view = session(&rig.app).await;
        assert_eq!(view["track"]["state"], "armed");
        assert_eq!(view["track"]["wanted"], true);
        fire_during_slew(&rig, true).await;
    })
    .await;
}
