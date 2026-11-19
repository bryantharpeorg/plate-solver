---
state: draft
depends_on_landed: ["030-imaging-camera-link", "031-imaging-frames-on-console", "032-imaging-solve-and-aim-point", "033-focus-reading-and-aid", "034-dark-and-flat-frames"]
# Drafted 2026-11-15 against `main` at 49bb210 (the drafting worktree's HEAD,
# 49bb210fe4860dd98ba24727074e7bd3cc0d7124) from
# docs/plans/2026-11-15-specs-030-037-imaging.md, by the grilling session of that day.
# A first drafter was cut off; the trio was finished the same day by a second, who
# brought it in line with the plan document's § Mount-neutral by design and its
# § Amendments after 030, 031, 032 and 034 were drafted and reviewed, and with the reviewed
# plans of 030, 031, 032 and 034.
# Reviewed the same day by a second agent, whose eighteen findings were applied to the
# three files (plan.md § Refinement Record, "What the review changed"), and revised the
# same night after the series audit of 030 to 037, whose six findings on this trio were
# applied (plan.md § Refinement Record, "What the audit changed"; spec.md
# § Clarifications, the third session).
# Citations checked against 49bb210fe4860dd98ba24727074e7bd3cc0d7124, the worktree's
# HEAD, on 2026-11-15 in that audit pass: every `path:line` in spec.md and plan.md was
# read there again, and none had moved.
# When drafted it was not probe-built: no story had been compiled or run. It builds on
# code that 030, 031, 032, 033 and 034 create, which did not exist yet: every interface
# of theirs named here is taken from that draft's
# plan.md as it stood in the drafting worktree (030, 031, 032 and 034 after their review,
# 033 as it was revised the same evening), or, for the four things the audit's findings
# on 031, 032 and 034 add, from those trios as they were being revised that night: the
# camera's lease (031 US9, as 031's plan.md § The camera lease gives it, in the shape
# specs/033-focus-reading-and-aid/plan.md § The camera lease writes down); 031's
# useColourHold (031 US7, from the finding A6); 032's Imaging::note_solve (032 US4,
# from the finding A3, as 032's plan.md gives it); and a set of darks or flats under
# the lease (034 A1, as 034's plan.md gives it). This trio is re-anchored against
# `main` after the five land and before it is flipped (plan.md § Prerequisites).
# Revised 2026-11-17 with the owner's answer to owner decision 13, a folder per run:
# a stack's subs are saved in `subs/` inside its own folder (§ Clarifications, the
# fourth session; plan.md § Refinement Record, "What the owner's answer changed").
# Probe-built 2026-11-18 and 19 (round 1): all ten stories were built from this trio on
# probe builds of 030 to 034, tests first, with both gates, and its 3 blocking and 38
# clarity defects were folded in on 2026-11-19 with the owner's answers of that day:
# US5-S2, S7 and S9 moved to US6 as US6-S7 to S9, US5's others were renumbered, and
# US8's README section moved to US10 (§ Clarifications, the fifth session; plan.md
# § Prerequisites 9 and § Refinement Record, "What probe round 1 changed").
# Re-anchored 2026-11-19 to `main` at 002dbef8 (002dbef8bda98d533a59911986f63416ff238b5e),
# where 030's eight stories, 031's nine, 032's seven, 033's nine and 034's ten have
# landed: every citation was compared line for line with 49bb210 and read again by its
# content, and the ones that moved are corrected, the glossary's among them; every name
# and behaviour taken from 030 to 034 was read in their code, and the few that differed
# in name, path or place are corrected, § A lease's folder now being written on 031's
# landed `frames::take`, `save` and `write`; no shared write's story changed; and the
# Constitution Check names the other stackers looked at and why each was turned down
# (plan.md § Prerequisites 1, 2, 5, 8, 13 and 14, § Constitution Check).
# It stays draft until the owner settles the owner decisions and flips it to `ready`,
# which gives the approvals listed under "Owner approvals" and dispatches it.
---

# Feature Specification: Live Stack

**Feature Branch**: `035-live-stack`

**Created**: 2026-11-15

**Status**: Draft

**Input**: The console keeps every imaging frame the scope node relays (specs 030 and 031),
knows where each one points (spec 032) and can calibrate it (spec 034). This spec builds the
picture. While the console tracks, it takes subs one after another, solves each with the
imaging solver, calibrates it, turns its mosaic into colour, places it on the first
sub's grid by the two imaging solves, and adds it to a stack with a coverage weight for
each pixel, leaving out pixels that stand far from what the stack already holds there,
such as a satellite's streak. A sub that does not solve, has too few stars or is clearly
softer than the others is dropped and counted with its reason, and the camera is never
kept waiting for the stack. The stack is shown as a colour picture with its background
made neutral and its brightness stretched, red at night on the Imaging page unless the
observer holds the control for colour, and is saved as a 32-bit FITS file and a PNG.
The page starts and stops it, and MCP clients get two tools. While it runs the stack
holds the imaging camera, so nothing else takes an exposure. It works before the
imaging run of spec 037 exists: the observer does a GoTo, turns Track on, and starts the
stack.

## Clarifications

### Session 2026-11-15 (the interview's decisions, from the plan document)

- Q: What does this trio read besides itself? → A: Nothing about the feature. `spec.md`
  holds every requirement and scenario, and `plan.md` holds every design decision, the
  interfaces, the shapes that existing tests pin and the alternatives rejected. The
  binding decisions come from `docs/plans/2026-11-15-specs-030-037-imaging.md`,
  `docs/adr/0002-imaging-cameras-through-indi-frames-relayed-console-stacks.md` and
  `GLOSSARY.md`; facts come from `docs/research/2026-11-15-imaging-camera.md`. A story
  that would change a decision escalates rather than re-deciding it.
- Q: Who builds the stack, and where? → A: The console, in its own code. The scope node
  takes each exposure and relays the imaging frame untouched; nothing here changes the
  scope node or the scope link. A sub is an exposure asked for with the purpose label
  `sub`, which 030 already carries.
- Q: How is a sub registered? → A: By its own imaging solve. The owner chose ASTAP as
  the imaging solver and it solves every sub; the stack places a sub by the full
  transform between the reference sub's imaging solve and its own, so a rotation, a
  change of scale, a shear and a shift are all carried. It never computes field rotation
  from the time, the site or where the mount points. A refinement on star positions is
  out of scope.
- Q: How accurate is that? → A: At the worked example's size a 0.2 px error at the
  corner of the picture allows 14″ in a solve's angle and 0.11″ in its centre. With N
  matched stars whose centroids scatter by s, one solve is good at the corner to
  0.06 px for s = 0.2 px and N = 50, to 0.13 px for N = 10, and to 0.15 and 0.35 px
  for s = 0.55 px; the difference of two independent solves is 1.4 times that, so
  0.08 to 0.5 px at the corner and less toward the centre. Catalogue and distortion
  errors are common to every sub of one field and cancel. How many stars ASTAP's last
  fit uses, and how well two of its solves of one field agree, is not established
  (`docs/research/2026-11-15-imaging-camera.md` § 8. Live stacking and § 13. Not
  established): it is a field check. In the tests the simulated solver adapter answers
  the simulator's truth, so the tolerance there is the arithmetic's own: a star's
  centroid in the stack within 0.02 px of its centroid in the reference sub (US3-S4;
  round 1 measured 0.002 px).
- Q: Why is rejection not a switch that starts off? → A: From 31° to 35° N the belt of
  geostationary satellites crosses a field centred on the Orion Nebula, and from 30° to
  40° N it passes within a degree. Those satellites cross a tracked field at the
  sidereal rate, 273 px in a 10 s sub at the worked example's scale, and leave roughly
  20 to 120 electrons a pixel against a few of noise
  (`docs/research/2026-11-15-imaging-camera.md` § 8. Live stacking). A plain mean keeps
  every one of them. Rejection is on unless a request turns it off.
- Q: In what order is a sub handled? → A: The master dark is subtracted, the master flat
  divided out and bad pixels repaired on the mosaic (034); then the mosaic is turned
  into colour, placed, tested and added; background, colour balance and stretch are
  for the display only and never touch what is saved
  (`docs/research/2026-11-15-imaging-camera.md` § 8. Live stacking and § 9. Calibration
  frames).
- Q: Who holds the imaging camera while a stack runs? → A: The stack, and nobody else.
  The plan document's amendment 1 gives the console's imaging state (031) one lease
  that names its holder: a single imaging frame, the focus aid, a set of darks or
  flats, a stack and an imaging run each take exposures, all but the single imaging
  frame under that lease (its § Interfaces shared across the specs), and a second
  taker is refused with the holder's name. A stack takes the lease as `the stack` when it starts, takes every sub
  under it, and releases it when it has ended; it adds no lock of its own and asks no
  other part whether it is busy. The lease is 031 US9's, in the shape 033's plan.md
  § The camera lease writes down for 031, name for name; plan.md § The camera's lease
  cites it and lists the six things this trio relies on, each confirmed at the
  re-anchoring (owner decision 21, plan.md § Prerequisites 8). A single imaging frame
  holds no lease (031 US9), so a frame in flight with nobody holding the lease is the
  one case the lease cannot answer: like 033's aid, a stack looks for an imaging frame
  in flight only while nobody holds the lease, and otherwise lets the lease answer,
  with the holder's name, even while the holder's own frame is exposing (FR-013,
  US6-S7).
- Q: What about colour? → A: Whether a UV/IR-cut filter sits in front of the camera is
  not known, and the camera's window passes infrared, so no fixed colour matrix is
  used. The display makes the background neutral in each channel by measurement and
  multiplies each channel by a gain the observer can set
  (`docs/research/2026-11-15-imaging-camera.md` § 10. Exposing the Orion Nebula, and
  colour).
- Q: What of the Orion Nebula's bright core? → A: At any sub length used for stacking
  the Trapezium's four stars have clipped cores in each sub, and no source gives the
  exposure at which the nebula itself clips. The stack is held in 32-bit reals, so a
  stack of unclipped subs never clips; only its display does, and the display's curve
  compresses highlights (`docs/research/2026-11-15-imaging-camera.md` § 10. Exposing
  the Orion Nebula, and colour). Choosing the exposure is the observer's, and later
  037's.
- Q: What must hold for another telescope later? → A: The plan document's
  § Mount-neutral by design binds this spec. No file under
  `crates/finder/src/imaging/` computes an altitude, an azimuth or a field rotation or
  reads an axis position; the stack reaches the mount not at all; nothing is sized to
  one focal length (thresholds are shares of the sub's size, deviations of the stack's
  own values, or ratios to a measured median, and the radius of the stack's own hint
  is the sub's own diagonal on the sky as its solve measured it); a hint's sources are
  tried in that section's order, the live pointing carried through the aim point
  first (owner decision 26); the learning set, the queue and every limit are counted
  in subs, never in seconds; a sub's place is whatever the two imaging
  solves say, so a set point that a later tracker moves between subs needs no change
  here; and the scenarios cover no rotation, a small rotation and a half turn (US2-S3,
  US3-S3, US3-S4, US6-S3).
- Q: Is anything keyed to the owner's camera? → A: No. Sizes, the Bayer pattern and the
  exposure come from the imaging frames. The owner's camera appears only as a worked
  example,
  and the simulated one stays 1236 × 822; no test renders the worked example's size.

### Session 2026-11-15 (review of `main` at 49bb210 and of the 030, 031, 032, 033 and 034 drafts)

- Q: What exists today? → A: Nothing of this feature. At 49bb210 the console has no
  imaging module; every file this spec extends under `crates/finder/src/imaging/` and
  `crates/simulator/src/imaging.rs` is created by 030, 031, 032, 033 or 034 and is named here
  as a plain path with the story that creates it. Seven things were read in the code:
  1. A sample at column c, row r has its centre at (c + 0.5, r + 0.5): the simulator
     renders at index + 0.5 (`crates/simulator/src/render.rs:57`) about a centre of
     (width/2, height/2) (`crates/math-core/src/lib.rs:156`), and 032's imaging solve
     measures from the same centre. Every placement here uses that convention.
  2. `finder` already has `image` with `png` and `jpeg` (`crates/finder/Cargo.toml:27`),
     `serde_json` (`crates/finder/Cargo.toml:32`) and the simulator
     (`crates/finder/Cargo.toml:14`). The stack needs no dependency and changes no
     `Cargo.toml`.
  3. The console's router has one `DELETE` today (`crates/finder/src/api.rs:555`) and
     its catch-all answers only `GET` (`crates/finder/src/api.rs:560`); this spec's
     three routes join 031's table in `crates/finder/src/imaging/routes.rs` (031 US1).
  4. A finder test is a module of a test binary; a file at the top level of
     `crates/finder/tests` fails the layout test
     (`crates/finder/tests/layout/main.rs:102`), and a module's name may hold only
     lower-case letters and `_` (`crates/finder/tests/layout/main.rs:155`). The new
     modules go after `mod session;` in `console` (`crates/finder/tests/console/main.rs:47`),
     after `mod simulate;` in `cli` (`crates/finder/tests/cli/main.rs:20`) and after
     `mod session_tools;` in `mcp` (`crates/finder/tests/mcp/main.rs:17`).
  5. An existing test calls every listed MCP tool with empty arguments on a console that
     takes no imaging frames (`crates/finder/tests/mcp/motion.rs:517`); the two new
     tools answer such a call at once (US10-S4).
  6. The night edition turns every `.photo` red by luminance
     (`crates/finder/web/src/index.css:118`), and 031 wrote the Red Torch Rule's one
     exception for "a picture from the imaging camera" (`DESIGN.md:198`). The stack's
     picture is such a picture; no rule of `DESIGN.md` changes here.
  7. The `test` gate ends a test after five minutes (`.config/nextest.toml:23`) on a
     test build at opt-level 1 (`Cargo.toml:37`). While a blocking task runs, tokio's
     paused clock does not advance, so a test in paused time never sees the stack fall
     behind unless its solver waits on tokio time (US6-S4).
- Q: Which of 030's and 031's interfaces does this spec build on? → A: From
  `specs/031-imaging-frames-on-console/plan.md` § Interfaces as reviewed, and for the two
  cameras named last from `specs/030-imaging-camera-link/plan.md`: `Imaging::ended`,
  `Imaging::abort` and `Imaging::view`, with `ImagingView`, `FrameView`, `RoiView` and
  `Refusal`, and the refusals and the imaging frame of `Imaging::take`, which a stack
  reaches
  through the lease (`crates/finder/src/imaging/mod.rs` and
  `crates/finder/src/imaging/frames.rs`, 031 US1 and US2); the saved file
  `frames/<UTC date>/<id>.fits`, read with `fits::decode`, and `fits::utc`
  (`crates/finder/src/imaging/fits.rs`, 031 US2 and US3); `preview::Rgb` and
  `preview::jpeg`
  (`crates/finder/src/imaging/preview.rs`, 031 US6); the route table and its
  extractors (`crates/finder/src/imaging/routes.rs`, 031 US1); the Imaging page, its
  one-second read of `GET /api/imaging` and its night control
  (`crates/finder/web/src/components/ImagingPage.tsx`, 031 US7); the scope node's
  stand-in camera, `SyntheticImagingCamera`, and the simulator's `SimImagingCamera`
  with its numbered steps for making an imaging frame (030 US1, US2). And two things
  the series audit's findings on 031 add, which 031's plan.md in this worktree now
  carries (its § The camera lease and § The page): the camera's lease of the plan
  document's amendment 1, `Imaging::lease`,
  `Imaging::holder`, `Lease::take` and `Lease::set` (031 US9), whose refusal reads
  `the imaging camera is held by {holder}`, and the hook that holds colour under an
  imaging picture at night, `useColourHold(night)` of
  `crates/finder/web/src/useColourHold.ts` (031 US7, from the audit's finding A6).
  Each was read again in their code on `main` at 002dbef8, now that both have landed,
  and holds as named here but for a few in shape or place, which are corrected
  (plan.md § Prerequisites 2).
- Q: Which of 032's and 034's? → A: Of 032, from
  `specs/032-imaging-solve-and-aim-point/plan.md` § Interfaces as reviewed:
  `ImagingSolver`, `SolveFrame`, `Region`, `SolveHint`, `HintSource`, `ImagingSolve`
  with `from_cd`, `sky_at`, `pixel_of` and `cd_deg`, `SolveError` (whose `NotSolved`
  and `TooFewStars` carry an optional `detail`) and `StarMeasure`
  (`crates/finder/src/imaging/solver.rs`, 032 US1); `solve_frame` with its `Ask` and
  `Ended`, which that plan writes for this spec to call on every sub, `Imaging::solver`
  and `Imaging::hint` (`crates/finder/src/imaging/solving.rs`, 032 US2 and US3);
  `SimulatedSolver` (032 US2); the view's `relation.angle_deg` and `relation.readings`
  and the session's `aim` (032 US4 and US5); and `Imaging::note_solve`, which takes an
  aim reading from a solve that 032's own task did not make and touches neither the
  page's `solve` nor the kept scale (032 A3, the series audit's finding on 032). Of 034,
  from
  `specs/034-dark-and-flat-frames/plan.md` § Interfaces as reviewed: `Mosaic` and
  `percentile` (`crates/finder/src/imaging/calibration.rs`, 034 US2);
  `Library::calibrate`, `FrameKey::from_image`, `Applied` and `Selection`
  (`crates/finder/src/imaging/calibration/library.rs`, 034 US3); `Calibration::library`
  and `AppState::calibration` (034 US4); `Imaging::root` (034 US4); and the simulator's
  light path, into which this spec's two additions fit (034 US1). Each was read again
  in their code on `main` at 002dbef8, now that both have landed, and holds as named
  here (plan.md § Prerequisites 2, 13 and 14).
- Q: What of 033, the focus reading and the focus aid? → A: 033 lands before this spec
  (034 names it, and so does this spec now). Its trio is in the worktree and was
  revised the evening this one was finished; it is read here as it then stood, and
  again in its code on `main` at 002dbef8 since it landed (plan.md § Prerequisites 5).
  Its plan
  writes `focus::read` for this spec to call on each sub
  (`crates/finder/src/imaging/focus.rs`, 033 US1), and its focus view names the focus
  region the console keeps (`crates/finder/src/imaging/focus/run.rs`, 033 US5). The
  gate of US6 uses both, so that a sub's focus reading is the glossary's: the stars'
  median size in the focus region, in arcseconds (`GLOSSARY.md:209`, `GLOSSARY.md:213`).
  The gate judges a sub against the other subs of its own stack, not against 033's
  focus record: the record's verdict is the run's focus check, which 037 owns. A stack
  cannot start while the focus aid runs, because the aid then holds the camera's lease,
  and the refusal is the lease's, naming `the focus aid` (owner decision 8, plan.md
  § Prerequisites 5).
- Q: Where does this trio depart from the plan document? → A: Nowhere, with the
  answers now written in. It departed in three places, each an owner decision, and the
  plan document's § Interfaces shared across the specs now gives the answer written
  here for each: it names `stack.rs` with `stack/service.rs` and `stack/display.rs`,
  the two child modules that keep each story's diff under 64 KiB (owner decision 19);
  it gives `runs/<UTC start>-<target>/` `stack.json`, `stack.fits`, `stack.png` and
  `subs/`, which holds each sub with its solve sidecar, listed in `stack.json` by its
  name there, while every other imaging frame stays under `frames/`, where 031 saves
  it (owner decision 13, settled by the owner on 2026-11-17); and it gives 035 three
  routes for a stack, the options and the display's gains riding on the start route's
  body (owner decision 18). Another answer to any of the three would depart from it
  again (plan.md § Prerequisites 10).
- Q: Where does it stay with the plan document, though a reader might think otherwise?
  → A: In two places. The order of a hint's sources is that document's § Mount-neutral
  by design, 4, unchanged: the live pointing carried through the aim point; then the
  last imaging solve, which during a stack is the stack's own last solved sub; then
  the session's target; then none (owner decision 26, FR-016, FR-025). The first draft
  of this trio put the stack's own hint ahead of the live pointing; that order is now
  decision 26's other answer, and choosing it would be a departure. And the plan
  document's Stacker row says that outliers are rejected, and its section on 035 that
  subs which measure poorly are dropped: both hold for every stack that is started
  without saying otherwise. `reject: false` and `focus_gate: false` exist only for the field
  checks that compare a stack with and without each (owner decisions 6 and 8); the
  page sends neither.
- Q: Which numbers come from a source, and which are design values? → A: The research
  note gives: 5 to 10 learning subs before one stacker starts to clip; a centre that
  may move by half the short side in another; a per-pixel count against darkened
  edges; a fitted linear gradient and an `asinh(a·x)/asinh(a)` curve in a third; dropped
  subs counted apart from stacked ones (`docs/research/2026-11-15-imaging-camera.md`
  § 8. Live stacking); and the plan document's 30% for a clearly soft reading. Every
  other number is a design value of this spec, named as a constant in plan.md
  § Interfaces. Those that are choices are listed for the owner: the half pixel the
  placing map may stray at a corner in decision 3, the rejection's numbers in
  decision 6, the grid a background level is read on in 7, the gate's in 8,
  the limits on a sub's place in 9, the queue's length in 10, the three failed
  exposures in 11, the display's numbers and the stretch's bounds in 16, the gains'
  bounds in 17, the MCP picture's width in 22 and the three misses in 26. The rest
  are numerical guards and not choices, and are given in plan.md alone: the edge
  slack of a millionth of a pixel, the two floors under a determinant, and the
  settings test's 0.001 and 1 ms. None was measured on a camera.
