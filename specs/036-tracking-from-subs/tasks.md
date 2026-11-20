---
description: "Task list for 036-tracking-from-subs: Tracking Steered by the Subs"
---

# Tasks: Tracking Steered by the Subs

**Prerequisites** (plan.md § Prerequisites):
- `015-tracking` and `038-track-follows-a-push` have landed on `main`: met;
- `030-imaging-camera-link`, `031-imaging-frames-on-console`, `032-imaging-solve-and-aim-point`,
  `033-focus-reading-and-aid`, `034-dark-and-flat-frames` and `035-live-stack` have
  landed, 031 with the camera's lease: not met;
- this trio has been re-anchored against `main` after they landed, each point of
  plan.md § What 035 must provide read in the landed code, and US7's calls placed
  against 035's taker, its returns and its worker by name and line: not met;
- the owner decisions in spec.md are settled and its owner approvals given, and the
  plan document agrees with them (plan.md § Prerequisites 9; in this worktree it
  carries the recommended answers, and it is not on `main`): not met;
- a probe build has confirmed the bounds of US3-S1, US3-S2, US4-S5, US6-S5 and US7-S1,
  and that US3's test file passes unedited once US6 has landed: met on 2026-11-19 by
  probe round 1 for the trio as it then stood, and not met for the trio as the
  reanalysis of that day left it, which no build has run: a second probe round builds
  all eleven stories (plan.md § Prerequisites 5);
- the documents this trio cites are on `main`: not met;
- 032's owner decision on a newly measured aim point while Track is on is settled,
  and, if its answer is that the tracker bounds the rate one tracking error may ask
  for, US3 carries that bound with one scenario (plan.md § Prerequisites 11): met on
  2026-11-17; the owner chose to hold the aim point, so no bound is added;
- 038's `follow` ends any claim the subs hold and forgets what was pending or held, with
  one scenario in US3 (plan.md § Prerequisites 12): met on 2026-11-19, as the scenario
  that is now US9-S12, and completed by the reanalysis with US9-S13 and US6-S6;
- no operator-staged file is needed.

