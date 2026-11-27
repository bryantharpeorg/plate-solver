---
state: draft
depends_on_landed: ["014-goto", "015-tracking", "016-mcp", "017-console-stop-confirm", "019-night-legibility", "021-chart-orientation", "022-altaz-guidance", "024-mount-link-recovery", "032-imaging-solve-and-aim-point", "038-track-follows-a-push", "040-stack-waits-for-steady", "050-tools-page-aligns-finder-camera", "051-tools-page-focuses-either-camera"]
# Drafted 2026-11-27 against `main` at f6d4f5fd, at the owner's ask of 2026-11-27 ("Your
# rec. Proceed", to the seat's ranked list after a field night), from a probe of that day
# on the branch `probe/055` cut from f6d4f5fd (plan.md § What was run and what was read).
# Every `path:line` here was read at f6d4f5fd. None of the nine owner decisions is
# answered yet; the scenarios are written to each recommendation. 014 is named for the
# direct move's guards and approach; 015 and 038 for Track and its push check; 016 for
# direct axis moves, MCP and `axis_move`; 021 for the chart rotation and the live
# pointing's up; 032 for the imaging solve; 019 for the night rule; 017, 022, 024, 040,
# 050 and 051 because each last changed a file this spec writes (plan.md § Refinement
# Record). 052, 053 and 054 are being drafted beside this spec; it is re-anchored after
# they land, before any flip.
---

# Feature Specification: The Nudge Arrows Follow the Frame

**Feature Branch**: `055-nudge-arrows-follow-the-frame`

**Created**: 2026-11-27

**Status**: Draft

**Input**: On a field night the owner had a target slightly off centre after a GoTo and
Track (the GoTo's error was 35″, yet the target was not centred in the finder camera's
view) and wanted to nudge it on from the console: plain **up, down, left and right**
buttons, **not** equatorial ones, since the mount is alt-az. Later that night, moving a
planet into the imaging camera's frame by hand over MCP (`move_axis`, a signed rate for
a set time; +0.1°/s for 2 s gave +0.19°, positive being higher azimuth or altitude), the
wish became buttons that **read the frame's own up and right**, so that a press moves
the picture the way the arrow points. On the same night every stack of M42 sat 6.2′ to
6.6′ off the imaging camera's centre. Today the page has no button that moves the
telescope a little, and MCP's moves are by axis and rate.

## Clarifications

### Session 2026-11-27 (the probe on `main` at f6d4f5fd)

- Q: What does this trio read besides itself? → A: Nothing about the feature. `spec.md`
  holds every requirement and scenario, and `plan.md` every design decision, the
  interfaces, the shapes that existing tests pin and the alternatives rejected. The
  probe's branch and logs are not in the repository; what they showed is written here.
- Q: What moves the mount by hand today? → A: Only MCP's two direct axis moves,
  `goto_axis_position` and `move_axis` (`crates/finder/src/mcp/tools.rs:430`,
  `crates/finder/src/mcp/tools.rs:459`), through `mcp::guard::start`
  (`crates/finder/src/mcp/guard.rs:21`), `api::start_motion`
  (`crates/finder/src/api.rs:797`), which refuses them over MCP with `Track is on`
  (`crates/finder/src/api.rs:817`) and refuses any move while a direct one runs with
  `the mount is already moving` (`crates/finder/src/api.rs:840`), and
  `GoTo::check_axis_move` (`crates/finder/src/goto.rs:772`), whose `Planner::direct`
  (`crates/finder/src/goto.rs:220`) asks for the night's sync and the guards of a GoTo.
  The page has none; the glossary says "Only MCP clients ask for one"
  (`GLOSSARY.md:274`).
- Q: Does a step on the sky fit the alt-az mount? → A: Yes. On an alt-az mount a
  camera's frame keeps its orientation to altitude and azimuth whatever the sky does, so
  one live pointing (the finder camera) or one imaging solve (the imaging camera) says
  for every later press which way the frame's up and right lie on the axes (plan.md
  § Technical Context). A step is an altitude change and an azimuth change scaled by the
  altitude's cosine. No equatorial direction is used anywhere.
- Q: What does Track do with a move of its axes? → A: Track holds a place on the sky.
  Its rate loop commands each axis the sky's rate plus a correction from each live
  pointing (`PROPORTIONAL_GAIN` 0.6 over a 10″ deadband,
  `crates/finder/src/tracking.rs:33`, `crates/finder/src/tracking.rs:35`), and never
  turns an axis against its gears' direction (`commanded`,
  `crates/finder/src/tracking.rs:1540`). 038's push check reads a pointing more than 5′
  from that place as a push by hand (`PUSH_THRESHOLD_DEG`,
  `crates/finder/src/session.rs:109`), and `Tracker::follow`
  (`crates/finder/src/tracking.rs:351`) makes a new place the one Track holds. A direct
  axis move while Track is on fights it, so a nudge with Track on moves Track's place
  instead (owner decision 4).
- Q: Which of the other drafts' files does this spec write? → A: Only
  `crates/finder/src/goto.rs` of 053's, and there only two new `AxisMove` variants and
  one new arm of `Planner::direct` with its helper; GoTo's correction solve, the model,
  the live solve and `session.rs` are not written (plan.md § Project Structure).

## Owner decisions

None is answered yet. Each keeps its options and its evidence, and the scenarios and
the record are written to the recommendations. One general rule is behind decisions 1
and 4 to 6: **a nudge is one step on the sky, never a sync and never a slip. With Track
off it is a direct axis move of both axes through every guard a GoTo passes; with Track
on it moves the place Track holds by the same step. Its larger step stays under 038's
5′ push threshold, so 038's push check, the model and the live solve's path to a sync
stay as they are.** So a nudge needs no rule of its own for the Sun, the minimum
altitude, cable wind, Stop, a GoTo, the model's epochs or 038's push; its own parts are
the step, the view the arrows follow, Track's moved place and one refusal while a push
by hand is still being read.

