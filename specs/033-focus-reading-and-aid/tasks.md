---
description: "Task list for 033-focus-reading-and-aid: Focus Reading and Focus Aid"
---

# Tasks: Focus Reading and Focus Aid

**Prerequisites** (plan.md § Prerequisites):
- `030-imaging-camera-link`, `031-imaging-frames-on-console` and
  `032-imaging-solve-and-aim-point` have landed on `main`, and this trio has been re-anchored to them: not met (plan.md § Prerequisites
  1 and 2);
- 031's trio carries the imaging camera's lease of the plan document's amendment 1 as
  its User Story 6, with `RoiEdit` defined, and this trio cites both as 031 names
  them: met in the trios as drafted, and checked in 031's code at the re-anchoring
  (plan.md § The camera lease, § Prerequisites 2 and 11);
- the owner decisions in spec.md are settled and its owner approvals given: not met;
- the owner has amended Constitution V or written how it reads for the imaging solver:
  not met (plan.md § Prerequisites 4);
- the trio has been probe-built: not met (plan.md § Prerequisites 5);
- the plan document, ADR-0002, the research note and the changed `GLOSSARY.md` are on
  `main`, and the glossary carries the two new entries and the three reworded ones of
  spec.md owner approval 11: not met on `main`; the drafting worktree's glossary carries
  the two new entries, and the three rewordings wait for the owner (plan.md
  § Prerequisites 6);
- the plan document's § 033, route table, storage line and module tree carry what this
  trio builds, while its "Settled decisions" row for Focus stands as written and spec.md
  owner decisions 6, 7, 8 and 23 are readings of it: met in the drafting worktree, and
  on `main` once the plan document is (plan.md § Prerequisites 9);
- no operator-staged file is needed: met.

**Rules for every story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml`,
  `CODEOWNERS`, `ergane.yaml`, `.specify/` or `crates/finder/web/playwright.config.ts`.
  If an existing test fails, stop and escalate with its exact output. Do not edit it.
  The one exception is the `mod` line of each new finder test module, added in
  alphabetical order to the binary's `main.rs` your story's tasks name, which holds no
  test (spec.md owner approval 12). Change nothing else there. `cargo fmt --all
  --check` fails a `mod` line that is out of order.
- New tests go in new files: the ones your story's tasks name. A new finder test is a
  module file in its binary's folder, never a file at the top level of
  `crates/finder/tests` or in `crates/finder/tests/support`, either of which fails the
  `layout` test. It binds the live helpers with `use crate::support::live as support;`,
  never with `#[path]`, and holds only the setup its own tests use, since an unused
  helper fails `clippy -D warnings`. A module that needs the scripted solver or the
  focus rig writes it out itself from spec.md § User Scenarios; it does not import
  another test module's helpers, and no story adds tests to another story's module.
  Every file a test makes is inside a `crate::support::Scratch`.
- Write only the files your story's tasks name; they are its `writes` in plan.md
  § Refinement Record. A story that needs another file escalates. No story writes
  `GLOSSARY.md`, `fixtures/`, `docs/`, or anything under `crates/scope-node`,
  `crates/scope-link` or `proto/`; under `crates/simulator` only US1 writes, and only
  `crates/simulator/src/optics.rs`.
- Add no dependency and change no `Cargo.toml` or `Cargo.lock`. Write no `unsafe`.
- Write nothing in `crates/finder/src/imaging/focus.rs` or under
  `crates/finder/src/imaging/focus/` that imports or calls the mount, an axis position,
  an altitude or an azimuth, and no size in pixels or arcseconds of one camera or
  telescope (plan.md § Plan traps 20). `aid.rs` reads no clock.
- Do not change 031's `take`, `set`, `abort` or lease, their refusals, or the imaging
  settings' keys. Do not add a method to the trait `ImagingSolver` or a field to
  `ImagingOptics`, `SolveFrame`, `Region` or `StarMeasure`.