**Rules for every story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml`,
  `CODEOWNERS`, `ergane.yaml`, `.specify/` or `crates/finder/web/playwright.config.ts`.
  If an existing test fails, stop and escalate with its exact output. Do not edit it.
  The one exception is the `mod` line of each new finder test module, added in
  alphabetical order to the binary's `main.rs` your story's tasks name, which holds no
  test (owner approval 9). Change nothing else there.
- New tests go in new files: the ones your story's tasks name. A new finder test is a
  module file in the binary's folder plan.md § Technical Context names, never a file at
  the top level of `crates/finder/tests` or in `crates/finder/tests/support`. A module
  writes its own setup, only the parts its own tests use, and never edits an earlier
  story's test file: the tracker rig and the scripted tracker of spec.md § User
  Scenarios are written out again where they are needed. It may use what the existing
  support files already offer.
- Write only the files your story's tasks name; they are its `writes` in plan.md
  § Refinement Record. A story that needs another file escalates.
- Tests compute their expected values themselves (plan.md § Plan traps 7): from the
  simulator's truth and transforms, the worked examples of spec.md, 015's
  `ephemeris::longest_exposure`, and a wrapper of the test's own round the mount. No
  test reads an expected value from the code under test.
- Every rate goes to the mount from the tracker's own rate task (plan.md § Plan traps
  1). No story sends a rate from `gap`, from `Steering` or from the stack.
- While a stack holds the tracker, a finder pointing changes no rate when it comes,
  whether the subs steer or have lapsed, and after a lapse it changes the place only, as
  a step in the next gap, never the tracking offset; only while the tracker is not
  steady, or with no stack, does it correct as 015 has it (plan.md § Plan traps 27 and
  28).
- A tracker rig waits for the tracker's first rates, and 3.5 s more, before any scenario
  asks it for a gap, in every module that writes the rig out (spec.md § User Scenarios,
  The tracker rig; plan.md § Plan traps 22). The scenarios that say "built without the
  wait" are the only ones that skip it.
- The stack changes the camera's exposure only through the lease it already holds,
  with `Lease::set`; never through `Imaging::set`, which that lease refuses (034's
  `Calibration::restore_exposure`, which went through it, is gone; plan.md § Plan traps 23).
- No file under `crates/finder/src/imaging/` computes an altitude, an azimuth, a
  parallactic angle or a field rotation, reads an axis count, or names the finder camera
  in a view's words. `steering.rs` and 035's `stack/service.rs` hold none of the
  thirteen names spec.md US2-S6 lists, in code or in a comment: a pause's reason is the
  tracker's word, written in `tracking.rs` and on the page only (plan.md § Plan traps
  38).
- `Steering` keeps only `Weak` references to a tracker (plan.md § Plan traps 15), and
  holds its lock across no await and across no call to its source, a tracker or
  `Imaging` (plan.md § Plan traps 14 and 26; § The steering, **Lock order**).
- Keep the story's diff under 64 KiB, measured as plan.md § Plan traps 10 says. If the
  finished story measures over it, do not drop or shorten a case to get under: say its
  size when you hand off.
- A test asserts nothing a later story changes, and no story edits an earlier story's
  test file. US9 and US10 are stories of tests only: they write nothing under
  `crates/finder/src`, their tests pass on the code of US3 and US6 as soon as they are
  written, and one that fails is escalated with its output (plan.md § Plan traps 36).
- Every code citation is `path:line` as read at 49bb210. If a cited line has moved,
  re-read it rather than guess.
- Add no dependency and change no `Cargo.toml`. Change nothing under
  `crates/scope-node`, `crates/scope-link` or `proto/`, and outside US1 nothing under
  `crates/simulator`.
- Do not edit `specs/036-tracking-from-subs/`.
- Run no tool the gates don't run: no `cargo deny`, `cargo machete`, Rust 1.96 or
  `ergane`.

## Phase 1: User Story 1 — The simulator remembers where the telescope pointed (Priority: P1)

### Tests first

- [ ] T001 [US1] Create `crates/simulator/tests/imaging_history.rs` with a simulator at
  40° N 75° W on a `VirtualClock` from 1768528800 and a `SimMount::with_config` with no
  backlash and no base tilt, `set_rate_error(0.25)` called before the azimuth rate is
  commanded, and failing tests for the truth of three instants of an exposure asked 30 s
  later, against `sim.telescope().eyepiece` read live, the azimuth's rise and the star's
  trail, measured over the window US1-S1 gives round the midpoint of the star's two
  places (US1-S1), and for the instant between two samples against a twin read live,
  with both axes turning and across 0° of azimuth (US1-S2).
- [ ] T002 [US1] Add failing tests to `crates/simulator/tests/imaging_history.rs` for a
  push by hand and the other three ways the memory begins again (US1-S3), a simulator
  with no mount (US1-S4), count steps on both axes (US1-S5), the memory's hour and a
  sample dropped after it (US1-S6), and a push made on the mount itself, which keeps the
  memory (US1-S7).

### Implementation

- [ ] T003 [US1] In `crates/simulator/src/optics.rs`, add `HISTORY_S`, the memory,
  `remember`, `forget` and `direction_at`, and make 030's `imaging_truth_at` and
  `imaging_rotation_at` take their direction from `direction_at`; call `forget` from
  `push_to` and `set_optics`. In `crates/simulator/src/controls.rs`, call `remember` in
  `deref` after `follow_mount`, and `forget` in `connect_mount` and `point_eyepiece_at`
  (plan.md § The simulator's memory). Leave `directions_at` as it is.
- [ ] T004 [US1] In `crates/simulator/src/mount.rs`, add `set_count_steps` and
  `count_steps` and the stepped clamp in `Motor::travel` (plan.md § Count steps).
  Add no field to `MountConfig`.
- [ ] T005 [US1] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US1-S1, US1-S2, US1-S3, US1-S4, US1-S5, US1-S6 and
  US1-S7).

## Phase 2: User Story 2 — The tracking error from a sub (Priority: P1)

### Tests first

- [ ] T006 [US2] Create `crates/finder/tests/console/steering_error.rs`, the module
  `steering_error` of finder's `console` test binary, with `mod steering_error;` in its
  alphabetical place in `crates/finder/tests/console/main.rs`. Write failing tests for
  the two worked tracking errors (US2-S1, US2-S2), the set point in a frame (US2-S3), a
  rendered star's centroid as the set point (US2-S4), a target at the far side of the
  sky and inputs that are not finite (US2-S5), the guard on the text of `steering.rs`,
  for thirteen names (US2-S6), and the trait's six provided answers (US2-S7).

### Implementation

- [ ] T007 [US2] Create `crates/finder/src/imaging/steering.rs` with `TrackingError`,
  `tracking_error`, `set_point_in_frame`, `Stepped`, `GapFuture` and the trait `Steered`
  with its five required and six provided methods (`longest_exposure_s`,
  `longest_exposure_ahead_s`, `steady`, `steady_between`, `pause` and `tune`), exactly
  as plan.md § The tracking error and the interface gives them, with no guarded name in
  a comment, and add `pub mod steering;` to
  `crates/finder/src/imaging/mod.rs`.
- [ ] T008 [US2] Add the **Tracking error** and **Set point** entries to `GLOSSARY.md`
  (plan.md § `GLOSSARY.md`).
- [ ] T009 [US2] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US2-S1, US2-S2, US2-S3, US2-S4, US2-S5, US2-S6 and
  US2-S7).

## Phase 3: User Story 3 — The tracker steered by tracking errors (Priority: P1)

### Tests first

- [ ] T010 [US3] Create `crates/finder/tests/motion/steering.rs`, the module `steering`
  of finder's `motion` test binary, with `mod steering;` in its alphabetical place,
  before `mod stop;`, in `crates/finder/tests/motion/main.rs`. Give it the tracker rig
  of spec.md § User Scenarios (the rig of
  `crates/finder/tests/motion/tracking/pi_loop.rs:104` with no base tilt and its tracker
  in an `Arc`, and waiting for the tracker's first rates and 3.5 s more), the truth
  error, the feed-forward rate, playing a sub, playing a sub late, the axis displacement
  and a seeded noise generator of its own. Write failing tests for the drift with
  feedback, with feedback a sub late and without (US3-S1) and for subs played without
  feedback beside noisy finder pointings (US3-S2). Thirteen scenarios that were this
  story's are US9's since 2026-11-19: this module needs no rig built without the wait
  and no goto console.
- [ ] T011 [US3] Add failing tests to `crates/finder/tests/motion/steering.rs` for an
  error under the deadband (US3-S3), a first error, two pairs with a large step between
  them that set no rate and a pair with a small one that sets it (US3-S4), a jump and
  the step's limit as an angle on the sky (US3-S5), and a rate change held to 0.1″/s
  (US3-S8).
- [ ] T012 [US3] Add failing tests to `crates/finder/tests/motion/steering.rs` for the
  finder camera while the subs steer, and the claim ending its proportional rate
  (US3-S6), and for the rates between gaps and after a stop (US3-S7).
- [ ] T013 [US3] Add failing tests to `crates/finder/tests/motion/steering.rs` for an
  error measured before the last gap's step, two errors handed over between two gaps and
  one no later than the last applied (US3-S9), an error measured before the last gap's
  rate change (US3-S10), and, after the claim has lapsed, the finder camera's step held
  for the next gap and applied there with the tracking offset left alone (US3-S11).

### Implementation

- [ ] T014 [US3] In `crates/finder/src/tracking.rs`, add the nine constants, the new
  fields of `Control` (`counts`, `direction`, `settling`, `pending`, `fresh_after`,
  `subs`, `lapsed`, `held`, `corrections`, `gaps`), `Subs`, `Held` (a step and no
  offset), `Correction`, `Shared::wake`, and `Tracker::target`, `steer` with its test of
  `fresh_after` and its three cases, `gap`, which queues its sender before it returns a
  `GapFuture` that owns only the receiver, `release` (which also forgets `held`) and
  `steered_by_subs` with `impl Steered for Tracker` (plan.md § The tracker). In 038's
  `Tracker::follow`, after it clears the samples, end the claim, clear a lapse, forget
  what was pending or held, as `release` does, and set `fresh_after` to the time now
  (plan.md § The tracker, **`follow`**). Put `counts` and `settling: true` into the
  `Control` that `start_with_resets` builds, and make `end` clear the claim, the pending
  error, `lapsed`, `held`, the remembered corrections and the waiting gaps.
- [ ] T015 [US3] In `crates/finder/src/tracking.rs`, make the top of `run`'s loop the
  `select!` of plan.md § The tracker, store `direction` in `Control` before the rates
  are sent and set `settling` false after them at the end of every turn, move the
  rate rule into `commanded`, and add `serve_gaps(&mut movement)` with its six steps,
  reading the target and the counts under the lock in step 1: the senders
  whose receiver has gone dropped first; a gap turned away while `settling` is true;
  the claim, which a lapsed tracker makes only with an error pending and which sets
  `previous_pointing` to `None` and forgets `held`; the deadband and the step's limit of
  each axis from the sky angle its arcsecond makes; for a lapsed claim, the held step
  applied (clamped to 30″ on the sky an axis, the tracking offset not touched,
  `proportional` zeroed); the drift rate with `between` and `unseen`, taken only under
  the jump test and with no more than two deadbands of steps between the two errors, and
  changing the offset by `SUB_RATE_STEP` at most; the step from `seen`; the lapse at the
  eighth idle gap, which sets `lapsed` and sends nothing; `base` sent at every other gap
  served, steered or lapsed, changed or not; the step made booked for the time its pulse
  lasted; the correction remembered for every gap that applied an error or a held step;
  and the step on the sky from the shared q and E of `axis_error`. In
  `Shared::pointing`, skip the proportional and offset block while the subs steer, and
  while `lapsed` write the newest pointing's step to `held` and leave the tracking
  offset alone, as plan.md § The tracker, **`Shared::pointing`**, gives it. Change the
  module's first line as plan.md gives it. Build all of plan.md § The tracker, not only
  what this story's eleven scenarios reach: US9's thirteen test the same code, the
  lapse's and the release's edges, a gap before the first rates and 038's `follow` among
  them.
- [ ] T016 [US3] Run the `lint` and `test` gates with every existing test unchanged and
  green, 015's tracking tests among them. Commit the green story (US3-S1, US3-S2,
  US3-S3, US3-S4, US3-S5, US3-S6, US3-S7, US3-S8, US3-S9, US3-S10 and US3-S11).

## Phase 4: User Story 4 — Corrections only in the gap (Priority: P1)

### Tests first

- [ ] T017 [US4] Create `crates/finder/tests/console/steering_gap.rs`, the module
  `steering_gap` of `console`, with its `mod` line in
  `crates/finder/tests/console/main.rs`, and in it the scripted tracker of spec.md
  § User Scenarios. Write failing tests for the order of the calls and the view
  (US4-S1), no tracker (US4-S2), a frame from before the stack, the `Hold` with the
  view's `holding`, and nothing handed over after a release (US4-S3), a named set point
  110″ out and a binned region (US4-S4), the imaging view's `steering` part on a console
  built with `with_imaging` (US4-S7), a stack whose first tracking error is over 180″
  (US4-S8), an exposure remembered from when `before_exposure` answered (US4-S9), a
  tracker with no target (US4-S10), a tracker dropped while the steering remembers it
  (US4-S11), and a set point changed during a stack and put back with `None` (US4-S12).
- [ ] T018 [US4] Create `crates/finder/tests/motion/steering_gap.rs`, the module
  `steering_gap` of `motion`, with `mod steering_gap;` after `mod steering_claim;` in
  `crates/finder/tests/motion/main.rs`, holding the tracker rig with its mount started
  on M42's catalogue place, its `Simulator`, which US4-S5 reads, and its wait for the
  first rates. Write failing tests for thirty subs played late through `Steering` with
  the worked ask and solves from the simulator's truth (US4-S5) and, on
  `crate::support::goto::Rig`, for `Session::steered` (US4-S6).

### Implementation

- [ ] T019 [US4] In `crates/finder/src/imaging/steering.rs`, add `Source`,
  `EXPOSURES_KEPT`, `FAR_ARCSEC`, `ExposureAsk`, `Go`, `Hold`, `Steering` with `new`,
  `set_source`, `hold`, `before_exposure`, `accepted`, which stands back from a stack
  whose first tracking error is over `FAR_ARCSEC`, `release`, `set_set_point` and
  `view`, the exposure records and the record of a frame, the private function that lets
  the tracker go, which a `set_set_point` that changes the set point calls, the count of
  `Hold`s alive, and `ErrorView` and `SteeringView` with US4's five keys (plan.md § The
  steering). Keep trackers as `Weak`, let the `Arc` go before a gap's answer is awaited,
  and hold no lock across an await.
- [ ] T020 [US4] Add `Session::steered` to `crates/finder/src/session.rs`, and to
  `crates/finder/src/imaging/mod.rs` the `steering` field made in `Imaging::start`,
  `Imaging::steering`, and the `steering` part of `ImagingView` (plan.md § The session,
  § The imaging state).
- [ ] T021 [US4] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US4-S1, US4-S2, US4-S3, US4-S4, US4-S5, US4-S6, US4-S7,
  US4-S8, US4-S9, US4-S10, US4-S11 and US4-S12).

## Phase 5: User Story 5 — The exposure cap (Priority: P2)

### Tests first

- [ ] T022 [US5] Create `crates/finder/tests/console/steering_cap.rs`, the module
  `steering_cap` of `console`, with its `mod` line in
  `crates/finder/tests/console/main.rs` and a scripted tracker that logs the turn it is
  asked about and each `tune`, and answers a longest exposure for now and one for the
  hour ahead. Write failing tests for the turn, compared with the test's own `hypot`
  expressions, and the halving with its floor of 1 s (US5-S1), the exposure and
  `limited` held, `over_limit`, and the view right after a release (US5-S2), the smear
  setting and its refusals (US5-S3), the view (US5-S4), the route on 032's solving rig,
  its four keys and its 400, 403, 415 and 409 among them (US5-S5), the switch off before
  a stack (US5-S7), the switch turned off during one (US5-S9), an exposure decided from
  the hour ahead (US5-S10), `view_for` and what a stack started now would take
  (US5-S12), and the two shares (US5-S13). Give no case a limit under half the exposure
  it answers: from US6 such a call waits.
- [ ] T023 [US5] Create `crates/finder/tests/motion/steering_cap.rs`, the module
  `steering_cap` of `motion`, with `mod steering_cap;` after `mod steering;` and before
  `mod steering_claim;` in `crates/finder/tests/motion/main.rs`, holding the part of the
  tracker rig it uses, for M42 and for a target of the test's own. Write failing tests
  for the tracker's longest exposure against 015's `ephemeris::longest_exposure` at the
  same instant (US5-S6), for a target the test finds to be due east at t*, asked at t*
  and 10 s later (US5-S8), for the tracker's limit of the hour ahead against the test's
  own 61 samples (US5-S11), and for a tracker tuned with other shares (US5-S14).

### Implementation

- [ ] T024 [US5] In `crates/finder/src/tracking.rs`, add `Tracker::longest_exposure_s`,
  `longest_exposure_ahead_s` with its two constants, and `tune`, with `step_share` and
  `rate_share` in `Control` and `serve_gaps` step 2 reading them where US3 wrote the two
  constants, and their three lines in `impl Steered for Tracker` (plan.md § The
  tracker's two answers). Do not edit US3's or US9's test files: a tracker nobody tunes
  uses 0.5 and 0.3.
- [ ] T025 [US5] In `crates/finder/src/imaging/steering.rs`, add the three constants,
  the four settings with `set_smear_px`, `set_on`, `set_step_share` and
  `set_rate_share`, `held` (the exposure and whether it was cut, decided together) and
  `asked`, the cap in `before_exposure`, decided from the tracker's limit of the hour
  ahead and not halved to under 1 s, `tune` right before every `gap`, `view_for`,
  `LimitView` and US5's nine keys of the view; read `on` where a gap would be asked for
  and with `on` false call neither `gap`, `tune` nor `steer`; make `set_on(false)` let
  the tracker go; make `release` forget what is held. In
  `crates/finder/src/imaging/routes.rs`, add `SteeringEdit`, `apply_steering` and
  `PUT /api/imaging/steering` (plan.md § The route). In
  `crates/finder/src/imaging/mod.rs`, give the view its ask with `view_for` (plan.md
  § The imaging state). Leave `Steering::view`'s signature as US4's tests call it.
- [ ] T026 [US5] Add the route's line to `crates/finder/src/lib.rs` and its row to the
  `## API` table of `crates/finder/README.md` (plan.md § The route).
