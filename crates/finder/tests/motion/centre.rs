//! US1-S1: centring from an imaging solve with no finder camera solves.
//! Oracles: the simulator's imaging truth, hand push and true eyepiece pixel,
//! M42's catalogue place, and the session's motion, aim and model before the
//! press. Angular tolerances below are arcseconds; time is injected and paused.
use crate::support::{goto::Stars, live as support, Scratch};
use axum::Router;
use finder::{
    imaging::{
        run::{console, script::Script, Steps},
        simulated::SimulatedSolver,
    },
    session::{Aim, Session},
    targets, AppState,
};
use math_core::{angular_distance, UnitVector};
use scope_link::memory;
use serde_json::{json, Value};
use simulator::{mount::SimMount, ImagingOptics, SimNode, Simulator, Site, VirtualClock};
use std::{sync::Arc, time::Duration};

pub(super) const CENTRE: &str = "/api/imaging/centre";
pub(super) const FINDER_ARCSEC_PER_PX: f64 = 12.0 * 3600.0 / 1456.0;

pub(super) enum Cause {
    Push,
    Aim,
}

pub(super) struct Rig {
    pub(super) app: Router,
    pub(super) state: AppState,
    pub(super) sim: Simulator,
    _scratch: Scratch,
}

impl Rig {
    pub(super) async fn new(angle: f64) -> Self {
        let scratch = Scratch::new("centre");
        let sim = Simulator::new(
            support::DB.clone(),
            Site::default(),
            VirtualClock::new(support::START),
        );
        sim.connect_mount(SimMount::new(sim.clock().clone()));
        sim.push_to(50.0, 120.0);
        sim.set_imaging_optics(ImagingOptics {
            angle_deg: angle,
            ..ImagingOptics::default()
        });
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
                    "node": {"address": "scope.local:1", "token": "test-token"},
                    "site": {"lat_deg": 40, "lon_deg": -75}, "target": "M42"
                }))
                .unwrap(),
                support::START,
            )
            .unwrap();
        let state = AppState::shared(support::DB.clone(), Default::default())
            .with_session(session)
            .with_clock(Arc::new(support::ConsoleClock(sim.clock().clone())))
            .with_link(transport)
            .with_live_solver(Arc::new(Stars))
            .with_imaging(scratch.0.join("imaging"))
            .with_imaging_solver(Arc::new(SimulatedSolver::new(sim.clone())));
        let scripted = Script::new().steps();
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
            _scratch: scratch,
        };
        rig.wait("/api/imaging", |v| v["state"] == "ready").await;
        rig.wait("/api/session", |v| {
            v["model"]["syncs"].as_u64().is_some_and(|n| n > 0)
        })
        .await;
        rig
    }

    pub(super) fn now(&self) -> f64 {
        self.sim.clock().now()
    }

    pub(super) async fn call(&self, method: &str, path: &str, body: Value) -> Value {
        let (status, answer) = super::nudge::write(&self.app, method, path, body).await;
        assert_eq!(status, 200, "{method} {path}: {answer}");
        answer
    }

    pub(super) async fn get(&self, path: &str) -> Value {
        self.call("GET", path, Value::Null).await
    }

    /// Poll every 100 ms of virtual time, bounded at 300 s.
    pub(super) async fn wait(&self, path: &str, done: impl Fn(&Value) -> bool) -> Value {
        for _ in 0..3000 {
            let view = self.get(path).await;
            if done(&view) {
                return view;
            }
            self.sleep(0.1).await;
        }
        panic!("{path} never reached its state: {}", self.get(path).await);
    }

    pub(super) async fn sleep(&self, seconds: f64) {
        tokio::time::sleep(Duration::from_secs_f64(seconds)).await;
    }

    /// Sensor centre's distance from a J2000 place, from simulator truth.
    pub(super) fn off(&self, place: (f64, f64)) -> f64 {
        let truth = self.sim.imaging_truth();
        sep((truth.ra_deg, truth.dec_deg), place)
    }

    pub(super) async fn solve(&self) -> Value {
        let view = self.call("POST", "/api/imaging/frame", Value::Null).await;
        let id = view["frame"]["id"].as_u64().unwrap();
        self.wait("/api/imaging", |v| {
            v["frame"]["id"] == id && v["frame"]["state"] == "saved"
        })
        .await;
        self.call("POST", "/api/imaging/solve", Value::Null).await;
        let view = self
            .wait("/api/imaging", |v| {
                v["solve"]["frame_id"] == id && v["solve"]["state"] != "solving"
            })
            .await;
        assert_eq!(view["solve"]["state"], "solved", "{view}");
        view["solve"].clone()
    }

    pub(super) async fn track_on(&self) {
        self.call("POST", "/api/track", json!({"on": true})).await;
        self.wait("/api/session", |v| v["track"]["state"] == "tracking")
            .await;
    }

    pub(super) async fn rests(&self) -> Value {
        self.wait("/api/session", |v| {
            v["axis_move"]["state"] != "moving" && v["track"]["state"] == "tracking"
        })
        .await;
        self.sleep(12.0).await;
        self.get("/api/session").await
    }

    /// Set the true or displaced finder aim before Track on; stop only the
    /// finder frames, then optionally push the tube by hand in horizon axes.
    pub(super) async fn displace(&self, cause: Cause, size: f64, angle: f64) {
        let (mut x, mut y) = self.sim.optics().eyepiece_px;
        let angle = angle.to_radians();
        if matches!(cause, Cause::Aim) {
            x += size * angle.cos() / FINDER_ARCSEC_PER_PX;
            y += size * angle.sin() / FINDER_ARCSEC_PER_PX;
        }
        self.state
            .session()
            .set_aim(Some(Aim { x, y }), self.now())
            .unwrap();
        self.track_on().await;
        self.sleep(30.0).await;
        self.sim.stop_frames();
        self.sleep(2.0).await;
        if matches!(cause, Cause::Push) {
            let alt = self.sim.telescope().camera.alt.to_radians();
            self.sim.push_by(
                size * angle.cos() / 3600.0 / alt.cos(),
                size * angle.sin() / 3600.0,
            );
        }
        self.sleep(2.0).await;
    }

    pub(super) fn held(&self) -> (f64, f64) {
        self.state.session().tracker().unwrap().target().unwrap()
    }
}

