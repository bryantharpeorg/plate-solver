# Implementation Plan: A Live Chart That Stays Calm

**Branch**: `020-live-chart-calm` | **Date**: 2026-11-13, refined 2026-11-14 | **Spec**: [spec.md](spec.md)

This plan records every design decision for the feature. A story that would change one
escalates rather than re-deciding it.

## Summary

Two fixes for star marks that flip between matched and unseen on every live frame.

- **US1, the server:** after a solve, cross-match each listed catalog star against every
  detected star, not only the 30 the solver used, and list the 60 brightest catalog stars
  in the frame whichever of them the solver matched. A new `detected` field carries the
  result; `matched` keeps the solver's meaning.
- **US2, the page:** on live frames, draw a star seen if it was seen in two of the
  previous three solved live frames within 10 s, keep the named-stars line's height, and
  say in a tapped star's caption why its ring is solid.

Nothing here changes the solve, the pointing, the guidance, the model or the scope link.

## Technical Context

**Language/Version**: Rust 2021 for `crates/finder`; TypeScript with React 19 for the
page.

**Primary Dependencies**: nothing new.

**Storage**: nothing new is persisted.

**Testing**:
- US1: a new integration test file, `crates/finder/tests/star_detected.rs`, using the
  bundled database and `orion2.jpg` as `crates/finder/tests/api.rs:210` does.
- US2: three new Vitest files: `crates/finder/web/src/calm.test.ts` (the pure rule),
  `crates/finder/web/src/App.calm.test.tsx` (the page) and
  `crates/finder/web/src/components/ChartSeen.test.tsx` (ring size and captions).

**Target Platform**: the finder console and its page.

**Constraints**:
- `web/dist` is committed and must be the build of its source.
- The cedar parity gates read `pointing` and the matched ids
  (`crates/finder/tests/api.rs:440` to `crates/finder/tests/api.rs:466`); neither changes.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | The solver, its inputs and `matched` are untouched. `detected` is computed after the solve from the report's own data. |
| II. Never weaken a gate | No existing test, golden, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS` or `ergane.yaml` is edited. New tests go in new files. |
| III. Tests first, independent expectations | US1's oracle is the report's own `detected` list, pixel distances, and the 60 brightest computed by the test from the database. US2's is DOM classes, attributes, caption text and a pure function's output. |
| IV. Critical paths | No `unsafe`, no dependency change, no database-format change, nothing under `proto/`. |
| V. Standalone Rust, honest licensing | Unaffected. |

## Project Structure

```text
crates/finder/
├── src/solve.rs                         # US1: StarMark.detected, FIELD_STARS membership
├── tests/star_detected.rs               # US1: new
└── web/
    ├── src/api.ts                       # US2: StarMark.detected?, StarMark.seen?
    ├── src/calm.ts                      # US2: new, the rule and useCalm
    ├── src/calm.test.ts                 # US2: new
    ├── src/labels.ts                    # US2: ringRadius from seen
    ├── src/App.tsx                      # US2: useCalm on the report; live to FieldList
    ├── src/App.calm.test.tsx            # US2: new
    ├── src/components/Chart.tsx         # US2: ring class, delay, layer filter, legend, caption
    ├── src/components/ChartSeen.test.tsx # US2: new
    ├── src/components/FieldList.tsx     # US2: named stars from seen, kept height while live
    └── dist/                            # US2: rebuilt
DESIGN.md                                # US2: § Chart (signature)
```

## Interfaces

### US1: `crates/finder/src/solve.rs`

```rust
/// Catalog stars listed in a report, brightest first, besides the solver's matches.
const FIELD_STARS: usize = 60;
/// A catalog star counts as detected when a detected star sits this close, in pixels.
const DETECTED_RADIUS_PX: f64 = 2.0;
```

`StarMark` gains, after `matched`:

```rust
    /// Whether any detected star lies within `DETECTED_RADIUS_PX` of `x`, `y`. True
    /// whenever `matched` is.
    pub detected: bool,
