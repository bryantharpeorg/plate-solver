//! Feed-forward tracking, corrected by the finder camera or steered by the subs, with latched safe endings.
//!
//! The caller centres the target before starting. All UTC comes from the console's
//! injected clock; timers and heartbeat ages use tokio's (pausable) clock.

use crate::{
    ephemeris,
    goto::{sun_horizontal, Planner, Wind},
    imaging::steering::{GapFuture, Steered, Stepped, TrackingError},
    link::{stop_deadline, LinkManager, MountClient},
    model::Model,
    session::{Clock, Site},
    sky::{self, Observer},
    targets::Target,
};
use scope_link::proto::{Axis, AxisInfo};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    sync::{oneshot, watch, Mutex as AsyncMutex, Notify},
    task::JoinHandle,
    time::{interval, sleep, timeout, Instant, MissedTickBehavior},
};

const TICK: Duration = Duration::from_millis(50);
const SECOND: Duration = Duration::from_secs(1);
const AXES: [Axis; 2] = [Axis::Azimuth, Axis::Altitude];
const PROPORTIONAL_GAIN: f64 = 0.6;
const INTEGRAL_TIME_S: f64 = 60.0;
const DEADBAND_ARCSEC: f64 = 10.0;
const FORECAST_S: f64 = 30.0 * 60.0;
const TAKEUP_RATE: f64 = 0.5;
const KEYHOLE_ALT: f64 = 88.0;

/// Shares of the sub error and measured drift taken out at each gap (036).
const SUB_STEP_SHARE: f64 = 0.5;
const SUB_RATE_SHARE: f64 = 0.3;
/// Maximum offset change in one gap, in arcseconds per second.
const SUB_RATE_STEP: f64 = 0.1;
/// The deadband is the larger of two axis counts and one arcsecond on the sky.
const SUB_DEADBAND_COUNTS: f64 = 2.0;
const SUB_DEADBAND_FLOOR_ARCSEC: f64 = 1.0;
/// Maximum displacement on the sky per axis and duration of its rate pulse.
const SUB_STEP_LIMIT_ARCSEC: f64 = 30.0;
const SUB_STEP_TIME: Duration = Duration::from_millis(500);
/// Eight idle gaps outlast the stack's four queued subs and one being solved.
const STALE_GAPS: u32 = 8;
/// Corrections remembered for late subs, oldest first.
const CORRECTIONS: usize = 16;

struct Measurement {
    baseline_deg: f64,
    saw_error: bool,
    settled_since: Option<f64>,
}
// Five per cent of 360 degrees per mean sidereal day (Meeus ch. 12).
const OFFSET_LIMIT: f64 = 0.05 * 360.985_647_366_29 * 3600.0 / 86_400.0;

/// Error of the aim point relative to the target, in the equatorial tangent plane.
#[derive(Debug, Clone, Copy)]
pub struct PointingError {
    /// Console UTC, Unix seconds.
    pub time: f64,
    /// Eastward error (RA times cos declination), arcseconds.
    pub east_arcsec: f64,
    /// Northward error (declination), arcseconds.
    pub north_arcsec: f64,
}

/// Snapshot consumed by the session. State and reason names are stable strings,
/// so later tracking phases can be reported without exhaustive enum matches.
#[derive(Debug, Clone)]
pub struct State {
    /// `off`, `tracking` or `keyhole_wait` (including the slew and exit solve).
    pub state: &'static str,
    /// `stopped`, `link_lost`, `sun_up`, `too_low`, or a motion failure reason.
    pub ended_reason: Option<String>,
    /// The target of the latest start, retained after ending.
    pub target: Option<Target>,
    /// Pointing errors over the last 60 seconds, for the session's RMS readout.
    pub samples: Vec<PointingError>,
    /// Additional azimuth and altitude rates, arcseconds per second.
    pub tracking_offset: [f64; 2],
    /// Whether the session may check live pointings for a push: true while
    /// tracking with no axis's backlash being measured; false otherwise,
    /// `off` and the keyhole included.
    pub checks_push: bool,
}
impl Default for State {
    fn default() -> Self {
        Self {
            state: "off",
            ended_reason: None,
            target: None,
            samples: Vec::new(),
            tracking_offset: [0.0; 2],
            checks_push: false,
        }
    }
}

#[derive(Default)]
struct Control {
    state: State,
    previous_pointing: Option<f64>,
    proportional: [f64; 2],
    proportional_until: f64,
    measurement: [Option<Measurement>; 2],
    encoder_residual_deg: [f64; 2],
    solve_after: Option<f64>,
    resume_pointing: Option<PointingError>,
    /// The target has been replaced by a followed push; the rate loop's next
    /// turn clears it and refreshes every forecast made for the old place.
    followed: bool,
    /// Counts a turn and the gear directions last left by the rate task.
    counts: [u32; 2],
    direction: [f64; 2],
    /// No gap may claim before the rate task's first rates.
    settling: bool,
    pending: Option<TrackingError>,
    /// Only errors measured after the last followed push may steer.
    fresh_after: f64,
    subs: Option<Subs>,
    /// A stack still holds the tracker, but its errors have gone quiet.
    lapsed: bool,
    held: Held,
    corrections: VecDeque<Correction>,
    gaps: Vec<oneshot::Sender<Stepped>>,
}

#[derive(Default)]
struct Subs {
    /// Time and axis errors of the last error applied in this claim.
    previous: Option<(f64, [f64; 2])>,
    idle: u32,
}

/// The newest finder proportional step after a lapse, in axis arcseconds.
/// No integral is held: the offset stays as the subs left it.
#[derive(Debug, Default, Clone, Copy)]
struct Held {
    step: [f64; 2],
}

struct Correction {
    /// Console time at the end of the gap.
    time: f64,
    /// Actual pulse displacement (axis arcseconds) and offset change (arcsec/s).
    step: [f64; 2],
    rate: [f64; 2],
}