- The focus aid takes 031's lease (031 US6) and releases it. Add no lock, flag or
  queue of this spec's own that keeps anyone else from the imaging camera (spec.md
  SC-007, plan.md § Plan traps 26). A test of the lease asserts a 409 and that the
  words contain the holder's name, never the rest of 031's sentence, `the imaging
  camera is held by {holder}` (plan.md § Plan traps 27). If 031 as landed has no
  `Imaging::lease`, `Imaging::holder`, `Lease::set` or `Lease::take`, stop and
  escalate: do not add them.
- Tests take their expected values from spec.md, which works each one from the
  simulator's constants, the published SplitMix64 stream or a worked example (plan.md
  § Plan traps 6). They compare `focus` and its parts key by key. No test of a story
  asserts something a later story changes (plan.md § Plan traps 14): above all, no
  test of US5 sends `check`, asserts that it is refused, or asserts that the aid's
  part of the view has no `check` key.
- Every scenario's Then clause is the test: assert each value it states, with the
  tolerance it states, against the oracle it names. Where a scenario says a test checks
  something first (the star counts of US2-S4 and US8-S2), that check is an assertion
  too and comes first. Where a scenario waits for an exact count of readings (US5-S1,
  US5-S6), the test asserts on the first view in which the count is reached.
- Use the words of `GLOSSARY.md`: focus reading, focus region, focus record, focus aid,
  imaging frame, imaging solver, solver port, solver adapter, scope node, console.
  Never "driver", "plugin", "provider", "light frame", "main camera", "focus score" or
  "autofocus". "Measure" alone is the solver port's star measure. No string shown to
  the observer says focus is good without naming the record (plan.md § Plan traps 25).
- Keep the story's diff under 64 KiB, measured as plan.md § Plan traps 3 says.
- Every code citation is `path:line` as read at 49bb210 and checked there again in the
  series audit's pass, and every file of 030, 031 and 032 is named without a line. If a
  cited line has moved, re-read it rather than guess.
- Do not edit `specs/033-focus-reading-and-aid/`.
- Run no tool the gates don't run: no `astap_cli`, `cargo deny`, `cargo machete`, Rust
  1.96 or `ergane`.

## Phase 1: User Story 1 — The focus reading, and a focuser to turn on the simulator (Priority: P1)

### Tests first

- [ ] T001 [US1] Create `crates/finder/tests/console/imaging_focus_reading.rs`, the
  module `imaging_focus_reading` of finder's `console` test binary, and add
  `mod imaging_focus_reading;` after 032's `mod imaging_astap;` in
  `crates/finder/tests/console/main.rs`. Give it, from spec.md § User Scenarios: a
  simulator built as the focus rig's, pointed with `push_to(40.0, 0.0)`; a helper that
  describes an imaging frame (whole, at binning 1 or 2, or with a region of interest)
  without a file; and the scripted solver. Write failing tests for the simulated
  solver's reading at focus errors of 0, 2.0 and −2.0 (US1-S1), the scripted worked
  examples of binning and region (US1-S2), and the rule of five stars and 20 frame
  pixels (US1-S3).
- [ ] T002 [US1] Add failing tests to
  `crates/finder/tests/console/imaging_focus_reading.rs` for the seeded noise and the
  sharpest point of the simulated solver adapter (US1-S4), and, on a simulated console
  built in process as `crates/finder/tests/console/live/whole_loop.rs` builds one, for
  `GET` and `POST /api/sim/focus` and their refusals (US1-S5).

### Implementation

- [ ] T003 [US1] In `crates/simulator/src/optics.rs`, add `focus_error` and
  `set_focus_error` (plan.md § The simulator's focuser). In
  `crates/finder/src/imaging/simulated.rs`, add `with_measure_noise` and
  `with_sharpest_at` and the last line of `measure`, with `unit` exactly as plan.md
  § The simulated solver adapter gives it; without either builder the adapter answers
  as before.
- [ ] T004 [US1] Create `crates/finder/src/imaging/focus.rs` with `MIN_STARS`,
  `FAR_OUT_PX`, `FocusReading`, `Unmeasurable`, `Read` and `read`, its four steps in
  the order plan.md § The focus reading gives, and add `pub mod focus;` to
  `crates/finder/src/imaging/mod.rs`.
- [ ] T005 [US1] In `crates/finder/src/api.rs`, add the route `/api/sim/focus` with
  `sim_focus` and `sim_set_focus` beside the other simulate routes, the key
  `focus_error` and the words `focus_error must be a number from -100 to 100` (plan.md
  § The simulate route). Add the route's row to the simulate table of
  `crates/finder/README.md`.
- [ ] T006 [US1] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US1-S1, US1-S2, US1-S3, US1-S4 and US1-S5).

