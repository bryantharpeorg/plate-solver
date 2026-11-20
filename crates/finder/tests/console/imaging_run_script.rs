//! US13: the script's contract, without a run. Oracles are the literal defaults,
//! call words and control outcomes in US13-S1 and US13-S2. All comparisons are
//! exact, including numerical fields; elapsed waits use Tokio's paused clock.

use std::{collections::BTreeMap, time::Duration};

use finder::imaging::run::{
    script::{Part, Script, Step, STACK_FOLDER},
    By, Centre, Ending, Failure, FocusCheck, FrameRef, MountState, RunTarget, StackProgress, Unit,
    Verdict,
};
use tokio::time::{sleep, timeout};

fn target() -> RunTarget {
    RunTarget {
        id: "NGC 1976".into(),
        name: "M42".into(),
        ra_deg: 83.82,
        dec_deg: -5.39,
    }
}

fn good_focus() -> FocusCheck {
    FocusCheck {
        verdict: Verdict::Good,
        reading: Some(3.0),
        record: Some(3.0),
        unit: Unit::Arcsec,
    }
}

fn centred() -> Centre {
    Centre {
        offset_arcsec: 0.0,
        tolerance_arcsec: 90.0,
        near_arcsec: 450.0,
        aim_moved: true,
    }
}

fn empty_progress(running: bool) -> StackProgress {
    StackProgress {
        running,
        stacked: 0,
        dropped: BTreeMap::new(),
        waiting_for_tracker: false,
        failed: None,
    }
}

#[tokio::test(start_paused = true)]
async fn defaults_and_all_nineteen_call_words_in_order() {
    let script = Script::new();
    let steps = script.steps();
    assert!(script.calls().is_empty());
    assert_eq!(steps.stack.progress(), empty_progress(false));

    assert_eq!(steps.camera.ready(), Ok(()));
    assert_eq!(steps.solver.ready(), Ok(()));
    let chosen = steps.mount.target();
    assert_eq!(chosen, Ok(target()));
    let chosen = chosen.unwrap();
    assert_eq!(steps.camera.hold(), Ok(()));
    assert_eq!(steps.mount.point(By::Page).await, Ok(()));
    assert_eq!(
        steps.camera.take().await,
        Ok(FrameRef {
            id: 1,
            file: "frames/2026-01-16/000001.fits".into(),
        })
    );
    let second = steps.camera.take().await;
    assert_eq!(
        second,
        Ok(FrameRef {
            id: 2,
            file: "frames/2026-01-16/000002.fits".into(),
        })
    );
    steps.camera.abort().await;
    steps.camera.release();
    assert_eq!(steps.focus.check().await, Ok(good_focus()));
    assert_eq!(steps.focus.open_aid().await, Ok(()));
    steps.focus.close_aid().await;
    assert_eq!(
        steps.solver.solve(&second.unwrap(), &chosen).await,
        Ok(centred())
    );
    assert_eq!(steps.mount.repoint().await, Ok(()));
    steps.mount.stop().await;
    assert_eq!(steps.stack.progress(), empty_progress(false));
    assert_eq!(STACK_FOLDER, "runs/2026-01-16T02-00-10Z-M42");
    assert_eq!(steps.stack.start(&chosen).await, Ok(STACK_FOLDER.into()));
    assert_eq!(steps.stack.progress(), empty_progress(true));
    assert_eq!(steps.stack.stop(true).await, Ok(()));
    assert_eq!(steps.stack.progress(), empty_progress(false));
    assert_eq!(steps.mount.point(By::Mcp).await, Ok(()));
    assert_eq!(steps.stack.stop(false).await, Ok(()));

    let expected_calls = [
        "camera.ready",
        "solver.ready",
        "mount.target",
        "camera.hold",
        "mount.point page",
        "camera.take",
        "camera.take",
        "camera.abort",
        "camera.release",
        "focus.check",
        "focus.open_aid",
        "focus.close_aid",
        "solver.solve 2",
        "mount.repoint",
        "mount.stop",
        "stack.start M42",
        "stack.stop keep",
        "mount.point mcp",
        "stack.stop discard",
    ];
    assert_eq!(script.calls(), expected_calls);
    assert_eq!(steps.mount.state(), MountState::Tracking { steady: true });
    assert!(steps.mount.track_on());
    assert!(steps.mount.steady_between(0.0, 10.0));
    assert_eq!(steps.stack.progress(), empty_progress(false));
    assert_eq!(script.calls(), expected_calls, "reads are not calls");
}

