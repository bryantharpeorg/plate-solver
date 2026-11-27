---
state: draft
depends_on_landed: ["013-live-finder", "015-tracking", "016-mcp", "031-imaging-frames-on-console", "033-focus-reading-and-aid", "035-live-stack", "037-imaging-run", "041-imaging-run-on-the-rig", "042-imaging-run-page-and-mcp", "043-imaging-abort-answers-in-time", "050-tools-page-aligns-finder-camera", "051-tools-page-focuses-either-camera"]
# Drafted 2026-11-27 against `main` at f6d4f5fd, at the owner's ask of 2026-11-27
# ("Your rec. Proceed", the owner's answer to the operator seat's ranked list of the
# defects of the field night of 2026-11-26), from a probe of that day on the branch
# `probe/054` cut from f6d4f5fd. Cynical review r1 (0 blockers, 6 majors, 10
# minors) folded the same day, with new probe commits on that branch (tip 965ec262,
# every gate passing; plan.md § What was run and what was read). Every `path:line`
# here was read at f6d4f5fd. Not yet probe-built from this text, and none of the
# five owner decisions answered: each carries its recommendation, and the scenarios
# and the record are written to the recommendations. 013 is named for the simulator
# and the live loop the rigs run on; 015 because a reading's altitude is Track's
# target's; 016 for MCP; 031 for the imaging camera's lease and the Imaging page; 033
# for the focus aid, its put-back, record and verdict and the Focus block; 037 and
# 041 for the imaging run, its focus gate and pause and its region refusal; 043
# because its silent-node bound must hold; 035, 042, 050 and 051 because each last
# changed a file this spec writes (plan.md § Refinement Record).
---

# Feature Specification: The Focus Pause Gives the Camera Back

**Feature Branch**: `054-focus-pause-gives-camera-back`

**Created**: 2026-11-27

**Status**: Draft

**Input**: On the field night of 2026-11-26 the owner's first imaging run paused at its
focus check, "Focus is clearly out: stars are 6.7 arcsec across against a record of 4.6
arcsec", the record read on the Pleiades at an altitude of about 74° and the target,
M42, at about 37°. `Go anyway` (`POST /api/imaging/run/continue` with
`{"anyway":true}`) then ended the run at once, `camera_failed`, with "the imaging camera
has a region of interest set: clear it before an imaging run." It happened twice. The
owner asks that going on from a focus pause go on, with the imaging camera whole; that
a target low in the sky not be called out of focus for the air alone; that the Imaging
page not blank its picture between frames; and that the unit of every focus size be
plain. Two smaller items of the same area were looked at: the starless wording of the
imaging focus check, and the unit.

## Clarifications

### Session 2026-11-27 (the probe on `main` at f6d4f5fd, and review r1's fold)

- Q: What does this trio read besides itself? → A: Nothing about the feature. `spec.md`
  holds every requirement and scenario, and `plan.md` every design decision, the
  interfaces, the shapes that existing tests pin and the alternatives rejected. The
  probe's branch and logs are not in the repository; what they showed is written here.
- Q: Why did `Go anyway` end the run? → A: Every answer to the pause closes the focus
  aid: the run's focus part calls 033's `focus_stop`
  (`crates/finder/src/imaging/run/console.rs:599`), which aborts the aid's exposure in
  flight (`crates/finder/src/imaging/focus/run.rs:449`). The aid's `finish`
  (`crates/finder/src/imaging/focus/run.rs:1047`) then puts the remembered settings back
  once (`crates/finder/src/imaging/focus/run.rs:1058`) and keeps a refusal only as the
  aid's `restore_error` (`crates/finder/src/imaging/focus/run.rs:1072`), which nothing in
  the run reads. The run then holds the camera again
  (`crates/finder/src/imaging/run.rs:1204`), and its camera part refuses the aid's focus
  region, still in force, with 041's words
  (`crates/finder/src/imaging/run/console.rs:378`, 041 owner decision 24). The scope
  node's INDI adapter answers an abort at once
  (`crates/scope-node/src/imaging/indi/mod.rs:1102`) and sends a settings change at once
  after it, though it waits for the device's answer to every vector a settings change
  sends (`crates/scope-node/src/imaging/indi/mod.rs:1419`,
  `crates/scope-node/src/imaging/indi/mod.rs:1427`); a refusal is the driver's
  (`the camera refused {property}: …`, `crates/scope-node/src/imaging/indi/mod.rs:1435`),
  which the console words as `the scope node did not apply the settings: …`
  (`crates/finder/src/imaging/mod.rs:713`). The simulated imaging camera takes settings
  at once after an abort (`crates/simulator/src/imaging.rs:750`,
  `crates/simulator/src/imaging.rs:834`), so no test could see it. INFERRED, never seen
  in a log: that the field camera refused the put-back. The field logs never read
  `restore_error`; the next night's reading of it settles the cause (plan.md § Outside
  the factory, step 3).
- Q: Was it reproduced? → A: Yes, run on the factory's rig: with the simulated imaging
  camera refusing every settings change for a while after an abort, going on anyway
  from a pause with the aid's exposure in flight ends the run `camera_failed` with
  041's words, the aid's `restore_error` `the scope node did not apply the settings:
  the camera refused the settings: it is still stopping an exposure` and the aid's
  focus region in force (plan.md § What was run and what was read).
- Q: What becomes of a region a failed put-back leaves? → A: At f6d4f5fd and in the
  first draft, it was lost to the next aid or check: `restore_error` lives on the
  aid's run, which the next start replaces (`crates/finder/src/imaging/focus/run.rs:228`),
  and the next aid remembers the left region as the settings in force
  (`crates/finder/src/imaging/focus/run.rs:719`) and puts it back itself. Run in
  review r1: after `Check focus again` the second pause's aid ran on the left tile; the
  aid alone stopped mid-exposure, a check, then a run start answered 409 with 041's
  words. This spec keeps the region and its words on the focus state (FR-003).
- Q: Does 041's region refusal change? → A: No. A start or a re-hold with a region in
  force is still refused (041 owner decision 24); when the region is one an aid's
  put-back left, the words name that instead of telling the observer to clear a region
  they never set.
- Q: Why did the check call M42's stars clearly out? → A: The verdict compares the
  reading with the record, `good` up to 1.10 times it and `slightly_soft` up to 1.30
  times (`crates/finder/src/imaging/focus.rs:224`, `crates/finder/src/imaging/focus.rs:227`,
  `crates/finder/src/imaging/focus.rs:261`), and the record holds no altitude
  (`crates/finder/src/imaging/focus.rs:397`). 6.7″ at 37° and 6.4″ at 38° are 1.457 and
  1.391 times a record of 4.6″ read at 74°, both `clearly_out` (run). Seeing widens a
  star with the air it is seen through, but the record is not seeing alone: the
  repository's survey adds the defocus ring and seeing "in quadrature: a model no source
  measured" (`docs/research/2026-11-15-imaging-camera.md:220`), says a reading cannot
  tell focus error from seeing, tube currents, a cooling mirror or miscollimation
  (`docs/research/2026-11-15-imaging-camera.md:227`), and the README says the same
  (`crates/finder/README.md:975`); 4.6″ at 0.5475″ a pixel is 8.4 px, more than the
  survey's 4″ seeing (`docs/research/2026-11-15-imaging-camera.md:15`). So the carry is
  one-sided and capped (owner decision 2).
