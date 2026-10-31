//! The shared observing session and the console's injectable clock.

use std::fmt;
use std::fs::{self, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::link::{self, LinkStatus, OutOfRange};
use scope_link::proto::CameraInfo;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::watch;

use serde::{Deserialize, Deserializer, Serialize};

use crate::pushto::{self, Guidance};
use crate::sky::{self, Frame, Observer};
use crate::targets::{self, Target};

/// The console's time source, in Unix seconds. Each application owns its clock.
pub trait Clock: Send + Sync {
    /// Read the console's time.
    fn now(&self) -> f64;
}

/// The real system clock, used by the binary and by default in the library.
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> f64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or_else(|e| -e.duration().as_secs_f64(), |d| d.as_secs_f64())
    }
}

/// A fixed Unix time for tests and simulations. No binary option constructs one.
pub struct FixedClock(pub f64);

impl Clock for FixedClock {
    fn now(&self) -> f64 {
        self.0
    }
}

/// Where the telescope stands, in degrees (north and east positive).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Site {
    /// Latitude within ±90°.
    pub lat_deg: f64,
    /// Longitude within ±180°.
    pub lon_deg: f64,
}

/// A camera control is automatic or a positive numeric value.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CameraValue {
    /// Let the node choose the value.
    Automatic(Automatic),
    /// A fixed exposure in milliseconds, or a fixed gain.
    Fixed(f64),
}
/// The JSON string `"auto"`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Automatic {
    /// Automatic camera control.
    #[serde(rename = "auto")]
    Auto,
}
impl Default for CameraValue {
    fn default() -> Self {
        Self::Automatic(Automatic::Auto)
    }
}
impl CameraValue {
    /// The fixed value, or none for automatic control.
    pub fn number(self) -> Option<f64> {
        match self {
            Self::Automatic(_) => None,
            Self::Fixed(value) => Some(value),
        }
    }
}

/// Durable camera settings. Until set, these defaults are used but omitted from reads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CameraSettings {
    /// Automatic, or exposure in milliseconds.
    pub exposure_ms: CameraValue,
    /// Automatic, or sensor gain.
    pub gain: CameraValue,
    /// Detection threshold, from 1 to 50 standard deviations.
    pub sigma: f64,
}
impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            exposure_ms: CameraValue::default(),
            gain: CameraValue::default(),
            sigma: 8.0,
        }
    }
}
/// A partial edit of the camera settings; null values are refused.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CameraSettingsEdit {
    /// Replacement exposure.
    #[serde(default, deserialize_with = "present_value")]
    pub exposure_ms: Option<CameraValue>,
    /// Replacement gain.
    #[serde(default, deserialize_with = "present_value")]
    pub gain: Option<CameraValue>,
    /// Replacement detection threshold.
    #[serde(default, deserialize_with = "present_value")]
    pub sigma: Option<f64>,
}
/// A partial pairing edit. Credentials are deliberately not Debug or Serialize.
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeEdit {
    /// Host and port, with no scheme or whitespace.
    #[serde(default, deserialize_with = "present_value")]
    pub address: Option<String>,
    /// Non-empty bearer token, never returned by the API.
    #[serde(default, deserialize_with = "present_value")]
    pub token: Option<String>,
}
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NodeSettings {
    pub(crate) address: Option<String>,
    pub(crate) token: Option<String>,
}
#[derive(Clone, PartialEq)]
pub(crate) struct LinkConfig {
    pub(crate) node: Option<NodeSettings>,
    pub(crate) settings: CameraSettings,
}
impl LinkConfig {
    pub(crate) fn paired(&self) -> bool {
        self.node
            .as_ref()
            .is_some_and(|node| node.address.is_some() && node.token.is_some())
    }
}

