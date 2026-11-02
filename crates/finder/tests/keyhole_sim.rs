//! Oracle: the simulator's celestial transform and mechanical eyepiece ray,
//! with configured 0.3° play. Crossing times are bisected independently of the
//! tracker. Bounds: entry within 1 s, exit aim within 30″, recovery within 60″.
#[path = "live/support.rs"]
mod support;

use finder::{
    link::{LinkManager, Transport},
    model::{separation, Sync},
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
    sync::{Arc, Mutex},
    time::Duration,
};
use support::{ConsoleClock, DB, START};

const STEP: Duration = Duration::from_millis(50);
const AXES: [Axis; 2] = [Axis::Azimuth, Axis::Altitude];

#[derive(Clone, Debug)]
enum Command {
    Rate(Axis, f64),
    Goto(Axis, i32, bool),
    Stop,
}
type Receipts = Arc<Mutex<Vec<(f64, Command)>>>;
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
    receipts: Receipts,
}
impl Mount for Recorded {
    fn axes(&self) -> [AxisInfo; 2] {
        self.mount.axes()
    }
    fn state(&self, axis: Axis) -> MountFuture<'_, AxisState> {
        self.mount.state(axis)
    }
    fn goto(&self, axis: Axis, position: i32) -> MountFuture<'_, ()> {
        Box::pin(async move {
            let stopped = !self.mount.state(Axis::Azimuth).await.moving
                && !self.mount.state(Axis::Altitude).await.moving;
            self.receipts
                .lock()
                .unwrap()
                .push((self.clock.now(), Command::Goto(axis, position, stopped)));
            self.mount.goto(axis, position).await;
        })
    }
    fn rate(&self, axis: Axis, rate: f64) -> MountFuture<'_, ()> {
        Box::pin(async move {
            self.receipts
                .lock()
                .unwrap()
                .push((self.clock.now(), Command::Rate(axis, rate)));
            self.mount.rate(axis, rate).await;
        })
    }
    fn stop_all(&self) -> MountFuture<'_, ()> {
        Box::pin(async move {
            self.receipts
                .lock()
                .unwrap()
                .push((self.clock.now(), Command::Stop));
            self.mount.stop_all().await;
        })
    }
}

