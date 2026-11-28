//! Oracles: the simulator's true tube and imaging truth, its own horizon and precession
//! transforms (the sky turns under a still mount), the steps' sizes and the ways'
//! angles, and the words of spec.md US2 and FR-004 and FR-005. No expected value is
//! read from the code under test.
use super::nudge::{miss, moved, nudge, refused, rest, synced, write, STEPS, WAYS};
use crate::support::{
    goto::{ConsoleClock, Rig, Stars, DB, START},
    live::request,
    Scratch,
};
use axum::{http::StatusCode, Router};
use finder::{imaging, samples::Samples, session::Session, AppState};
use math_core::UnitVector;
use scope_link::memory;
use scope_node::service::NodeControls;
use serde_json::{json, Value};
use simulator::{
    imaging::ImagingOptics,
    mount::SimMount,
    sky::{self, Site},
    SimNode, Simulator, VirtualClock,
};
use std::{
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};

/// The imaging rig: the tracking rig of `imaging_track.rs` (the simulator on the
/// shared database, the mount connected and pushed to 50° and 120°, the node over an
/// in-memory duplex at a 500 ms frame interval, the session at 40° N 75° W with M42,
/// the imaging folder and the simulated solver) with the imaging optics at imaging
/// angle 100°. It is built as a goto `Rig` so the US1 helpers read it.
fn imaging_rig(root: &Path) -> Rig {
    let sim = Simulator::new(DB.clone(), Site::default(), VirtualClock::new(START));
    let mount = SimMount::new(sim.clock().clone());
    sim.connect_mount(mount.clone());
    sim.push_to(50.0, 120.0);
    sim.set_imaging_optics(ImagingOptics {
        angle_deg: 100.0,
        ..ImagingOptics::default()
    });
    let node = Arc::new(
        SimNode::new(sim.clone(), "test-token", Default::default())
            .unwrap()
            .with_frame_interval(Duration::from_millis(500)),
    );
    let session = Session::default();
    session
        .edit(
            serde_json::from_value(json!({
                "node": {"address": "scope.local:1", "token": "test-token"},
                "site": {"lat_deg": 40, "lon_deg": -75},
                "target": "M42"
            }))
            .unwrap(),
            START,
        )
        .unwrap();
    let state = AppState::shared(DB.clone(), Samples::default())
        .with_session(session)
        .with_clock(Arc::new(ConsoleClock(sim.clock().clone())))
        .with_link(duplex(&node))
        .with_live_solver(Arc::new(Stars))
        .with_imaging(root)
        .with_imaging_solver(Arc::new(imaging::simulated::SimulatedSolver::new(
            sim.clone(),
        )));
    Rig {
        app: finder::router(state.clone()),
        sim,
        mount,
        state,
        stops: Arc::new(Mutex::new(Vec::new())),
        controls: NodeControls::default(),
    }
}

/// The node's routes over a fresh duplex for each connection, as `imaging_track.rs`.
fn duplex(node: &Arc<SimNode>) -> finder::link::Transport {
    let node = node.clone();
    finder::link::Transport::connector(move || {
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
    })
}

