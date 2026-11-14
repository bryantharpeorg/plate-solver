//! Feed-forward and finder-camera PI tracking, with latched safe endings.
//!
//! The caller centres the target before starting. All UTC comes from the console's
//! injected clock; timers and heartbeat ages use tokio's (pausable) clock.

use crate::{
    ephemeris,
    goto::{sun_horizontal, Planner, Wind},
    link::{stop_deadline, LinkManager, MountClient},
    model::Model,
    session::{Clock, Site},
    sky::{self, Observer},
    targets::Target,
};
use scope_link::proto::{Axis, AxisInfo};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    sync::{watch, Mutex as AsyncMutex},
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
}
impl Default for State {
    fn default() -> Self {
        Self {
            state: "off",
            ended_reason: None,
            target: None,
            samples: Vec::new(),
            tracking_offset: [0.0; 2],
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
}

struct Shared {
    clock: Arc<dyn Clock>,
    link: Arc<LinkManager>,
    model: Model,
    site: Site,
    min_alt_deg: Mutex<f64>,
    wind: Mutex<Wind>,
    control: Mutex<Control>,
    cancel: watch::Sender<u64>,
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
                model,
                site,
                min_alt_deg: Mutex::new(min_alt_deg),
                wind: Mutex::new(wind),
                control: Mutex::new(Control::default()),
                cancel: watch::channel(0).0,
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
            shared.run(target, client, axes, generation).await;
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
        let state = &mut control.state;
        let now = self.shared.clock.now();
        state.samples.retain(|s| now - s.time <= 60.0);
        state.clone()
    }

    /// Feed a live solve's aim-point error (or simulator truth plus noise).
    /// Errors are aim point minus target in the J2000 tangent plane, in arcseconds
    /// (east is RA times cos declination). The caller supplies at least one solve
    /// every five seconds. Both solved frames and simulator pointings use this path.
    pub fn pointing(&self, east_arcsec: f64, north_arcsec: f64) {
        self.shared.pointing(east_arcsec, north_arcsec);
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
        let Control {
            state,
            previous_pointing,
            proportional,
            proportional_until,
            ..
        } = &mut *control;
        // The first solve establishes timing; there is no preceding interval to
        // integrate. Later solves set the interval over which P removes 60% of
        // the error. Repeated timestamps must not accumulate an integral.
        let elapsed = previous_pointing.map(|previous| time - previous);
        let interval = elapsed.filter(|dt| *dt > 0.0).unwrap_or(1.0);
        *previous_pointing = Some(time);
        *proportional = [0.0; 2];
        *proportional_until = time + interval;
        if east_arcsec.hypot(north_arcsec) >= DEADBAND_ARCSEC {
            let target = state.target.as_ref().unwrap();
            let axis_error = self.axis_error(target, time, east_arcsec, north_arcsec);
            for i in 0..2 {
                proportional[i] = -PROPORTIONAL_GAIN * axis_error[i] / interval;
                if elapsed.is_some_and(|dt| dt > 0.0) {
                    // Ki = Kp / Ti. Integrating the proportional rate over the
                    // measured solve interval gives an offset in arcsec/s.
                    state.tracking_offset[i] = (state.tracking_offset[i]
                        + proportional[i] * interval / INTEGRAL_TIME_S)
                        .clamp(-OFFSET_LIMIT, OFFSET_LIMIT);
                }
            }
        }
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
    fn end(&self, reason: String) {
        let mut control = self.control.lock().unwrap();
        let state = &mut control.state;
        if state.state != "off" {
            state.state = "off";
            state.ended_reason = Some(reason);
            state.tracking_offset = [0.0; 2];
        }
        control.proportional = [0.0; 2];
        control.previous_pointing = None;
        for (i, measurement) in control.measurement.iter_mut().enumerate() {
            if measurement.take().is_some() {
                self.model.motion.crossing(i, false);
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

    fn axis_error(&self, target: &Target, time: f64, east: f64, north: f64) -> [f64; 2] {
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
        let (sin_q, cos_q) = q.sin_cos();
        // North-through-east azimuth reverses the equatorial east direction.
        // Wallace (2002): rotate to cross-elevation/elevation, then divide by cos E.
        [
            (-cos_q * east + sin_q * north) / ephemeris::apparent_from_true(alt).to_radians().cos(),
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
        self.model
            .inverse(az, ephemeris::apparent_from_true(alt), counts)
    }

    fn rates_at(&self, target: &Target, counts: [u32; 2], now: f64) -> Result<[f64; 2], String> {
        let positions = [now, now + 1.0].map(|time| {
            let (alt, az) = self.place(target, time);
            self.model
                .inverse(az, ephemeris::apparent_from_true(alt), counts)
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
    ) -> Result<[i32; 2], String> {
        loop {
            let az = client
                .axis_state(Axis::Azimuth)
                .await
                .map_err(|_| "link_lost")?;
            let alt = client
                .axis_state(Axis::Altitude)
                .await
                .map_err(|_| "link_lost")?;
            let positions = [az.position, alt.position];
            if let Some(p) = self.model.predict(positions.map(f64::from), counts) {
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
            for (i, measurement) in control.measurement.iter_mut().enumerate() {
                *measurement = None;
                self.model.motion.crossing(i, true);
            }
        }
        client.stop_all().await.map_err(|_| "link_lost")?;
        let counts = axes.map(|a| a.counts_per_revolution);
        let mut current = self.at_rest(client, counts).await?;
        let exit = self.keyhole_exit(target)?;
        let mut planner = Planner {
            model: &self.model,
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
                self.model
                    .motion
                    .moving(i, f64::from(waypoint[i]) - f64::from(current[i]));
                self.model.motion.crossing(i, true);
                client
                    .goto_axis(axis, waypoint[i])
                    .await
                    .map_err(|_| "mount refused the slew")?;
            }
            current = self.at_rest(client, counts).await?;
        }
        for i in 0..2 {
            self.model.motion.crossing(i, false);
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
                if let Some(b) = self.model.motion.backlash().axis(i) {
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
                if let Err(reason) = self.model.motion.measured(i, degrees) {
                    self.end(reason);
                    return;
                }
                self.model.motion.crossing(i, false);
            }
        }
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
        self.model.motion.moving(i, direction);
        self.model.motion.crossing(i, true);
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
        self.model.motion.moving(i, direction);
        self.model.motion.crossing(i, false);
        Ok(())
    }

    async fn guard(&self, target: &Target, generation: u64) -> String {
        let mut cancel = self.cancel.subscribe();
        let mut tick = interval(TICK);
        tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            if *cancel.borrow_and_update() != generation {
                return "stopped".into();
            }
            if let Some(reason) = self.ending(target) {
                return reason;
            }
            tokio::select! { biased; _ = cancel.changed() => {}, _ = tick.tick() => {} }
        }
    }

    async fn run(
        self: Arc<Self>,
        target: Target,
        client: MountClient,
        axes: [AxisInfo; 2],
        generation: u64,
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
                second.tick().await;
                let now = self.clock.now();
                if now >= refresh_at {
                    keyhole_entry = self.keyhole_entry(&target);
                    forecast = [0, 1].map(|i| self.reversal(&target, counts, i));
                    refresh_at = now + 60.0;
                }
                if keyhole_entry.is_some_and(|entry| now >= entry) {
                    direction = match self.keyhole(&target, &mut movement, axes).await {
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
                    let backlash = self.model.motion.backlash().axis(i);
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
                            self.model.motion.crossing(i, true);
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
                for (i, axis) in AXES.into_iter().enumerate() {
                    // Do not unload a gear on PI noise near a turning point,
                    // or undo a forecast take-up before the sign actually turns.
                    let requested = rates[i] + correction[i];
                    let rate =
                        if rates[i].signum() != direction[i] || requested * direction[i] < 0.0 {
                            0.0
                        } else {
                            requested
                        };
                    self.model.motion.moving(i, rate);
                    if movement.rate_axis(axis, rate).await.is_err() {
                        return "link_lost".into();
                    }
                }
            }
        };
        let reason = tokio::select! {
            biased;
            reason = self.guard(&target, generation) => reason,
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
