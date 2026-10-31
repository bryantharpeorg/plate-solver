//! Desk-only node wiring. Its private pairing session never reads a real node address.
use crate::{
    link::{LinkManager, Transport},
    session::{CameraSettings, CameraSettingsEdit, Clock, NodeEdit, Session, SessionEdit},
};
use pattern_database::{PatternDatabase, StarId};
use serde::{Deserialize, Serialize};
use simulator::{SimNode, Simulator, VirtualClock};
use std::{io::Read, sync::Arc};
use tokio::task::JoinHandle;

/// Production simulation uses random-port loopback TCP; paused tests use duplex IO.
#[derive(Clone, Copy)]
pub enum SimulationTransport {
    /// The CLI's node listens only on 127.0.0.1, on an OS-assigned port.
    Loopback,
    /// Simulator and console share one runtime with no listening socket.
    Memory,
}

/// The console and simulated node share the same real-rate virtual UTC clock.
pub struct ConsoleClock(pub VirtualClock);
impl Clock for ConsoleClock {
    fn now(&self) -> f64 {
        self.0.now()
    }
}

/// Own the service, its pairing and the telemetry/settings bridge together.
pub(crate) struct Simulation {
    pub(crate) simulator: Simulator,
    pub(crate) link: Arc<LinkManager>,
    pub(crate) clock: Arc<ConsoleClock>,
    _node: Arc<SimNode>,
    bridge: JoinHandle<()>,
}
impl Drop for Simulation {
    fn drop(&mut self) {
        self.bridge.abort();
    }
}
impl Simulation {
    pub(crate) async fn start(
        db: Arc<PatternDatabase>,
        session: Arc<Session>,
        start_unix_s: f64,
        mode: SimulationTransport,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let clock = Arc::new(ConsoleClock(VirtualClock::new(start_unix_s)));
        let site =
            session
                .snapshot(clock.now())
                .site
                .map_or_else(simulator::Site::default, |site| simulator::Site {
                    lat_deg: site.lat_deg,
                    lon_deg: site.lon_deg,
                });
        let simulator = Simulator::new(db, site, clock.0.clone());
        // OS entropy without a new dependency. Nothing persists or reports this token.
        let mut random = [0u8; 32];
        std::fs::File::open("/dev/urandom")?.read_exact(&mut random)?;
        let token: String = random.iter().map(|byte| format!("{byte:02x}")).collect();
        let node = Arc::new(SimNode::new(simulator.clone(), &token, Default::default())?);
        let (address, transport) = match mode {
            SimulationTransport::Loopback => {
                (node.listen_loopback().await?.to_string(), Transport::Tcp)
            }
            SimulationTransport::Memory => {
                let service = node.service();
                let transport = Transport::connector(move || {
                    let service = service.clone();
                    async move {
                        let (client, server) = tokio::io::duplex(64 * 1024);
                        tokio::spawn(async move {
                            if let Err(error) =
                                scope_link::memory::serve_on(server, service.into_server()).await
                            {
                                eprintln!("simulator memory transport: {error}");
                            }
                        });
                        Ok::<_, std::io::Error>(client)
                    }
                });
                ("scope-node.memory:1".to_owned(), transport)
            }
        };
        let mut changes = session.link_changes();
        let settings = changes.borrow_and_update().settings.clone();
        let paired = Arc::new(Session::default());
        paired.edit(
            SessionEdit {
                node: Some(Some(NodeEdit {
                    address: Some(address),
                    token: Some(token),
                })),
                camera_settings: Some(settings_edit(settings)),
                ..Default::default()
            },
            clock.now(),
        )?;
        let link = Arc::new(LinkManager::start(paired.clone(), clock.clone(), transport));
        let mut status = link.status();
        let bridge_clock = clock.clone();
        let bridge = tokio::spawn(async move {
            loop {
                tokio::select! {
                    result = status.changed() => {
                        if result.is_err() { return; }
                        session.set_link_status(status.borrow_and_update().clone());
                    }
                    result = changes.changed() => {
                        if result.is_err() { return; }
                        // Forward only camera settings. Address/token never cross this boundary.
                        let settings = changes.borrow_and_update().settings.clone();
                        let _ = paired.edit(SessionEdit {
                            camera_settings: Some(settings_edit(settings)),
                            ..Default::default()
                        }, bridge_clock.now());
                    }
                }
            }
        });
        Ok(Self {
            simulator,
            link,
            clock,
            _node: node,
            bridge,
        })
    }
}

fn settings_edit(settings: CameraSettings) -> CameraSettingsEdit {
    CameraSettingsEdit {
        exposure_ms: Some(settings.exposure_ms),
        gain: Some(settings.gain),
        sigma: Some(settings.sigma),
    }
}

