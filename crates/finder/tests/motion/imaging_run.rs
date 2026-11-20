//! US4: console parts on the run rig. Oracles are simulator truth, the session,
//! and 031's own routes. All polling is 100 ms of injected, paused time.
use std::{sync::Arc, time::Duration};

use axum::{http::StatusCode, Router};
use finder::{
    imaging::{
        self,
        run::{console, script::Script, By, Ending, Failure, MountState, MountSteps, Steps},
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

use crate::support::{
    goto::{sun_truth, Stars},
    live as support, Scratch,
};

const RUN: &str = "/api/imaging/run";
const TICK: Duration = Duration::from_millis(100);

struct Rig {
    app: Router,
    state: AppState,
    sim: Simulator,
    mount: SimMount,
    script: Arc<Script>,
    scratch: Scratch,
}

impl Rig {
    async fn new() -> Self {
        Self::at(support::START, "M42").await
    }

    async fn at(start: f64, target: &str) -> Self {
        let scratch = Scratch::new("run-parts");
        let sim = Simulator::new(
            support::DB.clone(),
            Site::default(),
            VirtualClock::new(start),
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
                    "site":{"lat_deg":40,"lon_deg":-75}, "target":target
                }))
                .unwrap(),
                start,
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
            solver: console::solver(&state),
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
            scratch,
        };
        rig.wait("/api/imaging", |v| v["state"] == "ready").await;
        rig.wait("/api/session", |v| {
            v["model"]["syncs"].as_u64().is_some_and(|n| n > 0)
        })
        .await;
        // The simulator's known finder pixel looking through the main telescope.
        let (x, y) = rig.sim.optics().eyepiece_px;
        rig.state
            .session()
            .set_aim(Some(Aim { x, y }), rig.now())
            .unwrap();
        rig
    }

    fn now(&self) -> f64 {
        self.sim.clock().now()
    }
    async fn send(&self, method: &str, path: &str, body: Value) -> (StatusCode, Value) {
        support::request(&self.app, method, path, body).await
    }
    async fn ok(&self, method: &str, path: &str, body: Value) -> Value {
        let (status, answer) = self.send(method, path, body).await;
        assert_eq!(status, 200, "{answer}");
        answer
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
        panic!("{path} did not reach its state: {}", self.get(path).await);
    }
    async fn edit(&self, edit: Value) -> Value {
        self.ok("PUT", "/api/session", edit).await
    }
    async fn start(&self) -> Value {
        let view = self.ok("POST", RUN, json!({})).await;
        assert_eq!(view["run"]["state"], "slewing");
        view
    }
    async fn stacking(&self) -> Value {
        self.start().await;
        let view = self
            .wait("/api/imaging", |v| {
                matches!(v["run"]["state"].as_str(), Some("stacking" | "ended"))
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
    /// Great-circle simulator truth, independent of the console's solve and model.
    fn true_offset(&self, name: &str) -> f64 {
        let t = finder::targets::find(name).unwrap();
        let p = self.sim.telescope().eyepiece;
        angular_distance(
            UnitVector::from_radec(p.ra.to_radians(), p.dec.to_radians()),
            UnitVector::from_radec(t.ra_deg.to_radians(), t.dec_deg.to_radians()),
        )
        .to_degrees()
            * 3600.0
    }
    /// Begin 2 s after Stop, then check both the mount port and physical truth
    /// at every 100 ms for 10 s. Rest permits no physical movement (1e-6 degrees).
    async fn rest_after(&self, stopped: f64) {
        while self.now() < stopped + 2.0 {
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
    async fn steady(&self, part: &Arc<dyn MountSteps>, seconds: usize) {
        for _ in 0..seconds * 10 {
            let read = part.state();
            if read == (MountState::Tracking { steady: true }) {
                return;
            }
            assert!(!matches!(read, MountState::Ended(_)), "{read:?}");
            tokio::time::sleep(TICK).await;
        }
        panic!("mount never steady: {:?}", part.state());
    }
    async fn no_start(&self, words: &str, contains: bool) {
        let (status, view) = self.send("POST", RUN, json!({})).await;
        assert_eq!(status, 409, "{view}");
        if contains {
            assert!(view["error"].as_str().unwrap().contains(words), "{view}");
        } else {
            assert_eq!(view["error"], words);
        }
        let session = self.get("/api/session").await;
        assert_ne!(session["track"]["wanted"], true, "{session}");
        assert!(self.get("/api/imaging").await.get("run").is_none());
        assert!(!self.scratch.0.join("imaging/runs").exists());
    }
    async fn frame_saved(&self) {
        self.wait("/api/imaging", |v| v["frame"]["state"] == "saved")
            .await;
    }
    async fn frame_route(&self) {
        self.ok("POST", "/api/imaging/frame", Value::Null).await;
        self.frame_saved().await;
    }
}

// S1: the 90 arcsecond floor, simulator truth and 040's whole fit reach.
#[tokio::test(start_paused = true)]
async fn centred_run_points_through_track_and_waits_for_the_fit_reach() {
    let rig = Rig::new().await;
    rig.start().await;
    let motion = rig.get("/api/session").await;
    assert_eq!(motion["motion"]["by"], "page");
    assert_eq!(motion["motion"]["tool"], "imaging_run");
    let mut armed = 0;
    let mut tracking = None;
    let mut stacked = None;
    for _ in 0..9000 {
        let session = rig.get("/api/session").await;
        let view = rig.get("/api/imaging").await;
        if session["track"]["state"] == "armed" {
            armed += 1;
            assert_eq!(view["run"]["state"], "slewing");
        }
        if session["track"]["state"] == "tracking" {
            tracking.get_or_insert(rig.now());
        }
        if let Some(first) = tracking {
            assert_eq!(session["track"]["state"], "tracking");
            if rig.now() - first <= 11.5 {
                assert!(
                    matches!(view["run"]["state"].as_str(), Some("slewing" | "tracking")),
                    "{view}"
                );
            }
        }
        if view["run"]["state"] == "stacking" {
            stacked = Some(view);
            break;
        }
        assert_ne!(view["run"]["state"], "ended", "{view}");
        tokio::time::sleep(TICK).await;
    }
    assert!(armed > 0);
    assert!(tracking.is_some());
    let view = stacked.expect("run never stacked");
    let centre = &view["run"]["centring"];
    assert_eq!(centre["tries"], 0);
    assert_eq!(centre["tolerance_arcsec"], 90.0);
    let offset = centre["offset_arcsec"].as_f64().unwrap();
    assert!(offset <= 90.0, "{centre}");
    let truth = rig.true_offset("M42");
    assert!(truth <= 110.0, "{truth}");
    assert!(
        (offset - truth).abs() <= 30.0,
        "solve {offset}, truth {truth}"
    );
    assert_eq!(view["frame"]["purpose"], "run");
    rig.ok("POST", "/api/stop", Value::Null).await;
}

// S2: each rejected start leaves no run, record or wanted Track, and returns the lease.
#[tokio::test(start_paused = true)]
async fn start_refusals_leave_no_run_or_lease() {
    let rig = Rig::new().await;
    rig.edit(json!({"target":null})).await;
    rig.no_start("no target is selected", false).await;
    rig.frame_route().await;
    rig.edit(json!({"target":"M8"})).await;
    assert_eq!(
        finder::pushto::class(
            finder::targets::find("M8").unwrap(),
            rig.state.session().snapshot(rig.now()).site,
            15.0,
            rig.now()
        ),
        finder::pushto::Class::BelowHorizon
    );
    rig.no_start("M8 is below the horizon", true).await;
    rig.frame_route().await;
    rig.edit(json!({"target":"M42"})).await;
    rig.ok(
        "PUT",
        "/api/imaging/settings",
        json!({"roi":{"x":0,"y":0,"width":412,"height":274}}),
    )
    .await;
    rig.no_start(
        "the imaging camera has a region of interest set: clear it before an imaging run",
        false,
    )
    .await;
    rig.ok("PUT", "/api/imaging/settings", json!({"roi":null}))
        .await;
    rig.frame_route().await;
    rig.sim.set_unresponsive(true);
    rig.wait("/api/imaging", |v| v["state"] != "ready").await;
    rig.no_start("the scope node is not connected", false).await;
    rig.sim.set_unresponsive(false);
    rig.wait("/api/imaging", |v| v["state"] == "ready").await;
    assert_ne!(rig.get("/api/session").await["track"]["wanted"], true);
    assert!(rig.get("/api/imaging").await.get("run").is_none());
    assert!(!rig.scratch.0.join("imaging/runs").exists());
    rig.frame_route().await;
}

// S3: Stop ends the run, saves the scripted stack once, and never restarts metal.
#[tokio::test(start_paused = true)]
async fn stop_while_stacking_stops_stack_once_and_mount_stays_at_rest() {
    let rig = Rig::new().await;
    rig.stacking().await;
    let stopped = rig.now();
    rig.ok("POST", "/api/stop", Value::Null).await;
    rig.ended("stopped").await;
    assert_eq!(
        rig.script
            .calls()
            .iter()
            .filter(|c| c.starts_with("stack.stop"))
            .count(),
        1
    );
    rig.rest_after(stopped).await;
}

// S4: each guard has its own rig, avoiding state left by a previous ending.
#[tokio::test(start_paused = true)]
async fn minimum_altitude_ends_too_low_and_stops_the_mount() {
    let rig = Rig::new().await;
    rig.stacking().await;
    let stopped = rig.now();
    rig.edit(json!({"min_alt_deg":80.0})).await;
    rig.ended("too_low").await;
    rig.rest_after(stopped).await;
}

#[tokio::test(start_paused = true)]
async fn link_loss_ends_the_run() {
    let rig = Rig::new().await;
    rig.stacking().await;
    rig.sim.set_unresponsive(true);
    rig.ended("link_lost").await;
}

#[tokio::test(start_paused = true)]
async fn target_edit_ends_run_and_leaves_track_on_its_original_target() {
    let rig = Rig::new().await;
    rig.stacking().await;
    rig.edit(json!({"target":"M45"})).await;
    rig.ended("target_changed").await;
    let s = rig.get("/api/session").await;
    assert_eq!(s["track"]["state"], "tracking");
    assert_eq!(s["track"]["target"]["messier"], "M42");
    rig.ok("POST", "/api/stop", Value::Null).await;
}

#[tokio::test(start_paused = true)]
async fn lasting_push_ends_run_while_track_follows_the_push() {
    let rig = Rig::new().await;
    rig.stacking().await;
    let first = rig.now();
    rig.mount.push_by(0.0, -2.0);
    rig.ended("pushed_away").await;
    assert!(rig.now() - first <= 60.0);
    let s = rig.get("/api/session").await;
    assert_eq!(s["track"]["state"], "tracking");
    assert!(matches!(
        s["track"]["push"]["state"].as_str(),
        Some("moving" | "following")
    ));
    rig.ok("POST", "/api/stop", Value::Null).await;
}

#[tokio::test(start_paused = true)]
async fn continuously_moving_push_ends_after_ten_seconds_of_grace() {
    let rig = Rig::new().await;
    rig.stacking().await;
    let first = rig.now();
    let mut ended = false;
    let mut moving = false;
    for _ in 0..20 {
        rig.mount.push_by(0.0, -0.1);
        for _ in 0..10 {
            tokio::time::sleep(TICK).await;
            let s = rig.get("/api/session").await;
            moving |= s["track"]["push"]["state"] == "moving";
            let v = rig.get("/api/imaging").await;
            if v["run"]["state"] == "ended" {
                assert_eq!(v["run"]["ended"]["reason"], "pushed_away");
                assert_eq!(s["track"]["state"], "tracking");
                assert_eq!(s["track"]["push"]["state"], "moving");
                ended = true;
                break;
            }
        }
        if ended {
            break;
        }
    }
    assert!(ended && moving);
    assert!(
        (10.0..=15.0).contains(&(rig.now() - first)),
        "elapsed {}",
        rig.now() - first
    );
    rig.ok("POST", "/api/stop", Value::Null).await;
}

#[tokio::test(start_paused = true)]
async fn dawn_ends_run_sun_up_and_stops_the_mount() {
    let (mut low, mut high) = (support::START, support::START + 86400.0 / 2.0);
    assert!(sun_truth(40.0, -75.0, low).0 < -0.83);
    assert!(sun_truth(40.0, -75.0, high).0 > -0.83);
    for _ in 0..50 {
        let mid = (low + high) / 2.0;
        if sun_truth(40.0, -75.0, mid).0 < -0.83 {
            low = mid;
        } else {
            high = mid;
        }
    }
    let rig = Rig::at(high - 120.0, "M81").await;
    rig.start().await;
    rig.ended("sun_up").await;
    rig.rest_after(rig.now()).await;
}

#[tokio::test(start_paused = true)]
async fn run_holds_camera_while_slewing_and_stop_returns_it() {
    let rig = Rig::new().await;
    rig.start().await;
    assert_eq!(rig.get("/api/imaging").await["run"]["state"], "slewing");
    let (status, v) = rig.send("POST", "/api/imaging/frame", Value::Null).await;
    assert_eq!(status, 409);
    assert_eq!(v["error"], "the imaging camera is held by the imaging run");
    rig.ok("POST", "/api/stop", Value::Null).await;
    rig.ended("stopped").await;
    rig.frame_route().await;
}

#[tokio::test(start_paused = true)]
async fn unsteady_transit_keeps_the_run_waiting_for_the_tracker() {
    let rig = Rig::at(support::START + 3242.7 - 15.0, "M42").await;
    rig.start().await;
    rig.wait("/api/imaging", |v| v["run"]["state"] == "tracking")
        .await;
    tokio::time::sleep(Duration::from_secs(30)).await;
    let v = rig.get("/api/imaging").await;
    assert_eq!(v["run"]["state"], "tracking");
    assert_eq!(
        v["run"]["message"],
        "Tracking M42. Waiting for the mount to settle; nothing to do."
    );
    rig.ok("POST", "/api/stop", Value::Null).await;
}

#[tokio::test(start_paused = true)]
async fn push_undone_before_grace_does_not_end_the_run() {
    let rig = Rig::new().await;
    rig.stacking().await;
    let first = rig.now();
    rig.mount.push_by(0.0, -0.1);
    let mut moving = false;
    for _ in 0..30 {
        let s = rig.get("/api/session").await;
        moving |= s["track"]["push"]["state"] == "moving";
        tokio::time::sleep(TICK).await;
    }
    assert!(
        moving,
        "negative control: the session must observe the push"
    );
    rig.mount.push_by(0.0, 0.1);
    while rig.now() - first < 30.0 {
        tokio::time::sleep(TICK).await;
    }
    let v = rig.get("/api/imaging").await;
    assert!(
        matches!(
            v["run"]["state"].as_str(),
            Some("stacking" | "paused_by_tracker")
        ),
        "{v}"
    );
    assert!(rig.get("/api/session").await["track"].get("push").is_none());
    rig.ok("POST", "/api/stop", Value::Null).await;
}

#[tokio::test(start_paused = true)]
async fn delete_while_slewing_stops_the_mount_without_an_exposure() {
    let rig = Rig::new().await;
    rig.start().await;
    assert_eq!(rig.get("/api/imaging").await["run"]["state"], "slewing");
    let stopped = rig.now();
    let v = rig.ok("DELETE", RUN, Value::Null).await;
    assert_eq!(v["run"]["state"], "ended");
    assert_eq!(v["run"]["ended"]["reason"], "stopped");
    assert_eq!(
        v["run"]["ended"]["message"],
        "Stopped: the mount was stopped."
    );
    assert_eq!(v["run"]["ended"]["track_on"], false);
    assert_eq!(rig.get("/api/session").await["track"]["state"], "off");
    assert!(v.get("frame").is_none());
    rig.rest_after(stopped).await;
}

// S5: calls on the part alone always finish before the next route is asked.
#[tokio::test(start_paused = true)]
async fn mount_part_points_repoints_and_refuses_to_restart_after_stop() {
    let rig = Rig::new().await;
    let part = console::mount(&rig.state);
    assert_eq!(part.target().unwrap().name, "M42");
    assert_eq!(part.point(By::Page).await, Ok(()));
    rig.steady(&part, 900).await;
    assert_eq!(part.repoint().await, Ok(()));
    rig.steady(&part, 120).await;
    let s = rig.get("/api/session").await;
    assert_eq!(s["motion"]["by"], "page");
    assert_eq!(s["motion"]["tool"], "imaging_run");
    let stopped = rig.now();
    rig.ok("POST", "/api/stop", Value::Null).await;
    assert_eq!(part.state(), MountState::Ended(Ending::Stopped));
    assert_eq!(part.repoint().await, Err(Failure::Stopped));
    assert_eq!(rig.get("/api/session").await["track"]["state"], "off");
    rig.rest_after(stopped).await;
}

#[tokio::test(start_paused = true)]
async fn repoint_after_target_edit_leaves_track_off() {
    let rig = Rig::new().await;
    let part = console::mount(&rig.state);
    part.target().unwrap();
    part.point(By::Page).await.unwrap();
    rig.steady(&part, 900).await;
    rig.edit(json!({"target":"M45"})).await;
    assert_eq!(part.repoint().await, Ok(()));
    assert_eq!(rig.get("/api/session").await["track"]["state"], "off");
    tokio::time::sleep(Duration::from_secs(10)).await;
    assert_eq!(rig.get("/api/session").await["track"]["state"], "off");
    assert_eq!(part.state(), MountState::Ended(Ending::TargetChanged));
}

#[tokio::test(start_paused = true)]
async fn point_refuses_a_target_other_than_the_one_it_answered() {
    let rig = Rig::new().await;
    let part = console::mount(&rig.state);
    assert_eq!(part.target().unwrap().name, "M42");
    rig.edit(json!({"target":"M45"})).await;
    assert_eq!(
        part.point(By::Page).await,
        Err("the target was changed".into())
    );
    assert_ne!(rig.get("/api/session").await["track"]["wanted"], true);
}

#[tokio::test(start_paused = true)]
async fn track_on_another_target_is_detected_even_when_session_target_returns() {
    let rig = Rig::new().await;
    let part = console::mount(&rig.state);
    part.point(By::Page).await.unwrap();
    rig.steady(&part, 900).await;
    rig.edit(json!({"target":"M45"})).await;
    rig.ok("POST", "/api/track", json!({"on":true})).await;
    rig.edit(json!({"target":"M42"})).await;
    assert_eq!(part.state(), MountState::Ended(Ending::TargetChanged));
    rig.ok("POST", "/api/stop", Value::Null).await;
}

#[tokio::test(start_paused = true)]
async fn track_started_again_after_stop_does_not_revive_the_part() {
    let rig = Rig::new().await;
    let part = console::mount(&rig.state);
    part.point(By::Page).await.unwrap();
    rig.steady(&part, 900).await;
    rig.ok("POST", "/api/stop", Value::Null).await;
    tokio::time::sleep(Duration::from_secs(1)).await;
    rig.ok("POST", "/api/track", json!({"on":true})).await;
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert_ne!(rig.get("/api/session").await["track"]["state"], "off");
    assert_eq!(part.state(), MountState::Ended(Ending::Stopped));
    rig.ok("POST", "/api/stop", Value::Null).await;
}

#[tokio::test(start_paused = true)]
async fn repoint_passes_tracks_own_refusal_on_as_mount_failure() {
    let rig = Rig::new().await;
    let part = console::mount(&rig.state);
    part.point(By::Page).await.unwrap();
    rig.steady(&part, 900).await;
    rig.edit(json!({"min_alt_deg":80.0})).await;
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert_eq!(
        part.repoint().await,
        Err(Failure::Mount("M42 is below the minimum altitude".into()))
    );
}

#[tokio::test(start_paused = true)]
async fn camera_and_solver_report_a_quarter_sensor_as_near() {
    let rig = Rig::new().await;
    let mount = console::mount(&rig.state);
    let target = mount.target().unwrap();
    mount.point(By::Page).await.unwrap();
    rig.steady(&mount, 900).await;
    tokio::time::sleep(Duration::from_secs(12)).await;
    let camera = console::camera(&rig.state);
    camera.hold().unwrap();
    let frame = camera.take().await.unwrap();
    camera.release();
    let centre = console::solver(&rig.state)
        .solve(&frame, &target)
        .await
        .unwrap();
    // 822 px at the simulator's optical scale gives a quarter-side of 448.9 arcseconds.
    assert!((centre.near_arcsec - 449.0).abs() <= 1.0, "{centre:?}");
    rig.ok("POST", "/api/stop", Value::Null).await;
}

// S6: a single frame holds no lease: hold must acquire it, then return it.
#[tokio::test(start_paused = true)]
async fn in_flight_single_frame_refuses_run_and_returns_the_lease() {
    let rig = Rig::new().await;
    rig.ok("PUT", "/api/imaging/settings", json!({"exposure_ms":10000}))
        .await;
    rig.ok("POST", "/api/imaging/frame", Value::Null).await;
    tokio::time::sleep(Duration::from_secs(1)).await;
    rig.no_start("an exposure is in progress", false).await;
    rig.frame_saved().await;
    rig.frame_route().await;
}