```

`star_marks` (`crates/finder/src/solve.rs:505`) takes the report's `detected` positions
as a new argument. It already has a local named `detected`, the matched centroids
distorted back (`crates/finder/src/solve.rs:514`); rename that local to `centroids` so the
new argument is not shadowed. Its matched loop is otherwise unchanged and sets
`detected: true`; a matched mark's `x` and `y` stay the detected centroid. The expected
loop becomes: take `db.nearby_stars(frame.center, frame.half_diagonal())`, which is
brightest-first (`crates/pattern-database/src/format.rs:53`), keep those that project
inside the frame, take the first `FIELD_STARS`, and skip any already matched. For each,
`detected` is true when `detected.iter().any(|[x, y]| (x - px).hypot(y - py) <= DETECTED_RADIUS_PX)`.
`MAX_EXPECTED_STARS` (`crates/finder/src/solve.rs:31`) is removed.

The report's `detected` list is capped at `MAX_DETECTED` = 500
(`crates/finder/src/solve.rs:29`); the cross-match uses that same list. The solver gets
at most 200 detections through the API (`crates/finder/src/api.rs:1158`) and the scope
link, so a matched centroid is always in that list.

### US1: the test harness (`crates/finder/tests/star_detected.rs`)

- Scenarios 1, 3 and 4 solve through `POST /api/solve` with `sample=orion2.jpg`, as
  `crates/finder/tests/api.rs:210` does.
- Scenario 2 needs the detections as `scope_link::proto::Centroid`s: decode with
  `solve::decode_gray`, solve with `solve::solve_frame`, and turn the report's `detected`
  into `Centroid { x, y, brightness: 0.0 }` in the report's order, which is brightest
  first. Remove the first two entries for the second solve. Both solves use default
  `SolveOptions`.
- "The 60 brightest catalog stars in the frame" is computed in the test from the bundled
  database's `nearby_stars`, the solved `Frame`'s `project` and `contains`, taking the
  first 60 that land inside. Scenario 2 uses the first solve's frame.

### US2: `crates/finder/web/src/api.ts`

`StarMark` gains two optional fields:

```ts
  /** A detected star sits where the solution puts this one (server, spec 020 US1). */
  detected?: boolean;
  /** How the page draws it, after the live hold. Page-only; never sent by the server. */
  seen?: boolean;
```

### US2: `crates/finder/web/src/calm.ts`

```ts
export const CALM_FRAMES = 3;
export const CALM_HITS = 2;
export const CALM_SECONDS = 10;

/** One solved live report: when it was exposed, and which star ids that report alone saw. */
export interface CalmEntry { time: number; seen: Set<string> }

/** Seen in this report alone. */
export const seenNow = (s: StarMark) => s.matched || s.detected === true;

/** The report with `seen` set on every star, given the previous solved live reports. */
export function calm(report: SolveReport, time: number, history: CalmEntry[]): SolveReport;

