//! US5: the MCP tool `centre` is listed with its schema, answers the route's view as its
//! structured content and its refusals as tool errors through the same function, shares the
//! rate limit, holds `motion` for its caller and logs its call without the token; and the
//! README and the glossary carry the centring where FR-012 and FR-013 place it. The oracles
//! are the simulator's own truth (its eyepiece pixel, its hand push and its imaging truth),
//! M42's catalogue place, and the words of FR-011 to FR-013 and plan.md § Interfaces.
use crate::support::{goto::Stars, live, mcp as support, Scratch};

use std::{
    fs,
    io::Write,
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{body::Body, http::Request, Router};
use finder::{
    imaging::simulated::SimulatedSolver,
    link::Transport,
    samples::Samples,
    session::{Aim, Session},
    targets, AppState,
};
use math_core::{angular_distance, UnitVector};
use scope_link::memory;
use serde_json::{json, Value};
use simulator::{mount::SimMount, ImagingOptics, SimNode, Simulator, Site, VirtualClock};
use support::{app, body, call, request, send, session};

const PARTS: &str = "centre takes nothing, to centre the target, or frame_id, x and y together";
const NOT_SOLVED: &str = "the imaging camera has not solved a frame yet";
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

/// The centring rig, as the motion stories build theirs: the test database on the virtual
/// clock from Unix 1768528800, the mount connected and pushed to alt 50° and az 120°, the
/// imaging camera at 17°, the node's star frames every 500 ms, the session on M42 at 40° N
/// 75° W, the imaging folder in a scratch directory, and MCP served with the call log.
struct Rig {
    sim: Simulator,
    state: AppState,
    app: Router,
    _scratch: Scratch,
}

impl Rig {
    async fn new(log: &Log) -> Self {
        let scratch = Scratch::new("mcp-centre");
        let sim = Simulator::new(
            live::DB.clone(),
            Site::default(),
            VirtualClock::new(live::START),
        );
        sim.connect_mount(SimMount::new(sim.clock().clone()));
        sim.push_to(50.0, 120.0);
        sim.set_imaging_optics(ImagingOptics {
            angle_deg: 17.0,
            ..ImagingOptics::default()
        });
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
        let state = AppState::shared(live::DB.clone(), Samples::default())
            .with_session(session)
            .with_clock(Arc::new(live::ConsoleClock(sim.clock().clone())))
            .with_link(transport)
            .with_live_solver(Arc::new(Stars))
            .with_imaging(scratch.0.join("imaging"))
            .with_imaging_solver(Arc::new(SimulatedSolver::new(sim.clone())))
            .with_call_log(log.clone());
        let app = app(state.clone(), "127.0.0.1");
        let rig = Self {
            sim,
            state,
            app,
            _scratch: scratch,
        };
        rig.wait("/api/imaging", |view| view["state"] == "ready")
            .await;
        rig.wait("/api/session", |view| {
            view["model"]["syncs"].as_u64().is_some_and(|n| n > 0)
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

    /// One route call, which must answer 200.
    async fn route(&self, method: &str, path: &str, value: Value) -> Value {
        let response = send(
            &self.app,
            Request::builder()
                .method(method)
                .uri(path)
                .header("host", "localhost:8080")
                .header("content-type", "application/json")
                .body(Body::from(value.to_string()))
                .unwrap(),
        )
        .await;
        let status = response.status();
        let answer = body(response).await;
        assert_eq!(status, 200, "{method} {path}: {answer}");
        answer
    }

    /// Poll the path every 100 ms of virtual time, bounded at 300 s.
    async fn wait(&self, path: &str, done: impl Fn(&Value) -> bool) -> Value {
        for _ in 0..3000 {
            let view = self.route("GET", path, Value::Null).await;
            if done(&view) {
                return view;
            }
            self.sleep(0.1).await;
        }
        panic!(
            "{path} never reached its state: {}",
            self.route("GET", path, Value::Null).await
        );
    }

    /// Sensor centre's distance from M42, from the simulator's truth.
    fn off_m42(&self) -> f64 {
        let truth = self.sim.imaging_truth();
        sep((truth.ra_deg, truth.dec_deg), m42())
    }

    /// Take a saved imaging frame and solve it, as the imaging camera's own routes do.
    async fn solve(&self) -> Value {
        let view = self.route("POST", "/api/imaging/frame", Value::Null).await;
        let id = view["frame"]["id"].as_u64().unwrap();
        self.wait("/api/imaging", |v| {
            v["frame"]["id"] == id && v["frame"]["state"] == "saved"
        })
        .await;
        self.route("POST", "/api/imaging/solve", Value::Null).await;
        let view = self
            .wait("/api/imaging", |v| {
                v["solve"]["frame_id"] == id && v["solve"]["state"] != "solving"
            })
            .await;
        assert_eq!(view["solve"]["state"], "solved", "{view}");
        view["solve"].clone()
    }
}

fn m42() -> (f64, f64) {
    let target = targets::find("M42").unwrap();
    (target.ra_deg, target.dec_deg)
}

fn sep(a: (f64, f64), b: (f64, f64)) -> f64 {
    angular_distance(
        UnitVector::from_radec(a.0.to_radians(), a.1.to_radians()),
        UnitVector::from_radec(b.0.to_radians(), b.1.to_radians()),
    )
    .to_degrees()
        * 3600.0
}

/// US5-S1: the tool is listed with its schema; before a solve it answers the route's words;
/// a malformed ask answers PARTS; the centring answers its view as structured content, holds
/// `motion` for its caller, and the next two calls meet the step under way and the limit; and
/// the centred target stays within 25″ of M42 after the step, with the call logged.
///
/// The rate limit takes a place for every call that gets past its parse: the first call,
/// which finds no solve, takes one, and the wait of 1.1 s frees it. An ask that does not
/// parse takes none. An ask of the wrong shape, `frame_id` alone, takes one, since the route
/// checks its shape after the limit.
#[tokio::test(start_paused = true)]
async fn centre_is_listed_and_moves_the_target_through_the_rate_limit() {
    let log = Log::default();
    let rig = Rig::new(&log).await;

    let listing = body(
        send(
            &rig.app,
            request(json!({"jsonrpc":"2.0","id":7,"method":"tools/list"})),
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

    assert_eq!(
        error(&tool(&rig.app, "centre", json!({})).await),
        NOT_SOLVED
    );
    rig.sleep(1.1).await;
    for arguments in [json!({"frame_id": 1}), json!({"q": 1})] {
        assert_eq!(error(&tool(&rig.app, "centre", arguments).await), PARTS);
    }

    let (x, y) = rig.sim.optics().eyepiece_px;
    rig.state
        .session()
        .set_aim(Some(Aim { x, y }), rig.now())
        .unwrap();
    rig.route("POST", "/api/track", json!({"on": true})).await;
    rig.wait("/api/session", |view| view["track"]["state"] == "tracking")
        .await;
    rig.sleep(30.0).await;
    rig.sim.stop_frames();
    rig.sim
        .push_by(240.0 / 3600.0 / 50f64.to_radians().cos(), 0.0);
    rig.sleep(2.0).await;
    let solve = rig.solve().await;
    let frame = solve["frame_id"].as_u64().unwrap();

    let off = rig.off_m42();
    let answer = tool(&rig.app, "centre", json!({})).await;
    assert_ne!(answer["isError"], true, "{answer}");
    let centred = answer["structuredContent"].clone();
    assert_eq!(centred["frame_id"], frame, "{answer}");
    assert_eq!(centred["moved"], "track", "{answer}");
    let offset = centred["offset_arcsec"].as_f64().unwrap();
    assert!((offset - off).abs() <= 5.0, "{answer}, true offset {off}");

    let moving = session(&rig.app).await;
    assert_eq!(moving["motion"]["by"], "mcp", "{moving}");
    assert_eq!(moving["motion"]["tool"], "centre", "{moving}");

    let moved =
        format!("the telescope moved after imaging frame {frame}: take another and solve it");
    assert_eq!(
        error(&tool(&rig.app, "centre", json!({})).await),
        moved.as_str()
    );
    assert_eq!(error(&tool(&rig.app, "centre", json!({})).await), LIMITED);

    rig.wait("/api/session", |view| view["axis_move"]["state"] == "done")
        .await;
    rig.sleep(12.0).await;
    let off = rig.off_m42();
    assert!(off <= 25.0, "{off} off M42");

    assert!(
        log.text().lines().any(|line| line.contains("mcp centre")),
        "{}",
        log.text()
    );
    assert!(!log.text().contains("test-token"));
}

/// US5-S2: the committed README and glossary carry the centring where FR-012 and FR-013
/// place it, with the words those requirements and plan.md § Interfaces give.
#[test]
fn the_readme_and_the_glossary_carry_the_centring() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let readme =
        fs::read_to_string(root.join("README.md")).expect("the committed README must be readable");
    let glossary = fs::read_to_string(root.join("../../GLOSSARY.md"))
        .expect("the committed glossary must be readable");
    assert!(readme.contains("The imaging camera has sixteen tools of its own"));

    let nudge = readme
        .find("The page's nudge arrows have one tool:")
        .expect("the nudge arrows have their paragraph");
    let paragraph = readme
        .find("Centring from an imaging solve has one tool:")
        .expect("the centring has its paragraph");
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
    let centre = row("| `POST /api/imaging/centre` |").expect("the centre row");
    let goto = row("| `POST /api/goto` |").expect("the goto row");
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

    let nudge = glossary
        .find("**Nudge**:\n")
        .expect("the glossary has the nudge");
    let centring = glossary
        .find("**Centring from an imaging solve**:\n")
        .expect("the glossary has the centring");
    assert!(nudge < centring);
    let entry = &glossary[centring..];
    let entry = &entry[..entry.find("\n\n").unwrap()];
    assert!(
        entry.starts_with(
            "**Centring from an imaging solve**:\nBringing a place, the session's target or a point tapped on the latest solved imaging frame, to the imaging camera's set point from that frame's imaging solve alone: the place Track holds becomes that place, and Track steps the telescope by what the solve says lies between them. It needs no live pointing and is never a sync; the finder camera's first solve within a minute after it takes the aim point from the place Track holds."
        ),
        "{entry}"
    );
    assert!(entry.contains("_Avoid_: recentre"), "{entry}");
}