- [ ] T027 [US5] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US5-S1, US5-S2, US5-S3, US5-S4, US5-S5, US5-S6, US5-S7,
  US5-S8, US5-S9, US5-S10, US5-S11, US5-S12, US5-S13 and US5-S14).

## Phase 6: User Story 6 — Not steady: pause, and drop what was taken across it (Priority: P2)

### Tests first

- [ ] T028 [US6] Create `crates/finder/tests/motion/steering_steady.rs`, the module
  `steering_steady` of `motion`, with its `mod` line after `mod steering_gap;` in
  `crates/finder/tests/motion/main.rs`, on the backlash rig (`crate::support::tracker`,
  with a scratch folder from `crate::support::Scratch`). Write failing tests for a
  take-up at transit, with the follower, the reader and the asker joined on the test's
  own task, and the tracker's word for it (US6-S1), for a first reversal on a fresh
  session (US6-S2), for a first reversal's measurement under a claim that lapsed before
  it, with gaps asked every 6 s (US6-S5), and, on the tracking console
  (`crate::support::goto::Rig`), for a push by hand that comes back and one that is
  followed (US6-S6); and on the backlash rig again for both axes stopped on the
  simulated mount itself (US6-S7), a quiet pointing before a lapsed gap (US6-S8),
  tracking errors from before and from inside a time not steady (US6-S9), and a gap
  queued behind a take-up when tracking ends (US6-S10). Nine scenarios on this rig are
  US10's since 2026-11-19.
