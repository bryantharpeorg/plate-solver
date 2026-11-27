---
state: draft
depends_on_landed: ["013-live-finder", "015-tracking", "016-mcp", "031-imaging-frames-on-console", "033-focus-reading-and-aid", "035-live-stack", "037-imaging-run", "041-imaging-run-on-the-rig", "042-imaging-run-page-and-mcp", "043-imaging-abort-answers-in-time", "050-tools-page-aligns-finder-camera", "051-tools-page-focuses-either-camera"]
# Drafted 2026-11-27 against `main` at f6d4f5fd, at the owner's ask of 2026-11-27
# ("Your rec. Proceed", the owner's answer to the operator seat's ranked list of the
# defects of the field night of 2026-11-26), from a probe of that day on the branch
# `probe/054` cut from f6d4f5fd (tip 6ca3d283, every gate passing; plan.md § What was
# run and what was read). Every `path:line` here was read at f6d4f5fd. Not yet
# reviewed, not yet probe-built from this text, and none of the five owner decisions
# answered: each carries its recommendation, and the scenarios and the record are
# written to the recommendations. 013 is named for the simulator and the live loop the
# rigs run on; 015 because a reading's altitude is Track's target's; 016 for MCP; 031
# for the imaging camera's lease and the Imaging page; 033 for the focus aid, its
# put-back, record and verdict and the Focus block; 037 and 041 for the imaging run,
# its focus gate and pause and its region refusal; 043 because its silent-node bound
# must hold; 035, 042, 050 and 051 because each last changed a file this spec writes
# (plan.md § Refinement Record).
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

### Session 2026-11-27 (the probe on `main` at f6d4f5fd)

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
  (`crates/finder/src/imaging/run/console.rs:378`, 041 owner decision 24). On the real
  camera the put-back came right after the abort: the scope node's INDI adapter answers
  an abort at once (`crates/scope-node/src/imaging/indi/mod.rs:1102`), and a settings
  change is the driver's to refuse (`the camera refused {property}: …`,
  `crates/scope-node/src/imaging/indi/mod.rs:1435`), which the console words as `the
  scope node did not apply the settings: …` (`crates/finder/src/imaging/mod.rs:713`).
  The simulated imaging camera takes settings at once after an abort
  (`crates/simulator/src/imaging.rs:750`, `crates/simulator/src/imaging.rs:834`), so no
  test could see it. INFERRED, not run: that the field camera refused the put-back; the
  field logs never read `restore_error`.
