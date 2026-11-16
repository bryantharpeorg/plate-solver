---
description: "Task list for 037-imaging-run: The Imaging Run"
---

# Tasks: The Imaging Run

**Prerequisites** (plan.md § Prerequisites):
- `014-goto`, `015-tracking`, `016-mcp` and `017-console-stop-confirm` have landed on
  `main`: met;
- `030-imaging-camera-link`, `031-imaging-frames-on-console`, `032-imaging-solve-and-aim-point`,
  `033-focus-reading-and-aid`, `034-dark-and-flat-frames`, `035-live-stack`,
  `036-tracking-from-subs` and `038-track-follows-a-push` have landed, and this trio has
  been re-anchored to them, with
  every name of plan.md § What 030 to 036 offer re-read on `main`: not met (plan.md
  § Prerequisites 1 and 2);
- 031 holds the lease of the imaging camera the plan document's amendment 1 names, as
  its US6, and 033 to 035 take it: met in the drafts, and re-read when 031 lands
  (plan.md § Prerequisites 3);
- the first twenty-five owner decisions in spec.md are settled and its owner approvals
  given, and 032's owner decision 25, the seam between 032 and 015 that decision 26
  cites, is settled in 032's trio: not met (plan.md § Prerequisites 4);
- 032's draft now holds a measured aim point back while Track is on (032's FR-036),
  the answer that by spec.md owner decision 26 changes FR-022's test and moves
  FR-025's wait after the re-centre's Track off, and this trio is rewritten for it, or
  the owner answers 032's decision 25 otherwise: not met, in this worktree's drafts
  (plan.md § Prerequisites 4);
- every story has been probe-built: not met (plan.md § Prerequisites 5);
- no operator-staged file is needed.

