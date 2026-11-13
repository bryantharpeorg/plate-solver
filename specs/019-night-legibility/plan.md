# Implementation Plan: A Night Page That Reads in the Dark

**Branch**: `019-night-legibility` | **Date**: 2026-11-13 | **Spec**: [spec.md](spec.md)

This plan records every design decision for the feature. A story that would change one
escalates rather than re-deciding it.

## Summary

Night mode's dimmer reds rise toward the ink, its text gets heavier and its small text a
step larger, and its chart puts the stars ahead of the grid. All of it is CSS under
`[data-theme="night"]` in one file. No component changes, the day edition is untouched,
and the page stays red-only and no brighter than today's ink.

## Technical Context

**Language/Version**: CSS (Tailwind 4) for the page; TypeScript for one new Playwright
file. No Rust changes.

**Primary Dependencies**: nothing new.

**Storage**: nothing.

**Testing**:
- One new Playwright file, `crates/finder/web/e2e/night-legibility.spec.ts`. It falls in
  the `default` project, because `crates/finder/web/playwright.config.ts` gives that
  project every file its other projects don't name.
- Computed styles need a real browser, so there is no new Vitest file.

**Target Platform**: the finder page, served by the `finder` binary.

**Constraints**:
- Red-only night: green and blue at zero, red no higher than 224.
- `web/dist` is committed and must be the build of its source.

**Scale/Scope**: one stylesheet, about 45 lines; two design records.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Nothing touches the solver or `crates/math-core`. |
| II. Never weaken a gate | No existing test, golden, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS`, `ergane.yaml` or `playwright.config.ts` is edited. New tests go in a new file. |
| III. Tests first, independent expectations | The oracle is the browser's computed style, and a contrast figure the test computes itself. |
| IV. Critical paths | No `unsafe`, no dependency change, nothing under `proto/`. |
| V. Standalone Rust, honest licensing | Unaffected. |

## Project Structure

```text
crates/finder/web/
├── src/index.css                    # US1: night tokens and night rules
├── e2e/night-legibility.spec.ts     # US1: new
└── dist/                            # US1: rebuilt
DESIGN.md, .impeccable/design.json   # US1: the night values
```

## Interfaces

### `crates/finder/web/src/index.css`

In the `[data-theme="night"]` token block, five values change and two are added:

```css
  --color-ink-2: #d40000;
  --color-ink-3: #a80000;
  --color-rule: #5a0000;
  --color-on-photo-2: #c00000;
  --color-grid: rgb(168 0 0 / 0.45);
  /* Red on black reads worse than ink on paper: small text is a step larger at night. */
  --text-xs: 0.875rem;
  --text-sm: 1rem;
```

In `@layer base`, before the night `.photo` rule:

```css
  /* Thin red strokes fade first in the dark, so night text is heavier. */
  [data-theme="night"] body {
    font-weight: 540;
  }
```

In `@layer components`, before `.furniture`:

```css
  /*
   * The night chart. Stars lead and the grid recedes: heavier rings at a fixed on-screen
   * width, unseen stars as plain dim rings, and larger, brighter edge labels.
   */
  [data-theme="night"] .edge-label {
    fill: var(--color-ink);
    font-size: 127%;
    font-weight: 640;
  }
  [data-theme="night"] .mark-matched circle.ring {
    stroke-width: 2.2px;
    vector-effect: non-scaling-stroke;
    opacity: 1;
  }
  [data-theme="night"] .mark-expected circle.ring {
    stroke-width: 1.2px;
    vector-effect: non-scaling-stroke;
    stroke-dasharray: none;
    opacity: 0.55;
  }
  [data-theme="night"] [data-picked="true"] circle.ring {
    stroke-width: 3.2px;
  }
  [data-theme="night"] .mark-label,
  [data-theme="night"] .object-label {
    font-weight: 680;
  }
  [data-theme="night"] .furniture {
    stroke-width: 1.8px;
    vector-effect: non-scaling-stroke;
  }
