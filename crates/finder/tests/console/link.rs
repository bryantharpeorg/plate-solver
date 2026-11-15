//! Oracles: the real simulator service, externally advanced time, cedar's 8σ
//! defaults, and the protocol's four allowed methods. Timing tolerance is 0.1 s.
use axum::{
    body::{to_bytes, Body},
    http::Request,
};
use finder::{
    link::{LinkManager, Transport},
    router,
    samples::Samples,
    session::{Clock, Session},
    AppState,
};
use pattern_database::PatternDatabase;
use scope_link::{memory, proto};
use serde_json::{json, Value};
use simulator::{SimCamera, SimNode, Simulator, Site, VirtualClock};
use std::{
    collections::BTreeSet,
    path::Path,
    sync::{Arc, LazyLock, Mutex},
    time::Duration,
};
use tonic::service::LayerExt;
use tower::{ServiceBuilder, ServiceExt};

static DB: LazyLock<Arc<PatternDatabase>> = LazyLock::new(|| {
    Arc::new(
        pattern_database::load_from_path(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../reference-solutions/cedar-solve/tetra3/data/default_database.npz"),
        )
        .unwrap(),
    )
});
struct ConsoleClock(VirtualClock);
impl Clock for ConsoleClock {
    fn now(&self) -> f64 {
        self.0.now()
    }
}
type Calls = Arc<Mutex<Vec<(String, String)>>>;

fn transport(node: Arc<SimNode>, calls: Calls) -> Transport {
    Transport::connector(move || {
        let node = node.clone();
        let calls = calls.clone();
        async move {
            let (client, server) = tokio::io::duplex(64 * 1024);
            // Record the HTTP/2 method and header before the real service's interceptor.
            let service = ServiceBuilder::new()
                .map_request(
                    move |request: tonic::codegen::http::Request<tonic::body::Body>| {
                        calls.lock().unwrap().push((
                            request.uri().path().to_owned(),
                            request
                                .headers()
                                .get("authorization")
                                .unwrap()
                                .to_str()
                                .unwrap()
                                .to_owned(),
                        ));
                        request
                    },
                )
                .into_inner()
                .named_layer(node.service().into_server());
            tokio::spawn(async move {
                memory::serve_on(server, service).await.unwrap();
            });
            Ok::<_, std::io::Error>(client)
        }
    })
}
fn session(settings: Value) -> Arc<Session> {
    let session = Arc::new(Session::default());
    session.edit(serde_json::from_value(json!({"node":{"address":"scope.local:50051","token":"test-token"},"camera_settings":settings})).unwrap(), 0.0).unwrap();
    session
}
fn simulator() -> Simulator {
    Simulator::new(
        DB.clone(),
        Site::default(),
        VirtualClock::new(1_768_528_800.0),
    )
}
async fn step() {
    tokio::time::sleep(Duration::from_millis(25)).await;
}
async fn connected(link: &LinkManager) {
    for _ in 0..240 {
        if link.status().borrow().connected {
            return;
        }
        step().await;
    }
    panic!("scope link did not connect");
}
async fn frame(link: &LinkManager) -> proto::StarFrame {
    let mut frames = link.frames();
    frames.borrow_and_update();
    tokio::time::timeout(Duration::from_secs(5), frames.changed())
        .await
        .unwrap()
        .unwrap();
    let result = frames.borrow_and_update().clone().unwrap();
    result
}

