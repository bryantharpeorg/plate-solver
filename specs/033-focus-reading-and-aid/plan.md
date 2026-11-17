# Implementation Plan: Focus Reading and Focus Aid

**Branch**: `033-focus-reading-and-aid` | **Date**: 2026-11-15 | **Spec**: [spec.md](spec.md)

This plan records every design decision for the feature. A story that would change one
escalates rather than re-deciding it.

## Summary

Add focus to the console's imaging module. `focus::read` asks the solver port for the
star measure of a saved imaging frame, or of a region of it, and turns it into a focus
reading in sensor pixels and, with a scale, arcseconds; under five stars, or above 20
frame pixels, a frame is unmeasurable. `focus::find_region` measures the nine tiles of a
full imaging frame and names the one with the smallest reading, or, when none can be
measured, the far-out one with the smallest size. `focus::Kept` keeps the focus region
and the focus record for one camera in `focus.json`: the record has tonight's part and
the earlier nights', is set only by a settled reading of the focus aid, and
`focus::judge` compares a reading with the smaller of the two, or with tonight's alone
once the aid has passed through focus tonight. `focus::aid::Aid` is arithmetic and
nothing else: from the readings alone it works out the median of the last three, a
change of more than 15% and the two frames set aside after it, the trend, the best of
the last 600 settled readings, and whether the reading has been far from the best, near
it, far and near again. `focus/run.rs` runs it on the console (US5): `POST
/api/imaging/focus` takes the imaging camera's lease from 031's imaging state under
the name `the focus aid`, remembers the camera's settings, finds or takes the focus
region, exposes that region alone at the smallest binning one imaging frame after
another, removes each frame's file once the next is saved, judges each settled reading
and then notes it in the record. Far out of focus, where no region can be found, it
runs on a tile for the time being and searches again at the first settled reading.
`DELETE` stops it. US8 adds `check`, with which it stops by itself at the first settled
reading and notes nothing. However it ends, the settings are put back and the lease is
released; while it is held, nothing else exposes or changes the camera. The simulator
gains a focuser to turn and the simulated solver adapter seeded noise and a sharpest
point. The Imaging page gets a Focus block and MCP clients two tools.

The lease is 031's User Story 6. This trio takes it and releases it, and creates no
part of it (§ The camera lease, § Prerequisites 11).

## Technical Context

**Language/Version**: Rust (MSRV 1.96) for US1 to US5, US7 and US8; TypeScript with
React 19 for US6.

**Primary Dependencies**: nothing new, and no `Cargo.toml` changes. `finder` already has
`simulator`, `serde`, `serde_json` and `tokio` with `fs`, `sync` and `time`
(`crates/finder/Cargo.toml:33`). The page adds no npm package.

**Storage**: in the imaging folder, `focus.json`. The focus aid's imaging frames are
031's own files under `frames/<UTC date>/`, and all but the last are removed. Nothing is
kept in `session.json`.

**Testing**:
- US1: `crates/finder/tests/console/imaging_focus_reading.rs` (new).
- US2: `crates/finder/tests/console/imaging_focus_region.rs` (new).
- US3: `crates/finder/tests/console/imaging_focus_record.rs` (new).
- US4: `crates/finder/tests/console/imaging_focus_arithmetic.rs` (new).
- US5: `crates/finder/tests/console/imaging_focus_aid.rs` (new).
- US6: `crates/finder/web/src/components/FocusBlock.test.tsx` (new), Vitest.
- US7: `crates/finder/tests/mcp/imaging_focus.rs` (new).
- US8: `crates/finder/tests/console/imaging_focus_check.rs` (new) and, for US8-S3,
  `crates/finder/tests/cli/imaging_focus.rs` (new).
- Where a finder test goes is 027's rule: `console` calls the router in process and
  moves no mount (`crates/finder/tests/console/main.rs:2`), `cli` starts the built
  `finder` (`crates/finder/tests/cli/main.rs:1`), `mcp` drives the MCP door in process
  (`crates/finder/tests/mcp/main.rs:1`). No test of this spec moves a mount. Each module
  is declared by one `mod` line in its binary's `main.rs`, which may hold nothing but
  module declarations after its support include
  (`crates/finder/tests/layout/main.rs:156`). The line goes in alphabetical order.
  The layout test does not check the order; `cargo fmt --all --check`, the first
  command of the `lint` gate, does, because rustfmt sorts a run of `mod` lines by
  default (`reorder_modules`, which `rustfmt.toml` leaves on). A module binds the live
  helpers it uses with `use crate::support::live as support;`
  (`crates/finder/tests/console/live/loop_.rs:3`), and writes the rest of its setup
  itself: `crates/finder/tests/support` takes no new file
  (`crates/finder/tests/layout/main.rs:126`). So the scripted solver is written out in
  each of `imaging_focus_reading`, `imaging_focus_region`, `imaging_focus_aid`,
  `imaging_focus_check` and the `mcp` module, and the focus rig in the last three:
  about 2 KiB and 4 KiB of the same setup, counted in each story's budget (§ Risks). No
  module imports another test module's helpers.
- No test runs ASTAP or the ASTAP adapter: every `measure` is the simulated solver
  adapter's or the scripted solver's.
- No Playwright test (spec owner decision 17). 031's `e2e/imaging.spec.ts` opens the
  simulated console's Imaging page, on which the Focus block now renders idle, and its
  full-page night screenshot must still count no pixel that is not red (031 US4-S8).

**Target Platform**: the `finder` console on the console machine. Nothing changes on the
scope node.

**Constraints**:
- No change under `crates/scope-node`, `crates/scope-link` or `proto/`; under
  `crates/simulator` only `crates/simulator/src/optics.rs` (US1); no `unsafe`; no
  dependency.
- No file of this spec reads the mount, an axis position, an altitude or an azimuth
  (the plan document, § Mount-neutral by design).
- No lock on the imaging camera of this spec's own: the aid takes and releases 031's
  lease (the plan document's amendment 1; 031 US6; § The camera lease).
- No constant is sized to one telescope or camera. The tiles are thirds, the thresholds
  fractions, the counts imaging frames; `FAR_OUT_PX` is 20 frame pixels, ASTAP's limit
  on a mono frame and half of it on a colour mosaic that the adapter averages 2 × 2
  (spec owner decision 5).
- No story's diff passes 64 KiB. US5 and US8 are two stories for that reason (§ Risks).
- `web/dist` is committed and must be the build of its source (US6).

**Scale/Scope**: three new module files, two routes, one simulate route, one file on
disk, two MCP tools, one page block.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Nothing touches the finder camera's solver, detection or `crates/math-core`. Console work is grounded in the cited sources: `docs/research/2026-11-15-imaging-camera.md` § 6. Focus for what a reading can and cannot say and for how tools guide a hand on the focuser, § 3. ASTAP for the star measure's limits, and the simulator's truth for the rest. No star detection or size measure is written here. |
| II. Never weaken a gate | No existing test, golden, fixture, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS`, `ergane.yaml` or `playwright.config.ts` is edited. New tests are new module files; each adds one `mod` line to its binary's `main.rs`, which holds no test (spec owner approval 12). 032's scenario of the simulated adapter's star measure passes unchanged, because the two new builders are off unless called. |
| III. Tests first, independent expectations | Oracles: the simulator's star size, worked from 030's point-spread constants in spec.md; worked examples whose arithmetic is shown; the published SplitMix64 stream; a `focus.json` written by hand in a test; and, for scenarios that check only words, statuses and keys, the scenario's own stated text. Every scenario names its oracle and tolerance. Time comes from the console's clock (`crates/finder/src/session.rs:24`) and tokio's paused time; nothing here reads a system clock. |
| IV. Critical paths | No `unsafe`, no `Cargo.toml` or `Cargo.lock` change, nothing under `proto/` or `crates/math-core`. |
| V. Proven open source first, honest licensing | The focus reading is measured by the imaging solver, ASTAP, through 032's solver port, as principle V of constitution 1.3.0 allows (§ Prerequisites 4). Nothing of ASTAP or of any focusing tool is copied; PHD2, SharpCap, N.I.N.A. and Ekos are cited in the research note as prior art for behaviour, not as sources of code. |

## Project Structure

```text
crates/simulator/
└── src/optics.rs                       # US1: focus_error, set_focus_error
crates/finder/
├── src/imaging/simulated.rs            # US1: with_measure_noise, with_sharpest_at
├── src/imaging/focus.rs                # US1: new; the reading. US2: tiles, find_region, Kept, focus.json. US3: the record, judge. US4: pub mod aid. US5: pub mod run
├── src/imaging/focus/aid.rs            # US4: new; the arithmetic
├── src/imaging/focus/run.rs            # US5: new; the aid on the console, the focus view. US8: check, focus_ended
├── src/imaging/mod.rs                  # US1: pub mod focus. US5: the focus state, the view's focus, describe
├── src/imaging/routes.rs               # US5: POST and DELETE /api/imaging/focus
├── src/api.rs                          # US1: GET and POST /api/sim/focus
├── src/main.rs                         # US8: the simulated solver's noise and sharpest point
├── src/lib.rs                          # US5: the two routes' doc lines. US8: check in the first
├── src/mcp/tools.rs                    # US7: get_focus_reading, set_focus_aid
├── README.md                           # US1: the simulate row. US5: two API rows. US8: ## Focus. US7: two tool lines
├── tests/console/main.rs               # US1 to US5 and US8: one mod line each
├── tests/console/imaging_focus_reading.rs     # US1: new
├── tests/console/imaging_focus_region.rs      # US2: new
├── tests/console/imaging_focus_record.rs      # US3: new
├── tests/console/imaging_focus_arithmetic.rs  # US4: new
├── tests/console/imaging_focus_aid.rs         # US5: new, US5-S1 to US5-S7
├── tests/console/imaging_focus_check.rs       # US8: new, US8-S1 and US8-S2
├── tests/cli/main.rs                   # US8: mod imaging_focus
├── tests/cli/imaging_focus.rs          # US8: new, US8-S3
├── tests/mcp/main.rs                   # US7: mod imaging_focus
├── tests/mcp/imaging_focus.rs          # US7: new
└── web/
    ├── src/api.ts                      # US6: the focus types and three calls
    ├── src/components/FocusBlock.tsx        # US6: new
    ├── src/components/FocusBlock.test.tsx   # US6: new
    ├── src/components/ImagingPage.tsx  # US6: places the block
    └── dist/                           # US6: rebuilt
DESIGN.md                               # US6: one entry, one clause, one sentence
.impeccable/design.json                 # US6: the Focus block component
```

## Interfaces

Sensor pixels, frame pixels, the simulator's star size H(e) and the rig's scale are as
spec.md § User Scenarios defines them. Everything 030, 031 and 032 create is named as their
reviewed plan.md gives it and is re-read on `main` before the flip (§ Prerequisites 2).
The two names of 031's that this trio leans on hardest, the imaging camera's lease (031
US6) and `RoiEdit`, which 031's `SettingsEdit` holds, are quoted in § The camera lease.

### The simulator's focuser (`crates/simulator/src/optics.rs`, US1)

On `optics::Simulator`, beside 030's `imaging_optics` and `set_imaging_optics`:

```rust
/// The focus error of the imaging optics: 0 is best focus, and its sign does not
/// change a star's size.
pub fn focus_error(&self) -> f64;
/// Turn the simulated focuser. Panics on a value that is not finite.
pub fn set_focus_error(&self, error: f64);
```

Each takes the state's lock once (`crates/simulator/src/optics.rs:65`):
`set_focus_error` writes `imaging.focus_error` and leaves every other field of the
imaging optics as it is. `ImagingOptics` gains no field, because 030's and 032's tests
build it by literal. They are reached through `controls::Simulator`'s `Deref`, as the
rest of the optics is.

### The simulate route (`crates/finder/src/api.rs`, US1)

Where the simulate routes are registered (`crates/finder/src/api.rs:470`), one more:
`.route("/api/sim/focus", get(sim_focus).post(sim_set_focus))`.

```rust
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FocusControl { focus_error: f64 }
```

Both handlers take `SessionWrite` (`crates/finder/src/api.rs:349`), as `sim_frames`
does (`crates/finder/src/api.rs:556`). `sim_focus` answers
`Json(json!({"focus_error": sim.focus_error()}))`. `sim_set_focus` reads the body
through `sim_json` (`crates/finder/src/api.rs:570`), which answers 400 for a missing or
unknown key; a value outside −100 to 100 answers `ApiError::bad_request` with exactly
`focus_error must be a number from -100 to 100`; otherwise it calls `set_focus_error`
and answers the same body as the `GET`. The key is `focus_error`, never `error`, which
every refusal of this API already uses for its words. Without a simulation the paths
fall to the 404 that every `/api/sim/*` path has (`crates/finder/src/api.rs:464`).

### The simulated solver adapter (`crates/finder/src/imaging/simulated.rs`, US1)

Two builders on 032's `SimulatedSolver`, both off unless called:

```rust
impl SimulatedSolver {
    /// Multiply each star measure by a seeded factor from `1 − fraction` to
    /// `1 + fraction`. The n-th star measure this solver answers takes the n-th number
    /// of the stream, so two solvers built alike answer alike.
    pub fn with_measure_noise(self, seed: u64, fraction: f64) -> Self;
    /// Stars are smallest at (x, y), given as fractions of the sensor's width and
    /// height, and grow away from it: `growth` sensor pixels of width, added in
    /// quadrature, at half the sensor's diagonal from that point.
    pub fn with_sharpest_at(self, x: f64, y: f64, growth: f64) -> Self;
}
```

`measure` keeps 032's steps (the delay, the count of seeded stars inside the region,
`TooFewStars` with none) and changes its last line. With W and H the optics' width and
height, b the frame's binning, `roi` the frame's region of interest and r the region
given, or (0, 0, `frame.width`, `frame.height`) without one:

- the measured region's centre, in sensor pixels: `cx = roi.x + (r.x + r.width/2)·b`,
  `cy = roi.y + (r.y + r.height/2)·b`, in `f64`;
- `d = hypot(cx − x·W, cy − y·H) / (hypot(W, H)/2)`, and 0 without `with_sharpest_at`;
- `sigma = √(PSF_SIGMA_PX² + (FOCUS_SIGMA_PX·focus_error)² + (growth·d)²)`;
- `half_flux_px = 2·√(2·ln 2)·sigma/b·factor`.

`factor` is 1 without `with_measure_noise`. With it, `factor = 1 + fraction·(2·u − 1)`
and u is the next number of this stream, n counting from 0 the star measures this
solver has answered (an `AtomicU64`; a call that answers `TooFewStars` takes none):

```rust
fn unit(seed: u64, n: u64) -> f64 {
    let mut z = seed.wrapping_add(n.wrapping_add(1).wrapping_mul(0x9e37_79b9_7f4a_7c15));
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^= z >> 31;
    ((z >> 11) as f64 + 0.5) / 9_007_199_254_740_992.0
}
```

That is SplitMix64: for the seed 0 the first three values of z are
`0xe220a8397b1dcdaf`, `0x6e789e6aa1b965f4` and `0x06c45d188009454f`, the published
stream the simulator's own test asserts, giving u = 0.8833108, 0.4315280 and 0.0264338
and, at `fraction` 0.02, factors of 1.0153324, 0.9972611 and 0.9810574 (spec US1-S4).
The simulator's generator is private to its crate (`crates/simulator/src/noise.rs:4`),
which is why these six lines are written here. The noise is uniform and bounded on
purpose: a test can state the bound.

### The focus reading (`crates/finder/src/imaging/focus.rs`, new, US1)

```rust
//! Focus (033): the focus reading, the focus region, the focus record and the verdict.
use super::solver::{ImagingSolver, Region, SolveError, SolveFrame};

/// Fewer stars than this in a star measure are no reading.
pub const MIN_STARS: u32 = 5;
/// A star measure above this many frame pixels is "far out". 20 frame pixels is
/// ASTAP's limit on a mono frame, and half of it on a colour mosaic that the adapter
/// averages 2 × 2 (`docs/research/2026-11-15-imaging-camera.md` § 3. ASTAP).
pub const FAR_OUT_PX: f64 = 20.0;

/// What one imaging frame, or a region of it, says about focus.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FocusReading {
    pub frame_id: u64,
    /// The median half-flux diameter in sensor pixels.
    pub px: f64,
    /// The same in arcseconds, when a scale was given.
    pub arcsec: Option<f64>,
    pub stars: u32,
    /// The part of the sensor that was measured, in sensor pixels.
    pub region: Region,
    /// The middle of the exposure, console Unix seconds.
    pub at: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Unmeasurable {
    TooFewStars { stars: u32 },
    /// The size in sensor pixels, for showing only.
    FarOut { px: f64, stars: u32 },
}

pub type Read = Result<FocusReading, Unmeasurable>;

/// `region` is in the frame's pixels; `None` is the whole frame. `scale` is arcseconds
/// a sensor pixel.
pub async fn read(solver: &dyn ImagingSolver, frame: SolveFrame, frame_id: u64,
    region: Option<Region>, scale: Option<f64>) -> Result<Read, SolveError>;
```

`read` awaits `solver.measure(frame.clone(), region)` and then, in this order:

1. the error `SolveError::TooFewStars`, whatever its `detail`, answers
   `Ok(Err(Unmeasurable::TooFewStars { stars: 0 }))`; any other error is `read`'s own,
   unchanged;
2. a star count under `MIN_STARS` answers `Ok(Err(TooFewStars { stars }))`;
3. a `half_flux_px` above `FAR_OUT_PX` answers `Ok(Err(FarOut { px, stars }))`, `px`
   being `half_flux_px·binning`;
4. otherwise the reading: `px = half_flux_px·binning`; `arcsec = px·scale`; `at =
   frame.start + frame.exposure_s/2`; and, with r the region given or (0, 0,
   `frame.width`, `frame.height`), `region = (roi.x + r.x·b, roi.y + r.y·b, r.width·b,
   r.height·b)`.

It keeps no state and writes nothing. 035 calls it for each sub.
`crates/finder/src/imaging/mod.rs` gains `pub mod focus;` in alphabetical order (US1).

`focus::MIN_STARS` is not 032's `simulated::MIN_STARS`, the 30 stars its simulated
adapter wants for a solve: the two live in different modules and neither is imported
into the other. `FAR_OUT_PX` is compared with the star measure as the solver adapter
reports it, in the frame's pixels. The ASTAP adapter measures a copy averaged 2 × 2
for a colour mosaic and doubles the answer (032, § The ASTAP adapter), so on such a
frame 20 here is 10 pixels of the picture ASTAP was given, half of ASTAP's own limit
(spec owner decision 5).

### The focus region (`crates/finder/src/imaging/focus.rs`, US2)

```rust
/// Tiles across and down.
pub const GRID: u32 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tile { pub col: u32, pub row: u32 }
impl Tile {
    /// `None` unless both are below `GRID`.
    pub fn new(col: u32, row: u32) -> Option<Self>;
}

/// The sensor pixels of a tile of `sensor`, for a camera at `binning`.
pub fn tile_region(sensor: Region, binning: u32, tile: Tile) -> Region;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TileRead { pub tile: Tile, pub read: Read }

#[derive(Debug, Clone, PartialEq)]
pub struct Search {
    /// All nine, in row-major order.
    pub tiles: Vec<TileRead>,
    /// The tile with the smallest reading; `None` when no tile could be measured.
    pub best: Option<Tile>,
    /// Only while `best` is `None`: the far-out tile with the smallest size. `None`
    /// when there is a `best` or no tile is far out.
    pub far_out: Option<Tile>,
}

/// Measure the nine tiles of an imaging frame and name the sharpest.
pub async fn find_region(solver: &dyn ImagingSolver, frame: SolveFrame, frame_id: u64,
    scale: Option<f64>) -> Result<Search, SolveError>;
```

`tile_region`: with `unit = 2·binning`, `w = (sensor.width/GRID)/unit·unit` and `h =
(sensor.height/GRID)/unit·unit` in whole numbers, the tile is `(sensor.x + col·w,
sensor.y + row·h, w, h)`. The multiple of twice the binning is 030's rule for a region
of interest, so every tile can be asked of the camera as it is. A few pixels at the
right and bottom edges belong to no tile.

`find_region` takes the tiles of `frame.roi` at `frame.binning`, row by row, and for
each calls `read` with the tile in frame pixels, `((t.x − roi.x)/b, (t.y − roi.y)/b,
t.width/b, t.height/b)`, one after another, never two at once. An error of `read` ends
it with that error. `best` is the tile whose reading has the smallest `px`, compared
with `<` so that the first of equal ones stays; an unmeasurable tile is never the best.
`far_out` is worked out only when `best` is `None`: among the tiles whose `read` is
`Err(Unmeasurable::FarOut { px, .. })`, the one with the smallest `px`, again compared
with `<`; `None` when no tile is far out. It is what the aid reads for the time being
when the stars are too large to choose a region by (§ The aid on the console, step 2;
spec owner decision 24). ASTAP's sizes above its limit are rough, so the choice among
far-out tiles is rough too; it only has to be a tile with stars in it.

### What is kept (`crates/finder/src/imaging/focus.rs`, US2; US3 adds the record)

```rust
pub const FORMAT: &str = "imaging-focus/1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source { Found, Fixed }

/// The focus region as it is kept. `x` to `height` are sensor pixels.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct KeptRegion {
    pub col: u32, pub row: u32,
    pub x: u32, pub y: u32, pub width: u32, pub height: u32,
    pub source: Source,
    /// For a found one: the reading it was found from. Skipped while `None`.
    pub px: Option<f64>, pub stars: Option<u32>, pub frame_id: Option<u64>,
    /// Console Unix seconds.
    pub at: f64,
}
impl KeptRegion {
    pub fn found(sensor: Region, binning: u32, tile: Tile, reading: &FocusReading) -> Self;
    pub fn fixed(sensor: Region, binning: u32, tile: Tile, at: f64) -> Self;
    pub fn tile(&self) -> Tile;
}

/// What the console keeps about focus for one camera. Its fields are private; US3 adds
/// the record to it and to the file.
#[derive(Debug)]
pub struct Kept { /* the file's path, the camera, the region; US3: the record */ }
impl Kept {
    /// Nothing known yet. `path` is `<imaging folder>/focus.json`; nothing is read.
    pub fn new(path: PathBuf) -> Self;
    /// The camera was described. For the camera already held this does nothing.
    /// Otherwise it reads the file and takes what it holds for this camera, or nothing.
    pub fn describe(&mut self, camera: CameraId);
    pub fn region(&self) -> Option<KeptRegion>;
    /// Keep a region in place of the last, and write the file.
    pub fn keep_region(&mut self, region: KeptRegion);
}
```

`CameraId` is 032's (`crates/finder/src/imaging/relation.rs`, 032 US3). `found` takes
its rectangle from `tile_region`, its `px`, `stars`, `frame_id` and `at` from the
reading; `fixed` has none of the three.

`focus.json`, on one line in the file:

```json
{"format":"imaging-focus/1",
 "camera":{"model":"simulated imaging camera","serial":"SIM-0001","width":1236,"height":822,"pixel_um":19.12},
 "region":{"col":2,"row":0,"x":824,"y":0,"width":412,"height":274,"source":"found","px":2.6,"stars":12,"frame_id":3,"at":1768528803.5},
 "record":{"arcsec_per_px":2.19,"tonight":{"px":3.1,"at":1768528860.0,"since":1768528800.0,"passed":true},"earlier":{"px":2.9,"at":1768400000.0}}}
