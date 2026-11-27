---
state: draft
depends_on_landed: ["014-goto", "015-tracking", "016-mcp", "017-console-stop-confirm", "019-night-legibility", "021-chart-orientation", "022-altaz-guidance", "024-mount-link-recovery", "032-imaging-solve-and-aim-point", "038-track-follows-a-push", "040-stack-waits-for-steady", "042-imaging-run-page-and-mcp", "050-tools-page-aligns-finder-camera", "051-tools-page-focuses-either-camera"]
# Drafted 2026-11-27 against `main` at f6d4f5fd, at the owner's ask of 2026-11-27 ("Your
# rec. Proceed", to the seat's ranked list after a field night), from a probe of that day
# on the branch `probe/055` cut from f6d4f5fd; cynical review round 1 folded the same day
# on that branch (plan.md § What was run and what was read). Every `path:line` here was
# read at f6d4f5fd. None of the ten owner decisions is answered yet; the scenarios are
# written to each recommendation. 014 is named for the direct move's guards and approach;
# 015 and 038 for Track and its push check; 016 for direct axis moves, MCP and
# `axis_move`; 021 for the chart rotation and the live pointing's up; 032 for the imaging
# solve; 019 for the night rule; 017, 022, 024, 040, 042, 050 and 051 because each last
# changed a file this spec writes (plan.md § Refinement Record). 052, 053 and 054 are
# drafted beside this spec and land before it, in that order; it is re-anchored on them
# before any flip.
---

# Feature Specification: The Nudge Arrows Follow the Frame

**Feature Branch**: `055-nudge-arrows-follow-the-frame`

**Created**: 2026-11-27

**Status**: Draft

**Input**: On a field night the owner had a target slightly off centre after a GoTo and
Track (the GoTo's error was 35″, yet the target was not centred in the finder camera's
view) and wanted to "nudge the scope onto it", with arrows where "up and down are
altitude, left and right are azimuth, as the alt-az mount moves" (10:03 PM). Later that
night, moving a planet into the imaging camera's frame by hand over MCP (`move_axis`, a
signed rate for a set time; +0.1°/s for 2 s gave +0.19°), the wish became
"up/down/left/right buttons that read the frame's own up and right" (11:24 to 11:32 PM).
On the same night every stack of M42 sat 6.2′ to 6.6′ off the imaging camera's centre.
Today the page has no button that moves the telescope a little, and MCP's moves are by
axis and rate. Those three quotations are the owner's only logged words on the feature.

## Clarifications

### Session 2026-11-27 (the probe on `main` at f6d4f5fd)

- Q: What does this trio read besides itself? → A: Nothing about the feature. `spec.md`
  holds every requirement and scenario, and `plan.md` every design decision, the
  interfaces, the shapes that existing tests pin and the alternatives rejected. The
  probe's branch and logs are not in the repository; what they showed is written here.
- Q: What moves the mount by hand today? → A: Only MCP's two direct axis moves,
  `goto_axis_position` and `move_axis` (`crates/finder/src/mcp/tools.rs:430`,
  `crates/finder/src/mcp/tools.rs:459`), through `mcp::guard::start`
  (`crates/finder/src/mcp/guard.rs:21`) and `api::start_motion`
  (`crates/finder/src/api.rs:797`), which refuses them over MCP with `Track is on`
  (`crates/finder/src/api.rs:817`) and, while a direct move runs, refuses the tools of
  one list (`crates/finder/src/api.rs:835`) with `the mount is already moving`; and
  `GoTo::check_axis_move` (`crates/finder/src/goto.rs:772`), whose `Planner::direct`
  (`crates/finder/src/goto.rs:220`) asks for the night's sync and the guards of a GoTo.
  The page has none; the glossary says "Only MCP clients ask for one"
  (`GLOSSARY.md:274`).
- Q: Does a step on the sky fit the alt-az mount? → A: Yes. A camera fixed to the tube of
  an alt-az mount keeps its orientation to altitude and azimuth whatever the sky does,
  so one live pointing (the finder camera) or one imaging solve (the imaging camera)
  says for every later press which way the frame's up and right lie on the axes (plan.md
  § Technical Context). A step is an altitude change and an azimuth change scaled by the
  altitude's cosine. No equatorial direction is used anywhere.
- Q: Which of the other drafts' files does this spec write? → A: Of 053's, only
  `crates/finder/src/goto.rs`, and there only two new `AxisMove` variants, one new arm of
  `Planner::direct`, its helper and one refusal; GoTo's correction solve, the model, the
  live solve and `session.rs` are not written. 052, 053 and 054 also write
  `crates/finder/README.md`, `GLOSSARY.md` and the `console` and `motion` test
  `main.rs` files, and 054 rewrites `crates/finder/web/src/components/ImagingPage.tsx`
  (plan.md § Prerequisites).

### Session 2026-11-27 (cynical review round 1, folded on `probe/055`)

- Q: With Track on, is a nudge read as a push by hand? → A: In the draft, yes: Track held
  the moved place at once while the axes lagged, and 038's check
  (`crates/finder/src/session.rs:1281`) read the gap as a push, marked a slip and opened
  a new epoch. Now, while a step is to be made or being made and for 1 s plus Track's
  settle after it, the tracker's `checks_push` (`crates/finder/src/tracking.rs:331`) is
  false and no live pointing feeds its loop, and a press while a step is still being made
  is refused (FR-011, FR-012). The review ran its two trial lines on the draft, and A
  and B then passed; the fold starts from them, and its design passes A and B as US3-S3
  and US3-S4 (run).
- Q: Is Track turned on during a press refused? → A: In the draft only when the
  telescope was off target: `start_tracking` GoTos first only when the pointing is more
  than 1′ from the target (`crates/finder/src/session.rs:1046`), and the reservation list
  (`crates/finder/src/api.rs:835`) did not hold `track` or `nudge`. Now it holds both
  (FR-008): Track on during a press and a second press at the approach's turnaround
  answer 409 `the mount is already moving` (US1-S3, US1-S4).
- Q: Is the approach GoTo's? → A: Now yes: every moved axis approaches its end from 0.5°
  below, as a GoTo approaches every axis (`crates/finder/src/goto.rs:370`), with GoTo's
  words for an approach under the floor (`crates/finder/src/goto.rs:372`). With Track on,
  Track makes the step and each moved axis comes back the way Track drives it (FR-010).
- Q: Which owner words does decision 9 rest on? → A: Only the three quoted in the Input.
  The draft's "a press moves the picture the way the arrow points" was the seat's gloss.

## Owner decisions

None is answered yet. Each keeps its options and its evidence, and the scenarios and
the record are written to the recommendations. One general rule is behind decisions 1,
4, 5 and 6: **a nudge is one step on the sky, never a sync, never a slip and never read
as a push. Every moved axis ends its step moving the way it is next driven: up from
0.5° below, as a GoTo's approach, with Track off; Track's own way with Track on. With
Track off a press is a direct axis move of both axes through every guard a GoTo passes;
with Track on the place Track holds moves by the step and Track makes it.** So a nudge
needs no rule of its own for the Sun, the minimum altitude, cable wind, Stop, a GoTo or
the model's epochs; its own parts are the step, the view the arrows follow, Track's
step, the quiet window and the refusals while a step or a push by hand is still being
made.

