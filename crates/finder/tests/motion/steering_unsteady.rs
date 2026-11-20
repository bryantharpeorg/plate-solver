//! US10: tracker edges on US6's unchanged implementation, in paused time.
//! Oracles: recorded mount rates (0.01 arcsec/s), simulator truth (0.15 arcsec
//! per sub), and the session's own measurement and push states. Steps are exact.
use std::time::Duration;

use crate::support::{goto, live::request, tracker::*, Scratch};
use finder::{
    imaging::steering::{Steered, Stepped, TrackingError},
    model::separation,
    session::Session,
    targets,
    tracking::Tracker,
};
use scope_node::mount::{Axis, Mount};
use serde_json::json;

const POLL: Duration = Duration::from_millis(10);

fn backlash_session(folder: &Scratch) -> Session {
    let path = folder.0.join("session.json");
    std::fs::write(
        &path,
        r#"{"site":null,"target":null,"min_alt_deg":15,"backlash":{"az_deg":0.3,"alt_deg":0.3}}"#,
    )
    .unwrap();
    Session::with_state_file(&path).unwrap()
}

fn transit() -> f64 {
    reversal(targets::find("M42").unwrap(), 1)
}

async fn backlash(folder: &Scratch) -> Rig {
    Rig::new(
        transit() - 120.0,
        targets::find("M42").unwrap().clone(),
        backlash_session(folder),
    )
    .await
}

// The ninth gap lapses the claim about 52 seconds before transit. No sub has
// supplied an error, and pointings resume only where a scenario asks for one.
async fn lapsed(folder: &Scratch) -> Rig {
    let rig = backlash(folder).await;
    rig.follow(transit() - 100.0, 60.0).await;
    rig.tracker.gap().await;
    assert!(rig.tracker.steered_by_subs());
    for _ in 0..8 {
        tokio::time::sleep(Duration::from_secs(6)).await;
        rig.tracker.gap().await;
    }
    assert!(!rig.tracker.steered_by_subs());
    rig
}

async fn until(rig: &Rig, time: f64) {
    if time > rig.now() {
        tokio::time::sleep(Duration::from_secs_f64(time - rig.now())).await;
    }
}

async fn wait_steady(rig: &Rig, wanted: bool, seconds: f64) {
    let deadline = rig.now() + seconds;
    while rig.tracker.steady() != wanted && rig.now() < deadline {
        tokio::time::sleep(POLL).await;
    }
    assert_eq!(
        rig.tracker.steady(),
        wanted,
        "at {}, pause {:?}",
        rig.now(),
        rig.tracker.pause()
    );
}

fn zero(step: Stepped) {
    assert_eq!((step.east_arcsec, step.north_arcsec), (0.0, 0.0));
}

fn steer(rig: &Rig, time: f64, east_arcsec: f64, north_arcsec: f64) {
    rig.tracker.steer(TrackingError {
        time,
        east_arcsec,
        north_arcsec,
    });
}

// pi_loop's independent finite difference: simulator horizon coordinates,
// Saemundsson refraction and the rig's known model, never the tracker's rates.
fn feed_forward(rig: &Rig, time: f64) -> [f64; 2] {
    let counts = rig.mount.axes().map(|a| a.counts_per_revolution);
    let positions = [time, time + 1.0].map(|t| {
        let (alt, az) = apparent(&rig.target, t);
        rig.model.inverse(az, alt, counts).unwrap()
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

fn azimuth_at(rig: &Rig, time: f64) -> (f64, f64) {
    let rates = rig.rates.lock().unwrap();
    let (t, _, rate) = rates
        .iter()
        .rev()
        .find(|(t, axis, _)| *t <= time && *axis == Axis::Azimuth)
        .expect("an azimuth rate in force");
    (*t, *rate)
}

fn assert_base(rig: &Rig, rate: (f64, f64), offset: f64) {
    let expected = feed_forward(rig, rate.0)[0] + offset;
    assert!(
        (rate.1 * 3600.0 - expected).abs() <= 0.01,
        "at {}: {} vs {expected} arcsec/s",
        rate.0,
        rate.1 * 3600.0
    );
}

async fn assert_gap_rates(rig: &Rig) {
    rig.tracker.gap().await;
    let answered = rig.now();
    let offset = rig.tracker.state().tracking_offset[0];
    assert_base(rig, azimuth_at(rig, answered), offset);
    let next = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let rate = rig
                .rates
                .lock()
                .unwrap()
                .iter()
                .find(|(t, axis, _)| *t > answered && *axis == Axis::Azimuth)
                .map(|(t, _, rate)| (*t, *rate));
            if let Some(rate) = rate {
                break rate;
            }
            tokio::time::sleep(POLL).await;
        }
    })
    .await
    .expect("first azimuth rate after the gap");
    assert_base(rig, next, offset);
}

