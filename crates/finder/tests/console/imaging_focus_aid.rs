//! The focus aid on the console (033 US5): a start that finds the region from
//! nine tiles of a whole-sensor frame and exposes that tile at the smallest
//! binning one frame after another, the focus object's shapes in the imaging
//! view and in the session and every refusal, a tile fixed by hand and found
//! again, the lease held against a frame, a settings change and a second
//! taker, and the endings. Oracles: a worked example (2.60, 2.62 and 2.58
//! sensor pixels times the rig's scale 206264.806 × 0.01912 / 1800 =
//! 2.1909906″ a sensor pixel; the fourth and fifth settled readings 2.60
//! again, judged against the 2.60 noted before them), the stated settings,
//! regions, words and statuses, compared exactly, and 031 US9's rules for the
//! lease.

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
        solver::{
            ImagingSolver, MeasureFuture, Region, SolveError, SolveFrame, SolveFuture, SolveHint,
            StarMeasure,
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
use tower::ServiceExt;

use crate::support::live as support;

/// One entry of the scripted solver's list (the shared setup): a star measure
/// of so many frame pixels and stars, or the error it answers.
#[derive(Clone)]
enum Entry {
    Stars(f64, u32),
    Error(SolveError),
}

/// What one `measure` call was given (the shared setup): the frame's region of
/// interest, binning and size, and the region asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Call {
    roi: Region,
    binning: u32,
    width: u32,
    height: u32,
    region: Option<Region>,
}

/// The scripted solver (the shared setup): `measure` keeps a note of each call
/// and answers the next entry of its list at once, repeating the last once the
/// list is used up; `solve` answers `Failed("not used")`.
struct Scripted {
    entries: Vec<Entry>,
    next: AtomicUsize,
    calls: std::sync::Mutex<Vec<Call>>,
}

impl Scripted {
    fn new(entries: Vec<Entry>) -> Self {
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

    fn outcome(&self) -> Result<StarMeasure, SolveError> {
        let n = self.next.fetch_add(1, Ordering::SeqCst);
        let last = self.entries.len() - 1;
        match &self.entries[n.min(last)] {
            Entry::Stars(px, stars) => Ok(StarMeasure {
                half_flux_px: *px,
                stars: *stars,
            }),
            Entry::Error(error) => Err(error.clone()),
        }
    }
}

impl ImagingSolver for Scripted {
    fn name(&self) -> &'static str {
        "scripted"
    }

    fn solve(&self, _frame: SolveFrame, _hint: SolveHint) -> SolveFuture<'_> {
        Box::pin(async { Err(SolveError::Failed("not used".to_string())) })
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
        Box::pin(async move { answer })
    }
}

/// The same star measure n times over.
fn stars(px: f64, n: u32) -> Vec<Entry> {
    vec![Entry::Stars(px, n); 1]
}

/// The focus rig (the shared setup): the simulator's database at 40° N 75° W
/// on the virtual clock from Unix 1768528800, pointed at the celestial pole,
/// where the imaging field moves by under a pixel in three seconds; a
/// `SimNode` over it with the simulated imaging camera, served over a fresh
/// duplex for each connection; and a console over that with imaging, a solver
/// and `with_focal_length_mm(1800.0)` — `solver`, `root` and `focal` name the
/// parts a scenario leaves out. No mount is connected.
struct Rig {
    app: Router,
    sim: Simulator,
    imaging: Option<Arc<Imaging>>,
    root: Option<PathBuf>,
    solver: Option<Arc<Scripted>>,
}

fn console(
    sim: &Simulator,
    root: Option<&Path>,
    solver: Option<Arc<Scripted>>,
    focal: bool,
) -> Rig {
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
    let mut state = AppState::shared(support::DB.clone(), Samples::default())
        .with_session(session)
        .with_clock(Arc::new(support::ConsoleClock(sim.clock().clone())))
        .with_link(transport)
        .with_live();
    let mut imaging = None;
    if let Some(root) = root {
        state = state.with_imaging(root);
        imaging = state.imaging();
    }
    if let Some(solver) = solver {
        state = state.with_imaging_solver(solver);
    }
    if focal {
        state = state.with_focal_length_mm(1800.0);
    }
    Rig {
        app: finder::router(state),
        sim: sim.clone(),
        imaging,
        root: root.map(Path::to_path_buf),
        solver: None,
    }
}

