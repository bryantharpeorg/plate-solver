# Implementation Plan: The Tools Page Focuses Either Camera

**Branch**: `051-tools-page-focuses-either-camera` | **Date**: 2026-11-26 |
**Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/051-tools-page-focuses-either-camera/spec.md`

## Summary

The Tools page gains a second block, `Focus a camera`, with a select of the camera. For
the imaging camera it renders 033's own Focus block, unchanged, over the Tools page's
imaging view. For the finder camera it runs a new focus aid on the console: with a fixed
exposure and gain set on Setup, one raw frame after another (025's `RawFrame`) is asked
of the scope node while the live solve goes on, each is measured at the centroids of its
own star frame (the node's detection, 048) as the median half-flux diameter of at most 15
isolated, unsaturated stars, each summed in sub-pixel circles so that a star of a pixel,
as a focused star of this optic is, still reads its size, and each measure goes as a
`Sample` into 033's own `Aid`, which gives the median of three, the trend, the best so
far and `found`. Beside the reading the page shows the star frame's detected count. The
aid changes no setting, holds no lease and keeps no record; a stop, a failure or a change
of exposure or gain ends it. The simulator's finder camera gains a focus error, so that
the gates can turn its focuser.

Four stories: the measure and the simulator's focuser; the run, its routes and view;
MCP, the README and the glossary; the page. Each is under 54 % of 65,536 B on probe
round 1's code with its fold (§ Plan traps 2). One general rule removes every side
effect the owner would otherwise approve one by one: **the finder camera's aid adds no
rule of its own to 033's arithmetic, takes nothing from the live solve and changes no
setting; it asks only that the exposure and gain be fixed, and ends when they change**
(spec.md § Owner decisions).

## Technical Context

**Language/Version**: Rust, MSRV 1.96, stable in CI; TypeScript and React for the page,
as built today.

**Primary Dependencies**: none new. `Cargo.toml`, `Cargo.lock`, `package.json` and
`package-lock.json` do not change. The run uses `tokio`, `axum`, `tonic` (for the raw
request's status code) and `serde`, all already dependencies of `crates/finder`.

**Storage**: none. The run is in memory; nothing is written to disk or to
`session.json`.

**Testing**:
- `crates/finder/tests/console/finder_focus_measure.rs` (new, US1): `measure` on drawn
  frames of the test's own making (US1-S1 to US1-S3, US1-S6) and on the simulator's
  finder camera (US1-S4), and the renderer's flux (US1-S5). US1-S1 to US1-S3, US1-S5
  and US1-S6 under `#[test]`; US1-S4 under `#[tokio::test(start_paused = true)]`, since
  `SimCamera` sleeps for the exposure.
- `crates/finder/tests/console/finder_focus_aid.rs` (new, US2): the focus rig with
  `pub struct Rig { pub app: Router, pub sim: Simulator, pub node: Arc<SimNode> }`,
  `pub fn rig() -> Rig` and its helpers, and US2-S1 to US2-S5 under
  `#[tokio::test(start_paused = true)]`.
- `crates/finder/tests/mcp/finder_focus.rs` (new, US3): its own rig, as the MCP
  binary's tests build theirs, US3-S1 under `#[tokio::test(start_paused = true)]` and
  US3-S2 under `#[test]`.
- `crates/finder/web/src/components/FocusTool.test.tsx` (new, US4): US4-S1 to US4-S4;
  `crates/finder/web/src/components/FocusToolPage.test.tsx` (new, US4): US4-S5;
  `crates/finder/web/e2e/tools-focus.spec.ts` (new, US4): US4-S6 under Playwright's
  simulate project.

**Target Platform**: the console on the Pi beside the scope node, as the field kit is
built, where the scope link is the Pi's own loopback, and the page in a phone's browser.
Each reading is one raw frame of 1,228,800 B on the link. On a remote link, the console
elsewhere over a phone's hotspot, one raw frame can hold up the link's heartbeat, every
250 ms, past its 1 s of silence (`crates/finder/src/link.rs:31`,
`crates/finder/src/link.rs:32`), and drop the link (read, not run). A cap on the cadence
would not shorten that one frame, so there is none; the README says where to run the
console.

**Constraints**:
- 033's module (`crates/finder/src/imaging/focus.rs` and `focus/`), 025's recording,
  the link, the live loop, the node, the proto and the node's detection are used and
  not changed (FR-003, FR-009).
- `RUSTFLAGS="-D warnings"`, `--all-features` clippy, and rustdoc with warnings denied.
- No test uses a camera, a mount, ASTAP or the network.

**Scale/Scope**: four stories; one new Rust module and three changed Rust files in
`crates/finder` (`lib.rs`, `api.rs`, `mcp/tools.rs`), three changed simulator files,
three new Rust test files, one new page component and its two tests, one browser test,
the README's rows, paragraph and section, and three glossary entries.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Detection and the plate solver are untouched; the aid measures at the centroids the node's own detection (cedar-detect, 048) gives, and the live solve is the live loop's own. The measure is the half-flux diameter as the repository's survey sets it out (`docs/research/2026-11-15-imaging-camera.md:206`), summed in sub-pixel circles, checked against a Gaussian's own half-flux diameter at three phases of the pixel grid and a uniform disc's; cedar-detect's own limit on stars smaller than its grid (`reference-solutions/cedar-detect/src/algorithm.rs:89`) is a Risks row and a README line; what rests on reading is listed in § What was run and what was read. |
| II. Never weaken a gate | No existing test, golden, fixture, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS` or `ergane.yaml` is edited. `playwright.config.ts` gains two list entries, an owner approval as 050's were. The only edits to existing test files are the `mod` lines of `crates/finder/tests/console/main.rs` and `crates/finder/tests/mcp/main.rs` (SC-002). The simulator draws exactly the pixels it drew at c75a6e76 at focus error 0 (US1-S4 pins their FNV-1a hash, computed at c75a6e76), so no existing caller observes a change (run: nextest's every test passed on the probe's tip, § What was run). |
| III. Tests first, independent expectations | The oracles are a Gaussian's half-flux diameter, 2·√(2 ln 2)·σ, at the simulator's own star width and at drawn stars integrated over each pixel, a uniform disc's, √2 times its radius, the order of small drawn stars' sizes and the bounds the scenarios write, the FNV-1a hash of c75a6e76's own pixels computed at c75a6e76, the drawn frames' own places and densities, 025's and 033's words, and the scenarios' words. No test calls `measure` for an expected value or reads `STARS_MEASURED`, `BOX_PX`, `SATURATED`, `THRESHOLD_NOISE`, `SUBPIXELS`, `APERTURE_MARGIN_PX` or `RAW_WAIT`. Time comes from a paused tokio clock and the simulator's virtual clock. |
| IV. Read the critical paths twice | None is touched: no `Cargo.toml` or `Cargo.lock` change, no `unsafe`, nothing under `proto/`, `crates/math-core`, `crates/pattern-database` or `crates/database-generation`, no `build.rs` or `.cargo/`. `crates/scope-link`'s proto is read, not changed. `crates/simulator` is not a critical path; its change is owner approval 1. |
| V. Proven open source first, with honest licensing | Looked for, in local sources only (no download): the repository's survey of 2026-11-15, § 6 "Focus" (`docs/research/2026-11-15-imaging-camera.md:203`), with its sources on PHD2's half-flux code, SharpCap, Ekos and N.I.N.A.; ASTAP's `-analyse`, installed as `astap_cli`; the vendored `reference-solutions/` (cedar-detect, cedar-solve), whose `StarDescription` gives a centroid, a peak, a brightness and a saturated count and no size (`reference-solutions/cedar-detect/src/algorithm.rs:503`); and the owner's own bring-up script, outside the repository. **PHD2's Star Profile** shows the half-flux diameter of this camera on a guide scope, its common pairing, but PHD2 must own the camera: the scope node exposes it in a loop of its own (`crates/scope-node/src/service.rs:268`) through INDI (045), so the node and the live solve stop while it runs, and it is a desktop program reached over the tunnel, not a phone page beside the live solve. **Ekos's Focus module** loops half-flux radii with no focuser in the same way, and for the same two reasons is not adopted. **A Bahtinov mask** needs a bright star and a view sharp enough to place its spikes, which the 640-pixel preview every 5 s is not, and a mask cut for the 40 mm objective. **SEP's `flux_radius`** is the method adopted, written fresh from its description (sub-pixel circles and a linear read of the half-flux crossing), with no SEP source read or copied: SEP is a C library with a Python wrapper; its README gives LGPL-3.0 for the library as a whole (the parts derived from Source Extractor; one file from photutils is BSD-3-Clause and the Python wrapper MIT), checked by the operator seat against the README on 2026-11-26, which is outside `deny.toml`'s allow list (`deny.toml:75`) and not the MIT OR Apache-2.0 of ADR 0003, and it would come through FFI and a build script, both critical paths (IV). **ASTAP's `-analyse`**, 033's measure, was tried on the earlier finder camera's frames and found 3 to 7 stars, under five in five of eight, at 033's signal-to-noise floor of 30 (`crates/finder/src/imaging/astap.rs:28`); it needs an imaging solver on the console, and the gates' simulated one reads the imaging camera's truth, not the frame (spec.md owner decision 1 (c)). A size from cedar-detect would be a change to the node and the proto (decision 1 (f)). 033's arithmetic is reused as it is. |

## Project Structure

```text
crates/simulator/src/
├── optics.rs                                   # US1: Pointing's finder_focus_error, its getter and setter
├── render.rs                                   # US1: render_with_sigma; render calls it
└── camera.rs                                   # US1: SimCamera draws with the focus error's width
crates/finder/
├── README.md                                   # US3: rows, MCP paragraph, ## Focusing the finder camera
├── src/
│   ├── lib.rs                                  # US1: pub mod finder_focus
│   ├── finder_focus.rs                         # US1 new: measure; US2: the run, view and routes
│   ├── api.rs                                  # US2: AppState's finder_focus, the routes merged
│   └── mcp/tools.rs                            # US3: get_finder_focus, set_finder_focus
├── tests/
│   ├── console/finder_focus_measure.rs         # US1, new
│   ├── console/finder_focus_aid.rs             # US2, new
│   ├── console/main.rs                         # US1, US2: one mod line each
│   ├── mcp/finder_focus.rs                     # US3, new
│   └── mcp/main.rs                             # US3: one mod line
└── web/
    ├── playwright.config.ts                    # US4: two list entries
    ├── e2e/tools-focus.spec.ts                 # US4, new
    ├── dist/                                   # US4: rebuilt
    └── src/
        ├── api.ts                              # US4: FinderFocus and its three calls
        └── components/
            ├── ToolsPage.tsx                   # US4: one line
            ├── FocusTool.tsx                   # US4, new
            ├── FocusTool.test.tsx              # US4, new
            └── FocusToolPage.test.tsx          # US4, new
