---
description: "Task list for 036-tracking-from-subs: Tracking Steered by the Subs"
---

# Tasks: Tracking Steered by the Subs

**Prerequisites** (plan.md § Prerequisites):
- `015-tracking` has landed on `main`: met;
- `030-imaging-camera-link`, `031-imaging-frames-on-console`, `032-imaging-solve-and-aim-point`,
  `033-focus-reading-and-aid`, `034-dark-and-flat-frames` and `035-live-stack` have
  landed, 031 with the camera's lease: not met;
- this trio has been re-anchored against `main` after they landed, each point of
  plan.md § What 035 must provide read in the landed code, and US7's calls placed
  against 035's taker, its returns and its worker by name and line: not met;
- the owner decisions in spec.md are settled and its owner approvals given, and the
  plan document agrees with them (plan.md § Prerequisites 9; in this worktree it
  carries the recommended answers, and it is not on `main`): not met;
- a probe build has confirmed the bounds of US3-S1, US3-S2, US4-S5, US6-S5 and
  US7-S1, and that US3's test file passes unedited once US6 has landed: not met;
- the documents this trio cites are on `main`: not met;
- 032's owner decision on a newly measured aim point while Track is on is settled,
  and, if its answer is that the tracker bounds the rate one tracking error may ask
  for, US3 carries that bound with one scenario (plan.md § Prerequisites 11): met on
  2026-11-17; the owner chose to hold the aim point, so no bound is added;
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
- While a stack holds the tracker, a finder pointing changes a rate only through a
  gap, whether the subs steer or have lapsed; only while the tracker is not steady, or
  with no stack, does it correct as 015 has it (plan.md § Plan traps 27 and 28).
- A tracker rig waits for the tracker's first rates before any scenario asks it for a
  gap, in every module that writes the rig out (spec.md § User Scenarios, The tracker
  rig; plan.md § Plan traps 22). The scenarios that say "built without the wait" are
  the only ones that skip it.
- The stack changes the camera's exposure only through the lease it already holds,
  with `Lease::set`; never through `Imaging::set`, which that lease refuses (034's
  `Calibration::restore_exposure`, which went through it, is gone; plan.md § Plan traps 23).
- No file under `crates/finder/src/imaging/` computes an altitude, an azimuth, a
  parallactic angle or a field rotation, reads an axis count, or names the finder
  camera in a view's words.
- `Steering` keeps only `Weak` references to a tracker (plan.md § Plan traps 15), and
  holds its lock across no await and across no call to its source, a tracker or
  `Imaging` (plan.md § Plan traps 14 and 26; § The steering, **Lock order**).
- Keep the story's diff under 64 KiB, measured as plan.md § Plan traps 10 says.
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
  commanded, and failing tests for the truth of three instants of an exposure asked
  30 s later, the azimuth's rise and the star's trail, measured over the window
  US1-S1 gives round the midpoint of the star's two places (US1-S1), and for the
  instant between two samples against a twin read live (US1-S2).
- [ ] T002 [US1] Add failing tests to `crates/simulator/tests/imaging_history.rs` for a
  push by hand (US1-S3), a simulator with no mount (US1-S4) and count steps (US1-S5).

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
  green. Commit the green story (US1-S1, US1-S2, US1-S3, US1-S4 and US1-S5).

## Phase 2: User Story 2 — The tracking error from a sub (Priority: P1)

### Tests first

- [ ] T006 [US2] Create `crates/finder/tests/console/steering_error.rs`, the module
  `steering_error` of finder's `console` test binary, with `mod steering_error;` in its
  alphabetical place in `crates/finder/tests/console/main.rs`. Write failing tests for
  the two worked tracking errors (US2-S1, US2-S2), the set point in a frame (US2-S3), a
  rendered star's centroid as the set point (US2-S4), a target at the far side of the
  sky, a time that is not finite and the trait's provided answers (US2-S5), and the
  guard on the text of `steering.rs` (US2-S6).

### Implementation

- [ ] T007 [US2] Create `crates/finder/src/imaging/steering.rs` with `TrackingError`,
  `tracking_error`, `set_point_in_frame`, `Stepped`, `GapFuture` and the trait `Steered`
  with its five required and three provided methods, exactly as plan.md § The tracking
  error and the interface gives them, and add `pub mod steering;` to
  `crates/finder/src/imaging/mod.rs`.
