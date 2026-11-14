---
state: draft
depends_on_landed: ["013-live-finder", "015-tracking", "016-mcp", "017-console-stop-confirm", "018-live-chart-steady", "019-night-legibility", "020-live-chart-calm"]
# Drafted 2026-11-13 against `main` at ce80f67, from the owner's report at the lake site
# that night, after a hand push to NGC 253: "the goto movement was not intuitive at all.
# it seems the chart plots based on the ecliptic and celestial coordinates but in my head
# with my alt azimuth telescope i just see left / right N° and up down N°".
# Refined 2026-11-14 against `main` at c1ed295: every story was built in throwaway
# worktrees (plan.md § Refinement Record), and the owner chose azimuth degrees for left
# and right and to build US3. It waits for 020 to land, and 021 waits for it. It stays
# draft until the owner flips it to `ready` on Tharpebox, which gives the approvals listed
# under "Owner approvals" and dispatches it.
---

# Feature Specification: Push-To in the Observer's Own Directions

**Feature Branch**: `022-altaz-guidance`

**Created**: 2026-11-13

**Status**: Draft

**Input**: The owner pushes a Dobsonian by hand and thinks of a move as "left or right so
many degrees, up or down so many degrees". On 2026-11-13 the page did not speak that way.
Its guidance line read "turn clockwise 3° · tilt down 1°", in whole degrees. Its chart is
a celestial one: an RA/Dec grid and a north-and-east compass, drawn the way the camera
sits, with nothing on it that says which way is up. Its reticle marks the frame's centre,
while the telescope was looking at the aim point, about 1.2° away. The numbers the console
computes are already altitude and azimuth. This feature makes the page print them in the
owner's words and to a tenth of a degree, mark the aim point on the chart, and show which
way is up and which way is right there.

## Clarifications

### Session 2026-11-13 (review of `main` at ce80f67)

- Q: What does this trio read besides itself? → A: Nothing about the feature. `spec.md`
  holds every requirement and scenario, and `plan.md` holds every design decision, the
  interfaces, the shapes that existing tests pin and the alternatives rejected. A story
  that would change a decision escalates rather than re-deciding it.
- Q: Are the guidance numbers wrong? → A: No. They are already alt-az. `turn_tilt`
  (`crates/finder/src/pushto.rs:162`) takes the azimuth difference, wrapped to ±180°, and
  the altitude difference, between the pointing's altitude and azimuth and the target's
  (`crates/finder/src/pushto.rs:131` to `crates/finder/src/pushto.rs:136`). A live
  pointing is taken at the aim point (`crates/finder/src/session.rs:1139` to
  `crates/finder/src/session.rs:1142`). Only the printing and the picture change here.
