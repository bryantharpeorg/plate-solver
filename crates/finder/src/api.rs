//! The HTTP surface: the JSON API and the embedded page.
//!
//! `POST /api/model/second-sync` takes no body and starts a guarded slew to the
//! nearest pointable spot at least 90° in azimuth from every nightly sync. It
//! settles, solves and takes a sync without corrections or changing the target.
//! Answers 200 with `{"started":true}`, or 409 with a JSON `error` on refusal.
//!
//! `GET /api/session` includes `backlash: {alt_deg?, az_deg?}` once measured.
//! It survives nights and restarts; resetting fixed terms clears it.
//!
//! `POST /api/model/reset` accepts exactly `{"what":"night"}` or
//! `{"what":"fixed"}` and returns the session. The night reset clears syncs,
//! zero points and base tilt, retaining fixed terms; the fixed reset clears backlash and
//! those hardware terms. Both routes enforce the console's same-origin rule:
//! a foreign Origin answers 403 with a JSON `error` before any model or axis change.

//! `POST /api/track` accepts exactly `{"on":true}` or `{"on":false}` and
//! returns the session. It enforces the same-origin rule. Track on answers 409
//! with a JSON `error` without moving unless a mount, a nightly sync, a site,
//! a set Sun and a pointable target are available. It arms behind a running GoTo,
//! or starts a GoTo first when the target is more than one arcminute off the aim
//! point. Track off stops both axes; the bodiless, any-origin `POST /api/stop`
//! also clears arming and ends tracking with `ended_reason: "stopped"`.
//!
//! Session reads include run-only `track: {wanted, state, target, ended_reason?,
//! readout?}` after the first accepted Track-on request. `state` is `armed` or
//! the tracker's stable state name. The readout contains `rms_arcsec` over the
//! last 60 seconds and `field_rotation_deg_per_min`; `longest_exposure_s` is
//! added with an imaging camera, as seconds or `"no limit"` for one pixel of
//! corner smear. Tracking is off at every binary start and is never persisted.
//!
//! `PUT /api/session` also accepts `imaging_camera: {width_mm, height_mm,
//! pitch_um}`. Dimensions must be positive and at most 100 mm, pitch positive
//! and at most 50 µm. Invalid edits answer 400 with a JSON `error` and change
//! nothing. Explicit null clears it. The camera survives restarts and is
//! omitted from session bodies until set, or after being cleared.

use std::collections::HashMap;
use std::convert::Infallible;
use std::future::Future;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::body::Bytes;
use axum::extract::multipart::{Field, MultipartError};
use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{DefaultBodyLimit, FromRequestParts, Multipart, Path, Query, State};
use axum::http::{header, request::Parts, uri::Authority, StatusCode, Uri};
use axum::response::{
    sse::{Event, KeepAlive, Sse},
    IntoResponse, Response,
};
use axum::routing::{any, get, post};
use axum::{Json, Router};
use pattern_database::PatternDatabase;
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::catalog::{self, SimilarReport};
use crate::link::{LinkManager, Transport};
use crate::live::{DbSolver, Live, Solver};
use crate::samples::{self, SampleInfo, Samples};
use crate::session::{Clock, Session, SessionEdit, SessionError, SessionView, SystemClock};
use crate::simulate::{Push, Simulation, SimulationTransport, Telescope};
use crate::sky::Observer;
use crate::solve::{self, DecodeError, SolveOptions, SolveReport};
use crate::targets::{self, SearchResult};

/// Largest request body `/api/solve` accepts, in bytes.
pub const SOLVE_BODY_LIMIT: usize = 32 * 1024 * 1024;
/// Longest solve timeout a request may ask for, in milliseconds.
const MAX_TIMEOUT_MS: u64 = 30_000;
/// How long a request holding the solve gate may take to send its form, by default.
const FORM_READ_TIMEOUT: Duration = Duration::from_secs(30);
/// Longer side of a sample's preview, in pixels.
const THUMB_SIDE: u32 = 320;
/// Most suggestions `/api/similar` returns.
const MAX_SIMILAR: usize = 20;

/// What every handler shares.
#[derive(Clone)]
pub struct AppState {
    db: Arc<PatternDatabase>,
    samples: Arc<Samples>,
    /// One solve at a time, upload included: a request takes the gate before its form is
    /// read, so a burst of uploads waits on the network instead of in memory.
    solve_gate: Arc<Semaphore>,
    /// How long a request holding the gate may take to send its form.
    form_timeout: Duration,
    thumbs: Arc<Mutex<HashMap<String, Bytes>>>,
    session: Arc<Session>,
    clock: Arc<dyn Clock>,
    pub(crate) allowed_hosts: Arc<Vec<String>>,
    mcp: crate::mcp::McpConfig,
    link: Option<Arc<LinkManager>>,
    live: Option<Arc<Live>>,
    mount_monitor: Option<Arc<crate::live::MountMonitor>>,
    simulation: Option<Arc<Simulation>>,
    goto: Option<Arc<crate::goto::GoTo>>,
    tracking_owner: Option<Arc<crate::session::TrackingOwner>>,
    motion: Arc<tokio::sync::Mutex<Option<Motion>>>,
    call_log: Arc<Mutex<Box<dyn Write + Send>>>,
}