- Q: Was it reproduced? → A: Yes, run on the factory's rig: with the simulated imaging
  camera refusing every settings change for 1 s after an abort, going on anyway from a
  pause with the aid's exposure in flight ends the run `camera_failed` with 041's words,
  the aid's `restore_error` `the scope node did not apply the settings: the camera
  refused the settings: it is still stopping an exposure` and the aid's focus region in
  force (plan.md § What was run and what was read).
- Q: Does 041's region refusal change? → A: No. A start or a re-hold with a region in
  force is still refused (041 owner decision 24); when the aid's put-back failed, the
  words name that instead of telling the observer to clear a region they never set.
- Q: Why did the check call M42's stars clearly out? → A: The verdict compares the
  reading with the record, `good` up to 1.10 times it and `slightly_soft` up to 1.30
  times (`crates/finder/src/imaging/focus.rs:224`, `crates/finder/src/imaging/focus.rs:227`,
  `crates/finder/src/imaging/focus.rs:261`), and the record holds no altitude
  (`crates/finder/src/imaging/focus.rs:397`). Seeing widens a star with the air it is
  seen through; 6.7″ at 37° and 6.4″ at 38° are 1.457 and 1.391 times a record of 4.6″
  read at 74°, both `clearly_out` (run, plan.md § What was run).
- Q: What blanks on the Imaging page? → A: The picture: the page draws it only while the
  latest frame has a preview (`crates/finder/web/src/components/ImagingPage.tsx:271`),
  a new frame starts `exposing` without one (`crates/finder/src/imaging/frames.rs:178`)
  and gets it once saved (`crates/finder/src/imaging/frames.rs:468`), and the page reads
  the view every second (`crates/finder/web/src/useImaging.ts:17`), so with the aid's
  1 s frames the picture is gone about half the time. The Focus block also hides its
  readings while the aid looks for the region again
  (`crates/finder/web/src/components/FocusBlock.tsx:184`).
- Q: What of the starless wording and the unit? → A: Owner decisions 4 and 5. The local
  ASTAP already answers a starless frame in a form the adapter reads as too few stars;
  the page already writes ″ or px, and MCP's tool descriptions are pinned by tests.

## Owner decisions

None is answered yet. Each keeps its options and its evidence, with a recommendation;
the scenarios and the record are written to the recommendations. Two general rules
carry the recommendations and remove the side effects a narrower fix would bring:
**whatever the focus aid sets, it gives back before anyone else holds the camera, or
says why it could not; and a reading is compared with the record at one altitude, the
reading's own.** The first puts the whole fix in the aid's one put-back, which every
way an aid ends goes through (a stop, a check that is done, a failure, a pause
answered either way, the Focus block's own stop), so the run remembers nothing, 037's
step traits and 041's refusal are not changed, and no answer to a pause is special.
The second changes neither 033's thresholds nor its verdicts, adds no record per
altitude, and leaves every record without an altitude judging as it does today.

1. **The put-back after an abort** (FR-002, FR-003). Recommended (a): **the aid's
   put-back tries again, once a second for up to 5 s, while the camera is ready, and
   041's refusal names a put-back that never took.** Evidence (run, plan.md § What was
   run): on main's code all four of US1's tests fail; with (a), a camera that refuses
   for 1 s after an abort lets the run stack on the whole sensor with no
   `restore_error`, and one that refuses for 10 s ends the run `camera_failed` with
   `the focus aid could not put the imaging camera's settings back: the scope node did
   not apply the settings: the camera refused the settings: it is still stopping an
   exposure`. The aid stopped alone answers 1.0 s and 5.0 s after the stop in paused
   time. 043's silent node still answers within its bound: a refusal while the camera
   is not `ready` is not tried again (the mutation that retries regardless fails 043's
   US1-S8 test). 1 s is the INDI adapter's own pause before it tries an exposure again
   (`crates/scope-node/src/imaging/indi/mod.rs:45`); 5 s is 043's `ABORT_WAIT`. Instead:
   (b) **the retry alone**: the 1 s case passes, but a camera slower than 5 s still ends
   the run with 041's words, which tell the observer to clear a region they never set
   (the mutation that drops the naming fails US1-S2); (c) **the naming alone**: the
   run still ends whenever the camera is slow to take settings after an abort, which on
   the field night was every time (US1-S1 fails); (d) **the run re-applies its own
   settings** after the aid closes. Trial-built (run, `probe/054` 5e5b5b84, set aside in
   2664feef): the run's focus part remembers the settings in force when it opens the
   aid and, when the aid left a `restore_error`, takes the lease as the imaging run and
   sets them back with the whole sensor. Tried once, right after the aid's refusal, it
   is refused too, the camera still stopping, and the run ends `camera_failed`; tried
   up to five times a second apart, the run stacks on the whole sensor, but the aid's
   `restore_error` stays in the view, so the Focus block says the settings were not put
   back when they were, and the aid stopped alone, from the Focus block or
   `set_focus_aid`, still leaves its region (US1-S3 fails both ways). It needs memory
   in the run that (a) does not, and a retry all the same; (e) **the scope node waits
   after an abort** before it answers: a change to the scope node and the INDI adapter,
   with a wait inferred from one night and untestable on the factory's rig without a
   driver, and a console that still fails the same way on a camera slower than the wait.
2. **The verdict and the altitude** (FR-005 to FR-012). Recommended (a): **the record
   keeps the altitude it was read at, and a reading is judged against the record
   carried to its own altitude by the airmass to the power 0.6**, the altitude being
   Track's target's, from the session's site, at the middle of the reading's exposure;
   with either altitude unknown (Track off, no site, a record made before this spec)
   the record judges as it is. Evidence (run, plan.md § What was run): the field night's
   record of 4.6″ at 74° carries to 6.092″ at 37° (a factor of 1.324), so 6.7″ there is
   1.0998 times it, `good` by 0.02 % (6.75″ would be `slightly_soft`), and to 6.010″ at
   38°, so 6.4″ is 1.065 times, `good`; at 37° the record then reads `good` up to 6.70″
   and `slightly_soft` up to 7.92″. Kasten and Young's airmass in place of the secant
   moves the carried size by under 0.1 %, which is enough to make the 6.7″ reading
   1.101 times, `slightly_soft`: the field night's first reading sits on the line
   either way, its second well inside it. In the rig, a record of 2.0 px at 74° is
   carried to 2.458 px at Track's target's altitude, within 0.5° of the simulator's own,
   and a reading of 2.826 px is judged `slightly_soft`, where uncarried it would be
   `clearly_out`. The exponent is seeing theory's, a star's size in seeing growing as the
   airmass to the power 3/5; no local source gives it (plan.md § Constitution Check, I
   and V). The pause sentence then names the carried size as the record
   (`crates/finder/src/imaging/run/console.rs:562` passes the reference); naming the
   carry there would need a field on 037's `FocusCheck`, which tests build by literal
   (plan.md § Plan traps), so the Focus block beside it names both altitudes. Instead:
   (b) **a record per altitude band** (20° to 40°, 40° to 60°, 60° to 90°): M42 at 37°
   has no record in its band, so the check answers `no_record` and the run pauses all
   the same until the owner makes one there; (c) **a note only**: the page names both
   altitudes and the verdict stays `clearly_out` at 1.457 and 1.391 times; (d) **looser
   thresholds**, at every altitude: a real focus error at the record's own altitude
   passes as well.
3. **What the view shows between previews** (FR-013, FR-014). Recommended (a): **the
   Imaging page keeps the last picture it drew until the next preview, and the Focus
   block keeps its readings while the aid looks for the region again.** Evidence (run):
   on main's page three of the four new Vitest tests fail; with (a) they pass, the whole
   Vitest suite and `CI=1 npm run e2e`, its red-only night check among it, unchanged.
   Instead: (b) **a `last_preview` in the frame view**: a change to 031's view, read by
   the page, MCP and their tests, for what the page alone needs (read, not run); (c) **a
   picture in the Focus block**: a second picture of the same frame on the same page;
   (d) **leave it**.
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
   far-out limit is about 11″. The page already writes every size with ″ or ` px`
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
   `focus::carry`, `focus::Altitudes`, `Kept::note_at`, `Kept::reference_at`,
   `Kept::altitudes` and `focus::run::CarriedView`; the fields `SizeAt::alt_deg` and
   `ReferenceView::carried`. No existing signature changes: `Kept::note` and
   `Kept::reference` keep theirs.
3. **`focus.json`** (FR-008): the optional `record.tonight.alt_deg` and
   `record.earlier_alt_deg`, its `format` still `imaging-focus/1`.
4. **041's refusal's second form** (FR-003): `the focus aid could not put the imaging
   camera's settings back: …`, when the aid's put-back failed; 041's words otherwise.
