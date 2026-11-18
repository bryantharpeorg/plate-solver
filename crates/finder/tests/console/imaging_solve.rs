//! Solving the latest imaging frame on the console (032 US2): the simulated
//! solver adapter answering from the simulator's truth, the route, the solve in
//! the imaging view, the solve sidecar beside the frame's file, and every
//! refusal. Oracles: the simulator's truth read by the test itself, its
//! point-spread constants, the scenario's words and statuses, and the test
//! solver's own 1.5 s on the virtual clock. Nothing here asserts a hint or a
//! relation.

use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use finder::{
    imaging::{
        self,
        solver::{
            HintSource, ImagingSolve, ImagingSolver, MeasureFuture, Region, SolveError, SolveFrame,
            SolveFuture, SolveHint,
        },
    },
    link::Transport,
    samples::Samples,
    session::Session,
    AppState,
};
use math_core::{angular_distance, UnitVector};
use scope_link::memory;
use serde_json::{json, Value};
use simulator::{
    imaging::{deep_field, FIELD_SEED},
    mount::SimMount,
    SimNode, Simulator, Site, VirtualClock,
};
use tower::ServiceExt;

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

/// The solving rig (the shared setup): its node, a paired session with the site
/// and no target or aim point, the imaging folder `root` when there is one, and
/// the solver when there is one.
struct Rig {
    app: Router,
    sim: Simulator,
    /// The console's imaging camera, when it takes imaging frames.
    imaging: Option<Arc<imaging::Imaging>>,
    /// The imaging folder R, when the console takes imaging frames.
    root: Option<PathBuf>,
}

fn rig(sim: &Simulator, root: Option<&Path>, solver: Option<Arc<dyn ImagingSolver>>) -> Rig {
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
        .with_live();
    let mut imaging = None;
    if let Some(root) = root {
        state = state.with_imaging(root);
        imaging = state.imaging();
    }
    if let Some(solver) = solver {
        state = state.with_imaging_solver(solver);
    }
    Rig {
        app: finder::router(state),
        sim: sim.clone(),
        imaging,
        root: root.map(Path::to_path_buf),
    }
}

/// The solving rig with the simulated solver over the simulator's truth.
fn solving_rig(root: &Path) -> Rig {
    let sim = sim();
    let solver = Arc::new(imaging::simulated::SimulatedSolver::new(sim.clone()));
    rig(&sim, Some(root), Some(solver))
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

async fn ready(app: &Router) -> Value {
    wait_until(app, |view| view["state"] == "ready").await
}

/// A saved frame (the shared setup): one frame asked for, awaited until saved.
/// Answers the view with the frame saved.
async fn saved_frame(app: &Router) -> Value {
    let (status, view) = post(app, "/api/imaging/frame").await;
    assert_eq!(status, StatusCode::OK, "{view}");
    let id = view["frame"]["id"].clone();
    wait_until(app, |view| {
        view["frame"]["id"] == id && view["frame"]["state"] == "saved"
    })
    .await
}

/// One request with a foreign `Origin`, as a browser on another site sends it.
async fn foreign_origin(app: &Router, method: &str, path: &str) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("host", "localhost:8080")
                .header("content-type", "application/json")
                .header("origin", "http://elsewhere.example")
                .body(Body::from(""))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    (status, value)
}

/// The `.solve.json` files under the imaging folder, relative paths, sorted.
fn solve_files(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(folder) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&folder) else {
            continue;
        };
        for entry in entries {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else if path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .ends_with(".solve.json")
            {
                found.push(path.strip_prefix(root).unwrap().display().to_string());
            }
        }
    }
    found.sort();
    found
}

/// The rig's imaging folder R.
fn root(rig: &Rig) -> &Path {
    rig.root.as_deref().expect("the rig takes imaging frames")
}

/// The great-circle separation of two places, in arcseconds.
fn separation_arcsec(ra1: f64, dec1: f64, ra2: f64, dec2: f64) -> f64 {
    angular_distance(
        UnitVector::from_radec(ra1.to_radians(), dec1.to_radians()),
        UnitVector::from_radec(ra2.to_radians(), dec2.to_radians()),
    )
    .to_degrees()
        * 3600.0
}