1. **Which frame the arrows follow** (FR-003 to FR-005). Recommended (a): **a choice of
   three views**: the mount's own (up is higher altitude, right a clockwise turn), the
   finder camera's live views as the page draws them, turned by the chart rotation, and
   the imaging camera's frame from its last solved imaging solve. Before a live pointing
   or a solved imaging solve the camera views refuse with their words. Evidence (run):
   with Track off every press of both steps each way landed within 3 % of its step in
   the mount view (US1-S1) and in the finder camera's view at chart rotation 200°
   (US2-S1), and within 4 % in the imaging camera's at imaging angle 100° (US2-S3); the
   draft's measurement runs at chart rotations 0°, 90° and 200° and imaging angles 17°,
   100° and 250° landed within 2.9 %. Instead: (b) the mount view only; (c) the imaging
   camera's frame only, falling back to the mount's before a solve (a silent fallback
   changes what an arrow does); (d) the finder camera's view only; (e) tap a point of the
   imaging camera's frame to centre it (`ImagingSolve::sky_at`,
   `crates/finder/src/imaging/solver.rs:210`, then a GoTo with Track off or
   `Tracker::follow` with Track on), the field driver's own note that night. (e) is a
   route, a click on the Imaging page's frame and their tests, about 9 KB, a story of its
   own on 054's Imaging page; the owner's logged words ask for buttons, so it is
   recommended as a follow-up spec, not built here.
2. **The step sizes** (FR-002). Recommended (a): **fine 30″ and coarse 4′ in every view**:
   fine is about 55 pixels of the imaging camera (0.5475″ a pixel) and the field night's
   35″ GoTo error is one press; coarse is about 438 pixels of the imaging camera and 53
   of the finder camera (4.55″ a pixel), and the field night's 6.2′ to 6.6′ offset is one
   coarse press and about five fine ones. One pair of steps for every view is one thing
   to learn, and each button names its size. The draft's reason for the 4′ cap, staying
   under 038's 5′ push threshold, is gone with FR-011, so steps per view are open:
   (c) a tenth of the chosen view's frame (about 3′ for the imaging camera, about 7′ for
   the finder camera's measured 1.617° × 1.21° field). Instead also: (b) one step of 1′;
   (d) steps the owner sets on Setup.
3. **Press or hold** (FR-001). Recommended (a): **one press, one step**: the move's end
   is known before it starts, so the guards check its whole path, and a phone that loses
   a release cannot leave the telescope running. Instead: (b) hold to move at a rate and
   stop on release (a release watchdog and a rate rule of its own).
4. **Track and a nudge** (FR-010 to FR-012). Recommended (e′): **the place Track holds
   moves by the step and Track makes the step at once, each moved axis going 0.5° past
   its end against the way Track drives it and coming back**, so its gears end on the
   flank Track drives them on. Evidence (run, simulated time): every step each way at
   the Track rig, near transit and on a setting target came within 10″ of the stepped
   place 2.3 s to 7.0 s after the press and stayed within 11.8″ of it over the next
   20 s; no push, no slip, the epoch unchanged; with no solves within 20″ in 5 s.
   Instead: (a) the draft's, the step added to the rate loop's commands, which never
   reverse an axis (`commanded`, `crates/finder/src/tracking.rs:1540`): with the gears
   within 10″ in under 2 s, against them a coarse step waited for the sky, about 66 s
   down and 80 s left at the Track rig and without bound near transit (altitude moving
   0.55″/s there); (b) only the place moves and the correction loop brings the
   telescope from the live solves (a coarse step in 30 s; with no solves never);
   (c) a press turns Track off (the target is lost); (d) refuse a press while Track is
   on; (e) the seat's lead, Track makes the step with GoTo's approach from below on every
   moved axis: right on a rising target, but on a setting target, where Track drives
   altitude down, the approach left the gears on the up flank and Track's next drive took
   up the backlash: 529″ off, read as a push, a slip and two new epochs; (f) refuse a
   press against the gears, with words: half the arrows refused at any moment (down and
   left at the Track rig and near transit, up and left on the setting target).
5. **Whether a nudge becomes a sync** (FR-013). Recommended (a): **never**: a nudge
   writes nothing to the pointing model, opens no zero-point epoch and marks no slip, and
   the live solve's syncs go on as before (053's rules, whatever they become, are not
   touched). Evidence (run): after the presses the model's epoch was the one before them,
   `slip_pending` stayed false, and the live solve recorded new syncs (US1-S1, US3-S1).
   Instead: (b) mark a slip, as a push by hand does; (c) make the next solve a sync of its
   own (053's to decide).
6. **Guards** (FR-006 to FR-009). Recommended (a): **every guard of a direct axis move,
   every moved axis approaching from below with GoTo's words, the reservation and a
   refusal near the zenith**: the Sun guard, the night's sync, the minimum altitude (an
   end below it answers `the position is below the minimum altitude`, an approach below
   it `approach would go below the minimum altitude`), a path clear of the Sun guard, the
   minimum altitude and cable wind, no GoTo running, the mount at rest, `nudge` and
   `track` refused while a direct move runs, and a press refused at or above 88°, Track's
   keyhole altitude (`KEYHOLE_ALT`, `crates/finder/src/tracking.rs:38`). Evidence (run):
   above 88° a coarse right press kept 75″ of its 240″ at 89.5° and 11″ at 89.95°; below
   it, at the tube's true 84.85°, it kept within 14.3 % (US1-S5's bound 25 %). Instead:
   (b) the draft's approach on falling axes only (a first up press after Track, a push or
   a Stop takes up the backlash); (c) no zenith refusal.
7. **The page** (FR-017 to FR-019). Recommended (a): **one Nudge block in two places**:
   in the main page's guidance block under Track, which shows only with a session target
   (`crates/finder/web/src/components/GuidanceBlock.tsx:21`), opening on the mount's
   view; and on the Imaging page after the imaging camera's frame, with or without a
   target, opening on the imaging camera's view when the session has one. The Imaging
   page half builds on 054's rewritten page once this spec is re-anchored. Instead:
   (b) the guidance block only (the draft's; the imaging camera's arrows then sit on a
   page without its frame); (c) the Imaging page only; (d) Setup's mount section.
8. **MCP** (FR-014). Recommended (a): **a `nudge` tool taking the route's three
   arguments**, logged and rate-limited as every moving tool is. Instead: (b) no tool.
