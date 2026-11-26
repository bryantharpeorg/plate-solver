---
description: "Task list for 050-tools-page-aligns-finder-camera: The Tools Page Aligns the Finder Camera"
---

# Tasks: The Tools Page Aligns the Finder Camera

**Prerequisites** (plan.md § Prerequisites, each with its evidence there):
- every spec in `depends_on_landed` has landed on `main`: met at ea3d6a9b;
- the owner decisions in spec.md are settled: met, all twelve, decisions 1 to 10
  answered (a) on 2026-11-26;
- every story has been probe-built from this text: met, probe round 1 built the four
  stories before this fold (0 blocking, 15 clarity, folded), and the five-story cut was
  proven on its code with every gate passing;
- the page half has run once: met on both probe rounds (`CI=1 npm run e2e`);
- no operator-staged file is needed: met.

**Rules for every story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml`,
  `CODEOWNERS`, `ergane.yaml` or `.specify/`. If an existing test fails, stop and
  escalate with its exact output. A test file an earlier story of this spec created is
  an existing test. The exceptions: each story's `mod` lines in
  `crates/finder/tests/console/main.rs`, `crates/finder/tests/motion/main.rs` or
  `crates/finder/tests/mcp/main.rs`, each in its alphabetical place, and US5's two list
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
- The results said here before each story's implementation are those of the re-cut of
  round 1's code (plan.md § What was run and what was read).

## Phase 1: User Story 1 - The console places the imaging camera's centre off the finder camera's frame (Priority: P1)

### Tests first

- [ ] T001 [US1] Create `crates/finder/tests/console/imaging_align_centre.rs` with one
  test for each of US1-S1, US1-S2, US1-S3, US1-S4 and US1-S5, under `#[test]`, building
  the centre frame as spec.md § User Scenarios gives it (and, for US1-S3's last step,
  with the solution's `distortion` −0.2), and add `mod imaging_align_centre;` to
  `crates/finder/tests/console/main.rs` in its alphabetical place. Before T003 the
  binary does not build (E0432, no `finder::imaging::align`); with T003 and without
  T002, US1-S1 to US1-S4 pass and US1-S5 panics at
  `crates/simulator/src/optics.rs:52`.

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
  `solver`, `imaging`, `focal_mm`, `interval_ms`, `folder`, the default being the rig:
  all on, 1800 mm, 500 ms, the rig's own scratch folder), its
  `pub struct Rig { pub app: Router, pub sim: Simulator, .. }` and
  `pub fn rig(e: (f64, f64), setup: Setup) -> Rig`, the `pub` helpers `call`,
  `wait_until`, `ready`, `last_after`, `start`, `oracle` and `assert_centre`, and one
  test for each of US2-S1, US2-S2, US2-S3, US2-S4, US2-S5 and US2-S6, under
  `#[tokio::test(start_paused = true)]`, and add `mod imaging_align;` to
  `crates/finder/tests/console/main.rs` in its alphabetical place. On US1 there is no
  route `/api/imaging/align`, so all six fail, each in `call` parsing the empty body of
  the 405 answer.

### Implementation

- [ ] T006 [US2] In `crates/finder/src/imaging/align.rs`, add `HOLDER`, `PURPOSE`,
  `ALIGN_EXPOSURE_MS`, `PAIR_S`, `SEARCH_RADIUS_DEG`, `AlignAsk`, `AlignLast` without
  `aim` and `done`, `AlignView`, `State` without `ended`, `view`, `state_refusal`,
  `Imaging::align_start` with FR-004's refusals, and the run's task, steps, frames and
  count of frames in a row that do not solve, a failed frame ending the run, with no
  stop, no settings remembered or put back, no frame removed, no call to `note_solve`
  and no Track or field-height check (FR-004 to FR-006; plan.md § Interfaces).
- [ ] T007 [US2] In `crates/finder/src/imaging/mod.rs`, add `Held`'s `align`,
  `ImagingView`'s `align` and the `Drop` line; in `crates/finder/src/imaging/routes.rs`,
  add the route with `post_align` only (FR-004, FR-006).
- [ ] T008 [US2] Run the `lint` and `test` gates with every existing test unchanged and
  green, US1's test file among them, then commit the green story (FR-004 to FR-006,
  FR-009; US2-S1, US2-S2, US2-S3, US2-S4, US2-S5, US2-S6).

