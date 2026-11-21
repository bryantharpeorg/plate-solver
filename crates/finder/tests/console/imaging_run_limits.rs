//! US7's oracles are the scenarios' literal states, words and calls. Comparisons
//! are exact; paused time brackets each limit within one second across 250 ms polls.

use std::{sync::Arc, time::Duration};

use finder::{
    imaging::run::{
        script::{Script, Step},
        By, Ending, Failure, MountState, RunRequest, RunView, Runner,
    },
    session::FixedClock,
};

fn scripted() -> (Arc<Script>, Arc<Runner>) {
    let script = Script::new();
    let runner = Runner::new(script.steps(), Arc::new(FixedClock(1768528800.0)));
    (script, runner)
}

async fn settle() {
    tokio::time::sleep(Duration::from_secs(1)).await;
}

async fn start(runner: &Arc<Runner>, request: RunRequest) {
    runner.start(request, By::Page).await.unwrap();
    settle().await;
    assert_eq!(runner.view().unwrap().state, "stacking");
}

fn calls(script: &Script, call: &str) -> usize {
    script.calls().iter().filter(|c| *c == call).count()
}

fn ended(view: &RunView, reason: &str, message: &str) {
    assert_eq!(view.state, "ended");
    let ending = view.ended.as_ref().unwrap();
    assert_eq!(ending.reason, reason);
    assert_eq!(ending.message, message);
    assert_eq!(view.message, message);
    assert!(ending.saved);
}

fn kept_once(script: &Script) {
    assert_eq!(calls(script, "stack.stop keep"), 1);
    assert_eq!(calls(script, "stack.stop discard"), 0);
}

// US7-S1: three taken, two stacked, is below the requested count.
#[tokio::test(start_paused = true)]
async fn only_stacked_subs_count_toward_the_limit() {
    let (script, runner) = scripted();
    start(
        &runner,
        RunRequest {
            subs: Some(3),
            ..RunRequest::default()
        },
    )
    .await;
    script.set_stack(2, &[("not_solved", 1)], false);
    settle().await;
    let view = runner.view().unwrap();
    assert_eq!(view.state, "stacking");
    assert_eq!(view.subs.stacked, 2);
    assert_eq!(view.subs.dropped["not_solved"], 1);
    assert_eq!(
        view.subs.stacked + view.subs.dropped.values().sum::<u64>(),
        3
    );
    assert_eq!(calls(&script, "stack.stop keep"), 0);
    script.set_stack(3, &[("not_solved", 1)], false);
    settle().await;
    ended(
        &runner.view().unwrap(),
        "count_limit",
        "Finished: the 3 subs asked for are stacked.",
    );
    kept_once(&script);
}

// US7-S7: the ending uses the request's choice of whether to keep subs.
#[tokio::test(start_paused = true)]
async fn count_limit_discards_subs_when_requested() {
    let (script, runner) = scripted();
    start(
        &runner,
        RunRequest {
            subs: Some(3),
            keep_subs: Some(false),
            ..RunRequest::default()
        },
    )
    .await;
    script.set_stack(3, &[("not_solved", 1)], false);
    settle().await;
    ended(
        &runner.view().unwrap(),
        "count_limit",
        "Finished: the 3 subs asked for are stacked.",
    );
    assert_eq!(calls(&script, "stack.stop discard"), 1);
    assert_eq!(calls(&script, "stack.stop keep"), 0);
}

// US7-S2 and S8: the 100 s held solve is outside the limit; tracker pauses are in it.
#[tokio::test(start_paused = true)]
async fn duration_starts_with_the_stack_and_includes_tracker_pauses() {
    for paused in [false, true] {
        let (script, runner) = scripted();
        script.hold(Step::Solve);
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
        tokio::time::sleep(Duration::from_secs(100)).await;
        assert_eq!(runner.view().unwrap().state, "solving");
        assert_eq!(calls(&script, "solver.solve 2"), 1);
        script.release(Step::Solve);
        script.set_stack(0, &[], paused);
        settle().await;
        let state = if paused {
            "paused_by_tracker"
        } else {
            "stacking"
        };
        assert_eq!(runner.view().unwrap().state, state);
        tokio::time::sleep(Duration::from_millis(58_600)).await;
        assert_eq!(runner.view().unwrap().state, state);
        assert_eq!(calls(&script, "stack.stop keep"), 0);
        settle().await;
        ended(
            &runner.view().unwrap(),
            "duration_limit",
            "Finished: the 1-minute limit was reached.",
        );
        kept_once(&script);
    }
}