- [ ] T008 [US2] Add the **Tracking error** and **Set point** entries to `GLOSSARY.md`
  (plan.md § `GLOSSARY.md`).
- [ ] T009 [US2] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US2-S1, US2-S2, US2-S3, US2-S4, US2-S5 and US2-S6).

## Phase 3: User Story 3 — The tracker steered by tracking errors (Priority: P1)

### Tests first

- [ ] T010 [US3] Create `crates/finder/tests/motion/steering.rs`, the module `steering`
  of finder's `motion` test binary, with `mod steering;` in its alphabetical place,
  before `mod stop;`, in `crates/finder/tests/motion/main.rs`. Give it the tracker rig
  of spec.md § User Scenarios (the rig of
  `crates/finder/tests/motion/tracking/pi_loop.rs:104` with no base tilt, keeping its
  `Simulator`, and waiting for the tracker's first rates unless a test asks it not
  to), the truth error, the feed-forward rate, playing a sub, playing a sub late, the
  axis displacement and a seeded noise generator of its own. Write failing
  tests for the drift with feedback, with feedback a sub late and without (US3-S1) and
  beside noisy finder pointings (US3-S2).
- [ ] T011 [US3] Add failing tests to `crates/finder/tests/motion/steering.rs` for the
  deadband and the step, each case on a fresh rig (US3-S3), a first error, a
  consistent second and a drift rate (US3-S4), and a jump, the step's limit and a step
  against the axis's motion (US3-S5).
- [ ] T012 [US3] Add failing tests to `crates/finder/tests/motion/steering.rs` for the
  finder camera while the subs steer, three idle gaps, a fourth that leaves the claim
  lapsed, two finder pointings after it that change no rate, the tracking error that
  takes the tracker back and forgets what they held, `release`, and a `steer`
  that is not finite or comes before any gap (US3-S6); the rates between gaps, after a
  stop, and a gap before a tracker's first rates on a rig built without the wait
  (US3-S7); and, on `crate::support::goto::Rig`, the pointing model's syncs while
  steered (US3-S8).
- [ ] T013 [US3] Add failing tests to `crates/finder/tests/motion/steering.rs` for an
  error measured before the last gap's step (US3-S9), one measured before the last
  gap's rate change (US3-S10), and, after the claim has lapsed, the finder camera's
  correction held for the next gap and applied there, with the rate log between gaps
  under noisy finder pointings (US3-S11).

### Implementation

- [ ] T014 [US3] In `crates/finder/src/tracking.rs`, add the eight constants, the new
  fields of `Control` (`counts`, `direction`, `settling`, `pending`, `subs`, `lapsed`,
  `held`, `corrections`, `gaps`), `Subs`, `Held`, `Correction`, `Shared::wake`, and
  `Tracker::target`, `steer` with its three cases, `gap`, `release` (which also
  forgets `held`) and `steered_by_subs` with `impl Steered for Tracker` (plan.md § The
  tracker). Put `counts` and `settling: true` into the `Control` that
  `start_with_resets` builds, and make `end` clear the claim, the pending error,
  `lapsed`, `held`, the remembered corrections and the waiting gaps.
- [ ] T015 [US3] In `crates/finder/src/tracking.rs`, make the top of `run`'s loop the
  `select!` of plan.md § The tracker, store `direction` in `Control` before the rates
  are sent and set `settling` false after them at the end of every turn, move the
  rate rule into `commanded`, and add `serve_gaps` with its six steps: the senders
  whose receiver has gone dropped first; a gap turned away while `settling` is true;
  the claim, which a lapsed tracker makes only with an error pending and which sets
  `previous_pointing` to `None` and forgets `held`; for a lapsed claim, the held
  correction applied (the offset within its limit, the step clamped to 30″ an axis,
  `proportional` zeroed) and `base` sent only when the offset changed; the drift rate
  with `between` and `unseen`; the step from `seen`; the lapse at the third idle gap,
  which sets `lapsed`; the correction remembered for every gap that applied an error
  or a held correction; and the step on the sky from the shared q and E of
  `axis_error`. In `Shared::pointing`, skip the proportional and offset block while
  the subs steer, and while `lapsed` redirect its two writes to `held` (the newest
  step replacing, the offset changes adding), as plan.md § The tracker,
  **`Shared::pointing`**, gives them. Change the module's first line as plan.md gives
  it.
