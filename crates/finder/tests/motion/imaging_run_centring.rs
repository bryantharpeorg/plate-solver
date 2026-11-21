//! US5-S4–S8 on the run rig. Oracles: the simulator's true imaging centre,
//! true aim point and physical mount, and the session's own Track and relation.
use std::{sync::Arc, time::Duration};

use axum::Router;
use finder::{
    imaging::{
        self,
        run::{console, script::Script, By, Centre, MountState, MountSteps, Steps},
    },
    samples::Samples,
    session::{Aim, Session},
    AppState,
};
use math_core::{angular_distance, UnitVector};
use scope_link::memory;
use scope_node::mount::{Axis, Mount};
use serde_json::{json, Value};
use simulator::{mount::SimMount, SimNode, Simulator, Site, VirtualClock};

use crate::support::{goto::Stars, live as support, Scratch};

const RUN: &str = "/api/imaging/run";
const TICK: Duration = Duration::from_millis(100);

struct Rig {
    app: Router,
    state: AppState,
    sim: Simulator,
    mount: SimMount,
    script: Arc<Script>,
    _scratch: Scratch,
}

impl Rig {
    async fn new(scripted_solver: bool) -> Self {
        let scratch = Scratch::new("run-centring");
        let sim = Simulator::new(
            support::DB.clone(),
            Site::default(),
            VirtualClock::new(support::START),
        );
        let mount = SimMount::new(sim.clock().clone());
        sim.connect_mount(mount.clone());
        sim.push_to(50.0, 120.0);
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
            .with_imaging(scratch.0.join("imaging"))
            .with_imaging_solver(Arc::new(imaging::simulated::SimulatedSolver::new(
                sim.clone(),
            )));
        let script = Script::new();
        let scripted = script.steps();
        let steps = Steps {
            mount: console::mount(&state),
            camera: console::camera(&state),
            solver: if scripted_solver {
                scripted.solver
            } else {
                console::solver(&state)
            },
            focus: scripted.focus,
            stack: scripted.stack,
        };
        let state = state.with_imaging_run_steps(steps);
        let rig = Self {
            app: finder::router(state.clone()),
            state,
            sim,
            mount,
            script,
            _scratch: scratch,
        };
        rig.wait("/api/imaging", |v| v["state"] == "ready").await;
        rig.wait("/api/session", |v| {
            v["model"]["syncs"].as_u64().is_some_and(|n| n > 0)
        })
        .await;
        assert!(rig.get("/api/session").await.get("aim").is_none());
        rig
    }

    fn now(&self) -> f64 {
        self.sim.clock().now()
    }

    async fn ok(&self, method: &str, path: &str, body: Value) -> Value {
        let (status, view) = support::request(&self.app, method, path, body).await;
        assert_eq!(status, 200, "{view}");
        view
    }

    async fn get(&self, path: &str) -> Value {
        self.ok("GET", path, Value::Null).await
    }

    async fn wait(&self, path: &str, check: impl Fn(&Value) -> bool) -> Value {
        for _ in 0..9000 {
            let view = self.get(path).await;
            if check(&view) {
                return view;
            }
            tokio::time::sleep(TICK).await;
        }
        panic!("{path} never reached its state: {}", self.get(path).await);
    }

    async fn start(&self) {
        self.ok("POST", RUN, json!({})).await;
    }

    fn scripted_centres(&self) {
        for _ in 0..4 {
            self.script.push_solve(Ok(Centre {
                offset_arcsec: 400.0,
                tolerance_arcsec: 90.0,
                near_arcsec: 450.0,
                aim_moved: true,
            }));
        }
    }

    fn true_offset(&self) -> f64 {
        let target = finder::targets::find("M42").unwrap();
        let p = self.sim.telescope().eyepiece;
        angular_distance(
            UnitVector::from_radec(p.ra.to_radians(), p.dec.to_radians()),
            UnitVector::from_radec(target.ra_deg.to_radians(), target.dec_deg.to_radians()),
        )
        .to_degrees()
            * 3600.0
    }

    /// Look at the offset immediately on the first stacking sample. The 20″
    /// tolerance is two tracker deadbands; aim pixels allow 032's 1.5 plus 0.5.
    async fn centred(&self, observe_track: bool) -> Value {
        let mut off_at = None;
        let mut resumed_at = None;
        for _ in 0..9000 {
            let session = self.get("/api/session").await;
            let imaging = &session["imaging"];
            assert_ne!(
                imaging["relation"]["last"]["outcome"], "moving",
                "{imaging}"
            );
            let run = &imaging["run"];
            assert_ne!(run["state"], "ended", "{run}");
            if observe_track {
                if run["state"] == "centring" && session["track"]["state"] == "off" {
                    off_at.get_or_insert(self.now());
                }
                if off_at.is_some() && session["track"]["state"] != "off" {
                    resumed_at.get_or_insert(self.now());
                }
            }
            if run["state"] == "stacking" {
                let tolerance = run["centring"]["tolerance_arcsec"].as_f64().unwrap();
                assert_eq!(tolerance, 90.0);
                assert!(run["centring"]["tries"].as_u64().unwrap() >= 1, "{run}");
                let offset = self.true_offset();
                assert!(offset <= tolerance + 20.0, "true offset {offset}: {run}");
                let aim = &session["aim"];
                let (x, y) = self.sim.optics().eyepiece_px;
                assert_eq!(aim["source"], "measured");
                assert!((aim["x"].as_f64().unwrap() - x).abs() <= 2.0, "{aim}");
                assert!((aim["y"].as_f64().unwrap() - y).abs() <= 2.0, "{aim}");
                if observe_track {
                    let elapsed = resumed_at.expect("Track resumed")
                        - off_at.expect("Track was off while centring");
                    assert!(elapsed <= 2.0, "Track stayed off {elapsed}s");
                }
                return run.clone();
            }
            tokio::time::sleep(TICK).await;
        }
        panic!("never stacked: {}", self.get("/api/imaging").await);
    }

