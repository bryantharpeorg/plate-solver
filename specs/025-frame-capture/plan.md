# Implementation Plan: Saving Live Frames at the Telescope

**Branch**: `025-frame-capture` | **Date**: 2026-11-14 | **Spec**: [spec.md](spec.md)

This plan records every design decision for the feature. A story that would change one
escalates rather than re-deciding it.

## Summary

Add a `Recorder` to the console. `POST /api/record` starts it in a new folder under the
state folder; it subscribes to the link's newest-frame channel and to the live loop's
reports, and appends each star frame, each live report and each preview to files until
it is stopped, reaches its frame limit or fails a write. US2 adds a scope-link service
that answers one call with the next star frame's 8-bit pixels, and a route that saves a
few of them into the running recording as PNGs. US3 adds a Recording section to the
Setup page.

## Technical Context

**Language/Version**: Rust (MSRV 1.96) for US1 and US2; TypeScript with React 19 for US3.

**Primary Dependencies**: nothing new. `finder` already has `image` with `png`, and
`tokio` with `fs` and `sync`; the scope node already has `tokio` with `sync`.

**Storage**: `recordings/<folder>/` under the console's state folder: `recording.json`,
`frames.jsonl`, `solves.jsonl`, `preview-<id>.jpg` and, with US2, `raw-<id>.png`.
Nothing about a recording is kept in `session.json`: it is run-only.

**Testing**:
- US1: `crates/finder/tests/recording.rs` (new), in-process and against the built binary.
- US2: `crates/scope-node/tests/raw_frame.rs` (new) for the node, and
  `crates/finder/tests/recording_raw.rs` (new) for the console.
- US3: `crates/finder/web/src/components/RecordingSection.test.tsx` (new), Vitest.
- No Playwright test, and no existing one renders the section: the one night check of
  Setup (`crates/finder/web/e2e/session.spec.ts:191`) runs on the session project's
  console, which never pairs a node, and the simulate project's Setup checks are day
  only. A new e2e file would join the default project, which has no node either, and
  joining the simulate project means editing `playwright.config.ts`. Night mode for the
  section rests on its using only existing classes (spec owner decision 11).
- Numbers read back from a file are compared after a `serde_json` round trip (spec
  § User Scenarios, **Comparing numbers**): the workspace's `serde_json` 1.0.150
  (`Cargo.lock:2509`) has no `float_roundtrip`, and no `Cargo.toml` may change.

**Target Platform**: the `finder` console, on the Surface (WSL2) or any desk machine;
for US2 also the scope node on the Pi.

**Constraints**:
- No change under `crates/simulator`; outside US2 none under `crates/scope-node` or
  `crates/scope-link`.
- `web/dist` is committed and must be the build of its source (US3).
- Night mode's red-only rule (US3), kept by existing classes and tokens only; no gate
  checks the section at night (spec owner decision 11).

**Scale/Scope**: one new module, two routes, one additive scope-link service, one page
section.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Nothing touches the solver, detection or `crates/math-core`. |
| II. Never weaken a gate | No existing test, golden, fixture, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS` or `ergane.yaml` is edited. New tests go in new files. |
| III. Tests first, independent expectations | The oracles are the frames and reports each test receives itself, cedar's detector re-run on saved pixels, the image decoder, the simulated start of `2026-01-16T02:00:00Z` and `finder::simulate::parse_start`. No new virtual clock. |
| IV. Critical paths | No `unsafe`, no dependency or `Cargo.toml` change, no database-format change, nothing under `proto/`. The one scope-link change is an additive service in `crates/scope-link/proto`, named for the owner (spec approval 2), as 017 did for its field. |
| V. Standalone Rust, honest licensing | Unaffected. |

## Project Structure

```text
crates/finder/
├── src/recording.rs                    # US1: new; US2: raw frames
├── src/api.rs                          # US1: with_recordings, /api/record, recording field; US2: /api/record/raw
├── src/lib.rs                          # US1: pub mod recording, the route's doc line; US2: the raw route's line
├── src/main.rs                         # US1: recordings/ in the state folder
├── src/link.rs                         # US2: RawClient, LinkManager::raw
├── README.md                           # US1: route and section; US2: raw route
├── tests/recording.rs                  # US1: new
├── tests/recording_raw.rs              # US2: new
└── web/
    ├── src/api.ts                      # US3: SessionRecording, postRecord, postRecordRaw
    ├── src/useSession.ts               # US3: copy `recording` from the session
    ├── src/components/RecordingSection.tsx       # US3: new
    ├── src/components/RecordingSection.test.tsx  # US3: new
    ├── src/components/SetupPage.tsx    # US3: placement
    └── dist/                           # US3: rebuilt
crates/scope-link/proto/scope_node/v1/scope_node.proto   # US2: ScopeNodeRaw (adds only)
crates/scope-node/
├── src/service.rs                      # US2: raw waiters, ScopeNodeRaw, routes
└── tests/raw_frame.rs                  # US2: new
GLOSSARY.md                             # US1: Recording; US2: Raw frame
DESIGN.md                               # US3: the section
.impeccable/design.json                 # US3: the Recording component
```

## Interfaces

### The recorder (`crates/finder/src/recording.rs`, new, US1)

```rust
//! Recordings (025): star frames, live reports and previews written to a folder on request.

/// The `format` of every `recording.json` this console writes.
pub const FORMAT: &str = "finder-recording/1";
/// Star frames after which a recording ends by itself (owner decision 3).
pub const FRAME_LIMIT: u64 = 3600;

/// A recording as `GET /api/session` reports it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RecordingView {
    pub on: bool,
    /// The folder's name under the recordings folder.
    pub folder: String,
    /// Console Unix seconds at the start.
    pub since: f64,
    pub frames: u64,
    pub solves: u64,
    pub previews: u64,
    /// `stopped`, `limit` or `write_failed`; absent while on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_reason: Option<&'static str>,
}

/// What a recording reads.
pub struct Sources {
    pub frames: watch::Receiver<Option<StarFrame>>,
    pub status: watch::Receiver<LinkStatus>,
    pub reports: broadcast::Receiver<LiveReport>,
    pub clock: Arc<dyn Clock>,
    pub session: Arc<Session>,
}

/// The recordings folder, the new recording's folder or its first `recording.json`
/// could not be written. Displays as `cannot make the recording folder: <io error>`.
#[derive(Debug)]
pub struct FolderError(pub std::io::Error);
// impl Display for FolderError, impl std::error::Error for FolderError

pub struct Recorder { /* root, limit, Mutex<Option<Run>> */ }

