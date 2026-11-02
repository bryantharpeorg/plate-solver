//! Safe GoTo planning and the solve/correct loop.

use crate::{
    ephemeris,
    link::{LinkManager, MountClient},
    live::Live,
    model::{separation, Model},
    session::{Clock, Session, Site},
    sky::{self, Observer},
    targets::Target,
};
use scope_link::proto::{self, Axis};
use serde::Serialize;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    sync::{watch, Mutex as AsyncMutex},
    task::JoinHandle,
};

const APPROACH: f64 = 0.5;
const ACCELERATION: f64 = 3.0;
const TICK: Duration = Duration::from_millis(50);
const AXES: [Axis; 2] = [Axis::Azimuth, Axis::Altitude];
fn wrap(angle: f64) -> f64 {
    (angle + 180.0).rem_euclid(360.0) - 180.0
}

/// Continuous telescope azimuth, independent of counts and zero-point epochs.
#[derive(Clone, Copy, Default, Debug)]
pub struct Wind {
    last: Option<f64>,
    /// Degrees turned since the night's first sync, clockwise positive.
    pub travel: f64,
}
impl Wind {
    /// Observe a telescope direction less than half a revolution from the last.
    pub fn observe(&mut self, az: f64) {
        if let Some(last) = self.last {
            self.travel += wrap(az - last);
        }
        self.last = Some(az);
    }
}

/// Sun centre's geometric (altitude, azimuth), degrees, at supplied Unix UT.
/// Meeus ch. 13 and 25: solar coordinates already refer to the equinox of date.
pub fn sun_horizontal(lat: f64, lon: f64, time: f64) -> (f64, f64) {
    let jd = sky::julian_date(time);
    let (ra, dec) = ephemeris::sun(jd);
    let h = (sky::gmst_deg(jd) + lon - ra).to_radians();
    let (d, l) = (dec.to_radians(), lat.to_radians());
    let alt = (l.sin() * d.sin() + l.cos() * d.cos() * h.cos()).asin();
    let az = (-h.sin() * d.cos()).atan2(l.cos() * d.sin() - l.sin() * d.cos() * h.cos());
    (alt.to_degrees(), az.to_degrees().rem_euclid(360.0))
}

