---
description: "Task list for 057-imaging-solve-centres-the-target: An Imaging Solve Centres the Target"
---

# Tasks: An Imaging Solve Centres the Target

**Prerequisites** (plan.md § Prerequisites, each with its evidence there):
- every spec in `depends_on_landed` has landed on `main`: met at 9c4a0a15;
- 052, 053, 054 and 055 have landed, in that order, and this trio is re-anchored on that
  `main`: not met (this text is stacked on 055's probe code at 8b7212bb);
- the owner decisions in spec.md are settled: not met, none of the eight is answered;
- every story has been probe-built from this text: not met (the probe of 2026-11-27
  built the five stories beside this text, not from it);
- the page half has run once: met on the probe (`CI=1 npm run e2e`, 40 of 40), to be run
  again on the re-anchored base;
- no operator-staged file is needed: met.

**Rules for every story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml`,
  `CODEOWNERS`, `ergane.yaml` or `.specify/`. If an existing test fails, stop and
  escalate with its exact output. A test file an earlier story of this spec created is
  an existing test; US1's rig and helpers are `pub(super)` from the start so later
  stories use them without editing it (plan.md § Plan traps 2). The exceptions: each
  story's `mod` lines in `crates/finder/tests/motion/main.rs` or
  `crates/finder/tests/mcp/main.rs`, each in its alphabetical place, and US5's two list
  entries in `crates/finder/web/playwright.config.ts` (spec owner approval 9).
- Write only the files your story's tasks name; they are its `writes` in plan.md
  § Refinement Record. A story that needs another file escalates.
- `crates/finder/src/model.rs`, `crates/finder/src/live.rs`,
  `crates/finder/src/session.rs`, `crates/finder/src/goto.rs`,
  `crates/finder/src/imaging/steering.rs`, `crates/finder/src/imaging/solver.rs`,
  `crates/finder/src/imaging/run/script.rs`, `crates/scope-node`, `crates/scope-link`,
  `crates/simulator`, `proto/`, the plate solver's crates, the databases and
  `NudgeBlock.tsx` are never touched, and nothing is added under
  `crates/finder/tests/support/`. In `crates/finder/src/tracking.rs`,
  `crates/finder/src/nudge.rs`, `crates/finder/src/imaging/run.rs` and `ImagingPage.tsx`
  only the changes of plan.md § Interfaces are made.
- Copy the words of spec.md's FRs and of plan.md § Interfaces exactly, the README's and
  the glossary's among them. Say "finder camera", never "finder" alone, in every
  message and label.
- No test downloads anything, and no test writes outside `CARGO_TARGET_TMPDIR`. Run no
  tool the gates don't run, and do not set `CARGO_TARGET_DIR`.
- The results said here before each story's implementation are the probe's
  (plan.md § What was run and what was read), where it ran them.

## Phase 1: User Story 1 - An imaging solve centres the target with Track's own step (Priority: P1)

### Tests first

- [ ] T001 [US1] Create `crates/finder/tests/motion/centre.rs` with US1-S1 and US1-S2
  under `#[tokio::test(start_paused = true)]`, building the centring rig, its two ways
  of putting the imaging camera's centre off M42, a solve and its rest as spec.md
  § User Scenarios defines them, with `Rig`, `Cause`, `refused`, `sep`, `m42` and
  `CENTRE` `pub(super)` (plan.md § Plan traps 2), and add `mod centre;` to
  `crates/finder/tests/motion/main.rs` in its alphabetical place. Before T002 to T005
  every centring answers 405 and both tests fail (run).

### Implementation

- [ ] T002 [US1] In `crates/finder/src/tracking.rs`, move `Tracker::nudge`'s body from
  the axes' read onwards into `step_between`, have `nudge` work out its moved place and
  call it, and add `Tracker::centre`, as plan.md § Interfaces gives them (FR-003).
- [ ] T003 [US1] In `crates/finder/src/nudge.rs`, move the Track branch of `nudge` into
  `pub(crate) async fn track_step` with its tool a parameter, and have `nudge` call it
  with `"nudge"` (FR-004).
- [ ] T004 [US1] Create `crates/finder/src/centre.rs` with its module doc, `CentreAsk`
  with no field, `CentreView`, `routes` and `centre` for the session's target, its
  checks in FR-002's order, as plan.md § Interfaces gives them (FR-001 to FR-005); add
  `pub mod centre;` to `crates/finder/src/lib.rs` after `pub mod catalog;`
  (`crates/finder/src/lib.rs:86`).
- [ ] T005 [US1] In `crates/finder/src/api.rs`, add `routes = routes.merge(crate::centre::routes());`
  after the nudge's merge (`crates/finder/src/api.rs:665`) and `"centre"` after
  `"nudge"` in `start_motion`'s reserved list (`crates/finder/src/api.rs:844`) (FR-004).
- [ ] T006 [US1] Run the `lint` and `test` gates with every existing test unchanged and
  green, then commit the green story (FR-001 to FR-005; US1-S1, US1-S2).

## Phase 2: User Story 2 - A tap on the solved frame centres that point (Priority: P1)

### Tests first

- [ ] T007 [US2] Create `crates/finder/tests/motion/centre_tap.rs` with US2-S1 and US2-S2
  on US1's rig from `super::centre` and 055's `send` from `super::nudge`, with its own
  copy of PARTS, and add `mod centre_tap;` to `crates/finder/tests/motion/main.rs` in
  its alphabetical place. Before T008 a tap answers 400 and both tests fail (run).

### Implementation

- [ ] T008 [US2] In `crates/finder/src/centre.rs`, add `CentreAsk`'s `frame_id`, `x` and
  `y`, `PARTS`, the ask's parts check, the point's frame and bounds checks and the
  sky at the pixel by `ImagingSolve::from_cd` and `sky_at`, as plan.md § Interfaces
  gives them (FR-006, FR-007).
- [ ] T009 [US2] Run the `lint` and `test` gates with every existing test unchanged and
  green, then commit the green story (FR-006, FR-007; US2-S1, US2-S2).

## Phase 3: User Story 3 - The imaging run centres from an imaging solve alone, and a stack holds it (Priority: P1)

### Tests first

- [ ] T010 [US3] Create `crates/finder/tests/motion/centre_run.rs` with US3-S1 and US3-S2
  on US1's rig from `super::centre`, and add `mod centre_run;` to
  `crates/finder/tests/motion/main.rs` in its alphabetical place. Before T011 and T012
  US3-S1 fails (the run stacks 387″ off with tries 0 and the warning) and US3-S2 passes
  (plan.md § Plan traps 4) (run).

### Implementation

- [ ] T011 [US3] In `crates/finder/src/imaging/run.rs`, add `MountSteps::centre` with its
  default, the trait doc's sentence, and the procedure's step after the re-centre
  branch with `measured`, as plan.md § Interfaces gives them (FR-008, FR-009).
- [ ] T012 [US3] In `crates/finder/src/imaging/run/console.rs`, add
  `ConsoleMount::centre` after its `steady_between`, as plan.md § Interfaces gives it
  (FR-008).
- [ ] T013 [US3] Run the `lint` and `test` gates with every existing test unchanged and
  green, then commit the green story (FR-008, FR-009; US3-S1, US3-S2).

## Phase 4: User Story 4 - MCP and the README carry the centring (Priority: P2)

### Tests first

- [ ] T014 [US4] Create `crates/finder/tests/mcp/centre.rs` with US4-S1 and US4-S2 on
  the MCP rig as spec.md § User Scenarios defines it, and add `mod centre;` to
  `crates/finder/tests/mcp/main.rs` after `mod calibration;`. Before T015 and T016 the
  tool is not listed and the README has no paragraph or section, and both tests fail
  (run).

### Implementation

- [ ] T015 [US4] In `crates/finder/src/mcp/tools.rs`, add `CentreArguments` after
  `NudgeArguments` and the tool `centre` before `set_tracking`, with plan.md
  § Interfaces' description (FR-010).
- [ ] T016 [US4] In `crates/finder/README.md`, add the MCP paragraph, the API row and
  the section with plan.md § Interfaces' words, each where it says; in `GLOSSARY.md`,
  add **Centring** after **Nudge** with FR-012's words (FR-011, FR-012).
- [ ] T017 [US4] Run the `lint` and `test` gates with every existing test unchanged and
  green, then commit the green story (FR-010 to FR-012; US4-S1, US4-S2).

## Phase 5: User Story 5 - The Centre block on the Imaging page (Priority: P2)

### Tests first

- [ ] T018 [US5] Create `crates/finder/web/src/components/CentreBlock.test.tsx` with
  US5-S1 to US5-S4 on the page rig and `ImagingPage`, and
  `crates/finder/web/e2e/solve-centre.spec.ts` with US5-S5 and US5-S6 on the simulated
  console, its 012 off-red helper kept local as `tracking.spec.ts` keeps it and its
  session put back at the end as `tracking.spec.ts` puts it back; add
  `"**/solve-centre.spec.ts"` to the default project's `testIgnore` and the simulate
  project's `testMatch` in `crates/finder/web/playwright.config.ts`, each after
  `"**/nudge.spec.ts"`. Before T019 the Vitest file cannot import `CentreBlock` (run).

### Implementation

- [ ] T019 [US5] Add `width` and `height` to `ImagingFrame`, and `Centred` and
  `postCentre` before `postRecord`, in `crates/finder/web/src/api.ts`; create
  `crates/finder/web/src/components/CentreBlock.tsx` as plan.md § Interfaces and
  FR-013 to FR-015 give it; render it in
  `crates/finder/web/src/components/ImagingPage.tsx` right after `ImagingSolveBlock`;
  and build `crates/finder/web/dist` with `npm run build` (FR-013 to FR-016).
- [ ] T020 [US5] Run the `lint` and `test` gates, `CI=1 npm run e2e` among them, with
  every existing test unchanged and green, then commit the green story with `dist`
  (FR-013 to FR-016; US5-S1, US5-S2, US5-S3, US5-S4, US5-S5, US5-S6).

## Execution Order

```text
US1 -> US2 -> US3 -> US4 -> US5
```

- **US1** starts from a `main` that holds every spec of `depends_on_landed`, re-anchored
  on 052, 053, 054 and 055.
- **US2** starts once US1 has merged: it adds the tap to US1's `centre` module and its
  tests use US1's rig.
- **US3** starts once US2 has merged: its mount part calls `centre`, and its tests use
  US1's rig.
- **US4** starts once US3 has merged: its tool reads the ask US2 completes.
- **US5** starts once US4 has merged: the page posts the tap, and its browser test needs
  the route.
- **`[P]`** marks nothing here: each story's implementation follows its tests.
