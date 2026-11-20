---
description: "Task list for 037-imaging-run: The Imaging Run"
---

# Tasks: The Imaging Run

**Prerequisites** (plan.md § Prerequisites):
- `014-goto`, `015-tracking`, `016-mcp` and `017-console-stop-confirm` have landed on
  `main`: met;
- `030-imaging-camera-link`, `031-imaging-frames-on-console`,
  `032-imaging-solve-and-aim-point`, `033-focus-reading-and-aid`,
  `034-dark-and-flat-frames` and `038-track-follows-a-push` have landed: met by
  2026-11-19;
- `035-live-stack` and `036-tracking-from-subs` have landed, and this trio has been
  re-anchored to all eight, with every name of plan.md § What 030 to 036 offer re-read
  on `main`: not met (plan.md § Prerequisites 1 and 2);
- 031 holds the lease of the imaging camera the plan document's amendment 1 names, as
  its US9, and 033 to 035 take it: met in the drafts, and re-read when 031 lands
  (plan.md § Prerequisites 3);
- the first twenty-five owner decisions in spec.md are settled and its owner approvals
  given: not met; the owner's seventeen answers of 2026-11-19 are folded in, each
  with a dated note (plan.md § Prerequisites 4);
- 032's owner decision 25, the seam between 032 and 015 that decision 26 cites, is
  settled in 032's trio, and this trio is written for its answer: met on 2026-11-17.
  The owner chose the hold (032's FR-036), so FR-022's test counts a held aim point as
  moved and FR-025's wait comes after the re-centre's Track off (plan.md
  § Prerequisites 4);
- every story has been probe-built: met on 2026-11-19 by probe round 1, for the trio as
  it then stood (plan.md § Prerequisites 5);
- a second probe round has built the stories that the fold of 2026-11-19 changed, US12
  and US1 as the owner's split of that day left them among them: not
  met (spec.md § Clarifications, the owner's answer 17);
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
  expected message by calling `Ending::message` (US12's test calls it, and compares what
  it answers with a sentence typed in), and none reads `run.json` or
  `stack.json` through the console's code (plan.md § Plan traps 6).
- Use the stand-in parts of `finder::imaging::run::script` for every part your story
  does not make real. Add no method to a part's trait and no field to `Steps`, do not
  change `Runner::new`, `start`, `stop`, `resume` or `view`, and after US12 do not write
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
  gives it up before the first focus check and before the stack is started, leaves
  it given up through a focus pause and every check that is repeated, takes it again
  after the focus gate, and gives it back at every ending and after a refused Track
  (plan.md § Plan traps 20).
- Wait a fixed time for no exposure: the only durations are `POLL`,
  `FOCUS_PAUSE_LIMIT`, the request's `duration_s`, the 5 s that `FRESH_POLLS` polls
  bound a re-centre's wait for the finder camera, `RETRY_WAIT` between two failed
  takes, `NO_SUB_LIMIT` of stacking with a failed sub and none stacked, `RECORD_EVERY`
  between two refreshes of the record and between two listed changes of `stacking`
  and `paused_by_tracker`, and, in `console.rs`, `PUSH_GRACE` and the
  look-back of the mount part's `steady`, which is 032's `FIT_REACH_S` or the exposure
  setting when that is longer. None of them waits for an exposure to finish (plan.md
  § Plan traps 21).
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
  its states, messages, view and calls, `stacking_since` and `limit_at` once the stack has
  started, a held take listed before it waits, and the keys of the view written as
  JSON at the first settle and at the sixth (US1-S1).
- [ ] T002 [US1] Add failing tests to that file for a lease that cannot be taken back
  after the focus check, read after a settle, with the five keys of its `ended`
  written as JSON, and for a focus check that fails with
  `Failure::Other`, reading only `focus` and `warnings` (US1-S2). Write no test here of
  a refused start, a mount that is not steady, a stop, the mount's endings or a dropped
  runner: they are this story's code (T004) and US2's tests (US2-S6 to US2-S10; T007).

### Implementation

- [ ] T003 [US1] In `crates/finder/src/imaging/run.rs`, which US12 created, add after
  what is there, and change none of it: the view's types `SubCounts`, `FocusNote`,
  `CentringNote`, `EndedNote` and `RunView`, with the `skip_serializing_if` attributes
  as the code block of plan.md § The machine has them, and the three tables of plan.md
  § The run's words that are made from a run's own state, whole: each state's message,
  the focus sentences and the warnings, the two for a target stacked off the centre each
  in its two forms, chosen by the offset against 036's
  `crate::imaging::steering::FAR_ARCSEC`, which is read and never copied. `EndedNote`
  holds `saved` and `track_on`, and
  the view `stacking_since`, `limit_at`, set with it, and `pause_ends_at`. A warning
  already in the list is not added again. Write neither
  `crates/finder/src/imaging/run/script.rs` nor `crates/finder/src/imaging/mod.rs`: they
  are US12's (T047, T048).
- [ ] T004 [US1] In the same file, add `Runner` with its `Drop`, exactly as plan.md
  § The machine gives them, on US12's traits, `Steps`, `Ending`, `Failure` and `run_id`,
  none of which this story changes. Read the clock once at a start, for `started_at` and
  the id alike. Write `start`'s seven steps in their order, the three helpers `guarded`,
  `steady` and `frame`, the task's six steps as "The task, in US1" gives them, with the
  lease given up before the focus check and before the stack and taken again between
  them, and the ending in its order. In this story a failed take, solve or stack start
  ends the run at once, every focus verdict goes on, the counts stay at zero, the
  stacking loop ends only on a stop or the mount's ending, and `resume` always refuses,
  with its two sentences (plan.md § The machine).
  A stop of the run ends it `RunStopped`; one
  that arrives while the state is `slewing` first awaits `mount.stop()` and ends it
  `Stopped`; and `stop` takes and lets go the lock of `start` before it sets the
  signal. At an ending read `mount.track_on()`, make a `RunStopped` that finds Track
  off `Stopped`, and set `saved` and `track_on`; in `view`, an ended run whose
  `track_on` is true reads the mount part again and keeps a false for good.
  Keep the latest run under one std `Mutex` in an inner `Arc` the task shares; the task
  holds that `Arc` and never the `Runner`, and `Drop` aborts the task. Never call a
  part, read the imaging view or await while that lock is held (plan.md § Plan traps
  23). What this story writes for a refused start, a stop, the mount's endings and a
  dropped runner is asserted by US2's tests (US2-S6 to US2-S10).
- [ ] T005 [US1] Run the `lint` and `test` gates with every existing test unchanged and
  green, US12's included. Commit the green story (US1-S1 and US1-S2).

## Phase 2: User Story 2 — Failed steps, counts and warnings, over injected steps (Priority: P1)

### Tests first

- [ ] T006 [US2] Create `crates/finder/tests/console/imaging_run_steps.rs`, the module
  `imaging_run_steps` of `console`, with `mod imaging_run_steps;` in its alphabetical
  place in `crates/finder/tests/console/main.rs`, and its own helpers for a scripted
  run and a settle. Write failing tests for failed takes, 5 s apart, and a stop of the
  run inside that wait (US2-S1), and for failed solves,
  a solve that cannot save, and a focus check that cannot save or whose solver cannot
  run (US2-S2).
- [ ] T007 [US2] Add failing tests to that file for the stack's counts, the tracker's
  pause and a slightly soft focus verdict in both units (US2-S3), and for a stack that
  cannot start, by each of five kinds of failure, or does not end cleanly, the warning
  for subs kept because nothing was stacked, and the message of an ending in progress
  with `Step::StackStop` held (US2-S4), and for the words a run adds to its message
  while it waits for the tracker in `solving` (US2-S5). Then write, in the same file,
  the tests of US1's machine that this story holds: a mount that is not steady (US2-S6);
  a stop in each of five states, with the aborted take, the stopped focus check and the
  lease given back last, the two sentences of a stop, `mount.stop` asked only of the run
  still slewing, `ended.saved` and `ended.track_on`, the second read again after the
  mount is set stopped (US2-S7); the seven endings of the mount's motion while stacking
  with their sentences, `saved` and `track_on`, and one at `first_frame` with `take`
  held and one while the mount is slewing (US2-S8); a runner dropped while its run
  stacks, the test keeping no `Steps` of its own (US2-S9); the start's refusals, with
  the lease asked last before Track and given back after a refused Track (US2-S10).
  These five pass before T008's change and must pass after it. If one fails before it,
  the fault is US1's: correct `run.rs` in T008, and escalate a fault of `script.rs`,
  which no story after US12 writes.

### Implementation

- [ ] T008 [US2] In `crates/finder/src/imaging/run.rs`, add `FRAME_TRIES` and
  `SOLVE_TRIES` and replace what US1 did with a failed take, solve and `stack.start` by
  the table of plan.md § Failed steps and counts in the machine: counted takes and
  solves, the two kinds of failed focus check that end the run, a failed `stack.start`
  sorted by its kind, the warning for a `stack.stop` that fails at an ending, the
  warning of `SlightlySoft`, and step 6 copying `stack.progress()` into `subs` and
  showing `paused_by_tracker`. Add `RETRY_WAIT`: after a counted take that is not the
  third, wait it, guarded, before the next take. In step 6 copy the counts first, then
  judge the mount's ending and a stop, then show the state. While `steady(None)` waits
  in `first_frame`, `focus_check`, `solving` or `centring`, add
  ` Waiting for the mount to settle; nothing to do.` to the state's message. At an
  ending, first
  note that the run is ending, which makes the message of a run that started its stack
  `Saving the stack: {stacked} stacked.`; read `stack.progress()` once more after
  `stack.stop` has answered and copy its counts, judging no rule on them, before
  `ended` is set; and with `keep_subs` false and nothing stacked add the warning
  `The subs were kept: nothing was stacked.` (plan.md § The
  machine, Ending). Read neither `running` nor `failed` yet.
- [ ] T009 [US2] Run the `lint` and `test` gates with every existing test unchanged and
  green, US12's and US1's included. Commit the green story (US2-S1, US2-S2, US2-S3,
  US2-S4, US2-S5, US2-S6, US2-S7, US2-S8, US2-S9 and US2-S10).

## Phase 3: User Story 3 — Start, stop and read a run over HTTP, and its record (Priority: P1)

### Tests first

- [ ] T010 [US3] Create `crates/finder/tests/console/imaging_run_routes.rs`, the module
  `imaging_run_routes` of `console`, with `mod imaging_run_routes;` in its alphabetical
  place in `crates/finder/tests/console/main.rs`, and the scripted console of spec.md
  § User Scenarios: 031's console rig, its session kept in `session.json` under the
  scratch folder with `Session::with_state_file`, with
  `with_imaging_run_steps(script.steps())`
  added last, R under the test's scratch folder, and helpers to send a request with and
  without an `Origin`, to wait on `GET /api/imaging`, and to parse `run.json` with
  `serde_json`. Write failing tests for a start with its view, session, `session.json`
  and `run.json` in the stack's folder (US3-S1), the refusals, each route of a console
  without parts sent a body it would otherwise accept (US3-S2), and a pause written to
  `run.json`, then `DELETE` sent right after `set_stack(5, &[], false)` with no settle
  between (US3-S3).
- [ ] T011 [US3] Add failing tests to that file for `continue` outside a pause (US3-S4),
  the run under `no_link` (US3-S5), two runs in one second that end before stacking
  (US3-S6), what `run.json` must not hold
  (US3-S7), the record refreshed at most once every 60 s while the stacked count rises
  (US3-S8), and `unfinished_run` on a console built over a folder whose newest record
  has no `ended` (US3-S9), a `runs` folder that cannot be written, under a run that
  stacks and under one stopped at `first_frame`, each with the warning that the record
  could not be saved (US3-S10), and three changes between `stacking` and
  `paused_by_tracker` within seconds, of which `run.json` lists the first, and one 60 s
  later (US3-S11).

### Implementation

- [ ] T012 [US3] Create `crates/finder/src/imaging/run/store.rs` with `FORMAT`, `Event`
  and `Store`, and add `pub mod store;`, `Runner::with_folder`, the list of events and
  the writes of `run.json` to `crates/finder/src/imaging/run.rs`, as plan.md § The
  record gives them: nothing written before the run stacks, the stack's folder adopted
  when the stack has started, one write at each later change of state, a failed write
  ending the run `save_failed`, a folder made at the ending for a run that never
  stacked, a failure there a warning, every write synchronous; and one more write when
  `subs.stacked` has risen since the last and that one is `RECORD_EVERY` old, which
  pushes no event. A change between `stacking` and `paused_by_tracker` is listed and
  written only when no such change was listed in the `RECORD_EVERY` before it.
  `with_folder` reads the newest `runs/*/run.json` once, and
  `Runner::unfinished` answers its folder when it has no `ended`, until the first
  accepted start. At an ending the `ended`
  event is pushed and the record made and written before the run counts as over: only
  then may a new start pass and `stop`'s waiters wake (plan.md § The machine, Ending).
- [ ] T013 [US3] In `crates/finder/src/imaging/mod.rs`, add `run` and `unfinished_run`
  to `ImagingView` and
  `attach_run` to `Imaging`, setting both wherever the view is built, in every state
  (plan.md § `crates/finder/src/imaging/mod.rs`). In `crates/finder/src/api.rs`, add the
  `imaging_runner` field, `with_imaging_run_steps` and `imaging_runner` (plan.md § The
  console).
- [ ] T014 [US3] In `crates/finder/src/imaging/routes.rs`, add `start_run`, `stop_run`
  and `continue_run` with their order of checks, the start wrapped in
  `api::start_motion` with `page` and `imaging_run` (plan.md § The routes). Add the
  three route lines to `crates/finder/src/lib.rs`, worded as that section gives them.
  The README is US11's.
- [ ] T015 [US3] Run the `lint` and `test` gates with every existing test unchanged and
  green, US12's, US1's and US2's included. Commit the green story (US3-S1, US3-S2, US3-S3,
  US3-S4, US3-S5, US3-S6, US3-S7, US3-S8, US3-S9, US3-S10 and US3-S11).

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
  § User Scenarios defines them. Write failing tests for a run that needs no re-centre,
  with the tolerance exactly 90.0, `slewing` read while Track is `armed`, nothing
  later than `tracking` read in the 9.5 s after Track first read `tracking`, and the
  imaging frame's purpose `run` (US4-S1), and for the refusals of a start with the lease
  given back, an unresponsive scope node among them (US4-S2).
- [ ] T017 [US4] Add failing tests to that file for Stop while stacking (US4-S3), the
  five endings of the mount's motion and the imaging frame refused during a run with
  `the imaging camera is held by the imaging run`, a tracker that is not steady
  near M42's transit, a push of 6′ undone 3 s later, and a `DELETE` while the run
  still slews (US4-S4), and the mount part alone:
  point, point again, and point again after a Stop, with rest read from 2 s after the
  stop, and, on a rig of its own, point again after the target was changed
  (US4-S5), and a start refused with `an exposure is in progress` while a single
  imaging frame is in flight, with the lease given back (US4-S6), each against the
  simulator's truth, the session or 031's own route as its scenario says. In US4-S5,
  await each part call to its answer before the next request
  is sent, or spawn it (plan.md § Plan traps 25).

### Implementation

- [ ] T018 [US4] Create `crates/finder/src/imaging/run/console.rs` with `CENTRE_SHARE`,
  `CENTRE_FLOOR_ARCSEC`, `mount`, `camera` and `solver`, and add `pub mod console;` to
  `crates/finder/src/imaging/run.rs`. Write the mount part's `target`, `point`, the
  table of `state` with its `repointing` flag, and `repoint`'s steps 1, 2 and 4 to 6:
  refuse when the stop generation has changed, Track off, Track on under
  `start_motion`, the generation compared before Track's answer is read, Stop asked
  again when it changed, and only then Track's error as a failure of the mount (plan.md
  § The console's mount part). Keep the id `target` answers, refuse in `point` when
  the session's target is no longer that one, and write step 3a: a re-centre whose
  target was changed answers `Ok` without asking Track. In `state`, read steady only
  when 036's tracker was steady over the larger of 032's `FIT_REACH_S` and the
  exposure setting (`steady_between`), and end `pushed_away` only on a push that is
  `following` or has read `moving` for `PUSH_GRACE`, a younger one reading not steady.
  Write `stop` over `api::stop`, `track_on` and `steady_between`. Step 3, the wait, is
  US5's. Write the camera part over
  031 US9's lease, with `HOLDER`, an `Arc` of the lease kept under a std `Mutex`, the
  lease taken in `hold` before the look for an imaging frame in flight and before the
  region-of-interest check, and the holder's take with the purpose `run` at the
  camera's settings as they stand, setting nothing and asking nothing of 036's
  steering; and the solver part, with the session's aim point read before and after
  the solve, and 032's `Session::held_aim` after it, for `aim_moved`, and
  `near_arcsec` from `CENTRE_NEAR_SHARE`; a take that 031 answers with no frame is a
  camera failure (plan.md § The
  console's camera and solver parts).
- [ ] T019 [US4] Run the `lint` and `test` gates with every existing test unchanged and
  green, US12's and US1's to US3's included. Commit the green story (US4-S1, US4-S2, US4-S3,
  US4-S4, US4-S5 and US4-S6).

## Phase 5: User Story 5 — Centring on the sensor (Priority: P1)

### Tests first

- [ ] T020 [US5] Create `crates/finder/tests/console/imaging_run_centring.rs`, the
  module `imaging_run_centring` of `console`, with its `mod` line in
  `crates/finder/tests/console/main.rs`, and failing scripted tests for one re-centre
  (US5-S1), a target 600″ off that is never centred, its offset written `10.0′`
  (US5-S2), and a solve that measured no new aim point, which is no try, four of them
  in a row with the target 400″ off, which is stacked with a warning, and 600″ off,
  which ends the run, the count started again by a re-centre, a target 400″ off after
  three tries that is stacked with a warning, both warnings saying at 400″ that the
  target is stacked where it is and at 180″ and 150″ that the stack's steering will
  bring it in, a failed `repoint`, a mount that is not steady after one, and a stop of
  the run while `repoint` is held (US5-S3).
- [ ] T021 [US5] Create `crates/finder/tests/motion/imaging_run_centring.rs`, the module
  `imaging_run_centring` of `motion`, with its `mod` line in
  `crates/finder/tests/motion/main.rs`, holding its own build of the run rig with no
  aim point. Write failing tests for centring from no aim point with the console's own
  mount, camera and solver parts, with the tolerance exactly 90.0 and Track read as no
  longer `off` within 2 s of the first sample that read it `off` while centring
  (US5-S4), for Stop while centring with the script's solver given 400″ four times
  (US5-S5), for the same centring at an
  exposure of 20 s, with 032's last aim reading never `moving` (US5-S6), and for a
  target changed as soon as the run reads `centring` (US5-S7), against the simulator's
  truth, the session and the simulator's mount.

### Implementation

- [ ] T022 [US5] In `crates/finder/src/imaging/run.rs`, add `CENTRE_TRIES` and
  `CENTRE_IDLE` and make step
  4 of the task the centring loop of plan.md § Centring in the machine, with `repoint`
  awaited unguarded and only after a solve whose `aim_moved` is true, a solve that
  measured no new aim point counted apart from the tries, and a target within
  `near_arcsec` stacked with its warning after the last try, and after the fourth solve
  in a row that measured no new aim point, each warning in the form its offset gives
  (plan.md § The run's words).
- [ ] T023 [US5] In `crates/finder/src/imaging/run/console.rs`, add `FRESH_POLLS` and
  step 3 of `repoint`: after the part's own Track off and with Track off, the wait for
  a live pointing newer than the one found then, at most `FRESH_POLLS` polls, going on
  to ask Track when none has come and refusing as stopped, with `repointing` cleared,
  when the stop generation changes meanwhile (plan.md § The console's mount part).
- [ ] T024 [US5] Run the `lint` and `test` gates with every existing test unchanged and
  green, US12's and US1's to US4's included. Commit the green story (US5-S1, US5-S2, US5-S3,
  US5-S4, US5-S5, US5-S6 and US5-S7).

## Phase 6: User Story 6 — The focus gate and its pause (Priority: P1)

### Tests first

- [ ] T025 [US6] Create `crates/finder/tests/console/imaging_run_focus.rs`, the module
  `imaging_run_focus` of `console`, with its `mod` line in
  `crates/finder/tests/console/main.rs`, and failing scripted tests for the pause, with
  its message and `pause_ends_at` (US6-S1), checking again (US6-S2), going on anyway
  (US6-S3), the 900
  s limit, read 899.5 s after the start and 1 s later (US6-S4), and a stop, a mount ending, an aid that cannot open and a resume
  outside a pause (US6-S5).
- [ ] T026 [US6] Add failing tests to that file for the route on the scripted console
  (US6-S6), and, on 033's focus rig with the console's own camera and focus parts and
  the script's mount, solver and stack, set up by running the focus aid until the
  imaging view's `focus.record.tonight` is present, for a run at the simulator's focus
  error 0.0 with `focus.unit` `arcsec`, a
  defocused one found in at most ten steps of 1.0, a refocused one, a start while the
  focus aid runs, refused with `the imaging camera is held by the focus aid` and
  followed by no `mount.point`, a
  camera with no focus record, and a run whose own check is stopped by a
  `DELETE /api/imaging/focus` the test spawns and does not await, which ends the run
  `stopped` (US6-S7). Add failing scripted tests for a check repeated once when the
  tracker was not steady during it (US6-S8), and for a second resume and a resume that
  comes after the limit has fired, with `Step::CloseAid` held (US6-S9), for the five
  other checks that pause, each with its words, a check that did not
  finish reading as unmeasurable only when its failure is `NotSolved` (US6-S10), and for
  a check still clearly
  out after `resume(false)`, which pauses again with no second `camera.release`
  (US6-S11).

### Implementation

- [ ] T027 [US6] In `crates/finder/src/imaging/run.rs`, add `FOCUS_PAUSE_LIMIT`, replace
  the going-on of step 3 for a verdict that pauses and for a check that did not finish
  with the pause of plan.md § The focus gate in the machine, a check that did not finish
  pausing with the words of unmeasurable when its failure is `NotSolved` and with
  `The focus check did not finish: {words}. Check the imaging camera, then check focus again or go on anyway.`
  otherwise, close the aid on every way
  out of a pause whether or not it opened, take the lease again on every way on to
  the solve, give it up no second time before a check that is repeated, and make
  `Runner::resume` hand `anyway` to the waiting task. Before a pause
  for `ClearlyOut` or `Unmeasurable`, ask `mount.steady_between` over the check's two
  clock readings and repeat an unsteady check once, a second unsteady one pausing with
  its warning. Set `pause_ends_at` while paused. Take resumes from the moment the
  state is `focus_pause`, act on the first, and refuse one that comes after a stop,
  the mount's ending or the limit has answered the pause, in that order when more than
  one is ready.
- [ ] T028 [US6] In `crates/finder/src/imaging/run/console.rs`, add `focus` over 033's
  `focus_start`, `focus_ended` and `focus_stop`, with the table that turns 033's view
  into a `FocusCheck`, as plan.md § The console's focus part gives it; a check 033 ended
  as `stopped` is `Failure::Stopped`, which ends the run, and one it ended as
  `no_region` is `Failure::NotSolved`.
- [ ] T029 [US6] Run the `lint` and `test` gates with every existing test unchanged and
  green, US12's and US1's to US5's included. Commit the green story (US6-S1, US6-S2, US6-S3,
  US6-S4, US6-S5, US6-S6, US6-S7, US6-S8, US6-S9, US6-S10 and US6-S11).

## Phase 7: User Story 7 — Stacking to a limit, and the endings a stack can cause (Priority: P1)

### Tests first

- [ ] T030 [US7] Create `crates/finder/tests/console/imaging_run_limits.rs`, the module
  `imaging_run_limits` of `console`, with its `mod` line in
  `crates/finder/tests/console/main.rs`, and failing scripted tests for the count limit
  with a dropped sub in its counts (US7-S1) and with subs discarded (US7-S7), the time
  limit, read 58.5 s after the settle and 1 s later (US7-S2), and the same under the
  tracker's pause (US7-S8), 600 s of stacking with failed subs and none stacked
  (US7-S3), and a stacked sub that starts the time again (US7-S9), the stack's own
  failures and a stack
  that ends by itself, each through `end_stack` (US7-S4), a stack's failure and the
  mount's ending in one poll (US7-S5), and what the 600 s do not count: `behind` and
  `unprocessed` subs, and time paused by the tracker (US7-S6).

### Implementation

- [ ] T031 [US7] In `crates/finder/src/imaging/run.rs`, add `NO_SUB_LIMIT`,
  `NOT_FAILURES` and the five
  checks of plan.md § Limits and the stack's endings in the machine to the stacking
  loop, in their order and after the mount's ending and a stop are judged, the time
  limit on tokio's clock from the instant `stack.start` answered, and a stack that no
  longer runs ending the run `StackStopped`.
- [ ] T032 [US7] Run the `lint` and `test` gates with every existing test unchanged and
  green, US12's and US1's to US6's included. Commit the green story (US7-S1, US7-S2, US7-S3,
  US7-S4, US7-S5, US7-S6, US7-S7, US7-S8 and US7-S9).