/// The focus rig with a scripted solver, built as the scenario's entries name.
fn focus_rig(root: &Path, entries: Vec<Entry>) -> Rig {
    let sim = Simulator::new(
        support::DB.clone(),
        Site::default(),
        VirtualClock::new(support::START),
    );
    sim.push_to(40.0, 0.0);
    let solver = Arc::new(Scripted::new(entries));
    let mut rig = console(&sim, Some(root), Some(solver.clone()), true);
    rig.solver = Some(solver);
    rig
}

/// The rig's imaging camera.
fn imaging(rig: &Rig) -> Arc<Imaging> {
    rig.imaging.clone().expect("the rig takes imaging frames")
}

/// The rig's imaging folder R.
fn root(rig: &Rig) -> &Path {
    rig.root.as_deref().expect("the rig takes imaging frames")
}

async fn get(app: &Router, path: &str) -> (StatusCode, Value) {
    support::request(app, "GET", path, json!(null)).await
}

async fn put(app: &Router, body: Value) -> (StatusCode, Value) {
    support::request(app, "PUT", "/api/imaging/settings", body).await
}

async fn post_frame(app: &Router) -> (StatusCode, Value) {
    support::request(app, "POST", "/api/imaging/frame", json!(null)).await
}

async fn post_focus(app: &Router, body: Value) -> (StatusCode, Value) {
    support::request(app, "POST", "/api/imaging/focus", body).await
}

async fn delete_focus(app: &Router) -> (StatusCode, Value) {
    support::request(app, "DELETE", "/api/imaging/focus", json!(null)).await
}

async fn post_abort(app: &Router) -> (StatusCode, Value) {
    support::request(app, "POST", "/api/imaging/abort", json!(null)).await
}

/// One request with a foreign `Origin`, as a browser on another site sends
/// it, with a `Host` of localhost on port 8080.
async fn foreign(app: &Router, method: &str, path: &str) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("host", "localhost:8080")
                .header("content-type", "application/json")
                .header("origin", "http://elsewhere.example")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    (status, value)
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