/// A path of stopped waypoints in absolute counts, including the final approach.
#[derive(Clone, Debug)]
pub struct Plan {
    /// Each pair can be sent simultaneously; wait for both axes before the next.
    pub waypoints: Vec<[i32; 2]>,
    /// Final aim point in counts.
    pub destination: [i32; 2],
    /// Console time at which the target's apparent place was evaluated.
    pub arrival: f64,
    /// Estimated motion duration, including acceleration and the last approach.
    pub duration: f64,
}
/// Inputs are snapshots; planning never moves an axis.
pub struct Planner<'a> {
    /// Current nightly model.
    pub model: &'a Model,
    /// Reported mount capabilities, azimuth then altitude.
    pub axes: [proto::AxisInfo; 2],
    /// Current absolute counts, both axes stopped.
    pub current: [i32; 2],
    /// Continuously observed cable wind.
    pub wind: Wind,
    /// Observing site.
    pub site: Site,
    /// Planning time, Unix seconds.
    pub start: f64,
    /// Minimum geometric altitude, degrees.
    pub min_alt: f64,
}
// Rest-to-rest trapezoid, as specified for the mount. The hardware's goto uses
// its reported top speed. Sample each axis independently, including acceleration.
fn travel_time(distance: f64, rate: f64) -> f64 {
    let distance = distance.abs();
    if distance <= rate * rate / ACCELERATION {
        2.0 * (distance / ACCELERATION).sqrt()
    } else {
        distance / rate + rate / ACCELERATION
    }
}
fn travelled(distance: f64, rate: f64, time: f64) -> f64 {
    let total = travel_time(distance, rate);
    let peak = rate.min((distance.abs() * ACCELERATION).sqrt());
    let ramp = peak / ACCELERATION;
    let d = if time >= total {
        distance.abs()
    } else if time < ramp {
        ACCELERATION * time * time / 2.0
    } else if time <= total - ramp {
        peak * (time - ramp / 2.0)
    } else {
        distance.abs() - ACCELERATION * (total - time).powi(2) / 2.0
    };
    distance.signum() * d
}
impl Planner<'_> {
    fn counts(&self) -> [u32; 2] {
        self.axes.each_ref().map(|a| a.counts_per_revolution)
    }
    fn degrees(&self, counts: [i32; 2]) -> [f64; 2] {
        [0, 1].map(|i| f64::from(counts[i]) * 360.0 / f64::from(self.counts()[i]))
    }
    fn encode(&self, degrees: [f64; 2]) -> Result<[i32; 2], String> {
        let values = [0, 1].map(|i| (degrees[i] * f64::from(self.counts()[i]) / 360.0).round());
        if values
            .iter()
            .any(|v| !v.is_finite() || *v < f64::from(i32::MIN) || *v > f64::from(i32::MAX))
        {
            return Err("axis position is outside the mount's range".into());
        }
        Ok(values.map(|v| v as i32))
    }
    fn predict(&self, degrees: [f64; 2]) -> Result<(f64, f64), String> {
        let p = self
            .model
            .predict(
                [0, 1].map(|i| degrees[i] * f64::from(self.counts()[i]) / 360.0),
                self.counts(),
            )
            .ok_or("the night has no sync yet")?;
        Ok((ephemeris::true_from_apparent(p.alt_deg), p.az_deg))
    }
    /// Two arrival-time estimates, then the apparent place and model inverse.
    pub fn plan(&self, target: &Target) -> Result<Plan, String> {
        self.plan_place(|time| {
            sky::alt_az(
                target.ra_deg,
                target.dec_deg,
                &Observer {
                    lat: self.site.lat_deg,
                    lon: self.site.lon_deg,
                    time,
                },
            )
        })
    }
    fn plan_place(&self, place: impl Fn(f64) -> (f64, f64)) -> Result<Plan, String> {
        if self.axes.iter().any(|a| {
            a.counts_per_revolution == 0
                || !a.can_goto
                || !a.top_rate_deg_per_s.is_finite()
                || a.top_rate_deg_per_s <= 0.0
        }) {
            return Err("no mount capable of positioning both axes is connected".into());
        }
        if sun_horizontal(self.site.lat_deg, self.site.lon_deg, self.start).0 >= -0.83 {
            return Err("Sun is up".into());
        }
        let mut estimate = 0.0;
        for iteration in 0..=2 {
            let arrival = self.start + estimate;
            let (alt, az) = place(arrival);
            if alt < self.min_alt {
                return Err("target is below the minimum altitude at arrival".into());
            }
            let axis = self
                .model
                .inverse(az, ephemeris::apparent_from_true(alt), self.counts())
                .ok_or("the night has no sync yet")?;
            let axis = [0, 1].map(|i| axis[i] * 360.0 / f64::from(self.counts()[i]));
            let current = self.degrees(self.current);
            let current_sky = self.predict(current)?;
            let nearest = ((current[0] - axis[0]) / 360.0).round();
            let mut candidates: Vec<_> = (-2..=2)
                .map(|k| [axis[0] + (nearest + f64::from(k)) * 360.0, axis[1]])
                .filter(|candidate| {
                    let turn = candidate[0] - current[0];
                    let wind = self.wind.travel + turn + wrap(az - current_sky.1 - turn);
                    (-360.0..=360.0).contains(&wind)
                })
                .collect();
            candidates.sort_by(|a, b| {
                (a[0] - current[0])
                    .abs()
                    .total_cmp(&(b[0] - current[0]).abs())
            });
            let mut chosen = None;
            for destination in candidates {
                let pre = destination.map(|d| d - APPROACH);
                if self.predict(pre)?.0 < self.min_alt {
                    return Err("approach would go below the minimum altitude".into());
                }
                let high_sky = current_sky
                    .0
                    .max(alt)
                    .max(sun_horizontal(self.site.lat_deg, self.site.lon_deg, self.start).0 + 20.0);
                let high = self
                    .model
                    .inverse(
                        current_sky.1,
                        ephemeris::apparent_from_true(high_sky.min(89.0)),
                        self.counts(),
                    )
                    .ok_or("cannot plan the high altitude path")?[1]
                    * 360.0
                    / f64::from(self.counts()[1]);
                // A high leg must stay high for the full azimuth turn, including
                // the fitted base tilt. Add its full possible vertical excursion.
                let high = high.max(current[1]).max(destination[1])
                    + 2.0 * self.model.base_tilt.map_or(0.0, |b| b.deg());
                let paths = [
                    vec![pre, destination],
                    vec![[current[0], pre[1]], pre, destination],
                    vec![[pre[0], current[1]], pre, destination],
                    vec![[current[0], high], [pre[0], high], pre, destination],
                ];
                for path in paths {
                    let waypoints = path
                        .into_iter()
                        .map(|p| self.encode(p))
                        .collect::<Result<Vec<_>, _>>()?;
                    if let Some(duration) = self.safe_path(&waypoints)? {
                        chosen = Some(Plan {
                            destination: self.encode(destination)?,
                            waypoints,
                            duration,
                            arrival,
                        });
                        break;
                    }
                }
                if chosen.is_some() {
                    break;
                }
            }
            let plan = chosen
                .ok_or("no path keeps clear of the Sun guard, minimum altitude and cable wind")?;
            if iteration == 2 {
                return Ok(plan);
            }
            estimate = plan.duration;
        }
        unreachable!()
    }
    fn safe_path(&self, waypoints: &[[i32; 2]]) -> Result<Option<f64>, String> {
        let initial = self.degrees(self.current);
        let mut from = initial;
        let mut previous = self.predict(from)?;
        let initial_az = previous.1;
        let mut below = previous.0 < self.min_alt;
        let mut elapsed = 0.0;
        for &waypoint in waypoints {
            let to = self.degrees(waypoint);
            let distances = [0, 1].map(|i| to[i] - from[i]);
            let duration = (0..2)
                .map(|i| travel_time(distances[i], self.axes[i].top_rate_deg_per_s))
                .fold(0.0, f64::max);
            let steps = (duration
                * self
                    .axes
                    .iter()
                    .map(|a| a.top_rate_deg_per_s)
                    .fold(0.0, f64::max)
                / 0.5)
                .ceil()
                .max(1.0) as usize;
            for step in 0..=steps {
                let time = duration * step as f64 / steps as f64;
                let axis = [0, 1].map(|i| {
                    from[i] + travelled(distances[i], self.axes[i].top_rate_deg_per_s, time)
                });
                let sky = self.predict(axis)?;
                // A telescope already below the minimum can only lift to safety.
                // No azimuth turn is permitted until it has cleared the minimum.
                if sky.0 < self.min_alt {
                    if !below || distances[0].abs() > 1e-9 || sky.0 < previous.0 - 1e-6 {
                        return Ok(None);
                    }
                } else {
                    below = false;
                }
                let turn = axis[0] - initial[0];
                let wind = self.wind.travel + turn + wrap(sky.1 - initial_az - turn);
                if wind.abs() > 360.0 + 1e-8 {
                    return Ok(None);
                }
                let sun = sun_horizontal(
                    self.site.lat_deg,
                    self.site.lon_deg,
                    self.start + elapsed + time,
                );
                if separation(sky.1, sky.0, sun.1, sun.0) <= 15.0 {
                    return Ok(None);
                }
                previous = sky;
            }
            from = to;
            elapsed += duration;
        }
        Ok(Some(elapsed))
    }
}

