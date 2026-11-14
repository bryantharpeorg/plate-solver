---
description: "Task list for 022-altaz-guidance: Push-To in the Observer's Own Directions"
---

# Tasks: Push-To in the Observer's Own Directions

**Prerequisites** (plan.md § Prerequisites):
- `013-live-finder`, `015-tracking`, `016-mcp`, `017-console-stop-confirm`,
  `018-live-chart-steady`, `019-night-legibility` and `020-live-chart-calm` have landed
  on `main`;
- the owner approvals in spec.md are given;
- no operator-staged file is needed.

**Rules for every story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml`,
  `CODEOWNERS` or `ergane.yaml`. If an existing test pins the old behaviour, stop and
  escalate with its exact output. Do not edit it.
- Write only the files your story's tasks name; they are its `writes` in plan.md
  § Refinement Record. A story that needs another file escalates.
- Add no field to `session::Pointing`. Leave `turn_deg`, `tilt_deg`, the target's
  placement, the solver and the scope link alone.
- The line `turn clockwise 34° · tilt up 12°` keeps its exact words.
- Run no tool the gates don't run, except `npm run format` in `crates/finder/web` to
  apply the formatting that `npm run format:check` checks. No `cargo deny`,
  `cargo machete`, Rust 1.96 or `ergane`.

## Phase 1: User Story 1 — The push in the observer's own directions (Priority: P1)

### Tests first

- [ ] T001 [US1] Create `crates/finder/tests/pushto_separation.rs` with failing tests for
  US1-S7, US1-S8 and US1-S9, on a fixed clock with a solved sample as
  `crates/finder/tests/pushto_api.rs` does, and live pointings 5.0 s and 5.1 s old as
  `crates/finder/tests/freshness_api.rs` builds them.
- [ ] T002 [US1] Create `crates/finder/web/src/components/GuidancePush.test.tsx` with
  failing tests for US1-S1, US1-S2, US1-S3, US1-S4, US1-S5 and US1-S6, using `SESSION`
  from `crates/finder/web/src/test/targets.ts` and rebuilding inline the five sessions
  `crates/finder/web/src/components/GuidanceBlock.test.tsx` builds without directions.

### Implementation

- [ ] T003 [US1] In `crates/finder/src/pushto.rs`, add `separation_deg` to `Guidance`,
  bind the pointing in the `if let` that sets turn and tilt, set the new field there and
  clear it where they are cleared, exactly as in plan.md § Interfaces (FR-002). Add
  `separation_deg?` to `Guidance` in `crates/finder/web/src/api.ts`.
- [ ] T004 [US1] In `crates/finder/web/src/components/GuidanceBlock.tsx`, print the push
  line and "away" above the old line and set the old line quiet, with the `tenths` and
  `push` functions, exactly as in plan.md § Interfaces (FR-001, FR-003, FR-004).
- [ ] T005 [US1] Replace the Turn entry in `GLOSSARY.md`, the sentence in `DESIGN.md`
  § Guidance block, and add the Push step to `DESIGN.md` § Typography › Hierarchy, with
  plan.md's text (FR-011).
- [ ] T006 [US1] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the rebuilt `crates/finder/web/dist`. Commit the green story.

## Phase 2: User Story 2 — The chart marks where the telescope looks (Priority: P1)

- [ ] T007 [US2] Create `crates/finder/web/src/components/ChartAim.test.tsx` with failing
  tests for US2-S1, US2-S2, US2-S3 and US2-S4, rendering `Chart` with a helper of its own
  that passes `live` and `session`, and the aim (320.5, 190) inside `ORION2`.
- [ ] T008 [US2] Add `AimMark` to `crates/finder/web/src/components/Chart.tsx` after the
  furniture, and its rules to `crates/finder/web/src/index.css` between the `.furniture`
  and `.furniture text` rules, exactly as in plan.md § Interfaces (FR-005, FR-006).
- [ ] T009 [US2] Add plan.md's "Aim mark" section to `DESIGN.md` after § Target reticle
  (FR-011).
- [ ] T010 [US2] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the rebuilt `crates/finder/web/dist`. Commit the green story.

## Phase 3: User Story 3 — The chart shows which way is up (Priority: P2)

- [ ] T011 [US3] Create `crates/finder/tests/pointing_up.rs` with failing tests for
  US3-S1, US3-S2, US3-S3 and US3-S4, taking the live event for US3-S4 as
  `crates/finder/tests/live/support.rs` does; `crates/finder/web/src/useLiveUp.test.ts`
  for US3-S5; and `crates/finder/web/src/components/ChartUp.test.tsx` for US3-S6,
  US3-S7, US3-S8 and US3-S9.
- [ ] T012 [US3] Add `up_deg` to `crates/finder/src/pushto.rs`, and the `up_deg` field to
  `LatestPointing` in `crates/finder/src/session.rs`, built in the view, exactly as in
  plan.md § Interfaces (FR-007).
- [ ] T013 [US3] Add `up_deg` to `LiveReport` in `crates/finder/src/live.rs` from the
  snapshot the loop already takes; add `up_deg?` to `LiveReport` and `SessionPointing` in
  `crates/finder/web/src/api.ts`; keep it on the rebuilt live pointing in
  `crates/finder/web/src/useLive.ts`; and list it among the live event's fields in
  `crates/finder/README.md` (FR-008, FR-011).
- [ ] T014 [US3] Add `UpMark` to `crates/finder/web/src/components/Chart.tsx` after the
  aim mark, under the condition plan.md gives, with both labels `pointer-events-none`;
  its rules to `crates/finder/web/src/index.css`; and plan.md's "Up mark" section to
  `DESIGN.md` (FR-009, FR-010, FR-011).
- [ ] T015 [US3] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the rebuilt `crates/finder/web/dist`. Commit the green story.

## Execution Order

```text
US1 → US2 → US3 (each waits for the one before it to merge)
```

- **US1** touches `crates/finder/src/pushto.rs`, the guidance block, `GLOSSARY.md` and
  `DESIGN.md`.
- **US2** touches `Chart.tsx`, `index.css` and `DESIGN.md`.
- **US3** touches `pushto.rs`, `session.rs`, `live.rs`, the README, the live hook,
  `Chart.tsx`, `index.css` and `DESIGN.md`.
- Every story rebuilds `dist`, so every edge is a merge edge.
