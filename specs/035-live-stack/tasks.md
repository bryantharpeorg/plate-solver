---
description: "Task list for 035-live-stack: Live Stack"
---

# Tasks: Live Stack

**Prerequisites** (plan.md § Prerequisites):
- `030-imaging-camera-link`, `031-imaging-frames-on-console`, `032-imaging-solve-and-aim-point`,
  `033-focus-reading-and-aid` and `034-dark-and-flat-frames` have landed on `main`: not
  met, all five are drafts;
- this trio has been re-anchored against `main` after they landed, every plain path of a
  file they create given its line: not met;
- 031 carries the camera's lease of the plan document's amendment 1 (031 US9) and the
  colour hold `useColourHold` (031 US7, from the audit's finding A6), and plan.md § The
  camera's lease and § The page have been read against 031's plan.md and code (plan.md
  § Prerequisites 8): met in the drafts by the series audit's findings on 031, not met
  on `main`;
- 032 offers `Imaging::note_solve` (032 US4, from the audit's finding A3), read against
  032's code (plan.md § Prerequisites 14): met in the drafts, not met on `main`;
- 033's revision has settled and this trio has been read against what it became
  (plan.md § Prerequisites 5): met in the drafts, not met on `main`;
- 036 sets a stack's exposure through the stack's lease, as plan.md § Shapes that
  existing tests pin, **The seam for 036 and 037**, asks, and 037 has been read
  against this trio, its `run::label` calling this trio's `label` (plan.md
  § Prerequisites 10): met for both as drafted;
- 034 takes the camera's lease for a set of darks or flats and has no
  `restore_exposure` (034 A1; plan.md § Prerequisites 13): met in the drafts, not met
  on `main`;
- the owner decisions in spec.md are settled and its owner approvals given: not met
  (decision 13 was settled on 2026-11-17, a folder per run, and is written in);
- every story has been probe-built: not met;
- the plan document, ADR-0002, the research note and the plan document's rewrite of
  `GLOSSARY.md` are on `main`: not met;
- no operator-staged file is needed: met.

**Rules for every story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml`,
  `CODEOWNERS`, `ergane.yaml` or `.specify/`. If an existing test fails, those of 030 to
  034 and of this spec's earlier stories included, stop and escalate with its exact
  output. Do not edit it. Two exceptions, both named in spec owner approval 9: the `mod`
  line of each new finder test module, added in alphabetical order to the binary's
  `main.rs` your story's tasks name, which holds no test; and, in US9 alone, the two
  list entries in `crates/finder/web/playwright.config.ts`. Change nothing else in
  those files.
- New tests go in new files: the ones your story's tasks name. A new finder test is a
  module file in the binary's folder, never a file at the top level of
  `crates/finder/tests` or in `crates/finder/tests/support`, either of which fails the
  `layout` test. It takes its scratch folder from `crate::support::Scratch`.
- Rigs are written once (spec.md § User Scenarios, **Rigs are written once**; plan.md
  § Test rigs). US5's module `stack_loop` holds them, `pub(crate)`; later `console`
  modules import them with `use crate::stack_loop::…`. Declare only what your own tests
  use, since an unused helper fails `clippy -D warnings`, and never edit an earlier
  story's test module to add one: if a helper you need is missing there, escalate.
- Write only the files your story's tasks name; they are its `writes` in plan.md
  § Refinement Record. A story that needs another file escalates.
- Tests compute their expected values themselves (plan.md § Plan traps 6): a FITS file is
  read with a reader of the tests' own, `stack.json` and solve sidecars with
  `serde_json`, pictures with `image`, and a sky place is projected by hand as spec.md
  § User Scenarios gives it. A test never calls `sky_at`, `pixel_of`, `between` or
  anything under `display` to make an expected value. Real numbers are compared within
  0.001 unless the scenario gives another tolerance.
- Keep the story's diff under 64 KiB, measured as plan.md § Plan traps 3 says.
- Every `path:line` was read at 49bb210, and read there again after the series audit,
  and names code that existed there, but for the `GLOSSARY.md` lines, which are of the
  plan document's rewrite; a file named without a line is created by 030, 031, 032, 033 or
  034, or by an earlier story of this spec. If a cited line has moved, re-read it
  rather than guess.
- Add no dependency and change no `Cargo.toml` or `Cargo.lock`. Change nothing under
  `crates/scope-node` or `crates/scope-link`, and outside US1 nothing under
  `crates/simulator`. Add no `unsafe`.
- Use the words of `GLOSSARY.md` exactly: imaging frame, sub, reference sub, stack,
  imaging solve, imaging solver, solver port, focus reading, focus region, master dark,
  master flat. Name no camera model, focal length, axis, altitude, azimuth or site in
  code under `crates/finder/src/imaging/stack.rs` or `crates/finder/src/imaging/stack/`,
  read no mount state there, and write no limit there as a number of seconds or of
  degrees (spec FR-019).
- The stack holds 031's lease on the camera and adds no lock of its own over it; it asks
  no other part whether it is taking imaging frames, and it changes no setting of the
  camera (plan.md § The camera's lease, § Plan traps 26, 27 and 32).
- A sub's hint comes from 032's `Imaging::hint` first, and that answer is used as it
  is when its source is a live pointing; the stack's own hint stands only in place of
  032's other sources (plan.md § Plan traps 30). Never put the stack's own hint ahead
  of a live pointing.
- A sub's solve reaches 032's state through `Imaging::note_solve` alone, once, for a
  stacked sub, with no lock held: never through `Imaging::solve` or 032's own task, and
  never so that it keeps a scale or shows as the page's `solve` (plan.md § Plan traps
  34).
- Hold neither the counts' lock nor the pixels' lock across an await or a call into
  `Imaging`; the pictures' own async lock (US7) is the one lock held across an await,
  and guards nothing but the making of a picture (plan.md § Plan traps 13). Run every
  pixel pass and every file write on the blocking pool (plan.md § Plan traps 14). Make
  no mean of the stack's full size but where `stack.fits` is written (plan.md § Plan
  traps 29).
- Do not edit `specs/035-live-stack/`.
- Run no tool the gates don't run: no `cargo deny`, `cargo machete`, Rust 1.96 or
  `ergane`.

## Phase 1: User Story 1 — A satellite's streak and a faint glow in the simulated sky (Priority: P1)

### Tests first

- [ ] T001 [US1] Create `crates/simulator/tests/imaging_scene.rs` with the sim rig of
  spec.md § User Scenarios and helpers to read a 16-bit sample at (x, y) of a frame and
  to take a frame of a given exposure from a camera built with noise off, no stars and
  gain 0. Write failing tests for the default scene and the unchanged frame (US1-S1)
  and for the streak on its one frame (US1-S2), each expected value computed in the
  test from the formula of the scenario's worked example.
- [ ] T002 [US1] Add failing tests to `crates/simulator/tests/imaging_scene.rs` for the
  glow at its four samples and at half the exposure (US1-S3), and for the cover, the
  flat panel and vignetting acting on the streak and the glow (US1-S4), computing 034's
  V in the test.

### Implementation

- [ ] T003 [US1] In `crates/simulator/src/imaging.rs`, add `SatelliteStreak`,
  `ExtendedSource` and `Scene` exactly as plan.md § The scene gives them. In
  `crates/simulator/src/optics.rs`, add the `imaging_scene` cell beside 034's
  `imaging_bench` and the methods `scene` and `set_scene` with their asserts. Export
  the three names from `crates/simulator/src/lib.rs`.
- [ ] T004 [US1] In `SimImagingCamera::expose` in `crates/simulator/src/imaging.rs`, read
  the scene and the frame number at step 1 and add each source and each satellite's
  streak as light in steps 5 and 6, before 034's V·D step, skipping both with an empty
  scene and under 034's cover or flat panel (plan.md § The scene, **Their part in a
  frame**).
- [ ] T005 [US1] Run the `lint` and `test` gates with every existing test unchanged and
  green, `crates/simulator/tests/imaging.rs` and
  `crates/simulator/tests/imaging_calibration.rs` included. Commit the green story
  (US1-S1, US1-S2, US1-S3 and US1-S4).

## Phase 2: User Story 2 — Colour from the mosaic, and where a sub falls on the reference (Priority: P1)

### Tests first

- [ ] T006 [US2] Create `crates/finder/tests/console/stack_colour.rs`, the module
  `stack_colour` of finder's `console` test binary, and add `mod stack_colour;` after
  `mod session;` in `crates/finder/tests/console/main.rs`. Write failing tests, as plain
  `#[test]`s over hand-built mosaics, for flat colours in all four patterns, mono and a
  one-pixel-wide mosaic (US2-S1) and for one lit sample (US2-S2).
- [ ] T007 [US2] Create `crates/finder/tests/console/stack_placement.rs`, the module
  `stack_placement`, with `mod stack_placement;` after `mod stack_colour;`. Write the
  by-hand projection of spec.md § User Scenarios, **Projecting by hand**, as private
  functions of the module. Write failing tests for the five exact maps (US2-S3), the
  shifted sub (US2-S4), the declination-60 case against the by-hand projection
  (US2-S5), the two `None` cases (US2-S6), and the 3° × 2° field whose near sub's
  corners lie 0.2 to 0.3 px from the by-hand projection and whose far sub is `None`
  (US2-S7).

### Implementation

- [ ] T008 [US2] Create `crates/finder/src/imaging/stack.rs` with `CORNER_SLACK_PX`,
  `Colour`, `colour` and `Placement` (`identity`, `between`, `apply`, `turn_deg`,
  `area_scale`, `centre_shift`) exactly as plan.md § The arithmetic gives them,
  `between` going through the sky at five points with 032's `sky_at` and `pixel_of`
  and then making the corner check at the reference grid's four corners (plan.md
  § The arithmetic, **`between`**; § Plan traps 33). Add `pub mod stack;` to
  `crates/finder/src/imaging/mod.rs`.
