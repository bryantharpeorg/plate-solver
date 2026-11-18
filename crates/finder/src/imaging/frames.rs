//! Receiving an imaging frame and saving it as FITS (031 US2): the view of the
//! latest frame, the task that follows the node's stream, the console's own
//! ids, and the file a whole frame is saved under, never replacing one there.
//! US4 adds the checks on what the node sends; US5 the frame-in-flight
//! refusals, abort and the node's own copy; US9 the lease's check in `take`.

use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

use scope_link::proto;
use tokio::sync::watch;

use super::{fits, preview, Held, Imaging, Refusal, RoiView};
use crate::{link::ImagingClient, session::Clock};
use fits::CardValue;

/// The most bytes of samples the console accepts for one frame (FR-010).
pub const FRAME_LIMIT: u64 = 256 * 1024 * 1024;
/// How long after the last chunk the next may take (FR-010).
pub const CHUNK_WAIT: Duration = Duration::from_secs(30);
/// The header may take twice the exposure plus this: the node may try twice
/// (FR-010).
pub const HEADER_SLACK: Duration = Duration::from_secs(150);

/// The header's wait, twice the exposure plus `HEADER_SLACK`, and its whole
/// seconds rounded up, as the failure's words give them (152 for a 1000 ms
/// exposure, 151 for 500 ms).
fn header_wait(exposure_ms: f64) -> (Duration, u64) {
    let wait = HEADER_SLACK + Duration::from_secs_f64(exposure_ms * 2.0 / 1000.0);
    (wait, wait.as_secs_f64().ceil() as u64)
}

