# Implementation Plan: Saving Live Frames at the Telescope

**Branch**: `025-frame-capture` | **Date**: 2026-11-14 | **Spec**: [spec.md](spec.md)

This plan records every design decision for the feature. A story that would change one
escalates rather than re-deciding it.

## Summary

Add a `Recorder` to the console. `POST /api/record` starts it in a new folder under the
state folder; it subscribes to the link's newest-frame channel and to the live loop's
reports, and appends each star frame, each live report and each preview to files until
it is stopped, reaches its frame limit, fails a write or sees a frame id that does not
rise, as after a scope node restart. US2 adds a scope-link service that answers one call
with the next star frame's 8-bit pixels, and a route that saves a few of them into the
running recording as PNGs. US3 adds a Recording section to the Setup page.

## Technical Context

**Language/Version**: Rust (MSRV 1.96) for US1 and US2; TypeScript with React 19 for US3.

**Primary Dependencies**: nothing new. `finder` already has `image` with `png`, and
`tokio` with `fs` and `sync`; the scope node already has `tokio` with `sync`.

**Storage**: `recordings/<folder>/` under the console's state folder: `recording.json`,
`frames.jsonl`, `solves.jsonl`, `preview-<id>.jpg` and, with US2, `raw-<id>.png`.
Nothing about a recording is kept in `session.json`: it is run-only.

**Testing**:
- US1: `crates/finder/tests/console/recording.rs` (new), the module `recording` of
  finder's `console` test binary, in-process, and for US1-S8 on a `Recorder` the test
  starts with channels of its own; and `crates/finder/tests/cli/recording.rs` (new), the
  module `recording` of `cli`, for US1-S7 against the built binary.
- US2: `crates/scope-node/tests/raw_frame.rs` (new) for the node;
  `crates/finder/tests/console/recording_raw.rs` (new), the module `recording_raw` of
  `console`, for the console; and `crates/finder/tests/motion/recording_raw.rs` (new),
  the module `recording_raw` of `motion`, for US2-S7, which moves the simulated mount.
- Where a finder test goes is 027's rule, by what it needs to run: `console` calls the
  router in process and moves no mount (`crates/finder/tests/console/main.rs:2`), `cli`
  starts the built `finder` (`crates/finder/tests/cli/main.rs:1`), and `motion` drives
  the simulated mount in paused time (`crates/finder/tests/motion/main.rs:1`). Only the
  tests that need another binary leave `console`: one in each story. Each module is
  declared by one `mod` line, in alphabetical order, in its binary's `main.rs`, which
  otherwise holds only the support include (`crates/finder/tests/layout/main.rs:156`).
  It binds the live helpers it uses with `use crate::support::live as support;`
  (`crates/finder/tests/console/live/loop_.rs:3`), never with `#[path]`, and writes the
  rest of its setup itself, only the parts its own tests use:
  `crates/finder/tests/support` takes no new file
  (`crates/finder/tests/layout/main.rs:127`), and an unused helper fails
  `clippy -D warnings`.
- US3: `crates/finder/web/src/components/RecordingSection.test.tsx` (new), Vitest.
- No new Playwright test. No existing one checks the section, and none checks it at
  night: the one night check of the whole Setup page
  (`crates/finder/web/e2e/session.spec.ts:191`) runs on the session project's console,
  which never pairs a node. The simulate project's checks open Setup with the simulated
  node (`crates/finder/web/e2e/simulate.spec.ts:288`,
  `crates/finder/web/e2e/goto.spec.ts:132`), so they render the section with no
  recording, one at night (`crates/finder/web/e2e/mcp.spec.ts:75`), whose screenshot is
  of the MCP region only (`crates/finder/web/e2e/mcp.spec.ts:131`). A check of the
  section would join the simulate project, which means editing `playwright.config.ts`,
  or pair a node that never connects on the default project's console, which its other
  checks share. Night mode for the section rests on its using only existing classes
  (spec owner decision 11, settled 2026-11-15: no night check).
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
| II. Never weaken a gate | No existing test, golden, fixture, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS` or `ergane.yaml` is edited. New tests go in new module files; each adds one `mod` line to its test binary's `main.rs`, which holds no test: owner approval 7, the edit 027's owner approval 7 asks of every later finder test. |
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
├── tests/cli/main.rs                   # US1: mod recording
├── tests/cli/recording.rs              # US1: new, US1-S7 (the built binary)
├── tests/console/main.rs               # US1: mod recording; US2: mod recording_raw
├── tests/console/recording.rs          # US1: new, every scenario but US1-S7
├── tests/console/recording_raw.rs      # US2: new, US2-S3 to US2-S6
├── tests/motion/main.rs                # US2: mod recording_raw
├── tests/motion/recording_raw.rs       # US2: new, US2-S7 (the mount moves)
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

/// A recording as `GET /api/session` reports it. Non-exhaustive, so a test outside the
/// crate reads it field by field and never builds a whole one, and US2 can add `raw`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct RecordingView {
    pub on: bool,
    /// The folder's name under the recordings folder.
    pub folder: String,
    /// Console Unix seconds at the start.
    pub since: f64,
    pub frames: u64,
    pub solves: u64,
    pub previews: u64,
    /// `stopped`, `limit`, `write_failed` or `node_restarted`; absent while on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_reason: Option<&'static str>,
}

/// What a recording reads. The route builds it from the link and the live loop; US1-S8
/// builds one by literal from channels of its own, so no story adds a field.
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

pub struct Recorder { /* root, limit, run: Mutex<Option<Arc<Mutex<Run>>>> */ }

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

The run (its view, folder, `recording.json` fields and the console clock from
`Sources.clock`) sits behind one `std::sync::Mutex` in an `Arc`, shared by the task and
the `Recorder`; every end writes `stopped_at` from that clock, and the task reads its
own clone of it for `received_at`. The task loops on a `biased` `tokio::select!`
with `frames.changed()` first, so a report ready at the same poll as its frame finds the
frame recorded, and `reports.recv()` second:
- **A frame**: a `None`, which the link publishes when the pairing changes
  (`crates/finder/src/link.rs:360`), is skipped and ends nothing. For a star frame, read
  the clock and `status.borrow().offset_s`, then take the lock; if the run is no longer
  on, return. Then, still under the lock and before anything of the frame is written:
  if the run has written a frame line and this frame's id is not above that line's
  (`last_id: Option<u64>` in the run, `None` until the first line), end with
  `node_restarted` and return, writing neither its preview nor its line (spec owner
  approval 6). Otherwise write its preview if it has one, then its frame line; set
  `last_id` to its id and keep the id in a set; count both. At the limit, end with
  `limit`.
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

Ending (stop, limit, a node restart or a failure): set `on` false and `ended_reason`,
and rewrite `recording.json` with the end fields, printing a failure of that last write
and otherwise ignoring it. A deleted folder therefore prints two lines: the failed
write, then the failed rewrite. A node restart writes nothing of the frame that showed
it, so `recording.json`'s counts are those of the lines before it (US1-S8).

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

taken from `session.snapshot(now)` (`crates/finder/src/session.rs:1002`): `site`, `aim`,
`camera_settings` and `model` as `GET /api/session` shows them, and `camera` as its
`node.camera` (`crates/finder/src/session.rs:341`). At the end it gains `stopped_at`,
`ended_reason`, `frames`, `solves` and `previews`.

### The console (`crates/finder/src/api.rs`, US1)

`AppState` (`crates/finder/src/api.rs:99`) gains `recordings: Option<Arc<Recorder>>`, and:

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
(`crates/finder/src/api.rs:436`). Its body is a `#[serde(deny_unknown_fields)]
RecordEdit { on: bool }`, read through `sim_json` (`crates/finder/src/api.rs:549`) as
`set_track` reads `TrackEdit` (`crates/finder/src/api.rs:685`), behind `SessionWrite`.
`on: true`, in this order: no recorder answers 409 `this console has no recordings
folder`; no link or live loop answers 409 `this console has no live link`; otherwise
`start` with `Sources` from `link.frames()`, `link.status()`, `live.subscribe()`, the
state's clock and session, and a `FolderError` answers 500 with its `to_string()`,
`cannot make the recording folder: <io error>`. `on: false` calls `stop` when there is a
recorder. Both then answer `get_session`.

`SessionResponse` (`crates/finder/src/api.rs:631`) gains

```rust
#[serde(skip_serializing_if = "Option::is_none")]
recording: Option<crate::recording::RecordingView>,
```

filled in `session_response` (`crates/finder/src/api.rs:651`) from
`state.recordings.as_ref().and_then(|r| r.view())`. The MCP tools `get_session`
(`crates/finder/src/mcp/tools.rs:348`) and `set_tracking`
(`crates/finder/src/mcp/tools.rs:314`) return `session_response` too, so their bodies
carry `recording` as well (spec FR-008); no MCP code changes.

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
`--simulate`); each file and the frame line's fields; the 3,600-frame limit and the four
`ended_reason`s, `stopped`, `limit`, `write_failed` and `node_restarted`; that a frame id
that does not rise, as after a scope node restart or a pairing with another node, ends
the recording with `node_restarted`, so the observer starts a new one; that `recording`
in the session is run-only; that a recording holds the site, times and solves, never the
node's address or token, so review one before committing it to this public repository;
and that `Live::start` replays frame lines, as
`crates/finder/tests/console/live/loop_.rs:180` feeds frames.

### `GLOSSARY.md` (US1)

After the **Preview** entry (`GLOSSARY.md:131`):

```markdown
**Recording**:
One folder of what the console saw between the observer's start and stop: each star frame's centroids, settings and axis counts, each live solve and each preview, kept for later tests.
_Avoid_: capture, log, session
```

### The scope link (`crates/scope-link/proto/scope_node/v1/scope_node.proto`, US2)

Appended at the end of the file, after `StopAllReply`
(`crates/scope-link/proto/scope_node/v1/scope_node.proto:147`), not beside
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
- In `run` (`crates/scope-node/src/service.rs:229`), inside the same
  `if !unresponsive` block that sends the star frame
  (`crates/scope-node/src/service.rs:285`), right after the send: take every waiting
  sender (`std::mem::take`) and answer each with that frame's `id`, `width`, `height`
  and pixels, through a private helper that clones the pixels for each extra waiter and
  moves them into the last. A frame the node doesn't send answers nobody.
- `impl ScopeNodeRaw for NodeService`: `raw_frame` waits for `controls.ready(false)`,
  as `request_preview` does (`crates/scope-node/src/service.rs:392`), pushes a new
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
#[non_exhaustive]
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
pub enum RawRefusal { NotRecording, NotConnected, Busy, Moving }

/// True while anything moves (spec FR-013); built in api.rs.
pub type Moving = Arc<dyn Fn() -> bool + Send + Sync>;

impl Recorder {
    /// Save `frames` raw frames into the recording on, one after another.
    pub fn request_raw(&self, frames: u8, client: Option<RawClient>, moving: Moving)
        -> Result<RecordingView, RawRefusal>;
}
```

