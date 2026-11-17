---
description: "Task list for 032-imaging-solve-and-aim-point: Solving Imaging Frames and the Measured Aim Point"
---

# Tasks: Solving Imaging Frames and the Measured Aim Point

**Prerequisites** (plan.md § Prerequisites):
- `030-imaging-camera-link` and `031-imaging-frames-on-console` have landed on `main`,
  and this trio has been re-anchored to them: not met (plan.md § Prerequisites 1 and 2);
- the owner decisions in spec.md are settled and its owner approvals given: not met;
- the owner has amended Constitution V or written how it reads for the imaging solver:
  not met (plan.md § Prerequisites 4);
- the saved captures of ASTAP's real output are staged under `fixtures/finder/astap/`:
  not met (plan.md § Prerequisites 5); US1 cannot pass without them;
- the trio has been probe-built: not met (plan.md § Prerequisites 6);
- the drafts outside this series that write the same files (`026-mount-sky`,
  `021-chart-orientation`, `022-altaz-guidance`) are ordered against this spec: not
  met (plan.md § Prerequisites 9);
- owner decision 25's answer is written into the trio it names: met on 2026-11-17,
  when the owner chose the held aim point (FR-036); 037's owner decision 26 cites it,
  and 037's re-centre is written for it (plan.md § Prerequisites 11).

**Rules for every story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml`,
  `CODEOWNERS`, `ergane.yaml`, `.specify/` or `crates/finder/web/playwright.config.ts`.
  If an existing test fails, stop and escalate with its exact output. Do not edit it.
  The one exception is the `mod` line of each new finder test module, added in
  alphabetical order to the binary's `main.rs` your story's tasks name, which holds no
  test (spec.md owner approval 13). Change nothing else there.
- New tests go in new files: the ones your story's tasks name. A new finder test is a
  module file in its binary's folder, never a file at the top level of
  `crates/finder/tests` or in `crates/finder/tests/support`, either of which fails the
  `layout` test. It binds the live helpers with `use crate::support::live as support;`,
  never with `#[path]`, and holds only the setup its own tests use, since an unused
  helper fails `clippy -D warnings`. A module that needs the solving rig writes it out
  itself from spec.md § User Scenarios; it does not import another test module's
  helpers, and the 4 to 5 KiB this costs is counted in the story's budget (plan.md
  § Risks). Every file a test makes is inside a `crate::support::Scratch`.
- Write only the files your story's tasks name; they are its `writes` in plan.md
  § Refinement Record. A story that needs another file escalates. No story writes
  `GLOSSARY.md`, `fixtures/`, `docs/` or anything under `crates/simulator`,
  `crates/scope-node` or `crates/scope-link`.
- Add no dependency and change no `Cargo.toml` or `Cargo.lock`. Write no `unsafe`.
- Write nothing under `crates/finder/src/imaging/` that reads an axis position or
  computes an altitude, an azimuth or a field rotation, and no constant sized to one
  telescope (plan.md § Plan traps 20 and 21).
- Tests compute their expected values themselves (plan.md § Plan traps 6): the truth
  from the simulator, separations and position angles with their own formulas, files
  with their own readers. They compare `solve`, `relation` and `aim` key by key. No test
  of a story asserts something a later story changes (plan.md § Plan traps 13).
- Use the words of `GLOSSARY.md`: imaging frame, imaging solve, imaging solver, solver
  port, solver adapter, aim point, imaging angle, live pointing, star frame, finder
  camera. Never "driver", "plugin", "provider", "light frame" or "main camera". A
  reading of the aim point is an aim reading (`AimReading`, `readings`); "measure"
  alone is the solver port's star measure.
- Every scenario's Then clause is the test: assert each value it states, with the
  tolerance it states, against the oracle it names. Where a scenario says a test
  checks something first (US1-S5's `/proc/self`, US2-S2's count of 40 stars), that
  check is an assertion too and comes first.
- Keep the story's diff under 64 KiB, measured as plan.md § Plan traps 3 says.
- Every code citation is `path:line` as checked at 49bb210, and every file of 030 and 031
  is named without a line. If a cited line has moved, re-read it rather than guess.
- Do not edit `specs/032-imaging-solve-and-aim-point/`.
- Run no tool the gates don't run: no `astap_cli`, `cargo deny`, `cargo machete`, Rust
  1.96 or `ergane`.