- Q: What was found? → A: Seven things, each checked against the code at ce80f67:
  1. The page prints whole degrees, worded as turn and tilt
     (`crates/finder/web/src/components/GuidanceBlock.tsx:78` and
     `crates/finder/web/src/components/GuidanceBlock.tsx:79`). A reading of 0° covers
     ±30′ on each axis, so both axes can read 0° with the target up to 42′ from the aim
     point. The 30 mm eyepiece shows a circle 41′ in radius and the 14 mm one about 23′.
  2. A whole degree also hides drift. With the telescope standing still on 2026-11-13,
     the turn to NGC 253 grew from 2.0° to 5.0° in about thirteen minutes, a quarter of
     a degree a minute.
  3. The glossary chose these words on purpose and lists "left, right" under Avoid for
     Turn (`GLOSSARY.md:37` to `GLOSSARY.md:39`). The voice contract already takes
     "right" to mean clockwise, "as you face where the telescope points"
     (`docs/voice/02-tool-contracts.md:65` to `docs/voice/02-tool-contracts.md:68`).
  4. The chart is drawn in the frame's own pixels
     (`crates/finder/web/src/components/Chart.tsx:183`). Its grid is the RA/Dec graticule
     (`crates/finder/src/solve.rs:465`, drawn at
     `crates/finder/web/src/components/Chart.tsx:206`), and its compass points to
     celestial north and east (`crates/finder/web/src/components/Chart.tsx:521` and
     `crates/finder/web/src/components/Chart.tsx:533`). Nothing on it is tied to the
     horizon.
  5. The reticle is at the frame's centre
     (`crates/finder/web/src/components/Chart.tsx:521`, drawn at
     `crates/finder/web/src/components/Chart.tsx:544`). The session reads carry the aim
     point (`crates/finder/src/session.rs:550`, `crates/finder/web/src/api.ts:248`), and
     `Chart.tsx` never reads it. That night the aim point was (588.7, 380.0) in a
     1296 × 972 frame: 121 px from the centre, 1.2° at 36″ per pixel.
  6. The camera sat with the sky's up 78° clockwise of the frame's up. Three measurements
     agree: Orion's belt in the frame (79°, spec 021); the solve's `north_deg` of 101.6°
     less the parallactic angle of 23.4° at NGC 253 (78.2°); and the pointing model's own
     fitted camera angle, −78.29° (`crates/finder/src/model.rs:75`). With the hotfix chip
     at 90° the page was still 12° off level, and celestial north lay another 23° from
     the sky's up.
  7. The console already works out the frame's roll against the zenith for the pointing
     model, by turning the aim pixel and a pixel just above it into altitude and azimuth
     (`crates/finder/src/model.rs:278` to `crates/finder/src/model.rs:288`). The same two
     rays give where up is in the frame, with no new sky maths.
- Q: What already exists? → A: Only a local hotfix built that night in the session
  worktree, uncommitted and not on `main`: a line `right 2.7° · down 1.4°` above the old
  one in the guidance block, which it sets in the quiet style, and a ring, a dot and the
  word "aim" at the aim point on a live chart. It is FR-001, FR-004, FR-005 and FR-006
  here, without tests. The probe found its `push` function, push line, quiet old line,
  `AimMark` and CSS identical to this plan. It stays off `main`; this spec replaces it.
- Q: Can the old line go? → A: Not by a builder. Five assertions pin its words:
  `crates/finder/web/src/components/GuidanceBlock.test.tsx:31`,
  `crates/finder/web/src/components/GuidanceBlock.test.tsx:41`,
  `crates/finder/web/src/App.live.test.tsx:151`,
  `crates/finder/web/src/App.live.test.tsx:162` and
  `crates/finder/web/e2e/simulate.spec.ts:191`. So this spec keeps the line, set quiet
  under the new one, and no existing test changes. Removing it is the owner's own commit
  (Owner approvals, 4).
- Q: Which degrees does left and right count? → A: The mount's: degrees of azimuth, the
  angle the base turns. That is `turn_deg` as it is today. On the sky the same move is
  shorter by the cosine of the altitude, so the two agree near the horizon and part near
  the zenith:

  | Altitude | A target 1.0° to the side on the sky reads |
  |---|---|
  | 27.5° (NGC 253 that night) | 1.1° |
  | 45° | 1.4° |
  | 60° | 2.0° |
  | 75° | 3.9° |
  | 85° | 11.5° |

  The page therefore also prints the distance on the sky from the aim point to the
  target ("2.8° away"), which means the same at every altitude and can be held against
  the eyepiece's field (FR-002, FR-003). The other choice, and why it is not
  recommended, is under Owner approvals, 2.
- Q: Can `up_deg` go on the session's `Pointing`, as 021's plan has it? → A: No. A unit
  test builds that struct field by field (`crates/finder/src/pushto.rs:421` to
  `crates/finder/src/pushto.rs:429`), so a new field there stops an existing test
  compiling, and no builder may edit it. `up_deg` goes on the view's wrapper,
  `LatestPointing` (`crates/finder/src/session.rs:424`), which only
  `crates/finder/src/session.rs:614` builds.
