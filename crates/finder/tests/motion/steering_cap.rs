//! US12: 015's exposure functions and independent simulator horizon differences
//! are the oracles for the tracker's limits and tuned shares, in paused time.
use crate::support::live::{ConsoleClock, DB, START};

use finder::{
    ephemeris::{self, Exposure},
    imaging::steering::{Steered, TrackingError},
    link::{LinkManager, Transport},
    model::Model,
    session::{Session, Site},
    sky::{self, Observer},
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

const AXES: [Axis; 2] = [Axis::Azimuth, Axis::Altitude];
const STEP: Duration = Duration::from_millis(50);
type Receipts = Arc<Mutex<Vec<(f64, Axis, f64)>>>;

struct NodeTime(VirtualClock);
impl NodeClock for NodeTime {
    fn now(&self) -> f64 {
        self.0.node_now()
    }
}

// Observe actual rate receipts so the rig waits for both axes before settling.
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

struct Rig {
    clock: VirtualClock,
    tracker: Arc<Tracker>,
    target: Target,
}
impl Rig {
    async fn new(target: Target) -> Self {
        let clock = VirtualClock::new(START);
        let mut model = Model::default();
        model.load_known_terms(0.0, 130.0, [0.0; 2]);
        let (alt, az) =
            simulator::sky::horizontal(target.vector(), simulator::Site::default(), START);
        // Saemundsson (Meeus 16.4): the tracker follows the apparent ray while
        // simulator optics are geometric, as in the other tracker rigs.
        let apparent = alt + 1.02 / (alt + 10.3 / (alt + 5.11)).to_radians().tan() / 60.0;
        let zeros = model.inverse(az, apparent, [360; 2]).unwrap();
        let mount = SimMount::with_config(
            clock.clone(),
            MountConfig {
                base_tilt_deg: 0.0,
                base_tilt_toward_deg: 130.0,
                zero_points_deg: zeros,
                backlash_deg: 0.0,
            },
        );
        mount.set_rate_error(0.0);
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
            SimCamera::new(sim),
            Arc::new(NodeTime(clock.clone())),
            "test-token",
            Default::default(),
            controls,
            Some(Arc::new(Recorded {
                mount,
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
            model,
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
        assert_eq!(tracker.state().state, "tracking");
        Self {
            clock,
            tracker,
            target,
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
    fn steer(&self, east_arcsec: f64, north_arcsec: f64) {
        self.tracker.steer(TrackingError {
            time: self.now(),
            east_arcsec,
            north_arcsec,
        });
    }
}

fn place(target: &Target, time: f64) -> (f64, f64) {
    sky::alt_az(
        target.ra_deg,
        target.dec_deg,
        &Observer {
            lat: 40.0,
            lon: -75.0,
            time,
        },
    )
}
fn rotation(target: &Target, time: f64) -> f64 {
    let (alt, az) = place(target, time);
    ephemeris::altaz_rates_from_alt_az(40.0, alt, az)
        .field_rotation_deg_per_h
        .to_radians()
        / 3600.0
}
fn exposure(rotation: f64) -> Exposure {
    // 1236 x 822 pixels at 19.12 micrometres, with 2 pixels of corner smear.
    ephemeris::longest_exposure(23.63232, 15.71664, 19.12, rotation, 2.0)
}
fn relative_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= expected.abs() * 1e-6,
        "{actual} vs {expected} seconds, tolerance one part in a million"
    );
}

// S1: 015's camera formula at the same console instant, for both signs of
// rotation; the M42 worked example is 37.28 s, hence the independent 30..45 bound.
#[tokio::test(start_paused = true)]
async fn longest_exposure_agrees_with_015_for_both_rotation_signs() {
    let turn = 2.0 / 618f64.hypot(411.0);
    for name in ["M42", "M81"] {
        let rig = Rig::new(targets::find(name).unwrap().clone()).await;
        let now = rig.now();
        let actual = rig.tracker.longest_exposure_s(turn).unwrap();
        let rate = rotation(&rig.target, now);
        let Exposure::Seconds(expected) = exposure(rate) else {
            panic!("{name} has a finite limit");
        };
        relative_close(actual, expected);
        assert!(actual > 30.0 && actual < 45.0, "{name}: {actual}");
        if name == "M42" {
            assert!(rate > 0.0);
        } else {
            assert!(rate < 0.0, "M81 rotation {rate}");
        }
        let steered: &dyn Steered = rig.tracker.as_ref();
        assert_eq!(steered.longest_exposure_s(turn), Some(actual));
        for invalid in [0.0, -1.0, f64::NAN] {
            assert_eq!(rig.tracker.longest_exposure_s(invalid), None);
            assert_eq!(steered.longest_exposure_s(invalid), None);
        }
        rig.tracker.stop("stopped").await;
        assert_eq!(rig.tracker.longest_exposure_s(turn), None);
        assert_eq!(steered.longest_exposure_s(turn), None);
    }
}

// S2: choose the instant before finding RA, so the <1e-9 rad/s rule is
// exercised at due east itself rather than at a nearby rounded catalogue place.
#[tokio::test(start_paused = true)]
async fn due_east_has_no_limit_at_the_chosen_instant() {
    const DUE_EAST: f64 = 1_768_528_830.0;
    assert_eq!(DUE_EAST - START, 30.0);
    let mut target = targets::find("M42").unwrap().clone();
    target.dec_deg = 18.75;
    let at_ra = |ra| {
        let mut candidate = target.clone();
        candidate.ra_deg = ra;
        place(&candidate, DUE_EAST)
    };
    let (mut low, mut high) = (0..359)
        .map(|ra| (f64::from(ra), f64::from(ra + 1)))
        .find(|(a, b)| {
            let (alt_a, az_a) = at_ra(*a);
            let (alt_b, az_b) = at_ra(*b);
            alt_a > 0.0 && alt_b > 0.0 && (az_a - 90.0) * (az_b - 90.0) < 0.0
        })
        .expect("neighbouring RAs bracket due east above the horizon");
    for _ in 0..60 {
        let middle = (low + high) / 2.0;
        if (at_ra(low).1 - 90.0) * (at_ra(middle).1 - 90.0) > 0.0 {
            low = middle;
        } else {
            high = middle;
        }
    }
    target.ra_deg = (low + high) / 2.0;
    let (alt, az) = place(&target, DUE_EAST);
    assert!((az - 90.0).abs() <= 1e-9, "azimuth {az}");
    assert!((alt - 30.0).abs() < 1.0, "altitude {alt}");
    let rig = Rig::new(target).await;
    tokio::time::sleep(Duration::from_secs_f64(DUE_EAST - rig.now())).await;
    let now = rig.now();
    let actual = rig.tracker.longest_exposure_s(0.0027);
    assert!((now - DUE_EAST).abs() <= 0.002, "asked at {now}");
    assert_eq!(exposure(rotation(&rig.target, now)), Exposure::NoLimit);
    assert_eq!(actual, None);

    rig.wait(10.0).await;
    let now = rig.now();
    let actual = rig.tracker.longest_exposure_s(0.0027).unwrap();
    let rate = rotation(&rig.target, now);
    assert!(rate.abs() >= 1e-9, "rotation {rate} rad/s");
    let Exposure::Seconds(expected) = ephemeris::longest_exposure(
        23.63232,
        15.71664,
        19.12,
        rate,
        0.0027 * 618f64.hypot(411.0),
    ) else {
        panic!("a finite limit ten seconds after due east");
    };
    relative_close(actual, expected);
    assert!(actual > 3600.0, "{actual}");
    rig.tracker.stop("stopped").await;
}

// S3: sample all 61 whole minutes ourselves, both ends included. M42 transits
// in this hour: the worked minimum is 34.33 s, less than its limit now.
#[tokio::test(start_paused = true)]
async fn hour_ahead_limit_agrees_with_61_independent_samples() {
    let rig = Rig::new(targets::find("M42").unwrap().clone()).await;
    let turn = 2.0 / 618f64.hypot(411.0);
    let now = rig.now();
    let actual = rig.tracker.longest_exposure_ahead_s(turn).unwrap();
    let expected = (0..=60)
        .map(|minute| turn / rotation(&rig.target, now + f64::from(minute) * 60.0).abs())
        .fold(f64::INFINITY, f64::min);
    relative_close(actual, expected);
    assert!(actual < rig.tracker.longest_exposure_s(turn).unwrap());
    assert!(actual > 33.0 && actual < 36.0, "{actual}");
    let steered: &dyn Steered = rig.tracker.as_ref();
    assert_eq!(steered.longest_exposure_ahead_s(turn), Some(actual));
    for invalid in [0.0, -1.0, f64::NAN] {
        assert_eq!(rig.tracker.longest_exposure_ahead_s(invalid), None);
        assert_eq!(steered.longest_exposure_ahead_s(invalid), None);
    }
    rig.tracker.stop("stopped").await;
    assert_eq!(rig.tracker.longest_exposure_ahead_s(turn), None);
    assert_eq!(steered.longest_exposure_ahead_s(turn), None);
}

// S4: differentiate the simulator's geometric horizon transform as 015's PI
// test does. The controller uses apparent altitude; 0.002 arcsec/s covers that
// small secant difference. Steps are checked in sky coordinates to 0.2 arcsec.
#[tokio::test(start_paused = true)]
async fn tuning_changes_the_rate_and_step_shares_and_reaches_the_trait() {
    let rig = Rig::new(targets::find("M42").unwrap().clone()).await;
    rig.tracker.gap().await;
    rig.tracker.tune(1.0, 0.6);
    rig.steer(0.0, 0.0);
    rig.tracker.gap().await;
    rig.wait(6.0).await;
    let time = rig.now();
    let reference =
        simulator::sky::horizontal(rig.target.vector(), simulator::Site::default(), time);
    let displaced = UnitVector::from_radec(
        (rig.target.ra_deg + 0.5 / 3600.0 / rig.target.dec_deg.to_radians().cos()).to_radians(),
        rig.target.dec_deg.to_radians(),
    );
    let measured = simulator::sky::horizontal(displaced, simulator::Site::default(), time);
    let axis_error = [
        ((measured.1 - reference.1 + 180.0).rem_euclid(360.0) - 180.0) * 3600.0,
        (measured.0 - reference.0) * 3600.0,
    ];
    rig.steer(0.5, 0.0);
    let step = rig.tracker.gap().await;
    assert_eq!([step.east_arcsec, step.north_arcsec], [0.0; 2]);
    let offset = rig.tracker.state().tracking_offset;
    for i in 0..2 {
        let expected = -0.6 * axis_error[i] / 6.0;
        assert!(
            (offset[i] - expected).abs() <= 0.002,
            "axis {i}: {} vs {expected} arcsec/s",
            offset[i]
        );
    }
    assert!(offset[0] > 0.05, "azimuth offset {}", offset[0]);
    rig.wait(6.0).await;
    rig.steer(12.0, -16.0);
    let step = rig.tracker.gap().await;
    assert!((step.east_arcsec + 12.0).abs() <= 0.2, "{step:?}");
    assert!((step.north_arcsec - 16.0).abs() <= 0.2, "{step:?}");
    rig.tracker.stop("stopped").await;

    let rig = Rig::new(targets::find("M42").unwrap().clone()).await;
    rig.tracker.gap().await;
    rig.tracker.tune(0.5, 0.0);
    rig.steer(0.0, 0.0);
    rig.tracker.gap().await;
    rig.wait(6.0).await;
    rig.steer(0.5, 0.0);
    rig.tracker.gap().await;
    assert_eq!(rig.tracker.state().tracking_offset, [0.0; 2]);
    rig.tracker.stop("stopped").await;

    let rig = Rig::new(targets::find("M42").unwrap().clone()).await;
    rig.tracker.gap().await;
    let steered: &dyn Steered = rig.tracker.as_ref();
    steered.tune(2.0, f64::NAN);
    steered.tune(f64::NAN, f64::NAN);
    rig.steer(12.0, -16.0);
    let step = rig.tracker.gap().await;
    assert!((step.east_arcsec + 12.0).abs() <= 0.2, "{step:?}");
    assert!((step.north_arcsec - 16.0).abs() <= 0.2, "{step:?}");
    rig.tracker.stop("stopped").await;
}