- [ ] T009 [US2] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US2-S1, US2-S2, US2-S3, US2-S4, US2-S5, US2-S6 and
  US2-S7).

## Phase 3: User Story 3 — Add subs on the reference grid (Priority: P1)

### Tests first

- [ ] T010 [US3] Create `crates/finder/tests/console/stack_accumulate.rs`, the module
  `stack_accumulate`, with `mod stack_accumulate;` after `mod session;` and before
  `mod stack_colour;`. Write failing tests, as plain `#[test]`s, for the identity added
  once and twice (US3-S1), the shifted and half-pixel cases and the two-sub mean
  (US3-S2), and the half turn, the quarter turn, the edge slack and the pixel that is
  not a number (US3-S3).
- [ ] T011 [US3] Add failing tests to `crates/finder/tests/console/stack_accumulate.rs`
  on the sim rig: three frames at three attitudes with a centroid helper of the
  module's own (US3-S4), and sixteen subs of the faint glow against four (US3-S5).

### Implementation

- [ ] T012 [US3] In `crates/finder/src/imaging/stack.rs`, add `EDGE_SLACK`, `Added` and
  `Stack` with `new`, `width`, `height`, `subs`, `add`, `weight`, `sum` and `mean`, the
  pass being the one loop of plan.md § The arithmetic, **`add`**: positions in `f64`,
  values in `f32`, no copy of the resampled sub, a pixel whose share is exactly 0 not
  read.
