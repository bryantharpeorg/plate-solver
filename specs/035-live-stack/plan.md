# Implementation Plan: Live Stack

**Branch**: `035-live-stack` | **Date**: 2026-11-15 | **Spec**: [spec.md](spec.md)

This plan records every design decision for the feature. A story that would change one
escalates rather than re-deciding it.

## Summary

The simulator's imaging camera gains a scene: a satellite's streak on chosen frames and a
faint glow fixed on the sky, whose known effect on a frame is the oracle of the stories
that reject and that add. The console gains `stack.rs`, pure arithmetic over one sub:
`colour` turns a calibrated mosaic into three planes of the same size; `Placement` is the
linear map and shift from the reference grid into a sub, read from the two imaging
solves at five points and checked against them at the grid's four corners; `Stack`
resamples a sub through that map and keeps a sum, a
weight and, for rejection, a sum of squared deviations for every pixel, in one pass. A
`Stacker` runs the loop on the console while it holds 031's lease on the camera (031
US6): one task asks for subs back to back, another takes them from a queue of four,
solves each through 032's `solve_frame` with a hint whose sources are tried in the plan
document's order, gates it (its focus reading through 033's `focus::read`), calibrates
it through 034's library and adds it, and hands each stacked sub's solve to 032's
`note_solve`, which keeps the aim point measured. `display.rs` reduces the stack's sums
and weights to at most 1920 wide, with no mean of the full size in between, takes a
fitted plane from each channel, applies the observer's gains and an `asinh` curve, and
hands 031's JPEG encoder the result. Three routes start, stop and
show a stack; its view rides in the imaging view; at its end `stack.fits`, `stack.png`
and `stack.json` are written under `runs/<id>/`, the folder named through the one
`label` rule that 037 shares. The Imaging page gains a Stack block whose picture obeys
031's night exception through 031's own `useColourHold`, and MCP clients two tools.
Nothing changes on the scope node or the scope link, and nothing here touches the
mount.

## Technical Context

**Language/Version**: Rust (MSRV 1.96) for US1 to US8 and US10; TypeScript with React 19
for US9.

**Primary Dependencies**: nothing new, and no `Cargo.toml` changes. `finder` already has
`image` with `png` and `jpeg` (`crates/finder/Cargo.toml:27`), `serde_json`
(`crates/finder/Cargo.toml:32`), `tokio` with `fs`, `sync` and `time`
(`crates/finder/Cargo.toml:33`), `base64` (`crates/finder/Cargo.toml:11`) and the
simulator (`crates/finder/Cargo.toml:14`); its tests have the scope node's crate
(`crates/finder/Cargo.toml:36`), whose camera trait the scripted camera implements. No
FITS crate, no linear-algebra crate and no thread pool: the cube is written by
`crates/finder/src/imaging/fits.rs` (031 US2, 034 US3), the plane fit is a 3 × 3 system
solved in place, and the pixel pass runs on one blocking thread. The page adds no npm
package.

**Storage**: under the console's imaging folder (031: `imaging/` in the state folder, or
`--imaging-dir`): `runs/<id>/stack.json`, and at a stack's end `runs/<id>/stack.fits`
and `runs/<id>/stack.png`. Each sub is an ordinary imaging frame under
`frames/<UTC date>/`, saved by 031's path, with 032's solve sidecar beside it. Nothing
about a stack is kept in `session.json`; a running stack lives in memory.

**Testing**:
- US1: `crates/simulator/tests/imaging_scene.rs` (new).
- US2: `crates/finder/tests/console/stack_colour.rs` and
  `crates/finder/tests/console/stack_placement.rs` (new), plain `#[test]`s.
- US3: `crates/finder/tests/console/stack_accumulate.rs` (new): worked examples, and two
  scenarios on the sim rig.
- US4: `crates/finder/tests/console/stack_reject.rs` (new): worked examples, and one
  scenario on the sim rig.
- US5: `crates/finder/tests/console/stack_loop.rs` (new), on the stand-in stacking rig
  and the sim stacking rig; it also holds the rigs the later modules import (§ Test
  rigs).
- US6: `crates/finder/tests/console/stack_gate.rs` (new).
- US7: `crates/finder/tests/console/stack_display.rs` (new).
- US8: `crates/finder/tests/console/stack_console.rs` (new) and, for US8-S7, which
  starts the built `finder`, `crates/finder/tests/cli/stack.rs` (new).
- US9: `crates/finder/web/src/components/StackBlock.test.tsx` (new, Vitest) and
  `crates/finder/web/e2e/stack.spec.ts` (new, Playwright, simulate project).
- US10: `crates/finder/tests/mcp/stack.rs` (new).
- Where a finder test goes is 027's rule: `console` calls the router and the state in
  process, `cli` starts the built `finder`, `mcp` goes through the MCP door. Each module
  is one file in its binary's folder and one `mod` line, in alphabetical order, in that
  binary's `main.rs` (`crates/finder/tests/layout/main.rs:102`); a module's name holds
  only lower-case letters and `_` (`crates/finder/tests/layout/main.rs:155`).
  `crates/finder/tests/support` takes no new file
  (`crates/finder/tests/layout/main.rs:127`), and an unused helper fails
  `clippy -D warnings`.
- Every Rust test that waits runs in paused tokio time, but US8-S7. No test moves a
  mount, and none renders the worked example's 4944 × 3284: the simulated camera's
  1236 × 822 and the stand-in's 64 × 48 carry every scenario.

**Target Platform**: the `finder` console on the console machine. The scope node is not
rebuilt for this spec.

**Constraints**:
- No `unsafe` (`crates/simulator/src/lib.rs:2`; `finder` adds none).
- No file under `crates/scope-node`, `crates/scope-link` or `proto/` changes, and no
  `Cargo.toml` or `Cargo.lock`.
- No file under `crates/finder/src/imaging/stack.rs` or
  `crates/finder/src/imaging/stack/` reads an axis position, the site or the mount's
  state, or computes an altitude, an azimuth or a field rotation; none holds a constant
  sized to one focal length or one sensor; no limit there is a number of seconds or of
  degrees on the sky (the plan document's § Mount-neutral by design; spec FR-019).
- The stack holds 031's lease on the camera while it runs and adds no lock of its own
  over the camera (the plan document's amendment 1; § The camera's lease).
- With the simulator's scene at its default every simulated frame is byte for byte
  034's.
- Every pixel pass and every file write runs on tokio's blocking pool, never on a
  runtime thread.
- `web/dist` is committed and must be the build of its source (US9).
- Budgets are things a test or a probe measures; this plan states none for the real
  camera. The stack reports the real time each sub took, for the bring-up probe to read.

**Scale/Scope**: three new source files in the console, one extended simulator module,
three routes, two MCP tools, one page block, ten stories.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Nothing touches the finder camera's solver, star detection or `crates/math-core`. Behaviour here is grounded in `docs/research/2026-11-15-imaging-camera.md` § 7. Field rotation, sub length and the drive, § 8. Live stacking and § 10. Exposing the Orion Nebula, and colour (the order of steps, a per-pixel count, a running-statistic test after a learning set, a centre that may move half the short side, a fitted gradient and an `asinh` curve for display only), in the plan document (the order of a hint's sources), or is a design value declared as one and listed for the owner (spec owner decisions 2 to 11, 16, 17, 22 and 26). |
| II. Never weaken a gate | No existing test, golden, fixture, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS` or `ergane.yaml` is edited. The only edits to existing test and gate files are the `mod` lines and the two list entries in `playwright.config.ts` that spec owner approval 9 names. Every test 030 to 034 land must pass unchanged; § Shapes that existing tests pin says what that requires. |
| III. Tests first, independent expectations | Oracles: worked examples by hand; the simulator's truth and its declared scene (itself pinned by worked examples in US1); the stand-in camera's sample rule; a scripted solver and a scripted camera of the tests' own; a by-hand tangent-plane projection that calls nothing under `finder::imaging`; and files the test reads with its own FITS reader. Clocks are injected, and virtual time is used only in tests. One reading is not: `took_ms` is taken with `std::time::Instant`, because it measures real work; no behaviour depends on it and no test asserts its size. |
| IV. Critical paths | No `unsafe`, no dependency or `Cargo.toml` change, nothing under `proto/`, no scope-link change, no database-format change, no `build.rs` change. |
| V. Standalone Rust, honest licensing | The stacker is this project's own code. `seiza-stacking` and AstraLite's stacking modules were read about and not adopted (§ Decisions); none of their source is copied. The imaging solve the stack registers by is ASTAP's, by the owner's decision in ADR-0002, which needs the owner's reading of V before 032 is flipped (§ Prerequisites 4). |

## Project Structure

```text
crates/simulator/
├── src/imaging.rs                      # US1: SatelliteStreak, ExtendedSource, Scene, their part in a frame
├── src/optics.rs                       # US1: the scene's cell and its two methods
├── src/lib.rs                          # US1: exports
└── tests/imaging_scene.rs              # US1: new
crates/finder/
├── README.md                           # US8: ## Live stack, three rows; US10: two tool lines
├── src/lib.rs                          # US8: three route lines
├── src/api.rs                          # US5: the Stacker in AppState; US8: attach, target_label
├── src/imaging/mod.rs                  # US2: pub mod stack; US8: ImagingView.stack, attach_stack
├── src/imaging/fits.rs                 # US8: RealCube, encode_cube
├── src/imaging/routes.rs               # US8: three routes
├── src/imaging/stack.rs                # US2: new, Colour, colour, Placement with its corner check; US3: Stack; US4: rejection, levels; US5: pub mod service; US6: judge, SubSettings, same_settings; US7: pub mod display
├── src/imaging/stack/service.rs        # US5: new, the loop, note_solve at step 8; US6: the gate and the queue; US7: the picture; US8: label, the folder and the files
├── src/imaging/stack/display.rs        # US7: new
├── src/mcp/tools.rs                    # US10: two tools
├── tests/console/main.rs               # US2 to US8: one mod line a module
├── tests/console/stack_colour.rs       # US2: new
├── tests/console/stack_placement.rs    # US2: new
├── tests/console/stack_accumulate.rs   # US3: new
├── tests/console/stack_reject.rs       # US4: new
├── tests/console/stack_loop.rs         # US5: new, and the rigs
├── tests/console/stack_gate.rs         # US6: new
├── tests/console/stack_display.rs      # US7: new
├── tests/console/stack_console.rs      # US8: new
├── tests/cli/main.rs                   # US8: mod stack
├── tests/cli/stack.rs                  # US8: new, US8-S7
├── tests/mcp/main.rs                   # US10: mod stack
├── tests/mcp/stack.rs                  # US10: new
└── web/
    ├── src/api.ts                      # US9: the stack's types and calls
    ├── src/components/StackBlock.tsx           # US9: new, holding colour through 031's useColourHold
    ├── src/components/StackBlock.test.tsx      # US9: new
    ├── src/components/ImagingPage.tsx  # US9: placement
    ├── e2e/stack.spec.ts               # US9: new
    ├── playwright.config.ts            # US9: two list entries (spec owner approval 9)
    └── dist/                           # US9: rebuilt
GLOSSARY.md                             # US5: Reference sub
DESIGN.md                               # US9
.impeccable/design.json                 # US9
```

## Interfaces

Every file under `crates/finder/src/imaging/`, `crates/simulator/src/imaging.rs`, the
Imaging page and the tests of 030, 031, 032, 033 and 034 is created by those specs and is
named here without a line; the story that, by their drafts, last writes each is in
§ Refinement Record's shared writes. Types of 031 (`Imaging`, `ImagingView`,
`FrameView`, `RoiView`, `Refusal`, `FitsImage`, `CardValue`, `preview::Rgb`), of 032
(`ImagingSolver`, `SolveFrame`, `Region`, `SolveHint`, `HintSource`, `ImagingSolve`,
`SolveError`, `StarMeasure`, `Ask`, `Ended`), of 033 (`focus::read`, `FocusReading`,
`Unmeasurable`, `FocusView`, `RegionOut`) and of 034 (`Mosaic`, `percentile`,
`Library`, `FrameKey`, `Selection`, `Applied`, `Calibration`) are as those drafts'
plan.md § Interfaces give them, name for name: 030's, 031's, 032's and 034's after their
review, 033's as it was revised the evening this trio was finished. Four things used
here come from the series audit's findings on 031, 032 and 034, which were being
written into those trios while this one was revised; each is written below as this
trio uses it, to be read in those trios and then their code at the re-anchoring
(§ Prerequisites 8, 13 and 14): the camera's lease (031 US6; § The camera's lease);
`useColourHold` (031 US4, from the finding A6; § The page); `Imaging::note_solve`
(032 US4, from the finding A3; § The stack on the console, the worker's step 8); and a
set of darks or flats that holds the lease (034 A1; § Prerequisites 13). Positions and
planes are as spec.md § User Scenarios defines them: a pixel's centre is at (x + 0.5,
y + 0.5).

### The scene (`crates/simulator/src/imaging.rs`, US1)

```rust
/// A satellite's streak across chosen frames (035). Sensor pixels, unbinned.
#[derive(Clone, Debug, PartialEq)]
pub struct SatelliteStreak {
    /// The camera's frame numbers it crosses, counted from 0 as `frames_taken` counts.
    pub frames: Vec<u64>,
    pub from: (f64, f64),
    pub to: (f64, f64),
    /// Electrons in a sample on the streak's centre line, whatever its filter.
    pub electrons: f64,
}

/// A faint round glow fixed on the sky (035).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExtendedSource {
    /// J2000 degrees of its centre.
    pub ra_deg: f64,
    pub dec_deg: f64,
    /// Its Gaussian width on the sky.
    pub sigma_arcsec: f64,
    /// Electrons a second at its centre in a sample whose filter passes all of it.
    pub electrons_per_s: f64,
    /// Relative light in red, green and blue.
    pub colour: [f64; 3],
}

/// What else is in the simulated sky (035).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Scene {
    pub satellites: Vec<SatelliteStreak>,
    pub sources: Vec<ExtendedSource>,
}
```

In `crates/simulator/src/optics.rs`, `optics::Simulator`
(`crates/simulator/src/optics.rs:74`) gains one field, `imaging_scene:
Arc<Mutex<Scene>>`, built with the default in `new`
(`crates/simulator/src/optics.rs:81`), beside `imaging_bench`, the cell 034 adds for
the sensor pattern and the light path, whose types this spec does not touch: 034's
tests build them. Two methods, reached through `controls::Simulator`'s `Deref` like the
rest:

```rust
pub fn scene(&self) -> Scene;
/// Asserts every number finite, every `electrons`, `electrons_per_s` and colour not
/// negative, and every `sigma_arcsec` above 0.
pub fn set_scene(&self, scene: Scene);
```

`crates/simulator/src/lib.rs` adds
`pub use imaging::{ExtendedSource, SatelliteStreak, Scene};`.

**Their part in a frame.** 030's `SimImagingCamera::expose` makes a frame in ten numbered
steps, and 034 changes steps 5 and 6 (`specs/034-dark-and-flat-frames/plan.md` § The
sensor pattern and the light path). This spec adds to them as follows and in no other
way. The scene is read once, at step 1, with the settings, together with N, the number
`frames_taken()` reports before this exposure.

- **Steps 5 and 6, light.** With 034's `cover` true, or with a `flat_panel`, the scene
  adds nothing. Otherwise, after the stars and the sky and before 034's multiplication
  by V·D:
  - each source whose place p can be projected adds, to every sample of the region
    whose centre (x + 0.5, y + 0.5) lies within ⌈4s⌉ + 1 pixels of p,
    `electrons_per_s · seconds · colour[its filter] · exp(−d²/(2·s²))` electrons, d
    being the distance from that centre to p. p is the mean of the source's places
    under R0 and R1, each found as a star's is; s is `sigma_arcsec` divided by the
    optics' scale in arcseconds a sensor pixel, `206264.806 · 2·tan(fov/2)/width`;
  - each satellite whose `frames` holds N adds, to every sample whose centre lies
    within ⌈4σ⌉ + 1 pixels of the segment from `from` to `to`,
    `electrons · exp(−d²/(2·σ²))` electrons, d being the distance from that centre to
    the nearest point of the segment and σ the frame's star width,
    √(`PSF_SIGMA_PX`² + (`FOCUS_SIGMA_PX` · focus_error)²).

  Both are light, so 034's V·D multiplies them, and step 7's noise counts them.

With an empty scene neither is run and the frame is 034's. The truth
(`imaging_truth`) is untouched. These shapes are the simulator's own, chosen so that
each has an exact worked example; what they stand for is in
`docs/research/2026-11-15-imaging-camera.md` § 8. Live stacking.

### The arithmetic (`crates/finder/src/imaging/stack.rs`, new, US2; `Stack`, US3; rejection, US4; `judge` and `same_settings`, US6)

```rust
//! Stacking arithmetic (035): colour from a mosaic, where a sub falls on the reference
//! grid, and the sums that make a stack. Pure functions; nothing here reads a file, a
//! clock, the mount or the site.
use super::calibration::Mosaic;
use super::solver::ImagingSolve;     // US6 adds `Region` to this import

// US5 adds: pub mod service;    US7 adds: pub mod display;

/// At each corner of the reference grid the placing map may lie this many of the sub's
/// pixels from where the two imaging solves put that corner (spec owner decision 3).
pub const CORNER_SLACK_PX: f64 = 0.5;             // US2
/// A sample point this close to the edge of a sub's pixel centres counts as on it.
pub const EDGE_SLACK: f64 = 1e-6;                 // US3
/// A pixel is tested once it holds this many subs.
pub const LEARNING_SUBS: u32 = 8;                 // US4
pub const REJECT_SIGMAS: f32 = 3.0;               // US4
/// The least a deviation may be: one count.
pub const SIGMA_FLOOR: f32 = 1.0;                 // US4
/// A level is read on a grid of about this many pixels a side.
pub const LEVEL_GRID: u32 = 512;                  // US4
/// A sub's centre may lie this share of the reference's shorter side from its centre.
pub const OFF_FIELD_SHARE: f64 = 0.5;             // US6
/// The placing map's area scale may differ from 1 by this much.
pub const AREA_TOLERANCE: f64 = 0.02;             // US6
/// A sub's gain or offset may differ from the reference sub's by this much.
pub const GAIN_TOLERANCE: f64 = 0.001;            // US6
/// Two exposure settings this many milliseconds apart are the same setting. A
/// tolerance for equality, not a limit on time.
pub const EXPOSURE_TOLERANCE_MS: f64 = 1.0;       // US6

/// Three planes of 32-bit reals, red, green and blue, each row by row from the top.
#[derive(Debug, Clone, PartialEq)]
pub struct Colour {
    pub width: u32,
    pub height: u32,
    /// Channel c at (x, y) is at `c·width·height + y·width + x`.
    pub planes: Vec<f32>,
}
impl Colour {
    /// All zero.
    pub fn new(width: u32, height: u32) -> Self;
    pub fn at(&self, channel: usize, x: u32, y: u32) -> f32;
    pub fn set(&mut self, channel: usize, x: u32, y: u32, value: f32);
    pub fn plane(&self, channel: usize) -> &[f32];
}

pub fn colour(mosaic: &Mosaic) -> Colour;

/// The reference grid's position p falls at `m·p + t` in the sub.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    pub m: [[f64; 2]; 2],
    pub t: [f64; 2],
}
impl Placement {
    pub fn identity() -> Self;
    /// `None` when a point it reads has no place in the sub, or when the map strays
    /// more than `CORNER_SLACK_PX` at a corner of the reference grid.
    pub fn between(reference: &ImagingSolve, sub: &ImagingSolve) -> Option<Self>;
    pub fn apply(&self, p: (f64, f64)) -> (f64, f64);
    /// How far the sub is turned from the reference, degrees, −180 to 180.
    pub fn turn_deg(&self) -> f64;
    /// The determinant of `m`: 1 for a rigid map, negative for a mirror.
    pub fn area_scale(&self) -> f64;
    /// Where the centre of a sub of this size falls on a reference grid of that size,
    /// less that grid's centre. `None` when `m` cannot be inverted.
    pub fn centre_shift(&self, reference: (u32, u32), sub: (u32, u32)) -> Option<(f64, f64)>;
}

// US3:
/// What one `add` did.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Added {
    /// Pixels of the grid the sub reached.
    pub covered: u64,
    pub added: u64,
    pub rejected: u64,
    /// What was added to the sub's red, green and blue before anything else.
    pub offsets: [f32; 3],
}

pub struct Stack { /* width, height, sum: Vec<f32> (3 planes), weight: Vec<f32>, subs: u32;
                      US4: squares: Option<Vec<f32>> (3 planes), levels: Option<Option<[f32; 3]>> */ }
impl Stack {
    /// A stack that neither matches backgrounds nor rejects.
    pub fn new(width: u32, height: u32) -> Self;
    pub fn width(&self) -> u32;
    pub fn height(&self) -> u32;
    /// Subs added so far.
    pub fn subs(&self) -> u32;
    pub fn add(&mut self, sub: &Colour, placement: &Placement) -> Added;
    /// How many subs were added at each pixel, row by row.
    pub fn weight(&self) -> &[f32];
    /// Three planes of sums, laid out as a `Colour`'s.
    pub fn sum(&self) -> &[f32];
    /// Sum over weight; 0.0 where the weight is 0.
    pub fn mean(&self) -> Colour;
    // US4:
    /// Test each pixel against what the stack holds there (§ Rejection).
    pub fn with_rejection(self) -> Self;
    /// Shift each later sub to the first sub's levels (§ Background match).
    pub fn with_background_match(self) -> Self;
}

// US4:
/// A picture's background level in red, green and blue.
pub fn levels(picture: &Colour) -> [f32; 3];

// US6:
/// Whether a sub so placed belongs in a stack: `Err("solve_differs")` or
/// `Err("off_field")`.
pub fn judge(placement: &Placement, reference: (u32, u32), sub: (u32, u32))
    -> Result<(), &'static str>;