## Phase 1: User Story 1 — The solver port and the ASTAP adapter (Priority: P1)

### Tests first

- [ ] T001 [US1] Create `crates/finder/tests/console/imaging_astap.rs`, the module
  `imaging_astap` of finder's `console` test binary, and add `mod imaging_astap;` after
  `mod imaging;` in `crates/finder/tests/console/main.rs`. Give it, from spec.md § User
  Scenarios: a helper that writes the stub program into a `Scratch` with mode 0700 and
  its `code` and `reply.*` files beside it; a helper that writes the test frame as a
  16-bit FITS file with the test's own writer; a FITS reader of the test's own for
  `input.fits`; the two worked answers as text; and the test's own separation and
  position-angle formulas. Every test here is a plain `#[tokio::test]`, in real time.
  Write failing tests for worked answer A through `read_answer` and `from_answer`
  (US1-S1) and for worked answer B through the stub adapter (US1-S2).
- [ ] T002 [US1] Add failing tests to `crates/finder/tests/console/imaging_astap.rs` for
  the exact argument lists (US1-S3), every exit code and unreadable answer (US1-S4), and
  the limits with a sleeping script: `/proc/self` seen first, the process gone, the
  work folder empty, and the longer limit of a hint with no place (US1-S5).
- [ ] T003 [US1] Add failing tests to `crates/finder/tests/console/imaging_astap.rs` for
  `measure` on a frame and on two regions of an `RGGB` frame, with the region outside
  the frame and the region with no whole cell (US1-S6), and for the saved captures read
  through `fixtures/finder/astap/captures.json` and replayed by the stub program, a
  case whose `ini` is `null` with no `reply.ini` (US1-S7). The captures are staged by
  the owner; if they are missing, stop and escalate.

### Implementation

- [ ] T004 [US1] Create `crates/finder/src/imaging/solver.rs` with `Region` and its
  `From<RoiView>`, `SolveFrame`, `HintSource`, `SolveHint` and its serialization,
  `Answer`, `Prepared`,
  `ImagingSolve` with `from_cd`, `from_answer`, `sky_at` and `pixel_of`, `StarMeasure`,
  `SolveError` with its reasons and messages, the two future types and the trait
  `ImagingSolver`, exactly as plan.md § The port gives them, and add `pub mod solver;`
  to `crates/finder/src/imaging/mod.rs`.
- [ ] T005 [US1] Create `crates/finder/src/imaging/astap.rs` with `AstapConfig`,
  `AstapSolver`, `solve_args`, `analyse_args`, `read_answer`, `read_error`,
  `read_measure` and `prepare`, exactly as plan.md § The ASTAP adapter gives them: the
  three limits and `with_limits`, the prepared copy's four steps, one call's six steps
  on the blocking pool against a real-time deadline (the whole-sky limit for a hint
  with no place), the exit-code table, and the work folder removed whatever happened.
  Add `pub mod astap;` to `crates/finder/src/imaging/mod.rs`.
- [ ] T006 [US1] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US1-S1, US1-S2, US1-S3, US1-S4, US1-S5, US1-S6 and
  US1-S7).

## Phase 2: User Story 2 — Solve the latest imaging frame on the console (Priority: P1)

### Tests first

