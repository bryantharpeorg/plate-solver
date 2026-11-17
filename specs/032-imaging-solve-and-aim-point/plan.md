# Implementation Plan: Solving Imaging Frames and the Measured Aim Point

**Branch**: `032-imaging-solve-and-aim-point` | **Date**: 2026-11-15 | **Spec**: [spec.md](spec.md)

This plan records every design decision for the feature. A story that would change one
escalates rather than re-deciding it.

## Summary

Add the solver port to the console: a trait that answers an imaging solve or a star
measure for a saved imaging frame. Two solver adapters stand behind it. The ASTAP
adapter writes a prepared copy of the frame into a work folder, runs ASTAP's
command-line program on it, reads the `.ini` it leaves and maps its exit codes. The
simulated adapter answers from the simulator's truth. `POST /api/imaging/solve` solves
the latest saved imaging frame, shows the solve in the imaging view and writes it beside
the frame as a solve sidecar. The hint comes from a live pointing, the last imaging
solve or the target; the first search is 3° wide; the scale a solve measures is kept in
`relation.json` and sizes the next one, with the focal length and ASTAP's own search
for the height as the sources it falls to in turn; after three misses in a row the
hint's place moves one source down in the same way. The console keeps a history of the
finder camera's live solves, bounded by time and thinned. After an imaging solve, made
by the route or offered by a later spec through `Imaging::note_solve`, it fits a line
through those around the exposure and takes an aim reading at the exposure's middle:
where the imaging camera's centre falls in the finder camera's frame, and the imaging
angle. It sets the session's aim point from the median of the last five aim readings,
sets outliers aside, and keeps the readings across restarts. While Track is on, the
session holds a newly measured aim point back and applies it at the first live solve
after Track is off, so 015's tracker never chases it. The Imaging page gets a solve
block, the live chart an outline of the imaging field, Setup a line saying how the aim
point was set, and MCP clients one tool.

## Technical Context

**Language/Version**: Rust (MSRV 1.96) for US1 to US5 and US7; TypeScript with React 19
for US6.

**Primary Dependencies**: nothing new, and no `Cargo.toml` changes. `finder` already has
`simulator`, `math-core`, `serde`, `serde_json` and `tokio` with `fs`, `sync` and `time`
(`crates/finder/Cargo.toml:33`). The program is run with `std::process`, because
`tokio`'s `process` feature is not enabled and may not be added. ASTAP is an external
program, installed by the owner, never built or linked here.

**Storage**: beside each saved imaging frame, `frames/<UTC date>/<id>.solve.json`; in
the imaging folder, `relation.json` and, while an ASTAP call runs,
`solver/solve-<pid>-<n>/`; in `session.json`, the key `aim_measured`.

**Testing**:
- US1: `crates/finder/tests/console/imaging_astap.rs` (new), in real time, against the
  stub program of spec.md § User Scenarios and, for US1-S7, the saved captures.
- US2: `crates/finder/tests/console/imaging_solve.rs` (new) and, for US2-S6,
  `crates/finder/tests/cli/imaging_solve.rs` (new).
- US3: `crates/finder/tests/console/imaging_hints.rs` (new) and, for US3-S6,
  `crates/finder/tests/cli/imaging_hints.rs` (new).
- US4: `crates/finder/tests/console/imaging_relation.rs` (new) and, for US4-S8,
  `crates/finder/tests/motion/imaging_track.rs` (new).
- US5: `crates/finder/tests/console/imaging_smoothing.rs` (new).
- US6: `crates/finder/web/src/components/ImagingSolveBlock.test.tsx`,
  `crates/finder/web/src/components/ImagingField.test.tsx` and
  `crates/finder/web/src/components/AimSource.test.tsx` (new), Vitest.
- US7: `crates/finder/tests/mcp/imaging_solve.rs` (new).
- Where a finder test goes is 027's rule: `console` calls the router in process and
  moves no mount (`crates/finder/tests/console/main.rs:2`), `cli` starts the built
  `finder` (`crates/finder/tests/cli/main.rs:1`), `mcp` drives the MCP door in process
  (`crates/finder/tests/mcp/main.rs:1`), and `motion` drives the simulated mount in
  paused time (`crates/finder/tests/motion/main.rs:1`). One test of this spec moves a
  mount, US4-S8, which turns Track on, and so it is a module of `motion`. Each
  module is declared by one `mod` line, in alphabetical order, in its binary's
  `main.rs` (`crates/finder/tests/layout/main.rs:156`), binds the live helpers with
  `use crate::support::live as support;`
  (`crates/finder/tests/console/live/loop_.rs:3`), and writes the rest of its setup
  itself: `crates/finder/tests/support` takes no new file
  (`crates/finder/tests/layout/main.rs:127`). So the solving rig of spec.md is written
  out in each of `imaging_solve`, `imaging_hints`, `imaging_relation`,
  `imaging_smoothing`, the `motion` module `imaging_track` and the `mcp` module
  `imaging_solve`: about 4 to 5 KiB of the same setup in five stories, US4 paying it
  twice, counted in each story's budget (§ Risks). No module
  imports another test module's helpers: a story may not edit an earlier story's test
  file, so a helper shared that way could never change.
- No Playwright test (spec owner decision 16).

**Target Platform**: the `finder` console on the console machine. Nothing changes on the
scope node.

**Constraints**:
- No change under `crates/simulator`, `crates/scope-node`, `crates/scope-link` or
  `proto/`; no `unsafe`; no dependency.
- No file under `crates/finder/src/imaging/` reads an axis position or computes an
  altitude, an azimuth or a field rotation (the plan document, § Mount-neutral by
  design).
- No constant is sized to one telescope: thresholds are arcseconds on the sky, degrees
  of angle, or fractions of a frame.
- `web/dist` is committed and must be the build of its source (US6).

**Scale/Scope**: five new modules, one route, one MCP tool, four flags, one page block,
one chart mark.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Nothing touches the finder camera's solver, detection or `crates/math-core`. Console work is grounded in the cited sources: ASTAP's own page for its flags, files and exit codes (`docs/research/2026-11-15-imaging-camera.md` § 3. ASTAP), and the simulator's truth for the rest. |
| II. Never weaken a gate | No existing test, golden, fixture, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS`, `ergane.yaml` or `playwright.config.ts` is edited. New tests are new module files; each adds one `mod` line to its binary's `main.rs`, which holds no test (spec owner approval 13). |
| III. Tests first, independent expectations | Oracles: the simulator's truth read by each test itself; two worked answers and the worked examples whose arithmetic is in spec.md; ASTAP's documents; saved captures whose star places come from a catalogue; and, for the scenarios that check only words, statuses and keys, the scenario's own stated text. Every scenario names its oracle and its tolerances. Time comes from outside everywhere but one place: the ASTAP adapter reads `std::time::Instant` twice, for the deadline of the child process it waits on and for `took_ms`, because a real program cannot be waited for on paused tokio time (§ Plan traps 7). The limits themselves are injected with `with_limits`, so US1-S5 proves the deadline in under two seconds; every other wait in a test is on virtual time. |
| IV. Critical paths | No `unsafe`, no `Cargo.toml` or `Cargo.lock` change, nothing under `proto/` or `crates/math-core`. |
| V. Proven open source first, honest licensing | The finder camera's solver stays our own Rust. The imaging solver is ASTAP, by the owner's decision (ADR-0002), reached the way principle V of constitution 1.3.0 allows: a separate program behind the solver port, never linked, which the gates reach through the simulated adapter and saved captures (§ Prerequisites 4). ASTAP is MPL-2.0; none of its source is copied; the README names it, its licence and the database's credit line (FR-013). |

## Project Structure

```text
crates/finder/
├── src/imaging/solver.rs               # US1: new; the port and the imaging solve's arithmetic
├── src/imaging/astap.rs                # US1: new; the ASTAP adapter
├── src/imaging/simulated.rs            # US2: new; the simulated adapter
├── src/imaging/solving.rs              # US2: new; solve_frame, SolveView, the sidecar, solve and solved; US3: hints and their turn; US4: the history's task, the aim reading's use, note_solve, aim_in_force, the hint's pointing; US5: the narrow radius
├── src/imaging/relation.rs             # US3: new; Relation, relation.json, the scale, hint_skip; US4: history and aim reading, view(in_force, held); US5: window
├── src/imaging/mod.rs                  # US1 to US3: mod lines; US2, US3: the view's two parts, the solver and the Relation in the state; US4: the history task's handle, the aim given to the view
├── src/imaging/routes.rs               # US2: POST /api/imaging/solve
├── src/api.rs                          # US2: with_imaging_solver; US3: with_focal_length_mm
├── src/main.rs                         # US2: the ASTAP flags and the simulated solver; US3: --focal-length-mm
├── src/lib.rs                          # US2: the route's doc line
├── src/session.rs                      # US4: AimView, set_measured_aim, aim_measured, the held aim point
├── src/sky.rs                          # US4: Frame::up_east_of_north_at
├── src/mcp/tools.rs                    # US7: solve_imaging_frame
├── README.md                           # US2: section, row, flags; US3: hints; US5: the aim point; US7: the tool
├── tests/console/main.rs               # US1 to US5: one mod line each
├── tests/console/imaging_astap.rs      # US1: new
├── tests/console/imaging_solve.rs      # US2: new
├── tests/console/imaging_hints.rs      # US3: new
├── tests/console/imaging_relation.rs   # US4: new
├── tests/console/imaging_smoothing.rs  # US5: new
├── tests/cli/main.rs                   # US2, US3: one mod line each
├── tests/cli/imaging_solve.rs          # US2: new, US2-S6
├── tests/cli/imaging_hints.rs          # US3: new, US3-S6
├── tests/motion/main.rs                # US4: mod imaging_track
├── tests/motion/imaging_track.rs       # US4: new, US4-S8
├── tests/mcp/main.rs                   # US7: mod imaging_solve
├── tests/mcp/imaging_solve.rs          # US7: new
└── web/
    ├── src/api.ts                      # US6: the solve and relation types, aim.source, postImagingSolve
    ├── src/useSession.ts               # US6: copy `imaging`
    ├── src/components/ImagingSolveBlock.tsx       # US6: new
    ├── src/components/ImagingSolveBlock.test.tsx  # US6: new
    ├── src/components/ImagingField.tsx            # US6: new; imagingFieldCorners and the outline
    ├── src/components/ImagingField.test.tsx       # US6: new
    ├── src/components/AimSource.test.tsx          # US6: new
    ├── src/components/ImagingPage.tsx  # US6: places the block
    ├── src/components/Chart.tsx        # US6: draws the outline
    ├── src/components/AimTap.tsx       # US6: the source line
    └── dist/                           # US6: rebuilt
DESIGN.md                               # US6: three changes
.impeccable/design.json                 # US6: the Imaging field component
fixtures/finder/astap/                  # the owner, before the flip; no story writes it
```

## Interfaces

Coordinates, angles and the worked answers are as spec.md § User Scenarios defines them.
Everything 030 and 031 create is named as their plan.md gives it and is re-read on `main`
before the flip (§ Prerequisites 2).

### The port (`crates/finder/src/imaging/solver.rs`, new, US1)

```rust
//! The solver port (032): how the console asks an imaging solver where a saved imaging
//! frame points, or how large its stars are.

/// A rectangle in pixels: its top-left corner, then its size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Region { pub x: u32, pub y: u32, pub width: u32, pub height: u32 }
/// 031's view of a region of interest has the same four numbers.
impl From<super::RoiView> for Region { /* field for field */ }

/// A saved imaging frame as a solver adapter needs it.
#[derive(Debug, Clone, PartialEq)]
pub struct SolveFrame {
    /// The saved FITS file. The simulated adapter does not open it.
    pub path: PathBuf,
    /// Delivered samples across and down.
    pub width: u32,
    pub height: u32,
    pub bits: u32,
    /// `"RGGB"`, `"BGGR"`, `"GRBG"`, `"GBRG"`, or `None` for mono.
    pub bayer: Option<String>,
    pub binning: u32,
    /// The region of the sensor the frame covers, in sensor pixels.
    pub roi: Region,
    /// The exposure's start in console Unix seconds, and its length in seconds.
    pub start: f64,
    pub exposure_s: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HintSource { Pointing, LastSolve, Target, None }

/// Where the solver is told to look. Serialized as `{"source", "ra_deg"?, "dec_deg"?,
/// "radius_deg", "fov_deg"?}`: the place and the field height are left out when absent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SolveHint {
    pub source: HintSource,
    /// J2000 degrees; `None` for a search of the whole sky.
    pub position: Option<(f64, f64)>,
    pub radius_deg: f64,
    /// The imaging frame's height on the sky, when known.
    pub fov_deg: Option<f64>,
}
impl SolveHint {
    /// No place, 180°, no field height.
    pub fn whole_sky() -> Self;
}

/// What a solver said about a picture, in the picture's own FITS pixels: column and
/// row counted from 1, rows in the file's order. Degrees.
#[derive(Debug, Clone, PartialEq)]
pub struct Answer {
    pub crpix: [f64; 2],
    pub crval: [f64; 2],
    /// `[[CD1_1, CD1_2], [CD2_1, CD2_2]]`.
    pub cd: [[f64; 2]; 2],
    pub warning: Option<String>,
}

/// How a prepared copy sits in its imaging frame: copy pixel (i, j) covers the frame's
/// pixels from (x0 + k·i, y0 + k·j), k on a side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Prepared { pub x0: u32, pub y0: u32, pub k: u32, pub width: u32, pub height: u32 }

#[derive(Debug, Clone, PartialEq)]
pub struct ImagingSolve {
    /// J2000 degrees of the imaging frame's centre; RA in 0 to 360.
    pub ra_deg: f64,
    pub dec_deg: f64,
    /// The angle of frame-up east of celestial north, 0 to 360.
    pub up_east_of_north_deg: f64,
    /// Per delivered sample.
    pub arcsec_per_px: f64,
    pub mirrored: bool,
    /// Degrees east (row 0) and north (row 1) for one pixel right (column 0) and one
    /// pixel down (column 1).
    pub cd_deg: [[f64; 2]; 2],
    pub width: u32,
    pub height: u32,
    /// Matched stars, when the solver says.
    pub stars: Option<u32>,
    pub took_ms: f64,
    pub warning: Option<String>,
}
impl ImagingSolve {
    /// Derives the angle, the scale and the mirror sense. `stars` and `warning` are
    /// `None` and `took_ms` 0.
    pub fn from_cd(ra_deg: f64, dec_deg: f64, cd_deg: [[f64; 2]; 2], width: u32, height: u32) -> Self;
    /// From a solver's answer about a prepared copy of a frame `width` × `height`.
    pub fn from_answer(answer: &Answer, copy: Prepared, width: u32, height: u32) -> Self;
    /// J2000 degrees of an imaging-frame position.
    pub fn sky_at(&self, pixel: (f64, f64)) -> (f64, f64);
    /// The imaging-frame position of a place; `None` a quarter turn or more away.
    pub fn pixel_of(&self, ra_deg: f64, dec_deg: f64) -> Option<(f64, f64)>;
}

/// The median half-flux diameter in imaging-frame pixels, and the star count.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StarMeasure { pub half_flux_px: f64, pub stars: u32 }

#[derive(Debug, Clone, PartialEq)]
pub enum SolveError {
    NotSolved { detail: Option<String> },
    TooFewStars { detail: Option<String> },
    NoDatabase { detail: Option<String> },
    TimedOut { seconds: u64 },
    Failed(String),
}
impl SolveError {
    pub fn reason(&self) -> &'static str;
    pub fn message(&self) -> String;
    pub fn detail(&self) -> Option<&str>;
}

pub type SolveFuture<'a> = Pin<Box<dyn Future<Output = Result<ImagingSolve, SolveError>> + Send + 'a>>;
pub type MeasureFuture<'a> = Pin<Box<dyn Future<Output = Result<StarMeasure, SolveError>> + Send + 'a>>;

/// The solver port. A solver adapter speaks one imaging solver's own interface.
pub trait ImagingSolver: Send + Sync {
    /// `astap` or `simulated`.
    fn name(&self) -> &'static str;
    fn solve(&self, frame: SolveFrame, hint: SolveHint) -> SolveFuture<'_>;
    /// `region` is in the frame's pixels; `None` is the whole frame.
    fn measure(&self, frame: SolveFrame, region: Option<Region>) -> MeasureFuture<'_>;
}
```

The futures follow the shape `live::Solver` already uses
(`crates/finder/src/live.rs:24`).

| `SolveError` | `reason()` | `message()` |
|---|---|---|
| `NotSolved` | `not_solved` | `the imaging solver found no solution` |
| `TooFewStars` | `too_few_stars` | `the imaging solver found too few stars in the imaging frame` |
| `NoDatabase` | `no_database` | `the imaging solver has no star database it can read` |
| `TimedOut { seconds }` | `timed_out` | `the imaging solver gave no answer within {seconds} s` |
| `Failed(text)` | `failed` | the text |

**`from_cd`.** With `det = cd[0][0]·cd[1][1] − cd[0][1]·cd[1][0]`:
`up_east_of_north_deg = atan2(−cd[0][1], −cd[1][1])` in degrees, `rem_euclid(360.0)`;
`arcsec_per_px = 3600·√|det|`; `mirrored = det < 0`. Up is one pixel toward smaller y, a
step of (0, −1), whose east and north parts are `−cd[0][1]` and `−cd[1][1]`. A frame
whose rows run from the top and that shows the sky as the eye sees it has a positive
determinant: for up at θ east of north and s degrees a pixel, `cd = [[−s·cos θ, −s·sin
θ], [s·sin θ, −s·cos θ]]`.

**The tangent plane.** With (α0, δ0) the centre and (ξ, η) east and north in radians:
- to the sky: `α = α0 + atan2(ξ, cos δ0 − η·sin δ0)`, `δ = atan2(sin δ0 + η·cos δ0,
  hypot(ξ, cos δ0 − η·sin δ0))`;
- from the sky: `c = sin δ0·sin δ + cos δ0·cos δ·cos(α − α0)`; none when `c ≤ 0`; `ξ =
  cos δ·sin(α − α0)/c`, `η = (cos δ0·sin δ − sin δ0·cos δ·cos(α − α0))/c`.

`sky_at((x, y))` takes `(dx, dy) = (x − width/2, y − height/2)`, `(ξ, η) =
cd_deg·(dx, dy)` turned to radians, and goes to the sky, RA reduced into 0 to 360.
`pixel_of` goes from the sky, turns (ξ, η) to degrees, multiplies by the inverse of
`cd_deg`, and adds (width/2, height/2).

**`from_answer`.** `cd_deg` is `answer.cd` with each number divided by `copy.k`. The
frame's centre in the copy's FITS pixels is `X = (width/2 − x0)/k + 0.5` and `Y =
(height/2 − y0)/k + 0.5`. `(ξ, η) = answer.cd·(X − crpix[0], Y − crpix[1])`, in degrees,
goes to the sky about `crval`: that is the centre. The warning is the answer's. For
worked answer A, `X − crpix[0]` and `Y − crpix[1]` are both 0, so the centre is `crval`.

### The ASTAP adapter (`crates/finder/src/imaging/astap.rs`, new, US1)

```rust
//! The ASTAP solver adapter (032): runs ASTAP's command-line program on a prepared copy.

