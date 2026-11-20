---
state: draft
depends_on_landed: ["014-goto", "015-tracking", "016-mcp", "017-console-stop-confirm", "030-imaging-camera-link", "031-imaging-frames-on-console", "032-imaging-solve-and-aim-point", "033-focus-reading-and-aid", "034-dark-and-flat-frames", "035-live-stack", "036-tracking-from-subs", "038-track-follows-a-push", "040-stack-waits-for-steady", "043-imaging-abort-answers-in-time"]
# The imaging run, first of three pieces: the run's parts, words and script, the machine
# with its retries, failures, counts and soft-focus warning, and the routes and the record,
# all on the scripted parts (US12, US13, US1, US2, US3). 041-imaging-run-on-the-rig gives the run
# the console's own parts; 042-imaging-run-page-and-mcp its page, MCP tools and README.
# Depends on 014 to 017, 030 to 036, 038, 040 and 043 having landed. State: draft, re-anchored to
# `main` before the flip. history.md has how it got here; flip.md what the flip needs.
# Re-anchored on 2026-11-20 to `main` at `6d79292b`, after 040 landed (#336 to #340), and
# carried across 043 to `2ae73c66` (#342) once it landed the same day.
---

# Feature Specification: The Imaging Run

**Feature Branch**: `037-imaging-run`

**Created**: 2026-11-15

**Status**: Draft

**Input**: Specs 030 to 036 give the console an imaging camera, imaging solves, a measured
aim point, a focus reading, calibration frames, a live stack and tracking from the subs,
each with its own route and MCP tool. This spec joins them into one procedure, the
imaging run: one ask, from the Imaging page or an MCP client, that goes to the session's
target, tracks it, takes a first imaging frame, checks focus, centres the target on the
imaging camera's sensor and stacks subs until it is stopped or a limit is reached, and
that ends safely whatever goes wrong. The run is a client of the console's GoTo, Track
and Stop and adds no other way to move the mount. An MCP client chooses the target and
the settings, starts, watches and stops; it never puts the steps in order itself. This spec is the first of three pieces cut from that procedure
along its build order. It writes the run's parts, its words and a script that stands in
for the parts, the machine with its retries and counts, and the routes and the record,
all proven on the script. `041-imaging-run-on-the-rig` then gives the run the console's
own mount, camera, focus, solver and stack parts and the centring, the focus gate and the
limits, and `042-imaging-run-page-and-mcp` its block on the Imaging page, its four MCP
tools and the README.

## User Scenarios & Testing *(mandatory)*

Every scenario shares these definitions unless it says otherwise.

- **The gate command.** Every Rust scenario runs in
  `cargo nextest run --workspace --all-targets --all-features --locked`, the `test`
  gate's first command, each test in its own process. The page's scenarios run in
  `npm test`, a later command of the same gate, in `crates/finder/web`.
- **Paused time.** A test of a `Runner` or of the in-process console runs under
  `#[tokio::test(start_paused = true)]`, so every wait is in virtual time. A test that
  starts the built `finder` is a plain `#[test]`.
- **The script** is `finder::imaging::run::script::Script` (US12): one stand-in for all
  five parts of a run. Left alone it answers: the target M42 (`id` `NGC 1976`, `name`
  `M42`, RA 83.82°, Dec −5.39°, the script's own constants); a camera and a solver that
  are ready and a lease that is free; a mount that is tracking and steady; imaging
  frames with ids 1, 2, 3 … and the file `frames/2026-01-16/00000N.fits`; a focus check
  that is good with a reading and a record of 3.0″; a solve 0″ from the sensor's centre
  with a tolerance of 90″ and a `near_arcsec` of 450″ that moved the aim point; a
  tracker that was steady over any past window; and a stack that starts in the
  folder `runs/2026-01-16T02-00-10Z-M42`, **the script's stack folder**, and runs with
  nothing stacked or dropped until it is stopped. A test changes one answer at a time:
  `set_mount`, `refuse`, `push_frame`, `push_focus`, `push_solve`, `fail_repoint`,
  `fail_open_aid`, `fail_stack_start`, `fail_stack_stop`, `unsteady`, `set_stack` and
  `end_stack`. `unsteady(n)` makes the mount answer the next n questions about a past
  window, `steady_between`, with no. The script's Track is on unless its mount is set
  to an ending that stops the mount (`Stopped`, `TooLow`, `SunUp`, `LinkLost`,
  `MountFailed`).
  `set_stack(stacked, dropped, waiting_for_tracker)` sets the stack's counts and whether
  it waits for the tracker, `dropped` a list of reason and count, and never ends the
  stack; `end_stack(failed)` makes it end by itself. `hold(step)` makes every call
  of a step wait, once it is listed, until `release(step)`, which also ends the hold.
  `calls()` is the list of calls the run made, in order, as plan.md § The script words
  them; reading the mount's or the stack's state, whether Track is on, or whether the
  tracker was steady over a window is not listed.
- **A scripted run** is a `Runner::new(script.steps(), clock)` whose clock is a
  `FixedClock` at Unix 1768528800 (`2026-01-16T02:00:00Z`,
  `crates/finder/src/session.rs:41`), started with
  `runner.start(RunRequest::default(), By::Page)` unless the scenario gives another
  request. **Settling** is sleeping 1 s of virtual time, four of the run's polls, before
  reading `runner.view()`.
- **The straight path** is what `calls()` holds when a scripted run has reached
  `stacking` with nothing changed: `camera.ready`, `solver.ready`, `mount.target`,
  `camera.hold`, `mount.point page`, `camera.take`, `camera.release`, `focus.check`,
  `camera.hold`, `camera.take`, `solver.solve 2`, `camera.release`, `stack.start M42`.
  The first imaging frame has the id 1 and the one that is solved the id 2. The lease
  is given up around the focus check and before the stack, which take it themselves.
- **The scripted console** is 031's console rig (031's spec.md § User Scenarios: a
  `Simulator` at 40° N 75° W on a `VirtualClock` from Unix 1768528800, a `SimNode` with
  the token `test-token`, a paired session, `with_link`, `with_live()` and
  `with_imaging(R)`), its session kept in `session.json` under the test's scratch
  folder (`Session::with_state_file`), with `with_imaging_run_steps(script.steps())`
  added last. **R** is the child path `imaging` of the test's scratch folder.
- **Scratch folders.** A finder test module takes its folder from
  `crate::support::Scratch::new(name)` (`crates/finder/tests/support/mod.rs:12`), which
  makes the folder under the directory cargo names and removes it when dropped.
- **Requests** go through the router in-process, as `request` in
  `crates/finder/tests/support/live.rs:90` sends them, with `Content-Type:
  application/json` and no `Origin`. A scenario that names an `Origin` builds its
  request itself with a `Host` of localhost on port 8080. A foreign `Origin` is
  `http://elsewhere.example`.
- **Waiting until** a condition means reading `GET /api/imaging` every 100 ms until it
  holds, for at most 900 s of virtual time, and failing otherwise.
- **Comparing a view.** A test compares `run` key by key, never whole, because later
  specs may add keys. `elapsed_s` is compared within 0.5 s, twice the run's poll.
- **Oracles.** Each scenario names its oracle: the simulator's truth; the session; the
  answer of another route of 030 to 036 at the same moment; or, for a scenario that
  checks only what the run says, calls and writes, the words, states, reasons and call
  lists the scenario itself states, which the test compares exactly. A test parses
  `run.json` and `stack.json` itself with `serde_json`, never through the console's
  code. A count or a word has no tolerance; every other tolerance is stated where the
  number is.


### User Story 1 - The run's states, over injected steps (Priority: P1)

As the console, I carry one imaging run from its start to its ending through five parts I
am handed: the mount, the camera, the focus check, the solver and the stack. I say at
every moment which state the run is in and why, and I end it cleanly when the observer
stops it or when the mount's motion ends.

**Why this priority**: Every other story hangs a real part, a route, a page or a tool on
this machine. It is tested with stand-in parts, so it needs none of 030 to 036's parts
to be right. It is written on User Story 12's traits, words and script, and changes
none of them.

**Independent Test**: `crates/finder/tests/console/imaging_run.rs`, the module
`imaging_run` of finder's `console` test binary. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test console imaging_run::`.

This story writes the whole machine after what User Story 12 wrote, and what is made
from a run's own state: the view's types, each state's message, the focus sentences and
the warnings. Its two scenarios assert the path on which every step answers and two
failures on it. What the machine does with a refused start, a mount that is not steady,
a stop, the mount's endings and a dropped runner is this story's code too, and is
asserted by US2's test module (US2-S6 to US2-S10), so that this story's diff stays under
64 KiB (plan.md § Risks).

