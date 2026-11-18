---
state: draft
depends_on_landed: ["030-imaging-camera-link", "031-imaging-frames-on-console", "038-track-follows-a-push"]
# Drafted 2026-11-15 against `main` at dbc2b5b (dbc2b5bf15103e59e7166d9b7a79b421f9d96ce4)
# from docs/plans/2026-11-15-specs-030-037-imaging.md, by the grilling session of that
# day. Reviewed once that day by a second agent (§ Clarifications, the third session),
# then compared with the six other trios the series then had by an auditor (the fourth
# session); this revision applies both.
# Every `path:line` in this trio was checked against `main` at 49bb210
# (49bb210fe4860dd98ba24727074e7bd3cc0d7124, 028's US5), the worktree's HEAD when the
# audit was applied; the ten citations that commit moved are re-anchored.
# Probe-built on 2026-11-18 (round 1, every story), on 030's landed code with 031's
# round-1 probe code on it: the round-1 probe fixes are in, and no story measured over
# the budget (plan.md § Refinement Record). It builds on code that
# 030-imaging-camera-link and 031-imaging-frames-on-console created; every interface of
# theirs named here was taken from their plan.md as drafted and read again on `main`
# after both landed.
# Re-anchored 2026-11-18 to `main` at e9687ce (e9687ce911e0c63529c04ad41ac877f2e4174a83),
# where 030's eight stories and 031's nine have landed: every citation was compared line
# for line with 49bb210, and the ones that moved are corrected (plan.md § Prerequisites 2).
# On 2026-11-18 the owner's captures of ASTAP's real output were folded in, and
# fixtures/finder/astap/ committed with them (plan.md § Prerequisites 5).
# It stays draft until the owner settles the owner decisions and flips it to `ready`,
# which gives the approvals listed under "Owner approvals" and dispatches it.
---

# Feature Specification: Solving Imaging Frames and the Measured Aim Point

**Feature Branch**: `032-imaging-solve-and-aim-point`

**Created**: 2026-11-15

**Status**: Draft

**Input**: Spec 031 gives the console imaging frames, saved as FITS. This spec works out
where each one points, and uses that to relate the two cameras with nothing to tap. The
console asks an imaging solver, through a solver port, for an imaging solve: the J2000
centre of the imaging frame, the angle of its up direction east of north, and its scale.
The imaging solver is the ASTAP program on the console machine; a simulated solver
adapter answers from the simulator's truth in tests and under `finder --simulate`. The
latest live pointing hints the solve, the first search is wide, and the measured scale
is kept; a hint that keeps failing gives way to the next source. Whenever an imaging
solve and a live pointing share a moment, the console measures the aim point (where the
imaging camera's centre falls in the finder camera's frame) and the imaging angle,
smooths them over the last five, and keeps them across restarts; while Track is on, a
newly measured aim point waits until Track is off, so the tracker never chases it.
Tapping a star stays for nights without an imaging camera. The Imaging page
gets a solve block, the live chart shows the imaging field, Setup says how the aim point
was set, and MCP clients get one tool.

## Clarifications

### Session 2026-11-15 (the interview's decisions, from the plan document)

- Q: What does this trio read besides itself? → A: Nothing about the feature. `spec.md`
  holds every requirement and scenario, and `plan.md` holds every design decision, the
  interfaces, the shapes that existing tests pin and the alternatives rejected. The
  binding decisions come from `docs/plans/2026-11-15-specs-030-037-imaging.md`,
  `docs/adr/0002-imaging-cameras-through-indi-frames-relayed-console-stacks.md` and
  `GLOSSARY.md`; facts come from `docs/research/2026-11-15-imaging-camera.md`. A story
  that would change a decision escalates rather than re-deciding it.
- Q: Which solver works out where an imaging frame points? → A: ASTAP's command-line
  program and one of its star databases, on the console machine, behind the solver port.
  It matches quads of stars, so it needs no starting angle, which the first solve after
  the camera is seated cannot supply (ADR-0002;
  `docs/research/2026-11-15-imaging-camera.md` § 3. ASTAP). The finder camera's solver
  stays this project's own. A simulated solver adapter answers from the simulator's
  truth in tests and under `finder --simulate`.
- Q: Why not this project's own solver? → A: It is lost-in-space over a pattern database
  built for fields of 10° to 30° and cannot solve a field under about 3°; an imaging
  frame here is under 1° (the plan document's table of what exists today;
  `docs/research/2026-11-15-imaging-camera.md` § 12. Prior art and what was not
  adopted, which gives what stretching that database to this field would cost).
- Q: How are the two cameras related? → A: By one aim point, with the imaging angle. The
  aim point is where the main telescope looks in the finder camera's frame, now the
  centre of the imaging camera's sensor. The console measures it whenever an imaging
  solve and a live pointing share a moment: after `POST /api/imaging/solve`, and for a
  solve a later spec makes through `Imaging::note_solve` (035: every accepted sub).
  Tapping a star stays for nights without the imaging camera. A second aim point for
  the eyepiece was rejected (ADR-0002).
- Q: Where does the hint for an imaging solve come from? → A: From a list tried in
  order: the live pointing carried through the aim point; the last imaging solve; the
  session's target; none, which means a search of the whole sky. A telescope with no
  finder camera still solves, and the aim point stays optional (the plan document,
  § Mount-neutral by design, rule 4). "Tried in order" is read across solves, since no
  solve is run twice: the first source that exists hints a solve, and after three
  misses in a row the next solve takes the next one (FR-014, owner decision 23).
- Q: Where does the scale come from? → A: From the first imaging solve, kept and reused.
  Before that, from the camera's pixel size and a focal length setting; with neither,
  the solver is asked to find the field height itself (rule 3 of the same section).
- Q: What is binding from "Mount-neutral by design"? → A: No file under
  `crates/finder/src/imaging/` computes alt-az arithmetic or reads axis counts; nothing
  is sized to one telescope; thresholds are in arcseconds or relative to a measured
  value. This governs where a brief says otherwise: the drafting brief asked that "the
  axes" have moved less than a stated amount during an exposure, and this spec judges
  that from the finder camera's live solves around the exposure instead, in arcseconds
  on the sky (owner decision 11).
- Q: Rule 6 of that section says nothing is "fixed in seconds". Which figures here are
  in seconds, and why may they be? → A: Six, none of them a wait for a sub. Three
  bound one run of the solver program on one picture (120 s for a hinted solve, 600 s
  for a search of the whole sky, 30 s for a star measure; owner decision 12): they do
  not grow with the exposure because the program's work does not. Two are measured
  from the exposure's own span, before its start and after its end, so a longer
  exposure widens them by itself: how fresh a live pointing must be to hint (30 s;
  owner decision 8) and how far the fit of the finder camera's live solves reaches
  (10 s, or the exposure if that is longer; owner decision 11). One is how long the
  history of live solves is kept (120 s, or four exposures if that is longer). The
  60″ of travel allowed during an exposure is an angle, not a time. No queue length
  and no count of subs is fixed here: the history is bounded by its span alone and,
  when live solves come faster than 2000 to the span, is thinned rather than cut
  (FR-019), so it always reaches back four exposures.
- Q: Is anything keyed to one camera? → A: No. Sizes, the Bayer pattern and the pixel
  size are read from the camera's description (030). The owner's camera, 4944 × 3284
  pixels of 4.78 µm at 1800 mm, appears only as a worked example.

### Session 2026-11-15 (review of `main` at dbc2b5b, re-checked at 49bb210)

- Q: What was found in the code? → A: Eleven things, each read at dbc2b5b and each
  line re-checked at 49bb210, where 028's US5 had moved ten of them in the tests:
  1. The aim point is `Aim { x, y }`, a durable full-frame pixel of the finder camera
     (`crates/finder/src/session.rs:467`). Only `Session::set_aim` changes it
     (`crates/finder/src/session.rs:1589`), called by the two `/api/aim` handlers
     (`crates/finder/src/api.rs:1026`, `crates/finder/src/api.rs:1078`). A live pointing
     is taken at it (`crates/finder/src/session.rs:1545`) and so is a sync
     (`crates/finder/src/model.rs:268`). Setting it resets nothing in the pointing
     model.
  2. Eleven test files build an `Aim` by literal, most as `Aim { x: 700.0, y: 420.0 }`,
     among them `crates/finder/tests/support/live.rs:73`, and six assertions require the
     session's `aim` to equal exactly `{"x":700.0,"y":420.0}`
     (`crates/finder/tests/console/aim.rs:37`, `crates/finder/tests/cli/aim.rs:198`,
     `crates/finder/tests/cli/aim.rs:209`,
     `crates/finder/tests/mcp/session_tools.rs:219`,
     `crates/finder/tests/console/recording.rs:613`,
     `crates/finder/tests/console/session/durable.rs:61`). So `Aim` gains no field, and
     a tapped aim point still reads as two keys (owner decision 1).
  3. A test requires a session file with a site, a minimum altitude and a target to
     hold exactly three keys (`crates/finder/tests/cli/session.rs:123`). A new stored
     key is therefore left out while it is false.
  4. A solved star frame's projection is `Frame` (`crates/finder/src/sky.rs:35`): it
     projects a sky direction to a pixel, lens distortion included
     (`crates/finder/src/sky.rs:101`), and back (`crates/finder/src/sky.rs:109`). Its
     rotation's third row is the camera's up axis (`crates/finder/src/sky.rs:17`), but
     the field is private, so the imaging angle needs one new method.
  5. A live report carries no projection (`crates/finder/src/live.rs:53`), only its
     mid-exposure time (`crates/finder/src/live.rs:57`), and reports are broadcast 16
     deep (`crates/finder/src/live.rs:163`). The session's latest pointing carries the
     `Frame` (`crates/finder/src/session.rs:537`), and a live pointing is that frame's
     sky direction at the aim point, or at the frame's centre without one
     (`crates/finder/src/session.rs:1548`). The console keeps no history of live
     solves, so this spec adds one.
  6. `finder`'s tokio has no `process` feature (`crates/finder/Cargo.toml:33`), and no
     `Cargo.toml` may change. The ASTAP adapter runs the program with `std::process` on
     the blocking pool.
  7. A test already stands a shell script in for an external program, written into a
     scratch folder with mode 0700 (`crates/scope-node/tests/rpicam.rs:140`). The ASTAP
     adapter's tests do the same.
  8. The simulated finder camera is 1456 × 1088 over 12° with the main telescope at
     pixel (700, 420) (`crates/simulator/src/optics.rs:27`): 29.78″ a pixel. Existing
     tests bound a live pointing at 30″ from the simulator's truth
     (`crates/finder/tests/console/live/loop_.rs:45`), which is one such pixel.
  9. The page's end-to-end checks require the simulated console to start with no aim
     point (`crates/finder/web/e2e/simulate.spec.ts:120`) and Setup to show exactly one
     `status` (`crates/finder/web/e2e/simulate.spec.ts:318`).
  10. The chart draws in the finder frame's pixels, with its reticle at the frame's
      centre and not at the aim point
      (`crates/finder/web/src/components/Chart.tsx:627`); `useSession` copies known
      session fields one at a time and drops the rest
      (`crates/finder/web/src/useSession.ts:30`).
  11. An MCP tool named `solve_frame` already solves an uploaded picture from the
      finder camera (`crates/finder/src/mcp/tools.rs:513`). The new tool is
      `solve_imaging_frame`.
- Q: What does this spec take from 030 and 031? → A: These, by name, from their plan.md
  as drafted; both have landed, each was read again on `main` at e9687ce (plan.md
  § Prerequisites 2), and each is named without a line:
  - `crates/finder/src/imaging/mod.rs` (031 US1): `Imaging`, `ImagingView`, `Refusal`,
    `RoiView`, `CameraView`, and the words a request is refused with while the camera
    is not `ready`; and `AppState::with_imaging` in `crates/finder/src/api.rs` (031
    US1);
  - `crates/finder/src/imaging/routes.rs` (031 US1): the `/api/imaging` routes;
  - `crates/finder/src/imaging/frames.rs` (031 US2): `FrameView`, with a saved frame's
    `id`, `file`, `width`, `height`, `bits`, `bayer`, `binning`, `roi`, `start` and
    `exposure_ms`;
  - `crates/finder/src/imaging/fits.rs` (031 US2 and US3): `FitsImage`, `encode` and
    `decode`;
  - `crates/simulator/src/imaging.rs` (030 US2): `SimImagingCamera`, `ImagingOptics`,
    `ImagingTruth`, `Simulator::imaging_truth_at`, `Simulator::imaging_optics`,
    `Simulator::set_imaging_optics`, `deep_field`, `FIELD_SEED`, `STARS_PER_SQ_DEG`
    (320), `PSF_SIGMA_PX` and `FOCUS_SIGMA_PX`. 030's draft already gives the truth
    the simulated solver adapter answers from, so this spec changes nothing under
    `crates/simulator` (SC-003);
  - `crates/finder/web/src/components/ImagingPage.tsx` and
    `crates/finder/web/src/useImaging.ts` (031 US7), whose Imaging frame block 031's
    own scenarios find as a region named `Imaging frame`;
  - the MCP tools `get_imaging_camera` and `take_imaging_frame` (031 US8).
- Q: Which ASTAP facts were checked, and against what? → A: Against ASTAP's own page on
  2026-11-15 (`docs/research/2026-11-15-imaging-camera.md` § 3. ASTAP): `-f` names the
  picture; `-ra` is in hours; `-spd` is declination + 90°; `-fov` is the picture's
  height in degrees, 0 for automatic; `-r` is the radius of the square search pattern in
  degrees; `-z` downsamples; `-s` caps the stars used, and is not passed here; `-d` is the
  database folder and `-D` its name; `-o` is the base path of the output files; `-analyse` prints `HFD_MEDIAN=` and `STARS=` without
  solving. An `.ini` is always written (`PLTSOLVD`, `CRPIX1/2`, `CRVAL1/2`, `CDELT1/2`,
  `CROTA1/2`, `CD1_1` to `CD2_2`, `CMDLINE`, `WARNING`, or `PLTSOLVD=F` with `ERROR`)
  and a `.wcs` only on success. Exit codes: 0, 1 no solution, 2 not enough stars, 16
  error reading the picture, 32 no star database, 33 error reading it, 34 error updating
  the input. About 30 stars are needed; saturated stars are ignored; a raw colour
  picture needs 2 × 2 binning. The height should be right "within preferably 5%";
  with `-fov 0` ASTAP "will try all FOV between 10 degrees and 0.3 degrees"; and the
  page's own example of the star measure is `-analyse 30`, printing `HFD_MEDIAN=3.3`
  and `STARS=666`. The owner's captures of 2026-11-18 found the installed version
  otherwise in two of these: every failure exits 1, and a solve with no solution writes
  no `ERROR` line; an `.ini` is written in every case, as the page says (the session of
  the owner's ASTAP captures, below).
- Q: What could not be checked? → A: The program itself. The grilling session could not
  run it, so no real output file is in the repository yet (plan.md § Prerequisites 5),
  and five things rest on the saved captures and the field checks: that the `CD`
  matrix refers to rows in the file's own order; that the `.ini` file holds bare
  `KEY=value` lines, without the `//` note the page prints after each; ASTAP's time
  for a hinted solve and for a search of the whole sky; how it measures stars on a
  small region; and how much the finder camera's live solves scatter about a line
  (`docs/research/2026-11-15-imaging-camera.md` § 13. Not established, and § 5.
  Relating two cameras for the one field figure there is). The owner's captures of
  2026-11-18 settled the first two: the row order and the bare lines are as assumed
  (the session of the owner's ASTAP captures, below).
- Q: Where does this trio depart from the plan document? → A: Nowhere now. It first
  departed in eight places, each an owner decision or approval below, and the plan
  document, on `main` since 030's flip (55e41f5), carries all eight with the recommended
  answers; an answer the owner settles otherwise is written back into it in the same
  change (plan.md § Prerequisites 10). In its § Solver port: an imaging solve's
  matched star count is optional, because ASTAP's `.ini` carries none (owner decision
  5); a hint names its source; an imaging solve also keeps its linear part, for 035
  (owner decision 20); the focal length is a console flag (owner decision 6); and the
  ASTAP adapter runs the program on a prepared copy of the saved FITS file in a work
  folder (owner decision 4). In its § Console: the files are `solver.rs`, `astap.rs`,
  `simulated.rs`, `solving.rs` and `relation.rs` (owner approval 7), and the storage
  layout holds `relation.json` and, while an ASTAP call runs, the work folder
  `solver/`, in which each call makes its own `solve-<pid>-<n>/` (owner approval 8).
  In its § Simulator: 032 adds nothing there, because 030's draft already carries
  `ImagingTruth` and nothing under `crates/simulator` changes (SC-003).

### Session 2026-11-15 (a second agent's review of this draft)

- Q: How is "the telescope was still" judged, and why not from two neighbouring live
  solves as first drafted? → A: From a straight line fitted through every live solve
  of the finder camera from 10 s before the exposure to 10 s after it (the exposure's
  own length each way if that is longer). Two neighbouring live solves are half a
  second apart, and one finder solve scatters by 0.3′ to 0.5′ at the telescope
  (`docs/research/2026-11-15-imaging-camera.md` § 5. Relating two cameras): their
  difference is 25″ to 40″ of noise, which the first draft multiplied by exposure over
  interval and read as 100″ or more of travel for a 2 s exposure. Nearly every solve
  would have been refused as `moving`, tracking or not, while every gate passed on the
  simulator's quieter finder. The fitted line divides the same scatter by the square
  root of the summed squared times instead (FR-020, US4-S2, owner decision 11).
- Q: What happens when the kept scale is no longer true: a reducer, a Barlow, or the
  same camera on another telescope? → A: Every hint's field height would be wrong and
  ASTAP wants it within about 5% (the research note, § 3. ASTAP), so every solve
  would fail and the scale would never be corrected. The field height therefore has
  up to three sources, taken in turn: the kept scale, the focal length, and none, with
  which ASTAP finds the height itself. Three solves in a row that end `not_solved` or
  `timed_out` move the console to the next source, and from the last back to the
  first; the first solve that succeeds replaces the kept scale and starts again from
  it (FR-016, US3-S7, owner decision 22).
- Q: Which live pointing hints a solve that is asked for after the telescope has moved
  on? → A: From US4 on, the live solve nearest the exposure's own middle, out of the
  history the console keeps, carried through the aim point; the session's latest
  pointing only when the history holds none (FR-035, US4-S4).
- Q: Does a failed solve go on to the next source of the hint? → A: Not within one
  solve: the first source that exists hints it, and a failed solve is not asked again
  (owner decision 19). This answer was revised in the fourth session: across solves
  the hint does give way, three misses in a row moving it one source down (FR-014,
  owner decision 23). The last imaging solve stops being a source when the session's
  target changes.

### Session 2026-11-15 (an auditor's comparison of the seven trios the series then had)

- Q: Which citations had moved, and against what is the trio now checked? → A: Ten, all
  in test files that 028's US5 (49bb210) rewrote:
  `crates/finder/tests/console/session/durable.rs` 59 and 73 (now 47 and 61),
  `crates/finder/tests/cli/aim.rs` 208, 219 and 227 (now 198, 209 and 217),
  `crates/finder/tests/cli/session.rs` 145 (now 123),
  `crates/finder/tests/console/recording.rs` 445 and 632 (now 426 and 613),
  `crates/finder/tests/mcp/motion.rs` 532 (now 528) and
  `crates/scope-node/tests/rpicam.rs` 129 (now 140). Each now cites the same text at
  49bb210, and so does every other `path:line` of the trio. `Scratch` is on `main`
  there and is cited with its line (`crates/finder/tests/support/mod.rs:12`).
- Q: What if a hint exists but is wrong? → A: As first drafted it never gave way. A
  `pointing` hint further from the truth than its radius (a finder camera more than 3°
  off the main axis before the aim point is measured, or a stale measured aim point
  under the narrow radius) would fail every solve, the search of the whole sky would
  never be reached, and 037's three tries would repeat one hint. The plan document's
  rule 4 says the sources are "tried in order". So the hint takes the turn owner
  decision 22 gives the field height: after three solves in a row that end
  `not_solved` or `timed_out`, the next solve takes the source one place down among
  those that exist, ending at `none`; a solved solve returns both to the first. No
  solve is run twice (FR-014, US3-S8, owner decision 23).
- Q: Does an aim reading come only from `POST /api/imaging/solve`? → A: As first
  drafted, yes: readings were taken only inside `Imaging::solve`'s task, while
  `solving::solve_frame`, which 035 calls for every sub, touches neither the view nor
  the relation. The plan document's § Settled decisions, Two cameras, has the console
  measure "whenever an imaging solve and a live pointing share a moment", so US4 adds
  `Imaging::note_solve`: a later spec offers it a solve it made itself, and a
  whole-sensor frame takes an aim reading exactly as after the route (FR-037, US4-S7).
- Q: What does 015's tracker do with an aim point measured while Track is on? → A: It
  chases it. While Track is on, every live pointing reaches the tracker as the
  target's offset from the aim point in force (`crates/finder/src/session.rs:1106`), and
  an offset of 10″ or more becomes a rate that removes 60% of it within the interval to
  the next live solve (`crates/finder/src/tracking.rs:29`,
  `crates/finder/src/tracking.rs:342`); only the slow integral term is clamped
  (`crates/finder/src/tracking.rs:352`). The first live pointing taken at a newly
  measured aim point therefore hands the tracker the whole step: on the simulator,
  where the finder frame's centre lies about 127 pixels, some 3800″, from the true aim
  point, about 2300″ within half a second, under Track's guards and outside GoTo's.
  This arises from this spec's landing, whenever an imaging solve measures an aim point
  while Track is on, and not only in 037's run. Owner decision 25 settles it here; the
  owner's answer, on 2026-11-17, holds the aim point back until Track is off (FR-036,
  US4-S8).