struct Shared {
    clock: Arc<dyn Clock>,
    link: Arc<LinkManager>,
    model: Mutex<Arc<Model>>,
    site: Site,
    min_alt_deg: Mutex<f64>,
    wind: Mutex<Wind>,
    control: Mutex<Control>,
    cancel: watch::Sender<u64>,
    wake: Notify,
}

/// One target's tracking owner. Construct with the session's current pointing
/// model, site and minimum altitude. Tracking is never persisted.
pub struct Tracker {
    shared: Arc<Shared>,
    // Serializes starts with the completion of the preceding run's StopAll.
    task: AsyncMutex<Option<JoinHandle<()>>>,
}
impl Drop for Tracker {
    fn drop(&mut self) {
        // The worker owns Shared, not Tracker: dropping the owner wakes the
        // worker so it can send StopAll before releasing its link and model.
        self.shared.end("stopped".into());
    }
}
impl Tracker {
    /// Supply all planning context up front, including the console's clock and
    /// paired link. `start` only needs the already centred target.
    pub fn new(
        clock: Arc<dyn Clock>,
        link: Arc<LinkManager>,
        model: Model,
        site: Site,
        min_alt_deg: f64,
    ) -> Self {
        let mut wind = Wind::default();
        for sync in &model.syncs {
            wind.observe(sync.az_deg);
        }
        Self {
            shared: Arc::new(Shared {
                clock,
                link,
                model: Mutex::new(Arc::new(model)),
                site,
                min_alt_deg: Mutex::new(min_alt_deg),
                wind: Mutex::new(wind),
                control: Mutex::new(Control::default()),
                cancel: watch::channel(0).0,
                wake: Notify::new(),
            }),
            task: AsyncMutex::new(None),
        }
    }

    /// Update the minimum geometric altitude for ending checks and future plans.
    /// A running target below the new limit ends as `too_low` on the next tick.
    pub fn set_min_alt_deg(&self, deg: f64) {
        *self.shared.min_alt_deg.lock().unwrap() = deg;
    }

    /// Begin sending rates immediately, then once per second. A running tracker
    /// cannot be replaced implicitly; an ended one requires an explicit start.
    pub async fn start(&self, target: Target) -> Result<(), String> {
        self.start_with_resets(target, None).await
    }

    /// Begin with the reset count last seen by the console. With no count yet,
    /// the first tick's read supplies the baseline before any rate is sent.
    pub async fn start_with_resets(
        &self,
        target: Target,
        resets: Option<u64>,
    ) -> Result<(), String> {
        let mut task = self.task.lock().await;
        if self.state().state != "off" {
            return Err("tracking is already running".into());
        }
        let generation = *self.shared.cancel.borrow();
        if let Some(previous) = task.take() {
            let _ = previous.await;
        }
        if let Some(reason) = self.shared.ending(&target) {
            return Err(reason);
        }
        let mut client = self.shared.link.mount().ok_or("no mount is connected")?;
        let axes = timeout(SECOND, client.describe_mount())
            .await
            .map_err(|_| "mount did not answer")?
            .map_err(|_| "no mount is connected")?
            .axes;
        if axes.len() != 2 || axes.iter().any(|a| a.counts_per_revolution == 0) {
            return Err("mount must describe two axes with nonzero counts".into());
        }
        let axes: [AxisInfo; 2] = axes.try_into().unwrap();
        let counts = [axes[0].counts_per_revolution, axes[1].counts_per_revolution];
        self.shared.rates(&target, counts)?;
        // Stop may have arrived while DescribeMount or a prior stop was pending.
        // The version and publication share the state lock with end().
        {
            let mut control = self.shared.control.lock().unwrap();
            if *self.shared.cancel.borrow() != generation {
                return Err("stopped".into());
            }
            *control = Control {
                counts,
                settling: true,
                state: State {
                    state: "tracking",
                    target: Some(target.clone()),
                    ..State::default()
                },
                ..Control::default()
            };
        }
        let shared = self.shared.clone();
        *task = Some(tokio::spawn(async move {
            shared.run(client, axes, generation, resets).await;
        }));
        Ok(())
    }

    /// Record the reason and cancel the rate loop before sending StopAll. Wait
    /// for that run's bounded stop round; a lost link's watchdog is independent.
    pub async fn stop(&self, reason: impl Into<String>) {
        self.stop_by(reason, stop_deadline()).await;
    }

    pub(crate) async fn stop_by(&self, reason: impl Into<String>, deadline: Instant) {
        let reason = reason.into();
        self.shared.end(reason.clone());
        let mut task = self.task.lock().await;
        // A concurrent start may have acquired the lifecycle lock first. Cancel
        // that run too before joining it, so Stop cannot wait on fresh tracking.
        self.shared.end(reason);
        if let Some(previous) = task.take() {
            let _ = previous.await;
        } else {
            self.shared
                .link
                .clone()
                .stop_until_acknowledged(None, deadline)
                .await;
        }
    }

    /// Latest state, with error samples aged using the console's clock.
    pub fn state(&self) -> State {
        let mut control = self.shared.control.lock().unwrap();
        let now = self.shared.clock.now();
        let mut state = {
            let state = &mut control.state;
            state.samples.retain(|s| now - s.time <= 60.0);
            state.clone()
        };
        state.checks_push = state.state == "tracking"
            && control.measurement[0].is_none()
            && control.measurement[1].is_none();
        state
    }

    /// Feed a live solve's aim-point error (or simulator truth plus noise).
    /// Errors are aim point minus target in the J2000 tangent plane, in arcseconds
    /// (east is RA times cos declination). The caller supplies at least one solve
    /// every five seconds. Both solved frames and simulator pointings use this path.
    pub fn pointing(&self, east_arcsec: f64, north_arcsec: f64) {
        self.shared.pointing(east_arcsec, north_arcsec);
    }

