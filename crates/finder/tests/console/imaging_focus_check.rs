//! The focus check (033 US8): `POST /api/imaging/focus` with `check` takes a
//! few imaging frames of the focus region, judges the first settled reading
//! against the focus record, notes nothing, puts the camera's settings back,
//! releases the lease and ends by itself with `done` — at its first settled
//! reading, at its first unmeasurable frame, or after nine frames with
//! neither — and `focus_ended` answers the focus view once no aid or check is
//! running; and the aid turned through focus on the simulator marks
//! tonight's record `passed`. Oracles: the hand-written `focus.json`, a
//! worked example (3.20/3.00 = 1.067; 3.20/2.80 = 1.143; 3.20/2.40 = 1.333;
//! 3.20/3.20 = 1.000; 2.4 and 3.2 sensor pixels times the rig's scale
//! 206264.806 × 19.12 / 1800 = 2.1909906″ a sensor pixel are 5.258377″ and
//! 7.011170″), three exposures of 1 s on the virtual clock before the check
//! can end, and, for the pass through focus, the simulator's truth: H(0) ×
//! the rig's scale = 6.191266″, with 2% of seeded noise on each imaging
//! frame, and H(1)/H(0) = 1.30, so every step is a change and every neighbour
//! of best focus is far from it.

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
        simulated::SimulatedSolver,
        solver::{
            ImagingSolver, MeasureFuture, Region, SolveFrame, SolveFuture, SolveHint, StarMeasure,
        },
        Imaging,
    },
    link::Transport,
    samples::Samples,
    session::Session,
    AppState,
};
use scope_link::memory;
use serde_json::{json, Value};
use simulator::{SimNode, Simulator, Site, VirtualClock};

use crate::support::live as support;

/// The scripted solver (the shared setup): `measure` keeps a note of each
/// call and answers the next entry of its list at once, repeating the last
/// once the list is used up; `solve` is never called by the aid.
struct Scripted {
    entries: Vec<(f64, u32)>,
    next: AtomicUsize,
    calls: std::sync::Mutex<Vec<Call>>,
}

/// What one `measure` call was given (the shared setup): the frame's region
/// of interest, binning and size, and the region asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Call {
    roi: Region,
    binning: u32,
    width: u32,
    height: u32,
    region: Option<Region>,
}

impl Scripted {
    fn new(entries: Vec<(f64, u32)>) -> Self {
        Self {
            entries,
            next: AtomicUsize::new(0),
            calls: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// The `measure` calls the solver was given, in order.
    fn calls(&self) -> Vec<Call> {
        self.calls.lock().unwrap().clone()
    }
}

impl ImagingSolver for Scripted {
    fn name(&self) -> &'static str {
        "scripted"
    }

    fn solve(&self, _frame: SolveFrame, _hint: SolveHint) -> SolveFuture<'_> {
        Box::pin(async {
            Err(finder::imaging::solver::SolveError::Failed(
                "the scripted solver does not solve".to_string(),
            ))
        })
    }

    fn measure(&self, frame: SolveFrame, region: Option<Region>) -> MeasureFuture<'_> {
        self.calls.lock().unwrap().push(Call {
            roi: frame.roi,
            binning: frame.binning,
            width: frame.width,
            height: frame.height,
            region,
        });
        let n = self.next.fetch_add(1, Ordering::SeqCst);
        let answer = Ok(StarMeasure {
            half_flux_px: self.entries[n.min(self.entries.len() - 1)].0,
            stars: self.entries[n.min(self.entries.len() - 1)].1,
        });
        Box::pin(async move { answer })
    }
}

/// The focus rig (the shared setup): the simulator's database at 40° N 75° W
/// on the virtual clock from Unix 1768528800, pointed at the celestial pole,
/// where the imaging field moves by under a pixel in three seconds; a
/// `SimNode` over it with the simulated imaging camera, served over a fresh
/// duplex for each connection; and a console over that with imaging, the
/// given solver and `with_focal_length_mm(1800.0)`, so every size reads in
/// arcseconds. No mount is connected.
struct Rig {
    app: Router,
    sim: Simulator,
    imaging: Arc<Imaging>,
    /// The imaging folder R.
    root: PathBuf,
    solver: Option<Arc<Scripted>>,
}

