---
state: draft
depends_on_landed: ["013-live-finder", "025-frame-capture", "045-node-runs-finder-over-indi", "048-node-detects-stars-on-a-dark-sky", "051-tools-page-focuses-either-camera"]
# Drafted 2026-11-27 against `main` at f6d4f5fd, at the owner's ask of 2026-11-27 ("Your
# rec. Proceed", on the seat's ranked list of a field night's defects, this first), from
# a probe of that day on the branch `probe/052` cut from f6d4f5fd (tip d78b8fce: a trial
# build of both stories with every scenario's test and both gates' Rust commands passing,
# the new tests failing with the pass turned off, the gates' candidates measured on drawn
# objects and on the simulator's bright fields; plan.md § What was run and what was
# read). Every `path:line` here was read at f6d4f5fd. Not reviewed yet; the owner
# decisions are open. 013 is named because its stories built the node's detection, the
# simulator's finder camera and the live loop, and last wrote
# `crates/scope-node/tests/detect.rs`; 025 because its raw frames replay through
# detection; 045 because its US2 last wrote the node's README and its INDI finder
# camera adapter reads the 8 bits whose top is 255; 048 because its stories last wrote
# the node's detection, star-detection's `lib.rs` and `solve_image`; 051 because it last
# wrote the console's test binary, the console's README and the glossary, and its
# simulated focuser widens the stars these tests saturate (plan.md § Refinement Record).
---

# Feature Specification: The Node Keeps Saturated Stars

**Feature Branch**: `052-node-keeps-saturated-stars`

**Created**: 2026-11-27

**Status**: Draft

**Input**: On a field night the finder camera (a ZWO ASI120MM Mini, 1280 × 960, about
1.62° wide, read as 8 bits by the scope node) gave `no_match` on the Orion Nebula's
field with 39 to 77 stars detected at 1 to 1.5 s, and on the Pleiades' and other fields
at 3 s with 54 to 185; fields holding 24 to 39 stars matched. So the aim point got no
readings, GoTo ended "no frame solved after settling" and Track "failed after 5
corrections". The seat's triage, offline: the frame's 8 brightest stars were missing
from the detected list; added back first, the frame solved at once, 30 of 30 matched.
Make the scope node's detection keep saturated stars, rank them first, and set aside
hot pixels, nebulae, trails and merged doubles; say so in the README.

## Clarifications

### Session 2026-11-27 (the probe on `main` at f6d4f5fd)

- Q: What does this trio read besides itself? → A: Nothing about the feature.
  `spec.md` holds every requirement and scenario, and `plan.md` every design decision,
  the interfaces, the shapes that existing tests pin and the alternatives rejected. The
  probe's branch and logs are not in the repository; what they showed is written here.
- Q: Why are the brightest stars lost? → A: By reading, and shown on the probe's drawn
  frames. The node keeps the first 200 of `star_detection::detect_stars`
  (`crates/scope-node/src/detect.rs:43`, `crates/scope-node/src/detect.rs:67`), which
  ports cedar-detect's gates. A candidate must stand strictly above its pixels two to
  either side (`crates/star-detection/src/detect.rs:180`), which a flat top wider than
  three binned pixels fails; a core wider than `max_size`, 7 binned pixels on a 1280
  wide frame at binning 2 (`crates/star-detection/src/binning.rs:108-114`), is refused
  (`crates/star-detection/src/detect.rs:293`), and so is a perimeter whose range passes
  3σ·noise (`crates/star-detection/src/detect.rs:361`). cedar-detect says so itself: "if
  a bright star is overexposed to a degree that it bleeds into too many adjacent pixels,
  CedarDetect will reject the bright star" and leaves exposure to its caller
  (`reference-solutions/cedar-detect/src/algorithm.rs:81-84`). The console sends the
  first 30 centroids to the solver (`crates/finder/src/solve.rs:68`,
  `crates/finder/src/solve.rs:427`), and a pattern database is built from catalog stars
  taken brightest first (`crates/database-generation/src/patterns.rs:34`,
  `crates/database-generation/src/thinning.rs:28`). On the probe's drawn frames, cedar's
  detector found no saturated star with a flat top of 32 to 401 pixels, at any of the
  widths and phases US1-S1 draws (run).