/// A camera range, inclusive.
#[derive(Debug, Serialize)]
pub struct CameraRange {
    /// Minimum accepted value.
    pub min: f64,
    /// Maximum accepted value.
    pub max: f64,
}
/// Camera details reported by Describe.
#[derive(Debug, Serialize)]
pub struct CameraView {
    /// Camera model.
    pub model: String,
    /// Full frame width.
    pub width: u32,
    /// Full frame height.
    pub height: u32,
    /// Accepted exposure range in milliseconds.
    pub exposure_ms: CameraRange,
    /// Accepted gain range.
    pub gain: CameraRange,
}
impl From<&CameraInfo> for CameraView {
    fn from(camera: &CameraInfo) -> Self {
        Self {
            model: camera.model.clone(),
            width: camera.width,
            height: camera.height,
            exposure_ms: CameraRange {
                min: camera.exposure_min_ms,
                max: camera.exposure_max_ms,
            },
            gain: CameraRange {
                min: camera.gain_min,
                max: camera.gain_max,
            },
        }
    }
}
/// Write-only pairing credentials and current link telemetry, without the token.
#[derive(Debug, Serialize)]
pub struct NodeView {
    /// Configured host and port.
    pub address: Option<String>,
    /// Whether a token has been stored.
    pub token_set: bool,
    /// Whether a heartbeat has arrived within one second.
    pub connected: bool,
    /// Time since the latest frame arrived, in console seconds.
    pub last_frame_age_s: Option<f64>,
    /// Node time less console time.
    pub offset_s: Option<f64>,
    /// Last described camera and its ranges.
    pub camera: Option<CameraView>,
    /// Latest frame's centroid count.
    pub stars: Option<usize>,
    /// Stored controls outside the camera's ranges, sent as automatic.
    pub out_of_range: Vec<OutOfRange>,
}

/// A checked partial edit. An omitted field is retained; null clears site or target.
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionEdit {
    /// A replacement site, or explicit null to clear it.
    #[serde(default, deserialize_with = "present_nullable")]
    pub site: Option<Option<Site>>,
    /// A replacement minimum altitude within 0–80°; null is invalid.
    #[serde(default, deserialize_with = "present_number")]
    pub min_alt_deg: Option<f64>,
    /// A catalog id, or explicit null to clear the target.
    #[serde(default, deserialize_with = "present_nullable")]
    pub target: Option<Option<String>>,
    /// Partial pairing edit, or null to clear the address and token.
    #[serde(default, deserialize_with = "present_nullable")]
    pub node: Option<Option<NodeEdit>>,
    /// Partial camera settings edit.
    #[serde(default, deserialize_with = "present_value")]
    pub camera_settings: Option<CameraSettingsEdit>,
}

fn present_nullable<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

fn present_value<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

fn present_number<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<f64>, D::Error> {
    f64::deserialize(deserializer).map(Some)
}

/// Durable full-frame pixel calibrated from a detected star.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Aim {
    /// Column in full-frame pixels.
    pub x: f64,
    /// Row in full-frame pixels.
    pub y: f64,
}
impl Aim {
    fn validate(self) -> Result<(), SessionError> {
        if self.x.is_finite() && self.y.is_finite() && self.x >= 0.0 && self.y >= 0.0 {
            Ok(())
        } else {
            Err(SessionError::Invalid(
                "aim must contain finite non-negative pixel coordinates".into(),
            ))
        }
    }
}
/// Where the latest pointing came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PointingSource {
    /// An uploaded image; it never expires.
    #[default]
    Upload,
    /// A star frame timed on the scope link.
    Live,
}
/// Pointing with its source.
#[derive(Debug, Clone, Serialize)]
pub struct LatestPointing {
    /// Original fields, flattened in JSON.
    #[serde(flatten)]
    pub pointing: Pointing,
    /// Upload or live.
    pub source: PointingSource,
    /// Exposure id, for live pointings only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame_id: Option<u64>,
}
impl std::ops::Deref for LatestPointing {
    type Target = Pointing;
    fn deref(&self) -> &Pointing {
        &self.pointing
    }
}
/// Field information appears only after receiving a live star frame.
#[derive(Debug, Clone, Serialize)]
pub struct FieldView {
    /// Latest solved horizontal field, in degrees.
    pub fov_deg: Option<f64>,
    /// Loaded database's lower field limit, in degrees.
    pub db_min_fov_deg: f64,
    /// Loaded database's upper field limit, in degrees.
    pub db_max_fov_deg: f64,
    /// Console seconds of rich frames without a solve.
    pub unsolved_s: f64,
}
#[derive(Clone)]
struct LiveField {
    fov_deg: Option<f64>,
    range: (f64, f64),
    unsolved_since: Option<f64>,
}