#[tokio::test(start_paused = true)]
async fn queued_answers_are_consumed_in_order_without_using_default_frame_numbers() {
    let script = Script::new();
    let steps = script.steps();
    script.push_frame(Err(Failure::Camera("boom".into())));
    script.push_frame(Ok(FrameRef {
        id: 9,
        file: "x.fits".into(),
    }));
    assert_eq!(
        steps.camera.take().await,
        Err(Failure::Camera("boom".into()))
    );
    assert_eq!(
        steps.camera.take().await,
        Ok(FrameRef {
            id: 9,
            file: "x.fits".into(),
        })
    );
    let frame = steps.camera.take().await;
    assert_eq!(
        frame,
        Ok(FrameRef {
            id: 1,
            file: "frames/2026-01-16/000001.fits".into(),
        })
    );

    script.push_focus(Err(Failure::Other("no".into())));
    assert_eq!(steps.focus.check().await, Err(Failure::Other("no".into())));
    assert_eq!(steps.focus.check().await, Ok(good_focus()));
    script.push_solve(Err(Failure::NotSolved("no".into())));
    let frame = frame.unwrap();
    assert_eq!(
        steps.solver.solve(&frame, &target()).await,
        Err(Failure::NotSolved("no".into()))
    );
    assert_eq!(steps.solver.solve(&frame, &target()).await, Ok(centred()));

    // Successful queued checks and solves are also returned unchanged and in order.
    let focus = FocusCheck {
        verdict: Verdict::ClearlyOut,
        reading: Some(8.0),
        record: None,
        unit: Unit::Px,
    };
    script.push_focus(Ok(focus));
    script.push_focus(Err(Failure::Other("second".into())));
    assert_eq!(steps.focus.check().await, Ok(focus));
    assert_eq!(
        steps.focus.check().await,
        Err(Failure::Other("second".into()))
    );
    assert_eq!(steps.focus.check().await, Ok(good_focus()));
    let centre = Centre {
        offset_arcsec: 120.0,
        tolerance_arcsec: 30.0,
        near_arcsec: 300.0,
        aim_moved: false,
    };
    script.push_solve(Ok(centre));
    script.push_solve(Err(Failure::NotSolved("second".into())));
    assert_eq!(steps.solver.solve(&frame, &target()).await, Ok(centre));
    assert_eq!(
        steps.solver.solve(&frame, &target()).await,
        Err(Failure::NotSolved("second".into()))
    );
    assert_eq!(steps.solver.solve(&frame, &target()).await, Ok(centred()));
}

