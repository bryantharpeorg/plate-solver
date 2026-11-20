//! US1-S1/S2: exact states, words, calls and JSON shapes from the specification,
//! over US12's independent script. All numbers compare exactly; waits use paused time.

use std::{collections::BTreeSet, sync::Arc, time::Duration};

use finder::{
    imaging::run::{
        script::{Part, Script, Step, STACK_FOLDER},
        By, Failure, MountState, RunRequest, Runner,
    },
    session::FixedClock,
};
use serde_json::json;

fn scripted_run() -> (Arc<Script>, Arc<Runner>) {
    let script = Script::new();
    let runner = Runner::new(script.steps(), Arc::new(FixedClock(1768528800.0)));
    (script, runner)
}

async fn settle() {
    tokio::time::sleep(Duration::from_secs(1)).await;
}

#[tokio::test(start_paused = true)]
async fn straight_path_states_words_calls_and_view() {
    let (script, runner) = scripted_run();
    for step in [Step::Take, Step::Check, Step::Solve, Step::StackStart] {
        script.hold(step);
    }
    script.set_mount(MountState::Slewing);
    let start = runner.start(RunRequest::default(), By::Page).await.unwrap();
    assert_eq!(start.id, "2026-01-16T02-00-00Z-M42");
    assert_eq!(start.by, "page");
    assert_eq!(
        serde_json::to_value(&start.target).unwrap(),
        json!({"id":"NGC 1976","name":"M42","ra_deg":83.82,"dec_deg":-5.39})
    );
    assert_eq!(start.started_at, 1768528800.0);
    assert_eq!(start.elapsed_s, 0.0);
    assert_eq!(
        serde_json::to_value(&start.settings).unwrap(),
        json!({"subs":null,"duration_s":7200.0,"keep_subs":true})
    );
    assert_eq!(
        serde_json::to_value(&start.subs).unwrap(),
        json!({"stacked":0,"dropped":{}})
    );
    assert_eq!(start.state, "slewing");
    assert!(start.folder.is_none());

    settle().await;
    let first = runner.view().unwrap();
    script.set_mount(MountState::Tracking { steady: true });
    settle().await;
    let second = runner.view().unwrap();
    assert_eq!(script.calls().last().unwrap(), "camera.take");
    script.release(Step::Take);
    settle().await;
    let third = runner.view().unwrap();
    script.release(Step::Check);
    settle().await;
    let fourth = runner.view().unwrap();
    script.release(Step::Solve);
    settle().await;
    let fifth = runner.view().unwrap();
    assert!(fifth.folder.is_none());
    assert!(fifth.stacking_since.is_none());
    assert!(fifth.limit_at.is_none());
    script.release(Step::StackStart);
    settle().await;
    let sixth = runner.view().unwrap();

    for (view, state, message) in [
        (&first, "slewing", "Slewing to M42."),
        (&second, "first_frame", "Taking a first imaging frame."),
        (&third, "focus_check", "Checking focus."),
        (&fourth, "solving", "Solving an imaging frame."),
        (&fifth, "stacking", "Stacking M42: 0 stacked, 0 dropped."),
        (&sixth, "stacking", "Stacking M42: 0 stacked, 0 dropped."),
    ] {
        assert_eq!(view.state, state);
        assert_eq!(view.message, message);
    }
    assert_eq!(sixth.folder.as_deref(), Some(STACK_FOLDER));
    assert_eq!(sixth.stacking_since, Some(1768528800.0));
    assert_eq!(sixth.limit_at, Some(1768536000.0));
    assert_eq!(
        serde_json::to_value(&sixth.focus).unwrap(),
        json!({"verdict":"good","reading":3.0,"record":3.0,"unit":"arcsec"})
    );
    assert_eq!(
        serde_json::to_value(&sixth.centring).unwrap(),
        json!({"tries":0,"offset_arcsec":0.0,"tolerance_arcsec":90.0})
    );
    assert_eq!(
        script.calls(),
        [
            "camera.ready",
            "solver.ready",
            "mount.target",
            "camera.hold",
            "mount.point page",
            "camera.take",
            "camera.release",
            "focus.check",
            "camera.hold",
            "camera.take",
            "solver.solve 2",
            "camera.release",
            "stack.start M42",
        ]
    );

    let first_json = serde_json::to_value(&first).unwrap();
    let sixth_json = serde_json::to_value(&sixth).unwrap();
    assert_eq!(
        first_json
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            "id",
            "state",
            "by",
            "target",
            "started_at",
            "elapsed_s",
            "settings",
            "subs",
            "message",
        ])
    );
    assert_eq!(
        sixth_json
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            "id",
            "state",
            "by",
            "target",
            "started_at",
            "elapsed_s",
            "settings",
            "subs",
            "message",
            "focus",
            "centring",
            "stacking_since",
            "limit_at",
            "folder",
        ])
    );
}

#[tokio::test(start_paused = true)]
async fn lease_refused_after_focus_ends_before_solving() {
    let (script, runner) = scripted_run();
    script.hold(Step::Check);
    runner.start(RunRequest::default(), By::Page).await.unwrap();
    settle().await;
    script.refuse(Part::Hold, "the imaging camera is held by a set of darks");
    script.release(Step::Check);
    settle().await;
    let view = runner.view().unwrap();
    assert_eq!(view.state, "ended");
    let ended = view.ended.unwrap();
    assert_eq!(ended.reason, "camera_failed");
    assert_eq!(
        view.message,
        "Ended: the imaging camera failed: the imaging camera is held by a set of darks."
    );
    assert_eq!(ended.message, view.message);
    assert!(!script
        .calls()
        .iter()
        .any(|call| call.starts_with("solver.solve")));
    let ended_json = serde_json::to_value(&ended).unwrap();
    assert_eq!(
        ended_json
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["reason", "at", "message", "saved", "track_on"])
    );
    assert_eq!(ended.at, 1768528800.0);
    assert!(!ended.saved);
    assert!(ended.track_on);
}

#[tokio::test(start_paused = true)]
async fn unfinished_focus_is_unmeasurable_with_its_warning() {
    let (script, runner) = scripted_run();
    script.push_focus(Err(Failure::Other(
        "the focus check ended: no_region".into(),
    )));
    runner.start(RunRequest::default(), By::Page).await.unwrap();
    settle().await;
    let view = runner.view().unwrap();
    assert_eq!(
        serde_json::to_value(&view.focus).unwrap(),
        json!({"verdict":"unmeasurable","unit":"px"})
    );
    assert_eq!(
        view.warnings,
        ["The focus check did not finish: the focus check ended: no_region."]
    );
}

#[tokio::test(start_paused = true)]
async fn stacking_limit_uses_the_requested_duration() {
    let (_script, runner) = scripted_run();
    runner
        .start(
            RunRequest {
                duration_s: Some(60.0),
                ..RunRequest::default()
            },
            By::Page,
        )
        .await
        .unwrap();
    settle().await;
    let view = runner.view().unwrap();
    assert_eq!(view.state, "stacking");
    assert_eq!(view.limit_at, Some(1768528860.0));
}
