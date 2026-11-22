# Implementation Plan: The Node Detects Stars on a Dark Sky

**Branch**: `048-node-detects-stars-on-a-dark-sky` | **Date**: 2026-11-22 | **Spec**: [spec.md](spec.md)

This plan records every design decision for the feature. A story that would change one
escalates rather than re-deciding it.

## Summary

Detection gains a floor under its noise estimate. `star_detection::detect_stars_with_noise_floor`
takes `detect_stars`'s arguments and a floor in 8-bit levels, and raises the detection
image's noise estimate to at least that floor before cedar's gates use it. `detect_stars`
calls it with cedar-detect's own 0.2, so it finds exactly what it found. The node's
`[detection]` takes `noise_floor`, off unless set, from 0.2 to 5, and
`scope_node::detect::detect` uses it when it is set (US1).

`Configure` keeps the node's own detection keys, so the floor survives the console's
pairing. Auto-exposure does not change (owner decision 5, answered): on the node it
never goes above 200 ms, its cap at the node's own frame interval. On the drawn dark sky,
with the README's floor of 0.5, it holds 200 ms and raises gain until it finds the
stars, every frame from the fourth solving; without a floor it falls to under 10 ms and
no frame from the third solves. Gain is noiseless in that model (spec § Assumptions), so
this shows auto-exposure's path and the floor surviving pairing, not that a real 200 ms
frame holds enough stars. A longer exposure needs a fixed exposure set in Setup.
`solve_image --noise-floor` replays recorded raw frames, so the floor can be tuned
offline (US2).

Two stories, in merge order US1, US2. A table of detection settings per camera adapter
is left to spec 045 (owner decision 4, answered; § Outside the factory).

## Technical Context

**Language/Version**: Rust, MSRV 1.96, stable in CI.

**Primary Dependencies**: none new. `image` is already a dev-dependency of
`star-detection` (its parity test reads cedar's JPEGs). The simulator's tests already
reach `scope-node`, `scope-link`, `plate-solver`, `pattern-database`, `math-core`,
`tonic` and `tokio` with `test-util`; the probe built every new test with no
`Cargo.toml` change. `Cargo.toml` and `Cargo.lock` do not change.

**Storage**: none.

**Testing**:
- `crates/star-detection/tests/noise_floor.rs` (new, US1): US1-S1 and US1-S2, under
  `#[test]`.
- `crates/simulator/tests/dark/mod.rs` (new, US1): the skies of § The skies. It is a
  module that each test binary declares with `mod dark;`, not a test binary itself.
- `crates/simulator/tests/dark_sky.rs` (new, US1): US1-S3 to US1-S5, under `#[test]`.
- `crates/scope-node/tests/noise_floor.rs` (new, US1): US1-S6 and US1-S7, under
  `#[test]`.
- `crates/simulator/tests/dark_sky_exposure.rs` (new, US2): US2-S1 and US2-S2, under
  `#[tokio::test(start_paused = true)]`.
- No Vitest or Playwright test: the page is unchanged.

**Target Platform**: `scope-node` on the Pi under systemd; the `solve_image` example on
the console's computer.

**Constraints**:
- `detect_stars`, `estimate_noise`, `DetectSettings::cedar()`, `Config::parse`,
  `Config::settings`, `Config::masked`, `NodeService::new`, `with_controls` and
  `with_controls_and_mount` keep their signatures; existing tests call them (§ Shapes
  that existing tests pin).
- `RUSTFLAGS="-D warnings"`, `--all-features` clippy, and rustdoc with warnings denied.
- No test uses a camera, a Pi or a network socket. No test sends a signal.

**Scale/Scope**: two stories, the scope node's detection with two reaches outside its
crate. US1 changes three source files of `crates/star-detection`, the node's `detect.rs`
and `config.rs` and its README; US2 the node's `service.rs` and README and the
`crates/plate-solver` example `solve_image`. They add five new test files, one of them a
shared module. Neither changes what the console does.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | `detect_stars` keeps every result: it calls the new function with `NOISE_FLOOR`, 0.2. `estimate_noise` already returns at least 0.2 (`crates/star-detection/src/noise.rs:91`), so the added `.max(0.2)` changes no value. US1-S1 holds this on cedar's 13 images at two settings, and `crates/star-detection/tests/parity.rs:107` still holds `detect_stars` to cedar-detect's goldens at binning 1. The floor itself is cedar-detect's: it sets `noise_floor = 0.2` because a background "crushed to black" leaves no noise to measure (`reference-solutions/cedar-detect/src/algorithm.rs:992-995`, `reference-solutions/cedar-detect/src/algorithm.rs:1070-1071`). This spec departs from cedar only in letting the config raise that value. The grounds are cedar's own comment, and tetra3's `get_centroids_from_image`, which takes an absolute threshold, `image_th` (`reference-solutions/cedar-solve/tetra3/tetra3.py:2404`, `reference-solutions/cedar-solve/tetra3/tetra3.py:2429`), and notes that a noise-derived threshold needs enough readout bit depth to capture the noise (`reference-solutions/cedar-solve/tetra3/tetra3.py:2425`). The README's 0.5 is the probe's, on a drawn sky. No real camera grounds it, so the floor is off by default and the README calls 0.5 a starting point. |
| II. Never weaken a gate | No existing test, golden, fixture, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS` or `ergane.yaml` is edited. Every existing test passed unchanged on the probe (§ What was run and what was read). The new tests go in new files. The shared skies go in a new module, not in `crates/simulator/tests/support/mod.rs`. The two alternatives that would need test edits, owner decisions 2 (c) and 5 (b), neither chosen, name their failing tests. |
| III. Tests first, independent expectations | The oracles are cedar's reference images; cedar's threshold rule written as arithmetic (US1-S2); the stars the test drew and the simulator's true camera direction; `main`'s own `--check-config` output, captured by the probe and written as literals; and the scenarios' words. One is partly the code under test: US1-S1 holds the new function to `detect_stars`, which the same diff rewrites to call it; `parity.rs` pins `detect_stars` to cedar-detect's goldens at binning 1 only (`crates/star-detection/tests/parity.rs:113`), and at binning 2 with row normalisation only `image_parity`'s solve does (`crates/plate-solver/tests/image_parity.rs:152`). No other test takes an expected value from the code under test. The clock for US2 is tokio's paused clock, and the sky camera sleeps each exposure on it. On `main`'s code, the tests that state today's behaviour pass and the rest do not build or fail (§ What was run and what was read). |
| IV. Critical paths | None is touched: nothing in `crates/math-core`, no `unsafe`, nothing under `proto/` or `crates/grpc-service`, `crates/pattern-database` or `crates/database-generation`, no `Cargo.toml`, `Cargo.lock`, `build.rs` or `.cargo/`. |
| V. Proven open source first, honest licensing | Detection is the solver's own Rust under Principle V's exception, so the plan looks only at what the references do. It found cedar-detect's fixed floor and its caller-supplied `noise_estimate` (`reference-solutions/cedar-detect/src/algorithm.rs:966`), and tetra3's absolute threshold (above). It takes cedar's floor and lets the config set it. The caller-supplied estimate is option (b) of owner decision 3, turned down there. An existing setting does much of the same on a dark night: Setup's detection threshold (sigma; `crates/finder/web/src/components/ScopeNodeSection.tsx:106-107`), (e) of owner decisions 3 and 5, which needs no code; the seat recommended the floor for what sigma cannot do, adapting by itself and leaving the drawn city sky alone (US1-S5), and the README documents sigma beside it as the no-code option (owner decision 3). The tuning tool reuses the repository's own `solve_image` example. The config key is one more serde field of `[detection]`. Not looked at: detectors outside `reference-solutions/`, such as Source Extractor's, which Principle V's exception keeps out. No reused code, no new license. |

## Project Structure

```text
crates/star-detection/
├── src/noise.rs                              # US1: NOISE_FLOOR made public
├── src/detect.rs                             # US1: detect_stars_with_noise_floor; detect_stars through it
├── src/lib.rs                                # US1: its re-export
└── tests/noise_floor.rs                      # US1, new
crates/scope-node/
├── src/detect.rs                             # US1: DetectSettings::noise_floor; detect uses it
├── src/config.rs                             # US1: the floor's check
├── src/service.rs                            # US2: Configure keeps the node's own detection keys
├── README.md                                 # US1: the section; US2: its later paragraphs
└── tests/noise_floor.rs                      # US1, new
crates/simulator/tests/
├── dark/mod.rs                               # US1, new: the skies
├── dark_sky.rs                               # US1, new
└── dark_sky_exposure.rs                      # US2, new
crates/plate-solver/examples/solve_image.rs   # US2: --noise-floor
```

## Interfaces

### `crates/star-detection` (US1)

```rust
// noise.rs: the constant becomes public; its value and doc meaning stay.
/// Minimum returned noise level, in ADU: cedar-detect's own floor.
pub const NOISE_FLOOR: f64 = 0.2;

// detect.rs
pub fn detect_stars(image: &[u8], width: usize, height: usize, sigma: f64,
                    binning: usize, normalize_rows: bool, detect_hot_pixels: bool) -> Vec<Star> {
    detect_stars_with_noise_floor(image, width, height, sigma, binning,
                                  normalize_rows, detect_hot_pixels, NOISE_FLOOR)
}

/// [`detect_stars`], with the noise estimate raised to at least `noise_floor`
/// levels. At or below cedar-detect's own floor, [`NOISE_FLOOR`], it finds
/// exactly what [`detect_stars`] finds.
#[allow(clippy::too_many_arguments)]
pub fn detect_stars_with_noise_floor(image: &[u8], width: usize, height: usize, sigma: f64,
                                     binning: usize, normalize_rows: bool,
                                     detect_hot_pixels: bool, noise_floor: f64) -> Vec<Star>

// lib.rs
pub use detect::{detect_stars, detect_stars_with_noise_floor};
```

The body of `detect_stars_with_noise_floor` is today's body of `detect_stars`, with
one change: the line at `crates/star-detection/src/detect.rs:47` becomes
`estimate_noise(&detection.data, detection.width, detection.height).max(noise_floor)`.
Every later use of `noise` reads the raised value: the two gates (`crates/star-detection/src/detect.rs:48-49`), the
hot-pixel check through the first gate, and the perimeter and core checks (`crates/star-detection/src/detect.rs:335`,
`crates/star-detection/src/detect.rs:340`). cedar-detect likewise floors its estimate before any check uses it
(`reference-solutions/cedar-detect/src/algorithm.rs:992-995`, `reference-solutions/cedar-detect/src/algorithm.rs:1070-1071`). Nothing else in the crate changes. The
same `#[allow]` sits on `plate_solver::solve_from_centroids`.

### `crates/scope-node/src/detect.rs` (US1)

```rust
pub struct DetectSettings {
    pub sigma: f64,
    pub noise_estimate: Option<f64>,
    /// The least noise detection assumes, in 8-bit levels of the detection
    /// image; absent, cedar-detect's own 0.2.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub noise_floor: Option<f64>,
    pub binning: usize,
    // ... the other fields, unchanged and in today's order
}
```

`DetectSettings::cedar()` sets `noise_floor: None`. The field sits right after
`noise_estimate`, so `--check-config` prints it right after `sigma`, since the absent
`noise_estimate` is not printed. `detect` calls `star_detection::detect_stars` while
`noise_floor` is `None`, and `detect_stars_with_noise_floor` with its value while it is
`Some`. It keeps `.take(200)` and the `Centroid` mapping as today.

### `crates/scope-node/src/config.rs` (US1)

In `validate`, after the binning check and before the cedar-default refusal (`crates/scope-node/src/config.rs:313`):

```rust
if let Some(floor) = self.detection.noise_floor {
    if !floor.is_finite() || !(0.2..=5.0).contains(&floor) {
        return Err(ConfigError("detection.noise_floor must be between 0.2 and 5".into()));
    }
}
```

`masked` does not change. It prints `toml::to_string(&self.detection)`, which carries
the floor when set and omits it when not. A value that is not a number fails in
`Config::parse` with today's message for the key, `invalid noise_floor at line <n>`:
the key comes from the line's text before `=`, at `crates/scope-node/src/config.rs:213-214`.
Nothing else in the file changes.

### The node's README (US1)

A new section, `## Dark skies: the noise floor`, placed right before `## The mount
link` (`crates/scope-node/README.md:120`). It says:
- why a background clipped to 0 gives 200 detections, by reading mostly noise (on a dark
  night the node reported 200 a frame and none matched), and drives auto-exposure darker
  (on a simulated dark sky gain and exposure fell to their least and the live solve
  never came);
- that `noise_floor` in `[detection]` sets the least noise detection assumes, in the
  8-bit levels of the frame detection reads;
- that it is off unless set, and must be between 0.2 and 5 when set;
- that a floor below a sky's own noise changes nothing (on the drawn city sky 0.5 and 1
  changed nothing); a town sky's noise on the finder camera is not measured, so leave the
  floor out in town, or compare star counts with and without it;
