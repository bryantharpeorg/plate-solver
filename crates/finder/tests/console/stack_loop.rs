//! US5's console loop. Oracles: the stand-in's sample rule (416 + x + 3y),
//! scripted imaging solves, FITS cards read here, and an injected paused clock.

use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use axum::{http::StatusCode, Router};
use finder::{
    imaging::{
        solver::{
            HintSource, ImagingSolve, ImagingSolver, MeasureFuture, Region, SolveError, SolveFrame,
            SolveFuture, SolveHint, StarMeasure,
        },
        stack::service::StartRequest,
        SettingsEdit,
    },
    link::Transport,
    session::Session,
    AppState,
};
use scope_link::memory;
use scope_node::{
    clock::NodeClock,
    imaging::{synthetic::SyntheticImagingCamera, ImagingCamera},
    service::{NodeControls, NodeService},
    synthetic::SyntheticCamera,
    Settings,
};
use serde_json::{json, Value};
use simulator::{mount::SimMount, Simulator, VirtualClock};

use crate::support::{live as support, Scratch};

#[derive(Clone)]
pub(crate) enum Scripted {
    Solve {
        ra_deg: f64,
        dec_deg: f64,
        up_deg: f64,
        arcsec_per_px: f64,
        mirrored: bool,
    },
    Fail(SolveError),
}

const STANDING: Scripted = Scripted::Solve {
    ra_deg: 83.8221,
    dec_deg: -5.3911,
    up_deg: 30.0,
    arcsec_per_px: 2.16,
    mirrored: false,
};

pub(crate) struct ScriptedSolver {
    script: Vec<(usize, Scripted)>,
    wait: Duration,
    calls: AtomicUsize,
    hints: Mutex<Vec<SolveHint>>,
    pub(crate) measure_script: Mutex<Vec<(usize, Result<f64, SolveError>)>>,
    pub(crate) measure_log: Mutex<Vec<(u32, Option<Region>)>>,
    pub(crate) measure_wait: Mutex<Duration>,
}

impl ScriptedSolver {
    pub(crate) fn new() -> Self {
        Self {
            script: Vec::new(),
            wait: Duration::ZERO,
            calls: AtomicUsize::new(0),
            hints: Mutex::new(Vec::new()),
            measure_script: Mutex::new(Vec::new()),
            measure_log: Mutex::new(Vec::new()),
            measure_wait: Mutex::new(Duration::ZERO),
        }
    }
    pub(crate) fn solve_at(mut self, n: usize, answer: Scripted) -> Self {
        self.script.push((n, answer));
        self
    }
    pub(crate) fn delay(mut self, each: Duration) -> Self {
        self.wait = each;
        self
    }
    pub(crate) fn hints(&self) -> Vec<SolveHint> {
        self.hints.lock().unwrap().clone()
    }
}

impl ImagingSolver for ScriptedSolver {
    fn name(&self) -> &'static str {
        "simulated"
    }
    fn solve(&self, frame: SolveFrame, hint: SolveHint) -> SolveFuture<'_> {
        Box::pin(async move {
            let n = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
            self.hints.lock().unwrap().push(hint);
            tokio::time::sleep(self.wait).await;
            match self
                .script
                .iter()
                .find(|(at, _)| *at == n)
                .map(|(_, s)| s.clone())
                .unwrap_or(STANDING)
            {
                Scripted::Fail(error) => Err(error),
                Scripted::Solve {
                    ra_deg,
                    dec_deg,
                    up_deg,
                    arcsec_per_px,
                    mirrored,
                } => {
                    let (sin, cos) = up_deg.to_radians().sin_cos();
                    let s = arcsec_per_px / 3600.0;
                    let mut cd = [[-s * cos, -s * sin], [s * sin, -s * cos]];
                    if mirrored {
                        cd[0][0] = -cd[0][0];
                        cd[1][0] = -cd[1][0];
                    }
                    let mut solve =
                        ImagingSolve::from_cd(ra_deg, dec_deg, cd, frame.width, frame.height);
                    solve.stars = Some(50);
                    Ok(solve)
                }
            }
        })
    }
    fn measure(&self, frame: SolveFrame, region: Option<Region>) -> MeasureFuture<'_> {
        Box::pin(async move {
            let n = {
                let mut log = self.measure_log.lock().unwrap();
                log.push((frame.width, region));
                log.len()
            };
            let wait = *self.measure_wait.lock().unwrap();
            tokio::time::sleep(wait).await;
            let half_flux_px = self
                .measure_script
                .lock()
                .unwrap()
                .iter()
                .find(|(at, _)| *at == n)
                .map(|(_, answer)| answer.clone())
                .unwrap_or(Ok(2.0))?;
            Ok(StarMeasure {
                half_flux_px,
                stars: 50,
            })
        })
    }
}

