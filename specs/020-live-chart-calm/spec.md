---
state: draft
depends_on_landed: ["013-live-finder", "017-console-stop-confirm", "018-live-chart-steady", "019-night-legibility"]
# Drafted 2026-11-13 against `main` at ce80f67, from the owner's screen recording of the
# live chart at the lake site that night ("still blinking, that's annoying as hell").
# Refined 2026-11-14 against `main` at c1ed295: both stories were built in throwaway
# worktrees (plan.md § Refinement Record), the scenarios they showed to be unsatisfiable
# were reworded, and the owner chose the star count (60) and the tap captions. It stays
# draft until the owner flips it to `ready` on Tharpebox, which gives the approvals listed
# under "Owner approvals" and dispatches it.
---

# Feature Specification: A Live Chart That Stays Calm

**Feature Branch**: `020-live-chart-calm`

**Created**: 2026-11-13

**Status**: Draft

**Input**: With 018 landed, the live chart no longer redraws on every frame. It still
blinks. On 2026-11-13 at a dark site (3 s exposures at gain 16), across 8 consecutive
solved live frames, 30 of the 58 catalog stars in the field flipped between matched (a
solid ring and a name) and unseen (a dashed ring). Saiph, magnitude 2.1, went matched,
unseen, matched, unseen, matched, unseen, matched, matched. The matched count per frame
was 30, 25, 30, 26, 28, 30, 27, 29. The named stars under "In the field" appeared and
vanished with them, and every row below that line moved up and down. Pointing and
guidance were steady throughout. This is cosmetic, and it makes the page hard to read
at the eyepiece.

## Clarifications

### Session 2026-11-13 (review of `main` at ce80f67)

- Q: What does this trio read besides itself? → A: Nothing about the feature. `spec.md`
  holds every requirement and scenario, and `plan.md` holds every design decision, the
  interfaces, the shapes that existing tests pin and the alternatives rejected. A story
  that would change a decision escalates rather than re-deciding it.
- Q: What was found? → A: Four things, each checked against the code at ce80f67 and
  again at c1ed295:
  1. The solver gets only the 30 brightest detections (`stars: 30`,
     `crates/finder/src/solve.rs:66`, taken at `crates/finder/src/solve.rs:419`). The
     live loop keeps that default (`crates/finder/src/live.rs:200`).
  2. A star mark's `matched` is the solver's own matched set
     (`crates/finder/src/solve.rs:523` to `crates/finder/src/solve.rs:540`). Under a dark
     sky the 30 brightest detections include noise, and the brightest stars can be missed
     (dark-sky detection, provisionally spec 023, see Assumptions). So which catalog
     stars the solver matches changes from frame to frame, even when the same stars are
     detected.
  3. Unmatched marks are the 25 brightest catalog stars in the frame that the solver did
     not match (`MAX_EXPECTED_STARS`, `crates/finder/src/solve.rs:31`, applied at
     `crates/finder/src/solve.rs:549`). That set depends on the matched set, so a star the
     solver drops can also drop out of the report altogether.
  4. The page draws the ring from `matched` alone
     (`crates/finder/web/src/components/Chart.tsx:464`), and the named-stars line from
     `matched && name` (`crates/finder/web/src/components/FieldList.tsx:16`). The line is
     rendered only when it has a name (`crates/finder/web/src/components/FieldList.tsx:49`),
     so the rows below it move whenever it comes or goes.
- Q: Can the server mark every detected catalog star as `matched`? → A: No.
  `crates/finder/tests/api.rs:237` and `crates/finder/tests/api.rs:241` want exactly 30
  matched marks for `orion2.jpg`, the solver's count. A new field carries the wider
  meaning (FR-001), and `matched` keeps the solver's.

### Session 2026-11-14 (refinement probe at c1ed295, and the owner's answers)

- Q: Why does the hold need two sightings in three frames, and not one? → A: A star seen
  once is as likely a coincidence as a star. Requiring two of the previous three keeps a
  noise detection from latching a ring solid. (The draft also cited
  `crates/finder/web/src/components/ChartLive.test.tsx:183`; that test renders `Chart`
  directly, and the hold lives in `App`, so it does not decide this either way.)
- Q: How many catalog stars does the report list? → A: The solver's matches plus every
  other star among the 60 brightest catalog stars in the frame. The owner chose 60 on
  2026-11-14 over the draft's 40. On `orion2.jpg` the probe measured that 40 would list
  45 marks where today lists 55, drop seven faint stars the detector found, and still let
  five matched stars fainter than the 40th come and go with the solver. The faintest
  matched star there is the 55th brightest in the frame.