1. **Which frame the arrows follow, and before any solve** (FR-003 to FR-005).
   Recommended (a): **a choice of three views, `mount` by default**: the mount's own
   (up is higher altitude, right a clockwise turn), the finder camera's live views as
   the page draws them, turned by the chart rotation, and the imaging camera's frame
   from its latest imaging solve. Before a live pointing or an imaging solve, the
   camera views refuse with `the finder camera has no live pointing yet` and `the
   imaging camera has not solved a frame yet`; the mount view needs only the night's
   sync, as every direct axis move does. Evidence (run, plan.md § What was run): with
   Track off, every press of both steps in all four directions landed within 2.0 % of
   its step in the mount view, within 2.1 % in the finder camera's view at chart
   rotations 0°, 90° and 200°, and within 2.9 % in the imaging camera's view at imaging
   angles 17°, 100° and 250°, its direction within 1.2°. Instead: (b) the mount view
   only (the field night's first wish; the owner turns the arrows in his head on a
   rotated chart); (c) the imaging camera's frame only, falling back to the mount's
   before a solve (silent fallbacks change what an arrow does); (d) the finder camera's
   view only.
2. **The step sizes** (FR-002). Recommended (a): **fine 30″ and coarse 4′**: fine is
   about 55 pixels of the imaging camera (0.5475″ a pixel), a sixtieth of its 30′
   height, and the field night's 35″ GoTo error is one press; coarse is about a seventh
   of that height, so the field night's 6.2′ to 6.6′ offset is two coarse presses and a
   few fine ones, and it stays under the 5′ push threshold. Instead: (b) one step of 1′;
   (c) steps scaled to the chosen view's field (a tenth of the frame: about 3′ for the
   imaging camera, 8′ for the finder camera's 1.29° height), which for the finder
   camera would go over the push threshold; (d) steps the owner sets on Setup.
3. **Press or hold** (FR-001). Recommended (a): **one press, one step**: the move's end
   is known before it starts, so the guards check its whole path, and a phone that
   loses a release cannot leave the telescope running. Instead: (b) hold to move at a
   rate and stop on release (needs a release watchdog and a rate rule of its own; the
   page's top-bar Stop stays the only Stop either way).
4. **Track and a nudge** (FR-008, FR-009). Recommended (a): **the place Track holds
   moves by the step, and the axes move it at once**: the step is added to the next
   commands on top of the sky's rate, as fast as the gears go the sky's way. Evidence
   (run): with the gears, the telescope was within 10″ of the new place 0.7 s to 1.8 s
   after the press and stayed within 15.2″ of it; against the gears, since Track never
   reverses an axis, the axis waits for the sky to come to it: 3.6 s to 5.6 s for a fine
   step, about 66 s down and 80 s left for a coarse one; with no live solves the
   telescope still moved within 10″ of the new place in about 1 s to 2 s. 038's push
   check never fired. Instead: (b) move only the place and let the correction loop
   bring the telescope there from the live solves (run: a coarse up press took 30 s,
   and with no solves it never moved); (c) a press turns Track off, then moves the axes
   (the target is lost); (d) refuse a press while Track is on.
5. **Whether a nudge becomes a sync** (FR-010). Recommended (a): **never**: a nudge
   writes nothing to the pointing model, opens no zero-point epoch and marks no slip,
   and the live solve's syncs go on as before (053's rules, whatever they become, are
   not touched). Evidence (run): after the presses the model's epoch was the one before
   them, `slip_pending` stayed false, and the live solve recorded new syncs. Instead:
   (b) mark a slip, as a push by hand does (a nudge is a commanded move that the
   encoders count, not a slip); (c) make the next solve a sync of its own (053's to
   decide).
6. **Guards** (FR-006, FR-007, FR-009). Recommended (a): **every guard of a direct axis
   move, and the approach from below**: the Sun guard, the night's sync, the minimum
   altitude (a press may not go below it, nor lower than the telescope is when it is
   already below), a path clear of the Sun guard, the minimum altitude and cable wind,
   no GoTo running and the mount at rest; an axis that moves down goes 0.5° past and
   comes back up, as a GoTo approaches (014's `APPROACH`,
   `crates/finder/src/goto.rs:23`), so its gears end on the same flank; same-origin
   writes; MCP's moving-call rate limit; and, with Track on, a refusal while a push by
   hand is still being read. Evidence (run): the probe's first fine press up and right
   moved the tube 0″ until both axes had been pressed once down and once left: the
   simulator's 0.3° of backlash. Instead: (b) the same with no approach (shorter, but
   a down press ends on the other flank and the next up press loses up to the
   backlash).
7. **The page** (FR-014, FR-015). Recommended (a): **a Nudge block in the main page's
   guidance block, under Track**, with the view's choice, the step's and four arrows.
   Instead: (b) also beside the imaging camera's frame on the Imaging page (054 is
   rewriting that page; a follow-up after it lands); (c) on Setup's mount section.
8. **MCP** (FR-011). Recommended (a): **a `nudge` tool taking the route's three
   arguments**, logged and rate-limited as every moving tool is. Instead: (b) no tool
   (MCP keeps `move_axis`).
9. **Which way an arrow moves the picture** (FR-002). Recommended (a): **the telescope's
   aim moves the way the arrow points in the chosen view**, so the frame moves that way
   over the sky and the stars in it move the other way, as a hand controller's arrows
   do: a target right of centre is centred by pressing right. Instead: (b) the stars
   move the way the arrow points, so that target is centred by pressing left. The
   owner's words ("a press moves the picture the way the arrow points") fit either
   reading.

