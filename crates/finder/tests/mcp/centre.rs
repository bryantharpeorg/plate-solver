//! US5: the MCP tool `centre` lists with its schema, answers the route's refusals and
//! the centring's answer through the same function, shares the rate limit and the call
//! log of the moving tools, and the README and the glossary carry the centring's words.
//! The oracles are the simulator's imaging truth, push and true eyepiece pixel, M42's
//! catalogue place, the scenario's own words and plan.md § Interfaces.
use crate::support::{goto::Stars, live, mcp as support, Scratch};

use std::{
    fs,
    io::Write,
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{body::Body, http::StatusCode, Router};
use finder::{
    imaging::simulated::SimulatedSolver,
    link::Transport,
    session::{Aim, Session},
    targets, AppState,
};
use math_core::{angular_distance, UnitVector};
use scope_link::memory;
use serde_json::{json, Value};
use simulator::{mount::SimMount, SimNode, Simulator, Site, VirtualClock};
use support::*;

const NOT_SOLVED: &str = "the imaging camera has not solved a frame yet";
const PARTS: &str = "centre takes nothing, to centre the target, or frame_id, x and y together";
const LIMITED: &str = "the rate limit of 2 moving calls a second was reached";

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
impl Log {
    fn text(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }
}

/// The node's routes over a fresh duplex for each connection.
fn duplex(node: Arc<SimNode>) -> Transport {
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

/// The centring rig: the simulator with the mount connected and pushed to alt 50°,
/// az 120°, its node and paired session with M42 as the target, the imaging camera
/// with the simulated solver, and the MCP door's call log.
struct Rig {
    sim: Simulator,
    state: AppState,
    app: Router,
    _scratch: Scratch,
}

impl Rig {
    async fn new(log: Log) -> Self {
        let scratch = Scratch::new("mcp-centre");
        let sim = Simulator::new(
            live::DB.clone(),
            Site::default(),
            VirtualClock::new(live::START),
        );
        sim.connect_mount(SimMount::new(sim.clock().clone()));
        sim.push_to(50.0, 120.0);
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
                    "site": {"lat_deg": 40, "lon_deg": -75}, "target": "M42"
                }))
                .unwrap(),
                live::START,
            )
            .unwrap();
        let state = AppState::shared(live::DB.clone(), Default::default())
            .with_session(session)
            .with_clock(Arc::new(live::ConsoleClock(sim.clock().clone())))
            .with_link(duplex(node))
            .with_live_solver(Arc::new(Stars))
            .with_imaging(scratch.0.join("imaging"))
            .with_imaging_solver(Arc::new(SimulatedSolver::new(sim.clone())))
            .with_call_log(log);
        let rig = Self {
            sim,
            app: app(state.clone(), "127.0.0.1"),
            state,
            _scratch: scratch,
        };
        rig.until("/api/imaging", |v| v["state"] == "ready").await;
        rig.until("/api/session", |v| {
            v["model"]["syncs"].as_u64().is_some_and(|n| n > 0)
        })
        .await;
        rig
    }

    fn now(&self) -> f64 {
        self.sim.clock().now()
    }

    async fn sleep(&self, seconds: f64) {
        tokio::time::sleep(Duration::from_secs_f64(seconds)).await;
    }

    /// Poll `path` every 100 ms of virtual time until `done` holds, for at most 300 s.
    async fn until(&self, path: &str, done: impl Fn(&Value) -> bool) -> Value {
        for _ in 0..3000 {
            let view = get(&self.app, path).await;
            if done(&view) {
                return view;
            }
            self.sleep(0.1).await;
        }
        panic!(
            "{path} never reached its state: {}",
            get(&self.app, path).await
        );
    }

    /// Sensor centre's distance from a J2000 place, from simulator truth.
    fn off(&self, place: (f64, f64)) -> f64 {
        let truth = self.sim.imaging_truth();
        sep((truth.ra_deg, truth.dec_deg), place)
    }

    /// One saved frame, solved; the solve's own answer.
    async fn solve(&self) -> Value {
        let view = post(&self.app, "/api/imaging/frame", json!({})).await;
        let id = view["frame"]["id"].as_u64().unwrap();
        self.until("/api/imaging", |v| {
            v["frame"]["id"] == id && v["frame"]["state"] == "saved"
        })
        .await;
        post(&self.app, "/api/imaging/solve", json!({})).await;
        let view = self
            .until("/api/imaging", |v| {
                v["solve"]["frame_id"] == id && v["solve"]["state"] != "solving"
            })
            .await;
        assert_eq!(view["solve"]["state"], "solved", "{view}");
        view["solve"].clone()
    }

    async fn track_on(&self) {
        post(&self.app, "/api/track", json!({"on": true})).await;
        self.until("/api/session", |v| v["track"]["state"] == "tracking")
            .await;
    }
}