/// US5-S1: a start with no kept region finds it from nine tiles of a
/// whole-sensor frame, keeps the sharpest, exposes that tile alone at binning 1
/// and 1000 ms, judges the first settled reading `no_record` before noting it
/// and the next ones `good` against it, removes every frame's file but the
/// last, and a stop puts the settings back (oracle: the worked example and the
/// stated settings).
#[tokio::test(start_paused = true)]
async fn a_start_finds_the_region_runs_and_is_stopped() {
    let scratch = crate::support::Scratch::new("imaging-focus-aid-run");
    let entries = || {
        let mut entries = vec![
            Entry::Stars(4.0, 12),
            Entry::Stars(3.5, 12),
            Entry::Stars(2.6, 12),
            Entry::Stars(3.6, 12),
            Entry::Stars(2.9, 12),
            Entry::Stars(3.4, 12),
            Entry::Stars(4.1, 12),
            Entry::Stars(2.2, 3),
            Entry::Stars(4.4, 12),
        ];
        entries.push(Entry::Stars(2.60, 12));
        entries.push(Entry::Stars(2.62, 12));
        entries.push(Entry::Stars(2.58, 12));
        entries.push(Entry::Stars(2.60, 12));
        entries
    };
    let rig = focus_rig(&scratch.0.join("imaging"), entries());
    wait_ready(&rig.app).await;
    let (status, _) = put(&rig.app, json!({"exposure_ms":2000,"binning":2})).await;
    assert_eq!(status, StatusCode::OK);

    let (status, view) = post_focus(&rig.app, json!({})).await;
    assert_eq!(status, StatusCode::OK, "{view}");
    let state = view["focus"]["aid"]["state"].as_str().unwrap();
    assert!(
        ["starting", "finding", "running"].contains(&state),
        "{view}"
    );

    // The first view, at exactly three readings: the first settled reading.
    let first = wait_until(&rig.app, |view| {
        view["focus"]["aid"]["readings"] == json!(3)
    })
    .await;
    let focus = &first["focus"];
    // The aid's own four: the exposure, the smallest binning, the region and
    // no node copy; the camera's gain and offset are its own defaults.
    let settings = &first["settings"];
    assert_eq!(settings["exposure_ms"], json!(1000.0), "{first}");
    assert_eq!(settings["binning"], json!(1), "{first}");
    assert_eq!(
        settings["roi"],
        json!({"x": 824, "y": 0, "width": 412, "height": 274}),
        "{first}"
    );
    assert_eq!(settings["save_at_node"], json!(false), "{first}");
    assert_eq!(first["frame"]["purpose"], json!("focus"), "{first}");
    assert_eq!(focus["unit"], json!("arcsec"));
    assert!((focus["arcsec_per_px"].as_f64().unwrap() - 2.1909906).abs() < 1e-6);
    let region = &focus["region"];
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
            json!(2),
            json!(0),
            json!(824),
            json!(0),
            json!(412),
            json!(274),
            json!("found")
        ),
        "{first}"
    );
    let aid = &focus["aid"];
    assert_eq!(aid["state"], json!("running"));
    assert_eq!(aid["exposure_ms"], json!(1000.0));
    let median = &aid["median"];
    assert!(
        (median["value"].as_f64().unwrap() - 5.696576).abs() < 1e-5,
        "{median}"
    );
    assert_eq!(median["readings"], json!(3));
    assert!(
        (aid["best"].as_f64().unwrap() - 5.696576).abs() < 1e-5,
        "{aid}"
    );
    assert_eq!(aid["found"], json!(false));
    assert!(aid.get("trend").is_none(), "{aid}");
    assert_eq!(aid["last"]["state"], json!("reading"));
    assert_eq!(aid["last"]["stars"], json!(12));
    let history = aid["history"].as_array().unwrap();
    let history: Vec<f64> = history.iter().map(|v| v.as_f64().unwrap()).collect();
    for (value, expected) in history.iter().zip([5.696576, 5.740395, 5.652756]) {
        assert!((value - expected).abs() < 1e-5, "{aid}");
    }
    assert_eq!(history.len(), 3);
    assert!(aid["cadence_ms"].as_f64().unwrap() >= 1000.0, "{aid}");
    assert_eq!(aid["verdict"], json!("no_record"), "{aid}");
    assert!(aid.get("reference").is_none(), "{aid}");
    assert!(
        (focus["record"]["tonight"]["value"].as_f64().unwrap() - 5.696576).abs() < 1e-5,
        "{focus}"
    );

    // The second view: the next settled readings judged against the first.
    let second = wait_until(&rig.app, |view| {
        view["focus"]["aid"]["readings"]
            .as_u64()
            .is_some_and(|n| n >= 5)
    })
    .await;
    let aid = &second["focus"]["aid"];
    assert_eq!(aid["verdict"], json!("good"), "{aid}");
    assert!(
        (aid["reference"]["value"].as_f64().unwrap() - 5.696576).abs() < 1e-5,
        "{aid}"
    );
    assert_eq!(aid["reference"]["of"], json!("tonight"));
    assert!(
        (aid["median"]["value"].as_f64().unwrap() - 5.696576).abs() < 1e-5,
        "{aid}"
    );

    // The stop: the settings back, the lease free, one file left.
    let (status, answer) = delete_focus(&rig.app).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    assert_eq!(answer["focus"]["aid"]["state"], json!("ended"));
    assert_eq!(answer["focus"]["aid"]["ended_reason"], json!("stopped"));
    assert_eq!(answer["settings"]["exposure_ms"], json!(2000.0));
    assert_eq!(answer["settings"]["binning"], json!(2));
    assert_eq!(
        answer["settings"]["roi"],
        json!({"x": 0, "y": 0, "width": 1236, "height": 822}),
        "{answer}"
    );
    assert_eq!(imaging(&rig).holder(), None);
    assert_eq!(frame_files(&rig, ".fits").len(), 1, "the last frame only");
    assert!(frame_files(&rig, ".part").is_empty());

    // The solver's calls: nine tile measures of a whole frame at binning 1,
    // in row-major order, then whole-frame reads of the tile it chose.
    let calls = rig.solver.as_ref().expect("the solver").calls();
    assert_eq!(calls.len(), 9 + 5);
    let whole = Region {
        x: 0,
        y: 0,
        width: 1236,
        height: 822,
    };
    let tile = |col: u32, row: u32| Region {
        x: 412 * col,
        y: 274 * row,
        width: 412,
        height: 274,
    };
    let row_major: Vec<Option<Region>> = (0..3)
        .flat_map(|row| (0..3).map(move |col| Some(tile(col, row))))
        .collect();
    for (n, call) in calls.iter().take(9).enumerate() {
        assert_eq!(
            *call,
            Call {
                roi: whole,
                binning: 1,
                width: 1236,
                height: 822,
                region: row_major[n]
            },
            "{call:?}"
        );
    }
    for call in calls.iter().skip(9) {
        assert_eq!(
            *call,
            Call {
                roi: tile(2, 0),
                binning: 1,
                width: 412,
                height: 274,
                region: None,
            },
            "{call:?}"
        );
    }

    // `focus.json`: the found region and tonight's record, no `passed` and no
    // scale, because the rig's scale comes from the focal length.
    let file = focus_file(&rig);
    assert_eq!(file["region"]["col"], json!(2));
    assert_eq!(file["region"]["px"], json!(2.6));
    assert_eq!(file["region"]["stars"], json!(12));
    assert_eq!(file["record"]["tonight"]["px"], json!(2.6));
    assert!(file["record"]["tonight"].get("passed").is_none(), "{file}");
    assert!(file["record"].get("arcsec_per_px").is_none(), "{file}");
}