#[tokio::test(start_paused = true)]
async fn the_default_duration_ends_an_otherwise_idle_stack_after_two_hours() {
    let (script, runner) = scripted();
    start(&runner, RunRequest::default()).await;
    tokio::time::sleep(Duration::from_millis(7_198_600)).await;
    assert_eq!(runner.view().unwrap().state, "stacking");
    settle().await;
    ended(
        &runner.view().unwrap(),
        "duration_limit",
        "Finished: the 120-minute limit was reached.",
    );
    kept_once(&script);
}

// US7-S3: the 600 s is since the last stacked sub, and not since the first drop.
#[tokio::test(start_paused = true)]
async fn no_stacked_sub_for_ten_minutes_names_failed_reasons_with_spaces() {
    let (script, runner) = scripted();
    start(&runner, RunRequest::default()).await;
    script.set_stack(4, &[], false);
    settle().await;
    script.set_stack(
        4,
        &[("behind", 30), ("not_solved", 7), ("too_few_stars", 3)],
        false,
    );
    tokio::time::sleep(Duration::from_millis(598_600)).await;
    assert_eq!(runner.view().unwrap().state, "stacking");
    settle().await;
    ended(
        &runner.view().unwrap(),
        "subs_failing",
        "Ended: no sub was stacked for 10 minutes (not solved 7, too few stars 3). Cloud, dew or a tree is the usual cause; check the sky and start again.",
    );
    kept_once(&script);
}

#[tokio::test(start_paused = true)]
async fn failed_reason_words_use_the_rise_since_the_last_stacked_sub() {
    let (script, runner) = scripted();
    start(&runner, RunRequest::default()).await;
    script.set_stack(4, &[("not_solved", 2)], false);
    settle().await;
    script.set_stack(4, &[("not_solved", 9)], false);
    tokio::time::sleep(Duration::from_secs(601)).await;
    ended(
        &runner.view().unwrap(),
        "subs_failing",
        "Ended: no sub was stacked for 10 minutes (not solved 7). Cloud, dew or a tree is the usual cause; check the sky and start again.",
    );
    kept_once(&script);
}

#[tokio::test(start_paused = true)]
async fn a_single_failed_sub_is_enough_after_ten_minutes() {
    let (script, runner) = scripted();
    start(&runner, RunRequest::default()).await;
    script.set_stack(4, &[], false);
    settle().await;
    script.set_stack(4, &[("not_solved", 1)], false);
    tokio::time::sleep(Duration::from_secs(601)).await;
    ended(
        &runner.view().unwrap(),
        "subs_failing",
        "Ended: no sub was stacked for 10 minutes (not solved 1). Cloud, dew or a tree is the usual cause; check the sky and start again.",
    );
    kept_once(&script);
}

// US7-S4: every self-ending stack is stopped once more to save it.
#[tokio::test(start_paused = true)]
async fn stack_failures_and_a_stop_without_failure_end_the_run_and_save() {
    for (failure, reason, message) in [
        (Some(Failure::Camera("no imaging frame header arrived within 152 s".into())), "camera_failed", "Ended: the imaging camera failed: no imaging frame header arrived within 152 s."),
        (Some(Failure::Save("no space left on device".into())), "save_failed", "Ended: the console could not save to its imaging folder: no space left on device. Check the disk."),
        (Some(Failure::SolverUnavailable("the imaging solver has no star database it can read".into())), "solver_unavailable", "Ended: the imaging solver cannot run: the imaging solver has no star database it can read."),
        (Some(Failure::NotSolved("the imaging solver found no solution".into())), "solver_failing", "Ended: 3 imaging frames in a row could not be solved: the imaging solver found no solution. Check for cloud, dew and focus."),
        (Some(Failure::Other("the stack's grid could not be built".into())), "stack_failed", "Ended: the stack stopped: the stack's grid could not be built."),
        (None, "stopped", "Ended: the stack was stopped."),
    ] {
        let (script, runner) = scripted();
        start(&runner, RunRequest::default()).await;
        script.end_stack(failure);
        settle().await;
        ended(&runner.view().unwrap(), reason, message);
        kept_once(&script);
    }
}