- Q: Is the history of live solves a fixed queue? → A: It was: 2000 finder fixes, which
  at two a second hold about 1000 s, less than the three exposures the fit of an aim
  reading reaches across (one before, the exposure, one after) once subs pass about
  330 s. Rule 6 of the plan document's § Mount-neutral by design
  forbids a fixed queue length. The history is now bounded by its span alone and
  thinned: a fix that follows the last one kept by less than the span over 2000 is not
  kept, and only fixes older than the span are dropped (FR-019, US4-S2).
- Q: What else changed? → A: The ASTAP adapter's error for its work folder says "the
  imaging solver's work folder", because the glossary keeps "solver" alone for neither
  solver. `034-dark-and-flat-frames` now names this spec in `depends_on_landed`, so
  plan.md § Prerequisites 9 no longer lists it. Four facts are cited to the research
  note that the audit's edits to it added, and the note now carries them: in its § 3.
  ASTAP, `-fov 0` trying every height from 10° down to 0.3°, the page's `-analyse`
  example with its printed `HFD_MEDIAN=3.3` and `STARS=666` (the note does not give
  the example's figure 30 itself, which rests on ASTAP's page, its source [13]), and
  "DSLR/OSC raw images require 2x2 binning"; and in its § 13, the two lines on the
  `.ini`'s form and the `CD` matrix's row order.

### Session 2026-11-17 (the owner's answers)

- Q: What did the owner answer for owner decision 25, and what changed here? → A: Hold
  it: a newly measured aim point is held while Track is on and used once Track is off.
  That is the answer this trio already recommended and writes as FR-036 and US4-S8, so
  no requirement, scenario or interface here changes. Owner decision 25 is marked
  settled; plan.md § Prerequisites 11 is met, and its § Risks and tasks.md say so. The
  answer's other half is 037's: its centring step turns Track off, takes the new aim
  point at the first live pointing after it, and turns Track on again, reading the
  held aim point through `Session::held_aim` (037's FR-022, FR-025). The edge case of
  a measured aim point while Track is on now says how that re-centre takes it up.

### Session 2026-11-18 (round 1 probes)

- Q: What did the probe builds find? → A: Every story was built from this trio in a
  throwaway worktree, on `main` at b1f01cc, where 030 had landed, with 031's round-1
  probe code on it, tests first, with both gates, `cargo deny check` and `cargo
  machete` (plan.md § Refinement Record, round 1). Every interface built as plan.md
  writes it, but for `take_aim_reading`'s signature and the place where
  `Imaging::solve` builds its hint; every story's tests passed, every story wrote
  exactly its declared files, and none passed the budget; US2 and US4 came closest.
  Five defects were blocking, each in a scenario as worded and none in an interface: the
  simulated adapter kept the sensor centre's up for a frame of a region, so US2-S2's
  top half missed its 0.01 pixel; US4-S5 tapped a star too far from the true aim point
  for the next solve's hint to reach; US4-S4's swung telescope moved the simulated
  adapter's answer too; US4-S8 solved while the GoTo's last approach was still inside
  the fit's reach; and US5's scenarios solved before the finder camera's history held
  enough live solves. Thirty-eight defects were reported in all; thirty-four fixes are
  in this trio, none adds a requirement, and the four left out were each marked as
  noted only or of low value by the probe that found it.
- Q: Which scenarios changed? → A: Each where a build broken on purpose passed every
  test (the probes' bite checks), or where a scenario as worded could not pass:
  - the solving rig: its node is served with `node.service().routes()`;
  - US1-S6: a 2 × 2 frame whose cell mean is rounded, a mono frame 4 × 2502 that is
    halved, and exit code 1 with the two lines still read;
  - US2-S2: a field height 1.06 times the frame's also answers `NotSolved`;
  - US2-S3: the abort is sent 1 s after the frame is asked for, and the frame waited
    for until it is `aborted`;
  - US2-S4: the test's solver waits 1.5 s before each failed answer, and `took_ms` is
    1500;
  - US3-S2: after the third solve, at binning 2, `relation.json` holds the third
    frame's scale, equal to the first solve's per sensor pixel;
  - US3-S4, new bullet: with the target set before the first solve, the last imaging
    solve still hints;
  - US3-S8: `hint_for` with `skip` 4, and, new, a rig whose solver times out three
    times and then finds too few stars three times;
  - US4-S2, new bullet: a drift of 63″ a second, inside the travel's allowance for its
    own error;
  - US4-S4: the simulated telescope is swung back before the solve is asked for, and
    the frame's truth is read before the swing;
  - US4-S5: the tap is on the centroid nearest (700, 420), and the next solve's hint
    is taken at the tapped pixel;
  - US4-S8: the wait after `tracking` is 15 s, the model's slip is never marked, and
    the oracle says what an aim point applied at once does on `main` since 038;
  - US5-S1: the outliers taken in an order that sets each aside, the mirrored one on a
    fresh window;
  - US5-S2, US5-S3 and US5-S4: each console waits for a live pointing and 5 s more
    before its first solve, as US4-S1 does;
  - US5-S4: a second console without the session file drops the stored readings at
    its first solve;
  - US7-S4: "at once" is within 1 s of tokio time.
- Q: Which behaviours did the probes settle where the plan left a choice, or wrote
  something that could not be built? → A: These, each written where plan.md gives the
  behaviour: for a frame of a region the simulated adapter takes the sensor's up as
  seen at the frame's centre, since north turns across the sensor (FR-008 now says
  so); `solved(id)` answers the latest solve once it is past `id` or has ended, woken
  by 031's watch of changes; `SolveHint`'s `Serialize` is written by hand;
  `Imaging::solve` builds the hint with the imaging state's lock released, then checks
  for a running solve again; `take_aim_reading` is a free function on the state the
  solve's task holds; every view, `Imaging::view`'s and the answers of 031's take, the
  settings edit and `Imaging::solve`, measures the relation against the aim point in
  force, read with the imaging state's lock released (as the owner settled on
  2026-11-18, after the re-anchor); `store_scale` and `Relation::take` save through one
  private `Relation::save`; a Bayer region's right and bottom edges go down to an even
  number; a test module binds the live helpers only when it uses them, and writes an expected
  number with no more digits than an `f64` needs; `ImagingField.tsx` writes its own
  path helper and its tick points away from the aim point; the README's count of
  imaging tools goes from three to four; and a word under `src` that names a Tailwind
  utility adds CSS, so the CSS asset's unchanged name is the check that the page
  added none.
- Q: What did the probes have to assume about ASTAP? → A: Thirteen facts. ASTAP was not
  run, and US1-S7 replayed stand-in captures written from this trio, which prove the
  replay and nothing of ASTAP. The thirteen are a checklist in plan.md § Capturing
  ASTAP's output for the owner's real captures (plan.md § Prerequisites 5), each with
  what changes here if a capture disagrees. No behaviour changed for them.
- Q: What is still caught by no test? → A: The order of the refusals when two apply at
  once; a `describe` of another camera zeroing the misses, and an uploaded pointing
  never hinting; which fix inside the exposure is the nearest, the drop of fixes of
  another finder size, the history task's span, uploaded pointings kept out of the
  history, and `track_on`'s `armed` branch, each of little effect on an aim reading; a
  reading set aside applying the median, which cannot differ from what was applied
  before it; and the up axis less its part along the sight line, which no test can
  tell apart. The probes judged each of low value. US1-S5's sleeping script may be
  killed before it writes its pid on a heavily loaded machine; it was not seen.

### Session 2026-11-18 (the owner's answers after the re-anchor to e9687ce)

- Q: Which view do 031's take, its settings edit and `Imaging::solve` answer? → A:
  `Imaging::view`, so each answer shows the aim reading. This trio had said those
  answers were built under the imaging state's lock and never showed one; on `main`
  031 builds no view under that lock, and its take, settings edit and abort already
  answer `Imaging::view` with the lock released. The owner chose that over adding a
  lock-held view to 031's `mod.rs`. It fits FR-023, and no scenario reads the
  difference (plan.md § The imaging state, tasks.md T022).
- Q: Does this spec depend on 038-track-follows-a-push? → A: Yes. US4-S8 asserts that
  the pointing model's slip is never marked, reading 038's `slip_pending`, so the owner
  added 038 to `depends_on_landed`. 038 landed on 2026-11-17, and this trio is
  anchored on top of it (plan.md § Prerequisites 1).
- Q: Does owner decision 25 still say what happens without the hold? → A: Its reason
  is reworded and its answer stands. Since 038, an aim point applied at once with a
  step of more than 5′ is read as a push by hand, which marks the model's slip, rather
  than chased by 015's tracker; a smaller step is still chased. The hold avoids both.
  The owner asked for the same rewording in 035 and 037.

### Session 2026-11-18 (the owner's ASTAP captures)

- Q: What did the owner's captures of ASTAP's real output show? → A: The owner ran
  plan.md § Capturing ASTAP's output with ASTAP CLI-2025.09.29 (Ubuntu's `astap-cli`
  2025.10.08-1) and the D50 database, on a DSS2 red picture of M13 given as a prepared
  copy with only the structural cards, and committed the files as
  `fixtures/finder/astap/` (owner approval 14, plan.md § Prerequisites 5). Eight of
  the thirteen facts the probes had assumed held: `-o` writes the `.ini` and, solved,
  the `.wcs`; the lines are bare `KEY=value` with no `//` note, and their numbers read
  with `f64`'s `FromStr` once trimmed; `CRPIX` is the centre; the `CD` matrix counts
  rows in the file's own order, three Gaia DR3 stars landing within 0.56 pixels of
  ASTAP's places for them; a copy with only the structural cards solves; `-analyse 30`
  prints its two lines on standard output and exits 0; and the `.wcs` is a FITS
  header. Three did not. Every failure exits 1, no solution, too few stars and no
  database alike, where ASTAP's page documents 1, 2 and 32. Without a database ASTAP
  writes an `.ini`: `PLTSOLVD=F`, `CMDLINE=…` and `ERROR=No star database found.`.
  With too few stars the `.ini` holds `ERROR=Not enough stars.` and a `WARNING`; with
  no solution it holds no `ERROR` line at all, and ASTAP prints
  `No solution found!  :(` on standard output instead. Each failed `.ini` begins with
  an empty line. The first run also showed that an Ubuntu install keeps the database
  in `/opt/astap`, where `astap_cli` does not look by itself, so the console needs
  `--astap-db /opt/astap` (plan.md § Prerequisites 8). Two facts the captures could
  not show, that ASTAP takes `-r 180.00` and `-fov 0` and that a killed `astap_cli`
  leaves no child, rest on the first solves on the console machine there.
- Q: How is an exit code of 1 told apart, and what changed for it? → A: By the `.ini`'s
  `ERROR` line, as the owner chose on 2026-11-18: `Not enough stars.` is
  `TooFewStars`, `No star database found.` is `NoDatabase`, and no `ERROR` line, no
  `.ini` or any other text is `NotSolved`, whose detail is that text when there is one
  and otherwise `No solution found!`, ASTAP's own words less their `:(`. Codes 2, 32
  and 33 keep their rows, for another version of ASTAP. FR-005 says so, and so do
  plan.md's exit-code table and `read_error`; `read_answer` skips a line with no `=`,
  as a failed `.ini`'s empty first line is. No owner decision changes, and FR-001's
  reasons and messages stand.
- Q: Which scenarios changed? → A:
  - US1-S4: its Given names the failed file, an empty line, `PLTSOLVD=F` and
    `CMDLINE=…`, the form of the saved `not-solved.ini`. Its Then reads exit code 1 with
    that file as `NotSolved` with `No solution found!`; with `ERROR=Not enough stars.`
    after it as `TooFewStars` with that detail; with `ERROR=No star database found.`
    as `NoDatabase` with that detail; with another `ERROR` as `NotSolved` with that
    text; and with no `reply.ini` as `NotSolved` with `No solution found!`. Exit code
    2 with the failed file, and 32 and 33 with no `reply.ini`, stay, for another
    version. Its oracle names the saved captures for exit code 1;
  - US1-S7: its Then gives each failed case's detail, `No solution found!`,
    `Not enough stars.` and `No star database found.`, each case replayed from exit
    code 1 and its own saved `.ini`; no case's `ini` is `null`.
- Q: What else changed? → A: US2-S4 is unchanged: its test solver's `NotSolved` already
  carries `No solution found!`, ASTAP's own words. Beside the scenarios: FR-005; owner
  approval 14, which now lists `no-database.ini`; the edge case of a missing database;
  two answers of the review session of 2026-11-15, which now point here; and, in
  plan.md, the exit-code table, `read_error`, `read_answer`, § Capturing ASTAP's output
  (its script passes `-d`, its `captures.json` is the committed one, and its thirteen
  checks are ticked, 11 and 12 left open), § Prerequisites 5, 6 and 8, § Risks and the
  Refinement Record; and tasks.md's Prerequisites, T002, T003 and T005.

## Owner decisions

Each is a design or taste call the plan document leaves open. None is settled but 25,
which the owner settled on 2026-11-17. A different answer is written into the trio and
re-validated before the flip, never left to a story.

1. **How the session says the aim point was set.** Recommended: `aim` keeps `x` and `y`
   and gains `source` with the value `measured` only when the console measured it; a
   tapped aim point reads exactly as today. Instead: `source` on every aim point,
   `tapped` included, which edits the six assertions Clarification 2 lists and needs
   its own approval.
2. **A tap or a reset while a measured aim point is in force.** Recommended: it is
   obeyed, and the next imaging solve that shares a moment with a live pointing measures
   the aim point again and replaces it. A tap or a reset also drops a measured aim point
   held back while Track is on (owner decision 25). Instead: refuse a tap while an
   imaging camera is solving.
3. **What a moved aim point does to the pointing model.** Recommended: nothing is reset,
   as a tap resets nothing today; a sync more than 0.5° from the model's prediction,
   or 2.5° before the base tilt is fitted, already opens a new zero-point epoch
   (`crates/finder/src/model.rs:513`). Instead: drop the night's syncs when the aim
   point moves by more than a stated amount.
4. **What ASTAP is given.** Recommended: a prepared copy in a work folder: a colour
   mosaic with each 2 × 2 cell averaged into one sample, halved again while it is over
   2500 rows, solved with `-z 1`. The saved imaging frame is never opened by ASTAP and
   the answer's pixels are known exactly. Instead: the saved file itself with `-z 2`.
5. **The matched star count.** Recommended: optional. The simulated adapter reports it;
   the ASTAP adapter reports none. Instead: read ASTAP's printed text for a count.
6. **Where the focal length is set.** Recommended: a console flag, `--focal-length-mm`,
   beside the ASTAP flags; it matters only until the first imaging solve has measured
   the scale. Instead: a key of `PUT /api/imaging/settings` and a field on the Imaging
   page, which adds a tenth key to the settings 031's MCP schema test counts.