impl Recorder {
    pub fn new(root: PathBuf, limit: u64) -> Self;
    /// The recordings folder, so `AppState::with_recording_limit` can rebuild the recorder.
    pub fn root(&self) -> &Path;
    /// Start a recording, or answer the one already on.
    pub fn start(&self, sources: Sources) -> Result<RecordingView, FolderError>;
    /// End the recording on with `stopped`; nothing when none is on.
    pub fn stop(&self);
    /// The latest recording, on or ended; `None` before the first start.
    pub fn view(&self) -> Option<RecordingView>;
}
```

`start` with no recording on:
1. `create_dir_all(root)`. Name the folder from the console clock's UTC, formatted
   `YYYY-MM-DDTHH-MM-SSZ` with a civil-from-days conversion written here (no time
   library; `finder::simulate::parse_start` at `crates/finder/src/simulate.rs:242` is
   the inverse and stays untouched). `create_dir` it; on `AlreadyExists` try `-2`, `-3`
   and so on. Either failure is a `FolderError`.
2. Mark the frame now on the channel as seen (`borrow_and_update`), so only frames
   published after the start are written.
3. Write `recording.json` (below) as `recording.json.part`, then rename it. A failure
   here is a `FolderError` too.
4. Spawn the recording task and keep its handle with the run.

The run's view, folder and `recording.json` fields sit behind one `std::sync::Mutex`
shared by the task and the `Recorder`. The task loops on a `biased` `tokio::select!`
with `frames.changed()` first, so a report ready at the same poll as its frame finds the
frame recorded, and `reports.recv()` second:
- **A frame**: read the clock and `status.borrow().offset_s`, then take the lock; if the
  run is no longer on, return. Write its preview if it has one, then its frame line;
  keep its id in a set; count both. At the limit, end with `limit`.
- **A report**: if its `frame_id` is in the set, take the lock; if the run is no longer
  on, return. Write `serde_json::to_string(&report)` as a solve line and count it.
  `Lagged` is skipped; `Closed` leaves only frames.
- **A failed write** prints it to standard error and ends the recording with
  `write_failed`.

Every write, its count and every end happen while holding that lock. `stop` takes the
same lock, sets `on` false and `ended_reason` `stopped`, rewrites `recording.json` with
the counts it then holds, and only then aborts the task. So no line can follow `stop`'s
answer, and `recording.json`'s counts equal the files', on any runtime.

Every line is written synchronously with `std::fs`: `OpenOptions::new().append(true)
.create(true)` on the file, then one `write_all` of the line and its newline. There is no
await inside a write, so aborting the task can't cut a line, and a deleted folder fails
the next open. A preview is written whole with `std::fs::write`, before its line, so a
line never names a missing file. Lines and `recording.json` are serialized with
`serde_json`, which writes each `f64` in its shortest exact form.

Ending (stop, limit or failure): set `on` false and `ended_reason`, and rewrite
`recording.json` with the end fields, printing a failure of that last write and
otherwise ignoring it. A deleted folder therefore prints two lines: the failed write,
then the failed rewrite.

A frame line, on one line in the file:

```json
{"id":7,"received_at":1768528803.5,"offset_s":-1768527800.0,"start_s":1003.1,"end_s":1003.3,
 "width":1456,"height":1088,"exposure_ms":200.0,"gain":1.0,
 "settings":{"exposure_ms":200.0,"gain":1.0,"detection":{"sigma":8.0,"binning":2,"normalize_rows":true,"detect_hot_pixels":true}},
 "centroids":[[700.5,420.25,1234.0]],
 "axis_start":{"azimuth":12,"altitude":-40},"axis_end":{"azimuth":12,"altitude":-40},
 "preview":"preview-7.jpg"}
```

`received_at` is the console clock when the frame arrived and `offset_s` the link's
`LinkStatus::offset_s` then (`crates/finder/src/link.rs:172`), node time less console
time. `settings` is null when the frame has none, its `exposure_ms` and `gain` are null
when automatic, and its `detection` is null when absent. `preview` appears only when the
frame carried one.

`recording.json` at the start:

```json
{"format":"finder-recording/1","finder":"<CARGO_PKG_VERSION>",
 "started_at":1768528801.2,"started_utc":"2026-01-16T02:00:01Z",
 "site":{"lat_deg":40.0,"lon_deg":-75.0},"aim":null,"camera_settings":{…},
 "model":{…},"camera":{…}}
```

taken from `session.snapshot(now)` (`crates/finder/src/session.rs:976`): `site`, `aim`,
`camera_settings` and `model` as `GET /api/session` shows them, and `camera` as its
`node.camera` (`crates/finder/src/session.rs:341`). At the end it gains `stopped_at`,
`ended_reason`, `frames`, `solves` and `previews`.

### The console (`crates/finder/src/api.rs`, US1)

`AppState` (`crates/finder/src/api.rs:93`) gains `recordings: Option<Arc<Recorder>>`, and:

```rust
/// Record on request under `root` (025). The binary passes `recordings/` in its state folder.
pub fn with_recordings(mut self, root: impl Into<PathBuf>) -> Self;
/// End each recording after `frames` star frames instead of `FRAME_LIMIT`. Call after
/// `with_recordings`.
pub fn with_recording_limit(mut self, frames: u64) -> Self;
```

`with_recording_limit` replaces the recorder with `Recorder::new(old.root().to_path_buf(),
frames)`, and does nothing without one.

The route `.route("/api/record", post(set_record))` sits beside `/api/track`
(`crates/finder/src/api.rs:428`). Its body is a `#[serde(deny_unknown_fields)]
RecordEdit { on: bool }`, read through `sim_json` (`crates/finder/src/api.rs:541`) as
`set_track` reads `TrackEdit` (`crates/finder/src/api.rs:677`), behind `SessionWrite`.
`on: true`, in this order: no recorder answers 409 `this console has no recordings
folder`; no link or live loop answers 409 `this console has no live link`; otherwise
`start` with `Sources` from `link.frames()`, `link.status()`, `live.subscribe()`, the
state's clock and session, and a `FolderError` answers 500 with its `to_string()`,
`cannot make the recording folder: <io error>`. `on: false` calls `stop` when there is a
recorder. Both then answer `get_session`.

`SessionResponse` (`crates/finder/src/api.rs:623`) gains

```rust
#[serde(skip_serializing_if = "Option::is_none")]
recording: Option<crate::recording::RecordingView>,
```

filled in `session_response` (`crates/finder/src/api.rs:643`) from
`state.recordings.as_ref().and_then(|r| r.view())`.

### The binary (`crates/finder/src/main.rs`, US1)

After the session file is chosen (`crates/finder/src/main.rs:98`), the state is built
with `.with_recordings(state_dir.join("recordings"))`. Under `--simulate` without
`--state` the folder is already `simulate/` (`crates/finder/src/main.rs:95`). No flag
and no new start-up line.

### `crates/finder/src/lib.rs` (US1)

`pub mod recording;`, and one line at the end of the route list, after the
`POST /api/stop` item that starts at `crates/finder/src/lib.rs:24`, written as it stands
here, with no backticks:
`* POST /api/record: {"on":true} or {"on":false}; records star frames, live reports and previews to a folder.`

### `crates/finder/README.md` (US1)

A row after `POST /api/track` in the `## API` table (`crates/finder/README.md:167`):

