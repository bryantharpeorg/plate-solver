//! US4's console contract. Oracles: the declared empty JSON object, a master
//! built by hand at the simulator's starting settings, and the saved imaging
//! frame's FITS cards. Values are exact, including the two frame keys.

use std::{fs, path::Path, sync::Arc, time::Duration};

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use finder::{
    imaging::{
        calibration::library::{FrameKey, Kind, MasterEntry},
        fits,
    },
    link::{LinkManager, Transport},
    session::Session,
    AppState,
};
use scope_link::memory;
use serde_json::{json, Value};
use simulator::{mount::SimMount, SimNode, Simulator};
use tower::ServiceExt;

use crate::support::{live as support, Scratch};

/// Shared with the later stories: the caller keeps the simulator so it can
/// change the sensor pattern or light path without rebuilding the console.
pub(crate) struct Rig {
    pub app: Router,
    pub state: AppState,
    pub link: Arc<LinkManager>,
}

pub(crate) fn sim_rig() -> Simulator {
    let sim = support::sim();
    sim.connect_mount(SimMount::new(sim.clock().clone()));
    sim.push_to(50.0, 120.0);
    sim
}

fn build_rig(sim: &Simulator, root: Option<&Path>, imaging_service: bool) -> Rig {
    let node = Arc::new(
        SimNode::new(sim.clone(), "test-token", Default::default())
            .unwrap()
            .with_frame_interval(Duration::from_millis(500)),
    );
    let transport = Transport::connector(move || {
        let node = node.clone();
        async move {
            let (client, server) = tokio::io::duplex(64 * 1024);
            let service = node.service();
            tokio::spawn(async move {
                if imaging_service {
                    memory::serve_routes_on(server, service.routes())
                        .await
                        .unwrap();
                } else {
                    memory::serve_on(server, service.into_server())
                        .await
                        .unwrap();
                }
            });
            Ok::<_, std::io::Error>(client)
        }
    });
    let session = Session::default();
    session
        .edit(
            serde_json::from_value(json!({
                "node": {"address":"scope.local:1", "token":"test-token"},
                "site": {"lat_deg":40, "lon_deg":-75}
            }))
            .unwrap(),
            support::START,
        )
        .unwrap();
    let state = AppState::shared(support::DB.clone(), finder::samples::Samples::default())
        .with_session(session)
        .with_clock(Arc::new(support::ConsoleClock(sim.clock().clone())))
        .with_link(transport)
        .with_live();
    let state = match root {
        Some(root) => state.with_imaging(root),
        None => state,
    };
    Rig {
        app: finder::router(state.clone()),
        link: state.link().unwrap(),
        state,
    }
}

pub(crate) fn console_rig(sim: &Simulator, root: &Path) -> Rig {
    build_rig(sim, Some(root), true)
}

