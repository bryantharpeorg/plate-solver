//! Whole-loop oracle: simulator's independent true eyepiece, the specified
//! (700, 420) calibration (0.5 px), and finder sky::alt_az of M78 (30 arcseconds).
#[path = "live/support.rs"]
mod support;
use finder::{samples::Samples, simulate::SimulationTransport, AppState};
use math_core::{angular_distance, UnitVector};
use serde_json::json;
use support::*;

#[tokio::test(start_paused = true)]
async fn first_frame_to_calibrating_tap_to_m78_guidance_over_the_memory_link() {
    let state = AppState::shared(DB.clone(), Samples::default())
        .with_simulation(START, SimulationTransport::Memory)
        .await
        .unwrap()
        .with_live();
    let sim = state.simulator().unwrap();
    let live = state.live().unwrap();
    let mut events = live.subscribe();
    let app = finder::router(state);
    assert_eq!(request(&app,"PUT","/api/session",json!({"site":{"lat_deg":40,"lon_deg":-75},"target":"M78","camera_settings":{"exposure_ms":200,"gain":1}})).await.0,200);
    let first = event(&mut events).await;
    assert_eq!(first.report.status, finder::solve::Status::MatchFound);
    let (_, read) = request(&app, "GET", "/api/session", json!(null)).await;
    assert!(read.get("aim").is_none());
    // Wait out the preview interval before centring a star, so the next exposure
    // carries a preview before sidereal motion moves it off the eyepiece pixel.
    assert_eq!(
        request(&app, "POST", "/api/sim/frames", json!({"running":false}))
            .await
            .0,
        200
    );
    tokio::time::sleep(std::time::Duration::from_secs(6)).await;
    let (_, pushed) = request(
        &app,
        "POST",
        "/api/sim/telescope",
        json!({"eyepiece_on_star":true}),
    )
    .await;
    assert!(pushed["eyepiece"]["ra"].is_number());
    assert_eq!(
        request(&app, "POST", "/api/sim/frames", json!({"running":true}))
            .await
            .0,
        200
    );
    // Verify a real catalog star was selected, rather than accepting a no-op.
    let truth = sim.telescope().eyepiece;
    let ray = UnitVector::from_radec(truth.ra.to_radians(), truth.dec.to_radians());
    assert!((0..DB.num_stars).any(|index| angular_distance(
        ray,
        DB.star_vector(pattern_database::StarId(index))
            .unwrap()
            .normalize()
            .unwrap()
    )
    .to_degrees()
        * 3600.0
        < 1.0));
    let preview = loop {
        let report = event(&mut events).await;
        let preview = live.preview(report.preview_seq.unwrap()).unwrap();
        if preview
            .centroids
            .iter()
            .any(|s| (s.x - 700.0).hypot(s.y - 420.0) < 0.5)
        {
            break preview;
        }
        assert!(sim.clock().now() - START < 20.0, "no star at the eyepiece");
    };
    let (status, read) = request(
        &app,
        "POST",
        "/api/aim",
        json!({"frame_id":preview.frame_id,"x":706,"y":420}),
    )
    .await;
    assert_eq!(status, 200, "{read}");
    assert!((read["aim"]["x"].as_f64().unwrap() - 700.0).abs() < 0.5);
    assert!((read["aim"]["y"].as_f64().unwrap() - 420.0).abs() < 0.5);
    let report = event(&mut events).await;
    assert_eq!(report.report.status, finder::solve::Status::MatchFound);
    let (_, read) = request(&app, "GET", "/api/session", json!(null)).await;
    let target = &read["target"];
    let time = read["time"].as_f64().unwrap();
    let (alt, az) = finder::sky::alt_az(
        target["ra_deg"].as_f64().unwrap(),
        target["dec_deg"].as_f64().unwrap(),
        &finder::sky::Observer {
            lat: 40.0,
            lon: -75.0,
            time,
        },
    );
    let truth = sim
        .directions_at(read["pointing"]["time"].as_f64().unwrap())
        .eyepiece;
    let guidance = &read["guidance"];
    assert_eq!(guidance["class"], "pointable");
    let reached = UnitVector::from_radec(
        (truth.az + guidance["turn_deg"].as_f64().unwrap()).to_radians(),
        (truth.alt + guidance["tilt_deg"].as_f64().unwrap()).to_radians(),
    );
    let error = angular_distance(
        reached,
        UnitVector::from_radec(az.to_radians(), alt.to_radians()),
    )
    .to_degrees()
        * 3600.0;
    assert!(
        error < 30.0,
        "guidance missed M78 by {error} arcseconds: {guidance}"
    );
    let streamed = serde_json::to_value(report.guidance.unwrap()).unwrap();
    assert!(
        (streamed["turn_deg"].as_f64().unwrap() - guidance["turn_deg"].as_f64().unwrap()).abs()
            < 1e-8
    );
}