/// The position angle from one place to another, east of north, in radians.
fn position_angle(ra1: f64, dec1: f64, ra2: f64, dec2: f64) -> f64 {
    let (ra1, dec1, ra2, dec2) = (
        ra1.to_radians(),
        dec1.to_radians(),
        ra2.to_radians(),
        dec2.to_radians(),
    );
    (dec2.cos() * (ra2 - ra1).sin())
        .atan2(dec1.cos() * dec2.sin() - dec1.sin() * dec2.cos() * (ra2 - ra1).cos())
}

/// One imaging frame of a solve, as the test describes it: no file is read.
fn frame(width: u32, height: u32, binning: u32, roi: Region) -> SolveFrame {
    SolveFrame {
        path: PathBuf::new(),
        width,
        height,
        bits: 16,
        bayer: None,
        binning,
        roi,
        start: 1_768_528_810.0,
        exposure_s: 1.0,
    }
}

/// The whole sensor's region, 1236 × 822.
fn whole() -> Region {
    Region {
        x: 0,
        y: 0,
        width: 1236,
        height: 822,
    }
}

/// A hint at a place (the test's own), with a radius of 3° and no field height.
fn hint_at(ra_deg: f64, dec_deg: f64) -> SolveHint {
    SolveHint {
        source: HintSource::Target,
        position: Some((ra_deg, dec_deg)),
        radius_deg: 3.0,
        fov_deg: None,
    }
}

/// US2-S1: a saved frame solves to the simulator's truth, its sidecar holds the
/// same numbers, and the frame's file is untouched (oracle: the truth read from
/// the simulator at the middle of the frame's exposure).
#[tokio::test(start_paused = true)]
async fn a_saved_frame_solves_to_the_truth_and_writes_its_sidecar() {
    let scratch = crate::support::Scratch::new("imaging-solve-s1");
    let folder = scratch.0.join("imaging");
    let rig = solving_rig(&folder);
    ready(&rig.app).await;
    assert_eq!(
        rig.imaging.as_ref().unwrap().solver().unwrap().name(),
        "simulated"
    );

    let view = saved_frame(&rig.app).await;
    let frame = &view["frame"];
    let id = frame["id"].as_u64().unwrap();
    assert_eq!(id, 1);
    let file = frame["file"].as_str().unwrap().to_string();
    let bytes_before = std::fs::read(root(&rig).join(&file)).unwrap();
    let truth = rig.sim.imaging_truth_at(
        frame["start"].as_f64().unwrap() + frame["exposure_ms"].as_f64().unwrap() / 2000.0,
    );

    // The request answers the view at once, the solve in it.
    let (status, answer) = post(&rig.app, "/api/imaging/solve").await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    assert_eq!(answer["solve"]["frame_id"], json!(id));
    assert!(
        answer["solve"]["state"] == json!("solving") || answer["solve"]["state"] == json!("solved"),
        "{answer}"
    );

    let view = wait_until(&rig.app, |view| {
        view["solve"]["frame_id"] == json!(id) && view["solve"]["state"] != "solving"
    })
    .await;
    let solve = &view["solve"];
    assert_eq!(solve["state"], json!("solved"));
    assert_eq!(solve["solver"], json!("simulated"));
    let (ra_deg, dec_deg) = (
        solve["ra_deg"].as_f64().unwrap(),
        solve["dec_deg"].as_f64().unwrap(),
    );
    assert!(
        separation_arcsec(ra_deg, dec_deg, truth.ra_deg, truth.dec_deg) < 0.5,
        "the solve's place is not within 0.5\" of the truth: {solve}"
    );
    let up = solve["up_east_of_north_deg"].as_f64().unwrap();
    let up_diff = (up - truth.up_east_of_north_deg + 180.0).rem_euclid(360.0) - 180.0;
    assert!(up_diff.abs() < 0.01, "the solve's up is out: {solve}");
    assert!(
        (solve["arcsec_per_px"].as_f64().unwrap() - truth.arcsec_per_px).abs() < 0.001,
        "the solve's scale is out: {solve}"
    );
    assert_eq!(solve["mirrored"], json!(false));
    assert!(solve["stars"].as_u64().unwrap() >= 30, "{solve}");
    assert_eq!(solve["file"], json!("frames/2026-01-16/000001.solve.json"));

    // The sidecar beside the frame's file.
    let sidecar = root(&rig).join("frames/2026-01-16/000001.solve.json");
    let sidecar: Value = serde_json::from_slice(&std::fs::read(&sidecar).unwrap()).unwrap();
    assert_eq!(sidecar["format"], json!("imaging-solve/1"));
    assert_eq!(sidecar["frame_id"], json!(1));
    assert_eq!(sidecar["frame"], json!("000001.fits"));
    assert_eq!(sidecar["solver"], json!("simulated"));
    assert_eq!(sidecar["solved"], json!(true));
    assert_eq!(sidecar["width"], json!(1236));
    assert_eq!(sidecar["height"], json!(822));
    for key in ["ra_deg", "dec_deg", "up_east_of_north_deg", "arcsec_per_px"] {
        assert!(
            (sidecar[key].as_f64().unwrap() - solve[key].as_f64().unwrap()).abs() < 1e-9,
            "the sidecar's {key} is not the view's: {sidecar}"
        );
    }
    let cd = solve["cd_deg"].as_array().unwrap();
    let sidecar_cd = sidecar["cd_deg"].as_array().unwrap();
    for (row, expected) in sidecar_cd.iter().zip(cd) {
        for (value, expected) in row
            .as_array()
            .unwrap()
            .iter()
            .zip(expected.as_array().unwrap())
        {
            assert!(
                (value.as_f64().unwrap() - expected.as_f64().unwrap()).abs() < 1e-9,
                "the sidecar's cd_deg is not the view's: {sidecar}"
            );
        }
    }

    // No `.part` name, and the frame's file is as it was.
    for entry in std::fs::read_dir(root(&rig).join("frames/2026-01-16")).unwrap() {
        let name = entry.unwrap().file_name();
        assert!(
            !name.to_str().unwrap().ends_with(".part"),
            "a .part name is left: {name:?}"
        );
    }
    assert_eq!(std::fs::read(root(&rig).join(&file)).unwrap(), bytes_before);

    // The session carries the same solve.
    let (_, session) = get(&rig.app, "/api/session").await;
    assert_eq!(session["imaging"]["solve"]["frame_id"], json!(id));
    assert_eq!(session["imaging"]["solve"]["state"], json!("solved"));
    assert_eq!(session["imaging"]["solve"]["ra_deg"], solve["ra_deg"]);
}

