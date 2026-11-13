---
state: draft
depends_on_landed: ["012-targets-and-push-to", "013-live-finder", "016-mcp"]
# Drafted 2026-11-13 against `main` at 76d17a6, from the owner's report of the first
# real-sky test on 2026-11-12. It stays draft until the owner has reviewed it and given
# the approvals listed under "Owner approvals". Flipping it to `ready` is the act that
# dispatches it.
---

# Feature Specification: A Live Chart That Holds Still

**Feature Branch**: `018-live-chart-steady`

**Created**: 2026-11-13

**Status**: Draft

**Input**: At the first real-sky test on 2026-11-12, every live frame solved: about one
frame a second, 17 to 26 stars, a 13.0° field. The finder page's chart never settled. On
every frame the grid lines drew in again and the star and object marks faded out and back
in, so it looked like a loop of painting, clearing and painting again. The solving is
fine; this is purely the page. Consecutive live frames must update the chart in place,
with the intro animation kept for uploaded and sample frames.

## Clarifications

### Session 2026-11-13 (review of `main` at 76d17a6)

- Q: What does this trio read besides itself? → A: Nothing about the feature. `spec.md`
  holds every requirement and scenario, and `plan.md` holds every design decision, the
  interfaces, the shapes that existing tests pin and the alternatives rejected. A story
  that would change a decision escalates rather than re-deciding it.
- Q: What was found? → A: Four things, each checked against the code at 76d17a6:
  1. The chart's SVG is keyed by `` `${report.name}:${report.pointing?.ra_deg ?? "none"}` ``
     (`crates/finder/web/src/components/Chart.tsx:115`, used at
     `crates/finder/web/src/components/Chart.tsx:139`).
  2. A live report has `name: null` (`crates/finder/src/solve.rs:439`) and an RA that
     changes on every frame, because the sky drifts even when the camera is still. So
     React mounts a new SVG for every frame.
  3. Every mount replays the intro: the grid lines' `.draw` (900 ms) and the `.ink-in`
     (500 ms) of the grid labels, the furniture, the stars and the objects, after delays of
     up to 600 ms plus 60 ms per object. The animations are at
     `crates/finder/web/src/index.css:317` and `crates/finder/web/src/index.css:322`, and
     the delays at:
     - `crates/finder/web/src/components/Chart.tsx:230` (grid lines);
     - `crates/finder/web/src/components/Chart.tsx:237` (grid labels);
     - `crates/finder/web/src/components/Chart.tsx:341` (objects);
     - `crates/finder/web/src/components/Chart.tsx:402` (stars);
     - `crates/finder/web/src/components/Chart.tsx:476` (furniture).

     With `prefers-reduced-motion: reduce` (`crates/finder/web/src/index.css:472`) the
     animations are off and the flashing stops.
  4. A throwaway Vitest probe at 76d17a6 rendered the chart with a live report, then
     re-rendered it with the RA moved by 0.004°, and found a different `svg[role="img"]`
     node.
- Q: The report suggested keying on `report.pointing?.source === "live"`. Does that work?
  → A: No. `SolveReport.pointing` (`crates/finder/web/src/api.ts:56`) has no `source`.
  The README's `pointing.source` is the session's pointing (`SessionPointing.source`,
  `crates/finder/web/src/api.ts:153`). The chart learns that it is live from a new `live` prop, which `App` sets
  from `showLive` (`crates/finder/web/src/App.tsx:117`).
- Q: What happens across an unsolved live frame? → A: The chart holds the last live solve
  through up to two unsolved live frames in a row, and shows the unsolved frame on the
  third. That matches the console's own rule, under which three consecutive failures clear
  the estimate (`crates/finder/README.md:316`). When solving resumes after that, the grid
  and the furniture appear without their intro, and marks new to the chart fade in once. So
  occasional failures never toggle the chart, and a run of failures changes it once
  (FR-006).
- Q: What is "the intro"? → A: The grid lines' `.draw`, and the `.ink-in` of the grid
  labels, the furniture, the stars and the objects.

## Owner approvals

Flipping this spec to `ready` approves the following. Each is named in the FR that makes
it. No existing test, golden or CI file changes.

1. **The hold.** Up to two unsolved live frames in a row, about 2 s at one frame a second,
   keep the last solve on the chart, while the header above it reports the newest frame
   (FR-006).
