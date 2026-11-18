//! Refusing a second imaging frame, aborting one, and asking for the scope
//! node's own copy (031 US5): the frame-in-flight refusals of
//! `POST /api/imaging/frame` and `PUT /api/imaging/settings`, the abort
//! route, and `save_at_node` with `node_file` and `node_save_error` in the
//! view. Oracles: the scenario's words and timings, the stand-in camera's
//! worked file size, and the node's and the console's FITS files, each read
//! with this module's own reader.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use finder::{link::Transport, session::Session, AppState};
use scope_link::memory;
use scope_node::{
    clock::NodeClock,
    imaging::{
        synthetic::SyntheticImagingCamera, Exposure, ImagingCamera, ImagingError, ImagingFrame,
        ImagingFuture, ImagingInfo, ImagingSettings, ImagingStatus,
    },
    service::{NodeControls, NodeService},
    synthetic::SyntheticCamera,
    Settings,
};
use serde_json::{json, Value};
use simulator::{mount::SimMount, Simulator, VirtualClock};
use tower::ServiceExt;

use crate::support::live as support;

/// A node's NodeClock on the simulator's virtual clock, so the stand-in
/// camera's frames follow the rig's time.
struct NodeTime(VirtualClock);
impl NodeClock for NodeTime {
    fn now(&self) -> f64 {
        self.0.node_now()
    }
}

/// The console rig of the shared setup: the sim rig's own node services, a
/// paired session, the imaging folder `root`, and imaging only when `root` is
/// `Some` (US1-S4's console without it).
fn state(sim: &Simulator, transport: Transport, root: Option<&Path>) -> AppState {
    let session = Session::default();
    session
        .edit(
            serde_json::from_value(json!({
                "node": {"address": "scope.local:1", "token": "test-token"},
                "site": {"lat_deg": 40, "lon_deg": -75}
            }))
            .unwrap(),
            support::START,
        )
        .unwrap();
    let mut state = AppState::shared(support::DB.clone(), finder::samples::Samples::default())
        .with_session(session)
        .with_clock(Arc::new(support::ConsoleClock(sim.clock().clone())))
        .with_link(transport)
        .with_live();
    if let Some(root) = root {
        state = state.with_imaging(root);
    }
    state
}

fn sim_rig() -> Simulator {
    let sim = support::sim();
    sim.connect_mount(SimMount::new(sim.clock().clone()));
    sim.push_to(50.0, 120.0);
    sim
}

/// One fresh duplex for each connection, serving the node's own routes.
fn duplex_service(service: NodeService) -> Transport {
    Transport::connector(move || {
        let service = service.clone();
        async move {
            let (client, server) = tokio::io::duplex(64 * 1024);
            tokio::spawn(async move {
                memory::serve_routes_on(server, service.routes())
                    .await
                    .unwrap();
            });
            Ok::<_, std::io::Error>(client)
        }
    })
}

/// The stand-in console rig (the shared setup): the node rig's
/// `SyntheticImagingCamera`, whose clocks read the simulator's virtual clock,
/// so its frames follow the sample rule. `save_dir` names the folder of the
/// node's own copy (US5-S4); `None` is a node with no save folder (US5-S3).
async fn stand_in(root: &Path, save_dir: Option<PathBuf>) -> Router {
    let sim = sim_rig();
    let clock = Arc::new(NodeTime(sim.clock().clone()));
    let mut node = NodeService::with_controls(
        SyntheticCamera::new(clock.clone()),
        clock.clone(),
        "test-token",
        Settings::default(),
        NodeControls::default(),
    )
    .unwrap()
    .with_imaging(SyntheticImagingCamera::new(clock));
    if let Some(dir) = save_dir {
        node = node.with_imaging_save_dir(dir);
    }
    node.controls()
        .set_frame_interval(Duration::from_millis(500));
    finder::router(state(&sim, duplex_service(node), Some(root)))
}

async fn get(app: &Router, path: &str) -> (StatusCode, Value) {
    support::request(app, "GET", path, json!(null)).await
}

async fn put(app: &Router, body: Value) -> (StatusCode, Value) {
    support::request(app, "PUT", "/api/imaging/settings", body).await
}

async fn post_frame(app: &Router) -> (StatusCode, Value) {
    support::request(app, "POST", "/api/imaging/frame", json!(null)).await
}

async fn post_abort(app: &Router) -> (StatusCode, Value) {
    support::request(app, "POST", "/api/imaging/abort", json!(null)).await
}

/// A request the shared `request` helper cannot send: a foreign `Origin`,
/// with a `Host` of localhost on port 8080 as the same-origin rule reads it.
async fn foreign_origin(app: &Router, method: &str, path: &str) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("host", "localhost:8080")
                .header("content-type", "application/json")
                .header("origin", "http://elsewhere.example")
                .body(Body::from(""))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    (status, value)
}