/// US2-S2: the simulated adapter called directly answers for a frame's binning
/// and region, refuses too few stars, a hint beyond its radius and a field
/// height 5% out, and measures stars from the point-spread constants (oracle:
/// the truth at the middle of the exposure and its constants).
#[tokio::test(start_paused = true)]
async fn the_simulated_adapter_answers_from_the_truth() {
    let sim = sim();
    let solver = imaging::simulated::SimulatedSolver::new(sim.clone());
    let truth = sim.imaging_truth_at(1_768_528_810.5);
    let s = truth.arcsec_per_px;
    let phi = truth.up_east_of_north_deg.to_radians();

    // A whole frame at binning 2, with no hint: the truth's centre and scale.
    let solve = solver
        .solve(frame(618, 411, 2, whole()), SolveHint::whole_sky())
        .await
        .unwrap();
    assert!(
        separation_arcsec(solve.ra_deg, solve.dec_deg, truth.ra_deg, truth.dec_deg) < 0.01,
        "{solve:?}"
    );
    assert!((solve.arcsec_per_px - 2.0 * s).abs() < 1e-6, "{solve:?}");

    // The top half of the sensor, once the test has counted its stars itself.
    let stars = deep_field(FIELD_SEED, truth.ra_deg, truth.dec_deg, 0.5);
    let mut counted = 0;
    for star in &stars {
        let rho = separation_arcsec(truth.ra_deg, truth.dec_deg, star.ra_deg, star.dec_deg);
        let theta = position_angle(truth.ra_deg, truth.dec_deg, star.ra_deg, star.dec_deg);
        let x = 618.0 - rho * (theta - phi).sin() / s;
        let y = 411.0 - rho * (theta - phi).cos() / s;
        if (0.0..1236.0).contains(&x) && (0.0..412.0).contains(&y) {
            counted += 1;
        }
    }
    assert!(counted >= 40, "only {counted} stars in the top half");
    let solve = solver
        .solve(
            frame(
                1236,
                412,
                1,
                Region {
                    x: 0,
                    y: 0,
                    width: 1236,
                    height: 412,
                },
            ),
            SolveHint::whole_sky(),
        )
        .await
        .unwrap();
    let separation = separation_arcsec(solve.ra_deg, solve.dec_deg, truth.ra_deg, truth.dec_deg);
    assert!(
        (separation - 205.0 * s).abs() < 0.5,
        "the separation is {separation}, not 205·s"
    );
    let (x, y) = solve.pixel_of(truth.ra_deg, truth.dec_deg).unwrap();
    assert!(
        (x - 618.0).abs() < 0.01 && (y - 411.0).abs() < 0.01,
        "({x}, {y})"
    );

    // A region that holds 1.4 stars on average answers TooFewStars.
    let error = solver
        .solve(
            frame(
                128,
                96,
                1,
                Region {
                    x: 0,
                    y: 0,
                    width: 128,
                    height: 96,
                },
            ),
            SolveHint::whole_sky(),
        )
        .await
        .unwrap_err();
    assert!(matches!(error, SolveError::TooFewStars { .. }), "{error:?}");

    // The hint's place: within its radius or not solved.
    let north = |deg: f64| hint_at(truth.ra_deg, truth.dec_deg + deg);
    assert!(matches!(
        solver
            .solve(frame(1236, 822, 1, whole()), north(5.0))
            .await
            .unwrap_err(),
        SolveError::NotSolved { .. }
    ));
    solver
        .solve(frame(1236, 822, 1, whole()), north(2.0))
        .await
        .unwrap();

    // The hint's field height: within 5% or not solved.
    let height = 822.0 * s / 3600.0;
    let sized = |fov_deg: f64| SolveHint {
        fov_deg: Some(fov_deg),
        ..north(0.0)
    };
    assert!(matches!(
        solver
            .solve(frame(1236, 822, 1, whole()), sized(2.0 * height))
            .await
            .unwrap_err(),
        SolveError::NotSolved { .. }
    ));
    assert!(matches!(
        solver
            .solve(frame(1236, 822, 1, whole()), sized(1.06 * height))
            .await
            .unwrap_err(),
        SolveError::NotSolved { .. }
    ));
    solver
        .solve(frame(1236, 822, 1, whole()), sized(1.04 * height))
        .await
        .unwrap();

    // The star measure, from the point-spread constants: 2·√(2·ln 2)·√(1.2² +
    // (1.0·focus error)²) pixels, divided by the binning.
    let measure = solver
        .measure(frame(1236, 822, 1, whole()), None)
        .await
        .unwrap();
    assert!(
        (measure.half_flux_px - 2.825784).abs() < 1e-6,
        "{measure:?}"
    );
    assert!(measure.stars >= 30, "{measure:?}");
    sim.set_imaging_optics(imaging_optics_with_focus(&sim.imaging_optics(), 2.0));
    let measure = solver
        .measure(frame(1236, 822, 1, whole()), None)
        .await
        .unwrap();
    assert!(
        (measure.half_flux_px - 5.492337).abs() < 1e-4,
        "{measure:?}"
    );
    let measure = solver
        .measure(frame(618, 411, 2, whole()), None)
        .await
        .unwrap();
    assert!(
        (measure.half_flux_px - 2.746168).abs() < 1e-4,
        "{measure:?}"
    );
}