- [ ] T029 [US6] Create `crates/finder/tests/console/steering_steady.rs`, the module
  `steering_steady` of `console`, with its `mod` line in
  `crates/finder/tests/console/main.rs` and a scripted tracker whose `steady`,
  `steady_between`, `pause` word and longest exposure the test sets and whose last
  window it keeps. Write failing tests for the wait and its word (US6-S3), for `check`:
  a sub the tracker was not steady for, one it was, a tracker that has gone, a sub taken
  with no tracker, the count after a `release` and after a new `Hold`, a frame started
  just before its record, and the drop with the switch off (US6-S4), a tracker that
  stops being steady while its gap is awaited (US6-S11), a gap that ends on a tracker no
  longer steady (US6-S12), the view before any wait (US6-S13), a wait abandoned with its
  `Hold` (US6-S14), the switch off, before the call and during the wait (US6-S15), a
  source that no longer answers a tracker (US6-S16), and the wait for the limit
  (US6-S17).

### Implementation

- [ ] T030 [US6] In `crates/finder/src/tracking.rs`, add `SETTLE` and, to `Control`,
  `steady`, `unsteady`, `settle_until`, `pushed`, `stalled`, `sent` and `cause`, the
  word written wherever `settling` is set, `Tracker::pause` with its line in
  `impl Steered for Tracker`, the list's first entry from before the start, `mark` and
  its calls, `settling = true` at the start of a
  take-up and of the keyhole pass and the settling rule at the end of every turn,
  `Tracker::pushed`, the stopped-axis test at the top of a turn with `sent` written
  wherever the task sends the azimuth axis a rate, `Tracker::steady` and
  `steady_between`, whose window does not touch a time that ends as it begins, with
  their lines in `impl Steered for Tracker`, `mark` forgetting `held` and setting
  `fresh_after` as the tracker stops being steady, `follow` and `end` clearing `pushed`,
  `steer` doing nothing while the tracker is not steady, the test in `serve_gaps` step 1
  changed from `settling` to `!steady`, and the held mode's test in `Shared::pointing`
  changed from `lapsed` to `lapsed && steady` (plan.md § The tracker's two answers). In
  `crates/finder/src/session.rs`, in 038's `track_live_pointing`, call `Tracker::pushed`
  wherever the run's push becomes `Moving` and wherever a `Moving` push ends with no
  follow, with the session's lock released, and change nothing else there (plan.md § The
  session, **The push**). Do not edit `crates/finder/tests/motion/steering.rs`,
  `steering_claim.rs` or US5's two test files: they pass as they stand.
