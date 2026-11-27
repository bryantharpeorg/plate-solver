//! Oracles: inverse Rodrigues rotation for the initial place, simulator truth,
//! and catalog M42. Tilt tolerance is 1′; first GoTo error is below 120″.
use crate::support::goto::{self as api, Rig};
use finder::{
    api::AppState,
    link::Transport,
    live::Solver,
    samples::Samples,
    session::{Aim, Session},
    sky,
};
use scope_link::memory;
use scope_node::{
    clock::NodeClock,
    mount::{Axis, Mount},
    service::{NodeControls, NodeService},
};
use serde_json::json;
use simulator::{
    mount::{MountConfig, SimMount},
    SimCamera, Simulator, VirtualClock,
};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

struct NodeTime(VirtualClock);
impl NodeClock for NodeTime {
    fn now(&self) -> f64 {
        self.0.node_now()
    }
}

// This story builds its own rig: shared support and earlier stories stay intact.
pub async fn rig_with(config: MountConfig, solver: Arc<dyn Solver>) -> Rig {
    let clock = VirtualClock::new(api::START);
    let sim = Simulator::new(api::DB.clone(), simulator::Site::default(), clock.clone());
    let mount = SimMount::with_config(clock.clone(), config);
    sim.connect_mount(mount.clone());
    let stops = Arc::new(Mutex::new(Vec::new()));
    let recorded = api::Recorded {
        mount: mount.clone(),
        clock: clock.clone(),
        stops: stops.clone(),
    };
    let node = NodeService::with_controls_and_mount(
        SimCamera::new(sim.clone()),
        Arc::new(NodeTime(clock.clone())),
        "test-token",
        Default::default(),
        NodeControls::default(),
        Some(Arc::new(recorded)),
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
            api::START,
        )
        .unwrap();
    session
        .set_aim(Some(Aim { x: 700.0, y: 420.0 }), api::START)
        .unwrap();
    let state = AppState::shared(api::DB.clone(), Samples::default())
        .with_session(session)
        .with_clock(Arc::new(api::ConsoleClock(clock)))
        .with_link(transport)
        .with_live_solver(solver);
    let app = finder::router(state.clone());
    let rig = Rig {
        sim,
        mount,
        state,
        app,
        stops,
        controls,
    };
    for _ in 0..100 {
        if rig.state.link().unwrap().status().borrow().connected {
            return rig;
        }
        rig.tick().await;
    }
    panic!("scope node did not connect");
}

pub fn latest_frame(rig: &Rig) -> u64 {
    rig.state
        .link()
        .unwrap()
        .frames()
        .borrow()
        .as_ref()
        .unwrap()
        .id
}

// Inverse of the mount's tilt rotation. At +4500 counts the tube has
// advanced 1° minus the 0.15° half backlash, hence the 0.85° offset.
fn zero_points(alt: f64, az: f64, tilt: f64, toward: f64) -> [f64; 2] {
    let (a, e, w) = (az.to_radians(), alt.to_radians(), toward.to_radians());
    let v = [e.cos() * a.cos(), e.cos() * a.sin(), e.sin()];
    let k = [-w.sin(), w.cos(), 0.0];
    let (s, c) = (-tilt).to_radians().sin_cos();
    let dot = k[0] * v[0] + k[1] * v[1];
    let cross = [k[1] * v[2], -k[0] * v[2], k[0] * v[1] - k[1] * v[0]];
    let v = [0, 1, 2].map(|i| c * v[i] + s * cross[i] + (1.0 - c) * dot * k[i]);
    [
        v[1].atan2(v[0]).to_degrees() - 0.85,
        v[2].asin().to_degrees() - 0.85,
    ]
}

