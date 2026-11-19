//! US6: settings, focus and placement gates, a bounded waiting queue, and the
//! loop's start refusals, simulated placement and measured aim point. Oracles:
//! scripted camera/solver answers, worked examples, simulator truth sidecars and
//! an independent tangent-plane projection. Counts are exact; angles/settings
//! use 1e-6, placements 0.01 px (scripted) or 0.5 px (simulated), aim 1.5 px.

use crate::{
    stack_camera::ScriptedCamera,
    stack_loop::{stand_in_rig, wait_stack, Rig, Scripted, ScriptedSolver},
    support::{live as support, Scratch},
};
use axum::{http::StatusCode, Router};
use finder::{
    imaging::{
        simulated::SimulatedSolver,
        solver::{ImagingSolver, Region, SolveError},
        stack::{judge, same_settings, service::StartRequest, Placement, SubSettings},
        Refusal,
    },
    link::Transport,
    session::Session,
    AppState,
};
use scope_link::memory;
use scope_node::{
    clock::NodeClock,
    imaging::synthetic::SyntheticImagingCamera,
    service::{NodeControls, NodeService},
    synthetic::SyntheticCamera,
    Settings,
};
use serde_json::{json, Value};
use simulator::{mount::SimMount, SimNode, Simulator, VirtualClock};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

