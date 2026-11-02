//! One observing session, real catalog renders and solves, and the simulator's
//! independent true eyepiece as oracle: every completed GoTo must be <60″ away.
#[path = "live/support.rs"]
mod support;

use finder::{
    live::{SolveFuture, Solver},
    samples::Samples,
    session::Aim,
    simulate::{parse_start, SimulationTransport},
    solve::{self, SolveOptions},
    AppState,
};
use scope_link::proto;
use scope_node::mount::{Axis, Mount};
use serde_json::{json, Value};
use std::{sync::Arc, time::Duration};
use support::{request, DB};

// Run the real solver on this runtime: a blocking worker must not let paused
// time advance past the star frame that it is solving.
struct Stars;
impl Solver for Stars {
    fn solve(&self, frame: proto::StarFrame, options: SolveOptions) -> SolveFuture<'_> {
        Box::pin(async move {
            solve::solve_centroids(
                &DB,
                &frame.centroids,
                (frame.width as usize, frame.height as usize),
                &options,
            )
        })
    }
}

struct Night {
    state: AppState,
    app: axum::Router,
}
impl Night {
    async fn call(&self, method: &str, path: &str, body: Value) -> Value {
        let (status, body) = request(&self.app, method, path, body).await;
        assert_eq!(status, 200, "{method} {path}: {body}");
        body
    }
    async fn view(&self) -> Value {
        self.call("GET", "/api/session", Value::Null).await
    }
    async fn tick(&self) {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    async fn at_rest(&self) -> bool {
        let mount = self.state.simulator().unwrap().mount().unwrap();
        let az = mount.state(Axis::Azimuth).await;
        let alt = mount.state(Axis::Altitude).await;
        !az.moving && !alt.moving && az.rate_deg_per_s == 0.0 && alt.rate_deg_per_s == 0.0
    }
    async fn start(&self, name: &str) {
        self.call("PUT", "/api/session", json!({"target": name}))
            .await;
        self.call("POST", "/api/goto", Value::Null).await;
        for _ in 0..100 {
            self.tick().await;
            if !self.at_rest().await {
                assert_eq!(self.view().await["goto"]["state"], "slewing");
                return;
            }
        }
        panic!("accepted GoTo never moved: {}", self.view().await);
    }
    async fn finish(&self) -> Value {
        for _ in 0..5000 {
            let view = self.view().await;
            if matches!(
                view["goto"]["state"].as_str(),
                Some("done" | "halted" | "failed")
            ) {
                return view["goto"].clone();
            }
            self.tick().await;
        }
        panic!("motion never finished: {}", self.view().await);
    }
    async fn goto(&self, name: &str) {
        self.start(name).await;
        let progress = self.finish().await;
        assert_eq!(progress["state"], "done", "{name}: {progress}");
        assert!(progress["error_arcsec"].as_f64().unwrap() < 60.0);
        assert!(progress["correction"].as_u64().unwrap() <= 5);
        let target = finder::targets::find(name).unwrap();
        let truth = self.state.simulator().unwrap().telescope().eyepiece;
        let error =
            finder::model::separation(truth.ra, truth.dec, target.ra_deg, target.dec_deg) * 3600.0;
        assert!(error < 60.0, "{name}: true eyepiece error {error}″");
        assert!(self.at_rest().await);
    }
    async fn halted(&self, reason: &str) {
        let progress = self.finish().await;
        assert_eq!(progress["state"], "halted", "{progress}");
        assert!(
            progress["reason"].as_str().unwrap().contains(reason),
            "{progress}"
        );
        // Allow the specified 3°/s² deceleration, then prove that neither a
        // delayed solve nor a stale command restarts either axis for another 2 s.
        for _ in 0..15 {
            self.tick().await;
        }
        assert!(self.at_rest().await);
        let mount = self.state.simulator().unwrap().mount().unwrap();
        let before = [
            mount.state(Axis::Azimuth).await,
            mount.state(Axis::Altitude).await,
        ];
        for _ in 0..20 {
            self.tick().await;
            assert!(self.at_rest().await);
        }
        assert_eq!(mount.state(Axis::Azimuth).await, before[0]);
        assert_eq!(mount.state(Axis::Altitude).await, before[1]);
    }
}

#[tokio::test(start_paused = true)]
async fn a_whole_night_syncs_five_gotos_hand_push_stop_and_dropped_link() {
    let start = parse_start("2026-01-16T02:00:00Z").unwrap();
    assert_eq!(start, 1_768_528_800.0); // Independently worked Unix epoch.
    let state = AppState::shared(DB.clone(), Samples::default())
        .with_simulation(start, SimulationTransport::Memory)
        .await
        .unwrap()
        .with_live_solver(Arc::new(Stars));
    state
        .session()
        .set_aim(Some(Aim { x: 700.0, y: 420.0 }), start)
        .unwrap();
    let sim = state.simulator().unwrap().clone();
    sim.push_to(50.0, 120.0);
    let night = Night {
        app: finder::router(state.clone()),
        state,
    };
    night
        .call(
            "PUT",
            "/api/session",
            json!({
                "site": {"lat_deg": 40, "lon_deg": -75},
                "camera_settings": {"exposure_ms": 200, "gain": 1}, "target": "M42"
            }),
        )
        .await;

    // First sync is a solved star frame, never an injected model observation.
    for _ in 0..100 {
        if night.view().await["model"]["syncs"].as_u64().unwrap_or(0) > 0 {
            break;
        }
        night.tick().await;
    }
    let first = night.view().await;
    assert_eq!(first["model"]["syncs"], 1);
    assert_eq!(first["pointing"]["source"], "live");
    assert_eq!(first["mount"]["connected"], true);
    assert!(first["model"]["base_tilt"].is_null());
    night
        .call("POST", "/api/model/second-sync", Value::Null)
        .await;
    assert_eq!(night.finish().await["state"], "done");
    let second = night.view().await;
    assert!(second["model"]["syncs"].as_u64().unwrap() >= 2);
    assert!(second["model"]["base_tilt"]["deg"].as_f64().unwrap() > 0.0);
    assert!(night.at_rest().await);

    for name in ["M42", "M31", "M45", "M81", "M1"] {
        night.goto(name).await;
    }

    let mount = sim.mount().unwrap();
    let before = [
        mount.state(Axis::Azimuth).await,
        mount.state(Axis::Altitude).await,
    ];
    let direction = mount.true_direction();
    sim.push_by(3.0, 0.0);
    assert_eq!(mount.state(Axis::Azimuth).await, before[0]);
    assert_eq!(mount.state(Axis::Altitude).await, before[1]);
    assert!((mount.true_direction().az - direction.az).abs() > 2.0);
    night.goto("M42").await;

    night.start("M31").await;
    night.call("POST", "/api/stop", Value::Null).await;
    night.halted("observer").await;

    night.start("M81").await;
    sim.set_unresponsive(true);
    night.halted("lost link").await;
}