- 0.5 as a starting point for a dark site, in a `toml` snippet;
- that `--check-config` prints it under `[detection]`;
- beside the floor, Setup's detection threshold (sigma), named as Setup labels it,
  "Detection threshold" in σ
  (`crates/finder/web/src/components/ScopeNodeSection.tsx:106-107`), as the no-code
  option (owner decision 3, FR-004): with no floor set, on a background clipped to 0, as
  on the simulated dark sky, sigma 20 acts as a floor of 0.5 and 40 as 1.0; like 0.5,
  about 20 to 40 is a starting point at a dark site, and it needs no config change; it
  costs stars in town, where a city frame's detections fall (on the drawn city frame, 30
  at sigma 8 and 8 at sigma 20), so it is set back to 8 there.

The example config at `crates/scope-node/README.md:48-59` does not change.

### `crates/scope-node/src/service.rs` (US2)

```rust
struct Shared {
    info: CameraInfo,
    /// The node's own detection settings, before any from the console.
    detection: DetectSettings,
    // ... unchanged
}
// in with_controls_and_mount, building Shared: detection: initial.detection.clone(),

/// Detection fields the link does not carry, such as the noise floor, stay as
/// the node's config set them.
fn from_wire(settings: proto::Settings, base: &DetectSettings) -> Result<Settings, Status>
// its DetectSettings literal ends `..base.clone()` where today it ends `..DetectSettings::cedar()` (:432)

// in configure (:477):
let settings = from_wire(request.into_inner(), &self.runtime.shared.detection)?;
```

`to_wire`, the link's messages, what a star frame reports and the auto-exposure cap
(`crates/scope-node/src/service.rs:305-313`) do not change. Nothing else in the file
changes.

### `crates/plate-solver/examples/solve_image.rs` (US2)

`Args` gains `noise_floor: Option<f64>`, `None` by default. `--noise-floor F` sets it,
and a value that is not a number calls `usage("bad --noise-floor")`. The detection call
at `crates/plate-solver/examples/solve_image.rs:145` becomes a match: `detect_stars` while it is `None`, and
`detect_stars_with_noise_floor` with it while it is `Some`, with the other arguments as
today. Three things gain `[--noise-floor F]` after `[--sigma S]`: the usage line (`crates/plate-solver/examples/solve_image.rs:97`),
the doc comment's command line, and the doc comment's option list, which describes the
flag as the scope node's `noise_floor`, left off for cedar's own 0.2. Examples are built
and linted by the gates and never run (§ Read from the diff).

### The node's README (US2)

US1's section gains, at its end, after its Setup's detection threshold (sigma)
paragraph:
- **The console and recordings.** The console's camera settings (Setup, MCP) set
  exposure, gain and sigma, and leave the floor as the config set it. A star frame does
  not report it, and neither does a raw recording's frame line, so a recording made with
  a floor set does not say how its centroids were found: note the floor beside the
  recording, or record with it unset.
- **The exposure cap.** Auto-exposure on the node never goes above 200 ms: its cap is
  the node's frame interval, 200 ms, or the camera mode's frame time when that is
  longer. On a dark sky with the floor set it holds 200 ms and raises gain while it
  finds fewer than 25 stars, up to the camera's top (63.9 on the OV5647). A 200 ms frame
  may still hold too few stars to solve; the fallback is a fixed longer exposure set in
  Setup, such as the 3000 ms at gain 16 that solved on a dark night. The README promises
  no star count (spec § Assumptions: the model's gain is noiseless).
- **Finding the value.** On a dark night, on the Pi the node will observe with (retune
  if that changes), with the floor unset, the node's `mode` the one observed with (such
  as `mode = "1296x972"`; `auto` takes the camera's first listed mode) and the mount
  still, record a few raw frames at a fixed 200 ms, the longest auto-exposure reaches,
  at each of gain 16, gain 32 and the camera's top (63.9 on the OV5647), the gains
  auto-exposure may reach with the floor set, and a few at a fixed 1000 ms and gain 1,
  with the console's `POST /api/record/raw` (the finder console's README). Then replay
  each `raw-<id>.png` with `cargo run --release -p plate-solver --example solve_image --
  --db <the database the console reports it uses> --noise-floor <F> <frame>`, trying a
  few floors. Take the least floor at which the 200 ms frames at gain 16 hold tens of
  stars, not hundreds, and solve (on the simulated sky 1215 with none, 46 at 0.5); if
  they already do with no floor, leave it unset. Then check that the frames at gain 32
  and at the top, where the camera's own noise is greatest, still solve at that floor;
  if they do not, take the next floor that solves all three. Last, check it live, with
  Setup's exposure and gain automatic: the node's exposure holds 200 ms and its gain
  rises while it finds fewer than 25 stars. If gain reaches its top with fewer than 25
  stars or the solve does not hold, lower the floor; if the counts then climb back to
  hundreds, set a fixed longer exposure in Setup instead.

## The skies

`crates/simulator/tests/dark/mod.rs` draws every frame of US1-S3 to US1-S5 and the sky
camera's. The bounds in those scenarios were measured on exactly this model, so it is
given exactly. A different random draw or draw order makes a different frame.

**Its public items**, which US1's and US2's tests use as they are; US2 does not edit the
module:

```rust
pub const WIDTH: u32 = 1296;
pub const HEIGHT: u32 = 972;
#[derive(Clone, Copy)]
pub struct Sky { pub sky: f64, pub black: f64, pub read: f64, pub flux: f64 } // levels at 1 s and gain 1
pub const DARK: Sky;                       // sky 0.05, black 2.0, read 2.0, flux 2.0
pub const CITY: Sky;                       // sky 30.0, black 0.0, read 3.0, flux 20.0
pub struct Field { pub stars: Vec<RenderStar>, pub truth: TrueDirection }
pub fn field() -> Field;                   // the field below, taken once
pub fn peak(flux: f64, star: &RenderStar) -> f64;
pub fn drawn(field: &Field, sky: Sky, scale: f64) -> Vec<RenderStar>;   // peak at least 0.5
pub fn render(field: &Field, sky: Sky, scale: f64, seed: u64) -> Vec<u8>; // the pixels, row-major
pub fn frame(pixels: Vec<u8>) -> GrayFrame;          // a frame for `detect`, below
pub fn floor(value: f64) -> DetectSettings;          // cedar's, with `noise_floor: Some(value)`
pub fn true_stars(centroids: &[Centroid], stars: &[RenderStar]) -> usize;
pub fn solve(field: &Field, centroids: &[Centroid]) -> Option<f64>;   // the console's solve's error in arcseconds, when it solves
```

`RenderStar` is `simulator::render::RenderStar`, `TrueDirection` is
`simulator::TrueDirection`, `Centroid` is `scope_link::proto::Centroid`, `GrayFrame` is
`scope_node::GrayFrame` and `DetectSettings` is `scope_node::detect::DetectSettings`.
`scale` is the exposure in seconds times the gain.

- **The database**: `reference-solutions/cedar-solve/tetra3/data/default_database.npz`,
  loaded once with `pattern_database::load_from_path` into a
  `LazyLock<Arc<PatternDatabase>>`, as `crates/simulator/tests/sky.rs` does.
