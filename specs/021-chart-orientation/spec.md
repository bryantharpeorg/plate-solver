---
state: draft
depends_on_landed: ["012-targets-and-push-to", "013-live-finder", "016-mcp", "017-console-stop-confirm", "018-live-chart-steady", "019-night-legibility", "020-live-chart-calm", "022-altaz-guidance"]
# Drafted 2026-11-13 against `main` at ce80f67, from the owner's report at the lake site
# that the chart moved sideways to his pushes, and his request for a setting "to turn via
# some offset degree". Refined 2026-11-14 against `main` at 0a33c19: every story was built
# in throwaway worktrees (plan.md § Refinement Record), and the owner dropped the
# automatic horizon level, so the typed angle is the whole feature. It waits for 020 and
# 022 to land. It stays draft until the owner flips it to `ready` on Tharpebox, which
# gives the approvals listed under "Owner approvals" and dispatches it.
---

# Feature Specification: A Chart Turned to the Sky

**Feature Branch**: `021-chart-orientation`

**Created**: 2026-11-13

**Status**: Draft

**Input**: The finder page draws each frame the way the camera captured it. On
2026-11-13 the finder camera sat roughly a quarter turn round in its holder. Orion's belt,
standing upright in the sky, lay almost flat on the page: Alnitak at (440, 393) and
Mintaka at (708, 343), about 79° clockwise of the sky's own up. Pushing the scope up moved
the stars sideways on the page, so the chart could not guide a push by eye. The owner
wants a setting that turns the view by any number of degrees, kept with the session, and
the chart, the live photo and the aim-point tap all turned alike.

## Clarifications

### Session 2026-11-13 (review of `main` at ce80f67)

- Q: What does this trio read besides itself? → A: Nothing about the feature. `spec.md`
  holds every requirement and scenario, and `plan.md` holds every design decision, the
  interfaces, the shapes that existing tests pin and the alternatives rejected. A story
  that would change a decision escalates rather than re-deciding it.
- Q: What was found? → A: Five things, each checked again at 0a33c19:
  1. The chart draws in the frame's own pixels: its SVG `viewBox` is the frame plus a
     paper margin (`crates/finder/web/src/components/Chart.tsx:183`), and its aspect is
     `W / H` (`crates/finder/web/src/components/Chart.tsx:177`).
  2. The compass draws north at the solve's `north_deg`, degrees clockwise from up
     (`crates/finder/web/src/components/Chart.tsx:521`, from
     `crates/finder/src/sky.rs:168`). Drawn inside a turned chart it stays correct.
  3. The live photo is a plain `<img>` under the chart
     (`crates/finder/web/src/components/LivePreview.tsx:6`).
  4. The aim tap maps a click to frame pixels by scaling against the photo's bounding box
     (`crates/finder/web/src/components/AimTap.tsx:57` to
     `crates/finder/web/src/components/AimTap.tsx:61`). A turned photo needs the inverse
     turn as well.
  5. The session persists the aim point in `session.json`
     (`crates/finder/src/session.rs:572` and `crates/finder/src/session.rs:1413`), and
     omits it from reads until set (`crates/finder/src/session.rs:550`). The stored struct
     denies unknown fields (`crates/finder/src/session.rs:1397`).
- Q: What already exists? → A: Only a local hotfix used that night, uncommitted and not on
  `main`: a "⟲ Turn 90°" chip under the chart cycling 0, 90, 180 and 270°
  counter-clockwise, kept in the browser's `localStorage` as `finder.turn`. It turned
  neither the aim tap nor any angle but quarter turns. This spec replaces it, and it stays
  off `main`.
- Q: Which way is positive? → A: Counter-clockwise on screen, as in the owner's "rotate
  90° counter-clockwise" and the hotfix's chip.
- Q: Is a fixed offset enough? → A: On the owner's Dobsonian, yes. The tube never rolls,
  so a camera fixed to it keeps the same angle to the horizon everywhere in the sky.

### Session 2026-11-14 (refinement probe at c1ed295, and the owner's answers)

- Q: Is the automatic horizon level built? → A: No. The owner dropped it on 2026-11-14.
  The typed angle is the whole feature. 022's `up_deg` is drawn as its up mark and does
  not turn the chart.
- Q: Why two stories for one setting? → A: Built together, the setting and the turned
  views came to about 55 KiB of diff in the probe before the label and arrow fixes below,
  against a 64 KiB budget per story. US1 keeps the setting; US2 turns the views by it.