impl AppState {
    /// State over a loaded database and a set of samples.
    pub fn new(db: PatternDatabase, samples: Samples) -> Self {
        Self::shared(Arc::new(db), samples)
    }

    /// State over a database another owner keeps too.
    pub fn shared(db: Arc<PatternDatabase>, samples: Samples) -> Self {
        Self {
            db,
            samples: Arc::new(samples),
            solve_gate: Arc::new(Semaphore::new(1)),
            form_timeout: FORM_READ_TIMEOUT,
            thumbs: Arc::new(Mutex::new(HashMap::new())),
            session: Arc::new(Session::default()),
            clock: Arc::new(SystemClock),
            allowed_hosts: Arc::new(Vec::new()),
            mcp: crate::mcp::McpConfig::default(),
            link: None,
            live: None,
            mount_monitor: None,
            simulation: None,
            goto: None,
            tracking_owner: None,
            motion: Arc::new(tokio::sync::Mutex::new(None)),
            call_log: Arc::new(Mutex::new(Box::new(std::io::stderr()))),
        }
    }

    /// The same state with another limit on how long a request holding the solve gate may
    /// take to send its form (30 s by default). A request over it is answered 408.
    pub fn with_form_timeout(mut self, timeout: Duration) -> Self {
        self.form_timeout = timeout;
        self
    }

    /// Serialize uploaded and live solves through the same gate.
    pub(crate) async fn solve_permit(&self) -> Result<OwnedSemaphorePermit, ApiError> {
        self.solve_gate.clone().acquire_owned().await.map_err(|_| {
            ApiError::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "the solver is shutting down",
            )
        })
    }

    /// Share this observing session among all clients. The default is in memory.
    pub fn with_session(mut self, session: Session) -> Self {
        self.session = Arc::new(session);
        self
    }

    /// Use another console clock for tests or simulations. The default is system time.
    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    /// Build the desk-only node and its isolated link. Call before `with_live`.
    /// The saved session supplies the site and camera settings, never pairing.
    pub async fn with_simulation(
        mut self,
        start_unix_s: f64,
        transport: SimulationTransport,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let simulation = Arc::new(
            Simulation::start(
                self.db.clone(),
                self.session.clone(),
                start_unix_s,
                transport,
            )
            .await?,
        );
        self.clock = simulation.clock.clone();
        self.link = Some(simulation.link.clone());
        self.mount_monitor = Some(Arc::new(crate::live::MountMonitor::start(
            simulation.link.clone(),
            self.session.clone(),
        )));
        self.simulation = Some(simulation);
        Ok(self)
    }

    /// Ground-truth controls, available only when constructed in simulate mode.
    pub fn simulator(&self) -> Option<simulator::Simulator> {
        self.simulation.as_ref().map(|s| s.simulator.clone())
    }

    /// Start the session's paired scope link, using TCP or a reusable connector.
    /// Set the session and clock before calling this builder.
    pub fn with_link(mut self, transport: Transport) -> Self {
        self.link = Some(Arc::new(LinkManager::start(
            self.session.clone(),
            self.clock.clone(),
            transport,
        )));
        self.mount_monitor = Some(Arc::new(crate::live::MountMonitor::start(
            self.link.as_ref().unwrap().clone(),
            self.session.clone(),
        )));
        self
    }

    /// The running link, including its newest-frame and health subscriptions.
    pub fn link(&self) -> Option<Arc<LinkManager>> {
        self.link.clone()
    }

    /// Shared session, for integrations recording live pointings.
    pub fn session(&self) -> Arc<Session> {
        self.session.clone()
    }

    /// Start live solving after setting the session, clock and link.
    pub fn with_live(self) -> Self {
        let solver = Arc::new(DbSolver(self.db.clone()));
        self.with_live_solver(solver)
    }

    /// Start with an injected async solver, for delayed and failing solves in tests.
    pub fn with_live_solver(mut self, solver: Arc<dyn Solver>) -> Self {
        let link = self
            .link
            .as_ref()
            .expect("with_live requires with_link first");
        self.live = Some(Arc::new(Live::start(
            self.session.clone(),
            self.clock.clone(),
            link.frames(),
            link.status(),
            solver,
            self.solve_gate.clone(),
            (
                f64::from(self.db.properties.min_fov),
                f64::from(self.db.properties.max_fov),
            ),
        )));
        self.tracking_owner = Some(Arc::new(self.session.tracking_owner()));
        self.goto = Some(Arc::new(crate::goto::GoTo::new(
            self.session.clone(),
            self.clock.clone(),
            link.clone(),
            self.live.as_ref().unwrap().clone(),
        )));
        self
    }

    /// Session motion controller, present alongside live solving.
    pub fn goto(&self) -> Option<Arc<crate::goto::GoTo>> {
        self.goto.clone()
    }

    /// Live reports and preview history for stream and calibration consumers.
    pub fn live(&self) -> Option<Arc<Live>> {
        self.live.clone()
    }

    /// Configure MCP on the console's bound address. The default enables it on localhost.
    /// The allowed host names are kept unchanged.
    pub fn with_mcp(mut self, config: crate::mcp::McpConfig) -> Self {
        self.mcp = config;
        self
    }

    /// Write moving and stopping MCP calls to this sink; defaults to stderr.
    pub fn with_call_log(mut self, writer: impl Write + Send + 'static) -> Self {
        self.call_log = Arc::new(Mutex::new(Box::new(writer)));
        self
    }

    pub(crate) fn log_call(
        &self,
        tool: &str,
        arguments: &serde_json::Value,
        result: Result<&serde_json::Value, &str>,
    ) {
        let result = match result {
            Ok(value) => value.to_string(),
            Err(reason) => format!("error: {reason}"),
        };
        let line = format!("mcp {tool} {arguments} -> {result}\n");
        let mut writer = self.call_log.lock().unwrap();
        let _ = writer.write_all(line.as_bytes());
        let _ = writer.flush();
    }

    /// Allow these DNS names for same-origin writes and `/mcp`, besides IPs and localhost.
    /// Names have no scheme or port; matching is case insensitive and exact.
    pub fn with_allowed_hosts(mut self, hosts: impl IntoIterator<Item = String>) -> Self {
        self.allowed_hosts = Arc::new(hosts.into_iter().collect());
        self
    }
}