**Acceptance Scenarios**:

1. **Given** a script holding `take`, `check`, `solve` and `stack_start`, with its mount
   set to slewing, **When** a test starts a scripted run, settles, sets the mount to
   tracking and steady, settles, and then releases the four steps one at a time in that
   order, settling after each, **Then** a committed test asserts that:
   - the start answers a view whose `id` is `2026-01-16T02-00-00Z-M42`, `by` `page`,
     `target` `{"id":"NGC 1976","name":"M42","ra_deg":83.82,"dec_deg":-5.39}`,
     `started_at` 1768528800.0, `settings`
     `{"subs":null,"duration_s":7200.0,"keep_subs":true}`, `subs`
     `{"stacked":0,"dropped":{}}`, the state `slewing` and no `folder`;
   - the states read at the six settles are, in order, `slewing`, `first_frame`,
     `focus_check`, `solving`, `stacking`, `stacking`, with the messages
     `Slewing to M42.`, `Taking a first imaging frame.`, `Checking focus.`,
     `Solving an imaging frame.` and, twice, `Stacking M42: 0 stacked, 0 dropped.`;
     and at the second settle, with `take` still held, the last of `calls()` is
     `camera.take`: a held call is listed before it waits;
   - at the fifth settle the view has no `folder`, and at the sixth `folder` is the
     script's stack folder, `stacking_since` is 1768528800.0, `limit_at` is
     1768536000.0, 7200 s later,
     `focus` is
     `{"verdict":"good","reading":3.0,"record":3.0,"unit":"arcsec"}` and `centring` is
     `{"tries":0,"offset_arcsec":0.0,"tolerance_arcsec":90.0}`;
   - `calls()` is exactly the straight path;
   - written with `serde_json`, the view read at the first settle is an object with
     exactly the keys `id`, `state`, `by`, `target`, `started_at`, `elapsed_s`,
     `settings`, `subs` and `message`, and the view read at the sixth has those and
     `focus`, `centring`, `stacking_since`, `limit_at` and `folder`: a key whose value
     is not there yet is left out, and so is an empty `warnings`.
2. **Given** a script, **Then** a committed test asserts that:
   - with `check` held, then
     `refuse(Part::Hold, "the imaging camera is held by a set of darks")`, then `check`
     released and the test settles, the run has ended `camera_failed` with
     `Ended: the imaging camera failed: the imaging camera is held by a set of darks.`
     and `calls()` holds no `solver.solve`: the lease could not be taken back after the
     focus check; and its `ended`, written with `serde_json`, is an object with exactly
     the keys `reason`, `at`, `message`, `saved` and `track_on`;
   - with `push_focus(Err(Failure::Other("the focus check ended: no_region")))`, after a
     settle, `focus` is `{"verdict":"unmeasurable","unit":"px"}` and `warnings` is
     `["The focus check did not finish: the focus check ended: no_region."]`. The test
     reads `focus` and `warnings` only, and asserts nothing of the state or of `calls()`
     after `focus.check`: from 041's US6 this run pauses there;
   - a run started with `RunRequest { duration_s: Some(60.0), ..RunRequest::default() }`
     reaches `stacking` after a settle with `limit_at` 1768528860.0: the limit is the
     request's.

---


### User Story 2 - Failed steps, counts and warnings, over injected steps (Priority: P1)

As the console, I do not give a run up for one imaging frame the camera drops or one
solve that finds nothing: I ask again, a counted number of times, and only then end the
run and say which part failed. While the stack runs I pass on its counts, I say when the
tracker has paused it, and I keep a warning when focus is slightly soft.

**Why this priority**: A night's first run meets a dropped imaging frame or a field that
does not solve long before it meets a clean path, and the record, the page and the tools
all show what this story computes.