/// The most recent solved frame's pointing, held at its frame time.
#[derive(Debug, Clone, Serialize)]
pub struct Pointing {
    /// Monotonically increasing within this observing session, starting at one.
    pub seq: u64,
    /// The solved projection, retained for placing targets on the frame.
    pub frame: Frame,
    /// J2000 right ascension at the aim point (the frame centre until one is set).
    pub ra_deg: f64,
    /// J2000 declination at the aim point, in degrees.
    pub dec_deg: f64,
    /// Geometric altitude at frame time when a site is set, in degrees.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt_deg: Option<f64>,
    /// Azimuth at frame time when a site is set, north through east, in degrees.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub az_deg: Option<f64>,
    /// Frame time on the console's clock, in Unix seconds.
    pub time: f64,
}

impl Pointing {
    fn with_site(&mut self, site: Option<Site>) {
        let horizontal = site.map(|site| {
            sky::alt_az(
                self.ra_deg,
                self.dec_deg,
                &Observer {
                    lat: site.lat_deg,
                    lon: site.lon_deg,
                    time: self.time,
                },
            )
        });
        self.alt_deg = horizontal.map(|(alt, _)| alt);
        self.az_deg = horizontal.map(|(_, az)| az);
    }
}

/// One consistent read of the observing session and its console time.
#[derive(Debug, Serialize)]
pub struct SessionView {
    /// The telescope's site, if set.
    pub site: Option<Site>,
    /// The minimum altitude in degrees.
    pub min_alt_deg: f64,
    /// The resolved canonical catalog object, if selected.
    pub target: Option<&'static Target>,
    /// The latest pointing, if a frame has solved in this session.
    pub pointing: Option<LatestPointing>,
    /// Unix seconds from the application's clock.
    pub time: f64,
    /// The selected target's guidance, derived at this read's console time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guidance: Option<Guidance>,
    /// Pairing and telemetry; omitted before pairing has ever been set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node: Option<NodeView>,
    /// Camera settings; omitted until a camera control has been set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camera_settings: Option<CameraSettings>,
    /// Calibrated pixel, omitted until set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aim: Option<Aim>,
    /// Live field report, omitted before the first star frame.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<FieldView>,
}

#[derive(Clone)]
struct Values {
    site: Option<Site>,
    min_alt_deg: f64,
    target: Option<&'static Target>,
    pointing: Option<Pointing>,
    node: Option<NodeSettings>,
    camera_settings: Option<CameraSettings>,
    link_status: LinkStatus,
    aim: Option<Aim>,
    source: PointingSource,
    frame_id: Option<u64>,
    field: Option<LiveField>,
}

impl Default for Values {
    fn default() -> Self {
        Self {
            site: None,
            min_alt_deg: 15.0,
            target: None,
            pointing: None,
            node: None,
            camera_settings: None,
            link_status: LinkStatus::default(),
            aim: None,
            source: PointingSource::Upload,
            frame_id: None,
            field: None,
        }
    }
}

