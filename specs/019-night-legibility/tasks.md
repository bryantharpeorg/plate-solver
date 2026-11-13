---
description: "Task list for 019-night-legibility: A Night Page That Reads in the Dark"
---

# Tasks: A Night Page That Reads in the Dark

**Prerequisites** (plan.md § Prerequisites):
- `016-mcp` and `018-live-chart-steady` have landed on `main`;
- the owner approvals in spec.md are given;
- `017-console-stop-confirm` is not running;
- no operator-staged file is needed.

**Rules for every story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml`,
  `CODEOWNERS`, `ergane.yaml` or `crates/finder/web/playwright.config.ts`. If an existing
  test fails on the new styles, stop and escalate with its exact output. Do not edit it.
- Write only the files your story's tasks name; they are its `writes` in plan.md
  § Refinement Record. A story that needs another file escalates.
- Every CSS change sits under `[data-theme="night"]`. No day rule changes.
- Leave every `.tsx` and `.ts` file under `crates/finder/web/src` alone.
- Run no tool the gates don't run: no `cargo deny`, `cargo machete`, Rust 1.96 or
  `ergane`.

## Phase 1: User Story 1 — The night page reads in the dark (Priority: P1)

### Tests first

- [ ] T001 [US1] Create `crates/finder/web/e2e/night-legibility.spec.ts` with the
  helpers of spec.md's shared setup: an `open(page, night)`, a contrast-on-black
  function, and on-screen stroke and text sizes as spec.md § Clarifications defines
  them. Write failing tests for the tokens and contrast (US1-S1) and for body weight and
  small text, day and night (US1-S2).
- [ ] T002 [US1] Add failing tests to `crates/finder/web/e2e/night-legibility.spec.ts`
  for the day chart's baseline (US1-S3) and the night chart (US1-S4).

### Implementation

- [ ] T003 [US1] In `crates/finder/web/src/index.css`, make the changes in plan.md
  § Interfaces exactly: the five night tokens (FR-001), the night body weight (FR-002),
  the two night text sizes (FR-003) and the night chart rules (FR-004). Add nothing
  outside `[data-theme="night"]` (FR-005, FR-006).
- [ ] T004 [US1] Record the new night values in `DESIGN.md` and
  `.impeccable/design.json`, as plan.md § Interfaces lists them (FR-007).
- [ ] T005 [US1] Run the `lint` and `test` gates with every existing Vitest and
  Playwright test unchanged and green. Commit the rebuilt `crates/finder/web/dist`.
  Commit the green story (US1-S1 to US1-S4).

## Execution Order

```text
US1
```

- **US1** is the only story. It touches only `crates/finder/web/`, `DESIGN.md` and
  `.impeccable/design.json`.
- **`[P]`** marks nothing here: every task shares a file with another.
