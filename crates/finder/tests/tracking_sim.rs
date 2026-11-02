//! US3 oracles: simulator mechanics and horizon transform, explicit virtual UTC,
//! and node-side command receipts. No controller output supplies an expectation.
#[path = "live/support.rs"]
mod support;

use finder::{
    ephemeris,
    link::{LinkManager, Transport},
    model::{separation, Model},
    session::{Session, Site},
    targets::{self, Target},
    tracking::Tracker,
};
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
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use support::{ConsoleClock, DB, START};

const STEP: Duration = Duration::from_millis(50);

struct NodeTime(VirtualClock);
impl NodeClock for NodeTime {
    fn now(&self) -> f64 {
        self.0.node_now()
    }
}

#[derive(Clone, Default)]
struct Receipts {
    rates: Arc<Mutex<Vec<(f64, Axis, f64)>>>,
    stops: Arc<Mutex<Vec<f64>>>,
}
#[derive(Default)]
struct RateDelay {
    enabled: AtomicBool,
    entered: AtomicBool,
}
#[derive(Clone)]
struct Recorded {
    mount: SimMount,
    clock: VirtualClock,
    receipts: Receipts,
    delay: Arc<RateDelay>,
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
            if self.delay.enabled.load(Ordering::SeqCst) {
                self.delay.entered.store(true, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
            self.receipts
                .rates
                .lock()
                .unwrap()
                .push((self.clock.now(), axis, rate));
            self.mount.rate(axis, rate).await;
        })
    }
    fn stop_all(&self) -> MountFuture<'_, ()> {
        Box::pin(async move {
            self.receipts.stops.lock().unwrap().push(self.clock.now());
            self.mount.stop_all().await;
        })
    }
}

// A network fault on the console's Stop RPC must not mask a missing node
// watchdog. Other scenarios delegate to the real token-free Stop service.
#[derive(Clone)]
struct StopLink {
    node: NodeService,
    silent: Arc<AtomicBool>,
}
#[tonic::async_trait]
impl proto::scope_node_stop_server::ScopeNodeStop for StopLink {
    async fn stop_all(
        &self,
        request: tonic::Request<proto::StopAllRequest>,
    ) -> Result<tonic::Response<proto::StopAllReply>, tonic::Status> {
        if self.silent.load(Ordering::SeqCst) {
            return std::future::pending().await;
        }
        proto::scope_node_stop_server::ScopeNodeStop::stop_all(&self.node, request).await
    }
}