- Q: Which other drafts write the same files? → A: 030, 031, 032, 033 and 034 land before
  this spec and are in its `depends_on_landed`; 036 and 037 land after it and name it
  in theirs. Both of those were drafted before this trio existed and list what they
  assume of it; they are re-anchored to it, not the reverse. plan.md § Shapes that
  existing tests pin names the places in the stack's loop that 036 adds to, and the
  rule 036 must follow there: the exposure of a stack is set once, before its first
  sub, through the stack's own lease, because every other way to change a setting is
  refused while the stack holds the camera. 036's plan now does exactly that, with the
  taker's `Lease::set` (plan.md § Prerequisites 10). 037's run id, and the label it
  hands `Stacker::start` for the stack's folder, both come from this trio's `label`, so
  the rule for a target's name in a folder is written once (FR-035). The shared files are
  `crates/finder/src/imaging/mod.rs`, `crates/finder/src/imaging/frames.rs` (since owner
  decision 13 was settled), `crates/finder/src/imaging/routes.rs`,
  `crates/finder/src/imaging/fits.rs`, `crates/finder/src/api.rs`,
  `crates/finder/src/mcp/tools.rs`, `crates/finder/README.md`, `GLOSSARY.md`, the
  Imaging page and the simulator's imaging camera (plan.md § Prerequisites 2).

### Session 2026-11-15 (the series audit of 030 to 037)

An auditor compared the seven trios the series then had and gave six findings on this
one. All six were
applied (plan.md § Refinement Record, "What the audit changed"). "031 US9", "031 A6",
"032 A3", "034 A1" and "037 A2" name what the same audit's findings add to those trios:
031's US9 builds the lease, its A6 makes the colour hold one hook in 031's US7, 032's
A3 puts `Imaging::note_solve` in 032's US4, 034's A1 has a set hold the lease, and
037's A2 has 037 call this trio's `label`. Those trios were being revised the same night;
what this trio uses of them was read there as far as they then carried it, and is
confirmed at the re-anchoring (plan.md § Prerequisites 8, 13 and 14).

- Q: A stack asked for while the focus aid or a set of darks holds the camera: which
  words does it hear? → A: The holder's name, from the lease. Those takers have a frame
  exposing almost all the time they hold the camera, so a start that looked for a frame
  in flight before it asked for the lease heard `an exposure is in progress` instead,
  against the plan document's amendment 1 and this trio's own edge case. `start` now
  looks for a frame in flight only while `Imaging::holder()` is `None`, as 033's
  `focus_start` does in its check 4; while somebody holds the lease that look is passed
  over and the lease answers (FR-013, US6-S7).
- Q: Does a sub's imaging solve measure the aim point? → A: Yes, for a stacked sub. The
  plan document's Two cameras row has the console measure the aim point whenever an
  imaging solve and a live pointing share a moment, and a stack is most of a night's
  imaging solves. A stale aim point would also put the target off the sensor's centre
  once the subs stop and the tracker's own corrections, carried through the aim point,
  steer again. After a sub is
  counted stacked, the stack hands its solve to 032's `Imaging::note_solve` (032 A3,
  written into 032's US4), which takes an aim reading when a live pointing shares the
  sub's moment. While Track is on, 032 holds a newly measured aim point back until
  Track is off, so during the stack the tracker never chases it and, since 038, a step
  of more than 5′ is never read as a push by hand that marks the pointing model's slip;
  the readings the subs gave are what it then applies. A sub's solve still changes
  neither the page's `solve` nor the kept scale, so the radius of 032's hint stays
  what it was (owner decision 20, FR-015, US6-S9).
- Q: Where is the rule for a target's name in a folder written? → A: Once, as
  `stack::service::label` in this trio: every character that is not an ASCII letter or
  digit made `-`. `AppState::target_label` calls it, and so does 037's `run::label`
  (037 A2), so a run's id and its stack's folder cannot drift apart (FR-035, US8-S3).
- Q: Who holds colour under the stack's picture at night? → A: 031's own hook,
  `useColourHold` (031 A6), which 031's US7 writes as
  `crates/finder/web/src/useColourHold.ts` and uses for its own picture. The Stack
  block writes no timer or listener of its own (FR-042, US9-S3).
- Q: Can the linear placing map be trusted on a wide field? → A: Not without a check.
  Its error against the exact mapping between the two tangent planes grows as the
  shift between the two centres times the square of the field's half-size: about
  0.1 px at the worked example's size for a sub at the off-field limit, but 0.74 px at
  the corners of a 3° × 2° field of 3600 × 2400 pixels for a sub 0.75° away, and 1.17 px
  for one 1° away on the diagonal, both of which `judge` allows. So `between` also maps the reference grid's four corners through the sky and
  answers `None` when any lies more than 0.5 px from the map; the worker counts that
  sub `off_field` (owner decision 3, FR-005, US2-S7).

### Session 2026-11-17 (the owner's answers)

- Q: Where are a stack's subs kept? → A: In a folder per run: `subs/` inside the
  stack's own folder, `runs/<id>/subs/<frame id, six digits>.fits`, each with its solve
  sidecar beside it, and not under `frames/<UTC date>/` with every other imaging frame
  (owner decision 13, settled by the owner). `stack.json` lists each sub by its name
  in the stack's folder, `subs/<frame id>.fits`, dropped subs too, so the folder stands
  on its own when another program opens it. US5 now makes the folder and its `subs/`
  at the start, with `-2` on a clash, and has 031's lease save the stack's subs there
  through `Lease::save_in`, the one edit to 031's code this answer needs; US8 writes
  `stack.json` into that folder. Every other imaging frame, and 031's id rule, are as
  before. What changed: FR-013, FR-014, FR-031 and FR-035; US5-S1, US6-S7, US5-S2,
  US5-S6, US8-S1, US8-S3 and US8-S7; owner approvals 3 and 5; the Edge Cases; the Work
  Graph, in which US5 now follows US2 by a direct edge and US8 no longer names US2;
  plan.md § Storage, § The camera's lease, § A lease's folder, § The stack on the
  console, § Shapes that existing tests pin, § Decisions, § Prerequisites 3 and 10,
  § Plan traps 21, 26 and 35, § Risks and § Refinement Record; tasks T018, T021, T022,
  T024, T036 and T039. 031's plan, 037's trio and the plan document were brought to it.

### Session 2026-11-19 (round 1 probes)

Probe round 1 built all ten stories from this trio on 2026-11-18 and 19, on probe builds
of 030 to 034, tests first and with both gates, in four agents: US1 to US3, US4 to US6,
US7 and US8, and US9 and US10. Every story built as written, and every one but US9
passed its full gates. The round found 3 blocking and 38 clarity defects,
D1 to D41, and all were folded into the three files on 2026-11-19 but for the points
the last answer below lists (plan.md § Prerequisites 9 says what the round settled, and
§ Refinement Record, "What probe round 1 changed", what it changed). Every case the
round proposed where a mutation survived a story's committed tests is now in its
scenario, in the probe's words; each was run on the probe's build, passed on the code
and failed under the mutation. Where two fixes touched one sentence they were merged.

The scenarios of User Story 5 after the owner's first answer below. Each moved or
renumbered scenario says in its text what it was; every reference in the trio uses the
new number.

| Was | Now | Module |
|---|---|---|
| US5-S1 | US5-S1 | `stack_loop` |
| US5-S2 | US6-S7 | `stack_gate` |
| US5-S3 | US5-S2 | `stack_loop` |
| US5-S4 | US5-S3 | `stack_loop` |
| US5-S5 | US5-S4 | `stack_loop` |
| US5-S6 | US5-S5 | `stack_loop` |
| US5-S7 | US6-S8 | `stack_gate` |
| US5-S8 | US5-S6 | `stack_loop` |
| US5-S9 | US6-S9 | `stack_gate` |

- Q: User Story 5 measured 73,780 B, over 64 KiB, and the cut plan.md § Risks named
  left 67,826 B (D9). How is it cut? → A: As D9 says, by the owner's answer of
  2026-11-19: three of its scenarios move to User Story 6, in
  `crates/finder/tests/console/stack_gate.rs`, which copies the sim stacking rig and
  the private helpers they need. The call of `note_solve`, the folder and
  `Lease::save_in` stay in User Story 5, whose code the moved scenarios still test.
  The refusals' rigs go with US6-S7: its consoles with no imaging solver, with a node
  with no imaging camera, and without `with_imaging` are built in `stack_gate` from its
  copies of `console` and `stand_in_node`, and the helper `refusal` is written there
  alone (plan.md § Test rigs). Measured on the probe's trees: User Story 5 about
  61,300 B, and User Story 6 about 50,000 B, estimated and not built. With the round's
  cases User Story 5 comes to about 67,200 B, over 65,536 B again (plan.md § Risks).
  036 cites US5-S6 where it cited US5-S8; 037 cites no renumbered scenario.
- Q: User Story 8 measured 65,294 B, 242 B under the budget (D26). Is it cut? → A: Yes,
  by the owner's answer of the same day: the README's `## Live stack` moves to User
  Story 10, which already writes that file (FR-039; plan.md § `crates/finder/README.md`;
  tasks T041 and T050). So cut, User Story 8 measured 57,483 B.
- Q: Are User Story 9 or 10 cut? → A: No, by the owner's answer: User Story 9 measured
  33,513 B, and User Story 10 18,134 B, or 25,945 B with `## Live stack`.
- Q: 034's trio pins its Calibration block as the Imaging page's last region, and
  User Story 9 puts the Stack block after it, so 034's test fails (D33). Who fixes it?
  → A: 034's trio, before 034 is flipped: there the block comes after 033's Focus
  block, not last. That edit waits on the owner, with 034's flip, and is not made
  here. This trio states only what its own tests may rely on: 034's test places its
  region after `Imaging frame` and `Focus`, and nothing of 031 to 034 asserts that a
  region is the page's last (plan.md § Shapes that existing tests pin, **The Imaging
  page without a stack**); the re-anchoring checks that no landed test reads the
  page's last region (plan.md § Prerequisites 2).
- Q: Where a report offered two fixes, which was taken? → A: For `took_ms`, which leaves
  out the picture the worker makes after a sub (D29): the definition is kept, and the
  README says the picture is not in it, so FR-017 and owner decision 25 stand. For
  `get_stack`'s picture (D39): it is made on tokio's blocking pool, as plan.md's
  Constraints and plan trap 14 have every pixel pass. For US3-S4 (D6): its tolerance
  is 0.02 px, so it fails a quarter-pixel error in the convention on its own (the
  first session's answer on accuracy, SC-003). D16's stop against a 10 000 ms
  exposure is a second case of US5-S4, beside the 1000 ms one, and the low proposal of
  D38, files with `fits` and `json` alone, is in US9-S4.
- Q: What did the round measure that the trio now relies on? → A: On three runs each,
  with the same numbers each time: each star of US3-S4 lands within 0.002 px of its
  place in the first sub; noise alone rejects about 8% of the pixels at US4-S4's ninth
  sub and 5% of good pixels a sub on the sim stacking rig, falling as subs are added;
  `took_ms` is about 8% of a 1 s exposure at the simulated size, and the picture the
  worker then makes about 22% more; with a solver ten times slower than the camera,
  `taken` reaches 30 at 30.1 s, 23 subs are `behind` and exactly 2 are stacked; and the
  aim point the stacked subs measure lies within 1.5 px of the simulator's. Memory
  needs the worker to let a sub's file bytes go once decoded (about 65 MB more at the
  worked example's size otherwise), and the full-size picture at the end costs about
  400 MB beyond the stack (plan.md § Risks).
- Q: Where do 031 to 034 differ from what this trio says? → A: 031's lease matches on
  all six points. Its refusal for a state is private to `imaging` and its `Refusal`
  has no 500 constructor, so the stack maps the words and builds the 500 itself
  (plan.md § The stack on the console, **`start`**). On landed 031, `frames::take`
  takes a `held` closure, `save` builds the frame's path, `write` makes the parent
  folder and the lease's `take` is async: § A lease's folder's three edits are made on
  those shapes at the re-anchoring. Landed 031's hook has `onPointerDown: () => void`
  and refuses a hold by day, and plan.md now quotes it so; its `fits::card` answers
  80 bytes, not a text. Landed 032's `Imaging::solve` answers the whole view, and its
  `note_solve`, `hint` and `solve_frame` are as the trio has them. 033's `focus::read`
  is as the trio has it. 034's interfaces built as its folded trio has them; on `main`
  the README counts the imaging tools in other words than 034's draft, so User Story
  10 raises the count in the words `main` has.
- Q: What was not folded? → A: Three things, each for a stated reason. The moved line
  of a citation of the simulator's true aim point (D20) is the re-anchoring's, which
  corrects every line number. Two rules of User Story 5 that no rig can reach (D16): a
  refused take ending the stack as `camera` needs a node link that drops mid-stack
  (its words were proposed and not run), and the taker's second look at the stopping
  mark cannot be reached on the tests' one-thread runtime (no case was proposed).
  Three rules of User Story 8
  (D32): the camera's `serial` (the stand-in has none), `stack` in the view whatever
  the state (needs a link that drops), and `stack.png` at the stack's full size (needs
  a camera wider than 1920).

### Session 2026-11-19 (the re-anchoring onto `main`)

- Q: What does the stack do when the answer to its take carries no frame? → A: It ends,
  with `ended_reason` `camera` and 031's words for the camera's state as its `error`,
  the words `start` refuses with when the camera is not `ready`. On `main` the view that
  `Lease::take` answers is built once the lock is released, and it carries `frame` only
  while the camera is `ready`, so a link that drops between the take and its answer
  leaves no frame id. The owner chose this ending on 2026-11-19, as for 034's set's frame
  and 033's focus aid. The alternative, waiting for the camera and taking again, would
  need a scenario of its own. No scenario reaches the case (plan.md § The stack on the
  console, **The taker**).

## Owner decisions

Each is a design or taste call the plan document leaves open, or, where its heading says
so, a departure from the plan document. Decision 13 is settled, and says so; the owner
settles each other before the flip, and a different answer is written into the trio and
re-validated, never left to a story.

1. **Ten stories, where the drafting brief suggested seven.** Recommended: ten, so that
   each stays under 64 KiB: the simulator's additions are a story of their own, the loop on
   the console and its gate are two, and the page and the MCP tools are two. Instead:
   seven, and a story found too large mid-epic then needs a relaunch.
2. **Colour on the delivered grid.** Recommended: bilinear, one colour pixel for each
   sample, so the stack has the sub's own size. Instead: one colour pixel for each
   2 × 2 cell, a quarter of the memory and time, at half the resolution.
3. **The placing transform.** Recommended: a linear map and a shift, read from the two
   imaging solves at five points of the reference grid; it carries rotation, scale,
   shear and a mirror, and differs from the exact mapping by about 0.1 px at most, at
   the worked example's size, while a sub's centre stays within half the short side of
   the reference's (inferred). Because that error grows as the shift times the square
   of the field's half-size, the map is also checked at the reference grid's four
   corners against the two solves, and a sub whose map strays more than 0.5 px there is
   dropped as `off_field`: never at the worked example's size, but on a field of a few
   degrees for a sub far from the reference (inferred; spec US2-S7). Instead: the exact
   mapping worked for every pixel, which needs no check and costs a division a pixel;
   another slack than 0.5 px; or shift and angle only, which the research note shows
   leaves up to 0.7 px in an hour.
4. **The canvas.** Recommended: the reference sub's grid. Pixels that later subs do not
   reach keep fewer subs and more noise, never a darker value. Instead: a canvas padded
   to the sub's diagonal, 2.2 times the memory, which loses nothing.
5. **Resampling.** Recommended: bilinear. Instead: bicubic at 1.7 times the cost, or
   Lanczos at about 8 times.
6. **Rejection.** Recommended: against the stack's own running mean and deviation at
   each pixel, after 8 subs there, at 3 deviations, with a floor of one count, the
   whole pixel left out when any channel fails. A streak that falls inside the first 8
   subs is kept and fades as the stack grows. It is on for every stack, as the plan
   document's Stacker row has it; a request may say `reject: false`, which is there
   only for the field check that compares the two, and which the page never sends.
   Instead: other numbers; each channel by itself; no way to turn it off; or the median
   of a window of five placed subs, which costs about as much memory again.
7. **Background match.** Recommended: before the test, each sub is shifted, channel by
   channel, so that its median equals the reference sub's, each median read on a grid
   of about 512 pixels a side (every kth pixel of every kth row). Instead: none; a
   scale as well, for changing transparency; a finer or a coarser grid.
8. **The focus gate.** Recommended: each solved sub's focus reading, taken as 033 takes
   one (`focus::read`) in the focus region the console keeps, or over the whole sub
   when it keeps none or the sub does not hold it; a sub more than 30% above the median
   of the stacked subs' readings is dropped, once 5 are known, and so is one 033 calls
   far out; `focus_gate: false` turns the measure off. Instead: the whole sub always;
   a judgement against 033's focus record; another share; no gate.
9. **A sub off the field, or solved differently.** Recommended: dropped when its centre
   lies more than half the reference's shorter side from the reference's centre, or
   when the placing map's area scale is outside 1 ± 0.02, a mirror included. Instead:
   other limits; a new stack when the field moves.
10. **The queue.** Recommended: four subs wait for the stack at most; a fifth pushes the
    oldest out, counted as `behind`. Instead: another length; dropping the newest.
11. **How a stack ends.** Recommended: when it is stopped, when an exposure is aborted,
    or when the camera fails (three subs in a row that do not arrive, or a refused
    exposure). It has no limit of subs or time. Instead: a limit; ending after many
    drops in a row. 037 owns the run's other endings.
12. **Track is not asked for.** Recommended: the stack starts whatever the mount does,
    so a mount that tracks by itself works later, and a sub that drifts off the field is
    dropped and counted. Instead: refuse to start unless Track is on.
13. **Where a stack and its subs are kept** (the recommended answer was a departure
    from the plan document; the settled one is not, and the plan document's
    § Interfaces shared across the specs gives it). **Settled by the owner on
    2026-11-17: a folder per run.** `runs/<UTC start>-<target>/` holds `stack.fits`,
    `stack.png`, `stack.json` and `subs/`. Every sub the stack takes is saved in
    `subs/` as `<frame id, six digits>.fits`, by 031's path through the stack's lease,
    with its solve sidecar beside it, and `stack.json` lists it by its name there,
    `subs/<frame id>.fits`, a dropped sub as well as a stacked one. 035 adds the way to
    do it, `Lease::save_in`, an edit to where 031 saves an imaging frame taken under a
    lease that names a folder; every other imaging frame is saved where 031 saves it,
    under `frames/<UTC date>/`, and 031's id rule is unchanged (§ Edge Cases, **Frame
    ids after a restart**). A folder per run is what another program opens a night's
    work from. Instead (the answer recommended before): each sub stays where 031 saves
    every imaging frame, with its solve sidecar, and `stack.json` lists it by its file;
    nothing in 031's path changes, and `run.json` and `subs/` stay 037's to settle.
14. **Saved when the stack ends.** Recommended: once, at the end; the subs on disk are
    the record if the console dies first. Instead: also after every so many subs.
15. **What the files hold.** Recommended: `stack.fits` is three planes of 32-bit reals,
    the mean, with the reference solve as standard celestial cards so other programs
    can place it; `stack.png` is the display picture at the stack's full size, 8 bits a
    channel. Instead: no celestial cards; a 16-bit PNG; a fourth plane of weights.
16. **The display.** Recommended: a plane fitted through the darker half of an 8 × 8
    grid of tiles is taken from each channel; white is the 99.9th percentile above the
    background, or 30 times the noise where that is more; the curve is `asinh` of
    strength 10, which the observer can change from 1 to 1000; the page's picture is at
    most 1920 wide, is made from a reduction of the stack's sums and weights and never
    from a mean of its full size, and is remade when the queue is empty or after every
    eighth stacked sub. Its look is for the design review, not a gate. Instead: other
    choices.
17. **Colour gains.** Recommended: 1.0 for each channel until the observer sets them,
    each from 0.1 to 10. Instead: gains measured from the stack's star colours; other
    bounds.
18. **Options ride on the start route** (a departure). Recommended:
    `POST /api/imaging/stack` takes `reject`, `focus_gate`, `gains` and `stretch`, and
    on a running stack changes `gains` and `stretch` alone. Instead: a fourth route.
19. **Two child modules** (a departure). Recommended: `stack.rs` holds the arithmetic,
    `stack/service.rs` the console's state and `stack/display.rs` the display. Instead:
    one file, as the plan document names it.
