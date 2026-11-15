//! Oracles: the simulator's mount, exact HTTP errors and saved PNG file counts.
//! Motion must interrupt a request within 30 s of virtual time, before ten saves.
use crate::support::live as support;
use axum::{
    body::{to_bytes, Body},
    http::Request,
    Router,
};
use finder::{link::Transport, samples::Samples, session::Session, AppState};
use scope_link::memory;
use scope_node::mount::{Axis, Mount};
use serde_json::{json, Value};
use simulator::{mount::SimMount, SimNode, Simulator};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tower::ServiceExt;

struct Folder(PathBuf);
impl Folder {
    fn new() -> Self {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("../../target/raw-moving-{}", std::process::id()));
        assert!(!path.exists(), "stale test folder: {}", path.display());
        Self(path)
    }
}
impl Drop for Folder {
    fn drop(&mut self) {
        if self.0.exists() {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }
}
fn setup(root: &Path) -> (Router, Simulator) {
    let sim = support::sim();
    sim.connect_mount(SimMount::new(sim.clock().clone()));
    sim.push_to(50.0, 120.0);
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
    let session = Session::default();
    session
        .edit(
            serde_json::from_value(json!({
                "node":{"address":"scope.local:50051","token":"test-token"},
                "site":{"lat_deg":40,"lon_deg":-75},
                "camera_settings":{"exposure_ms":200,"gain":1}
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
        .with_recordings(root);
    (finder::router(state), sim)
}
async fn request(app: &Router, path: &str, body: Value) -> (u16, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(if body.is_null() { "GET" } else { "POST" })
                .uri(path)
                .header("host", "localhost:8080")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status().as_u16();
    let value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    (status, value)
}
async fn wait(app: &Router, predicate: impl Fn(&Value) -> bool) -> Value {
    for _ in 0..300 {
        let value = request(app, "/api/session", Value::Null).await.1;
        if predicate(&value) {
            return value;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!(
        "session never reached expected state: {}",
        request(app, "/api/session", Value::Null).await.1
    );
}
fn raw_files(dir: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(dir)
        .unwrap()
        .map(|p| p.unwrap().path())
        .filter(|p| p.file_name().unwrap().to_str().unwrap().starts_with("raw-"))
        .collect()
}
#[tokio::test(start_paused = true)]
async fn raw_frames_are_refused_while_moving_and_a_move_ends_a_running_request() {
    let root = Folder::new();
    let (app, sim) = setup(&root.0);
    wait(&app, |v| {
        v["node"]["connected"] == true && v["node"]["offset_s"].is_number()
    })
    .await;
    let (status, start) = request(&app, "/api/record", json!({"on":true})).await;
    assert_eq!(status, 200, "{start}");
    let dir = root.0.join(start["recording"]["folder"].as_str().unwrap());
    let mount = sim.mount().unwrap();
    mount.rate(Axis::Azimuth, 1.0).await;
    wait(&app, |v| v["mount"]["moving"] == true).await;
    let (status, refused) = request(&app, "/api/record/raw", json!({"frames":1})).await;
    assert_eq!(status, 409, "{refused}");
    assert_eq!(refused["error"], "wait until the mount is still");
    assert!(raw_files(&dir).is_empty());
    mount.rate(Axis::Azimuth, 0.0).await;
    wait(&app, |v| v["mount"]["moving"] == false).await;
    let (status, accepted) = request(&app, "/api/record/raw", json!({"frames":10})).await;
    assert_eq!(status, 200, "{accepted}");
    mount.rate(Axis::Azimuth, 1.0).await;
    let ended = tokio::time::timeout(
        Duration::from_secs(30),
        wait(&app, |v| v["recording"]["raw"]["error"].is_string()),
    )
    .await
    .unwrap();
    assert_eq!(ended["recording"]["raw"]["wanted"], 10);
    assert_eq!(
        ended["recording"]["raw"]["error"],
        "the mount started moving"
    );
    let saved = ended["recording"]["raw"]["saved"].as_u64().unwrap();
    assert!(saved < 10);
    let files = raw_files(&dir);
    assert_eq!(files.len() as u64, saved);
    assert!(files
        .iter()
        .all(|p| p.extension().is_some_and(|e| e == "png")));
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert_eq!(raw_files(&dir).len() as u64, saved);
    mount.rate(Axis::Azimuth, 0.0).await;
    assert_eq!(
        request(&app, "/api/record", json!({"on":false})).await.0,
        200
    );
}