// S1: the preceding pointing makes the proportional rate outlast settling.
#[tokio::test(start_paused = true)]
async fn lapsed_gap_after_take_up_restores_the_rate_at_its_answer_and_next_turn() {
    let folder = Scratch::new("unsteady-lapsed-rate");
    let rig = lapsed(&folder).await;
    rig.tracker.pointing(0.0, 0.0);
    wait_steady(&rig, false, 60.0).await;
    rig.tracker.pointing(30.0, 0.0);
    wait_steady(&rig, true, 10.0).await;
    assert_gap_rates(&rig).await;
    rig.tracker.stop("stopped").await;
}

// S2: the session publishes its first measurement only after it has closed.
#[tokio::test(start_paused = true)]
async fn measuring_turns_away_a_gap_and_closes_before_backlash_is_published() {
    let rig = Rig::new(
        transit() - 120.0,
        targets::find("M42").unwrap().clone(),
        Session::default(),
    )
    .await;
    rig.follow(transit() + 30.0, 60.0).await;
    assert!(!rig.tracker.steady());
    assert_eq!(rig.tracker.pause(), Some("measuring"));
    zero(rig.tracker.gap().await);
    assert!(!rig.tracker.steered_by_subs());
    let reader = async {
        let mut measured = false;
        while rig.now() < transit() + 600.0 {
            let present = rig.body().await["backlash"]["alt_deg"].is_number();
            let steady = rig.tracker.steady();
            if present {
                measured = true;
                assert!(
                    steady,
                    "published backlash while not steady at {}",
                    rig.now()
                );
            }
            tokio::time::sleep(STEP).await;
        }
        measured
    };
    let (_, measured) = tokio::join!(rig.follow(transit() + 600.0, 60.0), reader);
    assert!(measured, "the first reversal measurement must close");
    rig.tracker.stop("stopped").await;
}

// S3: rate receipts locate the exact two-second settling boundary. The
// pre-start and post-stop windows use the console clock, including 037's reach.
#[tokio::test(start_paused = true)]
async fn start_settles_for_two_seconds_and_stop_preserves_only_past_windows() {
    let folder = Scratch::new("unsteady-start-stop");
    let rig = backlash(&folder).await;
    let start = rig.now();
    assert!(rig.rates.lock().unwrap().is_empty());
    assert!(!rig.tracker.steady());
    assert_eq!(rig.tracker.pause(), Some("starting"));
    zero(rig.tracker.gap().await);
    assert!(!rig.tracker.steered_by_subs());
    until(&rig, start + 0.1).await;
    let first = {
        let rates = rig.rates.lock().unwrap();
        for axis in AXES {
            assert!(rates.iter().any(|(_, a, _)| *a == axis));
        }
        rates.first().unwrap().0
    };
    assert!(!rig.tracker.steady());
    until(&rig, first + 1.9).await;
    assert!(!rig.tracker.steady());
    until(&rig, first + 2.1).await;
    assert!(rig.tracker.steady());
    assert_eq!(rig.tracker.pause(), None);
    assert!(rig.tracker.steady_between(first + 2.0, first + 3.0));
    assert!(!rig.tracker.steady_between(start - 20.0, start - 10.0));
    until(&rig, start + 6.0).await;
    let now = rig.now();
    assert!(!rig.tracker.steady_between(now - 10.0, now));
    until(&rig, start + 14.0).await;
    let now = rig.now();
    assert!(rig.tracker.steady_between(now - 10.0, now));
    rig.follow(start + 20.0, 60.0).await;
    let end = rig.now();
    rig.tracker.stop("stopped").await;
    assert!(!rig.tracker.steady());
    assert!(!rig.tracker.steady_between(end - 3.0, end + 0.1));
    assert!(!rig.tracker.steady_between(end + 1.0, end + 2.0));
    assert!(rig.tracker.steady_between(end - 3.0, end - 1.0));
}

