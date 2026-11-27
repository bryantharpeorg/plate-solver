---
description: "Task list for 055-nudge-arrows-follow-the-frame: The Nudge Arrows Follow the Frame"
---

# Tasks: The Nudge Arrows Follow the Frame

**Prerequisites** (plan.md § Prerequisites, each with its evidence there):
- every spec in `depends_on_landed` has landed on `main`: met at f6d4f5fd;
- 052, 053 and 054 have landed, in that order, and this trio is re-anchored on that
  `main`: not met;
- the owner decisions in spec.md are settled: not met, none of the ten is answered;
- every story has been probe-built from this text: not met (the fold of 2026-11-27 built
  the five stories beside this text, not from it);
- the page half has run once: met on the probe (`CI=1 npm run e2e`, 37 of 37), to be run
  again on the re-anchored base;
- no operator-staged file is needed: met.

**Rules for every story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml`,
  `CODEOWNERS`, `ergane.yaml` or `.specify/`. If an existing test fails, stop and
  escalate with its exact output. A test file an earlier story of this spec created is
  an existing test; US1's test helpers are `pub(super)` from the start so later stories
  use them without editing it (plan.md § Plan traps 3). The exceptions: each story's
  `mod` lines in `crates/finder/tests/motion/main.rs` or
  `crates/finder/tests/mcp/main.rs`, each in its alphabetical place, and US5's two list
  entries in `crates/finder/web/playwright.config.ts` (spec owner approval 9).
- Write only the files your story's tasks name; they are its `writes` in plan.md
  § Refinement Record. A story that needs another file escalates.
- `crates/finder/src/model.rs`, `crates/finder/src/live.rs`,
  `crates/finder/src/session.rs`, `crates/finder/src/imaging/`, `crates/scope-node`,
  `crates/scope-link`, `crates/simulator`, `proto/`, the plate solver's crates, the
  databases, `TrackControl.tsx`, `GoToControl.tsx` and `McpNote.tsx` are never touched,
  and nothing is added under `crates/finder/tests/support/`. In
  `crates/finder/src/goto.rs` and `ImagingPage.tsx` only the additions of plan.md
  § Interfaces are made.
- Copy the words of spec.md's FRs and of plan.md § Interfaces exactly, the README's and
  the glossary's among them. Say "finder camera", never "finder" alone, in every
  message and label.
- No test downloads anything, and no test writes outside `CARGO_TARGET_TMPDIR`. Run no
  tool the gates don't run, and do not set `CARGO_TARGET_DIR`.
- The results said here before each story's implementation are the probe's
  (plan.md § What was run and what was read), where it ran them.

## Phase 1: User Story 1 - With Track off, the arrows move the telescope one step in the mount's view (Priority: P1)

### Tests first

- [ ] T001 [US1] Create `crates/finder/tests/motion/nudge.rs` with one test for each of
  US1-S1 to US1-S5 under `#[tokio::test(start_paused = true)]`, building the motion
  rig, the synced rig and the bare console as spec.md § User Scenarios defines them,
  with the steps' sizes and the ways' angles written in the test and its helpers
  `pub(super)` (plan.md § Plan traps 3), and add `mod nudge;` to
  `crates/finder/tests/motion/main.rs` in its alphabetical place. Before T002 to T005
  every press answers 405 and every test fails (run).

### Implementation

- [ ] T002 [US1] In `crates/finder/src/goto.rs`, add `AxisMove::Offset` and
  `AxisMove::Through` after `Rate`, the two arms of `Planner::direct` with the zenith
  refusal, and the helper `through` after `direct`, as plan.md § Interfaces gives them
  (FR-006, FR-009).
- [ ] T003 [US1] In `crates/finder/src/mcp/guard.rs`, add `start_by` and make `start`
  call it with `"mcp"`; send `Through`'s first waypoint in `begin` and its last from the
  watcher `path` (FR-006, FR-007).