**Rules for every story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml`,
  `CODEOWNERS`, `ergane.yaml`, `.specify/` or `crates/finder/web/playwright.config.ts`.
  If an existing test fails, stop and escalate with its exact output. Do not edit it.
  The one exception is the `mod` line of each new finder test module, added in
  alphabetical order to the binary's `main.rs` your story's tasks name, which holds no
  test (spec.md owner approval 8). Change nothing else there. A test file an earlier
  story of this spec created is an existing test: add your tests in the new file your
  tasks name, and leave every earlier scenario passing (plan.md § Plan traps 24).
- A new finder test is a module file in the binary's folder plan.md § Technical Context
  names, never a file at the top level of `crates/finder/tests` or in
  `crates/finder/tests/support`, either of which fails the `layout` test. It takes its
  scratch folder from `crate::support::Scratch`, and holds only the setup its own tests
  use, since an unused helper fails `clippy -D warnings`. A rig test builds its own rig
  in its own module.
- Write only the files your story's tasks name; they are its `writes` in plan.md
  § Refinement Record. A story that needs another file escalates.
- Tests type in the words, reasons, states and call lists spec.md gives; none builds an
  expected message by calling `Ending::message`, and none reads `run.json` or
  `stack.json` through the console's code (plan.md § Plan traps 6).
- Use the stand-in parts of `finder::imaging::run::script` for every part your story
  does not make real. Add no method to a part's trait and no field to `Steps`, do not
  change `Runner::new`, `start`, `stop`, `resume` or `view`, and after US1 do not write
  `script.rs` (plan.md § Shapes that existing tests pin).
- The run asks the mount part and nothing else to move the mount. Import nothing of
  `crate::goto`, `crate::tracking`, `crate::ephemeris` or `scope_link` in
  `crates/finder/src/imaging/run.rs` or under `crates/finder/src/imaging/run/`, and of
  `crate::model` only `separation`, in `console.rs`. Write no identifier, import or
  arithmetic there for an axis, an altitude, an azimuth, a rate or axis counts, name no
  state of Track but `armed` and `off`, and write no focal length, field size or scale
  as a number. The rule is about code: the messages plan.md § The run's words gives and
  the doc comments it gives are written as they stand (plan.md § Plan traps 18).
- While the runner's own lock is held, call no part, read no imaging view and await
  nothing: copy what a step needs out of the lock, let it go, call the part, and lock
  again to record the answer (plan.md § Plan traps 23).
- Add no lock on the imaging camera: the run takes 031's lease through the camera part,
  gives it up before every focus check and before the stack is started, takes it again
  after the focus gate, and gives it back at every ending and after a refused Track
  (plan.md § Plan traps 20).
- Wait a fixed time for no exposure: the only durations are `POLL`,
  `FOCUS_PAUSE_LIMIT`, the request's `duration_s`, and the 5 s that `FRESH_POLLS` polls
  bound a re-centre's wait for the finder camera (plan.md § Plan traps 21).
- Keep the story's diff under 64 KiB, measured as plan.md § Plan traps 3 says.
- Every code citation is `path:line` as read at 49bb210; a file 030 to 036 create is
  named without a line. If a cited line has moved, re-read it rather than guess.
- Add no dependency and change no `Cargo.toml`. Write no `unsafe`. Change nothing under
  `crates/simulator`, `crates/scope-node` or `crates/scope-link`.
- Do not edit `specs/037-imaging-run/`.
- Run no tool the gates don't run: no `cargo deny`, `cargo machete`, Rust 1.96 or
  `ergane`.

## Phase 1: User Story 1 — The run's states, over injected steps (Priority: P1)

### Tests first

- [ ] T001 [US1] Create `crates/finder/tests/console/imaging_run.rs`, the module
  `imaging_run` of finder's `console` test binary, and add `mod imaging_run;` in its
  alphabetical place in `crates/finder/tests/console/main.rs`. Give it one helper that
  builds a scripted run as spec.md § User Scenarios defines it (a
  `Runner::new(script.steps(), clock)` over a `FixedClock` at 1768528800, the helper
  keeping no `Steps` of its own) and one that settles for 1 s, under
  `#[tokio::test(start_paused = true)]`. Write failing tests for the straight path with
  its states, messages, view and calls, and for `label` and `run_id` (US1-S1), the
  start's refusals, the lease asked last before Track and given back after a refused
  Track, and a lease that cannot be taken back after the focus check (US1-S2), and a
  mount that is not steady (US1-S3).
- [ ] T002 [US1] Add failing tests to that file for a stop in each of five states, with
  the aborted take, the stopped focus check and the lease given back last (US1-S4), the
  seven endings of the mount's motion (US1-S5), and a runner dropped while its run stacks
  (US1-S6).

### Implementation

- [ ] T003 [US1] Create `crates/finder/src/imaging/run/script.rs` with `Part`, `Step`,
  `STACK_FOLDER` and `Script`, whole: its defaults, every control, the holds and the
  call words exactly as plan.md § The script gives them. A call is listed when it is
  made, before any hold. `set_stack` sets the counts and the tracker's pause and never
  whether the stack runs; only `stack.start`, `stack.stop` and `end_stack` change that.
- [ ] T004 [US1] Create `crates/finder/src/imaging/run.rs` with the constants, `STATES`,
  `ENDINGS`, `By`, `Ending`, `Failure`, `Verdict`, `Unit`, the five traits, `Steps`,
  `RunRequest`, the view's types, `label` (one line that calls 035's
  `crate::imaging::stack::service::label`; do not write the rule again), `run_id` and
  `Runner` with its `Drop`, exactly as plan.md § The machine gives them, and add
  `pub mod run;` to
  `crates/finder/src/imaging/mod.rs`. Write every message from plan.md § The run's
  words. Write `start`'s seven steps in their order, the three helpers `guarded`,
  `steady` and `frame`, the task's six steps as "The task, in US1" gives them, with the
  lease given up before the focus check and before the stack and taken again between
  them, and the ending in its order. In this story a failed take, solve or stack start
  ends the run at once, every focus verdict goes on, the counts stay at zero, the
  stacking loop ends only on a stop or the mount's ending, and `resume` always refuses.
  Keep the latest run under one std `Mutex` in an inner `Arc` the task shares; the task
  holds that `Arc` and never the `Runner`, and `Drop` aborts the task. Never call a
  part, read the imaging view or await while that lock is held (plan.md § Plan traps
  23).