## Phase 8: User Story 8 — The whole run on the console's own parts, and in the binary (Priority: P1)

### Tests first

- [ ] T033 [US8] Create `crates/finder/tests/motion/imaging_run_stack.rs`, the module
  `imaging_run_stack` of `motion`, with its `mod` line in
  `crates/finder/tests/motion/main.rs`, holding its own build of the run rig with
  `with_imaging_run()` and its focus aid run until the imaging view's
  `focus.record.tonight` is present, and failing tests for a whole run to three subs
  (US8-S1), for `keep_subs` false (US8-S2), for a `DELETE` while stacking (US8-S8), for
  a stack
  stopped from 035's own route (US8-S4), for an imaging folder whose `runs` is a
  regular file (US8-S5), for a scope node that stops answering while the run stacks
  (US8-S6) and for the reserve file, its size worked out from the imaging view, once the
  run has a folder and after (US8-S7), and a test that
  reads the run's four source files from `CARGO_MANIFEST_DIR` and finds nothing of
  GoTo, the tracker, the ephemeris or the scope link named in them (US8-S9). Create
  `crates/finder/tests/cli/imaging_run.rs`, the module `imaging_run` of `cli`, with
  `mod imaging_run;` in its alphabetical place in `crates/finder/tests/cli/main.rs`,
  and a failing test for the built binary under `--simulate` (US8-S3).

