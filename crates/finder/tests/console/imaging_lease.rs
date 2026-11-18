//! The imaging camera's one holder (031 US9): the lease `Imaging::lease`
//! grants and `Lease` holds, the refusals it adds to `Imaging::set` and
//! `Imaging::take` for anyone else, what stays open to anyone, its release at
//! once when its task is aborted, and that a single imaging frame holds no
//! lease. Oracles: the rule of FR-028 and the scenarios' words, the stand-in
//! camera's settings and frames (030 US1), the file and preview of US2 and US6,
//! and tokio's paused clock.

use std::{path::Path, sync::Arc, time::Duration};

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
    Router,
};
use finder::{
    imaging::{Imaging, ImagingView, SettingsEdit},
    link::Transport,
    session::Session,
    AppState,
};
use scope_link::memory;
use scope_node::{
    clock::NodeClock,
    imaging::synthetic::SyntheticImagingCamera,
    service::{NodeControls, NodeService},
    synthetic::SyntheticCamera,
    Settings,
};
use serde_json::{json, Value};
use simulator::{mount::SimMount, Simulator, VirtualClock};
use tower::ServiceExt;

use crate::support::{live as support, mcp};

/// A node's NodeClock on the simulator's virtual clock, so the stand-in
/// camera's exposures follow the same clock the console reads.
struct NodeTime(VirtualClock);
impl NodeClock for NodeTime {
    fn now(&self) -> f64 {
        self.0.node_now()
    }
}