7. **The search radii.** Recommended: 3° for a hint from a live pointing before the aim
   point is measured, from the last imaging solve or from the target (the plan
   document's figure); the sensor's diagonal on the sky, never under 0.2° or over 3°,
   once the aim point is measured, so that a frame of any region of the sensor lies
   inside it; the whole sky with no hint. Instead: other figures, or refuse to solve
   with no hint.
8. **How fresh a live pointing must be to hint.** Recommended: no more than 30 s before
   the exposure's start or after its end. Instead: another figure.
9. **Smoothing.** Recommended: the median of the last five aim readings; once three
   are held, an aim reading more than 120″ on the sky or 1° of imaging angle from the
   median, or of the other mirror sense, is set aside; three set aside in a row
   replace the window, so a camera turned in the focuser takes over after three
   solves. Instead: other figures.
10. **When the aim point in force moves.** Recommended: only when the smoothed value is
    more than 10″ on the sky from it, so the pointing model's reference does not jitter
    with every solve. Instead: on every aim reading.
11. **Stillness, and where the aim point was at mid-exposure.** Recommended: a straight
    line is fitted, by least squares in each axis of the finder camera's frame, through
    where the imaging solve's centre falls in every live solve from 10 s before the
    exposure to 10 s after it (the exposure's length each way if that is longer). The
    aim reading is the line's value at the middle of the exposure. None is taken
    unless at least three live solves are in that span and one is inside the exposure
    itself; when the line's travel during the exposure is more than 60″ on the sky by
    more than its own standard error; or when the live solves scatter about the line
    by more than 90″. Instead: other figures; the difference of two neighbouring live
    solves, which the finder camera's scatter makes useless (Clarifications, the third
    session); or axis counts, which the mount-neutral rules forbid in imaging code.
12. **How long the solver program may run.** Recommended: 120 s for a solve with a
    hinted place, 600 s for a search of the whole sky, and 30 s for a star measure,
    after which the program is killed. ASTAP's only published timings, 4.8 s to 23.8 s
    for a start 90° away at a field 1.32° high, scale to minutes at a field 0.5° high,
    so a search of the whole sky gets its own limit and may still run out (Edge
    Cases). The first solves at bring-up, read from `took_ms`, give the real times.
    Instead: other figures.
13. **Which imaging frame the route solves.** Recommended: the latest saved one, with no
    body. Instead: a frame by id, which needs a way to find a frame of an earlier date.
14. **The sidecar.** Recommended: written for a failed solve too, with the reason, and
    replaced when the same imaging frame is solved again. Instead: only for a solved
    one.
15. **The ASTAP flags.** Recommended: `--astap` needs `--astap-db-name`; `--astap-db`
    is optional; none may be given with `--simulate`. Instead: a default database name.
16. **Checks of the page.** Recommended: Vitest only. A Playwright check would solve an
    imaging frame on the simulated console the other checks share, and leave it with a
    measured aim point (Clarification 9). Instead: a Playwright check that resets the
    aim point after itself.
17. **What the chart shows.** Recommended: the imaging field's outline, on the live
    chart, only while the aim point is a measured one. A tapped aim point is not drawn.
    Instead: also a mark at a tapped aim point.
18. **The work folder.** Recommended: each call's folder is removed when the call ends.
    Instead: keep the last failed one for study.
19. **A failed solve is not tried again wider.** Recommended. Instead: one more try at
    3°, which doubles the wait on every cloudy sub.
20. **The solve keeps its linear part.** Recommended: an imaging solve and its sidecar
    hold the four numbers that map an imaging-frame pixel step to east and north, so 035
    can place a sub by more than shift, angle and scale. Instead: centre, angle, scale
    and mirror only.
21. **Where the solve shows.** Recommended: `solve` and `relation` appear in the imaging
    view only while its `state` is `ready`, as 031's view holds nothing else outside
    it. Instead: always.
22. **When the kept scale stops solving.** The scale belongs to a camera on a
    telescope, but the console can only tell cameras apart. Recommended: the field
    height's sources are taken in turn: the kept scale, then the focal length, then
    none, with which ASTAP finds the height itself. Three solves in a row that end
    `not_solved` or `timed_out` move the console on to the next source that exists,
    and from the last back to the first, so no source can lock solving out; a solve
    that succeeds replaces the kept scale and starts again from it. No solve is run
    twice. A spell of bad solves under a true scale costs at most a few solves given
    a poorer height. The hint's source moves one place down at the same counts (owner
    decision 23). Instead: try a `not_solved` solve again at once with no field
    height, which doubles the wait on every failed sub (owner decision 19); or let a
    `--focal-length-mm` that disagrees with the kept scale by more than 5% clear it,
    which helps only an observer who restarts the console with the right figure.
23. **The hint's sources, tried in order across solves.** The plan document says the
    sources are "tried in order". Recommended: the first source that exists hints a
    solve, and a failed solve is not asked again (owner decision 19); after three
    solves in a row that end `not_solved` or `timed_out`, the next solve takes the
    source one place down among those that exist, and three misses more the next,
    ending at `none`, at the same counts that move the field height on (owner decision
    22); a solved solve returns both to the first. So a hint that exists but is wrong
    gives way, and the search of the whole sky is reached. The last imaging solve is a
    source only while the session's target is the one it was when that solve was made,
    so a new target is not hinted by the old field; it has no age limit otherwise.
    Instead: a ranking only, in which a wrong `pointing` hint fails every solve (the
    first draft); one solve per source until one succeeds, which multiplies the wait of
    every failed sub by up to four; turning from `none` back to the first source, as
    the field height turns; or an age limit on the last imaging solve.
24. **The floor given to ASTAP's `-analyse`.** Recommended: a signal-to-noise ratio of
    30, the figure ASTAP's own page uses in its example. 033 turns the star measure
    into the focus reading and may ask for another figure in its own trio. Instead:
    another figure now.
25. **A measured aim point and a running tracker** (a seam between this spec and 015
    that a run of 037 crosses; moved here from 037's owner decision 26, which now
    cites this one). As first drafted, an aim reading set the session's aim point at once.
    When this was put, 015's tracker would then have closed the whole step from the old
    aim point at a rate nothing bounds, from this spec's landing (Clarifications, the
    fourth session). Since 038 landed, a step of more than 5′ is read as a push by hand
    instead: nothing is handed to the tracker, but the pointing model's slip is marked,
    though nothing slipped; a smaller step is still closed by the tracker (Edge Cases, a
    measured aim point while Track is on).
    **Settled by the owner on 2026-11-17: hold it.** This spec holds a measured aim
    point back while Track is on, shows it as held, and applies it at the first live
    solve the session records after Track is off (FR-036, US4-S8). The hold avoids both,
    the chase and the false slip, where they arise, from this spec's landing, with no
    change to 015's tracker. 037's centring step turns Track off, takes the new aim
    point at the first live pointing after it, re-centres and turns Track on again
    (037's FR-022, FR-025). Instead:
    036's tracker bounds the rate one tracking error may ask for (037's first
    recommendation), which leaves the hazard live from this spec's landing until 036's
    and replaces FR-036 and US4-S8 with nothing here; or Track restarts through GoTo
    when the aim point changes, which this trio would write into
    `crates/finder/src/session.rs` and which needs the console's GoTo there; or the
    behaviour is accepted as drafted and bounded by a probe on the simulator and at the
    telescope. The answer is written here and in 037 (plan.md § Prerequisites 11).

## Owner approvals

Flipping this spec to `ready` approves the settled answer to every owner decision above,
and the following. Each is named in the FR or success criterion that makes it.

1. **The glossary's aim point changes meaning.** It was the pixel on which the eyepiece
   is centred, set by a tap. It becomes the spot where the main telescope looks: the
   centre of the imaging camera's sensor, measured by the console, or the centre of the
   eyepiece's view, tapped, when no imaging camera is attached. This extends
   013-live-finder's FR-025 and FR-026 (`specs/013-live-finder/spec.md:835`,
   `specs/013-live-finder/spec.md:839`), which stay in force for the tap: the session
   still holds one pixel, kept across restarts, and now also whether the console
   measured it (FR-022).
2. **No story writes `GLOSSARY.md` or the plan document.** The terms used here (aim
   point as changed, imaging angle, imaging solver, solver port, solver adapter, imaging
   solve) are written by the plan document's own change to `GLOSSARY.md`. That change
   also carries, on `main` since 030's flip (55e41f5; plan.md § Prerequisites 7), six
   entries for words this trio uses as terms, worded as here:
   - **Finder fix**: "One live solve of the finder camera as the console's history
     keeps it: when its star frame was taken, and how it maps the sky onto the finder
     camera's frame. _Avoid_: sample, sync (that ties the mount to the sky)".
   - **Star measure**: "The median half-flux diameter, in pixels, and the star count
     that a solver adapter reports for an imaging frame or a region of one. _Avoid_:
     focus reading (that is in arcseconds, for the focus region), HFD (alone)".
   - **Aim reading**: "One reading of the aim point and the imaging angle, taken from
     an imaging solve and the finder camera's live solves of the same moment. The
     console smooths the last few into the aim point it uses. _Avoid_: measure (that
     is the star measure), sample, calibration".
   - **Relation** (between the cameras): "What the console keeps between the finder
     camera and the imaging camera: the scale an imaging solve measured, the aim
     readings held, and from them the aim point and the imaging angle. _Avoid_:
     alignment, calibration, offset".
   - **Solve hint**: "What the console tells the imaging solver before an imaging
     solve: where to look, how far around it, and how tall the field is, each when
     known. _Avoid_: seed, guess, prior".
   - **Solve sidecar**: "The file beside a saved imaging frame that records its
     imaging solve, or why it failed. The imaging frame's own file is never
     rewritten. _Avoid_: header update, WCS file".
3. **An external program as the imaging solver**: ASTAP's `astap_cli`, MPL-2.0, run as
   a separate process and never linked, with a star database that asks for the credit
   "ESA/Gaia/DPAC". It is not a cargo dependency, and no `Cargo.toml`, `Cargo.lock` or
   `deny.toml` changes (FR-003, SC-003). Principle V of constitution 1.3.0 allows it: a
   separate program behind one of our ports, never linked (plan.md § Prerequisites 4).
4. **One route and two parts of the imaging object**: `POST /api/imaging/solve`
   (FR-009), and `solve` and `relation` in `GET /api/imaging` and in the `imaging`
   object of `GET /api/session`, which the MCP tools that return either carry too
   (FR-010, FR-017, FR-023, and `relation.held`, FR-036).
5. **The session's `aim` gains `source`**, and `session.json` gains `aim_measured`,
   written only when true (FR-022, owner decision 1).
6. **Four flags of `finder`**: `--astap`, `--astap-db`, `--astap-db-name` (FR-013) and
   `--focal-length-mm` (FR-018).
7. **Five new module files** under `crates/finder/src/imaging/`: `solver.rs` and
   `astap.rs` (FR-001), `simulated.rs` and `solving.rs` (FR-008, FR-009), and
   `relation.rs` (FR-017).
8. **Two file formats on disk and one work folder**, all three now in the plan
   document's storage layout on `main`: the solve sidecar `<frame id>.solve.json`, format
   `imaging-solve/1`, beside each imaging frame a solve was ended for (FR-011);
   `relation.json`, format `imaging-relation/1`, in the imaging folder (FR-017,
   FR-027); and the ASTAP adapter's work folder, `solver/` in the imaging folder, in
   which each call makes and removes one `solve-<pid>-<n>/` (FR-003, FR-006).
9. **New public names in existing modules**: `Frame::up_east_of_north_at` in
   `crates/finder/src/sky.rs` (FR-021); `AimView` and `Session::set_measured_aim` in
   `crates/finder/src/session.rs`, with `SessionView::aim` becoming an `AimView`
   (FR-022), and `Session::held_aim` (FR-036); `AppState::with_imaging_solver` and
   `AppState::with_focal_length_mm` (FR-009, FR-018); and, on 031's `Imaging`, the
   methods `set_solver`, `solver`, `solve`, `solved`, `hint`, `scale_arcsec_per_px`
   (FR-009, FR-018) and `note_solve` (FR-037).
10. **One MCP tool**: `solve_imaging_frame` (FR-033).
11. **`DESIGN.md`**: one new entry, Imaging field on the chart; one sentence added to
    the Aim-point tap entry; one sentence added to 031's Imaging page entry, with the
    words of plan.md § `DESIGN.md`; and one component in `.impeccable/design.json`
    (FR-032).
12. **Document sections**: `## Solving imaging frames` in `crates/finder/README.md`,
    one row of its API table and one line of its MCP tool list (FR-013, FR-018, FR-028,
    FR-034).
13. **Edits to existing test files** (SC-002), and no others: one `mod` line for each
    new finder test module, in alphabetical order, in
    `crates/finder/tests/console/main.rs` (`mod imaging_astap;`, `mod imaging_hints;`,
    `mod imaging_relation;`, `mod imaging_smoothing;`, `mod imaging_solve;`),
    `crates/finder/tests/cli/main.rs` (`mod imaging_hints;`, `mod imaging_solve;`),
    `crates/finder/tests/motion/main.rs` (`mod imaging_track;`) and
    `crates/finder/tests/mcp/main.rs` (`mod imaging_solve;`), as 027's layout asks of
    every new finder test.
14. **Operator-staged fixture files**, written by no story: the saved captures of
    ASTAP's real output under `fixtures/finder/astap/` (`captures.json`, `solved.ini`,
    `solved.wcs`, `not-solved.ini`, `too-few-stars.ini`, `no-database.ini`,
    `analyse.stdout` and `README.md`), with three
    hand-identified catalogue stars for the solved case, made as plan.md § Capturing
    ASTAP's output says (plan.md § Prerequisites 5).

## User Scenarios & Testing *(mandatory)*

Every scenario shares these definitions unless it says otherwise.

- **The gate command.** Every Rust scenario runs in
  `cargo nextest run --workspace --all-targets --all-features --locked`, the `test`
  gate's first command, each test in its own process. The page's scenarios run in
  `npm test`, a later command of the same gate, in `crates/finder/web`.
- **Paused time.** A test of the in-process console runs under
  `#[tokio::test(start_paused = true)]`, so every wait is in virtual time. A test that
  runs the stub program (US1) starts a real child process: it is a plain
  `#[tokio::test]`, in real time, and each wait it makes has the limit its scenario
  states. A test that starts the built `finder` is a plain `#[test]`.