// S4: ask after settling, so zero proves the held step was forgotten rather
// than merely that a gap during settling was turned away.
#[tokio::test(start_paused = true)]
async fn take_up_forgets_the_step_held_by_a_lapsed_claim() {
    let folder = Scratch::new("unsteady-forget-held");
    let rig = lapsed(&folder).await;
    tokio::time::sleep(Duration::from_secs(30)).await;
    assert!(rig.tracker.steady());
    rig.tracker.pointing(30.0, 0.0);
    wait_steady(&rig, false, 30.0).await;
    wait_steady(&rig, true, 10.0).await;
    zero(rig.tracker.gap().await);
    rig.tracker.stop("stopped").await;
}

// S5: pending sub feedback is discarded together with the old claim.
#[tokio::test(start_paused = true)]
async fn take_up_forgets_pending_error_and_the_next_steady_gap_claims_again() {
    let folder = Scratch::new("unsteady-forget-pending");
    let rig = backlash(&folder).await;
    rig.follow(transit() - 60.0, 60.0).await;
    rig.tracker.gap().await;
    assert!(rig.tracker.steered_by_subs());
    tokio::time::sleep(Duration::from_secs(50)).await;
    steer(&rig, rig.now(), 12.0, -16.0);
    wait_steady(&rig, false, 20.0).await;
    assert!(!rig.tracker.steered_by_subs());
    wait_steady(&rig, true, 10.0).await;
    zero(rig.tracker.gap().await);
    assert!(rig.tracker.steered_by_subs());
    rig.tracker.stop("stopped").await;
}

// S6: call the interface used by the stack and 037, before and after reversal.
#[tokio::test(start_paused = true)]
async fn trait_reports_the_trackers_start_and_take_up_windows() {
    let folder = Scratch::new("unsteady-trait");
    let rig = backlash(&folder).await;
    let start = rig.now();
    let steered: &dyn Steered = &rig.tracker;
    assert!(!steered.steady());
    assert_eq!(steered.pause(), Some("starting"));
    rig.follow(transit() + 10.0, 60.0).await;
    assert!(steered.steady());
    assert_eq!(steered.pause(), None);
    assert!(!steered.steady_between(start - 20.0, start - 10.0));
    assert!(!steered.steady_between(transit() - 5.0, transit() + 5.0));
    assert!(steered.steady_between(start + 5.0, start + 20.0));
    rig.tracker.stop("stopped").await;
}

// S7: drive 015's own overhead pass with true pointings every two seconds.
// The minute search uses the simulator's geometric altitude, as keyhole.rs does.
#[tokio::test(start_paused = true)]
async fn keyhole_wait_is_not_steady_and_its_start_remains_in_the_history() {
    let folder = Scratch::new("unsteady-keyhole");
    let mut target = targets::find("M42").unwrap().clone();
    target.dec_deg = 39.0;
    let entry = (1..360)
        .map(|n| START + f64::from(n) * 60.0)
        .find(|t| {
            simulator::sky::horizontal(target.vector(), simulator::Site::default(), *t).0 >= 88.0
        })
        .expect("the Dec +39 degree target reaches 88 degrees");
    let rig = Rig::new(entry - 300.0, target, backlash_session(&folder)).await;
    let deadline = rig.now() + 1800.0;
    let mut next_pointing = rig.now();
    let mut first_wait = None;
    while rig.now() < deadline {
        if rig.now() >= next_pointing {
            let e = rig.truth_error();
            rig.tracker.pointing(e[0], e[1]);
            next_pointing += 2.0;
        }
        let state = rig.tracker.state().state;
        let steady = rig.tracker.steady();
        let pause = rig.tracker.pause();
        if state == "keyhole_wait" {
            first_wait.get_or_insert(rig.now());
            assert!(!steady);
            assert_eq!(pause, Some("overhead"));
        }
        if first_wait.is_some() && state == "tracking" && steady {
            break;
        }
        tokio::time::sleep(STEP).await;
    }
    let first_wait = first_wait.expect("the tracker entered keyhole_wait");
    assert_eq!(rig.tracker.state().state, "tracking");
    assert!(
        rig.tracker.steady(),
        "the pass must finish within 1800 seconds"
    );
    assert!(!rig
        .tracker
        .steady_between(first_wait - 2.0, first_wait + 2.0));
    rig.tracker.stop("stopped").await;
}