- [ ] T005 [US1] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US1-S1, US1-S2, US1-S3, US1-S4, US1-S5 and US1-S6).

## Phase 2: User Story 2 — Failed steps, counts and warnings, over injected steps (Priority: P1)

### Tests first

- [ ] T006 [US2] Create `crates/finder/tests/console/imaging_run_steps.rs`, the module
  `imaging_run_steps` of `console`, with `mod imaging_run_steps;` in its alphabetical
  place in `crates/finder/tests/console/main.rs`, and its own helpers for a scripted
  run and a settle. Write failing tests for failed takes (US2-S1) and failed solves
  (US2-S2).
- [ ] T007 [US2] Add failing tests to that file for the stack's counts, the tracker's
  pause and a slightly soft focus verdict in both units (US2-S3), and for a stack that
  cannot start or does not end cleanly (US2-S4).

### Implementation

- [ ] T008 [US2] In `crates/finder/src/imaging/run.rs`, add `FRAME_TRIES` and
  `SOLVE_TRIES` and replace what US1 did with a failed take, solve and `stack.start` by
  the table of plan.md § Failed steps and counts in the machine: counted takes and
  solves, the two kinds of failed focus check that end the run, a failed `stack.start`
  sorted by its kind, the warning for a `stack.stop` that fails at an ending, the
  warning of `SlightlySoft`, and step 6 copying `stack.progress()` into `subs` and
  showing `paused_by_tracker`. Read neither `running` nor `failed` yet.
- [ ] T009 [US2] Run the `lint` and `test` gates with every existing test unchanged and
  green, US1's included. Commit the green story (US2-S1, US2-S2, US2-S3 and US2-S4).

## Phase 3: User Story 3 — Start, stop and read a run over HTTP, and its record (Priority: P1)

### Tests first

- [ ] T010 [US3] Create `crates/finder/tests/console/imaging_run_routes.rs`, the module
  `imaging_run_routes` of `console`, with `mod imaging_run_routes;` in its alphabetical
  place in `crates/finder/tests/console/main.rs`, and the scripted console of spec.md
  § User Scenarios: 031's console rig with `with_imaging_run_steps(script.steps())`
  added last, R under the test's scratch folder, and helpers to send a request with and
  without an `Origin`, to wait on `GET /api/imaging`, and to parse `run.json` with
  `serde_json`. Write failing tests for a start with its view, session and `run.json`
  in the stack's folder (US3-S1), the refusals (US3-S2) and `DELETE` (US3-S3).
- [ ] T011 [US3] Add failing tests to that file for `continue` outside a pause (US3-S4),
  the run under `no_link` (US3-S5), two runs in one second that end before stacking and
  a `runs` folder that cannot be written (US3-S6), and what `run.json` must not hold
  (US3-S7).

### Implementation

- [ ] T012 [US3] Create `crates/finder/src/imaging/run/store.rs` with `FORMAT`, `Event`
  and `Store`, and add `pub mod store;`, `Runner::with_folder`, the list of events and
  the writes of `run.json` to `crates/finder/src/imaging/run.rs`, as plan.md § The
  record gives them: nothing written before the run stacks, the stack's folder adopted
  when the stack has started, one write at each later change of state, a failed write
  ending the run `save_failed`, a folder made at the ending for a run that never
  stacked, a failure there a warning, every write synchronous.