    /// Hold this J2000 place in place of the target from the rate loop's next
    /// turn. The target keeps its name and takes the place's RA and Dec. Clears
    /// the proportional term, the previous pointing's time and the error
    /// samples; keeps the tracking offset. Refused, changing nothing, unless the
    /// state is `tracking` and no backlash is being measured, both read under
    /// the control lock it then writes.
    pub fn follow(&self, ra_deg: f64, dec_deg: f64) -> Result<(), String> {
        let mut control = self.shared.control.lock().unwrap();
        if control.state.state != "tracking" || control.measurement.iter().any(Option::is_some) {
            return Err("Track is not tracking; the push cannot be followed".into());
        }
        let Some(target) = control.state.target.as_mut() else {
            return Err("the tracker holds no target".into());
        };
        target.ra_deg = ra_deg;
        target.dec_deg = dec_deg;
        control.followed = true;
        control.proportional = [0.0; 2];
        control.previous_pointing = None;
        control.state.samples.clear();
        // Errors measured at the old place, or steps held for it, would steer
        // back toward it by up to 30 arcseconds a gap (036 FR-011).
        control.subs = None;
        control.pending = None;
        control.lapsed = false;
        control.held = Held::default();
        control.fresh_after = self.shared.clock.now();
        Ok(())
    }

    /// The J2000 place being tracked; `None` when off.
    pub fn target(&self) -> Option<(f64, f64)> {
        let control = self.shared.control.lock().unwrap();
        if control.state.state == "off" {
            None
        } else {
            control.state.target.as_ref().map(|t| (t.ra_deg, t.dec_deg))
        }
    }

    /// Hand over a sub's error. Nothing changes until the next gap.
    pub fn steer(&self, error: TrackingError) {
        let mut control = self.shared.control.lock().unwrap();
        if control.state.state != "tracking"
            || ![error.time, error.east_arcsec, error.north_arcsec]
                .into_iter()
                .all(f64::is_finite)
            || error.time <= control.fresh_after
        {
            return;
        }
        match &control.subs {
            Some(subs) if subs.previous.is_some_and(|(time, _)| time >= error.time) => return,
            Some(_) => {}
            None if control.lapsed => {}
            None => return,
        }
        if control
            .pending
            .is_none_or(|pending| pending.time < error.time)
        {
            control.pending = Some(error);
        }
    }

    /// Queue the gap now; awaiting the answer keeps no tracker alive.
    pub fn gap(&self) -> GapFuture {
        let mut control = self.shared.control.lock().unwrap();
        if control.state.state == "off" {
            return Box::pin(async { Stepped::default() });
        }
        let (send, receive) = oneshot::channel();
        control.gaps.push(send);
        drop(control);
        self.shared.wake.notify_one();
        Box::pin(async move { receive.await.unwrap_or_default() })
    }

    /// Let the finder camera correct again, preserving the learned offset.
    pub fn release(&self) {
        let mut control = self.shared.control.lock().unwrap();
        control.subs = None;
        control.pending = None;
        control.lapsed = false;
        control.held = Held::default();
    }

    /// Whether tracking errors currently steer the tracker.
    pub fn steered_by_subs(&self) -> bool {
        self.shared.control.lock().unwrap().subs.is_some()
    }

    /// Use this pointing model for every later rate, forecast and keyhole plan.
    pub fn set_model(&self, model: Model) {
        *self.shared.model.lock().unwrap() = Arc::new(model);
    }
}

impl Steered for Tracker {
    fn target(&self) -> Option<(f64, f64)> {
        self.target()
    }
    fn steer(&self, error: TrackingError) {
        self.steer(error);
    }
    fn gap(&self) -> GapFuture {
        self.gap()
    }
    fn release(&self) {
        self.release();
    }
    fn steered_by_subs(&self) -> bool {
        self.steered_by_subs()
    }
}

impl Shared {
    fn pointing(&self, east_arcsec: f64, north_arcsec: f64) {
        let mut control = self.control.lock().unwrap();
        if control.state.state == "off" || !east_arcsec.is_finite() || !north_arcsec.is_finite() {
            return;
        }
        let time = self.clock.now();
        let target = control.state.target.as_ref().unwrap();
        // Retain the night's cable travel through every solved azimuth, even
        // while waiting. A fresh Wind at the keyhole would forget earlier turns.
        let (_, az) = sky::alt_az(
            target.ra_deg + east_arcsec / (3600.0 * target.dec_deg.to_radians().cos()),
            target.dec_deg + north_arcsec / 3600.0,
            &Observer {
                lat: self.site.lat_deg,
                lon: self.site.lon_deg,
                time,
            },
        );
        self.wind.lock().unwrap().observe(az);
        if control.state.state == "keyhole_wait" {
            if control.solve_after.is_some_and(|after| time >= after) {
                control.resume_pointing = Some(PointingError {
                    time,
                    east_arcsec,
                    north_arcsec,
                });
            }
            return;
        }
        if control.subs.is_none() {
            let Control {
                state,
                previous_pointing,
                proportional,
                proportional_until,
                lapsed,
                held,
                ..
            } = &mut *control;
            // Preserve 015's timing and PI when no stack holds the tracker.
            let elapsed = previous_pointing.map(|previous| time - previous);
            let interval = elapsed.filter(|dt| *dt > 0.0).unwrap_or(1.0);
            *previous_pointing = Some(time);
            *proportional = [0.0; 2];
            *proportional_until = time + interval;
            held.step = [0.0; 2];
            if east_arcsec.hypot(north_arcsec) >= DEADBAND_ARCSEC {
                let target = state.target.as_ref().unwrap();
                let axis_error = self.axis_error(target, time, east_arcsec, north_arcsec);
                for i in 0..2 {
                    if *lapsed {
                        held.step[i] = -PROPORTIONAL_GAIN * axis_error[i];
                    } else {
                        proportional[i] = -PROPORTIONAL_GAIN * axis_error[i] / interval;
                        if elapsed.is_some_and(|dt| dt > 0.0) {
                            // Ki = Kp / Ti, integrated over the solve interval.
                            state.tracking_offset[i] = (state.tracking_offset[i]
                                + proportional[i] * interval / INTEGRAL_TIME_S)
                                .clamp(-OFFSET_LIMIT, OFFSET_LIMIT);
                        }
                    }
                }
            }
        }
        let state = &mut control.state;
        state.samples.retain(|s| time - s.time <= 60.0);
        state.samples.push(PointingError {
            time,
            east_arcsec,
            north_arcsec,
        });
        drop(control);
        self.settle(time, east_arcsec, north_arcsec);
    }
}

