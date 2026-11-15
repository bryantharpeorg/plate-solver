//! Recordings (025): star frames, live reports and previews written to a folder on request.

use crate::{
    link::{LinkStatus, RawClient},
    live::LiveReport,
    session::{Clock, Session},
};
use scope_link::proto::{RawFrameReply, StarFrame};
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    collections::HashSet,
    fmt,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    sync::{broadcast, watch},
    task::{AbortHandle, JoinHandle},
    time,
};

/// The `format` of every `recording.json` this console writes.
pub const FORMAT: &str = "finder-recording/1";
/// Star frames after which a recording ends by itself (owner decision 3).
pub const FRAME_LIMIT: u64 = 3600;
/// Most raw frames one request may ask for (owner decision 6).
pub const MAX_RAW_FRAMES: u8 = 10;
/// How long one raw frame may take to arrive.
pub const RAW_WAIT: Duration = Duration::from_secs(15);

/// Progress of the recording's latest raw-frame request.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct RawView {
    /// Raw frames requested.
    pub wanted: u8,
    /// PNGs saved.
    pub saved: u8,
    /// Why the request ended early; absent on completion or recording end.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<&'static str>,
}

/// Why a raw-frame request cannot begin, checked in this order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawRefusal {
    /// No recording is on.
    NotRecording,
    /// The paired link has no raw client.
    NotConnected,
    /// A raw-frame request is still running.
    Busy,
    /// An axis move, GoTo, Track or the mount is moving.
    Moving,
}

/// True while anything moves (spec FR-013); built in api.rs.
pub type Moving = Arc<dyn Fn() -> bool + Send + Sync>;

/// A recording as `GET /api/session` reports it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct RecordingView {
    /// Whether the recording is still accepting star frames and live reports.
    pub on: bool,
    /// The folder's name under the recordings folder.
    pub folder: String,
    /// Console Unix seconds at the start.
    pub since: f64,
    /// Star frame lines written.
    pub frames: u64,
    /// Live report lines written.
    pub solves: u64,
    /// Previews written.
    pub previews: u64,
    /// `stopped`, `limit`, `write_failed` or `node_restarted`; absent while on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_reason: Option<&'static str>,
    /// Latest raw-frame request, retained when the recording ends.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<RawView>,
}

/// What a recording reads, from the link and the live loop.
pub struct Sources {
    /// The link's newest star frame, with intermediate frames allowed to be missed.
    pub frames: watch::Receiver<Option<StarFrame>>,
    /// Link telemetry, including node time less console time.
    pub status: watch::Receiver<LinkStatus>,
    /// The same reports sent by the live event stream.
    pub reports: broadcast::Receiver<LiveReport>,
    /// Console receipt and end times.
    pub clock: Arc<dyn Clock>,
    /// Session metadata at the start.
    pub session: Arc<Session>,
}

/// The recordings folder, new folder or initial metadata could not be written.
#[derive(Debug)]
pub struct FolderError(pub io::Error);
impl fmt::Display for FolderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cannot make the recording folder: {}", self.0)
    }
}
impl std::error::Error for FolderError {}

struct Run {
    view: RecordingView,
    folder: PathBuf,
    metadata: Value,
    clock: Arc<dyn Clock>,
    last_id: Option<u64>,
    ids: HashSet<u64>,
    task: Option<JoinHandle<()>>,
    raw_task: Option<AbortHandle>,
}
impl Run {
    fn write_metadata(&self) -> io::Result<()> {
        let part = self.folder.join("recording.json.part");
        fs::write(&part, serde_json::to_vec(&self.metadata)?)?;
        fs::rename(part, self.folder.join("recording.json"))
    }