9. **When you press the right arrow while watching the imaging camera's frame, should
   the telescope turn right so the stars slide left, as a hand controller does, or should
   the stars slide right?** (FR-002). Recommended (a): **the telescope turns the way the
   arrow points and the stars slide the other way**, as hand controllers, mount panels and
   manual slews name their arrows; the logged words ("nudge the scope onto it"; "up and
   down are altitude") name the telescope's move. Instead: (b) the stars slide the way the
   arrow points. Answering (b) changes one angle in FR-002, φ to φ + 180°, and the
   scenarios' expected angles with it.
10. **Track turned on after nudges made with Track off** (FR-015). Recommended (c):
    **as today**: Track on GoTos the session's target first when the pointing is more
    than 1′ from it (015's FR-014), so it undoes Track-off nudges; the README says to
    centre with Track on. Evidence (run): two coarse right presses took the eyepiece
    258″ and then 119″ from M42 as the sky turned; Track on then GoTo'd back, 8.4″ by the
    GoTo's error, and held the eyepiece 5″ from M42 (US1-S4 bounds it within 30″). With
    Track off the sky moves about 15″ a second past the telescope, so where the arrows
    put it is a moment, not a place. Instead: (a) a Track-off press also moves the place a
    later Track on holds (a rule for which moment); (b) arrows only with Track on.

## Owner approvals

Flipping this spec to `ready` approves the answer to every owner decision above, and the
following. Each is named in the FR that makes it. No existing test, golden, fixture or CI
file changes.

1. **New public API in `crates/finder`'s library** (FR-001 to FR-012): the module
   `finder::nudge` with `FINE_ARCSEC`, `COARSE_ARCSEC`, `PUSH_QUIET_S`, `Direction`,
   `Step`, `View`, `NudgeAsk`, `NudgeView`, `Kept`, `offset`, `routes` and `nudge`; the
   `AxisMove` variants `Offset` and `Through`; `Tracker::nudge` and `Tracker::stepping`.
   No existing signature changes.
2. **A new route and tool** (FR-001, FR-014): `POST /api/mount/nudge` and the MCP tool
   `nudge`.
3. **015's page answer during a direct move** (FR-008): `POST /api/track` with `on` true
   while a direct axis move runs, a press's or an MCP client's, answers 409 `the mount is
   already moving`, on target or off. 016's Q10 reads: "The page's routes keep the
   answers 015 gives them (FR-009, FR-013)", and 015's FR-013 lists four conditions for
   Track on; this adds a fifth on the page's route.
4. **A page action in `motion`** (FR-007): 016's FR-014 names the page actions `goto`,
   `second_sync` and `track`; a nudge adds `nudge`.
5. **`axis_move` reports a Track-on step** (FR-010): 016's FR-022 reports the latest
   direct axis move; a Track-on nudge's step, which Track makes, is reported there too,
   with `tool` `nudge`, and is reserved as a direct move is.
6. **Track holds out the step's pointings** (FR-011): while a step is to be made or being
   made and for 1 s plus its settle after, Track's loop uses no live pointing and its
   `checks_push` is false, as 038's FR-001 reads that flag.
7. **The glossary** (FR-016): **Nudge** is added, and **Direct axis move** names the
   nudge.
8. **`crates/finder`'s README: its section, row and MCP paragraph** (FR-015). The pinned
   sentence `The imaging camera has sixteen tools of its own`
   (`crates/finder/README.md:88`) stays true: the tool is in a paragraph of its own.
9. **Two list entries in `crates/finder/web/playwright.config.ts`** (FR-020):
   `"**/nudge.spec.ts"` in the default project's `testIgnore` and in the simulate
   project's `testMatch`, each after `"**/tools-focus.spec.ts"`
   (`crates/finder/web/playwright.config.ts:45`,
   `crates/finder/web/playwright.config.ts:65`).
10. **The page** (FR-017 to FR-020): the guidance block's and the Imaging page's Nudge
    block; `ImagingPage` takes the session; the page's session copies `axis_move`.

### Other specs' requirements this spec changes

- 016's FR-014, which reads: "While a GoTo, a direct axis move or tracking runs, the
  session SHALL hold `motion`, with three fields: `by`: `page` or `mcp`; `tool`: the
  page action (`goto`, `second_sync`, `track`) or the MCP tool that started it;
  `since`: when it started." A nudge from the page holds `motion` with `by` `page` and
  `tool` `nudge` (FR-007).
- 016's FR-022, which reads: "The session SHALL report the latest direct axis move as
  `axis_move: {tool, state, reason?}` beside 014's `goto`". A Track-on step is reported
  there too (FR-010).
- 016's Q10 and 015's FR-013, quoted in owner approval 3 (FR-008).
- The glossary's **Direct axis move** (`GLOSSARY.md:273`), which reads: "Moving an axis
  to an axis position, or at a rate for a set time, asked for by axis rather than by
  target. Only MCP clients ask for one; it needs a sync and passes the same guards as a
  GoTo." FR-016 gives its new words.

No other spec's FR changes. 016's rule that over MCP a direct axis move is refused with
`Track is on` stands: a nudge with Track on is not one (FR-010). 038's FR-001 to FR-016
stand as written: they read `checks_push`, which FR-011 sets false for a while.

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
  with 0.3° of backlash and 1,620,000 counts a turn on a base tilted 1°, and its finder
  camera, whose frame-up is the zenith. **The synced rig at A** is that rig built at
  altitude A (50° unless said) after a fine `down` and a fine `left` press in the `mount`
  view, each read until it rests, so that both axes end on the flank a press leaves them
  on (Edge Cases: the first press after a sync). **Rests** means `GET /api/session` reads
  `axis_move.state` other than `moving` and `Rig::stationary` is true, read every 100 ms
  within 30 s, then 1 s more. **The turnaround** is the first 10 ms read, after the axes
  have moved, at which both rest while `axis_move.state` reads `moving`. **The tube's
  move** is the change of the simulator's `true_direction()`: its altitude change and
  its azimuth change times the cosine of the altitude before, in arcseconds. **Lands**
  within p % means that move is within p % of the step from the step's size at the
  press's angle clockwise of the zenith (up the altitude, right the azimuth).
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
- **The unit session** is a `SessionView` at 40° N 75° W whose latest pointing has
  `source` `live` and the `up_deg` the scenario gives, with the chart rotation it gives;
  **a solved imaging view** holds a latest solve of state `solved` at RA 83.82°, Dec
  −5.39° with the `cd_deg` it gives, S being one arcsecond a pixel: **unmirrored**
  `[[−S, 0], [0, −S]]` (one pixel right is west, one pixel down south) and **mirrored**
  `[[S, 0], [0, −S]]`. Those scenarios call `nudge::offset` in `crates/finder/src/nudge.rs`'s
  own tests for the value under test; every expected value comes from FR-004's and
  FR-005's words.
- **The Track rig at T** is the motion rig built at T (START unless said) with the
  simulator's eyepiece pointed at M42 before the sync, 1 s, Track turned on with
  `POST /api/track` `{"on": true}`, and 30 s of tracking, read `tracking`, as 038's push
  tests set it up. **Settled** means read every 100 ms, within 120 s, until the true
  place is within 10″ of the place Track holds. **The true place** is the simulator's
  eyepiece; **the stepped place** is the place Track holds just before the press, moved
  by the step's size at the press's angle clockwise of the zenith with the simulator's
  own horizon transform. **Read for N s** means read every 100 ms for N s: the first
  time the true place is within 10″ of the stepped place, the farthest it is from it
  over the 20 s after, whether any read's `track` has a `push` key and whether any reads
  `slip_pending` true.
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
- **Oracles.** The simulator's true tube direction, counts, eyepiece and imaging truth
  and its own horizon transform; the steps' sizes and the ways' angles; the mount's
  counts a turn (half a degree is 2,250 counts); the words of these scenarios and of
  014, 016 and 038; and DESIGN.md's night ink. No expected value comes from the code
  under test: no test reads `FINE_ARCSEC`, `COARSE_ARCSEC` or `PUSH_QUIET_S`.

### User Story 1 - With Track off, the arrows move the telescope one step in the mount's view (Priority: P1)

As the owner, I need one press of an arrow to move the telescope one small step on the
sky, up and down in altitude and left and right in azimuth, through every guard a GoTo
passes, each moved axis ending as a GoTo's does.

**Why this priority**: The step, the route, the approach and the guards are the
feature; the other stories add the camera views, Track, MCP and the page to them.

**Independent Test**: `crates/finder/tests/motion/nudge.rs`, new. The `test` gate's
nextest command runs it; during development,
`cargo nextest run -p finder --locked --test motion -E 'test(/^nudge::/)'`.

**Acceptance Scenarios**:

1. **Given** the synced rig, **When** a committed test presses coarse `right` in the `mount` view, reads `GET /api/session` at once, reads until it rests, then presses each step each way in the `mount` view, reading each until it rests, **Then** it asserts that the first answers 200 with exactly `{"up_arcsec": 0.0, "right_arcsec": 240.0, "moved": "axes"}` and the session read at once has `motion.by` `page` and `motion.tool` `nudge`; that every later press answers 200 with `moved` `axes` and rests with `axis_move.state` `done` and `axis_move.tool` `nudge`; that each lands within 3 %; that the model's `epoch` after the presses is the one before them; that `slip_pending` is false; and that the model holds a sync whose time is after the moment those presses began.
2. **Given** the synced rig, **When** a committed test presses fine `up`, `right`, `down` and `left` in the `mount` view and reads the moved axis's counts (altitude for up and down, azimuth for right and left) every 100 ms until it rests, **Then** it asserts that for each press the axis's end is at least 2,240 counts above the lowest count read, and that the end is above the start for `up` and `right` and below it for `down` and `left`.
3. **Given** the bare console, and then the synced rig, **When** a committed test presses fine `up` in the `mount` view on the bare console; then on the rig sends a foreign write, a press as `text/plain`, a press with the extra key `"x": 1` and a press with the direction `north`; presses fine `up` twice one after the other and reads until it rests; presses fine `down` and at the turnaround fine `up`, and reads until it rests; sets `PUT /api/session` `{"min_alt_deg": a − 0.02}`, a the session's `pointing.alt_deg`, and presses coarse `down`, then sets `{"min_alt_deg": a − 0.2}` and presses coarse `down` again, then coarse `right`, and reads until it rests; and stops the node's frames, sends `POST /api/model/reset` `{"what": "night"}` and presses fine `up`, **Then** it asserts that the bare console answers 409 with exactly `{"error": "no mount is connected"}`; that the foreign write answers 403 with exactly `{"error": "writes require the console's own origin and an allowed host"}`; that `text/plain` answers 415, the extra key 400 and `north` 400; that of each pair of presses the first answers 200 and the second, the turnaround's among them, 409 with exactly `{"error": "the mount is already moving"}`; that the first coarse `down` answers 409 with exactly `{"error": "the position is below the minimum altitude"}`, the second with exactly `{"error": "approach would go below the minimum altitude"}` and coarse `right` 200; and that the last press answers 409 with exactly `{"error": "the night has no sync yet"}`.
4. **Given** the motion rig with the simulator's eyepiece pointed at M42 before its sync and the target M42, **When** a committed test presses coarse `down` in the `mount` view and 100 ms later sends `POST /api/track` `{"on": true}`; reads until it rests; presses fine `down` and at the turnaround sends `POST /api/track` `{"on": true}`; reads until it rests; then sends `POST /api/track` `{"on": true}` and reads every 100 ms, within 120 s, until `track.state` is `tracking`, **Then** it asserts that both presses answer 200 and both Track requests during them 409 with exactly `{"error": "the mount is already moving"}`; that the moves rest with `axis_move.state` `done` while `track` is absent from the session or reads `off`; that the eyepiece is then more than 240″ from M42; and that the last Track request answers 200, `track.state` reads `tracking`, and the eyepiece is within 30″ of M42.
5. **Given** the synced rig at 86°, whose tube points about 1.15° lower on the simulator's tilted base, and then the motion rig at 89.5°, synced, **When** a committed test presses coarse `right` and then coarse `up` in the `mount` view on the first, reading each until it rests, and fine `up` on the second, **Then** it asserts that coarse `right` lands within 25 % and coarse `up` within 4 %, and that the last answers 409 with exactly `{"error": "the telescope is too near the zenith: nudges stop at 88° altitude"}`.

### User Story 2 - The arrows follow the finder camera's or the imaging camera's frame (Priority: P1)

As the owner, I need the arrows to follow the finder camera's live views as the page
draws them, or the imaging camera's own up and right from its last solved imaging solve,
so that a press moves the telescope the way the arrow points in the picture I watch.

**Why this priority**: "Buttons that read the frame's own up and right" was the field
night's wish; without this story the arrows read only the mount's axes.

**Independent Test**: `crates/finder/tests/motion/nudge_views.rs`, new, and the tests
module of `crates/finder/src/nudge.rs`. The `test` gate's nextest command runs both;
during development,
`cargo nextest run -p finder --locked --lib --test motion -E 'test(/^nudge_views::/) | test(/^nudge::tests::/)'`.

**Acceptance Scenarios**:

1. **Given** the bare console, and then the synced rig with `PUT /api/session` `{"chart_rotation_deg": 200}`, **When** a committed test presses fine `up` in the `finder_camera` view on the bare console, then on the rig presses each step each way in the `finder_camera` view, reading each until it rests, **Then** it asserts that the bare console answers 409 with exactly `{"error": "the finder camera has no live pointing yet"}`, and that each press on the rig answers 200 with `moved` `axes` and lands within 3 % at φ + 200° clockwise of the zenith (the simulator's live pointing reads `up_deg` near 0°, so this proves the chart rotation only).
2. **Given** the unit session with `up_deg` 40° at chart rotations 0° and 200°, **When** a committed unit test asks `offset` for a coarse press each way in the `finder_camera` view, **Then** it asserts that each answer is `[240 cos a, 240 sin a]` within 10⁻⁹ as `[up, right]`, a = φ + R − 40° with R the chart rotation.
3. **Given** the bare console, and then the imaging rig at an imaging angle of 100° once `GET /api/imaging` reads `state` `ready` and the model holds a sync, **When** a committed test presses fine `up` in the `imaging_camera` view on the bare console and on the rig; presses fine `down` and then fine `left` in the `mount` view, reading each until `axis_move.state` is `done`; takes an imaging frame with `POST /api/imaging/frame`, reads it until it is saved, solves it with `POST /api/imaging/solve` and reads until its solve is `solved`; and then presses each step each way in the `imaging_camera` view, reading each until `axis_move.state` is `done` and 1 s more, **Then** it asserts that both first presses answer 409 with exactly `{"error": "the imaging camera has not solved a frame yet"}`; and that after each later press, which answers 200, the frame's centre has moved from where it would be by the step's size within 4 % of the step, at the position angle east of north of the simulator's `up_east_of_north_deg` less φ, the way an unmirrored frame's arrow points.
4. **Given** solved imaging views, one unmirrored and one mirrored, and an unmirrored one whose solve was asked for at T while the imaging view still holds its frame, of the same id, begun at T − 1,200 s with a 2 s exposure, **When** a committed unit test asks `offset` for a coarse press each way in the `imaging_camera` view of each, **Then** it asserts that the mirrored frame's `up` and `down` equal the unmirrored frame's and its `right` and `left` equal the unmirrored frame's `left` and `right`; and that the late solve's answers equal those of a solve asked for at T − 1,199 s and differ by more than 1° from those of a solve asked for at T.
5. **Given** a solved imaging view at T, **When** a committed unit test asks `offset` for a coarse `right` in the `imaging_camera` view with it, then with later solves at T + 60 s reading `solving` and `failed` and the same kept frame, then with nothing kept for a solve reading `solving`, one reading `failed` and no imaging view, and then with a session without a site, **Then** it asserts that both later solves answer what the solved one did; that the three answer exactly `the imaging camera's solve is still running`, `the imaging camera's latest frame did not solve` and `the imaging camera has not solved a frame yet`; and that the last answers exactly `a site is needed`.

### User Story 3 - With Track on, the place Track holds moves by the step and Track makes it (Priority: P1)

As the owner, I need a press with Track on to move the place Track holds and to bring the
telescope there at once, with the gears ending the way Track drives them, and never to be
read as a push by hand.

**Why this priority**: The field night's centring was with Track on; without this story
a press with Track on is refused.

**Independent Test**: `crates/finder/tests/motion/nudge_track.rs`, new. The `test` gate's
nextest command runs it; during development,
`cargo nextest run -p finder --locked --test motion -E 'test(/^nudge_track::/) | test(/^tracking::/)'`.

**Acceptance Scenarios**:

1. **Given** the Track rig, **When** a committed test, each time settled, presses each step each way in the `mount` view, reads `GET /api/session` at once and reads for 30 s, **Then** it asserts that each press answers 200 with `moved` `track`; that the session read at once has `axis_move` exactly `{"state": "moving", "tool": "nudge"}`; that the true place comes within 10″ of the stepped place within 10 s and stays within 30″ of it over the next 20 s; that no read has a `push` key or `slip_pending` true; that `axis_move.state` then reads `done` and `track.state` `tracking`; and that the model's `epoch` after the presses is the one before them.
2. **Given** the Track rig at START + 4 h, where M42 has set west of the meridian and Track drives altitude down, **When** a committed test, each time settled, presses coarse `up` and then coarse `down` in the `mount` view and reads each for 30 s, **Then** it asserts the same as US3-S1 for each press.
3. **Given** the Track rig, settled, **When** a committed test presses coarse `left` in the `mount` view, presses coarse `left` again 1 s later and reads for 9 s more, **Then** it asserts that the first answers 200 and the second 409 with exactly `{"error": "the mount is already moving"}`; that no read has a `push` key or `slip_pending` true; that 10 s after the first press the true place is within 10″ of the stepped place of that press; and that the model's `epoch` is the one before.
4. **Given** the Track rig, settled, once with the simulator's mount then pushed by hand 1.5′ down (`push_by(0.0, −1.5/60)`) and once on a new Track rig pushed 1.5′ up, **When** a committed test presses coarse `up` after the push down and coarse `down` after the push up, each in the `mount` view, and reads each for 45 s, **Then** it asserts that each press answers 200; that the true place comes within 10″ of the stepped place; that no read has a `push` key or `slip_pending` true; and that the model's `epoch` is the one before.
5. **Given** the Track rig, settled, with the node's frames then stopped, **When** a committed test presses coarse `right` in the `mount` view and reads the true place every 100 ms for 5 s, **Then** it asserts that the press answers 200 and that the true place came within 20″ of the stepped place.
6. **Given** the Track rig with the simulator's mount pushed down by hand by 2° (`push_by(0.0, −2.0)`), and then the motion rig, synced, with the target M42 elsewhere and Track turned on, reading `armed`, **When** a committed test presses fine `up` in the `mount` view the first time `track.push.state` reads `moving`, again the first time it reads `following`, once more 10 s after that, and then once on the second rig, **Then** it asserts that the first two answer 409 with exactly `{"error": "a push by hand is still settling: press again in a few seconds"}`; that the third answers 200 with `moved` `track`; and that the last answers 409 with exactly `{"error": "Track is not tracking yet"}`.

### User Story 4 - MCP and the README carry the nudge (Priority: P2)

As an MCP client, I need the page's nudge as a tool with the route's answers, and as the
owner, I need the README and the glossary to say what a nudge is.

**Why this priority**: The field night's planet was moved over MCP; the route alone
carries the feature.

**Independent Test**: `crates/finder/tests/mcp/nudge.rs`, new. The `test` gate's
nextest command runs it; during development,
`cargo nextest run -p finder --locked --test mcp -E 'test(/^nudge::/)'`.

**Acceptance Scenarios**:

1. **Given** the MCP rig with the call log, **When** a committed test reads `tools/list`, calls `nudge` with `{"direction": "right", "step": "coarse", "view": "mount"}`, reads `GET /api/session` at once, calls `set_tracking` with `{"on": true}`, and reads the session until `axis_move.state` is not `moving`, **Then** it asserts that `nudge` is listed with a description containing "real telescope", an `outputSchema` of type `object` and `inputSchema` properties `direction`, `step` and `view` each of type `string`; that the call's `structuredContent` is exactly `{"up_arcsec": 0.0, "right_arcsec": 240.0, "moved": "axes"}`; that the session read at once has `motion.by` `mcp` and `motion.tool` `nudge`; that `set_tracking` answers `isError` true with the text `a direct axis move is running`; that the mount's azimuth counts grew and its altitude counts did not change; and that the call log has a line containing `mcp nudge` and no line containing `test-token`.
2. **Given** the MCP rig before its sync, then synced, **When** a committed test calls `nudge` with fine `up` in the `mount` view; after the sync calls it with `{"direction": "north", "step": "fine", "view": "mount"}`, with no `view`, and with the extra key `"x": 1`; waits 1.1 s; calls it three times one after another with coarse `up` in the `mount` view; reads until the move ends and 1 s more; and calls it with fine `up` in the `imaging_camera` view, **Then** it asserts that the first answers `isError` true with the text `the night has no sync yet`; that the three malformed calls answer `isError` true with exactly `nudge takes direction (up, right, down or left), step (fine or coarse) and view (mount, finder_camera or imaging_camera)`; that of the three the first is not an error, the second answers `the mount is already moving` and the third `the rate limit of 2 moving calls a second was reached`; and that the last answers `the imaging camera has not solved a frame yet`.
3. **Given** the committed `crates/finder/README.md` and `GLOSSARY.md`, **When** a committed test reads them, **Then** it asserts that the README still contains `The imaging camera has sixteen tools of its own`; that `The page's nudge arrows have one tool:` lies after `The Tools page's focus aid for the finder camera has two tools:` and before `### MCP under WSL2`, with a line starting ``- `nudge` takes `direction` `` between them; that the `## API` section has a row starting ``| `POST /api/mount/nudge` |``; that `## Nudging the telescope` lies after `## Drawn sky on the real mount` and before `## Recording a night` and contains `fine is 30″, coarse is 4′`, `the finder camera has no live pointing yet`, `is never read as a push and never a sync` and `Track on always does: centre with Track on`; and that the glossary holds the entries **Direct axis move** and **Nudge** whole, with FR-016's words and the nudge's `_Avoid_` line.

### User Story 5 - The Nudge block in the guidance block and on the Imaging page (Priority: P2)

As the owner at the telescope with a phone, I need four arrows under Track and beside
the imaging camera's frame, a choice of the view they follow and of the step, words while
a step is still being made, and the console's words when a press is refused.

**Why this priority**: The owner asked for buttons; the tool and the route are not
buttons.

**Independent Test**: `crates/finder/web/src/components/NudgeBlock.test.tsx` and
`crates/finder/web/e2e/nudge.spec.ts`, new. The `test` gate's `npm test` and
`CI=1 npm run e2e` run them; during development, `npm test -- NudgeBlock` and
`npx playwright test --project=simulate e2e/nudge.spec.ts` in `crates/finder/web`.

**Acceptance Scenarios**:

1. **Given** the page rig with `fetch` answering `{"up_arcsec": 0, "right_arcsec": 240, "moved": "axes"}`, **When** a committed test chooses `finder_camera` in the Nudge block's `Arrows follow` box, clicks `Coarse 4′` and then `Nudge right`, and re-renders the block from the same session with `axis_move` `{"state": "moving", "tool": "nudge"}` and then `{"state": "done", "tool": "nudge"}`, **Then** it asserts that `Coarse 4′` is pressed (`aria-pressed` true); that `fetch` was called with `/api/mount/nudge`, method `POST`, the header `Content-Type: application/json` and the body `{"direction":"right","step":"coarse","view":"finder_camera"}`; that `onChange` was called once; that the block's status reads exactly `Nudging right 4′…` after the answer and after the `moving` session; and that it reads exactly `Nudged right 4′` after the `done` one.
2. **Given** the page rig with `fetch` answering 409 with `{"error": "the night has no sync yet"}`, **When** a committed test clicks `Nudge up`, **Then** it asserts that the Nudge block's alert reads exactly `the night has no sync yet`.
3. **Given** the page rig's session without `imaging_camera`, then with it, then without `mount`, **When** a committed test renders each, **Then** it asserts that the `Arrows follow` box offers exactly `the mount` and `the finder camera`, then `the mount`, `the finder camera` and `the imaging camera`, and that without a mount there is no Nudge block.
4. **Given** `ImagingPage` rendered by Testing Library from the recorded session `tracking` without its `target`, with `fetch` answering a `ready` imaging view of the simulated imaging camera, **When** a committed test finds the region `Imaging frame`, **Then** it asserts that a region named `Nudge` follows it in the document and that its `Arrows follow` box reads `imaging_camera`.
5. **Given** the simulated console with Track off and the page open by day, **When** a committed browser test reads the `Arrows follow` box, clicks `Nudge right`, reads the session until `axis_move.state` is `done` and `mount.moving` false, clicks `Coarse 4′`, clicks `Nudge down` and at once `Nudge up`, and reads until it rests, **Then** it asserts that the box reads `mount`; that the first press's response is 200 with exactly `{"up_arcsec": 0, "right_arcsec": 30, "moved": "axes"}` and the block's status reads `Nudged right 30″`; that `Nudge down` answers exactly `{"up_arcsec": -240, "right_arcsec": 0, "moved": "axes"}`; and that `Nudge up` answers 409 and the block's alert reads `the mount is already moving`.
6. **Given** the simulated console with Track turned on with the guidance block's Track switch and read until `track.state` is `tracking`, **When** a committed browser test clicks `Coarse 4′` and `Nudge up` and reads the session every 500 ms for 10 s, **Then** it asserts that the press's response is 200 with exactly `{"up_arcsec": 240, "right_arcsec": 0, "moved": "track"}`; that the block's status comes to read `Nudged up 4′ · Track holds the new place` within 20 s; and that every read has `track.state` `tracking` and no `push`.
7. **Given** the simulated console with the page open by day, **When** a committed browser test screenshots the Nudge block, turns on `Night mode: red and black`, reads `data-theme` `night` on `html`, waits 1.5 s and screenshots the block again, **Then** it asserts with 012's off-red count (a pixel whose red is above 226 or whose green or blue is above 0) that the day screenshot has more than 1,000 off-red pixels and the night one none.

### Edge Cases

- **A press during a GoTo, a direct axis move or another press.** Refused with `a GoTo is
  already running` or `the mount is already moving`, the approach's turnaround included
  (US1-S3); Track turned on from the page during a press is refused with `the mount is
  already moving` (US1-S4), and from MCP with `a direct axis move is running` (US4-S1).
  With Track on, a press while Track still makes the last step is refused the same way
  (US3-S3).
- **The first press after a sync taken on the other flank.** A sync made while an axis's
  gears rest on its falling flank reads that axis's counts a backlash away from where a
  press leaves them; the first press up or right on it takes up the backlash once (on a
  fresh simulated rig a fine `up` moved the tube 510″ the wrong way) and every later
  press lands (US1-S1). A GoTo has the same one-time take-up; which flank a sync is read
  on is 053's. The synced rig presses down and left first for this reason.
- **Stop.** The top bar's Stop and MCP's `stop` end a press's move as they end any direct
  axis move (016's FR-012); `axis_move` then reads `halted` with `the observer stopped
  it`, and the block's status reads `Nudge stopped: ` and that reason. A Track-on step
  ends when Track does, `halted` with Track's reason. Not run for a nudge.
- **Near the minimum altitude.** A press whose end or approach would go below it is
  refused (US1-S3). A telescope already below it may not go lower (the floor `move_axis`
  uses, `crates/finder/src/goto.rs:200`), and since every moved axis approaches from
  0.5° below, an altitude press there is refused too: such a telescope climbs with a
  GoTo, not by nudges. Not run for the low start.
- **Across the Sun guard or the cable's turn.** Refused with `no path keeps clear of the
  Sun guard, minimum altitude and cable wind` (014's words). Not run for a nudge.
- **Near the zenith.** Refused at or above 88° altitude (US1-S5). Just below it a coarse
  `right` keeps less of its size as the azimuth turn grows (run: 10.1 % short at the
  tube's true 82.86°, 14.3 % at 84.85°, 29.6 % at 87.35°); `up` keeps it within 3.5 %
  up to 84.85°.
- **The imaging camera's view after a solve that did not solve.** The last solved
  frame's orientation holds until another solves (US2-S5). A camera turned in its holder
  after a solve gives arrows turned with it until the next solve that solves.
- **A press during an imaging run or a live stack, with Track on.** The step moves the
  frame for every later sub. 040's steady mark holds subs while a step is to be made or
  being made and through Track's settle after it, so subs of the step are not stacked.
  Not run.
- **Track turned on after nudges made with Track off.** Track GoTos the session's target
  first and holds it, undoing the nudges (US1-S4, owner decision 10): centre with Track
  on.
- **Track armed, in its keyhole wait or measuring backlash.** A press with Track wanted
  but not `tracking` is refused with `Track is not tracking yet`: armed is run (US3-S6);
  the keyhole wait and the backlash measure are read, not run.
- **A step from a Track error.** A step taken while Track is still 1.5′ off is not read
  as a push either way (US3-S4).

## Requirements *(mandatory)*

### Functional Requirements

**The route and the step** (US1):

- **FR-001**: The console SHALL serve `POST /api/mount/nudge`, a same-origin write as
  `POST /api/track` is, taking a JSON body of exactly `direction` (`up`, `right`,
  `down` or `left`), `step` (`fine` or `coarse`) and `view` (`mount`, and from US2
  `finder_camera` and `imaging_camera`), refusing any other key or value with 400 and
  any other content type with 415. One request is one step: a press. It SHALL answer 200
  with `{"up_arcsec": U, "right_arcsec": R, "moved": M}`, U and R the step in the
  mount's view (up the altitude, right a clockwise turn) and M `axes` with Track off or
  `track` with Track on, or 409 with a JSON `error` naming the refusal.
- **FR-002**: `fine` SHALL be 30″ and `coarse` 240″ on the sky (owner decision 2). The
  telescope's aim SHALL move the step's size at φ clockwise of the chosen view's up, φ
  0°, 90°, 180° and 270° for `up`, `right`, `down` and `left`, so the stars in that view
  slide the other way (owner decision 9 (a); its (b) is φ + 180° here).
- **FR-003**: In the `mount` view, `up` SHALL be higher altitude and `right` a clockwise
  turn, and the answer's U and R SHALL be exactly the step and 0 (or 0 and the step,
  signed).

**The camera views** (US2):

- **FR-004**: In the `finder_camera` view, the arrows SHALL follow the finder camera's
  live views as the page draws them, turned counter-clockwise by the session's chart
  rotation R: the press's angle clockwise of the zenith is φ + R − u, u the latest live
  pointing's `up_deg`. With no live pointing it SHALL answer 409 with `the finder camera
  has no live pointing yet`.
- **FR-005**: In the `imaging_camera` view, the arrows SHALL follow the imaging camera's
  frame as the last imaging solve that solved gives it: the solve's `cd_deg`, whose
  determinant's sign is the frame's handedness (a mirrored frame's is negative, and its
  right is the unmirrored frame's left), its RA and Dec, the frame's mid-exposure time
  (its start plus half its exposure, while the imaging view still holds the solved frame;
  else the moment the solve was asked for) and the session's site, turned into altitude
  and azimuth with the parallactic angle at that place and time; up is towards row 0 and
  right towards the last column. The console SHALL keep that orientation until another
  imaging solve solves: a later solve still running or not solved leaves it in use. With
  none kept it SHALL answer 409 with `the imaging camera's solve is still running`, `the
  imaging camera's latest frame did not solve` or `the imaging camera has not solved a
  frame yet`, as the latest solve reads `solving`, `failed` or nothing; and without a
  site `a site is needed`.

**Track off** (US1):

- **FR-006**: With Track off, a press SHALL be a direct axis move of both axes started
  through the path of MCP's direct axis moves (`api::start_motion` and
  `GoTo::check_axis_move`), so that it passes their every guard and refusal: no mount,
  `Sun is up`, `the night has no sync yet`, `a GoTo is already running`, `the mount is
  already moving` and `no path keeps clear of the Sun guard, minimum altitude and cable
  wind`. Its end SHALL be the model's inverse at the altitude and azimuth the step gives
  from the model's prediction at the axes' counts, added to the counts as a difference.
  Every axis whose counts the step changes SHALL first go to 0.5° below its end and then
  come up to it, as a GoTo approaches every axis (014). A waypoint below the floor, the
  lower of the minimum altitude and the predicted altitude at the press, SHALL be refused
  with 014's words: the end with `the position is below the minimum altitude`, the
  approach with `approach would go below the minimum altitude`.
- **FR-007**: While a press's move runs, the session SHALL hold `motion` with `by`
  `page` (from the route) or `mcp` (from the tool) and `tool` `nudge`, and SHALL report
  it as `axis_move` with `tool` `nudge` and `state` `moving`, then `done` or `halted`
  with its reason (016's FR-014 and FR-022).
- **FR-008**: While a direct axis move runs, `api::start_motion` SHALL refuse the tools
  `nudge` and `track`, beside those it refuses at f6d4f5fd (`goto`, `second_sync`,
  `goto_target`, `goto_axis_position`, `move_axis`), with 409 `the mount is already
  moving`: a second press, at the approach's turnaround too, and Track turned on from the
  page during a press or an MCP direct move, on target or off (owner approval 3).
- **FR-009**: A press SHALL answer 409 with `the telescope is too near the zenith: nudges
  stop at 88° altitude` when the model predicts the telescope at or above 88° altitude,
  Track's keyhole altitude (owner decision 6).

**Track on** (US3):

- **FR-010**: With Track on and `tracking`, a press SHALL call `Tracker::nudge` with the
  step and the session's model: the place Track holds SHALL move by the step (east and
  north from the step's up and right with the parallactic angle at the moment), the
  correction's memory SHALL be cleared as `follow` clears it, and Track's next turn
  SHALL stop the axes and make the step. Its end SHALL be the counts plus the model's
  inverse at the moved place on arrival less that at the held place now; every axis the
  step moves SHALL first go 0.5° past its end against the way Track drives that axis and
  then come back to it (from below while Track drives it up or holds it, from above while
  Track drives it down), so its gears end on the flank Track drives them on; the path
  SHALL first pass a GoTo's guards (FR-006's refusals). Track SHALL stay `tracking`. The
  step SHALL be reported as `axis_move` with `tool` `nudge`, `moving` from the press and
  `done` once made, or `halted` with Track's reason if Track ends first, and reserved as
  a direct move is (FR-008). A press with Track wanted but not `tracking`, or while a
  backlash is measured, SHALL answer 409 with `Track is not tracking yet`.
- **FR-011**: While a press's step is to be made or being made, and for 1 s plus Track's
  settle (`SETTLE`, `crates/finder/src/tracking.rs:31`) after it is made, the tracker's
  `checks_push` SHALL be false and no live pointing SHALL feed Track's loop, so 038's
  push check reads none of the step's frames: `track` gains no `push` key,
  `slip_pending` stays false and the epoch stays.
- **FR-012**: With Track on, a press SHALL answer 409 with `the mount is already moving`
  while the last press's step is still to be made or being made, and with `a push by
  hand is still settling: press again in a few seconds` while `track.push.state` is
  `moving`, or is `following` and less than 10 s from the follow (`PUSH_QUIET_S`, a copy
  of 038's `PUSH_REOPEN_S` at `crates/finder/src/session.rs:113`, whose doc line names
  the pair).

**Never a sync** (US1, US3):

- **FR-013**: A press SHALL write nothing to the pointing model: its zero-point epoch
  and `slip_pending` stay as they were, and the live solve's syncs go on as before
  (owner decision 5).

**MCP and the words** (US4):

- **FR-014**: The MCP endpoint SHALL offer `nudge`, taking `direction`, `step` and
  `view` and nothing else as the route does, through the same function, rate-limited
  and written to the call log as every moving tool is, answering the route's view as
  `structuredContent` or its refusal as a tool error, and a malformed call with
  `nudge takes direction (up, right, down or left), step (fine or coarse) and view
  (mount, finder_camera or imaging_camera)`. Its description SHALL say it moves a
  "real telescope". With Track on it is not refused with `Track is on`: it moves
  Track's place and Track makes the step (FR-010).
- **FR-015**: `crates/finder/README.md` SHALL gain the paragraph `The page's nudge
  arrows have one tool:` with its `nudge` entry after the finder camera's focus aid's
  tools and before `### MCP under WSL2`; the `## API` row of `POST /api/mount/nudge`
  after `POST /api/goto`'s; and the section `## Nudging the telescope` after `## Drawn
  sky on the real mount`, with the words of plan.md § Interfaces, which say that Track
  turned on after nudges made with Track off goes back to the target (owner decision
  10).
- **FR-016**: `GLOSSARY.md` SHALL define **Nudge**: "One press of a nudge arrow: the
  telescope moves one step on the sky, fine 30″ or coarse 4′, up, right, down or left
  in the view the arrows follow: the mount's own (up is higher altitude, right a
  clockwise turn), the finder camera's live views as the page turns them, or the
  imaging camera's frame from its latest solved imaging solve. With Track off it is a
  direct axis move; with Track on the place Track holds moves by the step and Track
  makes it. A nudge is never a sync, never a slip and never read as a push." with
  _Avoid_: jog, slew, manual move; and **Direct axis move** SHALL read: "Moving an axis
  to an axis position, or at a rate for a set time, asked for by axis rather than by
  target, or both axes by one nudge's step. MCP clients ask for the first two; a nudge
  with Track off is the third. Each needs a sync and passes the same guards as a GoTo."

**The page** (US5):

- **FR-017**: The guidance block, which shows with a session target, SHALL show under
  Track, while the mount is connected, a region named `Nudge` holding a box `Arrows
  follow` (`the mount`, `the finder camera`, and `the imaging camera` when the session
  has `imaging_camera`, the imaging camera's sensor geometry; `the mount` first), two
  buttons `Fine 30″` and `Coarse 4′` of which the chosen one is pressed (`Fine 30″`
  first), and four buttons `Nudge up`, `Nudge right`, `Nudge down` and `Nudge left`
  drawn as arrows in a cross, at least 44 CSS pixels each. A click SHALL post the press
  and disable the arrows until it answers; the status SHALL read `Nudging <way>…` until
  a later session reads the press's `axis_move` (`tool` `nudge`) other than `moving`,
  then `Nudged <way>`, with ` · Track holds the new place` when `moved` is `track`, or
  `Nudge stopped: <reason>` when `halted`; `<way>` is the direction and `30″` or `4′`.
  It SHALL refresh the session, and a refusal SHALL show its `error` in the block's
  alert.
- **FR-018**: The Imaging page SHALL show the same Nudge block after its imaging frame,
  with or without a session target, its `Arrows follow` box first on `the imaging
  camera` when the session has `imaging_camera`. `ImagingPage` SHALL take the session
  and its refresh from `App`, and the page's session SHALL copy `axis_move`.
- **FR-019**: The Nudge block SHALL follow the red-only night rule.
- **FR-020**: `playwright.config.ts` SHALL gain only owner approval 9's two entries, and
  `crates/finder/web/dist` SHALL be the build of its source.

### Key Entities

- **Nudge**: one press; a direction, a step and a view; answered with the step in the
  mount's view and what moved.
- **The view the arrows follow**: `mount`, `finder_camera` or `imaging_camera`; the
  last two read the latest live pointing or the kept imaging frame.
- **The kept imaging frame**: the frame's axes on altitude and azimuth from the last
  imaging solve that solved, with its frame's id.
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
  the finder camera's views (US1-S1, US2-S1) and within 4 % in the imaging camera's
  (US2-S3); with Track on the telescope is within 10″ of the stepped place within 10 s
  every way, rising or setting (US3-S1, US3-S2).
- **SC-004**: No press is a sync, a slip or read as a push (US1-S1, US3-S1 to US3-S4),
  every guard's refusal is the route's (US1-S3, US1-S5, US3-S6, US4-S2), and the page
  shows the console's own words (US5-S2, US5-S5).

## Assumptions

- **Earlier features are in place.** Every spec in `depends_on_landed` has landed. Every
  citation here was read at f6d4f5fd.
- **The simulator stands for the mount and cameras** in the gates: its mount's backlash,
  counts, rates and tilted base, its finder camera with the zenith at frame-up, and its
  imaging camera at the angle a test sets. The finder camera's roll is proven by a unit
  test (US2-S2), as the simulator cannot roll that camera.
- **The mount is alt-az.** Nothing here is equatorial; the future equatorial mount is
  not designed for (the step on the sky and the views carry over; the axes' arithmetic
  would not).
- **Out of scope:**
  - equatorial directions and a hand controller;
  - tapping a point of the imaging frame to centre it (owner decision 1 (e)), a
    follow-up spec;
  - solar-system targets;
  - 053's sync rules, GoTo's correction solve, the model and the live solve;
  - remembering the chosen view or step across page loads;
  - a press held to keep moving (owner decision 3 (b));
  - the field night's centring by the owner (plan.md § Outside the factory).
- **The factory's gates and CI:** the gates are `lint` and `test`, copied in plan.md
  § Verification. CI's other jobs run only in the merge queue, so no scenario's Then
  clause rests on them.

## Work Graph

```yaml
US1:
  depends_on: []
  implements: [FR-001, FR-002, FR-003, FR-006, FR-007, FR-008, FR-009, FR-013]
US2:
  depends_on: []
  depends_on_merged: [US1]
  implements: [FR-004, FR-005]
US3:
  depends_on: []
  depends_on_merged: [US2]
  implements: [FR-010, FR-011, FR-012]
US4:
  depends_on: []
  depends_on_merged: [US3]
  implements: [FR-014, FR-015, FR-016]
US5:
  depends_on: []
  depends_on_merged: [US4]
  implements: [FR-017, FR-018, FR-019, FR-020]
```

Five stories, each under 60 % of 65,536 B (39,321 B) at the factory's worst overrun seen,
1.35 times the probe's measure (plan.md § Plan traps 1): the mount's view with Track
off, the approach, the reservation and the zenith; the two camera views; Track's step;
MCP, the README and the glossary; the page. The cut: the draft's US1 gave up its finder
camera view and US2 its imaging camera view and Track, so each view and Track is a story
of its own, and US2's and US3's tests use US1's test helpers. Each merges after the one
before (`depends_on_merged`): US2 adds views to US1's `nudge` module; US3 adds Track's
branch beside them; US4's tool calls `nudge` with every view; the page posts the route and
its browser test needs Track's branch. Plain `depends_on` would order them without
putting the earlier code in the later story's base.
