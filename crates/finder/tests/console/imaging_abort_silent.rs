//! 043 US1: an abort and every stop awaiting it answer on a silent node.
//! Oracles: the scenarios' literal times and busy words, route bodies, ended
//! frames, leases and files on disk. All timing uses injected paused Tokio time;
//! aborts allow 100 ms and stops allow 500 ms for their own work.

use std::{path::Path, sync::Arc, time::Duration};

use axum::{http::StatusCode, Router};
use scope_node::{
    clock::NodeClock,
    imaging::{
        synthetic::SyntheticImagingCamera, Exposure, ImagingCamera, ImagingFrame, ImagingFuture,
        ImagingInfo, ImagingSettings, ImagingStatus,
    },
};
use serde_json::{json, Value};
use simulator::{LightPath, Simulator, VirtualClock};
use tokio::time::{self, Instant};

use crate::{
    calibration_console::{console_rig, request, sim_rig, wait_until, Rig},
    stack_loop::{stand_in_rig, ScriptedSolver},
    support::{live as support, Scratch},
};

const VIEW: &str = "/api/imaging";
const ABORT: &str = "/api/imaging/abort";
const STACK: &str = "/api/imaging/stack";
const FOCUS: &str = "/api/imaging/focus";
const CAL: &str = "/api/imaging/calibration";
const BUSY: &str = "the exposure failed: an exposure is already in progress";

/// Keep every camera operation unchanged except the time taken to abort.
struct SlowAbort {
    camera: SyntheticImagingCamera,
    late: Duration,
}

impl ImagingCamera for SlowAbort {
    fn describe(&self) -> ImagingFuture<'_, ImagingInfo> {
        self.camera.describe()
    }

    fn status(&self) -> ImagingFuture<'_, ImagingStatus> {
        self.camera.status()
    }

    fn apply<'a>(&'a self, settings: &'a ImagingSettings) -> ImagingFuture<'a, ImagingSettings> {
        self.camera.apply(settings)
    }

    fn expose<'a>(&'a self, exposure: &'a Exposure) -> ImagingFuture<'a, ImagingFrame> {
        self.camera.expose(exposure)
    }

    fn abort(&self) -> ImagingFuture<'_, ()> {
        Box::pin(async move {
            time::sleep(self.late).await;
            self.camera.abort().await
        })
    }
}

struct NodeTime(VirtualClock);

impl NodeClock for NodeTime {
    fn now(&self) -> f64 {
        self.0.node_now()
    }
}

async fn get(app: &Router) -> Value {
    let (status, view) = request(app, "GET", VIEW, Value::Null, None).await;
    assert_eq!(status, StatusCode::OK, "{view}");
    view
}