#[tokio::test(start_paused = true)]
async fn each_fail_control_is_consumed_once() {
    let script = Script::new();
    let steps = script.steps();
    script.fail_repoint(Failure::Other("no".into()));
    assert_eq!(
        steps.mount.repoint().await,
        Err(Failure::Other("no".into()))
    );
    assert_eq!(steps.mount.repoint().await, Ok(()));

    let script = Script::new();
    let steps = script.steps();
    script.fail_open_aid(Failure::Other("no".into()));
    assert_eq!(
        steps.focus.open_aid().await,
        Err(Failure::Other("no".into()))
    );
    assert_eq!(steps.focus.open_aid().await, Ok(()));

    let script = Script::new();
    let steps = script.steps();
    script.fail_stack_start(Failure::Other("no".into()));
    assert_eq!(
        steps.stack.start(&target()).await,
        Err(Failure::Other("no".into()))
    );
    assert_eq!(steps.stack.progress(), empty_progress(false));
    assert_eq!(steps.stack.start(&target()).await, Ok(STACK_FOLDER.into()));
    assert_eq!(steps.stack.progress(), empty_progress(true));

    let script = Script::new();
    let steps = script.steps();
    assert_eq!(steps.stack.start(&target()).await, Ok(STACK_FOLDER.into()));
    script.fail_stack_stop(Failure::Other("no".into()));
    assert_eq!(
        steps.stack.stop(true).await,
        Err(Failure::Other("no".into()))
    );
    assert_eq!(steps.stack.progress(), empty_progress(false));
    assert_eq!(steps.stack.stop(true).await, Ok(()));
}

#[tokio::test(start_paused = true)]
async fn all_five_refusals_persist_and_every_refused_call_is_listed() {
    for (part, words, call) in [
        (Part::Camera, "a", "camera.ready"),
        (Part::Solver, "b", "solver.ready"),
        (Part::Target, "c", "mount.target"),
        (Part::Hold, "d", "camera.hold"),
        (Part::Point, "e", "mount.point page"),
    ] {
        let script = Script::new();
        let steps = script.steps();
        script.refuse(part, words);
        for _ in 0..2 {
            let answer = match part {
                Part::Camera => steps.camera.ready(),
                Part::Solver => steps.solver.ready(),
                Part::Target => steps.mount.target().map(|_| ()),
                Part::Hold => steps.camera.hold(),
                Part::Point => steps.mount.point(By::Page).await,
            };
            assert_eq!(answer, Err(words.into()), "{part:?}");
        }
        assert_eq!(script.calls(), [call, call], "{part:?}");
    }
}

#[tokio::test(start_paused = true)]
async fn unsteady_only_changes_the_next_two_unlisted_reads() {
    let script = Script::new();
    let steps = script.steps();
    script.unsteady(2);
    assert!(!steps.mount.steady_between(0.0, 10.0));
    assert!(!steps.mount.steady_between(0.0, 10.0));
    assert!(steps.mount.steady_between(0.0, 10.0));
    assert!(script.calls().is_empty());
}

#[tokio::test(start_paused = true)]
async fn mount_state_and_track_on_follow_the_scripted_mount() {
    let script = Script::new();
    let steps = script.steps();
    script.set_mount(MountState::Slewing);
    assert_eq!(steps.mount.state(), MountState::Slewing);
    assert!(steps.mount.track_on());
    for (state, on) in [
        (MountState::Ended(Ending::Stopped), false),
        (MountState::Ended(Ending::TooLow), false),
        (MountState::Ended(Ending::SunUp), false),
        (MountState::Ended(Ending::LinkLost), false),
        (MountState::Ended(Ending::MountFailed("x".into())), false),
        (MountState::Slewing, true),
        (MountState::Tracking { steady: false }, true),
        (MountState::Ended(Ending::TargetChanged), true),
        (MountState::Ended(Ending::PushedAway), true),
    ] {
        script.set_mount(state.clone());
        assert_eq!(steps.mount.state(), state);
        assert_eq!(steps.mount.track_on(), on, "{state:?}");
    }
    assert!(script.calls().is_empty());
}

