//! US5-S1–S3: exact words, counts and call order from the scenarios, over 037's
//! script. All numeric expectations are exact; time is injected and paused.
use std::{sync::Arc, time::Duration};

use finder::{
    imaging::run::{
        script::{Script, Step},
        By, Centre, Failure, MountState, RunRequest, RunView, Runner,
    },
    session::FixedClock,
};
use serde_json::json;

fn rig(solves: &[(f64, bool)]) -> (Arc<Script>, Arc<Runner>) {
    let script = Script::new();
    for &(offset_arcsec, aim_moved) in solves {
        script.push_solve(Ok(Centre {
            offset_arcsec,
            tolerance_arcsec: 90.0,
            near_arcsec: 450.0,
            aim_moved,
        }));
    }
    let runner = Runner::new(script.steps(), Arc::new(FixedClock(1768528800.0)));
    (script, runner)
}

async fn settle() {
    tokio::time::sleep(Duration::from_secs(1)).await;
}

async fn start(runner: &Arc<Runner>) -> RunView {
    runner.start(RunRequest::default(), By::Page).await.unwrap();
    settle().await;
    runner.view().unwrap()
}

fn count(script: &Script, prefix: &str) -> usize {
    script
        .calls()
        .iter()
        .filter(|call| call.starts_with(prefix))
        .count()
}

fn centre(view: &RunView, tries: u32, offset: f64) {
    assert_eq!(
        serde_json::to_value(&view.centring).unwrap(),
        json!({"tries":tries,"offset_arcsec":offset,"tolerance_arcsec":90.0})
    );
}

#[tokio::test(start_paused = true)]
async fn one_recentre_then_solve_inside_tolerance_stacks() {
    let (script, runner) = rig(&[(400.0, true), (50.0, true)]);
    script.hold(Step::Repoint);
    let view = start(&runner).await;
    assert_eq!(view.state, "centring");
    assert_eq!(
        view.message,
        "Centring M42: 400″ from the sensor's centre, try 1 of 3."
    );
    centre(&view, 1, 400.0);
    script.release(Step::Repoint);
    settle().await;
    let view = runner.view().unwrap();
    assert_eq!(view.state, "stacking");
    centre(&view, 1, 50.0);
    let calls = script.calls();
    let solved = calls
        .iter()
        .position(|call| call == "solver.solve 2")
        .unwrap();
    assert_eq!(
        &calls[solved + 1..],
        [
            "mount.repoint",
            "camera.take",
            "solver.solve 3",
            "camera.release",
            "stack.start M42"
        ]
    );
}

#[tokio::test(start_paused = true)]
async fn three_tries_end_far_target_in_arcminutes_without_a_stack() {
    let (script, runner) = rig(&[(600.0, true); 4]);
    let view = start(&runner).await;
    assert_eq!(view.state, "ended");
    assert_eq!(view.ended.as_ref().unwrap().reason, "not_centred");
    assert_eq!(view.message, "Ended: M42 is still 10.0′ from the sensor's centre after 3 tries. Check the aim point on the console page, then start again.");
    centre(&view, 3, 600.0);
    assert_eq!(count(&script, "mount.repoint"), 3);
    assert_eq!(count(&script, "solver.solve"), 4);
    assert_eq!(count(&script, "stack.start"), 0);
}

#[tokio::test(start_paused = true)]
async fn no_new_aim_is_no_try_and_waits_for_steady_without_repoint() {
    let (script, runner) = rig(&[(400.0, false), (50.0, false)]);
    script.hold(Step::Solve);
    start(&runner).await;
    assert_eq!(script.calls().last().unwrap(), "solver.solve 2");
    script.set_mount(MountState::Tracking { steady: false });
    script.release(Step::Solve);
    settle().await;
    let view = runner.view().unwrap();
    assert_eq!(view.state, "centring");
    centre(&view, 0, 400.0);
    assert_eq!(view.message, "Centring M42: 400″ from the sensor's centre, no new aim point yet. Waiting for the mount to settle; nothing to do.");
    script.set_mount(MountState::Tracking { steady: true });
    settle().await;
    let view = runner.view().unwrap();
    assert_eq!(view.state, "stacking");
    centre(&view, 0, 50.0);
    assert_eq!(count(&script, "mount.repoint"), 0);
    assert_eq!(count(&script, "solver.solve"), 2);
}

#[tokio::test(start_paused = true)]
async fn four_idle_solves_stack_near_target_but_end_far_target() {
    for (offset, state, warning, message) in [
        (400.0, "stacking", Some("M42 is 400″ from the sensor's centre and no new aim point could be measured; it is stacked where it is."), None),
        (600.0, "ended", None, Some("Ended: M42 is 10.0′ from the sensor's centre and no new aim point could be measured.")),
    ] {
        let (script, runner) = rig(&[(offset, false); 4]);
        let view = start(&runner).await;
        assert_eq!(view.state, state);
        centre(&view, 0, offset);
        assert_eq!(count(&script, "solver.solve"), 4);
        assert_eq!(count(&script, "mount.repoint"), 0);
        assert_eq!(count(&script, "stack.start"), usize::from(state == "stacking"));
        assert_eq!(view.warnings, warning.into_iter().collect::<Vec<_>>());
        if let Some(message) = message {
            assert_eq!(view.ended.as_ref().unwrap().reason, "not_centred");
            assert_eq!(view.message, message);
        }
    }
}