- [ ] T031 [US6] In `crates/finder/src/imaging/steering.rs`, add `POLL`, the wait before
  the gap, the look at the tracker every `POLL` while the gap's answer is awaited, and
  the check after it in `before_exposure`, `steady` and `waiting` in the
  state, true and false when built and after `release`, `Verdict`, `NOT_STEADY`,
  `Steering::check`, `Steering::waiting`, the pause's word in the state, set from
  `Steered::pause` while the wait is for the tracker, the wait while the limit is under
  half the exposure held, with the tracker let go for it and the word `limit`, `hold`
  setting `dropped_not_steady` to 0, and US6's four keys of the view (plan.md § The
  steering).
- [ ] T032 [US6] Run the `lint` and `test` gates with every existing test unchanged and
  green, US3's among them. Commit the green story (US6-S1, US6-S2, US6-S3, US6-S4,
  US6-S5, US6-S6, US6-S7, US6-S8, US6-S9, US6-S10, US6-S11, US6-S12, US6-S13, US6-S14,
  US6-S15, US6-S16 and US6-S17).

## Phase 7: User Story 7 — The stack asks before every exposure (Priority: P2)

### Tests first

- [ ] T033 [US7] Create `crates/finder/tests/motion/steering_stack.rs`, the module
  `steering_stack` of `motion`, with `mod steering_stack;` after `mod steering_gap;` and
  before `mod steering_steady;` in `crates/finder/tests/motion/main.rs`, and the
  stacking rig of spec.md § User Story 7. Write failing tests, reading each saved sub
  from its FITS file (spec.md § User Story 7), for a stack on a mount 1% fast (US7-S1),
  with the smear set to 2 px, what the view says a stack would take, an exposure cut
  under the limit and the setting put back (US7-S2), a sub during which Track was turned
  off and on (US7-S3), a stack with no tracker (US7-S4), with a scripted tracker of the
  module's own given to the console's steering, a stack whose change of the exposure is
  refused (US7-S5), the guard on the text of
  `crates/finder/src/imaging/stack/service.rs`, read from `CARGO_MANIFEST_DIR`, for
  thirteen names (US7-S6), a stack on a rig whose aim point is 16 finder pixels from the
  true one, which the steering stands back from (US7-S7), a stack with the switch off
  and the finder camera's frames stopped (US7-S8), a stop while the first gap waits
  (US7-S9), and a link lost while the second waits, with the setting left at the stack's
  exposure (US7-S10).