async fn edit(app: &Router, exposure_ms: u64) {
    let (status, view) = request(
        app,
        "PUT",
        "/api/imaging/settings",
        json!({"exposure_ms": exposure_ms}),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{view}");
}

async fn silent_rig(sim: &Simulator, root: &Path) -> Rig {
    let mut rig = console_rig(sim, root);
    rig.state = rig
        .state
        .clone()
        .with_imaging_solver(Arc::new(ScriptedSolver::new()));
    rig.app = finder::router(rig.state.clone());
    wait_until(&rig.app, VIEW, |view| view["state"] == "ready").await;
    edit(&rig.app, 30000).await;
    rig
}

async fn slow_rig(name: &str, late_ms: u64) -> (crate::stack_loop::Rig, u64) {
    let camera = SlowAbort {
        camera: SyntheticImagingCamera::new(Arc::new(NodeTime(VirtualClock::new(support::START)))),
        late: Duration::from_millis(late_ms),
    };
    let rig = stand_in_rig(name, ScriptedSolver::new(), Some(Box::new(camera))).await;
    edit(&rig.app, 30000).await;
    let id = take(&rig.app).await;
    exposing(&rig.app).await;
    time::sleep(Duration::from_secs(1)).await;
    (rig, id)
}

async fn take(app: &Router) -> u64 {
    let (status, view) = request(app, "POST", "/api/imaging/frame", Value::Null, None).await;
    assert_eq!(status, StatusCode::OK, "{view}");
    assert_eq!(view["frame"]["state"], "exposing", "{view}");
    view["frame"]["id"].as_u64().unwrap()
}

async fn exposing(app: &Router) -> Value {
    wait_until(app, VIEW, |view| view["frame"]["state"] == "exposing").await
}

async fn take_until_ended(app: &Router, id: u64) -> Value {
    assert_eq!(take(app).await, id);
    wait_until(app, VIEW, |view| {
        view["frame"]["id"] == id
            && matches!(
                view["frame"]["state"].as_str(),
                Some("saved" | "failed" | "aborted")
            )
    })
    .await["frame"]
        .clone()
}

async fn timed(app: &Router, method: &str, path: &str) -> (Duration, Value) {
    let start = Instant::now();
    let (status, body) = time::timeout(
        Duration::from_secs(60),
        request(app, method, path, Value::Null, None),
    )
    .await
    .expect("the request must answer within 60 s");
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

async fn start_stack(app: &Router) -> String {
    let (status, view) = request(app, "POST", STACK, json!({}), None).await;
    assert_eq!(status, StatusCode::OK, "{view}");
    let id = view["stack"]["id"].as_str().unwrap().to_owned();
    exposing(app).await;
    id
}

/// US1-S1: giving up ends the console's frame, drops its call and stream, and
/// leaves the node's exposure running. A later saved frame proves recovery.
#[tokio::test(start_paused = true)]
async fn silent_abort_ends_the_frame_and_recovers_after_the_nodes_exposure() {
    let scratch = Scratch::new("abort-silent-recovery");
    let root = scratch.0.join("imaging");
    let sim = sim_rig();
    let rig = silent_rig(&sim, &root).await;
    let id = take(&rig.app).await;
    exposing(&rig.app).await;
    time::sleep(Duration::from_secs(1)).await;
    sim.set_unresponsive(true);

    let (elapsed, answer) = timed(&rig.app, "POST", ABORT).await;
    within(elapsed, 5000, 5100);
    assert_eq!(answer, json!({"state":"no_link"}));
    let frame = time::timeout(
        Duration::from_secs(1),
        rig.state.imaging().unwrap().ended(id),
    )
    .await
    .expect("the aborted frame has ended at the answer");
    assert_eq!(frame.id, id);
    assert_eq!(frame.state, "aborted");
    assert_eq!(frame.file, None);

    // The dropped call must have time to reach the silent node's cancellation
    // before its controls are allowed to answer again.
    time::sleep(Duration::from_secs(1)).await;
    assert_eq!(get(&rig.app).await, json!({"state":"no_link"}));
    sim.set_unresponsive(false);
    let resumed = Instant::now();
    let ready = wait_until(&rig.app, VIEW, |view| view["state"] == "ready").await;
    within(resumed.elapsed(), 0, 15000);
    assert_eq!(ready["frame"]["id"], id);
    assert_eq!(ready["frame"]["state"], "aborted");
    assert!(ready["frame"].get("file").is_none(), "{ready}");
    assert_eq!(ready["status"]["exposing"], false);
    assert!(!root.join("frames").exists());

    edit(&rig.app, 100).await;
    let next = take_until_ended(&rig.app, id + 1).await;
    assert_eq!(next["state"], "failed");
    assert_eq!(next["error"], BUSY);
    time::sleep(Duration::from_secs(25)).await;
    let later = take_until_ended(&rig.app, id + 2).await;
    assert_eq!(later["state"], "saved", "{later}");
}

/// US1-S2: an answering node still returns promptly and sends Aborted itself.
#[tokio::test(start_paused = true)]
async fn an_answering_node_aborts_without_waiting_for_the_limit() {
    let scratch = Scratch::new("abort-answering");
    let root = scratch.0.join("imaging");
    let sim = sim_rig();
    let rig = silent_rig(&sim, &root).await;
    let id = take(&rig.app).await;
    exposing(&rig.app).await;
    time::sleep(Duration::from_secs(1)).await;

    let (elapsed, answer) = timed(&rig.app, "POST", ABORT).await;
    within(elapsed, 0, 100);
    assert_eq!(answer["state"], "ready");
    let answered = Instant::now();
    let view = wait_until(&rig.app, VIEW, |view| view["frame"]["state"] == "aborted").await;
    within(answered.elapsed(), 0, 1000);
    assert_eq!(view["frame"]["id"], id);
    assert!(!root.join("frames").exists());
}

/// US1-S3: the first abort owns the task; a second has nothing left to stop.
#[tokio::test(start_paused = true)]
async fn a_second_abort_answers_while_the_first_waits() {
    let scratch = Scratch::new("abort-twice");
    let sim = sim_rig();
    let rig = silent_rig(&sim, &scratch.0.join("imaging")).await;
    take(&rig.app).await;
    exposing(&rig.app).await;
    sim.set_unresponsive(true);
    let app = rig.app.clone();
    let first = tokio::spawn(async move { timed(&app, "POST", ABORT).await });

    time::sleep(Duration::from_secs(3)).await;
    let (elapsed, second) = timed(&rig.app, "POST", ABORT).await;
    within(elapsed, 0, 100);
    assert_eq!(second, json!({"state":"no_link"}));
    let (elapsed, answer) = first.await.unwrap();
    within(elapsed, 5000, 5100);
    assert_eq!(answer, json!({"state":"no_link"}));
}

/// US1-S4: an 8 s abort is dropped at 5 s, with its frame task. Neither can
/// later abort the node's exposure or replace the latest frame with a saved one.
#[tokio::test(start_paused = true)]
async fn a_late_abort_is_dropped_and_its_stream_cannot_save_a_frame() {
    let (rig, id) = slow_rig("abort-too-late", 8000).await;
    let (elapsed, answer) = timed(&rig.app, "POST", ABORT).await;
    within(elapsed, 5000, 5100);
    assert_eq!(answer["state"], "ready");
    assert_eq!(answer["frame"]["id"], id);
    assert_eq!(answer["frame"]["state"], "aborted");

    time::sleep(Duration::from_secs(4)).await;
    edit(&rig.app, 100).await;
    let next = take_until_ended(&rig.app, id + 1).await;
    assert_eq!(next["state"], "failed");
    assert_eq!(next["error"], BUSY);
    time::sleep(Duration::from_secs(30)).await;
    assert_eq!(get(&rig.app).await["frame"]["id"], id + 1);
    assert!(!rig.root.join("frames").exists());
}

/// US1-S5: a 3 s answer is awaited and the next exposure can be saved.
#[tokio::test(start_paused = true)]
async fn an_abort_answered_inside_the_limit_still_stops_the_camera() {
    let (rig, id) = slow_rig("abort-inside-limit", 3000).await;
    let (elapsed, _) = timed(&rig.app, "POST", ABORT).await;
    within(elapsed, 3000, 3100);
    let answered = Instant::now();
    let view = wait_until(&rig.app, VIEW, |view| view["frame"]["state"] == "aborted").await;
    within(answered.elapsed(), 0, 60000);
    assert_eq!(view["frame"]["id"], id);
    edit(&rig.app, 100).await;
    let next = take_until_ended(&rig.app, id + 1).await;
    assert_eq!(next["state"], "saved", "{next}");
}

/// US1-S6: the stack stop awaits the bounded abort, writes its record and
/// releases the camera, even though no sub was taken or stacked.
#[tokio::test(start_paused = true)]
async fn a_stack_stop_on_a_silent_node_saves_its_record_and_releases_the_lease() {
    let scratch = Scratch::new("abort-stack-stop");
    let root = scratch.0.join("imaging");
    let sim = sim_rig();
    let rig = silent_rig(&sim, &root).await;
    let id = start_stack(&rig.app).await;
    time::sleep(Duration::from_secs(1)).await;
    sim.set_unresponsive(true);

    let (elapsed, answer) = timed(&rig.app, "DELETE", STACK).await;
    within(elapsed, 5000, 5500);
    assert_eq!(answer["state"], "no_link");
    assert_eq!(answer["stack"]["id"], id);
    assert_eq!(answer["stack"]["state"], "ended");
    assert_eq!(answer["stack"]["ended_reason"], "stopped");
    assert_eq!(answer["stack"]["taken"], 0);
    assert_eq!(answer["stack"]["stacked"], 0);
    let file = format!("runs/{id}/stack.json");
    assert_eq!(answer["stack"]["files"], json!({"json":file}));
    assert!(root.join(file).is_file());
    assert_eq!(rig.state.imaging().unwrap().holder(), None);
}

/// US1-S7: a stack stopping behind a waiting abort waits only its remaining
/// four seconds, rather than the node's full exposure.
#[tokio::test(start_paused = true)]
async fn a_stack_stop_behind_an_abort_uses_the_same_deadline() {
    let scratch = Scratch::new("abort-stack-already-waiting");
    let sim = sim_rig();
    let rig = silent_rig(&sim, &scratch.0.join("imaging")).await;
    start_stack(&rig.app).await;
    sim.set_unresponsive(true);
    let app = rig.app.clone();
    let abort = tokio::spawn(async move { timed(&app, "POST", ABORT).await });

    time::sleep(Duration::from_secs(1)).await;
    let (elapsed, answer) = timed(&rig.app, "DELETE", STACK).await;
    within(elapsed, 4000, 4500);
    assert_eq!(answer["stack"]["state"], "ended");
    assert_eq!(answer["stack"]["ended_reason"], "stopped");
    let (elapsed, _) = abort.await.unwrap();
    within(elapsed, 5000, 5100);
}

/// US1-S8: focus stops and releases the camera on a silent node; the retained
/// aid reports that its settings could not be restored when the link returns.
#[tokio::test(start_paused = true)]
async fn a_focus_stop_on_a_silent_node_ends_and_reports_the_restore_failure() {
    let scratch = Scratch::new("abort-focus-stop");
    let sim = sim_rig();
    let rig = silent_rig(&sim, &scratch.0.join("imaging")).await;
    let (status, answer) = request(
        &rig.app,
        "POST",
        FOCUS,
        json!({"tile":{"col":1,"row":1},"exposure_ms":30000}),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    exposing(&rig.app).await;
    time::sleep(Duration::from_secs(1)).await;
    sim.set_unresponsive(true);

    let (elapsed, answer) = timed(&rig.app, "DELETE", FOCUS).await;
    within(elapsed, 5000, 5500);
    assert_eq!(answer, json!({"state":"no_link"}));
    let imaging = rig.state.imaging().unwrap();
    assert!(!imaging.focus_running());
    assert_eq!(imaging.holder(), None);
    sim.set_unresponsive(false);
    let ready = wait_until(&rig.app, VIEW, |view| view["state"] == "ready").await;
    assert_eq!(ready["focus"]["aid"]["state"], "ended");
    assert_eq!(ready["focus"]["aid"]["ended_reason"], "stopped");
    assert_eq!(
        ready["focus"]["aid"]["restore_error"],
        "the scope node is not connected"
    );
    assert_eq!(ready["frame"]["state"], "aborted");
}

/// US1-S9: stopping a set of darks during a silent exposure answers in the same
/// bound, publishes stopped and releases the camera within one more second.
#[tokio::test(start_paused = true)]
async fn a_dark_set_stop_on_a_silent_node_releases_the_lease() {
    let scratch = Scratch::new("abort-darks-stop");
    let sim = sim_rig();
    sim.set_light_path(LightPath {
        cover: true,
        ..LightPath::default()
    });
    let rig = silent_rig(&sim, &scratch.0.join("imaging")).await;
    let (status, answer) = request(
        &rig.app,
        "POST",
        "/api/imaging/calibration/darks",
        json!({"count":3}),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let id = answer["set"]["id"].as_str().unwrap();
    wait_until(&rig.app, VIEW, |view| {
        view["calibration"]["set"]["state"] == "frames" && view["status"]["exposing"] == true
    })
    .await;
    time::sleep(Duration::from_secs(1)).await;
    sim.set_unresponsive(true);

    let (elapsed, answer) = timed(&rig.app, "DELETE", &format!("{CAL}/{id}")).await;
    within(elapsed, 5000, 5500);
    assert_eq!(answer["set"]["id"], id);
    let answered = Instant::now();
    let view = wait_until(&rig.app, CAL, |view| view["set"]["state"] == "stopped").await;
    within(answered.elapsed(), 0, 1000);
    assert_eq!(view["set"]["id"], id);
    assert_eq!(rig.state.imaging().unwrap().holder(), None);
}
