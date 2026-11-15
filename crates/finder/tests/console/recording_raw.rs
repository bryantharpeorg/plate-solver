//! Oracles: PNG decoding and cedar detection compared exactly with independently
//! parsed frame lines after a JSON round trip; literal route and error contracts.
use crate::support::live as support;
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use finder::{link::Transport, samples::Samples, session::Session, AppState};
use scope_link::{memory, proto};
use scope_node::{
    camera::FrameFuture, clock::NodeClock, service::NodeService, Camera, CameraInfo, GrayFrame,
    RangeError, Settings,
};
use serde_json::{json, Value};
use simulator::{mount::SimMount, SimNode, Simulator, VirtualClock};
use std::{
    io,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tower::ServiceExt;

struct Folder(PathBuf);
impl Folder {
    fn new(name: &str) -> Self {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("../../target/raw-{name}-{}", std::process::id()));
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
fn simulator() -> Simulator {
    let sim = support::sim();
    sim.connect_mount(SimMount::new(sim.clock().clone()));
    sim.push_to(50.0, 120.0);
    sim
}
fn state(sim: &Simulator, transport: Transport, root: Option<&Path>) -> AppState {
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
        .with_live();
    match root {
        Some(root) => state.with_recordings(root),
        None => state,
    }
}
#[derive(Clone, Copy)]
enum Service {
    Current,
    Legacy,
    Malformed,
}
fn setup(root: Option<&Path>, service: Service) -> (Router, Simulator) {
    let sim = simulator();
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
                let node = node.service();
                match service {
                    Service::Current => memory::serve_routes_on(server, node.routes())
                        .await
                        .unwrap(),
                    Service::Legacy => memory::serve_on(server, node.into_server()).await.unwrap(),
                    Service::Malformed => {
                        let routes = tonic::service::Routes::new(node.into_server()).add_service(
                            proto::scope_node_raw_server::ScopeNodeRawServer::with_interceptor(
                                Malformed,
                                scope_link::TokenInterceptor::new("test-token"),
                            ),
                        );
                        memory::serve_routes_on(server, routes).await.unwrap();
                    }
                }
            });
            Ok::<_, io::Error>(client)
        }
    });
    (finder::router(state(&sim, transport, root)), sim)
}
async fn request(app: &Router, method: &str, path: &str, value: Value) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("host", "localhost:8080")
                .header("content-type", "application/json")
                .body(Body::from(value.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let body =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    (status, body)
}
async fn read(app: &Router) -> Value {
    request(app, "GET", "/api/session", Value::Null).await.1
}
async fn wait(app: &Router, predicate: impl Fn(&Value) -> bool) -> Value {
    for _ in 0..300 {
        let value = read(app).await;
        if predicate(&value) {
            return value;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("session did not reach expected state: {}", read(app).await);
}
async fn connected(app: &Router) {
    wait(app, |v| {
        v["node"]["connected"] == true && v["node"]["offset_s"].is_number()
    })
    .await;
}
async fn record(app: &Router, on: bool) -> Value {
    let (status, value) = request(app, "POST", "/api/record", json!({"on":on})).await;
    assert_eq!(status, 200, "{value}");
    value
}
async fn raw(app: &Router, frames: u8) -> (StatusCode, Value) {
    request(app, "POST", "/api/record/raw", json!({"frames":frames})).await
}
fn folder(root: &Folder, value: &Value) -> PathBuf {
    root.0.join(value["recording"]["folder"].as_str().unwrap())
}
fn raw_files(dir: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(dir)
        .unwrap()
        .map(|p| p.unwrap().path())
        .filter(|p| p.file_name().unwrap().to_str().unwrap().starts_with("raw-"))
        .collect()
}
fn no_parts(dir: &Path) {
    assert!(std::fs::read_dir(dir).unwrap().all(|p| p
        .unwrap()
        .path()
        .extension()
        .is_none_or(|s| s != "part")));
}
async fn refused(app: &Router, error: &str) {
    let (status, body) = raw(app, 1).await;
    assert_eq!(status, 409, "{body}");
    assert_eq!(body["error"], error);
}

#[tokio::test(start_paused = true)]
async fn pngs_preserve_the_pixels_that_produced_each_frame_lines_centroids() {
    let root = Folder::new("pixels");
    let (app, _sim) = setup(Some(&root.0), Service::Current);
    connected(&app).await;
    record(&app, true).await;
    let (status, value) = raw(&app, 2).await;
    assert_eq!(status, 200, "{value}");
    let saved = wait(&app, |v| v["recording"]["raw"]["saved"] == 2).await;
    assert_eq!(saved["recording"]["raw"], json!({"wanted":2,"saved":2}));
    let stopped = record(&app, false).await;
    let dir = folder(&root, &stopped);
    let files = raw_files(&dir);
    assert_eq!(files.len(), 2);
    no_parts(&dir);
    let lines: Vec<Value> = std::fs::read_to_string(dir.join("frames.jsonl"))
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    for file in files {
        assert_eq!(file.extension().unwrap(), "png");
        let id: u64 = file
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
            .strip_prefix("raw-")
            .unwrap()
            .parse()
            .unwrap();
        let line = lines
            .iter()
            .find(|v| v["id"] == id)
            .expect("raw frame's line");
        let decoded = image::open(&file).unwrap();
        assert_eq!(decoded.color(), image::ColorType::L8);
        let gray = decoded.into_luma8();
        assert_eq!(
            (gray.width(), gray.height()),
            (
                line["width"].as_u64().unwrap() as u32,
                line["height"].as_u64().unwrap() as u32
            )
        );
        let d = &line["settings"]["detection"];
        let centroids: Vec<_> = star_detection::detect_stars(
            gray.as_raw(),
            gray.width() as usize,
            gray.height() as usize,
            d["sigma"].as_f64().unwrap(),
            d["binning"].as_u64().unwrap() as usize,
            d["normalize_rows"].as_bool().unwrap(),
            d["detect_hot_pixels"].as_bool().unwrap(),
        )
        .iter()
        .take(200)
        .map(|c| [c.x, c.y, c.brightness])
        .collect();
        assert!(!centroids.is_empty());
        let expected: Value =
            serde_json::from_str(&serde_json::to_string(&centroids).unwrap()).unwrap();
        assert_eq!(line["centroids"], expected);
    }
}

#[tokio::test(start_paused = true)]
async fn route_refuses_invalid_unconfigured_busy_and_disconnected_requests() {
    let root = Folder::new("refusals");
    let (app, _sim) = setup(Some(&root.0), Service::Current);
    connected(&app).await;
    refused(&app, "start a recording first").await;
    let (no_folder, _sim) = setup(None, Service::Current);
    connected(&no_folder).await;
    refused(&no_folder, "this console has no recordings folder").await;
    record(&app, true).await;
    for body in [
        json!({"frames":0}),
        json!({"frames":11}),
        json!({"frames":2.5}),
        json!({}),
        json!({"frames":1,"extra":true}),
        json!({"frames":256}),
        json!({"frames":-1}),
        json!({"frames":"1"}),
    ] {
        let (status, value) = request(&app, "POST", "/api/record/raw", body).await;
        assert_eq!(status, 400, "{value}");
        assert!(value["error"].is_string());
    }
    for (content, origin, expected) in [
        ("application/json", Some("http://elsewhere.example"), 403),
        ("text/plain", None, 415),
    ] {
        let mut req = Request::post("/api/record/raw")
            .header("host", "localhost:8080")
            .header("content-type", content);
        if let Some(origin) = origin {
            req = req.header("origin", origin);
        }
        let response = app
            .clone()
            .oneshot(req.body(Body::from("{\"frames\":1}")).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
    }
    assert_eq!(raw(&app, 10).await.0, 200);
    refused(&app, "raw frames are already being saved").await;
    record(&app, false).await;
    let transport = Transport::connector(|| async {
        Err::<tokio::io::DuplexStream, _>(io::Error::other("test connector cannot connect"))
    });
    let sim = simulator();
    let disconnected = finder::router(state(&sim, transport, Some(&root.0)));
    record(&disconnected, true).await;
    refused(&disconnected, "the scope node is not connected").await;
    record(&disconnected, false).await;
}

#[tokio::test(start_paused = true)]
async fn silent_node_times_out_after_fifteen_seconds_without_writing_a_raw_file() {
    let root = Folder::new("timeout");
    let (app, sim) = setup(Some(&root.0), Service::Current);
    connected(&app).await;
    let start = record(&app, true).await;
    sim.stop_frames();
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert_eq!(raw(&app, 1).await.0, 200);
    let ended = tokio::time::timeout(
        Duration::from_secs(20),
        wait(&app, |v| v["recording"]["raw"]["error"].is_string()),
    )
    .await
    .unwrap();
    assert_eq!(
        ended["recording"]["raw"],
        json!({"wanted":1,"saved":0,"error":"no raw frame arrived within 15 s"})
    );
    assert!(raw_files(&folder(&root, &start)).is_empty());
    record(&app, false).await;
}

#[tokio::test(start_paused = true)]
async fn ending_recording_cancels_raw_request_without_adding_an_error() {
    let root = Folder::new("stop");
    let (app, _sim) = setup(Some(&root.0), Service::Current);
    connected(&app).await;
    record(&app, true).await;
    assert_eq!(raw(&app, 10).await.0, 200);
    let stop = record(&app, false).await;
    tokio::time::sleep(Duration::from_secs(8)).await;
    let view = read(&app).await;
    let count = raw_files(&folder(&root, &stop)).len();
    assert!(count < 3);
    assert_eq!(view["recording"]["raw"]["wanted"], 10);
    assert!(view["recording"]["raw"]["saved"].as_u64().unwrap() < 3);
    assert_eq!(
        count as u64,
        view["recording"]["raw"]["saved"].as_u64().unwrap()
    );
    assert_eq!(view["recording"]["raw"], stop["recording"]["raw"]);
    assert!(view["recording"]["raw"].get("error").is_none());
    no_parts(&folder(&root, &stop));
}

#[derive(Clone)]
struct Malformed;
#[tonic::async_trait]
impl proto::scope_node_raw_server::ScopeNodeRaw for Malformed {
    async fn raw_frame(
        &self,
        _: tonic::Request<proto::RawFrameRequest>,
    ) -> Result<tonic::Response<proto::RawFrameReply>, tonic::Status> {
        Ok(tonic::Response::new(proto::RawFrameReply {
            id: 99,
            width: 4,
            height: 4,
            pixels: vec![10; 17],
        }))
    }
}
#[tokio::test(start_paused = true)]
async fn legacy_node_and_oversized_pixel_buffer_report_exact_errors_without_files() {
    let root = Folder::new("incompatible");
    for (service, error) in [
        (Service::Legacy, "this scope node cannot send raw frames"),
        (Service::Malformed, "the raw frame request failed"),
    ] {
        let (app, _sim) = setup(Some(&root.0), service);
        connected(&app).await;
        let start = record(&app, true).await;
        assert_eq!(raw(&app, 1).await.0, 200);
        let ended = wait(&app, |v| v["recording"]["raw"]["error"].is_string()).await;
        assert_eq!(
            ended["recording"]["raw"],
            json!({"wanted":1,"saved":0,"error":error})
        );
        assert!(raw_files(&folder(&root, &start)).is_empty());
        no_parts(&folder(&root, &start));
        record(&app, false).await;
    }
}

struct NodeTime(VirtualClock);
impl NodeClock for NodeTime {
    fn now(&self) -> f64 {
        self.0.node_now()
    }
}
struct LargeCamera {
    clock: VirtualClock,
}
impl Camera for LargeCamera {
    fn info(&self) -> CameraInfo {
        CameraInfo {
            model: "test gray camera".into(),
            width: 2592,
            height: 1944,
            exposure_min_ms: 1.0,
            exposure_max_ms: 2000.0,
            gain_min: 1.0,
            gain_max: 16.0,
            ..Default::default()
        }
    }
    fn apply(&mut self, settings: &Settings) -> Result<(), RangeError> {
        scope_node::camera::check_range(&self.info(), settings)
    }
    fn next_frame(&mut self) -> FrameFuture<'_> {
        Box::pin(async move {
            let start_s = self.clock.node_now();
            tokio::time::sleep(Duration::from_secs(2)).await;
            Ok(GrayFrame {
                pixels: vec![10; 2592 * 1944],
                width: 2592,
                height: 1944,
                start_s,
                end_s: self.clock.node_now(),
                exposure_ms: 200.0,
                gain: 1.0,
            })
        })
    }
}
#[tokio::test(start_paused = true)]
async fn paired_raw_client_accepts_a_five_megabyte_gray_frame() {
    let root = Folder::new("large");
    let sim = simulator();
    let node = NodeService::new(
        LargeCamera {
            clock: sim.clock().clone(),
        },
        Arc::new(NodeTime(sim.clock().clone())),
        "test-token",
        Settings::default(),
    )
    .unwrap();
    let transport = Transport::connector(move || {
        let routes = node.routes();
        async move {
            let (client, server) = tokio::io::duplex(64 * 1024);
            tokio::spawn(async move {
                memory::serve_routes_on(server, routes).await.unwrap();
            });
            Ok::<_, io::Error>(client)
        }
    });
    let app = finder::router(state(&sim, transport, Some(&root.0)));
    connected(&app).await;
    record(&app, true).await;
    assert_eq!(raw(&app, 1).await.0, 200);
    let saved = wait(&app, |v| v["recording"]["raw"]["saved"] == 1).await;
    assert_eq!(saved["recording"]["raw"], json!({"wanted":1,"saved":1}));
    let dir = folder(&root, &saved);
    let files = raw_files(&dir);
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].extension().unwrap(), "png");
    let decoded = image::open(&files[0]).unwrap();
    assert_eq!(decoded.color(), image::ColorType::L8);
    let gray = decoded.into_luma8();
    assert_eq!((gray.width(), gray.height()), (2592, 1944));
    assert!(gray.as_raw().iter().all(|v| *v == 10));
    no_parts(&dir);
    record(&app, false).await;
}