struct NodeTime(VirtualClock);
impl NodeClock for NodeTime {
    fn now(&self) -> f64 {
        self.0.node_now()
    }
}

pub(crate) struct Rig {
    pub state: AppState,
    pub app: Router,
    pub sim: Simulator,
    pub solver: Arc<ScriptedSolver>,
    pub root: PathBuf,
    _scratch: Scratch,
}

fn sim() -> Simulator {
    let sim = support::sim();
    sim.connect_mount(SimMount::new(sim.clock().clone()));
    sim.push_to(50.0, 120.0);
    sim
}

fn console(
    sim: &Simulator,
    transport: Transport,
    root: Option<&Path>,
    solver: Option<Arc<dyn ImagingSolver>>,
) -> AppState {
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
    // No live solver: the stack must obtain its own last-solve hint.
    let mut state = AppState::shared(support::DB.clone(), finder::samples::Samples::default())
        .with_session(session)
        .with_clock(Arc::new(support::ConsoleClock(sim.clock().clone())))
        .with_link(transport);
    if let Some(root) = root {
        state = state.with_imaging(root);
    }
    if let Some(solver) = solver {
        state = state.with_imaging_solver(solver);
    }
    state
}

fn stand_in_node(camera: Option<Box<dyn ImagingCamera>>) -> (Simulator, Transport) {
    let sim = sim();
    let clock = Arc::new(NodeTime(sim.clock().clone()));
    let node = NodeService::with_controls(
        SyntheticCamera::new(clock.clone()),
        clock.clone(),
        "test-token",
        Settings::default(),
        NodeControls::default(),
    )
    .unwrap()
    .with_imaging(camera.unwrap_or_else(|| Box::new(SyntheticImagingCamera::new(clock))));
    node.controls()
        .set_frame_interval(Duration::from_millis(500));
    let transport = Transport::connector(move || {
        let node = node.clone();
        async move {
            let (client, server) = tokio::io::duplex(64 * 1024);
            tokio::spawn(async move {
                memory::serve_routes_on(server, node.routes())
                    .await
                    .unwrap();
            });
            Ok::<_, std::io::Error>(client)
        }
    });
    (sim, transport)
}

pub(crate) async fn stand_in_rig(
    name: &str,
    solver: ScriptedSolver,
    camera: Option<Box<dyn ImagingCamera>>,
) -> Rig {
    let scratch = Scratch::new(name);
    let root = scratch.0.join("imaging");
    let (sim, transport) = stand_in_node(camera);
    let solver = Arc::new(solver);
    let state = console(&sim, transport, Some(&root), Some(solver.clone()));
    let app = finder::router(state.clone());
    wait_get(&app, |v| v["state"] == "ready").await;
    Rig {
        state,
        app,
        sim,
        solver,
        root,
        _scratch: scratch,
    }
}

