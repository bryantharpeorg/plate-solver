---
description: "Task list for 041-imaging-run-on-the-rig: The Imaging Run on the Console's Own Parts"
---

# Tasks: The Imaging Run on the Console's Own Parts

**Prerequisites** (flip.md § Prerequisites): every spec in spec.md's `depends_on_landed`
has landed and this trio is re-anchored to `main`; no operator-staged file is needed.

**Rules for every story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml`,
  `CODEOWNERS`, `ergane.yaml`, `.specify/` or `crates/finder/web/playwright.config.ts`.
  If an existing test fails, stop and escalate with its exact output. Do not edit it.
  The one exception is the `mod` line of each new finder test module, added in
  alphabetical order to the binary's `main.rs` your story's tasks name, which holds no
  test (spec.md SC-002). Change nothing else there. A test file 037 or an
  earlier story of this spec created is an existing test: add your tests in the new file your
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
  expected message by calling `Ending::message` (037's US12's test calls it, and compares what
  it answers with a sentence typed in), and none reads `run.json` or
  `stack.json` through the console's code (plan.md § Plan traps 6).
- Use the stand-in parts of `finder::imaging::run::script` for every part your story
  does not make real. Add no method to a part's trait and no field to `Steps`, do not
  change `Runner::new`, `start`, `stop`, `resume` or `view`, and after 037's US12 do not write
  `script.rs` (plan.md § Shapes that existing tests pin).
- The run asks the mount part and nothing else to move the mount. Import nothing of
  `crate::goto`, `crate::tracking`, `crate::ephemeris` or `scope_link` in
  `crates/finder/src/imaging/run.rs` or under `crates/finder/src/imaging/run/`, and of
  `crate::model` only `separation`, in `console.rs`. Write no identifier, import or
  arithmetic there for an axis, an altitude, an azimuth, a rate or axis counts, name no
  state of Track but `armed` and `off`, and write no focal length, field size or scale
  as a number. The rule is about code: the messages 037's plan.md § The run's words gives and
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
- Every code citation is `path:line` as read at 49bb210; a file 030 to 037 create is
  named without a line. If a cited line has moved, re-read it rather than guess.
- Add no dependency and change no `Cargo.toml`. Write no `unsafe`. Change nothing under
  `crates/simulator`, `crates/scope-node` or `crates/scope-link`.
- Do not edit `specs/037-imaging-run/`, `specs/041-imaging-run-on-the-rig/` or
  `specs/042-imaging-run-page-and-mcp/`.
- Run no tool the gates don't run: no `cargo deny`, `cargo machete`, Rust 1.96 or
  `ergane`.

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
  later than `tracking` read in the 11 s after Track first read `tracking`, and the
  imaging frame's purpose `run` (US4-S1), and for the refusals of a start with the lease
  given back, an unresponsive scope node among them (US4-S2).
- [ ] T017 [US4] Add failing tests to that file for Stop while stacking (US4-S3), the
  five endings of the mount's motion and the imaging frame refused during a run with
  `the imaging camera is held by the imaging run`, a tracker that is not steady
  near M42's transit, a push of 6′ undone 3 s later, a push that keeps moving, which
  ends the run between 10 s and 15 s after its first push, and a `DELETE` while the run
  still slews (US4-S4), and the mount part alone:
  point, point again, and point again after a Stop, with rest read from 2 s after the
  stop, and, on a rig of its own each, point again after the target was changed,
  `point` refused once the target was changed after `target` answered, Track turned on
  another target, Track turned on again after a Stop, a re-centre that Track refuses,
  and `near_arcsec` from one frame taken and solved by the camera and solver parts
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
  when 040's tracker was steady over the larger of 032's `FIT_REACH_S` and the
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
  green, 037's included. Commit the green story (US4-S1, US4-S2, US4-S3,
  US4-S4, US4-S5 and US4-S6).

## Phase 5: User Story 5 — Centring on the sensor (Priority: P1)

### Tests first

- [ ] T020 [US5] Create `crates/finder/tests/console/imaging_run_centring.rs`, the
  module `imaging_run_centring` of `console`, with its `mod` line in
  `crates/finder/tests/console/main.rs`, and failing scripted tests for one re-centre
  (US5-S1), a target 600″ off that is never centred, its offset written `10.0′`
  (US5-S2), and a solve that measured no new aim point, which is no try, four of them
  in a row with the target 400″ off, which is stacked with a warning, and 600″ off,
  which ends the run, the count started again by a re-centre, so that the 50″ solve
  after it stacks with no warning, the second `centring` message after a try, a target
  400″ off after three tries that is stacked with a warning, targets exactly at the
  tolerance and at `near_arcsec`, both warnings saying at 400″ that the
  target is stacked where it is and at 180″ and 150″ that the stack's steering will
  bring it in, a failed `repoint`, a mount that is not steady after one, and a stop of
  the run while `repoint` is held (US5-S3). The case of exactly 90″ passes before
  T022, and pins it once it is in.
- [ ] T021 [US5] Create `crates/finder/tests/motion/imaging_run_centring.rs`, the module
  `imaging_run_centring` of `motion`, with its `mod` line in
  `crates/finder/tests/motion/main.rs`, holding its own build of the run rig with no
  aim point. Write failing tests for centring from no aim point with the console's own
  mount, camera and solver parts, with the tolerance exactly 90.0 and Track read as no
  longer `off` within 2 s of the first sample that read it `off` while centring, and
  again with a finder frame every 2 s (US5-S4), for Stop while centring with the
  script's solver given 400″ four times (US5-S5), for the same centring at an
  exposure of 20 s, with 032's last aim reading never `moving` (US5-S6), and for a
  target changed as soon as the run reads `centring` (US5-S7), against the simulator's
  truth, the session and the simulator's mount; and for the mount part alone, with the
  true aim point set and no live pointing after its own Track off, whose re-centre goes
  on between 4.9 s and 5.5 s after it was asked (US5-S8), with this module's own wait
  for `Tracking { steady: true }`.

### Implementation

- [ ] T022 [US5] In `crates/finder/src/imaging/run.rs`, add `CENTRE_TRIES` and
  `CENTRE_IDLE` and make step
  4 of the task the centring loop of plan.md § Centring in the machine, with `repoint`
  awaited unguarded and only after a solve whose `aim_moved` is true, a solve that
  measured no new aim point counted apart from the tries, and a target within
  `near_arcsec` stacked with its warning after the last try, and after the fourth solve
  in a row that measured no new aim point, each warning in the form its offset gives
  (037's plan.md § The run's words).
- [ ] T023 [US5] In `crates/finder/src/imaging/run/console.rs`, add `FRESH_POLLS` and
  step 3 of `repoint`: after the part's own Track off and with Track off, the wait for
  a live pointing newer than the one found then, at most `FRESH_POLLS` polls, going on
  to ask Track when none has come and refusing as stopped, with `repointing` cleared,
  when the stop generation changes meanwhile (plan.md § The console's mount part).
- [ ] T024 [US5] Run the `lint` and `test` gates with every existing test unchanged and
  green, 037's and US4's included. Commit the green story (US5-S1, US5-S2, US5-S3,
  US5-S4, US5-S5, US5-S6, US5-S7 and US5-S8).

## Phase 6: User Story 6 — The focus gate and its pause (Priority: P1)

### Tests first

- [ ] T025 [US6] Create `crates/finder/tests/console/imaging_run_focus.rs`, the module
  `imaging_run_focus` of `console`, with its `mod` line in
  `crates/finder/tests/console/main.rs`, and failing scripted tests for the pause, with
  its message and `pause_ends_at` (US6-S1), checking again (US6-S2), going on anyway
  (US6-S3), the 900
  s limit, read 899.5 s after the start and 1 s later (US6-S4), and a stop, a mount ending, an aid that cannot open and a resume
  outside a pause (US6-S5). A resume and every ending leave no `pause_ends_at`
  (US6-S2, US6-S3, US6-S5).
- [ ] T026 [US6] Add failing tests to that file for the route on the scripted console
  (US6-S6), and, on 033's focus rig with the console's own camera and focus parts and
  the script's mount, solver and stack, set up by running the focus aid until the
  imaging view's `focus.record.tonight` is present, for a run at the simulator's focus
  error 0.0 with `focus.unit` `arcsec`, a
  defocused one found in at most ten steps of 1.0, a refocused one, a start while the
  focus aid runs, refused with `the imaging camera is held by the focus aid` and
  followed by no `mount.point`, a
  camera with no focus record, a rig with no imaging solver, whose run ends
  `solver_unavailable` at its focus check, and a run whose own check is stopped by a
  `DELETE /api/imaging/focus` the test spawns and does not await, which ends the run
  `stopped` (US6-S7). Add failing scripted tests for a check repeated once when the
  tracker was not steady during it, a no-record check that is not, the rule started
  again after a resume, and no `camera.release` before a repeat (US6-S8), and for a
  second resume and a resume that comes after the limit has fired, with `Step::CloseAid` held (US6-S9), for the five
  other checks that pause, each with its words, a check that did not
  finish reading as unmeasurable only when its failure is `NotSolved`, and a clearly-out
  check after one that did not finish and a `resume(false)`, which words its own
  verdict (US6-S10), and for
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
  for `ClearlyOut` or `Unmeasurable`, and not for a check that did not finish, ask
  `mount.steady_between` over the check's two
  clock readings and repeat an unsteady check once, a second unsteady one pausing with
  its warning. Set `pause_ends_at` while paused, and clear it at a resume and at every
  ending. Take resumes from the moment the
  state is `focus_pause`, act on the first, and refuse one that comes after a stop,
  the mount's ending or the limit has answered the pause, in that order when more than
  one is ready; the pause is answered by whoever takes it first, the task or a resume
  (plan.md § The focus gate in the machine).
- [ ] T028 [US6] In `crates/finder/src/imaging/run/console.rs`, add `focus` over 033's
  `focus_start`, `focus_ended` and `focus_stop`, with the table that turns 033's view
  into a `FocusCheck`, as plan.md § The console's focus part gives it; a check 033 ended
  as `stopped` is `Failure::Stopped`, which ends the run, and one it ended as
  `no_region` is `Failure::NotSolved`.
- [ ] T029 [US6] Run the `lint` and `test` gates with every existing test unchanged and
  green, 037's, US4's and US5's included. Commit the green story (US6-S1, US6-S2, US6-S3,
  US6-S4, US6-S5, US6-S6, US6-S7, US6-S8, US6-S9, US6-S10 and US6-S11).

## Phase 7: User Story 7 — Stacking to a limit, and the endings a stack can cause (Priority: P1)

### Tests first

- [ ] T030 [US7] Create `crates/finder/tests/console/imaging_run_limits.rs`, the module
  `imaging_run_limits` of `console`, with its `mod` line in
  `crates/finder/tests/console/main.rs`, and failing scripted tests for the count limit
  with a dropped sub in its counts (US7-S1) and with subs discarded (US7-S7), the time
  limit, read 58.6 s after the settle and 1 s later (US7-S2), and the same under the
  tracker's pause (US7-S8), 600 s of stacking with failed subs and none stacked, the
  words giving each reason's rise since the last stacked sub, and a single failed sub
  (US7-S3), and a stacked sub that starts the time again and keeps the drops it saw
  (US7-S9), the stack's own
  failures and a stack
  that ends by itself, each through `end_stack`, and a failure judged before the count
  limit (US7-S4), a stack's failure and the
  mount's ending in one poll (US7-S5), and what the 600 s do not count: `behind` and
  `unprocessed` subs, and time paused by the tracker (US7-S6). US7-S5 and US7-S9 pass
  before T031, and pin it once it is in.

### Implementation

- [ ] T031 [US7] In `crates/finder/src/imaging/run.rs`, add `NO_SUB_LIMIT`,
  `NOT_FAILURES` and the five
  checks of plan.md § Limits and the stack's endings in the machine to the stacking
  loop, in their order and after the mount's ending and a stop are judged, the time
  limit on tokio's clock from the instant `stack.start` answered, and a stack that no
  longer runs ending the run `StackStopped`.
- [ ] T032 [US7] Run the `lint` and `test` gates with every existing test unchanged and
  green, 037's and US4's to US6's included. Commit the green story (US7-S1, US7-S2, US7-S3,
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
  run has a folder and after, and `reserve_bytes` on the plan's worked figures in a
  plain `#[test]` (US8-S7), and a test, which passes before T034 too, that
  reads the run's four source files from `CARGO_MANIFEST_DIR` and finds nothing of
  GoTo, the tracker, the ephemeris or the scope link named in them (US8-S9). Create
  `crates/finder/tests/cli/imaging_run.rs`, the module `imaging_run` of `cli`, with
  `mod imaging_run;` in its alphabetical place in `crates/finder/tests/cli/main.rs`,
  and a failing test for the built binary under `--simulate` (US8-S3).