/// What a sub must share with the reference sub to be stacked with it.
#[derive(Debug, Clone, PartialEq)]
pub struct SubSettings {
    pub width: u32,
    pub height: u32,
    pub bits: u32,
    /// The Bayer pattern, or `None` for mono.
    pub bayer: Option<String>,
    pub binning: u32,
    /// The region of the sensor the sub covers, in sensor pixels.
    pub roi: Region,
    pub gain: Option<f64>,
    pub offset: Option<f64>,
    /// The console's exposure setting when the sub was asked for.
    pub exposure_ms: f64,
}
/// Whether a sub was taken at the reference sub's settings (§ The settings test).
pub fn same_settings(reference: &SubSettings, sub: &SubSettings) -> bool;
```

**`colour`.** The picture has the mosaic's width and height. For a mosaic with a Bayer
pattern, at least 2 wide and 2 high: at pixel (x, y), whose own colour is
`mosaic.colour(x, y)` (034), that channel is the sample; each of the other two channels
is the mean of the samples of that colour among the samples that touch (x, y): those at
(x + i, y + j) for i and j each −1, 0 or 1, not both 0, inside the frame. At a red or
blue site that is the four beside it for green and the four diagonal ones for the other
colour; at a green site, the two of each colour in line with it; at an edge or a
corner, those of them that exist, and at least one of each colour always does. For a
mono mosaic, or one under 2 wide or high, each plane is the samples. This is bilinear
interpolation on the delivered grid; it adds no sharpening and reads no setting.

**`between`.** With (W, H) the reference solve's `width` and `height`, c = (W/2, H/2),
q = min(W, H)/4 and f(p) = `sub.pixel_of(ra, dec)` for `(ra, dec) =
reference.sky_at(p)`, the two numbers handed over as `sky_at` gives them:

```text
fc  = f(c)
fxp = f((c.0 + q, c.1));  fxm = f((c.0 − q, c.1))
fyp = f((c.0, c.1 + q));  fym = f((c.0, c.1 − q))
m[0][0] = (fxp.0 − fxm.0)/(2q)    m[0][1] = (fyp.0 − fym.0)/(2q)
m[1][0] = (fxp.1 − fxm.1)/(2q)    m[1][1] = (fyp.1 − fym.1)/(2q)
t = fc − m·c
```

Any of the five being `None` makes the answer `None`. Then **the corner check**: for
each of the reference grid's four corners k = (0, 0), (W, 0), (0, H) and (W, H), in that
order, fk = f(k); fk `None`, or `hypot(fk.0 − apply(k).0, fk.1 − apply(k).1)` above
`CORNER_SLACK_PX`, makes the answer `None`. Otherwise the answer is `Some(Placement {
m, t })`. The worker counts a `None` as `off_field` (§ The stack on the console, step 6).

Because `sky_at` and `pixel_of`
use each solve's whole `cd_deg`, the map carries a rotation, a scale, a shear and a
mirror; because both go through the sky, it also carries the turn of north between the
two centres, which a map made from the two angles alone would miss (spec US2-S5). The
exact mapping between two tangent planes is projective; this one differs from it by
about d·r² radians, d the angle between the two centres and r the angle from the
centre, so in pixels it grows as the shift times the square of the field's half-size:
at the worst corner of the worked example's grid, about 0.04 px for centres 0.1°
apart and 0.10 px for the 0.25° that `judge` allows there; at the corners of a 3° × 2°
field of 3600 × 2400 pixels, 0.246 px for centres 0.249° apart east and west, 0.738 to
0.739 px for 0.747°, and 1.17 px for a sub 1° away on the diagonal, the most `judge`
allows there (inferred: worked by projecting the corners through this map and through
the two solves; spec US2-S7). Two tangent planes about one centre differ by
a linear map, so for a sub on the reference's centre, turned, scaled or mirrored, the
corner check is met to rounding (spec US2-S3). It is what stops a wide field's far sub
from being placed by a map that is wrong by more than half a pixel at its corners;
the exact mapping for every pixel is spec owner decision 3's other answer.

`apply(p)` is `(m[0][0]·p.0 + m[0][1]·p.1 + t[0], m[1][0]·p.0 + m[1][1]·p.1 + t[1])`.
`turn_deg` is `atan2(m[1][0] − m[0][1], m[0][0] + m[1][1])` in degrees: for a reference
up at θr and a sub up at θs it is θs − θr. `area_scale` is `m[0][0]·m[1][1] −
m[0][1]·m[1][0]`. `centre_shift`: with a determinant smaller than 1e-12 in size, `None`;
otherwise p = m⁻¹·((sub.0/2, sub.1/2) − t) and the answer is
(p.0 − reference.0/2, p.1 − reference.1/2).

**`add`** (US3), one pass over the stack's grid, W × H, for a sub Ws × Hs:

```text
for each row j, each column i:
    (px, py) = placement.apply((i + 0.5, j + 0.5));  u = px − 0.5;  v = py − 0.5
    unless Ws ≥ 2 and Hs ≥ 2 and −EDGE_SLACK ≤ u ≤ Ws − 1 + EDGE_SLACK
                              and −EDGE_SLACK ≤ v ≤ Hs − 1 + EDGE_SLACK: next pixel
    covered += 1
    u = clamp(u, 0, Ws − 1);  v = clamp(v, 0, Hs − 1)
    x0 = min(floor(u), Ws − 2);  fx = u − x0;  y0 = min(floor(v), Hs − 2);  fy = v − y0
    for each channel: value = the sum, over the four pixels (x0, y0), (x0 + 1, y0),
        (x0, y0 + 1), (x0 + 1, y0 + 1), of its share times its value, the shares being
        (1 − fx)(1 − fy), fx(1 − fy), (1 − fx)fy and fx·fy; a pixel whose share is
        exactly 0 is not read
    if any channel's value is not finite: rejected += 1; next pixel
    sum += value in each channel;  weight += 1;  added += 1
subs += 1
```

Positions are worked in `f64` and values in `f32`. No copy of the resampled sub is ever
held: the value is made and added in the same step, which is what keeps the working
memory to the stack and the sub in hand (§ Risks). `mean()` divides each sum by the
weight where the weight is above 0.

**Background match** (US4). `levels(picture)`: with k = max(1, ⌈max(width, height) /
`LEVEL_GRID`⌉), each channel's level is 034's `percentile` at 0.5 of that plane's values
at the pixels whose x and y are both multiples of k. A stack made
`with_background_match()` keeps the levels of the first sub it is given; for each later
sub it takes `offsets` = those levels less the sub's own, and adds `offsets[c]` to each
value of channel c right after the bilinear step. For the first sub, and without the
builder, `offsets` is zero.

**Rejection** (US4). A stack made `with_rejection()` also holds three planes of squared
deviations, S. In the pass above, after the finite check and before the sums, with w
the pixel's weight:

```text
if w ≥ LEARNING_SUBS:
    for each channel: mean = sum/w;  var = S/(w − 1);  d = value − mean
        if d·d > REJECT_SIGMAS² · max(var, SIGMA_FLOOR²): reject this pixel
    if rejected: rejected += 1; next pixel        (no sum, weight or S changes)
for each channel:
    d1 = value − (sum/w, or 0 when w is 0)
    sum += value
    d2 = value − sum/(w + 1)
    S += d1·d2
weight += 1;  added += 1
```

The pair d1·d2 is the usual running update of a sum of squared deviations; it never
subtracts two large numbers, which a sum of squares in 32-bit reals would (§ Plan traps
9). The test compares squares, so the pass takes no square root. A pixel is left out
whole when any one channel fails, so the three channels always hold the same subs and
one weight plane serves all three.

**`judge`** (US6). In this order: `area_scale` not finite, or `|area_scale − 1| >
AREA_TOLERANCE`, or `centre_shift` `None`: `Err("solve_differs")`; the shift's length
above `OFF_FIELD_SHARE · min(reference.0, reference.1)`: `Err("off_field")`; otherwise
`Ok(())`. A mirrored sub has a negative area scale and is refused by the first test.

**`same_settings`** (US6). True only when `width`, `height`, `bits`, `bayer`, `binning`
and `roi` are equal; `gain` is `None` in both, or `Some` in both and no more than
`GAIN_TOLERANCE` apart, and the same for `offset`; and the two `exposure_ms` are no
more than `EXPOSURE_TOLERANCE_MS` apart. It is a pure function so that every clause has
a worked example (spec US6-S1); the worker builds the two `SubSettings` (§ The stack on
the console, **The settings test**).

`crates/finder/src/imaging/mod.rs` gains `pub mod stack;` (US2), in alphabetical order
among its `pub mod` lines.

### The camera's lease (`crates/finder/src/imaging/mod.rs`, 031 US6; written by no story of this spec)

The plan document's amendment 1: "The console's imaging state (031) holds one lease
naming the holder; a second taker is refused with the holder's name. Each later spec
takes and releases that lease and adds no lock of its own." 031's US6, which the
series audit's finding on 031 adds to that trio, builds it "name for name as those
trios assume it", in the shape `specs/033-focus-reading-and-aid/plan.md` § The camera
lease writes down, and 033's, 034's and 037's trios use the same names. This section
cites it and adds nothing to it: no story of this spec writes any of it. 031's plan.md
§ The camera lease gives it with these names, and also has `Lease::set` and
`Lease::take` answer 409 `this console takes no imaging frames` once the `Imaging` is
gone; the re-anchoring reads each name and each point below in 031's code
(§ Prerequisites 8).
Only US5 uses it, in `Stacker::start` and in the taker, which owns the `Lease` and drops
it by returning, and one test of US5 takes a lease of its own and a frame with it.

```rust
// 031's (031 US6), as this trio uses it:
/// The imaging camera, held for one taker. Dropping it releases the camera at once.
/// It holds no strong reference to the `Imaging`.
pub struct Lease { /* 031's */ }
impl Imaging {
    /// Take the camera for `holder`, a name that reads well after "by". While another
    /// holds it: a `Refusal` with the status 409 whose message contains that holder's
    /// name. It does not wait.
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
```

This spec calls `Imaging::holder` (in `start`, step 5), `Imaging::lease` (step 6) and
`Lease::take` (the taker). It does not call `Lease::set`; `Lease::set` is what 036
calls, from the taker, to set a stack's exposure (§ Shapes that existing tests pin, the
last entry). What this spec relies on, and nothing more:

1. `lease(holder)` hands the camera to the caller or answers a 409 `Refusal` whose
   message contains the name of whoever holds it, in 031's words
   `the imaging camera is held by {holder}` (031 US6-S1). It does not wait, and it
   looks at nothing but the lease (spec US5-S2).
2. While a lease is out, `Imaging::take` and `Imaging::set` by anyone else, and so
   `POST /api/imaging/frame`, `PUT /api/imaging/settings`, 031's MCP tools, the focus
   aid and a set of darks or flats, are refused as in 1, with the holder's name, and
   that check comes before their check for an imaging frame in flight. So a second
   taker hears the holder's name even while a sub is exposing. Those refusals are
   031's code and 031's tests (031 US6-S2); spec US5-S8 reads one of them during a
   stack.
3. The holder's `take` behaves as 031's `take` does without a lease: it is refused only
   as `take` is, when the camera is not `ready` or an imaging frame is in flight (031
   US6-S3); the taker asks for a sub only once the last has ended.
4. `Imaging::abort`, `Imaging::ended`, `Imaging::view`, 032's `solve_frame` and 032's
   `Imaging::note_solve` are open to anyone, lease or no lease. Stopping an exposure is
   never refused, so the page's Abort ends a sub (spec US5-S6).
5. Dropping the `Lease` releases the camera, on every way out, also when the task that
   owns it is aborted, and a `lease` or a `take` asked for after that succeeds (spec
   US5-S4, US5-S8).
6. A single imaging frame asked for with `Imaging::take` holds no lease (031 US6).
   While it is in flight and nobody holds the lease, `holder()` is `None`, and that
   frame is the one thing `start`'s step 5 looks for. Spec US5-S2's case of a start
   right after `POST /api/imaging/frame` asserts `holder()` `None` there, so that a 031
   whose single frame did hold the lease shows up at the re-anchoring as that
   assertion and nothing subtler; `start` itself is right either way, because it looks
   for a frame in flight only while nobody holds the lease, and otherwise lets the
   lease answer.

Nothing in this spec depends on point 2 for `set`: a sub whose settings differ from the
reference sub's is caught by the settings test of US6 either way. 036 depends on it,
the other way about: under point 2 `Imaging::set` by anyone but the holder is refused
for as long as a stack holds the camera, so a stack's exposure can be changed only
through the stack's own lease, which is what 036's plan does (§ Shapes that existing
tests pin, the last entry; § Prerequisites 10). 034's `Calibration::restore_exposure`,
which went through `Imaging::set`, is gone (034 A1).

### The stack on the console (`crates/finder/src/imaging/stack/service.rs`, new, US5; the gate and the queue, US6; the picture, US7; the folder and the files, US8)

```rust
//! The live stack as the console runs it (035): subs taken back to back, solved, gated,
//! calibrated and added, until it is stopped.
/// The name a stack holds the imaging camera's lease under.
pub const HOLDER: &str = "the stack";
pub const FAILED_IN_A_ROW: u32 = 3;
pub const QUEUE_SUBS: usize = 4;                  // US6
pub const FOCUS_GATE_MIN: usize = 5;              // US6
pub const FOCUS_GATE_EXCESS: f64 = 0.3;           // US6
pub const REHINT_AFTER: u32 = 3;                  // US6
pub const DISPLAY_EVERY: u32 = 8;                 // US7
pub const FORMAT: &str = "finder-stack/1";        // US8

// US8:
/// Every character that is not an ASCII letter or digit made `-`.
pub fn label(name: &str) -> String;

/// The body of `POST /api/imaging/stack`. Absent is the default, or unchanged on a
/// running stack.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartRequest {
    pub reject: Option<bool>,
    pub focus_gate: Option<bool>,
    pub gains: Option<[f64; 3]>,
    pub stretch: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OptionsView { pub reject: bool, pub focus_gate: bool, pub gains: [f64; 3], pub stretch: f64 }

/// Subs not stacked, by reason. Every key is always written, in this order.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Dropped {
    pub not_solved: u32, pub too_few_stars: u32, pub solver_failed: u32, pub soft: u32,
    pub off_field: u32, pub solve_differs: u32, pub settings: u32, pub frame_failed: u32,
    pub unreadable: u32, pub behind: u32, pub unprocessed: u32,
}
impl Dropped { pub fn total(&self) -> u32; }

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ReferenceView {
    pub frame_id: u64, pub width: u32, pub height: u32, pub ra_deg: f64, pub dec_deg: f64,
    pub up_east_of_north_deg: f64, pub arcsec_per_px: f64, pub exposure_ms: f64,
}

/// The last sub that reached the solver. Every `Option` is skipped while `None`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LastView {
    pub frame_id: u64,
    /// `stacked`, or the key of `Dropped` it was counted under.
    pub outcome: &'static str,
    pub turn_deg: Option<f64>,
    pub shift_px: Option<[f64; 2]>,
    pub rejected_px: Option<u64>,
    pub focus_arcsec: Option<f64>,                // US6
    /// Real milliseconds from taking the sub off the queue to its outcome.
    pub took_ms: f64,
}

/// Files written at the end, relative to the imaging folder; only those written.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FilesView {                            // US8
    #[serde(skip_serializing_if = "Option::is_none")] pub fits: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub png: Option<String>,
    pub json: String,
}

/// `stack` in the imaging view. Serialized in this order; every `Option` is skipped
/// while `None`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct StackView {
    /// `stacking` or `ended`.
    pub state: &'static str,
    /// The stack's folder under `runs/`.
    pub id: String,
    /// Console Unix seconds.
    pub started_at: f64,
    pub options: OptionsView,
    pub taken: u32,
    pub stacked: u32,
    pub queued: u32,
    pub dropped: Dropped,
    /// Seconds of exposure in the stack.
    pub exposure_s: f64,
    pub reference: Option<ReferenceView>,
    pub last: Option<LastView>,
    /// What 034 found absent for the last sub calibrated: `dark`, `flat`, `bad_pixels`.
    pub missing: Option<Vec<&'static str>>,
    pub focus_median_arcsec: Option<f64>,         // US6
    pub preview: Option<String>,                  // US7
    pub ended_at: Option<f64>,
    /// `stopped` or `camera`.
    pub ended_reason: Option<&'static str>,
    pub error: Option<String>,
    pub files: Option<FilesView>,                 // US8
    pub save_error: Option<String>,               // US8
}

pub struct Stacker { /* imaging, calibration, clock, Mutex<the latest run's counts>,
                        Mutex<its pixels>, the run's stopping mark, the tasks' handles
                        (the taker owns the run's Lease);
                        US7: the pictures' own tokio::sync::Mutex<()> */ }
impl Stacker {
    pub fn new(imaging: Arc<Imaging>, calibration: Arc<Calibration>, clock: Arc<dyn Clock>)
        -> Arc<Self>;
    /// Start a stack, or change the gains and stretch of the one running.
    pub async fn start(self: &Arc<Self>, request: StartRequest, target: Option<&str>)
        -> Result<StackView, Refusal>;
    /// Stop the stack that is running and answer once it has ended. The latest view
    /// when none is running; `None` before the first start.
    pub async fn stop(&self) -> Option<StackView>;
    /// The latest stack, running or ended.
    pub fn view(&self) -> Option<StackView>;
    /// The latest stack's mean; `None` before a sub is stacked.
    pub fn mean(&self) -> Option<stack::Colour>;
    // US7:
    pub fn preview(&self) -> Option<Vec<u8>>;
    pub fn preview_within(&self, max_width: u32) -> Option<Vec<u8>>;
}
```

`Clock` is the console's (`crates/finder/src/session.rs:24`). Two locks, both
`std::sync::Mutex`: one over the counts and everything a view shows, one over the
`Stack` and the kept picture. `view()` takes only the first, so a page read never waits
for a pixel pass. Neither is held across an await, and neither while `Imaging` is
called, because `Imaging::view` calls back into `Stacker::view` (US8).

**The stopping mark** is one `tokio::sync::watch` channel of `bool`, made with the run:
`false` until the stack is ending and `true` from then on, set once, by whoever ends
the stack (**Ending**). The taker and the worker each hold a receiver. Below, "the
stack is stopping" is `*stopping.borrow()`, and to wait for the stack to stop is to
await `stopping.wait_for(|stopping| *stopping)`. 036's wait in the taker races the same
receiver (§ Shapes that existing tests pin, the last entry).

**`start`**, refusing in the order of spec FR-013, each a `Refusal`:

1. `gains` present and not three finite numbers each from 0.1 to 10: 400
   `gains must be three numbers from 0.1 to 10`; `stretch` present and not finite or
   outside 1 to 1000: 400 `stretch must be a number from 1 to 1000`.
2. A stack running: with `reject` or `focus_gate` present, 409
   `the stack is running: stop it to change reject or focus_gate`; otherwise the
   `gains` and `stretch` present are set on it; when either is present and the stack
   has a picture, its picture is made again (US7, awaited); and its view is answered.
   A request that names nothing, `StartRequest::default()` or the body `{}`, changes
   nothing there and answers the view.
3. `imaging.view()` not `ready`: 409 with 031's words for the state
   (`the scope node is not connected`, `this scope node has no imaging service`,
   `this scope node has no imaging camera`,
   `the imaging camera is not answering: {error}`).
4. `imaging.solver()` `None`: 409 `this console has no imaging solver`.
5. Nobody holds the lease (`imaging.holder()` is `None`) and the view's frame is
   `exposing` or `receiving`: 409 `an exposure is in progress`, the words 031's `set`
   and 033's `focus_start` use for it. While somebody holds the lease this step is
   passed over, whatever the frame's state, and step 6 answers with the holder's name.
   This is the check of 033's `focus_start`, its check 4: the focus aid and a set of
   darks or flats have a frame exposing almost all the time they hold the camera, and
   a look at the frame before the lease would answer them with these words and never
   with the holder's (§ The camera's lease, 6).
6. `imaging.lease(HOLDER)` (§ The camera's lease) refused: that `Refusal`, unchanged,
   status and words. No other part is asked whether it is taking frames: not
   `Calibration::busy`, not 033's `Imaging::focus_running`.
7. US8: the folder (§ The folder and the files); a failure is 500
   `cannot make the stack folder: {error}`, and the lease of step 6 is dropped before
   it is answered.

The steps are not one atomic step. The console runs on tokio's multi-threaded runtime
(`crates/finder/Cargo.toml:33`), so two starts can pass steps 1 to 5 at the same moment
on two threads, and having no await between steps 3 and 6 does not prevent that. What
makes two starts exclusive is the lease: step 6 hands the camera to one, and the other
is refused there with the lease's words, which name `the stack`. No test asserts which
of the two a same-instant start gets. Do not hold the counts' lock from step 2 to
step 6 to make them one step: `imaging.view()` and `imaging.lease` are calls into
`Imaging` (§ Plan traps 13).

Otherwise a run is made and replaces the latest: `state` `stacking`; `id` the console
clock's UTC, the first 19 characters of the second text `fits::utc` gives (031) with
each `:` made `-` and `Z` added, then `-` and `target` when one is given; `started_at`
the clock's reading; `options` from the request over the defaults (`reject` true,
`focus_gate` true, gains of 1.0, `stretch` 10.0); every count 0. The taker and the
worker are spawned, the `Lease` of step 6 is moved into the taker, and the view is
answered.

**The taker**, one task:

```text
failed = 0
loop:
    if the stack is stopping: return
    (the gap before an exposure: this spec does nothing here; 036 adds its wait)
    match lease.take("sub").await:
        Ok(view)     → id = the id of view.frame
        Err(refusal) → end the stack: camera, error = refusal.message;  return
    if the stack is stopping: imaging.abort().await
    frame = imaging.ended(id).await
    match frame.state:
        saved   → failed = 0;  taken += 1;  hand the frame to the worker
        failed  → taken += 1;  dropped.frame_failed += 1;  failed += 1
                  if failed == FAILED_IN_A_ROW: end the stack: camera, error = frame.error;  return
        aborted → end the stack: stopped;  return