- Q: What blanks on the Imaging page? → A: The picture: the page draws it only while the
  latest frame has a preview (`crates/finder/web/src/components/ImagingPage.tsx:271`),
  a new frame starts `exposing` without one (`crates/finder/src/imaging/frames.rs:178`)
  and gets it once saved (`crates/finder/src/imaging/frames.rs:468`), and the page reads
  the view every second (`crates/finder/web/src/useImaging.ts:17`), so with the aid's
  1 s frames the picture is gone about half the time. The Focus block also hides its
  readings while the aid looks for the region again
  (`crates/finder/web/src/components/FocusBlock.tsx:184`). Read from code; INFERRED that
  the page the owner watched was this one: the field notes do not record which page
  blanked, and the driver's own viewer was also running.
- Q: What of the starless wording and the unit? → A: Owner decisions 4 and 5. The local
  ASTAP already answers a starless frame in a form the adapter reads as too few stars;
  the page already writes ″ or px, and MCP's tool descriptions are pinned by tests.

## Owner decisions

None is answered yet. Each keeps its options and its evidence, with a recommendation;
the scenarios and the record are written to the recommendations. Two general rules
carry the recommendations and remove the side effects a narrower fix would bring:
**whatever the focus aid sets, it gives back before anyone else holds the camera, or
says why it could not, and no aid keeps what another aid left; and a reading is
compared with the record at one altitude, the reading's own, by a carry that never
makes the record stricter.** The first puts the fix in the aid's one put-back, which
every way an aid ends goes through (a stop, a check that is done, a failure, a pause
answered either way, the Focus block's own stop), with the region it could not take
back kept beside the record, so 037's step traits and 041's refusal are not changed.
The second changes neither 033's thresholds nor its verdicts, adds no record per
altitude, and leaves every record without an altitude judging as it does today.

1. **The put-back after an abort** (FR-001 to FR-004). The cause was never seen in a
   log (§ Clarifications); two driver behaviours could make it, and the scope node's
   scripted INDI server (`crates/scope-node/tests/indi_expose.rs:578`) builds both: one
   that answers the abort switch only once its camera has stopped and refuses settings
   before (call it the late answer), and one that answers the abort switch at once and
   refuses settings while it stops (the early answer). INDI hands a driver its messages
   in order, so a refusal on the field needs a driver whose abort returns before its
   camera stops; whether it then answers late or early is not read here (no INDI
   source on this machine; plan.md § What was run, read, not run). Recommended (a):
   **the aid's put-back tries again, once a second for up to 5 s, when the scope node
   refuses it (502) while the camera is ready; the region a put-back left is kept on
   the focus state with its words, named by 041's refusal while it stands and never
   kept by a later aid or check.** Evidence (run, plan.md § What was run): on main's
   code all seven of US1's tests fail; with (a), a camera that refuses for 0.5 s after
   an abort lets the run stack on the whole sensor, one that refuses for 10 s ends the
   run `camera_failed` with `the focus aid could not put the imaging camera's settings
   back: ` and the refusal, and the region survives a later check, is never kept by
   one, and becomes the observer's once they change the settings. It covers both
   driver behaviours while the camera stops within 5 s, and names any longer refusal
   plainly. 043's silent node still answers within its bound: its refusal (409, the
   camera not `ready`) is not tried again (the mutation that retries any refusal fails
   043's US1-S8 test). 1 s is the INDI adapter's own pause before it tries an exposure
   again (`crates/scope-node/src/imaging/indi/mod.rs:45`); 5 s is 043's `ABORT_WAIT`.
   Its cost: a stop, or a Stop of the run during a pause, answers up to about 5 s
   later while the camera refuses (§ Edge Cases). Instead: (b) **the scope node waits
   for the abort's answer, with the naming and no retry** (review r1's (f)): the
   adapter sends no settings change after the port's abort until the device has
   answered `CCD_ABORT_EXPOSURE`, within `ANSWER_WAIT`
   (`crates/scope-node/src/imaging/indi/mod.rs:43`), as it waits for every vector a
   settings change sends. Trial-built (run, `probe/054` 973565bf, set aside in
   cf547227): on the scripted server the put-back waits 1.5 s for a driver that answers
   1.5 s late and 5 s for one that never answers, and takes; a driver that answers
   early and refuses the frame is refused all the same (`the camera refused CCD_FRAME:
   …`). The form review r1 worded, the abort itself answering only at the device's
   answer, fails 030's US6-S6 test (`crates/scope-node/tests/indi_expose.rs:865`, by
   5 s: 030 pins `Aborted` "in the virtual instant `abort()` was called"), an existing
   test this spec does not edit. So (b) covers only the late answer, and adds a scope
   node change, its approval and a story; (c) **(b) with a short retry as a defence**:
   covers what (a) covers, with a scope node change for no case (a) leaves; (d) **the
   retry without the left region** (the first draft): a later aid or check keeps the
   left region and the naming is lost (review r1, run); (e) **the naming alone**: the
   run still ends whenever the camera is slow to take settings after an abort, which on
   the field night was every time (US1-S1 fails); (f) **the run re-applies its own
   settings** after the aid closes (trial-built, `probe/054` 5e5b5b84, set aside in
   2664feef): once, it is refused too; five times, the run stacks but the aid's
   `restore_error` stays and the aid stopped alone keeps its region; (g) **the scope
   node waits a fixed time after an abort**: testable on the scripted server as (b) is
   (the first draft's "untestable" was wrong), but its wait is a guess that (b) makes
   general.
2. **The verdict and the altitude** (FR-005 to FR-012). Recommended (a): **the record
   keeps the altitude it was read at, and a reading is judged against the record
   carried to its own altitude by the airmass to the power 0.6, the factor held
   between 1 and 1.30**: factor = min(1.30, max(1, (X(A)/X(F))^0.6)), X(h) the secant
   of the zenith distance, F the record's altitude and A the reading's, Track's
   target's while Track is `tracking`, from the session's site, at the middle of the
   reading's exposure; with either altitude unknown the record judges as it is. The
   carry only explains: it never makes the record stricter (a record read low judges a
   high target as it is), and it never lets a low target off by more than
   `slightly_soft` allows, so `clearly_out` starts at most at 1.69 times the record, 7.77″
   for the field's 4.6″, under the far-out limit of about 11″ at every altitude.
   Evidence (run, plan.md § What was run): on the field night's numbers 6.7″ at 37°
   reads 1.120 times the carried record of 5.98″, `slightly_soft`, and 6.4″ at 38° 1.070,
   `good`: the run goes on, with a warning for the first. Both factors pass the cap, so
   the verdicts do not depend on the secant against another airmass formula nor on the
   record's altitude anywhere in the session's 72° to 74° (the 4.60 is the session's
   last reading's altitude, not the reading's own; at 66° the two read 1.134 and 1.098).
   In the rig, a record of 2.0 px at 74° carries to 2.459 px at Track's target's 42.95°,
   and a reading of 2.826 px is `slightly_soft`, `clearly_out` uncarried. **Records
   are compared with each other as read** (owner approval 8): under a one-sided carry a
   size carried to the zenith is unchanged, so comparing two records at one altitude
   is comparing them as read. The pause sentence and MCP's `focus.record` then name the
   carried size (owner approval 7). Instead: (b) **a quadrature carry with a stated
   seeing share s**: reference = √((s·k)² + R² − s²), k the uncapped factor; 6.7″ at 37°
   reads 1.267 (`slightly_soft`) at s = 3.0″ and 1.317 (`clearly_out`) at 2.5″: the
   verdict rests on a share no reading here measures; (c) **no carry, a note on the
   page only**: the field night's readings stay `clearly_out` at 1.457 and 1.391; (d)
   **the full carry** (the first draft): it reads the whole size as seeing; from 4.6″
   at 74° `clearly_out` would start at 8.85″ at 30° and 11.12″ at 20°, past the far-out
   limit, so below about 20.6° no change can pause; a doubled optical blur at 20°
   (8.93″) reads `good`; a record of 6.1″ read at 37° carries to 4.67″ at 70°, and a
   well-focused 5.5″ there reads `slightly_soft`; and the field's 6.7″ sits 0.02 % under
   the `good` line; (e) **a record per altitude band**: M42 at 37° would have no record
   and the run would pause all the same; (f) **looser thresholds** at every altitude.
   Only a night with records at two altitudes and the same focus can settle the share
   of seeing (plan.md § Outside the factory, step 6).
3. **What the view shows between previews** (FR-013, FR-014). Recommended (a): **the
   Imaging page keeps the last picture it drew until the next preview, under a quiet
   line `Frame N, the last picture`, and the Focus block keeps its readings while the
   aid looks for the region again.** Evidence (run): on main's page four of the five
   new Vitest tests fail; with (a) they pass, the whole Vitest suite and
   `CI=1 npm run e2e`, its red-only night check among it, unchanged. Instead: (b) **a
   `last_preview` in the frame view**: a change to 031's view, read by the page, MCP
   and their tests, for what the page alone needs (read, not run); (c) **a picture in
   the Focus block**: a second picture of the same frame on the same page; (d) **leave
   it**.
4. **The starless wording** (out of scope here). Recommended (a): **not here**; capture
   what the field's ASTAP writes for a starless frame first (plan.md § Outside the
   factory, step 5). Evidence (run, plan.md § What was run): the local `astap_cli`
   (CLI-2025.09.29) answers `-analyse 30` on noise, bright noise, daylight and flat
   frames with `HFD_MEDIAN=99.0` and `STARS=0`, exit 0, which the adapter already reads
   as too few stars (`crates/finder/src/imaging/astap.rs:429`); the words of the field
   night, `the imaging solver's measure could not be read`
   (`crates/finder/src/imaging/astap.rs:439`), come only from output with neither line,
   which the field's older ASTAP must have written (read, not run: the scope node's
   ASTAP is not in the repository's sources). Instead: (b) **fold it in**, reading
   output with neither line as too few stars: inferred from one night's words, and it
   would hide a real failure to read the measure. Seen in passing, not in scope:
   saturated and all-zero frames answer `STARS=29721` and `STARS=29119` with
   `HFD_MEDIAN=99.0`, which the adapter reads as far out.
