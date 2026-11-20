//! US6: the mount's rate log and configured 0.3 degree play are the oracles.
//! Windows use the console clock; sampling tolerance is 50 ms, play 30 arcsec.
use std::{cell::RefCell, time::Duration};

use crate::support::{tracker::*, Scratch};
use finder::{session::Session, targets};
use scope_node::mount::Axis;

#[derive(Clone, Copy)]
struct Reading {
    time: f64,
    steady: bool,
    pause: Option<&'static str>,
    subs: bool,
}
fn read(rig: &Rig) -> Reading {
    Reading {
        time: rig.now(),
        steady: rig.tracker.steady(),
        pause: rig.tracker.pause(),
        subs: rig.tracker.steered_by_subs(),
    }
}
async fn readings(rig: &Rig, until: f64, notes: &RefCell<Vec<Reading>>) {
    while rig.now() < until {
        notes.borrow_mut().push(read(rig));
        tokio::time::sleep(STEP).await;
    }
}
fn nearest(notes: &[Reading], time: f64) -> Reading {
    *notes
        .iter()
        .min_by(|a, b| (a.time - time).abs().total_cmp(&(b.time - time).abs()))
        .unwrap()
}

// S1: known play is taken up inside one unbroken pause; the queued gap is
// answered after the pulse and before the two seconds of settling finish.
#[tokio::test(start_paused = true)]
async fn take_up_pauses_drops_claim_and_turns_away_a_queued_gap() {
    let folder = Scratch::new("steady-takeup");
    let path = folder.0.join("session.json");
    std::fs::write(
        &path,
        r#"{"site":null,"target":null,"min_alt_deg":15,"backlash":{"az_deg":0.3,"alt_deg":0.3}}"#,
    )
    .unwrap();
    let target = targets::find("M42").unwrap().clone();
    let transit = reversal(&target, 1);
    let rig = Rig::new(
        transit - 120.0,
        target,
        Session::with_state_file(&path).unwrap(),
    )
    .await;
    rig.follow(transit - 60.0, 60.0).await;
    rig.tracker.gap().await;
    let notes = RefCell::new(Vec::new());
    let asker = async {
        while rig.now() < transit + 120.0 {
            if notes.borrow().iter().any(|r: &Reading| !r.steady) {
                let answer = rig.tracker.gap().await;
                return Some((rig.now(), answer));
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        None
    };
    let (_, _, answer) = tokio::join!(
        rig.follow(transit + 120.0, 60.0),
        readings(&rig, transit + 120.0, &notes),
        asker
    );
    let notes = notes.into_inner();
    let first = notes.iter().position(|r| !r.steady).unwrap();
    let last = notes.iter().rposition(|r| !r.steady).unwrap();
    let (f, l) = (notes[first].time, notes[last].time);
    assert!(notes[first..=last]
        .iter()
        .all(|r| !r.steady && r.pause == Some("reversing")));
    let before = nearest(&notes, f - 1.0);
    assert!((before.time - (f - 1.0)).abs() <= 0.05);
    assert_eq!(before.pause, None);
    assert!(before.subs);
    assert!(!notes[last].subs);
    let (pulse, zero) = {
        let rates = rig.rates.lock().unwrap();
        let pulse = rates
            .iter()
            .position(|(_, axis, rate)| *axis == Axis::Altitude && rate.abs() == 0.5)
            .unwrap();
        let zero = rates[pulse + 1..]
            .iter()
            .find(|(_, axis, rate)| *axis == Axis::Altitude && *rate == 0.0)
            .unwrap()
            .0;
        (rates[pulse].0, zero)
    };
    for time in [pulse, zero] {
        assert!((f - 0.05..=l + 0.05).contains(&time), "{f} {time} {l}");
    }
    for (start, end) in [(f - 20.0, f - 1.0), (l + 1.0, l + 20.0)] {
        assert!(rig.tracker.steady_between(start, end));
    }
    for (start, end) in [(f - 5.0, f + 0.1), (l - 0.1, l + 5.0), (f - 5.0, l + 5.0)] {
        assert!(!rig.tracker.steady_between(start, end));
    }
    let (answered, step) = answer.expect("gap asked during the take-up");
    assert_eq!((step.east_arcsec, step.north_arcsec), (0.0, 0.0));
    assert!(answered >= zero);
    assert!(l - answered >= 1.5, "gap {answered}, last pause {l}");
    rig.tracker.stop("stopped").await;
}

// S2: a fresh mount measures its first reversal, with the rate log locating it.
#[tokio::test(start_paused = true)]
async fn first_reversal_is_not_steady_while_measuring() {
    let target = targets::find("M42").unwrap().clone();
    let transit = reversal(&target, 1);
    let rig = Rig::new(transit - 120.0, target, Session::default()).await;
    let start = rig.now();
    let notes = RefCell::new(Vec::new());
    tokio::join!(
        rig.follow(transit + 120.0, 60.0),
        readings(&rig, transit + 120.0, &notes)
    );
    let zero = rig
        .rates
        .lock()
        .unwrap()
        .iter()
        .find(|(_, axis, rate)| *axis == Axis::Altitude && *rate == 0.0)
        .unwrap()
        .0;
    let notes = notes.into_inner();
    let before: Vec<_> = notes
        .iter()
        .filter(|r| r.time >= start + 3.5 && r.time <= zero - 0.1)
        .collect();
    assert!(!before.is_empty());
    assert!(before.iter().all(|r| r.steady));
    let after = nearest(&notes, zero + 5.0);
    assert!((after.time - (zero + 5.0)).abs() <= 0.05);
    assert!(!after.steady);
    assert_eq!(after.pause, Some("measuring"));
    assert!(!rig.tracker.steady_between(zero + 0.1, zero + 10.0));
    rig.tracker.stop("stopped").await;
}

// S5: a lapsed claim must not hold the corrections that close a measurement.
#[tokio::test(start_paused = true)]
async fn lapsed_subs_allow_the_first_reversal_measurement_to_close() {
    let target = targets::find("M42").unwrap().clone();
    let transit = reversal(&target, 1);
    let rig = Rig::new(transit - 600.0, target, Session::default()).await;
    for _ in 0..500 {
        if rig.tracker.steady() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(rig.tracker.steady());
    rig.tracker.gap().await;
    for _ in 0..8 {
        tokio::time::sleep(Duration::from_secs(6)).await;
        rig.tracker.gap().await;
    }
    assert!(!rig.tracker.steered_by_subs());
    let asker = async {
        let mut notes = Vec::new();
        while rig.now() < transit + 600.0 {
            tokio::time::sleep(Duration::from_secs(6)).await;
            rig.tracker.gap().await;
            notes.push(read(&rig));
        }
        notes
    };
    let (_, notes) = tokio::join!(rig.follow(transit + 600.0, 60.0), asker);
    let play = rig.body().await["backlash"]["alt_deg"].as_f64().unwrap();
    assert!((play - 0.3).abs() * 3600.0 <= 30.0, "measured {play}");
    assert!(notes.iter().any(|r| !r.steady));
    assert!(notes.last().unwrap().steady);
    assert!(notes.iter().all(|r| !r.subs));
    assert!(rig.tracker.steady());
    rig.tracker.stop("stopped").await;
}
