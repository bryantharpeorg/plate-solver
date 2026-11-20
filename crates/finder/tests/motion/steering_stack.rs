//! US7: simulator truth at the instants recorded in each saved FITS sub,
//! 015's exposure formula, 031's refusals and a scripted tracker's gap log.
//! Time is paused; polls are 100 ms, bounded at 300 s. Angular bounds below
//! are in arcseconds; exposure settings and counts are exact.
use crate::support::{goto::Stars, live as support, Scratch};
use axum::{http::StatusCode, Router};
use finder::{
    ephemeris::{self, Exposure},
    imaging::{
        simulated::SimulatedSolver,
        stack::service::Dropped,
        steering::{GapFuture, Steered, Stepped, TrackingError},
    },
    link::Transport,
    session::{Aim, Session},
    sky::{self, Observer},
    targets, AppState,
};
use math_core::{angular_distance, UnitVector};
use scope_link::memory;
use serde_json::{json, Value};
use simulator::{
    mount::{MountConfig, SimMount},
    Optics, SimNode, Simulator,
};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

const POLL: Duration = Duration::from_millis(100);

struct Rig {
    app: Router,
    state: AppState,
    sim: Simulator,
    root: PathBuf,
    _scratch: Scratch,
}
impl Rig {
    async fn new(tracked: bool, rate_error: f64, displaced: bool, stop_frames: bool) -> Self {
        let scratch = Scratch::new("steering-stack");
        let root = scratch.0.join("imaging");
        let sim = support::sim();
        let mount = if tracked {
            SimMount::with_config(
                sim.clock().clone(),
                MountConfig {
                    base_tilt_deg: 0.0,
                    base_tilt_toward_deg: 0.0,
                    zero_points_deg: [120.0, 50.0],
                    backlash_deg: 0.0,
                },
            )
        } else {
            SimMount::new(sim.clock().clone())
        };
        mount.set_rate_error(rate_error);
        sim.connect_mount(mount);
        sim.push_to(50.0, 120.0);
        if tracked {
            let target = targets::find("M42").unwrap();
            sim.point_eyepiece_at(target.ra_deg, target.dec_deg);
            if displaced {
                let c = sim.telescope().camera;
                sim.set_optics(Optics {
                    eyepiece_px: (716.0, 420.0),
                    ..Optics::default()
                });
                sim.push_to(c.alt, c.az);
            }
        }
        let node = Arc::new(
            SimNode::new(sim.clone(), "test-token", Default::default())
                .unwrap()
                .with_frame_interval(Duration::from_millis(500)),
        );
        let transport = Transport::connector(move || {
            let node = node.clone();
            async move {
                let (client, server) = tokio::io::duplex(64 * 1024);
                tokio::spawn(async move {
                    memory::serve_routes_on(server, node.service().routes())
                        .await
                        .unwrap();
                });
                Ok::<_, std::io::Error>(client)
            }
        });
        let session = Session::default();
        let mut edit = json!({
            "node":{"address":"scope.local:1", "token":"test-token"},
            "site":{"lat_deg":40, "lon_deg":-75}
        });
        if tracked {
            edit["target"] = json!("M42");
        }
        session
            .edit(serde_json::from_value(edit).unwrap(), support::START)
            .unwrap();
        if tracked {
            session
                .set_aim(Some(Aim { x: 700.0, y: 420.0 }), support::START)
                .unwrap();
        }
        let state = AppState::shared(support::DB.clone(), Default::default())
            .with_session(session)
            .with_clock(Arc::new(support::ConsoleClock(sim.clock().clone())))
            .with_link(transport)
            .with_live_solver(Arc::new(Stars))
            .with_imaging(&root)
            .with_imaging_solver(Arc::new(SimulatedSolver::new(sim.clone())));
        let rig = Self {
            app: finder::router(state.clone()),
            state,
            sim,
            root,
            _scratch: scratch,
        };
        if tracked {
            rig.wait("/api/session", |view| {
                let pointing = &view["pointing"];
                let target = targets::find("M42").unwrap();
                !rig.state
                    .session()
                    .model(rig.sim.clock().now())
                    .syncs
                    .is_empty()
                    && pointing["ra_deg"]
                        .as_f64()
                        .zip(pointing["dec_deg"].as_f64())
                        .is_some_and(|(ra, dec)| {
                            separation((ra, dec), (target.ra_deg, target.dec_deg)) < 60.0
                        })
            })
            .await;
            rig.call("POST", "/api/track", json!({"on":true})).await;
            rig.wait("/api/session", |v| v["track"]["state"] == "tracking")
                .await;
            if stop_frames {
                rig.sim.stop_frames();
            }
        }
        rig.wait("/api/imaging", |v| v["state"] == "ready").await;
        rig
    }
    async fn call(&self, method: &str, path: &str, body: Value) -> Value {
        let (status, value) = support::request(&self.app, method, path, body).await;
        assert_eq!(status, StatusCode::OK, "{method} {path}: {value}");
        assert_ne!(value["stack"]["last"]["outcome"], "not_steady", "{value}");
        value
    }
    async fn imaging(&self) -> Value {
        self.call("GET", "/api/imaging", Value::Null).await
    }
    async fn wait(&self, path: &str, mut done: impl FnMut(&Value) -> bool) -> Value {
        let mut last = Value::Null;
        for _ in 0..3000 {
            last = self.call("GET", path, Value::Null).await;
            if done(&last) {
                return last;
            }
            tokio::time::sleep(POLL).await;
        }
        panic!("{path} never reached the awaited state: {last}");
    }
    async fn exposure(&self, ms: f64) {
        self.call("PUT", "/api/imaging/settings", json!({"exposure_ms":ms}))
            .await;
    }
    async fn start(&self) -> String {
        let v = self.call("POST", "/api/imaging/stack", json!({})).await;
        // Start spawns the taker; let it acquire its Hold before reading it.
        tokio::task::yield_now().await;
        v["stack"]["id"].as_str().unwrap().to_string()
    }
    async fn stop(&self) -> Value {
        self.call("DELETE", "/api/imaging/stack", Value::Null).await
    }
    fn folder(&self, id: &str) -> PathBuf {
        self.root.join("runs").join(id)
    }
    fn subs(&self, id: &str) -> Vec<PathBuf> {
        let mut paths: Vec<_> = fs::read_dir(self.folder(id).join("subs"))
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|e| e == "fits"))
            .collect();
        paths.sort();
        paths
    }
    fn drift(&self, path: &Path) -> f64 {
        let (start, exposure) = sub_times(path);
        let a = self.sim.imaging_truth_at(start);
        let b = self.sim.imaging_truth_at(start + exposure);
        separation((a.ra_deg, a.dec_deg), (b.ra_deg, b.dec_deg))
    }
}
fn separation(a: (f64, f64), b: (f64, f64)) -> f64 {
    angular_distance(
        UnitVector::from_radec(a.0.to_radians(), a.1.to_radians()),
        UnitVector::from_radec(b.0.to_radians(), b.1.to_radians()),
    )
    .to_degrees()
        * 3600.0
}