- Q: Does the history record what a frame saw, or what the page drew? → A: What the frame
  saw: `matched || detected` of that report alone. Recording the drawn value would let a
  held star hold itself for as long as it stays in the report (US2-S8).
- Q: What does a tap on a solid ring that the solver did not use say? → A: What happened
  (owner, 2026-11-14): "Detected; not used in the solve" when a detected star sits on it,
  and "Seen in recent frames, not in this one" when the hold keeps it (FR-009).
- Q: Which scenarios could not pass as drafted? → A: Three, now reworded. `orion2.jpg`
  does not solve with its five brightest detections removed (status `NoMatch`), but does
  with two (US1-S2). In the draft's US2-S5 the second of four alternating reports has
  only one sighting behind it, so the named stars cannot be the same after each report;
  and US2-S6 needed "exactly one" earlier report with the named star.

## Owner approvals

Flipping this spec to `ready` approves the following. Each is named in the FR that makes
it. No existing test, golden or CI file changes.

1. **Both parts are built.** US1 (server) removes most of the churn; US2 (page) hides
   what is left. To build only one, delete the other story from the Work Graph and its
   phase from tasks.md before flipping.
2. **A new `detected` field on every star mark**, true when a detected star sits within
   2 px of where the solution puts the catalog star (FR-001).
3. **Membership from the 60 brightest:** the report lists the solver's matches plus every
   other star among the 60 brightest catalog stars in the frame, where today it lists the
   25 brightest unmatched (FR-002). On `orion2.jpg` the chart shows 60 marks where it
   shows 55 today. Chosen by the owner on 2026-11-14.
4. **The hold:** on the live chart, a star seen in two of the previous three solved live
   frames keeps its solid ring through an unseen frame, for at most 10 s after it was last
   seen (FR-006).
5. **The "Matched" chip counts solid rings** as drawn, not the solver's count (FR-007).
   The header's "N of M stars matched" keeps the solver's count.
6. **The tap captions** "Detected; not used in the solve" and "Seen in recent frames, not
   in this one" (FR-009). Chosen by the owner on 2026-11-14.

## User Scenarios & Testing *(mandatory)*

US1's scenarios run in `cargo test --workspace --all-targets --all-features --locked`,
against the bundled database and cedar's sample `orion2.jpg`, as
`crates/finder/tests/api.rs:210` does. US2's run in `npm test` in `crates/finder/web`,
with `@testing-library/react` and the fixtures in `crates/finder/web/src/test/fixtures.ts`.
"The 60 brightest catalog stars in the frame" is computed by the test itself, from the
bundled database's `nearby_stars` and the solved `Frame`'s `project` and `contains`; that
is the oracle.

### User Story 1 - The report says which catalog stars were detected (Priority: P1)

As the console, after each solve I mark every catalog star in the frame that a detected
star sits on, not only the ones the solver happened to use, and I list the brightest
catalog stars in the frame whichever of them the solver matched.

**Why this priority**: It removes the cause. The same stars are detected frame after
frame, and the solver's choice among them no longer decides the chart.

**Independent Test**: `cargo test --workspace --all-targets --all-features --locked`
runs `crates/finder/tests/star_detected.rs`.

**Acceptance Scenarios**:

1. **Given** `orion2.jpg` solved through `POST /api/solve`, **When** a committed test
   reads `stars`, **Then** it asserts that:
   - every mark with `matched: true` has `detected: true`;
   - at least one mark with `matched: false` has `detected: true`;
   - every mark with `detected: true` has an entry of the report's `detected` within
     2.0 px of its `x` and `y`, and every mark with `detected: false` has none.
2. **Given** `orion2.jpg`'s detections solved with `solve::solve_centroids`, and the same
   list with its two brightest entries removed, **When** a committed test compares the
   two reports, **Then** it asserts that both solve, that the ids of the 60 brightest
   catalog stars in the first frame that lie at least 2 px inside its edges are present
   in both, and that each such star's `x` and `y` differ by less than 1 px between them.
   (One of the 60 on `orion2.jpg` sits 0.6 px inside the bottom edge.)
3. **Given** `orion2.jpg` solved as in scenario 1, **When** a committed test reads
   `pointing.matched` and the count of marks with `matched: true`, **Then** it asserts both
   are 30, as today.