impl Shared {
    /// The model the loop currently plans through; a handed-over replacement
    /// shares the same backlash and crossing state through `Model::motion`.
    fn model(&self) -> Arc<Model> {
        self.model.lock().unwrap().clone()
    }

    fn end(&self, reason: String) {
        let model = self.model();
        let mut control = self.control.lock().unwrap();
        let state = &mut control.state;
        if state.state != "off" {
            state.state = "off";
            state.ended_reason = Some(reason);
            state.tracking_offset = [0.0; 2];
        }
        control.proportional = [0.0; 2];
        control.previous_pointing = None;
        control.subs = None;
        control.pending = None;
        control.lapsed = false;
        control.held = Held::default();
        control.corrections.clear();
        control.gaps.clear();
        for (i, measurement) in control.measurement.iter_mut().enumerate() {
            if measurement.take().is_some() {
                model.motion.crossing(i, false);
            }
        }
        self.cancel
            .send_modify(|generation| *generation = generation.wrapping_add(1));
    }

    fn place(&self, target: &Target, time: f64) -> (f64, f64) {
        sky::alt_az(
            target.ra_deg,
            target.dec_deg,
            &Observer {
                lat: self.site.lat_deg,
                lon: self.site.lon_deg,
                time,
            },
        )
    }

    /// Parallactic angle of J2000 north and apparent altitude, in radians.
    fn axis_frame(&self, target: &Target, time: f64) -> (f64, f64) {
        let (alt, az) = self.place(target, time);
        // The direction of J2000 north in the horizon tangent plane gives the
        // parallactic angle, including precession of north (as Sync::from_frame
        // does for camera up). This also avoids mixing J2000 errors with q of date.
        let (north_alt, north_az) = sky::alt_az(
            target.ra_deg,
            target.dec_deg + 1e-5,
            &Observer {
                lat: self.site.lat_deg,
                lon: self.site.lon_deg,
                time,
            },
        );
        let cross = ((north_az - az + 180.0).rem_euclid(360.0) - 180.0) * alt.to_radians().cos();
        let q = cross.atan2(north_alt - alt);
        (q, ephemeris::apparent_from_true(alt).to_radians())
    }

    fn axis_error(&self, target: &Target, time: f64, east: f64, north: f64) -> [f64; 2] {
        let (q, elevation) = self.axis_frame(target, time);
        let (sin_q, cos_q) = q.sin_cos();
        // North-through-east azimuth reverses the equatorial east direction.
        // Wallace (2002): rotate to cross-elevation/elevation, then divide by cos E.
        [
            (-cos_q * east + sin_q * north) / elevation.cos(),
            sin_q * east + cos_q * north,
        ]
    }

    fn ending(&self, target: &Target) -> Option<String> {
        let status = self.link.status().borrow().clone();
        if !status.connected
            || status
                .last_heartbeat_at
                .is_none_or(|t| t.elapsed() >= SECOND)
        {
            return Some("link_lost".into());
        }
        let now = self.clock.now();
        if sun_horizontal(self.site.lat_deg, self.site.lon_deg, now).0 >= -0.83 {
            return Some("sun_up".into());
        }
        if self.place(target, now).0 < *self.min_alt_deg.lock().unwrap() {
            return Some("too_low".into());
        }
        None
    }

    fn rates(&self, target: &Target, counts: [u32; 2]) -> Result<[f64; 2], String> {
        self.rates_at(target, counts, self.clock.now())
    }

    fn positions(&self, target: &Target, counts: [u32; 2], time: f64) -> Option<[f64; 2]> {
        let (alt, az) = self.place(target, time);
        self.model()
            .inverse(az, ephemeris::apparent_from_true(alt), counts)
    }

    fn rates_at(&self, target: &Target, counts: [u32; 2], now: f64) -> Result<[f64; 2], String> {
        let model = self.model();
        let positions = [now, now + 1.0].map(|time| {
            let (alt, az) = self.place(target, time);
            model.inverse(az, ephemeris::apparent_from_true(alt), counts)
        });
        let [Some(here), Some(next)] = positions else {
            return Err("cannot invert the pointing model".into());
        };
        // Both inverse outputs are fractional counts. Use the short angular
        // difference across the azimuth wrap (zero points need not be zero).
        Ok([0, 1].map(|i| {
            let delta = (next[i] - here[i]) * 360.0 / f64::from(counts[i]);
            if i == 0 {
                (delta + 180.0).rem_euclid(360.0) - 180.0
            } else {
                delta
            }
        }))
    }