- [ ] T007 [US2] Create `crates/finder/tests/console/imaging_solve.rs`, the module
  `imaging_solve` of `console`, and add `mod imaging_solve;` after `mod
  imaging_preview;` in `crates/finder/tests/console/main.rs`. Give it the solving rig of
  spec.md § User Scenarios, under `#[tokio::test(start_paused = true)]`, with helpers to
  wait on `GET /api/imaging`, to take a saved frame, to solve it, and to read the
  frame's truth from the simulator. Write failing tests for a solved frame, its sidecar
  and its unchanged file (US1's port is used, not its adapter) (US2-S1) and for the
  simulated adapter called directly, counting the seeded stars in the top half of the
  sensor with the test's own arithmetic before solving it (US2-S2).
- [ ] T008 [US2] Add failing tests to `crates/finder/tests/console/imaging_solve.rs` for
  the refusals, the link down among them, and `Imaging::solver()` (US2-S3), for a
  solver of the test's own that answers the five errors at once and then a solve, with
  `took_ms` and `file` on each failure (US2-S4), and for the view before any solve and
  without imaging (US2-S5). Assert nothing about `solve.hint` or `relation`. Create
  `crates/finder/tests/cli/imaging_solve.rs`, the module `imaging_solve` of `cli`, with
  `mod imaging_solve;` after `mod imaging_frames;` in `crates/finder/tests/cli/main.rs`,
  and in it a failing test for the built binary under `--simulate` and for the two flag
  refusals (US2-S6).

### Implementation

- [ ] T009 [US2] Create `crates/finder/src/imaging/simulated.rs` with `SimulatedSolver`
  and its five solve steps and its measure, exactly as plan.md § The simulated adapter
  gives them, and add `pub mod simulated;` to `crates/finder/src/imaging/mod.rs`.
- [ ] T010 [US2] Create `crates/finder/src/imaging/solving.rs` with `Ask`, `Ended` and
  `solve_frame`, which solves one frame and writes its solve sidecar with `took_ms`
  whatever the outcome; `SolveView`; `Imaging::set_solver` and `Imaging::solver`;
  `Imaging::solve` with its refusals in order and its task built on `solve_frame`; and
  `Imaging::solved`, passing `SolveHint::whole_sky()` (plan.md § Solving on the
  console). In `crates/finder/src/imaging/mod.rs` add `pub mod solving;`, the `solve`
  field of `ImagingView`, filled only while `ready`, and to the state the solver and
  the latest solve, and nothing this story does not read: `set_solver` takes the
  session and the live loop already and binds them as `_session` and `_live`. In
  `crates/finder/src/imaging/routes.rs` add `POST /api/imaging/solve`. In
  `crates/finder/src/api.rs` add `with_imaging_solver` (plan.md § The imaging state,
  § The route and § The builders).
- [ ] T011 [US2] In `crates/finder/src/main.rs` add `--astap`, `--astap-db` and
  `--astap-db-name` and give the console the ASTAP adapter, or under `--simulate` the
  simulated one (plan.md § The binary). Add the route's line to
  `crates/finder/src/lib.rs`, and the row and the section `## Solving imaging frames` to
  `crates/finder/README.md` (plan.md § `crates/finder/src/lib.rs` and
  `crates/finder/README.md`).
- [ ] T012 [US2] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US2-S1, US2-S2, US2-S3, US2-S4, US2-S5 and US2-S6).

## Phase 3: User Story 3 — Hints, the wide first search and the kept scale (Priority: P1)

### Tests first

- [ ] T013 [US3] Create `crates/finder/tests/console/imaging_hints.rs`, the module
  `imaging_hints` of `console`, and add `mod imaging_hints;` after `mod
  imaging_frames;` in `crates/finder/tests/console/main.rs`, with the solving rig. Write
  failing tests for the first solve's hint from the live pointing (US3-S1), the kept
  scale, `Imaging::scale_arcsec_per_px()` and the next solve's field height at binning
  1 and 2 (US3-S2), the focal length (US3-S3), the other three sources and a changed
  target (US3-S4), a second console over the same folder with the same camera and with
  another (US3-S5), the field height's sources taken in turn after a change of the
  simulated optics, on the rig and on a `Relation` of the test's own (US3-S7), and the
  hint's source taken one place down after three misses from a tapped aim point 4.98°
  off, on two rigs, on a `Relation` of the test's own and through `hint_for` with
  literals (US3-S8). Assert a radius only where spec.md does.
- [ ] T014 [US3] Create `crates/finder/tests/cli/imaging_hints.rs`, the module
  `imaging_hints` of `cli`, with `mod imaging_hints;` after `mod imaging_frames;` in
  `crates/finder/tests/cli/main.rs`, and in it a failing test for `--focal-length-mm`
  shown in the imaging view and refused at 0 (US3-S6).

### Implementation

- [ ] T015 [US3] Create `crates/finder/src/imaging/relation.rs` with `FORMAT`,
  `SCALE_MISSES`, `CameraId`, `Scale`, `Stored` (its `format`, `camera` and `scale`),
  `load`, `save`, `RelationView` with its `scale` and `focal_length_mm`, and the
  in-memory `Relation` with `new`, `set_focal_length_mm`, `describe`,
  `scale_arcsec_per_px`, `height_inputs`, `store_scale`, `miss`, `hint_skip` and
  `view` (plan.md § The relation: the scale, the field height's sources and the hint's
  turn). In
  `crates/finder/src/imaging/mod.rs` add `pub mod relation;`, the `relation` field of
  `ImagingView` filled from `Relation::view`, and to the state the session, the last
  solved solve's centre with the session's target of that moment, and one `Relation`
  made on `relation.json` in the imaging folder; and add the call of `describe` where
  the task keeps the camera's description (plan.md § The imaging state).