/// Checked before either write route extracts its body (RFC 6454 sections 5 and 7).
/// No Origin is allowed for non-browser clients; browser writes also need a trusted Host
/// to prevent DNS rebinding. Forwarded headers cannot override the request's origin.
struct SessionWrite;

impl FromRequestParts<AppState> for SessionWrite {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, ApiError> {
        check_origin(parts, &state.allowed_hosts)?;
        Ok(Self)
    }
}

/// The existing write-origin policy, also used before MCP dispatch.
pub(crate) fn check_origin(parts: &Parts, allowed_hosts: &[String]) -> Result<(), ApiError> {
    let denied = || {
        ApiError::new(
            StatusCode::FORBIDDEN,
            "writes require the console's own origin and an allowed host",
        )
    };
    let mut origins = parts.headers.get_all(header::ORIGIN).iter();
    let Some(origin) = origins.next() else {
        return Ok(());
    };
    if origins.next().is_some() {
        return Err(denied());
    }
    let origin = origin.to_str().map_err(|_| denied())?;
    let origin_uri: Uri = origin.parse().map_err(|_| denied())?;
    let scheme = parts.uri.scheme_str().unwrap_or("http");
    let origin_authority = origin_uri.authority().ok_or_else(denied)?;
    // An Origin is only a scheme and authority, never a path, query or user info.
    if origin_uri.scheme_str() != Some(scheme)
        || !origin.eq_ignore_ascii_case(&format!("{scheme}://{origin_authority}"))
        || origin_authority.as_str().contains('@')
    {
        return Err(denied());
    }
    let host = request_host(parts).ok_or_else(denied)?;
    let default_port = match scheme {
        "http" => 80,
        "https" => 443,
        _ => return Err(denied()),
    };
    let port = |authority: &Authority| authority_port(authority, default_port);
    if !host.host().eq_ignore_ascii_case(origin_authority.host())
        || port(&host).is_none()
        || port(&host) != port(origin_authority)
    {
        return Err(denied());
    }
    if !host_allowed(parts, allowed_hosts) {
        return Err(denied());
    }
    Ok(())
}

fn authority_port(authority: &Authority, default_port: u16) -> Option<u16> {
    // Authority::port() also returns None for an invalid explicit port.
    let suffix = &authority.as_str()[authority.host().len()..];
    if suffix.is_empty() {
        Some(default_port)
    } else {
        suffix.strip_prefix(':')?.parse().ok()
    }
}

fn request_host(parts: &Parts) -> Option<Authority> {
    let mut hosts = parts.headers.get_all(header::HOST).iter();
    let host: Authority = hosts.next()?.to_str().ok()?.parse().ok()?;
    if hosts.next().is_some() || host.as_str().contains('@') || authority_port(&host, 80).is_none()
    {
        return None;
    }
    Some(host)
}

/// Exact, ASCII case-insensitive DNS names; literals and localhost need no listing.
pub(crate) fn host_allowed(parts: &Parts, allowed_hosts: &[String]) -> bool {
    let Some(host) = request_host(parts) else {
        return false;
    };
    let name = host.host();
    let ip = name.trim_start_matches('[').trim_end_matches(']');
    ip.parse::<std::net::IpAddr>().is_ok()
        || name.eq_ignore_ascii_case("localhost")
        || allowed_hosts
            .iter()
            .any(|allowed| name.eq_ignore_ascii_case(allowed))
}

/// The console's routes.
pub fn router(state: AppState) -> Router {
    let mut routes = Router::new()
        .route("/api/health", get(health))
        .route("/api/samples", get(list_samples))
        .route("/api/samples/{name}", get(sample_file))
        .route("/api/samples/{name}/thumb", get(sample_thumb))
        .route(
            "/api/solve",
            post(solve).layer(DefaultBodyLimit::max(SOLVE_BODY_LIMIT)),
        )
        .route("/api/similar", get(similar))
        .route("/api/targets", get(search_targets))
        .route("/api/session", get(get_session).put(put_session))
        .route("/api/mcp", get(get_mcp))
        .route("/api/goto", post(start_goto))
        .route("/api/track", post(set_track))
        .route("/api/stop", post(stop_mount))
        .route("/api/model/second-sync", post(second_sync))
        .route("/api/model/reset", post(reset_model))
        .route("/api/aim", post(set_aim).delete(reset_aim))
        .route("/api/live", get(live_stream))
        .route("/api/live/preview.jpg", get(live_preview))
        .route("/api/sim", any(api_not_found))
        .route("/api/sim/{*rest}", any(api_not_found))
        .route("/api/{*rest}", get(api_not_found))
        .fallback(get(page));
    if state.simulation.is_some() {
        routes = routes
            .route("/api/sim/telescope", get(sim_telescope).post(sim_push))
            .route("/api/sim/frames", post(sim_frames));
    }
    if state.mcp.enabled {
        routes = routes.merge(
            Router::new()
                .route_service("/mcp", crate::mcp::service(state.clone()))
                .layer(axum::middleware::from_fn_with_state(
                    state.clone(),
                    crate::mcp::door::door,
                )),
        );
    }
    routes.with_state(state)
}

