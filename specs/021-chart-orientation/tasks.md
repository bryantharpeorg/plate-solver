---
description: "Task list for 021-chart-orientation: A Chart Turned to the Sky"
---

# Tasks: A Chart Turned to the Sky

**Prerequisites** (plan.md § Prerequisites):
- `012-targets-and-push-to`, `013-live-finder`, `016-mcp`, `017-console-stop-confirm`,
  `018-live-chart-steady`, `019-night-legibility`, `020-live-chart-calm` and
  `022-altaz-guidance` have landed on `main`;
- the owner approvals in spec.md are given;
- no operator-staged file is needed.

**Rules for every story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml`,
  `CODEOWNERS` or `ergane.yaml`. If an existing test pins the old behaviour, stop and
  escalate with its exact output. Do not edit it.
- Write only the files your story's tasks name; they are its `writes` in plan.md
  § Refinement Record. A story that needs another file escalates.
- Taps, the aim point and every pixel sent to the server stay in frame coordinates.
  Leave the solver, the guidance, the scope link and uploads' and samples' charts alone.
- Run no tool the gates don't run, except `npm run format` in `crates/finder/web` to
  apply the formatting that `npm run format:check` checks. No `cargo deny`,
  `cargo machete`, Rust 1.96 or `ergane`.

## Phase 1: User Story 1 — The console keeps the camera's angle (Priority: P1)

### Tests first

- [ ] T001 [US1] Create `crates/finder/tests/session_rotation.rs` with failing tests for
  US1-S1, US1-S2, US1-S3 and US1-S4, restarting on a state folder as
  `crates/finder/tests/session_cli.rs` does and calling the MCP tools as
  `crates/finder/tests/mcp_session_tools.rs` does, with its own copies of their helpers.
- [ ] T002 [US1] Create `crates/finder/web/src/components/SetupRotation.test.tsx` with
  failing tests for US1-S5, US1-S6 and US1-S7, rendering `SetupPage` with a mocked `put`
  as `crates/finder/web/src/components/SetupPage.test.tsx` does, and `useSession` with a
  mocked `fetch`.

### Implementation

- [ ] T003 [US1] In `crates/finder/src/session.rs`, add `chart_rotation_deg` to
  `SessionEdit`, `SessionView`, `Values`, `StoredSession`, `edited`, `read_state` and
  `write_state`, exactly as in plan.md § Interfaces, and name it in both `SessionEdit`
  patterns in `crates/finder/src/mcp/tools.rs` so both tools refuse it (FR-001, FR-002).
- [ ] T004 [US1] Add `chart_rotation_deg?` to `ObservingSession` in
  `crates/finder/web/src/api.ts`; add it to `SessionEdit` and keep it in `request()` in
  `crates/finder/web/src/useSession.ts`; and add the "Chart rotation" section to
  `crates/finder/web/src/components/SetupPage.tsx` after Minimum altitude (FR-003,
  FR-004).
- [ ] T005 [US1] Add the Chart rotation entry to `GLOSSARY.md`, the paragraph to
  `crates/finder/README.md`, and the "Chart rotation — built" section to `DESIGN.md`,
  with plan.md's text (FR-010).
- [ ] T006 [US1] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the rebuilt `crates/finder/web/dist`. Commit the green story.

## Phase 2: User Story 2 — The live views turn by it (Priority: P1)

### Tests first

- [ ] T007 [US2] Create `crates/finder/web/src/labelsRotation.test.ts` with a failing
  test for US2-S4, calling `layoutLabels` directly with `WIDE` and checking the spots
  against `toView` and `fromView`.
- [ ] T008 [US2] Create `crates/finder/web/src/components/ChartRotation.test.tsx` with
  failing tests for US2-S1, US2-S2, US2-S3, US2-S5, US2-S6, US2-S7 and US2-S8, building
  `WIDE` from `ORION2` and mocking the photo's bounding box as
  `crates/finder/web/src/components/AimTap.test.tsx` does.
- [ ] T009 [US2] Create `crates/finder/web/src/App.rotation.test.tsx` with a failing test
  for US2-S9, with its own copy of the `FakeEventSource`, `CONNECTED` and `serve` harness
  from `crates/finder/web/src/App.live.test.tsx`.

### Implementation

- [ ] T010 [US2] Create `crates/finder/web/src/rotation.ts` with `rotationOf`,
  `rotatedBox`, `rotationTransform`, `toView`, `fromView` and `boxToFrame`, and give
  `layoutLabels` in `crates/finder/web/src/labels.ts` its `rotation` input, exactly as in
  plan.md § Interfaces (FR-005, FR-006).
- [ ] T011 [US2] In `crates/finder/web/src/components/Chart.tsx`, size the chart from the
  rotated box, draw the frame in the rotated group with the paper's edge outside it, pass
  `rotation` to `layoutLabels`, and give the grid's edge labels their transforms; add the
  upright-label rule to `crates/finder/web/src/index.css` (FR-005, FR-006, FR-009).
- [ ] T012 [US2] Rotate the photos in `crates/finder/web/src/components/LivePreview.tsx`
  and `crates/finder/web/src/components/AimTap.tsx`, with the tap through `boxToFrame`;
  rotate the arrows in `crates/finder/web/src/components/NextTargets.tsx`; and in
  `crates/finder/web/src/App.tsx` size the page from the rotated box and pass the
  rotation and the frame's size on (FR-007, FR-008, FR-009).
- [ ] T013 [US2] Append plan.md's paragraph to `DESIGN.md` § Chart rotation (FR-010).
- [ ] T014 [US2] Run the `lint` and `test` gates with every existing Vitest and
  Playwright test unchanged and green. Commit the rebuilt `crates/finder/web/dist`.
  Commit the green story.

## Execution Order

```text
US1 → US2 (US2 waits for US1 to merge)
```

- **US1** touches `crates/finder/src/session.rs`, `crates/finder/src/mcp/tools.rs`, the
  README, the page's session and Setup, `GLOSSARY.md` and `DESIGN.md`.
- **US2** touches only `crates/finder/web/` and `DESIGN.md`. It reads the field US1 adds.
- Both rebuild `dist`, so the edge is a merge edge.