fn console(sim: &Simulator, root: &Path, solver: Arc<dyn ImagingSolver>) -> Rig {
    let node = Arc::new(
        SimNode::new(sim.clone(), "test-token", Default::default())
            .unwrap()
            .with_frame_interval(Duration::from_millis(500)),
    );
    let transport = Transport::connector(move || {
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
    });
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
    let state = AppState::shared(support::DB.clone(), Samples::default())
        .with_session(session)
        .with_clock(Arc::new(support::ConsoleClock(sim.clock().clone())))
        .with_link(transport)
        .with_live()
        .with_imaging(root)
        .with_imaging_solver(solver)
        .with_focal_length_mm(1800.0);
    let imaging = state.imaging().expect("the rig takes imaging frames");
    Rig {
        app: finder::router(state),
        sim: sim.clone(),
        imaging,
        root: root.to_path_buf(),
        solver: None,
    }
}

/// The rig with the scripted solver, built as the scenario's entries name.
fn focus_rig(root: &Path, entries: Vec<(f64, u32)>) -> Rig {
    let sim = Simulator::new(
        support::DB.clone(),
        Site::default(),
        VirtualClock::new(support::START),
    );
    sim.push_to(40.0, 0.0);
    let solver = Arc::new(Scripted::new(entries));
    let mut rig = console(&sim, root, solver.clone());
    rig.solver = Some(solver);
    rig
}

/// US8-S2's rig: the simulated solver adapter with the scenario's seeded
/// noise, and the simulator's focuser set to the scenario's first error.
fn noise_rig(root: &Path) -> Rig {
    let sim = Simulator::new(
        support::DB.clone(),
        Site::default(),
        VirtualClock::new(support::START),
    );
    sim.push_to(40.0, 0.0);
    sim.set_focus_error(3.0);
    console(
        &sim,
        root,
        Arc::new(SimulatedSolver::new(sim.clone()).with_measure_noise(7, 0.02)),
    )
}

/// The rig's imaging folder R.
fn root(rig: &Rig) -> &Path {
    &rig.root
}

/// Write `focus.json` under R by hand (the shared setup): the scenario's
/// file, with `record` as given or without one.
fn write_focus_file(rig_root: &Path, record: Option<&str>) {
    let record = record
        .map(|record| format!(",\"record\":{record}"))
        .unwrap_or_default();
    std::fs::create_dir_all(rig_root).unwrap();
    std::fs::write(
        rig_root.join("focus.json"),
        format!(
            "{{\"format\":\"imaging-focus/1\",\
             \"camera\":{{\"model\":\"simulated imaging camera\",\"serial\":\"SIM-0001\",\
             \"width\":1236,\"height\":822,\"pixel_um\":19.12}},\
             \"region\":{{\"col\":1,\"row\":1,\"x\":412,\"y\":274,\"width\":412,\"height\":274,\
             \"source\":\"fixed\",\"at\":1768528000.0}}{record}}}"
        ),
    )
    .unwrap();
}

/// Tonight's record of `px`, read at the rig's start (the shared setup).
fn tonight(px: f64) -> String {
    format!("{{\"tonight\":{{\"px\":{px},\"at\":1768528000.0,\"since\":1768528000.0}}}}")
}

/// `focus.json` under R, parsed.
fn focus_file(rig: &Rig) -> Value {
    serde_json::from_slice(
        &std::fs::read(root(rig).join("focus.json")).expect("focus.json under R"),
    )
    .unwrap()
}

/// The region of tile `(col, row)` of the rig's 1236 × 822 sensor at binning 1.
fn tile(col: u32, row: u32) -> Region {
    Region {
        x: 412 * col,
        y: 274 * row,
        width: 412,
        height: 274,
    }
}

async fn get(app: &Router, path: &str) -> (StatusCode, Value) {
    support::request(app, "GET", path, json!(null)).await
}

async fn post_focus(app: &Router, body: Value) -> (StatusCode, Value) {
    support::request(app, "POST", "/api/imaging/focus", body).await
}