- Q: Why must the live event carry `up_deg` too? → A: The page rebuilds a live pointing
  from the event, field by field (`crates/finder/web/src/useLive.ts:46` to
  `crates/finder/web/src/useLive.ts:55`), and uses it in place of the session's whenever
  the stream is ahead of the poll. A field only on the session's pointing would come and
  go from frame to frame.
- Q: How does this sit with 021? → A: 021 turns the chart by a fixed angle typed into
  Setup; on 2026-11-14 the owner dropped its second story, which would have turned the
  chart by `up_deg` on every solve. This spec does not turn the chart. It adds `up_deg`
  (FR-007) and draws up and right at the aim point whatever way the chart is turned
  (FR-009). 021 lands after this spec (it names `022-altaz-guidance` in
  `depends_on_landed`), so both marks are on `main` when 021 turns them, and their labels
  carry no `transform`, so 021's upright-label rule applies to them.
- Q: Why no horizon grid? → A: It was considered and left out. The legend has five chips
  and a test wants exactly those five
  (`crates/finder/web/src/components/Chart.test.tsx:23` to
  `crates/finder/web/src/components/Chart.test.tsx:29`), so a second grid cannot have a
  chip of its own. Drawing altitude and azimuth lines also needs the reverse of
  `sky::alt_az` (`crates/finder/src/sky.rs:229`), which `sky.rs` does not have. And
  where the choice of grid is kept is a decision of its own. See Owner approvals, 8.

### Session 2026-11-14 (refinement probe at c1ed295, and the owner's answers)

- Q: Which unit do left and right count? → A: Degrees of azimuth, with "away" on the sky
  beside them. The owner chose this on 2026-11-14 (Owner approvals, 2).
- Q: Is US3 built? → A: Yes, the owner chose to build it on 2026-11-14. 021 no longer
  levels the chart by itself (the owner dropped its US2 the same day), so this spec's
  `up_deg` is drawn as the up mark and is not used to turn the chart.
- Q: Where do the page scenarios put the aim point? → A: Inside the fixture. `ORION2` is
  507 × 507, so the night's aim pixel (588.7, 380), from a 1296 × 972 frame, lies outside
  it and its mark would be clipped away. The scenarios use (320.5, 190).
- Q: Can the up mark's labels take taps? → A: No. Outside the mark's group they would
  catch taps meant for the stars beneath them and clear the pick, so both carry
  `pointer-events-none` (FR-009).
- Q: Is a bearing of exactly 0° drawn? → A: Yes. A truthiness test on `up_deg` would drop
  the mark at 0° and leave a stray `0` in the SVG; the condition is
  `up_deg !== undefined` (US3-S9).
- Q: Does the server's `up_deg` hold up? → A: Yes. The plan's code compiled as written,
  against the real signatures of `Frame::project`, `Frame::unproject`, `sky::alt_az` and
  `UnitVector`. On `orion2.jpg` it is within 0.145° of the solve's north less the
  parallactic angle at the centre (US3-S1) and 0.129° at an aim point (US3-S3); the worst
  anywhere in that frame was 0.248° at a corner, because the projection does not keep
  angles far off axis. On the simulator, whose camera top follows altitude, it reads
  359.99997° at the centre and leans about 6° either side, as expected. The sign on the
  real camera (about 78° for the night of 2026-11-13) is the owner's to see.
- Q: Which server scenarios could pass before the change? → A: US1-S8, US1-S9 and US3-S2
  as drafted asserted only that a field was absent. Each now also asserts the field
  appears once the condition clears, so it fails first.
- Q: Do the two printed numbers round alike? → A: They must. `toFixed(1)` prints 0.35 as
  "0.3" while the push line's rounding gives "0.4"; both use the push line's rounding
  (FR-003, US1-S4).