## Owner approvals

Flipping this spec to `ready` approves the answer to every owner decision above, and the
following. Each is named in the FR that makes it. No existing test, golden, fixture or CI
file changes.

1. **New public API in `crates/finder`'s library** (FR-001 to FR-008): the module
   `finder::nudge` with `FINE_ARCSEC`, `COARSE_ARCSEC`, `PUSH_QUIET_S`, `Direction`,
   `Step`, `View`, `NudgeAsk`, `NudgeView`, `offset`, `routes` and `nudge`; the
   `AxisMove` variants `Offset` and `Through`; and `Tracker::nudge`. No existing
   signature changes.
2. **A new route and tool** (FR-001, FR-011): `POST /api/mount/nudge` and the MCP tool
   `nudge`.
3. **A page action in `motion`** (FR-007): 016's FR-014 names the page actions `goto`,
   `second_sync` and `track`; a nudge from the page adds `nudge`.
4. **The glossary** (FR-013): **Nudge** is added, and **Direct axis move** names the
   nudge.
5. **`crates/finder`'s README: its section, row and MCP paragraph** (FR-012). The pinned sentence
   `The imaging camera has sixteen tools of its own` (`crates/finder/README.md:88`)
   stays true: the tool is in a paragraph of its own, after the finder camera's focus
   aid's.
6. **Two list entries in `crates/finder/web/playwright.config.ts`** (FR-016):
   `"**/nudge.spec.ts"` in the default project's `testIgnore` and in the simulate
   project's `testMatch`, each on a line of its own after `"**/tools-focus.spec.ts"`
   (`crates/finder/web/playwright.config.ts:45`,
   `crates/finder/web/playwright.config.ts:65`).
7. **The page** (FR-014, FR-015): the guidance block's Nudge block.

### Other specs' requirements this spec changes

- 016's FR-014, which reads: "While a GoTo, a direct axis move or tracking runs, the
  session SHALL hold `motion`, with three fields: `by`: `page` or `mcp`; `tool`: the
  page action (`goto`, `second_sync`, `track`) or the MCP tool that started it;
  `since`: when it started." A nudge from the page holds `motion` with `by` `page` and
  `tool` `nudge` while its move runs (FR-007).
- The glossary's **Direct axis move** (`GLOSSARY.md:273`), which reads: "Moving an axis
  to an axis position, or at a rate for a set time, asked for by axis rather than by
  target. Only MCP clients ask for one; it needs a sync and passes the same guards as a
  GoTo." FR-013 gives its new words.

No other spec's FR changes. 016's rule that over MCP a direct axis move is refused with
`Track is on` stands: a nudge with Track on is not a direct axis move (FR-008). 016's
FR-022 (`axis_move`) and 038's FR-001 to FR-016 stand as written: a nudge with Track off
is reported as the latest direct axis move, and a step is under 038's threshold.

## User Scenarios & Testing *(mandatory)*

Every scenario shares these definitions.

- **The gate commands.** Every Rust scenario runs in
  `cargo nextest run --workspace --all-targets --all-features --locked`, the `test`
  gate's first command, each test in its own process. The page's Vitest scenarios run in
  `npm test`, and the browser scenarios in `CI=1 npm run e2e`, later commands of the
  same gate, in `crates/finder/web`.
- **A press** is `POST /api/mount/nudge` whose `Host` header is localhost on port
  8080, with `Content-Type: application/json` and the body
  `{"direction": D, "step": S, "view": V}`. **The four ways** are `up`, `right`, `down`
  and `left`, at φ = 0°, 90°, 180° and 270° clockwise of the view's up; **the steps** are
  `fine`, 30″, and `coarse`, 240″. **A foreign write** is a press whose `Origin` is
  `http://elsewhere.example`.
- **The motion rig** is `crate::support::goto::Rig::new(START, 50.0, 120.0, true)` with
  START 1,768,528,800, its session's site 40° N 75° W and target M42, synced with
  `Rig::sync`, as `crates/finder/tests/support/goto.rs` builds it: the simulator's mount
  with 0.3° of backlash and 1,620,000 counts a turn, and its finder camera, whose
  frame-up is the zenith. **The synced rig** is that rig after a fine `down` and a fine
  `left` press in the `mount` view, each read until it rests, so that both axes end on
  the flank a GoTo leaves them on. **Rests** means `GET /api/session` reads
  `axis_move.state` other than `moving` and `Rig::stationary` is true, read every
  100 ms within 30 s, then 1 s more. **The tube's move** is the change of the
  simulator's `true_direction()`: its altitude change and its azimuth change times the
  cosine of the altitude before, in arcseconds. **Lands** within p % means that move is
  within p % of the step from the step's size at the press's angle clockwise of the
  zenith (up the altitude, right the azimuth).
- **The imaging rig** is `crates/finder/tests/motion/imaging_track.rs`'s tracking rig:
  a simulator on `crate::support::live::DB` with `Site::default()` and a
  `VirtualClock` from `crate::support::live::START`, its mount connected, pushed to
  altitude 50° and azimuth 120°, its imaging optics at the angle the scenario gives, a
  `SimNode` with the token `test-token` and a 500 ms frame interval over an in-memory
  duplex, and a session with that node, site 40° N 75° W and target M42, served with
  `with_imaging` in the test's scratch folder and the simulated imaging solver. **Its
  frame's centre** is the simulator's `imaging_truth()`, and **where it would be** is
  that centre's altitude and azimuth at the press, turned back to RA and Dec at the
  read, as the sky turns under a still mount.
- **The Track rig** is the motion rig with the simulator's eyepiece pointed at M42
  before the sync, Track turned on with `POST /api/track` `{"on": true}`, and 30 s of
  tracking, as 038's push tests set it up. **The true place** is the simulator's
  eyepiece; **the stepped place** is the place the step's size from the true place at
  the press, at the press's angle clockwise of the zenith, turned to RA and Dec with
  the simulator's own horizon transform.