/// Sky separation in arcseconds between two J2000 places.
fn sep(a: (f64, f64), b: (f64, f64)) -> f64 {
    angular_distance(
        UnitVector::from_radec(a.0.to_radians(), a.1.to_radians()),
        UnitVector::from_radec(b.0.to_radians(), b.1.to_radians()),
    )
    .to_degrees()
        * 3600.0
}

fn m42() -> (f64, f64) {
    let target = targets::find("M42").unwrap();
    (target.ra_deg, target.dec_deg)
}

/// `GET` a path, answering its JSON body.
async fn get(app: &Router, path: &str) -> Value {
    let response = send(
        app,
        axum::http::Request::get(path).body(Body::empty()).unwrap(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK, "GET {path}");
    body(response).await
}

/// `POST` a JSON body to a same-origin write route, answering its JSON body.
async fn post(app: &Router, path: &str, value: Value) -> Value {
    let response = send(
        app,
        axum::http::Request::post(path)
            .header("content-type", "application/json")
            .body(Body::from(value.to_string()))
            .unwrap(),
    )
    .await;
    let status = response.status();
    let answer = body(response).await;
    assert_eq!(status, StatusCode::OK, "POST {path}: {answer}");
    answer
}

/// One tool call's result: an MCP answer that never names the node's token.
async fn tool(app: &Router, name: &str, arguments: Value) -> Value {
    let response = send(app, call(name, arguments)).await;
    assert_eq!(response.status(), 200);
    let rpc = body(response).await;
    assert!(rpc.get("error").is_none(), "{rpc}");
    let result = rpc["result"].clone();
    assert!(!result.to_string().contains("test-token"), "{result}");
    result
}

/// The words of a tool error, which is its only content.
fn error(result: &Value) -> &str {
    assert_eq!(result["isError"], true, "{result}");
    assert_eq!(result["content"].as_array().unwrap().len(), 1, "{result}");
    result["content"][0]["text"].as_str().unwrap()
}

/// US5-S1: the tool is listed with its schema; its refusals answer the route's words;
/// the centring answers as its structured content, holds `motion` for its caller and
/// moves the telescope to the target; the next two calls meet the moved telescope and
/// the limit; and the call log names the call without the token.
#[tokio::test(start_paused = true)]
async fn centre_is_listed_answers_the_route_and_moves_the_telescope_through_the_limit() {
    let log = Log::default();
    let rig = Rig::new(log.clone()).await;
    let client = &rig.app;

    let listing = body(
        send(
            client,
            support::request(json!({"jsonrpc":"2.0","id":7,"method":"tools/list"})),
        )
        .await,
    )
    .await;
    let entry = listing["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|candidate| candidate["name"] == "centre")
        .expect("centre is listed");
    assert!(entry["description"]
        .as_str()
        .unwrap()
        .contains("real telescope"));
    assert_eq!(entry["outputSchema"]["type"], "object");
    assert_eq!(
        entry["inputSchema"]["properties"]["frame_id"]["type"],
        json!(["integer", "null"]),
        "{entry}"
    );
    for property in ["x", "y"] {
        assert_eq!(
            entry["inputSchema"]["properties"][property]["type"],
            json!(["number", "null"]),
            "{entry}"
        );
    }

    let refused = tool(client, "centre", json!({})).await;
    assert_eq!(error(&refused), NOT_SOLVED);
    rig.sleep(1.1).await;
    assert_eq!(
        error(&tool(client, "centre", json!({"frame_id": 1})).await),
        PARTS
    );
    assert_eq!(error(&tool(client, "centre", json!({"q": 1})).await), PARTS);

    // The aim point at the true eyepiece pixel, Track on, then the finder camera's
    // frames stopped and the mount pushed by hand 240″ in azimuth: Track holds the
    // place, and the imaging solve shows the centre off M42.
    let (x, y) = rig.sim.optics().eyepiece_px;
    rig.state
        .session()
        .set_aim(Some(Aim { x, y }), rig.now())
        .unwrap();
    rig.track_on().await;
    rig.sleep(30.0).await;
    rig.sim.stop_frames();
    rig.sleep(2.0).await;
    rig.sim
        .push_by(240.0 / 3600.0 / 50.0_f64.to_radians().cos(), 0.0);
    rig.sleep(2.0).await;
    let solved = rig.solve().await;
    let frame = solved["frame_id"].as_u64().unwrap();
    let off = rig.off(m42());

    let answer = tool(client, "centre", json!({})).await;
    assert_ne!(answer["isError"], true, "{answer}");
    let centred = &answer["structuredContent"];
    assert_eq!(centred["frame_id"], frame, "{answer}");
    assert_eq!(centred["moved"], "track", "{answer}");
    assert!(
        (centred["offset_arcsec"].as_f64().unwrap() - off).abs() <= 5.0,
        "{answer}, true offset {off}"
    );

    let read = get(client, "/api/session").await;
    assert_eq!(read["motion"]["by"], "mcp", "{read}");
    assert_eq!(read["motion"]["tool"], "centre", "{read}");

    let moved = tool(client, "centre", json!({})).await;
    assert_eq!(
        error(&moved),
        format!("the telescope moved after imaging frame {frame}: take another and solve it")
    );
    let limited = tool(client, "centre", json!({})).await;
    assert_eq!(error(&limited), LIMITED);

    rig.until("/api/session", |v| v["axis_move"]["state"] == "done")
        .await;
    rig.sleep(12.0).await;
    assert!(rig.off(m42()) <= 25.0, "{} off M42", rig.off(m42()));

    assert!(
        log.text().lines().any(|line| line.contains("mcp centre")),
        "{}",
        log.text()
    );
    assert!(!log.text().contains("test-token"));
}

/// US5-S2: the committed README and glossary name the centring where FR-012 and FR-013
/// place it, with the words those requirements and plan.md § Interfaces give.
#[test]
fn the_readme_and_the_glossary_name_the_centring_in_their_places() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let readme =
        fs::read_to_string(root.join("README.md")).expect("the committed README must be readable");
    let glossary = fs::read_to_string(root.join("../../GLOSSARY.md"))
        .expect("the committed glossary must be readable");
    assert!(readme.contains("The imaging camera has sixteen tools of its own"));

    let nudge = readme
        .find("The page's nudge arrows have one tool:")
        .expect("the nudge arrows have their own paragraph");
    let paragraph = readme
        .find("Centring from an imaging solve has one tool:")
        .expect("the centring has its own paragraph");
    let wsl = readme.find("### MCP under WSL2").unwrap();
    assert!(nudge < paragraph && paragraph < wsl);
    assert!(readme[paragraph..wsl]
        .lines()
        .any(|line| line.starts_with("- `centre` takes nothing")));

    let api = readme.find("\n## API\n").expect("the API section") + 1;
    let api_section = &readme[api..api + readme[api..].find("\n## ").unwrap()];
    let row = |prefix: &str| {
        api_section
            .lines()
            .position(|line| line.starts_with(prefix))
    };
    let centre = row("| `POST /api/imaging/centre` |").expect("the centring's API row");
    let goto = row("| `POST /api/goto` |").expect("the GoTo's API row");
    assert!(centre < goto);

    let nudging = readme
        .find("\n## Nudging the telescope\n")
        .expect("the nudging section");
    let centring = readme
        .find("\n## Centring from an imaging solve\n")
        .expect("the centring section");
    let recording = readme.find("\n## Recording a night\n").unwrap();
    assert!(nudging < centring && centring < recording);
    let section = readme[centring..recording]
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    for words in [
        "It needs no live pointing",
        "`a newer imaging frame was taken after frame N`",
        "`the telescope moved after imaging frame N: take another and solve it`",
        "A centring is never a sync.",
        "The finder camera's first solve within a minute after it takes the aim point",
    ] {
        assert!(section.contains(words), "the section lacks {words:?}");
    }

    let entry = "**Centring from an imaging solve**:\nBringing a place, the session's target or a point tapped on the latest solved imaging frame, to the imaging camera's set point from that frame's imaging solve alone: the place Track holds becomes that place, and Track steps the telescope by what the solve says lies between them. It needs no live pointing and is never a sync; the finder camera's first solve within a minute after it takes the aim point from the place Track holds.\n_Avoid_: recentre (the imaging run's Track off and on to a new aim point), slew to solve, plate-solve sync\n";
    let found = glossary
        .find(entry)
        .expect("the glossary defines the centring with FR-013's words");
    let nudge_entry = glossary
        .find("**Nudge**:\n")
        .expect("the glossary has the nudge");
    assert!(
        glossary[nudge_entry..found].ends_with("_Avoid_: jog, slew, manual move\n\n"),
        "the centring does not follow the nudge"
    );
}