4. **Given** `orion2.jpg` solved as in scenario 1, **When** a committed test reads the
   marks with `matched: false`, **Then** it asserts that they are exactly the 60
   brightest catalog stars in the frame less the matched ones, brightest first.

---

### User Story 2 - The live chart holds a star's look through a blink (Priority: P2)

As the observer at the eyepiece, I see a star that keeps showing up in the live frames
keep its solid ring and its name, even when one frame misses it. The named stars under
"In the field" stay put, and the rows below them don't move. Uploaded and sample frames
show exactly what their report says.

**Why this priority**: It hides what US1 leaves, such as a bright star the detector
misses in alternate frames.

**Independent Test**: `npm test` in `crates/finder/web` runs
`crates/finder/web/src/calm.test.ts`, `crates/finder/web/src/App.calm.test.tsx` and
`crates/finder/web/src/components/ChartSeen.test.tsx`.

**Acceptance Scenarios**:

1. **Given** four solved live reports 3 s apart in which star S is seen, unseen, seen,
   unseen, **When** a committed test applies `calm` to the fourth, **Then** it asserts
   that S is drawn seen.
2. **Given** two solved live reports in which S is seen, then unseen, **When** a committed
   test applies `calm` to the second, **Then** it asserts that S is drawn unseen.
3. **Given** six solved live reports 3 s apart in which S is seen three times, then unseen
   three times, **When** a committed test applies `calm` to each, **Then** it asserts that
   S is drawn seen through the fifth and unseen on the sixth.
4. **Given** three solved live reports 4 s apart in which S is seen, seen, unseen, **When**
   a committed test applies `calm` to a fourth report 12 s after the second in which S is
   unseen, **Then** it asserts that S is drawn unseen.
5. **Given** the App with four live reports 3 s apart in which `ORION2.stars[0]` is
   matched, unmatched, matched, unmatched, **When** a committed test advances them one at
   a time, **Then** it asserts that after the second that star's `[data-star]` element
   has the class `mark-expected`; that after the third and after the fourth it exists with
   the class `mark-matched`; and that the "In the field" named-stars text after the fourth
   equals the text after the third.
6. **Given** the App showing exactly one live report with a named matched star, **When** a
   committed test advances to a live report with no named star seen, three times,
   **Then** it asserts that the named-stars line (`[data-named-stars]`) is still in the
   document after each, holding no name. (jsdom has no layout, so the test asserts the
   element that holds the height, not the height.)
7. **Given** an uploaded report in which a star is unmatched and not detected, after
   three live reports in which it was seen, **When** a committed test shows the upload,
   **Then** it asserts that the star's element has the class `mark-expected`.
8. **Given** the App with six live reports 3 s apart in which `ORION2.stars[0]` is matched
   three times, then unmatched three times, **When** a committed test advances them one
   at a time, **Then** it asserts that the star's element has the class `mark-matched`
   after each of the first five and `mark-expected` after the sixth.
9. **Given** `Chart` rendered with a report holding star A with `matched: true` and star
   B with `matched: false` and `seen: true`, **When** a committed test reads their ring
   circles, **Then** it asserts that B's ring has the class `mark-matched` and the same
   `r` as A's ring; and with B's `seen` false, that B's ring is smaller than A's.
10. **Given** `Chart` rendered with a report holding unmatched stars C (`detected: true`),
    D (`detected: false`, `seen: true`) and E (`detected: false`, `seen: false`), **When**
    a committed test taps each, **Then** it asserts the caption under the chart reads
    `Detected; not used in the solve` for C, `Seen in recent frames, not in this one` for
    D, and `In the catalog but not detected` for E; and that a matched star's caption
    still begins `Matched;`.

---

### Edge Cases

- **The field moves.** Holds are by catalog id and apply only to stars in the newest
  report, at the newest positions. A star that leaves the report is not drawn.
- **An unsolved live frame.** 018's hold keeps the last solve; `calm` advances only on
  solved reports. The "In the field" section is hidden on an unsolved frame today
  (`crates/finder/web/src/App.tsx:246`), and stays so.
- **The first frames.** Nothing can be held in the first two solved reports after the
  page goes live or comes back from an upload: there is no track record yet.