- **The bare console** is `finder::router` over `AppState::shared` alone, with no link.
- **The MCP rig** is `crate::support::mcp::sim::Rig::new(START, 50.0, 120.0, true)`,
  synced, its tools called with `tools/call` as `crates/finder/tests/mcp/` calls them;
  **the call log** is `with_call_log` of a writer the test reads.
- **The page rig** is `GuidanceBlock` rendered by Testing Library from the recorded
  session `tracking` of `crates/finder/web/src/test/fixtures/session-track.ts`, with
  `fetch` stubbed to answer what the test writes, as the page's Track tests do.
- **The simulated console** is the simulate project's server of
  `crates/finder/web/playwright.config.ts`, the release `finder` with `--simulate`, at
  412 × 915 CSS pixels and a device scale of 2, with the site 40° N 75° W, the imaging
  camera `{"width_mm": 23.5, "height_mm": 15.6, "pitch_um": 3.76}`, the target M42, a
  minimum altitude of 15°, the simulated telescope at altitude 50° and azimuth 120°,
  the night reset and frames running until `model.syncs` is above 0, as
  `crates/finder/web/e2e/tracking.spec.ts` sets it up. **The Nudge block** is the
  region named `Nudge` inside the region whose name starts `Target M42`; every page
  locator of these scenarios is held to it.
- **Oracles.** The simulator's true tube direction, eyepiece and imaging truth and its
  own horizon transform; the steps' sizes and the ways' angles; the mount's counts a
  turn (half a degree is 2,250 counts); the words of these scenarios and of 014, 016
  and 038; and DESIGN.md's night ink. No expected value comes from the code under test:
  no test calls `offset` for an expected value or reads `FINE_ARCSEC`, `COARSE_ARCSEC`
  or `PUSH_QUIET_S`.

### User Story 1 - With Track off, the arrows move the telescope one step in the mount's or the finder camera's view (Priority: P1)

As the owner, I need one press of an arrow to move the telescope one small step on the
sky, the way the arrow points, in the mount's own view or in the finder camera's view as
the page draws it, through every guard a GoTo passes.

**Why this priority**: The step, the route and the guards are the feature; the other
stories add a view, Track, MCP and the page to them.

**Independent Test**: `crates/finder/tests/motion/nudge.rs`, new. The `test` gate's
nextest command runs it; during development,
`cargo nextest run -p finder --locked --test motion -E 'test(nudge::)'`.

**Acceptance Scenarios**:

1. **Given** the synced rig, **When** a committed test presses coarse `right` in the
   `mount` view, reads `GET /api/session` at once, reads until it rests, then presses
   each step each way in the `mount` view, reading each until it rests, **Then** it
   asserts that the first answers 200 with exactly
   `{"up_arcsec": 0.0, "right_arcsec": 240.0, "moved": "axes"}` and the session read at
   once has `motion.by` `page` and `motion.tool` `nudge`; that
   every press answers 200 with `moved` `axes` and its move rests with
   `axis_move.state` `done` and `axis_move.tool` `nudge`; that each lands within 3 %;
   that the model's `epoch` after the presses is the one before them; that
   `slip_pending` is false; and that the model holds a sync whose time is after the
   moment those presses began.
2. **Given** the synced rig with `PUT /api/session` `{"chart_rotation_deg": 200}`,
   **When** a committed test presses each step each way in the `finder_camera` view,
   reading each until it rests, **Then** it asserts that each answers 200 with `moved`
   `axes` and lands within 3 % at φ + 200° clockwise of the zenith.
3. **Given** the synced rig, **When** a committed test presses fine `down`, then fine
   `up`, then fine `left` in the `mount` view and reads the moved axis's counts every
   100 ms until each rests, **Then** it asserts that for `down` (altitude) and `left`
   (azimuth) the axis's end is at least 2,240 counts above the lowest count it read and
   below its start, and that for `up` no count read is below its start.
4. **Given** the bare console, and then the synced rig, **When** a committed test
   presses fine `up` on the bare console in the `finder_camera` and then the `mount`
   view; then on the rig sends a foreign write, a press as `text/plain`, a press with
   the extra key `"x": 1`, a press with the direction `north`, and two fine `up`
   presses in the `mount` view one after the other; reads until it rests; sets
   `PUT /api/session` `{"min_alt_deg": a − 0.2}`, a the session's `pointing.alt_deg`,
   and presses coarse `down` and then coarse `up`; reads until it rests; stops the
   node's frames, sends `POST /api/model/reset` `{"what": "night"}` and presses fine
   `up`, **Then** it asserts that the bare console answers 409 with exactly
   `{"error": "the finder camera has no live pointing yet"}` and
   `{"error": "no mount is connected"}`; that the foreign write answers 403 with
   exactly `{"error": "writes require the console's own origin and an allowed host"}`;
   that `text/plain` answers 415, the extra key 400 and `north` 400; that the first of
   the two presses answers 200 and the second 409 with exactly
   `{"error": "the mount is already moving"}`; that coarse `down` answers 409 with
   exactly `{"error": "the position is below the minimum altitude"}` and coarse `up`
   200; and that the last press answers 409 with exactly
   `{"error": "the night has no sync yet"}`.
5. **Given** the motion rig, synced, with the target M42, **When** a committed test
   presses coarse `down` in the `mount` view and, 100 ms later, sends
   `POST /api/track` `{"on": true}`, then reads until it rests, **Then** it asserts that
   the press answers 200, that Track's request answers 409 with exactly
   `{"error": "the mount is already moving"}`, and that the move rests with
   `axis_move.state` `done` while `track` is absent from the session or reads `off`.

