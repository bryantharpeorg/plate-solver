//! Tracking steered by the subs (036): the tracking error a sub's imaging solve
//! gives and the one interface between the stack and a tracker. Sky terms only.

use std::{
    collections::VecDeque,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex, Weak},
    time::Duration,
};

use serde::Serialize;
use tokio::time::sleep;

use crate::session::Clock;

use super::solver::{ImagingSolve, Region, SolveFrame};

/// How far the sky at the set point is from the target.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct TrackingError {
    /// Console Unix seconds: the sub's mid-exposure.
    pub time: f64,
    /// East (RA times cos Dec), in arcseconds.
    pub east_arcsec: f64,
    /// North, in arcseconds.
    pub north_arcsec: f64,
}

/// The sky at `set_point`, in the frame's pixels, less `target`, in J2000 degrees.
/// `None` when the target has no pixel in the solve or a number is not finite.
pub fn tracking_error(
    solve: &ImagingSolve,
    set_point: (f64, f64),
    target: (f64, f64),
    time: f64,
) -> Option<TrackingError> {
    if ![time, set_point.0, set_point.1, target.0, target.1]
        .into_iter()
        .all(f64::is_finite)
    {
        return None;
    }
    let pixel = solve.pixel_of(target.0, target.1)?;
    let (dx, dy) = (set_point.0 - pixel.0, set_point.1 - pixel.1);
    let cd = solve.cd_deg;
    let east_arcsec = 3600.0 * (cd[0][0] * dx + cd[0][1] * dy);
    let north_arcsec = 3600.0 * (cd[1][0] * dx + cd[1][1] * dy);
    (east_arcsec.is_finite() && north_arcsec.is_finite()).then_some(TrackingError {
        time,
        east_arcsec,
        north_arcsec,
    })
}

/// A set point given in sensor pixels, or the sensor's centre for `None`, in the
/// pixels of a frame that covers `roi` at `binning`.
pub fn set_point_in_frame(
    sensor: (u32, u32),
    set_point: Option<(f64, f64)>,
    roi: Region,
    binning: u32,
) -> (f64, f64) {
    let (sx, sy) = set_point.unwrap_or((f64::from(sensor.0) / 2.0, f64::from(sensor.1) / 2.0));
    (
        (sx - f64::from(roi.x)) / f64::from(binning),
        (sy - f64::from(roi.y)) / f64::from(binning),
    )
}

/// What a gap moved the telescope by, on the sky, in arcseconds.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Stepped {
    pub east_arcsec: f64,
    pub north_arcsec: f64,
}

/// A gap's answer. It owns only the answer's receiver and borrows nothing of the
/// tracker, so awaiting it keeps no tracker alive.
pub type GapFuture = Pin<Box<dyn Future<Output = Stepped> + Send>>;

/// The one interface between the stack and whatever tracker is running.
pub trait Steered: Send + Sync {
    /// The J2000 place, in degrees, the tracker keeps at the set point.
    fn target(&self) -> Option<(f64, f64)>;
    /// Hand over a tracking error. It changes nothing until the next gap.
    fn steer(&self, error: TrackingError);
    /// The stack is between two exposures. The tracker makes its correction now
    /// and answers once its rates are acknowledged. The gap is asked for when
    /// this is called, not when the future is first polled.
    fn gap(&self) -> GapFuture;
    /// The stack no longer steers.
    fn release(&self);
    /// Whether tracking errors, rather than its own corrections, steer it now.
    fn steered_by_subs(&self) -> bool;
    /// The longest exposure allowed for `turn_rad`, in seconds; `None` when
    /// nothing limits it.
    fn longest_exposure_s(&self, _turn_rad: f64) -> Option<f64> {
        None
    }
    /// The same for an exposure decided now and held: the shortest that answer
    /// will be over the hour ahead.
    fn longest_exposure_ahead_s(&self, _turn_rad: f64) -> Option<f64> {
        None
    }
    /// Whether it is steady enough to expose.
    fn steady(&self) -> bool {
        true
    }
    /// Whether it was steady for all of a window of console Unix seconds.
    fn steady_between(&self, _start: f64, _end: f64) -> bool {
        true
    }
    /// Why it is not steady, in one word of its own for the page; `None` while
    /// it is steady or has nothing to say.
    fn pause(&self) -> Option<&'static str> {
        None
    }
    /// The share of a tracking error to take out as a step, and of a drift rate
    /// to take out of the rate, each from 0 to 1. A tracker with no such law
    /// ignores them.
    fn tune(&self, _step_share: f64, _rate_share: f64) {}
}