```

It never waits for the worker. No exposure is asked for twice: nobody else can have the
camera while the lease is held, so a refusal means the camera is no longer `ready`. The
second look at the stopping mark, right after `take` answers, is for a stop whose own
`abort` ran before this frame existed: without it a stop would wait out a whole
exposure, which on another telescope may be minutes. In US5 the hand-over is an
unbounded queue; US6 bounds it (below). The taker owns the run's `Lease`: it is dropped
when the task returns, on each of the four returns above, and nothing else holds it.
A failed exposure is not asked for again, so every pass of the loop is a new sub.

**The worker**, one task, takes waiting subs oldest first. For a sub's `FrameView` F, in
this order, the first failure deciding its outcome and no later step being run (spec
FR-015):

1. Build the `SolveFrame` as 032's `Imaging::solve` builds one from a saved frame's
   view: the path is `imaging.root()` joined with `file`; `roi` is `Region::from` of
   the view's `RoiView`; `exposure_s` is `exposure_ms/1000`. A `saved` frame lacking
   `width`, `height`, `bits`, `binning`, `start` or `file` is `unreadable`.
2. US6: the settings test (below): `settings`.
3. The hint, from the plan document's sources in its order (spec FR-016). First
   `h = imaging.hint(&frame)` (032), which ranks a live pointing carried through the
   aim point, 032's own last solve, the session's target and none. Then:
   - `h.source` is `HintSource::Pointing`: the hint is h, as it is;
   - otherwise, when the stack has a reference sub and (US6) `misses` is below
     `REHINT_AFTER`: the stack's own hint, `SolveHint { source: HintSource::LastSolve,
     position: Some(the last solved sub's centre), radius_deg: hypot(width, height) ·
     a/3600, fov_deg: Some(height · a/3600) }`, a being that solve's `arcsec_per_px`
     and width and height this sub's. It stands where 032's `last_solve`, `target` or
     `none` would: the stack's last solved sub is the last imaging solve of this
     field. The radius is the sub's diagonal on the sky, so no number of degrees is
     written here;
   - otherwise h, as it is.

   `misses` (US6) is the run's count of misses in a row, 0 when the stack starts. Step
   4 alone changes it.
4. `solving::solve_frame(solver, clock, Ask { root, file, frame_id, frame, hint,
   requested_at: clock.now() })`, the solver being the one `imaging.solver()` gave at
   the start. Of the `Ended` it answers, an `outcome` of `Err(NotSolved { .. })` is
   `not_solved`; `Err(TooFewStars { .. })` is `too_few_stars`; any other error is
   `solver_failed`, the failed write of a solve sidecar among them. A solved sub's
   centre becomes the last solved sub's. US6: a solved sub sets `misses` to 0;
   `not_solved` and `too_few_stars` here each add 1; `solver_failed` leaves it as it
   is. No other step touches `misses`, so a sub dropped before its solve and a
   `too_few_stars` from the focus gate of step 5 count as no miss (spec FR-025).
5. US6: the focus gate (below): `soft`, `too_few_stars` or `solver_failed`.
6. With a reference sub: `Placement::between(reference solve, this solve)`; `None` is
   `off_field`; then, US6, `stack::judge`: `solve_differs` or `off_field`.
7. On the blocking pool: read the file, `fits::decode` it (031), take
   `FrameKey::from_image` and, with the key's `bayer`, `Mosaic::from_samples` (034);
   any error is `unreadable`. Then `calibration.library().calibrate(&key, &mut mosaic,
   None, angle)`, angle being `imaging.view()`'s `relation.angle_deg` (032) read before
   the pool is entered, or none; then `stack::colour`. With no reference sub yet, this
   sub becomes it: the pixels' `Stack` is
   `Stack::new(width, height).with_background_match()`, with `.with_rejection()` unless
   `reject` is false, and the placement is the identity. Then `stack.add`.
8. Counts, under the counts lock: `stacked += 1`; `exposure_s += exposure_ms/1000`;
   `missing` becomes the `selection.missing` of 034's `Applied`; for the reference sub
   `reference` is filled from its view and its solve. Then, with the counts lock let
   go, `imaging.note_solve(&frame, F.id, &solve)` (032 US4, from the audit's finding
   A3), `frame` being the `SolveFrame` of step 1 and `solve` the `ImagingSolve` of
   step 4. For a whole-sensor sub 032 takes an aim reading from it, which needs a live
   pointing that shares the sub's moment and otherwise is counted by 032 as
   `no_live_pointing`; for a sub of a region of the sensor it does nothing. It touches
   neither the page's `solve`, the kept scale nor the misses, so the stack's own counts
   and the hint's radius are as they were (spec FR-015, owner decision 20). Only a
   stacked sub reaches this step, so a sub dropped at any step gives no aim reading.

`note_solve` as this trio calls it is 032's, written into 032's US4 by the series
audit's finding on 032 (A3) and given in `specs/032-imaging-solve-and-aim-point/plan.md`
§ Solving on the console; it is read again in 032's code at the re-anchoring
(§ Prerequisites 14):

```rust
// 032's (032 US4, from the audit's finding A3):
impl Imaging {
    /// Offer a solve that `solve_frame` made outside `Imaging::solve` (035: every
    /// accepted sub). A whole-sensor frame takes an aim reading exactly as the task's
    /// step 2 does; a frame of a region does nothing. It changes neither the view's
    /// `solve`, the kept scale nor the misses.
    pub fn note_solve(&self, frame: &SolveFrame, frame_id: u64, solve: &ImagingSolve);
}
```

It is not `async` and answers nothing, so step 8 calls it and goes on. While Track is
on, 032 holds a newly measured aim point back and applies it at the first live solve
after Track is off (032's FR-036), so a tracked stack's subs fill 032's window without
moving the aim point under the tracker; with Track off, as on the sim stacking rig, the
aim point is applied at once (spec US5-S9).

Whatever the outcome from step 3 on, `last` becomes this sub: its `frame_id`, its
`outcome`, for a stacked sub `turn_deg`, `shift_px` (`centre_shift` of the reference's
size and this sub's, `[x, y]`) and `rejected_px` (`Added.rejected`), from US6 its
`focus_arcsec` when it was measured, and `took_ms`, read with `std::time::Instant` from
the moment the sub left the queue. Each outcome is also kept in a list of every sub
taken, in order, with its `frame_id` and `file` (US8 writes it).

Awaiting the solve or the measure, the worker also waits for the stack to stop; when it
stops first, the solve or the measure is dropped where it stands and this sub is counted
`unprocessed`.

**Ending.** A stack ends once, by whoever asks first: `stop()` (reason `stopped`), or
the taker (above). Ending: set the stopping mark; when it was `stop()` that asked,
await `imaging.abort()`, which ends an exposure in flight as aborted; wait for the
taker to return, which drops the run's `Lease` and so releases the camera; wait for the
worker to leave its loop; count every sub still waiting, and one abandoned in hand, as
`unprocessed`; set `ended_at`, `ended_reason` and, for `camera`, `error`; US8 writes the
files; then `state` becomes `ended` together with `files` or `save_error`. The lease
is dropped on every ending, the taker's own included, before any file is written and
before `state` becomes `ended` (spec FR-014), so the camera is free while a large stack
is saved. Nothing changes afterwards.
`stop()` awaits all of it and answers the view; with no stack running it answers
`view()`.

**The settings test** (US6), for a sub when the stack has a reference sub:
`stack::same_settings(&reference, &sub)` (§ The arithmetic), each `SubSettings` built
from that sub's `FrameView`: its `width`, `height`, `bits` and `binning`, which step 1
found present; its `bayer`; `roi` as `Region::from` of its `RoiView`; its `gain`, its
`offset` and its `exposure_ms`. False is the outcome `settings`. The reference sub's
`SubSettings` is kept with the run when the reference sub is chosen.
`FrameView::exposure_ms` is the console's exposure setting at the moment the sub was
asked for (031), so the test compares each sub's exposure with the reference sub's and
with nothing else (§ Shapes that existing tests pin, the last entry, says what that
asks of 036).

**The focus gate** (US6), for a solved sub while `focus_gate` is true. The region: with
`imaging.view()`'s `focus` (033's `FocusView`) holding a `region` (x, y, width, height
in sensor pixels) that lies inside the sub's `roi`, that is with `region.x ≥ roi.x`,
`region.y ≥ roi.y`, `region.x + region.width ≤ roi.x + roi.width` and the same down,
it is `Some(Region { x: (region.x − roi.x)/b, y: (region.y − roi.y)/b, width:
region.width/b, height: region.height/b })`, b being the sub's binning; otherwise
`None`, the whole sub. Then

```rust
focus::read(solver, frame, frame_id, region, Some(solve.arcsec_per_px / f64::from(b))).await
```

033 multiplies the solver's `half_flux_px` by the binning and by that scale, so the
reading's `arcsec` is `half_flux_px · solve.arcsec_per_px`. Its answers:

| `read` answers | The sub |
|---|---|
| `Err(_)` | `solver_failed` |
| `Ok(Err(Unmeasurable::TooFewStars { .. }))` | `too_few_stars` |
| `Ok(Err(Unmeasurable::FarOut { .. }))` | `soft` |
| `Ok(Ok(reading))` | judged on `reading.arcsec`, which is present because a scale was given |

With R the readings of the subs stacked so far, in order: when R holds
`FOCUS_GATE_MIN` or more and the reading is above `(1 + FOCUS_GATE_EXCESS)` times the
median of R, the outcome is `soft`. The median is the value at index ⌊0.5·(n − 1)⌋ of R
sorted. A sub's reading joins R only when the sub is stacked. `focus_median_arcsec` is
that median, present once R holds a reading. The stack keeps nothing of 033's: it reads
no focus record, notes no reading in it and gives no verdict of 033's (spec owner
decision 8).

**The queue** (US6). The hand-over holds at most `QUEUE_SUBS` saved subs. Handing over a
sub while it is full first removes the one that has waited longest, which is counted
`behind` and listed with that outcome. `queued` is how many wait.

**The picture** (US7). The pixels' lock also keeps the latest picture: 031's
`preview::Rgb`, its JPEG, and a count of pictures made. A picture is made after a sub
is stacked, when no sub waits or `DISPLAY_EVERY` subs have been stacked since the last
picture, and in `start` step 2 when the request names `gains` or `stretch` and the
stack has a picture. Making one is three steps, on the blocking pool:

1. Under the pixels' lock, `display::reduce(stack.sum(), stack.weight(), stack.width(),
   stack.height(), display::DISPLAY_WIDTH)`, and nothing else. The lock is held for
   that one pass over the sums and weights and is let go before step 2. No mean of the
   stack's full size is made: `Stack::mean` is not called here. At the worked
   example's size a mean is 195 MB and the reduction three planes of 1648 × 1094,
   about 22 MB (inferred from `docs/research/2026-11-15-imaging-camera.md` § 8. Live
   stacking), and with an empty queue a picture is made after almost every sub.
2. With no lock held, `display::background`, then `display::tone` with the stack's
   gains and stretch, then `preview::jpeg`.
3. Under the pixels' lock again, the picture and its JPEG are kept and the count
   rises by one; then, under the counts' lock, `preview` becomes
   `/api/imaging/stack/preview.jpg?n={count}`.

Pictures are made one at a time. The `Stacker` has a `tokio::sync::Mutex<()>` for
that alone: whoever makes a picture takes it first, reads the gains and the stretch
under the counts' lock once it has it, runs the three steps and lets it go. So a
picture asked for by a change of gains is never overtaken by an older one. It guards
nothing but the making of pictures, it is the one lock here that is held across an
await, and no call into `Imaging` is made while it is held. The worker awaits its
picture before it takes the next sub, so a picture always shows the sub just stacked.

`preview()` answers a copy of the JPEG. `preview_within(max)` copies the picture under
the pixels' lock and, with the lock let go, answers
`preview::jpeg(&display::shrink(&picture, max))`. A stack that ends keeps its picture,
and the next start drops it.

**The folder and the files** (US8). In `start`, step 7: `create_dir_all` of
`imaging.root()/runs`; `create_dir` of the folder named by the `id`, and on
`AlreadyExists` of that name with `-2`, `-3` and so on, the `id` becoming the name that
worked; any other failure is the 500. Then `stack.json` is written (below) with what is
known at the start; a failure there is the same 500. At the end, on the blocking pool,
in this order, each file whole under its name plus `.part` and then renamed:

1. with at least one sub stacked, `stack.fits`: `fits::encode_cube` of `stack.mean()`
   with the cards of § FITS. This is the one place outside the tests where a mean of
   the stack's full size is made; it and the file's bytes are dropped before step 2;
2. with it, `stack.png`: `display::render(&stack, gains, stretch, stack.width())`, the
   picture at the stack's full size with the stack's gains and stretch, encoded as an
   8-bit RGB PNG with `image`;
3. `stack.json`, with the end's keys.

The folder is not made again. The first failure stops the writing, and the view gets
`save_error` `could not save the stack: {error}` and no `files`; otherwise `files` names
what was written.

`stack.json`, written with `serde_json::to_vec_pretty`, its keys in this order: `format`
(`FORMAT`), `finder` (the crate's version), `id`, `started_at`, `started_utc` (the
second text of `fits::utc`), `target` (the label or null), `options`, `camera`
(`{"model", "serial"}` from the imaging view's camera); and at the end `ended_at`,
`ended_reason`, `error` (when there is one), `taken`, `stacked`, `dropped`,
`exposure_s`, `reference` (the view's object with `cd_deg`, the reference solve's, and
`file`, the reference sub's; null when no sub was stacked) and `subs`, an array in the
order taken of `{"frame_id", "file", "outcome"}` with, for a stacked sub, `turn_deg`,
`shift_px` and `rejected_px`, and from US6 `focus_arcsec` when measured. `file` is
031's `FrameView::file` and null for a sub that never arrived.

### The display (`crates/finder/src/imaging/stack/display.rs`, new, US7)

```rust
//! The stack as a picture (035): reduced, flattened, balanced, stretched. Display only;
//! nothing here changes what is saved as FITS.
pub const DISPLAY_WIDTH: u32 = 1920;
pub const MCP_WIDTH: u32 = 1024;
pub const TILES: u32 = 8;
pub const DEFAULT_STRETCH: f64 = 10.0;
/// White is this percentile of what is left above the background.
pub const WHITE_PERCENTILE: f64 = 0.999;
pub const NOISE_SPANS: f64 = 30.0;

/// A stack reduced by whole blocks to a mean, and which of its pixels any sub reached.
#[derive(Debug, Clone, PartialEq)]
pub struct Reduced { pub colour: stack::Colour, pub covered: Vec<bool> }

/// A background level of `a·x + b·y + c` at the position (x, y).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plane { pub a: f64, pub b: f64, pub c: f64 }

/// `sum` is three planes laid out as a `Colour`'s and `weight` one plane, both
/// `width` by `height`: what `Stack::sum()` and `Stack::weight()` give.
pub fn reduce(sum: &[f32], weight: &[f32], width: u32, height: u32, max_width: u32)
    -> Reduced;
pub fn background(reduced: &Reduced) -> [Plane; 3];
pub fn tone(reduced: &Reduced, planes: &[Plane; 3], gains: [f64; 3], stretch: f64)
    -> preview::Rgb;
/// `tone` of `reduce(stack.sum(), stack.weight(), stack.width(), stack.height(),
/// max_width)`, with its `background`.
pub fn render(stack: &stack::Stack, gains: [f64; 3], stretch: f64, max_width: u32)
    -> preview::Rgb;
/// An 8-bit picture reduced by whole blocks to at most `max_width`.
pub fn shrink(picture: &preview::Rgb, max_width: u32) -> preview::Rgb;
```

1. **`reduce`.** k = ⌈width/max_width⌉, at least 1. The result is ⌊width/k⌋ by
   ⌊height/k⌋; rows and columns that do not fill a block are dropped. For each block,
   W is the sum of its k × k weights, worked in `f64`. When W is above 0 the block is
   covered and each of its channels is the sum of that channel's k × k sums, worked in
   `f64`, over W; otherwise it is 0.0 in every channel and not covered. So a block's
   value is the mean of every sample added in it, each sub counting once, and for k = 1
   it is the stack's own mean. It reads the sums and the weights and makes nothing of
   the stack's full size but its own result: no `Colour` of the mean is built first
   (§ Plan traps 29).
2. **`background`.** With tw = ⌊width/`TILES`⌋ and th = ⌊height/`TILES`⌋, tile (p, q),
   p and q from 0 to 7, holds the pixels with p·tw ≤ x < (p + 1)·tw and q·th ≤ y <
   (q + 1)·th; there are none when tw or th is 0. For one channel: a tile counts when
   at least half of its pixels are covered; its level is 034's `percentile` at 0.5 of
   the channel over its covered pixels, and its centre is ((p + 0.5)·tw, (q + 0.5)·th).
   That `percentile` is the value at index ⌊0.5·(n − 1)⌋ of the sorted values, so for
   an even count it is the lower of the two middle ones, and a level read down a ramp
   sits half a pixel's step off the tile's centre: spec US7-S2's green plane comes to a
   `c` of 199.99, not 200, and is asserted within 0.02. Use `percentile` as it is; do
   not average the two middle values.
   The tiles kept are those whose level is at most the `percentile` at 0.5 of the
   counting tiles' levels. The plane is the least-squares fit of a·x + b·y + c to the
   kept tiles' centres and levels, worked in `f64` from the 3 × 3 normal equations.
   With fewer than 3 kept tiles, or a determinant of the normal matrix whose size is
   below 1e-9 times the product of its diagonal, the plane is flat: a = b = 0 and c
   the `percentile` at 0.5 of the kept tiles' levels. With no counting tile, c is the
   `percentile` at 0.5 of the channel over all covered pixels, or 0.0 with none.
3. **`tone`.** For each channel c and each covered pixel, v = gains[c]·(value −
   plane_c(x + 0.5, y + 0.5)). Over the covered pixels of channel c: m_c is the
   `percentile` at 0.5 of v and n_c is m_c less the `percentile` at 0.16. The span is
   the largest, over the three channels, of (`percentile` at `WHITE_PERCENTILE` − m_c)
   and `NOISE_SPANS`·n_c, and at least 1.0. A value becomes
   `round(255·asinh(stretch·t)/asinh(stretch))` with t = clamp((v − m_c)/span, 0, 1).
   A pixel not covered is (0, 0, 0). One span for all three channels keeps the ratio of
   the colours that the gains set.
4. **`shrink`.** k = ⌈width/max_width⌉; each whole k × k block becomes the mean of its
   bytes in each channel, rounded half away from zero.

The background's plane is what a fitted linear gradient is in one of the stackers read
(`docs/research/2026-11-15-imaging-camera.md` § 8. Live stacking); taking it from each
channel by itself is what makes the background neutral, which is the measured half of
colour balance, and the gains are the half the observer sets (the plan document's
Colour row). The curve and its strength are 031's preview's, so a sub's preview and the
stack look alike. `render` is what US8 writes `stack.png` from and what a test calls;
the page's picture is made from the three functions one after another, so that the
pixels' lock is held for `reduce` alone (§ The stack on the console, **The picture**).

### FITS (`crates/finder/src/imaging/fits.rs`, US8)

```rust
/// One FITS file of 32-bit reals in planes (035): a stack.
#[derive(Debug, Clone, PartialEq)]
pub struct RealCube {
    pub width: u32,
    pub height: u32,
    pub planes: u32,
    /// Plane after plane, each row by row from the top.
    pub samples: Vec<f32>,
    /// The cards after the structural ones, in order.
    pub cards: Vec<(String, CardValue)>,
}
pub fn encode_cube(cube: &RealCube) -> Vec<u8>;
```

`encode_cube` writes `SIMPLE = T`, `BITPIX = -32`, `NAXIS = 3`, `NAXIS1` (width),
`NAXIS2` (height), `NAXIS3` (planes), then `cards`, then `END`, each card formatted
exactly as 031's `encode` formats its own, with the same padding of the header and of
the data to multiples of 2880; each sample as a big-endian IEEE 754 single
(`f32::to_be_bytes`). 031's `encode` and `decode` and 034's `encode_real` and
`decode_real` are not edited. No reader is added: nothing in the console reads a stack
back.

**A stack's cards**, in order after `NAXIS3`: `IMAGETYP` (text `STACK`), `STACKID` (text,
the id), `NSUBS` (integer, the subs stacked), `EXPTIME` (real, `exposure_s`), `SUBEXP`
(real, the reference sub's seconds), `DATE-OBS` (text, the second text of `fits::utc`
for the reference sub's `start`), `INSTRUME` (text, the camera's model), `XBINNING` and
`YBINNING` (integer, the reference sub's binning), `ROWORDER` (text `TOP-DOWN`),
`CTYPE1` (text `RA---TAN`), `CTYPE2` (text `DEC--TAN`), `CRPIX1` (real, width/2 + 0.5),
`CRPIX2` (real, height/2 + 0.5), `CRVAL1` and `CRVAL2` (real, the reference solve's
`ra_deg` and `dec_deg`, each rounded to 9 decimal places), and `CD1_1`, `CD1_2`,
`CD2_1`, `CD2_2` (real, the reference solve's `cd_deg[0][0]`, `[0][1]`, `[1][0]` and
`[1][1]`, each rounded to 13 decimal places). Rounding is `(x·10ⁿ).round()/10ⁿ`; it
keeps each number inside a card's 20 columns. The file's first row is the picture's top
row, as 031's files are, so 032's `cd_deg` (east and north for a pixel right and a
pixel down) is the file's own matrix, and the centre (W/2, H/2) is the FITS pixel
(W/2 + 0.5, H/2 + 0.5), FITS counting a pixel's centre as a whole number from 1.

### 031's imaging state (`crates/finder/src/imaging/mod.rs`, US2; US8)

US2 adds `pub mod stack;`. US8: `ImagingView` gains, after its last part (`calibration`
in 034's draft, and whatever 033 adds),

```rust
#[serde(skip_serializing_if = "Option::is_none")]
pub stack: Option<stack::service::StackView>,
```

`None` in `ImagingView::off()`. `Imaging` gains a field
`stack: std::sync::OnceLock<std::sync::Weak<Stacker>>` and

```rust
/// Let every view this state answers carry the stack (035). The first call wins.
pub(crate) fn attach_stack(&self, stacker: &Arc<Stacker>);
```

Wherever `Imaging` builds the `ImagingView` it answers, after its own lock is released
and where 034 sets `calibration`, it sets `stack` to `attached.view()` when a stacker is
attached and still alive, whatever the view's `state`. The reference is weak because
the `Stacker` holds the `Arc<Imaging>`.

### The console (`crates/finder/src/api.rs`, US5; US8)

US5: `AppState` (`crates/finder/src/api.rs:101`) gains `stack: Option<Arc<Stacker>>` and

```rust
/// The live stack (035); present whenever `imaging()` and `calibration()` are.
pub fn stack(&self) -> Option<Arc<Stacker>>;
```

`with_imaging` (031 US1), right after 034 builds the `Calibration`, builds
`Stacker::new(imaging.clone(), calibration.clone(), self.clock.clone())` and keeps it.
No builder is added and no signature changes. US8 adds, in the same place,
`imaging.attach_stack(&stacker)`, and

```rust
/// The session target's label for a folder name (035): `stack::service::label` of its
/// Messier name when it has one, otherwise of its id. `None` without a target.
pub(crate) fn target_label(&self) -> Option<String>;
```

read from the session's snapshot (`crates/finder/src/session.rs:553`,
`crates/finder/src/targets.rs:41`, `crates/finder/src/targets.rs:43`). The rule itself
is `label` alone (§ The stack on the console), so `target_label` writes no character
test of its own: `label` maps each `char` of the name, an ASCII letter or digit to
itself and anything else to one `-`, and 037's `run::label` is the one line
`crate::imaging::stack::service::label(name)` (037 A2). For the session's target `M42`,
NGC 1976 with the Messier name `M42`, `target_label` is `Some("M42")` (spec US8-S3).
`SessionResponse` (`crates/finder/src/api.rs:652`) does not change: its `imaging` is
the view, which now carries the stack.

### The routes (`crates/finder/src/imaging/routes.rs`, US8)

Added to 031's `routes()`:

```rust
// POST   /api/imaging/stack              start_stack
// DELETE /api/imaging/stack              stop_stack
// GET    /api/imaging/stack/preview.jpg  stack_preview
```

- `start_stack` takes `SessionWrite` and `Result<Json<StartRequest>, JsonRejection>`
  through `sim_json` (`crates/finder/src/api.rs:570`), in that order, as 034's
  `take_darks` does; without a stacker it answers 409
  `this console takes no imaging frames`; it calls `Stacker::start(request,
  state.target_label().as_deref())`; a `Refusal` becomes `ApiError::new(status,
  message)`; otherwise it answers `Json(imaging.view())` with 200.
- `stop_stack` takes `SessionWrite` and no body; the same 409 without a stacker;
  `Stacker::stop().await`; then `Json(imaging.view())`.
- `stack_preview` takes no extractor but the state and ignores the query; it answers
  `([(CONTENT_TYPE, "image/jpeg")], bytes)` from `Stacker::preview()`, or 404
  `the console holds no stack picture`, as `live_preview` does
  (`crates/finder/src/api.rs:514`).

**The order a request is judged in**, the first that applies:

1. the origin (`SessionWrite`): 403
   `writes require the console's own origin and an allowed host`, the words of
   `check_origin` (`crates/finder/src/api.rs:361`);
2. on the `POST`, the body's shape (`sim_json`): 415 for a content type that is not
   JSON; 400 for a body that is not a JSON object, that has a key other than `reject`,
   `focus_gate`, `gains` and `stretch`, or in which `reject` or `focus_gate` is not
   true or false, `gains` is not an array of exactly three numbers, or `stretch` is
   not a number. The words of both are axum's own for the rejection, as for every
   route that goes through `sim_json`; a test asserts the status and that `error` is
   a text, never the words;
3. a console without a stacker: 409 `this console takes no imaging frames`;
4. `Stacker::start`'s own order (§ The stack on the console, **`start`**), or
   `Stacker::stop`.

`{}` is a body of the right shape: every key is absent. On a console that can stack it
starts a stack with the defaults and answers 200; on a running stack it changes
nothing and answers 200 with the view; on a console without a stacker it answers the
409 of step 3 (spec US8-S1, US8-S2, US5-S2).

`crates/finder/src/lib.rs` gains, after the last `/api/imaging` route line already
there and written as they stand here, with no backticks:
`* POST /api/imaging/stack: a JSON object with optional reject, focus_gate, gains and stretch; start the live stack, or change the gains and stretch of the one running.`,
`* DELETE /api/imaging/stack: no body; stop the live stack and save it as stack.fits, stack.png and stack.json.`
and
`* GET /api/imaging/stack/preview.jpg: the stack's latest picture as a colour JPEG at most 1920 wide.`

### `crates/finder/README.md` (US8; US10)

US8 adds, after the last `/api/imaging` row already in the `## API` table
(`crates/finder/README.md:167`; the table's last row at 49bb210 is at
`crates/finder/README.md:184`, and 031 to 034 add theirs), three rows worded like their
neighbours:

```markdown
| `POST /api/imaging/stack` | JSON object, every key optional: `reject`, `focus_gate`, `gains` (three numbers from 0.1 to 10), `stretch` (1 to 1000). Start the live stack; on a running stack, change `gains` and `stretch`. Answers 200 with the imaging view, 400 or 409 with a JSON `error`. Same-origin writes only. |
| `DELETE /api/imaging/stack` | No body. Stop the live stack and save it; answers 200 with the imaging view once `stack.fits`, `stack.png` and `stack.json` are written. Same-origin writes only. |
| `GET /api/imaging/stack/preview.jpg` | The stack's latest picture, a colour JPEG at most 1920 wide; 404 before there is one. |
```

and, at the end of the file, `## Live stack`, saying: what a stack is and that it runs
on whatever the camera points at, so the observer does a GoTo and turns Track on first;
that a stack holds the imaging camera from its start to its end, so a single imaging
frame, the focus aid and a set of darks or flats are refused while it runs, and a
stack is refused while one of those has the camera; the order a sub is handled in;
where a sub's hint comes from, in order: the live pointing through the aim point, the
stack's last solved sub, the session's target, none, and that the hint is narrow only
once the aim point is measured and a scale is kept; that each stacked sub's solve keeps
the aim point measured while the finder camera solves too, and keeps no scale; that a
sub is placed by its imaging solve and the reference sub's, that the map is checked at
the reference grid's corners, and what that needs of the imaging solver; the rejection rule, its
eight learning subs and why it is on by default; the eleven reasons a sub is dropped,
each in one line, and for `soft` that a sub's focus reading is taken in the focus
region when the console keeps one and judged against the other subs of the stack; that
the camera is never kept waiting and what `behind` means; the
`stack` object's keys; the display's steps and that they change nothing saved; the
gains and the stretch; the folder `runs/<id>/`, the three files, the cards of
`stack.fits` and the keys of `stack.json`, and that every sub stays under `frames/`
with its solve sidecar; that nothing is saved if the console exits mid-stack; the
memory a stack holds, as planes of the camera's size, with the picture made from a
reduction and a mean of the full size made only when the stack is saved; that a
trailed sub which solves is stacked unless the focus gate finds it soft; and `took_ms`
as the figure to read against the sub length at bring-up. US10 adds two lines under `## MCP clients`
(`crates/finder/README.md:57`), after the calibration tools' lines that 034 adds:
`set_stacking` and `get_stack`, each with one sentence.

### `GLOSSARY.md` (US5)

After the **Stack** entry (`GLOSSARY.md:171`), with the words of spec owner approval 6:

```markdown
**Reference sub**:
The first sub a stack accepts. Every later sub is rotated and shifted onto its grid by the two subs' imaging solves.
_Avoid_: reference frame, key frame, master
```

### The page (`crates/finder/web/src/`, US9)

`api.ts`, after the calibration types 034 adds:

```ts
export interface StackDropped {
  not_solved: number; too_few_stars: number; solver_failed: number; soft: number;
  off_field: number; solve_differs: number; settings: number; frame_failed: number;
  unreadable: number; behind: number; unprocessed: number;
}
export interface StackView {
  state: "stacking" | "ended"; id: string; started_at: number;
  options: { reject: boolean; focus_gate: boolean; gains: [number, number, number]; stretch: number };
  taken: number; stacked: number; queued: number; dropped: StackDropped; exposure_s: number;
  missing?: ("dark" | "flat" | "bad_pixels")[]; focus_median_arcsec?: number; preview?: string;
  ended_reason?: "stopped" | "camera"; error?: string;
  files?: { fits?: string; png?: string; json: string }; save_error?: string;
}
export interface StackStart { reject?: boolean; focus_gate?: boolean; gains?: [number, number, number]; stretch?: number }
export async function postImagingStack(body: StackStart): Promise<ImagingView>;
export async function deleteImagingStack(): Promise<ImagingView>;   // fetch(url, { method: "DELETE" })
```

`ImagingView` gains `stack?: StackView;`. Each call throws an `Error` with the answer's
`error` on a refusal, as `postTrack` does (`crates/finder/web/src/api.ts:312`).
`postImagingStack` sends `{ method: "POST", headers: { "Content-Type":
"application/json" }, body: JSON.stringify(body) }`.

`components/StackBlock.tsx`, `StackBlock({ view, night, onChange }: { view: ImagingView;
night: boolean; onChange: () => void })`, a `<section aria-labelledby="stack-title"
className="space-y-3">` whose `h2` (`section-title`) reads `Stack`. Its states, with
these words exactly:

- **The colour fields**, in every state: number fields `Red gain`, `Green gain` and
  `Blue gain` and `Stretch`, starting from the stack's `options` when there is a stack
  and from 1, 1, 1 and 10 otherwise. Drafts survive the one-second reads.
- **No stack, or one that ended**: the button `Start stacking` (`button
  button-primary`), which calls `postImagingStack({ gains: [r, g, b], stretch })` with
  the fields' numbers, then `onChange`. With no stack, also the `quiet` paragraph
  `Takes subs one after another and stacks them into one picture. Turn Track on first.`
  and no `status` or `alert`.
- **Stacking**: one `role="status"` element reading
  `Stacking · {stacked} of {taken} subs stacked · {exposure} of exposure`, the exposure
  being `{whole seconds} s` under 60 seconds and `{minutes} min {seconds} s` from 60,
  both rounded down. When the dropped counts sum above 0, a `quiet` line
  `Dropped {sum}: {parts}`, the parts being the counts above 0, in the order of
  `StackDropped`, joined by `, `, each `{count} {words}` with the words `not solved`,
  `too few stars`, `solver failed`, `soft`, `off the field`, `solved differently`,
  `settings changed`, `not received`, `unreadable`, `behind` and `unprocessed`. When
  `missing` is not empty, a `quiet` line `Stacked without {parts}.`, the parts being `a
  master dark`, `a master flat` and `a bad-pixel map` for `dark`, `flat` and
  `bad_pixels`, joined by `, `. The buttons `Stop stacking` (`button`), which calls
  `deleteImagingStack()` then `onChange`, and `Apply colour` (`button`), which calls
  `postImagingStack({ gains, stretch })` then `onChange`.
- **Ended**: one `role="status"` element reading
  `Stack {id}: {stacked} subs stacked, {dropped sum} dropped.`, followed, when `files`
  has both `fits` and `png`, by ` Saved {files.fits} and stack.png.` For `camera`, a
  `role="alert"` `Stacking stopped: {error}.` With `save_error`, a `role="alert"`
  `The stack was not saved: {save_error}.`
- **The picture**, when the stack has a `preview`, in either state:
  `<img alt="Stack of {stacked} subs" src={preview}>` with the class `photo` unless
  colour is held. With `night`, under it, the button `Hold to see colour` (`button`,
  `aria-pressed`) and the `quiet` line
  `The picture is red at night. Hold the control to see its colours for up to 10 seconds.`
  Colour is held through 031's own hook, `useColourHold` (031 A6), the one 031's
  picture uses: held from `pointerdown` or a space or Enter `keydown` whose `repeat` is
  false, until the first of `pointerup`, `pointerleave`, `pointercancel`, `blur`, the
  key's `keyup`, or 10 s. `StackBlock` calls the hook once, gives the picture the class
  `photo` while the hook says colour is not held, and spreads the hook's handlers on
  the button with `aria-pressed` set to `held`; it writes no timer, listener or key
  test of its own.
  Nothing but this picture's class changes. In daylight there is no button, and the
  hook's answer is not read.
- **A refused request**: a `role="alert"` with the answer's `error`.

Write each phrase a test asserts on one line of JSX, so `textContent` keeps it whole.
Only existing classes and tokens are used; `index.css` does not change.

`useColourHold` is 031's: the series audit's finding on 031 (A6) makes the hold one
hook, which 031's US4 creates as `crates/finder/web/src/useColourHold.ts` beside its
`useImaging.ts` and uses for its own picture
(`specs/031-imaging-frames-on-console/plan.md` § The page, and 031 spec.md US4-S5, which
renders it with `renderHook`). As 031's plan.md gives it, and as this trio uses it:

```ts
// 031's (031 US4, from the audit's finding A6), crates/finder/web/src/useColourHold.ts:
import type { KeyboardEvent, PointerEvent } from "react";

/** The longest one hold shows colour. */
export const COLOUR_HOLD_MS = 10_000;

export interface ColourHoldBind {
  onPointerDown: (event: PointerEvent<HTMLButtonElement>) => void;
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

`StackBlock` writes `const { held, bind } = useColourHold(night)` once, renders
`<button type="button" className="button" aria-pressed={held} {...bind}>Hold to see
colour</button>` at night, as 031's picture does, and gives the picture the class
`photo` unless `held`. The hook ends a hold when `night` turns false, so the block
needs nothing for that. The re-anchoring reads the hook in 031's code
(§ Prerequisites 8); if 031 lands it with another return shape, 031's is written here
and in `StackBlock`'s task, and the behaviour spec US9-S3 asserts does not change,
because it is 031's.

`ImagingPage.tsx`: for a ready view, `<StackBlock view={view} night={night}
onChange={refresh} />` after the Calibration block 034 places. `useImaging` and
`useSession` do not change: the block reads the view the page already polls.

`e2e/stack.spec.ts` runs in the simulate project. It defines its own off-red counter,
written as `crates/finder/web/e2e/session.spec.ts:45` writes it, and turns Night on as
that file does. `playwright.config.ts` gains `"**/stack.spec.ts"` at the end of the
default project's `testIgnore` (`crates/finder/web/playwright.config.ts:36`) and of the
simulate project's `testMatch` (`crates/finder/web/playwright.config.ts:53`), which
031's edit has already put one entry a line. The screenshot taken during a hold masks
the stack's picture with `mask` and `maskColor: "#000000"`.

### `DESIGN.md` and `.impeccable/design.json` (US9)

After the Calibration block entry 034 adds (the Recording entry is at `DESIGN.md:350`
at 49bb210, and 031, 032, 033 and 034 add theirs after it), a plain `###` heading and one
paragraph (the `>` marks only set the text apart here):

> ### Stack block — built
> On the Imaging page, after Calibration: a section titled Stack. Four ruled number
> fields set the red, green and blue gains and the stretch. One primary button, Start
> stacking, becomes a status of the subs stacked and taken and the exposure gathered,
> with Stop stacking and Apply colour as secondary buttons; quiet lines say how many
> subs were dropped and why, and which masters were missing. The stack's picture is a
> full-width ruled photograph under `.photo`, red by luminance at night, with the Hold
> to see colour button under it at night only, as the Red Torch Rule's one exception
> allows a picture from the imaging camera. After a stack the status names its folder
> and files. Existing ink, paper and red-only night tokens; no new class.

`.impeccable/design.json` gains one entry in `components`, after the Calibration block's,
formatted as its neighbours (`.impeccable/design.json:723` is Recording's at 49bb210):
`name` `Stack block`, `kind` `custom`, `refersTo` `button`, a `description` that says
the same in one sentence each, and `html` and `css` snippets under the class
`ds-stack-block`, using only `var(--color-ink)`.

### The MCP tools (`crates/finder/src/mcp/tools.rs`, US10)

One struct that only describes a tool's input, beside `TargetArguments`
(`crates/finder/src/mcp/tools.rs:31`) and written as it is, with
`#[expect(dead_code, reason = "schema only; StartRequest parses the actual arguments")]`:

```rust
#[derive(rmcp::schemars::JsonSchema)]
struct StackingArguments {
    on: bool,
    reject: Option<bool>,
    focus_gate: Option<bool>,
    gains: Option<Vec<f64>>,
    stretch: Option<f64>,
}
```

Two tools in the `#[tool_router]` block (`crates/finder/src/mcp/tools.rs:165`), after the
calibration tools 034 adds, each `output_schema = object_schema()`
(`crates/finder/src/mcp/tools.rs:104`):

- `set_stacking`, description
  `Start or stop the live stack of subs, as POST and DELETE /api/imaging/stack do. With on true it also takes reject, focus_gate, gains and stretch; on a running stack it changes gains and stretch.`,
  with `input_schema` from `StackingArguments` as `set_target` declares its own
  (`crates/finder/src/mcp/tools.rs:355`). It takes `arguments: JsonObject` and judges
  them in this order, each failure a `CallToolResult::error` with the words given:
  1. `on` absent or not a boolean: `on must be true or false`. This is its answer to
     `{}`, on every console, with or without imaging;
  2. `on` false and any other key present: `on false takes no other argument`;
  3. `on` true: `on` is removed and the rest is parsed with
     `serde_json::from_value::<StartRequest>(Value::Object(rest))`; an error is serde's
     own words (`error.to_string()`), which no test asserts. That covers a key that is
     not `reject`, `focus_gate`, `gains` or `stretch`, a `reject` or `focus_gate` that
     is not a boolean, `gains` that are not three numbers and a `stretch` that is not a
     number;
  4. no stacker: `this console takes no imaging frames`;
  5. `on` false: `Stacker::stop().await`, and `CallToolResult::structured` of the view,
     or of `{"state":"none"}` when it is `None`;
  6. `on` true: `Stacker::start(request, state.target_label().as_deref())`, and the
     view, or the `Refusal`'s words as the error, in `start`'s own order.

  The arguments' shape is judged before the console is asked what it has, as on the
  routes (spec US10-S3, US10-S4).
- `get_stack`, `annotations(read_only_hint = true)`, description
  `Read the live stack: its state, counts, dropped subs by reason and, when it has one, its picture as a colour JPEG.`.
  It takes `arguments: JsonObject`; any key is the error `get_stack takes no arguments`.
  It answers `CallToolResult::structured` of `Stacker::view()`, or of
  `{"state":"none"}` without a stacker or a stack, and pushes
  `ContentBlock::image(base64 of preview_within(display::MCP_WIDTH), "image/jpeg")`
  after the text block when a picture is kept, as 031's `take_imaging_frame` pushes its
  preview.

Neither is a moving tool: no rate limit and no call-log line. The MCP session tools
carry `imaging.stack` because they serialize `session_response`
(`crates/finder/src/mcp/tools.rs:347`); no other MCP code changes.

### Test rigs (`crates/finder/tests/console/stack_loop.rs`, US5)

Written once, `pub(crate)`, and imported by `stack_gate`, `stack_display` and
`stack_console` with `use crate::stack_loop::…`. The names are fixed here so that a
later story can call them without reading the file:

```rust
/// What the nth `solve` answers, n from 1, when the scenario scripts it.
pub(crate) enum Scripted { Solve { ra_deg: f64, dec_deg: f64, up_deg: f64, arcsec_per_px: f64, mirrored: bool }, Fail(SolveError) }
pub(crate) struct ScriptedSolver {
    /* the solve scripts, a delay, the hints given, */
    /// What the nth `measure` answers, n from 1: a half-flux diameter in pixels, or an error.
    pub(crate) measure_script: Mutex<Vec<(usize, Result<f64, SolveError>)>>,
    /// For every `measure` so far: the frame's width and the region it was given.
    pub(crate) measure_log: Mutex<Vec<(u32, Option<Region>)>>,
}
impl ScriptedSolver {
    pub(crate) fn new() -> Self;
    pub(crate) fn solve_at(self, n: usize, answer: Scripted) -> Self;
    pub(crate) fn delay(self, each: Duration) -> Self;
    pub(crate) fn hints(&self) -> Vec<SolveHint>;
}
impl ImagingSolver for ScriptedSolver { /* name() is "simulated" */ }

pub(crate) struct ScriptedCamera {
    /* a SyntheticImagingCamera, the failure scripts, a count, */
    /// The gain the nth `expose` answers its frame with, n from 1.
    pub(crate) gain_script: Mutex<Vec<(usize, f64)>>,
}
impl ScriptedCamera {
    pub(crate) fn new(clock: Arc<dyn NodeClock>) -> Self;
    pub(crate) fn fail_at(self, n: usize, text: &str) -> Self;
    pub(crate) fn fail_from(self, n: usize, text: &str) -> Self;
}
impl scope_node::imaging::ImagingCamera for ScriptedCamera { /* … */ }

pub(crate) struct Rig { pub state: AppState, pub app: Router, pub sim: Simulator,
    pub solver: Arc<ScriptedSolver>, pub root: PathBuf, /* the scratch folder's guard */ }
/// The stand-in stacking rig; with a camera, that one in the stand-in's place.
pub(crate) async fn stand_in_rig(name: &str, solver: ScriptedSolver, camera: Option<ScriptedCamera>) -> Rig;
/// Read the stack's view as JSON every 100 ms until `done` holds, for at most 300 s.
pub(crate) async fn wait_stack(rig: &Rig, done: impl Fn(&serde_json::Value) -> bool) -> serde_json::Value;
/// A FITS file's cards, as (keyword, value text) with quotes and padding removed.
pub(crate) fn cards(bytes: &[u8]) -> Vec<(String, String)>;

// US6, in `stack_gate`, over the three fields above:
pub(crate) trait GateScripts {
    /// The nth `measure` answers this half-flux diameter in pixels, or this error.
    fn measure_at(self, n: usize, answer: Result<f64, SolveError>) -> Self;
    /// For every `measure` so far: the frame's width and the region it was given.
    fn measures(&self) -> Vec<(u32, Option<Region>)>;
}
impl GateScripts for ScriptedSolver { /* … */ }
pub(crate) trait CameraScripts {
    /// The nth `expose` answers its frame with this gain.
    fn gain_at(self, n: usize, gain: f64) -> Self;
}
impl CameraScripts for ScriptedCamera { /* … */ }

// US8, in `stack_console`, private to it:
/// One 32-bit real of a cube read by hand: plane, x, y.
fn real_at(bytes: &[u8], plane: u32, x: u32, y: u32) -> f32;
```

`Scripted::Solve` builds `ImagingSolve::from_cd` for the frame's own width and height
from the form of spec.md § User Scenarios, with the matrix's first column negated when
`mirrored`, and sets `stars` to `Some(50)`. `Scripted::Fail` carries the `SolveError`
whole, so a scenario's `NotSolved` is `Fail(SolveError::NotSolved { detail: None })`.
The scripted `measure` answers `StarMeasure { half_flux_px, stars: 50 }`, 2.0 px unless
scripted, and logs every call. `wait_stack` reads
`serde_json::to_value(rig.state.stack().unwrap().view())`; `stack_console` has its own
wait over `GET /api/imaging`.

US5 declares only what US5's tests use, because a `pub(crate)` helper nobody in the
binary calls fails `clippy -D warnings`. US5's scenarios script no `measure` and no
gain, so US5 writes no method for either: it leaves the three `pub(crate)` fields
above, which its `measure` and `expose` already read and write, and US6 puts the
methods on them through the two traits, in its own module. A field that code locks and
reads is not dead, so US5 passes without them. `real_at` is first used by US8 and is
written there. `crates/finder/tests/mcp/stack.rs` cannot import any of this and writes
a scripted solver of its own with `new`, `solve` and `measure` alone.

## Shapes that existing tests pin

These are not changed by any story:

- **Every simulated frame with the default scene.** 030's and 034's simulator tests
  compare samples exactly; the scene's two additions run only when it is not empty, and
  `Scene` is a new cell, so `SensorPattern`, `LightPath` and `ImagingOptics`, which
  those tests build by literal, gain no field.
- **`Simulator::new` and the truth.** The scene changes neither; every rig that builds a
  simulator keeps compiling.
- **`Imaging::take`, `ended`, `abort`, `set` and `view`, and where a frame is saved.**
  The stack edits none of them. It takes its subs through the lease, which is 031's own
  code (§ The camera's lease), and calls `ended`, `abort` and `view` as they are. 031's
  tests pin `frames/<UTC date>/<id>.fits`, the id rule and the refusals' words; a sub
  is one more saved frame with the purpose label `sub`.
- **033's `focus::read`, its focus view and what it keeps.** `read` "keeps no state and
  writes nothing" and is called as it is; the stack reads `focus.region` from the
  imaging view and writes nothing of 033's: no region, no record, no `focus.json`.
- **`GET /api/imaging`, `GET /api/session` and the MCP session bodies on a console where
  no stack was started.** `stack` is skipped while there is none, so every body 031 to
  034 assert is byte for byte what it was, the views outside `ready` included.
- **032's `solve_frame`, `Imaging::solve`, the page's `solve`, the kept scale and the
  relation.** A sub's solve goes through `solve_frame`, which "touches neither the
  imaging view nor the relation"; `POST /api/imaging/solve` behaves as before during a
  stack. A stacked sub's solve then reaches the relation through `note_solve` alone,
  which 032 writes (032 A3) and which takes an aim reading as 032's own task does, so
  032's tests of the relation and the window hold as they are; neither the page's
  `solve` nor the kept scale changes (spec US5-S9).
- **034's `Library::calibrate` and its library.** Called as they are; `match` in the
  calibration view follows the latest sub, as 034's owner decision 22 has it.
  `Calibration::busy` and `restore_exposure` are gone (034 A1), and nothing here would
  call them: a set of darks or flats holds the lease as the stack does.
- **031's `encode` and `decode`, 034's `encode_real` and `decode_real`.** `encode_cube`
  is a new function; `decode` of a cube still answers
  `a 3-axis FITS file is not one image`, because 031's `decode` looks at `NAXIS` before
  `BITPIX`, and 034's `decode_real` answers the same.
- **`preview::rgb`, `preview::jpeg` and the eight kept previews.** The stack's picture
  is kept by the `Stacker`, not among them.
- **The MCP tool list as tests read it.** They look tools up by name; one calls every
  listed tool with `{}` and checks for the token
  (`crates/finder/tests/mcp/motion.rs:517`), so each new tool must answer `{}` at once
  on a console with no imaging (spec US10-S4).
- **Finder's test layout** (`crates/finder/tests/layout/main.rs:102`): no file at the
  top level of `crates/finder/tests`, no new file in `support`
  (`crates/finder/tests/layout/main.rs:127`), each binary's `main.rs` holding only its
  support include and `mod` lines (`crates/finder/tests/layout/main.rs:156`), and
  module names of lower-case letters and `_`.
- **The Imaging page without a stack.** The component tests of 031 to 034 render views
  with no `stack`; the block then shows one more region with a button and four fields,
  no `status`, no `alert` and no picture, so a test that reads the page's only status,
  alert or picture finds what it found. A test that reads every button or every spin
  button of the whole page would see five more: none of the four drafts describes one,
  and the re-anchoring checks (§ Prerequisites 2).
- **031's Playwright check of the Imaging page at night.** It never starts a stack, so
  the page it screenshots gains only ink the night edition already turns red.
- **Shapes this spec pins for its own later stories.** `StackView` is
  `#[non_exhaustive]` and tests compare it key by key; `Dropped` always writes its
  eleven keys, and no test counts them, since a later spec adds a reason; `Stack::new`,
  `add` and `Added` keep their signatures from US3 on, US4 adding builders only;
  `Stacker::start`, `stop`, `view` and `mean` keep theirs from US5 on.
- **The seam for 036 and 037.** Neither is planned here, but the places they add to
  and the rules they add under are fixed, so that neither needs this spec's loop
  redesigned. The plan document gives the stack and the tracker one interface, which
  036 defines; this is where it meets the loop.
  1. *The gap before an exposure.* In the taker's loop, after the look at the stopping
     mark and before `lease.take("sub")`, once for every sub. 036 awaits its
     `Steering::before_exposure` there in a `tokio::select!` that is `biased` toward
     the stopping mark's `wait_for`, and returns from the taker when the mark wins.
     While that wait lasts no exposure is asked for: that is how the stack pauses
     exposures while the tracker is not steady, and `stop()` still ends at once. This
     spec awaits nothing there.
  2. *A stack's exposure is set through the taker's lease, in that gap, before the
     stack's first exposure:* `lease.set(SettingsEdit { exposure_ms: Some(ms),
     ..Default::default() })`. No other way exists while the stack runs: `Imaging::set`
     is refused for anyone but the holder (§ The camera's lease, 2). 036's plan does
     exactly this, with `Lease::set` from the taker before its first `lease.take`
     (036 plan.md § The stack; § Prerequisites 10). The exposure the observer had set
     is put back the same way, through the lease, where the taker returns and before
     the lease is dropped, so that no other taker comes between.
  3. *The settings test compares each sub with the reference sub,* not with what the
     gap last set (**The settings test**). An exposure set before a stack's first
     exposure is the reference sub's and every sub's, and nothing is dropped. An
     exposure changed after the reference sub was taken makes every later sub
     `settings`. So the tracker's cap is decided once, before the first sub, and held,
     which is what 036's plan does. An exposure that follows the tracker's limit sub
     by sub would need subs weighted by exposure, and is out of scope (spec
     § Assumptions).
  4. *The worker's two places.* Before the settings test of step 2, where 036 drops a
     sub taken while the tracker was not steady, under a reason it adds to `Dropped`;
     and step 8, the one place a sub is accepted, where 036 hands the tracker that
     sub's `SolveFrame` and `ImagingSolve`, after this spec's `note_solve` and with no
     lock held.
  5. *037 wraps `Stacker::start` and `Stacker::stop` and reads `view()`.* It takes the
     folder this spec names and the list of subs in `stack.json` as they are; it lets
     go of its own lease before it calls `start`, which takes the lease as
     `the stack`; and its `run::label` is `label` of this file, so a run's id and its
     stack's folder end alike (037 A2).

## Decisions and the alternatives rejected

| Decision | Rejected | Why |
|---|---|---|
| The stacker is this project's own code | `seiza-stacking`; vendoring AstraLite's stacking modules | The owner chose to build it (the plan document's Stacker row). `seiza-stacking` is Apache-2.0, first published in July 2026 with 72 releases since, and depends on the `seiza` crate, whose licence expression includes CC-BY-4.0, which `deny.toml` does not allow (`deny.toml:61`); its default feature pulls in a GPU stack, and it was not built or run. AstraLite is MIT; its stacker modules depend only on `rayon` and `itertools`, but the crate as a whole brings a windowing toolkit, so reuse means vendoring one maintainer's young code, and it registers by star triangles with shift and angle only, where the owner's decision is registration from the imaging solves. Both are in `docs/research/2026-11-15-imaging-camera.md` § 12. Prior art and what was not adopted. A trial of either on real subs before the flip stays the owner's option (§ Prerequisites 6). |
| Registration from each sub's imaging solve | Star matching against the reference sub; rotation computed from site, time and target | The owner's decision: ASTAP solves every sub. It needs no star detection of ours on 16-bit data, works through a half turn and on any mount, and reads no clock or site (the plan document, § Mount-neutral by design). |
| The whole transform between two solves, as a linear map and a shift | Shift and angle; a similarity; the exact projective mapping for each pixel | Shift and angle leave 0.5 to 0.7 px over an hour that starts 90 minutes past transit; a similarity leaves about 0.2 px at the corner over one hour and 0.3 to 0.5 px over two, and a linear map 0.01 px or less (`docs/research/2026-11-15-imaging-camera.md` § 8. Live stacking). The solves' `cd_deg` already hold the linear part, so carrying it costs nothing. The exact mapping differs by about 0.1 px at most inside what `judge` allows at the worked example's size (inferred, § The arithmetic, **`between`**) and costs a division a pixel. |
| The linear map is checked at the reference grid's four corners, and a sub it misses by more than 0.5 px there is `off_field` | No check; the exact mapping for every pixel; a check at every pixel | The map's error grows as the shift times the square of the field's half-size: 0.10 px at most at the worked example's size, 1.17 px at the corners of a 3° × 2° field of 3600 × 2400 pixels inside `judge`'s limit (spec US2-S7). Four more projections a sub keep the cheap map honest on any field; the error is largest at the corners, so they bound it. The exact mapping is spec owner decision 3's other answer. |
| A stacked sub's solve gives 032 an aim reading through `note_solve` | A sub's solve feeds only the stack; every solved sub, stacked or not, also keeping the scale and showing as the page's solve | The plan document's Two cameras row measures the aim point whenever an imaging solve and a live pointing share a moment; a stack is most of a night's solves, and a stale aim point puts the target off the sensor's centre once the subs stop and the tracker's own corrections steer. A sub that was dropped may be off the field or soft, so it gives none. Keeping the scale from a sub would narrow every later hint on one sub's word, and showing it as the page's solve would replace the observer's own; neither is asked (spec owner decision 20). |
| `start` looks for a frame in flight only while nobody holds the lease | Looking always, before the lease (the trio's earlier text); leaving it all to the lease | 033's `focus_start` check 4 and its reason: a holder has a frame exposing almost all the time it holds the camera, so looking always answered `an exposure is in progress` and never the holder's name, against the amendment. Leaving it all to the lease would rest on a single imaging frame holding the lease, which 031 US6 says it does not. |
| The rule for a target's name in a folder is one function, `stack::service::label`, called by `target_label` and by 037 | The same rule written in `target_label` and again in 037's `run.rs` | One function cannot drift from itself, and a run's id and its stack's folder must end alike (037 A2). |
| The stack's picture holds colour through 031's `useColourHold` | A second hold, with its own timer and listeners, in `StackBlock` | 031 A6 takes the hold out of 031's page so every imaging picture shares it; the Red Torch Rule's one exception is then one piece of code. |
| The map is read through the sky at five points | The product of one solve's matrix and the other's inverse | Each solve's angle is from its own north; between two centres north turns by the difference in right ascension times the sine of the declination, 0.3 px at 500 px for centres 0.02° apart at declination 60° (spec US2-S5). |
| Colour by bilinear interpolation at the sub's own size | One colour pixel a 2 × 2 cell; a sharper demosaic | The stack keeps the camera's sampling, and the observer can forward binned 2 × 2 when memory or time is short. A sharper method invents detail that seeing does not hold at this scale and costs more. |
| Bad pixels repaired before colour, by 034 | Repair in the colour picture | A hot pixel spread by interpolation draws a coloured arc as the field turns (`docs/research/2026-11-15-imaging-camera.md` § 9. Calibration frames). |
| Bilinear resampling, fused with the sums | Bicubic or Lanczos; a resampled copy then a second pass | Bilinear is what the live stackers read use; bicubic costs 1.7 times and Lanczos about 8. A naive rejection written as separate array passes cost about 9 times everything else in the benchmark (`docs/research/2026-11-15-imaging-camera.md` § 8. Live stacking); one pass also saves a picture's worth of memory. |
| A weight for each pixel | One count for the stack | Dividing by one count darkens every edge later subs do not reach (`docs/research/2026-11-15-imaging-camera.md` § 8. Live stacking). |
| Rejection against the stack's running mean and deviation, after 8 subs | No rejection; a window of five placed subs and their median; dropping the brightest value | The geostationary belt crosses the showcase field. A window of five costs five pictures more of memory; dropping the brightest value biases every pixel. 5 to 10 learning subs is one stacker's own figure (`docs/research/2026-11-15-imaging-camera.md` § 8. Live stacking). |
| A sum of squared deviations kept by the running update | A sum of squares | In 32-bit reals a sum of squares loses the variance to rounding once the stack is deep (§ Plan traps 9). |
| A pixel is left out whole when one channel fails | Each channel by itself | One weight plane serves three channels, and a streak is white. |
| Background matched by adding, per channel, from medians on a grid | No matching; a scale | Without it a brightening sky makes every later pixel an outlier. A scale needs the stars' fluxes; it is left out with transparency. |
| Two tasks and a queue of four that drops the oldest | One task that takes and stacks in turn; an unbounded queue | The camera must never wait for the stack (spec FR-024), and a queue without a bound is memory without one. The newest subs are the ones whose hints are freshest. |
| A sub's hint: 032's when it comes from a live pointing; otherwise the stack's own, from its last solved sub, with a radius of the sub's diagonal on the sky | The stack's own hint ahead of the live pointing (the first draft); 032's `Imaging::hint` for every sub; a radius in degrees | The plan document fixes the order: the live pointing carried through the aim point, the last imaging solve, the session's target, none (§ Mount-neutral by design, 4; § Solver port). A live pointing follows a mount that has moved, and the stack's last solve does not. The stack's last solved sub is the last imaging solve of this field, good to arcseconds and with a known scale, so it stands where 032's wider `last_solve`, `target` or `none` would. A radius written in degrees would be sized to one telescope (§ Mount-neutral by design, 3). The cost of the order is in § Risks: while no aim point is measured, 032 gives a live pointing 3°. The first draft's order is spec owner decision 26's other answer. |
| Misses are counted at the solve alone, and three set the stack's own hint aside until a sub solves | Counting the focus gate's `too_few_stars`; using 032's hint once and going back | A sub that solved but has too few stars in the focus region is where the stack thinks it is; widening the search for it helps nothing. Going back to a narrow hint that has just missed three times, before anything has solved, misses again (spec FR-025, US6-S5). |
| The settings test is a pure function over `SubSettings`, compared with the reference sub | Comparing two `FrameView`s inside the worker; comparing with the sub before | Every clause then has a worked example (spec US6-S1): a sub's exposure is the console's setting, which no test can change while the stack holds the lease, and `FrameView` is non-exhaustive, so a test cannot build one. Comparing with the reference sub keeps the stack to one set of settings, which is what 036's single decision of the exposure rests on. |
| The page's picture is reduced straight from the sums and weights, under the pixels' lock for that pass alone | `Stack::mean()` and then a reduction; the lock held for the whole picture | A mean of the full size is 195 MB at the worked example's size (`docs/research/2026-11-15-imaging-camera.md` § 8. Live stacking) and would be made after almost every sub, since the queue is usually empty. The reduction reads the same planes once and keeps about 22 MB. Holding the lock through `background`, `tone` and the JPEG would keep `GET /api/imaging/stack/preview.jpg` waiting for all of it. |
| The gate takes each sub's focus reading through 033's `focus::read`, in the kept focus region | The solver port's `measure` over the whole sub; our own star measure on the colour picture; 033's verdict against the focus record | The glossary's focus reading is taken in the focus region, and 033's plan writes `read` for this spec to call. A measure of our own needs a detector for 16-bit data. The record's verdict is the focus check of a run, which 037 owns; a stack compares its own subs (spec owner decision 8). |
| The stack holds 031's lease from an accepted start until it has ended | Asking `Calibration::busy` and 033's `focus_running`, and waiting out a frame another party slips in; a lock of the stack's own | The plan document's amendment 1: one holder, named, on the imaging state, and no lock in a later spec. It also removes the retry loop the first draft needed (spec owner decision 21). |
| A dropped sub stays on disk and is listed | Deleting it | Every sub is kept (the plan document's What is kept row); another stacker may want it. |
| `runs/<id>/` holds the stack and a list; subs stay under `frames/` | Saving subs into `runs/<id>/subs/` | 031's `take` always saves under `frames/<UTC date>/` and its tests pin that and the id rule; `stack.json` ties the two together with no change to 031 (spec owner decision 13). |
| `stack.json`, not `run.json` | Writing `run.json` | The plan document gives `run.json` to the imaging run of 037. |
| Saved once, at the end | A copy every so many subs | The subs are the record; a copy of 195 MB at the worked example's size every few minutes stalls the worker (spec owner decision 14). |
| A FITS cube of three planes, written by hand | Three files; a FITS crate | One image, twenty cards and big-endian reals need no library; 031 and 034 write theirs the same way. |
| Celestial cards from the reference solve | None | They cost eight lines and let any astronomy program place the stack. |
| The display's steps are pure functions with a worked example each | One `render` | A percentile of a stretched picture cannot be checked by hand; each stage can. |
| One span for the three channels | A span a channel | A span a channel would undo the gains and make every picture grey. |
| The picture's name counts pictures | It counts subs | Changing the gains makes a new picture of the same subs, and the page must fetch it. |
| The stack's view rides in the imaging view | A route of its own; an event stream | The page already reads `GET /api/imaging` once a second (031). |
| `Stacker::stop` waits for the files | Answering at once | A client that stops a stack wants to know where it is. |
| Options and the display's gains on the start route | A settings route | The plan document gives 035 three routes for a stack, the start route taking the options and, on a running stack, the gains and stretch (spec owner decision 18). |
| `took_ms` from `std::time::Instant` | The console's clock | The console's clock is virtual in tests and under `--simulate`; the figure is for the bring-up probe and nothing depends on it. |
| One blocking thread for the pixel pass | Row bands on scoped threads | In the benchmark four threads take the total to about 0.7 of one thread's (`docs/research/2026-11-15-imaging-camera.md` § 8. Live stacking), a saving of about three tenths; the probe says whether it is needed (spec owner decision 25). |
| A scene in a cell of its own in the simulator | Fields on 034's `LightPath` | 034's tests build `LightPath` by literal. |

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

  Every story runs both, and the judge sees their results. nextest runs every test as
  its own process and ends one after five minutes (`.config/nextest.toml:23`), on a
  test build at opt-level 1 (`Cargo.toml:37`). The stories' development checks run one
  finder module with `cargo nextest run -p finder --locked --test <binary> <module>::`.
- **CI in the merge queue** runs every `ci.yml` job, including `cargo-deny`,
  `cargo-machete`, the build on Rust 1.96 and the finder-page checks. No story adds a
  dependency, so none of them has anything new to refuse.

## Prerequisites before this spec is flipped to `ready`

1. **The earlier specs have landed.** Not met. `030-imaging-camera-link`, `031-imaging-frames-on-console`,
   `032-imaging-solve-and-aim-point`, `033-focus-reading-and-aid` and
   `034-dark-and-flat-frames` are drafts in the drafting worktree, and no story of any
   of them is on `main`. All five are in this spec's `depends_on_landed`.
2. **The trio is re-anchored against `main`.** Not met. Every `path:line` was read at
   49bb210, the drafting worktree's HEAD, and the validator resolved them there. After
   030 to 034 land: every interface of theirs named in § Interfaces is read in their
   code and corrected here where it differs (the list is spec.md's second and third
   Clarifications sessions); every file they create is cited with a line; the story
   that last wrote each shared file is put right in § Refinement Record; § Shapes that
   existing tests pin is checked against their test files, the ones that read the
   Imaging page among them; and the validator is re-run on `main`.
3. **The owner decisions are settled and the approvals given** (spec § Owner decisions,
   § Owner approvals). Not met: all twenty-six decisions are open. Any answer other
   than the recommended one is written into the trio and re-validated before the flip.
4. **The constitution covers an imaging solve that is not our own Rust.** Not met, and
   032's to meet: principle V says "The solver is our own Rust", the stack registers
   every sub by ASTAP's solve, and the plan document's § Before any flip asks for the
   maintainer's amendment or written reading. This spec changes nothing there.
5. **033 has settled, and this trio is read against what it becomes.** Met in the
   drafts; not met on `main`. 033's trio was being revised the evening this one was
   finished, and was read here as it stood then; it changed under this trio once
   already. Its review and the series audit have been applied to it in the drafting
   worktree, and the three things of it used here were read again there and agree:
   `focus::read` with its `FocusReading` and
   `Unmeasurable` (§ The focus gate); the `region` of its `FocusView`, in sensor
   pixels; and, in spec US6-S6 alone, its `POST` and `DELETE /api/imaging/focus` with a
   `tile` and its `focus.aid.readings`, on a console whose solver is the tests' own.
   They are read again after it lands. If
   any differs, the gate, FR-022 and US6-S2 and US6-S6 are corrected before the flip.
6. **A trial of another stacking engine, if the owner wants one.** The owner's option,
   not a condition: the plan document's § Before any flip lists it. `seiza-stacking`
   and AstraLite's stacking modules exist (§ Decisions); this spec is written as our
   own code. If the owner asks for a trial of either on real subs and prefers the
   result, this trio is redrafted before the flip, and for `seiza-stacking` the
   CC-BY-4.0 in its dependency's licence expression is a `deny.toml` decision first.
7. **The documents this trio cites are on `main`.** Not met: the plan document,
   ADR-0002, the changed `GLOSSARY.md` and
   `docs/research/2026-11-15-imaging-camera.md` are in the drafting worktree,
   uncommitted. US5 inserts its entry after the **Stack** entry, which only the rewrite
   has.
8. **031 carries the camera's lease and the colour hold, and the series is re-anchored
   to them.** Met in the drafts by the series audit's findings on 031; not met on
   `main`. The plan document's amendment 1 gives 031 one lease naming its holder; the
   audit's finding on 031 gives it to 031's trio as US6, in the shape
   `specs/033-focus-reading-and-aid/plan.md` § The camera lease writes down, which
   § The camera's lease cites name for name; and its finding A6 takes the hold under an
   imaging picture out of 031's page as `useColourHold`, which § The page writes down as
   this trio uses it. When this trio was revised after the audit, 031's trio in the
   drafting worktree carried both: US6 and its plan.md § The camera lease, whose
   refusal reads `the imaging camera is held by {holder}`, and the hook in US4,
   `crates/finder/web/src/useColourHold.ts`, in its plan.md § The page. 031's revision
   is done, and both were read again in its trio and agree with § The camera's lease
   and § The page; they are read again in 031's code after it lands: every name; the
   six points
   of § The camera's lease, above all that the lease is checked before the frame in
   flight, that a single imaging frame holds no lease, that `Imaging::abort` still ends
   the holder's frame, and what the refusal's words are; and the hook's file, name and
   return shape. 034's part is prerequisite 13. If 031 lands without a lease, spec owner
   decision 21's other answer is written into this trio instead; if it lands without
   the hook, `StackBlock` is written against whatever 031's page then holds colour
   with, and FR-042's last clause is corrected with it.
9. **The trio is probe-built.** Not met. No story has been compiled. Before the flip
   each of the ten stories is built from this trio in a throwaway worktree on top of
   030 to 034, tests first, with both gates; each diff is measured as plan trap 3
   says; and what the probes find is folded back in. The probe also says whether a
   `pub(crate)` field that US5 leaves for US6 passes `clippy -D warnings` as § Test
   rigs expects. US3 and US5 are built first: US3 because its two simulator scenarios
   carry the spec's accuracy claims, US5 because it is the largest and holds the rigs.
   The first things to prove are in § Risks.
10. **The plan document agrees with the settled decisions, and 036 and 037 with this
    trio.** Three parts.
    - *The plan document.* Met in the drafts for the recommended answers; not met on
      `main`, where the plan document is not yet (prerequisite 7). This trio departed
      from its § Interfaces shared across the specs in three places (spec owner
      decisions 13, 18 and 19), and that section now gives their recommended answers:
      `stack.rs` with `stack/service.rs` and `stack/display.rs`;
      `runs/<UTC start>-<target>/` holding `stack.json`, `stack.fits` and `stack.png`,
      every sub left under `frames/` and no `subs/` folder; and three routes for 035,
      the start route taking the options. If the owner settles any of them otherwise, that
      section is corrected in the same change. The order of a hint's sources is not
      among them: this trio keeps the plan document's order. If the owner picks spec
      owner decision 26's other answer, that is a departure, and § Mount-neutral by
      design, 4 is corrected with it.
    - *036.* Met as drafted. 036's plan lists what it assumes of this trio under its
      § Prerequisites 3, **035 is read as landed**, and its § What 035 must provide,
      and is re-anchored to this trio, not the reverse. It sets a stack's exposure
      with the taker's `Lease::set`, in the gap before the first exposure, and puts
      the observer's back through the lease where the taker returns, before the lease
      is dropped (036 plan.md § The stack), as § Shapes that existing tests pin, **The
      seam for 036 and 037**, asks. 034's `Calibration::restore_exposure`, which 036's
      earlier draft capped the exposure with, is gone (034 A1). 036's waiting while
      the tracker is not steady needs nothing more of this trio than that gap and the
      stopping mark, and its `steering.accepted` in the worker's step 8 comes after
      this trio's `note_solve` (the seam, 4). A sub taken at an exposure that changes
      within a stack is not offered.
    - *037.* Met as drafted. 037's plan already takes this trio as it is: `Stacker::start`
      names its own folder, every sub is kept and listed in `stack.json`, and after
      the stop 037 removes from disk the subs that list names when they are not to be
      kept (037 plan.md § Decisions and § Prerequisites 2); and its `run::label` is the
      one line `crate::imaging::stack::service::label(name)` (037 A2). Nothing of this
      trio is open for 037 but the re-anchoring of prerequisite 2.
11. **No operator-staged file is needed.** Met.
12. **Outside the factory, before the first use at the telescope:** the console rebuilt
    from `main` after US10; GoTo and Track proven on the real mount (the plan
    document's § Before any flip, 4); and the field checks of spec.md § Assumptions.
    Not needed to flip.
13. **034 takes the lease for a set of darks or flats.** Met in the drafts by the series
    audit's finding on 034 (A1), which 034's plan.md now carries (its `HOLDER`, and
    `Calibration::busy` and `restore_exposure` gone); not met on `main`. A set of darks,
    and each step of a set of flats, holds 031's lease under the name
    `a set of darks or flats`, so a set
    asked for while a stack runs is refused at its start with `the stack`, and a stack
    asked for while a set runs is refused with the set's name; and 034's
    `Calibration::restore_exposure` is gone. Spec § Edge Cases rests on both. They are
    read again in 034's code after it lands (prerequisite 2).
14. **032 offers `Imaging::note_solve`.** Met in the drafts: the series audit's finding
    on 032 (A3) put it in 032's US4, and 032's plan.md § Solving on the console gives it
    with the signature the worker's step 8 calls (032's spec FR-037, US4-S7); not met on
    `main`. It is read again in 032's code after it lands: its name and arguments, that
    it is not `async`, that it takes an aim reading for a whole-sensor frame only and
    only when a live pointing shares the frame's moment, and that it touches neither the
    page's `solve`, the kept scale nor the misses. Spec US5-S9 rests on it. If 032 lands without it, spec owner decision 20's first
    other answer is written into this trio instead: FR-015's last sentence, the
    worker's call and US5-S9 go.

## Plan traps

Lessons from building 012 to 028 on this factory, and the traps this design adds, each
with the check that keeps it from recurring. They bind every story.

1. **Gate evidence or nothing.** The judge credits only gate results and the story's
   diff. *Check:* every Then clause names a committed test that the `test` gate runs.
2. **A named command is read literally.** *Check:* commands are copied from
   § Verification.
3. **One evidence budget.** *Check:* each story keeps its diff under 64 KiB, measured
   with `git diff <base> -- . ':(exclude)crates/finder/web/dist' | wc -c`.
4. **The sandbox has only the gates' tools.** *Check:* no task runs a tool outside them.
5. **A spec fixed mid-epic needs a relaunch.** *Check:* no builder edits
   `specs/035-live-stack/` or `.specify/`.
6. **An oracle copied from the code proves nothing.** *Check:* a test reads a FITS file
   with its own reader, parses `stack.json` and sidecars with `serde_json`, decodes
   pictures with `image`, projects sky places by hand as spec.md § User Scenarios
   gives it, and never calls `sky_at`, `pixel_of`, `between` or `display` to make an
   expected value.
7. **Half a pixel.** A pixel's centre is at (x + 0.5, y + 0.5), in the simulator, in
   032's solves and here. A map applied to (x, y) instead looks right for a small
   turn and is 1 px off for a half turn. *Check:* spec US3-S3's half turn and quarter
   turn are exact, and US3-S4 holds a half-turned frame to 0.2 px.
8. **North turns between two centres.** A map made as one solve's matrix times the
   other's inverse ignores it. *Check:* spec US2-S5, at declination 60°, fails such a
   map by 0.3 px against a by-hand projection.
9. **A variance from a sum of squares.** In 32-bit reals the sum of squares of a
   thousand values near 30000 is good to one part in ten million of 10¹², which is
   more than the variance it is meant to hold. *Check:* the stack keeps the running
   sum of squared deviations (§ Rejection), and spec US4-S3's first case needs a
   deviation right to 5%.
10. **Rejection in separate passes.** Written as whole-array steps it cost nine times
    the rest of the stack in the benchmark. *Check:* `add` is the one loop of
    § Interfaces, and no `Vec` the size of a picture is made inside it.
11. **A not-a-number that spreads.** Multiplying a neighbour by a share of 0 still
    gives not-a-number. *Check:* a pixel whose share is exactly 0 is not read, and spec
    US3-S3 counts one rejected pixel, not four.
12. **The learning set is counted at each pixel.** A corner that the tenth sub is the
    first to reach must not be tested against nothing. *Check:* the test reads the
    pixel's own weight (§ Rejection), and spec US4-S3 adds a wild eighth sub.
13. **The view's lock and the pixels' lock.** A page read that waits for a pixel pass
    stalls the session; a lock held while `Imaging` is called deadlocks, because
    `Imaging::view` calls `Stacker::view`. *Check:* `view()` takes only the counts'
    lock; neither of those two locks is held across an await or a call into `Imaging`;
    the pictures' own async lock is held across the picture's awaits and across
    nothing else, and no call into `Imaging` is made under it; and spec US8-S4 reads
    the view through both routes during a stack.
14. **Work on a runtime thread.** A pixel pass on the runtime stalls the heartbeat's
    task, and in a paused-time test lets virtual seconds pass that the blocking pool
    would have held. *Check:* every pixel pass and file write is in `spawn_blocking`;
    spec US5-S1 expects no sub dropped, which a stack that fell behind in virtual time
    would not give.
15. **A count that depends on which task ran first.** *Check:* no scenario names which
    sub was dropped as `behind` or `unprocessed`; they assert sums, bounds, and that
    `taken` equals `stacked` plus the dropped.
16. **Stopping waits for a solver, or for an exposure.** *Check:* the worker drops a
    solve in flight when the stack stops, the taker aborts a frame it started after
    the stop's own `abort` had run, and spec US5-S5 holds `stop()` to one second
    against a solver that takes thirty.
17. **Two endings.** The taker and `stop()` can both try to end a stack. *Check:* ending
    is done once, by whoever asks first, and spec US5-S6 ends a stack by an abort.
18. **A picture the browser does not fetch.** *Check:* the picture's name counts
    pictures, and spec US7-S5 sees it rise when only the gains change.
19. **The display changing what is saved.** *Check:* `display` takes the stack, or its
    sums and weights, by shared reference and returns pictures of its own; spec US8-S1
    reads the stand-in's own values out of `stack.fits` after a stack that was shown.
20. **A card too long for its columns.** *Check:* the celestial numbers are rounded
    (§ FITS), and spec US8-S1 reads them back within 10⁻¹².
21. **A stack's folder name that clashes.** *Check:* `-2` is added, and spec US8-S3
    starts two stacks in one second.
22. **`deny_unknown_fields` and the MCP tool's `on`.** `StartRequest` refuses `on`.
    *Check:* `set_stacking` removes `on` before it parses the rest, and spec US10-S2
    starts a stack with `{"on":true}`.
23. **027's layout.** *Check:* each new test is a module file in its binary's folder
    with one alphabetical `mod` line; module names hold no digit; no file is added to
    `crates/finder/tests/support`; a helper nobody calls is not written.
24. **The bundle must be rebuilt.** *Check:* US9 commits `crates/finder/web/dist` after
    `npm run build`.
25. **Each worktree builds on its own.** *Check:* no story sets `CARGO_TARGET_DIR`, and
    test folders come from `crate::support::Scratch`.
26. **A lease that outlives its stack.** A stack that ends by itself, or whose folder
    cannot be made, and keeps the camera would lock every other taker out until the
    console restarts. *Check:* the taker owns the `Lease`, so it is dropped on every
    return of the taker, and `start` drops it where it fails after taking it; spec
    US5-S4 takes a frame after a stack that ended as `camera`, and US5-S8 after one
    that was stopped.
27. **A second lock, or a look that hides the holder.** Asking `Calibration::busy` or
    `focus_running` before the lease would be a lock of this spec's own, and would
    refuse with words that do not name the holder; so would a look at the view's frame
    while somebody holds the lease, because a holder's frame is exposing almost all the
    time. *Check:* `start` looks at the view's frame only while `imaging.holder()` is
    `None`, as 033's `focus_start` check 4 does, then asks the lease, and asks no other
    part (§ The stack on the console, **`start`**, steps 5 and 6); spec US5-S2 reads the
    holder's name in the refusal while the holder's own frame is exposing, and
    `an exposure is in progress` only with `holder()` `None`.
28. **A number of degrees.** A hint radius or an off-field limit written in degrees is
    sized to one telescope. *Check:* the radius of the stack's own hint is the sub's
    diagonal times its solved scale, `judge` works in shares of the sub's size, and
    spec US5-S1 reads a radius of 0.048° on a sub 80 px across its diagonal.
29. **A mean of the full size for every picture.** `Stack::mean()` and then a
    reduction makes three planes of the camera's size after almost every sub. *Check:*
    the page's picture comes from `display::reduce` over `stack.sum()` and
    `stack.weight()`; `Stack::mean` is called only where `stack.fits` is written and
    by tests; and spec US7-S1's last case, 20.0 and not 26.667, fails a reduction made
    from the pixels' means.
30. **The stack's own hint ahead of a live pointing.** It is the narrower hint and the
    first draft used it first, against the plan document's order. *Check:* the
    worker's step 3 asks `imaging.hint` first and keeps its answer when the source is
    `pointing`; spec US5-S7 reads `pointing` in the solve sidecars of a stack on a rig
    with a live pointing, and US5-S1 reads `last_solve` on a rig with none.
31. **A miss counted outside the solve.** A focus gate's `too_few_stars` that widens
    the next hint sends the solver searching for a sub that was just solved. *Check:*
    `misses` changes in the worker's step 4 and nowhere else; spec US6-S5's second case
    has two solve misses and one such sub between them and still reads the stack's own
    hint.
32. **An exposure set behind the lease's back.** `Imaging::set`, and anything built on
    it, is refused while the stack holds the camera. *Check:* nothing in this spec
    changes a setting; § Shapes that existing tests pin tells 036 to go through the
    taker's lease, which 036's plan does (§ Prerequisites 10).
33. **A linear map trusted on a wide field.** The five-point map strays from the two
    solves by the shift times the square of the field's half-size, so a far sub on a
    field of a few degrees lands more than a pixel off at its corners although `judge`
    passes it. *Check:* `between` maps the four corners through the sky and answers
    `None` beyond `CORNER_SLACK_PX`; spec US2-S7 reads `None` for a sub 0.747° away on
    a 3° × 2° field and `Some`, between 0.2 and 0.3 px off at the corners, for one
    0.249° away, and US2-S3's same-centre maps still pass.
34. **A sub's solve that does more than measure the aim point.** Handing a sub's solve
    to 032's own task, or to `Imaging::solve`, would show it as the page's solve and
    keep its scale, which narrows every later hint on one sub's word; handing it over
    under a lock deadlocks, because `note_solve` is a call into `Imaging`. *Check:* the
    worker calls `note_solve` and nothing else of 032's state, once, in step 8, with no
    lock held, for stacked subs alone; spec US5-S9 reads the aim point measured from
    five readings with no `solve` in the view and a hint radius still 3°.

## Risks

| Risk | Mitigation |
|---|---|
| ASTAP's solves of one field do not agree to a fraction of a pixel, so stars are soft or doubled in the stack. `docs/research/2026-11-15-imaging-camera.md` § 8. Live stacking and § 13. Not established have no figure. | The first field check: star sizes in the stack against those in one sub. If they do not agree, a refinement of each sub's place on star positions is a later spec; `Placement` is where it would go. |
| The stack takes longer than a sub at the camera's full resolution. No timing with rejection exists; the benchmark without it gives shares only. | The camera never waits, the queue is bounded and `behind` counts what was lost; `took_ms` is in the view for the probe; forwarding binned 2 × 2 quarters the work; row bands on scoped threads are the next step (spec owner decision 25). |
| Memory. At the worked example's size the stack is 455 MB and the sub in hand 260 MB more; the page's picture adds a reduction of about 22 MB and its working copies, not a mean of 195 MB; and when the stack is saved, after the last sub has left, the mean and the bytes of `stack.fits` are 195 MB each, then the full-size picture for `stack.png` about as much again (inferred, `docs/research/2026-11-15-imaging-camera.md` § 8. Live stacking). | Stated in the README; binning 2 × 2 quarters it; no resampled copy is held (§ Plan traps 10) and no mean is made for a picture (§ Plan traps 29). A refusal by size is not built. |
| While no scale is kept, 032 gives a live pointing's hint 3°, and before the aim point is measured its place may be a degree from the sensor's field. The first stacked sub with a live pointing measures the aim point through `note_solve`, but a sub keeps no scale, so under the plan document's order that 3° hint is used for every sub, each solve searches further than it would from the stack's own hint, and a sub that cannot solve searches the whole 3°. | `took_ms` shows it and the queue is bounded; solving one imaging frame from the page first keeps the scale and measures the aim point (032), after which the hint is the sensor's diagonal; 037 measures it before it stacks. Spec owner decision 26's other answer puts the stack's own hint first, as a departure. The field check times a solve both ways. |
| An aim reading from a sub is wrong, and the aim point moves mid-stack. | It changes nothing in the stack, whose placements come from the imaging solves alone; 032's window of five sets an outlier aside and replaces the aim point only on three that agree (032 plan.md § The relation). The field check reads whether the aim point the subs measure stays steady through a stack. |
| A wide field's far sub is dropped as `off_field` by the corner check, though its centre is inside `judge`'s limit. | Only on a field of a few degrees with a sub far from the reference; never at the worked example's size, where the map strays 0.10 px at most (spec US2-S7). The count says why; spec owner decision 3's other answer, the exact mapping, would keep it. |
| ASTAP's star measure does not work on a raw colour mosaic, or costs a second run of the program for each sub. Neither is established (`docs/research/2026-11-15-imaging-camera.md` § 13. Not established). | `focus_gate: false` turns it off; the field check says which. |
| A sub whose stars are trailed still solves and is stacked. No gate on trailing is built; other stackers have one (`docs/research/2026-11-15-imaging-camera.md` § 8. Live stacking). | The focus gate drops a sub whose stars measure 30% larger than the others', which a clear trail does; a short trail gets through. The field check reads what gets through on a real night; a gate of its own would be a later spec. |
| A stack's first sub is a poor one, and every later sub is placed on it. | It is one sub among many in the mean; the observer stops and starts again. Choosing the reference sub by quality is not built. |
| A streak inside the first eight subs is kept. | It is an eighth of its strength at once and fades with the stack; said in the README and spec § Edge Cases. |
| Stars' cores vary with seeing and are rejected too often, dimming stars. | The deviation is learned at each pixel, so a core's own spread widens its own limit; the field check compares a stack with `reject` false. |
| The background's plane is pulled by a nebula that fills most tiles. | Only the darker half of the tiles is fitted; what is saved is untouched; the look is for the design review (spec owner decision 16). |
| The picture looks green before the observer sets gains. | Expected without a colour matrix (the plan document's Colour row); the gains are on the page; measured gains are spec owner decision 17's alternative. |
| 031 lands the lease under other names or with other rules: the frame in flight checked before the lease, a single imaging frame that holds the lease, a foreign `abort` refused, a refusal that does not name the holder. | § The camera's lease cites 031 US6 and lists the six points and where each is used, and 033's, 034's and 037's trios use the same shape; § Prerequisites 8 reads each in 031's plan.md and code before the flip. Only `Stacker::start`, the taker and three scenarios (spec US5-S2, US5-S4, US5-S8) touch it, and US5-S2 asserts `holder()` `None` during a single frame, so point 6 shows up there if it changes. |
| 031 lands with no lease at all. | Spec owner decision 21's other answer, which is the first draft's design: ask the two busy checks and wait out a foreign frame, three times at most. |
| US5's diff nears 64 KiB: the loop, the rigs and nine scenarios. | It is built second in the probe round (§ Prerequisites 9). If it passes the budget, US5-S7 and US5-S9, the two on the sim stacking rig, move to US6, whose diff is the smallest of the console stories; the call of `note_solve` stays in US5's step 8, which is one line. |
| US8's diff nears 64 KiB: three routes, the view, the cube, three files, the README and eight tests. | Measured in the probe round; the README's section can move to US10, which already writes that file. |
| A scenario's numbers rest on the simulator's noise, its seeded field or its star shapes and have not been run (US3-S4, US3-S5, US4-S4). | Each was worked from 030's constants and is given with its margin in spec.md; the probe build of US3 and US4 runs them, and a tolerance that proves wrong is corrected there, never by a story. |
| 033 changes `focus::read`, its focus view or its aid before it lands. | § Prerequisites 5. The gate touches 033 in two places, `read` and `focus.region`, and one scenario (spec US6-S6) drives its routes. |
| The focus region holds too few stars on a sparse field, so every sub is dropped as `too_few_stars`. | `focus_gate: false` turns the measure off, and the count says why the stack is empty; the field check reads what the gate drops on a real night. |
| 036 and 037 need more from the loop than this spec offers. | § Shapes that existing tests pin, **The seam for 036 and 037**, names the gap before an exposure, the stopping mark, the lease as the only way to a setting, what the settings test compares, and the worker's two places; `StackView` is non-exhaustive. Both trios are re-anchored to this one (§ Prerequisites 10). |

## Refinement Record

Drafted on 2026-11-15 by the grilling session against `main` at 49bb210, from
`docs/plans/2026-11-15-specs-030-037-imaging.md`,
`docs/research/2026-11-15-imaging-camera.md` § 7. Field rotation, sub length and the
drive, § 8. Live stacking, § 9. Calibration frames, § 10. Exposing the Orion Nebula,
and colour and § 12. Prior art and what was not adopted, and the trios of
`030-imaging-camera-link`, `031-imaging-frames-on-console`, `032-imaging-solve-and-aim-point`,
`033-focus-reading-and-aid` and `034-dark-and-flat-frames` as they stood in the drafting
worktree that day. A first drafter wrote spec.md and most of this plan against dbc2b5b
and was cut off before the record below and before tasks.md; a second finished the trio
against 49bb210, with the plan document's § Mount-neutral by design and its amendments
in hand, and with 030, 031, 032 and 034 as their review left them; a reviewer's findings
were then applied to the three files (below). **No probe build has been run.** No
story of this trio has been compiled, linted or tested, no interface above has been
built, and no diff has been measured: the session wrote documents only. This spec
also builds on code that does not exist: every type and function of 030 to 034
named here is its draft's, not something that was read in a file.

What the second pass changed, so that a reader of an earlier copy knows: the stack takes
031's lease for its whole length, as `the stack`, and asks no busy check, so the wait
for a frame another party slipped in is gone, and with it one constant; the focus gate
takes the glossary's focus reading through 033's `focus::read`, in the kept focus
region, and 033 joined `depends_on_landed`; the hint's radius is the sub's diagonal on the sky, no
longer held to half a degree; 032's `SolveError` variants and `pixel_of` are written as
its review left them; the test of every MCP tool is cited at its new line; two
scenarios were added (US5-S8, US6-S6) and five changed (US5-S1, US5-S2, US5-S4, US6-S2
and US6-S5, in their hint radius, their refusals or their cases); the Work Graph gained
the direct merge edges the record's shared writes need (US8 on US2 and US5, US10 on
US8); and seven scenarios lost a blank line inside their numbered item, which had cut
every later scenario of five stories out of what the judge is shown.

What the review changed. A second agent read the finished trio the same day and gave
eighteen findings. All eighteen were applied; three points inside two of them were
settled otherwise than the reviewer proposed, and are named at the end of this
passage. No scenario was added or removed, so the Work Graph and the list of tests
below keep their shape; the changed scenarios are US5-S1, US5-S2, US5-S7, US6-S1,
US6-S5, US7-S1, US7-S2, US7-S4, US8-S2, US8-S4, US10-S2 and US10-S4. In substance:

- a sub's hint follows the plan document's order again: 032's hint first, used as it
  is when it comes from a live pointing, and the stack's own in place of 032's other
  three sources. The first draft's order, the stack's own hint first, is now spec owner
  decision 26's other answer. Misses are counted at the solve alone and set the
  stack's own hint aside until a sub solves (spec FR-016, FR-025);
- the page's picture is reduced straight from the sums and weights, under the pixels'
  lock for that pass alone and one picture at a time; `reduce` and `render` changed
  their arguments (§ The display);
- the seam with 036 is written down (§ Shapes that existing tests pin): the gap before
  an exposure, the stopping mark, the lease as the only way to a setting during a
  stack, and what the settings test compares. 036's exposure cap, as drafted, would be
  refused under the lease; that is prerequisite 10, and 034's missing lease is the new
  prerequisite 13;
- the settings test became the pure function `same_settings` over `SubSettings`, so
  that every clause of spec FR-021 has a worked example;
- US7-S2's green plane is asserted where 034's lower-median `percentile` puts it,
  199.99 and not 200;
- the order in which a request's and a tool's arguments are judged is written out,
  with the answer to `{}` (§ The routes, § The MCP tools);
- three figures misquoted from the research were corrected, every research citation
  names the note with the section's number and title, and the text `decode` gives for
  a cube was corrected.

The three points settled otherwise, each on evidence read in the drafts. The
reviewer held that a sub taken at 036's capped exposure would be dropped as
`settings`: 036 decides the exposure once, before the first sub, so the reference sub
has it too and nothing is dropped, and the seam says that instead. The same finding
cites 036's § Prerequisites 10 for the list of what 036 assumes of this trio: 036's
plan then had eight prerequisites, and the list is under its third, where it still is. And a drop for an
exposure that differs was asked for through the scripted camera: a sub's `exposure_ms`
is the console's setting and not the camera's word, and nothing can change it while the
stack holds the lease, so that clause is proved through `same_settings` instead. The
numbers the review added or changed were worked with a short script: the green plane's
199.99, the reduction's 20.0, and 0.04 px and 0.10 px for the linear map's error at
0.1° and 0.25°.

What the audit changed. An auditor compared the seven trios the series then had the same
night and gave six findings on this one; all six were applied, in all three files, and
none was declined. Two scenarios were added (US2-S7, US5-S9) and two changed (US5-S2,
US8-S3), and the shared setup's sim stacking rig now says why its hints keep a radius
of 3°; the Work Graph keeps its shape, because each new requirement falls inside an FR
its story already implements (FR-005 in US2, FR-013 and FR-015 in US5, FR-035 in US8,
FR-042 in US9). In substance:

- `start`'s step 5 is 033's `focus_start` check 4: it looks for a frame in flight only
  while `imaging.holder()` is `None`, so a stack asked for while the focus aid or a set
  of darks holds the camera hears the holder's name, even while the holder's frame is
  exposing; US5-S2 gains that case and asserts `holder()` `None` during a single frame.
  The sentence that called steps 3 to 6 atomic was wrong on a multi-threaded runtime
  and now says that the lease is what makes two starts exclusive, as 033's plan says;
- a stacked sub's solve is handed to 032's `Imaging::note_solve` (032 A3) in the
  worker's step 8, after the counts, so the aim point stays measured through a stack;
  the page's `solve` and the kept scale still do not change, and US5-S9 proves all
  three on the sim stacking rig. Owner decision 20 now recommends that answer;
- the shared write of `crates/finder/tests/cli/main.rs` names 033's US8, its last
  writer after 033's split;
- § The camera's lease cites 031 US6 instead of standing in for it, adds the sixth
  point (a single imaging frame holds no lease), and § Shapes that existing tests pin,
  § Prerequisites 10 and 13 and § Risks no longer describe 036's earlier cap through
  `restore_exposure`, which 034 A1 removes and 036 replaces with `Lease::set`;
- the rule for a target's name in a folder is `stack::service::label`, which
  `target_label` and 037's `run::label` call, and `StackBlock` holds colour through
  031's `useColourHold` (031 A6) rather than a second hold of its own;
- `between` checks its map at the reference grid's four corners and answers `None`
  beyond 0.5 px, the worker counting such a sub `off_field`; owner decision 3 says why,
  and US2-S7 works it at a 3° × 2° field. The audit's other answer, the exact mapping,
  stays that decision's alternative.

The corner figures of US2-S7, and the 0.10 px and 1.17 px of § The arithmetic, were
worked with a short script that projects the corners through the five-point map and
through the two solves, written for the purpose from the shared setup's by-hand
projection. Every `path:line` in spec.md and plan.md was read again at 49bb210 in this
pass, the worktree's HEAD, and none had moved; the one added,
`crates/simulator/src/optics.rs:26`, was read there too.

Every `path:line` names a file that exists at 49bb210 and was read there, but for the
lines of `GLOSSARY.md`, which are lines of the plan document's uncommitted rewrite of
that file (§ Prerequisites 7). Every number in a scenario was worked by hand or with a
short script, and is shown as a worked example where it is not obvious; the placements
of US2-S3 to US2-S5 and the two centres of US6-S3 were worked again by the second pass
with a projection written for the purpose, and agreed. The numbers that rest on the
simulator's rendering (the star centroids and coverage of US3-S4, the signal and noise
of US3-S5, the streak's means and rejected counts of US4-S4, the drift of US5-S7) were
worked from 030's constants and have not been run against the simulated camera, which
is not built either. One fact about the tests' runtime was checked in tokio's own
documentation: while a blocking task runs, a paused clock does not advance.

Some things here rest on less than a reviewed plan.md, and each is named where it is
used. Four come from the series audit's findings on other trios, which were being
written into those trios while this one was revised: the camera's lease (031 US6),
cited in § The camera's lease with the six points relied on, and `useColourHold` (031
US4, from the finding A6), quoted in § The page, both as 031's plan.md gives them when
this trio was checked; `Imaging::note_solve` (032 US4, from the
finding A3), whose signature 032's plan.md gives as § The stack on the console writes
it; and a set of darks or flats under the lease (034 A1), which 034's plan.md carries.
§ Prerequisites 8, 13 and 14 say what must happen before the flip.
033's trio was being revised while this one was finished, and its plan.md changed once
under it: § Prerequisites 5 lists the three things of it that are used.

What a first refinement round must do is § Prerequisites 2, 5, 8, 9 and 14, and its findings
replace these paragraphs. Until then the record below is the draft's own account: its
tests are the scenarios, its writes are the files § Project Structure names, and its
shared writes name the next story of this spec to write the same file, or, for the
first story of this spec to write it, the story that, by the drafts of 030 to 034,
last writes the file before this spec does. A test file that 030 to 034 or an earlier
story of this spec creates cannot be listed under `tests` as existing, because the
validator resolves every such path in the drafting worktree; each is named under
`preserved_behavior` with what it must still assert, and moves to `tests` at the
re-anchoring. The record's `open_questions` are empty because the validator refuses a
record that holds any; what is not established is in § Risks and § Prerequisites, and
in spec.md's owner decisions 8, 13, 20, 21 and 26. Nothing in this record is an owner
approval: those are in spec.md, and flipping to `ready` gives them.

```yaml
version: 1
reviewed_revision: 49bb210fe4860dd98ba24727074e7bd3cc0d7124
stories:
  US1:
    tests:
      - path: crates/simulator/tests/imaging_scene.rs
        status: new
        disposition: add
        assertion: "the default scene is empty, and a frame taken after setting the default scene is byte for byte a frame taken without, every sample 165"
        decision: "spec.md:US1-S1"
      - path: crates/simulator/tests/imaging_scene.rs
        status: new
        disposition: add
        assertion: "a satellite's streak adds its electrons along the segment by distance on its frame alone, whatever the filter, and nothing beyond six pixels"
        decision: "spec.md:US1-S2"
      - path: crates/simulator/tests/imaging_scene.rs
        status: new
        disposition: add
        assertion: "a source adds a Gaussian glow by its colour and the exposure at its place on the sky"
        decision: "spec.md:US1-S3"
      - path: crates/simulator/tests/imaging_scene.rs
        status: new
        disposition: add
        assertion: "the cover and the flat panel remove streak and glow, and vignetting dims them, because both are light"
        decision: "spec.md:US1-S4"
      - path: crates/simulator/tests/frame_positions.rs
        status: existing
        disposition: preserve
        assertion: "the finder camera's simulated frames are unchanged"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "with the default scene every simulated imaging frame is byte for byte what 034 makes (spec.md FR-001)"
      - "SensorPattern, LightPath, ImagingOptics, Pointing and the imaging truth gain nothing (plan.md Shapes that existing tests pin)"
      - "crates/simulator/tests/imaging.rs, a test file 030 creates, passes unchanged: 030's simulated imaging frames keep their exact samples (plan.md Shapes that existing tests pin)"
      - "crates/simulator/tests/imaging_calibration.rs, a test file 034 creates, passes unchanged: the sensor pattern and the light path act as before with an empty scene (plan.md Shapes that existing tests pin)"
    checks:
      development:
        - "cargo nextest run -p simulator --locked --test imaging_scene"
        - "cargo nextest run -p simulator --locked --test imaging_calibration"
        - "cargo nextest run -p simulator --locked --test imaging"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - crates/simulator/src/imaging.rs
      - crates/simulator/src/lib.rs
      - crates/simulator/src/optics.rs
      - crates/simulator/tests/imaging_scene.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/simulator/src/imaging.rs
        other_spec: 034-dark-and-flat-frames
        other_story: US1
        disposition: merge_order
        reason: "035 depends_on_landed 034, whose US1 last changed the simulated imaging camera"
      - path: crates/simulator/src/lib.rs
        other_spec: 034-dark-and-flat-frames
        other_story: US1
        disposition: merge_order
        reason: "035 depends_on_landed 034, whose US1 last changed the simulator's lib.rs"
      - path: crates/simulator/src/optics.rs
        other_spec: 034-dark-and-flat-frames
        other_story: US1
        disposition: merge_order
        reason: "035 depends_on_landed 034, whose US1 last changed optics.rs"
    open_questions: []
  US2:
    tests:
      - path: crates/finder/tests/console/stack_colour.rs
        status: new
        disposition: add
        assertion: "colour of a mosaic of flat colours is flat in every pixel, edges included, for all four patterns, and a mono or a one-pixel-wide mosaic gives three equal planes"
        decision: "spec.md:US2-S1"
      - path: crates/finder/tests/console/stack_colour.rs
        status: new
        disposition: add
        assertion: "one lit red or green sample spreads to its neighbours by the bilinear weights, at a corner too, and to no other plane"
        decision: "spec.md:US2-S2"
      - path: crates/finder/tests/console/stack_placement.rs
        status: new
        disposition: add
        assertion: "between gives the identity, a quarter turn, a half turn, a scale and a mirror with their exact maps, shifts, turn_deg and area_scale"
        decision: "spec.md:US2-S3"
      - path: crates/finder/tests/console/stack_placement.rs
        status: new
        disposition: add
        assertion: "a sub 0.01 degrees east is the identity map with a shift of 18 px, and centre_shift is the opposite"
        decision: "spec.md:US2-S4"
      - path: crates/finder/tests/console/stack_placement.rs
        status: new
        disposition: add
        assertion: "at declination 60 the map agrees with a by-hand projection to 0.01 px at five points and carries the turn of north between the two centres"
        decision: "spec.md:US2-S5"
      - path: crates/finder/tests/console/stack_placement.rs
        status: new
        disposition: add
        assertion: "between is None more than a quarter turn of the sky away, and centre_shift is None for a map that cannot be inverted"
        decision: "spec.md:US2-S6"
      - path: crates/finder/tests/console/stack_placement.rs
        status: new
        disposition: add
        assertion: "on a 3 by 2 degree field of 3600 by 2400 pixels the map of a sub 0.249 degrees away lies 0.2 to 0.3 px from a by-hand projection at every corner, and between is None for a sub 0.747 degrees away whose corners lie more than 0.5 px off"
        decision: "spec.md:US2-S7"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and each binary's main.rs holds only its support include and mod lines"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "nothing in 031's frame path, 032's solving or 034's calibration changes: this story adds pure functions, one constant and one pub mod line"
      - "no dependency is added and no Cargo.toml changes"
      - "crates/finder/tests/console/imaging_solve.rs, a test file 032 creates, passes unchanged: ImagingSolve, sky_at and pixel_of are read and not changed (plan.md Shapes that existing tests pin)"
      - "crates/finder/tests/console/calibration.rs, a test file 034 creates, passes unchanged: Mosaic and its colours are read and not changed (plan.md Shapes that existing tests pin)"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console stack_colour::"
        - "cargo nextest run -p finder --locked --test console stack_placement::"
        - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - crates/finder/src/imaging/mod.rs
      - crates/finder/src/imaging/stack.rs
      - crates/finder/tests/console/main.rs
      - crates/finder/tests/console/stack_colour.rs
      - crates/finder/tests/console/stack_placement.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/imaging/mod.rs
        other_spec: 034-dark-and-flat-frames
        other_story: US4
        disposition: merge_order
        reason: "035 depends_on_landed 034, whose US4 last changed the imaging module"
      - path: crates/finder/src/imaging/mod.rs
        other_spec: 035-live-stack
        other_story: US8
        disposition: merge_order
        reason: "US8 follows US2 on the merge chain, and adds the stack to the imaging view"
      - path: crates/finder/src/imaging/stack.rs
        other_spec: 035-live-stack
        other_story: US3
        disposition: merge_order
        reason: "US3 follows US2 on the merge chain, and adds Stack and its add"
      - path: crates/finder/tests/console/main.rs
        other_spec: 034-dark-and-flat-frames
        other_story: US7
        disposition: merge_order
        reason: "035 depends_on_landed 034, whose US7 last changed console/main.rs"
      - path: crates/finder/tests/console/main.rs
        other_spec: 035-live-stack
        other_story: US3
        disposition: merge_order
        reason: "US3 follows US2 on the merge chain, and adds mod stack_accumulate before US2's mod stack_colour"
    open_questions: []
  US3:
    tests:
      - path: crates/finder/tests/console/stack_accumulate.rs
        status: new
        disposition: add
        assertion: "adding a picture with the identity gives sums equal to it, weights of one and a mean equal to it, and twice gives weights of two and the same mean"
        decision: "spec.md:US3-S1"
      - path: crates/finder/tests/console/stack_accumulate.rs
        status: new
        disposition: add
        assertion: "a shifted sub covers only where its pixel centres reach, a half-pixel shift returns a linear ramp exactly, and the mean divides by each pixel's own weight"
        decision: "spec.md:US3-S2"
      - path: crates/finder/tests/console/stack_accumulate.rs
        status: new
        disposition: add
        assertion: "a half turn and a quarter turn land exactly, the edge slack admits a sample a millionth outside, and a pixel that is not a number is rejected alone"
        decision: "spec.md:US3-S3"
      - path: crates/finder/tests/console/stack_accumulate.rs
        status: new
        disposition: add
        assertion: "three simulated frames at three attitudes, a half turn among them, stack with each star's centroid within 0.2 px of its place in the first"
        decision: "spec.md:US3-S4"
      - path: crates/finder/tests/console/stack_accumulate.rs
        status: new
        disposition: add
        assertion: "sixteen simulated subs of a faint glow give twice the signal over noise of four, within 15 percent"
        decision: "spec.md:US3-S5"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and each binary's main.rs holds only its support include and mod lines"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "US2's colour and Placement keep their signatures and results"
      - "no copy of a resampled sub is held: add is one pass (plan.md Plan traps 10)"
      - "crates/finder/tests/console/stack_colour.rs and crates/finder/tests/console/stack_placement.rs, test files this spec's earlier story creates, pass unchanged (plan.md Shapes that existing tests pin)"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console stack_accumulate::"
        - "cargo nextest run -p finder --locked --test console stack_placement::"
        - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - crates/finder/src/imaging/stack.rs
      - crates/finder/tests/console/main.rs
      - crates/finder/tests/console/stack_accumulate.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/imaging/stack.rs
        other_spec: 035-live-stack
        other_story: US4
        disposition: merge_order
        reason: "US4 follows US3 on the merge chain, and adds rejection, levels and the background match"
      - path: crates/finder/tests/console/main.rs
        other_spec: 035-live-stack
        other_story: US4
        disposition: merge_order
        reason: "US4 follows US3 on the merge chain, and adds mod stack_reject"
    open_questions: []
  US4:
    tests:
      - path: crates/finder/tests/console/stack_reject.rs
        status: new
        disposition: add
        assertion: "levels is each plane's median on a grid of every kth pixel"
        decision: "spec.md:US4-S1"
      - path: crates/finder/tests/console/stack_reject.rs
        status: new
        disposition: add
        assertion: "a stack made with_background_match shifts each later sub to the first sub's levels and reports the offsets"
        decision: "spec.md:US4-S2"
      - path: crates/finder/tests/console/stack_reject.rs
        status: new
        disposition: add
        assertion: "after eight subs a pixel more than three deviations from its mean, with a floor of one count, is left out whole in every channel, and before eight nothing is tested"
        decision: "spec.md:US4-S3"
      - path: crates/finder/tests/console/stack_reject.rs
        status: new
        disposition: add
        assertion: "a simulated streak on one sub of twelve leaves the stack within half a count of a control stack along its line, while a plain mean keeps it"
        decision: "spec.md:US4-S4"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and each binary's main.rs holds only its support include and mod lines"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "Stack::new, add and Added keep their signatures, and a stack made with Stack::new alone neither matches backgrounds nor rejects"
      - "crates/finder/tests/console/stack_accumulate.rs, a test file this spec's earlier story creates, passes unchanged: US3's sums, weights and means (plan.md Shapes that existing tests pin)"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console stack_reject::"
        - "cargo nextest run -p finder --locked --test console stack_accumulate::"
        - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - crates/finder/src/imaging/stack.rs
      - crates/finder/tests/console/main.rs
      - crates/finder/tests/console/stack_reject.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/imaging/stack.rs
        other_spec: 035-live-stack
        other_story: US5
        disposition: merge_order
        reason: "US5 follows US4 on the merge chain, and adds pub mod service"
      - path: crates/finder/tests/console/main.rs
        other_spec: 035-live-stack
        other_story: US5
        disposition: merge_order
        reason: "US5 follows US4 on the merge chain, and adds mod stack_loop"
    open_questions: []
  US5:
    tests:
      - path: crates/finder/tests/console/stack_loop.rs
        status: new
        disposition: add
        assertion: "a started stack takes subs with the purpose sub, solves, calibrates and adds each, with the first as the reference sub, 032's hint for the first and, on a rig with no live pointing, its own hint from the second on, and a mean equal to the stand-in's ramp"
        decision: "spec.md:US5-S1"
      - path: crates/finder/tests/console/stack_loop.rs
        status: new
        disposition: add
        assertion: "start refuses bad gains and stretch, those before it looks at the camera, a console without a solver or a camera, a single frame in flight while nobody holds the lease, and a camera whose lease another holder has, naming that holder even while the holder's own frame is exposing; on a running stack it changes gains and stretch only; a console without imaging has no stacker"
        decision: "spec.md:US5-S2"
      - path: crates/finder/tests/console/stack_loop.rs
        status: new
        disposition: add
        assertion: "subs that do not solve are dropped and counted as not_solved, too_few_stars or solver_failed, and the first sub that solves is the reference sub"
        decision: "spec.md:US5-S3"
      - path: crates/finder/tests/console/stack_loop.rs
        status: new
        disposition: add
        assertion: "a failed exposure is counted, three in a row end the stack as camera with the frame's words, and the camera is then free"
        decision: "spec.md:US5-S4"
      - path: crates/finder/tests/console/stack_loop.rs
        status: new
        disposition: add
        assertion: "stop answers within a second against a solver that takes thirty, counts the waiting subs as unprocessed, and nothing changes afterwards"
        decision: "spec.md:US5-S5"
      - path: crates/finder/tests/console/stack_loop.rs
        status: new
        disposition: add
        assertion: "an aborted exposure ends the stack as stopped"
        decision: "spec.md:US5-S6"
      - path: crates/finder/tests/console/stack_loop.rs
        status: new
        disposition: add
        assertion: "on the simulated camera and solver the reference agrees with its solve sidecar and a later sub's turn and shift with a by-hand projection, and with a live pointing each sub's hint comes from it, at 3 degrees"
        decision: "spec.md:US5-S7"
      - path: crates/finder/tests/console/stack_loop.rs
        status: new
        disposition: add
        assertion: "while a stack runs a single imaging frame is refused with the holder's name, the stack, and the stack goes on; after the stop a frame is taken"
        decision: "spec.md:US5-S8"
      - path: crates/finder/tests/console/stack_loop.rs
        status: new
        disposition: add
        assertion: "on the simulated camera and solver with a live pointing, five stacked subs leave the session's aim point measured within 1.5 finder pixels of the simulator's, from five aim readings, with no page solve and a hint radius still 3 degrees"
        decision: "spec.md:US5-S9"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and each binary's main.rs holds only its support include and mod lines"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "031's take, ended, abort and view, its frame ids and files, and the lease's refusals are 031's and unchanged; a sub is one more saved frame (plan.md Shapes that existing tests pin)"
      - "032's solve_frame is called as it is and touches neither the page's solve nor the relation; a stacked sub's solve reaches the relation only through 032's note_solve, and neither the page's solve nor the kept scale changes (spec.md FR-015)"
      - "032's Imaging::hint is asked for every sub and is not edited: a live pointing's hint is used as it is (plan.md Plan traps 30)"
      - "034's Library::calibrate is called as it is; Calibration::busy is not asked"
      - "nothing under crates/finder/src/imaging/stack.rs or stack/ reads the mount, the site or an axis position (spec.md FR-019)"
      - "crates/finder/tests/console/imaging_frames.rs, a test file 031 creates, passes unchanged: frame ids, files, cards and refusals (plan.md Shapes that existing tests pin)"
      - "crates/finder/tests/console/imaging_solve.rs, a test file 032 creates, passes unchanged: the page's solve and its sidecar (plan.md Shapes that existing tests pin)"
      - "crates/finder/tests/console/imaging_relation.rs, a test file 032 creates, passes unchanged: the aim reading, the window and the narrow radius, which note_solve reaches as 032's own task does (plan.md Shapes that existing tests pin)"
      - "crates/finder/tests/console/calibration_console.rs, a test file 034 creates, passes unchanged: with_imaging still builds the calibration state first (plan.md Shapes that existing tests pin)"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console stack_loop::"
        - "cargo nextest run -p finder --locked --test console imaging"
        - "cargo nextest run -p finder --locked --test console calibration_console::"
        - "cargo nextest run -p finder --locked --test console imaging_relation::"
        - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs, and a rebuild of the console"
    writes:
      - GLOSSARY.md
      - crates/finder/src/api.rs
      - crates/finder/src/imaging/stack.rs
      - crates/finder/src/imaging/stack/service.rs
      - crates/finder/tests/console/main.rs
      - crates/finder/tests/console/stack_loop.rs
    no_write_reason: ""
    shared_writes:
      - path: GLOSSARY.md
        other_spec: 034-dark-and-flat-frames
        other_story: US7
        disposition: merge_order
        reason: "035 depends_on_landed 034, whose US7 last changed the glossary"
      - path: crates/finder/src/api.rs
        other_spec: 034-dark-and-flat-frames
        other_story: US4
        disposition: merge_order
        reason: "035 depends_on_landed 034, whose US4 last changed api.rs"
      - path: crates/finder/src/api.rs
        other_spec: 035-live-stack
        other_story: US8
        disposition: merge_order
        reason: "US8 follows US5 on the merge chain, and attaches the stacker and adds target_label"
      - path: crates/finder/src/imaging/stack.rs
        other_spec: 035-live-stack
        other_story: US6
        disposition: merge_order
        reason: "US6 follows US5 on the merge chain, and adds judge"
      - path: crates/finder/src/imaging/stack/service.rs
        other_spec: 035-live-stack
        other_story: US6
        disposition: merge_order
        reason: "US6 follows US5 on the merge chain, and adds the gate and bounds the queue"
      - path: crates/finder/tests/console/main.rs
        other_spec: 035-live-stack
        other_story: US6
        disposition: merge_order
        reason: "US6 follows US5 on the merge chain, and adds mod stack_gate"
    open_questions: []
  US6:
    tests:
      - path: crates/finder/tests/console/stack_gate.rs
        status: new
        disposition: add
        assertion: "same_settings is true only for equal size, depth, pattern, binning and region, a gain and an offset absent in both or within 0.001, and an exposure within 1 ms; a sub whose gain differs from the reference sub's is dropped as settings"
        decision: "spec.md:US6-S1"
      - path: crates/finder/tests/console/stack_gate.rs
        status: new
        disposition: add
        assertion: "a sub more than 30 percent above the median focus reading of the stacked subs is dropped as soft once five are known, a far-out one always, too few stars and a failed measure are counted, and focus_gate false measures nothing"
        decision: "spec.md:US6-S2"
      - path: crates/finder/tests/console/stack_gate.rs
        status: new
        disposition: add
        assertion: "judge allows a shift up to half the shorter side and a half turn, and refuses a scale off by more than 2 percent and a mirror; the stack counts each under its reason"
        decision: "spec.md:US6-S3"
      - path: crates/finder/tests/console/stack_gate.rs
        status: new
        disposition: add
        assertion: "with a solver ten times slower than the camera at most four subs wait, the oldest are dropped as behind, and the camera keeps taking a sub a second"
        decision: "spec.md:US6-S4"
      - path: crates/finder/tests/console/stack_gate.rs
        status: new
        disposition: add
        assertion: "after three solves in a row that miss, 032's hint is used until a sub solves and the stack's own again after that; a focus gate's too few stars counts as no miss"
        decision: "spec.md:US6-S5"
      - path: crates/finder/tests/console/stack_gate.rs
        status: new
        disposition: add
        assertion: "with a focus region kept, every sub's star measure is asked for that region in the sub's own pixels"
        decision: "spec.md:US6-S6"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and each binary's main.rs holds only its support include and mod lines"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "US5's start, stop, view and mean keep their signatures; US5's module stack_loop is imported and not edited"
      - "033's focus::read is called as it is; the stack writes no focus region, record or focus.json"
      - "crates/finder/tests/console/stack_loop.rs, a test file this spec's earlier story creates, passes unchanged: with the default measure no sub is dropped by the gate (plan.md Shapes that existing tests pin)"
      - "crates/finder/tests/console/imaging_focus_reading.rs, a test file 033 creates, passes unchanged: focus::read is read and not changed (plan.md Shapes that existing tests pin)"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console stack_gate::"
        - "cargo nextest run -p finder --locked --test console stack_loop::"
        - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs, and the field check of what the gate drops on a real night"
    writes:
      - crates/finder/src/imaging/stack.rs
      - crates/finder/src/imaging/stack/service.rs
      - crates/finder/tests/console/main.rs
      - crates/finder/tests/console/stack_gate.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/imaging/stack.rs
        other_spec: 035-live-stack
        other_story: US7
        disposition: merge_order
        reason: "US7 follows US6 on the merge chain, and adds pub mod display"
      - path: crates/finder/src/imaging/stack/service.rs
        other_spec: 035-live-stack
        other_story: US7
        disposition: merge_order
        reason: "US7 follows US6 on the merge chain, and adds the kept picture"
      - path: crates/finder/tests/console/main.rs
        other_spec: 035-live-stack
        other_story: US7
        disposition: merge_order
        reason: "US7 follows US6 on the merge chain, and adds mod stack_display"
    open_questions: []
  US7:
    tests:
      - path: crates/finder/tests/console/stack_display.rs
        status: new
        disposition: add
        assertion: "reduce makes each whole block one pixel from a stack's sums and weights, the block's sums over its weights, and never widens past max_width"
        decision: "spec.md:US7-S1"
      - path: crates/finder/tests/console/stack_display.rs
        status: new
        disposition: add
        assertion: "background fits a plane through the darker half of the tiles of each channel, each tile's level a lower median, ignores a bright square, and is flat when the tiles lie on one line"
        decision: "spec.md:US7-S2"
      - path: crates/finder/tests/console/stack_display.rs
        status: new
        disposition: add
        assertion: "tone takes the plane, applies the gains and one span for three channels, and stretches with asinh to the worked bytes"
        decision: "spec.md:US7-S3"
      - path: crates/finder/tests/console/stack_display.rs
        status: new
        disposition: add
        assertion: "render of a stack 2400 wide is 1200 wide, black on the background and white on the bright band, as a JPEG too, and shrink halves it"
        decision: "spec.md:US7-S4"
      - path: crates/finder/tests/console/stack_display.rs
        status: new
        disposition: add
        assertion: "a stack has no picture before its first sub, a picture named by a count that rises with every picture, a new one when the gains change, and keeps it after it ends"
        decision: "spec.md:US7-S5"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and each binary's main.rs holds only its support include and mod lines"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "the display takes the stack, or its sums and weights, by shared reference and changes nothing that is saved (plan.md Plan traps 19)"
      - "no mean of the stack's full size is made for the page's picture (plan.md Plan traps 29)"
      - "031's preview::rgb, preview::jpeg and its eight kept previews are unchanged; the stack's picture is kept by the Stacker"
      - "crates/finder/tests/console/imaging_preview.rs, a test file 031 creates, passes unchanged (plan.md Shapes that existing tests pin)"
      - "crates/finder/tests/console/stack_loop.rs and crates/finder/tests/console/stack_gate.rs, test files this spec's earlier stories create, pass unchanged (plan.md Shapes that existing tests pin)"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console stack_display::"
        - "cargo nextest run -p finder --locked --test console stack_loop::"
        - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs, and a design review of the picture's look"
    writes:
      - crates/finder/src/imaging/stack.rs
      - crates/finder/src/imaging/stack/display.rs
      - crates/finder/src/imaging/stack/service.rs
      - crates/finder/tests/console/main.rs
      - crates/finder/tests/console/stack_display.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/imaging/stack/service.rs
        other_spec: 035-live-stack
        other_story: US8
        disposition: merge_order
        reason: "US8 follows US7 on the merge chain, and adds the folder and the files"
      - path: crates/finder/tests/console/main.rs
        other_spec: 035-live-stack
        other_story: US8
        disposition: merge_order
        reason: "US8 follows US7 on the merge chain, and adds mod stack_console"
    open_questions: []
  US8:
    tests:
      - path: crates/finder/tests/console/stack_console.rs
        status: new
        disposition: add
        assertion: "POST and DELETE /api/imaging/stack start and stop a stack and answer the imaging view; stack.fits holds the worked cards and the stand-in's values in three planes of 32-bit reals, stack.png the picture, and stack.json every sub with its outcome"
        decision: "spec.md:US8-S1"
      - path: crates/finder/tests/console/stack_console.rs
        status: new
        disposition: add
        assertion: "the routes refuse a foreign origin, a body of the wrong shape or type and bad gains, the preview is 404 until there is one, and a console without imaging answers 409 to a body of the right shape and 400 to one of the wrong shape"
        decision: "spec.md:US8-S2"
      - path: crates/finder/tests/console/stack_console.rs
        status: new
        disposition: add
        assertion: "a stack's id carries the target's label, a stack with no sub writes stack.json alone, a second stack in the same second takes the id with -2, and label makes every character that is not an ASCII letter or digit one dash"
        decision: "spec.md:US8-S3"
      - path: crates/finder/tests/console/stack_console.rs
        status: new
        disposition: add
        assertion: "the stack rides in GET /api/imaging and in the session's imaging from the first start, through its end, until the next start, and a console without imaging carries none"
        decision: "spec.md:US8-S4"
      - path: crates/finder/tests/console/stack_console.rs
        status: new
        disposition: add
        assertion: "a runs path that is a file refuses the start with 500, and a folder removed mid-stack ends the stack with save_error and no files"
        decision: "spec.md:US8-S5"
      - path: crates/finder/tests/console/stack_console.rs
        status: new
        disposition: add
        assertion: "encode_cube writes a BITPIX -32 cube with its cards and big-endian reals, plane after plane"
        decision: "spec.md:US8-S6"
      - path: crates/finder/tests/cli/stack.rs
        status: new
        disposition: add
        assertion: "the built finder under --simulate stacks and leaves one run folder with stack.fits, stack.png and stack.json, and nothing of the stack in session.json"
        decision: "spec.md:US8-S7"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and each binary's main.rs holds only its support include and mod lines"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/console/session/edits.rs
        status: existing
        disposition: preserve
        assertion: "GET /api/session bodies on a console without imaging are unchanged"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/mcp/endpoint.rs
        status: existing
        disposition: preserve
        assertion: "get_session over MCP returns the session body unchanged when the console has no imaging"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "the imaging view on a console where no stack was started is byte for byte what 031 to 034 assert: stack is skipped while there is none (plan.md Shapes that existing tests pin)"
      - "031's encode and decode and 034's encode_real and decode_real are not edited"
      - "no sub and no solve sidecar is moved, copied or rewritten; nothing of a stack is kept in session.json"
      - "crates/finder/tests/console/imaging.rs, a test file 031 creates, passes unchanged: exactly off without imaging and no extra key in a view with no stack (plan.md Shapes that existing tests pin)"
      - "crates/finder/tests/console/calibration_console.rs and crates/finder/tests/console/calibration_library.rs, test files 034 creates, pass unchanged: the calibration object in the view and real images (plan.md Shapes that existing tests pin)"
      - "crates/finder/tests/console/stack_loop.rs, crates/finder/tests/console/stack_gate.rs and crates/finder/tests/console/stack_display.rs, test files this spec's earlier stories create, pass unchanged (plan.md Shapes that existing tests pin)"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console stack_console::"
        - "cargo nextest run -p finder --locked --test cli stack::"
        - "cargo nextest run -p finder --locked --test console imaging"
        - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs, a rebuild of the console, and a first stack saved at the telescope"
    writes:
      - crates/finder/README.md
      - crates/finder/src/api.rs
      - crates/finder/src/imaging/fits.rs
      - crates/finder/src/imaging/mod.rs
      - crates/finder/src/imaging/routes.rs
      - crates/finder/src/imaging/stack/service.rs
      - crates/finder/src/lib.rs
      - crates/finder/tests/cli/main.rs
      - crates/finder/tests/cli/stack.rs
      - crates/finder/tests/console/main.rs
      - crates/finder/tests/console/stack_console.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/README.md
        other_spec: 034-dark-and-flat-frames
        other_story: US8
        disposition: merge_order
        reason: "035 depends_on_landed 034, whose US8 last changed the README"
      - path: crates/finder/README.md
        other_spec: 035-live-stack
        other_story: US10
        disposition: merge_order
        reason: "US10 follows US8 on the merge chain, and adds the two tools' lines"
      - path: crates/finder/src/imaging/fits.rs
        other_spec: 034-dark-and-flat-frames
        other_story: US3
        disposition: merge_order
        reason: "035 depends_on_landed 034, whose US3 last changed fits.rs"
      - path: crates/finder/src/imaging/routes.rs
        other_spec: 034-dark-and-flat-frames
        other_story: US7
        disposition: merge_order
        reason: "035 depends_on_landed 034, whose US7 last changed the imaging routes"
      - path: crates/finder/src/lib.rs
        other_spec: 034-dark-and-flat-frames
        other_story: US7
        disposition: merge_order
        reason: "035 depends_on_landed 034, whose US7 last changed lib.rs"
      - path: crates/finder/tests/cli/main.rs
        other_spec: 033-focus-reading-and-aid
        other_story: US8
        disposition: merge_order
        reason: "035 depends_on_landed 033, whose US8 last changed cli/main.rs"
    open_questions: []
  US9:
    tests:
      - path: crates/finder/web/src/components/StackBlock.test.tsx
        status: new
        disposition: add
        assertion: "with no stack the region shows Start stacking, its hint and the four colour fields, Start posts the gains and stretch, and a refusal is an alert"
        decision: "spec.md:US9-S1"
      - path: crates/finder/web/src/components/StackBlock.test.tsx
        status: new
        disposition: add
        assertion: "a running stack reads its exact status, its dropped reasons and its missing masters, and Stop sends DELETE"
        decision: "spec.md:US9-S2"
      - path: crates/finder/web/src/components/StackBlock.test.tsx
        status: new
        disposition: add
        assertion: "the picture has the photo class, and at night Hold to see colour removes it only while held, on every release event and after 10 s"
        decision: "spec.md:US9-S3"
      - path: crates/finder/web/src/components/StackBlock.test.tsx
        status: new
        disposition: add
        assertion: "an ended stack reads its id, counts and files, with an alert when the camera ended it or it was not saved"
        decision: "spec.md:US9-S4"
      - path: crates/finder/web/src/components/StackBlock.test.tsx
        status: new
        disposition: add
        assertion: "Apply colour posts the changed gains and stretch on a running stack and shows a refusal"
        decision: "spec.md:US9-S5"
      - path: crates/finder/web/src/components/StackBlock.test.tsx
        status: new
        disposition: add
        assertion: "the Imaging page shows the region for a ready view after Calibration, and a request is followed at once by a read of /api/imaging"
        decision: "spec.md:US9-S6"
      - path: crates/finder/web/e2e/stack.spec.ts
        status: new
        disposition: add
        assertion: "at night the page with a stack's picture has no off-red pixel, the picture alone shows colour while its control is held, and it is red again on release"
        decision: "spec.md:US9-S7"
      - path: crates/finder/web/e2e/session.spec.ts
        status: existing
        disposition: preserve
        assertion: "the night edition's off-red count on the pages it screenshots is unchanged"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/e2e/simulate.spec.ts
        status: existing
        disposition: preserve
        assertion: "the simulate project's existing checks pass with one more file in its list"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "the page adds no npm package, no class and no token; index.css does not change"
      - "red-only night mode: the stack's picture is a .photo, covered by 031's one exception to the Red Torch Rule; no DESIGN.md rule changes"
      - "031's useColourHold is called as it is and not edited; StackBlock writes no timer, listener or key test of its own (spec.md FR-042)"
      - "crates/finder/web/src/components/ImagingPage.test.tsx, a test file 031 creates, passes unchanged: the page's words, statuses, alerts and picture for views without a stack (plan.md Shapes that existing tests pin)"
      - "crates/finder/web/src/components/CalibrationBlock.test.tsx, a test file 034 creates, passes unchanged: 034's block is not moved or reworded (plan.md Shapes that existing tests pin)"
      - "crates/finder/web/e2e/imaging.spec.ts, a test file 031 creates, passes unchanged: the Imaging page at night has no off-red pixel outside the held picture, the Stack block included (plan.md Shapes that existing tests pin)"
    checks:
      development:
        - "npm test"
        - "npm run typecheck"
        - "npm run build"
        - "CI=1 npm run e2e"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs: finder-page, and a design review of the block on a phone in the dark"
    writes:
      - .impeccable/design.json
      - DESIGN.md
      - crates/finder/web/dist
      - crates/finder/web/e2e/stack.spec.ts
      - crates/finder/web/playwright.config.ts
      - crates/finder/web/src/api.ts
      - crates/finder/web/src/components/ImagingPage.tsx
      - crates/finder/web/src/components/StackBlock.test.tsx
      - crates/finder/web/src/components/StackBlock.tsx
    no_write_reason: ""
    shared_writes:
      - path: .impeccable/design.json
        other_spec: 034-dark-and-flat-frames
        other_story: US9
        disposition: merge_order
        reason: "035 depends_on_landed 034, whose US9 last changed design.json"
      - path: DESIGN.md
        other_spec: 034-dark-and-flat-frames
        other_story: US9
        disposition: merge_order
        reason: "035 depends_on_landed 034, whose US9 last changed DESIGN.md"
      - path: crates/finder/web/dist
        other_spec: 034-dark-and-flat-frames
        other_story: US9
        disposition: merge_order
        reason: "035 depends_on_landed 034, whose US9 last rebuilt the bundle"
      - path: crates/finder/web/playwright.config.ts
        other_spec: 031-imaging-frames-on-console
        other_story: US4
        disposition: merge_order
        reason: "035 depends_on_landed 031, whose US4 last changed playwright.config.ts"
      - path: crates/finder/web/src/api.ts
        other_spec: 034-dark-and-flat-frames
        other_story: US9
        disposition: merge_order
        reason: "035 depends_on_landed 034, whose US9 last changed api.ts"
      - path: crates/finder/web/src/components/ImagingPage.tsx
        other_spec: 034-dark-and-flat-frames
        other_story: US9
        disposition: merge_order
        reason: "035 depends_on_landed 034, whose US9 last changed the Imaging page"
    open_questions: []
  US10:
    tests:
      - path: crates/finder/tests/mcp/stack.rs
        status: new
        disposition: add
        assertion: "set_stacking and get_stack are listed with their exact descriptions, schemas and read-only hint"
        decision: "spec.md:US10-S1"
      - path: crates/finder/tests/mcp/stack.rs
        status: new
        disposition: add
        assertion: "set_stacking starts a stack, changes its gains and stops it, get_stack answers the view with the picture as a colour JPEG, and get_session carries the stack in its imaging"
        decision: "spec.md:US10-S2"
      - path: crates/finder/tests/mcp/stack.rs
        status: new
        disposition: add
        assertion: "bad arguments answer isError with the words of the route, and get_stack before any stack answers state none"
        decision: "spec.md:US10-S3"
      - path: crates/finder/tests/mcp/stack.rs
        status: new
        disposition: add
        assertion: "on a console without imaging both tools answer empty arguments at once, set_stacking judges its arguments' shape before it says the console takes no imaging frames, and no result holds the scope node's token"
        decision: "spec.md:US10-S4"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and each binary's main.rs holds only its support include and mod lines"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/mcp/motion.rs
        status: existing
        disposition: preserve
        assertion: "every listed tool answers empty arguments on a console with no imaging, and no result or call-log line holds the node token"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "neither tool is a moving tool: no rate limit and no call-log line"
      - "the MCP session tools carry imaging.stack only because they serialize the session response"
      - "crates/finder/tests/mcp/imaging.rs and crates/finder/tests/mcp/calibration.rs, test files 031 and 034 create, pass unchanged: their tools' names, schemas and answers (plan.md Shapes that existing tests pin)"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test mcp stack::"
        - "cargo nextest run -p finder --locked --test mcp motion::"
        - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs, and one stack started and read by an MCP client at the telescope"
    writes:
      - crates/finder/README.md
      - crates/finder/src/mcp/tools.rs
      - crates/finder/tests/mcp/main.rs
      - crates/finder/tests/mcp/stack.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/mcp/tools.rs
        other_spec: 034-dark-and-flat-frames
        other_story: US8
        disposition: merge_order
        reason: "035 depends_on_landed 034, whose US8 last changed the MCP tools"
      - path: crates/finder/tests/mcp/main.rs
        other_spec: 034-dark-and-flat-frames
        other_story: US8
        disposition: merge_order
        reason: "035 depends_on_landed 034, whose US8 last changed mcp/main.rs"
    open_questions: []
```
