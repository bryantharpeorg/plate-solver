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
- No Playwright test: the page's existing night and phone checks cover the new section
  through the Setup page, unchanged.

**Target Platform**: the `finder` console, on the Surface (WSL2) or any desk machine;
for US2 also the scope node on the Pi.

**Constraints**:
- No change under `crates/simulator`; outside US2 none under `crates/scope-node` or
  `crates/scope-link`.
- `web/dist` is committed and must be the build of its source (US3).
- Night mode's red-only rule (US3).

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
├── src/lib.rs                          # US1: pub mod recording, the route's doc line
├── src/main.rs                         # US1: recordings/ in the state folder
├── src/link.rs                         # US2: RawClient, LinkManager::raw
├── README.md                           # US1: route and section; US2: raw route
├── tests/recording.rs                  # US1: new
├── tests/recording_raw.rs              # US2: new
└── web/
    ├── src/api.ts                      # US3: SessionRecording, postRecord, postRecordRaw
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

/// The recordings folder, or the new recording's folder, could not be made.
#[derive(Debug)]
pub struct FolderError(pub std::io::Error);

pub struct Recorder { /* root, limit, Mutex<Option<Run>> */ }

impl Recorder {
    pub fn new(root: PathBuf, limit: u64) -> Self;
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
3. Write `recording.json` (below) as `recording.json.part`, then rename it.
4. Spawn the recording task and keep its handle with the view.

The task loops on `tokio::select!` over `frames.changed()` and `reports.recv()`:
- **A frame**: write its frame line, and its preview if it has one; keep its id in a
  set; count it. At the limit, end with `limit`.
- **A report**: if its `frame_id` is in the set, write `serde_json::to_string(&report)`
  as a solve line and count it. `Lagged` is skipped; `Closed` leaves only frames.
- **A failed write** ends the recording with `write_failed` and prints one line to
  standard error.

Every line is written synchronously with `std::fs`: `OpenOptions::new().append(true)
.create(true)` on the file, then one `write_all` of the line and its newline. There is no
await inside a write, so aborting the task can't cut a line, and a deleted folder fails
the next open. A preview is written whole with `std::fs::write`.

Ending (stop, limit or failure): abort or leave the task, set `on` false and
`ended_reason`, and rewrite `recording.json` with the end fields, ignoring a failure of
that last write after printing it.

A frame line, on one line in the file:

```json
{"id":7,"received_at":1768528803.5,"offset_s":-1768527803.2,"start_s":1003.1,"end_s":1003.3,
 "width":1456,"height":1088,"exposure_ms":200.0,"gain":1.0,
 "settings":{"exposure_ms":200.0,"gain":1.0,"detection":{"sigma":8.0,"binning":2,"normalize_rows":true,"detect_hot_pixels":true}},
 "centroids":[[700.5,420.25,1234.0]],
 "axis_start":{"azimuth":12,"altitude":-40},"axis_end":{"azimuth":12,"altitude":-40},
 "preview":"preview-7.jpg"}
```

`received_at` is the console clock when the frame arrived and `offset_s` the link's
`LinkStatus::offset_s` then (`crates/finder/src/link.rs:166`). `settings` is null when
the frame has none, and its `exposure_ms` and `gain` are null when automatic. `preview`
appears only when the frame carried one.

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

The route `.route("/api/record", post(set_record))` sits beside `/api/track`
(`crates/finder/src/api.rs:428`). Its body is a `#[serde(deny_unknown_fields)]
RecordEdit { on: bool }`, read through `sim_json` (`crates/finder/src/api.rs:541`) as
`set_track` reads `TrackEdit` (`crates/finder/src/api.rs:677`), behind `SessionWrite`.
`on: true`, in this order: no recorder answers 409 `this console has no recordings
folder`; no link or live loop answers 409 `this console has no live link`; otherwise
`start` with `Sources` from `link.frames()`, `link.status()`, `live.subscribe()`, the
state's clock and session, and a `FolderError` answers 500 with
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

`pub mod recording;`, and one line in the route list:
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

Appended after `ScopeNodeStop` (`crates/scope-link/proto/scope_node/v1/scope_node.proto:100`):

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
  sender and answer each with that frame's `id`, `width`, `height` and pixels (one
  clone per extra waiter). A frame the node doesn't send answers nobody.
