//! The finder camera's focus aid (051): the size of the stars in the finder
//! camera's raw frames, read beside the live solve and judged by 033's own
//! arithmetic (`imaging::focus::aid::Aid`). It changes no setting of the finder
//! camera and takes nothing from the live solve; each reading costs one raw
//! frame on the scope link (025).

use crate::{
    api::{sim_json, ApiError, AppState, SessionWrite},
    imaging::focus::{
        aid::{Aid, Sample},
        FAR_OUT_PX, MIN_STARS,
    },
    link::LinkManager,
    session::{CameraSettings, Clock},
};
use axum::{
    extract::{rejection::JsonRejection, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use scope_link::proto::{Centroid, RawFrameReply, StarFrame};
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex, Weak},
    time::Duration,
};
use tokio::{
    sync::watch,
    task::JoinHandle,
    time::{self, Instant},
};

/// The brightest unsaturated stars measured in a frame (US1).
pub const STARS_MEASURED: usize = 15;
/// Half the side of the square box each star is measured in, in pixels (US1).
pub const BOX_PX: i64 = 16;
/// A star with a joined pixel at or above this level is not measured (US1): the finder
/// camera's one saturation level, the scope node's detection's too (052).
pub const SATURATED: u8 = star_detection::saturated::SATURATED;
/// A pixel joins a star only this many noise levels above the background (US1).
pub const THRESHOLD_NOISE: f64 = 3.0;
/// Each pixel is summed as this many sub-pixels a side when a star's flux is
/// counted in circles (US1).
pub const SUBPIXELS: u32 = 5;
/// A star's circle reaches this far past its farthest joined pixel, in pixels (US1).
pub const APERTURE_MARGIN_PX: f64 = 1.0;

const SIDE: usize = (2 * BOX_PX + 1) as usize;

/// One raw frame measured at its own star frame's centroids, the node's
/// detection (US1): a reading, or why there is none, as 033's `Sample` names
/// it. The centroids are brightest first; a star is measured when its box lies
/// inside the frame and holds no other centroid, and its own pixels are under
/// `SATURATED` and have flux; the first `STARS_MEASURED` such stars are
/// measured, each as its half-flux diameter in sub-pixel circles.
pub fn measure(pixels: &[u8], width: u32, height: u32, centroids: &[Centroid]) -> Sample {
    assert_eq!(pixels.len(), width as usize * height as usize);
    let mut sizes = Vec::with_capacity(STARS_MEASURED);
    for (index, star) in centroids.iter().enumerate() {
        if !star.x.is_finite() || !star.y.is_finite() {
            continue;
        }
        let (x, y) = (star.x.floor() as i64, star.y.floor() as i64);
        if x < BOX_PX
            || y < BOX_PX
            || x >= i64::from(width) - BOX_PX
            || y >= i64::from(height) - BOX_PX
            || centroids.iter().enumerate().any(|(other_index, other)| {
                index != other_index
                    && (other.x - star.x).abs() <= BOX_PX as f64
                    && (other.y - star.y).abs() <= BOX_PX as f64
            })
        {
            continue;
        }
        if let Some(size) = star_size(pixels, width as usize, x, y) {
            sizes.push(size);
            if sizes.len() == STARS_MEASURED {
                break;
            }
        }
    }
    let stars = sizes.len() as u32;
    if stars < MIN_STARS {
        return Sample::TooFewStars { stars };
    }
    let px = median(&mut sizes);
    if px > FAR_OUT_PX {
        Sample::FarOut { px, stars }
    } else {
        Sample::Reading { px, stars }
    }
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_unstable_by(f64::total_cmp);
    (values[(values.len() - 1) / 2] + values[values.len() / 2]) / 2.0
}

// Offsets from the centre of the box's centre pixel, not the supplied centroid.
fn offset(index: usize) -> (f64, f64) {
    (
        (index % SIDE) as f64 - BOX_PX as f64,
        (index / SIDE) as f64 - BOX_PX as f64,
    )
}