```

`region` and `record` are left out while there is none, and so is every `None` inside
them; `passed` (US3) is left out while false and read as false when absent, so a file
written by hand without it is a record that has not been passed through. The file is
written with `serde_json` as `focus.json.part` and renamed, at every
change, by one private `save` that writes everything `Kept` holds, so that a change of
the region never loses the record and the reverse. A failed write is printed to
standard error and the value stays in memory. Reading answers nothing, without a panic,
when the file is missing, is not JSON, has another `format`, or has a `camera` that is
not equal to the one described. Before `describe` a `Kept` holds nothing, and
`keep_region` and `note` do nothing.

### The record and the verdict (`crates/finder/src/imaging/focus.rs`, US3)

```rust
/// A night for the focus record.
pub const NIGHT_S: f64 = 43_200.0;
pub const GOOD_FRACTION: f64 = 0.10;
pub const SOFT_FRACTION: f64 = 0.30;
/// A record made at a scale farther than this from the scale in force is another
/// telescope's.
pub const SCALE_TOLERANCE: f64 = 0.05;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict { Good, SlightlySoft, ClearlyOut, Unmeasurable, NoRecord }
impl Verdict {
    /// `good`, `slightly_soft`, `clearly_out`, `unmeasurable` or `no_record`.
    pub fn as_str(self) -> &'static str;
}