    // Called under the same lock as every write and count, including from stop.
    fn end(&mut self, reason: &'static str) {
        self.view.on = false;
        if let Some(task) = self.raw_task.take() {
            task.abort();
        }
        self.view.ended_reason = Some(reason);
        self.metadata["stopped_at"] = json!(self.clock.now());
        self.metadata["ended_reason"] = json!(reason);
        self.metadata["frames"] = json!(self.view.frames);
        self.metadata["solves"] = json!(self.view.solves);
        self.metadata["previews"] = json!(self.view.previews);
        if let Err(error) = self.write_metadata() {
            eprintln!("could not finish recording: {error}");
        }
    }

    fn write_failed(&mut self, error: io::Error) {
        eprintln!("could not write recording: {error}");
        self.end("write_failed");
    }

    fn frame(
        &mut self,
        frame: &StarFrame,
        received_at: f64,
        offset_s: Option<f64>,
    ) -> io::Result<()> {
        let preview = frame
            .preview_jpeg
            .as_ref()
            .map(|_| format!("preview-{}.jpg", frame.id));
        if let (Some(name), Some(bytes)) = (&preview, &frame.preview_jpeg) {
            fs::write(self.folder.join(name), bytes)?;
            self.view.previews += 1;
        }
        let settings = frame.settings.as_ref().map(|s| {
            json!({
                "exposure_ms":s.exposure_ms, "gain":s.gain,
                "detection":s.detection.as_ref().map(|d| json!({
                    "sigma":d.sigma, "binning":d.binning, "normalize_rows":d.normalize_rows,
                    "detect_hot_pixels":d.detect_hot_pixels
                }))
            })
        });
        let axis = |p: &scope_link::proto::AxisPositions| {
            json!({
                "azimuth":p.azimuth, "altitude":p.altitude
            })
        };
        let mut line = json!({
            "id":frame.id, "received_at":received_at, "offset_s":offset_s,
            "start_s":frame.start_s, "end_s":frame.end_s,
            "width":frame.width, "height":frame.height,
            "exposure_ms":frame.exposure_ms, "gain":frame.gain, "settings":settings,
            "centroids":frame.centroids.iter().map(|c| [c.x, c.y, c.brightness]).collect::<Vec<_>>(),
            "axis_start":frame.axis_start.as_ref().map(axis),
            "axis_end":frame.axis_end.as_ref().map(axis)
        });
        if let Some(name) = preview {
            line["preview"] = json!(name);
        }
        append(&self.folder.join("frames.jsonl"), &line)?;
        self.last_id = Some(frame.id);
        self.ids.insert(frame.id);
        self.view.frames += 1;
        Ok(())
    }
}