async fn tilt_rig(alt: f64, az: f64, tilt: f64, toward: f64) -> Rig {
    let rig = rig_with(
        MountConfig {
            base_tilt_deg: tilt,
            base_tilt_toward_deg: toward,
            zero_points_deg: zero_points(alt, az, tilt, toward),
            backlash_deg: 0.3,
        },
        Arc::new(api::Stars),
    )
    .await;
    rig.move_to([4500, 4500]).await;
    for _ in 0..15 {
        rig.tick().await;
    }
    let truth = rig.mount.true_direction();
    assert!(distance_deg([truth.alt, truth.az], [alt, az]) < 0.001);
    rig.state.session().reset_model(false, rig.now()).unwrap();
    rig.sync().await;
    assert_eq!(rig.state.session().model(rig.now()).syncs.len(), 1);
    rig
}
// Great-circle distance by the chord, independent of the model's separation.
fn distance_deg(a: [f64; 2], b: [f64; 2]) -> f64 {
    let vector = |[alt, az]: [f64; 2]| {
        let (e, a) = (alt.to_radians(), az.to_radians());
        [e.cos() * a.cos(), e.cos() * a.sin(), e.sin()]
    };
    let (a, b) = (vector(a), vector(b));
    let chord = (0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f64>().sqrt();
    2.0 * (chord / 2.0).asin().to_degrees()
}
fn wrap(angle: f64) -> f64 {
    (angle + 180.0).rem_euclid(360.0) - 180.0
}

fn place(rig: &Rig, alt: f64, az: f64) -> finder::targets::Target {
    let mut target = finder::targets::find("M42").unwrap().clone();
    let date = simulator::sky::horizontal_to_date(alt, az, simulator::Site::default(), rig.now());
    let (ra, dec) = simulator::sky::to_j2000(date, sky::julian_date(rig.now())).to_radec();
    target.ra_deg = ra.to_degrees();
    target.dec_deg = dec.to_degrees();
    target
}

async fn fits_and_gotos(tilt: f64) {
    for toward in [195.0, 300.0] {
        let rig = tilt_rig(75.0, 122.0, tilt, toward).await;
        let (status, body) = rig.call("/api/model/second-sync", false).await;
        assert_eq!(status, 200, "{body}");
        assert_eq!(body["started"], true);
        let progress = rig
            .finish_with(Some(15.0), true, |p| {
                assert_eq!(p["correction"], 0);
            })
            .await;
        assert_eq!(
            progress.last().unwrap()["state"],
            "done",
            "{tilt} toward {toward}: {progress:?}"
        );
        let model = rig.state.session().model(rig.now());
        assert_eq!(model.syncs.len(), 2);
        assert_eq!(model.epochs.len(), 1);
        let base = model.base_tilt.expect("second sync fits base tilt");
        assert!((base.deg() - tilt).abs() * 60.0 < 1.0, "{base:?}");
        let second = progress.last().unwrap();
        assert!(second["frame_id"].as_u64().unwrap() > 0);
        assert_eq!(
            second["frame_time"].as_f64(),
            Some(model.syncs.last().unwrap().time)
        );
        for (alt, az) in [(36.0, 127.0), (40.0, 250.0)] {
            let target = place(&rig, alt, az);
            let latest = latest_frame(&rig);
            let started = rig.now();
            rig.state
                .goto()
                .unwrap()
                .start_target(target)
                .await
                .unwrap();
            let mut first_error = None;
            let mut slewing_reads = 0;
            let progress = rig
                .finish_with(Some(15.0), true, |p| {
                    if p["state"] == "slewing" && p["correction"] == 0 {
                        slewing_reads += 1;
                        assert!(p.get("frame_id").is_none(), "{p}");
                        assert!(p.get("frame_time").is_none(), "{p}");
                    }
                    if let Some(error) = p["error_arcsec"].as_f64() {
                        first_error.get_or_insert(error);
                        assert!(p["frame_id"].as_u64().unwrap() > latest);
                        assert!(p["frame_time"].as_f64().unwrap() > started);
                    }
                })
                .await;
            assert!(slewing_reads > 0);
            assert_eq!(progress.last().unwrap()["state"], "done", "{progress:?}");
            assert!(
                first_error.unwrap() < 120.0,
                "{tilt} toward {toward}: {progress:?}"
            );
        }
        assert_eq!(rig.state.session().model(rig.now()).epochs.len(), 1);
    }
}

#[tokio::test(start_paused = true)]
async fn level_base() {
    fits_and_gotos(0.0).await;
}
#[tokio::test(start_paused = true)]
async fn one_degree_base() {
    fits_and_gotos(1.0).await;
}
#[tokio::test(start_paused = true)]
async fn two_degree_base() {
    fits_and_gotos(2.0).await;
}
#[tokio::test(start_paused = true)]
async fn three_degree_base() {
    fits_and_gotos(3.0).await;
}
#[tokio::test(start_paused = true)]
async fn four_degree_base() {
    fits_and_gotos(4.0).await;
}
#[tokio::test(start_paused = true)]
async fn five_degree_base() {
    fits_and_gotos(5.0).await;
}

#[tokio::test(start_paused = true)]
async fn second_sync_prefers_the_nearest_altitude_in_the_band() {
    for (alt, expected) in [(75.0, 60.0), (45.0, 45.0), (20.0, 30.0)] {
        let rig = tilt_rig(alt, 122.0, 0.0, 195.0).await;
        let before = rig.mount.state(Axis::Azimuth).await.position;
        assert_eq!(rig.call("/api/model/second-sync", false).await.0, 200);
        let progress = rig.finish_with(Some(15.0), true, |_| {}).await;
        assert_eq!(progress.last().unwrap()["state"], "done", "{progress:?}");
        let truth = rig.mount.true_direction();
        assert!((truth.alt - expected).abs() < 0.2, "{truth:?}");
        let turned = f64::from(rig.mount.state(Axis::Azimuth).await.position - before) * 360.0
            / f64::from(rig.mount.axes()[0].counts_per_revolution);
        assert!(turned.abs() >= 90.0, "{turned}");
        assert!(wrap(truth.az - 122.0).abs() >= 90.0, "{truth:?}");
        assert_eq!(rig.state.session().model(rig.now()).epochs.len(), 1);
    }
}

#[tokio::test(start_paused = true)]
async fn second_sync_says_when_taken_as_a_slip() {
    let rig = tilt_rig(75.0, 122.0, 8.0, 45.0).await;
    assert_eq!(rig.call("/api/model/second-sync", false).await.0, 200);
    let progress = rig
        .finish_with(Some(15.0), true, |p| {
            assert_eq!(p["correction"], 0);
        })
        .await;
    let last = progress.last().unwrap();
    assert_eq!(last["state"], "failed", "{progress:?}");
    assert_eq!(last["reason"], "taken as a slip: no base tilt fitted");
    let model = rig.state.session().model(rig.now());
    assert_eq!(model.epochs.len(), 2);
    assert!(model.base_tilt.is_none());
}
