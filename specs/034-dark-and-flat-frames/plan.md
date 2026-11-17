# Implementation Plan: Dark and Flat Frames

**Branch**: `034-dark-and-flat-frames` | **Date**: 2026-11-15 | **Spec**: [spec.md](spec.md)

This plan records every design decision for the feature. A story that would change one
escalates rather than re-deciding it. It was revised on the day it was drafted, after a
second agent's review; the stories were renumbered then, when the library story was
split in two. It was revised again after the series audit of 030 to 037, which put the
sets on 031's lease of the imaging camera (the plan document's amendment 1), and its
citations were checked at 49bb210 (§ Refinement Record).

## Summary

The simulator's imaging camera gains a sensor pattern (hot pixels, a column pattern) and
a light path (a cover, a flat panel, vignetting, dust), whose known effect on a frame is
every later story's oracle. The console gains `calibrate`, a function over one sub's
mosaic that subtracts a master dark, divides by a master flat and repairs bad pixels from
same-colour neighbours, in that order, and reports what ran. A calibration library keeps
masters as 32-bit real FITS files with one JSON index, matches them to a frame by rule
and warns when the camera has turned since the flats, reading the imaging angle that
032 measures. A bad-pixel map is found in each master dark. Two procedures take sets
through 031's own frame path, each holding 031's lease of the imaging camera while it
is taken: darks, checked against an offset level and for light that
spreads over touching samples, with a way past the check for an observer who knows the
telescope is covered; and flats, whose exposure is found from the frame's level and
which are finished by their own darks. Four routes, three MCP tools and a Calibration
block on the Imaging page reach them. Nothing changes on the scope node or the scope
link.

## Technical Context

**Language/Version**: Rust (MSRV 1.96) for US1 to US8; TypeScript with React 19 for US9.

**Primary Dependencies**: nothing new. `finder` already has `serde_json`
(`crates/finder/Cargo.toml:32`), `tokio` with `fs`, `sync` and `time`, and the simulator
as a dependency (`crates/finder/Cargo.toml:14`); its tests already have the scope node's
crate (`crates/finder/Cargo.toml:36`), which the scripted camera implements a trait of.
No FITS crate: masters are written by `crates/finder/src/imaging/fits.rs` (031 US2), which
gains a real image. The page adds no npm package.

**Storage**: under the console's imaging folder (031: `imaging/` in the state folder, or
`--imaging-dir`): `calibration/index.json`, `calibration/darks/<id>.fits`,
`calibration/darks/<id>.badpixels.json` and `calibration/flats/<id>.fits`. Every dark and
flat of a set is an ordinary imaging frame under `frames/<UTC date>/`, saved by 031's
path. Nothing about calibration is kept in `session.json`; a set being taken, and flats
that wait for their darks, live in memory for a run.

**Testing**:
- US1: `crates/simulator/tests/imaging_calibration.rs` (new).
- US2: `crates/finder/tests/console/calibration.rs` (new), the module `calibration` of
  finder's `console` test binary: pure functions, and one frame from the sim rig.
- US3: `crates/finder/tests/console/calibration_library.rs` (new): real images and the
  library on a scratch folder, plain `#[test]`s.
- US4: `crates/finder/tests/console/calibration_console.rs` (new): the view and the two
  routes on the console rig.
- US5: `crates/finder/tests/console/calibration_bad_pixels.rs` (new).
- US6: `crates/finder/tests/console/calibration_darks.rs` (new), on the console rig and
  the scripted console rig.
- US7: `crates/finder/tests/console/calibration_flats.rs` (new), likewise, and one
  scenario on 032's solving rig.
- US8: `crates/finder/tests/mcp/calibration.rs` (new), the module `calibration` of `mcp`.
- US9: `crates/finder/web/src/components/CalibrationBlock.test.tsx` (new, Vitest). No
  Playwright test: the block holds no picture, and 031's `e2e/imaging.spec.ts` already
  counts every off-red pixel of the Imaging page at night, block included (§ Shapes that
  existing tests pin).
- Where a finder test goes is 027's rule: `console` calls the router in process and
  moves no mount, `mcp` goes through the MCP door. Each module is one file in its
  binary's folder and one `mod` line, in alphabetical order, in that binary's `main.rs`
  (`crates/finder/tests/layout/main.rs:102`). `crates/finder/tests/support` takes no
  new file (`crates/finder/tests/layout/main.rs:127`), and an unused helper fails
  `clippy -D warnings`. So a rig that more than one module needs is written once, in the
  first module whose story needs it, declared `pub(crate)`, and imported by the later
  ones with `use crate::<module>::…`, which the layout test allows: the test's own FITS
  reader in `calibration_library` (US3); the console rig, the request helper for
  `DELETE` and a foreign `Origin`, and the wait in `calibration_console` (US4); the
  scripted camera and the scripted console rig in `calibration_darks` (US6). A later
  story adds nothing to an earlier story's module. A module may bind the live helpers
  with
  `use crate::support::live as support;` and takes its scratch folder from
  `crate::support::Scratch` (`crates/finder/tests/support/mod.rs:12`), which 028's US5
  added on `main` at 49bb210.
- Every Rust test that waits runs in paused tokio time. No test waits in real time, and
  none starts a built binary.
- No test renders the worked example's 4944 × 3284 size: the simulated camera's 1236 × 822
  and the stand-in's 64 × 48 carry every scenario.

**Target Platform**: the `finder` console on any desk machine. The scope node is not
rebuilt for this spec.

**Constraints**:
- No `unsafe` (`crates/simulator/src/lib.rs:2`; `finder` adds none).
- No file under `crates/scope-node` or `crates/scope-link` changes, and no `Cargo.toml`
  or `Cargo.lock`.
- Nothing is keyed to one camera model, sized to one telescope or tied to one kind of
  mount: every threshold is relative to a measured level or noise, or a share of the
  frame or of full scale, and no file here computes field rotation, reads axis counts or
  names a focal length (the plan document's § Mount-neutral by design).
- With the simulator's sensor pattern and light path at their defaults every simulated
  frame is byte for byte 030's.
- `web/dist` is committed and must be the build of its source (US9).
- Budgets are things a test or a probe measures; this plan states none for the real
  camera.

**Scale/Scope**: three new source files in the console, one extended simulator module,
four routes, three MCP tools, one page block, nine stories.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Nothing touches the solver, star detection or `crates/math-core`. Behaviour here is grounded in the research note's § 9 (the order of the steps, same-colour repair before colour, a flat less its own dark, 3.5 σ for the map, a flat's level kept between 30 and 80 percent, a level test for darks, 8 °C) or is a design value declared as one and listed for the owner (spec owner decisions 4 to 15 and 24). |
| II. Never weaken a gate | No existing test, golden, fixture, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS`, `ergane.yaml` or `playwright.config.ts` is edited. The only edits to existing test files are the `mod` lines spec owner approval 9 names. Every test 030, 031, 032 and 033 land must pass unchanged, and § Shapes that existing tests pin says what that requires. |
| III. Tests first, independent expectations | Oracles: the simulator's declared patterns (themselves pinned by worked examples in US1), worked examples by hand, a scripted camera of the test's own, and files the test reads with its own FITS reader. A master is compared with the mean the test takes of the saved frames. Clocks are injected; virtual time only in tests. |
| IV. Critical paths | No `unsafe`, no dependency or `Cargo.toml` change, nothing under `proto/`, no scope-link change, no database-format change, no `build.rs` change. |
| V. Proven open source first, honest licensing | The finder camera's solver is untouched. The arithmetic here is this project's own; the open stackers the research note cites were read for their order of steps, and none of their code is copied. |

## Project Structure

```text
crates/simulator/
├── src/imaging.rs                      # US1: SensorPattern, LightPath, and their part in a frame
├── src/optics.rs                       # US1: the shared cell and its four methods
├── src/lib.rs                          # US1: exports
└── tests/imaging_calibration.rs        # US1: new
crates/finder/
├── README.md                           # US4: ## Calibration frames, two rows; US5, US6, US7: paragraphs and rows; US8: tool lines
├── src/lib.rs                          # US4: two route lines; US6, US7: one each
├── src/api.rs                          # US4: the calibration state in AppState
├── src/imaging/mod.rs                  # US2: pub mod calibration; US4: ImagingView.calibration, root, attach_calibration
├── src/imaging/fits.rs                 # US3: RealImage, encode_real, decode_real
├── src/imaging/routes.rs               # US4: GET, DELETE; US6: darks; US7: flats
├── src/imaging/calibration.rs          # US2: new, the arithmetic; US3: pub mod library; US4: pub mod service; US5: bad_pixel_map
├── src/imaging/calibration/library.rs  # US3: new, files, index, match; US5: the map's file
├── src/imaging/calibration/service.rs  # US4: new, the console's state; US6: darks; US7: flats
├── src/mcp/tools.rs                    # US8: three tools
├── tests/console/main.rs               # US2 to US7: one mod line each
├── tests/console/calibration.rs             # US2: new
├── tests/console/calibration_library.rs     # US3: new
├── tests/console/calibration_console.rs     # US4: new
├── tests/console/calibration_bad_pixels.rs  # US5: new
├── tests/console/calibration_darks.rs       # US6: new
├── tests/console/calibration_flats.rs       # US7: new
├── tests/mcp/main.rs                   # US8: mod calibration
├── tests/mcp/calibration.rs            # US8: new
└── web/
    ├── src/api.ts                      # US9: calibration types and calls
    ├── src/components/CalibrationBlock.tsx       # US9: new
    ├── src/components/CalibrationBlock.test.tsx  # US9: new
    ├── src/components/ImagingPage.tsx  # US9: placement
    └── dist/                           # US9: rebuilt
GLOSSARY.md                             # US5: Bad-pixel map; US6: Offset level; US7: Flat label
DESIGN.md                               # US9
.impeccable/design.json                 # US9
```

## Interfaces

Every file under `crates/finder/src/imaging/` that this spec does not create,
`crates/simulator/src/imaging.rs`, the Imaging page and the tests of 030 to 033 is
created by `030-imaging-camera-link`, `031-imaging-frames-on-console`,
`032-imaging-solve-and-aim-point` or `033-focus-reading-and-aid` and is named here
without a line; the story that, by the drafts of 030, 031, 032 and 033, last writes each is
in § Refinement Record's shared writes. Types of 031 used below (`Imaging`,
`ImagingView`, `FrameView`, `SettingsEdit`, `Refusal`, `FitsImage`, `CardValue`,
`FitsError`) are as `specs/031-imaging-frames-on-console/plan.md` § Interfaces gives them;
so are the lease (`Lease`, `Imaging::lease`, `Imaging::holder`, `Lease::set`,
`Lease::take`), in its § The camera lease, and `FrameView::set_temp_c`; § The
camera's lease below quotes both and says what this spec relies on. Of 032 this
plan uses the imaging view's `relation: Option<RelationView>` and its
`angle_deg: Option<f64>`, as `specs/032-imaging-solve-and-aim-point/plan.md` § The
imaging state and § The relation give them, and in one test `SimulatedSolver` and
`AppState::with_imaging_solver`. Of 033 it uses one word: `focus`, the purpose label
(`PURPOSE`) of every imaging frame its focus aid takes
(`specs/033-focus-reading-and-aid/plan.md` § The aid on the console), which `match`
passes over.

### The sensor pattern and the light path (`crates/simulator/src/imaging.rs`, US1)

```rust
/// One sensor sample that gathers charge by itself (034). Sensor pixels, unbinned.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HotPixel { pub x: u32, pub y: u32, pub electrons_per_s: f64 }

/// The sensor's own signal, there with or without light (034).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SensorPattern {
    pub hot_pixels: Vec<HotPixel>,
    /// Counts from the pattern's lowest column to its highest; 0.0 is no pattern.
    pub column_amplitude: f64,
}

/// An evenly lit panel in front of the telescope (034).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlatPanel {
    /// Electrons a second in a sample whose filter passes all of it, before vignetting.
    pub electrons_per_s: f64,
    /// Relative light in red, green and blue.
    pub colour: [f64; 3],
}

/// A disc on the sensor that passes less light (034). Sensor pixels, unbinned.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DustShadow { pub x: f64, pub y: f64, pub radius_px: f64, pub depth: f64 }

/// What light meets on its way to the sensor (034).
#[derive(Clone, Debug, PartialEq)]
pub struct LightPath {
    /// A cover on the telescope: no light at all.
    pub cover: bool,
    pub flat_panel: Option<FlatPanel>,
    /// Light at the sensor's corner as a share of the light at its centre; 1.0 is none lost.
    pub corner_light: f64,
    pub dust: Vec<DustShadow>,
}
// impl Default for LightPath: cover false, flat_panel None, corner_light 1.0, dust empty
```

In `crates/simulator/src/optics.rs`, `optics::Simulator`
(`crates/simulator/src/optics.rs:74`) gains one field,
`imaging_bench: Arc<Mutex<(SensorPattern, LightPath)>>`, built with both defaults in
`new` (`crates/simulator/src/optics.rs:81`). It is a second cell because the first,
`Pointing`, is `Copy` (`crates/simulator/src/optics.rs:64`) and these hold lists. Four
methods, reached through `controls::Simulator`'s `Deref` like the rest:

```rust
pub fn sensor_pattern(&self) -> SensorPattern;
/// Asserts every rate and the amplitude are finite and not negative.
pub fn set_sensor_pattern(&self, pattern: SensorPattern);
pub fn light_path(&self) -> LightPath;
/// Asserts 0 < corner_light ≤ 1, every depth from 0 to 1, every radius above 0, and a
/// panel's rate and colours finite and not negative.
pub fn set_light_path(&self, path: LightPath);
```

`crates/simulator/src/lib.rs` adds
`pub use imaging::{DustShadow, FlatPanel, HotPixel, LightPath, SensorPattern};`.

**Their part in a frame.** 030's `SimImagingCamera::expose` makes a frame in ten numbered
steps (`specs/030-imaging-camera-link/plan.md` § The simulated imaging camera, **A
frame**). This spec changes them as follows and in no other way. The pattern and the
path are read once, at step 1, with the settings, and hold for the exposure.

- **Steps 5 and 6, light.** With `cover` true, neither step adds anything. Otherwise,
  with a `flat_panel`, neither the stars nor the sky add anything, and every sample of
  the region gets `electrons_per_s · seconds · colour[its filter]` electrons instead.
  Otherwise the stars and the sky are as 030 has them, with their arithmetic and its
  order untouched. Then, as a step of its own, every sample's electrons are multiplied
  by V(x, y)·D(x, y), (x, y) being the sample's sensor pixel:
  V = 1 − (1 − corner_light)·((x + 0.5 − W/2)² + (y + 0.5 − H/2)²)/((W/2)² + (H/2)²),
  with W × H the sensor's size from the imaging optics, and D the product of
  `1 − depth` over the dust shadows for which
  (x + 0.5 − shadow.x)² + (y + 0.5 − shadow.y)² ≤ radius_px². With `corner_light` 1.0
  and no dust that step is skipped.
- **Step 6b, hot pixels.** After the light and before the noise, each hot pixel that lies
  in the region adds `electrons_per_s · seconds` electrons to its sample, covered or
  not. Step 7's noise uses the sample's electrons as they then stand.
- **Step 8b, the column pattern.** After step 8 turns electrons into counts and before
  step 9 bins, the sample at sensor column x gains
  `column_amplitude · ((x mod 8) as f64 / 7.0 − 0.5)` counts. With an amplitude of 0.0
  that step is skipped, as step 6b is with no hot pixel.

The truth (`imaging_truth`) is untouched. These constants and shapes are the simulator's
own, chosen so that each effect has an exact worked example; what they stand for is in
`docs/research/2026-11-15-imaging-camera.md` § 9. Calibration frames.

### The arithmetic (`crates/finder/src/imaging/calibration.rs`, new, US2; `bad_pixel_map`, US5)

```rust
//! Calibrating a sub on its mosaic (034): dark, flat, bad pixels, in that order, before
//! any colour. Pure functions; nothing here reads a file or a clock.
use serde::Serialize;

// US3 adds: pub mod library;    US4 adds: pub mod service;

/// A sample is hot when it stands this many noises above its largest neighbour ...
pub const HOT_SIGMAS: f32 = 8.0;
/// ... and this many times as far above its colour's median as that neighbour does ...
pub const HOT_RATIO: f32 = 5.0;
/// ... and no more than this many of the samples that touch it are lit.
pub const HOT_LIT_NEIGHBOURS: usize = 1;
/// A touching sample is lit when it stands this many of its colour's noises above its
/// colour's median ...
pub const ADJACENT_SIGMAS: f32 = 3.0;
/// ... and this share of what the candidate stands above its own.
pub const ADJACENT_SHARE: f32 = 0.1;
/// A flat under this is not divided by; its sample is repaired instead.
pub const FLAT_FLOOR: f32 = 0.1;
/// The least a noise or a spread may be: one count, the samples' own step.
pub const NOISE_FLOOR: f32 = 1.0;

/// One frame's samples as real numbers, row by row from the top-left.
#[derive(Debug, Clone, PartialEq)]
pub struct Mosaic {
    pub width: u32,
    pub height: u32,
    /// `"RGGB"`, `"BGGR"`, `"GRBG"` or `"GBRG"`, or `None` for a mono sensor.
    pub bayer: Option<String>,
    pub samples: Vec<f32>,
}
impl Mosaic {
    /// From little-endian samples of 8 or 16 bits. `None` when `bits` is neither, or the
    /// length is not width × height × bits / 8.
    pub fn from_samples(samples: &[u8], width: u32, height: u32, bits: u32,
        bayer: Option<&str>) -> Option<Self>;
    /// 0 red, 1 green, 2 blue: the pattern's letter at (x mod 2, y mod 2), its four
    /// letters being the top row then the bottom row. Always 0 for a mono frame.
    pub fn colour(&self, x: u32, y: u32) -> usize;
    /// 3 for a mosaic, 1 for mono.
    pub fn colours(&self) -> usize;
    /// How far along a row or column the nearest sample of the same colour is: 2 for a
    /// mosaic, 1 for mono.
    pub fn step(&self) -> u32;
}

/// The value at index ⌊p·(n − 1)⌋ of the values sorted with `f32::total_cmp`, p from 0
/// to 1. `None` for no values.
pub fn percentile(values: &[f32], p: f64) -> Option<f32>;

/// A colour's median, and its noise: the median less the 16th percentile, at least
/// `NOISE_FLOOR`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Level { pub median: f32, pub noise: f32 }
/// One `Level` a colour, in colour order.
pub fn levels(mosaic: &Mosaic) -> Vec<Level>;

/// What a sub is calibrated with. Each slice is row by row, as the sub is.
#[derive(Debug, Clone, Copy, Default)]
pub struct Masters<'a> {
    pub dark: Option<&'a [f32]>,
    /// Already normalised.
    pub flat: Option<&'a [f32]>,
    pub bad_pixels: Option<&'a [(u32, u32)]>,
}

/// What `calibrate` did. Serialized with these six keys, in this order.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Report {
    /// `applied` or `missing`.
    pub dark: &'static str,
    pub flat: &'static str,
    /// `map` when a bad-pixel map was given, `sub` when hot pixels were looked for in
    /// the sub itself.
    pub bad_pixels: &'static str,
    pub repaired: u64,
    pub not_repaired: u64,
    pub low_flat: u64,
}