- **The field**: `Simulator::new(db, Site::default(), VirtualClock::new(1_768_528_800.0))`,
  then `set_optics(Optics { width: 1296, height: 972, fov_deg: 13.0, eyepiece_px:
  (648.0, 486.0) })`, then `push_to(50.0, 120.0)`. The stars are
  `field_stars(2000.0, 16.0)`'s `RenderStar`s, in its order. The truth is
  `telescope().camera`, a `TrueDirection` in degrees. Both are taken once per test, so the
  sky does not turn during it.
- **A sky** has four levels at 1 s and gain 1: the sky level, `black`, the read noise and
  the flux, the peak of a magnitude-6.75 star (`simulator::render::REFERENCE_MAGNITUDE`).
  `black` is the level the camera subtracts before clipping at 0, so that a pixel with
  no light reads 0.
- **The scale** `s` is the exposure in seconds times the gain. The sky level and the flux
  scale with `s`; `black` and the read noise do not.
- **A star's peak** is `flux · s · 10^(-0.4 · (magnitude - 6.75))`. **The drawn stars**
  are the field's stars whose peak is at least 0.5, in the field's order.
- **The light**: every pixel starts at `sky · s`. Each drawn star with peak `p` adds
  `p · exp(-r² / (2 · 1.2²))` to every pixel `(x, y)` with `x` from `floor(star.x) - 8`
  to `floor(star.x) + 8` and `y` likewise, clipped to the frame. Here `r²` is
  `(x + 0.5 - star.x)² + (y + 0.5 - star.y)²`, and 1.2 is
  `simulator::render::PSF_SIGMA_PX`.
- **The noise**: a SplitMix64 generator, state `seed`. Each draw adds
  `0x9e3779b97f4a7c15` to the state, then mixes it with the steps of
  `crates/simulator/src/noise.rs:18-24` (shift 30 and multiply `0xbf58476d1ce4e5b9`,
  shift 27 and multiply `0x94d049bb133111eb`, shift 31). A uniform is
  `((draw >> 11) as f64 + 0.5) / 2^53`. A normal is `sqrt(-2 · ln(u1)) · cos(2π · u2)`,
  where `u1` is drawn before `u2`. The frame first draws `(1296 + 1) · (972 + 1)`
  normals in a row-major grid of width 1297. Pixel `(x, y)` takes
  `n = (g[y][x] + g[y][x+1] + g[y+1][x] + g[y+1][x+1]) / 2`, which keeps one standard
  deviation and is shared with its right and lower neighbours (why shared: spec.md
  § Assumptions).
- **A pixel's value** is `clamp(round(light - black + read · n), 0, 255)` as `u8`.
- **The dark-sky frame** is the dark sky at `s = 1` from seed 13; **the city frame** is
  the city sky's.
- **A true star** is a centroid within 1.5 px of a drawn star, measured with `hypot` on
  `(x, y)`.
- **The console's solve** and **the sky camera** are as spec.md § User Scenarios defines
  them. The sky camera lives in `dark_sky_exposure.rs` (US2). Its frame `k`, counted from
  1, is `render(field, DARK, s, 13 + k)` at the exposure and gain the camera is set to,
  200 ms when exposure is automatic. It sleeps its exposure on tokio's clock and reports
  that exposure and gain. Its `info` gives one mode, 1296x972 at 46.34 frames/s, with
  the finder camera's listed ranges of spec.md: `exposure_min_ms` 0.13,
  `exposure_max_ms` 3066.985, `gain_min` 1 and `gain_max` 63.9375. Its `apply` checks the settings with
  `camera::check_range` and keeps them.
- **A frame for `detect`** in US1 is a `GrayFrame` of the pixels at 1296x972, its times
  0 to 1 s, at 1000 ms and gain 1. `detect` reads only the pixels and the size.

What the model gives (run on the probe): the dark-sky frame has 1,120,187 of 1,259,712
pixels at 0 (88.9 %) and 172 drawn stars. At 3000 ms and gain 16, the settings that
solved on the dark night, the dark sky drawn from seeds 13 to 18 gave 162 to 164
detections without a floor, 161 to 163 of them true, every frame solving within 6″; set
as a fixed exposure in Setup at the node's own interval, 162 to 171 from the second
frame, each solving within 5.8″.

## What was run and what was read

**Run on 2026-11-22 (UTC), before the draft**, on the branch `probe/048` cut from `main`
at b7e983a0. The trial fix was as § Interfaces then gave it, with a third story of
adapter tables since dropped, and the tests as the draft's scenarios gave them. Each item
ran three times with the same outcome unless it says otherwise:

- **On `main`'s code.** The probe checked out `main`'s `crates/star-detection` and
  `crates/scope-node` sources and ran a copy of the dark-sky tests without their floor
  parts. US1-S3, the draft's US2-S1 and US1-S5's solve without a floor passed. The other
  scenarios' tests name `detect_stars_with_noise_floor` or `noise_floor`, which `main`
  lacks, and do not build there.
- **The draft's 15 scenarios' tests with the fix**, with the 15 tests beside them in
  `parity.rs`, `crates/scope-node/tests/detect.rs`, `cli.rs`, `imaging_config.rs` and
  `mount_config.rs`. All 28 passed.
- **Both gates' Rust commands with the fix**: `lint`'s four and `test`'s two. Every
  existing test of the workspace passed unchanged. This ran twice, once at each of the
  probe's last two layouts, both with the third story.
- **US1-S3's frame from six seeds**, 13 to 18, once. Each gave 200 detections with 88.8
  to 89.0 % of its pixels at 0, 8 to 14 of the detections true and 6 to 9 of the 30
  brightest true. The console's solve passed for 3 of the 6, within 16.6″, 15.5″ and
  27.8″. Seed 13, the dark-sky frame, is one of the three: 14 true detections, 8 of them
  among the 30 brightest, solving within 16.6″. With floor 1.0 each seed gave 4 or 5
  detections, all true.
- **The city frame**: 30 detections, the console's solve within 0.5″, and identical
  centroids at floors 0.5 and 1.0.
- **The loop, at a 3000 ms frame interval with a first automatic frame of 1000 ms**,
  which is not the node's (review round 1): without a floor from gain 16, (1000, 16),
  (1000, 4.24), (1000, 1.12), (1000, 1), (265, 1), (70.2, 1), (18.6, 1) and (4.93, 1),
  every frame at 200; with floor 1.0 after pairing, from the third frame 3000 ms and gain
  1.90 with 31 to 34 detections. The fold's run replaces these for US2.
- **Owner decision 2 (c) with 1.0, once**: with `DetectSettings::cedar()` setting a floor
  of 1.0, two existing tests failed,
  `crowded_frame_keeps_the_detectors_brightest_two_hundred_in_order`
  (`crates/scope-node/tests/detect.rs:68`) and
  `raw_pixels_are_exactly_what_detected_the_streamed_stars`
  (`crates/scope-node/tests/raw_frame.rs:68`), and five of the new tests that state
  today's behaviour.
- **Owner decision 5 (b), trial B, once, at the same 3000 ms interval**: auto-exposure
  holding at 200 detections. From gain 16 it held 1000 ms and gain 16, and all 8 frames
  solved within 2″, the eighth at 145 detections. From gain 1 it held 1000 ms and gain 1
  on 200 noise points, and 2 frames of 8 solved; the eighth fell to 151. Either fall
  would step it down again. It failed
  `proportional_step_is_clamped_to_four_in_each_direction` and
  `target_band_holds_and_gain_waits_for_exposure_cap`
  (`crates/scope-node/src/exposure.rs:61`, `crates/scope-node/src/exposure.rs:73`). The trial was reverted.
- **`solve_image` on three frames the probe saved as PGM**, once each: the dark-sky frame,
  the dark sky at 3000 ms and gain 1.8, and the city frame.
  - The dark-sky frame: with no floor, 1094 stars and a match with 10 stars at 80″ RMS; with
    0.5, 20 stars and no match; with 1.0, 5 stars and no match; with 2.0, 3 stars, too
    few to try.
  - The 3000 ms frame: 1291, 75, 32 and 13 stars at those floors, every one solving.
  - The city frame: 30 stars, solving, at every floor.
- **Independent noise, once**, in the probe's exploration before the model was
  committed: the dark sky's levels with noise drawn per pixel gave 75 detections on the
  1 s frame, which solved within 5.3″.
- **`--check-config`, `main`'s binary against the fix's**, once. The README's example
  config and `cli.rs`'s printed the same with both. On `main`, `noise_floor = 1.0` failed
  with `invalid noise_floor at line 5`.

**Run in the fold of review round 1** (2026-11-22, UTC), on `probe/048` from 9503712a,
adding commits 41fc7833 (the third story dropped: `config.rs` back to `main`'s with US1's
floor check, its test and README section gone), 03310e63 (the range 0.2 to 5 and US1-S7's
cases), 7f1b8908 (a print-only sweep, `crates/simulator/tests/p048_fold.rs`, not a
scenario), d98169fa (US2's rig and scenarios at the node's own interval, and the README's
text), 7b4279d7 (the sweep saves frames for `solve_image`), 5d9032c6 (clippy in the
sweep) and 46c2269d (a README cross-reference). Once each unless it says otherwise:

- **The sweep**: US2's rig as spec.md then defined it, `NodeService::new` at 46.34
  frames/s with a first automatic frame of 200 ms, at six seed bases (frame `k` from seed
  `b + k`, `b` 13, 113, 213, 313, 413 and 513). Its sky camera had the simulator's
  ranges, gain 1 to 16 and exposure 1 to 3000 ms; review round 2 found the finder camera
  lists gain to 63.9375, and the fold of round 2 reran every row at the listed ranges
  (below). Its figures replace these where gain reached 16: floor 1.0 and sigma 20 to 40.
  - No floor, from gain 16: (200, 16), (200, 4.24), (200, 1.12), (200, 1), then 53, 14,
    3.7 and 1 ms at gain 1 (at one base 64.8, 17.2, 4.6 and 1.2 ms, after a frame of 150
    detections); every frame 150 to 200 detections; only the first solved. From gain 1:
    200, 53, 14, 3.7 and 1 ms, 147 to 200 detections, none solving.
  - Floor 0.5 after pairing, from gain 1: every frame at 200 ms; gain rose to 9.9 to
    14.6; the 6th to 12th frames held 23 to 43 detections, 6 or 7 of the 7 in 25 to 40;
    every frame from the fourth solved, the worst within 25.4″. From gain 16: every frame
    at 200 ms and solving, within 27.7″; the 6th to 12th held 24 to 48 at gains 11.4 to
    16.
  - Floor 1.0, from either gain: gain 16 by the third frame, 11 to 19 detections, below
    the band, every frame from the third solving within 6.4″.
  - Option (e), no floor and Setup's sigma sent on pairing, from gain 1: sigma 20 held 14
    to 37 detections on the 6th to 12th frames at gains 8.8 to 16, 3 to 7 of the 7 in 25
    to 40, every frame solving from the fifth or sixth, within 18.9″; sigma 30 pinned
    gain at 16 with 6 to 23, 11 or 12 of 16 frames solving; sigma 40 at 16 with 6 to 15,
    10 to 12 of 16 solving. The first frame held 200 in each: it starts before
    `Configure` applies.
  - Setup's fixed 3000 ms and gain 16, no floor: 162 to 171 detections from the second
    frame, every frame solving within 5.8″. Fixed 3000 ms with gain automatic and floor
    0.5: gain fell to 1, the 6th to 10th frames holding 29 to 44, every frame solving
    within 6.5″.
- **Single frames, seeds 13 to 18**: on the 1 s dark-sky frame sigma 20 gave 9 to 20
  detections, 5 to 7 true, and sigma 40 4 or 5, all true, as floors 0.5 (10 to 20, 6 or 7
  true) and 1.0 (4 or 5, all true) did. On the city frame sigma 20 gave 6 to 8 detections
  where sigma 8 gave 29 to 33, 4 of 6 solving, and sigma 30 and 40 gave 4 to 6, none
  solving; floors 0.5 and 1.0 changed nothing. At 200 ms and gain 16, floor 0.5 gave 32
  to 46 detections, 27 to 32 true, and 1.0 12 to 16, all true, every frame solving.
- **`solve_image` on the dark sky at 200 ms and gain 16**, seeds 13 and 14, saved as PGM:
  with no floor 1215 and 1204 stars, solving; with 0.5, 46 and 32, solving; with 1.0, 13
  and 12, solving; with 2.0, 6, no match. The README's procedure gives 0.5 here, as on
  the 3000 ms frame.
- **US2's tests at the tip, three times**: both passed. **With `main`'s `service.rs`**,
  whose `Configure` resets the floor: US2-S1 passed and US2-S2 failed at its fourth
  frame, at 53 ms.
- **US1-S7's test with the range 0.2 to 5, three times**: passed.
- **The nine scenario tests with 67 beside them**, at the tip, three times: `parity.rs`,
  the node's `detect.rs`, `raw_frame.rs`, `cli.rs`, `imaging_config.rs`,
  `mount_config.rs` and unit tests, and the simulator's `node.rs` and `sky.rs`. All 76
  passed.
- **clippy** with warnings denied on `star-detection`, `scope-node` and `simulator`, all
  targets and features: clean. `cargo fmt --check` flags only an earlier print-only file.
- **Owner decision 2 (c) with 0.5**, on the same three crates' tests:
  `raw_pixels_are_exactly_what_detected_the_streamed_stars` failed, and the new tests of
  US1-S3, US1-S6 and US2-S1; the finder console's tests were not run.
- **Size**: § Plan traps 2. The fold did not rerun the whole gates.

**Run in the fold of review round 2** (2026-11-22, UTC), on `probe/048` from 46c2269d,
adding commits c851c407 (the print-only sweep's sky camera at the finder camera's listed
ranges, floors 1.5 to 5, and US2's clauses over the review's 45 seed bases), bb976d00
(US2's sky camera at the listed ranges, US2-S2 without "below 16"), a4437bc0 (US1-S3's
and US1-S4's tests renamed `dark_sky_frame_*`) and adbda24f (the README text of
§ Interfaces). The sky camera had gain 1 to 63.9375 and exposure 0.13 to 3066.985 ms;
rows from gain 1 unless they say otherwise, six seed bases, counts of the 6th to 12th
frames. Once each unless it says otherwise:

- **No floor**: as in round 1, gain 16 to 1 at 200 ms, then 53, 14, 3.7 and 1 ms by the
  eighth frame and on to the listed least, 0.13 ms; 150 to 200 detections, only the
  first frame solving. From gain 1, none solving.
- **Floor 0.5**: as in round 1, since its gain never reached 16: every frame at 200 ms,
  gains 9.9 to 14.6, 23 to 43 detections, 6 or 7 of 7 in 25 to 40, every frame from the
  third or fourth solving, the worst within 25.4″.
- **Floor 1.0**: gain settled at 23.8 to 32.3 from the fourth frame, 26 to 40 detections,
  7 of 7 in the band at every base, every frame from the third solving within 4.6″ (from
  gain 16, gains 24.8 to 30.4 and every frame solving). Floors 1.5 and 2.0 settled at
  gains 37.2 to 49.4 with 29 to 39, 7 of 7 in the band. Floor 3.0 reached gain's top at 5
  of 6 bases with 23 to 31. Floors 4.0 and 5.0 held gain at its top from the fourth frame
  with 18 to 22 and 11 to 15, none in the band, every frame from the fourth solving.
- **Setup's sigma, no floor**, sent on pairing (the first frame, begun before `Configure`
  applies, held 200): sigma 20 held 15 to 45 at gains 8.8 to 24.7, 3 to 7 of 7 in the
  band, every frame from the fifth or sixth solving, the worst within 18.9″; sigma 30 16
  to 41 at gains 22.8 to 38.6, 5 to 7 of 7; sigma 40 17 to 36 at gains 28.7 to 51.4, 6
  or 7 of 7; both solving from the fifth.
- **Setup's fixed 3000 ms**, with gain 16 or automatic: as in round 1.
- **US2's clauses over the review's 45 seed bases**: US2-S1 held at 44 (a first frame of
  59 at one); US2-S2, as now written, at 43 (19 detections at two); its first solve fell
  on the fourth frame at 12; its worst error from the fourth was 25.4″; its highest gain
  was 16.5, which the dropped "below 16" would have failed.
- **US2's tests, three times**: both passed. **With `main`'s `service.rs`**: US2-S1 passed
  and US2-S2 failed at its fourth frame on the 200 ms clause, at 53.002 ms.
- **US1-S3 to US1-S5's tests, renamed, three times**: passed.
- **clippy** with warnings denied on `star-detection`, `scope-node` and `simulator`, all
  targets and features: clean. `cargo fmt --check` flags only the earlier print-only
  `p048_rerun.rs`.
- **The review's run**, review round 2 (logs in `probes/048/review-r2/`): the whole of
  `nextest` at 28f436df with the probe tip 46c2269d's story files, 1452 of 1452 passing;
  clippy with `-D warnings` and both doc builds clean, the doctests passing, and fmt
  flagging only `p048_rerun.rs`. It is the run behind "every existing test passes
  unchanged with both stories built", and it covers 047's finder tests beside 048. The
  fold's later commits change test files and the README only, rerun above. Its margins
  over 45 bases and 62 seeds are in § Plan traps 6.

**Run in review round 3 and its fold** (2026-11-22, UTC). The review reran the gates in
a scratch worktree of the probe's tip adbda24f carrying 28f436df's `crates/finder/**`
and `Cargo.lock`, so 28f436df plus the stories' files (logs in `probes/048/review-r3/`):
- **`nextest`**, the whole workspace: 1474 of 1474 passing, round 2's 1452 and the 22
  print-only `p048_*` tests, 047's finder tests among them. It is now the run behind
  "every existing test passes unchanged with both stories built".
- **clippy** with `-D warnings`, both doc builds and the doctests: clean; fmt flags only
  the print-only `p048_rerun.rs`, which no story writes.
- **With `main`'s `service.rs`**: US2-S1 passed and US2-S2 failed at its fourth frame on
  the 200 ms clause, at 53.002 ms.
- **The checks**: the validator rc 0 with no findings; the 93 backticked cites resolve
  at 28f436df.

The fold of round 3 changed wording only and ran nothing on the sky. It added commit
4eaa4ae8 on `probe/048`, the README text of § Interfaces (the floor left out in town,
200 ms frames at gains 16, 32 and the top, Setup's detection threshold (sigma)), which
writes only `crates/scope-node/README.md`, and re-measured the README's share of each
story (§ Plan traps 2).

**Run in review round 4 and its fold** (2026-11-22, UTC). The review reran the validator
and the judge's criteria parse only (logs in `probes/048/review-r4/`): rc 0, no findings,
US1 7 and US2 2; and the 96 backticked cites resolved at 28f436df. No gate was rerun:
4eaa4ae8 changes only `crates/scope-node/README.md`, which no gate reads (no crate
includes it, no `ci.yml` job reads it), so round 3's 1474 of 1474 stands. The fold
changed wording only and ran nothing but the checks. It added commit 174a48dc on
`probe/048`, the README text of § Interfaces (sigma 20 and 40 as floors of 0.5 and 1.0
on a clipped background, "Finding the value" as one rule, the Pi to tune on, the
database the console reports), which again writes only the README, so no gate rerun is
needed, and re-measured the README's share of each story (§ Plan traps 2).

**Read, not run**:
- that the dark night's Pi was the 3B+, whose image processor is the VC4, and that 046
  reads the Pi 5's frames, the same OV5647 through the PiSP
  (`specs/046-node-reads-pi-5-frames/spec.md:47`,
  `specs/046-node-reads-pi-5-frames/spec.md:50`,
  `fixtures/scope-node/pi5/list-cameras.txt:14`), so the floor is tuned on the Pi that
  observes;
- that since 047 the console may hold several databases and names the one in use
  (`crates/finder/README.md:243`), which `solve_image --db` is given;
- that Setup labels sigma "Detection threshold", in σ, with the hint "1–50 σ, or auto
  for cedar's 8 σ" (`crates/finder/web/src/components/ScopeNodeSection.tsx:106-107`,
  shown in Setup at `crates/finder/web/src/components/SetupPage.tsx:223`);
