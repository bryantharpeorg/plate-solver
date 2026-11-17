//! Oracles: independently received star frames and live reports (JSON-round-trip
//! exact), the simulated UTC epoch, and a fresh solve (0.01 deg, 0.05 s).
use crate::support::live as support;
use axum::{
    body::{to_bytes, Body},
    http::Request,
    Router,
};
use finder::{
    link::{LinkStatus, Transport},
    live::{DbSolver, Live},
    recording::{Recorder, Sources, FRAME_LIMIT},
    samples::Samples,
    session::{Aim, FixedClock, Session},
    AppState,
};
use scope_link::{
    memory,
    proto::{self, StarFrame},
};
use serde::Serialize;
use serde_json::{json, Value};
use simulator::{mount::SimMount, SimNode};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::sync::{broadcast, watch, Semaphore};
use tower::ServiceExt;

fn state() -> AppState {
    AppState::shared(support::DB.clone(), Samples::default())
        .with_clock(Arc::new(FixedClock(support::START)))
}
fn setup(root: &Path) -> AppState {
    let sim = support::sim();
    let mount = SimMount::new(sim.clock().clone());
    sim.connect_mount(mount);
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
    state()
        .with_session(session)
        .with_clock(Arc::new(support::ConsoleClock(sim.clock().clone())))
        .with_link(transport)
        .with_live()
        .with_recordings(root)
}
async fn read(app: &Router) -> Value {
    support::request(app, "GET", "/api/session", Value::Null)
        .await
        .1
}
async fn post(app: &Router, on: bool) -> Value {
    let (status, body) = support::request(app, "POST", "/api/record", json!({"on":on})).await;
    assert_eq!(status, 200, "{body}");
    body
}
async fn wait(app: &Router, predicate: impl Fn(&Value) -> bool) -> Value {
    for _ in 0..600 {
        let body = read(app).await;
        if predicate(&body) {
            return body;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("session never reached expected state: {}", read(app).await);
}
fn round(value: impl Serialize) -> Value {
    serde_json::from_str(&serde_json::to_string(&value).unwrap()).unwrap()
}
fn lines(folder: &Path, name: &str) -> Vec<Value> {
    text_lines(folder, name)
        .iter()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}
fn text_lines(folder: &Path, name: &str) -> Vec<String> {
    std::fs::read_to_string(folder.join(name))
        .unwrap_or_else(|e| panic!("{name}: {e}"))
        .lines()
        .map(str::to_owned)
        .collect()
}
fn metadata(folder: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(folder.join("recording.json")).unwrap()).unwrap()
}
fn previews(folder: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(folder)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            p.file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("preview-")
                && p.extension().is_some_and(|e| e == "jpg")
        })
        .collect()
}
fn folder(root: &Path, session: &Value) -> PathBuf {
    root.join(session["recording"]["folder"].as_str().unwrap())
}
fn settings(value: &Option<proto::Settings>) -> Value {
    value.as_ref().map_or(Value::Null, |s| {
        json!({
            "exposure_ms":s.exposure_ms,"gain":s.gain,
            "detection":s.detection.as_ref().map(|d| json!({"sigma":d.sigma,
                "binning":d.binning,"normalize_rows":d.normalize_rows,
                "detect_hot_pixels":d.detect_hot_pixels}))
        })
    })
}
fn axis(value: &Option<proto::AxisPositions>) -> Value {
    value.as_ref().map_or(
        Value::Null,
        |p| json!({"azimuth":p.azimuth,"altitude":p.altitude}),
    )
}