/// Read `GET /api/imaging` every 100 ms until `check` holds, for at most 60 s.
async fn wait_until(app: &Router, check: impl Fn(&Value) -> bool) -> Value {
    for _ in 0..600 {
        let (_, view) = get(app, "/api/imaging").await;
        if check(&view) {
            return view;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let (_, view) = get(app, "/api/imaging").await;
    panic!("imaging never reached the awaited state: {view}");
}

async fn ready(app: &Router) -> Value {
    wait_until(app, |view| view["state"] == "ready").await
}

/// The frame request answers 409 with exactly `text` (US5-S1).
async fn refused(app: &Router, text: &str) {
    let (status, answer) = post_frame(app).await;
    assert_eq!(status, StatusCode::CONFLICT, "{text}: {answer}");
    assert_eq!(answer["error"], json!(text), "{text}: {answer}");
}

/// The words of a camera whose INDI server cannot be reached (US1-S4).
const NO_INDI: &str = "cannot reach the INDI server at 127.0.0.1:7624: refused";

/// A camera of the test's own whose INDI server cannot be reached (US1-S4):
/// only the describe fails, so the console lands in the state `error`.
struct UnreachableIndi {
    inner: SyntheticImagingCamera,
}

impl ImagingCamera for UnreachableIndi {
    fn describe(&self) -> ImagingFuture<'_, ImagingInfo> {
        Box::pin(async { Err(ImagingError::Failed(NO_INDI.into())) })
    }
    fn status(&self) -> ImagingFuture<'_, ImagingStatus> {
        self.inner.status()
    }
    fn apply<'a>(&'a self, edit: &'a ImagingSettings) -> ImagingFuture<'a, ImagingSettings> {
        self.inner.apply(edit)
    }
    fn expose<'a>(&'a self, exposure: &'a Exposure) -> ImagingFuture<'a, ImagingFrame> {
        self.inner.expose(exposure)
    }
    fn abort(&self) -> ImagingFuture<'_, ()> {
        self.inner.abort()
    }
}

/// A saved FITS file's samples, read as the shared setup says and never
/// through the console's or the scope node's FITS code: 80-byte cards up to
/// `END`, the header padded to 2880 bytes, then the data, each 16-bit sample
/// the stored signed value plus `BZERO`.
fn samples(path: &Path) -> Vec<u16> {
    let bytes = std::fs::read(path).unwrap();
    let mut cards = Vec::new();
    let mut offset = 0;
    loop {
        let card = &bytes[offset..offset + 80];
        let keyword = String::from_utf8_lossy(&card[..8]).trim_end().to_string();
        offset += 80;
        if keyword == "END" {
            break;
        }
        cards.push((
            keyword,
            String::from_utf8_lossy(&card[10..]).trim().to_string(),
        ));
    }
    let value = |name: &str| {
        cards
            .iter()
            .find(|(keyword, _)| keyword == name)
            .unwrap_or_else(|| panic!("no {name} card in {}", path.display()))
            .1
            .clone()
    };
    let width: usize = value("NAXIS1").parse().unwrap();
    let height: usize = value("NAXIS2").parse().unwrap();
    assert_eq!(value("BITPIX"), "16", "{}", path.display());
    let data = &bytes[offset.div_ceil(2880) * 2880..];
    (0..width * height)
        .map(|index| {
            let stored = i16::from_be_bytes([data[2 * index], data[2 * index + 1]]);
            (i32::from(stored) + 32_768) as u16
        })
        .collect()
}

/// The stand-in rig's frames folder under R: `frames/<UTC date>/`, the sim
/// rig's start date.
fn frames(root: &Path) -> PathBuf {
    root.join("frames").join("2026-01-16")
}