### User Story 2 - The arrows follow the imaging camera's frame, and Track holds the nudged place (Priority: P1)

As the owner, I need the arrows to follow the imaging camera's own up and right from its
latest imaging solve, and a press with Track on to move the place Track holds, so that I
can centre a target in the imaging camera's frame while Track runs.

**Why this priority**: The field night's wish was the imaging camera's frame with Track
on; without this story the arrows do not reach it.

**Independent Test**: `crates/finder/tests/motion/nudge_imaging.rs` and
`crates/finder/tests/motion/nudge_track.rs`, new. The `test` gate's nextest command
runs them; during development,
`cargo nextest run -p finder --locked --test motion -E 'test(nudge_imaging::) | test(nudge_track::)'`.

**Acceptance Scenarios**:

1. **Given** the bare console, and then the imaging rig at an imaging angle of 100°
   once `GET /api/imaging` reads `state` `ready` and the model holds a sync, **When** a
   committed test presses fine `up` in the `imaging_camera` view on the bare console
   and on the rig; presses fine `down` and then fine `left` in the `mount` view, reading
   each until `axis_move.state` is `done`; takes an imaging frame with
   `POST /api/imaging/frame`, reads it until it is saved, solves it with
   `POST /api/imaging/solve` and reads until its solve is `solved`; and then presses
   each step each way in the `imaging_camera` view, reading each until
   `axis_move.state` is `done` and 1 s more, **Then** it asserts that both first
   presses answer 409 with exactly
   `{"error": "the imaging camera has not solved a frame yet"}`; and that after each
   later press, which answers 200, the frame's centre has moved from where it would be,
   by the step's size within 4 % of the step, the way the arrow points in the imaging
   frame: φ clockwise of the frame's up, read from the simulator's
   `up_east_of_north_deg`.
2. **Given** the Track rig, **When** a committed test, each time after reading every
   100 ms until the true place is within 10″ of the place Track holds, presses coarse
   `up` and then coarse `right` in the `mount` view and after each reads every 100 ms
   the first time the true place is within 10″ of the stepped place and the farthest it
   is from it over the next 20 s, **Then** it asserts that each answers 200 with `moved` `track`;
   that each comes within 10″ within 5 s and stays within 30″; that after each
   `track.state` is `tracking` and `track` has no `push` key; and that the model's
   `epoch` is the one before the presses and `slip_pending` false.
3. **Given** the Track rig, **When** a committed test presses fine `down` and then fine
   `left` in the `mount` view and reads as in US2-S2, **Then** it asserts that each
   answers 200 with `moved` `track`, comes within 10″ of the stepped place within 15 s
   and stays within 30″ of it, with `track.state` `tracking` and no `push` key.
4. **Given** the Track rig, read until the true place is within 10″ of the place Track
   holds, with the node's frames then stopped, **When** a committed test presses
   coarse `right` in the `mount` view and reads the true place every 100 ms for 5 s,
   **Then** it asserts that the press answers 200 and that the true place came within
   10″ of the stepped place.
5. **Given** the Track rig with the simulator's mount pushed down by hand by 2°
   (`push_by(0.0, -2.0)`), **When** a committed test presses fine `up` in
   the `mount` view the first time `track.push.state` reads `moving`, again the first
   time it reads `following`, and once more 10 s after that, **Then** it asserts that
   the first answers 409 with exactly
   `{"error": "a push by hand is still settling: press again in a few seconds"}`, that
   the second answers 409, and that the third answers 200 with `moved` `track`.

### User Story 3 - MCP and the README carry the nudge (Priority: P2)

As an MCP client, I need the page's nudge as a tool with the route's answers, and as the
owner, I need the README and the glossary to say what a nudge is.

**Why this priority**: The field night's planet was moved over MCP; the route alone
carries the feature.

**Independent Test**: `crates/finder/tests/mcp/nudge.rs`, new. The `test` gate's
nextest command runs it; during development,
`cargo nextest run -p finder --locked --test mcp -E 'test(nudge::)'`.

**Acceptance Scenarios**:

1. **Given** the MCP rig with the call log, **When** a committed test reads
   `tools/list`, calls `nudge` with `{"direction": "right", "step": "coarse", "view":
   "mount"}`, reads `GET /api/session` at once, calls `set_tracking` with
   `{"on": true}`, and reads the session until `axis_move.state` is not `moving`,
   **Then** it asserts that `nudge` is listed with a description containing "real
   telescope", an `outputSchema` of type `object` and `inputSchema` properties
   `direction`, `step` and `view` each of type `string`; that the call's
   `structuredContent` is exactly
   `{"up_arcsec": 0.0, "right_arcsec": 240.0, "moved": "axes"}`; that the session read
   at once has `motion.by` `mcp` and `motion.tool` `nudge`; that `set_tracking` answers
   `isError` true with the text `a direct axis move is running`; that the mount's
   azimuth counts grew and its altitude counts did not change; and that the call log
   has a line containing `mcp nudge` and no line containing `test-token`.
2. **Given** the MCP rig before its sync, then synced, **When** a committed test calls
   `nudge` with fine `up` in the `mount` view; after the sync calls it with
   `{"direction": "north", "step": "fine", "view": "mount"}`, with no `view`, and with
   the extra key `"x": 1`; waits 1.1 s; calls it three times one after another with coarse
   `up` in the `mount` view; reads until the move ends and 1 s more; and calls it with fine
   `up` in the `imaging_camera` view, **Then** it asserts that the first answers
   `isError` true with the text `the night has no sync yet`; that the three malformed
   calls answer `isError` true with exactly `nudge takes direction (up, right, down or
   left), step (fine or coarse) and view (mount, finder_camera or imaging_camera)`;
   that of the three the first is not an error, the second answers
   `the mount is already moving` and the third
   `the rate limit of 2 moving calls a second was reached`; and that the last answers
   `the imaging camera has not solved a frame yet`.