## Phase 2: User Story 2 — The focus region, found from a full imaging frame or fixed by hand (Priority: P1)

### Tests first

- [ ] T007 [US2] Create `crates/finder/tests/console/imaging_focus_region.rs`, the
  module `imaging_focus_region` of `console`, with `mod imaging_focus_region;` after
  US1's `mod imaging_focus_reading;` in `crates/finder/tests/console/main.rs`, the
  scripted solver and the frame helper written out again. Write failing tests for the
  tiles' rectangles (US2-S1), the scripted search and its calls (US2-S2), and the
  search's refusals, ties, errors and far-out tiles, with `far_out` named only when
  there is no `best` (US2-S3).
- [ ] T008 [US2] Add failing tests to
  `crates/finder/tests/console/imaging_focus_region.rs` for the search on the
  simulator with a sharpest point, each after the test's own count of the tile's stars
  (US2-S4), and for `Kept` and `focus.json` in a `Scratch`, parsed with `serde_json`
  (US2-S5). Assert the file's `region`; do not assert that `record` is absent.

### Implementation

- [ ] T009 [US2] In `crates/finder/src/imaging/focus.rs`, add `GRID`, `Tile`,
  `tile_region`, `TileRead`, `Search` with its `best` and `far_out`, and `find_region`
  (plan.md § The focus region): nine tiles in row-major order, one `read` after
  another, the smallest `px` with `<` for `best`, and, only without a `best`, the
  smallest far-out size with `<` for `far_out`.
- [ ] T010 [US2] In the same file, add `FORMAT`, `Source`, `KeptRegion` with `found`,
  `fixed` and `tile`, and `Kept` with `new`, `describe`, `region` and `keep_region`,
  writing `focus.json` whole under a `.part` name and renaming it, and reading nothing
  from a file that is missing, unreadable, of another format or of another camera
  (plan.md § What is kept).
- [ ] T011 [US2] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US2-S1, US2-S2, US2-S3, US2-S4 and US2-S5).

## Phase 3: User Story 3 — The focus record and the verdict (Priority: P1)

### Tests first

- [ ] T012 [US3] Create `crates/finder/tests/console/imaging_focus_record.rs`, the
  module `imaging_focus_record` of `console`, with `mod imaging_focus_record;` after
  `mod imaging_focus_reading;` in `crates/finder/tests/console/main.rs`. Write failing
  tests for `judge` at 1.097, 1.103, 1.297 and 1.303 of the reference and for the
  unmeasurable cases (US3-S1); for `note`, `record` and `reference` over two nights,
  with the smaller part judging until a `note` marks tonight's as passed (US3-S2); for
  the file's `record` beside its `region`, with `passed` in the file only once it is
  true (US3-S3); and for the 5% rule on measured scales, with a record that holds no
  scale never set aside (US3-S4).

### Implementation

- [ ] T013 [US3] In `crates/finder/src/imaging/focus.rs`, add `NIGHT_S`,
  `GOOD_FRACTION`, `SOFT_FRACTION`, `SCALE_TOLERANCE`, `Verdict` with `as_str`, and
  `judge` in the order plan.md § The record and the verdict gives.
- [ ] T014 [US3] In the same file, add `Mark`, `RecordView` with its `passed`, `Of`,
  and `Kept`'s `note` with its four arguments and six steps, `record` and `reference`
  with its three cases, exactly as plan.md § The record and the verdict gives them,
  with the record in `focus.json` written by the one `save` that also writes the
  region, and `passed` left out of the file while false. Change no signature US2's
  tests call.