/// US5-S1: `POST /api/imaging/frame` answers 403 with a foreign `Origin`; a
/// second request during an exposure 409 `an exposure is already in progress`
/// and a settings change then 409 `an exposure is in progress`; and each
/// console of US1-S4 refuses with its state's words.
#[tokio::test(start_paused = true)]
async fn a_second_frame_a_settings_change_and_each_state_are_refused() {
    let scratch = crate::support::Scratch::new("imaging-control-refusals");
    let root = scratch.0.join("imaging");
    let app = stand_in(&root, None).await;
    ready(&app).await;

    let (status, answer) = foreign_origin(&app, "POST", "/api/imaging/frame").await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{answer}");
    assert!(answer["error"].is_string(), "{answer}");

    // A second frame, and a settings change, during an exposure of 10 s.
    put(&app, json!({"exposure_ms":10000})).await;
    let (status, answer) = post_frame(&app).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    assert_eq!(answer["frame"]["state"], json!("exposing"));
    tokio::time::sleep(Duration::from_secs(1)).await;
    let (status, answer) = post_frame(&app).await;
    assert_eq!(status, StatusCode::CONFLICT, "{answer}");
    assert_eq!(
        answer["error"],
        json!("an exposure is already in progress"),
        "{answer}"
    );
    let (status, answer) = put(&app, json!({"gain":150})).await;
    assert_eq!(status, StatusCode::CONFLICT, "{answer}");
    assert_eq!(
        answer["error"],
        json!("an exposure is in progress"),
        "{answer}"
    );

    // A node served by into_server() alone, as a node built before 030.
    let sim = sim_rig();
    let clock = Arc::new(NodeTime(sim.clock().clone()));
    let node = NodeService::with_controls(
        SyntheticCamera::new(clock.clone()),
        clock.clone(),
        "test-token",
        Settings::default(),
        NodeControls::default(),
    )
    .unwrap();
    node.controls()
        .set_frame_interval(Duration::from_millis(500));
    let old = Transport::connector(move || {
        let node = node.clone();
        async move {
            let (client, server) = tokio::io::duplex(64 * 1024);
            let server_node = node.clone();
            tokio::spawn(async move {
                memory::serve_on(server, server_node.into_server())
                    .await
                    .unwrap();
            });
            Ok::<_, std::io::Error>(client)
        }
    });
    let app = finder::router(state(&sim, old, Some(&root)));
    wait_until(&app, |view| view["state"] == "no_service").await;
    refused(&app, "this scope node has no imaging service").await;

    // A NodeService with no imaging camera, served with routes().
    let sim = sim_rig();
    let clock = Arc::new(NodeTime(sim.clock().clone()));
    let node = NodeService::with_controls(
        SyntheticCamera::new(clock.clone()),
        clock.clone(),
        "test-token",
        Settings::default(),
        NodeControls::default(),
    )
    .unwrap();
    node.controls()
        .set_frame_interval(Duration::from_millis(500));
    let app = finder::router(state(&sim, duplex_service(node), Some(&root)));
    wait_until(&app, |view| view["state"] == "no_camera").await;
    refused(&app, "this scope node has no imaging camera").await;

    // A transport whose connector always answers an I/O error: no link.
    let sim = sim_rig();
    let broken = Transport::connector(|| async {
        Err::<tokio::io::DuplexStream, _>(std::io::Error::other("no route to the node"))
    });
    let app = finder::router(state(&sim, broken, Some(&root)));
    refused(&app, "the scope node is not connected").await;

    // A node whose camera cannot be reached.
    let sim = sim_rig();
    let clock = Arc::new(NodeTime(sim.clock().clone()));
    let node = NodeService::with_controls(
        SyntheticCamera::new(clock.clone()),
        clock.clone(),
        "test-token",
        Settings::default(),
        NodeControls::default(),
    )
    .unwrap()
    .with_imaging(UnreachableIndi {
        inner: SyntheticImagingCamera::new(Arc::new(NodeTime(sim.clock().clone()))),
    });
    let app = finder::router(state(&sim, duplex_service(node), Some(&root)));
    wait_until(&app, |view| view["state"] == "error").await;
    refused(
        &app,
        "the imaging camera is not answering: cannot reach the INDI server at 127.0.0.1:7624: refused",
    )
    .await;

    // A console built without with_imaging.
    let sim = sim_rig();
    let clock = Arc::new(NodeTime(sim.clock().clone()));
    let node = NodeService::with_controls(
        SyntheticCamera::new(clock.clone()),
        clock.clone(),
        "test-token",
        Settings::default(),
        NodeControls::default(),
    )
    .unwrap()
    .with_imaging(SyntheticImagingCamera::new(clock));
    let app = finder::router(state(&sim, duplex_service(node), None));
    refused(&app, "this console takes no imaging frames").await;
}