5. **The README's sentences** (FR-004, FR-012), in `## Focus` and `## An imaging run`.
6. **The page** (FR-013 to FR-015): the kept picture, the readings kept while the aid
   looks for the region again, and the words ` at N°` and `, as at A° from S at F°`.

## User Scenarios & Testing *(mandatory)*

Every scenario shares these definitions.

- **The gate commands.** Every Rust scenario runs in
  `cargo nextest run --workspace --all-targets --all-features --locked`, the `test`
  gate's first command, each test in its own process. The page's Vitest scenarios run
  in `npm test`, a later command of the same gate, in `crates/finder/web`.
- **The pause rig** is the console rig of
  `crates/finder/tests/console/imaging_run_focus.rs` (its `rig`,
  `crates/finder/tests/console/imaging_run_focus.rs:363`) with the console's own camera
  and focus parts: a simulator on `crate::support::live::DB` with `Site::default()` and
  a `VirtualClock` from `crate::support::live::START`, pushed to altitude 40° and
  azimuth 0°, focus error 0; its imaging camera
  `SimImagingCamera::new(sim).with_abort_settle(settle)`; a `SimNode::from_cameras` with
  the token `test-token` and a 500 ms frame interval, reached over an in-memory duplex;
  the session's node and site as that rig sets them
  (`crates/finder/tests/console/imaging_run_focus.rs:393`); `AppState::shared` with
  `with_session`, `with_clock` of the simulator's clock, `with_link`, `with_live()`,
  `with_imaging`, `with_focal_length_mm(1800.0)`, the simulated imaging solver with
  `with_measure_noise(7, 0.02)`, and `with_imaging_run_steps` of a `Script`'s steps whose
  camera and focus are `console::camera` and `console::focus`; served by
  `crate::support::mcp::app(state, "127.0.0.1")`, so that `/mcp` answers too. The tests
  run under `#[tokio::test(start_paused = true)]`, read `GET /api/imaging` every 100 ms,
  and every state awaited comes within 300 s of paused time.
- **Whole** means that `settings.roi` of `GET /api/imaging` has `x` 0, `y` 0 and the
  `camera`'s `width` and `height`.
- **Going on anyway mid-exposure** is, on a console with no focus record: wait for
  `state` `ready`; `POST /api/imaging/run` with `{}`, answered 200; wait until
  `run.state` is `focus_pause`, `focus.aid.state` `running` with `readings` at least 1,
  `frame.state` `exposing` and `frame.purpose` `focus`, and check there that
  `run.focus.verdict` is `no_record` and the settings are not whole; then
  `POST /api/imaging/run/continue` with `{"anyway":true}`, answered 200; then wait until
  `run.state` is `stacking` or `ended`.
