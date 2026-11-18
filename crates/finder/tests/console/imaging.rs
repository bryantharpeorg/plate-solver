//! The console's view of the paired node's imaging camera (031 US1): the second
//! connection, the view of camera, status and settings, the settings route and
//! every state that is not `ready`. Oracles: the simulated imaging camera's own
//! description and rules (030 US2), the stand-in camera's sample and cooler rules
//! (030 US1), and the times tokio's paused clock measures.

use std::{
    path::Path,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use finder::{link::Transport, session::Session, AppState};
use scope_link::{memory, proto};
use scope_node::{
    clock::NodeClock,
    imaging::{
        synthetic::SyntheticImagingCamera, Exposure, ImagingCamera, ImagingError, ImagingFrame,
        ImagingFuture, ImagingInfo, ImagingSettings, ImagingStatus,
    },
    service::{NodeControls, NodeService},
    Settings,
};
use serde_json::{json, Value};
use simulator::{mount::SimMount, SimCamera, SimNode, Simulator, VirtualClock};
use tower::ServiceExt;

use crate::support::live as support;

/// One tokio turn, so spawned transport tasks finish between reads.
async fn settle() {
    tokio::time::sleep(Duration::from_millis(25)).await;
}

/// The console rig of the shared setup: the sim rig with a connected mount, a
/// `SimNode` (its imaging camera the simulated one) over a fresh duplex per
/// connection, a paired session, and `AppState` with live and imaging.
struct Rig {
    sim: Simulator,
    app: Router,
    link: Arc<finder::link::LinkManager>,
}

fn console_clock(sim: &Simulator) -> Arc<support::ConsoleClock> {
    Arc::new(support::ConsoleClock(sim.clock().clone()))
}

/// A node's NodeClock on the simulator's virtual clock, so a test's own camera
/// follows the same clock the stand-in does.
struct NodeTime(VirtualClock);
impl NodeClock for NodeTime {
    fn now(&self) -> f64 {
        self.0.node_now()
    }
}

fn duplex(node: &Arc<SimNode>) -> Transport {
    let node = node.clone();
    Transport::connector(move || {
        let node = node.clone();
        async move {
            let (client, server) = tokio::io::duplex(64 * 1024);
            let service = node.service();
            tokio::spawn(async move {
                memory::serve_routes_on(server, service.routes())
                    .await
                    .unwrap();
            });
            Ok::<_, std::io::Error>(client)
        }
    })
}

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

fn state(
    sim: &Simulator,
    transport: Transport,
    root: Option<&Path>,
) -> (AppState, Arc<finder::link::LinkManager>) {
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
        .with_clock(console_clock(sim))
        .with_link(transport);
    let link = state.link().unwrap();
    let state = state.with_live();
    let state = match root {
        Some(root) => state.with_imaging(root),
        None => state,
    };
    (state, link)
}

fn sim_rig() -> Simulator {
    let sim = support::sim();
    sim.connect_mount(SimMount::new(sim.clock().clone()));
    sim.push_to(50.0, 120.0);
    sim
}

async fn rig(root: &Path) -> Rig {
    let sim = sim_rig();
    let node = Arc::new(
        SimNode::new(sim.clone(), "test-token", Settings::default())
            .unwrap()
            .with_frame_interval(Duration::from_millis(500)),
    );
    let (state, link) = state(&sim, duplex(&node), Some(root));
    Rig {
        sim,
        app: finder::router(state),
        link,
    }
}

/// A rig whose node's imaging camera is the test's own, built like the console rig
/// otherwise.
async fn camera_rig(root: &Path, camera: impl ImagingCamera + 'static) -> Rig {
    let sim = sim_rig();
    let node = NodeService::with_controls(
        SimCamera::new(sim.clone()),
        Arc::new(NodeTime(sim.clock().clone())),
        "test-token",
        Settings::default(),
        NodeControls::default(),
    )
    .unwrap()
    .with_imaging(camera);
    node.controls()
        .set_frame_interval(Duration::from_millis(500));
    let (state, link) = state(&sim, duplex_service(node), Some(root));
    Rig {
        sim,
        app: finder::router(state),
        link,
    }
}

async fn get(app: &Router, path: &str) -> (StatusCode, Value) {
    support::request(app, "GET", path, json!(null)).await
}

async fn put(app: &Router, body: Value) -> (StatusCode, Value) {
    support::request(app, "PUT", "/api/imaging/settings", body).await
}

/// A request the shared `request` helper cannot send: a chosen content type or a
/// foreign origin, with a `Host` of localhost on port 8080 as the same-origin
/// rule reads it.
async fn raw(
    app: &Router,
    method: &str,
    path: &str,
    body: &str,
    content_type: &str,
    origin: Option<&str>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header("host", "localhost:8080")
        .header("content-type", content_type);
    if let Some(origin) = origin {
        builder = builder.header("origin", origin);
    }
    let response = app
        .clone()
        .oneshot(builder.body(Body::from(body.to_string())).unwrap())
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

/// The simulated camera's description (030 US2-S1), as the view serializes it.
fn camera() -> Value {
    json!({
        "model": "simulated imaging camera",
        "serial": "SIM-0001",
        "width": 1236,
        "height": 822,
        "pixel_um": 19.12,
        "bits": 16,
        "bayer": "RGGB",
        "exposure_ms": {"min": 1.0, "max": 3600000.0},
        "gain": {"min": 0.0, "max": 300.0},
        "offset": {"min": 0.0, "max": 100.0},
        "binnings": [1, 2],
        "cooler": true
    })
}
/// The cooler's rule with the cooler off (the shared setup).
fn status() -> Value {
    json!({
        "temp_c": 20.0, "cooler_power": 0.0, "set_temp_c": null, "exposing": false
    })
}
/// The whole sensor of the simulated camera.
fn roi() -> Value {
    json!({"x": 0, "y": 0, "width": 1236, "height": 822})
}

/// The settings the console rig starts with (US1-S1); key by key, never whole.
fn assert_starting_settings(settings: &Value) {
    assert_eq!(settings["exposure_ms"], json!(1000.0));
    assert_eq!(settings["gain"], json!(120.0));
    assert_eq!(settings["offset"], json!(10.0));
    assert_eq!(settings["binning"], json!(1));
    assert_eq!(settings["roi"], roi());
    assert_eq!(settings["cooler_on"], json!(false));
    assert_eq!(settings["set_temp_c"], json!(null));
    assert_eq!(settings["forward_kib_per_s"], json!(null));
    assert_eq!(settings["save_at_node"], json!(false));
}

/// A camera of the test's own that stands in for the stand-in camera: it describes
/// itself as the stand-in does and answers an empty `apply` with the stand-in's
/// starting settings, while a mode chooses how a settings change is refused.
struct StandIn {
    inner: SyntheticImagingCamera,
    mode: Arc<Mutex<Refused>>,
}

#[derive(Clone, Copy)]
enum Refused {
    Nothing,
    Failed,
    Never,
}

impl StandIn {
    fn new(clock: Arc<dyn NodeClock>) -> Self {
        Self {
            inner: SyntheticImagingCamera::new(clock),
            mode: Arc::new(Mutex::new(Refused::Nothing)),
        }
    }
}

impl ImagingCamera for StandIn {
    fn describe(&self) -> ImagingFuture<'_, ImagingInfo> {
        self.inner.describe()
    }
    fn status(&self) -> ImagingFuture<'_, ImagingStatus> {
        self.inner.status()
    }
    fn apply<'a>(&'a self, edit: &'a ImagingSettings) -> ImagingFuture<'a, ImagingSettings> {
        if *edit == ImagingSettings::default() {
            return self.inner.apply(edit);
        }
        match *self.mode.lock().unwrap() {
            Refused::Nothing => self.inner.apply(edit),
            Refused::Failed => Box::pin(async {
                Err(ImagingError::Failed(
                    "the camera refused CCD_CONTROLS: no".into(),
                ))
            }),
            Refused::Never => Box::pin(std::future::pending()),
        }
    }
    fn expose<'a>(&'a self, exposure: &'a Exposure) -> ImagingFuture<'a, ImagingFrame> {
        self.inner.expose(exposure)
    }
    fn abort(&self) -> ImagingFuture<'_, ()> {
        self.inner.abort()
    }
}

