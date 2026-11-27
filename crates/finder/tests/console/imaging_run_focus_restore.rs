//! 054 US1: going on from a focus pause gives the camera back whole, and when
//! the camera will not take its settings back the console says so, rather than
//! clearing a region the observer never set. The rig is 033's focus rig
//! (`imaging_run_focus.rs`) with a simulated imaging camera that refuses the
//! settings for `settle` after each abort of an exposure (054). Oracles: the
//! words of the scope node's refusal and of 041's region refusal, written out
//! here; the windows of the scenarios on the paused tokio clock; the simulator's
//! own settings and frames; and the MCP tools, read through their own door.
//! Time is paused throughout.
use std::{fs, path::Path, sync::Arc, time::Duration};

use axum::{http::StatusCode, Router};
use finder::{
    imaging::{
        run::{console, script::Script, Runner},
        simulated::SimulatedSolver,
    },
    link::Transport,
    samples::Samples,
    session::Session,
    AppState,
};
use scope_link::memory;
use serde_json::{json, Value};
use simulator::{SimCamera, SimImagingCamera, SimNode, Simulator, Site, VirtualClock};
use tokio::time::Instant;

use crate::support::{live as support, mcp, Scratch};

const VIEW: &str = "/api/imaging";
const RUN: &str = "/api/imaging/run";
const CONTINUE: &str = "/api/imaging/run/continue";
const FOCUS: &str = "/api/imaging/focus";
const FRAME: &str = "/api/imaging/frame";
const SETTINGS: &str = "/api/imaging/settings";
/// The scope node's words for a settings change its camera refuses while it
/// is still stopping an exposure (054), as the console passes them on.
const REFUSED: &str = "the scope node did not apply the settings: the camera refused the settings: it is still stopping an exposure";
/// 041's region refusal, naming the region an aid could not put back (054).
const NAMED: &str = "the focus aid could not put the imaging camera's settings back: the scope node did not apply the settings: the camera refused the settings: it is still stopping an exposure";
/// 041's region refusal, for a region no aid left.
const REGION: &str =
    "the imaging camera has a region of interest set: clear it before an imaging run";

struct Rig {
    app: Router,
    mcp: Router,
    sim: Simulator,
    runner: Arc<Runner>,
    _script: Arc<Script>,
}