- **A position in an imaging frame** is (x, y) in pixels: x to the right from the left
  edge of column 0, y downward from the top edge of row 0. The sample at column c, row
  r covers the square from (c, r) to (c + 1, r + 1). The centre of a frame W wide and H
  high is (W/2, H/2). Up is toward smaller y. A saved imaging frame's first row in the
  file is its top row (031's `ROWORDER` card says `TOP-DOWN`).
- **Angles** are in degrees. "East of north" is measured at a place on the sky from the
  direction of the celestial north pole toward increasing right ascension, 0 to 360. A
  difference of two angles is reduced into −180 to 180 before it is compared.
- **A separation** is the great-circle angle between two J2000 places, computed by the
  test with `math_core::angular_distance` over `UnitVector::from_radec`, in arcseconds.
  **A position angle** from one place to another is computed by the test from the two
  places with the spherical formula `atan2(sin Δα · cos δ2, cos δ1 · sin δ2 − sin δ1 ·
  cos δ2 · cos Δα)`, east of north.
- **The solving rig** is 031's console rig with a simulated solver: a `Simulator` over
  the test database at the site 40° N 75° W on a `VirtualClock` from Unix 1768528800
  (`2026-01-16T02:00:00Z`), as `crates/finder/tests/support/live.rs:41` builds it, with
  a `SimMount::new` (`crates/simulator/src/mount.rs:232`) connected first and
  `push_to(50.0, 120.0)` after; a `SimNode` over it with a 500 ms frame interval and
  the token `test-token`, served over a fresh duplex for each connection with
  `memory::serve_routes_on` and `node.service().routes()` (a `SimNode` has no `routes()`
  of its own; `crates/finder/tests/support/mcp.rs:228` serves a node's service the same
  way); a session paired to it with
  the site 40° N 75° W, no target and no aim point; then `AppState::shared` with
  `with_session`, `with_clock` on the simulator's clock, `with_link`, `with_live()`,
  `with_imaging(R)` and `with_imaging_solver(Arc::new(SimulatedSolver::new(sim.clone())))`.
  **R** is the child path `imaging` of the test's scratch folder.
- **The simulator's truth.** The simulated finder camera is 1456 × 1088 over 12°
  (29.78″ a pixel) and the main telescope looks through its pixel (700, 420)
  (`crates/simulator/src/optics.rs:27`): **the true aim point**, read by a test as
  `sim.optics().eyepiece_px`. The simulated imaging camera (030 US2) is 1236 × 822 over
  0.75°, turned 17° from the finder camera's frame: **the true imaging angle**, read as
  `sim.imaging_optics().angle_deg`. **The truth of an imaging frame** is
  `sim.imaging_truth_at(t)` for t the middle of its exposure, `frame.start +
  frame.exposure_ms / 2000` from the imaging view: the J2000 centre, the angle of up
  east of north and the scale, about 2.1845″ a pixel.
- **Scratch folders.** A finder test module takes its folder from
  `crate::support::Scratch::new(name)`, the helper 028's US5 put in
  `crates/finder/tests/support/mod.rs:12`, which makes the folder under
  `CARGO_TARGET_TMPDIR` and removes it when dropped; its path is the field `.0`. No
  test builds a path from `CARGO_MANIFEST_DIR` and `../../target`.
- **Requests** go through the router in-process, as `request` in
  `crates/finder/tests/support/live.rs:90` sends them, with `Content-Type:
  application/json` and no `Origin`. A scenario that names an `Origin` builds its
  request with a helper of the module's own, with a `Host` of localhost on port 8080,
  as `crates/finder/tests/console/recording.rs:426` does. A foreign `Origin` is
  `http://elsewhere.example`.
- **Waiting until** a condition means reading `GET /api/imaging`, or `GET /api/session`
  for a condition on the session, every 100 ms until it holds, for at most 300 s
  unless the scenario states a shorter limit, and failing otherwise. A solving-rig scenario first waits
  until `state` is `ready`.
- **A saved frame** is taken with `POST /api/imaging/frame` and awaited until
  `frame.state` is `saved`. The exposure is 031's default, 1000 ms, unless the scenario
  sets `exposure_ms` first with `PUT /api/imaging/settings`. **Solving it** is `POST
  /api/imaging/solve` and waiting until `solve.frame_id` is that frame's id and
  `solve.state` is not `solving`.
- **Reading files.** A test parses a sidecar and `relation.json` itself with
  `serde_json`, and reads a FITS file with code of its own, never through the console's
  FITS code: 80-byte cards up to `END`, the header padded to 2880 bytes, then
  big-endian samples, each 16-bit sample being the stored signed value plus 32768.
- **Comparing a view.** A test compares `solve`, `relation` and `aim` key by key, never
  whole, because later stories and specs add keys. No test asserts that `relation` or a
  key of it is absent unless its scenario says so.
- **The stub program** stands in for `astap_cli`. A US1 test writes it into its scratch
  folder with mode 0700, as `crates/scope-node/tests/rpicam.rs:140` writes its script,
  with exactly this text:

  ```sh
  #!/bin/sh
  dir=$(dirname "$0")
  : > "$dir/args"
  base=
  input=
  while [ $# -gt 0 ]; do
    printf '%s\n' "$1" >> "$dir/args"
    case "$1" in
      -o) base=$2 ;;
      -f) input=$2 ;;
    esac
    shift
  done
  [ -n "$input" ] && cp "$input" "$dir/input.fits"
  [ -n "$base" ] && [ -f "$dir/reply.ini" ] && cp "$dir/reply.ini" "$base.ini"
  [ -n "$base" ] && [ -f "$dir/reply.wcs" ] && cp "$dir/reply.wcs" "$base.wcs"
  [ -f "$dir/reply.stdout" ] && cat "$dir/reply.stdout"
  exit "$(cat "$dir/code")"
  ```

  The test puts `code` (the exit code as text) and whichever `reply.*` files the
  scenario names beside it, and afterwards reads `args` (one argument a line) and
  `input.fits` (the file ASTAP was given). **The stub adapter** is an `AstapSolver`
  whose program is the stub, whose database name is `d50`, whose database folder is
  `/opt/astap`, and whose work folder is the child path `work` of the scratch folder.
- **A finder fix** in a US4 scenario is a `FinderFix` the test builds itself: its
  `frame` is `Frame::from_solution` of a `plate_solver::Solution` whose `ra` and `dec`
  are the stated RA and Dec in radians, whose `roll` is 0, whose `camera` is
  `PinholeCamera::new(1456.0, 1088.0, 12° in radians)` and whose other fields are the
  default (north up, no distortion, 29.78″ a pixel), as
  `crates/finder/tests/console/live/loop_.rs:128` builds a frame; its `time` is the
  stated one; its `frame_id` is its place in the list, counted from 0.
- **Worked answer A** is the example `.ini` on ASTAP's own page, for a picture 2328
  wide and 1760 high. The page is source [13] of
  `docs/research/2026-11-15-imaging-camera.md` § Sources, and its § 3. ASTAP lists the
  keys; the numbers below were read from the page on 2026-11-15, without the `//`
  note the page prints after each line:

  ```text
  PLTSOLVD=T
  CRPIX1= 1.1645000000000000E+003
  CRPIX2= 8.8050000000000000E+002
  CRVAL1= 1.5463033992314939E+002
  CRVAL2= 2.2039358425145043E+001
  CDELT1=-7.4798001762187193E-004
  CDELT2= 7.4845252983311850E-004
  CROTA1=-1.1668387329628058E+000
  CROTA2=-1.1900321176194073E+000
  CD1_1=-7.4781868711882519E-004
  CD1_2= 1.5241315209850368E-005
  CD2_1= 1.5534412042060001E-005
  CD2_2= 7.4829732842251226E-004
  CMDLINE=astap_cli -f frame.fits
  ```

  Read as a file whose first row is its top row, it says: centre RA 154.63033992314939°
  and Dec 22.039358425145043° (the reference pixel is the centre); up at
  `atan2(−CD1_2, −CD2_2)` = 181.16684° east of north; scale `3600·√|CD1_1·CD2_2 −
  CD1_2·CD2_1|` = 2.69358″ a pixel; and mirrored, because that determinant is negative.
- **Worked answer B** is hand-made for an imaging frame 1236 × 822 with the Bayer
  pattern `RGGB`, whose prepared copy is 618 × 411: up 30° east of north, 0.0012° for
  each prepared pixel, not mirrored, centred on RA 83.8221°, Dec −5.3911°:

  ```text
  PLTSOLVD=T
  CRPIX1= 3.0950000000000000E+002
  CRPIX2= 2.0600000000000000E+002
  CRVAL1= 8.3822100000000006E+001
  CRVAL2=-5.3910999999999998E+000
  CDELT1=-1.2000000000000000E-003
  CDELT2= 1.2000000000000000E-003
  CROTA1= 3.0000000000000000E+001
  CROTA2= 3.0000000000000000E+001
  CD1_1=-1.0392304845413263E-003
  CD1_2=-6.0000000000000000E-004
  CD2_1= 6.0000000000000000E-004
  CD2_2=-1.0392304845413263E-003
  CMDLINE=astap_cli -f frame.fits
  WARNING=Warning scale was inaccurate! Set FOV=0.49d
  ```

  For the imaging frame it says: centre (83.8221°, −5.3911°); up 30° east of north;
  2.16″ a pixel (half of 0.0012° × 3600); not mirrored; and the place 100 pixels above
  the centre is 216.0″ from it at position angle 30°.
- **The test frame** of a US1 scenario is a 16-bit FITS file the test writes itself,
  whose sample at column c, row r is `1000 + 4·c + 8·r`. Averaging its 2 × 2 cell whose
  top-left sample is (a, b) gives `1006 + 4·a + 8·b`.
- **Oracles.** Each scenario names its oracle: the simulator's truth, a worked answer
  or worked example, ASTAP's documents, the saved captures, or, for a scenario that
  checks only what the console says and writes, the words, statuses and keys the
  scenario itself states, which the test compares exactly.

### User Story 1 - The solver port and the ASTAP adapter (Priority: P1)

As the console, I ask an imaging solver where a saved imaging frame points, or how large
its stars are, without knowing which program answers. The ASTAP adapter runs ASTAP's
command-line program on a prepared copy, reads its answer and turns every way it can
fail into a reason.

**Why this priority**: Every later step of the imaging run rests on an imaging solve,
and the adapter is the one part that cannot be tried without the program's real output.

**Independent Test**: `crates/finder/tests/console/imaging_astap.rs`, the module
`imaging_astap` of finder's `console` test binary. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test console imaging_astap::`.

**Acceptance Scenarios**:

1. **Given** worked answer A, **When** a test reads it with `astap::read_answer` and
   builds the imaging solve of a mono imaging frame 2328 × 1760 with
   `ImagingSolve::from_answer`, the prepared copy being the frame itself, **Then** a
   committed test asserts `ra_deg` 154.63033992314939 and `dec_deg` 22.039358425145043
   within 1e-9°; `up_east_of_north_deg` 181.16684 within 1e-4°; `arcsec_per_px` 2.69358
   within 1e-4; `mirrored` true; `cd_deg` equal to the four `CD` values, as `[[CD1_1,
   CD1_2], [CD2_1, CD2_2]]`, each within 1e-18; and `warning` absent. Oracle: ASTAP's
   documents and the arithmetic of worked answer A.
2. **Given** the stub adapter with worked answer B as `reply.ini` and `code` 0, and the
   test frame at 1236 × 822 with the Bayer pattern `RGGB`, **When** a test calls
   `solve` with a hint at RA 83.8221°, Dec −5.3911°, a radius of 3° and a field height
   of 0.4997°, **Then** a committed test asserts that:
   - the answer has `ra_deg` 83.8221 and `dec_deg` −5.3911 within 1e-9°,
     `up_east_of_north_deg` 30.0 within 1e-9°, `arcsec_per_px` 2.16 within 1e-9,
     `mirrored` false, `stars` absent, `width` 1236, `height` 822 and `warning`
     `Warning scale was inaccurate! Set FOV=0.49d`;
   - `sky_at((618.0, 311.0))` is 216.0″ within 0.01″ from the centre, at position angle
     30.0° within 0.001°, and `pixel_of` of that place is (618.0, 311.0) within 1e-6;
   - `args` holds exactly eighteen lines: `-f`, a path ending `/frame.fits` inside the
     work folder, `-o`, that path without `.fits`, `-ra`, `5.588140`, `-spd`,
     `84.60890`, `-r`, `3.00`, `-fov`, `0.4997`, `-z`, `1`, `-D`, `d50`, `-d`,
     `/opt/astap`;
   - `input.fits` is a 16-bit picture 618 × 411 whose sample at column 0, row 0 is 1006
     and at column 5, row 3 is 1094;
   - afterwards the work folder holds no entry, and the test frame's bytes are what the
     test wrote;
   - oracle: worked answer B and the test frame's rule.
3. **Given** `astap::solve_args` called with the input `/w/frame.fits` and the base
   `/w/frame`, **Then** a committed test asserts that a hint with no position, a radius
   of 180° and no field height, the database name `d50` and no database folder give
   exactly `-f /w/frame.fits -o /w/frame -ra 0.000000 -spd 90.00000 -r 180.00 -fov 0 -z
   1 -D d50`, one argument a word; that a position of RA 10°, Dec −90° with a radius of
   500° gives `-ra 0.666667`, `-spd 0.00000` and `-r 180.00`; and that
   `astap::analyse_args` for the same input gives exactly `-f /w/frame.fits -analyse
   30`. Oracle: ASTAP's documents (hours, declination + 90°, the picture's height).
4. **Given** the stub adapter and a mono test frame 64 × 48, **Then** a committed test
   asserts, for each `code` and `reply.ini` below, the error `solve` answers, its
   `reason()` and its `message()`. **The failed file** is an empty line, then
   `PLTSOLVD=F` and `CMDLINE=astap_cli -f frame.fits`, each on its own line, the form of
   the saved `not-solved.ini`:
   - 1 with the failed file: `NotSolved` with the detail `No solution found!`,
     `not_solved`, `the imaging solver found no solution`;
   - 1 with the failed file and `ERROR=Not enough stars.` after it: `TooFewStars` with
     the detail `Not enough stars.`, `too_few_stars`, `the imaging solver found too few
     stars in the imaging frame`;
   - 1 with the failed file and `ERROR=No star database found.` after it: `NoDatabase`
     with the detail `No star database found.`, `no_database`, `the imaging solver has
     no star database it can read`;
   - 1 with the failed file and `ERROR=Something else.` after it: `NotSolved` with the
     detail `Something else.`, `not_solved`, `the imaging solver found no solution`;
   - 1 with no `reply.ini`: `NotSolved` with the detail `No solution found!`,
     `not_solved`, `the imaging solver found no solution`;
   - 2 with the failed file: `TooFewStars` with no detail, `too_few_stars`, `the
     imaging solver found too few stars in the imaging frame`;
   - 32 and 33, each with no `reply.ini`: `NoDatabase` with no detail, `no_database`,
     `the imaging solver has no star database it can read`;
   - 16: `Failed`, `failed`, `the imaging solver could not read the imaging frame`;
   - 34: `Failed`, `failed`, `the imaging solver ended with exit code 34`;
   - 0 with no `reply.ini`: `Failed`, `the imaging solver's answer could not be read:
     no .ini file was written`;
   - 0 with `PLTSOLVD=F`: `Failed`, `the imaging solver's answer could not be read:
     PLTSOLVD is not T`;
   - 0 with worked answer A less its `CD2_2` line: `Failed`, `the imaging solver's
     answer could not be read: CD2_2 is missing`;
   - 0 with worked answer A whose `CRVAL1` line is `CRVAL1= abc`: `Failed`, `the
     imaging solver's answer could not be read: CRVAL1 is not a number`;
   - an adapter whose program is a path that does not exist: `Failed`, with a message
     beginning `the imaging solver could not be started: `;
   - after each of these, the work folder holds no entry;
   - oracle: for exit code 1, the saved captures' exit codes and `.ini` files (plan.md
     § Capturing ASTAP's output) and the owner's rule for telling them apart; for 2,
     16, 32, 33 and 34, ASTAP's documented exit codes, kept for a version that gives
     them.
5. **Given** an `AstapSolver` whose program is a script of the test's own, `#!/bin/sh`,
   then `echo $$ > "$(dirname "$0")/pid"`, then `exec sleep 30`, built
   `with_limits` of 500 ms for a solve, 1500 ms for a search of the whole sky and
   500 ms for a star measure, **When** a test calls `solve` on a mono test frame
   64 × 48 with a hint at RA 10°, Dec 20° and a radius of 3°, **Then** a committed test
   first asserts that `/proc/self` exists, so that the next check can fail, and then
   that the call returns within 10 s of real time with `TimedOut`, `timed_out` and
   `the imaging solver gave no answer within 1 s`; that no `/proc/<pid>` exists for
   the pid the script wrote; and that the work folder holds no entry. **When** it
   calls `solve` again with `SolveHint::whole_sky()`, **Then** the committed test
   asserts `TimedOut` and `the imaging solver gave no answer within 2 s`, and that the
   call took at least 1.4 s of real time: a search of the whole sky has its own
   limit. Oracle: the stated limits.
6. **Given** the stub adapter with `reply.stdout` holding the two lines
   `HFD_MEDIAN=3.3` and `STARS=666`, and `code` 0, **Then** a committed test asserts
   that:
   - `measure` of a mono test frame 64 × 48 with no region answers `half_flux_px` 3.3
     and `stars` 666, with `args` exactly `-f`, a path ending `/frame.fits` inside the
     work folder, `-analyse`, `30`;
   - `measure` of an `RGGB` test frame 64 × 48 with the region x 10, y 10, width 21,
     height 21 answers `half_flux_px` 6.6 within 1e-12 and `stars` 666, and
     `input.fits` is 10 × 10 with sample (0, 0) 1126 and sample (9, 9) 1342;
   - the same with the region x 11, y 11, width 20, height 20 leaves an `input.fits` of
     9 × 9 whose sample (0, 0) is 1150;
   - `measure` of an `RGGB` frame 2 × 2 of the test's own, whose samples are 1000 and
     1001 in its first row and 1001 and 1000 in its second, leaves an `input.fits` of
     one sample, 1001: the mean of 4002, rounded half up (every cell of the test frame
     has a whole mean, so only this frame shows the rounding);
   - `measure` of a mono test frame 4 × 2502 leaves an `input.fits` of 2 × 1251, halved
     once since it had more than 2500 rows, whose sample (0, 0) is 1006, and answers
     `half_flux_px` 6.6 within 1e-12;
   - with `code` 1 and the same two lines, `measure` of the mono test frame 64 × 48
     still answers 3.3 and 666: the two lines are read whatever the exit code;
   - with `reply.stdout` `HFD_MEDIAN=0.0` and `STARS=0` it answers `TooFewStars`; with
     an empty `reply.stdout`, `Failed` and `the imaging solver's measure could not be
     read`; a region reaching outside the frame (x 60, y 40, width 10, height 10),
     `Failed` and `the region is outside the imaging frame`; and, on the `RGGB` test
     frame, the region x 11, y 11, width 2, height 2, which holds no whole cell,
     `Failed` and `the region is too small`;
   - oracle: ASTAP's documents (the two printed lines), the test frame's rule and the
     worked rounding.
7. **Given** the saved captures under `fixtures/finder/astap/` (owner approval 14), read
   through `captures.json`, and for each case the stub adapter replaying that case's
   saved files and exit code on a mono test frame of the size `captures.json` gives,
   **Then** a committed test asserts that:
   - `solved` answers an imaging solve, and for each of the three stars listed in
     `captures.json`, which do not lie in one line, `pixel_of` of its listed catalogue
     place is within 1.5 pixels of its listed pixel. The tolerance is in the
     picture's own pixels because a star's listed pixel is its brightest sample, good
     to half a pixel, whatever the picture's scale;
   - `not_solved` answers `NotSolved` with the detail `No solution found!`,
     `too_few_stars` answers `TooFewStars` with the detail `Not enough stars.`, and
     `no_database` answers `NoDatabase` with the detail `No star database found.`, each
     from the saved exit code, 1 for all three, and the case's saved `.ini` as
     `reply.ini`;
   - `analyse` answers the `half_flux_px` and `stars` that `captures.json` lists,
     within 1e-9;
   - oracle: the saved captures; the stars' places come from a catalogue, not from
     ASTAP, at the epoch of ASTAP's star databases (2025), and three of them fix the
     mirror sense as well as the angle and the scale.

---

### User Story 2 - Solve the latest imaging frame on the console (Priority: P1)

As the observer, I take an imaging frame and ask the console where it points. The answer
appears beside the frame and is written next to its file. On a desk, the simulated
solver answers from the simulator's truth.

**Why this priority**: It is the first thing the page and an MCP client can ask for, and
every later story reads what it stores.

**Independent Test**: `crates/finder/tests/console/imaging_solve.rs`, the module
`imaging_solve` of finder's `console` test binary, and, for US2-S6, which starts the
built `finder`, `crates/finder/tests/cli/imaging_solve.rs`, the module `imaging_solve`
of `cli`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs both; during
development, `cargo nextest run -p finder --locked --test console imaging_solve::` and
`cargo nextest run -p finder --locked --test cli imaging_solve::`.

**Acceptance Scenarios**:

1. **Given** the solving rig and a saved frame, **When** a test reads the frame's file,
   sends `POST /api/imaging/solve` and waits for the solve, **Then** a committed test
   asserts that:
   - the answer to the request was 200 with `solve.frame_id` the frame's id and
     `solve.state` `solving` or `solved`;
   - afterwards `solve.state` is `solved`, `solve.solver` is `simulated`, the place
     (`solve.ra_deg`, `solve.dec_deg`) is within 0.5″ of the frame's truth,
     `solve.up_east_of_north_deg` within 0.01° of it, `solve.arcsec_per_px` within
     0.001 of it, `solve.mirrored` false, `solve.stars` at least 30, and `solve.file`
     `frames/2026-01-16/000001.solve.json`;
   - that file under R parses with `format` `imaging-solve/1`, `frame_id` 1, `frame`
     `000001.fits`, `solver` `simulated`, `solved` true, `width` 1236, `height` 822,
     and `ra_deg`, `dec_deg`, `up_east_of_north_deg`, `arcsec_per_px` and the four
     numbers of `cd_deg` each within 1e-9 of the view's;
   - the folder `frames/2026-01-16` holds no name ending `.part`, and the frame's file
     has the bytes it had before the solve;
   - `GET /api/session` then has an `imaging.solve` with the same `frame_id`, `state`
     and `ra_deg`;
   - oracle: the simulator's truth.
2. **Given** a `SimulatedSolver::new(sim)` over the solving rig's simulator, called
   directly with imaging frames of the test's own description (no file is read), their
   exposure starting at Unix 1768528810 and lasting 1 s, T the truth at 1768528810.5
   and s its scale, **Then** a committed test asserts that:
   - a whole frame at binning 2 (618 × 411, region 0, 0, 1236, 822) with no hint
     answers a centre within 0.01″ of T and `arcsec_per_px` 2·s within 1e-6;
   - a frame of the region 0, 0, 1236, 412 at binning 1 (the top half of the sensor)
     answers a centre whose separation from T is `205·s` within 0.5″ (about 447.8″),
     and `pixel_of` of T is (618.0, 411.0) within 0.01. The test first counts the
     seeded stars in that region itself and requires at least 40, so that a later
     change of the simulator's star density fails here and not as a `TooFewStars`:
     for each star of `deep_field(FIELD_SEED, T.ra_deg, T.dec_deg, 0.5)`, with ρ its
     separation from T in arcseconds, θ the position angle from T to it and φ the
     truth's angle of up, the star is counted when `618 − ρ·sin(θ − φ)/s` is at least
     0 and under 1236 and `411 − ρ·cos(θ − φ)/s` is at least 0 and under 412. The
     simulator's density, 320 stars a square degree, puts 60 there on average, and 120
     in a whole frame;
   - a frame of the region 0, 0, 128, 96, which holds 1.4 stars on average, answers
     `TooFewStars`;
   - a whole frame at binning 1 with a hint 5° north of T and a radius of 3° answers
     `NotSolved`, and with a hint 2° north and the same radius it solves; with a field
     height of twice `822·s/3600`, or of 1.06 times `822·s/3600`, it answers
     `NotSolved`, and with 1.04 times `822·s/3600` it solves;
   - `measure` of a whole frame at binning 1 answers `half_flux_px` 2.825784 within
     1e-6 and at least 30 `stars`; after `set_imaging_optics` with `focus_error` 2.0,
     5.492337 within 1e-4, and 2.746168 within 1e-4 for the whole frame at binning 2;
   - oracle: the simulator's truth and its point-spread constants (`2·√(2·ln 2)` times
     a width of `√(1.2² + (1.0·focus_error)²)` pixels); the 3° and 5% rules are the
     simulated adapter's own, stated in FR-008.
