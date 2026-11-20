---
description: "Task list for 040-stack-waits-for-steady: The Stack Waits for a Steady Tracker, and Shows the Steering"
---

# Tasks: The Stack Waits for a Steady Tracker, and Shows the Steering

**Prerequisites**: flip.md § Prerequisites; `036-tracking-from-subs` has landed and this
trio has been re-anchored on it (plan.md § What 036 provides). None of them is a
builder's task: a story is dispatched only once they are met.

**Rules for every story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml`,
  `CODEOWNERS`, `ergane.yaml`, `.specify/` or `crates/finder/web/playwright.config.ts`.
  036's test files are existing tests. If an existing test fails, stop and escalate with
  its exact output. Do not edit it. The one exception is the `mod` line of each new
  finder test module, added in alphabetical order to the binary's `main.rs` your story's
  tasks name, which holds no test (owner approval 9). Change nothing else there.
- New tests go in new files: the ones your story's tasks name. A new finder test is a
  module file in the binary's folder plan.md § Technical Context names, never a file at
  the top level of `crates/finder/tests` or in `crates/finder/tests/support`. A module
  writes its own setup, only the parts its own tests use, and never edits an earlier
  story's test file: the rigs and the scripted tracker of spec.md § User Scenarios are
  written out again where they are needed. It may use what the existing support files
  already offer.
- Write only the files your story's tasks name; they are its `writes` in plan.md
  § Refinement Record. A story that needs another file escalates.
- Tests compute their expected values themselves (plan.md § Plan traps 7): from the
  simulator's truth, the worked examples of spec.md, 015's own functions and bounds, and
  the backlash rig's rate log. No test reads an expected value from the code under test.
- Every rate goes to the mount from the tracker's own rate task (plan.md § Plan traps
  1). No story sends a rate from `gap`, from `Steering` or from the stack.
- While a stack holds the tracker, a finder pointing changes no rate when it comes,
  whether the subs steer or have lapsed, and after a lapse it changes the place only, as
  a step in the next gap, never the tracking offset; only while the tracker is not
  steady, or with no stack, does it correct as 015 has it (plan.md § Plan traps 28;
  § What 036 provides, 2).
- A tracker rig waits for the tracker's first rates, and 3.5 s more, before any scenario
  asks it for a gap, in every module that writes the rig out (spec.md § User Scenarios,
  The tracker rig; plan.md § Plan traps 22).
- The stack changes the camera's exposure only through the lease it already holds,
  with `Lease::set`; never through `Imaging::set`, which that lease refuses (034's
  `Calibration::restore_exposure`, which went through it, is gone; plan.md § Plan traps 23).
- No file under `crates/finder/src/imaging/` computes an altitude, an azimuth, a
  parallactic angle or a field rotation, reads an axis count, or names the finder camera
  in a view's words. `steering.rs` and 035's `stack/service.rs` hold none of the
  thirteen names 036's spec.md US2-S6 lists, in code or in a comment: a pause's reason is
  the tracker's word, written in `tracking.rs` and on the page only (plan.md § Plan
  traps 38).
- `Steering` keeps only `Weak` references to a tracker (plan.md § Plan traps 15), and
  holds its lock across no await and across no call to its source, a tracker or
  `Imaging` (plan.md § Plan traps 14 and 26; § What 036 provides, 3, **Lock order**).
- Keep the story's diff under 64 KiB, measured as plan.md § Plan traps 10 says. If the
  finished story measures over it, do not drop or shorten a case to get under: say its
  size when you hand off.
- A test asserts nothing a later story changes, and no story edits an earlier story's
  test file, 036's among them. US10 is a story of tests only: it writes nothing under
  `crates/finder/src`, its tests pass on the code of US6 as soon as they are written, and
  one that fails is escalated with its output (plan.md § Plan traps 36).
- Every code citation is `path:line` as read at 49bb210. If a cited line has moved,
  re-read it rather than guess.
- Add no dependency and change no `Cargo.toml`. Change nothing under
  `crates/scope-node`, `crates/scope-link`, `crates/simulator` or `proto/`.
- Do not edit `specs/040-stack-waits-for-steady/` or `specs/036-tracking-from-subs/`.
- Run no tool the gates don't run: no `cargo deny`, `cargo machete`, Rust 1.96 or
  `ergane`.
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
  US10's.
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
  changed from `lapsed` to `lapsed && steady` (plan.md § Steady, the tracker's second answer). In
  `crates/finder/src/session.rs`, in 038's `track_live_pointing`, call `Tracker::pushed`
  wherever the run's push becomes `Moving` and wherever a `Moving` push ends with no
  follow, with the session's lock released, and change nothing else there (plan.md § The
  session's push). Do not edit `crates/finder/tests/motion/steering.rs`,
  `steering_claim.rs` or 036's US5's two test files: they pass as they stand.
- [ ] T031 [US6] In `crates/finder/src/imaging/steering.rs`, add `POLL`, the wait before
  the gap, the look at the tracker every `POLL` while the gap's answer is awaited, and
  the check after it in `before_exposure`, `steady` and `waiting` in the
  state, true and false when built and after `release`, `Verdict`, `NOT_STEADY`,
  `Steering::check`, `Steering::waiting`, the pause's word in the state, set from
  `Steered::pause` while the wait is for the tracker, the wait while the limit is under
  half the exposure held, with the tracker let go for it and the word `limit`, `hold`
  setting `dropped_not_steady` to 0, and US6's four keys of the view (plan.md § The
  steering's wait and its check).
- [ ] T032 [US6] Run the `lint` and `test` gates with every existing test unchanged and
  green, 036's US3's among them. Commit the green story (US6-S1, US6-S2, US6-S3, US6-S4,
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
  held (plan.md § The stack, § What 035 provides). Add nothing to 035's ending, name
  none of the thirteen names of 036's US2-S6, in code or in a comment, and change nothing else
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
  four keys of the view (plan.md § The steering's wait and its check, **The readout**). Add the four keys to
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

## Phase 10: User Story 10 — Not steady at its edges, tested on the tracker (Priority: P2)

Built after User Story 6 and before User Story 7 (spec.md § Work Graph). Its nine
scenarios test User Story 6's code, and this story writes nothing under
`crates/finder/src`.

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
(036-tracking-from-subs, landed) ──► US6 ──► US10 ──► US7 ──► US8 ──► US11
```

- **US6** adds to what 036 left in `steering.rs` and `tracking.rs`, and adds the push's
  calls to `session.rs`, after 036's `Session::steered`.
- **US10** holds nine scenarios that test US6's tracker, with no code of its own. It
  follows US6 and comes before US7, since both add a `mod` line to
  `crates/finder/tests/motion/main.rs`.
- **US7** wires the finished steering into 035's taker and worker; it is the one story
  that edits 035's stack, the one whose tests start a stack, and the one that changes a
  camera setting, through the stack's own lease. It writes its README section after
  036's route row, and names US10, the story before it to write
  `crates/finder/tests/motion/main.rs`.
- **US8** adds the readout's figures after US6's keys and shows them; it touches
  `steering.rs`, the page, one line of 035's `StackBlock.tsx`, the README section US7
  writes, `DESIGN.md` and `.impeccable/design.json`.
- **US11** adds the MCP tool `set_steering` last: it calls the function 036's route
  calls, and writes the README's list of tools after US8's keys.
- **`[P]`** marks nothing here: every task shares a file with another in its story.
