//! The tile read for the time being, and a frame kept for a solve (033 US9):
//! with every tile of the search far out, the aid runs on the far-out tile
//! with the smallest size, keeps no region, searches again at its first
//! settled reading and keeps the tile that search finds; a second search with
//! no best goes on with the tile it had; and a frame an imaging solve is
//! running on when its turn to be removed comes stays in place with the solve
//! sidecar that solve writes, while one whose solve has ended by then goes
//! with its sidecar. Oracles: a worked example (22.0, 21.0, 12.0 and 11.0
//! sensor pixels, times the rig's scale 206264.806 × 0.01912 / 1800 =
//! 2.1909906″ a sensor pixel, within 1e-5), and the stated regions, calls,
//! ids, times and files, compared exactly.

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
    imaging::solver::{
        ImagingSolver, MeasureFuture, Region, SolveError, SolveFrame, SolveFuture, SolveHint,
        StarMeasure,
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

/// The scripted solver's measure delay, which gives a test a moment in which
/// the aid's latest frame is `saved` (US9-S2).
const MEASURE: Duration = Duration::from_millis(300);

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

/// The scripted solver (the shared setup): `measure` keeps a note of each call
/// and answers the next entry of its list after [`MEASURE`], repeating the
/// last once the list is used up; `solve` answers `Failed` after its own
/// delay, which decides whether it is still running when the frame's turn to
/// be removed comes (US9-S2).
struct Scripted {
    /// A star measure of so many frame pixels and stars, one per call.
    entries: Vec<(f64, u32)>,
    next: AtomicUsize,
    calls: std::sync::Mutex<Vec<Call>>,
    solve: Duration,
}

impl Scripted {
    fn new(entries: Vec<(f64, u32)>, solve: Duration) -> Self {
        Self {
            entries,
            next: AtomicUsize::new(0),
            calls: std::sync::Mutex::new(Vec::new()),
            solve,
        }
    }

    /// The `measure` calls the solver was given, in order.
    fn calls(&self) -> Vec<Call> {
        self.calls.lock().unwrap().clone()
    }

    fn outcome(&self) -> Result<StarMeasure, SolveError> {
        let n = self.next.fetch_add(1, Ordering::SeqCst);
        let last = self.entries.len() - 1;
        let (px, stars) = self.entries[n.min(last)];
        Ok(StarMeasure {
            half_flux_px: px,
            stars,
        })
    }
}

impl ImagingSolver for Scripted {
    fn name(&self) -> &'static str {
        "scripted"
    }

    fn solve(&self, _frame: SolveFrame, _hint: SolveHint) -> SolveFuture<'_> {
        let solve = self.solve;
        Box::pin(async move {
            tokio::time::sleep(solve).await;
            Err(SolveError::Failed(
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
        let answer = self.outcome();
        Box::pin(async move {
            tokio::time::sleep(MEASURE).await;
            answer
        })
    }
}

/// The focus rig (the shared setup): the simulator's database at 40° N 75° W
/// on the virtual clock from Unix 1768528800, pointed at the celestial pole,
/// where the imaging field moves by under a pixel in three seconds; a
/// `SimNode` over it with the simulated imaging camera, served over a fresh
/// duplex for each connection; and a console over that with imaging, the
/// scripted solver and `with_focal_length_mm(1800.0)`, so every size reads in
/// arcseconds. No mount is connected.
struct Rig {
    app: Router,
    /// The imaging folder R.
    root: PathBuf,
    solver: Option<Arc<Scripted>>,
}

fn focus_rig(root: &Path, entries: Vec<(f64, u32)>, solve: Duration) -> Rig {
    let sim = Simulator::new(
        support::DB.clone(),
        Site::default(),
        VirtualClock::new(support::START),
    );
    sim.push_to(40.0, 0.0);
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
    let solver = Arc::new(Scripted::new(entries, solve));
    let state = AppState::shared(support::DB.clone(), Samples::default())
        .with_session(session)
        .with_clock(Arc::new(support::ConsoleClock(sim.clock().clone())))
        .with_link(transport)
        .with_live()
        .with_imaging(root)
        .with_imaging_solver(solver.clone())
        .with_focal_length_mm(1800.0);
    Rig {
        app: finder::router(state),
        root: root.to_path_buf(),
        solver: Some(solver),
    }
}

/// The rig's imaging folder R.
fn root(rig: &Rig) -> &Path {
    &rig.root
}

async fn get(app: &Router, path: &str) -> (StatusCode, Value) {
    support::request(app, "GET", path, json!(null)).await
}

async fn post_focus(app: &Router, body: Value) -> (StatusCode, Value) {
    support::request(app, "POST", "/api/imaging/focus", body).await
}

async fn post_solve(app: &Router) -> (StatusCode, Value) {
    support::request(app, "POST", "/api/imaging/solve", json!(null)).await
}

async fn delete_focus(app: &Router) -> (StatusCode, Value) {
    support::request(app, "DELETE", "/api/imaging/focus", json!(null)).await
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

/// The names of the files of the frames folder ending in one of `ends`.
fn frame_files(rig: &Rig, ends: &str) -> Vec<String> {
    let folder = root(rig).join("frames/2026-01-16");
    let mut found = Vec::new();
    for entry in std::fs::read_dir(&folder).expect("the frames folder") {
        let name = entry.unwrap().file_name().to_string_lossy().into_owned();
        if name.ends_with(ends) {
            found.push(name);
        }
    }
    found.sort();
    found
}

/// `focus.json` under R, parsed.
fn focus_file(rig: &Rig) -> Value {
    serde_json::from_slice(
        &std::fs::read(root(rig).join("focus.json")).expect("focus.json under R"),
    )
    .unwrap()
}

/// The search whose every tile is far out, the centre one the smallest
/// (US9-S1): nine entries, each `(25.0, 12)` but the fifth.
fn far_out_search() -> Vec<(f64, u32)> {
    (0..9)
        .map(|n| (if n == 4 { 22.0 } else { 25.0 }, 12))
        .collect()
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

/// US9-S1: with every tile of the search far out, a start runs on the far-out
/// tile with the smallest size, shown as the provisional region, keeps no
/// region, shows the rough size, gives a measurable frame no verdict until the
/// readings settle, searches again at the first settled reading and keeps the
/// tile that search finds (oracle: the worked example and the stated regions
/// and calls, compared exactly).
#[tokio::test(start_paused = true)]
async fn the_tile_read_for_the_time_being_until_the_region_is_found() {
    let scratch = crate::support::Scratch::new("imaging-focus-far-out-run");
    let mut entries = far_out_search();
    // The tile read for the time being: two far-out frames, then three
    // measurable ones, of which the last settles and starts the search again.
    for px in [22.0, 21.0, 12.0, 12.0, 12.0] {
        entries.push((px, 12));
    }
    // The search the first settled reading starts: every tile measurable, the
    // third — the top right one — the smallest.
    for n in 0..9 {
        entries.push((if n == 2 { 11.0 } else { 12.5 }, 12));
    }
    entries.push((11.0, 12));
    let rig = focus_rig(&scratch.0.join("imaging"), entries, Duration::from_secs(1));
    wait_ready(&rig.app).await;
    let (status, view) = post_focus(&rig.app, json!({})).await;
    assert_eq!(status, StatusCode::OK, "{view}");

    // The first view, at one reading: the far-out tile read for the time
    // being, its rough size, no median and nothing kept.
    let first = wait_until(&rig.app, |view| {
        view["focus"]["aid"]["readings"] == json!(1)
    })
    .await;
    let aid = &first["focus"]["aid"];
    assert_eq!(aid["state"], json!("running"), "{first}");
    let region = &first["focus"]["region"];
    assert_eq!(
        (
            region["col"].clone(),
            region["row"].clone(),
            region["x"].clone(),
            region["y"].clone(),
            region["width"].clone(),
            region["height"].clone(),
            region["source"].clone(),
        ),
        (
            json!(1),
            json!(1),
            json!(412),
            json!(274),
            json!(412),
            json!(274),
            json!("provisional"),
        ),
        "{first}"
    );
    assert_eq!(
        first["settings"]["roi"],
        json!({"x": 412, "y": 274, "width": 412, "height": 274}),
        "{first}"
    );
    assert_eq!(aid["last"]["state"], json!("far_out"), "{aid}");
    assert!(
        (aid["last"]["value"].as_f64().unwrap() - 48.201793).abs() < 1e-5,
        "{aid}"
    );
    assert_eq!(aid["verdict"], json!("clearly_out"), "{aid}");
    assert!(aid.get("median").is_none(), "{aid}");
    assert!(!root(&rig).join("focus.json").exists());

    // The view at three readings: that frame could be measured and the
    // readings held are one, so the `clearly_out` of the two far-out frames
    // before it is not kept.
    let third = wait_until(&rig.app, |view| {
        view["focus"]["aid"]["readings"] == json!(3)
    })
    .await;
    let aid = &third["focus"]["aid"];
    assert_eq!(aid["last"]["state"], json!("reading"), "{aid}");
    assert!(aid.get("verdict").is_none(), "{aid}");
    assert!(aid.get("reference").is_none(), "{aid}");

    // The search the first settled reading starts keeps the top right tile.
    let found = wait_until(&rig.app, |view| {
        view["focus"]["region"]["source"] == json!("found")
    })
    .await;
    assert_eq!(found["focus"]["region"]["col"], json!(2), "{found}");
    assert_eq!(found["focus"]["region"]["row"], json!(0), "{found}");

    // The view before the stop: the history the run has drawn so far.
    let before = wait_until(&rig.app, |view| {
        view["focus"]["aid"]["readings"]
            .as_u64()
            .is_some_and(|n| n >= 8)
    })
    .await;
    let history = before["focus"]["aid"]["history"].as_array().unwrap();
    for (value, expected) in history.iter().zip([
        48.201793, 46.010803, 26.291887, 26.291887, 26.291887, 24.100897,
    ]) {
        assert!(
            (value.as_f64().unwrap() - expected).abs() < 1e-5,
            "{history:?}"
        );
    }

    let (status, answer) = delete_focus(&rig.app).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    assert_eq!(
        answer["focus"]["aid"]["ended_reason"],
        json!("stopped"),
        "{answer}"
    );
    assert_eq!(
        answer["focus"]["region"]["source"],
        json!("found"),
        "{answer}"
    );

    // The solver's calls, in order: nine of the whole sensor, five of the
    // centre tile, nine of the whole sensor again, then the top right tile.
    let calls = rig.solver.as_ref().expect("the solver").calls();
    assert!(calls.len() >= 9 + 5 + 9 + 3, "{} calls", calls.len());
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
            9..=13 => (tile(1, 1), None),
            14..=22 => (whole, row_major[n - 14]),
            _ => (tile(2, 0), None),
        };
        assert_eq!(call.roi, roi, "{n}: {call:?}");
        assert_eq!(call.region, region, "{n}: {call:?}");
        assert_eq!(call.width, roi.width, "{n}: {call:?}");
        assert_eq!(call.height, roi.height, "{n}: {call:?}");
        assert_eq!(call.binning, 1, "{n}: {call:?}");
    }

    // `focus.json`: the found region and tonight's record, both at 11.0
    // sensor pixels.
    let file = focus_file(&rig);
    assert_eq!(file["region"]["col"], json!(2), "{file}");
    assert_eq!(file["region"]["row"], json!(0), "{file}");
    assert_eq!(file["region"]["source"], json!("found"), "{file}");
    assert_eq!(file["region"]["px"], json!(11.0), "{file}");
    assert_eq!(file["record"]["tonight"]["px"], json!(11.0), "{file}");
}

/// US9-S1: a second search that finds no best tile, whether or not it finds a
/// far-out one, goes on with the tile it had instead of ending `no_region`,
/// and the tile read for the time being is shown only while the aid runs and
/// never kept (oracle: the rules of FR-037 and FR-029, and the stated calls).
#[tokio::test(start_paused = true)]
async fn a_second_search_with_no_best_keeps_the_tile_it_had() {
    let scratch = crate::support::Scratch::new("imaging-focus-far-out-again");
    let mut entries = far_out_search();
    // Three measurable frames of the centre tile, of which the last settles;
    // then a search whose every tile is far out again.
    for _ in 0..3 {
        entries.push((12.0, 12));
    }
    for _ in 0..9 {
        entries.push((25.0, 12));
    }
    entries.push((9.0, 12));
    let rig = focus_rig(&scratch.0.join("imaging"), entries, Duration::from_secs(1));
    wait_ready(&rig.app).await;
    let (status, view) = post_focus(&rig.app, json!({})).await;
    assert_eq!(status, StatusCode::OK, "{view}");

    let view = wait_until(&rig.app, |view| {
        view["focus"]["aid"]["readings"] == json!(5)
    })
    .await;
    let aid = &view["focus"]["aid"];
    assert_eq!(aid["state"], json!("running"), "{view}");
    assert!(aid.get("ended_reason").is_none(), "{aid}");
    let region = &view["focus"]["region"];
    assert_eq!(
        (
            region["col"].clone(),
            region["row"].clone(),
            region["source"].clone()
        ),
        (json!(1), json!(1), json!("provisional")),
        "{view}"
    );

    // The solver's calls: nine of the whole sensor, three of the centre tile,
    // nine of the whole sensor again, then two more of the centre tile.
    let calls = rig.solver.as_ref().expect("the solver").calls();
    assert_eq!(calls.len(), 9 + 3 + 9 + 2, "{calls:?}");
    let whole = Region {
        x: 0,
        y: 0,
        width: 1236,
        height: 822,
    };
    for (n, call) in calls.iter().enumerate() {
        let expected = match n {
            0..=8 => whole,
            9..=11 => tile(1, 1),
            12..=20 => whole,
            _ => tile(1, 1),
        };
        assert_eq!(call.roi, expected, "{n}: {call:?}");
    }

    // The stop: nothing was kept, and no tile shows once the run has ended.
    let (status, answer) = delete_focus(&rig.app).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    assert_eq!(
        answer["focus"]["aid"]["ended_reason"],
        json!("stopped"),
        "{answer}"
    );
    assert!(answer["focus"].get("region").is_none(), "{answer}");
    let file = focus_file(&rig);
    assert!(file.get("region").is_none(), "{file}");
}

/// US9-S2: a frame an imaging solve is running on when its turn to be removed
/// comes stays in place for good with the solve sidecar that solve writes,
/// while one whose solve has ended by then goes with its sidecar; and the
/// aid's last sample carries its frame's id and the middle of its exposure
/// (oracle: the stated rules of FR-039 and FR-024, and the stated times and
/// files, compared exactly).
#[tokio::test(start_paused = true)]
async fn a_frame_under_a_solve_keeps_its_file_and_its_sidecar() {
    let scratch = crate::support::Scratch::new("imaging-focus-far-out-solve");
    let ask = || json!({"tile": {"col": 1, "row": 1}});
    let entries = || vec![(3.0, 12)];

    // The solve outlasts the aid's next frame, so the frame is still under it
    // when its turn comes.
    let rig = focus_rig(&scratch.0.join("a"), entries(), Duration::from_millis(2500));
    wait_ready(&rig.app).await;
    let (status, _) = post_focus(&rig.app, ask()).await;
    assert_eq!(status, StatusCode::OK);
    let saved = wait_until(&rig.app, |view| {
        view["frame"]["purpose"] == json!("focus") && view["frame"]["state"] == json!("saved")
    })
    .await;
    let id = saved["frame"]["id"].as_u64().unwrap();
    let at = saved["frame"]["start"].as_f64().unwrap()
        + saved["frame"]["exposure_ms"].as_f64().unwrap() / 2000.0;

    // The solve is asked for at once, while the aid measures that frame.
    let (status, answer) = post_solve(&rig.app).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    assert_eq!(answer["solve"]["state"], json!("solving"), "{answer}");
    assert_eq!(answer["solve"]["frame_id"], json!(id), "{answer}");

    // The aid's sample of that frame: the middle of its exposure.
    let view = wait_until(&rig.app, |view| {
        view["focus"]["aid"]["last"]["frame_id"] == json!(id)
    })
    .await;
    let last = &view["focus"]["aid"]["last"];
    assert!((last["at"].as_f64().unwrap() - at).abs() < 1e-6, "{last}");

    // The solve fails, and both its sidecar and the frame are still there,
    // although the aid has saved imaging frames since.
    let failed = wait_until(&rig.app, |view| view["solve"]["state"] == json!("failed")).await;
    let sidecar = failed["solve"]["file"].as_str().unwrap();
    let fits = sidecar.replace(".solve.json", ".fits");
    assert!(root(&rig).join(sidecar).exists(), "{sidecar}");
    assert!(root(&rig).join(&fits).exists(), "{fits}");

    // Two readings later the aid stops: the frame under the solve and the
    // last frame are the only files, with the one sidecar.
    let readings = failed["focus"]["aid"]["readings"].as_u64().unwrap();
    wait_until(&rig.app, |view| {
        view["focus"]["aid"]["readings"]
            .as_u64()
            .is_some_and(|n| n >= readings + 2)
    })
    .await;
    let (status, answer) = delete_focus(&rig.app).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let kept = frame_files(&rig, ".fits");
    assert_eq!(kept.len(), 2, "{kept:?}");
    assert!(kept.contains(
        &Path::new(&fits)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned()
    ));
    assert_eq!(
        frame_files(&rig, ".solve.json"),
        vec![Path::new(sidecar)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned()],
        "{answer}"
    );
    assert!(frame_files(&rig, ".part").is_empty());

    // The solve ends before the next frame is saved, so the frame and its
    // sidecar go together.
    let rig = focus_rig(&scratch.0.join("b"), entries(), Duration::from_millis(100));
    wait_ready(&rig.app).await;
    let (status, _) = post_focus(&rig.app, ask()).await;
    assert_eq!(status, StatusCode::OK);
    wait_until(&rig.app, |view| {
        view["frame"]["purpose"] == json!("focus") && view["frame"]["state"] == json!("saved")
    })
    .await;
    let (status, answer) = post_solve(&rig.app).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    assert_eq!(answer["solve"]["state"], json!("solving"), "{answer}");
    let failed = wait_until(&rig.app, |view| view["solve"]["state"] == json!("failed")).await;
    let readings = failed["focus"]["aid"]["readings"].as_u64().unwrap();
    wait_until(&rig.app, |view| {
        view["focus"]["aid"]["readings"]
            .as_u64()
            .is_some_and(|n| n >= readings + 2)
    })
    .await;
    let (status, answer) = delete_focus(&rig.app).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    assert_eq!(frame_files(&rig, ".fits").len(), 1, "{answer}");
    assert!(frame_files(&rig, ".solve.json").is_empty(), "{answer}");
    assert!(frame_files(&rig, ".part").is_empty(), "{answer}");
}
