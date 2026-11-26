---
description: "Task list for 050-tools-page-aligns-finder-camera: The Tools Page Aligns the Finder Camera"
---

# Tasks: The Tools Page Aligns the Finder Camera

**Prerequisites** (plan.md § Prerequisites):
- every spec in `depends_on_landed` has landed on `main`: met at ea3d6a9b;
- the owner decisions in spec.md are settled: not met, two settled and ten open;
- every story has been probe-built from this text: not met;
- the page half has run once: met on the probe (`CI=1 npm run e2e` 33/33);
- no operator-staged file is needed.

**Rules for every story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml`,
  `CODEOWNERS`, `ergane.yaml` or `.specify/`. If an existing test fails, stop and
  escalate with its exact output. A test file an earlier story of this spec created is
  an existing test. The exceptions: each story's `mod` lines in
  `crates/finder/tests/console/main.rs`, `crates/finder/tests/motion/main.rs` or
  `crates/finder/tests/mcp/main.rs`, each in its alphabetical place, and US4's two list
  entries in `crates/finder/web/playwright.config.ts` (spec owner approval 6).
- Write only the files your story's tasks name; they are its `writes` in plan.md
  § Refinement Record. A story that needs another file escalates.
- `crates/finder/src/imaging/relation.rs`, `solving.rs`, `solver.rs`, `simulated.rs`,
  `frames.rs` and `focus/`, `crates/finder/src/session.rs`, `crates/scope-node`,
  `crates/scope-link`, `proto/`, the plate solver's crates and the databases are never
  touched, and nothing is added under `crates/finder/tests/support/`. In
  `crates/simulator`, only US1's one assert changes.
- Copy the words of spec.md's FRs and of plan.md § Interfaces exactly, the README's and
  the glossary's among them.
- No test downloads anything, and no test writes outside `CARGO_TARGET_TMPDIR` or a
  `crate::support::Scratch` folder. Run no tool the gates don't run, and do not set
  `CARGO_TARGET_DIR`.

## Phase 1: User Story 1 - The console places the imaging camera's centre off the finder camera's frame (Priority: P1)

### Tests first

- [ ] T001 [US1] Create `crates/finder/tests/console/imaging_align_centre.rs` with one
  test for each of US1-S1, US1-S2, US1-S3, US1-S4 and US1-S5, under `#[test]`, building
  the centre frame as spec.md § User Scenarios gives it, and add
  `mod imaging_align_centre;` to `crates/finder/tests/console/main.rs` in its
  alphabetical place. They fail to build until T003 adds `centre_of`; without T002,
  US1-S5 panics at `crates/simulator/src/optics.rs:52` (by reading the assert).

### Implementation

- [ ] T002 [US1] In `crates/simulator/src/optics.rs`, make `eyepiece_ray`'s assert a
  finite pixel's, as plan.md § Interfaces gives it (FR-001).
- [ ] T003 [US1] Create `crates/finder/src/imaging/align.rs` with its module doc,
  `MIDDLE_SHARE`, `Centre` and `centre_of` (FR-002; plan.md § Interfaces), and add
  `pub mod align;` to `crates/finder/src/imaging/mod.rs`.
- [ ] T004 [US1] Run the `lint` and `test` gates with every existing test unchanged and
  green, then commit the green story (FR-001 to FR-003; US1-S1, US1-S2, US1-S3, US1-S4,
  US1-S5).

## Phase 2: User Story 2 - The finder alignment runs and shows the place frame by frame (Priority: P1)

### Tests first

- [ ] T005 [US2] Create `crates/finder/tests/console/imaging_align.rs` with the
  alignment rig of spec.md § User Scenarios and its `pub struct Setup` (`live`,
  `solver`, `imaging`, `focal_mm`, `interval_ms`, the default being the rig: all on,
  1800 mm, 500 ms), the `pub` helpers `rig`, `call`, `wait_until`, `ready`,
  `last_after`, `start`, `oracle` and `assert_centre`, and one test for each of US2-S1,
  US2-S2, US2-S3, US2-S4, US2-S5, US2-S6, US2-S7 and US2-S8, under
  `#[tokio::test(start_paused = true)]`, and add `mod imaging_align;` to
  `crates/finder/tests/console/main.rs` in its alphabetical place. On US1 there is no
  route `/api/imaging/align`, so all eight fail (by reading).

### Implementation

- [ ] T006 [US2] In `crates/finder/src/imaging/align.rs`, add `HOLDER`, `PURPOSE`,
  `ALIGN_EXPOSURE_MS`, `PAIR_S`, `SEARCH_RADIUS_DEG`, `AlignAsk`, `AlignLast` without
  `aim` and `done`, `AlignView`, `State`, `view`, `Imaging::align_start` with FR-004's
  refusals, `Imaging::align_stop`, `Imaging::align_running` and the run's task, steps,
  frames and count of frames in a row that do not solve, with no call to `note_solve`
  and no Track or field-height check (FR-004 to FR-008; plan.md § Interfaces).
- [ ] T007 [US2] In `crates/finder/src/imaging/mod.rs`, add `Held`'s `align`,
  `ImagingView`'s `align` and the `Drop` line; in `crates/finder/src/imaging/routes.rs`,
  add the route, `post_align` and `delete_align` (FR-004, FR-006, FR-007).
