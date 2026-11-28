//! US3-S1–S4: imaging-only run centring, a stack's hold, bounded tries and Stop.
//! Oracles: simulator imaging truth and M42's catalogue place (arcsecond bounds
//! below), the session before the run (exact), and 041/057's words and counts
//! (exact). All waits use injected, paused time.
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use finder::{
    imaging::run::{
        console, script::Script, By, Centre, Failure, MountState, MountSteps, RunRequest,
        RunTarget, Runner, StepFuture,
    },
    session::FixedClock,
};
use serde_json::{json, Value};

use super::centre::{m42, Cause, Rig, CENTRE};

async fn imaging_only_run(cause: Cause, size: f64, direction: f64) {
    let rig = Rig::new(17.0).await;
    rig.displace(cause, size, direction).await;
    let before = rig.get("/api/session").await;
    rig.call("POST", "/api/imaging/run", json!({})).await;
    let view = rig
        .wait("/api/imaging", |v| {
            matches!(v["run"]["state"].as_str(), Some("stacking" | "ended"))
        })
        .await;
    let run = &view["run"];
    assert_eq!(run["state"], "stacking", "{run}");
    assert_eq!(run["centring"]["tries"], 1, "{run}");
    assert!(
        run["centring"]["offset_arcsec"].as_f64().unwrap() <= 25.0,
        "{run}"
    );
    assert!(run.get("warnings").is_none(), "{run}");
    assert!(rig.off(m42()) <= 25.0, "{} off M42", rig.off(m42()));
    let after = rig.get("/api/session").await;
    assert_eq!(after["aim"], before["aim"]);
    assert_eq!(after["model"]["epoch"], before["model"]["epoch"]);
    assert_eq!(after["model"]["syncs"], before["model"]["syncs"]);
    rig.call("POST", "/api/stop", Value::Null).await;
}

// S1: each cause has a fresh rig and is independently runnable on the old code.
#[tokio::test(start_paused = true)]
async fn run_centres_after_a_push_without_finder_solves() {
    imaging_only_run(Cause::Push, 390.0, 30.0).await;
}

#[tokio::test(start_paused = true)]
async fn run_centres_with_a_displaced_aim_without_finder_solves() {
    imaging_only_run(Cause::Aim, 240.0, 210.0).await;
}

async fn stack_holds(cause: Cause, restart_frames: bool) {
    let rig = Rig::new(17.0).await;
    rig.displace(cause, 120.0, 120.0).await;
    rig.solve().await;
    rig.call("POST", CENTRE, json!({})).await;
    rig.rests().await;
    if restart_frames {
        rig.sim.start_frames();
    }
    rig.call("PUT", "/api/imaging/settings", json!({"exposure_ms": 2000}))
        .await;
    rig.call("POST", "/api/imaging/stack", json!({})).await;
    let view = rig
        .wait("/api/imaging", |v| {
            v["stack"]["stacked"].as_u64().is_some_and(|n| n >= 15)
        })
        .await;
    assert_eq!(view["steering"]["source"], "subs", "{view}");
    assert!(rig.off(m42()) <= 10.0, "{} off M42", rig.off(m42()));
    rig.call("DELETE", "/api/imaging/stack", Value::Null).await;
}

// S2: pins the place Track holds under a stack; this passes from US1 onward.
#[tokio::test(start_paused = true)]
async fn stack_holds_the_centred_target_without_finder_solves() {
    stack_holds(Cause::Push, false).await;
}

#[tokio::test(start_paused = true)]
async fn stack_holds_the_centred_target_when_finder_frames_return() {
    stack_holds(Cause::Aim, true).await;
}

/// Keep the script's other mount behaviour; the step itself deliberately moves
/// nothing, so all four solves remain 300 arcseconds off. The first step makes
/// the mount unsteady, exposing the run's waiting message and next-frame guard.
struct SteppingMount {
    mount: Arc<dyn MountSteps>,
    script: Arc<Script>,
    calls: AtomicUsize,
}

impl MountSteps for SteppingMount {
    fn target(&self) -> Result<RunTarget, String> {
        self.mount.target()
    }

    fn point(&self, by: By) -> StepFuture<'_, Result<(), String>> {
        self.mount.point(by)
    }

    fn state(&self) -> MountState {
        self.mount.state()
    }

    fn repoint(&self) -> StepFuture<'_, Result<(), Failure>> {
        self.mount.repoint()
    }

    fn stop(&self) -> StepFuture<'_, ()> {
        self.mount.stop()
    }

    fn track_on(&self) -> bool {
        self.mount.track_on()
    }

    fn steady_between(&self, start: f64, end: f64) -> bool {
        self.mount.steady_between(start, end)
    }

    fn centre(&self) -> StepFuture<'_, Result<bool, Failure>> {
        Box::pin(async move {
            if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
                self.script
                    .set_mount(MountState::Tracking { steady: false });
            }
            Ok(true)
        })
    }
}

// S3: exact counts, waiting words and measured warning after three steps.
#[tokio::test(start_paused = true)]
async fn run_waits_after_a_step_and_counts_three_steps_as_measured() {
    let script = Script::new();
    for _ in 0..4 {
        script.push_solve(Ok(Centre {
            offset_arcsec: 300.0,
            tolerance_arcsec: 90.0,
            near_arcsec: 450.0,
            aim_moved: false,
        }));
    }
    let mut steps = script.steps();
    let mount = Arc::new(SteppingMount {
        mount: steps.mount,
        script: script.clone(),
        calls: AtomicUsize::new(0),
    });
    steps.mount = mount.clone();
    let runner = Runner::new(steps, Arc::new(FixedClock(1768528800.0)));
    runner.start(RunRequest::default(), By::Page).await.unwrap();
    tokio::time::sleep(Duration::from_secs(1)).await;
    let first = runner.view().unwrap();
    assert_eq!(first.state, "centring");
    assert_eq!(first.message, "Centring M42: 300″ from the sensor's centre, try 1 of 3. Waiting for the mount to settle; nothing to do.");
    assert_eq!(mount.calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        script
            .calls()
            .iter()
            .filter(|s| s.starts_with("solver.solve"))
            .count(),
        1
    );
    script.set_mount(MountState::Tracking { steady: true });
    tokio::time::sleep(Duration::from_secs(5)).await;
    let view = runner.view().unwrap();
    assert_eq!(view.state, "stacking");
    assert_eq!(mount.calls.load(Ordering::SeqCst), 3);
    assert_eq!(view.centring.as_ref().unwrap().tries, 3);
    assert_eq!(
        view.warnings,
        ["M42 is 300″ from the sensor's centre after 3 tries; it is stacked where it is."]
    );
}

// S4: the console's Stop since point must win over the mount part's centre.
#[tokio::test(start_paused = true)]
async fn console_stop_prevents_the_mount_part_from_centring() {
    let rig = Rig::new(17.0).await;
    let part = console::mount(&rig.state);
    assert_eq!(part.point(By::Page).await, Ok(()));
    rig.wait("/api/session", |v| v["track"]["state"] == "tracking")
        .await;
    rig.sleep(30.0).await;
    rig.solve().await;
    rig.call("POST", "/api/stop", Value::Null).await;
    assert_eq!(part.centre().await, Err(Failure::Stopped));
    assert_ne!(rig.get("/api/session").await["axis_move"]["tool"], "centre");
}
