//! Route oracles: saved model terms and simulator axis counts (exact equality).
#[path = "goto_api.rs"]
mod api;
use api::Rig;
use axum::{
    body::{to_bytes, Body},
    http::Request,
    Router,
};
use finder::{
    model::{Model, Sync},
    samples::Samples,
    session::{FixedClock, Session},
    AppState,
};
use scope_node::mount::{Axis, Mount};
use serde_json::{json, Value};
use tower::ServiceExt;

const START: f64 = 1_768_528_800.0;
static DB: std::sync::LazyLock<std::sync::Arc<pattern_database::PatternDatabase>> =
    std::sync::LazyLock::new(|| {
        std::sync::Arc::new(
            pattern_database::load_from_path(
                &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../reference-solutions/cedar-solve/tetra3/data/default_database.npz"),
            )
            .unwrap(),
        )
    });

async fn reset(app: &Router, what: Value, foreign: bool) -> (u16, Value) {
    let mut request = Request::post("/api/model/reset")
        .header("host", "localhost")
        .header("content-type", "application/json");
    if foreign {
        request = request.header("origin", "http://foreign.invalid");
    }
    let response = app
        .clone()
        .oneshot(request.body(Body::from(what.to_string())).unwrap())
        .await
        .unwrap();
    (
        response.status().as_u16(),
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap(),
    )
}

