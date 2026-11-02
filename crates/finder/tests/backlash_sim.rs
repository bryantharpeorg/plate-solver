//! Oracles: configured 0.3 degree mechanical play and the simulator's independent
//! eyepiece ray. Exact command ordering; 60/30 arcsecond sky bounds in paused time.
#[path = "live/support.rs"]
mod support;

use finder::{
    link::Transport,
    model::{separation, Model},
    samples::Samples,
    session::{Session, Site},
    targets::{self, Target},
    tracking::Tracker,
    AppState,
};
use math_core::UnitVector;
use scope_link::{memory, proto};
use scope_node::{
    clock::NodeClock,
    mount::{Axis, AxisInfo, AxisState, Mount, MountFuture},
    service::{NodeControls, NodeService},
};
use simulator::{
    mount::{MountConfig, SimMount},
    SimCamera, Simulator, VirtualClock,
};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use support::{ConsoleClock, DB, START};

const STEP: Duration = Duration::from_millis(50);
const AXES: [Axis; 2] = [Axis::Azimuth, Axis::Altitude];
type Receipts = Arc<Mutex<Vec<(f64, Axis, f64)>>>;

struct NodeTime(VirtualClock);
impl NodeClock for NodeTime {
    fn now(&self) -> f64 {
        self.0.node_now()
    }
}
#[derive(Clone)]
struct Recorded {
    mount: SimMount,
    clock: VirtualClock,
    rates: Receipts,
}
impl Mount for Recorded {
    fn axes(&self) -> [AxisInfo; 2] {
        self.mount.axes()
    }
    fn state(&self, axis: Axis) -> MountFuture<'_, AxisState> {
        self.mount.state(axis)
    }
    fn goto(&self, axis: Axis, position: i32) -> MountFuture<'_, ()> {
        self.mount.goto(axis, position)
    }
    fn rate(&self, axis: Axis, rate: f64) -> MountFuture<'_, ()> {
        Box::pin(async move {
            self.rates
                .lock()
                .unwrap()
                .push((self.clock.now(), axis, rate));
            self.mount.rate(axis, rate).await;
        })
    }
    fn stop_all(&self) -> MountFuture<'_, ()> {
        self.mount.stop_all()
    }
}

// Saemundsson (Meeus 16.4), independent of the controller's implementation.
// The simulator has geometric optics. Its physical ray is compared with the
// target's apparent ray, as in US3, for both truth and rendered measurements.
fn apparent(target: &Target, time: f64) -> (f64, f64) {
    let (alt, az) = simulator::sky::horizontal(target.vector(), simulator::Site::default(), time);
    (
        alt + 1.02 / (alt + 10.3 / (alt + 5.11)).to_radians().tan() / 60.0,
        az,
    )
}
fn equatorial(alt: f64, az: f64, time: f64) -> UnitVector {
    simulator::sky::to_j2000(
        simulator::sky::horizontal_to_date(alt, az, simulator::Site::default(), time),
        simulator::sky::julian_date(time),
    )
}
fn error(aim: UnitVector, target: UnitVector) -> [f64; 2] {
    let (ra, dec) = target.to_radec();
    let (a, d) = aim.to_radec();
    [
        (((a - ra).to_degrees() + 180.0).rem_euclid(360.0) - 180.0) * dec.cos() * 3600.0,
        (d - dec).to_degrees() * 3600.0,
    ]
}