/// For a solve whose hint has a place.
pub const SOLVE_LIMIT: Duration = Duration::from_secs(120);
/// For a solve whose hint has none: a search of the whole sky.
pub const WHOLE_SKY_LIMIT: Duration = Duration::from_secs(600);
pub const MEASURE_LIMIT: Duration = Duration::from_secs(30);
/// The signal-to-noise floor given to `-analyse`: the figure of the example on ASTAP's
/// own page (spec owner decision 24).
pub const ANALYSE_SNR: u32 = 30;
/// A prepared copy is halved while it has more rows than this.
pub const MAX_ROWS: u32 = 2500;

#[derive(Debug, Clone)]
pub struct AstapConfig {
    pub program: PathBuf,
    pub database_name: String,
    pub database_dir: Option<PathBuf>,
    /// Each call makes, and removes, one folder in here.
    pub work_dir: PathBuf,
}

pub struct AstapSolver { /* the config, the three limits, a call counter */ }
impl AstapSolver {
    /// With `SOLVE_LIMIT`, `WHOLE_SKY_LIMIT` and `MEASURE_LIMIT`.
    pub fn new(config: AstapConfig) -> Self;
    pub fn with_limits(self, solve: Duration, whole_sky: Duration, measure: Duration) -> Self;
}
impl ImagingSolver for AstapSolver { /* name() is "astap" */ }

pub fn solve_args(input: &Path, base: &Path, hint: &SolveHint, database_name: &str,
    database_dir: Option<&Path>) -> Vec<String>;
pub fn analyse_args(input: &Path) -> Vec<String>;
/// The solve an `.ini` holds, or why it holds none.
pub fn read_answer(ini: &str) -> Result<Answer, String>;
/// The text of an `.ini`'s `ERROR` line, when it has one that is not empty.
pub fn read_error(ini: &str) -> Option<String>;
/// The two lines `-analyse` prints, in the prepared copy's pixels.
pub fn read_measure(stdout: &str) -> Result<StarMeasure, SolveError>;
/// The prepared copy of a frame, or of a region of it.
pub fn prepare(image: &FitsImage, bayer: bool, region: Option<Region>)
    -> Result<(FitsImage, Prepared), String>;
```

**The arguments.** `solve_args` answers, in this order: `-f`, the input; `-o`, the base;
`-ra`, the hint's RA divided by 15, `{:.6}`; `-spd`, its Dec + 90, `{:.5}`; `-r`, its
radius or 180 if it is larger, `{:.2}`; `-fov`, its field height `{:.4}`, or the single
character `0` without one; `-z`, `1`; `-D`, the database name; and, with a database
folder, `-d` and the folder. A hint with no position is sent as RA 0, Dec 0, which is
`-ra 0.000000 -spd 90.00000`. `analyse_args` answers `-f`, the input, `-analyse`, `30`.
Paths are written with `Path::display`.

**The prepared copy.** `prepare` reads the image's little-endian samples, 8 or 16 bits,
as numbers, and then:
1. takes the region, or the whole frame. A region with no width or height, or reaching
   past the frame's width or height, is the error `the region is outside the imaging
   frame`;
2. with `bayer`, moves the region inward to whole cells: its left and top edges up to
   the next even number, its right and bottom edges down to one; a region left with no
   whole cell is the error `the region is too small`. Each 2 × 2 cell becomes
   `(a + b + c + d + 2) / 4` in whole numbers, and k is 2. Without `bayer` the samples
   are copied and k is 1;
3. while the copy has more than `MAX_ROWS` rows, halves it the same way, 2 × 2, dropping
   an odd last row or column, and doubles k
   (`docs/research/2026-11-15-imaging-camera.md` § 3. ASTAP: ASTAP itself bins a
   picture more than 2500 rows high);
4. answers a `FitsImage` of 16 bits with no cards, its first row the frame's top row,
   and the `Prepared`.

It uses 031's `FitsImage` (`crates/finder/src/imaging/fits.rs`, 031 US2). The owner's
camera gives 2472 × 1642 at full resolution and 1236 × 821 from a frame binned 2 × 2.

**One call**, on `tokio::task::spawn_blocking`, in real time:
1. Make the folder `<work_dir>/solve-<process id>-<n>`, n counting this adapter's calls
   from 1. A failure is `Failed("the imaging solver's work folder could not be
   written: {error}")`.
2. Read the frame's file and `fits::decode` it (031 US2). A failure is `Failed("the
   imaging frame could not be read: {error}")`. `prepare` it; its error is `Failed`
   with that text. Write the copy with `fits::encode` as `frame.fits` in the folder.
3. Start the program with the arguments, the base being `<folder>/frame`, its standard
   input closed and its standard output and error sent to `stdout.txt` and `stderr.txt`
   in the folder, so no pipe can fill. A failure to start is `Failed("the imaging
   solver could not be started: {error}")`.
4. Call `try_wait` every 20 ms with `std::thread::sleep` until the program ends or a
   `std::time::Instant` deadline passes. The limit is the whole-sky one when
   `hint.position` is `None`, else the solve one. Past it, `kill` and `wait`, and
   answer `TimedOut` with the limit in whole seconds, rounded up.
5. Read the answer as the table below says. `took_ms` is the real time from step 3.
6. Remove the folder with `remove_dir_all`, whatever happened, ignoring its error.

| Exit code | `.ini` | `solve` answers |
|---|---|---|
| 0 | read by `read_answer` | the solve, by `from_answer` |
| 0 | not written | `Failed("the imaging solver's answer could not be read: no .ini file was written")` |
| 0 | `read_answer` fails | `Failed("the imaging solver's answer could not be read: {why}")` |
| 1 | any or none | `NotSolved`, its detail from `read_error` |
| 2 | any or none | `TooFewStars`, its detail from `read_error` |
| 16 | any or none | `Failed("the imaging solver could not read the imaging frame")` |
| 32, 33 | any or none | `NoDatabase`, its detail from `read_error` |
| any other N | any or none | `Failed("the imaging solver ended with exit code {N}")` |
| none (a signal) | any or none | `Failed("the imaging solver ended without an exit code")` |

`read_answer` splits each line at its first `=`, trims both sides, and ignores keys it
does not know. Its failures, tried in this order: `PLTSOLVD is not T`; then, for
`CRPIX1`, `CRPIX2`, `CRVAL1`, `CRVAL2`, `CD1_1`, `CD1_2`, `CD2_1` and `CD2_2` in that
order, `{KEY} is missing` or `{KEY} is not a number`. Numbers are read with `f64`'s
`FromStr`, which takes ASTAP's `1.1645000000000000E+003`. `WARNING` is kept when it is
not empty. `CDELT`, `CROTA` and `CMDLINE` are not read.

`measure` runs the same steps with `analyse_args` and `MEASURE_LIMIT`. When
`stdout.txt` holds both an `HFD_MEDIAN=` and a `STARS=` line that read as numbers, they
are the answer whatever the exit code: `TooFewStars` when the count is 0 or the diameter
is not above 0, else `half_flux_px` the diameter times k. Otherwise exit 16 is `Failed`
as above, any other code or none `Failed("the imaging solver's measure could not be
read")`.

### Capturing ASTAP's output (the saved captures; the owner, before the flip)

The saved captures are ASTAP's own files for one real picture, recorded once on any
machine with `astap_cli` and the D50 database installed. They have not been made: the
grilling session could not run downloaded programs (§ Prerequisites 5). The picture is
any real 16-bit mono FITS file of at most 2500 rows that ASTAP solves, finer than 3″ a
pixel, in which three catalogued stars that do not lie in one line can be picked out
by hand. The three are stars whose proper motion is under 0.05″ a year, with places
at epoch 2025, the epoch of ASTAP's star databases, which carry no proper motions
(`docs/research/2026-11-15-imaging-camera.md` § 3. ASTAP): then a listed place and a
listed pixel agree to within the half pixel a brightest sample is good to, and
US1-S7's 1.5 pixels has a pixel to spare.

```bash
#!/usr/bin/env bash
# capture-astap.sh FRAME.fits RA_HOURS SPD_DEG FOV_DEG OUT_DIR
set -uo pipefail
F=$1; RA=$2; SPD=$3; FOV=$4; OUT=$5; D=${ASTAP_DB:-d50}
mkdir -p "$OUT"; W=$(mktemp -d); cp "$F" "$W/frame.fits"
run() { n=$1; shift; astap_cli "$@" > "$W/$n.stdout" 2> "$W/$n.stderr"; echo "$n exit $?"; }
far=$(awk -v r="$RA" 'BEGIN { printf "%.6f", (r + 12) % 24 }')
python3 -c 'import sys; h = "".join(c.ljust(80) for c in ["SIMPLE  =                    T",
"BITPIX  =                   16", "NAXIS   =                    2",
"NAXIS1  =                   64", "NAXIS2  =                   48",
"BZERO   =                32768", "BSCALE  =                    1", "END"]).ljust(2880)
d = (1000 - 32768).to_bytes(2, "big", signed=True) * (64 * 48)
open(sys.argv[1], "wb").write(h.encode() + d + bytes(-len(d) % 2880))' "$W/blank.fits"
run solved        -f "$W/frame.fits" -o "$W/solved"        -ra "$RA"  -spd "$SPD" -r 3 -fov "$FOV" -z 1 -D "$D"
run not-solved    -f "$W/frame.fits" -o "$W/not-solved"    -ra "$far" -spd "$SPD" -r 1 -fov "$FOV" -z 1 -D "$D"
run too-few-stars -f "$W/blank.fits" -o "$W/too-few-stars" -ra "$RA"  -spd "$SPD" -r 1 -fov "$FOV" -z 1 -D "$D"
run no-database   -f "$W/frame.fits" -o "$W/no-database"   -ra "$RA"  -spd "$SPD" -r 1 -fov "$FOV" -z 1 -D "$D" -d "$W/nowhere"
run analyse       -f "$W/frame.fits" -analyse 30
cp "$W"/*.ini "$W"/solved.wcs "$W/analyse.stdout" "$OUT"/ 2>/dev/null
```

The owner then writes `captures.json` by hand from what the script printed, and commits
it with the copied files as `fixtures/finder/astap/`, with a `README.md` giving ASTAP's
version, the database, the date and where the picture came from:

```json
{"astap": "the version astap_cli prints", "database": "d50",
 "frame": {"width": 2472, "height": 1642},
 "cases": {
  "solved": {"exit": 0, "ini": "solved.ini", "wcs": "solved.wcs", "stars": [
    {"x": 1203.5, "y": 811.5, "ra_deg": 83.8186, "dec_deg": -5.3897},
    {"x": 310.5, "y": 1422.5, "ra_deg": 84.0534, "dec_deg": -5.6012},
    {"x": 2010.5, "y": 240.5, "ra_deg": 83.6071, "dec_deg": -5.2148}]},
  "not_solved": {"exit": 1, "ini": "not-solved.ini"},
  "too_few_stars": {"exit": 2, "ini": "too-few-stars.ini"},
  "no_database": {"exit": 32, "ini": null},
  "analyse": {"exit": 0, "stdout": "analyse.stdout", "half_flux_px": 3.3, "stars": 666}}}
```

The numbers above only show the shape. A star's `x` and `y` are the column and row of
its brightest sample, counted from the first sample in the file, plus 0.5; its place is
from a star catalogue, not from ASTAP: right ascension and declination in the J2000
frame at epoch 2025. An `ini` is `null` where ASTAP wrote none: the sample shows that
for the missing database, as US1-S4 expects, and the capture decides; if ASTAP did
write one there, its name goes in and US1-S4's bullet for exit 32 is corrected first.
`half_flux_px` and `stars` are the two numbers in `analyse.stdout`. US1-S7 replays each
case through the stub program: the case's `.ini` as `reply.ini`, its `.wcs` as
`reply.wcs`, its `stdout` as `reply.stdout`, its `exit` as `code`, on a mono test frame
of the `frame` size. If ASTAP's exit codes or files differ from what spec.md says (an
`.ini` when the database is missing, the code `-analyse` exits with, a `//` note after
each line of the `.ini` as its page prints them), this plan and US1-S4 are corrected
first.

### The simulated adapter (`crates/finder/src/imaging/simulated.rs`, new, US2)

```rust
//! The simulated solver adapter (032): answers from the simulator's truth.
/// ASTAP's page: "a minimum about 30 stars".
pub const MIN_STARS: usize = 30;
/// ASTAP's page: the height "within preferably 5% accuracy".
pub const FOV_TOLERANCE: f64 = 0.05;

pub struct SimulatedSolver { /* a simulator::Simulator, a delay */ }
impl SimulatedSolver {
    pub fn new(simulator: simulator::Simulator) -> Self;
    /// Wait this long on tokio time before answering.
    pub fn with_delay(self, delay: Duration) -> Self;
}
impl ImagingSolver for SimulatedSolver { /* name() is "simulated" */ }
```

`solve`, after the delay:
1. `truth = simulator.imaging_truth_at(frame.start + frame.exposure_s/2)` and `optics =
   simulator.imaging_optics()` (030 US2). With θ the truth's angle in radians and `s1`
   its scale in degrees a sensor pixel, the sensor's solve is `from_cd(truth.ra_deg,
   truth.dec_deg, [[−s1·cos θ, −s1·sin θ], [s1·sin θ, −s1·cos θ]], optics.width,
   optics.height)`.
2. The frame's centre is the sensor's `sky_at((roi.x + roi.width/2, roi.y +
   roi.height/2))`. The answer is `from_cd` of that centre, the same matrix times
   `frame.binning`, and the frame's width and height.
3. Stars: `simulator::imaging::deep_field(FIELD_SEED, truth.ra_deg, truth.dec_deg,
   radius)`, radius being half the sensor's diagonal in degrees plus 0.05; n is how many
   have a `pixel_of` on the answer with 0 ≤ x < width and 0 ≤ y < height. Fewer than
   `MIN_STARS` is `TooFewStars` with the detail `{n} stars`.
4. A hint with a position farther from the answer's centre than its radius is
   `NotSolved`; so is a hint whose field height differs from `height·arcsec_per_px/3600`
   by more than `FOV_TOLERANCE` of it. Neither has a detail.
5. Otherwise the answer, with `stars` n and `took_ms` the delay.

`measure`, after the delay: the stars of step 3 counted inside the region (the whole
frame without one); none is `TooFewStars`; else `half_flux_px = 2·√(2·ln 2)·√(PSF_SIGMA_PX²
+ (FOCUS_SIGMA_PX·optics.focus_error)²)/frame.binning` with 030's two constants. It
counts the simulator's seeded field, not what a camera given `with_stars` drew.

### Solving on the console (`crates/finder/src/imaging/solving.rs`, new, US2; US3 adds hints; US4 and US5 add to it)

```rust
//! Solving an imaging frame (032): the hint, the solve, its view and its solve sidecar.
pub const SIDECAR_FORMAT: &str = "imaging-solve/1";
// US3:
pub const HINT_AGE_S: f64 = 30.0;
pub const WIDE_RADIUS_DEG: f64 = 3.0;
pub const WHOLE_SKY_DEG: f64 = 180.0;
pub const ARCSEC_PER_RADIAN: f64 = 206_264.806;

/// One saved imaging frame to solve.
#[derive(Debug, Clone)]
pub struct Ask<'a> {
    /// The imaging folder, and the frame's file relative to it, as 031's
    /// `FrameView::file` gives it: `frames/<UTC date>/<id>.fits`.
    pub root: &'a Path,
    pub file: &'a str,
    pub frame_id: u64,
    pub frame: SolveFrame,
    pub hint: SolveHint,
    /// Console Unix seconds when the solve was asked for.
    pub requested_at: f64,
}

/// One ended solve.
#[derive(Debug, Clone, PartialEq)]
pub struct Ended {
    pub outcome: Result<ImagingSolve, SolveError>,
    /// The solver's own figure for a solved solve; for a failed one, the console
    /// clock's time from `requested_at` to the answer, in milliseconds.
    pub took_ms: f64,
    /// The solve sidecar, relative to the imaging folder; `None` when it could not be
    /// written, in which case `outcome` is `Err(Failed("could not save the imaging
    /// solve: {error}"))`.
    pub file: Option<String>,
}

/// Solve one saved imaging frame and write its solve sidecar beside it. It touches
/// neither the imaging view nor the relation. `Imaging::solve` is built on it, and a
/// later spec that solves a frame other than the latest one (035: every sub) calls it
/// directly.
pub async fn solve_frame(solver: &dyn ImagingSolver, clock: &dyn crate::session::Clock,
    ask: Ask<'_>) -> Ended;

/// The latest solve as the imaging view shows it. Every `Option` is skipped while `None`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct SolveView {
    pub frame_id: u64,
    /// `solving`, `solved` or `failed`.
    pub state: &'static str,
    /// Console Unix seconds.
    pub requested_at: f64,
    pub solver: &'static str,
    pub hint: SolveHint,
    pub ra_deg: Option<f64>, pub dec_deg: Option<f64>, pub up_east_of_north_deg: Option<f64>,
    pub arcsec_per_px: Option<f64>, pub mirrored: Option<bool>, pub cd_deg: Option<[[f64; 2]; 2]>,
    pub stars: Option<u32>, pub warning: Option<String>, pub took_ms: Option<f64>,
    /// `frames/<UTC date>/<id>.solve.json`, relative to the imaging folder. It and
    /// `took_ms` are set when the solve ends, solved or failed; `file` stays `None`
    /// only when the solve sidecar could not be written.
    pub file: Option<String>,
    pub reason: Option<&'static str>, pub error: Option<String>, pub detail: Option<String>,
}

impl Imaging {
    /// Give the console its imaging solver, the session that hints it and the live loop
    /// whose solves it remembers. Called by `AppState::with_imaging_solver`.
    pub fn set_solver(&self, solver: Arc<dyn ImagingSolver>, session: Arc<Session>, live: Option<Arc<Live>>);
    /// The console's imaging solver, when it has one. 033 measures stars through it.
    pub fn solver(&self) -> Option<Arc<dyn ImagingSolver>>;
    /// Start solving the latest imaging frame, and answer the view.
    pub async fn solve(&self) -> Result<ImagingView, Refusal>;
    /// The solve of the frame with this id once it is solved or failed.
    pub async fn solved(&self, frame_id: u64) -> SolveView;
    // US3:
    /// The focal length in millimetres, for `AppState::with_focal_length_mm`.
    pub fn set_focal_length_mm(&self, mm: f64);
    /// The hint the console would give this frame now. `Imaging::solve` uses it, and
    /// so does a later spec that calls `solve_frame` itself.
    pub fn hint(&self, frame: &SolveFrame) -> SolveHint;
    /// Arcseconds a sensor pixel: the kept scale, or without one the focal length's
    /// with the camera's pixel size; `None` with neither. 033 turns a star measure's
    /// pixels into arcseconds with it.
    pub fn scale_arcsec_per_px(&self) -> Option<f64>;
    // US4:
    /// Offer a solve that `solve_frame` made outside `Imaging::solve` (035: every
    /// accepted sub). A whole-sensor frame takes an aim reading exactly as the task's
    /// step 2 does; a frame of a region does nothing. It changes neither the view's
    /// `solve`, the kept scale nor the misses.
    pub fn note_solve(&self, frame: &SolveFrame, frame_id: u64, solve: &ImagingSolve);
}

// US4. The aim point the relation is measured against: the session's held one (the
// session holds it back while Track is on), marked `measured`, when there is one;
// else the session's `aim`. The `bool` is whether it is held.
pub(crate) fn aim_in_force(session: &Session, now: f64) -> (Option<AimView>, bool);

