//! The console takes a preview frame and saves nothing (058 US1): one frame
//! taken as `POST /api/imaging/frame` takes one, with its refusals and words,
//! but shown and saved nowhere, at the console or at the node, and ADR 0002's
//! amendment. Oracles: the simulated imaging camera's 1236 × 822 RGGB sensor,
//! the scenarios' words, routes and statuses, the folders' own listings, and
//! ADR 0002's text.

use std::{path::Path, sync::Arc, time::Duration};

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use finder::{
    imaging::{simulated::SimulatedSolver, Imaging},
    link::Transport,
    samples::Samples,
    session::Session,
    AppState,
};
use image::GenericImageView;
use scope_link::memory;
use serde_json::{json, Value};
use simulator::{mount::SimMount, SimNode, Simulator};
use tower::ServiceExt;

use crate::support::{live as support, Scratch};

/// The preview rig (the shared setup): its own node serving the simulator's
/// imaging camera, the node's own save folder N set through its service, the
/// simulated imaging solver over the simulator, a paired session, and the
/// imaging folder R.
struct Rig {
    app: Router,
    imaging: Arc<Imaging>,
}

fn sim_rig() -> Simulator {
    let sim = support::sim();
    sim.connect_mount(SimMount::new(sim.clock().clone()));
    sim.push_to(50.0, 120.0);
    sim
}

/// One fresh duplex for each connection, serving the node's own routes.
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

/// The preview rig: the node's folder N is set on the node's own service,
/// which every clone of it shares.
fn rig(root: &Path, node: &Path) -> Rig {
    let sim = sim_rig();
    let service = Arc::new(
        SimNode::new(sim.clone(), "test-token", Default::default())
            .unwrap()
            .with_frame_interval(Duration::from_millis(500)),
    );
    service.service().with_imaging_save_dir(node.to_path_buf());
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
    let state = AppState::shared(support::DB.clone(), Samples::default())
        .with_session(session)
        .with_clock(Arc::new(support::ConsoleClock(sim.clock().clone())))
        .with_link(duplex(&service))
        .with_live()
        .with_imaging(root)
        .with_imaging_solver(Arc::new(SimulatedSolver::new(sim.clone())));
    let imaging = state.imaging().expect("the rig's imaging state");
    Rig {
        app: finder::router(state),
        imaging,
    }
}

async fn get(app: &Router, path: &str) -> (StatusCode, Value) {
    support::request(app, "GET", path, json!(null)).await
}

async fn post(app: &Router, path: &str, body: Value) -> (StatusCode, Value) {
    support::request(app, "POST", path, body).await
}

async fn post_preview(app: &Router) -> (StatusCode, Value) {
    support::request(app, "POST", "/api/imaging/preview", json!(null)).await
}

async fn put(app: &Router, body: Value) -> (StatusCode, Value) {
    support::request(app, "PUT", "/api/imaging/settings", body).await
}

/// Read `GET /api/imaging` every 100 ms until `check` holds, for at most
/// 120 s of paused time.
async fn wait_until(app: &Router, check: impl Fn(&Value) -> bool) -> Value {
    for _ in 0..1200 {
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

/// Every file under `root`, as relative paths, sorted; a folder that is not
/// there yet holds none.
fn files(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(folder) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&folder) else {
            continue;
        };
        for entry in entries {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else {
                found.push(path.strip_prefix(root).unwrap().display().to_string());
            }
        }
    }
    found.sort();
    found
}

/// A GET whose body is bytes, with the answer's content type.
async fn get_bytes(app: &Router, path: &str) -> (StatusCode, Option<String>, Vec<u8>) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(path)
                .body(Body::from(""))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let kind = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap()
        .to_vec();
    (status, kind, body)
}

/// A write a browser on another site sends: a foreign `Origin`, with a `Host`
/// of localhost on port 8080 as the same-origin rule reads it.
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