```

Why these work without touching a component:
- The chart sets stroke widths and font sizes as SVG attributes in image pixels. A CSS
  rule outranks a presentation attribute, and `vector-effect: non-scaling-stroke` makes a
  `px` width a width on screen.
- An edge label inherits its size from its group, so `127%` turns 11 into 14.
- Tailwind 4's `text-sm` and `text-xs` read `--text-sm` and `--text-xs`, so redefining
  them under night resizes every use.

### `DESIGN.md` and `.impeccable/design.json`

- The token list: `night-ink-2` `#d40000`, `night-ink-3` `#a80000`, `night-rule`
  `#5a0000`, `night-grid` `rgb(168 0 0 / 0.45)`, and a new `night-on-photo-2` `#c00000`.
  In `design.json`, night's `on-photo-2` maps to `night-on-photo-2`.
- § Night edition: one sentence saying the dimmer inks sit close to Night Ink so that
  secondary text keeps at least 3.5 : 1 on black.
- § Typography: night body text is weight 540, and small text is a step larger.
- § Chart (signature): at night, matched rings are 2.2 px, unseen stars are plain dim
  rings, edge labels are 14 px in Night Ink, and the grid sits at 45%.

## Shapes that existing tests pin

These are not changed by any story:
- **Red-only night.** `crates/finder/web/e2e/console.spec.ts:83`,
  `crates/finder/web/e2e/simulate.spec.ts:104` and their twins in `session.spec.ts`,
  `goto.spec.ts` and `mcp.spec.ts` count pixels with any green or blue, or red above 226.
  Every new colour passes.
- **Phone first screens.** `console.spec.ts` and `session.spec.ts` check that the title,
  the chart and its legend fit a 412 × 915 and a 390 × 844 screen. Larger small text must
  not break them.
- **The laptop layout.** `console.spec.ts` checks the margins and the chart's place at
  1440 × 900.
- **Vitest.** No Vitest test reads the stylesheet.

## Decisions and the alternatives rejected

| Decision | Rejected | Why |
|---|---|---|
| Raise only the dimmer reds | Raise the ink to `#ff0000` | The night checks cap red at 226, and brighter ink costs dark adaptation. |
| CSS only, with `vector-effect` | A night branch in `Chart.tsx` | 018 changes `Chart.tsx`; CSS keeps this to one stylesheet. |
| Leave the grid's stroke width alone | Thin it with `vector-effect` | The grid's draw-in uses a dash array, which `non-scaling-stroke` would measure on screen. Dimming it is enough. |
| Keep star and object labels' size | Enlarge them | `src/labels.ts` lays labels out at fixed sizes; larger text would overlap. |
| Resize through `--text-sm` and `--text-xs` | Edit each component | One place, and every use follows. |
| Numbers from the approved mock-up | A contrast target alone | The owner looked at these values. |

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
- **CI in the merge queue** runs every `ci.yml` job, including the finder-page
  build-freshness check.

## Prerequisites before this spec is flipped to `ready`

1. **The earlier specs have landed:** `016-mcp` and `018-live-chart-steady`. 018 is
   draft as this is written, so 019 can't dispatch before it.
2. **`main` is still as read.** Every citation here is at 76d17a6. If `index.css` has
   moved, re-read it and re-run the validator.
3. **The owner approvals are given** (spec § Owner approvals).
4. **Not beside 017.** `017-console-stop-confirm` US5 also rebuilds
   `crates/finder/web/dist` and edits `DESIGN.md` and `.impeccable/design.json`.
5. **No operator-staged files are needed.**

## Plan traps

1. **Gate evidence or nothing.** The judge credits only gate results and the story's
   diff. *Check:* every Then clause names a committed test that `CI=1 npm run e2e` runs.
2. **A theme test passes on a page that ignores the theme.** *Check:* each night number
   is paired with the day page's different value in the same file.
3. **Computed colours come back as `rgb()`.** *Check:* the tests compare `rgb(224, 0, 0)`
   and `rgba(168, 0, 0, 0.45)`, and read tokens as written from the root.
4. **A CSS `px` stroke is in image pixels without `vector-effect`.** *Check:* each night
   stroke rule carries `vector-effect: non-scaling-stroke`, and the test's on-screen
   helper reads it.
5. **The chart inks in.** *Check:* the night chart test waits 1.5 s, as the existing
   night checks do.
6. **Larger text can break a phone layout.** *Check:* the existing phone tests run
   unchanged; a failure escalates with its output.
7. **A spec fixed mid-epic needs a relaunch.** *Check:* no builder edits
   `specs/019-night-legibility/`.