/// US5-S2: the focus object is absent without imaging or without a solver,
/// `{"unit":"px"}` without a scale, and the full shape with one, in the
/// imaging view and in the session alike (oracle: the stated keys, compared
/// exactly).
#[tokio::test(start_paused = true)]
async fn the_focus_object_in_both_views() {
    let scratch = crate::support::Scratch::new("imaging-focus-aid-shapes");
    let sim = Simulator::new(
        support::DB.clone(),
        Site::default(),
        VirtualClock::new(support::START),
    );
    sim.push_to(40.0, 0.0);

    // With the focal length: `unit` `arcsec` and the scale, nothing else.
    let rig = console(
        &sim,
        Some(&scratch.0.join("a")),
        Some(Arc::new(Scripted::new(stars(3.0, 12)))),
        true,
    );
    wait_ready(&rig.app).await;
    let (_, view) = get(&rig.app, "/api/imaging").await;
    let focus = &view["focus"];
    assert_eq!(focus["unit"], json!("arcsec"));
    assert!((focus["arcsec_per_px"].as_f64().unwrap() - 2.1909906).abs() < 1e-6);
    for key in ["aid", "region", "record"] {
        assert!(focus.get(key).is_none(), "{focus}");
    }
    let (_, session) = get(&rig.app, "/api/session").await;
    let session_focus = &session["imaging"]["focus"];
    assert_eq!(session_focus["unit"], json!("arcsec"));
    assert!((session_focus["arcsec_per_px"].as_f64().unwrap() - 2.1909906).abs() < 1e-6);
    assert!(session_focus.get("aid").is_none(), "{session_focus}");

    // Without the focal length: exactly `{"unit":"px"}`.
    let rig = console(
        &sim,
        Some(&scratch.0.join("b")),
        Some(Arc::new(Scripted::new(stars(3.0, 12)))),
        false,
    );
    wait_ready(&rig.app).await;
    let (_, view) = get(&rig.app, "/api/imaging").await;
    assert_eq!(view["focus"], json!({"unit": "px"}), "{view}");

    // Without a solver: no `focus` key at all, here or in the session.
    let rig = console(&sim, Some(&scratch.0.join("c")), None, true);
    wait_ready(&rig.app).await;
    let (_, view) = get(&rig.app, "/api/imaging").await;
    assert!(view.get("focus").is_none(), "{view}");
    let (_, session) = get(&rig.app, "/api/session").await;
    assert!(session["imaging"].get("focus").is_none(), "{session}");
}

