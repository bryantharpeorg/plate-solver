---
state: draft
depends_on_landed: ["016-mcp", "017-console-stop-confirm", "018-live-chart-steady", "020-live-chart-calm"]
# Drafted 2026-11-14 against `main` at c1ed295, at the owner's request ("even during the
# day, can we simulate the night sky for testing goto"), with the scope in a garage under
# cloud. The owner chose to include the daytime story ("Include daytime story") after
# being told it runs the Sun guard on a pretend Sun. It stays draft until the owner has
# reviewed it and given the approvals listed under "Owner approvals". Flipping it to
# `ready` is the act that dispatches it.
# Renumbered 2026-11-14 to 026 by the plate-solver operator on the owner's instruction
# (020 is live-chart-calm; 021 to 025 are taken), with the field-finding edits from the
# dark-site night of 13-14 November: power cycles, hand pushes and the real board's figures.
# Re-read against `main` at f8dab49. It stays draft.
---

# Feature Specification: A Drawn Sky on the Real Mount

**Feature Branch**: `026-mount-sky`

**Created**: 2026-11-14

**Status**: Draft

**Input**: GoTo, sync and tracking have only ever run against the simulator's mount. The
first powered GoTo on the real Sky-Watcher board needs stars, and the scope sits in a
garage under cloud. Add a console mode that drives the real mount through the scope node
as it always does, but replaces the stars in each frame with catalog stars drawn at the
pointing the mount's own encoders give. The console then solves, syncs, plans and
corrects exactly as it would under a real sky, while the motors, the serial link, the
watchdog and Stop are all real. A second story lets the mode run in daylight, on a night
clock, once the observer confirms the telescope is covered.

## Clarifications

### Session 2026-11-14 (review of `main` at c1ed295)

- Q: What does this trio read besides itself? → A: Nothing about the feature. `spec.md`
  holds every requirement and scenario, and `plan.md` holds every design decision, the
  interfaces, the shapes that existing tests pin and the alternatives rejected. A story
  that would change a decision escalates rather than re-deciding it.
- Q: What was found? → A: Six things, each checked against the code at c1ed295 and
  unchanged at f8dab49:
  1. A node frame carries centroids, not pixels: up to 200 `Centroid`s, brightest first,
     plus an optional preview JPEG (`crates/scope-link/proto/scope_node/v1/scope_node.proto:53`).
  2. Every frame carries the mount's encoder counts read just before and just after the
     exposure, `axis_start` and `axis_end`
     (`crates/scope-link/proto/scope_node/v1/scope_node.proto:68`). Syncs already take
     their counts from the frame, not from a separate read.
  3. Frames reach the console in one place, `active` in `crates/finder/src/link.rs:435`,
     which publishes the newest frame on a watch channel. Live solving and GoTo's
     settle wait both read that channel.
  4. The simulator's catalog renderer, `simulator::render::field_stars`
     (`crates/simulator/src/render.rs:27`), draws the stars for any camera rotation,
     and `simulator::sky::camera_rotation` (`crates/simulator/src/sky.rs:109`) gives that
     rotation from an altitude, azimuth, site and time. `finder` already depends on the
     `simulator` crate.
  5. The Sun guard refuses every motorised move, raw axis moves included, while the Sun
     is at or above −0.83° on the console clock (`crates/finder/src/goto.rs:332`,
     `crates/finder/src/goto.rs:711`, `crates/finder/src/goto.rs:811`,
     `crates/finder/src/session.rs:768`, `crates/finder/src/tracking.rs:382`). So in
     daylight no motor moves through the console, whatever the sky.
  6. The constitution allows a virtual clock only in tests and `finder --simulate`
     (`.specify/memory/constitution.md:47`), and `--simulate` never reaches a real node
     (013 FR-029). US2 needs an amendment, which the owner makes before flipping
     (plan.md § Prerequisites).
- Q: Where is the drawn sky pointing? → A: The first frame that carries encoder counts
  fixes it: those counts read as azimuth 180° and the tube's altitude as the observer
  measured it with a level and passed in `--tube-alt`. After that, each count is
  360° ÷ counts-per-revolution of travel on its axis. Matching the real altitude keeps
  the console's minimum altitude and horizon limits meaningful for the real tube.