3. **Given** the solving rig, **Then** a committed test asserts that `POST
   /api/imaging/solve`:
   - before any imaging frame answers 409 and `take an imaging frame first`;
   - on a rig of its own with a saved frame, after the simulator's
     `set_unresponsive(true)` (`crates/simulator/src/controls.rs:115`) and a wait
     until `state` is `no_link`, answers 409 and `the scope node is not connected`,
     031's words for that state;
   - with an exposure of 10000 ms set and a frame just asked for answers 409 and `the
     imaging frame is still arriving`; and 1 s later, after `POST /api/imaging/abort`
     and a wait until `frame.state` is `aborted`, 409 and `the latest imaging frame was
     not saved` (an abort sent before the scope node has begun the exposure ends
     nothing, and the frame is then saved; 031's US5-S2 waits 1 s too);
   - with a saved frame and a foreign `Origin` answers 403;
   - with a solver built `with_delay` of 5 s, a saved frame and a first request
     answered 200, answers a second request 1 s later with 409 and `an imaging solve is
     already running`, and the first solve still ends `solved`;
   - on a console built as the solving rig but without `with_imaging_solver` answers
     409 and `this console has no imaging solver`, and that console's
     `AppState::imaging()` answers an `Imaging` whose `solver()` is `None`, where the
     solving rig's has the `name()` `simulated`;
   - on a console built without `with_imaging` answers 409 and `this console takes no
     imaging frames`;
   - after each refusal, R holds no `.solve.json` file that was not there before;
   - oracle: the stated words and statuses, compared exactly.
4. **Given** the solving rig built instead with a solver of the test's own, which
   answers its calls in order, each failed answer after 1.5 s of tokio time and the
   last at once, with `NotSolved` and the detail `No solution found!`, `TooFewStars`,
   `NoDatabase`, `TimedOut { seconds: 120 }` (the test's solver waits only its 1.5 s;
   the limit itself is US1-S5's), `Failed` with `the imaging solver ended with exit
   code 34`, and then worked answer B's solve, **When**
   a test solves one saved frame six times, **Then** a committed test asserts that the
   first five end with `solve.state` `failed` and, in order, `solve.reason` and
   `solve.error` of `not_solved` and `the imaging solver found no solution` (with
   `solve.detail` `No solution found!`), `too_few_stars` and `the imaging solver found
   too few stars in the imaging frame`, `no_database` and `the imaging solver has no
   star database it can read`, `timed_out` and `the imaging solver gave no answer
   within 120 s`, `failed` and `the imaging solver ended with exit code 34`; that each
   of the five also has `solve.file` `frames/2026-01-16/000001.solve.json` and a
   `solve.took_ms` of 1500 within 1, the console clock's time from the request to the
   failed answer; that after each the sidecar has
   `solved` false, the same `reason` and `error`, and a `took_ms` equal to the view's;
   and that after the sixth the sidecar has `solved` true and no `reason`. Oracle: the
   words FR-001 gives each `SolveError`, compared exactly, and the test solver's own
   1.5 s on the virtual clock (answered at once, a `took_ms` hard-wired to 0 would
   pass).
5. **Given** the solving rig, **Then** a committed test asserts that `GET /api/imaging`
   before any solve has no `solve` key; and, on a console built without `with_imaging`,
   that its body is exactly `{"state":"off"}`. Oracle: the stated keys and body; the
   second is what 031's own scenario asserts of that console.
6. **Given** the built `finder` run with `--simulate`, `--state S` and `--listen` on
   port 0 of `127.0.0.1`, S a scratch folder, **When** a test waits until `GET
   /api/imaging` has `state` `ready`, takes a frame, and solves it, **Then** a
   committed test asserts that `solve.state` becomes `solved` with `solve.solver`
   `simulated`, and that the `.solve.json` file `solve.file` names exists under
   `S/imaging`. **And** the committed test asserts that `finder --astap /x` exits with
   a failure and names `--astap-db-name` on standard error, and that `finder --simulate
   --astap /x --astap-db-name d50` exits with a failure and names both `--simulate` and
   `--astap` there. Oracle: the stated state, solver name, file and flag names; the
   solve's numbers are US2-S1's to prove.

---

### User Story 3 - Hints, the wide first search and the kept scale (Priority: P1)

As the console, I tell the imaging solver where to look and how tall the field is: from
the live pointing, the last imaging solve or the target, searching wide at first, and
with the scale the first solve measured from then on. When three solves in a row find
nothing, I try the next place to look and the next field height.

**Why this priority**: Without a hint ASTAP searches the whole sky; with a wrong field
height it does not solve at all.

**Independent Test**: `crates/finder/tests/console/imaging_hints.rs`, the module
`imaging_hints` of finder's `console` test binary, and, for US3-S6, which starts the
built `finder`, `crates/finder/tests/cli/imaging_hints.rs`, the module `imaging_hints`
of `cli`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs both; during
development, `cargo nextest run -p finder --locked --test console imaging_hints::` and
`cargo nextest run -p finder --locked --test cli imaging_hints::`.

**Acceptance Scenarios**:

1. **Given** the solving rig, **When** a test waits until the session's `pointing` has
   the `source` `live`, takes a frame and solves it, **Then** a committed test asserts
   that `solve.hint.source` is `pointing`; `solve.hint.radius_deg` is 3.0; the place
   (`solve.hint.ra_deg`, `solve.hint.dec_deg`) is within 0.05° of the finder camera's
   true centre at the middle of the exposure (`sim.directions_at(t).camera`); the hint
   has no `fov_deg` key; the solve is `solved`; and the hint is more than 0.95° and
   less than 1.15° from the solved centre. Oracle: the simulator's truth (the main
   telescope looks 127 finder pixels, 1.05°, from the finder camera's centre).
2. **Given** the solve of US3-S1, **Then** a committed test asserts that
   `relation.scale.arcsec_per_px` is within 0.001 of the truth's scale and
   `relation.scale.frame_id` is that frame's id; that a second frame's solve has
   `solve.hint.fov_deg` within 1e-4 of `822·relation.scale.arcsec_per_px/3600` (about
   0.4988°); that, after `PUT /api/imaging/settings` with `{"binning":2}`, a third
   frame's solve has a `solve.hint.fov_deg` within 1e-3 of the same figure and is
   `solved`; and that `relation.json` under R parses with `format`
   `imaging-relation/1`, a `camera.model` equal to the imaging view's `camera.model`,
   and, after the third solve, a `scale.frame_id` of the third frame's id and a
   `scale.arcsec_per_px` within 1e-9 of the first solve's (a scale kept per delivered
   sample would be twice it at binning 2). **And** the committed test asserts
   that `Imaging::scale_arcsec_per_px()`, reached through `AppState::imaging()`, is
   `None` before the first solve and the view's `relation.scale.arcsec_per_px` within
   1e-12 after it. Oracle: the simulator's truth.
3. **Given** the solving rig built with `with_focal_length_mm(1800.0)` after
   `with_imaging_solver`, **Then** a committed test asserts that
   `Imaging::scale_arcsec_per_px()` is 2.190991 within 1e-5 before any solve; that the
   first solve's `solve.hint.fov_deg` is 0.500276 within 1e-5 (822 rows of 19.12 µm at
   1800 mm: 2.190991″ a pixel); and that it is `solved`. **Given** it built with
   `with_focal_length_mm(900.0)`, **Then** the committed test asserts that the first
   solve has `solve.hint.fov_deg` 1.000552 within 1e-5, ends `failed` with the reason
   `not_solved`, leaves no `relation.scale`, and that `relation.focal_length_mm` is
   900.0. Oracle: the worked arithmetic `206264.806 × pixel size in mm / focal
   length`, and the simulated solver's 5% rule (1800 mm is 0.3% from the truth, 900 mm
   is 100%).
4. **Given** the solving rig, **Then** a committed test asserts that:
   - with one frame solved, the simulator's star frames then stopped
     (`Simulator::stop_frames`, `crates/simulator/src/controls.rs:106`) and 120 s
     waited, the next frame's solve has `solve.hint.source` `last_solve`, a hint place
     within 1e-9° of the first solve's centre and `radius_deg` 3.0;
   - on a fresh rig whose star frames are stopped as soon as `state` is `ready`, and
     120 s waited, with `PUT /api/session` `{"target":"M42"}`, a frame's solve has
     `solve.hint.source` `target`, a hint place equal to the session's
     `target.ra_deg` and `target.dec_deg` within 1e-9°, `radius_deg` 3.0, and ends
     `failed` with the reason `not_solved`;
   - on another such rig with no target, a frame's solve has `solve.hint.source`
     `none`, no `ra_deg` or `dec_deg` key in the hint, `radius_deg` 180.0, and is
     `solved`;
   - on a rig of its own with no target and one frame solved, the star frames then
     stopped and 120 s waited as in the first bullet, and then `PUT /api/session`
     `{"target":"M42"}`, the next frame's solve has `solve.hint.source` `target`: the
     last imaging solve stops being a source when the session's target changes;
   - on a rig of its own whose session has the target M42 before the first frame, with
     one frame solved through `pointing`, the star frames then stopped and 120 s
     waited, the next frame's solve has `solve.hint.source` `last_solve`: the target is
     unchanged, so the last imaging solve is still a source;
   - oracle: the stated order of sources, each solve here being the first since the
     last solved one or the first of its rig; M42 is nearly 30° from where the rig
     points.
5. **Given** the R of a rig that has solved one frame, **When** a second console is
   built over the same R as the solving rig is, **Then** a committed test asserts that
   `relation.scale.arcsec_per_px` reads the first rig's value before any solve and that
   its first solve's `solve.hint.fov_deg` is within 1e-4 of `822` times it over 3600.
   **Given** instead that the test first rewrites `relation.json` with its
   `camera.serial` changed to `OTHER`, **Then** the committed test asserts that the
   second console's `relation` has no `scale` key before its first solve, and that the
   first solve's hint has no `fov_deg` key. Oracle: the first rig's own value, read by
   the test from its view and from the file; the stated keys.
6. **Given** the built `finder` run with `--simulate`, `--state S`, `--focal-length-mm
   1800` and `--listen` on port 0 of `127.0.0.1`, S a scratch folder, **Then** a
   committed test asserts that `GET /api/imaging`, once its `state` is `ready`, has
   `relation.focal_length_mm` 1800.0; and that `finder --focal-length-mm 0` exits with
   a failure and names `--focal-length-mm` on standard error. Oracle: the flag's own
   value and name.
7. **Given** the solving rig with one frame solved, so that a scale is kept, **When**
   a test widens the simulated imaging camera's field with `set_imaging_optics` to a
   `fov_deg` of 1.5, every other field as `imaging_optics()` gave it, and solves five
   more saved frames, one after another, **Then** a committed test asserts that:
   - the second, third and fourth solves each have a `solve.hint.fov_deg` within 1e-4
     of 822 times the kept scale over 3600 (about 0.4988°), end `failed` with the
     reason `not_solved`, and leave `relation.scale.frame_id` 1;
   - the fifth solve's hint has no `fov_deg` key, it is `solved`, and afterwards
     `relation.scale.frame_id` is 5 and `relation.scale.arcsec_per_px` is within 0.001
     of the new truth's scale (about 4.3692″ a pixel, twice the old);
   - a sixth solve has a `solve.hint.fov_deg` within 1e-4 of 822 times the new scale
     over 3600 (about 0.9976°) and is `solved`;
   - on a rig built with `with_focal_length_mm(1800.0)` the same change takes three
     solves more: the second, third and fourth solves have the kept scale's field
     height as above and fail; the fifth, sixth and seventh have a
     `solve.hint.fov_deg` of 0.500276 within 1e-5, the focal length's, and fail; the
     eighth has no `fov_deg` key and is `solved`;
   - a `Relation` of the test's own, made with `Relation::new` on a path in the
     scratch folder, `describe`d as a camera of the test's own, given
     `set_focal_length_mm(1800.0)` and `store_scale(2.0, 1, 0.0)`, answers
     `height_inputs()` with `(Some(2.0), Some(1800.0))`; after three `miss()`,
     `(None, Some(1800.0))`; after three more, `(None, None)`; after three more,
     `(Some(2.0), Some(1800.0))` again; after one more `miss()` and then
     `store_scale(2.1, 2, 1.0)`, `(Some(2.1), Some(1800.0))`; and after two `miss()`
     more, still that, the count having started again from the solved solve;
   - oracle: the simulator's truth before and after the change, and the stated rule
     of three; the simulated solver refuses a field height more than 5% out.
8. **Given** the solving rig with the session's target set by `PUT /api/session`
   `{"target":"M42"}` and a tapped aim point at (96, 420), set with
   `state.session().set_aim(Some(Aim { x: 96.0, y: 420.0 }), now)` before the first
   frame is taken: about 4.98° from the true aim point along the finder camera's rows,
   so that a live pointing taken there lies that far from where the imaging camera
   looks, **When** a test
   waits until the session's `pointing` has the `source` `live` and solves seven saved
   frames, one after another, **Then** a committed test asserts that:
   - the first, second and third solves have `solve.hint.source` `pointing`,
     `solve.hint.radius_deg` 3.0 and a hint place more than 4.5° from the frame's
     truth, and end `failed` with the reason `not_solved`;
   - the fourth, fifth and sixth have `solve.hint.source` `target`, a hint place equal
     to the session's `target.ra_deg` and `target.dec_deg` within 1e-9°, and
     `radius_deg` 3.0, and end `failed` with the reason `not_solved`: no imaging solve
     has succeeded, so there is no `last_solve` between the two;
   - the seventh has `solve.hint.source` `none`, no `ra_deg` or `dec_deg` key in the
     hint and `radius_deg` 180.0, and is `solved`;
   - on a rig of its own built the same way but with no target, the fourth solve has
     `solve.hint.source` `none` and `radius_deg` 180.0 and is `solved`;
   - a `Relation` of the test's own, made and described as in US3-S7, answers
     `hint_skip()` 0 at first and after two `miss()`, 1 after the third, 2 after the
     sixth, and 0 again after `store_scale(2.0, 1, 0.0)`;
   - `solving::hint_for`, called with a `HintInputs` literal for a whole frame 1236 ×
     822 at binning 1 exposed from Unix 1768528810 for 1 s, `sensor` (1236, 822), a
     `pointing` at RA 80°, Dec −5° timed 1768528810.5, a `last_solve` at RA 81°, Dec
     −5°, a `target` at RA 82°, Dec −5°, no `scale`, `pixel_um` 19.12, no
     `focal_length_mm` and `aim_measured` false, answers the source `pointing` with
     `skip` 0, `last_solve` with 1, `target` with 2, and `none` with radius 180.0
     with 3, with 4 and with 7 (a source picked at `skip` modulo the count would give
     `pointing` with 4); and, with `last_solve` and `target` both `None`, `none` with
     `skip` 1;
   - on a rig of its own with the target M42 and no aim point, built with an imaging
     solver of the test's own that answers `SolveError::TimedOut` to its first three
     solves, `SolveError::TooFewStars` to the next three and the simulated solver's
     answer after that, which waits until the session's `pointing` has the `source`
     `live` and solves seven saved frames, the first three solves have
     `solve.hint.source` `pointing` and the fourth to seventh `target`: `timed_out` is
     a miss and `too_few_stars` is not;
   - oracle: the stated order of sources and rule of three; the hint's distance from
     the truth is the test's own separation over the simulator's truth (along the
     rows, the tapped pixel is 632 finder pixels from the frame's centre, 728, and the
     true aim point 28, at a focal length of 728/tan 6° = 6926.5 pixels:
     atan(632/6926.5) − atan(28/6926.5) ≈ 4.98°);
     M42 is nearly 30° from where the rig points (US3-S4).

---

### User Story 4 - The aim point and imaging angle, measured from one moment (Priority: P1)

As the observer with an imaging camera in the focuser, I never tap a star. When an
imaging frame solves while the finder camera is solving too, the console works out
where the imaging camera's centre falls in the finder camera's frame and how the two
frames are turned, and moves the telescope by that aim point from then on. While Track
is on, a newly measured aim point waits until Track is off, so the telescope never
lurches after it. Without an imaging camera I still tap, as before.

**Why this priority**: It is the relation between the two cameras that the whole imaging
run steers by.

**Independent Test**: `crates/finder/tests/console/imaging_relation.rs`, the module
`imaging_relation` of finder's `console` test binary, and, for US4-S8, which turns Track
on and so moves the simulated mount, `crates/finder/tests/motion/imaging_track.rs`, the
module `imaging_track` of finder's `motion` test binary. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs both; during
development, `cargo nextest run -p finder --locked --test console imaging_relation::`
and `cargo nextest run -p finder --locked --test motion imaging_track::`.

**Acceptance Scenarios**:

1. **Given** the solving rig, **When** a test waits until the session's `pointing` has
   the `source` `live`, waits 5 s more, takes a frame and solves it, **Then** a
   committed test asserts that `GET /api/session` has an `aim` whose `x` is within 1.5
   of 700 and whose `y` is within 1.5 of 420, with `source` `measured`; that
   `imaging.relation.aim` has the same `x` and `y` within 1e-9; that
   `imaging.relation.angle_deg` is within 0.25° of 17.0; that
   `imaging.relation.mirrored` is false, `imaging.relation.readings` is 1,
   `imaging.relation.last.frame_id` is 1 and `imaging.relation.last.outcome` is
   `measured`; and that
   `imaging.relation.field.width_arcsec` and `height_arcsec` are within 0.5″ of 1236
   and 822 times the truth's scale. Oracle: the simulator's true aim point and true
   imaging angle; 1.5 finder pixels is the 30″ existing tests allow a live pointing
   (Clarification 8), plus half a pixel.
2. **Given** `relation::read_aim` called with finder fixes of the test's own making
   (the shared setup says how one is built), an exposure from 9.7 to 10.7, whose
   middle is 10.2, and an imaging solve C made with `ImagingSolve::from_cd` for a
   frame 1236 × 822, with up 47° east of north at 0.0006° a pixel, centred on the sky
   direction that `unproject((700.0, 420.0))` gives in a finder frame centred on RA
   100°, Dec 20°, **Then** a committed test asserts that:
   - **steady drift**: nine fixes at the times 8.0, 8.5, … 12.0, the one at time t
     centred on RA `100 + 0.004434074·(t − 10.2)`°, Dec 20° (the sky drifting 15.0″ a
     second), give `Measured` with `x` 700.0 and `y` 420.0 within 0.01, `at` 10.2,
     `mirrored` false, `finder_arcsec_per_px` 29.779 within 0.001, `finder_width`
     1456.0, `finder_height` 1088.0, and `angle_deg` within 0.01° of 47° less
     `up_east_of_north_at` of the fix at 10.0 at C's centre. The fix nearest the
     middle, taken alone, would be 0.1 pixel out;
   - **a still telescope and a scattering finder camera**: nine fixes at 8.2, 8.7, …
     12.2, the first, third, fifth, seventh and ninth centred on RA 100.005912099°
     and the others on RA 99.994087901°, all at Dec 20° (20.0″ either side on the
     sky), give `Measured` with `x` within 0.1 of 700.0 and `y` within 0.01 of 420.0.
     The line through them is flat and sits at their mean, 2.2″ from the middle, and
     they scatter 22″ about it; two neighbours differ by 40″, which the difference of
     two fixes would have read as 80″ of travel in this 1 s exposure;
   - **within the allowance**: the fixes of the second bullet drifting besides at
     0.018623111° of RA a second (63″ a second on the sky) give `Measured`: the line
     travels 63″ during the exposure, but the fixes scatter 22.5″ about it over D =
     15 s², so its travel is uncertain by 5.8″, and 63 less 5.8 is under the 60″
     allowed;
   - **moving**: the fixes of the first bullet drifting instead at 0.029560494° of RA
     a second (100″ a second on the sky) give `Moving`: the line travels 99″ during
     the exposure;
   - **unsteady**: the fixes of the second bullet with the fifth, the one at 10.2,
     centred instead on RA 100.5° give `Moving`: the line is still flat, and the fixes
     scatter 600″ about it, against the 90″ allowed;
   - **no live pointing**: the first bullet's fixes at 10.0 and 10.5 alone give
     `NoLivePointing`, two being fewer than three; and all nine of them with an
     exposure from 12.3 to 12.9, which holds none, give `NoLivePointing`;
   - **off the frame**: the first bullet's fixes with an imaging solve centred on RA
     100°, Dec 40° give `OffFrame`;
   - **the history**: a `FinderHistory` given 301 fixes at the times 0, 1, … 300 with
     `keep_s` 120.0 then holds 121, the oldest at time 180; and one given 3001 fixes
     at the times 0, 0.5, … 1500 with `keep_s` 1200.0 (four exposures of 300 s) holds
     1201, at the whole seconds from 300 to 1500: a fix that follows the last one kept
     by less than 1200/2000 = 0.6 s is not kept, so the history is thinned to one a
     second and still spans 1200 s, where the newest 2000 fixes would span 1000 s;
   - oracle: worked examples. A drift that is steady in right ascension is a straight
     line in the finder frame to well under 0.01 pixel over these four seconds, so the
     fitted line's value at the middle is where the finder frame centred on RA 100°
     puts C's centre: (700, 420).
3. **Given** `Frame::up_east_of_north_at` on a frame made with `Frame::from_solution`
   for RA 100°, Dec 20°, 1456 × 1088 over 12°, **Then** a committed test asserts that
   with roll 0 it answers 0.0 within 1e-9 at the frame's centre; with a roll of 0.3 rad
   it answers 360 less 17.188734, within 1e-6, at the centre; and that at the sky
   direction of pixel (700, 420) it differs from the centre's answer by less than
   0.5°. Oracle: the rotation's definition (`crates/finder/src/sky.rs:17`): up is north
   turned by the roll toward the west.
4. **Given** the solving rig, **Then** a committed test asserts that:
   - with the simulator's star frames stopped, 5 s waited, a frame taken and saved, the
     star frames started again and the frame solved, the solve is `solved`,
     `relation.last.outcome` is `no_live_pointing`, and the session has no `aim`;
   - on a fresh rig, with `exposure_ms` 8000, a frame's solve is `solved`,
     `relation.last.outcome` is `moving`, and the session has no `aim` (the untracked
     sky crosses about 117″ in 8 s, against the 60″ allowed);
   - on a fresh rig built with `with_focal_length_mm(900.0)`, a failed solve leaves no
     `relation.last` and no `aim`;
   - on a fresh rig, with the session's `pointing` live and 5 s more waited, a frame
     taken and saved and its truth read, the simulated telescope then swung away with
     `sim.push_to(50.0, 200.0)` and a wait until the session's `pointing` is more than
     20° from the frame's truth, the simulated telescope swung back with
     `sim.push_to(50.0, 120.0)` and, with no wait between, the solve asked for (the
     simulated adapter places its answer by the telescope's present altitude and
     azimuth, `Simulator::imaging_truth_at`, while the session's latest pointing is
     still the swung one), the frame's solve has `solve.hint.source` `pointing`, a hint
     place within 1.2° of the frame's truth, and is `solved`: the hint is the live
     solve of the exposure's own moment, not the session's latest pointing when the
     solve was asked for, which is some 49° away and outside any radius;
   - oracle: the stated rules; the simulator's truth and its sidereal drift.
5. **Given** the solving rig with a measured aim point in force, as after US4-S1,
   **Then** a committed test asserts that:
   - a tap is obeyed. The test takes a live preview that holds at least one star, as
     `crates/finder/tests/console/live/whole_loop.rs:63` takes one from a report's
     `preview_seq`, and sends `POST /api/aim` with that preview's `frame_id` and the
     `x` and `y` of its centroid nearest (700, 420), each rounded to a whole number
     (the next solve's hint is the live pointing taken at the tapped pixel, so a star
     more than 3°, some 360 finder pixels, from (700, 420) would put the hint outside
     its radius; the first centroid of the rig's preview is 483 pixels away). The
     answer is 200 with an `aim` of exactly the two keys `x` and `y`, each within 0.5
     of that centroid's; `GET /api/imaging` then has `relation.readings` 0 and no
     `relation.aim` or `relation.angle_deg` key, and still has `relation.scale`;
   - the next frame's solve measures again: its `solve.hint` place is within 0.1° of
     the session's `pointing` read when the solve was asked for (both are taken at the
     tapped pixel), and the session's `aim` has `source` `measured` and is within 1.5
     pixels of (700, 420);
   - `DELETE /api/aim` then answers 200 with no `aim` key, and `relation.readings` is
     0;
   - on a solving rig whose session was made with `Session::with_state_file` in the
     scratch folder, after one aim reading and with the console dropped,
     `session.json` holds `aim_measured` true, and a new `Session::with_state_file`
     over it reads an `aim` with `source` `measured` and the same `x` and `y`; and
     after `set_aim` with a pixel, `session.json` has no `aim_measured` key;
   - oracle: the stated keys and statuses, compared exactly; the centroid the test
     itself read from the preview; and the simulator's true aim point.
6. **Given** a console built as `crates/finder/tests/support/live.rs:60` builds its
   state with an aim point, with no imaging, **Then** a committed test asserts that
   `GET /api/session` has an `aim` equal to exactly `{"x":700.0,"y":420.0}` and that
   `GET /api/imaging` is exactly `{"state":"off"}`: the tapped aim point works without
   an imaging camera, and reads as it always has. Oracle: the pixel the helper set,
   and the two-key form the six existing assertions of Clarification 2 pin.
7. **Given** the solving rig, **When** a test waits until the session's `pointing` has
   the `source` `live`, waits 5 s more and takes a saved frame, builds that frame's
   `SolveFrame` from the imaging view's `frame` as `Imaging::solve` builds one (plan.md
   § Solving on the console), solves it itself with
   `SimulatedSolver::new(sim.clone()).solve(frame, SolveHint::whole_sky())`, and hands
   the answer to `note_solve` of the `Imaging` that `AppState::imaging()` answers, with
   the frame's id, **Then** a committed test asserts that:
   - the session's `aim` has `x` within 1.5 of 700, `y` within 1.5 of 420 and `source`
     `measured`; `relation.readings` is 1, `relation.last.frame_id` is the frame's id
     and `relation.last.outcome` is `measured`, as after US4-S1;
   - `GET /api/imaging` then has no `solve` key, its `relation` has no `scale` key, and
     `Imaging::scale_arcsec_per_px()` is `None`: neither the view's solve nor the kept
     scale changed;
   - a second saved frame, handed over with its own id as a `SolveFrame` whose `roi`
     the test sets to the region 0, 0, 1236, 412 and whose `width` and `height` are
     1236 and 412, with the first frame's answer (no solve is needed: the region is
     refused before the answer is read), leaves `relation.last.frame_id` the first
     frame's id and `relation.readings` 1: a frame of a region takes no aim reading;
   - oracle: the simulator's true aim point, as in US4-S1; the stated keys.
8. **Given** the tracking rig, which is the solving rig with two changes: it is built
   with `with_live_solver(Arc::new(crate::support::goto::Stars))` in place of
   `with_live()`, so that the finder camera's solves stay on the paused runtime and
   GoTo and Track run in virtual time (`crates/finder/tests/support/goto.rs:42`), and
   its session has the target M42, still with no aim point; **When** a test waits until
   the pointing model holds a sync (`state.session().model(now).syncs` not empty, read
   every 100 ms of virtual time, for at most 300 s;
   `crates/finder/tests/support/goto.rs:218` reads it the same way, once a tick),
   sends `POST /api/track` with `{"on":true}`, waits until the session's `track.state`
   is `tracking`, waits 15 s more (the fit reaches 10 s before the exposure, and the
   GoTo's last approach must be outside it), takes a saved frame and solves it,
   **Then** a committed test asserts that:
   - the solve is `solved`, `relation.last.outcome` is `measured`, `relation.held` is
     true, and `relation.aim` has `x` within 1.5 of 700 and `y` within 1.5 of 420;
   - the session has no `aim` key, and its `track.state` is still `tracking`;
   - the main telescope's true direction, `sim.directions_at(t).eyepiece` read every
     100 ms of virtual time for the 5 s after the test saw the solve end, is never more
     than 60″ from its first reading, the separation computed as the shared setup
     says; and `state.session().model(t).slip_pending`, read with each of those
     readings, is never true (an aim point applied at once is read as a push by hand,
     which marks the model's slip);
   - after `POST /api/track` with `{"on":false}`, the session's `aim` has the `source`
     `measured` within 5 s, its `x` and `y` equal `relation.aim`'s within 1e-9, and
     `relation` then has no `held` key;
   - oracle: the simulator's truth. 60″ is the bound existing tracking tests hold the
     telescope to (`crates/finder/tests/support/tracker.rs:3`). An aim point applied at
     once moves the live pointing by about 127 finder pixels, 3800″; 038's
     push-by-hand rule (`Session::track_live_pointing`) reads that step as a push,
     marks the model's slip and hands nothing over, so the telescope does not chase it
     either way. The 60″ bullet guards Track against any other lurch; `relation.held`,
     the absent `aim` and the model's unmarked slip are what fail on an aim point
     applied at once.

---

### User Story 5 - Smooth the aim readings, set outliers aside and keep them (Priority: P2)

As the observer, I want the aim point to settle and stay put: one odd solve must not
move the telescope's reference, a camera I have turned in the focuser must take over
after a few solves, and a restart must not forget what was measured.

**Why this priority**: A single aim reading still carries some of the finder camera's
scatter and all of one imaging solve's error; the run needs a steady value, and a
narrow search once it has one.

**Independent Test**: `crates/finder/tests/console/imaging_smoothing.rs`, the module
`imaging_smoothing` of finder's `console` test binary. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test console imaging_smoothing::`.

**Acceptance Scenarios**:

1. **Given** a `relation::Window`, **When** a test gives it aim readings of its own
   making, each an `AimReading` literal that is not mirrored, with a finder scale of
   30″ a pixel and a finder frame 1456 × 1088 unless the bullet says otherwise,
   **Then** a committed test asserts that:
   - after (700, 420, 17.0°), (702, 421, 17.2°), (698, 419, 16.8°), (701, 420.5,
     17.1°) and (699, 419.5, 16.9°), each `Taken::Measured`, `aim()` is (700.0, 420.0)
     and `angle_deg()` 17.0, within 1e-9, and `len()` is 5;
   - after a sixth, (703, 422, 17.3°), `len()` is still 5 and `aim()` is (701.0,
     420.5): the first has left;
   - two readings (700, 420, 359.0°) and (702, 421, 1.0°) give `aim()` (701.0, 420.5)
     and `angle_deg()` 0.0 within 1e-9; five at (700, 420) with the angles 359.8°,
     0.2°, 0.0°, 359.6° and 0.4° give 0.0 within 1e-9;
   - with three readings at (700, 420, 17.0°), a reading at (705, 420, 17.0°), 150″
     away, is `Taken::SetAside` and leaves `aim()` and `len()` as they were; so, next,
     is one at (700, 420, 18.5°); then one at (703, 420, 17.0°), 90″ away, is
     `Taken::Measured` and leaves `set_aside()` 0; on a fresh window of three readings
     at (700, 420, 17.0°), one at (700, 420, 17.0°) that is mirrored is
     `Taken::SetAside` (taken third in a row on the first window, it would answer
     `Taken::Replaced`); with two readings held, a reading at (760, 400, 40.0°) is
     `Taken::Measured`;
   - with three readings at (700, 420, 17.0°), three in a row at (760, 400, 40.0°) are
     `SetAside`, `SetAside` and `Replaced`, after which `len()` is 3, `set_aside()` is
     0, `aim()` is (760.0, 400.0) and `angle_deg()` 40.0;
   - with three readings at (700, 420, 17.0°), a reading at (350, 210, 17.0°) of a
     finder frame 728 × 544 is `Taken::Measured`, after which `len()` is 1 and
     `aim()` is (350.0, 210.0): readings of a finder frame of another size are
     dropped, since their pixels mean something else;
   - `relation::should_apply` is true with no aim point in force and with a tapped one;
     with a measured one at (700, 420) and 30″ a pixel it is false for a median of
     (700.3, 420.0), 9″ away, and true for (700.4, 420.0), 12″ away;
   - oracle: worked examples.
2. **Given** the solving rig, **When** a test waits until the session's `pointing` has
   the `source` `live`, waits 5 s more, as US4-S1 does, and solves six saved frames,
   one after another, **Then** a committed test asserts that after the fifth and after
   the sixth `relation.readings` is 5; that the session's `aim` is within 1.5 pixels of
   (700, 420) with `source` `measured`; that `relation.angle_deg` is within 0.25° of
   17.0; that `relation.aim` is within 10″ of the session's `aim` at 29.78″ a pixel
   (0.336 pixels); and that the sixth solve's `solve.hint.radius_deg` is within 0.001
   of the sensor's diagonal on the sky,
   `hypot(1236, 822)·relation.scale.arcsec_per_px/3600` (about 0.9007°), with a hint
   place within 0.05° of the solved centre. **And** the
   committed test asserts of `solving::hint_for`, called with a `HintInputs` literal
   whose frame is the region 927, 617, 309, 205 at binning 1 (a quarter-size corner of
   a sensor 1236 × 822), with a `pointing`, a `scale` of 2.1845, `skip` 0 and
   `aim_measured` true, that `radius_deg` is 0.900729 within 1e-5 and `fov_deg` 0.124395 within
   1e-5; that the same with `aim_measured` false gives a `radius_deg` of 3.0; that a
   sensor 100 × 80 at a scale of 2.0 gives 0.2, the floor; and that a sensor
   6000 × 4000 at 2.0 gives 3.0, the ceiling. The corner region's centre is
   `hypot(1081.5 − 618, 719.5 − 411)·2.1845/3600` = 0.338° from the sensor's, and its
   own diagonal is `hypot(309, 205)·2.1845/3600` = 0.225°: the test works both out and
   asserts that the first is under the hint's radius and over the second, which is
   why the radius is the sensor's diagonal and not the frame's. Oracle: the
   simulator's truth, and the worked arithmetic of the radius.
3. **Given** the solving rig with three frames solved, the first once the session's
   `pointing` has the `source` `live` and 5 s more have passed, as US4-S1 waits,
   **When** a test turns the
   simulated imaging camera with `set_imaging_optics` to an imaging angle of 40° and
   solves three more, **Then** a committed test asserts that after the fourth and the
   fifth solve `relation.last.outcome` is `set_aside`, `relation.set_aside` is 1 and
   then 2, `relation.angle_deg` is still within 0.25° of 17.0 and the session's `aim`
   has not changed at all; and that after the sixth `relation.last.outcome` is
   `replaced`, `relation.readings` is 3, `relation.set_aside` is 0 and
   `relation.angle_deg` is within 0.25° of 40.0. **When** it then moves the main
   telescope in the finder camera's frame with `set_optics` to an `eyepiece_px` of
   (760, 400) and solves three more, **Then** the committed test asserts that the
   session's `aim` ends within 1.5 pixels of (760, 400). Oracle: the simulator's truth.
4. **Given** a solving rig whose session was made with `Session::with_state_file` in
   the scratch folder, with three frames solved, the first once the session's
   `pointing` has the `source` `live` and 5 s more have passed, as US4-S1 waits,
   **When** the console is dropped, a second one is built over the same R and session
   file, and the test waits the same way before the second console's first solve (in
   each variant below too), **Then** a committed test
   asserts that before any solve `relation.readings` is 3 and `relation.aim` and
   `relation.angle_deg` read what the first console's last read gave, within 1e-9; that
   `relation.json` holds three `readings`; and that the second console's first solve
   leaves `relation.readings` 4. **Given** instead that the session file is removed
   before the second console is built, **Then** the committed test asserts that its
   `relation` has `readings` 0 and no `aim` key, and still has `scale`; and that its
   first solve leaves `relation.readings` 1, the three readings of `relation.json`
   being dropped since the session's aim point is not the measured one. **Given**
   instead that the session file is kept and the test first rewrites `relation.json`
   with its `camera.serial` changed to `OTHER`, **Then** the committed test asserts
   that the second console's imaging view has no `relation` key before its first
   solve, the readings of another camera being dropped with its scale, while the
   session's `aim` still has `source` `measured`; and that its first solve leaves
   `relation.readings` 1. Oracle: the first console's own values, read by the test
   from its view and from the file; the stated keys.

---

### User Story 6 - The solve on the Imaging page, the field on the chart, the source on Setup (Priority: P2)

As the observer with only the page to hand, I solve the imaging frame I just took and
read where it points; I see the imaging camera's field as an outline on the live chart;
and on Setup I see whether the aim point was measured or tapped.

**Why this priority**: The page is how the observer checks that the two cameras agree
before trusting a GoTo to centre a target on the sensor.

**Independent Test**: `npm test` in `crates/finder/web` runs
`src/components/ImagingSolveBlock.test.tsx`, `src/components/ImagingField.test.tsx` and
`src/components/AimSource.test.tsx`.

**Acceptance Scenarios**:

1. **Given** `ImagingSolveBlock` rendered with an imaging view whose `state` is `ready`
   and whose `frame` is `saved` with the id 3, and no `solve`, **Then** a committed
   test asserts one region named `Imaging solve` holding an enabled button named
   `Solve imaging frame` and no `status` or `alert`. **When** the test clicks it, with
   `fetch` answering 200 and a view, **Then** the committed test asserts that `fetch`
   was called with `/api/imaging/solve` and `{"method":"POST"}`, then `refresh`; and
   that a 409 answer with `{"error":"this console has no imaging solver"}` shows an
   alert with exactly that text and calls no `refresh`. With no `frame`, or a `frame`
   that is `exposing`, the button is disabled. Oracle: the stated names, roles, words
   and request, compared exactly.
2. **Given** the same with a `solve` for frame 3, **Then** a committed test asserts
   that:
   - `state` `solving` shows a status reading exactly `Solving imaging frame 3` and a
     disabled button;
   - `state` `solved` with `ra_deg` 83.8221, `dec_deg` −5.3911, `up_east_of_north_deg`
     30.04, `arcsec_per_px` 2.16, `mirrored` false and `stars` 118 shows a status
     reading exactly `Solved imaging frame 3: RA 83.8221° Dec -5.3911° · up 30.0° east
     of north · 2.16″ per pixel · 118 stars`; without `stars`, the same without its
     last part; with `mirrored` true, the same followed by ` · mirrored`;
   - `state` `failed` with the `error` `the imaging solver found no solution` shows an
     alert reading exactly `Imaging solve failed: the imaging solver found no
     solution.` and an enabled button;
   - oracle: the stated words, compared exactly; each number is the view's, written
     to the stated count of decimals.
3. **Given** the same with a `relation`, **Then** a committed test asserts that:
   - `scale` `{"arcsec_per_px":2.1846,"frame_id":3}` shows a paragraph reading exactly
     `Scale 2.185″ per sensor pixel, from imaging frame 3.`;
   - `aim` `{"x":700.12,"y":419.84}`, `angle_deg` 17.04 and `readings` 3 show a
     paragraph reading exactly `Aim point measured at (700.1, 419.8) px of the finder
     camera's frame · imaging angle 17.0° · from 3 imaging solves.`;
   - `last` with the outcome `moving` shows exactly `The last imaging solve did not
     measure the aim point: the telescope was moving.`; `no_live_pointing`, `…: no
     live pointing shared its moment.`; `off_frame`, `…: its centre is outside the
     finder camera's frame.`; `set_aside`, `…: it disagreed with the earlier ones and
     was set aside.`; and `measured` or `replaced` shows no such paragraph;
   - `held` true shows a paragraph reading exactly `The measured aim point waits until
     Track is off.`, and a `relation` without `held` shows no such paragraph;
   - none of these paragraphs has the role `status` or `alert`;
   - oracle: the stated words, compared exactly.
4. **Given** `ImagingPage` rendered with `fetch` answering `GET /api/imaging` with a
   `ready` view holding a `saved` frame, **Then** a committed test asserts a region
   named `Imaging solve` after the region named `Imaging frame` in document order.
   Oracle: the stated order; the region named `Imaging frame` is the one 031's own
   page scenarios find.
5. **Given** `imagingFieldCorners`, **Then** a committed test asserts, each number
   within 1e-9, that an aim point of (700, 420), an imaging angle of 0°, not mirrored,
   a field of 2700″ × 1800″ and a finder scale of 30″ a pixel give the corners (655,
   390), (745, 390), (745, 450) and (655, 450), in the order top-left, top-right,
   bottom-right, bottom-left of the imaging frame; that an imaging angle of 90° gives
   (670, 465), (670, 375), (730, 375) and (730, 465); that mirrored at 0° gives (745,
   390), (655, 390), (655, 450) and (745, 450); and that a finder scale of 0 gives
   null. Oracle: a worked example; a positive imaging angle turns the imaging frame's
   up toward the finder frame's left.
6. **Given** `Chart` rendered live with a solved report and a session holding `aim`
   `{"x":700,"y":420,"source":"measured"}` and an `imaging.relation` with `angle_deg`
   0, `mirrored` false and a `field` of 90 and 60 times the report's
   `pointing.arcsec_per_px`, **Then** a committed test asserts one element with the
   attribute `data-imaging-field` holding a `title` reading `Imaging field` and a path
   whose `d` begins `M655.0 390.0L745.0 390.0L745.0 450.0L655.0 450.0Z`. **And** the
   committed test asserts that no such element is drawn when the `aim` has no
   `source`, when the chart is not live, when the report has no pointing, or when the
   `relation` has no `field`. Oracle: the corners US6-S5 works out, written to one
   decimal, and the stated conditions.
7. **Given** `AimTap` rendered with a session holding `aim`
   `{"x":700,"y":420,"source":"measured"}` and an `imaging.relation.angle_deg` of
   17.04, **Then** a committed test asserts a paragraph reading exactly `Measured from
   the imaging camera. Imaging angle 17.0°.`, that the text `Aim point: (700, 420) px`
   is still shown, and that the section holds no `status` or `alert`. With an `aim`
   that has no `source`, the committed test asserts a paragraph reading exactly `Set
   by tapping a star.`; with no `aim`, neither paragraph. **Given** `SetupPage` fed by
   `useSession`, as `crates/finder/web/src/components/ScopeNodeSection.test.tsx:70`
   feeds it, with `fetch` answering every request with a session holding a connected
   `node`, that measured `aim` and that `imaging`, **Then** the committed test asserts
   the paragraph `Measured from the imaging camera. Imaging angle 17.0°.` Oracle: the
   stated words, compared exactly.

---

### User Story 7 - One MCP tool (Priority: P3)

As an MCP client, I ask the console to solve the latest imaging frame and get back
where it points, through the same console action and refusals as the page.

**Why this priority**: The owner's goal is an ask from an MCP client; this is the step
after taking a frame.

**Independent Test**: `crates/finder/tests/mcp/imaging_solve.rs`, the module
`imaging_solve` of finder's `mcp` test binary. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test mcp imaging_solve::`.

**Acceptance Scenarios**:

1. **Given** a console with MCP on, **When** a test calls `tools/list`, **Then** a
   committed test asserts that `solve_imaging_frame` is listed with `outputSchema`
   `{"type":"object"}`, no `annotations.readOnlyHint` of true, and the description
   `Solve the latest imaging frame, as POST /api/imaging/solve does. It waits until
   the solve ends and returns the imaging camera's state with the solve.` Oracle: the
   stated name, schema and words, compared exactly.
2. **Given** the solving rig with MCP on, its solver built `with_delay` of 5 s, and a
   saved frame, **When** a test reads the simulator's clock, calls
   `solve_imaging_frame` with `{}` and reads the clock again, **Then** a committed test
   asserts that the clock moved by at least 5.0 s, so the answer came only once the
   solve had ended; that `structuredContent.solve.state` is `solved` with the frame's
   id; and that `structuredContent.solve.ra_deg`, `dec_deg` and `file` equal those of
   `GET /api/imaging` read just after. Oracle: the stated delay on the virtual clock,
   and the route's own view.
3. **Given** the solving rig with MCP on, **Then** a committed test asserts that
   `solve_imaging_frame` with `{"id":1}` answers `isError` true and
   `solve_imaging_frame takes no arguments`; that before any imaging frame it answers
   `isError` true and `take an imaging frame first`; and that on a rig built with
   `with_focal_length_mm(900.0)`, whose solve fails, it answers `isError` true and
   `the imaging solver found no solution`. Oracle: the stated words, which are the
   route's and the solve's own.
4. **Given** a console with MCP on and no `with_imaging`, as
   `crates/finder/tests/support/mcp.rs:24` builds its state, **Then** a committed test
   asserts that `solve_imaging_frame` with `{}` answers within 1 s of tokio time, under
   `start_paused`, with `isError` true and `this console takes no imaging frames`; and,
   on the solving rig built without `with_imaging_solver`, within 1 s of tokio time
   too, `this console has no imaging solver`. No result of the tool
   on the solving rig contains `test-token`. Oracle: the stated words, and the token
   the rig paired with.

---

### Edge Cases

- **No imaging solver.** Without `--astap` and outside `--simulate`, a solve is refused
  with `this console has no imaging solver`; everything of 030 and 031 works as before, and
  the aim point is the tapped one.
- **ASTAP is not installed where the flag says.** The first solve fails with `the
  imaging solver could not be started: …`; the console does not check at start.
- **The database is missing or is not the one named.** ASTAP CLI-2025.09.29 exits 1
  with `ERROR=No star database found.` in its `.ini` (its page documents 32 and 33,
  which the adapter still reads), and the solve fails with the reason `no_database`.
  On a Debian or Ubuntu install the `d50` package puts the database in `/opt/astap`,
  where `astap_cli` does not look by itself, so the console needs
  `--astap-db /opt/astap` (plan.md § Prerequisites 8).
- **A field height on ASTAP's own boundary.** The worked example's frame is 0.4997°
  high, where ASTAP's automatic choice of database changes at 0.5°; the adapter always
  names the database with `-D`
  (`docs/research/2026-11-15-imaging-camera.md` § 3. ASTAP).
- **A frame out of focus, trailed, or of a sparse field.** ASTAP needs about 30 stars
  and ignores streaks and saturated stars; the solve fails with `too_few_stars` or
  `not_solved` and is not tried again wider (owner decision 19). 037 checks focus before
  the first solve.
- **A mirrored imaging solve.** A Newtonian's two mirrors do not mirror the picture; a
  star diagonal does. A mirrored solve is reported with `mirrored` true, measured like
  any other, and drawn mirrored on the chart. Nothing refuses it.
- **A frame of a region of the sensor.** It is solved like a whole one: the field
  height is its own rows, and its centre is its own. Once the aim point is measured
  the hint's place is the sensor's centre and its radius the sensor's diagonal, so a
  region anywhere on the sensor is inside the search (FR-028). The aim point is the
  centre of the sensor, so a frame of a region takes no aim reading (FR-020).
- **The optics change under the same camera.** A reducer, a Barlow, a coma corrector,
  or the camera moved to another telescope: the kept scale is then wrong and the
  console cannot tell, because it knows the camera, not the telescope. Three solves in
  a row fail `not_solved`; the next three take their field height from
  `--focal-length-mm` if it was given; then the hint carries no field height and ASTAP
  finds it. The first solve that succeeds replaces the kept scale (FR-016, owner
  decision 22). ASTAP's own search for the height covers 10° down to 0.3° (its page,
  read 2026-11-15), so a frame under 0.3° high solves only once `--focal-length-mm`
  is the new figure; the README says so.
