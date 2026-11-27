---
description: "Task list for 052-node-keeps-saturated-stars: The Node Keeps Saturated Stars"
---

# Tasks: The Node Keeps Saturated Stars

**Prerequisites** (plan.md § Prerequisites):
- every spec in `depends_on_landed` has landed on `main`: met at f6d4f5fd;
- `main` is still as read: met, f6d4f5fd at the draft, at probe round 1's fold and at the
  flip;
- the owner decisions in spec.md are settled: met, all seven, each answered with its
  recommendation on 2026-11-27;
- a review round of this trio, folded: met, round 1 folded;
- a probe round from this trio alone, folded: met, round 1 (491b7655) folded with
  ae80ff09;
- no operator-staged file is needed: met (US2's fixture is a story's own new file).

**Rules for every story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml`,
  `CODEOWNERS`, `ergane.yaml` or `.specify/`. If an existing test fails, stop and
  escalate with its exact output. A test file an earlier story of this spec created is
  an existing test. The exceptions: US2's replacement of
  `saved_finder_frame_has_identical_nonempty_centroids_in_console_order` in
  `crates/scope-node/tests/detect.rs` (spec owner approval 3); US2's `mod` line in
  `crates/finder/tests/console/main.rs`, in its alphabetical place; and US2's new
  fixture `fixtures/finder/tycho-pleiades.csv` (spec owner approval 7).
- Write only the files your story's tasks name; they are its `writes` in plan.md
  § Refinement Record. A story that needs another file escalates.
- `crates/star-detection/src/detect.rs`, `binning.rs`, `centroid.rs`, `noise.rs` and
  `star.rs`, `crates/scope-link`, `proto/`, the simulator's `src/`, the node's
  `config.rs`, `capture.rs`, `service.rs` and `exposure.rs`, `Cargo.toml` and
  `Cargo.lock` are never touched, nor any file of `crates/finder/src` but US1's
  `finder_focus.rs` and US2's `solve.rs`; nothing is added under
  `crates/finder/tests/support/`.
- Copy the words of spec.md's FRs and of plan.md § Interfaces exactly, the README's and
  the glossary's among them. Say "finder camera", never "finder" alone, in every message.
- No test downloads anything, and no test writes outside `CARGO_TARGET_TMPDIR`. Run no
  tool the gates don't run, and do not set `CARGO_TARGET_DIR`.
- The results said here before each story's implementation are the probe's
  (plan.md § What was run and what was read), where it ran them.

## Phase 1: User Story 1 - The pass keeps saturated stars (Priority: P1)

### Tests first

- [ ] T001 [US1] Create `crates/star-detection/tests/saturated.rs` with one `#[test]`
  each for US1-S1, US1-S2, US1-S3, US1-S4 and US1-S5, drawing the frames as spec.md
  § User Scenarios defines them (each pixel's level at its centre, a star trailed L px
  the mean over 2L centres evenly spaced along its trail) and taking cedar's list from
  `detect_stars` at sigma 8, binning 2, rows normalised and hot pixels rejected. Before
  T002 the test does not build (no `star_detection::saturated`; run).

### Implementation

- [ ] T002 [US1] Create `crates/star-detection/src/saturated.rs` with its module doc,
  its eight constants, `SetAside`, `saturated_stars` and `with_saturated_stars`, as
  FR-001 and plan.md § Interfaces give them, and add `pub mod saturated;` to
  `crates/star-detection/src/lib.rs` after `pub mod noise;` (FR-001, FR-002).
- [ ] T003 [US1] In `crates/finder/src/finder_focus.rs`, make `SATURATED`
  `star_detection::saturated::SATURATED` with the doc lines plan.md § Interfaces gives;
  051's focus tests pass unchanged (FR-003).
- [ ] T004 [US1] Run the `lint` and `test` gates with every existing test unchanged and
  green, then commit the green story (FR-001 to FR-003; US1-S1 to US1-S5).

## Phase 2: User Story 2 - The scope node and the console keep saturated stars (Priority: P1)

### Tests first