- **STILL** is `the camera refused the settings: it is still stopping an exposure`, the
  simulator's `STILL_STOPPING`; **REFUSED** is `the scope node did not apply the
  settings: ` followed by STILL.
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
  **carried(P, F, A)** is P·(X(A)/X(F))^0.6.
- **The page rig** is the component rendered by Testing Library with `fetch` stubbed to
  answer the views the test writes, as the page's existing tests do, each locator held
  to its block's region (`Imaging frame` or `Focus`).
- **Oracles.** The words of 041's refusal, of the simulator's refusal and of these
  scenarios; 031's own view of the settings and the camera; the airmass written in the
  test, the exponent 0.6 written in the test, and 033's thresholds 1.10 and 1.30; the
  field night's numbers; the simulator's own altitude; and the views the page tests
  write. No expected value comes from the code under test: no test reads
  `RESTORE_PAUSE`, `RESTORE_WAIT` or `SEEING_EXPONENT`, or calls `carry` for an expected
  value.

### User Story 1 - Going on from a focus pause gives the camera back whole (Priority: P1)

As the owner, I need `Go anyway` at a focus pause to go on with the imaging camera on
its whole sensor, and, when the camera will not take its settings back, to be told
that, not to clear a region I never set.

**Why this priority**: Without it no imaging run whose focus check pauses can go on;
on the field night that was every run.

**Independent Test**: `crates/finder/tests/console/imaging_run_focus_restore.rs`, new.
The `test` gate's nextest command runs it; during development,
`cargo nextest run -p finder --locked --all-features --test console -E 'test(imaging_run_focus_restore)'`.

**Acceptance Scenarios**:

1. **Given** the pause rig with a settle of 1 s, **When** a committed test goes on
   anyway mid-exposure, **Then** it asserts that `run.state` is `stacking`, that the
   settings are whole, and that `focus.aid` has no `restore_error`.
2. **Given** the pause rig with a settle of 10 s, **When** a committed test goes on
   anyway mid-exposure, **Then** it asserts that `run.state` is `ended` with
   `run.ended.reason` `camera_failed`; that `focus.aid.restore_error` is REFUSED; that
   `run.ended.message` is `Ended: the imaging camera failed: the focus aid could not
   put the imaging camera's settings back: ` followed by REFUSED and a full stop; that
   MCP's `get_imaging_run` answers that message as
   `result.structuredContent.run.ended.message`; and that MCP's `get_focus_reading`
   answers REFUSED as `result.structuredContent.aid.restore_error`.
3. **Given** the pause rig with a settle of 1 s, and again a fresh one with 10 s, on a
   console with no focus record, the aid started alone with `POST /api/imaging/focus`
   and `{"tile":{"col":1,"row":1}}`, answered 200, and waited for until
   `focus.aid.state` is `running`, `frame.state` `exposing` and `frame.purpose` `focus`,
   with the settings not whole, **When** a committed test sends
   `DELETE /api/imaging/focus` and measures the paused time until it answers, **Then**
   it asserts that each answers 200 with `focus.aid.ended_reason` `stopped`; that at
   1 s it answers 0.9 s to 2.0 s after it was sent, with no `restore_error` and the
   settings whole; and that at 10 s it answers 4.9 s to 6.0 s after it was sent, with
   `restore_error` REFUSED and `settings.roi` as it was while the aid ran.
4. **Given** `crates/finder/README.md`, **When** a committed test reads its `## Focus`
   section, up to the next `## ` heading, and its `## An imaging run` section, up to
   `### Field checklist`, each with its line breaks read as spaces, **Then** it asserts
   that the first contains `is tried again once a second for up to 5 s`, and the second
   `` `the focus aid could not put the imaging camera's settings back:` ``.

### User Story 2 - The focus record keeps the altitude it was read at (Priority: P1)

As the owner, I need the focus record to keep the altitude each of its parts was read
at, and a way to carry a size from one altitude to another, so that a reading can be
compared with the record as it would read at the reading's altitude.

**Why this priority**: The verdict cannot account for the air until the record knows
where it was made.

**Independent Test**: `crates/finder/tests/console/imaging_focus_carry.rs`, new. The
`test` gate's nextest command runs it; during development,
`cargo nextest run -p finder --locked --all-features --test console -E 'test(imaging_focus_carry)'`.

**Acceptance Scenarios**:

1. **Given** `focus::carry` and `focus::judge`, **When** a committed test calls
   `carry(4.6, 74.0, 37.0)`, `carry(4.6, 74.0, 74.0)` and `carry` of the first answer
   from 37° back to 74°, and judges the field night's readings, **Then** it asserts that
   the first is within 10⁻¹² of carried(4.6, 74, 37) and within 5·10⁻⁴ of 6.0924; that
   the second is 4.6; that the third is within 10⁻¹² of 4.6; that `judge(Ok(6.7),
   Some(4.6))` and `judge(Ok(6.4), Some(4.6))` are `Verdict::ClearlyOut`; that
   `judge(Ok(6.7), Some(carry(4.6, 74.0, 37.0)))` and `judge(Ok(6.4), Some(carry(4.6,
   74.0, 38.0)))` are `Verdict::Good`; and that `judge(Ok(8.0), Some(carry(4.6, 74.0,
   37.0)))` is `Verdict::ClearlyOut`.