async fn player_wait(rig: &Rig, steps: usize, readings: &mut Vec<(f64, bool)>) {
    for _ in 0..steps {
        readings.push((rig.now(), rig.tracker.steady()));
        tokio::time::sleep(STEP).await;
    }
    readings.push((rig.now(), rig.tracker.steady()));
}

// S8: each error arrives a sub late. A sub crossing a pause supplies neither
// a drift observation nor feedback, and clears the error due on the next sub.
#[tokio::test(start_paused = true)]
async fn late_subs_resume_after_reversal_with_at_most_point_fifteen_arcsec_drift() {
    let folder = Scratch::new("unsteady-late-subs");
    let rig = backlash(&folder).await;
    rig.mount.set_rate_error(0.01);
    let mut readings = Vec::new();
    let mut previous = None;
    let mut kept = Vec::new();
    let mut first_sub_end = None;
    while rig.now() <= transit() + 120.0 {
        let deadline = rig.now() + 10.0;
        while !rig.tracker.steady() && rig.now() < deadline {
            player_wait(&rig, 1, &mut readings).await;
        }
        assert!(
            rig.tracker.steady(),
            "settling must end before the next sub"
        );
        readings.push((rig.now(), true));
        rig.tracker.gap().await;
        let start = rig.now();
        let e0 = rig.truth_error();
        player_wait(&rig, 50, &mut readings).await;
        let em = rig.truth_error();
        let measured = TrackingError {
            time: rig.now(),
            east_arcsec: em[0],
            north_arcsec: em[1],
        };
        player_wait(&rig, 50, &mut readings).await;
        let e1 = rig.truth_error();
        let end = rig.now();
        let keep = rig.tracker.steady_between(start, end);
        first_sub_end.get_or_insert(end);
        player_wait(&rig, 20, &mut readings).await;
        if keep {
            kept.push((start, (e1[0] - e0[0]).hypot(e1[1] - e0[1])));
            if let Some(error) = previous.replace(measured) {
                rig.tracker.steer(error);
            }
        } else {
            previous = None;
        }
    }
    let first_sub_end = first_sub_end.expect("at least one sub was played");
    assert!(readings
        .iter()
        .any(|(t, steady)| *t > first_sub_end && !steady));
    let last_false = readings.iter().rposition(|(_, steady)| !steady).unwrap();
    let resumed = readings[last_false + 1..]
        .iter()
        .find(|(_, steady)| *steady)
        .expect("a true reading after the final pause")
        .0;
    let after: Vec<_> = kept.iter().filter(|(start, _)| *start >= resumed).collect();
    assert!(
        after.len() >= 4,
        "only {} subs after the last pause",
        after.len()
    );
    for (index, (start, drift)) in after.iter().enumerate().skip(3) {
        assert!(
            *drift <= 0.15,
            "sub {} after the last pause, at {start}: {drift} arcsec drift",
            index + 1
        );
    }
    rig.tracker.stop("stopped").await;
}

// S9: 5.4 seconds is the worked interval that leaves a proportional rate on
// the mount after the tracker's own expiry, between the last turn and the gap.
#[tokio::test(start_paused = true)]
async fn lapsed_gap_removes_a_rate_that_expired_after_the_settling_turn() {
    let folder = Scratch::new("unsteady-expired-rate");
    let rig = lapsed(&folder).await;
    let first_pointing = transit() - 5.4;
    until(&rig, first_pointing).await;
    rig.tracker.pointing(0.0, 0.0);
    wait_steady(&rig, false, 10.0).await;
    let p = rig.now();
    rig.tracker.pointing(30.0, 0.0);
    let expires = p + (p - first_pointing) + 0.01;
    let deadline = p + 10.0;
    while (!rig.tracker.steady() || rig.now() < expires) && rig.now() < deadline {
        tokio::time::sleep(POLL).await;
    }
    assert!(rig.tracker.steady() && rig.now() >= expires);
    let before = azimuth_at(&rig, rig.now());
    let expected = feed_forward(&rig, rig.now())[0] + rig.tracker.state().tracking_offset[0];
    assert!(
        (before.1 * 3600.0 - expected).abs() > 1.0,
        "the case requires an expired rate still on the mount: {} vs {expected}",
        before.1 * 3600.0
    );
    rig.tracker.gap().await;
    let answered = rig.now();
    let offset = rig.tracker.state().tracking_offset[0];
    assert_base(&rig, azimuth_at(&rig, answered), offset);
    rig.tracker.stop("stopped").await;
}