8. **The bundle must be rebuilt.** *Check:* commit `crates/finder/web/dist` after
   `npm run build`.
9. **Each worktree builds on its own.** *Check:* no story sets `CARGO_TARGET_DIR`.

## Risks

| Risk | Mitigation |
|---|---|
| It still reads poorly outdoors. | The owner checks on the Surface; the numbers sit in one block and tune cheaply. |
| The grid at 45% is too faint. | Named for the owner in spec § Owner approvals; one value. |
| Brighter secondary text costs dark adaptation. | Nothing exceeds today's ink, and screen brightness still dims the page. |

## Refinement Record

Drafted on 2026-11-13 by the plate-solver operator against `main` at 76d17a6, from the
owner's screenshot of the night page outdoors and the mock-up he approved. This plan's CSS
and the spec's test file were then applied to a throwaway copy of 76d17a6 that also held
018's planned `Chart.tsx` change, the base 019 builds on, with no existing test edited.
On that copy, `npm test` (245 tests), `npm run typecheck`, `npm run format:check`,
`npm run build` and `CI=1 npm run e2e` passed: 28 Playwright tests, the three new ones
and every existing phone, laptop and red-only check. With today's `index.css` restored,
the three new tests failed. The Cargo gates were not run. Writes come from the task text.
Shared writes name the 016 and 018 stories that last wrote the same file, and 017's US5,
which is draft. Nothing in this record is an owner approval: those are in spec.md, and
flipping to `ready` gives them.

```yaml
version: 1
reviewed_revision: 76d17a6b32c20245237375237b06aa39a375ee71
stories:
  US1:
    tests:
      - path: crates/finder/web/e2e/night-legibility.spec.ts
        status: new
        disposition: add
        assertion: "night's ink-2 is #d40000 and ink-3 #a80000 with the ink unchanged; quiet text has at least 3.5 : 1 on black and the old secondary red less; night body weight is 540 and small text 16 px where day has 400 and 14 px"
        decision: "spec.md:US1-S1"
      - path: crates/finder/web/e2e/night-legibility.spec.ts
        status: new
        disposition: add
        assertion: "the day chart has 1.3 px matched rings, 11 px edge labels and dashed unseen rings; the night chart has 2.2 px matched rings, 1.2 px undashed unseen rings, a grid at 45%, 14 px edge labels in the ink at weight 640 and star labels at 680"
        decision: "spec.md:US1-S3"
      - path: crates/finder/web/e2e/console.spec.ts
        status: existing
        disposition: preserve
        assertion: "night puts nothing on screen but red, none brighter than the ink; the phone first screens and the laptop layout hold"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/e2e/session.spec.ts
        status: existing
        disposition: preserve
        assertion: "with a target, guidance, title, chart and legend fit a phone's first screen, and night has no off-red pixel"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/e2e/simulate.spec.ts
        status: existing
        disposition: preserve
        assertion: "the live chart and decoded preview obey the red-only night rule"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "the day edition is unchanged (spec.md FR-005)"
      - "red-only night: no green or blue, no red above the ink (spec.md FR-006)"
      - "every animation stays off under reduced motion (DESIGN.md)"
    checks:
      development:
        - "CI=1 npm run e2e"
        - "npm run build"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs: finder-page"
    writes:
      - .impeccable/design.json
      - DESIGN.md
      - crates/finder/web/dist
      - crates/finder/web/e2e/night-legibility.spec.ts
      - crates/finder/web/src/index.css
    no_write_reason: ""
    shared_writes:
      - path: DESIGN.md
        other_spec: 018-live-chart-steady
        other_story: US1
        disposition: merge_order
        reason: "019 depends_on_landed 018, whose US1 adds the live chart's rule"
      - path: crates/finder/web/dist
        other_spec: 018-live-chart-steady
        other_story: US1
        disposition: merge_order
        reason: "019 depends_on_landed 018, whose US1 rebuilds the bundle"
      - path: .impeccable/design.json
        other_spec: 016-mcp
        other_story: US7
        disposition: merge_order
        reason: "019 depends_on_landed 016, whose US7 last recorded its views"
      - path: DESIGN.md
        other_spec: 017-console-stop-confirm
        other_story: US5
        disposition: concurrent
        reason: "017 is draft; plan.md Prerequisite 4 keeps the two from running at once"
      - path: .impeccable/design.json
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
