# Implementation Plan: The Node Keeps Saturated Stars

**Branch**: `052-node-keeps-saturated-stars` | **Date**: 2026-11-27 | **Spec**: `spec.md`

**Input**: Feature specification from `specs/052-node-keeps-saturated-stars/spec.md`

## Summary

After cedar's own pass, the scope node's detection finds each region of saturated
pixels, keeps the ones cedar set aside that look like the top of a star, and puts them
first. US1 writes the pass as a new module of `star-detection` and calls it from
`scope_node::detect::detect`; its tests draw saturated stars, trails, doubles, nebulae,
hot pixels and an edge star, read the saved finder camera frame's real saturated star, and run
the simulator's Orion Nebula and Pleiades fields through the console's solve and the
live loop. US2 writes the node's README section, a paragraph of the console's README, the
glossary entry and `solve_image --saturated`, with a test that the README states the
module's constants. cedar's `detect_stars` is not touched.

## Technical Context

**Language/Version**: Rust, MSRV 1.96 (`rust-version` in `Cargo.toml`).

**Primary Dependencies**: none new. `star-detection` stays dependency-free; the node
already depends on it; the console's test binary already reaches `star_detection`,
`scope_node` (a dev-dependency) and `simulator`.

**Testing**: `cargo nextest run --workspace --all-targets --all-features --locked`, the
`test` gate's first command. No page file changes, so the gate's npm half runs as it does
for every story and sees nothing new.

**Target Platform**: `scope-node` on the Pi; `solve_image` on the console's computer.

**Performance**: the pass reads the frame once for a 255 (`<[u8]>::contains`) and stops
there when it holds none; otherwise it reads it once more to list the saturated pixels,
samples every 64th pixel for the median and visits only saturated pixels and their rings.
The probe measured it only as a ratio to cedar's detection (§ What was run).

