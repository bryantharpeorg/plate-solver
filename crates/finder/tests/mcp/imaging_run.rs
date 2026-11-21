//! US10: the declared tool contract, route refusals, script calls and recording
//! mount are independent oracles. JSON, words and counts are exact; all waits
//! use paused time, including the one-second moving-call rate-limit windows.
use std::{path::PathBuf, sync::Arc, time::Duration};

use axum::{http::StatusCode, Router};
use finder::{
    imaging::run::{
        script::{Part, Script},
        FocusCheck, MountState, Unit, Verdict,
    },
    session::Session,
    AppState,
};
use scope_node::Settings;
use serde_json::{json, Value};
use simulator::{mount::SimMount, SimNode};

use crate::support::{live, mcp::*, Scratch};

const LIMITED: &str = "the rate limit of 2 moving calls a second was reached";
const NO_RUNNER: &str = "this console runs no imaging runs";

struct ConsoleRig {
    app: Router,
    script: Arc<Script>,
    log: PathBuf,
}

fn console(scratch: &Scratch, parts: bool) -> ConsoleRig {
    let sim = live::sim();
    sim.connect_mount(SimMount::new(sim.clock().clone()));
    let node = Arc::new(
        SimNode::new(sim.clone(), "test-token", Settings::default())
            .unwrap()
            .with_frame_interval(Duration::from_millis(500)),
    );
    let session = Session::with_state_file(scratch.0.join("session.json")).unwrap();
    session
        .edit(
            serde_json::from_value(json!({
                "node":{"address":"scope.local:1","token":"test-token"},
                "site":{"lat_deg":40,"lon_deg":-75},"target":"M42"
            }))
            .unwrap(),
            live::START,
        )
        .unwrap();
    let log = scratch.0.join("calls.log");
    let state = AppState::shared(live::DB.clone(), finder::samples::Samples::default())
        .with_session(session)
        .with_clock(Arc::new(live::ConsoleClock(sim.clock().clone())))
        .with_link(live::transport(node))
        .with_live()
        .with_imaging(scratch.0.join("imaging"))
        .with_call_log(std::fs::File::create(&log).unwrap());
    let script = Script::new();
    let state = if parts {
        state.with_imaging_run_steps(script.steps())
    } else {
        state
    };
    ConsoleRig {
        app: app(state, "127.0.0.1"),
        script,
        log,
    }
}

impl ConsoleRig {
    fn log(&self) -> String {
        std::fs::read_to_string(&self.log).unwrap()
    }
}

async fn tool(app: &Router, name: &str, arguments: Value) -> Value {
    let response = send(app, call(name, arguments)).await;
    assert_eq!(response.status(), StatusCode::OK);
    let rpc = body(response).await;
    assert!(rpc.get("error").is_none(), "{rpc}");
    assert!(!rpc.to_string().contains("test-token"), "{rpc}");
    rpc["result"].clone()
}

fn view(result: &Value) -> &Value {
    assert_ne!(result["isError"], true, "{result}");
    let content = &result["structuredContent"];
    assert_eq!(content.as_object().unwrap().len(), 1, "{result}");
    assert!(content.get("run").is_some(), "{result}");
    assert_eq!(
        serde_json::from_str::<Value>(result["content"][0]["text"].as_str().unwrap()).unwrap(),
        content["run"],
    );
    content
}

fn error(result: &Value) -> &str {
    assert_eq!(result["isError"], true, "{result}");
    assert_eq!(result["content"].as_array().unwrap().len(), 1);
    assert_eq!(result["content"][0]["type"], "text");
    result["content"][0]["text"].as_str().unwrap()
}

async fn settle() {
    tokio::time::sleep(Duration::from_secs(1)).await;
}

fn pause(script: &Script) {
    script.push_focus(Ok(FocusCheck {
        verdict: Verdict::ClearlyOut,
        reading: Some(9.8),
        record: Some(3.1),
        unit: Unit::Arcsec,
    }));
}