- [ ] T015 [US3] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US3-S1, US3-S2, US3-S3 and US3-S4).

## Phase 4: User Story 4 — The focus aid's arithmetic (Priority: P1)

### Tests first

- [ ] T016 [US4] Create `crates/finder/tests/console/imaging_focus_arithmetic.rs`, the
  module `imaging_focus_arithmetic` of `console`, with
  `mod imaging_focus_arithmetic;` before `mod imaging_focus_reading;` in
  `crates/finder/tests/console/main.rs`. It needs no rig: each test pushes samples into
  an `Aid`. Write failing tests for the 39 samples through focus and back (US4-S1),
  the slow turn (US4-S2), the unmeasurable frames and the history (US4-S3), the kept
  best and the history's length (US4-S4), and the three sequences for found (US4-S5).

### Implementation

- [ ] T017 [US4] Create `crates/finder/src/imaging/focus/aid.rs` with the eight
  constants, `Sample` and its `From<&Read>`, `Phase`, `Trend`, `Median`, `Step` and
  `Aid` with `push`'s four cases, `best`, `found`'s stages and `history`, exactly as
  plan.md § The aid's arithmetic gives them, and add `pub mod aid;` to
  `crates/finder/src/imaging/focus.rs`. The table in that section is the first test's
  answer, step by step.
- [ ] T018 [US4] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US4-S1, US4-S2, US4-S3, US4-S4 and US4-S5).

## Phase 5: User Story 5 — The focus aid on the console (Priority: P1)

This story does not write `check`: User Story 8 adds it. Do not send, refuse or
assert `check` here.

### Tests first

- [ ] T019 [US5] Create `crates/finder/tests/console/imaging_focus_aid.rs`, the module
  `imaging_focus_aid` of `console`, with `mod imaging_focus_aid;` after 032's
  `mod imaging_astap;` in `crates/finder/tests/console/main.rs`. Give it the focus rig
  and the scripted solver of spec.md § User Scenarios, with its slow measure and slow
  solve, and helpers to wait on `GET /api/imaging`, to list the frames folder and to
  parse `focus.json`. Write failing tests for a start that finds the region, runs and
  is stopped, with the first settled reading judged `no_record` and the next ones
  `good` against it (US5-S1), and for the focus object's shapes in the imaging view
  and in the session and every refusal (US5-S2).
- [ ] T020 [US5] Add failing tests to
  `crates/finder/tests/console/imaging_focus_aid.rs` for a tile fixed by hand and found
  again (US5-S3); for the lease: the aid holding the camera against a frame, a settings
  change and a second lease, a start refused while the test holds the lease, and a
  start refused with the test's name while the test's own frame is exposing (US5-S4);
  and for the endings: a solver error, a stop and an abort during a long exposure, no
  tile with five stars, and a lost link, each leaving the lease free (US5-S5).
- [ ] T021 [US5] Add failing tests to
  `crates/finder/tests/console/imaging_focus_aid.rs` for a start with every tile far
  out: the tile read for the time being, the second search at the first settled
  reading and the solver's calls in order (US5-S6); and for an imaging solve asked for
  while the aid measures a saved frame, whose frame and solve sidecar stay (US5-S7).

### Implementation

- [ ] T022 [US5] Create `crates/finder/src/imaging/focus/run.rs` with `PURPOSE`,
  `HOLDER`, `FOCUS_EXPOSURE_MS`, `FocusAsk` (`find`, `tile`, `exposure_ms`;
  `#[non_exhaustive]`), `TileAsk`, the focus state's type `State` and the view's types
  without `check` (plan.md § The aid on the console, § The focus view, § The imaging
  state), and add `pub mod run;` to `crates/finder/src/imaging/focus.rs`. In
  `crates/finder/src/imaging/mod.rs`, hold one `focus::run::State` made with
  `focus.json` in the imaging folder, call `describe` where the camera's description
  is kept, and add `focus` to `ImagingView`, filled only while `ready` with a solver;
  its `region` is the provisional tile of a run that has one and is not ended, else
  the kept region.