/// The optics with another focus error.
fn imaging_optics_with_focus(
    optics: &simulator::imaging::ImagingOptics,
    focus_error: f64,
) -> simulator::imaging::ImagingOptics {
    let mut optics = *optics;
    optics.focus_error = focus_error;
    optics
}

/// The solver of US2-S4: five errors in order, each after 1.5 s of tokio time,
/// then worked answer B's solve at once.
struct FiveErrors(AtomicUsize);

impl ImagingSolver for FiveErrors {
    fn name(&self) -> &'static str {
        "five errors"
    }

    fn solve(&self, _frame: SolveFrame, _hint: SolveHint) -> SolveFuture<'_> {
        Box::pin(async move {
            let n = self.0.fetch_add(1, Ordering::SeqCst);
            if n < 5 {
                tokio::time::sleep(Duration::from_millis(1500)).await;
            }
            match n {
                0 => Err(SolveError::NotSolved {
                    detail: Some("No solution found!".to_string()),
                }),
                1 => Err(SolveError::TooFewStars { detail: None }),
                2 => Err(SolveError::NoDatabase { detail: None }),
                3 => Err(SolveError::TimedOut { seconds: 120 }),
                4 => Err(SolveError::Failed(
                    "the imaging solver ended with exit code 34".to_string(),
                )),
                _ => Ok(worked_answer_b()),
            }
        })
    }

    fn measure(&self, _frame: SolveFrame, _region: Option<Region>) -> MeasureFuture<'_> {
        Box::pin(async {
            Err(SolveError::Failed(
                "the test solver measures nothing".to_string(),
            ))
        })
    }
}