**Independent Test**: `crates/finder/tests/console/imaging_run_steps.rs`, the module
`imaging_run_steps` of `console`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test console imaging_run_steps::`.

Scenarios US2-S6 to US2-S10 assert the machine as US1 leaves it. Their tests pass before
this story's code is written and must still pass after it.

**Acceptance Scenarios**:

1. **Given** a script, **Then** a committed test asserts that:
   - with `push_frame` given `Err(Failure::Camera("boom"))` twice, `calls()` holds one
     `camera.take` 4.5 s after the start and two 1 s later, and 11 s after the start
     the run has reached `stacking` and `calls()` holds `camera.take` four times: three
     for the first imaging frame, 5 s apart, and one for the solve;
   - with it given `Err(Failure::Camera("boom"))` twice and then
     `Err(Failure::Camera("gone"))`, 11 s after the start the run has ended
     `camera_failed` with `Ended: the imaging camera failed: gone.` after exactly three
     `camera.take`, the last failure's words, and `calls()` holds no `focus.check`; and
     on another such run a `runner.stop()`
     awaited 2 s after the start, inside the first wait, answers within 1 s a run ended
     `stopped` after one `camera.take`;
   - with `Err(Failure::Save("could not save the imaging frame: disk"))` once, the run
     ends `save_failed` at the first take, with
     `Ended: the console could not save to its imaging folder: could not save the imaging frame: disk. Check the disk.`
2. **Given** a script, **Then** a committed test asserts that:
   - with `push_solve` given `Err(Failure::NotSolved("the imaging solver found no solution"))`
     twice, the run reaches `stacking`, and `calls()` after `focus.check` is
     `camera.hold`, `camera.take`, `solver.solve 2`, `camera.take`, `solver.solve 3`,
     `camera.take`, `solver.solve 4`, `camera.release`, `stack.start M42`: the focus
     check is not repeated;
   - with it given three times, the run ends `solver_failing` with
     `Ended: 3 imaging frames in a row could not be solved: the imaging solver found no solution. Check for cloud, dew and focus.`;
   - with `Err(Failure::SolverUnavailable("the imaging solver has no star database it can read"))`
     once, the run ends `solver_unavailable` at the first solve, with
     `Ended: the imaging solver cannot run: the imaging solver has no star database it can read.`;
   - with `push_solve` given
     `Err(Failure::Save("could not save the imaging solve: disk"))` once, the run
     ends `save_failed` with
     `Ended: the console could not save to its imaging folder: could not save the imaging solve: disk. Check the disk.`
     and `calls()` holds `solver.solve` once: a solve that cannot save is not a
     solve that found nothing;
   - with `push_focus` given
     `Err(Failure::Save("could not save the imaging frame: disk"))` the run ends
     `save_failed`, and given
     `Err(Failure::SolverUnavailable("this console has no imaging solver"))` it ends
     `solver_unavailable`: a focus check that cannot save, or whose solver cannot run,
     ends the run by its kind.
3. **Given** a scripted run that is stacking, **Then** a committed test asserts that:
   - after `set_stack(5, &[("not_solved", 2)], false)` and a settle, `subs` is
     `{"stacked":5,"dropped":{"not_solved":2}}` and the message is
     `Stacking M42: 5 stacked, 2 dropped.`;
   - after `set_stack(5, &[("not_solved", 2)], true)` and a settle the state is
     `paused_by_tracker` and the message
     `Exposures paused: 5 stacked so far. They resume by themselves.`,
     and after `set_stack(5, &[("not_solved", 2)], false)` and a settle the state is
     `stacking` again;
   - a run whose `push_focus` gave a `Verdict::SlightlySoft` check with reading 3.6 and
     record 3.1 in arcseconds reaches `stacking` with `warnings` equal to
     `["Focus is slightly soft: stars are 3.6″ across against a record of 3.1″."]` and
     `focus.verdict` `slightly_soft`; and one given reading 6.6 and record 5.6 in
     pixels has the warning
     `Focus is slightly soft: stars are 6.6 px across against a record of 5.6 px.` and
     `focus.unit` `px`.
4. **Given** a script, **Then** a committed test asserts that:
   - with
     `fail_stack_start(Failure::Save("cannot make the stack folder: no space left on device"))`
     the run ends `save_failed` with
     `Ended: the console could not save to its imaging folder: cannot make the stack folder: no space left on device. Check the disk.`,
     never calls `stack.stop`, and has no `folder`;
   - with `fail_stack_start(Failure::Other("the stack is busy"))` it ends `stack_failed`
     with `Ended: the stack stopped: the stack is busy.`, and `calls()` ends
     `stack.start M42`, `camera.release`;
   - with
     `fail_stack_stop(Failure::Other("could not remove subs/000004.fits"))`
     a run stopped while stacking still ends `stopped`, with `ended.saved` false and
     `warnings` `["The stack did not end cleanly: could not remove subs/000004.fits."]`;
   - with `fail_stack_start(Failure::NotSolved("the imaging solver found no solution"))`
     it ends `solver_failing`, and with
     `fail_stack_start(Failure::Camera("the imaging camera is held by a set of darks"))`
     `camera_failed`, and with
     `fail_stack_start(Failure::SolverUnavailable("the imaging solver has no star database it can read"))`
     `solver_unavailable`;
   - a run started with `keep_subs` false and stopped while stacking with nothing
     stacked calls `stack.stop discard` once and has `warnings`
     `["The subs were kept: nothing was stacked."]`, and one stopped after
     `set_stack(2, &[], false)` and a settle has no warning;
   - with `stack_stop` held (`Step::StackStop`), a run that is stacking after
     `set_stack(2, &[], false)` and a settle, whose `runner.stop()` the test spawns,
     reads after a settle the state `stacking` and the message
     `Saving the stack: 2 stacked.`, and a start is refused with 409 and
     `an imaging run is already running`; once `stack_stop` is released the run has
     ended `stopped`; and with `stack_stop` held, after the spawned `runner.stop()` and
     a settle the test calls `set_stack(3, &[], false)`, and once `stack_stop` is
     released the run has ended `stopped` with `subs.stacked` 3: the counts are read
     once more after the stack has stopped.
5. **Given** a script holding `check` and `solve`, **When** a test starts a scripted
   run, settles, sets the mount to tracking and not steady, releases `check` and
   settles, **Then** a committed test asserts the state `solving`, the message
   `Solving an imaging frame. Waiting for the mount to settle; nothing to do.` and one
   `camera.take` in `calls()`; that after the mount is set steady and the test
   settles, the state is `solving` with the message `Solving an imaging frame.`; and
   that once `solve` is released and the test settles, the state is `stacking`.
6. **Given** a script whose mount is tracking and not steady, **When** a test starts a
   scripted run and settles, **Then** a committed test asserts the state `tracking`, the
   message `Tracking M42. Waiting for the mount to settle; nothing to do.` and neither
   `camera.take` nor `focus.check` in `calls()`; and that after the mount is set steady
   and the test settles, the state is `stacking`.
7. **Given** a scripted run held in turn at `slewing` (the mount set to slewing), at
   `first_frame` (`take` held), at `focus_check` (`check` held), at `solving` (`solve`
   held) and at `stacking`, **When** a test awaits `runner.stop()` in each, **Then** a
   committed test asserts for each that the state is `ended`, `ended.reason` is
   `stopped`, `ended.at` is 1768528800.0, and `elapsed_s` no longer rises over 10 s;
   that `ended.message` is `Imaging run stopped. Track is still on.`, with
   `ended.track_on` true, for the four held after `slewing`, and
   `Stopped: the mount was stopped.` for the run held at `slewing`, which called
   `mount.stop` once, before `camera.release`, while no other run called it: a stop
   that arrives before anything was exposed also stops the mount; that `ended.saved`
   is true for the run stopped while stacking and false for the others; that the run
   held at
   `first_frame` called `camera.abort` once; that the run held at `focus_check` called
   `focus.close_aid` once; that the run stopped while stacking called `stack.stop keep`
   once and the others never called `stack.stop`; that each run's last call is
   `camera.release`; that no call is added in the 10 s after the ending; that
   `calls()` never holds `mount.repoint`; and that, once the test has set the mount of
   the run stopped while stacking to `Ended(Ending::Stopped)`, its view's
   `ended.track_on` is false; and that a scripted run held at `first_frame` (`take`
   held), whose mount the test sets to `Ended(Ending::Stopped)` and which it then stops
   with `runner.stop()` with no settle between, ends `stopped` with
   `Stopped: the mount was stopped.` and `ended.track_on` false: a stop of the run that
   finds Track off says that the mount was stopped.
8. **Given** a scripted run that is stacking, **When** a test sets the mount to each of
   `Ended(Ending::Stopped)`, `Ended(Ending::TooLow)`, `Ended(Ending::SunUp)`,
   `Ended(Ending::LinkLost)`, `Ended(Ending::MountFailed("failed after 5 corrections"))`,
   `Ended(Ending::TargetChanged)` and `Ended(Ending::PushedAway)`, in a run of its own
   each, and settles, **Then** a
   committed test asserts `stack.stop keep` then `camera.release` as the last two calls,
   `stack.stop` made once, and `ended.reason` and `ended.message` equal to, in that
   order:
   `stopped`, `Stopped: the mount was stopped.`; `too_low`,
   `Ended: M42 is below the minimum altitude.`; `sun_up`, `Ended: the Sun is up.`;
   `link_lost`,
   `Ended: the mount or the scope link stopped answering. Check the scope node and the mount's cable, then start again.`;
   `mount_failed`,
   `Ended: Track ended and the run with it: failed after 5 corrections. Check the mount, then start again.`;
   `target_changed`,
   `Ended: the target was changed. Start a new imaging run for the new target.`;
   `pushed_away`,
   `Ended: the telescope was pushed away from M42. Track follows where it is left. Start a new imaging run to go back to M42.`
   Each of the seven has `ended.saved` true, and `ended.track_on` false for the first
   five and true for `target_changed` and `pushed_away`, after which Track is still on.
   The committed test also asserts the mount's ending in the two waits before
   stacking: a scripted run held at `first_frame` (`take` held), whose mount the test
   then sets to `Ended(Ending::TooLow)`, has after a settle `ended.reason` `too_low`
   and `camera.abort` called once; and a scripted run started with its mount set to
   slewing, whose mount the test sets to `Ended(Ending::SunUp)` after a settle, has
   after another settle `ended.reason` `sun_up` and `camera.release` as its last call;
   and a scripted run that is stacking, whose mount the test sets to
   `Ended(Ending::TooLow)` and which it then stops with `runner.stop()` with no settle
   between, ends `too_low`: while it stacks, the run judges the mount's ending before a
   stop.
9. **Given** a scripted run that is stacking, built so that the test holds the only
   `Arc` of the `Runner` and no `Steps` of its own, **When** the test drops that `Arc`
   and sleeps 1 s, **Then** a committed test asserts that `Arc::strong_count` of the
   script is 1 and that, 10 s later, `calls()` is what it was before the drop: a
   dropped runner drops its task and lets go of its parts, and asks nothing of them on
   the way. That is what lets a console dropped in the middle of a run still let go of
   Track, the rule 015 keeps at `crates/finder/tests/motion/tracking/routes.rs:456`.
10. **Given** a script, **Then** a committed test asserts that:
    - with `refuse(Part::Camera, "the scope node is not connected")` the start is refused
      with status 409 and those words, `calls()` is exactly `camera.ready`, and `view()`
      is `None`; with `Part::Solver` and words of the test's own, `calls()` is
      `camera.ready`, `solver.ready`; with `Part::Target`, those two and `mount.target`;
      with `refuse(Part::Hold, "the imaging camera is held by the focus aid")`, those
      three and `camera.hold`, and neither `mount.point` nor `camera.release`; with
      `Part::Point`, those four, `mount.point page` and `camera.release`; each refused
      with 409 and the test's words, and each leaving `view()` `None`. The lease is
      asked for after every check that needs no camera and before Track, and a lease
      that was not taken is not given back;
    - a request with `subs` 0 or 100001 is refused with 400 and
      `subs must be a whole number from 1 to 100000`, and one with `duration_s` 0.5,
      43200.5 or not finite with 400 and `duration_s must be a number from 1 to 43200`,
      each with an empty `calls()`;
    - a second start while a run is on is refused with 409 and
      `an imaging run is already running`, and after `runner.stop()` a start is accepted
      and `view()` shows the new run.

---


### User Story 3 - Start, stop and read a run over HTTP, and its record (Priority: P1)

As the observer, I start an imaging run with one request, read where it has got to in
the session, stop it with another, and find afterwards a file that says what the run
did.

**Why this priority**: The page and the MCP tools are both thin callers of these three
routes.