- Q: Which word does the code use? → A: "Rotation", as in `chart_rotation_deg`. "Turn"
  already names the base's motion in azimuth (`GLOSSARY.md`, and the guidance's
  `turn_deg`).
- Q: Which frames turn? → A: Live ones only: the live chart, the live photo under it, the
  aim-point photo and the live chart's "Just off the frame" arrows. The setting describes
  the finder camera, and only live frames are sure to come from it; uploads and samples
  draw as captured (Owner approvals, 3).
- Q: Do labels stay readable when turned? → A: Not if they are only turned back. The
  probe turned each label back about its own middle, with the layout left in the frame's
  axes. At 90° on `ORION2` two labels landed on their own star and three on other stars'
  rings, and one was clipped by 12.4 px. So labels are laid out in the turned view, where
  they are seen (FR-006). The grid's left and right edge labels carry their own rotation,
  so at 90° the left ones read upside down. They read along their edges, as today, with a
  half turn added to any that would read upside down (FR-006).
- Q: What breaks if only `session.rs` changes? → A: The crate stops compiling.
  `crates/finder/src/mcp/tools.rs:364` and `crates/finder/src/mcp/tools.rs:400`
  destructure `SessionEdit` field by field on purpose ("a new session field must be
  considered here explicitly"). Both MCP tools refuse the new field, like the other
  fields they don't change (FR-002).
- Q: Which page files did the draft miss? → A: `crates/finder/web/src/useSession.ts`,
  whose read drops any field it doesn't name (`crates/finder/web/src/useSession.ts:29` to
  `crates/finder/web/src/useSession.ts:48`) and which holds the page's `SessionEdit`
  (`crates/finder/web/src/useSession.ts:6`); and `crates/finder/web/src/App.tsx`, which
  places `LivePreview` (`crates/finder/web/src/App.tsx:241`) and sizes the page from the
  frame's aspect (`crates/finder/web/src/App.tsx:145` to
  `crates/finder/web/src/App.tsx:148`).
- Q: Does `value.rem_euclid(360.0)` keep values in [0, 360)? → A: Not quite. Through
  `PUT /api/session` the probe stored `-1e-20` as `360.0` and `-0.0` as `-0.0`. Both are
  stored as 0 (FR-001, US1-S1).
- Q: How bad is a downgrade? → A: Worse than the draft said. An older console binary given
  a `session.json` with the field prints "invalid session state JSON; keeping it as .bad",
  renames the file `session.json.bad` and starts a fresh session. The probe ran the real
  binary: the site, the pairing address and token were gone, and with them go the camera
  settings, the aim point and the pointing model. The `.bad` file keeps every byte, so
  deleting the one key from it and renaming it back recovers everything (Owner
  approvals, 1).
- Q: Can the page tests use `ORION2` as drafted? → A: Not for sizes. It is 507 × 507, so
  a turned box's width and height cannot be told apart. The size scenarios use the
  1296 × 972 frame the live camera sends.
- Q: What can jsdom check about the upright labels? → A: Not the drawn result. The suite
  loads no CSS, jsdom ignores rules inside `@layer`, and it doesn't resolve `var()`. It
  can check the class, the custom property, which elements the rule's selector matches,
  and that the rule is in `index.css` (US2-S3). How the label looks in a browser is the
  owner's to see (SC-004).
- Q: Which box does a tap measure against? → A: The photo's own, as today
  (`crates/finder/web/src/components/AimTap.tsx:57`). The existing tests mock only the
  `<img>`'s box, so measuring the button's would fail three of them; a turned photo's box
  is the turned frame's.

## Owner approvals

Flipping this spec to `ready` approves the following. Each is named in the FR that makes
it. No existing test, golden or CI file changes.

1. **A new session field, `chart_rotation_deg`**, kept in `session.json` and omitted from
   reads until set (FR-001). One risk: a console binary from before this spec, given a
   `session.json` that has the field, renames it `session.json.bad` and starts a fresh
   session. That loses the site, the pairing address and token, the camera settings, the
   aim point and the pointing model. Clearing the setting before a downgrade avoids it;
   deleting the key from `session.json.bad` and renaming it back to `session.json`
   recovers it.
2. **A "Chart rotation" section in Setup**, in degrees, after Minimum altitude (FR-003).
3. **Only live views turn.** Uploads and samples draw as captured (FR-009).
4. **Labels read upright** and are laid out where they are seen; an edge label that would
   read upside down gets a half turn (FR-006).
5. **The MCP tools refuse the field** with their existing messages (FR-002). An MCP
   client sees it in session reads.
