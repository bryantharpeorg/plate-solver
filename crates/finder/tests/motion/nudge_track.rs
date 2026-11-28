//! With Track on, a press moves the place Track holds by the step and Track makes the
//! step (055 US3). Oracles: the simulator's true eyepiece direction and its own horizon
//! transform (the stepped place), the steps' sizes and angles, the model's epoch and slip
//! mark, and the session's own `push`, `axis_move`, `motion` and `track` words, in paused
//! time. No expected value is read from the code under test.
use super::nudge::{nudge, refused, write, START, STEPS, WAYS};
use crate::support::{goto::Rig, tracker};
use finder::{model::separation, targets};
use math_core::UnitVector;
use serde_json::{json, Value};

const SETTLING: &str = "a push by hand is still settling: press again in a few seconds";
const NOT_TRACKING: &str = "Track is not tracking yet";

/// The Track rig at `start` as spec.md defines it: the simulator's eyepiece on M42 before
/// the sync, a second to settle, Track on, and 30 s of tracking, read `tracking`.
async fn track_rig(start: f64) -> Rig {
    let rig = Rig::new(start, 50.0, 120.0, true).await;
    let m42 = targets::find("M42").unwrap();
    rig.sim.point_eyepiece_at(m42.ra_deg, m42.dec_deg);
    rig.sync().await;
    // Track reads the newest solved pointing, so wait for one solved on M42.
    for _ in 0..10 {
        let pointing = rig.view().await["pointing"].clone();
        let within = separation(
            pointing["ra_deg"].as_f64().unwrap(),
            pointing["dec_deg"].as_f64().unwrap(),
            m42.ra_deg,
            m42.dec_deg,
        ) < 1.0 / 60.0;
        if within {
            break;
        }
        rig.tick().await;
    }
    for _ in 0..10 {
        rig.tick().await;
    }
    let (status, body) = write(&rig.app, "POST", "/api/track", json!({"on": true})).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["track"]["state"], "tracking", "{body}");
    for _ in 0..300 {
        rig.tick().await;
    }
    assert_eq!(rig.view().await["track"]["state"], "tracking");
    rig
}

/// The place Track holds, J2000 degrees.
fn held(rig: &Rig) -> (f64, f64) {
    rig.state
        .session()
        .tracker()
        .expect("Track holds a tracker")
        .target()
        .expect("Track holds a place")
}

/// The place the eyepiece points at now, J2000 degrees, from the simulator's own ray.
fn true_place(rig: &Rig) -> (f64, f64) {
    let direction = rig.mount.true_direction();
    let (ra, dec) = tracker::equatorial(direction.alt, direction.az, rig.now()).to_radec();
    (ra.to_degrees(), dec.to_degrees())
}

/// The altitude of a J2000 place at `time`, degrees, from the simulator's horizon transform.
fn altitude(place: (f64, f64), time: f64) -> f64 {
    let vector = UnitVector::from_radec(place.0.to_radians(), place.1.to_radians());
    simulator::sky::horizontal(vector, simulator::Site::default(), time).0
}

/// The place `held` moved by a step of `size` arcseconds at `angle` degrees clockwise of
/// the view's up, at `time`, with the simulator's horizon transform: the stepped place.
fn stepped(held: (f64, f64), size: f64, angle: f64, time: f64) -> (f64, f64) {
    let vector = UnitVector::from_radec(held.0.to_radians(), held.1.to_radians());
    let (alt, az) = simulator::sky::horizontal(vector, simulator::Site::default(), time);
    let angle = angle.to_radians();
    let up = size * angle.cos();
    let right = size * angle.sin();
    let (ra, dec) = tracker::equatorial(
        alt + up / 3600.0,
        az + right / 3600.0 / alt.to_radians().cos(),
        time,
    )
    .to_radec();
    (ra.to_degrees(), dec.to_degrees())
}

/// Arcseconds between two J2000 places given in degrees.
fn apart(a: (f64, f64), b: (f64, f64)) -> f64 {
    separation(a.0, a.1, b.0, b.1) * 3600.0
}