fn star_size(pixels: &[u8], width: usize, x: i64, y: i64) -> Option<f64> {
    let mut patch = [0.0; SIDE * SIDE];
    let mut edge = Vec::with_capacity(4 * (SIDE - 1));
    for row in 0..SIDE {
        for column in 0..SIDE {
            let value = f64::from(
                pixels[(y - BOX_PX + row as i64) as usize * width
                    + (x - BOX_PX + column as i64) as usize],
            );
            patch[row * SIDE + column] = value;
            if row == 0 || column == 0 || row == SIDE - 1 || column == SIDE - 1 {
                edge.push(value);
            }
        }
    }
    let background = median(&mut edge);
    for value in &mut edge {
        *value = (*value - background).abs();
    }
    let noise = (1.4826 * median(&mut edge)).max(0.5);
    let level = background + THRESHOLD_NOISE * noise;

    // A strict comparison preserves the first in reading order on a tie.
    let middle = BOX_PX as usize;
    let mut start = (middle - 2) * SIDE + middle - 2;
    for row in middle - 2..=middle + 2 {
        for column in middle - 2..=middle + 2 {
            let index = row * SIDE + column;
            if patch[index] > patch[start] {
                start = index;
            }
        }
    }
    if patch[start] <= level {
        return None;
    }

    // Only the 8-connected component of the start pixel belongs to the star.
    let mut seen = [false; SIDE * SIDE];
    let mut pending = vec![start];
    let mut own = Vec::new();
    seen[start] = true;
    while let Some(index) = pending.pop() {
        if patch[index] >= f64::from(SATURATED) {
            return None;
        }
        own.push(index);
        let (row, column) = (index / SIDE, index % SIDE);
        for r in row.saturating_sub(1)..=(row + 1).min(SIDE - 1) {
            for c in column.saturating_sub(1)..=(column + 1).min(SIDE - 1) {
                let neighbor = r * SIDE + c;
                if !seen[neighbor] && patch[neighbor] > level {
                    seen[neighbor] = true;
                    pending.push(neighbor);
                }
            }
        }
    }

    let (mut flux, mut cx, mut cy) = (0.0, 0.0, 0.0);
    for &index in &own {
        let value = patch[index] - background;
        let (dx, dy) = offset(index);
        flux += value;
        cx += dx * value;
        cy += dy * value;
    }
    cx /= flux;
    cy /= flux;
    let farthest = own
        .iter()
        .map(|&index| {
            let (dx, dy) = offset(index);
            (dx - cx).hypot(dy - cy)
        })
        .fold(0.0_f64, f64::max);
    let radius = (farthest + APERTURE_MARGIN_PX).min((BOX_PX - 1) as f64);

    // SEP's flux_radius method: sub-pixel circles, then linear interpolation
    // of the half-flux crossing. Written from the description, without SEP source.
    let mut parts = Vec::new();
    for (index, &value) in patch.iter().enumerate() {
        let (dx, dy) = offset(index);
        if (dx - cx).hypot(dy - cy) > radius + 1.0 {
            continue;
        }
        let part_flux = (value - background) / f64::from(SUBPIXELS * SUBPIXELS);
        for b in 0..SUBPIXELS {
            for a in 0..SUBPIXELS {
                let px = dx - 0.5 + (f64::from(a) + 0.5) / f64::from(SUBPIXELS);
                let py = dy - 0.5 + (f64::from(b) + 0.5) / f64::from(SUBPIXELS);
                let distance = (px - cx).hypot(py - cy);
                if distance <= radius {
                    parts.push((distance, part_flux));
                }
            }
        }
    }
    let total: f64 = parts.iter().map(|part| part.1).sum();
    if total <= 0.0 {
        return None;
    }
    parts.sort_unstable_by(|a, b| a.0.total_cmp(&b.0));
    let (mut previous_distance, mut enclosed) = (0.0, 0.0);
    let mut i = 0;
    while i < parts.len() {
        let distance = parts[i].0;
        let mut group_flux = parts[i].1;
        i += 1;
        // Nearby consecutive distances are one radius, including ties split
        // by floating-point rounding. Include the whole radius before crossing.
        while i < parts.len() && parts[i].0 - parts[i - 1].0 <= 1e-9 {
            group_flux += parts[i].1;
            i += 1;
        }
        let next = enclosed + group_flux;
        if next >= total / 2.0 {
            let fraction = (total / 2.0 - enclosed) / (next - enclosed);
            return Some(2.0 * (previous_distance + fraction * (distance - previous_distance)));
        }
        previous_distance = distance;
        enclosed = next;
    }
    None
}