async fn live_stream(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let receiver = state
        .live
        .as_ref()
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "no live loop"))?
        .subscribe();
    let stream = futures_util::stream::unfold(receiver, |mut receiver| async move {
        loop {
            match receiver.recv().await {
                Ok(report) => {
                    let event = Event::default()
                        .event("live")
                        .json_data(report)
                        .expect("finite live report");
                    return Some((Ok::<_, Infallible>(event), receiver));
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => return None,
            }
        }
    });
    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15))))
}

#[derive(Deserialize)]
struct PreviewQuery {
    seq: u64,
}
async fn live_preview(
    State(state): State<AppState>,
    Query(query): Query<PreviewQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let preview = state
        .live
        .as_ref()
        .and_then(|live| live.preview(query.seq))
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::NOT_FOUND,
                "the console no longer holds that preview",
            )
        })?;
    Ok(([(header::CONTENT_TYPE, "image/jpeg")], preview.jpeg))
}

async fn sim_telescope(State(state): State<AppState>, _write: SessionWrite) -> Json<Telescope> {
    Json(
        state
            .simulator()
            .expect("simulate route")
            .telescope()
            .into(),
    )
}
async fn sim_push(
    State(state): State<AppState>,
    _write: SessionWrite,
    push: Result<Json<Push>, JsonRejection>,
) -> Result<Json<Telescope>, ApiError> {
    let sim = state.simulator().expect("simulate route");
    sim_json(push)?
        .apply(&sim, &state.db)
        .map_err(ApiError::bad_request)?;
    Ok(Json(sim.telescope().into()))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FramesControl {
    running: bool,
}
async fn sim_frames(
    State(state): State<AppState>,
    _write: SessionWrite,
    control: Result<Json<FramesControl>, JsonRejection>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let running = sim_json(control)?.running;
    let sim = state.simulator().expect("simulate route");
    if running {
        sim.start_frames();
    } else {
        sim.stop_frames();
    }
    Ok(Json(serde_json::json!({"running":running})))
}
fn sim_json<T>(result: Result<Json<T>, JsonRejection>) -> Result<T, ApiError> {
    result.map(|Json(value)| value).map_err(|error| {
        ApiError::new(
            if error.status() == StatusCode::UNPROCESSABLE_ENTITY {
                StatusCode::BAD_REQUEST
            } else {
                error.status()
            },
            error.body_text(),
        )
    })
}

/// Run-only attribution, serialized only while its motion is active.
#[derive(Clone, Serialize)]
pub(crate) struct Motion {
    by: &'static str,
    tool: &'static str,
    since: f64,
}

/// Serialize starts and session reads together: attribution precedes motion and
/// a refusal restores the previous owner without exposing a partial start.
pub(crate) async fn start_motion<F, Fut>(
    state: &AppState,
    by: &'static str,
    tool: &'static str,
    start: F,
) -> Result<(), ApiError>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<(), ApiError>>,
{
    let mut record = state.motion.lock().await;
    let previous = record.replace(Motion {
        by,
        tool,
        since: state.clock.now(),
    });
    if let Err(error) = start().await {
        *record = previous;
        return Err(error);
    }
    Ok(())
}

#[derive(Serialize)]
pub(crate) struct SessionResponse {
    #[serde(flatten)]
    pub(crate) session: SessionView,
    #[serde(skip_serializing_if = "Option::is_none")]
    goto: Option<crate::goto::Progress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    goto_refusal: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    motion: Option<Motion>,
}
async fn get_mcp(State(state): State<AppState>) -> Json<crate::mcp::McpStatus> {
    Json(state.mcp.status(&state.allowed_hosts))
}

async fn get_session(State(state): State<AppState>) -> Json<SessionResponse> {
    Json(session_response(&state).await)
}

pub(crate) async fn session_response(state: &AppState) -> SessionResponse {
    let record = state.motion.lock().await;
    let session = state.session.snapshot(state.clock.now());
    let goto_refusal = if session.mount.as_ref().is_some_and(|m| m.connected) {
        match &state.goto {
            Some(goto) => goto.refusal().await,
            None => None,
        }
    } else {
        None
    };
    let goto = state.goto.as_ref().and_then(|g| g.progress());
    let running = goto
        .as_ref()
        .is_some_and(|g| matches!(g.state, "slewing" | "solving" | "correcting"))
        || session
            .track
            .as_ref()
            .is_some_and(|t| matches!(t.state, "armed" | "tracking" | "keyhole_wait"));
    SessionResponse {
        session,
        goto,
        goto_refusal,
        motion: record.clone().filter(|_| running),
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TrackEdit {
    on: bool,
}
async fn set_track(
    State(state): State<AppState>,
    _write: SessionWrite,
    edit: Result<Json<TrackEdit>, JsonRejection>,
) -> Result<Json<SessionResponse>, ApiError> {
    let edit = sim_json(edit)?;
    if edit.on {
        start_motion(&state, "page", "track", || start_tracking(&state)).await?;
    } else {
        let _ = stop_mount(State(state.clone())).await;
    }
    Ok(get_session(State(state)).await)
}

async fn start_goto(
    State(state): State<AppState>,
    _write: SessionWrite,
) -> Result<Json<serde_json::Value>, ApiError> {
    start_motion(&state, "page", "goto", || goto_target(&state)).await?;
    Ok(Json(serde_json::json!({"started": true})))
}

pub(crate) async fn goto_target(state: &AppState) -> Result<(), ApiError> {
    let goto = state
        .goto
        .as_ref()
        .ok_or_else(|| ApiError::new(StatusCode::CONFLICT, "no mount is connected"))?;
    goto.start()
        .await
        .map_err(|reason| ApiError::new(StatusCode::CONFLICT, reason))
}

pub(crate) async fn start_tracking(state: &AppState) -> Result<(), ApiError> {
    let link = state
        .link
        .clone()
        .ok_or_else(|| ApiError::new(StatusCode::CONFLICT, "no mount is connected"))?;
    let goto = state
        .goto
        .clone()
        .ok_or_else(|| ApiError::new(StatusCode::CONFLICT, "no mount is connected"))?;
    state
        .session
        .start_tracking(state.clock.clone(), link, goto)
        .await
        .map_err(|reason| ApiError::new(StatusCode::CONFLICT, reason))
}

async fn second_sync(
    State(state): State<AppState>,
    _write: SessionWrite,
) -> Result<Json<serde_json::Value>, ApiError> {
    start_motion(&state, "page", "second_sync", || async {
        let goto = state
            .goto
            .as_ref()
            .ok_or_else(|| ApiError::new(StatusCode::CONFLICT, "no mount is connected"))?;
        goto.second_sync()
            .await
            .map_err(|reason| ApiError::new(StatusCode::CONFLICT, reason))
    })
    .await?;
    Ok(Json(serde_json::json!({"started": true})))
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum ModelPart {
    Night,
    Fixed,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelReset {
    what: ModelPart,
}

async fn reset_model(
    State(state): State<AppState>,
    _write: SessionWrite,
    reset: Result<Json<ModelReset>, JsonRejection>,
) -> Result<Json<SessionView>, ApiError> {
    let Json(reset) = reset.map_err(|error| {
        ApiError::new(
            if error.status() == StatusCode::UNPROCESSABLE_ENTITY {
                StatusCode::BAD_REQUEST
            } else {
                error.status()
            },
            error.body_text(),
        )
    })?;
    state
        .session
        .reset_model(matches!(reset.what, ModelPart::Fixed), state.clock.now())
        .map(Json)
        .map_err(session_error)
}
// Intentionally no SessionWrite and no body extractor: stopping is accepted
// from every origin, even while another route is waiting on a solve.
async fn stop_mount(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(stop(&state).await)
}

pub(crate) async fn stop(state: &AppState) -> serde_json::Value {
    state.session.stop_tracking().await;
    if let Some(goto) = &state.goto {
        goto.stop().await;
    } else if let Some(mut mount) = state.link.as_ref().and_then(|link| link.mount()) {
        let _ = tokio::time::timeout(Duration::from_millis(200), mount.stop_all()).await;
    }
    serde_json::json!({"stopped": true})
}

async fn put_session(
    State(state): State<AppState>,
    _write: SessionWrite,
    edit: Result<Json<SessionEdit>, JsonRejection>,
) -> Result<Json<SessionView>, ApiError> {
    let Json(edit) = edit.map_err(session_edit_error)?;
    edit_session(&state, edit).map(Json)
}

pub(crate) fn session_edit_error(error: JsonRejection) -> ApiError {
    let status = if error.status() == StatusCode::UNPROCESSABLE_ENTITY {
        StatusCode::BAD_REQUEST
    } else {
        error.status()
    };
    ApiError::new(status, error.body_text())
}

pub(crate) fn edit_session(state: &AppState, edit: SessionEdit) -> Result<SessionView, ApiError> {
    state
        .session
        .edit(edit, state.clock.now())
        .map_err(session_error)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AimTap {
    frame_id: u64,
    x: f64,
    y: f64,
}

async fn set_aim(
    State(state): State<AppState>,
    _write: SessionWrite,
    tap: Result<Json<AimTap>, JsonRejection>,
) -> Result<Json<SessionView>, ApiError> {
    let Json(tap) = tap.map_err(|error| {
        ApiError::new(
            if error.status() == StatusCode::UNPROCESSABLE_ENTITY {
                StatusCode::BAD_REQUEST
            } else {
                error.status()
            },
            error.body_text(),
        )
    })?;
    let preview = state
        .live
        .as_ref()
        .and_then(|live| live.preview(tap.frame_id))
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::CONFLICT,
                "the console no longer holds that frame",
            )
        })?;
    let star = preview
        .centroids
        .iter()
        .filter_map(|star| {
            let distance = (star.x - tap.x).hypot(star.y - tap.y);
            (distance <= 10.0).then_some((distance, star))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, star)| star)
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "no star is close enough to the tap",
            )
        })?;
    state
        .session
        .set_aim(
            Some(crate::session::Aim {
                x: star.x,
                y: star.y,
            }),
            state.clock.now(),
        )
        .map(Json)
        .map_err(session_error)
}
async fn reset_aim(
    State(state): State<AppState>,
    _write: SessionWrite,
) -> Result<Json<SessionView>, ApiError> {
    state
        .session
        .set_aim(None, state.clock.now())
        .map(Json)
        .map_err(session_error)
}
fn session_error(error: SessionError) -> ApiError {
    let status = match error {
        SessionError::Invalid(_) => StatusCode::BAD_REQUEST,
        SessionError::UnknownTarget(_) => StatusCode::NOT_FOUND,
        SessionError::Storage(_) => StatusCode::INTERNAL_SERVER_ERROR,
    };
    ApiError::new(status, error.to_string())
}