// Read the FITS header independently of the console's FITS decoder. These
// runs fit within the given UTC date; midnight on that date is Unix 1768521600.
fn sub_times(path: &Path) -> (f64, f64) {
    let bytes = fs::read(path).unwrap();
    let cards: Vec<_> = bytes
        .as_chunks::<80>()
        .0
        .iter()
        .take_while(|c| &c[..8] != b"END     ")
        .filter(|c| c[8] == b'=')
        .map(|c| {
            (
                String::from_utf8_lossy(&c[..8]).trim().to_string(),
                String::from_utf8_lossy(&c[10..])
                    .trim()
                    .trim_matches('\'')
                    .trim()
                    .to_string(),
            )
        })
        .collect();
    let value = |key| cards.iter().find(|(k, _)| k == key).unwrap().1.as_str();
    let date = value("DATE-OBS");
    assert_eq!(&date[..11], "2026-01-16T");
    assert_eq!(date.len(), 23, "UTC to the millisecond");
    let number = |s: &str| s.parse::<f64>().unwrap();
    let start = 1_768_521_600.0
        + number(&date[11..13]) * 3600.0
        + number(&date[14..16]) * 60.0
        + number(&date[17..]);
    (start, number(value("EXPTIME")))
}
fn balanced(stack: &Value) {
    let drops: u64 = stack["dropped"]
        .as_object()
        .unwrap()
        .values()
        .map(|n| n.as_u64().unwrap())
        .sum();
    assert_eq!(
        stack["taken"].as_u64().unwrap(),
        stack["stacked"].as_u64().unwrap() + drops
    );
}