/// Worked answer B's solve for an imaging frame 1236 × 822: up 30° east of
/// north, 2.16″ a pixel, not mirrored, centred on RA 83.8221°, Dec −5.3911°.
fn worked_answer_b() -> ImagingSolve {
    let scale = 2.16 / 3600.0;
    let theta = (30.0_f64).to_radians();
    ImagingSolve::from_cd(
        83.8221,
        -5.3911,
        [
            [-scale * theta.cos(), -scale * theta.sin()],
            [scale * theta.sin(), -scale * theta.cos()],
        ],
        1236,
        822,
    )
}

/// US2-S4: each of the five solve errors shows its reason, message, detail,
/// sidecar and the console clock's own `took_ms`, and a later solved solve
/// replaces the sidecar (oracle: the words of FR-001 and the test solver's
/// 1.5 s).
#[tokio::test(start_paused = true)]
async fn each_solve_error_shows_its_reason_message_and_sidecar() {
    let scratch = crate::support::Scratch::new("imaging-solve-s4");
    let folder = scratch.0.join("imaging");
    let sim = sim();
    let rig = rig(
        &sim,
        Some(&folder),
        Some(Arc::new(FiveErrors(AtomicUsize::new(0)))),
    );
    ready(&rig.app).await;
    let view = saved_frame(&rig.app).await;
    let id = view["frame"]["id"].as_u64().unwrap();
    let sidecar = root(&rig).join("frames/2026-01-16/000001.solve.json");

    let errors = [
        (
            "not_solved",
            "the imaging solver found no solution",
            Some("No solution found!"),
        ),
        (
            "too_few_stars",
            "the imaging solver found too few stars in the imaging frame",
            None,
        ),
        (
            "no_database",
            "the imaging solver has no star database it can read",
            None,
        ),
        (
            "timed_out",
            "the imaging solver gave no answer within 120 s",
            None,
        ),
        ("failed", "the imaging solver ended with exit code 34", None),
    ];
    for (reason, error, detail) in errors {
        let (status, answer) = post(&rig.app, "/api/imaging/solve").await;
        assert_eq!(status, StatusCode::OK, "{answer}");
        let view = wait_until(&rig.app, |view| {
            view["solve"]["frame_id"] == json!(id) && view["solve"]["state"] == "failed"
        })
        .await;
        let solve = &view["solve"];
        assert_eq!(solve["reason"], json!(reason), "{solve}");
        assert_eq!(solve["error"], json!(error), "{solve}");
        assert_eq!(solve["detail"], json!(detail), "{solve}");
        assert_eq!(solve["file"], json!("frames/2026-01-16/000001.solve.json"));
        let took_ms = solve["took_ms"].as_f64().unwrap();
        assert!((took_ms - 1500.0).abs() < 1.0, "took_ms {took_ms}, {solve}");

        let sidecar: Value = serde_json::from_slice(&std::fs::read(&sidecar).unwrap()).unwrap();
        assert_eq!(sidecar["solved"], json!(false), "{sidecar}");
        assert_eq!(sidecar["reason"], json!(reason), "{sidecar}");
        assert_eq!(sidecar["error"], json!(error), "{sidecar}");
        assert_eq!(sidecar["took_ms"], json!(took_ms), "{sidecar}");
    }

    // The sixth solve is worked answer B's, and its sidecar replaces the last.
    let (status, answer) = post(&rig.app, "/api/imaging/solve").await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    wait_until(&rig.app, |view| {
        view["solve"]["frame_id"] == json!(id) && view["solve"]["state"] == "solved"
    })
    .await;
    let sidecar: Value = serde_json::from_slice(&std::fs::read(&sidecar).unwrap()).unwrap();
    assert_eq!(sidecar["solved"], json!(true), "{sidecar}");
    assert!(sidecar.get("reason").is_none(), "{sidecar}");
}

