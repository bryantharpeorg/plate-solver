//! US2: the simulator's Gaussian H(e) = 2 sqrt(2 ln 2) hypot(1.2, e)
//! is the size oracle (10% tolerance). Statuses, settings and errors use the
//! scenario's exact contract. Every wait uses paused time and is bounded.

use crate::support::live as support;
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use finder::{samples::Samples, session::Session, AppState};
use scope_link::memory;
use serde_json::{json, Value};
use simulator::{mount::SimMount, Optics, SimNode, Simulator};
use std::{sync::Arc, time::Duration};
use tower::ServiceExt;

const FOCUS: &str = "/api/finder-camera/focus";

pub struct Rig {
    pub app: Router,
    pub sim: Simulator,
    pub node: Arc<SimNode>,
}

pub fn rig() -> Rig {
    rig_with_settings(true)
}

fn rig_with_settings(fixed: bool) -> Rig {
    let sim = support::sim();
    sim.connect_mount(SimMount::new(sim.clock().clone()));
    let node = Arc::new(
        SimNode::new(sim.clone(), "test-token", Default::default())
            .unwrap()
            .with_frame_interval(Duration::from_millis(500)),
    );
    let server = node.clone();
    let transport = finder::link::Transport::connector(move || {
        let node = server.clone();
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
    let mut edit = json!({
        "node": {"address": "scope.local:1", "token": "test-token"},
        "site": {"lat_deg": 40, "lon_deg": -75}
    });
    if fixed {
        edit["camera_settings"] = json!({"exposure_ms": 500, "gain": 1});
    }
    session
        .edit(serde_json::from_value(edit).unwrap(), support::START)
        .unwrap();
    let state = AppState::shared(support::DB.clone(), Samples::default())
        .with_session(session)
        .with_clock(Arc::new(support::ConsoleClock(sim.clock().clone())))
        .with_link(transport)
        .with_live();
    Rig {
        app: finder::router(state),
        sim,
        node,
    }
}

async fn send(app: &Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = serde_json::from_slice(&bytes).expect("a JSON response");
    (status, body)
}

pub async fn call(app: &Router, method: &str, path: &str, body: Value) -> (StatusCode, Value) {
    send(
        app,
        Request::builder()
            .method(method)
            .uri(path)
            .header("host", "localhost:8080")
            .header("content-type", "application/json")
            .body(if matches!(method, "GET" | "DELETE") {
                Body::empty()
            } else {
                Body::from(body.to_string())
            })
            .unwrap(),
    )
    .await
}

async fn get(app: &Router, path: &str) -> Value {
    let (status, body) = call(app, "GET", path, Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body
}

pub async fn wait_until(app: &Router, check: impl Fn(&Value) -> bool) -> Value {
    for _ in 0..1200 {
        let body = get(app, FOCUS).await;
        if check(&body["focus"]) {
            return body["focus"].clone();
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!(
        "focus did not reach the awaited state: {}",
        get(app, FOCUS).await
    );
}

pub async fn live(app: &Router) {
    for _ in 0..1200 {
        if get(app, "/api/session").await["pointing"]["source"] == "live" {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("the finder camera never solved live");
}

pub async fn settled_after(app: &Router, n: u64) -> Value {
    wait_until(app, |v| settled(v) && v["readings"].as_u64().unwrap() > n).await
}

fn settled(v: &Value) -> bool {
    v["last"]["state"] == "reading" && v["median"]["readings"] == 3
}

async fn start(app: &Router) -> Value {
    let (status, body) = call(app, "POST", FOCUS, json!({})).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body["focus"].clone()
}

fn at_focus(value: &Value) {
    let expected = 2.0 * (2.0 * 2.0_f64.ln()).sqrt() * 1.2;
    let actual = value.as_f64().unwrap();
    assert!(
        (actual - expected).abs() <= 0.10 * expected,
        "{actual} != {expected}"
    );
}

#[tokio::test(start_paused = true)]
async fn readings_run_beside_the_live_solve_without_changing_settings() {
    let rig = rig();
    live(&rig.app).await;
    let before = get(&rig.app, "/api/session").await;
    assert_eq!(get(&rig.app, FOCUS).await, json!({"focus": null}));
    let view = start(&rig.app).await;
    assert_eq!(view["state"], "running");
    assert_eq!(view["readings"], 0);
    assert_eq!(view["found"], false);
    assert_eq!(view["history"], json!([]));
    let view = settled_after(&rig.app, 0).await;
    at_focus(&view["median"]["value"]);
    assert!(view["last"]["stars"].as_u64().unwrap() >= 5, "{view}");
    assert!(
        view["last"]["detected"].as_u64().unwrap() > view["last"]["stars"].as_u64().unwrap(),
        "{view}"
    );
    assert_eq!(
        view["history"].as_array().unwrap().len() as u64,
        view["readings"].as_u64().unwrap()
    );
    let after = get(&rig.app, "/api/session").await;
    assert_eq!(after["camera_settings"], before["camera_settings"]);
    assert!(
        after["pointing"]["seq"].as_u64().unwrap() > before["pointing"]["seq"].as_u64().unwrap()
    );
}

#[tokio::test(start_paused = true)]
async fn turning_through_focus_and_back_finds_the_best() {
    let rig = rig();
    live(&rig.app).await;
    rig.sim.set_finder_focus_error(3.0);
    start(&rig.app).await;
    settled_after(&rig.app, 0).await;
    rig.sim.set_finder_focus_error(0.0);
    let better = wait_until(&rig.app, |v| settled(v) && v["trend"] == "better").await;
    assert_eq!(better["found"], false);
    rig.sim.set_finder_focus_error(-3.0);
    let worse = wait_until(&rig.app, |v| settled(v) && v["trend"] == "worse").await;
    assert_eq!(worse["found"], false);
    rig.sim.set_finder_focus_error(0.0);
    let found = wait_until(&rig.app, |v| v["found"] == true).await;
    assert_eq!(found["state"], "running");
    assert_eq!(found["found"], true);
    at_focus(&found["best"]);
    assert!(found["median"]["value"].as_f64().unwrap() <= 1.10 * found["best"].as_f64().unwrap());
}

#[tokio::test(start_paused = true)]
async fn no_stars_and_changed_optics_keep_running_and_stop_is_idempotent() {
    let rig = rig();
    live(&rig.app).await;
    let settings = get(&rig.app, "/api/session").await["camera_settings"].clone();
    start(&rig.app).await;
    settled_after(&rig.app, 0).await;
    rig.sim.set_finder_focus_error(10.0);
    let empty = wait_until(&rig.app, |v| v["last"]["state"] == "too_few_stars").await;
    assert_eq!(empty["state"], "running");
    assert!(empty["last"]["stars"].as_u64().unwrap() < 5);
    assert!(empty["last"]["detected"].as_u64().unwrap() < 5);
    assert!(empty["last"].get("value").is_none(), "{empty}");
    assert!(empty.get("median").is_none(), "{empty}");
    assert_eq!(
        empty["history"].as_array().unwrap().last(),
        Some(&Value::Null)
    );
    rig.sim.set_optics(Optics {
        fov_deg: 24.0,
        ..Optics::default()
    });
    rig.sim.set_finder_focus_error(0.0);
    let recovered = settled_after(&rig.app, empty["readings"].as_u64().unwrap()).await;
    assert_eq!(recovered["state"], "running");
    let (status, stopped) = call(&rig.app, "DELETE", FOCUS, Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(stopped["focus"]["state"], "ended");
    assert_eq!(stopped["focus"]["ended"], "stopped");
    tokio::time::sleep(Duration::from_secs(5)).await;
    let (status, again) = call(&rig.app, "DELETE", FOCUS, Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(again["focus"]["readings"], stopped["focus"]["readings"]);
    assert_eq!(again["focus"]["ended"], "stopped");
    assert_eq!(
        get(&rig.app, "/api/session").await["camera_settings"],
        settings
    );
}

async fn refused(app: &Router, words: &str) {
    assert_eq!(
        call(app, "POST", FOCUS, json!({})).await,
        (StatusCode::CONFLICT, json!({"error": words}))
    );
}

#[tokio::test(start_paused = true)]
async fn refusals_origin_body_and_fifteen_seconds_without_frames() {
    let bare = finder::router(AppState::shared(support::DB.clone(), Samples::default()));
    refused(&bare, "the scope node is not connected").await;
    let automatic = rig_with_settings(false);
    live(&automatic.app).await;
    refused(
        &automatic.app,
        "the finder camera's exposure or gain is automatic: fix both on Setup",
    )
    .await;
    let rig = rig();
    live(&rig.app).await;
    for method in ["POST", "DELETE"] {
        let request = Request::builder()
            .method(method)
            .uri(FOCUS)
            .header("host", "localhost:8080")
            .header("origin", "http://elsewhere.example")
            .header("content-type", "text/plain")
            .body(Body::from("not JSON"))
            .unwrap();
        assert_eq!(
            send(&rig.app, request).await,
            (
                StatusCode::FORBIDDEN,
                json!({"error": "writes require the console's own origin and an allowed host"})
            )
        );
    }
    assert_eq!(get(&rig.app, FOCUS).await, json!({"focus": null}));
    assert_eq!(
        call(&rig.app, "POST", FOCUS, json!({"x": 1})).await.0,
        StatusCode::BAD_REQUEST
    );
    for body in [json!([]), Value::Null, json!(true), json!("{}")] {
        assert_eq!(
            call(&rig.app, "POST", FOCUS, body).await.0,
            StatusCode::BAD_REQUEST
        );
    }
    let request = Request::post(FOCUS)
        .header("content-type", "text/plain")
        .body(Body::from("{}"))
        .unwrap();
    assert_eq!(
        send(&rig.app, request).await.0,
        StatusCode::UNSUPPORTED_MEDIA_TYPE
    );
    assert_eq!(
        call(
            &rig.app,
            "PUT",
            "/api/session",
            json!({"camera_settings": {"exposure_ms": "auto"}})
        )
        .await
        .0,
        StatusCode::OK
    );
    refused(
        &rig.app,
        "the finder camera's exposure or gain is automatic: fix both on Setup",
    )
    .await;
    assert_eq!(
        call(
            &rig.app,
            "PUT",
            "/api/session",
            json!({"camera_settings": {"exposure_ms": 500}})
        )
        .await
        .0,
        StatusCode::OK
    );
    start(&rig.app).await;
    refused(&rig.app, "the finder camera's focus aid is already running").await;
    settled_after(&rig.app, 0).await;
    rig.node.service().controls().stop_frames();
    tokio::time::sleep(Duration::from_secs(10)).await;
    assert_eq!(get(&rig.app, FOCUS).await["focus"]["state"], "running");
    let ended = wait_until(&rig.app, |v| v["state"] == "ended").await;
    assert_eq!(ended["ended"], "failed");
    assert_eq!(ended["error"], "no raw frame arrived within 15 s");
}

#[tokio::test(start_paused = true)]
async fn changing_exposure_or_gain_ends_the_run() {
    let rig = rig();
    live(&rig.app).await;
    for settings in [json!({"exposure_ms": 250}), json!({"gain": 2})] {
        start(&rig.app).await;
        let before = settled_after(&rig.app, 0).await;
        assert_eq!(
            call(
                &rig.app,
                "PUT",
                "/api/session",
                json!({"camera_settings": settings})
            )
            .await
            .0,
            StatusCode::OK
        );
        let ended = wait_until(&rig.app, |v| v["state"] == "ended").await;
        assert_eq!(ended["ended"], "failed");
        assert_eq!(
            ended["error"],
            "the finder camera's exposure or gain changed during the run"
        );
        assert!(ended["readings"].as_u64().unwrap() >= before["readings"].as_u64().unwrap());
    }
    assert_eq!(
        get(&rig.app, "/api/session").await["camera_settings"]["exposure_ms"],
        250.0
    );
}
