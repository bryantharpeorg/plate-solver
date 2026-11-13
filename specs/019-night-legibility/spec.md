---
state: draft
depends_on_landed: ["016-mcp", "018-live-chart-steady"]
# Drafted 2026-11-13 against `main` at 76d17a6, from the owner's screenshot of the night
# page at the first real-sky test on 2026-11-12 and the mock-up he approved on 2026-11-13.
# It stays draft until the owner has reviewed it and given the approvals listed under
# "Owner approvals". Flipping it to `ready` is the act that dispatches it.
---

# Feature Specification: A Night Page That Reads in the Dark

**Feature Branch**: `019-night-legibility`

**Created**: 2026-11-13

**Status**: Draft

**Input**: Outside at the first real-sky test, the finder page's night mode was hard to
read. Most of the page is drawn in dimmed reds that barely stand out from black, in thin
type and thin strokes, and on the chart the grid competes with the stars. Night mode needs
brighter secondary reds, heavier and larger text, and a chart where the stars lead. The
day edition and the red-only rule do not change.

## Clarifications

### Session 2026-11-13 (review of `main` at 76d17a6)

- Q: What does this trio read besides itself? → A: Nothing about the feature. `spec.md`
  holds every requirement and scenario, and `plan.md` holds every design decision, the
  exact CSS and the shapes that existing tests pin. A story that would change a decision
  escalates rather than re-deciding it.
- Q: What was found? → A: Five things, each checked against the code at 76d17a6:
  1. Night's secondary red is `#a80000`, about 2.7 : 1 on black, and its tertiary red is
     `#760000`, about 1.8 : 1 (`crates/finder/web/src/index.css:50` and
     `crates/finder/web/src/index.css:51`). The main ink, `#e00000`, is about 4.2 : 1.
  2. The grid is the secondary red at 60% (`crates/finder/web/src/index.css:60`), drawn at
     the same on-screen width as much of the chart.
  3. Body text has no weight set, so it renders at 400.
  4. The chart's edge labels are 11 CSS pixels on screen, in the secondary red
     (`crates/finder/web/src/index.css:326`). Matched rings are 1.3 pixels at 80%
     (`crates/finder/web/src/index.css:340`), and unseen stars are dashed at 70% of a
     dimmer red (`crates/finder/web/src/index.css:344`).
  5. The night checks allow no green or blue and no red above 226
     (`crates/finder/web/e2e/simulate.spec.ts:104`), so the main ink can't be raised.
- Q: How bright may night get? → A: No brighter than today's ink. Only the dimmer reds
  rise toward it. Dimming the whole page stays with the screen's own brightness.
- Q: Where do the numbers come from? → A: From the mock-up the owner approved on
  2026-11-13, then rebuilt as plain CSS and run through the whole browser suite
  (plan.md § Refinement Record).
- Q: How is on-screen width asserted? → A: A committed Playwright test reads computed
  styles from the real page. A stroke with `vector-effect: non-scaling-stroke` is its
  `stroke-width` in CSS pixels. Any other stroke, and any SVG text size, is its computed
  value times the SVG's on-screen width over its `viewBox` width.

## Owner approvals

Flipping this spec to `ready` approves the following. Each is named in the FR that makes
it. No existing test, golden or CI file changes.

1. **The night reds** in FR-001: Second Ink `#d40000`, Third Ink `#a80000`, rule
   `#5a0000`, the dimmer photo ink `#c00000`, and the grid at 45%.
2. **Heavier night text**: body weight 540 (FR-002).
3. **Small text a step larger at night, everywhere on the page**: 14 px becomes 16 px and
   12 px becomes 14 px (FR-003).
4. **The night chart** in FR-004: thicker matched rings, unseen stars as plain dim rings,
   larger edge labels in the ink, heavier mark labels.
5. **`DESIGN.md` and `.impeccable/design.json` record these** (FR-007).

## User Scenarios & Testing *(mandatory)*

All scenarios share one setup:
- Each runs in Playwright's `default` project, in a new file
  `crates/finder/web/e2e/night-legibility.spec.ts`, against the real console, at a
  1440 × 900 viewport.
- "The night page" is the console opened with `finder.night` true, `finder.frame`
  `orion2.jpg` and a site in local storage, after `Solved in` is visible and the fonts
  have loaded. "The day page" is the same with `finder.night` false.
- "On screen" is defined in § Clarifications.
- "Contrast on black" is the WCAG contrast ratio of a computed `rgb()` colour against
  `#000000`, computed in the test.
- Every night assertion that names a number is paired, in the same file, with the day
  page's different value, so no test can pass on a page that ignores the theme.

### User Story 1 - The night page reads in the dark (Priority: P1)

As the observer at the eyepiece, I glance at the page in night mode. The secondary text,
the chart's edge labels and the star rings are readable at arm's length, the stars stand
out from the grid, and nothing on screen is brighter or less red than before.

**Why this priority**: Night mode is the only mode used at the telescope.

**Independent Test**: `CI=1 npm run e2e` in `crates/finder/web` runs
`crates/finder/web/e2e/night-legibility.spec.ts`.

**Acceptance Scenarios**:

1. **Given** the night page, **When** a committed test reads the root's tokens, **Then**
   it asserts that `--color-ink` is `#e00000`, `--color-ink-2` is `#d40000` and
   `--color-ink-3` is `#a80000`, that the computed colour of a `.quiet` element has a
   contrast on black of at least 3.5, that `rgb(168, 0, 0)` has at least 2.5, and that
   `rgb(168, 0, 0)` has less than 3.5: the old secondary red fails the new floor.
2. **Given** the day page, **When** a committed test reads computed styles, **Then** it
   asserts that `body` has `font-weight` `400` and that a `.text-sm` element has
   `font-size` `14px`. **Given** the night page, **Then** it asserts `540` and `16px`.
3. **Given** the day page, **When** a committed test reads the chart, **Then** it asserts
   that a matched ring is 1.3 px on screen to one decimal place, that an edge label is
   11 px on screen to the nearest pixel, and that an unseen star's ring has a
   `stroke-dasharray` other than `none`.
4. **Given** the night page, 1.5 s after it opens, **When** a committed test reads the
   chart, **Then** it asserts that:
   - a matched ring is 2.2 px on screen and an unseen star's ring 1.2 px, each to one
     decimal place;
   - an unseen star's ring has `stroke-dasharray` `none`;
   - a grid line is at most 1.05 px on screen, and its `stroke` is
     `rgba(168, 0, 0, 0.45)`;
   - an edge label is 14 px on screen to the nearest pixel, with `fill`
     `rgb(224, 0, 0)` and `font-weight` `640`;
   - a star's label has `font-weight` `680`.

---

### Edge Cases

- **The red-only rule.** Every new colour has green and blue at zero and red at or below
  224, so the existing night screenshot checks pass unchanged.
- **Phone width.** Larger small text may wrap sooner on a phone. The existing phone
  checks in `crates/finder/web/e2e/console.spec.ts` must pass unchanged; if one fails,
  the story escalates with its output.
- **A picked star.** Its ring stays thicker than an unpicked one: 3.2 px at night.
- **The grid's draw-in.** The grid keeps its user-unit stroke, so its draw-in animation is
  untouched.
- **Label layout.** Star and object labels keep their size, so `src/labels.ts` lays them
  out as before. Only their weight changes.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Under `[data-theme="night"]`, the tokens SHALL be: `--color-ink-2`
  `#d40000`, `--color-ink-3` `#a80000`, `--color-rule` `#5a0000`, `--color-on-photo-2`
  `#c00000` and `--color-grid` `rgb(168 0 0 / 0.45)`. `--color-ink` and every other night
  token SHALL stay as they are.
- **FR-002**: Under night, `body` SHALL have `font-weight: 540`.
- **FR-003**: Under night, `--text-xs` SHALL be `0.875rem` and `--text-sm` `1rem`.
- **FR-004**: Under night, the chart SHALL draw:
  - edge labels at 127% of their inherited size, weight 640, filled with `--color-ink`;
  - matched rings at 2.2 px with `vector-effect: non-scaling-stroke` and opacity 1;
  - unseen stars' rings at 1.2 px with `vector-effect: non-scaling-stroke`, no dashes and
    opacity 0.55;
  - a picked star's ring at 3.2 px;
  - star and object labels at weight 680;
  - the furniture's strokes at 1.8 px with `vector-effect: non-scaling-stroke`.
- **FR-005**: The day edition SHALL not change: no rule outside `[data-theme="night"]` is
  added, removed or edited.
- **FR-006**: Night SHALL still put nothing on screen but red, none brighter than the ink.
- **FR-007**: `DESIGN.md` (the token list, § Night edition, § Typography and § Chart
  (signature)) and `.impeccable/design.json` SHALL record the new night values, and
  `crates/finder/web/dist` SHALL be the build of its source.

### Key Entities

- **Night edition**: the page under `[data-theme="night"]`.
- **Second Ink, Third Ink**: `--color-ink-2` and `--color-ink-3`, the page's dimmer text
  colours.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario above has a committed test that asserts it. The
  `test` gate passes, including `npm test`, `npm run build` and `CI=1 npm run e2e`.
- **SC-002**: No existing test, golden, `scripts/ci/` file, `.github/workflows/ci.yml`,
  `CODEOWNERS` entry, `ergane.yaml` or `playwright.config.ts` is edited.
- **SC-003**: At night, secondary text has a contrast on black of at least 3.5, up from
  about 2.7.

## Assumptions

- **Earlier features are in place.** 016 and 018 have landed. Every citation here was
  read at 76d17a6.
- **Out of scope:**
  - the day edition;
  - a brightness or dimming control on the page;
  - the target arrow that can overlap a declination label on the chart's left edge;
  - the header's solve time, which read `0.37 ms` on the night and looks too fast;
  - the size of star and object labels, and anything in `src/labels.ts`;
  - the live preview, the survey view and the `#red-only` filter.
- **Outside the factory:** the owner judging the result outdoors on the Surface. The
  numbers are few and sit together, so a later tuning is a small change.
- **The factory's gates and CI:**
  - The factory's gates are `lint` and `test`, copied in plan.md § Verification.
  - CI's other jobs run only in the merge queue, so no scenario's Then clause rests on
    them.

## Work Graph

```yaml
US1:
  depends_on: []
  implements: [FR-001, FR-002, FR-003, FR-004, FR-005, FR-006, FR-007]
```

One story. It touches only `crates/finder/web/`, `DESIGN.md` and
`.impeccable/design.json`.