// The tracking console of routes.rs, written here because its centred helper
// is private. Start Track directly on M42 without a GoTo or extra settling wait.
async fn tracking_console() -> goto::Rig {
    let rig = goto::Rig::new(START, 50.0, 120.0, true).await;
    let target = targets::find("M42").unwrap();
    rig.sim.point_eyepiece_at(target.ra_deg, target.dec_deg);
    rig.sync().await;
    assert_eq!(rig.view().await["mount"]["connected"], true);
    for _ in 0..10 {
        let pointing = rig.view().await["pointing"].clone();
        if separation(
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
    let (status, view) = request(&rig.app, "POST", "/api/track", json!({"on":true})).await;
    assert_eq!(status, 200, "{view}");
    assert_eq!(view["track"]["state"], "tracking", "{view}");
    assert!(
        view.get("goto").is_none(),
        "centred Track must not run GoTo"
    );
    assert_eq!(view["track"]["target"]["messier"], "M42");
    for _ in 0..100 {
        if rig.state.session().steered().unwrap().steady() {
            return rig;
        }
        rig.tick().await;
    }
    panic!("tracking did not become steady within 10 seconds");
}

struct PushReading {
    time: f64,
    steady: bool,
    pause: Option<&'static str>,
    subs: bool,
}

// Every session read is immediately paired with the tracker, with no await
// between them. None means the push key is absent, rather than a null state.
async fn wait_push(rig: &goto::Rig, tracker: &Tracker, wanted: Option<&str>) -> PushReading {
    let deadline = rig.now() + 120.0;
    loop {
        let view = rig.view().await;
        let reading = PushReading {
            time: rig.now(),
            steady: tracker.steady(),
            pause: tracker.pause(),
            subs: tracker.steered_by_subs(),
        };
        let matches = match wanted {
            Some(state) => view["track"]["push"]["state"] == state,
            None => view["track"].get("push").is_none(),
        };
        if matches {
            return reading;
        }
        assert!(rig.now() < deadline, "push never became {wanted:?}: {view}");
        rig.tick().await;
    }
}

// S10: 038's session view is the oracle for a hand moving the tube. Returning
// to the old place ends the pause without follow; leaving it there follows it.
#[tokio::test(start_paused = true)]
async fn hand_push_drops_the_claim_until_it_returns_or_is_followed() {
    let rig = tracking_console().await;
    let tracker = rig.state.session().tracker().unwrap();
    assert!(tracker.steady());
    tracker.gap().await;
    assert!(tracker.steered_by_subs());
    rig.mount.push_by(0.0, -0.1);
    let moving = wait_push(&rig, &tracker, Some("moving")).await;
    assert!(!moving.steady);
    assert_eq!(moving.pause, Some("pushed"));
    assert!(!moving.subs);
    zero(tracker.gap().await);
    assert!(!tracker.steered_by_subs());
    rig.mount.push_by(0.0, 0.1);
    let returned = wait_push(&rig, &tracker, None).await;
    assert!(returned.steady);
    tracker.gap().await;
    assert!(tracker.steered_by_subs());
    rig.mount.push_by(0.0, -0.1);
    let moving = wait_push(&rig, &tracker, Some("moving")).await;
    assert!(!moving.steady);
    let following = wait_push(&rig, &tracker, Some("following")).await;
    assert!(following.steady);
    assert!(!tracker.steady_between(moving.time, following.time));
    tracker.gap().await;
    assert!(tracker.steered_by_subs());
    tracker.stop("stopped").await;
}

// S11: wait beyond the gap's immediate turn to select a rate from the normal
// one-second loop. Stop the simulated mount directly, as a failed node command
// does, without telling the tracker. Its next rates must restart the axes.
#[tokio::test(start_paused = true)]
async fn stopped_axes_drop_the_claim_and_mark_only_the_interrupted_rate_window() {
    let folder = Scratch::new("unsteady-stopped-axes");
    let rig = backlash(&folder).await;
    rig.follow(transit() - 60.0, 60.0).await;
    rig.tracker.gap().await;
    assert!(rig.tracker.steered_by_subs());
    tokio::time::sleep(Duration::from_millis(2500)).await;
    let before = rig.rates.lock().unwrap().len();
    let r = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let rate = rig
                .rates
                .lock()
                .unwrap()
                .iter()
                .skip(before)
                .find(|(_, axis, _)| *axis == Axis::Azimuth)
                .map(|(t, _, rate)| (*t, *rate));
            if let Some((time, rate)) = rate {
                assert_ne!(rate, 0.0, "the interrupted rate must be nonzero");
                break time;
            }
            tokio::time::sleep(POLL).await;
        }
    })
    .await
    .expect("the loop sends an azimuth rate");
    tokio::time::sleep(Duration::from_millis(500)).await;
    rig.mount.stop_all().await;
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert!(!rig.tracker.steady_between(r + 0.1, r + 0.9));
    assert!(rig.tracker.steady_between(r - 5.0, r - 0.1));
    assert!(rig.tracker.steady_between(r + 1.1, r + 1.9));
    assert!(rig.tracker.steady());
    assert!(!rig.tracker.steered_by_subs());
    let restarted = rig
        .rates
        .lock()
        .unwrap()
        .iter()
        .find(|(t, axis, _)| *t > r && *axis == Axis::Azimuth)
        .copied()
        .expect("an azimuth rate after the interrupted one");
    assert_ne!(restarted.2, 0.0);
    rig.tracker.stop("stopped").await;
}