/// Where a `Steering` finds the running tracker.
pub type Source = Arc<dyn Fn() -> Option<Arc<dyn Steered>> + Send + Sync>;

/// How many of the last exposures the steering remembers.
pub const EXPOSURES_KEPT: usize = 32;
/// A stack whose first tracking error exceeds this many arcseconds is not steered.
pub const FAR_ARCSEC: f64 = 180.0;
/// An exposure is not halved to under this many seconds.
pub const HALVING_FLOOR_S: f64 = 1.0;
const POLL: Duration = Duration::from_millis(250);
/// A sub whose tracker cannot vouch for its whole exposure is dropped with this reason.
pub const NOT_STEADY: &str = "not_steady";

/// Whether a sub can proceed to its solve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Keep,
    Drop(&'static str),
}
/// The shares handed to the tracker until set.
pub const STEP_SHARE: f64 = 0.5;
pub const RATE_SHARE: f64 = 0.3;

pub(crate) fn smear_checked(px: f64) -> Result<f64, &'static str> {
    if px.is_finite() && (0.1..=20.0).contains(&px) {
        Ok(px)
    } else {
        Err("the corner smear must be a number from 0.1 to 20")
    }
}

pub(crate) fn step_share_checked(share: f64) -> Result<f64, &'static str> {
    if share.is_finite() && (0.0..=1.0).contains(&share) {
        Ok(share)
    } else {
        Err("the step share must be a number from 0 to 1")
    }
}

pub(crate) fn rate_share_checked(share: f64) -> Result<f64, &'static str> {
    if share.is_finite() && (0.0..=1.0).contains(&share) {
        Ok(share)
    } else {
        Err("the rate share must be a number from 0 to 1")
    }
}

fn turn_for(ask: &ExposureAsk, set_point: Option<(f64, f64)>, smear_px: f64) -> f64 {
    let set = set_point.unwrap_or((f64::from(ask.sensor.0) / 2.0, f64::from(ask.sensor.1) / 2.0));
    let x = f64::from(ask.roi.x);
    let y = f64::from(ask.roi.y);
    let right = x + f64::from(ask.roi.width);
    let bottom = y + f64::from(ask.roi.height);
    let radius = [(x, y), (right, y), (x, bottom), (right, bottom)]
        .into_iter()
        .map(|(cx, cy)| (cx - set.0).hypot(cy - set.1))
        .fold(0.0, f64::max)
        / f64::from(ask.binning);
    smear_px / radius
}

fn decide(ask: &ExposureAsk, ahead: Option<f64>) -> (f64, bool) {
    let floor = ask.shortest_s.max(HALVING_FLOOR_S);
    let mut exposure = ask.wanted_s;
    if let Some(limit) = ahead {
        while exposure > limit && exposure / 2.0 >= floor {
            exposure /= 2.0;
        }
    }
    (exposure, exposure < ask.wanted_s)
}

/// The exposure the stack is about to take.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExposureAsk {
    /// The exposure the observer set, in seconds.
    pub wanted_s: f64,
    /// The camera's shortest exposure, in seconds.
    pub shortest_s: f64,
    pub sensor: (u32, u32),
    pub roi: Region,
    pub binning: u32,
}

/// The exposure to take, and whether it is shorter than asked.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Go {
    pub exposure_s: f64,
    pub limited: bool,
}

/// Releases the tracker when dropped.
pub struct Hold {
    steering: Arc<Steering>,
}

impl Drop for Hold {
    fn drop(&mut self) {
        self.steering.state.lock().expect("steering state").holds -= 1;
        self.steering.release();
    }
}

struct Taken {
    go_time: f64,
    tracker: Option<Weak<dyn Steered>>,
    sensor: (u32, u32),
    stepped: Stepped,
}

struct ReadoutError {
    error: TrackingError,
    arcsec_per_px: f64,
    drift: f64,
}

