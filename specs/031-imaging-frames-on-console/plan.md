# Implementation Plan: Imaging Frames on the Console

**Branch**: `031-imaging-frames-on-console` | **Date**: 2026-11-15 | **Spec**: [spec.md](spec.md)

This plan records every design decision for the feature. A story that would change one
escalates rather than re-deciding it.

## Summary

The console opens a second connection to the scope node for 030's `ScopeNodeImaging`
service alone, so that frame bytes never share the heartbeat's connection. It keeps the
camera's state in an `imaging` object, saves each imaging frame as a FITS file, makes a
colour preview, shows both on a new Imaging page that is red at night with colour while
a control is held, and offers three MCP tools. Its imaging state holds the camera for
one holder at a time, the lease that 033 to 037 take. Everything on the scope node's
side (the camera port, the camera adapters, the service, the simulated imaging camera,
the node's own copy and the probe) is `030-imaging-camera-link`, which has landed
before this trio's first story starts; § What this trio takes from 030 names what is
used of it.

## Technical Context

**Language/Version**: Rust (MSRV 1.96) for US1 to US6, US8 and US9; TypeScript with
React 19 for US7.

**Primary Dependencies**: none new, and no story changes a `Cargo.toml` or `Cargo.lock`.
`finder` has what it needs: `image` with `jpeg` (`crates/finder/Cargo.toml:27`),
`base64` (`crates/finder/Cargo.toml:11`), and `rmcp` 3.9.0's `ContentBlock::image`. No
FITS crate. The page adds no npm package.

**Storage**: on the console, `frames/<UTC date>/<id>.fits` under the imaging folder,
`imaging/` in the state folder or `--imaging-dir`; previews in memory only. Nothing
about imaging is kept in `session.json`: the camera keeps its own settings, and the
console's three (`exposure_ms`, `forward_kib_per_s`, `save_at_node`) last for a run.

**Testing**:
- US1: the module `imaging` of finder's `console` and `cli` test binaries (new files
  `crates/finder/tests/console/imaging.rs`, `crates/finder/tests/cli/imaging.rs`).
- US2: the module `imaging_frames` of `console` and `cli`.
- US3: the module `imaging_fits` of `console`.
- US4: the module `imaging_checks` of `console`, whose node's `ScopeNodeImaging` is the
  test's own.
- US5: the module `imaging_control` of `console`.
- US6: the module `imaging_preview` of `console`.
- US7: `crates/finder/web/src/components/ImagingPage.test.tsx` (new, Vitest) and
  `crates/finder/web/e2e/imaging.spec.ts` (new, Playwright, simulate project).
- US8: the module `imaging` of finder's `mcp` test binary.
- US9: the module `imaging_lease` of finder's `console` test binary (new file
  `crates/finder/tests/console/imaging_lease.rs`), which reaches the MCP door through
  `crate::support::mcp` as `crates/finder/tests/motion/mount_identity.rs:4` does.
- Where a finder test goes is 027's rule: `console` calls the router in process, `cli`
  starts the built `finder`, `mcp` goes through the MCP door. Each module is one file in
  its binary's folder and one `mod` line, in alphabetical order, in that binary's
  `main.rs` (`crates/finder/tests/layout/main.rs:102`). A module writes its own setup;
  `crates/finder/tests/support` takes no new file
  (`crates/finder/tests/layout/main.rs:127`), and an unused helper fails
  `clippy -D warnings`. It may bind the live helpers with
  `use crate::support::live as support;`, and it takes its scratch folder from
  `crate::support::Scratch` (`crates/finder/tests/support/mod.rs:12`), the helper
  028's US5 added.
- A test's files go where spec.md's shared setup says (**Scratch folders**): under
  `CARGO_TARGET_TMPDIR`, removed on drop, never in a path built from
  `CARGO_MANIFEST_DIR` and `../../target`.
- Every Rust test but the two `cli` modules runs in paused tokio time over in-memory
  streams, so every timeout in this plan is asserted exactly and none is waited for in
  real time.

**Target Platform**: the `finder` console on any desk machine, and the page in its
browser.

**Constraints**:
- No `unsafe`.
- A saved imaging frame holds the samples as they came, and is never rewritten.
- Nothing is keyed to one camera model.
- Rendering stays cheap: the simulated imaging camera is 1236 × 822 by default and no
  test renders the 4944 × 3284 size.
- `web/dist` is committed and must be the build of its source (US7).
- Budgets are things a test or 030's probe measures; this plan states none for the real
  camera.

**Scale/Scope**: one new module tree, five routes, three MCP tools, one page, one
console flag, one lease.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Nothing touches the solver, star detection or `crates/math-core`; the finder camera's path is unchanged. Behaviour here is grounded in the FITS rules the research note cites (`docs/research/2026-11-15-imaging-camera.md` § 2. INDI), in 030's interfaces, or in the simulator's truth. |
| II. Never weaken a gate | No existing test, golden, fixture, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS` or `ergane.yaml` is edited. The only edits to existing test or gate files are the `mod` lines and the two list entries in `playwright.config.ts` that spec owner approval 8 names, with the re-wrap of one list that Prettier requires. |
| III. Tests first, independent expectations | Oracles: the stand-in camera's sample rule, worked examples (FITS bytes, the preview, the time a paced frame takes), the simulator's truth, and the bytes a test's own node sends. Tests read FITS files with code of their own. Clocks are injected; virtual time only in tests and `finder --simulate`. |
| IV. Critical paths | No `unsafe`. No dependency change: no `Cargo.toml` and no `Cargo.lock` is written. No scope-link change: 030 made it. Nothing under `proto/`, no database-format change, no `build.rs` change. |
| V. Proven open source first, honest licensing | The finder camera's solver is untouched. Nothing of another project is linked or copied, and no crate is added. |

## Project Structure

```text
crates/finder/
├── README.md                           # US1, US2, US5, US6, US7, US8, US9
├── src/lib.rs                          # US1: pub mod imaging, route lines; US2, US5, US6: route lines
├── src/link.rs                         # US1: ImagingClient, connect_imaging
├── src/api.rs                          # US1: with_imaging, imaging field, merged routes, two visibilities
├── src/main.rs                         # US1: --imaging-dir
├── src/imaging/mod.rs                  # US1: new, state, RoiEdit; US2: frames; US5: abort, refusals; US6: previews; US9: the lease
├── src/imaging/routes.rs               # US1: new; US2, US5, US6: their routes
├── src/imaging/frames.rs               # US2: new, receive and save; US4: its checks; US5: abort, refusals, the node's copy; US6: the preview; US9: the lease's check in take
├── src/imaging/fits.rs                 # US2: new, encode; US3: decode
├── src/imaging/preview.rs              # US6: new
├── src/mcp/tools.rs                    # US8: three tools
├── tests/console/main.rs               # US1 to US6, US9: one mod line each
├── tests/console/imaging.rs            # US1: new
├── tests/console/imaging_frames.rs     # US2: new
├── tests/console/imaging_fits.rs       # US3: new
├── tests/console/imaging_checks.rs     # US4: new
├── tests/console/imaging_control.rs    # US5: new, the node's copy read back included
├── tests/console/imaging_preview.rs    # US6: new
├── tests/console/imaging_lease.rs      # US9: new
├── tests/cli/main.rs                   # US1, US2: one mod line each
├── tests/cli/imaging.rs                # US1: new
├── tests/cli/imaging_frames.rs         # US2: new
├── tests/mcp/main.rs                   # US8: mod imaging
├── tests/mcp/imaging.rs                # US8: new
└── web/
    ├── src/api.ts                      # US7: imaging types and calls
    ├── src/useImaging.ts               # US7: new
    ├── src/useColourHold.ts            # US7: new, the hold-to-see-colour control (035 reuses it)
    ├── src/App.tsx                     # US7: the /imaging route
    ├── src/components/TopBar.tsx       # US7: the Imaging link
    ├── src/components/ImagingPage.tsx          # US7: new
    ├── src/components/ImagingPage.test.tsx     # US7: new
    ├── e2e/imaging.spec.ts             # US7: new
    ├── playwright.config.ts            # US7: two list entries, one list re-wrapped (spec owner approval 8)
    └── dist/                           # US7: rebuilt
DESIGN.md                               # US7
.impeccable/design.json                 # US7
```

Nothing under `crates/scope-node`, `crates/scope-link` or `crates/simulator` is written
by this trio, and neither is `Cargo.lock`.

## Interfaces

### What this trio takes from 030 (named interfaces; no story here writes them)

Every name below is created by `030-imaging-camera-link` and was taken from its plan.md
as drafted. 030 has landed, and each was read on `main` at 49f8dcd and found as written
here (§ Prerequisites 2); they are named without a line. A story that finds one of them
different on `main` escalates rather than changing it.

**The service** (`crates/scope-link/proto/scope_node/v1/scope_node.proto`, 030 US3; 030's
plan.md § The scope link). `ScopeNodeImaging`, under the pairing token, with five calls;
the generated `ScopeNodeImagingClient` is what § The link wraps, and a test's own node
implements the generated `ScopeNodeImaging` trait and serves it with
`ScopeNodeImagingServer`:

| Call | Request → reply |
|---|---|
| `DescribeImaging` | `DescribeImagingRequest {}` → `ImagingInfo` |
| `ImagingStatus` | `ImagingStatusRequest {}` → `ImagingStatusReply` |
| `ConfigureImaging` | `ImagingSettings`, an absent field leaving that setting alone → `ImagingSettings`, the settings in force |
| `Expose` | `ExposeRequest` → a stream of `ExposeEvent`: one `header`, then `chunk`s of at most 262,144 bytes, then the end |
| `AbortExposure` | `AbortExposureRequest {}` → `AbortExposureReply {}` |

Its messages, as the proto gives them (an `optional` scalar and a message-typed field
are an `Option` in Rust; the generated code on `main` is the authority for every name):

```text
ImagingRange       { min: f64, max: f64 }
ImagingRoi         { x: u32, y: u32, width: u32, height: u32 }       unbinned sensor pixels
ImagingInfo        { model: String, serial: optional String, width: u32, height: u32,
                     pixel_um: f64, bits: u32, bayer: String (empty for a mono sensor),
                     exposure_ms: ImagingRange, gain: ImagingRange (absent: no control),
                     offset: ImagingRange (absent: no control), binnings: [u32], cooler: bool }
ImagingSettings    { gain: optional f64, offset: optional f64, binning: optional u32,
                     roi: ImagingRoi (absent: unchanged), cooler_on: optional bool,
                     set_temp_c: optional f64 }
ImagingStatusReply { temp_c: optional f64, cooler_power: optional f64 (percent),
                     set_temp_c: optional f64, exposing: bool }
ExposeRequest      { exposure_ms: f64, purpose: String, max_bytes_per_s: u32 (0: no limit),
                     save: bool }