```markdown
| `POST /api/record` | JSON `{"on":true}` or `{"on":false}`. Start or stop a recording; answers 200 with the session, 409 or 500 with a JSON `error`. Same-origin writes only. |
```

and a section `## Recording a night` at the end of the file saying: what is recorded;
where (`recordings/<UTC start>/` in the state folder, `simulate/recordings/` under
`--simulate`); each file and the frame line's fields; the 3,600-frame limit and the
three `ended_reason`s; that `recording` in the session is run-only; that a recording
holds the site, times and solves, never the node's address or token, so review one
before committing it to this public repository; and that `Live::start` replays frame
lines, as `crates/finder/tests/live.rs:181` feeds frames.

### `GLOSSARY.md` (US1)

After the **Preview** entry (`GLOSSARY.md:131`):

```markdown
**Recording**:
One folder of what the console saw between the observer's start and stop: each star frame's centroids, settings and axis counts, each live solve and each preview, kept for later tests.
_Avoid_: capture, log, session
```

### The scope link (`crates/scope-link/proto/scope_node/v1/scope_node.proto`, US2)

Appended at the end of the file, after `StopAllReply`
(`crates/scope-link/proto/scope_node/v1/scope_node.proto:142`), not beside
`ScopeNodeStop` (`crates/scope-link/proto/scope_node/v1/scope_node.proto:100`), which the
mount's messages follow:

```proto
// Raw frames on request (025). Every call requires the session's bearer token.
// A node built before 025 lacks this service and answers UNIMPLEMENTED.
service ScopeNodeRaw {
  // Waits for the next star frame the node sends and answers with its pixels.
  rpc RawFrame(RawFrameRequest) returns (RawFrameReply);
}
message RawFrameRequest {}
message RawFrameReply {
  // The star frame these pixels were detected on.
  uint64 id = 1;
  uint32 width = 2;
  uint32 height = 3;
  // 8-bit gray, row-major, width × height bytes, no padding.
  bytes pixels = 4;
}
```

### The scope node (`crates/scope-node/src/service.rs`, US2)

- `Commands` (`crates/scope-node/src/service.rs:83`) gains
  `raw: Vec<tokio::sync::oneshot::Sender<proto::RawFrameReply>>`.
- In `run` (`crates/scope-node/src/service.rs:223`), inside the same
  `if !unresponsive` block that sends the star frame
  (`crates/scope-node/src/service.rs:279`), right after the send: take every waiting
  sender (`std::mem::take`) and answer each with that frame's `id`, `width`, `height`
  and pixels, through a private helper that clones the pixels for each extra waiter and
  moves them into the last. A frame the node doesn't send answers nobody.
- `impl ScopeNodeRaw for NodeService`: `raw_frame` waits for `controls.ready(false)`,
  as `request_preview` does (`crates/scope-node/src/service.rs:386`), pushes a new
  sender and awaits its receiver, mapping a dropped sender to
  `Status::unavailable("the scope node stopped capturing")`. Waiters live in the
  service's shared commands, which outlive a stopped capture task, so in practice the
  console's `RAW_WAIT` is what ends such a call.
- `routes` (`crates/scope-node/src/service.rs:193`) adds
  `ScopeNodeRawServer::with_interceptor(self.clone(), self.runtime.token.clone())`.

### The link (`crates/finder/src/link.rs`, US2)

```rust
/// The largest raw-frame reply the console accepts; a 2592 × 1944 frame is about 5 MB.
pub const RAW_MESSAGE_LIMIT: usize = 16 * 1024 * 1024;

/// Raw-frame calls on the console's paired channel (025).
#[derive(Clone)]
pub struct RawClient { /* ScopeNodeRawClient<InterceptedService<Channel, BearerToken>> */ }
impl RawClient {
    /// Reuse a scope link channel and the session's bearer token; replies up to
    /// `RAW_MESSAGE_LIMIT`.
    pub fn new(channel: Channel, token: &str) -> Result<Self, Status>;
    /// The next star frame's pixels. A node without the service answers `Unimplemented`.
    pub async fn raw_frame(&mut self) -> Result<proto::RawFrameReply, Status>;
}
impl LinkManager {
    /// Raw-frame calls on the current connection; absent while unpaired or disconnected.
    pub fn raw(&self) -> Option<RawClient>;
}
```

`RawClient::new` sets `max_decoding_message_size(RAW_MESSAGE_LIMIT)`. Raw replies share
the paired connection with the heartbeat, the frame stream and Stop (spec owner decision
10). `pair`
(`crates/finder/src/link.rs:313`) builds it next to the `MountClient` and returns a
private `struct Paired { client, frames, camera, mount, raw }` instead of a longer
tuple. `run` publishes it on a new watch channel where it publishes the mount client
(`crates/finder/src/link.rs:376`) and clears it where it clears that
(`crates/finder/src/link.rs:393`). `run` gains a seventh parameter, the raw client's
sender, which is `clippy::too_many_arguments`' limit: it still passes, and a later
parameter would not. `LinkManager::start`'s signature, `MountClient` and the frame arm
don't change.

### Raw frames in the recorder (`crates/finder/src/recording.rs`, US2)

```rust
/// Most raw frames one request may ask for (owner decision 6).
pub const MAX_RAW_FRAMES: u8 = 10;
/// How long one raw frame may take to arrive.
pub const RAW_WAIT: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RawView {
    pub wanted: u8,
    pub saved: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<&'static str>,
}
// RecordingView gains:
//   #[serde(skip_serializing_if = "Option::is_none")]
//   pub raw: Option<RawView>,

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawRefusal { NotRecording, NotConnected, Busy }

impl Recorder {
    /// Save `frames` raw frames into the recording on, one after another.
    pub fn request_raw(&self, frames: u8, client: Option<RawClient>)
        -> Result<RecordingView, RawRefusal>;
}
```