/// Whether the tracker's push check reads the tracker's state as false.
fn checks_push(rig: &Rig) -> bool {
    rig.state
        .session()
        .tracker()
        .expect("Track holds a tracker")
        .state()
        .checks_push
}

async fn epoch(rig: &Rig) -> Value {
    rig.view().await["model"]["epoch"].clone()
}

/// Reads every 100 ms, within 120 s, until `condition` holds, and returns that read.
async fn until(rig: &Rig, mut condition: impl FnMut(&Value) -> bool) -> Value {
    let start = rig.now();
    loop {
        let view = rig.view().await;
        if condition(&view) {
            return view;
        }
        assert!(rig.now() - start < 120.0, "never came true: {view}");
        rig.tick().await;
    }
}

/// Settled: read every 100 ms, within 120 s, until the true place is within 10″ of the
/// place Track holds.
async fn settle(rig: &Rig) {
    let start = rig.now();
    while apart(true_place(rig), held(rig)) > 10.0 {
        assert!(
            rig.now() - start < 120.0,
            "Track did not settle: {}",
            rig.view().await
        );
        rig.tick().await;
    }
}

/// One session read: its time, the session, the true place's distance from the stepped
/// place in arcseconds, and whether the model marks a slip.
struct Read {
    at: f64,
    view: Value,
    apart: f64,
    slip: bool,
}

async fn read(rig: &Rig, stepped: (f64, f64)) -> Read {
    let at = rig.now();
    Read {
        at,
        view: rig.view().await,
        apart: apart(true_place(rig), stepped),
        slip: rig.state.session().model(at).slip_pending,
    }
}

/// Reads every 100 ms for `seconds`: the first tick, then the read.
async fn reads(rig: &Rig, stepped: (f64, f64), seconds: usize) -> Vec<Read> {
    let mut reads = Vec::new();
    for _ in 0..seconds * 10 {
        rig.tick().await;
        reads.push(read(rig, stepped).await);
    }
    reads
}

/// No read shows a push by hand, and none marks a slip.
fn quiet(reads: &[Read]) {
    for read in reads {
        assert!(read.view["track"].get("push").is_none(), "{}", read.view);
        assert!(!read.slip, "a slip is marked at {:.1} s", read.at);
    }
}

/// The true place comes within 10″ of the stepped place within 10 s of the press, and
/// stays within 30″ of it over the next 20 s.
fn lands(reads: &[Read], pressed: f64) {
    let first = reads
        .iter()
        .find(|read| read.apart <= 10.0)
        .expect("the place never came within 10 arcseconds");
    assert!(
        first.at - pressed <= 10.0 + 1e-6,
        "within 10 arcseconds only {:.1} s after the press",
        first.at - pressed
    );
    for read in reads
        .iter()
        .filter(|read| read.at >= first.at && read.at <= first.at + 20.0)
    {
        assert!(
            read.apart <= 30.0,
            "{:.1} arcseconds off {:.1} s after it came within 10",
            read.apart,
            read.at - first.at
        );
    }
}

/// A press with Track on in the mount's view (US3-S1): the answer, the session at once,
/// 30 s of reads, `done` with Track tracking and `motion` as it was before the press.
async fn press_on_track(rig: &Rig, way: &str, angle: f64, step: &str, size: f64) {
    settle(rig).await;
    let held = held(rig);
    let motion = rig.view().await["motion"].clone();
    let (status, body) = nudge(&rig.app, way, step, "mount").await;
    assert_eq!(status, 200, "{way} {step}: {body}");
    assert_eq!(body["moved"], "track", "{way} {step}: {body}");
    let pressed = rig.now();
    let stepped = stepped(held, size, angle, pressed);
    let mut all = vec![read(rig, stepped).await];
    let at_once = &all[0].view;
    assert_eq!(
        at_once["axis_move"],
        json!({"state": "moving", "tool": "nudge"}),
        "{way} {step}: {at_once}"
    );
    assert_eq!(at_once["motion"]["by"], "page", "{at_once}");
    assert_eq!(at_once["motion"]["tool"], "nudge", "{at_once}");
    all.extend(reads(rig, stepped, 30).await);
    quiet(&all);
    lands(&all, pressed);
    let done = all
        .iter()
        .find(|read| read.view["axis_move"]["state"] == "done")
        .unwrap_or_else(|| panic!("{way} {step} never reads done"));
    assert_eq!(done.view["track"]["state"], "tracking", "{way} {step}");
    assert_eq!(done.view["motion"], motion, "{way} {step}");
}