- Q: Does the finder camera's frame really have a flat top at 255? → A: By reading: the
  INDI finder camera adapter keeps a 16-bit sample's high byte (`crates/scope-node/src/indi_finder.rs:100-107`),
  so a sample at the camera's full scale reads 255. That the Mini's saturated samples
  read full scale is not measured; a camera whose clipped top reads below 255 is not
  helped (§ Edge Cases).
- Q: Does the simulator draw such a frame? → A: Yes, unchanged. The simulator's finder
  camera holds every pixel to 0 to 255 (`crates/simulator/src/render.rs:122`), and 051's
  focus error widens its stars (`crates/simulator/src/camera.rs:121`). Pointed at the
  Orion Nebula on the finder camera's 1280 × 960 grid at 10.5° (the bundled database's
  narrowest field is 10°), at focus error 3, 500 ms and gain 2, its first frame gives 69
  centroids, none within 3 px of the 10 brightest drawn stars, and the console's solve
  gives `no_match`; the Pleiades' field gives 56 and `no_match` (run). The field night's
  Orion frame gave 69 centroids, its 8 brightest stars missing, and `no_match`. The
  simulated frames are at about 30″ a pixel, not the Mini's 4.5″, and their stars are
  Gaussian; the match is in the symptom, not in the optics.
- Q: What does the fix do? → A: After cedar's own pass, the node finds each 8-connected
  region of pixels at 255 and keeps one cedar did not already find as a star when it has
  at least 9 pixels, is no longer than 1.5 times its width, its ring 3 pixels out from
  its box lies inside the frame, and that ring averages less than half of the way from
  the frame's median to 255; each is centred on the mean of its pixels' centres and put
  first, the most saturated pixels first, before cedar's list in its own order (owner
  decisions 1 to 4). With it, the two simulated fields solve within 30″ (run).
- Q: Which earlier requirements does this change? → A: 048's FR-002 reads:
  "`scope_node::detect::detect` SHALL call `detect_stars` while it is absent and
  `detect_stars_with_noise_floor` with it while present, keeping the brightest 200 as
  today" (`specs/048-node-detects-stars-on-a-dark-sky/spec.md:425-427`); it now also puts
  the saturated stars cedar set aside first (FR-003). 013's FR-006 reads: "for the same
  frame and the same detection settings the node's centroids equal the console's"
  (`specs/013-live-finder/spec.md:735-737`); for a frame holding a saturated star cedar
  sets aside, they no longer do. 025 made a raw recording a detection fixture: a frame
  line's settings, run through `detect_stars` and cut to 200, give its centroids
  (`specs/025-frame-capture/spec.md:572-575`); for such a frame they no longer do, and
  `solve_image --saturated` replays it as the node detected it (owner decision 5, owner
  approval 2). 013's FR-014, "Each star frame SHALL report the settings it was made
  with" (`specs/013-live-finder/spec.md:777-779`), still holds: the pass is no setting.
  048 left saturated stars out ("Bright saturated stars were not detected either; that
  is not this spec's", `specs/048-node-detects-stars-on-a-dark-sky/spec.md:71-72`), and
  so did 020 (`specs/020-live-chart-calm/spec.md:292-294`).
- Q: Do the cedar parity tests change? → A: No. `detect_stars` and
  `detect_stars_with_noise_floor` keep their code and every result (FR-002), so
  `cedar_detect_parity_on_all_reference_images` (`crates/star-detection/tests/parity.rs:107`)
  and the e2e evaluation against cedar see what they saw. One existing test changes: the
  node's centroids on the saved finder camera frame no longer equal `detect_stars`'s, because
  that real frame holds a saturated star cedar sets aside (owner approval 3).
- Q: What rests on a run, and what on reading? → A: Every scenario's test was written
  on the probe as the stories would write it and passed; US1-S4 to US1-S6 failed with
  the pass turned off (run). With both stories built, every other test of the workspace
  passed unchanged. The field night's frames were not replayed: only a 640 × 480 preview
  JPEG of them survives, which is not a fixture. plan.md § What was run and what was
  read lists each claim.

## Owner decisions

All open. One general rule is behind decisions 1 to 4: **a saturated star is the top
of a star, so the node keeps the regions of saturated pixels cedar's gates set aside
when they look like the top of a star, and touches nothing cedar keeps.** So cedar's
detector, its parity and its order stay as they are, no setting or link field is added,
and a frame with no such region gives exactly today's centroids. plan.md § Decisions and
the alternatives rejected gives the measures behind each.

1. **Where the pass runs** (FR-002, FR-003). Recommended **(a) in the scope node's
   detection only**, after cedar's pass, through a new module of `star-detection`;
   `detect_stars` keeps its code and results, and the console's Solve page, the gRPC
   service and `solve_image` detect as cedar does. Evidence: of cedar's 13 reference
   images, one gains a kept star at binning 1, the parity test's setting
   (`test_5mp_g100_e100ms.jpg`, run), so (b) would move a parity result; 048's owner
   kept cedar's detector untouched for the noise floor. (b) Inside `detect_stars`, for
   every caller: one rule everywhere, at the cost of Principle I's parity on that image
   and the e2e evaluation's inputs. (c) As (a), with a `[detection]` key to turn it off:
   a config key, a `--check-config` line (048's US1-S6 pins today's print) and
   Configure's handling (048's FR-007), for a switch with nothing to switch off, since
   the pass never removes or moves a star cedar keeps.
