//! Feed-forward tracking through the pointing model, with latched safe endings.
//!
//! The caller centres the target before starting. All UTC comes from the console's
//! injected clock; timers and heartbeat ages use tokio's (pausable) clock.

use crate::{
    ephemeris,
    goto::sun_horizontal,
    link::{LinkManager, MountClient},
    model::Model,
    session::{Clock, Site},
    sky::{self, Observer},
    targets::Target,
};
use scope_link::proto::Axis;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    sync::{watch, Mutex as AsyncMutex},
    task::JoinHandle,
    time::{interval, timeout, MissedTickBehavior},
};

const TICK: Duration = Duration::from_millis(50);
const SECOND: Duration = Duration::from_secs(1);
const STOP_TIMEOUT: Duration = Duration::from_millis(200);
const AXES: [Axis; 2] = [Axis::Azimuth, Axis::Altitude];

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
    /// `off` or `tracking` (the keyhole extension may also report `keyhole_wait`).
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

struct Shared {
    clock: Arc<dyn Clock>,
    link: Arc<LinkManager>,
    model: Model,
    site: Site,
    min_alt_deg: f64,
    state: Mutex<State>,
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
        Self {
            shared: Arc::new(Shared {
                clock,
                link,
                model,
                site,
                min_alt_deg,
                state: Mutex::new(State::default()),
                cancel: watch::channel(0).0,
            }),
            task: AsyncMutex::new(None),
        }
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
        let counts = [axes[0].counts_per_revolution, axes[1].counts_per_revolution];
        self.shared.rates(&target, counts)?;
        // Stop may have arrived while DescribeMount or a prior stop was pending.
        // The version and publication share the state lock with end().
        {
            let mut state = self.shared.state.lock().unwrap();
            if *self.shared.cancel.borrow() != generation {
                return Err("stopped".into());
            }
            *state = State {
                state: "tracking",
                target: Some(target.clone()),
                ..State::default()
            };
        }
        let shared = self.shared.clone();
        *task = Some(tokio::spawn(async move {
            shared.run(target, client, counts, generation).await;
        }));
        Ok(())
    }

    /// Record the reason and cancel the rate loop before sending StopAll. Wait
    /// for that run's bounded stop attempt; a lost link's watchdog is independent.
    pub async fn stop(&self, reason: impl Into<String>) {
        let reason = reason.into();
        self.shared.end(reason.clone());
        let mut task = self.task.lock().await;
        // A concurrent start may have acquired the lifecycle lock first. Cancel
        // that run too before joining it, so Stop cannot wait on fresh tracking.
        self.shared.end(reason);
        if let Some(previous) = task.take() {
            let _ = previous.await;
        } else if let Some(mut client) = self.shared.link.mount() {
            let _ = timeout(STOP_TIMEOUT, client.stop_all()).await;
        }
    }

    /// Latest state, with error samples aged using the console's clock.
    pub fn state(&self) -> State {
        let mut state = self.shared.state.lock().unwrap();
        let now = self.shared.clock.now();
        state.samples.retain(|s| now - s.time <= 60.0);
        state.clone()
    }

    /// Feed a live solve's aim-point error (or simulator truth plus noise).
    /// US3 records the readout samples; the closed-loop extension uses this same
    /// entry point to resolve the error into axis rate corrections.
    pub fn pointing(&self, east_arcsec: f64, north_arcsec: f64) {
        let mut state = self.shared.state.lock().unwrap();
        if state.state == "off" || !east_arcsec.is_finite() || !north_arcsec.is_finite() {
            return;
        }
        let time = self.shared.clock.now();
        state.samples.retain(|s| time - s.time <= 60.0);
        state.samples.push(PointingError {
            time,
            east_arcsec,
            north_arcsec,
        });
    }
}

impl Shared {
    fn end(&self, reason: String) {
        let mut state = self.state.lock().unwrap();
        if state.state != "off" {
            state.state = "off";
            state.ended_reason = Some(reason);
            state.tracking_offset = [0.0; 2];
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
        if self.place(target, now).0 < self.min_alt_deg {
            return Some("too_low".into());
        }
        None
    }

    fn rates(&self, target: &Target, counts: [u32; 2]) -> Result<[f64; 2], String> {
        let now = self.clock.now();
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
        mut client: MountClient,
        counts: [u32; 2],
        generation: u64,
    ) {
        let mut movement = client.clone();
        let rates = async {
            let mut second = interval(SECOND);
            second.set_missed_tick_behavior(MissedTickBehavior::Skip);
            loop {
                second.tick().await;
                let rates = match self.rates(&target, counts) {
                    Ok(rates) => rates,
                    Err(reason) => return reason,
                };
                for (axis, rate) in AXES.into_iter().zip(rates) {
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
        let _ = timeout(STOP_TIMEOUT, client.stop_all()).await;
    }
}