### Implementation

- [ ] T034 [US8] In `crates/finder/src/imaging/run/console.rs`, add `stack` over 035,
  036 and 040, as plan.md § The console's stack part and the wiring gives it: the set point
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
  removes the file first thing in `stop`, ignoring any failure to remove it, and when a
  chunk cannot be written it removes
  the file, stops the stack it has just started and fails the start as `Save`. With
  `keep_subs` false it removes subs only
  when the stack it stopped was saved, has at least one sub stacked and ended as
  `stopped`. In
  `crates/finder/src/api.rs`, add `with_imaging_run`, cloning the state for the parts
  before the runner is kept. In `crates/finder/src/main.rs`, call it after the imaging
  solver is set, on the `--simulate` path too.
- [ ] T035 [US8] Run the `lint` and `test` gates with every existing test unchanged and
  green, 037's and US4's to US7's included. Commit the green story (US8-S1, US8-S2, US8-S3,
  US8-S4, US8-S5, US8-S6, US8-S7, US8-S8 and US8-S9).

## Execution Order

```text
US4 ──► US5 ──► US6 ──► US7 ──► US8
```

- **`037-imaging-run`** has landed before US4: its machine, routes, record and script
  are on `main`.
- **US4** writes the console's own mount, camera and solver parts and proves them on the
  simulated mount with runs that need no re-centre.
- **US5** adds the centring loop to the machine and the wait a re-centre needs to the
  mount part, and proves a re-centre end to end.
- **US6** and **US7** each add a group of states or endings to
  `crates/finder/src/imaging/run.rs`, in the order a run meets them, so they cannot run
  together.
- **US8** adds the stack part, assembles the five parts and wires the binary.
- `042-imaging-run-page-and-mcp` builds on this spec once it has landed.
- **`[P]`** marks nothing here: the stories form one chain.