/// An error answered as JSON `{"error": "..."}`.
#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    pub(crate) message: String,
}

impl ApiError {
    fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }

    fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, message)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        #[derive(Serialize)]
        struct Body {
            error: String,
        }
        (
            self.status,
            Json(Body {
                error: self.message,
            }),
        )
            .into_response()
    }
}

impl From<MultipartError> for ApiError {
    fn from(e: MultipartError) -> Self {
        // `status()` tells a body over the limit (413) from a malformed one (400).
        Self::new(
            e.status(),
            format!("could not read the form: {}", e.body_text()),
        )
    }
}

impl From<DecodeError> for ApiError {
    fn from(e: DecodeError) -> Self {
        let status = match e {
            DecodeError::TooLarge(_) => StatusCode::PAYLOAD_TOO_LARGE,
            DecodeError::Unreadable(_) => StatusCode::UNSUPPORTED_MEDIA_TYPE,
        };
        Self::new(status, e.to_string())
    }
}

#[derive(Serialize)]
struct Health {
    status: &'static str,
    version: &'static str,
    database: DatabaseInfo,
    samples: usize,
}

#[derive(Serialize)]
struct DatabaseInfo {
    catalog: String,
    stars: usize,
    patterns: u32,
    min_fov: f32,
    max_fov: f32,
}