### Implementation

- [ ] T034 [US7] In `crates/finder/src/imaging/stack/service.rs`, add `not_steady` to
  `Dropped`, to its `total()` and, as an arm, to the match of its `count`, which is
  `unreachable!` for an outcome it does not name; in the taker, `Stacker::take_subs`,
  its parameter made `mut stopping`, the
  `Hold`, the `ExposureAsk` built without unwrapping the view, the local `restore_ms`,
  the loop run inside one `async` block, `before_exposure` before each take raced in a
  `biased` `tokio::select!` against the stopping mark's
  `stopping.wait_for(|stopping| *stopping)`, and before the
  first take the console's exposure set through the run's lease with `Lease::set`, a
  refusal ending the stack with `camera` and the words of plan.md § The stack; after
  that block, on every return, the observer's exposure put back through the lease
  before the task ends and the lease drops with it; in the worker, `check` in
  `Worker::process` after the `SolveFrame` is built and before the settings test, its
  drop answered as `Err(reason)` as the settings drop is, and `accepted` in
  `Worker::run` right after `note_solve` and before `make_picture`, with no stack lock
  held (plan.md § The stack, § What 035 must provide). Add nothing to 035's ending, name
  none of the thirteen names of US2-S6, in code or in a comment, and change nothing else
  in the file.
- [ ] T035 [US7] In `crates/finder/src/imaging/solving.rs`, give the steering its source
  in `set_solver` (plan.md § `set_solver`). Add the section
  `## Tracking steered by the subs` to `crates/finder/README.md`, with the meaning of
  each key of the `steering` part and the paragraph on why subs are dropped (plan.md
  § `crates/finder/README.md`).
- [ ] T036 [US7] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US7-S1, US7-S2, US7-S3, US7-S4, US7-S5, US7-S6, US7-S7,
  US7-S8, US7-S9 and US7-S10).

## Phase 8: User Story 8 — The readout on the Imaging page and in the session (Priority: P3)

### Tests first

- [ ] T037 [US8] Create `crates/finder/tests/console/steering_readout.rs`, the module
  `steering_readout` of `console`, with its `mod` line in
  `crates/finder/tests/console/main.rs`, a set clock, and a scripted tracker whose `gap`
  answers the `Stepped` the test sets. Write failing tests for the worked drift and
  error (US8-S1) and for the window and its emptying (US8-S2).
- [ ] T038 [US8] Create `crates/finder/web/src/components/SteeringBlock.test.tsx` with
  failing tests for US8-S3 to US8-S10, stubbing `fetch` as 031's Imaging page tests do;
  US8-S9 renders the block twice, and US8-S10 renders 035's `StackBlock`.

### Implementation