/// Read `GET /api/imaging` every 100 ms until `check` holds, for at most
/// 300 s, and answer the view in which it first held.
async fn wait_until(app: &Router, check: impl Fn(&Value) -> bool) -> Value {
    for _ in 0..3000 {
        let (_, view) = get(app, "/api/imaging").await;
        if check(&view) {
            return view;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let (_, view) = get(app, "/api/imaging").await;
    panic!("imaging never reached the awaited state: {view}");
}

async fn wait_ready(app: &Router) -> Value {
    wait_until(app, |view| view["state"] == "ready").await
}

/// Run a check on a fresh rig whose `focus.json` holds `record`, and answer
/// the view in which the check has ended (the shared setup).
async fn run_check(
    rig_root: &Path,
    record: Option<&str>,
    entries: Vec<(f64, u32)>,
) -> (Rig, Value) {
    let rig = focus_rig(rig_root, entries);
    write_focus_file(rig_root, record);
    wait_ready(&rig.app).await;
    let (status, view) = post_focus(&rig.app, json!({"check": true})).await;
    assert_eq!(status, StatusCode::OK, "{view}");
    let view = wait_until(&rig.app, |view| {
        view["focus"]["aid"]["state"] == json!("ended")
    })
    .await;
    (rig, view)
}

/// US8-S1: a check on the scenario's hand-written `focus.json` takes three
/// frames of the kept tile, ends `done` with `good` at 1.067 of tonight's
/// record, notes nothing, puts the settings back, frees the lease, and
/// `focus_ended`, awaited at once after the start, answers its view only
/// after three exposures of one second (oracle: the worked example, the
/// hand-written file and the stated calls, regions and settings, compared
/// exactly).
#[tokio::test(start_paused = true)]
async fn a_check_judges_the_first_settled_reading_and_notes_nothing() {
    let scratch = crate::support::Scratch::new("imaging-focus-check-good");
    let rig_root = scratch.0.join("imaging");
    let rig = focus_rig(&rig_root, vec![(3.20, 12), (3.25, 12), (3.15, 12)]);
    write_focus_file(&rig_root, Some(&tonight(3.0)));
    wait_ready(&rig.app).await;
    let (status, view) = post_focus(&rig.app, json!({"check": true})).await;
    assert_eq!(status, StatusCode::OK, "{view}");
    assert_eq!(view["focus"]["aid"]["check"], json!(true), "{view}");

    // `focus_ended`, awaited at once and before any read of the view, answers
    // only once the virtual clock has moved by three exposures.
    let t0 = rig.sim.clock().now();
    let ended = rig.imaging.focus_ended().await.expect("the focus view");
    let moved = rig.sim.clock().now() - t0;
    assert!(moved >= 3.0, "the clock moved only {moved} s");
    let aid = ended.aid.as_ref().expect("the aid");
    assert_eq!(aid.state, "ended");
    assert_eq!(aid.verdict, Some("good"));
    assert!(aid.check);

    let (_, view) = get(&rig.app, "/api/imaging").await;
    let aid = &view["focus"]["aid"];
    assert_eq!(aid["state"], json!("ended"), "{view}");
    assert_eq!(aid["ended_reason"], json!("done"), "{aid}");
    assert_eq!(aid["check"], json!(true), "{aid}");
    assert_eq!(aid["readings"], json!(3), "{aid}");
    assert!(
        (aid["median"]["value"].as_f64().unwrap() - 7.011170).abs() < 1e-5,
        "{aid}"
    );
    assert_eq!(aid["median"]["readings"], json!(3), "{aid}");
    assert_eq!(aid["verdict"], json!("good"), "{aid}");
    assert!(
        (aid["reference"]["value"].as_f64().unwrap() - 6.572972).abs() < 1e-5,
        "{aid}"
    );
    assert_eq!(aid["reference"]["of"], json!("tonight"), "{aid}");
    assert_eq!(
        view["settings"]["roi"],
        json!({"x": 0, "y": 0, "width": 1236, "height": 822}),
        "{view}"
    );
    assert_eq!(rig.imaging.holder(), None);

    // The solver: three whole-frame reads of the kept middle tile, nothing
    // else.
    let calls = rig.solver.as_ref().expect("the solver").calls();
    assert_eq!(calls.len(), 3, "{calls:?}");
    for call in &calls {
        assert_eq!(
            *call,
            Call {
                roi: tile(1, 1),
                binning: 1,
                width: 412,
                height: 274,
                region: None,
            },
            "{call:?}"
        );
    }

    // The record is still the hand-written one, with no `passed`.
    let file = focus_file(&rig);
    assert_eq!(
        file["record"]["tonight"]["px"].as_f64(),
        Some(3.0),
        "{file}"
    );
    assert!(file["record"]["tonight"].get("passed").is_none(), "{file}");
}

/// US8-S1: the check's verdict against every shape of the record (oracle: the
/// worked example's ratios 1.143, 1.333 and 1.000, and the hand-written
/// files, compared exactly).
#[tokio::test(start_paused = true)]
async fn a_check_s_verdicts_against_every_shape_of_the_record() {
    let scratch = crate::support::Scratch::new("imaging-focus-check-verdicts");
    let readings = || vec![(3.20, 12), (3.25, 12), (3.15, 12)];
    let folder = |n: u32| scratch.0.join(format!("imaging-{n}"));

    // Tonight's record at 2.8: slightly soft at 1.143.
    let (_, view) = run_check(&folder(0), Some(&tonight(2.8)), readings()).await;
    let aid = &view["focus"]["aid"];
    assert_eq!(aid["verdict"], json!("slightly_soft"), "{aid}");
    assert_eq!(aid["readings"], json!(3), "{aid}");

    // At 2.4: clearly out at 1.333.
    let (_, view) = run_check(&folder(1), Some(&tonight(2.4)), readings()).await;
    assert_eq!(
        view["focus"]["aid"]["verdict"],
        json!("clearly_out"),
        "{view}"
    );

    // No record: no verdict's reference, and the file gains no record.
    let (rig, view) = run_check(&folder(2), None, readings()).await;
    let aid = &view["focus"]["aid"];
    assert_eq!(aid["verdict"], json!("no_record"), "{aid}");
    assert!(aid.get("reference").is_none(), "{aid}");
    let file = focus_file(&rig);
    assert!(file.get("record").is_none(), "{file}");

    // A record 13 hours old: tonight's part counts with the earlier nights',
    // and the earlier part judges.
    let (rig, view) = run_check(
        &folder(3),
        Some("{\"tonight\":{\"px\":3.0,\"at\":1768482000.0,\"since\":1768482000.0}}"),
        readings(),
    )
    .await;
    let aid = &view["focus"]["aid"];
    assert_eq!(aid["reference"]["of"], json!("earlier"), "{aid}");
    assert!(
        (aid["reference"]["value"].as_f64().unwrap() - 6.572972).abs() < 1e-5,
        "{aid}"
    );
    assert_eq!(aid["verdict"], json!("good"), "{aid}");
    let record = &view["focus"]["record"];
    assert!(record.get("tonight").is_none(), "{record}");
    assert!(
        (record["earlier"]["value"].as_f64().unwrap() - 6.572972).abs() < 1e-5,
        "{record}"
    );
    assert_eq!(rig.imaging.holder(), None);

    // Tonight's part not passed through: the smaller part judges, and 2.4
    // sensor pixels are 5.258377″.
    let two_parts = "{\"tonight\":{\"px\":3.2,\"at\":1768528000.0,\"since\":1768528000.0},\
                     \"earlier\":{\"px\":2.4,\"at\":1768400000.0}}";
    let (_, view) = run_check(&folder(4), Some(two_parts), readings()).await;
    let aid = &view["focus"]["aid"];
    assert_eq!(aid["reference"]["of"], json!("earlier"), "{aid}");
    assert!(
        (aid["reference"]["value"].as_f64().unwrap() - 5.258377).abs() < 1e-5,
        "{aid}"
    );
    assert_eq!(aid["verdict"], json!("clearly_out"), "{aid}");

    // Tonight's part marked passed: tonight's alone judges, at 1.000.
    let passed = "{\"tonight\":{\"px\":3.2,\"at\":1768528000.0,\"since\":1768528000.0,\
                  \"passed\":true},\"earlier\":{\"px\":2.4,\"at\":1768400000.0}}";
    let (_, view) = run_check(&folder(5), Some(passed), readings()).await;
    let aid = &view["focus"]["aid"];
    assert_eq!(aid["reference"]["of"], json!("tonight"), "{aid}");
    assert!(
        (aid["reference"]["value"].as_f64().unwrap() - 7.011170).abs() < 1e-5,
        "{aid}"
    );
    assert_eq!(aid["verdict"], json!("good"), "{aid}");
}

/// US8-S1: the check's endings on a frame that measures nothing — at the
/// first unmeasurable frame with its verdict, and after nine imaging frames
/// with neither, with `unmeasurable` and no median of three (oracle: the
/// stated readings, states and verdicts, compared exactly).
#[tokio::test(start_paused = true)]
async fn a_check_ends_at_its_first_unmeasurable_frame() {
    let scratch = crate::support::Scratch::new("imaging-focus-check-unmeasurable");
    let folder = |n: u32| scratch.0.join(format!("imaging-{n}"));

    // Too few stars: unmeasurable, at once.
    let (_, view) = run_check(&folder(0), Some(&tonight(3.0)), vec![(3.0, 2)]).await;
    let aid = &view["focus"]["aid"];
    assert_eq!(aid["ended_reason"], json!("done"), "{aid}");
    assert_eq!(aid["readings"], json!(1), "{aid}");
    assert_eq!(aid["verdict"], json!("unmeasurable"), "{aid}");
    assert_eq!(aid["last"]["state"], json!("too_few_stars"), "{aid}");

    // Far out: clearly out, at once.
    let (_, view) = run_check(&folder(1), Some(&tonight(3.0)), vec![(25.0, 9)]).await;
    let aid = &view["focus"]["aid"];
    assert_eq!(aid["ended_reason"], json!("done"), "{aid}");
    assert_eq!(aid["readings"], json!(1), "{aid}");
    assert_eq!(aid["verdict"], json!("clearly_out"), "{aid}");
    assert_eq!(aid["last"]["state"], json!("far_out"), "{aid}");

    // Nine frames that settle nothing, each one a change: `unmeasurable`,
    // with no median of three readings.
    let alternating = || {
        (0..9)
            .map(|n| (if n % 2 == 0 { 3.0 } else { 4.0 }, 12))
            .collect()
    };
    let (_, view) = run_check(&folder(2), Some(&tonight(3.0)), alternating()).await;
    let aid = &view["focus"]["aid"];
    assert_eq!(aid["readings"], json!(9), "{aid}");
    assert_eq!(aid["ended_reason"], json!("done"), "{aid}");
    assert_eq!(aid["verdict"], json!("unmeasurable"), "{aid}");
    assert!(aid.get("median").is_none(), "{aid}");
}

/// US8-S1: a check on a rig with no `focus.json`, where every tile of the
/// search is far out, reads the first of the tied far-out tiles for the time
/// being and ends at its first unmeasurable frame, keeping no region and
/// writing no file (oracle: the stated calls and regions, compared exactly).
#[tokio::test(start_paused = true)]
async fn a_check_on_far_out_tiles_reads_one_tile_for_the_time_being() {
    let scratch = crate::support::Scratch::new("imaging-focus-check-far-out");
    let mut entries = vec![(25.0, 12); 9];
    entries.push((25.0, 9));
    let rig = focus_rig(&scratch.0.join("imaging"), entries);
    wait_ready(&rig.app).await;
    let (status, view) = post_focus(&rig.app, json!({"check": true})).await;
    assert_eq!(status, StatusCode::OK, "{view}");
    let view = wait_until(&rig.app, |view| {
        view["focus"]["aid"]["state"] == json!("ended")
    })
    .await;
    let aid = &view["focus"]["aid"];
    assert_eq!(aid["ended_reason"], json!("done"), "{aid}");
    assert_eq!(aid["verdict"], json!("clearly_out"), "{aid}");
    assert_eq!(aid["readings"], json!(1), "{aid}");
    assert_eq!(aid["last"]["state"], json!("far_out"), "{aid}");

    // The solver: nine tile measures of a whole-sensor frame, then one frame
    // of the first of the tied far-out tiles, the top left one.
    let calls = rig.solver.as_ref().expect("the solver").calls();
    assert_eq!(calls.len(), 10, "{calls:?}");
    let whole = Region {
        x: 0,
        y: 0,
        width: 1236,
        height: 822,
    };
    let row_major: Vec<Option<Region>> = (0..3)
        .flat_map(|row| (0..3).map(move |col| Some(tile(col, row))))
        .collect();
    for (n, call) in calls.iter().enumerate() {
        let (roi, region) = match n {
            0..=8 => (whole, row_major[n]),
            _ => (tile(0, 0), None),
        };
        assert_eq!(call.roi, roi, "{n}: {call:?}");
        assert_eq!(call.region, region, "{n}: {call:?}");
        assert_eq!(call.width, roi.width, "{n}: {call:?}");
        assert_eq!(call.height, roi.height, "{n}: {call:?}");
        assert_eq!(call.binning, 1, "{n}: {call:?}");
    }

    // Nothing was kept and nothing written.
    assert!(view["focus"].get("region").is_none(), "{view}");
    assert!(!root(&rig).join("focus.json").exists());
}

/// US8-S2: the simulated focuser turned from 3.0 through focus and back: the
/// aid's median follows the true size, the trend goes better, worse and
/// better, `found` turns true only once the reading is back near the best,
/// and only then is tonight's record marked `passed` (oracle: the simulator's
/// truth, H(0) × the rig's scale = 6.191266″, within 3% against 2% of seeded
/// noise).
#[tokio::test(start_paused = true)]
async fn the_focuser_turned_through_focus_and_back() {
    let scratch = crate::support::Scratch::new("imaging-focus-check-pass");
    let rig = noise_rig(&scratch.0.join("imaging"));
    wait_ready(&rig.app).await;

    // The guard: the middle tile holds at least eight stars for the whole
    // scenario (the plan's trap 8).
    let guard = SimulatedSolver::new(rig.sim.clone());
    let measure = guard
        .measure(
            SolveFrame {
                path: root(&rig).join("none.fits"),
                width: 412,
                height: 274,
                bits: 16,
                bayer: None,
                binning: 1,
                roi: tile(1, 1),
                start: support::START,
                exposure_s: 1.0,
            },
            None,
        )
        .await
        .expect("the tile measures");
    assert!(measure.stars >= 8, "{} stars", measure.stars);

    let (status, view) = post_focus(&rig.app, json!({"tile": {"col": 1, "row": 1}})).await;
    assert_eq!(status, StatusCode::OK, "{view}");
    let mut readings = 0;
    let steps = [3.0, 2.0, 1.0, 0.0, -1.0, -2.0, -1.0, 0.0];
    for (n, error) in steps.iter().enumerate() {
        rig.sim.set_focus_error(*error);
        readings += 6;
        let view = wait_until(&rig.app, |view| {
            view["focus"]["aid"]["readings"].as_u64() == Some(readings)
        })
        .await;
        let aid = &view["focus"]["aid"];
        match n {
            // After the fourth step, the first 0.0: the true size at best
            // focus, sharper than before, found still false, nothing passed.
            3 => {
                let value = aid["median"]["value"].as_f64().unwrap();
                assert!((value - 6.191266).abs() <= 0.03 * 6.191266, "{aid}");
                assert_eq!(aid["trend"], json!("better"), "{aid}");
                assert_eq!(aid["found"], json!(false), "{aid}");
                assert_eq!(aid["check"], json!(false), "{aid}");
                let file = focus_file(&rig);
                assert!(file["record"]["tonight"].get("passed").is_none(), "{file}");
            }
            // After the sixth step, the first −2.0: softer, and found still
            // false.
            5 => {
                assert_eq!(aid["trend"], json!("worse"), "{aid}");
                assert_eq!(aid["found"], json!(false), "{aid}");
            }
            // After the last step, back at 0.0: found, the best and
            // tonight's record at the true size, and the record passed.
            7 => {
                assert_eq!(aid["found"], json!(true), "{aid}");
                assert_eq!(aid["trend"], json!("better"), "{aid}");
                let best = aid["best"].as_f64().unwrap();
                assert!((best - 6.191266).abs() <= 0.03 * 6.191266, "{aid}");
                assert_eq!(aid["verdict"], json!("good"), "{aid}");
                let tonight = view["focus"]["record"]["tonight"]["value"]
                    .as_f64()
                    .unwrap();
                assert!((tonight - 6.191266).abs() <= 0.03 * 6.191266, "{view}");
                let file = focus_file(&rig);
                assert_eq!(file["record"]["tonight"]["passed"], json!(true), "{file}");
            }
            _ => {}
        }
    }
}