    // Locate each sign change in model space, including all fixed terms. The
    // 30-minute window is sampled each minute then bisected to 50 ms; forecasts
    // are cached between refreshes, keeping long paused-time runs inexpensive.
    fn reversal(&self, target: &Target, counts: [u32; 2], axis: usize) -> Option<f64> {
        let now = self.clock.now();
        let initial = self.rates_at(target, counts, now).ok()?[axis];
        let mut lo = now;
        for step in 1..=30 {
            let hi = now + f64::from(step) * FORECAST_S / 30.0;
            if self.rates_at(target, counts, hi).ok()?[axis] * initial <= 0.0 {
                let mut hi = hi;
                while hi - lo > TICK.as_secs_f64() {
                    let mid = (lo + hi) / 2.0;
                    if self.rates_at(target, counts, mid).ok()?[axis] * initial > 0.0 {
                        lo = mid;
                    } else {
                        hi = mid;
                    }
                }
                return Some((lo + hi) / 2.0);
            }
            lo = hi;
        }
        None
    }

    fn bisect(mut lo: f64, mut hi: f64, before: impl Fn(f64) -> bool) -> f64 {
        while hi - lo > TICK.as_secs_f64() {
            let mid = (lo + hi) / 2.0;
            if before(mid) {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        hi
    }

    // Forecast the next 30 minutes. US1's geometric altitude rate locates a
    // maximum between samples, so even a short, grazing passage is detected.
    fn keyhole_entry(&self, target: &Target) -> Option<f64> {
        let now = self.clock.now();
        if self.place(target, now).0 >= KEYHOLE_ALT {
            return Some(now);
        }
        let rising = |t| {
            let (alt, az) = self.place(target, t);
            ephemeris::altaz_rates_from_alt_az(self.site.lat_deg, alt, az).alt_arcsec_per_s > 0.0
        };
        let mut lo = now;
        for step in 1..=30 {
            let hi = now + f64::from(step) * FORECAST_S / 30.0;
            let peak = if rising(lo) && !rising(hi) {
                Self::bisect(lo, hi, rising)
            } else {
                hi
            };
            if self.place(target, peak).0 >= KEYHOLE_ALT {
                return Some(Self::bisect(lo, peak, |t| {
                    self.place(target, t).0 < KEYHOLE_ALT
                }));
            }
            lo = hi;
        }
        None
    }

    fn keyhole_exit(&self, target: &Target) -> Result<f64, String> {
        let now = self.clock.now();
        let mut lo = now;
        // Complete the crossing already forecast. At high latitudes its exit
        // can be beyond the entry forecast's 30-minute horizon. One rotation
        // also bounds the search for a direction permanently inside the circle.
        for minute in 1..=1440 {
            let hi = now + f64::from(minute) * 60.0;
            if self.place(target, hi).0 < KEYHOLE_ALT {
                return Ok(Self::bisect(lo, hi, |t| {
                    self.place(target, t).0 >= KEYHOLE_ALT
                }));
            }
            lo = hi;
        }
        Err("target does not leave the zenith circle".into())
    }

    // Observe continuously, including long cable-unwinding slews. Wind::observe
    // takes successive directions less than half a revolution apart.
    async fn at_rest(
        &self,
        client: &mut MountClient,
        counts: [u32; 2],
        resets: u64,
    ) -> Result<[i32; 2], String> {
        let model = self.model();
        loop {
            let az = client
                .axis_state(Axis::Azimuth)
                .await
                .map_err(|_| "link_lost")?;
            if az.mount_resets != resets {
                return Err("the mount was power-cycled".into());
            }
            let alt = client
                .axis_state(Axis::Altitude)
                .await
                .map_err(|_| "link_lost")?;
            if alt.mount_resets != resets {
                return Err("the mount was power-cycled".into());
            }
            let positions = [az.position, alt.position];
            if let Some(p) = model.predict(positions.map(f64::from), counts) {
                self.wind.lock().unwrap().observe(p.az_deg);
            }
            if !az.moving && !alt.moving {
                return Ok(positions);
            }
            sleep(TICK).await;
        }
    }

    async fn keyhole(
        &self,
        target: &Target,
        client: &mut MountClient,
        axes: [AxisInfo; 2],
        resets: u64,
    ) -> Result<[f64; 2], String> {
        {
            let mut control = self.control.lock().unwrap();
            if control.state.state == "off" {
                return Err("stopped".into());
            }
            control.state.state = "keyhole_wait";
            control.state.tracking_offset = [0.0; 2];
            control.proportional = [0.0; 2];
            control.previous_pointing = None;
            control.solve_after = None;
            control.resume_pointing = None;
            let model = self.model();
            for (i, measurement) in control.measurement.iter_mut().enumerate() {
                *measurement = None;
                model.motion.crossing(i, true);
            }
        }
        client.stop_all().await.map_err(|_| "link_lost")?;
        let counts = axes.map(|a| a.counts_per_revolution);
        let mut current = self.at_rest(client, counts, resets).await?;
        let exit = self.keyhole_exit(target)?;
        // The planner borrows the model this keyhole began with, even if the
        // session hands a newer one over while the slew runs.
        let model = self.model();
        let mut planner = Planner {
            model: &model,
            axes,
            current,
            wind: *self.wind.lock().unwrap(),
            site: self.site,
            min_alt: *self.min_alt_deg.lock().unwrap(),
            start: exit,
        };
        // Planner::plan aims at start + travel. The first plan estimates that
        // travel; the second subtracts it so the destination is the exit itself.
        let first = planner.plan(target)?;
        planner.start = exit - first.duration;
        let plan = planner.plan(target)?;
        for waypoint in plan.waypoints {
            for (i, axis) in AXES.into_iter().enumerate() {
                model
                    .motion
                    .moving(i, f64::from(waypoint[i]) - f64::from(current[i]));
                model.motion.crossing(i, true);
                client
                    .goto_axis(axis, waypoint[i])
                    .await
                    .map_err(|_| "mount refused the slew")?;
            }
            current = self.at_rest(client, counts, resets).await?;
        }
        for i in 0..2 {
            model.motion.crossing(i, false);
        }
        // Neither PI nor reversal take-up runs during this wait. Solves made
        // here still observe cable wind but cannot provide the resume pointing.
        sleep(Duration::from_millis(500)).await;
        while self.clock.now() < exit || self.place(target, self.clock.now()).0 >= KEYHOLE_ALT {
            sleep(TICK).await;
        }
        self.control.lock().unwrap().solve_after = Some(self.clock.now());
        let pointing = loop {
            if let Some(p) = self.control.lock().unwrap().resume_pointing {
                break p;
            }
            sleep(TICK).await;
        };
        // The planner's last approach drove both axes positively. Take up a
        // measured gear before reversing it, just as at a forecast reversal.
        let rates = self.rates(target, counts)?;
        let mut direction = [1.0; 2];
        for i in 0..2 {
            if rates[i] < 0.0 {
                if let Some(b) = self.model().motion.backlash().axis(i) {
                    self.take_up(client, i, -1.0, b).await?;
                    direction[i] = -1.0;
                }
            }
        }
        {
            let mut control = self.control.lock().unwrap();
            if control.state.state == "off" {
                return Err("stopped".into());
            }
            control.state.state = "tracking";
            control.solve_after = None;
            control.resume_pointing = None;
        }
        self.pointing(pointing.east_arcsec, pointing.north_arcsec);
        Ok(direction)
    }

    fn settle(&self, time: f64, east: f64, north: f64) {
        let mut measured = [None; 2];
        {
            let mut control = self.control.lock().unwrap();
            let Some(target) = control.state.target.as_ref() else {
                return;
            };
            let errors = self.axis_error(target, time, east, north);
            for i in 0..2 {
                let residual = control.encoder_residual_deg[i];
                let Some(m) = &mut control.measurement[i] else {
                    continue;
                };
                m.saw_error |= errors[i].abs() >= DEADBAND_ARCSEC;
                if m.saw_error && errors[i].abs() < DEADBAND_ARCSEC {
                    let since = *m.settled_since.get_or_insert(time);
                    if time - since >= 10.0 {
                        // Encoder drive minus the inverse model's feed-forward
                        // travel, removing the remaining solved error. This is
                        // the settled optical/count shift across the dead band,
                        // equivalent to integrating drive added by the PI loop.
                        measured[i] = Some((residual - m.baseline_deg - errors[i] / 3600.0).abs());
                        control.measurement[i] = None;
                    }
                } else {
                    m.settled_since = None;
                }
            }
        }
        for (i, degrees) in measured.into_iter().enumerate() {
            if let Some(degrees) = degrees {
                let model = self.model();
                if let Err(reason) = model.motion.measured(i, degrees) {
                    self.end(reason);
                    return;
                }
                model.motion.crossing(i, false);
            }
        }
    }

    /// Serve all live callers together, only from the once-a-second rate task.
    async fn serve_gaps(&self, movement: &mut MountClient) -> Result<(), String> {
        let senders: Vec<_> = std::mem::take(&mut self.control.lock().unwrap().gaps)
            .into_iter()
            .filter(|sender| !sender.is_closed())
            .collect();
        if senders.is_empty() {
            return Ok(());
        }
        let (target, counts, direction, offset, want, changed, applied, held_step) = {
            let mut control = self.control.lock().unwrap();
            // Dropping these senders answers zero, including before the first rates.
            if control.state.state != "tracking" || control.settling {
                return Ok(());
            }
            let target = control.state.target.clone().unwrap();
            let counts = control.counts;
            let direction = control.direction;
            let claimed = control.subs.is_none() && (!control.lapsed || control.pending.is_some());
            if claimed {
                control.subs = Some(Subs::default());
                control.lapsed = false;
                control.held = Held::default();
                control.proportional = [0.0; 2];
                control.proportional_until = 0.0;
                control.previous_pointing = None;
            }
            let (_, elevation) = self.axis_frame(&target, self.clock.now());
            let sky = [elevation.cos(), 1.0];
            let deadband = [0, 1].map(|i| {
                (SUB_DEADBAND_COUNTS * 1_296_000.0 / f64::from(counts[i]))
                    .max(SUB_DEADBAND_FLOOR_ARCSEC / sky[i])
            });
            let limit = sky.map(|scale| SUB_STEP_LIMIT_ARCSEC / scale);
            let mut want = [0.0; 2];
            let mut changed = [0.0; 2];
            let mut applied = None;
            let mut held_step = false;
            if control.subs.is_none() {
                let held = std::mem::take(&mut control.held);
                held_step = held.step.iter().any(|step| *step != 0.0);
                want = [0, 1].map(|i| held.step[i].clamp(-limit[i], limit[i]));
                control.proportional = [0.0; 2];
                control.proportional_until = 0.0;
            } else if let Some(error) = control.pending.take() {
                let t1 = error.time;
                let a = self.axis_error(&target, t1, error.east_arcsec, error.north_arcsec);
                let previous = control.subs.as_ref().unwrap().previous;
                control.subs.as_mut().unwrap().idle = 0;
                for i in 0..2 {
                    if let Some((t0, a0)) = previous {
                        let dt = t1 - t0;
                        if dt > 0.0 {
                            let between: f64 = control
                                .corrections
                                .iter()
                                .filter(|c| t0 < c.time && c.time <= t1)
                                .map(|c| c.step[i])
                                .sum();
                            // Offset changes after t0 were absent for this much of
                            // the measurement. Account for even changes after t1.
                            let unseen: f64 = control
                                .corrections
                                .iter()
                                .filter(|c| c.time > t0)
                                .map(|c| c.rate[i] * (c.time.min(t1) - t0))
                                .sum();
                            let drift_rate = (a[i] - a0[i] - between + unseen) / dt;
                            if drift_rate.abs() <= 2.0 * OFFSET_LIMIT
                                && between.abs() <= 2.0 * deadband[i]
                            {
                                let old = control.state.tracking_offset[i];
                                let new = (old
                                    - (SUB_RATE_SHARE * drift_rate)
                                        .clamp(-SUB_RATE_STEP, SUB_RATE_STEP))
                                .clamp(-OFFSET_LIMIT, OFFSET_LIMIT);
                                changed[i] = new - old;
                                control.state.tracking_offset[i] = new;
                            }
                        }
                    }
                    let after: f64 = control
                        .corrections
                        .iter()
                        .filter(|c| c.time > t1)
                        .map(|c| c.step[i])
                        .sum();
                    let seen = a[i] + after;
                    if seen.abs() >= deadband[i] {
                        want[i] = (-SUB_STEP_SHARE * seen).clamp(-limit[i], limit[i]);
                    }
                }
                applied = Some((t1, a));
            } else if !claimed {
                let subs = control.subs.as_mut().unwrap();
                subs.idle += 1;
                if subs.idle >= STALE_GAPS {
                    control.subs = None;
                    control.lapsed = true;
                    // Last rates have no finder proportional term: leave them in
                    // force at the lapse itself. Later gaps always refresh both.
                    return Ok(());
                }
            }
            (
                target,
                counts,
                direction,
                control.state.tracking_offset,
                want,
                changed,
                applied,
                held_step,
            )
        };
        let rates = self.rates(&target, counts)?;
        let base = [0, 1].map(|i| commanded(rates[i], direction[i], offset[i] / 3600.0));
        let pulsed = [0, 1].map(|i| {
            commanded(
                rates[i],
                direction[i],
                (offset[i] + want[i] / SUB_STEP_TIME.as_secs_f64()) / 3600.0,
            )
        });
        let pulse = [0, 1].map(|i| pulsed[i] - base[i]);
        let mut began = [Instant::now(); 2];
        let has_pulse = pulse.iter().any(|p| *p != 0.0);
        let model = self.model();
        if has_pulse {
            for (i, axis) in AXES.into_iter().enumerate() {
                model.motion.moving(i, pulsed[i]);
                movement
                    .rate_axis(axis, pulsed[i])
                    .await
                    .map_err(|_| "link_lost")?;
                began[i] = Instant::now();
            }
            sleep(SUB_STEP_TIME).await;
        }
        // Recompute feed-forward after the pulse, retaining its original base
        // for the displacement: sky motion is not part of the correction.
        let rates = self.rates(&target, counts)?;
        let mut step = [0.0; 2];
        for (i, axis) in AXES.into_iter().enumerate() {
            let rate = commanded(rates[i], direction[i], offset[i] / 3600.0);
            model.motion.moving(i, rate);
            movement
                .rate_axis(axis, rate)
                .await
                .map_err(|_| "link_lost")?;
            if has_pulse {
                step[i] = pulse[i] * 3600.0 * began[i].elapsed().as_secs_f64();
            }
        }
        {
            let mut control = self.control.lock().unwrap();
            if let (Some(error), Some(subs)) = (applied, control.subs.as_mut()) {
                subs.previous = Some(error);
            }
            if applied.is_some() || held_step {
                control.corrections.push_back(Correction {
                    time: self.clock.now(),
                    step,
                    rate: changed,
                });
                if control.corrections.len() > CORRECTIONS {
                    control.corrections.pop_front();
                }
            }
        }
        let (q, elevation) = self.axis_frame(&target, self.clock.now());
        let (sin_q, cos_q) = q.sin_cos();
        let x = step[0] * elevation.cos();
        let answer = Stepped {
            east_arcsec: -cos_q * x + sin_q * step[1],
            north_arcsec: sin_q * x + cos_q * step[1],
        };
        for sender in senders {
            let _ = sender.send(answer);
        }
        Ok(())
    }

    async fn stop_axis(client: &mut MountClient, axis: Axis) -> Result<(), String> {
        client.rate_axis(axis, 0.0).await.map_err(|_| "link_lost")?;
        loop {
            let state = client.axis_state(axis).await.map_err(|_| "link_lost")?;
            if !state.moving {
                return Ok(());
            }
            sleep(TICK).await;
        }
    }

    async fn take_up(
        &self,
        client: &mut MountClient,
        i: usize,
        direction: f64,
        degrees: f64,
    ) -> Result<(), String> {
        let model = self.model();
        model.motion.moving(i, direction);
        model.motion.crossing(i, true);
        Self::stop_axis(client, AXES[i]).await?;
        client
            .rate_axis(AXES[i], direction * TAKEUP_RATE)
            .await
            .map_err(|_| "link_lost")?;
        // Acceleration and braking have equal areas about this rectangular
        // pulse. Wait for rest before resuming feed-forward; never interleave
        // the one-second rate refresh with the take-up pulse.
        sleep(Duration::from_secs_f64(degrees / TAKEUP_RATE)).await;
        Self::stop_axis(client, AXES[i]).await?;
        model.motion.moving(i, direction);
        model.motion.crossing(i, false);
        Ok(())
    }

    async fn guard(&self, generation: u64) -> String {
        let mut cancel = self.cancel.subscribe();
        let mut tick = interval(TICK);
        tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            if *cancel.borrow_and_update() != generation {
                return "stopped".into();
            }
            let target = self.control.lock().unwrap().state.target.clone();
            if let Some(reason) = target.and_then(|target| self.ending(&target)) {
                return reason;
            }
            tokio::select! { biased; _ = cancel.changed() => {}, _ = tick.tick() => {} }
        }
    }

    async fn run(
        self: Arc<Self>,
        client: MountClient,
        axes: [AxisInfo; 2],
        generation: u64,
        mut resets: Option<u64>,
    ) {
        let mut movement = client.clone();
        let counts = axes.map(|a| a.counts_per_revolution);
        let rates = async {
            let mut second = interval(SECOND);
            second.set_missed_tick_behavior(MissedTickBehavior::Skip);
            let mut direction = [0.0_f64; 2];
            let mut forecast = [None; 2];
            let mut prepared = [false; 2];
            let mut keyhole_entry = None;
            let mut refresh_at = 0.0;
            loop {
                tokio::select! {
                    biased;
                    _ = self.wake.notified() => {
                        if let Err(reason) = self.serve_gaps(&mut movement).await {
                            return reason;
                        }
                        continue;
                    }
                    _ = second.tick() => {}
                }
                let state = match movement.axis_state(Axis::Azimuth).await {
                    Ok(state) => state,
                    Err(_) => return "link_lost".into(),
                };
                let resets = *resets.get_or_insert(state.mount_resets);
                if state.mount_resets != resets {
                    return "the mount was power-cycled".into();
                }
                let now = self.clock.now();
                // The rate loop plans for the place the tracker holds, which a
                // followed push can change between turns.
                let (followed, target) = {
                    let mut control = self.control.lock().unwrap();
                    (
                        std::mem::take(&mut control.followed),
                        control.state.target.clone(),
                    )
                };
                let Some(target) = target else {
                    return "stopped".into();
                };
                let model = self.model();
                if followed {
                    refresh_at = 0.0;
                }
                if now >= refresh_at {
                    keyhole_entry = self.keyhole_entry(&target);
                    forecast = [0, 1].map(|i| self.reversal(&target, counts, i));
                    refresh_at = now + 60.0;
                }
                if keyhole_entry.is_some_and(|entry| now >= entry) {
                    direction = match self.keyhole(&target, &mut movement, axes, resets).await {
                        Ok(direction) => direction,
                        Err(reason) => return reason,
                    };
                    keyhole_entry = None;
                    forecast = [None; 2];
                    prepared = [false; 2];
                    refresh_at = 0.0;
                    second.reset();
                }
                let rates = match self.rates(&target, counts) {
                    Ok(rates) => rates,
                    Err(reason) => return reason,
                };
                for i in 0..2 {
                    let next = if rates[i] == 0.0 {
                        direction[i]
                    } else {
                        rates[i].signum()
                    };
                    if next == 0.0 {
                        continue;
                    }
                    let backlash = model.motion.backlash().axis(i);
                    let approaching = backlash.is_some_and(|b| {
                        forecast[i].is_some_and(|t| t >= now && t - now <= b / TAKEUP_RATE + 1.0)
                    });
                    let initial_opposite = direction[i] == 0.0 && next < 0.0;
                    if prepared[i] && next == direction[i] {
                        prepared[i] = false;
                    }
                    if !prepared[i]
                        && (approaching
                            || initial_opposite
                            || (direction[i] != 0.0 && direction[i] != next))
                    {
                        if let Some(b) = backlash {
                            let new_direction = if approaching { -next } else { next };
                            if direction[i] != new_direction {
                                if let Err(reason) =
                                    self.take_up(&mut movement, i, new_direction, b).await
                                {
                                    return reason;
                                }
                                direction[i] = new_direction;
                                prepared[i] = approaching;
                                forecast[i] = None;
                            }
                        } else {
                            if let Err(reason) = Self::stop_axis(&mut movement, AXES[i]).await {
                                return reason;
                            }
                            let state = match movement.axis_state(AXES[i]).await {
                                Ok(s) => s,
                                Err(_) => return "link_lost".into(),
                            };
                            let expected =
                                self.positions(&target, counts, self.clock.now()).unwrap();
                            self.control.lock().unwrap().measurement[i] = Some(Measurement {
                                baseline_deg: (f64::from(state.position) - expected[i]) * 360.0
                                    / f64::from(counts[i]),
                                saw_error: false,
                                settled_since: None,
                            });
                            model.motion.crossing(i, true);
                            direction[i] = next;
                        }
                    }
                    if direction[i] == 0.0 {
                        direction[i] = next;
                    }
                }
                // Read counts at the same model time as the rates. Pointings
                // use this residual only to measure a settled dead band.
                for i in 0..2 {
                    if self.control.lock().unwrap().measurement[i].is_some() {
                        let state = match movement.axis_state(AXES[i]).await {
                            Ok(s) => s,
                            Err(_) => return "link_lost".into(),
                        };
                        let expected = self.positions(&target, counts, self.clock.now()).unwrap();
                        self.control.lock().unwrap().encoder_residual_deg[i] =
                            (f64::from(state.position) - expected[i]) * 360.0
                                / f64::from(counts[i]);
                    }
                }
                let correction = {
                    let control = self.control.lock().unwrap();
                    [0, 1].map(|i| {
                        let p = if self.clock.now() < control.proportional_until {
                            control.proportional[i]
                        } else {
                            0.0
                        };
                        (p + control.state.tracking_offset[i]) / 3600.0
                    })
                };
                self.control.lock().unwrap().direction = direction;
                for (i, axis) in AXES.into_iter().enumerate() {
                    let rate = commanded(rates[i], direction[i], correction[i]);
                    model.motion.moving(i, rate);
                    if movement.rate_axis(axis, rate).await.is_err() {
                        return "link_lost".into();
                    }
                }
                self.control.lock().unwrap().settling = false;
            }
        };
        let reason = tokio::select! {
            biased;
            reason = self.guard(generation) => reason,
            reason = rates => reason,
        };
        // Publish the ending before StopAll; dropping the selected-out rate
        // future and invalidating the node queue prevents any later refresh.
        self.end(reason);
        self.link
            .clone()
            .stop_until_acknowledged(Some(client), stop_deadline())
            .await;
    }
}

/// Feed-forward plus an extra rate, or zero instead of unloading an axis's gear.
/// Inputs and answer are degrees per second.
fn commanded(rate: f64, direction: f64, extra: f64) -> f64 {
    let requested = rate + extra;
    if rate.signum() != direction || requested * direction < 0.0 {
        0.0
    } else {
        requested
    }
}