- [ ] T004 [US1] Create `crates/finder/src/nudge.rs` with its module doc,
  `FINE_ARCSEC`, `COARSE_ARCSEC`, `Direction`, `Step`, `View` (`Mount` only),
  `NudgeAsk`, `NudgeView`, `offset`, `routes` and `nudge` with no Track branch, as
  plan.md § Interfaces gives them (FR-001 to FR-003, FR-013); add `pub mod nudge;` to
  `crates/finder/src/lib.rs` after `pub mod mount_sky;` (`crates/finder/src/lib.rs:95`).
- [ ] T005 [US1] In `crates/finder/src/api.rs`, add `"nudge"` and `"track"` to
  `start_motion`'s reserved list (`crates/finder/src/api.rs:835`) and
  `routes = routes.merge(crate::nudge::routes());` after the `finder_focus` merge
  (`crates/finder/src/api.rs:661`) (FR-008).
- [ ] T006 [US1] Run the `lint` and `test` gates with every existing test unchanged and
  green, then commit the green story (FR-001 to FR-003, FR-006 to FR-009, FR-013;
  US1-S1, US1-S2, US1-S3, US1-S4, US1-S5).

## Phase 2: User Story 2 - The arrows follow the finder camera's or the imaging camera's frame (Priority: P1)

### Tests first

- [ ] T007 [US2] Create `crates/finder/tests/motion/nudge_views.rs` with US2-S1 and
  US2-S3 under `#[tokio::test(start_paused = true)]`, on US1's helpers, building the
  imaging rig as spec.md § User Scenarios defines it, and add `mod nudge_views;` to
  `crates/finder/tests/motion/main.rs` in its alphabetical place. Before T008 both
  camera views answer 400 (run).

### Implementation

- [ ] T008 [US2] In `crates/finder/src/nudge.rs`, add `View::FinderCamera` and
  `View::ImagingCamera` and their arms of `offset`, `Kept` and `frame_axes`, and the
  tests module with US2-S2, US2-S4 and US2-S5 under `#[test]` on the unit session and
  the solved imaging views spec.md defines; in `crates/finder/src/api.rs`, add
  `AppState::nudge_kept`, as plan.md § Interfaces gives them (FR-004, FR-005).
- [ ] T009 [US2] Run the `lint` and `test` gates with every existing test unchanged and
  green, then commit the green story (FR-004, FR-005; US2-S1, US2-S2, US2-S3, US2-S4,
  US2-S5).

## Phase 3: User Story 3 - With Track on, the place Track holds moves by the step and Track makes it (Priority: P1)

### Tests first

- [ ] T010 [US3] Create `crates/finder/tests/motion/nudge_track.rs` with US3-S1 to
  US3-S6 under `#[tokio::test(start_paused = true)]`, on US1's helpers, building the
  Track rig as spec.md § User Scenarios defines it, and add `mod nudge_track;` to
  `crates/finder/tests/motion/main.rs` in its alphabetical place. Before T011 and T012
  every Track press answers 409 `the mount is already moving` or meets no refusal it is
  due (run).

### Implementation

- [ ] T011 [US3] In `crates/finder/src/tracking.rs`, add `Control::axes`,
  `Control::stepping` and `Control::quiet_until`, `Tracker::nudge`, `may_nudge` and
  `stepping`, `Shared::sky_step`, `step_path` and `step`, the free `moved`, the rate
  loop's step, and the `pointing`, `checks_push`, `pause` and steady-mark changes, as
  plan.md § Interfaces gives them (FR-010, FR-011, FR-012).
- [ ] T012 [US3] In `crates/finder/src/nudge.rs`, add `PUSH_QUIET_S`, the push-settling
  refusal, the Track branch of `nudge` and its watcher (FR-010, FR-012).
- [ ] T013 [US3] Run the `lint` and `test` gates with every existing test unchanged and
  green, then commit the green story (FR-010 to FR-012; US3-S1, US3-S2, US3-S3, US3-S4,
  US3-S5, US3-S6).