Refusals in this order: no recording on is `NotRecording`; a request whose `saved` is
below `wanted` and has no `error` is `Busy`; no client is `NotConnected`. An accepted
request sets `raw` to `{wanted, saved: 0}` and spawns a task whose `AbortHandle` the run
keeps under its lock: for each frame, `time::timeout(RAW_WAIT, client.raw_frame())`,
then on the blocking pool `image::GrayImage::from_raw(width, height, pixels)` (a size
mismatch is `the raw frame request failed`), saved with `ImageFormat::Png` as
`raw-<id>.png.part` and renamed to `raw-<id>.png`; then, under the lock and only while
the run is on, `saved += 1`. `Unimplemented` sets `this scope node cannot send raw
frames`, a timeout `no raw frame arrived within 15 s`, any other status `the raw frame
request failed`, and a failed write ends the recording with `write_failed` (the task
takes the clock for `stopped_at`). Every end of the recording aborts the raw task and
leaves `raw` as it was, with no `error`.

### The raw route (`crates/finder/src/api.rs`, US2)

`.route("/api/record/raw", post(record_raw))` behind `SessionWrite`, with
`#[serde(deny_unknown_fields)] RawEdit { frames: u8 }` read through `sim_json`; a value
outside 1 to `MAX_RAW_FRAMES` answers 400 with
`frames must be a whole number from 1 to 10`. Then, in this order: no recorder answers
409 `this console has no recordings folder`; `NotRecording`, `Busy` and `NotConnected`
answer 409 with `start a recording first`, `raw frames are already being saved` and
`the scope node is not connected`. Success answers `get_session`. The client comes from
`state.link().and_then(|link| link.raw())`.

### `crates/finder/src/lib.rs` (US2)

One line after US1's in the route list, written as it stands here, with no backticks:
`* POST /api/record/raw: {"frames":N}, N from 1 to 10; saves the next N star frames' pixels into the recording on.`

### `crates/finder/README.md` and `GLOSSARY.md` (US2)

A row after US1's:

```markdown
| `POST /api/record/raw` | JSON `{"frames":N}`, N from 1 to 10. While recording, save the next N star frames' 8-bit pixels as `raw-<id>.png`; progress is `recording.raw` in the session. Needs a scope node built with 025. Same-origin writes only. |
```

and a paragraph in `## Recording a night` on raw frames, their PNGs, `recording.raw`
and its three errors, and that a raw reply shares the scope link's connection, so raw
frames are best asked for while the mount is still: on slow Wi-Fi a transfer can delay
the heartbeat past one second, and the scope node then stops the mount. After
**Recording** in `GLOSSARY.md`:

```markdown
**Raw frame**:
The 8-bit gray pixels of one star frame, exactly as the scope node's detector read them, saved on request as a PNG next to that frame's line.
_Avoid_: raw (alone), RAW file, full-resolution preview
```

### The page (`crates/finder/web/src/`, US3)

`api.ts`, after `SessionTrack`:

```ts
export interface SessionRecording {
  on: boolean;
  folder: string;
  since: number;
  frames: number;
  solves: number;
  previews: number;
  ended_reason?: "stopped" | "limit" | "write_failed";
  raw?: { wanted: number; saved: number; error?: string };
}
```

`recording?: SessionRecording;` in `ObservingSession`
(`crates/finder/web/src/api.ts:230`), and `postRecord(on: boolean)` and
`postRecordRaw(frames: number)`, each written as `postTrack` is
(`crates/finder/web/src/api.ts:296`), posting `{on}` to `/api/record` and `{frames}` to
`/api/record/raw`, and each returning `Promise<ObservingSession>`.

`useSession.ts`: `request()` builds the session one known field at a time
(`crates/finder/web/src/useSession.ts:29`) and drops every other, so it gains one line
right after the `track` line (`crates/finder/web/src/useSession.ts:45`):

```ts
    ...(body?.recording != null && { recording: body.recording }),
```

Without it the real page never sees `recording`, while every test that passes the
session straight to the component still passes; US3-S5 drives the page through it.

`components/RecordingSection.tsx`:

```tsx
export function RecordingSection({ session, onChange }: { session: ObservingSession; onChange?: () => void }) {
  // Nothing without `session.node`; connected or not, the section shows.
  // <section aria-labelledby="recording-title" className="space-y-3">
  //   <h2 id="recording-title" className="section-title">Recording</h2>
  //   …
}
```

Its states, with these words exactly:
- **No recording, or one that ended**: the button `Start recording`
  (`button button-primary`). With no `recording`, a `quiet` paragraph:
  `Saves each star frame's stars, every solve and the previews to the console's
  recordings folder, to test with later.` and no `status` or `alert`.
- **On**: one `role="status"` element holding
  `Recording to {folder} · {frames} frames · {solves} solves`, and while `raw` is
  present and has no `error`, a second line `Raw frames: {saved} of {wanted} saved`.
  The buttons `Stop recording` and `Save 3 raw frames` (`button`); the second is
  disabled while `saved` is below `wanted` and there is no `error`. A raw `error` shows
  as `role="alert"`: `Raw frames stopped: {error}.`
- **Ended**: the status `Saved {folder}: {frames} frames, {solves} solves.`, followed
  for `limit` by ` It stopped at its frame limit.`; for `write_failed`, an alert
  `Recording stopped: the console could not write to its folder.`
- **A refused request**: an alert with the answer's `error`, as `TrackControl` shows one.

Each successful post calls `onChange`; a refused one does not. Write each phrase a test
asserts on one line of JSX, so `textContent` keeps it whole. A raw error alert and a
refused request's alert may show together; no scenario needs them apart.

`SetupPage.tsx`: `<RecordingSection session={session} onChange={onChange} />` right
after `<AimTap … />` (`crates/finder/web/src/components/SetupPage.tsx:179`).

### `DESIGN.md` (US3)

After the Aim-point tap entry (`DESIGN.md:343`), add a plain `###` heading and one
paragraph, like its neighbours. The `>` marks below only set the text apart here; the
entry is not a blockquote:

> ### Recording — built
> On the Setup page, after the aim point, while a scope node is paired: a section titled
> Recording with one primary button, Start recording. While recording, one status line
> names the folder and counts, with Stop recording and Save 3 raw frames as secondary
> buttons; raw progress sits under the status and a failure is an alert. After a
> recording the status says what was saved. Ink on paper with existing tokens only; at
> night it is red like every other section.

### `.impeccable/design.json` (US3)

Every earlier Setup section added a component here with its `DESIGN.md` entry. Append to
`components`, formatted as its neighbours:

```json
{"name": "Recording", "kind": "custom", "refersTo": "button",
 "description": "Built in RecordingSection.tsx on Setup after Aim point, whenever the session has a scope node, connected or not. Start recording is the primary button and POSTs /api/record on. While recording, one status names the folder, frames and solves, with Stop recording and Save 3 raw frames as secondary buttons; raw progress joins the status and a raw error is an alert. After a recording the status says what was saved, and why when it hit its frame limit; a failed write is an alert. Existing ink, paper and red-only night tokens; no new class.",
 "html": "<section class=\"ds-recording\"><h2>Recording</h2><p role=\"status\">Recording to 2026-01-16T02-00-00Z · 12 frames · 9 solves</p><button>Stop recording</button><button>Save 3 raw frames</button></section>",
 "css": ".ds-recording{color:var(--color-ink)}.ds-recording button{border:1px solid var(--color-ink);background:transparent;color:inherit;min-height:44px}"}
```

## Shapes that existing tests pin

These are not changed by any story:
- **The `ScopeNode` service and every existing message.** Two hand-written
  implementations of the trait (`crates/finder/tests/aim_cli.rs:22`,
  `crates/scope-link/tests/link.rs:55`) compile unchanged because the new call is in a
  new service. `StarFrame` gains no field: a test builds it as a literal naming every
  field (`crates/scope-link/tests/link.rs:25`).
- **`LinkManager::start`'s signature and `MountClient`.** Many tests reach the link
  through `with_link`; `crates/finder/tests/link.rs:187` and
  `crates/finder/tests/keyhole_sim.rs:229` call `LinkManager::start` directly, and
  `crates/finder/tests/mount_calls.rs:54` builds `MountClient` itself.
- **`GET /api/session` without a recording.** `recording` is skipped when absent, so
  every existing body is byte for byte what it was.
- **`GET /api/live`.** The recorder is one more subscriber of the same broadcast.
- **The binary's start-up lines.** `crates/finder/tests/simulate_cli.rs` reads the
  address line `finder console: http://`; nothing new is printed.
- **The Setup page with no `node`, or with a `node` and no `recording`.** Existing tests
  read the page's only `status` (`crates/finder/web/src/components/SetupPage.test.tsx:37`)
  and only `alert` (`crates/finder/web/src/components/AimTap.test.tsx:102`); the new
  section adds neither then, and no button whose name another test uses.
- **`useSession`'s other fields.** The one new line copies `recording` only; a test that
  drives the page through the hook
  (`crates/finder/web/src/components/ScopeNodeSection.test.tsx:70`) reads the same
  session as before.

## Decisions and the alternatives rejected

| Decision | Rejected | Why |
|---|---|---|
| Record from the link's frame channel and the live broadcast | A writer inside `active` in `link.rs` | The channel already feeds two readers; a writer in the link loop shares a task with the heartbeat, whose 1 s silence deadline a slow write could trip. A missed frame shows as a gap in the ids. |
| Writes, counts and ends under one lock; `stop` ends before it aborts | Abort the task, then rewrite `recording.json` | An abort does not wait: on a multi-thread runtime the task can finish a line and count it after the rewrite, so the counts and the files disagree. |
| Raw replies on the paired connection | A second connection; a 409 while the mount moves | Spec owner decision 10: no new transport and nothing refused, at the cost of a possible dropped link on slow Wi-Fi, which stops the mount. |
| Tests compare numbers after a `serde_json` round trip | A tolerance; the `float_roundtrip` feature | A tolerance weakens "exactly", and the feature is a `Cargo.toml` change. The files hold the exact shortest form. |
| Record on the console | Save `GET /api/live` with curl | Its events lack brightness, settings, axis counts, the offset and previews. |
| JSON Lines, one synchronous write per line, the file reopened each time | One JSON document | A console that dies mid-night would leave nothing readable, and a deleted folder would go unnoticed behind an open handle. |
| A solve is written only for a recorded frame | Every report | Each solve line then has its frame line. |
| Raw pixels through a new service | A new call on `ScopeNode` | Two existing tests implement that trait and would stop compiling; tonic's default stubs would need a `build.rs` change, a critical path. |
| Raw pixels by a unary call | A raw field on `StarFrame` | Every reader of the frame channel would carry up to 5 MB, the newest-frame channel may drop the very frame asked for, and a test's full literal (`crates/scope-link/tests/link.rs:25`) would stop compiling. |
| Raw frames saved by the console | Saved on the scope node's card | One folder, matched by id, and no SSH at the telescope. |
| 8-bit gray PNG | PGM, FITS | Lossless, about half the size on a night frame, read by `image::open` as the existing fixture is; `finder` already has the `png` feature. |
| A 16 MiB limit on the raw client only | Raising it for the whole link | Only raw replies are large. |
| `pair` returns a named struct | A five-element tuple | Easier to read, and keeps `clippy::type_complexity` away. |
| Folder in the state folder | A `--recordings` flag | `--state` and `--simulate` already keep nights apart. |
| A Setup page section | A top-bar button | The top bar holds motion and Stop; recording is set-up work and must not crowd Stop at night. |

## Verification (every story, before proposing)

- **The factory gates** (`ergane.yaml` at c1ed295):
  - `lint` = `cargo fmt --all --check`,
    `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`,
    `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features --locked`
    and `RUSTDOCFLAGS="-D warnings" cargo doc -p finder --no-deps`;
  - `test` = `cargo test --workspace --all-targets --all-features --locked`,
    `cargo test --workspace --locked --doc`, and then, in `crates/finder/web`,
    `npm ci --no-audit --no-fund`, `npm run typecheck`, `npm run format:check`,
    `npm test`, `npm run build`, `git status --porcelain -- dist | (! grep .)` and
    `CI=1 npm run e2e`.

  Every story runs both, and the judge sees their results.
- **CI in the merge queue** runs every `ci.yml` job, including the finder-page
  build-freshness check and the build on Rust 1.96. Coverage runs only by hand
  (constitution 1.1.0).

## Prerequisites before this spec is flipped to `ready`

1. **The earlier specs have landed:** `013-live-finder`, `014-goto`, `016-mcp`,
   `017-console-stop-confirm`, `019-night-legibility` and `020-live-chart-calm`. Not yet
   met: 017's last story landed at c1ed295 and 020-live-chart-calm's US1 at 2508423, but
   its US2 has not. That US2 writes `DESIGN.md`, `crates/finder/web/dist` and
   `crates/finder/web/src/api.ts`, as US3 does, and two rebuilds of `dist` in flight
   always conflict. Dispatching 025 first instead is the owner's call; it would mean
   dropping `020-live-chart-calm` from `depends_on_landed` and naming 025 in that spec's
   shared writes.
2. **`main` is still as read.** Every citation here was read at c1ed295 and re-checked at
   2508423, where only `crates/finder/src/solve.rs` had moved. Once 020-live-chart-calm's
   US2 lands, re-read `DESIGN.md:343`, `crates/finder/web/src/api.ts:230` and
   `crates/finder/web/src/api.ts:296`, and any other cited file `main` has changed, and
   re-run the validator.
