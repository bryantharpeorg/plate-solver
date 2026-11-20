---
description: "Task list for 036-tracking-from-subs: Tracking Steered by the Subs"
---

# Tasks: Tracking Steered by the Subs

**Prerequisites**: flip.md § Prerequisites. None of them is a builder's task: a story is
dispatched only once they are met.

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
  steady, or with no stack, does it correct as 015 has it (plan.md § Plan traps 27; 040's plan.md
  § Plan traps 28).
- A tracker rig waits for the tracker's first rates, and 3.5 s more, before any scenario
  asks it for a gap, in every module that writes the rig out (spec.md § User Scenarios,
  The tracker rig; plan.md § Plan traps 22). The scenarios that say "built without the
  wait" are the only ones that skip it.
- No file under `crates/finder/src/imaging/` computes an altitude, an azimuth, a
  parallactic angle or a field rotation, reads an axis count, or names the finder camera
  in a view's words. `steering.rs` holds none of the thirteen names spec.md US2-S6
  lists, in code or in a comment (plan.md § Plan traps 38).
- `Steering` keeps only `Weak` references to a tracker (plan.md § Plan traps 15), and
  holds its lock across no await and across no call to its source, a tracker or
  `Imaging` (plan.md § Plan traps 14 and 26; § The steering, **Lock order**).
- Keep the story's diff under 64 KiB, measured as plan.md § Plan traps 10 says. If the
  finished story measures over it, do not drop or shorten a case to get under: say its
  size when you hand off.
- A test asserts nothing a later story changes, and no story edits an earlier story's
  test file. US9 is a story of tests only: it writes nothing under `crates/finder/src`,
  its tests pass on the code of US3 as soon as they are written, and one that fails is
  escalated with its output (plan.md § Plan traps 36).
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
  feedback beside noisy finder pointings (US3-S2). The scenarios that need a rig
  built without the wait, or the goto console, are US9's: this module needs neither.
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
  it answers: from 040's US6 such a call waits.
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

## Phase 9: User Story 9 — The claim's edges, tested on the tracker (Priority: P1)

Built after User Story 3 and before User Story 4 (spec.md § Work Graph). Its thirteen
scenarios test User Story 3's code, and this story writes nothing under
`crates/finder/src`.

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

## Execution Order

```text
US1 ─────────────────────────┐
                             ├──► US4 ──► US5 ──► (040-stack-waits-for-steady)
US2 ──► US3 ──► US9 ─────────┘
```

- **US1** may run beside **US2** and **US3**: US1 writes only `crates/simulator`, the
  other two only `crates/finder` and `GLOSSARY.md`.
- **US3** needs US2's `Steered`.
- **US9** holds thirteen scenarios that test US3's tracker, with no code of its own. It
  follows US3 and comes before US4, since both add a `mod` line to
  `crates/finder/tests/motion/main.rs`.
- **US4** creates `Steering` in US2's file over the tracker US3 made steerable, and its
  US4-S5 needs US1's memory; it names US9 as well as US3, as the next story after US9 to
  write that `main.rs`.
- **US5** adds to `steering.rs` and `tracking.rs`, and gives the imaging view its ask in
  `imaging/mod.rs`, after US4.
- **`040-stack-waits-for-steady`** builds the rest once this spec has landed.
- **`[P]`** marks nothing here: every task shares a file with another in its story.