- [ ] T039 [US8] In `crates/finder/src/imaging/steering.rs`, add `stepped` to the
  exposure record, the window of ten errors, the records' steps in the drift, and the
  four keys of the view (plan.md § The steering, **The readout**). Add the four keys to
  the list of the `steering` part's keys in `crates/finder/README.md`, in US7's section:
  the root mean square of the last ten tracking errors and of the drifts between them,
  in arcseconds and pixels, once two errors have been handed over since the tracker was
  last let go.
- [ ] T040 [US8] Add `ImagingSteering`, `putImagingSteering` with its four keys and the
  optional `not_steady` of `StackDropped` to `crates/finder/web/src/api.ts`, create
  `crates/finder/web/src/components/SteeringBlock.tsx` with the words of plan.md § The
  page, a sentence for each pause word and the general one for any other, the four
  fields as drafts and the two shares under `Tuning`, place it in
  `crates/finder/web/src/components/ImagingPage.tsx` after the stack block, and add the
  words `while the mount was not steady` to the `Dropped` line of
  `crates/finder/web/src/components/StackBlock.tsx`, its filter of the counts above 0
  reading `(stack.dropped[key] ?? 0) > 0` (plan.md § The page).
- [ ] T041 [US8] Add the block's entry to `DESIGN.md` and its component to
  `.impeccable/design.json`. Run the `lint` and `test` gates with every existing Vitest
  and Playwright test unchanged and green, commit the rebuilt `crates/finder/web/dist`,
  and commit the green story (US8-S1, US8-S2, US8-S3, US8-S4, US8-S5, US8-S6, US8-S7,
  US8-S8, US8-S9 and US8-S10).

## Phase 9: User Story 9 — The claim's edges, tested on the tracker (Priority: P1)

Built after User Story 3 and before User Story 4 (spec.md § Work Graph). Its thirteen
scenarios were scenarios of User Story 3, or the later halves of some, until the cut of
2026-11-19 (spec.md § Clarifications, the session of the reanalysis). They test User
Story 3's code, and this story writes nothing under `crates/finder/src`.

### Tests first

- [ ] T042 [US9] Create `crates/finder/tests/motion/steering_claim.rs`, the module
  `steering_claim` of finder's `motion` test binary, with `mod steering_claim;` after
  `mod steering;` and before `mod stop;` in `crates/finder/tests/motion/main.rs`. Write
  out in it the tracker rig of spec.md § User Scenarios, with its wait for the first
  rates and 3.5 s more and a way to build it without that wait, and the helpers its
  tests use (the truth error, the feed-forward rate, playing a sub, the axis
  displacement, a seeded noise generator): import nothing from `steering.rs` and add
  nothing to it. Write tests for subs played with feedback beside noisy finder pointings
  (US9-S1), an error over the deadband on each axis (US9-S2), a step against the axis's
  motion (US9-S3), the lapse at the eighth idle gap and what takes the tracker back
  (US9-S4), `release` (US9-S5), and the errors that keep or make no claim (US9-S6).
- [ ] T043 [US9] Add tests to `crates/finder/tests/motion/steering_claim.rs` for a gap
  before a tracker's first rates, on a rig built without the wait (US9-S7), a gap nobody
  waits for (US9-S8), on the tracking console (`crate::support::goto::Rig`), the
  pointing model's syncs while steered (US9-S9), a held step over one gap's limit
  (US9-S10), noisy finder pointings after a lapse (US9-S11), a followed push with an
  error from before it that arrives late (US9-S12), and a followed push under a lapsed
  claim (US9-S13). On US3's code all thirteen scenarios already pass, and they pass
  before and after this story: no test of theirs fails first. One that fails is a
  finding about US3's code: stop and escalate with its output.

### Implementation

- [ ] T044 [US9] Change nothing under `crates/finder/src`. Run the `lint` and `test`
  gates with every existing test unchanged and green, US3's `steering` included and not
  edited. Commit the green story (US9-S1, US9-S2, US9-S3, US9-S4, US9-S5, US9-S6,
  US9-S7, US9-S8, US9-S9, US9-S10, US9-S11, US9-S12 and US9-S13).

## Phase 10: User Story 10 — Not steady at its edges, tested on the tracker (Priority: P2)

Built after User Story 6 and before User Story 7 (spec.md § Work Graph). Eight of its
nine scenarios were a scenario of User Story 6, or the later halves of two, until the
cut of 2026-11-19; the ninth is new. They test User Story 6's code, and this story
writes nothing under `crates/finder/src`.

### Tests first

- [ ] T045 [US10] Create `crates/finder/tests/motion/steering_unsteady.rs`, the module
  `steering_unsteady` of finder's `motion` test binary, with `mod steering_unsteady;`
  after `mod steering_steady;` and before `mod stop;` in
  `crates/finder/tests/motion/main.rs`, on the backlash rig (`crate::support::tracker`,
  with a scratch folder from `crate::support::Scratch`) and with the lapsed backlash rig
  of spec.md § User Scenarios written out in it. Write tests for the rate in force when
  a lapsed gap answers once a take-up has settled (US10-S1), a first reversal's
  measurement and its close (US10-S2), a start with a gap asked before the first rates,
  the 2 s of settling after them, the windows before the start and the window of 10 s
  that 037 asks, and a stop (US10-S3), and what a take-up forgets: a step held (US10-S4)
  and a claim with an error not yet applied (US10-S5).