## Phase 4: User Story 4 - MCP and the README carry the nudge (Priority: P2)

### Tests first

- [ ] T014 [US4] Create `crates/finder/tests/mcp/nudge.rs` with US4-S1 and US4-S2 under
  `#[tokio::test(start_paused = true)]` and US4-S3 under `#[test]`, on the MCP rig as
  spec.md § User Scenarios defines it, and add `mod nudge;` to
  `crates/finder/tests/mcp/main.rs` in its alphabetical place. Before T015 and T016 the
  tool is not listed and the README test fails (run).

### Implementation

- [ ] T015 [US4] In `crates/finder/src/mcp/tools.rs`, add `NudgeArguments` after
  `FinderFocusArguments` and the tool `nudge` after `move_axis`, with plan.md
  § Interfaces' description (FR-014).
- [ ] T016 [US4] In `crates/finder/README.md`, add the MCP paragraph, the API row and
  the section of plan.md § Interfaces in their places; in `GLOSSARY.md`, change
  **Direct axis move** and add **Nudge** after it with FR-016's words (FR-015,
  FR-016).
- [ ] T017 [US4] Run the `lint` and `test` gates with every existing test unchanged and
  green, then commit the green story (FR-014 to FR-016; US4-S1, US4-S2, US4-S3).

## Phase 5: User Story 5 - The Nudge block in the guidance block and on the Imaging page (Priority: P2)

### Tests first

- [ ] T018 [US5] Create `crates/finder/web/src/components/NudgeBlock.test.tsx` with
  US5-S1 to US5-S4 on the page rig and `ImagingPage`, and
  `crates/finder/web/e2e/nudge.spec.ts` with US5-S5, US5-S6 and US5-S7 on the simulated
  console, its 012 off-red helper kept local as `tracking.spec.ts` keeps it and its
  session put back at the end as `tracking.spec.ts` puts it back; add
  `"**/nudge.spec.ts"` to the default project's `testIgnore` and the simulate project's
  `testMatch` in `crates/finder/web/playwright.config.ts`, each after
  `"**/tools-focus.spec.ts"`. Before T019 the Vitest file finds no region `Nudge` and
  the first browser test finds no Nudge block (run).

### Implementation

- [ ] T019 [US5] Add `axis_move` to `ObservingSession` and `postNudge` with its types
  before `postRecord` in `crates/finder/web/src/api.ts`; copy `axis_move` in
  `crates/finder/web/src/useSession.ts`; create
  `crates/finder/web/src/components/NudgeBlock.tsx` as plan.md § Interfaces gives it;
  render it in `GuidanceBlock.tsx` after `TrackControl` and in `ImagingPage.tsx` after
  its imaging frame, with the two optional props; pass them from
  `crates/finder/web/src/App.tsx`; and build `dist` with `npm run build` (FR-017 to
  FR-020).
- [ ] T020 [US5] Run the `lint` and `test` gates, `CI=1 npm run e2e` among them, with
  every existing test unchanged and green, then commit the green story with `dist`
  (FR-017 to FR-020; US5-S1, US5-S2, US5-S3, US5-S4, US5-S5, US5-S6, US5-S7).

## Execution Order

```text
US1 -> US2 -> US3 -> US4 -> US5
```

- **US1** starts from a `main` that holds every spec of `depends_on_landed`, re-anchored
  on 052, 053 and 054.
- **US2** starts once US1 has merged: it adds views to US1's `nudge` module and its
  tests use US1's helpers.
- **US3** starts once US2 has merged: it adds Track's branch beside US2's views.
- **US4** starts once US3 has merged: its tool calls `nudge` with every view.
- **US5** starts once US4 has merged: the page posts the route, and its browser test
  needs Track's branch.
- **`[P]`** marks nothing here: each story's implementation follows its tests.