ExposeEvent        { event: one of header (ImagingFrameHeader) or chunk (bytes) }
ImagingFrameHeader { id: u64 (the node's count, from 1), width: u32, height: u32, bits: u32,
                     bayer: String, start_s: f64, end_s: f64 (the node's clock),
                     exposure_ms: f64, gain: optional f64, offset: optional f64,
                     binning: u32, temp_c: optional f64, byte_count: u64, purpose: String,
                     node_file: String, node_save_error: String, roi: ImagingRoi }
```

What the node answers, which § The console's imaging state and § Receiving and saving
turn into states and words:

- a wrong token: `Unauthenticated`, as the call's own status;
- a node built before 030: `Unimplemented`;
- a node with no imaging camera: `FailedPrecondition` with `no imaging camera`, as the
  call's status for the four unary calls and as the stream's first and only item for
  `Expose`;
- a value the camera refuses: `InvalidArgument` with the camera's text, for example
  `gain 301 is outside the camera's range 0–300`,
  `binning 3 is not one of the camera's binnings 1, 2` and
  `set_temp_c must be between -50 and 50`;
- an exposure already running: `ResourceExhausted` with
  `an exposure is already in progress`; an aborted one: `Aborted` with
  `the exposure was aborted`; a camera failure: `Internal` with the camera's words;
- for `Expose`, every failure after the token check is the stream's first and only
  item, never the call's own status; the node finishes an exposure whose caller has
  gone, and only `AbortExposure` aborts;
- with `max_bytes_per_s` above 0 the node sends a chunk no sooner than the bytes
  already sent divided by that rate after the first chunk;
- with `save` true the header's `node_file` names the node's own copy
  (`frame-000001.fits`), or its `node_save_error` says why there is none
  (`this scope node has no save_dir`, or `could not save the imaging frame: ` and the
  error); both are empty when `save` is false.

**The port** (`crates/scope-node/src/imaging/mod.rs`, 030 US1; 030's plan.md § The port),
which a test's own camera implements:

```rust
pub type ImagingFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, ImagingError>> + Send + 'a>>;
pub trait ImagingCamera: Send + Sync {
    fn describe(&self) -> ImagingFuture<'_, ImagingInfo>;
    fn status(&self) -> ImagingFuture<'_, ImagingStatus>;
    fn apply<'a>(&'a self, settings: &'a ImagingSettings) -> ImagingFuture<'a, ImagingSettings>;
    fn expose<'a>(&'a self, exposure: &'a Exposure) -> ImagingFuture<'a, ImagingFrame>;
    fn abort(&self) -> ImagingFuture<'_, ()>;
}
pub enum ImagingError { Invalid(String), Busy, Aborted, Failed(String) }
```

with `ImagingInfo`, `ImagingSettings`, `ImagingStatus`, `Exposure { exposure_ms, purpose }`,
`ImagingFrame`, `Roi`, `Range` and `BayerPattern` as that section gives them.
`ImagingFrame` derives `Default`, and a test ends every literal of it with
`..Default::default()`.

**The stand-in camera** (`crates/scope-node/src/imaging/synthetic.rs`, 030 US1; 030's
plan.md § The stand-in camera): `SyntheticImagingCamera::new(clock: Arc<dyn NodeClock>)`.
Its description, the settings it starts with, its sample rule and its cooler's rule are
repeated in spec.md's shared setup, because this trio's scenarios compute from them.

**The node's service** (`crates/scope-node/src/service.rs`, 030 US3 and US7; 030's
plan.md § The node's service and § The node's copy):

```rust
pub fn with_imaging(self, camera: impl ImagingCamera + 'static) -> Self;
pub fn with_imaging_save_dir(self, dir: PathBuf) -> Self;
```

`routes()` serves `ScopeNodeImaging` whether or not a camera is attached; the node
served by `into_server()` alone has no such service, as a node built before 030.

**The simulator** (`crates/simulator/src/imaging.rs`, `crates/simulator/src/optics.rs`
and `crates/simulator/src/node.rs`, 030 US2 and US3; 030's plan.md § The simulated
imaging camera and § `SimNode`): `SimImagingCamera::new(simulator)` with the builders
`with_stars(Vec<SimStar>)` and `with_noise(bool)`;
`SimStar { ra_deg, dec_deg, magnitude, colour: [f64; 3] }`;
`Simulator::imaging_truth_at(unix_s) -> ImagingTruth`, whose `ra_deg` and `dec_deg` are
the frame centre's; and `SimNode::new`, which attaches a `SimImagingCamera` over its
simulator, so that `finder --simulate` and the console rig have one. That camera
describes itself as model `simulated imaging camera`, serial `SIM-0001`, 1236 × 822,
pixel 19.12 µm, 16 bits, RGGB, exposure 1 to 3600000 ms, gain 0 to 300, offset 0 to
100, binnings `[1, 2]`, a cooler; it starts with gain 120, offset 10, binning 1, the
whole sensor and the cooler off; at binning 2 it delivers 618 × 411; and it follows the
stand-in's cooler's rule.

### The link (`crates/finder/src/link.rs`, US1)

```rust
/// Imaging calls on a connection of their own (031). Chunks are at most 256 KiB, so
/// tonic's default message limit stands.
#[derive(Clone)]
pub struct ImagingClient { /* ScopeNodeImagingClient<InterceptedService<Channel, BearerToken>> */ }
impl ImagingClient {
    pub fn new(channel: Channel, token: &str) -> Result<Self, Status>;
    pub async fn describe(&mut self) -> Result<proto::ImagingInfo, Status>;
    pub async fn status(&mut self) -> Result<proto::ImagingStatusReply, Status>;
    pub async fn configure(&mut self, edit: proto::ImagingSettings)
        -> Result<proto::ImagingSettings, Status>;
    pub async fn expose(&mut self, request: proto::ExposeRequest)
        -> Result<tonic::Streaming<proto::ExposeEvent>, Status>;
    pub async fn abort(&mut self) -> Result<(), Status>;
}
impl LinkManager {
    /// A new connection to the paired node, for the imaging service only. It carries
    /// no heartbeat. `FailedPrecondition` (`no scope node is paired`) when unpaired.
    pub async fn connect_imaging(&self) -> Result<ImagingClient, Status>;
}
```

`LinkManager` (`crates/finder/src/link.rs:222`) gains two fields, `session: Arc<Session>`
and `transport: Transport`, clones that `start_with_filter` (which `start`,
`crates/finder/src/link.rs:236`, calls) keeps before it spawns `run`. Neither `start`'s
nor `start_with_filter`'s signature changes, nor `run`'s eight parameters
(`crates/finder/src/link.rs:402`). `Transport::connect`
(`crates/finder/src/link.rs:165`) becomes `pub(crate)`. `connect_imaging` reads the
pairing from `self.session.link_changes().borrow()`
(`crates/finder/src/session.rs:1313`), the session the manager was started with, which
under `--simulate` is the private one (`crates/finder/src/simulate.rs:104`); calls
`self.transport.connect(address)`, which for a test's connector hands out a fresh
stream; and builds the client with the pairing's token. The link's own task, its status
and its other clients never see this connection.

### The console's imaging state (`crates/finder/src/imaging/mod.rs`, new, US1; US2, US5 and US6 extend it)

```rust
//! The imaging camera as the console holds it (031).
pub mod routes;
// US2: pub mod fits; pub mod frames;   US6: pub mod preview;

pub const STATUS_PERIOD: Duration = Duration::from_secs(2);
pub const RETRY_PERIOD: Duration = Duration::from_secs(5);
pub const DEFAULT_EXPOSURE_MS: f64 = 1000.0;
pub const CALL_WAIT: Duration = Duration::from_secs(5);
pub const SET_WAIT: Duration = Duration::from_secs(30);

/// `GET /api/imaging`, and `imaging` in the session. Non-exhaustive: later specs add parts.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct ImagingView {
    /// `off`, `no_link`, `no_service`, `no_camera`, `error` or `ready`.
    pub state: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camera: Option<CameraView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<StatusView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<SettingsView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame: Option<FrameView>,          // US2
}
impl ImagingView { pub fn off() -> Self; }

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RangeView { pub min: f64, pub max: f64 }
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RoiView { pub x: u32, pub y: u32, pub width: u32, pub height: u32 }

/// Serialized in this field order, with `null` for an absent serial, Bayer pattern,
/// gain or offset.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CameraView {
    pub model: String, pub serial: Option<String>, pub width: u32, pub height: u32,
    pub pixel_um: f64, pub bits: u32, pub bayer: Option<String>, pub exposure_ms: RangeView,
    pub gain: Option<RangeView>, pub offset: Option<RangeView>, pub binnings: Vec<u32>,
    pub cooler: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StatusView {
    pub temp_c: Option<f64>, pub cooler_power: Option<f64>, pub set_temp_c: Option<f64>,
    pub exposing: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct SettingsView {
    pub exposure_ms: f64, pub gain: Option<f64>, pub offset: Option<f64>, pub binning: u32,
    pub roi: RoiView, pub cooler_on: bool, pub set_temp_c: Option<f64>,
    pub forward_kib_per_s: Option<u32>, pub save_at_node: bool,
}

/// The body of `PUT /api/imaging/settings`. Absent is unchanged; null is refused but
/// for `roi` (the whole sensor) and `forward_kib_per_s` (no limit). Later specs build
/// one with `..Default::default()`, `roi` as `Some(Some(RoiEdit { .. }))` for a region
/// and `Some(None)` for the whole sensor.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingsEdit {
    pub exposure_ms: Option<f64>, pub gain: Option<f64>, pub offset: Option<f64>,
    pub binning: Option<u32>, pub roi: Option<Option<RoiEdit>>, pub cooler_on: Option<bool>,
    pub set_temp_c: Option<f64>, pub forward_kib_per_s: Option<Option<f64>>,
    pub save_at_node: Option<bool>,
}

/// The region of interest in a settings change, in unbinned sensor pixels: the body's
/// `{"x":…,"y":…,"width":…,"height":…}`. All four are required; another key is refused
/// (US1-S3). `set` sends it to the node as `ImagingRoi` with the same four numbers.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoiEdit { pub x: u32, pub y: u32, pub width: u32, pub height: u32 }

/// Why a request was not carried out: 400, 409 or 502, and the words.
#[derive(Debug, Clone, PartialEq)]
pub struct Refusal { pub status: axum::http::StatusCode, pub message: String }

pub struct Imaging { /* root, link, clock, Mutex<state>, a watch of changes, the task */ }
impl Imaging {
    /// Start the task that follows the link. Dropping the `Imaging` ends it.
    pub fn start(root: PathBuf, link: Arc<LinkManager>, clock: Arc<dyn Clock>) -> Arc<Self>;
    pub fn view(&self) -> ImagingView;
    pub async fn set(&self, edit: SettingsEdit) -> Result<ImagingView, Refusal>;
    // US2:
    pub async fn take(&self, purpose: &str) -> Result<ImagingView, Refusal>;
    /// The frame with this id as it ended. The console keeps the last ended frame beside
    /// the latest; `ended(id)` answers it when its id is `id`, and otherwise the latest
    /// frame if its id is `id` and it has ended. For an older id, the last ended frame.
    pub async fn ended(&self, id: u64) -> FrameView;
    // US5:
    pub async fn abort(&self) -> ImagingView;
    // US6:
    pub fn preview(&self, id: u64) -> Option<Vec<u8>>;
    // US9 (§ The camera lease): lease, holder.
}
```

Each plain `Option` of `SettingsEdit` uses a `deserialize_with` of `imaging/mod.rs`'s own
that reads the value itself, as `present_value` does
(`crates/finder/src/session.rs:452`), so a JSON null is an error and not "absent". The
two double options use one written as `present_nullable` is
(`crates/finder/src/session.rs:444`), the pattern `SessionEdit` uses for a field that
may be cleared (`crates/finder/src/session.rs:422`); both helpers are private there.

**The task** follows the `connected` flag of `link.status()` and holds at most one
`ImagingClient`. `LinkStatus` changes with every answered heartbeat, four times a second
(`crates/finder/src/link.rs:209`), so the task never waits for "the status changed":
it waits, with the watch's `wait_for`, for `connected` to differ from the value it last
acted on.

- `connected` false: drop the client, state `no_link`, wait for `connected` to be true;
- `connected` true and no client: `connect_imaging`, then `describe`. `Ok` gives
  `ready`: the camera is kept, `configure` with an empty message gives the settings in
  force, `status` the status, and the console's `exposure_ms` is clamped into the
  camera's range. `Unimplemented` gives `no_service`, `FailedPrecondition` `no_camera`,
  any other error (a failed `connect_imaging`, `configure` or `status` included) `error`
  with the status's message. In those three states the client is dropped and the task
  tries again after `RETRY_PERIOD` or when `connected` turns false, whichever is first
  (US1-S10 asserts the 5 s through answered heartbeats);
- `ready`: `status` every `STATUS_PERIOD`. A failed read drops the client and is
  classified as a failed `describe` is.

The view outside `ready` holds none of `camera`, `status`, `settings` and `frame`. In
the state `error` it holds `error`; a part a later spec adds to the view says itself
whether it shows outside `ready`. From US5, the view's `status.exposing` is the
console's: true while its latest frame is `exposing`, false otherwise, whatever the
camera's own status, read every 2 s, last said (spec FR-009).
Every call the task makes is limited to `CALL_WAIT`, 5 s, and raced against `connected`
turning false, so a silent node cannot hold the task past a lost link. A call that runs
out of time is a failed call whose message is `the scope node did not answer within 5 s`.

**`set`**, in this order: not `ready` answers 409 with the state's words (`no_link`:
`the scope node is not connected`; `no_service`: `this scope node has no imaging
service`; `no_camera`: `this scope node has no imaging camera`; `error`:
`the imaging camera is not answering: {error}`); from US9, a lease out to someone else
answers 409 `the imaging camera is held by {holder}` (§ The camera lease); from US5, a
frame `exposing` or `receiving` answers 409 `an exposure is in progress`; `exposure_ms` not
finite or outside the
camera's range answers 400
`exposure_ms {value} is outside the camera's range {min}–{max} ms`;
`forward_kib_per_s` not a whole number from 64 to 1048576 answers 400
`forward_kib_per_s must be a whole number from 64 to 1048576, or null`. Then, if any
camera field is present, `configure` with exactly those fields, `roi: null` sent as the
whole sensor, `ImagingRoi { x: 0, y: 0, width, height }` from the camera's description:
`InvalidArgument` answers 400 with the node's message, any other error 502
`the scope node did not apply the settings: {message}`, and nothing is kept; the call
is limited to `SET_WAIT`, 30 s, after which the message is `no answer within 30 s`. On success
the node's answer becomes the camera settings and the console's three are stored. An
edit with no camera field calls nothing.

`AppState` (`crates/finder/src/api.rs:101`) gains `imaging: Option<Arc<Imaging>>` and

```rust
/// Follow the paired node's imaging camera and keep frames under `root` (031). Call
/// after `with_link` or `with_simulation`; without a link it changes nothing.
pub fn with_imaging(mut self, root: impl Into<PathBuf>) -> Self;
pub fn imaging(&self) -> Option<Arc<Imaging>>;
```

`SessionResponse` (`crates/finder/src/api.rs:690`) gains, after `recording`,
`#[serde(skip_serializing_if = "Option::is_none")] imaging: Option<ImagingView>`, filled
in `session_response` (`crates/finder/src/api.rs:714`) from
`state.imaging.as_ref().map(|i| i.view())`. `SessionWrite`
(`crates/finder/src/api.rs:387`) and `sim_json` (`crates/finder/src/api.rs:608`) become
`pub(crate)`. `router` (`crates/finder/src/api.rs:478`) gains
`.merge(crate::imaging::routes::routes())` before `with_state`.

### The routes (`crates/finder/src/imaging/routes.rs`, new, US1; US2, US5 and US6 add theirs)

```rust
pub(crate) fn routes() -> Router<AppState>;
// US1:  GET  /api/imaging                         get_imaging
//       PUT  /api/imaging/settings                put_settings
// US2:  POST /api/imaging/frame                   take_frame
// US5:  POST /api/imaging/abort                   abort_frame
// US6:  GET  /api/imaging/frames/{id}/preview.jpg frame_preview
```

- `get_imaging` answers `Json(view)`, or `ImagingView::off()` without imaging.
- `put_settings` takes `SessionWrite` and `Result<Json<SettingsEdit>, JsonRejection>`
  through `sim_json`, as `set_record` does (`crates/finder/src/api.rs:803`); without
  imaging it answers 409 `this console takes no imaging frames`; a `Refusal` becomes
  `ApiError::new(status, message)`. `sim_json` passes axum's own status on
  (`crates/finder/src/api.rs:608`): 415 for a `Content-Type` other than
  `application/json`, 400 for a body that is not JSON, and 400 in place of 422 for JSON
  of the wrong shape.
- `take_frame` and `abort_frame` take `SessionWrite` and no body; the same 409 without
  imaging.
- `frame_preview` takes `Path<u64>`, so an id that is not a number is axum's 400, and
  answers `([(CONTENT_TYPE, "image/jpeg")], bytes)` or 404
  `the console no longer holds that preview`, as `live_preview` does
  (`crates/finder/src/api.rs:552`).

`crates/finder/src/lib.rs` gains `pub mod imaging;` and, after its last route line
(`crates/finder/src/lib.rs:35`), one line a route, written as they stand here:
`* GET /api/imaging: the imaging camera's state, description, status, settings and latest imaging frame.`
and `* PUT /api/imaging/settings: a partial edit of exposure_ms, gain, offset, binning, roi, cooler_on, set_temp_c, forward_kib_per_s and save_at_node.`
(US1);
`* POST /api/imaging/frame: no body; take one imaging frame and save it as FITS.` (US2);
`* POST /api/imaging/abort: no body; abort the exposure or the imaging frame in flight.` (US5);
`* GET /api/imaging/frames/{id}/preview.jpg: a colour JPEG of one of the last eight imaging frames.`
(US6).

`crates/finder/src/main.rs`: `Args` (`crates/finder/src/main.rs:18`) gains

```rust
/// Folder for imaging frames (default: imaging/ in the state folder).
#[arg(long)]
imaging_dir: Option<PathBuf>,
```

and the state gains, after `.with_live()` (`crates/finder/src/main.rs:168`),
`.with_imaging(args.imaging_dir.unwrap_or_else(|| state_dir.join("imaging")))`. No new
start-up line.

### Receiving and saving (`crates/finder/src/imaging/frames.rs`, new, US2; US4 and US5 extend it)

US2 builds this section but for the parts marked US4 and US5, which the stories that
test them add: US4 the checks on what the scope node sends (spec US4-S1), and US5 the
frame-in-flight refusals, abort and the scope node's copy (spec US5-S1 to US5-S4).

```rust
// US4:
/// The most bytes of samples the console accepts for one frame.
pub const FRAME_LIMIT: u64 = 256 * 1024 * 1024;
/// How long after the last chunk the next may take.
pub const CHUNK_WAIT: Duration = Duration::from_secs(30);
/// The header may take twice the exposure plus this: the node may try twice.
pub const HEADER_SLACK: Duration = Duration::from_secs(150);

/// The latest imaging frame as the view shows it. Fields appear as the frame moves on.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct FrameView {
    pub id: u64,
    /// `exposing`, `receiving`, `saved`, `failed` or `aborted`.
    pub state: &'static str,
    pub purpose: String,
    pub exposure_ms: f64,
    /// Console Unix seconds.
    pub requested_at: f64,
    /// Unbinned sensor pixels: the region the console's settings held when the frame was
    /// asked for, replaced by the header's `roi` when the header carries one.
    pub roi: RoiView,
    /// The set temperature the settings held, with the cooler on, when the frame was
    /// asked for; `SET-TEMP` is written from it. Skipped while `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub set_temp_c: Option<f64>,
    // from the header on; every `Option` below is skipped while it is `None`:
    pub width: Option<u32>, pub height: Option<u32>, pub bits: Option<u32>,
    pub bayer: Option<String>, pub gain: Option<f64>, pub offset: Option<f64>,
    pub binning: Option<u32>, pub temp_c: Option<f64>,
    /// The exposure's start in console Unix seconds, and as `YYYY-MM-DDTHH:MM:SS.mmm`.
    pub start: Option<f64>, pub date_obs: Option<String>,
    pub bytes: Option<u64>, pub received: Option<u64>,
    pub node_file: Option<String>, pub node_save_error: Option<String>,   // US5
    /// Console Unix seconds: when the last byte was in hand, and when the file was in place.
    pub received_at: Option<f64>, pub saved_at: Option<f64>,
    /// `frames/<UTC date>/<id>.fits`, relative to the imaging folder.
    pub file: Option<String>,
    pub preview: Option<String>,           // US6
    pub error: Option<String>,
}
```

**`take`** is `frames::take`, which `Imaging::take` calls, and makes its checks under
one acquisition of the state's lock: refused as `set` is while not `ready`; from US9,
refused as `set` is while a lease is out to someone else (§ The camera lease); from US5,
with a frame `exposing` or `receiving`, 409 `an exposure is already in progress`.
Otherwise the frame gets the next id (below), the state `exposing`, as `roi` the region
the settings hold at that moment, and as `set_temp_c` the set temperature the settings
hold then, only when `cooler_on` is true. The camera's serial is kept beside it for the
file's `SERIALNO` (spec owner decision 16). A task is spawned and the view is answered.
Every frame `take` starts is saved to a file; a later spec that wants no file kept for a
frame removes it once the frame has ended (§ Shapes that existing tests pin, last
bullet).

**The task**, on a clone of the client:

1. `expose` with the settings' `exposure_ms`, the purpose, `max_bytes_per_s` =
   `forward_kib_per_s × 1024` or 0, and `save` = `save_at_node` (US5; `false` before
   it); then the first message. From US4, both are within W = `2 × exposure +
   HEADER_SLACK`, else `no imaging frame header arrived within {n} s`, n being W in
   seconds rounded up to a whole number (152 for 1000 ms, 151 for 500 ms).
2. A status ends the frame, whether it comes as the call's own answer or as the stream's
   first item: from US5, `Aborted` ends it `aborted`; any other ends it `failed` with
   `the exposure failed: {message}`. A first message that is not a header is
   `the scope node sent no imaging frame header`.
3. From US4, the header must have `bits` 8 or 16 and `byte_count` equal to
   `width × height × bits/8`, above 0 and at most `FRAME_LIMIT`, else
   `the imaging frame's header does not add up`. The state becomes `receiving`; `start`
   is `start_s − offset_s`, with `offset_s` the link's clock offset now
   (`crates/finder/src/link.rs:201`), or the one read when the frame was asked for; and
   when the header carries `roi`, it replaces the frame's `roi`. A header without one
   (a test's own node) leaves the region the settings held.
4. Chunks are appended to a buffer of `byte_count` bytes, `received` rising. The end of
   the stream, an error on it, or an event that is not a chunk, before `byte_count` is
   reached: `the imaging frame was incomplete: {received} of {bytes} bytes`. From US4, a
   chunk that would take `received` past `byte_count` is
   `the scope node sent more bytes than the header said`, and `CHUNK_WAIT` without a
   chunk `the imaging frame stalled`. When `received` equals `byte_count` the frame is
   whole: `received_at` is the console clock's reading, and the stream is dropped
   without reading on.
5. The whole frame is saved on the blocking pool (below); a failure is
   `could not save the imaging frame: {error}`. US6 makes the preview in the same step.
6. The state becomes `saved`, and `saved_at` is the console clock's reading.

**`abort`** (US5): with a frame `exposing`, call `AbortExposure` on a clone of the
client and let the task end the frame when the node's `Aborted` arrives, or end it
`aborted` directly if that call fails; with a frame `receiving`, abort the task, which
drops the stream, and end the frame `aborted`. Otherwise nothing.

**The id** is the console's own. At the first `take` of a run, and at any `take` whose
UTC date differs from the last one's, the folder of the take's UTC date,
`frames/<that date>/`, is listed and the count is raised to the largest `NNNNNN.fits`
number in it; it never falls. The id is the count plus one, and the count rises with
every `take` that starts a frame, saved or not, so no id is used twice in a run. The
file goes under the date of the exposure's start (below), which for a frame asked for
just before midnight UTC is the next date; that folder is not listed for it, and the
existing-name check keeps any file already there.

**The file** is `frames/<UTC date of start>/<id as six digits>.fits`. It is written
whole as `<name>.part` with `create_new`. Then, if the final name exists
(`Path::try_exists`), the `.part` file is removed and the frame ends `failed` with
`could not save the imaging frame: frames/<date>/<id>.fits already exists`; otherwise
the `.part` file is renamed to it. A rename by itself replaces an existing file without
a word, which is why the check is there. One console writes an imaging folder, so a
check is enough; a hard link is not used because the folder may be on a file system
that has none. An existing name is never opened for writing.

**A stack's subs are saved elsewhere** (035, not this spec). The owner settled 035's
owner decision 13 on 2026-11-17 as a folder per run: a stack's subs go in
`runs/<stack id>/subs/`, not here. 035's US5 adds `Lease::save_in(folder)` to this
spec's lease and has this step write a frame taken under a lease that names a folder
as `<folder>/<id>.fits`, by the same `.part` and existing-name rule
(`specs/035-live-stack/plan.md` § A lease's folder). No story of this spec writes or
calls it: every frame this spec takes, through `Imaging::take` or `Lease::take`, is
saved under `frames/<UTC date>/` as above, the id rule above is the one count for
every frame, a stack's subs included, and this spec's tests read every file where this
section puts it.

### FITS at the console (`crates/finder/src/imaging/fits.rs`, new, US2; US3 adds `FitsError` and `decode`)

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum CardValue { Logical(bool), Int(i64), Real(f64), Text(String) }

/// One FITS image: `bits` 8 or 16, little-endian samples.
#[derive(Debug, Clone, PartialEq)]
pub struct FitsImage {
    pub width: u32,
    pub height: u32,
    pub bits: u32,
    pub samples: Vec<u8>,
    /// The cards after the structural ones, in order.
    pub cards: Vec<(String, CardValue)>,
}
pub fn encode(image: &FitsImage) -> Vec<u8>;
/// `YYYY-MM-DD` and `YYYY-MM-DDTHH:MM:SS.mmm` of console Unix seconds, in UTC.
pub fn utc(unix_s: f64) -> (String, String);

// US3:
/// Why bytes are not an image `encode` writes. Displays as its text.
#[derive(Debug, Clone, PartialEq)]
pub struct FitsError(pub String);
// impl fmt::Display (the text), impl std::error::Error
pub fn decode(bytes: &[u8]) -> Result<FitsImage, FitsError>;
```

`encode` writes `SIMPLE = T`, `BITPIX`, `NAXIS = 2`, `NAXIS1`, `NAXIS2`, then for 16 bits
`BZERO = 32768` and `BSCALE = 1`, then `cards`, then `END`; pads the header with spaces
and the data with zeros to multiples of 2880. A card is 80 bytes: the keyword padded to
8, `= `, then a logical (`T` or `F`) or a number right-justified to column 30, or a text
in single quotes from column 11, padded inside the quotes to 8 characters, quotes
doubled, cut at 68. A real is written with Rust's `{:?}` with `e` made `E` (`0.5`,
`100.0`, `-9.5`, `1E-7`). A 16-bit sample v is stored as the big-endian `i16`
`v − 32768`; an 8-bit sample as it is.

`decode` (US3) reads what `encode` writes and refuses the rest. Its checks, in this order,
each with exactly this text: the first card is not `SIMPLE` with the value `T`, or no
`END` card is found, `not a FITS image`; `NAXIS` is not 2,
`a {n}-axis FITS file is not one image`; `BITPIX` is neither 8 nor 16,
`unsupported FITS BITPIX {n}`; `BITPIX` is 16 and `BZERO` is not 32768,
`unsupported FITS BZERO {value}`, the value as the card writes it, or `0` when the card
is absent; fewer data bytes follow the header than `NAXIS1 × NAXIS2 × BITPIX/8`,
`the FITS data is short: {have} of {need} bytes`. It answers the samples little-endian,
a stored 16-bit value s as `s + 32768`, and as `cards` every card after `SIMPLE`,
`BITPIX`, `NAXIS`, `NAXIS1`, `NAXIS2`, `BZERO` and `BSCALE`, in the file's order up to
`END`: a value in single quotes is `Text`, its doubled quotes made single and its
trailing spaces dropped; `T` or `F` is `Logical`; a number with no `.`, `E` or `e` is
`Int`; any other number is `Real`. So `decode(&encode(&image))` equals `image` for
every image whose texts end in no space and whose reals are finite (US3-S1). It is
here because the plan document puts reading in this file; 032 is its first caller, and
a test of a saved file never reads it through `decode` (§ Plan traps 6).

**The worked FITS frame** of US3-S1 is the one 030's camera adapter reads from a
camera (030's plan.md § Reading a FITS BLOB), written out here because `decode` reads
it too. It is 5760 bytes: one header block with the cards
`SIMPLE  =                    T`, `BITPIX  =                   16`,
`NAXIS   =                    2`, `NAXIS1  =                    4`,
`NAXIS2  =                    2`, `BZERO   =                32768`,
`BSCALE  =                    1`, `XBINNING=                    1`,
`YBINNING=                    1`, `CCD-TEMP=                 -9.5`,
`BAYERPAT= 'RGGB    '` and `END`, each padded with spaces to 80 bytes and the block to
2880; then the 16 bytes `80 00 80 01 80 02 80 03 00 00 1C 40 7F FE 7F FF` and zeros to
2880. The test builds it with a helper of its own.

A saved frame's cards, in order: `DATE-OBS` (text), `EXPTIME` (real, seconds: the
header's `exposure_ms` over 1000), `GAIN`
(real), `OFFSET` (real), `XBINNING` and `YBINNING` (int), `XORGSUBF` and `YORGSUBF`
(int: the frame's `roi` `x` and `y` as it stands when the frame is saved, the header's,
each divided by the header's `binning` and rounded down, which is the origin in binned
pixels), `CCD-TEMP` (real), `SET-TEMP` (real, the frame's `set_temp_c`), `BAYERPAT`
(text), `INSTRUME` (text, the camera's model), `SERIALNO` (text, the camera's serial),
`IMAGETYP` (`DARK`, `FLAT` or `LIGHT`), `ROWORDER` (`TOP-DOWN`), `FRAMEID` (int, the
console's id), `PURPOSE` (text). `GAIN`, `OFFSET`, `CCD-TEMP` and `BAYERPAT` are left
out when the frame has none, `SET-TEMP` when the frame has no `set_temp_c`, and
`SERIALNO` when the camera has no serial. `XORGSUBF` and `YORGSUBF` are always written;
the region's size is `NAXIS1` and `NAXIS2` with `XBINNING` (spec owner decision 16).
The view's `frame.exposure_ms` stays the exposure the console asked for; only the file's
`EXPTIME` is the header's (spec US4-S2).

### The preview (`crates/finder/src/imaging/preview.rs`, new, US6)

```rust
pub const PREVIEW_SIDE: u32 = 1280;
pub const PREVIEW_QUALITY: u8 = 85;
pub const PREVIEWS_KEPT: usize = 8;
pub const STRETCH: f64 = 10.0;
pub const NOISE_FLOOR: f64 = 30.0;

pub struct Rgb { pub width: u32, pub height: u32, /** RGB, 3 bytes a pixel. */ pub pixels: Vec<u8> }
/// `bayer` is `"RGGB"`, `"BGGR"`, `"GRBG"` or `"GBRG"`; any other value, and `None`, is
/// read as mono.
pub fn rgb(samples: &[u8], width: u32, height: u32, bits: u32, bayer: Option<&str>) -> Rgb;
/// `None` when the picture has no pixels, which `image`'s encoder refuses.
pub fn jpeg(picture: &Rgb) -> Option<Vec<u8>>;
```

1. **Colour.** For a mosaic, each whole 2 × 2 cell becomes one pixel: red and blue are
   their samples, green the mean of its two; a trailing odd row or column is dropped. A
   mono frame gives one pixel a sample, the same in all three channels.
2. **Size.** With k = ⌈longer side / 1280⌉, each k × k block becomes its mean; trailing
   rows and columns that do not fill a block are dropped. k is 1 for a small frame.
3. **Stretch.** For a channel with n values, sorted, the p-th percentile is the value at
   index ⌊p·(n − 1)⌋. For each channel c: black_c is its 50th percentile, and noise_c is
   that less its 16th. The span is the largest, over the three channels, of
   (99.9th percentile − black_c) and 30·noise_c, and at least 1. A value v becomes
   `round(255·asinh(10·t)/asinh(10))` with t = clamp((v − black_c)/span, 0, 1).
4. **JPEG** at quality 85 with `image`'s encoder.

Worked example (US6-S1): nine red values 100 × 6, 400, 700, 900 give black 100, noise 0
and a 99.9th percentile of 700; nine blue values 50 × 7, 200, 350 give black 50 and 200;
nine green values 100 × 8 and 250, the mean of 400 and 100, give black 100 and a 99.9th
percentile of 100. The span is 600, so red 400 is t = 0.5, 197; red 700 and 900 are
255; blue 200 is t = 0.25, 140; blue 350 is 197; green 250 is t = 0.25, 140.

The console keeps the JPEGs of the last `PREVIEWS_KEPT` saved frames, by id, in memory.
`frame.preview` is `/api/imaging/frames/{id}/preview.jpg`, set when the frame is saved.
A preview that cannot be made, `jpeg` answering `None`, does not fail the frame;
`preview` is then absent (spec US6-S3, a 64 × 1 mosaic with no whole 2 × 2 cell).

### The page (`crates/finder/web/src/`, US7)

`api.ts`, after `SessionRecording`:

```ts
export interface ImagingRange { min: number; max: number }
export interface ImagingCameraInfo {
  model: string; serial: string | null; width: number; height: number; pixel_um: number;
  bits: number; bayer: string | null; exposure_ms: ImagingRange; gain: ImagingRange | null;
  offset: ImagingRange | null; binnings: number[]; cooler: boolean;
}
export interface ImagingStatus {
  temp_c: number | null; cooler_power: number | null; set_temp_c: number | null; exposing: boolean;
}
export interface ImagingSettings {
  exposure_ms: number; gain: number | null; offset: number | null; binning: number;
  roi: { x: number; y: number; width: number; height: number }; cooler_on: boolean;
  set_temp_c: number | null; forward_kib_per_s: number | null; save_at_node: boolean;
}
export interface ImagingFrame {
  id: number; state: "exposing" | "receiving" | "saved" | "failed" | "aborted";
  purpose: string; exposure_ms: number; start?: number; bytes?: number; received?: number;
  received_at?: number; file?: string; preview?: string; node_file?: string;
  node_save_error?: string; error?: string;
}
export interface ImagingView {
  state: "off" | "no_link" | "no_service" | "no_camera" | "error" | "ready";
  error?: string; camera?: ImagingCameraInfo; status?: ImagingStatus;
  settings?: ImagingSettings; frame?: ImagingFrame;
}
export type ImagingSettingsEdit = Partial<Omit<ImagingSettings, "roi">>;
export async function getImaging(): Promise<ImagingView>;
export async function putImagingSettings(edit: ImagingSettingsEdit): Promise<ImagingView>;
export async function postImagingFrame(): Promise<ImagingView>;    // fetch(url, { method: "POST" })
export async function postImagingAbort(): Promise<ImagingView>;
```

Each call throws an `Error` with the answer's `error` on a refusal, as `postTrack` does
(`crates/finder/web/src/api.ts:332`). `useImaging.ts` exports
`useImaging(): { view: ImagingView | null; refresh: () => Promise<void> }`: it reads
`getImaging()` when mounted and every 1000 ms after, ignores a failed read, and stops
when unmounted.

`App.tsx`: beside the `/setup` branch (`crates/finder/web/src/App.tsx:184`), the path
`/imaging` renders `<ImagingPage night={night} onBack={() => navigate("/")} />`, and
`TopBar` is given `imaging={session.node ? { current: path === "/imaging", onOpen: () =>
navigate("/imaging") } : undefined}`, and the frame picker's `onPick` goes home from
`/imaging` as it does from `/setup`. `useSession` does not change: the link depends on
`session.node` alone. `TopBar.tsx`: with that prop, after the Setup link
(`crates/finder/web/src/components/TopBar.tsx:111`), an `<a href="/imaging"
className="chip">Imaging</a>` with `aria-current="page"` when current and the Setup
link's click handling; without the prop, nothing.

`components/ImagingPage.tsx`, `ImagingPage({ night, onBack }: { night: boolean; onBack:
() => void })`, with these words exactly:

- A title block with the heading `Imaging` (`h1`) and the link `Back to console`.
- **Not ready**: one line and nothing else. `off` and `no_link`, a `status`:
  `The scope node is not connected.`; `no_service`, a `status`:
  `This scope node has no imaging service. Rebuild it to match this console.`;
  `no_camera`, a `status`: `This scope node has no imaging camera.`; `error`, an `alert`:
  `The imaging camera is not answering: {error}.`
- **Camera** (`section` labelled by an `h2` `Camera`): the line
  `{model} · {width} × {height} px · {pixel_um} µm · {bits}-bit · {bayer or "mono"}`, and
  a `status` line, by `settings.cooler_on` and `status`: with the cooler off,
  `Sensor {temp_c to one decimal} °C · cooler off`; with it on and a set temperature,
  `Sensor {temp} °C · cooler on at {power, whole}% · set {set_temp_c to one decimal} °C`;
  with it on and no set temperature, which is how the page finds the camera after the
  scope node restarts, `Sensor {temp} °C · cooler on at {power, whole}%`;
  with no temperature, `Sensor temperature not reported`.
- **Settings** (`h2` `Settings`): labelled fields `Exposure` (ms), `Gain`, `Offset`, a
  select `Binning` with one option a binning, worded `{b} × {b}`, the help text
  `2 × 2 is half resolution: a quarter of the bytes for each imaging frame.` when 2 is
  offered, a `role="switch"` `Cooler`, a field `Set temperature` (°C), a field
  `Forwarding limit` (KiB/s) with the help text
  `Empty for no limit. From 64 to 1048576 KiB a second.`, a checkbox
  `Also keep each imaging frame at the scope node`, and the button
  `Save imaging settings`
  (`button button-primary`). `Gain` and `Offset` are shown only when the camera has the
  range; `Cooler` and `Set temperature` only when it has a cooler. The checkbox is drawn
  in ink, not by the browser (`appearance-none`, a 1px `border-ink` box and
  `checked:bg-ink`), because a native box is gray at night. These are Tailwind
  utilities on existing tokens; `index.css` does not change. Drafts start from the
  view's settings on the first render and follow them only when those settings change,
  so they survive the one-second reads. Save sends one `putImagingSettings` whose keys
  are, in this order, `exposure_ms`, `gain`, `offset`, `binning`, `cooler_on`,
  `set_temp_c`, `forward_kib_per_s`, `save_at_node`, each only when its field is shown
  and, for `set_temp_c`, not empty; `forward_kib_per_s` is always sent, as the field's
  number or as `null` when the field is empty, so that a limit can be removed; then
  `refresh`. A refusal shows its words in an `alert`.
- **Imaging frame** (`h2` `Imaging frame`): the button `Take imaging frame`
  (`button button-primary`), disabled
  while the frame is `exposing` or `receiving`; then by state: `exposing`, a `status`
  `Exposing for {exposure_ms} ms` and the button `Abort exposure`; `receiving`, a
  `status` `Receiving {received} of {bytes} bytes` and `Abort exposure`; `saved`, a
  `status` `Saved {file}`, followed, when the frame has `start` and `received_at`, by
  ` · in hand {t} s after the exposure` with t =
  `(received_at − start − exposure_ms/1000).toFixed(1)`, then by
  ` · also at the scope node as {node_file}` when
  there is one, and with a `node_save_error` an `alert`
  `The scope node kept no copy: {node_save_error}.`; `failed`, an `alert`
  `Imaging frame failed: {error}.`; `aborted`, a `status` `Exposure aborted.` A refused
  request shows its words in an `alert`.
- **The picture**, when the frame has a `preview`: `<img alt="Imaging frame {id}"
  src={preview}>` with the class `photo` unless `held`, where
  `const { held, bind } = useColourHold(night)`. With `night`, under it, a `button`
  (`type="button"`, class `button`, `aria-pressed={held}`, `{...bind}` spread on it)
  whose text is `Hold to see colour`, and the `quiet` line
  `The picture is red at night. Hold the control to see its colours for up to 10
  seconds.` Nothing but the picture's class changes. In daylight there is no button and
  the class stays, which the night rule alone gives effect to
  (`crates/finder/web/src/index.css:118`).

`useColourHold.ts`, the control built once (spec owner decision 11; 035's stack picture
uses it too):

```ts
import type { KeyboardEvent } from "react";

/** The longest one hold shows colour. */
export const COLOUR_HOLD_MS = 10_000;

export interface ColourHoldBind {
  onPointerDown: () => void;
  onPointerUp: () => void;
  onPointerLeave: () => void;
  onPointerCancel: () => void;
  onBlur: () => void;
  onKeyDown: (event: KeyboardEvent<HTMLButtonElement>) => void;
  onKeyUp: (event: KeyboardEvent<HTMLButtonElement>) => void;
}

/** Colour while held, at night only, for at most COLOUR_HOLD_MS a hold. */
export function useColourHold(night: boolean): { held: boolean; bind: ColourHoldBind };
```

A hold begins at `onPointerDown`, or at `onKeyDown` of the key `" "` or `"Enter"` whose
`repeat` is false, when none is under way; a `keydown` whose `repeat` is true is
ignored, so a key held down, which the browser repeats, does not start the limit again.
It ends at the first of `onPointerUp`, `onPointerLeave`, `onPointerCancel`, `onBlur`,
`onKeyUp` of the key that began it, `COLOUR_HOLD_MS` after it began (one `setTimeout`,
cleared when the hold ends and on unmount), or `night` turning false. `held` is true only
while a hold is under way and `night` is true, so with `night` false the handlers change
nothing that shows.

Write each phrase a test asserts on one line of JSX, so `textContent` keeps it whole.
Only existing classes, and Tailwind utilities on existing tokens, are used; `index.css`
does not change.

`e2e/imaging.spec.ts` runs in the simulate project. It defines its own off-red counter,
written as `crates/finder/web/e2e/session.spec.ts:45` writes it, and
`playwright.config.ts` gains `"**/imaging.spec.ts"` at the end of the default project's
`testIgnore` (`crates/finder/web/playwright.config.ts:36`) and of the simulate project's
`testMatch` (`crates/finder/web/playwright.config.ts:53`). That second line is 101
characters today and would be 124, past Prettier's 120
(`crates/finder/web/.prettierrc.json:2`), so the `testMatch` list is written one entry a
line with a trailing comma, as the `testIgnore` list above it already is; `npm run
format` (`crates/finder/web/package.json:11`) writes it so, and `npm run format:check`
in the `test` gate requires it. The re-wrap is part of the approved edit (spec owner
approval 8). The screenshot taken during a
hold masks the picture with `mask` and `maskColor: "#000000"`.

### `DESIGN.md` and `.impeccable/design.json` (US7)

Four edits, with the words of spec owner approval 6: the Red Torch Rule's paragraph
(`DESIGN.md:198`), whose exception names both pictures from the imaging camera, an
imaging frame's preview and a stack's picture, so that 035 adds no rule; the first line
under Don't that names a white or non-red pixel
(`DESIGN.md:405`); in the Top bar line (`DESIGN.md:276`), `the Imaging link while a
scope node is paired,` after `the Setup link,`; and, after the Recording entry
(`DESIGN.md:385`), a plain `###` heading and one paragraph (the `>` marks only set the
text apart here):

> ### Imaging page — built
> The `/imaging` page shows the imaging camera the scope node relays: one ink line for
> the camera and one for its sensor temperature and cooler; a Settings form of labelled
> 44px ruled fields for exposure, gain, offset, binning, cooler, set temperature and
> forwarding limit, with only the controls the camera has, saved by one primary button;
> and an Imaging frame section whose primary button, Take imaging frame, becomes a
> status of the exposure and the bytes received, with Abort exposure beside it, then the
> saved file's name and how long after the exposure it was in hand. The latest imaging
> frame's preview is a full-width ruled photograph under `.photo`, so at night it is red
> by luminance like every other picture. Under it, at night only, the Hold to see colour
> button removes the filter from that one picture while it is held, for at most 10
> seconds: the Red Torch Rule's one exception. Everything else uses existing ink, paper
> and red-only night tokens.

`.impeccable/design.json` gains one entry in `components`, after `Recording`, formatted
as its neighbours: `name` `Imaging page`, `kind` `custom`, `refersTo` `button`, a
`description` that says what the `DESIGN.md` entry says, as its neighbours do, and
`html` and `css` snippets under the class `ds-imaging-page`, using only
`var(--color-ink)`. The class `ds-imaging`
is taken: it is the Imaging Camera entry's (`.impeccable/design.json:719`).

### The MCP tools (`crates/finder/src/mcp/tools.rs`, US8)

Two structs that only describe a tool's input, beside `TargetArguments`
(`crates/finder/src/mcp/tools.rs:31`) and written as it is; `SettingsEdit` parses the
arguments, so the route and the tool refuse with the same words:

```rust
#[derive(rmcp::schemars::JsonSchema)]
#[expect(dead_code, reason = "schema only; SettingsEdit parses the actual arguments")]
struct ImagingSettingsArguments {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "f64")]
    exposure_ms: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "f64")]
    gain: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "f64")]
    offset: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "u32")]
    binning: Option<u32>,
    roi: Option<ImagingRoiArguments>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    cooler_on: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "f64")]
    set_temp_c: Option<f64>,
    forward_kib_per_s: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    save_at_node: Option<bool>,
}
#[derive(rmcp::schemars::JsonSchema)]
#[expect(dead_code, reason = "schema only; SettingsEdit parses the actual arguments")]
struct ImagingRoiArguments { x: u32, y: u32, width: u32, height: u32 }
```

The seven keys that refuse null carry `#[schemars(with = …)]` with
`#[serde(default, skip_serializing_if = "Option::is_none")]`, so the schema lists them as
optional and not nullable, with no `required` list and no `default`; `roi` and
`forward_kib_per_s`, which null clears, stay plain `Option`s and nullable. Neither near
form does: `#[schemars(with = …)]` alone makes the seven `required`, and plain types with
`#[serde(default)]` publish `"default":0.0` and `"default":false`.

Three tools in the `#[tool_router]` block, each `output_schema = object_schema()`:

- `get_imaging_camera`, read-only, description
  `Read the imaging camera's state, description, status, settings and latest imaging frame.`:
  `CallToolResult::structured` of the view, or of `{"state":"off"}` without imaging.
- `set_imaging_camera`, description
  `Change the imaging camera's exposure, gain, offset, binning, region, cooler and forwarding settings, as PUT /api/imaging/settings does.`,
  with `input_schema = rmcp::handler::server::common::schema_for_input::<ImagingSettingsArguments>().expect("imaging settings schema")`,
  as `set_target` declares its own (`crates/finder/src/mcp/tools.rs:355`), so a client
  reads the nine key names from `tools/list`:
  takes `arguments: JsonObject`, parses them with
  `api::sim_json(Json::<SettingsEdit>::from_bytes(…))`, the route's own parse, so a
  parse error has the route's words, calls `Imaging::set`, and answers the view; a parse
  error or a `Refusal` is `CallToolResult::error` with the words.
- `take_imaging_frame`, description
  `Take one imaging frame, as POST /api/imaging/frame does. It waits until the imaging frame is saved and returns it with a colour JPEG preview.`:
  takes `arguments: JsonObject`; any key is the error
  `take_imaging_frame takes no arguments`. It calls `Imaging::take("frame")`, then
  `Imaging::ended(id)`, and answers from the frame `ended` returns. `saved`:
  `CallToolResult::structured` of the view with that frame as `frame`, with
  `ContentBlock::image(base64 of the preview, "image/jpeg")` pushed after the text
  block when the preview is held. `failed`: the error is the frame's own `error`,
  unchanged (for example `the exposure failed: the camera fell over`).
  `aborted`: the error `the exposure was aborted`.

Without imaging the last two answer the error `this console takes no imaging frames`.
None is a moving tool: no rate limit and no call-log line. The MCP session tools carry
`imaging` because they serialize `session_response`
(`crates/finder/src/mcp/tools.rs:347`); no other MCP code changes.

### The camera lease (`crates/finder/src/imaging/mod.rs` and `crates/finder/src/imaging/frames.rs`, US9)

The plan document's Amendment 1, name for name as 033's plan.md § The camera lease, 034's
and 035's § The camera's lease, 036's and 037's plans call it:

```rust
/// The imaging camera, held for one taker (031 US9). Dropping it releases the camera at
/// once, also when the task that owns it is aborted. It holds no strong reference to the
/// `Imaging`.
#[derive(Debug)]
pub struct Lease {
    imaging: std::sync::Weak<Imaging>,
    /// Which grant this is; the `Drop` clears the holder only while it is still this one.
    grant: u64,
}
impl Imaging {
    /// Take the camera for `holder`, a name that reads well after "held by" (`"the focus
    /// aid"`, `"the stack"`). It never waits and looks at nothing but the lease: while
    /// another holds the camera it answers `Refusal { status: StatusCode::CONFLICT,
    /// message: "the imaging camera is held by {that holder}" }`.
    pub fn lease(self: &Arc<Self>, holder: &'static str) -> Result<Lease, Refusal>;
    /// The holder's name while a lease is out.
    pub fn holder(&self) -> Option<&'static str>;
}
impl Lease {
    /// `Imaging::set` and `Imaging::take` for the holder: the same checks, refusals,
    /// words and order, less the lease's own. With the `Imaging` gone: 409
    /// `this console takes no imaging frames`.
    pub async fn set(&self, edit: SettingsEdit) -> Result<ImagingView, Refusal>;
    pub async fn take(&self, purpose: &str) -> Result<ImagingView, Refusal>;
}
impl Drop for Lease { /* upgrade; clear the holder if `grant` is the one out */ }
```

**The state.** The `Mutex<state>` of § The console's imaging state gains
`lease: Option<(u64, &'static str)>`, the grant and holder out, and `next_grant: u64`,
from 1. `lease` takes the lock once: with a lease out, the 409 above; otherwise
`next_grant` is taken and raised, `lease` set, and a `Lease` of `Arc::downgrade(self)`
and that grant answered. `holder` reads `lease`'s name. `Drop` upgrades the `Weak`; if
that succeeds and `lease` holds its grant, `lease` becomes `None`. Nothing is awaited in
any of the three, so a cancelled task's drop of its `Lease` releases the camera in the
instant the task ends. A `Lease` adds nothing to `Arc::strong_count` of the `Imaging`
(US9-S5). That scenario's second case also rests on § The console's imaging state's
"Dropping the `Imaging` ends it": the task that follows the link holds no `Arc<Imaging>`
of its own, so with no frame in flight the test's one `Arc` is the last.

**One check in two places.** `set` and `take` become thin wrappers of two private
functions that take the caller's grant, `None` for anyone and `Some(grant)` from a
`Lease`: `set_as(&self, grant: Option<u64>, edit)` and `take_as(&self, grant:
Option<u64>, purpose)`. Each makes its checks under the one lock acquisition that today
holds the state check and the frame-in-flight check, in this order: (1) not `ready`, the
state's words; (2) `lease` is `Some((g, h))` and the caller's grant is not `Some(g)`,
409 `the imaging camera is held by {h}`; (3) a frame `exposing` or `receiving`, the
frame-in-flight words of `set` or `take`; then the rest as today. `take`'s checks are
made in `frames.rs` (§ Receiving and saving, **`take`**), so US9 writes that file too:
`frames::take` takes the lease's check as its last parameter,
`held: impl FnOnce(&State) -> Result<(), Refusal>`, `State` being what the state's
`Mutex` guards, and calls it between the state's check and the frame-in-flight check; `take_as` passes check (2), for
its own grant, as that parameter. `set_as` makes its three checks in `mod.rs`.
`Lease::set` and `Lease::take` upgrade the `Weak` (failing as above) and call
`set_as(Some(self.grant), …)` and `take_as(Some(self.grant), …)`. So the routes and the
MCP tools, which call `Imaging::set` and `Imaging::take`, refuse with the holder's words
with no change to `routes.rs` or `tools.rs`.

**Open to anyone.** `abort`, `ended`, `view`, `preview` and the task that follows the
link look at no lease. A frame started by `take_as` holds no lease and is not ended when
a lease is released: once started it belongs to the `Imaging`'s frame task, as every
frame does (§ Receiving and saving).

**Not here.** A lease does not look at the state or at a frame in flight, does not wait,
is not lent from one holder to another, and is not shown in the view. A later spec that
wants either a waiting lease or the holder on the page adds it with an approval of its
own.

### The README

`crates/finder/README.md`: US1 adds `## Imaging frames` at the end, the two rows after
the `POST /api/record/raw` row (`crates/finder/README.md:201`) and `--imaging-dir`; US2
the `POST /api/imaging/frame` row, the `frame` object, the path and the cards, and how
to read the time a frame took to reach the console (`received_at − start −
exposure_ms/1000`, and that less the scope node's probe figure for the link alone); US5
the `POST /api/imaging/abort` row and the `frame` object's `node_file` and
`node_save_error`; US6 the preview row and how it is made; US7 one sentence in
`## Night mode` (`crates/finder/README.md:126`) on the held colour, which spec owner
approval 9 names; US8 one sentence and a three-item list in `## MCP clients`, after its
paragraph on Stop and before `### MCP under WSL2`, one item a tool: what it takes and
answers, and that `set_imaging_camera` and `take_imaging_frame` answer
`this console takes no imaging frames` at once on a console that takes none; US9 one
paragraph at the end of `## Imaging frames`, with these
words: "**One holder.** A part of the console that takes imaging frames one after
another holds the imaging camera while it runs. Meanwhile `PUT /api/imaging/settings`
and `POST /api/imaging/frame` answer 409, and `set_imaging_camera` and
`take_imaging_frame` a tool error, with `the imaging camera is held by` and the holder's
name. `GET /api/imaging`, the previews,
`POST /api/imaging/abort` and `get_imaging_camera` stay open to anyone, so an exposure
can always be stopped. A single imaging frame from the page or an MCP client holds no
lease: while it is in flight a second one is refused with
`an exposure is already in progress`, as before." Rows are worded like their neighbours
and end `Same-origin writes only.` where that applies.

## Shapes that existing tests pin

These are not changed by any story:

- **Everything 030 built.** Its service, its port, its stand-in cameras, the simulator's
  imaging camera and `SimNode` are read and never written: no story here writes a file
  under `crates/scope-node`, `crates/scope-link` or `crates/simulator`. 030's own tests
  pin them.
- **`LinkManager::start`'s and `start_with_filter`'s signatures, `MountClient`,
  `RawClient` and the link's one connection for everything that exists today.** A
  console built without `with_imaging` never opens a second connection, so every
  existing in-process test sees the link it saw.
- **`GET /api/session`, and the MCP session bodies, on a console built without
  `with_imaging`**: `imaging` is skipped when absent, so those bodies are byte for byte
  what they were. The binary's session gains `imaging`; its tests read the session key
  by key (`crates/finder/tests/cli/recording.rs` reads `recording`), and the page's
  `useSession` copies known fields only (`crates/finder/web/src/useSession.ts:30`).
- **The MCP tool list as tests read it.** They look tools up by name; one calls every
  listed tool with `{}` and checks for the token (`crates/finder/tests/mcp/motion.rs:528`),
  so each new tool must answer `{}` at once and without the token on a console with no
  imaging (US8-S5, US8-S6).
- **The binary's start-up lines.** `crates/finder/tests/cli/simulate.rs` reads the
  address line `finder console: http://`; nothing new is printed.
- **Finder's test layout** (`crates/finder/tests/layout/main.rs:102`): no file at the top
  level of `crates/finder/tests`, no new file in `support`, and each binary's `main.rs`
  holding only its support include and `mod` lines
  (`crates/finder/tests/layout/main.rs:156`).
- **The top bar without a scope node, and `TopBar` without the new prop.** Component
  tests render it without `imaging`; the default and session Playwright projects never
  pair a node, so their bar is today's. The simulate project's bar gains one chip.
- **The Setup page's one `status` and one `alert`**
  (`crates/finder/web/src/components/SetupPage.test.tsx:37`): the Imaging page is a
  different route.
- **Shapes this spec pins for its own later stories and for 032 to 037.** `ImagingView`,
  `SettingsView` and `FrameView` are `#[non_exhaustive]` and tests compare `settings` and
  `frame` key by key; tests build scope-link messages with `..Default::default()`. Two
  more, which 032 to 037 will meet. From US4 on a console test implements
  `ScopeNodeImaging` by hand (US4-S1, US4-S2), so a new call on that service stops an
  existing test compiling, which is 030's plan trap 7 again for the new service: a later
  spec's calls go in a new service of its own. A test's own camera builds an
  `ImagingFrame`, so a field added to it later breaks every such literal that does not
  end with `..Default::default()`, which the shared setup requires. `Imaging::take`
  always saves a file: 033's focus aid and 037's keep-subs switch save each frame and
  remove the file once the frame has ended, so no path without a file is needed; 035
  adds only a folder that a lease may name for its frames (§ Receiving and saving, **A
  stack's subs are saved elsewhere**). From
  US9 the lease's names and its refusal are pinned by US9's tests, which is what 033 to
  037 call. Files once written are pinned too: a card added later appears only in later
  files.

## Decisions and the alternatives rejected

| Decision | Rejected | Why |
|---|---|---|
| A second connection, for `ScopeNodeImaging` alone | Sharing the link's one connection | A frame may take many seconds to cross; the heartbeat's 1 s limit stops the mount. A separate connection has its own flow control and cannot delay a heartbeat (030 makes the service; this trio opens the connection). |
| The link manager opens the second connection from its own pairing | The imaging module reading the session's pairing | Under `--simulate` the pairing is in a private session; the manager already holds the right one and the transport. |
| `forward_kib_per_s` is a console setting, sent with each exposure | A node config key | The observer tunes it at the telescope from the Imaging page's `Forwarding limit` field, or an MCP client does, with no shell on the node. |
| The console's own frame ids | The node's | The node's restart at 1; a file must never be replaced. |
| A hand-written FITS writer and reader | A FITS crate | One image, a dozen cards and big-endian samples need no library; `fitsio` wraps C and the pure-Rust writers are young. |
| `IMAGETYP` `LIGHT`, `DARK`, `FLAT` | The two-word values some programs write | Both forms are read by the programs that will open these files. `LIGHT` is a named exception to the glossary's "avoid light": it is the FITS convention's value in that card, not this project's word for a sub. |
| The console's file takes a frame's region from the frame's header, and the console stamps the set temperature and serial | The console stamping the region from its settings, with nothing on the scope link (the review's draft); leaving the region to 033 | The plan document's Amendment 6 puts the region in the header and the file. The node knows the region the camera really used, so a camera changed by another INDI client is stamped right. The set temperature and serial stay the console's (spec owner decision 16, whose open part this is). |
| One lease on the console's imaging state, granted by name, never waiting, released on drop, checked by `set` and `take` after the state and before a frame in flight | A lock in each later spec; a lease that waits; a single frame that takes the lease | The plan document's Amendment 1. Checked inside `set` and `take`, it guards the routes and the MCP tools without touching them; a holder that heard the frame-in-flight words could not tell who to wait for; a single frame holding no lease keeps the page's one-frame flow as it was, and each holder looks for a frame in flight itself (spec US9). |
| The hold-to-see-colour control as one hook, `useColourHold` | The control written into each page block | 035's stack picture needs the same control and the same exception; one hook keeps one behaviour and one set of release events (spec owner decision 11). |
| An existing file name fails the frame | Trusting the count alone; a hard link | A rename replaces without a word; a check costs one call; hard links are missing on some file systems. |
| `decode` in this spec, tested by round trip and worked bytes | Leaving it to 032 | The plan document puts reading in `fits.rs`, and code with no test is not allowed, so it has a scenario (US3-S1). |
| `received_at` and `saved_at` on the frame, and the time in hand on the page | Timing the hop with the probe | The probe runs on the scope node alone and cannot see the link; the console can, and the plan document lists the forward time as a field check. |
| Previews in memory, the last eight | Previews on disk | The FITS file is the record; a preview is remade by any viewer. |
| The page reads `GET /api/imaging` once a second | Server-sent events | One more poll is simpler than a second stream and fast enough for an exposure's progress. |
| The Imaging link only while a node is paired | Always | A console at a desk keeps today's bar, and three Playwright projects' existing checks are undisturbed in two. |
| Colour only while held, at night only, 10 s at most | A switch | A switch left on would undo dark adaptation; the exception must end by itself. |
| `take_imaging_frame` waits for the frame | Answering with an id to poll | An MCP client asked for a picture; the wait is bounded by the console's own limits. |

## Verification (every story, before proposing)

- **The factory gates** (`ergane.yaml` at 49bb210, unchanged since dbc2b5b):
  - `lint` = `cargo fmt --all --check`,
    `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`,
    `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features --locked`
    and `RUSTDOCFLAGS="-D warnings" cargo doc -p finder --no-deps`;
  - `test` = `cargo nextest run --workspace --all-targets --all-features --locked`,
    `cargo test --workspace --locked --doc`, and then, in `crates/finder/web`,
    `npm ci --no-audit --no-fund`, `npm run typecheck`, `npm run format:check`,
    `npm test`, `npm run build`, `git status --porcelain -- dist | (! grep .)` and
    `CI=1 npm run e2e`.

  Every story runs both, and the judge sees their results. `--locked` holds as it
  stands: no story of this trio changes `Cargo.lock`.
- **CI in the merge queue** runs every `ci.yml` job, including `cargo-deny`,
  `cargo-machete`, the build on Rust 1.96 and the finder-page checks. No gate here runs
  them and no scenario's Then clause rests on them. This trio adds no dependency for
  `cargo-deny` or `cargo-machete` to judge.

## Prerequisites before this spec is flipped to `ready`

1. **The earlier specs have landed.** `025-frame-capture`: met (US3 at 820c558).
   `028-finder-tests-do-less`: met on `main` at 49bb210, where all six of its stories
   have landed (US2 at 1928771, US1 at 54e69f4, US3 at 64c3862, US4 at 6bd5a28, US6 at
   dbc2b5b, US5 at 49bb210). `030-imaging-camera-link`: met on `main` at 49f8dcd, where
   all eight of its stories have landed (US1 at 3849a0f, US2 at e867598, US3 at 209edcb,
   US4 at 2ed589c, US5 at e1cc38e, US6 at 58aff47, US7 at 4e7eef6, US8 at 49f8dcd). The
   worktree is at 49f8dcd.
2. **The trio is re-anchored to `main`.** Met at 49f8dcd, after 030 landed. Every
   `path:line` in spec.md, plan.md and tasks.md, 68 in all, was compared line for line
   at 49bb210 and at 49f8dcd. Forty-two read the same. Twenty-five had moved and are
   corrected to the line that now holds the same text, among them the top bar in
   `DESIGN.md:276` (was 274), `postTrack` in `crates/finder/web/src/api.ts:332` (was 312)
   and every cited line of `crates/finder/src/api.rs`, `link.rs` and `session.rs`. One
   no longer reads as cited: `max_concurrent_epics` is now at `ergane.yaml:196` and reads
   4, not 2, and spec.md's Clarification says so. No line is cited in a file 030 created.
   Every name of § What this trio takes from 030 was read on `main`: the five calls, the
   ten messages with the fields and optionality written there, the refusals and their words,
   `SimImagingCamera`, `SimStar`, `SyntheticImagingCamera`, `NodeService::with_imaging`
   and `with_imaging_save_dir`, the `ImagingCamera` port and `Exposure`. Each was built
   as written, so that section and spec.md's shared setup needed no correction beyond
   saying so. The two places round 1 found saying more than this trio did hold on
   `main`: `start` calls `start_with_filter`, and `run` takes eight parameters. The
   validator reports no refusal there.
3. **The owner decisions are settled and the approvals given** (spec § Owner decisions,
   § Owner approvals). Met on 2026-11-17. Of the sixteen decisions, 3 and 4 are settled
   by the plan document's Amendment 3, 8 by Amendment 5 and the region of 16 by
   Amendment 6; the other twelve, with the open part of 16 (the set temperature and
   serial), take their recommended answers, which the owner approved that day with the
   imaging series' other decisions ("im good with all of the info in the decisions
   doc"). None of the three rows the owner settled separately that day is this spec's
   (they are 034's, 035's and 037's). Decisions 2, 8, 9 and 16 each have a counterpart in
   030 (its owner decisions 7, 8, 9 and 12), settled the same way with it. The owner gave
   the approvals by flipping the spec, on 2026-11-17.
4. **The trio is probe-built.** Met on 2026-11-17, round 1, every story: each was built
   from this trio in a throwaway worktree, on top of 030's round-1 probe builds, tests
   first, with both gates, `cargo deny check` and `cargo machete`; each diff was
   measured as plan trap 3 says; and what the probes found is folded back in
   (§ Refinement Record, round 1). The draft's second story measured over the budget,
   and it is split in four as measured, US2 to US5.
5. **The documents this trio cites are on `main`.** Met: the plan document, ADR-0002,
   the changed `GLOSSARY.md` and the research note landed with 030's flip (55e41f5),
   none of them by a story. The two changes this trio asked of them are in: `GLOSSARY.md`
   carries the widened **Preview** that spec owner approval 7 words; and the research
   note's source list holds the SBIG FITS extension document and MaxIm DL's FITS header
   definitions (its source 47), which spec.md's Clarifications cite for `XORGSUBF`,
   `YORGSUBF` and `SET-TEMP`.
6. **No constitution change is needed for this spec.** Met. Principle V's "the solver is
   our own Rust" is not touched here; the imaging solver is 032's.
7. **Outside the factory, before the first use at the telescope:** the scope node
   rebuilt and probed as 030's plan.md § Prerequisites 8 says, and the console rebuilt
   from `main` after US9. Not needed to flip.
8. **Other drafts that write the same files are ordered.** Met for the drafts read here.
   `026-mount-sky`, `021-chart-orientation` and `022-altaz-guidance`, whose Refinement
   Records write files this spec writes (spec.md's Clarifications list them: nine, six
   and four files), have all landed (026 at 6cc8724, 021 at 2f6a777, 022 at 20ce2fe), and
   this trio is re-anchored on top of them (Prerequisite 2). The rule stays for any later
   draft: before either of two such specs is flipped, the one flipped second adds the
   other to its `depends_on_landed` and is re-anchored after the other lands. Whether a
   draft numbered 023, or any other, exists outside the branches read here was still open
   at the flip; one that does is flipped after this spec, by the same rule.
9. **The plan document agrees with the settled decisions.** Met on this side of the
   link. The plan document's Amendment 3 settles the one departure the review found
   here (owner decisions 3 and 4), and Amendments 1 and 6 are carried here (US9, owner
   decision 16). § Interfaces shared across the specs now names the file
   `crates/finder/src/imaging/routes.rs` in its module list and holds the cards this
   trio adds in its list of an imaging frame's FITS cards; the `frame` keys this trio
   adds are an addition the amendments' preamble covers. It now also carries 030's
   `save` on `ExposeRequest` (its `Expose` row's "whether the node also keeps the
   frame") and `[imaging] save_dir`, the departure 030's plan.md § Prerequisites 9
   lists; whether the scope node keeps a copy at all is still 030's open owner decision
   9, and when the owner settles it, this trio's `save_at_node` follows (owner decision
   9).
10. **The trios that call the lease use its names.** Met for the names: US9 has
    `Lease`, `Imaging::lease`, `Imaging::holder`, `Lease::set`, `Lease::take` and the
    refusal `the imaging camera is held by {holder}` as 033's plan.md § The camera lease
    writes them down, and its six scenarios are the six rules there. Each of 033 to 037
    is re-anchored to US9 after this spec lands. 032's Refinement Record names US9 as
    the last writer of `crates/finder/src/imaging/mod.rs`,
    `crates/finder/tests/console/main.rs` and `crates/finder/README.md`, and 035's names
    it for `crates/finder/src/imaging/frames.rs`. Round 1's renumbering changed those
    records, and every other citation of this spec's stories and scenarios in 032 to
    037, to the nine-story numbers; the re-anchoring itself is their trios' work, not
    this one's.

## Plan traps

Lessons from building 012 to 028 on this factory, and the traps this design adds, each
with the check that keeps it from recurring. They bind every story.

1. **Gate evidence or nothing.** The judge credits only gate results and the story's
   diff. *Check:* every Then clause names a committed test that the `test` gate runs.
2. **A named command is read literally.** *Check:* commands are copied from
   § Verification.
3. **One evidence budget.** *Check:* each story keeps its diff under 64 KiB, measured
   with `git diff <base> -- . ':(exclude)crates/finder/web/dist' ':(exclude)Cargo.lock' | wc -c`.
   A story that cannot escalates; it does not drop a scenario.
4. **The sandbox has only the gates' tools.** *Check:* no task runs `cargo deny`,
   `cargo machete` or anything else the gates do not.
5. **A spec fixed mid-epic needs a relaunch.** *Check:* no builder edits
   `specs/031-imaging-frames-on-console/` or `.specify/`.
6. **An oracle copied from the code proves nothing.** *Check:* tests compute the sample
   rule themselves, and read saved FITS files with their own code and never through
   `finder::imaging::fits`. US3-S1 tests `encode` and `decode` themselves, against
   bytes the scenario works out, not against each other alone.
7. **Real sockets and paused time do not mix.** With time paused, tokio jumps to the
   next timer whenever every task is idle, so a timeout fires while a real socket is
   still delivering. *Check:* every in-process test runs over in-memory streams; only
   the two `cli` modules use real sockets, in real time, with generous limits.
8. **Sidereal drift is large at this scale.** At 2.18″ a pixel an untracked star moves
   7 px a second. *Check:* US6-S4, the one scenario here that places a star, uses a
   20 ms exposure and puts the star at the truth for mid-exposure.
9. **Whole-object comparisons break at the next story.** *Check:* tests compare
   `settings` and `frame` key by key, and build link messages with
   `..Default::default()`.
10. **A JSON null read as "absent".** serde's `Option` takes null silently. *Check:*
    `SettingsEdit`'s plain options refuse null (US1-S3), and its two double options
    tell null from absent (US1-S2).
11. **The clock offset's sign.** `offset_s` is node time less console time. *Check:*
    `start = start_s − offset_s`; US2-S1 compares `frame.start` with the console clock
    read just before the request, to a millisecond.
12. **A pacing rule that counts the chunk it is about to send.** The scope node paces
    (030): the wait before chunk k is the bytes already sent over the rate, so the last
    of eight chunks leaves when seven have gone. *Check:* US2-S4 gives the worked time
    at the console, 28 s.
13. **A FITS card one column out.** Readers are strict about `= ` in columns 9 and 10
    and numbers ending in column 30. *Check:* US2-S1 reads the file with the test's own
    fixed-column reader, US3-S1 compares a whole real's card as text, and § FITS at the
    console gives the worked bytes.
14. **The bundle must be rebuilt.** *Check:* US7 commits `crates/finder/web/dist` after
    `npm run build`, so that the gate's `git status --porcelain -- dist` finds nothing.
15. **027's layout.** *Check:* each new finder test is a module file in its binary's
    folder with one alphabetical `mod` line, binding helpers with
    `use crate::support::live as support;` and holding only the setup its tests use.
16. **Each worktree builds on its own, and scratch files go where cargo says.** *Check:*
    no story sets `CARGO_TARGET_DIR`; every file or folder a test makes is inside a
    `crate::support::Scratch`; and no test builds a path from `CARGO_MANIFEST_DIR` and
    `../../target`, which 028 removed.
17. **A masked screenshot is pink.** Playwright paints masks magenta by default, which
    the off-red counter would count. *Check:* US7-S8's mask colour is black.
18. **A tool that hangs on `{}`.** An existing test calls every listed tool with empty
    arguments. *Check:* without imaging the new tools answer at once (US8-S5).
19. **The link's status changes four times a second.** `LinkStatus` holds the heartbeat
    count, so "wait until the status changes" returns at the next heartbeat, and a retry
    or a call raced against it would be cut short every 250 ms. *Check:* the imaging
    task waits on `connected` alone, with `wait_for`; US1-S10 asserts a retry period of
    exactly 5 s while heartbeats are answered.
20. **A rename replaces.** *Check:* the final name is tested before the rename, and an
    existing file fails the frame and is left as it is (US2-S3).
21. **A long list that Prettier re-wraps.** *Check:* US7 writes the simulate project's
    `testMatch` list one entry a line, which spec owner approval 8 names.
22. **A lease checked in the wrong place.** Checked after the frame-in-flight test, a
    second taker hears `an exposure is already in progress` and cannot tell who to wait
    for; checked in the routes, the MCP tools and later specs' own callers slip past it.
    *Check:* the check is inside `set_as` and `take_as`, between the state's and the
    frame's, under the same lock, which for `take` is taken in `frames.rs`; US9-S2
    asserts the holder's words both before and during the holder's frame, through
    `Imaging`, the routes and the MCP door, and the state's words first on a console
    with no link.
23. **A lease that outlives its holder or keeps the `Imaging` alive.** An `Arc` in the
    `Lease`, or a release that awaits, leaves the camera held after a task is cancelled.
    *Check:* `Lease` holds a `Weak` and a grant, its `Drop` awaits nothing, and US9-S5
    aborts a holder's task and reads `holder()` in the same virtual instant.
24. **A region from the settings, not the camera.** Stamping a file from what the
    console asked for writes the wrong origin when another INDI client moved the region.
    *Check:* `XORGSUBF` and `YORGSUBF` come from the frame's `roi` as it stands when it
    is saved, the header's; US4-S2's node sends a region the settings never held, and
    the file carries it.

## Risks

| Risk | Mitigation |
|---|---|
| An interface may not build as written: prost's names for 030's messages, as § What this trio takes from 030 writes them; a second connection made from the link manager's kept transport; a `Weak<Imaging>` in a `Lease` whose `Drop` takes the state's lock. | Proved in round 1 (Prerequisite 4), on 030's round-1 probe code: every interface built as plan.md writes it, with private additions only; US1-S5 counts exactly two connections; and in US9-S5 the `Lease` adds nothing to the strong count and is released in the instant its task is aborted, with no deadlock, since no task holds an `Arc<Imaging>`. Prerequisite 2: every name taken from 030 is read again on `main` after it lands. |
| A story passes 64 KiB. | Measured in round 1 (§ Refinement Record): US1 64,223 bytes, US6 26,407, US7 52,002, US8 19,580 and US9 23,788 with its write of `frames.rs`. The draft's second story measured 85,621 bytes, and the split prepared here before the probes (`decode` and the failure cases out) still left 71,496, so it is split in four where its measured code divides: US2 takes and saves a frame, about 57 KB estimated from the measured blocks; US3 is `decode`, 10,800 bytes measured; US4 the checks and US4-S2, about 16.5 KB; and US5 the refusals, abort and the node's copy, about 9 KB. The three estimates are measured when those stories are next built. US1 is at 98 % of the budget: if a later build passes it, US1-S9 and US1-S10, with the test's own camera, about 5.5 KB, become a story of their own, a spec change made before US1 is dispatched again. US7's Playwright split is not needed. |
| The second connection doubles what a poor link must carry at the worst moment. | The rate limit (US2-S4); the heartbeat is on its own connection and proven unaffected on the simulator (in round 1, 113 heartbeats answered while a frame crossed for 28 s of virtual time, the link connected at every read); the real link is a field check. |
| The new top-bar chip crowds the bar on a phone in the simulate project's existing checks. | Not seen in round 1: every simulate check passed with the Imaging chip, at the phone viewport too. The chip shows only with a paired node; if a later check fails, the chip's label hides below `sm` as the other chips' do, a change to § The page before the flip. |
| The simulate console now takes a second connection and describes an imaging camera at start; an existing simulate check may notice. | Not seen in round 1: every Rust test passed from US1 on, and the whole `CI=1 npm run e2e` passed with the second connection and the page unchanged, and again after US7 and US9. The session's `imaging` key is not read by the page before US7. |
| The preview's stretch looks poor on real frames. | It is a preview; owner decision 7 puts its look outside the gates, and 035 owns the stack's stretch. On the simulated field in round 1 the sky went to black with a little colour noise. Which percentile the noise is read from, and the JPEG's quality, are caught by no test for the same reason. |
| The hold control fails to release on some phone browser. | Five release events and the 10 s limit (US7-S5); a check on a real phone is outside the factory. |
| Specs 032 to 037 need a field this spec's types lack. | Views are non-exhaustive and tests compare key by key. A field they need on the scope link or on `ImagingFrame` is 030's to add, and changes the scope link, which the later spec must name for approval. |
| `main` moves again before the flip and shifts a cited line, or 030 lands differently from its plan.md. | Prerequisite 2: the line-for-line comparison is run again against the new `main`, as it was for 49bb210, and the names taken from 030 are read again there. |
| The view's `settings.roi` lags a region changed by another INDI client until the console's next settings read. | Files and `frame.roi` take the region from the frame's header, so a frame is placed right whatever the view shows (spec owner decision 16). |
| A later spec's holder forgets to drop its `Lease`, and the camera stays held. | The lease is a value: it is dropped when its owner returns or is cancelled (US9-S5). `holder()` names it, and the refusal says who holds the camera. |
| `026-mount-sky`, `021-chart-orientation` and `022-altaz-guidance` are drafts that write files this spec writes. | Prerequisite 8: two such specs are never flipped together; the second names the first in `depends_on_landed` and is re-anchored. |
| 030's owner decision 9 is still open: its other answer drops the scope node's copy, `save` on `ExposeRequest`, which the plan document now carries. | Prerequisite 9; this trio's `save_at_node`, US5-S3 and US5-S4 follow that decision (owner decision 9). |

## Refinement Record

Drafted on 2026-11-15 by the grilling session against `main` at dbc2b5b, from
`docs/plans/2026-11-15-specs-030-037-imaging.md` and the surveys of the camera path and
the console made that morning, as the console's half of the trio
`030-imaging-camera-link`. **No probe build had been run then.** No story of this trio
had been compiled, linted or tested, no interface above built and no diff measured: the
session wrote documents only. Every `path:line` was read at dbc2b5b,
re-read at 49bb210 and checked by the validator there; every number in a scenario was
worked by hand and is shown as a worked example where it is not obvious.

A second agent reviewed the draft the same day, when this trio and 030 were one. Its
twenty-eight findings were each checked against the files, the code at dbc2b5b and
49bb210, and the research, and applied, still with no probe build. Those on this side
of the link: scratch folders put where 028 put them; three other drafts named;
behaviour tested in the story that builds it (what the console shows of the node's "no
save folder" answer, `decode`, an 8-bit frame, the header wait, the 256 MiB limit, the
502 answers, the 5 s retry); a frame's region, set temperature and serial stamped into
its file; the hop to the console made measurable; and the departure from the plan
document written as owner decisions.

An auditor then compared the trio with 032 to 037, and its findings were applied the
same day, still with no probe build. Those on this side: US9 gives the console's imaging
state the one holder of the plan document's Amendment 1, under the names the later
trios call; US1 defines `RoiEdit`, which `SettingsEdit` named; the region of interest
comes with the frame's header and is written into the file (Amendment 6); the view's
frame carries its stamped set temperature; the hold-to-see-colour control is one hook;
and the departure an amendment settles is marked so. The worktree moved to 49bb210
meanwhile: every citation was compared at dbc2b5b and 49bb210 and the two that moved
are corrected (§ Prerequisites 2), so the record below is reviewed at 49bb210.

The trio was then split in two at the owner's ask, the same day and still with no probe
build. The scope node's stories stayed in 030, and the console's came here and were
renumbered from 1: the six stories they then made were the sixth, seventh, eighth,
tenth, eleventh and thirteenth stories of the one trio, and their requirements are
FR-001 to FR-034 in story order. No interface, scenario value or decision changed. One
scenario came from a story that stayed: the console's reading of the scope node's own
copy, now US5-S4, was the fifth scenario of that story (030's US7 now), and its test
moved from a module of its own into `imaging_frames`, and in round 1 into
`imaging_control`. Every edge to a story of 030 became this trio's `depends_on_landed`.

**Round 1 (probe builds), 2026-11-17.** Every story was built from this trio, tests
first, by three probe agents in throwaway worktrees, each story on the commit of the one
before it and nothing pushed. Their base was 030's round-1 probe code
(`probe/030-r1-c` at d727848) on `main` at 6cc8724, where 021, 022 and 026 had landed,
with this trio (`probe/031-r1-base` at 76da6ef); clippy and the workspace's 734 tests
passed there. Each story passed the `lint` gate, the `test` gate's cargo commands,
`cargo deny check` (with the base's findings) and `cargo machete`, and the `test` gate's
page commands ran on the page's story and on the last story's head; each new test
failed before the story's change and passed after; and each diff, measured as plan
trap 3 says, changed the story's declared writes and no other file, but for the
lease's. By the draft's numbers:

- US1: `probe/031-r1-a` at 9786619; 10 tests; 64,223 bytes; 11 files.
- US2: `probe/031-r1-a` at 9c622b9; 12 tests; 85,621 bytes; 10 files.
- US3, now US6: `probe/031-r1-b` at ac835bf; 4 tests; 26,407 bytes; 8 files.
- US4, now US7: `probe/031-r1-b` at 3b861a6; 11 Vitest and 2 Playwright tests; 52,002
  bytes; 12 files and `dist`.
- US5, now US8: `probe/031-r1-c` at b7f6359; 6 tests; 19,580 bytes; 4 files.
- US6, now US9: `probe/031-r1-c` at b09f0e0; 6 tests; 23,788 bytes; 5 files, of which
  `crates/finder/src/imaging/frames.rs` was not declared and was written on a commit of
  its own, c57c027.

The draft's US2 passed its 12 tests once its header wait, now US4-S1's, was given more
than the shared setup's 60 s, but its 10 declared writes measured 85,621 bytes. The
split prepared in § Risks was then built from US1's commit, on `probe/031-r1-a-split`,
tree for tree the same as the whole story: without `decode` and the failure cases it
measured 71,496 bytes (9bcf631), `decode` 10,800 (5eea930) and the failure cases
10,568 (443cd67). So it is split in four where those blocks divide: US2 takes and saves
a frame; US3 is `decode`, as measured; US4 the failure cases, as measured, with US4-S2,
whose node is the same test's own `ScopeNodeImaging`; and US5 the refusals, abort and
the node's copy. § Risks gives the estimated size of US2, US4 and US5, which are
measured when they are next built. The lease's check could be made under `take`'s lock
only in `frames.rs`, which US9 now writes. The workspace ran 772 tests after the lease's
story, Vitest 343 and Playwright 31. The probes reported forty-one defects, three
blocking: the split, the 60 s cap on a wait, and the lease's undeclared write. Their
fixes are in this trio: they settle choices the plan left open, correct wording a
builder could misread, and add the assertions that the probes' bite checks showed
missing (spec.md § Clarifications, the round-1 session). Every 030 interface this trio
uses was as 030's trio gives it.

The record below is round 1's account: its tests are the scenarios the probes built and
ran, with the draft's US2 regrouped as the split groups it, each new story's tests in a
module of its own; its writes are the files their diffs changed, US9's `frames.rs`
among them, divided for US2 to US5 by the parts of § Receiving and saving and § FITS at
the console each builds; and its shared writes name the merge edge that orders two
stories of this spec on one file, or the story of 025 or 028 that last wrote the file
on `main`. No file here is written by 030, so no
shared write names it. A file last written by an older landed spec (013, 014, 016, 017,
020, 024 or 027, each of which 028 already depends on) has no entry, because the
validator refuses a shared write that names a spec outside this spec's
`depends_on_landed`. The record's `open_questions` are empty for the same kind of
reason: the validator refuses a record that holds any. What is not established is
written where it can stay until it is settled: § Risks, and § Prerequisites 2. Nothing
in this record is an owner approval: those are in spec.md, and flipping to
`ready` gives them.

```yaml
version: 1
reviewed_revision: 49bb210fe4860dd98ba24727074e7bd3cc0d7124
stories:
  US1:
    tests:
      - path: crates/finder/tests/console/imaging.rs
        status: new
        disposition: add
        assertion: "GET /api/imaging and the session's imaging show the simulated camera, its status and the settings in force exactly"
        decision: "spec.md:US1-S1"
      - path: crates/finder/tests/console/imaging.rs
        status: new
        disposition: add
        assertion: "PUT /api/imaging/settings changes gain, binning, exposure, region, forwarding rate and the node copy, and the camera agrees"
        decision: "spec.md:US1-S2"
      - path: crates/finder/tests/console/imaging.rs
        status: new
        disposition: add
        assertion: "refused values, nulls, unknown keys, a region RoiEdit does not read and a body that is not JSON answer 400, a foreign origin 403 and another content type 415, and nothing changes"
        decision: "spec.md:US1-S3"
      - path: crates/finder/tests/console/imaging.rs
        status: new
        disposition: add
        assertion: "an older node, a node without a camera, no link, a failing camera and a console without imaging each show their state and refuse with their text"
        decision: "spec.md:US1-S4"
      - path: crates/finder/tests/console/imaging.rs
        status: new
        disposition: add
        assertion: "once star frames arrive, the console has opened exactly two connections and the scope link stays connected, and after 10 s more still two"
        decision: "spec.md:US1-S5"
      - path: crates/finder/tests/console/imaging.rs
        status: new
        disposition: add
        assertion: "a link drop shows no_link within 3 s and ready again within 15 s with the camera's settings kept"
        decision: "spec.md:US1-S6"
      - path: crates/finder/tests/cli/imaging.rs
        status: new
        disposition: add
        assertion: "the built finder under --simulate becomes ready with the simulated imaging camera, and takes --imaging-dir"
        decision: "spec.md:US1-S7"
      - path: crates/finder/tests/console/imaging.rs
        status: new
        disposition: add
        assertion: "switching the cooler on shows the simulated cooler's temperature and power within 5 s"
        decision: "spec.md:US1-S8"
      - path: crates/finder/tests/console/imaging.rs
        status: new
        disposition: add
        assertion: "a camera that refuses a setting answers 502 with the node's words, one that never answers 502 after exactly 30 s, and the settings stay as they were"
        decision: "spec.md:US1-S9"
      - path: crates/finder/tests/console/imaging.rs
        status: new
        disposition: add
        assertion: "a camera that cannot be described is asked again exactly every 5 s through answered heartbeats, and a describe that never answers is an error after 5 s with its text"
        decision: "spec.md:US1-S10"
      - path: crates/finder/tests/console/link.rs
        status: existing
        disposition: preserve
        assertion: "LinkManager::start, the link's frames, status and one connection are unchanged without imaging"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/console/session/edits.rs
        status: existing
        disposition: preserve
        assertion: "GET /api/session bodies without imaging are unchanged"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/mcp/endpoint.rs
        status: existing
        disposition: preserve
        assertion: "get_session over MCP returns the session body unchanged when the console has no imaging"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/cli/simulate.rs
        status: existing
        disposition: preserve
        assertion: "the binary's refusals and address line are unchanged"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and each binary's main.rs holds only its support include and mod lines"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "a console built without with_imaging opens one connection and serves the same session (plan.md Shapes that existing tests pin)"
      - "nothing about imaging is persisted in session.json"
      - "LinkManager::start's and start_with_filter's signatures and run's eight parameters are unchanged"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console imaging::"
        - "cargo nextest run -p finder --locked --test cli imaging::"
        - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs, and a rebuild of the console"
    writes:
      - crates/finder/README.md
      - crates/finder/src/api.rs
      - crates/finder/src/imaging/mod.rs
      - crates/finder/src/imaging/routes.rs
      - crates/finder/src/lib.rs
      - crates/finder/src/link.rs
      - crates/finder/src/main.rs
      - crates/finder/tests/cli/imaging.rs
      - crates/finder/tests/cli/main.rs
      - crates/finder/tests/console/imaging.rs
      - crates/finder/tests/console/main.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/README.md
        other_spec: 025-frame-capture
        other_story: US2
        disposition: merge_order
        reason: "031 depends_on_landed 025, whose US2 last changed the README"
      - path: crates/finder/README.md
        other_spec: 031-imaging-frames-on-console
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and adds its row and paragraphs"
      - path: crates/finder/src/api.rs
        other_spec: 025-frame-capture
        other_story: US2
        disposition: merge_order
        reason: "031 depends_on_landed 025, whose US2 last changed api.rs"
      - path: crates/finder/src/lib.rs
        other_spec: 025-frame-capture
        other_story: US2
        disposition: merge_order
        reason: "031 depends_on_landed 025, whose US2 last changed lib.rs"
      - path: crates/finder/src/lib.rs
        other_spec: 031-imaging-frames-on-console
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and adds its route line after US1's"
      - path: crates/finder/src/link.rs
        other_spec: 025-frame-capture
        other_story: US2
        disposition: merge_order
        reason: "031 depends_on_landed 025, whose US2 last changed link.rs"
      - path: crates/finder/src/main.rs
        other_spec: 025-frame-capture
        other_story: US1
        disposition: merge_order
        reason: "031 depends_on_landed 025, whose US1 last changed main.rs"
      - path: crates/finder/src/imaging/mod.rs
        other_spec: 031-imaging-frames-on-console
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and adds frames to the state"
      - path: crates/finder/src/imaging/routes.rs
        other_spec: 031-imaging-frames-on-console
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and adds its route"
      - path: crates/finder/tests/console/main.rs
        other_spec: 025-frame-capture
        other_story: US2
        disposition: merge_order
        reason: "031 depends_on_landed 025, whose US2 last changed console/main.rs"
      - path: crates/finder/tests/console/main.rs
        other_spec: 031-imaging-frames-on-console
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and adds mod imaging_frames after mod imaging"
      - path: crates/finder/tests/cli/main.rs
        other_spec: 028-finder-tests-do-less
        other_story: US3
        disposition: merge_order
        reason: "031 depends_on_landed 028, whose US3 last changed cli/main.rs"
      - path: crates/finder/tests/cli/main.rs
        other_spec: 031-imaging-frames-on-console
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and adds mod imaging_frames after mod imaging"
    open_questions: []
  US2:
    tests:
      - path: crates/finder/tests/console/imaging_frames.rs
        status: new
        disposition: add
        assertion: "a frame from the stand-in camera is saved as 000001.fits with the cards in order, the region's origin among them, DATE-OBS from the node clock and offset, the times it was received and saved, and samples equal to the sample rule"
        decision: "spec.md:US2-S1"
      - path: crates/finder/tests/console/imaging_frames.rs
        status: new
        disposition: add
        assertion: "a simulated frame is 2036160 bytes at 1236 x 822, 512640 at binning 2 and 83520 for a 400 x 400 region at binning 2, whose file carries the region's origin, the set temperature and the serial, and whose view carries set_temp_c; a fourth with the cooler off has no SET-TEMP"
        decision: "spec.md:US2-S2"
      - path: crates/finder/tests/console/imaging_frames.rs
        status: new
        disposition: add
        assertion: "ids continue after the largest file in the date's folder, and a file already under the next name fails the frame and is left as it is"
        decision: "spec.md:US2-S3"
      - path: crates/finder/tests/console/imaging_frames.rs
        status: new
        disposition: add
        assertion: "a frame in flight for 28 s at 64 KiB/s leaves the link connected while more than 100 heartbeats are answered, and its received_at reads 28 s ± 0.1 s after the end of the exposure"
        decision: "spec.md:US2-S4"
      - path: crates/finder/tests/cli/imaging_frames.rs
        status: new
        disposition: add
        assertion: "the built finder under --simulate --imaging-dir saves one 2036160-byte frame under that folder"
        decision: "spec.md:US2-S5"
      - path: crates/finder/tests/console/imaging_frames.rs
        status: new
        disposition: add
        assertion: "frames taken for the purposes dark and flat are saved with IMAGETYP DARK and FLAT and their PURPOSE"
        decision: "spec.md:US2-S6"
      - path: crates/finder/tests/console/recording.rs
        status: existing
        disposition: preserve
        assertion: "recordings, the frame channel and the live loop are unchanged while imaging frames are taken"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and each binary's main.rs holds only its support include and mod lines"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "the scope link's heartbeat, star frames, mount calls and raw frames never share the imaging connection (spec.md FR-014)"
      - "a saved imaging frame is never rewritten or replaced (spec.md FR-011)"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console imaging_frames::"
        - "cargo nextest run -p finder --locked --test cli imaging_frames::"
        - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs, and the first frame saved at the telescope"
    writes:
      - crates/finder/README.md
      - crates/finder/src/imaging/fits.rs
      - crates/finder/src/imaging/frames.rs
      - crates/finder/src/imaging/mod.rs
      - crates/finder/src/imaging/routes.rs
      - crates/finder/src/lib.rs
      - crates/finder/tests/cli/imaging_frames.rs
      - crates/finder/tests/cli/main.rs
      - crates/finder/tests/console/imaging_frames.rs
      - crates/finder/tests/console/main.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/README.md
        other_spec: 031-imaging-frames-on-console
        other_story: US5
        disposition: merge_order
        reason: "US5 depends_on_merged US2, and adds the abort row and the node's copy"
      - path: crates/finder/src/lib.rs
        other_spec: 031-imaging-frames-on-console
        other_story: US5
        disposition: merge_order
        reason: "US5 depends_on_merged US2, and adds the abort route's line"
      - path: crates/finder/src/imaging/mod.rs
        other_spec: 031-imaging-frames-on-console
        other_story: US5
        disposition: merge_order
        reason: "US5 depends_on_merged US2, and adds abort and the frame-in-flight refusal of set"
      - path: crates/finder/src/imaging/routes.rs
        other_spec: 031-imaging-frames-on-console
        other_story: US5
        disposition: merge_order
        reason: "US5 depends_on_merged US2, and adds the abort route"
      - path: crates/finder/src/imaging/fits.rs
        other_spec: 031-imaging-frames-on-console
        other_story: US3
        disposition: merge_order
        reason: "US3 depends_on_merged US2, and adds decode beside encode"
      - path: crates/finder/src/imaging/frames.rs
        other_spec: 031-imaging-frames-on-console
        other_story: US4
        disposition: merge_order
        reason: "US4 depends_on_merged US2, and adds the checks on what the node sends"
      - path: crates/finder/tests/console/main.rs
        other_spec: 031-imaging-frames-on-console
        other_story: US3
        disposition: merge_order
        reason: "US3 depends_on_merged US2, and adds mod imaging_fits before mod imaging_frames"
    open_questions: []
  US3:
    tests:
      - path: crates/finder/tests/console/imaging_fits.rs
        status: new
        disposition: add
        assertion: "encode writes the worked bytes and cards for a 16-bit and an 8-bit image, a whole real as 100.0, decode gives each back and reads the worked FITS frame, and each refusal displays its text"
        decision: "spec.md:US3-S1"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and each binary's main.rs holds only its support include and mod lines"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "encode, and every file US2 saves, are unchanged: decode only reads (spec.md FR-016)"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console imaging_fits::"
        - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - crates/finder/src/imaging/fits.rs
      - crates/finder/tests/console/imaging_fits.rs
      - crates/finder/tests/console/main.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/tests/console/main.rs
        other_spec: 031-imaging-frames-on-console
        other_story: US4
        disposition: merge_order
        reason: "US4 depends_on_merged US3, and adds mod imaging_checks before mod imaging_fits"
    open_questions: []
  US4:
    tests:
      - path: crates/finder/tests/console/imaging_checks.rs
        status: new
        disposition: add
        assertion: "an incomplete frame, a header that does not add up or is above 256 MiB, a chunk first, too many bytes, a node error, a stall after 30 s and no header within 152 s each end failed with their text and leave no file"
        decision: "spec.md:US4-S1"
      - path: crates/finder/tests/console/imaging_checks.rs
        status: new
        disposition: add
        assertion: "an 8-bit mono frame with no gain, offset or temperature is saved with BITPIX 8, no BZERO or BSCALE, none of the absent cards and its bytes unchanged, and the header's region replaces the settings' in its view and its origin cards"
        decision: "spec.md:US4-S2"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and each binary's main.rs holds only its support include and mod lines"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "a whole frame is saved as US2 saves it; the checks refuse only what breaks the rules of spec.md FR-010"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console imaging_checks::"
        - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - crates/finder/src/imaging/frames.rs
      - crates/finder/tests/console/imaging_checks.rs
      - crates/finder/tests/console/main.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/imaging/frames.rs
        other_spec: 031-imaging-frames-on-console
        other_story: US5
        disposition: merge_order
        reason: "US5 depends_on_merged US4, and adds abort, the frame-in-flight refusal of take and the node's copy"
      - path: crates/finder/tests/console/main.rs
        other_spec: 031-imaging-frames-on-console
        other_story: US5
        disposition: merge_order
        reason: "US5 depends_on_merged US4, and adds mod imaging_control after mod imaging_checks"
    open_questions: []
  US5:
    tests:
      - path: crates/finder/tests/console/imaging_control.rs
        status: new
        disposition: add
        assertion: "a foreign origin, a second request, a settings change during an exposure and every not-ready state are refused with their text"
        decision: "spec.md:US5-S1"
      - path: crates/finder/tests/console/imaging_control.rs
        status: new
        disposition: add
        assertion: "status.exposing is true 1 s into a 30 s exposure, abort ends it as aborted within 1 s with no file, and the next frame is 000002.fits"
        decision: "spec.md:US5-S2"
      - path: crates/finder/tests/console/imaging_control.rs
        status: new
        disposition: add
        assertion: "with save_at_node and a node that has no save folder the frame is still saved and its view carries the node's reason, and neither key without save_at_node"
        decision: "spec.md:US5-S3"
      - path: crates/finder/tests/console/imaging_control.rs
        status: new
        disposition: add
        assertion: "with save_at_node and a node that has a save folder the frame view names the node's file, whose samples equal the console's"
        decision: "spec.md:US5-S4"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and each binary's main.rs holds only its support include and mod lines"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "POST /api/imaging/abort with nothing in progress changes nothing (spec.md FR-013)"
      - "the tests of US2 to US4 pass unchanged: a frame with save_at_node off is saved as before"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console imaging_control::"
        - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - crates/finder/README.md
      - crates/finder/src/imaging/frames.rs
      - crates/finder/src/imaging/mod.rs
      - crates/finder/src/imaging/routes.rs
      - crates/finder/src/lib.rs
      - crates/finder/tests/console/imaging_control.rs
      - crates/finder/tests/console/main.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/README.md
        other_spec: 031-imaging-frames-on-console
        other_story: US6
        disposition: merge_order
        reason: "US6 depends_on_merged US5, and adds the preview's row and paragraph"
      - path: crates/finder/src/lib.rs
        other_spec: 031-imaging-frames-on-console
        other_story: US6
        disposition: merge_order
        reason: "US6 depends_on_merged US5, and adds the preview route's line"
      - path: crates/finder/src/imaging/mod.rs
        other_spec: 031-imaging-frames-on-console
        other_story: US6
        disposition: merge_order
        reason: "US6 depends_on_merged US5, and adds the kept previews"
      - path: crates/finder/src/imaging/routes.rs
        other_spec: 031-imaging-frames-on-console
        other_story: US6
        disposition: merge_order
        reason: "US6 depends_on_merged US5, and adds the preview route"
      - path: crates/finder/src/imaging/frames.rs
        other_spec: 031-imaging-frames-on-console
        other_story: US6
        disposition: merge_order
        reason: "US6 depends_on_merged US5, and makes the preview when a frame is saved"
      - path: crates/finder/tests/console/main.rs
        other_spec: 031-imaging-frames-on-console
        other_story: US6
        disposition: merge_order
        reason: "US6 depends_on_merged US5, and adds mod imaging_preview after mod imaging_frames"
    open_questions: []
  US6:
    tests:
      - path: crates/finder/tests/console/imaging_preview.rs
        status: new
        disposition: add
        assertion: "the worked 6 x 6 RGGB frame becomes the worked 3 x 3 picture, 197 for a half and 140 for a quarter of the span, green the mean of its two samples, and read as BGGR its red and blue swap"
        decision: "spec.md:US6-S1"
      - path: crates/finder/tests/console/imaging_preview.rs
        status: new
        disposition: add
        assertion: "a wide mono frame is box-averaged by 5, a 2600 x 1400 mosaic becomes 650 x 350, a flat frame is black, mono is gray, a block is its mean, and the span is at least 1"
        decision: "spec.md:US6-S2"
      - path: crates/finder/tests/console/imaging_preview.rs
        status: new
        disposition: add
        assertion: "a saved frame's preview route answers a 32 x 24 JPEG, an unknown id 404, a bad id 400, only the last eight are kept, and a frame with no whole 2 x 2 cell is saved with no preview"
        decision: "spec.md:US6-S3"
      - path: crates/finder/tests/console/imaging_preview.rs
        status: new
        disposition: add
        assertion: "every saturated pixel of a pure red simulated star is red and near the star, and a pure blue one's blue"
        decision: "spec.md:US6-S4"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and each binary's main.rs holds only its support include and mod lines"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "a frame is saved whether or not its preview can be made (plan.md The preview)"
      - "the finder camera's preview route and its last eight previews are unchanged"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console imaging_preview::"
        - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs, and a design review of the preview's look"
    writes:
      - crates/finder/README.md
      - crates/finder/src/imaging/frames.rs
      - crates/finder/src/imaging/mod.rs
      - crates/finder/src/imaging/preview.rs
      - crates/finder/src/imaging/routes.rs
      - crates/finder/src/lib.rs
      - crates/finder/tests/console/imaging_preview.rs
      - crates/finder/tests/console/main.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/README.md
        other_spec: 031-imaging-frames-on-console
        other_story: US7
        disposition: merge_order
        reason: "US7 depends_on_merged US6, and adds a sentence to Night mode"
      - path: crates/finder/tests/console/main.rs
        other_spec: 031-imaging-frames-on-console
        other_story: US9
        disposition: merge_order
        reason: "US9 depends_on_merged US6, and adds mod imaging_lease after mod imaging_frames"
      - path: crates/finder/src/imaging/mod.rs
        other_spec: 031-imaging-frames-on-console
        other_story: US9
        disposition: merge_order
        reason: "US9 depends_on_merged US6, and adds the lease to the state"
      - path: crates/finder/src/imaging/frames.rs
        other_spec: 031-imaging-frames-on-console
        other_story: US9
        disposition: merge_order
        reason: "US9 depends_on_merged US6, and adds the lease's check to take"
    open_questions: []
  US7:
    tests:
      - path: crates/finder/web/src/components/ImagingPage.test.tsx
        status: new
        disposition: add
        assertion: "the /imaging route shows the Imaging heading and Back to console, and the top bar shows the Imaging link only with a scope node, current on that route"
        decision: "spec.md:US7-S1"
      - path: crates/finder/web/src/components/ImagingPage.test.tsx
        status: new
        disposition: add
        assertion: "each state that is not ready shows exactly its words and no Settings or Frame region"
        decision: "spec.md:US7-S2"
      - path: crates/finder/web/src/components/ImagingPage.test.tsx
        status: new
        disposition: add
        assertion: "the ready page shows the camera line, the sensor line for each cooler state, the settings the camera has and the forwarding limit, and saves them with one PUT in the stated key order"
        decision: "spec.md:US7-S3"
      - path: crates/finder/web/src/components/ImagingPage.test.tsx
        status: new
        disposition: add
        assertion: "the Imaging frame region's button, statuses and alerts follow the frame's state with the stated words, the time in hand included, Take disabled and Abort shown while exposing or receiving, and post frame and abort"
        decision: "spec.md:US7-S4"
      - path: crates/finder/web/src/components/ImagingPage.test.tsx
        status: new
        disposition: add
        assertion: "the picture has the photo class, and at night Hold to see colour removes it only while held, on every release event and after 10 s from the first press, a repeated keydown not starting the limit again; and the useColourHold hook holds only at night"
        decision: "spec.md:US7-S5"
      - path: crates/finder/web/src/components/ImagingPage.test.tsx
        status: new
        disposition: add
        assertion: "the hook reads /api/imaging on opening, every second and after each request, stops when unmounted, and a failed read changes nothing shown"
        decision: "spec.md:US7-S6"
      - path: crates/finder/web/e2e/imaging.spec.ts
        status: new
        disposition: add
        assertion: "on the simulate console a frame taken from the page appears as a 618 x 411 picture"
        decision: "spec.md:US7-S7"
      - path: crates/finder/web/e2e/imaging.spec.ts
        status: new
        disposition: add
        assertion: "at night the page has no off-red pixel, the picture alone shows colour while the control is held, and it is red again on release"
        decision: "spec.md:US7-S8"
      - path: crates/finder/web/src/components/StopButton.test.tsx
        status: existing
        disposition: preserve
        assertion: "the top bar without the imaging prop renders as before"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/src/components/SetupPage.test.tsx
        status: existing
        disposition: preserve
        assertion: "the Setup page reads one status as before"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/e2e/session.spec.ts
        status: existing
        disposition: preserve
        assertion: "the console and Setup without a node are still red only at night, with today's top bar"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/e2e/simulate.spec.ts
        status: existing
        disposition: preserve
        assertion: "the simulate console's live chart, preview and Setup checks pass with the Imaging link in the bar"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/e2e/mcp.spec.ts
        status: existing
        disposition: preserve
        assertion: "the top bar's night check with an MCP move passes with the Imaging link in the bar"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "the page adds no npm package and index.css is unchanged (plan.md The page)"
      - "useSession copies the fields it copied before"
      - "red-only night mode everywhere but the one held picture (spec.md owner approval 6)"
    checks:
      development:
        - "npm test"
        - "npm run typecheck"
        - "npm run build"
        - "CI=1 npm run e2e"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs: finder-page; and a check of the hold control on a phone in the dark"
    writes:
      - .impeccable/design.json
      - DESIGN.md
      - crates/finder/README.md
      - crates/finder/web/dist
      - crates/finder/web/e2e/imaging.spec.ts
      - crates/finder/web/playwright.config.ts
      - crates/finder/web/src/App.tsx
      - crates/finder/web/src/api.ts
      - crates/finder/web/src/components/ImagingPage.test.tsx
      - crates/finder/web/src/components/ImagingPage.tsx
      - crates/finder/web/src/components/TopBar.tsx
      - crates/finder/web/src/useColourHold.ts
      - crates/finder/web/src/useImaging.ts
    no_write_reason: ""
    shared_writes:
      - path: .impeccable/design.json
        other_spec: 025-frame-capture
        other_story: US3
        disposition: merge_order
        reason: "031 depends_on_landed 025, whose US3 last changed design.json"
      - path: DESIGN.md
        other_spec: 025-frame-capture
        other_story: US3
        disposition: merge_order
        reason: "031 depends_on_landed 025, whose US3 last changed DESIGN.md"
      - path: crates/finder/README.md
        other_spec: 031-imaging-frames-on-console
        other_story: US8
        disposition: merge_order
        reason: "US8 depends_on_merged US7, and adds the three tools to MCP clients"
      - path: crates/finder/web/dist
        other_spec: 025-frame-capture
        other_story: US3
        disposition: merge_order
        reason: "031 depends_on_landed 025, whose US3 last rebuilt the bundle"
      - path: crates/finder/web/src/api.ts
        other_spec: 025-frame-capture
        other_story: US3
        disposition: merge_order
        reason: "031 depends_on_landed 025, whose US3 last changed api.ts"
    open_questions: []
  US8:
    tests:
      - path: crates/finder/tests/mcp/imaging.rs
        status: new
        disposition: add
        assertion: "the three tools are listed with object output schemas, only get_imaging_camera read-only, the stated descriptions, and set_imaging_camera's input schema holding exactly the nine settings keys, the seven that refuse null not nullable"
        decision: "spec.md:US8-S1"
      - path: crates/finder/tests/mcp/imaging.rs
        status: new
        disposition: add
        assertion: "get_imaging_camera and get_session's imaging equal GET /api/imaging, set_imaging_camera changes settings, and a refused gain and a parse error are tool errors with the route's text"
        decision: "spec.md:US8-S2"
      - path: crates/finder/tests/mcp/imaging.rs
        status: new
        disposition: add
        assertion: "take_imaging_frame answers only after the frame is saved, with the frame and a JPEG image block equal to the preview route's body"
        decision: "spec.md:US8-S3"
      - path: crates/finder/tests/mcp/imaging.rs
        status: new
        disposition: add
        assertion: "take_imaging_frame refuses arguments and a second exposure, and reports an aborted frame and a failed one, each as a tool error with its text"
        decision: "spec.md:US8-S4"
      - path: crates/finder/tests/mcp/imaging.rs
        status: new
        disposition: add
        assertion: "on a console with no imaging the tools answer at once: off, and this console takes no imaging frames"
        decision: "spec.md:US8-S5"
      - path: crates/finder/tests/mcp/imaging.rs
        status: new
        disposition: add
        assertion: "no result of the three tools holds the node's token"
        decision: "spec.md:US8-S6"
      - path: crates/finder/tests/mcp/motion.rs
        status: existing
        disposition: preserve
        assertion: "every listed tool called with empty arguments answers without the token, the three new ones included"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/mcp/frames.rs
        status: existing
        disposition: preserve
        assertion: "solve_frame, get_pointing and get_link are listed and answer as before"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and each binary's main.rs holds only its support include and mod lines"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "the sixteen existing tools, the door, the rate limit and the call log are unchanged"
      - "no MCP tool is added on the scope node"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test mcp imaging::"
        - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - crates/finder/README.md
      - crates/finder/src/mcp/tools.rs
      - crates/finder/tests/mcp/imaging.rs
      - crates/finder/tests/mcp/main.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/README.md
        other_spec: 031-imaging-frames-on-console
        other_story: US9
        disposition: merge_order
        reason: "US9 depends_on_merged US8, and adds the one-holder paragraph to Imaging frames"
    open_questions: []
  US9:
    tests:
      - path: crates/finder/tests/console/imaging_lease.rs
        status: new
        disposition: add
        assertion: "lease hands the camera to one holder at once, a second is refused at once with 409 the imaging camera is held by the test, holder names it, and a lease is granted after the first is dropped and on a console with no link"
        decision: "spec.md:US9-S1"
      - path: crates/finder/tests/console/imaging_lease.rs
        status: new
        disposition: add
        assertion: "while a lease is out Imaging::set and take, both routes and both MCP tools answer the holder's 409 before and during the holder's frame, nothing changes, and the state's words come first with no link"
        decision: "spec.md:US9-S2"
      - path: crates/finder/tests/console/imaging_lease.rs
        status: new
        disposition: add
        assertion: "the holder's Lease::set and Lease::take change the settings, take and save a frame, and refuse a bad value and a frame in flight with Imaging's own words"
        decision: "spec.md:US9-S3"
      - path: crates/finder/tests/console/imaging_lease.rs
        status: new
        disposition: add
        assertion: "with a lease out anyone reads the view and get_imaging_camera, aborts the holder's frame and fetches its preview"
        decision: "spec.md:US9-S4"
      - path: crates/finder/tests/console/imaging_lease.rs
        status: new
        disposition: add
        assertion: "aborting the holder's task releases the camera in the same instant, the lease adds nothing to the Imaging's strong count, its frame runs on unheld, and a lease of a dropped Imaging answers this console takes no imaging frames"
        decision: "spec.md:US9-S5"
      - path: crates/finder/tests/console/imaging_lease.rs
        status: new
        disposition: add
        assertion: "a frame from the page or take_imaging_frame holds no lease, a lease is granted during it, and the holder meets the frame-in-flight words itself"
        decision: "spec.md:US9-S6"
      - path: crates/finder/tests/mcp/motion.rs
        status: existing
        disposition: preserve
        assertion: "every listed tool called with empty arguments answers without the token, the three imaging tools included, on a console with no imaging and so no lease"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and each binary's main.rs holds only its support include and mod lines"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "with no lease out, Imaging::set and Imaging::take refuse and answer exactly as US1, US2 and US5 have them, so the tests of US1 to US5 and US8 pass unchanged (spec.md FR-030)"
      - "routes.rs and mcp/tools.rs are unchanged: the check is inside Imaging (plan.md The camera lease)"
      - "abort, ended, view and preview are open to anyone (spec.md FR-031)"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console imaging_lease::"
        - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs; 033 to 037 re-anchored to the lease"
    writes:
      - crates/finder/README.md
      - crates/finder/src/imaging/frames.rs
      - crates/finder/src/imaging/mod.rs
      - crates/finder/tests/console/imaging_lease.rs
      - crates/finder/tests/console/main.rs
    no_write_reason: ""
    shared_writes: []
    open_questions: []
```