// US3. The tests of US3 (US3-S8) and US5 (US5-S2) build one by literal, so every field
// is here from US3 on and no later story or spec adds one.
pub struct HintInputs<'a> {
    pub frame: &'a SolveFrame,
    /// The sensor's width and height in pixels, from the camera's description.
    pub sensor: (u32, u32),
    /// A live pointing: RA, Dec, time. US3 passes the session's latest when its source
    /// is live; from US4 on it is the history's (§ The hint).
    pub pointing: Option<(f64, f64, f64)>,
    pub last_solve: Option<(f64, f64)>,
    pub target: Option<(f64, f64)>,
    /// Arcseconds a sensor pixel, and the focal length: `Relation::height_inputs()`.
    pub scale: Option<f64>,
    pub pixel_um: f64,
    pub focal_length_mm: Option<f64>,
    /// Whether the session's aim point is a measured one. US3 passes false; US4 passes
    /// the truth; US5 makes `hint_for` read it.
    pub aim_measured: bool,
    /// How many places down the list of sources that exist the hint goes:
    /// `Relation::hint_skip()` (spec FR-014).
    pub skip: usize,
}
pub fn hint_for(inputs: &HintInputs<'_>) -> SolveHint;
```

The methods are `impl Imaging` blocks in a child module of the one that defines
`Imaging` (`crates/finder/src/imaging/mod.rs`, 031 US1), so they reach its private
state.

**`solve_frame`** awaits `solver.solve(ask.frame, ask.hint)`, takes `took_ms` (the
solve's own when solved; else `(clock.now() − ask.requested_at)·1000`), writes the
solve sidecar (below), and answers the `Ended`. A failed write turns the outcome into
`Failed("could not save the imaging solve: {error}")` and leaves `file` `None`. It does
not retry, and it knows nothing of the view, the relation or the session. `Clock` is
the console's (`crates/finder/src/session.rs:24`).

**`solve`**, refusing in this order, each a 409 `Refusal`: the camera is not `ready`,
with the words 031's `set` uses for each state; no solver, `this console has no imaging
solver`; a solve whose `state` is `solving`, `an imaging solve is already running`; no
frame, `take an imaging frame first`; a frame `exposing` or `receiving`, `the imaging
frame is still arriving`; a frame `failed` or `aborted`, `the latest imaging frame was
not saved`. Otherwise it builds the `SolveFrame` from the frame's view: the path is the
imaging folder joined with `file`; `roi` is `Region::from` of the view's `RoiView`;
`width`, `height`, `bits`, `binning`, `start` and `file` are `Option`s in 031's
`FrameView` that a `saved` frame always has, and were one missing the request is
refused with `the latest imaging frame was not saved`; `bayer` is as the view has it;
`exposure_s` is `exposure_ms/1000`. It builds the hint (US2:
`SolveHint::whole_sky()`; from US3: `self.hint(&frame)`), sets `solve` to `solving` with
`requested_at` from the console's clock, spawns the task and answers the view.

**The task** awaits `solve_frame` with that frame, hint and `requested_at`, and then,
in this order:
1. US3: tells the relation how the solve ended: a solved solve stores its scale; one
   that ended `not_solved` or `timed_out` is a miss (§ The relation), which also turns
   the hint (§ The hint);
2. US4: for a solved solve of a whole-sensor frame, takes an aim reading (§ The
   relation, the aim reading's use), through the same private function
   `note_solve` calls;
3. sets `solve` to `solved` or `failed` from the `Ended`, with its `took_ms` and
   `file` either way; from US3, keeps a solved solve's centre with the session's
   target of that moment as the last imaging solve; and wakes whoever waits in
   `solved`.

Steps 1 and 2 come before step 3 so that a reader who sees `solved` also sees what the
solve changed. Nothing here retries.

**The solve sidecar** is the frame's file name with `.fits` replaced by `.solve.json`.
It is written whole under that name plus `.part` and renamed over any earlier one, on
the blocking pool. Its keys, in order: `format`, `frame_id`, `frame` (the file's name
alone), `solver`, `requested_at`, `hint`, `solved`, `took_ms`, and then either `ra_deg`,
`dec_deg`, `up_east_of_north_deg`, `arcsec_per_px`, `mirrored`, `cd_deg`, `width`,
`height`, `stars` (when there are any) and `warning` (when there is one); or `reason`,
`error` and `detail` (when there is one). A failed solve's `took_ms` is the one the
view shows.

**The hint (US3).** US2 passes `SolveHint::whole_sky()`. From US3 on, `hint_for`:
- source and place: the sources that exist, in this order, are `pointing`, when it is
  `Some` and its time is at least `frame.start − HINT_AGE_S` and at most
  `frame.start + frame.exposure_s + HINT_AGE_S`; `last_solve`, when `Some`; `target`,
  when `Some`; and `none`, with no place, which always exists. With n of them, the
  hint takes the one at place `min(skip, n − 1)`, counted from 0: with `skip` 0 the
  first that exists, and never past `none`. The solve is made once, with that source
  (spec owner decisions 19 and 23); `skip` is what turns a later solve to the next
  source after three misses in a row;
- radius: `WHOLE_SKY_DEG` for `none`, else `WIDE_RADIUS_DEG`. US5 changes one case:
  for `pointing` with `aim_measured` and a `scale`, the sensor's diagonal on the sky,
  `hypot(sensor.0, sensor.1)·scale/3600`, held to at least `NARROW_MIN_DEG` and at
  most `WIDE_RADIUS_DEG`. It is the sensor's and not the frame's because the place of
  such a hint is the sensor's centre, and a frame of a region may lie anywhere on the
  sensor (spec US5-S2);
- field height: with s the `scale`, or without one `ARCSEC_PER_RADIAN·(pixel_um/1000)/
  focal_length_mm`, it is `height·binning·s/3600`; absent with neither.

`Imaging::hint` builds the inputs when the request is made:
- `sensor` and `pixel_um` from the camera's description;
- `pointing` (US3) from `session.snapshot(now).pointing` when its `source` is `Live`
  (`crates/finder/src/session.rs:424`): its `ra_deg`, `dec_deg` and `time`. It is
  already taken at the aim point (`crates/finder/src/session.rs:1192`);
- `pointing` (from US4, spec FR-035): among the history's fixes whose time passes the
  same test as above, the one nearest the exposure's middle, the earlier of two
  equally near. Its place is `fix.frame.unproject((aim.x, aim.y))` with the session's
  aim point of now, or `fix.frame.center` without one, as `record_live_pointing` does
  (`crates/finder/src/session.rs:1195`), turned to degrees; its time is the fix's.
  When the history holds no such fix, or the unprojection gives none, the US3 source
  is used;
- `last_solve`: the centre of the last solved imaging solve since the console
  started, kept in the imaging state with the session's target of that moment
  (`ra_deg` and `dec_deg`, or none); it is passed only while the session's target now
  is the same, both none or both the same two numbers;
- `target`: the session target's `ra_deg` and `dec_deg`
  (`crates/finder/src/targets.rs:53`);
- `scale` and `focal_length_mm`: `Relation::height_inputs()` (§ The relation), which
  is where the kept scale's three misses are counted;
- `skip`: `Relation::hint_skip()`, read under the same lock as `height_inputs`, so
  that the place and the field height turn on the same count;
- `aim_measured`: false in US3; from US4, whether the session's `aim` has the `source`
  `measured`. A measured aim point held back while Track is on does not count: the
  narrow radius is for a hint carried through the aim point live pointings are taken
  at.

### The imaging state (`crates/finder/src/imaging/mod.rs`, US1 to US4; never US5)

It gains `pub mod astap; pub mod solver;` (US1), `pub mod simulated; pub mod solving;`
(US2) and `pub mod relation;` (US3), each in alphabetical order. `ImagingView` gains,
after `frame`:

```rust
#[serde(skip_serializing_if = "Option::is_none")]
pub solve: Option<solving::SolveView>,          // US2
#[serde(skip_serializing_if = "Option::is_none")]
pub relation: Option<relation::RelationView>,   // US3
```

Both are filled only while `state` is `ready`. `solve` is `None` before the first
accepted request; `relation` is `None` while it would hold nothing.

`Imaging`'s state gains, story by story, only what that story reads, because a field
that is written and never read fails `clippy -D warnings`:
- US2: the solver and the latest `SolveView`. `set_solver` has its three parameters
  from US2 on, so that neither it nor `AppState::with_imaging_solver` changes later;
  US2 binds the two it does not use yet as `_session` and `_live`;
- US3: the session, which `Imaging::hint` reads; the last solved solve's centre with
  the session's target of that moment; and one `relation::Relation` (§ The relation),
  made with `Relation::new(root.join("relation.json"))` where the state is made. Where
  031's
  task keeps the camera's description on reaching `ready`, US3 adds
  `relation.describe(CameraId { .. })` from that description. `Imaging::view` fills
  `relation` with `relation.view()`;
- US4: the handle of the history's task, aborted where 031's own task is when the
  `Imaging` is dropped; and `Imaging::view` passes the aim point in force, as
  `relation.view(in_force, held)` with `(in_force, held) =
  solving::aim_in_force(&session, now)`, read before it takes its own lock, or
  `(None, false)` before `set_solver`.

That is all `mod.rs` holds of the relation. The history, the aim reading held, the
window, the misses and everything stored are fields of `Relation`, in `relation.rs`:
US4 and US5 add to that struct and to `solving.rs`, and US5 changes no line of
`mod.rs`, which is not among its writes.

### The route (`crates/finder/src/imaging/routes.rs`, US2)

It gains `POST /api/imaging/solve`, handled by `post_solve` (not to be confused with
`solving::solve_frame`, which it reaches through `Imaging::solve`): it takes
`SessionWrite` and no body,
as `take_frame` does; without imaging it answers 409 `this console takes no imaging
frames`; a `Refusal` becomes `ApiError::new(status, message)`; otherwise it answers
`Json(view)` with 200.

### The builders (`crates/finder/src/api.rs`, US2; US3)

Two builders on `AppState` (`crates/finder/src/api.rs:101`):

```rust
/// Give the console its imaging solver (032). Call after `with_live` and
/// `with_imaging`; without imaging it changes nothing.
pub fn with_imaging_solver(self, solver: Arc<dyn crate::imaging::solver::ImagingSolver>) -> Self;
/// The focal length, in millimetres, that sizes a solve's field until a scale is kept
/// (032). Call after `with_imaging`; a value that is not above 0 is ignored.
pub fn with_focal_length_mm(self, mm: f64) -> Self;      // US3
```

`with_imaging_solver` calls `set_solver` with the state's session
(`crates/finder/src/api.rs:242`) and live loop (`crates/finder/src/api.rs:301`).
`with_focal_length_mm` calls `Imaging::set_focal_length_mm`, which hands the value to
the `Relation`.

### The binary (`crates/finder/src/main.rs`, US2; US3)

`Args` (`crates/finder/src/main.rs:18`) gains:

```rust
/// ASTAP's command-line program: the imaging solver.
#[arg(long, requires = "astap_db_name", conflicts_with = "simulate")]
astap: Option<PathBuf>,
/// Folder of ASTAP's star database (default: where ASTAP itself looks).
#[arg(long, requires = "astap")]
astap_db: Option<PathBuf>,
/// Name of ASTAP's star database, such as d50.
#[arg(long, requires = "astap")]
astap_db_name: Option<String>,
/// Focal length in millimetres, used to size an imaging solve's field until a scale is measured.
#[arg(long, value_parser = focal_length)]      // US3
focal_length_mm: Option<f64>,
```

`requires` is already used there (`crates/finder/src/main.rs:41`). `focal_length` reads
an `f64` and refuses one that is not finite or not above 0 with `must be a number above
0`. After 031's `.with_imaging(dir)`: with `--astap`, `.with_imaging_solver` of an
`AstapSolver` whose work folder is `dir` joined with `solver`; under `--simulate`, of a
`SimulatedSolver` over `state.simulator()` (`crates/finder/src/api.rs:216`); otherwise
nothing. Then, with the flag, `.with_focal_length_mm`. No new start-up line.

### The relation (`crates/finder/src/imaging/relation.rs`, new, US3; US4 and US5 add to it)

```rust
//! What the console keeps between the finder camera and the imaging camera (032).
pub const FORMAT: &str = "imaging-relation/1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CameraId {
    pub model: String, pub serial: Option<String>, pub width: u32, pub height: u32, pub pixel_um: f64,
}
/// The kept scale: arcseconds a sensor pixel, and the solve it came from.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Scale { pub arcsec_per_px: f64, pub frame_id: u64, pub at: f64 }

/// After this many more solves have ended `not_solved` or `timed_out` with no solved
/// solve between them, the field height's next source is used (spec FR-016).
pub const SCALE_MISSES: u32 = 3;

/// `relation.json`. Each list defaults to empty when its key is absent and is left
/// out while empty.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stored {
    pub format: String,
    pub camera: CameraId,
    pub scale: Option<Scale>,
    pub readings: Vec<AimReading>,      // US5: the aim readings held, oldest first
    pub set_aside: Vec<AimReading>,     // US5
}
/// `None` when the file is missing, unreadable, of another format or of another camera.
pub fn load(path: &Path, camera: &CameraId) -> Option<Stored>;
/// Written as `relation.json.part` and renamed.
pub fn save(path: &Path, stored: &Stored) -> std::io::Result<()>;

/// `relation` in the imaging view. Every `Option` is skipped while `None`; the two
/// counts are always written by the story that adds them.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct RelationView {
    pub scale: Option<Scale>,
    pub focal_length_mm: Option<f64>,
    // US4:
    pub aim: Option<Xy>,
    pub angle_deg: Option<f64>,
    pub mirrored: Option<bool>,
    /// How many aim readings are held: `readings` in JSON.
    pub readings: usize,
    pub last: Option<Last>,
    pub field: Option<Field>,
    /// A measured aim point is held back while Track is on (spec FR-036). Written
    /// only while true: `#[serde(skip_serializing_if = "std::ops::Not::not")]`.
    pub held: bool,
    // US5:
    pub set_aside: usize,
}
// US4, the view's three small parts:
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Xy { pub x: f64, pub y: f64 }
/// How the last solved whole-sensor frame fared. In JSON: `measured`, `replaced`,
/// `set_aside`, `no_live_pointing`, `moving` or `off_frame`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LastOutcome { Measured, Replaced, SetAside, NoLivePointing, Moving, OffFrame }
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Last { pub frame_id: u64, pub outcome: LastOutcome }
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Field { pub width_arcsec: f64, pub height_arcsec: f64 }

/// What the console holds between the two cameras while it runs: the one value of
/// this type lives in the imaging state (§ The imaging state) from US3 on. Its fields
/// are private. US4 and US5 add fields and methods here; neither changes how `mod.rs`
/// holds it.
#[derive(Debug)]
pub struct Relation { /* US3: the file's path, the camera, the focal length, the kept
    scale, the count of misses. US4: the history, the aim reading held, the last
    outcome. US5: a `Window` in place of the one aim reading. */ }
impl Relation {
    /// Nothing known yet. `path` is `<imaging folder>/relation.json`; nothing is read.
    pub fn new(path: PathBuf) -> Self;
    /// A value that is not finite or not above 0 is ignored.
    pub fn set_focal_length_mm(&mut self, mm: f64);
    /// The camera was described. For the camera already held this does nothing.
    /// Otherwise it `load`s the file for this camera, takes what it holds or, with
    /// none, drops the kept scale and every aim reading, and zeroes the misses.
    pub fn describe(&mut self, camera: CameraId);
    /// The kept scale, or without one the focal length's with the camera's pixel
    /// size (`ARCSEC_PER_RADIAN·(pixel_um/1000)/focal length`); `None` with neither.
    pub fn scale_arcsec_per_px(&self) -> Option<f64>;
    /// `(scale, focal_length_mm)` for `HintInputs`, by the misses (below).
    pub fn height_inputs(&self) -> (Option<f64>, Option<f64>);
    /// A solve was solved: keep its scale in place of the last, zero the misses, and
    /// `save`. A failed save is printed to standard error and the value kept in memory.
    pub fn store_scale(&mut self, arcsec_per_px: f64, frame_id: u64, at: f64);
    /// A solve ended `not_solved` or `timed_out`: one more miss.
    pub fn miss(&mut self);
    /// How many places down the hint's list of sources the next solve goes: the
    /// misses in a row divided by `SCALE_MISSES`, a whole division (spec FR-014).
    pub fn hint_skip(&self) -> usize;
    /// US3. `None` while it would hold nothing.
    pub fn view(&self) -> Option<RelationView>;
    // US4 replaces `view` with:
    //   pub fn view(&self, in_force: Option<AimView>, held: bool) -> Option<RelationView>;
    // and adds:
    pub fn push_fix(&mut self, fix: FinderFix, keep_s: f64);
    pub fn fixes(&self) -> Vec<FinderFix>;
    /// Record what `read_aim` answered for a frame. `in_force` is the aim point in
    /// force of now, from `solving::aim_in_force`. It answers the aim point to hand to
    /// `Session::set_measured_aim`, when there is one to hand over.
    pub fn take(&mut self, outcome: Outcome, frame_id: u64, in_force: Option<AimView>)
        -> Option<(f64, f64)>;
}
```

**The scale (US3).** 031's task calls `describe` with the `CameraId` of the camera's
description each time it reaches `ready` (§ The imaging state). The task of § Solving
on the console calls `store_scale(solve.arcsec_per_px / frame.binning, frame_id, now)`
for a solved solve, now being the console's clock, and `miss()` for one that ended
`not_solved` or `timed_out`; any other failure calls neither. `relation.focal_length_mm`
shows the builder's value and is not stored. `relation.field` (US4) is the camera's
width and height times the kept scale, absent without one or before `describe`.

**The field height's sources (US3, spec FR-016).** `height_inputs` lists the sources
that exist, in this order: the kept scale, when one is kept; the focal length, when one
is set; and none, which always exists. With n of them and m misses, the source in turn
is number `(m / SCALE_MISSES) % n`, counted from 0, the division being a whole one. It
answers `(Some(kept), focal)` for the kept scale, `(None, focal)` for the focal length
and `(None, None)` for none, focal being the focal length as set or `None`. So a kept
scale that has stopped being true is passed over after three misses and tried again
later, and nothing has to be deleted by hand; `store_scale` zeroes m. The count is not
stored: a restart begins from the kept scale.

**The hint's turn (US3, spec FR-014).** `hint_skip` answers `m / SCALE_MISSES` from the
same m, so after three misses in a row the hint goes one source down as the field
height goes one source on, and `store_scale` returns both to the first. Unlike the
field height it does not turn back from the last source: `hint_for` holds it at
`none`, the search of the whole sky, which no hint can lock out. `describe` of another
camera zeroes m, and so both.

`view` answers `None` when the scale, the focal length, `aim`, `last` and `field` are
all `None`, both counts are 0 and, from US4, `held` is false.

**The history and the aim reading (US4).**

```rust
pub const HISTORY_MIN_S: f64 = 120.0;
/// The most fixes the history holds over its span: a fix that follows the newest one
/// kept by less than `keep_s / HISTORY_MAX as f64` seconds is not kept (spec FR-019).
/// It is a density, not a queue length: no fix is ever dropped for being one too many.
pub const HISTORY_MAX: usize = 2000;
/// The fit reaches this far before the exposure's start and after its end, or the
/// exposure's own length if that is longer.
pub const FIT_REACH_S: f64 = 10.0;
/// Fewer fixes than this in the fit take no aim reading.
pub const FIT_MIN: usize = 3;
pub const MAX_TRAVEL_ARCSEC: f64 = 60.0;
pub const MAX_SCATTER_ARCSEC: f64 = 90.0;

/// One live solve of the finder camera: a finder fix.
#[derive(Debug, Clone, Copy)]
pub struct FinderFix { pub time: f64, pub frame: crate::sky::Frame, pub frame_id: u64 }

