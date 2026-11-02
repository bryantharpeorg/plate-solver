//! US4 oracles: the simulator's physical eyepiece, seeded Gaussian measurement
//! noise, and catalog-rendered pixels solved through finder::solve. Time is paused.
#[path = "live/support.rs"]
mod support;

use finder::{
    link::{LinkManager, Transport},
    model::{separation, Model},
    session::{Session, Site},
    solve::{self, GrayImage, SolveOptions},
    targets::{self, Target},
    tracking::Tracker,
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

struct Rig {
    clock: VirtualClock,
    sim: Simulator,
    mount: SimMount,
    model: Model,
    rates: Receipts,
    tracker: Tracker,
}
impl Rig {
    async fn new(rate_error: f64) -> Self {
        let clock = VirtualClock::new(START);
        let target = targets::find("M42").unwrap();
        let mut model = Model::default();
        model.load_known_terms(1.0, 130.0, [0.0; 2]);
        let (alt, az) = apparent(target, START);
        let zeros = model.inverse(az, alt, [360; 2]).unwrap();
        let mount = SimMount::with_config(
            clock.clone(),
            MountConfig {
                base_tilt_deg: 1.0,
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
    async fn seconds(&self, seconds: usize) {
        for _ in 0..seconds * 20 {
            tokio::time::sleep(STEP).await;
        }
        assert_eq!(self.tracker.state().state, "tracking");
    }
    fn target_ray(&self) -> UnitVector {
        let (alt, az) = apparent(targets::find("M42").unwrap(), self.now());
        equatorial(alt, az, self.now())
    }
    fn truth_error(&self) -> [f64; 2] {
        let d = self.mount.true_direction();
        error(equatorial(d.alt, d.az, self.now()), self.target_ray())
    }
    fn true_error_arcsec(&self) -> f64 {
        let d = self.mount.true_direction();
        let (alt, az) = apparent(targets::find("M42").unwrap(), self.now());
        separation(d.az, d.alt, az, alt) * 3600.0
    }
    fn rendered_error(&self, id: u64) -> [f64; 2] {
        let optics = self.sim.optics();
        let stars = self
            .sim
            .field_stars(200.0, 1.0)
            .into_iter()
            .map(|(_, s)| s)
            .collect::<Vec<_>>();
        assert!(stars.len() >= 10, "the rendered field must contain stars");
        let pixels = simulator::render::render(&optics, &stars, 200.0, 1.0, 13, id);
        let (report, solved) = solve::solve_frame_with_frame(
            &DB,
            &GrayImage {
                pixels,
                width: optics.width as usize,
                height: optics.height as usize,
            },
            &SolveOptions {
                fov: Some(optics.fov_deg),
                ..Default::default()
            },
        );
        let solved = solved.unwrap_or_else(|| panic!("rendered solve failed: {report:?}"));
        let aim = solved.unproject(optics.eyepiece_px).unwrap();
        error(aim, self.target_ray())
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
    fn assert_rates(&self, since: f64, correction: [f64; 2]) {
        let rates = self.rates.lock().unwrap();
        for (i, axis) in AXES.into_iter().enumerate() {
            let axis_rates = rates
                .iter()
                .filter(|(t, a, _)| *t >= since && *a == axis)
                .collect::<Vec<_>>();
            assert!(!axis_rates.is_empty(), "connected mount must receive rates");
            for (time, _, rate) in axis_rates {
                let expected = self.feed_forward(*time)[i] + correction[i];
                assert!(
                    (rate * 3600.0 - expected).abs() <= 0.001,
                    "{axis:?}: {} vs {expected} arcsec/s",
                    rate * 3600.0
                );
            }
        }
    }
}

// Local seeded xorshift64 + Box-Muller: independent Gaussian draws, sigma 3 arcsec
// on each tangent-plane coordinate. Never draw from the simulator's private RNG.
struct Noise(u64);
impl Noise {
    fn uniform(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        ((self.0 >> 11) as f64 + 0.5) / (1_u64 << 53) as f64
    }
    fn pair(&mut self) -> [f64; 2] {
        let radius = 3.0 * (-2.0 * self.uniform().ln()).sqrt();
        let (s, c) = (std::f64::consts::TAU * self.uniform()).sin_cos();
        [radius * c, radius * s]
    }
}
#[derive(Clone, Copy)]
enum Feed {
    None,
    Noise,
    Rendered,
}
struct Run {
    errors: Vec<[f64; 2]>,
    corrected: bool,
    final_error: f64,
}
async fn run(feed: Feed, seconds: usize) -> Run {
    let rig = Rig::new(0.01).await;
    let mut noise = Noise(42);
    let mut errors = Vec::new();
    let mut corrected = false;
    for n in 0..seconds / 2 {
        let measured = match feed {
            Feed::None => None,
            Feed::Noise => {
                let truth = rig.truth_error();
                let noise = noise.pair();
                Some([truth[0] + noise[0], truth[1] + noise[1]])
            }
            Feed::Rendered => Some(rig.rendered_error(n as u64)),
        };
        if let Some([east, north]) = measured {
            rig.tracker.pointing(east, north);
            corrected |= rig
                .tracker
                .state()
                .tracking_offset
                .iter()
                .any(|v| *v != 0.0);
        }
        rig.seconds(2).await;
        errors.push(rig.truth_error());
    }
    assert!(rig.rates.lock().unwrap().len() >= seconds * 2);
    let final_error = rig.true_error_arcsec();
    rig.tracker.stop("stopped").await;
    Run {
        errors,
        corrected,
        final_error,
    }
}
fn rms(values: impl Iterator<Item = f64>) -> f64 {
    let values: Vec<_> = values.collect();
    assert!(!values.is_empty());
    (values.iter().map(|v| v * v).sum::<f64>() / values.len() as f64).sqrt()
}

#[tokio::test(start_paused = true)]
async fn one_percent_fast_mount_drifts_open_loop_and_holds_with_finder_corrections() {
    let open = run(Feed::None, 600).await;
    assert!(
        open.final_error > 60.0,
        "feed-forward drift {} arcsec",
        open.final_error
    );
    let closed = run(Feed::Noise, 600).await;
    assert!(
        closed.corrected,
        "a disconnected or inactive loop cannot pass"
    );
    let error = rms(closed.errors[60..].iter().map(|e| e[0].hypot(e[1])));
    eprintln!(
        "open-loop drift {} arcsec; closed-loop last-eight-minute RMS {error} arcsec",
        open.final_error
    );
    assert!(error <= 20.0, "last eight minutes RMS {error} arcsec");
}

#[tokio::test(start_paused = true)]
async fn sub_ten_arcsecond_errors_leave_rates_at_feed_forward_and_offset_zero() {
    let rig = Rig::new(0.0).await;
    for _ in 0..60 {
        let [east, north] = rig.truth_error();
        assert!(east.hypot(north) < 10.0);
        rig.tracker.pointing(east, north);
        rig.seconds(2).await;
        assert_eq!(rig.tracker.state().tracking_offset, [0.0; 2]);
    }
    rig.assert_rates(START, [0.0; 2]);
    rig.tracker.stop("stopped").await;
}

#[tokio::test(start_paused = true)]
async fn rendered_solves_and_noisy_truth_apply_corrections_and_agree_within_five_arcseconds() {
    let rendered = run(Feed::Rendered, 120).await;
    let noisy = run(Feed::Noise, 120).await;
    assert!(
        rendered.corrected && noisy.corrected,
        "both paths must apply corrections"
    );
    let difference = rms(rendered
        .errors
        .iter()
        .zip(&noisy.errors)
        .map(|(a, b)| (a[0] - b[0]).hypot(a[1] - b[1])));
    eprintln!("rendered/noisy true eyepiece difference {difference} arcsec RMS");
    assert!(
        difference <= 5.0,
        "true eyepiece errors differ by {difference} arcsec RMS"
    );
}

#[tokio::test(start_paused = true)]
async fn held_integral_applies_in_deadband_is_clamped_and_clears_on_stop_and_restart() {
    let rig = Rig::new(0.0).await;
    rig.tracker.pointing(0.0, 0.0);
    rig.seconds(2).await;
    rig.tracker.pointing(100.0, -50.0);
    let held = rig.tracker.state().tracking_offset;
    assert!(held.iter().any(|v| *v != 0.0));
    rig.tracker.pointing(100.0, -50.0);
    assert_eq!(
        rig.tracker.state().tracking_offset,
        held,
        "duplicate instants do not integrate twice"
    );
    rig.tracker.pointing(3.0, 4.0);
    rig.tracker.pointing(f64::NAN, 0.0);
    let since = rig.now() + 0.01;
    rig.seconds(4).await;
    assert_eq!(rig.tracker.state().tracking_offset, held);
    rig.assert_rates(since, held);
    for _ in 0..5 {
        rig.tracker.pointing(10000.0, -10000.0);
        rig.seconds(2).await;
    }
    // Five per cent of the Meeus sidereal rate, 360 deg per sidereal day.
    let limit = 0.05 * 360.985_647_366_29 * 3600.0 / 86400.0;
    for offset in rig.tracker.state().tracking_offset {
        assert!((offset.abs() - limit).abs() < 1e-9);
    }
    rig.tracker.stop("stopped").await;
    assert_eq!(rig.tracker.state().tracking_offset, [0.0; 2]);
    rig.tracker.pointing(1000.0, 1000.0);
    assert_eq!(rig.tracker.state().tracking_offset, [0.0; 2]);
    rig.tracker
        .start(targets::find("M42").unwrap().clone())
        .await
        .unwrap();
    let since = rig.now() + 0.01;
    rig.seconds(2).await;
    rig.assert_rates(since, [0.0; 2]);
    rig.tracker.stop("stopped").await;
}

#[tokio::test(start_paused = true)]
async fn solves_at_the_five_second_cadence_hold_the_design_rate_error() {
    let rig = Rig::new(0.01).await;
    let mut noise = Noise(42);
    let mut errors = Vec::new();
    for n in 0..120 {
        let truth = rig.truth_error();
        let noise = noise.pair();
        rig.tracker
            .pointing(truth[0] + noise[0], truth[1] + noise[1]);
        rig.seconds(5).await;
        if n >= 24 {
            errors.push(rig.true_error_arcsec());
        }
    }
    assert!(rig
        .tracker
        .state()
        .tracking_offset
        .iter()
        .any(|v| *v != 0.0));
    let error = rms(errors.into_iter());
    assert!(error <= 20.0, "five-second cadence RMS {error} arcsec");
    rig.tracker.stop("stopped").await;
}

#[tokio::test(start_paused = true)]
async fn pi_rotates_equatorial_error_into_axes_and_uses_the_measured_interval() {
    let rig = Rig::new(0.0).await;
    rig.tracker.pointing(0.0, 0.0);
    rig.seconds(2).await;
    let target = targets::find("M42").unwrap();
    let time = rig.now();
    let (ra, dec) = (target.ra_deg, target.dec_deg);
    // Independently differentiate the simulator's horizon transform at the
    // target, for an aim-point error of (+12, -16) arcsec. This pins rotation,
    // axis order, sign and sec(E), rather than only the resulting RMS.
    let reference = simulator::sky::horizontal(target.vector(), simulator::Site::default(), time);
    let displaced = UnitVector::from_radec(
        (ra + 12.0 / 3600.0 / dec.to_radians().cos()).to_radians(),
        (dec - 16.0 / 3600.0).to_radians(),
    );
    let measured = simulator::sky::horizontal(displaced, simulator::Site::default(), time);
    let axis_error = [
        ((measured.1 - reference.1 + 180.0).rem_euclid(360.0) - 180.0) * 3600.0,
        (measured.0 - reference.0) * 3600.0,
    ];
    rig.tracker.pointing(12.0, -16.0);
    let offset = rig.tracker.state().tracking_offset;
    for i in 0..2 {
        // The differential uses geometric E whereas PI uses apparent E.
        // At M42 here their secants differ by <0.001; allow 0.002 arcsec/s.
        assert!((offset[i] - (-0.6 * axis_error[i] / 60.0)).abs() <= 0.002);
    }
    let since = rig.now();
    rig.seconds(1).await;
    let rates = rig.rates.lock().unwrap().clone();
    for (i, axis) in AXES.into_iter().enumerate() {
        let (t, _, rate) = rates
            .iter()
            .rev()
            .find(|(t, a, _)| *t >= since && *a == axis)
            .unwrap();
        let correction = rate * 3600.0 - rig.feed_forward(*t)[i];
        assert!((correction - (-0.6 * axis_error[i] / 2.0 + offset[i])).abs() <= 0.002);
    }
    rig.tracker.stop("stopped").await;
}