async fn wait_get(app: &Router, done: impl Fn(&Value) -> bool) -> Value {
    for _ in 0..3000 {
        let (status, view) = support::request(app, "GET", "/api/imaging", Value::Null).await;
        assert_eq!(status, StatusCode::OK);
        if done(&view) {
            return view;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("imaging did not reach the awaited state");
}

pub(crate) async fn wait_stack(rig: &Rig, done: impl Fn(&Value) -> bool) -> Value {
    for _ in 0..3000 {
        let view = serde_json::to_value(rig.state.stack().unwrap().view()).unwrap();
        if done(&view) {
            return view;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!(
        "stack did not reach the awaited state: {:?}",
        rig.state.stack().unwrap().view()
    );
}

pub(crate) fn cards(bytes: &[u8]) -> Vec<(String, String)> {
    bytes
        .chunks_exact(80)
        .take_while(|c| &c[..8] != b"END     ")
        .filter_map(|c| {
            (c[8] == b'=').then(|| {
                (
                    String::from_utf8_lossy(&c[..8]).trim().to_string(),
                    String::from_utf8_lossy(&c[10..])
                        .trim()
                        .trim_matches('\'')
                        .trim()
                        .to_string(),
                )
            })
        })
        .collect()
}

fn close(value: f64, expected: f64) {
    assert!((value - expected).abs() <= 1e-6, "{value} != {expected}");
}
fn dropped(view: &Value) -> u64 {
    view["dropped"]
        .as_object()
        .unwrap()
        .values()
        .map(|v| v.as_u64().unwrap())
        .sum()
}
fn balanced(view: &Value) {
    assert_eq!(
        view["taken"].as_u64().unwrap(),
        view["stacked"].as_u64().unwrap() + dropped(view)
    );
}
fn sub(rig: &Rig, view: &Value, id: u64) -> PathBuf {
    rig.root
        .join("runs")
        .join(view["id"].as_str().unwrap())
        .join("subs")
        .join(format!("{id:06}.fits"))
}
fn sidecar(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path.with_extension("solve.json")).unwrap()).unwrap()
}

/// US5-S1: every saved sub is solved and calibrated on the blocking pool; the
/// first accepted sub fixes the grid. The diagonal's 80 * 2.16 / 3600 = .048°.
#[tokio::test(start_paused = true)]
async fn take_solve_place_add_and_stop() {
    let rig = stand_in_rig("stack-loop", ScriptedSolver::new(), None).await;
    let stack = rig.state.stack().unwrap();
    assert!(stack.view().is_none());
    assert!(stack.mean().is_none());
    assert!(stack.stop().await.is_none());
    let first =
        serde_json::to_value(stack.start(StartRequest::default(), None).await.unwrap()).unwrap();
    assert_eq!(first["state"], "stacking");
    let id = first["id"].as_str().unwrap();
    assert_eq!(id.len(), 20);
    assert!(
        id.starts_with("2026-01-16T02-") && id.ends_with('Z'),
        "{id}"
    );
    assert_eq!(first["taken"], 0);
    assert_eq!(first["stacked"], 0);
    assert_eq!(
        first["options"],
        json!({"reject":true,"focus_gate":true,"gains":[1.0,1.0,1.0],"stretch":10.0})
    );
    let second = wait_stack(&rig, |v| v["stacked"].as_u64().unwrap_or(0) >= 2).await;
    assert_eq!(second["queued"], 0);
    wait_stack(&rig, |v| v["stacked"].as_u64().unwrap_or(0) >= 5).await;
    let end = serde_json::to_value(stack.stop().await.unwrap()).unwrap();
    assert_eq!(end["state"], "ended");
    assert_eq!(end["ended_reason"], "stopped");
    assert!(end["ended_at"].as_f64().unwrap() >= end["started_at"].as_f64().unwrap());
    balanced(&end);
    for (reason, count) in end["dropped"].as_object().unwrap() {
        assert!(
            count.as_u64().unwrap() <= u64::from(reason == "unprocessed"),
            "{end}"
        );
    }
    close(
        end["exposure_s"].as_f64().unwrap(),
        end["stacked"].as_f64().unwrap(),
    );
    let reference = &end["reference"];
    assert_eq!(reference["frame_id"], 1);
    assert_eq!(reference["width"], 64);
    assert_eq!(reference["height"], 48);
    for (key, expected) in [
        ("ra_deg", 83.8221),
        ("dec_deg", -5.3911),
        ("up_east_of_north_deg", 30.0),
        ("arcsec_per_px", 2.16),
        ("exposure_ms", 1000.0),
    ] {
        close(reference[key].as_f64().unwrap(), expected);
    }
    let last = &end["last"];
    assert_eq!(last["outcome"], "stacked");
    close(last["turn_deg"].as_f64().unwrap(), 0.0);
    for x in last["shift_px"].as_array().unwrap() {
        close(x.as_f64().unwrap(), 0.0);
    }
    assert_eq!(last["rejected_px"], 0);
    assert!(last["took_ms"].as_f64().unwrap() >= 0.0);
    assert_eq!(end["missing"], json!(["dark", "flat", "bad_pixels"]));
    let mean = stack.mean().unwrap();
    assert_eq!((mean.width, mean.height), (64, 48));
    for c in 0..3 {
        for y in 1..47 {
            for x in 1..63 {
                assert!((mean.at(c, x, y) - (416 + x + 3 * y) as f32).abs() <= 1e-4);
            }
        }
    }
    let mut solved = 0;
    for id in 1..=end["taken"].as_u64().unwrap() {
        let path = sub(&rig, &end, id);
        assert!(cards(&fs::read(&path).unwrap()).contains(&("PURPOSE".into(), "sub".into())));
        if path.with_extension("solve.json").exists() {
            assert_eq!(sidecar(&path)["solved"], true);
            solved += 1;
        }
    }
    assert!(solved >= end["stacked"].as_u64().unwrap());
    assert!(!rig.root.join("frames").exists());
    let hints = rig.solver.hints();
    assert_eq!(hints[0].source, HintSource::None);
    for h in &hints[1..] {
        assert_eq!(h.source, HintSource::LastSolve);
        let (ra, dec) = h.position.unwrap();
        close(ra, 83.8221);
        close(dec, -5.3911);
        close(h.radius_deg, 0.048);
        close(h.fov_deg.unwrap(), 0.0288);
    }
}

/// US5-S1's negative control: a solve alone does not choose a reference sub;
/// the next hint follows the last solved sub rather than the reference sub.
#[tokio::test(start_paused = true)]
async fn unreadable_first_sub_and_last_solved_hint() {
    let rig = stand_in_rig(
        "stack-unreadable",
        ScriptedSolver::new().delay(Duration::from_secs(5)),
        None,
    )
    .await;
    let stack = rig.state.stack().unwrap();
    let view =
        serde_json::to_value(stack.start(StartRequest::default(), None).await.unwrap()).unwrap();
    while rig.solver.hints().is_empty() {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    fs::write(sub(&rig, &view, 1), b"not an image").unwrap();
    let read = wait_stack(&rig, |v| {
        v["dropped"]["unreadable"] == 1 && v["reference"]["frame_id"] == 2
    })
    .await;
    assert_eq!(read["dropped"]["unreadable"], 1);
    assert_eq!(rig.solver.hints()[1].source, HintSource::None);
    stack.stop().await;

    let rig = stand_in_rig(
        "stack-last-solved",
        ScriptedSolver::new().solve_at(
            2,
            Scripted::Solve {
                ra_deg: 83.811662,
                dec_deg: -5.385100,
                up_deg: 30.0,
                arcsec_per_px: 2.16,
                mirrored: false,
            },
        ),
        None,
    )
    .await;
    let stack = rig.state.stack().unwrap();
    stack.start(StartRequest::default(), None).await.unwrap();
    wait_stack(&rig, |v| v["last"]["frame_id"].as_u64().unwrap_or(0) >= 3).await;
    let (ra, dec) = rig.solver.hints()[2].position.unwrap();
    close(ra, 83.811662);
    close(dec, -5.385100);
    stack.stop().await;
}

/// US5-S2: scripted failures are independent of the stacker. Even a solved
/// antipode is dropped before disk decoding, and is never offered to note_solve.
#[tokio::test(start_paused = true)]
async fn failed_solves_and_an_antipode_are_dropped() {
    let solver = ScriptedSolver::new()
        .solve_at(1, Scripted::Fail(SolveError::NotSolved { detail: None }))
        .solve_at(2, Scripted::Fail(SolveError::NotSolved { detail: None }))
        .solve_at(5, Scripted::Fail(SolveError::TooFewStars { detail: None }))
        .solve_at(7, Scripted::Fail(SolveError::Failed("boom".into())))
        .solve_at(9, Scripted::Fail(SolveError::TimedOut { seconds: 120 }));
    let rig = stand_in_rig("stack-solve-failures", solver, None).await;
    let stack = rig.state.stack().unwrap();
    stack.start(StartRequest::default(), None).await.unwrap();
    wait_stack(&rig, |v| v["taken"].as_u64().unwrap_or(0) >= 10).await;
    let end = serde_json::to_value(stack.stop().await.unwrap()).unwrap();
    assert_eq!(end["reference"]["frame_id"], 3);
    assert_eq!(end["dropped"]["not_solved"], 2);
    assert_eq!(end["dropped"]["too_few_stars"], 1);
    assert_eq!(end["dropped"]["solver_failed"], 2);
    balanced(&end);
    assert_eq!(
        end["stacked"].as_u64().unwrap(),
        end["taken"].as_u64().unwrap() - 5 - end["dropped"]["unprocessed"].as_u64().unwrap()
    );
    for id in 1..=end["taken"].as_u64().unwrap() {
        assert!(sub(&rig, &end, id).is_file());
    }
    for (id, reason) in [
        (1, "not_solved"),
        (2, "not_solved"),
        (5, "too_few_stars"),
        (7, "failed"),
        (9, "timed_out"),
    ] {
        let solve = sidecar(&sub(&rig, &end, id));
        assert_eq!(solve["solved"], false);
        assert_eq!(solve["reason"], reason);
    }

    let rig = stand_in_rig(
        "stack-antipode",
        ScriptedSolver::new().solve_at(
            2,
            Scripted::Solve {
                ra_deg: 263.8221,
                dec_deg: 5.3911,
                up_deg: 30.0,
                arcsec_per_px: 2.16,
                mirrored: false,
            },
        ),
        None,
    )
    .await;
    let stack = rig.state.stack().unwrap();
    let view =
        serde_json::to_value(stack.start(StartRequest::default(), None).await.unwrap()).unwrap();
    while rig.solver.hints().len() < 2 {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    // A valid solve at the antipode must win over an unreadable file.
    fs::write(sub(&rig, &view, 2), b"not an image").unwrap();
    let view = wait_stack(&rig, |v| v["last"]["frame_id"] == 2).await;
    assert_eq!(view["last"]["outcome"], "off_field");
    assert_eq!(view["dropped"]["off_field"], 1);
    assert_eq!(view["dropped"]["unreadable"], 0);
    let (_, session) = support::request(&rig.app, "GET", "/api/session", Value::Null).await;
    assert_eq!(session["imaging"]["relation"]["last"]["frame_id"], 1);
    stack.stop().await;
}

/// US5-S3: stopping abandons the solver and waiting subs, aborts a long
/// exposure, releases the lease and freezes the counts. Tolerance: 1 virtual s.
#[tokio::test(start_paused = true)]
async fn stop_abandons_a_slow_solve_and_aborts_a_long_exposure() {
    let rig = stand_in_rig(
        "stack-stop-solve",
        ScriptedSolver::new().delay(Duration::from_secs(30)),
        None,
    )
    .await;
    let stack = rig.state.stack().unwrap();
    stack.start(StartRequest::default(), None).await.unwrap();
    wait_stack(&rig, |v| v["taken"].as_u64().unwrap_or(0) >= 3).await;
    let before = rig.sim.clock().now();
    let end = stack.stop().await.unwrap();
    assert!(rig.sim.clock().now() - before <= 1.0);
    assert_eq!(end.state, "ended");
    assert_eq!(end.ended_reason, Some("stopped"));
    assert_eq!(end.stacked, 0);
    assert_eq!(end.dropped.unprocessed, end.taken);
    assert_eq!(end.queued, 0);
    assert_eq!(rig.state.imaging().unwrap().holder(), None);
    tokio::time::sleep(Duration::from_secs(60)).await;
    assert_eq!(stack.view(), Some(end.clone()));
    assert_eq!(stack.stop().await, Some(end));

    let rig = stand_in_rig("stack-stop-exposure", ScriptedSolver::new(), None).await;
    let imaging = rig.state.imaging().unwrap();
    imaging
        .set(serde_json::from_value::<SettingsEdit>(json!({"exposure_ms":10000})).unwrap())
        .await
        .unwrap();
    let stack = rig.state.stack().unwrap();
    stack.start(StartRequest::default(), None).await.unwrap();
    wait_stack(&rig, |v| v["taken"].as_u64().unwrap_or(0) >= 1).await;
    wait_get(&rig.app, |v| v["frame"]["state"] == "exposing").await;
    let before = rig.sim.clock().now();
    let end = stack.stop().await.unwrap();
    assert!(rig.sim.clock().now() - before <= 1.0);
    assert_eq!(end.state, "ended");
    assert_eq!(end.ended_reason, Some("stopped"));
    assert_eq!(imaging.holder(), None);
}

/// Lease folder extension: the holder creates it, saves never replace a file,
/// and neither a missing folder nor the next lease inherits a previous folder.
#[tokio::test(start_paused = true)]
async fn a_save_folder_belongs_to_its_lease() {
    let rig = stand_in_rig("stack-lease-folder", ScriptedSolver::new(), None).await;
    let imaging = rig.state.imaging().unwrap();
    let lease = imaging.lease("the test").unwrap();
    let folder = rig.root.join("runs/test/subs");
    fs::create_dir_all(&folder).unwrap();
    lease.save_in("runs/test/subs");
    let id = lease.take("sub").await.unwrap().frame.unwrap().id;
    let frame = imaging.ended(id).await;
    assert_eq!(frame.file.as_deref(), Some("runs/test/subs/000001.fits"));
    // Pre-existing final files are kept and the temporary file is removed.
    fs::write(folder.join("000002.fits"), b"keep me").unwrap();
    let id = lease.take("sub").await.unwrap().frame.unwrap().id;
    let frame = imaging.ended(id).await;
    assert_eq!(frame.state, "failed");
    assert_eq!(
        frame.error.as_deref(),
        Some("could not save the imaging frame: runs/test/subs/000002.fits already exists")
    );
    assert_eq!(fs::read(folder.join("000002.fits")).unwrap(), b"keep me");
    assert!(!folder.join("000002.fits.part").exists());
    lease.save_in("runs/absent/subs");
    let id = lease.take("sub").await.unwrap().frame.unwrap().id;
    let frame = imaging.ended(id).await;
    assert_eq!(frame.state, "failed");
    assert!(frame
        .error
        .unwrap()
        .starts_with("could not save the imaging frame: "));
    assert!(!rig.root.join("runs/absent").exists());
    drop(lease);
    let lease = imaging.lease("the next test").unwrap();
    let id = lease.take("frame").await.unwrap().frame.unwrap().id;
    let frame = imaging.ended(id).await;
    assert_eq!(frame.file.as_deref(), Some("frames/2026-01-16/000004.fits"));
}