- that at binning 2 the detection image averages each 2x2 block
  (`crates/star-detection/src/binning.rs:118`), halving independent pixel noise, so a
  town sky's estimate on the finder camera may sit near 0.5 (spec Edge Cases);
- the console's pairing and settings path (spec Clarifications), and that the dark
  night's settings were not recorded;
- the finder camera's listed ranges (`fixtures/scope-node/list-cameras.txt:17`,
  `fixtures/scope-node/list-cameras.txt:27`), as the rpicam adapter reads them into its
  camera info (`crates/scope-node/src/rpicam/listing.rs:68-80`);
- that a sensor's analog gain amplifies its own shot and read noise, which the drawn sky
  leaves out (spec Assumptions);
- that with `mode = "auto"` the rpicam adapter takes the first listed mode, 640x480
  (`crates/scope-node/src/rpicam/mod.rs:81-83`, `fixtures/scope-node/list-cameras.txt:4`);
- that the rpicam adapter's first automatic frame is at most 200 ms: `rpicam-vid` is
  started without a shutter at 5 frames/s at most (`crates/scope-node/src/rpicam/mod.rs:174-186`),
  and the camera stack chooses the exposure within that frame;
- that the Pi camera's YUV passes through its image processor's tone curve (spec
  Assumptions);
- that `POST /api/record/raw` saves each frame's 8-bit pixels as `raw-<id>.png` in the
  recording's folder (`crates/finder/src/recording.rs:395`, `crates/finder/src/recording.rs:409`;
  `crates/finder/src/api.rs:626`), and that it is refused while the mount moves (the
  finder console's README). Nothing in the repository replays a recorded frame through
  the node's detection. `solve_image` reads PNG and PNM (`crates/plate-solver/examples/solve_image.rs:101`), and
  `crates/finder/tests/console/recording_raw.rs:209` detects a recorded frame in a test;
- cedar-detect's floor and tetra3's threshold (§ Constitution Check);
- that a 12-bit camera's frames reach detection as 8 bits, through spec 045's adapter
  (the constitution's product limits); 045 is not drafted;
- every `path:line` in the trio, at 28f436df: each cited line was compared with its
  text at b7e983a0, and the two that moved were read again; review round 4 resolved all
  96 there, and the cites the fold of round 4 added were read there.

**Not run at all**: a camera, a Pi, a recorded frame of a real sky, the console, the
page, the page half of the `test` gate (`npm ci` installs packages, and no story writes
anything the page is built from), and the factory's gate boundary.

## Read from the diff, not from a test

- **`solve_image --noise-floor`** (FR-006). Examples are compiled and linted by the gates
  and never run, so the flag, its usage line and its doc lines are the diff's. The probe
  ran it by hand on its saved frames (§ What was run).
- **The README sections** (FR-004, FR-005).
- **`exposure.rs` unchanged** (FR-008), and the files FR-009 names untouched: the diff's
  file list.

## Shapes that existing tests pin

Every existing test passes unchanged with both stories built: review round 3 ran the
whole of `nextest` at 28f436df with the story files of the probe's adbda24f, 1474 of
1474, 22 of them the probe's print-only tests (§ What was run). The files beside the
change, and what the stories do to what they observe:

- **`crates/star-detection/tests/parity.rs`**: `cedar_detect_parity_on_all_reference_images`
  (`crates/star-detection/tests/parity.rs:107`) calls `detect_stars` (`crates/star-detection/tests/parity.rs:113`) at binning 1 and sigma 8, without row
  normalisation, against cedar-detect's goldens. `detect_stars` now goes through the new
  function with 0.2, which changes no estimate (§ Constitution Check I).
- **`crates/scope-node/tests/detect.rs`**: `cedar_defaults_equal_every_console_field`
  (`crates/scope-node/tests/detect.rs:10`) compares named fields only, and the console's `DetectParams` has no floor.
  `saved_finder_frame_has_identical_nonempty_centroids_in_console_order` (`crates/scope-node/tests/detect.rs:26`) and
  `crowded_frame_keeps_the_detectors_brightest_two_hundred_in_order` (`crates/scope-node/tests/detect.rs:68`) detect with
  `DetectSettings::cedar()`. That has no floor, so `detect` takes the `detect_stars` path.
- **`crates/scope-node/tests/raw_frame.rs`**: `raw_pixels_are_exactly_what_detected_the_streamed_stars`
  (`crates/scope-node/tests/raw_frame.rs:68`) runs the node with cedar's settings and holds its centroids to `detect_stars`
  (`crates/scope-node/tests/raw_frame.rs:95`). It does not change.
- **`crates/scope-node/tests/cli.rs`**: `check_config_prints_every_setting_and_masks_the_token`
  (`crates/scope-node/tests/cli.rs:53`) and `start_refuses_unknown_keys_and_bad_values_naming_the_key` (`crates/scope-node/tests/cli.rs:86`) read
  what the output and the errors contain. Their configs have no floor, so the printed
  `[detection]` is today's (US1-S6 holds it exactly).
- **`crates/scope-node/tests/imaging_config.rs`** and **`mount_config.rs`**: they read the
  `[imaging]` and `[mount]` tables' errors and prints, which US1's floor check does not
  touch.
- **The unit tests in `crates/scope-node/src/config.rs`**:
  `minimal_config_uses_automatic_controls_and_cedar_defaults` (`crates/scope-node/src/config.rs:342`) holds
  `settings()` to `Settings::default()`, whose detection has no floor.
  `strict_values_and_nested_keys_fail_with_key_names_without_token`
  (`crates/scope-node/src/config.rs:351`) needs `sigma`, `binning` and `sigam` in the errors; the floor's check comes
  after binning's, and those messages are today's. `camera_mode_is_auto_or_a_size_and_is_printed` (`crates/scope-node/src/config.rs:377`) reads `mode`,
  which does not change.
- **The unit tests in `crates/scope-node/src/exposure.rs`** (`crates/scope-node/src/exposure.rs:61`, `crates/scope-node/src/exposure.rs:73`): the file does
  not change.
- **`crates/simulator/tests/node.rs`**: its nine tests (`crates/simulator/tests/node.rs:71` to `crates/simulator/tests/node.rs:473`) run the node with
  `Settings::default()`, whose detection is cedar's. Their `Configure` calls (`crates/simulator/tests/node.rs:198`,
  `crates/simulator/tests/node.rs:302`, `crates/simulator/tests/node.rs:334`, `crates/simulator/tests/node.rs:420`, `crates/simulator/tests/node.rs:456`) therefore fill the keys the link does not carry from
  cedar's, exactly as today.
- **`crates/simulator/tests/sky.rs`**: it detects with `DetectSettings::cedar()` (`crates/simulator/tests/sky.rs:62`,
  `crates/simulator/tests/sky.rs:125`, `crates/simulator/tests/sky.rs:260`), which does not change.
- **Every other caller of `detect_stars`** keeps its results:
  `crates/plate-solver/tests/image_parity.rs:152`,
  `crates/finder/tests/console/recording_raw.rs:209`, `crates/finder/src/solve.rs:392`,
  `crates/grpc-service/src/server.rs:60` and `crates/plate-solver/src/solve.rs:128`.
- **The link's tests** (`crates/scope-link/tests/link.rs:36`,
  `crates/finder/tests/console/link.rs:337`) build `proto::Detection`, which does not
  change.

`--check-config` neither opens the camera nor checks `rpicam`'s presence, so the config
tests run on a machine without one (run on the probe).

## Decisions and the alternatives rejected