fn geometric(target: &Target, time: f64) -> (f64, f64) {
    simulator::sky::horizontal(target.vector(), simulator::Site::default(), time)
}
fn apparent(target: &Target, time: f64) -> (f64, f64) {
    let (alt, az) = geometric(target, time);
    (
        alt + 1.02 / (alt + 10.3 / (alt + 5.11)).to_radians().tan() / 60.0,
        az,
    )
}
fn crossing(target: &Target, mut lo: f64, mut hi: f64, rising: bool) -> f64 {
    assert_eq!(geometric(target, lo).0 < 88.0, rising);
    assert_ne!(geometric(target, hi).0 < 88.0, rising);
    for _ in 0..40 {
        let mid = (lo + hi) / 2.0;
        if (geometric(target, mid).0 < 88.0) == rising {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (lo + hi) / 2.0
}
fn path() -> (Target, f64, f64) {
    let mut target = targets::find("M42").unwrap().clone();
    target.dec_deg = 39.0;
    let (entry, exit) = circle_crossings(&target, START);
    assert!(
        (900.0..1200.0).contains(&(exit - entry)),
        "roughly 18 minutes inside"
    );
    (target, entry, exit)
}
fn circle_crossings(target: &Target, start: f64) -> (f64, f64) {
    let entry = (1..360)
        .map(|n| start + f64::from(n) * 60.0)
        .find(|t| geometric(target, *t).0 >= 88.0)
        .unwrap();
    let exit = (1..60)
        .map(|n| entry + f64::from(n) * 60.0)
        .find(|t| geometric(target, *t).0 < 88.0)
        .unwrap();
    (
        crossing(target, entry - 60.0, entry, true),
        crossing(target, exit - 60.0, exit, false),
    )
}

struct Rig {
    clock: VirtualClock,
    mount: SimMount,
    target: Target,
    tracker: Tracker,
    receipts: Receipts,
}
impl Rig {
    async fn new(start: f64, target: Target, name: &str) -> Self {
        Self::with_wind(start, target, name, false).await
    }
    async fn with_wind(start: f64, target: Target, name: &str, wound: bool) -> Self {
        let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target")
            .join(format!("keyhole-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join("session.json");
        std::fs::write(
            &path,
            r#"{"site":null,"target":null,"min_alt_deg":15,"backlash":{"alt_deg":0.3}}"#,
        )
        .unwrap();
        let session = Arc::new(Session::with_state_file(&path).unwrap());
        assert_eq!(session.snapshot(start).backlash.unwrap().alt_deg, Some(0.3));
        // Positive final approach: the optical ray lags counts by half the play.
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
        for axis in AXES {
            mount.rate(axis, 0.5).await;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
        mount.stop_all().await;
        tokio::time::sleep(Duration::from_millis(500)).await;
        let sim = Simulator::new(DB.clone(), simulator::Site::default(), clock.clone());
        sim.connect_mount(mount.clone());
        let receipts = Receipts::default();
        let controls = NodeControls::default();
        controls.stop_frames();
        let node = NodeService::with_controls_and_mount(
            SimCamera::new(sim),
            Arc::new(NodeTime(clock.clone())),
            "test-token",
            Default::default(),
            controls,
            Some(Arc::new(Recorded {
                mount: mount.clone(),
                clock: clock.clone(),
                receipts: receipts.clone(),
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
                    "node":{"address":"scope.local:1","token":"test-token"}
                }))
                .unwrap(),
                start,
            )
            .unwrap();
        let console = Arc::new(ConsoleClock(clock.clone()));
        let link = Arc::new(LinkManager::start(
            session.clone(),
            console.clone(),
            transport,
        ));
        for _ in 0..100 {
            if link.status().borrow().connected {
                break;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        assert!(link.status().borrow().connected);
        let mut model = session.model(start);
        model.load_known_terms(0.0, 0.0, zeros.map(|z| z - 0.15));
        if wound {
            // A night's earlier syncs describe +350° of cable travel, ending
            // at this centred mount. Keep the independently known model terms.
            for travel in [0.0, 90.0, 180.0, 270.0, 350.0] {
                let a = (az - 350.0 + travel).rem_euclid(360.0);
                model.syncs.push(Sync {
                    time: start - 400.0 + travel,
                    axis: [a - zeros[0] + 0.15, alt - zeros[1] + 0.15],
                    counts_per_revolution: [360; 2],
                    az_deg: a,
                    alt_deg: alt,
                    roll_deg: None,
                    rms_arcsec: 1.0,
                    epoch: 0,
                });
            }
        }
        let tracker = Tracker::new(
            console,
            link,
            model,
            Site {
                lat_deg: 40.0,
                lon_deg: -75.0,
            },
            15.0,
        );
        tracker.start(target.clone()).await.unwrap();
        std::fs::remove_dir_all(folder).unwrap();
        Self {
            clock,
            mount,
            target,
            tracker,
            receipts,
        }
    }
    fn now(&self) -> f64 {
        self.clock.now()
    }
    fn error_at(&self, time: f64) -> f64 {
        let d = self.mount.true_direction();
        let (h, a) = apparent(&self.target, time);
        separation(d.az, d.alt, a, h) * 3600.0
    }
    fn pointing(&self) {
        let d = self.mount.true_direction();
        let to_j2000 = |h, a| {
            simulator::sky::to_j2000(
                simulator::sky::horizontal_to_date(h, a, simulator::Site::default(), self.now()),
                simulator::sky::julian_date(self.now()),
            )
            .to_radec()
        };
        let (ra, dec) = to_j2000(d.alt, d.az);
        let (h, a) = apparent(&self.target, self.now());
        let (target_ra, target_dec) = to_j2000(h, a);
        self.tracker.pointing(
            (((ra - target_ra).to_degrees() + 180.0).rem_euclid(360.0) - 180.0)
                * target_dec.cos()
                * 3600.0,
            (dec - target_dec).to_degrees() * 3600.0,
        );
    }
    async fn until(&self, time: f64, solve: bool) {
        let mut next_solve = self.now();
        while self.now() < time {
            if solve && self.now() >= next_solve {
                self.pointing();
                next_solve += 2.0;
            }
            tokio::time::sleep(STEP).await;
        }
    }
    async fn stopped(&self) -> bool {
        !self.mount.state(Axis::Azimuth).await.moving
            && !self.mount.state(Axis::Altitude).await.moving
    }
}

#[tokio::test(start_paused = true)]
async fn stops_at_entry_waits_at_exit_and_solves_before_resuming_with_backlash() {
    let (target, entry, exit) = path();
    let rig = Rig::new(entry - 300.0, target, "transit").await;
    rig.until(entry - 1.0, true).await;
    assert_eq!(rig.tracker.state().state, "tracking");
    assert!(rig.error_at(rig.now()) < 30.0);
    assert!(rig
        .receipts
        .lock()
        .unwrap()
        .iter()
        .any(|(_, c)| matches!(c, Command::Rate(_, r) if *r != 0.0)));
    while rig.tracker.state().state == "tracking" && rig.now() < entry + 1.0 {
        tokio::time::sleep(STEP).await;
    }
    assert_eq!(rig.tracker.state().state, "keyhole_wait");
    assert!((rig.now() - entry).abs() <= 1.0);
    rig.until(entry + 90.0, true).await;
    assert_eq!(rig.tracker.state().state, "keyhole_wait");
    assert!(rig.stopped().await);
    assert!(
        rig.error_at(exit) <= 30.0,
        "exit aim {} arcsec",
        rig.error_at(exit)
    );
    let wait_direction = rig.mount.true_direction();
    let wait_receipts = rig.receipts.lock().unwrap().len();
    rig.until(exit - 0.1, true).await;
    assert_eq!(rig.mount.true_direction(), wait_direction);
    assert_eq!(
        rig.receipts.lock().unwrap().len(),
        wait_receipts,
        "no rates during the wait"
    );
    assert_eq!(rig.tracker.state().tracking_offset, [0.0; 2]);
    // Solves made inside the circle cannot restart tracking. A new one after
    // exit is required, even though stale solves were supplied throughout.
    rig.until(exit + 4.0, false).await;
    assert_eq!(rig.tracker.state().state, "keyhole_wait");
    assert!(rig.stopped().await);
    let solve_time = rig.now();
    rig.until(exit + 30.0, true).await;
    assert_eq!(rig.tracker.state().state, "tracking");
    assert!(
        rig.error_at(rig.now()) <= 60.0,
        "recovered {} arcsec",
        rig.error_at(rig.now())
    );
    {
        let receipts = rig.receipts.lock().unwrap();
        let stop = receipts
            .iter()
            .position(|(t, c)| *t >= entry - 0.1 && matches!(c, Command::Stop))
            .unwrap();
        let gotos: Vec<_> = receipts
            .iter()
            .enumerate()
            .filter(|(_, (_, c))| matches!(c, Command::Goto(..)))
            .collect();
        assert!(
            gotos.len() >= 4 && gotos.len() % 2 == 0,
            "planner's stopped approach waypoints"
        );
        assert!(gotos[0].0 > stop);
        for pair in gotos.as_chunks::<2>().0 {
            assert!(matches!(
                pair[0].1 .1,
                Command::Goto(Axis::Azimuth, _, true)
            ));
            assert!(matches!(pair[1].1 .1, Command::Goto(Axis::Altitude, _, _)));
        }
        let last_alt = match gotos.last().unwrap().1 .1 {
            Command::Goto(_, p, _) => p,
            _ => unreachable!(),
        };
        let previous_alt = match gotos[gotos.len() - 3].1 .1 {
            Command::Goto(_, p, _) => p,
            _ => unreachable!(),
        };
        assert!(last_alt > previous_alt, "final altitude approach is upward");
        let takeup = receipts
            .iter()
            .position(|(t, c)| {
                *t >= solve_time && matches!(c, Command::Rate(Axis::Altitude, r) if *r == -0.5)
            })
            .unwrap();
        assert!(matches!(
            receipts[takeup - 1].1,
            Command::Rate(Axis::Altitude, 0.0)
        ));
        assert!(receipts[takeup + 1..].iter().any(
            |(_, c)| matches!(c, Command::Rate(Axis::Altitude, r) if *r < 0.0 && r.abs() < 0.1)
        ));
    }
    rig.tracker.stop("stopped").await;
}

#[tokio::test(start_paused = true)]
async fn planner_refusal_at_future_exit_is_latched_before_any_slew() {
    // Solar coordinates are transformed by the simulator, as in tracking_sim.
    // The Sun is still down at entry but up at the planned exit, proving that
    // this refusal comes from the planner's forecast guard, not the live guard.
    let sun_alt = |time| {
        let jd = simulator::sky::julian_date(time);
        let (ra, dec) = finder::ephemeris::sun(jd);
        let sun = math_core::UnitVector::from_radec(ra.to_radians(), dec.to_radians());
        simulator::sky::horizontal(
            simulator::sky::to_j2000(sun, jd),
            simulator::Site::default(),
            time,
        )
        .0
    };
    let (mut lo, mut hi) = (START + 8.0 * 3600.0, START + 12.0 * 3600.0);
    assert!(sun_alt(lo) < -0.83 && sun_alt(hi) > -0.83);
    for _ in 0..40 {
        let mid = (lo + hi) / 2.0;
        if sun_alt(mid) < -0.83 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let dawn = (lo + hi) / 2.0;
    let transit = simulator::sky::horizontal_to_date(89.0, 180.0, simulator::Site::default(), dawn);
    let (ra, dec) = simulator::sky::to_j2000(transit, simulator::sky::julian_date(dawn)).to_radec();
    let mut target = targets::find("M42").unwrap().clone();
    target.ra_deg = ra.to_degrees();
    target.dec_deg = dec.to_degrees();
    let (entry, exit) = circle_crossings(&target, dawn - 1800.0);
    assert!(sun_alt(entry + 5.0) < -0.83 && sun_alt(exit) > -0.83);
    let rig = Rig::new(entry - 5.0, target, "sun").await;
    rig.until(entry - 1.0, true).await;
    assert_eq!(rig.tracker.state().state, "tracking");
    assert!(!rig.stopped().await);
    rig.until(entry + 5.0, true).await;
    assert_eq!(rig.tracker.state().state, "off");
    assert_eq!(
        rig.tracker.state().ended_reason.as_deref(),
        Some("Sun is up")
    );
    assert!(rig.stopped().await);
    let before = rig.receipts.lock().unwrap().len();
    rig.until(rig.now() + 5.0, true).await;
    let receipts = rig.receipts.lock().unwrap();
    assert_eq!(receipts.len(), before, "refusal cannot restart rates");
    assert!(receipts
        .iter()
        .any(|(_, c)| matches!(c, Command::Rate(_, r) if *r != 0.0)));
    assert!(!receipts.iter().any(|(_, c)| matches!(c, Command::Goto(..))));
}

#[tokio::test(start_paused = true)]
async fn stop_during_slew_or_wait_prevents_exit_from_restarting_the_mount() {
    for offset in [5.0, 90.0] {
        let (target, entry, exit) = path();
        let rig = Rig::new(entry - 5.0, target, &format!("stop-{offset}")).await;
        rig.until(entry + offset, true).await;
        assert_eq!(rig.tracker.state().state, "keyhole_wait");
        assert_eq!(rig.stopped().await, offset == 90.0);
        assert!(rig
            .receipts
            .lock()
            .unwrap()
            .iter()
            .any(|(_, c)| matches!(c, Command::Goto(..))));
        rig.tracker.stop("stopped").await;
        // A slew at 3.4°/s brakes at the simulator's 3°/s²: 1.134 s.
        rig.until(rig.now() + 1.5, true).await;
        assert!(rig.stopped().await);
        let before = rig.receipts.lock().unwrap().len();
        rig.until(exit + 30.0, true).await;
        assert_eq!(rig.tracker.state().state, "off");
        assert_eq!(rig.tracker.state().ended_reason.as_deref(), Some("stopped"));
        assert_eq!(rig.receipts.lock().unwrap().len(), before);
        assert!(rig.stopped().await);
    }
}

#[tokio::test(start_paused = true)]
async fn keyhole_plan_unwinds_the_cable_relative_to_the_nights_first_sync() {
    let (target, entry, exit) = path();
    let rig = Rig::with_wind(entry - 5.0, target, "wind", true).await;
    rig.until(entry - 1.0, true).await;
    let before = rig.mount.state(Axis::Azimuth).await.position;
    rig.until(entry + 100.0, true).await;
    assert_eq!(rig.tracker.state().state, "keyhole_wait");
    assert!(rig.stopped().await);
    assert!(rig.error_at(exit) <= 30.0);
    let first = rig
        .receipts
        .lock()
        .unwrap()
        .iter()
        .find_map(|(_, c)| {
            if let Command::Goto(Axis::Azimuth, p, _) = c {
                Some(*p)
            } else {
                None
            }
        })
        .unwrap();
    let turn =
        f64::from(first - before) * 360.0 / f64::from(rig.mount.axes()[0].counts_per_revolution);
    // The short clockwise route would exceed a revolution from the first sync.
    // The legal planner route unwinds by about 240°, then approaches positively.
    assert!((-260.0..-220.0).contains(&turn), "unwinding turn {turn}");
    rig.tracker.stop("stopped").await;
}

#[tokio::test(start_paused = true)]
async fn forecast_detects_a_grazing_pass_between_its_minute_samples() {
    let transit_time = START + 600.0;
    let transit = simulator::sky::horizontal_to_date(
        88.00001,
        180.0,
        simulator::Site::default(),
        transit_time,
    );
    let (ra, dec) =
        simulator::sky::to_j2000(transit, simulator::sky::julian_date(transit_time)).to_radec();
    let mut target = targets::find("M42").unwrap().clone();
    target.ra_deg = ra.to_degrees();
    target.dec_deg = dec.to_degrees();
    let entry = crossing(&target, transit_time - 60.0, transit_time, true);
    let exit = crossing(&target, transit_time, transit_time + 60.0, false);
    assert!((1.0..5.0).contains(&(exit - entry)));
    let rig = Rig::new(entry - 120.333, target, "grazing").await;
    rig.until(entry - 1.0, true).await;
    assert_eq!(rig.tracker.state().state, "tracking");
    assert!(!rig.stopped().await);
    rig.until(entry + 1.0, false).await;
    assert_eq!(rig.tracker.state().state, "keyhole_wait");
    rig.tracker.stop("stopped").await;
}