GLOSSARY.md                                     # US3: Focus reading, Focus aid, Raw frame
```

## Interfaces

The code as it ran on `probe/051` (tip 1efd1bc3), and as probe round 1 built it from
this text on `probe/051-r1` (tip 9ef38442 with its fold). Bodies not given here are the
builder's, to these words and the FRs.

### `crates/simulator` (US1)

`optics.rs`: `Pointing` (`crates/simulator/src/optics.rs:74`) gains
`pub finder_focus_error: f64`, `0.0` in `Simulator::new`, and `Simulator` gains, after
`set_focus_error` (`crates/simulator/src/optics.rs:299`):

```rust
/// The finder camera's focus error (051): 0 is best focus, and its sign
/// does not change a star's size.
pub fn finder_focus_error(&self) -> f64;

/// Turn the finder camera's simulated focuser (051): its stars' Gaussian
/// width becomes `render::PSF_SIGMA_PX` and `imaging::FOCUS_SIGMA_PX` times
/// the error, added in quadrature, from the next exposure. Panics on a
/// value that is not finite.
pub fn set_finder_focus_error(&self, error: f64);
```

`render.rs`: `render` keeps its signature and calls `render_with_sigma` with
`PSF_SIGMA_PX`:

```rust
/// `render` with stars of Gaussian width `sigma_px` (051): each star keeps its
/// flux, so its peak falls as the square of `PSF_SIGMA_PX / sigma_px`, and its
/// stamp grows with it. At `PSF_SIGMA_PX` the pixels are `render`'s own.
pub fn render_with_sigma(
    optics: &Optics,
    stars: &[RenderStar],
    exposure_ms: f64,
    gain: f64,
    seed: u64,
    frame_id: u64,
    sigma_px: f64,
) -> Vec<u8>;
```

It asserts `sigma_px` finite and at least `PSF_SIGMA_PX`; above it, each star's peak is
multiplied by `(PSF_SIGMA_PX / sigma_px).powi(2)` and its stamp's radius is
`(8.0 * sigma_px / PSF_SIGMA_PX).ceil()`; at `PSF_SIGMA_PX` both are as before, so the
pixels are equal. `camera.rs`: `SimCamera::next_frame` draws with
`render::PSF_SIGMA_PX.hypot(crate::imaging::FOCUS_SIGMA_PX * p.finder_focus_error)`
through `render_with_sigma`, where it called `render`.

### `crates/finder/src/finder_focus.rs` (US1 new; US2 adds)

Module doc: "The finder camera's focus aid (051): the size of the stars in the finder
camera's raw frames, read beside the live solve and judged by 033's own arithmetic
(`imaging::focus::aid::Aid`). It changes no setting of the finder camera and takes
nothing from the live solve; each reading costs one raw frame on the scope link (025)."
`lib.rs` gains `pub mod finder_focus;` after `pub mod ephemeris;`.

US1:

```rust
/// The brightest unsaturated stars measured in a frame (US1).
pub const STARS_MEASURED: usize = 15;
/// Half the side of the square box each star is measured in, in pixels (US1).
pub const BOX_PX: i64 = 16;
/// A star with a joined pixel at or above this level is not measured (US1).
pub const SATURATED: u8 = 250;
/// A pixel joins a star only this many noise levels above the background
/// (US1).
pub const THRESHOLD_NOISE: f64 = 3.0;
/// Each pixel is summed as this many sub-pixels a side when a star's flux is
/// counted in circles (US1).
pub const SUBPIXELS: u32 = 5;
/// A star's circle reaches this far past its farthest joined pixel, in pixels
/// (US1).
pub const APERTURE_MARGIN_PX: f64 = 1.0;

/// One raw frame measured at its own star frame's centroids, the node's
/// detection (US1): a reading, or why there is none, as 033's `Sample` names
/// it. The centroids are brightest first; a star is measured when its box lies
/// inside the frame and holds no other centroid, and its own pixels are under
/// `SATURATED` and have flux; the first `STARS_MEASURED` such stars are
/// measured, each as its half-flux diameter in sub-pixel circles.
pub fn measure(pixels: &[u8], width: u32, height: u32, centroids: &[Centroid]) -> Sample;
```

`Centroid` is `scope_link::proto::Centroid`; `Sample`, `MIN_STARS` and `FAR_OUT_PX` are
033's (`crates/finder/src/imaging/focus/aid.rs:33`,
`crates/finder/src/imaging/focus.rs:16`, `crates/finder/src/imaging/focus.rs:21`). One
star's size, as FR-002 gives it, in coordinates relative to the centre of the box's
centre pixel (column `x.floor()`, row `y.floor()`; the box pixel `dx` columns and `dy`
rows from it has its centre at (`dx`, `dy`)): the 128 edge pixels' median, of the two
middle values their mean, as background; 1.4826 times their median absolute deviation,
at least 0.5, as noise; the start pixel the brightest of the 5 × 5 round the box's
centre, the first in reading order on a tie (a strict `>` while walking the rows top to
bottom and each row left to right), and nothing measured unless it is above background
plus three noise levels; an 8-connected fill from it over the pixels above that level,
its own pixels; none measured with one of them at `SATURATED` or more; their centroid
weighted by value less background; the circle's radius the farthest own pixel's centre's
distance plus `APERTURE_MARGIN_PX`, at most `BOX_PX − 1`; every box pixel whose centre
is within the radius plus 1 of the centre split into 5 × 5 parts at
(`dx − 0.5 + (i + 0.5)/5`, `dy − 0.5 + (j + 0.5)/5`), each carrying its pixel's value
less background over 25, kept when at most the radius from the centre; the parts sorted
by distance (`total_cmp`), each part within 10⁻⁹ px of the one before summed with it as
one distance (in cedar's own coordinates float rounding splits such ties by a star's
place in the frame), the running sum from 0 at distance 0, and the half-flux radius
where it first reaches half the kept parts' total, linearly between the distance before
and the distance reached; twice that radius. The method is SEP's `flux_radius`,
sub-pixel apertures read linearly, written fresh from that description; no source of
SEP's is read or copied (Constitution V).

US2 adds:

```rust
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
    /// The centroids of the frame's own star frame: the stars the scope node
    /// detected in it.
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
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FocusAsk {}

/// Why a start was refused: the status and the words.
#[derive(Debug, Clone, PartialEq)]
pub struct Refusal {
    pub status: StatusCode,
    pub message: String,
}

/// The finder camera's focus aid: at most one run at a time (US2).
#[derive(Default)]
pub struct FinderFocus { /* private: the run, under a std Mutex */ }

impl FinderFocus {
    /// The latest run, or `None` before the first start.
    pub fn view(&self) -> Option<FocusView>;
    /// Start a run over the link's raw frames (US2), under the session's
    /// camera settings, `None` before any is set.
    pub fn start(
        self: &Arc<Self>,
        link: Option<Arc<LinkManager>>,
        settings: Option<CameraSettings>,
        clock: Arc<dyn Clock>,
    ) -> Result<FocusView, Refusal>;
    /// Stop the run, if one runs, and answer the view (US2).
    pub fn stop(&self) -> Option<FocusView>;
}

pub(crate) fn routes() -> Router<AppState>; // GET, POST and DELETE /api/finder-camera/focus
```

`CameraSettings` is `crate::session::CameraSettings`
(`crates/finder/src/session.rs:313`), automatic when its `exposure_ms` or `gain` has no
`number()`; `None`, settings never set, is the session's default, automatic
(`crates/finder/src/session.rs:295`), and is refused as automatic. `start` refuses in
FR-004's order. The view's fields after each sample are 033's `Aid` answers: `readings`
is `Aid::readings()`, `last.state` is `Aid::phase()`'s `as_str()`, `median` is
`Aid::median()`, `best`, `trend` (`Trend::as_str()`), `found` and `history` likewise;
`last.detected` is the paired star frame's centroid count. The run's task holds a `Weak`
of the `FinderFocus`; a stop aborts it. Its loop: take `link.raw()` (none ends `failed`,
`the scope node is not connected`); send `raw_frame()` under
`tokio::time::timeout(RAW_WAIT, ..)`, keeping, while it is on its way, each new star
frame of `link.frames()` (its id, centroids, `exposure_ms` and `gain`, the last eight);
map the reply as FR-007 says; then wait up to `RAW_WAIT` more for the star frame of the
raw frame's id, skipping the raw frame when it does not come; the first star frame
paired sets the run's exposure and gain, and a later one with either different ends the
run `failed` before it is measured; measure in `tokio::task::spawn_blocking`, a
`JoinError` (a panic) ending the run `failed`,
`the focus reading of a raw frame failed`; push the `Sample`; repeat. The frames watch
ending also ends the run `failed`, `the scope node is not connected`. The run's mutex is
never held across an `await`.

`api.rs`: `AppState` (`crates/finder/src/api.rs:102`) gains the private field
`finder_focus: Arc<crate::finder_focus::FinderFocus>`, `Arc::default()` in
`AppState::shared` (`crates/finder/src/api.rs:144`), and

```rust
/// The finder camera's focus aid (051).
pub fn finder_focus(&self) -> Arc<crate::finder_focus::FinderFocus>;
```

`router` merges `crate::finder_focus::routes()` after the imaging routes
(`crates/finder/src/api.rs:653`). The `POST` handler takes `SessionWrite` (the
same-origin rule, `crates/finder/src/api.rs:530`) and
`Result<Json<FocusAsk>, JsonRejection>` through `sim_json`
(`crates/finder/src/api.rs:766`), then
`start(state.link(), state.session().snapshot(state.clock.now()).camera_settings, state.clock.clone())`,
a refusal answered as `ApiError::new(refusal.status, refusal.message)`; `DELETE` takes
`SessionWrite` and answers `{"focus": stop()}`; `GET` answers `{"focus": view()}`.

### `crates/finder/src/mcp/tools.rs` (US3)

```rust
#[derive(rmcp::schemars::JsonSchema)]
#[expect(dead_code, reason = "schema only; set_finder_focus reads on itself")]
struct FinderFocusArguments {
    on: bool,
}
```

`get_finder_focus`, read-only, with the description "Read the finder camera's focus
aid: the size of the finder camera's stars in its pixels, the median half-flux diameter
of the last three readings, the best so far, the trend since the last change, found once
the reading has been through focus and back, and the stars the scope node detected in
the last frame. null before the first start." `set_finder_focus`, with the input schema
of `FinderFocusArguments` and the description "Start or stop the finder camera's focus
aid, as POST and DELETE /api/finder-camera/focus do: raw frames of the finder camera
measured beside the live solve, which goes on. It moves nothing and changes no setting:
the observer turns the finder camera's focuser. The finder camera's exposure and gain
must be fixed first, on Setup; a change of either ends the run. Read get_finder_focus
every few seconds." Its checks in this order: `on` not a boolean,
`on must be true or false`; any other key, `set_finder_focus takes on and nothing else`;
then `start` (with the session's camera settings, as the route) or `stop`.

### The README (`crates/finder/README.md`, US3)

After the finder alignment's MCP paragraph and its closing sentence, before
`### MCP under WSL2`:

```markdown
The Tools page's focus aid for the finder camera has two tools:

- `get_finder_focus` takes no arguments and reads `{"focus": ...}`: the finder
  camera's focus aid, its readings in the finder camera's pixels, the best so
  far, the trend and `found`; `focus` is null before the first start.
- `set_finder_focus` takes `on` and nothing else; it starts or stops the aid as
  `POST` and `DELETE /api/finder-camera/focus` do and answers `{"focus": ...}`,
  or the route's refusal as a tool error. It moves nothing and changes no
  setting: the observer turns the focuser.
```

After the `DELETE /api/imaging/align` row of the route table:

```markdown
| `GET /api/finder-camera/focus` | `{"focus": ...}`: the finder camera's focus aid, null before the first start. |
| `POST /api/finder-camera/focus` | JSON `{}`. Start the finder camera's focus aid; answers 200 with `{"focus": ...}`, or 400, 409 or 415 with a JSON `error`. Same-origin writes only. |
| `DELETE /api/finder-camera/focus` | No body. Stop it and answer 200 with `{"focus": ...}`. Same-origin writes only. |
```

Before `## Calibration frames`:

```markdown
## Focusing the finder camera

The Tools page's Focus block focuses either camera. For the imaging camera it
is the Imaging page's Focus block, the same one, under the imaging camera's
lease (see Focus). For the finder camera it is the finder camera's own focus
aid: `POST /api/finder-camera/focus` starts it, `DELETE` stops it, and `GET`
reads it.

Set a fixed exposure and gain for the finder camera on Setup first. The aid
refuses to start while either is automatic, and a run ends `failed` when the
frames it measures change exposure or gain: the scope node's automatic
exposure steps whenever the star count leaves 25 to 40, which turning the
focuser does, and the star count beside the reading means something only at
one exposure. After changing either, start the aid again.

It asks the scope node for one raw frame after another (the raw frames of
recordings, one star frame's pixels each, `width × height` bytes on the scope
link) and measures each at the stars the node found in that frame: the median
half-flux diameter, in the finder camera's pixels, of at most 15 of the
brightest stars whose box of 33 pixels lies inside the frame and holds no
other star, and whose own pixels are all under 250. Each star's light is
summed in circles round its centre a fifth of a pixel at a time, so the
reading follows a star smaller than two pixels and does not jump as a star
moves across the pixel grid. A frame with fewer than five such stars says
nothing (`too_few_stars`), and one whose median is above 20 pixels is far out
(`far_out`), as for the imaging camera. The readings go through the imaging
camera's own arithmetic: the median of the last three since the last change,
a reading more than 15% from it as a turn of the focuser, the trend, the best
so far and `found` once the reading has been far from the best, near it, far
and near again. Beside the reading the page shows how many stars the node
detected in that frame.

It does not stop the live solve, which goes on with the same frames; Track
may stay on, and keeps the stars still. Each reading costs one raw frame on
the scope link, 1,228,800 bytes for the finder camera's 1280 × 960, so run
the console beside the scope node on the Pi, as the field kit does: over a
slower link, such as a phone's hotspot, one raw frame can hold up the link's
heartbeat until the link drops. It ends `stopped` by a stop, or `failed` with
the words when the scope node is not connected, cannot send raw frames, sends
none for 15 s, or changes exposure or gain. Far out of focus the scope node
finds no stars at all, and the aid says so: turn the focuser slowly through
its range, watching the live preview, until stars show, then use the aid.

At its sharpest a star may be small enough for the scope node's detection to
take it for a hot pixel, as cedar-detect itself warns. If the detected count
falls as the reading reaches its lowest, come back to where the count is
highest within a tenth of the best: the live solve needs the stars more than
the last tenth of sharpness. After a change of optics (a spacer, a reducer)
stop the aid and start it again, since the best so far is the old optic's.
Sizes are in the finder camera's pixels, which compare exactly for one camera
behind one optic.
```

### The glossary (`GLOSSARY.md`, US3)

Three definitions change; their _Avoid_ lines do not.

- **Raw frame** (`GLOSSARY.md:165`): "The 8-bit gray pixels of one star frame, exactly
  as the scope node's detector read them, saved on request as a PNG next to that
  frame's line, or measured by the finder camera's focus aid."
- **Focus reading** (`GLOSSARY.md:221`): "The median size of the stars in an imaging
  frame's focus region, as a half-flux diameter: in arcseconds, or in sensor pixels
  until the console knows the scale; for the finder camera, of the stars the scope node
  found in one of its raw frames, in the finder camera's pixels. Smaller is sharper."
- **Focus aid** (`GLOSSARY.md:233`): "A loop of short exposures of the focus region, or
  of the finder camera's raw frames beside the live solve, each with its focus reading,
  the best so far and the trend, shown while the observer turns the focuser."

### The page (`crates/finder/web/src/`, US4)

`api.ts`:

```ts
// The finder camera's focus aid (051): sizes in the finder camera's pixels.
export interface FinderFocus {
  state: "running" | "ended";
  since: number;
  readings: number;
  cadence_ms?: number;
  last?: {
    frame_id: number;
    state: "reading" | "settling" | "too_few_stars" | "far_out";
    value?: number;
    stars: number;
    detected: number;
  };
  median?: { value: number; readings: number };
  best?: number;
  trend?: "better" | "worse" | "same";
  found: boolean;
  history: (number | null)[];
  ended?: "stopped" | "failed";
  error?: string;
}
export async function getFinderFocus(): Promise<{ focus: FinderFocus | null }>;
export async function postFinderFocus(): Promise<{ focus: FinderFocus | null }>; // body "{}", Content-Type: application/json
export async function deleteFinderFocus(): Promise<{ focus: FinderFocus | null }>;
```

each through the file's own `json` helper, as the other calls are.

`components/FocusTool.tsx` exports `finderStatus(focus: FinderFocus): string`, whose
texts are US4-S1's (033's `lastStatus` words, `crates/finder/web/src/components/FocusBlock.tsx:50`,
with "finder camera frames" for "imaging frames", and the `too_few_stars` hint chosen
by `last.detected < 5`), and
`FocusTool({ view, refresh }: { view: ImagingView | null; refresh: () => Promise<void> | void })`:
a `section` labelled by its `h2` `Focus a camera` (class `section-title`), then a
`label` holding the text `Camera` and a `select` with `aria-label="Camera"` and the
options `finder` (`Finder camera`, the default) and `imaging` (`Imaging camera`). With
the finder camera picked, its finder part reads `getFinderFocus` at once and every
1000 ms, shows `Reading the finder camera's focus aid.` until the first read, then the
paragraph "The finder camera's star size, in its pixels: smaller is sharper. It is
read beside the live solve and changes no setting. Turn the focuser one way until the
reading has passed its lowest and risen again, then come back to the lowest.", the
button, and the reading's lines as FR-014 lists them: the status (`role="status"`)
while running or after a stop only, `Stars detected <n>` as a plain paragraph under it,
033's trend words (`crates/finder/web/src/components/FocusBlock.tsx:40`); its bars are
033's drawing (the last 60 values, 36 units at the largest, `currentColor`, a gap for
`null`) with the finder camera's label. With the imaging camera picked it renders
`<FocusBlock view={view} refresh={refresh} />` when `view?.focus` is set, else the
quiet line of FR-015. `ToolsPage.tsx` renders `<FocusTool view={view} refresh={refresh} />`
after `<AlignmentBlock .../>` (`crates/finder/web/src/components/ToolsPage.tsx:26`).

`playwright.config.ts`: `"**/tools-focus.spec.ts",` on its own line after
`"**/tools.spec.ts",` in the default project's `testIgnore`
(`crates/finder/web/playwright.config.ts:44`) and the simulate project's `testMatch`
(`crates/finder/web/playwright.config.ts:63`). `tools-focus.spec.ts` sends
`PUT /api/session` `{"camera_settings": {"exposure_ms": 200, "gain": 1}}` through
`page.request` before it opens `/tools`, the values `goto.spec.ts`, `mcp.spec.ts`,
`simulate.spec.ts` and `tracking.spec.ts` set (`crates/finder/web/e2e/goto.spec.ts:126`,
`crates/finder/web/e2e/simulate.spec.ts:284`), and its `finally` sends
`DELETE /api/finder-camera/focus` through `page.request`, which answers at once whether
or not a run runs. It presses the start button with a 15 s bound, so a page without the
block fails at the button, not at the 60 s test timeout (run on US3's page).

## What was run and what was read

Logs are under the operator's probe folder, not in the repository; the rounds are
listed in order.

**Run, the draft's round** (2026-11-26), on `probe/051`, cut from c75a6e76:

- **The commits.** f6b1478b (US1), d8226cad (US2), 7c529cb3 (US3), c5e41445 (US4),
  6274d092 (the probe's own measurements, not a story: a test file of `#[test]`s that
  print), b6a3c5c4, 806bf2d4 and 9a60d558 (the draft's checks and words folded into the
  stories).
- **The gates** on c5e41445, 6274d092, 806bf2d4 and 9a60d558, all passing (nextest
  1,527 of 1,527 at 806bf2d4, Vitest 605 of 605 and `CI=1 npm run e2e` 34 of 34 at
  9a60d558).
- **Data.** The draft's measure (the joined pixels' half-flux diameter read between
  pixel radii) on the simulator's finder camera at 1000 ms: 0.93 to 0.97 times H(e) at
  focus errors 0 to 3; at 200 ms 0.94 to 0.77; on the owner's eight 3 s frames of the
  earlier camera, 2.78 to 3.10 px; ASTAP's `-analyse` on them, 3 to 7 stars; 033's `Aid`
  over the owner's four focus logs, the 3 s logs settling 95 of 104 and 44 of 62, the
  1 s logs 21 of 224 and 15 of 214; the star count on the simulator flat at 36 from
  focus error 0 to 2 at 200 ms; hot clusters of the owner's dark frames 1.9 to 2.4 px.

**Run, the fold of the cynical review's round 1** (2026-11-26), on `probe/051`:

- **The commits.** 3660e086 (US1 and US2: the sub-pixel measure, saturation on the
  star's own pixels, drawn stars at three phases and US1-S6, c75a6e76's render pinned
  by its hash; the exposure refusal and end, the panic end, `last.detected`, the
  same-origin and exposure scenarios), 823b297e (US3: the README's section and the MCP
  descriptions), 43c7ef94 (US4: 033's words, the detected count, the stop status only
  after a stop, the browser test's fixed exposure and API stop, `dist` rebuilt),
  1ae33fea (US2-S1's detected count above the stars measured) and 1efd1bc3 (US1-S4's
  200 ms reading at e = 2).
- **The gates.** On 43c7ef94 every gate: `cargo fmt --check`, clippy with
  `-D warnings`, both `cargo doc` runs, nextest of the whole workspace (1,529 of 1,529:
  the 1,512 existing, the stories' 13 and the probe's own 4), the doc tests,
  `npm ci --offline`, the typecheck, Prettier, Vitest (605 of 605), the build, the dist
  check (clean) and `CI=1 npm run e2e` (34 of 34, `tools-focus.spec.ts`, 050's
  `tools.spec.ts` and the specs after them among them). Every gate again on the tip,
  1efd1bc3.
- **Each story's tests fail before it.** US1's file on c75a6e76's source: the console
  binary does not build. US2's five on US1's source: no route, so the `GET` answers no
  JSON. US3's two on US2's source: the tools are unknown, and the README has no section.
  US4's Vitest files on US3's source: `./FocusTool` does not resolve, and US4-S5 finds
  no region `Focus a camera`. And on the draft's own code of each story, the fold's
  checks fail: US1-S1 (its phases), US1-S2 (its saturated pixels), US1-S4 (its hash and
  200 ms reading) and US1-S6 on the draft's measure; US2-S1 and US2-S3 (no `detected`),
  US2-S4 (no automatic refusal) and US2-S5 (no end) on the draft's run; US3-S2 on the
  draft's README; US4-S1 to S3 on the draft's page.
- **Mutations, each run against its story's tests.** The sub-pixel parts off (each
  pixel at its centre) fails US1-S1, US1-S3 and US1-S4; equal distances read part by
  part fails US1-S6; saturation over the whole box fails US1-S2; the circle always
  `BOX_PX − 1` fails US1-S2; the centre the box's centre pixel fails US1-S1 and US1-S6;
  only the star's own pixels summed fails US1-S4 (its 200 ms reading); `render`'s
  Gaussian 1 % wider fails US1-S4 (its hash); the automatic refusal off and the
  same-origin rule off on `POST` each fail US2-S4; `detected` not carried fails US2-S1;
  the exposure rule off fails US2-S5; the README's sentence off fails US3-S2; and the
  page's detected line off, its hint without the count, and the stop status on a failed
  run fail US4-S2, US4-S1 and US4-S3. The start pixel taken as the last on a tie, and
  `render`'s stamp radius 9 at `PSF_SIGMA_PX` (equal pixels), fail nothing (§ Read from
  the diff).
- **The measure, on data.**
  - Drawn stars integrated over each pixel, noise-free, six at a time, as the measure
    rebuilt in probe round 1's fold reads them (below): at σ 1.2, 2.0 and 3.0, 1.024 to
    1.060, 1.013 to 1.022 and 1.002 to 1.011 times 2·√(2 ln 2)·σ over phases 0 to 0.5 of
    a pixel (the draft's: 0.950 to 1.119 at σ 1.2 and 2.0); at σ 0.5 and 0.8, 1.233,
    1.493 and 1.577 px and 2.124, 2.111 and 1.984 px at phases 0, 0.25 and 0.5, a spread
    of 28 % and 7 % (the draft's: 0.647 to 1.414 px and 1.414 to 2.000 px, 119 % and
    41 %); at σ 0.4, 0.938 to 1.639 times, a spread of 75 %. Fifteen stars at random
    phases, the mean over 20 frames of each frame's reading: 1.18, 1.29, 1.35, 1.52,
    1.70, 2.11, 2.54 and 2.98 px at σ 0.3, 0.35, 0.4, 0.5, 0.6, 0.8, 1.0 and 1.2, varying
    0.3 % to 7.7 % from frame to frame.
  - The review's model of this optic (40 mm f/4, 3.75 µm, 4.83″ a pixel, 8-bit, shot
    and read noise, 0.15 px of jitter, σ 0.33 px at focus, a fixed exposure giving 29
    detected stars at focus; 40 frames each, the circle's margin 1 px as in the code):
    the reading 1.29, 1.35, 1.53, 1.80, 2.19 and 3.06 px at 0, 10, 21, 32, 43 and 64 µm
    of focus error, moving 3.3 %, 4.0 %, 2.2 %, 1.4 %, 1.7 % and 1.4 % from frame to
    frame, 033's `Aid` settling 38 of 40 frames at each, every frame after the first two
    (the draft's measure: 17.1 % and 15.5 % inside the zone, settling 1 and 10); at twice
    that exposure, 5.1 % and 30 of 40 at focus; at σ 0.5 and 1.0 px, 1.5 % to 3.5 %,
    settling 38 but at σ 1.0 and 64 µm (31). A fixed circle of 6 px: 6.2 % and 30 of 40
    at focus. Thirty stars measured instead of 15 (at twice the exposure, margin 2 px):
    4.7 % against 5.3 % at focus. The circle's margin of 1, 2 and 3 px over 30 frames:
    3.7 %, 4.5 % and 5.9 % at focus. At one focus, half to four times the exposure moved
    the reading at most 3.5 %. The detected count at that exposure: 29.4, 28.6, 25.8,
    21.0, 16.4 and 8.8.
  - The simulator's finder camera, as the rebuilt measure reads it: 1.021, 1.029, 1.016,
    1.003 and 0.995 times H(e) at focus errors 0, 0.5, 1, 2 and 3 at 1000 ms with 15
    stars each; at 200 ms 1.024 to 0.912 (e = 2: 0.980 against 1.003 at 1000 ms, 2.3 %);
    detected stars 142 at focus error 0, 141 at 1 and 67 at 3, at 1000 ms, and at most
    one from focus error 4 (200 and 1000 ms) or 5 (500 ms) on.
  - The owner's eight 3 s frames of 2026-11-09 from the earlier finder camera (outside
    the repository): 3.16 to 3.64 px with 15 stars each; that night's 1 s and 3 s logs
    measured 12.6 and 11.4, and 15.0 and 13.9, stars a frame on average; hot clusters of
    the dark frames read 2.02 to 2.46 px.
  - c75a6e76's `render`, compiled with c75a6e76's own noise source into a scratch test
    (not committed), for the simulator's 147 stars at 1000 ms, gain 1, seed 13 and
    frame 0: 1,584,128 bytes, FNV-1a 64 `0xb1d308b97a09eb40`, equal to the probe's.
  - The measure's cost: 0.23 % of the simulator's 500 ms frame interval on a 1456 × 1088
    frame of 141 centroids, on the probe's host; not measured on the Pi.
  - cedar-detect's hot-pixel rule (`reference-solutions/cedar-detect/src/algorithm.rs:1178`:
    a pixel is hot when its two neighbours in its row hold at most a quarter of its
    excess, applied to the full-resolution pixels of each binned candidate,
    `reference-solutions/cedar-detect/src/algorithm.rs:1112`), on a pixel-integrated
    Gaussian: at σ 0.33 px the neighbours hold 0.149 of the centre on a pixel's centre,
    and the rule holds within 0.20 px of it, two fifths of the stars' phases (the review
    said "about 0.15 px" and "a third"), for a star whose other pixels of its binned
    block are below the detection's line; at σ 0.30 px, 0.24 px; from σ 0.40 px on,
    never.

**Run, probe round 1 from this text** (2026-11-26), on `probe/051-r1`, cut from
9b139bac (this trio over c75a6e76), each story built from spec.md, plan.md and tasks.md
alone:

- **The commits.** a946cd3f and 8606f067 (US1's tests and story), af5ff92f and a65202a5
  (US2's), 531600ac and e9f16660 (US3's), 062e3085 and 77491a47 (US4's, `dist`
  rebuilt); each story wrote only its `writes`.
- **Each story's tests fail before it**, as T001, T005, T009 and T013 say; US4-S6, then
  without its 15 s bound, waited at the button for the whole 60 s test timeout.
- **The gates** on 77491a47, every one passing: nextest 1,525 of 1,525 (the 1,512
  existing and the stories' 13), Vitest 605 of 605, `CI=1 npm run e2e` 34 of 34 and the
  dist check clean; each story's focused commands three times green.
- **The defects**: 0 blocking and 10 clarity: FR-002's coordinates and exact ties, which
  made a reading depend on a star's place in the frame (spec.md § Clarifications);
  claims no scenario tested, in US1-S2, US1-S4, US2-S4, US2-S5, US3-S1, US4-S1, US4-S2
  and US4-S4; this plan's § Read from the diff left incomplete; and the words of US4-S6,
  T014 and US1-S3. Each is folded below.

**Run, the fold of probe round 1** (2026-11-26), on `probe/051-r1` over 77491a47:

- **The commits.** 1a87ce8a (US1: `measure`'s offsets from the box's centre pixel and
  ties within 10⁻⁹ px; US1-S1's six sizes and checkerboard, US1-S2's star at column 640
  and pixels 8 right and 16 below, US1-S4's NaN and US1-S6's bound of 1.32), 9b691e75
  (US2-S4's rig without camera settings and its run still running 10 s into silence,
  US2-S5's change of gain), d577c82d (US3-S1's `{"x": 1}` and automatic gain) and
  9ef38442 (US4-S1's 2 of 5 detected, US4-S2's 60 bars, US4-S4's paragraph and US4-S6's
  15 s at the button).
- **The measure rebuilt.** US1-S6 reads 1.2332, 1.4928 and 1.5766 px at σ 0.5 and
  2.1242, 2.1110 and 1.9839 px at σ 0.8 at phases 0, 0.25 and 0.5, spreads 1.278 and
  1.071, as an independent build of FR-002 does; cedar's coordinates with ties within
  10⁻⁹ px read the same. The spreads at σ 0.5 and 0.8 of the broken builds: whole pixels
  5.838 and 1.465; the crossing at the nearest part 1.360 and 1.046; ties read part by
  part 1.360 and 1.071; ties joined at phase 0 and split at 0.25 and 0.5, 1.395 and
  1.056; cedar's coordinates with exact ties 1.284 and 1.077. So US1-S6's bound at σ 0.5
  is 1.32, about 3 % from 1.278 and from 1.360, and at σ 0.8 stays 1.1, which only whole
  pixels exceed. US1-S1's worst reading is 6.0 % of its 8 % (σ 1.2, phase 0.5); its six
  sizes read 1.3 % of 3 % from their oracle and its checkerboard 0.08 % of 1 % from the
  plain frame (the lower middle edge value +1.6 %, the upper −2.0 %); US1-S3's discs
  −1.0 %; US1-S4's reading at e = 2 and 200 ms, 2.3 % of 4 %.
- **Mutations, each run against its story's tests, each failing a new step.** The six
  sizes' upper or lower middle, and the box edge's lower or upper middle, fail US1-S1;
  the edge mean, a margin of 2 px, the circle always 15 px and the box check off by one
  fail US1-S2; the setter taking NaN, and a frame taken before the render, fail US1-S4;
  whole pixels, the nearest part and both broken tie rules fail US1-S6; absent settings
  taken as fixed, and `RAW_WAIT` 5 s, fail US2-S4; a change of gain not checked fails
  US2-S5; `set_finder_focus` ignoring the session's settings, and its checks swapped,
  fail US3-S1; the hint at five detected, 600 bars and a look-alike imaging block fail
  US4-S1, US4-S2 and US4-S4.
- **US4-S6 on US3's page** (that `dist` built into the release binary): it fails at the
  button at its 15 s bound and is reported there.
- **The gates** on 9ef38442, every one passing: `cargo fmt --check`, clippy with
  `-D warnings`, both `cargo doc` runs, nextest of the whole workspace (1,525 of 1,525),
  the doc tests, `npm ci --offline`, the typecheck, Prettier, Vitest (605 of 605), the
  build, the dist check (clean) and `CI=1 npm run e2e` (34 of 34); and each story's
  focused commands, US2's and US3's three times before their commits.
- **The sizes**: § Plan traps 2.

**Read, not run:**

- a scope node built before 025 answering `UNIMPLEMENTED`, any other error of the raw
  request, and a raw frame of the wrong length (025's words, copied);
- the link dropped between frames, and the frames watch ending;
- a raw frame whose star frame does not come within 15 s, skipped;
- a measure that panics, ending the run `failed`;
- two raw requests at once, the aid's and a recording's, each answered from the same
  pixels (`crates/scope-node/src/service.rs:351`);
- Track on, or a GoTo, during a run;
- the console stopping mid-run;
- a stored exposure outside the camera's range, sent to the node as automatic
  (`crates/finder/src/link.rs:407`);
- the link's load over anything but the simulator's in-memory duplex, and a phone's
  hotspot dropping the link;
- an optic moving the focal plane, so that the frames after it are out of focus;
- every claim of § Outside the factory, and the real finder camera behind the SV165 at
  night.

## Read from the diff, not from a test

Each passes every scenario built the other way (probe round 1 and its fold, § What was
run); the judge reads it in the diff.

- FR-002's start pixel, the first in reading order on a tie: no drawn or simulated star
  ties there.
- FR-002's rule that a star whose start pixel is not above the threshold is not
  measured, and the noise floor of 0.5: every star a scenario draws has such a pixel,
  and the drawn frames have no noise.
- FR-002's boundaries: at most `BOX_PX` in the isolation, more than the level for a
  star's own pixels, `SATURATED` or above, and reaching T/2 at the half-flux crossing.
- FR-002's offsets and ties: cedar's own coordinates with exact ties read US1-S6's
  spread at σ 0.5 as 1.284 against the rule's 1.278, too close for a bound (ties read
  part by part, and the nearest part, read 1.360 and fail it).
- FR-001's stamp radius: 9 at `PSF_SIGMA_PX` draws the same pixels, and a radius held at
  8 above it differs only under US1-S5's noise; and `render` calling
  `render_with_sigma`, since c75a6e76's body kept draws the same pixels.
- FR-005's pairing: a raw frame measured at the latest kept star frame, or
  `last.detected` taken from it, since the latest is the raw frame's own in every
  scenario; and trap 10's watching while the raw frame is on its way.
- FR-004's order of a running aid before a missing node: no scenario has a run whose
  link is gone.
- FR-015's `FocusBlock` itself rather than a copy of its heading, button and words
  (US4-S4).
- The README's rows and section and the glossary's entries (FR-011), beyond US3-S2's
  sentences, paragraph and headings.
- FR-007's mappings of `UNIMPLEMENTED`, another error, a wrong length, the link dropped
  and a panic (above).
- FR-008's "changes no setting" beyond US2-S1's and US2-S3's `camera_settings` checks,
  "reads the session's camera settings only at its start", and "writes no file".
- The cadence (`cadence_ms`) and `since`, which no scenario pins.

## Shapes that existing tests pin

- **The simulator's pixels.** `crates/finder/tests/motion/tracking/pi_loop.rs:235`
  calls `render::render` with its six arguments: the signature stays, and its pixels
  are c75a6e76's (US1-S4 pins their hash; the motion binary passed, run). 033's imaging
  focuser and its tests are untouched (run).
- **033's aid.** `crates/finder/tests/console/imaging_focus_aid.rs`,
  `imaging_focus_arithmetic.rs`, `imaging_focus_far_out.rs` and the other
  `imaging_focus_*` files pass unchanged: 033's module is used, not changed (run).
- **025's raw frames.** `crates/finder/tests/console/recording_raw.rs` and
  `crates/finder/tests/motion/recording_raw.rs` pass unchanged (run).
- **The README's MCP count.** `crates/finder/tests/console/imaging_run_docs.rs:139`
  asserts "sixteen tools of its own" (`crates/finder/README.md:88`): the new tools are
  a paragraph of their own and that sentence is not touched (run).
- **050's README check.** `crates/finder/tests/mcp/imaging_align.rs:200` finds the
  finder alignment's paragraph, its two entries before `### MCP under WSL2`, and
  `## Finder alignment` before `## Calibration frames`: the new paragraph follows the
  alignment's entries, and the new section follows `## Finder alignment` (run).
- **MCP's tool list.** No existing test counts the tools (run: the MCP binary passed
  with two more).
- **The Tools page.** `ToolsPage.test.tsx` and `tools.spec.ts` (050) pass unchanged with
  the second block: 050's locators of the alignment are held to its region (run:
  Vitest and Playwright, above). `FocusBlock.test.tsx` (033) passes unchanged, and
  `FocusBlock.tsx` is not edited (run).
- **The simulated console's state.** `tools-focus.spec.ts` sorts before
  `tools.spec.ts` in the simulate project; it sets the camera settings to 200 ms and
  gain 1, the values the specs before it last set (`crates/finder/web/e2e/simulate.spec.ts:284`), starts no
  imaging camera focus aid, takes no lease and stops its own run before it ends, so the
  specs after it see the server as before (run: 34 of 34).
- **`playwright.config.ts`.** The two entries put `tools-focus.spec.ts` in the simulate
  project only (run).

## Decisions and the alternatives rejected

The ten owner decisions, all open, are in spec.md § Owner decisions, each with its
recommendation. Below them:

| Decision | Chosen | Rejected |
|---|---|---|
| Where the module lives | `crates/finder/src/finder_focus.rs`, beside `recording.rs`, the other reader of raw frames | under `imaging/` (the imaging camera's module; the finder camera is not an imaging camera) |
| The size of a star | the half-flux diameter in sub-pixel circles of the box's pixels less background, round the centre of the joined pixels above three noise levels | the joined pixels alone read between pixel radii (steps with the phase below σ 1.2 px; reads with the exposure); every pixel of the box above background, unbounded (a hot pixel or a neighbour's wing counts: dark frames read 8 to 20 px so) |
| The circle | one pixel past the farthest joined pixel, at most `BOX_PX − 1` | a fixed radius (a focused star of a pixel in a circle of six carries that circle's noise: 6.2 % from frame to frame on the model against 3.3 %); a wider margin (2 and 3 px: 4.5 % and 5.9 %) |
| The parts of a pixel | 5 × 5, SEP's own default | 8 × 8 (2.3 % at σ 1.2 against 3.2 % across the phases, for 2.6 times the parts) |
| The stars measured | the node's centroids, isolated, unsaturated in their own pixels, the 15 brightest | a detection of the console's own (a second detector beside the node's, which the live solve uses); every centroid (crowded and saturated ones bias the median); 30 (4.7 % against 5.3 % at focus on the model, little gain for twice the work) |
| Saturation | the star's own pixels | the whole box (a hot pixel at 250 within 16 px drops a good star) |
| A raw frame's star frame | the frames seen while it was on its way, the last eight, matched by id | the watch's latest frame (it may already be the next frame's) |
| One exposure for a run | refuse an automatic exposure or gain at the start; end on a change | restart the arithmetic, hold the exposure, or any exposure (spec.md decision 3) |
| The measure's thread | `spawn_blocking`, a panic ending the run | the async runtime (a 1.2 MB frame's loops on a runtime thread) |
| The run's lifetime | a task holding a `Weak`, aborted by a stop | a task holding the state (a stopped run's task keeps it alive) |
| The defocused simulator | `render_with_sigma`, called by `render` at 1.2 px | a new argument to `render` (an edit of an existing test's call) |
| The display's history | the page's own bars, 033's drawing with the finder camera's label | importing `FocusBlock`'s `History` (an edit of `FocusBlock.tsx`, FR-017) |
| The story cut | four stories, US2 the largest at 53.7 % (§ Plan traps 2) | three, with MCP in the run's story (about 72 % at the overrun) |

## User Story 1 - The measure and the simulator's focuser

`measure`, the simulator's focus error and `render_with_sigma`, with their test file.
No route, no view.

## User Story 2 - The run

The run, its view, the routes and `AppState::finder_focus`, over US1's `measure`, with
the exposure rule. Its tests turn US1's focuser.

## User Story 3 - MCP and the words

The two tools, the README and the glossary, and the README's check.

## User Story 4 - The page

`FocusTool`, its line in `ToolsPage`, the browser test, the two config entries and the
rebuilt `dist`.

## Verification (every story, before proposing)

- **The factory gates** (`ergane.yaml` at c75a6e76):
  - `lint` = `cargo fmt --all --check`,
    `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`,
    `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features --locked`
    and `RUSTDOCFLAGS="-D warnings" cargo doc -p finder --no-deps`;
  - `test` = `cargo nextest run --workspace --all-targets --all-features --locked`,
    `cargo test --workspace --locked --doc`, and then, in `crates/finder/web`,
    `npm ci --no-audit --no-fund`, `npm run typecheck`, `npm run format:check`,
    `npm test`, `npm run build`, `git status --porcelain -- dist | (! grep .)` and
    `CI=1 npm run e2e`.

  Every story runs both, and the judge sees their results.
- **CI in the merge queue** runs every required `ci.yml` job.

## Prerequisites before this spec is flipped to `ready`

1. **The earlier specs have landed:** every spec in `depends_on_landed`. Met: each is
   on `main` at c75a6e76.
2. **`main` is still as read.** Every citation here was read at c75a6e76. Met: `main`
   is c75a6e76 at probe round 1's fold. If `main` moves in a file § Project Structure
   names, or in a test § Shapes names, re-read them and re-run the validator.
3. **The owner decisions are settled and the owner approvals given** (spec § Owner
   decisions, § Owner approvals). Not met: all ten are open; it waits for the owner's
   answers at the flip ask.
4. **Probe builds.** A probe round builds the stories from this text alone, and its
   defects are folded. Met: round 1 built the four stories from this text alone on
   `probe/051-r1` (tip 77491a47, every scenario and gate passing, 0 blocking and 10
   clarity defects), and its fold proved each change on that code (tip 9ef38442, every
   gate passing; § What was run and what was read).
5. **The page half has run once.** Met: `CI=1 npm run e2e` 34 of 34 on round 1's tip
   and on its fold's (§ What was run and what was read).
6. **No operator-staged files are needed.** Met: no story reads a file outside the
   repository; the probe's measurements of the owner's frames are not a story.
7. **The real optic's night series, only if the owner picks decision 10 (a).** It waits
   for that answer: under (a), § Outside the factory step 1 comes before the flip, and
   its results are folded here; under (b), recommended, it is the first step after the
   landing.

## Outside the factory

Written, not run. The owner's, at night, with the ASI120MM Mini behind the SV165's 40 mm
f/4 objective (about 1.72° by 1.29°, with the database built for its field, 047).

1. **The night series (spec.md owner decision 10).** On a tripod near Polaris, where a
   star trails under a pixel a second, with the finder camera's exposure fixed on Setup
   at what the automatic control had chosen: find the best focus roughly, then take 5 to
   7 focuser positions through it about a twentieth of a turn apart, and at each, with
   the aid running, about 10 readings and one raw frame request of 10 frames on a
   recording (025, at most 10 a request). Note at each position the readings, the
   detected count and whether 033's arithmetic settled. The design changes, in a
   follow-up spec, only if: the reading at the best position moves more than 7 % from
   frame to frame, or 033's `Aid` replayed over its 10 readings settles fewer than 4 of
   the 8 that can settle; no fixed exposure gives five measurable stars with the
   brightest under 250; or the detected count at the sharpest position is below two
   thirds of its highest a little off it.
2. **By day, a rough focus.** Point the guide scope at the most distant object in
   view, open Setup's live preview, and turn the SV165's helical focuser until its
   edges are sharpest. Note the focuser's position. Neither the aid nor the live solve
   reads a daytime scene.
3. **At night, find stars.** Point at a field rich in stars well above the horizon.
   On Setup, check that the finder camera's star frames hold stars; if they hold none,
   turn the focuser slowly through its range from the day's position, watching the
   preview, until stars show. The live solve need not succeed yet.
4. **Fix the exposure.** On Setup, set the finder camera's exposure and gain to fixed
   values: the ones the automatic control has chosen are a good start. The aid refuses
   to start while either is automatic. If the detected count is low at the start, a
   longer fixed exposure; if `too_few_stars` says the rest are saturated, a shorter one;
   then start the aid again.
5. **Start the aid.** Open **Tools**, **Focus a camera**, **Finder camera**, **Start
   the finder camera's focus aid**. Wait for `Median of the last 3 finder camera frames`;
   the large number is the finder camera's star size in its pixels: smaller is sharper.
6. **Turn, wait, read.** Turn the focuser a small step one way, then take your hand off:
   the next readings say `Settling after a change`, then the median of three again,
   and the trend line says `Sharper` or `Softer than before the last change`. Keep
   turning the same way while it says `Sharper`. Once it says `Softer` twice, turn back
   in smaller steps. `Passed through focus and came back to the best so far.` says that
   the reading has been far from the best, near it, far again and near again: stop
   there. `Best so far` is the target; a reading within a tenth of it is as good as the
   night allows. Watch `Stars detected`: if it falls as the reading reaches its lowest,
   come back to where it is highest within a tenth of the best.
7. **Lock it.** If the focuser has a lock, tighten it with the aid running and watch
   the reading stay near the best; if it rises, loosen, come back and lock again. Note
   the focuser's position, then **Stop the finder camera's focus aid**.
8. **After a spacer, or any change of optics.** Stop, start again, refocus: the best so
   far is the old optic's, and a change that moves the focal plane leaves the frames out
   of focus until refocused (steps 3 to 7).

## Plan traps

They bind every story.

1. **Gate evidence or nothing.** The judge credits only gate results and the story's
   diff. *Check:* every Then names a committed test the `test` gate runs.
2. **One evidence budget.** *Check:* each story's whole diff stays under 60 % of
   65,536 B (39,321 B), measured with
   `git diff <base> -- . ':(exclude)crates/finder/web/dist' | wc -c` from the previous
   story's tip, on code built from the trio. Probe round 1, built from this text (US1
   from 9b139bac, each later story from the story before), with each of its fold's
   commits added to its story: US1 26,350 + 7,627 = 33,977 B (51.8 %), US2 29,952 +
   5,255 = 35,207 B (53.7 %), US3 19,362 + 1,992 = 21,354 B (32.6 %) and US4 22,161 +
   3,172 = 25,333 B (38.7 %), `dist` 697 B more. A fold commit counts its context and
   removed lines; written once, the fold adds about 1,600, 700, 660 and 770 B. At the
   1.35 times a factory build has run over its probe on earlier specs: US1 about 45,900 B
   (70.0 %), US2 about 47,500 B (72.5 %), US3 about 28,800 B (44.0 %) and US4 about
   34,200 B (52.2 %): US1 and US2 over the 60 % line, every story under 64 KiB and far
   under the 128 KiB refusal.
3. **A spec fixed mid-epic needs a relaunch.** *Check:* no builder edits
   `specs/051-tools-page-focuses-either-camera/` or `.specify/`.
4. **An earlier story's test file is an existing test.** *Check:* US2 to US4 write no
   file of an earlier story's tests; US3's MCP rig is its own.
5. **No new support file.** *Check:* nothing new under `crates/finder/tests/support/`;
   the tests use `crate::support::live`'s `DB` and `START` as they are.
6. **An oracle copied from the code proves nothing.** *Check:* the tests write the
   Gaussian's and the disc's half-flux diameters, the hash of c75a6e76's pixels, the
   bounds, the places and the words as literals; none calls `measure` for an expected
   value or reads the module's constants.
7. **A negative assertion passes when nothing happens.** *Check:* US1-S2's equal
   readings stand beside `stars` 6; US2-S3's "no `last.value`, no `median`" beside
   `last.state` `too_few_stars` and `last.stars` under 5; US2-S4's `{"focus": null}`
   after the foreign writes beside their 403; US2-S2's `found` false beside the last
   reading's `found` true; US4-S3's no status beside its alert; US4-S4's "no button
   `Start the finder camera's focus aid`" beside 033's heading and button; the night
   screenshot's 0 beside the day one's count.
8. **The frame in flight at a turn was taken before it.** *Check:* US2-S2 and US2-S3
   read until the state they wait for holds, never the next frame alone.
9. **Paused time and the simulator's clock.** *Check:* every Rust test of this spec
   that awaits runs under `start_paused = true`, and every wait is bounded (120 s of
   paused time).
10. **The frames watch keeps only the latest.** *Check:* the run keeps the star frames
    it sees while a raw frame is on its way, from before it sends the request.
11. **The lock is never held across an await.** *Check:* `FinderFocus`'s mutex is a
    `std::sync::Mutex` taken only in `view`, `start`, `stop`, the end and the sample's
    push.
12. **The dist is rebuilt.** *Check:* US4 commits `crates/finder/web/dist` after
    `npm run build`.
13. **An unused item fails clippy.** *Check:* the rig's `pub` fields are each read by a
    US2 test (`node` by US2-S4); `FinderFocusArguments` carries
    `#[expect(dead_code, reason = ...)]`.
14. **rustdoc denies warnings.** *Check:* no doc comment holds an angle bracket outside
    backticks.
15. **Each worktree builds on its own.** *Check:* no story sets `CARGO_TARGET_DIR`.
16. **The README's MCP count and 050's README order are pinned.** *Check:* US3 leaves
    "sixteen tools of its own" as it is, and puts its paragraph after the finder
    alignment's entries and its section after `## Finder alignment` (US3-S2).
17. **The browser test runs on a shared server.** *Check:* US4-S6's locators are held
    to the region `Focus a camera`, but the page's heading and the top bar's night
    switch; it sets the camera settings the specs before it set, starts no imaging
    camera focus aid, and stops its run through `DELETE /api/finder-camera/focus` in its
    `finally`; its waits stay well under the 60 s test timeout
    (`crates/finder/web/playwright.config.ts:29`).
18. **Playwright's project flag takes `=`.** *Check:* the development command is
    `npx playwright test --project=simulate e2e/tools-focus.spec.ts`.
19. **"Finder" alone is the glossary's _Avoid_ for the camera.** *Check:* every message,
    label, button and route says "finder camera" (`/api/finder-camera/focus`,
    `Start the finder camera's focus aid`, `The finder camera's focus aid stopped.`).
20. **The aid refuses an automatic exposure.** *Check:* every rig that starts the aid
    fixes the session's camera settings first: the focus rig and the MCP rig with
    `{"exposure_ms": 500, "gain": 1}`, and the browser test with `PUT /api/session`.

## Risks

| Risk | Mitigation |
|---|---|
| The real objective, an achromat, and the real sky differ from the review's model, on which the scatter and settle figures rest. | Owner decision 10 names the night series and the results that would change the design; the code depends on no number of this optic. |
| At its sharpest, a faint star of under about σ 0.4 px is taken by the node's detection for a hot pixel (cedar-detect's own warning, `reference-solutions/cedar-detect/src/algorithm.rs:89`; the rule at `reference-solutions/cedar-detect/src/algorithm.rs:1178` holds within 0.20 px of a pixel's centre at σ 0.33 px, two fifths of the phases). | The detected count is shown beside the reading; the README and § Outside the factory step 6 say to come back to the highest count within a tenth of the best. The rule's arithmetic run on drawn stars; its effect on the node's detection, read, not run. |
| The sub-pixel measure spreads each pixel's light evenly over it, so a star much under a pixel reads about 1.2 px and its phase still moves it (28 % at σ 0.5 px between phases 0 and 0.5). | The median of 15 stars at their own phases rises with σ from 0.3 px (§ What was run); US1-S6 bounds the spread at σ 0.5 and 0.8. |
| Hot pixels the node takes for stars read about 2 px (the earlier camera's dark frames: 2.02 to 2.46 px), close to a focused star. | The median of 15 is held by the stars of a field rich in them; the node's detection (048) and its hot-pixel setting are the live solve's own; the real ASI120MM's frames are read, not run. |
| The simulator's noise repeats every second frame: a frame's stream starts at the seed plus the frame number times SplitMix64's step, and each draw adds that step (`crates/simulator/src/noise.rs:13`, `crates/simulator/src/noise.rs:19`), so frames n and n + 2 share their noise one draw apart (the review: 16,381 of 16,384 pixels equal). It hides frame-to-frame scatter from US2's tests. | The scatter is measured on the model instead (§ What was run); a follow-up spec fixes the simulator's noise, out of 051's scope (spec.md § Assumptions). |
| A camera whose frames report a measured exposure or gain, as the rpicam path's metadata does (`crates/scope-node/src/rpicam/metadata.rs:29`, `crates/scope-node/src/rpicam/metadata.rs:30`), could move a hair at fixed settings and end a run. | The finder camera is the ASI120MM through INDI (045), whose frames carry the exposure asked and the gain sent; read, not run. |
| Raw frames load the scope link beside the star frames. | The console runs beside the node on the Pi (§ Target Platform); one raw frame per reading, the node's own pixels; the link's other frames go on (run on the simulator; the real link's share, read). |
| A story grows past 60 % in the factory. | The largest measures 53.7 % on probe round 1's code with its fold, and US1 and US2 about 70 % and 72.5 % at the worst overrun seen (§ Plan traps 2); the judge refuses only at 128 KiB. |
| The finder camera's raw frames are 8-bit (045), so bright stars saturate early. | Stars with a saturated pixel of their own are left out (FR-002); the page says so with the count, and a shorter fixed exposure gives more unsaturated ones. |

## Refinement Record

Drafted on 2026-11-26 by the plate-solver operator seat's spec session against `main` at
c75a6e76, from a probe of that day on `probe/051` (a trial build of the four stories
with every scenario's test and both gates passing, each story's tests failing before it,
the measure's candidates measured on the simulator and on the owner's frames, ASTAP on
the same frames, 033's arithmetic replayed on the owner's logs, and the page measured in
a browser by day and by night), and a reading of 033's reading, arithmetic and block,
025's raw frames, the node's detection and star frames, 050's Tools page, the
simulator's finder camera, MCP's tools, the constitution, the glossary and the ADRs.
Folded the same day through a cynical review's round 1 (a blocker, six majors, fourteen
minors), with the probe extended to 1efd1bc3: a sub-pixel measure measured on drawn
frames at three phases, on the review's model of this optic, on the simulator and on the
owner's frames; one exposure a run; the detected count; a tenth owner decision; and
every gate, the fails-before and the mutations run again (§ What was run and what was
read). Probe round 1 then built the four stories from this text alone on `probe/051-r1`
(tip 77491a47: 0 blocking, 10 clarity defects), and its fold added four commits there
(tip 9ef38442): FR-002's offsets and ties, steps for the claims no scenario held, § Read
from the diff completed, and every gate run again. Writes come from the task text.
Shared writes name the spec story that last wrote the same file on `main`; within the
spec, each story's files written by an earlier one are ordered by `depends_on_merged`.

The ten owner decisions of spec.md are open; their questions, options and evidence are
spec.md § Owner decisions, and this record is written to their recommendations. Its
`open_questions` are empty so that the validator checks the rest of the record, as
050's draft did; the draft state and Prerequisite 3 hold readiness until the owner
answers, and a question left open then goes here. This record approves nothing; the
flip is the owner's.

```yaml
version: 1
reviewed_revision: c75a6e7677016d45a2c1a757cebdbe376f54bc81
stories:
  US1:
    tests:
      - path: crates/finder/tests/console/finder_focus_measure.rs
        status: new
        disposition: add
        assertion: "drawn 640 x 480 frames, background 10, each pixel the integral over its square of 200 exp(-r^2/2 sigma^2), six stars at the six places at phase 0, 0.25 and 0.5, sigma 1.2, 2.0 and 3.0, measured with their six centroids: each Sample::Reading with stars 6 and px within 8 % of 2 sqrt(2 ln 2) sigma; six stars at the six places of sigma 1.2, 1.6, 2.0, 2.4, 2.8 and 3.2: Reading stars 6, px within 3 % of the mean of 2 sqrt(2 ln 2) 2.0 and 2 sqrt(2 ln 2) 2.4; the sigma 3.0 frame at phase 0 with 2 added to each pixel whose column and row sum to an odd number (each box edge 64 pixels of 10 and 64 of 12): Reading stars 6, px within 1 % of the plain frame's"
        decision: "spec.md:US1-S1"
      - path: crates/finder/tests/console/finder_focus_measure.rs
        status: new
        disposition: add
        assertion: "a drawn sigma 2.0 frame of the six places, a star at (10.5, 240.5) over the edge, a star at (624.5, 240.5) whose box reaches column 640, two at (400.5, 400.5) and (410.5, 405.5), and one at (300.5, 420.5) of density 250 reaching 255, each with its centroid: Sample::Reading stars 6; the same frame with a pixel of 200 ten pixels right of and a pixel of 255 ten pixels below each of the six stars, and with a pixel of 200 eight pixels right of and a pixel of 255 sixteen pixels below each, no centroid: each Reading stars 6, px equal to the first within 1e-9"
        decision: "spec.md:US1-S2"
      - path: crates/finder/tests/console/finder_focus_measure.rs
        status: new
        disposition: add
        assertion: "four drawn stars of sigma 2.0 at the first four places: Sample::TooFewStars stars 4; a frame of background 10 alone and no centroid: TooFewStars stars 0; five uniform discs at the first five places, each pixel whose centre lies within 15 px of the place at level 60: Sample::FarOut stars 5, px within 5 % of 15 sqrt 2"
        decision: "spec.md:US1-S3"
      - path: crates/finder/tests/console/finder_focus_measure.rs
        status: new
        disposition: add
        assertion: "the simulator on DB, Site::default, VirtualClock at START, pushed to alt 50 az 120: finder_focus_error 0, then each e set reads back; render, before any frame is taken, of the 147 stars field_stars(1000.0, 1.0) gives at START, at 1000 ms gain 1 seed 13 frame 0, is 1584128 bytes with FNV-1a 64 0xb1d308b97a09eb40, c75a6e76's own, and render_with_sigma at PSF_SIGMA_PX the same bytes; SimCamera frames at 1000 ms gain 1 measured at the node's detect with DetectSettings::cedar: e 0, 1, 2, 3 each Reading stars 15 px within 10 % of H(e), each larger than the one before; e 2 at 200 ms within 4 % of e 2 at 1000 ms; e -3 px within 5 % of e 3's; e 8 TooFewStars under 5 stars; set_finder_focus_error(f64::NAN) panics"
        decision: "spec.md:US1-S4"
      - path: crates/finder/tests/console/finder_focus_measure.rs
        status: new
        disposition: add
        assertion: "Optics 64 x 64 otherwise default, one RenderStar at (32.5, 32.5) magnitude 8, frames 0 to 63 at 200 ms gain 1 seed 13: with the star no pixel at 250 or above; the mean sum of the star's pixels less the starless frame's above 50 at PSF_SIGMA_PX, and at sigma 3.0 within 5 % of it"
        decision: "spec.md:US1-S5"
      - path: crates/finder/tests/console/finder_focus_measure.rs
        status: new
        disposition: add
        assertion: "drawn frames of six stars at the six places at phase 0, 0.25 and 0.5, sigma 0.5, 0.8 and 1.2, each Reading stars 6: at each phase sigma 0.8 at least 1.1 times sigma 0.5 and sigma 1.2 at least 1.1 times sigma 0.8; the largest of the three phases at most 1.32 times the smallest at sigma 0.5 and at most 1.1 times at sigma 0.8"
        decision: "spec.md:US1-S6"
      - path: crates/finder/tests/motion/tracking/pi_loop.rs
        status: existing
        disposition: preserve
        assertion: "the tracking loop's rendered frames through render::render's six arguments, as before 051"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/console/imaging_focus_reading.rs
        status: existing
        disposition: preserve
        assertion: "033's focus reading from the simulated imaging solver and the imaging optics' focus error, as 033 words it"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "render::render draws exactly the pixels it drew at c75a6e76 (its signature unchanged)"
      - "the simulator's imaging focuser, set_focus_error, and 033's module are unchanged"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console -E 'test(finder_focus_measure)'"
        - "cargo nextest run -p simulator --locked"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - crates/simulator/src/camera.rs
      - crates/simulator/src/optics.rs
      - crates/simulator/src/render.rs
      - crates/finder/src/finder_focus.rs
      - crates/finder/src/lib.rs
      - crates/finder/tests/console/finder_focus_measure.rs
      - crates/finder/tests/console/main.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/simulator/src/optics.rs
        other_spec: 050-tools-page-aligns-finder-camera
        other_story: US1
        disposition: merge_order
        reason: "051 depends_on_landed 050, whose US1 last changed the simulator's optics (a3c98ccc)"
      - path: crates/simulator/src/render.rs
        other_spec: 013-live-finder
        other_story: US3
        disposition: merge_order
        reason: "051 depends_on_landed 013, whose US3 last changed the renderer (9511f164)"
      - path: crates/simulator/src/camera.rs
        other_spec: 013-live-finder
        other_story: US3
        disposition: merge_order
        reason: "051 depends_on_landed 013, whose US3 last changed SimCamera (9511f164)"
      - path: crates/finder/src/lib.rs
        other_spec: 047-console-uses-the-database-that-fits
        other_story: US1
        disposition: merge_order
        reason: "051 depends_on_landed 047, whose US1 last changed lib.rs (d3f8bd0f)"
      - path: crates/finder/tests/console/main.rs
        other_spec: 050-tools-page-aligns-finder-camera
        other_story: US3
        disposition: merge_order
        reason: "051 depends_on_landed 050, whose US3 last changed console/main.rs (6b70a4a2)"
    open_questions: []
  US2:
    tests:
      - path: crates/finder/tests/console/finder_focus_aid.rs
        status: new
        disposition: add
        assertion: "the focus rig, its session's camera_settings exposure_ms 500 gain 1, live: GET /api/finder-camera/focus 200 focus null; POST {} 200 with focus.state running, readings 0, found false, history []; the settled reading after 0 has median.value within 10 % of H(0), last.stars at least 5, last.detected above last.stars, history as long as readings; GET /api/session then reads the camera_settings read before the start and a pointing.seq above the one read before"
        decision: "spec.md:US2-S1"
      - path: crates/finder/tests/console/finder_focus_aid.rs
        status: new
        disposition: add
        assertion: "the focus rig, live, focuser at 3, started and its settled reading read: turned to 0 until a settled reading has trend better, to -3 until one has trend worse, to 0 until found true: the better and worse readings each found false; the last found true, state running, best within 10 % of H(0), median.value at most 1.10 times best"
        decision: "spec.md:US2-S2"
      - path: crates/finder/tests/console/finder_focus_aid.rs
        status: new
        disposition: add
        assertion: "the focus rig, live, started and settled: focuser at 10 until last.state too_few_stars: last.stars under 5, last.detected under 5, no last.value, no median, history ending in null, state running; set_optics fov 24 with the focuser at 0, the settled reading after n, n that view's readings: state running; DELETE 200 focus.state ended, ended stopped; 5 s later DELETE again 200 with the same readings and ended stopped; GET /api/session camera_settings as read before the start"
        decision: "spec.md:US2-S3"
      - path: crates/finder/tests/console/finder_focus_aid.rs
        status: new
        disposition: add
        assertion: "POST {} to the bare console 409 the scope node is not connected; to a live focus rig whose session was never given camera settings 409 the finder camera's exposure or gain is automatic: fix both on Setup; to the live rig a foreign write (Host localhost on port 8080, Origin http://elsewhere.example) of POST and of DELETE each 403 writes require the console's own origin and an allowed host; GET 200 focus null; x 1 400; {} as text/plain 415; PUT /api/session exposure_ms auto 200; {} 409 the finder camera's exposure or gain is automatic: fix both on Setup; PUT exposure_ms 500 200; {} 200; {} again 409 the finder camera's focus aid is already running; settled after 0, the node's frames stopped: 10 s later focus.state still running, then the run ends with focus.ended failed and focus.error exactly no raw frame arrived within 15 s"
        decision: "spec.md:US2-S4"
      - path: crates/finder/tests/console/finder_focus_aid.rs
        status: new
        disposition: add
        assertion: "the focus rig, live, started and settled: PUT /api/session camera_settings exposure_ms 250, read until ended; started again and settled: PUT camera_settings gain 2, read until ended: each run focus.ended failed, focus.error exactly the finder camera's exposure or gain changed during the run, readings at least its settled reading's; GET /api/session camera_settings.exposure_ms 250"
        decision: "spec.md:US2-S5"
      - path: crates/finder/tests/console/imaging_focus_aid.rs
        status: existing
        disposition: preserve
        assertion: "033's focus aid takes 031's lease, remembers and puts back the imaging settings, and words its states as 033 does"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/console/imaging_focus_arithmetic.rs
        status: existing
        disposition: preserve
        assertion: "033's Aid: median of three, the 15 % change, settling, trend, best and found, as 033 words them"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/console/recording_raw.rs
        status: existing
        disposition: preserve
        assertion: "025's raw frames saved beside a recording's lines, with 025's words for a node that cannot send them, a failed request and none within 15 s"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "033's Aid, Sample, Phase, Trend, MIN_STARS and FAR_OUT_PX are used, not changed"
      - "025's recording, LinkManager::raw and LinkManager::frames are used, not changed"
      - "the live loop and the session's camera settings are unchanged by a run"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console -E 'test(finder_focus_aid)'"
        - "cargo nextest run -p finder --locked --test console -E 'test(/imaging_focus|recording/)'"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs and the night of plan.md § Outside the factory"
    writes:
      - crates/finder/src/api.rs
      - crates/finder/src/finder_focus.rs
      - crates/finder/tests/console/finder_focus_aid.rs
      - crates/finder/tests/console/main.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/api.rs
        other_spec: 047-console-uses-the-database-that-fits
        other_story: US2
        disposition: merge_order
        reason: "051 depends_on_landed 047, whose US2 last changed api.rs (c15937e7)"
      - path: crates/finder/tests/console/main.rs
        other_spec: 050-tools-page-aligns-finder-camera
        other_story: US3
        disposition: merge_order
        reason: "051 depends_on_landed 050, whose US3 last changed console/main.rs (6b70a4a2)"
    open_questions: []
  US3:
    tests:
      - path: crates/finder/tests/mcp/finder_focus.rs
        status: new
        disposition: add
        assertion: "the bare console: set_finder_focus on true errs exactly the scope node is not connected; the MCP rig, its camera_settings exposure_ms 500 gain 1: get_finder_focus {} structuredContent focus null; set_finder_focus {} and x 1 each err on must be true or false, on true with x 1 errs set_finder_focus takes on and nothing else, get_finder_focus x 1 errs get_finder_focus takes no arguments; once live on true focus.state running, again errs the finder camera's focus aid is already running; get_finder_focus until focus.median.readings 3; on false focus.ended stopped; after PUT /api/session camera_settings gain auto, on true errs the finder camera's exposure or gain is automatic: fix both on Setup; tools/list holds both, get_finder_focus readOnlyHint true"
        decision: "spec.md:US3-S1"
      - path: crates/finder/tests/mcp/finder_focus.rs
        status: new
        disposition: add
        assertion: "crates/finder/README.md read as a file: it still holds The imaging camera has sixteen tools of its own; The Tools page's focus aid for the finder camera has two tools: comes after The Tools page's finder alignment has two tools: and before ### MCP under WSL2, with a list entry each for get_finder_focus and set_finder_focus between them; ## Focusing the finder camera comes after ## Finder alignment and before ## Calibration frames, and between them holds Set a fixed exposure and gain for the finder camera on Setup first."
        decision: "spec.md:US3-S2"
      - path: crates/finder/tests/console/imaging_run_docs.rs
        status: existing
        disposition: preserve
        assertion: "the README's MCP section still says sixteen tools of its own for the imaging camera"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/mcp/imaging_align.rs
        status: existing
        disposition: preserve
        assertion: "the finder alignment's MCP paragraph and its two entries come before ### MCP under WSL2, and ## Finder alignment before ## Calibration frames"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "the README's sentence The imaging camera has sixteen tools of its own is unchanged"
      - "033's and 050's MCP tools and their words are unchanged"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test mcp -E 'test(finder_focus)'"
        - "cargo nextest run -p finder --locked --test console -E 'test(docs)'"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - GLOSSARY.md
      - crates/finder/README.md
      - crates/finder/src/mcp/tools.rs
      - crates/finder/tests/mcp/finder_focus.rs
      - crates/finder/tests/mcp/main.rs
    no_write_reason: ""
    shared_writes:
      - path: GLOSSARY.md
        other_spec: 050-tools-page-aligns-finder-camera
        other_story: US4
        disposition: merge_order
        reason: "051 depends_on_landed 050, whose US4 last changed the glossary (338f984a)"
      - path: crates/finder/README.md
        other_spec: 050-tools-page-aligns-finder-camera
        other_story: US4
        disposition: merge_order
        reason: "051 depends_on_landed 050, whose US4 last changed the finder's README (338f984a)"
      - path: crates/finder/src/mcp/tools.rs
        other_spec: 050-tools-page-aligns-finder-camera
        other_story: US4
        disposition: merge_order
        reason: "051 depends_on_landed 050, whose US4 last changed mcp/tools.rs (338f984a)"
      - path: crates/finder/tests/mcp/main.rs
        other_spec: 050-tools-page-aligns-finder-camera
        other_story: US4
        disposition: merge_order
        reason: "051 depends_on_landed 050, whose US4 last changed mcp/main.rs (338f984a)"
    open_questions: []
  US4:
    tests:
      - path: crates/finder/web/src/components/FocusTool.test.tsx
        status: new
        disposition: add
        assertion: "finderStatus gives exactly: no last, Waiting for the first finder camera frame; a reading of 15 stars with 3 held, Median of the last 3 finder camera frames · 15 stars; 9 stars with 2 held, 2 of 3 finder camera frames since the last change · 9 stars; settling, Settling after a change; too_few_stars with 2 of 3 detected, No reading: 2 stars measured, and 5 are needed. Far out of focus the stars vanish: turn the focuser slowly until they show.; too_few_stars with 2 of 31 detected, No reading: 2 of the 31 stars detected could be measured, and 5 are needed: the rest are saturated, crowded or at the frame's edge.; far_out of 21.2, Far out of focus: stars about 21.20 px across; too_few_stars with 2 of 5 detected, No reading: 2 of the 5 stars detected could be measured, and 5 are needed: the rest are saturated, crowded or at the frame's edge."
        decision: "spec.md:US4-S1"
      - path: crates/finder/web/src/components/FocusTool.test.tsx
        status: new
        disposition: add
        assertion: "FocusTool with no imaging view and GET focus null: before the first read Reading the finder camera's focus aid.; Start the finder camera's focus aid sends POST /api/finder-camera/focus with Content-Type application/json and body {}; a running aid of median 2.83 of 3, last 15 stars of 31 detected, best 2.7, trend better, found true, history [5.9, 6.1, null, 2.8, 2.83] shows 2.83 px, the status Median of the last 3 finder camera frames · 15 stars, Stars detected 31, Best so far 2.70 px, Sharper than before the last change, Passed through focus and came back to the best so far. and the image Focus readings of the last 5 finder camera frames with 4 bars; after Stop the finder camera's focus aid, The finder camera's focus aid stopped.; rendered again over a running aid of 61 readings in history, the image Focus readings of the last 60 finder camera frames with 60 bars"
        decision: "spec.md:US4-S2"
      - path: crates/finder/web/src/components/FocusTool.test.tsx
        status: new
        disposition: add
        assertion: "an aid ended failed with the error no raw frame arrived within 15 s: the alert reads exactly The finder camera's focus aid stopped: no raw frame arrived within 15 s., the button Start the finder camera's focus aid, and no status"
        decision: "spec.md:US4-S3"
      - path: crates/finder/web/src/components/FocusTool.test.tsx
        status: new
        disposition: add
        assertion: "Imaging camera picked in the select Camera: with no view, The imaging camera's focus aid shows here once the imaging camera is ready.; with a view whose focus has unit px, 033's level-two heading Focus, its button Start focus aid and its words A focus reading is the size of the stars in the focus region: smaller is sharper., and no button Start the finder camera's focus aid"
        decision: "spec.md:US4-S4"
      - path: crates/finder/web/src/components/FocusToolPage.test.tsx
        status: new
        disposition: add
        assertion: "ToolsPage with no imaging view: the region Align the finder camera comes before the region Focus a camera in the document, and the select Camera reads Finder camera"
        decision: "spec.md:US4-S5"
      - path: crates/finder/web/e2e/tools-focus.spec.ts
        status: new
        disposition: add
        assertion: "the simulated console at 412 by 915, day: PUT /api/session camera_settings exposure_ms 200 gain 1 answers 200; /tools, Start the finder camera's focus aid in the region Focus a camera pressed within 15 s; within 20 s its status contains Median of the last 3 finder camera frames, a two-decimal px number and its readings' image show; scrollWidth at most 412 by day and by night; the day screenshot more than 10000 pixels off night red, the night one 0; Imaging camera picked shows Start focus aid within 15 s; Finder camera again and Stop the finder camera's focus aid, Start the finder camera's focus aid within 15 s; DELETE /api/finder-camera/focus in its finally"
        decision: "spec.md:US4-S6"
      - path: crates/finder/web/src/components/FocusBlock.test.tsx
        status: existing
        disposition: preserve
        assertion: "033's Focus block, its texts, buttons, record and verdict, as 033 words them"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/src/components/ToolsPage.test.tsx
        status: existing
        disposition: preserve
        assertion: "050's Tools page: Setup's Tools link, the heading Tools, the alignment's block and Back to console"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/e2e/tools.spec.ts
        status: existing
        disposition: preserve
        assertion: "050's finder alignment on the simulated console, by day and by night, its locators held to the region Align the finder camera"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "FocusBlock.tsx, AlignmentBlock.tsx, the Imaging page and Setup are unchanged"
      - "playwright.config.ts gains only its two list entries (owner approval 6)"
      - "dist is the build of its source"
    checks:
      development:
        - "npm test -- FocusTool"
        - "npx playwright test --project=simulate e2e/tools-focus.spec.ts"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml finder-page job and the night of plan.md § Outside the factory"
    writes:
      - crates/finder/web/dist
      - crates/finder/web/e2e/tools-focus.spec.ts
      - crates/finder/web/playwright.config.ts
      - crates/finder/web/src/api.ts
      - crates/finder/web/src/components/FocusTool.test.tsx
      - crates/finder/web/src/components/FocusTool.tsx
      - crates/finder/web/src/components/FocusToolPage.test.tsx
      - crates/finder/web/src/components/ToolsPage.tsx
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/web/dist
        other_spec: 050-tools-page-aligns-finder-camera
        other_story: US5
        disposition: merge_order
        reason: "051 depends_on_landed 050, whose US5 last rebuilt dist (c75a6e76)"
      - path: crates/finder/web/src/api.ts
        other_spec: 050-tools-page-aligns-finder-camera
        other_story: US5
        disposition: merge_order
        reason: "051 depends_on_landed 050, whose US5 last changed api.ts (c75a6e76)"
      - path: crates/finder/web/src/components/ToolsPage.tsx
        other_spec: 050-tools-page-aligns-finder-camera
        other_story: US5
        disposition: merge_order
        reason: "051 depends_on_landed 050, whose US5 made ToolsPage.tsx (c75a6e76)"
      - path: crates/finder/web/playwright.config.ts
        other_spec: 050-tools-page-aligns-finder-camera
        other_story: US5
        disposition: merge_order
        reason: "051 depends_on_landed 050, whose US5 last changed playwright.config.ts (c75a6e76)"
    open_questions: []
```