/// US10-S1: descriptions and schemas are the literal published contract.
#[tokio::test(start_paused = true)]
async fn listing_has_four_tools_with_descriptions_schemas_and_read_only_hint() {
    let scratch = Scratch::new("mcp-run-list");
    let rig = console(&scratch, true);
    let response = send(
        &rig.app,
        request(json!({"jsonrpc":"2.0","id":1,"method":"tools/list"})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let rpc = body(response).await;
    let tools = rpc["result"]["tools"].as_array().unwrap();
    for (name, description, keys) in [
        ("start_imaging_run", "Move a real telescope to the session's target and start an imaging run there, as POST /api/imaging/run does: GoTo, tracking, a first imaging frame, a focus check, centring on the imaging camera's sensor, and stacking until it is stopped or reaches a limit. Set the target first with set_target. Answers at once with the run in state slewing; read get_imaging_run every few seconds until state is ended. In state focus_pause the run waits up to 15 minutes for continue_imaging_run.", vec!["duration_s", "keep_subs", "subs"]),
        ("stop_imaging_run", "End the imaging run and save its stack, as DELETE /api/imaging/run does. It stops the mount only while the run still slews: call stop for that. Answers once the run has ended, which takes the stack's save and, during a re-centre, up to about 7 s more; with no run on it answers the last run unchanged.", vec![]),
        ("continue_imaging_run", "Go on from a focus pause of the imaging run, as POST /api/imaging/run/continue does: with anyway false, focus is checked again; with anyway true, the run goes on unchecked. Only valid in state focus_pause. An agent with nobody at the focuser should ask the observer, or pass anyway true.", vec!["anyway"]),
        ("get_imaging_run", "Read the imaging run: state (focus_pause needs continue_imaging_run; ended is final), ended.reason and ended.message once over, warnings, the focus verdict, the centring offset, counts of subs stacked and dropped by reason, limit_at for when the time limit ends it, and its message for the observer. null before the first run.", vec![]),
    ] {
        let entry = tools.iter().find(|tool| tool["name"] == name).unwrap();
        assert_eq!(entry["description"], description);
        assert_eq!(entry["outputSchema"], json!({"type":"object"}));
        if name == "get_imaging_run" {
            assert_eq!(entry["annotations"]["readOnlyHint"], true);
        } else {
            assert_ne!(entry["annotations"]["readOnlyHint"], true);
        }
        if !keys.is_empty() {
            let schema = &entry["inputSchema"];
            assert_eq!(schema["type"], "object");
            let mut actual: Vec<_> = schema["properties"].as_object().unwrap().keys().map(String::as_str).collect();
            actual.sort_unstable();
            assert_eq!(actual, keys);
            if name == "continue_imaging_run" {
                assert_eq!(schema["required"], json!(["anyway"]));
                assert_eq!(schema["properties"]["anyway"]["type"], "boolean");
            } else {
                assert!(schema.get("required").is_none_or(|keys| keys.as_array().unwrap().is_empty()));
            }
        }
    }
}

/// US10-S2: start attribution and the exact logged view; reads write no line.
#[tokio::test(start_paused = true)]
async fn start_and_read_share_the_run_and_only_start_is_logged() {
    let scratch = Scratch::new("mcp-run-start");
    let rig = console(&scratch, true);
    rig.script.set_mount(MountState::Slewing);
    let result = tool(&rig.app, "get_imaging_run", json!({})).await;
    assert_eq!(view(&result), &json!({"run":null}));
    let result = tool(&rig.app, "start_imaging_run", json!({"subs":5})).await;
    let started = view(&result).clone();
    assert_eq!(started["run"]["by"], "mcp");
    assert_eq!(started["run"]["settings"]["subs"], 5);
    assert_eq!(started["run"]["state"], "slewing");
    assert!(rig
        .script
        .calls()
        .iter()
        .any(|call| call == "mount.point mcp"));
    rig.script.set_mount(MountState::Tracking { steady: true });
    settle().await;
    let result = tool(&rig.app, "get_imaging_run", json!({})).await;
    let read = view(&result).clone();
    assert_eq!(read["run"]["state"], "stacking");
    let mut before = started["run"].as_object().unwrap().clone();
    let mut after = read["run"].as_object().unwrap().clone();
    for key in [
        "elapsed_s",
        "state",
        "message",
        "focus",
        "centring",
        "stacking_since",
        "limit_at",
        "folder",
    ] {
        before.remove(key);
        after.remove(key);
    }
    assert_eq!(before, after);
    let log = rig.log();
    assert_eq!(log.lines().count(), 1);
    let logged = log
        .lines()
        .next()
        .unwrap()
        .strip_prefix("mcp start_imaging_run {\"subs\":5} -> ")
        .unwrap();
    assert_eq!(serde_json::from_str::<Value>(logged).unwrap(), started);
}

/// US10-S3: route and serde refusals, with one second between ordinary starts.
#[tokio::test(start_paused = true)]
async fn arguments_and_run_guards_answer_tool_errors_in_route_words() {
    let scratch = Scratch::new("mcp-run-errors");
    let rig = console(&scratch, true);
    let result = tool(&rig.app, "start_imaging_run", json!({"subs":0})).await;
    assert_eq!(
        error(&result),
        "subs must be a whole number from 1 to 100000"
    );
    let result = tool(&rig.app, "stop_imaging_run", json!({"x":1})).await;
    assert_eq!(error(&result), "stop_imaging_run takes no arguments");
    let result = tool(&rig.app, "continue_imaging_run", json!({"anyway":true})).await;
    assert_eq!(error(&result), "no imaging run is running");
    let result = tool(&rig.app, "continue_imaging_run", json!({})).await;
    assert!(error(&result).starts_with("missing field `anyway`"));
    let result = tool(
        &rig.app,
        "continue_imaging_run",
        json!({"anyway":true,"x":1}),
    )
    .await;
    assert!(error(&result).starts_with("unknown field `x`"));
    settle().await;
    let bad = json!({"exposure_ms":1});
    let result = tool(&rig.app, "start_imaging_run", bad.clone()).await;
    assert!(error(&result).starts_with("unknown field `exposure_ms`"));
    let (status, route) = live::request(&rig.app, "POST", "/api/imaging/run", bad).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(route["error"], error(&result));
    assert!(rig.script.calls().is_empty());
}

/// US10-S3: body parsing precedes the absent runner, including continue's key.
#[tokio::test(start_paused = true)]
async fn without_parts_valid_writes_are_refused_and_read_is_null() {
    let scratch = Scratch::new("mcp-run-no-parts");
    let rig = console(&scratch, false);
    for (name, args) in [
        ("start_imaging_run", json!({})),
        ("stop_imaging_run", json!({})),
        ("continue_imaging_run", json!({"anyway":true})),
    ] {
        let result = tool(&rig.app, name, args).await;
        assert_eq!(error(&result), NO_RUNNER);
    }
    let result = tool(&rig.app, "get_imaging_run", json!({})).await;
    assert_eq!(view(&result), &json!({"run":null}));
    let result = tool(&rig.app, "continue_imaging_run", json!({})).await;
    assert!(error(&result).starts_with("missing field `anyway`"));
    settle().await;
    let result = tool(&rig.app, "start_imaging_run", json!({"exposure_ms":1})).await;
    assert!(error(&result).starts_with("unknown field `exposure_ms`"));
}

/// US10-S3: even argument errors consume moving slots, before body parsing.
#[tokio::test(start_paused = true)]
async fn malformed_starts_take_the_rate_limit_first_and_each_is_logged() {
    let scratch = Scratch::new("mcp-run-bad-rate");
    let rig = console(&scratch, true);
    for _ in 0..2 {
        let result = tool(&rig.app, "start_imaging_run", json!({"exposure_ms":1})).await;
        assert!(error(&result).starts_with("unknown field `exposure_ms`"));
    }
    let result = tool(&rig.app, "start_imaging_run", json!({})).await;
    assert_eq!(error(&result), LIMITED);
    assert!(rig.script.calls().is_empty());
    assert_eq!(rig.log().lines().count(), 3);
    // The other tools neither take a moving slot nor write a call-log line.
    let result = tool(&rig.app, "get_imaging_run", json!({})).await;
    assert_eq!(view(&result), &json!({"run":null}));
    let result = tool(&rig.app, "stop_imaging_run", json!({})).await;
    assert_eq!(view(&result), &json!({"run":null}));
    let result = tool(&rig.app, "continue_imaging_run", json!({"anyway":true})).await;
    assert_eq!(error(&result), "no imaging run is running");
    assert_eq!(rig.log().lines().count(), 3);
}

/// US10-S4: a rejected moving call is counted and logged, like an accepted one.
#[tokio::test(start_paused = true)]
async fn solver_refusals_match_the_route_and_third_start_is_rate_limited() {
    let scratch = Scratch::new("mcp-run-refusal-rate");
    let rig = console(&scratch, true);
    rig.script
        .refuse(Part::Solver, "this console has no imaging solver");
    let (status, route) = live::request(&rig.app, "POST", "/api/imaging/run", json!({})).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(route["error"], "this console has no imaging solver");
    for _ in 0..2 {
        let result = tool(&rig.app, "start_imaging_run", json!({})).await;
        assert_eq!(error(&result), "this console has no imaging solver");
        assert_eq!(route["error"], error(&result));
    }
    let result = tool(&rig.app, "start_imaging_run", json!({})).await;
    assert_eq!(error(&result), LIMITED);
    let log = rig.log();
    assert_eq!(log.lines().count(), 3);
    assert!(log
        .lines()
        .all(|line| line.starts_with("mcp start_imaging_run {} -> error: ")));
}

/// US10-S5: the script's count survives a stop, which writes no call-log line.
#[tokio::test(start_paused = true)]
async fn stop_saves_the_latest_stacked_count_and_returns_the_ended_run() {
    let scratch = Scratch::new("mcp-run-stop");
    let rig = console(&scratch, true);
    view(&tool(&rig.app, "start_imaging_run", json!({})).await);
    settle().await;
    let result = tool(&rig.app, "get_imaging_run", json!({})).await;
    assert_eq!(view(&result)["run"]["state"], "stacking");
    rig.script.set_stack(4, &[], false);
    settle().await;
    let before = rig.log();
    let result = tool(&rig.app, "stop_imaging_run", json!({})).await;
    assert_eq!(view(&result)["run"]["subs"]["stacked"], 4);
    assert_eq!(view(&result)["run"]["state"], "ended");
    assert_eq!(view(&result)["run"]["ended"]["reason"], "stopped");
    assert_eq!(rig.log(), before);
}

/// US10-S6: the recording mount, rather than telemetry, detects an unwanted Stop.
#[tokio::test(start_paused = true)]
async fn direct_axis_move_blocks_start_and_run_stop_never_stops_the_mount() {
    let scratch = Scratch::new("mcp-run-axis");
    let mut rig = sim::Rig::new(sim::START, 50.0, 120.0, true).await;
    rig.sync().await;
    let script = Script::new();
    rig.state = rig
        .state
        .with_imaging(scratch.0.join("imaging"))
        .with_imaging_run_steps(script.steps());
    rig.app = app(rig.state.clone(), "127.0.0.1");
    let result = tool(
        &rig.app,
        "move_axis",
        json!({"axis":"altitude","deg_per_s":0.5,"duration_s":30}),
    )
    .await;
    assert_ne!(result["isError"], true, "{result}");
    let result = tool(&rig.app, "start_imaging_run", json!({})).await;
    assert_eq!(error(&result), "a direct axis move is running");
    assert!(script.calls().is_empty());
    let result = tool(&rig.app, "stop", json!({})).await;
    assert_ne!(result["isError"], true, "{result}");
    for _ in 0..30 {
        rig.tick().await;
    }
    let result = tool(&rig.app, "start_imaging_run", json!({})).await;
    assert_eq!(view(&result)["run"]["by"], "mcp");
    settle().await;
    let result = tool(&rig.app, "get_imaging_run", json!({})).await;
    assert_eq!(view(&result)["run"]["state"], "stacking");
    let from = rig.commands.lock().unwrap().len();
    let result = tool(&rig.app, "stop_imaging_run", json!({})).await;
    assert_eq!(view(&result)["run"]["state"], "ended");
    assert_eq!(view(&result)["run"]["ended"]["reason"], "stopped");
    for _ in 0..10 {
        rig.tick().await;
    }
    assert!(rig.commands.lock().unwrap()[from..]
        .iter()
        .all(|command| command.action != sim::MountCommand::StopAll));
}

/// US10-S7/S8: resume answers the pause being handed over; a settled read sees
/// the go-anyway warning. All four valid calls and the log exclude the token.
#[tokio::test(start_paused = true)]
async fn continue_hands_over_the_pause_then_read_sees_the_warning_without_secrets() {
    let scratch = Scratch::new("mcp-run-continue");
    let rig = console(&scratch, true);
    pause(&rig.script);
    let start = tool(&rig.app, "start_imaging_run", json!({})).await;
    view(&start);
    settle().await;
    let paused = tool(&rig.app, "get_imaging_run", json!({})).await;
    assert_eq!(view(&paused)["run"]["state"], "focus_pause");
    let continued = tool(&rig.app, "continue_imaging_run", json!({"anyway":true})).await;
    assert_eq!(view(&continued)["run"]["state"], "focus_pause");
    settle().await;
    let read = tool(&rig.app, "get_imaging_run", json!({})).await;
    assert_ne!(view(&read)["run"]["state"], "focus_pause");
    assert_eq!(view(&read)["run"]["state"], "stacking");
    assert_eq!(
        view(&read)["run"]["warnings"],
        json!(["Focus was not checked again: the observer went on anyway."])
    );
    let stopped = tool(&rig.app, "stop_imaging_run", json!({})).await;
    assert_eq!(view(&stopped)["run"]["ended"]["reason"], "stopped");
    for answer in [start, paused, continued, read, stopped] {
        assert!(!answer.to_string().contains("test-token"));
    }
    assert_eq!(rig.log().lines().count(), 1);
    assert!(!rig.log().contains("test-token"));
}