#[derive(Default)]
struct State {
    used: bool,
    source: Option<Source>,
    set_point: Option<(f64, f64)>,
    tracker: Option<Weak<dyn Steered>>,
    holds: usize,
    taken: VecDeque<Taken>,
    errors: VecDeque<ReadoutError>,
    subs: u64,
    last: Option<ErrorView>,
    far: Option<f64>,
    smear_px: f64,
    on: bool,
    step_share: f64,
    rate_share: f64,
    held: Option<(f64, bool)>,
    asked: Option<ExposureAsk>,
    steady: bool,
    waiting: bool,
    pause: Option<&'static str>,
    dropped_not_steady: u64,
}

impl State {
    fn remember_error(&mut self, error: TrackingError, arcsec_per_px: f64, exposure_s: f64) {
        let drift = self.errors.back().map_or(0.0, |previous| {
            let elapsed = error.time - previous.error.time;
            if elapsed <= 0.0 {
                return 0.0;
            }
            let step = self
                .taken
                .iter()
                .filter(|record| {
                    record.go_time > previous.error.time && record.go_time <= error.time
                })
                .fold(Stepped::default(), |sum, record| Stepped {
                    east_arcsec: sum.east_arcsec + record.stepped.east_arcsec,
                    north_arcsec: sum.north_arcsec + record.stepped.north_arcsec,
                });
            (error.east_arcsec - previous.error.east_arcsec - step.east_arcsec)
                .hypot(error.north_arcsec - previous.error.north_arcsec - step.north_arcsec)
                * exposure_s
                / elapsed
        });
        self.errors.push_back(ReadoutError {
            error,
            arcsec_per_px,
            drift,
        });
        if self.errors.len() > 10 {
            self.errors.pop_front();
        }
    }

    fn readout(&self) -> Option<(f64, f64, f64)> {
        let n = self.errors.len();
        if n < 2 {
            return None;
        }
        let error = (self
            .errors
            .iter()
            .map(|entry| entry.error.east_arcsec.powi(2) + entry.error.north_arcsec.powi(2))
            .sum::<f64>()
            / n as f64)
            .sqrt();
        // The window's first error has no predecessor inside the window.
        let drift = (self
            .errors
            .iter()
            .skip(1)
            .map(|entry| entry.drift.powi(2))
            .sum::<f64>()
            / (n - 1) as f64)
            .sqrt();
        Some((error, drift, self.errors.back()?.arcsec_per_px))
    }

    fn remember(&mut self, taken: Taken) {
        self.taken.push_back(taken);
        if self.taken.len() > EXPOSURES_KEPT {
            self.taken.pop_front();
        }
    }
}

/// The console's steering between the stack and the running tracker.
/// Its state lock is a leaf: calls to the source or tracker always happen outside it.
pub struct Steering {
    clock: Arc<dyn Clock>,
    state: Mutex<State>,
}

impl Steering {
    pub fn new(clock: Arc<dyn Clock>) -> Arc<Self> {
        Arc::new(Self {
            clock,
            state: Mutex::new(State {
                smear_px: 6.0,
                on: true,
                steady: true,
                step_share: STEP_SHARE,
                rate_share: RATE_SHARE,
                ..State::default()
            }),
        })
    }

    pub fn set_source(&self, source: Source) {
        self.state.lock().expect("steering state").source = Some(source);
    }

    /// For the life of a stack's taker.
    pub fn hold(self: &Arc<Self>) -> Hold {
        {
            let mut state = self.state.lock().expect("steering state");
            state.holds += 1;
            state.dropped_not_steady = 0;
        }
        Hold {
            steering: self.clone(),
        }
    }