/// The stand-in console rig (the shared setup): the sim rig's Simulator with a
/// connected mount, the node rig whose imaging camera is the stand-in camera on
/// the node's clock, a paired session, the console's routes, its MCP door over
/// the same state, and its imaging camera.
struct Rig {
    app: Router,
    mcp: Router,
    state: AppState,
    imaging: Arc<Imaging>,
    clock: VirtualClock,
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

fn rig(sim: &Simulator, transport: Transport, root: &Path) -> Rig {
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
    let state = AppState::shared(support::DB.clone(), finder::samples::Samples::default())
        .with_session(session)
        .with_clock(Arc::new(support::ConsoleClock(sim.clock().clone())))
        .with_link(transport)
        .with_live()
        .with_imaging(root);
    Rig {
        app: finder::router(state.clone()),
        mcp: mcp::app(state.clone(), "127.0.0.1"),
        imaging: state.imaging().expect("imaging"),
        state,
        clock: sim.clock().clone(),
    }
}

/// The stand-in console rig: the node rig, whose cameras read the simulator's
/// virtual clock.
fn stand_in(root: &Path) -> Rig {
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
    node.controls()
        .set_frame_interval(Duration::from_millis(500));
    rig(&sim, duplex_service(node), root)
}

/// The console of US1-S4 whose transport's connector always answers an I/O
/// error: the link never connects, so imaging stays `no_link`.
fn no_link(root: &Path) -> Rig {
    let sim = sim_rig();
    let broken = Transport::connector(|| async {
        Err::<tokio::io::DuplexStream, _>(std::io::Error::other("no route to the node"))
    });
    rig(&sim, broken, root)
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

/// One preview route answer: its status and its content type.
async fn preview(app: &Router, id: u64) -> (StatusCode, String) {
    let response = app
        .clone()
        .oneshot(
            Request::get(format!("/api/imaging/frames/{id}/preview.jpg"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string();
    (status, content_type)
}

/// A settings edit in a Rust call: the `SettingsEdit` `serde_json::from_value`
/// makes of the JSON the scenario gives (the shared setup).
fn edit(value: Value) -> SettingsEdit {
    serde_json::from_value(value).unwrap()
}

/// Read the view every 100 ms until `check` holds, for at most 60 s.
async fn wait_until(imaging: &Arc<Imaging>, check: impl Fn(&ImagingView) -> bool) {
    for _ in 0..600 {
        if check(&imaging.view()) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!(
        "imaging never reached the awaited state: {:?}",
        imaging.view()
    );
}

async fn wait_ready(imaging: &Arc<Imaging>) {
    wait_until(imaging, |view| view.state == "ready").await;
}

/// The holder's name, read every 100 ms until it is `holder`.
async fn wait_holder(imaging: &Arc<Imaging>, holder: Option<&'static str>) {
    for _ in 0..600 {
        if imaging.holder() == holder {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("holder() never read {holder:?}: {:?}", imaging.holder());
}

/// Call a tool over the MCP door and answer its result, failing on a
/// JSON-RPC error.
async fn tool(mcp_door: &Router, name: &str, arguments: Value) -> Value {
    let response = mcp::send(mcp_door, mcp::call(name, arguments)).await;
    assert_eq!(response.status(), 200);
    let rpc = mcp::body(response).await;
    assert!(rpc.get("error").is_none(), "{rpc}");
    rpc["result"].clone()
}

/// The words of a tool error.
fn tool_error(result: &Value) -> &str {
    assert_eq!(result["isError"], true, "{result}");
    assert_eq!(result["content"][0]["type"], "text", "{result}");
    result["content"][0]["text"].as_str().unwrap()
}

/// The six requests a caller holding no lease makes, each answering 409 with
/// exactly `text` (US9-S2): through `Imaging`, through the routes, and over
/// the MCP door.
async fn refuses_every_write(rig: &Rig, text: &str) {
    let refusal = rig
        .imaging
        .set(edit(json!({"gain":150})))
        .await
        .unwrap_err();
    assert_eq!(refusal.status, StatusCode::CONFLICT, "{text}");
    assert_eq!(refusal.message, text);
    let refusal = rig.imaging.take("frame").await.unwrap_err();
    assert_eq!(refusal.status, StatusCode::CONFLICT, "{text}");
    assert_eq!(refusal.message, text);
    let (status, answer) = put(&rig.app, json!({"gain":150})).await;
    assert_eq!(status, StatusCode::CONFLICT, "{text}: {answer}");
    assert_eq!(answer["error"], json!(text), "{answer}");
    let (status, answer) = post_frame(&rig.app).await;
    assert_eq!(status, StatusCode::CONFLICT, "{text}: {answer}");
    assert_eq!(answer["error"], json!(text), "{answer}");
    let result = tool(&rig.mcp, "set_imaging_camera", json!({"gain":150})).await;
    assert_eq!(tool_error(&result), text, "{result}");
    let result = tool(&rig.mcp, "take_imaging_frame", json!({})).await;
    assert_eq!(tool_error(&result), text, "{result}");
}

/// US9-S1: the camera has one holder at a time. A second taker is refused at
/// once with 409 naming the holder, and once the first lets go the second has
/// it; and a lease looks at nothing but the lease, so a console with no link
/// grants one just the same (oracle: the rule of FR-028).
#[tokio::test(start_paused = true)]
async fn the_camera_is_held_by_one_holder_at_a_time() {
    let scratch = crate::support::Scratch::new("imaging-lease-one-holder");
    let root = scratch.0.join("imaging");
    let rig = stand_in(&root);
    wait_ready(&rig.imaging).await;

    let first = rig.imaging.lease("the test").expect("the first lease");
    assert_eq!(rig.imaging.holder(), Some("the test"));

    // The second taker is refused at once, with no time passing and the
    // holder named, and the holder keeps the camera.
    let before = tokio::time::Instant::now();
    let refusal = rig.imaging.lease("a second taker").unwrap_err();
    assert_eq!(tokio::time::Instant::now(), before, "the refusal waited");
    assert_eq!(refusal.status, StatusCode::CONFLICT);
    assert_eq!(refusal.message, "the imaging camera is held by the test");
    assert_eq!(rig.imaging.holder(), Some("the test"));

    // Once the first lets go, the second has the camera.
    drop(first);
    let second = rig
        .imaging
        .lease("a second taker")
        .expect("the second lease");
    assert_eq!(rig.imaging.holder(), Some("a second taker"));
    drop(second);
    assert_eq!(rig.imaging.holder(), None);

    // A lease looks at nothing but the lease (FR-028): a console whose
    // transport's connector always answers an I/O error grants one all the
    // same, its state `no_link`.
    let offline = no_link(&root);
    assert_eq!(offline.imaging.view().state, "no_link");
    offline
        .imaging
        .lease("the test")
        .expect("a lease on a console with no link");
}

/// US9-S2: while a lease is out, `Imaging::set` and `Imaging::take` answer the
/// holder's 409 to anyone else, before and during the holder's own frame, and
/// so do the routes and the MCP tools; nothing changes; and on a console with
/// no link the state's check comes before the lease's (oracles: the scenario's
/// words, and US1-S4's states).
#[tokio::test(start_paused = true)]
async fn every_write_but_the_holders_is_refused_with_the_holders_name() {
    let scratch = crate::support::Scratch::new("imaging-lease-refusals");
    let root = scratch.0.join("imaging");
    let rig = stand_in(&root);
    wait_ready(&rig.imaging).await;
    let (status, _) = put(&rig.app, json!({"exposure_ms":10000})).await;
    assert_eq!(status, StatusCode::OK);
    let lease = rig.imaging.lease("the test").expect("the lease");
    let held_by = "the imaging camera is held by the test";

    // With no frame in flight.
    refuses_every_write(&rig, held_by).await;
    // And again 1 s after the holder has started its own frame.
    let view = lease.take("frame").await.expect("the holder's frame");
    assert_eq!(view.frame.expect("the frame").id, 1);
    tokio::time::sleep(Duration::from_secs(1)).await;
    refuses_every_write(&rig, held_by).await;

    // Nothing changed: the stand-in camera keeps its gain, and the holder's
    // frame is the exposure in flight.
    let (_, read) = get(&rig.app, "/api/imaging").await;
    assert_eq!(read["settings"]["gain"], json!(100.0), "{read}");
    assert_eq!(read["frame"]["id"], json!(1), "{read}");
    assert_eq!(read["frame"]["state"], json!("exposing"), "{read}");
    assert_eq!(read["status"]["exposing"], json!(true), "{read}");

    // On a console with no link the state's words come first, for a caller
    // holding no lease and for the holder alike.
    let offline = no_link(&root);
    let lease = offline.imaging.lease("the test").expect("the lease");
    let no_link_words = "the scope node is not connected";
    let refusal = offline
        .imaging
        .set(edit(json!({"gain":150})))
        .await
        .unwrap_err();
    assert_eq!(refusal.status, StatusCode::CONFLICT);
    assert_eq!(refusal.message, no_link_words);
    let refusal = offline.imaging.take("frame").await.unwrap_err();
    assert_eq!(refusal.status, StatusCode::CONFLICT);
    assert_eq!(refusal.message, no_link_words);
    let refusal = lease.set(edit(json!({"gain":150}))).await.unwrap_err();
    assert_eq!(refusal.status, StatusCode::CONFLICT);
    assert_eq!(refusal.message, no_link_words);
}

/// US9-S3: the holder's own `Lease::set` and `Lease::take` are `Imaging::set`
/// and `Imaging::take` less the lease's check: the same answers, refusals,
/// words and order (oracles: the stand-in camera's settings and range, 030
/// US1; US2's file; US5-S1's refusals).
#[tokio::test(start_paused = true)]
async fn the_holders_own_calls_change_the_camera_and_save_frames() {
    let scratch = crate::support::Scratch::new("imaging-lease-holder");
    let root = scratch.0.join("imaging");
    let rig = stand_in(&root);
    wait_ready(&rig.imaging).await;
    let lease = rig.imaging.lease("the test").expect("the lease");

    // The holder changes the settings, and the route shows them.
    let view = lease
        .set(edit(json!({"exposure_ms":100,"gain":200})))
        .await
        .expect("the holder's settings change");
    let settings = view.settings.expect("the settings");
    assert_eq!(settings.exposure_ms, 100.0);
    assert_eq!(settings.gain, Some(200.0));
    let (_, read) = get(&rig.app, "/api/imaging").await;
    assert_eq!(read["settings"]["exposure_ms"], json!(100.0), "{read}");
    assert_eq!(read["settings"]["gain"], json!(200.0), "{read}");

    // A value the camera refuses answers its words.
    let refusal = lease.set(edit(json!({"gain":301}))).await.unwrap_err();
    assert_eq!(refusal.status, StatusCode::BAD_REQUEST);
    assert_eq!(
        refusal.message,
        "gain 301 is outside the camera's range 0–300"
    );

    // The holder takes a frame, which is saved as US2 saves it.
    let view = lease.take("frame").await.expect("the holder's frame");
    let frame = view.frame.expect("the frame");
    assert_eq!(frame.id, 1);
    assert_eq!(frame.state, "exposing");
    assert_eq!(frame.purpose, "frame");
    let frame = rig.imaging.ended(1).await;
    assert_eq!(frame.state, "saved");
    assert_eq!(frame.file.as_deref(), Some("frames/2026-01-16/000001.fits"));
    let file = root.join("frames/2026-01-16/000001.fits");
    assert_eq!(std::fs::metadata(&file).unwrap().len(), 11_520);

    // The holder meets every check of `Imaging::take` and `Imaging::set` but
    // the lease's own (FR-030).
    lease
        .set(edit(json!({"exposure_ms":10000})))
        .await
        .expect("the holder's settings change");
    lease
        .take("frame")
        .await
        .expect("the holder's second frame");
    tokio::time::sleep(Duration::from_secs(1)).await;
    let refusal = lease.take("frame").await.unwrap_err();
    assert_eq!(refusal.status, StatusCode::CONFLICT);
    assert_eq!(refusal.message, "an exposure is already in progress");
    let refusal = lease.set(edit(json!({"gain":150}))).await.unwrap_err();
    assert_eq!(refusal.status, StatusCode::CONFLICT);
    assert_eq!(refusal.message, "an exposure is in progress");
}

/// US9-S4: with a lease out, anyone reads the view, stops the exposure and
/// fetches the saved frame's preview (oracles: US5-S2's abort within 1 s, and
/// US6-S3's preview route).
#[tokio::test(start_paused = true)]
async fn the_view_the_abort_and_the_previews_stay_open_to_anyone() {
    let scratch = crate::support::Scratch::new("imaging-lease-open");
    let root = scratch.0.join("imaging");
    let rig = stand_in(&root);
    wait_ready(&rig.imaging).await;
    let (status, _) = put(&rig.app, json!({"exposure_ms":30000})).await;
    assert_eq!(status, StatusCode::OK);
    let lease = rig.imaging.lease("the test").expect("the lease");
    lease.take("frame").await.expect("the holder's frame");
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Anyone reads the view: the route, the tool and `Imaging::view` all
    // answer the holder's frame in flight.
    let (_, read) = get(&rig.app, "/api/imaging").await;
    assert_eq!(read["frame"]["id"], json!(1), "{read}");
    assert_eq!(read["frame"]["state"], json!("exposing"), "{read}");
    let result = tool(&rig.mcp, "get_imaging_camera", json!({})).await;
    assert_ne!(result["isError"], true, "{result}");
    assert_eq!(
        result["structuredContent"]["frame"]["id"],
        json!(1),
        "{result}"
    );
    assert_eq!(
        result["structuredContent"]["frame"]["state"],
        json!("exposing"),
        "{result}"
    );
    let frame = rig.imaging.view().frame.expect("the frame");
    assert_eq!(frame.id, 1);
    assert_eq!(frame.state, "exposing");

    // Anyone stops the exposure, and the frame ends aborted within 1 s of
    // the abort.
    let (status, _) = post_abort(&rig.app).await;
    assert_eq!(status, StatusCode::OK);
    let before = tokio::time::Instant::now();
    let frame = rig.imaging.ended(1).await;
    assert_eq!(frame.state, "aborted");
    assert!(
        before.elapsed() <= Duration::from_secs(1),
        "the abort took {:?}",
        before.elapsed()
    );

    // The holder takes another frame, saved with its preview; a caller
    // holding no lease reads the preview while the lease is still out.
    lease
        .set(edit(json!({"exposure_ms":100})))
        .await
        .expect("the holder's settings change");
    lease
        .take("frame")
        .await
        .expect("the holder's second frame");
    let frame = rig.imaging.ended(2).await;
    assert_eq!(frame.state, "saved");
    let (status, content_type) = preview(&rig.app, 2).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(content_type, "image/jpeg");
}

/// US9-S5: the lease is a value. A task aborted while it holds the camera
/// releases it in the same virtual instant, adds nothing to the `Imaging`'s
/// strong count but its own clone, and its frame runs on with no holder
/// (oracle: tokio's paused clock).
#[tokio::test(start_paused = true)]
async fn aborting_the_holders_task_releases_the_camera_at_once() {
    let scratch = crate::support::Scratch::new("imaging-lease-task");
    let root = scratch.0.join("imaging");
    let rig = stand_in(&root);
    let imaging = rig.imaging.clone();
    wait_ready(&imaging).await;
    let (status, _) = put(&rig.app, json!({"exposure_ms":10000})).await;
    assert_eq!(status, StatusCode::OK);

    let count = Arc::strong_count(&imaging);
    let holder = {
        let imaging = imaging.clone();
        tokio::spawn(async move {
            let lease = imaging.lease("the test").expect("the lease");
            lease.take("frame").await.expect("the holder's frame");
            imaging.ended(1).await;
        })
    };
    wait_holder(&imaging, Some("the test")).await;
    wait_until(&imaging, |view| {
        view.frame
            .as_ref()
            .is_some_and(|frame| frame.id == 1 && frame.state == "exposing")
    })
    .await;
    // The task's own clone, and nothing for the lease (FR-032).
    assert_eq!(Arc::strong_count(&imaging), count + 1);
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Aborting the task releases the camera in that same virtual instant.
    holder.abort();
    let error = holder.await.unwrap_err();
    assert!(
        error.is_cancelled(),
        "the task was not cancelled: {error:?}"
    );
    assert_eq!(imaging.holder(), None);

    // A lease can be taken and dropped again with no time passing.
    let before = tokio::time::Instant::now();
    {
        let second = imaging.lease("a second taker").expect("the second lease");
        assert_eq!(imaging.holder(), Some("a second taker"));
        drop(second);
    }
    assert_eq!(imaging.holder(), None);
    assert_eq!(
        tokio::time::Instant::now(),
        before,
        "taking and dropping a lease took time"
    );

    // The aborted task's frame runs on with no holder, so a second request
    // meets the frame-in-flight refusal (FR-033).
    let (status, answer) = post_frame(&rig.app).await;
    assert_eq!(status, StatusCode::CONFLICT, "{answer}");
    assert_eq!(
        answer["error"],
        json!("an exposure is already in progress"),
        "{answer}"
    );
    let frame = imaging.ended(1).await;
    assert_eq!(frame.state, "saved");
}

/// US9-S5, second case: a lease holds no strong reference, so once the
/// `Imaging` is gone its calls answer the console's refusal and its drop does
/// nothing (FR-032).
#[tokio::test(start_paused = true)]
async fn a_lease_of_a_dropped_imaging_refuses_and_releases_quietly() {
    let scratch = crate::support::Scratch::new("imaging-lease-dropped");
    let root = scratch.0.join("imaging");
    let rig = stand_in(&root);
    wait_ready(&rig.imaging).await;

    // The test's own `Imaging` over the rig's link and clock, a `Weak` of it
    // and a lease of it, then the test's one `Arc` is dropped.
    let link = rig.state.link().expect("the rig's link");
    let clock = Arc::new(support::ConsoleClock(rig.clock.clone()));
    let imaging = Imaging::start(root.clone(), link, clock);
    let weak = Arc::downgrade(&imaging);
    let lease = imaging.lease("the test").expect("the lease");
    drop(imaging);
    assert!(weak.upgrade().is_none(), "the Imaging is still alive");

    let refusal = lease.take("frame").await.unwrap_err();
    assert_eq!(refusal.status, StatusCode::CONFLICT);
    assert_eq!(refusal.message, "this console takes no imaging frames");
    let refusal = lease.set(SettingsEdit::default()).await.unwrap_err();
    assert_eq!(refusal.status, StatusCode::CONFLICT);
    assert_eq!(refusal.message, "this console takes no imaging frames");

    // Dropping the lease neither panics nor touches the rig's own imaging,
    // whose camera was never held.
    drop(lease);
    assert_eq!(rig.imaging.holder(), None);
}

/// US9-S6: an imaging frame asked for from the page or over the MCP door
/// holds no lease: `holder()` stays `None` while it is in flight, a lease is
/// still granted, and each holder looks for a frame in flight itself
/// (FR-033).
#[tokio::test(start_paused = true)]
async fn a_single_frame_holds_no_lease() {
    let scratch = crate::support::Scratch::new("imaging-lease-single-frame");
    let root = scratch.0.join("imaging");
    let rig = stand_in(&root);
    let imaging = rig.imaging.clone();
    wait_ready(&imaging).await;
    let (status, _) = put(&rig.app, json!({"exposure_ms":10000})).await;
    assert_eq!(status, StatusCode::OK);

    // A frame from the page holds no lease.
    let (status, answer) = post_frame(&rig.app).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert_eq!(imaging.holder(), None);

    // The camera can still be taken, and the holder looks for the frame in
    // flight itself.
    let lease = imaging.lease("the test").expect("the lease");
    let refusal = lease.take("frame").await.unwrap_err();
    assert_eq!(refusal.status, StatusCode::CONFLICT);
    assert_eq!(refusal.message, "an exposure is already in progress");
    let refusal = lease.set(edit(json!({"gain":150}))).await.unwrap_err();
    assert_eq!(refusal.status, StatusCode::CONFLICT);
    assert_eq!(refusal.message, "an exposure is in progress");

    // A caller holding no lease meets the holder's refusal, and the frame is
    // saved while the lease is still out.
    let (status, answer) = post_frame(&rig.app).await;
    assert_eq!(status, StatusCode::CONFLICT, "{answer}");
    assert_eq!(
        answer["error"],
        json!("the imaging camera is held by the test"),
        "{answer}"
    );
    let frame = imaging.ended(1).await;
    assert_eq!(frame.state, "saved");
    assert_eq!(frame.file.as_deref(), Some("frames/2026-01-16/000001.fits"));
    drop(lease);

    // A frame asked for over the MCP door holds no lease either: while the
    // call waits for its frame the camera stays free.
    let take = {
        let door = rig.mcp.clone();
        tokio::spawn(async move { tool(&door, "take_imaging_frame", json!({})).await })
    };
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert_eq!(imaging.holder(), None, "the tool's frame holds no lease");
    let result = take.await.unwrap();
    assert_ne!(result["isError"], true, "{result}");
    assert_eq!(
        result["structuredContent"]["frame"]["state"],
        json!("saved"),
        "{result}"
    );
}