/// Accepted pushes are exclusive and have no unknown fields.
#[derive(Deserialize)]
#[serde(untagged)]
pub(crate) enum Push {
    To(PushTo),
    By(PushBy),
    Star(OnStar),
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PushTo {
    alt_deg: f64,
    az_deg: f64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PushBy {
    turn_deg: f64,
    tilt_deg: f64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OnStar {
    eyepiece_on_star: bool,
}

impl Push {
    pub(crate) fn apply(self, sim: &Simulator, db: &PatternDatabase) -> Result<(), &'static str> {
        match self {
            Self::To(push) => {
                if !push.alt_deg.is_finite()
                    || push.alt_deg.abs() > 90.0
                    || !push.az_deg.is_finite()
                {
                    return Err("alt_deg must be within -90..90 and az_deg must be finite");
                }
                sim.push_to(push.alt_deg, push.az_deg);
            }
            Self::By(push) => {
                if !push.turn_deg.is_finite() || !push.tilt_deg.is_finite() {
                    return Err("turn_deg and tilt_deg must be finite");
                }
                sim.push_by(push.turn_deg, push.tilt_deg);
            }
            Self::Star(push) => {
                if !push.eyepiece_on_star {
                    return Err("eyepiece_on_star must be true");
                }
                let star = sim
                    .field_stars(200.0, 1.0)
                    .first()
                    .ok_or("no database star in the field")?
                    .0;
                let vector = db
                    .star_vector(StarId(star))
                    .and_then(|v| v.normalize())
                    .ok_or("invalid database star")?;
                let (ra, dec) = vector.to_radec();
                sim.point_eyepiece_at(ra.to_degrees(), dec.to_degrees());
            }
        }
        Ok(())
    }
}

/// JSON boundary for simulator truth; the simulator crate stays independent of serde.
#[derive(Serialize)]
pub(crate) struct Telescope {
    camera: Direction,
    eyepiece: Direction,
}
#[derive(Serialize)]
struct Direction {
    alt: f64,
    az: f64,
    ra: f64,
    dec: f64,
}
impl From<simulator::TrueDirection> for Direction {
    fn from(d: simulator::TrueDirection) -> Self {
        Self {
            alt: d.alt,
            az: d.az,
            ra: d.ra,
            dec: d.dec,
        }
    }
}
impl From<simulator::TrueDirections> for Telescope {
    fn from(d: simulator::TrueDirections) -> Self {
        Self {
            camera: d.camera.into(),
            eyepiece: d.eyepiece.into(),
        }
    }
}

/// Parse the CLI's UTC form without a time-library dependency.
pub fn parse_start(value: &str) -> Result<f64, &'static str> {
    let error = "--sim-start requires YYYY-MM-DDTHH:MM:SSZ (optional fractional seconds)";
    let bytes = value.as_bytes();
    if bytes.len() < 20
        || !value.is_ascii()
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes.last() != Some(&b'Z')
    {
        return Err(error);
    }
    let number = |range: std::ops::Range<usize>| -> Result<i64, &'static str> {
        let s = &value[range];
        if !s.bytes().all(|b| b.is_ascii_digit()) {
            return Err(error);
        }
        s.parse().map_err(|_| error)
    };
    let (year, month, day) = (number(0..4)?, number(5..7)?, number(8..10)?);
    let (hour, minute, second) = (number(11..13)?, number(14..16)?, number(17..19)?);
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if leap {
                29
            } else {
                28
            }
        }
        _ => return Err(error),
    };
    if day < 1 || day > days || hour > 23 || minute > 59 || second > 59 {
        return Err(error);
    }
    let fraction = if bytes.len() == 20 {
        0.0
    } else {
        if bytes[19] != b'.'
            || bytes.len() == 21
            || !bytes[20..bytes.len() - 1].iter().all(u8::is_ascii_digit)
        {
            return Err(error);
        }
        value[19..value.len() - 1]
            .parse::<f64>()
            .map_err(|_| error)?
    };
    // Gregorian days from civil date, with 1970-01-01 as day zero.
    let y = year - i64::from(month <= 2);
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let m = month + if month > 2 { -3 } else { 9 };
    let doy = (153 * m + 2) / 5 + day - 1;
    let days = era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468;
    Ok((days * 86400 + hour * 3600 + minute * 60 + second) as f64 + fraction)
}

#[cfg(test)]
mod tests {
    use super::parse_start;
    #[test]
    fn utc_parser_matches_worked_unix_epochs_and_refuses_invalid_dates() {
        for (value, expected) in [
            ("1970-01-01T00:00:00Z", 0.0),
            ("1969-12-31T23:59:59.5Z", -0.5),
            ("2000-02-29T00:00:00Z", 951782400.0),
            ("2026-01-16T02:00:00.125Z", 1768528800.125),
        ] {
            assert_eq!(parse_start(value).unwrap(), expected);
        }
        for value in [
            "",
            "2026-01-16T02:00:00+00:00",
            "2026-02-29T00:00:00Z",
            "1900-02-29T00:00:00Z",
            "2026-04-31T00:00:00Z",
            "2026-00-01T00:00:00Z",
            "2026-01-00T00:00:00Z",
            "2026-01-01T24:00:00Z",
            "2026-01-01T00:60:00Z",
            "2026-01-01T00:00:60Z",
            "2026-01-01T00:00:00.Z",
            "2026-01-01T00:00:00.1xZ",
            "2026-01-01T00:00:00.1.2Z",
            "2026-01-01T00:00:00.éZ",
        ] {
            assert!(parse_start(value).is_err(), "{value}");
        }
    }
}