- [ ] T016 [US3] In `crates/finder/src/imaging/solving.rs` add the four constants,
  `HintInputs` with every field plan.md gives it, `skip` among them, `hint_for`, which
  takes the source `min(skip, n − 1)` places down among those that exist,
  `Imaging::hint`, `Imaging::set_focal_length_mm` and `Imaging::scale_arcsec_per_px`;
  build the hint from the session's latest pointing, the last solve while the target
  is unchanged, and the target, with `Relation::height_inputs` for the field height
  and `Relation::hint_skip` for `skip`, read under one lock; and in the task
  call `store_scale` after a solved solve and `miss` after one that ended `not_solved`
  or `timed_out`, and keep a solved solve's centre with the session's target of that
  moment; store the session `set_solver` is given (plan.md § Solving on the console,
  the hint). In
  `crates/finder/src/api.rs` add `with_focal_length_mm`, and in
  `crates/finder/src/main.rs` the flag `--focal-length-mm`. Add the hint's paragraph to
  `## Solving imaging frames` in `crates/finder/README.md`.
- [ ] T017 [US3] Run the `lint` and `test` gates with every existing test, US2's
  included, unchanged and green. Commit the green story (US3-S1, US3-S2, US3-S3, US3-S4,
  US3-S5, US3-S6, US3-S7 and US3-S8).

## Phase 4: User Story 4 — The aim point and imaging angle, measured from one moment (Priority: P1)

### Tests first

- [ ] T018 [US4] Create `crates/finder/tests/console/imaging_relation.rs`, the module
  `imaging_relation` of `console`, and add `mod imaging_relation;` after `mod
  imaging_preview;` in `crates/finder/tests/console/main.rs`, with the solving rig.
  Write failing tests for one aim reading against the simulator's true aim point and
  imaging angle (US4-S1); for `relation::read_aim` on finder fixes of the test's own
  making, each bullet of the scenario a case with its stated numbers, and for
  `FinderHistory`, kept by time and thinned (US4-S2); and for
  `Frame::up_east_of_north_at` (US4-S3).
- [ ] T019 [US4] Add failing tests to `crates/finder/tests/console/imaging_relation.rs`
  for the three cases that take no aim reading and for the hint of a solve asked for
  after the simulated telescope has swung away (US4-S4), for a tap on a star of a live
  preview, a reset and `session.json` with a measured aim point (US4-S5), for a
  tapped aim point on a console with no imaging (US4-S6), and for a solve the test
  made itself and offered to `note_solve`, whole and as a region (US4-S7).
- [ ] T020 [US4] Create `crates/finder/tests/motion/imaging_track.rs`, the module
  `imaging_track` of finder's `motion` test binary, and add `mod imaging_track;` after
  `mod goto;` in `crates/finder/tests/motion/main.rs`. Give it the tracking rig of
  spec.md US4-S8, written out from the solving rig with
  `with_live_solver(Arc::new(crate::support::goto::Stars))` in place of `with_live()`
  and the target M42, under `#[tokio::test(start_paused = true)]`, and a failing test
  for US4-S8: a sync, Track on, `tracking`, one solved frame with `relation.held`
  true and no session `aim`, the true direction sampled every 100 ms for 5 s within
  60″ of the first sample, and Track off applying the held aim point within 5 s.

### Implementation

