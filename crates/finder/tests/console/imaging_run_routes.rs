//! 037 US3: HTTP and run.json. Oracles are the scenario's exact states,
//! words and records; counts are exact, elapsed time allows 0.5 s.
use std::{path::Path, sync::Arc, time::Duration};

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use finder::{
    imaging::run::{
        script::{Part, Script, Step, STACK_FOLDER},
        Ending, MountState,
    },
    link::Transport,
    session::Session,
    AppState,
};
use scope_link::memory;
use scope_node::Settings;
use serde_json::{json, Value};
use simulator::{mount::SimMount, SimNode, Simulator};
use tower::ServiceExt;

use crate::support::{live as support, Scratch};

const RUN: &str = "/api/imaging/run";
const CONTINUE: &str = "/api/imaging/run/continue";
const START_EVENTS: [&str; 5] = [
    "slewing",
    "first_frame",
    "focus_check",
    "solving",
    "stacking",
];

struct Rig {
    app: Router,
    sim: Simulator,
    script: Arc<Script>,
}

fn rig(scratch: &Scratch, parts: bool) -> Rig {
    let sim = support::sim();
    sim.connect_mount(SimMount::new(sim.clock().clone()));
    sim.push_to(50.0, 120.0);
    let node = Arc::new(
        SimNode::new(sim.clone(), "test-token", Settings::default())
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
    let state = AppState::shared(support::DB.clone(), finder::samples::Samples::default())
        .with_session(session)
        .with_clock(Arc::new(support::ConsoleClock(sim.clock().clone())))
        .with_link(transport)
        .with_live()
        .with_imaging(scratch.0.join("imaging"));
    let script = Script::new();
    let state = if parts {
        state.with_imaging_run_steps(script.steps())
    } else {
        state
    };
    Rig {
        app: finder::router(state),
        sim,
        script,
    }
}

async fn request(app: &Router, method: &str, path: &str, body: Value) -> (StatusCode, Value) {
    send(
        app,
        method,
        path,
        &body.to_string(),
        "application/json",
        None,
    )
    .await
}

async fn send(
    app: &Router,
    method: &str,
    path: &str,
    body: &str,
    content: &str,
    origin: Option<&str>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header("host", "localhost:8080")
        .header("content-type", content);
    if let Some(origin) = origin {
        builder = builder.header("origin", origin);
    }
    let response = app
        .clone()
        .oneshot(builder.body(Body::from(body.to_owned())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).expect("a JSON answer"),
    )
}

async fn get(app: &Router, path: &str) -> Value {
    let (status, view) = request(app, "GET", path, Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    view
}

async fn wait(app: &Router, check: impl Fn(&Value) -> bool) -> Value {
    for _ in 0..9000 {
        let view = get(app, "/api/imaging").await;
        if check(&view) {
            return view;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!(
        "imaging never reached awaited state: {}",
        get(app, "/api/imaging").await
    );
}

async fn settle() {
    tokio::time::sleep(Duration::from_secs(1)).await;
}

async fn start(rig: &Rig) -> Value {
    let (status, view) = request(&rig.app, "POST", RUN, json!({})).await;
    assert_eq!(status, StatusCode::OK, "{view}");
    view
}

async fn stacking(rig: &Rig) -> Value {
    start(rig).await;
    wait(&rig.app, |v| {
        v["run"]["state"] == "stacking" && v["run"].get("folder").is_some()
    })
    .await
}

async fn stop(rig: &Rig) -> Value {
    let (status, view) = request(&rig.app, "DELETE", RUN, Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{view}");
    view
}

fn record(root: &Path, folder: &str) -> Value {
    let path = root.join(folder).join("run.json");
    assert!(!path.with_file_name("run.json.part").exists());
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn events(record: &Value) -> Vec<&str> {
    record["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["state"].as_str().unwrap())
        .collect()
}

fn same_run(actual: &Value, expected: &Value) {
    for (key, value) in expected.as_object().unwrap() {
        if key == "elapsed_s" {
            assert!((actual[key].as_f64().unwrap() - value.as_f64().unwrap()).abs() <= 0.5);
        } else {
            assert_eq!(&actual[key], value, "{key}");
        }
    }
    for key in actual.as_object().unwrap().keys() {
        assert!(expected.get(key).is_some(), "extra {key}");
    }
}

/// US3-S1: no tracking event on a mount steady from the outset.
#[tokio::test(start_paused = true)]
async fn start_session_and_stack_record_agree() {
    let scratch = Scratch::new("run-routes-start");
    let rig = rig(&scratch, true);
    let answer = start(&rig).await;
    assert_eq!(answer["run"]["by"], "page");
    let id = answer["run"]["id"].as_str().unwrap();
    assert!(id.starts_with("2026-01-16T02-") && id.ends_with("-M42"));
    let view = wait(&rig.app, |v| {
        v["run"]["state"] == "stacking" && v["run"].get("folder").is_some()
    })
    .await;
    let run = &view["run"];
    same_run(&get(&rig.app, "/api/session").await["imaging"]["run"], run);
    let saved_session: Value =
        serde_json::from_slice(&std::fs::read(scratch.0.join("session.json")).unwrap()).unwrap();
    assert!(saved_session.get("run").is_none());
    assert!(!saved_session.to_string().contains("\"run\""));
    assert_eq!(run["folder"], STACK_FOLDER);
    let record = record(&scratch.0.join("imaging"), STACK_FOLDER);
    assert_eq!(record["format"], "imaging-run/1");
    assert_eq!(
        record["finder"],
        get(&rig.app, "/api/health").await["version"]
    );
    for key in [
        "id",
        "by",
        "target",
        "started_at",
        "settings",
        "subs",
        "focus",
        "centring",
    ] {
        assert_eq!(record[key], run[key], "{key}");
    }
    let id = run["id"].as_str().unwrap();
    let utc = format!("{}{}", &id[..11], id[11..20].replace('-', ":"));
    assert_eq!(record["started_utc"], utc);
    assert_eq!(record["state"], "stacking");
    assert_eq!(events(&record), START_EVENTS);
    let times: Vec<f64> = record["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["at"].as_f64().unwrap())
        .collect();
    assert_eq!(times[0], run["started_at"].as_f64().unwrap());
    assert!(times.windows(2).all(|w| w[1] >= w[0]));
    stop(&rig).await;
}

/// US3-S2: rejected starts have no run and no runs folder.
#[tokio::test(start_paused = true)]
async fn refusals_leave_no_run_or_folder() {
    let scratch = Scratch::new("run-routes-refusals");
    let rig = rig(&scratch, true);
    for (body, words) in [
        (
            json!({"subs":0}),
            Some("subs must be a whole number from 1 to 100000"),
        ),
        (
            json!({"duration_s":0}),
            Some("duration_s must be a number from 1 to 43200"),
        ),
        (json!({"subs":1.5}), None),
        (json!({"duration_s":"long"}), None),
        (json!({"exposure_ms":1}), None),
        (json!([null, null, null]), None),
        (json!(null), None),
        (json!("start"), None),
    ] {
        let (status, answer) = request(&rig.app, "POST", RUN, body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{answer}");
        assert!(answer["error"].is_string());
        if let Some(words) = words {
            assert_eq!(answer["error"], words);
        }
        assert!(get(&rig.app, "/api/imaging").await.get("run").is_none());
        assert!(!scratch.0.join("imaging/runs").exists());
    }
    for (content, origin, expected) in [
        ("text/plain", None, StatusCode::UNSUPPORTED_MEDIA_TYPE),
        (
            "application/json",
            Some("http://elsewhere.example"),
            StatusCode::FORBIDDEN,
        ),
    ] {
        let (status, answer) = send(&rig.app, "POST", RUN, "{}", content, origin).await;
        assert_eq!(status, expected);
        assert!(answer["error"].is_string());
        assert!(get(&rig.app, "/api/imaging").await.get("run").is_none());
        assert!(!scratch.0.join("imaging/runs").exists());
    }
    rig.script
        .refuse(Part::Solver, "this console has no imaging solver");
    let (status, answer) = request(&rig.app, "POST", RUN, json!({})).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(
        answer,
        json!({"error":"this console has no imaging solver"})
    );
    assert!(!scratch.0.join("imaging/runs").exists());
    assert!(get(&rig.app, "/api/imaging").await.get("run").is_none());
}

#[tokio::test(start_paused = true)]
async fn duplicate_start_and_all_routes_without_parts() {
    let scratch = Scratch::new("run-routes-duplicate");
    let running = rig(&scratch, true);
    stacking(&running).await;
    let (status, answer) = request(&running.app, "POST", RUN, json!({})).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(answer, json!({"error":"an imaging run is already running"}));
    stop(&running).await;
    let scratch = Scratch::new("run-routes-no-parts");
    let rig = rig(&scratch, false);
    for (method, path, body) in [
        ("POST", RUN, json!({})),
        ("DELETE", RUN, Value::Null),
        ("POST", CONTINUE, json!({"anyway":true})),
    ] {
        let (status, answer) = request(&rig.app, method, path, body).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(answer, json!({"error":"this console runs no imaging runs"}));
    }
}

/// US3-S3 and S7: DELETE includes a sub arriving since the last poll.
#[tokio::test(start_paused = true)]
async fn pause_and_stop_are_written_and_record_contains_no_session_secrets() {
    let scratch = Scratch::new("run-routes-stop");
    let rig = rig(&scratch, true);
    let empty = stop(&rig).await;
    assert!(empty.get("run").is_none());
    stacking(&rig).await;
    rig.script.set_stack(4, &[], false);
    settle().await;
    rig.script.set_stack(4, &[], true);
    settle().await;
    let root = scratch.0.join("imaging");
    let paused = record(&root, STACK_FOLDER);
    assert_eq!(paused["state"], "paused_by_tracker");
    assert_eq!(events(&paused).last(), Some(&"paused_by_tracker"));
    rig.script.set_stack(4, &[], false);
    settle().await;
    rig.script.set_stack(5, &[], false);
    let ended = stop(&rig).await;
    assert_eq!(ended["run"]["state"], "ended");
    assert_eq!(ended["run"]["ended"]["reason"], "stopped");
    let record = record(&root, STACK_FOLDER);
    assert_eq!(record["state"], "ended");
    assert_eq!(record["ended"], ended["run"]["ended"]);
    assert_eq!(record["subs"]["stacked"], 5);
    assert_eq!(events(&record).last(), Some(&"ended"));
    same_run(&stop(&rig).await["run"], &ended["run"]);
    let text = std::fs::read_to_string(root.join(STACK_FOLDER).join("run.json")).unwrap();
    for secret in ["test-token", "scope.local", "lat_deg"] {
        assert!(!text.contains(secret));
    }
    for key in [
        "message",
        "elapsed_s",
        "site",
        "stacking_since",
        "limit_at",
        "pause_ends_at",
        "folder",
    ] {
        assert!(record.get(key).is_none(), "{key}");
    }
}

/// US3-S4: strict continue body and the two refusals outside focus_pause.
#[tokio::test(start_paused = true)]
async fn continue_is_checked_before_resume() {
    let scratch = Scratch::new("run-routes-continue");
    let rig = rig(&scratch, true);
    for (running, words) in [
        (false, "no imaging run is running"),
        (true, "the imaging run is not paused"),
    ] {
        if running {
            stacking(&rig).await;
        }
        for anyway in [false, true] {
            let (status, answer) =
                request(&rig.app, "POST", CONTINUE, json!({"anyway":anyway})).await;
            assert_eq!(status, StatusCode::CONFLICT);
            assert_eq!(answer, json!({"error":words}));
        }
        for body in [
            json!({}),
            json!({"anyway":"yes"}),
            json!({"anyway":true,"x":1}),
            json!([true]),
        ] {
            let (status, answer) = request(&rig.app, "POST", CONTINUE, body).await;
            assert_eq!(status, StatusCode::BAD_REQUEST);
            assert!(answer["error"].is_string());
        }
        let (status, answer) = send(
            &rig.app,
            "POST",
            CONTINUE,
            "{\"anyway\":true}",
            "application/json",
            Some("http://elsewhere.example"),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert!(answer["error"].is_string());
    }
    let (status, answer) = send(
        &rig.app,
        "DELETE",
        RUN,
        "",
        "application/json",
        Some("http://elsewhere.example"),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(answer["error"].is_string());
    stop(&rig).await;
}

/// US3-S5: run attachment survives the imaging connection's no_link state.
#[tokio::test(start_paused = true)]
async fn lost_link_keeps_the_ended_run() {
    let scratch = Scratch::new("run-routes-link");
    let rig = rig(&scratch, true);
    wait(&rig.app, |v| v["state"] == "ready").await;
    stacking(&rig).await;
    rig.script.set_mount(MountState::Ended(Ending::LinkLost));
    rig.sim.set_unresponsive(true);
    let view = wait(&rig.app, |v| v["state"] == "no_link").await;
    assert_eq!(view["run"]["state"], "ended");
    assert_eq!(view["run"]["ended"]["reason"], "link_lost");
}

/// US3-S6: identical second-based ids reserve distinct folders on early stop.
#[tokio::test(start_paused = true)]
async fn early_stops_in_one_second_get_distinct_folders() {
    let scratch = Scratch::new("run-routes-collision");
    let rig = rig(&scratch, true);
    rig.script.hold(Step::Take);
    wait(&rig.app, |v| v["state"] == "ready").await;
    let time = get(&rig.app, "/api/session").await["time"]
        .as_f64()
        .unwrap();
    tokio::time::sleep(Duration::from_secs_f64(time.floor() + 1.05 - time)).await;
    let second = get(&rig.app, "/api/session").await["time"]
        .as_f64()
        .unwrap()
        .floor();
    let mut ended = Vec::new();
    for _ in 0..2 {
        start(&rig).await;
        let view = wait(&rig.app, |v| v["run"]["state"] == "first_frame").await;
        assert!(view["run"].get("folder").is_none());
        assert_eq!(view["run"]["subs"]["stacked"], 0);
        ended.push(stop(&rig).await["run"].clone());
    }
    assert_eq!(
        get(&rig.app, "/api/session").await["time"]
            .as_f64()
            .unwrap()
            .floor(),
        second
    );
    assert_eq!(ended[0]["id"], ended[1]["id"]);
    let folder = format!("runs/{}", ended[0]["id"].as_str().unwrap());
    assert_eq!(ended[0]["folder"], folder);
    assert_eq!(ended[1]["folder"], format!("{folder}-2"));
    assert!(!rig
        .script
        .calls()
        .iter()
        .any(|c| c.starts_with("stack.start")));
    for run in ended {
        assert_eq!(run["subs"]["stacked"], 0);
        let record = record(&scratch.0.join("imaging"), run["folder"].as_str().unwrap());
        assert_eq!(record["ended"]["reason"], "stopped");
        assert_eq!(events(&record), ["slewing", "first_frame", "ended"]);
    }
}

/// US3-S8: count refreshes are separated by at least sixty virtual seconds.
#[tokio::test(start_paused = true)]
async fn stacked_counts_refresh_without_events() {
    let scratch = Scratch::new("run-routes-refresh");
    let rig = rig(&scratch, true);
    stacking(&rig).await;
    rig.script.set_stack(3, &[], false);
    for (seconds, expected) in [(61, 3), (30, 3), (31, 4)] {
        tokio::time::sleep(Duration::from_secs(seconds)).await;
        let record = record(&scratch.0.join("imaging"), STACK_FOLDER);
        assert_eq!(record["subs"]["stacked"], expected);
        assert_eq!(record["state"], "stacking");
        assert_eq!(events(&record), START_EVENTS);
        if seconds == 61 {
            rig.script.set_stack(4, &[], false);
        }
    }
    stop(&rig).await;
}

/// US3-S9: only the newest record's ended key determines unfinished_run.
#[tokio::test(start_paused = true)]
async fn unfinished_record_is_read_once_until_an_accepted_start() {
    for unfinished in [true, false] {
        let scratch = Scratch::new("run-routes-unfinished");
        let root = scratch.0.join("imaging");
        let newest = "runs/2026-01-15T03-00-00Z-M31";
        for (folder, body) in [
            (
                "runs/2026-01-14T03-00-00Z-M31",
                json!({"format":"imaging-run/1","ended":{"reason":"stopped"}}),
            ),
            (
                newest,
                if unfinished {
                    json!({"format":"imaging-run/1","id":"2026-01-15T03-00-00Z-M31","state":"stacking"})
                } else {
                    json!({"format":"imaging-run/1","ended":{"reason":"stopped"}})
                },
            ),
        ] {
            if folder != newest || unfinished {
                std::fs::create_dir_all(root.join(folder)).unwrap();
                std::fs::write(root.join(folder).join("run.json"), body.to_string()).unwrap();
            }
        }
        let rig = rig(&scratch, true);
        let view = get(&rig.app, "/api/imaging").await;
        assert!(view.get("run").is_none());
        if unfinished {
            assert_eq!(view["unfinished_run"], newest);
            std::fs::write(root.join(newest).join("run.json"), "{}").unwrap();
            assert_eq!(
                get(&rig.app, "/api/imaging").await["unfinished_run"],
                newest
            );
            let (status, _) = request(&rig.app, "POST", RUN, json!({"subs":0})).await;
            assert_eq!(status, StatusCode::BAD_REQUEST);
            assert_eq!(
                get(&rig.app, "/api/imaging").await["unfinished_run"],
                newest
            );
        } else {
            assert!(view.get("unfinished_run").is_none());
        }
        assert!(start(&rig).await.get("unfinished_run").is_none());
        assert!(get(&rig.app, "/api/imaging")
            .await
            .get("unfinished_run")
            .is_none());
        stop(&rig).await;
    }
}

/// US3-S10: a failed active write ends the run; an ending write only warns.
#[tokio::test(start_paused = true)]
async fn unwritable_record_ends_or_warns_at_the_correct_stage() {
    for early in [false, true] {
        let scratch = Scratch::new("run-routes-unwritable");
        let root = scratch.0.join("imaging");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("runs"), "a file, not a directory").unwrap();
        let rig = rig(&scratch, true);
        if early {
            rig.script.hold(Step::Take);
        }
        start(&rig).await;
        let ended = if early {
            let view = wait(&rig.app, |v| v["run"]["state"] == "first_frame").await;
            assert!(view["run"].get("folder").is_none());
            stop(&rig).await
        } else {
            wait(&rig.app, |v| v["run"]["state"] == "ended").await
        };
        let run = &ended["run"];
        assert_eq!(
            run["ended"]["reason"],
            if early { "stopped" } else { "save_failed" }
        );
        if early {
            assert!(run.get("folder").is_none());
        } else {
            assert!(run["ended"]["message"]
                .as_str()
                .unwrap()
                .starts_with("Ended: the console could not save to its imaging folder:"));
            assert!(rig.script.calls().ends_with(&[
                "stack.start M42".into(),
                "stack.stop keep".into(),
                "camera.release".into()
            ]));
        }
        let warnings = run["warnings"].as_array().unwrap();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0]
            .as_str()
            .unwrap()
            .starts_with("The run's record could not be saved:"));
    }
}

/// US3-S11: only a listed pause transition holds back the next listed one.
#[tokio::test(start_paused = true)]
async fn tracker_transitions_are_listed_at_most_once_per_minute() {
    let scratch = Scratch::new("run-routes-transitions");
    let rig = rig(&scratch, true);
    stacking(&rig).await;
    rig.script.set_stack(0, &[], true);
    settle().await;
    rig.script.set_stack(0, &[], false);
    settle().await;
    assert_eq!(
        get(&rig.app, "/api/imaging").await["run"]["state"],
        "stacking"
    );
    rig.script.set_stack(0, &[], true);
    settle().await;
    assert_eq!(
        get(&rig.app, "/api/imaging").await["run"]["state"],
        "paused_by_tracker"
    );
    let first = record(&scratch.0.join("imaging"), STACK_FOLDER);
    assert_eq!(first["state"], "paused_by_tracker");
    let mut expected = START_EVENTS.to_vec();
    expected.push("paused_by_tracker");
    assert_eq!(events(&first), expected);
    tokio::time::sleep(Duration::from_secs(58)).await;
    rig.script.set_stack(0, &[], false);
    settle().await;
    let second = record(&scratch.0.join("imaging"), STACK_FOLDER);
    expected.push("stacking");
    assert_eq!(events(&second), expected);
    assert_eq!(second["state"], "stacking");
    let entries = second["events"].as_array().unwrap();
    let gap = entries[6]["at"].as_f64().unwrap() - entries[5]["at"].as_f64().unwrap();
    assert!((gap - 61.0).abs() <= 0.25, "transition gap {gap}");
    stop(&rig).await;
}