### Implementation

- [ ] T034 [US8] In `crates/finder/src/imaging/run/console.rs`, add `stack` over 035 and
  036, as plan.md § The console's stack part and the wiring gives it: the set point
  told to the steering, the stack started for the target's label, the counts with only
  the reasons above 0, the wait for the tracker, the stack's own ending sorted into a
  `Failure`, and with `keep_subs` false the removal of the subs `stack.json` lists.
  The part stops only the stack it started, and only while that stack reads
  `stacking`; when another stack has replaced it, `progress` answers the counts last
  read with `running` false, and `stop` answers `Ok(())` and removes no sub. A start
  that 035 answers with a stack whose `started_at` is before the part's own reading of
  the clock is another client's: a failure, with no id kept. Once its own stack has
  started the part writes `.reserve` in the stack's folder: `reserve_bytes` of zeros for
  the imaging view's camera and binning, `RESERVE_CHUNK` at a time with
  `tokio::task::yield_now().await` between two chunks and no `spawn_blocking`; it
  removes the file first thing in `stop`, and when a chunk cannot be written it removes
  the file, stops the stack it has just started and fails the start as `Save`. With
  `keep_subs` false it removes subs only
  when the stack it stopped was saved, has at least one sub stacked and ended as
  `stopped`. In
  `crates/finder/src/api.rs`, add `with_imaging_run`, cloning the state for the parts
  before the runner is kept. In `crates/finder/src/main.rs`, call it after the imaging
  solver is set, on the `--simulate` path too.