- **A hint that exists but is wrong.** A finder camera more than 3° off the main axis
  before the aim point is measured, a stale measured aim point under the narrow radius,
  or a target the telescope is not pointed at: three solves in a row fail
  `not_solved`, and the next solve takes the next source that exists, ending at a
  search of the whole sky (FR-014, owner decision 23). The field height moves on at the
  same counts. A solved solve starts both from the first again.
- **A solve with no hint.** With no fresh live pointing, no last imaging solve and no
  target, ASTAP searches the whole sky, and with no scale and no focal length it also
  searches for the field height. That can run to minutes and may end `timed_out`
  after its 600 s limit (owner decision 12). A telescope with no finder camera is
  better served by a target, which hints a 3° search. How long a search of the whole
  sky takes is a field check.
- **A solve asked for long after its exposure.** No live solve of that moment is kept
  any more, so the solve takes no aim reading (`no_live_pointing`), and the hint falls
  to the last imaging solve or the target, the session's latest pointing being more
  than 30 s from the exposure.
- **A solve asked for after the telescope has moved on.** The hint is the live solve
  nearest the exposure's own middle (FR-035), so the solve still looks where the
  imaging frame was taken.
- **The telescope untracked.** A solve still takes an aim reading while the sky's
  drift over the exposure is under 60″; the reading is the value, at the middle of the
  exposure, of a line fitted through the live solves around it.
