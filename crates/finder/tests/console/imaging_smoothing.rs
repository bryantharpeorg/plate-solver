//! The smoothed aim point (032 US5): the window of the last five aim
//! readings and the medians it answers with, the outliers set aside and the
//! three in a row that replace what is held, the readings kept in
//! `relation.json` across a restart, and the narrow radius a hint takes once
//! the session's aim point is measured. Oracles: the worked values of US5-S1
//! and their stated tolerances, the simulator's true aim point (700, 420)
//! and its true imaging angle, the worked arithmetic of the radius, and, for
//! the second console, the first one's own values read by the test from its
//! view and its file; nothing here is read from the code under test.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use axum::{http::StatusCode, Router};
use finder::{
    imaging::{
        self,
        relation::{should_apply, AimReading, Taken, Window},
        solver::{HintSource, Region, SolveFrame},
        solving::{hint_for, HintInputs},
    },
    samples::Samples,
    session::{AimView, Session},
    AppState,
};
use math_core::{angular_distance, UnitVector};
use scope_link::memory;
use serde_json::{json, Value};
use simulator::{
    imaging::ImagingOptics, mount::SimMount, Optics, SimNode, Simulator, Site, VirtualClock,
};

use crate::support::live as support;

/// The solving rig's simulator (the shared setup): the test database at
/// 40° N 75° W on the virtual clock from Unix 1768528800, the mount connected
/// first and pushed to alt 50°, az 120° after.
fn sim() -> Simulator {
    let sim = Simulator::new(
        support::DB.clone(),
        Site::default(),
        VirtualClock::new(support::START),
    );
    sim.connect_mount(SimMount::new(sim.clock().clone()));
    sim.push_to(50.0, 120.0);
    sim
}

/// The node's routes over a fresh duplex for each connection.
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

/// The solving rig (the shared setup): its node, the paired session, and the
/// imaging folder `root`.
struct Rig {
    app: Router,
    sim: Simulator,
}

fn rig(sim: &Simulator, root: &Path, session: Session) -> Rig {
    let node = Arc::new(
        SimNode::new(sim.clone(), "test-token", Default::default())
            .unwrap()
            .with_frame_interval(Duration::from_millis(500)),
    );
    session
        .edit(
            serde_json::from_value(json!({
                "node": {"address": "scope.local:1", "token": "test-token"},
                "site": {"lat_deg": 40, "lon_deg": -75}
            }))
            .unwrap(),
            support::START,
        )
        .unwrap();
    let solver = Arc::new(imaging::simulated::SimulatedSolver::new(sim.clone()));
    let state = AppState::shared(support::DB.clone(), Samples::default())
        .with_session(session)
        .with_clock(Arc::new(support::ConsoleClock(sim.clock().clone())))
        .with_link(duplex(&node))
        .with_live()
        .with_imaging(root)
        .with_imaging_solver(solver);
    Rig {
        app: finder::router(state),
        sim: sim.clone(),
    }
}

/// The solving rig over a fresh simulator, with the session of the caller's.
fn rig_with_session(root: &Path, session: Session) -> Rig {
    let sim = sim();
    rig(&sim, root, session)
}

/// The solving rig over a fresh simulator: the simulated solver over the
/// simulator's truth.
fn solving_rig(root: &Path) -> Rig {
    rig_with_session(root, Session::default())
}

async fn get(app: &Router, path: &str) -> (StatusCode, Value) {
    support::request(app, "GET", path, json!(null)).await
}

async fn post(app: &Router, path: &str) -> (StatusCode, Value) {
    support::request(app, "POST", path, json!(null)).await
}