- Q: What does the drawn sky not test? → A: Anything between the encoders and the sky:
  the tube slipping against the motors, backlash in the gears after the encoder, the
  base's real tilt, and the 2 to 3% by which the sky fell short of each commanded 1°
  move on the real board on 14 November (backlash or a scale error, not yet told
  apart). The drawn sky follows the encoders exactly, so it moves as far as the counts
  say and cannot show that shortfall. Only real stars test those.
- Q: Why its own state folder? → A: The night's syncs and zero points learned against a
  drawn sky are fiction. Keeping them in `mount-sky/session.json` means a real night
  never inherits them. The observer pairs the node and sets the site there once.

### Session 2026-11-14 (renumbering and the dark-site findings, `main` at f8dab49)

- Q: Why 026? → A: 020 is live-chart-calm, which has landed, and 021 to 025 are taken.
  The owner chose 026 and asked for the field findings below to be applied with it.
- Q: What did the real board show on 13-14 November? → A: Firmware 2.09, and 1,980,000
  counts per revolution on both axes, 5,500 a degree. Eight single-axis position moves
  of 1° or less worked on it: right axis, right direction, ending by themselves within
  26 counts of target. Long slews, rate moves and Stop on a moving axis were not tried,
  and a GoTo or Track under the drawn sky is how this mode rehearses them. The
  scenarios keep the `SimMount` fixture's 1,620,000 counts per revolution
  (`crates/simulator/src/mount.rs:12`), because the tests run against it.
- Q: What happens when the mount is power-cycled? → A: Spec 024-mount-link-recovery
  (a draft) makes the node initialise the board again after a power cycle, count each
  reset on `AxisState.mount_resets`, and treat counts that jump further than the top
  rate allows (`crates/scope-node/src/mount/skywatcher/protocol.rs:14`) as a reset; the
  console keeps the last count its mount poll saw. After a reset the counts are an
  unknown offset from the old ones, so the drawn sky's zero no longer means anything.
  When the session's count changes, this mode stops drawing for good (FR-014), and the
  banner says to restart the console with a fresh `--tube-alt`. 024 detects; this spec
  only reacts. No frame solves after that, so 024's refusal of GoTo, second sync and
  Track until a new sync stays on until the restart.
- Q: Why is 024 not in `depends_on_landed`? → A: It is not on `main` yet, and the
  validator refuses a dependency on a spec it cannot find. 024 having landed is plan.md
  Prerequisite 1 instead, and the flip waits for it; it goes into `depends_on_landed`
  once it is on `main`.
- Q: What about pushing the tube by hand? → A: With the board's own encoders off, as
  024 keeps them, a hand push moves the tube without changing the counts. The drawn sky
  stays where it was, no longer matches the tube, and the minimum altitude then guards
  an altitude the tube doesn't have. The console cannot see a push, so this spec doesn't
  try to detect one: the README and the banner forbid hand pushes in this mode (owner
  approval 5).

## Owner approvals

Flipping this spec to `ready` approves the following. Each is named in the FR that makes
it. No existing test, golden or CI file changes.

1. **A console mode that moves the real mount under a drawn sky**, `finder --mount-sky`
   (FR-001 to FR-008). Every guard stays on the real clock in US1. The drawn sky stops
   for good once the node reports a power cycle (FR-014).
2. **A night clock for daylight use** (US2, FR-009 to FR-011): with
   `--mount-sky-night UTC --covers-on`, the console's clock runs from that UTC at the
   real rate, and the Sun guard checks the Sun of that clock, not the real one. The
   console cannot see the covers: physical safety then rests on the observer's
   confirmation that the mirror's cover and the finder's cap are on.
3. **Constitution III amended to allow that clock**, in the flip PR, with the exact text
   in plan.md § Prerequisites, and `GLOSSARY.md`'s Sun guard entry recording it (FR-011).
