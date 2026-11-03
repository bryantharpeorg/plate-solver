//! One continuous virtual night. Oracles: the simulator's geometric sky and
//! mechanical eyepiece ray, not the tracker's forecast or reported error.
//! Pointings use seeded Gaussian noise (sigma 3″); limits are 30″ at reversal,
//! 1 s at the geometric altitude crossing, 60″ after the keyhole, and 30 s wall time.
#[path = "live/support.rs"]
mod support;

use finder::{
    link::{LinkManager, Transport},
    model::{separation, Model},
    session::{Session, Site},
    targets::{self, Target},
    tracking::Tracker,
};
use math_core::UnitVector;
use scope_link::{memory, proto};
use scope_node::{
    clock::NodeClock,
    mount::{Axis, Mount},
    service::{NodeControls, NodeService},
};
use simulator::{
    mount::{MountConfig, SimMount},
    SimCamera, Simulator, VirtualClock,
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use support::{ConsoleClock, DB, START};

const STEP: Duration = Duration::from_millis(50);
const AXES: [Axis; 2] = [Axis::Azimuth, Axis::Altitude];

fn geometric(target: &Target, time: f64) -> (f64, f64) {
    simulator::sky::horizontal(target.vector(), simulator::Site::default(), time)
}

// Saemundsson (Meeus 16.4), standard atmosphere. Simulator optics are geometric;
// the physical aim point must follow the target's apparent place.
fn apparent(target: &Target, time: f64) -> (f64, f64) {
    let (alt, az) = geometric(target, time);
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

// Choose a J2000 RA for the specified declination and meridian crossing. Only
// simulator transforms are used; the controller is not involved in the oracle.
fn target_at_transit(dec: f64, time: f64) -> Target {
    let mut target = targets::find("M42").unwrap().clone();
    target.dec_deg = dec;
    target.ra_deg = equatorial(90.0 - (40.0 - dec), 180.0, time)
        .to_radec()
        .0
        .to_degrees();
    let (mut lo, mut hi) = (target.ra_deg - 1.0, target.ra_deg + 1.0);
    for _ in 0..40 {
        target.ra_deg = (lo + hi) / 2.0;
        if geometric(&target, time).1 > 180.0 {
            lo = target.ra_deg;
        } else {
            hi = target.ra_deg;
        }
    }
    target
}

fn crossing(target: &Target, mut lo: f64, mut hi: f64, altitude: f64, rising: bool) -> f64 {
    assert_eq!(geometric(target, lo).0 < altitude, rising);
    assert_ne!(geometric(target, hi).0 < altitude, rising);
    for _ in 0..40 {
        let mid = (lo + hi) / 2.0;
        if (geometric(target, mid).0 < altitude) == rising {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (lo + hi) / 2.0
}

fn keyhole_target(exit: f64) -> Target {
    let mut target = target_at_transit(39.0, exit - 540.0);
    let (mut lo, mut hi) = (target.ra_deg - 1.0, target.ra_deg + 1.0);
    for _ in 0..40 {
        target.ra_deg = (lo + hi) / 2.0;
        if geometric(&target, exit).0 < 88.0 {
            lo = target.ra_deg;
        } else {
            hi = target.ra_deg;
        }
    }
    target
}

// Local xorshift64 and Box-Muller, independent of the simulator's private RNG.
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

struct NodeTime(VirtualClock);
impl NodeClock for NodeTime {
    fn now(&self) -> f64 {
        self.0.node_now()
    }
}

struct Rig {
    clock: VirtualClock,
    mount: SimMount,
    session: Arc<Session>,
    link: Arc<LinkManager>,
    model: Model,
    tracker: Tracker,
    noise: Noise,
    next_pointing: f64,
}
impl Rig {
    async fn new(target: &Target) -> Self {
        let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target")
            .join(format!("tracking-night-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join("session.json");
        std::fs::write(
            &path,
            r#"{"site":{"lat_deg":40,"lon_deg":-75},"target":null,"min_alt_deg":15,"backlash":{"alt_deg":0.3}}"#,
        )
        .unwrap();
        let session = Arc::new(Session::with_state_file(&path).unwrap());
        assert_eq!(session.snapshot(START).backlash.unwrap().alt_deg, Some(0.3));
        let clock = VirtualClock::new(START - 1.5);
        let (alt, az) = apparent(target, START);
        let mount = SimMount::with_config(
            clock.clone(),
            MountConfig {
                base_tilt_deg: 0.0,
                base_tilt_toward_deg: 0.0,
                zero_points_deg: [az - 0.35, alt - 0.35],
                backlash_deg: 0.3,
            },
        );
        // The GoTo's positive final approach loads the gears. At 0.5 deg/s
        // for 1 s including acceleration/braking, travel is 0.5 degrees.
        for axis in AXES {
            mount.rate(axis, 0.5).await;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
        mount.stop_all().await;
        tokio::time::sleep(Duration::from_millis(500)).await;
        mount.set_rate_error(0.01);
        assert_eq!(mount.rate_error(), 0.01);
        let truth = mount.truth();
        assert_eq!(truth.backlash_deg, 0.3);
        let mut model = session.model(clock.now());
        model.load_known_terms(
            truth.base_tilt_deg,
            truth.base_tilt_toward_deg,
            truth.zero_points_deg.map(|z| z - 0.15),
        );
        let sim = Simulator::new(DB.clone(), simulator::Site::default(), clock.clone());
        sim.connect_mount(mount.clone());
        let controls = NodeControls::default();
        controls.stop_frames(); // Long-run pointings use truth plus noise.
        let node = NodeService::with_controls_and_mount(
            SimCamera::new(sim),
            Arc::new(NodeTime(clock.clone())),
            "test-token",
            Default::default(),
            controls,
            Some(Arc::new(mount.clone())),
        )
        .unwrap();
        let transport =
            Transport::connector(move || {
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
                        .add_service(proto::scope_node_stop_server::ScopeNodeStopServer::new(node));
                        memory::serve_routes_on(server, routes).await.unwrap();
                    });
                    Ok::<_, std::io::Error>(client)
                }
            });
        session
            .edit(
                serde_json::from_value(serde_json::json!({
                    "node":{"address":"scope.local:1","token":"test-token"}
                }))
                .unwrap(),
                clock.now(),
            )
            .unwrap();
        let link = Arc::new(LinkManager::start(
            session.clone(),
            Arc::new(ConsoleClock(clock.clone())),
            transport,
        ));
        for _ in 0..100 {
            if link.status().borrow().connected {
                break;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        assert!(link.status().borrow().connected);
        let view = session.snapshot(clock.now());
        let tracker = Tracker::new(
            Arc::new(ConsoleClock(clock.clone())),
            link.clone(),
            model.clone(),
            view.site.unwrap(),
            view.min_alt_deg,
        );
        Self {
            clock,
            mount,
            session,
            link,
            model,
            tracker,
            noise: Noise(42),
            next_pointing: START,
        }
    }

    fn now(&self) -> f64 {
        self.clock.now()
    }

    fn error_arcsec(&self, target: &Target) -> f64 {
        let d = self.mount.true_direction();
        let (alt, az) = apparent(target, self.now());
        separation(d.az, d.alt, az, alt) * 3600.0
    }

    async fn tick(&mut self, target: &Target) {
        if self.now() >= self.next_pointing {
            let d = self.mount.true_direction();
            let (ra, dec) = equatorial(d.alt, d.az, self.now()).to_radec();
            let (alt, az) = apparent(target, self.now());
            let (target_ra, target_dec) = equatorial(alt, az, self.now()).to_radec();
            let noise = self.noise.pair();
            self.tracker.pointing(
                (((ra - target_ra).to_degrees() + 180.0).rem_euclid(360.0) - 180.0)
                    * target_dec.cos()
                    * 3600.0
                    + noise[0],
                (dec - target_dec).to_degrees() * 3600.0 + noise[1],
            );
            self.next_pointing = self.now() + 2.0;
        }
        tokio::time::sleep(STEP).await;
    }

    async fn stopped(&self) -> bool {
        !self.mount.state(Axis::Azimuth).await.moving
            && !self.mount.state(Axis::Altitude).await.moving
    }

    async fn acquire(&mut self, target: &Target, start: f64) {
        let (alt, az) = apparent(target, start);
        let counts = self.mount.axes().map(|a| a.counts_per_revolution);
        let dest = self.model.inverse(az, alt, counts).unwrap();
        // Same mount and model: a stopped two-waypoint positive approach.
        for offset in [-0.5, 0.0] {
            for (i, axis) in AXES.into_iter().enumerate() {
                self.mount
                    .goto(
                        axis,
                        (dest[i] + offset * f64::from(counts[i]) / 360.0).round() as i32,
                    )
                    .await;
            }
            while !self.stopped().await {
                assert!(
                    self.now() < start,
                    "acquisition must finish before tracking"
                );
                tokio::time::sleep(STEP).await;
            }
        }
        while self.now() < start {
            tokio::time::sleep(STEP).await;
        }
        self.tracker = Tracker::new(
            Arc::new(ConsoleClock(self.clock.clone())),
            self.link.clone(),
            self.model.clone(),
            Site {
                lat_deg: 40.0,
                lon_deg: -75.0,
            },
            15.0,
        );
        self.next_pointing = self.now();
        self.tracker.start(target.clone()).await.unwrap();
    }
}

#[tokio::test(start_paused = true)]
async fn an_hour_covers_transit_too_low_and_keyhole_in_under_thirty_wall_seconds() {
    let wall = Instant::now();
    let transit = START + 600.0;
    let first = target_at_transit(20.0, transit);
    let too_low = crossing(&first, transit, transit + 1800.0, 69.5, false);
    assert!((18.0 * 60.0..23.0 * 60.0).contains(&(too_low - transit)));
    let mut rig = Rig::new(&first).await;
    rig.tracker.start(first.clone()).await.unwrap();
    rig.tick(&first).await;
    assert_eq!(rig.tracker.state().state, "tracking");
    assert!(
        !rig.stopped().await,
        "accepted Track must move a connected mount"
    );
    let updated = rig
        .session
        .edit(
            serde_json::from_value(serde_json::json!({"min_alt_deg":69.5})).unwrap(),
            rig.now(),
        )
        .unwrap();
    assert_eq!(updated.min_alt_deg, 69.5);
    while rig.now() < too_low && rig.tracker.state().state == "tracking" {
        assert!(
            rig.error_arcsec(&first) <= 30.0,
            "reversal error {} arcsec at {} s from transit",
            rig.error_arcsec(&first),
            rig.now() - transit
        );
        rig.tick(&first).await;
    }
    while rig.tracker.state().state != "off" && rig.now() <= too_low + 1.0 {
        rig.tick(&first).await;
    }
    let first_end = rig.tracker.state();
    let ending_delay = rig.now() - too_low;
    while !rig.stopped().await && rig.now() < too_low + 1.0 {
        tokio::time::sleep(STEP).await;
    }
    let stopped_in_time = rig.stopped().await && rig.now() - too_low <= 1.0;
    // Join the first run before acquiring the next target, even if an ending
    // assertion fails. Assert the captured result after exercising both tracks.
    rig.tracker.stop("stopped").await;

    let end = START + 3600.0;
    let second = keyhole_target(end - 30.0);
    let entry = crossing(&second, end - 1500.0, end - 600.0, 88.0, true);
    let exit = crossing(&second, end - 300.0, end, 88.0, false);
    assert!((900.0..1200.0).contains(&(exit - entry)));
    assert!(rig.now() < entry - 300.0);
    rig.acquire(&second, entry - 300.0).await;
    let mut waited = false;
    let mut recovered = None;
    while rig.now() < exit + 30.0 {
        let state = rig.tracker.state();
        waited |= state.state == "keyhole_wait";
        if rig.now() < entry - 1.0 {
            assert_eq!(state.state, "tracking");
            assert!(rig.error_arcsec(&second) <= 30.0);
        }
        if rig.now() >= exit && state.state == "tracking" && rig.error_arcsec(&second) <= 60.0 {
            recovered.get_or_insert(rig.now());
        }
        assert!(state.ended_reason.is_none(), "{state:?}");
        rig.tick(&second).await;
    }
    assert!(
        waited,
        "the target must actually pass through the keyhole wait"
    );
    assert!(recovered.is_some_and(|time| time - exit <= 30.0));
    assert_eq!(rig.tracker.state().state, "tracking");
    assert!(rig.error_arcsec(&second) <= 60.0);
    assert!(
        rig.now() - START >= 3600.0,
        "one continuous hour of virtual time"
    );
    rig.tracker.stop("stopped").await;
    assert!(
        wall.elapsed() < Duration::from_secs(30),
        "wall time {:?}",
        wall.elapsed()
    );
    assert_eq!(first_end.state, "off");
    assert_eq!(first_end.ended_reason.as_deref(), Some("too_low"));
    assert!((0.0..=1.0).contains(&ending_delay));
    assert!(stopped_in_time);
}