- **The telescope bumped, or a slew that ends just before the exposure.** The live
  solves around the exposure do not lie on one line; when they scatter about it by
  more than 90″ no aim reading is taken (`moving`).
- **The finder camera lags or is absent.** The history misses reports the live
  broadcast dropped; fewer than three live solves around the exposure, or none inside
  it, take no aim reading. With no finder camera at all the console still solves,
  hinted by the last imaging solve or the target.
- **The observer taps while a measured aim point is in force.** The tap is obeyed; the
  next aim reading replaces it (owner decision 2). A tap or a reset while a measured
  aim point is held back also drops the held one.
- **A measured aim point arrives while Track is on.** 015's tracker takes every live
  pointing as the target's offset from the aim point in force
  (`crates/finder/src/session.rs:1106`) and turns an offset of 10″ or more into a rate
  that removes 60% of it before the next live solve, a rate nothing bounds
  (`crates/finder/src/tracking.rs:342`). A new aim point set at once would hand it the
  whole step, some 3800″ on the simulator, outside GoTo's checks. Since 038 landed, a
  step of more than 5′ is read as a push by hand instead
  (`Session::track_live_pointing`): nothing is handed to the tracker, but the pointing
  model's slip is marked, which the hold avoids too (US4-S8). So the aim point
  waits: it is held while Track is on (`relation.held`, and a line on the Imaging
  page), aim readings taken meanwhile are smoothed against it, and the first live
  solve the session records after Track is off applies it (FR-036, owner decision
  25). A run's re-centre turns Track off, waits for that live solve and turns Track on
  again, and so takes it up (037). A restart while
  one is held forgets it; the next aim reading measures it again.
- **Subs of many minutes.** The history of live solves spans four exposures, thinned to
  at most 2000 fixes over that span, so the fit of an aim reading still finds live
  solves an exposure before and after a sub of 20 minutes (FR-019).
- **A solve made outside `POST /api/imaging/solve`.** A later spec that solves a frame
  itself (035: every accepted sub) offers the solve to `Imaging::note_solve`; a
  whole-sensor frame takes an aim reading as after the route, and a region does
  nothing (FR-037).
- **The camera is turned in the focuser, or the finder camera is knocked.** The next two
  aim readings are set aside and the third replaces the window. Until then moves use
  the old aim point.
- **Another camera, or another finder camera mode.** A stored relation made for a camera
  of another model, serial, size or pixel size is ignored, and aim readings made in a
  finder frame of another size are dropped when the next one is taken.
- **An older `finder` reads a newer `session.json`.** It refuses the unknown
  `aim_measured` key, keeps the file with a `.bad` suffix and starts a fresh session,
  as it does for any state it cannot read.
- **The console exits during a solve.** The work folder of that call is left behind; it
  holds a prepared copy and nothing else of value. No sidecar is half-written: it is
  written under a `.part` name and renamed.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The console SHALL define the solver port: a trait `ImagingSolver` with
  `solve(frame, hint)`, answering an imaging solve or a `SolveError`, and
  `measure(frame, region)`, answering a star measure (the median half-flux diameter in
  imaging-frame pixels and a star count). A `SolveError` SHALL be one of `NotSolved`,
  `TooFewStars`, `NoDatabase`, `TimedOut` or `Failed`, with the reasons and messages of
  US1-S4 and US1-S5. No type of the port names a solver, a camera model or a mount.
- **FR-002**: An imaging solve SHALL hold the J2000 centre of the imaging frame, its
  linear part `cd_deg` (degrees east and north for a step of one pixel right and one
  pixel down), the frame's width and height, and, derived from the linear part exactly
  as worked answer A works them: the angle of up east of north, `atan2(−cd[0][1],
  −cd[1][1])`; the scale, `3600·√|det|`; and `mirrored`, true when the determinant is
  negative. `sky_at` and `pixel_of` SHALL map between imaging-frame pixels and the sky
  by the tangent-plane projection about the centre (US1-S1, US1-S2).
- **FR-003**: The ASTAP adapter SHALL run the configured program on a prepared copy in a
  work folder, with exactly the arguments of US1-S2 and US1-S3: `-f`, `-o`, `-ra` in
  hours to six decimals, `-spd` as declination + 90° to five, `-r` to two and never
  over 180, `-fov` as the hint's field height to four or `0` without one, `-z 1`, `-D`
  with the database name, and `-d` with the database folder when one is configured. A
  hint with no position SHALL be sent as RA 0 h, Dec 0° with a radius of 180°. It adds
  no dependency (owner approval 3).
- **FR-004**: The prepared copy SHALL be a 16-bit picture with the frame's top row
  first: a mono frame as it is; a frame with a Bayer pattern with each whole 2 × 2 cell
  averaged into one sample; and either halved again, 2 × 2, while it is more than 2500
  rows high. A region SHALL be cut first, moved inward to whole cells for a frame with
  a Bayer pattern. The imaging solve SHALL be stated for the imaging frame, not the
  copy (US1-S2, US1-S6). The saved imaging frame SHALL never be written to.
- **FR-005**: The adapter SHALL read the `.ini` the program wrote and map its exit code:
  0 with `PLTSOLVD=T` and the eight numbers to a solve, with `WARNING` kept as the
  solve's warning; 1 by the `.ini`'s `ERROR` line, `Not enough stars.` to
  `TooFewStars`, `No star database found.` to `NoDatabase`, and any other or none to
  `NotSolved`, whose detail is `No solution found!` when there is no `ERROR` line; 2 to
  `TooFewStars`; 32 and 33 to `NoDatabase`; an `ERROR` line as the error's detail; and
  everything else to `Failed`, with the messages of US1-S4. The saved captures SHALL be
  read the same way (US1-S7).
- **FR-006**: The adapter SHALL wait for the program on the blocking pool in real time,
  kill and reap it after its limit (by default 120 s for a solve whose hint has a
  place, 600 s for one whose hint has none, and 30 s for a star measure) and answer
  `TimedOut`; and SHALL remove its call's work folder whatever the outcome (US1-S5).
- **FR-007**: `measure` on the ASTAP adapter SHALL run the program with `-analyse 30`
  on the prepared copy of the frame or region and read `HFD_MEDIAN=` and `STARS=` from
  its standard output, answering the diameter in imaging-frame pixels (US1-S6).
