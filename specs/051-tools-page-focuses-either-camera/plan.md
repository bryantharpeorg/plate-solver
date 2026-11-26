# Implementation Plan: The Tools Page Focuses Either Camera

**Branch**: `051-tools-page-focuses-either-camera` | **Date**: 2026-11-26 |
**Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/051-tools-page-focuses-either-camera/spec.md`

## Summary

The Tools page gains a second block, `Focus a camera`, with a select of the camera. For
the imaging camera it renders 033's own Focus block, unchanged, over the Tools page's
imaging view. For the finder camera it runs a new focus aid on the console: one raw
frame after another (025's `RawFrame`) is asked of the scope node while the live solve
goes on, each is measured at the centroids of its own star frame (the node's detection,
048) as the median half-flux diameter of at most 15 isolated, unsaturated stars, and
each measure goes as a `Sample` into 033's own `Aid`, which gives the median of three,
the trend, the best so far and `found`. The aid changes no setting, holds no lease and
keeps no record; a stop or a failure ends it. The simulator's finder camera gains a
focus error, so that the gates can turn its focuser.

Four stories: the measure and the simulator's focuser; the run, its routes and view;
MCP, the README and the glossary; the page. Each is under 39 % of 65,536 B on the
probe's code (§ Plan traps 2). One general rule removes every side effect the owner
would otherwise approve one by one: **the finder camera's aid adds no rule of its own
to 033's arithmetic and takes nothing from the live solve** (spec.md § Owner decisions).

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
  frames of the test's own making (US1-S1 to US1-S3) and on the simulator's finder
  camera (US1-S4), and the renderer's flux (US1-S5). US1-S1 to US1-S3 and US1-S5 under
  `#[test]`; US1-S4 under `#[tokio::test(start_paused = true)]`, since `SimCamera`
  sleeps for the exposure.
- `crates/finder/tests/console/finder_focus_aid.rs` (new, US2): the focus rig with
  `pub struct Rig { pub app: Router, pub sim: Simulator, pub node: Arc<SimNode> }`,
  `pub fn rig() -> Rig` and its helpers, and US2-S1 to US2-S4 under
  `#[tokio::test(start_paused = true)]`.
- `crates/finder/tests/mcp/finder_focus.rs` (new, US3): its own rig, as the MCP
  binary's tests build theirs, US3-S1 under `#[tokio::test(start_paused = true)]` and
  US3-S2 under `#[test]`.
- `crates/finder/web/src/components/FocusTool.test.tsx` (new, US4): US4-S1 to US4-S4;
  `crates/finder/web/src/components/FocusToolPage.test.tsx` (new, US4): US4-S5;
  `crates/finder/web/e2e/tools-focus.spec.ts` (new, US4): US4-S6 under Playwright's
  simulate project.

**Target Platform**: the console on the Pi or a desktop, the page in a phone's browser.

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
| I. Agreeing with cedar is correctness | Detection and the plate solver are untouched; the aid measures at the centroids the node's own detection (cedar-detect, 048) gives, and the live solve is the live loop's own. The measure is the half-flux diameter as the repository's survey sets it out (`docs/research/2026-11-15-imaging-camera.md:210`), checked against a Gaussian's own half-flux diameter and a uniform disc's; what rests on reading is listed in § What was run and what was read. |
| II. Never weaken a gate | No existing test, golden, fixture, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS` or `ergane.yaml` is edited. `playwright.config.ts` gains two list entries, an owner approval as 050's were. The only edits to existing test files are the `mod` lines of `crates/finder/tests/console/main.rs` and `crates/finder/tests/mcp/main.rs` (SC-002). The simulator draws exactly the pixels it drew before at focus error 0 (US1-S4, run), so no existing caller observes a change (run: nextest's every test passed on the probe's tip, § What was run). |
| III. Tests first, independent expectations | The oracles are a Gaussian's half-flux diameter, 2·√(2 ln 2)·σ, at the simulator's own star width, a uniform disc's, √2 times its radius, the drawn frames' own places and levels, 025's and 033's words, and the scenarios' words. No test calls `measure` for an expected value or reads `STARS_MEASURED`, `BOX_PX`, `SATURATED`, `THRESHOLD_NOISE` or `RAW_WAIT`. Time comes from a paused tokio clock and the simulator's virtual clock. |
| IV. Read the critical paths twice | None is touched: no `Cargo.toml` or `Cargo.lock` change, no `unsafe`, nothing under `proto/`, `crates/math-core`, `crates/pattern-database` or `crates/database-generation`, no `build.rs` or `.cargo/`. `crates/scope-link`'s proto is read, not changed. `crates/simulator` is not a critical path; its change is owner approval 1. |
| V. Proven open source first, with honest licensing | Looked for, in local sources only: the repository's survey of 2026-11-15, § 6 "Focus" (`docs/research/2026-11-15-imaging-camera.md:203`), with its sources on PHD2's half-flux code, SharpCap, Ekos and N.I.N.A.; ASTAP's `-analyse`, installed as `astap_cli` and already 033's measure behind the imaging solver's port; INDI's CCD simulator's focus simulation (`docs/research/2026-11-15-imaging-camera.md:95`); the vendored `reference-solutions/` (cedar-detect, cedar-solve), whose `StarDescription` gives a centroid, a peak, a brightness and a saturated count and no size (`reference-solutions/cedar-detect/src/algorithm.rs:503`); and the owner's own bring-up script, outside the repository, which prints a median half-flux radius. ASTAP was tried on the finder camera's frames and found too few stars (owner decision 1 (b)); a size from cedar-detect would be a change to the node and the proto (decision 1 (d)). What is adopted is the half-flux recipe itself, PHD2's as the survey gives it, written here in a few dozen lines with no copied source and no new licence (ADR 0003). 033's arithmetic is reused as it is. |

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

The code as it ran on `probe/051` (tip 9a60d558). Bodies not given here are the
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
/// A star with a pixel at or above this level in its box is not measured (US1).
pub const SATURATED: u8 = 250;
/// A pixel counts toward a star's flux only this many noise levels above the
/// background (US1).
pub const THRESHOLD_NOISE: f64 = 3.0;

/// One raw frame measured at its own star frame's centroids, the node's
/// detection (US1): a reading, or why there is none, as 033's `Sample` names
/// it. The centroids are brightest first; a star is measured when its box lies
/// inside the frame, holds no other centroid and no pixel at `SATURATED`, and
/// has flux; the first `STARS_MEASURED` such stars are measured.
pub fn measure(pixels: &[u8], width: u32, height: u32, centroids: &[Centroid]) -> Sample;
```