impl Values {
    fn view(&self, time: f64) -> SessionView {
        SessionView {
            site: self.site,
            min_alt_deg: self.min_alt_deg,
            target: self.target,
            pointing: self.pointing.clone().map(|pointing| LatestPointing {
                pointing,
                source: self.source,
                frame_id: self.frame_id,
            }),
            aim: self.aim,
            field: self.field.as_ref().map(|field| FieldView {
                fov_deg: field.fov_deg,
                db_min_fov_deg: field.range.0,
                db_max_fov_deg: field.range.1,
                unsolved_s: field
                    .unsolved_since
                    .map_or(0.0, |since| (time - since).max(0.0)),
            }),
            time,
            node: (self.node.is_some() || self.link_status.connected).then(|| {
                let status = &self.link_status;
                let outside = status.camera.as_ref().map_or_else(Vec::new, |camera| {
                    link::camera_settings(&self.camera_settings.clone().unwrap_or_default(), camera)
                        .1
                });
                NodeView {
                    address: self.node.as_ref().and_then(|node| node.address.clone()),
                    token_set: self.node.as_ref().is_some_and(|node| node.token.is_some()),
                    connected: status.connected,
                    last_frame_age_s: status.last_frame_at.map(|at| (time - at).max(0.0)),
                    offset_s: status.offset_s,
                    camera: status.camera.as_ref().map(CameraView::from),
                    stars: status.stars,
                    out_of_range: outside,
                }
            }),
            camera_settings: self.camera_settings.clone(),
            guidance: self.target.map(|target| {
                pushto::guidance_with_source(
                    target,
                    self.site,
                    self.min_alt_deg,
                    time,
                    self.pointing.as_ref(),
                    self.source,
                )
            }),
        }
    }
}

/// One shared observing session. By default it lives entirely in memory.
pub struct Session {
    values: Mutex<Values>,
    state_file: Option<PathBuf>,
    changes: watch::Sender<LinkConfig>,
}
impl Default for Session {
    fn default() -> Self {
        Self::from_values(Values::default(), None)
    }
}

/// An edit failed without changing the observing session.
#[derive(Debug)]
pub enum SessionError {
    /// An invalid site or minimum altitude.
    Invalid(String),
    /// The target id does not exist in the catalog.
    UnknownTarget(String),
    /// The state file could not be written.
    Storage(io::Error),
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(message) => f.write_str(message),
            Self::UnknownTarget(id) => write!(f, "unknown target {id:?}"),
            Self::Storage(error) => write!(f, "could not write the session state file: {error}"),
        }
    }
}

impl std::error::Error for SessionError {}

impl Session {
    fn from_values(values: Values, state_file: Option<PathBuf>) -> Self {
        let changes = watch::channel(values.link_config()).0;
        Self {
            values: Mutex::new(values),
            state_file,
            changes,
        }
    }
    pub(crate) fn link_changes(&self) -> watch::Receiver<LinkConfig> {
        self.changes.subscribe()
    }
    pub(crate) fn set_link_status(&self, status: LinkStatus) {
        self.values.lock().expect("observing session").link_status = status;
    }

    /// Read all session fields together, using time from the application's clock.
    pub fn snapshot(&self, time: f64) -> SessionView {
        self.values.lock().expect("observing session").view(time)
    }

    /// Record a successful solve and return its sequence. Pointings are never persisted.
    /// The position is the solve report's J2000 centre, in degrees.
    pub fn record_pointing(&self, frame: Frame, ra_deg: f64, dec_deg: f64, time: f64) -> u64 {
        let mut values = self.values.lock().expect("observing session");
        let seq = values.pointing.as_ref().map_or(1, |p| p.seq + 1);
        let mut pointing = Pointing {
            seq,
            frame,
            ra_deg,
            dec_deg,
            time,
            alt_deg: None,
            az_deg: None,
        };
        pointing.with_site(values.site);
        values.pointing = Some(pointing);
        values.source = PointingSource::Upload;
        values.frame_id = None;
        seq
    }