2. **The grid draws in once per switch to live.** After the first solved live frame, the
   grid and the furniture never draw in again until the chart leaves live. A star or
   object new to the chart still fades in (FR-004, FR-005).
3. **A new optional `live` prop on `Chart`**, set by `App` (FR-001).
4. **`DESIGN.md` records the live chart's motion rule** (FR-008).

## User Scenarios & Testing *(mandatory)*

All scenarios share one setup:
- Each runs in Vitest with `@testing-library/react`, rendering `Chart` directly with
  `DEFAULT_LAYERS` and changing its props with `rerender`.
- "A live frame of F moved by d" is a new object
  `{ ...F, name: null, pointing: { ...F.pointing, ra_deg: F.pointing.ra_deg + d } }`, for
  the fixtures `ORION2` and `VIRGO` in `crates/finder/web/src/test/fixtures.ts`.
- "An unsolved live frame" is a new object
  `{ ...ORION2, name: null, pointing: null, stars: [], objects: [], grid: [], status: "no_match" }`.
- "The live chart" is `Chart` rendered with `live` true. "The upload chart" is `Chart`
  rendered without `live`.
- "The same node" means the same DOM element, compared with `toBe`.
- Every scenario that asserts the same node also shows, in the same file, a re-render
  that gives a different node (scenarios 2 and 7).

### User Story 1 - Live frames update the chart in place (Priority: P1)

As the observer at the eyepiece, I watch the chart while live frames arrive about once a
second. The grid, the star and object marks and their labels stay where they are and move
with the sky, instead of drawing in again on every frame. A star or object that enters the
field may fade in. Uploaded and sample frames still draw in as they do today.

**Why this priority**: The chart is the page's main view at the telescope, and it was
unreadable at the first real-sky test.

**Independent Test**: `npm test` in `crates/finder/web` runs
`crates/finder/web/src/components/ChartLive.test.tsx`.

**Acceptance Scenarios**:

1. **Given** the live chart rendered with a live frame of `ORION2` moved by 0, **When** a
   committed test re-renders it with a live frame of `ORION2` moved by 0.004, **Then** it
   asserts that:
   - the `svg[role="img"]` is the same node;
   - the `.grid-line` elements are the same nodes, in the same order, and there is at
     least one;
   - for every star in `ORION2.stars`, the `[data-star]` element is the same node.
2. **Given** the upload chart rendered with `ORION2`, **When** a committed test re-renders
   it with `VIRGO`, **Then** it asserts that the `svg[role="img"]` is a different node.
3. **Given** the live chart rendered with a live frame of `ORION2` whose `stars` omit
   `ORION2.stars[0]`, **When** a committed test re-renders it with a live frame of
   `ORION2` moved by 0.004, with every star, **Then** it asserts that every star in the
   first frame keeps the same `[data-star]` node, and that the `[data-star]` element for
   `ORION2.stars[0]` exists and has the class `ink-in`.
4. **Given** the live chart rendered with a live frame of `ORION2`, **When** a committed
   test re-renders it with two unsolved live frames in turn, **Then** it asserts after
   each that the `svg[role="img"]` is the same node, the `.grid-line` elements are the
   same nodes as on the solved frame, and the number of `[data-star]` elements equals
   `ORION2.stars.length`.
5. **Given** that chart after the two unsolved live frames, **When** a committed test
   re-renders it with a third unsolved live frame, **Then** it asserts that the
   `svg[role="img"]` is the same node, that there is no `.grid-line` element, and that the
   number of `.mark-detected` elements equals `ORION2.detected.length`.
6. **Given** that chart after the third unsolved live frame, **When** a committed test
   re-renders it with a live frame of `ORION2` moved by 0.012, **Then** it asserts that
   the `svg[role="img"]` is the same node, that there is at least one `.grid-line`
   element, that no element matches `.grid-line.draw`, and that no element matches
   `.furniture.ink-in`.
7. **Given** the upload chart rendered with `ORION2`, **When** a committed test re-renders
   it as the live chart with a live frame of `ORION2`, **Then** it asserts that the
   `svg[role="img"]` is a different node and that an element matches `.grid-line.draw`.
   **When** it then re-renders it as the upload chart with `VIRGO`, and once more as the
   live chart with a live frame of `ORION2`, **Then** it asserts that each switch gives a
   different `svg[role="img"]` node, and that an element matches `.grid-line.draw` after
   the return to live.