pub(super) fn m42() -> (f64, f64) {
    let target = targets::find("M42").unwrap();
    (target.ra_deg, target.dec_deg)
}

pub(super) fn sep(a: (f64, f64), b: (f64, f64)) -> f64 {
    angular_distance(
        UnitVector::from_radec(a.0.to_radians(), a.1.to_radians()),
        UnitVector::from_radec(b.0.to_radians(), b.1.to_radians()),
    )
    .to_degrees()
        * 3600.0
}

async fn centres(angle: f64, cause: Cause, size: f64, direction: f64) {
    let rig = Rig::new(angle).await;
    rig.displace(cause, size, direction).await;
    let off = rig.off(m42());
    assert!((off - size).abs() <= 15.0, "{off} off, expected {size}");
    let solve = rig.solve().await;
    let before = rig.get("/api/session").await;
    let off = rig.off(m42());
    let answer = rig.call("POST", CENTRE, json!({})).await;
    assert_eq!(answer["frame_id"], solve["frame_id"]);
    assert_eq!(answer["moved"], "track");
    assert!(
        (answer["offset_arcsec"].as_f64().unwrap() - off).abs() <= 5.0,
        "{answer}, true offset {off}"
    );
    let moving = rig.get("/api/session").await;
    assert_eq!(
        moving["axis_move"],
        json!({"state": "moving", "tool": "centre"})
    );
    assert_eq!(moving["motion"]["by"], "page");
    assert_eq!(moving["motion"]["tool"], "centre");
    let after = rig.rests().await;
    assert!(rig.off(m42()) <= 25.0, "{} off M42", rig.off(m42()));
    assert_eq!(
        after["axis_move"],
        json!({"state": "done", "tool": "centre"})
    );
    assert_eq!(after["motion"], before["motion"]);
    assert!(sep(rig.held(), m42()) <= 0.01);
    assert_eq!(after["aim"], before["aim"]);
    assert_eq!(after["model"]["syncs"], before["model"]["syncs"]);
    assert_eq!(after["model"]["epoch"], before["model"]["epoch"]);
    assert!(!rig.state.session().model(rig.now()).slip_pending);
    let checked = rig.solve().await;
    let centre = (
        checked["ra_deg"].as_f64().unwrap(),
        checked["dec_deg"].as_f64().unwrap(),
    );
    assert!(sep(centre, m42()) <= 25.0, "check solve: {checked}");
}

#[tokio::test(start_paused = true)]
async fn push_390_at_imaging_angle_17() {
    centres(17.0, Cause::Push, 390.0, 30.0).await;
}

#[tokio::test(start_paused = true)]
async fn push_120_at_imaging_angle_100() {
    centres(100.0, Cause::Push, 120.0, 120.0).await;
}

#[tokio::test(start_paused = true)]
async fn push_240_at_imaging_angle_250() {
    centres(250.0, Cause::Push, 240.0, 210.0).await;
}

#[tokio::test(start_paused = true)]
async fn aim_240_at_imaging_angle_100() {
    centres(100.0, Cause::Aim, 240.0, 300.0).await;
}