Refusals in this order: no recording on is `NotRecording`; a request whose `saved` is
below `wanted` and has no `error` is `Busy`; `moving()` true is `Moving` (owner decision
10); no client is `NotConnected`. An accepted request sets `raw` to `{wanted, saved: 0}`
and spawns a task whose `AbortHandle` the run keeps under its lock (`raw_task:
Option<AbortHandle>`; US1's view starts with `raw: None`): for each frame, first
`moving()`, which when true ends the request with exactly `the mount started moving`,
with no leading `stopped: ` (spec owner approval 4, settled 2026-11-15), then
`time::timeout(RAW_WAIT, client.raw_frame())`, then on the blocking pool a check
that `pixels` hold exactly `width × height` bytes, since
`image::GrayImage::from_raw(width, height, pixels)` alone accepts a longer buffer (a
mismatch either way is `the raw frame request failed`), saved with `ImageFormat::Png` as
`raw-<id>.png.part` and renamed to `raw-<id>.png`; then, under the lock and only while
the run is on, `saved += 1`. `Unimplemented` sets `this scope node cannot send raw
frames`, a timeout `no raw frame arrived within 15 s`, any other status `the raw frame
request failed`, and a failed write ends the recording with `write_failed` (its
`stopped_at` from the run's clock). Every `error` is set as `saved` is, under the lock
and only while the run is on: an abort takes effect only at the task's next await, so
on a multi-thread runtime a stop can land between a reply or timeout and the lock, and
must still leave no `error` (spec FR-014). Every end of the recording aborts the raw
task and leaves `raw` as it was, with no `error`.

### The raw route (`crates/finder/src/api.rs`, US2)

`.route("/api/record/raw", post(record_raw))` behind `SessionWrite`, with
`#[serde(deny_unknown_fields)] RawEdit { frames: u8 }` read through `sim_json`; 0 or 11
to 255 answers 400 with `frames must be a whole number from 1 to 10`, and any other bad
body (a fraction, a value that is not a `u8`, a missing field) answers 400 as `sim_json`
answers it. Then, in this order: no recorder answers
409 `this console has no recordings folder`; `NotRecording`, `Busy`, `Moving` and `NotConnected`
answer 409 with `start a recording first`, `raw frames are already being saved`,
`wait until the mount is still` and `the scope node is not connected`. Success answers
`get_session`. The `moving` closure holds a clone of the shared state and reads, each
time it is called, the test `running` already makes for the session's motion record
(`crates/finder/src/api.rs:664`): an axis move `moving`, a GoTo `slewing`, `solving` or
`correcting`, or Track `armed`, `tracking` or `keyhole_wait`; or the session's
`mount.moving`. That test moves into a private `fn motion_running(axis_move:
Option<&mcp::guard::Progress>, goto: Option<&goto::Progress>, session: &SessionView) ->
bool` (`crates/finder/src/mcp/guard.rs:14`, `crates/finder/src/goto.rs:494`), which
`session_response` and the closure both call, with no change to the motion record.
The closure is `motion_running(..) || mount.moving` over a fresh snapshot each call. No
scenario isolates the `motion_running` half: every real move also sets `mount.moving`
(US2-S7 drives that one), and a Track `armed` or a GoTo `solving` with a still mount is
hard to hold in a test, so the diff shows the closure calling `motion_running`. The
client comes from `state.link().and_then(|link| link.raw())`.

### `crates/finder/src/lib.rs` (US2)

One line after US1's in the route list, written as it stands here, with no backticks:
`* POST /api/record/raw: {"frames":N}, N from 1 to 10; saves the next N star frames' pixels into the recording on.`

### `crates/finder/README.md` and `GLOSSARY.md` (US2)

A row after US1's:

```markdown
| `POST /api/record/raw` | JSON `{"frames":N}`, N from 1 to 10. While recording, save the next N star frames' 8-bit pixels as `raw-<id>.png`; progress is `recording.raw` in the session. Needs a scope node built with 025. Refused while the mount moves. Same-origin writes only. |
```

and a paragraph in `## Recording a night` on raw frames, their PNGs, `recording.raw`
and its four errors, `this scope node cannot send raw frames`,
`no raw frame arrived within 15 s`, `the raw frame request failed` and
`the mount started moving`, and that raw frames are refused with `wait until the mount
is still` while anything moves, and a running request stops before its next frame with
`the mount started moving` once a move starts, because a raw reply shares the scope
link's connection with the heartbeat: on slow Wi-Fi a transfer can delay the heartbeat
past one second, and the scope node then stops the mount. After **Recording** in
`GLOSSARY.md`:

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
  ended_reason?: "stopped" | "limit" | "write_failed" | "node_restarted";
  raw?: { wanted: number; saved: number; error?: string };
}
```

`recording?: SessionRecording;` in `ObservingSession`
(`crates/finder/web/src/api.ts:234`), and `postRecord(on: boolean)` and
`postRecordRaw(frames: number)`, each written as `postTrack` is
(`crates/finder/web/src/api.ts:300`), posting `{on}` to `/api/record` and `{frames}` to
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
  present and has no `error`, a second line inside that same element,
  `Raw frames: {saved} of {wanted} saved`.
  The buttons `Stop recording` and `Save 3 raw frames` (`button`); the second is
  disabled while `saved` is below `wanted` and there is no `error`. A raw `error` shows
  as `role="alert"`: `Raw frames stopped: {error}.`, the error unchanged, so a request
  stopped by a move reads `Raw frames stopped: the mount started moving.` (spec owner
  approval 4).
- **Ended**: the status `Saved {folder}: {frames} frames, {solves} solves.`, followed
  for `limit` by ` It stopped at its frame limit.` and for `node_restarted` by
  ` It stopped when the scope node restarted. Start a new recording.` (spec owner
  approval 6), with no alert; for `write_failed`, an alert
  `Recording stopped: the console could not write to its folder.`
- **A refused request**: an alert with the answer's `error`, as `TrackControl` shows one.

Each successful post calls `onChange`; a refused one does not. Write each phrase a test
asserts on one line of JSX, so `textContent` keeps it whole. A raw error alert and a
refused request's alert may show together; no scenario needs them apart.

`SetupPage.tsx`: `<RecordingSection session={session} onChange={onChange} />` right
after `<AimTap … />` (`crates/finder/web/src/components/SetupPage.tsx:179`).

### `DESIGN.md` (US3)

After the Aim-point tap entry (`DESIGN.md:347`), add a plain `###` heading and one
paragraph, like its neighbours. The `>` marks below only set the text apart here; the
entry is not a blockquote:

> ### Recording — built
> On the Setup page, after the aim point, while a scope node is paired: a section titled
> Recording with one primary button, Start recording. While recording, one status line
> names the folder and counts, with Stop recording and Save 3 raw frames as secondary
> buttons; raw progress is a second line of the status and a failure is an alert, such as
> Raw frames stopped: the mount started moving. After a recording the status says what
> was saved, and why when it hit its frame limit or the scope node restarted, which asks
> for a new recording. Ink on paper with existing tokens only; at night it is red like
> every other section.

### `.impeccable/design.json` (US3)

Every earlier Setup section added a component here with its `DESIGN.md` entry. Append to
`components`, formatted as its neighbours:

```json
{"name": "Recording", "kind": "custom", "refersTo": "button",
 "description": "Built in RecordingSection.tsx on Setup after Aim point, whenever the session has a scope node, connected or not. Start recording is the primary button and POSTs /api/record on. While recording, one status names the folder, frames and solves, with Stop recording and Save 3 raw frames as secondary buttons; raw progress joins the status and a raw error is an alert. After a recording the status says what was saved, and why when it hit its frame limit or the scope node restarted; a failed write is an alert. Existing ink, paper and red-only night tokens; no new class.",
 "html": "<section class=\"ds-recording\"><h2>Recording</h2><p role=\"status\">Recording to 2026-01-16T02-00-00Z · 12 frames · 9 solves</p><button>Stop recording</button><button>Save 3 raw frames</button></section>",
 "css": ".ds-recording{color:var(--color-ink)}.ds-recording button{border:1px solid var(--color-ink);background:transparent;color:inherit;min-height:44px}"}
```

## Shapes that existing tests pin

These are not changed by any story:
- **The `ScopeNode` service and every existing message.** Two hand-written
  implementations of the trait (`crates/finder/tests/cli/aim.rs:22`,
  `crates/scope-link/tests/link.rs:55`) compile unchanged because the new call is in a
  new service. `StarFrame` gains no field: a test builds it as a literal naming every
  field (`crates/scope-link/tests/link.rs:25`).
- **`LinkManager::start`'s signature and `MountClient`.** Many tests reach the link
  through `with_link`; `crates/finder/tests/console/link.rs:187` and
  `crates/finder/tests/motion/tracking/keyhole.rs:228` call `LinkManager::start`
  directly, and `crates/finder/tests/motion/mount_calls.rs:54` builds `MountClient`
  itself.
- **`GET /api/session`, and the MCP session bodies, without a recording.** `recording`
  is skipped when absent, so every existing body, including what the MCP tools
  `get_session` and `set_tracking` return, is byte for byte what it was.
- **US1's own test, for US2.** `crates/finder/tests/console/recording.rs` builds
  `Sources` by literal and calls `Recorder::new`, `start` and `view` (US1-S8), so US2
  adds no field to `Sources` and changes none of those signatures. `RecordingView` and
  `RawView` are `#[non_exhaustive]`, so that test reads a view field by field, never
  builds or compares a whole one, and keeps compiling when US2 adds `raw`.
- **`GET /api/live`.** The recorder is one more subscriber of the same broadcast.
- **The binary's start-up lines.** `crates/finder/tests/cli/simulate.rs` reads the
  address line `finder console: http://`; nothing new is printed.
- **Finder's test layout.** `crates/finder/tests/layout/main.rs` fails a file at the top
  level of `crates/finder/tests` (`crates/finder/tests/layout/main.rs:109`), any file
  added to `crates/finder/tests/support` (`crates/finder/tests/layout/main.rs:127`) and
  a binary's `main.rs` holding anything but its support include and `mod` lines
  (`crates/finder/tests/layout/main.rs:156`). Each new module is a file in its binary's
  folder and one `mod` line in that `main.rs`; no existing module, helper or test
  changes.
- **The Setup page with no `node`, or with a `node` and no `recording`.** Existing tests
  read the page's only `status` (`crates/finder/web/src/components/SetupPage.test.tsx:37`)
  and only `alert` (`crates/finder/web/src/components/AimTap.test.tsx:102`), and in a
  real browser the simulate console's aim-tap check reads Setup's only `status`
  page-wide with the node paired (`crates/finder/web/e2e/simulate.spec.ts:318`); the
  new section adds neither then, and no button whose name another test uses.
- **`useSession`'s other fields.** The one new line copies `recording` only; a test that
  drives the page through the hook
  (`crates/finder/web/src/components/ScopeNodeSection.test.tsx:70`) reads the same
  session as before.

## Decisions and the alternatives rejected

| Decision | Rejected | Why |
|---|---|---|
| Record from the link's frame channel and the live broadcast | A writer inside `active` in `link.rs` | The channel already feeds two readers; a writer in the link loop shares a task with the heartbeat, whose 1 s silence deadline a slow write could trip. A missed frame shows as a gap in the ids. |
| Writes, counts and ends under one lock; `stop` ends before it aborts | Abort the task, then rewrite `recording.json` | An abort does not wait: on a multi-thread runtime the task can finish a line and count it after the rewrite, so the counts and the files disagree. |
| Raw replies on the paired connection, refused while anything moves | A second connection; allowing them during moves | Spec owner decision 10 (the owner, 2026-11-14): no new transport, and no raw reply can starve the heartbeat and stop the mount mid-move. One frame already on its way when a move starts still finishes. |
| Tests compare numbers after a `serde_json` round trip | A tolerance; the `float_roundtrip` feature | A tolerance weakens "exactly", and the feature is a `Cargo.toml` change. The files hold the exact shortest form. |
| Record on the console | Save `GET /api/live` with curl | Its events lack brightness, settings, axis counts, the offset and previews. |
| JSON Lines, one synchronous write per line, the file reopened each time | One JSON document | A console that dies mid-night would leave nothing readable, and a deleted folder would go unnoticed behind an open handle. |
| A solve is written only for a recorded frame | Every report | Each solve line then has its frame line. |
| A frame whose id is not above the last line's ends the recording as `node_restarted`, checked under the lock before its preview | Recording on with repeated ids; `create_new` names ending in `write_failed`; names from a count of the recording's own | Spec owner approval 6 (the owner, 2026-11-15): a repeated id would replace a preview and match a solve line to two frame lines; the other names change owner decision 2 or misname the cause. |
| The move error is `the mount started moving` | `stopped: the mount started moving`; the page dropping a leading `stopped: ` | Spec owner approval 4 (the owner, 2026-11-15): the page's `Raw frames stopped: {error}.` then reads once, and the page shows every raw error unchanged. |
| Raw pixels through a new service | A new call on `ScopeNode` | Two existing tests implement that trait and would stop compiling; tonic's default stubs would need a `build.rs` change, a critical path. |
| Raw pixels by a unary call | A raw field on `StarFrame` | Every reader of the frame channel would carry up to 5 MB, the newest-frame channel may drop the very frame asked for, and a test's full literal (`crates/scope-link/tests/link.rs:25`) would stop compiling. |
| Raw frames saved by the console | Saved on the scope node's card | One folder, matched by id, and no SSH at the telescope. |
| 8-bit gray PNG | PGM, FITS | Lossless, about half the size on a night frame, read by `image::open` as the existing fixture is; `finder` already has the `png` feature. |
| A 16 MiB limit on the raw client only | Raising it for the whole link | Only raw replies are large. |
| `pair` returns a named struct | A five-element tuple | Easier to read, and keeps `clippy::type_complexity` away. |
| Folder in the state folder | A `--recordings` flag | `--state` and `--simulate` already keep nights apart. |
| A Setup page section | A top-bar button | The top bar holds motion and Stop; recording is set-up work and must not crowd Stop at night. |

## Verification (every story, before proposing)

- **The factory gates** (`ergane.yaml` at e62ffc2, after pull request 214):
  - `lint` = `cargo fmt --all --check`,
    `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`,
    `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features --locked`
    and `RUSTDOCFLAGS="-D warnings" cargo doc -p finder --no-deps`;
  - `test` = `cargo nextest run --workspace --all-targets --all-features --locked`,
    `cargo test --workspace --locked --doc`, and then, in `crates/finder/web`,
    `npm ci --no-audit --no-fund`, `npm run typecheck`, `npm run format:check`,
    `npm test`, `npm run build`, `git status --porcelain -- dist | (! grep .)` and
    `CI=1 npm run e2e`.

  Every story runs both, and the judge sees their results. nextest runs every test as
  its own process, as many at once as the host has CPUs, reports every failure and ends
  a test after 5 minutes (`.config/nextest.toml`), on the root `Cargo.toml`'s test
  profile at opt-level 1. The stories' development checks run one finder module with
  `cargo nextest run -p finder --locked --test <binary> <module>::`, and the scope
  node's new test file with `cargo test`, which builds the same test profile.
- **CI in the merge queue** runs every `ci.yml` job, including the finder-page
  build-freshness check and the build on Rust 1.96. Coverage runs only by hand
  (constitution 1.1.0).

## Prerequisites before this spec is flipped to `ready`

1. **The earlier specs have landed:** `013-live-finder`, `014-goto`, `016-mcp`,
   `017-console-stop-confirm`, `019-night-legibility` and `020-live-chart-calm`. Met:
   020-live-chart-calm completed at f8dab49 (2026-11-14, US2 #212). That US2 wrote
   `DESIGN.md`, `crates/finder/web/dist` and `crates/finder/web/src/api.ts`, as US3
   does.
2. **`main` is still as read.** Every citation here was read at c1ed295 and re-checked at
   f8dab49, after 020-live-chart-calm landed: its US2 moved the cited lines of
   `DESIGN.md` and `crates/finder/web/src/api.ts` down by four, now `DESIGN.md:347`,
   `crates/finder/web/src/api.ts:234` and `crates/finder/web/src/api.ts:300`. `main`
   then moved to 1104b7d, and no cited line moved: 870b332 changes only CI workflow
   files and the page's Playwright version; e62ffc2 (pull request 214) runs the `test`
   gate's Rust half with `cargo nextest run` on an optimized test profile, so
   § Verification was copied again from its `ergane.yaml`; and 1104b7d adds the ready
   spec 027. `main` then moved to b8dceaa, merged in: 6c0e640 and b8dceaa land 027
   (Prerequisite 7), which moved every cited test and helper under `crates/finder/tests`,
   and each of those citations was re-read at b8dceaa. 027 changed nothing else this
   trio cites. `main` is now at f0ea2d5, merged in: 1c977a3 and 06a7d4e change only
   spec files, and de9bfe6, fc7d2bf, 2083915 and f0ea2d5 land 024 (Prerequisite 6),
   which moved cited lines of `crates/finder/src` and the scope node, each re-read at
   f0ea2d5. If `main` moves again in a cited file, re-read it and re-run the
   validator.
3. **The owner decisions are settled and the approvals given** (spec § Owner decisions,
   § Owner approvals). Met for the decisions: the owner settled decisions 1 to 10 on
   2026-11-14 and decision 11 and approvals 4 and 6 on 2026-11-15, and those answers are
   written here. Flipping to `ready` gives the approvals. Any other answer is written
   into the trio and re-validated before the flip, never left to a story.
4. **No operator-staged files are needed.**
5. **Outside the factory, after US2 lands:** rebuild the console and the scope node from
   that `main` before raw frames are asked for at the telescope.
6. **`024-mount-link-recovery` has landed, and this trio is re-anchored to it.** Met:
   the owner settled on 2026-11-15 that 024 flips first, and 024 completed at f0ea2d5
   (2026-11-15, US4 #226) after US1 at de9bfe6 (#222), US2 at fc7d2bf (#224) and US3 at
   2083915 (#225); `main` at f0ea2d5 was merged in. Every citation of a file 024 changed
   was re-read at f0ea2d5. Its US3 moved the cited lines of `crates/finder/src/api.rs`
   down by six to eight (`AppState` to :99, the `/api/track` route to :436,
   `live_stream` to :465, `sim_json` to :549, `SessionResponse` to :631,
   `session_response` to :651, the motion test to :664 and `set_track` to :685),
   `crates/finder/src/goto.rs:494` (`Progress`) by one, and
   `crates/finder/src/session.rs` by 23 to 54 (:652, :691, :1002 and :1234), and
   rewrote the mount poll in `crates/finder/src/live.rs`, which now hands `set_mount` a
   `MountReading` (:327). Its US2 and US3 added three fields to the scope-link proto,
   so `StopAllReply`, still its last message, is at :147. Its US1 moved
   `crates/scope-node/src/capture.rs:84` by three and the cited lines of `run` and
   `request_preview` in `crates/scope-node/src/service.rs` by six (:229, :285 and
   :392). Every other cited line of those files is where it was and means what this
   trio says. 024 changed no `Commands` field, `routes`, `AppState` field,
   `SessionResponse`, `StarFrame`, `Live`, `LinkManager` or `MountClient`, and no
   running state the motion test reads; `mount.moving` is still either axis moving,
   false when the poll fails; and `with_link` now also hands the console clock to the
   mount poll, which the shared setup already sets before it. US4 changed only
   `crates/scope-node/README.md`, `crates/scope-node/src/mount/skywatcher/mount_link.rs`,
   `crates/scope-node/src/mount/skywatcher/protocol.rs` and
   `crates/scope-node/tests/mount_garble.rs`, none of which this trio cites or writes.
   So no story's design changes. 024 is in `depends_on_landed`; the shared writes of the
   four files both specs write, `crates/finder/src/api.rs` (US1), the scope-link proto,
   `crates/scope-node/src/service.rs` and `crates/finder/tests/motion/main.rs` (US2),
   name 024's US3 as each file's last writer (in `motion/main.rs` it added
   `mod mount_identity;` before `mod night;`, after which US2's line still goes);
   spec.md owner approval 7 names the `mod` lines; and the validator was re-run.
7. **`027-finder-tests-by-system` has landed, and this trio is refreshed against its
   layout.** Met: 027 completed at b8dceaa (2026-11-15, US2 #218), and `main` at b8dceaa
   was merged in. Finder's integration tests are now the binaries `cli`, `console`,
   `layout`, `mcp` and `motion`, the helpers this trio cites are
   `crates/finder/tests/support/live.rs` and `crates/finder/tests/support/mcp.rs`, and
   the `layout` test fails any new top-level file in `crates/finder/tests`. So US1's
   tests are `crates/finder/tests/console/recording.rs` and, for US1-S7,
   `crates/finder/tests/cli/recording.rs`; US2's finder tests are
   `crates/finder/tests/console/recording_raw.rs` and, for US2-S7,
   `crates/finder/tests/motion/recording_raw.rs` (§ Technical Context). Each story's
   writes gain the `main.rs` of every binary it adds a module to, with a shared write
   naming 027's US2 as that file's last writer, and US1's shared writes name US2 for
   `crates/finder/tests/console/main.rs`. The Independent Tests and development checks
   name the binary and module, every citation of a moved test was re-read at b8dceaa,
   the Refinement Record's existing test paths follow the moves, and 027 is in
   `depends_on_landed`. US3 and `crates/scope-node/tests/raw_frame.rs` are unaffected.

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
   and decode PNGs with `image`. No test reads a recording's files through `Recorder`;
   US1-S8 drives one directly and reads only its `view()`, the counts and end that
   `GET /api/session` would show. Numbers
   read back from a file are compared after the round trip of spec § User Scenarios,
   exactly: the workspace's `serde_json` parses about one `f64` in eight back one ULP
   off. The only tolerances are US1-S3's 0.01° and 0.05 s, which bound a fresh solve of
   a replayed frame.
7. **A new call on `ScopeNode` breaks two tests.** *Check:* the call is in the new
   `ScopeNodeRaw` service, and `ScopeNode`, `StarFrame` and every existing message keep
   their calls and fields.
8. **tonic's 4 MiB default.** A 1456 × 1088 test frame passes without the raised limit.
   *Check:* US2-S6 sends a 2592 × 1944 frame.
9. **The newest-frame channel.** A recorder that writes the frame already on the channel
   at its start, or a solve of an earlier frame, gives solve lines without frame lines.
   *Check:* US1-S1 asserts that the first frame line's id is above that of the frame on
   the channel before the start; a run on the round-2 probe branch saw it fail with the
   start's `borrow_and_update` removed. Its check that every solve line names a frame line is a
   guard only: in the shared setup no earlier frame's solve is still running at the
   start.
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
16. **A restarted node's ids.** A check made after the preview is written, or against
    the frame on the channel rather than the last line written, replaces an earlier
    preview or misses an equal id. *Check:* US1-S8 sends a lower id carrying a preview
    of its own and asserts the earlier `preview-5.jpg` kept, then an equal id, and
    asserts `recording.json`'s counts and that nothing follows the end.
17. **027's layout.** A test file at the top level of `crates/finder/tests`, or a file
    added to `crates/finder/tests/support`, fails 027's `layout` test, and a helper a
    module does not use fails `clippy -D warnings`. *Check:* each new test is in the
    module § Technical Context names, declared by one alphabetical `mod` line in its
    binary's `main.rs`, binding the live helpers with
    `use crate::support::live as support;` and holding only the setup its own tests use.

## Risks

| Risk | Mitigation |
|---|---|
| A forgotten recording fills the disk. | The 3,600-frame limit, and the Setup page shows a running recording and its counts. |
| A recording with the site is published. | Owner decision 9, and the README's note; no address or token is ever written. |
| Raw requests slow the scope node. | One pixel clone per waiting call; at most ten frames a request, one request at a time. |
| A recorded frame is missed. | Gaps show in the ids; the cadence of 200 ms or more leaves the writer idle most of the time. |
| An older scope node. | It answers `Unimplemented`, which the page and session report as `this scope node cannot send raw frames`. |
| A raw reply on slow Wi-Fi delays the heartbeat past 1 s, the link drops and the node's watchdog stops the mount mid-GoTo, Track or axis move. | Spec owner decision 10: refused with 409 while anything moves (FR-013), and a running request stops before its next frame once a move starts (FR-014). Only a frame already on its way can overlap a move; the failure stops the mount, the safe direction. |
| A node restart or a re-pairing mid-recording repeats frame ids, and a repeated id's preview or raw PNG replaces the earlier file. | Spec owner approval 6 (settled 2026-11-15): the first frame whose id is not above the last line's ends the recording as `node_restarted` and writes nothing of it (US1-S8), and the page asks for a new recording (US3-S6). A raw request running then ends at its failed call or with the recording. Left over: a restarted node, or another node, whose first frame to arrive is already above the last id goes unnoticed; its frames join the recording, `recording.json`'s `camera` still names the first node's camera, and the lines' `offset_s` tell the nodes apart, though the ids still rise. |
| `020-live-chart-calm` writes `DESIGN.md`, `dist` and `api.ts`, as US3 does. | Landed at f8dab49; its three moved citations were re-read (Prerequisite 2). |
| `024-mount-link-recovery` writes `crates/finder/src/api.rs` (its US3; 025's US1 and US2) and, as 025's US2 does, the scope-link proto (its US2 and US3), `crates/scope-node/src/service.rs` (its US1 to US3) and `crates/finder/tests/motion/main.rs` (its US3). It also changes the mount poll that sets the session's `mount.moving` (`crates/finder/src/live.rs:327`), which FR-013 reads, and `with_link`, which every 025 test builds on. | Settled by the owner on 2026-11-15: 024 flips first, and 025 flips only after 024 has landed. Landed at f0ea2d5; 025 is re-read against its code, declares 024 in `depends_on_landed` and its shared writes, and is re-validated (Prerequisite 6), as 024's plan asks of whichever flips second (its Prerequisite 5). |
| Draft `026-mount-sky` (pull request 207) writes `link.rs` (its frame arm, and a filter on `LinkManager`), `api.rs`, `lib.rs`, `main.rs`, `crates/finder/README.md`, `GLOSSARY.md`, `DESIGN.md`, `dist` and `api.ts`: files of US1, US2 and US3. | Whichever is flipped second is re-read, declares the other in `depends_on_landed` and its shared writes, and is re-validated. |
| `027-finder-tests-by-system` moves the finder tests and helpers this trio cites, and its `layout` test fails a new top-level test file. Each new module adds a `mod` line to a binary's `main.rs`, which other stories edit too, and two stories adding neighbouring lines to one file do not merge cleanly. | Landed at b8dceaa; this trio is refreshed against its layout and re-validated (Prerequisite 7). US2's `mod recording_raw;` follows US1's `mod recording;` in `crates/finder/tests/console/main.rs`, which US2's `depends_on_merged` on US1 orders and US1's shared writes declare. 024 landed first, and its US3 added `mod mount_identity;` to `crates/finder/tests/motion/main.rs`, where US2's `mod recording_raw;` still goes after `mod night;` and US2's shared writes name 024's US3 (Prerequisite 6). |
| A story's diff nears 64 KiB. | The round-3 probes, built from this trio with the owner's answers and then given this round's clauses, measured US1 at 57,433 bytes (8.1 KB under), US2 at 54,397 and US3 at 20,580 (round 2: 49,579, 49,638 and 19,151). Most of US1's growth is US1-S8's test. Re-homed into 027's layout (§ Refinement Record), US1 measures about 59,200 bytes and US2 about 61,400, both under 64 KiB: each module binds `crates/finder/tests/support/live.rs` with `use crate::support::live as support;` and writes the rest of its setup, so US2's `console` module repeats US1's setup, about 6 KB, and its `motion` module the part US2-S7 uses, about 6 KB more. |

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

The trio was then rebased onto `main` at 2508423, whose only source change since c1ed295
is `crates/finder/src/solve.rs` (with a new test, now
`crates/finder/tests/console/star_detected.rs`); the one cited line of `solve.rs` was
re-read. After 020-live-chart-calm landed, `main` at
f8dab49 was merged in and the three citations its US2 moved (`DESIGN.md` and two in
`crates/finder/web/src/api.ts`) were re-read. The owner then settled the owner decisions,
and decision 10's answer added the raw route's motion test, US2-S7 and FR-013's and
FR-014's refusals, none of it compiled yet.

**Round 2.** On 2026-11-14 the three stories were probe-built again from scratch, from
the trio as it then stood on `main` at f8dab49, in throwaway worktrees (branches
`probe/025-r2-us1`, `probe/025-r2-us2` on it, `probe/025-r2-us3` on that), tests first,
with no existing test edited and no undeclared write, and the trio was reviewed again for
anchors and scenarios:

- **US1**: eight tests passing, stable over six runs, and `cargo test -p finder` (396),
  `clippy -p finder`, `fmt` and `cargo doc -p finder` with and without
  `--all-features` clean; diff 49,579 bytes. Every interface compiled as written; the
  `Recorder`'s run is an `Arc` its task shares, and keeps the console clock. Two of
  US1-S1's readings are now explicit, and its check of plan trap 9 now bites (a clause
  run on the probe branch, which failed with the start's `borrow_and_update` removed).
- **US2**: seven tests passing (two in `scope-node`, five in `finder`), and the
  `scope-link`, `scope-node`, `simulator` and `finder` tests (6, 55, 48 and 401) with
  their `clippy`, `fmt` and docs clean; diff 49,638 bytes. Every interface compiled as
  written, `motion_running` included. Mutations showed the 16 MiB limit (US2-S6), the
  per-frame motion check and `mount.moving` (US2-S7) are real checks; the plan now says
  no scenario isolates `motion_running`, and that a reply's length is checked before
  `GrayImage::from_raw`, which accepts a longer buffer. US2-S4's new `recording.raw`
  clause was run on the probe branch and passed.
- **US3**: seven new Vitest tests passing, with all 282 of the page's, typecheck,
  `format:check`, the build and a clean `dist`; diff 19,151 bytes, `.impeccable/design.json` written this
  time. Every interface compiled as written.
- **Review**: the tolerance rule now names US1-S3's bounds as the only ones; the README
  paragraph and the slow-Wi-Fi risk follow decision 10; the simulate project's Setup
  checks render the section, one at night, and `crates/finder/web/e2e/simulate.spec.ts`
  pins its one status; drafts 024 and 026 share files; and two owner questions are open
  (spec owner approvals 4 and 6).
- **Not run:** `CI=1 npm run e2e` and the workspace-wide gates. The `test` gate runs both.

**The owner's answers (2026-11-15).** The owner settled the round-2 questions, and the
answers were written in before any further probe build:
- **Owner approval 4**, drop `stopped: ` from the error: FR-014's move error is exactly
  `the mount started moving` (US2-S7, § Raw frames in the recorder, the README
  paragraph), and US3-S3 gains the alert `Raw frames stopped: the mount started
  moving.`
- **Owner approval 6**, end the recording: FR-007's new `ended_reason`
  `node_restarted`, checked in the recorder's frame arm under the run's lock before the
  frame's preview or line is written; the README's four reasons; the page's words
  (§ The page, FR-016); new scenarios US1-S8, on a `Recorder` the test starts with
  channels of its own, and US3-S6; the Edge Cases entry replaced; plan trap 16.
- **Owner decision 11**, keep no night check, on round 2's corrected basis.
- **The order with 024**: 024 flips first, and this spec after 024 lands, re-anchored to
  its code (Prerequisite 6). No `depends_on_landed` or shared write names 024 yet.

US1-S8 rests on `Recorder::new`, `Recorder::start`, `Sources`' public fields,
`LinkStatus`'s `Default` (`crates/finder/src/link.rs:165`), `FixedClock`
(`crates/finder/src/session.rs:41`) and `Session::default()`
(`crates/finder/src/session.rs:691`), each read for this change. The writes are
unchanged. Round 3 compiled every one of these clauses.

**Round 3.** On 2026-11-14 the three stories were probe-built again from scratch, from
the trio with the owner's answers, in throwaway worktrees (branches `probe/025-r3-us1`,
`probe/025-r3-us2` on it, `probe/025-r3-us3` on that), tests first, with no existing
test edited and no undeclared write, and the trio was reviewed again for anchors and
scenarios:

- **US1**: eight tests passing, stable over six runs, and `cargo test -p finder` (396),
  `clippy -p finder`, `fmt` and both `cargo doc -p finder` commands clean; diff 56,793
  bytes. Every interface compiled as written. US1-S8 failed with the restart check
  removed, with `<` for `<=`, and with the preview written before the check.
- **US2**: nine tests passing, seven in finder's `recording_raw.rs` and two in the scope
  node's `raw_frame.rs`, and the `scope-link`, `scope-node`, `finder` and `simulator`
  tests (6, 55, 403 and 48) with their `clippy`, `fmt` and docs clean; diff 54,378
  bytes. Every interface compiled as written. With 15 bytes, US2-S5 passed with
  the reply's length check deleted, since `GrayImage::from_raw` refuses a shorter
  buffer, so it now sends 17. The raw task's errors are set, like `saved`, under the
  run's lock and only while the run is on.
- **US3**: eight new Vitest tests passing, stable over ten runs, with all 283 of the
  page's, typecheck, `format:check`, the build, an unchanged CSS bundle and a clean
  `dist`; diff 20,580 bytes. Every interface compiled as written.
- **Under the new gate.** `main` moved to e62ffc2 and 1104b7d during the round. In a
  further throwaway worktree (branch `probe/025-r3-check`) the US1 and US2 probes were
  merged with `main` at 1104b7d and given this round's clauses (`model`, `received_at` and the wait for a model in
  US1-S1, `aim` in US1-S8, 17 bytes in US2-S5, both views `#[non_exhaustive]`). Under
  `cargo nextest run` on the opt-level 1 test profile the 17 new tests passed, the 15
  in `finder` over seven runs, and all 512 tests of `finder`, `scope-node`,
  `scope-link` and `simulator` passed, with the workspace's `clippy`, `fmt` and docs
  clean. Each new clause failed
  under the fault it guards against: `model` or `aim` written null, `received_at`
  written 0, and the length check deleted.
- **Review**: US1's test builds `Sources` and reads `RecordingView`, so both views are
  `#[non_exhaustive]` and § Shapes that existing tests pin says what US2 keeps; the MCP
  session tools carry `recording` (spec FR-008, owner approval 1); FR-001 and FR-013 name
  the 415 for a body that is not JSON; another node with higher ids is named in the edge
  case it falls under; § Verification is copied again from `ergane.yaml` at e62ffc2; and
  027 lands first (Prerequisite 7).
- **Not run:** `CI=1 npm run e2e` and the workspace-wide `cargo nextest run`. The `test`
  gate runs both.

**Re-homed for 027 (2026-11-15).** 027 landed at b8dceaa, and `main` was merged in. The
test-layer audit's refresh script rewrote every path of a moved test or helper, 31 of
them, carried 15 line anchors over by their text, and added 027 to `depends_on_landed`.
By hand:
- the home of each new test, by 027's rule (§ Technical Context): US1-S7, which starts
  the built `finder`, goes to `cli`, and US2-S7, which moves the simulated mount, to
  `motion`; every other new finder test stays in `console`;
- the `mod` lines, the three `main.rs` files in the writes and their shared writes, the
  Independent Tests and the development checks;
- the module header, `use crate::support::live as support;`, which re-anchors
  `crates/finder/tests/console/live/loop_.rs:3` from the `#[path]` line it named before
  027 to the `use` line that replaced it;
- SC-002, the tasks' rules, § Shapes that existing tests pin and plan trap 17 for the
  layout, and the layout guard as a preserved test of US1 and US2;
- the round-3 US2 count above, which had said seven tests.

In a throwaway worktree (branch `probe/025-r4-layout`), the round-3 check branch was
merged with `main` at b8dceaa and its two finder test files re-homed this way, every
test body unchanged. The 7 tests of `console`'s `recording`, the 6 of its
`recording_raw`, the one of `cli`'s `recording`, the one of `motion`'s `recording_raw`
and `layout`'s 3 passed, as did all 365 of finder's tests under `cargo test`, one
process a binary, with `clippy -p finder` over all targets and features and `fmt`
clean. `cargo nextest` was not run on this probe; round 3 ran the same tests under it.
Measured as plan trap 3 says, the re-homed tests add about 1.8 KB to US1's diff and
7.0 KB to US2's (§ Risks).

**Re-anchored to 024 (2026-11-15).** 024 landed at f0ea2d5, and `main` was merged in.
Every citation of a file 024 changed was re-read there, and the moved ones follow it
(Prerequisite 6). 024 joined `depends_on_landed`, and the shared writes of
`crates/finder/src/api.rs`, the scope-link proto, `crates/scope-node/src/service.rs` and
`crates/finder/tests/motion/main.rs` name 024's US3, each file's last writer, in place of
the 017 and 027 stories that wrote them before it. No interface, scenario, write or
test changed, and nothing was probe-built: 024 changed no type or signature a story
extends or calls.

Writes come from the task text and the probes. Shared writes name the merge edge that
orders them, and the stories that last wrote the same file on `main`,
`020-live-chart-calm`, `024-mount-link-recovery` and `027-finder-tests-by-system` among
them. Nothing in this record is an owner approval: those are in spec.md, and flipping to
`ready` gives them.

```yaml
version: 1
reviewed_revision: c1ed295734253c7d6121052b0a14beb2ee64bc17
stories:
  US1:
    tests:
      - path: crates/finder/tests/console/recording.rs
        status: new
        disposition: add
        assertion: "a start and stop write recording.json with the format, version, site, model, settings, camera, UTC name, stop time and counts, frames.jsonl and solves.jsonl whose counts match the session, a first frame after the one on the channel at the start, arrival times between the start and the stop, every solve naming a recorded frame, nothing after the stop, and no address or token"
        decision: "spec.md:US1-S1"
      - path: crates/finder/tests/console/recording.rs
        status: new
        disposition: add
        assertion: "each frame line equals the star frame the test saw after a serde_json round trip, centroids as triples in order, previews byte for byte, and each solve line is serde_json::to_string of the live report"
        decision: "spec.md:US1-S2"
      - path: crates/finder/tests/console/recording.rs
        status: new
        disposition: add
        assertion: "a recorded frame fed to a new Live solves within 0.01 deg of its recorded solve and within 0.05 s of its time"
        decision: "spec.md:US1-S3"
      - path: crates/finder/tests/console/recording.rs
        status: new
        disposition: add
        assertion: "bad bodies answer 400, a foreign origin 403, no recordings folder or no live loop 409, and an unmakeable folder 500, each leaving no recording"
        decision: "spec.md:US1-S4"
      - path: crates/finder/tests/console/recording.rs
        status: new
        disposition: add
        assertion: "a repeated start keeps its folder and a start after a stop makes a new one"
        decision: "spec.md:US1-S5"
      - path: crates/finder/tests/console/recording.rs
        status: new
        disposition: add
        assertion: "a limit of three ends the recording as limit after three frame lines, and a deleted folder ends it as write_failed without making the folder again"
        decision: "spec.md:US1-S6"
      - path: crates/finder/tests/cli/recording.rs
        status: new
        disposition: add
        assertion: "the binary under --simulate --state S records into S/recordings/<folder>/recording.json"
        decision: "spec.md:US1-S7"
      - path: crates/finder/tests/console/recording.rs
        status: new
        disposition: add
        assertion: "a frame id that does not rise after None ends a recorder fed by the test as node_restarted, writing nothing of that frame: two frame lines, the first preview's bytes kept, recording.json's aim, counts and end, no line after the end, and an equal id ends a second recording too"
        decision: "spec.md:US1-S8"
      - path: crates/finder/tests/console/live/loop_.rs
        status: existing
        disposition: preserve
        assertion: "live solving and its reports are unchanged with a recorder subscribed"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/mcp/endpoint.rs
        status: existing
        disposition: preserve
        assertion: "get_session over MCP still returns the session body unchanged when no recording exists"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/console/session/edits.rs
        status: existing
        disposition: preserve
        assertion: "GET /api/session bodies without a recording are unchanged"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/cli/simulate.rs
        status: existing
        disposition: preserve
        assertion: "the binary's refusals and address line are unchanged"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and each binary's main.rs holds only its support include and mod lines, with recording added to console's and cli's"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "live solving, the link and GET /api/live behave as without a recording (spec.md FR-004, plan.md Shapes that existing tests pin)"
      - "GET /api/session, and the MCP tools that return the session, without a recording are byte for byte unchanged"
      - "nothing about a recording is persisted in session.json"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console recording::"
        - "cargo nextest run -p finder --locked --test cli recording::"
        - "cargo nextest run -p finder --locked --test layout"
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
      - crates/finder/tests/cli/main.rs
      - crates/finder/tests/cli/recording.rs
      - crates/finder/tests/console/main.rs
      - crates/finder/tests/console/recording.rs
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
      - path: crates/finder/tests/console/main.rs
        other_spec: 025-frame-capture
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and adds mod recording_raw right after US1's mod recording"
      - path: crates/finder/README.md
        other_spec: 017-console-stop-confirm
        other_story: US4
        disposition: merge_order
        reason: "025 depends_on_landed 017, whose US4 last changed the README"
      - path: crates/finder/src/api.rs
        other_spec: 024-mount-link-recovery
        other_story: US3
        disposition: merge_order
        reason: "025 depends_on_landed 024, whose US3 last changed api.rs, after 017's US1 did"
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
      - path: crates/finder/tests/cli/main.rs
        other_spec: 027-finder-tests-by-system
        other_story: US2
        disposition: merge_order
        reason: "025 depends_on_landed 027, whose US2 last changed cli/main.rs"
      - path: crates/finder/tests/console/main.rs
        other_spec: 027-finder-tests-by-system
        other_story: US2
        disposition: merge_order
        reason: "025 depends_on_landed 027, whose US2 last changed console/main.rs"
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
      - path: crates/finder/tests/console/recording_raw.rs
        status: new
        disposition: add
        assertion: "two requested raw frames are saved as gray PNGs of their frame lines' size whose detected centroids equal the lines' after a serde_json round trip, with no .part file left"
        decision: "spec.md:US2-S3"
      - path: crates/finder/tests/console/recording_raw.rs
        status: new
        disposition: add
        assertion: "the raw route refuses without a recorder, without a recording, with bad bodies, a foreign origin, a running request or no node, a node sending no frame ends the request after 15 s, and a stop ends a request with its counts and no error"
        decision: "spec.md:US2-S4"
      - path: crates/finder/tests/console/recording_raw.rs
        status: new
        disposition: add
        assertion: "a node without ScopeNodeRaw gives the error this scope node cannot send raw frames, a reply longer than width x height gives the raw frame request failed, and neither leaves a file"
        decision: "spec.md:US2-S5"
      - path: crates/finder/tests/console/recording_raw.rs
        status: new
        disposition: add
        assertion: "a 2592 x 1944 raw frame arrives through the 16 MiB limit and decodes to its uniform pixels"
        decision: "spec.md:US2-S6"
      - path: crates/finder/tests/motion/recording_raw.rs
        status: new
        disposition: add
        assertion: "raw frames are refused with wait until the mount is still while the mount moves, and a running request stops with exactly the mount started moving once a move starts"
        decision: "spec.md:US2-S7"
      - path: crates/scope-link/tests/link.rs
        status: existing
        disposition: preserve
        assertion: "its hand-written ScopeNode implementation compiles and behaves unchanged"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/cli/aim.rs
        status: existing
        disposition: preserve
        assertion: "its hand-written ScopeNode node compiles and the aim tap behaves unchanged"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/console/link.rs
        status: existing
        disposition: preserve
        assertion: "LinkManager::start, the link's frames and its status are unchanged"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/motion/mount_calls.rs
        status: existing
        disposition: preserve
        assertion: "MountClient built on its own channel, and its token checks, are unchanged"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and each binary's main.rs holds only its support include and mod lines, with recording_raw added to console's and motion's"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "no existing scope-link message, field or call changes (spec.md FR-011)"
      - "the star-frame stream, previews and their schedule are unchanged"
      - "an older node keeps working with the new console, and an older console with the new node"
    checks:
      development:
        - "cargo test -p scope-node --test raw_frame --locked"
        - "cargo nextest run -p finder --locked --test console recording_raw::"
        - "cargo nextest run -p finder --locked --test motion recording_raw::"
        - "cargo nextest run -p finder --locked --test layout"
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
      - crates/finder/tests/console/main.rs
      - crates/finder/tests/console/recording_raw.rs
      - crates/finder/tests/motion/main.rs
      - crates/finder/tests/motion/recording_raw.rs
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
        other_spec: 024-mount-link-recovery
        other_story: US3
        disposition: merge_order
        reason: "025 depends_on_landed 024, whose US3 last changed the scope link proto, after its US2 and 017's US4 did"
      - path: crates/scope-node/src/service.rs
        other_spec: 024-mount-link-recovery
        other_story: US3
        disposition: merge_order
        reason: "025 depends_on_landed 024, whose US3 last changed service.rs, after its US1 and US2 and 017's US4 did"
      - path: crates/finder/tests/motion/main.rs
        other_spec: 024-mount-link-recovery
        other_story: US3
        disposition: merge_order
        reason: "025 depends_on_landed 024, whose US3 last changed motion/main.rs, adding mod mount_identity after 027's US2 made the file"
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
        assertion: "raw progress reads 1 of 3 saved with the button disabled, a raw error is an alert with the button enabled, and a move's error reads Raw frames stopped: the mount started moving."
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
      - path: crates/finder/web/src/components/RecordingSection.test.tsx
        status: new
        disposition: add
        assertion: "a recording ended as node_restarted says what was saved, that the scope node restarted and to start a new recording, offers Start recording and shows no alert"
        decision: "spec.md:US3-S6"
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
      - path: crates/finder/web/e2e/simulate.spec.ts
        status: existing
        disposition: preserve
        assertion: "Setup with the simulated node and no recording still shows exactly one status, the aim tap's"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "the page adds no npm package (plan.md Technical Context, Primary Dependencies)"
      - "red-only night mode: the section uses existing tokens and classes; no gate checks it at night (spec.md owner decision 11)"
    checks:
      development:
        - "npm test"
        - "npm run typecheck"
        - "npm run build"
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
