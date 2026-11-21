//! US6: exact scripted words and calls come from the scenarios. The rig uses
//! the simulator's focus error and 033's own routes as independent oracles.
//! Time is paused; scripted values and call counts are compared exactly.
use std::{sync::Arc, time::Duration};

use axum::http::StatusCode;
use finder::{
    imaging::run::{
        script::{Script, Step},
        By, Ending, Failure, FocusCheck, MountState, RunRequest, RunView, Runner, Unit, Verdict,
    },
    session::FixedClock,
};
use serde_json::json;

const OUT: FocusCheck = FocusCheck {
    verdict: Verdict::ClearlyOut,
    reading: Some(9.8),
    record: Some(3.1),
    unit: Unit::Arcsec,
};
const OUT_MESSAGE: &str = "Focus is clearly out: stars are 9.8″ across against a record of 3.1″. Turn the focuser while watching the focus aid below, then check focus again or go on anyway.";
const ANYWAY: &str = "Focus was not checked again: the observer went on anyway.";

fn scripted(checks: &[Result<FocusCheck, Failure>]) -> (Arc<Script>, Arc<Runner>) {
    let script = Script::new();
    for check in checks {
        script.push_focus(check.clone());
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

fn count(script: &Script, call: &str) -> usize {
    script.calls().iter().filter(|c| *c == call).count()
}

fn no_pause_deadline(view: &RunView) {
    assert!(serde_json::to_value(view)
        .unwrap()
        .get("pause_ends_at")
        .is_none());
}

fn no_solve(script: &Script) {
    assert!(!script.calls().iter().any(|c| c.starts_with("solver.solve")));
}

#[tokio::test(start_paused = true)]
async fn clearly_out_pauses_before_a_solve_with_a_deadline_and_aid() {
    let (script, runner) = scripted(&[Ok(OUT)]);
    let view = start(&runner).await;
    assert_eq!(view.state, "focus_pause");
    assert_eq!(
        serde_json::to_value(&view.focus).unwrap(),
        json!({
            "verdict":"clearly_out", "reading":9.8, "record":3.1, "unit":"arcsec"
        })
    );
    assert_eq!(view.message, OUT_MESSAGE);
    assert_eq!(view.pause_ends_at, Some(1768529700.0));
    assert!(script.calls().ends_with(&[
        "camera.take".into(),
        "camera.release".into(),
        "focus.check".into(),
        "focus.open_aid".into()
    ]));
    no_solve(&script);
}

#[tokio::test(start_paused = true)]
async fn resume_checks_again_or_goes_on_anyway_after_closing_the_aid() {
    for anyway in [false, true] {
        let (script, runner) = scripted(&[Ok(OUT)]);
        assert_eq!(start(&runner).await.state, "focus_pause");
        let before = script.calls().len();
        runner.resume(anyway).unwrap();
        settle().await;
        let view = runner.view().unwrap();
        assert_eq!(view.state, "stacking");
        no_pause_deadline(&view);
        assert_eq!(view.warnings, if anyway { vec![ANYWAY] } else { vec![] });
        let mut expected = vec!["focus.close_aid"];
        if !anyway {
            expected.push("focus.check");
        }
        expected.extend([
            "camera.hold",
            "camera.take",
            "solver.solve 2",
            "camera.release",
            "stack.start M42",
        ]);
        assert_eq!(&script.calls()[before..], expected);
        assert_eq!(count(&script, "focus.check"), if anyway { 1 } else { 2 });
    }
}

#[tokio::test(start_paused = true)]
async fn pause_expires_after_900_seconds_and_closes_the_aid_once() {
    let (script, runner) = scripted(&[Ok(OUT)]);
    start(&runner).await;
    tokio::time::sleep(Duration::from_secs_f64(898.5)).await;
    assert_eq!(runner.view().unwrap().state, "focus_pause");
    tokio::time::sleep(Duration::from_secs(1)).await;
    let view = runner.view().unwrap();
    assert_eq!(view.state, "ended");
    assert_eq!(view.ended.as_ref().unwrap().reason, "focus_timeout");
    assert_eq!(
        view.message,
        "Ended: focus was left paused for 15 minutes. Start again when ready."
    );
    no_pause_deadline(&view);
    assert_eq!(count(&script, "focus.close_aid"), 1);
    assert_eq!(count(&script, "stack.start M42"), 0);
}

#[tokio::test(start_paused = true)]
async fn stop_mount_ending_and_failed_open_all_close_before_release() {
    for mode in [0, 1, 2] {
        let (script, runner) = scripted(&[Ok(OUT)]);
        if mode == 2 {
            script.fail_open_aid(Failure::Other("the focus aid is busy".into()));
        }
        let view = start(&runner).await;
        assert_eq!(view.state, "focus_pause");
        assert_eq!(
            view.warnings,
            if mode == 2 {
                vec!["The focus aid could not be opened: the focus aid is busy."]
            } else {
                vec![]
            }
        );
        let view = if mode == 1 {
            script.set_mount(MountState::Ended(Ending::TooLow));
            settle().await;
            runner.view().unwrap()
        } else {
            runner.stop().await.unwrap()
        };
        assert_eq!(view.state, "ended");
        assert_eq!(
            view.ended.as_ref().unwrap().reason,
            if mode == 1 { "too_low" } else { "stopped" }
        );
        no_pause_deadline(&view);
        assert_eq!(count(&script, "focus.close_aid"), 1);
        assert!(script
            .calls()
            .ends_with(&["focus.close_aid".into(), "camera.release".into()]));
    }
    let (_, runner) = scripted(&[]);
    assert_eq!(start(&runner).await.state, "stacking");
    let refusal = runner.resume(true).unwrap_err();
    assert_eq!(refusal.status, StatusCode::CONFLICT);
    assert_eq!(refusal.message, "the imaging run is not paused");
}

#[tokio::test(start_paused = true)]
async fn only_unsteady_out_or_unmeasurable_verdicts_repeat_once() {
    for (checks, unsteady, expected_checks, state, warning) in [
        (vec![Ok(OUT)], 1, 2, "stacking", false),
        (vec![Ok(OUT), Ok(OUT)], 2, 2, "focus_pause", true),
        (vec![], 1, 1, "stacking", false),
        (
            vec![Ok(FocusCheck {
                verdict: Verdict::NoRecord,
                reading: Some(3.2),
                record: None,
                unit: Unit::Arcsec,
            })],
            1,
            1,
            "focus_pause",
            false,
        ),
        (
            vec![Ok(FocusCheck {
                verdict: Verdict::Unmeasurable,
                reading: None,
                record: None,
                unit: Unit::Px,
            })],
            1,
            2,
            "stacking",
            false,
        ),
        (
            vec![Err(Failure::NotSolved(
                "the focus check ended: no_region".into(),
            ))],
            1,
            1,
            "focus_pause",
            false,
        ),
    ] {
        let (script, runner) = scripted(&checks);
        script.unsteady(unsteady);
        let view = start(&runner).await;
        assert_eq!(view.state, state);
        assert_eq!(count(&script, "focus.check"), expected_checks);
        assert_eq!(
            count(&script, "focus.open_aid"),
            usize::from(state == "focus_pause")
        );
        if warning {
            assert_eq!(
                view.warnings,
                ["The tracker was not steady during the focus check."]
            );
        } else if checks.first().is_some_and(Result::is_err) {
            assert_eq!(
                view.warnings,
                ["The focus check did not finish: the focus check ended: no_region."]
            );
        } else {
            assert!(view.warnings.is_empty());
        }
        if state == "stacking" {
            assert_eq!(view.focus.as_ref().unwrap().verdict, "good");
        }
        if expected_checks == 2 {
            no_release_between_checks(&script);
        }
    }
}

fn no_release_between_checks(script: &Script) {
    let calls = script.calls();
    let first = calls.iter().position(|c| c == "focus.check").unwrap();
    let last = calls.iter().rposition(|c| c == "focus.check").unwrap();
    assert!(!calls[first..last].iter().any(|c| c == "camera.release"));
}

#[tokio::test(start_paused = true)]
async fn resume_starts_the_unsteady_rule_again_and_keeps_the_lease_given_up() {
    let (script, runner) = scripted(&[Ok(OUT), Ok(OUT), Ok(OUT)]);
    script.unsteady(1);
    assert_eq!(start(&runner).await.state, "focus_pause");
    assert_eq!(count(&script, "focus.check"), 2);
    script.unsteady(1);
    runner.resume(false).unwrap();
    settle().await;
    let view = runner.view().unwrap();
    assert_eq!(view.state, "stacking");
    assert_eq!(count(&script, "focus.check"), 4);
    assert!(view.warnings.is_empty());
    no_release_between_checks(&script);
}

#[tokio::test(start_paused = true)]
async fn a_check_still_out_pauses_again_without_a_second_release() {
    let (script, runner) = scripted(&[Ok(OUT), Ok(OUT)]);
    assert_eq!(start(&runner).await.state, "focus_pause");
    runner.resume(false).unwrap();
    settle().await;
    assert_eq!(runner.view().unwrap().state, "focus_pause");
    assert_eq!(count(&script, "focus.open_aid"), 2);
    assert_eq!(count(&script, "focus.close_aid"), 1);
    no_release_between_checks(&script);
}

#[tokio::test(start_paused = true)]
async fn first_resume_wins_and_a_resume_after_timeout_is_refused_while_closing() {
    let (script, runner) = scripted(&[Ok(OUT)]);
    start(&runner).await;
    runner.resume(false).unwrap();
    let refusal = runner.resume(true).unwrap_err();
    assert_eq!(refusal.status, StatusCode::CONFLICT);
    assert_eq!(refusal.message, "the imaging run is not paused");
    settle().await;
    assert_eq!(runner.view().unwrap().state, "stacking");
    assert_eq!(count(&script, "focus.check"), 2);
    assert!(runner.view().unwrap().warnings.is_empty());

    let (script, runner) = scripted(&[Ok(OUT)]);
    script.hold(Step::CloseAid);
    start(&runner).await;
    tokio::time::sleep(Duration::from_secs(900)).await;
    assert_eq!(count(&script, "focus.close_aid"), 1);
    assert_eq!(runner.view().unwrap().state, "focus_pause");
    let refusal = runner.resume(true).unwrap_err();
    assert_eq!(refusal.status, StatusCode::CONFLICT);
    assert_eq!(refusal.message, "the imaging run is not paused");
    script.release(Step::CloseAid);
    settle().await;
    let view = runner.view().unwrap();
    assert_eq!(view.state, "ended");
    assert_eq!(view.ended.as_ref().unwrap().reason, "focus_timeout");
    no_pause_deadline(&view);
}

#[tokio::test(start_paused = true)]
async fn every_pause_words_its_own_verdict_or_failure() {
    let unmeasurable = "Focus could not be measured: too few stars. Check for cloud or dew, then check focus again or go on anyway.";
    let failed = "the focus check ended: failed: the camera stopped answering";
    for (check, verdict, message, warning) in [
        (Ok(FocusCheck { verdict: Verdict::ClearlyOut, reading: Some(21.0), record: None, unit: Unit::Px }), "clearly_out", "Focus is clearly out: stars are 21.0 px across. Turn the focuser while watching the focus aid below, then check focus again or go on anyway.", None),
        (Ok(FocusCheck { verdict: Verdict::Unmeasurable, reading: None, record: None, unit: Unit::Px }), "unmeasurable", unmeasurable, None),
        (Ok(FocusCheck { verdict: Verdict::NoRecord, reading: Some(3.2), record: None, unit: Unit::Arcsec }), "no_record", "There is no focus record for this camera yet. Focus with the focus aid, then check focus again or go on anyway.", None),
        (Err(Failure::NotSolved("the focus check ended: no_region".into())), "unmeasurable", unmeasurable, Some("The focus check did not finish: the focus check ended: no_region.")),
        (Err(Failure::Other(failed.into())), "unmeasurable", "The focus check did not finish: the focus check ended: failed: the camera stopped answering. Check the imaging camera, then check focus again or go on anyway.", Some("The focus check did not finish: the focus check ended: failed: the camera stopped answering.")),
    ] {
        let (script, runner) = scripted(&[check]);
        let view = start(&runner).await;
        assert_eq!(view.state, "focus_pause");
        assert_eq!(view.message, message);
        assert_eq!(view.focus.as_ref().unwrap().verdict, verdict);
        assert_eq!(view.warnings, warning.into_iter().collect::<Vec<_>>());
        assert_eq!(count(&script, "focus.open_aid"), 1);
        no_solve(&script);
    }
    let (_, runner) = scripted(&[Err(Failure::Other(failed.into())), Ok(OUT)]);
    assert_eq!(start(&runner).await.state, "focus_pause");
    runner.resume(false).unwrap();
    settle().await;
    let view = runner.view().unwrap();
    assert_eq!(view.state, "focus_pause");
    assert_eq!(view.message, OUT_MESSAGE);
}

use axum::Router;
use finder::{
    imaging::{run::console, simulated::SimulatedSolver},
    link::Transport,
    samples::Samples,
    session::Session,
    AppState,
};
use scope_link::memory;
use serde_json::Value;
use simulator::{SimNode, Simulator, Site, VirtualClock};

use crate::support::{live as support, Scratch};

const RUN: &str = "/api/imaging/run";
const FOCUS: &str = "/api/imaging/focus";

struct Rig {
    app: Router,
    sim: Simulator,
    script: Arc<Script>,
    runner: Arc<Runner>,
}

/// 033's focus rig: the small simulated imaging camera pointed at the pole,
/// no mount connected, paired console, and the simulated imaging solver.
/// Only the camera and focus parts are real; the script supplies sky motion,
/// the run's solve and stack. A fresh scratch folder means no focus record.
fn rig(scratch: &Scratch, own_parts: bool, imaging_solver: bool) -> Rig {
    let sim = Simulator::new(
        support::DB.clone(),
        Site::default(),
        VirtualClock::new(support::START),
    );
    sim.push_to(40.0, 0.0);
    sim.set_focus_error(0.0);
    let node = Arc::new(
        SimNode::new(sim.clone(), "test-token", Default::default())
            .unwrap()
            .with_frame_interval(Duration::from_millis(500)),
    );
    let transport = Transport::connector(move || {
        let node = node.clone();
        async move {
            let (client, server) = tokio::io::duplex(64 * 1024);
            tokio::spawn(async move {
                memory::serve_routes_on(server, node.service().routes())
                    .await
                    .unwrap();
            });
            Ok::<_, std::io::Error>(client)
        }
    });
    let session = Session::with_state_file(scratch.0.join("session.json")).unwrap();
    session
        .edit(
            serde_json::from_value(json!({
                "node":{"address":"scope.local:1","token":"test-token"},
                "site":{"lat_deg":40,"lon_deg":-75}
            }))
            .unwrap(),
            support::START,
        )
        .unwrap();
    let mut state = AppState::shared(support::DB.clone(), Samples::default())
        .with_session(session)
        .with_clock(Arc::new(support::ConsoleClock(sim.clock().clone())))
        .with_link(transport)
        .with_live()
        .with_imaging(scratch.0.join("imaging"))
        .with_focal_length_mm(1800.0);
    if imaging_solver {
        state = state.with_imaging_solver(Arc::new(
            SimulatedSolver::new(sim.clone()).with_measure_noise(7, 0.02),
        ));
    }
    let script = Script::new();
    let mut steps = script.steps();
    if own_parts {
        steps.camera = console::camera(&state);
        steps.focus = console::focus(&state);
    }
    let state = state.with_imaging_run_steps(steps);
    let runner = state.imaging_runner().unwrap();
    Rig {
        app: finder::router(state),
        sim,
        script,
        runner,
    }
}

async fn get(app: &Router) -> Value {
    let (status, view) = support::request(app, "GET", "/api/imaging", Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    view
}

async fn wait(app: &Router, check: impl Fn(&Value) -> bool) -> Value {
    for _ in 0..3000 {
        let view = get(app).await;
        if check(&view) {
            return view;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("awaited imaging state not reached: {}", get(app).await);
}

async fn ask(app: &Router, method: &str, path: &str, body: Value) -> Value {
    let (status, view) = support::request(app, method, path, body).await;
    assert_eq!(status, StatusCode::OK, "{view}");
    view
}

async fn record(rig: &Rig) {
    wait(&rig.app, |v| v["state"] == "ready").await;
    ask(&rig.app, "POST", FOCUS, json!({})).await;
    wait(&rig.app, |v| v["focus"]["record"]["tonight"].is_object()).await;
    ask(&rig.app, "DELETE", FOCUS, Value::Null).await;
}

#[tokio::test(start_paused = true)]
async fn continue_route_checks_again_or_goes_on_anyway() {
    for anyway in [false, true] {
        let scratch = Scratch::new("run-focus-route");
        let rig = rig(&scratch, false, true);
        rig.script.push_focus(Ok(OUT));
        wait(&rig.app, |v| v["state"] == "ready").await;
        ask(&rig.app, "POST", RUN, json!({})).await;
        assert_eq!(
            wait(&rig.app, |v| v["run"]["state"] == "focus_pause").await["run"]["state"],
            "focus_pause"
        );
        ask(
            &rig.app,
            "POST",
            "/api/imaging/run/continue",
            json!({"anyway":anyway}),
        )
        .await;
        settle().await;
        let view = get(&rig.app).await;
        assert_eq!(view["run"]["state"], "stacking");
        if anyway {
            assert_eq!(view["run"]["warnings"], json!([ANYWAY]));
        } else {
            assert!(view["run"].get("warnings").is_none());
        }
        assert!(view["run"].get("pause_ends_at").is_none());
        rig.runner.stop().await;
    }
}

#[tokio::test(start_paused = true)]
async fn own_focus_part_passes_pauses_opens_aid_and_checks_after_refocusing() {
    let scratch = Scratch::new("run-own-focus");
    let rig = rig(&scratch, true, true);
    record(&rig).await;
    rig.runner
        .start(RunRequest::default(), By::Page)
        .await
        .unwrap();
    let view = wait(&rig.app, |v| v["run"]["state"] == "stacking").await;
    let focus = &view["run"]["focus"];
    assert_eq!(focus["verdict"], "good");
    assert_eq!(focus["unit"], "arcsec");
    assert!(focus["reading"].as_f64().unwrap() <= 1.10 * focus["record"].as_f64().unwrap());
    // The adapter preserves the check's own reading and reference exactly.
    assert_eq!(focus["reading"], view["focus"]["aid"]["median"]["value"]);
    assert_eq!(focus["record"], view["focus"]["aid"]["reference"]["value"]);
    rig.runner.stop().await;

    let mut clearly_out = false;
    for error in 1..=10 {
        rig.sim.set_focus_error(f64::from(error));
        ask(&rig.app, "POST", FOCUS, json!({"check":true})).await;
        let view = wait(&rig.app, |v| v["focus"]["aid"]["state"] == "ended").await;
        if view["focus"]["aid"]["verdict"] == "clearly_out" {
            clearly_out = true;
            break;
        }
    }
    assert!(clearly_out, "no clearly_out verdict by focus error 10.0");
    rig.runner
        .start(RunRequest::default(), By::Page)
        .await
        .unwrap();
    let view = wait(&rig.app, |v| {
        v["run"]["state"] == "focus_pause" && v["focus"]["aid"]["state"] == "running"
    })
    .await;
    assert_eq!(view["run"]["focus"]["verdict"], "clearly_out");
    assert_eq!(view["focus"]["aid"]["check"], false);
    rig.sim.set_focus_error(0.0);
    rig.runner.resume(false).unwrap();
    let view = wait(&rig.app, |v| v["run"]["state"] == "stacking").await;
    assert_eq!(view["focus"]["aid"]["state"], "ended");
    assert_eq!(view["run"]["focus"]["verdict"], "good");
    rig.runner.stop().await;
}

#[tokio::test(start_paused = true)]
async fn own_camera_refuses_focus_aid_holder_before_pointing_and_leaves_it_running() {
    let scratch = Scratch::new("run-focus-holder");
    let rig = rig(&scratch, true, true);
    record(&rig).await;
    ask(&rig.app, "POST", FOCUS, json!({})).await;
    wait(&rig.app, |v| v["focus"]["aid"]["state"] == "running").await;
    let before = rig.script.calls().len();
    let (status, view) = support::request(&rig.app, "POST", RUN, json!({})).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(view["error"], "the imaging camera is held by the focus aid");
    assert!(!rig.script.calls()[before..]
        .iter()
        .any(|c| c.starts_with("mount.point")));
    assert_eq!(get(&rig.app).await["focus"]["aid"]["state"], "running");
    ask(&rig.app, "DELETE", FOCUS, Value::Null).await;
}

#[tokio::test(start_paused = true)]
async fn own_focus_without_record_pauses_and_without_solver_ends_at_check() {
    for solver in [true, false] {
        let scratch = Scratch::new("run-focus-fresh");
        let rig = rig(&scratch, true, solver);
        wait(&rig.app, |v| v["state"] == "ready").await;
        rig.runner
            .start(RunRequest::default(), By::Page)
            .await
            .unwrap();
        let mut paused = false;
        let view = loop {
            let view = get(&rig.app).await;
            paused |= view["run"]["state"] == "focus_pause";
            if view["run"]["state"] == if solver { "focus_pause" } else { "ended" } {
                break view;
            }
            assert!(rig.sim.clock().now() < support::START + 300.0, "{view}");
            tokio::time::sleep(Duration::from_millis(100)).await;
        };
        if solver {
            assert_eq!(view["run"]["focus"]["verdict"], "no_record");
            rig.runner.stop().await;
        } else {
            assert_eq!(view["run"]["ended"]["reason"], "solver_unavailable");
            assert_eq!(
                view["run"]["ended"]["message"],
                "Ended: the imaging solver cannot run: this console has no imaging solver."
            );
            assert!(!paused);
        }
        no_solve(&rig.script);
    }
}

#[tokio::test(start_paused = true)]
async fn stopping_own_check_from_focus_route_ends_run_without_a_solve() {
    let scratch = Scratch::new("run-focus-check-stop");
    let rig = rig(&scratch, true, true);
    record(&rig).await;
    rig.runner
        .start(RunRequest::default(), By::Page)
        .await
        .unwrap();
    wait(&rig.app, |v| {
        v["run"]["state"] == "focus_check"
            && v["focus"]["aid"]["state"] == "running"
            && v["focus"]["aid"]["check"] == true
    })
    .await;
    let app = rig.app.clone();
    // Do not await DELETE: 033 waits until no aid runs, including a pause's aid.
    let _delete = tokio::spawn(async move { ask(&app, "DELETE", FOCUS, Value::Null).await });
    let view = wait(&rig.app, |v| v["run"]["state"] == "ended").await;
    assert_eq!(view["run"]["ended"]["reason"], "stopped");
    assert_eq!(
        view["run"]["ended"]["message"],
        "Ended: the focus check was stopped."
    );
    assert_eq!(view["focus"]["aid"]["state"], "ended");
    no_solve(&rig.script);
}
