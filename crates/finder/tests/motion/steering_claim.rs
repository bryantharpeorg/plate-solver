//! US9: the claim edges on US3's unchanged tracker. Simulator truth, independent
//! horizon differences and recorded rates are the oracles; time is paused.
//! The rig is local to this module, as required by the story's test contract.
use crate::support::live as support;

use finder::{
    imaging::steering::{Stepped, TrackingError},
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
        Self::start(rate_error, true).await
    }
    async fn start(rate_error: f64, wait_for_rates: bool) -> Self {
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
        if wait_for_rates {
            for _ in 0..500 {
                if AXES
                    .iter()
                    .all(|axis| rates.lock().unwrap().iter().any(|(_, a, _)| a == axis))
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            assert!(AXES.iter().all(|axis| rates
                .lock()
                .unwrap()
                .iter()
                .any(|(_, a, _)| a == axis)));
            tokio::time::sleep(Duration::from_millis(3500)).await;
        }
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
    fn assert_rates(&self, start: f64, end: f64, offset: [f64; 2]) {
        for (i, extra) in offset.into_iter().enumerate() {
            self.assert_axis_rates(start, end, i, extra);
        }
    }
    fn assert_axis_rates(&self, start: f64, end: f64, i: usize, offset: f64) {
        let log = self.rates.lock().unwrap();
        let axis = AXES[i];
        let entries: Vec<_> = log
            .iter()
            .filter(|(t, a, _)| *t >= start && *t <= end && *a == axis)
            .collect();
        assert!(!entries.is_empty(), "no {axis:?} rate in {start}..{end}");
        for (t, _, rate) in entries {
            let expected = self.feed_forward(*t)[i] + offset;
            assert!(
                (rate * 3600.0 - expected).abs() <= 0.001,
                "{axis:?}: {} vs {expected} arcsec/s at {t}",
                rate * 3600.0
            );
        }
    }
    async fn sub(&self) -> f64 {
        self.tracker.gap().await;
        assert!(self.tracker.steered_by_subs());
        let e0 = self.truth_error();
        self.wait(2.5).await;
        let em = self.truth_error();
        let tm = self.now();
        self.wait(2.5).await;
        let e1 = self.truth_error();
        self.wait(1.0).await;
        self.steer(tm, em);
        length([e1[0] - e0[0], e1[1] - e0[1]])
    }
    async fn lapse(&self, interval: f64) {
        self.tracker.gap().await;
        for _ in 0..8 {
            self.wait(interval).await;
            self.tracker.gap().await;
        }
        assert!(!self.tracker.steered_by_subs());
    }
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

fn noisy_pointings(rig: Arc<Rig>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut noise = Noise(42);
        loop {
            let e = rig.truth_error();
            let n = noise.pair();
            rig.tracker.pointing(e[0] + n[0], e[1] + n[1]);
            rig.wait(2.0).await;
        }
    })
}

// S1: simulator truth in forty 5 s subs; the finder has sigma 15″ per coordinate.
#[tokio::test(start_paused = true)]
async fn subs_steer_while_noisy_finder_pointings_do_not() {
    let rig = Arc::new(Rig::new(0.01).await);
    let pointings = noisy_pointings(rig.clone());
    for n in 1..=40 {
        let drift = rig.sub().await;
        if n >= 12 {
            assert!(drift <= 0.15, "sub {n}: true drift {drift} arcsec");
        }
    }
    pointings.abort();
    rig.tracker.stop("stopped").await;
}