/// `px` is a size in sensor pixels, or why there is none; `reference` likewise a size.
pub fn judge(px: Result<f64, Unmeasurable>, reference: Option<f64>) -> Verdict;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mark { pub px: f64, pub at: f64 }
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RecordView {
    pub tonight: Option<Mark>,
    pub earlier: Option<Mark>,
    /// Whether the focus aid has passed through focus tonight. False without a
    /// `tonight`.
    pub passed: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Of { Tonight, Earlier }

impl Kept {
    /// A settled reading of the focus aid. `passed` is the aid's `found()` after that
    /// reading. `scale` is the measured scale, an imaging solve's, or `None` while
    /// none has solved. True when the record changed.
    pub fn note(&mut self, px: f64, at: f64, scale: Option<f64>, passed: bool) -> bool;
    /// The record as it reads at `now`, for the measured scale in force.
    pub fn record(&self, now: f64, scale: Option<f64>) -> RecordView;
    /// What a reading is judged against at `now`.
    pub fn reference(&self, now: f64, scale: Option<f64>) -> Option<(f64, Of)>;
}
```

**`judge`**, in this order: `Err(TooFewStars)` is `Unmeasurable`; `Err(FarOut)` is
`ClearlyOut`; no reference is `NoRecord`; `px ≤ reference·(1 + GOOD_FRACTION)` is
`Good`; `px ≤ reference·(1 + SOFT_FRACTION)` is `SlightlySoft`; else `ClearlyOut`. A
reading smaller than the reference is `Good`.

**What is held**: `scale: Option<f64>`, `tonight: Option<(px, at, since, passed)>` and
`earlier: Option<Mark>`.

**The scale** given to all three methods is the measured one: the scale an imaging
solve gave, which 032 keeps. A caller never passes a scale worked out from the focal
length (§ The aid on the console says where the caller gets each of the two). So a
record made before any imaging solve holds no scale.

**Another telescope**: `foreign(scale)` is true when both the held scale and the one
given are `Some` and `|given/held − 1| > SCALE_TOLERANCE`. A record that holds no
scale is never foreign.

**`note(px, at, scale, passed)`**:
1. when `foreign(scale)`, drop `tonight` and `earlier`;
2. when a scale is given, hold it;
3. with no `tonight`: `tonight = (px, at, since: at, passed)`; changed;
4. with a `tonight` whose `at − since > NIGHT_S` (this reading's `at` against the held
   `since`): if there is no `earlier`, or the old `tonight`'s `px` is below
   `earlier`'s, `earlier` becomes the old `tonight`'s `(px, at)`; then `tonight = (px,
   at, since: at, passed)`; changed;
5. otherwise, two things, each on its own: when `px` is below `tonight`'s, `tonight`'s
   `px` and `at` become this reading's, `since` stays; changed. And when `passed` is
   true and `tonight`'s `passed` is false, `tonight`'s `passed` becomes true; changed.
   A `note` that does neither changes nothing, and a scale held in step 2 is then
   written at the next change only. `passed` is never set back to false within a
   night;
6. when changed, write the file. It answers whether it changed.

**`record(now, scale)`**: empty, with `passed` false, when `foreign(scale)`. Otherwise
`tonight` is shown while `now − since ≤ NIGHT_S`, and `passed` is its `passed`; a
`tonight` past that is not shown and counts with `earlier`, of which the smaller `px`
is shown as `earlier`, and `passed` is then false. Reading never writes.

**`reference(now, scale)`**, from `record(now, scale)`:
- a `tonight` with `passed` true: `tonight`'s `px`, `Tonight`;
- a `tonight` with `passed` false: `earlier`'s `px`, `Earlier`, when there is an
  `earlier` and its `px` is below `tonight`'s; otherwise `tonight`'s `px`, `Tonight`;
- no `tonight`: `earlier`'s `px`, `Earlier`, or `None` without one.

So the smaller part judges, which is what the glossary's one smallest value means, and
tonight's part judges alone only once the aid has shown, by passing through focus, what
tonight's sky allows (spec owner decision 8).

The sizes are sensor pixels and never arcseconds, so a record does not move when the
kept scale is refined by a later imaging solve.

### The aid's arithmetic (`crates/finder/src/imaging/focus/aid.rs`, new, US4)

```rust
//! The focus aid's arithmetic (033): what to tell a hand on the focuser, from the
//! readings alone. No clock, no camera, no file.

pub const WINDOW: usize = 3;
pub const CHANGE_FRACTION: f64 = 0.15;
/// Imaging frames set aside at a change: the one that showed it, and the next.
pub const SETTLE_FRAMES: u32 = 2;
pub const TREND_FRACTION: f64 = 0.05;
pub const NEAR_FRACTION: f64 = 0.10;
pub const RISE_FRACTION: f64 = 0.20;
/// The best so far is the smallest of this many settled readings.
pub const BEST_KEPT: usize = 600;
pub const HISTORY: usize = 120;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Sample {
    Reading { px: f64, stars: u32 },
    TooFewStars { stars: u32 },
    FarOut { px: f64, stars: u32 },
}
impl From<&super::Read> for Sample { /* a reading's px and stars, or the reason */ }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase { Reading, Settling, TooFewStars, FarOut }
impl Phase { /** `reading`, `settling`, `too_few_stars`, `far_out`. */ pub fn as_str(self) -> &'static str; }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trend { Better, Worse, Same }
impl Trend { /** `better`, `worse`, `same`. */ pub fn as_str(self) -> &'static str; }

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Median { pub px: f64, pub readings: usize }

/// What one sample did.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Step { pub settled: Option<f64> }

#[derive(Debug, Clone)]
pub struct Aid { /* below */ }
impl Aid {
    pub fn new() -> Self;
    /// Keep the best over `n` settled readings instead of `BEST_KEPT`.
    pub fn with_best_kept(self, n: usize) -> Self;
    pub fn push(&mut self, sample: Sample) -> Step;
    /// Samples pushed.
    pub fn readings(&self) -> u64;
    /// What the last sample was; `None` before the first.
    pub fn phase(&self) -> Option<Phase>;
    pub fn median(&self) -> Option<Median>;
    pub fn best(&self) -> Option<f64>;
    pub fn trend(&self) -> Option<Trend>;
    pub fn found(&self) -> bool;
    /// The last `HISTORY` samples, oldest first.
    pub fn history(&self) -> Vec<Option<f64>>;
}
impl Default for Aid { /* new() */ }
```

It holds: `window`, the readings held, at most `WINDOW`, oldest first; `skip`, frames
still to set aside; `level` and `before`, each an `Option<f64>`; `last_settled`;
`settled`, a `VecDeque<f64>` of at most the kept count; `trend`; `found`; `phase`;
`history`; and the count of samples.

**The median** of the window: the value itself with one, the mean of two, the middle of
three sorted. `median()` is `None` with an empty window.

**`push(sample)`** adds one to the count, pushes onto `history` (the `px` of a
`Reading` or a `FarOut`, `None` for `TooFewStars`, dropping the oldest beyond
`HISTORY`), and then:

1. `TooFewStars` or `FarOut`: when `last_settled` is `Some`, `before` becomes it; the
   window is emptied, `skip` and `level` cleared; the phase is that of the sample; no
   settled reading.
2. A `Reading` while `skip > 0`: `skip` falls by one; the phase is `Settling`; no
   settled reading.
3. A `Reading` with a window that is not empty, m its median, and `|px − m| >
   CHANGE_FRACTION·m`: a change. When `last_settled` is `Some`, `before` becomes it;
   the window is emptied and `level` cleared; `skip = SETTLE_FRAMES − 1`; the phase is
   `Settling`; no settled reading.
4. Any other `Reading` joins the window, the oldest leaving beyond `WINDOW`; the phase
   is `Reading`. With fewer than `WINDOW` readings held there is no settled reading.
   With `WINDOW`, m, its median, is a settled reading:
   - it joins `settled`, the oldest leaving beyond the kept count;
   - with no `level`, `level` becomes m; else when `|m − level| > TREND_FRACTION·level`,
     `before` becomes `level` and `level` becomes m;
   - when `before` is `Some(b)`: the trend is `Better` when `m < b·(1 − TREND_FRACTION)`,
     `Worse` when `m > b·(1 + TREND_FRACTION)`, else `Same`;
   - `last_settled` becomes m; `found` is worked out again (below); the step answers
     `Some(m)`.

`trend` and `found` keep their last value between settled readings.

**`best()`** is the smallest of `settled`, `None` while it is empty.

**`found`**: with b the best, a settled reading is near when it is at most
`b·(1 + NEAR_FRACTION)` and far when it is at least `b·(1 + RISE_FRACTION)`; one in
between is neither. Walking `settled` from the oldest with a stage that starts at 0: at
stage 0 a far one makes it 1; at stage 1 a near one makes it 2; at stage 2 a far one
makes it 3. `found` is true when the stage reached 3 and the newest settled reading is
near. It is worked out against the best of now, so a pass that ends on a slightly lower
best still counts the earlier pass as near (spec US4-S5).

The whole of spec US4-S1, step by step, as the rules give it (`·` is "unchanged"):

| n | sample | phase | median (held) | settled | trend | best | found |
|---|---|---|---|---|---|---|---|
| 1 to 4 | 6.0, 6.1, 5.9, 6.0 | reading | 6.0, 6.05, 6.0, 6.0 (1, 2, 3, 3) | at 3 and 4: 6.0 | none | 6.0 | no |
| 5, 6 | 4.5, 4.5 | settling | none | no | · | 6.0 | no |
| 7 to 9 | 4.4, 4.6, 4.5 | reading | 4.4, 4.5, 4.5 (1, 2, 3) | at 9: 4.5 | better at 9 | 4.5 | no |
| 10, 11 | 3.2, 3.2 | settling | none | no | · | 4.5 | no |
| 12 to 14 | 3.3, 3.1, 3.2 | reading | 3.3, 3.2, 3.2 | at 14: 3.2 | better | 3.2 | no |
| 15 to 19 | 2.8, 2.8, 2.9, 2.7, 2.8 | reading | 3.1, 2.8, 2.8, 2.8, 2.8 (3) | each | better | 3.1, then 2.8 | no |
| 20, 21 | 3.3, 3.3 | settling | none | no | · | 2.8 | no |
| 22 to 24 | 3.2, 3.4, 3.3 | reading | 3.2, 3.3, 3.3 | at 24: 3.3 | worse at 24 | 2.8 | no |
| 25, 26 | 4.0, 4.0 | settling | none | no | · | 2.8 | no |
| 27 to 29 | 4.1, 3.9, 4.0 | reading | 4.1, 4.0, 4.0 | at 29: 4.0 | worse | 2.8 | no |
| 30, 31 | 3.3, 3.3 | settling | none | no | · | 2.8 | no |
| 32 to 34 | 3.3, 3.3, 3.3 | reading | 3.3 | at 34: 3.3 | better at 34 | 2.8 | no |
| 35 | 2.85 | reading | 3.3 (3) | 3.3 | better | 2.8 | no |
| 36 to 39 | 2.85, 2.8, 2.9, 2.85 | reading | 2.85 (3) | each: 2.85 | better | 2.8 | yes |

At 15 the reading 2.8 is 12.5% below the median 3.2: under 15%, so it joins the window
and the settled reading 3.1 is within 5% of the level 3.2; at 16 the settled reading
2.8 is more than 5% from that level, so `before` becomes 3.2. At 35 the reading 2.85 is
13.6% below 3.3 and joins the window too.

### The camera lease (`crates/finder/src/imaging/mod.rs`, 031 US6; written by no story of this spec)

The plan document's amendment 1: "The console's imaging state (031) holds one lease
naming the holder; a second taker is refused with the holder's name. Each later spec
takes and releases that lease and adds no lock of its own." 031's trio carries it as
its User Story 6, and defines `RoiEdit` in its US1, beside the `SettingsEdit` that
holds it (031's plan.md § The console's imaging state). This section cites them; no
story of this spec creates, extends or changes any of it, and nothing here differs in
shape from 031's: the same four functions, the same six rules, and the same refusal
words. This trio, 035's (its plan.md § The camera's lease) and 037's take the lease in
this one shape. The first drafts of this trio wrote it down as an assumption, before
031's trio had it; the series audit gave it to 031 unchanged (spec.md § Clarifications,
the fifth session).

```rust
// 031's (the lease 031 US6; `RoiEdit` 031 US1, beside `SettingsEdit`), quoted:
/// The imaging camera, held for one taker. Dropping it releases the camera at once.
/// It holds no strong reference to the `Imaging`.
pub struct Lease { /* 031's */ }
impl Imaging {
    /// Take the camera for `holder`, a name that reads well after "by". While another
    /// holds it: a `Refusal` with the status 409 and the message
    /// `the imaging camera is held by {holder}`, naming that holder. It does not wait.
    pub fn lease(self: &Arc<Self>, holder: &'static str) -> Result<Lease, Refusal>;
    /// The holder's name while the camera is held.
    pub fn holder(&self) -> Option<&'static str>;
}
impl Lease {
    /// `Imaging::set` and `Imaging::take` for the holder: the same checks, refusals and
    /// words, less the lease's own.
    pub async fn set(&self, edit: SettingsEdit) -> Result<ImagingView, Refusal>;
    pub async fn take(&self, purpose: &str) -> Result<ImagingView, Refusal>;
}

/// The region of interest in a settings change, in sensor pixels: the body's
/// `{"x":…,"y":…,"width":…,"height":…}`, as 031's own scenarios send it.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoiEdit { pub x: u32, pub y: u32, pub width: u32, pub height: u32 }
```

031 US6's six rules, of which this spec relies on each and on nothing more:

1. `lease(holder)` hands the camera to the caller or answers a 409 `Refusal`, `the
   imaging camera is held by {holder}`, naming whoever holds it. It does not wait.
2. While a lease is out, `Imaging::set` and `Imaging::take` (the page's routes and the
   MCP tools of 031) are refused as in 1, with the holder's name, and that check comes
   before their check for an imaging frame in flight, though after their check of the
   camera's state (031 US6-S2: off the link every caller, the holder too, hears `the
   scope node is not connected`). So a second taker hears the holder's name even while
   the holder's own frame is exposing.
3. The holder's `set` and `take` behave as 031's `set` and `take` do for anyone while
   nobody holds the lease.
4. `Imaging::abort`, `Imaging::ended`, `Imaging::view` and 032's `Imaging::solve` are
   open to anyone, lease or no lease. Stopping an exposure is never refused.
5. Dropping the `Lease` releases the camera, also when the task that owns it is
   aborted, and a held `Lease` does not keep the `Imaging` alive.
6. A single imaging frame asked for with `Imaging::take` holds no lease. While it is in
   flight and nobody holds the lease, `holder()` is `None`, and `take` and `set` refuse
   as 031's plan has them (`an exposure is already in progress`, `an exposure is in
   progress`). This is 031's rule, not an assumption of this trio. Only spec US5-S2's
   one case of a start during such a frame rests on it, and that case asserts
   `holder()` is `None` there. `focus_start` itself would be right either way: it
   looks for a frame in flight only while nobody holds the lease, and otherwise lets
   the lease answer.

The tests of this spec assert of the refusal only the status 409 and that its words
contain the holder's name (spec.md § User Scenarios, "The lease"; § Plan traps 27), so
a later rewording of 031's sentence breaks nothing here. The re-anchoring
(§ Prerequisites 2) reads these names and rules in 031's code as landed; any difference
is corrected in this section, the task's "set" and "take" (§ The aid on the console),
spec US5-S2's one case, US5-S4, US5-S5 and the test setup's "The lease" before the
flip, and nowhere else, since nothing else in the trio names the lease.

`RoiEdit` is needed here because the task builds a `SettingsEdit` whose `roi` is
`Some(Some(RoiEdit { .. }))` for a tile and `Some(None)` for the whole sensor.

### The aid on the console (`crates/finder/src/imaging/focus/run.rs`, new, US5; US8 adds the check)

```rust
//! The focus aid and the focus check on the console (033).

/// The purpose label of every imaging frame the aid takes.
pub const PURPOSE: &str = "focus";
/// The name the aid and the check hold the imaging camera's lease under.
pub const HOLDER: &str = "the focus aid";
/// The focus exposure when the request names none.
pub const FOCUS_EXPOSURE_MS: f64 = 1000.0;
/// US8: a focus check ends after this many imaging frames at the latest.
pub const CHECK_FRAMES: u64 = 9;

/// The body of `POST /api/imaging/focus`. Tests outside the crate build it with
/// `FocusAsk::default()` and set fields; inside the crate a literal with
/// `..Default::default()` is fine.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct FocusAsk {
    /// US8.
    #[serde(default)] pub check: bool,
    #[serde(default)] pub find: bool,
    #[serde(default)] pub tile: Option<TileAsk>,
    #[serde(default)] pub exposure_ms: Option<f64>,
}
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TileAsk { pub col: u32, pub row: u32 }

impl Imaging {
    /// Start the focus aid, or (US8) with `ask.check` a focus check.
    pub async fn focus_start(self: &Arc<Self>, ask: FocusAsk) -> Result<ImagingView, Refusal>;
    /// Stop it and wait until it has ended. Nothing when none runs.
    pub async fn focus_stop(&self) -> ImagingView;
    /// Whether an aid or a check is running, whatever the camera's state. It keeps
    /// nobody out: the lease does that.
    pub fn focus_running(&self) -> bool;
    /// US8: the focus view once no aid or check is running; `None` when the view has
    /// no `focus`.
    pub async fn focus_ended(&self) -> Option<FocusView>;
}
```

US5 writes `FocusAsk` with `find`, `tile` and `exposure_ms`, and US8 adds `check`,
`CHECK_FRAMES`, `focus_ended` and step 4i below. Until US8 a body that holds `check`
is an unknown key and answers 400; no test of US5 sends one or asserts that (§ Plan
traps 14). `#[non_exhaustive]` is what lets US8 add the field without breaking a
literal in a test.

The methods are `impl Imaging` blocks in a descendant module of the one that defines
`Imaging` (`crates/finder/src/imaging/mod.rs`, 031 US1), so they reach its private
state, as 032's `solving.rs` does. `focus.rs` gains `pub mod aid;` (US4) and
`pub mod run;` (US5).

**`focus_start`**, refusing in this order:
1. the camera is not `ready`: 409 with 031's words for the state, written here as 031's
   plan gives them: `no_link`, `the scope node is not connected`; `no_service`, `this
   scope node has no imaging service`; `no_camera`, `this scope node has no imaging
   camera`; `error`, `the imaging camera is not answering: {error}`;
2. no solver (`self.solver()` is `None`): 409 `this console has no imaging solver`;
3. an aid or check is running: 409 `the focus aid is already running`;
4. nobody holds the lease (`self.holder()` is `None`) and the view's frame is
   `exposing` or `receiving`: 409 `an exposure is in progress`. While somebody holds
   the lease this check is passed over, whatever the frame's state, and check 8
   answers with the holder's name;
5. `find` true with a `tile`: 400 `give find or tile, not both`;
6. a `tile` that `Tile::new` refuses: 400 `tile col and row must each be 0, 1 or 2`;
7. an `exposure_ms` that is not finite or is outside the camera's range: 400
   `exposure_ms {value} is outside the camera's range {min}–{max} ms`, 031's text, the
   numbers with `Display` and an en dash;
8. `self.lease(HOLDER)` refused: that `Refusal`, unchanged (031 US6's 409, `the
   imaging camera is held by {holder}`).

The eight checks are not one atomic step. The console runs on tokio's multi-threaded
runtime (`crates/finder/Cargo.toml:33`), so two starts can pass checks 1 to 7 at the
same moment on two threads, and having no await between the checks does not prevent
that. What makes two starts exclusive is the lease: check 8 hands the camera to one,
and the other is refused there, with the lease's words, which name `the focus aid`. A
start that follows another by any interval a person or a test can make is refused by
check 3, with `the focus aid is already running`. Both are a 409, and no test asserts
which of the two a same-instant start gets. Do not hold the imaging state's lock from
check 3 to check 8 to make them one step: `lease` may take that same lock.

A refused start has changed nothing: no setting, no file, no focus state. Otherwise it
puts a new run in the focus state with the lifecycle `starting`, `since` from the
console's clock, a new `Aid` and the exposure (US8: and the ask's `check`), hands the
`Lease` to the task it spawns, and answers `self.view()`.

**Two scales.** `shown` is `Imaging::scale_arcsec_per_px()`: the measured scale, or
without one the focal length's. It turns sizes into arcseconds for the view and for
`focus::read`. `measured` is the scale an imaging solve gave and nothing else:
`self.view().relation.and_then(|r| r.scale).map(|s| s.arcsec_per_px)`, 032's kept
scale, in arcseconds a sensor pixel, `None` until an imaging solve has solved on this
camera. Only `measured` is ever passed to `Kept::note`, `Kept::record` and
`Kept::reference` (§ The record and the verdict; spec owner decision 10). Each is read
again where it is used, since a solve can end while the aid runs.

**The task** owns the `Lease` from its first line to its last, holds a `Weak` of the
`Imaging` and upgrades it for each step, so that dropping the last `Arc` ends it; it
never holds the imaging state's lock across an await. With `sensor` the region (0, 0,
the camera's width, its height), `b0` the smallest of the camera's binnings, e the
focus exposure, "set" meaning `Lease::set` with exactly the keys `exposure_ms`,
`binning`, `roi` and `save_at_node` of 031's `SettingsEdit`, and "take" meaning
`Lease::take(PURPOSE)`, whose answer is the imaging view with the new frame in it: the
frame's id, wherever a step below needs it, is that view's `frame.id`.

1. **Remember** the view's settings: `exposure_ms`, `binning`, `save_at_node`, and
   `roi`, as "the whole sensor" when it equals `sensor`.
2. **Choose the region.**
   - With `ask.tile`: `keep_region(KeptRegion::fixed(sensor, b0, tile, now))`.
   - Else with `ask.find`, or with no kept region: **search** (below). With a `best`:
     `keep_region(KeptRegion::found(sensor, b0, best, its reading))`. With no `best`
     and a `far_out` tile: the run reads that tile **for the time being**; nothing is
     kept, and the run remembers that its tile is provisional. With neither: the run
     ends `no_region` with `no tile of the imaging frame had five stars to measure`.
   - Else the kept region's tile.
   The region asked of the camera is always `tile_region(sensor, b0, tile)`, worked out
   again from the tile, so a region kept for another binning cannot be refused.
3. **Set** e, `b0`, that region and no node copy. The lifecycle becomes `running`.
4. **One frame**, repeated:
   a. when a stop was asked for, end `stopped`;
   b. take. A `Refusal` ends the run `failed` with the refusal's message. The frame's
      id is the answered view's `frame.id`;
   c. `Imaging::ended(id)`. When a stop was asked for, or the frame is `aborted`, end
      `stopped`. A `failed` frame ends the run `failed` with the frame's `error`. A
      `saved` frame: **remove the aid's previous saved frame** (below), then remember
      this frame's id and `file` as the previous one;
   d. a `saved` frame that lacks `width`, `height`, `bits`, `binning`, `start` or
      `file`, which 031's `FrameView` holds as `Option`s and a saved frame always has,
      ends the run `failed` with `the imaging frame was not saved`;
   e. build the `SolveFrame` from the frame's view, as 032's `Imaging::solve` builds
      its own: the path is the imaging folder joined with `file`, `roi` is
      `Region::from` of the view's `RoiView`, `exposure_s` is `exposure_ms/1000`; then
      `focus::read` of the whole frame with `shown`. An error ends the run `failed`
      with its `message()`;
   f. `Aid::push(Sample::from(&read))`; `cadence_ms` is the console clock's time since
      the last sample, in milliseconds, absent for the first; `last` is this sample,
      with `at` the middle of this frame's exposure, the `SolveFrame`'s `start +
      exposure_s/2`, whatever the sample was: a reading, a settling frame, one with
      too few stars or a far-out one;
   g. the verdict. For an unmeasurable frame it is `judge(Err(it), reference)`; for a
      settled reading m, `judge(Ok(m), reference)`; otherwise there is none. The
      reference is `kept.reference(now, measured)`, read **before** step h, and is
      kept beside the verdict. So a settled reading is never judged against itself:
      the first one of a camera with no record is `no_record`;
   h. a settled reading m is then noted: `kept.note(m, at, measured, aid.found())`,
      `at` being that of step f. US8: not with `check`;
   i. US8, with `check`: a settled reading or an unmeasurable frame ends the run
      `done`; so does the count reaching `CHECK_FRAMES`, with the verdict
      `Unmeasurable`;
   j. while the run's tile is provisional, a settled reading starts the **search**
      again. With a `best`: `keep_region(KeptRegion::found(..))`, the tile is no
      longer provisional, and set e, `b0`, that tile's region and no node copy.
      Without one: set the provisional tile's region again, and the next settled
      reading searches again. Either way the lifecycle is `running` again and the
      loop goes on at a. The `Aid` is not reset: the readings held carry over, and a
      reading on the new tile that differs by more than 15% is a change like any
      other. A check never reaches this step, since step i ends it first.
5. **End** with a reason and, for every reason but `stopped` and `done`, the words as
   `error`. When the view's frame is `exposing` or `receiving`, await `Imaging::ended`
   of it; then set the remembered four. A `Refusal` there is kept as `restore_error`,
   its message. Then drop the `Lease`; only then does the lifecycle become `ended`,
   and whoever waits in `focus_ended` or `focus_stop` is woken. So a reader who sees
   `ended` finds the camera free and the settings as they will stay.

**The search** (step 2 and step 4j): the lifecycle becomes `finding`; set e, `b0`, the
whole sensor (`roi: null`) and no node copy; take one frame as steps 4a to 4d do, so a
stop ends the run `stopped`, a failed frame ends it `failed`, and the frame becomes
the aid's previous saved frame like any other; build its `SolveFrame` as 4e does,
without feeding the aid; then `find_region(solver, frame, id, shown)`. An error ends
the run `failed` with the error's `message()`. A failed set anywhere in steps 2, 3 or
4j ends the run `failed` with the refusal's message.

**Removing the aid's previous saved frame** (step 4c). The aid removes the previous
frame's file and the file beside it whose name ends `.solve.json` in place of `.fits`,
ignoring a missing one and printing any other failure to standard error. One
exception: when the imaging view's `solve` (032's `SolveView`) has that frame's id as
its `frame_id` and the `state` `solving`, the frame is left in place for good, and so
is the solve sidecar its solve will write; the aid forgets it and never comes back to
it. 032 solves only the latest imaging frame and refuses one that is still arriving,
so a solve can start on an aid frame only in the gap while the aid measures it (step
4e), and only one solve runs at a time: when a frame's turn to be removed comes, a
solve of it is either still running, and the frame stays, or has ended and written
its sidecar, and both go. No solve loses its frame and no sidecar is left without one
(spec FR-024, US5-S7). The last frame the aid saved is never removed.

The remembered four are put back as one set: `exposure_ms`, `binning`, `save_at_node`,
and `roi` as `null` when the remembered region was the whole sensor. Every change of
the focus state wakes 031's watch of changes, if 031's state has one for the view; the
page reads the view once a second either way.

While the task holds the `Lease`, a `POST /api/imaging/frame`, a `PUT
/api/imaging/settings` or the MCP tools that do the same are refused by 031 with a 409,
`the imaging camera is held by the focus aid` (§ The camera lease, 2). The aid does
nothing about them and runs on. The first draft's endings `camera_busy` and
`settings_changed` do not exist.

**`focus_stop`**: with nothing running it answers the view. Otherwise it marks the stop,
calls `Imaging::abort()`, which ends an exposure in flight and does nothing otherwise
and which the lease does not guard, awaits the end and answers the view. It takes no
lease of its own: the task still holds the aid's, and has dropped it by the time the
end is signalled.

**`focus_running`** is true while the lifecycle is `starting`, `finding` or `running`,
read from the focus state alone, so it answers in every state of the camera.

**`focus_ended`** (US8) waits on the focus state's `Notify` until the lifecycle is not
`starting`, `finding` or `running`, and then answers `self.view().focus`. That is
`None` when the camera is not `ready` by then, or the console has no solver.

### The focus view (`crates/finder/src/imaging/focus/run.rs`, US5; US8 adds `check`)

```rust
/// `focus` in the imaging view. Every size is in `unit`. Every `Option` is skipped
/// while `None`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct FocusView {
    /// `arcsec` when the console knows the scale, else `px`: sensor pixels.
    pub unit: &'static str,
    pub arcsec_per_px: Option<f64>,
    pub region: Option<RegionOut>,
    pub record: Option<RecordOut>,
    pub aid: Option<AidView>,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RegionOut {
    pub col: u32, pub row: u32, pub x: u32, pub y: u32, pub width: u32, pub height: u32,
    /// `found` or `fixed` for the kept focus region; `provisional` for a tile a
    /// running aid reads for the time being.
    pub source: &'static str,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RecordOut { pub tonight: Option<SizeAt>, pub earlier: Option<SizeAt> }
/// A size and when it was read.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SizeAt { pub value: f64, pub at: f64 }

#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct AidView {
    /// `starting`, `finding`, `running` or `ended`.
    pub state: &'static str,
    /// US8 adds this field; US5's view has none.
    pub check: bool,
    /// Console Unix seconds at the start.
    pub since: f64,
    pub exposure_ms: f64,
    /// Samples so far.
    pub readings: u64,
    pub cadence_ms: Option<f64>,
    pub last: Option<LastView>,
    pub median: Option<MedianView>,
    pub best: Option<f64>,
    /// `better`, `worse` or `same`.
    pub trend: Option<&'static str>,
    pub found: bool,
    /// `good`, `slightly_soft`, `clearly_out`, `unmeasurable` or `no_record`.
    pub verdict: Option<&'static str>,
    pub reference: Option<ReferenceView>,
    /// The last 120 samples, oldest first; `null` for a frame with too few stars.
    pub history: Vec<Option<f64>>,
    /// `stopped`, `failed` or `no_region`; US8 adds `done`.
    pub ended_reason: Option<&'static str>,
    pub error: Option<String>,
    pub restore_error: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LastView {
    pub frame_id: u64, pub at: f64,
    /// `reading`, `settling`, `too_few_stars` or `far_out`.
    pub state: &'static str,
    pub value: Option<f64>, pub stars: Option<u32>,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MedianView { pub value: f64, pub readings: usize }
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ReferenceView { pub value: f64, /** `tonight` or `earlier`. */ pub of: &'static str }
```

The view is built when it is asked for, from the focus state and the two scales of
that moment (§ The aid on the console, **Two scales**). With a `shown` scale s, `unit`
is `arcsec`, `arcsec_per_px` is s and every size is its sensor pixels times s; without
one `unit` is `px`, `arcsec_per_px` is absent and every size is its sensor pixels. The
sizes are `record.*.value`, `last.value`, `median.value`, `best`, `reference.value` and
every number of `history`. `record` is `kept.record(now, measured)`, absent when it has
neither part; its `passed` is not in the view (the file has it, and the verdict's
`reference.of` says which part judged). `region` is the tile a running aid reads for
the time being, with the `source` `provisional`, while a run that has one is not
`ended`; otherwise the kept focus region with its own `source`; absent with neither.
`last.value` is the frame's own size: a reading's or a settling frame's `px`, a far-out
frame's rough size, absent with too few stars; `last.at` is the middle of that frame's
exposure, whatever the frame was (§ The aid on the console, step 4f). `aid` is the
latest run, running or ended, absent before the first start of this console's run. A
console with a solver and no scale that has never started the aid serializes `focus` as
exactly `{"unit":"px"}` (spec US5-S2).

### The imaging state (`crates/finder/src/imaging/mod.rs`, US1; US5)

- US1: `pub mod focus;`, in alphabetical order among the module lines.
- US5: `Imaging`'s state gains one value, the focus state. Its type is defined in
  `run.rs` with private fields, so that US8 adds to it without writing `mod.rs`:
  `focus::run::State`, made with `State::new(root.join("focus.json"))` where the
  imaging state is made. It holds a `focus::Kept` made with `Kept::new` of that path,
  the latest run (its lifecycle, its `Aid`, what step 4 keeps, whether its tile is
  provisional, the stop mark, the task's handle; from US8 its `check`) and a
  `tokio::sync::Notify` for the end, on which `focus_stop` waits (and, from US8,
  `focus_ended`). The `Lease` is not in the state: the task owns
  it. `root` is the imaging folder, where 032 keeps `relation.json` (the plan
  document's amendment 3). Where 031's task keeps the camera's description on
  reaching `ready`, and 032 calls `relation.describe`, US5 adds
  `kept.describe(CameraId { .. })` from the same description. The run's task is aborted
  where 031's own task is when the `Imaging` is dropped.
- US5: `ImagingView` gains, after 032's `relation`:

  ```rust
  #[serde(skip_serializing_if = "Option::is_none")]
  pub focus: Option<focus::run::FocusView>,
  ```

  filled by `Imaging::view` only while `state` is `ready` and the console has a solver.

### The routes (`crates/finder/src/imaging/routes.rs`, US5)

`.route("/api/imaging/focus", post(post_focus).delete(delete_focus))`.

- `post_focus` takes `SessionWrite` and `Result<Json<FocusAsk>, JsonRejection>` through
  `sim_json`, as 031's `put_settings` does: 415 for a `Content-Type` other than
  `application/json`, 400 for a body that is not JSON, an unknown key or a value of the
  wrong kind, before anything else. The words of such a 400 are axum's own, from the
  rejection's `body_text()`, and no scenario asserts them. Without imaging it answers
  409 `this console takes no imaging frames`; a `Refusal`, the lease's included,
  becomes `ApiError::new(status, message)` with its own status and words; otherwise
  `Json(view)` with 200. The page and every test send `{}` at the least, with
  `Content-Type: application/json`.
- `delete_focus` takes `SessionWrite` and no body; the same 409 without imaging;
  otherwise `Json(focus_stop().await)`.

### The binary (`crates/finder/src/main.rs`, US8)

Where 032 builds the simulated solver adapter under `--simulate`, after the state is
chosen (`crates/finder/src/main.rs:104`), it becomes
`SimulatedSolver::new(sim).with_measure_noise(32, 0.02).with_sharpest_at(0.60, 0.45, 3.0)`.
No flag and no new start-up line. With these the middle tile, whose centre is 0.1755 of
the half-diagonal from the sharpest point, reads 3.0858 sensor pixels at best focus and
5.6305 at a focus error of 2.0. Its nearest rival is the tile above it, whose centre is
0.3553 of the half-diagonal away: 3.7794 at best focus, 22% more, and 6.0386 at a focus
error of 2.0, 7.2% more. Spec US8-S3 finds the region at a focus error of 2.0, so 7.2%
is the margin that scenario has, against at most 4.1% that 2% of noise either way can
close (1.02/0.98).

### `crates/finder/src/lib.rs` and `crates/finder/README.md`

`lib.rs`: US5 adds two lines after 032's route line, which follows
`crates/finder/src/lib.rs:35`, written as they stand here, with no backticks:
`* POST /api/imaging/focus: {} or any of find, tile and exposure_ms; start the focus aid.`
and `* DELETE /api/imaging/focus: no body; stop the focus aid and put the imaging settings back.`
US8, which adds the key `check`, changes the first of the two to
`* POST /api/imaging/focus: {} or any of check, find, tile and exposure_ms; start the focus aid, or with check a focus check.`

`crates/finder/README.md`:
- US1 adds a row after the `POST /api/sim/frames` row (`crates/finder/README.md:374`):

  ```markdown
  | `GET /api/sim/focus`, `POST /api/sim/focus` | `{focus_error}`, from -100 to 100: the simulated focuser. 0 is best focus; the simulated imaging camera's stars grow with its size either way. Answers `{focus_error}`. |
  ```

- US5 adds two rows after 032's `POST /api/imaging/solve` row of the `## API` table
  (`crates/finder/README.md:167`). The first is written without `check`, which US8
  adds to it when it adds the key:

  ```markdown
  | `POST /api/imaging/focus` | JSON `{}` or any of `find`, `tile` (`{col, row}`, each 0 to 2) and `exposure_ms`. Start the focus aid on the focus region; answers 200 with the imaging view, whose `focus.aid` reports it, or 400 or 409 with a JSON `error`. Same-origin writes only. |
  | `DELETE /api/imaging/focus` | No body. Stop the focus aid, put the imaging settings back and answer 200 with the imaging view. Same-origin writes only. |
  ```

- US8 changes the first of those two rows to its final words:

  ```markdown
  | `POST /api/imaging/focus` | JSON `{}` or any of `check`, `find`, `tile` (`{col, row}`, each 0 to 2) and `exposure_ms`. Start the focus aid on the focus region, or with `check` a focus check that stops by itself with a verdict; answers 200 with the imaging view, whose `focus.aid` reports it, or 400 or 409 with a JSON `error`. Same-origin writes only. |
  ```

- US8 also adds a section `## Focus` after 032's `## Solving imaging frames`, saying,
  each in a sentence or a short paragraph: what a focus reading is and that the imaging solver
  measures it; that it is in arcseconds once a scale is known and in sensor pixels
  until then; the five-star rule and the 20-pixel rule; the nine tiles and how the
  focus region is found, kept and fixed; that far out of focus, where no tile can be
  measured, the aid reads the far-out tile with the smallest size for the time being,
  shows the rough size, keeps nothing, and looks for the sharpest tile again at its
  first settled reading; the focus record's two parts, what sets it, the 12-hour
  night, and that the smaller part judges until the aid has passed through focus
  tonight, after which tonight's judges alone; that the record is kept with the scale
  an imaging solve measured, the 5% rule, that a record made before any imaging solve
  carries no scale, and that `focus.json` can be removed to start again after the
  camera has been moved to another telescope before either was solved on; the five
  verdicts and their 10% and 30%;
  that no verdict says focus is good in absolute terms, because seeing, collimation, a
  mirror still cooling and tube currents enlarge stars as a focus error does; how the
  aid runs (the region of interest at the smallest binning, the median of three, a
  change and the two frames set aside, the trend, the best over 600 settled readings,
  "found" as far, near, far, near), with the advice to turn the focuser one way through
  the lowest reading and come back; the focus check and its nine frames; that the aid
  and the check hold the imaging camera while they run, so that an imaging frame or a
  settings change asked for meanwhile is refused with `the focus aid` in the words, and
  that a start is refused in the same way while something else holds the camera; what
  the aid does to the camera's settings and that it puts them back, with the four
  endings (`stopped`, `done`, `failed`, `no_region`); that the aid's imaging frames are
  saved with the purpose label `focus` and removed but for the last, and but for one
  an imaging solve was running on; the `focus` object
  and `focus.json`, which lies in the imaging folder; to track a field of stars first,
  away from any bright nebula; that a sparse field, such as one far from the Milky
  Way, may leave a tile with fewer than five stars at the default focus exposure, and
  that the cure is a longer one (`exposure_ms`, the page's Focus frame time) or
  focusing on a richer field nearby first and coming back; that `cadence_ms` is how long one reading takes, and
  that a forwarding limit makes it longer, so it is cleared while focusing; and to
  choose `Find the sharpest tile again` once focus is close, if the region was found
  while it was still well out, and after a change of telescope or collimation.
- US7 adds `get_focus_reading` and `set_focus_aid` to the tool list in `## MCP clients`
  (`crates/finder/README.md:57`), after 032's `solve_imaging_frame`, worded like their
  neighbours.

### The page (`crates/finder/web/src/`, US6)

`api.ts`, after 032's imaging types:

```ts
export interface ImagingFocusSized { value: number; at: number }
export interface ImagingFocusAid {
  state: "starting" | "finding" | "running" | "ended"; check: boolean; since: number;
  exposure_ms: number; readings: number; cadence_ms?: number;
  last?: { frame_id: number; at: number; state: "reading" | "settling" | "too_few_stars" | "far_out"; value?: number; stars?: number };
  median?: { value: number; readings: number }; best?: number;
  trend?: "better" | "worse" | "same"; found: boolean;
  verdict?: "good" | "slightly_soft" | "clearly_out" | "unmeasurable" | "no_record";
  reference?: { value: number; of: "tonight" | "earlier" };
  history: (number | null)[];
  ended_reason?: "stopped" | "done" | "failed" | "no_region";
  error?: string; restore_error?: string;
}
export interface ImagingFocus {
  unit: "arcsec" | "px"; arcsec_per_px?: number;
  region?: { col: number; row: number; x: number; y: number; width: number; height: number; source: "found" | "fixed" | "provisional" };
  record?: { tonight?: ImagingFocusSized; earlier?: ImagingFocusSized };
  aid?: ImagingFocusAid;
}
export interface ImagingFocusAsk { exposure_ms?: number; find?: boolean; tile?: { col: number; row: number }; check?: boolean }
// ImagingView gains: focus?: ImagingFocus;
export async function postImagingFocus(ask: ImagingFocusAsk): Promise<ImagingView>;
export async function deleteImagingFocus(): Promise<ImagingView>;   // fetch(url, { method: "DELETE" })
export async function putImagingWholeSensor(): Promise<ImagingView>;
```

`postImagingFocus` sends `{ method: "POST", headers: { "Content-Type":
"application/json" }, body: JSON.stringify(ask) }`; the block builds `ask` with its keys
in the order `exposure_ms`, `find`, `tile`, `check`, leaving out a key that is not set.
`putImagingWholeSensor` sends `PUT /api/imaging/settings` with the same headers and the
body `{"roi":null}`; 031's `ImagingSettingsEdit` leaves `roi` out, so this is a call of
its own. Each throws an `Error` with the answer's `error` on a refusal, as `postTrack`
does (`crates/finder/web/src/api.ts:312`).

`components/FocusBlock.tsx`, `FocusBlock({ view, refresh }: { view: ImagingView;
refresh: () => Promise<void> | void })`: nothing without `view.focus`; else
`<section aria-labelledby="focus-title">` holding
`<h2 id="focus-title" className="section-title">Focus</h2>`, with these words exactly,
each phrase a test asserts on one line of JSX. The `aria-labelledby` is what gives the
section the role `region` and the name `Focus` that spec US6-S1 looks for; a `section`
with no accessible name has no such role. The chart header's frame is labelled the same
way (`crates/finder/web/src/components/ChartHeader.tsx:44`). `size(v)` is `v.toFixed(2)` followed by `″` when
`unit` is `arcsec` and by ` px` when it is `px`. "Running" is an `aid` whose `state` is
not `ended`.

- Always: the `quiet` paragraph (`crates/finder/web/src/index.css:141`) `A focus
  reading is the size of the stars in the focus region: smaller is sharper. The console
  judges it against this camera's own focus record and never against a fixed number,
  because seeing, collimation and a warm mirror enlarge stars too. Use a field of stars
  away from any bright nebula.`
- With `unit` `px`: a `quiet` paragraph `No imaging scale is known yet, so sizes are in
  sensor pixels. Solve an imaging frame, or start the console with --focal-length-mm.`
- With a `region`: a `quiet` paragraph, name being, by `row` then `col`, `top left`,
  `top centre`, `top right`, `middle left`, `centre`, `middle right`, `bottom left`,
  `bottom centre`, `bottom right`. For the `source` `found` or `fixed` it is `Focus
  region: the {name} tile, {how}.`, how being `found from an imaging frame` or `set by
  hand`. For the `source` `provisional` it is `Focus region: the {name} tile for now.
  The sharpest tile is found once the stars are small enough to measure.`
- With a `record`: a `quiet` paragraph `Focus record: {size} tonight · {size} on
  earlier nights.`, or with one part only `Focus record: {size} tonight.` or `Focus
  record: {size} on earlier nights.`
- Not running: a `select` labelled `Focus region` whose options are `Kept, or the
  sharpest tile`, `Find the sharpest tile again`, and the nine names above with a
  capital first letter, in that order; a number field labelled `Focus frame time`, in
  ms, starting at 1000; the buttons `Start focus aid` (`button button-primary`) and
  `Check focus` (`button`). A click sends `postImagingFocus` with `exposure_ms` the
  field's number; `find: true` for the second option or `tile` for one of the nine;
  and `check: true` from `Check focus`; then `refresh`. A refusal shows its words in an
  `alert` and calls no `refresh`. Drafts of the select and the field survive the
  one-second reads.
- Not running, when `view.settings.roi` is not (0, 0, `view.camera.width`,
  `view.camera.height`): a `quiet` paragraph `The camera is reading a region of its
  sensor.` and the button `Use the whole sensor` (`button`), which calls
  `putImagingWholeSensor`, then `refresh`; a refusal shows its words in the same
  `alert` the start buttons use and calls no `refresh`.
- Running: the button `Stop focus aid` (`button`), which calls `deleteImagingFocus`,
  then `refresh`; and no select, field or start button.
- `state` `starting`: a `status` `Starting the focus aid`. `finding`: a `status`
  `Finding the focus region on a full imaging frame`.
- `state` `running`, or `ended` with `ended_reason` `done`: the reading block.
  - A `group-title` line (`crates/finder/web/src/index.css:136`) `Focus reading`, and
    under it the reading, a paragraph with the classes `text-[2.35rem] font-[680]
    tracking-[-0.02em] sm:text-[2.75rem]`, the Display size as the chart header writes
    it (`crates/finder/web/src/components/ChartHeader.tsx:96`): `size(median.value)`,
    or `—` without a `median`.
  - A `status`, by `last`: none, `Waiting for the first imaging frame`; `reading` with
    `median.readings` 3, `Median of the last 3 imaging frames · {stars} stars`;
    `reading` with fewer, `{n} of 3 imaging frames since the last change · {stars}
    stars`; `settling`, `Settling after a change`; `too_few_stars`, `Not measurable:
    {stars} stars in the focus region, and 5 are needed`; `far_out`, `Far out of focus:
    stars about {size(last.value)} across`.
  - With `last.state` `too_few_stars`: a `quiet` paragraph `Try a longer Focus frame
    time, or a field with more stars.`
  - With `best`: `Best so far {size(best)}`.
  - With `trend`: `Sharper than before the last change`, `Softer than before the last
    change` or `About the same as before the last change`.
  - `found` true: `Passed through focus and came back to the best so far.`; false,
    while running: `Turn the focuser one way until the reading has passed its lowest
    and risen again, then come back to the lowest.`
  - With `verdict`, one line. `ref` is `size(reference.value)` followed by ` tonight`
    or ` from earlier nights`, and `pct` is `Math.round((median.value/reference.value −
    1)·100)`:
    `good`, `Within 10% of the focus record of {ref}.`;
    `slightly_soft`, `Slightly soft: {pct}% larger than the focus record of {ref}.`;
    `clearly_out` with a `reference` and a `median`, `Clearly out: {pct}% larger than
    the focus record of {ref}.`, and otherwise `Clearly out: the stars are too large to
    measure.`;
    `unmeasurable` with `last.state` `too_few_stars`, `Not measurable: too few stars in
    the focus region.`, and otherwise `Not measurable: the reading did not settle.`;
    `no_record`, `No focus record for this camera yet. Run the focus aid through focus
    to make one.`
  - The history: `<svg data-focus-history role="img" aria-label="Focus readings of the
    last {n} imaging frames" viewBox="0 0 {4·n} 40">` over the last 60 values of
    `history`, n of them. For the i-th, counted from 0, when it is a number v: `<rect
    x={4·i} width="3" height={h.toFixed(1)} y={(40 − h).toFixed(1)}
    fill="currentColor" />` with `h = 36·v/max`, max the largest number shown. A `null`
    draws nothing and leaves its place empty. It has `width="100%"` and
    `preserveAspectRatio="none"`, no class that sets a colour and no `style`.
- `state` `ended`: `stopped`, a `status` `Focus aid stopped.`; `done`, a `status` `Focus
  check done.` above the reading block; any other reason, an `alert` `Focus aid
  stopped: {error}.` With `ended_reason` `no_region`, also the `quiet` paragraph `Try a
  longer Focus frame time, or a field with more stars.`, the same words the reading
  block uses for a frame with too few stars. With a `restore_error`, also an `alert`
  `The camera's settings were not put back: {restore_error}.`

Only existing classes and tokens are used, and Tailwind utilities for spacing and the
Display size; nothing sets a colour, so the block is ink by day and red at night with
no rule of its own. `ImagingPage.tsx` (031 US4) renders `<FocusBlock view={view}
refresh={refresh} />` right after 032's `<ImagingSolveBlock … />`, only when the view
is `ready`. `useImaging` and `useSession` do not change: the page already reads
`GET /api/imaging` once a second, which is the aid's own pace.

### `DESIGN.md` and `.impeccable/design.json` (US6)

Three edits. The `>` marks only set the text apart here.

After 032's Imaging field entry, a plain `###` heading and one paragraph:

> ### Focus block — built
> On the Imaging page, after the imaging solve, while the console has an imaging
> solver: a section titled Focus. A standing quiet paragraph says what a focus reading
> is, that it is judged against this camera's own focus record and never a fixed
> number, and that seeing, collimation and a warm mirror enlarge stars too. Idle, it
> offers a Focus region select, a Focus frame time field, a primary button, Start focus
> aid, and a secondary one, Check focus. While the aid runs, the settled reading is set
> in the Display size, the one number read from a step away with a hand on the focuser,
> over a status line, the best so far, whether the last change made the stars sharper or
> softer, a line that says to keep turning or that focus was passed and regained, the
> verdict against the record, and a history of ink bars drawn with currentColor. Stop
> focus aid is the only control then. An ending that was not asked for is an alert, and
> too few stars, in a frame or at such an ending, adds a quiet line naming the cure.
> Existing ink, paper and red-only night tokens; no colour of its own.

In the Display line of the Hierarchy (`DESIGN.md:213`), `RA and Dec.` becomes `RA and
Dec, and the focus reading on the Imaging page.`

At the end of 031's Imaging page entry, after the sentence 032 adds:

> After the imaging solve comes the Focus block.

`.impeccable/design.json` gains one entry in `components`, after 032's `Imaging field`,
formatted as its neighbours (`.impeccable/design.json:723`): `name` `Focus block`,
`kind` `custom`, `refersTo` `button`, a `description` that says the entry in one
sentence each, and `html` and `css` snippets under the class `ds-focus-block`, using
only `var(--color-ink)` and `currentColor`.

### The MCP tools (`crates/finder/src/mcp/tools.rs`, US7)

Two structs that only describe the second tool's input, beside `TargetArguments`
(`crates/finder/src/mcp/tools.rs:31`) and written as it is; `FocusAsk` parses the
arguments and `Imaging::focus_start` checks them, so the route and the tool refuse
with the same words wherever the words are the console's own (every `Refusal`). A
parse error is each layer's own text: the route's is axum's, from the rejection's
`body_text()`, and the tool's is `serde_json`'s error as a string. No scenario
asserts either:

```rust
#[derive(rmcp::schemars::JsonSchema)]
#[expect(dead_code, reason = "schema only; FocusAsk parses the actual arguments")]
struct FocusAidArguments {
    on: bool,
    check: Option<bool>,
    find: Option<bool>,
    tile: Option<FocusTileArguments>,
    exposure_ms: Option<f64>,
}
#[derive(rmcp::schemars::JsonSchema)]
#[expect(dead_code, reason = "schema only; FocusAsk parses the actual arguments")]
struct FocusTileArguments { col: u32, row: u32 }
```

Two tools in the `#[tool_router]` block (`crates/finder/src/mcp/tools.rs:165`), each
with `output_schema = object_schema()` (`crates/finder/src/mcp/tools.rs:104`):

- `get_focus_reading`, read-only, description
  `Read the focus reading: the focus region, the focus record, and the reading, best, trend and verdict of the focus aid or the last focus check.`:
  takes `arguments: JsonObject`; any key is the error `get_focus_reading takes no
  arguments`. Without imaging it answers the error `this console takes no imaging
  frames`. With imaging and a view that has `focus`, `CallToolResult::structured` of
  that `focus` object. Otherwise the error a start would be refused with: 031's words
  for a camera that is not `ready`, or `this console has no imaging solver`.
- `set_focus_aid`, description
  `Start or stop the focus aid, as POST and DELETE /api/imaging/focus do. With check true it takes a focus check, waits until it ends and returns its verdict against the focus record.`,
  with `input_schema = rmcp::handler::server::common::schema_for_input::<FocusAidArguments>().expect("focus aid schema")`,
  as `set_target` declares its own (`crates/finder/src/mcp/tools.rs:355`): takes
  `arguments: JsonObject` and reads it itself, so that `{}` is a tool error and not a
  protocol one (`set_tracking`, whose `on` is in its parameter type, is why the
  every-tool test names it, `crates/finder/src/mcp/tools.rs:298`):
  - `on` absent or not a boolean: the error `on must be true or false`;
  - `on` false with any other key: `set_focus_aid with on false takes no other
    argument`; else, without imaging, `this console takes no imaging frames`; else
    `Imaging::focus_stop`, then the view's `focus`, or the error as
    `get_focus_reading` gives it when the view has none;
  - `on` true: the other keys are parsed as `FocusAsk` (the arguments less `on`,
    through `serde_json::from_value`), a parse error being the error with serde's
    words; without imaging, `this console takes no imaging frames`;
    `Imaging::focus_start`, a `Refusal` being the error with its words. Then, without
    `check`: `CallToolResult::structured` of the answered view's `focus`, or the error
    as `get_focus_reading` gives it when that view has none. With `check`:
    `Imaging::focus_ended().await`; `Some(focus)` is `CallToolResult::structured` of
    it; `None`, which means the camera was no longer `ready` when the check ended, is
    the error as `get_focus_reading` gives it then (031's words for the camera's
    state).

Neither is a moving tool: no rate limit and no call-log line. They are written as 031's
`take_imaging_frame` and 032's `solve_imaging_frame` are.

## Shapes that existing tests pin

These are not changed by any story:

- **The simulated solver adapter's star measure without the new builders.** 032's
  scenario asserts 2.825784, 5.492337 and 2.746168 frame pixels from
  `SimulatedSolver::new(sim)` (032 US2-S2). Noise and the sharpest point are off unless
  a builder is called, and US1-S4 asserts the same three numbers again.
- **The solver port.** A test implements `ImagingSolver` by hand (032 US2-S4), and this
  spec's scripted solver does too. No method is added to the trait, and `SolveFrame`,
  `Region` and `StarMeasure`, which tests build by literal, gain no field.
- **`ImagingOptics` and the simulator's other controls.** `ImagingOptics` gains no
  field; `set_imaging_optics` and `imaging_optics` are untouched; the two new methods
  write one field of it. The simulator's noise generator
  (`crates/simulator/src/noise.rs:4`) and its test of the published stream
  (`crates/simulator/src/noise.rs:99`) are not touched.
- **The simulate routes.** `/api/sim/telescope` and `/api/sim/frames` keep their
  bodies, and an unknown `/api/sim/*` path still answers 404 with and without a
  simulation (`crates/finder/tests/motion/simulate.rs:94`); `/api/sim/focus` is a new
  name. The README's sentence that every such path answers 404 outside simulate mode
  (`crates/finder/README.md:379`) stays true.
- **031's `take`, `set` and `abort` and their refusals.** These are pinned by 031's
  own scenarios, and no story here changes them. The aid takes frames and changes
  settings through the lease and adds no refusal, no lock and no holder of its own. A
  frame the aid takes is a saved imaging frame like any other until the aid removes
  its file. The lease itself is 031 US6's, pinned by that story's own scenarios, which
  no story here changes; no test on `main` names it yet (§ The camera lease).
- **`GET /api/imaging`, `GET /api/session` and the MCP bodies on a console with no
  imaging solver, or outside `ready`.** `focus` is skipped then, so every body 031's
  tests read is what it was. 032's tests have a solver and compare `solve` and
  `relation` key by key; they are re-read before the flip for a comparison of a whole
  view that a new `focus` key would break (§ Prerequisites 2).
- **`set_imaging_camera`'s nine keys** (031 US5-S1). The focus exposure and the tile
  are in the request's body, not in the imaging settings, which gain no key.
- **The MCP tool list as tests read it.** One test calls every listed tool with `{}`
  and checks for the token (`crates/finder/tests/mcp/motion.rs:517`); both new tools
  answer `{}` at once on a console with no imaging (US7-S4).
- **031's Imaging page tests and its end-to-end checks.** The block renders nothing
  without `view.focus`, shows no `status` or `alert` while idle, and uses no label,
  button name or region name that 031 or 032 use: its field is `Focus frame time`,
  never a name holding `Exposure`, because Playwright's label match is by substring and
  031's check sets `Exposure`. The simulated console's Imaging page gains the idle
  block, which is ink on paper by day and red at night, so 031's counts of pixels that
  are not red hold (031 US4-S8, counted as `crates/finder/web/e2e/session.spec.ts:45`
  counts them).
- **The files 031 writes.** A saved frame's cards, name and folder are 031's. The id of
  the next frame is the largest file number in the day's folder plus one after a
  restart; the aid keeps its last frame's file, so no id is used twice.
- **The binary's start-up lines and its flags.** Tests of the built binary read its
  address from the line that begins `finder console: http://`
  (`crates/finder/tests/cli/recording.rs:54`): nothing new is printed and no flag is
  added.
- **Finder's test layout** (`crates/finder/tests/layout/main.rs:109`): no file at the
  top level of `crates/finder/tests`, no new file in `support`, and each binary's
  `main.rs` holding only its support include and `mod` lines.
- **Shapes this spec pins for its own later stories and for 034 to 037.** `FocusView`,
  `AidView` and `FocusAsk` are `#[non_exhaustive]`: tests compare `focus` key by key
  and build a `FocusAsk` from `FocusAsk::default()`, which is what lets US8 add `check`
  to the last two. `FocusReading`, `KeptRegion`, `Tile`, `Sample` and `Search` are
  built or read field by field in this spec's tests, so each has all its fields from
  the story that creates it. `Kept`'s `new`, `describe`, `region`, `keep_region`, `note`,
  `record` and `reference` are called by tests and keep their signatures. `focus.json`
  once written is pinned: a key added later must be optional.
- **What 034 to 037 are given, so that none of them edits this spec's files to get
  it.** The holder's name `the focus aid`: a set of darks or flats, a stack or a run
  that asks for the lease while the aid or a check runs is refused by 031 with it, and
  the aid's start is refused with theirs, so no spec asks another whether it is
  running. `Imaging::focus_running()`, for a page or a tool that wants to *say* that
  the aid is running. A later spec may show it; it must not refuse on it. A refusal
  built on `focus_running()` is a second gate on the camera, which the plan document's
  amendment 1 forbids ("adds no lock of its own"), and it is redundant: the lease
  already refuses, with `the focus aid` in the words (§ Plan traps 26). 034's draft
  plans no such refusal: its plan.md § Prerequisites 5 leaves it to the lease.
  `Imaging::focus_start` with `check` and `Imaging::focus_ended()` for the run's focus
  check, and `FocusAsk`'s `exposure_ms` for a run that wants a longer focus exposure
  than the default at a sparse target (spec owner decision 16). `focus::read` and
  `focus::judge` for a sub's own reading. And the purpose label `focus`, which 034's
  calibration must treat as neither a dark, a flat nor a sub.
- **How 037's run shares the camera with its focus check.** A focus check takes the
  lease itself, so a run that held the lease across it would be refused with its own
  name. 037's draft has settled this on its side (its spec owner decision 15): the run
  gives the lease up just before it calls `focus_start` with `check`, awaits
  `focus_ended()`, reads the five verdict words as they are here (`no_record`
  included, which its owner decision 21 turns into a pause), and takes the lease
  again. That needs nothing more of this trio. Its alternative, a lease that 031 lets
  a holder lend to a step, would give `focus_start` a variant that takes a lease
  handed to it: a change to this trio made before the flip, and only if the owner
  chooses it there.

## Decisions and the alternatives rejected

| Decision | Rejected | Why |
|---|---|---|
| The focus reading is the solver port's star measure, turned into sensor pixels and arcseconds | A half-flux measure of this project's own | The plan document: ASTAP behind the solver port measures stars; 032 already carries `measure`. One measurer for the solve and the reading means a frame that reads well is one the solver sees stars in. |
| Sizes are held in sensor pixels and shown in arcseconds when a scale is known | Arcseconds throughout, refusing without a scale | An unfocused frame does not solve, so a console with no `--focal-length-mm` would have no scale and no way to get one. Pixels of one camera on one telescope compare exactly, and the record does not move when a solve refines the scale (spec owner decision 6). |
| Under five stars is unmeasurable; above 20 frame pixels is far out | Showing whatever the solver said | Five is the floor one focusing tool uses; ASTAP says it analyses stars above 20 incorrectly, and an obstructed telescope's stars are rings by then (`docs/research/2026-11-15-imaging-camera.md` § 6. Focus, § 3. ASTAP). |
| The focus region is found from nine tiles of a full frame | The sensor's centre; a low percentile over the whole frame | The sharpest part of the field is where the optical axis lands, 210 to 630 pixels off for 1 to 3 mm of miscollimation on the worked example. A percentile over the frame cannot be asked of the camera as a region of interest. |
| Nine tiles, one star measure each | Twenty-five; four; a slid window; one measure of the whole frame split by star position | To magnitude G 14 a ninth of the worked example's field holds 3 to 4 stars at the galactic poles, which is under `MIN_STARS`, and 9 to 11 at galactic latitude 30° (the same note, § 4. Star counts and limiting magnitude at this field: 30 to 36 and 78 to 97 a field). A twenty-fifth holds 1 to 2 and 3 to 4. So nine is already thin at the sparsest fields and anything finer fails at ordinary ones; four tiles hold more but none lies on the sensor's centre. Spec owner decisions 2 and 16 put the count and the focus exposure to the owner together (§ Risks). The port answers a median for a region, not a star list. |
| Far out of focus with no region, the aid reads the far-out tile with the smallest size for the time being and searches again at the first settled reading | Ending `no_region`; the centre tile whatever its size | A millimetre out, stars are 34 pixels across (the same note, § 6. Focus), so at first light every tile is far out and a search has nothing to choose; ending there leaves the observer with no number to turn the focuser by. A tile chosen then is not kept, because it was chosen by nothing that says where the optical axis lands (spec owner decision 24). |
| The aid reads a region of interest at the smallest binning | A binned full frame | Software binning shortens nothing on the cable and halves the sampling; only a region shortens the read (the same note, § 1. The camera). |
| The record has two parts; the smaller judges, and tonight's judges alone once the aid has passed through focus tonight | One smallest value for all nights; tonight's as soon as it has one settled reading (the first draft) | Seeing differs by half again from night to night, so one rare night would make every later night read clearly out under one smallest value. But tonight's part from its first settled reading on judges nothing: the first reading of a night, however soft, hides a sharper record of earlier nights, and a check after a short session reads good against it. A pass through focus is the evidence that tonight's smallest is what tonight allows (spec owner decision 8). |
| A settled reading is judged before it is noted | Noting first | Noted first, every new low is compared with itself and reads good, and the first reading of a new camera reads good against nothing. |
| Only a settled reading of the open aid sets the record | Every reading; a check's own reading | The smallest of many single frames is biased low by their scatter and sets a bar no frame meets. A check that set the record would be judged against itself the next time (spec owner decision 7). |
| A night is 12 hours from its first settled reading | The local date at the site | It needs no site and no time zone, and focus code reads no session field. |
| A record at another measured scale judges nothing; only a scale an imaging solve gave is kept with the record or compared | A record keyed by a telescope's name; the scale in force whatever its source | The console knows cameras, not telescopes; a reducer or another tube changes the scale by far more than 5%. The scale worked out from `--focal-length-mm` is nominal: were it kept with the record, the first imaging solve could find it more than 5% off, throw the record away, and the next check would answer `no_record` (spec owner decision 10). |
| A change is noticed from the readings: one frame more than 15% from the median | A button the observer presses; a focuser position | The focuser is turned by hand in the dark and has no scale. Touching a long telescope shakes it, so the frame taken then reads large: the change shows itself. |
| Two frames set aside at a change, then a fresh median of three | A running median straight through | The research: "wait a second or two after each focuser change"; and a median that mixes readings from before and after a turn tells the observer nothing for two frames. |
| The trend compares with the settled reading before the change, with a 5% dead band, and follows a slow turn | The slope of the last few readings | The observer's question is "did that turn help", not "what is the slope"; a slope of three noisy readings flips at random. |
| "Found" is far, near, far, near against the best of now | The best alone; near the best after any earlier larger reading | A reading that only ever fell has not been shown to be the lowest. The research note: "Best seen so far, with a rise on both sides" is the only verdict the data supports (`docs/research/2026-11-15-imaging-camera.md` § 6. Focus). |
| The best is the smallest of the last 600 settled readings | The best since the start; a best that expires by the clock | Seeing and a cooling mirror move what is possible; counting in readings keeps it true at any focus exposure (the plan document, rule 6). |
| A focus check is the aid with `check`, ended by its first settled reading | Judging the latest imaging frame as it is; a route of its own | The check is then measured as the record was made: the same region, binning and smoothing. One loop serves both (spec owner decision 12). |
| The aid's frames are saved by 031 as they are and removed once the next is saved | Keeping all; a no-file path in `Imaging::take` | The solver port measures a file. 031's `take` and its hand-written test doubles stay untouched. Nine hundred files of a quarter hour's focusing help nobody (spec owner decision 13). |
| A frame an imaging solve is running on when its turn to be removed comes is left for good | Removing it anyway; removing it later, once the solve has ended | The lease leaves 032's solve open to anyone, and a solve takes seconds. Removed anyway, the solve loses its file, or writes a solve sidecar beside nothing. Removing it later needs a list of frames to come back to and a wait at the end; leaving it costs one file for each solve somebody asked for in the middle of focusing. |
| The aid holds 031's lease (031 US6) from its start until the settings are back, under one name, `the focus aid` | A lock or a "running" flag of this spec's own that others must ask; ending the aid when another party's frame or settings change slips in (the first draft); releasing the lease between frames | The plan document's amendment 1: one holder at a time, each spec takes and releases the lease and adds no lock. Holding it through the restore means the next taker never sees the aid's region or exposure (spec owner decision 14). |
| `focus_start` looks for a frame in flight only while nobody holds the lease, and otherwise lets the lease answer | Looking for it always, before the lease (the first draft); leaving it all to the lease | A stack or a set of darks has a frame exposing most of the time it holds the camera. Looking always, a start then got `an exposure is in progress` and never the holder's name, against the amendment's "refused with the holder's name". Leaving it all to the lease would rest on 031's own single frame holding the lease, which 031 US6's rule 6 says it does not (§ The camera lease, 6). |
| Two starts at once are kept apart by the lease, not by the order of the checks | Holding the focus state's lock across the checks | The console answers on several threads, so checks without an await between them are still not one step; and `lease` may take the lock the focus state lives in. |
| The lease is asked for last, after the request's own checks | First | A request that would be refused anyway never holds the camera, and a refused start has changed nothing. |
| Stop aborts the exposure and waits for the settings to be back | Answering at once | The next request then finds the camera as the observer left it. |
| The request carries the focus exposure and the tile | Two more imaging settings | 031's MCP test counts nine settings keys, and neither value means anything outside the aid. |
| `focus.json` in the imaging folder, one camera a file | `session.json`; `relation.json` | The region and the record belong to a camera and move with `--imaging-dir`; 032's file is 032's to write. |
| `focus.rs` with two children | One file | The arithmetic is tested without a console and the loop without arithmetic; three stories would otherwise rewrite one large file. |
| `/api/sim/focus` | A flag; nothing | The plan document's § Simulator names a focus error control; the existing simulate controls are routes of this kind, behind the same checks. |
| Seeded noise that is uniform and bounded, written in the adapter | The simulator's Gaussian generator | It is private to its crate, and a bounded noise lets a scenario state its bound exactly. |
| A sharpest point in the simulated adapter | A flat field | Without it every tile ties and the search proves nothing; with it the simulator's truth names the tile. |
| The reading in the Display size; ink bars with `currentColor` | The Headline size; a chart library | It is read from a step away. The page adds no package, and `currentColor` is red at night with no rule. |
| No Playwright check | One in the simulate project | It would edit `playwright.config.ts` and leave the shared simulated console on a region of its sensor (spec owner decision 17). |
| `get_focus_reading` reads; `set_focus_aid` with `check` waits | A reading tool that moves the camera's settings | A tool named "get" is read-only everywhere else in this console (spec owner decision 19). |

## Verification (every story, before proposing)

- **The factory gates** (`ergane.yaml` at 49bb210):
  - `lint` = `cargo fmt --all --check`,
    `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`,
    `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features --locked`
    and `RUSTDOCFLAGS="-D warnings" cargo doc -p finder --no-deps`;
  - `test` = `cargo nextest run --workspace --all-targets --all-features --locked`,
    `cargo test --workspace --locked --doc`, and then, in `crates/finder/web`,
    `npm ci --no-audit --no-fund`, `npm run typecheck`, `npm run format:check`,
    `npm test`, `npm run build`, `git status --porcelain -- dist | (! grep .)` and
    `CI=1 npm run e2e`.

  Every story runs both, and the judge sees their results. `--locked` and no manifest
  change mean `Cargo.lock` is untouched.
- **CI in the merge queue** runs every `ci.yml` job, including `cargo-deny`,
  `cargo-machete`, the build on Rust 1.96 and the finder-page checks. No story adds a
  dependency, so neither tool has anything new to judge.

## Prerequisites before this spec is flipped to `ready`

1. **030-imaging-camera-link, 031-imaging-frames-on-console and
   032-imaging-solve-and-aim-point have landed.** Not met: all three are reviewed
   drafts in this worktree. Everything of theirs named here is
   from their plan.md as reviewed.
2. **The trio is re-anchored to `main` after all three land.** Not met. Every `path:line`
   was read at 49bb210, the drafting worktree's HEAD, which has all of 028 and none of
   030, 031 and 032. To do then: re-read every cited line of
   `crates/finder/src/api.rs`, `crates/finder/src/main.rs`, `crates/finder/src/lib.rs`,
   `crates/finder/src/mcp/tools.rs`, `crates/finder/README.md`, `DESIGN.md`,
   `.impeccable/design.json`, `crates/simulator/src/optics.rs`,
   `crates/simulator/src/noise.rs` and the test files 030, 031 and 032 change; give a line
   to every file of 030, 031 and 032 named without one; and check, in their code as landed:
   - that `SettingsEdit` can be built with exactly `exposure_ms`, `binning`, `roi` and
     `save_at_node`, that its `roi` names the whole sensor with `Some(None)`, and that
     `RoiEdit` holds `x`, `y`, `width` and `height`, as 031's plan.md defines them
     (§ The camera lease);
   - that a saved `FrameView` carries `roi`, `binning`, `file`, `start` and
     `exposure_ms`, and that its `roi` is the region asked for;
   - the exact words of `set`'s refusal while an exposure is in progress, which
     `focus_start` answers with, and the four texts for a camera that is not `ready`;
   - that `Imaging::abort` ends a frame the aid's task is awaiting in `ended`;
   - the lease of 031 US6, name for name (§ The camera lease, § Prerequisites 11):
     how it is taken and released, how its holder takes a frame and changes the
     settings, that its refusal is a 409 whose words contain the holder's name
     (`the imaging camera is held by {holder}` as drafted), that it is checked before
     the frame in flight in `take` and `set`, that `abort` is not guarded by it, that a
     held lease does not keep the `Imaging` alive, and that 031's own single imaging
     frame holds none while in flight (031 US6's rule 6);
   - that 032's imaging view carries `solve` with `frame_id` and `state`, and
     `relation.scale.arcsec_per_px` only once an imaging solve has solved, which the
     aid's removal rule and its `measured` scale read (§ The aid on the console);
   - that 032 refuses a solve while the latest imaging frame is `exposing` or
     `receiving`, and writes a solve sidecar for a failed solve too, as spec US5-S7
     assumes;
   - that `Imaging`'s state can take the focus state, that its task has one place
     where the camera's description is kept on reaching `ready`, and that
     `Imaging::view` builds the view in one place;
   - that 032's `SimulatedSolver::measure` ends in the one line § The simulated solver
     adapter replaces, and where `main.rs` builds that solver;
   - that 032's `Imaging::scale_arcsec_per_px()` answers from the focal length before
     any solve, as the focus rig assumes;
   - that `ImagingPage.tsx` renders 032's block as a region named `Imaging solve`, and
     that no test of 031 or 032 compares a whole imaging view on a console with a
     solver, or finds a field by a name that `Focus frame time` or `Focus region`
     would also match;
   then re-run the validator on `main`. Where a citation of 030's, 031's or 032's trio and
   one here name the same line of the same file with different numbers, the line as it
   reads on `main` decides, and this trio's were last checked at 49bb210.
3. **The owner decisions are settled and the approvals given** (spec § Owner decisions,
   § Owner approvals). Not met: all twenty-four decisions are open. Four of them are
   chosen together, because they answer one question, whether a tile holds five
   measurable stars at a sparse field: 2 (the tiles), 16 (the focus exposure), 21 (the
   measure's floor) and 24 (far out at the start).
4. **Constitution V is read or amended by the owner**, as 032's Prerequisites ask: the
   focus reading is measured by ASTAP through the same port. Met: constitution 1.3.0 (PR 239, merged 2026-11-17 as `cc593c7`).
5. **The trio is probe-built.** Not met. No story has been compiled. Before the flip
   each of the eight is built from this trio in a throwaway worktree on `main` with 030, 031
   and 032 landed, tests first, with both gates; each diff is measured as plan trap 3
   says; and what the probes find is folded back in. US1, US5 and US8 are built first:
   US1 because every later scenario rests on the simulated adapter's two builders; US5
   because it is the largest story; US8 because two of its scenarios, and one of
   US2's, rest on how many seeded stars the simulator puts in a tile at the pole and
   at the simulated console's start (US2-S4, US8-S2 and US8-S3 each check or assume a
   count; § Risks).
6. **The documents this trio cites are on `main`.** Not met: the plan document,
   ADR-0002, the changed `GLOSSARY.md` and the research note are in the drafting
   worktree, uncommitted. Of the five things in `GLOSSARY.md` that spec owner approval
   11 words, that worktree's glossary carries two since the series audit: the new
   entries Focus check and Settled reading, in the approval's words, whose words the
   page and the MCP tools show. It also carries a Lease entry, which is 031's. Three
   are still to come: the entries reworded, Focus reading (sensor pixels until a scale
   is known), Focus region (or a tile fixed by hand) and Focus record (two parts, and
   which judges). The series' author writes them there once the owner has settled the
   decisions behind them (4, 6, 7 and 8); no story of this spec writes the glossary.
   Until then the trio would ship words that the glossary's three entries do not have.
7. **Outside the factory, before the first use at the telescope:** ASTAP installed and
   the console started with its flags and `--focal-length-mm` (032), so that sizes read
   in arcseconds before the first imaging solve. Not needed to flip.
8. **Other drafts that write the same files are ordered.** Met within the series as
   drafted: `034-dark-and-flat-frames` names this spec in its `depends_on_landed`, and
   its Prerequisites read this trio for the last writer of each shared file (US5 for
   `crates/finder/src/imaging/mod.rs` and `routes.rs`; US8 for
   `crates/finder/src/lib.rs` and the `console` binary's `main.rs`; US7 for
   `crates/finder/src/mcp/tools.rs`, `crates/finder/README.md` and
   the `mcp` binary's `main.rs`; US6 for the page, `DESIGN.md`,
   `.impeccable/design.json` and `dist`; US1 for `crates/simulator/src/optics.rs` and
   `crates/finder/src/api.rs`), for where the Focus block sits (after the Imaging
   solve block), for `Imaging::focus_running()`, which a later spec may show and must
   not refuse on (§ Shapes that existing tests pin), for the holder's name `the focus
   aid`, and for the answer that the aid's frames are saved and then removed. The
   review's split of US5 moved the `cli` binary's `main.rs`, `lib.rs` and `main.rs`
   from US5 to US8: 035's Refinement Record names US8 as the last writer of
   `crates/finder/tests/cli/main.rs`, and 037's names 035's US8, which writes it after
   this spec. 034's draft takes 031's lease under its own name, its owner decision 23
   settled by the plan document's amendment 1, and adds no refusal built on
   `focus_running()` (its plan.md § Prerequisites 5). 035's and
   037's drafts take the same lease as this one, 031 US6. Not met for the
   drafts on other branches that 031's Prerequisites name (`026-mount-sky`,
   `021-chart-orientation`, `022-altaz-guidance`): whichever of two such specs is
   flipped second names the other in `depends_on_landed` and is re-anchored after it
   lands.
9. **The plan document agrees with what this trio builds.** Met in the drafting
   worktree since the series audit, whose edits to the shared documents put the four
   corrections below into the plan document there; it reaches `main` with the plan
   document itself (§ Prerequisites 6). The plan document's "Settled decisions" table
   is not changed by any spec, and its row for Focus stands as written. Spec owner
   decisions 6, 7, 8 and 23 are declared as
   readings of that row, not changes to it: the reading is in arcseconds whenever a
   scale is known, and in sensor pixels only while none is; the record is still "the
   smallest reading", the smallest settled reading, kept in two parts with the rule for
   which judges; and `no_record` is the verdict where the row has no record to judge
   against. Only four places of the plan document are corrected, by the series'
   author, and no story writes them:
   - **§ 033**: the verdict as good, slightly soft, clearly out, unmeasurable, or no
     record yet, in place of "the three-way verdict" (owner decision 23); the focus
     aid's imaging frames one after another, of one second unless asked otherwise
     (owner decision 16); and the focus check as up to nine such frames ended by the
     first settled reading (owner decision 12);
   - **the route table**: the 033 row names the focus check, `POST
     /api/imaging/focus` with `check`, and, under `--simulate`, `GET` and `POST
     /api/sim/focus` (owner approval 2);
   - **the storage line**: `focus.json` in the imaging folder (owner approval 4), and
     that the focus aid removes its own imaging frames (owner decision 13);
   - **the module tree**: `focus.rs` with its two children, `focus/aid.rs` and
     `focus/run.rs` (owner approval 3).

   The rest of what this trio adds needs no change there: the plan document names the
   two MCP tools without saying what they do (owner decision 19), and the focus
   region's tile fixed by hand and the tile read for the time being (owner decisions 4
   and 24) are the glossary's rewording (§ Prerequisites 6). Both documents are put
   right so that 035 and 037 read what 033 builds. An owner who settles decision 4, 6,
   7, 8, 23 or 24 the other way drops that reading, and the trio is changed to match
   before the flip.
10. **No operator-staged file is needed.** Met: no story reads a fixture, and no test
    runs ASTAP.
11. **031's trio carries the imaging camera's lease, and defines `RoiEdit`.** Met in
    the trios as drafted: the series audit gave 031's trio the lease as its User Story
    6, with an owner approval for its public names and its refusal (031's owner
    approval 12, FR-028 to FR-033), and defined `RoiEdit` in its US1, in the shape this
    trio had written down and with nothing changed:
    `Imaging::lease(&Arc<Self>, &'static str) -> Result<Lease, Refusal>`, whose 409 is
    `the imaging camera is held by {holder}`;
    `Imaging::holder()`; `Lease::set` and `Lease::take`; in `take` and `set`, the
    lease's check before the check for a frame in flight; `abort`, `ended`, `view` and
    032's `solve` unguarded; the lease released when it is dropped, also when its task
    is aborted; no strong `Arc` in it; a single frame holding none; and `RoiEdit` with
    `x`, `y`, `width` and `height` (§ The camera lease). US5's code calls
    `Imaging::lease`, `Lease::set` and `Lease::take`, and its tests call
    `Imaging::lease` and `Imaging::holder`; they compile against 031 as drafted, and no
    story of 033, 035 or 037 adds to 031's state. What stays is the check in code:
    § Prerequisites 2 reads the lease in 031 as landed. If 031 lands with a lease whose
    `set` is not guarded, a settings change could reach the camera between two of the
    aid's frames, and the first draft's ending for it comes back as a spec change
    before the flip; if its single frame holds the lease, § The camera lease, 6, and
    US5-S2's one case change, and nothing else. If 031's lease can be lent to a step,
    037 may choose its alternative (§ Shapes that existing tests pin, last bullet).

## Plan traps

Lessons from building 012 to 028 on this factory, and the traps this design adds, each
with the check that keeps it from recurring. They bind every story.

1. **Gate evidence or nothing.** The judge credits only gate results and the story's
   diff. *Check:* every Then clause names a committed test that the `test` gate runs.
2. **A named command is read literally.** *Check:* commands are copied from
   § Verification.
3. **One evidence budget.** *Check:* each story keeps its diff under 64 KiB, measured
   with `git diff <base> -- . ':(exclude)crates/finder/web/dist' | wc -c`. A story that
   cannot escalates; it does not drop a scenario.
4. **The sandbox has only the gates' tools.** There is no ASTAP in it. *Check:* every
   `measure` in a test is the simulated solver adapter's or the scripted solver's.
5. **A spec fixed mid-epic needs a relaunch.** *Check:* no builder edits
   `specs/033-focus-reading-and-aid/` or `.specify/`.
6. **An oracle copied from the code proves nothing.** *Check:* expected sizes are
   worked in spec.md from 030's point-spread constants and the rig's scale; the worked
   examples of the arithmetic are written out step by step in § The aid's arithmetic;
   the noise's first numbers come from the published stream; a test parses `focus.json`
   itself, and US8-S1 writes one by hand.
7. **A ratio on a threshold.** 3.30/3.00 is not 1.1 in floating point. *Check:* every
   scenario asks for a verdict at least 0.3% from a threshold (3.29 and 3.31 against
   3.00), and the arithmetic's examples keep 12.5% and 13.6% away from 15%.
8. **A scenario that rides on a random count.** The simulator's stars are a seeded
   Poisson field, 13 to a tile on average, and five are needed. *Check:* every scenario
   that measures a tile through the simulated adapter first measures it itself and
   requires the count (5 in US2-S4, 8 in US8-S2); every other loop scenario uses the
   scripted solver, whose star counts are its own; the focus rig points at the pole, so
   the field does not drift out of a tile during a test.
9. **Paused time and a busy loop.** With time paused tokio jumps to the next timer when
   every task is idle; a loop with no await would never yield. *Check:* every turn of
   the aid's loop awaits an exposure of at least 1 ms, the camera's shortest.
10. **A lock held across an await.** *Check:* the focus state is read and written in
    short sections; `take`, `ended`, `set`, `read` and `find_region` are awaited with
    no lock held. The lease is not such a lock: it is 031's, and the task owns it.
11. **A task that keeps its owner alive.** *Check:* the aid's task holds a `Weak` of
    the `Imaging`, and is aborted where 031's own task is; § Prerequisites 2 checks
    that a held lease does not keep the `Imaging` alive either.
12. **Removing the wrong file, or a file something is still using.** *Check:* the aid
    removes only a path it was given by `Imaging::ended` for a frame it asked for
    itself, only after a later frame of its own is saved, and never a frame 032 is
    solving at that moment; US5-S1 asserts one file left, US5-S7 that a frame under a
    solve stays with its solve sidecar, and no other story removes anything.
13. **A lease that outlives the aid, or an aid that outlives its lease.** A lease left
    held refuses every later imaging frame until the console restarts; a lease dropped
    before the settings are back lets the next taker expose the aid's region. *Check:*
    the task owns the lease, puts the settings back through it and drops it before the
    lifecycle becomes `ended`; US5-S5 asserts `holder()` is `None` after every ending,
    a lost link included, US5-S4 that an imaging frame can be taken after a stop, and
    US8-S1 that the lease is free after a check.
14. **A test that pins what a later story changes.** US3 adds `record` to the file US2
    writes, US5 adds `focus` to the view, and US8 adds `check` to the request and to
    the aid's part of the view, and `done` to the endings. *Check:* US2's tests assert
    the file's `region` and never that `record` is absent; tests compare `focus` key by
    key; no test of US1 to US4 reads the imaging view; and no test of US5 sends
    `check`, asserts that it is refused, or asserts that the aid's part has no `check`
    key.
15. **A whole-view comparison in an earlier spec.** *Check:* § Prerequisites 2 re-reads
    031's and 032's tests for one; `focus` is absent without a solver.
16. **A label that matches by substring.** Playwright's `getByLabel("Exposure")` would
    also match a field named `Focus exposure`. *Check:* the field is `Focus frame
    time`; no new name contains `Exposure`, `Gain`, `Offset`, `Binning`, `Cooler`,
    `Take imaging frame` or `Solve imaging frame`.
17. **A night pixel.** *Check:* the block sets no colour; its bars are `currentColor`;
    US6-S5 asserts no `style` and no other `fill` or `stroke`; 031's full-page night
    check counts the idle block on the simulated console.
18. **`error` as a data key.** Every refusal of this API is `{"error": "…"}`. *Check:*
    the simulate route's key is `focus_error`.
19. **A tool that hangs or fails on `{}`.** *Check:* both tools read their own
    arguments and answer `{}` with a tool result at once (US7-S4).
20. **A constant sized to one telescope, or a wait counted in seconds.** *Check:*
    SC-005; `aid.rs` has no clock at all; the pause, the best and the check are counted
    in imaging frames.
21. **The bundle must be rebuilt.** *Check:* US6 commits `crates/finder/web/dist` after
    `npm run build`.
22. **027's layout.** *Check:* each new finder test is a module file in its binary's
    folder with one alphabetical `mod` line, binding helpers with
    `use crate::support::live as support;` and holding only the setup its tests use.
23. **Scratch files go where cargo says.** *Check:* every file a test makes is inside a
    `crate::support::Scratch`.
24. **A dead field.** A field that is written and never read fails `clippy -D
    warnings`. *Check:* US2 gives `Kept` only the region, US3 adds the record; the
    focus state enters `mod.rs` in US5, with its reader; and the run's `check` enters
    in US8, with step 4i, which reads it.
25. **Words that claim too much.** *Check:* SC-006; no string in `FocusBlock.tsx`, the
    README section or a tool description says focus is good, sharp or correct without
    "of the focus record", "so far" or "before the last change".
26. **A second way to keep others off the camera.** The first draft offered
    `focus_running()` for later specs to refuse on; the plan document's amendment 1
    forbids a lock of a spec's own. *Check:* SC-007; nothing in 031's `take` or `set`
    reads the focus state; within this spec `focus_running()` is read only by its own
    tests and views; and a later spec may show it but not refuse on it (§ Shapes that
    existing tests pin; 034's plan.md § Prerequisites 5 adds no such refusal).
27. **A lease sentence copied into a test.** 031 words the lease's refusal (`the
    imaging camera is held by {holder}`, 031 US6) and may reword it; a test here that
    copied the whole sentence would break on a change of 031's. *Check:* a test of the
    lease asserts the status 409 and that the words contain the holder's name, nothing
    more (spec.md § User Scenarios, "The lease").
28. **A reading judged against itself.** Noting a settled reading before judging it
    makes every new low `good` and the first reading of a new camera `good` against
    nothing. *Check:* step 4g reads the reference before step 4h notes; US5-S1 asserts
    `no_record` for the first settled reading and `good` against it for the next.
29. **A scale that is only nominal.** *Check:* `Kept` is given the measured scale and
    never `Imaging::scale_arcsec_per_px()`; US5-S1, on a rig whose only scale is the
    focal length's, asserts that the file's `record` has no `arcsec_per_px`.
30. **Two starts at once.** Checks without an await between them are not one step on a
    multi-threaded runtime. *Check:* nothing relies on it; the lease keeps two starts
    apart, and no test asserts which 409 a same-instant start gets.
31. **A count seen between two polls.** A scenario that asserts on the view at an exact
    count of readings would be flaky if a count could be skipped. *Check:* a test polls
    every 100 ms of virtual time and an imaging frame takes at least its exposure,
    1000 ms in every scenario that waits for an exact count (US5-S1, US5-S6).

## Risks

| Risk | Mitigation |
|---|---|
| Nothing here has been compiled, and neither have 030, 031 and 032, which it builds on. An interface may not build as written: `impl Imaging` blocks in a grandchild module against state 031 has not fixed; `focus_start(self: &Arc<Self>)` against how 031 hands out its `Imaging`; `SettingsEdit`'s `roi` type; a `Weak` in a task spawned from a method. | Prerequisites 2 and 5: re-anchor after all three land, then probe-build all eight, US1, US5 and US8 first. |
| The simulator's seeded field puts fewer stars in a tile than a scenario needs. The count is a Poisson draw of mean 13 on a fixed seed, and no story may change the seed, the density or the scenario. | US2-S4 and US8-S2 check the count first, so a shortfall shows as that guard (a chance near 1 in 200 for five stars, 1 in 25 for eight). US8-S3 assumes five in the middle tile at the simulated console's start. The US1 and US8 probes read all three counts; if one falls short the tile, the pointing or the start time is changed before the flip (§ Plan traps 8). |
| ASTAP's star measure on a tile of a raw colour mosaic is not what the design assumes: too few stars at its signal-to-noise floor, a floor under its own 2 × 2 averaged pixels, or sizes that do not fall through focus. | Field checks, named in spec.md. The floor is 032's owner decision 24 and this spec's 21; the tile is a third of the sensor and an owner decision; the port keeps another adapter possible. Not answerable in the factory. |
| With the defaults a tile holds fewer than five measurable stars at a sparse field, and the aid ends `no_region` or reads too few stars. The research note's own figures predict it (§ 4. Star counts and limiting magnitude at this field): a field holds 30 to 36 stars to G 14 at the galactic poles and 78 to 97 at galactic latitude 30°, so a ninth of it holds 3 to 4 and 9 to 11; and the limit it estimates, G 15.6 to 15.8 in 2 s at a signal-to-noise ratio of 10, becomes about G 14 for a 1 s frame at the measure's floor of 30 (half the exposure costs 0.4 to 0.75 of a magnitude and three times the ratio about 1.2: this plan's inference, on an estimate the note itself puts at a magnitude either way and only near the field's centre). Off-axis coma spreads the stars of the outer tiles and lowers them further. So at the galactic poles the defaults are predicted to fail; at latitude 30° and at the showcase target, whose field holds about 14 such stars a tile away from the Trapezium, to work. | Put to the owner with this arithmetic, as one question in three decisions (spec owner decisions 2, 16 and 21): the default focus exposure (1000 ms recommended; 2000 or 3000 ms reach about G 14.5 to G 15, where a polar tile holds 5 to 7), the measure's floor (30 recommended; 10 reaches about G 15 at 1 s), or both as they are with the cure named. The cure is named whichever is chosen: the page says `Try a longer Focus frame time, or a field with more stars.` on a frame with too few stars and after a `no_region` ending (spec FR-033, US6-S4), and the README's `## Focus` says why (spec FR-030). 037's run can pass its check a longer `exposure_ms`. The field check "how many measurable stars a tile of a one-second exposure holds" at three galactic latitudes tests the arithmetic; nothing in the factory can. |
| Far out of focus at first light the aid has no tile to choose, or chooses one by noise. | Spec owner decision 24: it reads the far-out tile with the smallest size for the time being, keeps nothing, and searches again at the first settled reading (spec US5-S6). The region kept then may still have been chosen with a thin margin; the README says to find it again once focus is close, and owner decision 3 names the alternative of doing that without being asked. ASTAP's sizes above 20 pixels are rough, and a far-out frame with few stars may read `too_few_stars` instead; then the start ends `no_region` and the observer fixes a tile by hand. The field check "starting the aid a millimetre or more out of focus" settles which. |
| The 15%, 5%, 10% and 20% are set against one reported figure for frame-to-frame scatter, about 10% of a single star's width, and a simulator with 2%. Too tight, and every frame is a change and nothing settles; too loose, and a real turn is missed. | They are five constants in `aid.rs` and one owner decision (11), proved as arithmetic on worked examples. The field check "the scatter of the reading at a fixed focus" settles them; until then the observer still has the number, the history and the picture. |
| The smallest of many settled readings is still a little low, so readings at the same focus sit a few percent above the record. | A settled reading is a median of three, and the bands are 10% and 30%. The field check above measures it; the alternative of owner decision 7 (only a pass seen to rise on both sides sets the record) removes it. |
| At the showcase target the bright core of the nebula fills the middle of the sensor, where the focus region usually is, and the solver measures poorly there. | The page and the README say to focus on a field of stars. Where the imaging run takes its focus check, at the target or beside it, is 037's and is named for it in § Shapes that existing tests pin; the field check "a focus check with the Orion Nebula's core in the focus region" informs it. |
| One reading takes much longer than a second at the telescope: the frame's forwarding, its save, the prepared copy and the solver's start. | `cadence_ms` reports it, and the field check reads it. The region is a ninth of the sensor; a smaller focus exposure and 031's forwarding settings are the levers; nothing in the arithmetic assumes a second. |
| A story passes 64 KiB. The first draft's US5 would have: the loop, the view, two routes, the README section and seven scenarios that build some fifteen rigs, by the review's count over the budget. | Split before any probe, as the review asked. US5 keeps `run.rs` without the check, the focus state, the two routes, `lib.rs`, the two API rows and seven scenarios in one test module: an estimate of 45 to 55 KiB (about 17 of source, 8 of the rig and the scripted solver, 25 to 30 of tests). US8 takes the check, `focus_ended`, `main.rs`, the README's `## Focus` and three scenarios in a module of its own and the `cli` module: an estimate of 22 to 28 KiB. Both are estimates until the probes measure them (§ Prerequisites 5). If US5 still passes the budget, its next cut is written here too: US5-S6 with step 4j (FR-037, the tile read for the time being) moves to a story between US5 and US8. US1 can shed the simulate route (FR-004, US1-S5) the same way. |
| The console exits, or the link drops, while the aid runs, and the camera is left on a region with the aid's exposure. | The Focus block offers `Use the whole sensor` whenever no aid runs and the region is not the whole sensor (US6-S4); exposure and binning are on 031's Settings form; the README says so. Nothing is persisted to undo it, by design: a stale "put back" after a restart could overwrite the observer's own change. |
| 031 lands with a lease unlike its US6 as drafted: other names, a refusal that does not name the holder, a `set` it does not guard, a lease that keeps the `Imaging` alive. | Prerequisite 11 is met in the trios as drafted, and Prerequisite 2 checks the lease in 031's code as landed. Everything this trio takes from the lease is cited in one section, six numbered rules and `RoiEdit`, and the tests of it assert only a 409 that names the holder. The re-anchoring corrects that section, the task's "set" and "take", US5-S2's one case, US5-S4, US5-S5 and the test setup; nothing else in the trio names the lease. |
| An imaging run (037) holds the camera and cannot ask for a focus check, which takes the lease itself. | Settled on 037's side: its spec owner decision 15 has the run give the lease up just before the check and take it again after, and its plan.md calls `focus_start` with `check` and `focus_ended()` and reads this trio's five verdict words (§ Shapes that existing tests pin, last bullet). In the instant of the hand-over another taker can get in; 037 names that risk and its answer. Only 037's alternative, a lease that can be lent, would change a signature here. |
| Another taker of frames (the observer, 034's sets, 035's stack) asks for the camera while the aid runs. | The lease: 031 refuses the second taker with `the imaging camera is held by the focus aid`, and the aid runs on (US5-S4). The observer stops the aid first; the page's alert shows 031's words. |
| `focus` appearing in the imaging view breaks a comparison in 031's or 032's tests, or the new block breaks a query in their page tests. | Prerequisite 2 re-reads them; `focus` is absent without a solver, and the idle block has no `status` or `alert`. |
| A found region goes stale after a re-collimation or a change of telescope, and readings there are larger than they could be. | `Find the sharpest tile again` is one choice on the page and one key in the request; the README says when to use it. A reading that is clearly out against the record is the prompt. |
| The owner moves one camera between two telescopes, and each move costs the earlier nights' record. | Spec owner decision 10 names the cost and its alternative, a record and a region for each scale in `focus.json`, to be chosen before the flip because the file's format is pinned once written. |
| The observer leaves a forwarding limit set, and one reading takes many seconds. | The aid does not touch 031's `forward_kib_per_s`; `cadence_ms` shows the time, and the README says to clear the limit while focusing. |
| 034 treats a frame with the purpose label `focus` as a sub when it looks for the latest sub to match calibration against. | 034's draft passes over the purpose label `focus`, as it does `dark` and `flat`, when it looks for the latest imaging frame to match calibration against (its owner decision 22, FR-017); § Shapes that existing tests pin names the label. |
| A second console, or two tabs, start the aid at once. | The second start is refused with `the focus aid is already running`, or, when the two arrive in the same instant on two threads, by the lease, with words that name `the focus aid`: a 409 either way, and one aid (§ The aid on the console; § Plan traps 30). |
| The first settled reading of a night is soft, and tonight's record hides a sharper one of earlier nights. | It does not: until the aid has passed through focus tonight the smaller part judges (spec owner decision 8, US3-S2, US8-S1). On a camera's first night there is no earlier part, and a check after a short, unfinished session is judged against whatever that session saw; the page names it as tonight's record, and owner decision 8 lists the stricter alternative, no record until a pass has been seen. |
| A record made at the nominal scale of `--focal-length-mm` is thrown away by the first imaging solve. | Only a measured scale is kept with the record or compared (spec owner decision 10, US3-S4, US5-S1). The cost is the other way round: a record made before any solve and carried to another telescope before either was solved on is not recognised as foreign; the README says to remove `focus.json` then. |
| Somebody asks for an imaging solve while the aid runs, and the aid removes the frame under it. | 032 refuses a solve while a frame is arriving, so it can start only in the gap while the aid measures; a frame under a running solve is left in place with its sidecar (spec FR-024, US5-S7). |

## Refinement Record

Drafted on 2026-11-15 by the grilling session, first against `main` at dbc2b5b and
then, in a second pass the same day, against `main` at 49bb210, from
`docs/plans/2026-11-15-specs-030-037-imaging.md`, the reviewed trios of 030, 031 and 032 in
the same worktree, 034's reviewed trio for what it asks of this spec, the research
note's sections on focus and on ASTAP with the verification of the web research behind
them, and a read of the simulator, the simulate routes, the MCP tools and the page.
**No probe build has been run.** No story of this trio has been compiled, linted or
tested, no interface above has been built, and no diff has been measured: the session
wrote documents only.

The second pass did four things. It re-read every `path:line` at 49bb210 and corrected
three that had moved. It took `SolveError`, `SettingsEdit`, `FrameView` and the routes'
status codes from 031's and 032's plan.md as reviewed. It brought the trio in line with
the plan document's "Mount-neutral by design" and "Amendments after 030, 031, 032 and 034
were drafted and reviewed": the aid now holds 031's lease, and the two endings the
first pass had for a camera shared without one are gone. And it worked every number in
a scenario again, outside the repository: the star sizes from 030's point-spread
constants, the noise from the published SplitMix64 stream, the sharpest-point sizes
for all nine tiles, and the arithmetic's five worked examples by a short script that
follows § The aid's arithmetic rule for rule, whose output is the table there and
agrees with spec.md in every figure. None was produced by code of this project. The
validator was run at 49bb210 after the pass.

What the pass could not do: 031's plan.md then gave the lease no name, so § The camera
lease was written as an assumed shape (§ Prerequisites 11; the fourth pass below).

A third pass the same day applied a review of the trio by a second agent, which had
re-derived every figure and found the validator clean, and raised eighteen findings
(spec.md § Clarifications, the fourth session, lists what was done about each). Still
no probe build was run. What changed in this record: `Kept::note` takes whether the aid
has passed through focus, and the reference is the smaller part of the record until it
has (US3); `find_region` names a far-out tile when no tile can be measured, and the aid
reads it for the time being (US2, US5); a settled reading is judged before it is noted,
the record takes only a measured scale, and a frame under an imaging solve is not
removed (US5); and the first draft's US5 is two stories, US5 and US8, with US6 and US7
both after US8 and no edge between them. The new scenarios' figures (the two parts of
the record through US3-S2, the scale cases of US3-S4, the run of US5-S6 through the
aid's arithmetic, and the tile sizes at a focus error of 2.0 behind US8-S3) were worked
by a short script outside the repository, as before.

What the third pass could not do is the review's one blocker: give 031's trio the
lease. That is an edit to 031's files, which a revision of this trio may not make, so
§ The camera lease was written as the amendment 031 needed.

A fourth pass the same day applied the series audit of the seven trios the series then had,
which raised three findings for this one (spec.md § Clarifications, the fifth session).
Still no probe build was run. The audit gave 031's trio the lease as its User Story 6,
with `RoiEdit` defined, in the shape this trio had written down, so § The camera lease
is now a citation of 031 US6 with nothing changed in shape (the four functions, the
six rules, and the refusal words `the imaging camera is held by {holder}`), its sixth
rule is 031's and no longer an assumption, and § Prerequisites 11 is met in the trios
as drafted. § Prerequisites 9 now says that the plan document's "Settled decisions"
row for Focus stands, that spec owner decisions 6, 7, 8 and 23 are readings of it, and
that only the plan document's § 033, route table, storage line and module tree are
corrected. Two pointers were corrected in spec.md: `imaging_optics` and
`set_imaging_optics` are on `optics::Simulator` in `crates/simulator/src/optics.rs`,
and the Out of scope line no longer says that 036 bounds the focus exposure, since 036
bounds only a stack's exposure. Every `path:line` in spec.md and plan.md was read again
at 49bb210, the worktree's HEAD, and none had moved. No scenario, FR, interface,
write or test changed, so the record below is as the third pass left it but for one
line of US5's `preserved_behavior`, which names 031 US6's lease as a thing US5 must
leave as it is; the validator was re-run after the pass.

What a first refinement round must do is § Prerequisites 2 and 5, and its findings
replace these paragraphs. Until then the record below is the draft's own account: its
tests are the scenarios, its writes are the files § Project Structure names, and its
shared writes name the merge edge that orders two stories of this spec on one file, or
the story of 030, 031 or 032 that last writes the file. An existing test is listed only when
its file is in the drafting worktree; 030's, 031's and 032's tests are named in § Shapes
that existing tests pin instead. The record's `open_questions` are empty because the
validator refuses a record that holds any; what is not established is in § Risks,
§ Prerequisites and spec.md's field checks. Nothing in this record is an owner
approval: those are in spec.md, and flipping to `ready` gives them.

```yaml
version: 1
reviewed_revision: 49bb210fe4860dd98ba24727074e7bd3cc0d7124
stories:
  US1:
    tests:
      - path: crates/finder/tests/console/imaging_focus_reading.rs
        status: new
        disposition: add
        assertion: "a focus reading of the simulated solver's star measure is 2.825784 sensor pixels at best focus and 5.492337 at a focus error of 2.0 either way, at binning 1 and 2, with arcseconds from the scale given and none without one, and the simulator's focus error reads back"
        decision: "spec.md:US1-S1"
      - path: crates/finder/tests/console/imaging_focus_reading.rs
        status: new
        disposition: add
        assertion: "a scripted star measure becomes sensor pixels by the frame's binning and arcseconds by the scale, with the measured region in sensor pixels, and the solver is called with exactly the frame and region given"
        decision: "spec.md:US1-S2"
      - path: crates/finder/tests/console/imaging_focus_reading.rs
        status: new
        disposition: add
        assertion: "four stars or the solver's TooFewStars is unmeasurable, five is a reading, 20 frame pixels is a reading and 20.5 is far out with its size in sensor pixels, and any other solver error passes through unchanged"
        decision: "spec.md:US1-S3"
      - path: crates/finder/tests/console/imaging_focus_reading.rs
        status: new
        disposition: add
        assertion: "measure noise with the seed 0 gives the published stream's first three factors and stays within 2 percent, the sharpest point gives the worked sizes for a frame, a binned frame and a region, and without either builder the adapter answers 032's numbers"
        decision: "spec.md:US1-S4"
      - path: crates/finder/tests/console/imaging_focus_reading.rs
        status: new
        disposition: add
        assertion: "GET and POST /api/sim/focus read and set the simulator's focus error, refuse a value over 100, a missing key, an unknown key and a foreign origin, and answer 404 without a simulation"
        decision: "spec.md:US1-S5"
      - path: crates/finder/tests/motion/simulate.rs
        status: existing
        disposition: preserve
        assertion: "the existing simulate routes keep their bodies and an unknown /api/sim path answers 404"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/console/live/whole_loop.rs
        status: existing
        disposition: preserve
        assertion: "a simulated console built in process solves and guides as before"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and each binary's main.rs holds only its support include and mod lines"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "the simulated solver adapter answers 032's numbers unless a new builder is called (spec.md FR-005)"
      - "ImagingOptics gains no field and the simulator's other controls are untouched"
      - "no Cargo.toml, Cargo.lock or deny.toml changes and no unsafe is written (spec.md SC-003)"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console imaging_focus_reading::"
        - "cargo nextest run -p simulator --locked"
        - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - crates/finder/README.md
      - crates/finder/src/api.rs
      - crates/finder/src/imaging/focus.rs
      - crates/finder/src/imaging/mod.rs
      - crates/finder/src/imaging/simulated.rs
      - crates/finder/tests/console/imaging_focus_reading.rs
      - crates/finder/tests/console/main.rs
      - crates/simulator/src/optics.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/README.md
        other_spec: 032-imaging-solve-and-aim-point
        other_story: US7
        disposition: merge_order
        reason: "033 depends_on_landed 032, whose US7 last changed the README"
      - path: crates/finder/README.md
        other_spec: 033-focus-reading-and-aid
        other_story: US5
        disposition: merge_order
        reason: "US5 depends_on_merged US1, and adds two API rows"
      - path: crates/finder/src/api.rs
        other_spec: 032-imaging-solve-and-aim-point
        other_story: US3
        disposition: merge_order
        reason: "033 depends_on_landed 032, whose US3 last changed api.rs"
      - path: crates/finder/src/imaging/focus.rs
        other_spec: 033-focus-reading-and-aid
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and adds the tiles, find_region and Kept"
      - path: crates/finder/src/imaging/mod.rs
        other_spec: 032-imaging-solve-and-aim-point
        other_story: US4
        disposition: merge_order
        reason: "033 depends_on_landed 032, whose US4 last changed imaging/mod.rs"
      - path: crates/finder/src/imaging/mod.rs
        other_spec: 033-focus-reading-and-aid
        other_story: US5
        disposition: merge_order
        reason: "US5 depends_on_merged US1, and adds the focus state and the view's focus"
      - path: crates/finder/src/imaging/simulated.rs
        other_spec: 032-imaging-solve-and-aim-point
        other_story: US2
        disposition: merge_order
        reason: "033 depends_on_landed 032, whose US2 created simulated.rs"
      - path: crates/finder/tests/console/main.rs
        other_spec: 032-imaging-solve-and-aim-point
        other_story: US5
        disposition: merge_order
        reason: "033 depends_on_landed 032, whose US5 last changed console/main.rs"
      - path: crates/finder/tests/console/main.rs
        other_spec: 033-focus-reading-and-aid
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and adds mod imaging_focus_region"
      - path: crates/simulator/src/optics.rs
        other_spec: 030-imaging-camera-link
        other_story: US2
        disposition: merge_order
        reason: "033 depends_on_landed 030, whose US2 last changed the simulator's optics.rs"
    open_questions: []
  US2:
    tests:
      - path: crates/finder/tests/console/imaging_focus_region.rs
        status: new
        disposition: add
        assertion: "a tile is a third of the sensor each way rounded down to a multiple of twice the binning, for a small and a large sensor and at binning 2, and a tile index of 3 is refused"
        decision: "spec.md:US2-S1"
      - path: crates/finder/tests/console/imaging_focus_region.rs
        status: new
        disposition: add
        assertion: "find_region measures nine tiles in row-major order in frame pixels and names the smallest reading among tiles with five stars, passing over a smaller one with three"
        decision: "spec.md:US2-S2"
      - path: crates/finder/tests/console/imaging_focus_region.rs
        status: new
        disposition: add
        assertion: "no measurable tile gives no best, equal tiles give the first, a TooFewStars tile is left out, far-out tiles are not candidates for best and the smallest of them is named as far_out only when there is no best, and another solver error ends the search at once"
        decision: "spec.md:US2-S3"
      - path: crates/finder/tests/console/imaging_focus_region.rs
        status: new
        disposition: add
        assertion: "with the simulator's stars sharpest at 0.8, 0.2 the search keeps the top right tile at 2.864762 sensor pixels, and at 0.5, 0.5 the middle tile at 2.825784, each after the test has seen five stars there"
        decision: "spec.md:US2-S4"
      - path: crates/finder/tests/console/imaging_focus_region.rs
        status: new
        disposition: add
        assertion: "Kept writes focus.json with its format, camera and region, reads it back for the same camera and not for another, keeps a fixed tile without a reading, and survives an unreadable file"
        decision: "spec.md:US2-S5"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and each binary's main.rs holds only its support include and mod lines"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "focus::read and its two constants are unchanged (spec.md FR-001, FR-002)"
      - "nothing is written outside the path Kept was given"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console imaging_focus_region::"
        - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - crates/finder/src/imaging/focus.rs
      - crates/finder/tests/console/imaging_focus_region.rs
      - crates/finder/tests/console/main.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/imaging/focus.rs
        other_spec: 033-focus-reading-and-aid
        other_story: US3
        disposition: merge_order
        reason: "US3 depends_on_merged US2, and adds the record and judge"
      - path: crates/finder/tests/console/main.rs
        other_spec: 033-focus-reading-and-aid
        other_story: US3
        disposition: merge_order
        reason: "US3 depends_on_merged US2, and adds mod imaging_focus_record"
    open_questions: []
  US3:
    tests:
      - path: crates/finder/tests/console/imaging_focus_record.rs
        status: new
        disposition: add
        assertion: "judge answers good at 1.097 of the reference, slightly_soft at 1.103 and 1.297, clearly_out at 1.303, no_record without one, unmeasurable for too few stars and clearly_out for far out whatever the reference"
        decision: "spec.md:US3-S1"
      - path: crates/finder/tests/console/imaging_focus_record.rs
        status: new
        disposition: add
        assertion: "note keeps tonight's smallest settled reading, a night ends 12 hours after its first, a new night moves the old record to earlier when smaller, the reference is the smaller of the two parts until a note marks tonight's as passed and tonight's alone from then on, and the mark stays for the night"
        decision: "spec.md:US3-S2"
      - path: crates/finder/tests/console/imaging_focus_record.rs
        status: new
        disposition: add
        assertion: "the record is written to focus.json beside the region with px, at and since, with passed only once it is true, read back for the same camera, empty for another, and neither write loses the other's part"
        decision: "spec.md:US3-S3"
      - path: crates/finder/tests/console/imaging_focus_record.rs
        status: new
        disposition: add
        assertion: "a record keeps the measured scale it was made at, still judges at a scale 4.1 percent away or with no scale, judges nothing at half the scale, and is replaced by the next note there; a record made with no scale is never set aside and takes the scale of the next note that changes it"
        decision: "spec.md:US3-S4"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and each binary's main.rs holds only its support include and mod lines"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "Kept's region, its file's region keys and US2's tests are unchanged (spec.md FR-008, FR-014)"
      - "reading the record never writes the file"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console imaging_focus_record::"
        - "cargo nextest run -p finder --locked --test console imaging_focus_region::"
        - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - crates/finder/src/imaging/focus.rs
      - crates/finder/tests/console/imaging_focus_record.rs
      - crates/finder/tests/console/main.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/imaging/focus.rs
        other_spec: 033-focus-reading-and-aid
        other_story: US4
        disposition: merge_order
        reason: "US4 depends_on_merged US3, and adds pub mod aid"
      - path: crates/finder/tests/console/main.rs
        other_spec: 033-focus-reading-and-aid
        other_story: US4
        disposition: merge_order
        reason: "US4 depends_on_merged US3, and adds mod imaging_focus_arithmetic"
    open_questions: []
  US4:
    tests:
      - path: crates/finder/tests/console/imaging_focus_arithmetic.rs
        status: new
        disposition: add
        assertion: "39 samples through focus and back give the worked medians, the two frames set aside at each change, better and worse trends, the best 2.8, and found only once the reading is back near the best after rising on both sides"
        decision: "spec.md:US4-S1"
      - path: crates/finder/tests/console/imaging_focus_arithmetic.rs
        status: new
        disposition: add
        assertion: "a slow turn with no change sets nothing aside and moves the trend once the settled reading is 5 percent from its level, to better, same and worse in turn"
        decision: "spec.md:US4-S2"
      - path: crates/finder/tests/console/imaging_focus_arithmetic.rs
        status: new
        disposition: add
        assertion: "a frame with too few stars and a far-out frame empty the readings held, set nothing aside, leave the best alone and appear in the history as nothing and as their size"
        decision: "spec.md:US4-S3"
      - path: crates/finder/tests/console/imaging_focus_arithmetic.rs
        status: new
        disposition: add
        assertion: "the best is the smallest of the last kept settled readings and forgets an older one, and the history holds the last 120 samples"
        decision: "spec.md:US4-S4"
      - path: crates/finder/tests/console/imaging_focus_arithmetic.rs
        status: new
        disposition: add
        assertion: "found is false for a reading that only fell and for one that rose and did not return, and true for far, near, far, near ending on a slightly lower best"
        decision: "spec.md:US4-S5"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and each binary's main.rs holds only its support include and mod lines"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "the reading, the region, the record and the verdict of US1 to US3 are unchanged"
      - "aid.rs reads no clock, camera or file"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console imaging_focus_arithmetic::"
        - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - crates/finder/src/imaging/focus.rs
      - crates/finder/src/imaging/focus/aid.rs
      - crates/finder/tests/console/imaging_focus_arithmetic.rs
      - crates/finder/tests/console/main.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/imaging/focus.rs
        other_spec: 033-focus-reading-and-aid
        other_story: US5
        disposition: merge_order
        reason: "US5 depends_on_merged US4, and adds pub mod run"
      - path: crates/finder/tests/console/main.rs
        other_spec: 033-focus-reading-and-aid
        other_story: US5
        disposition: merge_order
        reason: "US5 depends_on_merged US4, and adds mod imaging_focus_aid"
    open_questions: []
  US5:
    tests:
      - path: crates/finder/tests/console/imaging_focus_aid.rs
        status: new
        disposition: add
        assertion: "a start finds the region from nine tiles of a whole-sensor frame, exposes that tile at binning 1 and 1000 ms, shows the median of three in arcseconds with best and history, judges the first settled reading no_record before noting it and the next ones good against it, removes all but its last frame's file, writes no scale worked from the focal length into the record, and a stop puts the exposure, binning and region back"
        decision: "spec.md:US5-S1"
      - path: crates/finder/tests/console/imaging_focus_aid.rs
        status: new
        disposition: add
        assertion: "the focus object is absent from the imaging view and the session without a solver, exactly unit px without a scale, the same in the session as in the imaging view, and a start is refused without imaging, without a solver, off the link, during an exposure nobody holds the lease for, while running, for a bad body and for a foreign origin, each with its words"
        decision: "spec.md:US5-S2"
      - path: crates/finder/tests/console/imaging_focus_aid.rs
        status: new
        disposition: add
        assertion: "a tile fixed by hand needs no whole-sensor frame and is kept for the next start, and find takes nine tile measures again and keeps the first of tied tiles as found"
        decision: "spec.md:US5-S3"
      - path: crates/finder/tests/console/imaging_focus_aid.rs
        status: new
        disposition: add
        assertion: "while the aid runs it holds the camera's lease as the focus aid, so a frame, a settings change and a second lease are each refused with a 409 naming it and the aid runs on; a start while a test holds the lease is refused naming the test and changes nothing, also while the test's own frame is exposing; and the lease is free after a stop"
        decision: "spec.md:US5-S4"
      - path: crates/finder/tests/console/imaging_focus_aid.rs
        status: new
        disposition: add
        assertion: "the aid ends failed on a solver error, stopped on a delete or an abort within 5 s of a 10 s exposure, no_region with its words when no tile has five stars and none is far out, and failed with a restore_error when the link is lost, with the settings back and the lease free after each"
        decision: "spec.md:US5-S5"
      - path: crates/finder/tests/console/imaging_focus_aid.rs
        status: new
        disposition: add
        assertion: "with every tile far out a start reads the far-out tile with the smallest size as a provisional region, shows its rough size, keeps nothing, searches again at the first settled reading and keeps the tile that search finds as found, with the solver's calls in that order"
        decision: "spec.md:US5-S6"
      - path: crates/finder/tests/console/imaging_focus_aid.rs
        status: new
        disposition: add
        assertion: "a frame an imaging solve is running on when its turn to be removed comes stays in place with the solve sidecar that solve writes, and after the stop the frames folder holds that pair, the last frame and no other fits, solve sidecar or part file"
        decision: "spec.md:US5-S7"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and each binary's main.rs holds only its support include and mod lines, with imaging_focus_aid added to console's"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "031's take, set and abort and their refusals are unchanged, the imaging settings gain no key, and no lock on the imaging camera is added (spec.md SC-007)"
      - "031 US6's lease, its six rules and its refusal words are unchanged; the aid only takes it as the focus aid and drops it (plan.md The camera lease)"
      - "GET /api/imaging and the session's imaging object carry no focus without a solver or outside ready"
      - "no file is removed but an imaging frame the aid itself took, and its solve sidecar, and never one an imaging solve is running on (spec.md FR-024)"
      - "no file under crates/finder/src/imaging/focus imports or calls the mount, an axis, an altitude or an azimuth (spec.md SC-005)"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console imaging_focus_aid::"
        - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs, and the field checks of spec.md with the real camera and ASTAP"
    writes:
      - crates/finder/README.md
      - crates/finder/src/imaging/focus.rs
      - crates/finder/src/imaging/focus/run.rs
      - crates/finder/src/imaging/mod.rs
      - crates/finder/src/imaging/routes.rs
      - crates/finder/src/lib.rs
      - crates/finder/tests/console/imaging_focus_aid.rs
      - crates/finder/tests/console/main.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/README.md
        other_spec: 033-focus-reading-and-aid
        other_story: US8
        disposition: merge_order
        reason: "US8 depends_on_merged US5, and adds the Focus section and check to the route's row"
      - path: crates/finder/src/imaging/focus/run.rs
        other_spec: 033-focus-reading-and-aid
        other_story: US8
        disposition: merge_order
        reason: "US8 depends_on_merged US5, and adds check, CHECK_FRAMES, step 4i and focus_ended"
      - path: crates/finder/src/imaging/routes.rs
        other_spec: 032-imaging-solve-and-aim-point
        other_story: US2
        disposition: merge_order
        reason: "033 depends_on_landed 032, whose US2 last changed routes.rs"
      - path: crates/finder/src/lib.rs
        other_spec: 032-imaging-solve-and-aim-point
        other_story: US2
        disposition: merge_order
        reason: "033 depends_on_landed 032, whose US2 last changed lib.rs"
      - path: crates/finder/src/lib.rs
        other_spec: 033-focus-reading-and-aid
        other_story: US8
        disposition: merge_order
        reason: "US8 depends_on_merged US5, and adds check to the route's line"
      - path: crates/finder/tests/console/main.rs
        other_spec: 033-focus-reading-and-aid
        other_story: US8
        disposition: merge_order
        reason: "US8 depends_on_merged US5, and adds mod imaging_focus_check"
    open_questions: []
  US6:
    tests:
      - path: crates/finder/web/src/components/FocusBlock.test.tsx
        status: new
        disposition: add
        assertion: "the Imaging page shows a Focus region after Imaging solve with the standing paragraph, the Focus region select's eleven options, the Focus frame time field and two start buttons, no status or alert, and nothing without a focus object"
        decision: "spec.md:US6-S1"
      - path: crates/finder/web/src/components/FocusBlock.test.tsx
        status: new
        disposition: add
        assertion: "Start focus aid and Check focus post the exposure with find, a tile or check as chosen, a refusal is an alert, and a running aid offers only Stop focus aid, which sends DELETE"
        decision: "spec.md:US6-S2"
      - path: crates/finder/web/src/components/FocusBlock.test.tsx
        status: new
        disposition: add
        assertion: "a running aid shows the reading to two decimals in the view's unit, the status, best, trend, the turning or found line, each verdict's words naming the record, the record and the region, a provisional tile named as for now, and the pixel note without a scale"
        decision: "spec.md:US6-S3"
      - path: crates/finder/web/src/components/FocusBlock.test.tsx
        status: new
        disposition: add
        assertion: "starting, finding, settling, too few stars and far out each have their words, each ending its status or alert, too few stars in a frame and a no_region ending add the line about a longer Focus frame time, a failed put-back has its own alert, and Use the whole sensor appears only with a region set and no aid running"
        decision: "spec.md:US6-S4"
      - path: crates/finder/web/src/components/FocusBlock.test.tsx
        status: new
        disposition: add
        assertion: "the history draws one currentColor bar a measured frame, 36 units tall at the largest, leaves a gap for an unmeasurable one, shows the last 60, and the block sets no style and no other fill or stroke"
        decision: "spec.md:US6-S5"
    preserved_behavior:
      - "the page adds no npm package (plan.md Technical Context, Primary Dependencies)"
      - "red-only night mode: the block sets no colour, and 031's full-page night check counts it idle on the simulated console"
      - "031's and 032's blocks of the Imaging page, their names and their requests are unchanged"
    checks:
      development:
        - "npm test"
        - "npm run typecheck"
        - "npm run build"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs: finder-page, and a night check of the running aid at the telescope"
    writes:
      - .impeccable/design.json
      - DESIGN.md
      - crates/finder/web/dist
      - crates/finder/web/src/api.ts
      - crates/finder/web/src/components/FocusBlock.test.tsx
      - crates/finder/web/src/components/FocusBlock.tsx
      - crates/finder/web/src/components/ImagingPage.tsx
    no_write_reason: ""
    shared_writes:
      - path: .impeccable/design.json
        other_spec: 032-imaging-solve-and-aim-point
        other_story: US6
        disposition: merge_order
        reason: "033 depends_on_landed 032, whose US6 last changed design.json"
      - path: DESIGN.md
        other_spec: 032-imaging-solve-and-aim-point
        other_story: US6
        disposition: merge_order
        reason: "033 depends_on_landed 032, whose US6 last changed DESIGN.md"
      - path: crates/finder/web/dist
        other_spec: 032-imaging-solve-and-aim-point
        other_story: US6
        disposition: merge_order
        reason: "033 depends_on_landed 032, whose US6 last rebuilt the bundle"
      - path: crates/finder/web/src/api.ts
        other_spec: 032-imaging-solve-and-aim-point
        other_story: US6
        disposition: merge_order
        reason: "033 depends_on_landed 032, whose US6 last changed api.ts"
      - path: crates/finder/web/src/components/ImagingPage.tsx
        other_spec: 032-imaging-solve-and-aim-point
        other_story: US6
        disposition: merge_order
        reason: "033 depends_on_landed 032, whose US6 last changed ImagingPage.tsx"
    open_questions: []
  US7:
    tests:
      - path: crates/finder/tests/mcp/imaging_focus.rs
        status: new
        disposition: add
        assertion: "get_focus_reading and set_focus_aid are listed with object output schemas, only the first read-only, each with its description, and the second with exactly five input keys of which on is required"
        decision: "spec.md:US7-S1"
      - path: crates/finder/tests/mcp/imaging_focus.rs
        status: new
        disposition: add
        assertion: "set_focus_aid with check answers only after three one-second exposures with the check's verdict, and get_focus_reading then answers the route's own focus object"
        decision: "spec.md:US7-S2"
      - path: crates/finder/tests/mcp/imaging_focus.rs
        status: new
        disposition: add
        assertion: "set_focus_aid starts at once and stops, refuses a second start, a missing on, other keys with on false and find with a tile, and get_focus_reading refuses arguments, each with its words"
        decision: "spec.md:US7-S3"
      - path: crates/finder/tests/mcp/imaging_focus.rs
        status: new
        disposition: add
        assertion: "without imaging or a solver both tools answer at once with the route's words, and no result holds the pairing token"
        decision: "spec.md:US7-S4"
      - path: crates/finder/tests/mcp/motion.rs
        status: existing
        disposition: preserve
        assertion: "every listed tool answers an empty object without the pairing token"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and each binary's main.rs holds only its support include and mod lines, with imaging_focus added to mcp's"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "the existing MCP tools, their schemas and their refusals are unchanged"
      - "neither new tool moves the mount, is rate limited or writes a call-log line"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test mcp imaging_focus::"
        - "cargo nextest run -p finder --locked --test mcp"
        - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs, and one focus check asked for by an MCP client at the telescope"
    writes:
      - crates/finder/README.md
      - crates/finder/src/mcp/tools.rs
      - crates/finder/tests/mcp/imaging_focus.rs
      - crates/finder/tests/mcp/main.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/mcp/tools.rs
        other_spec: 032-imaging-solve-and-aim-point
        other_story: US7
        disposition: merge_order
        reason: "033 depends_on_landed 032, whose US7 last changed tools.rs"
      - path: crates/finder/tests/mcp/main.rs
        other_spec: 032-imaging-solve-and-aim-point
        other_story: US7
        disposition: merge_order
        reason: "033 depends_on_landed 032, whose US7 last changed mcp/main.rs"
    open_questions: []
  US8:
    tests:
      - path: crates/finder/tests/console/imaging_focus_check.rs
        status: new
        disposition: add
        assertion: "a focus check on a hand-written focus.json takes three frames of the kept tile, ends done with good at 1.067 of tonight's record, slightly_soft at 1.143, clearly_out at 1.333, no_record without one, against earlier nights after 13 hours, against the smaller part until tonight's is marked passed and against tonight's once it is, unmeasurable or clearly_out on its first bad frame or after nine unsettled ones, clearly_out after one frame when every tile is far out, never changes the record, frees the lease, and focus_ended answers its view"
        decision: "spec.md:US8-S1"
      - path: crates/finder/tests/console/imaging_focus_check.rs
        status: new
        disposition: add
        assertion: "with the simulator's focus turned from 3.0 through 0 to -2.0 and back, the aid's median is within 3 percent of the true size at best focus, the trend goes better, worse and better, found turns true only at the end, tonight's record is marked passed only then, and the best and tonight's record are within 3 percent of the true size"
        decision: "spec.md:US8-S2"
      - path: crates/finder/tests/cli/imaging_focus.rs
        status: new
        disposition: add
        assertion: "the binary under --simulate turns its focuser through /api/sim/focus and a focus check finds the middle tile, takes three noisy readings in sensor pixels near the true size and ends done with no_record"
        decision: "spec.md:US8-S3"
      - path: crates/finder/tests/cli/simulate.rs
        status: existing
        disposition: preserve
        assertion: "the binary's start-up lines, refusals and simulated state are unchanged"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and each binary's main.rs holds only its support include and mod lines, with imaging_focus_check added to console's and imaging_focus to cli's"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "the open aid of US5, its routes, its view's other keys, its endings and its tests are unchanged; a request without check behaves as before"
      - "the focus record is not changed by a check (spec.md FR-026)"
      - "the binary prints no new start-up line and takes no new flag"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console imaging_focus_check::"
        - "cargo nextest run -p finder --locked --test console imaging_focus_aid::"
        - "cargo nextest run -p finder --locked --test cli imaging_focus::"
        - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs, and the field checks of spec.md with the real camera and ASTAP"
    writes:
      - crates/finder/README.md
      - crates/finder/src/imaging/focus/run.rs
      - crates/finder/src/lib.rs
      - crates/finder/src/main.rs
      - crates/finder/tests/cli/imaging_focus.rs
      - crates/finder/tests/cli/main.rs
      - crates/finder/tests/console/imaging_focus_check.rs
      - crates/finder/tests/console/main.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/README.md
        other_spec: 033-focus-reading-and-aid
        other_story: US7
        disposition: merge_order
        reason: "US7 depends_on_merged US8, and adds the two tools to the MCP list"
      - path: crates/finder/src/main.rs
        other_spec: 032-imaging-solve-and-aim-point
        other_story: US3
        disposition: merge_order
        reason: "033 depends_on_landed 032, whose US3 last changed main.rs"
      - path: crates/finder/tests/cli/main.rs
        other_spec: 032-imaging-solve-and-aim-point
        other_story: US3
        disposition: merge_order
        reason: "033 depends_on_landed 032, whose US3 last changed cli/main.rs"
    open_questions: []
```