5. **The unit of every focus size** (FR-012). Recommended (a): **one README sentence**:
   every size in the imaging view's `focus` and in the imaging run's `focus` is in the
   `unit` beside it, `arcsec` or `px`, and at binning 1 and 0.55″ a sensor pixel the
   far-out limit is about 11″ (20 frame pixels at 0.5475″, the aid's own binning). The
   page already writes every size with ″ or ` px`
   (`crates/finder/web/src/components/FocusBlock.tsx:127`), and the views carry `unit`.
   Instead: (b) **the MCP tool descriptions too**: they are pinned word for word by
   `crates/finder/tests/mcp/imaging_focus.rs:242`, `crates/finder/tests/mcp/imaging_focus.rs:246`
   and `crates/finder/tests/mcp/imaging_run.rs:133`, existing tests this spec does not
   edit.

## Owner approvals

Flipping this spec to `ready` approves the answer to every owner decision above, and
the following. Each is named in the FR that makes it. No existing test, golden, fixture
or CI file changes.

1. **A change to the simulator** (FR-001): `SimImagingCamera::with_abort_settle` and
   `simulator::imaging::STILL_STOPPING`; at its default of zero the camera is
   f6d4f5fd's. Nothing else in `crates/simulator` changes.
2. **New public API in `crates/finder`'s library** (FR-002, FR-005 to FR-007, FR-011):
   `focus::run::RESTORE_PAUSE`, `focus::run::RESTORE_WAIT`, `focus::SEEING_EXPONENT`,
   `focus::CARRY_CAP`, `focus::carry`, `focus::Altitudes`, `Kept::note_at`,
   `Kept::reference_at`, `Kept::altitudes` and `focus::run::CarriedView`; the fields
   `SizeAt::alt_deg`, `LastView::alt_deg` and `ReferenceView::carried`. No existing
   signature changes: `Kept::note` and `Kept::reference` keep theirs.
3. **`focus.json`** (FR-008): the optional `record.tonight.alt_deg` and
   `record.earlier_alt_deg`, its `format` still `imaging-focus/1`.
4. **041's refusal's second form** (FR-003): `the focus aid could not put the imaging
   camera's settings back: …`, while a region an aid's put-back left is in force;
   041's words otherwise.
5. **The README's sentences** (FR-004, FR-012), in `## Focus` and `## An imaging run`,
   and `GLOSSARY.md`'s **Focus record** (FR-012).
6. **The page** (FR-013 to FR-015): the kept picture and its line `Frame N, the last
   picture`, the readings kept while the aid looks for the region again, and the words
   ` at N°`, ` with no altitude` and `, as it reads at A°: V`.
7. **What "the record" means in a check** (FR-010): the run's pause sentence
   (`crates/finder/src/imaging/run.rs:458`) and MCP's `get_imaging_run` `focus.record`
   carry the record as it judges the reading, the carried size, where they carried the
   record as read; the Focus block names both.
8. **Records compared as read** (FR-006, FR-007): a reading replaces tonight's record,
   and a night's record joins the earlier nights', by size as read, whatever their
   altitudes; a sharp focus read low (3.1″ at 30°) does not replace a softer one read
   high (2.9″ at 60°).

## User Scenarios & Testing *(mandatory)*

Every scenario shares these definitions.

- **The gate commands.** Every Rust scenario runs in
  `cargo nextest run --workspace --all-targets --all-features --locked`, the `test`
  gate's first command, each test in its own process. The page's Vitest scenarios run
  in `npm test`, a later command of the same gate, in `crates/finder/web`.
- **The pause rig** is the console rig of
  `crates/finder/tests/console/imaging_run_focus.rs` (its `rig`,
  `crates/finder/tests/console/imaging_run_focus.rs:363`, copied into the new file, as
  it is private) with the console's own camera and focus parts: a simulator on
  `crate::support::live::DB` with `Site::default()` and a `VirtualClock` from
  `crate::support::live::START`, pushed to altitude 40° and azimuth 0°, focus error 0;
  its imaging camera `SimImagingCamera::new(sim).with_abort_settle(settle)`; a
  `SimNode::from_cameras` with the token `test-token` and a 500 ms frame interval,
  reached over an in-memory duplex; the session's node and site as that rig sets them
  (`crates/finder/tests/console/imaging_run_focus.rs:393`); `AppState::shared` with
  `with_session`, `with_clock` of the simulator's clock, `with_link`, `with_live()`,
  `with_imaging`, `with_focal_length_mm(1800.0)`, the simulated imaging solver with
  `with_measure_noise(7, 0.02)`, and `with_imaging_run_steps` of a `Script`'s steps whose
  camera and focus are `console::camera` and `console::focus`; served by
  `crate::support::mcp::app(state, "127.0.0.1")`, so that `/mcp` answers too. The tests
  run under `#[tokio::test(start_paused = true)]`, read `GET /api/imaging` every 100 ms,
  and every state awaited comes within 300 s of paused time. No focus record is kept.
- **Whole** means that `settings.roi` of `GET /api/imaging` has `x` 0, `y` 0 and the
  `camera`'s `width` and `height`.
- **Going on anyway mid-exposure** is: wait for `state` `ready`; `POST /api/imaging/run`
  with `{}`, answered 200; wait until `run.state` is `focus_pause`, `focus.aid.state`
  `running` with `readings` at least 1, `frame.state` `exposing` and `frame.purpose`
  `focus`, and check there that `run.focus.verdict` is `no_record` and the settings are
  not whole; then `POST /api/imaging/run/continue` with `{"anyway":true}`, answered
  200; then wait until `run.state` is `stacking` or `ended`.
- **The aid alone exposing** is: wait for `state` `ready`; `POST /api/imaging/focus`
  with `{"tile":{"col":1,"row":1}}`, answered 200; wait until `focus.aid.state` is
  `running`, `frame.state` `exposing` and `frame.purpose` `focus`, and check there that
  the settings are not whole. **A console check** is `POST /api/imaging/focus` with
  `{"check":true}`, answered 200, then waiting until `focus.aid.check` is true and
  `focus.aid.state` `ended`.
- **STILL** is `the camera refused the settings: it is still stopping an exposure`, the
  simulator's `STILL_STOPPING`; **REFUSED** is `the scope node did not apply the
  settings: ` followed by STILL; **NAMED** is `the focus aid could not put the imaging
  camera's settings back: ` followed by REFUSED.
- **The motion rig** is a simulator on `crate::support::live::DB` with `Site::default()`
  and a `VirtualClock` from `crate::support::live::START`, with a `SimMount` connected,
  pushed to altitude 50° and azimuth 120°, focus error 0; served by `SimNode::new` with
  `test-token` and a 500 ms frame interval over an in-memory duplex; a session with that
  node, the site of `crates/finder/tests/console/imaging_run_focus.rs:393` and the
  target `M42`; `AppState::shared` with `with_session`, `with_clock` of the simulator's
  clock, `with_link`, `with_live_solver` of `crate::support::goto::Stars`,
  `with_imaging` and the simulated imaging solver without noise, served by
  `finder::router`. It waits for the imaging `state` `ready` and a first sync
  (`model.syncs` above 0 in `GET /api/session`), then sets the aim at the simulator's
  eyepiece pixel with `Session::set_aim`. Paused time; every read every 100 ms, every
  state within 900 s.
- **A record by hand of P px at A°** is `focus.json` in the imaging folder, written
  before the console starts: `format` `imaging-focus/1`, the simulated imaging camera's
  `camera` (`simulated imaging camera`, `SIM-0001`, 1236 × 822, 19.12 µm), the `fixed`
  region of tile column 1 and row 1 (x 412, y 274, 412 × 274), and `record.tonight` with
  `px` P, `at` and `since` 600 s before `START`, and `alt_deg` A.
- **Track on** is `POST /api/track` with `{"on":true}`, then waiting for `track.state`
  `tracking` in `GET /api/session`, then 15 s. **A check** is `POST /api/imaging/focus`
  with `{"check":true}`, then waiting for `focus.aid.state` `ended`. **The true
  altitude** is the simulator's `telescope().eyepiece.alt`. **s** is the view's
  `focus.arcsec_per_px`, or 1 when it has none.
- **The airmass** X(h) is 1/sin h, the secant of the zenith distance, and
  **carried(P, F, A)** is P·min(1.30, max(1, (X(A)/X(F))^0.6)).
- **The page rig** is the component rendered by Testing Library with `fetch` stubbed to
  answer the views the test writes, as the page's existing tests do, each locator held
  to its block's region (`Imaging frame` or `Focus`).
- **Oracles.** The words of 041's refusal, of the simulator's refusal and of these
  scenarios; 031's own view of the settings and the camera; the airmass, the exponent
  0.6 and the bounds 1 and 1.30 written in the test, and 033's thresholds 1.10 and 1.30;
  the field night's numbers; the simulator's own altitude; and the views the page tests
  write. No expected value comes from the code under test: no test reads
  `RESTORE_PAUSE`, `RESTORE_WAIT`, `SEEING_EXPONENT` or `CARRY_CAP`, or calls `carry` for
  an expected value.

### User Story 1 - Going on from a focus pause gives the camera back whole (Priority: P1)

As the owner, I need `Go anyway` at a focus pause to go on with the imaging camera on
its whole sensor, and, when the camera will not take its settings back, to be told
that, not to clear a region I never set, whatever I do with the aid meanwhile.

**Why this priority**: Without it no imaging run whose focus check pauses can go on;
on the field night that was every run.

**Independent Test**: `crates/finder/tests/console/imaging_run_focus_restore.rs`, new.
The `test` gate's nextest command runs it; during development,
`cargo nextest run -p finder --locked --all-features --test console -E 'test(imaging_run_focus_restore)'`.

**Acceptance Scenarios**:

1. **Given** the pause rig with a settle of 0.5 s, **When** a committed test goes on
   anyway mid-exposure, **Then** it asserts that `run.state` is `stacking`, that the
   settings are whole, and that `focus.aid` has no `restore_error`.
2. **Given** the pause rig with a settle of 10 s, **When** a committed test goes on
   anyway mid-exposure, **Then** it asserts that `run.state` is `ended` with
   `run.ended.reason` `camera_failed`; that `focus.aid.restore_error` is REFUSED; that
   `run.ended.message` is `Ended: the imaging camera failed: ` followed by NAMED and a
   full stop; that MCP's `get_imaging_run` answers that message as
   `result.structuredContent.run.ended.message`; and that MCP's `get_focus_reading`
   answers REFUSED as `result.structuredContent.aid.restore_error`.
3. **Given** the pause rig with a settle of 0.5 s, and again a fresh one with 10 s, and
   the aid alone exposing, **When** a committed test sends `DELETE /api/imaging/focus`
   and measures the paused time until it answers, **Then** it asserts that each answers
   200 with `focus.aid.ended_reason` `stopped`; that at 0.5 s it answers 0.9 s to 1.5 s
   after it was sent, with no `restore_error` and the settings whole; and that at 10 s
   it answers 4.9 s to 6.0 s after it was sent, with `restore_error` REFUSED and
   `settings.roi` as it was while the aid ran.
4. **Given** the pause rig with a settle of 20 s and the aid alone exposing, **When** a
   committed test sends `DELETE /api/imaging/focus`, answered 200, then makes a console
   check, then sends `POST /api/imaging/run` with `{}`, then, 21 s after the `DELETE`
   was sent, makes a console check again and sends `POST /api/imaging/run` with `{}`
   again, **Then** it asserts that the `DELETE` answers `focus.aid.restore_error`
   REFUSED; that the first check ends `focus.aid.ended_reason` `failed` with
   `settings.roi` as it was while the aid ran; that the first start answers 409 with
   `error` NAMED; that the second check ends `done` with the settings whole; and that
   the second start answers 200.
5. **Given** the pause rig with a settle of 10 s and the aid alone exposing, its
   `settings.roi` x 412, y 274, 412 × 274, **When** a committed test sends
   `DELETE /api/imaging/focus`, then, 11 s after it was sent, `POST /api/imaging/run`
   with `{}`, then `PUT /api/imaging/settings` with
   `{"roi":{"x":412,"y":274,"width":412,"height":274}}`, then `POST /api/imaging/run`
   with `{}` again, **Then** it asserts that the `DELETE` answers 200 with
   `focus.aid.restore_error` REFUSED; that the first start answers 409 with `error`
   NAMED; that the `PUT` answers 200; and that the second start answers 409 with
   `error` `the imaging camera has a region of interest set: clear it before an
   imaging run`.
6. **Given** the pause rig with a settle of 20 s, **When** a committed test waits for
   `state` `ready`, sends `POST /api/imaging/run` with `{}`, answered 200, waits until
   `run.state` is `focus_pause`, `focus.aid.state` `running`, `frame.state` `exposing`
   and `frame.purpose` `focus`, sends `POST /api/imaging/run/continue` with
   `{"anyway":false}`, answered 200, waits until `run.state` is `focus_pause` again with
   `focus.aid.check` false, `focus.aid.state` `ended` and `focus.aid.since` later than
   the first pause's, then sends `POST /api/imaging/run/continue` with
   `{"anyway":true}`, answered 200, and waits until `run.state` is `ended`, **Then** it
   asserts that `settings.roi` at the second pause is the first pause's, that
   `run.ended.reason` is `camera_failed`, and that `run.ended.message` is `Ended: the
   imaging camera failed: ` followed by NAMED and a full stop.
7. **Given** `crates/finder/README.md`, **When** a committed test reads its `## Focus`
   section, up to the next `## ` heading, and its `## An imaging run` section, up to
   `### Field checklist`, each with its line breaks read as spaces, **Then** it asserts
   that the first contains `is tried again once a second for up to 5 s` and `the next
   aid or check puts the whole sensor back`, and the second
   `` `the focus aid could not put the imaging camera's settings back:` ``.

### User Story 2 - The focus record keeps the altitude it was read at (Priority: P1)

As the owner, I need the focus record to keep the altitude each of its parts was read
at, and a way to carry a size from one altitude to another that never makes the record
stricter, so that a reading can be compared with the record as it would read at the
reading's altitude.

**Why this priority**: The verdict cannot account for the air until the record knows
where it was made.

**Independent Test**: `crates/finder/tests/console/imaging_focus_carry.rs`, new. The
`test` gate's nextest command runs it; during development,
`cargo nextest run -p finder --locked --all-features --test console -E 'test(imaging_focus_carry)'`.

**Acceptance Scenarios**:

1. **Given** `focus::carry` and `focus::judge`, **When** a committed test calls `carry`
   of 4.6 from 74° to 74°, 60° and 20° and of 6.1 from 37° to 70°, and judges the field
   night's readings, **Then** it asserts that `carry(4.6, 74.0, 74.0)` is 4.6; that
   `carry(4.6, 74.0, 60.0)` is within 10⁻¹² of carried(4.6, 74, 60) and between 4.6 and
   4.6·1.30, both excluded; that `carry(4.6, 74.0, 20.0)` is within 10⁻¹² of 4.6·1.30;
   that `carry(6.1, 37.0, 70.0)` is 6.1; that `judge(Ok(6.7), Some(4.6))` and
   `judge(Ok(6.4), Some(4.6))` are `Verdict::ClearlyOut`; that `judge(Ok(6.4),
   Some(carry(4.6, 74.0, 38.0)))` is `Verdict::Good`; that `judge(Ok(7.3),
   Some(carry(4.6, 74.0, 37.0)))` is `Verdict::SlightlySoft`; and that `judge(Ok(8.5),
   Some(carry(4.6, 74.0, 37.0)))` is `Verdict::ClearlyOut`.
2. **Given** a `Kept` on a `focus.json` in a scratch folder, described with the
   simulated imaging camera, and T = 1,768,528,800 s, **When** a committed test calls on
   it, in order, `note_at(3.0, T, None, false, Some(74.0))`, `note_at(2.9, T + 60, None,
   false, Some(60.0))` and `note_at(3.1, T + 120, None, false, Some(30.0))`, describes a
   second `Kept` on the same file and reads its `reference_at`, then calls on the first
   `note_at(3.2, T + 43260, None, false, None)` and `note(2.5, T + 43300, None, false)`,
   reading `reference_at`, `altitudes` and the file between them, **Then** it asserts
   that the first note answers true, `reference_at(T + 1, None)` then answers `(3.0,
   Of::Tonight, Some(74.0))` and the file's `record.tonight.alt_deg` is 74; that the
   second answers true and the third, compared as read, false, and `altitudes(T + 121,
   None).tonight` is `Some(60.0)`; that the second `Kept` answers `reference_at(T + 121,
   None)` as `(2.9, Of::Tonight, Some(60.0))`; that the note a night later answers true,
   `reference_at(T + 43261, None)` then answers `(2.9, Of::Earlier, Some(60.0))`, the
   file's `record.earlier_alt_deg` is 60 and its `record.tonight` has no `alt_deg`; and
   that the last answers true and `reference_at(T + 43301, None)` answers `(2.5,
   Of::Tonight, None)`.
3. **Given** a scratch folder of its own, a fresh `Kept` on `focus-scaled.json` there
   described with the same camera, and T as in US2-S2, **When** a committed test calls
   `note_at(3.0, T, Some(0.55), false, Some(74.0))` on it and reads its `altitudes` at the
   scales 0.55 and 1.1, then `note_at(5.0, T + 60, Some(1.1), false, None)`, then writes
   033's own `focus.json` by hand in that folder, with `format` `imaging-focus/1`, the
   simulated camera's `camera` block, no region and `record.tonight` of `px` 3.3 with
   `at` and `since` T − 800 and no altitudes, and describes another `Kept` on it,
   **Then** it asserts that the first note answers true; that `altitudes(T + 1,
   Some(0.55)).tonight` is `Some(74.0)` and `altitudes(T + 1, Some(1.1))` is
   `Altitudes::default()`; that the note at 1.1 answers true and `reference_at(T + 61,
   Some(1.1))` answers `(5.0, Of::Tonight, None)`; and that the other `Kept` answers
   `reference_at(T, None)` as `(3.3, Of::Tonight, None)` and `reference(T, None)` as
   `(3.3, Of::Tonight)`.

### User Story 3 - The aid and the check judge at Track's altitude (Priority: P1)

As the owner, I need each focus reading to carry the altitude of Track's target while
Track is tracking, the record to keep it, and a check to judge against the record
carried to that altitude, so that a target low in the sky is not called out of focus
for the air alone.

**Why this priority**: This is what turns the field night's `clearly_out` into a
verdict that lets the run go on.

**Independent Test**: `crates/finder/tests/motion/imaging_focus_altitude.rs`, new. The
`test` gate's nextest command runs it; during development,
`cargo nextest run -p finder --locked --all-features --test motion -E 'test(imaging_focus_altitude)'`.

**Acceptance Scenarios**:

1. **Given** the motion rig with a record by hand of 2.0 px at 74°, **When** a committed
   test sends `POST /api/track` with `{"on":true}` and at once makes a check and reads
   `track.state`, then waits for `track.state` `tracking`, waits 15 s, makes a check
   and reads the true altitude, **Then** it asserts that the first check ended `done`
   while `track.state` was still `armed`, its `focus.aid.reference` without `carried`
   and its `focus.aid.last` without `alt_deg`; that the second check's
   `focus.aid.ended_reason` is `done`; that its `focus.aid.reference.carried` has
   `from_alt_deg` 74, `alt_deg` within 0.5° of the true altitude and `from` within
   10⁻⁹ of 2.0·s; that `focus.aid.last.alt_deg` equals that `alt_deg`; that
   `focus.aid.reference.value` is within 10⁻⁹ of carried(2.0·s, 74, that `alt_deg`),
   relatively; that `focus.aid.verdict` is `good` when `focus.aid.median.value` is at
   most 1.10 times that value, `slightly_soft` when at most 1.30 times and `clearly_out`
   beyond, and is `slightly_soft`; that the median is above 1.30·2.0·s, so that the
   record uncarried would call it `clearly_out`; and that
   `focus.record.tonight.alt_deg` is 74.
2. **Given** the motion rig with no record and the simulator's focus error 0.5, **When**
   a committed test makes a check; starts the aid with `POST /api/imaging/focus` and
   `{}` and waits until `focus.record.tonight` is an object, stops it with
   `DELETE /api/imaging/focus`; turns Track on, sets the focus error to 0, starts the
   aid with `{"find":true}` and reads the true altitude, **Then** it asserts that the
   check's `focus.aid` has no `reference`; that the record made with Track off has no
   `record.tonight.alt_deg`; and that, with Track on, `focus.record.tonight.alt_deg`
   comes and is within 0.5° of the true altitude.
3. **Given** `crates/finder/README.md`, **When** a committed test reads its `## Focus`
   section, up to the next `## ` heading, and its `## An imaging run` section, up to
   `### Field checklist`, each with its line breaks read as spaces, **Then** it asserts
   that the first contains `grows as the airmass to the power 0.6`, `judged against the
   record carried to its own altitude`, `never by less than 1 or more than 1.30` and
   `the far-out limit is about 11″`, and that the second contains `against the record
   carried to the target's altitude`.

### User Story 4 - The Imaging page keeps its picture and readings, and names the altitudes (Priority: P2)

As the owner, I need the Imaging page to keep the last picture until the next one is
ready and say that it is the last, the Focus block to keep its readings while the aid
looks for its region again, and both lines that compare with the record to say at
what altitudes.

**Why this priority**: The run goes on and judges rightly without it; this makes the
page steady to read at the focuser.

**Independent Test**: `crates/finder/web/src/components/FocusView.test.tsx`, new. The
`test` gate's `npm test` runs it; during development, `npm test -- FocusView` in
`crates/finder/web`.

**Acceptance Scenarios**:

1. **Given** the page rig with `ImagingPage` (day), the view `ready` with the simulated
   imaging camera and frame 1 `saved` with `preview` `/api/imaging/frames/1/preview.jpg`,
   **When** a committed test renders it, then answers frame 2 `exposing` with no
   preview, then frame 2 with `preview` `/api/imaging/frames/2/preview.jpg`, **Then** it
   asserts, within the region `Imaging frame`, that the image `Imaging frame 1` shows
   `/api/imaging/frames/1/preview.jpg` first; that once the status reads `Exposing for
   1000 ms` the image `Imaging frame 1` is still there with the text `Frame 1, the last
   picture`; and that then the image `Imaging frame 2` shows
   `/api/imaging/frames/2/preview.jpg`, `Imaging frame 1` is gone and no text holds
   `the last picture`.
2. **Given** the page rig with `FocusBlock`, the unit `arcsec` at 0.55″ a pixel and an
   aid `finding` with 9 readings, the median, last and best 6.7, **When** a committed
   test renders it, then renders afresh an aid `finding` with 0 readings and no median,
   last, best or history, **Then** it asserts, within the region `Focus`, the text
   `6.70″` and the text `Finding the focus region on a full imaging frame` the first
   time, and no text `Focus reading` the second.
3. **Given** the page rig with `FocusBlock`, the record's `tonight` of 4.6 at `alt_deg`
   74, and an aid `running` with the median 6.7, its last reading's `alt_deg` 37, the
   verdict `slightly_soft` and the reference of 5.98 `tonight` `carried` from 4.6 at 74°
   to 37°, **When** a committed test renders it, **Then** it asserts, within the region
   `Focus`, the text `Focus record: 4.60″ at 74° tonight.` and the text `Slightly soft:
   12% larger than the focus record of 4.60″ at 74° tonight, as it reads at 37°: 5.98″.`
4. **Given** the page rig with `FocusBlock`, the record's `tonight` of 4.6 with no
   `alt_deg`, and an aid `running` with the median 6.7, its last reading's `alt_deg` 37,
   the verdict `clearly_out` and the reference of 4.6 `tonight` not carried, **When** a
   committed test renders it, **Then** it asserts, within the region `Focus`, the text
   `Focus record: 4.60″ with no altitude tonight.` and the text `Clearly out: 46% larger
   than the focus record of 4.60″ tonight.`

### Edge Cases

- **The camera refuses for longer than 5 s**: the aid's `restore_error` is the last
  refusal's words, the camera keeps the aid's region, and the run ends naming the
  put-back (US1-S2); `Use the whole sensor` on the Focus block, or
  `PUT /api/imaging/settings` with `{"roi":null}`, clears it (033 FR-033, unchanged).
- **A later aid or check, or the observer, after a failed put-back**: the region stays
  named while it is in force (US1-S4, US1-S6); a later aid or check remembers it as the
  whole sensor and so puts the whole sensor back (US1-S4); the observer's own settings
  change makes the region theirs, even the same one (US1-S5).
- **A Stop while the camera refuses**: the aid's stop, a Stop of the run during a
  pause (`crates/finder/src/imaging/run.rs:1406` closes the aid), and MCP's
  `set_focus_aid` stop answer up to about 5 s later than at f6d4f5fd (US1-S3), as the
  README says.
- **The scope node goes silent or the link drops during the put-back**: the camera is
  not `ready` and the refusal is 409, the last at once, and 043's bound holds (043
  US1-S8, its test unchanged; run).