/// US5-S2: every refusal of a start, each with its words, and a `DELETE` that
/// changes nothing with no aid running (oracle: the stated words and statuses,
/// compared exactly).
#[tokio::test(start_paused = true)]
async fn every_start_refusal() {
    let scratch = crate::support::Scratch::new("imaging-focus-aid-refusals");
    let sim = Simulator::new(
        support::DB.clone(),
        Site::default(),
        VirtualClock::new(support::START),
    );
    sim.push_to(40.0, 0.0);

    // Without imaging at all.
    let rig = console(
        &sim,
        None,
        Some(Arc::new(Scripted::new(stars(3.0, 12)))),
        true,
    );
    let (status, answer) = post_focus(&rig.app, json!({})).await;
    assert_eq!(status, StatusCode::CONFLICT, "{answer}");
    assert_eq!(
        answer["error"],
        json!("this console takes no imaging frames")
    );

    // Without a solver.
    let rig = console(&sim, Some(&scratch.0.join("a")), None, true);
    wait_ready(&rig.app).await;
    let (status, answer) = post_focus(&rig.app, json!({})).await;
    assert_eq!(status, StatusCode::CONFLICT, "{answer}");
    assert_eq!(answer["error"], json!("this console has no imaging solver"));

    // Off the link.
    let rig = focus_rig(&scratch.0.join("b"), stars(3.0, 12));
    wait_ready(&rig.app).await;
    rig.sim.set_unresponsive(true);
    wait_until(&rig.app, |view| view["state"] == "no_link").await;
    let (status, answer) = post_focus(&rig.app, json!({})).await;
    assert_eq!(status, StatusCode::CONFLICT, "{answer}");
    assert_eq!(answer["error"], json!("the scope node is not connected"));

    // A body that is not the aid's, and the three refusals of the body's own
    // numbers, and a foreign origin: none of them starts anything.
    let rig = focus_rig(&scratch.0.join("c"), stars(3.0, 12));
    wait_ready(&rig.app).await;
    let refused = |view: &Value| {
        assert!(view["focus"].get("aid").is_none(), "{view}");
    };
    let (status, answer) = post_focus(&rig.app, json!({"x": 1})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{answer}");
    refused(&answer);
    for (body, words) in [
        (
            json!({"find": true, "tile": {"col": 0, "row": 0}}),
            "give find or tile, not both",
        ),
        (
            json!({"tile": {"col": 3, "row": 0}}),
            "tile col and row must each be 0, 1 or 2",
        ),
        (
            json!({"exposure_ms": 0.5}),
            "exposure_ms 0.5 is outside the camera's range 1–3600000 ms",
        ),
    ] {
        let (status, answer) = post_focus(&rig.app, body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{answer}");
        assert_eq!(answer["error"], json!(words), "{answer}");
        refused(&answer);
    }
    let (status, answer) = foreign(&rig.app, "POST", "/api/imaging/focus").await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{answer}");

    // A `DELETE` with nothing running changes nothing.
    let (status, answer) = delete_focus(&rig.app).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    assert!(answer["focus"].get("aid").is_none(), "{answer}");

    // While an imaging frame of 10000 ms is exposing and nobody holds the
    // camera.
    let (status, _) = put(&rig.app, json!({"exposure_ms": 10000})).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = post_frame(&rig.app).await;
    assert_eq!(status, StatusCode::OK);
    let view = wait_until(&rig.app, |view| view["frame"]["state"] == json!("exposing")).await;
    assert_eq!(imaging(&rig).holder(), None, "{view}");
    let (status, answer) = post_focus(&rig.app, json!({})).await;
    assert_eq!(status, StatusCode::CONFLICT, "{answer}");
    assert_eq!(answer["error"], json!("an exposure is in progress"));
    refused(&answer);
    post_abort(&rig.app).await;

    // A second start, 1 s after a first.
    let rig = focus_rig(&scratch.0.join("d"), stars(3.0, 12));
    wait_ready(&rig.app).await;
    assert!(!imaging(&rig).focus_running());
    let (status, view) = post_focus(&rig.app, json!({"tile": {"col": 1, "row": 1}})).await;
    assert_eq!(status, StatusCode::OK, "{view}");
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert!(imaging(&rig).focus_running());
    let (status, answer) = post_focus(&rig.app, json!({})).await;
    assert_eq!(status, StatusCode::CONFLICT, "{answer}");
    assert_eq!(answer["error"], json!("the focus aid is already running"));
    // The session's `focus` says what the imaging view says.
    let (_, session) = get(&rig.app, "/api/session").await;
    assert_eq!(session["imaging"]["focus"]["unit"], json!("arcsec"));
    let (_, view) = get(&rig.app, "/api/imaging").await;
    assert_eq!(view["focus"]["aid"]["state"], json!("running"));
    assert_eq!(
        session["imaging"]["focus"]["aid"]["state"], view["focus"]["aid"]["state"],
        "{session}"
    );
    let (status, _) = delete_focus(&rig.app).await;
    assert_eq!(status, StatusCode::OK);
    assert!(!imaging(&rig).focus_running());
}

/// US5-S3: a tile fixed by hand needs no whole-sensor frame and is kept for the
/// next start; `find` measures the nine tiles again and keeps the first of
/// tied tiles (oracle: the stated regions).
#[tokio::test(start_paused = true)]
async fn a_tile_fixed_by_hand_and_found_again() {
    let scratch = crate::support::Scratch::new("imaging-focus-aid-fixed");
    let rig = focus_rig(&scratch.0.join("imaging"), stars(3.0, 12));
    let solver = rig.solver.clone().expect("the solver");
    wait_ready(&rig.app).await;
    let run = |body: Value| async {
        let (status, view) = post_focus(&rig.app, body).await;
        assert_eq!(status, StatusCode::OK, "{view}");
        wait_until(&rig.app, |view| {
            view["focus"]["aid"]["readings"]
                .as_u64()
                .is_some_and(|n| n >= 1)
        })
        .await
    };
    let stop = || async {
        let (status, _) = delete_focus(&rig.app).await;
        assert_eq!(status, StatusCode::OK);
    };
    let tile = |col: u32, row: u32| Region {
        x: 412 * col,
        y: 274 * row,
        width: 412,
        height: 274,
    };

    // Fixed by hand: no whole-sensor call, and the region kept as `fixed`.
    run(json!({"tile": {"col": 1, "row": 1}})).await;
    stop().await;
    for call in solver.calls() {
        assert_eq!(call.roi, tile(1, 1), "{call:?}");
    }
    let (_, view) = get(&rig.app, "/api/imaging").await;
    assert_eq!(view["focus"]["region"]["source"], json!("fixed"), "{view}");

    // Started with nothing: the kept region, so the same calls.
    let before = solver.calls().len();
    run(json!({})).await;
    stop().await;
    let calls = solver.calls();
    assert!(calls.len() > before, "the second start took no frames");
    for call in &calls[before..] {
        assert_eq!(call.roi, tile(1, 1), "{call:?}");
    }

    // `find`: nine measures of a whole-sensor frame, then whole-frame reads of
    // the first of the nine tied tiles, kept as `found`.
    let before = calls.len();
    run(json!({"find": true})).await;
    stop().await;
    let calls = solver.calls();
    let whole = Region {
        x: 0,
        y: 0,
        width: 1236,
        height: 822,
    };
    for call in &calls[before..before + 9] {
        assert_eq!(call.roi, whole, "{call:?}");
        assert_eq!(call.width, 1236);
        assert_eq!(call.height, 822);
    }
    for call in &calls[before + 9..] {
        assert_eq!(call.roi, tile(0, 0), "{call:?}");
        assert_eq!(call.region, None);
    }
    let (_, view) = get(&rig.app, "/api/imaging").await;
    assert_eq!(view["focus"]["region"]["source"], json!("found"), "{view}");
    assert_eq!(view["focus"]["region"]["col"], json!(0));
    assert_eq!(view["focus"]["region"]["row"], json!(0));
}

/// US5-S4: while the aid runs it holds the camera's lease as `the focus aid`,
/// so a frame, a settings change and a second lease are each refused naming
/// it and the aid runs on; the lease is free after a stop (oracle: the plan
/// document's amendment 1 and 031 US9's rules).
#[tokio::test(start_paused = true)]
async fn the_aid_holds_the_camera_while_it_runs() {
    let scratch = crate::support::Scratch::new("imaging-focus-aid-holds");
    let rig = focus_rig(&scratch.0.join("imaging"), stars(3.0, 12));
    let solver = rig.solver.clone().expect("the solver");
    wait_ready(&rig.app).await;
    let (status, _) = put(&rig.app, json!({"exposure_ms": 2000})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(imaging(&rig).holder(), None);

    let (status, view) = post_focus(&rig.app, json!({"tile": {"col": 1, "row": 1}})).await;
    assert_eq!(status, StatusCode::OK, "{view}");
    let first = wait_until(&rig.app, |view| {
        view["focus"]["aid"]["readings"]
            .as_u64()
            .is_some_and(|n| n >= 1)
    })
    .await;
    let readings = first["focus"]["aid"]["readings"].as_u64().unwrap();
    assert_eq!(imaging(&rig).holder(), Some("the focus aid"));
    let held_by = |answer: &Value| {
        answer["error"]
            .as_str()
            .expect("the refusal's words")
            .contains("the focus aid")
    };
    let (status, answer) = post_frame(&rig.app).await;
    assert_eq!(status, StatusCode::CONFLICT, "{answer}");
    assert!(held_by(&answer), "{answer}");
    let (status, answer) = put(&rig.app, json!({"roi": null})).await;
    assert_eq!(status, StatusCode::CONFLICT, "{answer}");
    assert!(held_by(&answer), "{answer}");
    let refusal = imaging(&rig).lease("a test").unwrap_err();
    assert_eq!(refusal.status, StatusCode::CONFLICT);
    assert!(refusal.message.contains("the focus aid"), "{refusal:?}");

    // Two readings later the aid is still running on the same region.
    wait_until(&rig.app, |view| {
        view["focus"]["aid"]["readings"]
            .as_u64()
            .is_some_and(|n| n >= readings + 2)
    })
    .await;
    let (_, view) = get(&rig.app, "/api/imaging").await;
    assert_eq!(view["focus"]["aid"]["state"], json!("running"), "{view}");
    assert_eq!(
        view["settings"]["roi"],
        json!({"x": 412, "y": 274, "width": 412, "height": 274}),
        "{view}"
    );
    for call in solver.calls() {
        assert_eq!(
            call.roi,
            Region {
                x: 412,
                y: 274,
                width: 412,
                height: 274
            },
            "{call:?}"
        );
    }

    let (status, _) = delete_focus(&rig.app).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(imaging(&rig).holder(), None);
    let (status, _) = post_frame(&rig.app).await;
    assert_eq!(status, StatusCode::OK);
}

/// US5-S4: a start while another holds the camera is refused naming the
/// holder and changes nothing, and answers 200 once the holder lets go
/// (oracle: 031 US9's rule 1).
#[tokio::test(start_paused = true)]
async fn a_start_while_another_holds_the_camera() {
    let scratch = crate::support::Scratch::new("imaging-focus-aid-refused");
    let rig = focus_rig(&scratch.0.join("imaging"), stars(3.0, 12));
    wait_ready(&rig.app).await;
    let (status, _) = put(&rig.app, json!({"exposure_ms": 2000})).await;
    assert_eq!(status, StatusCode::OK);
    let lease = imaging(&rig).lease("a test").expect("the lease");

    let (status, answer) = post_focus(&rig.app, json!({"tile": {"col": 1, "row": 1}})).await;
    assert_eq!(status, StatusCode::CONFLICT, "{answer}");
    assert!(
        answer["error"].as_str().unwrap().contains("a test"),
        "{answer}"
    );
    assert!(answer["focus"].get("aid").is_none(), "{answer}");
    let (_, view) = get(&rig.app, "/api/imaging").await;
    assert_eq!(view["settings"]["exposure_ms"], json!(2000.0), "{view}");
    assert!(!root(&rig).join("focus.json").exists());

    drop(lease);
    let (status, view) = post_focus(&rig.app, json!({"tile": {"col": 1, "row": 1}})).await;
    assert_eq!(status, StatusCode::OK, "{view}");
    let (status, _) = delete_focus(&rig.app).await;
    assert_eq!(status, StatusCode::OK);
}

/// US5-S4: a start while the holder's own imaging frame is exposing is refused
/// with the holder's name, not with `an exposure is in progress` (oracle:
/// 031 US9's rule 2, with the lease before the frame in flight).
#[tokio::test(start_paused = true)]
async fn a_start_while_the_holder_s_frame_is_exposing() {
    let scratch = crate::support::Scratch::new("imaging-focus-aid-exposing");
    let rig = focus_rig(&scratch.0.join("imaging"), stars(3.0, 12));
    let imaging = imaging(&rig);
    wait_ready(&rig.app).await;
    let (status, _) = put(&rig.app, json!({"exposure_ms": 10000})).await;
    assert_eq!(status, StatusCode::OK);
    let lease = imaging.lease("a test").expect("the lease");
    lease.take("test").await.expect("the holder's frame");
    wait_until(&rig.app, |view| view["frame"]["state"] == json!("exposing")).await;

    let (status, answer) = post_focus(&rig.app, json!({"tile": {"col": 1, "row": 1}})).await;
    assert_eq!(status, StatusCode::CONFLICT, "{answer}");
    assert!(
        answer["error"].as_str().unwrap().contains("a test"),
        "{answer}"
    );
    assert_ne!(
        answer["error"],
        json!("an exposure is in progress"),
        "{answer}"
    );
    assert!(answer["focus"].get("aid").is_none(), "{answer}");
}

/// US5-S5's shared setup: a focus rig of its own, ready, with the exposure set
/// to 2000 ms.
async fn endings_rig(root: &Path, entries: Vec<Entry>) -> Rig {
    let rig = focus_rig(root, entries);
    wait_ready(&rig.app).await;
    let (status, _) = put(&rig.app, json!({"exposure_ms": 2000})).await;
    assert_eq!(status, StatusCode::OK);
    rig
}

/// The view once the aid has ended.
async fn ended(rig: &Rig) -> Value {
    wait_until(&rig.app, |view| {
        view["focus"]["aid"]["state"] == json!("ended")
    })
    .await
}

/// The settings are back as they were.
fn back(view: &Value) {
    assert_eq!(view["settings"]["exposure_ms"], json!(2000.0), "{view}");
    assert_eq!(
        view["settings"]["roi"],
        json!({"x": 0, "y": 0, "width": 1236, "height": 822}),
        "{view}"
    );
}

/// US5-S5: the endings. A solver error ends the run `failed` with the solver's
/// words and the settings back; a stop and an abort during a long exposure end
/// it `stopped` at once; a search that finds nothing ends it `no_region` with
/// its words; and a lost link ends it `failed` with a `restore_error`, the
/// lease free after each (oracle: the stated reasons and words, compared
/// exactly, and 031 US9's rules for the lease).
#[tokio::test(start_paused = true)]
async fn the_aid_ends() {
    let scratch = crate::support::Scratch::new("imaging-focus-aid-ends");
    let folder = |n: u32| scratch.0.join(format!("imaging-{n}"));
    let tile = || json!({"tile": {"col": 1, "row": 1}});

    // A solver error.
    let rig = endings_rig(
        &folder(0),
        vec![
            Entry::Stars(3.0, 12),
            Entry::Error(SolveError::TimedOut { seconds: 30 }),
        ],
    )
    .await;
    let (status, _) = post_focus(&rig.app, tile()).await;
    assert_eq!(status, StatusCode::OK);
    let view = ended(&rig).await;
    let aid = &view["focus"]["aid"];
    assert_eq!(aid["ended_reason"], json!("failed"), "{aid}");
    assert_eq!(
        aid["error"],
        json!("the imaging solver gave no answer within 30 s"),
        "{aid}"
    );
    assert!(aid.get("restore_error").is_none(), "{aid}");
    back(&view);
    assert_eq!(imaging(&rig).holder(), None);

    // A stop while the frame is exposing: the answer comes within 5 s.
    let rig = endings_rig(&folder(1), stars(3.0, 12)).await;
    let (status, _) = post_focus(
        &rig.app,
        json!({"exposure_ms": 10000, "tile": {"col": 1, "row": 1}}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    wait_until(&rig.app, |view| view["frame"]["state"] == json!("exposing")).await;
    let before = tokio::time::Instant::now();
    let (status, view) = delete_focus(&rig.app).await;
    assert_eq!(status, StatusCode::OK, "{view}");
    assert!(
        before.elapsed() < Duration::from_secs(5),
        "the stop took {:?}",
        before.elapsed()
    );
    assert_eq!(view["focus"]["aid"]["state"], json!("ended"));
    assert_eq!(view["focus"]["aid"]["ended_reason"], json!("stopped"));
    back(&view);
    assert_eq!(imaging(&rig).holder(), None);

    // An abort of the aid's own frame ends it `stopped` too.
    let rig = endings_rig(&folder(2), stars(3.0, 12)).await;
    let (status, _) = post_focus(
        &rig.app,
        json!({"exposure_ms": 10000, "tile": {"col": 1, "row": 1}}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    wait_until(&rig.app, |view| view["frame"]["state"] == json!("exposing")).await;
    post_abort(&rig.app).await;
    let view = ended(&rig).await;
    assert_eq!(
        view["focus"]["aid"]["ended_reason"],
        json!("stopped"),
        "{view}"
    );
    back(&view);
    assert_eq!(imaging(&rig).holder(), None);

    // No tile with five stars, none far out: `no_region`, and no file kept.
    let rig = endings_rig(&folder(4), stars(3.0, 2)).await;
    let (status, _) = post_focus(&rig.app, json!({})).await;
    assert_eq!(status, StatusCode::OK);
    let view = ended(&rig).await;
    let aid = &view["focus"]["aid"];
    assert_eq!(aid["ended_reason"], json!("no_region"), "{aid}");
    assert_eq!(
        aid["error"],
        json!("no tile of the imaging frame had five stars to measure"),
        "{aid}"
    );
    assert!(view["focus"].get("region").is_none(), "{view}");
    back(&view);
    assert_eq!(imaging(&rig).holder(), None);
    assert!(!root(&rig).join("focus.json").exists());

    // A lost link: the aid ends `failed`, with a `restore_error`, the lease
    // free, and the camera answered again once the link is back.
    let rig = endings_rig(&folder(3), stars(3.0, 12)).await;
    let (status, _) = post_focus(&rig.app, tile()).await;
    assert_eq!(status, StatusCode::OK);
    wait_until(&rig.app, |view| {
        view["focus"]["aid"]["readings"]
            .as_u64()
            .is_some_and(|n| n >= 1)
    })
    .await;
    assert!(imaging(&rig).focus_running());
    rig.sim.set_unresponsive(true);
    for _ in 0..3000 {
        if !imaging(&rig).focus_running() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(!imaging(&rig).focus_running(), "the aid never ended");
    assert_eq!(imaging(&rig).holder(), None);
    rig.sim.set_unresponsive(false);
    wait_ready(&rig.app).await;
    let (_, view) = get(&rig.app, "/api/imaging").await;
    let aid = &view["focus"]["aid"];
    assert_eq!(aid["ended_reason"], json!("failed"), "{aid}");
    assert!(
        aid["restore_error"]
            .as_str()
            .is_some_and(|words| !words.is_empty()),
        "{aid}"
    );
}
