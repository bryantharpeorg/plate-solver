//! US4 oracles: the simulator's physical eyepiece, a seeded Box–Muller
//! sigma-3″ feed, and catalog-rendered pixels passed through finder::solve.
//! The simulator has geometric optics: the desired ray includes the independently
//! evaluated Meeus 16.4 refraction, as in tracking_sim's feed-forward oracle.
#[path = "live/support.rs"]
mod support;

use finder::{
    link::{LinkManager, Transport},
    model::Model,
    session::{Session, Site},
    solve::{self, GrayImage, SolveOptions, Status},
    targets::{self, Target},
    tracking::Tracker,
};
use math_core::UnitVector;
use scope_link::memory;
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
const ARCSEC: f64 = 180.0 * 3600.0 / std::f64::consts::PI;
const OFFSET_LIMIT: f64 = 0.05 * 15.041;

struct Noise(u64);
impl Noise {
    fn uniform(&mut self) -> f64 {
        // Local deterministic generator: no simulator-private RNG or dependency.
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        ((self.0 >> 11) as f64 + 0.5) / ((1_u64 << 53) as f64)
    }
    fn pair(&mut self) -> [f64; 2] {
        let r = 3.0 * (-2.0 * self.uniform().ln()).sqrt();
        let theta = std::f64::consts::TAU * self.uniform();
        [r * theta.cos(), r * theta.sin()]
    }
}

fn desired(target: &Target, time: f64) -> (f64, f64) {
    let (h, az) = simulator::sky::horizontal(target.vector(), Default::default(), time);
    (
        h + 1.02 / (h + 10.3 / (h + 5.11)).to_radians().tan() / 60.0,
        az,
    )
}
fn ray(alt: f64, az: f64, time: f64) -> UnitVector {
    simulator::sky::to_j2000(
        simulator::sky::horizontal_to_date(alt, az, Default::default(), time),
        simulator::sky::julian_date(time),
    )
}
fn dot(v: UnitVector, w: UnitVector) -> f64 {
    v.x * w.x + v.y * w.y + v.z * w.z
}
fn error(aim: UnitVector, target: UnitVector) -> [f64; 2] {
    let (ra, dec) = target.to_radec();
    let east = UnitVector {
        x: -ra.sin(),
        y: ra.cos(),
        z: 0.0,
    };
    let north = UnitVector {
        x: -dec.sin() * ra.cos(),
        y: -dec.sin() * ra.sin(),
        z: dec.cos(),
    };
    [east, north].map(|t| dot(aim, t).atan2(dot(aim, target)) * ARCSEC)
}
fn rms(errors: &[[f64; 2]]) -> f64 {
    assert!(!errors.is_empty());
    (errors
        .iter()
        .map(|e| e[0].powi(2) + e[1].powi(2))
        .sum::<f64>()
        / errors.len() as f64)
        .sqrt()
}

struct NodeTime(VirtualClock);
impl NodeClock for NodeTime {
    fn now(&self) -> f64 {
        self.0.node_now()
    }
}
#[derive(Clone, Copy)]
struct Rate {
    time: f64,
    axis: usize,
    rate: f64,
}
#[derive(Clone)]
struct Recorded {
    mount: SimMount,
    clock: VirtualClock,
    rates: Arc<Mutex<Vec<Rate>>>,
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
            self.rates.lock().unwrap().push(Rate {
                time: self.clock.now(),
                axis: axis.index(),
                rate,
            });
            self.mount.rate(axis, rate).await;
        })
    }
    fn stop_all(&self) -> MountFuture<'_, ()> {
        self.mount.stop_all()
    }
}