/** Keeps the history for live reports and clears it when the page leaves live. */
export function useCalm(report: SolveReport | null, time: number | null, live: boolean): SolveReport | null;
```

- `calm` returns a new report object whose `stars` are new objects with `seen` set. A
  star is seen when `seenNow(s)`, or when at least `CALM_HITS` of the last `CALM_FRAMES`
  entries of `history` hold its id and the latest entry that holds it has a `time` no
  more than `CALM_SECONDS` before `time`.
- **An entry records `seenNow` of its own report**, never the `seen` that `calm` set.
- `useCalm` is built like `Chart`'s `useSteady`
  (`crates/finder/web/src/components/Chart.tsx:80`): a `useState` holding the last
  `live` flag, the last source report, the calmed report shown and the history, advanced
  during render only when `report !== source` or `live` changed. The same report object
  rendered again returns the same calmed object and appends nothing. This matters
  because `useLive` re-renders `App` every 250 ms
  (`crates/finder/web/src/useLive.ts:61`), and test live reports mostly share one `time`.
- `useCalm` appends an entry only for a new solved report while live, keeps at most
  `CALM_FRAMES`, clears the history when `live` turns false, and returns the report
  unchanged when not live, when unsolved, or when `time` is null. The live time is the
  live event's `time` (`crates/finder/web/src/api.ts:133`).

### US2: `App`, `Chart`, `FieldList`, `labels`

- `App` (`crates/finder/web/src/App.tsx:130`) wraps the report once:
  `const report = useCalm(showLive ? liveReport : uploadedReport, live?.time ?? null, showLive);`.
  It passes `live={showLive}` to `FieldList` (`crates/finder/web/src/App.tsx:251`).
- `Chart` draws the ring class (`crates/finder/web/src/components/Chart.tsx:464`), its
  animation delay (`:465`), the layer filter (`:159`) and the legend count (`:602`) from
  `s.seen ?? seenNow(s)`.
- `Chart`'s picked-star caption (`crates/finder/web/src/components/Chart.tsx:648` to
  `:650`) becomes, for a star that is not matched: `Detected; not used in the solve` when
  `s.detected`, else `Seen in recent frames, not in this one` when `s.seen`, else
  `In the catalog but not detected`. The matched text is unchanged.
- `ringRadius` (`crates/finder/web/src/labels.ts:52`) becomes
  `(s.seen ?? seenNow(s)) ? 7.5 : 4.5`, so a held ring has a matched ring's size and the
  label layout keeps clear of it.
- `FieldList`'s `namedStars` (`crates/finder/web/src/components/FieldList.tsx:15`) filters
  by `s.seen ?? seenNow(s)`. `FieldList` takes an optional `live` prop; while it is true
  the named-stars paragraph is always rendered, with the attribute `data-named-stars`,
  holding a non-breaking space (`" "`) and no ring icon when empty. Without `live`
  it renders exactly as today.

### US2: the test harness

`App.calm.test.tsx` drives live reports the way
`crates/finder/web/src/App.live.test.tsx` does. Its `FakeEventSource` (line 10),
`CONNECTED` (line 30) and `serve` (line 45) are private to that file, so the new file
carries its own copy. Each emitted report gets its own `time`, 3 s apart unless a
scenario says otherwise.

### `DESIGN.md` § Chart (signature)

After 018's live-chart paragraph (`DESIGN.md:284` to `DESIGN.md:287`), add a paragraph of its own:

> A star keeps its solid ring on the live chart through a frame that misses it, if it
> was seen in two of the three frames before, for at most 10 s after it was last seen.
> Tapping a solid ring the solver did not use says why it is solid.

## Shapes that existing tests pin

These are not changed by any story:
- **The solver's count.** `crates/finder/tests/api.rs:237` wants `pointing.matched` = 30,
  and `crates/finder/tests/api.rs:241` exactly 30 marks with `matched: true`.
- **Unmatched marks lie in the frame.** `crates/finder/tests/api.rs:257`.
- **Named belt stars are labelled.** `crates/finder/tests/api.rs:253` wants Alnitak,
  Alnilam and Mintaka among the names.
- **Cedar's stars are matched.** `crates/finder/tests/api.rs:452` wants 90% of cedar's
  HIP ids among the `matched: true` marks.
- **A flipped star follows at once.** `crates/finder/web/src/components/ChartLive.test.tsx:183`
  renders `Chart` directly with every star's `matched` flipped and wants
  `ORION2.stars[0]`'s class to follow. `Chart` falls back to `seenNow`, and fixtures carry
  no `detected`, so it does.
- **The fixtures are captures.** `crates/finder/web/src/test/fixtures/orion2.json` holds a
  saved report with 30 matched and 25 unmatched marks and no `detected`;
  `crates/finder/web/src/components/Chart.test.tsx:26`, `:49` and `:54` count those. No
  story re-records it.
- **The named-stars count.** `crates/finder/web/src/components/FieldList.test.tsx:16`
  counts `matched && name` in `VIRGO`; with no `detected` and no `seen`, `seenNow` gives
  the same set.
- **Live star marks.** `crates/finder/web/src/App.live.test.tsx:108` wants `[data-star]`
  circles at each star's `x` and `y`, then the new count after a `VIRGO` live report. The
  hold changes no position and no membership.
- **The roster on the real binary.** `crates/finder/web/e2e/console.spec.ts:70` and
  `crates/finder/web/e2e/session.spec.ts:100` want the named stars of a solved
  `orion2.jpg` to read exactly `Alnilam, Alnitak, Mintaka`. With US1, Saiph stays
  unmatched and undetected (its nearest detection is 101.5 px away), so the roster does
  not change.
- **An unseen ring exists.** `crates/finder/web/e2e/night-legibility.spec.ts:103` and
  `:129` take the first `.mark-expected circle.ring` on an orion2 chart; after US1 most
  unmatched marks there are not detected, so one exists.

## Decisions and the alternatives rejected

| Decision | Rejected | Why |
|---|---|---|
| A new `detected` field | Widening `matched` | `crates/finder/tests/api.rs:241` pins exactly 30 matched marks. |
| Membership from the 60 brightest in frame | The 25 brightest unmatched; the 40 brightest | The first moves whenever the matched set does. On `orion2.jpg` 40 lists 45 marks, drops seven detected stars, and leaves five matched stars beyond the 40th to come and go; the faintest matched star is the 55th. The owner chose 60. |
| 2.0 px radius | The solution's residual | Matched marks sit 0.04 to 0.36 px from their projected positions on `orion2.jpg`; 2 px covers centroid noise without catching neighbours at this scale. |
| Hold needs two of three | Hold any star seen once | A single sighting is as likely noise as a star. |
| The history records what a frame saw | Recording the drawn value | A recorded hold would keep a star held for as long as it stays in the report (US2-S8). |
| 10 s after the last sighting | No cap; 10 s after the oldest frame | A star the scope has truly lost should not linger; the latest sighting is what US2-S4 measures. |
| Hold in `App` via `useCalm`, built like `useSteady` | Hold inside `Chart`; a ref appended on each render | The list under "In the field" must agree with the chart, and `App` re-renders every 250 ms. |
| A held ring at a matched ring's size | Only the class changes | With the class alone the probe drew a solid ring at the small radius, and labels were laid out against the wrong size. |
| Captions that say why a ring is solid | Today's "not detected" | It would contradict the drawing. The owner chose the wording. |
| Pass more detections to the solver | — | Out of scope: it changes the solve and its timing. |

## Verification (every story, before proposing)

- **The factory gates** (`ergane.yaml` at c1ed295):
  - `lint` = `cargo fmt --all --check`,
    `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`,
    `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features --locked`
    and `RUSTDOCFLAGS="-D warnings" cargo doc -p finder --no-deps`;
  - `test` = `cargo test --workspace --all-targets --all-features --locked`,
    `cargo test --workspace --locked --doc`, and then, in `crates/finder/web`,
    `npm ci --no-audit --no-fund`, `npm run typecheck`, `npm run format:check`,
    `npm test`, `npm run build`, `git status --porcelain -- dist | (! grep .)` and
    `CI=1 npm run e2e`.
- **CI in the merge queue** runs every required `ci.yml` job, including the cedar e2e
  eval. The coverage floor runs only by hand since 2026-11-14.

## Prerequisites before this spec is flipped to `ready`

1. **The earlier specs have landed:** 013, 017, 018 and 019. Met at c1ed295.
2. **`main` is still as read.** Every citation here is at c1ed295. If `solve.rs`,
   `Chart.tsx`, `App.tsx`, `FieldList.tsx`, `labels.ts` or `DESIGN.md` has changed since,
   re-read them.
3. **The owner approvals are given.** The star count and the captions were chosen on
   2026-11-14; flipping gives the rest.
4. **021 and 022 wait for this one.** Both name `020-live-chart-calm` in
   `depends_on_landed`, so neither dispatches before this lands.
5. **The refinement probe has run** (§ Refinement Record).

## Plan traps

1. **Gate evidence or nothing.** *Check:* every Then clause names a committed test that
   a gate runs.
2. **A named command is read literally.** *Check:* commands are copied from
   § Verification.
3. **One evidence budget.** *Check:* each story keeps its diff under 64 KiB, excluding
   `crates/finder/web/dist`. The probe's US1 was 12 KiB and its US2 28 KiB.
4. **A spec fixed mid-epic needs a relaunch.** *Check:* no builder edits
   `specs/020-live-chart-calm/`.
5. **A negative assertion passes when nothing is drawn.** *Check:* US2-S5 and US2-S8
   also assert the star's element exists.
6. **A scenario that is green before the change is a guard, not evidence.** US1-S3 and
   US2-S7 pass on `main` by design. *Check:* each story still has a scenario that fails
   first: US1-S1 and US1-S4; US2-S1, S5, S6, S8, S9 and S10.
7. **One report object, many renders.** `App` re-renders every 250 ms. *Check:* `useCalm`
   returns the same object for the same input, and US2-S5 shows `mark-expected` after
   the second report.
8. **A shadowed name.** `star_marks` already has a local `detected`. *Check:* the local is
   renamed `centroids` before the new argument is added.
9. **The bundle must be rebuilt.** *Check:* US2 commits `crates/finder/web/dist`.
10. **Each worktree builds on its own.** *Check:* no story sets `CARGO_TARGET_DIR`.
11. **Formatting is the repository's.** Prettier reflows some of the plan's one-line
    snippets. *Check:* run `npm run format`, then `npm run format:check`.

## Risks

| Risk | Mitigation |
|---|---|
| More dashed rings crowd a dense field. | 60 is about what the chart showed at the lake (58 marks) and five more than today on `orion2.jpg`; the owner chose the number. |
| A noise detection within 2 px marks a star detected. | At 2 px it needs a coincidence; at worst one ring is solid that should be dashed, and its caption says "Detected". |
| A held star is read as seen when it isn't. | At most 10 s after the last sighting, and only after two of three sightings; its caption says so. |
| A frame where a matched star is fainter than the 60th. | Membership then still moves for that star alone; on `orion2.jpg` the faintest matched star is the 55th. |

## Refinement Record

Drafted on 2026-11-13 by the plate-solver operator against `main` at ce80f67, from the
owner's recording and 8 consecutive live reports read from `/api/live` at the lake site
(not kept). On 2026-11-14 the plan was applied to throwaway worktrees of c1ed295, tests
first, with no existing test edited:

- **US1** (branch `probe/rust`): S1 failed before and passed after. S2 as drafted could
  not pass: with the five brightest detections removed `orion2.jpg` gives `NoMatch`
  (every k from 3 to 12 does), while k of 0, 1 or 2 solves with 30 matched. With two
  removed, the largest move of a common mark is 0.343 px. At 40, S2 passed on `main`
  too, so S4 was added; it fails on `main`, which lists 25 unmatched down to the 54th
  brightest. `fmt`, `clippy -p finder` and `cargo test -p finder --all-targets` passed
  (388 tests, none failed). Re-run at FIELD_STARS = 60 (commit `92a2935`): 60 marks, 30
  matched, 11 unmatched with `detected: true`, none of them named, so the roster stays
  "Alnilam, Alnitak, Mintaka"; every matched star lies inside the 60 in both solves; the
  two reports' mark-id sets are identical; and S1, S2 and S4 fail against c1ed295's
  `solve.rs` and pass after. HIP 28595, the 58th brightest, sits 0.6 px inside the bottom
  edge, hence S2's 2 px margin.
- **US2** (branch `probe/020-page`): `npm test` went from 258 to 278 passing, every
  existing test unchanged; typecheck, `format:check` and build passed, and `dist` was
  clean after rebuilding. The probe found the drafted S5 unsatisfiable, S6 needing
  "exactly one", `ringRadius` missing from the plan, the caption missing, and the
  history's reading unpinned; this trio now carries each fix. Non-live `FieldList`,
  `FieldRoster` and `Chart` markup was byte-identical for the ORION2, VIRGO and ROOF
  fixtures.
- **Not run on the Surface:** `CI=1 npm run e2e` (no Playwright browsers) and the
  workspace-wide Cargo gates. The `test` gate runs both.

Writes come from the task text. Shared writes name the landed stories that last wrote
each file. Nothing in this record is an owner approval: those are in spec.md, and
flipping to `ready` gives them.

```yaml
version: 1
reviewed_revision: c1ed295734253c7d6121052b0a14beb2ee64bc17
stories:
  US1:
    tests:
      - path: crates/finder/tests/star_detected.rs
        status: new
        disposition: add
        assertion: "every matched mark is detected; some unmatched mark is detected; detected marks have a detection within 2 px and others none; with the two brightest detections removed both solve and the 60 brightest at least 2 px inside the edges keep their ids and move < 1 px; matched stays 30; unmatched marks are exactly the 60 brightest less the matched, brightest first"
        decision: "spec.md:US1"
      - path: crates/finder/tests/api.rs
        status: existing
        disposition: preserve
        assertion: "orion2 solves with 30 matched marks on their stars; unmatched marks lie in the frame; the belt stars are named; cedar's stars are matched"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "the solve, pointing.matched and the 30 matched marks on orion2.jpg (crates/finder/tests/api.rs:237, :241)"
      - "cedar parity: 90% of cedar's HIP ids among the matched marks (crates/finder/tests/api.rs:452)"
      - "guidance, the sync model and the scope link are untouched (spec.md FR-003)"
    checks:
      development:
        - "cargo test -p finder --locked --test star_detected"
        - "cargo test -p finder --locked --test api"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs: test, e2e-eval and e2e-canary"
    writes:
      - crates/finder/src/solve.rs
      - crates/finder/tests/star_detected.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/solve.rs
        other_spec: 013-live-finder
        other_story: US6
        disposition: merge_order
        reason: "020 depends_on_landed 013, whose US6 last wrote solve.rs"
    open_questions: []
  US2:
    tests:
      - path: crates/finder/web/src/calm.test.ts
        status: new
        disposition: add
        assertion: "two of three hold; one of one doesn't; three misses drop on the sixth; 10 s after the last sighting drops"
        decision: "spec.md:US2-S1..S4"
      - path: crates/finder/web/src/App.calm.test.tsx
        status: new
        disposition: add
        assertion: "an alternating star is mark-expected after the second report and mark-matched after the third and fourth, with the named stars unchanged between them; the named-stars line stays while live with no name; uploads follow their report; the history records what each frame saw, so a star unseen three times drops on the sixth"
        decision: "spec.md:US2-S5..S8"
      - path: crates/finder/web/src/components/ChartSeen.test.tsx
        status: new
        disposition: add
        assertion: "a seen unmatched ring has the matched class and a matched ring's r; the tap caption says Detected, Seen in recent frames, or not detected"
        decision: "spec.md:US2-S9..S10"
      - path: crates/finder/web/src/components/ChartLive.test.tsx
        status: existing
        disposition: preserve
        assertion: "a star flipped in one re-render of Chart follows at once"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/src/components/Chart.test.tsx
        status: existing
        disposition: preserve
        assertion: "the five chips with the fixture's counts, and the counted mark classes"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/src/components/FieldList.test.tsx
        status: existing
        disposition: preserve
        assertion: "the named stars are the matched named stars of the fixture"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/src/App.live.test.tsx
        status: existing
        disposition: preserve
        assertion: "live star marks at each star's x and y, and the new count after a new report"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/e2e/console.spec.ts
        status: existing
        disposition: preserve
        assertion: "a solved orion2.jpg names exactly Alnilam, Alnitak, Mintaka"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/e2e/night-legibility.spec.ts
        status: existing
        disposition: preserve
        assertion: "an orion2 chart has a .mark-expected ring whose night style is read"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "uploaded and sample frames draw exactly their report (spec.md FR-005)"
      - "non-live FieldList, FieldRoster and Chart markup is unchanged for reports without detected"
      - "red-only night: no colour or token changes (DESIGN.md)"
      - "the header's N of M stars matched keeps the solver's count"
    checks:
      development:
        - "npm test"
        - "npm run build"
        - "CI=1 npm run e2e"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs: finder-page"
    writes:
      - DESIGN.md
      - crates/finder/web/dist
      - crates/finder/web/src/api.ts
      - crates/finder/web/src/calm.ts
      - crates/finder/web/src/calm.test.ts
      - crates/finder/web/src/labels.ts
      - crates/finder/web/src/App.tsx
      - crates/finder/web/src/App.calm.test.tsx
      - crates/finder/web/src/components/Chart.tsx
      - crates/finder/web/src/components/ChartSeen.test.tsx
      - crates/finder/web/src/components/FieldList.tsx
    no_write_reason: ""
    shared_writes:
      - path: DESIGN.md
        other_spec: 017-console-stop-confirm
        other_story: US5
        disposition: merge_order
        reason: "020 depends_on_landed 017, whose US5 last wrote DESIGN.md"
      - path: crates/finder/web/dist
        other_spec: 017-console-stop-confirm
        other_story: US5
        disposition: merge_order
        reason: "020 depends_on_landed 017, whose US5 last rebuilt the bundle"
      - path: crates/finder/web/src/api.ts
        other_spec: 017-console-stop-confirm
        other_story: US5
        disposition: merge_order
        reason: "020 depends_on_landed 017, whose US5 last wrote api.ts"
      - path: crates/finder/web/src/App.tsx
        other_spec: 018-live-chart-steady
        other_story: US1
        disposition: merge_order
        reason: "020 depends_on_landed 018, whose US1 last wrote App.tsx"
      - path: crates/finder/web/src/components/Chart.tsx
        other_spec: 018-live-chart-steady
        other_story: US1
        disposition: merge_order
        reason: "020 depends_on_landed 018, whose US1 last wrote Chart.tsx and added useSteady"
    open_questions: []
```