fn geometric(target: &Target, time: f64) -> (f64, f64) {
    simulator::sky::horizontal(target.vector(), simulator::Site::default(), time)
}
fn apparent(target: &Target, time: f64) -> (f64, f64) {
    let (h, az) = geometric(target, time);
    // Saemundsson, Meeus 16.4, standard atmosphere. The simulator itself has
    // geometric optics, so compare its physical eyepiece to this apparent ray.
    (
        h + 1.02 / (h + 10.3 / (h + 5.11)).to_radians().tan() / 60.0,
        az,
    )
}
fn sun_alt(time: f64) -> f64 {
    let jd = simulator::sky::julian_date(time);
    let (ra, dec) = ephemeris::sun(jd);
    let date = math_core::UnitVector::from_radec(ra.to_radians(), dec.to_radians());
    simulator::sky::horizontal(
        simulator::sky::to_j2000(date, jd),
        simulator::Site::default(),
        time,
    )
    .0
}
fn crossing(mut lo: f64, mut hi: f64, rising: bool, altitude: impl Fn(f64) -> f64, h: f64) -> f64 {
    assert_eq!(altitude(lo) < h, rising);
    assert_ne!(altitude(hi) < h, rising);
    for _ in 0..45 {
        let mid = (lo + hi) / 2.0;
        if (altitude(mid) < h) == rising {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (lo + hi) / 2.0
}

struct Rig {
    clock: VirtualClock,
    mount: SimMount,
    link: Arc<LinkManager>,
    controls: NodeControls,
    silent_stop: Arc<AtomicBool>,
    delay: Arc<RateDelay>,
    receipts: Receipts,
    tracker: Tracker,
}
impl Rig {
    async fn new(start: f64, target: &Target) -> Self {
        let clock = VirtualClock::new(start);
        // Arrange an already centred mount, without consuming slew time near
        // dawn or setting. Only this setup uses the inverse; motion is measured
        // independently by the simulator's exact base rotation and mechanics.
        let mut model = Model::default();
        model.load_known_terms(1.0, 130.0, [0.0; 2]);
        let (h, az) = apparent(target, start);
        let zeros = model.inverse(az, h, [360; 2]).unwrap();
        let mount = SimMount::with_config(
            clock.clone(),
            MountConfig {
                base_tilt_deg: 1.0,
                base_tilt_toward_deg: 130.0,
                zero_points_deg: zeros,
                backlash_deg: 0.0,
            },
        );
        assert_eq!(mount.rate_error(), 0.0);
        let truth = mount.truth();
        model.load_known_terms(
            truth.base_tilt_deg,
            truth.base_tilt_toward_deg,
            truth.zero_points_deg,
        );
        let sim = Simulator::new(DB.clone(), simulator::Site::default(), clock.clone());
        sim.connect_mount(mount.clone());
        let receipts = Receipts::default();
        let delay = Arc::new(RateDelay::default());
        let controls = NodeControls::default();
        controls.stop_frames();
        let node = NodeService::with_controls_and_mount(
            SimCamera::new(sim),
            Arc::new(NodeTime(clock.clone())),
            "test-token",
            Default::default(),
            controls.clone(),
            Some(Arc::new(Recorded {
                mount: mount.clone(),
                clock: clock.clone(),
                receipts: receipts.clone(),
                delay: delay.clone(),
            })),
        )
        .unwrap();
        let silent_stop = Arc::new(AtomicBool::new(false));
        let stop_link = StopLink {
            node: node.clone(),
            silent: silent_stop.clone(),
        };
        let transport = Transport::connector(move || {
            let node = node.clone();
            let stop_link = stop_link.clone();
            async move {
                let (client, server) = tokio::io::duplex(64 * 1024);
                tokio::spawn(async move {
                    let routes = tonic::service::Routes::new(node.clone().into_server())
                        .add_service(
                            proto::scope_node_mount_server::ScopeNodeMountServer::with_interceptor(
                                node,
                                scope_link::TokenInterceptor::new("test-token"),
                            ),
                        )
                        .add_service(proto::scope_node_stop_server::ScopeNodeStopServer::new(
                            stop_link,
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
                start,
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
            link.clone(),
            model,
            Site {
                lat_deg: 40.0,
                lon_deg: -75.0,
            },
            15.0,
        );
        Self {
            clock,
            mount,
            link,
            controls,
            silent_stop,
            delay,
            receipts,
            tracker,
        }
    }
    fn now(&self) -> f64 {
        self.clock.now()
    }
    async fn tick(&self) {
        tokio::time::sleep(STEP).await;
    }
    async fn stationary(&self) -> bool {
        !self.mount.state(Axis::Azimuth).await.moving
            && !self.mount.state(Axis::Altitude).await.moving
    }
    async fn begin(&self, target: &Target) {
        assert_eq!(self.tracker.state().state, "off");
        self.tracker.start(target.clone()).await.unwrap();
        for _ in 0..4 {
            self.tick().await;
        }
        assert_eq!(self.tracker.state().state, "tracking");
        // Positive controls precede every negative motion assertion.
        assert!(!self.stationary().await);
        for axis in [Axis::Azimuth, Axis::Altitude] {
            assert!(self
                .receipts
                .rates
                .lock()
                .unwrap()
                .iter()
                .any(|(_, a, r)| *a == axis && r.abs() > 0.0));
        }
    }
    async fn stays_ended(&self, reason: &str) {
        assert_eq!(self.tracker.state().state, "off");
        assert_eq!(self.tracker.state().ended_reason.as_deref(), Some(reason));
        let rates = self.receipts.rates.lock().unwrap().len();
        for _ in 0..60 {
            self.tick().await;
            assert!(self.stationary().await);
        }
        assert_eq!(self.receipts.rates.lock().unwrap().len(), rates);
    }
}

#[tokio::test(start_paused = true)]
async fn model_feed_forward_keeps_m42_within_ten_arcseconds_for_ten_minutes() {
    let target = targets::find("M42").unwrap();
    let rig = Rig::new(START, target).await;
    let started = rig.now();
    rig.begin(target).await;
    let mut worst = 0.0_f64;
    while rig.now() - started < 600.0 {
        let direction = rig.mount.true_direction();
        let (h, az) = apparent(target, rig.now());
        let error = separation(direction.az, direction.alt, az, h) * 3600.0;
        worst = worst.max(error);
        assert!(error <= 10.0, "{error} arcsec at {}", rig.now() - started);
        assert_eq!(rig.tracker.state().state, "tracking");
        rig.tick().await;
    }
    let rates = rig.receipts.rates.lock().unwrap().clone();
    for axis in [Axis::Azimuth, Axis::Altitude] {
        let times: Vec<_> = rates
            .iter()
            .filter(|(_, a, _)| *a == axis)
            .map(|(t, _, _)| *t)
            .collect();
        assert!(times.len() >= 600);
        assert!(times[0] - started <= 0.05);
        // One millisecond is tokio's timer resolution, not a motion tolerance.
        assert!(times.windows(2).all(|w| w[1] - w[0] <= 1.001));
        assert!(rig.now() - times.last().unwrap() <= 1.001);
    }
    assert!(worst <= 10.0);
    rig.tracker.stop("stopped").await;
}

#[tokio::test(start_paused = true)]
async fn geometric_fifteen_degree_crossing_stops_both_axes_within_one_second() {
    let target = targets::find("M42").unwrap();
    let setting = crossing(
        START,
        START + 6.0 * 3600.0,
        false,
        |t| geometric(target, t).0,
        15.0,
    );
    let rig = Rig::new(setting - 5.0, target).await;
    rig.begin(target).await;
    while rig.now() < setting {
        assert_eq!(rig.tracker.state().state, "tracking");
        rig.tick().await;
    }
    // Refraction still puts the apparent ray above 15°; ending must be geometric.
    assert!(apparent(target, setting).0 > 15.0);
    while !rig.stationary().await && rig.now() <= setting + 1.0 {
        rig.tick().await;
    }
    assert!(rig.stationary().await);
    let stop = rig.receipts.stops.lock().unwrap()[0];
    assert!((setting..=setting + 1.0).contains(&stop));
    rig.stays_ended("too_low").await;
}

#[tokio::test(start_paused = true)]
async fn observer_stop_wins_over_next_rate_and_explicit_restart_is_required() {
    let target = targets::find("M42").unwrap();
    let rig = Rig::new(START, target).await;
    rig.begin(target).await;
    // Stop immediately before a scheduled rate refresh.
    tokio::time::sleep(Duration::from_millis(799)).await;
    let stopped = rig.now();
    let before = rig.receipts.rates.lock().unwrap().len();
    rig.tracker.stop("stopped").await;
    while !rig.stationary().await && rig.now() - stopped <= 0.5 {
        rig.tick().await;
    }
    assert!(rig.stationary().await);
    assert!(rig.now() - stopped <= 0.5);
    assert!(rig.receipts.stops.lock().unwrap()[0] - stopped <= 0.5);
    rig.stays_ended("stopped").await;
    assert_eq!(rig.receipts.rates.lock().unwrap().len(), before);
    rig.begin(target).await;
    assert!(rig.tracker.state().ended_reason.is_none());
    rig.tracker.stop("stopped").await;
}

#[tokio::test(start_paused = true)]
async fn watchdog_stops_after_one_second_of_silence_and_link_return_never_resumes() {
    let target = targets::find("M42").unwrap();
    let rig = Rig::new(START, target).await;
    rig.begin(target).await;
    let status = rig.link.status().borrow().clone();
    let last = rig.now() - status.last_heartbeat_at.unwrap().elapsed().as_secs_f64();
    rig.controls.set_unresponsive(true);
    rig.silent_stop.store(true, Ordering::SeqCst);
    for _ in 0..30 {
        rig.tick().await;
    }
    // StopAll is also silent: this receipt is the independent node watchdog.
    let stop = rig.receipts.stops.lock().unwrap()[0];
    assert!(
        (stop - last - 1.0).abs() <= 0.1,
        "{stop} vs last beat {last}"
    );
    assert!(rig.stationary().await);
    rig.stays_ended("link_lost").await;
    rig.controls.set_unresponsive(false);
    rig.silent_stop.store(false, Ordering::SeqCst);
    for _ in 0..200 {
        rig.tick().await;
        if rig.link.status().borrow().connected {
            break;
        }
    }
    assert!(rig.link.status().borrow().connected);
    rig.stays_ended("link_lost").await;
}

#[tokio::test(start_paused = true)]
async fn sun_centre_crossing_minus_point_eight_three_stops_within_one_second() {
    let dawn = crossing(
        START + 8.0 * 3600.0,
        START + 12.0 * 3600.0,
        true,
        sun_alt,
        -0.83,
    );
    let target = targets::find("M81").unwrap();
    let rig = Rig::new(dawn - 5.0, target).await;
    rig.begin(target).await;
    while rig.now() < dawn {
        assert_eq!(rig.tracker.state().state, "tracking");
        rig.tick().await;
    }
    while !rig.stationary().await && rig.now() <= dawn + 1.0 {
        rig.tick().await;
    }
    assert!(rig.stationary().await);
    let stop = rig.receipts.stops.lock().unwrap()[0];
    assert!((dawn..=dawn + 1.0).contains(&stop));
    rig.stays_ended("sun_up").await;
}

#[tokio::test(start_paused = true)]
async fn north_crossing_uses_the_short_axis_difference() {
    let mut target = targets::find("M81").unwrap().clone();
    let date = simulator::sky::horizontal_to_date(30.0, 359.98, simulator::Site::default(), START);
    let (ra, dec) = simulator::sky::to_j2000(date, simulator::sky::julian_date(START)).to_radec();
    target.ra_deg = ra.to_degrees();
    target.dec_deg = dec.to_degrees();
    let rig = Rig::new(START, &target).await;
    rig.begin(&target).await;
    for _ in 0..1200 {
        let direction = rig.mount.true_direction();
        let (h, az) = apparent(&target, rig.now());
        assert!(separation(direction.az, direction.alt, az, h) * 3600.0 <= 10.0);
        rig.tick().await;
    }
    assert!(
        apparent(&target, rig.now()).1 < 1.0,
        "must have crossed north"
    );
    assert!(rig
        .receipts
        .rates
        .lock()
        .unwrap()
        .iter()
        .all(|(_, _, rate)| rate.abs() < 0.02));
    rig.tracker.stop("stopped").await;
}

#[tokio::test(start_paused = true)]
async fn pointing_samples_age_on_console_time_and_are_ignored_after_ending() {
    let target = targets::find("M42").unwrap();
    let rig = Rig::new(START, target).await;
    rig.tracker.pointing(30.0, -20.0);
    assert!(rig.tracker.state().samples.is_empty());
    rig.begin(target).await;
    assert_eq!(
        rig.tracker.start(target.clone()).await.unwrap_err(),
        "tracking is already running"
    );
    rig.tracker.pointing(30.0, -20.0);
    rig.tracker.pointing(f64::NAN, 2.0);
    let state = rig.tracker.state();
    assert_eq!(state.samples.len(), 1);
    assert_eq!(state.samples[0].time, rig.now());
    assert_eq!(state.samples[0].east_arcsec, 30.0);
    assert_eq!(state.samples[0].north_arcsec, -20.0);
    assert_eq!(state.tracking_offset, [0.0; 2]);
    assert_eq!(state.target.unwrap().id, target.id);
    // Feed-forward continues while samples expire, without changing the rates.
    for _ in 0..1202 {
        rig.tick().await;
    }
    assert!(rig.tracker.state().samples.is_empty());
    rig.tracker.pointing(5.0, 6.0);
    assert_eq!(rig.tracker.state().samples.len(), 1);
    rig.tracker.stop("stopped").await;
    rig.tracker.pointing(7.0, 8.0);
    assert_eq!(rig.tracker.state().samples.len(), 1);
    rig.stays_ended("stopped").await;
}

#[tokio::test(start_paused = true)]
async fn stop_cancels_an_in_flight_rate_before_it_can_restart_an_axis() {
    let target = targets::find("M42").unwrap();
    let rig = Rig::new(START, target).await;
    rig.begin(target).await;
    rig.delay.enabled.store(true, Ordering::SeqCst);
    for _ in 0..30 {
        rig.tick().await;
        if rig.delay.entered.load(Ordering::SeqCst) {
            break;
        }
    }
    assert!(rig.delay.entered.load(Ordering::SeqCst));
    let before = rig.receipts.rates.lock().unwrap().len();
    let stopped = rig.now();
    rig.tracker.stop("stopped").await;
    while !rig.stationary().await && rig.now() - stopped <= 0.5 {
        rig.tick().await;
    }
    assert!(rig.stationary().await);
    assert!(rig.now() - stopped <= 0.5);
    rig.stays_ended("stopped").await;
    assert_eq!(rig.receipts.rates.lock().unwrap().len(), before);
}

#[tokio::test(start_paused = true)]
async fn missing_pointing_model_refuses_rates_on_a_mount_proven_to_move() {
    let target = targets::find("M42").unwrap();
    let rig = Rig::new(START, target).await;
    rig.begin(target).await;
    rig.tracker.stop("stopped").await;
    let before = rig.receipts.rates.lock().unwrap().len();
    let tracker = Tracker::new(
        Arc::new(ConsoleClock(rig.clock.clone())),
        rig.link.clone(),
        Model::default(),
        Site {
            lat_deg: 40.0,
            lon_deg: -75.0,
        },
        15.0,
    );
    assert_eq!(
        tracker.start(target.clone()).await.unwrap_err(),
        "cannot invert the pointing model"
    );
    assert_eq!(tracker.state().state, "off");
    rig.stays_ended("stopped").await;
    assert_eq!(rig.receipts.rates.lock().unwrap().len(), before);
}

#[tokio::test(start_paused = true)]
async fn dropping_the_tracking_owner_stops_the_connected_mount() {
    let target = targets::find("M42").unwrap();
    let rig = Rig::new(START, target).await;
    rig.begin(target).await;
    let Rig {
        tracker,
        mount,
        receipts,
        ..
    } = rig;
    let before = receipts.rates.lock().unwrap().len();
    drop(tracker);
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(!mount.state(Axis::Azimuth).await.moving);
    assert!(!mount.state(Axis::Altitude).await.moving);
    assert!(!receipts.stops.lock().unwrap().is_empty());
    tokio::time::sleep(Duration::from_secs(3)).await;
    assert_eq!(receipts.rates.lock().unwrap().len(), before);
}
