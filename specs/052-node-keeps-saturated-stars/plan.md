# Implementation Plan: The Node Keeps Saturated Stars

**Branch**: `052-node-keeps-saturated-stars` | **Date**: 2026-11-27 | **Spec**: `spec.md`

**Input**: Feature specification from `specs/052-node-keeps-saturated-stars/spec.md`

## Summary

After cedar's own pass, detection finds each region of saturated pixels, keeps the ones
cedar set aside that look like the top of a star, round or trailed with the rest of the
frame, and places each by its brightness. US1 writes the pass as a new module of
`star-detection`, makes it the finder camera's one saturation level with 051's focus aid,
and tests it on drawn stars, trails, doubles, nebulae, hot pixels, an edge star and a
washed-out frame. US2 calls it from the scope node's `detect` and the console's
`solve_frame_with_frame`, replaces the saved finder camera frame's test, and runs the
simulator's Orion Nebula and Pleiades fields through the console's solve and the live
loop, the Pleiades at the finder camera's own 1.617° over a narrow database built in the
test, and an uploaded frame through the Solve page's path. US3 writes the node's README
section, a paragraph of the console's README, the glossary entry and
`solve_image --saturated`, with a test that the README states the module's constants.
cedar's `detect_stars` is not touched.

## Technical Context

**Language/Version**: Rust, MSRV 1.96 (`rust-version` in `Cargo.toml`).

**Primary Dependencies**: none new. `star-detection` stays dependency-free; the node and
the console already depend on it; the console's test binary already reaches
`star_detection`, `scope_node` and `database_generation` (dev-dependencies),
`simulator`, `pattern_database`, `math_core` and `image`.

**Testing**: `cargo nextest run --workspace --all-targets --all-features --locked`, the
`test` gate's first command. No page file changes, so the gate's npm half runs as it does
for every story and sees nothing new.

**Target Platform**: `scope-node` on the Pi; the console and `solve_image` on the
console's computer.

**Performance**: the pass counts the frame's pixels at 250 or above in one scan and stops
there when there are none or more than 24,576; otherwise it samples every 64th pixel for
the median and visits only those pixels and their rings. The probe measured it only as a
ratio to cedar's detection (§ What was run).