// S2: 2.1 altitude counts and 3 azimuth counts exceed the two-count deadband.
// Half the independent sky displacement is removed in 0.5..0.6 s. The other
// axis keeps only feed-forward, including during the pulse (0.001″/s tolerance).
#[tokio::test(start_paused = true)]
async fn over_deadband_steps_only_the_axis_with_an_error() {
    for (axis, counts) in [(1, 2.1), (0, 3.0)] {
        let rig = Rig::new(0.0).await;
        rig.tracker.gap().await;
        let mut displacement = [0.0; 2];
        displacement[axis] =
            counts * 1_296_000.0 / f64::from(rig.mount.axes()[axis].counts_per_revolution);
        let e = rig.axis_displacement(displacement, rig.now());
        rig.steer(rig.now(), e);
        let before = rig.truth_error();
        let call = rig.now();
        let made = step(rig.tracker.gap().await);
        let answer = rig.now();
        close(made, e.map(|v| -0.5 * v), 0.05);
        let after = rig.truth_error();
        close([after[0] - before[0], after[1] - before[1]], made, 0.05);
        assert!(
            (0.5..=0.6).contains(&(answer - call)),
            "gap {} s",
            answer - call
        );
        rig.assert_axis_rates(call, answer, 1 - axis, 0.0);
        rig.tracker.stop("stopped").await;
    }
}

// S3: the independent FF rate gives the most an axis can lose by holding still
// for half a second. A 60″ azimuth error asks for a larger step against motion.
#[tokio::test(start_paused = true)]
async fn step_against_motion_can_only_hold_the_axis_still() {
    let rig = Rig::new(0.0).await;
    rig.tracker.gap().await;
    let time = rig.now();
    let rate = rig.feed_forward(time)[0];
    assert!(rate > 0.0, "M42's azimuth must be moving forward: {rate}");
    rig.steer(time, rig.axis_displacement([60.0, 0.0], time));
    let made = step(rig.tracker.gap().await);
    let expected = length(rig.axis_displacement([rate / 2.0, 0.0], time));
    assert!(
        (length(made) - expected).abs() <= 0.3,
        "step {made:?}, expected length {expected}"
    );
    rig.tracker.stop("stopped").await;
}

// S4: FR-011's eight consecutive idle gaps, exact claim states/zero steps, and
// recorded FF rates (0.001″/s). A reclaim forgets the held finder step.
#[tokio::test(start_paused = true)]
async fn eighth_idle_gap_lapses_until_an_error_and_reclaim_forgets_held_step() {
    let rig = Rig::new(0.0).await;
    rig.tracker.gap().await;
    for _ in 0..7 {
        rig.tracker.gap().await;
        assert!(rig.tracker.steered_by_subs());
    }
    rig.wait(0.2).await;
    let count = rig.rates.lock().unwrap().len();
    assert_eq!(rig.tracker.gap().await, Stepped::default());
    assert!(!rig.tracker.steered_by_subs());
    assert_eq!(
        rig.rates.lock().unwrap().len(),
        count,
        "lapse gap sent a rate"
    );
    assert_eq!(rig.tracker.gap().await, Stepped::default());
    assert!(!rig.tracker.steered_by_subs());

    let since = rig.now();
    rig.tracker.pointing(30.0, 0.0);
    rig.wait(2.0).await;
    rig.tracker.pointing(30.0, 0.0);
    rig.wait(1.0).await;
    rig.assert_rates(since + 0.001, rig.now() - 0.001, [0.0; 2]);
    assert_eq!(rig.tracker.state().tracking_offset, [0.0; 2]);
    let offset = rig.tracker.state().tracking_offset;
    rig.steer(rig.now(), [0.0; 2]);
    assert_eq!(rig.tracker.gap().await, Stepped::default());
    assert!(rig.tracker.steered_by_subs());
    assert_eq!(rig.tracker.state().tracking_offset, offset);
    for _ in 0..8 {
        rig.tracker.gap().await;
    }
    assert!(!rig.tracker.steered_by_subs());
    assert_eq!(rig.tracker.gap().await, Stepped::default());
    assert!(!rig.tracker.steered_by_subs());
    rig.tracker.stop("stopped").await;

    let rig = Rig::new(0.0).await;
    rig.tracker.gap().await;
    for _ in 0..7 {
        rig.tracker.gap().await;
    }
    rig.steer(rig.now(), [0.0; 2]);
    rig.tracker.gap().await;
    for _ in 0..7 {
        rig.tracker.gap().await;
        assert!(
            rig.tracker.steered_by_subs(),
            "steer did not reset idle count"
        );
    }
    rig.tracker.stop("stopped").await;
}