/// S1: each saved sub's simulator truth, at most 0.15 arcsec from sub 14.
#[tokio::test(start_paused = true)]
async fn accepted_subs_steer_a_mount_one_percent_fast_and_stop_releases_it() {
    let rig = Rig::new(true, 0.01, false, false).await;
    rig.exposure(5000.0).await;
    let id = rig.start().await;
    let view = rig
        .wait("/api/imaging", |v| {
            v["steering"]["subs"].as_u64().unwrap_or(0) >= 18
        })
        .await;
    assert_eq!(view["steering"]["source"], "subs");
    assert_eq!(view["stack"]["dropped"]["not_steady"], 0);
    rig.stop().await;
    let subs = rig.subs(&id);
    assert!(subs.len() >= 18);
    for (n, path) in subs.iter().enumerate().skip(13) {
        let drift = rig.drift(path);
        assert!(drift <= 0.15, "sub {} drifts {drift} arcsec", n + 1);
    }
    let view = rig.imaging().await;
    assert_eq!(view["stack"]["dropped"]["not_steady"], 0);
    assert_eq!(view["steering"]["source"], "tracker");
    assert_eq!(view["steering"]["subs"], 0);
}

/// S2: 015's formula at the console read's time, within 2%; exact settings.
#[tokio::test(start_paused = true)]
async fn the_cap_sets_thirty_seconds_through_the_lease_and_restores_sixty() {
    let rig = Rig::new(true, 0.0, false, false).await;
    rig.call("PUT", "/api/imaging/steering", json!({"smear_px":2}))
        .await;
    rig.exposure(60000.0).await;
    let before = rig.imaging().await;
    assert_eq!(before["steering"]["would_take_s"], 30.0);
    assert!(before["steering"].get("exposure_s").is_none());
    let id = rig.start().await;
    let view = rig
        .wait("/api/imaging", |_| !rig.subs(&id).is_empty())
        .await;
    let now = rig.sim.clock().now();
    let target = targets::find("M42").unwrap();
    let (alt, az) = sky::alt_az(
        target.ra_deg,
        target.dec_deg,
        &Observer {
            lat: 40.0,
            lon: -75.0,
            time: now,
        },
    );
    let rotation = ephemeris::altaz_rates_from_alt_az(40.0, alt, az)
        .field_rotation_deg_per_h
        .to_radians()
        / 3600.0;
    let Exposure::Seconds(expected) =
        ephemeris::longest_exposure(23.63232, 15.71664, 19.12, rotation, 2.0)
    else {
        panic!("M42 has a finite limit");
    };
    let actual = view["steering"]["longest_exposure_s"].as_f64().unwrap();
    assert!(
        (actual - expected).abs() <= expected * 0.02,
        "{actual} vs {expected} s"
    );
    assert_eq!(sub_times(&rig.subs(&id)[0]).1, 30.0);
    assert_eq!(view["settings"]["exposure_ms"], 30000.0);
    assert_eq!(view["steering"]["limited"], true);
    assert_eq!(view["steering"]["exposure_s"], 30.0);
    assert_eq!(view["steering"]["over_limit"], false);
    rig.stop().await;
    assert_eq!(rig.imaging().await["settings"]["exposure_ms"], 60000.0);
}

/// S3: crossing the end of a tracker drops a sub before solve or last; all
/// twelve reasons balance and persist in the manifest. Counts are exact.
#[tokio::test(start_paused = true)]
async fn turning_track_off_during_a_sub_drops_it_before_solving() {
    let rig = Rig::new(true, 0.0, false, false).await;
    rig.exposure(5000.0).await;
    let id = rig.start().await;
    rig.wait("/api/imaging", |v| {
        v["stack"]["stacked"].as_u64().unwrap_or(0) >= 2
    })
    .await;
    let exposing = rig
        .wait("/api/imaging", |v| v["frame"]["state"] == "exposing")
        .await;
    let taken = exposing["stack"]["taken"].as_u64().unwrap();
    rig.call("POST", "/api/track", json!({"on":false})).await;
    rig.call("POST", "/api/track", json!({"on":true})).await;
    rig.wait("/api/imaging", |v| {
        v["stack"]["taken"].as_u64().unwrap_or(0) >= taken + 3
    })
    .await;
    rig.stop().await;
    let view = rig.imaging().await;
    let dropped = view["stack"]["dropped"]["not_steady"].as_u64().unwrap();
    assert!(dropped >= 1, "{view}");
    assert_eq!(view["steering"]["dropped_not_steady"], dropped);
    balanced(&view["stack"]);
    let manifest: Value =
        serde_json::from_slice(&fs::read(rig.folder(&id).join("stack.json")).unwrap()).unwrap();
    let drops: Vec<_> = manifest["subs"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| s["outcome"] == "not_steady")
        .collect();
    assert_eq!(drops.len() as u64, dropped);
    for sub in drops {
        let frame_id = sub["frame_id"].as_u64().unwrap();
        assert!(!rig
            .folder(&id)
            .join(format!("subs/{frame_id:06}.solve.json"))
            .exists());
    }
    assert_eq!(
        Dropped {
            not_steady: 2,
            ..Default::default()
        }
        .total(),
        2
    );
}