/// A JSON request, including DELETE and the optional Origin used for the
/// same-origin refusal. The body is read exactly as the observer would see it.
pub(crate) async fn request(
    app: &Router,
    method: &str,
    path: &str,
    body: Value,
    origin: Option<&str>,
) -> (StatusCode, Value) {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header("host", "localhost")
        .header("content-type", "application/json");
    if let Some(origin) = origin {
        request = request.header("origin", origin);
    }
    let response = app
        .clone()
        .oneshot(request.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

/// Poll a route's body in paused time. No timeout surrounds a blocking task:
/// advancing virtual time does not measure disk work's elapsed time.
pub(crate) async fn wait_until(app: &Router, path: &str, check: impl Fn(&Value) -> bool) -> Value {
    for _ in 0..600 {
        let (status, body) = request(app, "GET", path, Value::Null, None).await;
        assert_eq!(status, StatusCode::OK);
        if check(&body) {
            return body;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let (_, body) = request(app, "GET", path, Value::Null, None).await;
    panic!("{path} never reached the awaited state: {body}");
}

async fn get(app: &Router, path: &str) -> Value {
    let (status, body) = request(app, "GET", path, Value::Null, None).await;
    assert_eq!(status, StatusCode::OK);
    body
}

async fn calibration(app: &Router) -> Value {
    get(app, "/api/imaging/calibration").await
}

async fn edit(app: &Router, body: Value) -> Value {
    let (status, body) = request(app, "PUT", "/api/imaging/settings", body, None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["calibration"], calibration(app).await);
    body
}

async fn take(app: &Router) {
    let (status, body) = request(app, "POST", "/api/imaging/frame", Value::Null, None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["calibration"], calibration(app).await);
}

async fn saved(app: &Router, id: u64) -> Value {
    wait_until(app, "/api/imaging", |body| {
        body["frame"]["id"] == id && body["frame"]["state"] == "saved"
    })
    .await
}

fn master() -> MasterEntry {
    MasterEntry {
        id: "dark-20260116T020003Z".into(),
        kind: Kind::Dark,
        made_at: support::START + 3.0,
        frames: 20,
        checked: true,
        model: "simulated imaging camera".into(),
        serial: Some("SIM-0001".into()),
        width: 1236,
        height: 822,
        origin_x: 0,
        origin_y: 0,
        bits: 16,
        bayer: Some("RGGB".into()),
        gain: Some(120.0),
        offset: Some(10.0),
        binning: 1,
        set_temp_c: None,
        temp_c: Some(15.0),
        exposure_ms: 200.0,
        level: Some(160.0),
        bad_pixels: None,
        label: None,
        imaging_angle_deg: None,
        level_share: None,
        dark_frames: None,
        file: String::new(),
    }
}

/// US4-S1: exact route/view/session parity and hand-built matching settings;
/// deletion and malformed ids leave independent sentinel files untouched.
#[tokio::test(start_paused = true)]
async fn library_routes_views_and_master_removal() {
    let scratch = Scratch::new("calibration-console-library");
    let root = scratch.0.join("imaging");
    let sim = sim_rig();
    let rig = console_rig(&sim, &root);
    wait_until(&rig.app, "/api/imaging", |v| v["state"] == "ready").await;
    assert!(rig.link.status().borrow().connected);
    let empty = json!({"masters":[]});
    assert_eq!(calibration(&rig.app).await, empty);
    assert_eq!(get(&rig.app, "/api/imaging").await["calibration"], empty);
    assert_eq!(
        get(&rig.app, "/api/session").await["imaging"]["calibration"],
        empty
    );

    let library = rig.state.calibration().unwrap();
    let added = library
        .library()
        .add(master(), &vec![160.0; 1236 * 822])
        .unwrap();
    edit(&rig.app, json!({"exposure_ms":200})).await;
    take(&rig.app).await;
    saved(&rig.app, 1).await;
    let matched = json!({"frame":1,"dark":added.id,"flat":null,"bad_pixels":null,
        "missing":["flat","bad_pixels"]});
    let view = calibration(&rig.app).await;
    assert_eq!(
        view,
        json!({"masters":[serde_json::to_value(&added).unwrap()], "match":matched})
    );
    assert_eq!(get(&rig.app, "/api/imaging").await["calibration"], view);
    assert_eq!(
        get(&rig.app, "/api/session").await["imaging"]["calibration"],
        view
    );
    edit(&rig.app, json!({"gain":0})).await;
    take(&rig.app).await;
    saved(&rig.app, 2).await;
    let unmatched = json!({"frame":2,"dark":null,"flat":null,"bad_pixels":null,
        "missing":["dark","flat","bad_pixels"]});
    assert_eq!(calibration(&rig.app).await["match"], unmatched);

    let path = format!("/api/imaging/calibration/{}", added.id);
    let (status, body) = request(
        &rig.app,
        "DELETE",
        &path,
        Value::Null,
        Some("http://elsewhere"),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(body["error"].is_string());
    assert!(root.join("calibration").join(&added.file).exists());
    assert_eq!(
        calibration(&rig.app).await["masters"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let (status, removed) = request(&rig.app, "DELETE", &path, Value::Null, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(removed, calibration(&rig.app).await);
    assert_eq!(removed, json!({"masters":[], "match":unmatched}));
    assert_eq!(
        fs::read_dir(root.join("calibration/darks"))
            .unwrap()
            .count(),
        0
    );
    let (status, body) = request(&rig.app, "DELETE", &path, Value::Null, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(
        body,
        json!({"error":format!("no master or set has the id {}", added.id)})
    );

    let index = fs::read(root.join("calibration/index.json")).unwrap();
    let ids = [
        ("A".into(), "A".into()),
        ("a%2Fb".into(), "a/b".into()),
        ("a".repeat(65), "a".repeat(65)),
    ];
    fs::create_dir_all(root.join("calibration/a")).unwrap();
    fs::write(root.join("calibration/a/b"), b"sentinel").unwrap();
    for (encoded, id) in ids {
        let sentinel = root.join("calibration").join(&id);
        fs::write(&sentinel, b"sentinel").unwrap();
        let (status, body) = request(
            &rig.app,
            "DELETE",
            &format!("/api/imaging/calibration/{encoded}"),
            Value::Null,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(
            body,
            json!({"error":format!("no master or set has the id {id}")})
        );
        assert_eq!(fs::read(&sentinel).unwrap(), b"sentinel");
        assert_eq!(
            fs::read(root.join("calibration/index.json")).unwrap(),
            index
        );
    }
    drop(rig);
}

/// US4-S2: exact off shape; origin before the no-imaging refusal; a route
/// outside ready reads the library, including an independently corrupted index.
#[tokio::test(start_paused = true)]
async fn off_and_not_ready_keep_their_shapes() {
    let scratch = Scratch::new("calibration-console-states");
    let root = scratch.0.join("imaging");
    let sim = sim_rig();
    let rig = build_rig(&sim, None, true);
    assert_eq!(calibration(&rig.app).await, json!({"masters":[]}));
    let path = "/api/imaging/calibration/dark-20260116T020003Z";
    let (status, body) = request(&rig.app, "DELETE", path, Value::Null, None).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(
        body,
        json!({"error":"this console takes no imaging frames"})
    );
    let (status, _) = request(
        &rig.app,
        "DELETE",
        path,
        Value::Null,
        Some("http://elsewhere"),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(get(&rig.app, "/api/imaging").await, json!({"state":"off"}));
    assert!(!root.exists());
    drop(rig);

    for unreadable in [false, true] {
        if unreadable {
            fs::create_dir_all(root.join("calibration")).unwrap();
            fs::write(root.join("calibration/index.json"), b"not json").unwrap();
        }
        let rig = build_rig(&sim, Some(&root), false);
        let view = wait_until(&rig.app, "/api/imaging", |v| v["state"] == "no_service").await;
        assert!(view.get("calibration").is_none());
        let library = calibration(&rig.app).await;
        assert_eq!(library["masters"], json!([]));
        assert!(library.get("match").is_none());
        if unreadable {
            assert!(library["error"]
                .as_str()
                .unwrap()
                .starts_with("the calibration index could not be read: "));
            assert_eq!(
                fs::read(root.join("calibration/index.json")).unwrap(),
                b"not json"
            );
        } else {
            assert_eq!(library, json!({"masters":[]}));
        }
        drop(rig);
    }
}

/// US4-S3: the saved FITS file and the view independently agree with every
/// declared simulator setting. Only eligible saved imaging frames advance match.
#[tokio::test(start_paused = true)]
async fn saved_frame_key_and_match_survive_other_purposes_and_exposure() {
    let scratch = Scratch::new("calibration-console-match");
    let root = scratch.0.join("imaging");
    let sim = sim_rig();
    let rig = console_rig(&sim, &root);
    wait_until(&rig.app, "/api/imaging", |v| v["state"] == "ready").await;
    edit(
        &rig.app,
        json!({"cooler_on":true,"set_temp_c":-10,"exposure_ms":200}),
    )
    .await;
    assert!(calibration(&rig.app).await.get("match").is_none());
    take(&rig.app).await;
    let saved_view = saved(&rig.app, 1).await;
    assert_eq!(saved_view["frame"]["set_temp_c"], -10.0);
    assert_eq!(calibration(&rig.app).await["match"]["frame"], 1);
    let imaging = rig.state.imaging().unwrap();
    let view = imaging.view();
    let image = fits::decode(
        &fs::read(root.join(view.frame.as_ref().unwrap().file.as_ref().unwrap())).unwrap(),
    )
    .unwrap();
    let from_file = FrameKey::from_image(&image).unwrap();
    let from_view = FrameKey::from_view(&view).unwrap();
    let expected = FrameKey {
        model: "simulated imaging camera".into(),
        serial: Some("SIM-0001".into()),
        width: 1236,
        height: 822,
        origin_x: 0,
        origin_y: 0,
        bits: 16,
        bayer: Some("RGGB".into()),
        gain: Some(120.0),
        offset: Some(10.0),
        binning: 1,
        set_temp_c: Some(-10.0),
        temp_c: Some(-10.0),
        exposure_ms: 200.0,
    };
    assert_eq!(from_file, expected);
    assert_eq!(from_view, expected);
    // Changing the current settings must not rewrite the saved frame's key.
    edit(&rig.app, json!({"set_temp_c":-5})).await;
    assert_eq!(FrameKey::from_view(&imaging.view()).unwrap(), expected);
    imaging.take("dark").await.unwrap();
    saved(&rig.app, 2).await;
    assert_eq!(calibration(&rig.app).await["match"]["frame"], 1);
    edit(&rig.app, json!({"exposure_ms":10000})).await;
    take(&rig.app).await;
    wait_until(&rig.app, "/api/imaging", |v| {
        v["status"]["exposing"] == true
    })
    .await;
    assert_eq!(calibration(&rig.app).await["match"]["frame"], 1);
    saved(&rig.app, 3).await;
    assert_eq!(calibration(&rig.app).await["match"]["frame"], 3);
    edit(&rig.app, json!({"exposure_ms":200})).await;
    for (purpose, id) in [("focus", 4), ("flat", 5)] {
        imaging.take(purpose).await.unwrap();
        saved(&rig.app, id).await;
        assert_eq!(calibration(&rig.app).await["match"]["frame"], 3);
    }
    drop(rig);

    let root = scratch.0.join("second/imaging");
    let sim = sim_rig();
    let rig = console_rig(&sim, &root);
    wait_until(&rig.app, "/api/imaging", |v| v["state"] == "ready").await;
    edit(&rig.app, json!({"exposure_ms":200})).await;
    for (purpose, id) in [("dark", 1), ("focus", 2)] {
        rig.state.imaging().unwrap().take(purpose).await.unwrap();
        saved(&rig.app, id).await;
        assert!(calibration(&rig.app).await.get("match").is_none());
    }
    drop(rig);
}