/// Displays as `the master {master} holds {have} samples and the sub {need}`.
#[derive(Debug, Clone, PartialEq)]
pub struct SizeMismatch { pub master: &'static str, pub have: usize, pub need: usize }
// impl fmt::Display, impl std::error::Error

pub fn calibrate(sub: &mut Mosaic, masters: &Masters<'_>) -> Result<Report, SizeMismatch>;
pub fn find_hot_pixels(mosaic: &Mosaic) -> Vec<(u32, u32)>;
/// How many samples stand more than `HOT_SIGMAS` noises above their colour's median
/// without being hot pixels: light that reached the sensor, not the sensor's own signal.
pub fn lit_samples(mosaic: &Mosaic) -> u64;
/// Answers (repaired, not repaired).
pub fn repair(mosaic: &mut Mosaic, bad: &[(u32, u32)]) -> (u64, u64);
/// Divide each colour by its own median and answer the medians, in colour order.
pub fn normalise_flat(flat: &mut Mosaic) -> Vec<f32>;
```

**`calibrate`**, in this order:

1. If a dark is given and its length is not the sub's, `SizeMismatch` with `master`
   `dark`; then the same for the flat with `flat`. The sub is untouched.
2. With a dark, each sample becomes `sample − dark`. Nothing is clipped.
3. With a flat, each sample whose flat is a number and at least `FLAT_FLOOR` becomes
   `sample / flat`; any other sample is left as it is, joins the bad list and is counted
   in `low_flat`.
4. The bad list is completed: with `bad_pixels` given, its places; otherwise
   `find_hot_pixels` of the sub as it now stands. Then `repair`.

**`find_hot_pixels`**: with `levels` giving each colour's median B and noise N, and s the
mosaic's `step`, a sample v at (x, y) of colour c is hot when all three of these hold:

1. it has at least one of the neighbours (x − s, y), (x + s, y), (x, y − s), (x, y + s)
   inside the frame and, with n the largest of those inside the frame,
   `v − n > HOT_SIGMAS · N_c`;
2. `v − B_c > HOT_RATIO · max(n − B_c, 0)`;
3. at most `HOT_LIT_NEIGHBOURS` of the samples that touch it are lit. The samples that
   touch it are (x + i, y + j) for i and j each −1, 0 or 1, not both 0, inside the
   frame: up to eight, of whatever colour. One of them, a, of colour k, is **lit** when
   `a − B_k > max(ADJACENT_SIGMAS · N_k, ADJACENT_SHARE · (v − B_c))`.

The answer is in row order: by y, then x. On a mono frame s is 1, so the neighbours of
tests 1 and 2 are four of the eight of test 3.

A star's light reaches the samples around its peak and a hot pixel's does not. Tests 1
and 2 read that two samples away, which is the nearest sample of the same colour. For a
Gaussian star of width σ samples centred on a sample, the neighbour two away holds
exp(−2/σ²) of the peak: a ratio of 4.0 at the simulator's sharpest star (σ 1.2), which
test 2 keeps, but 7.4 at σ 1.0 and 22.8 at σ 0.8, which it would flatten. In-focus stars
at the worked example's scale are 4.6 to 5.5 pixels across
(`docs/research/2026-11-15-imaging-camera.md` § 6. Focus), 2.3 to 2.75 samples at
binning 2, where σ is about 1.0 to 1.2 and tests 1 and 2 alone are not safe; a telescope
with a coarser scale is worse. Test 3 reads the samples that touch the peak, one sample
away, where a star centred on a sample leaves exp(−1/(2σ²)) of its peak: 0.71 at σ 1.2,
0.46 at σ 0.8, 0.25 at σ 0.6. With the colours of a star differing by up to a factor of
2.5 between neighbouring filters, a star of σ 0.6 or more, 1.4 samples across at half
its height, lights at least the four samples beside its peak by more than a tenth of
that peak and is kept, at any binning. The floor of `ADJACENT_SIGMAS` noises keeps noise
from lighting a sample; it also means a star whose peak is under about 30 noises may
not light its neighbours and can lose its brightest sample. One lit sample is allowed so
that two hot pixels that touch, which real sensors have and the simulator's H has, are
both found; three that touch are left for the bad-pixel map. In nebulosity every
touching sample stands above the frame's median by the nebula's own level g, so a hot
pixel is found there only when it stands more than 10·g above its median: test 2 alone
asked for 5·g. The five constants are spec owner decision 14.

**`lit_samples`**: the number of samples (x, y), of colour c, with
`v − B_c > HOT_SIGMAS · N_c` that are not in the answer of `find_hot_pixels` for the
same mosaic. A covered sensor's bright samples are hot pixels, alone or in pairs, and
are not counted; a star's are counted. The dark test uses it (§ The console's
calibration state).

**`repair`**: the places inside the frame, each once, are the bad set. For each, the good
neighbours are those of the four at distance `step` that are inside the frame and not in
the bad set. With at least one, the sample becomes their median: the middle one of an
odd number, the mean of the middle two of an even number. With none it is left and
counted as not repaired. Because a bad sample is never read as a neighbour, the order of
the list does not matter.

**`normalise_flat`**: for each colour, m is the `percentile` at 0.5 of that colour's
samples; with m above 0 each is divided by m, otherwise each becomes 0.0.

**`bad_pixel_map`** (US5), with `BadPixels`:

```rust
pub const BAD_PIXEL_SIGMAS: f32 = 3.5;
/// A map of more than this share of the frame is no map.
pub const BAD_PIXEL_MAX_SHARE: f64 = 0.01;

#[derive(Debug, Clone, PartialEq)]
pub struct BadPixels {
    /// (x, y), by y then x. Empty when `too_many`.
    pub pixels: Vec<(u32, u32)>,
    pub level: f32,
    pub spread: f32,
    pub too_many: bool,
}
pub fn bad_pixel_map(master_dark: &[f32], width: u32, height: u32) -> BadPixels;
```

`level` is the `percentile` at 0.5 of all the samples, `spread` is `level` less the
`percentile` at 0.16, at least `NOISE_FLOOR`. A sample is bad when it is not finite or is
above `level + BAD_PIXEL_SIGMAS · spread`. When the bad samples number more than
`BAD_PIXEL_MAX_SHARE` times width × height, `too_many` is true and `pixels` is empty.
The spread is the master's own, fixed pattern included, so a column pattern widens the
line and is not itself mapped (US5-S2).

`crates/finder/src/imaging/mod.rs` gains `pub mod calibration;` (US2), in alphabetical
order among its `pub mod` lines.

### Real images (`crates/finder/src/imaging/fits.rs`, US3)

```rust
/// One FITS image of 32-bit reals (034): a master frame.
#[derive(Debug, Clone, PartialEq)]
pub struct RealImage {
    pub width: u32,
    pub height: u32,
    /// Row by row from the top.
    pub samples: Vec<f32>,
    /// The cards after the structural ones, in order.
    pub cards: Vec<(String, CardValue)>,
}
pub fn encode_real(image: &RealImage) -> Vec<u8>;
pub fn decode_real(bytes: &[u8]) -> Result<RealImage, FitsError>;
```

`encode_real` writes `SIMPLE = T`, `BITPIX = -32`, `NAXIS = 2`, `NAXIS1`, `NAXIS2`, then
`cards`, then `END`, each card formatted exactly as 031's `encode` formats its own, with
the same padding of the header and of the data to multiples of 2880; no `BZERO` and no
`BSCALE`; each sample as a big-endian IEEE 754 single (`f32::to_be_bytes`).

`decode_real` reads what `encode_real` writes. Its checks, in this order, each with
exactly this text: the first card is not `SIMPLE` with the value `T`, or there is no
`END`, `not a FITS image`; `NAXIS` is not 2, `a {n}-axis FITS file is not one image`;
`BITPIX` is not −32, `not a 32-bit real FITS image: BITPIX {n}`; fewer data bytes than
`NAXIS1 × NAXIS2 × 4`, `the FITS data is short: {have} of {need} bytes`. `cards` are
every card after `NAXIS2` up to `END`, typed as 031's `decode` types them.

031's `encode` and `decode` are not edited: `decode` of a real image still answers
`unsupported FITS BITPIX -32`, which 031's US2-S10 asserts.

### The library (`crates/finder/src/imaging/calibration/library.rs`, new, US3; the map's file, US5)

```rust
//! The calibration library on disk (034): masters, their index, and which match a frame.
pub const FORMAT: &str = "finder-calibration/1";
pub const BAD_PIXELS_FORMAT: &str = "finder-bad-pixels/1";    // US5
pub const SET_TEMP_TOLERANCE_C: f64 = 1.0;
pub const SENSOR_TEMP_TOLERANCE_C: f64 = 8.0;
pub const EXPOSURE_TOLERANCE_MS: f64 = 1.0;
pub const SETTING_TOLERANCE: f64 = 0.001;
pub const FLAT_ANGLE_WARN_DEG: f64 = 1.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind { Dark, Flat }

/// One master as `index.json` and the calibration object list it. Every key is always
/// written, in this order; an absent value is null.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MasterEntry {
    pub id: String,
    pub kind: Kind,
    /// Console Unix seconds when the master was made.
    pub made_at: f64,
    /// The frames averaged.
    pub frames: u32,
    /// False when the darks behind this master were taken with `anyway`, so that the
    /// dark test did not judge them; true otherwise.
    pub checked: bool,
    pub model: String,
    pub serial: Option<String>,
    pub width: u32,
    pub height: u32,
    /// The frame's origin on the sensor in binned pixels (031's XORGSUBF, YORGSUBF).
    pub origin_x: u32,
    pub origin_y: u32,
    pub bits: u32,
    pub bayer: Option<String>,
    pub gain: Option<f64>,
    pub offset: Option<f64>,
    pub binning: u32,
    pub set_temp_c: Option<f64>,
    /// The frames' mean sensor temperature; null when any had none.
    pub temp_c: Option<f64>,
    pub exposure_ms: f64,
    /// A dark's median in counts; null for a flat.
    pub level: Option<f64>,
    /// The size of a dark's bad-pixel map; null for a flat and for a dark without one.
    pub bad_pixels: Option<u64>,
    /// A flat's label; null for a dark.
    pub label: Option<String>,
    /// The imaging angle when a flat's frames were taken; null when unknown or a dark.
    pub imaging_angle_deg: Option<f64>,
    /// A flat's level as a share of full scale; null for a dark.
    pub level_share: Option<f64>,
    /// The darks taken off a flat; null for a dark.
    pub dark_frames: Option<u32>,
    /// `darks/<id>.fits` or `flats/<id>.fits`, relative to the library's folder.
    pub file: String,
}

/// What a frame is matched by.
#[derive(Debug, Clone, PartialEq)]
pub struct FrameKey {
    pub model: String,
    pub serial: Option<String>,
    pub width: u32,
    pub height: u32,
    pub origin_x: u32,
    pub origin_y: u32,
    pub bits: u32,
    pub bayer: Option<String>,
    pub gain: Option<f64>,
    pub offset: Option<f64>,
    pub binning: u32,
    pub set_temp_c: Option<f64>,
    pub temp_c: Option<f64>,
    pub exposure_ms: f64,
}
impl FrameKey {
    /// From a saved imaging frame's cards. The error names the first card it lacks:
    /// `the imaging frame's file has no {card} card`.
    pub fn from_image(image: &fits::FitsImage) -> Result<Self, String>;
    /// From the latest frame of an imaging view, when it is saved.
    pub fn from_view(view: &ImagingView) -> Option<Self>;
    /// Whether this frame belongs in a set whose first frame had the key `first` and
    /// which asked for `exposure_ms`: the rule under **Matching**.
    pub fn agrees(&self, first: &FrameKey, exposure_ms: f64) -> bool;
}

/// The masters chosen for a frame. Serialized in this order.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Selection {
    /// The imaging frame the key came from, when the console took it from one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame: Option<u64>,
    pub dark: Option<String>,
    pub flat: Option<String>,
    pub bad_pixels: Option<u64>,
    /// Those of `dark`, `flat` and `bad_pixels` that are absent, in that order.
    pub missing: Vec<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flat_turned_deg: Option<f64>,
}

pub struct Applied {
    pub selection: Selection,
    pub report: calibration::Report,
    /// One line for each chosen master that could not be read or did not fit.
    pub problems: Vec<String>,
}

pub struct Library { /* root, Mutex<index and its error>, Mutex<the masters last read> */ }
impl Library {
    /// Read `index.json` under `root`. Never fails: see `error`.
    pub fn open(root: PathBuf) -> Self;
    pub fn error(&self) -> Option<String>;
    /// By `made_at`, then `id`.
    pub fn masters(&self) -> Vec<MasterEntry>;
    pub fn new_id(&self, kind: Kind, unix_s: f64) -> String;
    /// Write the master's file and list it. `entry.file` is set here.
    pub fn add(&self, entry: MasterEntry, samples: &[f32]) -> std::io::Result<MasterEntry>;
    /// True when a master had that id.
    pub fn remove(&self, id: &str) -> std::io::Result<bool>;
    pub fn select(&self, key: &FrameKey, flat_label: Option<&str>, angle_deg: Option<f64>)
        -> Selection;
    pub fn calibrate(&self, key: &FrameKey, sub: &mut calibration::Mosaic,
        flat_label: Option<&str>, angle_deg: Option<f64>) -> Applied;
    // US5:
    pub fn attach_bad_pixels(&self, id: &str, map: &calibration::BadPixels)
        -> std::io::Result<MasterEntry>;
}
```

**The folder.** `root` is `calibration/` under the imaging folder:

```text
calibration/
├── index.json
├── darks/<id>.fits
├── darks/<id>.badpixels.json      (US5)
└── flats/<id>.fits
```

**`index.json`**, written whole with `serde_json::to_vec_pretty` as `index.json.part` and
renamed over the old one:

```json
{"format": "finder-calibration/1", "masters": [ { "id": "dark-20260116T020003Z", "kind": "dark", … } ]}
```

`open`: no file is an empty library. A file that is not JSON, whose `format` is not
`FORMAT`, or one of whose masters does not parse as a `MasterEntry`, is an empty library
whose `error` is `the calibration index could not be read: {why}`, why being
`serde_json`'s message or `its format is {format}`. The next `add` renames that file to
`index.json.unreadable`, replacing an earlier one, before it writes the new index, and
clears the error.

**`new_id`**: `dark-` or `flat-`, then the UTC second of `unix_s` rounded down, as
`YYYYMMDDTHHMMSSZ` (031's `fits::utc` gives the date and the time; its `-`, `:` and
fraction are dropped). While a master with that id is listed, or its file exists, `-2`,
`-3` and so on are added.

**`add`**: makes `darks/` or `flats/`; writes `encode_real` of the samples and the cards
below as `<file>.part` with `create_new`; if the final name exists, removes the `.part`
file and answers an `std::io::Error` of kind `AlreadyExists`; otherwise renames, lists
the entry and writes the index. Nothing is listed when the file could not be written.

**A master's cards**, in order after `NAXIS2`: `IMAGETYP` (`MASTER DARK` or
`MASTER FLAT`), `CALID` (the id), `NFRAMES` (integer), `NDARKS` (integer; a flat only),
`DARKTEST` (logical, the entry's `checked`: `T` or `F`), `EXPTIME` (real, seconds),
`GAIN` (real), `OFFSET` (real), `XBINNING` and `YBINNING` (integer), `XORGSUBF` and
`YORGSUBF` (integer), `CCD-TEMP` (real), `SET-TEMP` (real),
`BAYERPAT` (text), `INSTRUME` (text, the model), `SERIALNO` (text), `ROWORDER`
(`TOP-DOWN`), `DATE` (text, `fits::utc` of `made_at`), then for a flat `FLATLBL` (text)
and `IMGANGLE` (real). `GAIN`, `OFFSET`, `CCD-TEMP`, `SET-TEMP`, `BAYERPAT`, `SERIALNO`
and `IMGANGLE` are left out when the entry has none.

**`remove`**: unlists the master, writes the index, then removes its `.fits` file and its
`.badpixels.json` file if present, ignoring a file already gone. The masters last read
are forgotten.

**`FrameKey::from_image`** reads `INSTRUME` (model), `SERIALNO`, `GAIN`, `OFFSET`,
`XBINNING` (binning), `XORGSUBF`, `YORGSUBF`, `CCD-TEMP`, `SET-TEMP`, `BAYERPAT` and
`EXPTIME` (seconds, times 1000), with the image's own width, height and bits. The cards
it needs and names when absent are, in this order, `INSTRUME`, `EXPTIME`, `XBINNING`,
`XORGSUBF` and `YORGSUBF`. **`from_view`** answers `None` unless the view has a `camera`
and a `frame` whose state is `saved`; then the model and serial are the camera's; width,
height, bits, Bayer pattern, gain, offset, binning, sensor temperature and exposure are
the frame's; the origin is the frame's `roi.x` and `roi.y` each divided by its binning;
and the set temperature is the frame's own `set_temp_c` (§ The camera's lease, the
last paragraph): the one 031 kept beside the frame for its file's `SET-TEMP` card,
present only when the cooler was on. Nothing is read from the view's `settings`, so a
key taken from the view and one taken from the saved file agree (spec US4-S3).

**Matching.** `same(a, b)` for two optional numbers: both absent, or both present and
within `SETTING_TOLERANCE`. A master of either kind **fits** a key when `model`, `serial`,
`width`, `height`, `origin_x`, `origin_y`, `bits`, `bayer` and `binning` are equal.

- A **dark** matches when it fits, `same` holds for gain and for offset,
  `|exposure_ms − key's| ≤ EXPOSURE_TOLERANCE_MS`, and the temperatures agree: both have
  a set temperature and they differ by at most `SET_TEMP_TOLERANCE_C`; or neither has
  one, and either both have a sensor temperature and these differ by at most
  `SENSOR_TEMP_TOLERANCE_C`, or at least one has none. Its **distance** is the
  difference that was compared, 0.0 when none was. Of the matching darks the least
  distance wins, then the greatest `made_at`, then the greatest `id`.
- A **flat** matches when it fits and, with `flat_label` given, its label is that one.
  Its gain, offset, set temperature, exposure and sensor temperature are not compared.
  The greatest `made_at` wins, then the greatest `id`. This is the wider match the owner
  settled on 2026-11-17, a departure from the plan document's first key that he
  approves and that the plan document now writes (spec owner decision 7).