| Decision | Rejected | Why |
|---|---|---|
| `noise_floor`, in 8-bit levels, 0.2 to 5 | (b) another name, such as `min_noise`; (c) a share of full scale, such as 0.004 for one level; (d) the camera's own units, scaled by the adapter's conversion | Owner decision 1, answered. cedar-detect's name and unit; (c) is in no unit cedar uses; (d) ties the setting to 045's conversion, not designed yet; detection reads 8 bits for every camera adapter. 0.2 is cedar's own value. 5 is the most that still lets a star through at the node's highest sigma, 50: above 5.1 the gate `2·σ·floor` exceeds 510, which no 8-bit pixel reaches (`crates/star-detection/src/detect.rs:48`, `crates/star-detection/src/detect.rs:146`). The row prefilter's `as u8` (`crates/star-detection/src/detect.rs:116`) already wraps above a gate of 255 today, at sigma 26 or more with an estimate of 5; the gate itself still holds. |
| Off by default; the README gives 0.5 | (b) a default for rpicam; (c) a default for every adapter, in `DetectSettings::cedar()` | Owner decision 2, answered. (c) fails existing tests, whose edits would need approval (§ What was run); (b) costs faint stars in town unseen, since a city sky's real noise on the OV5647 is not measured. 0.5 is what the README's procedure gives on the probe's frames; US1-S4 uses 1.0 to show the mechanism (spec US1). |
| Inside `star-detection`, in a new function, with Setup's sigma documented beside it | (b) the node passes an estimate; (c) filter centroids; (d) lift or clip the frame; (e) Setup's sigma, no code, alone | Owner decision 3, answered: (a), with (e) in the README as the no-code option (FR-004). (a) keeps `detect_stars` and parity by construction, and writes `crates/star-detection` beside the scope node; (b), cedar-detect's own caller-supplied `noise_estimate` (`reference-solutions/cedar-detect/src/algorithm.rs:966`), bins twice or exports the cascade; (c) leaves the gates open; (d) changes cedar's input and cannot restore clipped noise. (e): on a background clipped to 0 the estimate is 0.2 and the gates and the perimeter check scale with σ·noise (`crates/star-detection/src/detect.rs:48-49`, `crates/star-detection/src/detect.rs:335`), so sigma 20 acts much as floor 0.5 at sigma 8, and sigma 40 as floor 1.0; on the 1 s dark-sky frame sigma 20 gave 20 detections, 7 true, as floor 0.5 did, and sigma 40 4, all true. On the drawn sky at the node's interval and the camera's listed gain range, sigma 20, 30 and 40 held 15 to 45 detections on the 6th to 12th frames at gains 8.8 to 51.4, 3 to 7 of 7 in 25 to 40, every frame from the fifth or sixth solving; floor 0.5 held 23 to 43 at gains 9.9 to 14.6, 6 or 7 of 7, from the third or fourth (§ What was run, round 2). In town (e) costs stars: sigma 20 drops the drawn city frame from 30 detections to 8, and 30 or 40 leaves it unsolved. The floor adapts by itself, acting only where a sky's own noise is below it, so it left the drawn city sky alone (US1-S5) and needs no change of Setup between sites; a town sky's noise on the finder camera is not measured (decision 2's row), so the README leaves the floor out in town or has the observer compare star counts with and without it (review round 3). |
| Raise the estimate after `estimate_noise` | Change `estimate_noise`'s floor | `estimate_noise` is public and keeps its signature and results; the floor belongs to the caller. |
| `Option<f64>`, absent by default | `f64` defaulting to 0.2 | Absent keeps today's print exact (`skip_serializing_if`) and today's `detect_stars` path. |
| `Configure` keeps the node's own keys | Reset to cedar's as today; carry the floor on the link | Owner approval 2. Without it the console's pairing drops the floor (the mutation); the link is a later spec's to change (owner decision 7). |
| No change to auto-exposure or its cap | (b) hold at the 200-detection cap, treating it as "unknown"; (c) a black-sky guard, no lowering while the estimate sits at 0.2; (d) raise the cap, as a config key or a longer default frame interval; (e) Setup's sigma | Owner decision 5, answered (a). On the drawn sky at the node's 200 ms cap, with floor 0.5, it holds 200 ms, raises gain to 9.9 to 14.6, and every frame from the fourth solves (US2-S2); gain is noiseless there (spec § Assumptions). (b), measured at a 3000 ms interval, fails two existing tests (`crates/scope-node/src/exposure.rs:61`, `crates/scope-node/src/exposure.rs:73`) and holds a dark start on noise (§ What was run); (c) needs `star-detection` to report its estimate; (d) changes `crates/scope-node/src/service.rs:305-313` or the default frame interval (`crates/scope-node/src/service.rs:58`) that many existing tests' nodes run at, and makes the node's frames as slow as its exposure; (e) reaches the band at sigma 20 to 40 on the drawn sky but costs the drawn city sky stars (decision 3's row). |
| `solve_image --noise-floor` | (b) no tool: the seat replays recorded frames in a probe branch; (c) a node flag that detects a PNG | Owner decision 6, answered. The example already reads a raw frame's PNG and solves it with cedar's settings; it writes a `crates/plate-solver` example beside the scope node. (c) needs PNG decoding in the node's binary and a `Cargo.toml` change, and cannot solve. |
| No floor in the console in 048 | (b) a later spec carries the floor on the link's `Detection` | Owner decision 7, answered. `--check-config` and the README show the floor. (b) is a `proto/` change after which Setup and MCP show it and star frames and recordings report it again (owner approval 3); 047's stories wrote `crates/finder/**` and `Cargo.lock`, none of this spec's files. |
| US2's rig at the node's own interval | The draft's 3000 ms interval and 1000 ms first frame | The node's binary runs `NodeControls::default()`, whose 200 ms interval caps auto-exposure (review round 1); a longer interval tested a node that does not exist. |
| US2's sky camera with the finder camera's listed ranges | The simulator's gain 1 to 16 and exposure 1 to 3000 ms | The OV5647 lists gain to 63.9375 (`fixtures/scope-node/list-cameras.txt:17`), and the rpicam adapter gives auto-exposure that top; at 16 the rig's own ceiling stood in for the camera's, and since exposure is capped at 200 ms gain is US2's only lever (review round 2). "Below 16" left US2-S2 with it. |
| The config tests run the built binary | `Config::parse` alone | `--check-config` is what the observer runs; `cli.rs` does the same. Each also checks `Config::parse` for the settings. |
| The skies in a new `tests/dark/mod.rs` | `tests/support/mod.rs`; one file per story | `support/mod.rs` is an existing test file, which no story edits; US2's test reads US1's skies. |
| Two stories | One; the draft's three | One would be about 38,500 B, 58.8 % of 65,536 B before any overrun and about 52,000 B at 1.35 times, over the 39,321 B budget; the floor (US1) lands alone. The third story, per-camera detection tables, is 045's (owner decision 4). |

## User Story 1 - Detection has a noise floor

Tests first: `crates/star-detection/tests/noise_floor.rs`,
`crates/simulator/tests/dark/mod.rs` with `dark_sky.rs`, and
`crates/scope-node/tests/noise_floor.rs`. Then the floor in `star-detection`,
`DetectSettings::noise_floor` and `detect`, the check in `config.rs`, and the README
section.

## User Story 2 - Auto-exposure finds the stars on a dark sky

Tests first: `crates/simulator/tests/dark_sky_exposure.rs`, holding the sky camera, the
node rig and the pairing settings, and declaring US1's `mod dark;`. Then `Shared`'s
detection and `from_wire` in `service.rs`, `--noise-floor` in `solve_image`, and the
README's later paragraphs.

## Verification (every story, before proposing)

- **The factory gates** (`ergane.yaml` at 28f436df, `ergane.yaml:83-107`):
  - `lint` = `cargo fmt --all --check`,
    `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`,
    `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features --locked`
    and `RUSTDOCFLAGS="-D warnings" cargo doc -p finder --no-deps`;
  - `test` = `cargo nextest run --workspace --all-targets --all-features --locked`,
    `cargo test --workspace --locked --doc`, and then, in `crates/finder/web`,
    `npm ci --no-audit --no-fund`, `npm run typecheck`, `npm run format:check`,
    `npm test`, `npm run build`, `git status --porcelain -- dist | (! grep .)` and
    `CI=1 npm run e2e`.

  The story runs both, and the judge sees their results.
- **CI in the merge queue** runs every required `ci.yml` job.

## Prerequisites before this spec is flipped to `ready`

1. **The earlier specs have landed:** every spec in `depends_on_landed`. Met at 28f436df.
2. **`main` is still as read.** Every citation here was read at 28f436df. If `main`
   moves in a file § Project Structure names, or in a test § Shapes names, re-read them
   and re-run the validator. 047 landed at 28f436df and wrote only `crates/finder/**`
   and `Cargo.lock` (its Refinement Record), none of this spec's files.
3. **The owner decisions are settled and the owner approvals given** (spec § Owner
   decisions, § Owner approvals). Met for the decisions: all seven were answered on
   2026-11-22, and approval 3 was approved then; the flip gives the approvals.
4. **Probe builds.** Not met: no builder has built a story from this trio alone.

## Outside the factory

Written here and not run.

**The exposure cap.** On the node, auto-exposure never goes above 200 ms (owner
decision 5 (a)). A longer exposure, such as the 3000 ms at gain 16 that solved on the
dark night, needs a fixed exposure set in Setup; auto-exposure then moves only what is
left automatic.

**Recording frames**, by the owner, on the next dark night, before or after 048 lands,
with the floor unset in the node's config (owner approval 3: "record [the dark night's]
frames with the floor unset"):

1. On the Pi the node will observe with (retune if that changes: the dark night's was
   the 3B+, and 046 reads the Pi 5's frames), set `mode = "1296x972"` in the node's
   config, the mode to observe and tune with. Pair the console with the scope node at
   the dark site. Set Setup's exposure and gain to fixed values, starting at 200 ms and
   gain 16, the longest auto-exposure reaches.
2. Start a recording, keep the mount still, and `POST /api/record/raw` with
   `{"frames": 4}`.
3. Repeat at 200 ms and gain 32 and at 200 ms and gain's top (63.9 on the OV5647), the
   gains auto-exposure may reach with the floor set (on the drawn sky floors of 0.5 to 2
   settled at gains 9.9 to 49.4, and floors of 3 and more at or near the top), where the
   camera's own noise, which the drawn sky leaves out, is greatest; at 1000 ms and gain
   1, the night's failing exposure; and at 3000 ms and gain 16, the settings that
   solved: twenty frames in all.
4. Copy the recording's `raw-<id>.png` files to the console's computer.

**Setup's detection threshold (sigma), the no-code check** (owner decision 3, which the
README documents beside the floor), the same night, with the floor unset and Setup's
exposure and gain automatic: set Setup's "Detection threshold"
(`crates/finder/web/src/components/ScopeNodeSection.tsx:106-107`) to 20, then 30, then
40, and note at each the star count, the gain and whether the live solve holds. On the
drawn sky, at the camera's listed gain range and 200 ms, sigma 20 held 3 to 7 of the 6th
to 12th frames in the band of 25 to 40 stars at gains 8.8 to 24.7, sigma 30 5 to 7 at
22.8 to 38.6 and sigma 40 6 or 7 at 28.7 to 51.4, all below gain's top (63.9 on the
OV5647). Gain is noiseless there (spec § Assumptions), so the real gains and counts are
this check's to find. Set sigma back to 8 before town, where sigma 20 cost the drawn
city frame most of its stars.

**Tuning the floor**, once US2 has landed: replay each frame with `solve_image`, with no
floor and at 0.5, 1, 1.5 and 2, as the README's "Finding the value" says. Note each
star count and solve. Take the least floor at which the 200 ms frames at gain 16 hold
tens of stars, not hundreds, and solve (on the probe's drawn frames at gain 16, 0.5:
1215 stars with none, 46 at 0.5, 13 at 1.0, all solving); if they already do with no
floor, leave it unset. Then check that the 200 ms frames at gain 32 and at the top still
solve at that floor; if they do not, take the next floor that solves all three. Write the
floor taken in the node's config as `[detection]` `noise_floor`.

**A town frame**, optional, by the owner, to see whether the floor may stay set in town:
record one raw frame in town with the floor unset and the mount still, with Setup's
exposure and gain fixed at the values auto-exposure settles at there, and replay it with
`solve_image` with no floor and at the floor written in the config. The same star count
and solve at both mean the floor changes nothing on that sky; fewer stars at that floor
mean the town sky's noise sits below it, so leave it out in town, as the README says.

**The field check**, by the owner, at a dark site, once 048 has landed:

1. Build a new Pi kit from the landing commit, and install its node binary. A kit built
   between US1 and US2 loses the floor when the console pairs.