20. **What a sub's solve feeds besides the stack.** Recommended: every sub's solve
    writes its solve sidecar, and a stacked sub's solve is also handed to 032's
    `Imaging::note_solve`, which takes an aim reading from it when a live pointing
    shares its moment, as the plan document's Two cameras row asks of every imaging
    solve; 032 applies the aim point at once, or, while Track is on, once Track is off.
    It changes neither the page's `solve` nor the kept scale, so the radius of 032's
    hint is what it was before the stack. Instead: a sub's solve feeds only the
    stack, and the aim point stays as it was for the whole of a stack (the first
    draft's answer, under which the target leaves the sensor's centre when the subs
    stop and the tracker's own corrections steer again); or every solved sub, stacked
    or not, refreshes the aim point and the kept scale as a solve from the page does.
21. **Who holds the camera** (the plan document's amendment 1; 034's owner decision 23
    asked for it). Recommended: the stack takes 031's lease (031 US9) as `the stack`
    for its whole length, from an accepted start until it has ended, and is refused,
    with the holder's name, while anything else has it, even while that holder's own
    frame is exposing; it looks for a frame in flight only while nobody holds the
    lease. Instead: ask `Calibration::busy` and `Imaging::focus_running` at the start
    and wait out an imaging frame another party slips in, as the first draft of this
    trio did, which the amendment forbids as a lock of this spec's own.
22. **The MCP picture.** Recommended: the display shrunk to at most 1024 wide.
    Instead: the page's.
23. **A browser check of the stack at night.** Recommended: one Playwright file in the
    simulate project, which needs the edit of `playwright.config.ts` that owner
    approval 9 names. Instead: component tests only.
24. **The simulator's scene is for tests.** Recommended: no route sets a streak or a
    glow under `finder --simulate`. Instead: a simulate-only route.
25. **One thread, and a measured time.** Recommended: the pixel pass runs on one
    blocking thread, and the stack reports the real time each sub took as `took_ms`,
    for the bring-up probe to read. Instead: row bands on scoped threads now.
26. **A sub's hint.** Recommended: the plan document's order of sources, with the
    stack standing in for the second. Every sub is first given 032's own hint, and when
    that comes from a live pointing it is used as it is. Otherwise, once the stack has
    a reference sub, the hint is the stack's own: the last solved sub's centre, a
    radius of the sub's own diagonal on the sky and the field height from that solve's
    scale, so no number of degrees is written down. After three solves in a row that
    miss, the stack's own hint is set aside and 032's is used until a sub solves.
    Instead: the stack's own hint ahead of the live pointing, as the first draft had
    it (a departure from the plan document's § Mount-neutral by design, 4: narrower
    while no aim point is measured, but blind to a mount that has moved); 032's hint
    for every sub; another count of misses.

## Owner approvals

Flipping this spec to `ready` approves the settled answer to every owner decision above,
and the following. Each is named in the FR or success criterion that makes it.

1. **Three routes and one part of the imaging view**: `POST /api/imaging/stack`,
   `DELETE /api/imaging/stack` and `GET /api/imaging/stack/preview.jpg` (FR-031 to
   FR-033), the request keys `reject`, `focus_gate`, `gains` and `stretch`, and the
   `stack` object of `GET /api/imaging` and of `imaging` in `GET /api/session`, which
   the MCP tools that return the session carry too (FR-034, FR-049).
2. **Two MCP tools**: `set_stacking` and `get_stack` (FR-047, FR-048).
3. **Three new source files**, `crates/finder/src/imaging/stack.rs`,
   `crates/finder/src/imaging/stack/service.rs` and
   `crates/finder/src/imaging/stack/display.rs` (owner decision 19), and these edits to
   files 031 and 034 create: `ImagingView` gains `stack` and `Imaging` gains
   `attach_stack`; `AppState::with_imaging` builds the stack's state and `AppState`
   gains `stack()` and `target_label()`; `fits.rs` gains `RealCube` and `encode_cube`;
   031's `Lease` (031 US9) gains `save_in`, and 031's save writes an imaging frame taken
   under a lease that names a folder into that folder, every other one where it did
   (FR-014, owner decision 13); `ImagingPage.tsx` renders the block (FR-012, FR-034,
   FR-036, FR-044). Among the new
   files' public names, `stack::service::label` is the one that another spec calls:
   037's `run::label` is that one line (FR-035).
4. **The simulator's new public names**: `SatelliteStreak`, `ExtendedSource` and
   `Scene`, exported from `crates/simulator`, and `Simulator::scene` and `set_scene`
   (FR-001).
5. **A stack's folder and files**: `runs/<id>/stack.fits` with the cards of FR-036,
   `runs/<id>/stack.png`, `runs/<id>/stack.json` (format `finder-stack/1`) and
   `runs/<id>/subs/`, which holds every sub the stack takes as
   `<frame id, six digits>.fits` with its solve sidecar, under the imaging folder
   (FR-013, FR-014, FR-035 to FR-037; owner decisions 13 and 15).
6. **One new `GLOSSARY.md` term** (FR-020), with these words:

   > **Reference sub**:
   > The first sub a stack accepts. Every later sub is rotated and shifted onto its grid by the two subs' imaging solves.
   > _Avoid_: reference frame, key frame, master

7. **`DESIGN.md`: one component entry, Stack block**, after the Calibration block entry
   that 034 adds, with the words of plan.md § `DESIGN.md`, and its entry in
   `.impeccable/design.json` (FR-045). No rule changes: the stack's picture is a
   picture from the imaging camera with the Hold to see colour control under it, which
   031's exception already covers.
8. **Document sections**: `## Live stack` in `crates/finder/README.md`, three rows of
   its API table and two lines of its MCP tool list (FR-039, FR-049), and three lines
   of the route list in `crates/finder/src/lib.rs`.
9. **Edits to existing test and gate files** (SC-002), and no others:
   - one `mod` line for each new finder test module, in alphabetical order, in
     `crates/finder/tests/console/main.rs` (`mod stack_accumulate;`,
     `mod stack_colour;`, `mod stack_console;`, `mod stack_display;`,
     `mod stack_gate;`, `mod stack_loop;`, `mod stack_placement;`,
     `mod stack_reject;`), `crates/finder/tests/cli/main.rs` (`mod stack;`) and
     `crates/finder/tests/mcp/main.rs` (`mod stack;`), as 027's layout asks;
   - in `crates/finder/web/playwright.config.ts`, the string `"**/stack.spec.ts"` added
     to the default project's `testIgnore` list and to the simulate project's
     `testMatch` list, each on a line of its own (owner decision 23).
10. **No dependency, no scope-link change, no config key and no flag.** Nothing under
    `crates/scope-node` or `crates/scope-link` changes, and no `Cargo.toml` or
    `Cargo.lock`.
11. **The purpose label `sub`** on every exposure a stack asks for (FR-014), which
    030's list of labels already names, and **the holder name `the stack`** under which
    it takes 031's lease (FR-013).

## User Scenarios & Testing *(mandatory)*

Every scenario shares these definitions unless it says otherwise.

- **The gate command.** Every Rust scenario runs in
  `cargo nextest run --workspace --all-targets --all-features --locked`, the `test`
  gate's first command, each test in its own process. The page's scenarios run in
  `npm test` and `CI=1 npm run e2e`, later commands of the same gate, in
  `crates/finder/web`.
- **Paused time.** A Rust test that waits runs under
  `#[tokio::test(start_paused = true)]`, so every wait is in virtual time. A test of a
  pure function is a plain `#[test]`. US8-S7 starts the built `finder` and is a plain
  `#[test]` in real time.
- **A position** on a grid is (x, y) in pixels: x to the right from the left edge of
  column 0, y downward from the top edge of row 0. The pixel at column c, row r covers
  the square from (c, r) to (c + 1, r + 1), so its centre is at (c + 0.5, r + 0.5). The
  centre of a grid W wide and H high is (W/2, H/2). Up is toward smaller y.
- **A colour picture** (`Colour`) has three planes, red, green and blue, each row by row
  from the top; the value of channel c at (x, y) is at index `c·W·H + y·W + x`.
- **A solve of the form (α, δ, θ, s)** is `ImagingSolve::from_cd(α, δ, [[−k·cos θ,
  −k·sin θ], [k·sin θ, −k·cos θ]], W, H)` with k = s/3600: centred on J2000 (α, δ) in
  degrees, up at θ east of north, s arcseconds a pixel, not mirrored (032's matrix for
  such an imaging frame). **A truth solve** of a simulated imaging frame is that form
  with
  `sim.imaging_truth_at(t)` for t the middle of its exposure.
- **Projecting by hand.** A test that needs where a sky place (α, δ) falls on a grid
  with the solve (α0, δ0, θ, s) works it out itself, calling nothing under
  `finder::imaging`: with b = (cos δ0·cos α0, cos δ0·sin α0, sin δ0), e = (−sin α0,
  cos α0, 0), n = (−sin δ0·cos α0, −sin δ0·sin α0, cos δ0) and v the place's unit
  vector, ξ = (v·e)/(v·b) and η = (v·n)/(v·b); then dx = (−ξ·cos θ + η·sin θ)·R/s and
  dy = (−ξ·sin θ − η·cos θ)·R/s with R = 206264.806, and the position is (W/2 + dx,
  H/2 + dy). The inverse takes (dx, dy) to ξ = (−dx·cos θ − dy·sin θ)·s/R, η = (dx·sin θ
  − dy·cos θ)·s/R and v ∝ b + ξ·e + η·n.
- **The centroid** of a star in a plane is the mean of pixel centres weighted by
  (value − background) over the 25 × 25 pixels about the brightest one, which is 030's
  centroid; here the background is the plane's median unless a scenario gives one.
- **The sim rig** is 030's: a `Simulator` over the test database at the site 40° N 75° W
  on a `VirtualClock` from Unix 1768528800 (`2026-01-16T02:00:00Z`), given
  `push_to(50.0, 120.0)`, as `crates/finder/tests/support/live.rs:41` builds it, and a
  `SimImagingCamera::new(sim.clone())` over it: 1236 × 822 over 0.75°, about 2.1845″ a
  pixel, `RGGB`, gain 120 and offset 10 in force. **sim** is that `Simulator`. A test
  that wants one place at the grid's centre in every imaging frame calls
  `sim.point_eyepiece_at(α, δ)` (`crates/simulator/src/controls.rs:97`) just before
  each exposure. With gain 0 applied one electron is 0.25 counts, the offset level is
  160 counts and the sky adds 5 counts a second.
- **The stand-in stacking rig** is 031's stand-in console rig (an in-process console
  paired to a node whose imaging camera is the stand-in, 64 × 48, 16 bits, `RGGB`,
  whose every imaging frame at the settings it starts with reads
  `416 + ((c + 3·r) mod 251)` at column c, row r), built with `with_imaging(R)` and
  then `with_imaging_solver` of **the scripted solver**. The console's exposure is
  031's default, 1000 ms. No master is in the library, so calibration changes no
  sample of such an imaging frame, and in its colour picture every plane reads
  `416 + x + 3·y` at every pixel with 1 ≤ x ≤ 62 and 1 ≤ y ≤ 46 (worked example: a
  bilinear rule returns a linear ramp unchanged away from the edges). The rig has no
  live pointing, because the node's finder camera is the scope node's stand-in, whose
  stars are no sky; it has no target and no aim point either. So 032's
  `Imaging::hint` answers there with the source `none` and a radius of 180°.
- **The scripted solver** is an `ImagingSolver` of the tests' own. Its nth `solve`,
  n from 1, answers what the scenario scripts for n, and otherwise **the standing
  solve**: the form (83.8221°, −5.3911°, 30°, 2.16″) for the imaging frame's own width
  and height, with `stars` 50. Its nth `measure` answers the scripted star size or
  error, and otherwise 2.0 px with 50 stars. With a delay it waits that long on tokio
  time before each `solve`, and with a measure delay that long before each `measure`.
  It keeps every hint it was given and, for every `measure`,
  the width of the imaging frame and the region it was given. Its `name()` is `simulated`. A
  scenario writes `NotSolved` and `TooFewStars` for `SolveError::NotSolved { detail:
  None }` and `SolveError::TooFewStars { detail: None }`.
- **The scripted camera** is an `ImagingCamera` of the tests' own that wraps 030's
  stand-in camera and stands in its place on the node: every call passes through, but
  an `expose` the scenario scripts to fail waits the exposure on tokio time and then
  answers `Failed` with the scripted text, an `expose` scripted to carry a gain
  answers its imaging frame with that `gain`, and an `expose` scripted to add counts
  answers its imaging frame with that many counts added to every sample.
- **The sim stacking rig** is 032's solving rig: the sim rig's `Simulator` with a
  `SimMount::new` connected first and `push_to(50.0, 120.0)` after, behind an in-process
  console built with `with_live()`, `with_imaging(R)` and
  `with_imaging_solver(Arc::new(SimulatedSolver::new(sim.clone())))`. Its telescope
  does not track, so the sky drifts across the sensor between subs. Its finder camera
  sees the simulated sky, so once the session's `pointing` has the `source` `live`,
  032's `Imaging::hint` answers there with the source `pointing`, and with a radius of
  3°: no aim point is measured before the stack (032 US3-S1), and once the subs have
  measured one through `note_solve` the radius stays 3° because no scale is kept, and
  032 narrows the radius only when one is (032 US5).
- **The stack's view** is `state.stack().unwrap().view()` in US5 to US7, and the `stack`
  object of `GET /api/imaging` from US8 on. **Waiting until** a condition means reading
  it every 100 ms until the condition holds, for at most 300 s of virtual time, and
  failing otherwise. A rig's scenario first waits until `GET /api/imaging` has `state`
  `ready`. **Σ dropped** is the sum of every count in the view's `dropped`, taken over
  whatever keys it holds, eleven here; a test never asserts how many there are, since a
  later spec adds reasons.
- **Rigs are written once.** The scripted solver, the scripted camera, the stand-in
  stacking rig, the wait and a FITS reader of the tests' own are written in the module
  `stack_loop` (US5), declared `pub(crate)`, and imported by the later `console`
  modules with `use crate::stack_loop::…` (plan.md § Test rigs gives their names). The
  sim stacking rig is written in `stack_gate` (US6), the one `console` module whose
  scenarios use it since round 1's cut, with the private helpers it needs copied from
  `stack_loop`. A module declares only what its own tests use; a later story adds nothing to an earlier
  story's module. The `mcp` and `cli` binaries cannot import from `console`, so
  `crates/finder/tests/mcp/stack.rs` writes the part it needs again.
- **Scratch folders.** A finder test module takes its folder from
  `crate::support::Scratch::new(name)` (`crates/finder/tests/support/mod.rs:12`), which
  makes the folder under `CARGO_TARGET_TMPDIR` and removes it when dropped. **R** is the
  child path `imaging` of the test's scratch folder.
- **Requests** go through the router in-process, as `request` in
  `crates/finder/tests/support/live.rs:90` sends them, with `Content-Type:
  application/json` and no `Origin`. A scenario that names an `Origin`, another
  `Content-Type` or the method `DELETE` builds its request with a helper of the
  module's own, with a `Host` of localhost on port 8080. A foreign `Origin` is
  `http://elsewhere.example`.
- **Reading files in a test.** A test parses `stack.json` and a solve sidecar itself
  with `serde_json`, decodes a PNG or JPEG with `image`, and reads a FITS file with
  code of its own, never through the console's FITS code: 80-byte cards up to `END`,
  the header padded to 2880 bytes, then big-endian samples; for `BITPIX` −32 they are
  IEEE 754 single-precision numbers, plane after plane, each row by row from the top.
- **Comparing.** Two real numbers are equal when they differ by at most 0.001, unless a
  scenario gives another tolerance. Counts and texts are exact. A test compares a view
  key by key, never whole, because later stories and specs add keys.
- **Oracles.** Each scenario names its oracle: a worked example given here, the
  simulator's truth or its declared scene, the stand-in camera's sample rule, the
  scripted solver's answers, or files the test read itself.

### User Story 1 - A satellite's streak and a faint glow in the simulated sky (Priority: P1)

As a test of any stacking story, I can give the simulated imaging camera a satellite's
streak on the imaging frames I choose and a faint round glow fixed on the sky, and I
know exactly what each adds to an imaging frame.

**Why this priority**: Rejection and the growth of signal over noise can only be proved
against light whose place and strength a test knows.