- [ ] T013 [US3] In `crates/finder/src/imaging/mod.rs`, add `run` to `ImagingView` and
  `attach_run` to `Imaging`, setting `run` wherever the view is built, in every state
  (plan.md § `crates/finder/src/imaging/mod.rs`). In `crates/finder/src/api.rs`, add the
  `imaging_runner` field, `with_imaging_run_steps` and `imaging_runner` (plan.md § The
  console).
- [ ] T014 [US3] In `crates/finder/src/imaging/routes.rs`, add `start_run`, `stop_run`
  and `continue_run` with their order of checks, the start wrapped in
  `api::start_motion` with `page` and `imaging_run` (plan.md § The routes). Add the
  three route lines to `crates/finder/src/lib.rs`, worded as that section gives them.
  The README is US11's.
- [ ] T015 [US3] Run the `lint` and `test` gates with every existing test unchanged and
  green, US1's and US2's included. Commit the green story (US3-S1, US3-S2, US3-S3,
  US3-S4, US3-S5, US3-S6 and US3-S7).

## Phase 4: User Story 4 — Pointing through Track, on the console's own mount, camera and solver parts (Priority: P1)

### Tests first

- [ ] T016 [US4] Create `crates/finder/tests/motion/imaging_run.rs`, the module
  `imaging_run` of `motion`, with `mod imaging_run;` in its alphabetical place in
  `crates/finder/tests/motion/main.rs`, and the centred rig of spec.md US4: 032's
  solving rig built with `with_live_solver(Arc::new(crate::support::goto::Stars))` in
  place of `with_live()`, the target M42, a first sync awaited, the session's aim point
  set to the true one, the parts `console::mount`, `console::camera` and
  `console::solver` with the script's focus and stack, and the test's own handle of the
  `SimMount`. Add helpers for the true offset and for "at rest for 10 s" as spec.md
  § User Scenarios defines them. Write failing tests for a run that needs no re-centre
  (US4-S1) and for the refusals of a start with the lease given back (US4-S2).
- [ ] T017 [US4] Add failing tests to that file for Stop while stacking (US4-S3), the
  five endings of the mount's motion and the imaging frame refused during a run with
  `the imaging camera is held by the imaging run` (US4-S4), and the mount part alone:
  point, point again, and point again after a Stop
  (US4-S5), each against the simulator's truth, the session or 031's own route as its
  scenario says.

### Implementation

