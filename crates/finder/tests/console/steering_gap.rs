//! US4: worked sky errors (0.001 arcsec), scripted calls and virtual time are
//! independent oracles for FR-013 through FR-016. Views are read key by key.
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use finder::{
    imaging::{
        solver::{ImagingSolve, Region, SolveFrame},
        steering::{ExposureAsk, GapFuture, Steered, Steering, Stepped, TrackingError},
    },
    session::{Clock, FixedClock},
};
use serde_json::{json, Value};
use tokio::time::Instant;

use crate::support::{live, Scratch};

const START: f64 = 1_768_528_800.0;
const TARGET: (f64, f64) = (83.81867, -5.38967);
const CD: [[f64; 2]; 2] = [
    [-5.196152422706632e-4, -3.0e-4],
    [3.0e-4, -5.196152422706632e-4],
];

fn ask() -> ExposureAsk {
    ExposureAsk {
        wanted_s: 5.0,
        shortest_s: 0.001,
        sensor: (1236, 822),
        roi: Region {
            x: 0,
            y: 0,
            width: 1236,
            height: 822,
        },
        binning: 1,
    }
}
fn frame(start: f64) -> SolveFrame {
    SolveFrame {
        path: "unused.fits".into(),
        width: 1236,
        height: 822,
        bits: 16,
        bayer: None,
        binning: 1,
        roi: ask().roi,
        start,
        exposure_s: 5.0,
    }
}
fn solve() -> ImagingSolve {
    ImagingSolve::from_cd(83.8221, -5.3911, CD, 1236, 822)
}
fn displaced(east: f64) -> ImagingSolve {
    ImagingSolve::from_cd(
        TARGET.0 + east / (3600.0 * TARGET.1.to_radians().cos()),
        TARGET.1,
        CD,
        1236,
        822,
    )
}
fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{actual} vs {expected}"
    );
}
#[derive(Debug, Clone, PartialEq)]
enum Call {
    Gap,
    Steer(TrackingError),
    Release,
}

struct Scripted {
    clock: Arc<dyn Clock>,
    target: Option<(f64, f64)>,
    wait: Duration,
    log: Mutex<Vec<(f64, Call)>>,
    claimed: AtomicBool,
}
impl Scripted {
    fn new(clock: Arc<dyn Clock>, wait: Duration, target: Option<(f64, f64)>) -> Arc<Self> {
        Arc::new(Self {
            clock,
            target,
            wait,
            log: Mutex::default(),
            claimed: AtomicBool::new(false),
        })
    }
    fn record(&self, call: Call) {
        self.log.lock().unwrap().push((self.clock.now(), call));
    }
    fn calls(&self) -> Vec<Call> {
        self.log
            .lock()
            .unwrap()
            .iter()
            .map(|(_, c)| c.clone())
            .collect()
    }
    fn errors(&self) -> Vec<TrackingError> {
        self.calls()
            .into_iter()
            .filter_map(|c| match c {
                Call::Steer(e) => Some(e),
                _ => None,
            })
            .collect()
    }
}
impl Steered for Scripted {
    fn target(&self) -> Option<(f64, f64)> {
        self.target
    }
    fn steer(&self, error: TrackingError) {
        self.record(Call::Steer(error));
    }
    fn gap(&self) -> GapFuture {
        self.record(Call::Gap);
        self.claimed.store(true, Ordering::SeqCst);
        let wait = self.wait;
        Box::pin(async move {
            tokio::time::sleep(wait).await;
            Stepped::default()
        })
    }
    fn release(&self) {
        self.record(Call::Release);
        self.claimed.store(false, Ordering::SeqCst);
    }
    fn steered_by_subs(&self) -> bool {
        self.claimed.load(Ordering::SeqCst)
    }
}
type Cell = Arc<Mutex<Option<Arc<dyn Steered>>>>;
struct Rig {
    steering: Arc<Steering>,
    tracker: Arc<Scripted>,
    cell: Cell,
}
impl Rig {
    fn new(wait_ms: u64) -> Self {
        Self::with_clock(Arc::new(FixedClock(START)), wait_ms, Some(TARGET))
    }
    fn with_clock(clock: Arc<dyn Clock>, wait_ms: u64, target: Option<(f64, f64)>) -> Self {
        let steering = Steering::new(clock.clone());
        let tracker = Scripted::new(clock, Duration::from_millis(wait_ms), target);
        let cell: Cell = Arc::new(Mutex::new(Some(tracker.clone())));
        steering.set_source(Arc::new({
            let cell = cell.clone();
            move || cell.lock().unwrap().clone()
        }));
        Self {
            steering,
            tracker,
            cell,
        }
    }
    fn view(&self) -> Value {
        serde_json::to_value(self.steering.view().unwrap()).unwrap()
    }
}

