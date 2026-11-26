//! US2: live finder alignment over the simulator's truth. The independent
//! oracle is the eyepiece pixel and a 1456 × 1088 pinhole, 12° wide: 2 px
//! and 0.02° tolerances. Statuses, settings and refusal words are exact.

use std::{path::PathBuf, sync::Arc, time::Duration};

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use finder::{imaging::simulated::SimulatedSolver, samples::Samples, session::Session, AppState};
use scope_link::memory;
use serde_json::{json, Value};
use simulator::{mount::SimMount, Optics, SimNode, Simulator, Site, VirtualClock};
use tower::ServiceExt;

use crate::support::{live as support, Scratch};

pub struct Setup {
    pub live: bool,
    pub solver: bool,
    pub imaging: bool,
    pub focal_mm: Option<f64>,
    pub interval_ms: u64,
    pub folder: Option<PathBuf>,
}

impl Default for Setup {
    fn default() -> Self {
        Self {
            live: true,
            solver: true,
            imaging: true,
            focal_mm: Some(1800.0),
            interval_ms: 500,
            folder: None,
        }
    }
}

pub struct Rig {
    pub app: Router,
    pub sim: Simulator,
    _scratch: Scratch,
}

pub fn rig(e: (f64, f64), setup: Setup) -> Rig {
    let scratch = Scratch::new("imaging-align");
    let sim = Simulator::new(
        support::DB.clone(),
        Site::default(),
        VirtualClock::new(support::START),
    );
    sim.connect_mount(SimMount::new(sim.clock().clone()));
    sim.push_to(50.0, 120.0);
    sim.set_optics(Optics {
        eyepiece_px: e,
        ..Optics::default()
    });
    let node = Arc::new(
        SimNode::new(sim.clone(), "test-token", Default::default())
            .unwrap()
            .with_frame_interval(Duration::from_millis(setup.interval_ms)),
    );
    let transport = finder::link::Transport::connector(move || {
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
    });
    let session = Session::default();
    let site = Site::default();
    session
        .edit(
            serde_json::from_value(json!({
                "node": {"address": "scope.local:1", "token": "test-token"},
                "site": {"lat_deg": site.lat_deg, "lon_deg": site.lon_deg}
            }))
            .unwrap(),
            support::START,
        )
        .unwrap();
    let mut state = AppState::shared(support::DB.clone(), Samples::default())
        .with_session(session)
        .with_clock(Arc::new(support::ConsoleClock(sim.clock().clone())))
        .with_link(transport);
    if setup.live {
        state = state.with_live();
    }
    if setup.imaging {
        state = state.with_imaging(setup.folder.unwrap_or_else(|| scratch.0.join("imaging")));
    }
    if setup.solver {
        state = state.with_imaging_solver(Arc::new(SimulatedSolver::new(sim.clone())));
    }
    if let Some(mm) = setup.focal_mm {
        state = state.with_focal_length_mm(mm);
    }
    Rig {
        app: finder::router(state),
        sim,
        _scratch: scratch,
    }
}

pub async fn call(app: &Router, method: &str, path: &str, body: Value) -> (StatusCode, Value) {
    support::request(app, method, path, body).await
}