/// The latest imaging frame as the view shows it. Fields appear as the frame
/// moves on: the first six at once, the rest from the header on, and the last
/// four when the frame is whole, saved or failed (FR-009). Non-exhaustive:
/// later stories add parts.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[non_exhaustive]
pub struct FrameView {
    pub id: u64,
    /// `exposing`, `receiving`, `saved`, `failed` or `aborted`.
    pub state: &'static str,
    pub purpose: String,
    pub exposure_ms: f64,
    /// Console Unix seconds.
    pub requested_at: f64,
    /// Unbinned sensor pixels: the region the console's settings held when the
    /// frame was asked for, replaced by the header's when the header carries
    /// one (owner decision 16).
    pub roi: RoiView,
    /// The set temperature the settings held, with the cooler on, when the
    /// frame was asked for; the file's `SET-TEMP` is written from it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub set_temp_c: Option<f64>,
    // From the header on; every `Option` below is skipped while it is `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bits: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bayer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gain: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binning: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temp_c: Option<f64>,
    /// The exposure's start in console Unix seconds, and `DATE-OBS` as UTC to
    /// the millisecond.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_obs: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub received: Option<u64>,
    /// The node's own copy of the frame, and the reason there is none, when
    /// the frame was asked for with `save_at_node` (FR-009).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_save_error: Option<String>,
    /// Console Unix seconds: when the last byte was in hand, and when the file
    /// was in place.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub received_at: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub saved_at: Option<f64>,
    /// `frames/<UTC date>/<id>.fits`, relative to the imaging folder.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// `/api/imaging/frames/<id>/preview.jpg`, set when the frame is saved
    /// and its preview could be made (FR-018).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// The words of each state that is not `ready`, as `set` refuses them (FR-008).
fn state_refusal(state: &str, error: Option<&str>) -> Refusal {
    match state {
        "no_link" => Refusal::conflict("the scope node is not connected"),
        "no_service" => Refusal::conflict("this scope node has no imaging service"),
        "no_camera" => Refusal::conflict("this scope node has no imaging camera"),
        _ => Refusal::conflict(format!(
            "the imaging camera is not answering: {}",
            error.unwrap_or_default()
        )),
    }
}

/// The largest six-digit id already saved in the date's folder (FR-011).
fn largest_id(root: &Path, date: &str) -> u64 {
    let Ok(entries) = std::fs::read_dir(root.join("frames").join(date)) else {
        return 0;
    };
    entries
        .filter_map(|entry| {
            let name = entry.ok()?.file_name();
            let digits = name.to_str()?.strip_suffix(".fits")?;
            (digits.len() == 6 && digits.bytes().all(|b| b.is_ascii_digit()))
                .then(|| digits.parse::<u64>().ok())
                .flatten()
        })
        .max()
        .unwrap_or(0)
}

/// `Imaging::take` (FR-008): the checks, the id and everything the frame task
/// needs, under one acquisition of the state's lock. US5 adds the
/// frame-in-flight refusal and US9 the lease's check, which `held` makes
/// between the state's and the frame's.
pub(super) fn take(
    imaging: &Imaging,
    purpose: &str,
    held: impl FnOnce(&Held) -> Result<(), Refusal>,
) -> Result<FrameTask, Refusal> {
    let mut guard = imaging.state.lock().expect("imaging state");
    let (client, settings, camera) = match guard.state {
        "ready" => (
            guard.client.clone().expect("a ready imaging client"),
            guard.settings.clone().expect("the settings in force"),
            guard.camera.clone().expect("a described camera"),
        ),
        state => return Err(state_refusal(state, guard.error.as_deref())),
    };
    // The lease's check (FR-029), between the state's and the frame's.
    held(&guard)?;
    // A second frame while one is exposing or receiving is refused (FR-008).
    if guard
        .frame
        .as_ref()
        .is_some_and(|frame| matches!(frame.state, "exposing" | "receiving"))
    {
        return Err(Refusal::conflict("an exposure is already in progress"));
    }
    // The id is the console's own count (FR-011): it starts after the largest
    // id in the folder of the take's UTC date, and never falls.
    let (date, _) = fits::utc(imaging.clock.now());
    if guard.count_date.as_deref() != Some(date.as_str()) {
        guard.count = guard.count.max(largest_id(&imaging.root, &date));
        guard.count_date = Some(date);
    }
    let id = guard.count + 1;
    guard.count = id;
    let frame = FrameView {
        id,
        state: "exposing",
        purpose: purpose.to_string(),
        exposure_ms: settings.exposure_ms,
        requested_at: imaging.clock.now(),
        roi: settings.roi,
        set_temp_c: settings.cooler_on.then_some(settings.set_temp_c).flatten(),
        width: None,
        height: None,
        bits: None,
        bayer: None,
        gain: None,
        offset: None,
        binning: None,
        temp_c: None,
        start: None,
        date_obs: None,
        bytes: None,
        received: None,
        node_file: None,
        node_save_error: None,
        received_at: None,
        saved_at: None,
        file: None,
        preview: None,
        error: None,
    };
    guard.frame = Some(frame.clone());
    drop(guard);
    imaging.changes.send_modify(|count| *count += 1);
    Ok(FrameTask {
        held: imaging.state.clone(),
        changes: imaging.changes.clone(),
        client,
        root: imaging.root.clone(),
        clock: imaging.clock.clone(),
        offset_s: imaging.link.status().borrow().offset_s.unwrap_or(0.0),
        frame,
        model: camera.model,
        serial: camera.serial,
        forward_kib_per_s: settings.forward_kib_per_s,
        save: settings.save_at_node,
    })
}

/// One frame in flight: what its task needs beside the state it reports to.
pub(super) struct FrameTask {
    held: Arc<Mutex<Held>>,
    changes: watch::Sender<u64>,
    /// The client the console aborts this frame's exposure with (US5).
    pub(super) client: ImagingClient,
    root: PathBuf,
    clock: Arc<dyn Clock>,
    /// Node time less console time, read when the frame was asked for.
    offset_s: f64,
    frame: FrameView,
    model: String,
    serial: Option<String>,
    forward_kib_per_s: Option<u32>,
    /// Whether the node also keeps a copy of this frame (FR-008).
    save: bool,
}

impl FrameTask {
    /// The latest frame moves on; anyone waiting on it wakes.
    fn publish(&self) {
        self.held.lock().expect("imaging state").frame = Some(self.frame.clone());
        self.changes.send_modify(|count| *count += 1);
    }