/// US1-S1: a preview frame is answered as a frame asked to be kept is, with
/// the purpose `preview`, ends `shown` with its preview and none of a saved
/// frame's keys, answers a 618 × 411 JPEG, and writes nothing, at the console
/// or at the node; a saved frame after it takes the next id and saves its file
/// and the node's copy.
#[tokio::test(start_paused = true)]
async fn a_preview_frame_is_shown_and_saved_nowhere() {
    let scratch = Scratch::new("imaging-preview-frames-shown");
    let root = scratch.0.join("imaging");
    let node = scratch.0.join("node");
    let rig = rig(&root, &node);
    ready(&rig.app).await;
    let (status, _) = put(&rig.app, json!({"exposure_ms":200,"save_at_node":true})).await;
    assert_eq!(status, StatusCode::OK);

    let (status, answer) = post_preview(&rig.app).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let frame = &answer["frame"];
    assert_eq!(frame["id"], json!(1));
    assert_eq!(frame["purpose"], json!("preview"));
    assert_eq!(frame["state"], json!("exposing"));

    let view = wait_until(&rig.app, |view| {
        view["frame"]["id"] == json!(1) && view["frame"]["state"] == "shown"
    })
    .await;
    let frame = &view["frame"];
    assert_eq!(frame["width"], json!(1236), "{frame}");
    assert_eq!(frame["height"], json!(822), "{frame}");
    assert_eq!(
        frame["preview"],
        json!("/api/imaging/frames/1/preview.jpg"),
        "{frame}"
    );
    for key in ["file", "saved_at", "node_file", "node_save_error", "error"] {
        assert!(frame.get(key).is_none(), "{key} in {frame}");
    }

    let (status, kind, body) = get_bytes(&rig.app, "/api/imaging/frames/1/preview.jpg").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(kind.as_deref(), Some("image/jpeg"));
    let decoded = image::load_from_memory(&body).unwrap();
    assert_eq!(decoded.dimensions(), (618, 411));

    // Nothing was written, at the console or at the node.
    assert_eq!(files(&root), Vec::<String>::new());
    assert_eq!(files(&node), Vec::<String>::new());

    // A saved frame after it: the next id, its file and the node's copy.
    let (status, _) = post(&rig.app, "/api/imaging/frame", json!(null)).await;
    assert_eq!(status, StatusCode::OK);
    let view = wait_until(&rig.app, |view| {
        view["frame"]["id"] == json!(2) && view["frame"]["state"] == "saved"
    })
    .await;
    let node_file = view["frame"]["node_file"]
        .as_str()
        .expect("the saved frame's node_file")
        .to_string();
    assert_eq!(
        files(&root),
        vec!["frames/2026-01-16/000002.fits".to_string()]
    );
    assert_eq!(files(&node), vec![node_file]);
}

