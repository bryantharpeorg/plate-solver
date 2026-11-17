//! US2 oracles (026): the night clock runs the console on a night's time behind
//! its two flags, so the Sun guard checks that night's Sun, and the session says
//! so. The day console, built as US1's setup, carries no night key. The rig is
//! the part of US1's setup this story needs: a node, a session and
//! `with_mount_sky`, with no solver and no recording mount.

use crate::support::mcp as support;
use axum::Router;
use finder::{link::Transport, samples::Samples, session::Session, AppState};
use pattern_database::PatternDatabase;
use scope_link::memory;
use scope_node::{
    clock::NodeClock,
    mount::Mount,
    service::{NodeControls, NodeService},
};
use serde_json::json;
use simulator::{mount::SimMount, SimCamera, Simulator, Site, VirtualClock};
use std::{
    path::Path,
    sync::{Arc, LazyLock},
    time::Duration,
};
use support::sim::{ConsoleClock, START};

static DB: LazyLock<Arc<PatternDatabase>> = LazyLock::new(|| {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../reference-solutions/cedar-solve/tetra3/data/default_database.npz");
    Arc::new(pattern_database::load_from_path(&path).unwrap())
});

const SITE: Site = Site {
    lat_deg: 40.0,
    lon_deg: -75.0,
};
/// A console time with the Sun up at the site, when the sky still solves.
const DAY: f64 = START + 15.0 * 3600.0;
/// The night clock the daylight console runs on, as its UTC label says.
const NIGHT: f64 = START;
const NIGHT_UTC: &str = "2026-01-16T02:00:00Z";

struct NodeTime(VirtualClock);
impl NodeClock for NodeTime {
    fn now(&self) -> f64 {
        self.0.node_now()
    }
}

/// The console over a paired node in the day, on the night clock or the day's own.
async fn rig(night: bool) -> Router {
    let clock = VirtualClock::new(DAY);
    let sim = Simulator::new(DB.clone(), SITE, clock.clone());
    let mount = SimMount::new(clock.clone());
    sim.connect_mount(mount.clone());
    let node = NodeService::with_controls_and_mount(
        SimCamera::new(sim.clone()),
        Arc::new(NodeTime(clock.clone())),
        "test-token",
        Default::default(),
        NodeControls::default(),
        Some(Arc::new(mount) as Arc<dyn Mount>),
    )
    .unwrap();
    let controls = node.controls();
    controls.set_frame_interval(Duration::from_millis(500));
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
    let session = Session::default();
    session
        .edit(
            serde_json::from_value(json!({
                "node":{"address":"scope.local:1","token":"test-token"},
                "site":{"lat_deg":40,"lon_deg":-75}, "target":"M42",
                "camera_settings":{"exposure_ms":200,"gain":1}
            }))
            .unwrap(),
            DAY,
        )
        .unwrap();
    let state = AppState::shared(DB.clone(), Samples::default()).with_session(session);
    let state = if night {
        state
            .with_night_clock(NIGHT, NIGHT_UTC)
            .with_mount_sky(transport, 30.0)
    } else {
        state
            .with_clock(Arc::new(ConsoleClock(clock)))
            .with_mount_sky(transport, 30.0)
    };
    let app = support::app(state, "127.0.0.1");
    for _ in 0..100 {
        let view = support::session(&app).await;
        if view["node"]["connected"] == true && view["mount"]["connected"] == true {
            return app;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!(
        "scope link did not connect: {}",
        support::session(&app).await
    );
}

/// A daylight console on the night clock reports `night_utc` and the night's time;
/// one built as the setup builds it carries no night key and the day's time
/// (US2-S4).
#[tokio::test(start_paused = true)]
async fn a_night_clock_runs_the_console_and_a_day_one_does_not() {
    let night = rig(true).await;
    let day = rig(false).await;
    let view = support::session(&night).await;
    assert_eq!(view["mount_sky"]["night_utc"], json!(NIGHT_UTC), "{view}");
    let time = view["time"].as_f64().unwrap();
    assert!(
        (time - NIGHT).abs() < 1.0,
        "the night clock read {time}, not {NIGHT}"
    );
    let view = support::session(&day).await;
    assert!(
        !view["mount_sky"]
            .as_object()
            .unwrap()
            .contains_key("night_utc"),
        "{view}"
    );
    let time = view["time"].as_f64().unwrap();
    assert!(
        (time - DAY).abs() < 1.0,
        "the day clock read {time}, not {DAY}"
    );
}