**Constraints**: detection input stays 8-bit gray (the constitution's product limits);
no parallelism; no allocation the size of the frame beyond the list of saturated pixels.

**Scale/Scope**: two stories, seven files written by US1 (two of them new source and two
new tests) and five by US2 (one new test).

## Constitution Check

| Principle | How this plan meets it |
|-----------|------------------------|
| I. Agreeing with cedar is correctness | `detect_stars` and `detect_stars_with_noise_floor` keep their code and results (FR-002), so cedar-detect's parity (`crates/star-detection/tests/parity.rs:107`) and the e2e evaluation's inputs are as before. The pass is not cedar-detect's, which rejects an overexposed star and leaves exposure to its caller (`reference-solutions/cedar-detect/src/algorithm.rs:81-84`). It is grounded in the other reference, tetra3's spot extraction (`reference-solutions/cedar-solve/tetra3/tetra3.py:2404-2407`): label connected regions of a thresholded image (`reference-solutions/cedar-solve/tetra3/tetra3.py:2597`), refuse by area and by the ratio of the second moments' axes (`reference-solutions/cedar-solve/tetra3/tetra3.py:2632-2652`), with 1.5 the ratio of tetra3's own example (`reference-solutions/cedar-solve/tetra3/tetra3.py:1462`), and centre each on its weighted mean plus 0.5. Here the threshold is 255, so every weight is equal. The ring stands for the sky as cedar-detect's perimeter does, three pixels out from a core (`reference-solutions/cedar-detect/src/algorithm.rs:536-544`). The thresholds 9 pixels and a share of 0.5 are this plan's, chosen on drawn objects (§ Decisions). |
| II. Never weaken a gate | One existing test changes, for the owner's approval (spec.md owner approval 3): the saved finder camera frame's test is replaced by one that keeps every check it made and holds the rest of the list to `detect_stars` exactly. No tolerance is broadened, no golden or CI file is touched. |
| III. Tests first, with independent expectations | Every expected place is drawn by the test or the simulator's truth; the saved frame's star is the mean of its own 19 pixels, written as a fraction; the README's numbers come from the module's constants. US1's new tests fail with the pass turned off (run). |
| IV. Read the critical paths twice | None is touched: no `proto/`, `math-core`, database crate, `Cargo.toml` or `Cargo.lock`. The scope node's detection is the critical change by its effect; its diff is one call. |
| V. Proven open source first | Looked for, from local sources only: `reference-solutions/` holds cedar-detect, which has no path for a saturated star, and tetra3, whose spot extraction is the shape taken here (above). The crate caches (`~/.cargo/registry`, the factory's) hold no image-analysis crate, only `image` for decoding; nothing like `imageproc`'s connected components is available offline, and a dependency for one flood fill would be a Principle IV change. Detection is the solver's own Rust under Principle V's exception, so the pass is written here. Not looked at: detectors outside `reference-solutions/` (Source Extractor and its saturation flags), which the exception keeps out. No reused code, no new license. |

## Project Structure

```text
crates/star-detection/src/saturated.rs        # US1, new: the pass
crates/star-detection/src/lib.rs              # US1: pub mod saturated;
crates/star-detection/tests/saturated.rs      # US1, new: US1-S1 to US1-S3
crates/scope-node/src/detect.rs               # US1: with_saturated_stars before the cut
crates/scope-node/tests/detect.rs             # US1: one test replaced (US1-S4)
crates/finder/tests/console/saturated_field.rs # US1, new: US1-S5, US1-S6
crates/finder/tests/console/main.rs           # US1: mod saturated_field;
crates/scope-node/README.md                   # US2: ## Bright stars: saturated stars
crates/finder/README.md                       # US2: one paragraph
GLOSSARY.md                                   # US2: Saturated star
crates/plate-solver/examples/solve_image.rs   # US2: --saturated
crates/scope-node/tests/saturated_readme.rs   # US2, new: US2-S1
```

## Interfaces

### `crates/star-detection/src/saturated.rs` (US1)

```rust
pub const SATURATED: u8 = 255;
pub const MIN_PIXELS: usize = 9;
pub const MAX_AXIS_RATIO: f64 = 1.5;
pub const RING_MARGIN: usize = 3;
pub const MAX_RING_SHARE: f64 = 0.5;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SetAside { Small, Edge, Elongated, Extended }

pub fn with_saturated_stars(image: &[u8], width: usize, height: usize, stars: Vec<Star>)
    -> Vec<Star>;
pub fn saturated_stars(image: &[u8], width: usize, height: usize, stars: &[Star])
    -> (Vec<Star>, Vec<SetAside>);
```

`saturated_stars` asserts `image.len() == width * height`, as `detect_stars` does, and
answers two empty lists when the frame holds no 255. Otherwise, in order:

1. The median: of the pixels at indices 0, 64, 128 and so on, the least level at which
   the count so far passes half the samples.
2. Each region: from each pixel at 255 not yet visited, in reading order, an 8-connected
   flood over pixels at 255, summing its count n, x, y, x², y² and xy (pixel indices) and
   its box (least and greatest x and y).
3. A region of n = 1, or whose box, read as `left ≤ s.x ≤ right + 1` and
   `top ≤ s.y ≤ bottom + 1`, holds any star `s` of `stars`, is passed over: neither kept
   nor set aside.
4. `Small` when n < `MIN_PIXELS`.
5. `Edge` when `left < RING_MARGIN`, `top < RING_MARGIN`,
   `right + RING_MARGIN ≥ width` or `bottom + RING_MARGIN ≥ height`.
6. `Elongated` when, with the mean (cx, cy) and variances vxx, vyy, vxy about it, the
   eigenvalues (vxx + vyy)/2 ± √((vxx − vyy)²/4 + vxy²) give a minor not above 0 or
   √(major/minor) above `MAX_AXIS_RATIO`.
7. `Extended` when the mean of the ring, the one-pixel perimeter of the box widened by
   `RING_MARGIN` on each side, less the median, exceeds `MAX_RING_SHARE` × (255 − median).
8. Otherwise kept: `Star::new(cx + 0.5, cy + 0.5, SATURATED, brightness, n)`, brightness
   the sum over the pixels strictly inside the ring of (pixel − ring mean), which
   `Star::new` holds at 0 or above.

The kept stars are sorted by `num_saturated`, most first, then by `Star`'s own order.
`with_saturated_stars` returns `stars` itself when none is kept, else the kept stars
followed by `stars`. The module doc says what cedar's gates set aside and cites
cedar-detect's caveat and tetra3's extraction.

### `crates/scope-node/src/detect.rs` (US1)

`detect` keeps its two branches, then passes their `stars` through
`star_detection::saturated::with_saturated_stars(&frame.pixels, width, height, stars)`
before `.take(200)` and the map to `Centroid`. The doc line of `detect` gains "with the
saturated stars cedar sets aside first (052)".

### The node's README (`crates/scope-node/README.md`, US2)

Inserted before `## The mount link`, exactly:

````markdown
## Bright stars: saturated stars

A bright star saturates the finder camera: its brightest pixels read 255, the most an
8-bit frame holds, and its top is flat. cedar-detect's gates take a flat top only a few
binned pixels wide and set a wider one aside, so on a longer exposure the brightest stars
of a frame, the ones the star patterns are built from, went missing, and on a field
night the Orion Nebula's and the Pleiades' fields gave `no_match` with tens of stars
detected.

The node's detection keeps them. After cedar-detect's own pass it finds each region of
saturated pixels, and keeps one as a star when cedar-detect found no star in it and it is:

- at least 9 pixels: a hot pixel is one;
- round, its length at most 1.5 times its width: a trail, or two stars run together,
  is longer;
- dark around it, the ring 3 pixels out from it averaging less than 0.5 of the way from
  the frame's median to 255: the bright core of a nebula is not;
- whole, that ring inside the frame.

Each is centred on the mean of its saturated pixels and put first, the most saturated
pixels first; cedar-detect's stars follow in their own order, and the node keeps the
first 200 as before. A star frame does not say which centroids are saturated, nor how
many regions were set aside.

The console's Solve page and `solve_image` detect as cedar-detect does. To replay a raw
frame as the node detected it, here or when finding the noise floor's value, add
`--saturated`, which also prints how many saturated regions were kept and set aside, and
why:

```sh
cargo run --release --locked -p plate-solver --example solve_image -- \
  --db <the-database-the-console-reports> --saturated <frame>
```

On the simulator's Orion Nebula and Pleiades fields at the finder camera's 1280 x 960
grid, with their brightest stars saturated, the node's detection solves where
cedar-detect's alone gives `no_match`. Whether a real finder camera frame does is a field
night's check.
````

### The console's README (`crates/finder/README.md`, US2)

Before the line `` `GET /api/live` is `text/event-stream`. `` and a blank line, exactly:

```markdown
The live solve takes the scope node's centroids as they come, the saturated stars
cedar-detect sets aside first ([scope node README](../scope-node/README.md), Bright stars:
saturated stars); an uploaded frame on the Solve page is detected as cedar-detect detects it.
```

### The glossary (`GLOSSARY.md`, US2)

After **Centroid**'s entry and its blank line, before **Pattern**, exactly:

```markdown
**Saturated star**:
A star whose brightest pixels read 255, the most an 8-bit frame holds, so its top is flat. The scope node's detection keeps one that cedar-detect's gates set aside, first, when it is round, dark around and whole in the frame.
_Avoid_: blown star, overexposed star
```

### `solve_image` (`crates/plate-solver/examples/solve_image.rs`, US2)

`Args` gains `saturated: bool`, set by `--saturated`; the usage line reads
`... [--noise-floor F] [--saturated] [--stars N] ...`, the doc block's synopsis lists
`[--saturated]` after `[--noise-floor F]`, and its option list gains, after
`--noise-floor`'s two lines:

```text
//! * `--saturated` — keep the saturated stars cedar sets aside, first, as the scope node
//!   does (`star_detection::saturated`), and print how many regions were kept and set aside.
```

With the flag, after detection: `saturated_stars(&img, w, h, &stars)`; its kept stars
followed by the detected ones become `stars`; the counts of each reason, in the order
small, edge, elongated, extended, are kept. Without `--json`, before the result line, it
prints `<path>: saturated stars kept <k>, set aside <a> (small <s>, edge <e>, elongated
<l>, extended <x>)`; with `--json`, the object gains `,"saturated_kept":<k>,
"saturated_set_aside":<a>` after `t_solve_ms`. Without the flag nothing it prints changes.

## What was run and what was read

The probe, on `probe/052` cut from f6d4f5fd, in the operator's probe folder's `logs/`
(not in the repository), with the factory's toolchain (rustc 1.100.0, nextest 0.9.152)
and `--locked`:

- **72559cc6**, a sweep (throwaway) on `main`'s detection: the simulator's finder camera
  on the Orion Nebula's and the Pleiades' fields, 1280 × 960 at 10.5°, 12° and 15°,
  focus errors 0 to 3, exposures 200 to 2000 ms and gains 1 to 16 (log p0). At focus
  error 3, 500 ms and gain 2 and 10.5°, the Orion field gave 69 centroids, none within
  3 px of its 10 brightest drawn stars, and `no_match`; the Pleiades', 56 and `no_match`.
  At 1000 ms and gain 16 every frame was white (the simulator's sky passes 255): not
  used.
- **00e6a018**, the first trial (a ring at twice a region's radius): every frame of the
  sweep that gave `no_match` matched within 7″ (log p1). The whole workspace: 1525 tests
  passed and one failed, the saved finder camera frame's (38 centroids against 37; log p2).
- **28fa8572**, the measurement sources (throwaway, not compiled at the tip):
  - drawn objects through four rings (3 or 6 px out, share 0.25 or 0.5) and the first
    trial's (logs p3, p4); the numbers are spec.md owner decision 3's;
  - on 5 fields (the Orion Nebula's, the Pleiades', Orion's belt, Vega's, Capella's) at
    10.5° and 12°, focus errors 1 to 3, 200 to 750 ms at gain 1 or 2 (logs p5 to p7):
    520 kept stars and both centroids (owner decision 2), 115 frames with a kept star
    and three ranks (owner decision 4), the reasons set aside (small 339, edge 43,
    elongated 25, extended 0; cedar's stars passed over 333), one cedar star within a
    kept star's ring;
  - 120 frames of the simulator's default fields (60 pointings, focus errors 0 and 1,
    200 ms, gain 1): 120 solve with and without the pass, 9 changed (logs p5, p6);
  - the cost as a ratio of the node's detection to cedar's alone, in the test profile:
    1.02 on 64 frames with no pixel at 255, 1.26 on 116 with some, and 1.11 to 1.39
    (median 1.27) on the 115 bright frames (logs p6, p7).
- **5a0b5f95**, the module as § Interfaces gives it.
- **64890b11**, US1 as a story would write it: US1-S1 to US1-S3 passed (log p8, which
  also printed cedar's 13 reference images through the pass: one kept star, on
  `test_5mp_g100_e100ms.jpg` at binning 1 only); US1-S4 (log p9); US1-S5 and US1-S6
  (log p10). With the pass turned off in `detect` (a mutation, reverted), US1-S4,
  US1-S5 and US1-S6 failed: 37 centroids against 38, a bright star missing from the
  node's first 10, and the live report `NoMatch` with "No star pattern in this frame
  matched the database..." (log p11).
- **462e2a3c**, US2: US2-S1 passed (log p12); `solve_image` by hand, debug build, over
  the bundled database (log p13): the two bright frames `NO_MATCH` (69 and 56 stars)
  without `--saturated` and `MATCH_FOUND` with it (kept 7 and 7, set aside 1 small and
  0), within 5″ of the simulator's truth; the saved finder camera frame `MATCH_FOUND` both ways
  (kept 1).
- **5f041e61**, the two stories' files only: `lint`'s commands exit 0 (log p14);
  `test`'s Rust commands, 1531 of 1531 (1524 existing unchanged, one replaced, six new)
  and the doc tests (log p15). The npm half was not run: no story writes the page.
- **d78b8fce**, the tip: US1-S1 also holds cedar's list clear of each saturated star
  (FR-002); `star-detection`'s tests and clippy pass (log p16), and `lint` and `test`'s
  Rust commands again at the tip (log p17).

Read, not run: the field night's frames (only a 640 × 480 preview survives); the INDI
finder camera adapter's 255 at full scale; the Mini's hot pixels; the pass's cost on the Pi; every
`path:line` cited, at f6d4f5fd.

## Read from the diff, not from a test

- FR-002, beyond US1-S1's check that cedar's list still sets each drawn saturated star
  aside: `crates/star-detection/src/detect.rs` and `crates/star-detection/src/binning.rs`
  are not in the diff, and the parity tests hold their results.
- FR-001's brightness of a kept star, its order among kept stars with equal counts, and
  the sampled median: no scenario pins them beyond US1-S1's one star a frame.
- FR-004: the diff's file list.
- FR-005's paragraph of the console's README and the README section's prose beyond US2-S1's
  phrases.
- FR-007's behaviour: examples are compiled and linted by the gates and never run
  (048's precedent for `--noise-floor`); US2-S1 reads its usage line and doc line, and
  the probe ran it by hand.

## Shapes that existing tests pin

With both stories built, every existing test but the replaced one passed (log p15).
The files beside the change:

- **`crates/star-detection/tests/parity.rs`** and **`noise_floor.rs`**: call
  `detect_stars` and `detect_stars_with_noise_floor`, which do not change.
- **`crates/scope-node/tests/detect.rs`**: `cedar_defaults_equal_every_console_field`
  (`crates/scope-node/tests/detect.rs:10`) compares named fields;
  `crowded_frame_keeps_the_detectors_brightest_two_hundred_in_order`
  (`crates/scope-node/tests/detect.rs:68`) draws peaks of 108, so the pass returns
  cedar's list; the saved finder camera frame's test (`crates/scope-node/tests/detect.rs:26`)
  is replaced (US1-S4).
- **`crates/scope-node/tests/raw_frame.rs`** (`crates/scope-node/tests/raw_frame.rs:68`,
  `crates/scope-node/tests/raw_frame.rs:95`) and
  **`crates/finder/tests/console/recording_raw.rs`**
  (`crates/finder/tests/console/recording_raw.rs:209`) hold the node's streamed
  or recorded centroids to `detect_stars`; their simulated frames hold no saturated star
  cedar sets aside, so they pass unchanged (run). A future frame that does would fail
  them: plan trap 3.
- **`crates/simulator/tests/dark_sky.rs`**, **`dark_sky_exposure.rs`** and **`sky.rs`**
  detect with `scope_node::detect::detect`: they pass unchanged (run).
- **051's `crates/finder/tests/console/finder_focus_measure.rs`** and
  **`finder_focus_aid.rs`** measure at the node's centroids: they pass unchanged (run).

## Decisions and the alternatives rejected

The options of spec.md § Owner decisions, with the probe's measures:

- **Where** (decision 1): (a) the node only, through a new module; (b) inside
  `detect_stars` moves the parity image `test_5mp_g100_e100ms.jpg` at binning 1;
  (c) a switch adds a config key, a `--check-config` line and Configure's handling for
  nothing to switch off.
- **Centroid** (decision 2): (a) mean of the saturated pixels; (b) flux-weighted inside
  the ring, its tail pulled by neighbours (90th percentile 1.15 px against 0.29 px);
  (c) a wing fit, unreferenced.
- **Gates** (decision 3): the ring at 2r (first trial) kept smooth and lobed nebulae;
  3 px with 0.25 set aside soft stars; 6 px with 0.5 kept a core in a faint halo;
  3 px with 0.5 kept every drawn star and set aside every drawn nebula, trail, double 8
  to 12 px apart and edge star. An area cap (tetra3's) cannot tell a nebula core from a
  bright star.
- **Rank** (decision 4): first 113 of 115, merged 113, last 79; only first keeps cedar's
  list intact behind the new stars.
- **The report** (decision 5): a star frame field is a protocol change and a write to
  `crates/finder/src/live.rs` while 053 changes it.
- **Exposure hint** (decision 6): later, if a field night shows the need.

## User Story 1 - The pass and the node

Tests first: `crates/star-detection/tests/saturated.rs` fails to build without the
module; the replaced test and `saturated_field.rs` fail with the module but without the
call in `detect` (run, as the mutation above). Then the module, then the call.

## User Story 2 - The words and the replay

The README test first (it fails on the missing section), then the README, the finder
README's paragraph, the glossary and `solve_image`.

## Verification (every story, before proposing)

- `cargo fmt --all --check`, then the `lint` gate's clippy and doc commands.
- The story's development commands (spec.md, each story's Independent Test), then the
  `test` gate.
- Every existing test unchanged and green but US1-S4's, which US1 replaces.

## Prerequisites before this spec is flipped to `ready`

- Every spec in `depends_on_landed` has landed: met at f6d4f5fd.
- `main` is still as read, or the cites are re-read.
- A review round of this trio, folded.
- The six owner decisions answered.
- Optionally, a probe round that builds both stories from this text alone.

## Outside the factory

Written, not run. On the next field night, with the kit holding this spec:

1. With the finder camera at gain 80 and sigma 8, as on the last field night, point at
   the Orion Nebula's field, then the Pleiades', each with its brightest stars
   saturated, and Track on. At each of 1000, 1500 and 3000 ms, let about 20 live reports
   pass and note how many are `match_found` and the detected counts.
2. At each setting, record 3 raw frames with `POST /api/record/raw` (the console's README's
   Recording a night), as future fixtures; note exposure, gain and the field beside the
   recording.
3. On the console's computer, replay each with `solve_image --saturated` over the
   database the console reports, and note kept and set-aside counts and reasons.
4. If a field still gives `no_match` with its brightest stars kept, or a nebula's core is
   kept and leads the list, bring the recordings back: the next spec's probe replays them
   through `scope_node::detect::detect`.

## Plan traps

1. **Do not touch `detect_stars`.** The pass changes a parity image at binning 1
   (§ What was run); inside `detect_stars` it would move cedar's parity and the e2e
   evaluation's inputs. Mechanism: the reference images do hold saturated stars. Check:
   `crates/star-detection/src/detect.rs` is not in the diff.
2. **The replaced test keeps its checks.** Principle II: the replacement keeps the
   fixture's size checks, the non-empty list and the exact equality of every centroid
   after the first; it adds only the saturated star. Check: US1-S4's words.
3. **Tests holding the node to `detect_stars`** (`raw_frame.rs`, `recording_raw.rs`) pass
   only because their simulated frames hold no saturated star cedar sets aside. A story
   that changes the simulator's frames could break them; FR-004 keeps the simulator's
   `src/` out.
4. **The live test's first reports.** An exposure in flight when the frames stop finishes
   with the old optics (`crates/finder/README.md`, Simulated night: "An exposure already
   in progress finishes before a push or stop takes effect"), so its report is 1456 wide;
   US1-S6 counts only 1280-wide reports.
5. **Lines wrap in the README.** US2-S1 reads the section with its whitespace runs as one
   space; a phrase split across lines still matches.
6. **Names.** "finder camera", never "finder" alone; "scope node"; "saturated star" as the
   glossary gives it.
7. **No timing or place** in the README or a message: the field night is "a field night".
8. **Concurrent drafts.** Specs 053 and 054 are drafted beside this one and may write the
   console's README, the glossary or the console's test binary's `main.rs`; the floor orders
   them by `depends_on_landed`, and a story rebases on what landed.

## Risks

- **A real star's halo** (scattered light, reflections, a heavy-winged profile beyond the
  Moffat β 1.8 drawn) could brighten its ring past the share and set it aside as
  extended.
- **A nebula's compact saturated knot** in dark surroundings is kept, first; on the Orion
  Nebula the Trapezium's stars are catalogued, elsewhere a knot would be a false first
  star.
- **A clipped top below 255**, from a camera or a conversion, gets no help.
- **The cost on the Pi** is measured only as a ratio on the factory's machine.

## Refinement Record

Drafted on 2026-11-27 by the plate-solver operator seat's spec session against `main` at
f6d4f5fd, from a probe of that day on `probe/052` (a trial build of both stories with
every scenario's test and both gates' Rust commands passing at d78b8fce, US1's new tests
failing with the pass turned off, the gates' candidates measured on drawn objects and on
the simulator's bright fields), and a reading of cedar-detect's gates and their port, the
node's detection and its callers, 048's dark-sky work and tests, the simulator's finder
camera, the console's live loop and solve, 025's raw frames, 051's focus measure,
tetra3's spot extraction, the constitution, the glossary and the ADRs. Writes come from
the task text. Shared writes name the spec story that last wrote the same file on
`main`; within the spec, US2 follows US1 by `depends_on_merged`.

The six owner decisions of spec.md are open. This record, like the scenarios, is written
to their recommendations (decisions 1 to 4 for US1, 5 and 6 for US2), so its
`open_questions` are empty; an answer other than a recommendation reopens the record and
the scenarios it names. This record approves nothing; the flip is the owner's.

```yaml
version: 1
reviewed_revision: f6d4f5fd482ccc9d5b634ef1aa5124c112769583
stories:
  US1:
    tests:
      - path: crates/star-detection/tests/saturated.rs
        status: new
        disposition: add
        assertion: "drawn 640 x 480 frames, background 10, one Gaussian at (p, p), p 200.5, 200.75, 201.0, sigma/P 1.5/3000, 3/3000, 3/30000, 5/3000, and Moffats at (200.5, 200.5) alpha 2, P 25500, beta 2.5 and 1.8, each with a Gaussian at (400.5, 300.5) sigma 1.5 P 100: saturated_stars with cedar's list (sigma 8, binning 2, rows normalised, hot pixels rejected) keeps one star within 0.1 px of (p, p), peak_value 255, num_saturated the frame's count of 255, sets nothing aside; with_saturated_stars gives it then cedar's list unchanged; cedar's list has a star within 0.5 px of (400.5, 300.5) and none within 3 px of (p, p)"
        decision: "spec.md:US1-S1"
      - path: crates/star-detection/tests/saturated.rs
        status: new
        disposition: add
        assertion: "segments (150, 100) to (450, 340) at 400 within 1, 2, 3 px; Gaussian pairs sigma 3 at (300.5 -/+ d/2, 240.5), P 3000/3000 d 8, 10, 12 and P 3000/800 d 8, 12; nebulae sigma 40 P h plus sigma s P c at (320.5, 240.5), (h, s, c) (200, 8, 400), (60, 8, 400), (0, 25, 400), (150, 3, 3000); three lobes at (300.5, 230.5), (330.5, 250.5), (315.5, 270.5) sigma 30, 25, 20 P 180, 180, 150; a Gaussian sigma 3 P 3000 at (5.5, 240.5): none kept; segments and pairs [Elongated], nebulae [Extended], the edge star [Edge]; with_saturated_stars gives cedar's list unchanged; blocks of 255 from (320, 240) with no stars: 1 x 1 [], 2 x 2 and 4 wide 2 high [Small]"
        decision: "spec.md:US1-S2"
      - path: crates/star-detection/tests/saturated.rs
        status: new
        disposition: add
        assertion: "two Gaussians sigma 3 P 3000 at (300.5 -/+ d/2, 240.5): nothing set aside, every kept star within 0.1 px of y 240.5; d 4 and 6 one kept within 0.1 px of x 300.5; d 16 and 24 two kept within 0.1 px of 300.5 -/+ d/2"
        decision: "spec.md:US1-S3"
      - path: crates/scope-node/tests/detect.rs
        status: existing
        disposition: replace
        assertion: "saved_finder_frame_keeps_its_saturated_star_first_then_the_consoles_centroids_in_order replaces saved_finder_frame_has_identical_nonempty_centroids_in_console_order: fixtures/scope-node/finder-frame.png at the size fixtures.json gives; its pixels at 255 exactly (20-24, 187), (20, 21, 23, 24, 188), (20-22, 189), (20-24, 190), (22, 23, 191); detect_stars at DetectParams::cedar non-empty, no star within 5 px of (22.5, 189.5); detect with DetectSettings::cedar gives one more centroid, the first within 1e-9 px of (416/19 + 0.5, 3586/19 + 0.5), every later one equal to detect_stars' in order, x, y and brightness"
        decision: "spec.md:FR-003"
      - path: crates/scope-node/tests/detect.rs
        status: existing
        disposition: preserve
        assertion: "cedar_defaults_equal_every_console_field and crowded_frame_keeps_the_detectors_brightest_two_hundred_in_order as before: the crowded grid's peaks of 108 hold no 255"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/console/saturated_field.rs
        status: new
        disposition: add
        assertion: "bright frames (SimCamera 500 ms gain 2, DB, Site::default, START, optics 1280 x 960 at 10.5 degrees eyepiece (640, 480), point_eyepiece_at, finder camera focus error 3) of the Orion Nebula (83.82, -5.39) and the Pleiades (56.75, 24.12): each of the first 5 drawn stars of field_stars(500, 2) at least 20 px inside has a node centroid (detect, DetectSettings::cedar) within 1.5 px among the first 10 and no centroid of cedar's list within 3 px; solve_centroids with SolveOptions::default of cedar's list cut to 200 NoMatch, of the node's MatchFound within 30 arcsec of telescope().camera"
        decision: "spec.md:US1-S5"
      - path: crates/finder/tests/console/saturated_field.rs
        status: new
        disposition: add
        assertion: "AppState::shared over DB with simulation at START over SimulationTransport::Memory and the live loop, frames stopped, the simulator set as a bright frame's for the Orion Nebula, PUT /api/session camera_settings exposure_ms 500 gain 2 answered 200, frames started: the first three live reports 1280 wide each MatchFound, pointing within 60 arcsec of the simulator's camera direction read then, all within 30 s of START, under start_paused"
        decision: "spec.md:US1-S6"
      - path: crates/star-detection/tests/parity.rs
        status: existing
        disposition: preserve
        assertion: "detect_stars at binning 1, sigma 8 matches cedar-detect's goldens on the 13 reference images, as before 052"
        decision: "spec.md:FR-002"
      - path: crates/star-detection/tests/noise_floor.rs
        status: existing
        disposition: preserve
        assertion: "detect_stars_with_noise_floor at floors 0 to 0.2 equals detect_stars star for star, as before 052"
        decision: "spec.md:FR-002"
      - path: crates/scope-node/tests/raw_frame.rs
        status: existing
        disposition: preserve
        assertion: "the node's streamed centroids equal detect_stars of the raw pixels on its simulated frames, which hold no saturated star cedar sets aside"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/console/recording_raw.rs
        status: existing
        disposition: preserve
        assertion: "a recorded raw frame's line replays through detect_stars to its centroids on the simulated frames, as 025 words it"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/simulator/tests/dark_sky.rs
        status: existing
        disposition: preserve
        assertion: "048's dark and city skies through scope_node::detect::detect, counts and solves as 048 words them"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "detect_stars and detect_stars_with_noise_floor: code and results as at f6d4f5fd"
      - "a frame with no pixel at 255, or none kept, gives the node exactly cedar's list, cut to 200"
      - "the star frame, the scope link's protocol, DetectSettings and the node's config as at f6d4f5fd"
    checks:
      development:
        - "cargo nextest run -p star-detection --locked --test saturated"
        - "cargo nextest run -p scope-node --locked --test detect"
        - "cargo nextest run -p finder --locked --test console -E 'test(saturated_field)'"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs (e2e-eval among them)"
    writes:
      - crates/star-detection/src/saturated.rs
      - crates/star-detection/src/lib.rs
      - crates/star-detection/tests/saturated.rs
      - crates/scope-node/src/detect.rs
      - crates/scope-node/tests/detect.rs
      - crates/finder/tests/console/saturated_field.rs
      - crates/finder/tests/console/main.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/star-detection/src/lib.rs
        other_spec: 048-node-detects-stars-on-a-dark-sky
        other_story: US1
        disposition: merge_order
        reason: "052 depends_on_landed 048, whose US1 last changed lib.rs (ce7f675a)"
      - path: crates/scope-node/src/detect.rs
        other_spec: 048-node-detects-stars-on-a-dark-sky
        other_story: US1
        disposition: merge_order
        reason: "052 depends_on_landed 048, whose US1 last changed the node's detect (ce7f675a)"
      - path: crates/scope-node/tests/detect.rs
        other_spec: 013-live-finder
        other_story: US2
        disposition: merge_order
        reason: "052 depends_on_landed 013, whose US2 last changed tests/detect.rs (ee280ae7)"
      - path: crates/finder/tests/console/main.rs
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US2
        disposition: merge_order
        reason: "052 depends_on_landed 051, whose US2 last changed console/main.rs (f9706cc0)"
    open_questions: []
  US2:
    tests:
      - path: crates/scope-node/tests/saturated_readme.rs
        status: new
        disposition: add
        assertion: "crates/scope-node/README.md's section from ## Bright stars: saturated stars to the next ## heading, whitespace runs as one space, holds its brightest pixels read 255, at least 9 pixels: a hot pixel is one, its length at most 1.5 times its width, the ring 3 pixels out from it averaging less than 0.5 of the way from the frame's median to 255, whole, that ring inside the frame, put first, the most saturated pixels first, the node keeps the first 200 as before, A star frame does not say which centroids are saturated, nor how many regions were set aside., --saturated <frame>, each number formatted from SATURATED, MIN_PIXELS, MAX_AXIS_RATIO, RING_MARGIN, MAX_RING_SHARE; GLOSSARY.md holds **Saturated star**: then a line beginning A star whose brightest pixels read 255; solve_image.rs holds [--noise-floor F] [--saturated] [--stars N] and a doc line beginning //! * `--saturated` -"
        decision: "spec.md:US2-S1"
    preserved_behavior:
      - "solve_image without --saturated prints and detects as at f6d4f5fd"
      - "the README's other sections, the glossary's other entries"
    checks:
      development:
        - "cargo nextest run -p scope-node --locked --test saturated_readme"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs (docs among them)"
    writes:
      - crates/scope-node/README.md
      - crates/finder/README.md
      - GLOSSARY.md
      - crates/plate-solver/examples/solve_image.rs
      - crates/scope-node/tests/saturated_readme.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/scope-node/README.md
        other_spec: 045-node-runs-finder-over-indi
        other_story: US2
        disposition: merge_order
        reason: "052 depends_on_landed 045, whose US2 last changed the node's README (ea3d6a9b)"
      - path: crates/finder/README.md
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US3
        disposition: merge_order
        reason: "052 depends_on_landed 051, whose US3 last changed the console's README (9424a371)"
      - path: GLOSSARY.md
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US3
        disposition: merge_order
        reason: "052 depends_on_landed 051, whose US3 last changed the glossary (9424a371)"
      - path: crates/plate-solver/examples/solve_image.rs
        other_spec: 048-node-detects-stars-on-a-dark-sky
        other_story: US2
        disposition: merge_order
        reason: "052 depends_on_landed 048, whose US2 last changed solve_image (75cb2c8c)"
    open_questions: []
```