- [ ] T008 [US2] Run the `lint` and `test` gates with every existing test unchanged and
  green, US1's test file among them, then commit the green story (FR-004 to FR-009;
  US2-S1, US2-S2, US2-S3, US2-S4, US2-S5, US2-S6, US2-S7, US2-S8).

## Phase 3: User Story 3 - The alignment says Done, refuses Track and a missing field height, and MCP and the README carry it (Priority: P2)

### Tests first

- [ ] T009 [US3] Create `crates/finder/tests/console/imaging_align_aim.rs` with one test
  for each of US3-S1, US3-S2, US3-S3 and US3-S4, under
  `#[tokio::test(start_paused = true)]`, using
  `super::imaging_align::{assert_centre, call, last_after, ready, rig, start, Setup}`,
  and add `mod imaging_align_aim;` to `crates/finder/tests/console/main.rs` in its
  alphabetical place. On US2 `last` has no `aim` and no `done` and the start has no
  field-height refusal, so all four fail (by reading).
- [ ] T010 [US3] Create `crates/finder/tests/motion/imaging_align_track.rs` with the
  Track rig of spec.md § User Scenarios and one test for each of US3-S5 and US3-S6,
  under `#[tokio::test(start_paused = true)]`, and add `mod imaging_align_track;` to
  `crates/finder/tests/motion/main.rs` in its alphabetical place. On US2 the start does
  not read Track, so both fail (by reading).
- [ ] T011 [US3] Create `crates/finder/tests/mcp/imaging_align.rs` with its rig, the
  alignment rig at (700, 420) served with `/mcp` as the MCP binary's other tests serve
  theirs, and the bare MCP console, and one test for US3-S7, under
  `#[tokio::test(start_paused = true)]`, and add `mod imaging_align;` to
  `crates/finder/tests/mcp/main.rs` in its alphabetical place. On US2 the tools are
  unknown, so it fails (by reading).

### Implementation

- [ ] T012 [US3] In `crates/finder/src/imaging/align.rs`, add `DONE_FRAMES`, Track's
  words and `track_on`, call `Imaging::note_solve` for every solve, add `AlignLast`'s
  `aim` and `done` with the count of frames in a row in the middle, and add FR-011's
  and FR-012's refusals to the start and FR-011's check before every frame (FR-010 to
  FR-012; plan.md § Interfaces).
- [ ] T013 [US3] In `crates/finder/src/mcp/tools.rs`, add `AlignArguments`,
  `get_finder_alignment` and `set_finder_alignment`, each reading the imaging state
  before its arguments, with plan.md § Interfaces' descriptions (FR-013).
- [ ] T014 [US3] In `crates/finder/README.md`, add the MCP paragraphs, the two rows and
  `## Finder alignment` before `## Calibration frames`; in `GLOSSARY.md`, change
  **Purpose label**, **Lease** and the _Avoid_ lines of **Sync**, **Pointing model**,
  **Zero-point epoch** and **Relation**, and add **Finder alignment** before **Focus
  check**, each as plan.md § Interfaces gives it (FR-014).
- [ ] T015 [US3] Run the `lint` and `test` gates with every existing test unchanged and
  green, US1's and US2's test files among them, then commit the green story (FR-010 to
  FR-015; US3-S1, US3-S2, US3-S3, US3-S4, US3-S5, US3-S6, US3-S7).

## Phase 4: User Story 4 - The Tools page holds the finder alignment (Priority: P2)

### Tests first

- [ ] T016 [US4] Create `crates/finder/web/src/components/AlignmentBlock.test.tsx` with
  one test for each of US4-S1, US4-S2, US4-S3 and US4-S4, and
  `crates/finder/web/src/components/ToolsPage.test.tsx` with one test for each of US4-S5
  and US4-S6, stubbing `fetch` and `EventSource` as the page's existing tests do.
- [ ] T017 [US4] Create `crates/finder/web/e2e/tools.spec.ts` with a test for US4-S7,
  its own copy of `imaging.spec.ts`'s off-red count, and its preparation through the
  API (Track off, one imaging frame solved), and add `"**/tools.spec.ts"` to the
  default project's `testIgnore` and the simulate project's `testMatch` in
  `crates/finder/web/playwright.config.ts`, each on a line of its own (spec owner
  approval 6).

### Implementation

- [ ] T018 [US4] In `crates/finder/web/src/`, add the types and calls to `api.ts`,
  create `components/AlignmentBlock.tsx` and `components/ToolsPage.tsx`, add `/tools` to
  `App.tsx` with the session passed and the frame picker's home navigation, and add the
  `Tools` link to `components/SetupPage.tsx` (FR-016 to FR-019; plan.md § Interfaces).
  Run `npm run build` and commit `crates/finder/web/dist` (FR-020).
- [ ] T019 [US4] Run the `lint` and `test` gates with every existing test unchanged and
  green, then commit the green story (FR-016 to FR-020; US4-S1, US4-S2, US4-S3, US4-S4,
  US4-S5, US4-S6, US4-S7).

## Execution Order

```text
US1 -> US2 -> US3 -> US4
```

- **US1** starts from a `main` that holds every spec of `depends_on_landed`.
- **US2** starts once US1 has merged: its loop calls US1's `centre_of`, and its tests
  point the simulator off the frame.
- **US3** starts once US2 has merged: it adds two keys and two refusals to US2's code,
  and its tests use US2's helpers.
- **US4** starts once US3 has merged: the page reads US3's `done` and `aim`, and its
  browser test meets US3's refusals.
- **`[P]`** marks nothing here: each story's implementation follows its tests.