    /// In the gap, before every exposure. Remember only when the answer is ready.
    pub async fn before_exposure(&self, ask: ExposureAsk) -> Go {
        {
            let mut state = self.state.lock().expect("steering state");
            state.used = true;
            state.asked = Some(ask);
        }
        let mut released_for_limit = false;
        let mut stepped = Stepped::default();
        loop {
            let source = self.state.lock().expect("steering state").source.clone();
            let mut tracker = source.and_then(|source| source());
            let weak = tracker.as_ref().map(Arc::downgrade);
            self.state.lock().expect("steering state").tracker = weak.clone();
            if !self.read_steady(tracker.as_deref()) {
                drop(tracker);
                sleep(POLL).await;
                continue;
            }
            if tracker
                .as_ref()
                .is_some_and(|t| self.under_half(t.as_ref(), &ask))
            {
                if !released_for_limit {
                    self.let_go();
                    released_for_limit = true;
                }
                self.limit_wait();
                drop(tracker);
                sleep(POLL).await;
                continue;
            }
            released_for_limit = false;
            // Read the switch here: it may have changed during the wait.
            let (feedback, step_share, rate_share) = {
                let state = self.state.lock().expect("steering state");
                (
                    state.on && state.far.is_none(),
                    state.step_share,
                    state.rate_share,
                )
            };
            if let Some(t) = tracker.as_ref().filter(|_| feedback) {
                t.tune(step_share, rate_share);
                let mut answer = t.gap();
                drop(tracker);
                loop {
                    tokio::select! {
                        step = &mut answer => {
                            stepped.east_arcsec += step.east_arcsec;
                            stepped.north_arcsec += step.north_arcsec;
                            break;
                        },
                        _ = sleep(POLL) => {
                            let current = weak.as_ref().and_then(Weak::upgrade);
                            self.read_steady(current.as_deref());
                        }
                    }
                }
                // The answer owns no tracker; it may have gone while awaited.
                tracker = weak.as_ref().and_then(Weak::upgrade);
                if !self.read_steady(tracker.as_deref()) {
                    continue;
                }
            }
            let (held, set_point, smear_px) = {
                let state = self.state.lock().expect("steering state");
                (state.held, state.set_point, state.smear_px)
            };
            let decided = held.unwrap_or_else(|| {
                let ahead = tracker
                    .as_ref()
                    .and_then(|t| t.longest_exposure_ahead_s(turn_for(&ask, set_point, smear_px)));
                decide(&ask, ahead)
            });
            self.state
                .lock()
                .expect("steering state")
                .held
                .get_or_insert(decided);
            // The first cap may reach its floor while still over twice the limit.
            if tracker
                .as_ref()
                .is_some_and(|t| self.under_half(t.as_ref(), &ask))
            {
                self.let_go();
                released_for_limit = true;
                self.limit_wait();
                drop(tracker);
                sleep(POLL).await;
                continue;
            }
            let go_time = self.clock.now();
            let mut state = self.state.lock().expect("steering state");
            let (exposure_s, limited) = state.held.unwrap();
            state.steady = true;
            state.waiting = false;
            state.pause = None;
            state.remember(Taken {
                go_time,
                tracker: tracker.as_ref().map(Arc::downgrade),
                sensor: ask.sensor,
                stepped,
            });
            return Go {
                exposure_s,
                limited,
            };
        }
    }

    // Read the tracker outside the leaf lock, then publish its answer.
    fn read_steady(&self, tracker: Option<&dyn Steered>) -> bool {
        let steady = tracker.is_none_or(Steered::steady);
        let pause = if steady {
            None
        } else {
            tracker.and_then(Steered::pause)
        };
        let mut state = self.state.lock().expect("steering state");
        state.steady = steady;
        state.waiting = !steady;
        state.pause = pause;
        steady
    }

    fn under_half(&self, tracker: &dyn Steered, ask: &ExposureAsk) -> bool {
        let (held, set_point, smear_px) = {
            let state = self.state.lock().expect("steering state");
            (state.held, state.set_point, state.smear_px)
        };
        held.is_some_and(|(exposure, _)| {
            tracker
                .longest_exposure_s(turn_for(ask, set_point, smear_px))
                .is_some_and(|limit| limit < exposure / 2.0)
        })
    }

    fn limit_wait(&self) {
        let mut state = self.state.lock().expect("steering state");
        state.waiting = true;
        state.pause = Some("limit");
    }

    /// Whether a call before an exposure is waiting for the tracker or its limit.
    pub fn waiting(&self) -> bool {
        self.state.lock().expect("steering state").waiting
    }