**Independent Test**: `crates/finder/tests/console/imaging_run_routes.rs`, the module
`imaging_run_routes` of `console`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test console imaging_run_routes::`.

**Acceptance Scenarios**:

1. **Given** the scripted console, **When** a test sends `POST /api/imaging/run` with
   `{}` and waits until `run.state` is `stacking` and `run.folder` is present, **Then**
   a committed test asserts that:
   - the answer is 200 and an imaging view whose `run.by` is `page` and whose `run.id`
     begins `2026-01-16T02-` and ends `-M42`;
   - `GET /api/session` carries the same `run` under `imaging`, key for key but for
     `elapsed_s`, and the scripted console's `session.json` holds no `run` key;
   - `run.folder` is the script's stack folder, and R joined with it holds `run.json`
     with `format` `imaging-run/1`, `finder` equal to the `version` of
     `GET /api/health`, `id`, `by`, `target`, `started_at` and `settings` equal to the
     view's, `started_utc` equal to the id's first twenty characters with the two `-`
     of its time made `:`, `state` `stacking`, and `events` whose `state`s are
     `slewing`, `first_frame`, `focus_check`, `solving`, `stacking`, in that order,
     with `at` never falling. The script's mount is steady from the start, so the run
     never shows `tracking`.
2. **Given** the scripted console, **Then** a committed test asserts that:
   - `POST /api/imaging/run` with `{"subs":0}` and `{"duration_s":0}` each answers 400
     with the words of US2-S10, and with `{"subs":1.5}`, `{"duration_s":"long"}` and
     `{"exposure_ms":1}` each answers 400; with a `Content-Type` of `text/plain` it
     answers 415; with a foreign `Origin` 403; each with a JSON `error`, after which
     `GET /api/imaging` has no `run` key and R holds no `runs` folder;
   - with `refuse(Part::Solver, "this console has no imaging solver")` it answers 409
     with exactly that `error`, and R holds no `runs` folder;
   - a second `{}` while a run is on answers 409 and
     `an imaging run is already running`;
   - a console built the same way without `with_imaging_run_steps` answers
     `POST /api/imaging/run` with `{}`, `DELETE /api/imaging/run`, and
     `POST /api/imaging/run/continue` with `{"anyway":true}`, each with 409 and
     `this console runs no imaging runs`.
3. **Given** the scripted console with a run that is stacking, after
   `set_stack(4, &[], false)` and a settle, **When** a test calls
   `set_stack(4, &[], true)`, settles and reads `run.json`, calls
   `set_stack(4, &[], false)` and settles, and then calls `set_stack(5, &[], false)`
   and, with no settle between, sends `DELETE /api/imaging/run`, **Then** a committed
   test asserts that the `run.json` read at the pause has `state` `paused_by_tracker`
   and `paused_by_tracker` as the last of its `events`: a later change of state is
   written when it happens; that the answer of the `DELETE` is 200
   with `run.state` `ended` and `run.ended.reason` `stopped`; that `run.json` then has
   `state` `ended`, an `ended` equal to the view's, `subs.stacked` 5, the count the
   stack held when it was stopped, and `ended` as the last of its `events`; that a
   second `DELETE` answers 200 with the same `run`; and that on a console where no run
   was ever started `DELETE` answers 200 with no `run` key.
4. **Given** the scripted console, **Then** a committed test asserts that
   `POST /api/imaging/run/continue` with `{"anyway":true}` answers 409 and
   `no imaging run is running` before any run, and 409 and
   `the imaging run is not paused` while one is stacking; that `{}`, `{"anyway":"yes"}`
   and `{"anyway":true,"x":1}` answer 400; and that a foreign `Origin` answers 403.
5. **Given** the scripted console with a run that is stacking, **When** a test sets the
   script's mount to `Ended(Ending::LinkLost)`, makes the simulator unresponsive
   (`Simulator::set_unresponsive`, `crates/simulator/src/controls.rs:117`) and waits
   until the imaging view's `state` is `no_link`, **Then** a committed test asserts that
   the view still carries `run`, with `state` `ended` and `ended.reason` `link_lost`.
6. **Given** the scripted console with the script holding `take`, **When** a test reads
   the console's `time` from `GET /api/session`, sleeps the virtual time left to the
   next whole second and 50 ms more, starts a run, waits until its `run.state` is
   `first_frame`, stops it with `DELETE /api/imaging/run`, starts another, waits the
   same way and stops that, and then reads `time` again and finds it in the same whole
   second (the test fails otherwise), **Then** a committed test asserts that the two
   runs have the same `id`; that neither run stacked; that neither view had a `folder`
   before its stop; that after its stop the first run's
   `folder` is `runs/` followed by its `id`; that the second's is the same with `-2`
   added; and that each folder holds a `run.json` whose `ended.reason` is `stopped` and
   whose `events` are `slewing`, `first_frame`, `ended`.
7. **Given** the run of US3-S3 after its stop, **Then** a committed test asserts that
   `run.json` contains none of `test-token`, `scope.local` and `lat_deg`, and has no
   `message`, `elapsed_s` or `site` key.
8. **Given** the scripted console with a run that is stacking, **When** a test calls
   `set_stack(3, &[], false)` and sleeps 61 s, calls `set_stack(4, &[], false)` and
   sleeps 30 s, and then sleeps 31 s more, reading `run.json` after each sleep,
   **Then** a committed test asserts that the three records read `subs.stacked` 3, 3
   and 4, each with `state` `stacking` and the five `events` of US3-S1 and no more:
   while the stacked count rises the record is refreshed, at most once every 60 s, and
   a refresh is no event.
9. **Given** an R in which the test has written
   `runs/2026-01-15T03-00-00Z-M31/run.json` holding
   `{"format":"imaging-run/1","id":"2026-01-15T03-00-00Z-M31","state":"stacking"}`, with
   no `ended`, and an older `runs/2026-01-14T03-00-00Z-M31/run.json` holding
   `{"format":"imaging-run/1","ended":{"reason":"stopped"}}`, and the scripted console
   built over it, **Then** a committed test
   asserts that `GET /api/imaging` carries `unfinished_run`
   `runs/2026-01-15T03-00-00Z-M31` and no `run`; that once a start has been accepted
   the view has no `unfinished_run`; and that on a console built over an R whose only
   record holds an `ended` object the view never has the key.
10. **Given** a scripted console whose R already holds a regular file named `runs`, in a
    run of its own each, **Then** a committed test asserts that:
    - a run started and waited for until it has ended has `ended.reason` `save_failed`,
      an `ended.message` beginning
      `Ended: the console could not save to its imaging folder:`, a `calls()` that ends
      `stack.start M42`, `stack.stop keep`, `camera.release`, and `warnings` holding one
      entry, which begins `The run's record could not be saved:`: the write that failed
      while the run stacked ended it, and the write at the ending failed too;
    - with the script holding `take`, a run stopped with `DELETE /api/imaging/run` once
      its `run.state` is `first_frame` answers 200 with `ended.reason` `stopped`, no
      `folder`, and one warning beginning `The run's record could not be saved:`: a
      folder that cannot be made at the ending is a warning and changes no reason.
11. **Given** the scripted console with a run that is stacking, **When** a test calls
    `set_stack(0, &[], true)` and settles, calls `set_stack(0, &[], false)` and settles,
    calls `set_stack(0, &[], true)`, settles and reads `run.json`, and then sleeps 58 s,
    calls `set_stack(0, &[], false)`, settles and reads `run.json` again, **Then** a
    committed test asserts that `run.state` read `stacking` after the second settle;
    that the first record has the `state` `paused_by_tracker` and `events` that end
    `stacking`, `paused_by_tracker`, with one `paused_by_tracker` among them; and that
    the second record's `events` end `paused_by_tracker`, `stacking`: a change between
    `stacking` and `paused_by_tracker` is listed and written at most once every 60 s,
    and the view shows every one at once; that change comes 61 s after the first was
    listed and 59 s after the last one held back: only a listed change holds the next
    back.

---


### User Story 12 - The run's parts and words, and a script to test them with (Priority: P1)

As the console, I name what an imaging run is made of before any run exists: the five
parts a run is handed, what a start may ask for, the words for where a run is and why it
ended, with a sentence for every ending, and a scripted stand-in for the five parts, so
that the machine and every later story are written against names that do not move.

This story is built first, before User Story 13 and User Story 1 (Work Graph), though
its number comes after those of User Stories 1 to 3. It writes every public
item of `finder::imaging::run` but the view's types and the `Runner`, and `script.rs`
whole; User Story 13 tests the script, and User Story 1 adds the machine after it and
changes none of it. What is made from
a run's own state stays with the machine, because only a run can show it: the view, each
state's message, the focus sentences and the warnings. `RunView` is `#[non_exhaustive]`,
so no test outside the crate can build one.

**Why this priority**: The machine, the console's own parts, the page and the tools are
all written against these traits and words, and every scripted test of this spec runs on
this script, which no later story may write again. Tested by itself, a wrong sentence is
found here, and a script that lists a call late in User Story 13, and not three stories
on.

**Independent Test**: `crates/finder/tests/console/imaging_run_words.rs`, the module
`imaging_run_words` of finder's `console` test binary. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test console imaging_run_words::`.
Its tests make no `Runner`. A test of US12-S1 calls `Ending::message` and
compares its answer with the sentence typed here; it never builds an expected sentence
with it (plan.md § Plan traps 6).

**Acceptance Scenarios**:

1. **Given** `finder::imaging::run`, the name `M42` and the settings
   `RunSettings { subs: Some(3), duration_s: 7200.0, keep_subs: true }`, **Then** a
   committed test asserts that:
   - `STATES` is exactly `slewing`, `tracking`, `first_frame`, `focus_check`,
     `focus_pause`, `solving`, `centring`, `stacking`, `paused_by_tracker`, `ended`, and
     `ENDINGS` exactly `stopped`, `count_limit`, `duration_limit`, `too_low`, `sun_up`,
     `link_lost`, `mount_failed`, `target_changed`, `pushed_away`, `camera_failed`,
     `solver_unavailable`, `solver_failing`, `save_failed`, `focus_timeout`,
     `not_centred`, `subs_failing`, `stack_failed`, each in that order: the page, the
     tools and the README are written from these two lists (FR-042, US11-S1);
   - `reason()` and `message("M42", &settings)` of each ending, a `String` it holds
     being made with `.into()`, are: for `Stopped`, `stopped` and
     `Stopped: the mount was stopped.`; for `RunStopped`, `stopped` and
     `Imaging run stopped. Track is still on.`; for `StackStopped`, `stopped` and
     `Ended: the stack was stopped.`; for `CheckStopped`, `stopped` and
     `Ended: the focus check was stopped.`; for `CountLimit`, `count_limit` and
     `Finished: the 3 subs asked for are stacked.`; for `DurationLimit`,
     `duration_limit` and `Finished: the 120-minute limit was reached.`; for `TooLow`,
     `too_low` and `Ended: M42 is below the minimum altitude.`; for `SunUp`, `sun_up`
     and `Ended: the Sun is up.`; for `LinkLost`, `link_lost` and
     `Ended: the mount or the scope link stopped answering. Check the scope node and the mount's cable, then start again.`;
     for `MountFailed("failed after 5 corrections")`, `mount_failed` and
     `Ended: Track ended and the run with it: failed after 5 corrections. Check the mount, then start again.`;
     for `TargetChanged`, `target_changed` and
     `Ended: the target was changed. Start a new imaging run for the new target.`; for
     `PushedAway`, `pushed_away` and
     `Ended: the telescope was pushed away from M42. Track follows where it is left. Start a new imaging run to go back to M42.`;
     for `CameraFailed("boom")`, `camera_failed` and
     `Ended: the imaging camera failed: boom.`; for
     `SolverUnavailable("no star database")`, `solver_unavailable` and
     `Ended: the imaging solver cannot run: no star database.`; for
     `SolverFailing("no solution")`, `solver_failing` and
     `Ended: 3 imaging frames in a row could not be solved: no solution. Check for cloud, dew and focus.`;
     for `SaveFailed("disk")`, `save_failed` and
     `Ended: the console could not save to its imaging folder: disk. Check the disk.`;
     for `FocusTimeout`, `focus_timeout` and
     `Ended: focus was left paused for 15 minutes. Start again when ready.`; for
     `NotCentred { offset_arcsec: 400.0, measured: true }`, `not_centred` and
     `Ended: M42 is still 400″ from the sensor's centre after 3 tries. Check the aim point on the console page, then start again.`;
     for `NotCentred { offset_arcsec: 400.0, measured: false }`, `not_centred` and
     `Ended: M42 is 400″ from the sensor's centre and no new aim point could be measured.`;
     for `SubsFailing("not solved 7, too few stars 3")`, `subs_failing` and
     `Ended: no sub was stacked for 10 minutes (not solved 7, too few stars 3). Cloud, dew or a tree is the usual cause; check the sky and start again.`;
     and for `StackFailed("the stack is busy")`, `stack_failed` and
     `Ended: the stack stopped: the stack is busy.`;
   - with other settings and offsets: `CountLimit` with `subs` `Some(1)` says
     `Finished: the 1 sub asked for is stacked.`; `DurationLimit` with `duration_s`
     5400.0 says `Finished: the 90-minute limit was reached.`, with 90.0
     `Finished: the 1.5-minute limit was reached.` and with 1.0
     `Finished: the 0.1-minute limit was reached.`; and `NotCentred` with `measured`
     true says `599″` for an `offset_arcsec` of 599.0 where the sentence above says
     `400″`, `10.0′` for 600.0 and `63.2′` for 3792.0;
   - every reason above is an entry of `ENDINGS`, and every entry of `ENDINGS` is the
     reason of at least one ending above; `By::Page.as_str()` is `page` and
     `By::Mcp.as_str()` `mcp`; `as_str()` of `Verdict::Good`, `SlightlySoft`,
     `ClearlyOut`, `Unmeasurable` and `NoRecord` is `good`, `slightly_soft`,
     `clearly_out`, `unmeasurable` and `no_record`; `Unit::Arcsec.as_str()` is `arcsec`
     and `Unit::Px.as_str()` `px`; and `Failure::Stopped.words()` is `stopped` while
     `Failure::Camera("boom".into()).words()` is `boom`.
2. **Given** `finder::imaging::run`, **Then** a committed test asserts that:
   - `label("M42")` is `M42`, `label("NGC 1976")` is `NGC-1976`,
     `run_id(1768528800.0, "M42")` is `2026-01-16T02-00-00Z-M42` and
     `run_id(1768528800.0, "NGC 1976")` is `2026-01-16T02-00-00Z-NGC-1976` (a worked
     example of 035's rule, which `label` calls: every character that is not an ASCII
     letter or digit becomes `-`);
   - `serde_json::from_str::<RunRequest>` of `{}` is `RunRequest::default()`, whose
     three fields are `None`; of `{"subs":3,"duration_s":60.0,"keep_subs":false}` it
     holds those three values; and of `{"exposure_ms":500}` it is an error: a key the
     request does not know is refused;
   - `serde_json::to_string` of
     `RunSettings { subs: None, duration_s: 7200.0, keep_subs: true }` is
     `{"subs":null,"duration_s":7200.0,"keep_subs":true}`, and of
     `RunTarget { id: "NGC 1976".into(), name: "M42".into(), ra_deg: 83.82, dec_deg: -5.39 }`
     it is `{"id":"NGC 1976","name":"M42","ra_deg":83.82,"dec_deg":-5.39}`;
   - `POLL` is 250 ms, `DEFAULT_DURATION_S` 7200.0, `MAX_DURATION_S` 43200.0 and
     `MAX_SUBS` 100000.

---


### User Story 13 - The script's defaults and controls, tested with no run (Priority: P1)

As the factory that builds User Story 12, I test in a story of their own what that
story's script answers with no run: every default and call word in its order, and every
control, so that each later scripted test stands on a script that has been proven.

This story is built after User Story 12 and before User Story 1 (Work Graph). Its two
scenarios test User Story 12's script: this story writes nothing under
`crates/finder/src`, and on User Story 12's code its scenarios already pass (tasks.md,
US13's test tasks).

**Why this priority**: Every scripted test of this spec, of 041 and of 042 runs on this
script, which no story after User Story 12 may write again. A script that lists a call
late or answers a control wrongly is found here, before the machine is written on it.
It is P1, as User Story 12 is.

**Independent Test**: `crates/finder/tests/console/imaging_run_script.rs`, the module
`imaging_run_script` of finder's `console` test binary. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test console imaging_run_script::`.
Its tests make no `Runner`, and those that await a part run under
`#[tokio::test(start_paused = true)]`.

**Acceptance Scenarios**:

1. **Given** a new `Script` and the `Steps` its `steps()` answers, **When** a test makes
   these calls of the five parts in this order, awaiting each that answers a future:
   `camera.ready()`, `solver.ready()`, `mount.target()`, `camera.hold()`,
   `mount.point(By::Page)`, `camera.take()` twice, `camera.abort()`, `camera.release()`,
   `focus.check()`, `focus.open_aid()`, `focus.close_aid()`, `solver.solve` of the
   second frame and the target, `mount.repoint()`, `mount.stop()`, `stack.start` of the
   target, `stack.stop(true)`, `mount.point(By::Mcp)` and `stack.stop(false)`, **Then**
   a committed test asserts that:
   - each answers its default: `Ok(())` for the two `ready`, for `hold`, both `point`,
     `open_aid`, `repoint` and both `stack.stop`; the target
     `RunTarget { id: "NGC 1976".into(), name: "M42".into(), ra_deg: 83.82, dec_deg: -5.39 }`;
     the takes `FrameRef { id: 1, file: "frames/2026-01-16/000001.fits".into() }` and
     the same with 2 and `000002.fits`; the check
     `FocusCheck { verdict: Verdict::Good, reading: Some(3.0), record: Some(3.0), unit: Unit::Arcsec }`;
     the solve
     `Centre { offset_arcsec: 0.0, tolerance_arcsec: 90.0, near_arcsec: 450.0, aim_moved: true }`;
     and `stack.start` `Ok` of `STACK_FOLDER`, which is `runs/2026-01-16T02-00-10Z-M42`;
   - `calls()` is then exactly `camera.ready`, `solver.ready`, `mount.target`,
     `camera.hold`, `mount.point page`, `camera.take`, `camera.take`, `camera.abort`,
     `camera.release`, `focus.check`, `focus.open_aid`, `focus.close_aid`,
     `solver.solve 2`, `mount.repoint`, `mount.stop`, `stack.start M42`,
     `stack.stop keep`, `mount.point mcp`, `stack.stop discard`: one string a call, in
     the order the calls were made;
   - `mount.state()` is `MountState::Tracking { steady: true }`, `mount.track_on()` true
     and `mount.steady_between(0.0, 10.0)` true, read after the last call, and none of
     the three, nor `stack.progress()`, adds to `calls()`;
   - `stack.progress()` has `running` false before `stack.start`, true after it and
     false again after `stack.stop(true)`, with `stacked` 0, `dropped` empty,
     `waiting_for_tracker` false and `failed` `None` each time.
