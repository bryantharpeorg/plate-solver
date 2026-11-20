//! US2's oracle is the specification's literal words, counts and call order over
//! the independent script. Comparisons are exact; all waits use paused time.

use std::{sync::Arc, time::Duration};

use finder::{
    imaging::run::{
        script::{Part, Script, Step},
        By, Ending, Failure, FocusCheck, FrameRef, MountState, RunRequest, RunView, Runner, Unit,
        Verdict,
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

async fn start(runner: &Arc<Runner>) {
    runner.start(RunRequest::default(), By::Page).await.unwrap();
    settle().await;
}

fn calls(script: &Script, prefix: &str) -> usize {
    script
        .calls()
        .iter()
        .filter(|call| call.starts_with(prefix))
        .count()
}

fn assert_ended(view: &RunView, reason: &str, message: &str) {
    assert_eq!(view.state, "ended");
    let ended = view.ended.as_ref().unwrap();
    assert_eq!(ended.reason, reason);
    assert_eq!(ended.message, message);
    assert_eq!(view.message, message);
}

// US2-S1: two takes fail, then the first frame and the solve frame succeed.
#[tokio::test(start_paused = true)]
async fn failed_takes_wait_five_seconds_before_each_retry() {
    let (script, runner) = scripted_run();
    for _ in 0..2 {
        script.push_frame(Err(Failure::Camera("boom".into())));
    }
    runner.start(RunRequest::default(), By::Page).await.unwrap();
    tokio::time::sleep(Duration::from_millis(4500)).await;
    assert_eq!(calls(&script, "camera.take"), 1);
    tokio::time::sleep(Duration::from_millis(500)).await;
    // Both tasks wake at this instant. Let the runner poll without advancing time.
    tokio::task::yield_now().await;
    assert_eq!(calls(&script, "camera.take"), 2);
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(calls(&script, "camera.take"), 2);
    tokio::time::sleep(Duration::from_secs(4)).await;
    assert_eq!(calls(&script, "camera.take"), 2);
    tokio::time::sleep(Duration::from_millis(500)).await;
    tokio::task::yield_now().await;
    assert_eq!(calls(&script, "camera.take"), 4);
    settle().await;
    assert_eq!(runner.view().unwrap().elapsed_s, 11.0);
    assert_eq!(runner.view().unwrap().state, "stacking");
    assert_eq!(calls(&script, "camera.take"), 4);
}

#[tokio::test(start_paused = true)]
async fn three_failed_takes_end_with_the_last_error_before_focus() {
    let (script, runner) = scripted_run();
    for words in ["boom", "boom", "gone"] {
        script.push_frame(Err(Failure::Camera(words.into())));
    }
    runner.start(RunRequest::default(), By::Page).await.unwrap();
    tokio::time::sleep(Duration::from_secs(11)).await;
    assert_ended(
        &runner.view().unwrap(),
        "camera_failed",
        "Ended: the imaging camera failed: gone.",
    );
    assert_eq!(calls(&script, "camera.take"), 3);
    assert_eq!(calls(&script, "focus.check"), 0);
    assert_eq!(script.calls().last().unwrap(), "camera.release");
}

#[tokio::test(start_paused = true)]
async fn stop_interrupts_the_first_retry_wait() {
    let (script, runner) = scripted_run();
    for _ in 0..3 {
        script.push_frame(Err(Failure::Camera("boom".into())));
    }
    runner.start(RunRequest::default(), By::Page).await.unwrap();
    tokio::time::sleep(Duration::from_secs(2)).await;
    let view = tokio::time::timeout(Duration::from_secs(1), runner.stop())
        .await
        .unwrap()
        .unwrap();
    assert_ended(&view, "stopped", "Imaging run stopped. Track is still on.");
    assert_eq!(calls(&script, "camera.take"), 1);
}

#[tokio::test(start_paused = true)]
async fn a_take_that_cannot_save_ends_without_retrying() {
    let (script, runner) = scripted_run();
    script.push_frame(Err(Failure::Save(
        "could not save the imaging frame: disk".into(),
    )));
    start(&runner).await;
    assert_ended(&runner.view().unwrap(), "save_failed", "Ended: the console could not save to its imaging folder: could not save the imaging frame: disk. Check the disk.");
    assert_eq!(calls(&script, "camera.take"), 1);
    assert_eq!(calls(&script, "focus.check"), 0);
}

#[tokio::test(start_paused = true)]
async fn every_other_take_failure_is_counted_and_the_mount_can_interrupt_its_wait() {
    for failure in [
        Failure::SolverUnavailable("boom".into()),
        Failure::NotSolved("boom".into()),
        Failure::Mount("boom".into()),
        Failure::Other("boom".into()),
    ] {
        let (script, runner) = scripted_run();
        for _ in 0..3 {
            script.push_frame(Err(failure.clone()));
        }
        runner.start(RunRequest::default(), By::Page).await.unwrap();
        tokio::time::sleep(Duration::from_secs(11)).await;
        assert_ended(
            &runner.view().unwrap(),
            "camera_failed",
            "Ended: the imaging camera failed: boom.",
        );
        assert_eq!(calls(&script, "camera.take"), 3);
        assert_eq!(calls(&script, "focus.check"), 0);
    }
    let (script, runner) = scripted_run();
    script.push_frame(Err(Failure::Camera("boom".into())));
    start(&runner).await;
    script.set_mount(MountState::Ended(Ending::LinkLost));
    settle().await;
    assert_ended(&runner.view().unwrap(), "link_lost", "Ended: the mount or the scope link stopped answering. Check the scope node and the mount's cable, then start again.");
    assert_eq!(calls(&script, "camera.take"), 1);
    // There is no exposure to abort during the retry wait.
    assert_eq!(calls(&script, "camera.abort"), 0);
}

#[tokio::test(start_paused = true)]
async fn a_successful_take_resets_the_camera_failure_count() {
    let (script, runner) = scripted_run();
    for id in [1, 2] {
        for _ in 0..2 {
            script.push_frame(Err(Failure::Camera("boom".into())));
        }
        script.push_frame(Ok(FrameRef {
            id,
            file: format!("frames/{id}.fits"),
        }));
    }
    runner.start(RunRequest::default(), By::Page).await.unwrap();
    tokio::time::sleep(Duration::from_secs(21)).await;
    assert_eq!(runner.view().unwrap().state, "stacking");
    assert_eq!(calls(&script, "camera.take"), 6);
    assert_eq!(calls(&script, "focus.check"), 1);
    assert_eq!(calls(&script, "solver.solve 2"), 1);
}

// US2-S2: only a new imaging frame and solve are repeated, never the focus check.
#[tokio::test(start_paused = true)]
async fn solves_without_a_solution_retry_with_new_frames_and_no_focus_check() {
    let (script, runner) = scripted_run();
    for _ in 0..2 {
        script.push_solve(Err(Failure::NotSolved(
            "the imaging solver found no solution".into(),
        )));
    }
    start(&runner).await;
    assert_eq!(runner.view().unwrap().state, "stacking");
    let recorded = script.calls();
    let focus = recorded
        .iter()
        .position(|call| call == "focus.check")
        .unwrap();
    assert_eq!(
        &recorded[focus + 1..],
        [
            "camera.hold",
            "camera.take",
            "solver.solve 2",
            "camera.take",
            "solver.solve 3",
            "camera.take",
            "solver.solve 4",
            "camera.release",
            "stack.start M42"
        ]
    );
    assert_eq!(calls(&script, "focus.check"), 1);
}

#[tokio::test(start_paused = true)]
async fn three_unsolved_frames_end_solver_failing() {
    let (script, runner) = scripted_run();
    for _ in 0..3 {
        script.push_solve(Err(Failure::NotSolved(
            "the imaging solver found no solution".into(),
        )));
    }
    start(&runner).await;
    assert_ended(&runner.view().unwrap(), "solver_failing", "Ended: 3 imaging frames in a row could not be solved: the imaging solver found no solution. Check for cloud, dew and focus.");
    assert_eq!(calls(&script, "solver.solve"), 3);
    assert_eq!(calls(&script, "camera.take"), 4);
    assert_eq!(calls(&script, "focus.check"), 1);
}

#[tokio::test(start_paused = true)]
async fn other_counted_solve_failures_use_the_last_errors_words() {
    let (script, runner) = scripted_run();
    for failure in [
        Failure::Camera("first".into()),
        Failure::Mount("second".into()),
        Failure::Other("last".into()),
    ] {
        script.push_solve(Err(failure));
    }
    start(&runner).await;
    assert_ended(&runner.view().unwrap(), "solver_failing", "Ended: 3 imaging frames in a row could not be solved: last. Check for cloud, dew and focus.");
    assert_eq!(calls(&script, "solver.solve"), 3);
    assert_eq!(calls(&script, "focus.check"), 1);
}

#[tokio::test(start_paused = true)]
async fn unavailable_solver_and_unsaved_solve_end_immediately_by_kind() {
    for (failure, reason, message) in [
        (Failure::SolverUnavailable("the imaging solver has no star database it can read".into()), "solver_unavailable", "Ended: the imaging solver cannot run: the imaging solver has no star database it can read."),
        (Failure::Save("could not save the imaging solve: disk".into()), "save_failed", "Ended: the console could not save to its imaging folder: could not save the imaging solve: disk. Check the disk."),
    ] {
        let (script, runner) = scripted_run();
        script.push_solve(Err(failure));
        start(&runner).await;
        assert_ended(&runner.view().unwrap(), reason, message);
        assert_eq!(calls(&script, "solver.solve"), 1);
        assert_eq!(calls(&script, "stack.start"), 0);
    }
}

#[tokio::test(start_paused = true)]
async fn focus_that_cannot_save_or_run_its_solver_ends_by_kind() {
    for (failure, reason, message) in [
        (Failure::Save("could not save the imaging frame: disk".into()), "save_failed", "Ended: the console could not save to its imaging folder: could not save the imaging frame: disk. Check the disk."),
        (Failure::SolverUnavailable("this console has no imaging solver".into()), "solver_unavailable", "Ended: the imaging solver cannot run: this console has no imaging solver."),
    ] {
        let (script, runner) = scripted_run();
        script.push_focus(Err(failure));
        start(&runner).await;
        assert_ended(&runner.view().unwrap(), reason, message);
        assert_eq!(calls(&script, "focus.check"), 1);
        assert_eq!(calls(&script, "solver.solve"), 0);
    }
}

// US2-S3: the stack owns the counts and the tracker pause, in both directions.
#[tokio::test(start_paused = true)]
async fn stack_counts_and_tracker_pause_are_shown_as_reported() {
    let (script, runner) = scripted_run();
    start(&runner).await;
    script.set_stack(5, &[("not_solved", 2)], false);
    settle().await;
    let view = runner.view().unwrap();
    assert_eq!(
        serde_json::to_value(&view.subs).unwrap(),
        json!({"stacked":5,"dropped":{"not_solved":2}})
    );
    assert_eq!(view.state, "stacking");
    assert_eq!(view.message, "Stacking M42: 5 stacked, 2 dropped.");
    script.set_stack(5, &[("not_solved", 2)], true);
    settle().await;
    let view = runner.view().unwrap();
    assert_eq!(view.state, "paused_by_tracker");
    assert_eq!(
        view.message,
        "Exposures paused: 5 stacked so far. They resume by themselves."
    );
    assert_eq!(
        serde_json::to_value(&view.subs).unwrap(),
        json!({"stacked":5,"dropped":{"not_solved":2}})
    );
    script.set_stack(5, &[("not_solved", 2)], false);
    settle().await;
    assert_eq!(runner.view().unwrap().state, "stacking");
    assert_eq!(
        runner.view().unwrap().message,
        "Stacking M42: 5 stacked, 2 dropped."
    );
}

#[tokio::test(start_paused = true)]
async fn slightly_soft_focus_keeps_its_warning_in_either_unit() {
    for (reading, record, unit, word, warning) in [
        (
            3.6,
            3.1,
            Unit::Arcsec,
            "arcsec",
            "Focus is slightly soft: stars are 3.6″ across against a record of 3.1″.",
        ),
        (
            6.6,
            5.6,
            Unit::Px,
            "px",
            "Focus is slightly soft: stars are 6.6 px across against a record of 5.6 px.",
        ),
    ] {
        let (script, runner) = scripted_run();
        script.push_focus(Ok(FocusCheck {
            verdict: Verdict::SlightlySoft,
            reading: Some(reading),
            record: Some(record),
            unit,
        }));
        start(&runner).await;
        let view = runner.view().unwrap();
        assert_eq!(view.state, "stacking");
        assert_eq!(view.warnings, [warning]);
        let focus = view.focus.unwrap();
        assert_eq!(focus.verdict, "slightly_soft");
        assert_eq!(focus.unit, word);
        assert_eq!(focus.reading, Some(reading));
        assert_eq!(focus.record, Some(record));
        settle().await;
        assert_eq!(runner.stop().await.unwrap().warnings, [warning]);
    }
}

// US2-S4: starting a stack is a hand-over; a failed start never owns a stack.
#[tokio::test(start_paused = true)]
async fn stack_start_failure_is_sorted_by_kind_without_stopping_a_stack() {
    for (failure, reason, message) in [
        (Failure::Save("cannot make the stack folder: no space left on device".into()), "save_failed", "Ended: the console could not save to its imaging folder: cannot make the stack folder: no space left on device. Check the disk."),
        (Failure::Other("the stack is busy".into()), "stack_failed", "Ended: the stack stopped: the stack is busy."),
        (Failure::NotSolved("the imaging solver found no solution".into()), "solver_failing", "Ended: 3 imaging frames in a row could not be solved: the imaging solver found no solution. Check for cloud, dew and focus."),
        (Failure::Camera("the imaging camera is held by a set of darks".into()), "camera_failed", "Ended: the imaging camera failed: the imaging camera is held by a set of darks."),
        (Failure::SolverUnavailable("the imaging solver has no star database it can read".into()), "solver_unavailable", "Ended: the imaging solver cannot run: the imaging solver has no star database it can read."),
        (Failure::Stopped, "stopped", "Stopped: the mount was stopped."),
        (Failure::Mount("Track failed".into()), "stack_failed", "Ended: the stack stopped: Track failed."),
    ] {
        let (script, runner) = scripted_run();
        script.fail_stack_start(failure);
        start(&runner).await;
        let view = runner.view().unwrap();
        assert_ended(&view, reason, message);
        assert!(view.folder.is_none());
        assert!(!view.ended.unwrap().saved);
        assert_eq!(calls(&script, "stack.stop"), 0);
        let recorded = script.calls();
        assert_eq!(&recorded[recorded.len() - 2..], ["stack.start M42", "camera.release"]);
    }
}

#[tokio::test(start_paused = true)]
async fn unclean_stack_stop_warns_and_preserves_the_ending() {
    let (script, runner) = scripted_run();
    script.fail_stack_stop(Failure::Other("could not remove subs/000004.fits".into()));
    start(&runner).await;
    let view = runner.stop().await.unwrap();
    assert_ended(&view, "stopped", "Imaging run stopped. Track is still on.");
    assert!(!view.ended.unwrap().saved);
    assert_eq!(
        view.warnings,
        ["The stack did not end cleanly: could not remove subs/000004.fits."]
    );
    assert_eq!(calls(&script, "stack.stop"), 1);
    assert_eq!(script.calls().last().unwrap(), "camera.release");
}

#[tokio::test(start_paused = true)]
async fn discarded_subs_warn_only_when_nothing_was_stacked() {
    for stacked in [0, 2] {
        let (script, runner) = scripted_run();
        runner
            .start(
                RunRequest {
                    keep_subs: Some(false),
                    ..RunRequest::default()
                },
                By::Page,
            )
            .await
            .unwrap();
        settle().await;
        assert_eq!(runner.view().unwrap().state, "stacking");
        script.set_stack(stacked, &[], false);
        settle().await;
        let view = runner.stop().await.unwrap();
        assert_eq!(view.subs.stacked, stacked);
        assert_eq!(calls(&script, "stack.stop discard"), 1);
        assert_eq!(calls(&script, "stack.stop"), 1);
        if stacked == 0 {
            assert_eq!(view.warnings, ["The subs were kept: nothing was stacked."]);
        } else {
            assert!(view.warnings.is_empty());
        }
    }
}

#[tokio::test(start_paused = true)]
async fn ending_saves_before_admitting_another_run_and_reads_the_final_counts() {
    for final_count in [2, 3] {
        let (script, runner) = scripted_run();
        script.hold(Step::StackStop);
        start(&runner).await;
        script.set_stack(2, &[], false);
        settle().await;
        let stopping = tokio::spawn({
            let runner = runner.clone();
            async move { runner.stop().await.unwrap() }
        });
        settle().await;
        assert!(!stopping.is_finished());
        let view = runner.view().unwrap();
        assert_eq!(view.state, "stacking");
        assert_eq!(view.message, "Saving the stack: 2 stacked.");
        assert!(view.ended.is_none());
        let refusal = runner
            .start(RunRequest::default(), By::Page)
            .await
            .unwrap_err();
        assert_eq!(refusal.status.as_u16(), 409);
        assert_eq!(refusal.message, "an imaging run is already running");
        if final_count == 3 {
            script.set_stack(3, &[("not_solved", 1)], false);
        }
        script.release(Step::StackStop);
        let view = stopping.await.unwrap();
        assert_ended(&view, "stopped", "Imaging run stopped. Track is still on.");
        assert_eq!(view.subs.stacked, final_count);
        if final_count == 3 {
            assert_eq!(
                serde_json::to_value(&view.subs).unwrap(),
                json!({"stacked":3,"dropped":{"not_solved":1}})
            );
        }
        assert!(view.ended.unwrap().saved);
        assert_eq!(calls(&script, "stack.stop"), 1);
        assert_eq!(script.calls().last().unwrap(), "camera.release");
    }
}

// US2-S5: the state's words change as soon as its steady wait ends.
#[tokio::test(start_paused = true)]
async fn solving_shows_a_wait_for_the_tracker_until_it_is_steady() {
    let (script, runner) = scripted_run();
    script.hold(Step::Check);
    script.hold(Step::Solve);
    start(&runner).await;
    script.set_mount(MountState::Tracking { steady: false });
    script.release(Step::Check);
    settle().await;
    let view = runner.view().unwrap();
    assert_eq!(view.state, "solving");
    assert_eq!(
        view.message,
        "Solving an imaging frame. Waiting for the mount to settle; nothing to do."
    );
    assert_eq!(calls(&script, "camera.take"), 1);
    script.set_mount(MountState::Tracking { steady: true });
    settle().await;
    assert_eq!(runner.view().unwrap().state, "solving");
    assert_eq!(runner.view().unwrap().message, "Solving an imaging frame.");
    script.release(Step::Solve);
    settle().await;
    assert_eq!(runner.view().unwrap().state, "stacking");
}

// US2-S6 to S10 preserve the US1 machine before the US2 changes as well as after.
#[tokio::test(start_paused = true)]
async fn preserved_tracking_waits_without_taking_a_frame_or_checking_focus() {
    let (script, runner) = scripted_run();
    script.set_mount(MountState::Tracking { steady: false });
    start(&runner).await;
    let view = runner.view().unwrap();
    assert_eq!(view.state, "tracking");
    assert_eq!(
        view.message,
        "Tracking M42. Waiting for the mount to settle; nothing to do."
    );
    assert_eq!(calls(&script, "camera.take"), 0);
    assert_eq!(calls(&script, "focus.check"), 0);
    script.set_mount(MountState::Tracking { steady: true });
    settle().await;
    assert_eq!(runner.view().unwrap().state, "stacking");
}

#[tokio::test(start_paused = true)]
async fn preserved_stop_in_each_state_cleans_up_once_and_freezes_elapsed_time() {
    for (state, held) in [
        ("slewing", None),
        ("first_frame", Some(Step::Take)),
        ("focus_check", Some(Step::Check)),
        ("solving", Some(Step::Solve)),
        ("stacking", None),
    ] {
        let (script, runner) = scripted_run();
        if state == "slewing" {
            script.set_mount(MountState::Slewing);
        }
        if let Some(step) = held {
            script.hold(step);
        }
        start(&runner).await;
        assert_eq!(runner.view().unwrap().state, state);
        let view = runner.stop().await.unwrap();
        let slewing = state == "slewing";
        assert_ended(
            &view,
            "stopped",
            if slewing {
                "Stopped: the mount was stopped."
            } else {
                "Imaging run stopped. Track is still on."
            },
        );
        let ended = view.ended.unwrap();
        assert_eq!(ended.at, 1768528800.0);
        assert_eq!(ended.saved, state == "stacking");
        // The script's stop only lists the call; its mount remains as configured.
        assert!(ended.track_on);
        assert_eq!(
            calls(&script, "camera.abort"),
            usize::from(state == "first_frame")
        );
        assert_eq!(
            calls(&script, "focus.close_aid"),
            usize::from(state == "focus_check")
        );
        assert_eq!(
            calls(&script, "stack.stop keep"),
            usize::from(state == "stacking")
        );
        assert_eq!(
            calls(&script, "stack.stop"),
            usize::from(state == "stacking")
        );
        assert_eq!(calls(&script, "mount.stop"), usize::from(slewing));
        assert_eq!(calls(&script, "mount.repoint"), 0);
        let recorded = script.calls();
        assert_eq!(recorded.last().unwrap(), "camera.release");
        if slewing {
            assert_eq!(
                &recorded[recorded.len() - 2..],
                ["mount.stop", "camera.release"]
            );
        }
        tokio::time::sleep(Duration::from_secs(10)).await;
        assert_eq!(runner.view().unwrap().elapsed_s, view.elapsed_s);
        assert_eq!(script.calls(), recorded);
        if state == "stacking" {
            script.set_mount(MountState::Ended(Ending::Stopped));
            assert!(!runner.view().unwrap().ended.unwrap().track_on);
            script.set_mount(MountState::Tracking { steady: true });
            assert!(!runner.view().unwrap().ended.unwrap().track_on);
        }
    }
}

#[tokio::test(start_paused = true)]
async fn preserved_run_stop_finding_track_off_says_the_mount_was_stopped() {
    let (script, runner) = scripted_run();
    script.hold(Step::Take);
    start(&runner).await;
    assert_eq!(runner.view().unwrap().state, "first_frame");
    script.set_mount(MountState::Ended(Ending::Stopped));
    let view = runner.stop().await.unwrap();
    assert_ended(&view, "stopped", "Stopped: the mount was stopped.");
    assert!(!view.ended.unwrap().track_on);
    assert_eq!(calls(&script, "camera.abort"), 1);
    assert_eq!(calls(&script, "mount.stop"), 0);
    assert_eq!(script.calls().last().unwrap(), "camera.release");
}

#[tokio::test(start_paused = true)]
async fn preserved_seven_mount_endings_stop_the_stack_and_keep_their_words() {
    for (ending, reason, message, track_on) in [
        (Ending::Stopped, "stopped", "Stopped: the mount was stopped.", false),
        (Ending::TooLow, "too_low", "Ended: M42 is below the minimum altitude.", false),
        (Ending::SunUp, "sun_up", "Ended: the Sun is up.", false),
        (Ending::LinkLost, "link_lost", "Ended: the mount or the scope link stopped answering. Check the scope node and the mount's cable, then start again.", false),
        (Ending::MountFailed("failed after 5 corrections".into()), "mount_failed", "Ended: Track ended and the run with it: failed after 5 corrections. Check the mount, then start again.", false),
        (Ending::TargetChanged, "target_changed", "Ended: the target was changed. Start a new imaging run for the new target.", true),
        (Ending::PushedAway, "pushed_away", "Ended: the telescope was pushed away from M42. Track follows where it is left. Start a new imaging run to go back to M42.", true),
    ] {
        let (script, runner) = scripted_run();
        start(&runner).await;
        assert_eq!(runner.view().unwrap().state, "stacking");
        script.set_mount(MountState::Ended(ending));
        settle().await;
        let view = runner.view().unwrap();
        assert_ended(&view, reason, message);
        let ended = view.ended.unwrap();
        assert!(ended.saved);
        assert_eq!(ended.track_on, track_on);
        assert_eq!(calls(&script, "stack.stop"), 1);
        let recorded = script.calls();
        assert_eq!(&recorded[recorded.len() - 2..], ["stack.stop keep", "camera.release"]);
    }
}

#[tokio::test(start_paused = true)]
async fn preserved_mount_endings_interrupt_a_take_and_a_slew_wait() {
    for (taking, ending, reason, message) in [
        (
            true,
            Ending::TooLow,
            "too_low",
            "Ended: M42 is below the minimum altitude.",
        ),
        (false, Ending::SunUp, "sun_up", "Ended: the Sun is up."),
    ] {
        let (script, runner) = scripted_run();
        if taking {
            script.hold(Step::Take);
        } else {
            script.set_mount(MountState::Slewing);
        }
        start(&runner).await;
        assert_eq!(
            runner.view().unwrap().state,
            if taking { "first_frame" } else { "slewing" }
        );
        script.set_mount(MountState::Ended(ending));
        settle().await;
        let view = runner.view().unwrap();
        assert_ended(&view, reason, message);
        assert!(!view.ended.unwrap().saved);
        assert_eq!(calls(&script, "camera.abort"), usize::from(taking));
        assert_eq!(calls(&script, "mount.stop"), 0);
        assert_eq!(script.calls().last().unwrap(), "camera.release");
    }
}

#[tokio::test(start_paused = true)]
async fn preserved_stacking_judges_the_mounts_ending_before_a_simultaneous_stop() {
    let (script, runner) = scripted_run();
    start(&runner).await;
    script.set_mount(MountState::Ended(Ending::TooLow));
    let view = runner.stop().await.unwrap();
    assert_ended(
        &view,
        "too_low",
        "Ended: M42 is below the minimum altitude.",
    );
    assert!(view.ended.unwrap().saved);
}

#[tokio::test(start_paused = true)]
async fn preserved_dropping_the_only_runner_releases_its_parts_without_calls() {
    let (script, runner) = scripted_run();
    start(&runner).await;
    assert_eq!(runner.view().unwrap().state, "stacking");
    assert_eq!(Arc::strong_count(&runner), 1);
    let recorded = script.calls();
    drop(runner);
    settle().await;
    assert_eq!(Arc::strong_count(&script), 1);
    assert_eq!(script.calls(), recorded);
    tokio::time::sleep(Duration::from_secs(10)).await;
    assert_eq!(script.calls(), recorded);
}

#[tokio::test(start_paused = true)]
async fn preserved_start_refusals_stop_at_the_first_part_and_release_only_a_taken_lease() {
    let all_calls = [
        "camera.ready",
        "solver.ready",
        "mount.target",
        "camera.hold",
        "mount.point page",
        "camera.release",
    ];
    for (part, words, count) in [
        (Part::Camera, "the scope node is not connected", 1),
        (Part::Solver, "no solver configured", 2),
        (Part::Target, "choose a target first", 3),
        (Part::Hold, "the imaging camera is held by the focus aid", 4),
        (Part::Point, "Track cannot start", 6),
    ] {
        let (script, runner) = scripted_run();
        script.refuse(part, words);
        let refusal = runner
            .start(RunRequest::default(), By::Page)
            .await
            .unwrap_err();
        assert_eq!(refusal.status.as_u16(), 409);
        assert_eq!(refusal.message, words);
        assert_eq!(script.calls(), all_calls[..count]);
        assert!(runner.view().is_none());
    }
}

#[tokio::test(start_paused = true)]
async fn preserved_invalid_limits_refuse_before_calling_any_part() {
    for subs in [0, 100001] {
        let (script, runner) = scripted_run();
        let refusal = runner
            .start(
                RunRequest {
                    subs: Some(subs),
                    ..RunRequest::default()
                },
                By::Page,
            )
            .await
            .unwrap_err();
        assert_eq!(refusal.status.as_u16(), 400);
        assert_eq!(
            refusal.message,
            "subs must be a whole number from 1 to 100000"
        );
        assert!(script.calls().is_empty());
        assert!(runner.view().is_none());
    }
    for duration in [0.5, 43200.5, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let (script, runner) = scripted_run();
        let refusal = runner
            .start(
                RunRequest {
                    duration_s: Some(duration),
                    ..RunRequest::default()
                },
                By::Page,
            )
            .await
            .unwrap_err();
        assert_eq!(refusal.status.as_u16(), 400);
        assert_eq!(
            refusal.message,
            "duration_s must be a number from 1 to 43200"
        );
        assert!(script.calls().is_empty());
        assert!(runner.view().is_none());
    }
}

#[tokio::test(start_paused = true)]
async fn preserved_second_start_refuses_until_the_run_has_ended() {
    let (script, runner) = scripted_run();
    start(&runner).await;
    let recorded = script.calls();
    let refusal = runner
        .start(RunRequest::default(), By::Page)
        .await
        .unwrap_err();
    assert_eq!(refusal.status.as_u16(), 409);
    assert_eq!(refusal.message, "an imaging run is already running");
    assert_eq!(script.calls(), recorded);
    runner.stop().await.unwrap();
    let next = runner.start(RunRequest::default(), By::Mcp).await.unwrap();
    assert!(next.ended.is_none());
    assert_eq!(next.state, "slewing");
    assert_eq!(next.by, "mcp");
    assert_eq!(runner.view().unwrap(), next);
    settle().await;
    assert_eq!(runner.view().unwrap().state, "stacking");
}