- Q: Where does e2e set an aim point? → A: Not in the simulator. `crates/finder/web/e2e/goto.spec.ts:148`
  sets one and `crates/finder/web/e2e/simulate.spec.ts:312` sets another by a tap, so
  the aim mark appears on live charts in the GoTo and tracking projects. The marks use
  only `--color-on-photo` and `--color-photo`, which are red and black at night.

## Owner approvals

Flipping this spec to `ready` approves the following. Each is named in the FR that makes
it. No existing test, golden or CI file changes.

1. **The page says left and right.** The guidance block leads with
   `right 2.7° · down 1.4°` (FR-001), and the glossary's entry for Turn stops listing
   "left, right" under Avoid (FR-011). Turn and tilt stay the names of the two motions.
2. **Left and right count azimuth degrees**, the angle the base turns, with the distance
   on the sky printed beside them (FR-001 to FR-003). Chosen by the owner on 2026-11-14.
   It is exact for the mount at every altitude, the zenith included, and "away" answers
   "is it in the eyepiece yet?" in the sky's own degrees. High in the sky the turn reads
   more than the distance on the sky (twice at 60°).
3. **A new optional field on guidance, `separation_deg`** (FR-002). MCP clients see it
   too, in `get_guidance`. No tool argument changes.
4. **The old line stays, quiet, under the new one** (FR-004). Taking it out is one commit
   of yours, or mine at your word, after US1 lands. It deletes the line and changes the
   five assertions named above, the sentence in `DESIGN.md` § Guidance block, and
   `PRODUCT.md:71`. No builder can make it.
5. **The aim mark**: a ring, a dot and the word "aim" in Frame Ink, on a live chart, when
   an aim point is set (FR-005).
6. **A new optional field, `up_deg`**, on the session's pointing and on the live event
   (FR-007, FR-008).
7. **US3 is built**, the up-and-right mark. Chosen by the owner on 2026-11-14.
8. **No horizon grid here.** It would be its own spec, after 021 and this one have landed
   and the owner has seen a turned chart with up and right marked.
9. **A new step in the type hierarchy**, "Push" at 1.5rem and weight 600, the largest
   text in the guidance block (FR-011).

## User Scenarios & Testing *(mandatory)*

Server scenarios run in `cargo test --workspace --all-targets --all-features --locked`.
They use a fixed console clock, the site 40° N, 75° W and the sample `orion2.jpg`, as
`crates/finder/tests/pushto_api.rs:58` does. Page scenarios run in `npm test` in
`crates/finder/web`, with `@testing-library/react`, `SESSION` from
`crates/finder/web/src/test/targets.ts` (turn 33.6°, tilt 12.4°, pointing `seq` 7) and
the fixture `ORION2` from `crates/finder/web/src/test/fixtures.ts`. jsdom has no layout,
so page tests assert text, attributes and directions computed from attributes, never
drawn sizes.

### User Story 1 - The push in the observer's own directions (Priority: P1)

As the observer at the tube, I read which way to push and how far in my own words, to a
tenth of a degree, with the distance left on the sky under it.

**Why this priority**: It is what the owner asked for, and it is the line he steers by.

**Independent Test**: `cargo test --workspace --all-targets --all-features --locked`
runs `crates/finder/tests/pushto_separation.rs`; `npm test` runs
`crates/finder/web/src/components/GuidancePush.test.tsx`.

**Acceptance Scenarios**:

1. **Given** `GuidanceBlock` rendered with `SESSION`, **When** a committed test reads the
   element with `data-push`, **Then** it asserts its text is `right 33.6° · up 12.4°`.
2. **Given** the same with `turn_deg` −2.74 and `tilt_deg` −1.37, **When** a committed
   test reads it, **Then** it asserts `left 2.7° · down 1.4°`.
3. **Given** the same with `turn_deg` −0.04 and `tilt_deg` 0.04, **When** a committed
   test reads it, **Then** it asserts `right 0.0° · up 0.0°`.