- [ ] T021 [US4] In `crates/finder/src/sky.rs` add `Frame::up_east_of_north_at`. In
  `crates/finder/src/session.rs` add `AimView`, make `SessionView::aim` one, add
  `aim_measured` to the values and to `StoredSession`, written only when true, clear it
  in `set_aim`, and add `Session::set_measured_aim` (plan.md § The session and § The
  finder frame's up axis). Add the held aim point: `held_aim` in the values and never
  in `StoredSession`, `TrackRun::track_on`, the hold in `set_measured_aim` while Track
  is on, `Session::held_aim`, its drop in `set_aim`, and its application at the top of
  `record_live_pointing` with Track off (plan.md § The session, the held aim point).
  `Aim` and `set_aim`'s signature do not change, and nothing of Track,
  `start_tracking`, `stop_tracking`, `track_live_pointing` or
  `crates/finder/src/tracking.rs` is touched.
- [ ] T022 [US4] In `crates/finder/src/imaging/relation.rs` add the six constants,
  `FinderFix`, `FinderHistory`, `AimReading` with every field plan.md gives it,
  `Outcome`, `read_aim` with its eight steps, `Xy`, `LastOutcome`, `Last` and `Field`,
  the view's `aim`, `angle_deg`, `mirrored`, `readings`, `last`, `field` and `held`,
  and on `Relation` the history, the one aim reading held, `push_fix`, `fixes`, `take`
  and `view(in_force, held)` (plan.md § The relation, the history and the aim
  reading). `FinderHistory::push` keeps a fix only if it follows the newest by at
  least `keep_s / HISTORY_MAX as f64` and drops fixes for their age alone. In
  `crates/finder/src/imaging/solving.rs` start the history's task in `set_solver` from
  the live loop it is given; add `aim_in_force` and the private `take_aim_reading`
  with its four steps, call it as the task's step 2 after a solved solve, and add
  `Imaging::note_solve`, which calls only it (plan.md § The relation, the aim
  reading's use); take the hint's `pointing` from the history's fix nearest the
  exposure's middle, and pass `aim_measured` (plan.md § Solving on the console, the
  hint). In `crates/finder/src/imaging/mod.rs` add only the history task's handle to
  the state and `aim_in_force`'s two values to the call of `Relation::view`. Hold the
  imaging state's lock across no call into the session (plan.md § Plan traps 30).
- [ ] T023 [US4] Run the `lint` and `test` gates with every existing test, US2's and
  US3's and the `motion` binary's tracking tests included, unchanged and green. Commit
  the green story (US4-S1, US4-S2, US4-S3, US4-S4, US4-S5, US4-S6, US4-S7 and
  US4-S8).

## Phase 5: User Story 5 — Smooth the aim readings, set outliers aside and keep them (Priority: P2)

### Tests first

- [ ] T024 [US5] Create `crates/finder/tests/console/imaging_smoothing.rs`, the module
  `imaging_smoothing` of `console`, and add `mod imaging_smoothing;` after `mod
  imaging_relation;` in `crates/finder/tests/console/main.rs`, with the solving rig.
  Write failing tests for the window's worked values, the mirror sense, a finder frame
  of another size and `should_apply` (US5-S1); six solves, the narrow radius, and
  `hint_for` called with literals for a corner region, the floor and the ceiling
  (US5-S2); a turned camera and a moved aim point (US5-S3); and a second console with
  the session file, without it, and with a `relation.json` of another camera (US5-S4).

### Implementation

- [ ] T025 [US5] In `crates/finder/src/imaging/relation.rs` add the six constants,
  `Taken`, `Window` and `should_apply`, the `readings` and `set_aside` keys of `Stored`,
  and the view's `set_aside`; inside `Relation`, replace the one aim reading with a
  `Window`, so that `take` clears it when the session's aim point is not the measured
  one, gives the reading to `Window::take`, answers the median only when `should_apply`
  says so and saves `relation.json`, while `describe` and `store_scale` read and write
  the window's lists; `take` and `view(in_force, held)` keep their signatures, and
  `in_force` is still the held aim point while one is held (plan.md § The relation,
  the window). In `crates/finder/src/imaging/solving.rs` change only `hint_for`: the
  narrow radius, the sensor's diagonal; `take_aim_reading` and `note_solve` stay as
  US4 wrote them. Do not touch `crates/finder/src/imaging/mod.rs` or
  `crates/finder/src/session.rs`. Add `### The measured aim point` to
  `crates/finder/README.md`, the held aim point and `note_solve` included (plan.md
  § `crates/finder/src/lib.rs` and `crates/finder/README.md`).
- [ ] T026 [US5] Run the `lint` and `test` gates with every existing test, US3's and
  US4's included, unchanged and green. Commit the green story (US5-S1, US5-S2, US5-S3
  and US5-S4).

## Phase 6: User Story 6 — The solve on the Imaging page, the field on the chart, the source on Setup (Priority: P2)

### Tests first

