//! The aim point and the imaging angle, measured from one moment (032 US4):
//! the history of the finder camera's live solves, the line fitted through
//! them, the aim reading an imaging solve takes from it, the session's aim
//! point measured, the tap that replaces it and the solve offered from outside
//! the route. Oracles: the simulator's true aim point and true imaging angle,
//! the worked examples of spec.md US4-S2 with their stated numbers, the
//! rotation's definition in `sky.rs`, the stated keys and statuses, and the
//! centroid the test itself read from a preview; nothing here is read from the
//! code under test.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use axum::{http::StatusCode, Router};
use finder::{
    imaging::{
        self,
        relation::{read_aim, FinderFix, FinderHistory, Outcome},
        solver::{ImagingSolver, Region, SolveFrame, SolveHint},
    },
    samples::Samples,
    session::{Aim, Session},
    sky::Frame,
    AppState,
};
use math_core::{angular_distance, UnitVector};
use scope_link::memory;
use serde_json::{json, Value};
use simulator::{mount::SimMount, SimNode, Simulator, Site, VirtualClock};

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

/// The solving rig (the shared setup): its node, the paired session, the
/// imaging folder `root`, and the state it answers through.
struct Rig {
    app: Router,
    sim: Simulator,
    imaging: Arc<imaging::Imaging>,
    root: PathBuf,
    live: Arc<finder::live::Live>,
}

fn rig(
    sim: &Simulator,
    root: &Path,
    solver: Arc<dyn ImagingSolver>,
    focal_length_mm: Option<f64>,
    session: Session,
) -> Rig {
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
    let mut state = AppState::shared(support::DB.clone(), Samples::default())
        .with_session(session)
        .with_clock(Arc::new(support::ConsoleClock(sim.clock().clone())))
        .with_link(duplex(&node))
        .with_live()
        .with_imaging(root);
    let imaging = state.imaging().expect("the rig takes imaging frames");
    let live = state.live().expect("the rig solves live");
    state = state.with_imaging_solver(solver);
    if let Some(mm) = focal_length_mm {
        state = state.with_focal_length_mm(mm);
    }
    Rig {
        app: finder::router(state),
        sim: sim.clone(),
        imaging,
        root: root.to_path_buf(),
        live,
    }
}

/// The solving rig over a fresh simulator, with the session of the caller's.
fn rig_with_session(root: &Path, session: Session) -> Rig {
    let sim = sim();
    let solver = Arc::new(imaging::simulated::SimulatedSolver::new(sim.clone()));
    rig(&sim, root, solver, None, session)
}

/// The solving rig over a fresh simulator: the simulated solver over the
/// simulator's truth.
fn solving_rig(root: &Path) -> Rig {
    rig_with_session(root, Session::default())
}