/// A camera of the test's own that answers `describe` as a mode chooses, noting
/// tokio's clock at each call.
struct Describes {
    inner: SyntheticImagingCamera,
    times: Arc<Mutex<Vec<tokio::time::Instant>>>,
    mode: Arc<Mutex<DescribeMode>>,
}

#[derive(Clone, Copy)]
enum DescribeMode {
    NotYet,
    Never,
}

impl Describes {
    fn new(clock: Arc<dyn NodeClock>) -> Self {
        Self {
            inner: SyntheticImagingCamera::new(clock),
            times: Arc::new(Mutex::new(Vec::new())),
            mode: Arc::new(Mutex::new(DescribeMode::NotYet)),
        }
    }
}

impl ImagingCamera for Describes {
    fn describe(&self) -> ImagingFuture<'_, ImagingInfo> {
        let mut times = self.times.lock().unwrap();
        times.push(tokio::time::Instant::now());
        let calls = times.len();
        drop(times);
        match *self.mode.lock().unwrap() {
            DescribeMode::NotYet if calls < 3 => {
                Box::pin(async { Err(ImagingError::Failed("not yet".into())) })
            }
            DescribeMode::Never => Box::pin(std::future::pending()),
            _ => self.inner.describe(),
        }
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

/// US1-S1: the ready view and the session's `imaging` show the simulated camera,
/// its status and the settings in force exactly (oracle: the camera's own
/// description, 030 US2-S1).
#[tokio::test(start_paused = true)]
async fn the_ready_view_shows_the_simulated_camera() {
    let scratch = crate::support::Scratch::new("imaging-ready");
    let rig = rig(&scratch.0.join("imaging")).await;
    let view = ready(&rig.app).await;
    assert_eq!(view["camera"], camera());
    assert_eq!(view["status"], status());
    assert_starting_settings(&view["settings"]);
    assert!(view.get("frame").is_none());
    let (_, session) = get(&rig.app, "/api/session").await;
    assert_eq!(session["imaging"], view);
}

/// US1-S2: settings changes reach the camera and the console keeps its own three.
#[tokio::test(start_paused = true)]
async fn settings_changes_reach_the_camera_and_the_console_keeps_its_own() {
    let scratch = crate::support::Scratch::new("imaging-settings");
    let rig = rig(&scratch.0.join("imaging")).await;
    ready(&rig.app).await;

    let (status, view) = put(&rig.app, json!({"exposure_ms":500,"gain":200,"binning":2})).await;
    assert_eq!(status, StatusCode::OK);
    let settings = &view["settings"];
    assert_eq!(settings["exposure_ms"], json!(500.0));
    assert_eq!(settings["gain"], json!(200.0));
    assert_eq!(settings["binning"], json!(2));
    assert_eq!(settings["offset"], json!(10.0));
    assert_eq!(settings["roi"], roi());
    assert_eq!(settings["cooler_on"], json!(false));
    assert_eq!(settings["set_temp_c"], json!(null));
    assert_eq!(settings["forward_kib_per_s"], json!(null));
    assert_eq!(settings["save_at_node"], json!(false));
    // The camera itself holds the change (oracle: the camera, asked again).
    let mut client = rig.link.connect_imaging().await.unwrap();
    let applied = client
        .configure(proto::ImagingSettings::default())
        .await
        .unwrap();
    assert_eq!(applied.gain, Some(200.0));
    assert_eq!(applied.binning, Some(2));

    let region = json!({"x":400,"y":200,"width":400,"height":400});
    let (_, view) = put(&rig.app, json!({"roi": region})).await;
    assert_eq!(view["settings"]["roi"], region);
    let (_, view) = put(&rig.app, json!({"roi": null})).await;
    assert_eq!(view["settings"]["roi"], roi());

    let (_, view) = put(
        &rig.app,
        json!({"forward_kib_per_s":4096,"save_at_node":true}),
    )
    .await;
    assert_eq!(view["settings"]["forward_kib_per_s"], json!(4096));
    assert_eq!(view["settings"]["save_at_node"], json!(true));
    let (_, view) = put(&rig.app, json!({"forward_kib_per_s":null})).await;
    assert_eq!(view["settings"]["forward_kib_per_s"], json!(null));
    assert_eq!(view["settings"]["save_at_node"], json!(true));

    // An empty edit answers 200 and changes nothing.
    let (status, view) = put(&rig.app, json!({})).await;
    assert_eq!(status, StatusCode::OK);
    let settings = &view["settings"];
    assert_eq!(settings["exposure_ms"], json!(500.0));
    assert_eq!(settings["gain"], json!(200.0));
    assert_eq!(settings["binning"], json!(2));
    assert_eq!(settings["roi"], roi());
    assert_eq!(settings["forward_kib_per_s"], json!(null));
    assert_eq!(settings["save_at_node"], json!(true));
}

/// US1-S3: refused edits answer 400 with the camera's or the console's words and
/// change nothing; a foreign origin 403, a foreign content type 415, a body that
/// is not JSON 400.
#[tokio::test(start_paused = true)]
async fn refused_edits_answer_400_and_change_nothing() {
    let scratch = crate::support::Scratch::new("imaging-refusals");
    let rig = rig(&scratch.0.join("imaging")).await;
    ready(&rig.app).await;
    let bodies: [(Value, Option<&str>); 11] = [
        (
            json!({"gain":301}),
            Some("gain 301 is outside the camera's range 0–300"),
        ),
        (
            json!({"binning":3}),
            Some("binning 3 is not one of the camera's binnings 1, 2"),
        ),
        (
            json!({"exposure_ms":0.5}),
            Some("exposure_ms 0.5 is outside the camera's range 1–3600000 ms"),
        ),
        (
            json!({"forward_kib_per_s":10}),
            Some("forward_kib_per_s must be a whole number from 64 to 1048576, or null"),
        ),
        (
            json!({"forward_kib_per_s":100.5}),
            Some("forward_kib_per_s must be a whole number from 64 to 1048576, or null"),
        ),
        (
            json!({"set_temp_c":60}),
            Some("set_temp_c must be between -50 and 50"),
        ),
        (json!({"gain":null}), None),
        (json!({"gian":1}), None),
        (json!({"binning":"2"}), None),
        (json!({"roi":{"x":400,"y":200,"width":400}}), None),
        (
            json!({"roi":{"x":400,"y":200,"width":400,"height":400,"z":1}}),
            None,
        ),
    ];
    for (body, text) in bodies {
        let (status, answer) = put(&rig.app, body.clone()).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
        assert!(answer["error"].is_string(), "{body}");
        if let Some(text) = text {
            assert_eq!(answer["error"], json!(text), "{body}");
        }
        let (_, view) = get(&rig.app, "/api/imaging").await;
        assert_starting_settings(&view["settings"]);
    }
    let (status, _) = raw(
        &rig.app,
        "PUT",
        "/api/imaging/settings",
        "{\"gain\":150}",
        "application/json",
        Some("http://elsewhere.example"),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = raw(
        &rig.app,
        "PUT",
        "/api/imaging/settings",
        "{\"gain\":150}",
        "text/plain",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    let (status, _) = raw(
        &rig.app,
        "PUT",
        "/api/imaging/settings",
        "not json",
        "application/json",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (_, view) = get(&rig.app, "/api/imaging").await;
    assert_starting_settings(&view["settings"]);
}

/// US1-S4: every other pairing says which state it is in and refuses with its words.
#[tokio::test(start_paused = true)]
async fn other_pairings_show_their_state_and_refuse() {
    let scratch = crate::support::Scratch::new("imaging-pairings");
    let root = scratch.0.join("imaging");

    // A node served by into_server() alone, as a node built before 030.
    let sim = sim_rig();
    let node = NodeService::with_controls(
        SimCamera::new(sim.clone()),
        Arc::new(NodeTime(sim.clock().clone())),
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
    let (no_service, _link) = state(&sim, old, Some(&root));
    let app = finder::router(no_service);
    let view = wait_until(&app, |view| view["state"] == "no_service").await;
    assert!(view.get("camera").is_none());
    let (status, answer) = put(&app, json!({"gain":150})).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(
        answer["error"],
        json!("this scope node has no imaging service")
    );

    // A NodeService with no imaging camera, served with routes().
    let sim = sim_rig();
    let node = NodeService::with_controls(
        SimCamera::new(sim.clone()),
        Arc::new(NodeTime(sim.clock().clone())),
        "test-token",
        Settings::default(),
        NodeControls::default(),
    )
    .unwrap();
    node.controls()
        .set_frame_interval(Duration::from_millis(500));
    let (no_camera, _link) = state(&sim, duplex_service(node), Some(&root));
    let app = finder::router(no_camera);
    let view = wait_until(&app, |view| view["state"] == "no_camera").await;
    assert!(view.get("camera").is_none());
    let (status, answer) = put(&app, json!({"gain":150})).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(
        answer["error"],
        json!("this scope node has no imaging camera")
    );

    // A transport whose connector always answers an I/O error: no link at all.
    let sim = sim_rig();
    let broken = Transport::connector(|| async {
        Err::<tokio::io::DuplexStream, _>(std::io::Error::other("no route to the node"))
    });
    let (no_link, _link) = state(&sim, broken, Some(&root));
    let app = finder::router(no_link);
    settle().await;
    let (_, view) = get(&app, "/api/imaging").await;
    assert_eq!(view["state"], json!("no_link"));
    let (status, answer) = put(&app, json!({"gain":150})).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(answer["error"], json!("the scope node is not connected"));

    // A node whose camera cannot be reached.
    let sim = sim_rig();
    let camera = UnreachableIndi {
        inner: SyntheticImagingCamera::new(Arc::new(NodeTime(sim.clock().clone()))),
    };
    let node = NodeService::with_controls(
        SimCamera::new(sim.clone()),
        Arc::new(NodeTime(sim.clock().clone())),
        "test-token",
        Settings::default(),
        NodeControls::default(),
    )
    .unwrap()
    .with_imaging(camera);
    let (unreachable, _link) = state(&sim, duplex_service(node), Some(&root));
    let app = finder::router(unreachable);
    let view = wait_until(&app, |view| view["state"] == "error").await;
    assert_eq!(
        view["error"],
        json!("cannot reach the INDI server at 127.0.0.1:7624: refused")
    );
    assert!(view.get("camera").is_none());
    let (status, answer) = put(&app, json!({"gain":150})).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(
        answer["error"],
        json!("the imaging camera is not answering: cannot reach the INDI server at 127.0.0.1:7624: refused")
    );

    // A console built without with_imaging.
    let sim = sim_rig();
    let node = Arc::new(
        SimNode::new(sim.clone(), "test-token", Settings::default())
            .unwrap()
            .with_frame_interval(Duration::from_millis(500)),
    );
    let (off, _link) = state(&sim, duplex(&node), None);
    let app = finder::router(off);
    let (_, session) = get(&app, "/api/session").await;
    assert!(session.get("imaging").is_none());
    let (status, view) = get(&app, "/api/imaging").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(view, json!({"state": "off"}));
    let (status, answer) = put(&app, json!({"gain":150})).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(
        answer["error"],
        json!("this console takes no imaging frames")
    );
}

/// The words of a camera whose INDI server cannot be reached (US1-S4).
const NO_INDI: &str = "cannot reach the INDI server at 127.0.0.1:7624: refused";

/// A camera of the test's own whose INDI server cannot be reached (US1-S4). Its
/// other calls answer as the stand-in camera does, so only the describe fails.
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

/// US1-S5: the console opens exactly two connections and keeps them.
#[tokio::test(start_paused = true)]
async fn the_imaging_connection_is_one_of_two_and_lasts() {
    let scratch = crate::support::Scratch::new("imaging-connections");
    let sim = sim_rig();
    let node = Arc::new(
        SimNode::new(sim.clone(), "test-token", Settings::default())
            .unwrap()
            .with_frame_interval(Duration::from_millis(500)),
    );
    let streams = Arc::new(AtomicUsize::new(0));
    let transport = {
        let streams = streams.clone();
        Transport::connector(move || {
            let node = node.clone();
            let streams = streams.clone();
            async move {
                streams.fetch_add(1, Ordering::SeqCst);
                let (client, server) = tokio::io::duplex(64 * 1024);
                let service = node.service();
                tokio::spawn(async move {
                    memory::serve_routes_on(server, service.routes())
                        .await
                        .unwrap();
                });
                Ok::<_, std::io::Error>(client)
            }
        })
    };
    let root = scratch.0.join("imaging");
    let (state, _link) = state(&sim, transport, Some(&root));
    let app = finder::router(state);

    // Wait for ready and the first star frame together, as the scenario reads them.
    let mut view = Value::Null;
    for _ in 0..600 {
        let (_, imaging) = get(&app, "/api/imaging").await;
        let (_, session) = get(&app, "/api/session").await;
        if imaging["state"] == "ready" && session["node"]["stars"].is_number() {
            view = imaging;
            assert_eq!(session["node"]["connected"], json!(true));
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert_eq!(view["state"], json!("ready"), "imaging never became ready");
    assert_eq!(streams.load(Ordering::SeqCst), 2);
    tokio::time::sleep(Duration::from_secs(10)).await;
    assert_eq!(streams.load(Ordering::SeqCst), 2);
}

/// US1-S6: a dropped link shows no_link within 3 s, ready again within 15 s, and
/// the camera keeps its settings.
#[tokio::test(start_paused = true)]
async fn a_dropped_link_shows_no_link_and_ready_again() {
    let scratch = crate::support::Scratch::new("imaging-drop");
    let rig = rig(&scratch.0.join("imaging")).await;
    ready(&rig.app).await;
    let (_, view) = put(&rig.app, json!({"gain":200})).await;
    assert_eq!(view["settings"]["gain"], json!(200.0));
    tokio::time::sleep(Duration::from_millis(1500)).await;
    let start = tokio::time::Instant::now();
    rig.sim.set_unresponsive(true);
    let view = wait_until(&rig.app, |view| view["state"] == "no_link").await;
    assert!(start.elapsed() <= Duration::from_secs(3));
    assert!(view.get("camera").is_none());
    rig.sim.set_unresponsive(false);
    let start = tokio::time::Instant::now();
    // Every read until ready shows no camera; ready brings it back.
    for _ in 0..150 {
        let (_, view) = get(&rig.app, "/api/imaging").await;
        if view["state"] == "ready" {
            assert!(start.elapsed() <= Duration::from_secs(15));
            assert_eq!(view["settings"]["gain"], json!(200.0));
            return;
        }
        assert!(view.get("camera").is_none(), "{view}");
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("imaging never became ready again");
}

/// US1-S8: switching the cooler on shows the cooler's rule within 5 s.
#[tokio::test(start_paused = true)]
async fn the_cooler_reaches_its_set_temperature() {
    let scratch = crate::support::Scratch::new("imaging-cooler");
    let rig = rig(&scratch.0.join("imaging")).await;
    ready(&rig.app).await;
    let (_, view) = put(&rig.app, json!({"cooler_on":true,"set_temp_c":-10})).await;
    assert_eq!(view["settings"]["cooler_on"], json!(true));
    let cooled = json!({
        "temp_c": -10.0, "cooler_power": 75.0, "set_temp_c": -10.0, "exposing": false
    });
    let start = tokio::time::Instant::now();
    for _ in 0..50 {
        let (_, view) = get(&rig.app, "/api/imaging").await;
        if view["status"] == cooled {
            assert!(start.elapsed() <= Duration::from_secs(5));
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("the cooler never reached its set temperature");
}

/// US1-S9: a camera that refuses a settings change answers 502 with the node's
/// words, one that never answers 502 after exactly 30 s, and nothing is kept.
#[tokio::test(start_paused = true)]
async fn a_camera_that_refuses_or_never_answers_fails_the_edit() {
    let scratch = crate::support::Scratch::new("imaging-apply");
    let clock = Arc::new(NodeTime(VirtualClock::new(support::START)));
    let camera = StandIn::new(clock);
    let mode = camera.mode.clone();
    let rig = camera_rig(&scratch.0.join("imaging"), camera).await;
    ready(&rig.app).await;
    let (_, view) = get(&rig.app, "/api/imaging").await;
    assert_eq!(view["settings"]["gain"], json!(100.0));

    *mode.lock().unwrap() = Refused::Failed;
    let (status, answer) = put(&rig.app, json!({"gain":150})).await;
    assert_eq!(status, StatusCode::BAD_GATEWAY);
    assert_eq!(
        answer["error"],
        json!("the scope node did not apply the settings: the camera refused CCD_CONTROLS: no")
    );
    let (_, view) = get(&rig.app, "/api/imaging").await;
    assert_eq!(view["settings"]["gain"], json!(100.0));

    *mode.lock().unwrap() = Refused::Never;
    let start = tokio::time::Instant::now();
    let (status, answer) = put(&rig.app, json!({"gain":150})).await;
    let elapsed = start.elapsed();
    assert_eq!(status, StatusCode::BAD_GATEWAY);
    assert_eq!(
        answer["error"],
        json!("the scope node did not apply the settings: no answer within 30 s")
    );
    assert!(
        elapsed >= Duration::from_secs(30) && elapsed <= Duration::from_millis(30_200),
        "the edit took {elapsed:?}"
    );
    let (_, view) = get(&rig.app, "/api/imaging").await;
    assert_eq!(view["settings"]["gain"], json!(100.0));
}

/// US1-S10: a camera that cannot be described is asked again exactly every 5 s,
/// and a describe that never answers is an error within 5 s of its first call.
#[tokio::test(start_paused = true)]
async fn a_failing_describe_is_retried_every_five_seconds() {
    let scratch = crate::support::Scratch::new("imaging-retry");
    let clock = Arc::new(NodeTime(VirtualClock::new(support::START)));
    let camera = Describes::new(clock);
    let times = camera.times.clone();
    let rig = camera_rig(&scratch.0.join("imaging"), camera).await;
    let view = wait_until(&rig.app, |view| {
        view["state"] == "error" && view["error"] == json!("not yet")
    })
    .await;
    assert!(view.get("camera").is_none());
    ready(&rig.app).await;
    let times = times.lock().unwrap();
    assert_eq!(times.len(), 3);
    assert_eq!(times[1].duration_since(times[0]), Duration::from_secs(5));
    assert_eq!(times[2].duration_since(times[1]), Duration::from_secs(5));
}

#[tokio::test(start_paused = true)]
async fn a_describe_that_never_answers_is_an_error_within_five_seconds() {
    let scratch = crate::support::Scratch::new("imaging-hang");
    let clock = Arc::new(NodeTime(VirtualClock::new(support::START)));
    let camera = Describes::new(clock);
    let times = camera.times.clone();
    *camera.mode.lock().unwrap() = DescribeMode::Never;
    let rig = camera_rig(&scratch.0.join("imaging"), camera).await;
    wait_until(&rig.app, |view| {
        view["state"] == "error"
            && view["error"] == json!("the scope node did not answer within 5 s")
    })
    .await;
    let called = times.lock().unwrap()[0];
    let elapsed = tokio::time::Instant::now().duration_since(called);
    assert!(
        elapsed >= Duration::from_secs(5) && elapsed <= Duration::from_millis(5_200),
        "the error came after {elapsed:?}"
    );
}