- [ ] T013 [US3] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US3-S1, US3-S2, US3-S3, US3-S4 and US3-S5).

## Phase 4: User Story 4 — A satellite's streak leaves no mark (Priority: P1)

### Tests first

- [ ] T014 [US4] Create `crates/finder/tests/console/stack_reject.rs`, the module
  `stack_reject`, with `mod stack_reject;` after `mod stack_placement;`. Write failing
  tests, as plain `#[test]`s, for `levels` (US4-S1), the background match (US4-S2) and
  the five rejection cases (US4-S3).
- [ ] T015 [US4] Add a failing test to `crates/finder/tests/console/stack_reject.rs` on
  the sim rig for the streak on one sub of twelve against a control stack and a plain
  mean (US4-S4).

### Implementation

- [ ] T016 [US4] In `crates/finder/src/imaging/stack.rs`, add `LEARNING_SUBS`,
  `REJECT_SIGMAS`, `SIGMA_FLOOR`, `LEVEL_GRID`, `levels`, `with_background_match` and
  `with_rejection`, with the running update of the squared deviations inside the one
  pass of `add` (plan.md § The arithmetic, **Background match** and **Rejection**;
  § Plan traps 9, 10 and 12). `Stack::new`, `add` and `Added` keep their signatures.
- [ ] T017 [US4] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US4-S1, US4-S2, US4-S3 and US4-S4).

## Phase 5: User Story 5 — The stack on the console: take, solve, place, add, stop (Priority: P1)

### Tests first