#[tokio::test(start_paused = true)]
async fn a_recentre_resets_the_consecutive_idle_count() {
    let (script, runner) = rig(&[
        (400.0, false),
        (400.0, false),
        (400.0, false),
        (400.0, true),
        (400.0, false),
        (400.0, false),
        (400.0, false),
        (50.0, false),
    ]);
    let view = start(&runner).await;
    assert_eq!(view.state, "stacking");
    centre(&view, 1, 50.0);
    assert_eq!(count(&script, "solver.solve"), 8);
    assert_eq!(count(&script, "mount.repoint"), 1);
    assert!(view.warnings.is_empty());
}

#[tokio::test(start_paused = true)]
async fn idle_solve_after_a_try_uses_the_no_new_aim_message() {
    let (script, runner) = rig(&[(400.0, true), (400.0, false), (50.0, false)]);
    script.hold(Step::Repoint);
    let view = start(&runner).await;
    assert_eq!(
        view.message,
        "Centring M42: 400″ from the sensor's centre, try 1 of 3."
    );
    script.hold(Step::Solve);
    script.release(Step::Repoint);
    settle().await;
    assert_eq!(script.calls().last().unwrap(), "solver.solve 3");
    script.set_mount(MountState::Tracking { steady: false });
    script.release(Step::Solve);
    settle().await;
    let view = runner.view().unwrap();
    assert_eq!(view.state, "centring");
    centre(&view, 1, 400.0);
    assert_eq!(view.message, "Centring M42: 400″ from the sensor's centre, no new aim point yet. Waiting for the mount to settle; nothing to do.");
    script.set_mount(MountState::Tracking { steady: true });
    settle().await;
    assert_eq!(runner.view().unwrap().state, "stacking");
    assert_eq!(count(&script, "mount.repoint"), 1);
}

#[tokio::test(start_paused = true)]
async fn near_bounds_and_both_warning_forms_follow_the_offset() {
    for (offset, moved, tries, warning) in [
        (400.0, true, 3, "M42 is 400″ from the sensor's centre after 3 tries; it is stacked where it is."),
        (450.0, true, 3, "M42 is 450″ from the sensor's centre after 3 tries; it is stacked where it is."),
        (180.0, true, 3, "M42 is 180″ from the sensor's centre after 3 tries; the stack's steering will bring it in."),
        (150.0, false, 0, "M42 is 150″ from the sensor's centre and no new aim point could be measured; the stack's steering will bring it in."),
    ] {
        let (script, runner) = rig(&[(offset, moved); 4]);
        let view = start(&runner).await;
        assert_eq!(view.state, "stacking");
        centre(&view, tries, offset);
        assert_eq!(view.warnings, [warning]);
        assert_eq!(count(&script, "mount.repoint"), tries as usize);
        assert_eq!(count(&script, "solver.solve"), 4);
    }
}

#[tokio::test(start_paused = true)]
async fn exactly_the_tolerance_needs_no_try() {
    let (script, runner) = rig(&[(90.0, true)]);
    let view = start(&runner).await;
    assert_eq!(view.state, "stacking");
    centre(&view, 0, 90.0);
    assert_eq!(count(&script, "mount.repoint"), 0);
    assert!(view.warnings.is_empty());
}

#[tokio::test(start_paused = true)]
async fn failed_repoint_ends_with_mount_words_or_stopped() {
    for (failure, reason) in [
        (
            Failure::Mount("the mount is already moving".into()),
            "mount_failed",
        ),
        (Failure::Stopped, "stopped"),
    ] {
        let (script, runner) = rig(&[(400.0, true)]);
        script.fail_repoint(failure);
        let view = start(&runner).await;
        assert_eq!(view.state, "ended");
        assert_eq!(view.ended.as_ref().unwrap().reason, reason);
        if reason == "mount_failed" {
            assert_eq!(view.message, "Ended: Track ended and the run with it: the mount is already moving. Check the mount, then start again.");
        }
        assert_eq!(count(&script, "mount.repoint"), 1);
        assert_eq!(count(&script, "solver.solve"), 1);
        assert_eq!(script.calls().last().unwrap(), "camera.release");
        assert_eq!(count(&script, "stack.start"), 0);
    }
}

#[tokio::test(start_paused = true)]
async fn no_take_after_repoint_until_mount_is_steady() {
    let (script, runner) = rig(&[(400.0, true), (50.0, true)]);
    script.hold(Step::Repoint);
    start(&runner).await;
    assert_eq!(script.calls().last().unwrap(), "mount.repoint");
    script.set_mount(MountState::Tracking { steady: false });
    script.release(Step::Repoint);
    tokio::time::sleep(Duration::from_secs(10)).await;
    assert_eq!(script.calls().last().unwrap(), "mount.repoint");
    assert_eq!(runner.view().unwrap().state, "centring");
    script.set_mount(MountState::Tracking { steady: true });
    settle().await;
    assert_eq!(runner.view().unwrap().state, "stacking");
    assert_eq!(count(&script, "camera.take"), 3);
}

#[tokio::test(start_paused = true)]
async fn stop_waits_for_repoint_and_prevents_another_take() {
    let (script, runner) = rig(&[(400.0, true), (50.0, true)]);
    script.hold(Step::Repoint);
    start(&runner).await;
    assert_eq!(script.calls().last().unwrap(), "mount.repoint");
    let stop = runner.stop();
    tokio::pin!(stop);
    tokio::select! {
        answer = &mut stop => panic!("stop abandoned repoint: {answer:?}"),
        _ = tokio::time::sleep(Duration::from_secs(10)) => {},
    }
    script.release(Step::Repoint);
    let view = stop.await.unwrap();
    assert_eq!(view.state, "ended");
    assert_eq!(view.ended.unwrap().reason, "stopped");
    let calls = script.calls();
    let repoint = calls
        .iter()
        .position(|call| call == "mount.repoint")
        .unwrap();
    assert_eq!(&calls[repoint + 1..], ["camera.release"]);
}