3. **Given** the committed `crates/finder/README.md` and `GLOSSARY.md`, **When** a
   committed test reads them, **Then** it asserts that the README still contains `The
   imaging camera has sixteen tools of its own`; that `The page's nudge arrows have one
   tool:` lies after `The Tools page's focus aid for the finder camera has two tools:`
   and before `### MCP under WSL2`, with a line starting ``- `nudge` takes `direction` ``
   between them; that the `## API` section has a row starting
   ``| `POST /api/mount/nudge` |``; that `## Nudging the telescope` lies after
   `## Drawn sky on the real mount` and before `## Recording a night` and contains
   `fine is 30″, coarse is 4′`, `the finder camera has no live pointing yet` and `A
   nudge is never a sync`; and that the glossary has the entry **Nudge** with the
   words of FR-013 and no longer contains `Only MCP clients ask for one;`.

### User Story 4 - The guidance block's Nudge block (Priority: P2)

As the owner at the telescope with a phone, I need four arrows under Track, a choice of
the view they follow and of the step, and the console's words when a press is refused.

**Why this priority**: The owner asked for buttons; the tool and the route are not
buttons.

**Independent Test**: `crates/finder/web/src/components/NudgeBlock.test.tsx` and
`crates/finder/web/e2e/nudge.spec.ts`, new. The `test` gate's `npm test` and
`CI=1 npm run e2e` run them; during development, `npm test -- NudgeBlock` and
`npx playwright test --project=simulate e2e/nudge.spec.ts` in `crates/finder/web`.

**Acceptance Scenarios**:

1. **Given** the page rig with `fetch` answering
   `{"up_arcsec": 0, "right_arcsec": 240, "moved": "axes"}`, **When** a committed test
   chooses `finder_camera` in the Nudge block's `Arrows follow` box, clicks `Coarse 4′`
   and then `Nudge right`, **Then** it asserts that `Coarse 4′` is pressed
   (`aria-pressed` true); that `fetch` was called with `/api/mount/nudge`, method
   `POST`, the header `Content-Type: application/json` and the body
   `{"direction":"right","step":"coarse","view":"finder_camera"}`; that `onChange` was
   called once; and that the block's status reads exactly `Nudged right 4′`.