2. **Given** a `Kept` on a `focus.json` in a scratch folder, described with the
   simulated imaging camera, and T = 1,768,528,800 s, **When** a committed test calls,
   in order, `note_at(3.0, T, None, false, Some(74.0))`, `note_at(2.9, T + 60, None,
   false, Some(60.0))`, `note_at(3.1, T + 120, None, false, Some(30.0))`, a second
   `Kept` on the same file, `note_at(3.2, T + 43260, None, false, None)` and
   `note(2.5, T + 43300, None, false)`, reading `reference_at`, `altitudes` and the file
   between them, **Then** it asserts that the first answers true, `reference_at(T + 1,
   None)` then answers `(3.0, Of::Tonight, Some(74.0))` and the file's
   `record.tonight.alt_deg` is 74; that the second answers true and the third false,
   and `altitudes(T + 121, None).tonight` is `Some(60.0)`; that the second `Kept`
   answers `reference_at(T + 121, None)` as `(2.9, Of::Tonight, Some(60.0))`; that the
   note a night later answers true, `reference_at(T + 43261, None)` then answers
   `(2.9, Of::Earlier, Some(60.0))`, the file's `record.earlier_alt_deg` is 60 and its
   `record.tonight` has no `alt_deg`; and that the last answers true and
   `reference_at(T + 43301, None)` answers `(2.5, Of::Tonight, None)`.
3. **Given** a fresh `Kept` on another file, described with the same camera, and 033's
   own `focus.json` written by hand with `record.tonight` of `px` 3.3 and no altitudes,
   **When** a committed test calls `note_at(3.0, T, Some(0.55), false, Some(74.0))` on
   the first and reads its `altitudes` at the scales 0.55 and 1.1, then
   `note_at(5.0, T + 60, Some(1.1), false, None)`, and describes a third `Kept` on 033's
   file, **Then** it asserts that the first note answers true; that `altitudes(T + 1,
   Some(0.55)).tonight` is `Some(74.0)` and `altitudes(T + 1, Some(1.1))` is
   `Altitudes::default()`; that the note at 1.1 answers true and
   `reference_at(T + 61, Some(1.1))` answers `(5.0, Of::Tonight, None)`; and that the
   third answers `reference_at(T, None)` as `(3.3, Of::Tonight, None)` and
   `reference(T, None)` as `(3.3, Of::Tonight)`.

### User Story 3 - The aid and the check judge at Track's altitude (Priority: P1)

As the owner, I need each focus reading to carry the altitude of Track's target, the
record to keep it, and a check to judge against the record carried to that altitude,
so that a target low in the sky is not called out of focus for the air alone.

**Why this priority**: This is what turns the field night's `clearly_out` into the
verdict the optics deserve.

**Independent Test**: `crates/finder/tests/motion/imaging_focus_altitude.rs`, new. The
`test` gate's nextest command runs it; during development,
`cargo nextest run -p finder --locked --all-features --test motion -E 'test(imaging_focus_altitude)'`.

**Acceptance Scenarios**:

1. **Given** the motion rig with a record by hand of 2.0 px at 74°, and Track on,
   **When** a committed test makes a check and reads the true altitude, **Then** it
   asserts that `focus.aid.ended_reason` is `done`; that `focus.aid.reference.carried`
   has `from_alt_deg` 74, `alt_deg` within 0.5° of the true altitude and `from` within
   10⁻⁹ of 2.0·s; that `focus.aid.reference.value` is within 10⁻⁹ of
   carried(2.0·s, 74, that `alt_deg`), relatively; that `focus.aid.verdict` is `good`
   when `focus.aid.median.value` is at most 1.10 times that value, `slightly_soft` when
   at most 1.30 times and `clearly_out` beyond, and is `slightly_soft`; that the median
   is above 1.30·2.0·s, so that the record uncarried would call it `clearly_out`; and
   that `focus.record.tonight.alt_deg` is 74.
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
   that the first contains `a star's size grows as the airmass to the power 0.6`,
   `judged against the record carried to its own altitude` and `the far-out limit is
   about 11″`, and that the second contains `against the record carried to the target's
   altitude`.

### User Story 4 - The Imaging page keeps its picture and readings, and names the altitudes (Priority: P2)