- [ ] T035 [US8] Run the `lint` and `test` gates with every existing test unchanged and
  green, US12's and US1's to US7's included. Commit the green story (US8-S1, US8-S2, US8-S3,
  US8-S4, US8-S5, US8-S6, US8-S7, US8-S8 and US8-S9).

## Phase 9: User Story 9 — The run on the Imaging page (Priority: P2)

### Tests first

- [ ] T036 [US9] Create `crates/finder/web/src/components/ImagingRunBlock.test.tsx` with
  failing tests for US9-S1, US9-S2, US9-S3, US9-S4, US9-S5 and US9-S6, stubbing `fetch`
  as 031's Imaging page tests do, with the cases each scenario lists: a limit typed as
  `0`, a draft that survives a new view, no pause buttons outside a pause, the elapsed
  time rounded down, no brackets with none dropped, a limit's ending as a status and an
  ended run's warnings; and for US9-S7, reading `DESIGN.md` and
  `.impeccable/design.json` with `readWebFile` from `src/test/files.ts`. Add failing
  tests for the ended line's forms when the stack was not saved, `1 sub`, and the line
  that Track is still on (US9-S8), the second tap of `Stop imaging run` under fake
  timers, the button while its request is out and while the run saves its stack
  (US9-S9), a refused or unanswered `Check focus again` and when its alert goes
  (US9-S10), the start form's placeholder, quiet line, size estimate and the line of
  an unfinished run (US9-S11), and the time a pause has left and the time stacked
  against the limit (US9-S12), and what a click on `Start imaging run` posts, with
  a refusal's alert gone once a run appears (US9-S13).