// S5: FR-011 exact release/reset behaviour, including a pending error, a lapse
// and a finder pointing whose integral history must not cross the claim.
#[tokio::test(start_paused = true)]
async fn release_forgets_pending_errors_and_the_next_gap_claims_afresh() {
    for error_before_release in [false, true] {
        let rig = Rig::new(0.0).await;
        rig.tracker.gap().await;
        if error_before_release {
            rig.steer(rig.now(), [12.0, -16.0]);
        }
        rig.tracker.release();
        assert!(!rig.tracker.steered_by_subs());
        if !error_before_release {
            rig.steer(rig.now(), [12.0, -16.0]);
        }
        assert_eq!(rig.tracker.gap().await, Stepped::default());
        assert!(rig.tracker.steered_by_subs());
        rig.tracker.stop("stopped").await;
    }
    let rig = Rig::new(0.0).await;
    rig.lapse(0.0).await;
    rig.tracker.release();
    assert_eq!(rig.tracker.gap().await, Stepped::default());
    assert!(rig.tracker.steered_by_subs());
    rig.tracker.stop("stopped").await;

    let rig = Rig::new(0.0).await;
    rig.tracker.pointing(30.0, 0.0);
    rig.wait(2.0).await;
    rig.tracker.gap().await;
    rig.tracker.release();
    rig.tracker.pointing(30.0, 0.0);
    assert_eq!(rig.tracker.state().tracking_offset, [0.0; 2]);
    rig.tracker.stop("stopped").await;
}

// S6: nonfinite errors count as no error; one before the first gap is dropped.
// Oracle: FR-011, exact claim/offset/zero step, with no numerical tolerance.
#[tokio::test(start_paused = true)]
async fn nonfinite_errors_keep_no_claim_and_unclaimed_errors_are_dropped() {
    let rig = Rig::new(0.0).await;
    rig.tracker.gap().await;
    for _ in 0..8 {
        rig.steer(rig.now(), [f64::NAN, 0.0]);
        rig.tracker.gap().await;
    }
    assert!(!rig.tracker.steered_by_subs());
    rig.tracker.stop("stopped").await;

    let rig = Rig::new(0.0).await;
    rig.steer(rig.now(), [12.0, -16.0]);
    assert_eq!(rig.tracker.gap().await, Stepped::default());
    assert_eq!(rig.tracker.state().tracking_offset, [0.0; 2]);
    rig.tracker.stop("stopped").await;
}

// S7: before the first rate-task turn there is no direction loaded. The gap
// must answer exactly zero without claiming or sending a preliminary zero rate.
#[tokio::test(start_paused = true)]
async fn gap_before_first_rates_neither_claims_nor_sends_zero() {
    let rig = Rig::start(0.0, false).await;
    assert!(rig.rates.lock().unwrap().is_empty());
    assert_eq!(rig.tracker.gap().await, Stepped::default());
    assert!(!rig.tracker.steered_by_subs());
    rig.wait(0.1).await;
    let log = rig.rates.lock().unwrap().clone();
    for (i, axis) in AXES.into_iter().enumerate() {
        let (time, _, rate) = log.iter().find(|(_, a, _)| *a == axis).expect("first rate");
        let expected = rig.feed_forward(*time)[i];
        assert!(
            (rate * 3600.0 - expected).abs() <= 0.001,
            "{axis:?}: first rate {} vs {expected}",
            rate * 3600.0
        );
    }
    rig.tracker.stop("stopped").await;
}

// S8: a zero timeout polls once and drops the receiver before the rate task
// serves it. Plan trap 18 requires the claim to remain absent, exactly.
#[tokio::test(start_paused = true)]
async fn gap_dropped_after_one_poll_claims_nothing() {
    let rig = Rig::new(0.0).await;
    assert!(tokio::time::timeout(Duration::ZERO, rig.tracker.gap())
        .await
        .is_err());
    rig.wait(0.1).await;
    assert!(!rig.tracker.steered_by_subs());
    rig.tracker.stop("stopped").await;
}