**Constraints**: detection input stays 8-bit gray (the constitution's product limits);
no parallelism; no allocation the size of the frame beyond the set of at most 24,576
saturated pixels.

**Scale/Scope**: three stories: US1 four files (two new), US2 six (two new, one of them a
fixture), US3 five (one new).

## Constitution Check

| Principle | How this plan meets it |
|-----------|------------------------|
| I. Agreeing with cedar is correctness | `detect_stars` and `detect_stars_with_noise_floor` keep their code and results (FR-002), so cedar-detect's parity (`crates/star-detection/tests/parity.rs:107`) and the e2e evaluation's inputs are as before. The pass is not cedar-detect's, which rejects an overexposed star and leaves exposure to its caller (`reference-solutions/cedar-detect/src/algorithm.rs:81-84`). **tetra3's**, from its spot extraction (`reference-solutions/cedar-solve/tetra3/tetra3.py:2404-2407`): labelled connected regions of a thresholded image (`reference-solutions/cedar-solve/tetra3/tetra3.py:2597`); the least area of 5 pixels, its `min_area` default (`reference-solutions/cedar-solve/tetra3/tetra3.py:2406`, `reference-solutions/cedar-solve/tetra3/tetra3.py:2632`); the ratio of the second moments' axes, refused above 1.5, the ratio of tetra3's own example (`reference-solutions/cedar-solve/tetra3/tetra3.py:2641-2652`, `reference-solutions/cedar-solve/tetra3/tetra3.py:1462`); the centre of mass plus 0.5 (`reference-solutions/cedar-solve/tetra3/tetra3.py:2625-2633`), here with every weight equal. **Not tetra3's nor cedar-detect's, and so spec.md owner approval 6's exception, each measured** (spec.md owner decisions 3 and 7): 8-connectivity, where tetra3 labels with scipy's default 4-connectivity; the level 250, where cedar-detect counts a pixel saturated at exactly 255 (`reference-solutions/cedar-detect/src/algorithm.rs:798`, `crates/star-detection/src/star.rs:15`) and 250 is 051's focus aid's (`crates/finder/src/finder_focus.rs:40`); the frame bound of 24,576 pixels, where tetra3 caps each spot at 100 pixels; the trail's 10° and 3 regions; the ring 3 frame pixels out from the box, where cedar-detect's perimeter lies three binned pixels out from its core (`reference-solutions/cedar-detect/src/algorithm.rs:536-538`), 6 frame pixels at binning 2; and the ring's share of 0.5. A builder who reads the constitution first finds them approved by name, not invented. |
| II. Never weaken a gate | One existing test changes, for the owner's approval (spec.md owner approval 3): the saved finder camera frame's test is replaced by one that keeps every check it made and holds the rest of the list to `detect_stars` exactly. No tolerance is broadened, no golden, existing fixture or CI file is touched. |
| III. Tests first, with independent expectations | Every expected place is drawn by the test or the simulator's truth; the saved frame's star is the mean of its own 29 pixels, written as a fraction; US2-S4's database is built from the fixture's catalog, not from the code under test; the README's numbers come from the module's constants. Each new test fails on the state before its story (run). |
| IV. Read the critical paths twice | None is touched: no `proto/`, `math-core`, database crate, `Cargo.toml` or `Cargo.lock`. The scope node's and the console's detection are critical by their effect; each diff is one call. |
| V. Proven open source first | Looked for, from local sources only: `reference-solutions/` holds cedar-detect, which has no path for a saturated star, and tetra3, whose spot extraction is the shape taken here (above); taken whole, its area window of 5 to 100 pixels over the blob above threshold would refuse the brightest tops and needs its background filter. The crate caches (`~/.cargo/registry`, the factory's) hold no image-analysis crate, only `image` for decoding; nothing like `imageproc`'s connected components is available offline, and a dependency for one flood fill would be a Principle IV change. Detection is the solver's own Rust under Principle V's exception, so the pass is written here. Not looked at: detectors outside `reference-solutions/` (Source Extractor and its saturation flags), which the exception keeps out. No reused code, no new license; the new fixture's rows come from the Tycho catalogue the kit's narrow database is built from. |

## Project Structure

```text
crates/star-detection/src/saturated.rs          # US1, new: the pass
crates/star-detection/src/lib.rs                # US1: pub mod saturated;
crates/star-detection/tests/saturated.rs        # US1, new: US1-S1 to US1-S5
crates/finder/src/finder_focus.rs               # US1: SATURATED is the pass's
crates/scope-node/src/detect.rs                 # US2: with_saturated_stars before the cut
crates/scope-node/tests/detect.rs               # US2: one test replaced (US2-S1)
crates/finder/src/solve.rs                      # US2: with_saturated_stars after detect_stars
crates/finder/tests/console/saturated_field.rs  # US2, new: US2-S2 to US2-S5
crates/finder/tests/console/main.rs             # US2: mod saturated_field;
fixtures/finder/tycho-pleiades.csv              # US2, new: 203 Tycho stars (owner approval 7)
crates/scope-node/README.md                     # US3: ## Bright stars: saturated stars
crates/finder/README.md                         # US3: one paragraph
GLOSSARY.md                                     # US3: Saturated star
crates/plate-solver/examples/solve_image.rs     # US3: --saturated
crates/scope-node/tests/saturated_readme.rs     # US3, new: US3-S1
```

## Interfaces

### `crates/star-detection/src/saturated.rs` (US1)

```rust
pub const SATURATED: u8 = 250;
pub const MIN_PIXELS: usize = 5;
pub const MAX_SATURATED_PIXELS: usize = 24_576;
pub const MAX_AXIS_RATIO: f64 = 1.5;
pub const TRAIL_TOLERANCE_DEG: f64 = 10.0;
pub const TRAIL_MIN_REGIONS: usize = 3;
pub const RING_MARGIN: usize = 3;
pub const MAX_RING_SHARE: f64 = 0.5;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SetAside { Small, Edge, Elongated, Extended }

pub fn with_saturated_stars(image: &[u8], width: usize, height: usize, stars: Vec<Star>)
    -> Vec<Star>;
pub fn saturated_stars(image: &[u8], width: usize, height: usize, stars: &[Star])
    -> (Vec<Star>, Vec<SetAside>);
```

`saturated_stars` asserts `image.len() == width * height`, as `detect_stars` does. In
order:

1. The count of pixels at `SATURATED` or above: none answers two empty lists; more than
   `MAX_SATURATED_PIXELS` answers no star and `[SetAside::Extended]`, before any region
   is followed.
2. The median: of the pixels at indices 0, 64, 128 and so on, the least level at which
   the count so far passes half the samples.
3. Each region: from each pixel at `SATURATED` or above not yet visited, in reading
   order, an 8-connected flood over such pixels, summing its count n, x, y, x², y² and
   xy (pixel indices), its box (least and greatest x and y) and its brightest pixel.
4. A region of n = 1, or whose box, read as `left ≤ s.x ≤ right + 1` and
   `top ≤ s.y ≤ bottom + 1`, holds any star `s` of `stars`, is passed over: neither kept
   nor set aside.
5. `Small` when n < `MIN_PIXELS`.
6. `Edge` when `left < RING_MARGIN`, `top < RING_MARGIN`,
   `right + RING_MARGIN ≥ width` or `bottom + RING_MARGIN ≥ height`.
7. With the mean (cx, cy) and variances vxx, vyy, vxy about it, the eigenvalues
   (vxx + vyy)/2 ± √((vxx − vyy)²/4 + vxy²): `Elongated` when the minor is not above 0;
   long when √(major/minor) exceeds `MAX_AXIS_RATIO`, its major axis at
   ½·atan2(2·vxy, vxx − vyy) in degrees.
8. The ring is the one-pixel perimeter of the box widened by `RING_MARGIN` on each side;
   it is bright when its mean less the median exceeds `MAX_RING_SHARE` × (250 − median).
   A region not long is `Extended` when its ring is bright and kept otherwise.
9. The frame's trail: the axial median of the axes of the regions found long at step 7
   (none passed over or set aside before it): with m the circular mean of their doubled
   angles, (m + the median of each doubled angle less m, taken into −180° to 180°) / 2,
   of an even count the mean of the two middle deviations; it stands when at least
   `TRAIL_MIN_REGIONS` of them lie within `TRAIL_TOLERANCE_DEG` of it, the angle between
   two axes being half the doubled difference taken into −180° to 180°. A long region
   within that tolerance of a standing trail is `Extended` when its ring is bright and
   kept otherwise; every other long region is `Elongated`, whatever its ring.
10. Set-aside reasons are given in the reading order of each region's first pixel. A
    kept region becomes `Star::new(cx + 0.5, cy + 0.5, peak, brightness, n)`, peak its
    brightest pixel and brightness the sum over the pixels strictly inside the ring of
    (pixel − ring mean), which `Star::new` holds at 0 or above; the kept stars are sorted
    as `Star` orders them, brightest first.

`with_saturated_stars` returns `stars` itself when none is kept; else, walking `stars` in
order, each kept star goes before the first star of `stars` whose brightness is below
its own, and those left after the last go at the end. The module doc says what cedar's
gates set aside and cites cedar-detect's caveat and tetra3's extraction.

### `crates/finder/src/finder_focus.rs` (US1)

The constant keeps its name and value, and becomes the pass's; it is 051's file (US1
writes it, a shared write with 051's US2):

```rust
/// A star with a joined pixel at or above this level is not measured (US1): the finder
/// camera's one saturation level, the scope node's detection's too (052).
pub const SATURATED: u8 = star_detection::saturated::SATURATED;
```

### `crates/scope-node/src/detect.rs` (US2)

`detect` keeps its two branches, then passes their `stars` through
`star_detection::saturated::with_saturated_stars(&frame.pixels, width, height, stars)`
before `.take(200)` and the map to `Centroid`. Its doc line reads: "The port supplies
gray pixels; star-detection supplies brightness order, the saturated stars cedar sets
aside among them (052)."

### `crates/finder/src/solve.rs` (US2)

In `solve_frame_with_frame`, the stars `detect_stars` gives (`crates/finder/src/solve.rs:392`)
pass through `star_detection::saturated::with_saturated_stars(&image.pixels, image.width,
image.height, ...)` before the map to places, with the comment "With the saturated stars
cedar sets aside, as the scope node detects (052)." `solve_centroids` and every other
function of the file stay as they are.

### The Tycho fixture (`fixtures/finder/tycho-pleiades.csv`, US2)

The 203 stars of the kit's narrow Tycho database within 1.3° of the Pleiades' field, as
its rows were read (spec.md owner approval 7); the story copies the file exactly, its
comment line, header and rows, 205 lines and 5,329 bytes, ending in a newline:

```text
# 203 stars of the narrow Tycho database tyc_1p6_1p85 (tyc_main, proper motion to 2026.9) within 1.3 deg of RA 56.75 Dec 24.12, brightest first
ra_deg,dec_deg,mag
56.871297,24.104827,2.88
57.290698,24.053063,3.63
56.219140,24.113047,3.71
56.456882,24.367393,3.88
56.581724,23.948031,4.17
56.302260,24.467010,4.30
57.296893,24.136328,5.06
57.086895,23.420857,5.45
56.201112,24.289177,5.46
56.290744,24.838946,5.66
56.477130,24.554212,5.77
57.431509,23.711485,6.18
56.837904,24.116040,6.29
56.512261,24.527602,6.44
57.027012,24.988220,6.47
57.237682,23.856727,6.48
57.340756,24.380534,6.61
57.492086,23.848324,6.81
56.872882,24.288001,6.83
56.747582,24.519751,6.83
56.453545,23.146861,6.89
57.718609,23.961152,6.96
57.125476,24.345128,6.96
56.821119,23.726546,6.98
56.837487,23.803047,7.03
56.407660,24.335334,7.19
56.851886,23.914371,7.30
56.613877,24.254591,7.38
57.617099,24.495186,7.45
55.930216,25.080427,7.54
57.320047,24.395728,7.54
57.485959,24.348632,7.55
56.850359,24.588160,7.70
56.861940,23.677925,7.79
56.465256,24.038514,7.92
55.923212,23.648693,7.95
57.358433,24.247402,8.00
56.496606,25.398248,8.05
56.001162,24.556685,8.07
57.420622,23.341103,8.13
56.642489,23.623693,8.14
57.300902,23.886561,8.15
56.107409,24.394176,8.18
56.845530,22.921801,8.19
56.358975,24.034606,8.22
55.930380,24.374095,8.22
56.765006,24.819651,8.30
56.945302,25.385424,8.30
56.567254,24.189530,8.34
57.183064,23.259347,8.38
57.056556,24.318219,8.41
56.557173,24.196543,8.44
56.342797,23.204429,8.48
56.830855,24.138857,8.52
56.904358,23.608781,8.54
57.109834,25.322587,8.56
56.553068,23.128100,8.67
56.277088,24.263182,8.70
56.767738,23.994830,8.71
56.084422,24.874806,8.82
57.863628,24.518089,8.83
56.925057,23.543353,8.88
56.860311,24.658457,8.90
56.393891,24.462953,8.94
56.272436,23.702331,8.98
56.213577,23.268745,9.01
56.337012,24.921680,9.03
56.818284,25.293236,9.03
56.097641,24.131699,9.06
57.294717,23.267063,9.09
57.386177,23.379519,9.14
57.070219,25.214779,9.20
57.477954,24.218082,9.22
57.565353,24.666707,9.23
57.797464,24.593054,9.37
58.028946,23.586023,9.39
56.666801,23.109943,9.39
57.760922,23.428242,9.40
55.377968,23.801686,9.42
56.659838,22.919313,9.43
57.415328,23.055090,9.45
57.300438,23.408645,9.46
55.729771,24.492864,9.46
55.808581,23.723665,9.49
57.145379,24.180726,9.53
55.994714,23.882175,9.54
55.928106,23.594228,9.58
56.792022,24.275698,9.58
57.441467,24.245860,9.61
57.191520,23.525332,9.61
56.425757,24.905610,9.63
56.984022,23.191687,9.64
56.497587,25.051544,9.65
56.200719,23.589593,9.66
56.664356,24.102879,9.75
57.030522,24.141908,9.77
56.308906,23.121896,9.78
56.975406,25.129992,9.79
56.790813,23.782469,9.81
56.351089,24.885734,9.82
56.633548,24.870399,9.84
56.417550,24.626483,9.86
57.390323,24.628581,9.86
57.487319,23.952898,9.88
56.423240,24.287975,9.91
57.471855,23.529986,9.93
56.655999,23.787251,9.95
56.525180,23.218502,9.95
56.710613,23.239377,9.96
56.169618,24.817999,9.96
56.452835,23.148155,9.97
56.717017,23.211360,9.97
56.979149,23.436659,10.00
55.981945,24.251217,10.05
57.987582,23.901477,10.07
56.602386,25.211266,10.10
56.608707,25.019470,10.13
56.542174,23.339947,10.18
57.109349,24.048965,10.19
56.523832,24.992842,10.21
55.691887,24.271772,10.23
57.643224,24.791425,10.23
57.485297,23.217958,10.24
57.762882,23.901984,10.25
57.019754,23.381785,10.25
56.989797,25.012800,10.27
56.662020,24.959818,10.27
56.399304,24.082883,10.28
55.965218,23.222874,10.30
55.732346,23.876151,10.31
56.054710,24.403052,10.31
56.411164,22.942634,10.34
56.969522,23.941034,10.34
55.702440,24.481973,10.36
57.834845,24.381509,10.37
57.069943,23.635839,10.38
56.041012,24.246345,10.41
57.071599,23.890907,10.42
56.601798,23.322053,10.42
56.083097,24.795935,10.42
56.715644,25.126048,10.42
58.108907,23.861305,10.43
58.071102,24.399900,10.45
56.870525,24.556511,10.46
57.667205,23.933954,10.47
57.151055,25.330453,10.47
55.585343,24.468383,10.48
55.999556,24.146810,10.49
55.947206,23.211213,10.49
56.994165,24.100457,10.53
56.878516,25.254276,10.53
57.419105,24.038436,10.55
56.574757,23.487304,10.58
56.234327,24.433550,10.58
56.725520,23.583487,10.59
56.403948,24.901372,10.59
56.730759,24.796172,10.59
56.763564,24.637496,10.59
56.069238,24.270609,10.62
55.908032,23.534974,10.64
56.353873,23.190125,10.64
55.951107,25.190486,10.64
57.299195,24.636591,10.64
57.640533,24.347270,10.67
58.035953,24.230316,10.69
56.265944,25.257457,10.69
56.298650,22.953030,10.70
57.813843,23.621115,10.70
56.015631,24.989210,10.70
55.852246,23.226492,10.72
57.142896,23.434855,10.72
57.704109,24.429469,10.74
56.697322,22.914574,10.75
55.844644,23.289680,10.76
56.525679,25.016314,10.77
56.920102,24.363891,10.79
55.646671,24.538033,10.80
56.593339,24.569782,10.83
56.225093,23.920087,10.84
57.105620,24.877291,10.85
57.273213,23.780233,10.87
57.202865,24.633849,10.87
56.614612,25.135890,10.88
55.530039,23.528088,10.90
57.012190,25.340731,10.92
56.630280,25.323329,10.92
56.535112,22.979079,10.93
56.017141,24.502810,10.96
56.058583,24.779436,10.96
57.591513,24.466043,10.97
57.359546,24.938262,11.01
57.620562,24.516634,11.05
55.883327,23.674380,11.06
55.800610,23.603433,11.09
55.753041,24.545253,11.15
56.002859,23.544550,11.18
56.953365,23.217459,11.21
57.073566,25.047241,11.27
56.435510,24.715162,11.33
56.313441,25.289272,11.33
56.944107,25.286725,11.33
57.369539,23.708758,11.43
55.955085,24.249472,11.46
```

### The node's README (`crates/scope-node/README.md`, US3)

Inserted before `## The mount link`, exactly:

````markdown
## Bright stars: saturated stars

A bright star saturates the finder camera: its brightest pixels read 250 or more, at or
near the most an 8-bit frame holds, and its top is flat. cedar-detect's gates take a flat
top only a few binned pixels wide and set a wider one aside, so the brightest stars of a
frame, the ones the star patterns are built from, went missing, and on a field night the
Orion Nebula's field gave `no_match` with tens of stars detected.

The node's detection keeps them. After cedar-detect's own pass it finds each region of
saturated pixels, and keeps one as a star when cedar-detect found no star in it and it is:

- at least 5 pixels, tetra3's smallest spot;
- round, its length at most 1.5 times its width: two stars run together are longer, and
  so is a trail, which is kept when at least 3 of the frame's long regions lie within 10
  degrees of their common direction, as every star's trail does while the mount is still;
- dark around it, the ring 3 pixels out from it averaging less than 0.5 of the way from
  the frame's median to 250: the bright core of a nebula is not;
- whole, that ring inside the frame.

A frame holding more than 24576 saturated pixels, the Moon, a white frame or a twilight
sky, is set aside whole, and the node sends cedar-detect's stars as before.

Each is centred on the mean of its saturated pixels, its brightness the sum of its box
inside the ring less the ring's mean, and placed before the first of cedar-detect's stars
that is fainter, so the list stays brightest first and cedar-detect's stars keep their own
order; the node keeps the first 200 as before. A star frame does not say which centroids
are saturated, nor how many regions were set aside.

The console's Solve page detects an uploaded frame as the node does. `solve_image`
detects as cedar-detect does; to replay a raw frame as the node detected it, here or when
finding the noise floor's value, add `--saturated`, which also prints how many saturated
regions were kept and set aside, and why. It does not combine with `--dark`: the node
subtracts no dark.

```sh
cargo run --release --locked -p plate-solver --example solve_image -- \
  --db <the-database-the-console-reports> --saturated <frame>
```

On drawn frames at the finder camera's own 1.617 degrees over a narrow database of Tycho
stars around the Pleiades, untrailed and trailed 9 pixels, and on the simulator's Orion
Nebula and Pleiades fields at the finder camera's pixel grid at a 10.5 degree field, the
node's detection solves where cedar-detect's alone gives `no_match`. Whether a real finder
camera frame does is a field night's check.
````

### The console's README (`crates/finder/README.md`, US3)

Before the line `` `GET /api/live` is `text/event-stream`. `` and a blank line, exactly:

```markdown
The live solve takes the scope node's centroids as they come, with the saturated stars
cedar-detect sets aside among them ([scope node README](../scope-node/README.md), Bright
stars: saturated stars); a frame uploaded on the Solve page or through MCP is detected the
same way.
```

### The glossary (`GLOSSARY.md`, US3)

After **Centroid**'s entry and its blank line, before **Pattern**, exactly:

```markdown
**Saturated star**:
A star whose brightest pixels read 250 or more, the finder camera's one saturation level, so its top is flat. 051's focus aid measures none; the scope node's detection keeps one that cedar-detect's gates set aside, by its brightness, when it is round or lies along the frame's trail, dark around and whole in the frame.
_Avoid_: blown star, overexposed star
```

### `solve_image` (`crates/plate-solver/examples/solve_image.rs`, US3)

`Args` gains `saturated: bool`, set by `--saturated`; after the arguments are read,
`--saturated` with `--dark` calls `usage("--saturated does not combine with --dark: the
scope node subtracts no dark")`. The usage line reads
`... [--noise-floor F] [--saturated] [--stars N] ...`, the doc block's synopsis lists
`[--saturated]` after `[--noise-floor F]`, and its option list gains, after
`--noise-floor`'s two lines:

```text
//! * `--saturated` — keep the saturated stars cedar sets aside, by their brightness, as the
//!   scope node does (`star_detection::saturated`), and print how many regions were kept
//!   and set aside. It does not combine with `--dark`: the node subtracts no dark.
```

With the flag, after detection: `saturated_stars(&img, w, h, &stars)` gives the counts
of kept stars and of each reason, in the order small, edge, elongated, extended, and
`with_saturated_stars(&img, w, h, stars)` becomes `stars`. Without `--json`, before the
result line, it prints `<path>: saturated stars kept <k>, set aside <a> (small <s>, edge
<e>, elongated <l>, extended <x>)`; with `--json`, the object gains
`,"saturated_kept":<k>,"saturated_set_aside":<a>` after `t_solve_ms`. Without the flag
nothing it prints changes.

## What was run and what was read

The probe, on `probe/052` cut from f6d4f5fd, in the operator's probe folder's `logs/`
(not in the repository), with the factory's toolchain (rustc 1.100.0, nextest 0.9.152)
and `--locked`. The draft's commits (logs p0 to p17):

- **72559cc6**, a sweep (throwaway) on `main`'s detection: the simulator's finder camera
  on the Orion Nebula's and the Pleiades' fields, 1280 × 960 at 10.5°, 12° and 15°,
  focus errors 0 to 3, exposures 200 to 2000 ms and gains 1 to 16 (log p0). At focus
  error 3, 500 ms, gain 2 and 10.5°, the Orion field gave 69 centroids, none within 3 px
  of its 10 brightest drawn stars, and `no_match`; the Pleiades', 56 and `no_match`.
- **00e6a018**, the first trial (a ring at twice a region's radius): every frame of the
  sweep that gave `no_match` matched (log p1); the whole workspace failed only the saved
  finder camera frame's test (log p2).
- **28fa8572**, the measurement sources (throwaway): drawn objects through four rings
  (logs p3, p4); 5 fields at 10.5° and 12°, focus errors 1 to 3, 200 to 750 ms at gain 1
  or 2 (logs p5 to p7): 520 kept stars and both centroids (owner decision 2), 115 frames
  with a kept star and three ranks; 120 frames of the simulator's default fields solved
  with and without the pass; the cost as a ratio of the node's detection to cedar's
  alone: 1.02 on frames with no pixel at 255 and 1.11 to 1.39 on the bright frames.
- **5a0b5f95** to **d78b8fce**: the draft's two stories, every scenario passing (logs p8
  to p17); cedar's 13 reference images through the pass (one kept star, on
  `test_5mp_g100_e100ms.jpg` at binning 1 only, log p8); `solve_image --saturated` by
  hand (log p13).

The fold's commits, after cynical review round 1 (logs `fold-r1-*`):

- **4377e15e** and **2c9d4568**, a measurement example (throwaway, never a story's):
  frames drawn at the finder camera's geometry (1280 × 960 at 1.617°) over the kit's
  narrow Tycho database (fields 1.6° to 1.85°) for the Orion Nebula's field, the
  Pleiades' and a control field with no star brighter than 8.6: Gaussian σ 1.3, Moffat
  α 2.5 with β 2.5 and 1.8, saturating from magnitude 7 and 8.5, trails 0, 4.5 and 9 px
  at 20°, four nebulae on the Orion field, shot and read noise; the simulator's bright
  frames; US1-S1 to S3's drawn objects; the saved frame; the night's stretched preview
  (a proxy). A candidate pass with every gate a parameter equalled the module on 224
  frames with the drafted values (log m0) and the recommended ones (log m8). It
  measured the trail options (m1, m2), the levels 255, 250 and 240 and the connectivity
  (m3), the small regions and 5 pixels against 9 (m4), the largest star tops, the
  Moon, a planet, a white frame, twilight and the cost as ratios (m5), a narrow database
  built from 203 Tycho stars around the Pleiades under three noise models (m6, m7), the
  drafted rule against the recommended one on every frame (m9), and decision 4's two
  ranks (m10, with **16664ba0**). The numbers are spec.md owner decisions 1 to 4 and 7's.
- **1c499ebf**, **d67a8214** and **840e7ca1**: US1, US2 and US3 as the stories would
  write them (log p1). With every kept star first, the workspace failed 013's
  `ten_dense_frames_have_all_fields_top_200_and_ids_count_dropped_exposures` at its
  brightness order (`crates/simulator/tests/node.rs:111`, log p3); **50286987** places
  each kept star by its brightness (decision 4 (b), log p4). **b971a8de** and
  **13c17ed4** drop the example (it stays in history); **62b814ef** makes two doc lines
  plainer.
- At **13c17ed4**: `lint`'s commands exit 0 (log p5) and `test`'s Rust commands pass,
  1535 of 1535 and the doc tests (log p6). At the tip, **62b814ef**, the three stories'
  files only, again: `cargo fmt --all --check`, clippy with `-D warnings`, both
  `cargo doc` runs (log p9); 1535 of 1535 (1524 existing unchanged, one replaced, ten
  new) and the doc tests (log p10). The npm half was not run: no story writes the page.
- Fails before (log p8): US1's test does not build at f6d4f5fd (no module), and on the
  draft's module US1-S1, US1-S4 and US1-S5 fail; with US1 alone (the node's `detect.rs`
  and the console's `solve.rs` as at f6d4f5fd), US2-S1 to US2-S5 fail; with US1 and US2
  alone, US3-S1 fails on the missing section.
- Mutations, each once and reverted (log p7): the level 255 fails US2-S1; 9 pixels fails
  US1-S1; a bound of 30,000 fails US1-S5; the trail rule off fails US1-S4 and US2-S4; a
  tolerance of 30° fails US1-S4; every kept star first fails US1-S1 and 013's
  dense-frame test; a ring share of 0.9 fails US1-S2; the node's call removed fails
  US2-S1 to US2-S5; the console's call removed fails US2-S5; the `--dark` refusal removed
  fails US3-S1.
- The cost, as a ratio of the pass to cedar's detection of the same frame (log m5): a
  white frame about 145 as drafted and 0.2 with the frame bound; a drawn Moon about 7.4
  and 0.13; twilight about 4.9 and 0.2.

Probe round 1, from this trio alone, on `probe/052-r1` cut from 7d306e7e (the trio on
f6d4f5fd), its logs beside the first probe's, the owner decisions built to their
recommendations:

- **986f34b0** to **491b7655**: each story's tests, then its code (US1 26be56b0, US2
  f1c92069, US3 491b7655), 15 files, exactly the three `writes` lists. Every scenario's
  test passed on the first build of its story's code but US2-S4, whose database was
  first built in degrees (trap 6). Before each story: US1's test does not build; US2-S1
  is one centroid short, US2-S2 lacks a bright star in the node's first 10, US2-S3 to
  US2-S5 answer `NoMatch`, and with the node's call alone US2-S5's places are unequal;
  US3-S1 finds no section. Each focused command passed three runs in a row. At
  491b7655 the `lint` gate's commands exit 0 and `test`'s Rust commands pass, 1535 of
  1535 and the doc tests.
- Mutations, each once and reverted: every gate and rule of FR-001 failed a scenario
  but the choices § Read from the diff lists and five that are steps now; with the
  fixture's units, its cached database, T007's claim of what fails and that list, 0
  blocking and 10 clarity defects, each folded here.

The fold of probe round 1, one commit on `probe/052-r1` (logs `fold-probe-r1-*`):

- **ae80ff09**, US1's test only: US1-S1's brightness, US1-S2's top cedar keeps, pair
  under a halo and touching blocks, and US1-S4's trail at the edge, 3,568 B of diff. Each
  step passed and failed under one mutation of the module, each reverted: a region
  holding a star of cedar's kept (US1-S2), the brightness over the region's own box
  (US1-S1), 4-connectivity (US1-S2), an edge region's axis counted toward the trail
  (US1-S4), and a long region with a bright ring and no trail set aside as `Extended`
  (US1-S2).
- At **ae80ff09**: `cargo fmt --all --check`, clippy with `-D warnings`, both
  `cargo doc` runs, 1535 of 1535 (the steps are in US1's five tests, so the count is
  unchanged) and the doc tests; `cargo deny` offline and `cargo machete` clean; each
  story's focused commands pass.

Read, not run: the field night's frames (only a 640 × 480 preview survives); the INDI
finder camera adapter's 255 at full scale, at every gain; the Mini's hot pixels; the
pass's cost on the Pi; every `path:line` cited, at f6d4f5fd.

## Read from the diff, not from a test

Probe round 1 found five choices of FR-001 no scenario saw; each is now a step that its
mutation fails (§ What was run): the passed-over rule and 8-connectivity (US1-S2's top
cedar keeps and its touching blocks), a kept star's brightness (US1-S1), a long region
with a bright ring and no trail (US1-S2's pair under a halo) and which regions make the
trail (US1-S4's trail at the edge). What no scenario sees:

- FR-002, beyond US1-S1's check that cedar's list still sets each drawn saturated star
  aside: `crates/star-detection/src/detect.rs` and `crates/star-detection/src/binning.rs`
  are not in the diff, and the parity tests hold their results.
- FR-001's set-aside order beyond each scenario's; the sampled median beyond US2-S1's
  saved frame; the trail's median of an even count (the mean of the two middle
  deviations, where the lower middle passes every scenario); the bound's strict "more
  than" (no frame holds exactly 24,576 pixels at 250 or above); the passed-over box's
  bounds (`left ≤ s.x ≤ right + 1`, step 4) beyond a star at its middle; each ring pixel
  counted once; a minor of exactly 0 (`Elongated`); a kept star placed after cedar's
  stars of equal brightness; and `peak_value` beyond 255, the only top drawn. The probe's
  mutations of the set-aside order, the even median, the bound, the ring's corners, the
  minor, the ties and `peak_value` each passed every test (run).
- T006's fixture: its 205 lines equal § Interfaces' text byte for byte; no scenario reads
  its rows beyond solving with them, so a row dropped or changed passes every test (run).
- FR-003: `finder_focus.rs`'s one line; 051's focus tests pass unchanged, and no test
  can tell one constant from two of the same value.
- FR-004's doc line, FR-006 (the diff's file list), and FR-007's paragraph of the
  console's README and the README section's prose beyond US3-S1's phrases.
- FR-009's behaviour: examples are compiled and linted by the gates and never run
  (048's precedent for `--noise-floor`); US3-S1 reads its usage line, its doc line and
  its refusal's words.

## Shapes that existing tests pin

With all three stories built, every existing test but the replaced one passed (log p10),
and again at probe round 1's 491b7655 and its fold's ae80ff09. The files beside the
change:

- **`crates/star-detection/tests/parity.rs`** and **`noise_floor.rs`**: call
  `detect_stars` and `detect_stars_with_noise_floor`, which do not change.
- **`crates/scope-node/tests/detect.rs`**: `cedar_defaults_equal_every_console_field`
  (`crates/scope-node/tests/detect.rs:10`) compares named fields;
  `crowded_frame_keeps_the_detectors_brightest_two_hundred_in_order`
  (`crates/scope-node/tests/detect.rs:68`) draws peaks of 108, so the pass returns
  cedar's list; the saved finder camera frame's test (`crates/scope-node/tests/detect.rs:26`)
  is replaced (US2-S1).
- **`crates/simulator/tests/node.rs`**: the dense frame's centroids must stay in falling
  brightness (`crates/simulator/tests/node.rs:111`), as the star frame says
  (`crates/scope-link/proto/scope_node/v1/scope_node.proto:64`); its frames hold small
  saturated tops the pass keeps, so it passes only because each kept star is placed by
  its brightness (decision 4 (b)).
- **`crates/scope-node/tests/raw_frame.rs`** (`crates/scope-node/tests/raw_frame.rs:68`,
  `crates/scope-node/tests/raw_frame.rs:95`) and
  **`crates/finder/tests/console/recording_raw.rs`**
  (`crates/finder/tests/console/recording_raw.rs:209`) hold the node's streamed or
  recorded centroids to `detect_stars`; their simulated frames hold no saturated star the
  pass keeps, so they pass unchanged (run). A future frame that does would fail them:
  plan trap 3.
- **`crates/finder/tests/console/star_detected.rs`** (`crates/finder/tests/console/star_detected.rs:38`)
  and **`crates/finder/tests/mcp/frames.rs`** (`crates/finder/tests/mcp/frames.rs:71`)
  solve cedar's sample `orion2.jpg` through `solve_frame`; the pass keeps nothing there,
  so they pass unchanged (run).
- **`crates/simulator/tests/dark_sky.rs`**, **`dark_sky_exposure.rs`** and **`sky.rs`**
  detect with `scope_node::detect::detect`: they pass unchanged (run).
- **051's `crates/finder/tests/console/finder_focus_measure.rs`** and
  **`finder_focus_aid.rs`** measure at the node's centroids with `SATURATED` 250: they
  pass unchanged (run).

## Decisions and the alternatives rejected

The options of spec.md § Owner decisions, with the probe's measures:

- **Where** (decision 1): (d) the node and the console's `solve_frame_with_frame`; (a)
  the node only leaves an uploaded finder camera frame `no_match`; (b) inside
  `detect_stars` moves the parity image `test_5mp_g100_e100ms.jpg` at binning 1; (c) a
  switch adds a config key, a `--check-config` line and Configure's handling for nothing
  to switch off.
- **Centroid** (decision 2): (a) the mean of the saturated pixels; (b) flux-weighted
  inside the ring, its tail pulled by neighbours (90th percentile 1.15 px against
  0.29 px); (c) a wing fit, unreferenced.
- **Gates** (decision 3): the level 250 shared with 051's aid, measured equal to 255 and
  240 on drawn frames and better than 255 on the proxy; 8-connectivity, equal to 4 on
  drawn frames; 5 pixels, since the small regions were stars cedar missed; the frame
  bound, above every star frame drawn and below the Moon and twilight; the ring 3 px out
  with 0.5. A per-region cap in place of the bound solves a drawn Moon beside stars but
  follows a white frame pixel by pixel, and with a frame share too is two constants;
  tetra3's area cap refuses the brightest tops.
- **Rank** (decision 4): (b) by brightness, the same solves as every kept star first and
  the star frame brightest first; (a) breaks that order on 38 of 53 and 48 of 116 frames
  with a kept star and fails 013's dense-frame test; (c) last, 79 of 115 (the draft's).
- **The report** (decision 5): a star frame field is a protocol change and a write to
  `crates/finder/src/live.rs` while 053 changes it.
- **Exposure hint** (decision 6): later, if a field night shows the need.
- **Trails** (decision 7): (b) the trail-aware ratio solves 10 of 12 frames trailed 9 px
  against 2, changing nothing untrailed; (a) leaves the 3 s failures to an exposure rule;
  (c) a looser ratio keeps merged doubles.

## User Story 1 - The pass

Tests first: `crates/star-detection/tests/saturated.rs` fails to build without the
module. Then the module and its `lib.rs` line, then `finder_focus.rs`'s constant.

## User Story 2 - The node and the console

Tests first: the replaced test and `saturated_field.rs` (with its `mod` line and the
fixture) fail with US1 merged but without the two calls (run). Then the call in
`detect`, its doc line, and the call in `solve_frame_with_frame`.

## User Story 3 - The words and the replay

The README test first (it fails on the missing section), then the README, the console's
README's paragraph, the glossary and `solve_image`.

## Verification (every story, before proposing)

- `cargo fmt --all --check`, then the `lint` gate's clippy and doc commands.
- The story's development commands (spec.md, each story's Independent Test), then the
  `test` gate.
- Every existing test unchanged and green but US2-S1's, which US2 replaces.

## Prerequisites before this spec is flipped to `ready`

- Every spec in `depends_on_landed` has landed: met at f6d4f5fd, where 013, 025, 045,
  047, 048 and 051 have all landed.
- `main` is still as read, or the cites are re-read: met, `main` at f6d4f5fd at this
  fold and at the flip, every cite read there.
- A review round of this trio, folded: met, cynical review round 1 (0 blockers, 2 majors,
  11 minors) folded at 7d306e7e.
- The seven owner decisions answered: met, by the owner on 2026-11-27, each with its
  recommendation ("Flip 052 now").
- A probe round that builds the three stories from this text alone: met, round 1 at
  491b7655 (0 blocking, 10 clarity), folded here with ae80ff09 (§ What was run).

## Outside the factory

Written, not run. On the next field night, with the kit holding this spec, and the
finder camera at gain 80 and sigma 8 as on the last field night unless a step says:

1. **Track on.** Point at the Orion Nebula's field, then the Pleiades', each with its
   brightest stars saturated. At each of 1000, 1500 and 3000 ms, let about 20 live
   reports pass and note how many are `match_found` and the detected counts.
2. **Track off, the mount still** (the first sync and a GoTo's settling). On the
   Pleiades' field, at 1500 and 3000 ms, the same 20 reports each; the stars trail about
   4.5 and 9 px, decision 7's case. Until this step has passed, keep the finder camera's
   exposure at or under 1.5 s while the mount is still.
3. **A low gain.** On one bright star, at the lowest gain the camera takes and an
   exposure that saturates it, record raw frames and note whether its top reads 250 or
   more; if it clips below, the pass does nothing at that gain (spec.md § Edge Cases).
4. At each setting, record 3 raw frames with `POST /api/record/raw` (the console's
   README's Recording a night), as future fixtures; note exposure, gain, Track and the
   field beside the recording.
5. On the console's computer, replay each with `solve_image --saturated` over the
   database the console reports, and note kept and set-aside counts and reasons; upload
   one to the Solve page and see it match as the live loop did.
6. If a field still gives `no_match` with its brightest stars kept, or a nebula's core,
   a planet or a pair is kept and leads the list, bring the recordings back: the next
   spec's probe replays them through `scope_node::detect::detect`.

## Plan traps

1. **Do not touch `detect_stars`.** The pass changes a parity image at binning 1
   (§ What was run); inside `detect_stars` it would move cedar's parity and the e2e
   evaluation's inputs. Mechanism: the reference images do hold saturated stars. Check:
   `crates/star-detection/src/detect.rs` is not in the diff.
2. **The replaced test keeps its checks.** Principle II: the replacement keeps the
   fixture's size checks, the non-empty list and the exact equality of every centroid
   after the first; it adds only the saturated star. Check: US2-S1's words.
3. **Tests holding the node to `detect_stars`** (`raw_frame.rs`, `recording_raw.rs`) pass
   only because their simulated frames hold no saturated star the pass keeps. A story
   that changes the simulator's frames could break them; FR-006 keeps the simulator's
   `src/` out.
4. **Brightest first.** A kept star is placed by its brightness, never put first
   regardless: 013's dense-frame test (`crates/simulator/tests/node.rs:111`) holds the
   star frame's order and fails otherwise (run).
5. **The live test's first reports.** An exposure in flight when the frames stop finishes
   with the old optics (`crates/finder/README.md`, Simulated night: "An exposure already
   in progress finishes before a push or stop takes effect"), so its report is 1456 wide;
   US2-S3 counts only 1280-wide reports.
6. **US2-S4's database** is built once into `CARGO_TARGET_TMPDIR` as a complete file, a
   process-unique `.part` renamed into place, as 047's narrow test database is
   (`crates/finder/tests/console/databases/mod.rs:16`,
   `crates/finder/tests/console/databases/mod.rs:53`): each test runs in its own process.
   Its rows are read from the fixture, never from the kit's database file, which is not
   in the repository. The fixture's places are degrees and a `CatalogEntry` takes radians
   (`crates/database-generation/src/catalog.rs:19-22`): rows left in degrees draw a
   blank field frame and the node's centroids give `TooFew` (run). The test builds the
   file only when it is missing (`crates/finder/tests/console/databases/mod.rs:64`), so
   one built wrong stays for every later run: after changing how it is built, delete
   `052-tycho-pleiades.npz` from `CARGO_TARGET_TMPDIR` (`target/tmp`) first.
7. **Lines wrap in the README.** US3-S1 reads the section with its whitespace runs as one
   space; a phrase split across lines still matches.
8. **Names.** "finder camera", never "finder" alone; "scope node"; "saturated star" as the
   glossary gives it.
9. **No timing or place** in the README or a message: the field night is "a field night".
10. **Concurrent drafts.** Specs 053 and 054 are drafted beside this one and write the
    console's README, the glossary and the console's test binary's `main.rs`; neither
    writes `solve.rs`, `finder_focus.rs` or `crates/scope-node/src/detect.rs`. The floor
    orders them by `depends_on_landed`, and a story rebases on what landed.

## Risks

- **The fix needs nearly all of a field's missing bright stars.** Of the triage's 10 on
  the night's Orion frame, one lies at x 1.3 (`Edge`) and one is half of a merged double
  (`Elongated`); the 7 the pass keeps, put first, still match, but the 3 largest alone,
  or one, do not, and a nebula point put before the 7 still matches where one put before
  3 does not (the review's runs over the narrow Tycho database).
- **A real star's halo** (scattered light, reflections, wings heavier than the Moffat
  β 1.8 drawn) could brighten its ring past the share and set it aside as extended.
- **A nebula's compact saturated knot or a planet** in dark surroundings is kept, by its
  brightness often near the front; one such false star drawn first still solved (run),
  but several might not.
- **A trailed close pair** lying along the trail is kept as one star at its middle, about
  3 px from each (decision 7).
- **A clipped top below 250**, from a camera, a conversion or a low gain, gets no help.
- **The cost on the Pi** is measured only as a ratio on the factory's machine; a frame
  just under the bound follows up to 24,576 pixels.

## Refinement Record

Drafted on 2026-11-27 by the plate-solver operator seat's spec session against `main` at
f6d4f5fd, from a probe of that day on `probe/052`, and folded the same day with cynical
review round 1 (2 majors, 11 minors) from the fold's probe commits (tip 62b814ef: the
three stories built, every scenario's test and both gates' Rust commands passing, each
new test failing on the state before its story, each gate mutated once); probed from the
trio alone (round 1, `probe/052-r1` at 491b7655: 0 blocking, 10 clarity defects) and
folded with one commit of US1 steps (ae80ff09, each step failing under its mutation, the
gates' Rust commands passing); and a reading of
cedar-detect's gates and their port, the node's detection and its callers, the console's
frame detection and its callers, 048's dark-sky work and tests, the simulator's finder
camera, the console's live loop and solve, 025's raw frames, 047's narrow test database,
051's focus measure, tetra3's spot extraction, the constitution, the glossary and the
ADRs. Writes come from the task text. Shared writes name the spec story that last wrote
the same file on `main`; within the spec, US2 follows US1 and US3 follows US2 by
`depends_on_merged`.

The seven owner decisions of spec.md are answered, each with its recommendation
(2026-11-27). This record, like the scenarios, is
written to their recommendations (decisions 2 to 4 and 7 for US1, 1 and 5 for US2, 5 and
6 for US3), so its `open_questions` are empty; an answer other than a recommendation
would have reopened the record and the scenarios it names. This record approves nothing;
the owner flipped the spec on 2026-11-27.

```yaml
version: 1
reviewed_revision: f6d4f5fd482ccc9d5b634ef1aa5124c112769583
stories:
  US1:
    tests:
      - path: crates/star-detection/tests/saturated.rs
        status: new
        disposition: add
        assertion: "drawn 640 x 480 frames, background 10, one Gaussian at (p, p), p 200.5, 200.75, 201.0, sigma/P 1.5/3000, 3/3000, 3/30000, 5/3000, Moffats at (200.5, 200.5) alpha 2, P 25500, beta 2.5 and 1.8, and a Gaussian sigma 1.5 P 340 at (200.5, 200.5) (a top of five pixels), each with a Gaussian at (400.5, 300.5) sigma 1.5 P 100: saturated_stars with cedar's list (sigma 8, binning 2, rows normalised, hot pixels rejected) keeps one star within 0.1 px of its place, peak_value 255, num_saturated the frame's count of pixels at 250 or above, sets nothing aside; with_saturated_stars gives it then cedar's list unchanged, and a list of brightness 1e6 at (50.5, 50.5) and 10 at (600.5, 50.5) with it between them; cedar's list has a star within 0.5 px of (400.5, 300.5) and none within 3 px of the kept place; for sigma 3 P 3000 at (200.5, 200.5) the kept brightness within 1e-6 of the sum over the saturated box widened by 2 of each pixel less the mean of the perimeter of the box widened by 3"
        decision: "spec.md:US1-S1"
      - path: crates/star-detection/tests/saturated.rs
        status: new
        disposition: add
        assertion: "segments (150, 100) to (450, 340) at 400 within 1, 2, 3 px; Gaussian pairs sigma 3 at (300.5 -/+ d/2, 240.5), P 3000/3000 d 8, 10, 12 and P 3000/800 d 8, 12, and P 3000/3000 d 12 under a Gaussian sigma 40 P 200 at (300.5, 240.5); nebulae sigma 40 P h plus sigma s P c at (320.5, 240.5), (h, s, c) (200, 8, 400), (60, 8, 400), (0, 25, 400), (150, 3, 3000); three lobes at (300.5, 230.5), (330.5, 250.5), (315.5, 270.5) sigma 30, 25, 20 P 180, 180, 150; a Gaussian sigma 3 P 3000 at (5.5, 240.5); a Gaussian sigma 1 P 1000 at (200.5, 200.5), nine pixels cedar keeps: none kept; segments and pairs (the halo's too) [Elongated], nebulae [Extended], the edge star [Edge], cedar's kept top [] with cedar's list within 0.5 px of (200.5, 200.5); with_saturated_stars gives cedar's list unchanged; blocks of 255 with no stars: from (320, 240) 1 x 1 [], 2 x 2 and 4 wide 1 high [Small]; two 3 x 3 from (300, 240) and (303, 243), touching at a corner, [Elongated]"
        decision: "spec.md:US1-S2"
      - path: crates/star-detection/tests/saturated.rs
        status: new
        disposition: add
        assertion: "two Gaussians sigma 3 P 3000 at (300.5 -/+ d/2, 240.5): nothing set aside, every kept star within 0.1 px of y 240.5; d 4 and 6 one kept within 0.1 px of x 300.5; d 16 and 24 two kept within 0.1 px of 300.5 -/+ d/2"
        decision: "spec.md:US1-S3"
      - path: crates/star-detection/tests/saturated.rs
        status: new
        disposition: add
        assertion: "Gaussians sigma 1.5 P 3000 trailed 9 px at 20 degrees (the mean over 18 centres) at the first 5, 3 and 2 of (100.5, 100.5), (300.5, 150.5), (500.5, 100.5), (150.5, 350.5), (450.5, 380.5), each frame with an untrailed pair sigma 3 P 3000 at (295.5, 300.5) and (305.5, 300.5): 5 and 3 trails keep exactly their trailed stars within 0.1 px of their places and set the pair aside [Elongated]; 2 trails keep none, [Elongated, Elongated, Elongated]; trails only at the first 2 and (5.5, 240.5) keep none, [Elongated, Elongated, Edge]"
        decision: "spec.md:US1-S4"
      - path: crates/star-detection/tests/saturated.rs
        status: new
        disposition: add
        assertion: "a disc 1000 above background over every pixel centre within r of (320, 240), no stars: r 88 keeps one star within 0.1 px of (320, 240), nothing set aside; r 89 keeps none, [Extended]; a 640 x 480 frame all 255 keeps none, [Extended], and with_saturated_stars of it with no stars gives no star"
        decision: "spec.md:US1-S5"
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
      - path: crates/finder/tests/console/finder_focus_measure.rs
        status: existing
        disposition: preserve
        assertion: "051's focus measure skips a star with a pixel at finder_focus::SATURATED (250) or above, as before 052"
        decision: "spec.md:FR-003"
    preserved_behavior:
      - "detect_stars and detect_stars_with_noise_floor: code and results as at f6d4f5fd"
      - "finder_focus::SATURATED: name, value 250 and the focus aid's behaviour as at f6d4f5fd"
    checks:
      development:
        - "cargo nextest run -p star-detection --locked --test saturated"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs (e2e-eval among them)"
    writes:
      - crates/star-detection/src/saturated.rs
      - crates/star-detection/src/lib.rs
      - crates/star-detection/tests/saturated.rs
      - crates/finder/src/finder_focus.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/star-detection/src/lib.rs
        other_spec: 048-node-detects-stars-on-a-dark-sky
        other_story: US1
        disposition: merge_order
        reason: "052 depends_on_landed 048, whose US1 last changed lib.rs (ce7f675a)"
      - path: crates/finder/src/finder_focus.rs
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US2
        disposition: merge_order
        reason: "052 depends_on_landed 051, whose US2 last changed finder_focus.rs (f9706cc0)"
    open_questions: []
  US2:
    tests:
      - path: crates/scope-node/tests/detect.rs
        status: existing
        disposition: replace
        assertion: "saved_finder_frame_keeps_its_saturated_star_first_then_the_consoles_centroids_in_order replaces saved_finder_frame_has_identical_nonempty_centroids_in_console_order: fixtures/scope-node/finder-frame.png at the size fixtures.json gives; its pixels at 250 or above exactly (20, 186), (19-24, 187), (19-24, 188), (19-25, 189), (20-24, 190), (21-24, 191); detect_stars at DetectParams::cedar non-empty, no star within 5 px of (22.5, 189.5); detect with DetectSettings::cedar gives one more centroid, the first within 1e-9 px of (632/29 + 0.5, 5473/29 + 0.5), every later one equal to detect_stars' in order, x, y and brightness"
        decision: "spec.md:US2-S1"
      - path: crates/scope-node/tests/detect.rs
        status: existing
        disposition: preserve
        assertion: "cedar_defaults_equal_every_console_field and crowded_frame_keeps_the_detectors_brightest_two_hundred_in_order as before: the crowded grid's peaks of 108 hold no 250"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/console/saturated_field.rs
        status: new
        disposition: add
        assertion: "bright frames (SimCamera 500 ms gain 2, DB, Site::default, START, optics 1280 x 960 at 10.5 degrees eyepiece (640, 480), point_eyepiece_at, finder camera focus error 3) of the Orion Nebula (83.82, -5.39) and the Pleiades (56.75, 24.12): each of the first 5 drawn stars of field_stars(500, 2) at least 20 px inside has a node centroid (detect, DetectSettings::cedar) within 1.5 px among the first 10 and no centroid of cedar's list within 3 px; solve_centroids over DB with SolveOptions::default of cedar's list cut to 200 NoMatch, of the node's MatchFound within 30 arcsec of telescope().camera"
        decision: "spec.md:US2-S2"
      - path: crates/finder/tests/console/saturated_field.rs
        status: new
        disposition: add
        assertion: "AppState::shared over DB with simulation at START over SimulationTransport::Memory and the live loop, frames stopped, the simulator set as a bright frame's for the Orion Nebula, PUT /api/session camera_settings exposure_ms 500 gain 2 answered 200, frames started: the first three live reports 1280 wide each MatchFound, pointing within 60 arcsec of the simulator's camera direction read then, all within 30 s of START, under start_paused"
        decision: "spec.md:US2-S3"
      - path: crates/finder/tests/console/saturated_field.rs
        status: new
        disposition: add
        assertion: "a narrow database built in CARGO_TARGET_TMPDIR, only when its file is missing, from the 203 rows of fixtures/finder/tycho-pleiades.csv, degrees as the radians a CatalogEntry takes (clean_and_limit, enumerate_patterns 1.6 to 1.85 degrees, 150, 20, 50, build_pattern_catalog 0.001); field frames 1280 x 960 of field_stars(1e9, 1) of a simulator on it, optics 1.617 degrees eyepiece (640, 480), pointed at the Pleiades: background 10, peak 255 x 10^(-0.4 (m - 8.5)), Moffats alpha 2.5 beta 1.8 within 64 px, and Gaussians sigma 1.3 trailed 9 px at 20 degrees within 16 px; on each, solve_centroids over that database of cedar's list cut to 200 NoMatch, of detect's (DetectSettings::cedar, as a GrayFrame) MatchFound within 30 arcsec of telescope().camera"
        decision: "spec.md:US2-S4"
      - path: crates/finder/tests/console/saturated_field.rs
        status: new
        disposition: add
        assertion: "the Orion Nebula's and the Pleiades' bright frames, each encoded as a PNG, decode_gray then solve_frame over DB with SolveOptions::default, as the Solve page does: the report's first 200 detected places equal detect's centroids' (x, y) in order, and the report is MatchFound within 30 arcsec of telescope().camera"
        decision: "spec.md:US2-S5"
      - path: crates/simulator/tests/node.rs
        status: existing
        disposition: preserve
        assertion: "ten dense frames each carry 200 centroids in falling brightness through the scope link, as 013 words it, the kept saturated stars among them by their brightness"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/scope-node/tests/raw_frame.rs
        status: existing
        disposition: preserve
        assertion: "the node's streamed centroids equal detect_stars of the raw pixels on its simulated frames, which hold no saturated star the pass keeps"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/console/recording_raw.rs
        status: existing
        disposition: preserve
        assertion: "a recorded raw frame's line replays through detect_stars to its centroids on the simulated frames, as 025 words it"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/console/star_detected.rs
        status: existing
        disposition: preserve
        assertion: "cedar's orion2.jpg through solve_frame solves with its detections in brightness order, as 020 words it; the pass keeps nothing there"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/simulator/tests/dark_sky.rs
        status: existing
        disposition: preserve
        assertion: "048's dark and city skies through scope_node::detect::detect, counts and solves as 048 words them"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "a frame with no pixel at 250 or above, or none kept, gives the node and the Solve page exactly cedar's list, the node's cut to 200"
      - "the star frame, the scope link's protocol, DetectSettings and the node's config as at f6d4f5fd"
      - "solve_centroids, the live loop and every file of crates/finder/src but solve.rs and finder_focus.rs as at f6d4f5fd"
    checks:
      development:
        - "cargo nextest run -p scope-node --locked --test detect"
        - "cargo nextest run -p finder --locked --test console -E 'test(saturated_field)'"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs (e2e-eval among them)"
    writes:
      - crates/scope-node/src/detect.rs
      - crates/scope-node/tests/detect.rs
      - crates/finder/src/solve.rs
      - crates/finder/tests/console/saturated_field.rs
      - crates/finder/tests/console/main.rs
      - fixtures/finder/tycho-pleiades.csv
    no_write_reason: ""
    shared_writes:
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
      - path: crates/finder/src/solve.rs
        other_spec: 047-console-uses-the-database-that-fits
        other_story: US1
        disposition: merge_order
        reason: "052 depends_on_landed 047, whose US1 last changed the console's solve.rs (d3f8bd0f)"
      - path: crates/finder/tests/console/main.rs
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US2
        disposition: merge_order
        reason: "052 depends_on_landed 051, whose US2 last changed console/main.rs (f9706cc0)"
    open_questions: []
  US3:
    tests:
      - path: crates/scope-node/tests/saturated_readme.rs
        status: new
        disposition: add
        assertion: "crates/scope-node/README.md's section from ## Bright stars: saturated stars to the next ## heading, whitespace runs as one space, holds its brightest pixels read 250 or more, at least 5 pixels, tetra3's smallest spot, its length at most 1.5 times its width, which is kept when at least 3 of the frame's long regions lie within 10 degrees of their common direction, the ring 3 pixels out from it averaging less than 0.5 of the way from the frame's median to 250, whole, that ring inside the frame, A frame holding more than 24576 saturated pixels, placed before the first of cedar-detect's stars that is fainter, so the list stays brightest first, the node keeps the first 200 as before, A star frame does not say which centroids are saturated, nor how many regions were set aside., The console's Solve page detects an uploaded frame as the node does., --saturated <frame>, It does not combine with `--dark`, each number formatted from SATURATED, MIN_PIXELS, MAX_AXIS_RATIO, TRAIL_MIN_REGIONS, TRAIL_TOLERANCE_DEG, RING_MARGIN, MAX_RING_SHARE, MAX_SATURATED_PIXELS; GLOSSARY.md holds **Saturated star**: then a line beginning A star whose brightest pixels read 250 or more; solve_image.rs holds [--noise-floor F] [--saturated] [--stars N], a doc line beginning //! * `--saturated` - and --saturated does not combine with --dark"
        decision: "spec.md:US3-S1"
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