- [ ] T018 [US5] Create `crates/finder/tests/console/stack_loop.rs`, the module
  `stack_loop`, with `mod stack_loop;` after `mod stack_colour;`. Write, `pub(crate)`
  and with the names of plan.md § Test rigs, `Scripted`, `ScriptedSolver` with its
  `measure_script` and `measure_log` fields, `ScriptedCamera` with its `gain_script`
  field, `Rig`, `stand_in_rig`, `wait_stack` and `cards`, and no method that US5's own
  tests do not call. Write failing tests for a stack started, run and stopped, with
  the first hint 032's and the later ones the stack's own (US5-S1), and for the
  refusals of `start` in their order (US5-S2): among them the lease held by the test as
  `probe`, that lease held while a frame the test took with it is exposing, which must
  still name `probe`, a single imaging frame in flight with `holder()` `None`, a
  console without imaging, a regular file named `runs` that keeps the folder from
  being made, with `holder()` `None` afterwards, and a second stack in the same second,
  whose `id` ends `-2`. US5-S1 reads every sub in `runs/<id>/subs/` and no file under
  `frames/`.
- [ ] T019 [US5] Add failing tests to `crates/finder/tests/console/stack_loop.rs` for
  subs that do not solve (US5-S3), a failed exposure and three in a row, with the frame
  taken afterwards (US5-S4), a stop against a slow solver (US5-S5) and an aborted
  exposure (US5-S6).
- [ ] T020 [US5] Add failing tests to `crates/finder/tests/console/stack_loop.rs` for a
  stack on the sim stacking rig, started once the session's pointing is live, with a
  by-hand projection of the module's own and the hint's source read from two solve
  sidecars (US5-S7), for a single imaging frame refused while the stack holds the
  camera and taken after the stop (US5-S8), and for the aim point that five stacked
  subs keep measured through `note_solve`, read from `GET /api/session` against
  `sim.optics().eyepiece_px`, with no `solve` in the imaging view and the last sub's
  hint radius still 3.0 (US5-S9).

### Implementation

- [ ] T021 [US5] Create `crates/finder/src/imaging/stack/service.rs` with `HOLDER`,
  `FAILED_IN_A_ROW`, `StartRequest`, `OptionsView`, `Dropped`, `ReferenceView`,
  `LastView`, the `#[non_exhaustive]` `StackView` and `Stacker` with `new`, `start`,
  `stop`, `view` and `mean`, as plan.md § The stack on the console gives them, leaving
  out the fields and constants marked for US6, US7 and US8. Add `pub mod service;` to
  `crates/finder/src/imaging/stack.rs`. In 031's `crates/finder/src/imaging/mod.rs` and
  `crates/finder/src/imaging/frames.rs`, add `Lease::save_in` and the save of a frame
  taken under a lease that names a folder in that folder, exactly as plan.md § A
  lease's folder gives them, the folder kept with the lease's grant and cleared with
  it, changing nothing else of 031's take, save, cards or id rule.
- [ ] T022 [US5] In `crates/finder/src/imaging/stack/service.rs`, write `start` in its
  order, looking for an imaging frame in flight in step 5 only while
  `imaging.holder()` is `None`, taking 031's lease as `HOLDER` in step 6 and asking no
  other part whether it is taking imaging frames, and in step 7 making `runs/<id>/`,
  `-2` and on for a name that is taken, and `subs` in it, then calling
  `lease.save_in`, with the lease dropped and a 500 when the folder cannot be made
  (plan.md § The stack on the console, **The folder and the files**); the
  stopping mark, a `tokio::sync::watch` channel of `bool`; the taker, which owns the
  lease, takes every sub through it, looks at the stopping mark again right after each
  `take` and never waits for the worker; the worker's steps 1, 3, 4, 6 (`between`
  alone), 7 and 8, step 3 asking `imaging.hint` first and using the stack's own hint
  only when that answer does not come from a live pointing and the stack has a
  reference sub, and step 8 calling `imaging.note_solve(&frame, F.id, &solve)` after
  the counts, with the counts' lock let go; and the ending, done once, in which the
  lease is dropped when the taker returns, before `state` becomes `ended` (plan.md
  § The stack on the console, § The camera's lease, § Plan traps 13, 14, 16, 17, 26,
  27, 30, 34 and 35).
- [ ] T023 [US5] In `crates/finder/src/api.rs`, add the `stack` field and
  `AppState::stack()`, and build the `Stacker` in `with_imaging` right after 034's
  `Calibration`. Add the **Reference sub** entry to `GLOSSARY.md` after the **Stack**
  entry, with the three lines of spec owner approval 6.