- **A refusal that cannot change** (a 400, as for a setting out of range): the last at
  once, never tried again. Read from the diff: no rig makes a put-back the console
  refuses with 400.
- **Anyone else asking for the camera during the retries**: refused with the aid's name,
  as during the aid (033 FR-027, 031's lease); the run's own re-hold comes only after
  the aid has ended.
- **Track not `tracking` (off, or `armed` while its GoTo runs), no site, the target at
  or below the horizon, or a record made before this spec**: no altitude, no carry, the
  verdict as at f6d4f5fd (US2-S2, US3-S1, US3-S2; the existing tests of 033 and 041 pass
  unchanged; run).
- **A record made with Track off, judged while Track is tracking**: uncarried, and the
  Focus block's record line says ` with no altitude` (US4-S4).
- **Track's target and where the telescope points**: the altitude is the target's,
  which Track holds; in the rig it is within 0.5° of the simulator's own (US3-S1).
- **Very low targets**: the factor reaches the cap of 1.30 at about 37° from a record
  made at 74°, so below that the record is loosened no further and `clearly_out` stays
  reachable, at 1.69 times the record (US2-S1, `carry(4.6, 74.0, 20.0)`).
- **A record read low judging a high target** (the reverse case): the factor would be
  below 1, so the record judges as it is (US2-S1, `carry(6.1, 37.0, 70.0)`); a soft
  focus high up is then judged against a lenient record (6.7″ at 70° against 6.1″ read
  at 37° is `good`), which the next record made high corrects (plan.md § Risks).
- **A record kept across nights**: the earlier nights' part keeps its altitude (US2-S2).
- **The pause sentence**: it names the carried size as the record; the Focus block
  names both altitudes (owner approval 7).

## Requirements *(mandatory)*

### Functional Requirements

**The put-back** (US1):

- **FR-001**: `SimImagingCamera` SHALL gain `with_abort_settle(settle: Duration)`,
  zero by default, and `crates/simulator/src/imaging.rs` the public `STILL_STOPPING`,
  `the camera refused the settings: it is still stopping an exposure`. While less than
  `settle` has passed since an abort of an exposure in flight, on tokio's clock, its
  `apply` of any settings other than `ImagingSettings::default()` SHALL answer
  `ImagingError::Failed` with `STILL_STOPPING`, after its exposing check and before any
  setting changes; at zero it SHALL refuse none, so that every existing caller sees
  f6d4f5fd's camera. Nothing else in `crates/simulator` SHALL change (owner approval 1).
- **FR-002**: 033 FR-027 says: "However it ends, it SHALL wait out an imaging frame in
  flight, put the remembered settings back, reporting a failure to do so as
  `restore_error`, and then release the lease, so that the next taker is not refused."
  This spec keeps that and SHALL make the put-back so: a try the scope node refuses
  (the refusal's status 502) while the imaging camera's state is `ready` SHALL be
  followed by another `RESTORE_PAUSE` (1 s) later, so long as that try starts within
  `RESTORE_WAIT` (5 s) of the first; the first try that takes leaves no
  `restore_error`, and otherwise the last refusal's words SHALL be `restore_error`. Any
  other refusal SHALL be the last at once. Only then SHALL the lease be released and the
  aid end, and `DELETE /api/imaging/focus` (033 FR-028) answers then. 033 FR-021's
  remembering SHALL take a region in force that an earlier put-back left (FR-003) as the
  whole sensor. `RESTORE_PAUSE` and `RESTORE_WAIT` SHALL be public constants of
  `crates/finder/src/imaging/focus/run.rs` (owner decision 1 (a), owner approval 2).
- **FR-003**: 041 FR-021 says that holding the lease the console's camera part "SHALL
  refuse … with `the imaging camera has a region of interest set: clear it before an
  imaging run` while the settings hold one (owner decision 24)". This spec keeps the
  refusal. The focus state, not the aid's run, SHALL keep the region in force when a
  put-back failed, with that put-back's words, set as the aid ends; it SHALL be cleared
  when a put-back takes, when the camera is described again, and when
  `Imaging::set` (`PUT /api/imaging/settings`, `crates/finder/src/imaging/mod.rs:584`,
  and MCP's `set_imaging_camera`) changes the settings. While the settings in force hold
  that region, the refusal SHALL be worded `the focus aid could not put the imaging
  camera's settings back: ` followed by those words, and with 041's words otherwise.
  037 FR-002's "SHALL end `camera_failed` with the camera part's words when it cannot"
  then gives the run's `ended.message` `Ended: the imaging camera failed: ` followed by
  those words and a full stop, and MCP's `get_imaging_run` and `get_focus_reading` carry
  the same words, their tools and descriptions unchanged (owner approval 4).
- **FR-004**: `crates/finder/README.md` SHALL say, in `## Focus` after the sentence
  that ends `puts the settings back and releases the camera.`, that a put-back the
  scope node refuses while the camera is ready is tried again once a second for up to
  5 s and that a stop answers that much later, that one that never takes is the aid's
  `restore_error`, how the region it leaves is cleared, and that it stays named and the
  next aid or check puts the whole sensor back; and, in `## An imaging run` after `the
  page shows its time left.`, that every answer to a pause, and a Stop, closes the aid
  and that a put-back that failed ends the run `camera_failed` after `` `the focus aid
  could not put the imaging camera's settings back:` ``, each in the words of plan.md
  § Interfaces. The field checklist keeps its 18 steps (owner approval 5).

**The record's altitude** (US2):

- **FR-005**: `crates/finder/src/imaging/focus.rs` SHALL gain `SEEING_EXPONENT`, 0.6,
  `CARRY_CAP`, 1.30, and `pub fn carry(px: f64, from_alt_deg: f64, to_alt_deg: f64) ->
  f64`, the size `px` read at `from_alt_deg` times the factor
  (X(`to_alt_deg`)/X(`from_alt_deg`))^`SEEING_EXPONENT` held between 1 and `CARRY_CAP`,
  where X(h) is 1/sin h of the geometric altitude h, as `crate::sky::alt_az` gives it
  (no refraction: under 0.1 % on the factor above 20°) (owner decision 2 (a), owner
  approval 2).
- **FR-006**: 033 FR-011 says: "`Kept::note` SHALL take a settled reading and whether
  the aid reports `found` at it: with no record of tonight, or one whose night began
  more than 12 hours before the reading, it starts a night with that reading, the old
  night's record joining the earlier nights' if it is the smaller; otherwise it replaces
  tonight's record only when smaller." This spec keeps that, its sizes compared as read
  whatever their altitudes (owner approval 8), and SHALL add
  `Kept::note_at(px, at, scale, passed, alt_deg: Option<f64>) -> bool`, which notes as
  `note` does and keeps `alt_deg` with the part the reading makes: a reading that starts
  a night or replaces tonight's record sets tonight's altitude to `alt_deg`; a night's
  record that joins the earlier nights' brings its altitude with it; a note that changes
  no size changes no altitude. `Kept::note` SHALL be `note_at` with no altitude, its
  signature unchanged, and a record the foreign rule (033 FR-013) drops SHALL drop its
  altitudes with it.
- **FR-007**: `Kept::reference_at(now, scale) -> Option<(f64, Of, Option<f64>)>` SHALL
  answer `Kept::reference` (033 FR-012, unchanged: the smaller of the two parts, as
  read) with the altitude of the part it names, and `Kept::altitudes(now, scale) ->
  Altitudes` the altitude of each part as `Kept::record` shows it at `now`: tonight's
  while its night began at most 12 hours ago, the earlier part's that of whichever
  record shows there, and none at a foreign scale. `Altitudes` SHALL have `tonight` and
  `earlier`, each an `Option<f64>`, and `of(Of)`.
- **FR-008**: 033 FR-014 says: "The record SHALL be written to `focus.json` with the
  focus region, neither write losing what the other wrote, and read back when the camera
  is described." This spec SHALL write, while known, tonight's altitude as `alt_deg` in
  `record.tonight` and the earlier nights' as `record.earlier_alt_deg`, each left out
  when unknown, with the `format` still `imaging-focus/1`; a file without them SHALL
  read as before, with no altitudes (owner approval 3).

**Judging at Track's altitude** (US3):

- **FR-009**: A focus reading's altitude SHALL be the geometric altitude of Track's
  target, from the session's site, at the middle of the reading's exposure (033's
  `at`), computed with `crate::sky::alt_az`, while the session's Track `state` is
  `tracking` and a site is set; and none otherwise (Track `off`, or `armed` while its
  GoTo runs, `crates/finder/src/session.rs:179`), or when it is at or below 0°. The
  session SHALL be read with the imaging state's lock released (plan.md § Interfaces).
  The mount, the live solve and `crates/finder/src/{model,goto,live,session}.rs` are
  read through `SessionView` only, and not changed.
- **FR-010**: 033 FR-025 says: "While the aid runs, its verdict SHALL be `judge` of each
  settled reading, or of an unmeasurable frame, against the reference (FR-012) as it
  stood before that reading was noted; the reference SHALL be kept with the verdict".
  This spec SHALL make that reference the record's part, carried by `carry` from the
  part's altitude to the reading's when both are known, and the part as it is
  otherwise; `judge` (033 FR-010) and its thresholds SHALL be unchanged; and each
  settled reading the aid notes (a check notes none) SHALL be noted with `note_at` and
  its altitude. The run's pause sentence (`crates/finder/src/imaging/run.rs:458`) SHALL
  be unchanged in its words, the record it names being that reference (owner approval
  7).
- **FR-011**: 033 FR-029 says that the focus view SHALL carry "`unit` (`arcsec` with a
  scale, else `px`), `arcsec_per_px`, `region`, `record` and `aid`, with every size in
  that unit". This spec SHALL add to each record part (`SizeAt`) its `alt_deg` when
  known, to the aid's `last` (`LastView`) the reading's `alt_deg` when known, and to the
  aid's `reference` its `carried`, `{"from", "from_alt_deg", "alt_deg"}`, when it was
  carried: the record's own size in the view's unit, its altitude and the reading's.
  Each SHALL be left out otherwise (owner approval 2).
- **FR-012**: `crates/finder/README.md` SHALL say, in `## Focus` after the sentence that
  ends `beyond which its star sizes stop being trustworthy.`, that every focus size is in
  the `unit` beside it and that at binning 1 and 0.55″ a sensor pixel the far-out limit
  is about 11″ (owner decision 5 (a)); in `## Focus` after the paragraph that ends
  `never against a fixed number.`, how the record is carried and held between 1 and
  1.30, and that records are compared as read; and in `## An imaging run`, that the
  check is judged against the record carried to the target's altitude, whose size the
  pause's words and `focus.record` give; each in the words of plan.md § Interfaces.
  `GLOSSARY.md`'s **Focus record** SHALL say that each part keeps its altitude when
  known and that a reading is judged against the record carried to its altitude, in
  the words of plan.md § Interfaces (owner approval 5).

**The page** (US4):

- **FR-013**: 031 FR-022 says the page SHALL "show the latest preview as a picture".
  This spec SHALL keep the last picture the page drew, with its own frame's alt text
  `Imaging frame <id>` and hold button, while the latest frame has no preview yet, until
  the next preview, with the quiet line `Frame <id>, the last picture` under it while
  it is kept (owner decision 3 (a), owner approval 6).
- **FR-014**: 033 FR-032 says: "While an aid runs or after a check it SHALL show the
  settled reading in the Display size". This spec SHALL show it, and the lines that go
  with it, also while the aid looks for the region again (`finding`) with `readings`
  above 0; with none, as before.
- **FR-015**: The Focus block SHALL write a record part that has `alt_deg` as its size
  followed by ` at N°`, N the altitude to the nearest degree, and one without it, while
  the aid's `last` has an `alt_deg`, as its size followed by ` with no altitude`; when
  the reference has `carried`, the verdict line SHALL name the record as `S at F°`
  followed by ` tonight` or ` from earlier nights` and `, as it reads at A°: V`, S the
  record's own size, F its altitude, A the reading's and V the reference's value, the
  percentage still against V; without them every line reads as before.
  `crates/finder/web/src/api.ts` SHALL gain the optional `alt_deg` of a record part and
  of `last`, and `carried` (owner approval 6).

### Key Entities

- **A reading's altitude**: the geometric altitude of Track's target at the middle of
  the reading's exposure, from the session's site, while Track is tracking.
- **The record's altitudes**: one for tonight's part and one for the earlier nights',
  each the altitude of the reading that made it, when known.
- **The carried reference**: the record's part as it judges a reading at the reading's
  altitude, its size times the ratio of the two airmasses to the power 0.6, that factor
  held between 1 and 1.30.
- **The region an aid left**: the region in force when a put-back failed, and that
  put-back's words, kept on the focus state until a put-back takes, the camera is
  described again or the observer changes the settings.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario has a committed test that asserts it, and the
  `lint` and `test` gates pass.
- **SC-002**: No existing test, golden, fixture, `scripts/ci/` file, `.github/` file,
  `CODEOWNERS` entry or `ergane.yaml` is edited; every existing test passes unchanged
  (plan.md § Shapes that existing tests pin). The only edits to existing test files are
  the `mod` lines of `crates/finder/tests/console/main.rs` and
  `crates/finder/tests/motion/main.rs`.
- **SC-003**: Going on anyway from a pause with the aid's exposure in flight goes on to
  the stack on the whole sensor when the camera takes its settings back within 5 s
  (US1-S1), and otherwise ends naming the put-back in the run, the view and MCP
  (US1-S2), also after the pause was answered with `Check focus again` (US1-S6); the
  aid stopped alone does the same (US1-S3), and no later aid or check keeps the region
  it left (US1-S4).
- **SC-004**: On the field night's numbers, 6.7″ at 37° and 6.4″ at 38° against 4.6″
  read at 74°, the run goes on (`slightly_soft` and `good`, plan.md § What was run;
  US2-S1 pins 6.4″ and a reading well inside `slightly_soft`), and in the rig a check
  while Track is tracking is judged against the record carried to the target's
  altitude, and one while Track is armed against the record as it is (US3-S1).
- **SC-005**: The Imaging page's picture stays between previews, saying it is the last,
  and the Focus block's readings while the aid looks for its region again (US4-S1,
  US4-S2).

## Assumptions

- **Earlier features are in place.** Every spec in `depends_on_landed` has landed. Every
  citation here was read at f6d4f5fd.
- **The simulated camera's refusal stands for the real one**: a camera that refuses a
  settings change for a while after an abort, with the driver's words. INFERRED from the
  field night, which never read `restore_error` (plan.md § Outside the factory, step 3,
  reads it).
- **Seeing makes part of the record's star size**, which grows with the airmass; how
  large a part is not measured, so the carry is held between 1 and 1.30 (owner decision
  2; plan.md § Risks).
- **Out of scope:**
  - the aid's measure (ASTAP) and the starless wording (owner decision 4);
  - the dark check, frame saving, 041's region refusal itself (decision 24 stands),
    detection (052's) and the sync model (053's);
  - the scope node, the INDI adapter and the scope link's protocol (owner decision 1
    (b) is set aside);
  - MCP's tool descriptions (owner decision 5);
  - the field check of all this, which is the owner's (plan.md § Outside the factory).