`Centroid` is `scope_link::proto::Centroid`; `Sample`, `MIN_STARS` and `FAR_OUT_PX`
are 033's (`crates/finder/src/imaging/focus/aid.rs:33`,
`crates/finder/src/imaging/focus.rs:16`, `crates/finder/src/imaging/focus.rs:21`). The
box is centred on the pixel holding the centroid (column `(x - 0.5).round()`, row
likewise). One star's size, as FR-002 gives it: the edge pixels' median as background;
1.4826 times their median absolute deviation, at least 0.5, as noise; the brightest
pixel within two pixels of the centroid, if above background plus three noise levels,
as the start of an 8-connected fill over the pixels above that threshold; the filled
pixels' flux above background, their flux-weighted centre, their radii from it sorted,
and the radius at which the running flux crosses half, interpolated linearly between
the two pixels' radii; twice that radius. A star whose box crosses the edge, holds
another centroid within `BOX_PX` in both axes, holds a pixel at `SATURATED` or more,
or has no pixel above the threshold is not measured and not counted.

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
    pub stars: u32,
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
    /// Start a run over the link's raw frames (US2).
    pub fn start(
        self: &Arc<Self>,
        link: Option<Arc<LinkManager>>,
        clock: Arc<dyn Clock>,
    ) -> Result<FocusView, Refusal>;
    /// Stop the run, if one runs, and answer the view (US2).
    pub fn stop(&self) -> Option<FocusView>;
}