- [ ] T016 [US3] Run the `lint` and `test` gates with every existing test unchanged and
  green, 015's tracking tests among them. Commit the green story (US3-S1, US3-S2,
  US3-S3, US3-S4, US3-S5, US3-S6, US3-S7, US3-S8, US3-S9, US3-S10 and US3-S11).

## Phase 4: User Story 4 — Corrections only in the gap (Priority: P1)

### Tests first

- [ ] T017 [US4] Create `crates/finder/tests/console/steering_gap.rs`, the module
  `steering_gap` of `console`, with its `mod` line in
  `crates/finder/tests/console/main.rs`, and in it the scripted tracker of spec.md
  § User Scenarios. Write failing tests for the order of the calls and the view
  (US4-S1), no tracker and no target (US4-S2), a frame from before the stack, the
  `Hold` and nothing handed over after a release (US4-S3), and a named set point and a
  binned region (US4-S4).
- [ ] T018 [US4] Create `crates/finder/tests/motion/steering_gap.rs`, the module
  `steering_gap` of `motion`, with `mod steering_gap;` after `mod steering;` in
  `crates/finder/tests/motion/main.rs`, holding the tracker rig with its mount started
  on M42's catalogue place and its wait for the first rates. Write failing tests for
  thirty subs played late through `Steering` with the worked ask and solves from the
  simulator's truth (US4-S5) and, on `crate::support::goto::Rig`, for
  `Session::steered` (US4-S6).

### Implementation

- [ ] T019 [US4] In `crates/finder/src/imaging/steering.rs`, add `Source`,
  `EXPOSURES_KEPT`, `ExposureAsk`, `Go`, `Hold`, `Steering` with `new`, `set_source`,
  `hold`, `before_exposure`, `accepted`, `release`, `set_set_point` and `view`, the
  exposure records and the record of a frame, and `ErrorView` and `SteeringView` with
  US4's three keys (plan.md § The steering). Keep trackers as `Weak`, and hold no lock
  across an await.
- [ ] T020 [US4] Add `Session::steered` to `crates/finder/src/session.rs`, and to
  `crates/finder/src/imaging/mod.rs` the `steering` field made in `Imaging::start`,
  `Imaging::steering`, and the `steering` part of `ImagingView` (plan.md § The session,
  § The imaging state).
- [ ] T021 [US4] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US4-S1, US4-S2, US4-S3, US4-S4, US4-S5 and US4-S6).

## Phase 5: User Story 5 — The exposure cap (Priority: P2)

### Tests first

- [ ] T022 [US5] Create `crates/finder/tests/console/steering_cap.rs`, the module
  `steering_cap` of `console`, with its `mod` line in
  `crates/finder/tests/console/main.rs` and a scripted tracker that logs the turn it is
  asked about. Write failing tests for the turn, compared with the test's own `hypot`
  expressions, and the halving (US5-S1), the exposure and `limited` held and
  `over_limit` (US5-S2), the smear setting and its refusals (US5-S3), the view and the
  switch (US5-S4), and the route on 032's solving rig, its 400, 403, 415 and 409
  among them (US5-S5).
- [ ] T023 [US5] Create `crates/finder/tests/motion/steering_cap.rs`, the module
  `steering_cap` of `motion`, with `mod steering_cap;` after `mod steering;` in
  `crates/finder/tests/motion/main.rs`, holding the part of the tracker rig it uses,
  for M42 and for a target of the test's own. Write a failing test for the tracker's
  longest exposure against 015's `ephemeris::longest_exposure` at the same instant,
  and for a target the test finds to be due east at t*, asked at t* and 10 s later
  (US5-S6).

### Implementation