// S9: real finder frames still update the session's own pointing model and
// the tracker's samples. Only PI is suppressed: offset tolerance 0.002 arcsec/s.
#[tokio::test(start_paused = true)]
async fn finder_keeps_syncing_the_pointing_model_while_subs_steer() {
    let rig = crate::support::goto::Rig::new(START, 50.0, 120.0, true).await;
    let target = targets::find("M42").unwrap();
    rig.sim.point_eyepiece_at(target.ra_deg, target.dec_deg);
    rig.sync().await;
    for _ in 0..10 {
        let pointing = rig.view().await["pointing"].clone();
        if finder::model::separation(
            pointing["ra_deg"].as_f64().unwrap(),
            pointing["dec_deg"].as_f64().unwrap(),
            target.ra_deg,
            target.dec_deg,
        ) < 1.0 / 60.0
        {
            break;
        }
        rig.tick().await;
    }
    let (status, body) = support::request(
        &rig.app,
        "POST",
        "/api/track",
        serde_json::json!({"on":true}),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["track"]["state"], "tracking", "{body}");
    assert!(
        body.get("goto").is_none(),
        "centred Track ran a GoTo: {body}"
    );
    // First rates plus 040's settling interval, measured from first tracking.
    for _ in 0..45 {
        rig.tick().await;
    }
    let tracker = rig.state.session().tracker().expect("running tracker");
    tracker.gap().await;
    assert!(tracker.steered_by_subs());
    let offset = tracker.state().tracking_offset;
    let samples = tracker.state().samples.len();
    let newest_sync = || {
        rig.state
            .session()
            .model(rig.now())
            .syncs
            .iter()
            .map(|s| s.time)
            .reduce(f64::max)
            .expect("solved sync")
    };
    let before = newest_sync();
    for _ in 0..10 {
        tracker.steer(TrackingError {
            time: rig.now(),
            east_arcsec: 0.0,
            north_arcsec: 0.0,
        });
        assert_eq!(tracker.gap().await, Stepped::default());
        assert!(tracker.steered_by_subs());
        for _ in 0..20 {
            rig.tick().await;
            assert!(tracker.steered_by_subs());
        }
    }
    let after = newest_sync();
    assert!(
        after - before >= 15.0,
        "newest sync advanced {} s",
        after - before
    );
    assert!(
        tracker.state().samples.len() > samples,
        "finder samples stopped growing"
    );
    close(tracker.state().tracking_offset, offset, 0.002);
    let (status, body) = support::request(
        &rig.app,
        "POST",
        "/api/track",
        serde_json::json!({"on":false}),
    )
    .await;
    assert_eq!(status, 200, "{body}");
}

// Independent finite difference of the simulator horizon transform, as in
// tracking/pi_loop.rs's (+12, -16) worked example; no tracker maths are called.
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

// S10: the held 015 proportional step uses its 0.6 gain, capped to 30 arcsec
// on the sky per axis. Oracle: independent transforms, length tolerance 3%.
// A newest pointing under the 10 arcsec PI deadband clears the held step exactly.
#[tokio::test(start_paused = true)]
async fn held_finder_step_is_capped_and_cleared_by_a_pointing_in_deadband() {
    let rig = Rig::new(0.0).await;
    rig.lapse(6.0).await;
    rig.tracker.pointing(100.0, 0.0);
    rig.wait(2.0).await;
    let d = axis_error(rig.now(), [100.0, 0.0]);
    rig.tracker.pointing(100.0, 0.0);
    rig.wait(1.0).await;
    let time = rig.now();
    let alt = apparent(targets::find("M42").unwrap(), time).0;
    let expected =
        length(rig.axis_displacement([30.0 / alt.to_radians().cos(), -0.6 * d[1]], time));
    let made = step(rig.tracker.gap().await);
    assert!(
        (length(made) - expected).abs() <= 0.03 * expected,
        "step {made:?}, expected length {expected}"
    );
    assert!(!rig.tracker.steered_by_subs());
    rig.tracker.stop("stopped").await;

    let rig = Rig::new(0.0).await;
    rig.lapse(6.0).await;
    rig.tracker.pointing(30.0, 0.0);
    rig.wait(2.0).await;
    rig.tracker.pointing(3.0, 4.0);
    rig.wait(1.0).await;
    assert_eq!(rig.tracker.gap().await, Stepped::default());
    rig.tracker.stop("stopped").await;
}