2. Set the floor and `mode = "1296x972"` in the node's config and run `--check-config`:
   the floor prints under `[detection]`. With `mode = "auto"` the rpicam adapter takes
   the first listed mode, 640x480 (`crates/scope-node/src/rpicam/mod.rs:81-83`,
   `fixtures/scope-node/list-cameras.txt:4`), not the 1296x972 the floor was tuned on.
3. Pair the console and set Setup's exposure and gain to automatic: fixed values left in
   Setup from the recording step would replace the config's when it pairs
   (`crates/finder/src/link.rs:476-477`, `crates/scope-node/src/service.rs:407-435`).
4. Within a dozen frames the node's exposure should hold 200 ms and its gain rise while
   it finds fewer than 25 stars, and, as on the drawn sky, the live solve hold. Gain at
   its top (63.9 on the OV5647) with fewer than 25 stars means the floor is too high for
   that sky, or the 200 ms frames hold too few stars: lower the floor, and if the counts then climb back
   to hundreds, set a fixed longer exposure in Setup instead, such as 3000 ms at gain 16.
   Counts near 200 mean the floor is too low. Note the gains and counts: US2-S2's model
   treats gain as noiseless (spec § Assumptions), so this is the first test of 200 ms at
   high gain on a real dark sky.

**What spec 045 must add** for the ASI120MM Mini on INDI:
- an `indi` value of `Adapter`, with its own keys (its server's address, its device, its
  offset);
- the conversion of its 12-bit samples to the 8-bit frames detection reads, with any
  floor in those 8-bit levels;
- the offset, which lifts the background off 0, so that the noise estimate measures
  real noise. Its own floor may then be unneeded: 045's probe should measure it;
- **where each camera adapter's own detection settings live**, if anywhere (owner
  decision 4, left to 045). The shapes 048's draft weighed: (a) a table named after the
  camera adapter, `[rpicam]`, `[simulated]` and 045's `[indi]`, holding a `detection`
  table set over `[detection]` key by key while `adapter` names it, another adapter's
  table checked and not used; (b) `[detection.<adapter>]` subtables, keeping detection in
  one place but the adapter's other keys elsewhere; (c) named camera profiles,
  `[[camera]]` entries with a name and an adapter and `camera = "<name>"`, so that two
  cameras of one adapter could differ, at the cost of moving today's top-level `adapter`
  and `mode` into a profile; (d) as (a), but the adapter's table replacing `[detection]`
  whole. Whatever the shape, the console's pairing replaces four of the five keys such a
  table could hold: `Configure` takes sigma, binning, row normalisation and hot-pixel
  rejection from the link, which always sends binning 2, rows normalised, hot pixels
  rejected and Setup's sigma (`crates/finder/src/link.rs:441-448`), so only the floor
  outlasts pairing, and a table could hold just that. Evidence: trial C, commit
  78f7e266 on `probe/048` (cut from b7e983a0), built (a) with header-line config errors
  naming the table; its six scenario tests passed there, at about 13,900 B of diff.

**The root cause, not in 048**: the Pi camera's background sat near 0 on the dark night,
probably because its processing clips it there (the drawn sky clips it by construction).
Lifting it off 0 at the source, through the black level (in libcamera's own term) of the
camera's tuning file or by capturing raw frames, is worth examining once real dark-night
frames exist.

## Plan traps

They bind every story.

1. **Gate evidence or nothing.** The judge credits only gate results and the story's
   diff. *Check:* every Then names a committed test that the `test` gate runs.
2. **One evidence budget.** *Check:* each story's diff stays under 39,321 B, 60 % of
   64 KiB, measured with `git diff <base> -- . ':(exclude)crates/finder/web/dist' | wc -c`.
   Measured on the probe's files in the fold of review round 2, the README again in the
   folds of rounds 3 and 4 (174a48dc), each story's part measured as its own diff:
   - US1, about 23,950 B, 36.5 % of 65,536 B. That is `star-detection` 2,866, `detect.rs`
     2,402, `config.rs` 838, its part of the README about 2,090, `star-detection`'s test
     3,533, `dark/mod.rs` 6,528, `dark_sky.rs` 2,212 and the node's `noise_floor.rs`
     3,470.
   - US2, about 16,550 B, 25.3 %. That is `service.rs` 2,135, `solve_image` 4,213, its
     README part about 2,830 and `dark_sky_exposure.rs` 7,368.
   - At the 1.35 times a factory build has run over its probe on earlier specs, US1 is
     about 32,300 B, 49.3 %, and US2 about 22,300 B, 34.1 %: both under 39,321 B.
3. **A spec fixed mid-epic needs a relaunch.** *Check:* no builder edits
   `specs/048-node-detects-stars-on-a-dark-sky/` or `.specify/`.
4. **An oracle copied from the code proves nothing.** *Check:* every expected output,
   message and print is a literal in the test. True stars are counted against the drawn
   stars' positions, and solves against the simulator's truth. US1-S1's expected values
   come from `detect_stars`, which the same diff rewrites to call the new function:
   `parity.rs` holds it to cedar's goldens at binning 1, and at binning 2 with row
   normalisation only `image_parity`'s solve does (spec § User Scenarios, Oracles). No
   test formats a message for an expected value.
5. **A negative assertion passes when nothing happens.** *Check:* US1-S2 finds the faint
   star at 0.7 before asserting none at 1.0. US1-S3 asserts 200 detections and its
   zeros before its bounds. US1-S5 asserts the solve before the equality. US2-S1 asserts
   the first frame's 200 centroids, its exposure, gain and solve before asserting that no
   frame from the third solves. Two of US2-S2's clauses cannot fail and prove nothing
   about FR-007: the pairing settings are the node's initial settings' wire form
   (`to_wire`, `crates/scope-node/src/service.rs:395`), so `Configure` answering them
   and every frame reporting them hold for a `Configure` that does nothing. FR-007 rests
   on the 200 ms, solve and count clauses: today's `Configure` resets the floor and fails
   the first of them at the fourth frame, at 53.002 ms (§ What was run). US2-S2 does not
   tell a floor of 0.5 from 1.0, and need not: at the camera's listed gain range both
   pass every clause on the sweep's figures (§ What was run, round 2).
6. **The skies must be drawn exactly.** *Check:* `dark/mod.rs` follows § The skies to
   the draw order. The bounds of US1-S3, US1-S4 and US2 were measured on that model and
   no other; a builder that finds them failing on a model it changed has changed the
   model. US1-S4's "at least 4" equals the minimum of the seed sweep (4 or 5 over seeds
   13 to 18); seed 13 gives 5 every time, but a model drawn in another floating-point
   order could give 3. US2-S2's "at least 5 of 7" sits one under the minimum over six
   seed bases, 6, and its 20 to 50 around their 23 to 43. Its "each frame from the
   fourth solves" has no margin: the first solve falls exactly on the fourth frame at
   bases 13 and 313, and its 30″ bound sits 4.6″ above the worst error, 25.4″. Over
   other draws, which a builder's changed model amounts to: across the review's 45 seed
   bases, US2-S2 as written held at 43 (19 detections at two) and US2-S1 at 44 (a first
   frame of 59 at one), at the listed ranges (fold of round 2); across 62 seeds, US1-S3
   held at 57 (151 to 193 centroids at four, 11 true among the 30 brightest at two) and
   US1-S4 at 54 (8 seeds give 3), from review round 2. Those are the clauses that fail
   first.
7. **The shared skies live in a new module.** *Check:* `crates/simulator/tests/dark/mod.rs`
   starts `#![allow(dead_code)]`, as `crates/simulator/tests/support/mod.rs` does, holds
   the public items of § The skies, and each binary that uses it declares `mod dark;`.
   `support/mod.rs` is not touched. US2 does not edit `dark/mod.rs`: its sky camera and
   node rig live in `dark_sky_exposure.rs`.
8. **The scratch configs must not collide.** *Check:* each config file goes under
   `CARGO_TARGET_TMPDIR`, named for its test file, the process id and a counter, and is
   removed after the run.
9. **Paused time.** *Check:* US2's tests run under `start_paused = true`. The sky camera
   sleeps each exposure with `tokio::time::sleep`, and each frame is awaited under a
   10 s `tokio::time::timeout`, which bounds a stall and is not a duration the test
   asserts.
10. **An unused helper fails clippy.** *Check:* each test file holds only the helpers its
    own tests use; `dark/mod.rs` is the exception, under its `allow`.
11. **clippy counts arguments.** *Check:* `detect_stars_with_noise_floor` carries
    `#[allow(clippy::too_many_arguments)]`.
12. **rustdoc denies warnings.** *Check:* no doc comment holds an angle bracket outside
    backticks or a code block, in `solve_image`'s doc comment among them.
13. **Each worktree builds on its own.** *Check:* no story sets `CARGO_TARGET_DIR`.
14. **The lockfile does not move.** *Check:* every cargo command runs with `--locked`.

## Risks

| Risk | Mitigation |
|---|---|
| A real dark-sky frame's noise differs from the drawn sky's, and 0.5 is wrong for it. | The floor is off by default; the owner tunes it from recorded frames with `solve_image` (§ Outside the factory). |
| A floor set too high loses faint stars, and at the 200 ms cap auto-exposure cannot lengthen to make up for it. | The README says to take the least floor that works; the field check names the sign (gain at its top, 63.9 on the OV5647, under 25 stars) and the fallback, a fixed longer exposure in Setup. |
| The drawn sky's gain is noiseless, so a real 200 ms frame at high gain may hold fewer stars than US2-S2's. | Spec § Assumptions says so; the README promises no star count and gives the fixed longer exposure as the fallback; the field check measures it. |
| A floor left set in town costs faint stars, if a town sky's real noise sits below it. | The README leaves the floor out in town or has the observer compare star counts with and without it; § Outside the factory's optional town frame measures it. |
| Setup's sigma, raised for the no-code check, is left high in town. | § Outside the factory says to set it back to 8. |
| A recording made with a floor set no longer reproduces its centroids. | Owner approval 3; the recording step records with the floor unset or notes it. |
| A future `DetectSettings` key would now survive `Configure` where it was reset. | Intended (owner approval 2): the node's config owns every key the link does not carry. |
| A kit built between US1 and US2 drops the floor on pairing. | § Outside the factory: build the field kit from the spec's landing. |
| A later spec changes the link's `Detection`. | Owner decision 7 (b) is that spec; `from_wire` then takes the floor from the link. |

