//! US4: simulator mid-exposure truth and the rate receipt log are the oracles.
//! The thirty late subs exercise the complete Steering-to-tracker path.
use crate::support::live as support;

use finder::{
    imaging::{
        solver::{ImagingSolve, Region, SolveFrame},
        steering::{ExposureAsk, Steering},
    },
    link::{LinkManager, Transport},
    model::Model,
    session::{Session, Site},
    targets::{self, Target},
    tracking::Tracker,
};
use math_core::{angular_distance, UnitVector};
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

fn ask() -> ExposureAsk {
    ExposureAsk {
        wanted_s: 5.0,
        shortest_s: 0.001,
        sensor: (1236, 822),
        roi: Region {
            x: 0,
            y: 0,
            width: 1236,
            height: 822,
        },
        binning: 1,
    }
}
fn frame(start: f64) -> SolveFrame {
    SolveFrame {
        path: "unused.fits".into(),
        width: 1236,
        height: 822,
        bits: 16,
        bayer: None,
        binning: 1,
        roi: ask().roi,
        start,
        exposure_s: 5.0,
    }
}

// S5: no refraction is drawn, so begin on the catalogue place. Each solve
// arrives a sub late, and past truth is bracketed by reads at both ends.
#[tokio::test(start_paused = true)]
async fn thirty_late_subs_correct_drift_only_between_exposures() {
    let rig = Rig::new(0.01).await;
    let steering = Steering::new(Arc::new(ConsoleClock(rig.clock.clone())));
    steering.set_source(Arc::new({
        let tracker = Arc::downgrade(&rig.tracker);
        move || {
            tracker
                .upgrade()
                .map(|t| t as Arc<dyn finder::imaging::steering::Steered>)
        }
    }));
    let hold = steering.hold();
    let mut previous = None;
    let mut errors = Vec::new();
    let mut drifts = Vec::new();
    for _ in 0..30 {
        let go = steering.before_exposure(ask()).await;
        assert_eq!(go.exposure_s, 5.0);
        assert!(!go.limited);
        let start = rig.now();
        let offset = rig.tracker.state().tracking_offset;
        let first = rig.sim.imaging_truth();
        rig.wait(5.0).await;
        let last = rig.sim.imaging_truth();
        let ray = |ra: f64, dec: f64| UnitVector::from_radec(ra.to_radians(), dec.to_radians());
        drifts.push(
            angular_distance(
                ray(first.ra_deg, first.dec_deg),
                ray(last.ra_deg, last.dec_deg),
            )
            .to_degrees()
                * 3600.0,
        );
        rig.assert_rates(start + 0.001, start + 5.0, offset, 0.001);
        rig.wait(1.0).await;
        if let Some(start) = previous.replace(start) {
            let truth = rig.sim.imaging_truth_at(start + 2.5);
            let scale = truth.arcsec_per_px / 3600.0;
            let (sin, cos) = truth.up_east_of_north_deg.to_radians().sin_cos();
            let solve = ImagingSolve::from_cd(
                truth.ra_deg,
                truth.dec_deg,
                [[-scale * cos, -scale * sin], [scale * sin, -scale * cos]],
                1236,
                822,
            );
            steering.accepted(&frame(start), &solve);
            let view = steering.view().unwrap();
            assert_eq!(view.subs as usize, errors.len() + 1);
            let error = view.last.unwrap();
            assert_eq!(error.time, start + 2.5);
            errors.push(error.east_arcsec.hypot(error.north_arcsec));
        }
    }
    assert_eq!(drifts.len(), 30);
    assert_eq!(errors.len(), 29);
    assert!(errors[0] <= 5.0, "first tracking error {}", errors[0]);
    assert!(drifts[0] >= 0.6, "first drift {}", drifts[0]);
    for (i, error) in errors.iter().enumerate().skip(13) {
        assert!(*error <= 3.0, "sub {} error {error}", i + 1);
    }
    for (i, drift) in drifts.iter().enumerate().skip(13) {
        assert!(*drift <= 0.15, "sub {} drift {drift}", i + 1);
    }
    drop(hold);
    rig.tracker.stop("stopped").await;
}