/// Owns the latest recording, which is run-only and never persisted in the session.
pub struct Recorder {
    root: PathBuf,
    limit: u64,
    run: Mutex<Option<Arc<Mutex<Run>>>>,
}
impl Recorder {
    /// Record under `root`, ending after `limit` star frames.
    pub fn new(root: PathBuf, limit: u64) -> Self {
        Self {
            root,
            limit,
            run: Mutex::new(None),
        }
    }
    /// The recordings folder, so `AppState::with_recording_limit` can rebuild the recorder.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Start a recording, or answer the one already on.
    pub fn start(&self, mut sources: Sources) -> Result<RecordingView, FolderError> {
        let mut latest = self.run.lock().expect("recorder");
        if let Some(run) = latest.as_ref() {
            let run = run.lock().expect("recording");
            if run.view.on {
                return Ok(run.view.clone());
            }
        }
        let now = sources.clock.now();
        let utc = utc(now);
        let name = utc.replace(':', "-");
        fs::create_dir_all(&self.root).map_err(FolderError)?;
        let mut folder_name = name.clone();
        let mut suffix = 2_u64;
        loop {
            match fs::create_dir(self.root.join(&folder_name)) {
                Ok(()) => break,
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                    folder_name = format!("{name}-{suffix}");
                    suffix += 1;
                }
                Err(error) => return Err(FolderError(error)),
            }
        }
        // The value already on the watch channel belongs to before this start.
        sources.frames.borrow_and_update();
        let snapshot = sources.session.snapshot(now);
        let metadata = json!({
            "format":FORMAT, "finder":env!("CARGO_PKG_VERSION"),
            "started_at":now, "started_utc":utc,
            "site":snapshot.site, "aim":snapshot.aim,
            "camera_settings":snapshot.camera_settings, "model":snapshot.model,
            "camera":snapshot.node.as_ref().and_then(|n| n.camera.as_ref())
        });
        let run = Run {
            view: RecordingView {
                on: true,
                folder: folder_name.clone(),
                since: now,
                frames: 0,
                solves: 0,
                previews: 0,
                ended_reason: None,
                raw: None,
            },
            folder: self.root.join(folder_name),
            metadata,
            clock: sources.clock.clone(),
            last_id: None,
            ids: HashSet::new(),
            task: None,
            raw_task: None,
        };
        run.write_metadata().map_err(FolderError)?;
        let view = run.view.clone();
        let shared = Arc::new(Mutex::new(run));
        let task = tokio::spawn(record(shared.clone(), sources, self.limit));
        shared.lock().expect("recording").task = Some(task);
        *latest = Some(shared);
        Ok(view)
    }

    /// End the recording on with `stopped`; nothing when none is on.
    pub fn stop(&self) {
        if let Some(run) = self.run.lock().expect("recorder").as_ref() {
            let mut run = run.lock().expect("recording");
            if run.view.on {
                run.end("stopped");
            }
            // An abort alone does not wait for a write running on another worker.
            if let Some(task) = run.task.take() {
                task.abort();
            }
        }
    }

    /// Save `frames` raw frames into the recording on, one after another.
    pub fn request_raw(
        &self,
        frames: u8,
        client: Option<RawClient>,
        moving: Moving,
    ) -> Result<RecordingView, RawRefusal> {
        let latest = self.run.lock().expect("recorder");
        let shared = latest.as_ref().ok_or(RawRefusal::NotRecording)?;
        let mut run = shared.lock().expect("recording");
        if !run.view.on {
            return Err(RawRefusal::NotRecording);
        }
        if run
            .view
            .raw
            .as_ref()
            .is_some_and(|r| r.saved < r.wanted && r.error.is_none())
        {
            return Err(RawRefusal::Busy);
        }
        if moving() {
            return Err(RawRefusal::Moving);
        }
        let client = client.ok_or(RawRefusal::NotConnected)?;
        run.view.raw = Some(RawView {
            wanted: frames,
            saved: 0,
            error: None,
        });
        let task = tokio::spawn(record_raw(shared.clone(), frames, client, moving));
        run.raw_task = Some(task.abort_handle());
        Ok(run.view.clone())
    }

    /// The latest recording, on or ended; `None` before the first start.
    pub fn view(&self) -> Option<RecordingView> {
        self.run
            .lock()
            .expect("recorder")
            .as_ref()
            .map(|run| run.lock().expect("recording").view.clone())
    }
}
impl Drop for Recorder {
    fn drop(&mut self) {
        self.stop();
    }
}

// Stop may acquire the lock after a timeout/reply but before this task can.
// An ended recording retains its progress and never acquires a new raw error.
fn raw_error(run: &Mutex<Run>, error: &'static str) {
    let mut run = run.lock().expect("recording");
    if run.view.on {
        if let Some(raw) = &mut run.view.raw {
            raw.error = Some(error);
        }
    }
}