/// Every wait is bounded by 120 seconds of paused time.
pub async fn wait_until(app: &Router, check: impl Fn(&Value) -> bool) -> Value {
    for _ in 0..1200 {
        let (status, view) = call(app, "GET", "/api/imaging", json!(null)).await;
        assert_eq!(status, StatusCode::OK);
        if check(&view) {
            return view;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let (_, view) = call(app, "GET", "/api/imaging", json!(null)).await;
    panic!("imaging did not reach the awaited state: {view}");
}

pub async fn ready(app: &Router) {
    let view = wait_until(app, |v| v["state"] == "ready").await;
    assert!(view.get("align").is_none(), "{view}");
    for _ in 0..1200 {
        let (_, session) = call(app, "GET", "/api/session", json!(null)).await;
        if session["pointing"]["source"] == "live" {
            tokio::time::sleep(Duration::from_secs(5)).await;
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("the finder camera never solved live");
}

pub async fn last_after(app: &Router, after: u64) -> Value {
    let view = wait_until(app, |v| {
        v["align"]["last"]["outcome"] == "measured"
            && v["align"]["last"]["frame_id"]
                .as_u64()
                .is_some_and(|id| id > after)
    })
    .await;
    assert_eq!(view["align"]["state"], "running");
    view["align"]["last"].clone()
}

pub async fn start(app: &Router, ask: Value) -> Value {
    let (status, view) = call(app, "POST", "/api/imaging/align", ask).await;
    assert_eq!(status, StatusCode::OK, "{view}");
    assert_eq!(view["align"]["state"], "running");
    view
}

/// Pinhole arithmetic from the rig's optics, independent of `centre_of`.
pub fn oracle(e: (f64, f64)) -> (f64, f64, f64) {
    let f = 728.0 / 6_f64.to_radians().tan();
    let (dx, dy) = (e.0 - 728.0, 544.0 - e.1);
    (
        (dx / f).atan().to_degrees(),
        (dy / f).atan().to_degrees(),
        (dx.hypot(dy) / f).atan().to_degrees(),
    )
}

pub fn assert_centre(last: &Value, e: (f64, f64), on_frame: bool) {
    assert_eq!(last["outcome"], "measured", "{last}");
    let centre = &last["centre"];
    let (right, up, separation) = oracle(e);
    for (key, expected, tolerance) in [
        ("x", e.0, 2.0),
        ("y", e.1, 2.0),
        ("right_deg", right, 0.02),
        ("up_deg", up, 0.02),
        ("separation_deg", separation, 0.02),
    ] {
        let actual = centre[key].as_f64().unwrap();
        assert!(
            (actual - expected).abs() <= tolerance,
            "{key}: {actual} != {expected}: {last}"
        );
    }
    assert_eq!(centre["finder_width"], 1456.0);
    assert_eq!(centre["finder_height"], 1088.0);
    assert_eq!(centre["on_frame"], on_frame);
}

#[tokio::test(start_paused = true)]
async fn whole_sensor_frames_show_the_imaging_centre() {
    let rig = rig((700.0, 420.0), Setup::default());
    ready(&rig.app).await;
    let (status, _) = call(
        &rig.app,
        "PUT",
        "/api/imaging/settings",
        json!({
            "roi": {"x": 0, "y": 0, "width": 618, "height": 410}
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let view = start(&rig.app, json!({})).await;
    assert!(view["align"]["since"]
        .as_f64()
        .is_some_and(|at| at >= support::START));
    assert_eq!(
        view["align"],
        json!({"state": "running", "since": view["align"]["since"],
        "exposure_ms": 500.0, "frames": 0})
    );
    let last = last_after(&rig.app, 0).await;
    assert_centre(&last, (700.0, 420.0), true);
    let (_, view) = call(&rig.app, "GET", "/api/imaging", json!(null)).await;
    assert_eq!(view["frame"]["purpose"], "align");
    assert_eq!(view["settings"]["exposure_ms"], 500.0);
    assert_eq!(
        view["settings"]["roi"],
        json!({"x": 0, "y": 0, "width": 1236, "height": 822})
    );
    assert_eq!(view["settings"]["save_at_node"], false);
}

#[tokio::test(start_paused = true)]
async fn centres_off_each_side_still_measure() {
    for e in [(-60.0, 420.0), (-300.0, 420.0), (1800.0, 100.0)] {
        let rig = rig(e, Setup::default());
        ready(&rig.app).await;
        start(&rig.app, json!({})).await;
        assert_centre(&last_after(&rig.app, 0).await, e, false);
    }
}

#[tokio::test(start_paused = true)]
async fn unsuccessful_solves_count_and_keep_running() {
    for (e, focal) in [((-1100.0, 420.0), 1800.0), ((700.0, 420.0), 2200.0)] {
        let rig = rig(
            e,
            Setup {
                focal_mm: Some(focal),
                ..Setup::default()
            },
        );
        ready(&rig.app).await;
        start(&rig.app, json!({})).await;
        let view = wait_until(&rig.app, |v| v["align"]["last"]["unsolved"] == 3).await;
        assert_eq!(view["align"]["state"], "running");
        assert_eq!(view["align"]["frames"], 3);
        let last = &view["align"]["last"];
        assert_eq!(last["outcome"], "not_solved");
        assert_eq!(last["error"], "the imaging solver found no solution");
        assert!(last.get("centre").is_none(), "{last}");
    }
}

#[tokio::test(start_paused = true)]
async fn turning_the_screws_changes_the_next_frames() {
    let rig = rig((-300.0, 420.0), Setup::default());
    ready(&rig.app).await;
    start(&rig.app, json!({})).await;
    let mut last = last_after(&rig.app, 0).await;
    assert_centre(&last, (-300.0, 420.0), false);
    for (e, on_frame) in [((1800.0, 100.0), false), ((900.0, 700.0), true)] {
        rig.sim.set_optics(Optics {
            eyepiece_px: e,
            ..Optics::default()
        });
        // The next exposure was already started before the turn.
        let skipped = last_after(&rig.app, last["frame_id"].as_u64().unwrap()).await;
        last = last_after(&rig.app, skipped["frame_id"].as_u64().unwrap()).await;
        assert_centre(&last, e, on_frame);
    }
}

#[tokio::test(start_paused = true)]
async fn without_a_nearby_live_solve_the_run_keeps_taking_frames() {
    let rig = rig((700.0, 420.0), Setup::default());
    ready(&rig.app).await;
    start(&rig.app, json!({})).await;
    let first = last_after(&rig.app, 0).await;
    assert_centre(&first, (700.0, 420.0), true);
    let (status, answer) = call(
        &rig.app,
        "PUT",
        "/api/session",
        json!({
            "camera_settings": {"exposure_ms": 1, "gain": 1}
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let view = wait_until(&rig.app, |v| {
        v["align"]["last"]["outcome"] == "no_live_solve"
    })
    .await;
    assert_eq!(view["align"]["state"], "running");
    assert!(
        view["align"]["last"]["frame_id"].as_u64().unwrap() > first["frame_id"].as_u64().unwrap()
    );
    assert!(view["align"]["last"].get("centre").is_none(), "{view}");
}

async fn refusal(app: &Router, body: Value, status: StatusCode, words: &str) {
    let answer = call(app, "POST", "/api/imaging/align", body).await;
    assert_eq!(answer, (status, json!({"error": words})));
}

#[tokio::test(start_paused = true)]
async fn start_refusals_and_the_camera_lease() {
    let no_imaging = rig(
        (700.0, 420.0),
        Setup {
            imaging: false,
            ..Setup::default()
        },
    );
    refusal(
        &no_imaging.app,
        json!({}),
        StatusCode::CONFLICT,
        "this console takes no imaging frames",
    )
    .await;
    let no_live = rig(
        (700.0, 420.0),
        Setup {
            live: false,
            ..Setup::default()
        },
    );
    wait_until(&no_live.app, |v| v["state"] == "ready").await;
    refusal(
        &no_live.app,
        json!({}),
        StatusCode::CONFLICT,
        "the finder camera has no live solve",
    )
    .await;
    let no_solver = rig(
        (700.0, 420.0),
        Setup {
            solver: false,
            ..Setup::default()
        },
    );
    ready(&no_solver.app).await;
    refusal(
        &no_solver.app,
        json!({}),
        StatusCode::CONFLICT,
        "this console has no imaging solver",
    )
    .await;
    let rig = rig((700.0, 420.0), Setup::default());
    ready(&rig.app).await;
    let response = rig
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/imaging/align")
                .header("content-type", "text/plain")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert!(serde_json::from_slice::<Value>(&bytes).unwrap()["error"].is_string());
    refusal(
        &rig.app,
        json!({"exposure_ms": 1e9}),
        StatusCode::BAD_REQUEST,
        "exposure_ms 1000000000 is outside the camera's range 1–3600000 ms",
    )
    .await;
    let (status, _) = call(&rig.app, "POST", "/api/imaging/align", json!({"x": 1})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = call(
        &rig.app,
        "PUT",
        "/api/imaging/settings",
        json!({"exposure_ms": 60000}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = call(&rig.app, "POST", "/api/imaging/frame", json!(null)).await;
    assert_eq!(status, StatusCode::OK);
    refusal(
        &rig.app,
        json!({}),
        StatusCode::CONFLICT,
        "an exposure is in progress",
    )
    .await;
    wait_until(&rig.app, |v| v["frame"]["state"] == "saved").await;
    let (status, _) = call(&rig.app, "POST", "/api/imaging/focus", json!({})).await;
    assert_eq!(status, StatusCode::OK);
    refusal(
        &rig.app,
        json!({}),
        StatusCode::CONFLICT,
        "the imaging camera is held by the focus aid",
    )
    .await;
    let (status, _) = call(&rig.app, "DELETE", "/api/imaging/focus", json!(null)).await;
    assert_eq!(status, StatusCode::OK);
    start(&rig.app, json!({})).await;
    refusal(
        &rig.app,
        json!({}),
        StatusCode::CONFLICT,
        "the finder alignment is already running",
    )
    .await;
    for path in ["/api/imaging/focus", "/api/imaging/frame"] {
        assert_eq!(
            call(&rig.app, "POST", path, json!({})).await,
            (
                StatusCode::CONFLICT,
                json!({"error": "the imaging camera is held by the finder alignment"})
            )
        );
    }
}