/// Reads `GET /api/imaging` every 100 ms of virtual time until `check` holds, for at
/// most 300 s.
async fn imaging_until(app: &Router, check: impl Fn(&Value) -> bool) -> Value {
    for _ in 0..3000 {
        let (_, view) = request(app, "GET", "/api/imaging", json!(null)).await;
        if check(&view) {
            return view;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("imaging never reached the awaited state");
}

/// Reads the pointing model every 100 ms of virtual time until it holds a sync.
async fn synced_model(rig: &Rig) {
    for _ in 0..3000 {
        if !rig.state.session().model(rig.now()).syncs.is_empty() {
            return;
        }
        rig.tick().await;
    }
    panic!("the pointing model never held a sync");
}

/// Where the frame's centre would be at `t1` had the mount not moved: its J2000 place
/// at `t0` as an altitude and azimuth, turned back to J2000 at `t1` as the sky turns
/// under a still mount, with the simulator's own horizon and precession transforms.
fn where_it_would_be(ra_deg: f64, dec_deg: f64, t0: f64, t1: f64) -> UnitVector {
    let site = Site {
        lat_deg: 40.0,
        lon_deg: -75.0,
    };
    let place = UnitVector::from_radec(ra_deg.to_radians(), dec_deg.to_radians());
    let (alt, az) = sky::horizontal(place, site, t0);
    sky::to_j2000(
        sky::horizontal_to_date(alt, az, site, t1),
        sky::julian_date(t1),
    )
}

/// The displacement from `from` to `to`, in arcseconds: `[east, north]` at `from`.
fn shift(from: UnitVector, to: UnitVector) -> [f64; 2] {
    let (ra, dec) = from.to_radec();
    let d = [to.x - from.x, to.y - from.y, to.z - from.z];
    let east = [-ra.sin(), ra.cos(), 0.0];
    let north = [-dec.sin() * ra.cos(), -dec.sin() * ra.sin(), dec.cos()];
    let arcsec = 180.0 * 3600.0 / std::f64::consts::PI;
    let along = |u: [f64; 3]| (0..3).map(|i| u[i] * d[i]).sum::<f64>() * arcsec;
    [along(east), along(north)]
}

// US2-S1: the finder camera's view refuses with no live pointing, then turns the steps
// with the chart rotation, 200° here, and the zenith at about 0°.
#[tokio::test(start_paused = true)]
async fn the_finder_camera_view_follows_the_live_frame_turned_by_the_chart_rotation() {
    let bare = finder::router(AppState::shared(DB.clone(), Samples::default()));
    refused(
        &bare,
        "up",
        "fine",
        "finder_camera",
        "the finder camera has no live pointing yet",
    )
    .await;

    let rig = synced(50.0).await;
    let (status, body) = write(
        &rig.app,
        "PUT",
        "/api/session",
        json!({"chart_rotation_deg": 200}),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let up = rig.view().await["pointing"]["up_deg"].as_f64().unwrap();
    assert!(
        up < 1.0 || up > 359.0,
        "the live pointing reads up_deg {up}, not near 0"
    );

    for (step, size) in STEPS {
        for (way, angle) in WAYS {
            let before = rig.mount.true_direction();
            let (status, body) = nudge(&rig.app, way, step, "finder_camera").await;
            assert_eq!(status, 200, "{way} {step}: {body}");
            assert_eq!(body["moved"], "axes");
            rest(&rig).await;
            let landed = miss(
                moved(before, rig.mount.true_direction()),
                size,
                angle + 200.0,
            );
            assert!(
                landed < 0.03,
                "{way} {step} missed by {landed:.4} of its step"
            );
        }
    }
}

// US2-S3: the imaging camera's view refuses until a frame has solved, then every step
// moves the frame's centre by the step at the position angle of an unmirrored frame.
#[tokio::test(start_paused = true)]
async fn the_imaging_camera_view_follows_the_frame_from_its_last_solve() {
    let bare = finder::router(AppState::shared(DB.clone(), Samples::default()));
    refused(
        &bare,
        "up",
        "fine",
        "imaging_camera",
        "the imaging camera has not solved a frame yet",
    )
    .await;

    let scratch = Scratch::new("nudge-views-imaging");
    let rig = imaging_rig(&scratch.0.join("imaging"));
    imaging_until(&rig.app, |view| view["state"] == "ready").await;
    synced_model(&rig).await;
    refused(
        &rig.app,
        "up",
        "fine",
        "imaging_camera",
        "the imaging camera has not solved a frame yet",
    )
    .await;

    // Fine down and then fine left in the mount's view, each read until it is done.
    for direction in ["down", "left"] {
        let (status, body) = nudge(&rig.app, direction, "fine", "mount").await;
        assert_eq!(status, 200, "{body}");
        rest(&rig).await;
    }

    // One imaging frame, saved, then solved.
    let (status, view) = request(&rig.app, "POST", "/api/imaging/frame", json!(null)).await;
    assert_eq!(status, StatusCode::OK, "{view}");
    let id = view["frame"]["id"].as_u64().unwrap();
    imaging_until(&rig.app, |view| {
        view["frame"]["id"] == json!(id) && view["frame"]["state"] == "saved"
    })
    .await;
    let (status, answer) = request(&rig.app, "POST", "/api/imaging/solve", json!(null)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let view = imaging_until(&rig.app, |view| {
        view["solve"]["frame_id"] == json!(id) && view["solve"]["state"] != "solving"
    })
    .await;
    assert_eq!(view["solve"]["state"], json!("solved"), "{}", view["solve"]);

    for (step, size) in STEPS {
        for (way, angle) in WAYS {
            let t0 = rig.now();
            let before = rig.sim.imaging_truth();
            let (status, body) = nudge(&rig.app, way, step, "imaging_camera").await;
            assert_eq!(status, 200, "{way} {step}: {body}");
            assert_eq!(body["moved"], "axes");
            rest(&rig).await;
            assert_eq!(rig.view().await["axis_move"]["state"], "done");
            let t1 = rig.now();
            let after = rig.sim.imaging_truth();
            let would = where_it_would_be(before.ra_deg, before.dec_deg, t0, t1);
            let centre =
                UnitVector::from_radec(after.ra_deg.to_radians(), after.dec_deg.to_radians());
            let [east, north] = shift(would, centre);
            // An unmirrored frame's up lies at `up_east_of_north_deg` east of north; a
            // press's way is that many degrees clockwise of it, so its position angle
            // east of north is `up_east_of_north_deg` less the way's angle.
            let position = (before.up_east_of_north_deg - angle).to_radians();
            let landed = (east - size * position.sin()).hypot(north - size * position.cos()) / size;
            assert!(
                landed < 0.04,
                "{way} {step} missed by {landed:.4} of its step: moved {east:.1}, {north:.1} arcsec"
            );
        }
    }
}