#[derive(Debug, Default)]
pub struct FinderHistory { /* a VecDeque<FinderFix>, oldest first */ }
impl FinderHistory {
    /// Keeps the fix unless one is held and the fix's time is less than
    /// `keep_s / HISTORY_MAX as f64` after the newest one's; then drops every fix whose
    /// time is more than `keep_s` before the newest one's. Nothing else is dropped.
    pub fn push(&mut self, fix: FinderFix, keep_s: f64);
    pub fn fixes(&self) -> Vec<FinderFix>;
}

/// One reading of the aim point and the imaging angle: an aim reading. The tests of
/// US4 and US5 build it by literal, so every field is here from US4 on and no later
/// story or spec adds one.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AimReading {
    pub x: f64, pub y: f64, pub angle_deg: f64, pub mirrored: bool,
    /// The middle of the imaging exposure, console Unix seconds.
    pub at: f64,
    pub frame_id: u64,
    pub finder_arcsec_per_px: f64,
    /// The size of the finder frame `x` and `y` are pixels of.
    pub finder_width: f64,
    pub finder_height: f64,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Outcome { Measured(AimReading), NoLivePointing, Moving, OffFrame }

pub fn read_aim(fixes: &[FinderFix], solve: &ImagingSolve, start: f64, exposure_s: f64,
    frame_id: u64) -> Outcome;
```

"Measure" alone is the solver port's star measure (`ImagingSolver::measure`,
`StarMeasure`, `MEASURE_LIMIT`); a reading of the aim point is an aim reading here, in
the view (`readings`) and in the file, though the aim point it gives is still
"measured" (`Outcome::Measured`, `source: "measured"`).

The history is filled by a task `set_solver` starts when it is given a live loop. It
reads `live.subscribe()` (`crates/finder/src/live.rs:263`); for each report with a
`pointing_seq` it reads `session.snapshot(clock.now()).pointing` and, when that
pointing's `seq` is the report's and its `source` is `Live`, pushes `FinderFix { time:
pointing.time, frame: pointing.frame, frame_id: report.frame_id }` with `keep_s` the
larger of `HISTORY_MIN_S` and four times the settings' exposure in seconds, through
`Relation::push_fix`. A lagged receiver carries on; a closed one ends the task. A
report whose pointing has already been replaced is skipped. The task's handle is kept
with the imaging state and aborted when the `Imaging` is dropped, as 031's own task is.
At two live solves a second the history keeps every fix while `keep_s` is under
1000 s and thins to one a second at 1200 s, four subs of 300 s, so the fit of
`read_aim`, which reaches one exposure before the sub and one after it, always finds
its span (spec US4-S2).

`read_aim`, with `t0 = start`, `t1 = start + exposure_s`, `ti` their middle,
`R = max(exposure_s, FIT_REACH_S)` and c the solve's centre as a unit vector
(`UnitVector::from_radec` of its RA and Dec in radians):
1. F is the fix with `t0 ≤ time ≤ t1` nearest `ti`, the earlier of two equally near.
   None: `NoLivePointing`.
2. `pF = F.frame.project(c)` (`crates/finder/src/sky.rs:101`). Missing, or outside F's
   frame (`contains(pF, 0.0)`, `crates/finder/src/sky.rs:130`): `OffFrame`.
   `φ = F.frame.up_east_of_north_at(c)`; none: `OffFrame`. `s` is
   `F.frame.arcsec_per_px()`.
3. The fitted fixes are those with `t0 − R ≤ time ≤ t1 + R` whose frame has F's
   `size()` and whose own `project(c)` exists and lies inside their own frame; F is one
   of them. Each gives a point `(τ, x, y)`, with `τ = time − ti` and (x, y) that
   projection. Fewer than `FIT_MIN` points: `NoLivePointing`.
4. The line, by least squares in each axis. With n points, `τ̄`, `x̄` and `ȳ` their
   means and `D = Σ(τ − τ̄)²`: when D is not above 1e-9, `NoLivePointing`; else
   `bx = Σ(τ − τ̄)(x − x̄)/D`, `ax = x̄ − bx·τ̄`, and `by`, `ay` likewise. The aim point
   at the middle of the exposure is `(ax, ay)`.
5. The scatter about the line: `sx = √(Σ(x − ax − bx·τ)²/(n − 2))`, `sy` likewise, and
   `scatter = hypot(sx, sy)·s` in arcseconds. Above `MAX_SCATTER_ARCSEC`: `Moving`.
6. The travel during the exposure: `travel = hypot(bx, by)·exposure_s·s`, and its
   standard error `error = hypot(sx, sy)/√D·exposure_s·s`. `travel − error` above
   `MAX_TRAVEL_ARCSEC`: `Moving`.
7. The imaging angle is `(solve.up_east_of_north_deg − φ).rem_euclid(360.0)`.
8. `Measured` with x `ax`, y `ay`, the angle, the solve's `mirrored`, `at` ti, the
   frame's id, `s`, and F's `size()` as `finder_width` and `finder_height`.

Why a line through all of them and not the difference of two: the finder camera's
solves scatter by σ each (0.3′ to 0.5′ at the telescope,
`docs/research/2026-11-15-imaging-camera.md` § 5. Relating two cameras). Two fixes one
frame interval Δ apart give a rate uncertain by `√2·σ/Δ`, which for a 2 s exposure and
half a second between frames is 100″ or more of apparent travel from a telescope that
is standing still. The fitted rate is uncertain by `σ/√D`; with forty fixes over
twenty seconds D is about 1500 s² and the same scatter is under 2″ of travel. The
scatter test is what tells a telescope that was bumped, or a slew that ended inside
the span, from a steady drift: its fixes do not lie on one line.

**The aim reading's use (US4).** One private function of `solving.rs`,
`take_aim_reading(&self, frame: &SolveFrame, frame_id: u64, solve: &ImagingSolve)`, is
step 2 of the task of § Solving on the console and the whole of `note_solve`:
1. Unless the frame's `roi` is the whole sensor of the description (x and y 0, the
   description's width and height), it returns and nothing changes.
2. Under the imaging state's lock it reads `relation.fixes()` and releases the lock;
   then `read_aim(&fixes, solve, frame.start, frame.exposure_s, frame_id)`.
3. It reads `(in_force, _) = aim_in_force(&session, now)`, takes the lock again, hands
   the outcome to `Relation::take` with the frame's id and `in_force`, and releases
   the lock.
4. When `take` answered an aim point, it calls `Session::set_measured_aim` with it,
   which applies it or, while Track is on, holds it back (§ The session); a failure of
   that call is printed to standard error.

The imaging state's lock is never held across a call into the session (§ Plan traps
30). `note_solve` reads no view, stores no scale and counts no miss, so it changes
neither `solve`, `relation.scale` nor the hint's turn. `aim_in_force` reads
`session.held_aim()` first: with one held it answers `(Some(AimView { x, y, source:
Some("measured") }), true)`; else `(session.snapshot(now).aim, false)`.

`take` in US4: when `in_force` does not have the `source` `measured`, the aim reading
held is dropped first. Then `Measured(reading)` becomes the one aim reading held, sets
`last` to the frame's id and `Measured`, and answers `Some((reading.x, reading.y))`;
any other outcome sets `last` to the frame's id and `NoLivePointing`, `Moving` or
`OffFrame`, changes nothing else and answers `None`. Because `in_force` is the held aim
point while one is held, aim readings taken while Track is on are kept, and smoothed in
US5, against it.

`view` in US4, with `in_force` and `held` from `aim_in_force`: `aim`, `angle_deg` and
`mirrored` come from the aim reading held, and `readings` is 1, only while `in_force`
has the `source` `measured` and one is held; otherwise the three are `None` and
`readings` is 0. `held` is the argument. `last` and `field` do not depend on either.

**The window (US5).**

```rust
pub const WINDOW: usize = 5;
pub const OUTLIER_ARCSEC: f64 = 120.0;
pub const OUTLIER_ANGLE_DEG: f64 = 1.0;
pub const REPLACE_AFTER: usize = 3;
pub const AIM_STEP_ARCSEC: f64 = 10.0;
pub const NARROW_MIN_DEG: f64 = 0.2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Taken { Measured, SetAside, Replaced }

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Window { /* the aim readings held, oldest first, and those set aside */ }
impl Window {
    pub fn new() -> Self;
    pub fn from_parts(readings: Vec<AimReading>, set_aside: Vec<AimReading>) -> Self;
    pub fn take(&mut self, reading: AimReading) -> Taken;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn set_aside(&self) -> usize;
    /// The medians of x and of y.
    pub fn aim(&self) -> Option<(f64, f64)>;
    /// The median imaging angle, taken around the circle, 0 to 360.
    pub fn angle_deg(&self) -> Option<f64>;
    /// The newest aim reading's.
    pub fn mirrored(&self) -> Option<bool>;
    pub fn readings(&self) -> &[AimReading];
    pub fn aside(&self) -> &[AimReading];
    pub fn clear(&mut self);
}
/// Whether the session's aim point should be set from a median.
pub fn should_apply(in_force: Option<AimView>, median: (f64, f64), finder_arcsec_per_px: f64) -> bool;
```

- A median of an odd count is the middle value; of an even count, the mean of the
  middle two. The angle's median: with a0 the oldest angle held, each angle becomes its
  difference from a0 reduced into −180 to 180, the median of those is added to a0, and
  the sum is taken `rem_euclid(360.0)`.
- `take` first looks at the finder frame: when the reading's `finder_width` or
  `finder_height` differs from that of the newest aim reading held, or of the newest
  set aside when none is held, everything held and set aside is dropped, because
  pixels of a finder frame of another size mean something else.
- `take` then: with at least `REPLACE_AFTER` aim readings held, the reading is an
  outlier when its distance from `aim()` in pixels times its own
  `finder_arcsec_per_px` is above `OUTLIER_ARCSEC`, or its angle differs from
  `angle_deg()` by more than `OUTLIER_ANGLE_DEG`, or its `mirrored` is not
  `mirrored()`. An outlier is set aside; when that makes `REPLACE_AFTER` set aside,
  they become the aim readings held, nothing is set aside, and the answer is
  `Replaced`; otherwise `SetAside`. A reading that is not an outlier clears those set
  aside, is added, pushes out the oldest beyond `WINDOW`, and answers `Measured`.
- `should_apply` is true with no aim point in force, with one whose `source` is not
  `measured`, and when the distance from it to the median in pixels times
  `finder_arcsec_per_px` is above `AIM_STEP_ARCSEC`.

In US5 a `Window` replaces the one aim reading `Relation` held in US4, and
`Relation::take` and `Relation::view` keep their signatures: only their bodies change,
in `relation.rs`.
- `Relation::take`: when `in_force` does not have the `source` `measured`, the window
  is cleared first. `Measured(reading)` goes to `Window::take`; `last` is the frame's
  id and `Measured`, `SetAside` or `Replaced` as the window answered. After `Measured`
  or `Replaced` it answers `Some(aim())` when `should_apply(in_force, aim(),
  reading.finder_arcsec_per_px)` is true, else `None`; after `SetAside`, `None`. The
  other outcomes are as in US4. After every call that reached `Window::take` it
  `save`s `relation.json` with the window's two lists; a failed save is printed to
  standard error.
- `Relation::describe` takes the stored lists into the window with `from_parts`, and
  `Relation::store_scale` writes the window's two lists with the scale, so that
  neither save loses what the other wrote.
- `Relation::view`: `aim`, `angle_deg` and `mirrored` are the window's, and `readings`
  its `len()`, while `in_force` has the `source` `measured`; otherwise the three are
  `None` and `readings` is 0. `held` is still the argument. `set_aside` is the window's
  count. `last.outcome` now also takes `set_aside` and `replaced`.
- `solving.rs` changes in one place: `hint_for` reads `aim_measured` for the narrow
  radius (§ Solving on the console). `take_aim_reading`, and so the task and
  `note_solve`, are US4's, unchanged.

### The session (`crates/finder/src/session.rs`, US4)

```rust
/// The aim point as the session reports it. `source` is `measured` when the console
/// measured it from the imaging camera, and absent when it was tapped.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct AimView {
    pub x: f64,
    pub y: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<&'static str>,
}
impl Session {
    /// Persist a measured aim point (032). Like `set_aim`, and the read then carries
    /// `source: "measured"` until the next `set_aim`. While Track is on it holds the
    /// aim point back instead: nothing is persisted, the view's `aim` is unchanged,
    /// and `held_aim` answers it until the first live solve recorded with Track off
    /// applies it (spec FR-036).
    pub fn set_measured_aim(&self, aim: Aim, time: f64) -> Result<SessionView, SessionError>;
    /// The measured aim point held back while Track is on, if any (032).
    pub fn held_aim(&self) -> Option<Aim>;
}
```

- `Aim` (`crates/finder/src/session.rs:395`) does not change. `SessionView::aim`
  (`crates/finder/src/session.rs:569`) becomes `Option<AimView>`.
- The session's values (`crates/finder/src/session.rs:593`) gain `aim_measured: bool`,
  false at first. The view builds `AimView` from the aim and that flag. `set_aim`
  (`crates/finder/src/session.rs:1234`) clears it where it sets the aim
  (`crates/finder/src/session.rs:1240`); `set_measured_aim` validates, sets both, and
  persists before committing, as `set_aim` does.
- `StoredSession` (`crates/finder/src/session.rs:1452`) gains, after `aim`
  (`crates/finder/src/session.rs:1467`), `#[serde(default, skip_serializing_if =
  "std::ops::Not::not")] aim_measured: bool`. It is read as true only with an `aim`, and
  written from the values (`crates/finder/src/session.rs:1550`).
- A recording's `recording.json` serializes the view's `aim`
  (`crates/finder/src/recording.rs:250`), so a recording made with a measured aim point
  says so, and one made with a tapped one reads as before.

**The held aim point (US4, spec FR-036, owner decision 25).**
- The values gain `held_aim: Option<Aim>`, `None` at first. It is run-only:
  `StoredSession` does not carry it, so a restart forgets it.
- Track is on when the session's `track` would read `wanted` true: a private
  `fn track_on(&self) -> bool` on `TrackRun`, `self.wanted && (self.armed ||
  self.tracker.as_ref().is_some_and(|t| t.state().state != "off"))`, the same test
  `TrackRun::view` makes for `TrackView::wanted`
  (`crates/finder/src/session.rs:182`), and false with no `track`. The tracker's state
  is read under the values lock exactly as `TrackRun::view` already reads it there.
- `set_measured_aim(aim, time)` validates the aim as `set_aim` does. With Track on it
  sets `held_aim = Some(aim)` and answers the view, writing nothing. With Track off it
  sets `aim = Some(aim)`, `aim_measured = true` and `held_aim = None`, persisting
  before committing, as `set_aim` does.
- `set_aim` (`crates/finder/src/session.rs:1234`), the tap's and the reset's way in,
  also sets `held_aim = None`, so a tap or a reset drops a held aim point.
- `record_live_pointing` (`crates/finder/src/session.rs:1192`) applies a held aim
  point first: when `held_aim` is `Some` and Track is off, it does what
  `set_measured_aim` does with Track off, under the lock it already holds, and only
  then takes the live pointing at the aim point, so that pointing is the first one
  taken at the new aim point. A failed write is printed to standard error, as
  `Session::snapshot` prints one (`crates/finder/src/session.rs:1007`), the held aim
  point is dropped and the aim point stays as it was: the next aim reading measures it
  again.
- Nothing of Track, of `start_tracking`, `stop_tracking`, `track_live_pointing` or of
  `crates/finder/src/tracking.rs` changes. With Track on, live pointings stay taken at
  the aim point Track started from, so 015's tracker sees no step; the first live
  solve after any end of Track (the observer's Stop or Track off, or the tracker's own
  ending) takes the new one up.

### The finder frame's up axis (`crates/finder/src/sky.rs`, US4)

```rust
impl Frame {
    /// The angle, east of celestial north in 0 to 360, of this frame's up axis as seen
    /// at sky direction `v`: the up axis less its part along `v`. `None` where that is
    /// not defined: at a celestial pole, or where the up axis lies along `v`.
    pub fn up_east_of_north_at(&self, v: UnitVector) -> Option<f64>;
}
```

With k the third row of the frame's rotation, the camera's up in J2000
(`crates/finder/src/sky.rs:17`), and ẑ the pole: `u = k − (k·v)·v`, `n = ẑ − (ẑ·v)·v`,
`e = ẑ × v`; any of the three shorter than 1e-9 answers `None`; else, with n and e
normalised, `atan2(u·e, u·n)` in degrees, `rem_euclid(360.0)`. This is the definition
030's simulator uses for the true imaging angle (`crates/simulator/src/imaging.rs`, 030
US2: up is `cos α·u0 + sin α·l0`, u0 being the finder's up less its part along the
imaging boresight), so the aim reading's angle and the truth are the same quantity.

### The page (`crates/finder/web/src/`, US6)

`api.ts`: `aim?: { x: number; y: number; source?: "measured" }` in `ObservingSession`
(`crates/finder/web/src/api.ts:264`), `imaging?: ImagingView;` beside it, and, after
031's imaging types:

```ts
export interface ImagingSolveHint {
  source: "pointing" | "last_solve" | "target" | "none";
  ra_deg?: number; dec_deg?: number; radius_deg: number; fov_deg?: number;
}
export interface ImagingSolve {
  frame_id: number; state: "solving" | "solved" | "failed"; requested_at: number; solver: string;
  hint: ImagingSolveHint; ra_deg?: number; dec_deg?: number; up_east_of_north_deg?: number;
  arcsec_per_px?: number; mirrored?: boolean; cd_deg?: number[][]; stars?: number; warning?: string;
  took_ms?: number;
  file?: string; reason?: string; error?: string; detail?: string;
}
export interface ImagingRelation {
  scale?: { arcsec_per_px: number; frame_id: number; at?: number };
  focal_length_mm?: number; aim?: { x: number; y: number }; angle_deg?: number; mirrored?: boolean;
  readings?: number; set_aside?: number; held?: boolean;
  last?: { frame_id: number; outcome: "measured" | "replaced" | "set_aside" | "no_live_pointing" | "moving" | "off_frame" };
  field?: { width_arcsec: number; height_arcsec: number };
}
// ImagingView gains: solve?: ImagingSolve; relation?: ImagingRelation;
export async function postImagingSolve(): Promise<ImagingView>;   // fetch(url, { method: "POST" })
```

`postImagingSolve` is written as 031's `postImagingFrame` is and throws an `Error` with
the answer's `error` on a refusal.

`useSession.ts`: one line after the `recording` line
(`crates/finder/web/src/useSession.ts:46`): `...(body?.imaging != null && { imaging:
body.imaging }),`. The `aim` line (`crates/finder/web/src/useSession.ts:38`) already
copies the whole object, `source` included.

`components/ImagingSolveBlock.tsx`, `ImagingSolveBlock({ view, refresh }: { view:
ImagingView; refresh: () => Promise<void> | void })`: a `section` labelled by an `h2`
`Imaging solve`, with these words exactly, each phrase a test asserts on one line of
JSX:
- the button `Solve imaging frame` (`button`), disabled unless `view.frame?.state` is
  `saved`, and while `view.solve?.state` is `solving`. A click calls `postImagingSolve`,
  then `refresh`; a refusal shows its words in an `alert` and calls no `refresh`;
- `solving`: a `status` `Solving imaging frame {frame_id}`;
- `solved`: a `status` `Solved imaging frame {frame_id}: RA {ra_deg.toFixed(4)}° Dec
  {dec_deg.toFixed(4)}° · up {up_east_of_north_deg.toFixed(1)}° east of north ·
  {arcsec_per_px.toFixed(2)}″ per pixel`, then ` · {stars} stars` when `stars` is
  present, then ` · mirrored` when `mirrored` is true;
- `failed`: an `alert` `Imaging solve failed: {error}.`;
- with `relation.scale`, a `quiet` paragraph `Scale {arcsec_per_px.toFixed(3)}″ per
  sensor pixel, from imaging frame {frame_id}.`;
- with `relation.aim`, a `quiet` paragraph `Aim point measured at ({x.toFixed(1)},
  {y.toFixed(1)}) px of the finder camera's frame · imaging angle
  {angle_deg.toFixed(1)}° · from {readings} imaging solves.`;
- with `relation.last.outcome` one of the four below, a `quiet` paragraph `The last
  imaging solve did not measure the aim point: {why}`, why being `the telescope was
  moving.` for `moving`, `no live pointing shared its moment.` for `no_live_pointing`,
  `its centre is outside the finder camera's frame.` for `off_frame`, and `it disagreed
  with the earlier ones and was set aside.` for `set_aside`;
- with `relation.held` true, a `quiet` paragraph `The measured aim point waits until
  Track is off.`

Only existing classes are used. `ImagingPage.tsx` (031 US4) renders
`<ImagingSolveBlock view={view} refresh={refresh} />` right after its Imaging frame
section, only when the view is `ready`.

`components/ImagingField.tsx`:

```tsx
/** The imaging frame's corners in the finder frame's pixels: top-left, top-right,
 *  bottom-right, bottom-left. Null when the finder scale is not above 0. */