/// US1-S2: a preview frame holds no lease and is refused as any single frame:
/// the lease's and the frame-in-flight words, the settings' and the focus
/// aid's, a foreign write's 403, the solve's refusal of a frame with no file,
/// and a console with no imaging; and nothing is written.
#[tokio::test(start_paused = true)]
async fn a_preview_frame_holds_no_lease_and_refuses_as_a_single_frame() {
    let scratch = Scratch::new("imaging-preview-frames-refusals");
    let root = scratch.0.join("imaging");
    let node = scratch.0.join("node");
    let rig = rig(&root, &node);
    ready(&rig.app).await;
    put(&rig.app, json!({"exposure_ms":2000})).await;

    let lease = rig.imaging.lease("the stack").unwrap();
    let (status, answer) = post_preview(&rig.app).await;
    assert_eq!(status, StatusCode::CONFLICT, "{answer}");
    assert_eq!(
        answer["error"],
        json!("the imaging camera is held by the stack"),
        "{answer}"
    );
    drop(lease);

    let (status, answer) = post_preview(&rig.app).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    assert_eq!(answer["frame"]["id"], json!(1), "{answer}");

    // A preview frame in flight refuses every other start, and a settings
    // change, with 031's and 033's words.
    let (status, answer) = post_preview(&rig.app).await;
    assert_eq!(status, StatusCode::CONFLICT, "{answer}");
    assert_eq!(answer["error"], json!("an exposure is already in progress"));
    let (status, answer) = post(&rig.app, "/api/imaging/frame", json!(null)).await;
    assert_eq!(status, StatusCode::CONFLICT, "{answer}");
    assert_eq!(answer["error"], json!("an exposure is already in progress"));
    let (status, answer) = put(&rig.app, json!({"exposure_ms":100})).await;
    assert_eq!(status, StatusCode::CONFLICT, "{answer}");
    assert_eq!(answer["error"], json!("an exposure is in progress"));
    let (status, answer) = post(&rig.app, "/api/imaging/focus", json!({})).await;
    assert_eq!(status, StatusCode::CONFLICT, "{answer}");
    assert_eq!(answer["error"], json!("an exposure is in progress"));
    assert_eq!(rig.imaging.holder(), None);

    // A foreign write is refused under the same-origin rule.
    let (status, answer) = foreign_origin(&rig.app, "POST", "/api/imaging/preview").await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{answer}");

    wait_until(&rig.app, |view| view["frame"]["state"] == "shown").await;

    // A shown frame has no file, so the solve refuses it with 032's words.
    let (status, answer) = post(&rig.app, "/api/imaging/solve", json!(null)).await;
    assert_eq!(status, StatusCode::CONFLICT, "{answer}");
    assert_eq!(
        answer["error"],
        json!("the latest imaging frame was not saved"),
        "{answer}"
    );

    // A console with no imaging answers its own words.
    let bare = finder::router(AppState::shared(support::DB.clone(), Samples::default()));
    let (status, answer) = post_preview(&bare).await;
    assert_eq!(status, StatusCode::CONFLICT, "{answer}");
    assert_eq!(
        answer["error"],
        json!("this console takes no imaging frames"),
        "{answer}"
    );

    assert_eq!(files(&root), Vec::<String>::new());
}

/// US1-S3: an abort ends a preview frame `aborted` with no preview, the next
/// preview frame takes the id after it, and nothing is written.
#[tokio::test(start_paused = true)]
async fn an_aborted_preview_frame_has_no_preview() {
    let scratch = Scratch::new("imaging-preview-frames-abort");
    let root = scratch.0.join("imaging");
    let node = scratch.0.join("node");
    let rig = rig(&root, &node);
    ready(&rig.app).await;
    put(&rig.app, json!({"exposure_ms":5000})).await;

    let (status, answer) = post_preview(&rig.app).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    assert_eq!(answer["frame"]["id"], json!(1), "{answer}");
    // One second into the exposure: the frame is still in flight (plan trap 8).
    tokio::time::sleep(Duration::from_secs(1)).await;
    let (status, _) = post(&rig.app, "/api/imaging/abort", json!(null)).await;
    assert_eq!(status, StatusCode::OK);

    let view = wait_until(&rig.app, |view| view["frame"]["state"] == "aborted").await;
    assert!(view["frame"].get("preview").is_none(), "{view}");

    put(&rig.app, json!({"exposure_ms":100})).await;
    let (status, answer) = post_preview(&rig.app).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    assert_eq!(answer["frame"]["id"], json!(2), "{answer}");
    let view = wait_until(&rig.app, |view| {
        view["frame"]["id"] == json!(2) && view["frame"]["state"] == "shown"
    })
    .await;
    assert_eq!(
        view["frame"]["preview"],
        json!("/api/imaging/frames/2/preview.jpg"),
        "{view}"
    );

    assert_eq!(files(&root), Vec::<String>::new());
}

/// US1-S4: ADR 0002 names the preview frame's amendment (oracle: the ADR's
/// text, spec 058 FR-002).
#[test]
fn the_adr_amends_its_categorical_save() {
    let text =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "../../docs/adr/0002-imaging-cameras-through-indi-frames-relayed-console-stacks.md",
        ))
        .unwrap();
    assert!(
        text.contains(
            "Amended by spec 058: the console saves every frame taken to be kept; \
             a preview frame is shown and saved nowhere."
        ),
        "ADR 0002 holds no amendment of spec 058"
    );
}