- **The factory's gates and CI:** the gates are `lint` and `test`, copied in plan.md
  § Verification. CI's other jobs run only in the merge queue, so no scenario's Then
  clause rests on them.

## Work Graph

```yaml
US1:
  depends_on: []
  implements: [FR-001, FR-002, FR-003, FR-004]
US2:
  depends_on: []
  depends_on_merged: [US1]
  implements: [FR-005, FR-006, FR-007, FR-008]
US3:
  depends_on: []
  depends_on_merged: [US2]
  implements: [FR-009, FR-010, FR-011, FR-012]
US4:
  depends_on: []
  implements: [FR-013, FR-014, FR-015]
```

Four stories, each under 60 % of 65,536 B (39,321 B) as the probe's commits measure
them, the largest US1 at about 48.8 % (plan.md § Plan traps 1): the put-back and its
words; the record's altitude and the carry; judging at Track's altitude; the page. At
the factory's worst overrun seen, 1.35 times, US1 would be about 66 %, over the line;
plan.md § Plan traps 1 names the split to make if a build runs over. US2 merges after
US1, so that the two stories' `mod` lines in `crates/finder/tests/console/main.rs` are
not written at once; US3 merges after US2, whose `carry` and `note_at` it calls, and so
after US1, which also writes `crates/finder/src/imaging/focus/run.rs` and the README.
US4 shares no file with them and calls none of their code: its tests answer the views
they write, so it runs beside them.
