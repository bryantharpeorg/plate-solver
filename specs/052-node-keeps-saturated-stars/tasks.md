---
description: "Task list for 052-node-keeps-saturated-stars: The Node Keeps Saturated Stars"
---

# Tasks: The Node Keeps Saturated Stars

**Prerequisites** (plan.md § Prerequisites):
- every spec in `depends_on_landed` has landed on `main`: met at f6d4f5fd;
- `main` is still as read: met at the draft, f6d4f5fd;
- the owner decisions in spec.md are settled: not yet, all six open;
- a review round of this trio, folded: not yet;
- no operator-staged file is needed: met.

**Rules for every story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml`,
  `CODEOWNERS`, `ergane.yaml` or `.specify/`. If an existing test fails, stop and
  escalate with its exact output. A test file an earlier story of this spec created is
  an existing test. The exceptions: US1's replacement of
  `saved_finder_frame_has_identical_nonempty_centroids_in_console_order` in
  `crates/scope-node/tests/detect.rs` (spec owner approval 3), and US1's `mod` line in
  `crates/finder/tests/console/main.rs`, in its alphabetical place.
- Write only the files your story's tasks name; they are its `writes` in plan.md
  § Refinement Record. A story that needs another file escalates.
- `crates/star-detection/src/detect.rs`, `binning.rs`, `centroid.rs`, `noise.rs` and
  `star.rs`, `crates/finder/src`, `crates/scope-link`, `proto/`, the simulator's `src/`,
  the node's `config.rs`, `capture.rs`, `service.rs` and `exposure.rs`, `Cargo.toml` and
  `Cargo.lock` are never touched, and nothing is added under `crates/finder/tests/support/`.
- Copy the words of spec.md's FRs and of plan.md § Interfaces exactly, the README's and
  the glossary's among them. Say "finder camera", never "finder" alone, in every message.
- No test downloads anything, and no test writes outside `CARGO_TARGET_TMPDIR`. Run no
  tool the gates don't run, and do not set `CARGO_TARGET_DIR`.
- The results said here before each story's implementation are the probe's
  (plan.md § What was run and what was read), where it ran them.

## Phase 1: User Story 1 - The scope node keeps saturated stars, first (Priority: P1)

### Tests first

- [ ] T001 [US1] Create `crates/star-detection/tests/saturated.rs` with one `#[test]`
  each for US1-S1, US1-S2 and US1-S3, drawing the frames as spec.md § User Scenarios
  defines them (each pixel's level at its centre) and taking cedar's list from
  `detect_stars` at sigma 8, binning 2, rows normalised and hot pixels rejected. Before
  T004 the test does not build (no `star_detection::saturated`; run).
- [ ] T002 [US1] In `crates/scope-node/tests/detect.rs`, replace
  `saved_finder_frame_has_identical_nonempty_centroids_in_console_order` with
  `saved_finder_frame_keeps_its_saturated_star_first_then_the_consoles_centroids_in_order`
  (US1-S4), keeping its reading of the fixture, its size checks and its exact equality,
  now from the second centroid on; leave the file's other two tests as they are. Before
  T005 it fails: 37 centroids against 38 (run).
- [ ] T003 [US1] Create `crates/finder/tests/console/saturated_field.rs` with US1-S5 and
  US1-S6, each under `#[tokio::test(start_paused = true)]`, using
  `crate::support::live::{event, request, DB, START}`, and add `mod saturated_field;` to
  `crates/finder/tests/console/main.rs` after `mod recording_raw;`. Before T005 both fail:
  a bright star missing from the node's first 10, and the live report `NoMatch` (run).

### Implementation

- [ ] T004 [US1] Create `crates/star-detection/src/saturated.rs` with its module doc,
  `SATURATED`, `MIN_PIXELS`, `MAX_AXIS_RATIO`, `RING_MARGIN`, `MAX_RING_SHARE`,
  `SetAside`, `saturated_stars` and `with_saturated_stars`, as FR-001 and plan.md
  § Interfaces give them, and add `pub mod saturated;` to
  `crates/star-detection/src/lib.rs` after `pub mod noise;` (FR-001, FR-002).
- [ ] T005 [US1] In `crates/scope-node/src/detect.rs`, pass the detected stars through
  `star_detection::saturated::with_saturated_stars` before `.take(200)`, and add the
  doc words plan.md § Interfaces gives (FR-003).
- [ ] T006 [US1] Run the `lint` and `test` gates with every existing test but T002's
  unchanged and green, then commit the green story (FR-001 to FR-004; US1-S1 to US1-S6).

## Phase 2: User Story 2 - The README says which saturated regions are stars (Priority: P2)

### Tests first

- [ ] T007 [US2] Create `crates/scope-node/tests/saturated_readme.rs` with US2-S1 under
  `#[test]`, its numbers formatted from `star_detection::saturated`'s constants. Before
  T008 to T010 it fails: the README has no such section (run).

### Implementation

- [ ] T008 [US2] In `crates/scope-node/README.md`, insert the section
  `## Bright stars: saturated stars` before `## The mount link`, and in
  `crates/finder/README.md` the paragraph before `` `GET /api/live` is
  `text/event-stream`. ``, each exactly as plan.md § Interfaces gives it (FR-005).
- [ ] T009 [US2] In `GLOSSARY.md`, add **Saturated star** after **Centroid**, exactly as
  plan.md § Interfaces gives it (FR-006).
- [ ] T010 [US2] In `crates/plate-solver/examples/solve_image.rs`, add `--saturated` as
  FR-007 and plan.md § Interfaces give it: the field, the parse, the usage line, the
  synopsis, the doc lines, the pass after detection and the two outputs (FR-007).
- [ ] T011 [US2] Run the `lint` and `test` gates with every existing test unchanged and
  green, then commit the green story (FR-005 to FR-007; US2-S1).

## Execution Order

US1, then US2 (`depends_on_merged`): US2's test reads US1's constants and `solve_image`
calls US1's `saturated_stars`. Within each story, the tests first, then the
implementation, then the gates.