// S1: the answer waits for the gap; US2-S1 supplies the independent sky error.
#[tokio::test(start_paused = true)]
async fn gap_precedes_exposure_and_accepted_error_is_at_its_middle() {
    let rig = Rig::new(700);
    assert!(rig.steering.view().is_none());
    let called = Instant::now();
    let go = rig.steering.before_exposure(ask()).await;
    assert_eq!(called.elapsed(), Duration::from_millis(700));
    assert_eq!(go.exposure_s, 5.0);
    assert!(!go.limited);
    assert_eq!(rig.tracker.calls(), [Call::Gap]);
    assert_eq!(rig.view()["source"], "subs");
    assert_eq!(rig.view()["subs"], 0);
    rig.steering.accepted(&frame(START), &solve());
    let errors = rig.tracker.errors();
    assert_eq!(errors.len(), 1);
    let e = errors[0];
    assert_eq!(e.time, START + 2.5);
    near(e.east_arcsec, 12.2934, 0.001);
    near(e.north_arcsec, -5.1480, 0.001);
    assert_eq!(rig.tracker.calls(), [Call::Gap, Call::Steer(e)]);
    assert_eq!(rig.view()["subs"], 1);
    assert_eq!(
        rig.view()["last"],
        json!({"time": e.time, "east_arcsec": e.east_arcsec, "north_arcsec": e.north_arcsec})
    );
    rig.steering.release();
    assert_eq!(
        rig.tracker.calls(),
        [Call::Gap, Call::Steer(e), Call::Release]
    );
    assert_eq!(rig.view()["source"], "tracker");
    assert_eq!(rig.view()["subs"], 0);
    assert!(rig.view().get("last").is_none());
}

// S2: no tracker means no wait and no error intake.
#[tokio::test(start_paused = true)]
async fn absent_tracker_answers_immediately_and_counts_nothing() {
    let rig = Rig::new(700);
    *rig.cell.lock().unwrap() = None;
    let called = Instant::now();
    let go = rig.steering.before_exposure(ask()).await;
    assert_eq!(called.elapsed(), Duration::ZERO);
    assert_eq!(go.exposure_s, 5.0);
    assert!(!go.limited);
    rig.steering.accepted(&frame(START), &solve());
    assert_eq!(rig.view()["source"], "none");
    assert_eq!(rig.view()["subs"], 0);
    assert!(rig.view().get("last").is_none());
    assert!(rig.tracker.calls().is_empty());
}

// S3: old frames and released exposures never steer; a cancelled first gap
// still releases, and mid-exposure selects the newest applicable record.
#[tokio::test(start_paused = true)]
async fn holds_release_even_a_cancelled_gap_and_records_match_by_middle() {
    let rig = Rig::new(0);
    rig.steering.before_exposure(ask()).await;
    rig.steering.accepted(&frame(START - 100.0), &solve());
    assert!(rig.tracker.errors().is_empty());
    assert_eq!(rig.view()["subs"], 0);
    assert_eq!(rig.view()["holding"], false);
    let hold = rig.steering.hold();
    assert_eq!(rig.view()["holding"], true);
    drop(hold);
    assert_eq!(rig.view()["holding"], false);
    assert_eq!(rig.tracker.calls(), [Call::Gap, Call::Release]);
    rig.steering.accepted(&frame(START), &solve());
    assert!(rig.tracker.errors().is_empty());

    let rig = Rig::new(700);
    let hold = rig.steering.hold();
    {
        let future = rig.steering.before_exposure(ask());
        tokio::pin!(future);
        tokio::select! {
            _ = &mut future => panic!("gap answered early"),
            _ = tokio::time::sleep(Duration::from_millis(100)) => {}
        }
    }
    drop(hold);
    assert_eq!(rig.tracker.calls(), [Call::Gap, Call::Release]);

    let rig = Rig::new(0);
    rig.steering.before_exposure(ask()).await;
    rig.steering.accepted(&frame(START - 0.01), &solve());
    assert_eq!(rig.tracker.errors().len(), 1);

    let rig = Rig::new(0);
    rig.steering.before_exposure(ask()).await;
    let second = Scripted::new(Arc::new(FixedClock(START)), Duration::ZERO, Some(TARGET));
    *rig.cell.lock().unwrap() = Some(second.clone());
    rig.steering.before_exposure(ask()).await;
    rig.steering.accepted(&frame(START), &solve());
    assert!(rig.tracker.errors().is_empty());
    assert_eq!(second.errors().len(), 1);
}

