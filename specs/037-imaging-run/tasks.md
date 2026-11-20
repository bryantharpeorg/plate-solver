---
description: "Task list for 037-imaging-run: The Imaging Run"
---

# Tasks: The Imaging Run

**Prerequisites** (flip.md § Prerequisites): every spec in spec.md's `depends_on_landed`
has landed and this trio is re-anchored to `main`; no operator-staged file is needed.

**Rules for every story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml`,
  `CODEOWNERS`, `ergane.yaml`, `.specify/` or `crates/finder/web/playwright.config.ts`.
  If an existing test fails, stop and escalate with its exact output. Do not edit it.
  The one exception is the `mod` line of each new finder test module, added in
  alphabetical order to the binary's `main.rs` your story's tasks name, which holds no
  test (spec.md SC-002). Change nothing else there. A test file an earlier
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
- Do not edit `specs/037-imaging-run/`, `specs/041-imaging-run-on-the-rig/` or
  `specs/042-imaging-run-page-and-mcp/`.
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
  The README is 042's US11's.
- [ ] T015 [US3] Run the `lint` and `test` gates with every existing test unchanged and
  green, US12's, US1's and US2's included. Commit the green story (US3-S1, US3-S2, US3-S3,
  US3-S4, US3-S5, US3-S6, US3-S7, US3-S8, US3-S9, US3-S10 and US3-S11).

## Phase 12: User Story 12 — The run's parts and words, and a script to test them with (Priority: P1)

Built first, before User Story 1 (spec.md § Work Graph), though its number and its
tasks' numbers come last. It writes no `Runner`, no view and no message of a state.

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
US12 ──► US1 ──► US2 ──► US3
```

- **US12** comes first: it writes the traits, the words and the script that the machine
  and every scripted test are written on, and proves them with no run.
- **US1** writes the view and the machine on them: every other story hangs a route, a
  real part, a page or a tool on its machine.
- **US2** completes the machine for a step that fails and for the stack's counts, before
  anything is served.
- **US3** serves the machine and gives it its record; 041's US4 to US6 drive their real parts
  through its routes.
- After US3, `041-imaging-run-on-the-rig` and then `042-imaging-run-page-and-mcp` build
  on this spec once it has landed.
- **`[P]`** marks nothing here: the stories form one chain.