- [ ] T046 [US10] Add tests to `crates/finder/tests/motion/steering_unsteady.rs` for the
  trait's answers (US10-S6), the keyhole pass, read for at most 1800 s (US10-S7), subs
  played late through the transit on a mount 1% fast (US10-S8), and a proportional rate
  whose time has run out before a lapsed gap (US10-S9). On US6's code the scenarios
  already pass, and no test of theirs fails first; one that fails is a finding about
  US6's code: stop and escalate with its output. US10-S8's bound is the one probe round
  2 writes into spec.md: if the scenario still reads "the bound probe round 2 measures"
  when this story is built, stop and escalate, and assert no figure of your own.

### Implementation

- [ ] T047 [US10] Change nothing under `crates/finder/src`. Run the `lint` and `test`
  gates with every existing test unchanged and green, US6's `steering_steady` modules
  included and not edited. Commit the green story (US10-S1, US10-S2, US10-S3, US10-S4,
  US10-S5, US10-S6, US10-S7, US10-S8 and US10-S9).

## Phase 11: User Story 11 — An MCP client sets the steering (Priority: P3)

Built last, after User Story 8 (spec.md § Work Graph).

### Tests first

- [ ] T048 [US11] Create `crates/finder/tests/mcp/steering.rs`, the module `steering` of
  finder's `mcp` test binary, with `mod steering;` after `mod stack;` and before
  `mod tracking;` in `crates/finder/tests/mcp/main.rs`. Build in it the imaging console
  of spec.md § User Story 11, and the helpers its tests use to list the tools and to
  call one: `mcp` can import neither `console`'s nor `motion`'s rigs. Write failing
  tests for the tool's listing (US11-S1), the four settings set through it and read back
  through `get_imaging_camera` and `get_session` (US11-S2), and its refusals, with a
  console that takes no imaging frames (US11-S3).

### Implementation

- [ ] T049 [US11] In `crates/finder/src/mcp/tools.rs`, add `SteeringArguments` and the
  tool `set_steering`, which parses its arguments as the route's `SteeringEdit` and
  calls `crate::imaging::routes::apply_steering` (plan.md § The MCP tool). Leave
  `get_imaging_camera`'s description and every other tool as they are: a test of 031's
  asserts that description's words.
- [ ] T050 [US11] In `crates/finder/README.md`, in the list of the imaging camera's
  tools, make the count twelve, add to `get_imaging_camera`'s line that the view carries
  the `stack` and `steering` parts, and add `set_steering`'s line after `get_stack`'s
  (plan.md § The MCP tool).
- [ ] T051 [US11] Run the `lint` and `test` gates with every existing test unchanged and
  green, the `mcp` binary's among them. Commit the green story (US11-S1, US11-S2 and
  US11-S3).

## Execution Order

```text
US1 ─────────────────────────┐
                             ├──► US4 ──► US5 ──► US6 ──► US10 ──► US7 ──► US8 ──► US11
US2 ──► US3 ──► US9 ─────────┘
```

- **US1** may run beside **US2** and **US3**: US1 writes only `crates/simulator`, the
  other two only `crates/finder` and `GLOSSARY.md`.
- **US3** needs US2's `Steered`.
- **US9** holds thirteen scenarios cut from US3 on 2026-11-19, and tests US3's tracker
  with no code of its own. It follows US3 and comes before US4, since both add a `mod`
  line to `crates/finder/tests/motion/main.rs`.
- **US4** creates `Steering` in US2's file over the tracker US3 made steerable, and its
  US4-S5 needs US1's memory; it names US9 as well as US3, as the next story after US9 to
  write that `main.rs`.
- **US5** and **US6** each add to `steering.rs` and `tracking.rs`, US5 first; US5 also
  gives the imaging view its ask in `imaging/mod.rs`, and US6 also adds the push's calls
  to `session.rs`, after US4's `Session::steered`.
- **US10** holds nine scenarios, eight of them cut from US6 on 2026-11-19, and tests
  US6's tracker with no code of its own. It follows US6 and comes before US7, for the
  same `main.rs`.
- **US7** wires the finished steering into 035's taker and worker; it is the one story
  that edits 035's stack, the one whose tests start a stack, and the one that changes a
  camera setting, through the stack's own lease. It also follows US5, whose README row
  it writes after, and names US10, the story before it to write
  `crates/finder/tests/motion/main.rs`.
- **US8** adds the readout's figures after US6's keys and shows them; it touches
  `steering.rs`, the page, one line of 035's `StackBlock.tsx`, the README section US7
  writes, `DESIGN.md` and `.impeccable/design.json`.
- **US11** adds the MCP tool `set_steering` last: it calls the function US5's route
  calls, and writes the README's list of tools after US8's keys.
- **`[P]`** marks nothing here: every task shares a file with another in its story.
