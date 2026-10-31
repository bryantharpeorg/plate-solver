//! Router oracles: independent finder alt/az (30 arcseconds), simulator truth
//! (1e-10 degrees), strict requests, and zero frame delivery while stopped.
#[path = "live/support.rs"]
mod support;
use axum::{
    body::{to_bytes, Body},
    http::Request,
};
use finder::{samples::Samples, session::Session, simulate::SimulationTransport, AppState};
use serde_json::{json, Value};
use std::time::Duration;
use support::*;
use tower::ServiceExt;

#[tokio::test(start_paused = true)]
async fn pushed_telescope_solves_at_the_requested_direction_and_reads_both_true_directions() {
    let state = AppState::shared(DB.clone(), Samples::default())
        .with_simulation(START, SimulationTransport::Memory)
        .await
        .unwrap()
        .with_live();
    let sim = state.simulator().unwrap();
    let mut events = state.live().unwrap().subscribe();
    let app = finder::router(state);
    event(&mut events).await;
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/sim/telescope",
            json!({"alt_deg":40,"az_deg":200})
        )
        .await
        .0,
        200
    );
    // One exposure may have been in progress at the instant of the push.
    event(&mut events).await;
    let report = event(&mut events).await;
    assert_eq!(report.report.status, finder::solve::Status::MatchFound);
    let (_, read) = request(&app, "GET", "/api/session", json!(null)).await;
    let p = &read["pointing"];
    let (alt, az) = finder::sky::alt_az(
        p["ra_deg"].as_f64().unwrap(),
        p["dec_deg"].as_f64().unwrap(),
        &finder::sky::Observer {
            lat: 40.0,
            lon: -75.0,
            time: p["time"].as_f64().unwrap(),
        },
    );
    let distance = math_core::angular_distance(
        math_core::UnitVector::from_radec(az.to_radians(), alt.to_radians()),
        math_core::UnitVector::from_radec(200_f64.to_radians(), 40_f64.to_radians()),
    );
    assert!(distance.to_degrees() * 3600.0 < 30.0, "{alt}, {az}");
    let (_, read) = request(&app, "GET", "/api/sim/telescope", json!(null)).await;
    let truth = sim.telescope();
    for (name, direction) in [("camera", truth.camera), ("eyepiece", truth.eyepiece)] {
        for (key, value) in [
            ("alt", direction.alt),
            ("az", direction.az),
            ("ra", direction.ra),
            ("dec", direction.dec),
        ] {
            assert!((read[name][key].as_f64().unwrap() - value).abs() < 1e-10);
        }
    }
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/sim/telescope",
            json!({"turn_deg":10,"tilt_deg":-5})
        )
        .await
        .0,
        200
    );
    assert_eq!(sim.telescope().camera.alt, 35.0);
    assert_eq!(sim.telescope().camera.az, 210.0);
}

#[tokio::test(start_paused = true)]
async fn controls_are_absent_outside_simulation_and_foreign_origins_cannot_change_any_control() {
    let offline = finder::router(AppState::shared(DB.clone(), Samples::default()));
    for (method, path, body) in [
        ("GET", "/api/sim/telescope", json!(null)),
        (
            "POST",
            "/api/sim/telescope",
            json!({"alt_deg":40,"az_deg":200}),
        ),
        ("POST", "/api/sim/frames", json!({"running":false})),
        ("GET", "/api/sim/unknown", json!(null)),
        ("POST", "/api/sim/unknown", json!({})),
    ] {
        assert_eq!(request(&offline, method, path, body).await.0, 404);
    }
    let session = Session::default();
    session
        .edit(
            serde_json::from_value(json!({"site":{"lat_deg":-30,"lon_deg":20}})).unwrap(),
            START,
        )
        .unwrap();
    let state = AppState::shared(DB.clone(), Samples::default())
        .with_session(session)
        .with_simulation(START, SimulationTransport::Memory)
        .await
        .unwrap()
        .with_live();
    let sim = state.simulator().unwrap();
    let truth = sim.telescope();
    // The initial simulator site is the saved session site, not the desk fallback.
    let horizontal = finder::sky::alt_az(
        truth.camera.ra,
        truth.camera.dec,
        &finder::sky::Observer {
            lat: -30.0,
            lon: 20.0,
            time: START,
        },
    );
    assert!((horizontal.0 - 45.0).abs() < 1e-6);
    let mut frames = state.link().unwrap().frames();
    let app = finder::router(state);
    for (method, path, body) in [
        ("GET", "/api/sim/telescope", json!(null)),
        (
            "POST",
            "/api/sim/telescope",
            json!({"alt_deg":40,"az_deg":200}),
        ),
        (
            "POST",
            "/api/sim/telescope",
            json!({"eyepiece_on_star":true}),
        ),
        ("POST", "/api/sim/frames", json!({"running":false})),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(path)
                    .header("host", "localhost")
                    .header("origin", "http://elsewhere.example")
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), 403);
    }
    assert_eq!(sim.telescope(), truth);
    tokio::time::timeout(Duration::from_secs(10), async {
        while frames.borrow_and_update().is_none() {
            frames.changed().await.unwrap();
        }
    })
    .await
    .unwrap();
    assert!(frames.borrow().is_some());
    for body in [
        json!({"alt_deg":91,"az_deg":200}),
        json!({"alt_deg":40}),
        json!({"eyepiece_on_star":false}),
        json!({"alt_deg":40,"az_deg":200,"extra":true}),
        json!({"alt_deg":40,"az_deg":200,"turn_deg":1,"tilt_deg":1}),
    ] {
        assert_eq!(
            request(&app, "POST", "/api/sim/telescope", body).await.0,
            400
        );
    }
    let response = app
        .oneshot(
            Request::post("/api/sim/telescope")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 415);
    assert!(!to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test(start_paused = true)]
async fn frames_stop_and_resume_and_camera_settings_still_reach_the_private_node() {
    let state = AppState::shared(DB.clone(), Samples::default())
        .with_simulation(START, SimulationTransport::Memory)
        .await
        .unwrap()
        .with_live();
    let mut frames = state.link().unwrap().frames();
    let mut events = state.live().unwrap().subscribe();
    let app = finder::router(state);
    event(&mut events).await;
    assert_eq!(
        request(&app, "POST", "/api/sim/frames", json!({"running":false}))
            .await
            .0,
        200
    );
    // Let an exposure already in progress finish before testing the stopped interval.
    tokio::time::sleep(Duration::from_secs(3)).await;
    frames.borrow_and_update();
    assert!(
        tokio::time::timeout(Duration::from_secs(2), frames.changed())
            .await
            .is_err()
    );
    assert_eq!(
        request(
            &app,
            "PUT",
            "/api/session",
            json!({"camera_settings":{"exposure_ms":150,"gain":2,"sigma":7}})
        )
        .await
        .0,
        200
    );
    assert_eq!(
        request(&app, "POST", "/api/sim/frames", json!({"running":true}))
            .await
            .0,
        200
    );
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            frames.changed().await.unwrap();
            let frame = frames.borrow_and_update().clone().unwrap();
            if frame.settings.as_ref().and_then(|s| s.exposure_ms) == Some(150.0) {
                assert_eq!(frame.settings.unwrap().gain, Some(2.0));
                assert_eq!(frame.exposure_ms, 150.0);
                break;
            }
        }
    })
    .await
    .unwrap();
    let (_, read): (_, Value) = request(&app, "GET", "/api/session", json!(null)).await;
    assert_eq!(read["node"]["connected"], true);
}