- [ ] T005 [US2] In `crates/scope-node/tests/detect.rs`, replace
  `saved_finder_frame_has_identical_nonempty_centroids_in_console_order` with
  `saved_finder_frame_keeps_its_saturated_star_first_then_the_consoles_centroids_in_order`
  (US2-S1), keeping its reading of the fixture, its size checks and its exact equality,
  now from the second centroid on, and writing the level 250 as a number; leave the
  file's other two tests as they are. Before T008 it fails: one centroid short (run).
- [ ] T006 [US2] Create `fixtures/finder/tycho-pleiades.csv` exactly as plan.md
  § Interfaces gives it: its comment line, its header and its 203 rows, 5,329 bytes
  (spec.md owner approval 7). The kit's database it was read from is not in the
  repository; the story copies the text and reads no other source. Its places are
  degrees; a `CatalogEntry` takes radians.
- [ ] T007 [US2] Create `crates/finder/tests/console/saturated_field.rs` with US2-S2 to
  US2-S5, US2-S2, US2-S3 and US2-S5 under `#[tokio::test(start_paused = true)]` and
  US2-S4 under `#[test]`, using `crate::support::live::{event, request, DB, START}`, and
  add `mod saturated_field;` to `crates/finder/tests/console/main.rs` after
  `mod recording_raw;`. US2-S4 builds its database once into `CARGO_TARGET_TMPDIR` from
  the fixture, each row's degrees converted to the radians a `CatalogEntry` takes, a
  process-unique `.part` file renamed into place, and only when the file is missing:
  after changing how it is built, delete `052-tycho-pleiades.npz` from
  `CARGO_TARGET_TMPDIR` first (plan trap 6). Before T008 and T009 all four fail: US2-S2
  with the node's first 10 short of a bright star, US2-S3 to US2-S5 with `NoMatch` where
  a match is asserted; with T008 alone, US2-S5 fails with the uploaded frame's places
  unequal to the node's (run).

### Implementation

- [ ] T008 [US2] In `crates/scope-node/src/detect.rs`, pass the detected stars through
  `star_detection::saturated::with_saturated_stars` before `.take(200)`, and change the
  doc line to the words plan.md § Interfaces gives (FR-004).
- [ ] T009 [US2] In `crates/finder/src/solve.rs`, pass the stars `detect_stars` gives
  in `solve_frame_with_frame` through `with_saturated_stars`, with the comment plan.md
  § Interfaces gives; change nothing else in the file (FR-005).
- [ ] T010 [US2] Run the `lint` and `test` gates with every existing test but T005's
  unchanged and green, then commit the green story (FR-004 to FR-006; US2-S1 to US2-S5).

## Phase 3: User Story 3 - The README says which saturated regions are stars (Priority: P2)

### Tests first

- [ ] T011 [US3] Create `crates/scope-node/tests/saturated_readme.rs` with US3-S1 under
  `#[test]`, its numbers formatted from `star_detection::saturated`'s constants. Before
  T012 to T014 it fails: the README has no such section (run).

### Implementation

- [ ] T012 [US3] In `crates/scope-node/README.md`, insert the section
  `## Bright stars: saturated stars` before `## The mount link`, and in
  `crates/finder/README.md` the paragraph before `` `GET /api/live` is
  `text/event-stream`. ``, each exactly as plan.md § Interfaces gives it (FR-007).
- [ ] T013 [US3] In `GLOSSARY.md`, add **Saturated star** after **Centroid**, exactly as
  plan.md § Interfaces gives it (FR-008).
- [ ] T014 [US3] In `crates/plate-solver/examples/solve_image.rs`, add `--saturated` as
  FR-009 and plan.md § Interfaces give it: the field, the parse, the refusal with
  `--dark`, the usage line, the synopsis, the doc lines, the pass after detection and
  the two outputs (FR-009).
- [ ] T015 [US3] Run the `lint` and `test` gates with every existing test unchanged and
  green, then commit the green story (FR-007 to FR-009; US3-S1).

## Execution Order

US1, then US2, then US3 (`depends_on_merged`): US2 calls US1's `with_saturated_stars`;
US3's test reads US1's constants, `solve_image` calls US1's functions, and its words
describe the node and the Solve page as US2 leaves them. Within each story, the tests
first, then the implementation, then the gates.