// US3-S1: with Track on, each step each way moves the place Track holds and Track makes
// the step; the model's epoch is the one before the presses.
#[tokio::test(start_paused = true)]
async fn each_step_each_way_moves_the_held_place_and_track_makes_it() {
    let rig = track_rig(START).await;
    let before = epoch(&rig).await;
    for (step, size) in STEPS {
        for (way, angle) in WAYS {
            press_on_track(&rig, way, angle, step, size).await;
        }
    }
    assert_eq!(epoch(&rig).await, before);
}

// US3-S2: at START + 4 h, where M42 has set west of the meridian and Track drives
// altitude down, coarse up and then coarse down land as US3-S1 says.
#[tokio::test(start_paused = true)]
async fn coarse_up_then_down_land_while_track_drives_altitude_down() {
    let rig = track_rig(START + 4.0 * 3600.0).await;
    let m42 = targets::find("M42").unwrap();
    let place = (m42.ra_deg, m42.dec_deg);
    assert!(
        altitude(place, rig.now() + 60.0) < altitude(place, rig.now()),
        "M42 is not setting at START + 4 h"
    );
    let before = epoch(&rig).await;
    press_on_track(&rig, "up", 0.0, "coarse", 240.0).await;
    press_on_track(&rig, "down", 180.0, "coarse", 240.0).await;
    assert_eq!(epoch(&rig).await, before);
}

// US3-S3: a second coarse press 1 s after the first is refused while the first step is
// made; the first lands within 10″ ten seconds after its press.
#[tokio::test(start_paused = true)]
async fn a_second_press_during_a_step_is_refused_and_the_first_still_lands() {
    let rig = track_rig(START).await;
    settle(&rig).await;
    let before = epoch(&rig).await;
    let held = held(&rig);
    let (status, body) = nudge(&rig.app, "left", "coarse", "mount").await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["moved"], "track", "{body}");
    let pressed = rig.now();
    let stepped = stepped(held, 240.0, 270.0, pressed);
    let first = reads(&rig, stepped, 1).await;
    assert_eq!(
        first.last().unwrap().view["axis_move"]["state"],
        "moving",
        "one second after the press"
    );
    refused(
        &rig.app,
        "left",
        "coarse",
        "mount",
        "the mount is already moving",
    )
    .await;
    let rest = reads(&rig, stepped, 9).await;
    let all: Vec<Read> = first.into_iter().chain(rest).collect();
    quiet(&all);
    let ten = all.last().unwrap();
    assert!(
        ten.apart <= 10.0,
        "{:.1} arcseconds from the stepped place ten seconds after the press",
        ten.apart
    );
    assert_eq!(epoch(&rig).await, before);
}

// US3-S4: a push by hand of 1.5′ is held, and a coarse press after it lands, with Track
// then holding the place; once down and once on a new rig up.
#[tokio::test(start_paused = true)]
async fn a_coarse_press_after_a_small_push_by_hand_lands_and_track_holds() {
    for (push, way, angle) in [(-1.5, "up", 0.0), (1.5, "down", 180.0)] {
        let rig = track_rig(START).await;
        settle(&rig).await;
        let before = epoch(&rig).await;
        rig.mount.push_by(0.0, push / 60.0);
        let held = held(&rig);
        let (status, body) = nudge(&rig.app, way, "coarse", "mount").await;
        assert_eq!(status, 200, "{way} after a {push}′ push: {body}");
        let pressed = rig.now();
        let stepped = stepped(held, 240.0, angle, pressed);
        let reads = reads(&rig, stepped, 45).await;
        quiet(&reads);
        assert!(
            reads.iter().any(|read| read.apart <= 10.0),
            "{way} after a {push}′ push: the place never came within 10 arcseconds"
        );
        for read in reads.iter().filter(|read| read.at >= pressed + 35.0) {
            assert!(
                read.apart <= 15.0,
                "{way} after a {push}′ push: {:.1} arcseconds off {:.1} s after the press",
                read.apart,
                read.at - pressed
            );
        }
        assert_eq!(epoch(&rig).await, before);
    }
}