- [ ] T023 [US5] In `crates/finder/src/imaging/focus/run.rs`, add `focus_start` with
  its eight refusals in order, the fourth only while nobody holds the lease and the
  last of them 031's lease (031 US6) asked for as `HOLDER`; the task's five steps
  (remember, choose the region, set, one frame after another, end) with the search,
  the provisional tile and step 4j; `focus_stop` and `focus_running` (plan.md § The
  camera lease, § The aid on the console). The task owns the lease, sets and takes through
  it, takes each frame's id from the answered view's `frame.id`, holds a `Weak` of the
  `Imaging` and no lock across an await. It reads the reference before it notes a
  settled reading (steps 4g and 4h), and gives `Kept` the measured scale, never
  `Imaging::scale_arcsec_per_px()`. It removes only the file of its own previous
  frame, and that frame's solve sidecar, and not a frame the view's `solve` is
  `solving`. On every ending it puts the settings back and then drops the lease,
  before the lifecycle becomes `ended`.
- [ ] T024 [US5] In `crates/finder/src/imaging/routes.rs`, add `POST` and `DELETE
  /api/imaging/focus` (plan.md § The routes); in `crates/finder/src/lib.rs`, add the
  two routes' lines as US5 writes them; in `crates/finder/README.md`, add the two API
  rows as US5 writes them, without `check` (plan.md § `crates/finder/src/lib.rs` and
  `crates/finder/README.md`).
- [ ] T025 [US5] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US5-S1, US5-S2, US5-S3, US5-S4, US5-S5, US5-S6 and
  US5-S7).

## Phase 6: User Story 8 — The focus check, and a pass through focus on the simulator (Priority: P1)

Built after User Story 5 and before User Stories 6 and 7 (spec.md § Work Graph).

### Tests first

- [ ] T026 [US8] Create `crates/finder/tests/console/imaging_focus_check.rs`, the
  module `imaging_focus_check` of `console`, with `mod imaging_focus_check;` after
  US4's `mod imaging_focus_arithmetic;` in `crates/finder/tests/console/main.rs`, the
  focus rig and the scripted solver written out again, and a helper that writes
  `focus.json` by hand. Do not add to `imaging_focus_aid.rs`. Write failing tests for
  the focus check on that file with each of its verdicts and endings, the two cases
  of which part of the record judges, the far-out start, the record left unchanged,
  the lease free and `focus_ended` (US8-S1).
- [ ] T027 [US8] Add a failing test to
  `crates/finder/tests/console/imaging_focus_check.rs` for the simulator's focus turned
  through best and back, after the test's own count of eight stars in the middle tile,
  with `passed` in the file only at the end (US8-S2). Create
  `crates/finder/tests/cli/imaging_focus.rs`, the module `imaging_focus` of `cli`, with
  `mod imaging_focus;` after 031's `mod imaging;` in `crates/finder/tests/cli/main.rs`,
  and in it a failing test for a focus check through the built binary under
  `--simulate` (US8-S3).

### Implementation

- [ ] T028 [US8] In `crates/finder/src/imaging/focus/run.rs`, add `check` to `FocusAsk`,
  to the run and to `AidView`, `CHECK_FRAMES`, the ending `done`, step 4i, the rule
  that a check notes nothing (step 4h), and `focus_ended` (plan.md § The aid on the
  console, § The focus view). Change nothing a request without `check` does.
- [ ] T029 [US8] In `crates/finder/src/main.rs`, give the simulated solver adapter its
  noise and sharpest point under `--simulate` (plan.md § The binary). In
  `crates/finder/src/lib.rs`, change the `POST /api/imaging/focus` line to name
  `check`; in `crates/finder/README.md`, change that route's row the same way and add
  the `## Focus` section with everything plan.md § `crates/finder/src/lib.rs` and
  `crates/finder/README.md` lists for it.
- [ ] T030 [US8] Run the `lint` and `test` gates with every existing test unchanged and
  green, US5's among them. Commit the green story (US8-S1, US8-S2 and US8-S3).