3. **The owner decisions are settled and the approvals given** (spec § Owner decisions,
   § Owner approvals). An answer other than the recommended one is written into the trio
   and re-validated before the flip, never left to a story.
4. **No operator-staged files are needed.**
5. **Outside the factory, after US2 lands:** rebuild the console and the scope node from
   that `main` before raw frames are asked for at the telescope.

## Plan traps

Lessons from building 012 to 019 on this factory, each with its mechanism and the check
that keeps it from recurring. They bind every story.

1. **Gate evidence or nothing.** The judge credits only gate results and the story's
   diff. *Check:* every Then clause names a committed test that the `test` gate runs.
2. **A named command is read literally.** *Check:* commands are copied from
   § Verification.
3. **One evidence budget.** *Check:* each story keeps its diff under 64 KiB, measured
   with `git diff <base> -- . ':(exclude)crates/finder/web/dist' | wc -c`.
4. **The sandbox has only the gates' tools.** *Check:* no task runs a tool outside them.
5. **A spec fixed mid-epic needs a relaunch.** The judge grades the spec as dispatched.
   *Check:* no builder edits `specs/025-frame-capture/` or `.specify/`.
6. **An oracle copied from the code proves nothing.** *Check:* tests parse the files
   with `serde_json` themselves and compare them with frames and reports they received
   on their own subscriptions, re-run `star_detection::detect_stars` on saved pixels,
   and decode PNGs with `image`. No test reads a recording through `Recorder`. Numbers
   are compared after the round trip of spec § User Scenarios, never with a tolerance:
   the workspace's `serde_json` parses about one `f64` in eight back one ULP off.
7. **A new call on `ScopeNode` breaks two tests.** *Check:* the call is in the new
   `ScopeNodeRaw` service, and `ScopeNode`, `StarFrame` and every existing message keep
   their calls and fields.
8. **tonic's 4 MiB default.** A 1456 × 1088 test frame passes without the raised limit.
   *Check:* US2-S6 sends a 2592 × 1944 frame.
9. **The newest-frame channel.** A recorder that writes the frame already on the channel
   at its start, or a solve of an earlier frame, gives solve lines without frame lines.
   *Check:* US1-S1 asserts every solve line names a frame line.
10. **A cut line, or a line after the stop.** Aborting a task at an await can leave half
    a line, and an abort does not wait for a write already running on another worker.
    *Check:* lines are written synchronously, one call each, under the run's lock that
    `stop` also takes; PNGs and `recording.json` are written under a `.part` name and
    renamed; US1-S1 asserts that nothing follows the stop.
11. **The Setup page's only status and alert.** *Check:* US3-S1 asserts the section shows
    neither with a node and no recording.
12. **The bundle must be rebuilt.** *Check:* US3 commits `crates/finder/web/dist` after
    `npm run build`, so that the gate's `git status --porcelain -- dist` finds nothing.
13. **Each worktree builds on its own.** *Check:* no story sets `CARGO_TARGET_DIR`, and
    test folders go under the workspace's `target/`, as existing tests put them.
14. **`useSession` copies known fields only.** A component test that passes the session
    straight in passes while the real page drops `recording`. *Check:* US3 writes
    `crates/finder/web/src/useSession.ts`, and US3-S5 drives `SetupPage` through it.
15. **The rig's order.** Connecting a `SimMount` after `push_to` re-points the camera
    near the horizon. *Check:* the shared setup connects the mount first.

## Risks

| Risk | Mitigation |
|---|---|
| A forgotten recording fills the disk. | The 3,600-frame limit, and the Setup page shows a running recording and its counts. |
| A recording with the site is published. | Owner decision 9, and the README's note; no address or token is ever written. |
| Raw requests slow the scope node. | One pixel clone per waiting call; at most ten frames a request, one request at a time. |
| A recorded frame is missed. | Gaps show in the ids; the cadence of 200 ms or more leaves the writer idle most of the time. |
| An older scope node. | It answers `Unimplemented`, which the page and session report as `this scope node cannot send raw frames`. |
| A raw reply on slow Wi-Fi delays the heartbeat past 1 s, the link drops and the node's watchdog stops the mount mid-GoTo, Track or axis move. | Spec owner decision 10: at most ten frames a request, one request at a time, and the README says to ask while the mount is still. The failure stops the mount, the safe direction. |
| `020-live-chart-calm` US2 (`ready`) writes `DESIGN.md`, `dist` and `api.ts`, as US3 does. | 025 depends on it landing (Prerequisite 1), and its cited lines are re-read then (Prerequisite 2). |
| Draft 020 "mount-sky" (pull request 207) writes `link.rs` (its frame arm, and a filter on `LinkManager`), `api.rs`, `lib.rs`, `main.rs`, `crates/finder/README.md`, `GLOSSARY.md`, `DESIGN.md`, `dist` and `api.ts`: files of US1, US2 and US3. | Whichever is flipped second is re-read, declares the other in `depends_on_landed` and its shared writes, and is re-validated. |
| A story's diff nears 64 KiB. | The probes measured US1 at 51,667 bytes, US2 at 46,881 and US3 at 16,699. Tests may share `crates/finder/tests/live/support.rs` with `#[path]`; US2's test repeats US1's setup, about 6 KB. |

## Refinement Record

Drafted on 2026-11-14 by the plate-solver operator against `main` at c1ed295, from gap
3 of the field findings of 13–14 November and a read of the frame path from the camera
port to `GET /api/live`. The same day the three stories were probe-built from the draft
trio in throwaway worktrees of c1ed295, tests first, with no existing test edited, and
the trio was reviewed against the code:

- **US1** (branch `probe/025-us1`): seven tests, all passing; `fmt`, `clippy -p finder`,
  `cargo doc -p finder` and `cargo test -p finder` passed; diff 51,667 bytes. US1-S2 as
  drafted failed with a correct recorder, because the test's `serde_json` parse came
  back one ULP off; it now compares after a round trip. The plan gained `Recorder::root`,
  `FolderError`'s message, the `biased` select, the single lock that orders a stop after
  every write, and the preview written before its line.
- **US2** (branch `probe/025-us2`, on US1): six tests, all passing; `scope-link`,
  `scope-node`, `finder` and `simulator` tests (6, 53, 395 and 48) and their `clippy`
  passed; diff 46,881 bytes. Every interface compiled as written. US2-S3 needed the same
  round trip. Removing the 16 MiB limit made US2-S6 fail, so it is a real check. The
  timeout, a short reply and the console without a recorder had no scenario and now do.