// US3-S5: with the node's frames stopped, a coarse right press still lands within 20″ in 5 s.
#[tokio::test(start_paused = true)]
async fn with_no_frames_a_press_still_lands_within_twenty_arcseconds_in_five_seconds() {
    let rig = track_rig(START).await;
    settle(&rig).await;
    rig.controls.stop_frames();
    let held = held(&rig);
    let (status, body) = nudge(&rig.app, "right", "coarse", "mount").await;
    assert_eq!(status, 200, "{body}");
    let pressed = rig.now();
    let stepped = stepped(held, 240.0, 90.0, pressed);
    let reads = reads(&rig, stepped, 5).await;
    let closest = reads
        .iter()
        .map(|read| read.apart)
        .fold(f64::INFINITY, f64::min);
    assert!(
        closest <= 20.0,
        "the place came no closer than {closest:.1} arcseconds in 5 s"
    );
}

// US3-S6: a press during a push by hand is refused until Track has followed it for 10 s;
// a second rig that is not tracking refuses with its own words.
#[tokio::test(start_paused = true)]
async fn a_press_waits_out_a_push_by_hand_and_refuses_when_track_is_not_tracking() {
    let rig = track_rig(START).await;
    settle(&rig).await;
    rig.mount.push_by(0.0, -2.0);
    until(&rig, |view| view["track"]["push"]["state"] == "moving").await;
    refused(&rig.app, "up", "fine", "mount", SETTLING).await;
    let following = until(&rig, |view| view["track"]["push"]["state"] == "following").await;
    refused(&rig.app, "up", "fine", "mount", SETTLING).await;
    let since = following["track"]["push"]["at"].as_f64().unwrap();
    while rig.now() - since < 10.0 {
        rig.tick().await;
    }
    let (status, body) = nudge(&rig.app, "up", "fine", "mount").await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["moved"], "track", "{body}");

    let second = Rig::new(START, 50.0, 120.0, true).await;
    second.sync().await;
    let (status, body) = write(&second.app, "POST", "/api/track", json!({"on": true})).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["track"]["state"], "armed", "{body}");
    refused(&second.app, "up", "fine", "mount", NOT_TRACKING).await;
}

// US3-S7: while a step is made, and for 1 s plus Track's settle after it, the tracker's
// push check reads false; 3.5 s after the step ends it reads true.
#[tokio::test(start_paused = true)]
async fn the_push_check_reads_nothing_while_a_step_is_made_and_for_its_window() {
    let rig = track_rig(START).await;
    settle(&rig).await;
    let (status, body) = nudge(&rig.app, "left", "coarse", "mount").await;
    assert_eq!(status, 200, "{body}");
    let pressed = rig.now();
    loop {
        let view = rig.view().await;
        let pushed = checks_push(&rig);
        assert!(
            !pushed,
            "checks_push while the step is made at {:.1} s",
            rig.now() - pressed
        );
        if view["axis_move"]["state"] != "moving" {
            break;
        }
        assert!(rig.now() - pressed < 60.0, "the step never ended: {view}");
        rig.tick().await;
    }
    for _ in 0..25 {
        rig.tick().await;
    }
    assert!(!checks_push(&rig), "checks_push 2.5 s after the step ends");
    for _ in 0..10 {
        rig.tick().await;
    }
    assert!(checks_push(&rig), "checks_push 3.5 s after the step ends");
}