As the owner, I need the Imaging page to keep the last picture until the next one is
ready, the Focus block to keep its readings while the aid looks for its region again,
and both lines that compare with the record to say at what altitudes.

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
   1000 ms` the image `Imaging frame 1` is still there; and that then the image
   `Imaging frame 2` shows `/api/imaging/frames/2/preview.jpg` and `Imaging frame 1` is
   gone.
2. **Given** the page rig with `FocusBlock`, the unit `arcsec` at 0.55″ a pixel and an
   aid `finding` with 9 readings, the median, last and best 6.7, **When** a committed
   test renders it, **Then** it asserts, within the region `Focus`, the text `6.70″`
   and the text `Finding the focus region on a full imaging frame`.
3. **Given** the page rig with `FocusBlock` and an aid `finding` with 0 readings and no
   median, last, best or history, **When** a committed test renders it, **Then** it
   asserts that the region `Focus` has no text `Focus reading`.
4. **Given** the page rig with `FocusBlock`, the record's `tonight` of 4.6 at `alt_deg`
   74, and an aid `running` with the median 6.7, the verdict `good` and the reference
   of 6.09 `tonight` `carried` from 4.6 at 74° to 37°, **When** a committed test renders
   it, **Then** it asserts, within the region `Focus`, the text `Focus record: 4.60″ at
   74° tonight.` and the text `Within 10% of the focus record of 6.09″ tonight, as at
   37° from 4.60″ at 74°.`

### Edge Cases

- **The camera refuses for longer than 5 s**: the aid's `restore_error` is the last
  refusal's words, the camera keeps the aid's region, and the run ends naming the
  put-back (US1-S2); `Use the whole sensor` on the Focus block, or
  `PUT /api/imaging/settings` with `{"roi":null}`, clears it (033 FR-033, unchanged).
- **The scope node goes silent or the link drops during the put-back**: the camera is
  not `ready`, the refusal is the last at once, and 043's bound holds (043 US1-S8, its
  test unchanged; run).
- **A refusal for another reason while the camera is ready** (a setting out of range):
  tried again for up to 5 s, then named as it is today; the stop answers up to about
  5 s later than at f6d4f5fd. Read, not run.
- **Anyone else asking for the camera during the retries**: refused with the aid's name,
  as during the aid (033 FR-027, 031's lease); the run's own re-hold comes only after
  the aid has ended.
- **Track off, no site, the target at or below the horizon, or a record made before this
  spec**: no altitude, no carry, the verdict as at f6d4f5fd (US2-S2, US3-S2; the
  existing tests of 033 and 041 pass unchanged; run).
- **Track's target and where the telescope points**: the altitude is the target's,
  which Track holds; in the rig it is within 0.5° of the simulator's own (US3-S1).
- **Very low targets**: the carry grows fast near the horizon (1.859 times from 74° to
  20°), so a reading nearly twice the record reads `good` at 20° (plan.md § Risks).
- **A record kept across nights**: the earlier nights' part keeps its altitude (US2-S2).
- **The pause sentence**: it names the carried size as the record; the Focus block
  names both altitudes (owner decision 2).

## Requirements *(mandatory)*

### Functional Requirements

**The put-back** (US1):

- **FR-001**: `SimImagingCamera` SHALL gain `with_abort_settle(settle: Duration)`,
  zero by default, and `crates/simulator/src/imaging.rs` the public `STILL_STOPPING`,
  `the camera refused the settings: it is still stopping an exposure`. For `settle`
  after each abort of an exposure in flight, on tokio's clock, its `apply` of any
  settings other than `ImagingSettings::default()` SHALL answer `ImagingError::Failed`
  with `STILL_STOPPING`, after its exposing check and before any setting changes; at
  zero it SHALL refuse none, so that every existing caller sees f6d4f5fd's camera.
  Nothing else in `crates/simulator` SHALL change (owner approval 1).
- **FR-002**: 033 FR-027 says: "However it ends, it SHALL wait out an imaging frame in
  flight, put the remembered settings back, reporting a failure to do so as
  `restore_error`, and then release the lease, so that the next taker is not refused."
  This spec keeps that and SHALL make the put-back so: a try the camera refuses while
  the imaging camera's state is `ready` SHALL be followed by another `RESTORE_PAUSE`
  (1 s) later, so long as that try starts within `RESTORE_WAIT` (5 s) of the first; the
  first try that takes leaves no `restore_error`, and otherwise the last refusal's words
  SHALL be `restore_error`. A refusal while the camera is not `ready` SHALL be the last
  at once. Only then SHALL the lease be released and the aid end, and
  `DELETE /api/imaging/focus` (033 FR-028) answers then. `RESTORE_PAUSE` and
  `RESTORE_WAIT` SHALL be public constants of `crates/finder/src/imaging/focus/run.rs`
  (owner decision 1 (a), owner approval 2).
- **FR-003**: 041 FR-021 says that holding the lease the console's camera part "SHALL
  refuse … with `the imaging camera has a region of interest set: clear it before an
  imaging run` while the settings hold one (owner decision 24)". This spec keeps the
  refusal and SHALL word it `the focus aid could not put the imaging camera's settings
  back: ` followed by the imaging view's `focus.aid.restore_error` when the latest aid's
  view carries one, and with 041's words otherwise. 037 FR-002's "SHALL end
  `camera_failed` with the camera part's words when it cannot" then gives the run's
  `ended.message` `Ended: the imaging camera failed: ` followed by those words and a
  full stop, and MCP's `get_imaging_run` and `get_focus_reading` carry the same words,
  their tools and descriptions unchanged (owner approval 4).
- **FR-004**: `crates/finder/README.md` SHALL say, in `## Focus` after the sentence
  that ends `puts the settings back and releases the camera.`, that a put-back the
  camera refuses while it is ready is tried again once a second for up to 5 s, that one
  that never takes is the aid's `restore_error`, and how the region it leaves is
  cleared; and, in `## An imaging run` after `the page shows its time left.`, that every
  answer to a pause closes the aid and that a put-back that failed ends the run
  `camera_failed` after `` `the focus aid could not put the imaging camera's settings
  back:` ``, each in the words of plan.md § Interfaces. The field checklist keeps its
  18 steps (owner approval 5).