2. **The centroid of a saturated star** (FR-001). Recommended **(a) the mean of its
   saturated pixels' centres**, tetra3's centre of mass with every weight 255. Evidence
   (run): on drawn Gaussian stars of σ 1.5 to 5 px at phases 0, 0.25 and 0.5 px and on
   Moffat stars, within 0.08 px of the place (0.16 px for a barely saturated σ 2 star of
   P 600); on the simulator's bright fields, 520 kept
   stars, median 0.16 px, 90th percentile 0.29 px, largest 1.39 px from the nearest
   drawn star. (b) Flux-weighted over its box inside the ring, less the ring's mean:
   median 0.14 px but 90th percentile 1.15 px and largest 3.81 px on the same stars, as
   a neighbour's light pulls it. (c) A profile fitted to its unsaturated wings: no
   reference does it, and more code.
3. **The gates that still set a region aside** (FR-001). Recommended **(a)**: fewer than
   9 pixels (a hot pixel, or a core cedar keeps); a ring 3 px out from its box that
   crosses the frame's edge; longer than 1.5 times its width by its second moments
   (tetra3's `max_axis_ratio` in its own example, `reference-solutions/cedar-solve/tetra3/tetra3.py:1462`;
   trails, two stars run together); a ring averaging more than 0.5 of the way from the
   frame's median to 255 (cedar's perimeter, three pixels out, standing for the sky:
   nebulae); and a region holding a star cedar found. Evidence (run): it keeps every
   drawn star of US1-S1 and soft ones of σ 4 to 8 px, and sets aside every trail,
   double 8 to 12 px apart, nebula and edge star of US1-S2. Measured against it: a ring
   at twice the region's radius kept a smooth nebula and a nebula of three lobes as
   stars; 3 px with a share of 0.25 set aside soft stars of σ 4 to 8 px and a Moffat star
   of β 1.8; 6 px with 0.5 kept a star-like core in a faint halo. (b) tetra3's gates
   alone, an area window and the axis ratio: an area cap (tetra3's default 100 pixels)
   cannot tell a nebula's core from a bright star, whose tops reached 460 pixels on the
   simulator's fields. (c) Relax cedar's own gates: changes `detect_stars` (as 1 (b)).
4. **Where a saturated star ranks** (FR-001, FR-003). Recommended **(a) first, most
   saturated pixels first, before cedar's list in its own order**: under one optic, a
   wider flat top is a brighter star, and every top cedar sets aside is wider than any
   it keeps. Evidence (run), on 115 simulated frames with a kept saturated star: (a)
   solved 113, (b) merged into cedar's order by its clipped brightness 113, (c) after
   cedar's list 79. Only (a) leaves cedar's list unchanged behind the new stars; on 120
   frames of the simulator's default fields the pass changed 9 and all 120 solved with
   and without it.
5. **Whether the live report names what was set aside** (FR-004, FR-007). Recommended
   **(a) not in the live report; `solve_image --saturated` prints, for a recorded raw
   frame, how many regions were kept and set aside and why**, and the README says a star
   frame carries neither. The live report would need a field on the star frame, a scope
   link protocol change (Principle IV), written by the node's capture and service and
   read in `crates/finder/src/live.rs`, which spec 053 is changing now: a third story.
   (b) That, as a later spec once 053 has landed. (c) Nothing at all: then a raw frame
   recorded on a field night replays through `solve_image` as cedar detects it, not as
   the node did.
6. **An exposure hint that keeps the brightest stars just under saturation**. Recommended
   **(a) later, and only if a field night shows saturated frames still fail** once this
   lands: with the saturated stars kept, saturation no longer costs the solve on the
   simulated fields. (b) Here: needs 5 (b)'s link field, and a rule for "too many"
   saturated stars that no field night has measured. (c) Auto-exposure aiming below
   saturation: changes 013's FR-015, out of scope.

## Owner approvals

Flipping this spec to `ready` approves the answer to every owner decision above, and
the following. Each is named in the FR that makes it. No golden, fixture, `scripts/ci/`
or CI file changes.

1. **New public API** (FR-001): the module `star_detection::saturated` with
   `SATURATED`, `MIN_PIXELS`, `MAX_AXIS_RATIO`, `RING_MARGIN`, `MAX_RING_SHARE`,
   `SetAside`, `saturated_stars` and `with_saturated_stars`. No existing signature
   changes.
2. **The node's centroids** (FR-003): for a frame holding a saturated star cedar sets
   aside, the node's centroids no longer equal `detect_stars`'s (013's FR-006) and a raw
   recording's frame line no longer replays through `detect_stars` alone (025); with
   none, nothing changes.
3. **One existing test replaced** (FR-003; Principle II):
   `saved_finder_frame_has_identical_nonempty_centroids_in_console_order`
   (`crates/scope-node/tests/detect.rs:26`) holds the node's centroids on
   `fixtures/scope-node/finder-frame.png` equal to `detect_stars`'s; that frame holds a
   saturated star of 19 pixels that cedar sets aside, so it fails with the pass (run: 38
   centroids against 37). US1 replaces it with
   `saved_finder_frame_keeps_its_saturated_star_first_then_the_consoles_centroids_in_order`
   (US1-S4), which keeps every one of its checks and holds the rest of the list to
   `detect_stars`'s exactly.
4. **The glossary** (FR-006): **Saturated star**.
5. **The README sections and `solve_image --saturated`** (FR-005, FR-007).

## User Scenarios & Testing *(mandatory)*

Every scenario shares these definitions.

- **The gate command.** Every scenario runs in
  `cargo nextest run --workspace --all-targets --all-features --locked`, the `test`
  gate's first command, each test in its own process.
- **A drawn frame** is a 640 × 480 frame of the test's own making: at each pixel's
  centre (column + 0.5, row + 0.5), background 10 plus each light the scenario names,
  rounded and held to 0 to 255. **A Gaussian of σ and peak P at (x, y)** adds
  P·exp(−r²/2σ²), r the distance from (x, y); **a Moffat of α, β and P** adds
  P·(1 + r²/α²)^−β. **cedar's list** is `detect_stars` of the frame at sigma 8, binning
  2, rows normalised and hot pixels rejected, the node's settings.
- **A bright frame** is the first frame of a `SimCamera` at 500 ms and gain 2 over a
  simulator on `crate::support::live::DB` with `Site::default()` and a `VirtualClock`
  from `crate::support::live::START`, its optics 1280 × 960 at 10.5° with the eyepiece
  at (640, 480), `point_eyepiece_at` the field, and `set_finder_focus_error(3.0)`. **The
  Orion Nebula's field** is RA 83.82°, Dec −5.39°; **the Pleiades'** RA 56.75°, Dec
  24.12°. **Its drawn stars** are `field_stars(500.0, 2.0)` read before the frame, in
  their order; **its truth** is `telescope().camera`.
- **The console's solve** is `finder::solve::solve_centroids` over
  `crate::support::live::DB` with `SolveOptions::default()` and the frame's size, as the
  live loop calls it (`crates/finder/src/live.rs:40`); it sends the first 30 centroids.
- **Oracles.** The drawn places and shapes; the simulator's drawn stars and true
  direction; the saved finder camera frame's own pixels; cedar's list, which this spec does not
  change; the constants as the README states them. No expected place comes from the code
  under test.

### User Story 1 - The scope node keeps saturated stars, first (Priority: P1)

As the observer, when the finder camera's brightest stars saturate, I need the scope
node to keep them as stars and send them first, so that the live solve finds the field
the patterns were built for, and hot pixels, nebulae, trails and merged doubles stay out.

**Why this priority**: Without it a field whose brightest stars saturate gives
`no_match`, and with it the aim point, GoTo and Track.

**Independent Test**: `crates/star-detection/tests/saturated.rs` (US1-S1 to US1-S3) and
`crates/finder/tests/console/saturated_field.rs` (US1-S5, US1-S6), new, and the replaced
test of `crates/scope-node/tests/detect.rs` (US1-S4). The `test` gate runs them; during
development, `cargo nextest run -p star-detection --locked --test saturated`,
`cargo nextest run -p scope-node --locked --test detect` and
`cargo nextest run -p finder --locked --test console -E 'test(saturated_field)'`.

**Acceptance Scenarios**:

1. **Given** drawn frames each of one Gaussian at (p, p) for p = 200.5, 200.75 and 201.0, of σ 1.5, 3, 3 and 5 and P 3000, 3000, 30000 and 3000, two drawn frames of one Moffat at (200.5, 200.5) of α 2, P 25500 and β 2.5 and 1.8, and in each also a Gaussian at (400.5, 300.5) of σ 1.5 and P 100, **When** a committed test calls `saturated_stars` on each with cedar's list and `with_saturated_stars` with it, **Then** it asserts that each keeps exactly one star, within 0.1 px of (p, p) in x and y, with `peak_value` 255 and `num_saturated` the frame's count of pixels at 255, and sets nothing aside; that `with_saturated_stars` gives that star and then cedar's list unchanged; and that cedar's list holds a star within 0.5 px of (400.5, 300.5) and none within 3 px of (p, p).
2. **Given** drawn frames of a segment from (150, 100) to (450, 340) at 400 within 1, 2 and 3 px of it; of two Gaussians of σ 3 at (300.5 ∓ d/2, 240.5), P 3000 and 3000 for d = 8, 10 and 12 and P 3000 and 800 for d = 8 and 12; of a Gaussian of σ 40 and P h at (320.5, 240.5) plus one of σ s and P c there, for (h, s, c) = (200, 8, 400), (60, 8, 400), (0, 25, 400) and (150, 3, 3000); of three Gaussians at (300.5, 230.5), (330.5, 250.5) and (315.5, 270.5) of σ 30, 25 and 20 and P 180, 180 and 150; of a Gaussian of σ 3 and P 3000 at (5.5, 240.5); and three frames of background 10 with 255 in a block of 1 × 1, 2 × 2 and 4 wide by 2 high from (320, 240), **When** a committed test calls `saturated_stars` on the first five kinds with cedar's list and on the blocks with no stars, **Then** it asserts that none keeps a star; that the segments and the pairs are each set aside as `[Elongated]`, the five nebulae as `[Extended]`, the star at 5.5 as `[Edge]`, the 1 × 1 block as `[]` and the other two blocks as `[Small]`; and that `with_saturated_stars` gives each of the first five kinds' cedar's list unchanged.
3. **Given** drawn frames of two Gaussians of σ 3 and P 3000 at (300.5 ∓ d/2, 240.5) for d = 4, 6, 16 and 24, **When** a committed test calls `saturated_stars` on each with cedar's list, **Then** it asserts that none sets anything aside, every kept star is within 0.1 px of y 240.5, at d = 4 and 6 one star is kept within 0.1 px of x 300.5, and at d = 16 and 24 two are kept within 0.1 px of 300.5 ∓ d/2.
4. **Given** `fixtures/scope-node/finder-frame.png` as the existing test reads it, **When** a committed test lists its pixels at 255 and detects it with `scope_node::detect::detect` and `DetectSettings::cedar()` and with `detect_stars` at `DetectParams::cedar()`'s settings, **Then** it asserts that the frame and its size are as the fixture's `fixtures.json` says, that its pixels at 255 are exactly (20 to 24, 187), (20, 21, 23 and 24, 188), (20 to 22, 189), (20 to 24, 190) and (22 and 23, 191) as (x, y), that `detect_stars` gives a non-empty list with no star within 5 px of (22.5, 189.5), that the node gives one centroid more, the first within 10⁻⁹ px of (416/19 + 0.5, 3586/19 + 0.5), and that every later one equals `detect_stars`'s in order, x, y and brightness exactly.
5. **Given** a bright frame of the Orion Nebula's field and one of the Pleiades', **When** a committed test detects each with `scope_node::detect::detect` and `DetectSettings::cedar()` and with cedar's list cut to 200 as `Centroid`s, and runs the console's solve on both, **Then** it asserts that each of the 5 first drawn stars at least 20 px inside the frame has a node centroid within 1.5 px among the node's first 10 and no centroid of cedar's list within 3 px; that the console's solve of cedar's list answers `NoMatch`; and that of the node's centroids `MatchFound`, its pointing within 30″ of the frame's truth.
6. **Given** `AppState::shared` over `crate::support::live::DB` with simulation at `crate::support::live::START` over `SimulationTransport::Memory` and the live loop, its simulator's frames stopped, then set as a bright frame's simulator is for the Orion Nebula's field, `PUT /api/session` with `camera_settings` `{"exposure_ms": 500, "gain": 2}` answered 200, and the frames started again, **When** a committed test under `#[tokio::test(start_paused = true)]` reads live reports until three are 1280 wide, **Then** it asserts that each of those three is `MatchFound` with its pointing within 60″ of the simulator's camera direction read then, and that they come within 30 s of `START` on the simulator's clock.

### User Story 2 - The README says which saturated regions are stars (Priority: P2)

As the observer, I need the node's README to say that saturated stars are kept, which
regions are set aside and why, that a star frame does not say so, and how to replay a
recorded raw frame as the node detected it.

**Why this priority**: The behaviour works without it; the next field night's checks
and the replay of its raw frames need it.

**Independent Test**: `crates/scope-node/tests/saturated_readme.rs`, new. The `test`
gate runs it; during development,
`cargo nextest run -p scope-node --locked --test saturated_readme`.

**Acceptance Scenarios**:

1. **Given** the committed `crates/scope-node/README.md`, `GLOSSARY.md` and `crates/plate-solver/examples/solve_image.rs`, **When** a committed test reads the README's section from its line `## Bright stars: saturated stars` to the next `## ` heading, its whitespace runs read as one space, **Then** it asserts that the section holds `its brightest pixels read 255`, `at least 9 pixels: a hot pixel is one`, `its length at most 1.5 times its width`, `the ring 3 pixels out from it averaging less than 0.5 of the way from the frame's median to 255`, `whole, that ring inside the frame`, `put first, the most saturated pixels first`, `the node keeps the first 200 as before`, `A star frame does not say which centroids are saturated, nor how many regions were set aside.` and `--saturated <frame>`, each number formatted from `SATURATED`, `MIN_PIXELS`, `MAX_AXIS_RATIO`, `RING_MARGIN` and `MAX_RING_SHARE`; that the glossary holds a line `**Saturated star**:` followed by a line beginning `A star whose brightest pixels read 255`; and that the example holds `[--noise-floor F] [--saturated] [--stars N]` and a doc line beginning ``//! * `--saturated` —``.

### Edge Cases

- **Two stars run into one round top** closer than about the top's own radius are kept
  as one star at their middle (US1-S3 at 4 and 6 px); 8 to 12 px apart they are set
  aside as elongated (US1-S2); farther apart, two stars (US1-S3).
- **A star embedded in nebulosity** whose ring stays bright is set aside with the
  nebula (US1-S2's (150, 3, 3000)); a smooth round knot with dark surroundings is kept,
  since by shape it is a star.
- **A saturated star near the frame's edge** whose ring would cross it is set aside
  (US1-S2), as cedar refuses a blob near the edge.
- **A camera whose clipped top reads below 255** gets no help: the pass looks only for
  255. The INDI finder camera adapter's high byte reads 255 at full scale, by reading.
- **Hot pixels**: a lone one is no region; a block of 2 to 8 is small (US1-S2). A block
  of 9 or more that cedar keeps is cedar's, as today; one it does not keep would be
  kept as a star if round and dark around (not drawn; § Assumptions).
- **More than 200 stars**: the saturated stars come first, so the node's cut to 200
  drops the faintest of cedar's, never a saturated star.
- **A frame with no pixel at 255** gives exactly cedar's list, as today; the pass costs
  one scan of the frame for a 255 (plan.md § What was run gives the cost as a ratio).
- **The finder camera's focus aid** (051) measures no star with a pixel at 250 or above
  (`crates/finder/src/finder_focus.rs:40`); a saturated star now in the list is skipped
  by it as before, and its neighbours within the aid's box are no longer measured. 051's
  tests pass unchanged (run).

## Requirements *(mandatory)*

### Functional Requirements

**Detection** (US1):

- **FR-001**: `star-detection` SHALL gain `pub mod saturated` with `SATURATED` 255,
  `MIN_PIXELS` 9, `MAX_AXIS_RATIO` 1.5, `RING_MARGIN` 3, `MAX_RING_SHARE` 0.5, the enum
  `SetAside` (`Small`, `Edge`, `Elongated`, `Extended`), and
  `saturated_stars(image, width, height, stars) -> (Vec<Star>, Vec<SetAside>)`, which
  takes each 8-connected region of pixels at `SATURATED` in reading order of its first
  pixel; passes over a region of one pixel or whose box holds a star of `stars`; sets a
  region aside as `Small` below `MIN_PIXELS` pixels, `Edge` when its box widened by
  `RING_MARGIN` on each side leaves the frame, `Elongated` when the root of its second
  moments' major over minor eigenvalue exceeds `MAX_AXIS_RATIO` or the minor is not
  positive, and `Extended` when that widened box's one-pixel perimeter (its ring)
  averages more than `MAX_RING_SHARE` of the way from the median of every 64th pixel of
  the frame to `SATURATED`, checked in that order; and keeps every other region as a
  `Star` at the mean of its pixels' centres, `peak_value` `SATURATED`, `num_saturated`
  its pixel count, and brightness the sum over the box inside the ring less the ring's
  mean for each pixel; the kept stars ordered by pixel count, most first, then as `Star`
  orders them. `with_saturated_stars(image, width, height, stars)` SHALL give the kept
  stars followed by `stars` unchanged, and `stars` itself when none is kept (owner
  decisions 2 to 4, owner approval 1).
- **FR-002**: `detect_stars` and `detect_stars_with_noise_floor` SHALL keep their
  signatures, code and results, so that cedar's list still sets a wide saturated star
  aside (US1-S1) and cedar's parity tests pass unchanged (owner decision 1).
- **FR-003**: `scope_node::detect::detect` SHALL pass the stars it detects, with or
  without a noise floor, through `with_saturated_stars` before keeping the first 200, so
  that saturated stars cedar sets aside come first (048's FR-002 changed; owner decision
  1, owner approvals 2 and 3).
- **FR-004**: Nothing else SHALL change: the scope link's protocol and the star frame,
  `DetectSettings` and the node's config, auto-exposure, `crates/finder/src`, the
  simulator's `src/`, `Cargo.toml` and `Cargo.lock`, and every existing test but US1-S4's
  (owner decision 5).

**Words and replay** (US2):

- **FR-005**: The node's README SHALL gain the section `## Bright stars: saturated
  stars`, before `## The mount link`, in plan.md § Interfaces' words; and the finder
  README's live stream section a paragraph, before `` `GET /api/live` is
  `text/event-stream`. ``, saying the live solve takes the node's centroids with the
  saturated stars first and the Solve page detects as cedar-detect does.
- **FR-006**: `GLOSSARY.md` SHALL gain **Saturated star**, after **Centroid**, in plan.md
  § Interfaces' words (owner approval 4).
- **FR-007**: The `solve_image` example SHALL take `--saturated` and, when given, pass
  its detected stars through `saturated_stars` and put the kept ones first, print
  `<image>: saturated stars kept <k>, set aside <a> (small <s>, edge <e>, elongated <l>,
  extended <x>)` before its result line, or add `"saturated_kept"` and
  `"saturated_set_aside"` to its JSON; its usage line and doc comment SHALL list the flag
  (owner decision 5, owner approval 5).

### Key Entities

- **Saturated star**: a star whose brightest pixels read 255, so its top is flat; the
  node keeps one cedar's gates set aside, first, when it is round, dark around and whole
  in the frame.
- **Region**: an 8-connected set of pixels at 255.
- **Ring**: the one-pixel perimeter of a region's box widened by 3 pixels on each side,
  standing for the sky around it, as cedar's perimeter does for a core.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario has a committed test that asserts it, and the
  `lint` and `test` gates pass.
- **SC-002**: Every existing test passes unchanged but the one owner approval 3
  replaces; no golden, fixture, `scripts/ci/` file, `.github/` file, `CODEOWNERS`
  entry, `ergane.yaml` or `playwright.config.ts` is edited.
- **SC-003**: On the simulator's Orion Nebula and Pleiades fields at the finder camera's
  grid with their brightest stars saturated, the live solve matches within 60″ and the
  console's solve of a frame within 30″, where cedar's detection alone gives `no_match`
  (US1-S5, US1-S6). Simulated, not a real finder camera's frames (plan.md § Outside the
  factory).

## Assumptions

- **Earlier features are in place.** Every spec in `depends_on_landed` has landed. Every
  citation here was read at f6d4f5fd.
- **The scope** is the scope node's detection, with reaches into `star-detection` (the
  new module), the console's test binary (US1-S5, US1-S6), the plate solver's example
  (US2) and the glossary. The live report, the scope link, sync intake (spec 053), the
  solver's matching, the databases, the node's 8-bit read and auto-exposure are out.
- **The simulated fields stand for the field night's** in their symptom only: the
  brightest stars flat at 255 and missing, tens of stars detected, `no_match`. Their
  stars are Gaussian at about 30″ a pixel; the Mini's are about 4.5″ a pixel, through
  an achromat. Whether a real frame solves is the next field night's (plan.md § Outside
  the factory).
- **Hot pixels on the finder camera are single**: not measured on the Mini. A block of 9
  or more saturated hot pixels that cedar does not keep would be kept.

## Work Graph

```yaml
US1:
  depends_on: []
  implements: [FR-001, FR-002, FR-003, FR-004]
US2:
  depends_on: []
  depends_on_merged: [US1]
  implements: [FR-005, FR-006, FR-007]
```

Two stories, each under 60 % of 65,536 B (39,321 B) as the probe's commits measure
them: US1 25,754 B (39 %), US2 12,088 B (18 %); at the factory's worst overrun seen,
1.35 times, 34,768 B and 16,319 B, still under the line. US2 merges after US1
(`depends_on_merged`): its README test reads US1's constants and `solve_image` calls
US1's `saturated_stars`.