// S6: Session's own Track state is the oracle, with exact catalogue coordinates.
#[tokio::test(start_paused = true)]
async fn session_exposes_only_its_running_tracker() {
    let rig = crate::support::goto::Rig::new(START, 50.0, 120.0, true).await;
    let session = rig.state.session();
    assert!(session.steered().is_none());
    rig.sim.point_eyepiece_at(83.81867, -5.38967);
    rig.sync().await;
    let (status, body) = support::request(
        &rig.app,
        "POST",
        "/api/track",
        serde_json::json!({"on":true}),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let mut tracking = false;
    for _ in 0..3000 {
        if rig.view().await["track"]["state"] == "tracking" {
            tracking = true;
            break;
        }
        rig.tick().await;
    }
    assert!(tracking, "session never reported tracking");
    let tracker = session.steered().expect("running tracker");
    assert_eq!(tracker.target(), Some((83.81867, -5.38967)));
    let (status, body) = support::request(
        &rig.app,
        "POST",
        "/api/track",
        serde_json::json!({"on":false}),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert!(session.steered().is_none());
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
// Used only for the feed-forward rate oracle; optics draw no refraction.
fn apparent(target: &Target, time: f64) -> (f64, f64) {
    let (alt, az) = simulator::sky::horizontal(target.vector(), simulator::Site::default(), time);
    (
        alt + 1.02 / (alt + 10.3 / (alt + 5.11)).to_radians().tan() / 60.0,
        az,
    )
}
struct Rig {
    clock: VirtualClock,
    sim: Simulator,
    mount: SimMount,
    model: Model,
    rates: Receipts,
    tracker: Arc<Tracker>,
}
impl Rig {
    async fn new(rate_error: f64) -> Self {
        let clock = VirtualClock::new(START);
        let target = targets::find("M42").unwrap();
        let mut model = Model::default();
        model.load_known_terms(0.0, 130.0, [0.0; 2]);
        let (alt, az) =
            simulator::sky::horizontal(target.vector(), simulator::Site::default(), START);
        let zeros = model.inverse(az, alt, [360; 2]).unwrap();
        let mount = SimMount::with_config(
            clock.clone(),
            MountConfig {
                base_tilt_deg: 0.0,
                base_tilt_toward_deg: 130.0,
                zero_points_deg: zeros,
                backlash_deg: 0.0,
            },
        );
        mount.set_rate_error(rate_error);
        let truth = mount.truth();
        model.load_known_terms(
            truth.base_tilt_deg,
            truth.base_tilt_toward_deg,
            truth.zero_points_deg,
        );
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
        let session = Arc::new(Session::default());
        session
            .edit(
                serde_json::from_value(serde_json::json!({
                    "node":{"address":"scope.local:1","token":"test-token"}
                }))
                .unwrap(),
                START,
            )
            .unwrap();
        let console = Arc::new(ConsoleClock(clock.clone()));
        let link = Arc::new(LinkManager::start(session, console.clone(), transport));
        for _ in 0..100 {
            if link.status().borrow().connected {
                break;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        assert!(link.status().borrow().connected);
        let tracker = Arc::new(Tracker::new(
            console,
            link,
            model.clone(),
            Site {
                lat_deg: 40.0,
                lon_deg: -75.0,
            },
            15.0,
        ));
        tracker.start(target.clone()).await.unwrap();
        for _ in 0..500 {
            if AXES
                .iter()
                .all(|axis| rates.lock().unwrap().iter().any(|(_, a, _)| a == axis))
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(AXES
            .iter()
            .all(|axis| rates.lock().unwrap().iter().any(|(_, a, _)| a == axis)));
        tokio::time::sleep(Duration::from_millis(3500)).await;
        Self {
            clock,
            sim,
            mount,
            model,
            rates,
            tracker,
        }
    }
    fn now(&self) -> f64 {
        self.clock.now()
    }
    async fn wait(&self, seconds: f64) {
        for _ in 0..(seconds / STEP.as_secs_f64()).round() as usize {
            tokio::time::sleep(STEP).await;
        }
        assert_eq!(self.tracker.state().state, "tracking");
    }
    fn feed_forward(&self, time: f64) -> [f64; 2] {
        let counts = self.mount.axes().map(|a| a.counts_per_revolution);
        let positions = [time, time + 1.0].map(|t| {
            let (alt, az) = apparent(targets::find("M42").unwrap(), t);
            self.model.inverse(az, alt, counts).unwrap()
        });
        [0, 1].map(|i| {
            let delta = (positions[1][i] - positions[0][i]) * 360.0 / f64::from(counts[i]);
            (if i == 0 {
                (delta + 180.0).rem_euclid(360.0) - 180.0
            } else {
                delta
            }) * 3600.0
        })
    }
    fn assert_rates(&self, start: f64, end: f64, offset: [f64; 2], tolerance: f64) {
        let log = self.rates.lock().unwrap();
        for (i, axis) in AXES.into_iter().enumerate() {
            let entries: Vec<_> = log
                .iter()
                .filter(|(t, a, _)| *t >= start && *t <= end && *a == axis)
                .collect();
            assert!(!entries.is_empty(), "no {axis:?} rate in {start}..{end}");
            for (t, _, rate) in entries {
                let expected = self.feed_forward(*t)[i] + offset[i];
                assert!(
                    (rate * 3600.0 - expected).abs() <= tolerance,
                    "{axis:?}: {} vs {expected} arcsec/s at {t}",
                    rate * 3600.0
                );
            }
        }
    }
}