async fn health(State(state): State<AppState>) -> Json<Health> {
    let p = &state.db.properties;
    Json(Health {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        database: DatabaseInfo {
            catalog: p.star_catalog.clone(),
            stars: state.db.num_stars,
            patterns: p.num_patterns,
            min_fov: p.min_fov,
            max_fov: p.max_fov,
        },
        samples: state.samples.len(),
    })
}

async fn list_samples(State(state): State<AppState>) -> Json<Vec<SampleInfo>> {
    Json(state.samples.list())
}

async fn read_sample(state: &AppState, name: &str) -> Result<Vec<u8>, ApiError> {
    let path = state
        .samples
        .path(name)
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, format!("no sample named {name:?}")))?;
    tokio::fs::read(&path).await.map_err(|e| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("could not read the sample {name:?}: {e}"),
        )
    })
}

async fn sample_file(State(state): State<AppState>, Path(name): Path<String>) -> Response {
    match read_sample(&state, &name).await {
        Ok(bytes) => (
            [
                (header::CONTENT_TYPE, samples::content_type(&name)),
                (header::CACHE_CONTROL, "public, max-age=3600"),
            ],
            bytes,
        )
            .into_response(),
        Err(e) => e.into_response(),
    }
}

async fn sample_thumb(State(state): State<AppState>, Path(name): Path<String>) -> Response {
    let cached = state
        .thumbs
        .lock()
        .expect("thumb cache")
        .get(&name)
        .cloned();
    let jpeg = match cached {
        Some(jpeg) => jpeg,
        None => {
            let bytes = match read_sample(&state, &name).await {
                Ok(bytes) => bytes,
                Err(e) => return e.into_response(),
            };
            let made =
                tokio::task::spawn_blocking(move || solve::thumbnail_jpeg(&bytes, THUMB_SIDE))
                    .await
                    .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "preview failed"))
                    .and_then(|r| r.map_err(ApiError::from));
            match made {
                Ok(jpeg) => {
                    let jpeg = Bytes::from(jpeg);
                    let mut thumbs = state.thumbs.lock().expect("thumb cache");
                    thumbs.insert(name, jpeg.clone());
                    jpeg
                }
                Err(e) => return e.into_response(),
            }
        }
    };
    (
        [
            (header::CONTENT_TYPE, "image/jpeg"),
            (header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        jpeg,
    )
        .into_response()
}

/// Where a solve's frame comes from.
enum Source {
    Upload {
        bytes: Vec<u8>,
        name: Option<String>,
    },
    Sample(String),
}

/// The fields of a `/api/solve` form.
struct SolveForm {
    source: Source,
    options: SolveOptions,
}

async fn text(field: Field<'_>) -> Result<String, ApiError> {
    Ok(field.text().await?.trim().to_string())
}

fn parse<T: std::str::FromStr>(name: &str, value: &str) -> Result<T, ApiError> {
    value
        .parse()
        .map_err(|_| ApiError::bad_request(format!("{name} is not a number: {value:?}")))
}

impl SolveForm {
    /// Read and check the form: a frame (`image` file or `sample` name) and options.
    async fn read(multipart: &mut Multipart) -> Result<Self, ApiError> {
        let mut source = None;
        let mut o = SolveOptions::default();
        while let Some(field) = multipart.next_field().await? {
            let name = field.name().unwrap_or("").to_string();
            match name.as_str() {
                "image" => {
                    let file_name = field.file_name().map(str::to_string);
                    let bytes = field.bytes().await?.to_vec();
                    source = Some(Source::Upload {
                        bytes,
                        name: file_name,
                    });
                }
                "sample" => source = Some(Source::Sample(text(field).await?)),
                "fov" => {
                    let v = text(field).await?;
                    o.fov = if v.is_empty() {
                        None
                    } else {
                        Some(parse(&name, &v)?)
                    };
                }
                "fov_max_error" => o.fov_max_error = parse(&name, &text(field).await?)?,
                "distortion" => o.distortion = parse(&name, &text(field).await?)?,
                "bin" => o.binning = parse(&name, &text(field).await?)?,
                "sigma" => o.sigma = parse(&name, &text(field).await?)?,
                "stars" => o.stars = parse(&name, &text(field).await?)?,
                "timeout_ms" => o.timeout_ms = parse(&name, &text(field).await?)?,
                _ => {
                    field.bytes().await?;
                }
            }
        }
        let source = source.ok_or_else(|| {
            ApiError::bad_request("send a frame: an `image` file or a `sample` name")
        })?;
        check(&o)?;
        o.timeout_ms = o.timeout_ms.min(MAX_TIMEOUT_MS);
        Ok(Self { source, options: o })
    }
}

fn check(o: &SolveOptions) -> Result<(), ApiError> {
    let fail = |m: &str| Err(ApiError::bad_request(m));
    // A positive, finite number: NaN fails every comparison, so test finiteness first.
    let positive = |v: f64| v.is_finite() && v > 0.0;
    if let Some(fov) = o.fov {
        if !positive(fov) || fov >= 180.0 {
            return fail("fov must be between 0 and 180 degrees");
        }
    }
    if !positive(o.fov_max_error) {
        return fail("fov_max_error must be a positive number of degrees");
    }
    if !o.distortion.is_finite() || o.distortion.abs() >= 1.0 {
        return fail("distortion must be between -1 and 1");
    }
    if ![1, 2, 4, 8].contains(&o.binning) {
        return fail("bin must be 1, 2, 4 or 8");
    }
    if !positive(o.sigma) {
        return fail("sigma must be a positive number");
    }
    if !(4..=200).contains(&o.stars) {
        return fail("stars must be between 4 and 200");
    }
    if o.timeout_ms == 0 {
        return fail("timeout_ms must be positive");
    }
    Ok(())
}

async fn solve(
    State(state): State<AppState>,
    _write: SessionWrite,
    mut multipart: Multipart,
) -> Result<Json<SolveReport>, ApiError> {
    // Take the gate before reading the form. Each upload may be up to SOLVE_BODY_LIMIT, and
    // reading them first would hold every queued one in memory at once; this way a waiting
    // request's body stays unread. A client that stalls while holding the gate is cut off.
    let permit = state.solve_permit().await?;
    let form = tokio::time::timeout(state.form_timeout, SolveForm::read(&mut multipart))
        .await
        .map_err(|_| {
            ApiError::new(
                StatusCode::REQUEST_TIMEOUT,
                "the form took too long to arrive",
            )
        })??;
    let (bytes, name) = match form.source {
        Source::Upload { bytes, name } => (bytes, name),
        Source::Sample(name) => (read_sample(&state, &name).await?, Some(name)),
    };
    solve_upload(&state, permit, bytes, name, form.options)
        .await
        .map(Json)
}

/// Solve an upload and record its latest pointing exactly as the page does.
/// The caller takes the gate before reading the multipart form or starting a solve.
pub(crate) async fn solve_upload(
    state: &AppState,
    permit: OwnedSemaphorePermit,
    bytes: Vec<u8>,
    name: Option<String>,
    options: SolveOptions,
) -> Result<SolveReport, ApiError> {
    let db = state.db.clone();
    let session = state.session.clone();
    let clock = state.clock.clone();
    let report = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let t = Instant::now();
        let image = solve::decode_gray(&bytes)?;
        let decode = t.elapsed().as_secs_f64() * 1e3;
        let (mut report, frame) = solve::solve_frame_with_frame(&db, &image, &options);
        if let (Some(frame), Some(pointing)) = (frame, &report.pointing) {
            report.pointing_seq = Some(session.record_pointing(
                frame,
                pointing.ra_deg,
                pointing.dec_deg,
                clock.now(),
            ));
        }
        report.timing_ms.decode = decode;
        report.name = name;
        Ok::<_, DecodeError>(report)
    })
    .await
    .map_err(|_| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "the solver stopped unexpectedly",
        )
    })??;
    Ok(report)
}