- [ ] T024 [US5] Run the `lint` and `test` gates with every existing test unchanged and
  green, 031's `imaging_frames` and `imaging_lease` and 032's `imaging_relation`
  included. Commit the green story (US5-S1, US5-S2,
  US5-S3, US5-S4, US5-S5, US5-S6, US5-S7, US5-S8 and US5-S9).

## Phase 6: User Story 6 — The gate, and a queue the camera never waits on (Priority: P1)

### Tests first

- [ ] T025 [US6] Create `crates/finder/tests/console/stack_gate.rs`, the module
  `stack_gate`, with `mod stack_gate;` after `mod stack_colour;` and before
  `mod stack_loop;`. Import the rigs with `use crate::stack_loop::…` and write the
  traits `GateScripts` and `CameraScripts` of plan.md § Test rigs over the fields US5
  left. Write failing tests for `same_settings`, as a plain `#[test]` over
  `SubSettings` literals, and for a sub at another gain on the rig (US6-S1), and for
  the focus gate: a soft sub, too few stars, a failed measure, a far-out sub and
  `focus_gate` false (US6-S2).
- [ ] T026 [US6] Add failing tests to `crates/finder/tests/console/stack_gate.rs` for
  `judge` and its two counts on the rig (US6-S3), the queue of four against a slow
  solver (US6-S4), 032's hint after three solves in a row that miss, kept until a sub
  solves, and a focus gate's too few stars counting as no miss (US6-S5), and the kept
  focus region
  handed to every sub's star measure, set through 033's `POST` and
  `DELETE /api/imaging/focus` (US6-S6).

### Implementation

- [ ] T027 [US6] In `crates/finder/src/imaging/stack.rs`, add `OFF_FIELD_SHARE`,
  `AREA_TOLERANCE` and `judge`, and `GAIN_TOLERANCE`, `EXPOSURE_TOLERANCE_MS`,
  `SubSettings` and `same_settings`, with 032's `Region` added to the file's import
  (plan.md § The arithmetic, **`judge`** and **`same_settings`**).
- [ ] T028 [US6] In `crates/finder/src/imaging/stack/service.rs`, add `QUEUE_SUBS`,
  `FOCUS_GATE_MIN`, `FOCUS_GATE_EXCESS` and `REHINT_AFTER`; the settings test as the
  worker's step 2, through `stack::same_settings` against the reference sub's
  `SubSettings`, kept with the run; the focus gate as its step 5, through 033's
  `focus::read` with the region and the scale plan.md § The stack on the console,
  **The focus gate** gives; `judge` in step 6; the bounded queue that drops the sub
  that has waited longest as `behind`; `misses`, changed in step 4 and nowhere else,
  which sets the stack's own hint aside in step 3 while it is `REHINT_AFTER` or more
  (plan.md § Plan traps 31); `focus_arcsec` in `LastView` and `focus_median_arcsec` in
  `StackView`.
- [ ] T029 [US6] Run the `lint` and `test` gates with every existing test unchanged and
  green, US5's `stack_loop` included. Commit the green story (US6-S1, US6-S2, US6-S3,
  US6-S4, US6-S5 and US6-S6).

## Phase 7: User Story 7 — The display picture (Priority: P2)

### Tests first

- [ ] T030 [US7] Create `crates/finder/tests/console/stack_display.rs`, the module
  `stack_display`, with `mod stack_display;` after `mod stack_colour;` and before
  `mod stack_gate;`. Write failing tests, as plain `#[test]`s, for `reduce` over
  hand-built sums and weights (US7-S1), `background` (US7-S2), `tone` (US7-S3) and
  `render` of a `Stack` the test fills with one `add`, its JPEG and `shrink` (US7-S4).
- [ ] T031 [US7] Add a failing test to `crates/finder/tests/console/stack_display.rs` on
  the stand-in stacking rig, imported from `stack_loop`, for the kept picture, its name
  and `preview_within` (US7-S5).

### Implementation

- [ ] T032 [US7] Create `crates/finder/src/imaging/stack/display.rs` with its constants,
  `Reduced`, `Plane`, `reduce`, `background`, `tone`, `render` and `shrink`, exactly as
  plan.md § The display gives them, using 034's `percentile` and 031's `preview::Rgb`;
  `reduce` reads a stack's sums and weights block by block and builds no mean of the
  stack's full size.
  Add `pub mod display;` to `crates/finder/src/imaging/stack.rs`.