/// 033's focus rig: the small simulated imaging camera pointed at the pole,
/// no mount connected, paired console, the simulated imaging solver and the
/// console's own camera and focus parts, with a fresh scratch folder (so no
/// focus record). The camera refuses the settings for `settle` after each
/// abort (054). The rig's node is served over a fresh duplex for each
/// connection, and the same console answers the MCP door.
fn focus_rig(scratch: &Scratch, settle: Duration) -> Rig {
    let sim = Simulator::new(
        support::DB.clone(),
        Site::default(),
        VirtualClock::new(support::START),
    );
    sim.push_to(40.0, 0.0);
    sim.set_focus_error(0.0);
    let imaging = SimImagingCamera::new(sim.clone()).with_abort_settle(settle);
    let node = Arc::new(
        SimNode::from_cameras(
            sim.clone(),
            SimCamera::new(sim.clone()),
            imaging,
            "test-token",
            Default::default(),
        )
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
    let state = AppState::shared(support::DB.clone(), Samples::default())
        .with_session(session)
        .with_clock(Arc::new(support::ConsoleClock(sim.clock().clone())))
        .with_link(transport)
        .with_live()
        .with_imaging(scratch.0.join("imaging"))
        .with_focal_length_mm(1800.0)
        .with_imaging_solver(Arc::new(
            SimulatedSolver::new(sim.clone()).with_measure_noise(7, 0.02),
        ));
    let script = Script::new();
    let mut steps = script.steps();
    steps.camera = console::camera(&state);
    steps.focus = console::focus(&state);
    let state = state.with_imaging_run_steps(steps);
    let runner = state.imaging_runner().unwrap();
    Rig {
        app: finder::router(state.clone()),
        mcp: mcp::app(state, "127.0.0.1"),
        sim,
        runner,
        _script: script,
    }
}

async fn get(app: &Router) -> Value {
    let (status, view) = support::request(app, "GET", VIEW, Value::Null).await;
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

async fn refused(app: &Router, method: &str, path: &str, body: Value) -> (StatusCode, Value) {
    support::request(app, method, path, body).await
}

async fn ready(app: &Router) {
    wait(app, |v| v["state"] == "ready").await;
}

/// The settings in force are the whole sensor: x 0, y 0, the camera's width
/// and height (041's rule).
fn whole(view: &Value) -> bool {
    let roi = &view["settings"]["roi"];
    let camera = &view["camera"];
    roi["x"] == 0
        && roi["y"] == 0
        && roi["width"] == camera["width"]
        && roi["height"] == camera["height"]
}

/// The run has paused at its focus check, and the aid it opened is on an
/// exposure of its own.
fn paused_mid_exposure(view: &Value) -> bool {
    view["run"]["state"] == "focus_pause"
        && view["focus"]["aid"]["state"] == "running"
        && view["frame"]["state"] == "exposing"
        && view["frame"]["purpose"] == "focus"
}

/// The aid alone, on tile (1, 1), exposing for thirty seconds, so an abort
/// lands on an exposure in flight.
async fn aid_exposing(app: &Router) -> Value {
    ready(app).await;
    ask(
        app,
        "POST",
        FOCUS,
        json!({"tile":{"col":1,"row":1},"exposure_ms":30000}),
    )
    .await;
    wait(app, |v| {
        v["focus"]["aid"]["state"] == "running"
            && v["frame"]["state"] == "exposing"
            && v["frame"]["purpose"] == "focus"
    })
    .await
}

/// A request sent now, answered 200, and the paused time it took to answer.
async fn timed(app: &Router, method: &str, path: &str) -> (Duration, Value) {
    let start = Instant::now();
    let (status, body) = support::request(app, method, path, Value::Null).await;
    let elapsed = start.elapsed();
    assert_eq!(status, StatusCode::OK, "{body}");
    (elapsed, body)
}

fn within(elapsed: Duration, min_ms: u64, max_ms: u64) {
    assert!(
        (Duration::from_millis(min_ms)..=Duration::from_millis(max_ms)).contains(&elapsed),
        "answered after {elapsed:?}, expected {min_ms}..={max_ms} ms"
    );
}

/// One MCP tool's result, called with no arguments.
async fn tool(app: &Router, name: &str) -> Value {
    let response = mcp::send(app, mcp::call(name, json!({}))).await;
    assert_eq!(response.status(), StatusCode::OK);
    let rpc = mcp::body(response).await;
    assert!(rpc.get("error").is_none(), "{rpc}");
    rpc["result"].clone()
}

/// US1-S1: at the short settle, going on anyway mid-exposure gives the camera
/// back whole, and the aid reports no restore error (oracle: the settle of
/// 0.5 s, one retry a second later, and the whole sensor).
#[tokio::test(start_paused = true)]
async fn going_on_anyway_mid_exposure_at_a_short_settle_gives_the_whole_sensor_back() {
    let scratch = Scratch::new("restore-short-anyway");
    let rig = focus_rig(&scratch, Duration::from_millis(500));
    ready(&rig.app).await;
    ask(&rig.app, "POST", RUN, json!({})).await;
    let view = wait(&rig.app, |v| {
        paused_mid_exposure(v) && v["focus"]["aid"]["readings"].as_u64().unwrap_or(0) >= 1
    })
    .await;
    assert_eq!(view["run"]["focus"]["verdict"], "no_record");
    assert!(!whole(&view), "{view}");

    ask(&rig.app, "POST", CONTINUE, json!({"anyway":true})).await;
    let view = wait(&rig.app, |v| v["run"]["state"] == "stacking").await;
    assert!(whole(&view), "{view}");
    assert!(view["focus"]["aid"]["restore_error"].is_null(), "{view}");
    rig.runner.stop().await;
}

/// US1-S2: at the long settle, going on anyway mid-exposure ends the run
/// `camera_failed`, with the region the aid could not put back named, and MCP
/// answers the same words (oracle: 041's refusal and 037's ending words, and
/// the restore words the scope node gives, written out here).
#[tokio::test(start_paused = true)]
async fn going_on_anyway_mid_exposure_at_a_long_settle_ends_the_run_with_the_region_named() {
    let scratch = Scratch::new("restore-long-anyway");
    let rig = focus_rig(&scratch, Duration::from_secs(10));
    ready(&rig.app).await;
    ask(&rig.app, "POST", RUN, json!({})).await;
    wait(&rig.app, paused_mid_exposure).await;

    ask(&rig.app, "POST", CONTINUE, json!({"anyway":true})).await;
    let view = wait(&rig.app, |v| v["run"]["state"] == "ended").await;
    assert_eq!(view["run"]["ended"]["reason"], "camera_failed", "{view}");
    assert_eq!(view["focus"]["aid"]["restore_error"], REFUSED, "{view}");
    assert_eq!(
        view["run"]["ended"]["message"],
        format!("Ended: the imaging camera failed: {NAMED}."),
    );

    let result = tool(&rig.mcp, "get_imaging_run").await;
    assert_eq!(
        result["structuredContent"]["run"]["ended"]["message"],
        format!("Ended: the imaging camera failed: {NAMED}."),
        "{result}"
    );
    let result = tool(&rig.mcp, "get_focus_reading").await;
    assert_eq!(
        result["structuredContent"]["aid"]["restore_error"], REFUSED,
        "{result}"
    );
    rig.runner.stop().await;
}

/// US1-S3: a stop answers once its put-back has had its tries. At the short
/// settle that is 0.9 s to 1.5 s, with the whole sensor back and no restore
/// error. At the long one it is 4.9 s to 6.0 s, with the region as the aid
/// left it, and once the link has dropped and come back the start is refused
/// for the region alone, as 041 words it (oracle: the settles, one retry a
/// second, and the five-second bound).
#[tokio::test(start_paused = true)]
async fn a_stop_answers_once_its_put_back_has_had_its_tries() {
    // The settle, the window of the answer in milliseconds, and whether the
    // camera is still stopping when the put-back is tried.
    let short = (Duration::from_millis(500), 900, 1500);
    let long = (Duration::from_secs(10), 4900, 6000);
    let scratch = Scratch::new("restore-stop-short");
    let rig = focus_rig(&scratch, short.0);
    ready(&rig.app).await;
    let aid = aid_exposing(&rig.app).await;
    assert!(!whole(&aid), "{aid}");
    let (elapsed, answer) = timed(&rig.app, "DELETE", FOCUS).await;
    within(elapsed, short.1, short.2);
    assert_eq!(
        answer["focus"]["aid"]["ended_reason"], "stopped",
        "{answer}"
    );
    assert!(
        answer["focus"]["aid"]["restore_error"].is_null(),
        "{answer}"
    );
    assert!(whole(&answer), "{answer}");
    rig.runner.stop().await;

    let scratch = Scratch::new("restore-stop-long");
    let rig = focus_rig(&scratch, long.0);
    ready(&rig.app).await;
    let aid = aid_exposing(&rig.app).await;
    let roi = aid["settings"]["roi"].clone();
    assert!(!whole(&aid), "{aid}");
    let (elapsed, answer) = timed(&rig.app, "DELETE", FOCUS).await;
    within(elapsed, long.1, long.2);
    assert_eq!(
        answer["focus"]["aid"]["ended_reason"], "stopped",
        "{answer}"
    );
    assert_eq!(answer["focus"]["aid"]["restore_error"], REFUSED, "{answer}");
    assert_eq!(answer["settings"]["roi"], roi, "{answer}");

    // The link drops and comes back: the console describes the camera again,
    // and the region it could not put back is no longer named.
    rig.sim.set_unresponsive(true);
    wait(&rig.app, |v| v["state"] != "ready").await;
    rig.sim.set_unresponsive(false);
    ready(&rig.app).await;
    let view = get(&rig.app).await;
    assert!(!whole(&view), "{view}");
    let (status, answer) = refused(&rig.app, "POST", RUN, json!({})).await;
    assert_eq!(status, StatusCode::CONFLICT, "{answer}");
    assert_eq!(answer["error"], REGION);
    rig.runner.stop().await;
}

/// US1-S4: a region an aid left stays named through a refused settings change
/// and a run's start, is kept by the next check's put-back of the whole sensor
/// until it takes, and the second check ends `done` with the whole sensor
/// (oracle: 20 s of settle from the stop, and the second check at 21 s).
#[tokio::test(start_paused = true)]
async fn a_region_an_aid_left_is_named_until_the_next_check_puts_the_whole_sensor_back() {
    let scratch = Scratch::new("restore-left-check");
    let rig = focus_rig(&scratch, Duration::from_secs(20));
    let aid = aid_exposing(&rig.app).await;
    let roi = aid["settings"]["roi"].clone();
    let sent = Instant::now();
    let (_, answer) = timed(&rig.app, "DELETE", FOCUS).await;
    assert_eq!(answer["focus"]["aid"]["restore_error"], REFUSED, "{answer}");

    ask(&rig.app, "POST", FOCUS, json!({"check":true})).await;
    let view = wait(&rig.app, |v| v["focus"]["aid"]["ended_reason"] == "failed").await;
    assert_eq!(view["focus"]["aid"]["state"], "ended", "{view}");
    assert_eq!(view["settings"]["roi"], roi, "{view}");

    let (status, answer) = refused(&rig.app, "PUT", SETTINGS, json!({"roi":null})).await;
    assert_eq!(status, StatusCode::BAD_GATEWAY, "{answer}");
    let (status, answer) = refused(&rig.app, "POST", RUN, json!({})).await;
    assert_eq!(status, StatusCode::CONFLICT, "{answer}");
    assert_eq!(answer["error"], NAMED);

    tokio::time::sleep_until(sent + Duration::from_secs(21)).await;
    ask(&rig.app, "POST", FOCUS, json!({"check":true})).await;
    let view = wait(&rig.app, |v| {
        v["focus"]["aid"]["state"] == "ended" && v["focus"]["aid"]["ended_reason"] == "done"
    })
    .await;
    assert!(whole(&view), "{view}");
    ask(&rig.app, "POST", RUN, json!({})).await;
    rig.runner.stop().await;
}

/// US1-S5: a region an aid left is named while it stands, a frame taken then
/// shows the region, and the observer's own settings change clears the naming
/// (oracle: the aid's tile at (1, 1) of the 1648 × 1096 sensor, 412 × 274 px,
/// the settle of 10 s, and 041's refusal).
#[tokio::test(start_paused = true)]
async fn a_region_an_aid_left_is_named_until_the_observer_sets_the_settings() {
    let scratch = Scratch::new("restore-left-set");
    let rig = focus_rig(&scratch, Duration::from_secs(10));
    let aid = aid_exposing(&rig.app).await;
    assert_eq!(
        aid["settings"]["roi"],
        json!({"x":412,"y":274,"width":412,"height":274}),
        "{aid}"
    );
    let sent = Instant::now();
    let (_, answer) = timed(&rig.app, "DELETE", FOCUS).await;
    assert_eq!(answer["focus"]["aid"]["restore_error"], REFUSED, "{answer}");

    tokio::time::sleep_until(sent + Duration::from_secs(11)).await;
    let taken = ask(&rig.app, "POST", FRAME, Value::Null).await;
    let id = taken["frame"]["id"].as_u64().unwrap();
    let frame = wait(&rig.app, |v| {
        v["frame"]["id"] == id && v["frame"]["state"] == "saved"
    })
    .await["frame"]
        .clone();
    assert_eq!(frame["width"], 412, "{frame}");
    assert_eq!(frame["height"], 274, "{frame}");

    let (status, answer) = refused(&rig.app, "POST", RUN, json!({})).await;
    assert_eq!(status, StatusCode::CONFLICT, "{answer}");
    assert_eq!(answer["error"], NAMED);

    let region = json!({"roi":{"x":412,"y":274,"width":412,"height":274}});
    ask(&rig.app, "PUT", SETTINGS, region).await;
    let (status, answer) = refused(&rig.app, "POST", RUN, json!({})).await;
    assert_eq!(status, StatusCode::CONFLICT, "{answer}");
    assert_eq!(answer["error"], REGION);
    rig.runner.stop().await;
}

/// US1-S6: a check that follows a failed put-back keeps the region named, so
/// the pause it leads to, and the run's `anyway` after it, end `camera_failed`
/// with the region's words; the second pause keeps the first pause's region
/// (oracle: the settle of 20 s, and the first pause's own view).
#[tokio::test(start_paused = true)]
async fn a_check_after_a_failed_put_back_keeps_the_region_named_through_the_next_pause() {
    let scratch = Scratch::new("restore-left-pause");
    let rig = focus_rig(&scratch, Duration::from_secs(20));
    ready(&rig.app).await;
    ask(&rig.app, "POST", RUN, json!({})).await;
    let first = wait(&rig.app, paused_mid_exposure).await;
    let first_since = first["focus"]["aid"]["since"].as_f64().unwrap();
    let first_roi = first["settings"]["roi"].clone();

    ask(&rig.app, "POST", CONTINUE, json!({"anyway":false})).await;
    let second = wait(&rig.app, |v| {
        v["run"]["state"] == "focus_pause"
            && v["focus"]["aid"]["check"] == false
            && v["focus"]["aid"]["state"] == "ended"
            && v["focus"]["aid"]["since"].as_f64().unwrap() > first_since
    })
    .await;
    assert_eq!(second["settings"]["roi"], first_roi, "{second}");

    ask(&rig.app, "POST", CONTINUE, json!({"anyway":true})).await;
    let view = wait(&rig.app, |v| v["run"]["state"] == "ended").await;
    assert_eq!(view["run"]["ended"]["reason"], "camera_failed", "{view}");
    assert_eq!(
        view["run"]["ended"]["message"],
        format!("Ended: the imaging camera failed: {NAMED}."),
    );
    rig.runner.stop().await;
}

/// US1-S7: the README says what a put-back the scope node refuses does, in
/// `## Focus`, and what the run does with it, in `## An imaging run` (FR-004).
/// Line breaks read as spaces, so a sentence may be wrapped.
#[test]
fn the_readme_says_what_a_put_back_that_fails_does() {
    let text = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("README.md"))
        .expect("finder's committed README must be readable");
    let focus = section(&text, "\n## Focus\n", "\n## ");
    for words in [
        "is tried again once a second for up to 5 s",
        "the next aid or check puts the whole sensor back",
    ] {
        assert!(focus.contains(words), "## Focus lacks {words:?}");
    }
    let run = section(&text, "\n## An imaging run\n", "\n### Field checklist");
    let words = "`the focus aid could not put the imaging camera's settings back:`";
    assert!(run.contains(words), "## An imaging run lacks {words:?}");
}

/// The text after `heading` up to `next`, with each line break read as a space.
fn section(text: &str, heading: &str, next: &str) -> String {
    let (_, body) = text
        .split_once(heading)
        .unwrap_or_else(|| panic!("missing heading {heading:?}"));
    let (body, _) = body
        .split_once(next)
        .unwrap_or_else(|| panic!("no {next:?} after {heading:?}"));
    body.replace('\n', " ")
}