4. **The page's "Drawn sky" banner** and its words (FR-012, FR-013), including the
   power-cycle line and the hand-push warning.
5. **No hand pushes in this mode.** The README and the banner forbid pushing the tube
   by hand while the console runs with `--mount-sky` (FR-008, FR-012). The console does
   not detect a push: with the board's encoders off nothing it reads changes, so this
   rests on the observer.

## User Scenarios & Testing *(mandatory)*

All Rust scenarios share one setup unless a scenario says otherwise:
- An in-process console built as `crates/finder/tests/mcp/support.rs:188` builds its
  `Rig`: a `NodeService` with a `SimCamera` and a recording `SimMount` behind the node's
  mount port, wrapped in a test `Mount` that passes every call to it and answers
  `resets()` (024's `Mount::resets`) with a count the test sets, 0 unless a scenario
  sets it, over a memory transport, with a session holding that node's pairing, the
  site 40° N 75° W and camera settings of 200 ms and gain 1. Instead of `with_link` it
  calls `with_mount_sky(transport, 30.0)`.
- The clock is a `VirtualClock` at `2026-01-16T02:00:00Z` ("the night"), when the Sun is
  below −15° at that site, unless a scenario names `2026-01-16T17:00:00Z` ("the day").
- The `SimMount`'s zero points are `[300, 70]`, so the node's own `SimCamera` points more
  than 40° from where the drawn sky does. A solve that agrees with the drawn sky
  therefore cannot come from the node's stars.
- "The truth of counts `[a, e]`" is, with `[a0, e0]` the session's
  `mount_sky.zero_counts` and `c` the counts per revolution: azimuth
  `180 + (a − a0) × 360 / c[0]` modulo 360 and apparent altitude
  `30 + (e − e0) × 360 / c[1]`, in degrees. Each test computes it itself.
- "Horizontal from a pointing" is the pointing's `ra_deg` and `dec_deg` turned into
  azimuth and geometric altitude by `simulator::sky::horizontal` at the pointing's
  `time`, then into apparent altitude by `finder::ephemeris::apparent_from_true`.

### User Story 1 - GoTo against a drawn sky on the real mount (Priority: P1)

As the observer with the scope in a garage under cloud, I start the console with
`--mount-sky` and the tube's altitude. The page solves frames, syncs and GoTos as it
would under the stars, and the real motors carry out every move, under every guard the
console already has.

**Why this priority**: Position moves of 1° are proven on the real board (firmware
2.09, 14 November); long slews, rate moves and Stop on a moving axis are not, and the
first powered GoTo is the riskiest step left. It needs no real sky to rehearse.

**Independent Test**: `cargo test -p finder --test mount_sky --locked`.

**Acceptance Scenarios**:

1. **Given** the setup at night, **When** the first frame with encoder counts has been
   solved, **Then** a committed test asserts that the session's `mount_sky.tube_alt_deg`
   is `30`, that `mount_sky.zero_counts` equals that frame's mid-exposure counts rounded
   to whole counts, and that horizontal from the session's live pointing is within 0.05°
   of azimuth 180° and altitude 30°.
2. **Given** a `MountSky` built directly over the test database and the setup's session
   once `GET /api/session` shows `mount.connected` true, so that the session holds the
   site, counts per revolution `[1620000, 1620000]` (the `SimMount` fixture's figure; the
   real board reports 1,980,000) and a reset count of 0, **When** a test passes it a frame
   with id 7, `start_s` 10, `end_s` 10.2, width 1456, height 1088, exposure 200 ms,
   gain 1, three centroids, a preview JPEG and counts `[1000, 2000]` at both ends, with
   offset 0, **Then** it asserts that the returned frame keeps the id, times, size,
   exposure, gain and both counts, has no preview, has between 20 and 200 centroids, and
   lists them in non-increasing brightness.
3. **Given** the setup at night with the aim set to the frame's centre, **When** the
   test starts a GoTo to M42 through `POST /api/goto` and it reads `done`, **Then** it
   asserts that the truth of the last frame's mid-exposure counts is within 0.05° of
   M42's apparent azimuth and altitude at that frame's console time, computed with
   `simulator::sky::horizontal` and `finder::ephemeris::apparent_from_true`.
4. **Given** the setup in the day, **When** the test asks for the same GoTo, **Then** it
   asserts that the answer is the GoTo refusal `Sun is up` and that the recording mount
   received no `GotoAxis` or `RateAxis` command.
5. **Given** the setup with the node built without a mount, **When** three frames have
   arrived, **Then** a test asserts that each frame the link publishes has no
   centroids, that the session has no live pointing, and that
   `mount_sky.zero_counts` is `null`.
6. **Given** the built `finder` binary, **When** a test runs it with `--mount-sky` and no
   `--tube-alt`, with `--tube-alt 95`, with `--tube-alt 30` alone, and with
   `--mount-sky --tube-alt 30 --simulate`, **Then** it asserts that each exits with a
   non-zero status before listening.
7. **Given** the built binary run with `--mount-sky --tube-alt 30`, listening on port 0
   of `127.0.0.1`, with `XDG_STATE_HOME` set to an empty temporary folder and no `--state`, **When** a test
   sends `PUT /api/session` with `{"min_alt_deg":20}`, **Then** it asserts that
   `GET /api/session` has `mount_sky` equal to `{"tube_alt_deg":30.0,"zero_counts":null}`,
   that `finder/mount-sky/session.json` exists under that folder, and that
   `finder/session.json` does not.
8. **Given** the setup at night after the first frame with encoder counts has been
   solved, **When** the test sets the mount's reset count to 1, waits until
   `GET /api/session` shows `mount.resets` 1, and lets three more frames arrive,
   **Then** a committed test asserts that the last frame the link published before the
   count was set had at least 20 centroids, that each of the three has no centroids and
   no preview, that `mount_sky.power_cycled` is `true` with `mount_sky.zero_counts`
   unchanged, and that `POST /api/goto` to M42 answers `409`; and that after the test
   sets the count back to 0 and three more frames arrive, those have no centroids either
   and `mount_sky.power_cycled` is still `true`.

---

### User Story 2 - The same test in daylight, on a night clock (Priority: P2)

As the observer in a closed garage in daylight, with the mirror's cover and the finder's
cap on, I start the console on a night clock. The Sun guard checks that night's Sun, so
the console moves the mount; it will not start in that mode until I say the covers
are on.

**Why this priority**: The scope is reachable in daylight; clear, dark nights are scarce.

**Independent Test**: `cargo test -p finder --test mount_sky_night --locked`.

**Acceptance Scenarios**:

1. **Given** the built binary, **When** a test runs it with
   `--mount-sky --tube-alt 30 --mount-sky-night 2026-01-16T02:00:00Z` and no
   `--covers-on`, **Then** it asserts a non-zero exit before listening and that standard
   error contains `--covers-on`.
2. **Given** the built binary, **When** a test runs it with
   `--mount-sky-night 2026-01-16T02:00:00Z --covers-on` and no `--mount-sky`, and with
   `--covers-on` alone, **Then** it asserts a non-zero exit before listening each time.
3. **Given** the built binary run with `--mount-sky --tube-alt 30
   --mount-sky-night 2026-01-16T02:00:00Z --covers-on`, listening on port 0 of
   `127.0.0.1`, with a temporary `--state`, **When** a test reads `GET /api/session` within 30 s of start,
   **Then** it asserts that `mount_sky.night_utc` is `2026-01-16T02:00:00Z`, that `time`
   lies between 1768528800 and 1768528860, and that standard error contains
   `pretend Sun`.
4. **Given** two consoles built as in US1's setup, one through
   `with_night_clock(1768528800.0, "2026-01-16T02:00:00Z")` before `with_mount_sky` and
   one without, **When** a test reads `GET /api/session` from each, **Then** it asserts
   that the first has `mount_sky.night_utc` equal to `2026-01-16T02:00:00Z` and the
   second has no `night_utc` key in `mount_sky`.

---

### User Story 3 - The page says the sky is drawn (Priority: P3)

As anyone looking at the page, I see at once that its stars are drawn from the mount,
not seen by the camera, that the tube must not be pushed by hand, after a power cycle
that the console must be restarted, and when the night clock is on, that the covers
must stay on.

**Why this priority**: A drawn sky that looks real is a trap at the telescope.

**Independent Test**: `npm test` in `crates/finder/web` runs
`src/components/MountSkyBanner.test.tsx`.

**Acceptance Scenarios**:

1. **Given** the page rendered with a session whose `mount_sky` is
   `{"tube_alt_deg":30,"zero_counts":[1,2]}`, **When** a committed test reads it,
   **Then** it asserts one element with role `status` whose text contains `Drawn sky`,
   `not seen by the camera` and `No hand pushes`, and none containing
   `Night clock` or `power-cycled`.
2. **Given** the same with `night_utc` `2026-01-16T02:00:00Z`, **Then** the test asserts
   that the status also contains `Night clock from 2026-01-16T02:00:00Z` and
   `Keep the mirror cover and the finder cap on`.
3. **Given** a session without `mount_sky`, **Then** the test asserts no element
   contains `Drawn sky`.
4. **Given** the session of US3-S1 with `power_cycled` `true`, **Then** the test asserts
   that the status also contains `The mount was power-cycled` and
   `restart the console with a fresh --tube-alt`.

---

### Edge Cases

- **The mount power-cycles.** 024's node initialises the board again and counts the
  reset, and the counts after it are an unknown offset from the old ones. When the
  session's reset count changes, the drawn sky stops for good: every later frame reaches
  the console with no centroids, nothing solves, 024's refusal of GoTo, second sync and
  Track stays on, and the banner says the mount was power-cycled and to restart the
  console with a fresh `--tube-alt` (FR-014, US1-S8). After the restart, reset the night
  (`POST /api/model/reset` with `{"what":"night"}`): its syncs were made against the old
  zero.
- **A hand push.** With the board's own encoders off, as 024 keeps them, a push moves
  the tube and not the counts, so the drawn sky silently stops matching the tube and the
  minimum altitude guards an altitude the tube no longer has. The console does not
  detect it: the README and the banner forbid hand pushes in this mode (owner approval
  5). After one, restart as after a power cycle.
- **Fast motion.** Frames taken while an axis moves are drawn at the mid-exposure counts
  without trails; syncs already skip them (`crates/finder/src/model.rs:246`).
- **The real altitude limit.** The console's minimum altitude applies to the drawn
  altitude, which matches the real tube only as well as `--tube-alt` and the board's
  counts per revolution (1,980,000 on the real board) are right. Check both with a level
  before a long move.
- **No mount on the node.** Frames arrive with no stars and nothing solves (US1-S5).
- **Stop, the watchdog and the serial link** are the real ones and change in no way.
- **`--simulate`** is unchanged and cannot be combined with `--mount-sky` (US1-S6).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: `finder` SHALL accept `--mount-sky`, which requires `--tube-alt DEG` with
  DEG from 0 to 90 inclusive, and SHALL refuse `--tube-alt` without `--mount-sky` and
  `--mount-sky` with `--simulate`.
- **FR-002**: Without `--state`, `--mount-sky` SHALL keep its session in `mount-sky/`
  under the default state folder.
- **FR-003**: Under `--mount-sky`, the console SHALL replace each frame's centroids with
  the catalog stars at the pointing its mid-exposure counts give, at its mid-exposure
  console time, brightest first and at most 200, and SHALL drop its preview, before
  anything else in the console reads the frame. Every other field SHALL be kept.
- **FR-004**: The pointing SHALL be the truth of counts as defined in § User Scenarios,
  fixed by the first frame with both counts, a site, counts per revolution, a reset count
  the session has seen and a clock offset.
- **FR-005**: A frame lacking any of those SHALL reach the console with no centroids.
- **FR-006**: `GET /api/session` SHALL report `mount_sky` with `tube_alt_deg` and
  `zero_counts`, and SHALL omit the key outside the mode.
- **FR-007**: The mode SHALL change no guard: the Sun guard, the minimum altitude, Stop
  and the watchdog behave as without it.
- **FR-008**: `crates/finder/README.md` SHALL document the flags of US1 and US2, SHALL
  forbid pushing the tube by hand in this mode, and SHALL say that after a power cycle
  the console must be restarted with a fresh `--tube-alt`.
- **FR-009**: `finder` SHALL accept `--mount-sky-night UTC`, which requires both
  `--mount-sky` and `--covers-on`; `--covers-on` SHALL require `--mount-sky-night`.
  With it, the console clock SHALL run from that UTC at the real rate.
- **FR-010**: With `--mount-sky-night`, the console SHALL print a warning naming the
  pretend Sun and the covers at start, and `mount_sky` SHALL carry `night_utc`.
- **FR-011**: `GLOSSARY.md`'s Sun guard entry SHALL record the night clock exception.
- **FR-012**: The page SHALL show the "Drawn sky" banner whenever the session has
  `mount_sky`, forbidding hand pushes, with the power-cycle line when it has
  `power_cycled` and the night clock line when it has `night_utc`.
- **FR-013**: `DESIGN.md` SHALL describe the banner, and `crates/finder/web/dist` SHALL
  be the build of its source.
- **FR-014**: Once the zero counts are fixed, the console SHALL stop drawing for good
  when the session's last-seen count of the node's mount resets (024's
  `AxisState.mount_resets`) differs from the count it had seen when it fixed them: that
  frame and every later one SHALL reach the console with no centroids and no preview,
  even if the count changes back, and `mount_sky` SHALL then carry `power_cycled: true`,
  a key it omits until then.

### Key Entities

- **Drawn sky**: the catalog stars the console draws in place of a frame's own, at the
  pointing the encoders give.
- **Zero counts**: the encoder counts that read as azimuth 180° and the tube's altitude.
- **Night clock**: the console clock under `--mount-sky-night`.
- **Power-cycled**: the drawn sky's stopped state once the node's reset count has
  changed; it lasts until the console restarts.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario has a committed test that asserts it, and the
  `lint` and `test` gates pass.
- **SC-002**: No existing test, golden, `scripts/ci/` file, `.github/` file, `CODEOWNERS`
  entry, `ergane.yaml` or `playwright.config.ts` is edited.
- **SC-003**: Against the drawn sky, a GoTo ends within 0.05° of its target in the
  encoders' own terms (US1-S3).

## Assumptions

- **Earlier features are in place.** 014 to 019 and 020-live-chart-calm have landed,
  and 024-mount-link-recovery lands before this spec is flipped (plan.md
  § Prerequisites). Every citation here was read at c1ed295 and re-read at f8dab49.
- **Out of scope:**
  - noise, trails, clouds or a daylight background in the drawn sky;
  - drawing the sky on the scope node, or any change under `crates/scope-node` or
    `crates/scope-link`;
  - telling MCP clients about the drawn sky beyond what `GET /api/session` carries;
  - copying pairing or site from the default session into `mount-sky/`;
  - detecting a hand push: this mode forbids them instead (Edge Cases);
  - any change to `--simulate`.
- **Outside the factory:** the first run against the real board in the garage, by the
  owner, with the covers on.
- **The factory's gates and CI:** the gates are `lint` and `test`, copied in plan.md
  § Verification. CI's other jobs run only in the merge queue, so no scenario's Then
  clause rests on them.

## Work Graph

```yaml
US1:
  depends_on: []
  implements: [FR-001, FR-002, FR-003, FR-004, FR-005, FR-006, FR-007, FR-008, FR-014]
US2:
  depends_on: []
  depends_on_merged: [US1]
  implements: [FR-009, FR-010, FR-011]
US3:
  depends_on: []
  depends_on_merged: [US1]
  implements: [FR-012, FR-013]
```

Both edges are merge edges: US2 extends US1's flags, `MountSky` and README section, and
US3 reads US1's `mount_sky` field, `power_cycled` included. US2 touches only Rust and
`GLOSSARY.md`; US3 touches only `crates/finder/web/` and `DESIGN.md`, so they may run
beside each other.