/// How long one raw frame may take to arrive: 025's limit (US2).
pub const RAW_WAIT: Duration = Duration::from_secs(15);

/// The last sample in the view (US2).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct LastView {
    pub frame_id: u64,
    /// `reading`, `settling`, `too_few_stars` or `far_out`: 033's words.
    pub state: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    /// The stars measured.
    pub stars: u32,
    /// The centroids of the frame's own star frame.
    pub detected: u32,
}

/// The median of the readings held (US2).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct MedianView {
    pub value: f64,
    pub readings: usize,
}

/// `focus` in `GET /api/finder-camera/focus` (US2): the latest run, running or
/// ended. Every size is in the finder camera's pixels.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FocusView {
    /// `running` or `ended`.
    pub state: &'static str,
    pub since: f64,
    pub readings: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cadence_ms: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last: Option<LastView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub median: Option<MedianView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trend: Option<&'static str>,
    pub found: bool,
    pub history: Vec<Option<f64>>,
    /// `stopped` or `failed`, once ended.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// The body of `POST /api/finder-camera/focus`: nothing (US2).
#[derive(Debug, Clone, Default)]
pub struct FocusAsk {}

impl<'de> Deserialize<'de> for FocusAsk {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // Require an object: a derived empty struct would also accept `[]`.
        let fields = serde_json::Map::<String, serde_json::Value>::deserialize(deserializer)?;
        if !fields.is_empty() {
            return Err(serde::de::Error::custom(
                "the finder camera's focus aid takes an empty object",
            ));
        }
        Ok(Self {})
    }
}

/// Why a start was refused: the status and the words.
#[derive(Debug, Clone, PartialEq)]
pub struct Refusal {
    pub status: StatusCode,
    pub message: String,
}

/// The finder camera's focus aid: at most one run at a time (US2).
#[derive(Default)]
pub struct FinderFocus {
    run: Mutex<Option<Run>>,
}

struct Run {
    id: u64,
    view: FocusView,
    aid: Aid,
    last_at: Option<Instant>,
    task: Option<JoinHandle<()>>,
}

impl FinderFocus {
    /// The latest run, or `None` before the first start.
    pub fn view(&self) -> Option<FocusView> {
        self.run.lock().unwrap().as_ref().map(|r| r.view.clone())
    }