- **Switching between upload and live.** Leaving live clears the history.
- **Reduced motion and night mode.** Unchanged. No colour or token changes.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Every `StarMark` SHALL carry `detected: bool`: true when `matched` is true,
  or when an entry of the report's `detected` lies within `DETECTED_RADIUS_PX` = 2.0 px of
  the mark's `x` and `y`. `matched` and `pointing.matched` SHALL keep their meaning, the
  solver's.
- **FR-002**: The report's `stars` SHALL be the solver-matched stars, then every other
  catalog star among the `FIELD_STARS` = 60 brightest catalog stars inside the frame,
  brightest first. `MAX_EXPECTED_STARS` is retired.
- **FR-003**: Nothing about the solve, the pointing, the guidance, the sync model or the
  scope link SHALL change.
- **FR-004**: The page SHALL draw a star "seen" (`mark-matched`, a matched star's ring
  size, its label, and a name under "In the field") from `seen`, defined in FR-005 and
  FR-006. Without either, `seen` is `matched || detected`, with `detected` false when
  absent.
- **FR-005**: For uploaded and sample frames, `seen` SHALL be `matched || detected` of
  that report.
- **FR-006**: For live frames, `seen` SHALL also be true when the star was seen in at
  least `CALM_HITS` = 2 of the previous `CALM_FRAMES` = 3 solved live reports, and the
  latest of those reports that saw it is no more than `CALM_SECONDS` = 10 s before the
  newest by live time. Each history entry records which stars that report alone saw
  (`matched || detected`), never the held value. The history advances once per new
  solved report and clears when the page leaves live.
- **FR-007**: The legend's "Matched" and "Unseen" chips SHALL count and filter by `seen`.
- **FR-008**: While live, the named-stars line under "In the field" SHALL keep its height
  when it has no name, so the content below it doesn't move.
- **FR-009**: A tapped star's caption SHALL say, when it is not matched:
  `Detected; not used in the solve` if `detected` is true; otherwise
  `Seen in recent frames, not in this one` if `seen` is true; otherwise, as today,
  `In the catalog but not detected`. A matched star's caption is unchanged.
- **FR-010**: `DESIGN.md` § Chart (signature) SHALL record the hold, and
  `crates/finder/web/dist` SHALL be the build of its source.

### Key Entities

- **Detected mark**: a catalog star with a detected star within 2 px of where the
  solution puts it.
- **Seen**: how the page draws a star: solid ring at a matched star's size, label and
  name.
- **Hold**: a star seen in two of the previous three solved live frames, kept seen for
  at most 10 s after it was last seen.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario above has a committed test that asserts it. The
  `lint` and `test` gates pass.
- **SC-002**: No existing test, golden, `scripts/ci/` file, `.github/workflows/ci.yml`,
  `CODEOWNERS` entry or `ergane.yaml` is edited.
- **SC-003**: `crates/finder/web/dist` is the build of its source, as the `test` gate's
  `git status --porcelain -- dist` check proves.
- **SC-004**: On the owner's next night out, replaying a run of 8 solved live frames
  shows no more than 5 of the stars in the field changing look. This is checked by the
  owner, not by a gate.

## Assumptions

- **Earlier features are in place.** 013, 017, 018 and 019 have landed. Every citation
  here was read at c1ed295.
- **Out of scope:**
  - detection itself. Bright stars that saturate into a flat top, and noise that passes
    as stars when the sky background is near zero, belong to a separate spec on dark-sky
    detection (provisionally 023);
  - how many detections go to the solver (`stars: 30`);
  - the header's "N of M stars matched" and RMS, which keep the solver's numbers;
  - the chart's rotation (spec 021) and the push-to marks (spec 022), which build on this
    spec once it has landed;
  - the MCP tools' payloads beyond the new field appearing in reports;
  - `.impeccable/design.json`, which no gate reads; 018 left it alone for the same
    section of `DESIGN.md`;
  - the archived OpenSpec text in `openspec/specs/finder-console/spec.md` that still says
    "up to 25".
- **The factory's gates and CI:** the gates are `lint` and `test`, copied in plan.md
  § Verification. CI's other jobs run only in the merge queue.

## Work Graph

```yaml
US1:
  depends_on: []
  implements: [FR-001, FR-002, FR-003]
US2:
  depends_on: []
  depends_on_merged: [US1]
  implements: [FR-004, FR-005, FR-006, FR-007, FR-008, FR-009, FR-010]
```

US2 shares no file with US1. It waits for US1 to merge so that its `test` gate's
Playwright run serves a console that already sends `detected`, and the page is checked
against the real field rather than only against fixtures.
