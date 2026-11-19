//! US10: stacking through the MCP door. Oracles: the declared tool contract,
//! a scripted solve of the stand-in's 64 × 48 samples, and simulator truth.
//! JSON, errors and dimensions are exact; all waits use paused tokio time.

use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::Router;
use base64::prelude::{Engine as _, BASE64_STANDARD};
use finder::{
    imaging::{
        simulated::SimulatedSolver,
        solver::{
            ImagingSolve, ImagingSolver, MeasureFuture, Region, SolveFrame, SolveFuture, SolveHint,
            StarMeasure,
        },
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

use crate::support::{live, mcp::*, Scratch};

struct ScriptedSolver;
impl ScriptedSolver {
    fn new() -> Self {
        Self
    }
}
impl ImagingSolver for ScriptedSolver {
    fn name(&self) -> &'static str {
        "simulated"
    }
    fn solve(&self, frame: SolveFrame, _hint: SolveHint) -> SolveFuture<'_> {
        Box::pin(async move {
            // Fixed centre and north-up scale: every sub covers the same grid.
            let scale = 2.16 / 3600.0;
            let mut solve = ImagingSolve::from_cd(
                83.8221,
                -5.3911,
                [[-scale, 0.0], [0.0, -scale]],
                frame.width,
                frame.height,
            );
            solve.stars = Some(50);
            Ok(solve)
        })
    }
    fn measure(&self, _frame: SolveFrame, _region: Option<Region>) -> MeasureFuture<'_> {
        Box::pin(async {
            Ok(StarMeasure {
                half_flux_px: 2.0,
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

#[derive(Clone, Default)]
struct Log(Arc<Mutex<Vec<u8>>>);
impl Write for Log {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct Rig {
    app: Router,
    root: PathBuf,
    log: Log,
    _scratch: Scratch,
}

fn sim() -> Simulator {
    let sim = live::sim();
    sim.connect_mount(SimMount::new(sim.clock().clone()));
    sim.push_to(50.0, 120.0);
    sim
}

fn console(sim: &Simulator, transport: Transport, root: &Path) -> AppState {
    let session = Session::default();
    session
        .edit(
            serde_json::from_value(json!({
                "node":{"address":"scope.local:1","token":"test-token"},
                "site":{"lat_deg":40,"lon_deg":-75}
            }))
            .unwrap(),
            live::START,
        )
        .unwrap();
    AppState::shared(live::DB.clone(), finder::samples::Samples::default())
        .with_session(session)
        .with_clock(Arc::new(live::ConsoleClock(sim.clock().clone())))
        .with_link(transport)
        .with_imaging(root)
}

async fn rig(name: &str, simulated: bool) -> Rig {
    let scratch = Scratch::new(name);
    let root = scratch.0.join("imaging");
    let sim = sim();
    let state = if simulated {
        // Rebuild 032's solving rig with live finder solves; the imaging
        // solver reads this simulator's truth, not the scripted answers.
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
        console(&sim, transport, &root)
            .with_live()
            .with_imaging_solver(Arc::new(SimulatedSolver::new(sim.clone())))
    } else {
        let clock: Arc<dyn NodeClock> = Arc::new(NodeTime(sim.clock().clone()));
        let node = NodeService::with_controls(
            SyntheticCamera::new(clock.clone()),
            clock.clone(),
            "test-token",
            Settings::default(),
            NodeControls::default(),
        )
        .unwrap()
        .with_imaging(SyntheticImagingCamera::new(clock));
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
        console(&sim, transport, &root).with_imaging_solver(Arc::new(ScriptedSolver::new()))
    };
    let log = Log::default();
    let rig = Rig {
        app: app(state.with_call_log(log.clone()), "127.0.0.1"),
        root,
        log,
        _scratch: scratch,
    };
    for _ in 0..600 {
        let result = tool(&rig.app, "get_imaging_camera", json!({})).await;
        if result["structuredContent"]["state"] == "ready" {
            return rig;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("the imaging camera did not become ready");
}

async fn tool(app: &Router, name: &str, arguments: Value) -> Value {
    let response = send(app, call(name, arguments)).await;
    assert_eq!(response.status(), 200);
    let rpc = body(response).await;
    assert!(rpc.get("error").is_none(), "{rpc}");
    let result = rpc["result"].clone();
    assert!(
        !result.to_string().contains("test-token"),
        "{name}: {result}"
    );
    result
}

fn view(result: &Value) -> &Value {
    assert_ne!(result["isError"], true, "{result}");
    assert_eq!(result["content"][0]["type"], "text");
    assert_eq!(
        serde_json::from_str::<Value>(result["content"][0]["text"].as_str().unwrap()).unwrap(),
        result["structuredContent"]
    );
    &result["structuredContent"]
}

fn error_text(result: &Value) -> &str {
    assert_eq!(result["isError"], true, "{result}");
    result["content"][0]["text"].as_str().unwrap()
}

fn picture(result: &Value, dimensions: (u32, u32)) -> image::RgbImage {
    view(result);
    assert_eq!(result["content"].as_array().unwrap().len(), 2);
    let image = &result["content"][1];
    assert_eq!(image["type"], "image");
    assert_eq!(image["mimeType"], "image/jpeg");
    let bytes = BASE64_STANDARD
        .decode(image["data"].as_str().unwrap())
        .unwrap();
    let decoded = image::load_from_memory_with_format(&bytes, image::ImageFormat::Jpeg).unwrap();
    assert_eq!(decoded.color(), image::ColorType::Rgb8);
    let rgb = decoded.into_rgb8();
    assert_eq!(rgb.dimensions(), dimensions);
    rgb
}

async fn wait_stack(app: &Router, done: impl Fn(&Value) -> bool) -> Value {
    for _ in 0..3000 {
        let result = tool(app, "get_stack", json!({})).await;
        if done(view(&result)) {
            return result;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("the stack did not reach the awaited state");
}

/// US10-S1: exact listing contract, including the five input keys.
#[tokio::test]
async fn the_two_tools_are_listed_with_their_schemas_and_descriptions() {
    let app = app(state(), "127.0.0.1");
    let response = send(
        &app,
        request(json!({"jsonrpc":"2.0","id":1,"method":"tools/list"})),
    )
    .await;
    assert_eq!(response.status(), 200);
    let listing = body(response).await;
    let tools = listing["result"]["tools"].as_array().unwrap();
    let named = |name| tools.iter().find(|tool| tool["name"] == name).unwrap();
    for name in ["set_stacking", "get_stack"] {
        assert_eq!(named(name)["outputSchema"], json!({"type":"object"}));
        assert_eq!(
            named(name)["annotations"]["readOnlyHint"] == true,
            name == "get_stack"
        );
    }
    assert_eq!(named("set_stacking")["description"], "Start or stop the live stack of subs, as POST and DELETE /api/imaging/stack do. With on true it also takes reject, focus_gate, gains and stretch; on a running stack it changes gains and stretch.");
    assert_eq!(named("get_stack")["description"], "Read the live stack: its state, counts, dropped subs by reason and, when it has one, its picture as a colour JPEG.");
    let keys: Vec<_> = named("set_stacking")["inputSchema"]["properties"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(keys, ["focus_gate", "gains", "on", "reject", "stretch"]);
    assert_eq!(
        named("set_stacking")["inputSchema"]["required"],
        json!(["on"])
    );
}

/// US10-S2: start, read with a picture, session readback, edit, stop and
/// file existence through MCP. Target label and an empty call log are exact.
#[tokio::test(start_paused = true)]
async fn a_stack_can_be_started_read_changed_and_saved_without_a_call_log_line() {
    let rig = rig("mcp-stack-actions", false).await;
    view(&tool(&rig.app, "set_target", json!({"target":"M42"})).await);
    let started = tool(&rig.app, "set_stacking", json!({"on":true})).await;
    assert_eq!(view(&started)["state"], "stacking");
    assert_eq!(
        view(&started)["options"],
        json!({
            "reject":true,"focus_gate":true,"gains":[1.0,1.0,1.0],"stretch":10.0
        })
    );
    let id = view(&started)["id"].as_str().unwrap();
    assert!(id.ends_with("Z-M42"), "{id}");
    let result = wait_stack(&rig.app, |v| {
        v["stacked"].as_u64().unwrap_or(0) >= 2 && v["preview"].is_string()
    })
    .await;
    assert_eq!(view(&result)["id"], id);
    assert!(view(&result)["stacked"].as_u64().unwrap() >= 2);
    picture(&result, (64, 48));
    let session = tool(&rig.app, "get_session", json!({})).await;
    assert_eq!(view(&session)["imaging"]["stack"]["id"], id);
    let changed = tool(&rig.app, "set_stacking", json!({"on":true,"gains":[2,1,1]})).await;
    assert_eq!(view(&changed)["options"]["gains"], json!([2.0, 1.0, 1.0]));
    assert_eq!(view(&changed)["id"], id);
    let changed = tool(&rig.app, "set_stacking", json!({"on":true,"stretch":20})).await;
    assert_eq!(view(&changed)["options"]["stretch"], 20.0);
    for key in ["reject", "focus_gate"] {
        assert_eq!(
            error_text(&tool(&rig.app, "set_stacking", json!({"on":true,key:false})).await),
            "the stack is running: stop it to change reject or focus_gate"
        );
    }
    let ended = tool(&rig.app, "set_stacking", json!({"on":false})).await;
    assert_eq!(view(&ended)["state"], "ended");
    assert_eq!(view(&ended)["ended_reason"], "stopped");
    let fits = view(&ended)["files"]["fits"].as_str().unwrap();
    assert_eq!(fits, format!("runs/{id}/stack.fits"));
    assert!(rig.root.join(fits).is_file(), "{fits}");
    let result = tool(&rig.app, "get_stack", json!({})).await;
    assert_eq!(view(&result)["state"], "ended");
    picture(&result, (64, 48));
    assert!(rig.log.0.lock().unwrap().is_empty());
}

/// US10-S3: the specified refusals and empty-stack answers, compared exactly.
#[tokio::test(start_paused = true)]
async fn bad_arguments_are_refused_and_an_absent_stack_reads_and_stops_as_none() {
    let rig = rig("mcp-stack-arguments", false).await;
    let result = tool(&rig.app, "get_stack", json!({})).await;
    assert_eq!(view(&result), &json!({"state":"none"}));
    assert_eq!(result["content"].as_array().unwrap().len(), 1);
    let result = tool(&rig.app, "set_stacking", json!({"on":false})).await;
    assert_eq!(view(&result), &json!({"state":"none"}));
    for (arguments, expected) in [
        (json!({}), "on must be true or false"),
        (json!({"on":"yes"}), "on must be true or false"),
        (
            json!({"on":false,"reject":true}),
            "on false takes no other argument",
        ),
        (
            json!({"on":true,"gains":[0,1,1]}),
            "gains must be three numbers from 0.1 to 10",
        ),
        (
            json!({"on":true,"stretch":0}),
            "stretch must be a number from 1 to 1000",
        ),
    ] {
        assert_eq!(
            error_text(&tool(&rig.app, "set_stacking", arguments).await),
            expected
        );
    }
    for arguments in [
        json!({"on":true,"colour":1}),
        json!({"on":true,"reject":"yes"}),
        json!({"on":true,"focus_gate":1}),
        json!({"on":true,"gains":[1,1]}),
        json!({"on":true,"stretch":"ten"}),
        json!({"on":true,"gains":null}),
        json!({"on":true,"reject":null}),
    ] {
        assert!(!error_text(&tool(&rig.app, "set_stacking", arguments).await).is_empty());
    }
    assert_eq!(
        error_text(&tool(&rig.app, "get_stack", json!({"x":1})).await),
        "get_stack takes no arguments"
    );
    assert_eq!(
        view(&tool(&rig.app, "get_stack", json!({})).await),
        &json!({"state":"none"})
    );
}

/// US10-S4: no imaging; shape errors precede capability errors, and empty
/// arguments answer with zero elapsed virtual time. Every tool call checks
/// the paired token is absent, including both tools with {} on the stand-in.
#[tokio::test(start_paused = true)]
async fn a_console_without_imaging_judges_shape_first_and_answers_at_once() {
    let app = app(state(), "127.0.0.1");
    let before = tokio::time::Instant::now();
    let result = tokio::time::timeout(Duration::from_secs(1), tool(&app, "get_stack", json!({})))
        .await
        .expect("get_stack must answer at once");
    assert_eq!(view(&result), &json!({"state":"none"}));
    assert_eq!(result["content"].as_array().unwrap().len(), 1);
    for (arguments, expected) in [
        (json!({}), "on must be true or false"),
        (json!({"on":"yes"}), "on must be true or false"),
        (
            json!({"on":false,"reject":true}),
            "on false takes no other argument",
        ),
        (json!({"on":true}), "this console takes no imaging frames"),
        (json!({"on":false}), "this console takes no imaging frames"),
    ] {
        let result = tokio::time::timeout(
            Duration::from_secs(1),
            tool(&app, "set_stacking", arguments),
        )
        .await
        .expect("set_stacking must answer at once");
        assert_eq!(error_text(&result), expected);
    }
    let result = tool(&app, "set_stacking", json!({"on":true,"colour":1})).await;
    assert_ne!(error_text(&result), "this console takes no imaging frames");
    assert_eq!(tokio::time::Instant::now(), before);
}

/// US10-S5: 1236 × 822 / k=2 gives exactly 618 × 411. Decode the actual
/// JPEG and confirm colour survives transport, with no display code as oracle.
#[tokio::test(start_paused = true)]
async fn the_simulated_stack_picture_is_colour_and_at_most_1024_wide() {
    let rig = rig("mcp-stack-sim-picture", true).await;
    let started = tool(&rig.app, "set_stacking", json!({"on":true})).await;
    assert_eq!(view(&started)["state"], "stacking");
    let result = wait_stack(&rig.app, |v| v["preview"].is_string()).await;
    let rgb = picture(&result, (618, 411));
    assert!(rgb.pixels().any(|p| p[0] != p[1] || p[1] != p[2]));
    assert_eq!(
        view(&tool(&rig.app, "set_stacking", json!({"on":false})).await)["state"],
        "ended"
    );
}
