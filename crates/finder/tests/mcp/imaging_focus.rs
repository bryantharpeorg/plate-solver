//! The focus MCP tools (033 US7): the listing of `get_focus_reading` and
//! `set_focus_aid`, a focus check that waits until it ends, the start, the
//! stop and every refused argument, and a console without imaging or without
//! a solver. Oracles: the stated names, schema and words, compared exactly;
//! three exposures of 1 s on the virtual clock (a check of the kept middle
//! tile cannot end sooner); US8-S1's worked verdict — the median 3.20 sensor
//! pixels times the rig's scale is 7.011170″, judged `good` at 1.067 of
//! tonight's record 3.0; the route's own view; and the token the rig paired
//! with, which no result may name.

use crate::support::{live, mcp as support};

use std::{path::Path, sync::Arc, time::Duration};

use axum::{body::Body, Router};
use finder::{
    imaging::{
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
use support::*;

/// The scripted solver (the shared setup): `measure` answers the next entry
/// of its list at once, repeating the last once the list is used up; `solve`
/// is never called by the aid.
struct Scripted {
    entries: Vec<(f64, u32)>,
    next: std::sync::atomic::AtomicUsize,
}

impl Scripted {
    fn new(entries: Vec<(f64, u32)>) -> Self {
        Self {
            entries,
            next: std::sync::atomic::AtomicUsize::new(0),
        }
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

    fn measure(&self, _frame: SolveFrame, _region: Option<Region>) -> MeasureFuture<'_> {
        let n = self.next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let answer = Ok(StarMeasure {
            half_flux_px: self.entries[n.min(self.entries.len() - 1)].0,
            stars: self.entries[n.min(self.entries.len() - 1)].1,
        });
        Box::pin(async move { answer })
    }
}

/// The focus rig with MCP on (the shared setup): the test database at 40° N
/// 75° W on the virtual clock from Unix 1768528800, pointed at the celestial
/// pole, where the imaging field moves by under a pixel in three seconds; a
/// `SimNode` over it with the simulated imaging camera, served over a fresh
/// duplex for each connection; and a console over that with the imaging
/// folder `root`, the scripted solver, `with_focal_length_mm(1800.0)` and the
/// solver omitted when there is none, served through the MCP door.
struct Rig {
    app: Router,
    sim: Simulator,
    imaging: Arc<Imaging>,
}

fn rig(root: &Path, solver: Option<Arc<Scripted>>) -> Rig {
    let sim = Simulator::new(
        live::DB.clone(),
        Site::default(),
        VirtualClock::new(live::START),
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
            live::START,
        )
        .unwrap();
    let mut state = AppState::shared(live::DB.clone(), Samples::default())
        .with_session(session)
        .with_clock(Arc::new(live::ConsoleClock(sim.clock().clone())))
        .with_link(transport)
        .with_live()
        .with_imaging(root.to_path_buf())
        .with_focal_length_mm(1800.0);
    if let Some(solver) = solver {
        state = state.with_imaging_solver(solver);
    }
    let imaging = state.imaging().expect("the rig takes imaging frames");
    Rig {
        app: app(state, "127.0.0.1"),
        sim: sim.clone(),
        imaging,
    }
}

/// The focus rig with the scripted solver, built as the scenario's entries
/// name (the shared setup).
fn focus_rig(root: &Path, entries: Vec<(f64, u32)>) -> Rig {
    rig(root, Some(Arc::new(Scripted::new(entries))))
}

/// Write `focus.json` under the rig's imaging folder by hand (the shared
/// setup): the kept middle tile and tonight's record of `px`.
fn write_focus_file(root: &Path, px: f64) {
    std::fs::create_dir_all(root).unwrap();
    std::fs::write(
        root.join("focus.json"),
        format!(
            "{{\"format\":\"imaging-focus/1\",\
             \"camera\":{{\"model\":\"simulated imaging camera\",\"serial\":\"SIM-0001\",\
             \"width\":1236,\"height\":822,\"pixel_um\":19.12}},\
             \"region\":{{\"col\":1,\"row\":1,\"x\":412,\"y\":274,\"width\":412,\"height\":274,\
             \"source\":\"fixed\",\"at\":1768528000.0}},\
             \"record\":{{\"tonight\":{{\"px\":{px},\"at\":1768528000.0,\"since\":1768528000.0}}}}}}"
        ),
    )
    .unwrap();
}

/// Call a tool and answer its result, failing on a JSON-RPC error.
async fn tool(app: &Router, name: &str, arguments: Value) -> Value {
    let response = send(app, call(name, arguments)).await;
    assert_eq!(response.status(), 200);
    let rpc = body(response).await;
    assert!(rpc.get("error").is_none(), "{rpc}");
    rpc["result"].clone()
}

/// The words of a tool error.
fn error_text(result: &Value) -> &str {
    assert_eq!(result["isError"], true, "{result}");
    assert_eq!(result["content"][0]["type"], "text", "{result}");
    result["content"][0]["text"].as_str().unwrap()
}

/// `GET /api/imaging`.
async fn imaging(app: &Router) -> Value {
    let response = send(
        app,
        axum::http::Request::get("/api/imaging")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(response.status(), 200);
    body(response).await
}

/// Read `GET /api/imaging` every 100 ms until `check` holds, for at most
/// 60 s, and answer the view in which it first held.
async fn wait_until(app: &Router, check: impl Fn(&Value) -> bool) -> Value {
    for _ in 0..600 {
        let view = imaging(app).await;
        if check(&view) {
            return view;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!(
        "imaging never reached the awaited state: {}",
        imaging(app).await
    );
}

async fn ready(app: &Router) -> Value {
    wait_until(app, |view| view["state"] == "ready").await
}

/// US7-S1: both tools are listed as specified — the output schema, the
/// read-only hint of the reading tool alone, each description, and the five
/// input keys of which `on` alone is required (oracle: the stated names,
/// schema and words, compared exactly).
#[tokio::test]
async fn the_two_focus_tools_are_listed_as_specified() {
    let app = app(state(), "127.0.0.1");
    let response = send(
        &app,
        request(json!({"jsonrpc":"2.0", "id":1, "method":"tools/list"})),
    )
    .await;
    let listing = body(response).await;
    let tools = listing["result"]["tools"].as_array().unwrap();
    let reading = tools
        .iter()
        .find(|tool| tool["name"] == "get_focus_reading")
        .expect("get_focus_reading is listed");
    let aid = tools
        .iter()
        .find(|tool| tool["name"] == "set_focus_aid")
        .expect("set_focus_aid is listed");

    assert_eq!(reading["outputSchema"], json!({"type":"object"}));
    assert_eq!(aid["outputSchema"], json!({"type":"object"}));
    assert_eq!(reading["annotations"]["readOnlyHint"], json!(true));
    assert_ne!(aid["annotations"]["readOnlyHint"], json!(true));
    assert_eq!(
        reading["description"],
        "Read the focus reading: the focus region, the focus record, and the reading, best, trend and verdict of the focus aid or the last focus check."
    );
    assert_eq!(
        aid["description"],
        "Start or stop the focus aid, as POST and DELETE /api/imaging/focus do. With check true it takes a focus check, waits until it ends and returns its verdict against the focus record."
    );
    let mut keys: Vec<&str> = aid["inputSchema"]["properties"]
        .as_object()
        .expect("the input schema's properties")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(keys, ["check", "exposure_ms", "find", "on", "tile"]);
    assert_eq!(aid["inputSchema"]["required"], json!(["on"]));
}

/// US7-S2: `set_focus_aid` with `check` answers only once the check has ended
/// — at the least three exposures of one second on the virtual clock — with
/// the worked verdict, and `get_focus_reading` then answers the route's own
/// focus object (oracles: the virtual clock, US8-S1's worked median and
/// verdict, and the route's own view).
#[tokio::test(start_paused = true)]
async fn a_check_answers_only_once_it_has_ended_and_the_reading_follows() {
    let scratch = crate::support::Scratch::new("mcp-imaging-focus-check");
    let root = scratch.0.join("imaging");
    let rig = focus_rig(&root, vec![(3.20, 12), (3.25, 12), (3.15, 12)]);
    write_focus_file(&root, 3.0);
    ready(&rig.app).await;

    let before = rig.sim.clock().now();
    let result = tool(&rig.app, "set_focus_aid", json!({"on":true,"check":true})).await;
    let moved = rig.sim.clock().now() - before;
    assert!(moved >= 3.0, "the clock moved only {moved} s");
    assert_ne!(result["isError"], true, "{result}");
    let focus = &result["structuredContent"];
    let aid = &focus["aid"];
    assert_eq!(aid["state"], json!("ended"), "{focus}");
    assert_eq!(aid["ended_reason"], json!("done"), "{aid}");
    assert_eq!(aid["verdict"], json!("good"), "{aid}");
    assert!(
        (aid["median"]["value"].as_f64().unwrap() - 7.011170).abs() < 1e-5,
        "{aid}"
    );

    // The reading tool answers the same focus the route's own view shows,
    // read just after.
    let result = tool(&rig.app, "get_focus_reading", json!({})).await;
    assert_ne!(result["isError"], true, "{result}");
    let read = &result["structuredContent"];
    let view = imaging(&rig.app).await;
    let route = &view["focus"];
    for key in [
        "unit",
        "aid.verdict",
        "aid.median.value",
        "record.tonight.value",
    ] {
        let read_at = key.split('.').fold(read, |value, part| &value[part]);
        let route_at = key.split('.').fold(route, |value, part| &value[part]);
        assert_eq!(read_at, route_at, "{key}: {read} vs {route}");
    }
    assert!(!read.to_string().contains("test-token"), "{read}");
}

/// US7-S3: the aid starts at once, refuses a second start, stops with
/// `stopped`, and every refused argument answers the route's own words
/// (oracle: the stated words).
#[tokio::test(start_paused = true)]
async fn the_aid_starts_stops_and_refuses_every_bad_argument() {
    let scratch = crate::support::Scratch::new("mcp-imaging-focus-arguments");
    let root = scratch.0.join("imaging");
    let rig = focus_rig(&root, vec![(3.0, 12)]);
    ready(&rig.app).await;

    // A start answers the focus view at once, the aid not ended.
    let result = tool(
        &rig.app,
        "set_focus_aid",
        json!({"on":true,"tile":{"col":1,"row":1}}),
    )
    .await;
    assert_ne!(result["isError"], true, "{result}");
    assert_ne!(
        result["structuredContent"]["aid"]["state"],
        json!("ended"),
        "{result}"
    );

    // A second start while the first runs: the route's own refusal.
    let result = tool(
        &rig.app,
        "set_focus_aid",
        json!({"on":true,"tile":{"col":1,"row":1}}),
    )
    .await;
    assert_eq!(error_text(&result), "the focus aid is already running");

    // A stop waits until the aid has ended and answers `stopped`.
    let result = tool(&rig.app, "set_focus_aid", json!({"on":false})).await;
    assert_ne!(result["isError"], true, "{result}");
    assert_eq!(
        result["structuredContent"]["aid"]["state"],
        json!("ended"),
        "{result}"
    );
    assert_eq!(
        result["structuredContent"]["aid"]["ended_reason"],
        json!("stopped"),
        "{result}"
    );
    assert_eq!(rig.imaging.holder(), None);

    // Every refused argument, with its words.
    let result = tool(&rig.app, "set_focus_aid", json!({})).await;
    assert_eq!(error_text(&result), "on must be true or false");
    let result = tool(&rig.app, "set_focus_aid", json!({"on":false,"check":true})).await;
    assert_eq!(
        error_text(&result),
        "set_focus_aid with on false takes no other argument"
    );
    let result = tool(
        &rig.app,
        "set_focus_aid",
        json!({"on":true,"find":true,"tile":{"col":0,"row":0}}),
    )
    .await;
    assert_eq!(error_text(&result), "give find or tile, not both");
    let result = tool(&rig.app, "get_focus_reading", json!({"x":1})).await;
    assert_eq!(error_text(&result), "get_focus_reading takes no arguments");
}

/// US7-S4: without imaging both tools answer at once with the route's words,
/// without a solver the reading names the solver, and no result of either
/// tool on the focus rig holds the node's token (oracles: tokio's paused
/// clock, the stated words and the pairing's token).
#[tokio::test(start_paused = true)]
async fn without_imaging_or_a_solver_the_tools_answer_at_once() {
    // A console with no imaging, as the support's plain state builds it.
    let app = app(state(), "127.0.0.1");
    let before = tokio::time::Instant::now();
    let result = tool(&app, "get_focus_reading", json!({})).await;
    assert!(
        before.elapsed() < Duration::from_secs(1),
        "the tool answered late: {:?}",
        before.elapsed()
    );
    assert_eq!(error_text(&result), "this console takes no imaging frames");
    let before = tokio::time::Instant::now();
    let result = tool(&app, "set_focus_aid", json!({})).await;
    assert!(
        before.elapsed() < Duration::from_secs(1),
        "the tool answered late: {:?}",
        before.elapsed()
    );
    assert_eq!(error_text(&result), "on must be true or false");
    let result = tool(&app, "set_focus_aid", json!({"on":true})).await;
    assert_eq!(error_text(&result), "this console takes no imaging frames");

    // The focus rig with no solver: the reading names the solver.
    let scratch = crate::support::Scratch::new("mcp-imaging-focus-no-solver");
    let root = scratch.0.join("imaging");
    let bare = rig(&root, None);
    ready(&bare.app).await;
    let result = tool(&bare.app, "get_focus_reading", json!({})).await;
    assert_eq!(error_text(&result), "this console has no imaging solver");

    // On the focus rig, no result of either tool names the pairing token.
    let scratch = crate::support::Scratch::new("mcp-imaging-focus-token");
    let root = scratch.0.join("imaging");
    let rig = focus_rig(&root, vec![(3.0, 12)]);
    ready(&rig.app).await;
    let results = [
        tool(&rig.app, "get_focus_reading", json!({})).await,
        tool(&rig.app, "get_focus_reading", json!({"x":1})).await,
        tool(&rig.app, "set_focus_aid", json!({})).await,
        tool(&rig.app, "set_focus_aid", json!({"on":true})).await,
        tool(
            &rig.app,
            "set_focus_aid",
            json!({"on":true,"tile":{"col":1,"row":1}}),
        )
        .await,
        tool(&rig.app, "set_focus_aid", json!({"on":false})).await,
    ];
    for result in &results {
        assert!(!result.to_string().contains("test-token"), "{result}");
    }
}