    /// Record a live solve at the current aim pixel, timed at mapped mid-exposure.
    pub fn record_live_pointing(&self, frame: Frame, time: f64, frame_id: u64) -> Option<u64> {
        let mut values = self.values.lock().expect("observing session");
        let direction = match values.aim {
            Some(aim) => frame.unproject((aim.x, aim.y))?,
            None => frame.center,
        };
        let (ra, dec) = direction.to_radec();
        let seq = values.pointing.as_ref().map_or(1, |p| p.seq + 1);
        let mut pointing = Pointing {
            seq,
            frame,
            ra_deg: ra.to_degrees(),
            dec_deg: dec.to_degrees(),
            time,
            alt_deg: None,
            az_deg: None,
        };
        pointing.with_site(values.site);
        values.pointing = Some(pointing);
        values.source = PointingSource::Live;
        values.frame_id = Some(frame_id);
        if let Some(field) = &mut values.field {
            field.fov_deg = Some(frame.fov_deg());
            field.unsolved_since = None;
        }
        Some(seq)
    }

    /// Note receipt, including frames dropped while solving.
    pub(crate) fn receive_live_frame(&self, stars: usize, time: f64, range: (f64, f64)) {
        let mut values = self.values.lock().expect("observing session");
        let field = values.field.get_or_insert(LiveField {
            fov_deg: None,
            range,
            unsolved_since: None,
        });
        if stars >= 10 {
            field.unsolved_since.get_or_insert(time);
        }
    }

    /// Persist the aim for subsequent solves, or reset it.
    pub fn set_aim(&self, aim: Option<Aim>, time: f64) -> Result<SessionView, SessionError> {
        if let Some(aim) = aim {
            aim.validate()?;
        }
        let mut values = self.values.lock().expect("observing session");
        let mut next = values.clone();
        next.aim = aim;
        if let Some(path) = &self.state_file {
            write_state(path, &next).map_err(SessionError::Storage)?;
        }
        *values = next;
        Ok(values.view(time))
    }

    /// Validate the entire edit and persist it before changing any field.
    pub fn edit(&self, edit: SessionEdit, time: f64) -> Result<SessionView, SessionError> {
        let mut values = self.values.lock().expect("observing session");
        let next = values.edited(edit)?;
        if let Some(path) = &self.state_file {
            write_state(path, &next).map_err(SessionError::Storage)?;
        }
        let previous = values.link_config();
        *values = next;
        let config = values.link_config();
        if config != previous {
            self.changes.send_replace(config);
        }
        Ok(values.view(time))
    }

    /// Load a durable observing session, creating its state file and parent folder.
    ///
    /// An unreadable or invalid file is reported on stderr and renamed with a `.bad`
    /// suffix before a fresh session is written. Site, minimum altitude, canonical
    /// target id, pairing and camera settings are durable; clock and pointing are not.
    pub fn with_state_file(path: impl Into<PathBuf>) -> io::Result<Self> {
        let path = path.into();
        let values = match read_state(&path) {
            Ok(values) => values,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let values = Values::default();
                write_state(&path, &values)?;
                values
            }
            Err(error) => {
                eprintln!(
                    "could not read session state file {}: {error}; keeping it as .bad",
                    path.display()
                );
                fs::rename(&path, path.with_added_extension("bad"))?;
                let values = Values::default();
                write_state(&path, &values)?;
                values
            }
        };
        Ok(Self::from_values(values, Some(path)))
    }
}