/// The solving rig with a focal length instead of a kept scale.
fn fresh_rig(root: &Path, focal_length_mm: f64) -> Rig {
    let sim = sim();
    let solver = Arc::new(imaging::simulated::SimulatedSolver::new(sim.clone()));
    rig(
        &sim,
        root,
        solver,
        Some(focal_length_mm),
        Session::default(),
    )
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

/// Read `GET /api/session` every 100 ms until `check` holds, for at most 300 s.
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

/// The imaging truth at the middle of the view's latest frame's exposure.
fn mid_truth(rig: &Rig, view: &Value) -> simulator::imaging::ImagingTruth {
    let frame = &view["frame"];
    rig.sim.imaging_truth_at(
        frame["start"].as_f64().unwrap() + frame["exposure_ms"].as_f64().unwrap() / 2000.0,
    )
}

/// A finder frame of the test's own making (the shared setup): a frame from a
/// solution at the stated RA and Dec in degrees, the stated roll in radians,
/// and the finder camera's 1456 × 1088 over 12°, no distortion.
fn finder_frame(ra_deg: f64, dec_deg: f64, roll: f64) -> Frame {
    Frame::from_solution(&plate_solver::Solution {
        ra: Some(ra_deg.to_radians()),
        dec: Some(dec_deg.to_radians()),
        roll: Some(roll),
        camera: Some(math_core::PinholeCamera::new(
            1456.0,
            1088.0,
            12_f64.to_radians(),
        )),
        ..Default::default()
    })
    .unwrap()
}

/// A finder fix of the test's own making (the shared setup): its frame from a
/// solution at the stated RA and Dec, no roll, the stated time and id.
fn fix(ra_deg: f64, dec_deg: f64, time: f64, frame_id: u64) -> FinderFix {
    FinderFix {
        time,
        frame: finder_frame(ra_deg, dec_deg, 0.0),
        frame_id,
    }
}

/// The imaging solve C of US4-S2: up 47° east of north at 0.0006° a pixel, for
/// a frame 1236 × 822, centred on the sky direction of pixel (700, 420) in a
/// finder frame centred on RA 100°, Dec 20°.
fn solve_c() -> finder::imaging::solver::ImagingSolve {
    let frame = finder_frame(100.0, 20.0, 0.0);
    let direction = frame.unproject((700.0, 420.0)).unwrap();
    let (ra, dec) = direction.to_radec();
    let (s, theta) = (0.0006, 47_f64.to_radians());
    finder::imaging::solver::ImagingSolve::from_cd(
        ra.to_degrees(),
        dec.to_degrees(),
        [
            [-s * theta.cos(), -s * theta.sin()],
            [s * theta.sin(), -s * theta.cos()],
        ],
        1236,
        822,
    )
}

/// The nine fixes of the first bullet of US4-S2, at the times 8.0 to 12.0,
/// drifting at the stated RA rate about the middle 10.2.
fn drifting_fixes(rate_deg_per_s: f64) -> Vec<FinderFix> {
    (0..9)
        .map(|n| {
            let t = 8.0 + 0.5 * f64::from(n as u32);
            fix(100.0 + rate_deg_per_s * (t - 10.2), 20.0, t, n)
        })
        .collect()
}

/// The nine fixes of the second bullet of US4-S2, at the times 8.2 to 12.2,
/// the odd ones 20.0″ of RA either side of RA 100°.
fn scattering_fixes() -> Vec<FinderFix> {
    (0..9)
        .map(|n| {
            let t = 8.2 + 0.5 * f64::from(n as u32);
            let ra = if n % 2 == 0 {
                100.005_912_099
            } else {
                99.994_087_901
            };
            fix(ra, 20.0, t, n)
        })
        .collect()
}

/// The reading `read_aim` answered, as a `Measured` one.
fn measured(outcome: Outcome) -> finder::imaging::relation::AimReading {
    match outcome {
        Outcome::Measured(reading) => reading,
        other => panic!("read_aim answered {other:?}, not Measured"),
    }
}

/// US4-S1: one solve while the finder camera is solving too measures where the
/// imaging camera's centre falls in the finder camera's frame, sets the
/// session's aim point from it, marked `measured`, and shows the imaging
/// angle and the field's size on the sky (oracle: the simulator's true aim
/// point (700, 420), its true imaging angle 17°, and its truth's scale at the
/// middle of the exposure, read by the test itself).
#[tokio::test(start_paused = true)]
async fn one_solve_measures_the_aim_point_and_the_imaging_angle() {
    let scratch = crate::support::Scratch::new("imaging-relation-s1");
    let folder = scratch.0.join("imaging");
    let rig = solving_rig(&folder);
    ready(&rig.app).await;
    pointing_live(&rig.app).await;
    settle().await;

    let view = solve_frame(&rig.app).await;
    let id = view["frame"]["id"].as_u64().unwrap();
    let truth = mid_truth(&rig, &view);
    assert_eq!(view["solve"]["state"], json!("solved"), "{}", view["solve"]);

    let (_, session) = get(&rig.app, "/api/session").await;
    let aim = &session["aim"];
    assert_eq!(aim["source"], json!("measured"), "{aim}");
    assert!(
        (aim["x"].as_f64().unwrap() - 700.0).abs() < 1.5,
        "the aim point is {aim}"
    );
    assert!(
        (aim["y"].as_f64().unwrap() - 420.0).abs() < 1.5,
        "the aim point is {aim}"
    );

    let (_, imaging) = get(&rig.app, "/api/imaging").await;
    let relation = &imaging["relation"];
    assert!(
        (relation["aim"]["x"].as_f64().unwrap() - aim["x"].as_f64().unwrap()).abs() < 1e-9,
        "{relation}"
    );
    assert!(
        (relation["aim"]["y"].as_f64().unwrap() - aim["y"].as_f64().unwrap()).abs() < 1e-9,
        "{relation}"
    );
    assert!(
        (relation["angle_deg"].as_f64().unwrap() - rig.sim.imaging_optics().angle_deg).abs() < 0.25,
        "{relation}"
    );
    assert_eq!(relation["mirrored"], json!(false), "{relation}");
    assert_eq!(relation["readings"], json!(1), "{relation}");
    assert_eq!(relation["last"]["frame_id"], json!(id), "{relation}");
    assert_eq!(relation["last"]["outcome"], json!("measured"), "{relation}");
    assert!(
        (relation["field"]["width_arcsec"].as_f64().unwrap() - 1236.0 * truth.arcsec_per_px).abs()
            < 0.5,
        "{relation}"
    );
    assert!(
        (relation["field"]["height_arcsec"].as_f64().unwrap() - 822.0 * truth.arcsec_per_px).abs()
            < 0.5,
        "{relation}"
    );
}

/// US4-S2: a steady drift in right ascension is a straight line in the finder
/// frame, and the line's value at the middle of the exposure is where the
/// finder frame centred on RA 100° puts C's centre: (700, 420), with the
/// stated numbers; the fix nearest the middle, taken alone, would be 0.1
/// pixel out (oracle: the worked example's stated numbers, and the fix's own
/// projection read by the test).
#[test]
fn read_aim_fits_a_line_through_a_steady_drift() {
    let fixes = drifting_fixes(0.004434074);
    let solve = solve_c();
    let reading = measured(read_aim(&fixes, &solve, 9.7, 1.0, 3));
    assert!((reading.x - 700.0).abs() < 0.01, "x is {}", reading.x);
    assert!((reading.y - 420.0).abs() < 0.01, "y is {}", reading.y);
    assert!((reading.at - 10.2).abs() < 1e-9, "at is {}", reading.at);
    assert!(!reading.mirrored, "mirrored");
    assert!(
        (reading.finder_arcsec_per_px - 29.779).abs() < 0.001,
        "the finder scale is {}",
        reading.finder_arcsec_per_px
    );
    assert_eq!(reading.finder_width, 1456.0);
    assert_eq!(reading.finder_height, 1088.0);
    let centre = UnitVector::from_radec(solve.ra_deg.to_radians(), solve.dec_deg.to_radians());
    let up = fixes[4].frame.up_east_of_north_at(centre).unwrap();
    let expected = (47.0 - up).rem_euclid(360.0);
    assert!(
        (reading.angle_deg - expected).abs() < 0.01,
        "the angle {} is not 47° less {up}",
        reading.angle_deg
    );
    // The fix nearest the middle, taken alone, would be 0.1 pixel out.
    let (nearest, _) = fixes[4].frame.project(centre).unwrap();
    assert!(
        (nearest - 700.0).abs() > 0.05 && (nearest - 700.0).abs() < 0.15,
        "the nearest fix alone puts the centre at {nearest}"
    );
}

/// US4-S2: a still telescope and a scattering finder camera: nine fixes 20.0″
/// of RA either side, two neighbours differing by 40″, give Measured at the
/// fixes' mean, the line through them flat (oracle: the worked example's
/// stated numbers).
#[test]
fn read_aim_measures_a_still_telescope_with_a_scattering_camera() {
    let fixes = scattering_fixes();
    let reading = measured(read_aim(&fixes, &solve_c(), 9.7, 1.0, 3));
    assert!((reading.x - 700.0).abs() < 0.1, "x is {}", reading.x);
    assert!((reading.y - 420.0).abs() < 0.01, "y is {}", reading.y);
}

/// US4-S2: a drift of 63″ a second on the sky is within the allowance: the
/// line travels 63″ during the exposure, but the fixes scatter 22.5″ about it
/// over D = 15 s², so its travel is uncertain by 5.8″, and 63 less 5.8 is
/// under the 60″ allowed (oracle: the worked example's stated numbers).
#[test]
fn read_aim_allows_a_drift_within_the_travel_allowance() {
    let fixes: Vec<FinderFix> = (0..9)
        .map(|n| {
            let t = 8.2 + 0.5 * f64::from(n as u32);
            let side = if n % 2 == 0 {
                0.005_912_099
            } else {
                -0.005_912_099
            };
            fix(100.0 + side + 0.018623111 * (t - 10.2), 20.0, t, n)
        })
        .collect();
    let outcome = read_aim(&fixes, &solve_c(), 9.7, 1.0, 3);
    assert!(
        matches!(outcome, Outcome::Measured(_)),
        "read_aim answered {outcome:?}"
    );
}

/// US4-S2: a drift of 100″ a second on the sky is Moving: the line travels 99″
/// during the exposure (oracle: the worked example's stated numbers).
#[test]
fn read_aim_refuses_a_moving_telescope() {
    let fixes = drifting_fixes(0.029560494);
    let outcome = read_aim(&fixes, &solve_c(), 9.7, 1.0, 3);
    assert!(matches!(outcome, Outcome::Moving), "{outcome:?}");
}

/// US4-S2: a fix half a degree off the line is unsteady: the line is still
/// flat, and the fixes scatter 600″ about it, against the 90″ allowed (oracle:
/// the worked example's stated numbers).
#[test]
fn read_aim_refuses_an_unsteady_stretch() {
    let mut fixes = scattering_fixes();
    fixes[4] = fix(100.5, 20.0, 10.2, 4);
    let outcome = read_aim(&fixes, &solve_c(), 9.7, 1.0, 3);
    assert!(matches!(outcome, Outcome::Moving), "{outcome:?}");
}

/// US4-S2: two fixes, being fewer than three, take no reading, and an exposure
/// that holds none of the fixes takes none either (oracle: the stated rules).
#[test]
fn read_aim_takes_no_reading_without_a_live_pointing() {
    let fixes = drifting_fixes(0.004434074);
    let two = [fixes[4], fixes[5]];
    let outcome = read_aim(&two, &solve_c(), 9.7, 1.0, 3);
    assert!(matches!(outcome, Outcome::NoLivePointing), "{outcome:?}");
    let outcome = read_aim(&fixes, &solve_c(), 12.3, 0.6, 3);
    assert!(matches!(outcome, Outcome::NoLivePointing), "{outcome:?}");
}

/// US4-S2: an imaging solve centred 20° from the finder frames falls outside
/// them (oracle: the stated rule).
#[test]
fn read_aim_is_off_the_frame_when_the_centre_falls_outside() {
    let fixes = drifting_fixes(0.004434074);
    let (s, theta) = (0.0006, 47_f64.to_radians());
    let solve = finder::imaging::solver::ImagingSolve::from_cd(
        100.0,
        40.0,
        [
            [-s * theta.cos(), -s * theta.sin()],
            [s * theta.sin(), -s * theta.cos()],
        ],
        1236,
        822,
    );
    let outcome = read_aim(&fixes, &solve, 9.7, 1.0, 3);
    assert!(matches!(outcome, Outcome::OffFrame), "{outcome:?}");
}

/// US4-S2: the history keeps 120 s of fixes one a second, and thins 3001 fixes
/// half a second apart to one a second over its whole 1200 s span: a fix that
/// follows the last one kept by less than the span over 2000 is not kept, and
/// only fixes older than the span are dropped (oracle: the stated numbers).
#[test]
fn the_history_keeps_its_span_and_thins_to_a_density() {
    let frame = finder_frame(100.0, 20.0, 0.0);
    let mut history = FinderHistory::default();
    for n in 0..301 {
        history.push(
            FinderFix {
                time: f64::from(n as u32),
                frame,
                frame_id: n,
            },
            120.0,
        );
    }
    let fixes = history.fixes();
    assert_eq!(fixes.len(), 121, "the history holds {} fixes", fixes.len());
    assert_eq!(fixes[0].time, 180.0);
    assert_eq!(fixes.last().unwrap().time, 300.0);

    let mut history = FinderHistory::default();
    for n in 0..3001 {
        history.push(
            FinderFix {
                time: f64::from(n as u32) * 0.5,
                frame,
                frame_id: n,
            },
            1200.0,
        );
    }
    let fixes = history.fixes();
    assert_eq!(fixes.len(), 1201, "the history holds {} fixes", fixes.len());
    assert_eq!(fixes[0].time, 300.0);
    assert_eq!(fixes.last().unwrap().time, 1500.0);
    assert!(
        fixes.iter().all(|fix| fix.time.fract() == 0.0),
        "the history is not thinned to one a second"
    );
}

/// US4-S3: the frame's up axis answers 0 at the centre with no roll, 360 less
/// 17.188734° with a roll of 0.3 rad, and within half a degree of that at the
/// sky direction of pixel (700, 420) (oracle: the rotation's definition in
/// `sky.rs`: up is north turned by the roll toward the west).
#[test]
fn up_east_of_north_at_answers_the_frames_up() {
    let flat = finder_frame(100.0, 20.0, 0.0);
    let at_centre = flat.up_east_of_north_at(flat.center).unwrap();
    assert!(at_centre.abs() < 1e-9, "at the centre {at_centre}");

    let rolled = finder_frame(100.0, 20.0, 0.3);
    let at_centre = rolled.up_east_of_north_at(rolled.center).unwrap();
    assert!(
        (at_centre - (360.0 - 17.188734)).abs() < 1e-6,
        "at the centre {at_centre}"
    );
    let direction = rolled.unproject((700.0, 420.0)).unwrap();
    let at_pixel = rolled.up_east_of_north_at(direction).unwrap();
    let difference = (at_pixel - at_centre + 180.0).rem_euclid(360.0) - 180.0;
    assert!(
        difference.abs() < 0.5,
        "the two answers differ by {difference}"
    );
}

/// US4-S4: a solve whose exposure no live solve shares takes no aim reading
/// and says why, with the star frames stopped before the exposure and started
/// again after it, while the imaging frame itself still solves (oracle: the
/// stated rules; the imaging camera is independent of the star frames).
#[tokio::test(start_paused = true)]
async fn a_solve_with_no_live_pointing_in_its_exposure_takes_no_reading() {
    let scratch = crate::support::Scratch::new("imaging-relation-s4a");
    let folder = scratch.0.join("imaging");
    let rig = solving_rig(&folder);
    ready(&rig.app).await;
    rig.sim.stop_frames();
    settle().await;
    let view = saved_frame(&rig.app).await;
    rig.sim.start_frames();
    let view = solve_saved(&rig.app, view["frame"]["id"].as_u64().unwrap()).await;
    assert_eq!(view["solve"]["state"], json!("solved"), "{}", view["solve"]);
    assert_eq!(
        view["relation"]["last"]["outcome"],
        json!("no_live_pointing"),
        "{}",
        view["relation"]
    );
    let (_, session) = get(&rig.app, "/api/session").await;
    assert!(session.get("aim").is_none(), "{session}");
}

/// US4-S4: an 8 s exposure of the untracked sky crosses about 117″, against
/// the 60″ allowed, so its solve takes no aim reading and says `moving`
/// (oracle: the sidereal drift of the untracked sky).
#[tokio::test(start_paused = true)]
async fn an_untracked_long_exposure_is_moving() {
    let scratch = crate::support::Scratch::new("imaging-relation-s4b");
    let folder = scratch.0.join("imaging");
    let rig = solving_rig(&folder);
    ready(&rig.app).await;
    let (status, answer) = put(
        &rig.app,
        "/api/imaging/settings",
        json!({"exposure_ms": 8000}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let view = solve_frame(&rig.app).await;
    assert_eq!(view["solve"]["state"], json!("solved"), "{}", view["solve"]);
    assert_eq!(
        view["relation"]["last"]["outcome"],
        json!("moving"),
        "{}",
        view["relation"]
    );
    let (_, session) = get(&rig.app, "/api/session").await;
    assert!(session.get("aim").is_none(), "{session}");
}

/// US4-S4: a failed solve leaves no `relation.last` and no aim (oracle: the
/// stated rules; the failure is the focal length's doubled field height, from
/// US3-S3's 900 mm).
#[tokio::test(start_paused = true)]
async fn a_failed_solve_leaves_no_last_and_no_aim() {
    let scratch = crate::support::Scratch::new("imaging-relation-s4c");
    let folder = scratch.0.join("imaging");
    let rig = fresh_rig(&folder, 900.0);
    ready(&rig.app).await;
    let view = solve_frame(&rig.app).await;
    assert_eq!(view["solve"]["state"], json!("failed"), "{}", view["solve"]);
    assert_eq!(
        view["solve"]["reason"],
        json!("not_solved"),
        "{}",
        view["solve"]
    );
    let (_, imaging) = get(&rig.app, "/api/imaging").await;
    assert!(
        imaging["relation"].get("last").is_none(),
        "{}",
        imaging["relation"]
    );
    let (_, session) = get(&rig.app, "/api/session").await;
    assert!(session.get("aim").is_none(), "{session}");
}

/// US4-S4: a solve asked for after the simulated telescope has swung away and
/// back is hinted by the live solve of the exposure's own moment, not by the
/// session's latest pointing, which is still the swung one, some 49° away
/// (oracle: the frame's truth read by the test before the swing, and the
/// simulator's sidereal drift).
#[tokio::test(start_paused = true)]
async fn a_swung_telescope_is_hinted_by_the_exposures_own_moment() {
    let scratch = crate::support::Scratch::new("imaging-relation-s4d");
    let folder = scratch.0.join("imaging");
    let rig = solving_rig(&folder);
    ready(&rig.app).await;
    pointing_live(&rig.app).await;
    settle().await;

    let view = saved_frame(&rig.app).await;
    let id = view["frame"]["id"].as_u64().unwrap();
    let frame = &view["frame"];
    let middle = frame["start"].as_f64().unwrap() + frame["exposure_ms"].as_f64().unwrap() / 2000.0;
    let truth = rig.sim.imaging_truth_at(middle);

    rig.sim.push_to(50.0, 200.0);
    let swung = wait_session(&rig.app, |session| {
        let pointing = &session["pointing"];
        pointing.is_object()
            && separation_deg(
                pointing["ra_deg"].as_f64().unwrap(),
                pointing["dec_deg"].as_f64().unwrap(),
                truth.ra_deg,
                truth.dec_deg,
            ) > 20.0
    })
    .await;
    let pointing_before = (
        swung["pointing"]["ra_deg"].as_f64().unwrap(),
        swung["pointing"]["dec_deg"].as_f64().unwrap(),
    );
    rig.sim.push_to(50.0, 120.0);
    let view = solve_saved(&rig.app, id).await;
    let solve = &view["solve"];
    assert_eq!(solve["state"], json!("solved"), "{solve}");
    assert_eq!(solve["hint"]["source"], json!("pointing"), "{solve}");
    let separation = separation_deg(
        solve["hint"]["ra_deg"].as_f64().unwrap(),
        solve["hint"]["dec_deg"].as_f64().unwrap(),
        truth.ra_deg,
        truth.dec_deg,
    );
    assert!(
        separation < 1.2,
        "the hint is {separation}° from the frame's truth"
    );
    let swung_out = separation_deg(
        pointing_before.0,
        pointing_before.1,
        truth.ra_deg,
        truth.dec_deg,
    );
    assert!(
        swung_out > 20.0,
        "the swung pointing is only {swung_out}° out"
    );
}

/// US4-S5: a tap on the star of a live preview nearest the true aim point
/// replaces a measured aim point with a two-key one, the relation shows no
/// aim reading while the aim point in force is not a measured one, the next
/// frame's solve is hinted at the tapped pixel and measures again, and a
/// reset clears it (oracle: the centroid the test itself read from the
/// preview, the stated keys and statuses, and the simulator's true aim point).
#[tokio::test(start_paused = true)]
async fn a_tap_replaces_a_measured_aim_point_and_the_next_solve_measures_again() {
    let scratch = crate::support::Scratch::new("imaging-relation-s5a");
    let folder = scratch.0.join("imaging");
    let rig = solving_rig(&folder);
    ready(&rig.app).await;
    pointing_live(&rig.app).await;
    settle().await;

    // A measured aim point in force, as after US4-S1.
    let first = solve_frame(&rig.app).await;
    assert_eq!(
        first["solve"]["state"],
        json!("solved"),
        "{}",
        first["solve"]
    );
    let (_, session) = get(&rig.app, "/api/session").await;
    assert_eq!(session["aim"]["source"], json!("measured"), "{session}");

    // The live preview that holds at least one star, and its centroid nearest
    // (700, 420).
    let mut events = rig.live.subscribe();
    let preview = loop {
        let report = support::event(&mut events).await;
        let Some(seq) = report.preview_seq else {
            continue;
        };
        let Some(preview) = rig.live.preview(seq) else {
            continue;
        };
        if !preview.centroids.is_empty() {
            break preview;
        }
    };
    let star = preview
        .centroids
        .iter()
        .min_by(|a, b| {
            ((a.x - 700.0).hypot(a.y - 420.0)).total_cmp(&((b.x - 700.0).hypot(b.y - 420.0)))
        })
        .expect("a centroid");
    let (status, read) = support::request(
        &rig.app,
        "POST",
        "/api/aim",
        json!({"frame_id": preview.frame_id, "x": star.x.round(), "y": star.y.round()}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{read}");
    let aim = &read["aim"];
    let keys = aim.as_object().expect("the tapped aim");
    assert_eq!(keys.len(), 2, "{aim}");
    assert!(keys.contains_key("x") && keys.contains_key("y"), "{aim}");
    assert!(
        (aim["x"].as_f64().unwrap() - star.x).abs() < 0.5,
        "the tap answered {aim}"
    );
    assert!(
        (aim["y"].as_f64().unwrap() - star.y).abs() < 0.5,
        "the tap answered {aim}"
    );

    let (_, imaging) = get(&rig.app, "/api/imaging").await;
    let relation = &imaging["relation"];
    assert_eq!(relation["readings"], json!(0), "{relation}");
    assert!(relation.get("aim").is_none(), "{relation}");
    assert!(relation.get("angle_deg").is_none(), "{relation}");
    assert!(relation.get("scale").is_some(), "{relation}");

    // The next frame's solve measures again: its hint is taken at the tapped
    // pixel, like the session's pointing read when the solve was asked for.
    let view = saved_frame(&rig.app).await;
    let id = view["frame"]["id"].as_u64().unwrap();
    let (_, before) = get(&rig.app, "/api/session").await;
    let pointing = (
        before["pointing"]["ra_deg"].as_f64().unwrap(),
        before["pointing"]["dec_deg"].as_f64().unwrap(),
    );
    let view = solve_saved(&rig.app, id).await;
    let solve = &view["solve"];
    assert_eq!(solve["state"], json!("solved"), "{solve}");
    let separation = separation_deg(
        solve["hint"]["ra_deg"].as_f64().unwrap(),
        solve["hint"]["dec_deg"].as_f64().unwrap(),
        pointing.0,
        pointing.1,
    );
    assert!(
        separation < 0.1,
        "the hint is {separation}° from the session's pointing"
    );
    let (_, session) = get(&rig.app, "/api/session").await;
    assert_eq!(session["aim"]["source"], json!("measured"), "{session}");
    assert!(
        (session["aim"]["x"].as_f64().unwrap() - 700.0).abs() < 1.5,
        "{session}"
    );
    assert!(
        (session["aim"]["y"].as_f64().unwrap() - 420.0).abs() < 1.5,
        "{session}"
    );

    // The reset clears it.
    let (status, read) = support::request(&rig.app, "DELETE", "/api/aim", json!(null)).await;
    assert_eq!(status, StatusCode::OK, "{read}");
    assert!(read.get("aim").is_none(), "{read}");
    let (_, imaging) = get(&rig.app, "/api/imaging").await;
    assert_eq!(imaging["relation"]["readings"], json!(0), "{imaging}");
}

/// US4-S5: a measured aim point survives a restart of the session, marked in
/// `session.json` only while true, and a tap clears the mark (oracle: the
/// stated keys of `session.json`, and the aim the first rig measured, read by
/// the test from the session's view).
#[tokio::test(start_paused = true)]
async fn a_measured_aim_point_survives_a_restart_and_a_tap_clears_its_mark() {
    let scratch = crate::support::Scratch::new("imaging-relation-s5b");
    let folder = scratch.0.join("imaging");
    let path = scratch.0.join("session.json");
    let rig = rig_with_session(&folder, Session::with_state_file(&path).unwrap());
    ready(&rig.app).await;
    pointing_live(&rig.app).await;
    settle().await;

    let view = solve_frame(&rig.app).await;
    assert_eq!(view["solve"]["state"], json!("solved"), "{}", view["solve"]);
    let (_, session) = get(&rig.app, "/api/session").await;
    let measured = (
        session["aim"]["x"].as_f64().unwrap(),
        session["aim"]["y"].as_f64().unwrap(),
    );
    drop(rig);

    let file: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(file["aim_measured"], json!(true), "{file}");
    let reloaded = Session::with_state_file(&path).unwrap();
    let aim = reloaded
        .snapshot(support::START)
        .aim
        .expect("the aim point");
    assert_eq!(aim.source, Some("measured"), "{aim:?}");
    assert!((aim.x - measured.0).abs() < 1e-9, "{aim:?}");
    assert!((aim.y - measured.1).abs() < 1e-9, "{aim:?}");

    reloaded
        .set_aim(Some(Aim { x: 700.0, y: 420.0 }), support::START)
        .unwrap();
    let file: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert!(file.get("aim_measured").is_none(), "{file}");
}

/// US4-S6: a console with no imaging reads a tapped aim point as exactly the
/// two keys the six existing assertions pin, and `GET /api/imaging` stays
/// `{"state":"off"}` (oracle: the pixel the helper set).
#[tokio::test(start_paused = true)]
async fn a_tapped_aim_point_reads_as_it_always_has_without_imaging() {
    let sim = support::sim();
    let state = support::state(&sim, None, true);
    let app = finder::router(state);
    let (_, session) = support::request(&app, "GET", "/api/session", json!(null)).await;
    assert_eq!(session["aim"], json!({"x": 700.0, "y": 420.0}));
    let (_, imaging) = support::request(&app, "GET", "/api/imaging", json!(null)).await;
    assert_eq!(imaging, json!({"state": "off"}));
}

/// The view's saved frame as `Imaging::solve` builds one (plan.md § Solving on
/// the console), for a solve the test makes itself.
fn frame_of(rig: &Rig, frame: &Value) -> SolveFrame {
    SolveFrame {
        path: rig.root.join(frame["file"].as_str().unwrap()),
        width: frame["width"].as_u64().unwrap() as u32,
        height: frame["height"].as_u64().unwrap() as u32,
        bits: frame["bits"].as_u64().unwrap() as u32,
        bayer: frame["bayer"].as_str().map(str::to_string),
        binning: frame["binning"].as_u64().unwrap() as u32,
        roi: Region {
            x: frame["roi"]["x"].as_u64().unwrap() as u32,
            y: frame["roi"]["y"].as_u64().unwrap() as u32,
            width: frame["roi"]["width"].as_u64().unwrap() as u32,
            height: frame["roi"]["height"].as_u64().unwrap() as u32,
        },
        start: frame["start"].as_f64().unwrap(),
        exposure_s: frame["exposure_ms"].as_f64().unwrap() / 1000.0,
    }
}

/// US4-S7: a solve made outside `Imaging::solve` and offered to `note_solve`
/// takes an aim reading exactly as the route does, changing neither the
/// imaging view's `solve` nor the kept scale, and a frame of a region does
/// nothing (oracle: the simulator's true aim point, as in US4-S1, and the
/// stated keys).
#[tokio::test(start_paused = true)]
async fn note_solve_measures_from_a_solve_made_outside_the_route() {
    let scratch = crate::support::Scratch::new("imaging-relation-s7");
    let folder = scratch.0.join("imaging");
    let rig = solving_rig(&folder);
    ready(&rig.app).await;
    pointing_live(&rig.app).await;
    settle().await;

    // The frame's own SolveFrame, solved by the test itself.
    let view = saved_frame(&rig.app).await;
    let id = view["frame"]["id"].as_u64().unwrap();
    let frame = frame_of(&rig, &view["frame"]);
    let solver = imaging::simulated::SimulatedSolver::new(rig.sim.clone());
    let solve = solver
        .solve(frame.clone(), SolveHint::whole_sky())
        .await
        .expect("the whole sky solves");
    rig.imaging.note_solve(&frame, id, &solve);

    let (_, session) = get(&rig.app, "/api/session").await;
    let aim = &session["aim"];
    assert_eq!(aim["source"], json!("measured"), "{aim}");
    assert!(
        (aim["x"].as_f64().unwrap() - 700.0).abs() < 1.5,
        "the aim point is {aim}"
    );
    assert!(
        (aim["y"].as_f64().unwrap() - 420.0).abs() < 1.5,
        "the aim point is {aim}"
    );
    let (_, imaging) = get(&rig.app, "/api/imaging").await;
    let relation = &imaging["relation"];
    assert_eq!(relation["readings"], json!(1), "{relation}");
    assert_eq!(relation["last"]["frame_id"], json!(id), "{relation}");
    assert_eq!(relation["last"]["outcome"], json!("measured"), "{relation}");
    assert!(imaging.get("solve").is_none(), "{imaging}");
    assert!(relation.get("scale").is_none(), "{relation}");
    assert_eq!(rig.imaging.scale_arcsec_per_px(), None);

    // A frame of a region takes no aim reading.
    let view = saved_frame(&rig.app).await;
    let second = view["frame"]["id"].as_u64().unwrap();
    let mut region = frame_of(&rig, &view["frame"]);
    region.roi = Region {
        x: 0,
        y: 0,
        width: 1236,
        height: 412,
    };
    region.width = 1236;
    region.height = 412;
    rig.imaging.note_solve(&region, second, &solve);
    let (_, imaging) = get(&rig.app, "/api/imaging").await;
    let relation = &imaging["relation"];
    assert_eq!(relation["last"]["frame_id"], json!(id), "{relation}");
    assert_eq!(relation["readings"], json!(1), "{relation}");
}
