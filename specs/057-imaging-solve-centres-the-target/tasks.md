---
description: "Task list for 057-imaging-solve-centres-the-target: An Imaging Solve Centres the Target"
---

# Tasks: An Imaging Solve Centres the Target

**Prerequisites** (plan.md § Prerequisites, each with its evidence there):
- every spec in `depends_on_landed` has landed on `main`: met at 936b140d;
- 052, 053, 054 and 055 have landed, in that order, and this trio is re-anchored on that
  `main`: met at 936b140d (the replay passed every gate step there);
- 058's preview frames read against FR-002 and FR-014, if 058 lands first: met, 058 had not
  landed at 936b140d;
- the owner decisions in spec.md are settled: met, each with its recommendation, on
  2026-11-28;
- every story has been probe-built from this text: met, round 1 of 2026-11-27 on
  `probe/057-r1` (tip ba98d527: 0 blocking, 10 clarity), folded, its proofs on that
  branch at f7d6779a;
- the page half has run once: met on `probe/057-r1` (`CI=1 npm run e2e` at each
  story's tip and at f7d6779a) and again on the re-anchored base at 936b140d (40 of 40);
- no operator-staged file is needed: met.

**Rules for every story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml`,
  `CODEOWNERS`, `ergane.yaml` or `.specify/`. If an existing test fails, stop and
  escalate with its exact output. A test file an earlier story of this spec created is
  an existing test; US1's rig and helpers are `pub(super)` from the start so later
  stories use them without editing it (plan.md § Plan traps 2). The exceptions: each
  story's `mod` lines in `crates/finder/tests/motion/main.rs` or
  `crates/finder/tests/mcp/main.rs`, each in its alphabetical place, and US6's two list
  entries in `crates/finder/web/playwright.config.ts` (spec owner approval 9).
- Write only the files your story's tasks name; they are its `writes` in plan.md
  § Refinement Record. A story that needs another file escalates.
- `crates/finder/src/model.rs`, `crates/finder/src/live.rs`,
  `crates/finder/src/goto.rs`, `crates/finder/src/imaging/steering.rs`,
  `crates/finder/src/imaging/solver.rs`, `crates/finder/src/imaging/run/script.rs`,
  `crates/scope-node`, `crates/scope-link`, `crates/simulator`, `proto/`, the plate
  solver's crates, the databases and `NudgeBlock.tsx` are never touched, and nothing is
  added under `crates/finder/tests/support/`. `crates/finder/src/session.rs` is written
  by US4 only. In `crates/finder/src/tracking.rs`, `crates/finder/src/nudge.rs`,
  `crates/finder/src/session.rs`, `crates/finder/src/imaging/run.rs` and
  `ImagingPage.tsx` only the changes of plan.md § Interfaces are made.
- Copy the words of spec.md's FRs and of plan.md § Interfaces exactly, the README's, the
  glossary's and the MCP tool's among them. Say "finder camera", never "finder" alone,
  in every message and label.
- No test downloads anything, and no test writes outside `CARGO_TARGET_TMPDIR`. Run no
  tool the gates don't run, and do not set `CARGO_TARGET_DIR`.
- The results said here before each story's implementation are the probe's
  (plan.md § What was run and what was read), where it ran them.

## Phase 1: User Story 1 - An imaging solve centres the target with Track's own step (Priority: P1)

### Tests first

- [ ] T001 [US1] Create `crates/finder/tests/motion/centre.rs` with US1-S1 under
  `#[tokio::test(start_paused = true)]`, building the centring rig, its
  two ways of putting the imaging camera's centre off M42, a solve and its rest as
  spec.md § User Scenarios defines them, and add `mod centre;` to
  `crates/finder/tests/motion/main.rs` in its alphabetical place. Make every item that
  US2, US3 and US4 use `pub(super)`, named in plan.md § Plan traps 2: `Rig` and its
  fields `app`, `state` and `sim`; its methods `new`, `now`, `call`, `get`, `wait`,
  `sleep`, `off`, `solve`, `track_on`, `rests`, `displace` and `held`; `Cause`; `sep`
  and `m42`; `CENTRE` and `FINDER_ARCSEC_PER_PX`. Before T002 to T005 every centring
  answers 405 and the test fails (run).

### Implementation

- [ ] T002 [US1] In `crates/finder/src/tracking.rs`, move `Tracker::nudge`'s body into
  `step_between`, its held place and moved place worked out by a `places` closure under
  its lock, have `nudge` call it with that closure, and add `Tracker::centre`, as plan.md
  § Interfaces gives them (FR-003).
- [ ] T003 [US1] In `crates/finder/src/nudge.rs`, move `track_press`'s body after its
  push check into `pub(crate) async fn track_step` with its tool and its `now`
  parameters, give `watch_step` the tool, and have `track_press` call it with `"nudge"`
  (FR-004).
- [ ] T004 [US1] Create `crates/finder/src/centre.rs` with its module doc, `AGE_S`,
  `CentreAsk` with no field, `CentreView`, `routes` and `centre` for the session's
  target, its checks in FR-002's order and the step from the sky at the set point
  (`set_point_in_frame` and `sky_at`), as plan.md § Interfaces gives them (FR-001 to
  FR-005); add `pub mod centre;` to `crates/finder/src/lib.rs` after `pub mod catalog;`
  (`crates/finder/src/lib.rs:86`).
- [ ] T005 [US1] In `crates/finder/src/api.rs`, add `routes = routes.merge(crate::centre::routes());`
  after the nudge's merge (`crates/finder/src/api.rs:667`) and `"centre"` after
  `"nudge"` in `start_motion`'s reserved list (`crates/finder/src/api.rs:846`) (FR-004).
- [ ] T006 [US1] Run the `lint` and `test` gates with every existing test unchanged and
  green, then commit the green story (FR-001 to FR-005; US1-S1).

## Phase 2: User Story 2 - A tap on the solved frame centres that point (Priority: P1)

### Tests first

- [ ] T007 [US2] Create `crates/finder/tests/motion/centre_tap.rs` with US2-S1 to US2-S5
  on US1's rig from `super::centre` and 055's `write` from `super::nudge`, with its own
  `send` (055's is private) and its own copy of PARTS and of the refusals' words, and
  add `mod centre_tap;` to `crates/finder/tests/motion/main.rs` in its alphabetical
  place. Before T008 a tap
  answers 400 and US2-S1 and US2-S2 fail; US2-S3, US2-S4 and US2-S5 pass from US1 on
  (plan.md § Plan traps 4) (run).

### Implementation

- [ ] T008 [US2] In `crates/finder/src/centre.rs`, add `CentreAsk`'s `frame_id`, `x` and
  `y`, `PARTS`, the ask's parts check, the point's frame and bounds checks and the
  sky at the pixel by `ImagingSolve::from_cd` and `sky_at`, as plan.md § Interfaces
  gives them (FR-006, FR-007).
- [ ] T009 [US2] Run the `lint` and `test` gates with every existing test unchanged and
  green, then commit the green story (FR-006, FR-007; US2-S1, US2-S2, US2-S3, US2-S4,
  US2-S5).

## Phase 3: User Story 3 - The imaging run centres from an imaging solve alone, and a stack holds it (Priority: P1)

### Tests first

- [ ] T010 [US3] Create `crates/finder/tests/motion/centre_run.rs` with US3-S1 to US3-S4,
  US3-S1, US3-S2 and US3-S4 on US1's rig from `super::centre` and US3-S3 on 037's
  script with its own wrapped mount part (plan.md § Plan traps 9), and add
  `mod centre_run;` to `crates/finder/tests/motion/main.rs` in its alphabetical place.
  Before T011 and T012 the file does not compile (no `MountSteps::centre`); US3-S1's
  push and aim cases each stack with tries 0, and US3-S2 passes (plan.md § Plan traps 4)
  (run).

### Implementation

- [ ] T011 [US3] In `crates/finder/src/imaging/run.rs`, add `MountSteps::centre` with its
  default, the trait doc's sentence, and the procedure's step after the re-centre
  branch with `measured`, as plan.md § Interfaces gives them (FR-008, FR-009).
- [ ] T012 [US3] In `crates/finder/src/imaging/run/console.rs`, add
  `ConsoleMount::centre` after its `steady_between`, answering `Stopped` after the
  console's Stop, as plan.md § Interfaces gives it (FR-008).
- [ ] T013 [US3] Run the `lint` and `test` gates with every existing test unchanged and
  green, then commit the green story (FR-008, FR-009; US3-S1, US3-S2, US3-S3, US3-S4).

## Phase 4: User Story 4 - The finder camera's next solve takes the aim point from a centring (Priority: P1)

### Tests first

- [ ] T014 [US4] Create `crates/finder/tests/motion/centre_aim.rs` with US4-S1, US4-S2
  and US4-S3 on US1's rig from `super::centre`, its frames-running cause a helper of
  its own (plan.md § Plan traps 10), and add `mod centre_aim;` to
  `crates/finder/tests/motion/main.rs` in its alphabetical place. Before T015 and T016
  the three tests fail: at 390″ a push is read, the aim point is not measured, and
  `aim` changes at Track off (run).

### Implementation

- [ ] T015 [US4] In `crates/finder/src/session.rs`, add `TrackRun`'s `centred` with its
  `None`, `Session::centred`, and the aim reading in `record_live_pointing` after its
  held aim point's block and before it reads the direction, as plan.md § Interfaces
  gives them (FR-010).
- [ ] T016 [US4] In `crates/finder/src/centre.rs`, call
  `state.session().centred(now + AGE_S)` after the step is handed to Track (FR-010).
- [ ] T017 [US4] Run the `lint` and `test` gates with every existing test unchanged and
  green, then commit the green story (FR-010; US4-S1, US4-S2, US4-S3).

## Phase 5: User Story 5 - MCP and the README carry the centring (Priority: P2)

### Tests first

- [ ] T018 [US5] Create `crates/finder/tests/mcp/centre.rs` with US5-S1 and US5-S2 on
  the MCP rig as spec.md § User Scenarios defines it, and add `mod centre;` to
  `crates/finder/tests/mcp/main.rs` after `mod calibration;`. Before T019 and T020 the
  tool is not listed and the README has no paragraph or section, and both tests fail
  (run).

### Implementation

- [ ] T019 [US5] In `crates/finder/src/mcp/tools.rs`, add `CentreArguments` after
  `NudgeArguments` and the tool `centre` before `set_tracking`, with plan.md
  § Interfaces' description (FR-011).
- [ ] T020 [US5] In `crates/finder/README.md`, add the MCP paragraph, the API row and
  the section with plan.md § Interfaces' words, each where it says; in `GLOSSARY.md`,
  add **Centring from an imaging solve** after **Nudge** with FR-013's words (FR-012,
  FR-013).
- [ ] T021 [US5] Run the `lint` and `test` gates with every existing test unchanged and
  green, then commit the green story (FR-011 to FR-013; US5-S1, US5-S2).

## Phase 6: User Story 6 - The Centre block on the Imaging page (Priority: P2)

### Tests first

- [ ] T022 [US6] Create `crates/finder/web/src/components/CentreBlock.test.tsx` with
  US6-S1 to US6-S4 on the page rig and `ImagingPage`, and
  `crates/finder/web/e2e/solve-centre.spec.ts` with US6-S5 and US6-S6 on the simulated
  console, its 012 off-red helper kept local as `tracking.spec.ts` keeps it and its
  session put back at the end as `tracking.spec.ts` puts it back; add
  `"**/solve-centre.spec.ts"` to the default project's `testIgnore` and the simulate
  project's `testMatch` in `crates/finder/web/playwright.config.ts`, each after
  `"**/nudge.spec.ts"`. Before T023 the Vitest file cannot import `CentreBlock` and the
  browser test finds no Centre block (run).

### Implementation

- [ ] T023 [US6] Add `width` and `height` to `ImagingFrame`, and `Centred` and
  `postCentre` before `postRecord`, in `crates/finder/web/src/api.ts`; create
  `crates/finder/web/src/components/CentreBlock.tsx` as plan.md § Interfaces and
  FR-014 to FR-016 give it, its status held at `Centring …` until a session shows the
  step `moving`; render it in `crates/finder/web/src/components/ImagingPage.tsx` right
  after `ImagingSolveBlock`; and build `crates/finder/web/dist` with `npm run build`
  (FR-014 to FR-017).
- [ ] T024 [US6] Run the `lint` and `test` gates, `CI=1 npm run e2e` among them, with
  every existing test unchanged and green, then commit the green story with `dist`
  (FR-014 to FR-017; US6-S1, US6-S2, US6-S3, US6-S4, US6-S5, US6-S6).

## Execution Order

```text
US1 -> US2 -> US3 -> US4 -> US5 -> US6
```

- **US1** starts from a `main` that holds every spec of `depends_on_landed`, re-anchored
  on 052, 053, 054 and 055.
- **US2** starts once US1 has merged: it adds the tap to US1's `centre` module and its
  tests use US1's rig.
- **US3** starts once US2 has merged: its mount part calls `centre`, and its tests use
  US1's rig.
- **US4** starts once US3 has merged: it marks the centring in US1's `centre` and its
  tests use US1's rig.
- **US5** starts once US4 has merged: its tool reads the ask US2 completes, and its
  words name US4's aim point.
- **US6** starts once US5 has merged: the page posts the tap, and its browser test needs
  the route.
- **`[P]`** marks nothing here: each story's implementation follows its tests.