## Phase 7: User Story 6 — The Focus block on the Imaging page (Priority: P2)

### Tests first

- [ ] T031 [US6] Create `crates/finder/web/src/components/FocusBlock.test.tsx` with
  failing tests for US6-S1, US6-S2, US6-S3, US6-S4 and US6-S5, stubbing `fetch` as
  031's and 032's Imaging page tests do. Each asserted phrase is compared whole.

### Implementation

- [ ] T032 [US6] Add the focus types, `focus` in `ImagingView`, `postImagingFocus`,
  `deleteImagingFocus` and `putImagingWholeSensor` to `crates/finder/web/src/api.ts`;
  create `crates/finder/web/src/components/FocusBlock.tsx` as
  `<section aria-labelledby="focus-title">` with
  `<h2 id="focus-title" className="section-title">Focus</h2>`, with the words, the
  request bodies' key order and the history of plan.md § The page, setting no colour
  and no `style`; and place it in `crates/finder/web/src/components/ImagingPage.tsx`
  right after 032's Imaging solve block.
- [ ] T033 [US6] Add the Focus block entry, the Display line's clause and the Imaging
  page entry's sentence to `DESIGN.md`, and the Focus block component to
  `.impeccable/design.json` (plan.md § `DESIGN.md` and `.impeccable/design.json`). Run
  the `lint` and `test` gates with every existing Vitest and Playwright test unchanged
  and green, commit the rebuilt `crates/finder/web/dist`, and commit the green story
  (US6-S1, US6-S2, US6-S3, US6-S4 and US6-S5).

## Phase 8: User Story 7 — Two MCP tools (Priority: P2)

### Tests first

- [ ] T034 [US7] Create `crates/finder/tests/mcp/imaging_focus.rs`, the module
  `imaging_focus` of finder's `mcp` test binary, with `mod imaging_focus;` after 031's
  `mod imaging;` in `crates/finder/tests/mcp/main.rs`, the focus rig and the scripted
  solver written out again and MCP on. Write failing tests for the tool list (US7-S1),
  a focus check that waits and the read that follows (US7-S2), the start, the stop and
  every refused argument (US7-S3), and a console without imaging or without a solver,
  with no result holding the token (US7-S4).

### Implementation

- [ ] T035 [US7] In `crates/finder/src/mcp/tools.rs`, add `FocusAidArguments`,
  `FocusTileArguments`, `get_focus_reading` and `set_focus_aid`, each reading its own
  arguments so that `{}` is a tool result, with the descriptions and words of plan.md
  § The MCP tools; after a check, a `focus_ended()` of `None` is the error
  `get_focus_reading` gives then. Add the two tools to the list in `## MCP clients` of
  `crates/finder/README.md`.
- [ ] T036 [US7] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US7-S1, US7-S2, US7-S3 and US7-S4).

## Execution Order

```text
US1 ──► US2 ──► US3 ──► US4 ──► US5 ──► US8 ──┬──► US6
                                               └──► US7
```

- **US1** comes first: every later story measures through `focus::read`, and the
  simulator's focuser and the simulated solver adapter's two builders are what the
  later scenarios turn.
- **US2** adds the tiles and `Kept` to US1's `focus.rs`.
- **US3** adds the record to US2's `Kept` and its file.
- **US4** is arithmetic alone; it follows US3 only because both write `focus.rs` and
  the `console` binary's `main.rs`.
- **US5** puts US1 to US4 on the console: the routes, the loop, the lease and the view.
- **US8** adds the focus check to US5's loop, and proves the whole on the simulator and
  through the built binary. It has the number 8 because it was split from US5 in
  review; it is the sixth story built.
- **US6** shows US5's and US8's view and calls their routes; it touches only
  `crates/finder/web/`, `DESIGN.md` and `.impeccable/design.json`.
- **US7** calls US5's and US8's actions; it writes the README after US8.
- **`[P]`**: US6 and US7 share no file and can run side by side once US8 has merged.
  Nothing else here is parallel: every other story shares a file with the one before
  it, and every task shares a file with another in its story.