// S4: spec's matrix arithmetic for a named sensor point and a binned region.
#[tokio::test(start_paused = true)]
async fn set_point_is_translated_from_sensor_into_the_accepted_frame() {
    let rig = Rig::new(0);
    rig.steering.set_set_point(Some((668.0, 386.0)));
    rig.steering.before_exposure(ask()).await;
    rig.steering.accepted(&frame(START), &solve());
    let e = rig.tracker.errors()[0];
    near(e.east_arcsec, -54.2373, 0.001);
    near(e.north_arcsec, 95.6174, 0.001);

    let rig = Rig::new(0);
    rig.steering.before_exposure(ask()).await;
    let mut frame = frame(START);
    frame.width = 200;
    frame.height = 200;
    frame.roi = Region {
        x: 400,
        y: 200,
        width: 400,
        height: 400,
    };
    frame.binning = 2;
    let solve = ImagingSolve::from_cd(
        83.8221,
        -5.3911,
        CD.map(|row| row.map(|v| 2.0 * v)),
        200,
        200,
    );
    rig.steering.accepted(&frame, &solve);
    let e = rig.tracker.errors()[0];
    near(e.east_arcsec, -33.2577, 0.001);
    near(e.north_arcsec, -6.2848, 0.001);
}

// S7: FR-016, the same steering appears in imaging and session views.
#[tokio::test(start_paused = true)]
async fn imaging_and_session_show_steering_only_after_its_first_use() {
    let scratch = Scratch::new("steering-gap-view");
    let state = live::state(&live::sim(), None, false).with_imaging(&scratch.0);
    let app = finder::router(state.clone());
    let (_, view) = live::request(&app, "GET", "/api/imaging", Value::Null).await;
    assert!(view.get("steering").is_none());
    let (_, session) = live::request(&app, "GET", "/api/session", Value::Null).await;
    assert!(session["imaging"].get("steering").is_none());
    let steering = state.imaging().unwrap().steering();
    steering.set_source(Arc::new(|| None));
    steering.before_exposure(ask()).await;
    for path in ["/api/imaging", "/api/session"] {
        let (_, mut view) = live::request(&app, "GET", path, Value::Null).await;
        if path == "/api/session" {
            view = view["imaging"].clone();
        }
        assert_eq!(view["steering"]["source"], "none");
        assert_eq!(view["steering"]["subs"], 0);
    }
}

// S8: FR-014's 180 arcsec boundary applies only to the first error.
#[tokio::test(start_paused = true)]
async fn far_first_error_releases_and_stands_back_until_release() {
    let rig = Rig::new(0);
    rig.steering.before_exposure(ask()).await;
    rig.steering.accepted(&frame(START), &displaced(200.0));
    assert_eq!(rig.tracker.calls(), [Call::Gap, Call::Release]);
    near(rig.view()["far_arcsec"].as_f64().unwrap(), 200.0, 0.01);
    assert_eq!(rig.view()["source"], "tracker");
    assert_eq!(rig.view()["subs"], 0);
    assert!(rig.view().get("last").is_none());
    rig.steering.before_exposure(ask()).await;
    rig.steering.accepted(&frame(START), &solve());
    assert_eq!(rig.tracker.calls(), [Call::Gap, Call::Release]);
    rig.steering.release();
    rig.steering.before_exposure(ask()).await;
    assert_eq!(
        rig.tracker
            .calls()
            .iter()
            .filter(|c| **c == Call::Gap)
            .count(),
        2
    );
    assert!(rig.view().get("far_arcsec").is_none());
    rig.steering.accepted(&frame(START), &displaced(100.0));
    assert_eq!(rig.tracker.errors().len(), 1);
    assert!(rig.view().get("far_arcsec").is_none());
    rig.steering.before_exposure(ask()).await;
    rig.steering.accepted(&frame(START), &displaced(200.0));
    assert_eq!(rig.tracker.errors().len(), 2);
    assert!(rig.view().get("far_arcsec").is_none());
}

struct MovingClock(Instant);
impl Clock for MovingClock {
    fn now(&self) -> f64 {
        START + self.0.elapsed().as_secs_f64()
    }
}