## Phase 3: User Story 3 - The alignment stops and puts the camera back, says Done, and refuses Track and a missing field height (Priority: P2)

### Tests first

- [ ] T009 [US3] Create `crates/finder/tests/console/imaging_align_stop.rs` with one test
  for each of US3-S1 and US3-S2, under `#[tokio::test(start_paused = true)]`, using
  `super::imaging_align::{call, last_after, ready, rig, start, wait_until, Setup}`,
  US3-S1 handing the rig a `crate::support::Scratch` folder of its own through
  `Setup::folder` and listing its `frames`; and create
  `crates/finder/tests/console/imaging_align_aim.rs` with one test for each of US3-S3,
  US3-S4, US3-S5, US3-S6 and US3-S7, under `#[tokio::test(start_paused = true)]`, using
  `super::imaging_align::{assert_centre, call, last_after, ready, rig, start, wait_until, Setup}`;
  and add `mod imaging_align_aim;` and `mod imaging_align_stop;` to
  `crates/finder/tests/console/main.rs`, each in its alphabetical place. On US2: US3-S1
  and US3-S2 fail in `call` at the empty body of the 405 answer to `DELETE`; US3-S3
  reads `done` absent where `false` is due; US3-S4 finds no frame with `aim`
  `off_frame` and US3-S5 none with `no_live_pointing` and `done`; US3-S6 reads the
  session's `aim.source` not `measured`; and US3-S7 is answered 200 where 409 is due.
- [ ] T010 [US3] Create `crates/finder/tests/motion/imaging_align_track.rs` with the
  Track rig of spec.md § User Scenarios and one test for each of US3-S8 and US3-S9,
  under `#[tokio::test(start_paused = true)]`, and add `mod imaging_align_track;` to
  `crates/finder/tests/motion/main.rs` in its alphabetical place. On US2 the start does
  not read Track: US3-S8 is answered 200 where 409 is due, and US3-S9 waits out its
  300 s while the run goes on.

### Implementation

- [ ] T011 [US3] In `crates/finder/src/imaging/align.rs`, add `State`'s `ended`,
  `Imaging::align_stop` and `Imaging::align_running`, the stop's check before each frame
  and after it ends, the three settings remembered before the steps and put back after
  them at every end, the previous frame's files removed unless 032's solve is
  `solving` it (FR-007, FR-008); `DONE_FRAMES`, Track's words and `track_on`, a call to
  `Imaging::note_solve` for every solve, `AlignLast`'s `aim` and `done` with the count of
  frames in a row in the middle, and FR-011's and FR-012's refusals in the start and
  FR-011's check before every frame (FR-010 to FR-012; plan.md § Interfaces).
- [ ] T012 [US3] In `crates/finder/src/imaging/routes.rs`, add `.delete(delete_align)`
  to the route and `delete_align` (FR-007).
- [ ] T013 [US3] Run the `lint` and `test` gates with every existing test unchanged and
  green, US1's and US2's test files among them, then commit the green story (FR-007,
  FR-008, FR-010 to FR-012, FR-015; US3-S1, US3-S2, US3-S3, US3-S4, US3-S5, US3-S6,
  US3-S7, US3-S8, US3-S9).

## Phase 4: User Story 4 - MCP and the README carry the finder alignment (Priority: P2)

### Tests first

- [ ] T014 [US4] Create `crates/finder/tests/mcp/imaging_align.rs` with its rig, the
  alignment rig at (700, 420) served with `/mcp` as the MCP binary's other tests serve
  theirs, and the bare MCP console, one test for US4-S1 under
  `#[tokio::test(start_paused = true)]` and one for US4-S2 under `#[test]`, reading
  `crates/finder/README.md` from `env!("CARGO_MANIFEST_DIR")` as
  `crates/finder/tests/console/imaging_run_docs.rs` does, and add `mod imaging_align;`
  to `crates/finder/tests/mcp/main.rs` in its alphabetical place. On US3 the tools are
  unknown (`tool not found`) and the README has no line
  `The Tools page's finder alignment has two tools:`, so both fail.