2. **Given** a new `Script` for each bullet, its parts called as in US13-S1, **Then** a
   committed test asserts that:
   - after `push_frame(Err(Failure::Camera("boom".into())))` and then
     `push_frame(Ok(FrameRef { id: 9, file: "x.fits".into() }))`, three takes answer
     that `Err`, that `Ok` and then the frame with the id 1: a pushed answer is given as
     it was pushed, in the order pushed, and uses no number; and one
     `push_focus(Err(Failure::Other("no".into())))` and one
     `push_solve(Err(Failure::NotSolved("no".into())))` are each answered once, the next
     check and the next solve being the defaults;
   - `fail_repoint`, `fail_open_aid`, `fail_stack_start` and `fail_stack_stop`, each
     given `Failure::Other("no".into())`, make the next `mount.repoint`,
     `focus.open_aid`, `stack.start` and `stack.stop` answer `Err` of it and the one
     after that `Ok`; after the failed `stack.start`, `running` is false;
   - `refuse(Part::Camera, "a")`, `refuse(Part::Solver, "b")`,
     `refuse(Part::Target, "c")`, `refuse(Part::Hold, "d")` and
     `refuse(Part::Point, "e")` make `camera.ready`, `solver.ready`, `mount.target`,
     `camera.hold` and `mount.point` answer `Err` of their words, the second time they
     are asked as the first, and every refused call is in `calls()`;
   - after `unsteady(2)`, `mount.steady_between(0.0, 10.0)` answers false, false and
     then true;
   - `set_mount(MountState::Slewing)` makes `mount.state()` answer it;
     `mount.track_on()` is false after `set_mount` of each of `Ended(Ending::Stopped)`,
     `Ended(Ending::TooLow)`, `Ended(Ending::SunUp)`, `Ended(Ending::LinkLost)` and
     `Ended(Ending::MountFailed("x".into()))`, and true after `Slewing`,
     `Tracking { steady: false }`, `Ended(Ending::TargetChanged)` and
     `Ended(Ending::PushedAway)`;
   - on a script whose stack was started,
     `set_stack(5, &[("not_solved", 2), ("behind", 0)], true)` makes `stack.progress()`
     hold `stacked` 5, `dropped` with the one key `not_solved` and its 2, and
     `waiting_for_tracker` true, with `running` still true and `failed` `None`;
     `end_stack(Some(Failure::Camera("gone".into())))` then makes `running` false and
     `failed` that failure, the counts unchanged;
   - with `hold(Step::Take)`, a `camera.take()` the test spawns is the last of `calls()`
     1 s later and has not answered 10 s after that, while a `focus.check()` the test
     awaits meanwhile answers; 1 s after `release(Step::Take)` the take has answered the
     frame with the id 1, and another `camera.take()` answers with no release: the hold
     has ended. `hold(Step::StackStop)` and `hold(Step::CloseAid)` hold `stack.stop` and
     `focus.close_aid` in the same way, each listed first and answering once released.

---


### Edge Cases