struct Rig {
    clock: VirtualClock,
    sim: Simulator,
    mount: SimMount,
    model: Model,
    tracker: Tracker,
    rates: Arc<Mutex<Vec<Rate>>>,
    controls: NodeControls,
}
impl Rig {
    async fn new(rate_error: f64) -> Self {
        let target = targets::find("M42").unwrap();
        let clock = VirtualClock::new(START);
        let mut model = Model::default();
        model.load_known_terms(1.0, 130.0, [0.0; 2]);
        let (alt, az) = desired(target, START);
        let mount = SimMount::with_config(
            clock.clone(),
            MountConfig {
                base_tilt_deg: 1.0,
                base_tilt_toward_deg: 130.0,
                zero_points_deg: model.inverse(az, alt, [360; 2]).unwrap(),
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
        let sim = Simulator::new(DB.clone(), Default::default(), clock.clone());
        sim.connect_mount(mount.clone());
        let rates = Arc::new(Mutex::new(Vec::new()));
        let controls = NodeControls::default();
        controls.stop_frames();
        let node = NodeService::with_controls_and_mount(
            SimCamera::new(sim.clone()),
            Arc::new(NodeTime(clock.clone())),
            "test-token",
            Default::default(),
            controls.clone(),
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
                    memory::serve_routes_on(server, node.routes())
                        .await
                        .unwrap();
                });
                Ok::<_, std::io::Error>(client)
            }
        });
        let session = Arc::new(Session::default());
        session
            .edit(
                serde_json::from_value(serde_json::json!({
                    "node": {"address": "scope.local:1", "token": "test-token"}
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
            tracker,
            rates,
            controls,
        }
    }
    fn desired_ray(&self) -> UnitVector {
        let (h, az) = desired(targets::find("M42").unwrap(), self.clock.now());
        ray(h, az, self.clock.now())
    }
    fn truth_error(&self) -> [f64; 2] {
        let aim = self.mount.true_direction();
        error(ray(aim.alt, aim.az, self.clock.now()), self.desired_ray())
    }
    fn forward(&self, time: f64) -> [f64; 2] {
        let counts = self.mount.axes().map(|a| a.counts_per_revolution);
        let positions = [time, time + 1.0].map(|t| {
            let (h, az) = desired(targets::find("M42").unwrap(), t);
            self.model.inverse(az, h, counts).unwrap()
        });
        [0, 1].map(|i| {
            let d = (positions[1][i] - positions[0][i]) * 360.0 / f64::from(counts[i]);
            if i == 0 {
                (d + 180.0).rem_euclid(360.0) - 180.0
            } else {
                d
            }
        })
    }
    async fn until(&self, deadline: f64) {
        while self.clock.now() < deadline {
            tokio::time::sleep(STEP).await;
        }
        assert_eq!(self.tracker.state().state, "tracking");
    }
    fn solved_error(&self, id: u64) -> [f64; 2] {
        let optics = self.sim.optics();
        let stars = self.sim.field_stars(200.0, 1.0);
        assert!(stars.len() >= 4, "render must contain a solvable field");
        let pixels = simulator::render::render(
            &optics,
            &stars.iter().map(|(_, s)| *s).collect::<Vec<_>>(),
            200.0,
            1.0,
            13,
            id,
        );
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
        assert_eq!(report.status, Status::MatchFound, "frame {id}");
        // Use the calibrated off-centre aim point, never the boresight.
        let aim = solved.unwrap().unproject(optics.eyepiece_px).unwrap();
        error(aim, self.desired_ray())
    }
}

#[derive(Clone, Copy)]
enum Feed {
    None,
    Truth,
    Rendered,
}
async fn run(seconds: u32, feed: Feed) -> Vec<[f64; 2]> {
    let rig = Rig::new(0.01).await;
    let start = rig.clock.now();
    let mut noise = Noise(73);
    let mut errors = Vec::new();
    for sample in 1..=seconds / 2 {
        rig.until(start + f64::from(sample * 2)).await;
        let truth = rig.truth_error();
        errors.push(truth);
        let measured = match feed {
            Feed::None => continue,
            Feed::Truth => {
                let n = noise.pair();
                [truth[0] + n[0], truth[1] + n[1]]
            }
            Feed::Rendered => rig.solved_error(u64::from(sample)),
        };
        rig.tracker.pointing(measured[0], measured[1]);
        assert!(rig
            .tracker
            .state()
            .tracking_offset
            .iter()
            .all(|v| v.abs() <= OFFSET_LIMIT));
    }
    assert!(rig.mount.state(Axis::Azimuth).await.moving);
    assert!(rig.mount.state(Axis::Altitude).await.moving);
    assert!(rig.rates.lock().unwrap().len() >= seconds as usize * 2);
    rig.tracker.stop("stopped").await;
    assert_eq!(rig.tracker.state().tracking_offset, [0.0; 2]);
    errors
}

#[tokio::test(start_paused = true)]
async fn one_percent_fast_mount_drifts_open_loop_but_holds_with_finder_corrections() {
    let forward = run(600, Feed::None).await;
    let last = forward.last().unwrap();
    let drift = last[0].hypot(last[1]);
    assert!(drift > 60.0, "feed-forward drift {drift}″");
    let closed = run(600, Feed::Truth).await;
    let settled = rms(&closed[60..]);
    println!("600 s: open drift {drift:.3}″; closed last-eight-minute RMS {settled:.3}″");
    assert!(settled <= 20.0, "last eight minutes RMS {settled}″");
}

#[tokio::test(start_paused = true)]
async fn sub_ten_arcsecond_errors_leave_both_feed_forward_rates_and_zero_offset_unchanged() {
    let rig = Rig::new(0.0).await;
    let start = rig.clock.now();
    for sample in 1..=60 {
        rig.until(start + f64::from(sample * 2)).await;
        let e = rig.truth_error();
        assert!(e[0].hypot(e[1]) < 10.0);
        rig.tracker.pointing(e[0], e[1]);
        assert_eq!(rig.tracker.state().tracking_offset, [0.0; 2]);
    }
    let rates = rig.rates.lock().unwrap().clone();
    for axis in 0..2 {
        assert!(rates.iter().filter(|r| r.axis == axis).count() >= 120);
    }
    for r in rates {
        let difference = (r.rate - rig.forward(r.time)[r.axis]).abs() * 3600.0;
        assert!(
            difference <= 0.001,
            "axis {} rate differs by {difference}″/s",
            r.axis
        );
    }
    rig.tracker.stop("stopped").await;
}

#[tokio::test(start_paused = true)]
async fn two_minutes_of_rendered_solved_frames_agree_with_noisy_truth_within_five_arcseconds() {
    let rendered = run(120, Feed::Rendered).await;
    let truth = run(120, Feed::Truth).await;
    assert_eq!(rendered.len(), 60);
    assert_eq!(truth.len(), rendered.len());
    let differences: Vec<_> = rendered
        .iter()
        .zip(&truth)
        .map(|(a, b)| [a[0] - b[0], a[1] - b[1]])
        .collect();
    let difference = rms(&differences);
    println!("120 s: rendered/truth difference RMS {difference:.3}″");
    assert!(
        difference <= 5.0,
        "rendered versus truth error RMS {difference}″"
    );
    // Also constrain each run against the independent physical truth.
    assert!(rms(&rendered) <= 20.0);
    assert!(rms(&truth) <= 20.0);
}

#[tokio::test(start_paused = true)]
async fn parallactic_rotation_and_cos_e_scale_match_known_horizontal_errors() {
    let rig = Rig::new(0.0).await;
    rig.tracker.pointing(0.0, 0.0);
    let start = rig.clock.now();
    rig.until(start + 2.0).await;
    let (h, az) = desired(targets::find("M42").unwrap(), rig.clock.now());
    // Independent horizon displacements, transformed by the simulator into
    // equatorial errors. Expected rates follow gain 0.6 / 2 s and Ti = 60 s.
    let measured = error(
        ray(h + 20.0 / 3600.0, az - 30.0 / 3600.0, rig.clock.now()),
        rig.desired_ray(),
    );
    rig.tracker.pointing(measured[0], measured[1]);
    let expected_i = [0.6, -0.4];
    for (actual, expected) in rig
        .tracker
        .state()
        .tracking_offset
        .into_iter()
        .zip(expected_i)
    {
        assert!(
            (actual - expected).abs() < 0.002,
            "{actual} versus {expected}"
        );
    }
    rig.until(start + 3.2).await;
    let rates = rig.rates.lock().unwrap().clone();
    for (axis, expected_p) in [9.0, -6.0].into_iter().enumerate() {
        let rate = rates.iter().rev().find(|r| r.axis == axis).unwrap();
        let actual = (rate.rate - rig.forward(rate.time)[axis]) * 3600.0;
        assert!(
            (actual - expected_p - expected_i[axis]).abs() < 0.02,
            "axis {axis}: {actual}"
        );
    }
    rig.tracker.stop("stopped").await;
}

#[tokio::test(start_paused = true)]
async fn integral_is_bounded_held_in_deadband_and_cleared_on_stop_and_restart() {
    let rig = Rig::new(0.0).await;
    rig.tracker.pointing(0.0, 0.0);
    let start = rig.clock.now();
    rig.until(start + 2.0).await;
    // A known finite disturbance exercises both axes; sky errors, not axis
    // commands, enter through the same path as solved frames.
    rig.tracker.pointing(1000.0, -1000.0);
    let offset = rig.tracker.state().tracking_offset;
    assert!(offset.iter().all(|v| (v.abs() - OFFSET_LIMIT).abs() < 1e-9));
    rig.until(start + 4.0).await;
    rig.tracker.pointing(3.0, -4.0);
    assert_eq!(rig.tracker.state().tracking_offset, offset);
    rig.until(start + 6.0).await;
    // The deadband suppresses P but keeps I in the actual commanded rates.
    let rates = rig.rates.lock().unwrap().clone();
    for (axis, expected) in offset.into_iter().enumerate() {
        let r = rates.iter().rev().find(|r| r.axis == axis).unwrap();
        let actual = (r.rate - rig.forward(r.time)[axis]) * 3600.0;
        assert!((actual - expected).abs() <= 0.001);
    }
    rig.tracker.pointing(f64::NAN, 0.0);
    rig.tracker.pointing(0.0, f64::INFINITY);
    assert_eq!(rig.tracker.state().tracking_offset, offset);
    rig.tracker.stop("stopped").await;
    assert_eq!(rig.tracker.state().tracking_offset, [0.0; 2]);
    rig.tracker.pointing(1000.0, -1000.0);
    assert_eq!(rig.tracker.state().tracking_offset, [0.0; 2]);
    rig.tracker
        .start(targets::find("M42").unwrap().clone())
        .await
        .unwrap();
    assert_eq!(rig.tracker.state().tracking_offset, [0.0; 2]);
    rig.tracker.stop("stopped").await;
}

#[tokio::test(start_paused = true)]
async fn stale_proportional_expires_and_link_loss_clears_a_nonzero_integral() {
    let rig = Rig::new(0.0).await;
    rig.tracker.pointing(0.0, 0.0);
    let start = rig.clock.now();
    rig.until(start + 2.0).await;
    rig.tracker.pointing(30.0, -20.0);
    let offset = rig.tracker.state().tracking_offset;
    assert!(offset.iter().all(|v| v.abs() > 0.01));
    // Receiving the same clock instant again must not integrate it twice.
    rig.tracker.pointing(30.0, -20.0);
    assert_eq!(rig.tracker.state().tracking_offset, offset);
    rig.until(start + 8.2).await;
    let rates = rig.rates.lock().unwrap().clone();
    for (axis, expected) in offset.into_iter().enumerate() {
        let r = rates.iter().rev().find(|r| r.axis == axis).unwrap();
        let extra = (r.rate - rig.forward(r.time)[axis]) * 3600.0;
        assert!((extra - expected).abs() <= 0.001);
    }
    assert!(rig.mount.state(Axis::Azimuth).await.moving);
    assert!(rig.mount.state(Axis::Altitude).await.moving);
    rig.controls.set_unresponsive(true);
    for _ in 0..40 {
        tokio::time::sleep(STEP).await;
    }
    let state = rig.tracker.state();
    assert_eq!(state.state, "off");
    assert_eq!(state.ended_reason.as_deref(), Some("link_lost"));
    assert_eq!(state.tracking_offset, [0.0; 2]);
    assert!(!rig.mount.state(Axis::Azimuth).await.moving);
    assert!(!rig.mount.state(Axis::Altitude).await.moving);
    rig.tracker.pointing(30.0, -20.0);
    assert_eq!(rig.tracker.state().tracking_offset, [0.0; 2]);
}