    /// For every sub, before it is solved: whether its tracker was steady for it.
    pub fn check(&self, frame: &SolveFrame) -> Verdict {
        let time = frame.start + frame.exposure_s / 2.0;
        let weak = {
            let state = self.state.lock().expect("steering state");
            state
                .taken
                .iter()
                .rev()
                .find(|record| record.go_time <= time)
                .and_then(|record| record.tracker.clone())
        };
        let Some(weak) = weak else {
            return Verdict::Keep;
        };
        if weak.upgrade().is_none_or(|tracker| {
            !tracker.steady_between(frame.start, frame.start + frame.exposure_s)
        }) {
            self.state
                .lock()
                .expect("steering state")
                .dropped_not_steady += 1;
            Verdict::Drop(NOT_STEADY)
        } else {
            Verdict::Keep
        }
    }

    /// Hand over the tracking error of a sub that the stack accepted.
    pub fn accepted(&self, frame: &SolveFrame, solve: &ImagingSolve) {
        let time = frame.start + frame.exposure_s / 2.0;
        let (weak, sensor, set_point) = {
            let state = self.state.lock().expect("steering state");
            if !state.on || state.far.is_some() {
                return;
            }
            let Some(record) = state
                .taken
                .iter()
                .rev()
                .find(|record| record.go_time <= time)
            else {
                return;
            };
            (record.tracker.clone(), record.sensor, state.set_point)
        };
        let Some(tracker) = weak.and_then(|weak| weak.upgrade()) else {
            return;
        };
        let Some(target) = tracker.target() else {
            return;
        };
        let set = set_point_in_frame(sensor, set_point, frame.roi, frame.binning);
        let Some(error) = tracking_error(solve, set, target, time) else {
            return;
        };
        let far = {
            let mut state = self.state.lock().expect("steering state");
            if !state.on || state.far.is_some() {
                return;
            }
            let length = error.east_arcsec.hypot(error.north_arcsec);
            if state.subs == 0 && length > FAR_ARCSEC {
                state.far = Some(length);
                true
            } else {
                false
            }
        };
        if far {
            tracker.release();
        } else {
            tracker.steer(error);
            let mut state = self.state.lock().expect("steering state");
            state.remember_error(error, solve.arcsec_per_px, frame.exposure_s);
            state.subs += 1;
            state.last = Some(ErrorView {
                time: error.time,
                east_arcsec: error.east_arcsec,
                north_arcsec: error.north_arcsec,
            });
        }
    }

    /// End this stack's claim and forget its exposures.
    pub fn release(&self) {
        let tracker = {
            let mut state = self.state.lock().expect("steering state");
            state.taken.clear();
            state.subs = 0;
            state.last = None;
            state.errors.clear();
            state.far = None;
            state.held = None;
            state.asked = None;
            state.steady = true;
            state.waiting = false;
            state.pause = None;
            state.tracker.take()
        };
        if let Some(tracker) = tracker.and_then(|weak| weak.upgrade()) {
            tracker.release();
        }
    }

    // End the claim while keeping the stack's exposure records and last tracker.
    fn let_go(&self) {
        let tracker = {
            let mut state = self.state.lock().expect("steering state");
            state.subs = 0;
            state.last = None;
            state.errors.clear();
            state.tracker.clone()
        };
        if let Some(tracker) = tracker.and_then(|weak| weak.upgrade()) {
            tracker.release();
        }
    }

    /// The set point in sensor pixels; `None` is the sensor's centre.
    pub fn set_set_point(&self, sensor_px: Option<(f64, f64)>) {
        let changed = {
            let mut state = self.state.lock().expect("steering state");
            let changed = state.set_point != sensor_px;
            state.set_point = sensor_px;
            changed
        };
        if changed {
            self.let_go();
        }
    }

    /// Set the allowed corner smear in delivered pixels.
    pub fn set_smear_px(&self, px: f64) -> Result<(), String> {
        let px = smear_checked(px)?;
        let mut state = self.state.lock().expect("steering state");
        state.smear_px = px;
        state.used = true;
        Ok(())
    }

    /// Turn feedback on or let the tracker go at once; the cap stays in force.
    pub fn set_on(&self, on: bool) {
        {
            let mut state = self.state.lock().expect("steering state");
            state.on = on;
            state.used = true;
        }
        if !on {
            self.let_go();
        }
    }

    pub fn set_step_share(&self, share: f64) -> Result<(), String> {
        let share = step_share_checked(share)?;
        let mut state = self.state.lock().expect("steering state");
        state.step_share = share;
        state.used = true;
        Ok(())
    }