#[tokio::test(start_paused = true)]
async fn heartbeats_clock_session_and_only_the_real_protocol_calls() {
    let sim = simulator();
    let node = Arc::new(
        SimNode::new(sim.clone(), "test-token", Default::default())
            .unwrap()
            .with_frame_interval(Duration::from_millis(500)),
    );
    let calls = Calls::default();
    let state = AppState::shared(DB.clone(), Samples::default())
        .with_clock(Arc::new(ConsoleClock(sim.clock().clone())));
    let state = state.with_link(transport(node, calls.clone()));
    let link = state.link().unwrap();
    let app = router(state);
    app.clone()
        .oneshot(
            Request::put("/api/session")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"node":{"address":"scope.local:50051","token":"test-token"}})
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    for _ in 0..400 {
        step().await;
    }
    let read = app
        .oneshot(Request::get("/api/session").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let read: Value =
        serde_json::from_slice(&to_bytes(read.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert!(link.status().borrow().heartbeats >= 40);
    let latest = link.frames().borrow().clone().unwrap();
    let node = &read["node"];
    assert_eq!(node["connected"], true);
    assert!(node["last_frame_age_s"].as_f64().unwrap() <= 0.6);
    assert!((node["offset_s"].as_f64().unwrap() - (1000.0 - 1_768_528_800.0)).abs() < 1e-6);
    assert_eq!(node["camera"]["model"], "simulator");
    assert_eq!(
        node["camera"]["exposure_ms"],
        json!({"min":1.0,"max":2000.0})
    );
    assert_eq!(node["camera"]["gain"], json!({"min":1.0,"max":16.0}));
    assert_eq!(node["camera"]["width"], 1456);
    assert_eq!(node["camera"]["height"], 1088);
    assert!(node["stars"].as_u64().unwrap() > 0);
    assert_eq!(node["stars"], latest.centroids.len());
    let calls = calls.lock().unwrap();
    assert!(
        calls
            .iter()
            .filter(|(path, _)| path.ends_with("/Heartbeat"))
            .count()
            >= 40
    );
    assert!(calls
        .iter()
        .all(|(_, header)| header == "Bearer test-token"));
    assert_eq!(
        calls
            .iter()
            .map(|(p, _)| p.rsplit('/').next().unwrap())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["Describe", "Frames", "Heartbeat", "Configure"])
    );
}

#[tokio::test(start_paused = true)]
async fn silence_disconnects_at_one_second_and_reconnects_after_return() {
    let sim = simulator();
    let node = Arc::new(SimNode::new(sim.clone(), "test-token", Default::default()).unwrap());
    let session = session(json!({}));
    let link = LinkManager::start(
        session,
        Arc::new(ConsoleClock(sim.clock().clone())),
        transport(node, Calls::default()),
    );
    connected(&link).await;
    let last = link.status().borrow().last_heartbeat_at.unwrap();
    let stopped = tokio::time::Instant::now();
    sim.set_unresponsive(true);
    while link.status().borrow().connected {
        step().await;
    }
    assert!((last.elapsed().as_secs_f64() - 1.0).abs() <= 0.1);
    while stopped.elapsed() < Duration::from_secs(3) {
        step().await;
    }
    sim.set_unresponsive(false);
    let returned = tokio::time::Instant::now();
    connected(&link).await;
    assert!(returned.elapsed().as_secs_f64() <= 5.5);
    assert!(link.status().borrow().heartbeats >= 2);
}

#[tokio::test(start_paused = true)]
async fn stored_settings_apply_on_connect_and_changes_apply_without_repairing() {
    let sim = simulator();
    let node = Arc::new(SimNode::new(sim.clone(), "test-token", Default::default()).unwrap());
    let session = session(json!({"exposure_ms":200,"gain":4,"sigma":5}));
    let calls = Calls::default();
    let link = LinkManager::start(
        session.clone(),
        Arc::new(ConsoleClock(sim.clock().clone())),
        transport(node, calls.clone()),
    );
    connected(&link).await;
    let _first = frame(&link).await;
    let second = frame(&link).await;
    assert_eq!(second.exposure_ms, 200.0);
    assert_eq!(second.gain, 4.0);
    assert_eq!(second.settings.unwrap().detection.unwrap().sigma, 5.0);
    session
        .edit(
            serde_json::from_value(json!({"camera_settings":{"exposure_ms":300,"gain":"auto"}}))
                .unwrap(),
            0.0,
        )
        .unwrap();
    let _first = frame(&link).await;
    let second = frame(&link).await;
    assert_eq!(second.exposure_ms, 300.0);
    assert_eq!(second.settings.unwrap().gain, None);
    assert_eq!(
        calls
            .lock()
            .unwrap()
            .iter()
            .filter(|(p, _)| p.ends_with("/Describe"))
            .count(),
        1
    );
}

#[tokio::test(start_paused = true)]
async fn stored_outside_both_ranges_is_sent_as_auto_and_kept_in_the_session() {
    let sim = simulator();
    let mut camera = SimCamera::new(sim.clone());
    camera.set_ranges((1.0, 1000.0), (1.0, 8.0)).unwrap();
    let node = Arc::new(
        SimNode::from_camera(sim.clone(), camera, "test-token", Default::default()).unwrap(),
    );
    let session = session(json!({"exposure_ms":2000,"gain":16}));
    let link = LinkManager::start(
        session.clone(),
        Arc::new(ConsoleClock(sim.clock().clone())),
        transport(node, Calls::default()),
    );
    connected(&link).await;
    let _first = frame(&link).await;
    let second = frame(&link).await;
    let settings = second.settings.unwrap();
    assert_eq!(settings.exposure_ms, None);
    assert_eq!(settings.gain, None);
    let read = serde_json::to_value(session.snapshot(sim.clock().now())).unwrap();
    assert_eq!(read["camera_settings"]["exposure_ms"], 2000.0);
    assert_eq!(
        read["node"]["out_of_range"],
        json!([{"setting":"exposure_ms","stored":2000.0,"min":1.0,"max":1000.0},{"setting":"gain","stored":16.0,"min":1.0,"max":8.0}])
    );
}

#[tokio::test(start_paused = true)]
async fn failed_transport_uses_the_declared_backoff_and_pairing_changes_wake_it() {
    let attempts = Arc::new(Mutex::new(Vec::new()));
    let recorded = attempts.clone();
    let transport = Transport::connector(move || {
        recorded.lock().unwrap().push(tokio::time::Instant::now());
        async { Err::<tokio::io::DuplexStream, _>(std::io::Error::other("offline")) }
    });
    let session = session(json!({}));
    let link = LinkManager::start(
        session.clone(),
        Arc::new(finder::session::FixedClock(0.0)),
        transport,
    );
    for _ in 0..520 {
        step().await;
    }
    let times = attempts.lock().unwrap().clone();
    assert!(times.len() >= 6, "{} tries", times.len());
    // Oracle: the plan's exact sequence; scheduling tolerance 25 ms.
    for (pair, expected) in times.windows(2).zip([0.5, 1.0, 2.0, 4.0, 5.0]) {
        assert!(((pair[1] - pair[0]).as_secs_f64() - expected).abs() <= 0.025);
    }
    assert!(!link.status().borrow().connected);
    session
        .edit(serde_json::from_value(json!({"node":null})).unwrap(), 0.0)
        .unwrap();
    for _ in 0..240 {
        step().await;
    }
    assert_eq!(attempts.lock().unwrap().len(), times.len());
    assert!(session.snapshot(0.0).node.is_none());
}

#[tokio::test(start_paused = true)]
async fn defaults_are_cedar_and_a_stopped_frame_stream_does_not_stop_heartbeats() {
    let sim = simulator();
    let node = Arc::new(SimNode::new(sim.clone(), "test-token", Default::default()).unwrap());
    let session = Arc::new(Session::default());
    session
        .edit(
            serde_json::from_value(
                json!({"node":{"address":"scope.local:50051","token":"test-token"}}),
            )
            .unwrap(),
            0.0,
        )
        .unwrap();
    let link = LinkManager::start(
        session.clone(),
        Arc::new(ConsoleClock(sim.clock().clone())),
        transport(node, Calls::default()),
    );
    connected(&link).await;
    let latest = frame(&link).await;
    assert_eq!(
        latest.settings.unwrap(),
        proto::Settings {
            exposure_ms: None,
            gain: None,
            detection: Some(proto::Detection {
                sigma: 8.0,
                binning: 2,
                normalize_rows: true,
                detect_hot_pixels: true,
            }),
        }
    );
    assert!(session
        .snapshot(sim.clock().now())
        .camera_settings
        .is_none());
    sim.stop_frames();
    for _ in 0..80 {
        step().await;
    }
    let read = session.snapshot(sim.clock().now());
    let status = read.node.unwrap();
    assert!(status.connected);
    assert!(status.last_frame_age_s.unwrap() >= 1.5);
    assert_eq!(
        status.stars,
        link.frames()
            .borrow()
            .as_ref()
            .map(|frame| frame.centroids.len())
    );
}