/// US2-S3: the route refuses before any imaging frame, and the solving rig's
/// solver answers by name (oracle: the scenario's words and statuses).
#[tokio::test(start_paused = true)]
async fn solve_before_any_frame_is_refused() {
    let scratch = crate::support::Scratch::new("imaging-solve-refusal-no-frame");
    let folder = scratch.0.join("imaging");
    let rig = solving_rig(&folder);
    ready(&rig.app).await;

    let before = solve_files(root(&rig));
    let (status, answer) = post(&rig.app, "/api/imaging/solve").await;
    assert_eq!(status, StatusCode::CONFLICT, "{answer}");
    assert_eq!(answer["error"], json!("take an imaging frame first"));
    assert_eq!(solve_files(root(&rig)), before);
}

/// US2-S3: the route refuses while the link is down, with 031's words for
/// `no_link` (oracle: the scenario's words and statuses).
#[tokio::test(start_paused = true)]
async fn solve_while_the_link_is_down_is_refused() {
    let scratch = crate::support::Scratch::new("imaging-solve-refusal-no-link");
    let folder = scratch.0.join("imaging");
    let rig = solving_rig(&folder);
    ready(&rig.app).await;
    saved_frame(&rig.app).await;

    rig.sim.set_unresponsive(true);
    wait_until(&rig.app, |view| view["state"] == "no_link").await;
    let before = solve_files(root(&rig));
    let (status, answer) = post(&rig.app, "/api/imaging/solve").await;
    assert_eq!(status, StatusCode::CONFLICT, "{answer}");
    assert_eq!(answer["error"], json!("the scope node is not connected"));
    assert_eq!(solve_files(root(&rig)), before);
}