/// S4: the unchanged 032 solving rig, no target or tracker; exact 2 s subs.
#[tokio::test(start_paused = true)]
async fn without_a_tracker_the_stack_is_neither_capped_paused_nor_dropped() {
    let rig = Rig::new(false, 0.0, false, false).await;
    rig.exposure(2000.0).await;
    let id = rig.start().await;
    rig.wait("/api/imaging", |v| {
        assert_eq!(v["steering"]["source"], "none");
        assert_eq!(v["steering"]["subs"], 0);
        assert_eq!(v["steering"]["waiting"], false);
        v["stack"]["taken"].as_u64().unwrap_or(0) >= 3
    })
    .await;
    rig.stop().await;
    let view = rig.imaging().await;
    assert_eq!(view["steering"]["source"], "none");
    assert_eq!(view["steering"]["subs"], 0);
    assert_eq!(view["stack"]["dropped"]["not_steady"], 0);
    assert!(rig.subs(&id).len() >= 3);
    for sub in rig.subs(&id) {
        assert_eq!(sub_times(&sub).1, 2.0);
    }
}

// A gap waits ten seconds and owns no tracker. The log is the independent
// oracle for cancellation and release. 37.15 s halves a 60 s setting once.
#[derive(Default)]
struct ScriptedTracker {
    log: Mutex<Vec<&'static str>>,
}
impl ScriptedTracker {
    fn events(&self) -> Vec<&'static str> {
        self.log.lock().unwrap().clone()
    }
}
impl Steered for ScriptedTracker {
    fn target(&self) -> Option<(f64, f64)> {
        let target = targets::find("M42").unwrap();
        Some((target.ra_deg, target.dec_deg))
    }
    fn steer(&self, _: TrackingError) {}
    fn gap(&self) -> GapFuture {
        self.log.lock().unwrap().push("gap");
        Box::pin(async {
            tokio::time::sleep(Duration::from_secs(10)).await;
            Stepped::default()
        })
    }
    fn release(&self) {
        self.log.lock().unwrap().push("release");
    }
    fn steered_by_subs(&self) -> bool {
        false
    }
    fn longest_exposure_s(&self, _: f64) -> Option<f64> {
        Some(37.15)
    }
    fn longest_exposure_ahead_s(&self, _: f64) -> Option<f64> {
        Some(37.15)
    }
}
async fn scripted_rig() -> (Rig, Arc<ScriptedTracker>) {
    let rig = Rig::new(false, 0.0, false, false).await;
    rig.exposure(60000.0).await;
    let tracker = Arc::new(ScriptedTracker::default());
    let source = tracker.clone();
    rig.state
        .imaging()
        .unwrap()
        .steering()
        .set_source(Arc::new(move || Some(source.clone())));
    (rig, tracker)
}

/// S5: the link fails well inside the ten-second first gap (031's 3 s
/// detection bound). No sub may be taken at the observer's longer setting.
#[tokio::test(start_paused = true)]
async fn a_refused_exposure_change_ends_as_camera_without_taking_a_sub() {
    let (rig, tracker) = scripted_rig().await;
    rig.start().await;
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert_eq!(tracker.events(), ["gap"]);
    rig.sim.set_unresponsive(true);
    let view = rig
        .wait("/api/imaging", |v| v["stack"]["state"] == "ended")
        .await;
    assert_eq!(view["stack"]["ended_reason"], "camera");
    assert_eq!(
        view["stack"]["error"],
        "the exposure could not be set to 30000 ms: the scope node is not connected"
    );
    assert_eq!(view["stack"]["taken"], 0);
    assert_eq!(tracker.events(), ["gap", "release"]);
    rig.sim.set_unresponsive(false);
    let ready = rig.wait("/api/imaging", |v| v["state"] == "ready").await;
    assert_eq!(ready["settings"]["exposure_ms"], 60000.0);
}

/// S6: plan rule 1, checked against the entire stack service text.
#[test]
fn the_stack_reaches_the_tracker_only_through_steering() {
    let text = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/imaging/stack/service.rs"),
    )
    .unwrap();
    for name in [
        "ephemeris",
        "alt_az",
        "parallactic",
        "counts_per_revolution",
        "AxisInfo",
        "MountClient",
        "azimuth",
        "altitude",
        "field_rotation",
        "hour_angle",
        "meridian",
        "backlash",
        "keyhole",
    ] {
        assert!(!text.contains(name), "stack/service.rs holds {name}");
    }
}

