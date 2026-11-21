//! US8: all five console parts, with the session and 033/035's routes and files
//! as independent oracles. Counts, byte sizes and words below are exact.
use std::{fs, path::PathBuf, sync::Arc, time::Duration};

use axum::Router;
use finder::{
    imaging::{
        run::{console, console::reserve_bytes, Failure},
        simulated::SimulatedSolver,
    },
    samples::Samples,
    session::{Aim, Session},
    AppState,
};
use scope_link::memory;
use serde_json::{json, Value};
use simulator::{mount::SimMount, SimNode, Simulator, Site, VirtualClock};

use crate::support::{goto::Stars, live as support, Scratch};

const RUN: &str = "/api/imaging/run";
const TICK: Duration = Duration::from_millis(100);

struct Rig {
    app: Router,
    state: AppState,
    sim: Simulator,
    root: PathBuf,
    _scratch: Scratch,
}

impl Rig {
    async fn new() -> Self {
        let scratch = Scratch::new("run-stack");
        let root = scratch.0.join("imaging");
        let sim = Simulator::new(
            support::DB.clone(),
            Site::default(),
            VirtualClock::new(support::START),
        );
        sim.connect_mount(SimMount::new(sim.clock().clone()));
        sim.push_to(50.0, 120.0);
        sim.set_focus_error(0.0);
        let node = Arc::new(
            SimNode::new(sim.clone(), "test-token", Default::default())
                .unwrap()
                .with_frame_interval(Duration::from_millis(500)),
        );
        let transport = finder::link::Transport::connector(move || {
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
        let session = Session::with_state_file(scratch.0.join("session.json")).unwrap();
        session
            .edit(
                serde_json::from_value(json!({
                    "node":{"address":"scope.local:1","token":"test-token"},
                    "site":{"lat_deg":40,"lon_deg":-75}, "target":"M42"
                }))
                .unwrap(),
                support::START,
            )
            .unwrap();
        let state = AppState::shared(support::DB.clone(), Samples::default())
            .with_session(session)
            .with_clock(Arc::new(support::ConsoleClock(sim.clock().clone())))
            .with_link(transport)
            .with_live_solver(Arc::new(Stars))
            .with_imaging(&root)
            .with_imaging_solver(Arc::new(SimulatedSolver::new(sim.clone())))
            .with_imaging_run();
        let rig = Self {
            app: finder::router(state.clone()),
            state: state.clone(),
            sim,
            root,
            _scratch: scratch,
        };
        rig.wait("/api/imaging", |v| v["state"] == "ready").await;
        rig.wait("/api/session", |v| {
            v["model"]["syncs"].as_u64().is_some_and(|n| n > 0)
        })
        .await;
        let (x, y) = rig.sim.optics().eyepiece_px;
        state
            .session()
            .set_aim(Some(Aim { x, y }), rig.sim.clock().now())
            .unwrap();
        // Establish tonight's reference through the focus aid's own route.
        rig.ok("POST", "/api/imaging/focus", json!({})).await;
        rig.wait("/api/imaging", |v| {
            v["focus"]["record"]["tonight"].is_object()
        })
        .await;
        rig.ok("DELETE", "/api/imaging/focus", Value::Null).await;
        rig
    }

    async fn ok(&self, method: &str, path: &str, body: Value) -> Value {
        let (status, value) = support::request(&self.app, method, path, body).await;
        assert_eq!(status, 200, "{method} {path}: {value}");
        value
    }
    async fn get(&self, path: &str) -> Value {
        self.ok("GET", path, Value::Null).await
    }
    async fn wait(&self, path: &str, done: impl Fn(&Value) -> bool) -> Value {
        for _ in 0..9000 {
            let view = self.get(path).await;
            if done(&view) {
                return view;
            }
            tokio::time::sleep(TICK).await;
        }
        panic!("{path} did not reach its state: {}", self.get(path).await);
    }
    async fn start(&self, request: Value) {
        self.ok("POST", RUN, request).await;
    }
    async fn stacking(&self) -> Value {
        self.start(json!({})).await;
        let view = self
            .wait("/api/imaging", |v| {
                v["run"]["folder"].is_string() || v["run"]["state"] == "ended"
            })
            .await;
        assert_eq!(view["run"]["state"], "stacking", "{view}");
        view
    }
    async fn ended(&self, reason: &str) -> Value {
        let view = self
            .wait("/api/imaging", |v| v["run"]["state"] == "ended")
            .await;
        assert_eq!(view["run"]["ended"]["reason"], reason, "{view}");
        view
    }
    fn folder(&self, view: &Value) -> PathBuf {
        self.root.join(view["run"]["folder"].as_str().unwrap())
    }
    fn manifest(&self, view: &Value, name: &str) -> Value {
        serde_json::from_slice(&fs::read(self.folder(view).join(name)).unwrap()).unwrap()
    }
}

// S1: the count is read again after 035 has stopped and drained its last sub.
#[tokio::test(start_paused = true)]
async fn whole_run_saves_three_subs_and_its_record_beside_the_stack() {
    let rig = Rig::new().await;
    rig.start(json!({"subs":3})).await;
    let view = rig.ended("count_limit").await;
    let run = &view["run"];
    assert_eq!(
        run["ended"]["message"],
        "Finished: the 3 subs asked for are stacked."
    );
    assert_eq!(run["ended"]["saved"], true);
    let stacked = run["subs"]["stacked"].as_u64().unwrap();
    assert!(stacked >= 3);
    let id = view["stack"]["id"].as_str().unwrap();
    assert!(id.ends_with("-M42"), "{id}");
    assert_eq!(run["folder"], format!("runs/{id}"));
    let folder = rig.folder(&view);
    for name in ["run.json", "stack.json", "stack.fits", "stack.png"] {
        assert!(folder.join(name).is_file(), "missing {name}");
    }
    assert!(!folder.join(".reserve").exists());
    let record = rig.manifest(&view, "run.json");
    let stack = rig.manifest(&view, "stack.json");
    assert_eq!(record["ended"]["reason"], "count_limit");
    assert_eq!(record["subs"]["stacked"], stacked);
    assert_eq!(stack["stacked"], stacked);
    let kept: Vec<_> = stack["subs"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|sub| sub["outcome"] == "stacked")
        .collect();
    assert_eq!(kept.len() as u64, stacked);
    for sub in kept {
        let file = sub["file"].as_str().unwrap();
        assert!(file.starts_with("subs/"), "{file}");
        assert!(folder.join(file).is_file(), "missing {file}");
    }
    let total = |drops: &Value| -> u64 {
        drops
            .as_object()
            .unwrap()
            .values()
            .map(|n| n.as_u64().unwrap())
            .sum()
    };
    assert_eq!(total(&run["subs"]["dropped"]), total(&stack["dropped"]));
    tokio::time::sleep(Duration::from_secs(2)).await;
    let session = rig.get("/api/session").await;
    assert_eq!(session["track"]["state"], "tracking");
    assert_eq!(session["track"]["target"]["messier"], "M42");
    assert_eq!(run["ended"]["track_on"], true);
}

// S2: the saved manifest is the independent list of files to remove.
#[tokio::test(start_paused = true)]
async fn count_limit_can_discard_subs_and_solve_sidecars_after_saving() {
    let rig = Rig::new().await;
    rig.start(json!({"subs":2,"keep_subs":false})).await;
    let view = rig.ended("count_limit").await;
    let folder = rig.folder(&view);
    assert!(folder.join("stack.fits").is_file());
    let stack = rig.manifest(&view, "stack.json");
    assert!(stack["stacked"].as_u64().unwrap() >= 2);
    let listed: Vec<_> = stack["subs"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|sub| sub["file"].as_str())
        .collect();
    assert!(listed.len() >= 2);
    for file in listed {
        assert!(!folder.join(file).exists(), "kept {file}");
        assert!(!folder.join(file).with_extension("solve.json").exists());
    }
}

// S4: stopping through 035's own door ends the run, without zero-count reasons.
#[tokio::test(start_paused = true)]
async fn stack_route_stop_ends_the_run_and_omits_empty_drop_reasons() {
    let rig = Rig::new().await;
    rig.stacking().await;
    rig.wait("/api/imaging", |v| {
        v["stack"]["stacked"].as_u64().is_some_and(|n| n >= 1)
    })
    .await;
    rig.ok("DELETE", "/api/imaging/stack", Value::Null).await;
    let view = rig.ended("stopped").await;
    for count in view["run"]["subs"]["dropped"].as_object().unwrap().values() {
        assert!(count.as_u64().unwrap() > 0);
    }
}

// S5: a file where the stack's parent directory should be makes saving fail.
#[tokio::test(start_paused = true)]
async fn stack_folder_failure_ends_as_save_failed() {
    let rig = Rig::new().await;
    fs::write(rig.root.join("runs"), b"not a directory").unwrap();
    rig.start(json!({})).await;
    let view = rig.ended("save_failed").await;
    assert!(
        view["run"]["ended"]["message"]
            .as_str()
            .unwrap()
            .starts_with(
            "Ended: the console could not save to its imaging folder: cannot make the stack folder"
        ),
        "{view}"
    );
}

// S6: either Track or the console's stack can observe the lost link first.
#[tokio::test(start_paused = true)]
async fn lost_scope_link_finishes_the_run_with_the_stack_stopped() {
    let rig = Rig::new().await;
    rig.stacking().await;
    rig.sim.set_unresponsive(true);
    let view = rig
        .wait("/api/imaging", |v| v["run"]["state"] == "ended")
        .await;
    match view["run"]["ended"]["reason"].as_str().unwrap() {
        "link_lost" => {}
        "camera_failed" => assert!(
            view["run"]["ended"]["message"]
                .as_str()
                .unwrap()
                .contains("the scope node is not connected"),
            "{view}"
        ),
        _ => panic!("unexpected ending: {view}"),
    }
    assert_eq!(view["stack"]["state"], "ended");
}

// S7: arithmetic is typed from the plan, independently of reserve_bytes.
#[test]
fn reserve_size_uses_rounded_up_binned_sides_and_zero_means_one() {
    assert_eq!(reserve_bytes(4944, 3284, 1), 260_318_656);
    assert_eq!(reserve_bytes(4944, 3284, 3), 43_845_616);
    assert_eq!(reserve_bytes(1236, 822, 0), 32_017_096);
}

#[tokio::test(start_paused = true)]
async fn reserve_is_present_after_start_and_freed_before_the_stack_is_saved() {
    let rig = Rig::new().await;
    let view = rig.stacking().await;
    let camera = &view["camera"];
    assert_eq!(camera["width"], 1236);
    assert_eq!(camera["height"], 822);
    assert_eq!(view["settings"]["binning"], 1);
    let binning = view["settings"]["binning"].as_u64().unwrap().max(1);
    let width = camera["width"].as_u64().unwrap().div_ceil(binning);
    let height = camera["height"].as_u64().unwrap().div_ceil(binning);
    let expected = 15 * width * height + 16_777_216;
    assert_eq!(expected, 32_017_096);
    let folder = rig.folder(&view);
    let reserve = folder.join(".reserve");
    assert_eq!(fs::metadata(&reserve).unwrap().len(), expected);
    // The reserved file contains zeros, including its final partial chunk.
    assert!(fs::read(&reserve).unwrap().iter().all(|&byte| byte == 0));
    rig.ok("DELETE", RUN, Value::Null).await;
    rig.ended("stopped").await;
    assert!(!reserve.exists());
    assert!(folder.join("stack.json").is_file());
}

// S8: stopping the run saves the stack, leaves Track on, and returns the lease.
#[tokio::test(start_paused = true)]
async fn delete_run_saves_stack_and_returns_camera_without_stopping_track() {
    let rig = Rig::new().await;
    rig.stacking().await;
    rig.ok("DELETE", RUN, Value::Null).await;
    let view = rig.ended("stopped").await;
    assert_eq!(view["stack"]["state"], "ended");
    assert!(rig.folder(&view).join("run.json").is_file());
    assert_eq!(rig.get("/api/session").await["track"]["state"], "tracking");
    rig.ok("POST", "/api/imaging/frame", Value::Null).await;
}

// S9: the run's source cannot reach a mount except through the console's Track.
#[test]
fn run_sources_reach_the_mount_through_track_only() {
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/imaging");
    for file in ["run.rs", "run/script.rs", "run/store.rs", "run/console.rs"] {
        let source = fs::read_to_string(src.join(file)).unwrap();
        for forbidden in [
            "crate::goto",
            "crate::tracking",
            "crate::ephemeris",
            "scope_link",
            "unsafe",
        ] {
            assert!(!source.contains(forbidden), "{file} contains {forbidden}");
        }
        for line in source.lines().filter(|line| line.contains("model::")) {
            assert_eq!(file, "run/console.rs");
            assert!(line.contains("separation"), "{line}");
        }
    }
}

// FR-033: 035 joins an existing stack; the run part must refuse to adopt it.
#[tokio::test(start_paused = true)]
async fn part_refuses_an_existing_stack_without_stopping_or_removing_it() {
    let rig = Rig::new().await;
    rig.ok("POST", "/api/imaging/stack", json!({})).await;
    let view = rig
        .wait("/api/imaging", |v| {
            v["stack"]["stacked"].as_u64().is_some_and(|n| n > 0)
        })
        .await;
    let id = view["stack"]["id"].clone();
    let part = console::stack(&rig.state);
    let target = console::mount(&rig.state).target().unwrap();
    assert_eq!(
        part.start(&target).await,
        Err(Failure::Other("another stack is already running".into()))
    );
    let progress = part.progress();
    assert!(!progress.running);
    assert_eq!(progress.stacked, 0);
    assert!(progress.dropped.is_empty());
    part.stop(false).await.unwrap();
    let view = rig.get("/api/imaging").await;
    assert_eq!(view["stack"]["id"], id);
    assert_eq!(view["stack"]["state"], "stacking");
    let folder = rig.root.join("runs").join(id.as_str().unwrap());
    assert!(!folder.join(".reserve").exists());
    rig.ok("DELETE", "/api/imaging/stack", Value::Null).await;
    let manifest: Value =
        serde_json::from_slice(&fs::read(folder.join("stack.json")).unwrap()).unwrap();
    for sub in manifest["subs"].as_array().unwrap() {
        if let Some(file) = sub["file"].as_str() {
            assert!(folder.join(file).is_file());
        }
    }
}

// FR-033: a replacement belongs to its client; the part retains its last counts.
#[tokio::test(start_paused = true)]
async fn part_preserves_last_counts_and_subs_when_another_stack_replaces_its_own() {
    let rig = Rig::new().await;
    let part = console::stack(&rig.state);
    let target = console::mount(&rig.state).target().unwrap();
    let folder = rig.root.join(part.start(&target).await.unwrap());
    rig.wait("/api/imaging", |v| {
        v["stack"]["stacked"].as_u64().is_some_and(|n| n > 0)
    })
    .await;
    let last = part.progress();
    assert!(last.running && last.stacked > 0);
    rig.ok("DELETE", "/api/imaging/stack", Value::Null).await;
    let manifest: Value =
        serde_json::from_slice(&fs::read(folder.join("stack.json")).unwrap()).unwrap();
    let files: Vec<_> = manifest["subs"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|sub| sub["file"].as_str())
        .flat_map(|file| {
            let sub = folder.join(file);
            [sub.clone(), sub.with_extension("solve.json")]
        })
        .filter(|path| path.is_file())
        .collect();
    assert!(!files.is_empty());
    let replacement = rig.ok("POST", "/api/imaging/stack", json!({})).await;
    let progress = part.progress();
    assert!(!progress.running);
    assert_eq!(progress.stacked, last.stacked);
    assert_eq!(progress.dropped, last.dropped);
    part.stop(false).await.unwrap();
    assert!(!folder.join(".reserve").exists());
    let view = rig.get("/api/imaging").await;
    assert_eq!(view["stack"]["id"], replacement["stack"]["id"]);
    assert_eq!(view["stack"]["state"], "stacking");
    for file in files {
        assert!(file.is_file(), "removed {}", file.display());
    }
    rig.ok("DELETE", "/api/imaging/stack", Value::Null).await;
}
