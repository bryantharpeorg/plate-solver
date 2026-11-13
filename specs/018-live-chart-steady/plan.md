# Implementation Plan: A Live Chart That Holds Still

**Branch**: `018-live-chart-steady` | **Date**: 2026-11-13 | **Spec**: [spec.md](spec.md)

This plan records every design decision for the feature. A story that would change one
escalates rather than re-deciding it.

## Summary

Tell the chart when it is drawing live frames, and give its SVG a constant key while it
is. Key grid paths by their line, so a line that persists keeps its element. Decide each
grid's and furniture's intro when it mounts, and skip it once a solved live frame has been
drawn. Stars and objects keep their `ink-in`, so only marks new to the chart fade in.
Through up to two unsolved live frames in a row, keep the last live solve drawn. Uploaded
and sample frames are unchanged.

Nothing here changes the server, the live stream, the header, the lists or the CSS.

## Technical Context

**Language/Version**: TypeScript with React 19 for the page. No Rust changes.

**Primary Dependencies**: nothing new.

**Storage**: nothing new is persisted.

**Testing**:
- Vitest with `@testing-library/react`, in one new file,
  `crates/finder/web/src/components/ChartLive.test.tsx`.
- No new Playwright test. The behaviour is the component's, and the simulator's frames all
  solve, so it cannot show the hold.

**Target Platform**: the finder page, served by the `finder` binary.

**Constraints**:
- `web/dist` is committed and must be the build of its source.
- The red-only night mode, and every animation removed under reduced motion.

**Scale/Scope**: one component, one prop, one paragraph of `DESIGN.md`.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Nothing touches the solver or `crates/math-core`. |
| II. Never weaken a gate | No existing test, golden, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS` or `ergane.yaml` is edited. New tests go in a new file. |
| III. Tests first, independent expectations | The oracles are DOM node identity and class names, read from the rendered chart. |
| IV. Critical paths | No `unsafe`, no dependency change, no database-format change, nothing under `proto/`. |
| V. Standalone Rust, honest licensing | Unaffected. |

## Project Structure

```text
crates/finder/web/
├── src/components/Chart.tsx            # US1: live prop, key, hold, mount-time intro
├── src/components/ChartLive.test.tsx   # US1: new
├── src/App.tsx                         # US1: live={showLive}
└── dist/                               # US1: rebuilt
DESIGN.md                               # US1: § Chart (signature)
```

## Interfaces

### `Chart` (`crates/finder/web/src/components/Chart.tsx`)

`Props` gains:

```ts
  /** The report is the newest live frame's, not an uploaded or sample frame's. */
  live?: boolean;
```

The hold and the intro come from a small state machine beside the component:

```ts
/** Unsolved live frames in a row that keep the last live solve on the chart. */
export const LIVE_HOLD = 2;

interface Steady {
  live: boolean;
  /** The report this state was last advanced for. */
  seen: SolveReport | null;
  /** The newest solved live report. */
  last: SolveReport | null;
  /** Unsolved live reports since `last`. */
  misses: number;
  /** A solved live chart has been drawn since the chart switched to live. */
  settled: boolean;
}

const UNSTEADY: Steady = { live: false, seen: null, last: null, misses: 0, settled: false };