**Independent Test**: `crates/simulator/tests/imaging_scene.rs`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p simulator --locked --test imaging_scene`.

**Acceptance Scenarios**:

1. **Given** the sim rig, **Then** a committed test asserts that `sim.scene()` is
   `Scene::default()`, with no satellite and no source, and that `sim.set_scene` with
   a source whose `sigma_arcsec` is 0.0 panics. **When** two cameras built alike
   with noise off, no stars (`with_stars(vec![])`) and gain 0 each take a 1000 ms
   imaging frame, the second after `sim.set_scene(Scene::default())`, **Then** the
   committed test asserts that every sample of both is exactly 165 and that the two
   imaging frames are equal byte for byte (worked example: 160 of offset and 20
   electrons of sky at 0.25).
2. **Given** the sim rig with the scene's one satellite `SatelliteStreak { frames:
   vec![1], from: (100.0, 200.5), to: (300.0, 200.5), electrons: 4000.0 }`, a camera with
   noise off, no stars and gain 0, **When** a test takes three 1000 ms imaging frames,
   **Then** a committed test asserts that every sample of the first and of the third is
   exactly
   165; and that the second reads, each within 1 count, 1165 at (150, 200), 872 at
   (150, 201) and at (150, 199), 209 at (150, 203), 169 at (150, 204), 165 at
   (150, 207), 1082 at (99, 200), 165 at (50, 200), and the same value at (151, 200)
   as at (150, 200) whatever their filters (worked example: 160 + 5 +
   0.25·4000·exp(−d²/(2·1.2²)) for a sample whose centre is d from the segment, with d
   of 0, 1, 3, 4 and, past the end, 0.5, the 4 being 165 + 1000·exp(−16/2.88) = 168.9;
   nothing beyond ⌈4·1.2⌉ + 1 = 6). With `sim.set_focus_error(2.0)` set before the
   three imaging frames, the second reads 1077 at (150, 201), within 1 count (worked
   example: σ² = 1.2² + 2.0² = 5.44, and 165 + 1000·exp(−1/10.88) = 1077.2).
3. **Given** the sim rig, a camera with noise off, no stars and gain 0, and the scene's
   one source `ExtendedSource { ra_deg, dec_deg, sigma_arcsec: 20.0·k, electrons_per_s:
   800.0, colour: [0.5, 1.0, 0.25] }`, with (`ra_deg`, `dec_deg`) the truth's centre for
   the middle of the exposure and k the truth's arcseconds a pixel, **When** a test
   takes a 1000 ms imaging frame, **Then** a committed test asserts, each within 2
   counts, that it reads 365 at (618, 411), 265 at (618, 410), 215 at (619, 411),
   283 at (638, 411) and 173 at (668, 411), and exactly 165 at (0, 0); and that a
   500 ms imaging frame taken next, with the source's place set again to the truth's
   centre for the middle of that exposure, reads 262 at (618, 411) (worked example:
   165 + 0.25·800·colour·exp(−d²/(2·20²)), d from the sample's centre to the grid's
   centre; green at (618, 411), red at
   (618, 410), blue at (619, 411); at (668, 411), 165 + 200·exp(−(50.5² + 0.5²)/800)
   = 173.3; for 500 ms, 160 + 2.5 + 99.9). With noise on, a source of
   `electrons_per_s` 40000.0 and `colour` `[1.0, 1.0, 1.0]` at the truth's centre for
   the middle of each exposure, the 11 × 11 samples about (618, 411) of a 1000 ms
   imaging frame differ from those of one taken the same way with noise off by a
   standard deviation of at least 20 counts (worked example: 0.25·√(40000 + 20 + 9) =
   50 counts; the sky and the read noise alone give 1.35).
4. **Given** the scenes of US1-S2 and US1-S3 together, the glow of US1-S3 placed at
   the truth's centre for the middle of the first imaging frame, noise off and gain 0,
   **Then** a committed test asserts that with 034's `cover` true every sample of the
   second imaging frame is exactly 160; that with 034's flat panel
   `FlatPanel { electrons_per_s: 2000.0, colour: [0.6, 1.0, 0.8] }` the second imaging
   frame equals, byte for byte, one taken the same way with `Scene::default()`; and
   that with `corner_light` 0.5 and neither cover nor panel the second imaging frame
   reads at (150, 200),
   within 1 count, 160 + (5 + 1000)·V(150, 200), V being 034's vignetting, which the
   test computes itself (oracle: 034's light path; the streak and the glow are light).

---

### User Story 2 - Colour from the mosaic, and where a sub falls on the reference (Priority: P1)

As the stack, I turn a calibrated sub's mosaic into a colour picture of the same size,
and from two imaging solves I get the map that says where each pixel of the reference
grid falls in the sub.

**Why this priority**: Every later story stacks colour pictures through that map.

**Independent Test**: `crates/finder/tests/console/stack_colour.rs` and
`crates/finder/tests/console/stack_placement.rs`, the modules `stack_colour` and
`stack_placement` of finder's `console` test binary. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs both; during
development, `cargo nextest run -p finder --locked --test console stack_colour::` and
`cargo nextest run -p finder --locked --test console stack_placement::`.

**Acceptance Scenarios**:

1. **Given** a 4 × 4 `RGGB` mosaic whose red samples are 100.0, green 200.0 and blue
   300.0, **Then** a committed test asserts that `colour` gives a 4 × 4 picture whose
   every pixel, corners and edges included, is (100.0, 200.0, 300.0); the same for
   `BGGR`, `GRBG` and `GBRG` mosaics filled by their own patterns; that a 3 × 2 mono
   mosaic of 1.0 to 6.0 gives three planes each equal to its samples; and that a 1 × 4
   `RGGB` mosaic gives three planes each equal to its samples (oracle: worked example).
2. **Given** `RGGB` mosaics of 0.0 but for one sample, **Then** a committed test asserts
   of `colour`:
   - 6 × 6 with (2, 2) = 400.0: the red plane is 400.0 at (2, 2), 200.0 at (1, 2),
     (3, 2), (2, 1) and (2, 3), 100.0 at (1, 1), (3, 1), (1, 3) and (3, 3), and 0.0
     elsewhere, summing to 1600.0; the green and blue planes are 0.0 everywhere;
   - 4 × 4 with (0, 0) = 400.0: the red plane is 400.0 at (0, 0), 200.0 at (1, 0) and
     (0, 1), 100.0 at (1, 1), and 0.0 elsewhere;
   - 6 × 6 with (3, 2) = 400.0: the green plane is 400.0 at (3, 2), 100.0 at (2, 2),
     (4, 2), (3, 1) and (3, 3), and 0.0 elsewhere, summing to 800.0
   - (worked example: a channel a pixel does not sample is the mean of that colour's
     samples among the up to eight samples that touch it).
3. **Given** the reference solve (83.8221°, −5.3911°, 0°, 2.0″) for 1236 × 822, **Then**
   a committed test asserts of `Placement::between(reference, sub)`, numbers within
   0.000001:
   - with the sub the same solve: `m` is `[[1, 0], [0, 1]]` and `t` is `[0, 0]`;
   - with the sub (83.8221°, −5.3911°, 90°, 2.0″): `m` is `[[0, −1], [1, 0]]`, `t` is
     `[1029, −207]`, `apply((718.0, 411.0))` is (618.0, 511.0), `turn_deg()` is 90.0,
     its `area_scale()` is 1.0 and its `centre_shift((1236, 822), (1236, 822))` is
     (0.0, 0.0);
   - with the sub (83.8221°, −5.3911°, 180°, 2.0″): `m` is `[[−1, 0], [0, −1]]`, `t` is
     `[1236, 822]`, `apply((0.0, 0.0))` is (1236.0, 822.0) and `turn_deg()` is 180.0 or
     −180.0;
   - with the sub (83.8221°, −5.3911°, 0°, 4.0″) for 618 × 411: `m` is
     `[[0.5, 0], [0, 0.5]]`, `t` is `[0, 0]`, `area_scale()` is 0.25 and its
     `centre_shift((1236, 822), (618, 411))` is (0.0, 0.0);
   - with the sub `ImagingSolve::from_cd(83.8221, −5.3911, [[k, 0.0], [0.0, −k]], 1236,
     822)`, k = 2.0/3600, a mirrored sub: `m` is `[[−1, 0], [0, 1]]`, `t` is
     `[1236, 0]` and `area_scale()` is −1.0
   - (worked example: a point 100 px to the right of the reference's centre is 200″ west;
     in a sub whose up is east, west is down).
4. **Given** the reference solve (10.0°, 0.0°, 0°, 2.0″) and the sub (10.01°, 0.0°, 0°,
   2.0″), both 1236 × 822, **Then** a committed test asserts that `between` gives `m`
   within 0.000001 of the identity and `t` within 0.0001 of `[18, 0]`; that
   `apply((618.0, 411.0))` is (636.0, 411.0) within 0.0001; and that
   `centre_shift((1236, 822), (1236, 822))` is (−18.0, 0.0) within 0.0001 (worked
   example: 0.01° is 36″, 18 px, and east is to the left).
5. **Given** the reference solve (30.0°, 60.0°, 0°, 2.0″) and the sub (30.04°, 60.0°,
   0°, 2.0″), both 1236 × 822, **Then** a committed test asserts that, for each of the
   reference positions (118, 411), (1118, 411), (618, 111), (618, 711) and (1118, 711),
   `apply` is within 0.01 px of the place the test finds by projecting by hand: the
   reference position to the sky, and the sky to the sub; and that `turn_deg()` is
   −0.03464 within 0.0005 (worked example: the meridians converge, so the sub's north
   is turned from the reference's by 0.04°·sin 60°; a map made from the two angles
   alone would be the identity, wrong by 0.3 px at 500 px).
6. **Given** the reference solve of US2-S3 and a sub (203.8°, 5.4°, 0°, 2.0″), more than
   a quarter turn of the sky away, **Then** a committed test asserts that `between` is
   `None`; and that `Placement::identity().centre_shift((64, 48), (64, 48))` is
   `Some((0.0, 0.0))` and a placement whose `m` is all zero gives `None`.
7. **Given** a field of 3° × 2°: the reference solve (83.8221°, −5.3911°, 0°, 3.0″) for
   3600 × 2400, a near sub (84.0721°, −5.3911°, 0°, 3.0″) and a far sub (84.5721°,
   −5.3911°, 0°, 3.0″), each for 3600 × 2400, **Then** a committed test asserts that:
   - `between(reference, near)` is `Some`, and at each of the reference grid's four
     corners (0, 0), (3600, 0), (0, 2400) and (3600, 2400) its `apply` lies between
     0.2 and 0.3 px from the place the test finds by projecting by hand: the corner to
     the sky with the reference, and the sky to the near sub;
   - `between(reference, far)` is `None`;
   - `between` is `None` for each of the subs (84.4606°, −4.7544°, 0°, 3.0″) and
     (83.1836°, −4.7544°, 0°, 3.0″), each for 3600 × 2400, 0.9° from the reference
     toward the north-east and the north-west (worked example: each map strays 1.05 px
     at two opposite corners and 0.21 px at the other two, and its centre lies 1080 px
     from the reference's, inside `judge`'s 1200);
   - `stack::CORNER_SLACK_PX` is 0.5
   - (worked example: the near sub's centre is 0.249° east of the reference's on the
     sky, and its map strays 0.246 px at every corner; the far sub's is 0.747° east, 896
     px, under half the shorter side's 1200 px, so `judge` would pass its shift and its
     area scale of 1.00026, and only the corners, 0.738 to 0.739 px off, keep it out. At
     the worked example's size the map strays at most about 0.10 px at the off-field
     limit, so the check never drops a sub there. Oracle: the by-hand projection of the
     shared setup, which is the exact mapping).

---

### User Story 3 - Add subs on the reference grid (Priority: P1)

As the stack, I resample each sub's colour picture onto the reference grid through its
placing map and keep, for every pixel, the sum of what was added and how many subs
reached it, so that the mean is right where only some subs reach.

**Why this priority**: It is the stack itself. A single count for the whole picture
would darken every edge the sky's rotation uncovers.

**Independent Test**: `crates/finder/tests/console/stack_accumulate.rs`, the module
`stack_accumulate` of `console`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test console stack_accumulate::`.

**Acceptance Scenarios**:

1. **Given** an 8 × 6 colour picture P whose red is 100 + 2·x + 3·y, green 50 + x and
   blue 7.0, and `Stack::new(8, 6)`, **When** a test adds P with
   `Placement::identity()`, **Then** a committed test asserts the answer `covered` 48,
   `added` 48, `rejected` 0 and `offsets` `[0.0, 0.0, 0.0]`, `subs()` 1, every weight
   1.0, `sum()` equal to P's planes and `mean()` equal to P; and after the same again,
   every weight 2.0, `sum()` twice P's planes and `mean()` equal to P.