/// GoTo progress retained in the session until the next requested GoTo.
#[derive(Clone, Debug, Serialize)]
pub struct Progress {
    /// `slewing`, `solving`, `correcting`, `done`, `failed`, or `halted`.
    /// Solving includes the settle wait before the first eligible exposure.
    pub state: &'static str,
    /// Number of correction slews started (zero for the initial slew).
    pub correction: u8,
    /// Last solved separation at the aim point, in arcseconds.
    pub error_arcsec: Option<f64>,
    /// Terminal failure or halt reason.
    pub reason: Option<String>,
}
#[derive(Default)]
struct State {
    progress: Option<Progress>,
    busy: bool,
    generation: u64,
    wind: Wind,
    wind_time: f64,
    night: Option<f64>,
}
/// One session's motion owner. Dropping it cancels its observation task.
pub struct GoTo {
    session: Arc<Session>,
    clock: Arc<dyn Clock>,
    link: Arc<LinkManager>,
    live: Arc<Live>,
    state: Arc<Mutex<State>>,
    cancel: watch::Sender<u64>,
    start_gate: AsyncMutex<()>,
    monitor: JoinHandle<()>,
}
impl Drop for GoTo {
    fn drop(&mut self) {
        self.monitor.abort();
    }
}
impl GoTo {
    /// Observe solved telescope azimuth continuously, across syncs and epochs.
    pub fn new(
        session: Arc<Session>,
        clock: Arc<dyn Clock>,
        link: Arc<LinkManager>,
        live: Arc<Live>,
    ) -> Self {
        let state = Arc::new(Mutex::new(State::default()));
        let observed = state.clone();
        let mut reports = live.subscribe();
        let sync_session = session.clone();
        let monitor = tokio::spawn(async move {
            loop {
                let report = match reports.recv().await {
                    Ok(r) => r,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => return,
                };
                if report.pointing_seq.is_none() {
                    continue;
                }
                let model = sync_session.model(report.time);
                let Some(first) = model.syncs.first() else {
                    continue;
                };
                let pointing = sync_session.snapshot(report.time).pointing;
                let Some(az) = pointing.and_then(|p| p.az_deg) else {
                    continue;
                };
                let mut state = observed.lock().unwrap();
                if state.night != model.expires_at {
                    state.night = model.expires_at;
                    state.wind = Wind::default();
                    state.wind.observe(first.az_deg);
                    state.wind_time = first.time;
                }
                if report.time >= state.wind_time {
                    state.wind.observe(az);
                    state.wind_time = report.time;
                }
            }
        });
        Self {
            session,
            clock,
            link,
            live,
            state,
            cancel: watch::channel(0).0,
            start_gate: AsyncMutex::new(()),
            monitor,
        }
    }
    /// Latest progress, absent before the first request.
    pub fn progress(&self) -> Option<Progress> {
        self.state.lock().unwrap().progress.clone()
    }
    /// Start toward the session's canonical target, refusing before any movement.
    pub async fn start(self: &Arc<Self>) -> Result<(), String> {
        let target = self
            .session
            .snapshot(self.clock.now())
            .target
            .ok_or("no target is selected")?
            .clone();
        self.start_target(target).await
    }
    /// Plan a supplied celestial direction through the same mandatory guards.
    /// Also supports the second sync's chosen spot without changing the target.
    pub async fn start_target(self: &Arc<Self>, target: Target) -> Result<(), String> {
        let _gate = self.start_gate.lock().await;
        if self.state.lock().unwrap().busy {
            return Err("a GoTo is already running".into());
        }
        let generation = *self.cancel.borrow();
        let mut client = self.link.mount().ok_or("no mount is connected")?;
        let plan = tokio::time::timeout(
            Duration::from_secs(1),
            self.prepare(&target, &mut client, false),
        )
        .await
        .map_err(|_| "mount did not answer")??;
        if *self.cancel.borrow() != generation {
            return Err("the observer stopped it".into());
        }
        {
            let mut state = self.state.lock().unwrap();
            state.busy = true;
            state.generation = generation;
            state.progress = Some(Progress {
                state: "slewing",
                correction: 0,
                error_arcsec: None,
                reason: None,
            });
        }
        let owner = self.clone();
        tokio::spawn(async move {
            owner.run(target, client, plan.0, generation).await;
        });
        Ok(())
    }
    async fn prepare(
        &self,
        target: &Target,
        client: &mut MountClient,
        anchor: bool,
    ) -> Result<(Plan, [proto::AxisInfo; 2]), String> {
        let now = self.clock.now();
        let view = self.session.snapshot(now);
        if !self.link.status().borrow().connected {
            return Err("no mount is connected".into());
        }
        let axes: [proto::AxisInfo; 2] = client
            .describe_mount()
            .await
            .map_err(|_| "no mount is connected")?
            .axes
            .try_into()
            .map_err(|_| "no mount is connected")?;
        let mut model = self.session.model(now);
        if model.syncs.is_empty() {
            return Err("the night has no sync yet".into());
        }
        // A centring correction uses the fresh sync as a local zero-point
        // measurement. The global fit retains all nightly evidence; anchoring
        // its zero points here prevents older residuals from diluting the loop.
        if anchor {
            let sync = model.syncs.last().unwrap().clone();
            for _ in 0..4 {
                let expected = model
                    .inverse(sync.az_deg, sync.alt_deg, sync.counts_per_revolution)
                    .ok_or("cannot refine the sync zero points")?;
                let epoch = model.epochs.last_mut().unwrap();
                epoch.ia +=
                    (sync.axis[0] - expected[0]) * 360.0 / f64::from(sync.counts_per_revolution[0]);
                epoch.ie +=
                    (expected[1] - sync.axis[1]) * 360.0 / f64::from(sync.counts_per_revolution[1]);
            }
        }
        let site = view.site.ok_or("a site is needed")?;
        if sun_horizontal(site.lat_deg, site.lon_deg, now).0 >= -0.83 {
            return Err("Sun is up".into());
        }
        let name = target.messier.as_ref().unwrap_or(&target.id);
        match crate::pushto::class(target, Some(site), view.min_alt_deg, now) {
            crate::pushto::Class::Pointable => {}
            crate::pushto::Class::BelowHorizon => {
                return Err(format!("{name} is below the horizon"))
            }
            crate::pushto::Class::TooLow => {
                return Err(format!("{name} is below the minimum altitude"))
            }
            _ => return Err(format!("{name} is not pointable: Sun guard")),
        }
        let mut current = [0; 2];
        for (i, axis) in AXES.into_iter().enumerate() {
            let reading = client
                .axis_state(axis)
                .await
                .map_err(|_| "mount did not answer")?;
            if reading.moving {
                return Err("the mount is already moving".into());
            }
            current[i] = reading.position;
        }
        let wind = {
            let mut state = self.state.lock().unwrap();
            if state.night != model.expires_at || state.wind.last.is_none() {
                state.night = model.expires_at;
                state.wind = Wind::default();
                state.wind.observe(model.syncs[0].az_deg);
            }
            if let Some(p) = model.predict(
                current.map(f64::from),
                axes.each_ref().map(|a| a.counts_per_revolution),
            ) {
                state.wind.observe(p.az_deg);
                state.wind_time = now;
            }
            state.wind
        };
        let planner = Planner {
            model: &model,
            axes,
            current,
            wind,
            site,
            start: now,
            min_alt: view.min_alt_deg,
        };
        Ok((planner.plan(target)?, axes))
    }
    fn update(
        &self,
        generation: u64,
        phase: &'static str,
        correction: u8,
        error: Option<f64>,
        reason: Option<String>,
    ) {
        let mut state = self.state.lock().unwrap();
        if state.busy && state.generation == generation && *self.cancel.borrow() == generation {
            state.progress = Some(Progress {
                state: phase,
                correction,
                error_arcsec: error,
                reason,
            });
        }
    }
    /// Stop bypasses planning and solving, and is safe even without a running GoTo.
    pub async fn stop(&self) {
        self.cancel.send_modify(|n| *n = n.wrapping_add(1));
        {
            let mut state = self.state.lock().unwrap();
            if state.busy {
                if let Some(progress) = &mut state.progress {
                    progress.state = "halted";
                    progress.reason = Some("the observer stopped it".into());
                }
            }
        }
        if let Some(mut client) = self.link.mount() {
            let _ = tokio::time::timeout(Duration::from_millis(200), client.stop_all()).await;
        }
    }
    async fn guard(&self, generation: u64) -> String {
        let mut cancel = self.cancel.subscribe();
        let mut ticks = tokio::time::interval(TICK);
        loop {
            if *cancel.borrow_and_update() != generation {
                return "the observer stopped it".into();
            }
            let status = self.link.status().borrow().clone();
            if !status.connected
                || status
                    .last_heartbeat_at
                    .is_none_or(|t| t.elapsed() >= Duration::from_secs(1))
            {
                return "halted by a lost link".into();
            }
            if let Some(site) = self.session.snapshot(self.clock.now()).site {
                if sun_horizontal(site.lat_deg, site.lon_deg, self.clock.now()).0 >= -0.83 {
                    return "halted by the Sun guard".into();
                }
            }
            tokio::select! { _ = ticks.tick() => {}, _ = cancel.changed() => {} }
        }
    }
    async fn run(
        self: Arc<Self>,
        target: Target,
        mut client: MountClient,
        plan: Plan,
        generation: u64,
    ) {
        let mut movement = client.clone();
        let result = tokio::select! {
            biased;
            reason = self.guard(generation) => Err((true, reason)),
            result = self.motion(&target, &mut movement, plan, generation) => result.map_err(|reason| (false, reason)),
        };
        if let Err((halted, reason)) = result {
            let _ = tokio::time::timeout(Duration::from_millis(200), client.stop_all()).await;
            let progress = self.progress().unwrap();
            self.update(
                generation,
                if halted { "halted" } else { "failed" },
                progress.correction,
                progress.error_arcsec,
                Some(reason),
            );
        }
        let mut state = self.state.lock().unwrap();
        if state.generation == generation {
            state.busy = false;
        }
    }
    async fn motion(
        &self,
        target: &Target,
        client: &mut MountClient,
        mut plan: Plan,
        generation: u64,
    ) -> Result<(), String> {
        let mut reports = self.live.subscribe();
        let mut correction = 0;
        let mut error = None;
        loop {
            self.update(generation, "slewing", correction, error, None);
            for waypoint in &plan.waypoints {
                for (i, axis) in AXES.into_iter().enumerate() {
                    client
                        .goto_axis(axis, waypoint[i])
                        .await
                        .map_err(|_| "mount refused the slew")?;
                }
                loop {
                    let az = client
                        .axis_state(Axis::Azimuth)
                        .await
                        .map_err(|_| "mount did not answer")?;
                    let alt = client
                        .axis_state(Axis::Altitude)
                        .await
                        .map_err(|_| "mount did not answer")?;
                    if !az.moving && !alt.moving {
                        break;
                    }
                    tokio::time::sleep(TICK).await;
                }
            }
            self.update(generation, "solving", correction, error, None);
            tokio::time::sleep(Duration::from_millis(500)).await;
            let settled = self.clock.now();
            // Identify the first exposure STARTING after settle, not a late solve
            // of an exposure made during motion. The live loop takes the sync.
            let solve = async {
                let mut frames = self.link.frames();
                let first_id = loop {
                    let status = self.link.status().borrow().clone();
                    let frame = frames.borrow_and_update().clone();
                    if let Some(frame) = frame {
                        if status
                            .offset_s
                            .is_some_and(|offset| frame.start_s - offset >= settled)
                        {
                            break frame.id;
                        }
                    }
                    frames.changed().await.map_err(|_| "scope link ended")?;
                };
                loop {
                    let report = match reports.recv().await {
                        Ok(r) => r,
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(_) => return Err("live solving ended"),
                    };
                    if report.frame_id >= first_id && report.pointing_seq.is_some() {
                        let view = self.session.snapshot(report.time);
                        let pointing = view.pointing.ok_or("solve did not yield an aim point")?;
                        if pointing.frame_id != Some(report.frame_id) {
                            continue;
                        }
                        return Ok(separation(
                            pointing.ra_deg,
                            pointing.dec_deg,
                            target.ra_deg,
                            target.dec_deg,
                        ) * 3600.0);
                    }
                }
            };
            error = Some(
                tokio::time::timeout(Duration::from_secs(30), solve)
                    .await
                    .map_err(|_| "no frame solved after settling")??,
            );
            self.update(generation, "solving", correction, error, None);
            if error.unwrap() < 60.0 {
                self.update(generation, "done", correction, error, None);
                return Ok(());
            }
            if correction == 5 {
                return Err("failed after 5 corrections".into());
            }
            correction += 1;
            self.update(generation, "correcting", correction, error, None);
            // Retain the numbered correction long enough for session readers.
            tokio::time::sleep(Duration::from_millis(100)).await;
            plan = self.prepare(target, client, true).await?.0;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Epoch, Sync};

    fn model() -> Model {
        let mut model = Model::default();
        model.add_sync(Sync {
            time: 0.0,
            axis: [0.0, 45.0],
            counts_per_revolution: [360; 2],
            az_deg: 0.0,
            alt_deg: 45.0,
            roll_deg: None,
            rms_arcsec: 1.0,
            epoch: 0,
        });
        model
    }
    fn planner(model: &Model, current: [i32; 2], wind: Wind) -> Planner<'_> {
        Planner {
            model,
            current,
            wind,
            axes: [0, 1].map(|_| proto::AxisInfo {
                counts_per_revolution: 360,
                top_rate_deg_per_s: 3.4,
                can_goto: true,
            }),
            site: Site {
                lat_deg: 40.0,
                lon_deg: -75.0,
            },
            start: 1768528800.0,
            min_alt: 15.0,
        }
    }
    // Oracle: 350 + 30 exceeds +360; the equivalent turn is 30 - 360 = -330.
    #[test]
    fn cable_unwinds_after_three_hundred_fifty_degrees() {
        let model = model();
        let mut wind = Wind::default();
        for az in [0.0, 90.0, 180.0, 270.0, 350.0] {
            wind.observe(az);
        }
        let p = planner(&model, [350, 45], wind);
        let plan = p.plan_place(|_| (45.0, 20.0)).unwrap();
        assert_eq!(plan.destination[0] - 350, -330);
    }
    #[test]
    fn cable_wind_survives_a_power_cycle_and_zero_point_epoch() {
        let mut model = model();
        let mut wind = Wind::default();
        for az in [0.0, 90.0, 180.0, 270.0, 350.0] {
            wind.observe(az);
        }
        // Power cycle resets counts, while a new sync sees the same telescope azimuth.
        model.epochs.push(Epoch { ia: 10.0, ie: 0.0 });
        wind.observe(350.0);
        let p = planner(&model, [0, 45], wind);
        let plan = p.plan_place(|_| (45.0, 20.0)).unwrap();
        assert_eq!(plan.destination[0], -330);
        assert_eq!(wind.travel, 350.0);
    }
    // Oracle: independently integrate a 3.4 deg/s, 3 deg/s² trapezoid,
    // including the 0.5-degree approach. The selected distance makes 40 s.
    #[test]
    fn apparent_place_is_evaluated_at_arrival_within_one_second() {
        let model = model();
        let mut p = planner(&model, [0, 45 * 4500], Wind::default());
        for axis in &mut p.axes {
            axis.counts_per_revolution = 1_620_000;
        }
        let distance = (40.0 - 2.0 * (0.5_f64 / 3.0).sqrt() - 3.4 / 3.0) * 3.4 + 0.5;
        let plan = p
            .plan_place(|time| (45.0, distance + (time - p.start - 40.0) / 240.0))
            .unwrap();
        assert!((plan.arrival - p.start - 40.0).abs() < 1.0);
        let expected = model
            .inverse(
                distance,
                ephemeris::apparent_from_true(45.0),
                [1_620_000; 2],
            )
            .unwrap();
        for (actual, expected) in plan.destination.into_iter().zip(expected) {
            assert!((f64::from(actual) - expected).abs() <= 1.0);
        }
    }
}