### Implementation

- [ ] T015 [US4] In `crates/finder/src/mcp/tools.rs`, add `AlignArguments`,
  `get_finder_alignment` and `set_finder_alignment`, each reading the imaging state
  before its arguments, with plan.md § Interfaces' descriptions (FR-013).
- [ ] T016 [US4] In `crates/finder/README.md`, add the MCP paragraphs, the two rows and
  `## Finder alignment` before `## Calibration frames`; in `GLOSSARY.md`, change
  **Purpose label**, **Lease** and the _Avoid_ lines of **Sync**, **Pointing model**,
  **Zero-point epoch** and **Relation**, and add **Finder alignment** before **Focus
  check**, each as plan.md § Interfaces gives it (FR-014).
- [ ] T017 [US4] Run the `lint` and `test` gates with every existing test unchanged and
  green, the earlier stories' test files and `imaging_run_docs.rs` among them, then
  commit the green story (FR-013, FR-014, FR-021; US4-S1, US4-S2).

## Phase 5: User Story 5 - The Tools page holds the finder alignment (Priority: P2)

### Tests first

- [ ] T018 [US5] Create `crates/finder/web/src/components/AlignmentBlock.test.tsx` with
  one test for each of US5-S1, US5-S2, US5-S3 and US5-S4, and
  `crates/finder/web/src/components/ToolsPage.test.tsx` with one test for each of US5-S5
  and US5-S6, stubbing `fetch` and `EventSource` as the page's existing tests do.
- [ ] T019 [US5] Create `crates/finder/web/e2e/tools.spec.ts` with a test for US5-S7,
  its own copy of `imaging.spec.ts`'s off-red count, and its preparation through the
  API (Track off, one imaging frame solved), and add `"**/tools.spec.ts"` to the
  default project's `testIgnore` and the simulate project's `testMatch` in
  `crates/finder/web/playwright.config.ts`, each on a line of its own (spec owner
  approval 6). Before T020 to T022, `AlignmentBlock.test.tsx` fails to resolve
  `./AlignmentBlock`, US5-S5 finds no link `Tools` and US5-S7 times out on it; US5-S6
  passes before and after, since it pins the link's absence.

### Implementation

- [ ] T020 [US5] In `crates/finder/web/src/`, add the types and calls to `api.ts` and
  create `components/AlignmentBlock.tsx` (FR-016, FR-019; plan.md § Interfaces).
- [ ] T021 [US5] Create `components/ToolsPage.tsx`, add `/tools` to `App.tsx` with the
  session passed and the frame picker's home navigation, and add the `Tools` link to
  `components/SetupPage.tsx` (FR-017, FR-018). Run `npm run build` and commit
  `crates/finder/web/dist` (FR-020).
- [ ] T022 [US5] Run the `lint` and `test` gates with every existing test unchanged and
  green, then commit the green story (FR-016 to FR-020; US5-S1, US5-S2, US5-S3, US5-S4,
  US5-S5, US5-S6, US5-S7).

## Execution Order

```text
US1 -> US2 -> US3 -> US4 -> US5
```

- **US1** starts from a `main` that holds every spec of `depends_on_landed`.
- **US2** starts once US1 has merged: its loop calls US1's `centre_of`, and its tests
  point the simulator off the frame.
- **US3** starts once US2 has merged: it adds the stop, two keys and two refusals to
  US2's code, and its tests use US2's helpers.
- **US4** starts once US3 has merged: its tools start and stop US3's run and read its
  `done` and `aim`.
- **US5** starts once US4 has merged: the page reads US3's `done` and `aim`, and its
  browser test meets US3's refusals.
- **`[P]`** marks nothing here: each story's implementation follows its tests.
