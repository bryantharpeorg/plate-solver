---
description: "Task list for 018-live-chart-steady: A Live Chart That Holds Still"
---

# Tasks: A Live Chart That Holds Still

**Prerequisites** (plan.md § Prerequisites):
- `012-targets-and-push-to`, `013-live-finder` and `016-mcp` have landed on `main`;
- the owner approvals in spec.md are given;
- `017-console-stop-confirm` is not running;
- no operator-staged file is needed.

**Rules for every story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml`,
  `CODEOWNERS` or `ergane.yaml`. If an existing test pins the old behaviour, stop and
  escalate with its exact output. Do not edit it.
- Write only the files your story's tasks name; they are its `writes` in plan.md
  § Refinement Record. A story that needs another file escalates.
- New tests go in the new file `crates/finder/web/src/components/ChartLive.test.tsx`.
- Leave `crates/finder/web/src/index.css`, the server, the live stream, the live
  preview, the solver and the scope link alone.
- Run no tool the gates don't run: no `cargo deny`, `cargo machete`, Rust 1.96 or
  `ergane`.

## Phase 1: User Story 1 — Live frames update the chart in place (Priority: P1)

### Tests first

- [ ] T001 [US1] Create `crates/finder/web/src/components/ChartLive.test.tsx` with
  helpers for spec.md's shared setup: a `chart(report, live)` element with
  `DEFAULT_LAYERS`, a `frame(base, dra)` that builds "a live frame of F moved by d" as a
  new object, and an `unsolved()` that builds "an unsolved live frame" as a new object.
  Write failing tests for:
  - a live frame moved by 0.004 keeping the same `svg[role="img"]`, `.grid-line` and
    `[data-star]` nodes (US1-S1);
  - a different upload giving a different SVG (US1-S2);
  - a star new to a live frame with `ink-in`, and every persisting star's node kept
    (US1-S3).
- [ ] T002 [US1] Add failing tests to `crates/finder/web/src/components/ChartLive.test.tsx`
  for:
  - two unsolved live frames keeping the SVG, the grid's nodes and every star (US1-S4);
  - the third dropping the grid and showing the detected stars (US1-S5);
  - the next solve drawing the grid without `draw` and the furniture without `ink-in`,
    with at least one `.grid-line` (US1-S6);
  - each switch between upload and live giving a new SVG, with the grid drawing in after
    each switch to live (US1-S7);
  - a solve between unsolved live frames resetting the count (US1-S8).

### Implementation

- [ ] T003 [US1] In `crates/finder/web/src/components/Chart.tsx`, add the `live` prop,
  `LIVE_HOLD`, `Steady`, `advance`, `useSteady`, the key, the grid paths' keys, and the
  mount-time `intro` of `Grid` and `Furniture`, exactly as in plan.md § Interfaces.
  Leave `StarSymbol`, `ObjectSymbol` and the upload path unchanged.
- [ ] T004 [US1] In `crates/finder/web/src/App.tsx`, pass `live={showLive}` to the
  existing `<Chart … />`, and change nothing else.
- [ ] T005 [US1] Add plan.md § Interfaces' paragraph to `DESIGN.md` § Chart (signature),
  after "With reduced motion all animation is off."
- [ ] T006 [US1] Run the `lint` and `test` gates with every existing Vitest and
  Playwright test unchanged and green. Commit the rebuilt `crates/finder/web/dist`.
  Commit the green story (FR-001 to FR-008; US1-S1 to US1-S8). In the story's summary,
  say in one line what the chart does across an unsolved live frame.

## Execution Order

```text
US1
```

- **US1** is the only story. It touches only `crates/finder/web/` and `DESIGN.md`.
- **`[P]`** marks nothing here: every task shares a file with another.
