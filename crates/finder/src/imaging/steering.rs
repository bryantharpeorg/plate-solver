//! Tracking steered by the subs (036): the tracking error a sub's imaging solve
//! gives and the one interface between the stack and a tracker. Sky terms only.

use std::{
    collections::VecDeque,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex, Weak},
};

use serde::Serialize;

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
}

#[derive(Default)]
struct State {
    used: bool,
    source: Option<Source>,
    set_point: Option<(f64, f64)>,
    tracker: Option<Weak<dyn Steered>>,
    holds: usize,
    taken: VecDeque<Taken>,
    subs: u64,
    last: Option<ErrorView>,
    far: Option<f64>,
}

impl State {
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
            state: Mutex::new(State::default()),
        })
    }

    pub fn set_source(&self, source: Source) {
        self.state.lock().expect("steering state").source = Some(source);
    }

    /// For the life of a stack's taker.
    pub fn hold(self: &Arc<Self>) -> Hold {
        self.state.lock().expect("steering state").holds += 1;
        Hold {
            steering: self.clone(),
        }
    }

    /// In the gap, before every exposure. Remember only when the answer is ready.
    pub async fn before_exposure(&self, ask: ExposureAsk) -> Go {
        let source = {
            let mut state = self.state.lock().expect("steering state");
            state.used = true;
            state.source.clone()
        };
        let tracker = source.and_then(|source| source());
        let weak = tracker.as_ref().map(Arc::downgrade);
        let far = {
            let mut state = self.state.lock().expect("steering state");
            state.tracker = weak.clone();
            state.far.is_some()
        };
        if let Some(tracker) = tracker {
            if !far {
                let answer = tracker.gap();
                drop(tracker);
                answer.await;
            }
        }
        // A gap's future owns no tracker. It may have disappeared while waiting.
        let tracker = weak
            .and_then(|weak| weak.upgrade())
            .map(|tracker| Arc::downgrade(&tracker));
        let go_time = self.clock.now();
        self.state.lock().expect("steering state").remember(Taken {
            go_time,
            tracker,
            sensor: ask.sensor,
        });
        Go {
            exposure_s: ask.wanted_s,
            limited: false,
        }
    }

    /// Hand over the tracking error of a sub that the stack accepted.
    pub fn accepted(&self, frame: &SolveFrame, solve: &ImagingSolve) {
        let time = frame.start + frame.exposure_s / 2.0;
        let (weak, sensor, set_point) = {
            let state = self.state.lock().expect("steering state");
            if state.far.is_some() {
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
            if state.far.is_some() {
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
            state.far = None;
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

    /// `None` until the steering is first used.
    pub fn view(&self) -> Option<SteeringView> {
        let (source, mut view) = {
            let state = self.state.lock().expect("steering state");
            if !state.used {
                return None;
            }
            (
                state.source.clone(),
                SteeringView {
                    source: "none",
                    subs: state.subs,
                    last: state.last.clone(),
                    far_arcsec: state.far,
                    holding: state.holds > 0,
                },
            )
        };
        if let Some(tracker) = source.and_then(|source| source()) {
            view.source = if tracker.steered_by_subs() {
                "subs"
            } else {
                "tracker"
            };
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
}
