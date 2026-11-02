//! Independent oracles: simulator counts (one count), its exact base tilt (1′),
//! solved charts, and the Sun's upper transit at 40 N, 75 W (17:10 UT ± 5 min).
#[path = "live/support.rs"]
mod support;
use finder::{
    samples::Samples,
    session::{Aim, Session},
    AppState,
};
use scope_link::{memory, proto};
use scope_node::mount::{Axis, Mount};
use serde_json::json;
use simulator::{
    mount::{MountConfig, SimMount},
    SimNode, Simulator,
};
use std::{sync::Arc, time::Duration};
use support::*;

// Independently undo the simulator's specified one-degree rotation to place
// each sync at the requested true horizontal coordinates (Rodrigues formula).
fn nominal(az: f64, alt: f64) -> [f64; 2] {
    let (a, e, t) = (az.to_radians(), alt.to_radians(), 130_f64.to_radians());
    let v = [e.cos() * a.cos(), e.cos() * a.sin(), e.sin()];
    let k = [-t.sin(), t.cos(), 0.0];
    let (s, c) = (-1_f64).to_radians().sin_cos();
    let dot = k[0] * v[0] + k[1] * v[1];
    let cross = [k[1] * v[2], -k[0] * v[2], k[0] * v[1] - k[1] * v[0]];
    let v = [0, 1, 2].map(|i| v[i] * c + cross[i] * s + k[i] * dot * (1.0 - c));
    [
        v[1].atan2(v[0]).to_degrees().rem_euclid(360.0),
        v[2].asin().to_degrees(),
    ]
}
fn setup(session: Session) -> (Simulator, SimMount, AppState) {
    let sim = sim();
    let mount = SimMount::with_config(
        sim.clock().clone(),
        MountConfig {
            base_tilt_deg: 1.0,
            base_tilt_toward_deg: 130.0,
            zero_points_deg: nominal(120.0, 50.0),
            backlash_deg: 0.0,
        },
    );
    sim.connect_mount(mount.clone());
    session.edit(serde_json::from_value(json!({"node":{"address":"scope.local:50051","token":"test-token"},"site":{"lat_deg":40,"lon_deg":-75},"camera_settings":{"exposure_ms":200,"gain":1}})).unwrap(),START).unwrap();
    session
        .set_aim(Some(Aim { x: 700.0, y: 420.0 }), START)
        .unwrap();
    let node = Arc::new(
        SimNode::new(sim.clone(), "test-token", Default::default())
            .unwrap()
            .with_frame_interval(Duration::from_millis(500)),
    );
    let transport = finder::link::Transport::connector(move || {
        let service = node.service();
        async move {
            let (client, server) = tokio::io::duplex(64 * 1024);
            tokio::spawn(async move {
                memory::serve_routes_on(server, service.routes())
                    .await
                    .unwrap();
            });
            Ok::<_, std::io::Error>(client)
        }
    });
    let state = AppState::shared(DB.clone(), Samples::default())
        .with_session(session)
        .with_clock(Arc::new(ConsoleClock(sim.clock().clone())))
        .with_link(transport);
    (sim, mount, state)
}
fn solved(report: &finder::live::LiveReport) {
    assert_eq!(report.report.status, finder::solve::Status::MatchFound);
    assert!(report.pointing_seq.is_some());
    assert!(!report.report.grid.is_empty());
    assert!(!report.report.stars.is_empty());
}
async fn second(
    state: &AppState,
    mount: &SimMount,
    events: &mut tokio::sync::broadcast::Receiver<finder::live::LiveReport>,
) {
    let first = event(events).await;
    solved(&first);
    let session = state.session();
    assert_eq!(session.model(first.time).syncs.len(), 1);
    let zero = nominal(120.0, 50.0);
    let target = nominal(250.0, 50.0);
    let counts = [0, 1].map(|i| ((target[i] - zero[i]) * 4500.0).round() as i32);
    let mut client = state.link().unwrap().mount().unwrap();
    for (i, axis) in [proto::Axis::Azimuth, proto::Axis::Altitude]
        .into_iter()
        .enumerate()
    {
        client.goto_axis(axis, counts[i]).await.unwrap();
    }
    loop {
        let report = event(events).await;
        let model = session.model(report.time);
        if !mount.state(Axis::Azimuth).await.moving
            && !mount.state(Axis::Altitude).await.moving
            && model.syncs.len() >= 2
            && model.syncs.last().unwrap().axis == counts.map(f64::from)
        {
            solved(&report);
            assert!((model.syncs.first().unwrap().az_deg - 120.0).abs() * 3600.0 < 30.0);
            assert!((model.syncs.last().unwrap().az_deg - 250.0).abs() * 3600.0 < 30.0);
            break;
        }
        assert!(report.time - START < 60.0);
    }
}
#[tokio::test(start_paused = true)]
async fn still_frames_at_two_per_second_keep_the_newest_mid_exposure_sync() {
    let (sim, mount, state) = setup(Session::default());
    let state = state.with_live();
    let mut events = state.live().unwrap().subscribe();
    for _ in 0..20 {
        let report = event(&mut events).await;
        solved(&report);
        let model = state.session().model(report.time);
        assert_eq!(model.syncs.len(), 1);
        let sync = &model.syncs[0];
        assert_eq!(sync.time, report.time);
        for (i, axis) in [Axis::Azimuth, Axis::Altitude].into_iter().enumerate() {
            assert!((sync.axis[i] - f64::from(mount.state(axis).await.position)).abs() <= 1.0);
        }
    }
    assert!(sim.clock().now() - START >= 9.5);
    let view = state.session().snapshot(sim.clock().now());
    assert!(view.mount.unwrap().connected);
}
#[tokio::test(start_paused = true)]
async fn two_degree_per_second_frames_are_solved_and_charted_without_syncs() {
    let (sim, mount, state) = setup(Session::default());
    mount.rate(Axis::Azimuth, 2.0).await;
    let state = state.with_live();
    let mut events = state.live().unwrap().subscribe();
    for _ in 0..4 {
        let report = event(&mut events).await;
        solved(&report);
    }
    assert!((mount.state(Axis::Azimuth).await.rate_deg_per_s - 2.0).abs() < 1e-8);
    assert!(state
        .session()
        .snapshot(sim.clock().now())
        .pointing
        .is_some());
    assert!(state.session().model(sim.clock().now()).syncs.is_empty());
}
#[tokio::test(start_paused = true)]
async fn uploaded_frame_updates_pointing_with_a_connected_mount_but_never_syncs() {
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use tower::ServiceExt;
    let (sim, _mount, state) = setup(Session::default());
    for _ in 0..100 {
        if state.link().unwrap().status().borrow().connected {
            break;
        }
        tick().await;
    }
    let mut mount = state.link().unwrap().mount().unwrap();
    assert_eq!(mount.describe_mount().await.unwrap().axes.len(), 2);
    let session = state.session();
    let app = finder::router(state);
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../reference-solutions/cedar-solve/examples/data/medium_fov/orion_belt.jpg");
    let bytes = std::fs::read(fixture).unwrap();
    let mut body=b"--model-test\r\nContent-Disposition: form-data; name=\"image\"; filename=\"stars.jpg\"\r\nContent-Type: image/jpeg\r\n\r\n".to_vec();
    body.extend(bytes);
    body.extend(b"\r\n--model-test--\r\n");
    let response = app
        .oneshot(
            Request::post("/api/solve")
                .header("content-type", "multipart/form-data; boundary=model-test")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let report: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(report["status"], "match_found");
    assert!(report["grid"]
        .as_array()
        .is_some_and(|grid| !grid.is_empty()));
    let view = session.snapshot(sim.clock().now());
    assert_eq!(
        view.pointing.unwrap().source,
        finder::session::PointingSource::Upload
    );
    assert!(session.model(sim.clock().now()).syncs.is_empty());
}
#[tokio::test(start_paused = true)]
async fn second_sync_fits_the_simulator_base_without_opening_an_epoch() {
    let (sim, mount, state) = setup(Session::default());
    let state = state.with_live();
    let mut events = state.live().unwrap().subscribe();
    second(&state, &mount, &mut events).await;
    let model = state.session().model(sim.clock().now());
    assert_eq!(model.epochs.len(), 1);
    let base = model.base_tilt.unwrap();
    assert!((base.deg() - 1.0).abs() * 60.0 < 1.0);
    assert!(
        (base.toward_deg() - 130.0).abs() * 60.0 < 1.0,
        "base: {base:?}, direction {}",
        base.toward_deg()
    );
}
#[tokio::test(start_paused = true)]
async fn durable_night_survives_local_midnight_then_expires_at_solar_noon() {
    let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target")
        .join(format!("model-session-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let path = folder.join("session.json");
    let (sim, mount, state) = setup(Session::with_state_file(&path).unwrap());
    let state = state.with_live();
    let mut events = state.live().unwrap().subscribe();
    second(&state, &mount, &mut events).await;
    let before = state.session().model(sim.clock().now());
    assert_eq!(before.epochs.len(), 1);
    assert!(before.base_tilt.is_some());
    drop(state);
    // A stored fixed calibration is an independent fixture; its values must be
    // preserved bit for bit even when the night's fitted terms expire.
    let mut stored: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    stored["model"]["fixed"] = json!({"ca":0.1,"npae":0.05,"tf":0.08,"camera":4.0});
    std::fs::write(&path, serde_json::to_vec(&stored).unwrap()).unwrap();
    let restarted = Session::with_state_file(&path).unwrap();
    let night = restarted.model(START + 3.5 * 3600.0);
    assert_eq!(night.syncs.len(), before.syncs.len());
    for (actual, expected) in night.syncs.iter().zip(&before.syncs) {
        assert_eq!(actual.axis, expected.axis);
        assert_eq!(actual.time, expected.time);
        assert_eq!(actual.epoch, expected.epoch);
        // Decimal JSON round trips to f64 within 1e-10 degrees / arcseconds.
        for (a, b) in [
            (actual.az_deg, expected.az_deg),
            (actual.alt_deg, expected.alt_deg),
            (actual.roll_deg.unwrap(), expected.roll_deg.unwrap()),
            (actual.rms_arcsec, expected.rms_arcsec),
        ] {
            assert!((a - b).abs() < 1e-10);
        }
    }
    assert_eq!(night.epochs.len(), before.epochs.len());
    for (a, b) in night.epochs.iter().zip(&before.epochs) {
        assert!((a.ia - b.ia).abs() < 1e-10);
        assert!((a.ie - b.ie).abs() < 1e-10);
    }
    let (a, b) = (night.base_tilt.unwrap(), before.base_tilt.unwrap());
    assert!((a.an - b.an).abs() < 1e-10);
    assert!((a.aw - b.aw).abs() < 1e-10);
    let noon = night.expires_at.unwrap();
    assert!((noon - (START + 15.0 * 3600.0 + 10.0 * 60.0)).abs() < 300.0);
    let fixed = night.fixed;
    assert!(fixed.is_some());
    let after = restarted.snapshot(noon + 1.0).model.unwrap();
    assert_eq!(after.syncs, 0);
    assert!(after.base_tilt.is_none());
    assert!(after.ia.is_none());
    assert!(after.ie.is_none());
    assert_eq!(after.fixed, fixed);
    let cleared = restarted.model(noon + 1.0);
    assert!(cleared.syncs.is_empty());
    assert!(cleared.epochs.is_empty());
    assert_eq!(cleared.fixed, fixed);
    let again = Session::with_state_file(&path).unwrap().model(noon + 2.0);
    assert!(again.syncs.is_empty());
    assert_eq!(again.fixed, fixed);
    std::fs::remove_dir_all(folder).unwrap();
}