2. **Given** P and fresh stacks of 8 × 6, **Then** a committed test asserts:
   - with `Placement { m: [[1, 0], [0, 1]], t: [2, 1] }`: `covered` 30; the weight is
     1.0 where x ≤ 5 and y ≤ 4 and 0.0 elsewhere; the mean's red there is
     107 + 2·x + 3·y; and the mean is 0.0 in every channel where the weight is 0.0;
   - with `t` `[0.5, 0]`: `covered` 42; the mean's red is 101 + 2·x + 3·y where x ≤ 6
     and the weight is 0.0 at x = 7;
   - with `t` `[100, 0]`: `covered` 0, every weight 0.0 and `subs()` 1;
   - on one stack, the identity and then `t` `[2, 1]`: the weight is 2.0 where x ≤ 5 and
     y ≤ 4 and 1.0 elsewhere, the mean's red is 103.5 at (0, 0) and 129.0 at (7, 5)
   - (worked example: a bilinear rule returns a linear ramp exactly, and a pixel is
     reached only when its sample point lies among the sub's pixel centres).
3. **Given** P, **Then** a committed test asserts:
   - with `Placement { m: [[−1, 0], [0, −1]], t: [8, 6] }`, a half turn: `covered` 48
     and the mean's red at (x, y) is 129 − 2·x − 3·y;
   - with a 6 × 8 picture Q whose red is 10·x + y and
     `Placement { m: [[0, −1], [1, 0]], t: [6, 0] }` on a stack of 8 × 6, a quarter
     turn: `covered` 48 and the mean's red at (x, y) is 10·(5 − y) + x;
   - with `t` `[−0.0000005, 0]`: `covered` 48 and the mean equals P; with `t`
     `[−0.01, 0]`: `covered` 42;
   - with P's blue set to `f32::NAN` at (3, 3) and the identity: `covered` 48, `added`
     47, `rejected` 1 and the weight 0.0 at (3, 3) alone
   - (worked example: reference pixel (x, y) takes the sub's pixel (7 − x, 5 − y) in the
     half turn and (5 − y, x) in the quarter turn).
4. **Given** the sim rig with the imaging optics' `focus_error` 2.0, gain 0 applied,
   noise off and five white stars of magnitude 7.0 at (ξ, η) of (0, 0) and
   (±0.15°, ±0.10°) about the first truth's centre, by the inverse of the shared
   setup's projection, **When** a test takes three 10 ms subs, the first with the
   optics' angle 17° and the centre re-pointed at the first truth's centre, the second
   with the angle 22° and the centre re-pointed at (ξ, η) of (0.02°, −0.01°) about the
   first truth's centre, by the same inverse, the third with the angle 197° and the
   first pointing again, turns each
   into colour and adds each to `Stack::new(1236, 822)` through `between` of the first
   sub's truth solve and its own, **Then** a committed test asserts that each
   placement's `turn_deg()` is within 0.02 of that sub's truth angle less the first
   sub's, the second's between 4.9 and 5.1 in size and the third's within 0.02 of 180
   in size; that for each of the five stars the centroid in the green plane of the
   stack's mean is within 0.02 px, in x and in y, of its centroid in the green plane of
   the first sub's own colour picture, each centroid taken about the brightest pixel
   within 6 px of the star's place in the first sub, found by projecting by hand, and
   its brightest value there, less the plane's median, is at least 0.85 of the first
   sub's, less its median; and that `covered` is at least 0.99 of 1,015,992 for the third
   sub and between 0.85 and 0.97 of it for the second (oracle: the simulator's truth
   and its rendering of the first sub; a sign error in the turn moves a star by 26 px,
   and a half-pixel error in the convention moves the half-turned sub by 1 px).
5. **Given** the sim rig with noise on, no stars, at the rig's own gain of 120, 0.995
   counts an electron, and the scene's one source at the first truth's centre with
   `sigma_arcsec` 30 times the truth's arcseconds a pixel, `electrons_per_s` 40.0 and
   `colour` `[1.0, 1.0, 1.0]`, **When** a test takes sixteen 100 ms subs, each after
   re-pointing at that centre, and adds each through
   `between` of the first sub's truth solve and its own to one stack, and the first
   four also to a second stack, **Then** a committed test asserts, with S the mean of
   the mean's green plane within 30 px of the grid's centre less its mean between 150
   and 200 px from it, and N the standard deviation of the green plane between 150 and
   200 px: that S of the sixteen is between 2.66 and 3.60; that N of the sixteen is
   less than 0.6 of N of the four; and that (S/N of the sixteen)/(S/N of the four) is
   between 1.7 and 2.3 (oracle: the simulator's declared source, 0.787 of 3.98 counts
   over the disc, and the square-root law: four times the subs, twice the signal over
   noise).

---

### User Story 4 - A satellite's streak leaves no mark (Priority: P1)

As the observer imaging the Orion Nebula, where the geostationary belt crosses the
field, I get a stack in which a streak that crossed one sub does not show, because a
pixel far from what the stack already holds there is not added.

**Why this priority**: Without it the showcase target's picture carries a line for
every satellite of the night.

**Independent Test**: `crates/finder/tests/console/stack_reject.rs`, the module
`stack_reject` of `console`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test console stack_reject::`.

**Acceptance Scenarios**:

1. **Given** `finder::imaging::stack::levels`, **Then** a committed test asserts that an
   8 × 6 picture whose planes are 5.0, 9.0 and 2.0 has the levels `[5.0, 9.0, 2.0]`;
   that a 1030 × 4 picture whose red is 1.0 where x is a multiple of 3 and 100.0
   elsewhere has the red level 1.0; and that a 4 × 1030 picture whose red is 1.0 where
   y is a multiple of 3 and 100.0 elsewhere has the red level 1.0 (worked example:
   with the longer side over 512 the level is the median of every third pixel of every
   third row, ⌈1030/512⌉ = 3).
2. **Given** `Stack::new(4, 4).with_background_match()`, **When** a test adds a picture
   of (100.0, 200.0, 300.0) and then one of (110.0, 195.0, 300.0), both with the
   identity, **Then** a committed test asserts that the second answer's `offsets` are
   `[−10.0, 5.0, 0.0]`, that the mean is (100.0, 200.0, 300.0) everywhere, and that
   without `with_background_match` the mean is (105.0, 197.5, 300.0); and that, on the
   stack made `with_background_match()`, a third picture of (120.0, 190.0, 300.0)
   answers `offsets` `[−20.0, 10.0, 0.0]`.
3. **Given** `Stack::new(2, 2).with_rejection()` and pictures the same in all three
   channels and all four pixels unless said, added with the identity, **Then** a
   committed test asserts, each case on a fresh stack:
   - after 98, 102, 98, 102, 98, 102, 98 and 102, a ninth of 106 but 107 at (0, 0)
     answers `covered` 4, `added` 3 and `rejected` 1; the weight is 8.0 at (0, 0) and
     9.0 elsewhere; the mean is 100.0 at (0, 0) and 100.667 elsewhere (worked example:
     the eight have mean 100 and deviation √(32/7) = 2.138, so 3 deviations are 6.41);
   - after the same eight, a ninth of 106.3 everywhere answers `added` 4 and
     `rejected` 0 (worked example: 3 × 2.138 = 6.41; over w it would be 6.0);
   - after eight of 100 added with `t` `[1.0, 0.0]`, which reach column 0 alone, a
     ninth of 100 and then a tenth of 200 with the identity: the tenth answers `added` 2
     and `rejected` 2 (worked example: column 1 holds one sub and is still learning);
   - after eight of 100, a ninth of 103 is added everywhere and a ninth of 104 nowhere
     (worked example: the deviation's floor is 1.0);
   - after eight of 100, a ninth of 100 whose blue alone is 130 at (1, 1) leaves the
     weight 8.0 and all three sums unchanged at (1, 1);
   - after seven of 100, an eighth of 10000 is added everywhere, with `rejected` 0;
   - the first case on `Stack::new(2, 2)` adds the ninth everywhere.
4. **Given** the sim rig with noise on and no stars, **When** a test takes twelve
   1000 ms subs with the scene's one satellite `SatelliteStreak { frames: vec![9],
   from: (200.0, 300.5), to: (1000.0, 500.5), electrons: 200.0 }`, and then twelve more
   from a second camera built alike after `sim.set_scene(Scene::default())`, and adds
   each dozen with the identity to stacks made
   `with_rejection().with_background_match()`, X from the first dozen and C from the
   second, and the first dozen also to a plain `Stack::new`, Y, **Then** a committed
   test asserts, with L the pixels whose centres lie within 1.0 px of the segment and
   at least 12 px from both its ends, and each mean taken of the mean's green plane
   over L: that Y's exceeds C's by at least 8.0; that X's is within 0.5 of C's; that
   the tenth answer for X has `rejected` at most 0.2 of 1,015,992 (noise alone rejects
   about 8% at the ninth sub); that the first eight answers for X have `rejected` 0;
   and that every `offsets` value for X is within 0.5 of 0.0 (oracle: the simulator's
   declared streak and a control stack with the same noise; the streak adds 140 to 200 counts
   to one sub in twelve where a pixel's deviation is about 5).

---

### User Story 5 - The stack on the console: take, solve, place, add, stop (Priority: P1)

As the observer whose telescope is tracking a target, I start a stack, and the console
takes subs one after another, solves each, calibrates it and adds it to the stack, with
the first sub that solves as the reference sub, until I stop it. A sub that does not
solve is dropped and counted with the reason, and while the finder camera solves too,
every stacked sub keeps the aim point measured.

Round 1's probe build measured this story over the 64 KiB budget, so three of its
scenarios are User Story 6's: the refusals of `start` (US6-S7), a sub placed by two
simulated solves (US6-S8) and the aim point the stacked subs keep measured (US6-S9).
What they test is still this story's code: `start` and its refusals, the folder and
`Lease::save_in`, and the call of `note_solve`. Its other scenarios were renumbered
(§ Clarifications, the session of 2026-11-19).

**Why this priority**: It is the first time a picture grows on the console from subs
the camera takes.

**Independent Test**: `crates/finder/tests/console/stack_loop.rs`, the module
`stack_loop` of `console`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test console stack_loop::`.

**Acceptance Scenarios**:

1. **Given** the stand-in stacking rig, **When** a test calls
   `start(StartRequest::default(), None)` on `state.stack().unwrap()`, waits until the
   view's `stacked` is at least 5 and calls `stop()`, **Then** a committed test asserts
   that:
   - the first answer has `state` `stacking`, an `id` of 20 characters beginning
     `2026-01-16T02-` and ending `Z`, `taken` 0, `stacked` 0 and `options`
     `{"reject":true,"focus_gate":true,"gains":[1.0,1.0,1.0],"stretch":10.0}`;
   - the last has `state` `ended`, `ended_reason` `stopped`, an `ended_at` at least its
     `started_at`, `taken` equal to `stacked` plus Σ dropped, every count of `dropped`
     0 but `unprocessed`, which is at most 1, and `exposure_s` equal to `stacked`;
   - `reference` has `frame_id` 1, `width` 64, `height` 48, `ra_deg` 83.8221, `dec_deg`
     −5.3911, `up_east_of_north_deg` 30.0, `arcsec_per_px` 2.16 and `exposure_ms`
     1000.0; `last` has `outcome` `stacked`, `turn_deg` 0.0, `shift_px` `[0.0, 0.0]`,
     `rejected_px` 0 and a `took_ms` that is a number not below 0; and `missing` is
     `["dark","flat","bad_pixels"]`;
   - `state.stack().unwrap().mean()` is a 64 × 48 picture whose three planes each read
     `416 + x + 3·y` at every pixel with 1 ≤ x ≤ 62 and 1 ≤ y ≤ 46;
   - the view read when `stacked` first reaches 2 has `queued` 0;
   - for every frame id from 1 to `taken`,
     `runs/<stack id>/subs/<frame id as six digits>.fits` under R, `<stack id>` being
     the view's `id`, has the card `PURPOSE` `sub`, and for every stacked one the solve
     sidecar beside it has `solved` true; and R holds no file under `frames/` (owner
     decision 13: a stack's subs are kept in its own folder);
   - the first hint the scripted solver was given has `source` `none`, and from its
     second on every hint has `source` `last_solve`, the place (83.8221, −5.3911),
     `radius_deg` 0.048 and `fov_deg` 0.0288, each within 0.000001
   - (oracle: the stand-in camera's sample rule and the standing solve; a sub's
     diagonal is 80 px, 172.8″ at 2.16″ a pixel, and its height 48 px, 103.68″; the
     rig has no live pointing, so 032's hint never has the source `pointing` there,
     and the stack's own hint stands in its place once there is a reference sub).
   **Given** a fresh stand-in stacking rig whose scripted solver waits 5 s, with the
   1st sub's file overwritten by the test while its solve waits, **Then** the committed
   test asserts that `dropped.unreadable` is 1 and the 2nd hint has `source` `none`
   (worked example: a sub that solves but is not stacked is no reference sub); and, on
   a fresh rig with the 2nd `solve` scripted at (83.811662°, −5.385100°), that the 3rd
   hint's place is (83.811662, −5.385100) within 0.000001.
2. **Given** the stand-in stacking rig whose scripted solver answers its 1st and 2nd
   `solve` with `NotSolved`, its 5th with `TooFewStars`, its 7th with `Failed("boom")`
   and its 9th with `TimedOut { seconds: 120 }`, **When** a test starts a stack, waits
   until `taken` is at least 10 and stops it, **Then** a committed test asserts
   `reference.frame_id` 3, `dropped.not_solved` 2, `dropped.too_few_stars` 1,
   `dropped.solver_failed` 2, `stacked` equal to `taken` less 5 less
   `dropped.unprocessed`, and that the files of all `taken` subs exist in the stack's
   `subs` folder, the five dropped ones among them. **Given** a fresh stand-in stacking
   rig with the 2nd `solve` scripted at (263.8221°, +5.3911°), **Then** the committed
   test asserts that once `last.frame_id` is 2 the view has
   `last.outcome` `off_field` and `dropped.off_field` 1, and `GET /api/session` has
   `imaging.relation.last.frame_id` 1 (worked example: no point of the reference grid
   has a place in a sub solved at the antipode). This scenario was US5-S3 until round
   1's cut.
3. **Given** the stand-in stacking rig whose node holds a scripted camera, **Then** a
   committed test asserts that:
   - with its 2nd `expose` scripted to fail with `the camera fell over`, a stack run
     until `taken` is at least 4 has `dropped.frame_failed` 1 and `state` `stacking`;
   - with every `expose` from its 2nd on scripted so, the stack ends by itself with
     `state` `ended`, `ended_reason` `camera`, `error`
     `the exposure failed: the camera fell over`, `taken` 4, `dropped.frame_failed` 3
     and `stacked` plus `dropped.unprocessed` equal to 1; and `POST /api/imaging/frame`
     sent after that answers 200, the stack having released the camera;
   - with its 2nd, 4th and 6th `expose` scripted to fail, a stack run until `taken` is
     at least 8 has `state` `stacking` and `dropped.frame_failed` 3;
   - with its 2nd `expose` adding 100 counts to every sample, `mean()` read when
     `stacked` is 2 equals the one read when it was 1, within 0.001;
   - once `stacked` is 1 the test removes `runs/<id>/subs`, and the view reaches
     `dropped.frame_failed` 1.
   This scenario was US5-S4 until round 1's cut.
4. **Given** the stand-in stacking rig whose scripted solver waits 30 s, **When** a test
   starts a stack, waits until `taken` is at least 3, calls `stop()` and then lets 60 s
   pass, **Then** a committed test asserts that `stop()` returned within 1 s of virtual
   time with `state` `ended`, `ended_reason` `stopped`, `stacked` 0 and
   `dropped.unprocessed` equal to `taken`, and that the view 60 s later equals it key
   for key. **Given** a fresh stand-in stacking rig, **Then** the committed test asserts
   that with the exposure set to 10 000 ms through `Imaging::set` before the start, a
   test that calls `stop()` once `taken` is at least 1 finds it returned within 1 s of
   virtual time. This scenario was US5-S5 until round 1's cut.
5. **Given** the stand-in stacking rig with a stack whose `stacked` has reached 2,
   **When** a test sends `POST /api/imaging/abort`, **Then** a committed test asserts
   that within 5 s the view has `state` `ended` and `ended_reason` `stopped`, and that
   `taken` equals `stacked` plus Σ dropped. This scenario was US5-S6 until round 1's
   cut.
6. **Given** the stand-in stacking rig with a stack whose `stacked` has reached 1,
   **When** a test sends `POST /api/imaging/frame`, **Then** a committed test asserts
   that the answer is 409 with a JSON `error` that contains `the stack`; that 5 s later the
   stack's `taken` is larger than it was at the request, its `state` is still
   `stacking` and `dropped.frame_failed` is 0; and that once `stop()` has answered, the
   same request answers 200 and the imaging frame it starts ends `saved` with the
   `purpose` `frame` and a `file` that begins `frames/2026-01-16/`, and that once
   `stop()` has answered, a frame the test takes with
   `state.imaging().unwrap().lease("the test")` and its `take("frame")` has a `file`
   beginning `frames/` (oracle: the plan
   document's amendment 1: one holder of the camera at a time, and a second taker
   refused with the holder's name; owner decision 13: the stack's folder goes with its
   lease). This scenario was US5-S8 until round 1's cut.

---

### User Story 6 - The gate, and a queue the camera never waits on (Priority: P1)

As the observer, I get a stack that leaves out a sub that is clearly softer than the
others, has drifted off the field, was solved to another scale or was taken at other
settings, each counted with its reason; and when the console cannot keep up, the camera
goes on taking subs and the oldest waiting ones are dropped.

Since round 1's cut this story also holds three scenarios of User Story 5's loop,
US6-S7 to US6-S9, moved here so that User Story 5 stays under the 64 KiB budget: the
refusals of `start`, a sub placed by two simulated solves, and the aim point the
stacked subs keep measured. They test User Story 5's code, and its module holds the sim
stacking rig they run on (§ Clarifications, the session of 2026-11-19).

**Why this priority**: One bad sub in fifty spoils a picture, and a stack that makes the
camera wait throws away clear sky.

**Independent Test**: `crates/finder/tests/console/stack_gate.rs`, the module
`stack_gate` of `console`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test console stack_gate::`.

**Acceptance Scenarios**:

1. **Given** `finder::imaging::stack::same_settings` and the reference settings
   `SubSettings { width: 64, height: 48, bits: 16, bayer: Some("RGGB".into()),
   binning: 1, roi: Region { x: 0, y: 0, width: 64, height: 48 }, gain: Some(120.0),
   offset: Some(10.0), exposure_ms: 1000.0 }`, **Then** a committed test asserts that
   it is true for a sub with the same settings, with `gain` `Some(120.0005)` and with
   `exposure_ms` 1000.5; true when `gain` and `offset` are `None` in both; and false
   for a sub that differs from the reference in one thing alone, for each of: `width`
   32, `height` 24, `bits` 8, `bayer` `Some("GRBG".into())`, `bayer` `None`, `binning`
   2, `roi` with `x` 2, `gain` `Some(120.002)`, `gain` `None`, `offset`
   `Some(10.002)`, `offset` `None`, `exposure_ms` 1001.5 and `exposure_ms` 998.5
   (worked example: size, depth, pattern, binning and region equal; gain and offset
   each absent in both or within 0.001; the exposure within 1 ms). **Given** the
   stand-in stacking rig whose node holds a scripted camera whose 4th `expose` carries
   the gain 50.0, **When** a test runs a stack until `taken` is at least 6 and stops
   it, **Then** the committed test asserts `dropped.settings` 1 and `stacked` equal to
   `taken` less 1 less `dropped.unprocessed` (oracle: the scripted camera; every other
   sub has the stand-in's own gain, and each is compared with the reference sub).
2. **Given** the stand-in stacking rig, **Then** a committed test asserts, each on a
   fresh rig run until `taken` is at least 10 and stopped unless it says otherwise:
   - with the scripted star sizes 5.0 px for the 2nd `measure`, 2.7 px for the 7th and
     2.5 px for the 8th: `dropped.soft` is 1, `focus_median_arcsec` is 4.32, and
     `stacked` is `taken` less 1 less `dropped.unprocessed` (worked example: 2.0 px at
     2.16″ is 4.32″; 2.7 is 35% above the median of the six before it and 2.5 is 25%;
     the 2nd is not judged, because fewer than 5 are known);
   - with the 3rd `measure` answering `TooFewStars` and the 4th `Failed("x")`:
     `dropped.too_few_stars` 1 and `dropped.solver_failed` 1;
   - with the scripted star size 25.0 px for the 3rd `measure`: `dropped.soft` is 1 and
     `stacked` is `taken` less 1 less `dropped.unprocessed` (worked example: 033 calls
     a star measure above 20 px far out, and such a sub is dropped however few
     readings are known);
   - started with `focus_gate` `Some(false)`: the scripted solver's `measure` was never
     called, `dropped.soft` is 0 and the view has no `focus_median_arcsec`;
   - with 2.7 px for the 6th `measure` alone, `dropped.soft` is 1; with 3.0 px for the
     2nd `measure`, the view read when `stacked` first reaches 2 has
     `focus_median_arcsec` 4.32 (worked example: of 4.32 and 6.48 the value at index
     ⌊0.5·(2 − 1)⌋ is 4.32);
   - with 3.0 px for the 6th to the 12th `measure`, a stack run until `taken` is at
     least 13 has `dropped.soft` 7 (worked example: R stays five of 4.32; were soft
     readings to join, the 12th would be judged against 6.48);
   - with 2.7 px for the 7th `measure`, the view whose `last.frame_id` is 7 has
     `last.outcome` `soft` and `last.focus_arcsec` 5.832, and the one for 8 has
     `last.focus_arcsec` 4.32;
   - with each `measure` waiting 30 s, a stop asked once `taken` is 2 returns within
     1 s with `stacked` 0 and `dropped.unprocessed` equal to `taken`.
3. **Given** `finder::imaging::stack::judge` and the stand-in stacking rig, **Then** a
   committed test asserts that:
   - `judge` of the identity for (64, 48) and (64, 48) is `Ok`; of `t` `[−20, 0]`,
     `Ok`; of `t` `[−30, 0]`, `Err("off_field")`; of `m` `[[0.94, 0], [0, 0.94]]`,
     `Err("solve_differs")`; of `m` `[[−1, 0], [0, 1]]` with `t` `[64, 0]`,
     `Err("solve_differs")`; of `m` `[[−1, 0], [0, −1]]` with `t` `[64, 48]`, `Ok`;
     of `t` `[−17, −17]`, `Err("off_field")`; of `m` `[[−1, 0], [0, 1]]` with `t`
     `[94, 0]`, `Err("solve_differs")`; and of `m` `[[NaN, 0], [0, 1]]`,
     `Err("solve_differs")` (worked example: half the shorter side is 24 px; 0.94² is
     0.884; 17·√2 is 24.04, over 24; the mirror is refused before its 30 px shift is
     read);
   - on the rig, with the scripted solves the standing solve but centred on
     (83.811662°, −5.385100°) for the 3rd sub and on (83.806442°, −5.382100°) for the
     4th, at 2.30″ a pixel for the 5th and up at 210° for the 6th, a stack run until
     `taken` is at least 7 has `dropped.off_field` 1 and `dropped.solve_differs` 1; and
     of the views the test read every 100 ms, the one whose `last.frame_id` is 3 has
     `last.outcome` `stacked` and `last.shift_px` within 0.01 of `[20.0, 0.0]`, the one
     for 4 has `last.outcome` `off_field`, the one for 5 `solve_differs`, and the one
     for 6 `stacked` with `last.turn_deg` within 0.000001 of 180.0 or −180.0
   - (worked example: the two centres are where the reference grid's positions (52, 24)
     and (62, 24) fall on the sky).
4. **Given** the stand-in stacking rig whose scripted solver waits 10 s, **When** a test
   starts a stack, reads the view every 100 ms until `taken` is at least 30 and stops
   it, **Then** a committed test asserts that every read had `queued` at most 4; that
   `taken` reached 30 within 35 s of virtual time; that `dropped.behind` is at least
   15 and `stacked` at least 2; that `taken` equals `stacked` plus Σ dropped; and that
   the view read when `stacked` first reaches 2 has `last.frame_id` at least 7
   (measured 8: when the first solve ends at 11 s the four waiting are the newest;
   oracle: one sub a second against one solve in ten).
5. **Given** the stand-in stacking rig, **Then** a committed test asserts, each on a
   fresh rig:
   - with the scripted solver answering its 4th, 5th, 6th and 7th `solve` with
     `NotSolved`, and a stack run until `stacked` is at least 5: the hints of the 2nd
     to the 6th `solve` have `source` `last_solve` and `radius_deg` 0.048 within
     0.000001; the 7th's and the 8th's have a `source` other than `last_solve` and a
     `radius_deg` of at least 3.0; and the 9th's has `source` `last_solve` and
     `radius_deg` 0.048 again (worked example: the third miss in a row is the 6th
     `solve`, so the 7th is given 032's hint; the 7th misses too, so the 8th is as
     well; the 8th solves, and the stack's own hint is back for the 9th);
   - with the scripted solver answering its 4th and 5th `solve` with `NotSolved` and
     its 3rd `measure` with `TooFewStars`, and a stack run until `taken` is at least
     7: `dropped.too_few_stars` is 1, `dropped.not_solved` is 2, and the hint of the
     6th `solve` has `source` `last_solve` and `radius_deg` 0.048 within 0.000001
     (worked example: the 3rd sub solved, so its too few stars in the focus gate count
     as no miss, and two misses in a row leave the stack's own hint in place);
   - with the 4th and 5th `solve` `NotSolved` and the 6th `Failed("x")`, the 7th hint
     has `source` `last_solve`; with the 4th, 5th and 6th `TooFewStars`, the 7th hint's
     `source` is not `last_solve`
   - (oracle: the standing solve, and 032's hint, which on this rig has the source
     `none` and a radius of 180°).
6. **Given** the stand-in stacking rig, **When** a test sends `POST /api/imaging/focus`
   with `{"tile":{"col":1,"row":1}}`, waits until `focus.aid.readings` is at least 1,
   sends `DELETE /api/imaging/focus`, starts a stack, waits until `stacked` is at least
   3 and stops it, **Then** a committed test asserts that the imaging view's
   `focus.region` has `x` 20, `y` 16, `width` 20 and `height` 16; that every `measure`
   the scripted solver was given for an imaging frame 64 wide carried the region
   `Some(Region { x: 20, y: 16, width: 20, height: 16 })`; that there were at least 3
   of them; and that `dropped.soft` is 0 (worked example: 033's tile (1, 1) of a
   64 × 48 sensor at binning 1 is 20 × 16 pixels at (20, 16), and a sub of the whole
   sensor has the sensor's own pixels; the aid's own imaging frames are 20 wide). The
   committed test also asserts that after the `DELETE`, with
   `PUT /api/imaging/settings` `{"binning":2}` before the start, the subs' measures
   carry `Region { x: 10, y: 8, width: 10, height: 8 }` and `focus_median_arcsec` is
   4.32; with `{"roi":{"x":16,"y":8,"width":32,"height":32}}`,
   `Region { x: 4, y: 8, width: 20, height: 16 }`; with
   `{"roi":{"x":0,"y":0,"width":32,"height":24}}`, `None` (worked example: (region −
   roi)/b, and a region not inside the sub's roi is the whole sub; 033 multiplies the
   measure by b, the stack divides the scale by b).
7. **Given** the rigs named, **Then** a committed test asserts of `start`:
   - on the stand-in stacking rig, `gains` `[0.0, 1.0, 1.0]` answers a `Refusal` of 400
     `gains must be three numbers from 0.1 to 10`, `stretch` 0.5 one of 400
     `stretch must be a number from 1 to 1000`, and the two together the gains' words,
     and none of the three starts a stack;
   - on 031's stand-in console rig, which has no imaging solver, it answers 409
     `this console has no imaging solver`;
   - on a console whose node has no imaging camera, 409
     `this scope node has no imaging camera`, and `gains` `[0.0, 1.0, 1.0]` there
     still answers the 400 of the first case (worked example: a request's own numbers
     are judged before the camera's state);
   - on a console built without `with_imaging`, `state.stack()` is `None`;
   - on the stand-in stacking rig while the test itself holds the camera's lease, taken
     with `state.imaging().unwrap().lease("probe")`, a `Refusal` of 409 whose message
     contains `probe`, and no stack; once the test drops that lease, the default
     request answers `state` `stacking`;
   - on a fresh stand-in stacking rig while the test holds `lease("probe")` and a frame
     it took with that lease's own `take("frame")` is exposing, the test having read
     the view's `frame.state` `exposing` just before: a `Refusal` of 409 whose message
     contains `probe` and does not contain `an exposure is in progress`, and no stack
     (worked example: the stand-in's exposure is 1000 ms of tokio time and the start is
     asked at once; a holder's frame is passed over, and the lease names the holder);
   - on a fresh rig, right after `POST /api/imaging/frame` answered 200, with
     `state.imaging().unwrap().holder()` `None` and the view's `frame.state`
     `exposing`: 409 `an exposure is in progress` and no stack (worked example: a
     single imaging frame holds no lease, 031 US9, so it is the one frame in flight
     that the lease cannot name);
   - with a stack running: the default request answers the same `id`; `reject`
     `Some(false)` answers 409
     `the stack is running: stop it to change reject or focus_gate`; and `gains`
     `Some([2.0, 1.0, 1.0])` answers `options.gains` `[2.0, 1.0, 1.0]` and the same
     `id`;
   - on a stand-in stacking rig on which, after it is built and before the start, the
     test wrote a regular file named `runs` in R: a `Refusal` of 500 whose message
     begins `cannot make the stack folder:`, no stack, and
     `state.imaging().unwrap().holder()` `None` (worked example: the lease taken in
     step 6 is let go when the folder cannot be made);
   - on a fresh rig, a stack started and stopped and a second started in the same
     second: the second's `id` is the first's followed by `-2`, R holds
     `runs/<id>/subs/` for each of the two `id`s, and right after the second `start`
     answers, `mean()` is `None`;
   - `stop()` before any start answers `None`, and `view()` is `None`.
   This scenario was US5-S2 until round 1's cut.
8. **Given** the sim stacking rig, **When** a test waits until the session's `pointing`
   has the `source` `live`, starts a stack, waits until `stacked` is at least 4 and
   stops it, **Then** a committed test asserts that `reference`
   agrees with the solve sidecar of its `frame_id` (`ra_deg`, `dec_deg`,
   `up_east_of_north_deg` and `arcsec_per_px`, each within 0.000001); that `last` has
   `outcome` `stacked`, a `turn_deg` within 0.02 of its own sub's sidecar angle less
   the reference's, and a `shift_px` within 0.5 px of where the test, projecting by
   hand, finds that sidecar's centre on the reference grid, less the grid's centre;
   that the shift is longer than 3 px; that `dropped.not_solved`,
   `dropped.too_few_stars` and `dropped.solver_failed` are 0; and that the solve
   sidecar of the reference sub and the solve sidecar of `last`'s sub each have
   `hint.source` `pointing` and `hint.radius_deg` 3.0 (oracle: the simulated
   solver's answers, which are the simulator's truth, written to the sidecars; a
   telescope that does not track; and the plan document's order of a hint's sources,
   in which a live pointing comes before the stack's own last solve). The committed
   test also asserts that a stack run until `stacked` is 9 has `last.rejected_px` above
   0 (measured 52,746 at the 9th, from the simulated camera's noise). This scenario was
   US5-S7 until round 1's cut.
9. **Given** the sim stacking rig, whose Track is off, **When** a test waits until the
   session's `pointing` has the `source` `live` and then 5 s more, starts a stack,
   waits until `stacked` is at least 5 and stops it, **Then** a committed test asserts
   that `GET /api/session` has an `aim` whose `source` is `measured`, whose `x` is
   within 1.5 of `sim.optics().eyepiece_px.0` and whose `y` is within 1.5 of
   `sim.optics().eyepiece_px.1`; that `imaging.relation.readings` in that body is 5 and
   its `imaging.relation` has no `scale` key and no `held` key; that its `imaging` has
   no `solve` key, as before the stack; and that the solve sidecar of `last`'s sub has
   `hint.source` `pointing` and `hint.radius_deg` 3.0 (oracle: the simulator's true
   aim point, 700 and 420 at `crates/simulator/src/optics.rs:27`, read through
   `sim.optics()`; 1.5 finder pixels is 032 US4-S1's bound; 032's window of five aim
   readings, filled by the first five stacked subs through `note_solve`, and applied at
   once because Track is off; and 032's hint, which is narrow only with an aim point
   measured and a scale kept, while a sub's solve keeps no scale, as 032's US4-S7
   asserts of `note_solve`). This scenario was US5-S9 until round 1's cut.

---

### User Story 7 - The display picture (Priority: P2)

As the observer, I see the stack as a picture: no wider than 1920 pixels, its background
flat and neutral, its colours multiplied by the gains I set, and its brightness
stretched so that faint parts show and bright ones do not burn out.

**Why this priority**: The stack is numbers until it is shown; but nothing that is saved
depends on how it is shown.

**Independent Test**: `crates/finder/tests/console/stack_display.rs`, the module
`stack_display` of `console`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test console stack_display::`.

**Acceptance Scenarios**:

1. **Given** `finder::imaging::stack::display::reduce`, which is handed a stack's three
   planes of sums, its plane of weights, its width, its height and a `max_width`,
   **Then** a committed test asserts that sums 1920 wide and 1 high, with every weight
   1.0, stay 1920 wide for `max_width` 1920; that sums 1921 × 5 are reduced to 960 × 2
   and sums 3841 × 6 to 1280 × 2; that 4 × 2 sums whose red is 10, 20, 30, 0 in the top
   row and 50, 60, 0, 0 below, with the weights 1, 1, 1, 0 and 1, 1, 0, 0, reduced for
   `max_width` 2 are 2 × 1 with red 35.0 and 30.0, both covered, while with the
   right-hand four sums and weights all 0 the second is not covered and reads 0.0; and
   that 2 × 2 sums whose red is 30, 20, 0, 50, with the weights 3, 1, 0, 1, reduced
   for `max_width` 1 are one pixel, covered, with red 20.0 (worked example: a block's
   value is its sums added together over its weights added together, 100 over 5, so
   every sub counts once; the mean of the three pixels' own means would be 26.667).
2. **Given** `background` of an 800 × 600 reduced picture, all covered, **Then** a
   committed test asserts:
   - with red 100 + 0.1·(x + 0.5) + 0.05·(y + 0.5), green 200 − 0.02·(x + 0.5) and blue
     50.0: the red plane has `a` 0.1 and `b` 0.05 within 0.0005 and `c` 100 within 0.1;
     the green `a` −0.02 and `b` 0.0 within 0.0005 and `c` 199.99 within 0.002; the blue
     0.0, 0.0 and 50.0 (worked example: a tile's level is 034's `percentile` at 0.5,
     which for an even count is the lower of the two middle values; down a ramp of 0.02
     a pixel across a tile 100 wide that is the value half a pixel past the tile's
     centre, so the green `c` comes to 199.99 and not 200);
   - with 50.0 in every channel but 550.0 where 300 ≤ x < 500 and 225 ≤ y < 375: every
     plane is 0.0, 0.0 and 50.0 within 0.01 (worked example: the four tiles the square
     fills are among the brighter half and are left out);
   - with the first picture and only its pixels of y < 75 covered: every plane is
     flat, `a` and `b` 0.0 (worked example: the tiles that count lie on one line);
   - with the first picture and only its pixels of y < 37 covered: no tile is half
     covered, and every plane is flat at that channel's median over its covered pixels,
     red 140.925, green 191.99 and blue 50.0, within 0.01;
   - for a 4 × 4 picture of 7.0: every plane is 0.0, 0.0 and 7.0.
3. **Given** `tone` of a 3 × 3 reduced picture, all covered, with flat planes of 0.0,
   whose red is six of 0.0 and 300.0, 600.0 and 800.0, whose green is 0.0 and whose
   blue is seven of 0.0 and 150.0 and 300.0, **Then** a committed test asserts, for
   gains of 1.0 and a stretch of 10.0, that the reds of 0, 300, 600 and 800 become 0,
   197, 255 and 255, every green 0, and the blues of 150 and 300 become 140 and 197;
   that with the gains `[1.0, 1.0, 2.0]` the blues become 197 and 255; that with a
   stretch of 1.0 the red of 300 becomes 139; that a pixel not covered is (0, 0, 0)
   (worked example: the span is 600, the largest 99.9th percentile; 300 is t = 0.5 and
   255·asinh(5)/asinh(10) is 196.7); that with every red raised by 100, the reds of
   100, 400 and 700 become 0, 197 and 255; that with red 0, 10, 20, 20, 20, 20, 20, 20,
   300 and green and blue 0, the red of 300 becomes 249 (worked example: median 20,
   p16 10, span 30 × 10 = 300); that with 10·(x + 0.5) added to the red and a red plane
   of `a` 10, the reds are as with flat planes; and that with the uncovered pixel at
   5000.0 in every channel, it is (0, 0, 0) and the reds of 300, 600 and 800 are still
   197, 255 and 255.
4. **Given** `Stack::new(2400, 100)` to which a test has added once, with the identity,
   a picture of 1000.0 in every channel but 9000.0 where 1000 ≤ x < 1400, so that
   every weight is 1.0, **Then** a committed test asserts that `render` of that stack
   with gains of 1.0, a stretch of 10.0 and `max_width` 1920 is 1200 × 50; that its pixel at
   (100, 25) is (0, 0, 0) and its pixel at (600, 25) is (255, 255, 255); that
   `preview::jpeg` of it decodes to 1200 × 50 with every channel at most 6 at (100, 25)
   and at least 249 at (600, 25); that `shrink` of it to 1024 is 600 × 25 with the
   same two colours at (50, 12) and (300, 12); and that `shrink` of a 2 × 2 picture
   whose rows are (0, 0, 0), (1, 2, 255) to 1 is (1, 1, 128).
5. **Given** the stand-in stacking rig, **Then** a committed test asserts that before a
   stack starts, and while `stacked` is 0, the view has no `preview` and `preview()` is
   `None`; that once `stacked` is at least 1 the view's `preview` is
   `/api/imaging/stack/preview.jpg?n=` followed by a whole number of at least 1, and
   `preview()` decodes to a 64 × 48 JPEG; that `preview_within(32)` decodes to 32 × 24;
   that the answer to a `start` with `gains` `Some([2.0, 1.0, 1.0])` on the running
   stack has a `preview` whose number is larger than the one read just before it; that
   after `stop()` the view keeps its `preview` and `preview()` still answers; that with
   a solver that takes 3 s a sub, `preview` is absent while `stacked` is below 8 and is
   `/api/imaging/stack/preview.jpg?n=1` at `stacked` 9; that a start after `stop()`
   answers no `preview` and `preview()` is `None`; and that `gains`
   `[1.0, 1.0, 0.5]`, then `stretch` 1.0, each change `preview()`'s bytes.

---

### User Story 8 - Start, stop and keep a stack (Priority: P1)

As the observer, I start and stop the stack over the console's API, see it in the
observing session, and find it afterwards on the console's disk: the stack as a FITS
file of three planes of 32-bit reals that any astronomy program opens, the picture as a
PNG, and a list of every sub with what became of it.

**Why this priority**: Until it has a route nobody can start it, and until it is saved
the night's work is gone when the console stops.

**Independent Test**: `crates/finder/tests/console/stack_console.rs`, the module
`stack_console` of `console`, and, for US8-S7, `crates/finder/tests/cli/stack.rs`, the
module `stack` of `cli`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs both; during
development, `cargo nextest run -p finder --locked --test console stack_console::` and
`cargo nextest run -p finder --locked --test cli stack::`.

**Acceptance Scenarios**:

1. **Given** the stand-in stacking rig, **When** a test sends `POST /api/imaging/stack`
   with `{}`, waits until `stack.stacked` is at least 3 and sends
   `DELETE /api/imaging/stack`, **Then** a committed test asserts that:
   - both answers are 200 and are imaging views; the first has `stack.state`
     `stacking`; the second has `stack.state` `ended`, `stack.ended_reason` `stopped`
     and `stack.files` `{"fits":"runs/<id>/stack.fits","png":"runs/<id>/stack.png",
     "json":"runs/<id>/stack.json"}` with `<id>` the stack's `id`;
   - right after the `POST` answers, `runs/<id>/stack.json` has `format`
     `finder-stack/1`, `finder` the crate's version (`CARGO_PKG_VERSION`), `id` the
     stack's `id`, a `started_utc` of 23 characters whose 5th is `-` and whose 11th is
     `T`, `camera` `{"model":"synthetic imaging camera","serial":null}` and no
     `ended_at` key;
   - those three files exist under R, no name in `runs/<id>/` or in its `subs/` ends
     `.part`, and no file there contains `test-token`;
   - `stack.fits` is 40320 bytes; its cards are, in order, `SIMPLE` T, `BITPIX` −32,
     `NAXIS` 3, `NAXIS1` 64, `NAXIS2` 48, `NAXIS3` 3, `IMAGETYP` `STACK`, `STACKID` the
     id, `NSUBS` and `EXPTIME` both `stack.stacked`, `SUBEXP` 1.0, `DATE-OBS` (a text of
     23 characters, equal to the `DATE-OBS` card of `subs/000001.fits`), `INSTRUME`
     `synthetic imaging camera`, `XBINNING` 1, `YBINNING` 1,
     `ROWORDER` `TOP-DOWN`, `CTYPE1` `RA---TAN`, `CTYPE2` `DEC--TAN`, `CRPIX1` 32.5,
     `CRPIX2` 24.5, `CRVAL1` 83.8221, `CRVAL2` −5.3911, `CD1_1` −0.0005196152423,
     `CD1_2` −0.0003, `CD2_1` 0.0003, `CD2_2` −0.0005196152423, `END`, the last eight
     numbers within 0.000000000001; and its red plane reads 424.0 at (2, 2), its green
     425.0 at (3, 2) and its blue 428.0 at (3, 3);
   - `stack.png` decodes to a 64 × 48 picture of 8 bits a channel;
   - `stack.json` has `format` `finder-stack/1`, `id`, `target` null, `ended_reason`
     `stopped`, `taken`, `stacked` and `dropped` equal to the answer's, `options` equal
     to the answer's, `reference.frame_id` 1, `reference.file` `subs/000001.fits` and
     `reference.cd_deg`
     `[[−0.0005196152, −0.0003], [0.0003, −0.0005196152]]` within 0.0000000001, and
     `subs` with `taken` entries in the order taken, each with `frame_id`, `outcome`
     and the `file` `subs/<frame_id as six digits>.fits`, which exists in `runs/<id>/`
     under R; those whose `outcome` is `stacked` number `stacked`
   - (oracle: the stand-in camera's sample rule and the standing solve: 2.16″ is 0.0006°
     a pixel, up at 30°).
   **Given** a fresh rig whose scripted solver waits 3 s, **When** a test runs a stack
   until `taken` is at least 12 and sends the `DELETE`, **Then** the committed test
   asserts that `dropped.behind` and `dropped.unprocessed` are each at least 1 and
   `taken` is above `stacked`; that `stack.json`'s `subs` has `taken` entries, as many
   whose `outcome` is a reason of `dropped` as that reason's count, and `stacked` whose
   `outcome` is `stacked`; and that `stack.fits`'s `NSUBS` is `stacked`. **Given** a
   fresh rig whose scripted camera fails its 3rd `expose` with `the camera fell over`
   and answers its 5th with the gain 50.0, and whose scripted solver answers its 3rd
   `solve` with `NotSolved` and its 7th `measure` with 9.0 px, **When** a test runs a
   stack until `taken` is at least 12 and `queued` is 0 and sends the `DELETE`,
   **Then** the committed test asserts that `dropped.frame_failed`, `dropped.settings`
   and `dropped.not_solved` are each 1 and `dropped.soft` at least 1; that
   `stack.json`'s `subs` holds as many entries of each `outcome` as the counts say;
   that each `stacked` entry has a number `turn_deg`, a `shift_px` of two numbers and a
   whole number `rejected_px`; and that each `stacked` or `soft` entry has a number
   `focus_arcsec`. **Given** a fresh rig whose scripted solver answers its 1st `solve`
   with the standing solve centred on (83.822123456789°, −5.391123456789°), **When** a
   test runs a stack as the first case does, **Then** the committed test asserts that
   its `stack.fits` has `CRVAL1` 83.822123457 and `CRVAL2` −5.391123457, each within
   0.000000000001. And it asserts that of three
   stacks started on one rig with `{}`, with `{"gains":[1,1,0.1]}` and with
   `{"stretch":1}`, the second's `stack.png` differs from the first's in its blue, and
   the third's differs from the first's.
2. **Given** the stand-in stacking rig, **Then** a committed test asserts that
   `POST /api/imaging/stack` answers 403 for `{}` with a foreign `Origin`; 400 for
   `{"reject":"yes"}`, for `{"colour":true}` and for `{"gains":[1,2]}`; 415 for a body
   sent as `text/plain`; 400 with `gains must be three numbers from 0.1 to 10` for
   `{"gains":[0,1,1]}`; and, each with a JSON `error`, starts no stack; that
   `DELETE /api/imaging/stack` with no stack answers 200 with no `stack` key, and with a
   foreign `Origin` 403; that `GET /api/imaging/stack/preview.jpg` answers 404
   `the console holds no stack picture` before a stack has a picture and 200 with
   `Content-Type` `image/jpeg` after; and that on a console built without
   `with_imaging` the `POST` with `{}` and the `DELETE` answer 409
   `this console takes no imaging frames`, the `POST` with `{"reject":"yes"}` answers
   400, and the `GET` 404 (worked example: a body's shape is judged before the
   console is asked what it has; `{}` is a body of the right shape, and on a console
   that can stack it starts a stack with the defaults, as US8-S1 shows).
3. **Given** the stand-in stacking rig, **Then** a committed test asserts that with the
   session's target set to `M42` a stack's `id` ends `Z-M42` and `stack.json`'s `target`
   is `M42`; that with the session's target set to `NGC 7000`, which has no Messier
   name, a stack's `id` ends `Z-NGC-7000`; that a stack started and stopped before any
   sub is taken has `stacked` 0, a
   `files` object with `json` alone, no `stack.fits` or `stack.png` in its folder and
   an empty `subs` there; and that `finder::imaging::stack::service::label` gives
   `NGC-1976` for `NGC 1976`, `M42` for `M42`, `Barnard-s-Loop` for
   `Barnard’s Loop` and `N-buleuse` for `Nébuleuse` (worked example: every character
   that is not an ASCII letter or digit, the space, the right single quotation mark and
   the `é` among them, becomes one `-`;
   the session's target `M42` is NGC 1976, whose Messier name `M42` is the one
   `target_label` hands to `label`).
4. **Given** the stand-in stacking rig, **Then** a committed test asserts that
   `GET /api/imaging` and `imaging` of `GET /api/session` have no `stack` key before the
   first start; that during a stack both carry `stack` with the same `id`; that after
   it ended both still carry it, `ended`, with the same `id`; that a new start replaces
   it with another `id`; and that on a console built without `with_imaging` the body
   of `GET /api/imaging` has no `stack` key, and that of `GET /api/session` has no
   `imaging` key or an `imaging` without a `stack` key.
5. **Given** a stand-in stacking rig on which, after it is built and before the start,
   the test wrote a regular file named `runs` in R, **Then** a committed test
   asserts that `POST /api/imaging/stack` answers 500 with an
   `error` beginning `cannot make the stack folder:` and that no `stack` appears.
   **Given** a stack whose `stacked` has reached 2 and whose folder the test then
   removes, **Then** the committed test asserts that the `DELETE` answers 200 with
   `stack.state` `ended`, `stack.ended_reason` `stopped`, no `stack.files` and a
   `stack.save_error` beginning `could not save the stack:`. **Given** a stack in whose
   folder the test makes, mid-stack, a folder named `stack.fits.part`, **Then** the
   committed test asserts that the `DELETE` answers 200 with no `stack.files`, a
   `stack.save_error` beginning `could not save the stack:` and no `stack.png` in the
   folder.
6. **Given** `finder::imaging::fits::encode_cube`, **Then** a committed test asserts that
   a `RealCube` 2 wide, 2 high, of 3 planes, whose samples are 1.0 to 12.0 and whose one
   card is `NSUBS` integer 4, encodes to 5760 bytes; that, read as the shared setup
   says, its cards are `SIMPLE` T, `BITPIX` −32, `NAXIS` 3, `NAXIS1` 2, `NAXIS2` 2,
   `NAXIS3` 3, `NSUBS` 4, `END`; that its data begins `3F 80 00 00 40 00 00 00 40 40 00
   00 40 80 00 00 40 A0 00 00` and its twelfth number is `41 40 00 00`; that the bytes
   after its eighth card, the `END`, up to byte 2880 are spaces; and that the rest of
   the file is zero (worked example: big-endian IEEE 754 singles, plane after plane).
7. **Given** the built `finder` run with `--simulate`, `--state S` and `--listen` on
   port 0 of `127.0.0.1`, **When** a test reads `GET /api/imaging` until `state` is
   `ready`, for at most 20 s, sends `POST /api/imaging/stack` with `{}`,
   reads `GET /api/imaging` every 200 ms until `stack.stacked` is at least 1, for at
   most 60 s, and sends the `DELETE`, **Then** a committed test asserts that
   `S/imaging/runs/` holds exactly one folder, named as `stack.id`, that it holds
   `stack.fits`, `stack.png`, `stack.json` and a folder `subs` with at least
   `stack.stacked` files whose names end `.fits`, that `S/imaging/frames`, if it
   exists, holds no file, and that `S/session.json`, if the console wrote one, does not
   contain the text `"stack"`.

---

### User Story 9 - The stack on the Imaging page (Priority: P2)

As the observer with only the page to hand, on the Imaging page I start the stack, watch
the picture grow with its counts and what was dropped, set the colour gains, and stop
it. At night the picture is red, and shows its colours only while I hold the control
under it.

**Why this priority**: At the telescope the page is often the only thing to hand, and
the picture is the one place the page may show colour in the dark.

**Independent Test**: `npm test` in `crates/finder/web` runs
`src/components/StackBlock.test.tsx`; `CI=1 npm run e2e` runs `e2e/stack.spec.ts` in the
simulate project (owner approval 9). Both are commands of the `test` gate.

**Acceptance Scenarios**:

1. **Given** `StackBlock` rendered with 031's ready view, which has no `stack`, **Then**
   a committed test asserts one region named `Stack` holding a button `Start stacking`,
   the text
   `Takes subs one after another and stacks them into one picture. Turn Track on first.`,
   fields `Red gain`, `Green gain` and `Blue gain` each with the value 1 and `Stretch`
   with the value 10, no `status`, `alert` or picture, and no button `Stop stacking` or
   `Apply colour`. **When** the test clicks
   `Start stacking`, with `fetch` answering 200 and a view, **Then** the committed test
   asserts that `fetch` was called with `/api/imaging/stack` and
   `{"method":"POST","headers":{"Content-Type":"application/json"},"body":"{\"gains\":[1,1,1],\"stretch\":10}"}`,
   then `onChange`; and that a 409 answer with
   `{"error":"this console has no imaging solver"}` shows an alert with exactly that
   text and calls no `onChange`.
2. **Given** `StackBlock` with a `stack` of `state` `stacking`, `taken` 14, `stacked` 9,
   `exposure_s` 90.0 and every count of `dropped` 0, **Then** a committed test asserts a
   status reading exactly `Stacking · 9 of 14 subs stacked · 1 min 30 s of exposure`,
   a button `Stop stacking`, which calls `fetch` with `/api/imaging/stack` and
   `{"method":"DELETE"}` and then `onChange`, a button `Apply colour`, and no button
   `Start stacking`. With `exposure_s` 45.0 the status ends `45 s of exposure`. With
   `exposure_s` 119.9 the status ends `1 min 59 s of exposure`, and with 60.0
   `1 min 0 s of exposure`. With
   `dropped` `{"not_solved":2,"too_few_stars":1,"solver_failed":1,"soft":3,"off_field":1,
   "solve_differs":1,"settings":1,"frame_failed":1,"unreadable":1,"behind":4,
   "unprocessed":0}` the region also holds exactly
   `Dropped 16: 2 not solved, 1 too few stars, 1 solver failed, 3 soft, 1 off the field, 1 solved differently, 1 settings changed, 1 not received, 1 unreadable, 4 behind`.
   With `missing` `["dark","flat","bad_pixels"]` it holds exactly
   `Stacked without a master dark, a master flat, a bad-pixel map.`, and with `missing`
   `[]` no text beginning `Stacked without`.
3. **Given** `StackBlock` with a stacking `stack` whose `stacked` is 9 and whose
   `preview` is `/api/imaging/stack/preview.jpg?n=9`, **Then** a committed test asserts
   a picture named `Stack of 9 subs` with that source and the class `photo`. With
   `night` true it asserts, inside the region, a button `Hold to see colour` with
   `aria-pressed="false"` and the text
   `The picture is red at night. Hold the control to see its colours for up to 10 seconds.`;
   that a `pointerdown` on the button removes the class `photo` and sets
   `aria-pressed="true"`; that a `pointerup`, a `pointerleave`, a `pointercancel` and a
   `blur` each restore both; and that, with fake timers, 10 s after a `pointerdown`
   with no release both are restored. With `night` false there is no such button.
4. **Given** `StackBlock` with a `stack` of `state` `ended`, `id`
   `2026-01-16T02-00-05Z-M42`, `stacked` 9, `taken` 14 and `dropped` with `not_solved`
   2, `behind` 2 and `unprocessed` 1,
   **Then** a committed test asserts, for `ended_reason` `stopped` with `files`
   `{"fits":"runs/2026-01-16T02-00-05Z-M42/stack.fits","png":"runs/2026-01-16T02-00-05Z-M42/stack.png","json":"runs/2026-01-16T02-00-05Z-M42/stack.json"}`,
   a status reading exactly
   `Stack 2026-01-16T02-00-05Z-M42: 9 subs stacked, 5 dropped. Saved runs/2026-01-16T02-00-05Z-M42/stack.fits and stack.png.`,
   a button `Start stacking`, no button `Stop stacking` or `Apply colour`, no text
   `Takes subs one after another and stacks them into one picture. Turn Track on first.`,
   no text beginning `Dropped` and no alert; for
   `ended_reason` `camera` with `error` `the scope node is not connected`, also an
   alert `Stacking stopped: the scope node is not connected.`; with `save_error`
   `could not save the stack: disk full` and no `files`, the status ends at `dropped.`
   and an alert reads `The stack was not saved: could not save the stack: disk full.`;
   and with `files` holding `json` alone, or `fits` and `json` alone, the status ends
   at `dropped.`.
5. **Given** `StackBlock` with a stacking `stack` whose `options` are
   `{"reject":true,"focus_gate":true,"gains":[1,1,1],"stretch":10}`, **When** a test
   sets `Red gain` to 1.5 and `Stretch` to 25 and clicks `Apply colour`, **Then** a
   committed test asserts that `fetch` was called with `/api/imaging/stack` and the
   body `{"gains":[1.5,1,1],"stretch":25}`, then `onChange`; and that a 400 answer with
   `{"error":"gains must be three numbers from 0.1 to 10"}` shows an alert with exactly
   that text; that with `options` gains `[2, 1, 0.5]` and stretch 20 the four fields
   read `2`, `1`, `0.5` and `20`; and that after `Red gain` is set to 1.5, a new view
   with the same options and `taken` 15 leaves it at `1.5`, and a view whose gains are
   `[3, 1, 1]` sets it to `3`.
6. **Given** `ImagingPage` rendered with 031's ready view, to which the test adds the
   `calibration` `{"masters":[]}` so that 034's block is on the page, and with each of
   031's not-ready views, **Then** a committed test asserts one region named `Stack`
   for the ready view, after the region named `Calibration` in document order, and none
   for the others. **Given** `ImagingPage` fed by its hook with fake timers, **When** a test
   clicks `Start stacking`, with `fetch` answering the request 200, **Then** the
   committed test asserts that `/api/imaging` is read again at once, and that when the
   next read's `stack` is `stacking` the region shows its status.
7. **Given** the simulate project's console on a 412 × 915 viewport at a device scale of
   2, **When** a Playwright test opens `/imaging`, clicks `Start stacking` and waits up
   to 30 s for a picture whose name begins `Stack of`, **Then** a committed test
   asserts, counting off-red pixels as `crates/finder/web/e2e/session.spec.ts:45`
   counts them: exactly 0 in a full-page screenshot once Night is on; more than 100 in
   a screenshot of that picture while the mouse is held down on the region's
   `Hold to see colour`, and exactly 0 in a full-page screenshot taken during that hold
   with that picture masked in black; and exactly 0 in a
   full-page screenshot after the mouse is released. It then clicks `Stop stacking` and
   asserts a status beginning `Stack ` within 30 s (oracle: `DESIGN.md`'s Night Ink and
   the Red Torch Rule's one exception).

---

### User Story 10 - Two MCP tools (Priority: P2)

As an MCP client, I start and stop the stack and read it back with its picture in
colour, through the same console actions and refusals as the page.

**Why this priority**: The owner's goal is an ask from an MCP client that ends in a
picture; this is the picture's way out.

**Independent Test**: `crates/finder/tests/mcp/stack.rs`, the module `stack` of finder's
`mcp` test binary. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test mcp stack::`.

**Acceptance Scenarios**:

1. **Given** a console with MCP on, **When** a test calls `tools/list`, **Then** a
   committed test asserts that `set_stacking` and `get_stack` are listed, each with
   `outputSchema` `{"type":"object"}`; that only `get_stack` has
   `annotations.readOnlyHint` true; that `set_stacking`'s description is
   `Start or stop the live stack of subs, as POST and DELETE /api/imaging/stack do. With on true it also takes reject, focus_gate, gains and stretch; on a running stack it changes gains and stretch.`
   and its `inputSchema.properties` has exactly the keys `on`, `reject`, `focus_gate`,
   `gains` and `stretch`; and that `get_stack`'s description is
   `Read the live stack: its state, counts, dropped subs by reason and, when it has one, its picture as a colour JPEG.`
2. **Given** the stand-in stacking rig with MCP on, **Then** a committed test asserts
   that `set_stacking` with `{"on":true}` answers a `structuredContent` whose `state`
   is `stacking`; that, once the stack's `stacked` is at least 2, `get_stack` with `{}`
   answers a `structuredContent` whose `id` is that stack's and whose `stacked` is at
   least 2, and a second content block of type `image` with `mimeType` `image/jpeg`
   whose data decodes to a 64 × 48 JPEG; that the MCP tool `get_session`, called with
   `{}` while that stack runs, answers a `structuredContent` whose `imaging.stack.id`
   is that stack's `id`; that `set_stacking` with
   `{"on":true,"gains":[2,1,1]}` answers `options.gains` `[2.0, 1.0, 1.0]`; and that
   `set_stacking` with `{"on":false}` answers `state` `ended` and `ended_reason`
   `stopped`, with `files.fits` naming a file that exists under R; that with the
   session's target `M42` a stack started by `set_stacking` has an `id` ending `Z-M42`;
   and that on such a rig with a call log, `set_stacking` with `{"on":true}`,
   `get_stack` with `{}` and `set_stacking` with `{"on":false}` leave the log empty
   (neither tool writes a call-log line).
3. **Given** the same rig, **Then** a committed test asserts that `set_stacking` with
   `{}` and with `{"on":"yes"}` answers `isError` true and `on must be true or false`;
   with `{"on":false,"reject":true}`, `isError` true and
   `on false takes no other argument`; with `{"on":true,"gains":[0,1,1]}`, `isError`
   true and `gains must be three numbers from 0.1 to 10`; with
   `{"on":true,"colour":1}`, `isError` true; that `get_stack` with `{"x":1}` answers
   `isError` true and `get_stack takes no arguments`; that `get_stack` before any
   stack answers `structuredContent` `{"state":"none"}` with no image; and that
   `set_stacking` with `{"on":false}` before any stack answers `structuredContent`
   `{"state":"none"}`.
4. **Given** a console with MCP on and no `with_imaging`, as
   `crates/finder/tests/support/mcp.rs:24` builds its state, **Then** a committed test
   asserts that `get_stack` with `{}` answers `structuredContent` `{"state":"none"}`;
   that `set_stacking` with `{}` answers at once (with no virtual time passing) with
   `isError` true and
   `on must be true or false`; that with `{"on":false,"reject":true}` it answers
   `isError` true and `on false takes no other argument`; that with
   `{"on":true,"colour":1}` it answers `isError` true and a text that is not
   `this console takes no imaging frames`; and that with `{"on":true}` and with
   `{"on":false}` it answers `isError` true and `this console takes no imaging frames`
   (worked example: the arguments' shape is judged before the console is asked what
   it has). **Given** the stand-in stacking rig with MCP on, **Then** no result of the
   two tools, called with `{}`, contains `test-token`.
5. **Given** the sim stacking rig with MCP on, written again in
   `crates/finder/tests/mcp/stack.rs` from `SimNode` and `SimulatedSolver` with
   `with_live`, as `crates/finder/tests/mcp/imaging_solve.rs` builds 032's solving rig,
   **When** a stack has a `preview`, **Then** a committed test asserts that
   `get_stack`'s image decodes to 618 × 411 (worked example: 1236 over k = 2).

---

### Edge Cases

- **No master matches.** A sub is still calibrated as far as 034 can, stacked, and the
  view's `missing` names what was absent. The stack never refuses for want of a master.
- **A streak inside the first eight subs.** Nothing is tested against a pixel that holds
  fewer than eight subs, so the streak is added; it is an eighth of its strength then
  and fades as the stack grows (owner decision 6).
- **A pixel few subs reach.** After a turn of the field, corners hold fewer subs. Their
  mean is still a mean, with more noise, and they are tested only once eight subs have
  reached them. A fifth of the area is so affected after 30° of rotation
  (`docs/research/2026-11-15-imaging-camera.md` § 7. Field rotation, sub length and the
  drive).
- **A half turn**, as after a meridian flip on another mount, is a placement like any
  other (US3-S3, US6-S3).
- **The field moves away**, as after a GoTo to another target. Every later sub is
  dropped as `off_field` and counted; the observer stops the stack and starts another.
- **The sky brightens or dims.** Each sub is shifted to the reference sub's background
  before it is tested, so a rising Moon does not make the stack reject everything. A
  change of transparency is not corrected.
- **Cloud.** A sub taken through cloud fails to solve or to measure and is dropped;
  three misses in a row set the stack's own hint aside until a sub solves (US6-S5).
  The stack does not end.
- **A sub with trailed stars.** A trailed sub that still solves is stacked unless the
  focus gate finds it soft. Trailed stars measure larger, so a clear trail fails the
  30% test; a short one passes it, and with `focus_gate` false every one passes. No
  gate on trailing is built (§ Assumptions, Out of scope), though other stackers have
  one (`docs/research/2026-11-15-imaging-camera.md` § 8. Live stacking).
- **No aim point is measured yet.** 032 then gives a live pointing's hint a radius of
  3°, and its place may lie a degree from the sensor's field. The first stacked sub
  whose moment a live pointing shares measures the aim point through `note_solve`: with
  Track off it applies at once, so the place is right from the next sub on; with Track
  on, 032 holds it back until Track is off, and the hint keeps the place it had. A
  sub's solve keeps no scale, so the radius stays 3° either way and each sub's solve
  searches further than it would from the stack's own hint; `took_ms` shows the cost. Solving one imaging frame from the page first
  keeps the scale and measures the aim point (032), and the hint's radius is then the
  sensor's own diagonal. Putting the stack's own hint first instead is owner decision
  26's other answer.
- **No live pointing during a stack**, as with the finder camera covered or a telescope
  without one. `note_solve` then takes no aim reading and the aim point stays as it
  was; the stack is not affected.
- **The observer changes the exposure or the gain mid-stack.** 031 refuses a settings
  change while an imaging frame is in flight, and, under its lease (031 US9), from
  anyone but the holder while the stack holds the camera at all. Should a sub differ from the
  reference sub all the same, because the camera changed something itself, it is
  dropped as `settings`, and so is every later one that differs.
- **Something else wants the camera during a stack.** A single imaging frame, the focus
  aid or a set of darks or flats asked for while a stack runs is refused by 031 with
  the holder's name, `the stack`, and the stack is not disturbed (US5-S6). A stack asked
  for while one of those holds the camera is refused the same way, with that holder's
  name, `the focus aid` or `a set of darks or flats`, even while that holder's own
  frame is exposing, which is almost always (US6-S7). A set of darks, and each step of a
  set of flats, holds the lease under the name `a set of darks or flats` (034 A1), so
  such a set or step is refused at its start while a stack runs, and a stack cannot
  start between two of its frames.
- **The focus region lies outside the sub**, as when the observer has set a region of
  interest. The sub's focus reading is then taken over the whole sub.
- **Someone aborts the exposure.** The stack ends as `stopped`, as if it had been
  stopped.
- **The link drops.** The sub in flight fails and is counted; the next exposure is
  refused because the camera is not ready, and the stack ends as `camera`. What it
  holds is saved.
- **The console exits mid-stack.** Nothing of the stack is saved but the `stack.json`
  of its start; every sub is in the stack's `subs` folder with its solve sidecar (owner
  decisions 13 and 14).
- **The stack's folder is removed mid-stack.** The save does not make it again, so the
  next sub cannot be saved and is counted `frame_failed`, three in a row end the stack
  as `camera`, and its files are not written (`save_error`, US8-S5).
- **Frame ids after a restart.** A sub takes its id from 031's one count, which after a
  restart starts after the largest id in that UTC date's folder under `frames/` and
  does not look in a stack's folder. So an imaging frame taken after a restart may
  carry the id of a sub kept that day in an earlier stack's `subs`; no file is
  replaced, because each stack's `subs` folder is new. 031's id rule is not changed
  (owner decision 13).
- **Midnight UTC during a stack.** Every sub goes into the stack's one folder, whatever
  its date; the count carries on rising.
- **An 8-bit or a mono camera.** Samples become reals as they are; a mono sub gives
  three equal planes, and the files and the picture are the same shapes.
- **A sub at binning 2** is a mosaic of half the size (030) and is stacked as one; its
  imaging solve already gives its own scale.
- **Memory.** At the worked example's size the stack is seven planes of 32-bit reals,
  455 MB, and the sub in hand another 260 MB
  (`docs/research/2026-11-15-imaging-camera.md` § 8. Live stacking). The page's picture
  is made from a reduction of the sums and weights, three planes of 1648 × 1094, about
  22 MB, and never from a mean of the stack's full size. When the stack is saved, after
  the last sub has left, the mean and the bytes of `stack.fits` are 195 MB each
  (inferred from the same figures). Forwarding binned 2 × 2 quarters all of it.
- **`finder --simulate`.** A stack runs on the simulated camera and solver; its
  telescope drifts unless a GoTo and Track are on, so subs walk off the field after
  about a minute.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: `crates/simulator` SHALL export `SatelliteStreak`, `ExtendedSource` and
  `Scene`, and `Simulator` SHALL gain `scene()` and `set_scene(scene)`. With the default
  scene every simulated imaging frame SHALL be byte for byte what it is without this
  spec (owner approval 4).
- **FR-002**: On an exposure whose number, counted from 0 as `frames_taken`
  counts, is in a satellite's `frames`, each sample within ⌈4σ⌉ + 1 pixels of the
  segment from `from` to `to` SHALL gain `electrons·exp(−d²/(2σ²))` electrons, d being
  the distance from the sample's centre to the segment and σ the imaging frame's star
  width, whatever the sample's filter.
- **FR-003**: Each source SHALL add `electrons_per_s·seconds·colour[filter]·
  exp(−d²/(2s²))` electrons to every sample within ⌈4s⌉ + 1 pixels of its place, s being
  `sigma_arcsec` over the optics' arcseconds a pixel and its place the mean of where it
  falls at the exposure's start and end. A streak and a source are light: 034's cover
  and flat panel remove them, its vignetting and dust dim them, and they are added
  before the noise.
- **FR-004**: `stack::colour(mosaic)` SHALL give a picture of the mosaic's own size in
  which a pixel's own colour is its sample and each other colour is the mean of that
  colour's samples among the samples that touch it, inside the mosaic; a mono mosaic, or
  one under 2 pixels wide or high, SHALL give three planes equal to its samples.
- **FR-005**: `Placement::between(reference, sub)` SHALL be the linear map and shift
  that agree with `sub.pixel_of(reference.sky_at(p))` at the reference grid's centre
  and at four points a quarter of its shorter side from it, by central differences; it
  SHALL be `None` when any of the five has no place in the sub, and when, at any of the
  reference grid's four corners (0, 0), (W, 0), (0, H) and (W, H), the sub has no place
  or the map's `apply` lies more than `CORNER_SLACK_PX`, 0.5 px, from
  `sub.pixel_of(reference.sky_at(corner))` (owner decision 3). It SHALL use nothing but
  the two imaging solves.
- **FR-006**: `Placement` SHALL offer `identity`, `apply`, `turn_deg` (the turn of the
  sub from the reference, −180 to 180), `area_scale` (the determinant of `m`) and
  `centre_shift` (where the sub's centre falls on the reference grid, less that grid's
  centre; `None` when `m` cannot be inverted).
- **FR-007**: `Stack::add(sub, placement)` SHALL, for each pixel of the stack's grid,
  take the sample point `placement.apply` of the pixel's centre; count the pixel as
  covered when that point lies among the sub's pixel centres, within 0.000001; and for a
  covered pixel take each channel's bilinear value from the four nearest sub pixels,
  reading none whose share is exactly 0. A covered pixel whose value is not a finite
  number in any channel SHALL be counted as rejected and not added.
- **FR-008**: A `Stack` SHALL hold, as 32-bit reals, three planes of sums and one plane
  of weights, the weight being how many subs were added at that pixel; `mean()` SHALL
  be the sum over the weight, and 0.0 where the weight is 0.
- **FR-009**: `stack::levels(picture)` SHALL be each plane's median over the pixels whose
  x and y are multiples of k, k = max(1, ⌈longer side/512⌉). A stack made
  `with_background_match()` SHALL add to each later sub's values, channel by channel,
  the first sub's level less that sub's own, and report those three numbers as
  `offsets`.
- **FR-010**: A stack made `with_rejection()` SHALL keep, for each pixel and channel,
  the sum of squared deviations of what was added, and SHALL not add a covered pixel
  that already holds 8 or more subs when, in any channel, its value differs from that
  pixel's mean by more than 3 times max(its deviation, 1.0). Such a pixel SHALL change
  no sum and no weight and be counted as rejected.
- **FR-011**: `Stack::new` SHALL make a stack that neither matches backgrounds nor
  rejects; `add` SHALL answer `covered`, `added`, `rejected` and `offsets`.
- **FR-012**: `AppState::with_imaging` SHALL build one `Stacker` over the console's
  `Imaging`, its calibration state and its clock, and `AppState::stack()` SHALL answer
  it; a console built without `with_imaging` has none (owner approval 3).
- **FR-013**: `Stacker::start(request, target)` SHALL refuse, the first that applies:
  `gains` that are not three finite numbers from 0.1 to 10, 400
  `gains must be three numbers from 0.1 to 10`; a `stretch` not finite or outside 1 to
  1000, 400 `stretch must be a number from 1 to 1000`; with a stack running, a request
  naming `reject` or `focus_gate`, 409
  `the stack is running: stop it to change reject or focus_gate`, and any other request
  SHALL set the `gains` and `stretch` it names on that stack and answer its view; a
  camera that is not `ready`, 409 with 031's words for its state; no imaging solver,
  409 `this console has no imaging solver`; nobody holding the camera's lease
  (`Imaging::holder()` `None`) and an imaging frame exposing or receiving, 409
  `an exposure is in progress`, a look that SHALL be passed over while somebody holds
  the lease, whatever the frame's state; the camera's lease not to be had, 031's own
  refusal, a 409 that names the holder. Otherwise it SHALL hold the lease as `the stack`
  and start a stack whose `id` is the console's UTC then as `YYYY-MM-DDTHH-MM-SSZ`,
  followed by `-` and the target's label when one is given, with `reject` and
  `focus_gate` true, gains of 1.0 and a stretch of 10.0 unless the request says
  otherwise, and answer its view. Before it answers, it SHALL make the stack's folder,
  `runs/<id>/` under the imaging folder, `-2`, `-3` and so on added to the `id` when
  the name is taken, and in it the folder `subs`; when either cannot be made it SHALL
  let the lease go, start no stack and answer 500 `cannot make the stack folder: {error}`
  (owner decision 13). It SHALL add no lock of its own over the camera and ask no other
  part whether it is taking imaging frames (owner approval 11).
- **FR-014**: While a stack runs it SHALL ask for one exposure after another under its
  lease, with the purpose label `sub`, each as soon as the last has ended. A sub that
  is saved SHALL wait for the stack; one that failed SHALL be counted as
  `frame_failed`. Every sub SHALL be saved by 031's path, through the lease, in the
  stack's `subs` folder as `<frame id, six digits>.fits`, never under `frames/`, and
  its solve sidecar is written beside it; the folder goes with the lease, so an imaging
  frame taken once the stack has let the lease go is saved under `frames/<UTC date>/`
  as before, and 031's id rule is unchanged (owner decision 13). The stack SHALL
  release the lease once its taker has returned, on every ending, before its files are
  written and before `state` becomes `ended` (owner approval 11).
- **FR-015**: Each waiting sub SHALL be handled in this order, the first step that
  fails deciding its outcome and no later step being run: its view read (a saved sub
  whose view lacks a size, a depth, a binning, a start or a file is `unreadable`); its
  hint (FR-016); its imaging solve through 032's `solve_frame`, which writes its solve
  sidecar; when the stack has a reference sub, `between` of the reference sub's solve
  and its own; its file read and decoded and its mosaic calibrated through 034's
  library; `colour`; `add`. US6 puts three more steps into that order and nowhere
  else: the settings test (FR-021) before the hint, the focus gate (FR-022) right
  after the solve, and `judge` (FR-023) right after `between`. So a sub that is both
  off the field and unreadable on disk is counted `off_field`. The first sub to reach
  `add` SHALL be the reference sub, and the stack's grid its size. The stack SHALL
  match backgrounds always and reject unless `reject` is false. Once a sub is counted
  stacked, the stack SHALL hand its `SolveFrame`, its frame id and its imaging solve to
  032's `Imaging::note_solve`, which takes an aim reading from it when a live pointing
  shares its moment, and SHALL hand no other sub's solve to it; a sub's solve SHALL
  change neither the page's `solve` nor the kept scale (owner decision 20).
- **FR-016**: A sub's hint SHALL come from the plan document's sources in the plan
  document's order. The stack SHALL ask 032's `Imaging::hint` for every sub. When
  that hint has the source `pointing`, a live pointing carried through the aim point,
  it SHALL be used as it is. Otherwise, once the stack has a reference sub, the hint
  SHALL be the stack's own, standing where 032's `last_solve`, `target` or `none`
  would: the centre of the stack's last solved sub with the source `last_solve`, a
  radius of the sub's diagonal on the sky and the sub's height on the sky, both from
  that solve's scale. Until the stack has a reference sub, 032's hint SHALL be used
  whatever its source (owner decision 26).
- **FR-017**: The view SHALL count `taken` (subs saved or failed), `stacked`, `queued`
  and, in `dropped`, eleven reasons, every one always present: a sub whose solve ended
  `NotSolved` is `not_solved`; `TooFewStars`, `too_few_stars`; any other solver error,
  `solver_failed`; a sub whose file cannot be read or is not an image, `unreadable`; a
  sub still waiting when the stack ends, `unprocessed`; and `frame_failed`. When a stack
  has ended, `taken` SHALL equal `stacked` plus the sum of `dropped`. The view SHALL
  also carry `exposure_s`, `reference`, `last` (with the real time the sub took,
  `took_ms`) and `missing`, what 034 found absent for the last sub calibrated.
- **FR-018**: A stack SHALL end as `stopped` on `Stacker::stop()` and when an exposure
  ends aborted, and as `camera`, with the words as `error`, when three subs in a row
  fail or an exposure is refused. `stop()` SHALL abort the exposure in
  flight, abandon a solve or a measure in flight, and answer once the stack has ended;
  nothing SHALL change in an ended stack's counts.
- **FR-019**: No file under `crates/finder/src/imaging/stack.rs` or
  `crates/finder/src/imaging/stack/` SHALL read an axis position, the site or the
  mount's state, compute an altitude, an azimuth or a field rotation, or hold a
  constant sized to one focal length or one sensor; no limit there SHALL be a number of
  seconds or of degrees on the sky. The stack reads the console's clock only for its
  own timestamps, and the settings test's millisecond is a tolerance for calling two
  exposure settings equal, not a limit on time.
- **FR-020**: `GLOSSARY.md` SHALL define **Reference sub** with the words of owner
  approval 6.
- **FR-021**: `stack::same_settings(reference, sub)` SHALL be true only when the two
  `SubSettings` have equal `width`, `height`, `bits`, Bayer pattern, `binning` and
  region, a `gain` and an `offset` that are each absent in both or present in both and
  within 0.001, and an `exposure_ms` within 1. A sub for which it is false against the
  reference sub SHALL be dropped as `settings` before it is solved. The comparison is
  always with the reference sub, never with the sub before it.
- **FR-022**: Unless `focus_gate` is false, each solved sub SHALL be given a focus
  reading through 033's `focus::read`: in the focus region the imaging view names when
  that region lies inside the sub, over the whole sub otherwise, with the sub's own
  solve as the scale, so that the reading is the half-flux diameter times the solve's
  arcseconds a pixel. With 5 or more readings of stacked subs known, a sub whose
  reading is more than 1.3 times their median SHALL be dropped as `soft`, and so SHALL
  a sub 033 calls far out, however few are known. A sub with too few stars to measure
  SHALL be dropped as `too_few_stars`, and one whose measure fails as `solver_failed`.
  The view SHALL carry the median as `focus_median_arcsec` and the sub's reading in
  `last` (owner decision 8).
- **FR-023**: `stack::judge(placement, reference size, sub size)` SHALL answer
  `Err("solve_differs")` when `area_scale` is not finite or is outside 1 ± 0.02 or
  `centre_shift` is `None`, and otherwise `Err("off_field")` when the shift is longer
  than half the reference's shorter side; a sub so judged, or whose `between` is
  `None` (`off_field`), SHALL be dropped and counted under that reason.
- **FR-024**: At most 4 saved subs SHALL wait for the stack. A sub saved while 4 wait
  SHALL push out the one that has waited longest, counted as `behind`. The exposures of
  FR-014 SHALL never wait for a sub to be handled.
- **FR-025**: The stack SHALL count misses in a row: a sub whose imaging solve ends
  `NotSolved` or `TooFewStars` adds one; a sub whose imaging solve succeeds sets the
  count to 0; and nothing else changes it, neither another solver error, nor a sub
  dropped before its solve, nor a `too_few_stars` from the focus gate of a sub that
  solved. While the count is 3 or more, the stack's own hint of FR-016 SHALL be set
  aside and 032's `Imaging::hint` used as it is, until a sub solves.
- **FR-026**: `display::reduce(sum, weight, width, height, max_width)` SHALL take a
  stack's three planes of sums and its plane of weights and, with k =
  ⌈width/max_width⌉, make each whole k × k block one pixel: in each channel the
  block's sums added together over its weights added together, covered when those
  weights come to more than 0, and 0.0 and not covered otherwise. It SHALL drop rows
  and columns that do not fill a block.
- **FR-027**: `display::background(reduced)` SHALL give, for each channel, the
  least-squares plane through the centres and medians of those tiles of an 8 × 8 grid
  that are at least half covered and no brighter than the median tile; and a flat
  plane at their median when they are fewer than 3 or lie on one line, at the channel's
  median when there is no tile, and at 0.0 when nothing is covered.
- **FR-028**: `display::tone(reduced, planes, gains, stretch)` SHALL make each covered
  value v = gain·(value − plane), take for each channel its median m and noise n (m
  less the 16th percentile), take one span, the largest over the channels of the 99.9th
  percentile less m and of 30·n, at least 1.0, and write
  `round(255·asinh(stretch·t)/asinh(stretch))` with t = clamp((v − m)/span, 0, 1). A
  pixel not covered SHALL be black.
- **FR-029**: After a sub is stacked, when no sub waits or 8 have been stacked since the
  picture was last made, and at once when a request on a running stack names `gains`
  or `stretch`, the stack SHALL make its picture: `reduce` of its sums and weights to
  at most 1920 wide, then `background`, `tone` and a JPEG through 031's
  `preview::jpeg`. For that picture it SHALL make no mean of the stack's full size,
  SHALL hold the lock over the stack's pixels for the reduction alone, and SHALL make
  one picture at a time. It SHALL keep the picture and name it in the view as
  `/api/imaging/stack/preview.jpg?n=` and the count of pictures this stack has made, so
  that the name changes with every picture. An ended stack SHALL keep its last picture
  until the next start.
- **FR-030**: `display::shrink(picture, max_width)` SHALL reduce an 8-bit picture by
  whole blocks as FR-026 does, and `Stacker::preview_within(max_width)` SHALL answer
  the kept picture so shrunk, as a JPEG.
- **FR-031**: `POST /api/imaging/stack` SHALL take a JSON object with only the optional
  keys `reject` and `focus_gate` (each true or false), `gains` (three numbers) and
  `stretch` (a number), under the same-origin rule, and answer 200 with the imaging
  view. The empty object `{}` is such a body: it starts a stack with the defaults,
  and on a running stack changes nothing and answers the view. A request SHALL be
  judged in this order: 403 for a foreign origin; 415 for another content type, and
  400 for any other body; 409 `this console takes no imaging frames` without imaging;
  FR-013's refusals in FR-013's order; and 500 `cannot make the stack folder: {error}`
  when FR-013's folder cannot be made, or FR-035's `stack.json` cannot be written there
  at the start (owner approval 1).
- **FR-032**: `DELETE /api/imaging/stack` SHALL, under the same-origin rule, stop the
  stack that is running and answer 200 with the imaging view once it has ended and its
  files are written; with none running it SHALL change nothing and answer the view.
- **FR-033**: `GET /api/imaging/stack/preview.jpg` SHALL answer the kept picture as
  `image/jpeg`, whatever the query, or 404 `the console holds no stack picture`.
- **FR-034**: `GET /api/imaging`, and `imaging` in `GET /api/session`, SHALL carry
  `stack`, the view of FR-017, from the first accepted start until the console exits,
  whatever the camera's state, and omit it before, as SHALL every console built
  without imaging. Nothing of a stack SHALL be kept in `session.json`.
- **FR-035**: A stack's folder SHALL be the one FR-013 makes, `runs/<id>/` under the
  imaging folder, holding `subs` with the stack's subs (FR-014). `stack.json` SHALL be
  written there at the start, the lease let go and the 500 of FR-031 answered when it
  cannot be, and rewritten at the end, with `format` `finder-stack/1`, the console's
  version, `id`, `started_at`, `target`, `options` and the camera's model and serial,
  and at the end `ended_at`, `ended_reason`, `error`, the counts, `reference` with its
  `cd_deg` and file, and `subs`: every sub taken, in order, with its `frame_id`, its
  `file`, its name in the stack's folder, `subs/<frame id, six digits>.fits` (null for
  one that never arrived), and its `outcome`, `stacked` or the reason it was dropped.
  A dropped sub SHALL stay in `subs` and be listed like a stacked one; `reference`'s
  file SHALL be named the same way (owner approval 5, owner decision 13). The target's
  label in an `id` SHALL be `stack::service::label` of the session target's Messier
  name when it has one and of its id otherwise; `label` SHALL make every character of a
  name that is not an ASCII letter or digit one `-` and keep every other, and it is the
  one copy of that rule, which 037 calls too (owner approval 3).
- **FR-036**: When a stack ends holding at least one sub, `stack.fits` SHALL be written:
  `BITPIX` −32, three axes, the mean's red, green and blue planes, each row by row from
  the top, with the cards `IMAGETYP` `STACK`, `STACKID`, `NSUBS`, `EXPTIME` (the
  stacked seconds), `SUBEXP`, `DATE-OBS` (the reference sub's start), `INSTRUME`,
  `XBINNING`, `YBINNING`, `ROWORDER` `TOP-DOWN`, and the reference solve as `CTYPE1`
  `RA---TAN`, `CTYPE2` `DEC--TAN`, `CRPIX1` and `CRPIX2` (the grid's centre plus 0.5),
  `CRVAL1`, `CRVAL2` and the four `CD` cards. `fits::encode_cube` SHALL write such a
  file.
- **FR-037**: With it, `stack.png` SHALL be written: `display::render` of the stack,
  the picture of FR-026 to FR-028, at the stack's full size, with the stack's gains
  and stretch, 8 bits a channel.
- **FR-038**: Each file SHALL be written under its name plus `.part` and renamed. A
  failure SHALL leave the stack `ended` with its reason, no `files`, and `save_error`
  `could not save the stack: {error}`. The view's `files` SHALL name the files written.
  No sub and no solve sidecar SHALL be moved, copied or rewritten.
- **FR-039**: `crates/finder/README.md` SHALL gain three rows of its API table, and the
  route list of `crates/finder/src/lib.rs` three lines, as plan.md § Interfaces words
  them; and US10 SHALL add `## Live stack` at the end of the README, as plan.md
  § Interfaces words it (owner approval 8).
- **FR-040**: The `finder` binary SHALL offer the stack wherever it offers imaging
  frames and an imaging solver, `--simulate` included, with no new flag.
- **FR-041**: The Imaging page SHALL show a region named Stack with, by the stack's
  state, the words, buttons, counts, dropped reasons and missing masters of plan.md
  § The page, and a refused request's words in an alert.
- **FR-042**: The stack's picture SHALL be an `img` named `Stack of {stacked} subs` with
  the class `photo`, and at night only SHALL have under it the `Hold to see colour`
  control of 031's exception: colour while held, 10 seconds at most, held through
  031's `useColourHold` (031 A6), with no timer or listener of the block's own.
- **FR-043**: The region SHALL hold fields for the three gains and the stretch, sent
  with `Start stacking` and, on a running stack, by `Apply colour`.
- **FR-044**: `ImagingPage` SHALL render the region for a ready view, after the
  Calibration region, and read `GET /api/imaging` again at once after a request of the
  region succeeds; `api.ts` SHALL carry the stack's types and its two calls.
- **FR-045**: `DESIGN.md` SHALL describe the Stack block, `.impeccable/design.json`
  SHALL list it, and `crates/finder/web/dist` SHALL be the build of its source (owner
  approval 7).
- **FR-046**: `crates/finder/web/e2e/stack.spec.ts` SHALL check, in the simulate
  project, that the page with a stack's picture has no off-red pixel at night but in
  that picture while its control is held; `playwright.config.ts` SHALL list the file as
  owner approval 9 says.
- **FR-047**: The MCP tool `set_stacking` SHALL take `on` and, with `on` true, the keys
  of FR-031. It SHALL judge its arguments in this order, each an error: `on` absent or
  not true or false, `on must be true or false`, which is its answer to `{}`; `on`
  false with any other key, `on false takes no other argument`; `on` true with a key
  that is not FR-031's or a value that is not of FR-031's type, the parser's words; a
  console without imaging, `this console takes no imaging frames`. Then, with `on`
  true, it SHALL start or change a stack as FR-013 does and answer its view, its
  refusals being FR-013's words; with `on` false it SHALL stop the stack as FR-032
  does and answer its view, or `{"state":"none"}` when there is none (owner approval
  2).
- **FR-048**: The MCP tool `get_stack`, read-only, SHALL answer the stack's view, or
  `{"state":"none"}`, and, when a picture is kept, that picture at most 1024 wide as a
  colour JPEG in a second content block.
- **FR-049**: Neither tool SHALL be a moving tool or put the scope node's token in a
  result; each SHALL answer a call with `{}` at once on a console without imaging; the
  MCP tools that return the session SHALL carry `stack` in its `imaging`, as FR-034's
  routes do; and `crates/finder/README.md` SHALL list both under `## MCP clients`.

### Key Entities

- **Sub**: an imaging frame taken for a stack (`GLOSSARY.md:165`), kept in its stack's
  `subs` folder.
- **Reference sub**: the first sub a stack accepts; its grid is the stack's.
- **Colour picture**: three planes of 32-bit reals the size of a sub.
- **Placement**: the linear map and shift from the reference grid into a sub, read from
  two imaging solves at five points and checked against them at the grid's corners.
- **Stack**: sums, weights and squared deviations for every pixel of the reference grid
  (`GLOSSARY.md:193`).
- **Dropped sub**: a sub the stack did not add, counted under one of eleven reasons.
- **Display picture**: the stack reduced, flattened, balanced and stretched to 8 bits.
- **Scene**: the simulator's satellites and glows, for tests.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario has a committed test that asserts it, and the
  `lint` and `test` gates pass.
- **SC-002**: No existing test, golden, `scripts/ci/` file, `.github/` file, `CODEOWNERS`
  entry, `ergane.yaml` or `.specify/` file is edited. The only edits to existing test
  and gate files are the `mod` lines and the two list entries of
  `playwright.config.ts` that owner approval 9 names.
- **SC-003**: Against the simulator's truth, a star's centroid in a stack of three subs
  at three attitudes, a half turn among them, is within 0.02 px of its centroid in the
  reference sub (US3-S4).
- **SC-004**: With four times the subs a faint source's signal over noise doubles,
  within 15% (US3-S5).
- **SC-005**: A streak 30 times the noise that crosses one sub of twelve changes the
  stack along its line by less than half a count (US4-S4).
- **SC-006**: No file of the stack reads the mount or the site, or works out a place
  on the sky from the time: no sidereal time, altitude or azimuth. The stack reads the
  console's clock only for its own timestamps (FR-019). And a stack runs from start to
  saved files on the simulated camera and solver with no mount moving (US8-S7).
- **SC-007**: During a stack with a live pointing, the subs keep the aim point measured:
  after five stacked subs it is within 1.5 finder pixels of the simulator's truth,
  from five aim readings, while the page's `solve` and the hint's radius are what they
  were before the stack (US6-S9).

## Assumptions

- **030, 031, 032, 033 and 034 have landed**: `030-imaging-camera-link`,
  `031-imaging-frames-on-console`, `032-imaging-solve-and-aim-point`,
  `033-focus-reading-and-aid` and `034-dark-and-flat-frames` are on `main` at 002dbef8,
  with the interfaces their trios gave, 031 with the camera's lease of the plan
  document's amendment 1 (031 US9) and `useColourHold` (031 A6), 032 with
  `Imaging::note_solve` (032 A3), and 034 taking that lease for a set of darks or flats
  (034 A1), the four things the series audit's findings on those trios added; this trio
  was re-anchored to their code on 2026-11-19 (plan.md § Prerequisites 1, 2, 5, 8, 13
  and 14). Every `path:line` here was checked at 49bb210, the drafting worktree's HEAD,
  and read again by its content on `main` at 002dbef8, whose line numbers are cited,
  the lines of `GLOSSARY.md` among them; every file 030, 031, 032, 033 or 034 created is
  named without a line.
- **The plan document's `GLOSSARY.md` rewrite is on `main`**, since 030's flip
  (`55e41f5`): US5 inserts its entry after one only the rewrite has, the **Stack** entry
  (plan.md § Prerequisites 7).
- **Out of scope:**
  - a refinement of a sub's place on star positions, and any registration that does not
    come from the imaging solves;
  - tracking from the subs, the longest exposure the tracker allows, and pausing while
    it is not steady (036; plan.md § Shapes that existing tests pin names where it adds
    them);
  - an exposure that changes within a stack: a sub whose exposure differs from the
    reference sub's is dropped (FR-021), so 036 decides the exposure once, before the
    first sub;
  - a gate on trailed stars, beyond what the focus gate catches;
  - the imaging run: GoTo, the focus check, centring, a keep-subs switch, `run.json`
    and the run's endings (037);
  - saving a stack before it ends, resuming one, or stacking subs already on disk;
  - a canvas larger than the reference sub, mosaics, drizzle and any resampling but
    bilinear;
  - a per-channel shift for atmospheric dispersion, and a correction for transparency;
  - colour gains measured from the stars, a colour matrix, and any setting kept across
    restarts;
  - a weight for a sub by its quality, and dropping a sub for anything but FR-017,
    FR-021 to FR-024;
  - a plane of weights or of rejections in the files, and a calibrated sub on disk;
  - planets and the Moon;
  - the camera's lease itself, which is 031's (031 US9), and any other lock over the
    camera (owner decision 21);
  - a sub's solve as the page's `solve`, a scale kept from a sub's solve, and an aim
    reading from a sub that was not stacked (owner decision 20);
  - the exact mapping between two tangent planes for every pixel (owner decision 3);
  - a judgement of a sub against 033's focus record, and the focus check of an imaging
    run (037);
  - more than one thread for the pixel pass (owner decision 25);
  - any change to the scope node, the scope link, a `Cargo.toml` or the constitution.
- **Outside the factory:** rebuilding the console; the field checks at the telescope: a
  stack of a bright cluster and of the Orion Nebula; a satellite's trail rejected; the
  corners after half an hour of field rotation; how well two of ASTAP's solves of one
  field agree, read from stars in consecutive subs; `took_ms` at the camera's full
  resolution and binned 2 × 2, against the sub length; whether ASTAP's star measure
  works on a raw colour mosaic, what the focus gate drops on a real night, and whether
  trailed subs get past it; how long a sub's solve takes from a live pointing's hint
  before the aim point is measured, and whether the aim point the subs measure stays
  steady through a stack; the
  colour gains that make the background and the stars look right, with and without a
  UV/IR-cut filter; a trial of `seiza-stacking` or of AstraLite's stacking modules on
  real subs, if the owner wants one (plan.md § Prerequisites 6); and a design review of
  the block on a phone in the dark.
- **The factory's gates and CI:** the gates are `lint` and `test`, copied in plan.md
  § Verification. CI's other jobs run only in the merge queue, so no scenario's Then
  clause rests on them.

## Work Graph

```yaml
US1:
  depends_on: []
  implements: [FR-001, FR-002, FR-003]
US2:
  depends_on: []
  depends_on_merged: [US1]
  implements: [FR-004, FR-005, FR-006]
US3:
  depends_on: []
  depends_on_merged: [US2]
  implements: [FR-007, FR-008]
US4:
  depends_on: []
  depends_on_merged: [US3]
  implements: [FR-009, FR-010, FR-011]
US5:
  depends_on: []
  depends_on_merged: [US2, US4]
  implements: [FR-012, FR-013, FR-014, FR-015, FR-016, FR-017, FR-018, FR-019, FR-020]
US6:
  depends_on: []
  depends_on_merged: [US5]
  implements: [FR-021, FR-022, FR-023, FR-024, FR-025]
US7:
  depends_on: []
  depends_on_merged: [US6]
  implements: [FR-026, FR-027, FR-028, FR-029, FR-030]
US8:
  depends_on: []
  depends_on_merged: [US5, US7]
  implements: [FR-031, FR-032, FR-033, FR-034, FR-035, FR-036, FR-037, FR-038, FR-039, FR-040]
US9:
  depends_on: []
  depends_on_merged: [US8]
  implements: [FR-041, FR-042, FR-043, FR-044, FR-045, FR-046]
US10:
  depends_on: []
  depends_on_merged: [US8, US9]
  implements: [FR-047, FR-048, FR-049]
```

Every edge is a merge edge, and the stories run one after another. US2's tests need
nothing of US1, but US3 and US4 take subs from the scene US1 adds, and one chain keeps
two stories from writing `crates/finder/tests/console/main.rs` at once. US3 adds to the
file US2 creates, and US4 to US3's `Stack`. US5 calls all of it from the console's loop;
US5 also names US2, because it is the next story after US2 to write
`crates/finder/src/imaging/mod.rs`, where it lets the lease save a stack's subs in the
stack's folder. US6 adds its gate and replaces its queue; US7 gives the loop a picture
to keep; US8 puts routes, the view and the files over the loop, and reads US7's
picture. US8 also names US5, because it is the next story after US5 to write
`crates/finder/src/imaging/mod.rs` and `crates/finder/src/api.rs`.
US9 calls US8's routes and shows its view. US10 follows US9 so that the chain stays one
line, and names US8 because it is the next story to write `crates/finder/README.md`.