- [ ] T024 [US5] In `crates/finder/src/tracking.rs`, add `Tracker::longest_exposure_s`
  and its line in `impl Steered for Tracker` (plan.md § The tracker's two answers).
- [ ] T025 [US5] In `crates/finder/src/imaging/steering.rs`, add the two settings,
  `set_smear_px`, `set_on`, `held` (the exposure and whether it was cut, decided
  together) and the cap in `before_exposure`, `LimitView` and US5's six keys of the
  view; with `on` false call neither `gap` nor `steer`; make `release` forget what is
  held. In
  `crates/finder/src/imaging/routes.rs`, add `SteeringEdit` and
  `PUT /api/imaging/steering` (plan.md § The route).
- [ ] T026 [US5] Add the route's line to `crates/finder/src/lib.rs` and its row to the
  `## API` table of `crates/finder/README.md` (plan.md § The route).
- [ ] T027 [US5] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US5-S1, US5-S2, US5-S3, US5-S4, US5-S5 and US5-S6).

## Phase 6: User Story 6 — Not steady: pause, and drop what was taken across it (Priority: P2)

### Tests first

- [ ] T028 [US6] Create `crates/finder/tests/motion/steering_steady.rs`, the module
  `steering_steady` of `motion`, with its `mod` line after `mod steering_gap;` in
  `crates/finder/tests/motion/main.rs`, on the backlash rig
  (`crate::support::tracker`, with a scratch folder from `crate::support::Scratch`).
  Write failing tests for a take-up at transit, with the follower, the reader and the
  asker joined on the test's own task (US6-S1), for a first reversal, a start
  with a gap asked before the first rates, a window before the start and a stop
  (US6-S2), and for a first reversal's measurement under a claim that lapsed before
  it, with gaps asked every 6 s (US6-S5).
- [ ] T029 [US6] Create `crates/finder/tests/console/steering_steady.rs`, the module
  `steering_steady` of `console`, with its `mod` line in
  `crates/finder/tests/console/main.rs` and a scripted tracker whose `steady` and
  `steady_between` the test sets and whose last window it keeps. Write failing tests
  for the wait, a tracker that stops being steady during its gap, the view before any
  wait and a wait abandoned with its `Hold` (US6-S3), and for `check`: a sub the
  tracker was not steady for, one it was, a tracker that has gone and a sub taken with
  no tracker (US6-S4).

### Implementation

- [ ] T030 [US6] In `crates/finder/src/tracking.rs`, add `steady` and `unsteady` to
  `Control`, the list's first entry from before the start, `mark` and its six calls,
  `settling = true` at the start of a take-up and of the keyhole pass,
  `Tracker::steady` and `steady_between` with their lines in `impl Steered for
  Tracker`, `mark` forgetting `held` as the tracker stops being steady, the test in
  `serve_gaps` step 1 changed from `settling` to `!steady`, and the held mode's test in
  `Shared::pointing` changed from `lapsed` to `lapsed && steady` (plan.md § The
  tracker's two answers). Do not edit `crates/finder/tests/motion/steering.rs`: US3's
  tests pass as they stand.
- [ ] T031 [US6] In `crates/finder/src/imaging/steering.rs`, add `POLL`, the wait before
  the gap and the check after it in `before_exposure`, `steady` and `waiting` in the
  state, true and false when built and after `release`, `Verdict`, `NOT_STEADY`,
  `Steering::check`, `Steering::waiting`, and US6's three keys of the view (plan.md
  § The steering).
- [ ] T032 [US6] Run the `lint` and `test` gates with every existing test unchanged and
  green, US3's among them. Commit the green story (US6-S1, US6-S2, US6-S3, US6-S4 and
  US6-S5).

## Phase 7: User Story 7 — The stack asks before every exposure (Priority: P2)

### Tests first

- [ ] T033 [US7] Create `crates/finder/tests/motion/steering_stack.rs`, the module
  `steering_stack` of `motion`, with `mod steering_stack;` after `mod steering_gap;` in
  `crates/finder/tests/motion/main.rs`, and the stacking rig of spec.md § User Story 7.
  Write failing tests for a stack on a mount 1% fast with the switch on, and with the
  switch off and the finder camera's frames stopped (US7-S1), an exposure cut under
  the limit and the setting put back (US7-S2), a sub during which Track was turned off
  and on (US7-S3), a stack with no tracker (US7-S4), with a scripted tracker of
  the module's own given to the console's steering, a stack whose change of the
  exposure is refused (US7-S5), and the guard on the text of
  `crates/finder/src/imaging/stack/service.rs`, read from `CARGO_MANIFEST_DIR`
  (US7-S6).

### Implementation