impl Values {
    fn link_config(&self) -> LinkConfig {
        LinkConfig {
            node: self.node.clone(),
            settings: self.camera_settings.clone().unwrap_or_default(),
        }
    }
    fn edited(&self, edit: SessionEdit) -> Result<Self, SessionError> {
        if let Some(Some(site)) = edit.site {
            check_range("site.lat_deg", site.lat_deg, -90.0, 90.0)?;
            check_range("site.lon_deg", site.lon_deg, -180.0, 180.0)?;
        }
        if let Some(min) = edit.min_alt_deg {
            check_range("min_alt_deg", min, 0.0, 80.0)?;
        }
        let target = edit
            .target
            .map(|id| {
                id.map(|id| targets::find(&id).ok_or(SessionError::UnknownTarget(id)))
                    .transpose()
            })
            .transpose()?;
        let mut next = self.clone();
        if let Some(site) = edit.site {
            next.site = site;
            if let Some(pointing) = &mut next.pointing {
                pointing.with_site(site);
            }
        }
        if let Some(min) = edit.min_alt_deg {
            next.min_alt_deg = min;
        }
        if let Some(target) = target {
            next.target = target;
        }
        if let Some(node) = edit.node {
            next.node = match node {
                None => None,
                Some(edit) => {
                    let mut node = self.node.clone().unwrap_or_default();
                    if let Some(address) = edit.address {
                        validate_address(&address)?;
                        node.address = Some(address);
                    }
                    if let Some(token) = edit.token {
                        if token.is_empty() || scope_link::BearerToken::new(&token).is_err() {
                            return Err(SessionError::Invalid(
                                "node.token must be a non-empty bearer token".into(),
                            ));
                        }
                        node.token = Some(token);
                    }
                    (node.address.is_some() || node.token.is_some()).then_some(node)
                }
            };
            if next.node != self.node {
                next.link_status = LinkStatus::default();
            }
        }
        if let Some(edit) = edit.camera_settings {
            let is_set = edit.exposure_ms.is_some() || edit.gain.is_some() || edit.sigma.is_some();
            let mut settings = self.camera_settings.clone().unwrap_or_default();
            for (field, value) in [("exposure_ms", edit.exposure_ms), ("gain", edit.gain)] {
                if let Some(value) = value {
                    if let Some(number) = value.number() {
                        if !number.is_finite() || number <= 0.0 {
                            return Err(SessionError::Invalid(format!(
                                "camera_settings.{field} must be auto or a positive finite number"
                            )));
                        }
                        if self.link_status.connected {
                            if let Some(camera) = &self.link_status.camera {
                                let (min, max) = if field == "exposure_ms" {
                                    (camera.exposure_min_ms, camera.exposure_max_ms)
                                } else {
                                    (camera.gain_min, camera.gain_max)
                                };
                                if !(min..=max).contains(&number) {
                                    return Err(SessionError::Invalid(format!(
                                        "{field} is outside the camera's range {min}–{max}"
                                    )));
                                }
                            }
                        }
                    }
                    if field == "exposure_ms" {
                        settings.exposure_ms = value;
                    } else {
                        settings.gain = value;
                    }
                }
            }
            if let Some(sigma) = edit.sigma {
                if !(1.0..=50.0).contains(&sigma) {
                    return Err(SessionError::Invalid(
                        "camera_settings.sigma must be within 1–50".into(),
                    ));
                }
                settings.sigma = sigma;
            }
            if is_set {
                next.camera_settings = Some(settings);
            }
        }
        Ok(next)
    }
}

fn validate_address(address: &str) -> Result<(), SessionError> {
    let invalid = || {
        SessionError::Invalid(
            "node.address must be a host and port (1–65535), without whitespace or a scheme".into(),
        )
    };
    if address.chars().any(char::is_whitespace) || address.contains('@') {
        return Err(invalid());
    }
    let authority: axum::http::uri::Authority = address.parse().map_err(|_| invalid())?;
    let host = authority.host();
    let valid_host = if host.starts_with('[') {
        host.trim_start_matches('[')
            .trim_end_matches(']')
            .parse::<std::net::Ipv6Addr>()
            .is_ok()
    } else {
        !host.is_empty()
            && host.split('.').all(|label| {
                !label.is_empty()
                    && label.len() <= 63
                    && label
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'-')
                    && !label.starts_with('-')
                    && !label.ends_with('-')
            })
    };
    if !valid_host || authority.port_u16().is_none_or(|port| port == 0) {
        return Err(invalid());
    }
    Ok(())
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredSession {
    site: Option<Site>,
    min_alt_deg: f64,
    target: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    node: Option<NodeSettings>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    camera_settings: Option<CameraSettings>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    aim: Option<Aim>,
}