#[tokio::test]
async fn resets_clear_only_the_requested_terms_and_persist_across_restart() {
    // A persisted fixture of a night's state with explicit nonzero terms. Loading it
    // exercises the same persisted format as a restarted console, without fitting
    // expectations with the implementation under test.
    let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target")
        .join(format!("model-api-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let path = folder.join("session.json");
    let fixed = json!({"ca":0.02,"npae":-0.03,"tf":0.01,"camera":4.0});
    let capture = json!({"site":{"lat_deg":40,"lon_deg":-75},"min_alt_deg":15,
        "model":{"syncs":[{"time":START,"axis":[0,0],"counts_per_revolution":[1620000,1620000],
        "az_deg":120,"alt_deg":50,"roll_deg":0,"rms_arcsec":1,"epoch":0}],
        "epochs":[{"ia":-120,"ie":50}],"base_tilt":{"an":-0.6427876097,"aw":-0.7660444431},
        "fixed":fixed,"expires_at":START+40000.0,"rms_arcsec":2}});
    std::fs::write(&path, capture.to_string()).unwrap();
    let state = AppState::shared(DB.clone(), Samples::default())
        .with_session(Session::with_state_file(&path).unwrap())
        .with_clock(std::sync::Arc::new(FixedClock(START)));
    let before = state.session().model(START);
    assert_eq!(before.syncs.len(), 1);
    assert_ne!(before.epochs[0].ia, 0.0);
    assert_ne!(before.epochs[0].ie, 0.0);
    assert!(before.base_tilt.is_some());
    assert_eq!(serde_json::to_value(before.fixed).unwrap(), fixed);
    let app = finder::router(state.clone());
    let (status, body) = reset(&app, json!({"what":"night"}), false).await;
    assert_eq!(status, 200, "{body}");
    let night = state.session().model(START);
    assert!(night.syncs.is_empty());
    assert!(night.epochs.is_empty());
    assert!(night.base_tilt.is_none());
    assert!(night.expires_at.is_none());
    assert_eq!(night.fixed, before.fixed);
    assert_eq!(Session::with_state_file(&path).unwrap().model(START), night);
    assert_eq!(body["model"]["syncs"], 0);
    assert_eq!(body["model"]["ia"], Value::Null);
    assert_eq!(body["model"]["ie"], Value::Null);
    assert_eq!(body["model"]["fixed"], fixed);
    let (status, body) = reset(&app, json!({"what":"fixed"}), false).await;
    assert_eq!(status, 200, "{body}");
    assert!(state.session().model(START).fixed.is_none());
    assert!(Session::with_state_file(&path)
        .unwrap()
        .model(START)
        .fixed
        .is_none());
    assert!(body.get("model").is_none());

    // The fixed reset also preserves a populated night's syncs, zero points and
    // base tilt; it must not silently act as another night reset.
    std::fs::write(&path, capture.to_string()).unwrap();
    let state = AppState::shared(DB.clone(), Samples::default())
        .with_session(Session::with_state_file(&path).unwrap())
        .with_clock(std::sync::Arc::new(FixedClock(START)));
    let before = state.session().model(START);
    let app = finder::router(state.clone());
    for what in ["night", "fixed"] {
        let (status, body) = reset(&app, json!({"what":what}), true).await;
        assert_eq!(status, 403, "{body}");
        assert!(body["error"].is_string());
        assert_eq!(state.session().model(START), before);
    }
    let (status, body) = reset(&app, json!({"what":"fixed"}), false).await;
    assert_eq!(status, 200, "{body}");
    let after = state.session().model(START);
    assert_eq!(after.syncs, before.syncs);
    assert_eq!(after.epochs, before.epochs);
    assert_eq!(after.base_tilt, before.base_tilt);
    assert!(after.fixed.is_none());
    assert_eq!(Session::with_state_file(&path).unwrap().model(START), after);
    std::fs::remove_dir_all(folder).unwrap();
}

#[tokio::test(start_paused = true)]
async fn foreign_model_writes_leave_a_working_mount_and_populated_model_unchanged() {
    let rig = Rig::new(START, 50.0, 120.0, true).await;
    rig.sync().await;
    assert_eq!(rig.view().await["mount"]["connected"], true);
    assert_eq!(rig.call("/api/goto", false).await.0, 200);
    for _ in 0..100 {
        if !rig.stationary().await {
            break;
        }
        rig.tick().await;
    }
    assert!(
        !rig.stationary().await,
        "positive control: an accepted GoTo moves"
    );
    assert_eq!(rig.call("/api/stop", false).await.0, 200);
    for _ in 0..20 {
        rig.tick().await;
    }
    assert!(rig.stationary().await);
    rig.controls.stop_frames();
    for _ in 0..10 {
        rig.tick().await;
    }
    let before = rig.state.session().model(rig.now());
    assert!(!before.syncs.is_empty());
    let counts = [
        rig.mount.state(Axis::Azimuth).await.position,
        rig.mount.state(Axis::Altitude).await.position,
    ];
    for what in [None, Some("night"), Some("fixed")] {
        let (status, body) = match what {
            None => {
                let (s, b) = rig.call("/api/model/second-sync", true).await;
                (s.as_u16(), b)
            }
            Some(what) => reset(&rig.app, json!({"what":what}), true).await,
        };
        assert_eq!(status, 403, "{body}");
        assert!(body["error"].as_str().unwrap().contains("origin"));
        for _ in 0..15 {
            rig.tick().await;
            assert!(rig.stationary().await);
        }
        assert_eq!(
            counts,
            [
                rig.mount.state(Axis::Azimuth).await.position,
                rig.mount.state(Axis::Altitude).await.position
            ]
        );
        assert_eq!(rig.state.session().model(rig.now()), before);
    }
}

#[tokio::test]
async fn invalid_reset_requests_are_json_errors_and_leave_the_model_unchanged() {
    let session = Session::default();
    session
        .edit(
            serde_json::from_value(json!({"site":{"lat_deg":40,"lon_deg":-75}})).unwrap(),
            START,
        )
        .unwrap();
    session
        .record_sync(Sync {
            time: START,
            axis: [0.0, 0.0],
            counts_per_revolution: [360; 2],
            az_deg: 120.0,
            alt_deg: 50.0,
            roll_deg: None,
            rms_arcsec: 1.0,
            epoch: 0,
        })
        .unwrap();
    let state = AppState::shared(DB.clone(), Samples::default())
        .with_session(session)
        .with_clock(std::sync::Arc::new(FixedClock(START)));
    let before: Model = state.session().model(START);
    let app = finder::router(state.clone());
    for body in [
        json!({}),
        json!({"what":"all"}),
        json!({"what":"night","extra":true}),
    ] {
        let (status, body) = reset(&app, body, false).await;
        assert_eq!(status, 400, "{body}");
        assert!(body["error"].is_string());
        assert_eq!(state.session().model(START), before);
    }
}