### Implementation

- [ ] T037 [US9] Add `ImagingRun`, `ImagingRunRequest`, `postImagingRun`,
  `deleteImagingRun` and `postImagingRunContinue` to `crates/finder/web/src/api.ts`,
  and `run` to `ImagingView`. Create
  `crates/finder/web/src/components/ImagingRunBlock.tsx` with the words of plan.md
  § The page, keeping its two fields and its checkbox in its own state, and render it
  in `crates/finder/web/src/components/ImagingPage.tsx` right after the title block.
  The three calls throw the block's own words when `fetch` rejects; the type gains
  `stacking_since`, `limit_at`, `pause_ends_at`, `ended.saved` and `ended.track_on`,
  and the view
  `unfinished_run`.
- [ ] T038 [US9] Add the Imaging run entry to `DESIGN.md` and the Imaging run component
  to `.impeccable/design.json` (plan.md § `DESIGN.md` and `.impeccable/design.json`).
  Run the `lint` and `test` gates with every existing Vitest and Playwright test
  unchanged and green, commit the rebuilt `crates/finder/web/dist`, and commit the
  green story (US9-S1, US9-S2, US9-S3, US9-S4, US9-S5, US9-S6, US9-S7, US9-S8, US9-S9,
  US9-S10, US9-S11, US9-S12 and US9-S13).