- [ ] T018 [US4] Create `crates/finder/src/imaging/run/console.rs` with `CENTRE_SHARE`,
  `CENTRE_FLOOR_ARCSEC`, `mount`, `camera` and `solver`, and add `pub mod console;` to
  `crates/finder/src/imaging/run.rs`. Write the mount part's `target`, `point`, the
  table of `state` with its `repointing` flag, and `repoint`'s steps 1 and 3 to 6:
  refuse when the stop generation has changed, Track off, Track on under
  `start_motion`, the generation compared before Track's answer is read, Stop asked
  again when it changed, and only then Track's error as a failure of the mount (plan.md
  § The console's mount part). Step 2, the wait, is US5's. Write the camera part over
  031 US6's lease, with `HOLDER`, an `Arc` of the lease kept under a std `Mutex`, the
  lease taken in `hold` before the look for an imaging frame in flight and before the
  region-of-interest check, and the holder's take with the purpose `run` at the
  camera's settings as they stand, setting nothing and asking nothing of 036's
  steering; and the solver part, with the session's aim point read before and after
  the solve for `aim_moved` (plan.md § The console's camera and solver parts).
- [ ] T019 [US4] Run the `lint` and `test` gates with every existing test unchanged and
  green, US1's to US3's included. Commit the green story (US4-S1, US4-S2, US4-S3,
  US4-S4 and US4-S5).

## Phase 5: User Story 5 — Centring on the sensor (Priority: P1)

### Tests first

- [ ] T020 [US5] Create `crates/finder/tests/console/imaging_run_centring.rs`, the
  module `imaging_run_centring` of `console`, with its `mod` line in
  `crates/finder/tests/console/main.rs`, and failing scripted tests for one re-centre
  (US5-S1), a target that is never centred (US5-S2), and a solve that did not move the
  aim point, a failed `repoint`, a mount that is not steady after one, and a stop of
  the run while `repoint` is held (US5-S3).
- [ ] T021 [US5] Create `crates/finder/tests/motion/imaging_run_centring.rs`, the module
  `imaging_run_centring` of `motion`, with its `mod` line in
  `crates/finder/tests/motion/main.rs`, holding its own build of the run rig with no
  aim point. Write failing tests for centring from no aim point with the console's own
  mount, camera and solver parts (US5-S4), and for Stop while centring with the
  script's solver answering 400″ every time (US5-S5), against the simulator's truth and
  the simulator's mount.

### Implementation

- [ ] T022 [US5] In `crates/finder/src/imaging/run.rs`, add `CENTRE_TRIES` and make step
  4 of the task the centring loop of plan.md § Centring in the machine, with `repoint`
  awaited unguarded and only after a solve whose `aim_moved` is true.
- [ ] T023 [US5] In `crates/finder/src/imaging/run/console.rs`, add `FRESH_POLLS` and
  step 2 of `repoint`: the wait, with Track on, for a live pointing newer than the one
  found, at most `FRESH_POLLS` polls, going on when none has come and refusing as
  stopped when the stop generation changes meanwhile (plan.md § The console's mount
  part).
- [ ] T024 [US5] Run the `lint` and `test` gates with every existing test unchanged and
  green, US1's to US4's included. Commit the green story (US5-S1, US5-S2, US5-S3,
  US5-S4 and US5-S5).

## Phase 6: User Story 6 — The focus gate and its pause (Priority: P1)

### Tests first

- [ ] T025 [US6] Create `crates/finder/tests/console/imaging_run_focus.rs`, the module
  `imaging_run_focus` of `console`, with its `mod` line in
  `crates/finder/tests/console/main.rs`, and failing scripted tests for the pause and
  its five messages (US6-S1), checking again (US6-S2), going on anyway (US6-S3), the 900
  s limit (US6-S4), and a stop, a mount ending, an aid that cannot open and a resume
  outside a pause (US6-S5).
- [ ] T026 [US6] Add failing tests to that file for the route on the scripted console
  (US6-S6), and, on 033's focus rig with the console's own camera and focus parts and
  the script's mount, solver and stack, for a run at the simulator's focus error 0.0, a
  defocused one found in at most ten steps of 1.0, a refocused one, a start while the
  focus aid runs, refused with `the imaging camera is held by the focus aid`, and a
  camera with no focus record (US6-S7).

### Implementation

- [ ] T027 [US6] In `crates/finder/src/imaging/run.rs`, add `FOCUS_PAUSE_LIMIT`, replace
  the going-on of step 3 for a verdict that pauses and for a check that did not finish
  with the pause of plan.md § The focus gate in the machine, close the aid on every way
  out of a pause whether or not it opened, take the lease again on every way on to the
  solve, and make `Runner::resume` hand `anyway` to the waiting task.
- [ ] T028 [US6] In `crates/finder/src/imaging/run/console.rs`, add `focus` over 033's
  `focus_start`, `focus_ended` and `focus_stop`, with the table that turns 033's view
  into a `FocusCheck`, as plan.md § The console's focus part gives it.
- [ ] T029 [US6] Run the `lint` and `test` gates with every existing test unchanged and
  green, US1's to US5's included. Commit the green story (US6-S1, US6-S2, US6-S3,
  US6-S4, US6-S5, US6-S6 and US6-S7).

## Phase 7: User Story 7 — Stacking to a limit, and the endings a stack can cause (Priority: P1)

### Tests first

- [ ] T030 [US7] Create `crates/finder/tests/console/imaging_run_limits.rs`, the module
  `imaging_run_limits` of `console`, with its `mod` line in
  `crates/finder/tests/console/main.rs`, and failing scripted tests for the count limit
  with subs kept and discarded (US7-S1), the time limit with and without the tracker's
  pause (US7-S2), ten drops in a row (US7-S3), and the stack's own failures and a stack
  that ends by itself, each through `end_stack` (US7-S4).

### Implementation

- [ ] T031 [US7] In `crates/finder/src/imaging/run.rs`, add `DROP_RUN` and the five
  checks of plan.md § Limits and the stack's endings in the machine to the stacking
  loop, in their order, the time limit on tokio's clock from the instant `stack.start`
  answered.
- [ ] T032 [US7] Run the `lint` and `test` gates with every existing test unchanged and
  green, US1's to US6's included. Commit the green story (US7-S1, US7-S2, US7-S3 and
  US7-S4).

## Phase 8: User Story 8 — The whole run on the console's own parts, and in the binary (Priority: P1)

### Tests first

- [ ] T033 [US8] Create `crates/finder/tests/motion/imaging_run_stack.rs`, the module
  `imaging_run_stack` of `motion`, with its `mod` line in
  `crates/finder/tests/motion/main.rs`, holding its own build of the run rig with
  `with_imaging_run()`, and failing tests for a whole run to three subs (US8-S1) and
  for `keep_subs` false and a `DELETE` while stacking (US8-S2). Create
  `crates/finder/tests/cli/imaging_run.rs`, the module `imaging_run` of `cli`, with
  `mod imaging_run;` in its alphabetical place in `crates/finder/tests/cli/main.rs`,
  and a failing test for the built binary under `--simulate` (US8-S3).

### Implementation

- [ ] T034 [US8] In `crates/finder/src/imaging/run/console.rs`, add `stack` over 035 and
  036, as plan.md § The console's stack part and the wiring gives it: the set point
  told to the steering, the stack started for the target's label, the counts with only
  the reasons above 0, the wait for the tracker, the stack's own ending sorted into a
  `Failure`, and with `keep_subs` false the removal of the subs `stack.json` lists. In
  `crates/finder/src/api.rs`, add `with_imaging_run`, cloning the state for the parts
  before the runner is kept. In `crates/finder/src/main.rs`, call it after the imaging
  solver is set, on the `--simulate` path too.
- [ ] T035 [US8] Run the `lint` and `test` gates with every existing test unchanged and
  green, US1's to US7's included. Commit the green story (US8-S1, US8-S2 and US8-S3).

## Phase 9: User Story 9 — The run on the Imaging page (Priority: P2)

### Tests first

- [ ] T036 [US9] Create `crates/finder/web/src/components/ImagingRunBlock.test.tsx` with
  failing tests for US9-S1, US9-S2, US9-S3, US9-S4, US9-S5 and US9-S6, stubbing `fetch`
  as 031's Imaging page tests do, and for US9-S7, reading `DESIGN.md` and
  `.impeccable/design.json` with `readWebFile` from `src/test/files.ts`.

### Implementation

- [ ] T037 [US9] Add `ImagingRun`, `ImagingRunRequest`, `postImagingRun`,
  `deleteImagingRun` and `postImagingRunContinue` to `crates/finder/web/src/api.ts`,
  and `run` to `ImagingView`. Create
  `crates/finder/web/src/components/ImagingRunBlock.tsx` with the words of plan.md
  § The page, keeping its two fields and its checkbox in its own state, and render it
  in `crates/finder/web/src/components/ImagingPage.tsx` right after the title block.
- [ ] T038 [US9] Add the Imaging run entry to `DESIGN.md` and the Imaging run component
  to `.impeccable/design.json` (plan.md § `DESIGN.md` and `.impeccable/design.json`).
  Run the `lint` and `test` gates with every existing Vitest and Playwright test
  unchanged and green, commit the rebuilt `crates/finder/web/dist`, and commit the
  green story (US9-S1, US9-S2, US9-S3, US9-S4, US9-S5, US9-S6 and US9-S7).

## Phase 10: User Story 10 — Four MCP tools (Priority: P2)

### Tests first

- [ ] T039 [US10] Create `crates/finder/tests/mcp/imaging_run.rs`, the module
  `imaging_run` of finder's `mcp` test binary, with `mod imaging_run;` in its
  alphabetical place in `crates/finder/tests/mcp/main.rs`, serving the scripted console
  through the door as `crates/finder/tests/support/mcp.rs` serves a state, with a call
  log the test reads. Write failing tests for the list (US10-S1), a start and a read
  (US10-S2), the errors (US10-S3), the rate limit (US10-S4), stop and continue
  (US10-S5), a start during a direct axis move on `crate::support::mcp::motion::Rig`
  (US10-S6), and the token (US10-S7). Leave 1 s of virtual time between two starts but
  in US10-S4.

### Implementation

- [ ] T040 [US10] In `crates/finder/src/mcp/tools.rs`, add `ImagingRunArguments`,
  `ImagingRunContinueArguments` and the four tools with the descriptions, order of
  checks, parsing and answers of plan.md § The MCP tools. In
  `crates/finder/src/api.rs`, add `start_imaging_run` to the one `matches!` of
  `start_motion` plan.md § The console names. The README is US11's.
- [ ] T041 [US10] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US10-S1, US10-S2, US10-S3, US10-S4, US10-S5, US10-S6
  and US10-S7).