/// `/api/similar` query: an object and, optionally, where and when it is seen from.
#[derive(Debug, Deserialize)]
struct SimilarQuery {
    id: String,
    lat: Option<f64>,
    lon: Option<f64>,
    /// Unix time in seconds; now when absent.
    time: Option<f64>,
    limit: Option<usize>,
}

async fn similar(
    State(state): State<AppState>,
    Query(q): Query<SimilarQuery>,
) -> Result<Json<SimilarReport>, ApiError> {
    let reference = catalog::find(&q.id).ok_or_else(|| {
        ApiError::new(
            StatusCode::NOT_FOUND,
            format!("no Messier object {:?}", q.id),
        )
    })?;
    let session = state.session.snapshot(state.clock.now());
    let observer = match (q.lat, q.lon) {
        (Some(lat), Some(lon)) => {
            if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
                return Err(ApiError::bad_request(
                    "lat must be within ±90 degrees and lon within ±180",
                ));
            }
            Some(Observer {
                lat,
                lon,
                time: q.time.unwrap_or(session.time),
            })
        }
        (None, None) => session.site.map(|site| Observer {
            lat: site.lat_deg,
            lon: site.lon_deg,
            time: q.time.unwrap_or(session.time),
        }),
        _ => return Err(ApiError::bad_request("give both lat and lon, or neither")),
    };
    let limit = q.limit.unwrap_or(8).clamp(1, MAX_SIMILAR);
    Ok(Json(catalog::similar_above(
        reference,
        observer.as_ref(),
        limit,
        session.min_alt_deg,
    )))
}