export function imagingFieldCorners(
  aim: { x: number; y: number }, angleDeg: number, mirrored: boolean,
  field: { width_arcsec: number; height_arcsec: number }, finderArcsecPerPx: number,
): [number, number][] | null;
export function ImagingField(props: { corners: [number, number][]; u: number });
```

With α the angle in radians, `up = (−sin α, −cos α)`, `right = (cos α, −sin α)`, or its
negative when mirrored, `hw = width_arcsec/(2·finderArcsecPerPx)` and `hh` likewise: the
corners are `aim + hh·up − hw·right`, `aim + hh·up + hw·right`, `aim − hh·up +
hw·right`, `aim − hh·up − hw·right`. `ImagingField` draws `<g data-imaging-field
className="furniture pointer-events-none" strokeWidth={1.3 * u}>` holding `<title>Imaging
field</title>` and one `<path fill="none">` whose `d` is the four corners as `M`, `L`,
`L`, `L`, `Z`, each number `toFixed(1)` as the chart's `toPath` writes them
(`crates/finder/web/src/components/Chart.tsx`), followed by a tick from the middle of
the top side outward by `6·u`. The `furniture` class gives it ink by day and red at
night with no new CSS.

`Chart.tsx`: after the target mark
(`crates/finder/web/src/components/Chart.tsx:241`), when `live`, the report has a
pointing, `session?.aim?.source === "measured"`, and `session.imaging?.relation` has
both `angle_deg` and `field`, it draws `ImagingField` with the corners of the session's
`aim`, that angle, `mirrored` (false when absent), that field and the report's
`pointing.arcsec_per_px`; nothing when the corners are null.

`AimTap.tsx`: after the coordinates' paragraph
(`crates/finder/web/src/components/AimTap.tsx:48`), a `quiet` paragraph: `Measured from
the imaging camera.` followed, when `session.imaging?.relation?.angle_deg` is a number,
by ` Imaging angle {angle_deg.toFixed(1)}°.`, when the shown aim point has the `source`
`measured`; `Set by tapping a star.` when it has an aim point without one; nothing with
no aim point. It follows the component's own `aim` state, so a tap or a reset changes
the line at once.

### `DESIGN.md` and `.impeccable/design.json` (US6)

Three edits. The `>` marks only set the text apart here.

After 031's Imaging page entry, a plain `###` heading and one paragraph:

> ### Imaging field on the chart — built
> While the aim point is one the console measured, the live chart draws the imaging
> camera's field as a thin ink outline centred on the aim point: a rectangle of the
> sensor's size on the sky, turned by the imaging angle, with a short tick on its up
> side. It is furniture, like the reticle and the compass: no fill, no label, not a
> chart layer, and red at night with the rest. A tapped aim point draws nothing.

At the end of the Aim-point tap entry (`DESIGN.md:347`):

> Under the coordinates a quiet line says how the aim point was set: Set by tapping a
> star, or, when the console measured it, Measured from the imaging camera, with the
> imaging angle.

At the end of 031's Imaging page entry:

> After the imaging frame comes an Imaging solve block: one secondary button, Solve
> imaging frame; a status with the solved centre, the angle of up east of north, the
> scale and the star count, or an alert with the reason it failed; and quiet lines for
> the kept scale, the measured aim point, and, while Track is on, that a newly
> measured aim point waits until Track is off.

`.impeccable/design.json` gains one entry in `components`, after 031's `Imaging page`,
formatted as its neighbours: `name` `Imaging field`, `kind` `custom`, `refersTo`
`display`, a `description` saying the first paragraph in one sentence each, and `html` and
`css` snippets under the class `ds-imaging-field`, using only `var(--color-ink)`.

### The MCP tool (`crates/finder/src/mcp/tools.rs`, US7)

One tool in the `#[tool_router]` block (`crates/finder/src/mcp/tools.rs:165`), with
`output_schema = object_schema()` (`crates/finder/src/mcp/tools.rs:104`) and no
read-only annotation:

- `solve_imaging_frame`, description
  `Solve the latest imaging frame, as POST /api/imaging/solve does. It waits until the solve ends and returns the imaging camera's state with the solve.`:
  takes `arguments: JsonObject`; any key is the error `solve_imaging_frame takes no
  arguments`. Without imaging it answers the error `this console takes no imaging
  frames`. It calls `Imaging::solve`; a `Refusal` is `CallToolResult::error` with its
  words. Then `Imaging::solved(frame_id)` with the id of the view's `solve`: `solved`
  answers `CallToolResult::structured` of `Imaging::view()`; `failed` answers the error
  with the solve's own `error`.

It is not a moving tool: no rate limit and no call-log line. It is written as 031's
`take_imaging_frame` is (031 US5).

### `crates/finder/src/lib.rs` and `crates/finder/README.md`

`lib.rs` (US2): one line after 031's last route line, written as it stands here, with no
backticks:
`* POST /api/imaging/solve: no body; solve the latest imaging frame and write its solve beside it.`

`crates/finder/README.md`:
- US2 adds a row after 031's last row of the `## API` table
  (`crates/finder/README.md:167`):

  ```markdown
  | `POST /api/imaging/solve` | No body. Solve the latest saved imaging frame with the imaging solver; answers 200 with the imaging view, whose `solve` goes from `solving` to `solved` or `failed`, or 409 with a JSON `error`. Same-origin writes only. |
  ```

  and a section `## Solving imaging frames` after 031's `## Imaging frames`, saying:
  what an imaging solve is; that the imaging solver is ASTAP's `astap_cli`, an MPL-2.0
  program the owner installs with one of its star databases, whose data asks for the
  credit "ESA/Gaia/DPAC"; the three flags, and that `--simulate` uses the simulated
  solver adapter; that ASTAP is given a copy and never the saved file, in a work
  folder `solver/` under the imaging folder whose leftovers may be deleted; the
  `solve` object, the solve sidecar's name and keys; the five reasons and their
  messages; and the three limits, with the warning that a search of the whole sky may
  run out its 600 s.
- US3 adds, in that section: the hint's four sources in order, with the rules that a
  failed solve is not asked again, that after three misses in a row the next solve
  takes the next source that exists, ending at a search of the whole sky, and that
  the last imaging solve stops counting when the target changes; the 30 s rule; the radii (3° and the whole sky); the field
  height's three sources and the rule of three misses that moves from one to the
  next, with what to do after a change of optics (nothing, or give
  `--focal-length-mm` its new figure when the frame is under 0.3° high, the smallest
  height ASTAP searches for by itself); `relation.json` and its scale; and
  `--focal-length-mm`.