- `flat_turned_deg`: with a flat chosen, `angle_deg` given and the flat's
  `imaging_angle_deg` present, d = |((angle − flat's + 180) mod 360, taken in 0 to 360)
  − 180|; when d is above `FLAT_ANGLE_WARN_DEG` the selection carries d.
- `bad_pixels` is the chosen dark's. `missing` lists `dark`, `flat` and `bad_pixels`,
  those that are absent, in that order. `frame` is `None`: the console sets it. All of
  this is US3's; US5 changes nothing in `select`, it only gives an entry a `bad_pixels`
  to carry.
- **`agrees`**: `self` fits `first` (the nine keys above), `same` holds between them for
  gain, for offset and for the set temperature, and
  `|self.exposure_ms − exposure_ms| ≤ EXPOSURE_TOLERANCE_MS`. Sensor temperature is not
  compared. The sets use it to hold every frame to their first and to the exposure they
  asked for; a set's first frame is asked `first.agrees(&first, e)`, which tests its
  exposure alone.

**`calibrate`**: `select`; read the chosen dark and flat with `decode_real`, and (from
US5 on) the dark's map when its entry has one, keeping the last dark, flat and map read
so that a run of subs reads each once; a master that cannot be read, or whose width and height are
not the sub's, adds `the master {dark|flat} {id} could not be used: {why}` to `problems`
and is not given to `calibration::calibrate`, so that step is `missing`; a map that
cannot be read adds `the bad-pixel map of {id} could not be used: {why}` and the search
in the sub runs instead. Then `calibration::calibrate`. It reads files, so a caller on
the runtime runs it on the blocking pool.

**The map's file** (US5). `attach_bad_pixels`: with `too_many`, nothing is written and
the entry is answered as it is. Otherwise `darks/<id>.badpixels.json` is written as a
`.part` file and renamed:

```json
{"format":"finder-bad-pixels/1","width":20,"height":20,"level":160.0,"spread":1.0,"sigmas":3.5,"pixels":[[3,2],[5,5]]}
```

the entry's `bad_pixels` becomes the number of pixels, and the index is written. An id
that is not a listed dark is an `std::io::Error` of kind `NotFound`.

### The camera's lease (`crates/finder/src/imaging/mod.rs`, 031 US6; written by no story of this spec)

The plan document's amendment 1: "The console's imaging state (031) holds one lease
naming the holder; a second taker is refused with the holder's name. Each later spec
takes and releases that lease and adds no lock of its own." The series audit gave it to
031's trio as its User Story 6 (031 spec.md, owner approval 12), and 031's plan.md now
gives its interface in § The camera lease, in the shape
`specs/033-focus-reading-and-aid/plan.md` § The camera lease quotes and 035's and 037's
plans take too. This spec uses that shape name for name, and the re-anchoring reads
every name again in 031's code (§ Prerequisites 2 and 11):

```rust
// 031's (031 US6; its plan.md § The camera lease), as 033's, 035's and 037's plans quote it:
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
```

What this spec relies on, and nothing more:

1. `lease(holder)` hands the camera to the caller or answers a 409 `Refusal` whose
   message contains the name of whoever holds it. It does not wait (spec US6-S9).
2. While a lease is out, `Imaging::take` and `Imaging::set` by anyone else, and so
   `POST /api/imaging/frame`, `PUT /api/imaging/settings`, 031's MCP tools, 033's focus
   aid and 035's stack, are refused as in 1, with the holder's name, before their check
   for a frame in flight. Those refusals are 031's code and tests; spec US6-S9 reads
   two of them while a set is exposing, to show the name they carry is `HOLDER`.
3. The holder's `Lease::set` and `Lease::take` behave as 031's `set` and `take` do
   today: they refuse while a frame is in flight and while the camera is not `ready`.
4. `Imaging::abort`, `Imaging::ended` and `Imaging::view` are open to anyone, lease or
   no lease, so `DELETE /api/imaging/calibration/{id}` and `POST /api/imaging/abort`
   stop a set's exposure (spec US6-S6).
5. Dropping the `Lease` releases the camera, also when the task that owns it is
   aborted, and a `lease` asked for after that succeeds (spec US6-S9).
6. A single imaging frame asked for with `Imaging::take` holds no lease, so while it is
   in flight `holder()` is `None` (spec US6-S5 asserts it there, so that a 031 whose
   single frame does take the lease shows up at the re-anchoring as that assertion).

The tests of this spec assert of that refusal only the status 409 and that its words
contain the holder's name (spec.md § User Scenarios, "The lease"), so a rewording of
031's sentence breaks nothing here.

The same audit gives 031's `FrameView` one more key, `set_temp_c: Option<f64>` (031
spec.md FR-009, its US2): the set temperature `take` keeps beside the frame for its
file's `SET-TEMP` card, the one the settings held with the cooler on when the frame was
asked for, skipped while `None` like the view's other `Option`s. `FrameKey::from_view`
reads it (§ The library); nothing else here does.

### The console's calibration state (`crates/finder/src/imaging/calibration/service.rs`, new, US4; darks, US6; flats, US7)

```rust
//! The calibration library as the console holds it, and the sets it takes (034).
pub const MIN_SET_FRAMES: u32 = 3;                  // US6
pub const MAX_SET_FRAMES: u32 = 64;
/// A dark may read this many noises above the offset level.
pub const DARK_RISE_SIGMAS: f64 = 2.0;
/// An offset level above this share of full scale is an uncovered camera.
pub const COVER_SHARE: f64 = 0.2;
/// A dark may hold this share of its samples lit (`calibration::lit_samples`) ...
pub const LIT_SHARE: f64 = 0.0001;
/// ... or this many, whichever is more.
pub const LIT_MIN: u64 = 8;
/// The name a set of darks, and each step of a set of flats, holds the imaging
/// camera's lease under; 031's refusals show it to every other taker.
pub const HOLDER: &str = "a set of darks or flats";
pub const FLAT_TARGET: f64 = 0.45;                  // US7
pub const FLAT_LOW: f64 = 0.40;
pub const FLAT_HIGH: f64 = 0.50;
pub const FLAT_SATURATED: f64 = 0.90;
pub const FLAT_SEARCH_FRAMES: u32 = 8;
pub const FLAT_MAX_EXPOSURE_MS: f64 = 30_000.0;

/// `GET /api/imaging/calibration`, and `calibration` in the imaging view.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct CalibrationView {
    pub masters: Vec<MasterEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(rename = "match", skip_serializing_if = "Option::is_none")]
    pub matched: Option<Selection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub set: Option<SetView>,                       // US6
}
impl CalibrationView {
    /// `{"masters":[]}`: a console that takes no imaging frames.
    pub fn empty() -> Self;
}

/// The latest set. Serialized in this order.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct SetView {                                // US6; US7 adds the three keys marked
    pub id: String,
    /// `dark` or `flat`.
    pub kind: &'static str,
    /// `offset`, `frames`, `search`, `needs_darks`, `flat_darks`, `done`, `failed` or
    /// `stopped`.
    pub state: &'static str,
    pub wanted: u32,
    pub taken: u32,
    pub exposure_ms: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,                      // US7
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset_level: Option<f64>,
    /// The latest frame's level in counts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<f64>,
    /// A flat set's mean level as a share of full scale.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level_share: Option<f64>,                   // US7
    #[serde(skip_serializing_if = "Option::is_none")]
    pub darks_taken: Option<u32>,                   // US7
    /// True when the dark test refused the set or its darks; left out when false.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub not_dark: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub struct Calibration { /* imaging, library, Mutex<the key last matched>; US6: clock, Mutex<set and what it holds> */ }
impl Calibration {
    /// The library is opened at `root`, `calibration/` under the imaging folder.
    pub fn new(imaging: Arc<Imaging>, root: PathBuf, clock: Arc<dyn Clock>) -> Arc<Self>;
    pub fn library(&self) -> &Library;
    /// `imaging` is the view the match is worked out from.
    pub fn view(&self, imaging: Option<&ImagingView>) -> CalibrationView;
    /// Remove a master, or (US6, US7) stop or drop the set with this id.
    pub async fn remove(&self, id: &str) -> Result<CalibrationView, Refusal>;
    // US6:
    pub async fn take_darks(self: &Arc<Self>, request: DarksRequest)
        -> Result<CalibrationView, Refusal>;
    // US7:
    pub async fn take_flats(self: &Arc<Self>, request: FlatsRequest)
        -> Result<CalibrationView, Refusal>;
}

/// The body of `POST /api/imaging/calibration/darks`.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DarksRequest {                                         // US6
    pub count: Option<u32>,
    /// Take the set whatever the dark test says.
    pub anyway: Option<bool>,
}

/// The body of `POST /api/imaging/calibration/flats`.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlatsRequest {                                         // US7
    pub count: Option<u32>,
    pub label: Option<String>,
    pub darks: Option<bool>,
    /// With `darks`: take them whatever the dark test says.
    pub anyway: Option<bool>,
}
```

**`new`.** US4 has no use for the clock: it writes the third parameter as `_clock` and
keeps nothing of it, so that no field goes unread under `-D warnings`. US6 names it
`clock` and keeps it in a field it adds, with the set's state. The signature's types do
not change, so `crates/finder/src/api.rs` is written once, by US4.

**`view`** (US4): `masters` from the library, `error` from `library.error()`, `set` from
the latest set (US6). `matched` comes from **the key last matched**, a frame id and a
`FrameKey` that the service keeps for a run:

1. With `imaging` given, in state `ready`, whose `frame` is `saved` with a purpose other
   than `dark`, `flat` and `focus` and an id above the kept id (or with nothing kept),
   the kept pair becomes that frame's id and `FrameKey::from_view(imaging)`. `focus` is
   read as 033's constant, `crate::imaging::focus::run::PURPOSE`, so that the two
   specs cannot drift apart; `dark` and `flat` are this module's own words. 033's aid
   removes each of its frames when it saves the next, so a match on one would name a
   file about to go.
2. With a pair kept, `matched` is `library.select(&key, None, angle)` with `frame` set
   to the kept id; with none, it is `None`. The angle is `imaging`'s
   `relation.angle_deg` (032) when `imaging` is given and carries one, otherwise none.

So `match` stays while later frames are exposing, or are darks, flats or focus frames,
and the camera's turn is judged against the imaging angle as it stands now. `view` reads
no file and waits for nothing; its one lock is held for step 1 alone.

**`remove`** (US4): an id that is not `dark-` or `flat-` followed by 8 digits, `T`, 6
digits, `Z` and optionally `-` and digits answers 404
`no master or set has the id {id}` without touching the disk. Otherwise
`library.remove(id)` on the blocking pool: true answers the view, false the same 404, an
error 500 `could not remove the master: {error}`. US6 puts one step first: when the
latest set has this id and is being taken (`offset`, `frames`, `search` or
`flat_darks`), its stop flag is set, `imaging.abort()` is awaited, and the view is
answered; the runner ends the set `stopped`. US7 adds: when it is in `needs_darks`, the
flats in hand are dropped and the set becomes `stopped` at once.

**The set's lease** (US6; US7 for the two steps of flats). The plan document's
amendment 1, as spec owner decision 23 records it. The request takes the lease with
`imaging.lease(HOLDER)` as the last of its checks (§ **`take_darks`**, 6) and moves the
`Lease` into the task it spawns; the `Lease` is never kept in the set's state, so no
lock of this module guards it. The task owns it from its first line to its last:

- every exposure the set asks for is set with
  `lease.set(SettingsEdit { exposure_ms: Some(e), ..Default::default() })`;
- every imaging frame is taken with `lease.take(purpose)`, whose answer is the imaging
  view with the new frame in it, the frame's id being that view's `frame.id`;
- `imaging.abort()`, `imaging.ended(id)` and `imaging.view()` are called on the
  `Imaging` as anyone may (§ The camera's lease, 4);
- **the ending**, however the task ends: the exposure in force before the request is
  put back with `lease.set` of that exposure alone, a `Refusal` there being ignored
  (a camera that is no longer `ready` refuses it, and the setting then stays where the
  set left it, spec § Edge Cases); then the `Lease` is dropped; and only then is the
  set's final state written (`needs_darks`, `done`, `failed` or `stopped`). So a reader
  who sees the final state finds the camera free and the setting put back.

The task never ends while a frame of its own is in flight: once `lease.take` has
answered, every way out passes step 2 of the helper below, which awaits the frame's
end. So the ending's `lease.set` is never refused for the set's own frame. Flats that
wait for their darks hold no lease; the request for their darks takes a new one and
spawns a new task (§ **`take_flats`**, *The second step*). This module adds no lock over
the camera and no `busy` reading: who holds the camera is `imaging.holder()`, 031's.

**A set's frame** (US6), one private helper used by every step. It is given the
`Lease`, the purpose and the exposure e the frame is to have:

1. `lease.take(purpose)`. A `Refusal` ends the step as failed with the refusal's
   message.
2. `imaging.ended(id)`. `saved` goes on; `aborted` ends the set `stopped`; `failed` ends
   the step as failed with the frame's `error`.
3. On the blocking pool: read `imaging.root()` joined with the frame's `file`,
   `fits::decode` it, take `FrameKey::from_image` and `Mosaic::from_samples`. Any error
   ends the step as failed with `could not read {file}: {error}`.
4. The key must agree with the set's first frame's key and with e:
   `key.agrees(&first, e)`, `first` being this key itself for the set's first frame.
   Otherwise the set fails with `the imaging settings changed during the set`.
5. With a set temperature s and a sensor temperature t both in the key, and
   `|t − s| > SET_TEMP_TOLERANCE_C` (the library's constant), the set fails with
   `the sensor is at {t:.1} °C, not yet at its set temperature {s:.1} °C`, whether or
   not the set was asked for with `anyway`.

The stop flag is read three times: before step 1, when a set stops without taking the
frame; right after `take` answers in step 1, when a flag set in the meantime has
`imaging.abort()` awaited there, since the `abort` of `remove` may have run before the
frame existed; and after step 2. A failed step of a dark set, or of a flat set's darks,
is worded `a dark failed: {why}`; of a flat set's flats, `a flat failed: {why}`. The
exposure is changed with
`lease.set(SettingsEdit { exposure_ms: Some(e), ..Default::default() })`; a `Refusal`
there fails the step with its message. The failures of steps 4 and 5 carry no such
prefix: they are the set's own.

**`take_darks`** (US6). Refusals, in this order, the first that applies answering:

1. `count` absent or outside `MIN_SET_FRAMES` to `MAX_SET_FRAMES`: 400
   `count must be a whole number from 3 to 64`;
2. the imaging view not `ready`: 409 with 031's words for its state
   (`the scope node is not connected`, `this scope node has no imaging service`,
   `this scope node has no imaging camera`,
   `the imaging camera is not answering: {error}`);
3. a set in `offset`, `frames`, `search` or `flat_darks`: 409
   `a set of darks or flats is already being taken`;
4. (US7 adds this one) a set in `needs_darks`: 409
   `flats are waiting for their darks: take those or drop them first`;
5. nobody holds the lease (`imaging.holder()` is `None`) and the view's frame is
   `exposing` or `receiving`: 409 `an exposure is in progress`. While somebody holds
   the lease this check is passed over, whatever the frame's state, and check 6
   answers with the holder's name;
6. `imaging.lease(HOLDER)` refused: that `Refusal`, unchanged, its status and its
   words.

The set checks come before the frame check because a set being taken is nearly always
inside an exposure of its own, and the request must be told which of the two it is
(US6-S5, US8-S3). The frame check is made only while nobody holds the lease because
031's single imaging frame holds none, so only this check can name it, while a frame of
a holder's is better named by the holder (US6-S9). Checks 1 to 6 have no await between
them, but the console runs on a multi-threaded runtime, so two requests can pass checks
1 to 5 at the same moment; the lease is what makes them exclusive: one is handed the
camera at check 6 and the other is refused there, with `a set of darks or flats` in the
words. No test asserts which of the two refusals a same-instant request gets. Do not
hold this module's lock from check 3 to check 6 to make them one step: `lease` takes
031's lock. Otherwise the set is made, with the id
`library.new_id(Kind::Dark, clock.now())`, `state` `offset`, `wanted` N, `taken` 0 and
`exposure_ms` E, the exposure the settings hold; a task is spawned and the `Lease` moved
into it; and the view is answered. `anyway` is true when the request's is `Some(true)`.

The task:

1. Sets the exposure to the camera's shortest (`camera.exposure_ms.min` of the view),
   lo, and takes the **offset frame** with the purpose `dark` and the exposure lo. L is
   the `percentile` at 0.5 of all its samples, its noise S is L less the `percentile` at
   0.16, at least `NOISE_FLOOR`, and F is 2^bits − 1. `offset_level` becomes L. Unless
   `anyway`: if L > `COVER_SHARE` · F the set fails with
   `the camera is not covered: its shortest exposure reads {L} of {F}` and `not_dark`.
2. Sets the exposure to E; `state` becomes `frames`. For each of the N darks, purpose
   `dark` and exposure E: its median M becomes `level`. Unless `anyway`, two tests, in
   this order:
   - if M > L + `DARK_RISE_SIGMAS` · S the set fails with
     `the darks are not dark: one reads {M} against an offset level of {L}; cover the telescope`
     and `not_dark`;
   - with n = `calibration::lit_samples` of the dark's mosaic and c its number of
     samples, if n > `LIT_MIN` and n as `f64` > `LIT_SHARE` · c the set fails with
     `the darks are not dark: one holds {n} bright samples that are not hot pixels; cover the telescope`
     and `not_dark`.

   Otherwise its samples are added to a sum of `u32` and `taken` rises.
3. The master is the sum over N as `f32`. Its entry takes the first dark's key
   (`model` to `set_temp_c`), `exposure_ms` E, `frames` N, `checked` true unless
   `anyway`, `temp_c` the mean of the darks' sensor temperatures or null when any had
   none, `level` the master's median, `made_at` the clock's reading, and the set's id.
   On the blocking pool: `library.add`, then `bad_pixel_map` and
   `library.attach_bad_pixels`. An error is `the master could not be saved: {error}`.
4. **The ending** with E, the final state being `done`.

`{L}`, `{M}`, `{F}` and `{n}` are written with `{}`: a median of whole samples is a whole
number and prints with no fraction. However the task ends, it goes through **the
ending** of § **The set's lease** with E, so that a test that waits for the state finds
the setting restored and `holder()` `None`.

Why three tests and a way past them. The level test alone passes an uncovered camera
wherever the sky adds less than `DARK_RISE_SIGMAS` noises in the exposure: on the
simulated camera at gain 0 and 200 ms the sky adds one count against a line of two, and
the master would hold the field's stars. The lit test catches that case by what light
looks like: about 1200 samples of that simulated frame stand more than 8 noises above
their medians in touching groups, against 101 allowed, while its three hot pixels, two
of which touch, are found as hot pixels and not counted (US6-S1, US6-S3). `LIT_MIN`
keeps one cosmic-ray track in a small region frame from refusing a set. And the tests
are only as good as their constants on a camera nobody has measured: a warm sensor at a
long exposure, a glow, or a sensor whose shortest exposure reads lower than a long dark
would each refuse real darks, so `anyway` takes the set and the entry records that
nothing judged it (spec owner decision 4).

**`take_flats`** (US7). The body is valid as flats when `count` and `label` are present
and `darks` and `anyway` are each absent or false, and as darks when `darks` is true and
`count` and `label` are absent; anything else answers 400
`send count and label to take flats, or darks: true to take their darks`. Then, for
flats, `count` as for darks, and a label that is not 1 to 32 bytes of `a` to `z`, `0`
to `9` and `-` beginning with a letter or digit, 400
`label must be 1 to 32 characters of a to z, 0 to 9 and -, starting with a letter or digit`.
Then the refusals 2 to 6 of `take_darks`, in that order, with one difference at 4: for
`{"darks":true}` it is 409 `no flats are waiting for their darks` unless the latest set
is a flat set in `needs_darks`, and the request passes that check when it is. A request
that passes all six holds the lease, `HOLDER` again, and is accepted; the `Lease` is
moved into the step's task and owned by it as § **The set's lease** says. Each of the
two steps holds a lease of its own, and none is held between them.

*The first step.* The set is made with the id `library.new_id(Kind::Flat, clock.now())`,
`kind` `flat`, `state` `search`, the label, `wanted` N, `taken` 0 and `exposure_ms` E0,
the exposure in force. With lo the camera's shortest exposure, hi the smaller of its
longest and `FLAT_MAX_EXPOSURE_MS`, F as above, and a frame's **level** the largest
`median` of `levels` of its mosaic:

```text
e = clamp(E0, lo, hi) rounded to 0.001 ms; tried = []; points = []
repeat at most FLAT_SEARCH_FRAMES times:
    set the exposure to e; take a frame with the purpose `flat` and the exposure e; M = its level
    tried += e; p = 100·M/F
    if FLAT_LOW·F ≤ M ≤ FLAT_HIGH·F: the exposure is e; stop searching
    if M ≥ FLAT_SATURATED·F:       next = e / 8
    else:
        points += (e, M)
        if points has one entry:   next = e·2 when M < FLAT_TARGET·F, else e / 2
        else, with (ea, Ma) and (eb, Mb) its last two entries:
            k = (Mb − Ma) / (eb − ea)
            unless k > 0: fail `the flat's level does not rise with the exposure: {ea} ms and {eb} ms read {Ma} and {Mb}`
            next = eb + (FLAT_TARGET·F − Mb) / k
    next = clamp(next, lo, hi) rounded to 0.001 ms
    if next is in tried:
        fail `the flat is too dim: {e} ms reads {p}% of full scale; is the panel lit?` when M < FLAT_LOW·F,
        else `the flat is too bright: {e} ms reads {p}% of full scale; dim the panel`
    last = (e, p)
    e = next
if no exposure was found, with (el, pl) = last:
    fail `no exposure put the flat between 40 and 50% of full scale in 8 tries; the last, {el} ms, read {pl}%`
```

`last` is the eighth frame taken and its level, not the ninth exposure the search would
have asked for. Rounding to 0.001 ms is `(x · 1000.0).round() / 1000.0`. `{e}`, `{el}`,
`{ea}`, `{eb}`, `{Ma}` and `{Mb}` are written with `{}`, and `{p}` and `{pl}` with
`{:.1}`. With the exposure found, `exposure_ms` becomes it and `state` `frames`; N flats
are taken at it, each checked: a level outside `FLAT_LOW`·F to `FLAT_HIGH`·F fails the
set with `a flat reads {p}% of full scale, outside 40 to 50; is the panel steady?`;
otherwise its samples are summed and `taken` rises. The frames the search took are not
among the N. Then `level_share` becomes the mean of the N levels over F; the sum, the
set's first frame's key, the mean sensor temperature and the imaging angle are kept with
the set, the angle being `imaging.view()`'s `relation.angle_deg` (032) read then, or
none when the view carries none; and the step goes through **the ending** with E0, its
final state being `needs_darks`. A failure goes through it with E0 too and ends the set
`failed`. Flats in `needs_darks` hold no lease: the observer may take imaging frames
and change settings until the darks are asked for.

*The second step.* `state` becomes `flat_darks`, `darks_taken` 0, and `error` and
`not_dark` are cleared. With E1 the exposure in force: an offset frame at the camera's
shortest exposure and then N darks at the flats' exposure are taken as `take_darks`
takes its own, with the same three tests, the same texts and the same `anyway`, each
frame also held to the flats' first frame by step 4 of the helper, `darks_taken` rising.
A failure goes through **the ending** with E1 and returns the set to `needs_darks` with
that `error`, and `not_dark` when a test refused it, the flats still in hand; a stop
ends it `stopped` and drops them. On success the master is, sample by sample,
(flats' sum − darks' sum)/N as `f32`, made a `Mosaic` with the flats' pattern and passed
to `normalise_flat`; then every sample that is not above 0, or not finite, becomes 0.0.
Its entry takes the flats' key, `exposure_ms` the flats' exposure, `frames` N,
`dark_frames` N, `checked` true unless `anyway`, the label, the angle kept,
`level_share`, `made_at` and the set's id; `library.add`; and the step goes through
**the ending** with E1, its final state being `done`.

**What the three requests answer.** `remove`, `take_darks` and `take_flats` answer what
`get_calibration` answers (§ The routes): the `calibration` of `imaging.view()` when
that view is `ready`, otherwise `view(None)`. A refused request answers its `Refusal`.

No lock of this module is held across an await, and none while `Imaging` is called:
`Imaging::view` calls back into `Calibration::view`.

### 031's imaging state (`crates/finder/src/imaging/mod.rs`, US4)

`ImagingView` gains, after its last part (`frame` in 031's draft; 032 adds `solve` and
`relation` after it, and 033 its `focus`):

```rust
#[serde(skip_serializing_if = "Option::is_none")]
pub calibration: Option<calibration::service::CalibrationView>,
```

`None` in `ImagingView::off()`. `Imaging` gains a field
`calibration: std::sync::OnceLock<std::sync::Weak<Calibration>>` and

```rust
/// The imaging folder this state saves under.
pub fn root(&self) -> &Path;
/// Let every view this state answers carry the calibration object (034). The first
/// call wins.
pub(crate) fn attach_calibration(&self, calibration: &Arc<Calibration>);
```

Wherever `Imaging` builds the `ImagingView` it answers (`view`, and through it `set`,
`take` and `abort`), after its own lock is released and only when the view's `state` is
`ready`, it sets `calibration` to `attached.view(Some(&view))` when a calibration is
attached and still alive. The view handed over is the finished one, `relation`
included, since that is where `Calibration::view` reads the imaging angle. The
reference is weak because `Calibration` holds the `Arc<Imaging>`.

### The console (`crates/finder/src/api.rs`, US4)

`AppState` (`crates/finder/src/api.rs:101`) gains `calibration: Option<Arc<Calibration>>`
and

```rust
/// The calibration library and its sets (034); present whenever `imaging()` is.
pub fn calibration(&self) -> Option<Arc<Calibration>>;
```

`with_imaging` (031 US1), right after it starts the `Imaging` over `root`, builds
`Calibration::new(imaging.clone(), root.join("calibration"), self.clock.clone())`, calls
`imaging.attach_calibration(&calibration)` and keeps it. No builder is added and no
signature changes; 032's `with_imaging_solver` and `with_focal_length_mm` are untouched. `SessionResponse` (`crates/finder/src/api.rs:652`) does not change:
its `imaging` is the view, which now carries the object.

### The routes (`crates/finder/src/imaging/routes.rs`, US4; US6; US7)

Added to 031's `routes()`:

```rust
// US4:  GET    /api/imaging/calibration          get_calibration
//       DELETE /api/imaging/calibration/{id}     delete_calibration
// US6:  POST   /api/imaging/calibration/darks    take_darks
// US7:  POST   /api/imaging/calibration/flats    take_flats
```

- `get_calibration` answers `Json`: without a calibration, `CalibrationView::empty()`;
  otherwise the imaging view's `calibration` when it has one, and
  `calibration.view(None)` when the imaging state is not `ready`.
- `delete_calibration` takes `SessionWrite` and `Path<String>`; without a calibration it
  answers 409 `this console takes no imaging frames`; a `Refusal` becomes
  `ApiError::new(status, message)`, as 031's handlers do.
- `take_darks` and `take_flats` take `SessionWrite` and
  `Result<Json<DarksRequest>, JsonRejection>` (or `FlatsRequest`) through `sim_json`
  (`crates/finder/src/api.rs:570`), which 031 made `pub(crate)`: 415 for another
  `Content-Type`, 400 for a body that is not JSON or of the wrong shape, an unknown key
  included. Without a calibration, the same 409.

**The order a request is judged in**, the first that applies answering: the origin
(`SessionWrite`, 403); the body's shape (`sim_json`, 415 or 400), on the two `POST`
routes; a console without a calibration (409 `this console takes no imaging frames`);
then what `Calibration::remove`, `take_darks` or `take_flats` refuses, in the order
§ The console's calibration state gives, the count and the label first. So on a console
that takes no imaging frames `{"count":2}` answers 409 and `{"count":"4"}` answers 400
(US6-S5). Written as handlers: the two extractors are the handler's arguments, in that
order, and the handler's first statement looks for the calibration.

The router's matcher takes the static segments `darks` and `flats` beside the parameter
`{id}`; each path has one method, so `DELETE …/darks` and `POST …/<an id>` answer 405,
which no scenario asks for.

`crates/finder/src/lib.rs` gains, after the last `/api/imaging` route line already
there (031's, 032's and 033's by then) and written as they stand here, with no
backticks:
`* GET /api/imaging/calibration: the calibration library's masters, the set being taken and what matches the latest imaging frame.`
and
`* DELETE /api/imaging/calibration/{id}: remove a master dark or master flat, or stop or drop the set with that id.`
(US4);
`* POST /api/imaging/calibration/darks: {"count":N}, N from 3 to 64, and "anyway":true to pass the dark test; take darks at the settings in force and add their master dark.`
(US6);
`* POST /api/imaging/calibration/flats: {"count":N,"label":"..."} to take flats, then {"darks":true} to take their darks and add the master flat.`
(US7).

### `crates/finder/README.md` and `GLOSSARY.md`

`crates/finder/README.md`. US4 adds, after the last `/api/imaging` row already in the
`## API` table (the table's heading is `crates/finder/README.md:167` and its last row
`crates/finder/README.md:184` at 49bb210; 031, 032 and 033 add theirs after it), two
rows worded like their neighbours:

```markdown
| `GET /api/imaging/calibration` | The calibration library: `masters`, the `set` being taken, what would `match` the latest imaging frame, and `error` when the library's index is unreadable. |
| `DELETE /api/imaging/calibration/{id}` | Remove a master dark or master flat and its files, or stop or drop the set with that id. Answers 200 with the calibration object, 404 for an unknown id. Same-origin writes only. |
```

and, at the end of the file, `## Calibration frames`, saying: what a master dark, a
master flat and the bad-pixel map remove, and that a sub is calibrated dark first, then
flat, then bad pixels, before any colour; the folder `calibration/` under the imaging
folder, its four kinds of file and the cards of a master; every key of a master's entry;
when a master dark matches and when a master flat does, with the five tolerances and
that a flat needs only the sub's camera, size, origin, bits, pattern and binning, not
its gain, offset, set temperature or exposure; the
turned-camera warning, and that the imaging angle is known only while the aim point is
the measured one; that `calibrate` reports `applied` or `missing` for each master and
`map` or `sub` for bad pixels; that `match` describes the latest imaging frame seen
saved that is neither a dark, a flat nor a frame of the focus aid; and that nothing is
ever removed but by the
`DELETE` route. US5 adds the paragraph on the bad-pixel map: its rule, its file, the
1 percent limit, and the search in the sub that stands in for it with its three tests
and their limits (stars under 1.4 samples across, three hot pixels that touch,
nebulosity). US6 adds the row

```markdown
| `POST /api/imaging/calibration/darks` | JSON `{"count":N}`, N from 3 to 64. Cover the telescope first. Takes one offset frame and N darks at the settings in force, checks that they are dark, and adds their master dark and its bad-pixel map; progress is `set`. `"anyway":true` takes them whatever the check says. Same-origin writes only. |
```

and the paragraph on taking darks: the offset frame and offset level, the three
refusals of the dark test and their words, `anyway` and what `checked` false and
`DARKTEST` F then record, the refusal of a sensor more than 1.0 °C from its set
temperature, that each dark is kept as an imaging frame under `frames/` and what a set
of 20 costs in disk, that a set is stopped with the `DELETE` route or by aborting its
exposure, that a set holds the imaging camera from its start to its end so that an
imaging frame, a settings change, the focus aid or a stack asked for meanwhile is
refused with the words `a set of darks or flats` in it, and that a set asked for while
another holds the camera is refused with that holder's name, and the limits of the test
that spec.md § Edge Cases names. US7 adds the row

```markdown
| `POST /api/imaging/calibration/flats` | JSON `{"count":N,"label":"..."}` with a flat panel lit: finds the exposure and takes N flats. Then, with the telescope covered, `{"darks":true}`, with `"anyway":true` to pass the dark test: takes their darks and adds the master flat. Same-origin writes only. |
```

and the paragraph on taking flats: the two steps, the search and its five failures, the
flat label, that each step holds the imaging camera while it runs and that flats in hand
hold nothing, so the camera is free between the two steps, that flats in hand are lost
when the console restarts, and that new flats are needed when the camera is turned. US8 adds, in the tool list of `## MCP clients`
(`crates/finder/README.md:57`), one line a tool after the imaging tools already there.

`GLOSSARY.md`. US5 adds **Bad-pixel map** after the **Calibration library** entry
(`GLOSSARY.md:167`), US6 adds **Offset level** after the **Flat** entry
(`GLOSSARY.md:163`), and US7 adds **Flat label** after the **Offset level** entry, each
with exactly the three lines spec owner approval 6 gives. The two cited lines are of the
glossary as the plan document rewrites it, which is in the drafting worktree and not
committed; at dbc2b5b and at 49bb210 neither entry exists (§ Prerequisites 6).

### The MCP tools (`crates/finder/src/mcp/tools.rs`, US8)

Two structs that only describe a tool's input, beside `TargetArguments`
(`crates/finder/src/mcp/tools.rs:31`) and written as it is, with
`#[expect(dead_code, reason = "schema only; DarksRequest parses the actual arguments")]`
(and `FlatsRequest` for the second):

```rust
#[derive(rmcp::schemars::JsonSchema)]
struct DarksArguments { count: u32, anyway: Option<bool> }
#[derive(rmcp::schemars::JsonSchema)]
struct FlatsArguments {
    count: Option<u32>, label: Option<String>, darks: Option<bool>, anyway: Option<bool>,
}
```

Three tools in the `#[tool_router]` block (`crates/finder/src/mcp/tools.rs:165`), after
the imaging tools already there (031's three, 032's one and 033's two), each
`output_schema = object_schema()`
(`crates/finder/src/mcp/tools.rs:104`):

- `list_calibration`, `annotations(read_only_hint = true)`, description
  `List the calibration library's master darks and master flats, the set being taken, and what matches the latest imaging frame.`:
  `CallToolResult::structured` of what `get_calibration` answers.
- `take_darks`, description
  `Take a set of darks with the telescope covered and add their master dark to the calibration library, as POST /api/imaging/calibration/darks does; anyway: true passes the dark test. It answers once the set has started; read its progress with list_calibration.`,
  with `input_schema` from `DarksArguments` as `set_target` declares its own
  (`crates/finder/src/mcp/tools.rs:355`): takes `arguments: JsonObject`, parses them as
  `DarksRequest` the way `set_target` parses its edit, calls `Calibration::take_darks`,
  and answers the view; a parse error or a `Refusal` is `CallToolResult::error` with
  the words.
- `take_flats`, description
  `Take a set of flats of a lit flat panel, or with darks: true the darks that finish them, as POST /api/imaging/calibration/flats does; anyway: true passes the dark test of those darks. It answers once the step has started; read its progress with list_calibration.`,
  with `input_schema` from `FlatsArguments`: the same with `FlatsRequest` and
  `Calibration::take_flats`.

Without a calibration the first answers `{"masters":[]}` and the other two the error
`this console takes no imaging frames`, before the arguments are looked at. None is a
moving tool: no rate limit and no call-log line.

### The page (`crates/finder/web/src/`, US9)

`api.ts`, after 031's imaging types:

```ts
export interface CalibrationMaster {
  id: string; kind: "dark" | "flat"; made_at: number; frames: number; checked: boolean;
  gain: number | null; offset: number | null; binning: number;
  set_temp_c: number | null; temp_c: number | null; exposure_ms: number;
  bad_pixels: number | null; label: string | null; imaging_angle_deg: number | null;
}
export interface CalibrationMatch {
  frame?: number; dark: string | null; flat: string | null; bad_pixels: number | null;
  missing: string[]; flat_turned_deg?: number;
}
export interface CalibrationSet {
  id: string; kind: "dark" | "flat";
  state: "offset" | "frames" | "search" | "needs_darks" | "flat_darks" | "done" | "failed" | "stopped";
  wanted: number; taken: number; exposure_ms: number; label?: string;
  offset_level?: number; level?: number; level_share?: number; darks_taken?: number;
  not_dark?: boolean; error?: string;
}
export interface ImagingCalibration {
  masters: CalibrationMaster[]; error?: string; match?: CalibrationMatch; set?: CalibrationSet;
}
export async function postCalibrationDarks(count: number, anyway?: boolean): Promise<ImagingCalibration>;
export async function postCalibrationFlats(count: number, label: string): Promise<ImagingCalibration>;
export async function postCalibrationFlatDarks(anyway?: boolean): Promise<ImagingCalibration>;
export async function deleteCalibration(id: string): Promise<ImagingCalibration>;
```

`ImagingView` gains `calibration?: ImagingCalibration;`. The three posts are written as
`postTrack` is (`crates/finder/web/src/api.ts:312`), with the bodies `{count}`,
`{count, label}` and `{darks: true}`, and with `anyway` given as true the bodies
`{count, anyway: true}` and `{darks: true, anyway: true}`, the keys in that order;
`deleteCalibration` calls
`fetch(\`/api/imaging/calibration/${encodeURIComponent(id)}\`, { method: "DELETE" })`.
Each throws an `Error` with the answer's `error` on a refusal.

`components/CalibrationBlock.tsx`,
`CalibrationBlock({ calibration, onChange }: { calibration: ImagingCalibration; onChange: () => void })`,
a `section` labelled by an `h2` `Calibration` (`section-title`), with these words
exactly:

- **The match.** With `match`: one line,
  `Imaging frame {frame} would get: {dark} · {flat} · {bad}`, where {dark} is
  `its master dark` or `no master dark`; {flat} is `the master flat {label}`, the label
  of the master whose id `match.flat` is, or `no master flat`; and {bad} is
  `a bad-pixel map of {bad_pixels}` or `hot pixels found in the sub itself`. With
  `flat_turned_deg`, also an `alert`:
  `The camera has turned {flat_turned_deg to one decimal}° since the flats {label} were taken. Take new flats.`
  Without `match`, the `quiet` line
  `Take an imaging frame to see which masters match it.`
- **The library.** With `error`, an `alert`:
  `The calibration library is unreadable: {error}.` With no masters, the `quiet` line
  `No master darks or flats yet.` Otherwise a list, one item a master, oldest first:
  - a dark:
    `Master dark · {exposure_ms} ms · gain {gain} · offset {offset} · {b} × {b} · {temp} · {frames} frames · {bad} · {date}`,
    where {temp} is `set {set_temp_c to one decimal} °C`, or without one
    `sensor {temp_c to one decimal} °C`, {bad} is `{bad_pixels} bad pixels` or
    `no bad-pixel map`, and {date} is the UTC date of `made_at` as `YYYY-MM-DD`; the
    `gain`, `offset` and {temp} parts are each left out, with their ` · `, when null;
  - a flat:
    `Master flat {label} · gain {gain} · offset {offset} · {b} × {b} · {temp} · {frames} frames · {date}`,
    with the same {temp} and the same parts left out when null, followed by
    ` · camera at {imaging_angle_deg to one decimal}°` when it has an angle. Its gain,
    offset and set temperature are shown as what its flats were taken at, though none
    of them restricts the match (spec owner decision 7);
  - either, when `checked` is false, followed by ` · dark test skipped`;
  - each with a button `Remove` (`button`) whose `aria-label` is `Remove {id}`, which
    calls `deleteCalibration(id)`.
- **Darks.** A number field `Dark frames` (default 20), the button `Take darks`
  (`button button-primary`), which calls `postCalibrationDarks`, and the `quiet` line
  `Cover the telescope first. Darks use the exposure, gain, offset, binning and set temperature in force.`
- **Flats.** A number field `Flat frames` (default 20), a text field `Flat label`, the
  button `Take flats` (`button button-primary`), disabled while the label is empty, which
  calls `postCalibrationFlats`, and the `quiet` line
  `Light the flat panel first. The console finds the exposure; the flats' darks follow with the telescope covered.`
- **The set.** By `set.kind` and `set.state`, a `status` unless said otherwise:
  - `dark`, `offset`: `Taking darks: reading the offset level`;
  - `dark`, `frames`: `Taking darks: {taken} of {wanted} at {exposure_ms} ms`;
  - `flat`, `search`: `Taking flats: finding the exposure`;
  - `flat`, `frames`: `Taking flats: {taken} of {wanted} at {exposure_ms} ms`;
  - `flat`, `needs_darks`:
    `Flats in hand: {wanted} at {exposure_ms} ms. Cover the telescope, then take their darks.`,
    with the buttons `Take the flats' darks` (`button button-primary`), which calls
    `postCalibrationFlatDarks`, and `Drop these flats` (`button`), which calls
    `deleteCalibration(set.id)`; with `set.error`, also an `alert`
    `The flats' darks stopped: {error}.`; and with `set.not_dark` as well, the `quiet`
    line of the dark test below and a button `Take the flats' darks anyway` (`button`),
    which calls `postCalibrationFlatDarks(true)`;
  - `flat`, `flat_darks`: `Taking the flats' darks: {darks_taken} of {wanted}`;
  - `done`: `Master dark saved from {wanted} darks.` or
    `Master flat {label} saved from {wanted} flats.`;
  - `failed`: an `alert`, `Darks stopped: {error}.` or `Flats stopped: {error}.`; for a
    `dark` set with `set.not_dark`, also the `quiet` line of the dark test,
    `Only if the telescope is covered: a warm sensor or a long exposure can read above the offset level.`,
    and a button `Take them anyway` (`button`), which calls
    `postCalibrationDarks(set.wanted, true)`;
  - `stopped`: `Darks stopped.` or `Flats stopped.`

  In `offset`, `frames`, `search` and `flat_darks` a button `Stop` (`button`) calls
  `deleteCalibration(set.id)`. In those four states and in `needs_darks`, `Take darks`
  and `Take flats` are disabled.
- **A refused request** shows its words in an `alert`, as the Imaging page's other
  requests do. Each request that succeeds calls `onChange`; a refused one does not.

Numbers are written with `String(n)`, so 5000.0 reads `5000` and 366.634 `366.634`.
Write each phrase a test asserts on one line of JSX, so `textContent` keeps it whole.
Only existing classes and tokens are used; `index.css` does not change. The field drafts
are component state and survive the page's one-second reads.

`components/ImagingPage.tsx` (031 US4): as the page's last block, after the Imaging
frame section, its picture, 032's Imaging solve block and 033's `<FocusBlock … />`,
which 033 renders right after 032's (`specs/033-focus-reading-and-aid/plan.md` § The
page),
`{view.state === "ready" && view.calibration && <CalibrationBlock
calibration={view.calibration} onChange={refresh} />}`, `refresh` being the hook's. A
view without `calibration` renders no block, which is what keeps the component tests of
031, 032 and 033 as they are.

### `DESIGN.md` and `.impeccable/design.json` (US9)

After the last entry that 031, 032 and 033 add for the Imaging page, 033's Focus block
entry by their drafts (031's US4 puts the first of them after the Recording entry,
`DESIGN.md:350`), a plain `###` heading
and one paragraph (the `>` marks only set the text apart here):

> ### Calibration block — built
> On the Imaging page, under the picture, while the imaging camera is ready: a section
> titled Calibration. One ink line says which masters the latest imaging frame would get
> and what is missing, with an alert when the camera has turned since the flats. A ruled
> list names each master dark and master flat with its settings, frame count and date,
> each with a secondary Remove button. Two short forms follow, Dark frames with Take
> darks, and Flat frames and Flat label with Take flats, each of 44px ruled fields with
> a primary button and one quiet line saying what to do first. While a set is taken one
> status line counts its frames beside a secondary Stop button; flats in hand wait with
> Take the flats' darks and Drop these flats; a refusal or a failed set is an alert, and
> a set the dark test refused adds one quiet line and a secondary button to take the
> darks anyway. The block holds no picture. Existing ink, paper and red-only night
> tokens only.

`.impeccable/design.json` gains one entry in `components`, after the last Imaging page
component already there and formatted as its neighbours: `name` `Calibration block`, `kind` `custom`,
`refersTo` `button`, a `description` that says the same in one sentence each, and `html`
and `css` snippets under the class `ds-calibration`, using only `var(--color-ink)`.

## Shapes that existing tests pin

These are not changed by any story. The tests of 030, 031, 032 and 033 named here do not
exist at 49bb210; each is a scenario of that spec's `spec.md`, and each is re-read in
its code when this trio is re-anchored (§ Prerequisites 2 and 5).

- **A simulated imaging frame with nothing switched on.** 030's
  `crates/simulator/tests/imaging.rs` asserts exact samples (its US2-S4 a mean of 165.0,
  its US2-S7 samples of exactly 160) and equal bytes from cameras built alike. With the
  default sensor pattern and light path no arithmetic is added to a sample: V and D are
  not multiplied in, and nothing is added for hot pixels or columns (US1-S1).
- **`ImagingOptics` and `Pointing`.** Neither gains a field: tests build
  `ImagingOptics` with its defaults or by literal, and `Pointing` is `Copy`
  (`crates/simulator/src/optics.rs:64`). The new state is a cell of its own.
- **`GET /api/imaging` on a console without imaging is exactly `{"state":"off"}`**, and
  a view outside `ready` holds `state`, and `error` for `error`, and nothing else (031's
  US1-S4). `calibration` is set only on a `ready` view.
- **The session's `imaging` equals the body of `GET /api/imaging`** (031's US1-S1), and
  `get_imaging_camera` answers that body (031's US5-S2). Both are the one view, so
  both carry `calibration` alike.
- **`camera` and `status` are compared whole and `settings` and `frame` key by key**
  (031's shared setup). No key is added to any of the four.
- **`solve` and `relation` are compared key by key** (032's shared setup), and
  `RelationView` is `#[non_exhaustive]`: this spec reads `relation.angle_deg` and adds
  nothing to either. No test of 032 asserts that the imaging view has no `calibration`
  key.
- **031's `decode` refuses `BITPIX` −32** with `unsupported FITS BITPIX -32` (its
  US2-S10). `encode` and `decode` are not edited; real images have functions of their
  own.
- **Frame ids and files.** Each frame of a set takes the console's next id and is saved
  as 031 saves every frame, so its numbering, its `.part` rule and its
  already-exists rule hold unchanged.
- **`Imaging::take`, `ended`, `set` and `abort`, and 031's lease,** keep their
  signatures and refusals. The sets take the camera through 031's lease
  (`Imaging::lease`, `Lease::set`, `Lease::take`) and call `abort`, `ended` and `view`
  as anyone may; this spec adds nothing to `Imaging`'s lease or to its refusals
  (§ The camera's lease; spec owner decision 23).
- **033's focus aid.** Its frames keep the purpose `focus` and are still saved and
  removed by the aid as 033's tests assert; `match` only passes over them. Its lease,
  under `the focus aid`, is refused while a set holds the camera by 031's code, and
  033's tests, which take no set, are unchanged.
- **The MCP tool list as tests read it.** They look tools up by name; one calls every
  listed tool with `{}` on a console with no imaging and checks for the token
  (`crates/finder/tests/mcp/motion.rs:517`), so each new tool answers `{}` at once there
  (US8-S4).
- **Finder's test layout** (`crates/finder/tests/layout/main.rs:102`): no file at the top
  level of `crates/finder/tests`, no new file in `support`, and each binary's `main.rs`
  holding only its support include and `mod` lines
  (`crates/finder/tests/layout/main.rs:156`).
- **The Imaging page without `calibration`.** 031's
  `crates/finder/web/src/components/ImagingPage.test.tsx` renders views that have no
  `calibration`, and reads the page's statuses and alerts; the block renders nothing
  then. Its reads of `/api/imaging` (031's US4-S6) are the hook's, which the block
  only asks to read again after a request of its own.
- **The Imaging page at night.** 031's `crates/finder/web/e2e/imaging.spec.ts` counts
  off-red pixels in a full-page screenshot of the simulate project's page, which now
  shows the block: it must be exactly 0, which existing classes and tokens give. The
  block adds no `img`, so that test's count of the picture is unchanged.
- **Shapes this spec pins for its own later stories and for 035 to 037.**
  `CalibrationView` and `SetView` are `#[non_exhaustive]`; `MasterEntry` is not, and its
  keys are what `index.json` files on a disk hold: a key added later must have a serde
  default. `Library::add` never makes a map by itself (US3's and US4's scenarios assert
  `bad_pixels` null after `add`), so the map is attached by a call of its own. US6's
  tests read `SetView` key by key, so US7 adds keys and changes none. The scripted
  camera and the rigs are `pub(crate)` in the module that first needs them, and a later
  story imports them and edits nothing there. `HOLDER`, `a set of darks or flats`, is
  the name 035's and 037's takers hear in 031's refusal while a set is taken, and they
  pass that refusal on unchanged; no later spec asks this module whether a set is
  running.

## Decisions and the alternatives rejected

| Decision | Rejected | Why |
|---|---|---|
| Dark, then flat, then bad pixels, on the mosaic | Colour first; bad pixels first | The order of the open stackers the research note read; a hot pixel spread by colour interpolation cannot be repaired afterwards, and a repair made before the flat averages samples that are still unevenly lit. |
| A calibrated sub is `f32` and keeps values below zero | Clipping at zero | A clipped background has a mean pushed up by half its noise; the stack is floating point (spec owner decision 11). |
| Same-colour neighbours two samples away along the row and column, for every colour | The diagonal neighbours for green | One rule for all three colours and no need to know which diagonal is green; the samples used are the same plane's. |
| The median of the good neighbours | Their mean | A neighbour on a star's wing or a cosmic-ray hit does not drag the repair. |
| Hot pixels in the sub by a noise test, a ratio test and a count of lit touching samples | The noise test alone; the first two alone | A star's centre stands far above its neighbours in noise terms, so the noise test alone takes stars. The ratio is read two samples away, where a star 2.6 samples across or less is already at background, so the first two tests take star cores at binning 2 and on any coarser scale. The samples that touch the peak are lit by a star and not by a hot pixel, at any sampling (spec owner decision 14). |
| A touching sample is lit by a share of the candidate's height as well as by noises | A few noises alone | Inside nebulosity every touching sample is many noises above the frame's median; by noises alone no hot pixel would ever be found on the showcase target. The share keeps the test a ratio, as test 2 is. |
| One lit touching sample is allowed | None | Hot pixels come in touching pairs on real sensors, and the simulator's H holds one; a star lights at least four. |
| Greens normalised together, by the median | Each Bayer position by itself; by the largest value | Together, a real difference between the two greens is divided out; the median ignores hot pixels and dust where a largest value is set by one sample (spec owner decision 12). |
| A flat under 0.1 is not divided by | Dividing whatever the flat | A division by a dead or deeply shadowed sample makes a bright point that then needs repair anyway. |
| The spread of the master dark itself as the map's ruler | The single frames' noise over √N | A fixed column pattern is in the spread and so is not mapped; the frames' noise alone would map half of every patterned column. |
| No map above 1 percent of the frame | Repairing whatever is found | A map that large means the master is not a dark or the rule does not fit the sensor; repairing a hundredth of a frame is damage, and the search in the sub still runs. |
| The map attached by its own call | `Library::add` computing it | A test that adds a master must know what the entry will hold; US3's scenarios stay true when US5 lands. |
| Masters as 32-bit real FITS, by new functions in `fits.rs` | 16-bit masters; a private format; editing 031's `decode` | A mean of 20 frames is finer than a count; any astronomy program opens the file; 031's test pins `decode`'s refusal of −32. |
| One JSON index, the FITS cards a copy | The cards as the only truth | Listing and matching read one small file, not every master's header. |
| An unreadable index is moved aside at the next write | Refusing every write; rebuilding it from the files | The observer can still make masters that night, and nothing is destroyed. |
| A set takes its frames through 031's frame path, `Lease::take` being `Imaging::take` for the holder, and reads the files back | A second frame path that does not save | 031 has one path and its tests pin it; the files are worth keeping (spec owner decision 2). |
| The offset frame: one dark of the shortest exposure | The camera's offset setting; a fixed count | The setting's worth in counts is the camera's own and unknown; a measured level needs no model (the research note has not established this camera's scaling, § 13). |
| The dark test in units of the offset frame's noise | Counts; a share of full scale | Noise scales with gain, binning and the camera's sample scaling, so one factor serves every camera. |
| A third part of the dark test: bright samples that are not hot pixels | The level alone | The level passes an uncovered camera whenever the sky adds under two noises, which this spec's own simulated camera does at gain 0 and 200 ms; stars are what an uncovered frame has and a covered one has not. |
| `anyway`, recorded as `checked` false | No way past the test; a setting that switches it off | A refusal of real darks must not end a night's calibration on a camera nobody has measured; a per-request key leaves the test on by default and leaves a mark in the master (spec owner decision 4). |
| A sensor more than 1.0 °C from its set temperature fails the set, `anyway` or not | Keying by set temperature alone; waiting inside the set | The master would be kept, and matched, under a temperature it was not made at; a wait of unknown length inside a set hides a cooler that cannot reach its setting (spec owner decision 24). |
| A set's frames are held to its first and to the exposure asked for, by `FrameKey::agrees` | Comparing the settings but not the exposure | The lease keeps the console's own settings changes out during a step, but not a camera that changes a setting by itself, nor a change made between the two steps of a set of flats; darks averaged at two exposures would be kept as one. |
| Set checks before the frame-in-flight check | The frame first | A set being taken is nearly always exposing, so the other order would never say that a set is running. |
| A set holds 031's lease from start to end, under `HOLDER`, taken last of its request's checks | A lock of this spec's own (`busy()` for others to ask) and a restore that waits out a frame slipped in between two of a set's frames | The plan document's amendment 1: one lease on 031's imaging state, taken and released by each later spec, which adds no lock of its own. With the lease nothing can slip in, and every other taker hears the set's name without asking this module (spec owner decision 23). |
| The frame-in-flight check only while nobody holds the lease | Always; never | 031's single frame holds no lease, so only this check names it; a holder's frame is better named by its holder, as 033's `focus_start` and 035's `start` do. |
| Flats that wait hold no lease | Holding it until their darks are taken | The observer covers the telescope in between, which can take minutes, and may want a frame or a focus run meanwhile; a lease kept by no running task would also outlive a restart's memory of the flats. |
| A set puts the exposure back through its lease, drops it, and only then writes its final state | Writing the state first | A test, or a client, that waits for the state then finds the setting as it was and the camera free. |
| Flats, then their darks, as two requests on one route | One request with a wait; the darks first | A person has to cover the telescope in between, and the exposure is not known before the flats. |
| A failed second step returns to `needs_darks` | Dropping the flats | The usual cause is a cover not yet on; the flats are still good. |
| The search follows a straight line through its last two unsaturated frames | Bisection; a table of exposures | A sensor is linear below saturation, so two frames usually land the third; bisection takes more frames. |
| `DELETE` also stops and drops a set | A fifth route | The plan document names four routes for this spec (spec owner decision 10). |
| The calibration object rides in the imaging view | A second poll by the page | The page already reads `GET /api/imaging` once a second; one view means one refresh. |
| `Imaging` holds the calibration weakly and asks it for its part of the view | Each handler adding the part | Every answer that is a view carries it, the MCP tools' included, with one edit. |
| `match` from the latest saved frame | From the settings in force | A frame's real size is its own (030 FR-003); a guess from the region and binning can be wrong on a real camera. |
| `match` passes over `dark`, `flat` and `focus` frames | Passing over darks and flats only | 033's focus aid takes a small region about once a second and removes each frame when it saves the next; a match on one would describe a tile that is about to be deleted (spec owner decision 22). |
| The key's set temperature from `frame.set_temp_c` | From the settings when the frame is first seen | The frame's own value is the one in its file, so the view's key and the file's agree whatever changed in between (spec owner decision 22). |
| The key of that frame is kept until a newer sub is saved | Only while the very latest frame is a saved sub | Once frames follow one another the latest frame is exposing most of the time, and the page would say nothing matches. |
| The imaging angle read from the imaging view's `relation` | A setter for a later spec to call | 032 lands first and already carries it; a setter with no caller leaves the warning dead and lets the factory run this epic beside 032 or 033 (spec owner decision 20). |
| A flat matched on camera, size, origin, bits, pattern and binning alone | The plan document's key as it was first written: gain, offset and set temperature as well | Settled by the owner on 2026-11-17, a departure he approves and the plan document now writes: a master flat has its own dark taken off, so gain, offset and temperature do not change it, and flats made once serve every gain, with the cooler on or off (spec owner decision 7). |
| A turned camera warns and the flat is still applied | Treating it as missing | Most of a flat is the radial fall-off, which a turn about the axis leaves alone (spec owner decision 8). |
| No Playwright test | One in the simulate project | It needs an edit of `playwright.config.ts`, the simulate console has no cover to take darks under, and the block has no picture for the night rule to bite on beyond what 031's full-page check counts. |
| The scripted camera wraps the stand-in | A camera written from nothing | Settings, timing, `Busy` and abort come with it; the test writes only the level. |
| Rigs written once and imported by later test modules | Each module writing its own | The same hundred lines in three modules cost each story's diff budget, and the layout test allows a module to use another's `pub(crate)` items. |
| Nine stories: the library on disk apart from the library in the console | One library story | One story held the real-image codec, the index, both matching rules, the console's state, the view, two routes and a README section; it would have passed 64 KiB, and a split found mid-epic needs a relaunch. |

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

  Every story runs both, and the judge sees their results. `--locked` holds for every
  story as it stands: no story changes `Cargo.lock`. The stories' development checks run
  one finder module with `cargo nextest run -p finder --locked --test <binary> <module>::`
  and the simulator's new file with
  `cargo nextest run -p simulator --locked --test imaging_calibration`.
- **CI in the merge queue** runs every `ci.yml` job, including `cargo-deny`,
  `cargo-machete`, the build on Rust 1.96 and the finder-page checks. This spec adds no
  dependency, so the first two see nothing new. Coverage runs only by hand.

## Prerequisites before this spec is flipped to `ready`

1. **`030-imaging-camera-link`, `031-imaging-frames-on-console`,
   `032-imaging-solve-and-aim-point` and `033-focus-reading-and-aid` have landed.** Not
   met: all four are `state: draft` in the drafting worktree, and no story of any of
   them is on `main`. All four are in
   this spec's `depends_on_landed`, the plan document's settled order, so the factory
   cannot dispatch this epic beside any of them.
2. **This trio is re-anchored against `main` after 030, 031, 032 and 033 land.** Not met.
   Every interface of 030 and 031 named in § Interfaces is read again in the code that landed:
   `Imaging::take`, `ended`, `set`, `abort` and `view`; the lease, `Imaging::lease`,
   `Imaging::holder`, `Lease::set` and `Lease::take`, and its six points of § The
   camera's lease; `ImagingView`, `FrameView` with its `set_temp_c`, `SettingsEdit` and
   `Refusal`; where `Imaging` builds its view; `with_imaging`; `routes()`; `fits.rs`'s
   card formatting, `decode`, `CardValue` and `utc`; the saved file's cards; the ten
   steps of `SimImagingCamera::expose`; `ImagingPage.tsx`, its hook and its tests'
   fixtures; the stand-in console rig and the stand-in camera. Of 032:
   `ImagingView::relation`, `RelationView::angle_deg` and when it is present; the
   solving rig, `SimulatedSolver` and `with_imaging_solver`; how a first aim reading and
   three more after a turn of the camera move `relation.angle_deg` (US7-S6); and that
   `DELETE /api/aim` takes the measured angle out of the view. Of 033: the purpose
   `PURPOSE` and its module path, the last writers of § Prerequisites 5, and where its
   Focus block sits. Two points are settled then and written into the trio:
   - whether a saved file's `EXPTIME` is the frame's own exposure or the one asked for.
     Either way a setting changed between two frames shows in it, which is all
     `FrameKey::agrees` needs;
   - whether an exposure-only `Lease::set` can wait on anything. § **The set's lease**
     assumes it answers at once, as 031's `set` does in its draft.

   Every plain path of a file 030, 031, 032 or 033 creates gains its line, the shared writes
   of § Refinement Record are checked against the stories that really wrote each file,
   the citations checked at 49bb210 that 030, 031, 032 and 033 move are re-read
   (`crates/finder/src/api.rs`, `crates/finder/src/mcp/tools.rs`,
   `crates/finder/README.md`, `DESIGN.md`, `crates/finder/src/lib.rs`,
   `crates/simulator/src/optics.rs`, the two `main.rs` of the test binaries), and the
   validator is re-run.
3. **The owner decisions are settled and the approvals given** (spec § Owner decisions,
   § Owner approvals). Not met: of the twenty-four, 17 and 23 are settled by the plan
   document's amendments 2 and 1 and ask nothing more of the owner, and 7 was settled
   by the owner on 2026-11-17 and is written in (the wider match of a master flat); the
   other twenty-one are open. Nothing left reaches outside this trio: the line of the plan
   document's Deferred list that amendment 2 implies is there now, written by the
   series audit's shared edit of that document, not by this trio.
4. **A probe build of every story.** Not met. No story has been compiled or run
   (§ Refinement Record). The probe settles, at the least: each story's diff against
   64 KiB, US3, US6 and US7 first; every worked number of spec.md that rests on the
   simulator's noise or its seeded field (US1-S2, US2-S8, US2-S11, US5-S4, US6-S1 to
   US6-S3, US7-S5, US7-S6, US7-S7), and above all the count of lit samples in US6-S3's
   uncovered frame, which was worked with a model of 030's camera and not with its
   code; that `Imaging` builds its view in one place; that the router takes `darks` and
   `flats` beside `{id}`; and that no scenario of 030, 031, 032 or 033 changes.
5. **033 is read.** Met against 033's draft in this worktree
   (`specs/033-focus-reading-and-aid/`), and read again in its code at the re-anchoring
   (prerequisite 2). The four questions an earlier copy left open, answered and written
   into the trio:
   - (a) the story of 033 that last writes each file this spec shares with it, for
     § Refinement Record's shared writes: `crates/simulator/src/optics.rs` and
     `crates/finder/src/api.rs`, 033 US1; `crates/finder/src/imaging/mod.rs` and
     `crates/finder/src/imaging/routes.rs`, 033 US5; `crates/finder/src/lib.rs` and
     `crates/finder/tests/console/main.rs`, 033 US8; `crates/finder/README.md`,
     `crates/finder/src/mcp/tools.rs` and `crates/finder/tests/mcp/main.rs`, 033 US7,
     which follows US8; `DESIGN.md`, `.impeccable/design.json`, `crates/finder/web/dist`,
     `crates/finder/web/src/api.ts` and `ImagingPage.tsx`, 033 US6. 033 does not write
     `crates/simulator/src/imaging.rs`, `crates/simulator/src/lib.rs` or
     `crates/finder/src/imaging/fits.rs`, whose last writers stay 030's (the two
     simulator files, 030 US2) and 031's (`fits.rs`, 031 US2);
   - (b) where 033 places its block: its Focus block follows 032's Imaging solve block
     (033 plan.md § The page), so the Calibration block, placed after it, is the page's
     last block until 035 places its own after it;
   - (c) whether a set should refuse on 033's `Imaging::focus_running`: no. The lease
     refuses a set while the aid holds the camera, with `the focus aid` in the words,
     and refuses the aid while a set holds it (§ The camera's lease, 2); no refusal of
     this spec's own is added;
   - (d) whether 033 takes frames without saving them: no. The aid saves every frame
     through `Lease::take` and removes each when it saves the next (033 plan.md § The
     aid on the console, "Removing the aid's previous saved frame"), so `Imaging` has no
     unsaved path and spec owner decision 2 stands as written; `match` passes over
     those frames (spec owner decision 22).

   035, 036 and 037 land after this spec and name it in their `depends_on_landed`.
6. **The plan document's `GLOSSARY.md` rewrite is on `main`.** Not met: the rewrite is
   an uncommitted change in the drafting worktree. T022 inserts **Bad-pixel map** "after
   the **Calibration library** entry" and T027 **Offset level** "after the **Flat**
   entry", and neither entry exists at 49bb210; the four `GLOSSARY.md` lines this trio
   cites are the rewrite's. It lands with the plan document, before or with the first
   of the eight specs. The three entries of spec owner approval 6 then reach the
   glossary through US5, US6 and US7; that approval is part of prerequisite 3.
7. **The research note holds what is cited.** Met at drafting: §§ 1, 6, 8, 9, 11 and 13
   of `docs/research/2026-11-15-imaging-camera.md` are present in the drafting worktree
   under those numbers and titles. Re-checked at the re-anchoring, since the note is
   written beside this trio.
8. **No operator-staged file is needed.** Met: every fixture is made by a test from the
   simulator or a scripted camera.
9. **The scratch-folder helper.** Met: `crate::support::Scratch` is on `main` at
   49bb210, the worktree's HEAD, and is cited at `crates/finder/tests/support/mod.rs:12`.
10. **Outside the factory, after the spec lands:** rebuild the console; then the field
    checks of spec.md § Assumptions, the first of which is whether real darks pass the
    three parts of the dark test at the owner's exposures.
11. **031's trio carries the imaging camera's lease and `FrameView::set_temp_c`.** Not
    met on `main`, where no trio of this series is committed yet; met in this worktree:
    the series audit gave 031's spec.md the lease as US6 and the frame's set temperature
    in its FR-009, and 031's plan.md, from which this trio takes 031's interfaces, now
    has both, the lease in § The camera lease and `set_temp_c` in `FrameView`
    (§ Receiving and saving). Its names are the ones § The camera's lease uses, name for
    name, and its rules give the six points that section relies on. US4 cannot be built without
    `set_temp_c` (its `from_view` reads it), and US6 and US7 cannot be built without the
    lease (their code calls `Imaging::lease`, `Lease::set` and `Lease::take`, and their
    tests `Imaging::holder` and `Imaging::lease`). No story of this spec may add either
    to 031's imaging state. If 031 lands with either changed, every name in § The
    camera's lease is replaced with 031's own, and spec US4-S3, US6-S1, US6-S5, US6-S6,
    US6-S9 and US7-S1 are corrected if 031's lease promises something else.

## Plan traps

Lessons from building 012 to 028 on this factory, and this spec's own, each with its
mechanism and the check that keeps it from recurring. They bind every story.

1. **Gate evidence or nothing.** The judge credits only gate results and the story's
   diff. *Check:* every Then clause names a committed test that the `test` gate runs.
2. **A named command is read literally.** *Check:* commands are copied from
   § Verification.
3. **One evidence budget.** *Check:* each story keeps its diff under 64 KiB, measured
   with `git diff <base> -- . ':(exclude)crates/finder/web/dist' | wc -c`.
4. **The sandbox has only the gates' tools.** *Check:* no task runs a tool outside them.
5. **A spec fixed mid-epic needs a relaunch.** The judge grades the spec as dispatched.
   *Check:* no builder edits `specs/034-dark-and-flat-frames/` or `.specify/`.
6. **An oracle copied from the code proves nothing.** *Check:* a test reads a master with
   a FITS reader of its own, never `decode_real`, except where `decode_real` is the thing
   tested (US3-S1); compares a master with the mean it takes itself of the saved frames;
   computes V, D and the column pattern from the formulas of spec.md's shared setup; and
   builds its masters by hand for `calibrate`. No test reads a master through the
   `Library` to check what the `Library` wrote.
7. **A frame that must not change.** Rewriting 030's sums to make room for the new terms
   (the stars and the sky gathered in another order, a factor applied to a sum where it
   was applied to each part) moves a sample by a count wherever a rounding sits near a
   half. *Check:* 030's steps keep their arithmetic and order; the new terms are steps
   of their own, skipped at their defaults; US1-S1 compares bytes, and 030's exact
   scenarios run unchanged.
8. **`Pointing` is `Copy`.** A `Vec` in it stops the simulator compiling in a dozen
   places. *Check:* the pattern and the path are in a cell of their own.
9. **A later story must not falsify an earlier story's test.** A library that made a map
   inside `add` would turn US3's and US4's `bad_pixels` null into a number when US5
   lands. *Check:* the map is attached by `attach_bad_pixels`, and US5-S5's last sentence
   asserts that a master only added keeps null. The same holds for `SetView`: US7 adds
   keys and changes none.
10. **The lock and the callback.** `Imaging::view` asks `Calibration::view` for its part,
    and the sets call `Imaging`. A lock of this module held across either is a deadlock
    on a multi-thread runtime and a hang in a test. *Check:* the last sentence of § The
    console's calibration state; the set's state is copied out under its lock and the
    lock dropped before any await or any call into `Imaging`.
11. **The setting restored and the lease dropped before the state is final.** A test
    that waits for `done` and reads the exposure or `holder()` at once would otherwise
    race the ending. *Check:* every end of a set goes through **the ending** of § The
    set's lease, `lease.set` of the exposure, then the `Lease` dropped, then the final
    state written; US6-S1, US6-S3, US6-S6 and US7-S2 read the setting right after the
    state, and US6-S1, US6-S6, US6-S9 and US7-S1 read `holder()` there.
12. **A median of a uniform frame has no spread.** Without the floor the dark test's line
    would sit on the offset level and refuse a frame one count above it. *Check:*
    `NOISE_FLOOR`, and US6-S4's first four cases, exact on either side of the line.
13. **A map of a tiny frame.** One percent of 100 samples is one, so two bad samples in a
    10 × 10 test frame are already too many. *Check:* US5-S1 and US5-S5 use 20 × 20;
    US5-S3 uses 10 × 10 on purpose.
14. **Paused time and the blocking pool.** Tokio's paused clock runs on while a blocking
    task works, so a timeout wrapped round one fires in a test and never in the field.
    *Check:* no blocking call here is inside a `tokio::time::timeout`; the only waits
    are 031's own, inside `take` and `ended`. For the same reason no scenario asks a
    test to slip a request in between two frames of a set: when a set's task yields
    there is not a thing a test can time. None needs to: the lease is held between two
    frames exactly as during one, so US6-S9 asks while a dark exposes and proves the
    gap with it. What the lease cannot keep out is tested where it can be driven:
    `FrameKey::agrees` as a pure function (US3-S3), and a frame whose gain differs
    (US6-S7).
15. **A static segment beside a parameter.** `…/calibration/darks` and
    `…/calibration/{id}` are different routes with different methods. *Check:* the four
    routes are registered as § The routes lists them, and US4-S1's `DELETE` and US6-S5's
    `POST` both pass in one router.
16. **The Imaging page's fixtures have no `calibration`.** A block that rendered a status
    for a missing object would break the component tests of 031, 032 and 033. *Check:*
    the block is
    rendered only when the view carries the object, and US9-S1 asserts its absence.
17. **The bundle must be rebuilt.** *Check:* US9 commits `crates/finder/web/dist` after
    `npm run build`, so that the gate's `git status --porcelain -- dist` finds nothing.
18. **Each worktree builds on its own.** *Check:* no story sets `CARGO_TARGET_DIR`, and
    test folders come from `crate::support::Scratch` under `CARGO_TARGET_TMPDIR`.
19. **027's layout.** A test file at the top level of `crates/finder/tests`, or a file
    added to `crates/finder/tests/support`, fails the `layout` test, and a helper a
    module does not use fails `clippy -D warnings`. *Check:* each new test is in the
    module § Technical Context names, declared by one alphabetical `mod` line, holding
    only the setup its own tests use. A rig two modules need is `pub(crate)` in the
    first and imported by the second; `pub(crate)` on an item its own module does not
    use is still an unused item, so nothing is declared ahead of its first use.
20. **Words the page and the routes share.** A refusal reaches the page as the route
    wrote it. *Check:* the page shows `error` unchanged inside its own sentence, and
    US9-S4 uses a short stand-in error, not a copy of a route's text. The page never
    reads an `error` to decide what to offer: `Take them anyway` hangs on the view's
    `not_dark`, not on the words.
21. **A number printed with a fraction.** `{}` of 160.0 prints `160`, but of a mean it
    would print a long fraction. *Check:* the dark-test texts print medians and a
    count, which are whole; US6-S4 asserts them exactly. The temperature text prints
    with `{:.1}`, so −10 reads `-10.0` (US6-S7).
22. **The refusal that hides another.** A set being taken is nearly always exposing. If
    the frame in flight were looked for first, a request made while a set runs would
    never be told that a set is running. *Check:* the order of § **`take_darks`**, and
    US6-S5, US7-S4 and US8-S3, each of which asks while the set's own frame exposes.
23. **A star taken for a hot pixel.** The nearest sample of a star's own colour is two
    samples from its peak, where a sharp star is already at background. *Check:* test 3
    of § **`find_hot_pixels`** reads the samples that touch; US2-S9 keeps stars 0.8 and
    0.5 samples wide, and US2-S11 a simulated star at binning 2 beside three hot
    pixels.
24. **A view that forgets.** `match` taken from the very latest frame alone is absent
    whenever that frame is exposing, a dark or a flat. *Check:* the key last matched is
    kept in the service; US4-S3 reads `match` through a dark and through an exposure.
25. **An avoided word in a refusal.** The glossary lists "calibration set" under what to
    avoid, and the constitution binds its words in messages. *Check:* the refusal reads
    `a set of darks or flats is already being taken`, `HOLDER` is
    `a set of darks or flats`, and no string, key, card or document of this spec says
    "calibration set". The glossary's Imaging frame entry avoids "frame" alone in what
    the observer reads: the dark test says `the darks are not dark`, and the search
    `the flat's level does not rise with the exposure`.
26. **A frame-in-flight check that hides the holder.** Looked for first, or always, a
    frame in flight would answer `an exposure is in progress` while the focus aid or a
    stack holds the camera, and the observer would never learn who holds it. *Check:*
    check 5 of § **`take_darks`** runs only while `imaging.holder()` is `None`; US6-S9's
    second rig asks while a lease of the test's own exposes, and US6-S5 asserts
    `holder()` `None` where the single frame answers.
27. **A lock of this spec's own.** A `busy` flag, or a mutex held across the set,
    beside 031's lease would be the second lock amendment 1 forbids, and could disagree
    with the lease after a panic. *Check:* the `Lease` is owned by the set's task and
    never kept in the set's state; the only "a set is running" check is refusal 3,
    which reads the set's own state and exists to give its own words; nothing outside
    this module asks it.
28. **A lease kept while flats wait.** The step's task ends at `needs_darks`; a lease
    moved anywhere else would outlive it and lock the camera until the darks are taken.
    *Check:* the first step's task drops its `Lease` before it writes `needs_darks`,
    and US7-S1 reads `holder()` `None` there.

## Risks

| Risk | Mitigation |
|---|---|
| 030, 031, 032 or 033 lands with interfaces other than its draft's. | § Prerequisites 2 and 11: the trio is re-anchored to their code and re-validated before the flip. The seams are few: four methods of `Imaging` and its lease, the view with `frame.set_temp_c` and `relation.angle_deg`, the saved file's cards, `fits.rs`'s card writer, the simulator's frame steps and 033's `PURPOSE`. |
| 031's lease is in 031's plan.md only since the series audit, and not yet on `main`. | § Prerequisites 11. The shape here is 031's plan.md § The camera lease, the one 033's plan quotes from 031's US6, name for name, and the scenarios assert of its refusal only a 409 with the holder's name. |
| A story's diff passes 64 KiB. | Not measured: no probe build. The library story was split in two before the flip (US3 the library on disk, US4 the library in the console), and the rigs are written once and imported. The largest that remain are US3, US6 and US7; US6 grew in review by the third dark test, `anyway` and the temperature rule, and lost the waiting restore to the lease. If the probe finds US6 over the limit it splits where its scenarios do: the set and its tests (US6-S1 to US6-S5), then stopping, failures and the lease (US6-S6 to US6-S9). |
| Worked numbers that rest on simulated noise are wrong by a count. | Each has a tolerance and a worked example; the probe build of § Prerequisites 4 runs them. US2-S8 draws no stars, so no seeded star can sit in a block; US7-S7's blocks are 10,000 samples and use medians. |
| The dark test refuses the owner's real darks. | `anyway` takes the set and the master says `checked` false; spec owner decision 4 and the first field check. Each part of the test is one constant: `DARK_RISE_SIGMAS`, `COVER_SHARE`, `LIT_SHARE` with `LIT_MIN`. |
| The dark test passes an uncovered tube. | The lit test refuses a frame with stars in it, and US6-S3 proves it on the simulated camera where the level passes. What is left is a field with almost no stars; written in spec.md § Edge Cases and in the README. |
| The lit test's worked number is wrong. | About 1200 lit samples against 101 allowed in US6-S3 was worked with a model of 030's simulated camera, twelve times the line; the probe build runs it on the camera itself (§ Prerequisites 4). |
| The real camera's full scale is not 65535. | The research note has not established it (§ 13). The search then fails with words that say where the level stopped, and nothing is made; spec owner decision 9 names the alternative of a full-scale setting. |
| A master is applied to a sub taken at another white balance. | The plan document's amendment 2 (spec owner decision 17, settled): 030's INDI adapter sets each white-balance control neutral each time it connects, so every frame of every night is on the same footing; no spec records the controls per frame, and nothing here can see them. A camera reached by another adapter later must do the same. |
| Two takers of frames run at once. | The plan document's amendment 1 (spec owner decision 23, settled): a set holds 031's lease from its start to its end, so every other taker is refused with `a set of darks or flats` in the words, and a set is refused while another holds the camera. The one gap is a single frame started on another thread in the instant before the set holds the lease; the set then fails at its first setting, before it has taken anything (spec § Edge Cases). |
| Disk: a set of 20 at the worked example's size is about 650 MB of frames, and a master 65 MB. | Spec owner decision 2. The page and the README say that each dark and flat is kept. Nothing is deleted by the console. |
| Memory at the worked example's size. | A set holds one sum of `u32` and one frame as `f32`, about 130 MB together, and a flat set that waits holds its sum, 65 MB. `Library::calibrate` keeps one dark and one flat read, 130 MB. Stated for 035's budget; not measured. |
| `Library::calibrate` reads files on the caller's thread. | Stated in its interface: a caller on the runtime uses the blocking pool. 035 is its first caller in product code. |
| The search for hot pixels in the sub takes a star's core. | Test 3 keeps any star at least 1.4 samples across at half its height, at any binning. Sharper stars, stars under about 30 noises at their peak, three hot pixels that touch and hot pixels in bright nebulosity are the stated limits; the rule applies only without a map (spec owner decision 14). |
| 032, 033 and 035 to 037 write `mod.rs`, `routes.rs`, `tools.rs`, the README, the Imaging page and the simulator's optics too. | 032 and 033 are in `depends_on_landed` and land first, and § Refinement Record's shared writes name the story of each that last writes a file before this spec (§ Prerequisites 5); 035 to 037 name this spec in theirs. § Prerequisites 2 re-anchors the trio to what the three wrote. |
| The imaging angle is absent on most nights. | It is carried only while the aim point is the measured one (032). Flats taken with a tapped aim point record no angle and nothing warns; the flat label is then all the observer has (spec owner decision 20). |
| US7-S6 leans on 032's behaviour. | It needs a first aim reading near 17°, a median near 19.5° after three more, and no angle after `DELETE /api/aim`, all from 032's draft. The rule itself is proved without 032 in US3-S4; § Prerequisites 2 re-reads the three points in 032's code. |
| A sensor that cannot reach its set temperature. | A set fails and says so, with or without `anyway`; the observer sets a temperature the cooler holds (spec owner decision 24). |
| The night rule and a new block. | The block holds no picture and uses existing classes; 031's Playwright check of the whole Imaging page at night counts its pixels in the simulate project. |

## Refinement Record

Drafted on 2026-11-15 by the grilling session against `main` at dbc2b5b, from
`docs/plans/2026-11-15-specs-030-037-imaging.md`, the research note's § 9 and the drafts
of `030-imaging-camera-link`, `031-imaging-frames-on-console` and `032-imaging-solve-and-aim-point` as they stood in the
drafting worktree that day; revised the same day after a second agent's review of the
trio; and revised again after the series audit of 030 to 037, with every citation
checked at 49bb210. **No probe build has been run.** No story of this trio has been
compiled, linted or tested, no interface above has been built, and no diff has been
measured: the sessions wrote documents only. This spec also builds on code that does
not exist: every type and function of 030, 031, 032 and 033 named here is its draft's, not
something that was read in a file, and 031's lease and `FrameView::set_temp_c` are as
031's plan.md now gives them for its US6 and FR-009, in the shape 033's plan quotes
(§ The camera's lease, § Prerequisites 11).

Every `path:line` names a file that exists at 49bb210, the worktree's HEAD, and was
read there, but for four lines of `GLOSSARY.md`, which are lines of the plan document's
uncommitted rewrite of that file (§ Prerequisites 6). Between dbc2b5b and 49bb210 only
028's US5 landed, in test files; of the citations, one had moved and one was wrong at
both commits, and both are corrected: the MCP token test, cited at line 521 of
`crates/finder/tests/mcp/motion.rs` before, is now `crates/finder/tests/mcp/motion.rs:517`,
its `fn` line, and the `## API` table's last row is `crates/finder/README.md:184`, line
167 being its heading. The scratch-folder helper is now cited at
`crates/finder/tests/support/mod.rs:12`. The uncommitted glossary moved under this
trio while it was re-anchored (the series audit added entries to it), so its hot pixel
entry is now cited at `GLOSSARY.md:331`; the other three glossary lines held. Every
number in a scenario was worked by hand or with a short script, and is shown as a
worked example where it is not obvious. The ones that rest on the simulator's noise or
its seeded field (a covered frame's standard deviation, the medians of US6-S3, the
search of US7-S5, the binned frame of US2-S11 and the count of lit samples in US6-S3)
were worked from 030's constants, the last two with a model of its camera written for
the purpose, and have not been run against the camera itself, which is not built
either.

What the review changed, so that a reader of an earlier copy knows: the library story
was split, making nine stories, and every story from the old fourth on moved up one
number; 032 and 033 joined `depends_on_landed` and the imaging angle is read from 032's
view; a master flat is matched by the plan document's key; the search for hot pixels in
a sub gained its third test; the dark test gained its third part and `anyway`; a set
checks the sensor against its set temperature and holds its frames to the exposure it
asked for; `match` stays on the latest sub; and the refusal order of the two taking
routes was put right. What the audit changed: a set holds 031's lease under `HOLDER`
from start to end, and `Calibration::busy`, `Calibration::restore_exposure`,
`RESTORE_TRIES` and the scenario of the waiting restore are gone, US6-S9 now proving
the lease; owner decisions 17 and 23 are settled by the plan document's amendments 2
and 1; `match` passes over `focus` frames and takes the set temperature from
`frame.set_temp_c`; the dark test and the search say `the darks are not dark` and
`the flat's level does not rise with the exposure`; and the shared writes below name
033's stories.

What the owner's answer of 2026-11-17 changed: a master flat matches on camera, size,
origin, bits, pattern and binning alone, so gain, offset and set temperature no longer
restrict it (spec owner decision 7, FR-015, US3-S4, and § The library's **Matching**).
A master dark's match is unchanged. No citation moved.

What a first refinement round must do is § Prerequisites 2, 4 and 11, and its findings
replace these paragraphs. Until then the record below is the draft's own account: its
tests are the scenarios, its writes are the files § Project Structure names, and its
shared writes name the merge edge that orders two stories of this spec on one file, or
the story that, by the drafts of 030, 031, 032 and 033, last writes the file before this
spec does: 033's where 033 writes it (§ Prerequisites 5), otherwise 032's, 031's or 030's. A
test file that 030, 031, 032, 033 or an earlier story of this spec creates cannot be listed
under `tests` as existing, because the validator resolves every such path in the
drafting worktree; each is named under `preserved_behavior` with what it must still
assert, and moves to `tests` at the re-anchoring, when the file exists. A file last
written on `main` by a spec outside this spec's `depends_on_landed` (`GLOSSARY.md`, by
025) has no entry for that spec, because the validator refuses one. The record's
`open_questions` are empty because the validator refuses a record that holds any; what
is not established is in § Risks and § Prerequisites 2, 4 and 11, and in spec.md's
owner decisions 4, 9, 20 and 24. Nothing in this record is an owner approval: those are
in spec.md, and flipping to `ready` gives them.

```yaml
version: 1
reviewed_revision: 49bb210fe4860dd98ba24727074e7bd3cc0d7124
stories:
  US1:
    tests:
      - path: crates/simulator/tests/imaging_calibration.rs
        status: new
        disposition: add
        assertion: "the default sensor pattern and light path are empty, and a frame taken with them set is byte for byte a frame taken without"
        decision: "spec.md:US1-S1"
      - path: crates/simulator/tests/imaging_calibration.rs
        status: new
        disposition: add
        assertion: "the cover leaves every sample at the offset level with noise off, and at its mean and read noise with noise on"
        decision: "spec.md:US1-S2"
      - path: crates/simulator/tests/imaging_calibration.rs
        status: new
        disposition: add
        assertion: "each hot pixel adds its rate times the exposure at its own place, covered or not, and a quarter of it to one binned sample"
        decision: "spec.md:US1-S3"
      - path: crates/simulator/tests/imaging_calibration.rs
        status: new
        disposition: add
        assertion: "the column pattern adds 2 times x mod 8 less 7 counts whatever the gain and exposure, and its mean to a binned column"
        decision: "spec.md:US1-S4"
      - path: crates/simulator/tests/imaging_calibration.rs
        status: new
        disposition: add
        assertion: "the flat panel replaces sky and stars, scales with its colour and the exposure, and falls to the corner by the vignetting formula"
        decision: "spec.md:US1-S5"
      - path: crates/simulator/tests/imaging_calibration.rs
        status: new
        disposition: add
        assertion: "a dust shadow dims the samples whose centre is within its radius, the sky falls to the corner too, and the cover hides panel and dust"
        decision: "spec.md:US1-S6"
      - path: crates/simulator/tests/frame_positions.rs
        status: existing
        disposition: preserve
        assertion: "the finder camera's simulated frames are unchanged"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "with the default sensor pattern and light path every simulated imaging frame is byte for byte what 030 makes (spec.md FR-001)"
      - "ImagingOptics, Pointing and the imaging truth gain nothing (plan.md Shapes that existing tests pin)"
      - "crates/simulator/tests/imaging.rs, a test file 030 creates, passes unchanged: 030's simulated imaging frames keep their exact samples, means and equal bytes with nothing switched on (plan.md Shapes that existing tests pin)"
    checks:
      development:
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
      - crates/simulator/tests/imaging_calibration.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/simulator/src/imaging.rs
        other_spec: 030-imaging-camera-link
        other_story: US2
        disposition: merge_order
        reason: "034 depends_on_landed 030, whose US2 created the simulated imaging camera"
      - path: crates/simulator/src/lib.rs
        other_spec: 030-imaging-camera-link
        other_story: US2
        disposition: merge_order
        reason: "034 depends_on_landed 030, whose US2 last changed the simulator's lib.rs"
      - path: crates/simulator/src/optics.rs
        other_spec: 033-focus-reading-and-aid
        other_story: US1
        disposition: merge_order
        reason: "034 depends_on_landed 033, whose US1 last changed optics.rs, after 030's US2, with the focuser"
    open_questions: []
  US2:
    tests:
      - path: crates/finder/tests/console/calibration.rs
        status: new
        disposition: add
        assertion: "Mosaic::from_samples reads 16-bit and 8-bit samples, each pattern's colours, a mono frame, and refuses a wrong length or bit depth; percentile takes the value at the floored index"
        decision: "spec.md:US2-S1"
      - path: crates/finder/tests/console/calibration.rs
        status: new
        disposition: add
        assertion: "calibrate subtracts the dark, then divides by the flat, reports applied or missing for each, searches the sub when no map is given, and keeps values below zero"
        decision: "spec.md:US2-S2"
      - path: crates/finder/tests/console/calibration.rs
        status: new
        disposition: add
        assertion: "normalise_flat divides each colour by its own median and answers the medians, and a sub divided by that flat keeps each colour's level"
        decision: "spec.md:US2-S3"
      - path: crates/finder/tests/console/calibration.rs
        status: new
        disposition: add
        assertion: "repair replaces a listed sample with the median of its good same-colour neighbours two away, at edges, with bad neighbours left out, for green and for mono, and counts those it cannot repair"
        decision: "spec.md:US2-S4"
      - path: crates/finder/tests/console/calibration.rs
        status: new
        disposition: add
        assertion: "find_hot_pixels finds an isolated sample and one six times its neighbours' height with nothing lit around it, leaves a star-like peak of ratio four, and calibrate searches the sub only when no map is given"
        decision: "spec.md:US2-S5"
      - path: crates/finder/tests/console/calibration.rs
        status: new
        disposition: add
        assertion: "a flat under 0.1 or not a number is not divided by; its sample is repaired and counted in low_flat"
        decision: "spec.md:US2-S6"
      - path: crates/finder/tests/console/calibration.rs
        status: new
        disposition: add
        assertion: "a master of the wrong length is refused with its words and the sub is left as it was"
        decision: "spec.md:US2-S7"
      - path: crates/finder/tests/console/calibration.rs
        status: new
        disposition: add
        assertion: "a simulated frame with hot pixels, a column pattern, vignetting and a dust shadow, calibrated with masters made from the simulator's formulas, has a level background at corner, centre and under the dust, and repaired hot samples"
        decision: "spec.md:US2-S8"
      - path: crates/finder/tests/console/calibration.rs
        status: new
        disposition: add
        assertion: "find_hot_pixels keeps stars 0.8 and 0.5 samples wide whose touching samples are lit, finds two hot pixels that touch, and leaves three that touch"
        decision: "spec.md:US2-S9"
      - path: crates/finder/tests/console/calibration.rs
        status: new
        disposition: add
        assertion: "lit_samples counts the samples more than 8 noises above their colour's median that are not hot pixels: 21 for the star, 0 for a lone hot pixel and for a pair, 9 for the mosaic of the fifth scenario, 0 for a flat mosaic"
        decision: "spec.md:US2-S10"
      - path: crates/finder/tests/console/calibration.rs
        status: new
        disposition: add
        assertion: "on a simulated frame at binning 2 with three hot pixels and a star, find_hot_pixels answers exactly the three binned places and calibrate repairs them and leaves the star's brightest sample"
        decision: "spec.md:US2-S11"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and each binary's main.rs holds only its support include and mod lines"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "nothing in 031's frame path, preview or routes changes: this story adds pure functions and one pub mod line"
      - "no dependency is added and no Cargo.toml changes"
      - "crates/finder/tests/console/imaging_frames.rs, a test file 031 creates, passes unchanged: 031's saved frames, their cards and fits::decode are unchanged (plan.md Shapes that existing tests pin)"
      - "crates/finder/tests/console/imaging_solve.rs, a test file 032 creates, passes unchanged: 032's solves read the imaging module as before (plan.md Shapes that existing tests pin)"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console calibration::"
        - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - crates/finder/src/imaging/calibration.rs
      - crates/finder/src/imaging/mod.rs
      - crates/finder/tests/console/calibration.rs
      - crates/finder/tests/console/main.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/imaging/calibration.rs
        other_spec: 034-dark-and-flat-frames
        other_story: US3
        disposition: merge_order
        reason: "US3 depends_on_merged US2, and adds pub mod library"
      - path: crates/finder/src/imaging/mod.rs
        other_spec: 033-focus-reading-and-aid
        other_story: US5
        disposition: merge_order
        reason: "034 depends_on_landed 033, whose US5 last changed the imaging module, after 032's US4"
      - path: crates/finder/src/imaging/mod.rs
        other_spec: 034-dark-and-flat-frames
        other_story: US4
        disposition: merge_order
        reason: "US4 depends_on_merged US2, and adds the calibration object to the view"
      - path: crates/finder/tests/console/main.rs
        other_spec: 033-focus-reading-and-aid
        other_story: US8
        disposition: merge_order
        reason: "034 depends_on_landed 033, whose US8 last changed console/main.rs, after 032's US5"
      - path: crates/finder/tests/console/main.rs
        other_spec: 034-dark-and-flat-frames
        other_story: US3
        disposition: merge_order
        reason: "US3 depends_on_merged US2, and adds mod calibration_library after US2's mod calibration"
    open_questions: []
  US3:
    tests:
      - path: crates/finder/tests/console/calibration_library.rs
        status: new
        disposition: add
        assertion: "encode_real writes a BITPIX -32 image with its cards and big-endian reals, decode_real reads it back and refuses a 16-bit image and short data, and 031's decode still refuses BITPIX -32"
        decision: "spec.md:US3-S1"
      - path: crates/finder/tests/console/calibration_library.rs
        status: new
        disposition: add
        assertion: "a new library is empty, names a master by kind and UTC second, writes its file with the master's cards, DARKTEST among them, and index.json with its entry, leaves no .part file, reads the same list when reopened, numbers a second id and never replaces a file"
        decision: "spec.md:US3-S2"
      - path: crates/finder/tests/console/calibration_library.rs
        status: new
        disposition: add
        assertion: "a master dark matches by camera, size, origin, bits, pattern, binning, gain, offset, exposure within 1 ms and set temperature within 1 degree, or sensor temperature within 8 when neither is set, the nearest then newest wins, and FrameKey::agrees holds a frame to another's settings and to an exposure within 1 ms"
        decision: "spec.md:US3-S3"
      - path: crates/finder/tests/console/calibration_library.rs
        status: new
        disposition: add
        assertion: "a master flat matches by camera, size, origin, bits, pattern and binning alone, whatever its gain, offset, set temperature, exposure and sensor temperature, the newest or the named label wins, and a camera turned by more than 1 degree is reported with the flat still chosen"
        decision: "spec.md:US3-S4"
      - path: crates/finder/tests/console/calibration_library.rs
        status: new
        disposition: add
        assertion: "Library::calibrate applies the matching dark and flat, finds and repairs a hot sample in the sub when the library is empty and reports what was missing, and names a master whose file is gone in problems"
        decision: "spec.md:US3-S5"
      - path: crates/finder/tests/console/calibration_library.rs
        status: new
        disposition: add
        assertion: "remove deletes a master's file and entry, and an unreadable index is reported, moved aside at the next add and replaced"
        decision: "spec.md:US3-S6"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and each binary's main.rs holds only its support include and mod lines"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "031's encode and decode are not edited"
      - "US2's functions keep their signatures and results"
      - "no console state, route or view changes in this story: the library is files and pure rules"
      - "crates/finder/tests/console/imaging_frames.rs, a test file 031 creates, passes unchanged: 031's fits::encode and decode, the refusal of BITPIX -32 included, and its frame ids and files (plan.md Shapes that existing tests pin)"
      - "crates/finder/tests/console/calibration.rs, a test file this spec's earlier story creates, passes unchanged: US2's calibrate, repair, find_hot_pixels and lit_samples are unchanged (plan.md Shapes that existing tests pin)"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console calibration_library::"
        - "cargo nextest run -p finder --locked --test console calibration::"
        - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - crates/finder/src/imaging/calibration.rs
      - crates/finder/src/imaging/calibration/library.rs
      - crates/finder/src/imaging/fits.rs
      - crates/finder/tests/console/calibration_library.rs
      - crates/finder/tests/console/main.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/imaging/calibration.rs
        other_spec: 034-dark-and-flat-frames
        other_story: US4
        disposition: merge_order
        reason: "US4 depends_on_merged US3, and adds pub mod service"
      - path: crates/finder/src/imaging/calibration/library.rs
        other_spec: 034-dark-and-flat-frames
        other_story: US5
        disposition: merge_order
        reason: "US5 depends_on_merged US3, and adds the map's file"
      - path: crates/finder/src/imaging/fits.rs
        other_spec: 031-imaging-frames-on-console
        other_story: US2
        disposition: merge_order
        reason: "034 depends_on_landed 031, whose US2 created fits.rs"
      - path: crates/finder/tests/console/main.rs
        other_spec: 034-dark-and-flat-frames
        other_story: US4
        disposition: merge_order
        reason: "US4 depends_on_merged US3, and adds mod calibration_console"
    open_questions: []
  US4:
    tests:
      - path: crates/finder/tests/console/calibration_console.rs
        status: new
        disposition: add
        assertion: "GET /api/imaging/calibration, the imaging view and the session carry the masters and the match for the latest imaging frame, and DELETE removes a master, answers 404 for unknown and ill-formed ids and 403 for a foreign origin"
        decision: "spec.md:US4-S1"
      - path: crates/finder/tests/console/calibration_console.rs
        status: new
        disposition: add
        assertion: "a console without imaging answers an empty library, refuses DELETE with 409 and a foreign origin with 403 first, GET /api/imaging stays exactly off, and a view that is not ready carries no calibration"
        decision: "spec.md:US4-S2"
      - path: crates/finder/tests/console/calibration_console.rs
        status: new
        disposition: add
        assertion: "a frame key read from a saved file's cards agrees with the one read from the imaging view and its frame.set_temp_c, match stays on the latest sub through a dark, an exposure, a focus frame and a flat, and is absent while the only frames are a dark and a focus frame"
        decision: "spec.md:US4-S3"
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
      - "a console built without with_imaging serves the same session and exactly {\"state\":\"off\"} (plan.md Shapes that existing tests pin)"
      - "nothing about calibration is persisted in session.json"
      - "US3's library, its files and its rules are unchanged; this story adds the console's state over them"
      - "crates/finder/tests/console/imaging.rs, a test file 031 creates, passes unchanged: 031's imaging view: exactly off without imaging, no extra key outside ready, and the session's imaging equal to GET /api/imaging (plan.md Shapes that existing tests pin)"
      - "crates/finder/tests/console/imaging_relation.rs, a test file 032 creates, passes unchanged: 032's relation and its angle_deg are read and not changed (plan.md Shapes that existing tests pin)"
      - "crates/finder/tests/console/imaging_focus_aid.rs, a test file 033 creates, passes unchanged: the focus aid's frames keep the purpose focus and are saved and removed as before, and the view's focus is unchanged; match only passes over them (plan.md Shapes that existing tests pin)"
      - "crates/finder/tests/console/calibration_library.rs, a test file this spec's earlier story creates, passes unchanged: US3's library scenarios (plan.md Shapes that existing tests pin)"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console calibration_console::"
        - "cargo nextest run -p finder --locked --test console imaging"
        - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs, and a rebuild of the console"
    writes:
      - crates/finder/README.md
      - crates/finder/src/api.rs
      - crates/finder/src/imaging/calibration.rs
      - crates/finder/src/imaging/calibration/service.rs
      - crates/finder/src/imaging/mod.rs
      - crates/finder/src/imaging/routes.rs
      - crates/finder/src/lib.rs
      - crates/finder/tests/console/calibration_console.rs
      - crates/finder/tests/console/main.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/README.md
        other_spec: 033-focus-reading-and-aid
        other_story: US7
        disposition: merge_order
        reason: "034 depends_on_landed 033, whose US7, after its US8, last changed the README"
      - path: crates/finder/README.md
        other_spec: 034-dark-and-flat-frames
        other_story: US5
        disposition: merge_order
        reason: "US5 depends_on_merged US4, and adds the bad-pixel map's paragraph"
      - path: crates/finder/src/api.rs
        other_spec: 033-focus-reading-and-aid
        other_story: US1
        disposition: merge_order
        reason: "034 depends_on_landed 033, whose US1 last changed api.rs, after 032's US3"
      - path: crates/finder/src/imaging/calibration.rs
        other_spec: 034-dark-and-flat-frames
        other_story: US5
        disposition: merge_order
        reason: "US5 depends_on_merged US4, and adds bad_pixel_map"
      - path: crates/finder/src/imaging/calibration/service.rs
        other_spec: 034-dark-and-flat-frames
        other_story: US6
        disposition: merge_order
        reason: "US6 depends_on_merged US4, and adds the darks set"
      - path: crates/finder/src/imaging/routes.rs
        other_spec: 033-focus-reading-and-aid
        other_story: US5
        disposition: merge_order
        reason: "034 depends_on_landed 033, whose US5 last changed the imaging routes, after 032's US2"
      - path: crates/finder/src/imaging/routes.rs
        other_spec: 034-dark-and-flat-frames
        other_story: US6
        disposition: merge_order
        reason: "US6 depends_on_merged US4, and adds the darks route"
      - path: crates/finder/src/lib.rs
        other_spec: 033-focus-reading-and-aid
        other_story: US8
        disposition: merge_order
        reason: "034 depends_on_landed 033, whose US8 last changed lib.rs, after 032's US2"
      - path: crates/finder/src/lib.rs
        other_spec: 034-dark-and-flat-frames
        other_story: US6
        disposition: merge_order
        reason: "US6 depends_on_merged US4, and adds its route line after US4's"
      - path: crates/finder/tests/console/main.rs
        other_spec: 034-dark-and-flat-frames
        other_story: US5
        disposition: merge_order
        reason: "US5 depends_on_merged US4, and adds mod calibration_bad_pixels"
    open_questions: []
  US5:
    tests:
      - path: crates/finder/tests/console/calibration_bad_pixels.rs
        status: new
        disposition: add
        assertion: "bad_pixel_map lists the samples more than 3.5 spreads above the median, row by row, with a spread floor of one count"
        decision: "spec.md:US5-S1"
      - path: crates/finder/tests/console/calibration_bad_pixels.rs
        status: new
        disposition: add
        assertion: "a column pattern widens the spread and is not mapped, while a sample above the line is"
        decision: "spec.md:US5-S2"
      - path: crates/finder/tests/console/calibration_bad_pixels.rs
        status: new
        disposition: add
        assertion: "a map of more than 1 percent of the frame is no map, one sample in a hundred is kept, and a sample that is not a number is mapped"
        decision: "spec.md:US5-S3"
      - path: crates/finder/tests/console/calibration_bad_pixels.rs
        status: new
        disposition: add
        assertion: "the map of a mean of simulated covered frames is exactly the simulator's hot pixels"
        decision: "spec.md:US5-S4"
      - path: crates/finder/tests/console/calibration_bad_pixels.rs
        status: new
        disposition: add
        assertion: "attach_bad_pixels writes the map beside its master and its size into the entry and index, select and calibrate use it, remove deletes it, a too-large map leaves null and the search in the sub, and a master only added keeps null"
        decision: "spec.md:US5-S5"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and each binary's main.rs holds only its support include and mod lines"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "Library::add makes no map by itself (plan.md Plan traps 9)"
      - "US2's functions and US3's files, index keys and select are unchanged"
      - "crates/finder/tests/console/calibration_library.rs, a test file this spec's earlier story creates, passes unchanged: US3's library scenarios: a master only added has bad_pixels null and its selection names bad_pixels missing (plan.md Shapes that existing tests pin)"
      - "crates/finder/tests/console/calibration_console.rs, a test file this spec's earlier story creates, passes unchanged: US4's match for a master only added names bad_pixels missing (plan.md Shapes that existing tests pin)"
      - "crates/finder/tests/console/calibration.rs, a test file this spec's earlier story creates, passes unchanged: US2's calibrate, repair and find_hot_pixels are unchanged (plan.md Shapes that existing tests pin)"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console calibration_bad_pixels::"
        - "cargo nextest run -p finder --locked --test console calibration_library::"
        - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - GLOSSARY.md
      - crates/finder/README.md
      - crates/finder/src/imaging/calibration.rs
      - crates/finder/src/imaging/calibration/library.rs
      - crates/finder/tests/console/calibration_bad_pixels.rs
      - crates/finder/tests/console/main.rs
    no_write_reason: ""
    shared_writes:
      - path: GLOSSARY.md
        other_spec: 034-dark-and-flat-frames
        other_story: US6
        disposition: merge_order
        reason: "US6 depends_on_merged US5, and adds Offset level"
      - path: crates/finder/README.md
        other_spec: 034-dark-and-flat-frames
        other_story: US6
        disposition: merge_order
        reason: "US6 depends_on_merged US5, and adds the darks row and paragraph"
      - path: crates/finder/tests/console/main.rs
        other_spec: 034-dark-and-flat-frames
        other_story: US6
        disposition: merge_order
        reason: "US6 depends_on_merged US5, and adds mod calibration_darks"
    open_questions: []
  US6:
    tests:
      - path: crates/finder/tests/console/calibration_darks.rs
        status: new
        disposition: add
        assertion: "a set of four darks under the cover makes a master dark equal to the mean of its four saved frames, with its entry, checked true, its cards, hot samples, bad-pixel map, five kept DARK frames, the lease held under HOLDER from the answer until done and free before and after, and the exposure setting restored"
        decision: "spec.md:US6-S1"
      - path: crates/finder/tests/console/calibration_darks.rs
        status: new
        disposition: add
        assertion: "binning, the cooler and the set temperature in force are carried into the master's entry, file and binned map"
        decision: "spec.md:US6-S2"
      - path: crates/finder/tests/console/calibration_darks.rs
        status: new
        disposition: add
        assertion: "an uncovered camera is refused as not dark against the offset level where the sky raises it, by its bright samples that are not hot pixels where the sky does not, and as not covered when bright at its shortest exposure, each with not_dark, nothing added and the exposure restored"
        decision: "spec.md:US6-S3"
      - path: crates/finder/tests/console/calibration_darks.rs
        status: new
        disposition: add
        assertion: "on a scripted camera a dark two counts above the offset level passes and three fails with its exact words, an offset level of 13108 is refused where 13107 passes, and with anyway both are taken and the master says checked false and DARKTEST F"
        decision: "spec.md:US6-S4"
      - path: crates/finder/tests/console/calibration_darks.rs
        status: new
        disposition: add
        assertion: "the darks route refuses bad counts and bodies with 400, a foreign origin with 403, another content type with 415, a running set before a frame in flight, an exposure in progress, a node without the service and a console without imaging with 409 and their words, in the order origin, body shape, no imaging, count"
        decision: "spec.md:US6-S5"
      - path: crates/finder/tests/console/calibration_darks.rs
        status: new
        disposition: add
        assertion: "DELETE with the running set's id, or an aborted exposure, ends the set stopped with no master and the exposure restored, and the lease is held under HOLDER while the set runs and free after"
        decision: "spec.md:US6-S6"
      - path: crates/finder/tests/console/calibration_darks.rs
        status: new
        disposition: add
        assertion: "a failed frame ends the set with a dark failed and the frame's words, a frame whose gain differs with the settings changed, and a sensor 2.5 degrees from its set temperature with its exact words whether or not anyway was sent, each with no master"
        decision: "spec.md:US6-S7"
      - path: crates/finder/tests/console/calibration_darks.rs
        status: new
        disposition: add
        assertion: "a later imaging frame at the same settings matches the master dark and its map, and of two matching masters the newest is chosen"
        decision: "spec.md:US6-S8"
      - path: crates/finder/tests/console/calibration_darks.rs
        status: new
        disposition: add
        assertion: "while a dark exposes the set holds the lease under HOLDER, so an imaging frame, a settings change and a second lease are refused with a set of darks or flats in the words; a darks request while a lease of the test's own exposes answers the lease's refusal naming probe, not an exposure is in progress, and is accepted once that lease is dropped"
        decision: "spec.md:US6-S9"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and each binary's main.rs holds only its support include and mod lines"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "Imaging::take, ended, set and abort and 031's lease keep their signatures and refusals; the set takes the lease and adds no lock of its own (plan.md Shapes that existing tests pin)"
      - "nothing on the scope node or the scope link changes"
      - "every dark is saved as 031 saves every imaging frame, and no file is replaced"
      - "crates/finder/tests/console/calibration_console.rs, a test file this spec's earlier story creates, passes unchanged: US4's routes and view: GET and DELETE behave as before when no set exists (plan.md Shapes that existing tests pin)"
      - "crates/finder/tests/console/calibration_library.rs, a test file this spec's earlier story creates, passes unchanged: US3's FrameKey::agrees and matching are unchanged (plan.md Shapes that existing tests pin)"
      - "crates/finder/tests/console/imaging_frames.rs, a test file 031 creates, passes unchanged: 031's frame taking, ids, files, abort and refusals are unchanged with the sets as one more caller (plan.md Shapes that existing tests pin)"
      - "crates/finder/tests/console/imaging_lease.rs, a test file 031 creates, passes unchanged: 031's lease, its refusal and its release are used as they are, and nothing is added to them (plan.md The camera's lease)"
      - "crates/finder/tests/console/imaging_focus_aid.rs, a test file 033 creates, passes unchanged: the focus aid takes and releases the lease as before; none of its tests runs a set (plan.md Shapes that existing tests pin)"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console calibration_darks::"
        - "cargo nextest run -p finder --locked --test console calibration_console::"
        - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs, a rebuild of the console, and a first dark set on the real camera"
    writes:
      - GLOSSARY.md
      - crates/finder/README.md
      - crates/finder/src/imaging/calibration/service.rs
      - crates/finder/src/imaging/routes.rs
      - crates/finder/src/lib.rs
      - crates/finder/tests/console/calibration_darks.rs
      - crates/finder/tests/console/main.rs
    no_write_reason: ""
    shared_writes:
      - path: GLOSSARY.md
        other_spec: 034-dark-and-flat-frames
        other_story: US7
        disposition: merge_order
        reason: "US7 depends_on_merged US6, and adds Flat label"
      - path: crates/finder/README.md
        other_spec: 034-dark-and-flat-frames
        other_story: US7
        disposition: merge_order
        reason: "US7 depends_on_merged US6, and adds the flats row and paragraph"
      - path: crates/finder/src/imaging/calibration/service.rs
        other_spec: 034-dark-and-flat-frames
        other_story: US7
        disposition: merge_order
        reason: "US7 depends_on_merged US6, and adds the flat set to the runner"
      - path: crates/finder/src/imaging/routes.rs
        other_spec: 034-dark-and-flat-frames
        other_story: US7
        disposition: merge_order
        reason: "US7 depends_on_merged US6, and adds the flats route"
      - path: crates/finder/src/lib.rs
        other_spec: 034-dark-and-flat-frames
        other_story: US7
        disposition: merge_order
        reason: "US7 depends_on_merged US6, and adds its route line after US6's"
      - path: crates/finder/tests/console/main.rs
        other_spec: 034-dark-and-flat-frames
        other_story: US7
        disposition: merge_order
        reason: "US7 depends_on_merged US6, and adds mod calibration_flats"
    open_questions: []
  US7:
    tests:
      - path: crates/finder/tests/console/calibration_flats.rs
        status: new
        disposition: add
        assertion: "on a scripted linear camera the search asks for 100, 200 and 366.634 ms, takes three flats there holding the lease, waits for the darks holding none, then takes the lease again for an offset frame and three darks and adds a master flat of ones with its entry, checked true, its cards and ten kept frames"
        decision: "spec.md:US7-S1"
      - path: crates/finder/tests/console/calibration_flats.rs
        status: new
        disposition: add
        assertion: "the search steps down from a saturated start, and fails with its exact words when the panel is too dim, too bright, does not rise, or eight tries do not find the level, naming the eighth exposure taken, and restoring the exposure each time"
        decision: "spec.md:US7-S2"
      - path: crates/finder/tests/console/calibration_flats.rs
        status: new
        disposition: add
        assertion: "a flat outside 40 to 50 percent fails the set, darks taken with the panel still lit return the set to needs_darks with the dark test's words and not_dark until a covered retry finishes it, and anyway takes darks the test refused and marks the master checked false"
        decision: "spec.md:US7-S3"
      - path: crates/finder/tests/console/calibration_flats.rs
        status: new
        disposition: add
        assertion: "the flats route refuses bad bodies, counts and labels with 400 and their words, anyway without darks, darks with nothing waiting, a second set, a running set and a console without imaging with 409, and DELETE drops flats that wait"
        decision: "spec.md:US7-S4"
      - path: crates/finder/tests/console/calibration_flats.rs
        status: new
        disposition: add
        assertion: "a master flat made from simulated flats and their darks equals the simulator's vignetting and dust over each colour's median, with the panel's colour, the offset level, the column pattern and a hot pixel gone"
        decision: "spec.md:US7-S5"
      - path: crates/finder/tests/console/calibration_flats.rs
        status: new
        disposition: add
        assertion: "the imaging angle 032 measured is kept in the master flat's entry and file, a later match warns once the measured angle has moved by more than a degree, and nothing warns once the aim point is no longer the measured one"
        decision: "spec.md:US7-S6"
      - path: crates/finder/tests/console/calibration_flats.rs
        status: new
        disposition: add
        assertion: "a simulated sky frame calibrated with a master dark and a master flat the console made has the same background at corner and centre, and the match names both masters and the map"
        decision: "spec.md:US7-S7"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and each binary's main.rs holds only its support include and mod lines"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "US6's SetView keys keep their names and meanings; US7 only adds keys"
      - "the dark test, its three texts and anyway are the same for a flat set's darks"
      - "nothing on the scope node or the scope link changes"
      - "crates/finder/tests/console/calibration_darks.rs, a test file this spec's earlier story creates, passes unchanged, and is not edited: US6's dark sets, their refusals, texts and restored exposure, and the scripted camera this story imports from it (plan.md Shapes that existing tests pin)"
      - "crates/finder/tests/console/calibration_library.rs, a test file this spec's earlier story creates, passes unchanged: US3's matching of flats and the turned-camera rule are unchanged (plan.md Shapes that existing tests pin)"
      - "crates/finder/tests/console/imaging_relation.rs, a test file 032 creates, passes unchanged: 032's aim readings and the imaging angle they give are read and not changed (plan.md Shapes that existing tests pin)"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console calibration_flats::"
        - "cargo nextest run -p finder --locked --test console calibration_darks::"
        - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs, a rebuild of the console, and a first flat set on the real camera"
    writes:
      - GLOSSARY.md
      - crates/finder/README.md
      - crates/finder/src/imaging/calibration/service.rs
      - crates/finder/src/imaging/routes.rs
      - crates/finder/src/lib.rs
      - crates/finder/tests/console/calibration_flats.rs
      - crates/finder/tests/console/main.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/README.md
        other_spec: 034-dark-and-flat-frames
        other_story: US8
        disposition: merge_order
        reason: "US8 depends_on_merged US7, and adds the three tools' lines"
    open_questions: []
  US8:
    tests:
      - path: crates/finder/tests/mcp/calibration.rs
        status: new
        disposition: add
        assertion: "tools/list names list_calibration, take_darks and take_flats with their schemas, anyway among their keys, their descriptions and the read-only hint on the first alone"
        decision: "spec.md:US8-S1"
      - path: crates/finder/tests/mcp/calibration.rs
        status: new
        disposition: add
        assertion: "list_calibration equals GET /api/imaging/calibration, and take_darks answers before its set is done and the library then holds the master"
        decision: "spec.md:US8-S2"
      - path: crates/finder/tests/mcp/calibration.rs
        status: new
        disposition: add
        assertion: "the two taking tools refuse with the routes' words, take_flats passes through to a set that fails as the route's would, and a second set is refused while one runs"
        decision: "spec.md:US8-S3"
      - path: crates/finder/tests/mcp/calibration.rs
        status: new
        disposition: add
        assertion: "on a console without imaging list_calibration answers an empty library and the other two refuse at once, and no result holds the token"
        decision: "spec.md:US8-S4"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and each binary's main.rs holds only its support include and mod lines, with calibration added to mcp's"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/mcp/motion.rs
        status: existing
        disposition: preserve
        assertion: "every listed tool called with empty arguments answers without the node's token"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/mcp/endpoint.rs
        status: existing
        disposition: preserve
        assertion: "the MCP door and get_session are unchanged"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "no tool is a moving tool: no rate limit and no call-log line"
      - "the existing tools keep their names, schemas and answers"
      - "crates/finder/tests/mcp/imaging.rs, a test file 031 creates, passes unchanged: 031's three imaging tools and their answers are unchanged (plan.md Shapes that existing tests pin)"
      - "crates/finder/tests/mcp/imaging_solve.rs, a test file 032 creates, passes unchanged: 032's tool and its answer are unchanged (plan.md Shapes that existing tests pin)"
      - "crates/finder/tests/mcp/imaging_focus.rs, a test file 033 creates, passes unchanged: 033's two tools and their answers are unchanged (plan.md Shapes that existing tests pin)"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test mcp calibration::"
        - "cargo nextest run -p finder --locked --test mcp motion::"
        - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs, and a rebuild of the console"
    writes:
      - crates/finder/README.md
      - crates/finder/src/mcp/tools.rs
      - crates/finder/tests/mcp/calibration.rs
      - crates/finder/tests/mcp/main.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/mcp/tools.rs
        other_spec: 033-focus-reading-and-aid
        other_story: US7
        disposition: merge_order
        reason: "034 depends_on_landed 033, whose US7 last changed the MCP tools, after 032's US7"
      - path: crates/finder/tests/mcp/main.rs
        other_spec: 033-focus-reading-and-aid
        other_story: US7
        disposition: merge_order
        reason: "034 depends_on_landed 033, whose US7 last changed mcp/main.rs, after 032's US7"
    open_questions: []
  US9:
    tests:
      - path: crates/finder/web/src/components/CalibrationBlock.test.tsx
        status: new
        disposition: add
        assertion: "an empty calibration shows the region with its two forms, hints and no status or alert, and the Imaging page shows the block only when its ready view carries calibration"
        decision: "spec.md:US9-S1"
      - path: crates/finder/web/src/components/CalibrationBlock.test.tsx
        status: new
        disposition: add
        assertion: "each master reads its exact line, with and without gain, offset, set temperature, map and angle, with dark test skipped when checked is false, and Remove sends DELETE with its id"
        decision: "spec.md:US9-S2"
      - path: crates/finder/web/src/components/CalibrationBlock.test.tsx
        status: new
        disposition: add
        assertion: "Take darks and Take flats post their counts and label, call onChange, and show a refusal as an alert"
        decision: "spec.md:US9-S3"
      - path: crates/finder/web/src/components/CalibrationBlock.test.tsx
        status: new
        disposition: add
        assertion: "each state of a dark set and a flat set reads its exact words, with Stop, Take the flats' darks and Drop these flats sending their requests, the Take buttons disabled, and Take them anyway or Take the flats' darks anyway offered only when the view says not_dark"
        decision: "spec.md:US9-S4"
      - path: crates/finder/web/src/components/CalibrationBlock.test.tsx
        status: new
        disposition: add
        assertion: "the match reads what the latest imaging frame would get, a turned camera and an unreadable library are alerts"
        decision: "spec.md:US9-S5"
      - path: crates/finder/web/src/components/CalibrationBlock.test.tsx
        status: new
        disposition: add
        assertion: "through the page's hook a request is followed at once by a read of /api/imaging, and the next view's set shows its status"
        decision: "spec.md:US9-S6"
    preserved_behavior:
      - "the page adds no npm package"
      - "red-only night mode: the block holds no picture and uses existing tokens and classes"
      - "a view without calibration renders no block"
      - "crates/finder/web/src/components/ImagingPage.test.tsx, a test file 031 creates, passes unchanged: 031's Imaging page reads the same words, statuses and alerts for views without calibration (plan.md Shapes that existing tests pin)"
      - "crates/finder/web/src/components/ImagingSolveBlock.test.tsx, a test file 032 creates, passes unchanged: 032's block on the Imaging page is not moved or reworded (plan.md Shapes that existing tests pin)"
      - "crates/finder/web/src/components/FocusBlock.test.tsx, a test file 033 creates, passes unchanged: 033's Focus block keeps its place after 032's block, and the Calibration block comes after it (plan.md Shapes that existing tests pin)"
      - "crates/finder/web/e2e/imaging.spec.ts, a test file 031 creates, passes unchanged: the Imaging page at night has no off-red pixel outside the held picture, the Calibration block included (plan.md Shapes that existing tests pin)"
    checks:
      development:
        - "npm test"
        - "npm run typecheck"
        - "npm run build"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs: finder-page, and a design review on a phone in the dark"
    writes:
      - .impeccable/design.json
      - DESIGN.md
      - crates/finder/web/dist
      - crates/finder/web/src/api.ts
      - crates/finder/web/src/components/CalibrationBlock.test.tsx
      - crates/finder/web/src/components/CalibrationBlock.tsx
      - crates/finder/web/src/components/ImagingPage.tsx
    no_write_reason: ""
    shared_writes:
      - path: .impeccable/design.json
        other_spec: 033-focus-reading-and-aid
        other_story: US6
        disposition: merge_order
        reason: "034 depends_on_landed 033, whose US6 last changed design.json, after 032's US6"
      - path: DESIGN.md
        other_spec: 033-focus-reading-and-aid
        other_story: US6
        disposition: merge_order
        reason: "034 depends_on_landed 033, whose US6 last changed DESIGN.md, after 032's US6"
      - path: crates/finder/web/dist
        other_spec: 033-focus-reading-and-aid
        other_story: US6
        disposition: merge_order
        reason: "034 depends_on_landed 033, whose US6 last rebuilt the bundle, after 032's US6"
      - path: crates/finder/web/src/api.ts
        other_spec: 033-focus-reading-and-aid
        other_story: US6
        disposition: merge_order
        reason: "034 depends_on_landed 033, whose US6 last changed api.ts, after 032's US6"
      - path: crates/finder/web/src/components/ImagingPage.tsx
        other_spec: 033-focus-reading-and-aid
        other_story: US6
        disposition: merge_order
        reason: "034 depends_on_landed 033, whose US6 last changed the Imaging page, placing its Focus block after 032's"
    open_questions: []
```