async fn record_raw(run: Arc<Mutex<Run>>, frames: u8, mut client: RawClient, moving: Moving) {
    for _ in 0..frames {
        if moving() {
            raw_error(&run, "the mount started moving");
            return;
        }
        let reply = match time::timeout(RAW_WAIT, client.raw_frame()).await {
            Ok(Ok(reply)) => reply,
            Ok(Err(status)) => {
                raw_error(
                    &run,
                    if status.code() == tonic::Code::Unimplemented {
                        "this scope node cannot send raw frames"
                    } else {
                        "the raw frame request failed"
                    },
                );
                return;
            }
            Err(_) => {
                raw_error(&run, "no raw frame arrived within 15 s");
                return;
            }
        };
        let shared = run.clone();
        match tokio::task::spawn_blocking(move || save_raw(&shared, reply)).await {
            Ok(true) => {}
            Ok(false) => return,
            Err(_) => {
                raw_error(&run, "the raw frame request failed");
                return;
            }
        }
    }
}

// Encoding belongs on the blocking pool. The same lock orders each complete
// write, rename and count before stop; a queued blocking job checks on first.
fn save_raw(shared: &Mutex<Run>, reply: RawFrameReply) -> bool {
    // GrayImage::from_raw accepts a buffer that is too long, so check exactly.
    if u64::from(reply.width) * u64::from(reply.height) != reply.pixels.len() as u64 {
        raw_error(shared, "the raw frame request failed");
        return false;
    }
    let Some(image) = image::GrayImage::from_raw(reply.width, reply.height, reply.pixels) else {
        raw_error(shared, "the raw frame request failed");
        return false;
    };
    let mut run = shared.lock().expect("recording");
    if !run.view.on {
        return false;
    }
    let name = format!("raw-{}.png", reply.id);
    let part = run.folder.join(format!("{name}.part"));
    let result = image
        .save_with_format(&part, image::ImageFormat::Png)
        .map_err(io::Error::other)
        .and_then(|()| fs::rename(&part, run.folder.join(name)));
    if let Err(error) = result {
        let _ = fs::remove_file(&part);
        run.write_failed(error);
        return false;
    }
    run.view.raw.as_mut().expect("raw request").saved += 1;
    true
}

async fn record(run: Arc<Mutex<Run>>, mut sources: Sources, limit: u64) {
    let mut reports_open = true;
    loop {
        tokio::select! {
            biased;
            changed = sources.frames.changed() => {
                if changed.is_err() { return; }
                let Some(frame) = sources.frames.borrow_and_update().clone() else { continue; };
                let at = sources.clock.now();
                let offset = sources.status.borrow().offset_s;
                let mut run = run.lock().expect("recording");
                if !run.view.on { return; }
                // Check the last written line before touching this frame's preview.
                if run.last_id.is_some_and(|id| frame.id <= id) {
                    run.end("node_restarted");
                    return;
                }
                if let Err(error) = run.frame(&frame, at, offset) {
                    run.write_failed(error);
                    return;
                }
                if run.view.frames >= limit {
                    run.end("limit");
                    return;
                }
            }
            report = sources.reports.recv(), if reports_open => {
                let report = match report {
                    Ok(report) => report,
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => { reports_open = false; continue; }
                };
                let mut run = run.lock().expect("recording");
                if !run.view.on { return; }
                if !run.ids.contains(&report.frame_id) { continue; }
                if let Err(error) = append(&run.folder.join("solves.jsonl"), &report) {
                    run.write_failed(error);
                    return;
                }
                run.view.solves += 1;
            }
        }
    }
}

// Reopen on every append: deletion must fail the next write. No await can split a line.
fn append(path: &Path, value: &impl Serialize) -> io::Result<()> {
    let mut line = serde_json::to_vec(value)?;
    line.push(b'\n');
    OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)?
        .write_all(&line)
}

// Gregorian civil-from-days, the inverse of simulate::parse_start. Euclidean
// division also keeps dates before the Unix epoch on the correct civil day.
fn utc(seconds: f64) -> String {
    let seconds = seconds.floor() as i64;
    let days = seconds.div_euclid(86400) + 719468;
    let era = days.div_euclid(146097);
    let doe = days - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = year + i64::from(month <= 2);
    let within = seconds.rem_euclid(86400);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        within / 3600,
        within / 60 % 60,
        within % 60
    )
}