pub(crate) fn routes() -> Router<AppState>; // GET, POST and DELETE /api/finder-camera/focus
```

The view's fields after each sample are 033's `Aid` answers: `readings` is
`Aid::readings()`, `last.state` is `Aid::phase()`'s `as_str()`, `median` is
`Aid::median()`, `best`, `trend` (`Trend::as_str()`), `found` and `history` likewise.
The run's task holds a `Weak` of the `FinderFocus`; a stop aborts it. Its loop: take
`link.raw()` (none ends `failed`, `the scope node is not connected`); send
`raw_frame()` under `tokio::time::timeout(RAW_WAIT, ..)`, keeping, while it is on its
way, each new star frame of `link.frames()` (its id and centroids, the last eight); map
the reply as FR-007 says; then wait up to `RAW_WAIT` more for the star frame of the raw
frame's id, skipping the raw frame when it does not come; measure in
`tokio::task::spawn_blocking`; push the `Sample`; repeat. The frames watch ending also
ends the run `failed`, `the scope node is not connected`. The run's mutex is never held
across an `await`.

`api.rs`: `AppState` (`crates/finder/src/api.rs:102`) gains the private field
`finder_focus: Arc<crate::finder_focus::FinderFocus>`, `Arc::default()` in
`AppState::shared` (`crates/finder/src/api.rs:144`), and

```rust
/// The finder camera's focus aid (051).
pub fn finder_focus(&self) -> Arc<crate::finder_focus::FinderFocus>;
```

`router` merges `crate::finder_focus::routes()` after the imaging routes
(`crates/finder/src/api.rs:653`). The `POST` handler takes `SessionWrite` (the
same-origin rule) and `Result<Json<FocusAsk>, JsonRejection>` through `sim_json`
(`crates/finder/src/api.rs:766`), then `start(state.link(), state.clock.clone())`,
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
of the last three readings, the best so far, the trend since the last turn of the
focuser and found once the reading has been through focus and back. null before the
first start." `set_finder_focus`, with the input schema of `FinderFocusArguments` and
the description "Start or stop the finder camera's focus aid, as POST and DELETE
/api/finder-camera/focus do: raw frames of the finder camera measured beside the live
solve, which goes on. It moves nothing and changes no setting: the observer turns the
finder camera's focuser. Read get_finder_focus every few seconds." Its checks in this
order: `on` not a boolean, `on must be true or false`; any other key,
`set_finder_focus takes on and nothing else`; then `start` or `stop` as the route.

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
reads it. It asks the scope node for one raw frame after another (the raw
frames of recordings, one star frame's pixels each, `width × height` bytes on
the scope link) and measures each at the stars the node found in that frame:
the median half-flux diameter, in the finder camera's pixels, of at most 15 of
the brightest stars whose box of 33 pixels lies inside the frame, holds no
other star and no pixel at 250 or above. A frame with fewer than five such
stars says nothing (`too_few_stars`), and one whose median is above 20 pixels
is far out (`far_out`), as for the imaging camera. The readings go through the
imaging camera's own arithmetic: the median of the last three since the last
change, a reading more than 15% from it as a turn of the focuser, the trend,
the best so far and `found` once the reading has been far from the best, near
it, far and near again.

It changes no setting of the finder camera and does not stop the live solve,
which goes on with the same frames; Track may stay on, and keeps the stars
still. Each reading costs one raw frame on the scope link, so the live solve's
frames share the link with it while it runs. It ends `stopped` by a stop, or
`failed` with the words when the scope node is not connected, cannot send raw
frames or sends none for 15 s. Far out of focus the scope node finds no stars
at all, and the aid says so: turn the focuser slowly through its range,
watching the live preview, until stars show, then use the aid. After a change
of optics (a spacer, a reducer) stop the aid and start it again, since the best
so far is the old optic's. Sizes are in the finder camera's pixels, which
compare exactly for one camera behind one optic.
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
texts are US4-S1's, and
`FocusTool({ view, refresh }: { view: ImagingView | null; refresh: () => Promise<void> | void })`:
a `section` labelled by its `h2` `Focus a camera` (class `section-title`), then a
`label` holding the text `Camera` and a `select` with `aria-label="Camera"` and the
options `finder` (`Finder camera`, the default) and `imaging` (`Imaging camera`). With
the finder camera picked, its finder part reads `getFinderFocus` at once and every
1000 ms, shows `Reading the finder camera's focus aid.` until the first read, then the
paragraph "The finder camera's star size, in its pixels: smaller is sharper. It is
read beside the live solve and changes no setting. Turn the focuser one way until the
reading has passed its lowest and risen again, then come back to the lowest.", the
button, and the reading's lines as FR-014 lists them; its bars are 033's drawing (the
last 60 values, 36 units at the largest, `currentColor`, a gap for `null`) with the
finder camera's label. With the imaging camera picked it renders
`<FocusBlock view={view} refresh={refresh} />` when `view?.focus` is set, else the
quiet line of FR-015. `ToolsPage.tsx` renders `<FocusTool view={view} refresh={refresh} />`
after `<AlignmentBlock .../>` (`crates/finder/web/src/components/ToolsPage.tsx:26`).

`playwright.config.ts`: `"**/tools-focus.spec.ts",` on its own line after
`"**/tools.spec.ts",` in the default project's `testIgnore`
(`crates/finder/web/playwright.config.ts:44`) and the simulate project's `testMatch`
(`crates/finder/web/playwright.config.ts:63`).

## What was run and what was read