#[tokio::test(start_paused = true)]
async fn start_stop_metadata_counts_arrival_times_and_no_writes_after_stop() {
    let root = crate::support::Scratch::new("recording-metadata");
    let state = setup(&root.0);
    let app = finder::router(state.clone());
    wait(&app, |s| !s["model"].is_null()).await;
    let frames = state.link().unwrap().frames();
    let before = loop {
        if let Some(frame) = frames.borrow().as_ref() {
            break frame.id;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    };
    let start = post(&app, true).await;
    assert_eq!(start["recording"]["on"], true);
    let name = start["recording"]["folder"].as_str().unwrap();
    assert!(name.starts_with("2026-01-16T02-"));
    assert_eq!(name.len(), 20);
    let dir = folder(&root.0, &start);
    let initial = metadata(&dir);
    assert_eq!(initial["format"], "finder-recording/1");
    let health = support::request(&app, "GET", "/api/health", Value::Null)
        .await
        .1;
    assert_eq!(initial["finder"], health["version"]);
    assert_eq!(initial["site"], json!({"lat_deg":40.0,"lon_deg":-75.0}));
    assert_eq!(initial["started_at"], start["recording"]["since"]);
    let utc = initial["started_utc"].as_str().unwrap();
    assert_eq!(
        finder::simulate::parse_start(utc).unwrap(),
        initial["started_at"].as_f64().unwrap().floor()
    );
    assert_eq!(name, utc.replace(':', "-"));
    assert_eq!(initial["aim"], Value::Null);
    assert!(!initial["model"].is_null());
    for field in ["model", "camera_settings"] {
        assert_eq!(initial[field], start[field]);
    }
    assert_eq!(initial["camera"], start["node"]["camera"]);
    wait(&app, |s| {
        s["recording"]["solves"].as_u64().unwrap_or(0) >= 2
    })
    .await;
    let stop = post(&app, false).await;
    assert_eq!(stop["recording"]["on"], false);
    assert_eq!(stop["recording"]["ended_reason"], "stopped");
    let meta = metadata(&dir);
    assert_eq!(meta["ended_reason"], "stopped");
    assert!(meta["stopped_at"].as_f64().unwrap() >= meta["started_at"].as_f64().unwrap());
    let frames = lines(&dir, "frames.jsonl");
    let solves = lines(&dir, "solves.jsonl");
    assert!(!frames.is_empty());
    assert!(solves.len() >= 2);
    for (key, count) in [
        ("frames", frames.len()),
        ("solves", solves.len()),
        ("previews", previews(&dir).len()),
    ] {
        assert_eq!(meta[key], count);
        assert_eq!(stop["recording"][key], count);
    }
    let ids: Vec<_> = frames.iter().map(|f| f["id"].as_u64().unwrap()).collect();
    assert!(ids[0] > before);
    assert!(ids.windows(2).all(|w| w[0] < w[1]));
    for frame in &frames {
        let at = frame["received_at"].as_f64().unwrap();
        assert!(at >= start["recording"]["since"].as_f64().unwrap());
        assert!(at <= stop["time"].as_f64().unwrap());
    }
    assert!(solves
        .iter()
        .all(|s| ids.contains(&s["frame_id"].as_u64().unwrap())));
    tokio::time::sleep(Duration::from_millis(1500)).await;
    assert_eq!(lines(&dir, "frames.jsonl"), frames);
    assert_eq!(lines(&dir, "solves.jsonl"), solves);
    for file in std::fs::read_dir(&dir).unwrap() {
        let bytes = std::fs::read(file.unwrap().path()).unwrap();
        for secret in [b"scope.local".as_slice(), b"test-token".as_slice()] {
            assert!(!bytes.windows(secret.len()).any(|w| w == secret));
        }
    }
}

#[tokio::test(start_paused = true)]
async fn files_equal_independently_observed_frames_reports_and_previews() {
    let root = crate::support::Scratch::new("recording-payloads");
    let state = setup(&root.0);
    let mut incoming = state.link().unwrap().frames();
    let mut reports = state.live().unwrap().subscribe();
    let observed = Arc::new(std::sync::Mutex::new(HashMap::new()));
    let saved = observed.clone();
    let observer = tokio::spawn(async move {
        while incoming.changed().await.is_ok() {
            if let Some(frame) = incoming.borrow_and_update().clone() {
                saved.lock().unwrap().insert(frame.id, frame);
            }
        }
    });
    let app = finder::router(state);
    post(&app, true).await;
    wait(&app, |s| {
        s["recording"]["previews"].as_u64().unwrap_or(0) >= 1
            && s["recording"]["frames"].as_u64().unwrap_or(0) >= 3
    })
    .await;
    let stop = post(&app, false).await;
    observer.abort();
    let dir = folder(&root.0, &stop);
    let mut report_json = HashMap::new();
    while let Ok(report) = reports.try_recv() {
        report_json.insert(report.frame_id, serde_json::to_string(&report).unwrap());
    }
    let observed = observed.lock().unwrap();
    let frames = lines(&dir, "frames.jsonl");
    let mut compared = 0;
    for line in &frames {
        let Some(frame) = observed.get(&line["id"].as_u64().unwrap()) else {
            continue;
        };
        compared += 1;
        let expected = round(
            json!({"id":frame.id,"width":frame.width,"height":frame.height,
            "start_s":frame.start_s,"end_s":frame.end_s,"exposure_ms":frame.exposure_ms,
            "gain":frame.gain,"settings":settings(&frame.settings),
            "axis_start":axis(&frame.axis_start),"axis_end":axis(&frame.axis_end),
            "centroids":frame.centroids.iter().map(|c| [c.x,c.y,c.brightness]).collect::<Vec<_>>() }),
        );
        assert!(!line["axis_start"].is_null());
        for (key, value) in expected.as_object().unwrap() {
            assert_eq!(&line[key], value, "{key}");
        }
        match &frame.preview_jpeg {
            Some(bytes) => {
                assert_eq!(line["preview"], format!("preview-{}.jpg", frame.id));
                assert_eq!(
                    &std::fs::read(dir.join(line["preview"].as_str().unwrap())).unwrap(),
                    bytes
                );
            }
            None => assert!(line.get("preview").is_none()),
        }
    }
    assert!(compared >= 2);
    let solves = text_lines(&dir, "solves.jsonl");
    assert!(!solves.is_empty());
    for line in solves {
        let value: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(
            &line,
            report_json
                .get(&value["frame_id"].as_u64().unwrap())
                .unwrap()
        );
    }
    assert_eq!(
        previews(&dir).len(),
        stop["recording"]["previews"].as_u64().unwrap() as usize
    );
}

fn restore_frame(line: &Value, dir: &Path) -> StarFrame {
    let positions = |v: &Value| {
        (!v.is_null()).then(|| proto::AxisPositions {
            azimuth: v["azimuth"].as_i64().unwrap() as i32,
            altitude: v["altitude"].as_i64().unwrap() as i32,
        })
    };
    let s = &line["settings"];
    let d = &s["detection"];
    StarFrame {
        id: line["id"].as_u64().unwrap(),
        start_s: line["start_s"].as_f64().unwrap(),
        end_s: line["end_s"].as_f64().unwrap(),
        width: line["width"].as_u64().unwrap() as u32,
        height: line["height"].as_u64().unwrap() as u32,
        exposure_ms: line["exposure_ms"].as_f64().unwrap(),
        gain: line["gain"].as_f64().unwrap(),
        settings: (!s.is_null()).then(|| proto::Settings {
            exposure_ms: s["exposure_ms"].as_f64(),
            gain: s["gain"].as_f64(),
            detection: (!d.is_null()).then(|| proto::Detection {
                sigma: d["sigma"].as_f64().unwrap(),
                binning: d["binning"].as_u64().unwrap() as u32,
                normalize_rows: d["normalize_rows"].as_bool().unwrap(),
                detect_hot_pixels: d["detect_hot_pixels"].as_bool().unwrap(),
            }),
        }),
        centroids: line["centroids"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| proto::Centroid {
                x: c[0].as_f64().unwrap(),
                y: c[1].as_f64().unwrap(),
                brightness: c[2].as_f64().unwrap(),
            })
            .collect(),
        axis_start: positions(&line["axis_start"]),
        axis_end: positions(&line["axis_end"]),
        preview_jpeg: line["preview"]
            .as_str()
            .map(|name| std::fs::read(dir.join(name)).unwrap()),
    }
}

#[tokio::test(start_paused = true)]
async fn saved_star_frame_replays_through_a_fresh_live_solver() {
    let root = crate::support::Scratch::new("recording-replay");
    let state = setup(&root.0);
    let app = finder::router(state);
    wait(&app, |s| !s["model"].is_null()).await;
    post(&app, true).await;
    wait(&app, |s| {
        s["recording"]["solves"].as_u64().unwrap_or(0) >= 2
    })
    .await;
    let stopped = post(&app, false).await;
    let dir = folder(&root.0, &stopped);
    let solves = lines(&dir, "solves.jsonl");
    let frames = lines(&dir, "frames.jsonl");
    let (line, solve) = frames
        .iter()
        .find_map(|f| {
            solves
                .iter()
                .find(|s| s["frame_id"] == f["id"] && !s["pointing_seq"].is_null())
                .map(|s| (f, s))
        })
        .expect("a solved recorded star frame");
    let (tx, rx) = watch::channel(None);
    let (_status_tx, status) = watch::channel(LinkStatus {
        connected: true,
        offset_s: line["offset_s"].as_f64(),
        ..Default::default()
    });
    let live = Live::start(
        Arc::new(Session::default()),
        Arc::new(FixedClock(line["received_at"].as_f64().unwrap())),
        rx,
        status,
        Arc::new(DbSolver(support::DB.clone())),
        Arc::new(Semaphore::new(1)),
        (10.0, 30.0),
    );
    let mut reports = live.subscribe();
    tx.send_replace(Some(restore_frame(line, &dir)));
    let report = support::event(&mut reports).await;
    let pointing = report.report.pointing.unwrap();
    assert!(
        (pointing.ra_deg - solve["report"]["pointing"]["ra_deg"].as_f64().unwrap()).abs() < 0.01
    );
    assert!(
        (pointing.dec_deg - solve["report"]["pointing"]["dec_deg"].as_f64().unwrap()).abs() < 0.01
    );
    assert!((report.time - solve["time"].as_f64().unwrap()).abs() < 0.05);
}

#[tokio::test(start_paused = true)]
async fn invalid_requests_and_missing_recording_sources_leave_no_recording() {
    let scratch = crate::support::Scratch::new("recording-refusals");
    let root = scratch.0.join("root");
    let app = finder::router(setup(&root));
    for body in [
        json!({}),
        json!({"on":"yes"}),
        json!({"on":true,"raw":true}),
    ] {
        let (status, value) = support::request(&app, "POST", "/api/record", body).await;
        assert_eq!(status, 400);
        assert!(value["error"].is_string());
    }
    for (content, origin, expected) in [
        ("application/json", Some("http://foreign.example"), 403),
        ("text/plain", None, 415),
    ] {
        let mut request = Request::post("/api/record")
            .header("host", "localhost:8080")
            .header("content-type", content);
        if let Some(origin) = origin {
            request = request.header("origin", origin);
        }
        let response = app
            .clone()
            .oneshot(request.body(Body::from("{\"on\":true}")).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
        let value: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert!(value["error"].is_string());
    }
    assert!(read(&app).await.get("recording").is_none());
    assert!(!root.exists());
    assert!(post(&app, false).await.get("recording").is_none());
    for (app, error) in [
        (
            finder::router(state()),
            "this console has no recordings folder",
        ),
        (
            finder::router(state().with_recordings(&root)),
            "this console has no live link",
        ),
    ] {
        let (status, value) =
            support::request(&app, "POST", "/api/record", json!({"on":true})).await;
        assert_eq!(status, 409);
        assert_eq!(value["error"], error);
        assert!(read(&app).await.get("recording").is_none());
    }
    assert!(!root.exists());
    std::fs::write(&root, b"a regular file").unwrap();
    let app = finder::router(setup(&root));
    let (status, value) = support::request(&app, "POST", "/api/record", json!({"on":true})).await;
    assert_eq!(status, 500);
    assert!(value["error"]
        .as_str()
        .unwrap()
        .contains("recording folder"));
    assert!(read(&app).await.get("recording").is_none());
}

#[tokio::test(start_paused = true)]
async fn repeated_start_keeps_folder_and_next_start_uses_a_new_folder() {
    let root = crate::support::Scratch::new("recording-starts");
    let app = finder::router(setup(&root.0));
    let first = post(&app, true).await;
    let second = post(&app, true).await;
    assert_eq!(first["recording"]["folder"], second["recording"]["folder"]);
    post(&app, false).await;
    let third = post(&app, true).await;
    assert_ne!(first["recording"]["folder"], third["recording"]["folder"]);
    // No virtual time has elapsed: this also exercises the -2 collision suffix.
    assert_eq!(
        third["recording"]["folder"],
        format!("{}-2", first["recording"]["folder"].as_str().unwrap())
    );
    assert!(folder(&root.0, &first).is_dir());
    assert!(folder(&root.0, &third).is_dir());
    post(&app, false).await;
}

#[tokio::test(start_paused = true)]
async fn frame_limit_and_deleted_folder_end_without_extra_lines_or_recreation() {
    let root = crate::support::Scratch::new("recording-limit");
    let app = finder::router(setup(&root.0).with_recording_limit(3));
    post(&app, true).await;
    let ended = wait(&app, |s| s["recording"]["on"] == false).await;
    assert_eq!(ended["recording"]["ended_reason"], "limit");
    assert_eq!(ended["recording"]["frames"], 3);
    assert_eq!(lines(&folder(&root.0, &ended), "frames.jsonl").len(), 3);
    let root = crate::support::Scratch::new("recording-deleted");
    let app = finder::router(setup(&root.0));
    post(&app, true).await;
    let running = wait(&app, |s| {
        s["recording"]["frames"].as_u64().unwrap_or(0) >= 1
    })
    .await;
    let dir = folder(&root.0, &running);
    std::fs::remove_dir_all(&dir).unwrap();
    let ended = tokio::time::timeout(
        Duration::from_secs(5),
        wait(&app, |s| s["recording"]["on"] == false),
    )
    .await
    .unwrap();
    assert_eq!(ended["recording"]["ended_reason"], "write_failed");
    assert!(!dir.exists());
}

async fn recorder_wait(
    recorder: &Recorder,
    predicate: impl Fn(&finder::recording::RecordingView) -> bool,
) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if predicate(&recorder.view().unwrap()) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .unwrap();
}
fn own_sources(
    session: Arc<Session>,
) -> (
    watch::Sender<Option<StarFrame>>,
    watch::Sender<LinkStatus>,
    broadcast::Sender<finder::live::LiveReport>,
    Sources,
) {
    let (frames_tx, frames) = watch::channel(None);
    let (status_tx, status) = watch::channel(LinkStatus::default());
    let (reports_tx, reports) = broadcast::channel(16);
    (
        frames_tx,
        status_tx,
        reports_tx,
        Sources {
            frames,
            status,
            reports,
            clock: Arc::new(FixedClock(support::START)),
            session,
        },
    )
}

#[tokio::test(start_paused = true)]
async fn lower_or_equal_ids_end_before_writing_and_none_keeps_recording_on() {
    let root = crate::support::Scratch::new("recording-restart");
    let recorder = Recorder::new(root.0.clone(), FRAME_LIMIT);
    let session = Arc::new(Session::default());
    session
        .set_aim(Some(Aim { x: 700.0, y: 420.0 }), support::START)
        .unwrap();
    let (tx, _status, _reports, sources) = own_sources(session.clone());
    recorder.start(sources).unwrap();
    tx.send_replace(Some(StarFrame {
        id: 5,
        preview_jpeg: Some(vec![1, 2, 3]),
        ..Default::default()
    }));
    recorder_wait(&recorder, |v| v.frames == 1).await;
    tx.send_replace(Some(StarFrame {
        id: 6,
        ..Default::default()
    }));
    recorder_wait(&recorder, |v| v.frames == 2).await;
    tx.send_replace(None);
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert!(recorder.view().unwrap().on);
    assert_eq!(recorder.view().unwrap().frames, 2);
    tx.send_replace(Some(StarFrame {
        id: 5,
        preview_jpeg: Some(vec![9, 9, 9]),
        ..Default::default()
    }));
    recorder_wait(&recorder, |v| !v.on).await;
    let view = recorder.view().unwrap();
    assert_eq!(view.ended_reason, Some("node_restarted"));
    assert_eq!(view.frames, 2);
    assert_eq!(view.solves, 0);
    assert_eq!(view.previews, 1);
    let dir = root.0.join(&view.folder);
    let frames = lines(&dir, "frames.jsonl");
    assert_eq!(
        frames
            .iter()
            .map(|f| f["id"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        vec![5, 6]
    );
    assert_eq!(previews(&dir).len(), 1);
    assert_eq!(
        std::fs::read(dir.join("preview-5.jpg")).unwrap(),
        vec![1, 2, 3]
    );
    let meta = metadata(&dir);
    assert_eq!(meta["aim"], json!({"x":700.0,"y":420.0}));
    assert_eq!(meta["ended_reason"], "node_restarted");
    assert_eq!(meta["stopped_at"], support::START);
    assert_eq!(meta["frames"], 2);
    assert_eq!(meta["solves"], 0);
    assert_eq!(meta["previews"], 1);
    assert!(frames
        .iter()
        .all(|f| f["offset_s"].is_null() && f["settings"].is_null()));
    tx.send_replace(Some(StarFrame {
        id: 7,
        ..Default::default()
    }));
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert_eq!(lines(&dir, "frames.jsonl"), frames);
    let (tx, _status, _reports, sources) = own_sources(session);
    let second = recorder.start(sources).unwrap();
    assert_ne!(second.folder, view.folder);
    tx.send_replace(Some(StarFrame {
        id: 3,
        ..Default::default()
    }));
    recorder_wait(&recorder, |v| v.frames == 1).await;
    tx.send_replace(Some(StarFrame {
        id: 3,
        ..Default::default()
    }));
    recorder_wait(&recorder, |v| !v.on).await;
    assert_eq!(
        recorder.view().unwrap().ended_reason,
        Some("node_restarted")
    );
    assert_eq!(lines(&root.0.join(second.folder), "frames.jsonl").len(), 1);
}