    /// Both physical direction (1e-6 degrees) and the mount port must stay at
    /// rest at every 100 ms sample for ten seconds, starting two seconds later.
    async fn rest_after(&self, at: f64) {
        while self.now() < at + 2.0 {
            tokio::time::sleep(TICK).await;
        }
        let first = self.mount.true_direction();
        for _ in 0..100 {
            for axis in [Axis::Azimuth, Axis::Altitude] {
                assert!(!self.mount.state(axis).await.moving, "mount restarted");
            }
            let p = self.mount.true_direction();
            assert!((p.az - first.az).abs() < 1e-6 && (p.alt - first.alt).abs() < 1e-6);
            tokio::time::sleep(TICK).await;
        }
    }

    async fn steady(&self, part: &Arc<dyn MountSteps>) {
        for _ in 0..9000 {
            let read = part.state();
            if read == (MountState::Tracking { steady: true }) {
                return;
            }
            assert!(!matches!(read, MountState::Ended(_)), "{read:?}");
            tokio::time::sleep(TICK).await;
        }
        panic!("mount never steady: {:?}", part.state());
    }
}

#[tokio::test(start_paused = true)]
async fn no_aim_is_measured_and_centred_after_the_first_fresh_pointing() {
    let rig = Rig::new(false).await;
    rig.start().await;
    rig.centred(true).await;
    rig.ok("POST", "/api/stop", Value::Null).await;
}

#[tokio::test(start_paused = true)]
async fn slow_finder_camera_still_centres_in_one_try() {
    let rig = Rig::new(false).await;
    rig.sim.set_frame_interval(Duration::from_secs(2));
    rig.start().await;
    let run = rig.centred(false).await;
    assert_eq!(run["centring"]["tries"], 1);
    rig.ok("POST", "/api/stop", Value::Null).await;
}

#[tokio::test(start_paused = true)]
async fn console_stop_during_centring_wins_and_mount_stays_at_rest() {
    let rig = Rig::new(true).await;
    rig.scripted_centres();
    rig.start().await;
    let view = rig
        .wait("/api/imaging", |v| {
            matches!(
                v["run"]["state"].as_str(),
                Some("centring" | "stacking" | "ended")
            )
        })
        .await;
    assert_eq!(view["run"]["state"], "centring", "{view}");
    let stopped = rig.now();
    rig.ok("POST", "/api/stop", Value::Null).await;
    let view = rig
        .wait("/api/imaging", |v| v["run"]["state"] == "ended")
        .await;
    assert_eq!(view["run"]["ended"]["reason"], "stopped");
    assert!(!rig
        .script
        .calls()
        .iter()
        .any(|call| call.starts_with("stack.start")));
    rig.rest_after(stopped).await;
}

#[tokio::test(start_paused = true)]
async fn twenty_second_exposures_never_fit_an_aim_reading_into_motion() {
    let rig = Rig::new(false).await;
    rig.ok("PUT", "/api/imaging/settings", json!({"exposure_ms":20000}))
        .await;
    rig.start().await;
    rig.centred(false).await;
    rig.ok("POST", "/api/stop", Value::Null).await;
}

#[tokio::test(start_paused = true)]
async fn target_edit_during_centring_does_not_start_track_on_the_new_target() {
    let rig = Rig::new(true).await;
    rig.scripted_centres();
    rig.start().await;
    let view = rig
        .wait("/api/imaging", |v| {
            matches!(
                v["run"]["state"].as_str(),
                Some("centring" | "stacking" | "ended")
            )
        })
        .await;
    assert_eq!(view["run"]["state"], "centring", "{view}");
    rig.ok("PUT", "/api/session", json!({"target":"M45"})).await;
    let view = rig
        .wait("/api/imaging", |v| v["run"]["state"] == "ended")
        .await;
    assert_eq!(view["run"]["ended"]["reason"], "target_changed");
    assert_eq!(view["run"]["ended"]["track_on"], false);
    let ended = rig.now();
    tokio::join!(rig.rest_after(ended), async {
        tokio::time::sleep(Duration::from_secs(10)).await;
        assert_eq!(rig.get("/api/session").await["track"]["state"], "off");
    });
    assert!(!rig
        .script
        .calls()
        .iter()
        .any(|call| call.starts_with("stack.start")));
}

#[tokio::test(start_paused = true)]
async fn repoint_without_live_frames_waits_five_seconds_then_goes_on() {
    let rig = Rig::new(false).await;
    let (x, y) = rig.sim.optics().eyepiece_px;
    rig.state
        .session()
        .set_aim(Some(Aim { x, y }), rig.now())
        .unwrap();
    let part = console::mount(&rig.state);
    part.target().unwrap();
    part.point(By::Page).await.unwrap();
    rig.steady(&part).await;
    rig.sim.stop_frames();
    let started = tokio::time::Instant::now();
    assert_eq!(part.repoint().await, Ok(()));
    let elapsed = started.elapsed().as_secs_f64();
    assert!((4.9..=5.5).contains(&elapsed), "repoint took {elapsed}s");
    assert!(rig.get("/api/imaging").await.get("run").is_none());
    rig.ok("POST", "/api/stop", Value::Null).await;
}