- US5 adds, in that section, `### The measured aim point`: what is measured and when
  (an imaging solve of the whole sensor, and a line fitted through the finder
  camera's live solves from 10 s before the exposure to 10 s after it, read at the
  exposure's middle); the three outcomes that take no aim reading and what each means
  (`no_live_pointing`: none inside the exposure, or fewer than three in the span;
  `moving`: more than 60″ of travel during the exposure, or more than 90″ of scatter
  about the line; `off_frame`); the median of five, what is set aside and when three
  replace the window; the 10″ step; that a tap or a reset is obeyed and replaced at
  the next aim reading; that an aim point measured while Track is on waits, shown as
  held, until the first live solve after Track is off, so the tracker never chases
  it, and that a restart forgets a held one; that a later spec may offer its own
  solves (`Imaging::note_solve`); the narrow radius, the sensor's diagonal; that the
  live pointing a hint uses is the live solve nearest the exposure's own middle; and
  what `relation.json` and `session.json` keep.
- US7 adds `solve_imaging_frame` to the tool list in `## MCP clients`
  (`crates/finder/README.md:57`), after 031's three, worded like its neighbours.

## Shapes that existing tests pin

These are not changed by any story:

- **`Aim` and `Session::set_aim`.** Eleven test files build an `Aim` by literal, most as
  `Aim { x: 700.0, y: 420.0 }`, and call `set_aim(Some(..), time)`, among them
  `crates/finder/tests/support/live.rs:73` and
  `crates/finder/tests/console/session/durable.rs:47`. `Aim` gains no field and
  `set_aim` keeps its signature; the measured path is a second method.
- **A tapped aim point's JSON.** Six assertions require `aim` to equal exactly
  `{"x":700.0,"y":420.0}` (`crates/finder/tests/console/aim.rs:37`,
  `crates/finder/tests/cli/aim.rs:198`, `crates/finder/tests/cli/aim.rs:209`,
  `crates/finder/tests/mcp/session_tools.rs:219`,
  `crates/finder/tests/console/recording.rs:613`,
  `crates/finder/tests/console/session/durable.rs:61`). `source` is skipped unless
  measured, and nothing in those tests measures.
- **`session.json`'s keys.** A session with a site, a minimum altitude and a target
  stores exactly three keys (`crates/finder/tests/cli/session.rs:123`), and a reset aim
  point leaves no `aim` key (`crates/finder/tests/cli/aim.rs:217`). `aim_measured` is
  written only when true, and the held aim point never.
- **Track, the tracker and live pointings.** Track's routes, its states and the
  tracker are not changed (`crates/finder/tests/motion/tracking/routes.rs:85` turns
  Track on through a GoTo, and `crates/finder/tests/motion/tracking/routes.rs:95`
  without one). `record_live_pointing` takes every live pointing as before while no
  aim point is held, which is always on a console without an imaging solver
  (`crates/finder/tests/console/live/loop_.rs:45` bounds those pointings at 30″).
- **`POST /api/aim` and `DELETE /api/aim`**: their bodies, statuses and words
  (`crates/finder/tests/console/aim.rs:37`), and the session view they answer.
- **`Frame`'s existing methods** (`crates/finder/src/sky.rs:35`): one method is added,
  none changes; a test builds frames with `Frame::from_solution`
  (`crates/finder/tests/console/live/loop_.rs:128`).
- **The live loop.** `LiveReport` (`crates/finder/src/live.rs:53`) gains no field and
  `Live::start` keeps its signature; the history is one more subscriber of the
  broadcast, as 025's recorder is.
- **`GET /api/session`, `GET /api/imaging` and the MCP bodies before any solve.**
  `solve` and `relation` are skipped while absent, a console without imaging still
  answers `{"state":"off"}`, and the view outside `ready` holds nothing new, so every
  body 031's tests read is what it was. A console with no solver never fills either.
- **The simulated console of the page's end-to-end checks.** It starts with no aim
  point (`crates/finder/web/e2e/simulate.spec.ts:120`) and nothing solves an imaging
  frame there, so it stays so; Setup still shows one `status`
  (`crates/finder/web/e2e/simulate.spec.ts:318`), since the new lines are plain
  paragraphs.
- **Setup's aim section.** Existing tests find the coordinates with
  `/Aim point:.*700.*420/` (`crates/finder/web/src/components/AimTap.test.tsx:88`) and
  the reset with `/Aim point: frame centre/`
  (`crates/finder/web/src/components/AimTap.test.tsx:112`); the new paragraph matches
  neither.
- **The chart without a measured aim point**: no new element is drawn, so every chart
  test sees the nodes it saw (`crates/finder/web/src/components/ChartLive.test.tsx:46`).
- **`useSession`'s other fields** (`crates/finder/web/src/useSession.ts:29`): one line
  is added.
- **The MCP tool list as tests read it.** One test calls every listed tool with `{}`
  and checks for the token (`crates/finder/tests/mcp/motion.rs:528`); the new tool
  answers `{}` at once on a console with no imaging (US7-S4).
- **The binary's start-up lines and its existing flags**
  (`crates/finder/tests/cli/flags.rs:13`): nothing new is printed, and no existing flag
  changes.
- **Finder's test layout** (`crates/finder/tests/layout/main.rs:109`): no file at the
  top level of `crates/finder/tests`, no new file in `support`, and each binary's
  `main.rs` holding only its support include and `mod` lines.
- **031's own tests, once they exist.** They compare `settings` and `frame` key by key
  and `camera` and `status` whole; this spec adds no key to `camera` or `status`. Its
  MCP test counts nine keys in `set_imaging_camera`'s schema, which is why the focal
  length is not a setting (spec owner decision 6). Its Imaging page tests are re-read
  before the flip for a query that an extra button or region would break
  (§ Prerequisites 2).
- **Shapes this spec pins for its own later stories and for 033 to 037.** `SolveView`
  and `RelationView` are `#[non_exhaustive]` and tests compare them key by key.
  `SolveFrame`, `SolveHint`, `ImagingSolve`, `FinderFix`, `AimReading` and `HintInputs`
  are built by literal in this spec's tests, so each has all its fields from the story
  that creates it (`HintInputs::skip` from US3), and a later story or spec that adds
  a field to one edits those tests and must plan it. A test implements `ImagingSolver` by hand (US2-S4), so a new
  method on the trait needs a default. `Relation`'s methods `new`, `describe`,
  `set_focal_length_mm`, `store_scale`, `miss`, `height_inputs` and `hint_skip` are
  called by a test (US3-S7, US3-S8) and keep their signatures, and so does
  `Imaging::note_solve` (US4-S7). A solve sidecar once written is pinned too:
  a key added later appears only in later files.
- **What 033 to 037 are given, so that none of them edits this spec's files to get
  it.** `solving::solve_frame` with `Imaging::hint` solves any saved frame and writes
  its solve sidecar without touching the view or the relation: 035 solves every sub
  with them, and offers each accepted sub's solve to `Imaging::note_solve` for an aim
  reading. `Imaging::solver()` hands 033 the port for `measure`, and
  `Imaging::scale_arcsec_per_px()` the scale that turns `half_flux_px` into
  arcseconds. `Imaging::solve` stays the latest frame's.

## Decisions and the alternatives rejected

| Decision | Rejected | Why |
|---|---|---|
| A port with two methods and boxed futures | An enum of solvers; a synchronous trait | The plan document names the port; `live::Solver` already has this shape; ASTAP blocks for seconds and the simulated adapter waits on virtual time. |
| ASTAP run with `std::process` on the blocking pool, polled every 20 ms against a real-time deadline | `tokio::process` | `finder`'s tokio lacks the `process` feature and no `Cargo.toml` may change. A real child and virtual time do not mix anyway (§ Plan traps 7). |
| A prepared copy in a work folder, solved with `-z 1` | The saved file with `-z 2` or `-z 0` | The saved imaging frame is never opened by another program; the output files land in the work folder; and the answer's pixels are known exactly, where ASTAP's own downsampling leaves it unsaid which pixels its `CD` matrix counts (spec owner decision 4). |
| Mosaic cells averaged into one sample | Extracting green; `-check y` | ASTAP's page: "DSLR/OSC raw images require 2x2 binning" (`docs/research/2026-11-15-imaging-camera.md` § 3. ASTAP), and one mean per cell is the same step 031's preview takes. |
| The database always named with `-D` | ASTAP's automatic choice | The worked example's height, 0.4997°, sits on the line where that choice changes. |
| The angle, scale and mirror sense from the `CD` matrix | `CROTA` and `CDELT` | Two angles and two scales describe a skewed matrix ambiguously; the matrix is what 035 needs, and one formula serves both adapters. |
| A mirrored solve is reported | Refusing it | A Newtonian's two mirrors do not mirror, a star diagonal does, and neither is an error (spec.md § Edge Cases); the plan document's `ImagingSolve` says whether the frame is mirrored. |
| `stars` optional | Reading ASTAP's printed text | The `.ini` is the documented interface and carries no count (spec owner decision 5). |
| A hint with no place is sent as RA 0, Dec 0, radius 180° | Leaving `-ra` and `-spd` out | Without them ASTAP takes a start from the file's header or its own last solve; a stated start is the same every time. |
| The simulated adapter answers from the simulator's sky, with a 30-star rule, a radius rule and a 5% field rule | Solving the rendered pixels; always succeeding; a 10% field rule | The simulator's seeded stars are not a real sky, so no solver could match them; the three rules are what make the hint's stories testable, and 30 stars and 5% are the figures on ASTAP's own page. |
| A search of the whole sky gets its own, longer limit, 600 s | One limit of 120 s for every solve | ASTAP's published timings for a start 90° away, scaled to a field half a degree high, run to minutes; under 120 s a solve with no hint would nearly always be killed (spec owner decision 12). |
| One public `solve_frame` that solves and writes the solve sidecar, with `Imaging::solve` built on it | The solve and the sidecar inside `Imaging::solve` only | 035 solves every sub, not the latest frame; without this it would edit `solving.rs`. |
| The route solves the latest saved frame and answers at once | A body with an id; a request that waits | Ids repeat across dates in 031's layout; the page already reads the view once a second; the MCP tool waits instead (spec owner decision 13). |
| A sidecar for every ended solve, replaced on a new one | Only for solved; a list of attempts | The plan document: a solve is a sidecar; a failure's reason is worth keeping; the frame is never rewritten. |
| `relation.json` in the imaging folder for the scale and the aim readings; the aim point itself in the session | Everything in `session.json`; everything in the imaging folder | The aim point is the session's and every pointing reads it there; the scale and the window belong to a camera and move with `--imaging-dir`. |
| The scale kept per sensor pixel | Per delivered sample | One number serves every binning and region. |
| The focal length as a flag | An imaging setting | It matters once, and a tenth settings key breaks 031's schema count (spec owner decision 6). |
| A history of live solves built by pairing each report with the session's pointing | A field on `LiveReport`; a second solve of the star frame | The report gains no field, so nothing that builds or reads one changes; the session already holds the projection for the newest solve. |
| The aim reading taken from a least-squares line through every finder fix from 10 s before the exposure to 10 s after it, read at the exposure's middle | The nearest live solve alone; the line between two neighbouring live solves | Untracked, the sky crosses a quarter of a finder pixel between two star frames, so one fix alone is out by that. Two neighbours half a second apart differ by 25″ to 40″ of the finder camera's own scatter, which a 2 s exposure turns into 100″ of apparent travel: nearly every solve at the telescope would be refused as moving. The fit divides the scatter by `√D` instead (US4-S2). |
| Stillness judged from that line (travel beyond its standard error, and scatter about it), in arcseconds | Axis counts; the mount's `moving` flag | The mount-neutral rules: imaging code reads no axis and no mount. The scatter test catches a bump or a slew's tail, which a travel test alone would average away. |
| The field height's sources taken in turn after three misses each: the kept scale, the focal length, none | A kept scale used for ever; a second try of the same solve with no height; clearing the scale when the focal length disagrees | The relation is keyed by camera, and the scale belongs to camera and telescope together: after a change of optics a scale used for ever fails every solve, with no cure but deleting a file. A second try doubles every failed sub's wait. Turning through the sources cannot lock: a true scale comes round again (spec owner decision 22). |
| The hint's sources tried in order across solves: the first that exists, one place down after each three misses in a row, held at `none`; the last imaging solve dropped when the target changes | A ranking only; one solve per source until one succeeds; turning from `none` back to the first; an age limit on the last imaging solve | A ranking alone lets a hint that exists but is wrong (a finder camera over 3° off the main axis, a stale measured aim point) fail every solve and never reach the whole sky. A ladder within one sub multiplies the wait of every failed sub by up to four. `none` cannot lock solving out, and a solved solve starts again from the first. An age says nothing about a telescope that was sent elsewhere a moment ago; the target is what changes then (spec owner decision 23). |
| A measured aim point held back while Track is on and applied at the first live solve after Track is off | 036's tracker bounding the rate one error may ask for; Track restarting through GoTo on a changed aim point; accepting the chase and bounding it by a probe | 015's tracker turns the whole step into a rate nothing bounds, from this spec's landing; only a hold closes that here, with no line of the tracker changed. 036 lands four specs later. A restart through GoTo needs the GoTo inside the session. A probe measures a hazard and does not remove it (spec owner decision 25). |
| The hold kept by the session, applied inside `record_live_pointing` | The hold kept by the relation and applied by the history's task | The session already knows Track's state and the tap; applying it there, before the live pointing is taken, makes that pointing the first at the new aim point, which a re-centre of 037 reads. |
| `Imaging::note_solve` for a solve made outside the route | Aim readings from `Imaging::solve` alone; `solve_frame` taking readings itself | The plan document has the console measure whenever an imaging solve and a live pointing share a moment; `solve_frame` promises 035 it touches neither the view nor the relation, and stays so. |
| The history bounded by time and thinned to `keep_s / HISTORY_MAX` | A queue of 2000 fixes | A queue at two fixes a second holds 1000 s, under the three exposures the fit spans once subs pass 330 s; the plan document's rule 6 forbids a fixed queue length. |
| From US4, the `pointing` hint is the finder fix nearest the exposure's middle | The session's latest pointing at the moment the solve is asked for | A client that takes a frame, moves the telescope and then solves would hint the new place; the history already holds the fix of the exposure's own moment (spec FR-035). |
| The narrow radius is the sensor's diagonal | The frame's own diagonal | The narrow hint's place is the sensor's centre; a frame of a corner region lies farther from it than its own diagonal (spec US5-S2). |
| One in-memory `Relation` in the imaging state from US3 on, changed only in `relation.rs` | The scale, the history, the reading held and the window as separate fields of the state in `mod.rs` | US5 replaces the one aim reading with the window and does not write `mod.rs`; a field there would make it. |
| "Aim reading" for a reading of the aim point | "Measure" | The solver port already has a star measure (`measure`, `StarMeasure`, `MEASURE_LIMIT`); one word for two things in one module tree misleads a reader and a builder. |
| The imaging angle against the finder frame's up axis, not against its roll | Subtracting the finder solve's roll | North turns across a 12° frame; the up axis seen at the aim point is what the simulator's truth is defined by, and what a second telescope would need too. |
| `source` only on a measured aim point | A `source` on every aim point; a sibling key | Six existing assertions pin a tapped aim point's two keys (spec owner decision 1). |
| A second method, `set_measured_aim` | A flag argument on `set_aim` | Eleven test files call `set_aim` as it is. |
| The median of five, three set aside replace | A mean; a filter with a gain | A median ignores one bad solve outright; replacement after three is how a turned camera takes over without a control. |
| A 10″ step before the session's aim point moves | Setting it on every aim reading | Every change moves the reference later syncs and pointings are taken at; under the finder camera's own scatter that is noise in the pointing model. |
| Nothing in the pointing model reset when the aim point moves | Dropping the night's syncs | A tap resets nothing today, and the model's slip rule opens a new epoch for a large move (spec owner decision 3). |
| The chart's outline computed on the page from four numbers | Corners computed by the console | The page already has the finder camera's scale with each live report; the console sends what it measured. |
| No Playwright check | One in the simulate project | It would leave the shared simulated console with a measured aim point, which other checks require absent (spec owner decision 16). |

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

  Every story runs both, and the judge sees their results. `--locked` and no manifest
  change mean `Cargo.lock` is untouched.
- **CI in the merge queue** runs every `ci.yml` job, including `cargo-deny`,
  `cargo-machete`, the build on Rust 1.96 and the finder-page checks. No story adds a
  dependency, so neither tool has anything new to judge.

## Prerequisites before this spec is flipped to `ready`

1. **030-imaging-camera-link and 031-imaging-frames-on-console have landed.** Not met:
   both are drafts in this worktree. Everything of theirs named here is from their
   plan.md as drafted.
2. **The trio is re-anchored to `main` after 030 and 031 land.** Not met. Every `path:line` was
   read at dbc2b5b and re-checked at 49bb210 (028's US5), the worktree's HEAD when the
   audit was applied; `main` does not yet hold 030 or 031. To do once they land: re-read every
   cited line of `crates/finder/src/api.rs`,
   `crates/finder/src/main.rs`, `crates/finder/src/lib.rs`, `crates/finder/README.md`,
   `DESIGN.md`, `crates/finder/web/src/api.ts` and the test files 028 and 031 change;
   give a line to every file of 030 and 031 named without one (`Imaging`, `ImagingView`,
   `FrameView`, `Refusal`, `fits::decode`, the routes, `ImagingPage.tsx`,
   `take_imaging_frame`); check that `FrameView` carries `roi`,
   `binning`, `start` and `exposure_ms` for a saved frame, that `Imaging`'s state can
   take the fields § The imaging state names, that 031's task has one place where the
   camera's description is kept on reaching `ready`, that `ImagingPage.tsx` renders
   its Imaging frame block as a region named `Imaging frame` (spec US6-S4), and that
   031's Imaging page tests hold with one more region and button; then re-run the
   validator on `main`.

   Ten of the citations moved when `main` went from dbc2b5b to 49bb210 while this trio
   was being drafted, because that commit rewrote the tests they point into; the
   validator refused three of them there, as past the end of their file. The audit
   re-anchored all ten, and every other `path:line` was read again at 49bb210 and
   cites the same text it did at dbc2b5b. `Scratch` is now cited with its line
   (`crates/finder/tests/support/mod.rs:12`). For the record, the ten:

   | File | Cited line at dbc2b5b | Cited now, at 49bb210 |
   |---|---|---|
   | `crates/finder/tests/console/session/durable.rs` | 59 | 47 |
   | `crates/finder/tests/console/session/durable.rs` | 73 | 61 |
   | `crates/finder/tests/cli/aim.rs` | 208 | 198 |
   | `crates/finder/tests/cli/aim.rs` | 219 | 209 |
   | `crates/finder/tests/cli/aim.rs` | 227 | 217 |
   | `crates/finder/tests/cli/session.rs` | 145 | 123 |
   | `crates/finder/tests/console/recording.rs` | 445 | 426 |
   | `crates/finder/tests/console/recording.rs` | 632 | 613 |
   | `crates/finder/tests/mcp/motion.rs` | 532 | 528 |
   | `crates/scope-node/tests/rpicam.rs` | 129 | 140 |
3. **The owner decisions are settled and the approvals given** (spec § Owner decisions,
   § Owner approvals). Not met: all twenty-five decisions are open.
4. **Constitution V is read or amended by the owner.** Met: the owner amended it in
   constitution 1.3.0 (PR 239, merged 2026-11-17 as `cc593c7`). Principle V now reads "Proven open source first, with honest
   licensing": the plate solver stays our own Rust, and a separate program called
   behind one of our ports, never linked, is one of the ways reused software comes in.
5. **The saved captures are recorded and staged.** Not met: the grilling session could
   not run ASTAP. The owner runs § Capturing ASTAP's output and commits
   `fixtures/finder/astap/`. US1 cannot pass without it, and no story may write it. If
   the captures show ASTAP behaving otherwise than spec.md says, the trio is corrected
   first.
6. **The trio is probe-built.** Not met. No story has been compiled. Before the flip
   each of the seven is built from this trio in a throwaway worktree on `main` with 030
   and 031 landed, tests first, with both gates; each diff is measured as plan trap 3 says; and
   what the probes find is folded back in. US1, US2 and US4 are built first: US1
   because it runs a child process from a test; US2 because US2-S2 and every later
   scenario rest on how many seeded stars the simulator puts in a frame, which the
   probe counts (the scenario's own guard asks for 40 in the top half of the sensor
   and the simulated adapter for 30); US4 because its tolerances, and the 60″ and 90″
   of the stillness test, are derived and not measured, and because US4-S8 is the
   trio's one test that moves a mount, through a GoTo of some 30° and Track (§ Risks).
7. **The documents this trio cites are on `main`.** Not met: the plan document,
   ADR-0002, the changed `GLOSSARY.md` and the research note are in the drafting
   worktree, uncommitted. There `GLOSSARY.md` already carries the six entries spec
   owner approval 2 words: Finder fix, Star measure, Aim reading, Relation, Solve hint
   and Solve sidecar.
8. **Outside the factory, before the first use at the telescope:** ASTAP and a star
   database installed on the console machine, the console started with the three flags,
   and one solve timed. Not needed to flip.
9. **Other drafts that write the same files are ordered.** Not met. 031's
   Prerequisites name `026-mount-sky`, `021-chart-orientation` and `022-altaz-guidance`
   as drafts on branches; of this spec's files they write at least
   `crates/finder/src/api.rs`, `crates/finder/src/session.rs`,
   `crates/finder/web/src/components/Chart.tsx`, `DESIGN.md` and
   `crates/finder/web/dist`. Whichever of two such specs is flipped second names the
   other in `depends_on_landed` and is re-anchored after it lands. Within this series
   every later trio, 033 to 037, already names this one in `depends_on_landed`.
10. **The plan document agrees with the settled decisions.** Not met on `main`, which
    does not hold the plan document yet (§ Prerequisites 7). In the drafting worktree
    it already agrees with the recommended answers: the eight departures spec.md's
    Clarifications list are written into it, in its § Solver port (the optional star
    count, the hint's source, the linear part, the focal length as a flag, the
    prepared copy of the saved FITS file in a work folder), its § Console (the five
    file names, and `relation.json` and the `solver/` work folder in the storage
    layout) and its § Simulator (no change for the solver's truth). If the owner
    settles owner decision 4, 5, 6 or 20, or approval 7 or 8, otherwise, those three
    sections of the plan document are corrected in the same change, so that 033 to 037
    read what 032 builds.
11. **Owner decision 25's answer is written into the trio it names.** Met on
    2026-11-17. The owner chose the held aim point, this trio's own recommended answer,
    already written here (spec FR-036, US4-S8, § The session). 037's owner decision 26
    cites spec owner decision 25, and 037's re-centre is written for the answer: the
    aim point a run's first imaging solve measures is held while Track is on and
    applied at the first live solve after the run's own Track off, so the run tests
    whether it moved with `Session::held_aim` as well as the session's `aim`, and waits
    for that live solve after its Track off before it asks Track again (037's FR-022,
    FR-025). No line of 036 changes.

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
   test of the ASTAP adapter runs the stub program or reads the saved captures; no task
   runs `astap_cli`, `cargo deny` or `cargo machete`.
5. **A spec fixed mid-epic needs a relaunch.** *Check:* no builder edits
   `specs/032-imaging-solve-and-aim-point/` or `.specify/`.
6. **An oracle copied from the code proves nothing.** *Check:* tests read the truth from
   the simulator themselves, compute separations and position angles with their own
   formulas, parse sidecars and `relation.json` with `serde_json`, read FITS files with
   their own reader, and take worked answers' expected numbers from spec.md. US1-S7's
   star places come from a catalogue.
7. **A real child process and paused time do not mix.** With time paused, tokio jumps
   to the next timer whenever every task is idle, so a virtual deadline passes while a
   real program still runs. *Check:* the adapter waits on the blocking pool against
   `std::time::Instant`; US1's tests are plain `#[tokio::test]`; no console-level test
   uses the ASTAP adapter.
8. **Hours, not degrees.** `-ra` is in hours and `-spd` is declination + 90. *Check:*
   US1-S2 asserts `5.588140` and `84.60890` for RA 83.8221°, Dec −5.3911°.
9. **The height, not the width.** `-fov` is the picture's height. *Check:* FR-016 uses
   rows; US3-S2 asserts 822 rows' worth.
10. **Rows from the top.** FITS readers assume the first row is the bottom; these files
    put the top first. *Check:* the angle is `atan2(−cd[0][1], −cd[1][1])` and an
    unmirrored sky has a positive determinant; worked answer A, read that way, is
    mirrored at 181.17°, and worked answer B is not, at 30°; US1-S7's three stars fix
    the sense on a real picture.
11. **A child that outlives its call.** *Check:* past the deadline the adapter kills and
    reaps; US1-S5 asserts the process is gone and the work folder empty.
12. **A pipe that fills.** *Check:* the program's output goes to files in the work
    folder, read after it ends.
13. **A test that pins what a later story changes.** US3 changes the hint US2 sends, US4
    sets an aim point where US3's tests had none, and US5 narrows a radius. *Check:*
    US2's tests assert nothing about `solve.hint` or `relation`; US3 asserts a radius
    only for a console's first solve and for `last_solve`, `target` and `none`; no test
    of US2 or US3 asserts that the session has no `aim` after a solve. US4 changes
    where the `pointing` hint comes from, and US3's scenarios hold either way: where
    they expect `pointing` the finder camera is solving at the exposure's own moment,
    and where they expect another source its last solve is 120 s old, past the 30 s
    both rules allow.
14. **`Aim` built by literal, and a tapped aim point compared whole.** *Check:* `Aim`
    gains no field; `source` is skipped unless measured; US4-S6 asserts the two-key
    form.
15. **A stored key that is always written.** *Check:* `aim_measured` is skipped while
    false; `crates/finder/tests/cli/session.rs:123` still counts three keys.
16. **Whole-object comparisons break at the next story.** *Check:* tests compare
    `solve`, `relation` and `aim` key by key.
17. **The live broadcast lags.** It is 16 deep and a slow reader loses reports. *Check:*
    the history's task carries on after `Lagged`, and a moment with no fix takes no
    aim reading instead of a wrong one (US4-S4).
18. **An angle's median across 0.** *Check:* differences are reduced before the median;
    US5-S1 asserts 0.0 for angles on both sides of north.
19. **The first aim reading after a tap.** A window that kept its old readings would
    out-vote the new one. *Check:* the window is cleared when the session's aim point is
    not the measured one; US4-S5 asserts `readings` 0 after a tap.
20. **Alt-az arithmetic or axis counts in imaging code.** *Check:* SC-005; stillness is
    projections of one sky direction through the finder fixes and a straight line
    through them.
21. **A constant sized to one telescope.** *Check:* every threshold of § The relation is
    in arcseconds on the sky or degrees of angle, converted with the finder solve's own
    scale; the narrow radius is the sensor's diagonal at the kept scale; the fit's reach
    and the history's length grow with the exposure.
22. **`useSession` copies known fields only.** *Check:* US6 writes
    `crates/finder/web/src/useSession.ts`, and US6-S7 drives `SetupPage` through it.
23. **The bundle must be rebuilt.** *Check:* US6 commits `crates/finder/web/dist` after
    `npm run build`.
24. **A tool that hangs on `{}`.** *Check:* without imaging, and without a solver,
    `solve_imaging_frame` answers at once (US7-S4).
25. **027's layout.** *Check:* each new finder test is a module file in its binary's
    folder with one alphabetical `mod` line, binding helpers with
    `use crate::support::live as support;` and holding only the setup its tests use.
26. **Scratch files go where cargo says.** *Check:* every file a test makes, the stub
    program included, is inside a `crate::support::Scratch`.
27. **The shared simulated console.** *Check:* no Playwright test is added, and nothing
    solves on a console that is not asked to.
28. **A scenario that rides on a random count.** The simulator's stars are a seeded
    Poisson field of 320 a square degree, and the simulated adapter refuses a frame
    with fewer than 30: a region that holds 30 on average solves or not by the seed.
    *Check:* every frame a scenario expects to solve is a whole frame (120 stars on
    average) or the top half of the sensor (60), US2-S2 counts the stars itself and
    asks for 40 before it solves the half, and the one region expected to fail holds
    1.4. No test solves a quarter of the sensor; the corner region of US5-S2 goes to
    `hint_for` alone.
29. **Noise multiplied by a short baseline.** A rate taken from two fixes one frame
    apart is the finder camera's scatter divided by half a second. *Check:* `read_aim`
    fits every fix in its span, needs three, and subtracts the travel's standard error
    before it calls the telescope moving; US4-S2's still telescope with fixes 40″
    apart must be `Measured`.
30. **A lock held across a call into the session.** `Imaging::view` reads the session's
    aim, and the session's own view is built beside the imaging view. *Check:* the
    imaging state's lock is released before `session.snapshot` or
    `Session::set_measured_aim` is called, and `Imaging::view` reads the session's aim
    before it takes its own lock; no session method calls into `Imaging`.
31. **A kept value that can lock itself in.** A wrong kept scale fails every solve, and
    a failed solve never corrects it. *Check:* the field height's sources are taken in
    turn after three misses (US3-S7), and nothing else this spec keeps is used without
    a way back: the aim readings are replaced after three outliers (US5-S3) and
    emptied by a tap or a reset (US4-S5).
32. **Too many arguments.** `clippy -D warnings` refuses a function of more than seven.
    *Check:* `solve_frame` takes its frame, hint and ids as one `Ask`.
33. **An aim point that moves under a running tracker.** 015's tracker turns a step in
    the aim point into a rate nothing bounds. *Check:* `set_measured_aim` holds the aim
    point while `TrackRun::track_on`; only `record_live_pointing` with Track off and
    `set_aim` touch `held_aim`; US4-S8 reads the simulator's true direction for 5 s
    and asserts under 60″; no line of `crates/finder/src/tracking.rs` or of Track's
    routes is in US4's diff.
34. **A hint that exists but is wrong.** A ranking alone would repeat one failing hint
    for ever. *Check:* `hint_for` takes the source `min(skip, n − 1)` places down, and
    `skip` is `Relation::hint_skip()`, the same misses that turn the field height;
    US3-S8 reaches `target` at the fourth solve and `none` at the seventh.
35. **A queue length in the history.** *Check:* `FinderHistory::push` drops a fix only
    for its age and thins new ones by spacing; US4-S2's 3001 fixes keep the whole
    1200 s.
36. **An aim reading only the route takes.** *Check:* the task's step 2 and
    `note_solve` are one private function; US4-S7 drives `note_solve` with a solve the
    test made and asserts that the view's `solve` and the kept scale did not change.

## Risks

| Risk | Mitigation |
|---|---|
| Nothing here has been compiled, and 030 and 031, which it builds on, have not either. An interface may not build as written: `impl Imaging` blocks in child modules against state 031 has not yet fixed; `SessionView::aim` changing type under a `json!` in `recording.rs`; clap's `requires` on an `Option<PathBuf>`; and what the review added and nobody has built either: the `Relation` held in 031's state and reached from its task, `solve_frame` borrowing a solver and a clock across an await, and the history's task pushing into that state. | Prerequisites 2 and 6: re-anchor after 030 and 031 land, then probe-build all seven. |
| The simulator's seeded field puts fewer stars in a frame than a scenario needs: the count is a Poisson draw on a fixed seed, and no story may change the seed, the density or the scenario. | Every frame expected to solve averages 60 stars or more against the 30 needed, which leaves a chance under one in 100,000 for a half frame; US2-S2 counts first and asks for 40 (a chance of 1 in 400 of falling short), so a shortfall shows as that guard and not as a wrong outcome. The US2 probe build reads the count; if the guard fails there, the region is changed before the flip (§ Plan traps 28). |
| ASTAP behaves otherwise than its page says: an `.ini` is or is not written when the database is missing; `-analyse` exits with a code that carries its result; the `CD` matrix counts rows from the bottom whatever the file says. | Prerequisite 5. US1-S7 replays what ASTAP really wrote; `measure` already trusts the two printed lines over the exit code; three catalogue stars fix the row order. The trio is corrected to the captures before the flip. |
| The tolerances of US4 and US5 (1.5 finder pixels, 0.25°) are derived from the 30″ that existing tests allow a live pointing, not measured. | The first probe build of US4 measures the scatter on the simulator over fifty solves. A tolerance is then kept or the scenario corrected before the flip; none is widened by a story. |
| A story passes 64 KiB. The likeliest are US1 (the port, the adapter, the prepared copy and seven scenarios), US2 (twelve files, a README section and six scenarios with some thirty assertions: an estimate of 55 to 65 KiB) and US4 (the history, the aim reading, `note_solve`, the session and its held aim point, the sky, eight scenarios and a second test module with its own rig). Each of US2 to US5 and US7 also writes the solving rig out again, 4 to 5 KiB a time (§ Technical Context, Testing). Estimates; nothing is measured. | Measured on the probes. Splits written in advance: US1 sheds `measure` and `prepare`'s regions (US1-S6, FR-007) into a story of its own after it; US2 sheds the simulated adapter (`simulated.rs`, FR-008, US2-S2, in a test module of its own, `imaging_simulated`) into a story before the route, which then needs no rig at all; US4 sheds the session's `source` and its persistence (US4-S5's second half, US4-S6) into a story before it, and the held aim point (FR-036, US4-S8, the `motion` module) into a story after it. Each split is a spec change made before the flip. |
| The stillness test's figures (60″ of travel, 90″ of scatter, three fixes, 10 s of reach) are set against one field figure for the finder camera's scatter, 0.3′ to 0.5′, and against a simulator whose finder is quieter. Too tight, and no aim reading is ever taken at the telescope; too loose, and a slew's tail is read as a reading. | US4-S2 pins the arithmetic on worked examples, with a still telescope whose fixes are 40″ apart. The first probe build of US4 measures the simulator's scatter about the line. The field check "the share of solves that take an aim reading while tracking" is the one that settles the figures; they are four constants in `relation.rs` and one owner decision (11). Until that check, an observer who gets no aim reading still has the tap. |
| A solve with no hint, or one given no field height after three misses, runs ASTAP's slowest search and may be killed at its limit. | The whole-sky limit is 600 s and an owner decision (12); the field check times it. The sources are taken in turn, so a true kept scale comes back after at most six such solves. A target or a finder camera avoids the case. |
| ASTAP's `.ini` carries a `//` note after each value, as its page prints the example, and `read_answer` then reads no number. | The saved captures show the file as it is (Prerequisite 5); if the notes are there, `read_answer` cuts each value at ` //` and worked answer A is corrected, before the flip. |
| ASTAP does not solve real frames from this telescope: too few stars in a short sub at a sparse field, stars swollen by coma, a frame binned 2 × 2 and averaged again to 2.2″ a pixel, bright nebulosity. | Field checks, named in spec.md. The port keeps another adapter possible; the database is a flag. Not answerable in the factory. |
| A hinted solve takes longer than the sub cadence 035 wants. | The first solves at bring-up give it, read from `took_ms` (Prerequisite 8); `-s`, a smaller radius and the database are the levers, each one line here; 035 owns the cadence. |
| The aim point shifts with altitude as the tube flexes, so a median of five lags after a long slew. | The window turns over in five solves and a jump of more than 120″ replaces it in three. A model of the shift is out of scope; the field check "steady from one part of the sky to another" measures it. |
| A wrong solve that ASTAP reports as solved sets a wrong aim point as the first aim reading. | With one or two aim readings held nothing is an outlier, so the next two outvote it: the median of three is right. Until then a GoTo may centre on the wrong spot by the solve's error; the observer can tap or reset. |
| The 10″ step leaves the session's aim point up to 10″ from the median. | That is a third of a finder pixel, under the finder camera's own scatter; spec owner decision 10's alternative removes it. |
| A measured aim point changes what every later sync is taken at, and the pointing model mixes syncs from before and after. | Spec owner decision 3: the same is true of a tap today; the closed GoTo loop corrects on the live pointing; a move over 0.5° opens a new epoch. |
| The new button and region on the Imaging page break a query in 031's page tests. | Prerequisite 2 re-reads them; the block shows no `status` or `alert` until a solve is asked for. |
| `021-chart-orientation`, a draft, also writes `Chart.tsx`. | Prerequisite 9. |
| A work folder is left behind by a console that exits mid-solve. | It holds one prepared copy; folders are named by process and count, so a later run never reuses one. The README says they may be deleted. |
| A measured aim point held for the whole of a long stack (Track on for hours) leaves GoTo and Track on the old aim point, which may be a degree from the sensor's centre before the first measurement. | The target still lands inside the 3° the first hint searches; the page says the aim point waits; the first Track off applies it, and 037's re-centre turns Track off and on for that reason. Owner decision 25, settled on 2026-11-17, chose this. |
| 037's centring rests on the hold: it reads `Session::held_aim`, and waits after its own Track off for the first live solve, which applies the held aim point. | Prerequisite 11: 037 is written for the settled answer (037's FR-022, FR-025). A change to FR-036 or to `held_aim` is re-read in 037 before either trio is flipped. |
| US4-S8 needs a GoTo of some 30° and Track to settle on the simulator in virtual time, and its 60″ is set against the tracker's deadband, not measured. | The rig is the one 015's own Track tests run (`crates/finder/tests/motion/tracking/routes.rs:85`); the first probe build of US4 times it and reads the travel without the hold, to show the scenario fails on the first draft. |
| A hint turned to `none` after misses runs ASTAP's slowest search on every later failed sub until one solves. | Misses are only `not_solved` and `timed_out`, not `too_few_stars`, which a cloudy sub gives; the whole-sky limit is 600 s; a solved solve starts again from the first source. |

## Refinement Record

Drafted on 2026-11-15 by the grilling session against `main` at dbc2b5b, from
`docs/plans/2026-11-15-specs-030-037-imaging.md`, 030's and 031's draft trios (one trio then) in the same worktree,
the surveys of the console and the solver made that morning, the web research on
narrow-field solving with its verification, and ASTAP's own page read that day. **No
probe build has been run.** No story of this trio has been compiled, linted or tested,
no interface above has been built, and no diff has been measured: the session wrote
documents only. ASTAP has not been run, and the program of § Capturing ASTAP's output
has not been run either. Every `path:line` was read at dbc2b5b and is now checked, by
hand and by the validator, at 49bb210; every number in a scenario was worked by hand or
with a calculator and is shown as a worked example where it is not obvious.

A second agent reviewed the draft the same day, reading the trio against the plan
document, the code at dbc2b5b and the research, and reworking its arithmetic. It built
nothing either. This revision applies its seventeen findings: the stillness test and
the aim reading are now a fitted line and not the difference of two finder fixes; the
field height's sources are taken in turn, so a kept scale cannot lock solving out; one
`Relation` in the imaging state is named and typed; US2-S2 no longer rides on a count
of thirty seeded stars; and the smaller ones (§ Decisions, § Plan traps 28 to 32,
§ Risks). The new scenarios' numbers were worked with a pinhole projection written for
the purpose, outside the repository, not with any code of this project. One finding
was taken in part then: the citations stayed at dbc2b5b.

An auditor then compared the seven trios the series then had, and this revision applies
its
six findings for this one, again without building anything: the ten citations that
49bb210 moved are re-anchored and every other one re-read there; the hint's place now
turns one source down after three misses, as the field height does (US3-S8);
`Imaging::note_solve` lets 035 offer a sub's solve for an aim reading (US4-S7); a
measured aim point is held while Track is on, owner decision 25 moving here from 037
(US4-S8, the trio's one test in `motion`); the history is bounded by time and thinned;
and the adapter's work-folder error and § Prerequisites 9 are corrected. The numbers
of US3-S8 and US4-S2's history were worked by hand; US4-S8's 60″ is the bound existing
tracking tests hold, not a measurement.

On 2026-11-17 the owner settled owner decision 25 with the hold this trio recommended,
so no requirement, scenario or interface changed; the decision, § Prerequisites 11 and
§ Risks now say it is settled (spec.md § Clarifications, the last session).

What a first refinement round must do is § Prerequisites 2, 5 and 6, and its findings
replace this paragraph. Until then the record below is the draft's own account: its
tests are the scenarios, its writes are the files § Project Structure names, and its
shared writes name the merge edge that orders two stories of this spec on one file, or
the story of 031 that last writes the file. A file last written by an older landed spec
(`crates/finder/src/session.rs`, `crates/finder/src/sky.rs`,
`crates/finder/web/src/components/Chart.tsx`,
`crates/finder/web/src/components/AimTap.tsx`, `crates/finder/web/src/useSession.ts`)
has no entry, because the validator refuses a shared write that names a spec outside
this spec's `depends_on_landed`. An existing test is listed only when its file is in the
drafting worktree; 030's and 031's tests are named in § Shapes that existing tests pin
instead.
The record's `open_questions` are empty because the validator refuses a record that
holds any; what is not established is in § Risks and § Prerequisites. Nothing in this
record is an owner approval: those are in spec.md, and flipping to `ready` gives them.

```yaml
version: 1
reviewed_revision: 49bb210fe4860dd98ba24727074e7bd3cc0d7124
stories:
  US1:
    tests:
      - path: crates/finder/tests/console/imaging_astap.rs
        status: new
        disposition: add
        assertion: "ASTAP's documented .ini reads as a centre, an up angle of 181.16684 degrees, 2.69358 arcseconds a pixel and mirrored for a frame whose rows run from the top"
        decision: "spec.md:US1-S1"
      - path: crates/finder/tests/console/imaging_astap.rs
        status: new
        disposition: add
        assertion: "the adapter gives the stub program a 618 x 411 cell-averaged copy and exactly eighteen arguments in hours and declination plus 90, and reads worked answer B as up 30 degrees, 2.16 arcseconds a pixel, not mirrored, leaving the work folder empty and the frame untouched"
        decision: "spec.md:US1-S2"
      - path: crates/finder/tests/console/imaging_astap.rs
        status: new
        disposition: add
        assertion: "solve_args and analyse_args give the exact argument lists for no position, a position near the pole, a radius over 180 and no field height"
        decision: "spec.md:US1-S3"
      - path: crates/finder/tests/console/imaging_astap.rs
        status: new
        disposition: add
        assertion: "exit codes 1, 2, 16, 32, 33 and 34, a missing or unreadable .ini and a program that cannot start each give their error, reason and message"
        decision: "spec.md:US1-S4"
      - path: crates/finder/tests/console/imaging_astap.rs
        status: new
        disposition: add
        assertion: "a program that sleeps is killed after the limit, the call answers TimedOut, the process is gone where /proc is seen to exist, the work folder is empty, and a solve with no hinted place runs to its own longer limit"
        decision: "spec.md:US1-S5"
      - path: crates/finder/tests/console/imaging_astap.rs
        status: new
        disposition: add
        assertion: "measure runs -analyse 30 on the copy of a frame or of a region moved inward to whole cells, scales the diameter by the copy's factor, and refuses no stars, unreadable output, a region outside the frame and a region with no whole cell"
        decision: "spec.md:US1-S6"
      - path: crates/finder/tests/console/imaging_astap.rs
        status: new
        disposition: add
        assertion: "the saved captures of ASTAP's real output replay as a solve that puts three catalogue stars within 1.5 pixels of where they were picked out, as not solved, too few stars and no database, and as the listed star measure"
        decision: "spec.md:US1-S7"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and each binary's main.rs holds only its support include and mod lines"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "no Cargo.toml, Cargo.lock or deny.toml changes and no unsafe is written (spec.md SC-003)"
      - "the saved imaging frame is never written to (spec.md FR-004)"
      - "the finder camera's solver and live loop are untouched"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console imaging_astap::"
        - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs, and one real solve with ASTAP on the console machine"
    writes:
      - crates/finder/src/imaging/astap.rs
      - crates/finder/src/imaging/mod.rs
      - crates/finder/src/imaging/solver.rs
      - crates/finder/tests/console/imaging_astap.rs
      - crates/finder/tests/console/main.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/imaging/mod.rs
        other_spec: 031-imaging-frames-on-console
        other_story: US6
        disposition: merge_order
        reason: "032 depends_on_landed 031, whose US6 last changed imaging/mod.rs"
      - path: crates/finder/src/imaging/mod.rs
        other_spec: 032-imaging-solve-and-aim-point
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and adds its mod lines and the view's solve"
      - path: crates/finder/tests/console/main.rs
        other_spec: 031-imaging-frames-on-console
        other_story: US6
        disposition: merge_order
        reason: "032 depends_on_landed 031, whose US6 last changed console/main.rs"
      - path: crates/finder/tests/console/main.rs
        other_spec: 032-imaging-solve-and-aim-point
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and adds mod imaging_solve"
    open_questions: []
  US2:
    tests:
      - path: crates/finder/tests/console/imaging_solve.rs
        status: new
        disposition: add
        assertion: "a saved frame solves to the simulator's truth within 0.5 arcseconds, 0.01 degrees and 0.001 arcseconds a pixel, and its sidecar holds the same numbers while the frame's file is unchanged"
        decision: "spec.md:US2-S1"
      - path: crates/finder/tests/console/imaging_solve.rs
        status: new
        disposition: add
        assertion: "the simulated adapter answers for binning and for the top half of the sensor once the test has counted at least 40 seeded stars there, refuses too few stars, a hint beyond its radius and a field height over 5 percent out, and measures stars from the point-spread constants"
        decision: "spec.md:US2-S2"
      - path: crates/finder/tests/console/imaging_solve.rs
        status: new
        disposition: add
        assertion: "the route refuses with its text before a frame, with the link down, during a frame, after an aborted one, from a foreign origin, during a solve, without a solver and without imaging, writes no sidecar, and Imaging::solver answers the solver or none"
        decision: "spec.md:US2-S3"
      - path: crates/finder/tests/console/imaging_solve.rs
        status: new
        disposition: add
        assertion: "each of the five solve errors shows its reason, message, detail, took_ms and file in the view and its reason, message and took_ms in the sidecar, and a later solved solve replaces the sidecar"
        decision: "spec.md:US2-S4"
      - path: crates/finder/tests/console/imaging_solve.rs
        status: new
        disposition: add
        assertion: "the imaging view has no solve key before a solve, and a console without imaging still answers exactly state off"
        decision: "spec.md:US2-S5"
      - path: crates/finder/tests/cli/imaging_solve.rs
        status: new
        disposition: add
        assertion: "the built finder under --simulate solves a frame with the simulated solver and writes its sidecar, --astap needs --astap-db-name and conflicts with --simulate"
        decision: "spec.md:US2-S6"
      - path: crates/finder/tests/console/session/edits.rs
        status: existing
        disposition: preserve
        assertion: "GET /api/session bodies of a console without imaging are unchanged"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/cli/flags.rs
        status: existing
        disposition: preserve
        assertion: "the binary's existing flags behave as before"
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
      - "a console with no imaging solver answers every request of 031 as before, and solve and relation are absent from its view"
      - "nothing changes under crates/simulator, crates/scope-node or crates/scope-link"
      - "the imaging frame's file is never rewritten; the sidecar is a separate file"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console imaging_solve::"
        - "cargo nextest run -p finder --locked --test cli imaging_solve::"
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
      - crates/finder/src/imaging/simulated.rs
      - crates/finder/src/imaging/solving.rs
      - crates/finder/src/lib.rs
      - crates/finder/src/main.rs
      - crates/finder/tests/cli/imaging_solve.rs
      - crates/finder/tests/cli/main.rs
      - crates/finder/tests/console/imaging_solve.rs
      - crates/finder/tests/console/main.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/README.md
        other_spec: 031-imaging-frames-on-console
        other_story: US6
        disposition: merge_order
        reason: "032 depends_on_landed 031, whose US6 last changed the README"
      - path: crates/finder/README.md
        other_spec: 032-imaging-solve-and-aim-point
        other_story: US3
        disposition: merge_order
        reason: "US3 depends_on_merged US2, and adds the hint's paragraph to US2's section"
      - path: crates/finder/src/api.rs
        other_spec: 031-imaging-frames-on-console
        other_story: US1
        disposition: merge_order
        reason: "032 depends_on_landed 031, whose US1 last changed api.rs"
      - path: crates/finder/src/api.rs
        other_spec: 032-imaging-solve-and-aim-point
        other_story: US3
        disposition: merge_order
        reason: "US3 depends_on_merged US2, and adds with_focal_length_mm"
      - path: crates/finder/src/imaging/mod.rs
        other_spec: 032-imaging-solve-and-aim-point
        other_story: US3
        disposition: merge_order
        reason: "US3 depends_on_merged US2, and adds mod relation and the view's relation"
      - path: crates/finder/src/imaging/routes.rs
        other_spec: 031-imaging-frames-on-console
        other_story: US3
        disposition: merge_order
        reason: "032 depends_on_landed 031, whose US3 last changed imaging/routes.rs"
      - path: crates/finder/src/imaging/solving.rs
        other_spec: 032-imaging-solve-and-aim-point
        other_story: US3
        disposition: merge_order
        reason: "US3 depends_on_merged US2, and adds the hint"
      - path: crates/finder/src/lib.rs
        other_spec: 031-imaging-frames-on-console
        other_story: US3
        disposition: merge_order
        reason: "032 depends_on_landed 031, whose US3 last changed lib.rs"
      - path: crates/finder/src/main.rs
        other_spec: 031-imaging-frames-on-console
        other_story: US1
        disposition: merge_order
        reason: "032 depends_on_landed 031, whose US1 last changed main.rs"
      - path: crates/finder/src/main.rs
        other_spec: 032-imaging-solve-and-aim-point
        other_story: US3
        disposition: merge_order
        reason: "US3 depends_on_merged US2, and adds --focal-length-mm"
      - path: crates/finder/tests/cli/main.rs
        other_spec: 031-imaging-frames-on-console
        other_story: US2
        disposition: merge_order
        reason: "032 depends_on_landed 031, whose US2 last changed cli/main.rs"
      - path: crates/finder/tests/cli/main.rs
        other_spec: 032-imaging-solve-and-aim-point
        other_story: US3
        disposition: merge_order
        reason: "US3 depends_on_merged US2, and adds mod imaging_hints before mod imaging_solve"
      - path: crates/finder/tests/console/main.rs
        other_spec: 032-imaging-solve-and-aim-point
        other_story: US3
        disposition: merge_order
        reason: "US3 depends_on_merged US2, and adds mod imaging_hints"
    open_questions: []
  US3:
    tests:
      - path: crates/finder/tests/console/imaging_hints.rs
        status: new
        disposition: add
        assertion: "a console's first solve is hinted by the live pointing with a radius of 3 degrees and no field height, about 1.05 degrees from the solved centre"
        decision: "spec.md:US3-S1"
      - path: crates/finder/tests/console/imaging_hints.rs
        status: new
        disposition: add
        assertion: "the solved scale is kept per sensor pixel in the view, in relation.json and in Imaging::scale_arcsec_per_px, and sizes the next solve's field at binning 1 and 2"
        decision: "spec.md:US3-S2"
      - path: crates/finder/tests/console/imaging_hints.rs
        status: new
        disposition: add
        assertion: "a focal length of 1800 mm gives a scale of 2.190991 arcseconds a pixel and a field height of 0.500276 degrees and solves, and 900 mm gives 1.000552 degrees and does not"
        decision: "spec.md:US3-S3"
      - path: crates/finder/tests/console/imaging_hints.rs
        status: new
        disposition: add
        assertion: "with no fresh live pointing the hint is the last solve, then the target, then none with a radius of 180 degrees, a target far from the frame does not solve, and a changed target outranks the last solve"
        decision: "spec.md:US3-S4"
      - path: crates/finder/tests/console/imaging_hints.rs
        status: new
        disposition: add
        assertion: "a second console over the same folder reads the kept scale, and ignores one stored for another camera"
        decision: "spec.md:US3-S5"
      - path: crates/finder/tests/cli/imaging_hints.rs
        status: new
        disposition: add
        assertion: "the built finder shows --focal-length-mm in relation.focal_length_mm and refuses a focal length of 0"
        decision: "spec.md:US3-S6"
      - path: crates/finder/tests/console/imaging_hints.rs
        status: new
        disposition: add
        assertion: "after the simulated field is widened, three solves fail on the kept scale's field height, three more on the focal length's when one is set, the next is given none and solves, its scale replaces the kept one, and Relation::height_inputs turns through its sources every three misses and starts again after a solved solve"
        decision: "spec.md:US3-S7"
      - path: crates/finder/tests/console/imaging_hints.rs
        status: new
        disposition: add
        assertion: "a pointing hint about 5 degrees from the truth fails three times, the target then fails three times, and the seventh solve searches the whole sky and solves; with no target the fourth does; Relation::hint_skip counts a place per three misses and hint_for takes that place among the sources that exist, held at none"
        decision: "spec.md:US3-S8"
      - path: crates/finder/tests/console/pointing.rs
        status: existing
        disposition: preserve
        assertion: "the latest pointing and its source are recorded as before; the hint only reads them"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and each binary's main.rs holds only its support include and mod lines"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "US2's scenarios hold unchanged: none asserts a hint or a relation"
      - "the session's pointing, target and aim point are read and never written by the hint"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console imaging_hints::"
        - "cargo nextest run -p finder --locked --test cli imaging_hints::"
        - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - crates/finder/README.md
      - crates/finder/src/api.rs
      - crates/finder/src/imaging/mod.rs
      - crates/finder/src/imaging/relation.rs
      - crates/finder/src/imaging/solving.rs
      - crates/finder/src/main.rs
      - crates/finder/tests/cli/imaging_hints.rs
      - crates/finder/tests/cli/main.rs
      - crates/finder/tests/console/imaging_hints.rs
      - crates/finder/tests/console/main.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/README.md
        other_spec: 032-imaging-solve-and-aim-point
        other_story: US5
        disposition: merge_order
        reason: "US5 depends_on_merged US3, and adds the measured aim point to the section US3 last wrote"
      - path: crates/finder/src/imaging/mod.rs
        other_spec: 032-imaging-solve-and-aim-point
        other_story: US4
        disposition: merge_order
        reason: "US4 depends_on_merged US3, and adds the history task's handle to the state and the session's aim to the view's call"
      - path: crates/finder/src/imaging/relation.rs
        other_spec: 032-imaging-solve-and-aim-point
        other_story: US4
        disposition: merge_order
        reason: "US4 depends_on_merged US3, and adds the history and the aim reading"
      - path: crates/finder/src/imaging/solving.rs
        other_spec: 032-imaging-solve-and-aim-point
        other_story: US4
        disposition: merge_order
        reason: "US4 depends_on_merged US3, and takes an aim reading after a solve and the hint's pointing from the history"
      - path: crates/finder/tests/console/main.rs
        other_spec: 032-imaging-solve-and-aim-point
        other_story: US4
        disposition: merge_order
        reason: "US4 depends_on_merged US3, and adds mod imaging_relation"
    open_questions: []
  US4:
    tests:
      - path: crates/finder/tests/console/imaging_relation.rs
        status: new
        disposition: add
        assertion: "one solve sets the session's aim point within 1.5 finder pixels of the simulator's, marked measured, with the imaging angle within 0.25 degrees of 17 and the field's size from the kept scale"
        decision: "spec.md:US4-S1"
      - path: crates/finder/tests/console/imaging_relation.rs
        status: new
        disposition: add
        assertion: "read_aim fits a line through nine finder fixes and reads it at the middle of the exposure to within 0.01 pixel for a steady drift and 0.1 pixel for a still telescope whose fixes scatter 20 arcseconds either side, answers Moving for 100 arcseconds a second of drift and for a fix half a degree off the line, NoLivePointing for two fixes or none inside the exposure, OffFrame for a centre outside the finder frame, and the history keeps 120 seconds, and thins 3001 fixes to one a second over 1200 seconds rather than cutting them"
        decision: "spec.md:US4-S2"
      - path: crates/finder/tests/console/imaging_relation.rs
        status: new
        disposition: add
        assertion: "Frame::up_east_of_north_at is 0 for no roll, 360 less the roll in degrees at the centre, and within half a degree of that at the aim point"
        decision: "spec.md:US4-S3"
      - path: crates/finder/tests/console/imaging_relation.rs
        status: new
        disposition: add
        assertion: "a solve with no live pointing in its exposure, or with 8 s of untracked drift, or that fails, sets no aim point and says why, and a solve asked for after the telescope has swung away is hinted by the live solve of the exposure's own moment and solves"
        decision: "spec.md:US4-S4"
      - path: crates/finder/tests/console/imaging_relation.rs
        status: new
        disposition: add
        assertion: "a tap on a star of a live preview replaces a measured aim point with a two-key one and leaves the relation no aim reading, the next solve measures again, a reset clears it, and session.json keeps aim_measured only while true"
        decision: "spec.md:US4-S5"
      - path: crates/finder/tests/console/imaging_relation.rs
        status: new
        disposition: add
        assertion: "a console with no imaging reads a tapped aim point as exactly x and y"
        decision: "spec.md:US4-S6"
      - path: crates/finder/tests/console/imaging_relation.rs
        status: new
        disposition: add
        assertion: "a solve the test made itself and offered to note_solve sets a measured aim point within 1.5 finder pixels of the simulator's, leaves the view's solve and the kept scale absent, and a frame of a region offered the same way changes nothing"
        decision: "spec.md:US4-S7"
      - path: crates/finder/tests/motion/imaging_track.rs
        status: new
        disposition: add
        assertion: "with Track on, an imaging solve's aim reading is held, the session keeps no aim, the main telescope's true direction moves under 60 arcseconds in the next 5 s, and Track off applies the held aim point within 5 s"
        decision: "spec.md:US4-S8"
      - path: crates/finder/tests/motion/tracking/routes.rs
        status: existing
        disposition: preserve
        assertion: "Track's routes, its GoTo hand-off and its states are unchanged"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/console/aim.rs
        status: existing
        disposition: preserve
        assertion: "the tap's snap, its refusals, its two-key aim and the reset are unchanged"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/cli/aim.rs
        status: existing
        disposition: preserve
        assertion: "the built finder's tapped aim reads as two keys and a reset leaves no aim in session.json"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/cli/session.rs
        status: existing
        disposition: preserve
        assertion: "a session with a site, a minimum altitude and a target stores exactly three keys"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/console/session/durable.rs
        status: existing
        disposition: preserve
        assertion: "set_aim with an Aim literal persists and reads back as two keys"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/mcp/session_tools.rs
        status: existing
        disposition: preserve
        assertion: "get_session over MCP shows a tapped aim as two keys"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/console/recording.rs
        status: existing
        disposition: preserve
        assertion: "recording.json's aim for a tapped aim point is the two-key form"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/console/live/loop_.rs
        status: existing
        disposition: preserve
        assertion: "live solving, its reports and the pointing at a tapped aim are unchanged with the history subscribed"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and each binary's main.rs holds only its support include and mod lines"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "Aim, Session::set_aim and both /api/aim routes keep their shapes and words (spec.md FR-022)"
      - "LiveReport gains no field and Live::start keeps its signature"
      - "no file under crates/finder/src/imaging reads an axis position or computes an altitude or azimuth (spec.md SC-005)"
      - "no line of crates/finder/src/tracking.rs, of Track's routes or of start_tracking and stop_tracking changes; a console with no imaging solver never holds an aim point (spec.md FR-036)"
      - "solving::solve_frame still touches neither the imaging view nor the relation (spec.md FR-009)"
      - "US2's and US3's scenarios hold unchanged"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console imaging_relation::"
        - "cargo nextest run -p finder --locked --test motion imaging_track::"
        - "cargo nextest run -p finder --locked --test console aim::"
        - "cargo nextest run -p finder --locked --test motion tracking::"
        - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs, and the aim point measured at the telescope against a tapped one"
    writes:
      - crates/finder/src/imaging/mod.rs
      - crates/finder/src/imaging/relation.rs
      - crates/finder/src/imaging/solving.rs
      - crates/finder/src/session.rs
      - crates/finder/src/sky.rs
      - crates/finder/tests/console/imaging_relation.rs
      - crates/finder/tests/console/main.rs
      - crates/finder/tests/motion/imaging_track.rs
      - crates/finder/tests/motion/main.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/imaging/relation.rs
        other_spec: 032-imaging-solve-and-aim-point
        other_story: US5
        disposition: merge_order
        reason: "US5 depends_on_merged US4, and replaces the one aim reading inside Relation with the window"
      - path: crates/finder/src/imaging/solving.rs
        other_spec: 032-imaging-solve-and-aim-point
        other_story: US5
        disposition: merge_order
        reason: "US5 depends_on_merged US4, and narrows the radius"
      - path: crates/finder/tests/console/main.rs
        other_spec: 032-imaging-solve-and-aim-point
        other_story: US5
        disposition: merge_order
        reason: "US5 depends_on_merged US4, and adds mod imaging_smoothing"
    open_questions: []
  US5:
    tests:
      - path: crates/finder/tests/console/imaging_smoothing.rs
        status: new
        disposition: add
        assertion: "the window's medians, its angle across north, what it sets aside at 120 arcseconds, 1 degree or the other mirror sense, replacement after three, the drop of readings from a finder frame of another size and the 10 arcsecond step are the worked values"
        decision: "spec.md:US5-S1"
      - path: crates/finder/tests/console/imaging_smoothing.rs
        status: new
        disposition: add
        assertion: "six solves hold five aim readings with the aim point within 1.5 pixels and the angle within 0.25 degrees of the simulator's, the sixth is hinted within the sensor's diagonal, and hint_for gives that diagonal for a frame of a corner region, 0.2 degrees at the floor and 3 at the ceiling"
        decision: "spec.md:US5-S2"
      - path: crates/finder/tests/console/imaging_smoothing.rs
        status: new
        disposition: add
        assertion: "a camera turned to 40 degrees is set aside twice and then replaces the window, and a moved aim point is followed after three solves"
        decision: "spec.md:US5-S3"
      - path: crates/finder/tests/console/imaging_smoothing.rs
        status: new
        disposition: add
        assertion: "a second console over the same folder and session file reads the three aim readings and adds a fourth, without the session file reads none, and drops those stored for another camera"
        decision: "spec.md:US5-S4"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and each binary's main.rs holds only its support include and mod lines"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "US3's and US4's scenarios hold unchanged: one aim reading is a window of one, and a first solve's radius is still 3 degrees"
      - "AimReading and HintInputs gain no field, Relation::take and Relation::view keep their signatures, and crates/finder/src/imaging/mod.rs is not changed"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console imaging_smoothing::"
        - "cargo nextest run -p finder --locked --test console imaging_relation::"
        - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs, and the aim point watched across the sky at the telescope"
    writes:
      - crates/finder/README.md
      - crates/finder/src/imaging/relation.rs
      - crates/finder/src/imaging/solving.rs
      - crates/finder/tests/console/imaging_smoothing.rs
      - crates/finder/tests/console/main.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/README.md
        other_spec: 032-imaging-solve-and-aim-point
        other_story: US7
        disposition: merge_order
        reason: "US7 depends_on_merged US5, and adds the tool to the MCP list of the README US5 last wrote"
    open_questions: []
  US6:
    tests:
      - path: crates/finder/web/src/components/ImagingSolveBlock.test.tsx
        status: new
        disposition: add
        assertion: "the block shows one button and no status, posts the solve and refreshes, shows a refusal as an alert, and disables the button without a saved frame"
        decision: "spec.md:US6-S1"
      - path: crates/finder/web/src/components/ImagingSolveBlock.test.tsx
        status: new
        disposition: add
        assertion: "solving, solved with and without stars and mirrored, and failed read exactly their words"
        decision: "spec.md:US6-S2"
      - path: crates/finder/web/src/components/ImagingSolveBlock.test.tsx
        status: new
        disposition: add
        assertion: "the scale, the measured aim point, each outcome that took no aim reading and the held aim point read exactly their words, as plain paragraphs"
        decision: "spec.md:US6-S3"
      - path: crates/finder/web/src/components/ImagingSolveBlock.test.tsx
        status: new
        disposition: add
        assertion: "the Imaging page places the Imaging solve region after the Imaging frame region"
        decision: "spec.md:US6-S4"
      - path: crates/finder/web/src/components/ImagingField.test.tsx
        status: new
        disposition: add
        assertion: "imagingFieldCorners gives the worked corners at 0 and 90 degrees and mirrored, and null for no finder scale"
        decision: "spec.md:US6-S5"
      - path: crates/finder/web/src/components/ImagingField.test.tsx
        status: new
        disposition: add
        assertion: "the live chart draws one titled outline with the worked path for a measured aim point, and none for a tapped one, a chart that is not live, an unsolved report or no field"
        decision: "spec.md:US6-S6"
      - path: crates/finder/web/src/components/AimSource.test.tsx
        status: new
        disposition: add
        assertion: "Setup's aim section says measured with the imaging angle or set by tapping, adds no status or alert, and Setup fed by useSession keeps imaging and the aim's source"
        decision: "spec.md:US6-S7"
      - path: crates/finder/web/src/components/AimTap.test.tsx
        status: existing
        disposition: preserve
        assertion: "the tap, its messages, the coordinates line and the reset are unchanged"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/src/components/ChartLive.test.tsx
        status: existing
        disposition: preserve
        assertion: "the live chart keeps its nodes across reports with no measured aim point"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/src/components/SetupPage.test.tsx
        status: existing
        disposition: preserve
        assertion: "Setup reads one status and its forms work as before"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/e2e/simulate.spec.ts
        status: existing
        disposition: preserve
        assertion: "the simulated console starts with no aim point and Setup shows exactly one status, the aim tap's"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "the page adds no npm package and no CSS"
      - "red-only night mode: the outline uses the furniture class and the block existing classes; no gate checks them at night (spec.md owner decision 16)"
      - "no Playwright file or playwright.config.ts changes"
    checks:
      development:
        - "npm test"
        - "npm run typecheck"
        - "npm run build"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs: finder-page, and a look at the outline at night"
    writes:
      - .impeccable/design.json
      - DESIGN.md
      - crates/finder/web/dist
      - crates/finder/web/src/api.ts
      - crates/finder/web/src/components/AimSource.test.tsx
      - crates/finder/web/src/components/AimTap.tsx
      - crates/finder/web/src/components/Chart.tsx
      - crates/finder/web/src/components/ImagingField.test.tsx
      - crates/finder/web/src/components/ImagingField.tsx
      - crates/finder/web/src/components/ImagingPage.tsx
      - crates/finder/web/src/components/ImagingSolveBlock.test.tsx
      - crates/finder/web/src/components/ImagingSolveBlock.tsx
      - crates/finder/web/src/useSession.ts
    no_write_reason: ""
    shared_writes:
      - path: .impeccable/design.json
        other_spec: 031-imaging-frames-on-console
        other_story: US4
        disposition: merge_order
        reason: "032 depends_on_landed 031, whose US4 last changed design.json"
      - path: DESIGN.md
        other_spec: 031-imaging-frames-on-console
        other_story: US4
        disposition: merge_order
        reason: "032 depends_on_landed 031, whose US4 last changed DESIGN.md"
      - path: crates/finder/web/dist
        other_spec: 031-imaging-frames-on-console
        other_story: US4
        disposition: merge_order
        reason: "032 depends_on_landed 031, whose US4 last rebuilt the bundle"
      - path: crates/finder/web/src/api.ts
        other_spec: 031-imaging-frames-on-console
        other_story: US4
        disposition: merge_order
        reason: "032 depends_on_landed 031, whose US4 last changed api.ts"
      - path: crates/finder/web/src/components/ImagingPage.tsx
        other_spec: 031-imaging-frames-on-console
        other_story: US4
        disposition: merge_order
        reason: "032 depends_on_landed 031, whose US4 creates ImagingPage.tsx"
    open_questions: []
  US7:
    tests:
      - path: crates/finder/tests/mcp/imaging_solve.rs
        status: new
        disposition: add
        assertion: "solve_imaging_frame is listed with an object output schema, not read-only, with its description"
        decision: "spec.md:US7-S1"
      - path: crates/finder/tests/mcp/imaging_solve.rs
        status: new
        disposition: add
        assertion: "solve_imaging_frame answers only once a solve delayed by 5 s of virtual time has ended, with the solve the imaging view then shows"
        decision: "spec.md:US7-S2"
      - path: crates/finder/tests/mcp/imaging_solve.rs
        status: new
        disposition: add
        assertion: "arguments, no frame and a failed solve are each a tool error with its text"
        decision: "spec.md:US7-S3"
      - path: crates/finder/tests/mcp/imaging_solve.rs
        status: new
        disposition: add
        assertion: "without imaging or without a solver the tool answers at once with its text, and no result holds the node's token"
        decision: "spec.md:US7-S4"
      - path: crates/finder/tests/mcp/motion.rs
        status: existing
        disposition: preserve
        assertion: "every listed tool called with empty arguments answers without the token, the new one included"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/mcp/frames.rs
        status: existing
        disposition: preserve
        assertion: "solve_frame, the finder camera's upload tool, is listed and answers as before"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and each binary's main.rs holds only its support include and mod lines"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "the existing tools, the door, the rate limit and the call log are unchanged"
      - "no MCP tool is added on the scope node"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test mcp imaging_solve::"
        - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - crates/finder/README.md
      - crates/finder/src/mcp/tools.rs
      - crates/finder/tests/mcp/imaging_solve.rs
      - crates/finder/tests/mcp/main.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/mcp/tools.rs
        other_spec: 031-imaging-frames-on-console
        other_story: US5
        disposition: merge_order
        reason: "032 depends_on_landed 031, whose US5 last changed tools.rs"
      - path: crates/finder/tests/mcp/main.rs
        other_spec: 031-imaging-frames-on-console
        other_story: US5
        disposition: merge_order
        reason: "032 depends_on_landed 031, whose US5 last changed mcp/main.rs"
    open_questions: []
```
