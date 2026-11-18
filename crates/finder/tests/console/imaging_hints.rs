//! The hint the console gives an imaging solve (032 US3): the wide first
//! search from the live pointing, the other sources it falls to in turn, the
//! scale a solved frame measures and keeps, and the focal length that sizes
//! the field until one is kept. Oracles: the simulator's truth and directions
//! read by the test itself, the worked arithmetic
//! `206264.806 × pixel size in mm / focal length in mm`, and the stated rules
//! of three; nothing here is read from the code under test.

use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use axum::{http::StatusCode, Router};
use finder::{
    imaging::{
        self,
        relation::{CameraId, Relation},
        solver::{
            HintSource, ImagingSolver, MeasureFuture, Region, SolveError, SolveFrame, SolveFuture,
            SolveHint,
        },
        solving::{hint_for, HintInputs},
    },
    link::Transport,
    samples::Samples,
    session::{Aim, Session},
    AppState,
};
use math_core::{angular_distance, UnitVector};
use scope_link::memory;
use serde_json::{json, Value};
use simulator::{imaging::ImagingOptics, mount::SimMount, SimNode, Simulator, Site, VirtualClock};

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
fn duplex(node: &Arc<SimNode>) -> Transport {
    let node = node.clone();
    Transport::connector(move || {
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

/// The solving rig (the shared setup): its node, the paired session, the
/// imaging folder `root`, the solver and the focal length.
struct Rig {
    app: Router,
    sim: Simulator,
    imaging: Arc<imaging::Imaging>,
    root: PathBuf,
    session: Arc<Session>,
}

fn rig(
    sim: &Simulator,
    root: &Path,
    solver: Arc<dyn ImagingSolver>,
    focal_length_mm: Option<f64>,
) -> Rig {
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
                "site": {"lat_deg": 40, "lon_deg": -75}
            }))
            .unwrap(),
            support::START,
        )
        .unwrap();
    let mut state = AppState::shared(support::DB.clone(), Samples::default())
        .with_session(session)
        .with_clock(Arc::new(support::ConsoleClock(sim.clock().clone())))
        .with_link(duplex(&node))
        .with_live()
        .with_imaging(root);
    let session = state.session();
    let imaging = state.imaging().expect("the rig takes imaging frames");
    state = state.with_imaging_solver(solver);
    if let Some(mm) = focal_length_mm {
        state = state.with_focal_length_mm(mm);
    }
    Rig {
        app: finder::router(state),
        sim: sim.clone(),
        imaging,
        root: root.to_path_buf(),
        session,
    }
}

/// The solving rig over a fresh simulator: the simulated solver over the
/// simulator's truth, and an optional focal length.
fn fresh_rig(root: &Path, focal_length_mm: Option<f64>) -> Rig {
    let sim = sim();
    let solver = Arc::new(imaging::simulated::SimulatedSolver::new(sim.clone()));
    rig(&sim, root, solver, focal_length_mm)
}

/// The solving rig with the simulated solver over the simulator's truth.
fn solving_rig(root: &Path) -> Rig {
    fresh_rig(root, None)
}

async fn get(app: &Router, path: &str) -> (StatusCode, Value) {
    support::request(app, "GET", path, json!(null)).await
}

async fn post(app: &Router, path: &str) -> (StatusCode, Value) {
    support::request(app, "POST", path, json!(null)).await
}