/// US2-S3: the route refuses while a frame is still arriving, and after an
/// abort, 1 s after the frame was asked for, with the abort's own wait
/// (oracle: the scenario's words and statuses).
#[tokio::test(start_paused = true)]
async fn solve_while_a_frame_arrives_and_after_an_abort_is_refused() {
    let scratch = crate::support::Scratch::new("imaging-solve-refusal-arriving");
    let folder = scratch.0.join("imaging");
    let rig = solving_rig(&folder);
    ready(&rig.app).await;

    let (status, answer) = support::request(
        &rig.app,
        "PUT",
        "/api/imaging/settings",
        json!({"exposure_ms":10000}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let (status, answer) = post(&rig.app, "/api/imaging/frame").await;
    assert_eq!(status, StatusCode::OK, "{answer}");

    let before = solve_files(root(&rig));
    let (status, answer) = post(&rig.app, "/api/imaging/solve").await;
    assert_eq!(status, StatusCode::CONFLICT, "{answer}");
    assert_eq!(
        answer["error"],
        json!("the imaging frame is still arriving")
    );
    assert_eq!(solve_files(root(&rig)), before);

    tokio::time::sleep(Duration::from_secs(1)).await;
    let (status, answer) = post(&rig.app, "/api/imaging/abort").await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    wait_until(&rig.app, |view| view["frame"]["state"] == "aborted").await;

    let before = solve_files(root(&rig));
    let (status, answer) = post(&rig.app, "/api/imaging/solve").await;
    assert_eq!(status, StatusCode::CONFLICT, "{answer}");
    assert_eq!(
        answer["error"],
        json!("the latest imaging frame was not saved")
    );
    assert_eq!(solve_files(root(&rig)), before);
}

/// US2-S3: the route refuses a foreign `Origin` (oracle: the same-origin rule).
#[tokio::test(start_paused = true)]
async fn solve_from_a_foreign_origin_is_refused() {
    let scratch = crate::support::Scratch::new("imaging-solve-refusal-origin");
    let folder = scratch.0.join("imaging");
    let rig = solving_rig(&folder);
    ready(&rig.app).await;
    saved_frame(&rig.app).await;

    let before = solve_files(root(&rig));
    let (status, answer) = foreign_origin(&rig.app, "POST", "/api/imaging/solve").await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{answer}");
    assert_eq!(solve_files(root(&rig)), before);
}

/// US2-S3: a second solve while one runs is refused, and the first still ends
/// solved (oracle: the scenario's words and statuses).
#[tokio::test(start_paused = true)]
async fn a_second_solve_while_one_runs_is_refused() {
    let scratch = crate::support::Scratch::new("imaging-solve-refusal-running");
    let folder = scratch.0.join("imaging");
    let sim = sim();
    let solver = Arc::new(
        imaging::simulated::SimulatedSolver::new(sim.clone()).with_delay(Duration::from_secs(5)),
    );
    let rig = rig(&sim, Some(&folder), Some(solver));
    ready(&rig.app).await;
    let view = saved_frame(&rig.app).await;
    let id = view["frame"]["id"].as_u64().unwrap();

    let (status, answer) = post(&rig.app, "/api/imaging/solve").await;
    assert_eq!(status, StatusCode::OK, "{answer}");

    tokio::time::sleep(Duration::from_secs(1)).await;
    let before = solve_files(root(&rig));
    let (status, answer) = post(&rig.app, "/api/imaging/solve").await;
    assert_eq!(status, StatusCode::CONFLICT, "{answer}");
    assert_eq!(
        answer["error"],
        json!("an imaging solve is already running")
    );
    assert_eq!(solve_files(root(&rig)), before);

    let view = wait_until(&rig.app, |view| {
        view["solve"]["frame_id"] == json!(id) && view["solve"]["state"] == "solved"
    })
    .await;
    assert_eq!(view["solve"]["state"], json!("solved"), "{view}");
}

/// US2-S3 and US2-S5: without a solver the route refuses and `solver()` answers
/// none; without imaging the route refuses with 031's words and the view is
/// exactly `{"state":"off"}` (oracle: the scenario's words, statuses and body).
#[tokio::test(start_paused = true)]
async fn solve_without_a_solver_or_imaging_is_refused() {
    let scratch = crate::support::Scratch::new("imaging-solve-refusal-no-solver");
    let folder = scratch.0.join("imaging");

    // A console built as the solving rig but without `with_imaging_solver`.
    let unsolved = sim();
    let unsolved = rig(&unsolved, Some(&folder), None);
    ready(&unsolved.app).await;
    saved_frame(&unsolved.app).await;
    assert!(unsolved.imaging.as_ref().unwrap().solver().is_none());
    let before = solve_files(root(&unsolved));
    let (status, answer) = post(&unsolved.app, "/api/imaging/solve").await;
    assert_eq!(status, StatusCode::CONFLICT, "{answer}");
    assert_eq!(answer["error"], json!("this console has no imaging solver"));
    assert_eq!(solve_files(root(&unsolved)), before);

    // A console built without `with_imaging`.
    let bare = sim();
    let bare = rig(&bare, None, None);
    let (status, answer) = get(&bare.app, "/api/imaging").await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    assert_eq!(answer, json!({"state":"off"}));
    let (status, answer) = post(&bare.app, "/api/imaging/solve").await;
    assert_eq!(status, StatusCode::CONFLICT, "{answer}");
    assert_eq!(
        answer["error"],
        json!("this console takes no imaging frames")
    );
}

/// US2-S5: the imaging view has no `solve` key before any solve (oracle: the
/// stated keys).
#[tokio::test(start_paused = true)]
async fn the_view_before_any_solve_has_no_solve_key() {
    let scratch = crate::support::Scratch::new("imaging-solve-view");
    let folder = scratch.0.join("imaging");
    let rig = solving_rig(&folder);
    ready(&rig.app).await;
    saved_frame(&rig.app).await;

    let (_, view) = get(&rig.app, "/api/imaging").await;
    assert!(view.get("solve").is_none(), "{view}");
    let (_, session) = get(&rig.app, "/api/session").await;
    assert!(session["imaging"].get("solve").is_none(), "{session}");
}