2. **Given** the page rig with `fetch` answering 409 with
   `{"error": "the night has no sync yet"}`, **When** a committed test clicks `Nudge
   up`, **Then** it asserts that the Nudge block's alert reads exactly `the night has
   no sync yet`.
3. **Given** the page rig's session without `imaging_camera`, then with it, then
   without `mount`, **When** a committed test renders each, **Then** it asserts that
   the `Arrows follow` box offers exactly `the mount` and `the finder camera`, then
   `the mount`, `the finder camera` and `the imaging camera`, and that without a mount
   there is no Nudge block.
4. **Given** the simulated console with Track off and the page open by day, **When** a
   committed browser test reads the `Arrows follow` box, clicks `Nudge right`, reads
   the session until `axis_move.state` is `done` and `mount.moving` false, clicks
   `Coarse 4′`, clicks `Nudge down` and at once `Nudge up`, and reads until it rests,
   **Then** it asserts that the box reads `mount`; that the first press's response is
   200 with exactly `{"up_arcsec": 0, "right_arcsec": 30, "moved": "axes"}` and the
   block's status reads `Nudged right 30″`; that `Nudge down` answers exactly
   `{"up_arcsec": -240, "right_arcsec": 0, "moved": "axes"}`; and that `Nudge up`
   answers 409 and the block's alert reads `the mount is already moving`.
5. **Given** the simulated console with Track turned on with the guidance block's Track
   switch and read until `track.state` is `tracking`, **When** a committed browser test
   clicks `Coarse 4′` and `Nudge up` and reads the session, **Then** it asserts that
   the press's response is 200 with exactly
   `{"up_arcsec": 240, "right_arcsec": 0, "moved": "track"}`; that the block's status
   reads `Nudged up 4′ · Track holds the new place`; and that `track.state` is
   `tracking` with no `push`.
6. **Given** the simulated console with the page open by day, **When** a committed
   browser test screenshots the Nudge block, turns on `Night mode: red and black`, reads
   `data-theme` `night` on `html`, waits 1.5 s and screenshots the block again,
   **Then** it asserts with 012's off-red count (a pixel whose red is above 226 or whose
   green or blue is above 0) that the day screenshot has more than 1,000 off-red pixels
   and the night one none.

### Edge Cases

- **A press during a GoTo, a direct axis move or another press.** Refused with
  `a GoTo is already running` or `the mount is already moving` (US1-S4), as every direct
  axis move is; the page's Track turned on during a press is refused with `the mount is
  already moving` (US1-S5), and MCP's with `a direct axis move is running` (US3-S1),
  both as at f6d4f5fd.
- **Stop.** The top bar's Stop and MCP's `stop` end a press's move as they end any
  direct axis move (016's FR-012); `axis_move` then reads `halted` with `the observer
  stopped it`. Not run for a nudge; the path is `mcp::guard`'s, whose Stop tests pass
  unchanged (run).
- **Near the minimum altitude.** A press that would end or pass below it is refused
  (US1-S4); a telescope already below it may not go lower, the floor `move_axis` uses
  (`crates/finder/src/goto.rs:200`), and the path check keeps 016's owner approval 11
  ("A telescope parked low may climb"). The low start is not run for a nudge.
- **Across the Sun guard or the cable's turn.** Refused with `no path keeps clear of the
  Sun guard, minimum altitude and cable wind` (014's words). Not run for a nudge.
- **The imaging camera's view after the optics change.** The latest imaging solve
  carries the frame's orientation until the next; a camera turned in its holder after
  a solve gives arrows turned with it until the next solve (plan.md § Risks).
- **A coarse press while Track's error is already large.** The step is under 038's 5′
  threshold, but a step against an error of more than 1′ could be read as a push; the
  probe never saw one (plan.md § Risks).
- **Track in its keyhole wait or armed.** A press with Track wanted but not `tracking`
  is refused with `Track is not tracking yet`. Not run.

## Requirements *(mandatory)*

### Functional Requirements

**The step and the views** (US1, US2):

- **FR-001**: The console SHALL serve `POST /api/mount/nudge`, a same-origin write as
  `POST /api/track` is, taking a JSON body of exactly `direction` (`up`, `right`,
  `down` or `left`), `step` (`fine` or `coarse`) and `view` (`mount`, `finder_camera`
  or, from US2, `imaging_camera`), refusing any other key or value with 400 and any
  other content type with 415. One request is one step: a press. It SHALL answer 200
  with `{"up_arcsec": U, "right_arcsec": R, "moved": M}`, U and R the step in the
  mount's view (up the altitude, right a clockwise turn) and M `axes` with Track off or
  `track` with Track on, or 409 with a JSON `error` naming the refusal.
- **FR-002**: `fine` SHALL be 30″ and `coarse` 240″ on the sky (owner decision 2). The
  telescope's aim SHALL move the step's size at φ clockwise of the chosen view's up,
  φ 0°, 90°, 180° and 270° for `up`, `right`, `down` and `left` (owner decision 9).
- **FR-003**: In the `mount` view, `up` SHALL be higher altitude and `right` a clockwise
  turn, and the answer's U and R SHALL be exactly the step and 0 (or 0 and the step,
  signed).
- **FR-004**: In the `finder_camera` view, the arrows SHALL follow the finder camera's
  live views as the page draws them, turned counter-clockwise by the session's chart
  rotation R: the press's angle clockwise of the zenith is φ + R − u, u the latest live
  pointing's `up_deg`. With no live pointing it SHALL answer 409 with `the finder
  camera has no live pointing yet`.
- **FR-005**: *(US2.)* In the `imaging_camera` view, the arrows SHALL follow the imaging
  camera's frame as its latest imaging solve gives it: the solve's `cd_deg`, RA, Dec and
  time, and the session's site, turned into altitude and azimuth with the parallactic
  angle at the solve's place and time; up is towards row 0 and right towards the last
  column. Without a solved latest imaging solve it SHALL answer 409 with `the imaging
  camera has not solved a frame yet`, and without a site `a site is needed`.

**Track off** (US1):

- **FR-006**: With Track off, a press SHALL be a direct axis move of both axes started
  through the path of MCP's direct axis moves (`api::start_motion` and
  `GoTo::check_axis_move`), so that it passes their every guard and refusal: no mount,
  `Sun is up`, `the night has no sync yet`, `a GoTo is already running`, `the mount is
  already moving`, `the position is below the minimum altitude` and `no path keeps
  clear of the Sun guard, minimum altitude and cable wind`. Its end SHALL be the model's
  inverse at the altitude and azimuth the step gives from the model's prediction at the
  axes' counts, added to the counts as a difference. An axis whose counts go down SHALL
  first go 0.5° past its end and then come up to it, as a GoTo's approach does. Every
  waypoint SHALL be at or above the lower of the minimum altitude and the predicted
  altitude at the press.
- **FR-007**: While a press's move runs, the session SHALL hold `motion` with `by`
  `page` (from the route) or `mcp` (from the tool) and `tool` `nudge`, and SHALL report
  it as `axis_move` with `tool` `nudge` and `state` `moving`, then `done` or `halted`
  with its reason (016's FR-014 and FR-022).

**Track on** (US2):

- **FR-008**: With Track on and `tracking`, a press SHALL call `Tracker::nudge` with the
  step: the place Track holds SHALL move by it (east and north from the step's up and
  right with the parallactic angle at the moment), and the step, turned into each axis's
  arcseconds, SHALL be added to the axes' next commands over the sky's rate until it is
  done, never turning an axis against its gears' direction. The tracker SHALL clear its
  correction's memory as `follow` does, use no live pointing of the move until 1 s plus
  its settle after it, and stay `tracking`; no push is read and `track` gains no key. A
  press with Track wanted but not `tracking`, or with a backlash being measured, SHALL
  answer 409 with `Track is not tracking yet`.
- **FR-009**: With Track on, a press SHALL answer 409 with `a push by hand is still
  settling: press again in a few seconds` while `track.push.state` is `moving`, or is
  `following` and less than 10 s from the follow (038's `PUSH_REOPEN_S`).

**Never a sync** (US1, US2):

- **FR-010**: A press SHALL write nothing to the pointing model: its zero-point epoch
  and `slip_pending` stay as they were, and the live solve's syncs go on as before
  (owner decision 5).

**MCP and the words** (US3):

- **FR-011**: The MCP endpoint SHALL offer `nudge`, taking `direction`, `step` and
  `view` and nothing else as the route does, through the same function, rate-limited
  and written to the call log as every moving tool is, answering the route's view as
  `structuredContent` or its refusal as a tool error, and a malformed call with
  `nudge takes direction (up, right, down or left), step (fine or coarse) and view
  (mount, finder_camera or imaging_camera)`. Its description SHALL say it moves a
  "real telescope". With Track on it is not refused with `Track is on`: it moves
  Track's place (FR-008).
- **FR-012**: `crates/finder/README.md` SHALL gain the paragraph `The page's nudge
  arrows have one tool:` with its `nudge` entry after the finder camera's focus aid's
  tools and before `### MCP under WSL2`; the `## API` row of `POST /api/mount/nudge`
  after `POST /api/goto`'s; and the section `## Nudging the telescope` after `## Drawn
  sky on the real mount`, with the words of plan.md § Interfaces.
- **FR-013**: `GLOSSARY.md` SHALL define **Nudge**: "One press of a nudge arrow: the
  telescope moves one step on the sky, fine 30″ or coarse 4′, up, right, down or left
  in the view the arrows follow: the mount's own (up is higher altitude, right a
  clockwise turn), the finder camera's live views as the page turns them, or the
  imaging camera's frame from its latest imaging solve. With Track off it is a direct
  axis move; with Track on the place Track holds moves by the step. A nudge is never a
  sync and never a slip." with _Avoid_: jog, slew, manual move; and **Direct axis
  move** SHALL read: "Moving an axis to an axis position, or at a rate for a set time,
  asked for by axis rather than by target, or both axes by one nudge's step. MCP
  clients ask for the first two; a nudge with Track off is the third. Each needs a sync
  and passes the same guards as a GoTo."

**The page** (US4):

- **FR-014**: The guidance block SHALL show, under Track and while the mount is
  connected, a region named `Nudge` holding a box `Arrows follow` (`the mount`, `the
  finder camera`, and `the imaging camera` when the session has an imaging camera;
  `the mount` first), two buttons `Fine 30″` and `Coarse 4′` of which the chosen one is
  pressed (`Fine 30″` first), and four buttons `Nudge up`, `Nudge right`, `Nudge down`
  and `Nudge left` drawn as arrows in a cross, at least 44 CSS pixels each. A click
  SHALL post the press, disable the arrows until it answers, then show the status
  `Nudged <way> <30″ or 4′>`, with ` · Track holds the new place` when `moved` is
  `track`, and refresh the session; a refusal SHALL show its `error` in the block's
  alert.
- **FR-015**: The Nudge block SHALL follow the red-only night rule.
- **FR-016**: `playwright.config.ts` SHALL gain only owner approval 6's two entries, and
  `crates/finder/web/dist` SHALL be the build of its source.

### Key Entities

- **Nudge**: one press; a direction, a step and a view; answered with the step in the
  mount's view and what moved.
- **The view the arrows follow**: `mount`, `finder_camera` or `imaging_camera`; the
  last two read the latest live pointing or imaging solve.
- **The place Track holds**: 038's followed place, moved by a nudge with Track on.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario has a committed test that asserts it, and the
  `lint` and `test` gates pass.
- **SC-002**: No existing test, golden, fixture, `scripts/ci/` file, `.github/` file,
  `CODEOWNERS` entry or `ergane.yaml` is edited, and `playwright.config.ts` gains only
  its two list entries. Every existing test passes unchanged (plan.md § Shapes that
  existing tests pin). The only edits to existing test files are the `mod` lines of
  `crates/finder/tests/motion/main.rs` and `crates/finder/tests/mcp/main.rs`.
- **SC-003**: With Track off each press lands within 3 % of its step in the mount's and
  the finder camera's views (US1-S1, US1-S2) and within 4 % in the imaging camera's
  (US2-S1); with Track on the telescope is within 10″ of the stepped place within 5 s
  with the gears and 15 s against them for a fine step (US2-S2, US2-S3).
- **SC-004**: No press is a sync or a slip (US1-S1, US2-S2), every guard's refusal is
  the route's (US1-S4, US2-S5, US3-S2), and the page shows the console's own words
  (US4-S2, US4-S4).

## Assumptions

- **Earlier features are in place.** Every spec in `depends_on_landed` has landed. Every
  citation here was read at f6d4f5fd.
- **The simulator stands for the mount and cameras** in the gates: its mount's backlash,
  counts and rates, its finder camera with the zenith at frame-up, and its imaging
  camera at the angle a test sets.
- **The mount is alt-az.** Nothing here is equatorial; the future equatorial mount is
  not designed for (the step on the sky and the views carry over; the axes' arithmetic
  would not).
- **Out of scope:**
  - equatorial directions and a hand controller;
  - centring from an imaging solve alone (a GoTo to the imaging camera's solved place);
  - solar-system targets;
  - 053's sync rules, GoTo's correction solve, the model and the live solve;
  - the Imaging page (owner decision 7 (b)), and remembering the chosen view or step
    across page loads;
  - a press held to keep moving (owner decision 3 (b));
  - the field night's centring by the owner (plan.md § Outside the factory).
- **The factory's gates and CI:** the gates are `lint` and `test`, copied in plan.md
  § Verification. CI's other jobs run only in the merge queue, so no scenario's Then
  clause rests on them.

## Work Graph

```yaml
US1:
  depends_on: []
  implements: [FR-001, FR-002, FR-003, FR-004, FR-006, FR-007, FR-010]
US2:
  depends_on: []
  depends_on_merged: [US1]
  implements: [FR-005, FR-008, FR-009]
US3:
  depends_on: []
  depends_on_merged: [US2]
  implements: [FR-011, FR-012, FR-013]
US4:
  depends_on: []
  depends_on_merged: [US3]
  implements: [FR-014, FR-015, FR-016]
```

Four stories, each under 60 % of 65,536 B (39,321 B) as the probe's commits measure
them, the largest US2 at about 41 %; at the factory's worst overrun seen, 1.35 times,
each stays under that line (plan.md § Plan traps 1): the step, the route and the guards;
the imaging camera's view and Track; MCP, the README and the glossary; the page. Each
merges after the one before (`depends_on_merged`): US2 adds a view and Track's branch to
US1's `nudge` module; US3's tool calls US2's `nudge` with every view; the page posts the
route and its browser test needs Track's branch. Plain `depends_on` would order them
without putting the earlier code in the later story's base.