6. **Two stories**, US1 the setting and US2 the turned views, for the diff budget.

## User Scenarios & Testing *(mandatory)*

Server scenarios run in `cargo test --workspace --all-targets --all-features --locked`.
The restart in US1-S3 runs the binary on a state folder, as
`crates/finder/tests/session_cli.rs:43` does; the MCP calls in US1-S4 go through the
endpoint as `crates/finder/tests/mcp_session_tools.rs:185` does. Page scenarios run in
`npm test` in `crates/finder/web`, with `@testing-library/react`, `DEFAULT_LAYERS` and
the fixture `ORION2` from `crates/finder/web/src/test/fixtures.ts`. `WIDE` is `ORION2`
with `width` 1296 and `height` 972, built in the test file. jsdom has no layout, so page
tests assert attributes, inline styles and mocked bounding boxes, as
`crates/finder/web/src/components/AimTap.test.tsx:52` does.

### User Story 1 - The console keeps the camera's angle (Priority: P1)

As the observer, I enter the angle my camera sits at in Setup once, and the console keeps
it with the session, across restarts and for every device that views it.

**Why this priority**: The turned views in US2 read it. On its own it is a saved number.

**Independent Test**: `cargo test --workspace --all-targets --all-features --locked`
runs `crates/finder/tests/session_rotation.rs`; `npm test` runs
`crates/finder/web/src/components/SetupRotation.test.tsx`.

**Acceptance Scenarios**:

1. **Given** a console, **When** a committed test sends
   `PUT /api/session {"chart_rotation_deg": 450}`, then `-90`, `0`, `-1e-20` and `-0.0`,
   **Then** it asserts each answer and the read after it give 90, 270, 0, 0 and 0, and
   that each 0 has a positive sign.
2. **Given** a console with `chart_rotation_deg` 30, **When** a committed test sends
   `"x"`, `true`, `[90]` and `"90"` for it, **Then** it asserts each answers 400 with a
   JSON `error` and leaves the whole read unchanged; **and** that a fresh console's read
   has no `chart_rotation_deg` key.
3. **Given** a console with a state folder and `chart_rotation_deg` 90, **When** a
   committed test restarts the binary on the same folder, **Then** it asserts the read
   gives 90 and `session.json` holds it; **and when** it then sends `null`, **then** the
   read and `session.json` have no `chart_rotation_deg` key.
4. **Given** a console, **When** a committed test calls the MCP tool `set_target` with
   `{"target": "M42", "chart_rotation_deg": 90}` and `set_session` with the same
   arguments, **Then** it asserts they are refused with "set_target changes only target"
   and "set_session changes only site, min_alt_deg and target", and that the read is
   unchanged.
5. **Given** `SetupPage` rendered with a session whose `chart_rotation_deg` is 79 and a
   mocked `put`, **When** a committed test reads the page, **Then** it asserts a section
   titled "Chart rotation" comes after "Minimum altitude" and before the scope node's
   section, and that its field holds `79`; **and when** it enters `85` and presses "Save
   chart rotation", **then** `put` receives `{"chart_rotation_deg": 85}` and the section
   says "Chart rotation saved."
6. **Given** the same, **When** a committed test empties the field and saves, **Then** it
   asserts `put` receives `{"chart_rotation_deg": null}` and the section says "Chart
   rotation cleared."; **and when** it enters `abc` and saves, **then** `put` is not
   called and the section says "Chart rotation must be a number of degrees."
7. **Given** `useSession` with a mocked `fetch` whose read answers `chart_rotation_deg`
   90, **When** a committed test reads the hook's session, **Then** it asserts it has
   `chart_rotation_deg` 90; and that a read without the field gives a session without
   the key.

---

### User Story 2 - The live views turn by it (Priority: P1)

As the observer, the live chart, the live photo and the aim-point photo are all turned by
the angle I set, with their labels upright and clear of the stars, the compass still true,
and a tap on the turned aim-point photo setting the aim at the frame pixel under my
finger.

**Why this priority**: It is what the owner asked for, and it makes push-to by eye work
on his Dob.

**Independent Test**: `npm test` runs `crates/finder/web/src/labelsRotation.test.ts`,
`crates/finder/web/src/components/ChartRotation.test.tsx` and
`crates/finder/web/src/App.rotation.test.tsx`.

**Acceptance Scenarios**:

1. **Given** `Chart` rendered live with `WIDE` and a session whose `chart_rotation_deg`
   is 90, **When** a committed test reads the SVG, **Then** it asserts the `viewBox` is
   `-m -m (972 + 2m) (1296 + 2m)` with `m` = 30 × 972 / 320 (jsdom measures no width, so
   one CSS pixel is the turned box's width over 320), that one group holds the
   frame with the transform `translate(486 648) rotate(-90) translate(-648 -486)`
   exactly, and that the paper's edge is a child of the SVG outside that group.
2. **Given** the same at 30°, **When** a committed test reads the SVG and the figure,
   **Then** it asserts the `viewBox` width is `1296·cos 30° + 972·sin 30° + 2m` and its
   height `1296·sin 30° + 972·cos 30° + 2m`, with `m` 30 × the box's width over 320, each
   to 0.01, and that the figure's `--chart-aspect` is
   `(1296·cos 30° + 972·sin 30°) / (1296·sin 30° + 972·cos 30°)`, to 1e-9.
3. **Given** `Chart` rendered live with `ORION2`, the grid on and `chart_rotation_deg`
   90, **When** a committed test reads the SVG, **Then** it asserts the SVG has the class
   `chart-rotated` and the inline style `--label-rotation: 90deg`; that every star,
   object and compass label matches `.chart-rotated text:not([transform])`; that
   `crates/finder/web/src/index.css` holds that rule with `rotate: var(--label-rotation)`;
   and that every grid edge label has a `transform`, with `rotate(180` in it on the left
   edge only. At 180° it asserts `rotate(180` on the top and bottom edges only.
4. **Given** `layoutLabels` with `WIDE`'s stars, objects and detections and `rotation`
   90, then 79, **When** a committed test reads the spots, **Then** it asserts that no
   star label's `box` overlaps another label's box, or the square around any star's ring
   or any detection centred at `toView` of its pixel; that every star label's box, its
   corners taken back through `fromView`, lies inside the frame by at least 3u; and that
   `toView` of the spot's anchor, moved by the box's centre less the box's own anchor
   point (plan.md § Interfaces), gives the box's centre, to 1e-6. With `rotation` 0 it
   asserts the spots equal those with no `rotation`.
5. **Given** `AimTap` showing a live preview of 1296 × 972 with `chart_rotation_deg` 90
   and the photo's bounding box mocked at 364 × 485.33 CSS px, **When** a committed test
   clicks at box pixel (200, 996) scaled to that box, **Then** it asserts the aim request
   carries x and y within 0.5 px of (300, 200). It asserts the same round trip at 0° and
   30°, with the click placed by `toView`.
6. **Given** `LivePreview` with width 1296, height 972 and rotation 90, **When** a
   committed test reads its box and photo, **Then** it asserts the box's
   `aspect-ratio` is `0.75 / 1` and the photo's transform is
   `translate(-50%, -50%) rotate(-90deg)`.
7. **Given** `NextTargets` with `ORION2`'s `nearby` and rotation 90, **When** a committed
   test reads the arrows, **Then** it asserts each arrow's group has the transform
   `rotate(d − 90)` for its object's `direction_deg` `d`, and each keeps the label
   `${Math.round(d)}° from up on the frame`.
8. **Given** `Chart` rendered three ways, live with no `chart_rotation_deg`, live with 0,
   and not live with 90, **When** a committed test compares each SVG's markup with the
   same chart's with the field absent, **Then** it asserts they are identical; and that
   `LivePreview` with rotation 0 sets no inline style.
9. **Given** the page live with a 1296 × 972 report and a session whose
   `chart_rotation_deg` is 90, **When** a committed test reads it, **Then** it asserts
   the chart's frame group has `rotate(-90)` in its transform, the page's
   `--chart-aspect` is 0.75, and the live photo's box has `aspect-ratio` `0.75 / 1`.

---

### Edge Cases

- **Angles past a full turn or negative.** Normalised to [0, 360), with −0 as 0
  (FR-001).
- **An unsolved live frame.** The chart keeps 018's hold, turned the same way.
- **Uploads and samples.** Drawn as captured (FR-009).
- **Changing the setting between 0 and another angle.** The frame's marks move into or
  out of the turned group, so React draws them afresh and they ink in once, as on a new
  chart. Between two angles other than 0 they update in place.
- **The placeholder while the first frame solves** stays unturned; it shows for about a
  second before the first report.
- **Night mode and reduced motion.** Unchanged.
- **The phone.** The setting is the session's, so every device viewing the console sees
  the same turn.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: `PUT /api/session` SHALL accept `chart_rotation_deg`: a number, stored
  normalised to [0, 360) with −0 stored as 0, or `null` to clear. Anything else answers
  400 with a JSON `error` and changes nothing. Reads SHALL include it once set and omit
  it when unset. `session.json` keeps it across restarts, reads a file without it as
  unset, and checks and normalises a stored value as a `PUT` would.
- **FR-002**: The MCP tools `set_target` and `set_session` SHALL refuse
  `chart_rotation_deg` with their existing refusals and change nothing.
- **FR-003**: Setup SHALL show a "Chart rotation" section after Minimum altitude, with a
  degrees field, a "Save chart rotation" button and the hint "Turns the live chart and
  photos counter-clockwise, in degrees. Set it once to match how the camera sits in its
  holder." An empty field clears the setting; anything but a number is refused on the
  page.
- **FR-004**: The page's session SHALL keep `chart_rotation_deg` from every read.
- **FR-005**: The live chart SHALL draw its frame inside one group turned
  counter-clockwise by the rotation, in a `viewBox` sized to the turned frame's bounding
  box plus the margin, with its aspect set from that box. The paper's edge stays level
  and outside the group, sized to the box. Quarter turns are exact.
- **FR-006**: Every label without its own transform SHALL be turned back about its own
  middle, so it reads upright. Star and object labels SHALL be laid out in the turned
  view, so they keep off the marks and inside the frame as seen. Grid edge labels SHALL
  read along their edges, with a half turn about their middle added to any that would
  read upside down. The compass, the grid, the marks, the aim and up marks and the target
  mark turn with the frame.
- **FR-007**: The live photo under the chart and the aim-point photo SHALL be turned by
  the same angle inside a box of the turned frame's aspect. A tap on the turned
  aim-point photo SHALL set the aim at the frame pixel under it, to 0.5 px.
- **FR-008**: On a live page, the "Just off the frame" arrows SHALL be drawn turned by
  the rotation, and their labels SHALL keep the frame's own bearing.
- **FR-009**: Only live views turn. Uploads, samples, and every view with no rotation
  or 0, SHALL be drawn exactly as today. Taps, the aim point and every pixel sent to the
  server stay in frame coordinates.
- **FR-010**: `GLOSSARY.md` SHALL define Chart rotation, `crates/finder/README.md` SHALL
  document the field (US1), `DESIGN.md` SHALL record the Setup section (US1) and the
  turned views (US2), and `crates/finder/web/dist` SHALL be the build of its source.

### Key Entities

- **Chart rotation**: the session's angle, degrees counter-clockwise, by which every live
  view of a frame is turned.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario above has a committed test that asserts it. The
  `lint` and `test` gates pass.
- **SC-002**: No existing test, golden, `scripts/ci/` file, `.github/workflows/ci.yml`,
  `CODEOWNERS` entry or `ergane.yaml` is edited.
- **SC-003**: `crates/finder/web/dist` is the build of its source.
- **SC-004**: At the telescope, with the rotation set, Orion's belt stands upright on the
  live chart when it stands upright in the sky, its names read upright beside their
  stars, and pushing the tube up moves the stars down the page. Checked by the owner, not
  by a gate.

## Assumptions

- **Earlier features are in place.** 012, 013, 016, 017, 018, 019, 020 and 022 have
  landed. Every citation here was read at 0a33c19, whose code is c1ed295's. 020 and 022
  change `Chart.tsx`, `App.tsx`, `api.ts`, `labels.ts`, `index.css`, `DESIGN.md`,
  `GLOSSARY.md`, `README.md` and `session.rs` first, so their line numbers are re-read
  when both have landed (plan.md § Prerequisites).
- **Out of scope:**
  - the automatic horizon level (dropped by the owner on 2026-11-14);
  - mirrored images;
  - the push-to guidance's numbers, which 022 puts in the observer's own directions;
  - the survey view, which draws the sky north up;
  - the placeholder shown while the first frame solves;
  - `crates/finder/src/lib.rs`'s list of `PUT /api/session` fields, which already omits
    `node` and `camera_settings`;
  - `.impeccable/design.json`, which no gate reads; it is refreshed separately with
    `/impeccable document`.
- **The factory's gates and CI:** the gates are `lint` and `test`, copied in plan.md
  § Verification.

## Work Graph

```yaml
US1:
  depends_on: []
  implements: [FR-001, FR-002, FR-003, FR-004, FR-010]
US2:
  depends_on: []
  depends_on_merged: [US1]
  implements: [FR-005, FR-006, FR-007, FR-008, FR-009, FR-010]
```

The edge is a merge edge: both stories edit `DESIGN.md` and rebuild
`crates/finder/web/dist`, and US2 reads the field US1 adds.