4. **Given** the same with `separation_deg` 2.83, **When** a committed test reads the
   element with `data-away`, **Then** it asserts its text is `2.8° away`; with
   `separation_deg` 0.35, that it is `0.4° away`; and with no `separation_deg`, that
   there is no such element.
5. **Given** `GuidanceBlock` rendered with `SESSION`, **When** a committed test reads the
   block, **Then** it asserts the element with `data-push` comes before the element
   whose text is `turn clockwise 34° · tilt up 12°`, and that the latter has the class
   `quiet`.
6. **Given** `GuidanceBlock` rendered with no pointing, below the horizon, too low,
   inside the Sun guard and without a site (the five sessions
   `crates/finder/web/src/components/GuidanceBlock.test.tsx` builds without directions),
   and `SESSION` rendered with the `stale` prop, each with `separation_deg` 2.83 added
   to its guidance, **When** a committed test reads each, **Then** it asserts none has
   an element with `data-push` or `data-away`; and that `SESSION` with the same guidance
   and no `stale` prop has both.
7. **Given** a console with a site, `orion2.jpg` solved and the target M78, **When** a
   committed test reads `/api/session`, **Then** it asserts `guidance.separation_deg` is
   within 1e-6° of the angle between the read's own `pointing.ra_deg` and
   `pointing.dec_deg` and its own `target.ra_deg` and `target.dec_deg`, which the test
   computes by the haversine formula.
8. **Given** the console of scenario 7, **When** a committed test changes the target to
   M8, which is below the horizon at that time, **Then** it asserts the guidance's class
   is `below_horizon` and it has no `separation_deg`; **and when** it changes the target
   back to M78, it asserts `separation_deg` is present again. The same for a session
   with no pointing, where it asserts the class is `pointable` and there is no
   `separation_deg`.
9. **Given** a session with a site and the target M78, and a live pointing 5.0 s old,
   built as `crates/finder/tests/freshness_api.rs:51` builds one, **When** a committed
   test reads the guidance, **Then** it asserts `separation_deg` is within 1e-6° of the
   haversine angle between the read's pointing and target; **and when** a live pointing
   5.1 s old replaces it (`crates/finder/tests/freshness_api.rs:57`), it asserts the
   guidance has `stale` and neither `turn_deg` nor `separation_deg`.

---

### User Story 2 - The chart marks where the telescope looks (Priority: P1)

As the observer, I see on the live chart the spot my eyepiece is centred on, so the
picture and the push agree: the target's mark comes to that spot, not to the frame's
centre.

**Why this priority**: Without it the picture is 1.2° wrong on the owner's telescope.

**Independent Test**: `npm test` runs
`crates/finder/web/src/components/ChartAim.test.tsx`.

**Acceptance Scenarios**:

1. **Given** `Chart` rendered with `ORION2`, `live` true and a session whose `aim` is
   (320.5, 190), **When** a committed test reads the SVG, **Then** it asserts one element
   with `data-aim` whose `transform` is `translate(320.5 190)`, holding a `title` with
   the text `Aim point: where the telescope looks`, two `circle` elements and a `text`
   reading `aim` that has no `transform` attribute and whose `text-anchor` is `end`
   (the aim lies right of the frame's middle, so the word sits toward it); and that the
   element comes after the element with the class `furniture` in document order.
2. **Given** the same without `live`, and the same with `live` and no `aim`, **When** a
   committed test reads each SVG, **Then** it asserts neither has an element with
   `data-aim`.
3. **Given** the first rendering, **When** a committed test reads the legend, **Then** it
   asserts its buttons are the five that
   `crates/finder/web/src/components/Chart.test.tsx:23` lists, with the same counts, and
   that the element with the class `furniture` is the same, attribute for attribute, as
   in a rendering without `aim`.
4. **Given** the first rendering with an unsolved report (`ORION2` with `pointing` null),
   **When** a committed test reads the SVG, **Then** it asserts the element with
   `data-aim` is still there.

---

### User Story 3 - The chart shows which way is up (Priority: P2)

As the observer, I see at the aim point which way on the picture is up and which way is
right, so I can read a push off the chart however the camera sits in its holder.

**Why this priority**: The numbers lead. This makes the picture agree with them, and
lets the owner check the camera's angle against the sky.

**Independent Test**: `cargo test --workspace --all-targets --all-features --locked`
runs `crates/finder/tests/pointing_up.rs`; `npm test` runs
`crates/finder/web/src/components/ChartUp.test.tsx` and
`crates/finder/web/src/useLiveUp.test.ts`.

**Acceptance Scenarios**:

1. **Given** a console with a site and `orion2.jpg` solved, **When** a committed test
   reads `/api/session`, **Then** it asserts `pointing.up_deg` is within 0.3° of the
   solve report's `pointing.north_deg` less the parallactic angle, modulo 360°. The test
   computes the parallactic angle as `atan2(sin H, tan φ cos δ − sin δ cos H)` from the
   read's own pointing, precessed to the date with `finder::sky::precess_from_j2000`,
   and the hour angle from `finder::sky::gmst_deg`. The bound is 0.3° because the
   report's north is J2000's, and precession has turned north by up to about 0.15°
   since.
2. **Given** the same console without a site, **When** a committed test reads
   `/api/session`, **Then** it asserts the pointing has no `up_deg`; **and when** it then
   sets the site, it asserts the next read's pointing has `up_deg`.
3. **Given** a session with a site and an aim point of (200, 150), and a live pointing
   recorded on a hand-built frame as `crates/finder/tests/freshness_api.rs:51` records
   one, **When** a committed test reads the session, **Then** it asserts `up_deg` is
   within 0.3° of `Frame::north_at` at the aim point's direction less the parallactic
   angle there, modulo 360°.
4. **Given** a console with a simulated scope node and a site, **When** a committed test
   takes a solved live event as `crates/finder/tests/live/support.rs:118` does and then
   reads the session, **Then** it asserts the event's `up_deg` equals the session
   pointing's `up_deg` when their sequence numbers match.
5. **Given** `useLive` fed a live event with `up_deg` 78 and a `pointing_seq` ahead of
   the session's, **When** a committed test reads the hook's session, **Then** it asserts
   `pointing.up_deg` is 78; and with an event that has no `up_deg`, it asserts the
   pointing has no `up_deg` key.
6. **Given** `Chart` rendered with `ORION2` and `pointing_seq` 7, `live` true, and
   `SESSION` with an `aim` of (320.5, 190) and `up_deg` 78 on its pointing, **When** a
   committed test reads the SVG, **Then** it asserts one element with `data-up` whose
   `transform` is `translate(320.5 190) rotate(78)`, and two `text` elements with
   `data-up-label`, reading `up` and `right`, outside that element, neither with a
   `transform` attribute and both with the class `pointer-events-none`. It asserts the
   bearing of `up` from (320.5, 190), `atan2(x − 320.5, 190 − y)` on its `x` and `y`
   attributes, is 78° to 0.01°, and the bearing of `right` is 168° to 0.01°.
7. **Given** the same with no `aim`, and the same with the `aim` but without `live`,
   **When** a committed test reads each SVG, **Then** it asserts the `transform` starts
   `translate(${W / 2} ${H / 2})`, with `ORION2`'s width and height.
8. **Given** the same with no `up_deg`, and the same with `pointing_seq` 8 on the
   report, **When** a committed test reads each SVG, **Then** it asserts neither has an
   element with `data-up` or `data-up-label`.
9. **Given** `Chart` rendered as in scenario 6 but with no target set and `up_deg` 0,
   **When** a committed test reads the SVG, **Then** it asserts one element with
   `data-up` whose `transform` ends `rotate(0)`, and that the SVG has no text node
   reading `0` outside an element.

---

### Edge Cases

- **Near the zenith.** A small distance on the sky can need a large turn, up to half a
  turn of the base. The push line says so truthfully, and "away" stays small (FR-001,
  FR-003). `turn_tilt` maps exactly −180° to +180°
  (`crates/finder/src/pushto.rs:161`), so exactly half a turn reads "right 180.0°"; a
  turn within 0.05° of it reads "180.0°" with the word its sign gives.
- **A reading at zero.** A magnitude that rounds to 0.0 prints "right" or "up", so the
  word does not flicker while the telescope sits on the target (FR-001).
- **No aim point set.** The eyepiece is taken to be at the frame's centre
  (`crates/finder/src/session.rs:466`). The aim mark is absent, and the up mark sits on
  the centre reticle (FR-005, FR-009).
- **No site.** There is no turn or tilt, as today, and no `up_deg` (FR-007).
- **Uploads and samples.** They are other cameras' frames, so they get no aim mark. They
  do get `up_deg` when a site is set, because the angle is as true of an uploaded
  pointing, and the up mark is drawn on them at the frame's centre.
- **A stale live pointing.** The push line and "away" go with the old line, as today.
  The marks stay on the held chart.
- **A turned chart (021).** Both marks are drawn in the frame's coordinates beside the
  furniture, so they turn with the frame, and their labels carry no `transform`, so
  021's rule keeps them upright.
- **Night mode and reduced motion.** Existing tokens only, so every pixel stays red or
  black. Nothing is animated.
- **The phone.** The longest push line, `right 180.0° · down 90.0°`, measures about
  269 px at 1.5rem in Archivo at weight 600, inside the 328 px a 360 px screen leaves.
  The pointable block grows by about 66 px, so on a 390 × 844 phone the chart's legend
  may drop below the first screen; no e2e test measures that state.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Where the guidance block prints directions today, it SHALL first print the
  push line, in an element with `data-push`: `right` when `turn_deg` is positive and
  `left` when negative, then its magnitude to one decimal and `°`, then ` · `, then `up`
  when `tilt_deg` is positive and `down` when negative, then its magnitude to one
  decimal and `°`. A magnitude that rounds to 0.0 prints `right` or `up`. It is set at
  1.5rem, weight 600, in ink.
- **FR-002**: Guidance SHALL carry `separation_deg`, the angle on the sky between the
  pointing's J2000 position and the target's, in degrees, exactly when it carries
  `turn_deg` and `tilt_deg`. A stale live pointing clears all three.
- **FR-003**: Under the push line, the block SHALL print `separation_deg` to one decimal,
  rounded as the push line rounds, as `2.8° away`, in the quiet style, in an element
  with `data-away`, when guidance carries it.
- **FR-004**: The line `turn clockwise 34° · tilt up 12°` SHALL stay, worded exactly as
  today, under those two and in the quiet style.
- **FR-005**: On a live chart, when the session has an aim point, the chart SHALL draw
  the aim mark at it in the frame's coordinates, after the furniture: a group with
  `data-aim`, a title "Aim point: where the telescope looks", a ring of radius 9 chart
  units, a centre dot, and the upright word "aim" on the side toward the frame's middle,
  all in Frame Ink, with the label cased like a star name. At night the ring is 2.2px
  and non-scaling, like a matched star's.
- **FR-006**: The aim mark SHALL NOT change the centre reticle, the compass, the scale
  bar, the target mark or the legend. It is absent on a chart that is not live and when
  no aim point is set.
- **FR-007**: A session read's pointing SHALL carry `up_deg` when the session has a
  site: where the zenith points in the frame at the pointing's own position, in degrees
  clockwise from the frame's up, in [0, 360), at the pointing's time. Without a site it
  is omitted.
- **FR-008**: A solved live event SHALL carry the same `up_deg` as the session pointing
  it made, and the page's live pointing SHALL keep it.
- **FR-009**: When the chart shows the session's latest pointing and that pointing has
  `up_deg`, the chart SHALL draw the up mark at the aim point on a live chart that has
  one, and otherwise at the frame's centre: a group with `data-up`, moved to that point
  and rotated by `up_deg`, holding an arm toward up with an arrowhead, from 13 to 34
  chart units out, and an arm toward the right, from 13 to 22 units out; and, outside
  the group, the upright words `up` and `right` with `data-up-label`, 9 units beyond
  the arms' ends. It is in Frame Ink, with the furniture's stroke widths. Neither the
  group nor its labels take pointer events. It needs no target, only `up_deg`, which
  may be 0.
- **FR-010**: The up mark SHALL be absent when the pointing has no `up_deg`, and when the
  chart's report is not the session's latest pointing.
- **FR-011**: `GLOSSARY.md` SHALL say that the page prints a clockwise turn as "right"
  and an anticlockwise one as "left", as the observer faces where the telescope points,
  and SHALL no longer list "left, right" under Avoid for Turn. `DESIGN.md` SHALL record
  the push line (in § Guidance block and as a "Push" step in § Typography › Hierarchy),
  the aim mark and the up mark. `crates/finder/README.md` SHALL list `up_deg` among the
  live event's fields. `crates/finder/web/dist` SHALL be the build of its source.

### Key Entities

- **Push line**: the turn and tilt printed as left or right and up or down, to a tenth
  of a degree.
- **Separation**: the angle on the sky from the aim point to the target.
- **Aim mark**: the chart's mark at the aim point.
- **Up**: where the zenith points in the frame at the aim point, clockwise from the
  frame's up. Right is a quarter turn clockwise from it.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario above has a committed test that asserts it. The
  `lint` and `test` gates pass.
- **SC-002**: No existing test, golden, `scripts/ci/` file, `.github/workflows/ci.yml`,
  `CODEOWNERS` entry or `ergane.yaml` is edited.
- **SC-003**: `crates/finder/web/dist` is the build of its source.
- **SC-004**: At the telescope, the owner pushes to a target by the push line alone and
  finds it in the 30 mm eyepiece when "away" reads 0.5° or less; and the target's mark
  on the chart sits on the aim mark when it is centred in the eyepiece. Checked by the
  owner, not by a gate.

## Assumptions

- **Earlier features are in place.** 013, 015, 016, 017, 018, 019 and 020 have landed. Every
  citation here was read at c1ed295; 020 changes `Chart.tsx`, `App.tsx`, `api.ts` and
  `DESIGN.md` first, so their line numbers are re-read when 020 has landed.
- **Out of scope:**
  - turning or levelling the chart (spec 021);
  - a horizon grid (Owner approvals, 8);
  - the edge arrow for a target off the frame, which stays on the ray from the frame's
    centre with a whole-degree distance from it
    (`crates/finder/web/src/components/TargetMark.tsx:20` to
    `crates/finder/web/src/components/TargetMark.tsx:27`,
    `crates/finder/src/pushto.rs:188`). Existing tests pin that placement;
  - the eyepiece's field drawn as a ring, which needs the eyepieces recorded somewhere;
  - the MCP tools' arguments, and the voice contract's words;
  - the words Turn and Tilt in GoTo, Track and the mount's own messages.
  - `.impeccable/design.json`, which no gate reads; it is refreshed separately with
    `/impeccable document`.
- **The factory's gates and CI:** the gates are `lint` and `test`, copied in plan.md
  § Verification.

## Work Graph

```yaml
US1:
  depends_on: []
  implements: [FR-001, FR-002, FR-003, FR-004, FR-011]
US2:
  depends_on: []
  depends_on_merged: [US1]
  implements: [FR-005, FR-006]
US3:
  depends_on: []
  depends_on_merged: [US1, US2]
  implements: [FR-007, FR-008, FR-009, FR-010]
```

Every edge is a merge edge: all three stories edit `DESIGN.md` and rebuild
`crates/finder/web/dist`, US2 and US3 both edit `Chart.tsx` and `index.css`, and US1 and
US3 both edit `pushto.rs` and `api.ts`.