- **US3** (branch `probe/025-us3`, on US2): ten Vitest tests and every existing one
  passed, with typecheck, `format:check`, the build and a clean `dist`; diff 16,699
  bytes. The probe found that `useSession` drops `recording`, so the real page would
  never show one; US3 now writes it and US3-S5 checks it.
- **Review**: the drafted stop could let a line follow it on a multi-thread runtime; the
  shared setup pointed the camera below the horizon; no gate sees the section at night
  (spec owner decision 11); raw replies share the heartbeat's connection (spec owner
  decision 10); `020-live-chart-calm` writes three of US3's files.
- **Not run:** `CI=1 npm run e2e` and the workspace-wide gates. The `test` gate runs both.

The trio was then rebased onto `main` at 2508423, whose only code change since c1ed295
is `crates/finder/src/solve.rs`; its one cited line was re-read. Writes come from the
task text and the probes. Shared writes name the merge edge that orders them, and the
stories that last wrote the same file on `main`, or will once `020-live-chart-calm`
lands. Nothing in this record is an owner approval: those are in spec.md, and flipping
to `ready` gives them.

```yaml
version: 1
reviewed_revision: c1ed295734253c7d6121052b0a14beb2ee64bc17
stories:
  US1:
    tests:
      - path: crates/finder/tests/recording.rs
        status: new
        disposition: add
        assertion: "a start and stop write recording.json with the format, version, site, settings, camera, UTC name, stop time and counts, frames.jsonl and solves.jsonl whose counts match the session, every solve naming a recorded frame, nothing after the stop, and no address or token"
        decision: "spec.md:US1-S1"
      - path: crates/finder/tests/recording.rs
        status: new
        disposition: add
        assertion: "each frame line equals the star frame the test saw after a serde_json round trip, centroids as triples in order, previews byte for byte, and each solve line is serde_json::to_string of the live report"
        decision: "spec.md:US1-S2"
      - path: crates/finder/tests/recording.rs
        status: new
        disposition: add
        assertion: "a recorded frame fed to a new Live solves within 0.01 deg of its recorded solve and within 0.05 s of its time"
        decision: "spec.md:US1-S3"
      - path: crates/finder/tests/recording.rs
        status: new
        disposition: add
        assertion: "bad bodies answer 400, a foreign origin 403, no recordings folder or no live loop 409, and an unmakeable folder 500, each leaving no recording"
        decision: "spec.md:US1-S4"
      - path: crates/finder/tests/recording.rs
        status: new
        disposition: add
        assertion: "a repeated start keeps its folder and a start after a stop makes a new one"
        decision: "spec.md:US1-S5"
      - path: crates/finder/tests/recording.rs
        status: new
        disposition: add
        assertion: "a limit of three ends the recording as limit after three frame lines, and a deleted folder ends it as write_failed without making the folder again"
        decision: "spec.md:US1-S6"
      - path: crates/finder/tests/recording.rs
        status: new
        disposition: add
        assertion: "the binary under --simulate --state S records into S/recordings/<folder>/recording.json"
        decision: "spec.md:US1-S7"
      - path: crates/finder/tests/live.rs
        status: existing
        disposition: preserve
        assertion: "live solving and its reports are unchanged with a recorder subscribed"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/session_api.rs
        status: existing
        disposition: preserve
        assertion: "GET /api/session bodies without a recording are unchanged"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/simulate_cli.rs
        status: existing
        disposition: preserve
        assertion: "the binary's refusals and address line are unchanged"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "live solving, the link and GET /api/live behave as without a recording (spec.md FR-004, plan.md Shapes that existing tests pin)"
      - "GET /api/session without a recording is byte for byte unchanged"
      - "nothing about a recording is persisted in session.json"
    checks:
      development:
        - "cargo test -p finder --test recording --locked"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs, and a first recording at the telescope"
    writes:
      - GLOSSARY.md
      - crates/finder/README.md
      - crates/finder/src/api.rs
      - crates/finder/src/lib.rs
      - crates/finder/src/main.rs
      - crates/finder/src/recording.rs
      - crates/finder/tests/recording.rs
    no_write_reason: ""
    shared_writes:
      - path: GLOSSARY.md
        other_spec: 025-frame-capture
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and adds Raw frame after Recording"
      - path: crates/finder/README.md
        other_spec: 025-frame-capture
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and adds its route to US1's table and section"
      - path: crates/finder/src/api.rs
        other_spec: 025-frame-capture
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and adds the raw route"
      - path: crates/finder/src/recording.rs
        other_spec: 025-frame-capture
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and adds raw frames to the recorder"
      - path: crates/finder/src/lib.rs
        other_spec: 025-frame-capture
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and adds the raw route's line after US1's"
      - path: crates/finder/README.md
        other_spec: 017-console-stop-confirm
        other_story: US4
        disposition: merge_order
        reason: "025 depends_on_landed 017, whose US4 last changed the README"
      - path: crates/finder/src/api.rs
        other_spec: 017-console-stop-confirm
        other_story: US1
        disposition: merge_order
        reason: "025 depends_on_landed 017, whose US1 last changed api.rs"
      - path: crates/finder/src/lib.rs
        other_spec: 017-console-stop-confirm
        other_story: US1
        disposition: merge_order
        reason: "025 depends_on_landed 017, whose US1 last changed lib.rs"
      - path: crates/finder/src/main.rs
        other_spec: 016-mcp
        other_story: US1
        disposition: merge_order
        reason: "025 depends_on_landed 016, whose US1 last changed main.rs"
    open_questions: []
  US2:
    tests:
      - path: crates/scope-node/tests/raw_frame.rs
        status: new
        disposition: add
        assertion: "RawFrame answers width x height pixels of a streamed frame's id and size, and cedar's detector on them gives exactly that frame's centroids"
        decision: "spec.md:US2-S1"
      - path: crates/scope-node/tests/raw_frame.rs
        status: new
        disposition: add
        assertion: "RawFrame needs the token, and two calls waiting while frames are stopped are both answered with the same id"
        decision: "spec.md:US2-S2"
      - path: crates/finder/tests/recording_raw.rs
        status: new
        disposition: add
        assertion: "two requested raw frames are saved as gray PNGs of their frame lines' size whose detected centroids equal the lines' after a serde_json round trip, with no .part file left"
        decision: "spec.md:US2-S3"
      - path: crates/finder/tests/recording_raw.rs
        status: new
        disposition: add
        assertion: "the raw route refuses without a recorder, without a recording, with bad bodies, a foreign origin, a running request or no node, a node sending no frame ends the request after 15 s, and a stop ends a request"
        decision: "spec.md:US2-S4"
      - path: crates/finder/tests/recording_raw.rs
        status: new
        disposition: add
        assertion: "a node without ScopeNodeRaw gives the error this scope node cannot send raw frames, a short reply gives the raw frame request failed, and neither leaves a file"
        decision: "spec.md:US2-S5"
      - path: crates/finder/tests/recording_raw.rs
        status: new
        disposition: add
        assertion: "a 2592 x 1944 raw frame arrives through the 16 MiB limit and decodes to its uniform pixels"
        decision: "spec.md:US2-S6"
      - path: crates/scope-link/tests/link.rs
        status: existing
        disposition: preserve
        assertion: "its hand-written ScopeNode implementation compiles and behaves unchanged"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/aim_cli.rs
        status: existing
        disposition: preserve
        assertion: "its hand-written ScopeNode node compiles and the aim tap behaves unchanged"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/link.rs
        status: existing
        disposition: preserve
        assertion: "LinkManager::start, the link's frames and its status are unchanged"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/mount_calls.rs
        status: existing
        disposition: preserve
        assertion: "MountClient built on its own channel, and its token checks, are unchanged"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "no existing scope-link message, field or call changes (spec.md FR-011)"
      - "the star-frame stream, previews and their schedule are unchanged"
      - "an older node keeps working with the new console, and an older console with the new node"
    checks:
      development:
        - "cargo test -p scope-node --test raw_frame --locked"
        - "cargo test -p finder --test recording_raw --locked"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs, and a rebuild of the scope node on the Pi"
    writes:
      - GLOSSARY.md
      - crates/finder/README.md
      - crates/finder/src/api.rs
      - crates/finder/src/lib.rs
      - crates/finder/src/link.rs
      - crates/finder/src/recording.rs
      - crates/finder/tests/recording_raw.rs
      - crates/scope-link/proto/scope_node/v1/scope_node.proto
      - crates/scope-node/src/service.rs
      - crates/scope-node/tests/raw_frame.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/link.rs
        other_spec: 017-console-stop-confirm
        other_story: US1
        disposition: merge_order
        reason: "025 depends_on_landed 017, whose US1 last changed link.rs"
      - path: crates/scope-link/proto/scope_node/v1/scope_node.proto
        other_spec: 017-console-stop-confirm
        other_story: US4
        disposition: merge_order
        reason: "025 depends_on_landed 017, whose US4 last changed the scope link proto"
      - path: crates/scope-node/src/service.rs
        other_spec: 017-console-stop-confirm
        other_story: US4
        disposition: merge_order
        reason: "025 depends_on_landed 017, whose US4 last changed service.rs"
    open_questions: []
  US3:
    tests:
      - path: crates/finder/web/src/components/RecordingSection.test.tsx
        status: new
        disposition: add
        assertion: "Setup shows a Recording region after Aim point with Start recording and no status or alert when a node is paired, posts on and shows a refusal as an alert, and shows no region without a node"
        decision: "spec.md:US3-S1"
      - path: crates/finder/web/src/components/RecordingSection.test.tsx
        status: new
        disposition: add
        assertion: "a running recording reads exactly its folder and counts, Stop recording posts off and Save 3 raw frames posts three, and a disconnected node still offers Stop recording"
        decision: "spec.md:US3-S2"
      - path: crates/finder/web/src/components/RecordingSection.test.tsx
        status: new
        disposition: add
        assertion: "raw progress reads 1 of 3 saved with the button disabled, and a raw error is an alert with the button enabled"
        decision: "spec.md:US3-S3"
      - path: crates/finder/web/src/components/RecordingSection.test.tsx
        status: new
        disposition: add
        assertion: "an ended recording says what was saved, adds the frame limit for limit, and alerts for write_failed"
        decision: "spec.md:US3-S4"
      - path: crates/finder/web/src/components/RecordingSection.test.tsx
        status: new
        disposition: add
        assertion: "SetupPage fed by useSession shows a running recording's status, so the hook keeps recording"
        decision: "spec.md:US3-S5"
      - path: crates/finder/web/src/components/ScopeNodeSection.test.tsx
        status: existing
        disposition: preserve
        assertion: "Setup fed by useSession still reads token_set and never a token"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/src/components/SetupPage.test.tsx
        status: existing
        disposition: preserve
        assertion: "Setup without a node reads one status and its forms work as before"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/src/components/AimTap.test.tsx
        status: existing
        disposition: preserve
        assertion: "Setup with a node reads one alert from the aim tap"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/e2e/session.spec.ts
        status: existing
        disposition: preserve
        assertion: "the Setup page without a node is still red only at night; it never renders the Recording section"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "the page adds no npm package (plan.md Technical Context, Primary Dependencies)"
      - "red-only night mode: the section uses existing tokens and classes; no gate checks it at night (spec.md owner decision 11)"
    checks:
      development:
        - "npm test"
        - "npm run typecheck"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs: finder-page"
    writes:
      - .impeccable/design.json
      - DESIGN.md
      - crates/finder/web/dist
      - crates/finder/web/src/api.ts
      - crates/finder/web/src/components/RecordingSection.test.tsx
      - crates/finder/web/src/components/RecordingSection.tsx
      - crates/finder/web/src/components/SetupPage.tsx
      - crates/finder/web/src/useSession.ts
    no_write_reason: ""
    shared_writes:
      - path: .impeccable/design.json
        other_spec: 017-console-stop-confirm
        other_story: US5
        disposition: merge_order
        reason: "025 depends_on_landed 017, whose US5 last changed design.json"
      - path: DESIGN.md
        other_spec: 020-live-chart-calm
        other_story: US2
        disposition: merge_order
        reason: "025 depends_on_landed 020-live-chart-calm, whose US2 writes DESIGN.md after 017's US5 did"
      - path: crates/finder/web/dist
        other_spec: 020-live-chart-calm
        other_story: US2
        disposition: merge_order
        reason: "025 depends_on_landed 020-live-chart-calm, whose US2 rebuilds the bundle after 017's US5 did"
      - path: crates/finder/web/src/api.ts
        other_spec: 020-live-chart-calm
        other_story: US2
        disposition: merge_order
        reason: "025 depends_on_landed 020-live-chart-calm, whose US2 writes api.ts after 017's US5 did"
      - path: crates/finder/web/src/useSession.ts
        other_spec: 016-mcp
        other_story: US7
        disposition: merge_order
        reason: "025 depends_on_landed 016, whose US7 last changed useSession.ts"
      - path: crates/finder/web/src/components/SetupPage.tsx
        other_spec: 016-mcp
        other_story: US7
        disposition: merge_order
        reason: "025 depends_on_landed 016, whose US7 last changed SetupPage.tsx"
    open_questions: []
```