- `impl ScopeNodeRaw for NodeService`: `raw_frame` waits for `controls.ready(false)`,
  as `request_preview` does (`crates/scope-node/src/service.rs:386`), pushes a new
  sender and awaits its receiver. A dropped sender answers
  `Status::unavailable("the scope node stopped capturing")`.
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

`RawClient::new` sets `max_decoding_message_size(RAW_MESSAGE_LIMIT)`. `pair`
(`crates/finder/src/link.rs:313`) builds it next to the `MountClient` and returns a
private `struct Paired { client, frames, camera, mount, raw }` instead of a longer
tuple. `run` publishes it on a new watch channel where it publishes the mount client
(`crates/finder/src/link.rs:376`) and clears it where it clears that
(`crates/finder/src/link.rs:393`). `LinkManager::start`'s signature, `MountClient` and
the frame arm don't change.

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

pub enum RawRefusal { NotRecording, NotConnected, Busy }

impl Recorder {
    /// Save `frames` raw frames into the recording on, one after another.
    pub fn request_raw(&self, frames: u8, client: Option<RawClient>)
        -> Result<RecordingView, RawRefusal>;
}
```

Refusals in this order: no recording on is `NotRecording`; a request whose `saved` is
below `wanted` and has no `error` is `Busy`; no client is `NotConnected`. An accepted
request sets `raw` to `{wanted, saved: 0}` and spawns a task kept with the recording:
for each frame, `time::timeout(RAW_WAIT, client.raw_frame())`, then on the blocking pool
`image::GrayImage::from_raw(width, height, pixels)` (a size mismatch is
`the raw frame request failed`), saved with `ImageFormat::Png` as `raw-<id>.png.part`
and renamed to `raw-<id>.png`, then `saved += 1`. `Unimplemented` sets
`this scope node cannot send raw frames`, a timeout `no raw frame arrived within 15 s`,
any other status `the raw frame request failed`, and a failed write ends the recording
with `write_failed`. Ending the recording aborts the raw task.

### The raw route (`crates/finder/src/api.rs`, US2)

`.route("/api/record/raw", post(record_raw))` behind `SessionWrite`, with
`#[serde(deny_unknown_fields)] RawEdit { frames: u8 }` read through `sim_json`; a value
outside 1 to `MAX_RAW_FRAMES` answers 400 with
`frames must be a whole number from 1 to 10`. No recorder answers 409
`this console has no recordings folder`. `NotRecording`, `Busy` and `NotConnected`
answer 409 with `start a recording first`, `raw frames are already being saved` and
`the scope node is not connected`. Success answers `get_session`. The client comes from
`state.link().and_then(|link| link.raw())`.

### `crates/finder/README.md` and `GLOSSARY.md` (US2)

A row after US1's:

```markdown
| `POST /api/record/raw` | JSON `{"frames":N}`, N from 1 to 10. While recording, save the next N star frames' 8-bit pixels as `raw-<id>.png`; progress is `recording.raw` in the session. Needs a scope node built with 025. Same-origin writes only. |
```

and a paragraph in `## Recording a night` on raw frames, their PNGs, `recording.raw`
and its three errors. After **Recording** in `GLOSSARY.md`:

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
`/api/record/raw`.

`components/RecordingSection.tsx`:

```tsx
export function RecordingSection({ session, onChange }: { session: ObservingSession; onChange?: () => void }) {
  // Nothing without a paired node.
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

Each successful post calls `onChange`. Write each phrase a test asserts on one line of
JSX, so `textContent` keeps it whole.

`SetupPage.tsx`: `<RecordingSection session={session} onChange={onChange} />` right
after `<AimTap … />` (`crates/finder/web/src/components/SetupPage.tsx:179`).

### `DESIGN.md` (US3)

After the Aim-point tap entry (`DESIGN.md:343`), add:

> ### Recording — built
> On the Setup page, after the aim point, while a scope node is paired: a section titled
> Recording with one primary button, Start recording. While recording, one status line
> names the folder and counts, with Stop recording and Save 3 raw frames as secondary
> buttons; raw progress sits under the status and a failure is an alert. After a
> recording the status says what was saved. Ink on paper with existing tokens only; at
> night it is red like every other section.

## Shapes that existing tests pin

These are not changed by any story:
- **The `ScopeNode` service and every existing message.** Two hand-written
  implementations of the trait (`crates/finder/tests/aim_cli.rs:22`,
  `crates/scope-link/tests/link.rs:55`) compile unchanged because the new call is in a
  new service. `StarFrame` gains no field: a test builds it as a literal naming every
  field (`crates/scope-link/tests/link.rs:25`).
- **`LinkManager::start`'s signature and `MountClient`**, used by many tests through
  `with_link` and directly in `crates/finder/tests/link.rs`.
- **`GET /api/session` without a recording.** `recording` is skipped when absent, so
  every existing body is byte for byte what it was.
- **`GET /api/live`.** The recorder is one more subscriber of the same broadcast.
- **The binary's start-up lines.** `crates/finder/tests/simulate_cli.rs` reads the
  address line `finder console: http://`; nothing new is printed.
- **The Setup page with no `node`, or with a `node` and no `recording`.** Existing tests
  read the page's only `status` (`crates/finder/web/src/components/SetupPage.test.tsx:37`)
  and only `alert` (`crates/finder/web/src/components/AimTap.test.tsx:102`); the new
  section adds neither then, and no button whose name another test uses.

## Decisions and the alternatives rejected

| Decision | Rejected | Why |
|---|---|---|
| Record from the link's frame channel and the live broadcast | A writer inside `active` in `link.rs` | The channel already feeds two readers; a writer in the link loop shares a task with the heartbeat and Stop's client. A missed frame shows as a gap in the ids. |
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
   `017-console-stop-confirm` and `019-night-legibility`. Met: 017's last story landed at
   c1ed295.
2. **`main` is still as read.** Every citation here is at c1ed295. If `main` has moved in
   these files, re-read them and re-run the validator.
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
   and decode PNGs with `image`. No test reads a recording through `Recorder`.
7. **A new call on `ScopeNode` breaks two tests.** *Check:* the call is in the new
   `ScopeNodeRaw` service, and `ScopeNode`, `StarFrame` and every existing message keep
   their calls and fields.
8. **tonic's 4 MiB default.** A 1456 × 1088 test frame passes without the raised limit.
   *Check:* US2-S6 sends a 2592 × 1944 frame.
9. **The newest-frame channel.** A recorder that writes the frame already on the channel
   at its start, or a solve of an earlier frame, gives solve lines without frame lines.
   *Check:* US1-S1 asserts every solve line names a frame line.
10. **A cut line.** Aborting a task at an await can leave half a line. *Check:* lines are
    written synchronously, one call each, and PNGs and `recording.json` are written
    under a `.part` name and renamed.
11. **The Setup page's only status and alert.** *Check:* US3-S1 asserts the section shows
    neither with a node and no recording.
12. **The bundle must be rebuilt.** *Check:* US3 commits `crates/finder/web/dist` after
    `npm run build`, so that the gate's `git status --porcelain -- dist` finds nothing.
13. **Each worktree builds on its own.** *Check:* no story sets `CARGO_TARGET_DIR`, and
    test folders go under the workspace's `target/`, as existing tests put them.

## Risks

| Risk | Mitigation |
|---|---|
| A forgotten recording fills the disk. | The 3,600-frame limit, and the Setup page shows a running recording and its counts. |
| A recording with the site is published. | Owner decision 9, and the README's note; no address or token is ever written. |
| Raw requests slow the scope node. | One pixel clone per waiting call; at most ten frames a request, one request at a time. |
| A recorded frame is missed. | Gaps show in the ids; the cadence of 200 ms or more leaves the writer idle most of the time. |
| An older scope node. | It answers `Unimplemented`, which the page and session report as `this scope node cannot send raw frames`. |
| Draft 020 "mount-sky" (pull request 207) also edits `link.rs`'s frame arm and adds a filter to `LinkManager`. | Whichever lands second is re-read and re-validated before it is flipped. |