pub struct Rig {
    clock: VirtualClock,
    mount: SimMount,
    model: Model,
    pub session: Arc<Session>,
    app: axum::Router,
    target: Target,
    rates: Receipts,
    tracker: Tracker,
}
impl Rig {
    pub async fn new(start: f64, target: Target, session: Session) -> Self {
        let clock = VirtualClock::new(start - 1.5);
        let (alt, az) = apparent(&target, start);
        let zeros = [az - 0.35, alt - 0.35];
        let mount = SimMount::with_config(
            clock.clone(),
            MountConfig {
                base_tilt_deg: 0.0,
                base_tilt_toward_deg: 0.0,
                zero_points_deg: zeros,
                backlash_deg: 0.3,
            },
        );
        // Load both gears on the GoTo's positive approach side. The independent
        // play operator leaves the tube 0.15 degrees behind the encoder.
        for axis in AXES {
            mount.rate(axis, 0.5).await;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
        mount.stop_all().await;
        tokio::time::sleep(Duration::from_millis(500)).await;
        let sim = Simulator::new(DB.clone(), simulator::Site::default(), clock.clone());
        sim.connect_mount(mount.clone());
        let rates = Receipts::default();
        let controls = NodeControls::default();
        controls.stop_frames();
        let node = NodeService::with_controls_and_mount(
            SimCamera::new(sim.clone()),
            Arc::new(NodeTime(clock.clone())),
            "test-token",
            Default::default(),
            controls,
            Some(Arc::new(Recorded {
                mount: mount.clone(),
                clock: clock.clone(),
                rates: rates.clone(),
            })),
        )
        .unwrap();
        let transport = Transport::connector(move || {
            let node = node.clone();
            async move {
                let (client, server) = tokio::io::duplex(64 * 1024);
                tokio::spawn(async move {
                    let routes = tonic::service::Routes::new(node.clone().into_server())
                        .add_service(
                            proto::scope_node_mount_server::ScopeNodeMountServer::with_interceptor(
                                node.clone(),
                                scope_link::TokenInterceptor::new("test-token"),
                            ),
                        )
                        .add_service(proto::scope_node_stop_server::ScopeNodeStopServer::new(
                            node,
                        ));
                    memory::serve_routes_on(server, routes).await.unwrap();
                });
                Ok::<_, std::io::Error>(client)
            }
        });
        session
            .edit(
                serde_json::from_value(serde_json::json!({
                    "node":{"address":"scope.local:1","token":"test-token"},
                    "site":{"lat_deg":40,"lon_deg":-75}
                }))
                .unwrap(),
                start,
            )
            .unwrap();
        let console = Arc::new(ConsoleClock(clock.clone()));
        let state = AppState::shared(DB.clone(), Samples::default())
            .with_session(session)
            .with_clock(console.clone())
            .with_link(transport);
        let session = state.session();
        let link = state.link().unwrap();
        for _ in 0..100 {
            if link.status().borrow().connected {
                break;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        assert!(link.status().borrow().connected);
        let mut model = session.model(start);
        model.load_known_terms(0.0, 0.0, zeros.map(|z| z - 0.15));
        let tracker = Tracker::new(
            console,
            link,
            model.clone(),
            Site {
                lat_deg: 40.0,
                lon_deg: -75.0,
            },
            15.0,
        );
        tracker.start(target.clone()).await.unwrap();
        Self {
            clock,
            mount,
            model,
            session,
            target,
            rates,
            tracker,
            app: finder::router(state),
        }
    }
    pub fn now(&self) -> f64 {
        self.clock.now()
    }
    fn target_ray(&self) -> UnitVector {
        let (alt, az) = apparent(&self.target, self.now());
        equatorial(alt, az, self.now())
    }
    pub fn truth_error(&self) -> [f64; 2] {
        let d = self.mount.true_direction();
        error(equatorial(d.alt, d.az, self.now()), self.target_ray())
    }
    pub fn true_error_arcsec(&self) -> f64 {
        let d = self.mount.true_direction();
        let (alt, az) = apparent(&self.target, self.now());
        separation(d.az, d.alt, az, alt) * 3600.0
    }
}

// Numerical search uses the simulator's celestial transform, not the tracker's
// feed-forward or forecast. M42's maximum altitude is its transit; M81's maximum
// azimuth is the western greatest elongation.
pub fn reversal(target: &Target, axis: usize) -> f64 {
    turning_point(target, axis, START, true)
}
fn turning_point(target: &Target, axis: usize, start: f64, maximum: bool) -> f64 {
    let coordinate = |t: f64| {
        let (h, a) = apparent(target, t);
        [a, h][axis]
    };
    let mut lo = start;
    for n in 1..96 {
        let t = start + f64::from(n) * 900.0;
        if (coordinate(t) < coordinate(t - 1.0)) == maximum
            && (coordinate(t - 900.0) > coordinate(t - 901.0)) == maximum
        {
            lo = t - 900.0;
            break;
        }
    }
    let mut hi = lo + 900.0;
    assert_eq!(coordinate(lo + 1.0) > coordinate(lo), maximum);
    assert_eq!(coordinate(hi + 1.0) < coordinate(hi), maximum);
    for _ in 0..40 {
        let mid = (lo + hi) / 2.0;
        if (coordinate(mid + 0.5) > coordinate(mid - 0.5)) == maximum {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (lo + hi) / 2.0
}
impl Rig {
    pub async fn follow(&self, until: f64, bound: f64) {
        while self.now() < until {
            let e = self.truth_error();
            self.tracker.pointing(e[0], e[1]);
            for _ in 0..40 {
                assert!(
                    self.true_error_arcsec() <= bound,
                    "{} arcsec at {}",
                    self.true_error_arcsec(),
                    self.now()
                );
                tokio::time::sleep(STEP).await;
            }
            assert_eq!(self.tracker.state().state, "tracking");
        }
    }
    fn assert_reversal(&self, axis: Axis, since: f64, takeup: bool) {
        let rates = self.rates.lock().unwrap();
        let rates: Vec<_> = rates
            .iter()
            .filter(|(t, a, _)| *a == axis && *t >= since)
            .collect();
        let before = rates.iter().find(|(_, _, r)| *r != 0.0).unwrap().2.signum();
        let first_negative = rates
            .iter()
            .position(|(_, _, r)| *r * before < 0.0)
            .unwrap();
        assert!(rates[..first_negative]
            .iter()
            .any(|(_, _, r)| *r * before > 0.0));
        assert_eq!(
            rates[first_negative - 1].2,
            0.0,
            "zero before opposite rate"
        );
        if takeup {
            assert_eq!(rates[first_negative].2, -before * 0.5);
            let end = rates[first_negative + 1..]
                .iter()
                .find(|(_, _, r)| *r == 0.0)
                .unwrap();
            assert!((end.0 - rates[first_negative].0 - 0.6).abs() < 0.02);
            assert!(rates
                .iter()
                .any(|(t, _, r)| *t > end.0 && *r * before < 0.0 && r.abs() < 0.1));
        }
    }
    pub async fn body(&self) -> serde_json::Value {
        support::request(&self.app, "GET", "/api/session", serde_json::Value::Null)
            .await
            .1
    }
}

pub async fn measured(session: Session) -> Rig {
    let target = targets::find("M42").unwrap().clone();
    let transit = reversal(&target, 1);
    let rig = Rig::new(transit - 600.0, target, session).await;
    let start = rig.now();
    rig.follow(transit + 600.0, 60.0).await;
    assert!(rig.true_error_arcsec() <= 30.0);
    rig.assert_reversal(Axis::Altitude, start, false);
    let b = rig.body().await["backlash"]["alt_deg"].as_f64().unwrap();
    assert!((b - 0.3).abs() * 3600.0 <= 30.0, "measured {b}");
    rig
}

#[tokio::test(start_paused = true)]
async fn first_transit_measures_then_fixed_reset_removes_the_observed_value() {
    let rig = measured(Session::default()).await;
    assert!(rig.body().await["backlash"]["alt_deg"].as_f64().unwrap() > 0.0);
    let (status, body) = support::request(
        &rig.app,
        "POST",
        "/api/model/reset",
        serde_json::json!({"what":"fixed"}),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert!(body.get("backlash").is_none());
    assert!(rig.body().await.get("backlash").is_none());
    rig.tracker.stop("stopped").await;
}

#[tokio::test(start_paused = true)]
async fn first_azimuth_reversal_measures_its_own_dead_band() {
    let target = targets::find("M81").unwrap().clone();
    let turn = reversal(&target, 0);
    let rig = Rig::new(turn - 600.0, target, Session::default()).await;
    let start = rig.now();
    rig.follow(turn + 600.0, 60.0).await;
    assert!(rig.true_error_arcsec() <= 30.0);
    rig.assert_reversal(Axis::Azimuth, start, false);
    let body = rig.body().await;
    let az = body["backlash"]["az_deg"].as_f64().unwrap();
    assert!((az - 0.3).abs() * 3600.0 <= 30.0, "measured {az}");
    assert!(body["backlash"].get("alt_deg").is_none());
    rig.tracker.stop("stopped").await;
}

#[tokio::test(start_paused = true)]
async fn another_target_on_the_same_mount_takes_up_the_measured_altitude_play() {
    let mut rig = measured(Session::default()).await;
    let b = rig.body().await["backlash"]["alt_deg"].as_f64().unwrap();
    rig.tracker.stop("stopped").await;
    let mut next = rig.target.clone();
    next.ra_deg += 5.0;
    let transit = reversal(&next, 1);
    // A GoTo-style final positive approach, on this same mechanical mount.
    let (h, a) = apparent(&next, rig.now() + 10.0);
    let counts = rig.mount.axes().map(|a| a.counts_per_revolution);
    let dest = rig.model.inverse(a, h, counts).unwrap();
    for offset in [-0.5, 0.0] {
        for (i, axis) in AXES.into_iter().enumerate() {
            rig.mount
                .goto(
                    axis,
                    (dest[i] + offset * f64::from(counts[i]) / 360.0).round() as i32,
                )
                .await;
        }
        loop {
            tokio::time::sleep(STEP).await;
            if !rig.mount.state(Axis::Azimuth).await.moving
                && !rig.mount.state(Axis::Altitude).await.moving
            {
                break;
            }
        }
    }
    rig.target = next.clone();
    rig.tracker.start(next).await.unwrap();
    rig.follow(transit - 60.0, 180.0).await;
    let start = rig.now();
    rig.follow(transit + 120.0, 30.0).await;
    // Command duration follows the measured value, within its 30 arcsec tolerance.
    assert!((b - 0.3).abs() < 30.0 / 3600.0);
    rig.assert_reversal(Axis::Altitude, start, true);
    rig.tracker.stop("stopped").await;
}

#[tokio::test(start_paused = true)]
async fn m81_takes_up_azimuth_play_at_greatest_elongation() {
    let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target")
        .join(format!("backlash-az-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let path = folder.join("session.json");
    std::fs::write(
        &path,
        r#"{"site":null,"target":null,"min_alt_deg":15,"backlash":{"az_deg":0.3,"alt_deg":0.3}}"#,
    )
    .unwrap();
    let target = targets::find("M81").unwrap().clone();
    // March puts the western elongation in darkness. Its positive hour angle
    // exercises the negative-to-positive reversal, opposite to a transit.
    let turn = turning_point(&target, 0, START + 60.0 * 86_400.0, false);
    let (h, _) = apparent(&target, turn);
    assert!((h - 43.5).abs() < 0.5, "M81 elongation altitude {h}");
    let (_, az) = apparent(&target, turn);
    let (e, a, p) = (h.to_radians(), az.to_radians(), 40_f64.to_radians());
    let hour_angle = (-a.sin() * e.cos())
        .atan2(e.sin() * p.cos() - e.cos() * p.sin() * a.cos())
        .to_degrees();
    assert!(
        (hour_angle - 71.3).abs() < 0.5,
        "M81 hour angle {hour_angle}"
    );
    let rig = Rig::new(
        turn - 120.0,
        target,
        Session::with_state_file(&path).unwrap(),
    )
    .await;
    rig.follow(rig.now() + 10.0, 30.0).await;
    let start = rig.now();
    rig.follow(turn + 120.0, 30.0).await;
    rig.assert_reversal(Axis::Azimuth, start, true);
    rig.tracker.stop("stopped").await;
    std::fs::remove_dir_all(folder).unwrap();
}

#[tokio::test(start_paused = true)]
async fn descending_syncs_are_added_without_changing_epoch_or_biasing_ie() {
    let target = targets::find("M42").unwrap().clone();
    let transit = reversal(&target, 1);
    let rig = Rig::new(transit - 600.0, target, Session::default()).await;
    let mut up_ie = None;
    let mut down = 0;
    let mut held = 0;
    while rig.now() < transit + 600.0 {
        let e = rig.truth_error();
        rig.tracker.pointing(e[0], e[1]);
        {
            let d = rig.mount.true_direction();
            let counts = rig.mount.axes().map(|a| a.counts_per_revolution);
            let axis = [
                rig.mount.state(Axis::Azimuth).await.position as f64,
                rig.mount.state(Axis::Altitude).await.position as f64,
            ];
            let accepted = rig
                .session
                .record_sync(finder::model::Sync {
                    time: rig.now(),
                    axis,
                    counts_per_revolution: counts,
                    az_deg: d.az,
                    alt_deg: d.alt,
                    roll_deg: None,
                    rms_arcsec: 3.0,
                    epoch: 0,
                })
                .unwrap();
            if rig.now() >= transit && rig.body().await.get("backlash").is_none() {
                if rig.mount.state(Axis::Altitude).await.rate_deg_per_s < 0.0 {
                    assert!(!accepted, "unknown reverse-side sync must be held out");
                    held += 1;
                }
                rig.follow(rig.now() + 2.0, 60.0).await;
                continue;
            }
            assert!(accepted);
            let model = rig.session.model(rig.now());
            if rig.now() < transit {
                up_ie = Some(model.epochs.last().unwrap().ie);
            } else {
                down += 1;
                assert_eq!(model.epochs.len(), 1);
                assert!(model.syncs.iter().any(|s| s.time > transit));
                assert!((model.epochs.last().unwrap().ie - up_ie.unwrap()).abs() * 3600.0 <= 30.0);
            }
        }
        rig.follow(rig.now() + 2.0, 60.0).await;
    }
    assert!(held > 10, "syncs while measuring must actually be held out");
    assert!(down > 10, "descending syncs must actually be added");
    rig.tracker.stop("stopped").await;
}