- **The console exits mid-run.** Nothing resumes at the next start: a run is run-only,
  like Track. A run that was stacking leaves its folder with `run.json` as last
  written and no `ended`, 035's `stack.json` as written at the start, and every sub
  taken, whatever `keep_subs` said. The stacked picture itself is lost: 035 writes it
  only when the stack ends. The subs can be stacked in another program. `run.json` is
  refreshed at most every 60 s while the stacked count rises, so it says how far the
  run got (FR-017), and after the restart the Imaging page says that the last imaging
  run did not finish and where its subs are, until a new run is started (FR-016,
  042's FR-035). A run that had not yet stacked leaves no record. The scope node's watchdog
  stops the mount when the console is gone, as for any motion.
- **Another taker gets the lease in the instant of a hand-over.** The run's next step
  is refused with that holder's name. A focus check that is refused pauses the run, as
  one that could not finish; a stack that is refused, or that turns out to be one
  somebody else started, ends it `stack_failed` (US2-S4), and nothing of that other
  stack is stopped or removed; a
  lease that cannot be taken back after the focus gate ends it `camera_failed`
  (US1-S2).
- **Somebody aborts an exposure during a run** (031's route, tool or button, open to
  anyone). The run's own imaging frame is counted as a failed take and taken again
  (FR-009). A sub that is aborted ends 035's stack as `stopped`, so the run ends
  `stopped` and its stack is saved (041's FR-032).
- **The scope link drops for a moment.** A take of the run's own is refused at once
  while the camera is not ready. The run asks again 5 s later, and again 5 s after
  that, so a link back within 10 s costs nothing; three refusals end it
  `camera_failed` (FR-009). While it stacks, a lost link ends Track as `link_lost` and
  035's stack as a camera failure; the run judges the mount's ending first, and reads
  `camera_failed` only when the stack saw it a poll before Track did (041's FR-032, US8-S6).
- **The run's own stop arrives while it is re-centring.** The re-centre is finished
  first, within about 7 s, and the run then ends `stopped` with Track on, as after any
  stop of the run once it has exposed (FR-005, US5-S3; owner decision 14).
- **The run's own stop arrives while it is still slewing.** Nothing has been exposed
  yet, and the only thing the run has done is start that motion, so the run asks for
  Stop too and ends `stopped`, saying `Stopped: the mount was stopped.` (FR-005,
  US2-S7, 041's US4-S4; owner decision 14).
- **The console is dropped in the middle of a run** (a test does this; the binary's
  state lives until it exits). The runner's task is aborted, nothing more is asked of
  any part, and Track is let go as 015 has it (FR-007, US2-S9).
- **The tracker is never steady.** The run stays in `tracking`, or in
  `paused_by_tracker` once it stacks, until the observer stops it, a guard ends the
  motion or, while it stacks, the time limit is reached. The run has no timeout of its
  own for an unsteady tracker: on a later mount a meridian flip is a long not-steady
  stretch (rule 1). The time limit is another matter: it counts the clock from the
  start of stacking, pauses included, because its job is to end a forgotten run
  (041's FR-030, US7-S2, US7-S8).
- **Slightly soft focus.** The run goes on and keeps the warning in `run.warnings` and
  `run.json` (US2-S3).
- **The focus check cannot finish** (no tile of the imaging frame had five stars, the
  camera failed under it). The run pauses and says why in a warning. When no tile had
  stars enough, the pause reads as for unmeasurable; otherwise it says that the focus
  check did not finish, why, and to check the imaging camera (US1-S2, 041's US6-S10).
- **The tracker pauses the stack again and again** (gusts, a pass near the zenith). The
  view shows every change between `stacking` and `paused_by_tracker` at once; `run.json`
  lists and writes one at most every 60 s, so a long night does not grow the record with
  every pause (FR-017, US3-S11).
- **The run's record cannot be written.** While the run is on, that ends it
  `save_failed`. At the ending it is a warning, and the run keeps the reason it ended
  with; a folder that could not be made leaves the view without a `folder` (FR-017,
  US3-S10).
- **Two runs in one second on one target that both end before stacking.** The second
  folder's name ends `-2` (US3-S6).
- **A camera whose exposure is minutes long** (a later equatorial mount). Nothing here
  waits a fixed time for an imaging frame; a stop abandons the run's own exposure with
  031's abort (FR-005).
- **An exposure setting longer than the tracker's limit, on an alt-az mount.** The
  stack's subs are capped by 036; the run's own imaging frames are not, so their
  corners smear and their centre does not once the target is centred. The solve's
  centre is all the run reads; a solve that fails for lack of stars is counted
  (US2-S2). A known limit, which the field checklist has the observer look for (042's FR-041).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The console SHALL carry out an imaging run through five parts it is handed,
  each a trait in `finder::imaging::run`: the mount, the camera, the focus check, the
  solver and the stack. The run SHALL reach the mount through the mount part only,
  whose calls are: say the target, ask Track for it, ask Track for it again, and, for
  a run stopped while it still slews, ask for Stop. It reads three answers back: the
  state of the motion, whether Track is on, and whether the tracker was steady over a
  past window.
  Neither `crates/finder/src/imaging/run.rs` nor any file under
  `crates/finder/src/imaging/run/` SHALL send an axis command, read an axis position,
  compute an altitude, an azimuth, a field rotation or a rate, or hold a number sized
  to one telescope (a focal length, a field size or a scale) (SC-006, US8-S9).
- **FR-002**: A start SHALL be refused, in this order and with nothing asked of a later
  part: with 400 for a `subs` outside 1 to 100000 or a `duration_s` outside 1 to 43200;
  with 409 and `an imaging run is already running` while one is on; with 409 and the
  part's own words when the camera or the solver says it is not ready, in that order;
  with 409 and the mount part's words when there is no target; with 409 and the camera
  part's words when it cannot take the lease of the imaging camera; and with 409 and
  Track's own words when Track refuses, in which case the lease SHALL be given back
  first. A refused start SHALL leave the earlier run's view, or none, in place. A
  started run SHALL give the lease up just before the first focus check, leaving it
  given up through a focus pause and every check that is repeated, and just before it
  starts the stack, SHALL take it again when it goes on from the focus gate to the
  solve, and SHALL end `camera_failed` with the camera part's words when it cannot.
- **FR-003**: An accepted start SHALL go through these states, each shown by its word:
  `slewing` while the mount part says a GoTo is running; `tracking` while it tracks but
  is not steady; `first_frame` while the run's first imaging frame is taken;
  `focus_check` while the focus part checks focus; `solving` while an imaging frame is
  taken and solved; and `stacking` once the stack is asked to start. The run SHALL
  start a take of its own, or a focus check, only while the mount is tracking and
  steady; it does not interrupt a focus check or the focus aid for the tracker, but
  it asks afterwards whether the tracker was steady during a check that would pause
  the run (041's FR-026). While it waits for the tracker in any state after `tracking`, its
  message SHALL say so. The two word lists are FR-042's.
- **FR-004**: The run's view SHALL hold `id`, `state`, `by` (`page` or `mcp`), `target`
  (`id`, `name`, `ra_deg`, `dec_deg`), `started_at` (console Unix seconds), `elapsed_s`
  (rising while the run is on, fixed once it has ended), `settings` (`subs` or null,
  `duration_s`, `keep_subs`), `subs` (`stacked` and `dropped` by reason), `focus`
  (`verdict`, `reading`, `record`, `unit`) and `centring` (`tries`, `offset_arcsec`,
  `tolerance_arcsec`) once known, `stacking_since` and `limit_at` (console Unix seconds:
  when the stack started, and when the time limit will end the run) once the stack
  has started, `pause_ends_at` (console Unix seconds) while in `focus_pause`,
  `message`, `warnings` (left out while empty), `ended`
  (`reason`, `at`, `message`, `saved`, `track_on`) once ended, and `folder` once the
  run has one. `ended.saved` SHALL be true only when the run started a stack and the
  stack part stopped and saved it. `ended.track_on` SHALL be whether Track was on at
  the ending, and SHALL read false from the moment Track is found off afterwards. The `id`
  SHALL be the start's UTC as `YYYY-MM-DDTHH-MM-SSZ`, a `-`, and the target's name with
  every character that is not an ASCII letter or digit made `-`, by 035's one function
  for that rule, which `run::label` calls. Every message SHALL be worded as plan.md
  § The run's words gives it.
- **FR-005**: `Runner::stop` SHALL end a run that is on as `stopped`, whatever its state,
  and answer once it has ended; with none on it SHALL answer the latest view. Every wait
  of the run SHALL give way to a stop and to an ending of the mount's motion within one
  poll of 250 ms; a take that is given up SHALL be aborted through the camera part, and
  a focus check that is given up SHALL be stopped through the focus part. A stop that
  arrives while the run is still `slewing` SHALL also ask the mount part for Stop,
  and the run SHALL then say that the mount was stopped (owner decision 14). A stop
  that arrives while a start is still asking Track SHALL act on the run that start
  makes. Six calls
  are the exception, because abandoning one half-way would leave Track off, or a stack
  or a focus aid running, with nobody owning it: starting and stopping the stack,
  opening and closing the focus aid, asking Track again, and that Stop. The run SHALL
  wait for each to answer and act on a stop as soon as it has. Each is bounded by the
  part behind it; asking Track again takes at most the 5 s of 041's FR-025, Track's own wait
  of up to 2 s for the axes to rest, and the mount's acknowledgements of one Track off
  and one Track on: about 7 s; and starting the stack includes writing the reserve of
  041's FR-033, which takes as long as the disk needs for that file.
- **FR-006**: When the mount part reports that its motion ended, the run SHALL end with
  the reason it gives: `stopped`, `too_low`, `sun_up`, `link_lost`, `target_changed`,
  `pushed_away`, or `mount_failed` with the mount's words. At every ending the run SHALL stop a focus
  check it gave up, stop the stack if it started it, which saves it, and give the lease
  back whether or not it still holds it, in that order, and SHALL ask nothing of the
  mount but the Stop of FR-005 (owner decision 1).
- **FR-007**: A `Runner` that is dropped SHALL abort its run's task, and the task SHALL
  hold nothing that keeps the `Runner` alive. An aborted task SHALL ask nothing more of
  any part and SHALL let go of every part it holds, so that a console dropped in the
  middle of a run lets go of Track as it does without one.
- **FR-008**: `finder::imaging::run::script` SHALL offer `Script`, a stand-in for all
  five parts with the default answers, controls, holds and call list of plan.md § The
  script. Its stack SHALL run from an accepted start until it is stopped or ended by
  `end_stack`, whatever counts `set_stack` gives it (owner approval 5; US13-S1,
  US13-S2).
- **FR-009**: A take that the camera part fails SHALL be asked again 5 s later, a wait
  that gives way to a stop and to the mount's ending; three failures in a
  row SHALL end the run `camera_failed` with the last error, and a failure to save SHALL
  end it `save_failed` at once (owner decision 9).
- **FR-010**: A solve that finds no solution SHALL be followed by a new imaging frame and
  another solve, without another focus check; three in a row SHALL end the run
  `solver_failing`, and a solver that cannot run SHALL end it `solver_unavailable` at
  once.
- **FR-011**: While stacking the view's `subs` SHALL be the stack part's counts as last
  read, and the state SHALL read `paused_by_tracker` while the stack says it is waiting
  for the tracker. A focus verdict of slightly soft SHALL add a warning and go on.
  From the moment an ending of a run that started its stack is decided until the run
  has ended, its message SHALL read `Saving the stack: {stacked} stacked.`, the state
  word staying as it was.
- **FR-012**: A stack that cannot start SHALL end the run `camera_failed`, `save_failed`,
  `solver_unavailable` or `solver_failing` by the failure's kind, and `stack_failed`
  otherwise, without the stack being stopped. A stack that does not end cleanly at an
  ending SHALL add a warning and leave the ending's reason as it is. The counts SHALL
  be read once more after the stack has stopped, so that the view, the ending and the
  record hold the stack's last counts; and with `keep_subs` false and nothing stacked
  the run SHALL add the warning `The subs were kept: nothing was stacked.`
- **FR-013**: `POST /api/imaging/run` SHALL accept a JSON object with the optional keys
  `subs`, `duration_s` and `keep_subs` and no other, under the same-origin rule, and
  start a run by `page` through `start_motion` with the tool name `imaging_run`. It
  SHALL answer 200 with the imaging view; 403 for a foreign origin; 415 for a body that
  is not JSON; 400 for JSON of another shape or FR-002's values; 409 with
  `this console runs no imaging runs` on a console given no parts; and FR-002's 409s
  (owner approval 1).
- **FR-014**: `DELETE /api/imaging/run` SHALL, under the same-origin rule, end the run
  that is on as `stopped`, wait until it has ended, and answer 200 with the imaging
  view; with no run on it SHALL answer 200 and change nothing; on a console given no
  parts it SHALL answer 409 with `this console runs no imaging runs`. It SHALL not ask
  for Stop itself; the run asks for Stop only when it is stopped while still `slewing`
  (FR-005,
  owner decision 14).
- **FR-015**: `POST /api/imaging/run/continue` SHALL accept exactly `{"anyway":true}` or
  `{"anyway":false}` under the same-origin rule and answer 200 with the imaging view,
  409 with `no imaging run is running` when none is on, and 409 with
  `the imaging run is not paused` when the run is not in `focus_pause`, or when its
  pause has already been answered (041's FR-027), and 409 with
  `this console runs no imaging runs` on a console given no parts.
- **FR-016**: The imaging view SHALL carry `run` from the first accepted start until the
  console exits, in every state of the view, `no_link` and `error` included, and so
  SHALL `imaging` in `GET /api/session` and in the MCP tools that return the session.
  Nothing about a run SHALL be written to `session.json` (owner approval 2). From its
  start until the first accepted start of a run, a console whose imaging folder's
  newest `runs/*/run.json` holds no `ended` SHALL carry that record's folder, relative
  to the imaging folder, as `unfinished_run` in the imaging view; it reads the folder
  once, when it is given its parts.
- **FR-017**: A run SHALL keep `run.json`: in the stack's folder from the moment the
  stack has started, and, for a run that ends before that, in `runs/<id>/` under the
  imaging folder, made at the ending, with `-2`, `-3` and so on added when the name is
  taken. It SHALL hold `format` `imaging-run/1`, `finder` (the console's version), `id`,
  `by`, `target`, `started_at`, `started_utc`, `settings`, `state`, `events` (one
  `{at, state}` for each state entered since the start, `ended` last, but that a
  change between `stacking` and `paused_by_tracker` is listed at most once every
  60 s), and `subs`,
  `focus`, `centring`, `warnings` and `ended` as the view has them. It SHALL be
  rewritten whole, under a `.part` name and renamed, at every change of state it
  lists once the run has a folder, at most once every 60 s while the stacked count has
  risen since
  the last write, and at the ending. It SHALL hold no site, node address or token. A
  write that fails while the run is on SHALL end the run `save_failed`; one that fails
  at the ending SHALL add a warning (owner approval 4).
- **FR-018**: `AppState::with_imaging_run_steps` SHALL give the console the parts of its
  runs and attach the run's view to the imaging view, and `crates/finder/src/lib.rs`
  SHALL list the three routes.
- **FR-042**: `finder::imaging::run` SHALL offer, before any `Runner` exists and
  unchanged by every later story: the five parts' traits with every type they exchange,
  and `Steps`; the request and its settings; `STATES`, the ten state words, and
  `ENDINGS`, the seventeen ending reasons, each in its order; for every ending its
  reason, which is one of `ENDINGS`, and its sentence, worded as plan.md § The run's
  words gives it; and `label` and `run_id` (US12-S1, US12-S2).


### Key Entities

- **Imaging run**: the console's whole procedure from a chosen target to a saved stack
  (`GLOSSARY.md`).
- **Part**: one of the five things a run asks: the mount, the camera, the focus check,
  the solver, the stack.
- **State**: where a run is, one of ten words.
- **Ending**: why a run is over, one of seventeen reasons, with a message for the
  observer.
- **The run's record**: `run.json`, in the stack's folder or in `runs/<run id>/` under
  the imaging folder.
- **Lease**: 031's one hold on the imaging camera (031 US9), which a run takes for its
  own imaging frames and hands over to the focus check and the stack (the plan
  document, amendment 1).
- **Set point**: where on the sensor the run wants the target, today its centre (036's
  glossary term).


## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario has a committed test that asserts it, and the
  `lint` and `test` gates pass.
- **SC-002**: No existing test, golden, `scripts/ci/` file, `.github/` file, `CODEOWNERS`
  entry, `ergane.yaml` or `playwright.config.ts` is edited. The only edits to existing
  files under `crates/finder/tests` are one `mod` line for each new test module, in
  alphabetical order, in its binary's `main.rs`: `imaging_run`, `imaging_run_routes`,
  `imaging_run_script`, `imaging_run_steps` and `imaging_run_words` in
  `crates/finder/tests/console/main.rs`.
  No other line of that file and no other existing test changes.
- **SC-003**: No `Cargo.toml`, `Cargo.lock` or `deny.toml` changes, no scope-link message,
  config key, console flag or glossary term is added, and no `unsafe` is written.
- **SC-006**: No file of this spec under `crates/finder/src/imaging/` holds an
  identifier, an import or arithmetic for an axis, an altitude, an azimuth, a rate or
  axis counts, and none holds a focal length, a field size or a scale as a number. The
  rule is about code: the words an observer reads and the doc comments are exempt, so
  the `too_low` message may name the minimum altitude and a comment may say that
  nothing here knows an axis (FR-001; plan.md § Plan traps 18). 041's US8-S9 reads the
  four files for what they import.
- **SC-007**: Every story's diff, measured as plan.md § Plan traps 3 says, is under
  64 KiB.


## Assumptions

- **Earlier features are in place.** Every spec this one names in `depends_on_landed` has
  landed (flip.md § Prerequisites 1), and the trio is read against their code. It was
  re-anchored to that code on 2026-11-20 (flip.md § Prerequisites 2 and 7): every
  `path:line` here was read by its content on `main` at `6d79292b` and carried across 043
  to `2ae73c66`, whose line numbers are cited; those the draft held were first read at 49bb210.
- **Out of scope:**
  - an equatorial mount, meridian flips, dithering and any tracker but 015's as 036
    leaves it; the run is written so that none of them needs a change here;
  - plans of several targets, schedules, and a run that resumes after the console
    restarts;
  - autofocus, and any change to 033's verdict or its thresholds;
  - choosing the exposure, the gain or the binning: they are the imaging camera's
    settings (031), capped by 036 for a stack's subs only; capping the run's own
    imaging frames too, which needs a getter 036 does not offer;
  - any change to GoTo, Track, Stop or their guards, and any new mount command; what
    the tracker does with a newly measured aim point is 032's and 015's, and the owner
    settled it as 032's owner decision 25: 032 holds it back while Track is on (owner
    decision 26);
  - the lease itself, and how 033 to 035 take it: they are 031's and theirs;
  - a free-space check (owner decision 7);
  - a `target` argument (owner decision 10);
  - listing, fetching or deleting runs through the API, and returning the stacked
    picture from a run tool: the picture is 035's `get_stack`;
  - MCP on the scope node.
- **Outside the factory:** rebuilding the console, and the field checks at the telescope,
  which follow the checklist of 042's FR-041 (flip.md § Field checks).
- **The factory's gates and CI:** the gates are `lint` and `test`, copied in plan.md
  § Verification. CI's other jobs run only in the merge queue, so no scenario's Then
  clause rests on them.


## Work Graph

```yaml
US1:
  depends_on: []
  depends_on_merged: [US12, US13]
  implements: [FR-001, FR-002, FR-003, FR-004, FR-005, FR-006, FR-007]
US2:
  depends_on: []
  depends_on_merged: [US1]
  implements: [FR-009, FR-010, FR-011, FR-012]
US3:
  depends_on: []
  depends_on_merged: [US1, US2]
  implements: [FR-013, FR-014, FR-015, FR-016, FR-017, FR-018]
US12:
  depends_on: []
  implements: [FR-008, FR-042]
US13:
  depends_on: []
  depends_on_merged: [US12]
  implements: []
```

Every edge is a merge edge, and the stories run one after another. An edge names each
earlier story of this spec that last wrote a file the story writes next, so that the
later story builds on the merged file.

- **US12** comes first and names no story: it writes the parts' traits, the words and
  the script. Its number comes after US1 to US3, and every other story kept its number
  when it was made.
- **US13** tests US12's script: it writes nothing under `crates/finder/src` and
  implements no FR. It follows US12, whose code it tests, and adds its `mod` line to
  `crates/finder/tests/console/main.rs` after US12's.
- **US1** adds the view and the machine to the `crates/finder/src/imaging/run.rs` that
  US12 creates, and its `mod` line to `crates/finder/tests/console/main.rs` after
  US13's, so it names both; the script is proven before the machine is written on it.
- **US2** grows US1's machine in `crates/finder/src/imaging/run.rs`: the retries, the
  table of failed steps, the stack's counts and the warning for soft focus.
- **US3** serves that `Runner` and adds the record to its machine. It names US2 for
  `run.rs` and `crates/finder/tests/console/main.rs`, and US1 as well; US12 writes
  `crates/finder/src/imaging/mod.rs` before it, and US3 follows US12 through US1.

The stories after US3 are 041's US4 to US8, in `041-imaging-run-on-the-rig`, and 042's
US9 to 042's US11, in `042-imaging-run-page-and-mcp`, which build on this spec once it has
landed.