- [ ] T034 [US7] In `crates/finder/src/imaging/stack/service.rs`, add `not_steady` to
  `Dropped` and its `total()`; in the taker, the `Hold`, the `ExposureAsk` built
  without unwrapping the view, the local `restore_ms`, the loop run inside one `async`
  block, `before_exposure` before each take raced in a `biased` `tokio::select!`
  against the stopping mark's `stopping.wait_for(|stopping| *stopping)`, and before the
  first take the console's exposure set through the run's lease with `Lease::set`, a
  refusal ending the stack with `camera` and the words of plan.md § The stack; after
  that block, on every return, the observer's exposure put back through the lease
  before the task ends and the lease drops with it; in the worker, `check` before the
  settings test and `accepted` where a sub is counted as stacked (plan.md § The stack,
  § What 035 must provide). Add nothing to 035's ending, name none of the six names
  of US2-S6, and change nothing else in the file.
- [ ] T035 [US7] In `crates/finder/src/imaging/solving.rs`, give the steering its source
  in `set_solver` (plan.md § `set_solver`). Add the section
  `## Tracking steered by the subs` to `crates/finder/README.md` (plan.md
  § `crates/finder/README.md`).
- [ ] T036 [US7] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US7-S1, US7-S2, US7-S3, US7-S4, US7-S5 and US7-S6).

## Phase 8: User Story 8 — The readout on the Imaging page and in the session (Priority: P3)

### Tests first

- [ ] T037 [US8] Create `crates/finder/tests/console/steering_readout.rs`, the module
  `steering_readout` of `console`, with its `mod` line in
  `crates/finder/tests/console/main.rs`, a set clock, and a scripted tracker whose `gap`
  answers the `Stepped` the test sets. Write failing tests for the worked drift and
  error (US8-S1) and for the window and its emptying (US8-S2).
- [ ] T038 [US8] Create `crates/finder/web/src/components/SteeringBlock.test.tsx` with
  failing tests for US8-S3, US8-S4, US8-S5 and US8-S6, stubbing `fetch` as 031's
  Imaging page tests do; US8-S6's second half renders 035's `StackBlock`.

### Implementation

- [ ] T039 [US8] In `crates/finder/src/imaging/steering.rs`, add `stepped` to the
  exposure record, the window of ten errors, the records' steps in the drift, and the
  four keys of the view (plan.md § The steering, **The readout**).
- [ ] T040 [US8] Add `ImagingSteering`, `putImagingSteering` and the optional
  `not_steady` of `StackDropped` to `crates/finder/web/src/api.ts`, create
  `crates/finder/web/src/components/SteeringBlock.tsx` with the words of plan.md § The
  page, place it in `crates/finder/web/src/components/ImagingPage.tsx` after the stack
  block, and add the words `not steady` to the `Dropped` line of
  `crates/finder/web/src/components/StackBlock.tsx`.
- [ ] T041 [US8] Add the block's entry to `DESIGN.md` and its component to
  `.impeccable/design.json`. Run the `lint` and `test` gates with every existing Vitest
  and Playwright test unchanged and green, commit the rebuilt `crates/finder/web/dist`,
  and commit the green story (US8-S1, US8-S2, US8-S3, US8-S4, US8-S5 and US8-S6).

## Execution Order

```text
US1 ───────────────┐
                   ├──► US4 ──► US5 ──► US6 ──► US7 ──► US8
US2 ──► US3 ───────┘
```

- **US1** may run beside **US2** and **US3**: US1 writes only `crates/simulator`, the
  other two only `crates/finder` and `GLOSSARY.md`.
- **US3** needs US2's `Steered`.
- **US4** creates `Steering` in US2's file over the tracker US3 made steerable, and its
  US4-S5 needs US1's memory.
- **US5** and **US6** each add to `steering.rs` and `tracking.rs`, US5 first.
- **US7** wires the finished steering into 035's taker and worker; it is the
  one story that edits 035's stack, the one whose tests start a stack, and the one
  that changes a camera setting, through the stack's own lease. It also follows US5,
  whose README row it writes after.
- **US8** adds the readout's figures after US6's keys and shows them; it touches
  `steering.rs`, the page, one line of 035's `StackBlock.tsx`, `DESIGN.md` and
  `.impeccable/design.json`.
- **`[P]`** marks nothing here: every task shares a file with another in its story.
