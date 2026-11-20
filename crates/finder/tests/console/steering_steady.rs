//! US6: scripted tracker answers, worked exposure windows and tokio's clock.
//! Poll bounds are 0.25 s, response bounds 0.5 s, windows exact in Unix seconds.
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use finder::{
    imaging::{
        solver::{Region, SolveFrame},
        steering::{
            ExposureAsk, GapFuture, Go, Steered, Steering, Stepped, TrackingError, Verdict,
        },
    },
    session::FixedClock,
};
use tokio::{
    task::JoinHandle,
    time::{sleep, Instant},
};

const START: f64 = 1_768_528_800.0;
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
        bayer: Some("RGGB".into()),
        binning: 1,
        roi: ask().roi,
        start,
        exposure_s: 5.0,
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
enum Call {
    Gap,
    Release,
    Steer(TrackingError),
}
struct Script {
    steady: bool,
    between: bool,
    pause: Option<&'static str>,
    limit: Option<f64>,
    window: Option<(f64, f64)>,
    calls: Vec<Call>,
    claimed: bool,
}
struct Scripted {
    script: Mutex<Script>,
    delay: Duration,
}
impl Steered for Scripted {
    fn target(&self) -> Option<(f64, f64)> {
        Some((83.81867, -5.38967))
    }
    fn steer(&self, error: TrackingError) {
        self.script.lock().unwrap().calls.push(Call::Steer(error));
    }
    fn gap(&self) -> GapFuture {
        let mut s = self.script.lock().unwrap();
        s.calls.push(Call::Gap);
        s.claimed = true;
        let delay = self.delay;
        Box::pin(async move {
            sleep(delay).await;
            Stepped::default()
        })
    }
    fn release(&self) {
        let mut s = self.script.lock().unwrap();
        s.calls.push(Call::Release);
        s.claimed = false;
    }
    fn steered_by_subs(&self) -> bool {
        self.script.lock().unwrap().claimed
    }
    fn steady(&self) -> bool {
        self.script.lock().unwrap().steady
    }
    fn steady_between(&self, start: f64, end: f64) -> bool {
        let mut s = self.script.lock().unwrap();
        s.window = Some((start, end));
        s.between
    }
    fn pause(&self) -> Option<&'static str> {
        self.script.lock().unwrap().pause
    }
    fn longest_exposure_s(&self, _: f64) -> Option<f64> {
        self.script.lock().unwrap().limit
    }
    fn longest_exposure_ahead_s(&self, turn: f64) -> Option<f64> {
        self.longest_exposure_s(turn)
    }
}
type Cell = Arc<Mutex<Option<Arc<dyn Steered>>>>;
struct Rig {
    steering: Arc<Steering>,
    tracker: Arc<Scripted>,
    cell: Cell,
}
impl Rig {
    fn new(delay_ms: u64) -> Self {
        let tracker = Arc::new(Scripted {
            script: Mutex::new(Script {
                steady: true,
                between: true,
                pause: None,
                limit: None,
                window: None,
                calls: Vec::new(),
                claimed: false,
            }),
            delay: Duration::from_millis(delay_ms),
        });
        let cell: Cell = Arc::new(Mutex::new(Some(tracker.clone())));
        let steering = Steering::new(Arc::new(FixedClock(START)));
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
    fn steady(&self, steady: bool, pause: Option<&'static str>) {
        let mut s = self.tracker.script.lock().unwrap();
        s.steady = steady;
        s.pause = pause;
    }
    fn limit(&self, limit: f64) {
        self.tracker.script.lock().unwrap().limit = Some(limit);
    }
    fn calls(&self) -> Vec<Call> {
        self.tracker.script.lock().unwrap().calls.clone()
    }
    fn start(&self, ask: ExposureAsk) -> JoinHandle<(Go, Instant)> {
        let steering = self.steering.clone();
        tokio::spawn(async move { (steering.before_exposure(ask).await, Instant::now()) })
    }
    fn view(&self, steady: bool, waiting: bool, pause: Option<&str>, dropped: u64) {
        let view = self.steering.view().unwrap();
        assert_eq!(view.steady, steady);
        assert_eq!(view.waiting, waiting);
        assert_eq!(view.pause, pause);
        assert_eq!(view.dropped_not_steady, dropped);
        assert_eq!(self.steering.waiting(), waiting);
    }
}

// S3: no gap during a pause, and a prompt answer with the pause cleared.
#[tokio::test(start_paused = true)]
async fn wait_for_steady_passes_the_trackers_word_through() {
    let rig = Rig::new(100);
    rig.steady(false, Some("reversing"));
    let task = rig.start(ask());
    sleep(Duration::from_millis(10_100)).await;
    assert!(!task.is_finished());
    rig.view(false, true, Some("reversing"), 0);
    assert!(rig.calls().is_empty());
    let changed = Instant::now();
    rig.steady(true, None);
    sleep(Duration::from_secs(1)).await;
    assert!(task.is_finished());
    let (_, answered) = task.await.unwrap();
    assert!(answered - changed <= Duration::from_millis(500));
    assert_eq!(rig.calls(), [Call::Gap]);
    rig.view(true, false, None, 0);
}

// S4: records are selected by mid-exposure; a gone tracker cannot vouch for one.
#[tokio::test(start_paused = true)]
async fn check_uses_the_exposure_window_and_counts_one_stacks_drops() {
    let rig = Rig::new(0);
    rig.steering.before_exposure(ask()).await;
    rig.tracker.script.lock().unwrap().between = false;
    assert_eq!(
        rig.steering.check(&frame(START)),
        Verdict::Drop("not_steady")
    );
    rig.view(true, false, None, 1);
    assert_eq!(
        rig.tracker.script.lock().unwrap().window,
        Some((START, START + 5.0))
    );
    rig.tracker.script.lock().unwrap().between = true;
    assert_eq!(rig.steering.check(&frame(START)), Verdict::Keep);
    rig.view(true, false, None, 1);
    *rig.cell.lock().unwrap() = None;
    drop(rig.tracker);
    assert_eq!(
        rig.steering.check(&frame(START)),
        Verdict::Drop("not_steady")
    );

    let rig = Rig::new(0);
    rig.steering.before_exposure(ask()).await;
    rig.tracker.script.lock().unwrap().between = false;
    assert_eq!(
        rig.steering.check(&frame(START)),
        Verdict::Drop("not_steady")
    );
    rig.steering.release();
    rig.view(true, false, None, 1);
    let _hold = rig.steering.hold();
    rig.view(true, false, None, 0);

    let rig = Rig::new(0);
    *rig.cell.lock().unwrap() = None;
    rig.steering.before_exposure(ask()).await;
    assert_eq!(rig.steering.check(&frame(START)), Verdict::Keep);
    rig.view(true, false, None, 0);

    for off in [false, true] {
        let rig = Rig::new(0);
        if off {
            rig.steering.set_on(false);
        }
        rig.steering.before_exposure(ask()).await;
        rig.tracker.script.lock().unwrap().between = false;
        let start = if off { START } else { START - 0.01 };
        assert_eq!(
            rig.steering.check(&frame(start)),
            Verdict::Drop("not_steady")
        );
        rig.view(true, false, None, 1);
    }
}

// S6: views begin quiet, with or without a running tracker.
#[tokio::test(start_paused = true)]
async fn view_before_any_wait_is_steady() {
    let rig = Rig::new(0);
    rig.steering.set_smear_px(6.0).unwrap();
    rig.view(true, false, None, 0);
    rig.steering.before_exposure(ask()).await;
    rig.view(true, false, None, 0);
    let rig = Rig::new(0);
    *rig.cell.lock().unwrap() = None;
    rig.steering.before_exposure(ask()).await;
    rig.view(true, false, None, 0);
}

// S7: stopping a stack drops its wait, then the Hold clears the status.
#[tokio::test(start_paused = true)]
async fn dropping_the_hold_clears_an_abandoned_wait() {
    let rig = Rig::new(0);
    rig.steady(false, Some("reversing"));
    let hold = rig.steering.hold();
    let task = rig.start(ask());
    sleep(Duration::from_secs(2)).await;
    assert!(!task.is_finished());
    rig.view(false, true, Some("reversing"), 0);
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    drop(hold);
    rig.view(true, false, None, 0);
}

// S8: the switch is read where a gap would be asked, after the wait.
#[tokio::test(start_paused = true)]
async fn switch_off_still_waits_before_and_during_a_pause() {
    for off_before in [true, false] {
        let rig = Rig::new(0);
        rig.steady(false, Some("reversing"));
        if off_before {
            rig.steering.set_on(false);
        }
        let task = rig.start(ask());
        sleep(Duration::from_millis(500)).await;
        if !off_before {
            rig.steering.set_on(false);
        }
        sleep(Duration::from_millis(500)).await;
        assert!(!task.is_finished());
        rig.view(false, true, Some("reversing"), 0);
        rig.steady(true, None);
        sleep(Duration::from_millis(500)).await;
        assert!(task.is_finished());
        task.await.unwrap();
        assert!(!rig.calls().contains(&Call::Gap));
        rig.view(true, false, None, 0);
    }
}

// S9: re-read the source, rather than keeping the vanished tracker alive.
#[tokio::test(start_paused = true)]
async fn disappearing_source_ends_the_wait_within_one_poll() {
    let rig = Rig::new(0);
    rig.steady(false, None);
    let task = rig.start(ask());
    sleep(Duration::from_millis(1100)).await;
    assert!(!task.is_finished());
    let changed = Instant::now();
    *rig.cell.lock().unwrap() = None;
    sleep(Duration::from_millis(250)).await;
    assert!(task.is_finished());
    let (_, answered) = task.await.unwrap();
    assert!(answered - changed <= Duration::from_millis(250));
    rig.view(true, false, None, 0);
}

// S10: worked 60 -> 30 s cap; 14 is under half, 15 is exactly half.
#[tokio::test(start_paused = true)]
async fn limit_wait_releases_once_and_resumes_at_exactly_half() {
    let rig = Rig::new(0);
    rig.limit(37.15);
    let ask = ExposureAsk {
        wanted_s: 60.0,
        ..ask()
    };
    let go = rig.steering.before_exposure(ask).await;
    assert_eq!(go.exposure_s, 30.0);
    assert!(go.limited);
    rig.limit(14.0);
    let task = rig.start(ask);
    sleep(Duration::from_millis(1100)).await;
    assert!(!task.is_finished());
    rig.view(true, true, Some("limit"), 0);
    assert!(rig.steering.view().unwrap().over_limit);
    assert_eq!(rig.calls(), [Call::Gap, Call::Release]);
    let changed = Instant::now();
    rig.limit(15.0);
    sleep(Duration::from_millis(500)).await;
    assert!(task.is_finished());
    let (go, answered) = task.await.unwrap();
    assert!(answered - changed <= Duration::from_millis(500));
    assert_eq!(go.exposure_s, 30.0);
    assert!(go.limited);
    assert_eq!(rig.calls(), [Call::Gap, Call::Release, Call::Gap]);
    rig.view(true, false, None, 0);
}
#[tokio::test(start_paused = true)]
async fn first_cap_checks_the_limit_again_after_its_floor_is_decided() {
    let rig = Rig::new(0);
    rig.limit(0.4);
    let task = rig.start(ask());
    sleep(Duration::from_secs(1)).await;
    assert!(!task.is_finished());
    rig.view(true, true, Some("limit"), 0);
    assert_eq!(
        rig.calls().iter().filter(|c| **c == Call::Release).count(),
        1
    );
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
}

// S11: the view keeps reading the tracker during a long gap.
#[tokio::test(start_paused = true)]
async fn gap_wait_shows_the_trackers_new_pause_word() {
    let rig = Rig::new(5000);
    let task = rig.start(ask());
    sleep(Duration::from_millis(500)).await;
    rig.steady(false, Some("overhead"));
    sleep(Duration::from_millis(600)).await;
    assert!(!task.is_finished());
    rig.view(false, true, Some("overhead"), 0);
    assert_eq!(rig.calls(), [Call::Gap]);
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
}

// S12: the first gap ends unsteady; the second takes another second after
// the change at 5.5 s. Quarter-second polling puts the answer in 6.4..6.8 s.
#[tokio::test(start_paused = true)]
async fn gap_ending_unsteady_does_not_count() {
    let rig = Rig::new(1000);
    let called = Instant::now();
    let task = rig.start(ask());
    sleep(Duration::from_millis(500)).await;
    rig.steady(false, None);
    sleep(Duration::from_millis(3500)).await;
    assert!(!task.is_finished());
    sleep(Duration::from_millis(1500)).await;
    rig.steady(true, None);
    sleep(Duration::from_millis(1400)).await;
    assert!(task.is_finished());
    let (_, answered) = task.await.unwrap();
    assert!((6.4..=6.8).contains(&(answered - called).as_secs_f64()));
    assert_eq!(rig.calls(), [Call::Gap, Call::Gap]);
    rig.view(true, false, None, 0);
}