fn read_state(path: &Path) -> io::Result<Values> {
    let bytes = fs::read(path)?;
    let stored: StoredSession = serde_json::from_slice(&bytes)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid session state JSON"))?;
    let aim = stored.aim;
    if let Some(aim) = aim {
        aim.validate()
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    }
    Values::default()
        .edited(SessionEdit {
            site: Some(stored.site),
            min_alt_deg: Some(stored.min_alt_deg),
            target: Some(stored.target),
            node: stored.node.map(|node| {
                Some(NodeEdit {
                    address: node.address,
                    token: node.token,
                })
            }),
            camera_settings: stored.camera_settings.map(|settings| CameraSettingsEdit {
                exposure_ms: Some(settings.exposure_ms),
                gain: Some(settings.gain),
                sigma: Some(settings.sigma),
            }),
        })
        .map(|mut values| {
            values.aim = aim;
            values
        })
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

fn write_state(path: &Path, values: &Values) -> io::Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_added_extension("tmp");
    // Remove an interrupted temporary write before creating a new owner-only file.
    match fs::remove_file(&temporary) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options.open(&temporary)?;
    serde_json::to_writer(
        &file,
        &StoredSession {
            site: values.site,
            min_alt_deg: values.min_alt_deg,
            target: values.target.map(|target| target.id.clone()),
            node: values.node.clone(),
            camera_settings: values.camera_settings.clone(),
            aim: values.aim,
        },
    )
    .map_err(io::Error::other)?;
    file.sync_all()?;
    fs::rename(temporary, path)
}

fn check_range(field: &str, value: f64, min: f64, max: f64) -> Result<(), SessionError> {
    if (min..=max).contains(&value) {
        Ok(())
    } else {
        Err(SessionError::Invalid(format!(
            "{field} must be within {min}–{max} degrees"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Oracle: a failed durable write must leave the previously observed session intact.
    #[test]
    fn failed_state_write_does_not_commit_the_edit() {
        let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target")
            .join(format!("session-write-failure-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join("session.json");
        let session = Session::with_state_file(&path).unwrap();
        // A directory at the required temporary filename makes writing fail even as root.
        std::fs::create_dir(path.with_added_extension("tmp")).unwrap();
        let edit = SessionEdit {
            min_alt_deg: Some(30.0),
            ..SessionEdit::default()
        };
        assert!(matches!(
            session.edit(edit, 0.0),
            Err(SessionError::Storage(_))
        ));
        assert_eq!(session.snapshot(0.0).min_alt_deg, 15.0);
        let restored = Session::with_state_file(&path).unwrap();
        assert_eq!(restored.snapshot(0.0).min_alt_deg, 15.0);
        std::fs::remove_dir_all(folder).unwrap();
    }

    // Oracle: FR-009's exact defaults; no numerical tolerance.
    #[test]
    fn fresh_session_has_no_site_target_or_pointing_and_fifteen_degree_minimum() {
        let view = Session::default().snapshot(1768528800.0);
        assert!(view.site.is_none());
        assert!(view.target.is_none());
        assert!(view.pointing.is_none());
        assert_eq!(view.min_alt_deg, 15.0);
    }

    // Oracle: 2026-01-16 02:00 UT = 1768528800 Unix seconds, exactly.
    #[test]
    fn fixed_clock_reads_the_same_unix_time_twice() {
        let clock = FixedClock(1768528800.0);
        assert_eq!(clock.now(), 1768528800.0);
        assert_eq!(clock.now(), 1768528800.0);
    }
}