8. **Given** the live chart rendered with a live frame of `ORION2`, **When** a committed
   test re-renders it with, in turn, an unsolved live frame, a live frame of `ORION2`
   moved by 0.004, and two more unsolved live frames, **Then** it asserts that the
   `.grid-line` elements after the last of them are the same nodes as on the first
   frame: a solve resets the count of unsolved frames.

---

### Edge Cases

- **The first live frame is unsolved.** There is nothing to hold, so the chart shows that
  frame as it does today. The first solved live frame plays the intro.
- **A layer chip is turned on while live.** Turning the grid off and on again mounts the
  grid afresh. After the first solved live frame it appears without drawing in. An upload
  chart behaves as today.
- **A star changes from matched to expected.** It keeps its element, because its `id` is
  the same. Its class changes and its fade does not replay.
- **Reduced motion.** Nothing animates, live or not, as today.
- **Night mode.** Unchanged. No colour or token changes.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: `Chart` SHALL take an optional `live` prop, false when absent. `App` SHALL
  pass `live={showLive}`. Nothing else in `App` changes.
- **FR-002**: While `live` is true, the chart SHALL keep one SVG element across reports.
  Without `live`, the SVG's key SHALL stay
  `` `${report.name}:${report.pointing?.ra_deg ?? "none"}` ``.
- **FR-003**: Grid paths SHALL be keyed by their line's `kind` and `value_deg` and the
  path's index within the line, so that a line in consecutive reports keeps its elements.
  Stars and objects keep their keys, `id`.
- **FR-004**: The grid's draw-in, and the ink-in of the grid labels and the furniture,
  SHALL be decided when each mounts. Each plays its intro unless a solved live report has
  already been drawn since the chart switched to live. Stars and objects keep `ink-in`, so
  a mark new to the chart fades in, and a mark that persists keeps its element and does not
  fade in again.
- **FR-005**: Switching the chart to live, and switching it away, SHALL each mount a new
  SVG. The intro therefore plays at most once per switch to live, on the first solved live
  report.
- **FR-006**: While `live` is true, an unsolved report SHALL leave the last solved live
  report drawn (its grid, marks, furniture, legend and target mark) for up to
  `LIVE_HOLD` = 2 unsolved reports in a row. The third shows the unsolved report as today.
  A solved report resets the count. Each new report object counts once, whatever else
  re-renders the chart.
- **FR-007**: Uploaded and sample frames SHALL behave exactly as before: the same key, the
  same intro, the same unsolved view.
- **FR-008**: `DESIGN.md` § Chart (signature) SHALL record the live chart's rule, and
  `crates/finder/web/dist` SHALL be the build of its source.

### Key Entities

- **Live chart**: `Chart` rendered with `live` true, for the newest live frame.
- **Intro**: the grid's draw-in and the ink-in of labels, furniture and marks.
- **Hold**: the last solved live report, drawn through up to two unsolved live reports.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario above has a committed test that asserts it. The
  `test` gate passes, including `npm test`, `npm run build` and `CI=1 npm run e2e`.
- **SC-002**: No existing test, golden, `scripts/ci/` file, `.github/workflows/ci.yml`,
  `CODEOWNERS` entry or `ergane.yaml` is edited.
- **SC-003**: `crates/finder/web/dist` is the build of its source, as the `test` gate's
  `git status --porcelain -- dist` check proves.

## Assumptions

- **Earlier features are in place.** 012, 013 and 016 have landed. Every citation here was
  read at 76d17a6.
- **Out of scope:**
  - the upload and sample frames' animations;
  - night mode;
  - the live preview below the legend;
  - the solver and the scope link;
  - the header, the field roster and the field list, which follow the newest report as
    today and so still change on an unsolved live frame;
  - the chart's accessible name, which stays `Chart of Live frame <id> …` while it holds.
- **The factory's gates and CI:**
  - The factory's gates are `lint` and `test`, copied in plan.md § Verification.
  - CI's other jobs run only in the merge queue, so no scenario's Then clause rests on
    them.

## Work Graph

```yaml
US1:
  depends_on: []
  implements: [FR-001, FR-002, FR-003, FR-004, FR-005, FR-006, FR-007, FR-008]
```

One story. It touches only `crates/finder/web/` and `DESIGN.md`.