    pub fn set_rate_share(&self, share: f64) -> Result<(), String> {
        let share = rate_share_checked(share)?;
        let mut state = self.state.lock().expect("steering state");
        state.rate_share = share;
        state.used = true;
        Ok(())
    }

    /// `None` until the steering is first used.
    pub fn view(&self) -> Option<SteeringView> {
        self.view_for(None)
    }

    /// Also show what a stack started with `ask` would take while a tracker runs.
    /// Making a view marks nothing used.
    pub fn view_for(&self, ask: Option<ExposureAsk>) -> Option<SteeringView> {
        let (source, used, asked, set_point, mut view) = {
            let state = self.state.lock().expect("steering state");
            let readout = state.readout();
            (
                state.source.clone(),
                state.used,
                state.asked,
                state.set_point,
                SteeringView {
                    source: "none",
                    subs: state.subs,
                    last: state.last.clone(),
                    far_arcsec: state.far,
                    holding: state.holds > 0,
                    smear_px: state.smear_px,
                    on: state.on,
                    step_share: state.step_share,
                    rate_share: state.rate_share,
                    exposure_s: state.held.map(|held| held.0),
                    limited: state.held.is_some_and(|held| held.1),
                    longest_exposure_s: None,
                    over_limit: false,
                    would_take_s: None,
                    steady: state.steady,
                    waiting: state.waiting,
                    pause: state.pause.filter(|_| state.waiting),
                    dropped_not_steady: state.dropped_not_steady,
                    error_rms_arcsec: readout.map(|(error, _, _)| error),
                    error_rms_px: readout.map(|(error, _, scale)| error / scale),
                    drift_arcsec: readout.map(|(_, drift, _)| drift),
                    drift_px: readout.map(|(_, drift, scale)| drift / scale),
                },
            )
        };
        let tracker = source.and_then(|source| source());
        if !used && (tracker.is_none() || ask.is_none()) {
            return None;
        }
        if let Some(tracker) = tracker {
            view.source = if tracker.steered_by_subs() {
                "subs"
            } else {
                "tracker"
            };
            if let Some(asked) = asked.or(ask) {
                let limit = tracker.longest_exposure_s(turn_for(&asked, set_point, view.smear_px));
                view.longest_exposure_s = Some(match limit {
                    Some(seconds) => LimitView::Seconds(seconds),
                    None => LimitView::Text("no limit"),
                });
                view.over_limit = limit
                    .zip(view.exposure_s)
                    .is_some_and(|(limit, exposure)| limit < exposure);
            }
            if let Some(ask) = ask {
                let ahead =
                    tracker.longest_exposure_ahead_s(turn_for(&ask, set_point, view.smear_px));
                view.would_take_s = Some(decide(&ask, ahead).0);
            }
        }
        Some(view)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ErrorView {
    pub time: f64,
    pub east_arcsec: f64,
    pub north_arcsec: f64,
}

/// Serialized as a number of seconds, or the string `no limit`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(untagged)]
pub enum LimitView {
    Seconds(f64),
    Text(&'static str),
}

/// The `steering` part of the imaging view.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct SteeringView {
    /// `subs`, `tracker` or `none`.
    pub source: &'static str,
    /// Tracking errors handed over since the tracker was last released.
    pub subs: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last: Option<ErrorView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub far_arcsec: Option<f64>,
    /// Whether a stack keeps a `Hold` alive.
    pub holding: bool,
    pub smear_px: f64,
    pub on: bool,
    pub step_share: f64,
    pub rate_share: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exposure_s: Option<f64>,
    pub limited: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub longest_exposure_s: Option<LimitView>,
    pub over_limit: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub would_take_s: Option<f64>,
    pub steady: bool,
    pub waiting: bool,
    /// The tracker's word, or `limit`, only while waiting.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pause: Option<&'static str>,
    /// This stack's drops, retained after release and reset by the next Hold.
    pub dropped_not_steady: u64,
    /// RMS of the last ten tracking errors, once at least two have arrived.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_rms_arcsec: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_rms_px: Option<f64>,
    /// RMS drift during a sub, from pairs within the same last-ten window.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drift_arcsec: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drift_px: Option<f64>,
}