// S12: a quiet pointing clears the tracker's proportional state without
// sending a rate. The gap must clear the rate still running on the mount.
#[tokio::test(start_paused = true)]
async fn quiet_pointing_before_a_lapsed_gap_leaves_no_proportional_rate_running() {
    let folder = Scratch::new("unsteady-quiet-pointing");
    let rig = lapsed(&folder).await;
    rig.tracker.pointing(0.0, 0.0);
    wait_steady(&rig, false, 60.0).await;
    rig.tracker.pointing(30.0, 0.0);
    wait_steady(&rig, true, 10.0).await;
    rig.tracker.pointing(0.0, 0.0);
    assert_gap_rates(&rig).await;
    rig.tracker.stop("stopped").await;
}

// S13: FR-011/021 reject both an error handed over during the pause and one
// measured before it. Only a fresh error takes a lapsed tracker back.
#[tokio::test(start_paused = true)]
async fn errors_from_before_or_during_take_up_cannot_reclaim_a_lapsed_tracker() {
    let folder = Scratch::new("unsteady-stale-errors");
    let rig = lapsed(&folder).await;
    let before = rig.now();
    wait_steady(&rig, false, 60.0).await;
    steer(&rig, rig.now(), 12.0, -16.0);
    wait_steady(&rig, true, 10.0).await;
    steer(&rig, before, 12.0, -16.0);
    zero(rig.tracker.gap().await);
    assert!(!rig.tracker.steered_by_subs());
    steer(&rig, rig.now(), 0.0, 0.0);
    rig.tracker.gap().await;
    assert!(rig.tracker.steered_by_subs());
    rig.tracker.stop("stopped").await;
}

// S14: the gap is queued while take-up holds the rate task. Stop must answer
// it rather than strand the stack's before_exposure future indefinitely.
#[tokio::test(start_paused = true)]
async fn stop_answers_a_gap_queued_behind_take_up_with_no_step() {
    let folder = Scratch::new("unsteady-stop-queued-gap");
    let rig = backlash(&folder).await;
    rig.follow(transit() - 3.0, 60.0).await;
    wait_steady(&rig, false, 5.0).await;
    let asker =
        async { tokio::time::timeout(Duration::from_millis(1100), rig.tracker.gap()).await };
    let stopper = async {
        tokio::time::sleep(Duration::from_millis(100)).await;
        rig.tracker.stop("stopped").await;
    };
    let (answer, ()) = tokio::join!(asker, stopper);
    zero(answer.expect("Stop must answer every queued gap within 1.1 seconds"));
}