**The record's altitude** (US2):

- **FR-005**: `crates/finder/src/imaging/focus.rs` SHALL gain `SEEING_EXPONENT`, 0.6,
  and `pub fn carry(px: f64, from_alt_deg: f64, to_alt_deg: f64) -> f64`, the size `px`
  read at `from_alt_deg` times (X(`to_alt_deg`)/X(`from_alt_deg`))^`SEEING_EXPONENT`,
  where X(h) is 1/sin h (owner decision 2 (a), owner approval 2).
- **FR-006**: 033 FR-011 says: "`Kept::note` SHALL take a settled reading and whether
  the aid reports `found` at it: with no record of tonight, or one whose night began
  more than 12 hours before the reading, it starts a night with that reading, the old
  night's record joining the earlier nights' if it is the smaller; otherwise it replaces
  tonight's record only when smaller." This spec keeps that and SHALL add
  `Kept::note_at(px, at, scale, passed, alt_deg: Option<f64>) -> bool`, which notes as
  `note` does and keeps `alt_deg` with the part the reading makes: a reading that starts
  a night or replaces tonight's record sets tonight's altitude to `alt_deg`; a night's
  record that joins the earlier nights' brings its altitude with it; a note that changes
  no size changes no altitude. `Kept::note` SHALL be `note_at` with no altitude, its
  signature unchanged, and a record the foreign rule (033 FR-013) drops SHALL drop its
  altitudes with it.
- **FR-007**: `Kept::reference_at(now, scale) -> Option<(f64, Of, Option<f64>)>` SHALL
  answer `Kept::reference` (033 FR-012, unchanged) with the altitude of the part it
  names, and `Kept::altitudes(now, scale) -> Altitudes` the altitude of each part as
  `Kept::record` shows it at `now`: tonight's while its night began at most 12 hours
  ago, the earlier part's that of whichever record shows there, and none at a foreign
  scale. `Altitudes` SHALL have `tonight` and `earlier`, each an `Option<f64>`, and
  `of(Of)`.
- **FR-008**: 033 FR-014 says: "The record SHALL be written to `focus.json` with the
  focus region, neither write losing what the other wrote, and read back when the camera
  is described." This spec SHALL write, while known, tonight's altitude as `alt_deg` in
  `record.tonight` and the earlier nights' as `record.earlier_alt_deg`, each left out
  when unknown, with the `format` still `imaging-focus/1`; a file without them SHALL
  read as before, with no altitudes (owner approval 3).

**Judging at Track's altitude** (US3):