#[derive(Deserialize)]
struct TargetsQuery {
    q: Option<String>,
    limit: Option<usize>,
}

async fn search_targets(
    State(state): State<AppState>,
    query: Result<Query<TargetsQuery>, QueryRejection>,
) -> Result<Json<Vec<SearchResult>>, ApiError> {
    let Query(query) = query.map_err(|error| ApiError::bad_request(error.body_text()))?;
    search(&state, query.q.as_deref().unwrap_or(""), query.limit).map(Json)
}

pub(crate) fn search(
    state: &AppState,
    q: &str,
    limit: Option<usize>,
) -> Result<Vec<SearchResult>, ApiError> {
    let q = q.trim();
    if q.is_empty() {
        return Err(ApiError::bad_request(
            "give a nonempty q to search for a target",
        ));
    }
    let session = state.session.snapshot(state.clock.now());
    Ok(targets::search(q, limit.unwrap_or(20).clamp(1, 50))
        .into_iter()
        .map(|object| SearchResult::at(object, session.site, session.min_alt_deg, session.time))
        .collect())
}

async fn api_not_found(uri: Uri) -> ApiError {
    ApiError::new(
        StatusCode::NOT_FOUND,
        format!("no API route {}", uri.path()),
    )
}

/// The page's production build, embedded at compile time (read from disk in debug
/// builds). Rebuild it with `npm run build` in `web/` after changing the page.
#[derive(RustEmbed)]
#[folder = "web/dist"]
struct Page;

/// The media type of a page file, by extension.
fn page_type(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "woff2" => "font/woff2",
        "json" | "webmanifest" => "application/json",
        _ => "application/octet-stream",
    }
}

/// Serve a page file. Fingerprinted assets cache forever; everything else revalidates.
/// A path with no file extension is a route inside the page and gets `index.html`.
async fn page(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };
    let (path, file) = match Page::get(path) {
        Some(file) => (path, file),
        None if !path.contains('.') => match Page::get("index.html") {
            Some(file) => ("index.html", file),
            None => return StatusCode::NOT_FOUND.into_response(),
        },
        None => return StatusCode::NOT_FOUND.into_response(),
    };
    let cache = if path.starts_with("assets/") {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    };
    (
        [
            (header::CONTENT_TYPE, page_type(path)),
            (header::CACHE_CONTROL, cache),
        ],
        file.data,
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_are_checked() {
        let ok = SolveOptions::default();
        assert!(check(&ok).is_ok());
        let bad = [
            SolveOptions {
                fov: Some(0.0),
                ..ok
            },
            SolveOptions {
                fov: Some(f64::NAN),
                ..ok
            },
            SolveOptions {
                fov_max_error: -1.0,
                ..ok
            },
            SolveOptions {
                distortion: 1.5,
                ..ok
            },
            SolveOptions { binning: 3, ..ok },
            SolveOptions { sigma: 0.0, ..ok },
            SolveOptions { stars: 3, ..ok },
            SolveOptions {
                timeout_ms: 0,
                ..ok
            },
        ];
        for o in bad {
            let e = check(&o).unwrap_err();
            assert_eq!(e.status, StatusCode::BAD_REQUEST, "{o:?}");
        }
    }

    #[test]
    fn page_types() {
        assert_eq!(page_type("index.html"), "text/html; charset=utf-8");
        assert_eq!(
            page_type("assets/app-1.js"),
            "text/javascript; charset=utf-8"
        );
        assert_eq!(page_type("assets/app-1.css"), "text/css; charset=utf-8");
        assert_eq!(page_type("favicon.svg"), "image/svg+xml");
        assert_eq!(page_type("a.woff2"), "font/woff2");
        assert_eq!(page_type("manifest.webmanifest"), "application/json");
        assert_eq!(page_type("blob"), "application/octet-stream");
    }

    #[test]
    fn decode_errors_map_to_statuses() {
        let e: ApiError = DecodeError::TooLarge("x".into()).into();
        assert_eq!(e.status, StatusCode::PAYLOAD_TOO_LARGE);
        let e: ApiError = DecodeError::Unreadable("x".into()).into();
        assert_eq!(e.status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }
}