/** Advance the live chart's state by one report. */
export function advance(state: Steady, report: SolveReport | null, live: boolean): Steady {
  if (!live || !report) return { ...UNSTEADY, live, seen: report };
  const base = state.live ? state : { ...UNSTEADY, live };
  if (report === base.seen) return base;
  const settled = base.settled || base.last !== null;
  if (report.pointing) return { live, seen: report, last: report, misses: 0, settled };
  return { ...base, seen: report, misses: base.misses + 1, settled };
}
```

`useSteady(report, live)` keeps a `Steady` in `useState`, and advances it during render
when `report` or `live` differs from the stored one. This is React's pattern for storing
information from previous renders. It returns:
- `shown`: the report to draw. While live, an unsolved report with a stored `last` and
  `misses <= LIVE_HOLD` gives `last`. Otherwise, and always when not live, it gives the
  report itself;
- `intro`: `!settled` while live, and `true` otherwise.

`Chart` renames its `report` prop to `newest`, calls `useSteady(newest, live)`, and draws
`shown` everywhere it drew `report`: geometry, layers, legend, furniture and target mark.
`label`, `imageUrl` and `busy` are drawn as they are passed.

The key:

```ts
const key = live ? "live" : `${report.name}:${report.pointing?.ra_deg ?? "none"}`;
```

`Grid` and `Furniture` each take an `intro: boolean` and read it once, at mount, with
`const [intro] = useState(introNow)`. With `intro` false:
- grid paths have the class `grid-line` without `draw`, and no animation delay;
- the grid labels' group has no `ink-in` class and no animation delay;
- the furniture group has the class `furniture` without `ink-in`, and no animation delay.

With `intro` true each is exactly as at 76d17a6. Grid paths are keyed
`` `${line.kind}:${line.value_deg}.${j}` ``. `StarSymbol` and `ObjectSymbol` are
unchanged.

### `App` (`crates/finder/web/src/App.tsx`)

One prop is added to the existing `<Chart … />`: `live={showLive}`. Nothing else changes.

### `DESIGN.md` § Chart (signature)

After "With reduced motion all animation is off." add:

> On the live chart, the grid draws in and the furniture inks in once, on the first solved
> live frame. After that, marks that persist from frame to frame update in place, and only
> a star or object new to the chart inks in. Through up to two unsolved live frames in a
> row, the chart keeps the last live solve.

## Shapes that existing tests pin

These are not changed by any story:
- **The unsolved upload view.** `crates/finder/web/src/components/Chart.test.tsx:90`
  renders `ROOF` without `live` and wants five `.mark-detected` and no `.grid-line`.
- **The grid and its labels.** `crates/finder/web/src/components/Chart.test.tsx:82` wants grid labels in the margin.
- **Live star marks.** `crates/finder/web/src/App.live.test.tsx:108` wants
  `[data-star]` circles at each star's `x` and `y`, then the new count after a `VIRGO`
  live report.
- **An unsolved live report.** `crates/finder/web/src/App.live.test.tsx:184` wants the header's region named
  `No match` and the target region to say `pointing stale`. The header follows the
  newest report, so the hold doesn't touch it.
- **The chart's accessible name.** `crates/finder/web/e2e/simulate.spec.ts:129`,
  `crates/finder/web/e2e/simulate.spec.ts:263` and `crates/finder/web/e2e/tracking.spec.ts:124` find it with
  `/^Chart of Live frame /`, and `crates/finder/web/e2e/console.spec.ts:71` with
  `/^Chart of/`.
- **The night screenshots.** `crates/finder/web/e2e/simulate.spec.ts:273`, `crates/finder/web/e2e/tracking.spec.ts:131` and
  `crates/finder/web/e2e/goto.spec.ts:202` wait 1500 ms for the ink-in before counting
  off-red pixels. No colour changes.
- **The CSS.** `.draw`, `.ink-in` and the reduced-motion rule in
  `crates/finder/web/src/index.css` are unchanged.

## Decisions and the alternatives rejected

| Decision | Rejected | Why |
|---|---|---|
| An explicit `live` prop from `App` | Keying on `report.pointing?.source` | `SolveReport.pointing` has no `source`; that field is the session's pointing. |
| An explicit `live` prop from `App` | Treating `report.name === null` as live | Nothing promises that an upload or sample always has a name. |
| Decide the intro when an element mounts | A class on the SVG that turns `.draw` off once settled | Removing `draw` from a grid still drawing in would snap it. Mount-time needs no CSS change. |
| Hold the last solve for two unsolved frames | Show every unsolved frame at once | Occasional failures would blink the grid on and off at up to 1 Hz, the flashing the spec removes. |
| Hold for two, drop on the third | Hold until the next solve | The chart could stay stale for minutes. Three matches the console's own rule for clearing its estimate. |
| Hold inside `Chart` | Hold in `App`, for the header and lists too | Larger, and it would change what the header reports. Out of scope (spec § Assumptions). |
| Keep `ink-in` on stars and objects | Turn it off once settled, like the grid | A star entering the field may fade in (spec FR-004). |

## Verification (every story, before proposing)

- **The factory gates** (`ergane.yaml` at 76d17a6):
  - `lint` = `cargo fmt --all --check`,
    `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`,
    `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features --locked`
    and `RUSTDOCFLAGS="-D warnings" cargo doc -p finder --no-deps`;
  - `test` = `cargo test --workspace --all-targets --all-features --locked`,
    `cargo test --workspace --locked --doc`, and then, in `crates/finder/web`,
    `npm ci --no-audit --no-fund`, `npm run typecheck`, `npm run format:check`,
    `npm test`, `npm run build`, `git status --porcelain -- dist | (! grep .)` and
    `CI=1 npm run e2e`.

  Every story runs both, and the judge sees their results.
- **CI in the merge queue** runs every `ci.yml` job, including coverage at 80%, the
  finder-page build-freshness check and the build on Rust 1.96.

## Prerequisites before this spec is flipped to `ready`

1. **The earlier specs have landed:** `012-targets-and-push-to`, `013-live-finder` and
   `016-mcp`. Met: 016's last story landed at bcf3765, before 76d17a6.
2. **`main` is still as read.** Every citation here is at 76d17a6. If `main` has moved in
   these files, re-read them and re-run the validator.
3. **The owner approvals are given** (spec § Owner approvals). Flipping to `ready`
   records them.
4. **Not beside 017.** `017-console-stop-confirm` US5 also rebuilds
   `crates/finder/web/dist` and edits `DESIGN.md`. If 017 is `ready` or running, flip 018
   only after 017 has landed, or the other way round.
5. **No operator-staged files are needed.**

## Plan traps

Lessons from building 012 to 016 on this factory, each with its mechanism and the check
that keeps it from recurring. They bind every story.

1. **Gate evidence or nothing.** The judge credits only gate results and the story's
   diff. *Check:* every Then clause names a committed test that `npm test` runs.
2. **A named command is read literally.** *Check:* commands are copied from
   § Verification.
3. **One evidence budget.** *Check:* the story keeps its diff under 64 KiB, measured with
   `git diff <base> -- . ':(exclude)crates/finder/web/dist' | wc -c`.
4. **The sandbox has only the gates' tools.** *Check:* no task runs a tool outside them.
5. **A spec fixed mid-epic needs a relaunch.** The judge grades the spec as dispatched.
   *Check:* no builder edits `specs/018-live-chart-steady/`.
6. **A same-node assertion passes on a chart that never re-rendered.** *Check:* every
   such test re-renders with a new report object, and the file also shows a re-render
   that gives a different node (spec scenarios 2 and 7).
7. **A negative assertion passes when nothing is drawn.** "No `.grid-line.draw`" passes
   on a chart with no grid. *Check:* scenario 6 also asserts at least one `.grid-line`.
8. **Updating state during render loops if it never settles.** *Check:* `useSteady` sets
   state only when `report` or `live` differs from the stored one, and `advance` returns
   the same `seen` for the same report.
9. **The bundle must be rebuilt.** *Check:* commit `crates/finder/web/dist` after
   `npm run build`, so that the gate's `git status --porcelain -- dist` finds nothing.
10. **Each worktree builds on its own.** *Check:* no story sets `CARGO_TARGET_DIR`.

## Risks

| Risk | Mitigation |
|---|---|
| A held chart is read as current. | It lasts at most two frames, about 2 s, and the header above it reports the newest frame. |
| A grid line that newly enters the field draws in. | That is a mark entering the field, which the spec allows. Persisting lines keep their elements. |
| A hold shows a picked mark from the held frame. | Picking already keeps its selection across reports; nothing new. |

## Refinement Record

Drafted on 2026-11-13 by the plate-solver operator against `main` at 76d17a6, from the
owner's report of the first real-sky test. This plan's code was then applied to a
throwaway copy of 76d17a6, with no existing test edited, and six probe tests covered
scenarios 1 to 8. Before the change, the tests of scenarios 1, 3 to 6 and 8 failed, and
those of 2 and 7, the controls, passed. After it, every one passed, and `npm test` (34
files, 245 tests), `npm run typecheck`, `npm run format:check` and `npm run build` passed.
The Cargo gates and Playwright were not run. Writes come from the task text. Shared
writes name the 012 and 016 stories that last wrote the same file on `main`, and 017's
US5, which is draft. Nothing in this record is an owner approval: those are in spec.md,
and flipping to `ready` gives them.

```yaml
version: 1
reviewed_revision: 76d17a6b32c20245237375237b06aa39a375ee71
stories:
  US1:
    tests:
      - path: crates/finder/web/src/components/ChartLive.test.tsx
        status: new
        disposition: add
        assertion: "a live frame moved by 0.004 deg keeps the same svg, grid-line and data-star nodes; a different upload gives a different svg; a star new to a live frame inks in while every persisting star keeps its node"
        decision: "spec.md:US1-S1"
      - path: crates/finder/web/src/components/ChartLive.test.tsx
        status: new
        disposition: add
        assertion: "two unsolved live frames keep the same svg and grid-line nodes and every star; the third leaves no grid-line and shows the detected stars; the next solve draws grid lines without draw and furniture without ink-in, in the same svg"
        decision: "spec.md:US1-S4"
      - path: crates/finder/web/src/components/ChartLive.test.tsx
        status: new
        disposition: add
        assertion: "each switch between upload and live gives a different svg, and switching to live draws the grid in; a solve between unsolved live frames resets the count"
        decision: "spec.md:US1-S7"
      - path: crates/finder/web/src/components/Chart.test.tsx
        status: existing
        disposition: preserve
        assertion: "an unsolved upload shows only the detected stars and no grid line; the grid's labels sit in the margin"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/src/App.live.test.tsx
        status: existing
        disposition: preserve
        assertion: "live star marks sit at each star's x and y and follow a new live report; an unsolved live report leaves the header's No match region saying pointing stale"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/e2e/simulate.spec.ts
        status: existing
        disposition: preserve
        assertion: "the live chart is found by its accessible name, and the live page obeys the red-only night rule after the ink-in"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "uploaded and sample frames keep their key, their intro and their unsolved view (spec.md FR-007)"
      - "every animation stays off under reduced motion (DESIGN.md)"
      - "red-only night mode: no colour or token changes"
    checks:
      development:
        - "npm test"
        - "npm run typecheck"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs: finder-page and coverage at 80%"
    writes:
      - DESIGN.md
      - crates/finder/web/dist
      - crates/finder/web/src/App.tsx
      - crates/finder/web/src/components/Chart.tsx
      - crates/finder/web/src/components/ChartLive.test.tsx
    no_write_reason: ""
    shared_writes:
      - path: DESIGN.md
        other_spec: 016-mcp
        other_story: US7
        disposition: merge_order
        reason: "018 depends_on_landed 016, whose US7 last recorded its views"
      - path: crates/finder/web/dist
        other_spec: 016-mcp
        other_story: US7
        disposition: merge_order
        reason: "018 depends_on_landed 016, whose US7 last rebuilt the bundle"
      - path: crates/finder/web/src/App.tsx
        other_spec: 016-mcp
        other_story: US7
        disposition: merge_order
        reason: "018 depends_on_landed 016, whose US7 last changed App.tsx"
      - path: crates/finder/web/src/components/Chart.tsx
        other_spec: 012-targets-and-push-to
        other_story: US8
        disposition: merge_order
        reason: "018 depends_on_landed 012, whose US8 last changed Chart.tsx"
      - path: DESIGN.md
        other_spec: 017-console-stop-confirm
        other_story: US5
        disposition: concurrent
        reason: "017 is draft; plan.md Prerequisite 4 keeps the two from running at once"
      - path: crates/finder/web/dist
        other_spec: 017-console-stop-confirm
        other_story: US5
        disposition: concurrent
        reason: "017 is draft; plan.md Prerequisite 4 keeps the two from running at once"
    open_questions: []
```