pub(crate) trait GateScripts {
    fn measure_at(self, n: usize, answer: Result<f64, SolveError>) -> Self;
    fn measures(&self) -> Vec<(u32, Option<Region>)>;
    fn measure_delay(self, each: Duration) -> Self;
}
impl GateScripts for ScriptedSolver {
    fn measure_at(self, n: usize, answer: Result<f64, SolveError>) -> Self {
        self.measure_script.lock().unwrap().push((n, answer));
        self
    }
    fn measures(&self) -> Vec<(u32, Option<Region>)> {
        self.measure_log.lock().unwrap().clone()
    }
    fn measure_delay(self, each: Duration) -> Self {
        *self.measure_wait.lock().unwrap() = each;
        self
    }
}
pub(crate) trait CameraScripts {
    fn gain_at(self, n: usize, gain: f64) -> Self;
}
impl CameraScripts for ScriptedCamera {
    fn gain_at(self, n: usize, gain: f64) -> Self {
        self.gain_script.lock().unwrap().push((n, gain));
        self
    }
}
struct NodeTime(VirtualClock);
impl NodeClock for NodeTime {
    fn now(&self) -> f64 {
        self.0.node_now()
    }
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

fn stand_in_node(camera: bool) -> (Simulator, Transport) {
    let sim = sim();
    let clock = Arc::new(NodeTime(sim.clock().clone()));
    let node = NodeService::with_controls(
        SyntheticCamera::new(clock.clone()),
        clock.clone(),
        "test-token",
        Settings::default(),
        NodeControls::default(),
    )
    .unwrap();
    let node = if camera {
        node.with_imaging(SyntheticImagingCamera::new(clock))
    } else {
        node
    };
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

async fn get(app: &Router, path: &str) -> Value {
    let (status, view) = support::request(app, "GET", path, Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{view}");
    view
}
fn files_under(path: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in fs::read_dir(path).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files.extend(files_under(&path));
        } else {
            files.push(path);
        }
    }
    files
}
fn refusal(
    result: Result<finder::imaging::stack::service::StackView, Refusal>,
    status: u16,
    message: &str,
) {
    let error = result.unwrap_err();
    assert_eq!(error.status.as_u16(), status);
    assert_eq!(error.message, message);
}
async fn until_taken(rig: &Rig, taken: u64) -> finder::imaging::stack::service::StackView {
    let stack = rig.state.stack().unwrap();
    stack.start(StartRequest::default(), None).await.unwrap();
    wait_stack(rig, |v| v["taken"].as_u64().unwrap_or(0) >= taken).await;
    let end = stack.stop().await.unwrap();
    balanced(&serde_json::to_value(&end).unwrap());
    end
}
fn settings() -> SubSettings {
    SubSettings {
        width: 64,
        height: 48,
        bits: 16,
        bayer: Some("RGGB".into()),
        binning: 1,
        roi: Region {
            x: 0,
            y: 0,
            width: 64,
            height: 48,
        },
        gain: Some(120.0),
        offset: Some(10.0),
        exposure_ms: 1000.0,
    }
}

/// US6-S1: equality clauses and tolerances worked independently in the spec.
#[test]
fn settings_are_compared_with_explicit_tolerances() {
    let reference = settings();
    assert!(same_settings(&reference, &reference));
    for sub in [
        SubSettings {
            gain: Some(120.0005),
            ..settings()
        },
        SubSettings {
            exposure_ms: 1000.5,
            ..settings()
        },
    ] {
        assert!(same_settings(&reference, &sub));
    }
    let absent = SubSettings {
        gain: None,
        offset: None,
        ..settings()
    };
    assert!(same_settings(&absent, &absent));
    for sub in [
        SubSettings {
            width: 32,
            ..settings()
        },
        SubSettings {
            height: 24,
            ..settings()
        },
        SubSettings {
            bits: 8,
            ..settings()
        },
        SubSettings {
            bayer: Some("GRBG".into()),
            ..settings()
        },
        SubSettings {
            bayer: None,
            ..settings()
        },
        SubSettings {
            binning: 2,
            ..settings()
        },
        SubSettings {
            roi: Region {
                x: 2,
                ..reference.roi
            },
            ..settings()
        },
        SubSettings {
            gain: Some(120.002),
            ..settings()
        },
        SubSettings {
            gain: None,
            ..settings()
        },
        SubSettings {
            offset: Some(10.002),
            ..settings()
        },
        SubSettings {
            offset: None,
            ..settings()
        },
        SubSettings {
            exposure_ms: 1001.5,
            ..settings()
        },
        SubSettings {
            exposure_ms: 998.5,
            ..settings()
        },
    ] {
        assert!(!same_settings(&reference, &sub), "{sub:?}");
    }
}

#[tokio::test(start_paused = true)]
async fn a_changed_gain_is_dropped_before_solving() {
    let camera =
        ScriptedCamera::new(Arc::new(NodeTime(VirtualClock::new(support::START)))).gain_at(4, 50.0);
    let rig = stand_in_rig(
        "gate-settings",
        ScriptedSolver::new(),
        Some(Box::new(camera)),
    )
    .await;
    let end = until_taken(&rig, 6).await;
    assert_eq!(end.dropped.settings, 1);
    assert_eq!(end.stacked, end.taken - 1 - end.dropped.unprocessed);
    assert!(!sub(&rig, &serde_json::to_value(&end).unwrap(), 4)
        .with_extension("solve.json")
        .exists());
}

/// US6-S2: 2 px * 2.16 = 4.32 arcsec; 2.7 is 35% worse, 2.5 is 25%.
#[tokio::test(start_paused = true)]
async fn focus_gate_uses_only_stacked_readings_and_keeps_each_last_reading() {
    let rig = stand_in_rig(
        "gate-focus",
        ScriptedSolver::new()
            .measure_at(2, Ok(5.0))
            .measure_at(7, Ok(2.7))
            .measure_at(8, Ok(2.5)),
        None,
    )
    .await;
    let end = until_taken(&rig, 10).await;
    assert_eq!(end.dropped.soft, 1);
    close(end.focus_median_arcsec.unwrap(), 4.32);
    assert_eq!(end.stacked, end.taken - 1 - end.dropped.unprocessed);

    let rig = stand_in_rig(
        "gate-last-focus",
        ScriptedSolver::new().measure_at(7, Ok(2.7)),
        None,
    )
    .await;
    let stack = rig.state.stack().unwrap();
    stack.start(StartRequest::default(), None).await.unwrap();
    let seventh = wait_stack(&rig, |v| v["last"]["frame_id"] == 7).await;
    assert_eq!(seventh["last"]["outcome"], "soft");
    close(seventh["last"]["focus_arcsec"].as_f64().unwrap(), 5.832);
    let eighth = wait_stack(&rig, |v| v["last"]["frame_id"] == 8).await;
    close(eighth["last"]["focus_arcsec"].as_f64().unwrap(), 4.32);
    stack.stop().await;
}

#[tokio::test(start_paused = true)]
async fn focus_gate_minimum_lower_median_and_rejected_readings() {
    let rig = stand_in_rig(
        "gate-five",
        ScriptedSolver::new().measure_at(6, Ok(2.7)),
        None,
    )
    .await;
    assert_eq!(until_taken(&rig, 10).await.dropped.soft, 1);
    let rig = stand_in_rig(
        "gate-lower-median",
        ScriptedSolver::new().measure_at(2, Ok(3.0)),
        None,
    )
    .await;
    let stack = rig.state.stack().unwrap();
    stack.start(StartRequest::default(), None).await.unwrap();
    let second = wait_stack(&rig, |v| v["stacked"] == 2).await;
    close(second["focus_median_arcsec"].as_f64().unwrap(), 4.32);
    stack.stop().await;
    let mut solver = ScriptedSolver::new();
    for n in 6..=12 {
        solver = solver.measure_at(n, Ok(3.0));
    }
    let rig = stand_in_rig("gate-rejected-not-learned", solver, None).await;
    assert_eq!(until_taken(&rig, 13).await.dropped.soft, 7);
}

#[tokio::test(start_paused = true)]
async fn focus_measure_failures_far_out_and_disabled_gate() {
    let rig = stand_in_rig(
        "gate-measure-fail",
        ScriptedSolver::new()
            .measure_at(3, Err(SolveError::TooFewStars { detail: None }))
            .measure_at(4, Err(SolveError::Failed("x".into()))),
        None,
    )
    .await;
    let end = until_taken(&rig, 10).await;
    assert_eq!(end.dropped.too_few_stars, 1);
    assert_eq!(end.dropped.solver_failed, 1);
    let rig = stand_in_rig(
        "gate-far-out",
        ScriptedSolver::new().measure_at(3, Ok(25.0)),
        None,
    )
    .await;
    let end = until_taken(&rig, 10).await;
    assert_eq!(end.dropped.soft, 1);
    assert_eq!(end.stacked, end.taken - 1 - end.dropped.unprocessed);
    let rig = stand_in_rig(
        "gate-disabled",
        ScriptedSolver::new().measure_at(1, Ok(25.0)),
        None,
    )
    .await;
    let stack = rig.state.stack().unwrap();
    stack
        .start(
            StartRequest {
                focus_gate: Some(false),
                ..Default::default()
            },
            None,
        )
        .await
        .unwrap();
    wait_stack(&rig, |v| v["taken"].as_u64().unwrap_or(0) >= 10).await;
    let end = stack.stop().await.unwrap();
    assert!(rig.solver.measures().is_empty());
    assert_eq!(end.dropped.soft, 0);
    assert!(end.focus_median_arcsec.is_none());
}

#[tokio::test(start_paused = true)]
async fn stopping_abandons_a_measure_in_flight() {
    let rig = stand_in_rig(
        "gate-stop-measure",
        ScriptedSolver::new().measure_delay(Duration::from_secs(30)),
        None,
    )
    .await;
    let stack = rig.state.stack().unwrap();
    stack.start(StartRequest::default(), None).await.unwrap();
    wait_stack(&rig, |v| v["taken"].as_u64().unwrap_or(0) >= 2).await;
    assert_eq!(rig.solver.measures().len(), 1);
    let before = tokio::time::Instant::now();
    let end = tokio::time::timeout(Duration::from_secs(1), stack.stop())
        .await
        .unwrap()
        .unwrap();
    assert!(before.elapsed() <= Duration::from_secs(1));
    assert_eq!(end.stacked, 0);
    assert_eq!(end.dropped.unprocessed, end.taken);
}

/// US6-S3: half the shorter side is 24 px; a mirror fails before shift.
#[test]
fn placement_judge_checks_area_before_distance() {
    let identity = Placement::identity();
    for (placement, expected) in [
        (identity, Ok(())),
        (
            Placement {
                t: [-20.0, 0.0],
                ..identity
            },
            Ok(()),
        ),
        (
            Placement {
                t: [-30.0, 0.0],
                ..identity
            },
            Err("off_field"),
        ),
        (
            Placement {
                m: [[0.94, 0.0], [0.0, 0.94]],
                ..identity
            },
            Err("solve_differs"),
        ),
        (
            Placement {
                m: [[-1.0, 0.0], [0.0, 1.0]],
                t: [64.0, 0.0],
            },
            Err("solve_differs"),
        ),
        (
            Placement {
                m: [[-1.0, 0.0], [0.0, -1.0]],
                t: [64.0, 48.0],
            },
            Ok(()),
        ),
        (
            Placement {
                t: [-17.0, -17.0],
                ..identity
            },
            Err("off_field"),
        ),
        (
            Placement {
                m: [[-1.0, 0.0], [0.0, 1.0]],
                t: [94.0, 0.0],
            },
            Err("solve_differs"),
        ),
        (
            Placement {
                m: [[f64::NAN, 0.0], [0.0, 1.0]],
                ..identity
            },
            Err("solve_differs"),
        ),
    ] {
        assert_eq!(
            judge(&placement, (64, 48), (64, 48)),
            expected,
            "{placement:?}"
        );
    }
}
fn solve(ra_deg: f64, dec_deg: f64, up_deg: f64, arcsec_per_px: f64) -> Scripted {
    Scripted::Solve {
        ra_deg,
        dec_deg,
        up_deg,
        arcsec_per_px,
        mirrored: false,
    }
}
#[tokio::test(start_paused = true)]
async fn placement_gate_counts_and_reports_shifts_and_half_turns() {
    let rig = stand_in_rig(
        "gate-placement",
        ScriptedSolver::new()
            .solve_at(3, solve(83.811662, -5.385100, 30.0, 2.16))
            .solve_at(4, solve(83.806442, -5.382100, 30.0, 2.16))
            .solve_at(5, solve(83.8221, -5.3911, 30.0, 2.30))
            .solve_at(6, solve(83.8221, -5.3911, 210.0, 2.16)),
        None,
    )
    .await;
    let stack = rig.state.stack().unwrap();
    stack.start(StartRequest::default(), None).await.unwrap();
    let third = wait_stack(&rig, |v| v["last"]["frame_id"] == 3).await;
    assert_eq!(third["last"]["outcome"], "stacked");
    assert!((third["last"]["shift_px"][0].as_f64().unwrap() - 20.0).abs() < 0.01);
    assert!(third["last"]["shift_px"][1].as_f64().unwrap().abs() < 0.01);
    let fourth = wait_stack(&rig, |v| v["last"]["frame_id"] == 4).await;
    assert_eq!(fourth["last"]["outcome"], "off_field");
    let fifth = wait_stack(&rig, |v| v["last"]["frame_id"] == 5).await;
    assert_eq!(fifth["last"]["outcome"], "solve_differs");
    let sixth = wait_stack(&rig, |v| v["last"]["frame_id"] == 6).await;
    assert_eq!(sixth["last"]["outcome"], "stacked");
    close(sixth["last"]["turn_deg"].as_f64().unwrap().abs(), 180.0);
    wait_stack(&rig, |v| v["taken"].as_u64().unwrap_or(0) >= 7).await;
    let end = stack.stop().await.unwrap();
    assert_eq!(end.dropped.off_field, 1);
    assert_eq!(end.dropped.solve_differs, 1);
    balanced(&serde_json::to_value(end).unwrap());
}

/// US6-S4: one sub each second against one solve each ten seconds.
#[tokio::test(start_paused = true)]
async fn queue_drops_the_oldest_without_delaying_the_camera() {
    let rig = stand_in_rig(
        "gate-queue",
        ScriptedSolver::new().delay(Duration::from_secs(10)),
        None,
    )
    .await;
    let stack = rig.state.stack().unwrap();
    stack.start(StartRequest::default(), None).await.unwrap();
    let began = tokio::time::Instant::now();
    let mut second = None;
    loop {
        let view = serde_json::to_value(stack.view()).unwrap();
        assert!(view["queued"].as_u64().unwrap() <= 4, "{view}");
        if second.is_none() && view["stacked"].as_u64().unwrap() >= 2 {
            second = Some(view.clone());
        }
        if view["taken"].as_u64().unwrap() >= 30 {
            break;
        }
        assert!(began.elapsed() <= Duration::from_secs(35));
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(began.elapsed() <= Duration::from_secs(35));
    let end = stack.stop().await.unwrap();
    assert!(end.dropped.behind >= 15);
    assert!(end.stacked >= 2);
    assert!(second.unwrap()["last"]["frame_id"].as_u64().unwrap() >= 7);
    assert_eq!(end.taken, end.stacked + end.dropped.total());
    assert_eq!(end.queued, 0);
}
fn miss() -> Scripted {
    Scripted::Fail(SolveError::NotSolved { detail: None })
}
fn own_hint(hint: &finder::imaging::solver::SolveHint) {
    assert_eq!(hint.source, finder::imaging::solver::HintSource::LastSolve);
    close(hint.radius_deg, 0.048);
}
/// US6-S5: only solve misses count, and a successful solve restores the hint.
#[tokio::test(start_paused = true)]
async fn three_solve_misses_set_aside_the_stacks_hint_until_a_solve() {
    let rig = stand_in_rig(
        "gate-rehint",
        ScriptedSolver::new()
            .solve_at(4, miss())
            .solve_at(5, miss())
            .solve_at(6, miss())
            .solve_at(7, miss()),
        None,
    )
    .await;
    let stack = rig.state.stack().unwrap();
    stack.start(StartRequest::default(), None).await.unwrap();
    wait_stack(&rig, |v| v["stacked"].as_u64().unwrap_or(0) >= 5).await;
    stack.stop().await;
    let hints = rig.solver.hints();
    assert!(hints.len() >= 9);
    for h in &hints[1..6] {
        own_hint(h);
    }
    for h in &hints[6..8] {
        assert_ne!(h.source, finder::imaging::solver::HintSource::LastSolve);
        assert!(h.radius_deg >= 3.0);
    }
    own_hint(&hints[8]);
}
#[tokio::test(start_paused = true)]
async fn a_focus_failure_and_other_solver_errors_do_not_add_misses() {
    let rig = stand_in_rig(
        "gate-focus-no-miss",
        ScriptedSolver::new()
            .solve_at(4, miss())
            .solve_at(5, miss())
            .measure_at(3, Err(SolveError::TooFewStars { detail: None })),
        None,
    )
    .await;
    let end = until_taken(&rig, 7).await;
    assert_eq!(end.dropped.too_few_stars, 1);
    assert_eq!(end.dropped.not_solved, 2);
    own_hint(&rig.solver.hints()[5]);
    for (name, error, expected_own) in [
        ("gate-failed-no-miss", SolveError::Failed("x".into()), true),
        (
            "gate-few-is-miss",
            SolveError::TooFewStars { detail: None },
            false,
        ),
    ] {
        let mut solver = ScriptedSolver::new();
        for n in 4..=6 {
            solver = solver.solve_at(
                n,
                Scripted::Fail(if n == 6 {
                    error.clone()
                } else if expected_own {
                    SolveError::NotSolved { detail: None }
                } else {
                    error.clone()
                }),
            );
        }
        let rig = stand_in_rig(name, solver, None).await;
        until_taken(&rig, 8).await;
        let hints = rig.solver.hints();
        assert_eq!(
            hints[6].source == finder::imaging::solver::HintSource::LastSolve,
            expected_own
        );
    }
}

/// US6-S6: the aid's sensor region is translated into the sub's own pixels.
#[tokio::test(start_paused = true)]
async fn focus_region_is_retained_and_mapped_for_binning_and_roi() {
    for (name, edit, width, expected) in [
        (
            "gate-region",
            json!({}),
            64,
            Some(Region {
                x: 20,
                y: 16,
                width: 20,
                height: 16,
            }),
        ),
        (
            "gate-region-binned",
            json!({"binning":2}),
            32,
            Some(Region {
                x: 10,
                y: 8,
                width: 10,
                height: 8,
            }),
        ),
        (
            "gate-region-roi",
            json!({"roi":{"x":16,"y":8,"width":32,"height":32}}),
            32,
            Some(Region {
                x: 4,
                y: 8,
                width: 20,
                height: 16,
            }),
        ),
        (
            "gate-region-outside",
            json!({"roi":{"x":0,"y":0,"width":32,"height":24}}),
            32,
            None,
        ),
    ] {
        let rig = stand_in_rig(name, ScriptedSolver::new(), None).await;
        let (status, answer) = support::request(
            &rig.app,
            "POST",
            "/api/imaging/focus",
            json!({"tile":{"col":1,"row":1}}),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{answer}");
        wait_get(&rig.app, |v| {
            v["focus"]["aid"]["readings"].as_u64().unwrap_or(0) >= 1
        })
        .await;
        let (status, answer) =
            support::request(&rig.app, "DELETE", "/api/imaging/focus", Value::Null).await;
        assert_eq!(status, StatusCode::OK, "{answer}");
        let view = get(&rig.app, "/api/imaging").await;
        for (key, n) in [("x", 20), ("y", 16), ("width", 20), ("height", 16)] {
            assert_eq!(view["focus"]["region"][key], n);
        }
        let (status, answer) =
            support::request(&rig.app, "PUT", "/api/imaging/settings", edit).await;
        assert_eq!(status, StatusCode::OK, "{answer}");
        let stack = rig.state.stack().unwrap();
        stack.start(StartRequest::default(), None).await.unwrap();
        wait_stack(&rig, |v| v["stacked"].as_u64().unwrap_or(0) >= 3).await;
        let end = stack.stop().await.unwrap();
        assert_eq!(end.dropped.soft, 0);
        close(end.focus_median_arcsec.unwrap(), 4.32);
        let measures: Vec<_> = rig
            .solver
            .measures()
            .into_iter()
            .filter(|(w, _)| *w == width)
            .collect();
        assert!(measures.len() >= 3, "{measures:?}");
        assert!(measures.iter().all(|(_, r)| *r == expected), "{measures:?}");
    }
}

/// US6-S7: request numbers precede console state, which precedes the lease.
#[tokio::test(start_paused = true)]
async fn start_refusals_follow_the_declared_order() {
    let rig = stand_in_rig("gate-start", ScriptedSolver::new(), None).await;
    let stack = rig.state.stack().unwrap();
    assert!(stack.view().is_none());
    assert!(stack.stop().await.is_none());
    for request in [
        StartRequest {
            gains: Some([0.0, 1.0, 1.0]),
            ..Default::default()
        },
        StartRequest {
            stretch: Some(0.5),
            ..Default::default()
        },
        StartRequest {
            gains: Some([0.0, 1.0, 1.0]),
            stretch: Some(0.5),
            ..Default::default()
        },
    ] {
        let words = if request.gains.is_some() {
            "gains must be three numbers from 0.1 to 10"
        } else {
            "stretch must be a number from 1 to 1000"
        };
        refusal(stack.start(request, None).await, 400, words);
        assert!(stack.view().is_none());
    }
    let scratch = Scratch::new("gate-no-solver");
    let (sim, transport) = stand_in_node(true);
    let state = console(&sim, transport, Some(&scratch.0), None);
    let app = finder::router(state.clone());
    wait_get(&app, |v| v["state"] == "ready").await;
    refusal(
        state
            .stack()
            .unwrap()
            .start(StartRequest::default(), None)
            .await,
        409,
        "this console has no imaging solver",
    );
    assert!(state.stack().unwrap().view().is_none());
    let scratch = Scratch::new("gate-no-camera");
    let (sim, transport) = stand_in_node(false);
    let state = console(
        &sim,
        transport,
        Some(&scratch.0),
        Some(Arc::new(ScriptedSolver::new())),
    );
    let app = finder::router(state.clone());
    wait_get(&app, |v| v["state"] == "no_camera").await;
    let stack = state.stack().unwrap();
    refusal(
        stack.start(StartRequest::default(), None).await,
        409,
        "this scope node has no imaging camera",
    );
    refusal(
        stack
            .start(
                StartRequest {
                    gains: Some([0.0, 1.0, 1.0]),
                    ..Default::default()
                },
                None,
            )
            .await,
        400,
        "gains must be three numbers from 0.1 to 10",
    );
    assert!(stack.view().is_none());
    let (sim, transport) = stand_in_node(true);
    assert!(console(&sim, transport, None, None).stack().is_none());
}

#[tokio::test(start_paused = true)]
async fn a_lease_names_its_holder_even_during_its_own_exposure() {
    let rig = stand_in_rig("gate-held", ScriptedSolver::new(), None).await;
    let stack = rig.state.stack().unwrap();
    let imaging = rig.state.imaging().unwrap();
    let lease = imaging.lease("probe").unwrap();
    let error = stack
        .start(StartRequest::default(), None)
        .await
        .unwrap_err();
    assert_eq!(error.status, StatusCode::CONFLICT);
    assert!(error.message.contains("probe"));
    assert!(stack.view().is_none());
    drop(lease);
    assert_eq!(
        stack
            .start(StartRequest::default(), None)
            .await
            .unwrap()
            .state,
        "stacking"
    );
    stack.stop().await;
    let rig = stand_in_rig("gate-held-exposing", ScriptedSolver::new(), None).await;
    let stack = rig.state.stack().unwrap();
    let imaging = rig.state.imaging().unwrap();
    let lease = imaging.lease("probe").unwrap();
    lease.take("frame").await.unwrap();
    assert_eq!(
        get(&rig.app, "/api/imaging").await["frame"]["state"],
        "exposing"
    );
    let error = stack
        .start(StartRequest::default(), None)
        .await
        .unwrap_err();
    assert_eq!(error.status, StatusCode::CONFLICT);
    assert!(error.message.contains("probe"));
    assert!(!error.message.contains("an exposure is in progress"));
    assert!(stack.view().is_none());
    imaging.abort().await;
}
#[tokio::test(start_paused = true)]
async fn an_unleased_exposure_refuses_start_and_running_options_are_limited() {
    let rig = stand_in_rig("gate-unleased-frame", ScriptedSolver::new(), None).await;
    let stack = rig.state.stack().unwrap();
    let imaging = rig.state.imaging().unwrap();
    let (status, view) =
        support::request(&rig.app, "POST", "/api/imaging/frame", Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(imaging.holder(), None);
    assert_eq!(view["frame"]["state"], "exposing");
    refusal(
        stack.start(StartRequest::default(), None).await,
        409,
        "an exposure is in progress",
    );
    assert!(stack.view().is_none());
    imaging.abort().await;
    imaging.ended(view["frame"]["id"].as_u64().unwrap()).await;
    let first = stack.start(StartRequest::default(), None).await.unwrap();
    assert_eq!(
        stack.start(StartRequest::default(), None).await.unwrap().id,
        first.id
    );
    refusal(
        stack
            .start(
                StartRequest {
                    reject: Some(false),
                    ..Default::default()
                },
                None,
            )
            .await,
        409,
        "the stack is running: stop it to change reject or focus_gate",
    );
    let changed = stack
        .start(
            StartRequest {
                gains: Some([2.0, 1.0, 1.0]),
                ..Default::default()
            },
            None,
        )
        .await
        .unwrap();
    assert_eq!(changed.id, first.id);
    assert_eq!(changed.options.gains, [2.0, 1.0, 1.0]);
    stack.stop().await;
}
#[tokio::test(start_paused = true)]
async fn folder_failure_releases_the_lease_and_same_second_runs_get_distinct_folders() {
    let rig = stand_in_rig("gate-folder-failed", ScriptedSolver::new(), None).await;
    fs::create_dir_all(&rig.root).unwrap();
    fs::write(rig.root.join("runs"), b"a regular file").unwrap();
    let stack = rig.state.stack().unwrap();
    let error = stack
        .start(StartRequest::default(), None)
        .await
        .unwrap_err();
    assert_eq!(error.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(error.message.starts_with("cannot make the stack folder:"));
    assert!(stack.view().is_none());
    assert_eq!(rig.state.imaging().unwrap().holder(), None);
    let rig = stand_in_rig("gate-same-second", ScriptedSolver::new(), None).await;
    let stack = rig.state.stack().unwrap();
    let first = stack.start(StartRequest::default(), None).await.unwrap();
    stack.stop().await;
    let second = stack.start(StartRequest::default(), None).await.unwrap();
    assert_eq!(second.id, format!("{}-2", first.id));
    assert!(stack.mean().is_none());
    for id in [&first.id, &second.id] {
        assert!(rig.root.join("runs").join(id).join("subs").is_dir());
    }
    stack.stop().await;
}

struct SimRig {
    state: AppState,
    app: Router,
    sim: Simulator,
    root: PathBuf,
    _scratch: Scratch,
}
async fn sim_rig(name: &str) -> SimRig {
    let scratch = Scratch::new(name);
    let root = scratch.0.join("imaging");
    let sim = sim();
    let node = Arc::new(
        SimNode::new(sim.clone(), "test-token", Default::default())
            .unwrap()
            .with_frame_interval(Duration::from_millis(500)),
    );
    let transport = Transport::connector(move || {
        let node = node.clone();
        async move {
            let (client, server) = tokio::io::duplex(64 * 1024);
            tokio::spawn(async move {
                memory::serve_routes_on(server, node.service().routes())
                    .await
                    .unwrap();
            });
            Ok::<_, std::io::Error>(client)
        }
    });
    let state = console(&sim, transport, Some(&root), None)
        .with_live()
        .with_imaging_solver(Arc::new(SimulatedSolver::new(sim.clone())));
    let app = finder::router(state.clone());
    wait_get(&app, |v| v["state"] == "ready").await;
    for _ in 0..3000 {
        if get(&app, "/api/session").await["pointing"]["source"] == "live" {
            return SimRig {
                state,
                app,
                sim,
                root,
                _scratch: scratch,
            };
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("pointing did not become live");
}
async fn stack_on(rig: &SimRig, subs: u32) -> Value {
    let stack = rig.state.stack().unwrap();
    stack.start(StartRequest::default(), None).await.unwrap();
    for _ in 0..3000 {
        if stack.view().unwrap().stacked >= subs {
            return serde_json::to_value(stack.stop().await.unwrap()).unwrap();
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("stack did not reach {subs}: {:?}", stack.view());
}
fn sim_sidecar(rig: &SimRig, view: &Value, id: &Value) -> Value {
    sidecar(
        &rig.root
            .join("runs")
            .join(view["id"].as_str().unwrap())
            .join("subs")
            .join(format!("{:06}.fits", id.as_u64().unwrap())),
    )
}
/// A by-hand tangent plane projection of the later solve's centre onto the
/// reference grid. No ImagingSolve, Placement or other imaging arithmetic.
fn centre_by_hand(reference: &Value, later: &Value) -> [f64; 2] {
    let ra = reference["ra_deg"].as_f64().unwrap().to_radians();
    let dec = reference["dec_deg"].as_f64().unwrap().to_radians();
    let a = later["ra_deg"].as_f64().unwrap().to_radians();
    let d = later["dec_deg"].as_f64().unwrap().to_radians();
    let v = [d.cos() * a.cos(), d.cos() * a.sin(), d.sin()];
    let b = [dec.cos() * ra.cos(), dec.cos() * ra.sin(), dec.sin()];
    let e = [-ra.sin(), ra.cos(), 0.0];
    let n = [-dec.sin() * ra.cos(), -dec.sin() * ra.sin(), dec.cos()];
    let dot = |w: [f64; 3]| v.into_iter().zip(w).map(|(x, y)| x * y).sum::<f64>();
    let xi = dot(e) / dot(b);
    let eta = dot(n) / dot(b);
    let up = reference["up_east_of_north_deg"]
        .as_f64()
        .unwrap()
        .to_radians();
    let scale = reference["arcsec_per_px"].as_f64().unwrap();
    [
        (-xi * up.cos() + eta * up.sin()) * 206264.806 / scale,
        (-xi * up.sin() - eta * up.cos()) * 206264.806 / scale,
    ]
}
/// US6-S8: two simulator-truth sidecars and an independent projection locate
/// the last sub, including the untracked field's drift. Rejection starts at 9.
#[tokio::test(start_paused = true)]
async fn simulated_subs_are_placed_by_their_solves_with_live_pointing_hints() {
    let rig = sim_rig("gate-sim-placement").await;
    let end = stack_on(&rig, 4).await;
    assert_eq!(end["last"]["outcome"], "stacked");
    let reference = sim_sidecar(&rig, &end, &end["reference"]["frame_id"]);
    let last = sim_sidecar(&rig, &end, &end["last"]["frame_id"]);
    for key in ["ra_deg", "dec_deg", "up_east_of_north_deg", "arcsec_per_px"] {
        close(
            end["reference"][key].as_f64().unwrap(),
            reference[key].as_f64().unwrap(),
        );
    }
    let turn = last["up_east_of_north_deg"].as_f64().unwrap()
        - reference["up_east_of_north_deg"].as_f64().unwrap();
    assert!((end["last"]["turn_deg"].as_f64().unwrap() - turn).abs() <= 0.02);
    let expected = centre_by_hand(&reference, &last);
    for (i, expected) in expected.iter().enumerate() {
        assert!((end["last"]["shift_px"][i].as_f64().unwrap() - expected).abs() <= 0.5);
    }
    let shift = &end["last"]["shift_px"];
    assert!(shift[0].as_f64().unwrap().hypot(shift[1].as_f64().unwrap()) > 3.0);
    for reason in ["not_solved", "too_few_stars", "solver_failed"] {
        assert_eq!(end["dropped"][reason], 0);
    }
    for sidecar in [&reference, &last] {
        assert_eq!(sidecar["hint"]["source"], "pointing");
        close(sidecar["hint"]["radius_deg"].as_f64().unwrap(), 3.0);
    }
    let rig = sim_rig("gate-sim-rejection").await;
    let end = stack_on(&rig, 9).await;
    assert!(end["last"]["rejected_px"].as_u64().unwrap() > 0);
    assert!(files_under(&rig.root)
        .iter()
        .any(|p| p.extension().is_some_and(|e| e == "fits")));
}
/// US6-S9: five accepted whole-sensor subs fill 032's five-reading window,
/// without publishing a page solve or keeping a scale. Track is off.
#[tokio::test(start_paused = true)]
async fn stacked_subs_keep_the_aim_point_measured_without_keeping_a_scale() {
    let rig = sim_rig("gate-sim-aim").await;
    tokio::time::sleep(Duration::from_secs(5)).await;
    assert!(get(&rig.app, "/api/session").await["imaging"]
        .get("solve")
        .is_none());
    let end = stack_on(&rig, 5).await;
    let session = get(&rig.app, "/api/session").await;
    assert_eq!(session["aim"]["source"], "measured");
    let truth = rig.sim.optics().eyepiece_px;
    assert!(
        (session["aim"]["x"].as_f64().unwrap() - truth.0).abs() <= 1.5,
        "{session}"
    );
    assert!(
        (session["aim"]["y"].as_f64().unwrap() - truth.1).abs() <= 1.5,
        "{session}"
    );
    assert_eq!(session["imaging"]["relation"]["readings"], 5);
    assert!(session["imaging"]["relation"].get("scale").is_none());
    assert!(session["imaging"]["relation"].get("held").is_none());
    assert!(session["imaging"].get("solve").is_none());
    let last = sim_sidecar(&rig, &end, &end["last"]["frame_id"]);
    assert_eq!(last["hint"]["source"], "pointing");
    close(last["hint"]["radius_deg"].as_f64().unwrap(), 3.0);
}