## Phase 10: User Story 10 — Four MCP tools (Priority: P2)

### Tests first

- [ ] T039 [US10] Create `crates/finder/tests/mcp/imaging_run.rs`, the module
  `imaging_run` of finder's `mcp` test binary, with `mod imaging_run;` in its
  alphabetical place in `crates/finder/tests/mcp/main.rs`, serving the scripted console
  through the door as `crates/finder/tests/support/mcp.rs` serves a state, with a call
  log the test reads. Write failing tests for the list (US10-S1), a start and a read
  (US10-S2), the errors, each tool of a console without parts called with arguments it
  would otherwise accept, and an unknown key of `continue_imaging_run` (US10-S3), the
  rate limit (US10-S4), stop (US10-S5) and continue, read back with
  `get_imaging_run` after a settle (US10-S8), a start during a direct axis move on
  `crate::support::mcp::sim::Rig` (`crates/finder/tests/support/mcp.rs:182`), and
  `stop_imaging_run` sending the recording mount no `StopAll`
  (US10-S6), and the token (US10-S7). Leave 1 s of virtual time between two starts but
  in US10-S4.

### Implementation

- [ ] T040 [US10] In `crates/finder/src/mcp/tools.rs`, add `ImagingRunArguments`,
  `ImagingRunContinueArguments` and the four tools with the descriptions, order of
  checks, parsing and answers of plan.md § The MCP tools, `start_imaging_run` answering
  and logging `runner.view()` as read once `start_motion` has answered. In
  `crates/finder/src/api.rs`, add `start_imaging_run` to the one `matches!` of
  `start_motion` plan.md § The console names. The README is US11's.