#[tokio::test(start_paused = true)]
async fn stack_failure_wins_over_the_count_limit_in_the_same_poll() {
    let (script, runner) = scripted();
    start(
        &runner,
        RunRequest {
            subs: Some(3),
            ..RunRequest::default()
        },
    )
    .await;
    script.set_stack(3, &[], false);
    script.end_stack(Some(Failure::Camera("the camera stopped answering".into())));
    settle().await;
    ended(
        &runner.view().unwrap(),
        "camera_failed",
        "Ended: the imaging camera failed: the camera stopped answering.",
    );
    kept_once(&script);
}

// US7-S5: Track's ending takes precedence when both parts have seen a lost link.
#[tokio::test(start_paused = true)]
async fn mount_ending_wins_over_stack_failure_in_the_same_poll() {
    let (script, runner) = scripted();
    start(&runner, RunRequest::default()).await;
    script.end_stack(Some(Failure::Camera(
        "the scope node is not connected".into(),
    )));
    script.set_mount(MountState::Ended(Ending::LinkLost));
    settle().await;
    ended(
        &runner.view().unwrap(),
        "link_lost",
        "Ended: the mount or the scope link stopped answering. Check the scope node and the mount's cable, then start again.",
    );
    kept_once(&script);
}

#[tokio::test(start_paused = true)]
async fn run_stop_wins_over_stack_failure_in_the_same_poll() {
    let (script, runner) = scripted();
    start(&runner, RunRequest::default()).await;
    script.end_stack(Some(Failure::Camera("the camera stopped answering".into())));
    ended(
        &runner.stop().await.unwrap(),
        "stopped",
        "Imaging run stopped. Track is still on.",
    );
    kept_once(&script);
}

// US7-S6: queue drops say nothing about the sky; tracker pauses stop only quiet time.
#[tokio::test(start_paused = true)]
async fn behind_and_unprocessed_are_not_failed_subs() {
    let (script, runner) = scripted();
    start(&runner, RunRequest::default()).await;
    script.set_stack(4, &[], false);
    settle().await;
    script.set_stack(4, &[("behind", 30), ("unprocessed", 2)], false);
    tokio::time::sleep(Duration::from_secs(700)).await;
    assert_eq!(runner.view().unwrap().state, "stacking");
    assert_eq!(calls(&script, "stack.stop keep"), 0);
}

#[tokio::test(start_paused = true)]
async fn tracker_pauses_do_not_count_as_time_without_a_stacked_sub() {
    let (script, runner) = scripted();
    start(&runner, RunRequest::default()).await;
    script.set_stack(4, &[], false);
    settle().await;
    script.set_stack(4, &[("not_solved", 3)], true);
    tokio::time::sleep(Duration::from_secs(700)).await;
    assert_eq!(runner.view().unwrap().state, "paused_by_tracker");
    script.set_stack(4, &[("not_solved", 3)], false);
    tokio::time::sleep(Duration::from_millis(598_600)).await;
    assert_eq!(runner.view().unwrap().state, "stacking");
    settle().await;
    ended(
        &runner.view().unwrap(),
        "subs_failing",
        "Ended: no sub was stacked for 10 minutes (not solved 3). Cloud, dew or a tree is the usual cause; check the sky and start again.",
    );
    kept_once(&script);
}

// US7-S9: a stacked sub resets both the clock and the reasons' baseline.
#[tokio::test(start_paused = true)]
async fn a_stacked_sub_starts_the_quiet_time_again() {
    let (script, runner) = scripted();
    start(&runner, RunRequest::default()).await;
    script.set_stack(4, &[], false);
    settle().await;
    script.set_stack(4, &[("not_solved", 9)], false);
    tokio::time::sleep(Duration::from_secs(400)).await;
    script.set_stack(5, &[("not_solved", 9)], false);
    settle().await;
    script.set_stack(5, &[("not_solved", 18)], false);
    tokio::time::sleep(Duration::from_secs(400)).await;
    assert_eq!(runner.view().unwrap().state, "stacking");
    assert_eq!(calls(&script, "stack.stop keep"), 0);
}

#[tokio::test(start_paused = true)]
async fn a_stacked_sub_keeps_the_drops_it_saw_in_the_baseline() {
    let (script, runner) = scripted();
    start(&runner, RunRequest::default()).await;
    script.set_stack(4, &[("not_solved", 9)], false);
    settle().await;
    script.set_stack(5, &[("not_solved", 9)], false);
    tokio::time::sleep(Duration::from_secs(700)).await;
    assert_eq!(runner.view().unwrap().state, "stacking");
    assert_eq!(calls(&script, "stack.stop keep"), 0);
}
