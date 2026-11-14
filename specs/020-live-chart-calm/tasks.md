---
description: "Task list for 020-live-chart-calm: A Live Chart That Stays Calm"
---

# Tasks: A Live Chart That Stays Calm

**Prerequisites** (plan.md § Prerequisites):
- `013-live-finder`, `017-console-stop-confirm`, `018-live-chart-steady` and
  `019-night-legibility` have landed on `main`;
- the owner approvals in spec.md are given;
- no operator-staged file is needed.

**Rules for every story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml`,
  `CODEOWNERS` or `ergane.yaml`. If an existing test pins the old behaviour, stop and
  escalate with its exact output. Do not edit it.
- Write only the files your story's tasks name; they are its `writes` in plan.md
  § Refinement Record. A story that needs another file escalates.
- Leave the solver, its inputs, `matched`, `pointing`, the guidance, the model and the
  scope link alone.
- Run no tool the gates don't run, except `npm run format` in `crates/finder/web` to
  apply the formatting that `npm run format:check` checks. No `cargo deny`,
  `cargo machete`, Rust 1.96 or `ergane`.

## Phase 1: User Story 1 — The report says which catalog stars were detected (Priority: P1)

### Tests first

- [ ] T001 [US1] Create `crates/finder/tests/star_detected.rs` with failing tests for
  US1-S1 (detected marks and their 2 px detections), US1-S2 (both solve with the two
  brightest detections removed; the 60 brightest at least 2 px inside the edges keep their
  ids and move < 1 px), US1-S3
  (`matched` stays 30) and US1-S4 (unmatched marks are exactly the 60 brightest less the
  matched), building the detections and the 60 brightest as plan.md § Interfaces "the
  test harness" says.

### Implementation

- [ ] T002 [US1] In `crates/finder/src/solve.rs`, rename `star_marks`' local `detected` to
  `centroids`, add `FIELD_STARS` = 60, `DETECTED_RADIUS_PX` and `StarMark.detected`, pass
  the report's detections to `star_marks`, and replace the expected-star selection,
  exactly as in plan.md § Interfaces. Remove `MAX_EXPECTED_STARS` (FR-001, FR-002,
  FR-003).
- [ ] T003 [US1] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story.

## Phase 2: User Story 2 — The live chart holds a star's look through a blink (Priority: P2)

### Tests first

- [ ] T004 [US2] Create `crates/finder/web/src/calm.test.ts` with failing tests for
  US2-S1 to US2-S4, calling `calm` directly with hand-built reports, times and history.
- [ ] T005 [US2] Create `crates/finder/web/src/App.calm.test.tsx` with failing tests for
  US2-S5 to US2-S8, with its own copy of the `FakeEventSource`, `CONNECTED` and `serve`
  harness from `crates/finder/web/src/App.live.test.tsx`, and a distinct `time` on each
  emitted report.
- [ ] T006 [US2] Create `crates/finder/web/src/components/ChartSeen.test.tsx` with failing
  tests for US2-S9 (ring class and radius) and US2-S10 (tap captions).

### Implementation

- [ ] T007 [US2] Add `detected?` and `seen?` to `StarMark` in
  `crates/finder/web/src/api.ts`, and create `crates/finder/web/src/calm.ts` with
  `CALM_FRAMES`, `CALM_HITS`, `CALM_SECONDS`, `seenNow`, `calm` and `useCalm` exactly as
  in plan.md § Interfaces, with each history entry recording `seenNow` of its own report
  (FR-004, FR-005, FR-006).
- [ ] T008 [US2] In `crates/finder/web/src/App.tsx`, wrap the report in `useCalm` at line
  130 and pass `live={showLive}` to `FieldList`. In
  `crates/finder/web/src/components/Chart.tsx` draw the ring class, delay, layer filter
  and legend count from `s.seen ?? seenNow(s)`, and write the picked-star caption. In
  `crates/finder/web/src/labels.ts` size `ringRadius` from the same. In
  `crates/finder/web/src/components/FieldList.tsx` filter named stars the same way and
  keep the `data-named-stars` line while live (FR-004, FR-006, FR-007, FR-008, FR-009).
- [ ] T009 [US2] Add plan.md's paragraph to `DESIGN.md` § Chart (signature) (FR-010).
- [ ] T010 [US2] Run the `lint` and `test` gates with every existing Vitest and
  Playwright test unchanged and green. Commit the rebuilt `crates/finder/web/dist`.
  Commit the green story.

## Execution Order

```text
US1 → US2 (US2 waits for US1 to merge)
```

- **US1** touches only `crates/finder/src/solve.rs` and a new test file.
- **US2** touches only `crates/finder/web/` and `DESIGN.md`. It waits for US1's merge so
  that its Playwright run serves a console that sends `detected`.