## Phase 11: User Story 11 — The README and the field checklist (Priority: P3)

### Tests first

- [ ] T042 [US11] Create `crates/finder/tests/console/imaging_run_docs.rs`, the module
  `imaging_run_docs` of `console`, with its `mod` line in
  `crates/finder/tests/console/main.rs`, reading `README.md` from `CARGO_MANIFEST_DIR`.
  Write failing tests for the section's words (US11-S1), the checklist's fourteen steps
  and the word `corner` in its step 8 (US11-S2), and the three rows of the API table
  and the four tools in MCP clients (US11-S3).

### Implementation

- [ ] T043 [US11] In `crates/finder/README.md`, add the three rows to the `## API`
  table, the four tools to `## MCP clients`, and the sections `## An imaging run` and
  `### Field checklist: the first imaging run`, with the rows, the content and the
  fourteen steps of plan.md § The README: rows, tools, the section and the field
  checklist, every state word and ending reason between backticks, and the known limit
  that the run's own imaging frames are not capped as subs are, in the prose and in
  step 8.
- [ ] T044 [US11] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US11-S1, US11-S2 and US11-S3).

## Execution Order

```text
US1 ──► US2 ──► US3 ──► US4 ──► US5 ──► US6 ──► US7 ──► US8 ──► US9 ──► US10 ──► US11
```

- **US1** comes first: every other story hangs a route, a real part, a page or a tool
  on its machine and uses its script.
- **US2** completes the machine for a step that fails and for the stack's counts, before
  anything is served.
- **US3** serves the machine and gives it its record; US4 to US6 drive their real parts
  through its routes.
- **US4** writes the console's own mount, camera and solver parts and proves them on the
  simulated mount with runs that need no re-centre.
- **US5** adds the centring loop to the machine and the wait a re-centre needs to the
  mount part, and proves a re-centre end to end.
- **US6** and **US7** each add a group of states or endings to
  `crates/finder/src/imaging/run.rs`, in the order a run meets them, so they cannot run
  together.
- **US8** adds the stack part, assembles the five parts and wires the binary.
- **US9** touches only `crates/finder/web/`, `DESIGN.md` and `.impeccable/design.json`,
  and reads every state and ending the earlier stories make.
- **US10** writes `crates/finder/src/api.rs` after US8.
- **US11** comes last: it writes the README for the routes and tools as they were
  built, and its test reads it.
- **`[P]`** marks nothing here: the stories form one chain.