/// S7: 16 finder pixels are about 477 arcsec. Simulator eyepiece readings
/// stay within 60 arcsec of the first, and 038 reads no push at any poll.
#[tokio::test(start_paused = true)]
async fn a_displaced_aim_point_makes_the_steering_stand_back() {
    let rig = Rig::new(true, 0.0, true, false).await;
    rig.exposure(5000.0).await;
    rig.start().await;
    let first = rig.sim.telescope().eyepiece;
    let mut last = Value::Null;
    for _ in 0..3000 {
        let session = rig.call("GET", "/api/session", Value::Null).await;
        assert!(session["track"].get("push").is_none(), "{session}");
        last = rig.imaging().await;
        let eyepiece = rig.sim.telescope().eyepiece;
        let moved = separation((first.ra, first.dec), (eyepiece.ra, eyepiece.dec));
        assert!(moved <= 60.0, "eyepiece moved {moved} arcsec");
        if last["stack"]["taken"].as_u64().unwrap_or(0) >= 12 {
            break;
        }
        tokio::time::sleep(POLL).await;
    }
    assert!(last["stack"]["taken"].as_u64().unwrap() >= 12, "{last}");
    let far = last["steering"]["far_arcsec"].as_f64().unwrap();
    assert!((420.0..=540.0).contains(&far), "far {far} arcsec");
    assert_eq!(last["steering"]["source"], "tracker");
    assert_eq!(last["steering"]["subs"], 0);
    rig.stop().await;
}

/// S8: negative control. With both correction streams off, a 1% fast mount
/// drifts about 0.749 arcsec per 5 s; at least 8 of the first 12 exceed 0.5.
#[tokio::test(start_paused = true)]
async fn the_switch_off_leaves_the_uncorrected_drift_visible() {
    let rig = Rig::new(true, 0.01, false, true).await;
    rig.exposure(5000.0).await;
    rig.call("PUT", "/api/imaging/steering", json!({"on":false}))
        .await;
    let id = rig.start().await;
    rig.wait("/api/imaging", |v| {
        assert_eq!(v["steering"]["subs"], 0);
        assert_eq!(v["steering"]["source"], "tracker");
        rig.subs(&id).len() >= 12
    })
    .await;
    rig.stop().await;
    let subs = rig.subs(&id);
    assert!(subs.len() >= 12);
    let drifts: Vec<_> = subs.iter().take(12).map(|s| rig.drift(s)).collect();
    assert!(
        drifts.iter().filter(|&&d| d > 0.5).count() >= 8,
        "{drifts:?}"
    );
    assert_eq!(rig.imaging().await["steering"]["subs"], 0);
}

/// S9: 035 stop cancels a first gap within 0.5 s of virtual time.
#[tokio::test(start_paused = true)]
async fn stopping_during_the_first_gap_releases_without_exposing() {
    let (rig, tracker) = scripted_rig().await;
    rig.start().await;
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert_eq!(tracker.events(), ["gap"]);
    let before = tokio::time::Instant::now();
    tokio::time::timeout(Duration::from_millis(500), rig.stop())
        .await
        .unwrap();
    assert!(before.elapsed() <= Duration::from_millis(500));
    let view = rig.imaging().await;
    assert_eq!(view["stack"]["state"], "ended");
    assert_eq!(view["stack"]["ended_reason"], "stopped");
    assert_eq!(view["stack"]["taken"], 0);
    assert_eq!(tracker.events(), ["gap", "release"]);
}

/// S10: the second gap does not set again. The take's exact refusal wins;
/// refusing restoration changes nothing else and keeps the 30 s setting.
#[tokio::test(start_paused = true)]
async fn losing_the_link_in_the_second_gap_keeps_the_take_refusal_and_setting() {
    let (rig, tracker) = scripted_rig().await;
    rig.start().await;
    rig.wait("/api/imaging", |v| v["stack"]["taken"] == 1).await;
    assert_eq!(tracker.events().iter().filter(|&&e| e == "gap").count(), 2);
    rig.sim.set_unresponsive(true);
    let view = rig
        .wait("/api/imaging", |v| v["stack"]["state"] == "ended")
        .await;
    assert_eq!(view["stack"]["ended_reason"], "camera");
    assert_eq!(view["stack"]["taken"], 1);
    assert_eq!(view["stack"]["error"], "the scope node is not connected");
    assert_eq!(tracker.events().last(), Some(&"release"));
    rig.sim.set_unresponsive(false);
    let ready = rig.wait("/api/imaging", |v| v["state"] == "ready").await;
    assert_eq!(ready["settings"]["exposure_ms"], 30000.0);
}