// S11: after a lapse, noisy finder corrections reach the mount only in gaps.
// Every intervening rate is independently computed FF + the gap's offset,
// to 0.001 arcsec/s. The offset stays exactly fixed and at least one gap steps.
#[tokio::test(start_paused = true)]
async fn lapsed_noisy_finder_pointings_step_in_gaps_and_never_change_exposure_rates() {
    let rig = Arc::new(Rig::new(0.01).await);
    let pointings = noisy_pointings(rig.clone());
    rig.lapse(6.0).await;
    let offset_at_lapse = rig.tracker.state().tracking_offset;
    let mut previous_answer = None;
    let mut stepped = false;
    for _ in 0..10 {
        rig.wait(6.0).await;
        let call = rig.now();
        if let Some((answer, offset)) = previous_answer {
            rig.assert_rates(answer + 0.001, call - 0.001, offset);
        }
        let made = step(rig.tracker.gap().await);
        assert!(!rig.tracker.steered_by_subs());
        stepped |= length(made) > 1.0;
        previous_answer = Some((rig.now(), rig.tracker.state().tracking_offset));
    }
    assert_eq!(rig.tracker.state().tracking_offset, offset_at_lapse);
    let (answer, offset) = previous_answer.unwrap();
    rig.wait(6.0).await;
    rig.assert_rates(answer + 0.001, rig.now(), offset);
    assert!(
        stepped,
        "none of the ten gaps applied a finder step over 1 arcsec"
    );
    pointings.abort();
    rig.tracker.stop("stopped").await;
}

// S12: FR-011 / 038 follow. An old error is dropped both when pending at follow
// and when delivered late after reclaim; a fresh error steps half (0.1 arcsec).
#[tokio::test(start_paused = true)]
async fn followed_push_forgets_pending_and_late_errors_from_the_old_place() {
    let rig = Rig::new(0.0).await;
    rig.tracker.gap().await;
    rig.wait(6.0).await;
    rig.steer(rig.now(), [12.0, -16.0]);
    let followed_at = rig.now();
    let target = targets::find("M42").unwrap();
    let east = target.ra_deg + 600.0 / 3600.0 / target.dec_deg.to_radians().cos();
    assert_eq!(rig.tracker.follow(east, target.dec_deg), Ok(()));
    assert!(!rig.tracker.steered_by_subs());
    assert_eq!(rig.tracker.gap().await, Stepped::default());
    assert!(rig.tracker.steered_by_subs());
    rig.steer(followed_at - 2.5, [-600.0, 0.0]);
    assert_eq!(rig.tracker.gap().await, Stepped::default());
    rig.wait(1.0).await;
    rig.steer(rig.now(), [12.0, -16.0]);
    close(step(rig.tracker.gap().await), [-6.0, 8.0], 0.1);
    rig.tracker.stop("stopped").await;
}

// S13: a follow clears both the lapse and the old place's held finder step.
// Oracle: FR-011's exact zero step and a fresh claim at the next gap.
#[tokio::test(start_paused = true)]
async fn followed_push_clears_a_lapse_and_its_held_finder_step() {
    let rig = Rig::new(0.0).await;
    rig.lapse(0.0).await;
    rig.tracker.pointing(30.0, 0.0);
    rig.wait(2.0).await;
    rig.tracker.pointing(30.0, 0.0);
    let target = targets::find("M42").unwrap();
    let east = target.ra_deg + 600.0 / 3600.0 / target.dec_deg.to_radians().cos();
    assert_eq!(rig.tracker.follow(east, target.dec_deg), Ok(()));
    assert!(!rig.tracker.steered_by_subs());
    assert_eq!(rig.tracker.gap().await, Stepped::default());
    assert!(rig.tracker.steered_by_subs());
    rig.tracker.stop("stopped").await;
}