- **FR-009**: A focus reading's altitude SHALL be the altitude of Track's target, from
  the session's site, at the middle of the reading's exposure (033's `at`), computed
  with `crate::sky::alt_az`, while the session's Track is on (its `state` not `off`) and
  a site is set; and none otherwise, or when it is at or below 0°. The session SHALL be
  read with the imaging state's lock released (plan.md § Interfaces). The mount, the
  live solve and `crates/finder/src/{model,goto,live,session}.rs` are read through
  `SessionView` only, and not changed.
- **FR-010**: 033 FR-025 says: "While the aid runs, its verdict SHALL be `judge` of each
  settled reading, or of an unmeasurable frame, against the reference (FR-012) as it
  stood before that reading was noted; the reference SHALL be kept with the verdict".
  This spec SHALL make that reference the record's part, carried by `carry` from the
  part's altitude to the reading's when both are known, and the part as it is
  otherwise; `judge` (033 FR-010) and its thresholds SHALL be unchanged; and each
  settled reading the aid notes (a check notes none) SHALL be noted with `note_at` and
  its altitude. The run's pause sentence (`crates/finder/src/imaging/run.rs:458`) SHALL
  be unchanged, the record it names being that reference.
- **FR-011**: 033 FR-029 says that the focus view SHALL carry "`unit` (`arcsec` with a
  scale, else `px`), `arcsec_per_px`, `region`, `record` and `aid`, with every size in
  that unit". This spec SHALL add to each record part (`SizeAt`) its `alt_deg` when
  known, and to the aid's `reference` its `carried`, `{"from", "from_alt_deg",
  "alt_deg"}`, when it was carried: the record's own size in the view's unit, its
  altitude and the reading's. Both SHALL be left out otherwise (owner approval 2).
- **FR-012**: `crates/finder/README.md` SHALL say, in `## Focus` after the sentence that
  ends `beyond which its star sizes stop being trustworthy.`, that every focus size is in
  the `unit` beside it and that at binning 1 and 0.55″ a sensor pixel the far-out limit
  is about 11″ (owner decision 5 (a)); in `## Focus` after the paragraph that ends
  `never against a fixed number.`, how the record is carried; and in `## An imaging run`,
  that the check is judged against the record carried to the target's altitude; each in
  the words of plan.md § Interfaces (owner approval 5).

**The page** (US4):

- **FR-013**: 031 FR-022 says the page SHALL "show the latest preview as a picture".
  This spec SHALL keep the last picture the page drew, with its own frame's alt text
  `Imaging frame <id>` and hold button, while the latest frame has no preview yet, until
  the next preview (owner decision 3 (a), owner approval 6).
- **FR-014**: 033 FR-032 says: "While an aid runs or after a check it SHALL show the
  settled reading in the Display size". This spec SHALL show it, and the lines that go
  with it, also while the aid looks for the region again (`finding`) with `readings`
  above 0; with none, as before.
- **FR-015**: The Focus block SHALL write a record part that has `alt_deg` as its size
  followed by ` at N°`, N the altitude to the nearest degree, and, when the reference
  has `carried`, the verdict line's `tonight` or `from earlier nights` followed by `,
  as at A° from S at F°`, A the reading's altitude, S the record's own size and F its
  altitude; without them both read as before. `crates/finder/web/src/api.ts` SHALL gain
  the optional `alt_deg` and `carried` (owner approval 6).

### Key Entities

- **A reading's altitude**: the altitude of Track's target at the middle of the
  reading's exposure, from the session's site, while Track is on.
- **The record's altitudes**: one for tonight's part and one for the earlier nights',
  each the altitude of the reading that made it, when known.
- **The carried reference**: the record's part as it would read at the reading's
  altitude, its size times the ratio of the two airmasses to the power 0.6.

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
  (US1-S2); the aid stopped alone does the same (US1-S3).
- **SC-004**: The field night's readings, 6.7″ at 37° and 6.4″ at 38° against 4.6″ at
  74°, judge `good` (US2-S1), and in the rig a check under Track is judged against the
  record carried to the target's altitude (US3-S1).
- **SC-005**: The Imaging page's picture stays between previews and the Focus block's
  readings while the aid looks for its region again (US4-S1, US4-S2).

## Assumptions

- **Earlier features are in place.** Every spec in `depends_on_landed` has landed. Every
  citation here was read at f6d4f5fd.
- **The simulated camera's refusal stands for the real one**: a camera that refuses a
  settings change for a while after an abort, with the driver's words. INFERRED from the
  field night, which never read `restore_error` (plan.md § Outside the factory, step 3,
  reads it).
- **Seeing dominates the record's star size** at the altitudes the owner images at, so
  the airmass carry applies to the whole size; where optics dominate it overstates
  (plan.md § Risks).
- **Out of scope:**
  - the aid's measure (ASTAP) and the starless wording (owner decision 4);
  - the dark check, frame saving, 041's region refusal itself (decision 24 stands),
    detection (052's) and the sync model (053's);
  - the scope node, the INDI adapter and the scope link's protocol;
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
them, the largest US3 at about 34.1 %, and each still under that line at the factory's
worst overrun seen, 1.35 times (US3 about 46.0 %; plan.md § Plan traps 1): the put-back
and its words; the record's altitude and the carry; judging at Track's altitude; the
page. US2 merges after US1, so that the two stories' `mod` lines in
`crates/finder/tests/console/main.rs` are not written at once; US3 merges after US2,
whose `carry` and `note_at` it calls, and so after US1, which also writes
`crates/finder/src/imaging/focus/run.rs` and the README. US4 shares no file with them
and calls none of their code: its tests answer the views they write, so it runs beside
them.