- [ ] T027 [US6] Create `crates/finder/web/src/components/ImagingSolveBlock.test.tsx`
  with failing tests for US6-S1, US6-S2, US6-S3 and US6-S4, stubbing `fetch` as the
  existing page tests do.
- [ ] T028 [US6] Create `crates/finder/web/src/components/ImagingField.test.tsx` with
  failing tests for US6-S5 and US6-S6, and
  `crates/finder/web/src/components/AimSource.test.tsx` with a failing test for US6-S7,
  feeding `SetupPage` through `useSession` as
  `crates/finder/web/src/components/ScopeNodeSection.test.tsx` does.

### Implementation

- [ ] T029 [US6] In `crates/finder/web/src/api.ts` add `ImagingSolveHint`,
  `ImagingSolve`, `ImagingRelation` (its `held` included), `solve` and `relation` on
  `ImagingView`, `source`
  on the session's `aim`, `imaging` on `ObservingSession` and `postImagingSolve`; copy
  `imaging` in `request()` of `crates/finder/web/src/useSession.ts`. Create
  `crates/finder/web/src/components/ImagingSolveBlock.tsx` with the words of plan.md
  § The page and place it in `crates/finder/web/src/components/ImagingPage.tsx` after
  the Imaging frame section.
- [ ] T030 [US6] Create `crates/finder/web/src/components/ImagingField.tsx` with
  `imagingFieldCorners` and `ImagingField`, draw it in
  `crates/finder/web/src/components/Chart.tsx` after the target mark under the
  conditions of plan.md § The page, and add the source line to
  `crates/finder/web/src/components/AimTap.tsx`.
- [ ] T031 [US6] Make the three edits to `DESIGN.md` and add the Imaging field component
  to `.impeccable/design.json` (plan.md § `DESIGN.md` and `.impeccable/design.json`).
  Run the `lint` and `test` gates with every existing Vitest and Playwright test
  unchanged and green, commit the rebuilt `crates/finder/web/dist`, and commit the green
  story (US6-S1, US6-S2, US6-S3, US6-S4, US6-S5, US6-S6 and US6-S7).

## Phase 7: User Story 7 — One MCP tool (Priority: P3)

### Tests first

- [ ] T032 [US7] Create `crates/finder/tests/mcp/imaging_solve.rs`, the module
  `imaging_solve` of finder's `mcp` test binary, and add `mod imaging_solve;` after `mod
  imaging;` in `crates/finder/tests/mcp/main.rs`, with the solving rig built with MCP
  on. Write failing tests for US7-S1, US7-S2 (its solver built `with_delay` of 5 s, the
  simulator's clock read before and after the call), US7-S3 and US7-S4.

### Implementation

- [ ] T033 [US7] Add `solve_imaging_frame` to `crates/finder/src/mcp/tools.rs` (plan.md
  § The MCP tool), and its line to the tool list in `## MCP clients` of
  `crates/finder/README.md`.
- [ ] T034 [US7] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US7-S1, US7-S2, US7-S3 and US7-S4).

## Execution Order

```text
US1 ──► US2 ──► US3 ──► US4 ──► US5 ──► US6 ──► US7
```

The Work Graph has two more edges than this line shows, US3 to US5 and US5 to US7, for
the README section each pair writes in turn; they do not change the order.

- **US1** comes first: it defines the port every later story calls, and the adapter the
  binary is given.
- **US2** puts the port behind a route, with the simulated adapter every later Rust test
  uses, and creates `solving.rs`, the README section and the view's `solve`.
- **US3** gives US2's solve its hint, with its turn after three misses, and creates
  the `Relation` and `relation.json` with the scale.
- **US4** takes its aim reading from US2's solve and from `note_solve`, reads US3's
  scale for the field, takes the hint's pointing from its own history, and changes the
  session, holding a measured aim point while Track is on; it is the one story with a
  test in `motion`.
- **US5** turns US4's one aim reading into a window, inside `Relation`, and narrows
  US3's radius.
- **US6** touches only `crates/finder/web/`, `DESIGN.md` and `.impeccable/design.json`,
  and shows what US2 to US5 put in the imaging view and the session.
- **US7** comes last: it calls US2's action, and adds its line to the README section the
  earlier stories write.
- **`[P]`** marks nothing here: every task shares a file with another in its story, and
  the stories share `mod.rs`, `solving.rs`, `relation.rs` and the README.