## Refinement Record

Drafted on 2026-11-22 (UTC) by a plate-solver spec session against `main` at b7e983a0,
folded the same day after review round 1 and the owner's answers to decisions 4 and 5,
re-anchored at 28f436df, folded again after review round 2 and the owner's answers to
decisions 1, 2, 3, 6 and 7, and a third and a fourth time, in wording only, after review
rounds 3 and 4. It rests on a probe of the fault and a trial fix, rerun in the first
fold at the node's own frame interval and in the second at the finder camera's listed
ranges; a reading of the
node's detection, config, service and auto-exposure, of `star-detection` and
cedar-detect, of 013's and 025's trios and of the constitution. Writes come from the task
text. Shared writes name the merge edge that orders them, and the story that last wrote
the same file on `main`.
`crates/star-detection/src/detect.rs`, `lib.rs`, `noise.rs` and
`crates/plate-solver/examples/solve_image.rs` were last changed by #98, #101, #62 and
#101, not a spec's stories, and have no entry.

Every owner decision is answered (spec.md § Owner decisions), and the scenarios, the
writes and this record follow the answers, so the record's `open_questions` are empty.
Owner decision 3's answer puts Setup's detection threshold (sigma), the no-code option,
in the README section US1 writes (FR-004), and US1's section also leaves the floor out
in town, whose noise on the finder camera is not measured (review round 3). This record
approves nothing: the owner approvals of spec.md are given by the flip.

```yaml
version: 1
reviewed_revision: 28f436dfbdd6b9e7aead37928f52a980027560fc
stories:
  US1:
    tests:
      - path: crates/star-detection/tests/noise_floor.rs
        status: new
        disposition: add
        assertion: "each of the 13 images of cedar_goldens_bin1_sigma8.json read as luma from reference-solutions/cedar-detect/test_data, at sigma 8 with hot pixels rejected, at binning 1 without row normalisation and at binning 2 with it: detect_stars_with_noise_floor at floors 0.0, 0.1 and 0.2 equals detect_stars star for star and in order, and no detect_stars result is empty"
        decision: "spec.md:US1-S1"
      - path: crates/star-detection/tests/noise_floor.rs
        status: new
        disposition: add
        assertion: "32x32 frames of 0 at sigma 8, binning 1, no row normalisation, no hot-pixel rejection: levels 3 6 3 at x 15 16 17 of row 10 give one star with detect_stars and with floors 0.2 and 0.7 and none with 1.0; levels 9 18 9 give one star within 0.1 px of (16.5, 10.5) with floor 1.0 and none with 2.5"
        decision: "spec.md:US1-S2"
      - path: crates/simulator/tests/dark_sky.rs
        status: new
        disposition: add
        assertion: "the dark-sky frame, the dark sky at 1 s and gain 1 from seed 13, with scope_node::detect::detect and DetectSettings::cedar(): at least 80 percent of its pixels are 0, it gives 200 centroids, at most 15 are true stars within 1.5 px of a drawn star, and at most 10 of its 30 brightest are"
        decision: "spec.md:US1-S3"
      - path: crates/simulator/tests/dark_sky.rs
        status: new
        disposition: add
        assertion: "the dark-sky frame with cedar's settings and noise_floor 1.0: at least 4 centroids, every one a true star"
        decision: "spec.md:US1-S4"
      - path: crates/simulator/tests/dark_sky.rs
        status: new
        disposition: add
        assertion: "the city frame with cedar's settings: the console's solve of its 30 brightest is within 30 arcsec of the simulator's true camera direction; with noise_floor 0.5 and with 1.0 the centroids equal the first centroid for centroid"
        decision: "spec.md:US1-S5"
      - path: crates/scope-node/tests/noise_floor.rs
        status: new
        disposition: add
        assertion: "the built scope-node --check-config on the head with adapter simulated and no [detection], and with [detection] noise_floor = 1.0: both exit 0; the first's stdout holds today's [detection] print, the second's holds [detection] sigma = 8.0 noise_floor = 1.0 binning = 2 in that order; Config::parse gives detection.noise_floor None and Some(1.0)"
        decision: "spec.md:US1-S6"
      - path: crates/scope-node/tests/noise_floor.rs
        status: new
        disposition: add
        assertion: "the head with adapter simulated, [detection] on line 4 and noise_floor on line 5: noise_floor 0.1, 5.5 and nan each exit 1 with stderr exactly scope-node: detection.noise_floor must be between 0.2 and 5; 'high' exits 1 with exactly scope-node: invalid noise_floor at line 5; 0.2 and 5 exit 0"
        decision: "spec.md:US1-S7"
      - path: crates/star-detection/tests/parity.rs
        status: existing
        disposition: preserve
        assertion: "detect_stars at binning 1 and sigma 8 matches cedar-detect's goldens on every reference image"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/scope-node/tests/detect.rs
        status: existing
        disposition: preserve
        assertion: "cedar's settings equal the console's field by field, the node's and the console's detection agree on the saved finder frame, and a crowded frame keeps the detector's brightest 200 in order"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/scope-node/tests/raw_frame.rs
        status: existing
        disposition: preserve
        assertion: "the raw pixels are exactly what detected the streamed stars with cedar's settings"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/scope-node/tests/cli.rs
        status: existing
        disposition: preserve
        assertion: "check-config prints every setting with the token masked, and bad keys are refused naming the key"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/simulator/tests/sky.rs
        status: existing
        disposition: preserve
        assertion: "the simulator's rendered sky detects with cedar's settings and solves to the true camera direction"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "detect_stars keeps its signature and every result (crates/star-detection/tests/parity.rs:107)"
      - "DetectSettings::cedar() has no floor, so a config without one detects exactly as today"
      - "--check-config prints [detection] exactly as today when no floor is set"
      - "the node keeps the brightest 200 centroids"
    checks:
      development:
        - "cargo nextest run -p star-detection --locked --test noise_floor"
        - "cargo nextest run -p simulator --locked --test dark_sky"
        - "cargo nextest run -p scope-node --locked --test noise_floor"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - crates/scope-node/README.md
      - crates/scope-node/src/config.rs
      - crates/scope-node/src/detect.rs
      - crates/scope-node/tests/noise_floor.rs
      - crates/simulator/tests/dark/mod.rs
      - crates/simulator/tests/dark_sky.rs
      - crates/star-detection/src/detect.rs
      - crates/star-detection/src/lib.rs
      - crates/star-detection/src/noise.rs
      - crates/star-detection/tests/noise_floor.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/scope-node/README.md
        other_spec: 048-node-detects-stars-on-a-dark-sky
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and adds paragraphs at the end of the section US1 writes, which holds the floor and, beside it, Setup's sigma as the no-code option (FR-004, owner decision 3)"
      - path: crates/scope-node/src/detect.rs
        other_spec: 013-live-finder
        other_story: US2
        disposition: merge_order
        reason: "048 depends_on_landed 013, whose US2 last changed detect.rs (ee280ae7)"
      - path: crates/scope-node/src/config.rs
        other_spec: 030-imaging-camera-link
        other_story: US7
        disposition: merge_order
        reason: "048 depends_on_landed 030, whose US7 last changed config.rs (4e7eef6f)"
      - path: crates/scope-node/README.md
        other_spec: 044-node-stops-mount-before-exit
        other_story: US1
        disposition: merge_order
        reason: "048 depends_on_landed 044, whose US1 last changed the node's README (6fb836c9)"
    open_questions: []
  US2:
    tests:
      - path: crates/simulator/tests/dark_sky_exposure.rs
        status: new
        disposition: add
        assertion: "the node rig, NodeService::new at the node's 200 ms frame interval over the sky camera at 46.34 frames/s with the OV5647's listed ranges, gain 1 to 63.9375 and exposure 0.13 to 3066.985 ms, whose first automatic frame is 200 ms, with cedar's settings and the camera at gain 16 while gain is automatic, 8 star frames: each holds at least 100 centroids; the first holds 200 at 200 ms and gain 16 and solves within 30 arcsec; neither gain nor exposure ever rises; exposure stays 200 ms while gain is above 1; the eighth is at gain 1 and under 10 ms; no frame from the third solves"
        decision: "spec.md:US2-S1"
      - path: crates/simulator/tests/dark_sky_exposure.rs
        status: new
        disposition: add
        assertion: "the node rig with cedar's settings and noise_floor 0.5, the camera at gain 1 while gain is automatic, Configure with the pairing settings then 12 star frames: Configure answers the pairing settings; every frame reports them and is at 200 ms; each frame from the fourth solves within 30 arcsec; each frame from the sixth holds 20 to 50 centroids at a gain above 8; at least 5 of those 7 frames hold 25 to 40"
        decision: "spec.md:US2-S2"
      - path: crates/simulator/tests/node.rs
        status: existing
        disposition: preserve
        assertion: "the node's frames, previews, auto-exposure, fixed controls and settings refusals over the simulator, with Configure filling the keys the link does not carry from cedar's"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/scope-node/tests/raw_frame.rs
        status: existing
        disposition: preserve
        assertion: "the raw pixels are exactly what detected the streamed stars with cedar's settings"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "the scope link's messages and what a star frame reports are unchanged"
      - "Configure takes sigma, binning, row normalisation and hot-pixel rejection from the link as today"
      - "crates/scope-node/src/exposure.rs and the auto-exposure cap are unchanged (013 FR-015)"
      - "solve_image without --noise-floor detects as today"
    checks:
      development:
        - "cargo nextest run -p simulator --locked --test dark_sky_exposure"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs, the recorded frames, the floor's tuning and the field check of plan.md § Outside the factory"
    writes:
      - crates/plate-solver/examples/solve_image.rs
      - crates/scope-node/README.md
      - crates/scope-node/src/service.rs
      - crates/simulator/tests/dark_sky_exposure.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/scope-node/src/service.rs
        other_spec: 030-imaging-camera-link
        other_story: US7
        disposition: merge_order
        reason: "048 depends_on_landed 030, whose US7 last changed service.rs (4e7eef6f)"
    open_questions: []
```