    /// Start a run over the link's raw frames (US2), under the session's
    /// camera settings, `None` before any is set.
    pub fn start(
        self: &Arc<Self>,
        link: Option<Arc<LinkManager>>,
        settings: Option<CameraSettings>,
        clock: Arc<dyn Clock>,
    ) -> Result<FocusView, Refusal> {
        let mut run = self.run.lock().unwrap();
        let refuse = |message: &str| Refusal {
            status: StatusCode::CONFLICT,
            message: message.into(),
        };
        if run.as_ref().is_some_and(|r| r.view.state == "running") {
            return Err(refuse("the finder camera's focus aid is already running"));
        }
        let link = link
            .filter(|link| link.raw().is_some())
            .ok_or_else(|| refuse("the scope node is not connected"))?;
        if !settings.is_some_and(|s| s.exposure_ms.number().is_some() && s.gain.number().is_some())
        {
            return Err(refuse(
                "the finder camera's exposure or gain is automatic: fix both on Setup",
            ));
        }
        let view = FocusView {
            state: "running",
            since: clock.now(),
            readings: 0,
            cadence_ms: None,
            last: None,
            median: None,
            best: None,
            trend: None,
            found: false,
            history: Vec::new(),
            ended: None,
            error: None,
        };
        let id = run.as_ref().map_or(0, |r| r.id + 1);
        let shared = Arc::downgrade(self);
        let task = tokio::spawn(async move {
            if let Err(error) = focus_loop(&shared, id, &link).await {
                if let Some(shared) = shared.upgrade() {
                    let mut run = shared.run.lock().unwrap();
                    if let Some(run) = run
                        .as_mut()
                        .filter(|r| r.id == id && r.view.state == "running")
                    {
                        run.view.state = "ended";
                        run.view.ended = Some("failed");
                        run.view.error = Some(error.into());
                        run.task = None;
                    }
                }
            }
        });
        *run = Some(Run {
            id,
            view: view.clone(),
            aid: Aid::new(),
            last_at: None,
            task: Some(task),
        });
        Ok(view)
    }

    /// Stop the run, if one runs, and answer the view (US2).
    pub fn stop(&self) -> Option<FocusView> {
        let mut run = self.run.lock().unwrap();
        let run = run.as_mut()?;
        if run.view.state == "running" {
            if let Some(task) = run.task.take() {
                task.abort();
            }
            run.view.state = "ended";
            run.view.ended = Some("stopped");
        }
        Some(run.view.clone())
    }

    fn push(&self, id: u64, frame: &StarInfo, sample: Sample) -> bool {
        let mut run = self.run.lock().unwrap();
        let Some(run) = run
            .as_mut()
            .filter(|r| r.id == id && r.view.state == "running")
        else {
            return false;
        };
        run.aid.push(sample);
        let (value, stars) = match sample {
            Sample::Reading { px, stars } | Sample::FarOut { px, stars } => (Some(px), stars),
            Sample::TooFewStars { stars } => (None, stars),
        };
        let now = Instant::now();
        run.view.cadence_ms = run
            .last_at
            .map(|at| now.duration_since(at).as_secs_f64() * 1000.0);
        run.last_at = Some(now);
        run.view.readings = run.aid.readings();
        run.view.last = Some(LastView {
            frame_id: frame.id,
            state: run.aid.phase().expect("a sample was pushed").as_str(),
            value,
            stars,
            detected: frame.centroids.len() as u32,
        });
        run.view.median = run.aid.median().map(|m| MedianView {
            value: m.px,
            readings: m.readings,
        });
        run.view.best = run.aid.best();
        run.view.trend = run.aid.trend().map(|t| t.as_str());
        run.view.found = run.aid.found();
        run.view.history = run.aid.history();
        true
    }
}

impl Drop for FinderFocus {
    fn drop(&mut self) {
        if let Some(task) = self
            .run
            .get_mut()
            .unwrap()
            .as_mut()
            .and_then(|r| r.task.take())
        {
            task.abort();
        }
    }
}

// Retain only the metadata needed for pairing, never previews or mount data.
#[derive(Clone)]
struct StarInfo {
    id: u64,
    centroids: Vec<Centroid>,
    exposure_ms: f64,
    gain: f64,
}

fn keep_frame(frames: &mut watch::Receiver<Option<StarFrame>>, kept: &mut VecDeque<StarInfo>) {
    if let Some(frame) = frames.borrow_and_update().as_ref() {
        if kept.back().is_none_or(|last| last.id != frame.id) {
            kept.push_back(StarInfo {
                id: frame.id,
                centroids: frame.centroids.clone(),
                exposure_ms: frame.exposure_ms,
                gain: frame.gain,
            });
            if kept.len() > 8 {
                kept.pop_front();
            }
        }
    }
}