/// US5-S2: an exposure of 30 s shows `status.exposing` true 1 s in, an abort
/// ends the frame `aborted` within 1 s with no file under `frames/`, the next
/// frame is `000002.fits`, and an abort with nothing exposing changes nothing.
#[tokio::test(start_paused = true)]
async fn an_exposure_is_aborted_and_the_next_frame_is_saved() {
    let scratch = crate::support::Scratch::new("imaging-control-abort");
    let root = scratch.0.join("imaging");
    let app = stand_in(&root, None).await;
    ready(&app).await;
    put(&app, json!({"exposure_ms":30000})).await;

    let (status, answer) = post_frame(&app).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    assert_eq!(answer["frame"]["id"], json!(1));
    tokio::time::sleep(Duration::from_secs(1)).await;
    // 1 s into the exposure, before the abort, the console's own exposing
    // flag is true.
    let (_, view) = get(&app, "/api/imaging").await;
    assert_eq!(view["status"]["exposing"], json!(true), "{view}");

    let (status, answer) = post_abort(&app).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    // The frame ends aborted within 1 s.
    let mut aborted = None;
    for _ in 0..10 {
        let (_, view) = get(&app, "/api/imaging").await;
        if view["frame"]["state"] == "aborted" {
            aborted = Some(view);
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let view = aborted.expect("the frame was not aborted within 1 s");
    assert_eq!(view["frame"]["id"], json!(1), "{view}");
    assert_eq!(view["status"]["exposing"], json!(false), "{view}");
    assert!(view["frame"].get("file").is_none(), "{view}");
    assert!(!root.join("frames").exists(), "a file under frames/");

    // The next frame is the second: the count rose with the aborted one.
    put(&app, json!({"exposure_ms":100})).await;
    let (status, answer) = post_frame(&app).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    assert_eq!(answer["frame"]["id"], json!(2));
    wait_until(&app, |view| view["frame"]["state"] == "saved").await;
    assert!(frames(&root).join("000002.fits").exists());

    // An abort with nothing exposing answers 200 and changes nothing.
    let (_, before) = get(&app, "/api/imaging").await;
    let (status, after) = post_abort(&app).await;
    assert_eq!(status, StatusCode::OK, "{after}");
    assert_eq!(after, before, "the abort changed the view");
}

/// US5-S3: with `save_at_node` and a node that has no save folder the frame is
/// still saved and the view carries the node's reason, `node_save_error`, with
/// no `node_file`; without `save_at_node` the view carries neither key
/// (oracle: 030's US3-S6, the node's answer to `save` with no folder).
#[tokio::test(start_paused = true)]
async fn the_nodes_copy_when_the_node_has_no_save_folder() {
    let scratch = crate::support::Scratch::new("imaging-control-no-save-dir");
    let root = scratch.0.join("imaging");
    let app = stand_in(&root, None).await;
    ready(&app).await;

    put(&app, json!({"exposure_ms":100,"save_at_node":true})).await;
    let (status, _) = post_frame(&app).await;
    assert_eq!(status, StatusCode::OK);
    let view = wait_until(&app, |view| view["frame"]["state"] == "saved").await;
    let frame = &view["frame"];
    assert_eq!(
        frame["node_save_error"],
        json!("this scope node has no save_dir")
    );
    assert!(frame.get("node_file").is_none(), "{view}");
    let file = frames(&root).join("000001.fits");
    assert_eq!(std::fs::metadata(&file).unwrap().len(), 11_520);

    put(&app, json!({"save_at_node":false})).await;
    let (status, _) = post_frame(&app).await;
    assert_eq!(status, StatusCode::OK);
    let view = wait_until(&app, |view| {
        view["frame"]["id"] == json!(2) && view["frame"]["state"] == "saved"
    })
    .await;
    assert!(view["frame"].get("node_file").is_none(), "{view}");
    assert!(view["frame"].get("node_save_error").is_none(), "{view}");
}

/// US5-S4: with `save_at_node` and a node given `.with_imaging_save_dir(D)`,
/// D a folder that does not exist yet, the frame's view names the node's own
/// copy as `node_file` with no `node_save_error`, D holds it, and its samples
/// equal the console's (oracle: the two files, each read with this module's
/// own reader).
#[tokio::test(start_paused = true)]
async fn the_node_keeps_a_copy_of_the_frame_when_asked() {
    let scratch = crate::support::Scratch::new("imaging-control-node-copy");
    let root = scratch.0.join("imaging");
    let dir = scratch.0.join("node");
    let app = stand_in(&root, Some(dir.clone())).await;
    ready(&app).await;

    put(&app, json!({"exposure_ms":100,"save_at_node":true})).await;
    let (status, _) = post_frame(&app).await;
    assert_eq!(status, StatusCode::OK);
    let view = wait_until(&app, |view| view["frame"]["state"] == "saved").await;
    let frame = &view["frame"];
    assert_eq!(frame["node_file"], json!("frame-000001.fits"));
    assert!(frame.get("node_save_error").is_none(), "{view}");

    let node_file = dir.join("frame-000001.fits");
    assert!(node_file.exists(), "the node kept no copy");
    let node_samples = samples(&node_file);
    assert_eq!(node_samples.len(), 3072);
    let console_samples = samples(&frames(&root).join("000001.fits"));
    assert_eq!(node_samples, console_samples, "the two files differ");
}