    /// The frame has ended; it is kept beside the latest as the last one, and
    /// its task is no longer in flight (US5).
    fn finish(&self) {
        let mut held = self.held.lock().expect("imaging state");
        held.frame = Some(self.frame.clone());
        held.ended = Some(self.frame.clone());
        held.frame_task = None;
        drop(held);
        self.changes.send_modify(|count| *count += 1);
    }
}

/// How one frame's task ended: saved, failed with its words, or aborted
/// because the observer stopped it (FR-009, FR-013).
enum Outcome {
    Saved,
    Failed(String),
    Aborted,
}

/// Follow one exposure from the node and save the whole frame (031 US2): a
/// failure ends the frame `failed` with its words, an abort ends it `aborted`
/// (US5).
pub(super) async fn run(task: FrameTask) {
    let mut task = task;
    match follow(&mut task).await {
        Outcome::Saved => {}
        Outcome::Failed(error) => {
            task.frame.state = "failed";
            task.frame.error = Some(error);
            task.finish();
        }
        Outcome::Aborted => {
            task.frame.state = "aborted";
            task.finish();
        }
    }
}

/// The exposure, the chunks and the save. Answers how the frame ended.
async fn follow(task: &mut FrameTask) -> Outcome {
    let request = proto::ExposeRequest {
        exposure_ms: task.frame.exposure_ms,
        purpose: task.frame.purpose.clone(),
        max_bytes_per_s: task
            .forward_kib_per_s
            .map(|kib| kib.saturating_mul(1024))
            .unwrap_or(0),
        save: task.save,
    };
    // The call and the first message are both within twice the exposure plus
    // the node's second try (FR-010).
    let (wait, whole) = header_wait(task.frame.exposure_ms);
    let first = tokio::time::timeout(wait, async {
        let mut stream = task.client.expose(request).await?;
        let first = stream.message().await?;
        Ok::<_, tonic::Status>((stream, first))
    })
    .await;
    let (mut stream, mut header) = match first {
        Err(_) => {
            return Outcome::Failed(format!("no imaging frame header arrived within {whole} s"))
        }
        // A status that ends the frame before the header, as the call's own
        // answer or as the stream's first item: an abort ends the frame
        // `aborted`, anything else fails it (FR-013).
        Ok(Err(status)) if status.code() == tonic::Code::Aborted => return Outcome::Aborted,
        Ok(Err(status)) => {
            return Outcome::Failed(format!("the exposure failed: {}", status.message()))
        }
        Ok(Ok((
            stream,
            Some(proto::ExposeEvent {
                event: Some(proto::expose_event::Event::Header(header)),
            }),
        ))) => (stream, header),
        // No first message, or a first message that is not a header.
        Ok(Ok((_, None))) | Ok(Ok((_, Some(_)))) => {
            return Outcome::Failed("the scope node sent no imaging frame header".to_string())
        }
    };
    // The header's byte count must be its width times its height times its
    // bytes a sample, above 0 and at most `FRAME_LIMIT` (FR-010). A size too
    // large to hold is one that does not add up.
    let bytes_a_sample = match header.bits {
        8 | 16 => u64::from(header.bits / 8),
        _ => return Outcome::Failed("the imaging frame's header does not add up".to_string()),
    };
    let expected = u64::from(header.width)
        .checked_mul(u64::from(header.height))
        .and_then(|pixels| pixels.checked_mul(bytes_a_sample));
    if header.byte_count == 0
        || header.byte_count > FRAME_LIMIT
        || expected != Some(header.byte_count)
    {
        return Outcome::Failed("the imaging frame's header does not add up".to_string());
    }
    // The header fills the view, the state becomes `receiving`, and the
    // header's region, when it carries one, replaces the settings'.
    let start = header.start_s - task.offset_s;
    let (date, date_obs) = fits::utc(start);
    {
        let frame = &mut task.frame;
        frame.state = "receiving";
        frame.width = Some(header.width);
        frame.height = Some(header.height);
        frame.bits = Some(header.bits);
        frame.bayer = (!header.bayer.is_empty()).then_some(header.bayer.clone());
        frame.gain = header.gain;
        frame.offset = header.offset;
        frame.binning = Some(header.binning);
        frame.temp_c = header.temp_c;
        frame.start = Some(start);
        frame.date_obs = Some(date_obs);
        frame.bytes = Some(header.byte_count);
        frame.received = Some(0);
        // The node's own copy, when the frame was asked for with `save` (US5).
        frame.node_file = (!header.node_file.is_empty()).then_some(header.node_file.clone());
        frame.node_save_error =
            (!header.node_save_error.is_empty()).then_some(header.node_save_error.clone());
        if let Some(roi) = header.roi.take() {
            frame.roi = RoiView::from(roi);
        }
    }
    task.publish();
    let mut samples = Vec::new();
    let mut received: u64 = 0;
    loop {
        if received >= header.byte_count {
            // The frame is whole; the stream is dropped without reading on.
            task.frame.received = Some(header.byte_count);
            task.frame.received_at = Some(task.clock.now());
            drop(stream);
            return match save(task, &samples, &date, header.exposure_ms).await {
                Some(error) => Outcome::Failed(error),
                None => Outcome::Saved,
            };
        }
        // Each chunk is within `CHUNK_WAIT` of the last (FR-010).
        match tokio::time::timeout(CHUNK_WAIT, stream.message()).await {
            Err(_) => return Outcome::Failed("the imaging frame stalled".to_string()),
            Ok(Ok(Some(proto::ExposeEvent {
                event: Some(proto::expose_event::Event::Chunk(chunk)),
            }))) => {
                // A chunk past the header's count is one too many (FR-010).
                if received + chunk.len() as u64 > header.byte_count {
                    return Outcome::Failed(
                        "the scope node sent more bytes than the header said".to_string(),
                    );
                }
                samples.extend_from_slice(&chunk);
                received += chunk.len() as u64;
                task.frame.received = Some(received);
                task.publish();
            }
            // The end of the stream, an error on it, or an event that is not a
            // chunk, before the header's count is reached.
            Ok(_) => {
                return Outcome::Failed(format!(
                    "the imaging frame was incomplete: {received} of {} bytes",
                    header.byte_count
                ))
            }
        }
    }
}

/// Save the whole frame on the blocking pool (FR-011), under the date of the
/// exposure's start, make its preview in the same step (FR-017) and keep it
/// among the last `PREVIEWS_KEPT` (FR-018), and stamp the view. Answers the
/// words of a failure. A preview that cannot be made does not fail the frame:
/// `preview` is then absent.
async fn save(
    task: &mut FrameTask,
    samples: &[u8],
    date: &str,
    header_exposure_ms: f64,
) -> Option<String> {
    let relative = format!("frames/{date}/{:06}.fits", task.frame.id);
    let image = fits::FitsImage {
        width: task.frame.width.expect("the header's width"),
        height: task.frame.height.expect("the header's height"),
        bits: task.frame.bits.expect("the header's bits"),
        samples: samples.to_vec(),
        cards: cards(
            &task.frame,
            header_exposure_ms,
            &task.model,
            task.serial.as_deref(),
        ),
    };
    let bytes = fits::encode(&image);
    let root = task.root.clone();
    let target = relative.clone();
    let raw = image.samples;
    let (width, height, bits) = (image.width, image.height, image.bits);
    let bayer = task.frame.bayer.clone();
    let saved = tokio::task::spawn_blocking(move || {
        let picture = preview::rgb(&raw, width, height, bits, bayer.as_deref());
        write(&root, &target, &bytes).map(|()| preview::jpeg(&picture))
    })
    .await
    .expect("the frame writer cannot be cancelled");
    let jpeg = match saved {
        Err(error) => return Some(format!("could not save the imaging frame: {error}")),
        Ok(jpeg) => jpeg,
    };
    if let Some(jpeg) = jpeg {
        let mut held = task.held.lock().expect("imaging state");
        held.previews.push((task.frame.id, jpeg));
        while held.previews.len() > preview::PREVIEWS_KEPT {
            held.previews.remove(0);
        }
        drop(held);
        task.frame.preview = Some(format!("/api/imaging/frames/{}/preview.jpg", task.frame.id));
    }
    task.frame.state = "saved";
    task.frame.file = Some(relative);
    task.frame.saved_at = Some(task.clock.now());
    task.finish();
    None
}

/// The saved frame's cards (FR-012): the plan's list, in order, each written
/// only when the frame has it. `XORGSUBF` and `YORGSUBF` are the frame's
/// region's origin in binned pixels.
fn cards(
    frame: &FrameView,
    header_exposure_ms: f64,
    model: &str,
    serial: Option<&str>,
) -> Vec<(String, CardValue)> {
    let binning = frame.binning.unwrap_or(1).max(1);
    let mut cards = vec![
        (
            "DATE-OBS".to_string(),
            CardValue::Text(frame.date_obs.clone().expect("a started frame")),
        ),
        (
            "EXPTIME".to_string(),
            CardValue::Real(header_exposure_ms / 1000.0),
        ),
    ];
    if let Some(gain) = frame.gain {
        cards.push(("GAIN".to_string(), CardValue::Real(gain)));
    }
    if let Some(offset) = frame.offset {
        cards.push(("OFFSET".to_string(), CardValue::Real(offset)));
    }
    cards.push(("XBINNING".to_string(), CardValue::Int(i64::from(binning))));
    cards.push(("YBINNING".to_string(), CardValue::Int(i64::from(binning))));
    cards.push((
        "XORGSUBF".to_string(),
        CardValue::Int(i64::from(frame.roi.x / binning)),
    ));
    cards.push((
        "YORGSUBF".to_string(),
        CardValue::Int(i64::from(frame.roi.y / binning)),
    ));
    if let Some(temp_c) = frame.temp_c {
        cards.push(("CCD-TEMP".to_string(), CardValue::Real(temp_c)));
    }
    if let Some(set_temp_c) = frame.set_temp_c {
        cards.push(("SET-TEMP".to_string(), CardValue::Real(set_temp_c)));
    }
    if let Some(bayer) = &frame.bayer {
        cards.push(("BAYERPAT".to_string(), CardValue::Text(bayer.clone())));
    }
    cards.push(("INSTRUME".to_string(), CardValue::Text(model.to_string())));
    if let Some(serial) = serial {
        cards.push(("SERIALNO".to_string(), CardValue::Text(serial.to_string())));
    }
    // The purpose's label is the FITS convention's value (owner decision 2).
    let imagetyp = match frame.purpose.as_str() {
        "dark" => "DARK",
        "flat" => "FLAT",
        _ => "LIGHT",
    };
    cards.push((
        "IMAGETYP".to_string(),
        CardValue::Text(imagetyp.to_string()),
    ));
    cards.push((
        "ROWORDER".to_string(),
        CardValue::Text("TOP-DOWN".to_string()),
    ));
    cards.push(("FRAMEID".to_string(), CardValue::Int(frame.id as i64)));
    cards.push((
        "PURPOSE".to_string(),
        CardValue::Text(frame.purpose.clone()),
    ));
    cards
}

/// The file's place under the imaging folder: the relative name the view shows
/// and the error words use, split for the platform's separators.
fn place(root: &Path, relative: &str) -> PathBuf {
    let mut path = root.to_path_buf();
    for part in relative.split('/') {
        path.push(part);
    }
    path
}

/// Write the frame under a `.part` name and rename it into place, never
/// replacing a file that is already there (FR-011). Answers the error the
/// caller prefixes with `could not save the imaging frame: `.
fn write(root: &Path, relative: &str, bytes: &[u8]) -> Result<(), String> {
    let file = place(root, relative);
    let part = place(root, &format!("{relative}.part"));
    std::fs::create_dir_all(file.parent().expect("the frames folder"))
        .map_err(|error| error.to_string())?;
    let mut out = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&part)
        .map_err(|error| error.to_string())?;
    out.write_all(bytes)
        .and_then(|()| out.flush())
        .map_err(|error| error.to_string())?;
    drop(out);
    if file.try_exists().map_err(|error| error.to_string())? {
        std::fs::remove_file(&part).map_err(|error| error.to_string())?;
        return Err(format!("{relative} already exists"));
    }
    std::fs::rename(&part, &file).map_err(|error| error.to_string())
}