// S9: FR-013 records the answer time, excluding a sub finished during the gap.
#[tokio::test(start_paused = true)]
async fn exposure_record_begins_at_the_answer_not_the_call() {
    let rig = Rig::with_clock(Arc::new(MovingClock(Instant::now())), 700, Some(TARGET));
    rig.steering.before_exposure(ask()).await;
    let mut early = frame(START + 0.1);
    early.exposure_s = 0.4;
    rig.steering.accepted(&early, &solve());
    assert!(rig.tracker.errors().is_empty());
    rig.steering.accepted(&frame(START + 0.7), &solve());
    assert_eq!(rig.tracker.errors().len(), 1);
}

// S10: FR-014, a tracker without a target receives and releases nothing.
#[tokio::test(start_paused = true)]
async fn tracker_without_target_is_neither_steered_nor_released() {
    let rig = Rig::with_clock(Arc::new(FixedClock(START)), 0, None);
    rig.steering.before_exposure(ask()).await;
    rig.steering.accepted(&frame(START), &solve());
    assert_eq!(rig.tracker.calls(), [Call::Gap]);
    assert_eq!(rig.view()["subs"], 0);
    assert!(rig.view().get("far_arcsec").is_none());
}

// S11: FR-013, remembered exposures and the last tracker own only Weak refs.
#[tokio::test(start_paused = true)]
async fn remembered_exposure_keeps_no_tracker_alive() {
    let rig = Rig::new(0);
    rig.steering.before_exposure(ask()).await;
    let weak = Arc::downgrade(&rig.tracker);
    *rig.cell.lock().unwrap() = None;
    drop(rig.tracker);
    assert!(weak.upgrade().is_none());
    rig.steering.accepted(&frame(START), &solve());
    assert_eq!(rig.steering.view().unwrap().subs, 0);
}

// FR-013 also forbids keeping a tracker alive while its gap is awaited.
#[tokio::test(start_paused = true)]
async fn waiting_for_gap_keeps_no_tracker_alive() {
    let rig = Rig::new(700);
    let weak = Arc::downgrade(&rig.tracker);
    let task = tokio::spawn({
        let steering = rig.steering.clone();
        async move { steering.before_exposure(ask()).await }
    });
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(rig.tracker.calls(), [Call::Gap]);
    *rig.cell.lock().unwrap() = None;
    drop(rig.tracker);
    assert!(weak.upgrade().is_none());
    assert_eq!(task.await.unwrap().exposure_s, 5.0);
    rig.steering.accepted(&frame(START), &solve());
    assert_eq!(rig.steering.view().unwrap().subs, 0);
}

// FR-013 keeps exactly the last 32 exposures. Distinct midpoints identify
// the evicted oldest record and the still available next-oldest record.
#[tokio::test(start_paused = true)]
async fn exposure_memory_retains_thirty_two_records() {
    let clock = Arc::new(MovingClock(Instant::now()));
    let rig = Rig::with_clock(clock.clone(), 0, Some(TARGET));
    let mut frames = Vec::new();
    for _ in 0..33 {
        rig.steering.before_exposure(ask()).await;
        let mut sub = frame(clock.now());
        sub.exposure_s = 0.2;
        frames.push(sub);
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    rig.steering.accepted(&frames[0], &solve());
    assert!(rig.tracker.errors().is_empty());
    rig.steering.accepted(&frames[1], &solve());
    assert_eq!(rig.tracker.errors().len(), 1);
    near(
        rig.tracker.errors()[0].time,
        frames[1].start + 0.1,
        0.000001,
    );
}

// S12: FR-014, a change ends the claim; None restores US2-S1's sensor centre.
#[tokio::test(start_paused = true)]
async fn changed_set_point_releases_once_and_none_restores_sensor_centre() {
    let rig = Rig::new(0);
    rig.steering.set_set_point(Some((668.0, 386.0)));
    rig.steering.before_exposure(ask()).await;
    rig.steering.accepted(&frame(START), &solve());
    assert_eq!(rig.tracker.calls().len(), 2);
    assert_eq!(rig.view()["subs"], 1);
    rig.steering.set_set_point(None);
    assert_eq!(rig.tracker.calls()[2], Call::Release);
    assert_eq!(rig.tracker.calls().len(), 3);
    assert_eq!(rig.view()["subs"], 0);
    assert!(rig.view().get("last").is_none());
    rig.steering.before_exposure(ask()).await;
    assert_eq!(rig.tracker.calls()[3], Call::Gap);
    rig.steering.accepted(&frame(START), &solve());
    let errors = rig.tracker.errors();
    assert_eq!(errors.len(), 2);
    near(errors[1].east_arcsec, 12.2934, 0.001);
    near(errors[1].north_arcsec, -5.1480, 0.001);
    rig.steering.set_set_point(None);
    assert_eq!(rig.tracker.calls().len(), 5);
}