async fn put(app: &Router, path: &str, body: Value) -> (StatusCode, Value) {
    support::request(app, "PUT", path, body).await
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

async fn ready(app: &Router) -> Value {
    wait_until(app, |view| view["state"] == "ready").await
}

/// Wait until the session's `pointing` has the `source` `live`.
async fn pointing_live(app: &Router) {
    for _ in 0..600 {
        let (_, session) = get(app, "/api/session").await;
        if session["pointing"]["source"] == "live" {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("the session's pointing never went live");
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

/// One frame taken and solved: the view once the solve has ended.
async fn solve_frame(app: &Router) -> Value {
    let view = saved_frame(app).await;
    let id = view["frame"]["id"].as_u64().unwrap();
    let (status, answer) = post(app, "/api/imaging/solve").await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    wait_until(app, |view| {
        view["solve"]["frame_id"] == json!(id) && view["solve"]["state"] != "solving"
    })
    .await
}

/// The rig's imaging folder R.
fn root(rig: &Rig) -> &Path {
    &rig.root
}

/// The great-circle separation of two places, in degrees.
fn separation_deg(ra1: f64, dec1: f64, ra2: f64, dec2: f64) -> f64 {
    angular_distance(
        UnitVector::from_radec(ra1.to_radians(), dec1.to_radians()),
        UnitVector::from_radec(ra2.to_radians(), dec2.to_radians()),
    )
    .to_degrees()
}

/// The imaging truth at the middle of the view's latest frame's exposure.
fn mid_truth(rig: &Rig, view: &Value) -> simulator::imaging::ImagingTruth {
    let frame = &view["frame"];
    rig.sim.imaging_truth_at(
        frame["start"].as_f64().unwrap() + frame["exposure_ms"].as_f64().unwrap() / 2000.0,
    )
}

/// One imaging frame of a hint, as the test describes it: no file is read.
fn frame() -> SolveFrame {
    SolveFrame {
        path: PathBuf::new(),
        width: 1236,
        height: 822,
        bits: 16,
        bayer: None,
        binning: 1,
        roi: Region {
            x: 0,
            y: 0,
            width: 1236,
            height: 822,
        },
        start: 1_768_528_810.0,
        exposure_s: 1.0,
    }
}

/// A camera id of the test's own, for a `Relation` of the test's own.
fn camera() -> CameraId {
    CameraId {
        model: "the test's camera".to_string(),
        serial: None,
        width: 1236,
        height: 822,
        pixel_um: 19.12,
    }
}

/// US3-S1: a console's first solve is hinted by the live pointing, with a
/// radius of 3° and no field height, about 1.05° from the solved centre
/// (oracle: the simulator's directions at the middle of the exposure, and its
/// truth for the separation).
#[tokio::test(start_paused = true)]
async fn the_first_solve_is_hinted_by_the_live_pointing() {
    let scratch = crate::support::Scratch::new("imaging-hints-s1");
    let folder = scratch.0.join("imaging");
    let rig = solving_rig(&folder);
    ready(&rig.app).await;
    pointing_live(&rig.app).await;

    let view = saved_frame(&rig.app).await;
    let frame = &view["frame"];
    let id = frame["id"].as_u64().unwrap();
    let middle = frame["start"].as_f64().unwrap() + frame["exposure_ms"].as_f64().unwrap() / 2000.0;
    let camera = rig.sim.directions_at(middle).camera;

    let (status, answer) = post(&rig.app, "/api/imaging/solve").await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let view = wait_until(&rig.app, |view| {
        view["solve"]["frame_id"] == json!(id) && view["solve"]["state"] != "solving"
    })
    .await;
    let solve = &view["solve"];
    assert_eq!(solve["state"], json!("solved"), "{solve}");
    let hint = &solve["hint"];
    assert_eq!(hint["source"], json!("pointing"), "{solve}");
    assert_eq!(hint["radius_deg"], json!(3.0), "{solve}");
    assert!(hint.get("fov_deg").is_none(), "{solve}");
    let (ra_deg, dec_deg) = (
        hint["ra_deg"].as_f64().unwrap(),
        hint["dec_deg"].as_f64().unwrap(),
    );
    let from_camera = separation_deg(ra_deg, dec_deg, camera.ra, camera.dec);
    assert!(
        from_camera < 0.05,
        "the hint is {from_camera:.4}° from the finder camera's centre, not within 0.05°"
    );
    // The finder camera looks 127 finder pixels, 1.05°, from the main
    // telescope, so the hint is that far from the solved centre.
    let from_solved = separation_deg(
        ra_deg,
        dec_deg,
        solve["ra_deg"].as_f64().unwrap(),
        solve["dec_deg"].as_f64().unwrap(),
    );
    assert!(
        (0.95..1.15).contains(&from_solved),
        "the hint is {from_solved:.4}° from the solved centre"
    );
}

/// US3-S2: a solved frame's scale is kept per sensor pixel in the view, in
/// `relation.json` and in `Imaging::scale_arcsec_per_px`, and sizes the next
/// solve's field at binning 1 and 2 (oracle: the truth's scale at the middle
/// of the first exposure).
#[tokio::test(start_paused = true)]
async fn the_solved_scale_is_kept_and_sizes_the_next_solve() {
    let scratch = crate::support::Scratch::new("imaging-hints-s2");
    let folder = scratch.0.join("imaging");
    let rig = solving_rig(&folder);
    ready(&rig.app).await;
    pointing_live(&rig.app).await;
    assert_eq!(rig.imaging.scale_arcsec_per_px(), None);

    // The first solve, whose scale is kept.
    let view = solve_frame(&rig.app).await;
    let id = view["frame"]["id"].as_u64().unwrap();
    let truth = mid_truth(&rig, &view);
    let scale = view["relation"]["scale"]["arcsec_per_px"].as_f64().unwrap();
    assert_eq!(view["relation"]["scale"]["frame_id"], json!(id), "{view}");
    assert!(
        (scale - truth.arcsec_per_px).abs() < 0.001,
        "the kept scale {scale} is not the truth's {truth:?}"
    );
    assert!(
        (rig.imaging.scale_arcsec_per_px().unwrap() - scale).abs() < 1e-12,
        "scale_arcsec_per_px is not the view's kept scale"
    );

    // The second solve's field height is the frame's 822 sensor rows times the
    // kept scale.
    let view = solve_frame(&rig.app).await;
    let height = 822.0 * scale / 3600.0;
    assert!(
        (view["solve"]["hint"]["fov_deg"].as_f64().unwrap() - height).abs() < 1e-4,
        "the second solve's field height is out: {view}"
    );

    // At binning 2 the same figure, and the solve still succeeds.
    let (status, answer) = put(&rig.app, "/api/imaging/settings", json!({"binning":2})).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let view = solve_frame(&rig.app).await;
    let solve = &view["solve"];
    assert!(
        (solve["hint"]["fov_deg"].as_f64().unwrap() - height).abs() < 1e-3,
        "the third solve's field height is out: {view}"
    );
    assert_eq!(solve["state"], json!("solved"), "{solve}");
    let third = view["frame"]["id"].as_u64().unwrap();

    // `relation.json` names the camera, and the third solve replaced the
    // first's scale with its own: the same figure per sensor pixel.
    let file: Value =
        serde_json::from_slice(&std::fs::read(root(&rig).join("relation.json")).unwrap()).unwrap();
    assert_eq!(file["format"], json!("imaging-relation/1"), "{file}");
    let (_, view_now) = get(&rig.app, "/api/imaging").await;
    assert_eq!(
        file["camera"]["model"],
        view_now["camera"]["model"].clone(),
        "{file}"
    );
    assert_eq!(file["scale"]["frame_id"], json!(third), "{file}");
    assert!(
        (file["scale"]["arcsec_per_px"].as_f64().unwrap() - scale).abs() < 1e-9,
        "the third solve's kept scale is not the first's per sensor pixel: {file}"
    );
}

/// US3-S3: a focal length of 1800 mm gives a scale of 2.190991″ a pixel and a
/// field height of 0.500276° and the solve succeeds; 900 mm gives
/// 1.000552° and the solve fails (oracle: the worked arithmetic
/// `206264.806 × pixel size in mm / focal length`, and the simulated solver's
/// 5% rule: 1800 mm is 0.3% from the truth, 900 mm is 100%).
#[tokio::test(start_paused = true)]
async fn a_focal_length_sizes_the_first_solve() {
    let scratch = crate::support::Scratch::new("imaging-hints-s3");
    let folder = scratch.0.join("imaging");

    // 1800 mm: 2.190991″ a pixel, a field 0.500276° high, solved. Each rig
    // has its own imaging folder, so one's kept scale cannot size the other's
    // first solve.
    let long = fresh_rig(&folder.join("f1800"), Some(1800.0));
    ready(&long.app).await;
    let scale = long.imaging.scale_arcsec_per_px().unwrap();
    assert!(
        (scale - 2.190991).abs() < 1e-5,
        "the focal length's scale is {scale}, not 2.190991"
    );
    pointing_live(&long.app).await;
    let view = solve_frame(&long.app).await;
    let solve = &view["solve"];
    assert!(
        (solve["hint"]["fov_deg"].as_f64().unwrap() - 0.500276).abs() < 1e-5,
        "the first solve's field height is out: {solve}"
    );
    assert_eq!(solve["state"], json!("solved"), "{solve}");

    // 900 mm: twice the field height, 100% out, not solved.
    let short = fresh_rig(&folder.join("f900"), Some(900.0));
    ready(&short.app).await;
    pointing_live(&short.app).await;
    let view = solve_frame(&short.app).await;
    let solve = &view["solve"];
    assert!(
        (solve["hint"]["fov_deg"].as_f64().unwrap() - 1.000552).abs() < 1e-5,
        "the first solve's field height is out: {solve}"
    );
    assert_eq!(solve["state"], json!("failed"), "{solve}");
    assert_eq!(solve["reason"], json!("not_solved"), "{solve}");
    let relation = &view["relation"];
    assert!(relation.get("scale").is_none(), "{relation}");
    assert_eq!(relation["focal_length_mm"], json!(900.0), "{relation}");
}

/// US3-S4: without a fresh live pointing the hint falls to the last imaging
/// solve's centre, 3° wide (oracle: the stated order of sources; the place is
/// the first solve's centre read from the view).
#[tokio::test(start_paused = true)]
async fn the_hint_falls_to_the_last_solve_without_a_fresh_pointing() {
    let scratch = crate::support::Scratch::new("imaging-hints-s4a");
    let folder = scratch.0.join("imaging");
    let rig = solving_rig(&folder);
    ready(&rig.app).await;
    pointing_live(&rig.app).await;
    let first = solve_frame(&rig.app).await;
    let centre = (
        first["solve"]["ra_deg"].as_f64().unwrap(),
        first["solve"]["dec_deg"].as_f64().unwrap(),
    );

    rig.sim.stop_frames();
    tokio::time::sleep(Duration::from_secs(120)).await;
    let view = solve_frame(&rig.app).await;
    let solve = &view["solve"];
    let hint = &solve["hint"];
    assert_eq!(hint["source"], json!("last_solve"), "{solve}");
    assert_eq!(hint["radius_deg"], json!(3.0), "{solve}");
    let (ra_deg, dec_deg) = (
        hint["ra_deg"].as_f64().unwrap(),
        hint["dec_deg"].as_f64().unwrap(),
    );
    assert!(
        (ra_deg - centre.0).abs() < 1e-9 && (dec_deg - centre.1).abs() < 1e-9,
        "the hint is not the first solve's centre {centre:?}: {solve}"
    );
}

/// US3-S4: on a console with no live pointing and no imaging solve, the hint
/// is the session's target, 3° wide, and a target nearly 30° from where the
/// rig points does not solve (oracle: the stated order and the solver's
/// radius rule).
#[tokio::test(start_paused = true)]
async fn the_hint_falls_to_the_target_without_a_fresh_pointing() {
    let scratch = crate::support::Scratch::new("imaging-hints-s4b");
    let folder = scratch.0.join("imaging");
    let rig = solving_rig(&folder);
    ready(&rig.app).await;
    rig.sim.stop_frames();
    tokio::time::sleep(Duration::from_secs(120)).await;
    let (status, answer) = put(&rig.app, "/api/session", json!({"target":"M42"})).await;
    assert_eq!(status, StatusCode::OK, "{answer}");

    let view = solve_frame(&rig.app).await;
    let solve = &view["solve"];
    let hint = &solve["hint"];
    assert_eq!(hint["source"], json!("target"), "{solve}");
    assert_eq!(hint["radius_deg"], json!(3.0), "{solve}");
    let (_, session) = get(&rig.app, "/api/session").await;
    let (ra_deg, dec_deg) = (
        hint["ra_deg"].as_f64().unwrap(),
        hint["dec_deg"].as_f64().unwrap(),
    );
    assert!(
        (ra_deg - session["target"]["ra_deg"].as_f64().unwrap()).abs() < 1e-9
            && (dec_deg - session["target"]["dec_deg"].as_f64().unwrap()).abs() < 1e-9,
        "the hint is not the session's target: {solve}"
    );
    assert_eq!(solve["state"], json!("failed"), "{solve}");
    assert_eq!(solve["reason"], json!("not_solved"), "{solve}");
}

/// US3-S4: with no live pointing, no imaging solve and no target, the hint is
/// a search of the whole sky, 180° wide with no place, and it solves (oracle:
/// the stated order).
#[tokio::test(start_paused = true)]
async fn the_hint_falls_to_no_place_without_any_source() {
    let scratch = crate::support::Scratch::new("imaging-hints-s4c");
    let folder = scratch.0.join("imaging");
    let rig = solving_rig(&folder);
    ready(&rig.app).await;
    rig.sim.stop_frames();
    tokio::time::sleep(Duration::from_secs(120)).await;

    let view = solve_frame(&rig.app).await;
    let solve = &view["solve"];
    let hint = &solve["hint"];
    assert_eq!(hint["source"], json!("none"), "{solve}");
    assert!(
        hint.get("ra_deg").is_none() && hint.get("dec_deg").is_none(),
        "{solve}"
    );
    assert_eq!(hint["radius_deg"], json!(180.0), "{solve}");
    assert_eq!(solve["state"], json!("solved"), "{solve}");
}

/// US3-S4: the last imaging solve stops being a source when the session's
/// target changes (oracle: the stated rule).
#[tokio::test(start_paused = true)]
async fn a_changed_target_outranks_the_last_solve() {
    let scratch = crate::support::Scratch::new("imaging-hints-s4d");
    let folder = scratch.0.join("imaging");
    let rig = solving_rig(&folder);
    ready(&rig.app).await;
    pointing_live(&rig.app).await;
    let first = solve_frame(&rig.app).await;
    assert_eq!(first["solve"]["state"], json!("solved"), "{first}");

    rig.sim.stop_frames();
    tokio::time::sleep(Duration::from_secs(120)).await;
    let (status, answer) = put(&rig.app, "/api/session", json!({"target":"M42"})).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let view = solve_frame(&rig.app).await;
    assert_eq!(view["solve"]["hint"]["source"], json!("target"), "{view}");
}

/// US3-S4: an unchanged target keeps the last imaging solve a source (oracle:
/// the stated rule).
#[tokio::test(start_paused = true)]
async fn an_unchanged_target_keeps_the_last_solve() {
    let scratch = crate::support::Scratch::new("imaging-hints-s4e");
    let folder = scratch.0.join("imaging");
    let rig = solving_rig(&folder);
    ready(&rig.app).await;
    pointing_live(&rig.app).await;
    let (status, answer) = put(&rig.app, "/api/session", json!({"target":"M42"})).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let first = solve_frame(&rig.app).await;
    assert_eq!(
        first["solve"]["hint"]["source"],
        json!("pointing"),
        "{first}"
    );
    assert_eq!(first["solve"]["state"], json!("solved"), "{first}");

    rig.sim.stop_frames();
    tokio::time::sleep(Duration::from_secs(120)).await;
    let view = solve_frame(&rig.app).await;
    let solve = &view["solve"];
    assert_eq!(solve["hint"]["source"], json!("last_solve"), "{solve}");
}

/// US3-S5: a second console over the same imaging folder reads the first's
/// kept scale, and ignores a file made for another camera (oracle: the first
/// rig's own value, read by the test from its view and from the file).
#[tokio::test(start_paused = true)]
async fn a_second_console_reads_the_kept_scale_for_its_camera() {
    let scratch = crate::support::Scratch::new("imaging-hints-s5");
    let folder = scratch.0.join("imaging");

    // One solved frame keeps a scale.
    let first_rig = solving_rig(&folder);
    ready(&first_rig.app).await;
    pointing_live(&first_rig.app).await;
    let first = solve_frame(&first_rig.app).await;
    assert_eq!(first["solve"]["state"], json!("solved"), "{first}");
    let scale = first["relation"]["scale"]["arcsec_per_px"]
        .as_f64()
        .unwrap();
    let file: Value =
        serde_json::from_slice(&std::fs::read(root(&first_rig).join("relation.json")).unwrap())
            .unwrap();
    assert!(
        (file["scale"]["arcsec_per_px"].as_f64().unwrap() - scale).abs() < 1e-12,
        "the file's scale is not the view's: {file}"
    );

    // A second console over the same R reads it, and its first solve's field
    // height is 822 sensor rows times it.
    let second = fresh_rig(&folder, None);
    let view = ready(&second.app).await;
    assert!(
        (view["relation"]["scale"]["arcsec_per_px"].as_f64().unwrap() - scale).abs() < 1e-12,
        "the second console does not read the first's scale: {view}"
    );
    pointing_live(&second.app).await;
    let view = solve_frame(&second.app).await;
    assert!(
        (view["solve"]["hint"]["fov_deg"].as_f64().unwrap() - 822.0 * scale / 3600.0).abs() < 1e-4,
        "the second console's first solve is not sized by the kept scale: {view}"
    );

    // A file made for another camera is ignored: no scale, no field height.
    let path = root(&second).join("relation.json");
    let mut other: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    other["camera"]["serial"] = json!("OTHER");
    std::fs::write(&path, serde_json::to_vec(&other).unwrap()).unwrap();
    let third = fresh_rig(&folder, None);
    let view = ready(&third.app).await;
    assert!(
        view["relation"].get("scale").is_none(),
        "a file made for another camera is not ignored: {view}"
    );
    pointing_live(&third.app).await;
    let view = solve_frame(&third.app).await;
    assert!(
        view["solve"]["hint"].get("fov_deg").is_none(),
        "the third console's first solve has a field height: {view}"
    );
}

/// US3-S7: after the simulated optics are widened, three solves fail on the
/// kept scale's field height, the next is given none and solves, its scale
/// replaces the kept one, and the one after is sized by the new scale
/// (oracle: the truth's scale before and after the change, and the stated rule
/// of three).
#[tokio::test(start_paused = true)]
async fn the_field_height_turns_through_its_sources() {
    let scratch = crate::support::Scratch::new("imaging-hints-s7a");
    let folder = scratch.0.join("imaging");
    let rig = solving_rig(&folder);
    ready(&rig.app).await;
    pointing_live(&rig.app).await;
    let first = solve_frame(&rig.app).await;
    assert_eq!(first["solve"]["state"], json!("solved"), "{first}");
    let scale = first["relation"]["scale"]["arcsec_per_px"]
        .as_f64()
        .unwrap();

    // The imaging camera's field is doubled.
    rig.sim.set_imaging_optics(ImagingOptics {
        fov_deg: 1.5,
        ..rig.sim.imaging_optics()
    });

    // The second, third and fourth solves fail on the kept scale's field
    // height, and the kept scale is not replaced.
    for n in 2..=4 {
        let view = solve_frame(&rig.app).await;
        let solve = &view["solve"];
        assert!(
            (solve["hint"]["fov_deg"].as_f64().unwrap() - 822.0 * scale / 3600.0).abs() < 1e-4,
            "solve {n} is not sized by the kept scale: {solve}"
        );
        assert_eq!(solve["state"], json!("failed"), "solve {n}: {solve}");
        assert_eq!(solve["reason"], json!("not_solved"), "solve {n}: {solve}");
        assert_eq!(
            view["relation"]["scale"]["frame_id"],
            json!(1),
            "solve {n}: {view}"
        );
    }

    // The fifth is given no field height and solves; its scale, twice the old,
    // replaces the kept one.
    let fifth = solve_frame(&rig.app).await;
    assert!(
        fifth["solve"]["hint"].get("fov_deg").is_none(),
        "the fifth solve is sized: {fifth}"
    );
    assert_eq!(fifth["solve"]["state"], json!("solved"), "{fifth}");
    let truth = mid_truth(&rig, &fifth);
    let new_scale = fifth["relation"]["scale"]["arcsec_per_px"]
        .as_f64()
        .unwrap();
    assert_eq!(fifth["relation"]["scale"]["frame_id"], json!(5), "{fifth}");
    assert!(
        (new_scale - truth.arcsec_per_px).abs() < 0.001,
        "the new kept scale {new_scale} is not the new truth's {truth:?}"
    );
    assert!(
        (new_scale - 2.0 * scale).abs() < 0.01,
        "the new kept scale {new_scale} is not about twice the old {scale}"
    );

    // The sixth is sized by the new scale and solves.
    let sixth = solve_frame(&rig.app).await;
    assert_eq!(sixth["solve"]["state"], json!("solved"), "{sixth}");
    assert!(
        (sixth["solve"]["hint"]["fov_deg"].as_f64().unwrap() - 822.0 * new_scale / 3600.0).abs()
            < 1e-4,
        "the sixth solve is not sized by the new scale: {sixth}"
    );
}

/// US3-S7: with a focal length set the same change takes three solves more —
/// the kept scale's, the focal length's, then none — and the eighth solves
/// (oracle: the stated rule of three and the focal length's 0.500276°).
#[tokio::test(start_paused = true)]
async fn the_field_height_turns_through_the_focal_length_too() {
    let scratch = crate::support::Scratch::new("imaging-hints-s7b");
    let folder = scratch.0.join("imaging");
    let sim = sim();
    let rig = rig(
        &sim,
        &folder,
        Arc::new(imaging::simulated::SimulatedSolver::new(sim.clone())),
        Some(1800.0),
    );
    ready(&rig.app).await;
    pointing_live(&rig.app).await;
    let first = solve_frame(&rig.app).await;
    assert_eq!(first["solve"]["state"], json!("solved"), "{first}");
    let scale = first["relation"]["scale"]["arcsec_per_px"]
        .as_f64()
        .unwrap();

    rig.sim.set_imaging_optics(ImagingOptics {
        fov_deg: 1.5,
        ..rig.sim.imaging_optics()
    });

    // The kept scale's field height, three times.
    for n in 2..=4 {
        let view = solve_frame(&rig.app).await;
        let solve = &view["solve"];
        assert!(
            (solve["hint"]["fov_deg"].as_f64().unwrap() - 822.0 * scale / 3600.0).abs() < 1e-4,
            "solve {n} is not sized by the kept scale: {solve}"
        );
        assert_eq!(solve["state"], json!("failed"), "solve {n}: {solve}");
        assert_eq!(solve["reason"], json!("not_solved"), "solve {n}: {solve}");
    }
    // The focal length's, three times.
    for n in 5..=7 {
        let view = solve_frame(&rig.app).await;
        let solve = &view["solve"];
        assert!(
            (solve["hint"]["fov_deg"].as_f64().unwrap() - 0.500276).abs() < 1e-5,
            "solve {n} is not sized by the focal length: {solve}"
        );
        assert_eq!(solve["state"], json!("failed"), "solve {n}: {solve}");
        assert_eq!(solve["reason"], json!("not_solved"), "solve {n}: {solve}");
    }
    // The eighth is given no field height and solves.
    let eighth = solve_frame(&rig.app).await;
    assert!(
        eighth["solve"]["hint"].get("fov_deg").is_none(),
        "the eighth solve is sized: {eighth}"
    );
    assert_eq!(eighth["solve"]["state"], json!("solved"), "{eighth}");
}

/// US3-S7: a `Relation` of the test's own turns through the field height's
/// sources every three misses, and starts again after a solved solve (oracle:
/// the stated rule of three).
#[test]
fn a_relation_turns_through_its_sources_every_three_misses() {
    let scratch = crate::support::Scratch::new("imaging-hints-s7c");
    let mut relation = Relation::new(scratch.0.join("relation.json"));
    relation.describe(camera());
    relation.set_focal_length_mm(1800.0);
    relation.store_scale(2.0, 1, 0.0);
    assert_eq!(relation.height_inputs(), (Some(2.0), Some(1800.0)));
    for _ in 0..3 {
        relation.miss();
    }
    assert_eq!(relation.height_inputs(), (None, Some(1800.0)));
    for _ in 0..3 {
        relation.miss();
    }
    assert_eq!(relation.height_inputs(), (None, None));
    for _ in 0..3 {
        relation.miss();
    }
    assert_eq!(relation.height_inputs(), (Some(2.0), Some(1800.0)));
    relation.miss();
    relation.store_scale(2.1, 2, 1.0);
    assert_eq!(relation.height_inputs(), (Some(2.1), Some(1800.0)));
    relation.miss();
    relation.miss();
    assert_eq!(
        relation.height_inputs(),
        (Some(2.1), Some(1800.0)),
        "the count has not started again from the solved solve"
    );
}

/// US3-S8: a live pointing taken at a tapped aim point 4.98° from where the
/// imaging camera looks fails three times, the target then fails three times,
/// and the seventh solve searches the whole sky and succeeds (oracle: the
/// stated order of sources and rule of three; the hint's distance from the
/// truth is the test's own separation over the simulator's truth).
#[tokio::test(start_paused = true)]
async fn the_hint_takes_the_next_source_after_three_misses() {
    let scratch = crate::support::Scratch::new("imaging-hints-s8a");
    let folder = scratch.0.join("imaging");
    let rig = solving_rig(&folder);
    ready(&rig.app).await;
    let (status, answer) = put(&rig.app, "/api/session", json!({"target":"M42"})).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    // A tapped aim point about 4.98° from the true one along the finder
    // camera's rows, so a live pointing taken there lies that far from where
    // the imaging camera looks.
    rig.session
        .set_aim(Some(Aim { x: 96.0, y: 420.0 }), rig.sim.clock().now())
        .unwrap();
    pointing_live(&rig.app).await;

    for n in 1..=7 {
        let view = solve_frame(&rig.app).await;
        let solve = &view["solve"];
        let hint = &solve["hint"];
        let truth = mid_truth(&rig, &view);
        match n {
            1..=3 => {
                assert_eq!(hint["source"], json!("pointing"), "solve {n}: {solve}");
                assert_eq!(hint["radius_deg"], json!(3.0), "solve {n}: {solve}");
                let separation = separation_deg(
                    hint["ra_deg"].as_f64().unwrap(),
                    hint["dec_deg"].as_f64().unwrap(),
                    truth.ra_deg,
                    truth.dec_deg,
                );
                assert!(
                    separation > 4.5,
                    "solve {n}: the hint is {separation:.3}° from the truth"
                );
            }
            4..=6 => {
                assert_eq!(hint["source"], json!("target"), "solve {n}: {solve}");
                assert_eq!(hint["radius_deg"], json!(3.0), "solve {n}: {solve}");
                let (_, session) = get(&rig.app, "/api/session").await;
                assert!(
                    (hint["ra_deg"].as_f64().unwrap()
                        - session["target"]["ra_deg"].as_f64().unwrap())
                    .abs()
                        < 1e-9
                        && (hint["dec_deg"].as_f64().unwrap()
                            - session["target"]["dec_deg"].as_f64().unwrap())
                        .abs()
                            < 1e-9,
                    "solve {n}: the hint is not the session's target: {solve}"
                );
            }
            _ => {
                assert_eq!(hint["source"], json!("none"), "solve {n}: {solve}");
                assert!(
                    hint.get("ra_deg").is_none() && hint.get("dec_deg").is_none(),
                    "solve {n}: {solve}"
                );
                assert_eq!(hint["radius_deg"], json!(180.0), "solve {n}: {solve}");
            }
        }
        let solved = n == 7;
        if solved {
            assert_eq!(solve["state"], json!("solved"), "solve {n}: {solve}");
        } else {
            assert_eq!(solve["state"], json!("failed"), "solve {n}: {solve}");
            assert_eq!(solve["reason"], json!("not_solved"), "solve {n}: {solve}");
        }
    }
}

/// US3-S8: on a rig built the same way but with no target, the fourth solve
/// searches the whole sky and succeeds (oracle: the stated order, one source
/// fewer).
#[tokio::test(start_paused = true)]
async fn the_next_source_without_a_target_is_the_whole_sky() {
    let scratch = crate::support::Scratch::new("imaging-hints-s8b");
    let folder = scratch.0.join("imaging");
    let rig = solving_rig(&folder);
    ready(&rig.app).await;
    rig.session
        .set_aim(Some(Aim { x: 96.0, y: 420.0 }), rig.sim.clock().now())
        .unwrap();
    pointing_live(&rig.app).await;

    for n in 1..=4 {
        let view = solve_frame(&rig.app).await;
        let solve = &view["solve"];
        if n < 4 {
            assert_eq!(
                solve["hint"]["source"],
                json!("pointing"),
                "solve {n}: {solve}"
            );
            assert_eq!(solve["state"], json!("failed"), "solve {n}: {solve}");
            assert_eq!(solve["reason"], json!("not_solved"), "solve {n}: {solve}");
        } else {
            assert_eq!(solve["hint"]["source"], json!("none"), "solve {n}: {solve}");
            assert_eq!(
                solve["hint"]["radius_deg"],
                json!(180.0),
                "solve {n}: {solve}"
            );
            assert_eq!(solve["state"], json!("solved"), "solve {n}: {solve}");
        }
    }
}

/// US3-S8: `Relation::hint_skip` counts a place down the hint's sources every
/// three misses, and a solved solve starts again from the first (oracle: the
/// stated rule of three).
#[test]
fn a_relations_hint_skip_counts_one_place_per_three_misses() {
    let scratch = crate::support::Scratch::new("imaging-hints-s8c");
    let mut relation = Relation::new(scratch.0.join("relation.json"));
    relation.describe(camera());
    assert_eq!(relation.hint_skip(), 0);
    relation.miss();
    relation.miss();
    assert_eq!(relation.hint_skip(), 0, "two misses are not one place");
    relation.miss();
    assert_eq!(relation.hint_skip(), 1);
    for _ in 0..3 {
        relation.miss();
    }
    assert_eq!(relation.hint_skip(), 2);
    relation.store_scale(2.0, 1, 0.0);
    assert_eq!(relation.hint_skip(), 0);
}

/// US3-S8: `hint_for` takes the source `skip` places down among those that
/// exist, held at `none`, and a whole-frame hint carries no field height
/// without a scale or a focal length (oracle: the stated order and the
/// skip figures of the scenario).
#[test]
fn hint_for_takes_the_source_skip_places_down() {
    let frame = frame();
    let inputs =
        |skip: usize, last_solve: Option<(f64, f64)>, target: Option<(f64, f64)>| HintInputs {
            frame: &frame,
            sensor: (1236, 822),
            pointing: Some((80.0, -5.0, 1_768_528_810.5)),
            last_solve,
            target,
            scale: None,
            pixel_um: 19.12,
            focal_length_mm: None,
            aim_measured: false,
            skip,
        };
    let all = |skip: usize| inputs(skip, Some((81.0, -5.0)), Some((82.0, -5.0)));
    for (skip, source, place) in [
        (0, HintSource::Pointing, Some((80.0, -5.0))),
        (1, HintSource::LastSolve, Some((81.0, -5.0))),
        (2, HintSource::Target, Some((82.0, -5.0))),
        (3, HintSource::None, None),
        (4, HintSource::None, None),
        (7, HintSource::None, None),
    ] {
        let hint = hint_for(&all(skip));
        assert_eq!(hint.source, source, "skip {skip}");
        assert_eq!(hint.position, place, "skip {skip}");
        assert_eq!(
            hint.radius_deg,
            if source == HintSource::None {
                180.0
            } else {
                3.0
            },
            "skip {skip}"
        );
        assert_eq!(hint.fov_deg, None, "skip {skip}");
    }
    // A source picked at `skip` modulo the count would give `pointing` with
    // skip 4; the hint is held at `none` instead.
    let hint = hint_for(&inputs(1, None, None));
    assert_eq!(hint.source, HintSource::None);
    assert_eq!(hint.position, None);
    assert_eq!(hint.radius_deg, 180.0);
}

/// The solver of US3-S8's last bullet: `TimedOut` to its first three solves,
/// `TooFewStars` to the next three, the simulated solver's answer after that.
struct TimedThenFew {
    simulated: imaging::simulated::SimulatedSolver,
    calls: AtomicUsize,
}

impl ImagingSolver for TimedThenFew {
    fn name(&self) -> &'static str {
        "timed then few"
    }

    fn solve(&self, frame: SolveFrame, hint: SolveHint) -> SolveFuture<'_> {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        if n < 3 {
            Box::pin(async { Err(SolveError::TimedOut { seconds: 120 }) })
        } else if n < 6 {
            Box::pin(async { Err(SolveError::TooFewStars { detail: None }) })
        } else {
            self.simulated.solve(frame, hint)
        }
    }

    fn measure(&self, _frame: SolveFrame, _region: Option<Region>) -> MeasureFuture<'_> {
        Box::pin(async {
            Err(SolveError::Failed(
                "the test solver measures nothing".to_string(),
            ))
        })
    }
}

/// US3-S8: `timed_out` is a miss and `too_few_stars` is not, so the hint
/// stays on the target source through the too-few-stars answers (oracle: the
/// stated rule; the sources read from the solves' views).
#[tokio::test(start_paused = true)]
async fn a_timed_out_solve_is_a_miss_and_a_too_few_stars_solve_is_not() {
    let scratch = crate::support::Scratch::new("imaging-hints-s8e");
    let folder = scratch.0.join("imaging");
    let sim = sim();
    let solver = Arc::new(TimedThenFew {
        simulated: imaging::simulated::SimulatedSolver::new(sim.clone()),
        calls: AtomicUsize::new(0),
    });
    let rig = rig(&sim, &folder, solver, None);
    ready(&rig.app).await;
    let (status, answer) = put(&rig.app, "/api/session", json!({"target":"M42"})).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    pointing_live(&rig.app).await;

    for n in 1..=7 {
        let view = solve_frame(&rig.app).await;
        let hint = &view["solve"]["hint"];
        if n <= 3 {
            assert_eq!(hint["source"], json!("pointing"), "solve {n}: {view}");
        } else {
            assert_eq!(hint["source"], json!("target"), "solve {n}: {view}");
        }
    }
}