#[tokio::test(start_paused = true)]
async fn stack_counts_do_not_end_a_stack_and_end_stack_preserves_counts() {
    let script = Script::new();
    let steps = script.steps();
    assert_eq!(steps.stack.start(&target()).await, Ok(STACK_FOLDER.into()));
    script.set_stack(5, &[("not_solved", 2), ("behind", 0)], true);
    let mut expected = StackProgress {
        running: true,
        stacked: 5,
        dropped: BTreeMap::from([("not_solved".into(), 2)]),
        waiting_for_tracker: true,
        failed: None,
    };
    assert_eq!(steps.stack.progress(), expected);

    script.end_stack(Some(Failure::Camera("gone".into())));
    expected.running = false;
    expected.failed = Some(Failure::Camera("gone".into()));
    assert_eq!(steps.stack.progress(), expected);

    // A later count update must leave both the ending and its failure intact.
    script.set_stack(6, &[("behind", 1)], false);
    expected.stacked = 6;
    expected.dropped = BTreeMap::from([("behind".into(), 1)]);
    expected.waiting_for_tracker = false;
    assert_eq!(steps.stack.progress(), expected);
    script.end_stack(None);
    expected.failed = None;
    assert_eq!(steps.stack.progress(), expected);
    assert_eq!(script.calls(), ["stack.start M42"]);
}

#[tokio::test(start_paused = true)]
async fn held_take_is_listed_before_waiting_and_release_ends_its_hold() {
    let script = Script::new();
    let steps = script.steps();
    script.hold(Step::Take);
    let camera = steps.camera.clone();
    let take = tokio::spawn(async move { camera.take().await });
    sleep(Duration::from_secs(1)).await;
    assert_eq!(script.calls(), ["camera.take"]);
    assert!(!take.is_finished());

    assert_eq!(
        timeout(Duration::from_secs(1), steps.focus.check()).await,
        Ok(Ok(good_focus()))
    );
    sleep(Duration::from_secs(10)).await;
    assert!(!take.is_finished(), "the held take must not time out");
    assert_eq!(script.calls(), ["camera.take", "focus.check"]);

    script.release(Step::Take);
    sleep(Duration::from_secs(1)).await;
    assert!(take.is_finished());
    assert_eq!(
        take.await.unwrap(),
        Ok(FrameRef {
            id: 1,
            file: "frames/2026-01-16/000001.fits".into(),
        })
    );
    assert_eq!(
        timeout(Duration::from_secs(1), steps.camera.take()).await,
        Ok(Ok(FrameRef {
            id: 2,
            file: "frames/2026-01-16/000002.fits".into(),
        }))
    );
    assert_eq!(
        script.calls(),
        ["camera.take", "focus.check", "camera.take"]
    );
}

#[tokio::test(start_paused = true)]
async fn held_stack_stop_and_close_aid_are_listed_before_waiting_and_released_once() {
    for (step, call) in [
        (Step::StackStop, "stack.stop keep"),
        (Step::CloseAid, "focus.close_aid"),
    ] {
        let script = Script::new();
        let steps = script.steps();
        script.hold(step);
        let held_steps = steps.clone();
        let held = tokio::spawn(async move {
            match step {
                Step::StackStop => held_steps.stack.stop(true).await,
                Step::CloseAid => {
                    held_steps.focus.close_aid().await;
                    Ok(())
                }
                _ => unreachable!(),
            }
        });
        sleep(Duration::from_secs(1)).await;
        assert_eq!(script.calls(), [call]);
        assert!(!held.is_finished(), "{step:?}");
        assert_eq!(
            timeout(Duration::from_secs(1), steps.focus.check()).await,
            Ok(Ok(good_focus()))
        );
        sleep(Duration::from_secs(10)).await;
        assert!(!held.is_finished(), "{step:?}");
        assert_eq!(script.calls(), [call, "focus.check"]);

        script.release(step);
        sleep(Duration::from_secs(1)).await;
        assert!(held.is_finished(), "{step:?}");
        assert_eq!(held.await.unwrap(), Ok(()));
        match step {
            Step::StackStop => assert_eq!(
                timeout(Duration::from_secs(1), steps.stack.stop(true)).await,
                Ok(Ok(()))
            ),
            Step::CloseAid => assert_eq!(
                timeout(Duration::from_secs(1), steps.focus.close_aid()).await,
                Ok(())
            ),
            _ => unreachable!(),
        }
        assert_eq!(script.calls(), [call, "focus.check", call]);
    }
}