- [ ] T033 [US7] In `crates/finder/src/imaging/stack/service.rs`, add `DISPLAY_EVERY`
  and keep the latest picture and its JPEG under the pixels' lock. Make a picture in
  the three steps of plan.md § The stack on the console, **The picture**: `reduce`
  under the pixels' lock and nothing else there; `background`, `tone` and the JPEG
  with no lock held; then the keeping. Make one picture at a time, under the pictures'
  own async lock; make one when the queue is empty or after every eighth stacked sub,
  and when a request on a running stack names `gains` or `stretch`. Add `preview`,
  `preview_within` and the view's `preview` (plan.md § Plan traps 13, 18 and 29).
- [ ] T034 [US7] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US7-S1, US7-S2, US7-S3, US7-S4 and US7-S5).

## Phase 8: User Story 8 — Start, stop and keep a stack (Priority: P1)

### Tests first

- [ ] T035 [US8] Create `crates/finder/tests/console/stack_console.rs`, the module
  `stack_console`, with `mod stack_console;` after `mod stack_colour;` and before
  `mod stack_display;`. Import the rigs from `stack_loop`, and write a wait over
  `GET /api/imaging`, a request helper for an `Origin`, another `Content-Type` and
  `DELETE`, and `real_at`. Write failing tests for a stack started and stopped over the
  routes with its three files (US8-S1) and for the routes' refusals and the preview
  route (US8-S2).
- [ ] T036 [US8] Add failing tests to `crates/finder/tests/console/stack_console.rs` for
  the target's label, an empty stack with its empty `subs`, and `label`'s three
  worked names, `NGC 1976`, `M42` and `Barnard’s Loop` (US8-S3), the
  stack in the imaging view and the session, and none of it on a console without
  imaging (US8-S4), a folder that cannot be made and
  one removed mid-stack (US8-S5), and `encode_cube` (US8-S6).
- [ ] T037 [US8] Create `crates/finder/tests/cli/stack.rs`, the module `stack` of `cli`,
  with `mod stack;` after `mod simulate;` in `crates/finder/tests/cli/main.rs`, and in
  it a failing test for the built binary under `--simulate --state` (US8-S7).

### Implementation

- [ ] T038 [US8] In `crates/finder/src/imaging/fits.rs`, add `RealCube` and
  `encode_cube` beside 031's and 034's functions, reusing their card formatting and
  editing none of them (plan.md § FITS).
- [ ] T039 [US8] In `crates/finder/src/imaging/stack/service.rs`, add `FORMAT`, the
  public `label`, which maps each `char` of a name that is not an ASCII letter or digit
  to one `-`, `FilesView`, `stack.json` at the start in the folder US5 makes, with the
  lease dropped when it cannot be written, and at the end `stack.fits` with the cards
  of plan.md § FITS, `stack.png` from `display::render` of the stack at its own width,
  and `stack.json`, which names each sub and the reference sub by its path inside the
  folder, `subs/<frame id>.fits`, each under its name plus `.part` and renamed, with
  `files` or `save_error` in the view (plan.md § The stack on the console, **The folder
  and the files**).
- [ ] T040 [US8] In `crates/finder/src/imaging/mod.rs`, add `ImagingView.stack` and
  `attach_stack`, filling `stack` wherever the view is built, whatever its state. In
  `crates/finder/src/api.rs`, call `attach_stack` in `with_imaging` and add
  `target_label`, which hands the session target's Messier name, or its id, to
  `stack::service::label` and tests no character itself. In
  `crates/finder/src/imaging/routes.rs`, add the three routes in the
  order a request is judged in. Add their three lines to the route list in
  `crates/finder/src/lib.rs` (plan.md § 031's imaging state, § The console, § The
  routes).
- [ ] T041 [US8] Add the three rows and the `## Live stack` section to
  `crates/finder/README.md` (plan.md § `crates/finder/README.md`).
- [ ] T042 [US8] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US8-S1, US8-S2, US8-S3, US8-S4, US8-S5, US8-S6 and
  US8-S7).

## Phase 9: User Story 9 — The stack on the Imaging page (Priority: P2)

### Tests first

- [ ] T043 [US9] Create `crates/finder/web/src/components/StackBlock.test.tsx` with
  failing tests for the block without a stack and its start (US9-S1), a running stack's
  words (US9-S2), the picture and its night control with fake timers (US9-S3), an ended
  stack (US9-S4), `Apply colour` (US9-S5), and the block's place on `ImagingPage` with
  the read that follows a request (US9-S6), stubbing `fetch` as the existing page tests
  do.