## Refinement Record

Drafted on 2026-11-14 by the plate-solver operator against `main` at c1ed295, from gap
3 of the field findings of 13–14 November and a read of the frame path from the camera
port to `GET /api/live`. No code was built or probed: the interfaces were read from the
cited lines. The risk that a replayed frame solves differently is carried by US1-S3, and
that tonic's limit or the node's waiters behave otherwise by US2-S2 and US2-S6. Writes
come from the task text. Shared writes name the merge edge that orders them, and the 016
and 017 stories that last wrote the same file on `main`. Nothing in this record is an
owner approval: those are in spec.md, and flipping to `ready` gives them.

```yaml
version: 1
reviewed_revision: c1ed295734253c7d6121052b0a14beb2ee64bc17
stories:
  US1:
    tests:
      - path: crates/finder/tests/recording.rs
        status: new
        disposition: add
        assertion: "a start and stop write recording.json with the format, version, site and UTC name, frames.jsonl and solves.jsonl whose counts match the session, every solve naming a recorded frame, nothing after the stop, and no address or token"
        decision: "spec.md:US1-S1"
      - path: crates/finder/tests/recording.rs
        status: new
        disposition: add
        assertion: "each frame line equals the star frame the test saw, centroids as triples in order, previews byte for byte, and each solve line equals the live report's JSON"
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
        assertion: "RawFrame needs the token, and two waiting calls are both answered with the same id"
        decision: "spec.md:US2-S2"
      - path: crates/finder/tests/recording_raw.rs
        status: new
        disposition: add
        assertion: "two requested raw frames are saved as gray PNGs of their frame lines' size whose detected centroids equal the lines', with no .part file left"
        decision: "spec.md:US2-S3"
      - path: crates/finder/tests/recording_raw.rs
        status: new
        disposition: add
        assertion: "the raw route refuses without a recording, with bad bodies, a foreign origin, a running request or no node, and a stop ends a request"
        decision: "spec.md:US2-S4"
      - path: crates/finder/tests/recording_raw.rs
        status: new
        disposition: add
        assertion: "a node without ScopeNodeRaw gives the error this scope node cannot send raw frames and no file"
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
        assertion: "the link's frames, status and mount client are unchanged"
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
        assertion: "Setup shows a Recording region with Start recording and no status or alert when a node is paired, posts on and shows a refusal as an alert, and shows no region without a node"
        decision: "spec.md:US3-S1"
      - path: crates/finder/web/src/components/RecordingSection.test.tsx
        status: new
        disposition: add
        assertion: "a running recording reads its folder and counts, Stop recording posts off and Save 3 raw frames posts three"
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
        assertion: "the Setup page at night is still red only"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "the page adds no npm package (DESIGN.md)"
      - "red-only night mode: the section uses existing tokens and classes"
    checks:
      development:
        - "npm test"
        - "npm run typecheck"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs: finder-page"
    writes:
      - DESIGN.md
      - crates/finder/web/dist
      - crates/finder/web/src/api.ts
      - crates/finder/web/src/components/RecordingSection.test.tsx
      - crates/finder/web/src/components/RecordingSection.tsx
      - crates/finder/web/src/components/SetupPage.tsx
    no_write_reason: ""
    shared_writes:
      - path: DESIGN.md
        other_spec: 017-console-stop-confirm
        other_story: US5
        disposition: merge_order
        reason: "025 depends_on_landed 017, whose US5 last changed DESIGN.md"
      - path: crates/finder/web/dist
        other_spec: 017-console-stop-confirm
        other_story: US5
        disposition: merge_order
        reason: "025 depends_on_landed 017, whose US5 last rebuilt the bundle"
      - path: crates/finder/web/src/api.ts
        other_spec: 017-console-stop-confirm
        other_story: US5
        disposition: merge_order
        reason: "025 depends_on_landed 017, whose US5 last changed api.ts"
      - path: crates/finder/web/src/components/SetupPage.tsx
        other_spec: 016-mcp
        other_story: US7
        disposition: merge_order
        reason: "025 depends_on_landed 016, whose US7 last changed SetupPage.tsx"
    open_questions: []
```
