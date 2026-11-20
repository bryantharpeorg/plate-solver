//! US3: simulator truth, independent horizon differences and worked control-law
//! examples are the oracles. Every timing and tolerance is explicit below.
use crate::support::live as support;

use finder::{
    imaging::steering::{Steered, Stepped, TrackingError},
    link::{LinkManager, Transport},
    model::Model,
    session::{Session, Site},
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
        let (alt, az) = apparent(target, START);
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
    fn target_ray(&self) -> UnitVector {
        let (alt, az) = apparent(targets::find("M42").unwrap(), self.now());
        equatorial(alt, az, self.now())
    }
    fn truth_error(&self) -> [f64; 2] {
        let d = self.mount.true_direction();
        error(equatorial(d.alt, d.az, self.now()), self.target_ray())
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
    // Horizon displacement converted independently back into the sky's tangent plane.
    fn axis_displacement(&self, axis: [f64; 2], time: f64) -> [f64; 2] {
        let (alt, az) = apparent(targets::find("M42").unwrap(), time);
        error(
            equatorial(alt + axis[1] / 3600.0, az + axis[0] / 3600.0, time),
            equatorial(alt, az, time),
        )
    }
    fn steer(&self, time: f64, e: [f64; 2]) {
        self.tracker.steer(TrackingError {
            time,
            east_arcsec: e[0],
            north_arcsec: e[1],
        });
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
    async fn sub(&self, feedback: bool, late: bool, previous: &mut Option<TrackingError>) -> Sub {
        let call = self.now();
        if feedback {
            self.tracker.gap().await;
        }
        let answer = self.now();
        let offset = self.tracker.state().tracking_offset;
        let e0 = self.truth_error();
        self.wait(2.5).await;
        let em = self.truth_error();
        let tm = self.now();
        self.wait(2.5).await;
        let e1 = self.truth_error();
        self.wait(1.0).await;
        if feedback {
            let measured = TrackingError {
                time: tm,
                east_arcsec: em[0],
                north_arcsec: em[1],
            };
            if late {
                if let Some(e) = previous.replace(measured) {
                    self.tracker.steer(e);
                }
            } else {
                self.tracker.steer(measured);
            }
        }
        Sub {
            call,
            answer,
            offset,
            em,
            drift: length([e1[0] - e0[0], e1[1] - e0[1]]),
        }
    }
}
struct Sub {
    call: f64,
    answer: f64,
    offset: [f64; 2],
    em: [f64; 2],
    drift: f64,
}
fn length(e: [f64; 2]) -> f64 {
    e[0].hypot(e[1])
}
fn step(s: Stepped) -> [f64; 2] {
    [s.east_arcsec, s.north_arcsec]
}
fn close(actual: [f64; 2], expected: [f64; 2], tolerance: f64) {
    for i in 0..2 {
        assert!(
            (actual[i] - expected[i]).abs() <= tolerance,
            "{actual:?} vs {expected:?}, tolerance {tolerance}"
        );
    }
}
// Local xorshift64 and Box-Muller, sigma 15 arcseconds on each sky coordinate.
struct Noise(u64);
impl Noise {
    fn uniform(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        ((self.0 >> 11) as f64 + 0.5) / (1_u64 << 53) as f64
    }
    fn pair(&mut self) -> [f64; 2] {
        let r = 15.0 * (-2.0 * self.uniform().ln()).sqrt();
        let (s, c) = (std::f64::consts::TAU * self.uniform()).sin_cos();
        [r * c, r * s]
    }
}

// S1: simulator truth; 1% of M42's 14.97 arcsec/s is 0.749 arcsec in 5 s.
#[tokio::test(start_paused = true)]
async fn subs_remove_rate_error_even_when_delivered_a_sub_late() {
    for late in [false, true] {
        let rig = Rig::new(0.01).await;
        let mut previous = None;
        for n in 0..40 {
            let sub = rig.sub(true, late, &mut previous).await;
            if n == 0 {
                assert!(sub.drift >= 0.6, "first drift {}", sub.drift);
                assert!(rig.tracker.steered_by_subs());
            }
            if n >= 11 {
                assert!(
                    sub.drift <= 0.15,
                    "sub {}, late {late}: drift {}",
                    n + 1,
                    sub.drift
                );
                assert!(
                    length(sub.em) <= 3.0,
                    "sub {}, late {late}: error {:?}",
                    n + 1,
                    sub.em
                );
            }
        }
        if !late {
            let expected = -0.0099 * rig.feed_forward(rig.now())[0];
            assert!(
                (rig.tracker.state().tracking_offset[0] - expected).abs() <= expected.abs() * 0.1
            );
        }
        rig.tracker.stop("stopped").await;
    }
    let rig = Rig::new(0.01).await;
    for _ in 0..20 {
        assert!(rig.sub(false, false, &mut None).await.drift >= 0.6);
        assert_eq!(rig.tracker.state().tracking_offset, [0.0; 2]);
    }
    rig.tracker.stop("stopped").await;
}

// S2: noise is independent of the simulator; truth measures smear, not the solves.
#[tokio::test(start_paused = true)]
async fn noisy_finder_pointings_smear_subs_without_feedback() {
    let rig = Arc::new(Rig::new(0.01).await);
    let task_rig = rig.clone();
    let pointings = tokio::spawn(async move {
        let mut noise = Noise(42);
        loop {
            let e = task_rig.truth_error();
            let n = noise.pair();
            task_rig.tracker.pointing(e[0] + n[0], e[1] + n[1]);
            task_rig.wait(2.0).await;
        }
    });
    let mut drifts = Vec::new();
    for _ in 0..20 {
        drifts.push(rig.sub(false, false, &mut None).await.drift);
    }
    pointings.abort();
    drifts.sort_by(f64::total_cmp);
    assert!((drifts[9] + drifts[10]) / 2.0 > 3.0, "drifts {drifts:?}");
    rig.tracker.stop("stopped").await;
}

// S3: two counts = 1.6 arcsec here; 1.9 counts is below the axis deadband.
#[tokio::test(start_paused = true)]
async fn under_two_counts_neither_steps_nor_changes_rate() {
    let rig = Rig::new(0.0).await;
    rig.tracker.gap().await;
    let count = 1_296_000.0 / f64::from(rig.mount.axes()[1].counts_per_revolution);
    rig.steer(
        rig.now(),
        rig.axis_displacement([0.0, 1.9 * count], rig.now()),
    );
    let before = rig.truth_error();
    assert_eq!(rig.tracker.gap().await, Stepped::default());
    assert_eq!(rig.tracker.state().tracking_offset, [0.0; 2]);
    close(rig.truth_error(), before, 0.05);
    rig.tracker.stop("stopped").await;
}

// Independent finite difference of the simulator's horizon transform (015 PI oracle).
fn axis_error(time: f64, e: [f64; 2]) -> [f64; 2] {
    let target = targets::find("M42").unwrap();
    let displaced = UnitVector::from_radec(
        (target.ra_deg + e[0] / 3600.0 / target.dec_deg.to_radians().cos()).to_radians(),
        (target.dec_deg + e[1] / 3600.0).to_radians(),
    );
    let reference = simulator::sky::horizontal(target.vector(), simulator::Site::default(), time);
    let measured = simulator::sky::horizontal(displaced, simulator::Site::default(), time);
    [
        ((measured.1 - reference.1 + 180.0).rem_euclid(360.0) - 180.0) * 3600.0,
        (measured.0 - reference.0) * 3600.0,
    ]
}

// S4: large pulse pairs are excluded; a small pair measures 0.3 of its residual drift.
#[tokio::test(start_paused = true)]
async fn large_steps_do_not_measure_drift_but_small_steps_do() {
    let rig = Rig::new(0.0).await;
    rig.tracker.gap().await;
    rig.steer(rig.now(), [12.0, -16.0]);
    close(step(rig.tracker.gap().await), [-6.0, 8.0], 0.1);
    assert_eq!(rig.tracker.state().tracking_offset, [0.0; 2]);
    rig.wait(6.0).await;
    rig.steer(rig.now(), [8.0, -8.0]);
    close(step(rig.tracker.gap().await), [-4.0, 4.0], 0.1);
    assert_eq!(rig.tracker.state().tracking_offset, [0.0; 2]);
    rig.wait(6.0).await;
    let t0 = rig.now();
    rig.steer(t0, [4.0, -4.0]);
    close(step(rig.tracker.gap().await), [-2.0, 2.0], 0.1);
    assert_eq!(rig.tracker.state().tracking_offset, [0.0; 2]);
    rig.wait(6.0).await;
    let t1 = rig.now();
    rig.steer(t1, [3.3, -2.0]);
    rig.tracker.gap().await;
    let expected = axis_error(t1, [1.3, 0.0]).map(|d| -0.3 * d / (t1 - t0));
    close(rig.tracker.state().tracking_offset, expected, 0.002);
    rig.tracker.stop("stopped").await;
}

// S5: reject a jump over twice the offset limit; the 30 arcsec cap is on the sky.
#[tokio::test(start_paused = true)]
async fn jumps_set_no_rate_and_steps_are_limited_on_the_sky() {
    let rig = Rig::new(0.0).await;
    rig.tracker.gap().await;
    rig.steer(rig.now(), [0.0; 2]);
    rig.tracker.gap().await;
    rig.wait(6.0).await;
    let e = rig.axis_displacement([-20.0, 0.0], rig.now());
    rig.steer(rig.now(), e);
    close(step(rig.tracker.gap().await), e.map(|v| -0.5 * v), 0.2);
    close(rig.tracker.state().tracking_offset, [0.0; 2], 0.002);
    rig.wait(6.0).await;
    rig.steer(rig.now(), rig.axis_displacement([-100.0, 0.0], rig.now()));
    let made = step(rig.tracker.gap().await);
    let alt = apparent(targets::find("M42").unwrap(), rig.now()).0;
    let limit = length(rig.axis_displacement([30.0 / alt.to_radians().cos(), 0.0], rig.now()));
    assert!(
        (length(made) - limit).abs() <= 0.03 * limit,
        "{made:?}, cap {limit}"
    );
    rig.tracker.stop("stopped").await;
}

// S8: -6 arcsec in 6 seconds asks for +0.3 arcsec/s, clamped to +0.1 in a gap.
#[tokio::test(start_paused = true)]
async fn each_gap_changes_the_rate_by_at_most_a_tenth() {
    let rig = Rig::new(0.0).await;
    rig.tracker.gap().await;
    rig.steer(rig.now(), [0.0; 2]);
    rig.tracker.gap().await;
    rig.wait(6.0).await;
    rig.steer(rig.now(), rig.axis_displacement([-6.0, 0.0], rig.now()));
    rig.tracker.gap().await;
    let offset = rig.tracker.state().tracking_offset;
    assert!((offset[0] - 0.1).abs() <= 0.001, "{offset:?}");
    assert!(offset[1].abs() <= 0.002, "{offset:?}");
    rig.tracker.stop("stopped").await;
}

// S6: claims stop finder PI without losing the live samples. Trait dispatch is used too.
#[tokio::test(start_paused = true)]
async fn claim_ends_finder_pi_but_keeps_its_samples() {
    let rig = Rig::new(0.0).await;
    let tracker: &dyn Steered = rig.tracker.as_ref();
    tracker.gap().await;
    assert!(tracker.steered_by_subs());
    let since = rig.now() + 0.001;
    for _ in 0..10 {
        rig.tracker.pointing(30.0, 0.0);
        rig.wait(2.0).await;
    }
    rig.assert_rates(since, rig.now(), [0.0; 2], 0.001);
    let state = rig.tracker.state();
    assert_eq!(state.tracking_offset, [0.0; 2]);
    assert!(state.samples.len() >= 9);
    assert!(state
        .samples
        .iter()
        .all(|s| s.east_arcsec == 30.0 && s.north_arcsec == 0.0));
    rig.tracker.stop("stopped").await;

    let rig = Rig::new(0.0).await;
    rig.tracker.pointing(30.0, 0.0);
    rig.tracker.gap().await;
    let since = rig.now() + 0.001;
    rig.wait(2.0).await;
    rig.assert_rates(since, rig.now(), [0.0; 2], 0.001);
    rig.tracker.stop("stopped").await;
}

// S7: both acknowledgements belong to each gap; all intervening rates are FF + offset.
#[tokio::test(start_paused = true)]
async fn rates_stay_as_the_gap_left_them_and_nothing_follows_stop() {
    let rig = Rig::new(0.01).await;
    let mut previous = None;
    let mut subs: Vec<Sub> = Vec::new();
    for _ in 0..10 {
        let sub = rig.sub(true, false, &mut previous).await;
        if let Some(last) = subs.last() {
            rig.assert_rates(last.answer + 0.001, sub.call - 0.001, last.offset, 0.001);
        }
        let log = rig.rates.lock().unwrap();
        for axis in AXES {
            assert!(log
                .iter()
                .any(|(t, a, _)| *t >= sub.call && *t <= sub.answer && *a == axis));
        }
        drop(log);
        subs.push(sub);
    }
    let tracker: &dyn Steered = rig.tracker.as_ref();
    let target = targets::find("M42").unwrap();
    assert_eq!(tracker.target(), Some((target.ra_deg, target.dec_deg)));
    rig.tracker.stop("stopped").await;
    assert_eq!(tracker.target(), None);
    assert!(!tracker.steered_by_subs());
    let count = rig.rates.lock().unwrap().len();
    let now = rig.now();
    assert_eq!(tracker.gap().await, Stepped::default());
    assert_eq!(rig.now(), now);
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert_eq!(rig.rates.lock().unwrap().len(), count);
}

// S9: late steps are judged against gap history; only the newest valid error is used.
#[tokio::test(start_paused = true)]
async fn late_errors_account_for_steps_and_obey_time_order() {
    let rig = Rig::new(0.0).await;
    rig.tracker.gap().await;
    let t = rig.now();
    rig.steer(t - 3.0, [12.0, -16.0]);
    close(step(rig.tracker.gap().await), [-6.0, 8.0], 0.1);
    assert_eq!(rig.tracker.state().tracking_offset, [0.0; 2]);
    rig.steer(t - 1.0, [12.0, -16.0]);
    close(step(rig.tracker.gap().await), [-3.0, 4.0], 0.1);
    close(rig.tracker.state().tracking_offset, [0.0; 2], 0.002);
    rig.tracker.stop("stopped").await;

    let rig = Rig::new(0.0).await;
    rig.tracker.gap().await;
    let t = rig.now();
    rig.steer(t - 2.0, [12.0, -16.0]);
    rig.steer(t - 1.0, [6.0, -8.0]);
    close(step(rig.tracker.gap().await), [-3.0, 4.0], 0.1);
    rig.tracker.stop("stopped").await;

    let rig = Rig::new(0.0).await;
    rig.tracker.gap().await;
    let t = rig.now();
    rig.steer(t, [12.0, -16.0]);
    rig.tracker.gap().await;
    rig.steer(t - 1.0, [12.0, -16.0]);
    assert_eq!(rig.tracker.gap().await, Stepped::default());
    rig.tracker.stop("stopped").await;
}

// S10: two late equal drifts remove 0.3 + 0.21, not 0.3 + 0.3, of the rate.
#[tokio::test(start_paused = true)]
async fn late_errors_account_for_rate_changes_they_did_not_see() {
    let rig = Rig::new(0.0).await;
    rig.tracker.gap().await;
    let t = rig.now();
    rig.steer(t - 20.0, [0.0; 2]);
    assert_eq!(rig.tracker.gap().await, Stepped::default());
    rig.steer(t - 14.0, [0.5, 0.0]);
    assert_eq!(rig.tracker.gap().await, Stepped::default());
    let o1 = rig.tracker.state().tracking_offset;
    let expected = axis_error(t - 14.0, [0.5, 0.0]).map(|d| -0.3 * d / 6.0);
    close(o1, expected, 0.002);
    rig.steer(t - 8.0, [1.0, 0.0]);
    assert_eq!(rig.tracker.gap().await, Stepped::default());
    let o2 = rig.tracker.state().tracking_offset;
    close(o2, o1.map(|v| v * 1.7), 0.003);
    assert!((o2[0] - 2.0 * o1[0]).abs() > 0.006, "{o1:?} -> {o2:?}");
    rig.tracker.stop("stopped").await;
}

// S11: a lapsed claim holds only the newest P step, never an integral or a rate.
#[tokio::test(start_paused = true)]
async fn lapsed_finder_steps_wait_for_gaps_and_are_remembered_on_reclaim() {
    let rig = Rig::new(0.0).await;
    rig.tracker.gap().await;
    for _ in 0..8 {
        rig.wait(6.0).await;
        rig.tracker.gap().await;
    }
    assert!(!rig.tracker.steered_by_subs());
    let o = rig.tracker.state().tracking_offset;
    assert_eq!(o, [0.0; 2]);
    let e0 = rig.truth_error();
    let time = rig.now();
    rig.tracker.pointing(30.0, 0.0);
    rig.wait(2.0).await;
    rig.tracker.pointing(30.0, 0.0);
    rig.wait(1.0).await;
    let call = rig.now();
    rig.assert_rates(time + 0.001, call - 0.001, [0.0; 2], 0.001);
    assert_eq!(rig.tracker.state().tracking_offset, o);
    let made = step(rig.tracker.gap().await);
    close(made, [-18.0, 0.0], 0.1);
    let e1 = rig.truth_error();
    close([e1[0] - e0[0], e1[1] - e0[1]], made, 0.5);
    assert_eq!(rig.tracker.state().tracking_offset, o);
    assert!(!rig.tracker.steered_by_subs());
    let log = rig.rates.lock().unwrap().clone();
    for (i, axis) in AXES.into_iter().enumerate() {
        let (t, _, rate) = log.iter().rev().find(|(_, a, _)| *a == axis).unwrap();
        assert!((rate * 3600.0 - rig.feed_forward(*t)[i] - o[i]).abs() <= 0.01);
    }
    rig.steer(time, [36.0, 0.0]);
    close(
        step(rig.tracker.gap().await),
        [-0.5 * (36.0 + made[0]), -0.5 * made[1]],
        0.3,
    );
    assert!(rig.tracker.steered_by_subs());
    rig.tracker.stop("stopped").await;
}