/// Read `GET /api/imaging` every 100 ms until `check` holds, for at most 60 s.
async fn wait_until(app: &Router, check: impl Fn(&Value) -> bool) -> Value {
    for _ in 0..600 {
        let (_, view) = get(app, "/api/imaging").await;
        if check(&view) {
            return view;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let (_, view) = get(app, "/api/imaging").await;
    panic!("imaging never reached the awaited state: {view}");
}

/// Read `GET /api/session` every 100 ms until `check` holds, for at most
/// 300 s.
async fn wait_session(app: &Router, check: impl Fn(&Value) -> bool) -> Value {
    for _ in 0..3000 {
        let (_, session) = get(app, "/api/session").await;
        if check(&session) {
            return session;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let (_, session) = get(app, "/api/session").await;
    panic!("the session never reached the awaited state: {session}");
}

async fn ready(app: &Router) -> Value {
    wait_until(app, |view| view["state"] == "ready").await
}

/// Wait until the session's `pointing` has the `source` `live`.
async fn pointing_live(app: &Router) {
    wait_session(app, |session| session["pointing"]["source"] == "live").await;
}

/// The 5 s more the scenarios wait after the pointing is live, so the fit's
/// reach holds the finder camera's solves.
async fn settle() {
    tokio::time::sleep(Duration::from_secs(5)).await;
}

/// A saved frame (the shared setup): one frame asked for, awaited until
/// saved. Answers the view with the frame saved.
async fn saved_frame(app: &Router) -> Value {
    let (status, view) = post(app, "/api/imaging/frame").await;
    assert_eq!(status, StatusCode::OK, "{view}");
    let id = view["frame"]["id"].clone();
    wait_until(app, |view| {
        view["frame"]["id"] == id && view["frame"]["state"] == "saved"
    })
    .await
}

/// POST the solve of the latest saved frame and wait until it has ended.
async fn solve_saved(app: &Router, id: u64) -> Value {
    let (status, answer) = post(app, "/api/imaging/solve").await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    wait_until(app, |view| {
        view["solve"]["frame_id"] == json!(id) && view["solve"]["state"] != "solving"
    })
    .await
}

/// One frame taken and solved: the view once the solve has ended.
async fn solve_frame(app: &Router) -> Value {
    let view = saved_frame(app).await;
    solve_saved(app, view["frame"]["id"].as_u64().unwrap()).await
}

/// The great-circle separation of two places, in degrees.
fn separation_deg(ra1: f64, dec1: f64, ra2: f64, dec2: f64) -> f64 {
    angular_distance(
        UnitVector::from_radec(ra1.to_radians(), dec1.to_radians()),
        UnitVector::from_radec(ra2.to_radians(), dec2.to_radians()),
    )
    .to_degrees()
}

/// An aim reading of the test's own making (US5-S1): the stated place and
/// angle, not mirrored, made in a finder frame 1456 × 1088 at 30″ a pixel.
fn reading(x: f64, y: f64, angle_deg: f64) -> AimReading {
    AimReading {
        x,
        y,
        angle_deg,
        mirrored: false,
        at: 0.0,
        frame_id: 0,
        finder_arcsec_per_px: 30.0,
        finder_width: 1456.0,
        finder_height: 1088.0,
    }
}

/// US5-S1: five aim readings give the medians of their x, their y and their
/// angle taken around the circle, the sixth pushes the first out, and two
/// readings either side of north give 0° (oracle: the worked values of the
/// scenario, within 1e-9).
#[test]
fn the_window_answers_the_medians_of_its_readings() {
    let mut window = Window::new();
    for (x, y, angle) in [
        (700.0, 420.0, 17.0),
        (702.0, 421.0, 17.2),
        (698.0, 419.0, 16.8),
        (701.0, 420.5, 17.1),
        (699.0, 419.5, 16.9),
    ] {
        assert_eq!(window.take(reading(x, y, angle)), Taken::Measured);
    }
    assert_eq!(window.len(), 5);
    let (x, y) = window.aim().unwrap();
    assert!((x - 700.0).abs() < 1e-9, "x is {x}");
    assert!((y - 420.0).abs() < 1e-9, "y is {y}");
    assert!((window.angle_deg().unwrap() - 17.0).abs() < 1e-9);

    // The sixth: the first has left (FR-024).
    assert_eq!(window.take(reading(703.0, 422.0, 17.3)), Taken::Measured);
    assert_eq!(window.len(), 5);
    let (x, y) = window.aim().unwrap();
    assert!((x - 701.0).abs() < 1e-9, "x is {x}");
    assert!((y - 420.5).abs() < 1e-9, "y is {y}");

    // Two readings either side of north, and five around it (FR-024).
    let mut window = Window::new();
    window.take(reading(700.0, 420.0, 359.0));
    window.take(reading(702.0, 421.0, 1.0));
    let (x, y) = window.aim().unwrap();
    assert!((x - 701.0).abs() < 1e-9, "x is {x}");
    assert!((y - 420.5).abs() < 1e-9, "y is {y}");
    assert!((window.angle_deg().unwrap() - 0.0).abs() < 1e-9);
    let mut window = Window::new();
    for angle in [359.8, 0.2, 0.0, 359.6, 0.4] {
        window.take(reading(700.0, 420.0, angle));
    }
    assert!((window.angle_deg().unwrap() - 0.0).abs() < 1e-9);
}

/// US5-S1: with three or more held, a reading 150″ away, 1.5° of angle away
/// or of the other mirror sense is set aside and moves nothing; a reading
/// 90″ away is taken and clears those set aside; the third set aside in a row
/// replaces what is held; with fewer than three held nothing is an outlier
/// (oracle: the worked values of the scenario, at 30″ a pixel).
#[test]
fn the_window_sets_outliers_aside() {
    let mut window = Window::new();
    for _ in 0..3 {
        assert_eq!(window.take(reading(700.0, 420.0, 17.0)), Taken::Measured);
    }
    let (aim, len) = (window.aim().unwrap(), window.len());
    assert_eq!(window.take(reading(705.0, 420.0, 17.0)), Taken::SetAside);
    assert_eq!(window.aim().unwrap(), aim, "the aim point moved");
    assert_eq!(window.len(), len, "the window changed");
    assert_eq!(window.set_aside(), 1);
    assert_eq!(window.take(reading(700.0, 420.0, 18.5)), Taken::SetAside);
    assert_eq!(window.set_aside(), 2);
    // 90″ away is within the 120″ allowed, and clears those set aside.
    assert_eq!(window.take(reading(703.0, 420.0, 17.0)), Taken::Measured);
    assert_eq!(window.set_aside(), 0);

    // The other mirror sense is an outlier too, and taken third in a row
    // replaces what is held (FR-025).
    let mut turned = Window::new();
    for _ in 0..3 {
        turned.take(reading(700.0, 420.0, 17.0));
    }
    let mut mirrored = reading(700.0, 420.0, 17.0);
    mirrored.mirrored = true;
    assert_eq!(turned.take(mirrored), Taken::SetAside);
    assert_eq!(turned.take(mirrored), Taken::SetAside);
    assert_eq!(turned.take(mirrored), Taken::Replaced);

    // With fewer than three held nothing is an outlier.
    let mut two = Window::new();
    two.take(reading(700.0, 420.0, 17.0));
    two.take(reading(700.0, 420.0, 17.0));
    assert_eq!(two.take(reading(760.0, 400.0, 40.0)), Taken::Measured);
}

/// US5-S1: three readings at (760, 400) and 40° in a row are set aside, set
/// aside and then replace the window with themselves (oracle: the worked
/// values of the scenario, within 1e-9).
#[test]
fn three_outliers_in_a_row_replace_the_window() {
    let mut window = Window::new();
    for _ in 0..3 {
        window.take(reading(700.0, 420.0, 17.0));
    }
    for _ in 0..2 {
        assert_eq!(window.take(reading(760.0, 400.0, 40.0)), Taken::SetAside);
    }
    assert_eq!(window.take(reading(760.0, 400.0, 40.0)), Taken::Replaced);
    assert_eq!(window.len(), 3);
    assert_eq!(window.set_aside(), 0);
    let (x, y) = window.aim().unwrap();
    assert!((x - 760.0).abs() < 1e-9, "x is {x}");
    assert!((y - 400.0).abs() < 1e-9, "y is {y}");
    assert!((window.angle_deg().unwrap() - 40.0).abs() < 1e-9);
}

/// US5-S1: a reading made in a finder frame of another size drops everything
/// held and set aside, since its pixels mean something else (oracle: the
/// worked values of the scenario).
#[test]
fn a_reading_of_another_finder_frame_drops_what_is_held() {
    let mut window = Window::new();
    for _ in 0..3 {
        window.take(reading(700.0, 420.0, 17.0));
    }
    let mut small = reading(350.0, 210.0, 17.0);
    small.finder_width = 728.0;
    small.finder_height = 544.0;
    assert_eq!(window.take(small), Taken::Measured);
    assert_eq!(window.len(), 1);
    let (x, y) = window.aim().unwrap();
    assert!((x - 350.0).abs() < 1e-9, "x is {x}");
    assert!((y - 210.0).abs() < 1e-9, "y is {y}");
}

/// US5-S1: the session's aim point is set from the medians with none in
/// force, with a tapped one, and with a measured one only beyond 10″ on the
/// sky (oracle: the worked values of the scenario, at 30″ a pixel).
#[test]
fn should_apply_moves_the_aim_point_only_beyond_the_step() {
    let measured = AimView {
        x: 700.0,
        y: 420.0,
        source: Some("measured"),
    };
    assert!(
        should_apply(None, (700.0, 420.0), 30.0),
        "no aim point in force"
    );
    assert!(
        should_apply(
            Some(AimView {
                x: 700.0,
                y: 420.0,
                source: None
            }),
            (700.0, 420.0),
            30.0
        ),
        "a tapped aim point"
    );
    assert!(
        !should_apply(Some(measured), (700.3, 420.0), 30.0),
        "9 arcseconds away"
    );
    assert!(
        should_apply(Some(measured), (700.4, 420.0), 30.0),
        "12 arcseconds away"
    );
}

/// US5-S2: six solves hold five aim readings, the session's aim point is
/// measured at the simulator's true one, the angle is the true 17°, the
/// median stays within 10″ of the aim point in force, and the sixth solve is
/// hinted within the sensor's diagonal on the sky, its place within 0.05° of
/// the solved centre (oracle: the simulator's true aim point (700, 420) and
/// its true imaging angle 17°, and the worked radius
/// `hypot(1236, 822)·scale/3600`).
#[tokio::test(start_paused = true)]
async fn six_solves_smooth_the_aim_point_and_narrow_the_hint() {
    let scratch = crate::support::Scratch::new("imaging-smoothing-s2a");
    let folder = scratch.0.join("imaging");
    let rig = solving_rig(&folder);
    ready(&rig.app).await;
    pointing_live(&rig.app).await;
    settle().await;

    let mut view = Value::Null;
    for n in 0..6 {
        view = solve_frame(&rig.app).await;
        assert_eq!(view["solve"]["state"], json!("solved"), "{}", view["solve"]);
        if n >= 4 {
            assert_eq!(
                view["relation"]["readings"],
                json!(5),
                "{}",
                view["relation"]
            );
        }
    }
    let relation = &view["relation"];
    let (_, session) = get(&rig.app, "/api/session").await;
    let aim = &session["aim"];
    assert_eq!(aim["source"], json!("measured"), "{aim}");
    assert!((aim["x"].as_f64().unwrap() - 700.0).abs() < 1.5, "{aim}");
    assert!((aim["y"].as_f64().unwrap() - 420.0).abs() < 1.5, "{aim}");
    assert!(
        (relation["angle_deg"].as_f64().unwrap() - 17.0).abs() < 0.25,
        "{relation}"
    );
    let distance = (relation["aim"]["x"].as_f64().unwrap() - aim["x"].as_f64().unwrap())
        .hypot(relation["aim"]["y"].as_f64().unwrap() - aim["y"].as_f64().unwrap());
    assert!(
        distance < 10.0 / 29.78,
        "relation.aim is {distance} px from the session's aim"
    );

    // The sixth solve's hint: the sensor's diagonal at the kept scale
    // (FR-028), its place the aim point's own direction.
    let scale = relation["scale"]["arcsec_per_px"].as_f64().unwrap();
    let radius = 1236.0_f64.hypot(822.0) * scale / 3600.0;
    assert!(
        (view["solve"]["hint"]["radius_deg"].as_f64().unwrap() - radius).abs() < 0.001,
        "the hint is {}",
        view["solve"]["hint"]
    );
    let separation = separation_deg(
        view["solve"]["hint"]["ra_deg"].as_f64().unwrap(),
        view["solve"]["hint"]["dec_deg"].as_f64().unwrap(),
        view["solve"]["ra_deg"].as_f64().unwrap(),
        view["solve"]["dec_deg"].as_f64().unwrap(),
    );
    assert!(
        separation < 0.05,
        "the hint is {separation}° from the solved centre"
    );
}

/// US5-S2: `hint_for` gives a hint from a live pointing the sensor's diagonal
/// once the session's aim point is measured, never under 0.2° or over 3°, and
/// 3° while it is not; the corner region's centre is farther from the
/// sensor's centre than the region's own diagonal, which is why the radius is
/// the sensor's (oracle: the worked arithmetic
/// `hypot(1236, 822)·2.1845/3600` = 0.900729, `205·2.1845/3600` = 0.124395,
/// `hypot(463.5, 308.5)·2.1845/3600` = 0.338 and `hypot(309, 205)·2.1845/
/// 3600` = 0.225).
#[test]
fn hint_for_narrows_a_pointing_hint_to_the_sensors_diagonal() {
    // A quarter-size corner of the sensor 1236 × 822, at binning 1.
    let frame = SolveFrame {
        path: PathBuf::new(),
        width: 309,
        height: 205,
        bits: 16,
        bayer: None,
        binning: 1,
        roi: Region {
            x: 927,
            y: 617,
            width: 309,
            height: 205,
        },
        start: 1_768_528_810.0,
        exposure_s: 1.0,
    };
    let inputs = |sensor: (u32, u32), scale: f64, aim_measured: bool| HintInputs {
        frame: &frame,
        sensor,
        pointing: Some((80.0, -5.0, 1_768_528_810.5)),
        last_solve: None,
        target: None,
        scale: Some(scale),
        pixel_um: 19.12,
        focal_length_mm: None,
        aim_measured,
        skip: 0,
    };
    let hint = hint_for(&inputs((1236, 822), 2.1845, true));
    assert_eq!(hint.source, HintSource::Pointing);
    assert!(
        (hint.radius_deg - 0.900_729).abs() < 1e-5,
        "the radius is {}",
        hint.radius_deg
    );
    assert!((hint.fov_deg.unwrap() - 0.124_395).abs() < 1e-5);
    // Without a measured aim point the search stays wide (FR-028).
    assert!((hint_for(&inputs((1236, 822), 2.1845, false)).radius_deg - 3.0).abs() < 1e-9);
    // The floor and the ceiling (FR-028): a sensor 100 × 80 at a scale of 2.0
    // gives 0.2, and a sensor 6000 × 4000 gives 3.0.
    assert!((hint_for(&inputs((100, 80), 2.0, true)).radius_deg - 0.2).abs() < 1e-9);
    assert!((hint_for(&inputs((6000, 4000), 2.0, true)).radius_deg - 3.0).abs() < 1e-9);
    // The corner region's centre is 0.338° from the sensor's centre and its
    // own diagonal is 0.225°: the first is under the hint's radius and over
    // the second.
    let centre = (1081.5_f64 - 618.0).hypot(719.5 - 411.0) * 2.1845 / 3600.0;
    let diagonal = 309.0_f64.hypot(205.0) * 2.1845 / 3600.0;
    assert!((centre - 0.338).abs() < 5e-4, "the centre is {centre}° out");
    assert!(
        (diagonal - 0.225).abs() < 5e-4,
        "the diagonal is {diagonal}°"
    );
    assert!(
        centre < hint.radius_deg,
        "the centre is not inside the hint"
    );
    assert!(centre > diagonal, "the frame's diagonal would cover it");
}

/// US5-S3: a camera turned in the focuser to 40° is set aside twice and then
/// replaces the window, the session's aim point standing still throughout,
/// and a telescope moved in the finder camera's frame is followed after three
/// solves (oracle: the simulator's true imaging angles 17° and 40°, and its
/// true aim point, the eyepiece pixel (760, 400)).
#[tokio::test(start_paused = true)]
async fn a_turned_camera_takes_over_and_a_moved_aim_point_is_followed() {
    let scratch = crate::support::Scratch::new("imaging-smoothing-s3");
    let folder = scratch.0.join("imaging");
    let rig = solving_rig(&folder);
    ready(&rig.app).await;
    pointing_live(&rig.app).await;
    settle().await;

    for _ in 0..3 {
        let view = solve_frame(&rig.app).await;
        assert_eq!(view["solve"]["state"], json!("solved"), "{}", view["solve"]);
    }
    let (_, session) = get(&rig.app, "/api/session").await;
    let aim = (
        session["aim"]["x"].as_f64().unwrap(),
        session["aim"]["y"].as_f64().unwrap(),
    );
    assert_eq!(session["aim"]["source"], json!("measured"), "{session}");

    // The camera turned in the focuser: the angle moves, the place does not.
    rig.sim.set_imaging_optics(ImagingOptics {
        angle_deg: 40.0,
        ..Default::default()
    });
    for n in 0..3 {
        let view = solve_frame(&rig.app).await;
        assert_eq!(view["solve"]["state"], json!("solved"), "{}", view["solve"]);
        let relation = &view["relation"];
        if n < 2 {
            assert_eq!(
                relation["last"]["outcome"],
                json!("set_aside"),
                "{relation}"
            );
            assert_eq!(relation["set_aside"], json!(n + 1), "{relation}");
            assert!(
                (relation["angle_deg"].as_f64().unwrap() - 17.0).abs() < 0.25,
                "{relation}"
            );
        } else {
            assert_eq!(relation["last"]["outcome"], json!("replaced"), "{relation}");
            assert_eq!(relation["readings"], json!(3), "{relation}");
            assert_eq!(relation["set_aside"], json!(0), "{relation}");
            assert!(
                (relation["angle_deg"].as_f64().unwrap() - 40.0).abs() < 0.25,
                "{relation}"
            );
        }
        let (_, session) = get(&rig.app, "/api/session").await;
        assert!(
            (session["aim"]["x"].as_f64().unwrap() - aim.0).abs() < 1e-9
                && (session["aim"]["y"].as_f64().unwrap() - aim.1).abs() < 1e-9,
            "the aim point moved: {session}"
        );
    }

    // The telescope moved in the finder camera's frame: three solves and the
    // window has followed.
    rig.sim.set_optics(Optics {
        eyepiece_px: (760.0, 400.0),
        ..Default::default()
    });
    for _ in 0..3 {
        let view = solve_frame(&rig.app).await;
        assert_eq!(view["solve"]["state"], json!("solved"), "{}", view["solve"]);
    }
    let (_, session) = get(&rig.app, "/api/session").await;
    let aim = &session["aim"];
    assert!((aim["x"].as_f64().unwrap() - 760.0).abs() < 1.5, "{aim}");
    assert!((aim["y"].as_f64().unwrap() - 400.0).abs() < 1.5, "{aim}");
}

/// US5-S4: a second console over the same imaging folder and session file
/// reads the three aim readings the first kept, shows the first's last
/// values, and its first solve leaves four (oracle: the first console's own
/// values, read by the test from its view and from its file).
#[tokio::test(start_paused = true)]
async fn a_second_console_keeps_the_readings_the_first_measured() {
    let scratch = crate::support::Scratch::new("imaging-smoothing-s4a");
    let folder = scratch.0.join("imaging");
    let path = scratch.0.join("session.json");
    let rig = rig_with_session(&folder, Session::with_state_file(&path).unwrap());
    ready(&rig.app).await;
    pointing_live(&rig.app).await;
    settle().await;
    for _ in 0..3 {
        let view = solve_frame(&rig.app).await;
        assert_eq!(view["solve"]["state"], json!("solved"), "{}", view["solve"]);
    }
    let (_, first) = get(&rig.app, "/api/imaging").await;
    let (aim, angle) = (
        (
            first["relation"]["aim"]["x"].as_f64().unwrap(),
            first["relation"]["aim"]["y"].as_f64().unwrap(),
        ),
        first["relation"]["angle_deg"].as_f64().unwrap(),
    );
    drop(rig);

    let file: Value =
        serde_json::from_slice(&std::fs::read(folder.join("relation.json")).unwrap()).unwrap();
    assert_eq!(
        file["readings"].as_array().unwrap().len(),
        3,
        "relation.json holds {} readings",
        file["readings"]
    );

    let second = rig_with_session(&folder, Session::with_state_file(&path).unwrap());
    ready(&second.app).await;
    let (_, view) = get(&second.app, "/api/imaging").await;
    let relation = &view["relation"];
    assert_eq!(relation["readings"], json!(3), "{relation}");
    assert!(
        (relation["aim"]["x"].as_f64().unwrap() - aim.0).abs() < 1e-9,
        "{relation}"
    );
    assert!(
        (relation["aim"]["y"].as_f64().unwrap() - aim.1).abs() < 1e-9,
        "{relation}"
    );
    assert!(
        (relation["angle_deg"].as_f64().unwrap() - angle).abs() < 1e-9,
        "{relation}"
    );
    pointing_live(&second.app).await;
    settle().await;
    let view = solve_frame(&second.app).await;
    assert_eq!(view["solve"]["state"], json!("solved"), "{}", view["solve"]);
    assert_eq!(
        view["relation"]["readings"],
        json!(4),
        "{}",
        view["relation"]
    );
}

/// US5-S4: without the session file the second console keeps the scale but
/// holds no aim reading — its relation has `readings` 0 and no `aim` — and
/// its first solve drops the three stored readings, since the session's aim
/// point is not the measured one, and leaves one (oracle: the stated keys, and
/// the file's three readings read by the test).
#[tokio::test(start_paused = true)]
async fn a_second_console_without_the_session_file_drops_the_stored_readings() {
    let scratch = crate::support::Scratch::new("imaging-smoothing-s4b");
    let folder = scratch.0.join("imaging");
    let path = scratch.0.join("session.json");
    let rig = rig_with_session(&folder, Session::with_state_file(&path).unwrap());
    ready(&rig.app).await;
    pointing_live(&rig.app).await;
    settle().await;
    for _ in 0..3 {
        let view = solve_frame(&rig.app).await;
        assert_eq!(view["solve"]["state"], json!("solved"), "{}", view["solve"]);
    }
    drop(rig);
    std::fs::remove_file(&path).unwrap();

    let second = rig_with_session(&folder, Session::with_state_file(&path).unwrap());
    ready(&second.app).await;
    let (_, view) = get(&second.app, "/api/imaging").await;
    let relation = &view["relation"];
    assert_eq!(relation["readings"], json!(0), "{relation}");
    assert!(relation.get("aim").is_none(), "{relation}");
    assert!(relation.get("scale").is_some(), "{relation}");
    pointing_live(&second.app).await;
    settle().await;
    let view = solve_frame(&second.app).await;
    assert_eq!(view["solve"]["state"], json!("solved"), "{}", view["solve"]);
    assert_eq!(
        view["relation"]["readings"],
        json!(1),
        "{}",
        view["relation"]
    );
}

/// US5-S4: a `relation.json` of another camera is dropped with its scale,
/// leaving the second console no `relation` key before its first solve, while
/// the session's aim point stays the measured one; the first solve leaves one
/// reading (oracle: the stated keys, and the file the test rewrote).
#[tokio::test(start_paused = true)]
async fn a_second_console_drops_the_readings_of_another_camera() {
    let scratch = crate::support::Scratch::new("imaging-smoothing-s4c");
    let folder = scratch.0.join("imaging");
    let path = scratch.0.join("session.json");
    let rig = rig_with_session(&folder, Session::with_state_file(&path).unwrap());
    ready(&rig.app).await;
    pointing_live(&rig.app).await;
    settle().await;
    for _ in 0..3 {
        let view = solve_frame(&rig.app).await;
        assert_eq!(view["solve"]["state"], json!("solved"), "{}", view["solve"]);
    }
    drop(rig);

    let relation_path = folder.join("relation.json");
    let mut file: Value = serde_json::from_slice(&std::fs::read(&relation_path).unwrap()).unwrap();
    file["camera"]["serial"] = json!("OTHER");
    std::fs::write(&relation_path, serde_json::to_vec(&file).unwrap()).unwrap();

    let second = rig_with_session(&folder, Session::with_state_file(&path).unwrap());
    ready(&second.app).await;
    let (_, view) = get(&second.app, "/api/imaging").await;
    assert!(view.get("relation").is_none(), "{view}");
    let (_, session) = get(&second.app, "/api/session").await;
    assert_eq!(session["aim"]["source"], json!("measured"), "{session}");
    pointing_live(&second.app).await;
    settle().await;
    let view = solve_frame(&second.app).await;
    assert_eq!(view["solve"]["state"], json!("solved"), "{}", view["solve"]);
    assert_eq!(
        view["relation"]["readings"],
        json!(1),
        "{}",
        view["relation"]
    );
}