**Run** (logs under the operator's probe folder, not in the repository), on
`probe/051`, cut from c75a6e76:

- **The commits.** f6b1478b (US1), d8226cad (US2), 7c529cb3 (US3), c5e41445 (US4),
  6274d092 (the probe's own measurements, not a story: a test file of
  `#[test]`s that print), b6a3c5c4 (this draft's checks folded into the stories' tests:
  a hot pixel by each of the six stars, the reducer step of US2-S3, US4-S5's page test
  and US1-S5's flux), 806bf2d4 (the glossary's **Raw frame**) and 9a60d558 (the
  button's and the stop's words, and the run's panic messages, said with "finder
  camera", `dist` rebuilt).
- **The gates.** On c5e41445 and 6274d092: `cargo fmt --check`, clippy with
  `-D warnings`, both `cargo doc` runs, nextest of the whole workspace (1,526 of
  1,526), the doc tests, `npm ci --offline`, the typecheck, Prettier, Vitest, the
  build, the dist check (clean) and `CI=1 npm run e2e` (34 of 34, `tools-focus.spec.ts`
  and 050's `tools.spec.ts` among them). On 806bf2d4: fmt, clippy, both `cargo doc`
  runs, nextest of the whole workspace (1,527 of 1,527: the 1,512 existing, the
  stories' 11 and the probe's own 4) and the doc tests again, and the typecheck,
  Prettier and Vitest (605 of 605: the 600 existing and the stories' 5). On 9a60d558:
  fmt, clippy, the aid's Rust tests, the typecheck, Prettier, Vitest (605 of 605), the
  build, the dist check (clean once committed) and `CI=1 npm run e2e` (34 of 34).
- **Each story's tests fail before it.** US2's four on US1's source: the `GET`
  answers no `{"focus": null}` and the `POST`s' empty answers fail `call`'s JSON read.
  US3's two on US2's source: the tools are unknown, and the README has no line
  `The Tools page's focus aid for the finder camera has two tools:`. US4's Vitest files
  on US3's source: `FocusTool.test.tsx` fails to resolve `./FocusTool`, and
  `FocusToolPage.test.tsx` finds no region `Focus a camera`.
- **Mutations, each run against the scenarios' tests.** The joined-pixels rule off
  (any pixel above the threshold counted) and the isolation rule off each fail US1-S2;
  `found` computed apart from 033's `Aid` fails US2-S2; `SimCamera` ignoring the focus
  error fails US1-S4; the flux not kept (the peak not lowered) fails US1-S5, and no
  scenario failed under it before US1-S5 was added.
- **The measure's candidates, on data.**
  - On the simulator's finder camera at 1000 ms, focus errors 0, 0.5, 1, 2 and 3: the
    reading at 0.94, 0.93, 0.97, 0.97 and 0.96 times H(e), 15 stars each, rising with e;
    at e 4, 5 to 7 stars; from e 6, none detected. At 200 ms, 0.94 to 0.77 times H(e),
    still rising; too few stars from e 4.
  - Star count and the brightest centroid's brightness, from the star frame alone, at
    200 ms: 36 stars at e 0 to 2, and a brightness that rose and fell.
  - The owner's dark frames of the earlier finder camera, its hot clusters detected as
    stars: 8 to 20 px each, with the box's every pixel above background counted, by a
    detection of the console's own or at the node's centroids alike; 1.9 to 2.4 px with
    only the pixels joined to the brightest, a hot cluster's own size.
  - The owner's eight 3 s frames of 2026-11-09 from an earlier finder camera, an OV5647
    behind a 16 mm lens (outside the repository): 2.78 to 3.10 px, 15 stars each. His
    own script's log of that night: half-flux radii 1.21 to 1.56 px, diameters 2.42 to
    3.12. ASTAP's `-analyse` on the same eight: 3 to 7 stars, under five in five frames,
    at 3.4 to 4.5 px.
  - 033's `Aid` replayed over the owner's four focus logs of that camera (doubled to
    diameters): the 3 s logs settled 95 of 104 and 44 of 62 readings; the 1 s logs, of
    8 stars a frame, 21 of 224 and 15 of 214.
  - The measure's cost: under 1 % of the simulator's 500 ms frame interval.
- **The run beside the live solve.** In US2-S1's rig, the session's
  `camera_settings` unchanged and `pointing.seq` rising while the aid ran; the reducer's
  24° field mid-run (US2-S3) and the run went on.
- **The page by night and on a phone.** `tools-focus.spec.ts`'s screenshots at 412 by
  915: more than 10,000 pixels off night red by day and none by night; no horizontal
  scroll.

**Read, not run:**

- a scope node built before 025 answering `UNIMPLEMENTED`, any other error of the raw
  request, and a raw frame of the wrong length (025's words, copied);
- the link dropped between frames, and the frames watch ending;
- a raw frame whose star frame does not come within 15 s, skipped;
- two raw requests at once, the aid's and a recording's, each answered from the same
  pixels (`crates/scope-node/src/service.rs:351`);
- Track on, or a GoTo, during a run;
- the console stopping mid-run;
- a reducer moving the focal plane, so that the frames after it are out of focus;
- every claim of § Outside the factory, and the real finder camera at night.

## Read from the diff, not from a test

- FR-002's rule that a star whose brightest pixel near its centroid is not above the
  threshold is not measured: turning it off failed no scenario, since every star a
  scenario draws has such a pixel. The judge reads it in the diff.
- The README's rows and section and the glossary's entries (FR-011), beyond US3-S2's
  sentence, paragraph and headings.
- FR-007's mappings of `UNIMPLEMENTED`, another error and a wrong length, and the link
  dropped (above).
- FR-008's "changes no setting" beyond US2-S1's and US2-S3's `camera_settings` checks,
  and "writes no file".
- The cadence (`cadence_ms`) and `since`, which no scenario pins.

## Shapes that existing tests pin

- **The simulator's pixels.** `crates/finder/tests/motion/tracking/pi_loop.rs:235`
  calls `render::render` with its six arguments: the signature stays, and its pixels
  are as before (US1-S4 checks the equality; the motion binary passed, run). 033's
  imaging focuser and its tests are untouched (run).
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
  `tools.spec.ts` in the simulate project; it starts no imaging camera focus aid, takes
  no lease and stops its own run before it ends, so the specs after it see the server
  as before (run: 34 of 34).
- **`playwright.config.ts`.** The two entries put `tools-focus.spec.ts` in the simulate
  project only (run).

## Decisions and the alternatives rejected

The nine owner decisions, all open, are in spec.md § Owner decisions, each with its
recommendation. Below them:

| Decision | Chosen | Rejected |
|---|---|---|
| Where the module lives | `crates/finder/src/finder_focus.rs`, beside `recording.rs`, the other reader of raw frames | under `imaging/` (the imaging camera's module; the finder camera is not an imaging camera) |
| The size of a star | the half-flux diameter of the joined pixels above three noise levels | every pixel of the box above background (a hot pixel or a neighbour's wing counts: dark frames read 8 to 20 px so) |
| The stars measured | the node's centroids, isolated, unsaturated, the 15 brightest | a detection of the console's own (a second detector beside the node's, which the live solve uses); every centroid (crowded and saturated ones bias the median) |
| A raw frame's star frame | the frames seen while it was on its way, the last eight, matched by id | the watch's latest frame (it may already be the next frame's) |
| The measure's thread | `spawn_blocking` | the async runtime (a 1.2 MB frame's loops on a runtime thread) |
| The run's lifetime | a task holding a `Weak`, aborted by a stop | a task holding the state (a stopped run's task keeps it alive) |
| The defocused simulator | `render_with_sigma`, called by `render` at 1.2 px | a new argument to `render` (an edit of an existing test's call) |
| The display's history | the page's own bars, 033's drawing with the finder camera's label | importing `FocusBlock`'s `History` (an edit of `FocusBlock.tsx`, FR-017) |
| The story cut | four stories, US2 the largest at 38.6 % (§ Plan traps 2) | three, with MCP in the run's story (about 63 % at the overrun) |

## User Story 1 - The measure and the simulator's focuser

`measure`, the simulator's focus error and `render_with_sigma`, with their test file.
No route, no view.

## User Story 2 - The run

The run, its view, the routes and `AppState::finder_focus`, over US1's `measure`. Its
tests turn US1's focuser.

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
2. **`main` is still as read.** Every citation here was read at c75a6e76. If `main`
   moves in a file § Project Structure names, or in a test § Shapes names, re-read them
   and re-run the validator.
3. **The owner decisions are settled and the owner approvals given** (spec § Owner
   decisions, § Owner approvals). Not met: all nine are open.
4. **Probe builds.** A probe round builds the stories from this text alone, and its
   defects are folded. Not met: the probe of this draft built the stories before the
   text, and the text was written from it.
5. **The page half has run once.** Met on the probe (§ What was run and what was read).
6. **No operator-staged files are needed.** Met: no story reads a file outside the
   repository; the probe's measurements of the owner's frames are not a story.

## Outside the factory

Written, not run. The owner's, at night, with the ASI120MM Mini behind the SV165's 40 mm
f/4 objective (about 1.72° by 1.29°, with the database built for its field, 047), and
again with a focal reducer if one is added (its field needs a database of its own).

1. **By day, a rough focus.** Point the guide scope at the most distant object in
   view, open Setup's live preview, and turn the SV165's helical focuser until its
   edges are sharpest. Note the focuser's position. Neither the aid nor the live solve
   reads a daytime scene.
2. **At night, find stars.** Point at a field rich in stars well above the horizon.
   On Setup, check that the finder camera's star frames hold stars; if they hold none,
   turn the focuser slowly through its range from the day's position, watching the
   preview, until stars show. The live solve need not succeed yet.
3. **Fix the exposure.** On Setup, set the finder camera's exposure to about 1 to 3 s.
   The node's automatic exposure is allowed, but the owner's 1 s logs of an earlier
   camera rarely settled where the 3 s logs settled nearly every reading.
4. **Start the aid.** Open **Tools**, **Focus a camera**, **Finder camera**, **Start
   the finder camera's focus aid**. Wait for `Median of the last 3 frames`; the large number is the
   finder camera's star size in its pixels: smaller is sharper.
5. **Turn, wait, read.** Turn the focuser a small step one way, then take your hand off:
   the next readings say `Settling after a change`, then the median of three again,
   and the trend line says `Sharper` or `Softer than before the last turn`. Keep turning
   the same way while it says `Sharper`. Once it says `Softer` twice, turn back in
   smaller steps. `Passed through focus and came back to the best so far.` says that the
   reading has been far from the best, near it, far again and near again: stop there.
   `Best so far` is the target; a reading within a tenth of it is as good as the night
   allows.
6. **Lock it.** If the focuser has a lock, tighten it with the aid running and watch
   the reading stay near the best; if it rises, loosen, come back and lock again. Note
   the focuser's position, then **Stop the finder camera's focus aid**.
7. **With the reducer.** Fit it, and expect the frames to be out of focus: the focal
   plane moves, and the live solve fails until refocused. The aid says
   `No reading: ... turn the focuser slowly until they show.`: repeat steps 2 to 6,
   with **Stop** and **Start** after fitting it, since the best so far is the old
   optic's. The field doubles and the scale halves, so a focused star is fewer pixels
   across than without it; compare readings only within one optic.
8. **After a spacer.** As for the reducer: stop, start again, refocus.

## Plan traps

They bind every story.

1. **Gate evidence or nothing.** The judge credits only gate results and the story's
   diff. *Check:* every Then names a committed test the `test` gate runs.
2. **One evidence budget.** *Check:* each story's whole diff stays under 60 % of
   65,536 B (39,321 B), measured with
   `git diff <base> -- . ':(exclude)crates/finder/web/dist' | wc -c` from the previous
   story's tip, on code built from the trio. The probe's commits with the folds' checks
   and words: US1 21,931 B (33.5 %), US2 about 25,300 B (38.6 %), US3 about 16,450 B
   (25.1 %) and US4 20,609 B (31.4 %), `dist` about 700 B more. At the 1.35 times a
   factory build has run over its probe on earlier specs, the largest is about
   34,200 B (52.2 %, US2), under the 60 % line and the 128 KiB refusal.
3. **A spec fixed mid-epic needs a relaunch.** *Check:* no builder edits
   `specs/051-tools-page-focuses-either-camera/` or `.specify/`.
4. **An earlier story's test file is an existing test.** *Check:* US2 to US4 write no
   file of an earlier story's tests; US3's MCP rig is its own.
5. **No new support file.** *Check:* nothing new under `crates/finder/tests/support/`;
   the tests use `crate::support::live`'s `DB` and `START` as they are.
6. **An oracle copied from the code proves nothing.** *Check:* the tests write the
   Gaussian's and the disc's half-flux diameters, the places and the words as literals;
   none calls `measure` for an expected value or reads the module's constants.
7. **A negative assertion passes when nothing happens.** *Check:* US1-S2's equal
   readings stand beside `stars` 6; US2-S3's "no `last.value`, no `median`" beside
   `last.state` `too_few_stars` and `last.stars` under 5; US2-S2's `found` false beside
   the last reading's `found` true; US4-S4's "no button `Start the finder camera's focus aid`" beside
   033's heading and button; the night screenshot's 0 beside the day one's count.
8. **The frame in flight at a turn was taken before it.** *Check:* US2-S2 and US2-S3
   read until the state they wait for holds, never the next frame alone.
9. **Paused time and the simulator's clock.** *Check:* every Rust test of this spec
   that awaits runs under `start_paused = true`, and every wait is bounded (120 s of
   paused time).
10. **The frames watch keeps only the latest.** *Check:* the run keeps the star frames
    it sees while a raw frame is on its way, from before it sends the request.
11. **The lock is never held across an await.** *Check:* `FinderFocus`'s mutex is a
    `std::sync::Mutex` taken only in `view`, `start`, `stop` and the sample's push.
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
    switch; it starts no imaging camera focus aid and stops its own run in a `finally`;
    its waits stay well under the 60 s test timeout
    (`crates/finder/web/playwright.config.ts:29`).
18. **Playwright's project flag takes `=`.** *Check:* the development command is
    `npx playwright test --project=simulate e2e/tools-focus.spec.ts`.
19. **"Finder" alone is the glossary's _Avoid_ for the camera.** *Check:* every message,
    label, button and route says "finder camera" (`/api/finder-camera/focus`,
    `Start the finder camera's focus aid`, `The finder camera's focus aid stopped.`).

## Risks

| Risk | Mitigation |
|---|---|
| The real finder camera's readings rarely settle at a short exposure (the owner's 1 s logs of an earlier camera settled about one in ten). | The README and § Outside the factory step 3 say to fix 1 to 3 s on Setup; 033's rules are kept, not loosened. |
| With the reducer, a focused star may be under 2 px across, where a half-flux diameter flattens near focus. | The minimum still marks the best; the probe read drawn σ 1.2 stars within 8 % (read, not run, below that). |
| Raw frames load the scope link beside the star frames. | One raw frame per reading, the node's own pixels; the link's other frames go on (run on the simulator; the real link's share, read). |
| A story grows past 60 % in the factory. | The largest measures 38.6 % on the probe's code, about 52 % at the usual overrun (§ Plan traps 2). |
| Hot pixels the node takes for stars read about 2 px, as sharp as a star in focus (the owner's dark frames of an earlier camera, 1.9 to 2.4 px). | The median of 15 is held by the stars of a field rich in them; the node's detection (048) and its hot-pixel setting are the live solve's own; the real ASI120MM's frames are read, not run. |
| The finder camera's raw frames are 8-bit (045), so bright stars saturate early. | Saturated stars are left out (FR-002); a fixed exposure lower on Setup gives more unsaturated ones. |

## Refinement Record

Drafted on 2026-11-26 by the plate-solver operator seat's spec session against `main` at
c75a6e76, from a probe of that day on `probe/051` (tip 9a60d558: a trial build of the
four stories with every scenario's test and both gates passing, each story's tests
failing before it, the measure's candidates measured on the simulator and on the
owner's frames, ASTAP on the same frames, 033's arithmetic replayed on the owner's
logs, and the page measured in a browser by day and by night), and a reading of 033's
reading, arithmetic and block, 025's raw frames, the node's detection and star frames,
050's Tools page, the simulator's finder camera, MCP's tools, the constitution, the
glossary and the ADRs. Writes come from the task text. Shared writes name the spec
story that last wrote the same file on `main`; within the spec, each story's files
written by an earlier one are ordered by `depends_on_merged`.

The nine owner decisions of spec.md are open; their questions, options and evidence
are spec.md § Owner decisions, and this record is written to their recommendations.
Its `open_questions` are empty so that the validator checks the rest of the record, as
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
        assertion: "drawn 640 x 480 frames, background 10, six stars at the six places, peak 200, sigma 1.2, 2.0 and 3.0, measured with their six centroids: each Sample::Reading with stars 6 and px within 8 % of 2 sqrt(2 ln 2) sigma"
        decision: "spec.md:US1-S1"
      - path: crates/finder/tests/console/finder_focus_measure.rs
        status: new
        disposition: add
        assertion: "a drawn sigma 2.0 peak 200 frame of the six places, a star at (10.5, 240.5) over the edge, two at (400.5, 400.5) and (410.5, 405.5), and one at (300.5, 420.5) of peak 245 reaching 255, each with its centroid: Sample::Reading stars 6; the same frame with a pixel of 200 ten pixels right of each of the six stars and no centroid: Reading stars 6, px equal to the first within 1e-9"
        decision: "spec.md:US1-S2"
      - path: crates/finder/tests/console/finder_focus_measure.rs
        status: new
        disposition: add
        assertion: "four drawn stars at the first four places: Sample::TooFewStars stars 4; a frame of background 10 alone and no centroid: TooFewStars stars 0; five uniform discs of level 60 and radius 15 px at the first five places: Sample::FarOut stars 5, px within 5 % of 15 sqrt 2"
        decision: "spec.md:US1-S3"
      - path: crates/finder/tests/console/finder_focus_measure.rs
        status: new
        disposition: add
        assertion: "the simulator on DB, Site::default, VirtualClock at START, pushed to alt 50 az 120: finder_focus_error 0, then each e set reads back; render_with_sigma at PSF_SIGMA_PX equals render pixel for pixel for its stars at 1000 ms gain 1 seed 13 frame 0; SimCamera frames at 1000 ms gain 1 measured at the node's detect with DetectSettings::cedar: e 0, 1, 2, 3 each Reading stars 15 px within 10 % of H(e), each larger than the one before; e -3 px within 5 % of e 3's; e 8 TooFewStars under 5 stars"
        decision: "spec.md:US1-S4"
      - path: crates/finder/tests/console/finder_focus_measure.rs
        status: new
        disposition: add
        assertion: "Optics 64 x 64 otherwise default, one RenderStar at (32.5, 32.5) magnitude 8, frames 0 to 63 at 200 ms gain 1 seed 13: with the star no pixel at 250 or above; the mean sum of the star's pixels less the starless frame's above 50 at PSF_SIGMA_PX, and at sigma 3.0 within 5 % of it"
        decision: "spec.md:US1-S5"
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
      - "render::render draws exactly the pixels it drew before (its signature unchanged)"
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
        assertion: "the focus rig, live: GET /api/finder-camera/focus 200 focus null; POST {} 200 with focus.state running, readings 0, found false, history []; the settled reading after 0 has median.value within 10 % of H(0), last.stars at least 5, history as long as readings; GET /api/session then reads the camera_settings read before the start and a pointing.seq above the one read before"
        decision: "spec.md:US2-S1"
      - path: crates/finder/tests/console/finder_focus_aid.rs
        status: new
        disposition: add
        assertion: "the focus rig, live, focuser at 3, started and its settled reading read: turned to 0 until a settled reading has trend better, to -3 until one has trend worse, to 0 until found true: the better and worse readings each found false; the last found true, state running, best within 10 % of H(0), median.value at most 1.10 times best"
        decision: "spec.md:US2-S2"
      - path: crates/finder/tests/console/finder_focus_aid.rs
        status: new
        disposition: add
        assertion: "the focus rig, live, started and settled: focuser at 10 until last.state too_few_stars: last.stars under 5, no last.value, no median, history ending in null, state running; set_optics fov 24 with the focuser at 0, the settled reading after that count: state running; DELETE 200 focus.state ended, ended stopped; 5 s later DELETE again 200 with the same readings and ended stopped; GET /api/session camera_settings as read before the start"
        decision: "spec.md:US2-S3"
      - path: crates/finder/tests/console/finder_focus_aid.rs
        status: new
        disposition: add
        assertion: "POST /api/finder-camera/focus {} to the bare console 409 the scope node is not connected; to the live rig x 1 400, {} as text/plain 415, {} 200, {} again 409 the finder camera's focus aid is already running; the node's frames stopped: the run ends with focus.ended failed and focus.error exactly no raw frame arrived within 15 s"
        decision: "spec.md:US2-S4"
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
        assertion: "the bare console: set_finder_focus on true errs exactly the scope node is not connected; the MCP rig: get_finder_focus {} structuredContent focus null; set_finder_focus {} errs on must be true or false, on true with x 1 errs set_finder_focus takes on and nothing else, get_finder_focus x 1 errs get_finder_focus takes no arguments; once live on true focus.state running, again errs the finder camera's focus aid is already running; get_finder_focus until focus.median.readings 3; on false focus.ended stopped; tools/list holds both, get_finder_focus readOnlyHint true"
        decision: "spec.md:US3-S1"
      - path: crates/finder/tests/mcp/finder_focus.rs
        status: new
        disposition: add
        assertion: "crates/finder/README.md read as a file: it still holds The imaging camera has sixteen tools of its own; The Tools page's focus aid for the finder camera has two tools: comes after The Tools page's finder alignment has two tools: and before ### MCP under WSL2, with a list entry each for get_finder_focus and set_finder_focus between them; ## Focusing the finder camera comes after ## Finder alignment and before ## Calibration frames"
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
        assertion: "finderStatus gives exactly: no last, Reading the first raw frame of the finder camera.; a reading of 15 stars with 3 held, Median of the last 3 frames · 15 stars; 9 stars with 2 held, 2 of 3 frames since the last change · 9 stars; settling, Settling after a change; too_few_stars with 2, No reading: 2 stars measured, and 5 are needed. Far out of focus the stars vanish: turn the focuser slowly until they show.; far_out of 21.2, Far out of focus: stars about 21.20 px across"
        decision: "spec.md:US4-S1"
      - path: crates/finder/web/src/components/FocusTool.test.tsx
        status: new
        disposition: add
        assertion: "FocusTool with no imaging view and GET focus null: before the first read Reading the finder camera's focus aid.; Start the finder camera's focus aid sends POST /api/finder-camera/focus with Content-Type application/json and body {}; a running aid of median 2.83 of 3, last 15 stars, best 2.7, trend better, found true, history [5.9, 6.1, null, 2.8, 2.83] shows 2.83 px, Median of the last 3 frames · 15 stars, Best so far 2.70 px, Sharper than before the last turn, Passed through focus and came back to the best so far. and the image Focus readings of the last 5 frames of the finder camera with 4 bars; after Stop the finder camera's focus aid, The finder camera's focus aid stopped."
        decision: "spec.md:US4-S2"
      - path: crates/finder/web/src/components/FocusTool.test.tsx
        status: new
        disposition: add
        assertion: "an aid ended failed with the error no raw frame arrived within 15 s: the alert reads exactly The finder camera's focus aid stopped: no raw frame arrived within 15 s. and the button Start the finder camera's focus aid"
        decision: "spec.md:US4-S3"
      - path: crates/finder/web/src/components/FocusTool.test.tsx
        status: new
        disposition: add
        assertion: "Imaging camera picked in the select Camera: with no view, The imaging camera's focus aid shows here once the imaging camera is ready.; with a view whose focus has unit px, 033's level-two heading Focus and its button Start focus aid, and no button Start the finder camera's focus aid"
        decision: "spec.md:US4-S4"
      - path: crates/finder/web/src/components/FocusToolPage.test.tsx
        status: new
        disposition: add
        assertion: "ToolsPage with no imaging view: the region Align the finder camera comes before the region Focus a camera in the document, and the select Camera reads Finder camera"
        decision: "spec.md:US4-S5"
      - path: crates/finder/web/e2e/tools-focus.spec.ts
        status: new
        disposition: add
        assertion: "the simulated console at 412 by 915, day: /tools, Start the finder camera's focus aid in the region Focus a camera; within 20 s its status contains Median of the last 3 frames, a two-decimal px number and its readings' image show; scrollWidth at most 412 by day and by night; the day screenshot more than 10000 pixels off night red, the night one 0; Imaging camera picked shows Start focus aid within 15 s; Finder camera again and Stop the finder camera's focus aid, Start the finder camera's focus aid within 15 s"
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