- [ ] T041 [US10] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US10-S1, US10-S2, US10-S3, US10-S4, US10-S5, US10-S6,
  US10-S7 and US10-S8).

## Phase 11: User Story 11 — The README and the field checklist (Priority: P3)

### Tests first

- [ ] T042 [US11] Create `crates/finder/tests/console/imaging_run_docs.rs`, the module
  `imaging_run_docs` of `console`, with its `mod` line in
  `crates/finder/tests/console/main.rs`, reading `README.md` from `CARGO_MANIFEST_DIR`.
  Write failing tests for the section's words, with `corner`, `7200` and
  `What to do next` in its prose before the checklist (US11-S1), the checklist's
  eighteen steps
  and the word `corner` in its step 8 (US11-S2), each text read to the next heading or
  to the end of the file, and the three rows of the API table
  and the four tools in MCP clients (US11-S3).

### Implementation

- [ ] T043 [US11] In `crates/finder/README.md`, add the three rows to the `## API`
  table, the four tools to `## MCP clients` with the count in the sentence before its
  list made four higher, and the sections `## An imaging run` and
  `### Field checklist: the first imaging run`, with the rows, the content and the
  eighteen steps of plan.md § The README: rows, tools, the section and the field
  checklist, every state word and ending reason between backticks, and the known limit
  that the run's own imaging frames are not capped as subs are, in the prose and in
  step 8. The prose also says, as that section lists them: the four sentences of
  `stopped` and what the mount does after each, what to do next after each ending,
  in a fourth column of the table of endings headed `What to do next`, the waits and
  counts as they now are,
  that the imaging exposure should be at least as long as the finder camera's (and
  step 4 of the checklist says so), what a console that exits mid-run leaves, and the
  reserve file.