- **FR-008**: The simulated solver adapter SHALL answer from the simulator: the truth at
  the middle of the frame's exposure, moved to the centre of the frame's region, with
  its up taken there, and scaled by its binning, never mirrored; `TooFewStars` with
  fewer than 30 stars of the simulator's seeded field in the frame; `NotSolved` when
  the hint's place is farther from the answer than its radius, or its field height is
  more than 5% from the frame's, the accuracy ASTAP's page asks for; and a star measure
  from the simulator's point-spread constants and focus error (US2-S2). It reads no
  file.
- **FR-009**: `POST /api/imaging/solve` SHALL take no body, under the same-origin rule,
  and solve the latest imaging frame when it is `saved`, answering 200 with the imaging
  view at once. It SHALL answer 409 with the texts of US2-S3 when the console takes no
  imaging frames, when the camera is not `ready` (031's words), when it has no imaging
  solver, when a solve is running, and when there is no saved latest frame.
  `AppState::with_imaging_solver` SHALL give the console its solver, and
  `Imaging::solver()` SHALL answer it (US2-S3). The solve and its solve sidecar SHALL
  be made by one public function, `solving::solve_frame`, which takes a frame and a
  hint and touches neither the imaging view nor the relation, so that a later spec can
  solve a frame that is not the latest one.
- **FR-010**: The imaging view SHALL carry `solve` from the first accepted request:
  `frame_id`, `state` (`solving`, `solved` or `failed`), `requested_at`, `solver` and
  `hint`; once the solve has ended, solved or failed, `took_ms` and, when the solve
  sidecar was written, `file`; and, solved, `ra_deg`, `dec_deg`,
  `up_east_of_north_deg`, `arcsec_per_px`, `mirrored`, `cd_deg`, and `stars` and
  `warning` when there are any; or, failed, `reason`, `error` and `detail` when there
  is one (US2-S1, US2-S4). `GET /api/session` and the MCP tools that return the
  imaging view carry it too. It SHALL be absent before the first request and outside
  `ready` (owner decision 21).
- **FR-011**: Each solve that ends SHALL write `<frame id>.solve.json` beside the
  imaging frame's file, under a `.part` name and then renamed over any earlier one,
  with `format` `imaging-solve/1`, the frame's id and file name, the solver's name, the
  hint, `solved`, `took_ms`, and the solve's numbers or the failure's `reason`, `error`
  and `detail`. The imaging frame's file SHALL not change (US2-S1, US2-S4).
- **FR-012**: A failed solve SHALL show the reason and message of its `SolveError`
  unchanged, and SHALL not be tried again by the console (US2-S4, owner decision 19).
- **FR-013**: The `finder` binary SHALL take `--astap PROGRAM`, `--astap-db-name NAME`
  and `--astap-db FOLDER`; `--astap` SHALL require `--astap-db-name` and conflict with
  `--simulate`; under `--simulate` the console SHALL use the simulated solver adapter;
  with neither it has no imaging solver (US2-S6). `crates/finder/README.md` SHALL
  document the route, the flags, the sidecar, the reasons, ASTAP's licence and the
  database's credit line.
- **FR-014**: The hint for a solve SHALL be the first of these that exists: the
  session's latest pointing when its source is live and its time is no more than 30 s
  before the exposure's start or after its end (`pointing`); the centre of the last
  imaging frame solved since the console started, while the session's target is what
  it was when that frame was solved (`last_solve`); the session's target (`target`);
  or no position (`none`). The hint SHALL name its source. A failed solve SHALL not be
  asked again; after three misses in a row the next solve SHALL take the next source.
  A miss is a solve that ended `not_solved` or `timed_out`; with m misses since the
  last solved solve, the hint SHALL take, among the sources above that exist, the one
  `m / 3` places down (whole division), or `none` when there are not that many, and a
  solved solve SHALL return it to the first (US3-S1, US3-S4, US3-S8, owner decision
  23).
- **FR-015**: The hint's radius SHALL be 3° for `pointing`, `last_solve` and `target`,
  and 180° for `none` (US3-S1, US3-S4).
- **FR-016**: The hint's field height SHALL be the frame's rows times its binning times
  a scale, over 3600. The scale has up to three sources, in this order: the kept
  scale; `206264.806 × pixel size in mm / focal length in mm`, when a focal length is
  set; and none, which leaves the field height out. The console SHALL start from the
  first source that exists, move to the next one that exists, and from the last back
  to the first, each time three more solves have ended `not_solved` or `timed_out`
  with no solved solve between them, and start again from the first after a solved
  solve (US3-S2, US3-S3, US3-S7, owner decision 22).
- **FR-017**: Every solved imaging frame SHALL store its scale, per sensor pixel, with
  its frame id and time, in `relation.json` in the imaging folder, replacing the one
  kept, and the imaging view SHALL carry it as `relation.scale`. The file SHALL name
  the camera's model, serial, size and pixel size; a file made for another camera
  SHALL be ignored (US3-S2, US3-S5, US3-S7).
- **FR-018**: `AppState::with_focal_length_mm` and the flag `--focal-length-mm` SHALL
  set the focal length of FR-016, shown as `relation.focal_length_mm` (US3-S3,
  US3-S6). `Imaging::scale_arcsec_per_px()` SHALL answer the kept scale, or without one
  the focal length's, in arcseconds a sensor pixel, for the specs that turn pixels
  into arcseconds (US3-S2, US3-S3). `crates/finder/README.md` SHALL document the
  hint's sources, the turn to the next one after three misses, its radii and its field
  height.
- **FR-019**: The console SHALL keep a history of the finder camera's live solves (each
  one's mid-exposure time on the console's clock, its projection and its frame id) for
  the longer of 120 s and four imaging exposures, bounded by that span alone: a live
  solve that follows the last one kept by less than the span divided by 2000 SHALL
  not be kept, and only live solves older than the span SHALL be dropped. It SHALL
  tolerate reports the live broadcast dropped (US4-S1, US4-S2).
- **FR-020**: After a solve of a whole-sensor imaging frame, the console SHALL take an
  aim reading from the history's live solves between 10 s before the exposure and
  10 s after it, or the exposure's own length each way if that is longer. It SHALL
  project the imaging solve's centre through each of them, fit a straight line by
  least squares to those places against time, in each axis of the finder camera's
  frame, and take the line's value at the middle of the exposure as the aim point. It
  SHALL take none, and say why in `relation.last.outcome`: when no live solve is
  inside the exposure or fewer than three are in the span (`no_live_pointing`); when
  the centre falls outside the finder camera's frame (`off_frame`); and when the
  line's travel during the exposure exceeds 60″ on the sky by more than its standard
  error, or the live solves scatter about the line by more than 90″ (`moving`)
  (US4-S1, US4-S2, US4-S4, owner decision 11). It reads no axis position and computes
  no altitude or azimuth.
- **FR-021**: The imaging angle SHALL be the imaging solve's angle of up east of north,
  less the angle east of north of the finder frame's up axis seen at the imaging
  solve's centre, in 0 to 360; `Frame::up_east_of_north_at` SHALL give the latter
  (US4-S2, US4-S3).
- **FR-022**: An aim reading SHALL become the session's aim point through
  `Session::set_measured_aim`, persisted like a tapped one, while Track is off (FR-036
  says what happens while it is on). The session's `aim` SHALL
  then read `{x, y, "source":"measured"}`; a tapped aim point SHALL read `{x, y}` as
  before, and a tap or a reset SHALL clear the mark. `session.json` SHALL hold
  `aim_measured` only when true. This extends 013's FR-025 and FR-026 (owner approval
  1; US4-S1, US4-S5, US4-S6).
- **FR-023**: The imaging view's `relation` SHALL carry, while the aim point in force
  is a measured one (the session's, or one held back while Track is on, FR-036):
  `aim`, `angle_deg` and `mirrored`, when an aim reading is held, and `readings`, the
  count held; and always, once known: `last` (`frame_id`, `outcome`) and `field`
  (`width_arcsec`, `height_arcsec`: the sensor's size times the kept scale). When the
  aim point in force is not a measured one, `readings` SHALL be 0 and `aim` and
  `angle_deg` absent (US4-S1, US4-S5).
- **FR-024**: The console SHALL hold the last five aim readings and use their medians:
  of x, of y, and of the imaging angle taken around the circle; the mean of the middle
  two for an even count (US5-S1, US5-S2).
- **FR-025**: With three or more aim readings held, one more than 120″ on the sky from
  the median aim point, more than 1° from the median imaging angle, or of the other
  mirror sense SHALL be set aside (`set_aside`); a reading that is taken clears those
  set aside; the third set aside in a row SHALL replace the window with the three
  (`replaced`) (US5-S1, US5-S3).
- **FR-026**: The session's aim point SHALL be set from the medians when none is in
  force, when the one in force was tapped, or when it is more than 10″ on the sky from
  them; otherwise it SHALL be left as it is (US5-S1, US5-S2).
- **FR-027**: The aim readings held and those set aside SHALL be kept in
  `relation.json`, each with the size of the finder frame it was made in, and read at
  start. They SHALL be dropped when the camera differs, when an aim reading made in a
  finder frame of another size is taken, or when the session's aim point is not the
  measured one (US5-S1, US5-S4).
- **FR-028**: Once the session's aim point is the measured one, and while the kept
  scale is the field height's source (FR-016), a hint from a live pointing SHALL use a
  radius of the sensor's diagonal on the sky
  (the camera's whole width and height, whatever region the frame covers), never
  under 0.2° or over 3° (US5-S2). `crates/finder/README.md` SHALL describe how the aim
  point is measured, smoothed, held while Track is on and kept.
- **FR-029**: The Imaging page SHALL show a block headed Imaging solve after the
  imaging frame's, with the button `Solve imaging frame`, the solve's state in the
  words of US6-S2, and the relation's lines, the held aim point's among them, in the
  words of US6-S3 (US6-S1 to US6-S4).
- **FR-030**: The live chart SHALL draw the imaging field as an outline centred on the
  aim point, turned by the imaging angle, sized from `relation.field` and the finder
  camera's scale, with a mark on its up side, only while the aim point is a measured
  one (US6-S5, US6-S6).
- **FR-031**: Setup's Aim point section SHALL say `Measured from the imaging camera.
  Imaging angle {angle}°.` for a measured aim point and `Set by tapping a star.` for a
  tapped one, adding no `status` or `alert` (US6-S7).
- **FR-032**: The page's session SHALL keep `imaging` and `aim.source`; `DESIGN.md`
  SHALL describe the three changes and `.impeccable/design.json` SHALL list the Imaging
  field component; `crates/finder/web/dist` SHALL be the build of its source.
- **FR-033**: The MCP tool `solve_imaging_frame` SHALL take no arguments, do what `POST
  /api/imaging/solve` does, wait until the solve ends, and answer the imaging view; a
  refusal or a failed solve SHALL be a tool error with the route's or the solve's own
  words (US7-S1 to US7-S4).
- **FR-034**: `crates/finder/README.md` SHALL list the tool in `## MCP clients`.
- **FR-035**: From the story that keeps the history of live solves (US4), the
  `pointing` source of FR-014 SHALL be the history's live solve nearest the middle of
  the exposure, among those no more than 30 s before the exposure's start or after
  its end, carried through the aim point as a live pointing is (the finder frame's sky
  direction at the aim point, or at its centre without one); and the session's latest
  pointing, as FR-014 has it, only when the history holds none (US4-S4).
- **FR-036**: While Track is on (the session's `track.wanted` is true),
  `Session::set_measured_aim` SHALL hold the aim point back: the session's `aim` and
  `session.json` SHALL stay as they are, `Session::held_aim` SHALL answer it, the
  imaging view SHALL carry `relation.held` true, and aim readings SHALL be smoothed
  against it as the aim point in force. The first live solve the session records with
  Track off SHALL apply it, before that solve's pointing is taken, as FR-022 applies an
  aim reading. A tap or a reset SHALL drop it, and it SHALL not be stored. So the main
  telescope moves no further in the 5 s after an aim reading taken while Track is on
  than Track alone moves it (US4-S8, owner decision 25). No line of 015's tracker or of
  Track's routes changes.
- **FR-037**: `Imaging::note_solve` SHALL take an aim reading from an imaging solve made
  outside `Imaging::solve`, for a whole-sensor frame, exactly as FR-020 to FR-022 and
  FR-036 take one after `POST /api/imaging/solve`, and SHALL change neither the imaging
  view's `solve`, the kept scale nor the misses; for a frame of a region of the sensor
  it SHALL do nothing (US4-S7). It is for a later spec that solves frames itself (035:
  every accepted sub).

### Key Entities

- **Imaging solve**: where an imaging frame points: centre, angle of up east of north,
  scale, mirror sense and linear part.
- **Solve hint**: where the imaging solver is told to look: a source, a place when
  there is one, a radius and a field height when known.
- **Star measure**: the median half-flux diameter in pixels and the star count of an
  imaging frame or a region of one.
- **Solve sidecar**: `<frame id>.solve.json`, a solve's record beside its imaging
  frame.
- **Finder fix**: one live solve of the finder camera as the history keeps it: its
  mid-exposure time, its projection and its frame id.
- **Aim reading**: one reading of the aim point and imaging angle, from an imaging
  solve and the finder fixes around its exposure.
- **Relation**: what the console keeps between the two cameras: the scale, the aim
  readings held, and from them the aim point and imaging angle. A measured aim point
  that waits while Track is on is the session's, not the relation's, and is never
  stored (FR-036).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario has a committed test that asserts it, and the
  `lint` and `test` gates pass.
- **SC-002**: No existing test, golden, `scripts/ci/` file, `.github/` file,
  `CODEOWNERS` entry, `ergane.yaml`, `.specify/` file or `playwright.config.ts` is
  edited. The only edits to existing files under `crates/finder/tests` are the `mod`
  lines of owner approval 13.
- **SC-003**: No `Cargo.toml`, `Cargo.lock` or `deny.toml` changes, no `unsafe` is
  written, and nothing changes under `crates/simulator`, `crates/scope-node` or
  `crates/scope-link`.
- **SC-004**: On the simulator, a measured aim point is within 1.5 finder pixels of the
  true one and the imaging angle within 0.25° of the true one, after one aim reading
  and after five (US4-S1, US5-S2).
- **SC-005**: No file under `crates/finder/src/imaging/` names an axis count, an
  altitude, an azimuth or the `ephemeris` module (the mount-neutral rules).
- **SC-006**: On the simulator with Track on, the main telescope moves less than 60″ on
  the sky in the 5 s after an aim reading, and the measured aim point is in force
  within 5 s of Track off (US4-S8).

## Assumptions

- **Earlier features are in place.** 030-imaging-camera-link and
  031-imaging-frames-on-console have landed, with the interfaces Clarifications list, and so have 014's GoTo and 015's Track, which US4-S8
  turns on. Every `path:line` here was checked at 49bb210, the worktree's HEAD when the
  audit was applied, and re-anchored to `main` at e9687ce, where both have landed
  (plan.md § Prerequisites 2); every file of 030 and 031 is named without a line.
- **Out of scope:**
  - solving every sub of a stack, and dropping subs that fail (035);
  - the focus reading, focus region and focus aid, which use `measure` (033);
  - centring a target on the sensor, and any change to GoTo, Track or 015's tracker
    (036, 037): an aim point measured while Track is on waits instead (FR-036);
  - a hinted solver of this project's own, astrometry.net, or any second adapter;
  - a model of how the aim point shifts with altitude;
  - solving an imaging frame by id, or one from an earlier night;
  - a field on the page for the focal length (owner decision 6);
  - a route, a flag or a button that forgets the kept scale or the aim readings: the
    scale's sources are taken in turn (FR-016), and a tap or a reset empties the
    readings;
  - any change to the tap, its route or its words;
  - an aim-point tool for MCP clients.
- **Outside the factory:** installing ASTAP and a star database on the console machine;
  recording the saved captures (plan.md § Prerequisites 5); and the field checks:
  - ASTAP solving real subs at a rich field and a sparse one, with and without bright
    nebulosity, at binning 1 and 2, and how long a hinted solve takes;
  - a solve with no hint, timed, with and without `--focal-length-mm`;
  - the D50 database against D80 on the same subs, the frame's height being on the
    line where ASTAP's own choice changes;
  - that up on the page is up on the sky and a real solve is not mirrored;
  - the share of solves that take an aim reading while the telescope tracks, and the
    scatter of the finder camera's live solves about the fitted line, which is what
    the 60″ and 90″ of owner decision 11 are set against;
  - the measured aim point steady from one part of the sky to another;
  - the first aim reading taken while Track is on, watched at the telescope: the tube
    does not move toward it, and it is applied once Track is off;
  - any night check of the chart's outline.
- **The factory's gates and CI:** the gates are `lint` and `test`, copied in plan.md
  § Verification. CI's other jobs run only in the merge queue, so no scenario's Then
  clause rests on them.

## Work Graph

```yaml
US1:
  depends_on: []
  implements: [FR-001, FR-002, FR-003, FR-004, FR-005, FR-006, FR-007]
US2:
  depends_on: []
  depends_on_merged: [US1]
  implements: [FR-008, FR-009, FR-010, FR-011, FR-012, FR-013]
US3:
  depends_on: []
  depends_on_merged: [US2]
  implements: [FR-014, FR-015, FR-016, FR-017, FR-018]
US4:
  depends_on: []
  depends_on_merged: [US3]
  implements: [FR-019, FR-020, FR-021, FR-022, FR-023, FR-035, FR-036, FR-037]
US5:
  depends_on: []
  depends_on_merged: [US3, US4]
  implements: [FR-024, FR-025, FR-026, FR-027, FR-028]
US6:
  depends_on: []
  depends_on_merged: [US5]
  implements: [FR-029, FR-030, FR-031, FR-032]
US7:
  depends_on: []
  depends_on_merged: [US5, US6]
  implements: [FR-033, FR-034]
```

Every edge is a merge edge, and the stories run one after another. US2 implements the
port US1 defines and adds its module lines after US1's. US3 gives US2's solve its hint,
with the turn to the next source that US3's own count of misses drives, and creates
`relation.json`. US4 takes its aim reading from US2's solve and from `note_solve`,
reads US3's scale for the field, takes the `pointing` hint from its own history
(FR-035), holds a measured aim point while Track is on, and adds to the relation; it
is the only story that writes `crates/finder/tests/motion/main.rs`. US5 turns US4's single aim reading into a window; its edge to US3 is
for the radius it narrows and for the README section US3 last wrote, which US4 does
not touch. US6 shows what US2 to US5 put in the imaging view and the
session. US7 calls US2's action; it comes last, after US6, and its edge to US5 is for
the README section US5 last wrote, which US6 does not touch.