async fn raw_frame(
    link: &LinkManager,
    frames: &mut watch::Receiver<Option<StarFrame>>,
    kept: &mut VecDeque<StarInfo>,
) -> Result<RawFrameReply, &'static str> {
    let mut client = link.raw().ok_or("the scope node is not connected")?;
    // Subscribe and keep the current frame before the request starts. A raw
    // reply may arrive after the watch has already moved to the next frame.
    keep_frame(frames, kept);
    let request = time::timeout(RAW_WAIT, client.raw_frame());
    tokio::pin!(request);
    loop {
        tokio::select! {
            result = &mut request => return match result {
                Ok(Ok(reply)) if (reply.width as usize).checked_mul(reply.height as usize) == Some(reply.pixels.len()) => Ok(reply),
                Ok(Ok(_)) => Err("the raw frame request failed"),
                Ok(Err(status)) if status.code() == tonic::Code::Unimplemented => Err("this scope node cannot send raw frames"),
                Ok(Err(_)) => Err("the raw frame request failed"),
                Err(_) => Err("no raw frame arrived within 15 s"),
            },
            changed = frames.changed() => {
                changed.map_err(|_| "the scope node is not connected")?;
                keep_frame(frames, kept);
            }
        }
    }
}

async fn pair_frame(
    id: u64,
    frames: &mut watch::Receiver<Option<StarFrame>>,
    kept: &mut VecDeque<StarInfo>,
) -> Result<Option<StarInfo>, &'static str> {
    let wait = async {
        loop {
            keep_frame(frames, kept);
            if let Some(frame) = kept.iter().find(|frame| frame.id == id) {
                return Ok(frame.clone());
            }
            frames
                .changed()
                .await
                .map_err(|_| "the scope node is not connected")?;
        }
    };
    match time::timeout(RAW_WAIT, wait).await {
        Ok(result) => result.map(Some),
        Err(_) => Ok(None), // Its own star frame never came; skip these pixels.
    }
}

async fn focus_loop(
    shared: &Weak<FinderFocus>,
    id: u64,
    link: &LinkManager,
) -> Result<(), &'static str> {
    let mut frames = link.frames();
    let mut kept = VecDeque::with_capacity(8);
    let mut settings = None;
    loop {
        let raw = raw_frame(link, &mut frames, &mut kept).await?;
        let Some(frame) = pair_frame(raw.id, &mut frames, &mut kept).await? else {
            continue;
        };
        let current = (frame.exposure_ms, frame.gain);
        if settings.is_some_and(|first| first != current) {
            return Err("the finder camera's exposure or gain changed during the run");
        }
        settings = Some(current);
        let centroids = frame.centroids.clone();
        let sample = tokio::task::spawn_blocking(move || {
            measure(&raw.pixels, raw.width, raw.height, &centroids)
        })
        .await
        .map_err(|_| "the focus reading of a raw frame failed")?;
        let Some(shared) = shared.upgrade() else {
            return Ok(());
        };
        if !shared.push(id, &frame, sample) {
            return Ok(());
        }
    }
}

pub(crate) fn routes() -> Router<AppState> {
    Router::new().route(
        "/api/finder-camera/focus",
        get(get_focus).post(start_focus).delete(stop_focus),
    )
}

async fn get_focus(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({"focus": state.finder_focus().view()}))
}

async fn start_focus(
    State(state): State<AppState>,
    _write: SessionWrite,
    ask: Result<Json<FocusAsk>, JsonRejection>,
) -> Result<Json<serde_json::Value>, ApiError> {
    sim_json(ask)?;
    let view = state
        .finder_focus()
        .start(
            state.link(),
            state.session().snapshot(state.clock.now()).camera_settings,
            state.clock.clone(),
        )
        .map_err(|refusal| ApiError::new(refusal.status, refusal.message))?;
    Ok(Json(serde_json::json!({"focus": view})))
}

async fn stop_focus(
    State(state): State<AppState>,
    _write: SessionWrite,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({"focus": state.finder_focus().stop()}))
}