- [ ] T044 [US9] Create `crates/finder/web/e2e/stack.spec.ts` with a failing Playwright
  test for the stack's picture at night (US9-S7), and add `"**/stack.spec.ts"` to the
  default project's `testIgnore` and the simulate project's `testMatch` in
  `crates/finder/web/playwright.config.ts`, each on a line of its own (spec owner
  approval 9).

### Implementation

- [ ] T045 [US9] Add `StackDropped`, `StackView`, `StackStart`, `postImagingStack` and
  `deleteImagingStack` to `crates/finder/web/src/api.ts`, and `stack` to
  `ImagingView`. Create `crates/finder/web/src/components/StackBlock.tsx` with the
  words of plan.md § The page, holding colour under its picture through 031's
  `useColourHold(night)` of `crates/finder/web/src/useColourHold.ts` (031 US7, from the
  audit's finding A6): the picture's class `photo` unless `held`, and `{...bind}` with
  `aria-pressed={held}` on `Hold to see colour`, writing no timer, listener or key test
  of its own, and place it in
  `crates/finder/web/src/components/ImagingPage.tsx` after the Calibration block.
- [ ] T046 [US9] Add the Stack block entry to `DESIGN.md` and the Stack block component
  to `.impeccable/design.json` (plan.md § `DESIGN.md` and `.impeccable/design.json`).
  Run the `lint` and `test` gates with every existing Vitest and Playwright test
  unchanged and green, commit the rebuilt `crates/finder/web/dist`, and commit the green
  story (US9-S1, US9-S2, US9-S3, US9-S4, US9-S5, US9-S6 and US9-S7).

## Phase 10: User Story 10 — Two MCP tools (Priority: P2)

### Tests first

- [ ] T047 [US10] Create `crates/finder/tests/mcp/stack.rs`, the module `stack` of
  finder's `mcp` test binary, with `mod stack;` after `mod session_tools;` in
  `crates/finder/tests/mcp/main.rs`, and a scripted solver of the module's own with
  `new`, `solve` and `measure` alone. Write failing tests for the tools' listing
  (US10-S1), a stack started, read with its picture and through `get_session`, changed
  and stopped (US10-S2), the refusals (US10-S3), and a console without imaging, the
  order its arguments are judged in there, and the token check (US10-S4).

### Implementation

- [ ] T048 [US10] In `crates/finder/src/mcp/tools.rs`, add `StackingArguments` and the
  tools `set_stacking` and `get_stack` after 034's tools, with the descriptions and
  errors of plan.md § The MCP tools, `set_stacking` judging its arguments in that
  section's six steps and removing `on` before the rest is parsed as `StartRequest`
  (plan.md § Plan traps 22).
- [ ] T049 [US10] Add the two tools' lines under `## MCP clients` in
  `crates/finder/README.md`.
- [ ] T050 [US10] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US10-S1, US10-S2, US10-S3 and US10-S4).

## Execution Order

```text
US1 ──► US2 ──► US3 ──► US4 ──► US5 ──► US6 ──► US7 ──► US8 ──► US9 ──► US10
```

- **US1** comes first: US3 and US4 take frames from the scene it adds.
- **US2** creates `stack.rs` and its first two test modules; it needs nothing of US1,
  and follows it only so that one chain orders every story.
- **US3** adds `Stack` to US2's file, and **US4** rejection and the background match to
  US3's `Stack`.
- **US5** runs all of it from the console's loop under 031's lease, saves every sub in
  the stack's own `subs/` through the lease, and writes the rigs every later `console`
  module imports. It follows US2 by a direct edge too, as the next story to write
  `crates/finder/src/imaging/mod.rs`.
- **US6** adds the gate to US5's worker and bounds its queue; **US7** gives the loop a
  picture to keep.
- **US8** puts the routes, the view and the files over the loop and reads US7's picture.
- **US9** calls US8's routes and shows its view; it touches only
  `crates/finder/web/`, `DESIGN.md` and `.impeccable/design.json`.
- **US10** comes last: it follows US9 only because both it and US8 write
  `crates/finder/README.md`.
- **`[P]`** marks nothing here: every story adds a line to a `main.rs` or a file another
  story writes, and the chain is one line.