- [ ] T044 [US11] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US11-S1, US11-S2 and US11-S3).

## Phase 12: User Story 12 — The run's parts and words, and a script to test them with (Priority: P1)

Built first, before User Story 1 (spec.md § Work Graph). It has the number 12 because
the owner cut it from User Story 1 on 2026-11-19, and its tasks are numbered after the
last of the others for the same reason. It writes no `Runner`, no view and no message of
a state.

### Tests first

- [ ] T045 [US12] Create `crates/finder/tests/console/imaging_run_words.rs`, the module
  `imaging_run_words` of finder's `console` test binary, and add
  `mod imaging_run_words;` in its alphabetical place in
  `crates/finder/tests/console/main.rs`. Write failing tests for the two word lists; for
  the reason and the sentence of every ending, each sentence typed in from spec.md and
  compared with what `Ending::message` answers, with `1 sub`, the minutes of a time
  limit and an offset in arcseconds and in arcminutes; and for the words of `By`,
  `Verdict`, `Unit` and `Failure` (US12-S1). Then for `label` and `run_id`, the request
  parsed from JSON, the settings and the target written as JSON, and the four constants
  (US12-S2). No test of this story makes a `Runner`.
- [ ] T046 [US12] Add failing tests to that file, under
  `#[tokio::test(start_paused = true)]`, that call the script's five parts themselves:
  every default and the nineteen call words in their order, what is not listed, and the
  stack's `running` (US12-S3); and every control: the three queues, the four `fail_`
  controls, the five refusals, `unsteady`, `set_mount` with `track_on`, `set_stack` and
  `end_stack`, and `hold` and `release` on `Step::Take`, `Step::StackStop` and
  `Step::CloseAid`, the held call spawned by the test (US12-S4).

### Implementation

- [ ] T047 [US12] Create `crates/finder/src/imaging/run.rs` with what plan.md § The
  parts and the words gives, exactly: the module's doc, `pub mod script;`, the
  constants, `STATES`, `ENDINGS`, `StepFuture`, `By`, `Ending` with `reason` and
  `message`, `Failure`, `RunTarget`, `MountState`, the five traits with every method
  that section lists, `FrameRef`, `Verdict`, `Unit`, `FocusCheck`, `Centre`,
  `StackProgress`, `Steps`, `RunRequest`, `RunSettings`, `label` (one line that calls
  035's `crate::imaging::stack::service::label`; do not write the rule again) and
  `run_id`; and add `pub mod run;` to `crates/finder/src/imaging/mod.rs`, in
  alphabetical order. The mount's trait has `stop`, `track_on` and `steady_between`
  beside its first four calls; `Ending` has four variants whose reason is `stopped`,
  each with its sentence, and `NotCentred` its two. Write every ending's sentence from
  plan.md § The run's words, with its rules for an offset, the minutes of a time limit
  and `1 sub`. Write no `Runner`, no view type and no message of a state, a verdict or a
  warning: they are US1's (T003, T004).
- [ ] T048 [US12] Create `crates/finder/src/imaging/run/script.rs` with `Part`, `Step`,
  `STACK_FOLDER` and `Script`, whole: its defaults, every control, the holds and the
  call words exactly as plan.md § The script gives them. A call is listed when it is
  made, before any hold. A refusal lasts for the script's life, and a pushed answer
  uses no frame number. `set_stack` sets the counts and the tracker's pause and never
  whether the stack runs; only `stack.start`, `stack.stop` and `end_stack` change that.
  `Step` has `StackStop` and `CloseAid` beside the first five. `mount.stop` is listed
  and changes nothing; `track_on` answers from the mount's state as set, false for the
  five endings that stop the mount; `steady_between` answers true but for the next n
  calls after `unsteady(n)`; neither of the last two is listed. The default solve has
  a `near_arcsec` of 450.0.
- [ ] T049 [US12] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US12-S1, US12-S2, US12-S3 and US12-S4).

## Execution Order

```text
US12 ──► US1 ──► US2 ──► US3 ──► US4 ──► US5 ──► US6 ──► US7 ──► US8 ──► US9 ──► US10 ──► US11
```

- **US12** comes first: it writes the traits, the words and the script that the machine
  and every scripted test are written on, and proves them with no run.
- **US1** writes the view and the machine on them: every other story hangs a route, a
  real part, a page or a tool on its machine.
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
