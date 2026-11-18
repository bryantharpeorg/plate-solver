---
state: ready
depends_on_landed: ["030-imaging-camera-link", "031-imaging-frames-on-console", "032-imaging-solve-and-aim-point"]
# Drafted 2026-11-15 against `main` at 49bb210 (the drafting worktree's HEAD,
# 49bb210fe4860dd98ba24727074e7bd3cc0d7124) from
# docs/plans/2026-11-15-specs-030-037-imaging.md, by the grilling session of that day.
# A first draft was written at dbc2b5b. A second pass re-read every `path:line` at
# 49bb210, took every interface of 030, 031 and 032 from their reviewed plan.md, and brought
# the trio in line with the plan document's "Mount-neutral by design" and its
# "Amendments after 030, 031, 032 and 034 were drafted and reviewed" (§ Clarifications, the
# third session). A review of the trio by a second agent was applied the same day
# (§ Clarifications, the fourth session), and then the series audit of 030 to 037
# (§ Clarifications, the fifth session): the imaging camera's lease, which User Story 5
# is written against, is cited from 031's User Story 9, which the same audit gave
# 031's trio (plan.md § The camera lease, § Prerequisites 11).
# Citations checked against 49bb210fe4860dd98ba24727074e7bd3cc0d7124, the worktree's
# HEAD, on 2026-11-15 in the fifth session: every `path:line` in spec.md and plan.md
# was read there again, and none had moved.
# Probe-built on 2026-11-18 (round 1, every story), on top of 031's and 032's round-1
# probe builds: the round-1 probe fixes are in, and User Story 5, which measured over
# the budget, is cut as the owner chose: User Story 9 takes the tile read for the time
# being and the frame kept for a solve (plan.md § Risks, § Refinement Record).
# It builds on code that 030-imaging-camera-link, 031-imaging-frames-on-console and
# 032-imaging-solve-and-aim-point created; every interface of theirs named here was taken
# from that trio's plan.md, and the trio was re-anchored to `main` after all three landed
# and before the flip (plan.md § Prerequisites).
# Re-anchored 2026-11-18 to `main` at 670ddcf (670ddcf9d286ef415c6eba73b65a897460f4deed),
# where 030's eight stories, 031's nine and 032's seven have landed: every citation was
# compared line for line with 49bb210 and read again by its content, and the ones that
# moved are corrected; every name and behaviour taken from 030, 031 and 032 was read in
# their code, and the few that differed in name, path or place are corrected; the answers
# of 031's take, settings edit and abort and of 032's solve are `Imaging::view`'s own on
# `main`, so they carry `focus` too; and the three glossary rewordings of owner approval
# 11 are written into `GLOSSARY.md` beside this trio (plan.md § Prerequisites 2 and 6).
# The owner approved the imaging series' recommended answers on 2026-11-17.
# Probe-built again on 2026-11-18 (round 2) on `main` at 670ddcf: US5 as cut and US9
# from this trio, nothing blocking; US5 measured 763 bytes under the budget, and the
# owner kept it so. The owner's answers that day are in: a take answered with no frame
# ends the run `failed`, and US9-S1 pins FR-037's and FR-029's two untested rules
# (§ Clarifications, the session of the re-anchoring).
# Flipped to ready 2026-11-18 by the operator seat on the owner's answer the same day ("Yes,
# flip after the fold", once the round-2 probe and the owner's answers were folded in),
# which gives the owner approvals listed under "Owner approvals". Every owner decision
# takes its recommended answer, which the owner approved on 2026-11-17 with the imaging
# series' recommendations.
---

# Feature Specification: Focus Reading and Focus Aid

**Feature Branch**: `033-focus-reading-and-aid`

**Created**: 2026-11-15

**Status**: Draft

**Input**: The imaging camera sits at a focuser the observer turns by hand, and an
imaging frame that is out of focus does not solve. This spec lets the console say how
sharp the stars are and help the observer make them sharper. A focus reading is the
median size of the stars in the focus region of an imaging frame, as a half-flux diameter
in arcseconds (in sensor pixels until the console knows the scale), asked of the imaging
solver through the solver port's star measure. The focus region is the tile of a full
imaging frame with the smallest reading, found from the frame and never assumed to be the
centre, or a tile the observer fixes by hand. The console keeps a focus record for the
camera, tonight's and earlier nights', and judges a reading against it: good, slightly
soft, clearly out, unmeasurable, or no record yet. It never says focus is good in
absolute terms. The focus aid takes short exposures of the focus region one after another
and shows, for each, the reading, the best so far, the trend since the focuser last
moved, and whether the reading has passed through its lowest and come back. When the
stars are too far out of focus to choose a region by, it runs on a tile for the time
being, shows their rough size, and chooses the region once they can be measured. A focus
check takes up to nine imaging frames of the focus region and stops by itself with a
verdict, for the imaging run of spec 037. While either runs it holds the imaging
camera's lease, so nothing else takes an exposure or changes the camera's settings under
it. The Imaging page gets a Focus block, MCP clients two tools, and the simulator a
focuser to turn.

## Clarifications

### Session 2026-11-15 (the interview's decisions, from the plan document)

- Q: What does this trio read besides itself? → A: Nothing about the feature. `spec.md`
  holds every requirement and scenario, and `plan.md` holds every design decision, the
  interfaces, the shapes that existing tests pin and the alternatives rejected. The
  binding decisions come from `docs/plans/2026-11-15-specs-030-037-imaging.md`,
  `docs/adr/0002-imaging-cameras-through-indi-frames-relayed-console-stacks.md` and
  `GLOSSARY.md`; facts come from `docs/research/2026-11-15-imaging-camera.md`. A story
  that would change a decision escalates rather than re-deciding it.
- Q: What is a focus reading, and who measures it? → A: The median half-flux diameter of
  the stars in the focus region of one imaging frame, in arcseconds; smaller is sharper.
  Until the console knows the scale it is shown in sensor pixels: this trio reads the
  plan document's "Settled decisions" row for Focus, which stands as written, as
  arcseconds whenever a scale is known (owner decision 6, and the list below).
  The imaging solver measures it, through the solver port's `measure` (032): ASTAP at the
  telescope, the simulated solver adapter in tests and under `finder --simulate`. This
  spec writes no star detection of its own.
- Q: How is a reading judged? → A: Against the focus record, never against a fixed
  number: good within 10% of it, slightly soft when 10% to 30% larger, clearly out
  beyond that or when it cannot be measured. Which part of the record judges, and what
  a reading is called when there is no record at all, the plan document does not say:
  owner decisions 8 and 23. What the imaging run does with each verdict
  (warn and carry on, or pause and open the focus aid, with the observer always able to
  say go anyway) is 037's.
- Q: Why is no absolute "focus is good" claimed? → A: Because the data cannot support
  one. A reading cannot tell a focus error from seeing, a mirror still cooling, tube
  currents or miscollimation: each enlarges stars the same way. "Best seen so far, with
  a rise on both sides" is the only verdict the data supports
  (`docs/research/2026-11-15-imaging-camera.md` § 6. Focus). The page says so to the
  observer in its own words (FR-032).
- Q: Why a focus region, and why is it found rather than assumed? → A: Without a
  corrector a fast Newtonian's stars grow with distance from the optical axis, and the
  axis lands where the collimation puts it, not at the sensor's centre: on the worked
  example 1 to 3 mm of miscollimation moves the sharpest part of the field by 210 to
  630 pixels (the same section). So the console measures tiles of a full imaging frame
  and keeps the one with the smallest reading.
- Q: Why does the focus aid read a region of interest and not a binned full frame? → A:
  Binning 2 × 2 is done in software after the download on the worked example's camera
  and shortens nothing on the cable; only a smaller region does
  (`docs/research/2026-11-15-imaging-camera.md` § 1. The camera and § 11. Frame
  transport). A binned frame also halves the sampling of stars that are a few pixels
  across.
- Q: What is binding from "Mount-neutral by design"? → A: No file of this spec reads the
  mount, an axis position, an altitude or an azimuth; the focus aid does not start, stop
  or check tracking. Nothing is sized to one telescope: the tiles are thirds of the
  sensor, every threshold is a fraction of a measured value, and the one fixed size is
  20 frame pixels: ASTAP's limit on a mono frame, and half of it on a colour mosaic
  that the adapter averages 2 × 2 (owner decision 5). Nothing is counted in seconds that should be
  counted in imaging frames: the pause after a change is two imaging frames, the best so
  far is kept over the last 600 settled readings, a focus check is at most nine imaging
  frames. Two figures are times, and neither is a wait for a sub: the focus exposure,
  which is a setting (1000 ms unless asked otherwise), and the length of a night for the
  focus record, 12 hours.
- Q: Is anything keyed to one camera? → A: No. Sizes, binnings and the exposure range
  are read from the camera's description (030). The owner's camera, 4944 × 3284 pixels
  of 4.78 µm at 1800 mm, appears only as a worked example.
- Q: The owner plans a second telescope, a refractor on an equatorial mount. What of
  this spec carries over? → A: All of it, unchanged (the plan document's "Another
  telescope later" row). Nothing here knows the mount. The tiles are thirds of whatever
  sensor the camera describes, every threshold is a fraction of a measured value, and
  sizes are shown with whatever scale the console has. The focus record is kept with
  the scale an imaging solve measured, so a record made on one telescope does not
  judge a reading made on another once each has been solved on (owner decision 10).
  The one thing a change of telescope leaves stale is the kept focus region, which the
  observer has found again (Edge Cases).
- Q: Does the plan document's "Colour" row reach this spec? → A: Only as a caveat. A
  focus reading is taken from the samples as the camera delivered them: the imaging
  solver measures a copy of its own making (032), and no colour step of this project
  is involved. Whether a UV/IR-cut filter is fitted is not known. With mirrors only,
  all colours focus together, so the answer does not move a reading on the worked
  example; a refracting corrector adds a small colour shift
  (`docs/research/2026-11-15-imaging-camera.md` § 10. Exposing the Orion Nebula, and
  colour). That is one more reason the verdict is against this camera's own record on
  this telescope and never absolute.

### Session 2026-11-15 (review of `main`, re-read at 49bb210)

- Q: What was found in the code? → A: Eight things, each checked at 49bb210 and read
  again on `main` at 670ddcf, whose line numbers are cited:
  1. The simulator's desk controls are routes registered only when the console was built
     with a simulation (`crates/finder/src/api.rs:551`), behind the same-origin check,
     reading their bodies through `sim_json` (`crates/finder/src/api.rs:652`). Outside
     simulate mode every `/api/sim/*` path answers 404
     (`crates/finder/src/api.rs:545`), and the README lists them in one table
     (`crates/finder/README.md:434`).
  2. A test builds a simulated console in process
     (`crates/finder/tests/console/live/whole_loop.rs:12`) and reaches its simulator
     through `AppState::simulator` (`crates/finder/src/api.rs:223`). So the simulator's
     new control is tested in the `console` binary, without the built `finder`.
  3. The simulator's random numbers are a type private to its crate
     (`crates/simulator/src/noise.rs:4`), and that file's own test asserts the published
     SplitMix64 stream (`crates/simulator/src/noise.rs:99`). 030 makes one of its
     methods `pub`; the type stays private. The simulated solver adapter lives in
     `finder`, so its seeded noise is written there, from the same published stream.
  4. The simulator's telescope state is one value behind a mutex
     (`crates/simulator/src/optics.rs:66`), where 030 puts the imaging optics with their
     focus error.
  5. One MCP test calls every listed tool with `{}` and checks the result for the
     pairing token (`crates/finder/tests/mcp/motion.rs:517`). `set_tracking`, whose `on`
     is required by its parameter type (`crates/finder/src/mcp/tools.rs:346`), is given
     `{"on":false}` there by name (`crates/finder/tests/mcp/motion.rs:537`). A new tool
     with a required key therefore parses its own arguments and answers `{}` with a tool
     error, at once.
  6. The page's Display size is written out on the element
     (`crates/finder/web/src/components/ChartHeader.tsx:96`), and `DESIGN.md` reserves it
     for RA and Dec (`DESIGN.md:213`).
  7. Finder's tests are modules of five binaries; a file at the top level of
     `crates/finder/tests` fails the `layout` test
     (`crates/finder/tests/layout/main.rs:109`), as does a new file in its support
     folder (`crates/finder/tests/layout/main.rs:126`). A test's scratch folder comes
     from `Scratch` (`crates/finder/tests/support/mod.rs:12`).
  8. The console's clock is a trait (`crates/finder/src/session.rs:24`), so every time
     this spec stores comes from outside.
- Q: What does this spec take from 030, 031 and 032? → A: These, by name, taken from their
  plan.md and, now that all three have landed, read in their code on `main` at 670ddcf
  (plan.md § Prerequisites 2); each is named without a line:
  - `crates/finder/src/imaging/mod.rs` (031 US1): `Imaging`, `ImagingView`,
    `SettingsView`, `SettingsEdit` with its `RoiEdit` (`x`, `y`, `width` and `height`
    in sensor pixels, as 031's plan.md defines it; quoted in plan.md § The camera
    lease), `CameraView`, `RoiView`,
    `Refusal`, `Imaging::view`, `Imaging::set`, `Imaging::take`, `Imaging::abort` and
    `Imaging::ended`, and the words a request is refused with while the camera is not
    `ready` or an exposure is in progress;
  - `crates/finder/src/imaging/frames.rs` (031 US2 and US5): `FrameView`, the purpose
    label, and a saved frame's `id`, `file`, `width`, `height`, `bits`, `bayer`,
    `binning`, `roi`, `start` and `exposure_ms`;
  - `crates/finder/src/imaging/routes.rs` (031 US1), where the two routes go, with
    `SessionWrite` and `sim_json`, which 031 makes visible to it;
  - `crates/finder/src/imaging/solver.rs` (032 US1): `Region`, `SolveFrame`,
    `StarMeasure`, `SolveError` (whose `TooFewStars`, `NotSolved` and `NoDatabase` each
    carry a `detail`) with its `message()`, and the trait `ImagingSolver` with its
    `measure`;
  - `crates/finder/src/imaging/simulated.rs` (032 US2): `SimulatedSolver`;
  - `crates/finder/src/imaging/solving.rs` (032 US2, US3): `Imaging::solver`,
    `Imaging::scale_arcsec_per_px`, and `solve` in the imaging view, a `SolveView`
    whose `frame_id` and `state` say which imaging frame a solve is running on; and
    `crates/finder/src/imaging/relation.rs` (032 US3): `CameraId`, and `relation.scale`
    in the imaging view, the scale an imaging solve measured, which 032 keeps across
    restarts and which is absent until one has solved;
  - `crates/simulator/src/imaging.rs` (030 US2): `ImagingOptics` with its
    `focus_error`, `PSF_SIGMA_PX` and `FOCUS_SIGMA_PX`; and
    `crates/simulator/src/optics.rs` (030 US2): `imaging_optics` and
    `set_imaging_optics` on `optics::Simulator`, reached through
    `controls::Simulator`'s `Deref`, beside which this spec's US1 puts the focuser;
  - `crates/finder/web/src/components/ImagingPage.tsx` and
    `crates/finder/web/src/useImaging.ts` (031 US7), and 032's Imaging solve block,
    which 032's own scenarios find as a region named `Imaging solve`;
  - the MCP tools `get_imaging_camera` and `take_imaging_frame` (031 US8);
  - the imaging camera's lease (031 US9), which the plan document's amendment 1 gives
    to 031's imaging state: `Lease`, `Imaging::lease`, `Imaging::holder`, `Lease::set`
    and `Lease::take`, whose refusal while another holds the camera is a 409 with the
    words `the imaging camera is held by {holder}` (the third and fifth sessions below;
    plan.md § The camera lease).
- Q: What do the other trios leave for this spec to settle? → A: Three things. 031
  says `Imaging::take` always saves a file, and its plan.md now takes owner decision
  13's answer for the focus aid, which, about once a second, saves each imaging frame
  and removes its file, so that no path without a file is needed (031 plan.md § Shapes
  that existing tests pin). 032's owner decision 24
  leaves the signal-to-noise floor of ASTAP's star measure to this trio: owner decision
  21. 034 asked whether the focus aid can be asked if it is running, where the Focus
  block sits on the Imaging page, and whether aid frames are saved: FR-029, FR-031 and
  FR-024 answer it, and 034's plan.md § Prerequisites 5 records those answers. 034's
  owner decision 23, who holds the camera while frames are taken one
  after another, is no longer open: the plan document's amendment 1 answers it for the
  series.
- Q: Which facts of the research does the design rest on? → A: From
  `docs/research/2026-11-15-imaging-camera.md` § 6. Focus and § 3. ASTAP: a half-flux
  diameter is monotonic through focus and reads large, not zero, when detection fails;
  ASTAP's star measure reports "the image median HFD" and a star count, analyses stars
  above a half-flux diameter of 20 incorrectly, and wants no large bright object in the
  picture; one tool reports a placeholder under five stars; the tools that guide a hand
  on the focuser all keep a history of readings, and one of them (SharpCap) also marks
  the best so far, colours each reading better or worse than recent ones and says to
  reset when conditions change, while the others plot a plain time series; one says to
  "wait a second or two after each focuser change"; "Best seen so far, with a rise on
  both sides" is the only verdict the data supports; a scatter of 0.4 to 0.5 pixels
  between frames is reported; and focusing exposures of a second or less do better than
  long ones. From § 4. Star counts and limiting magnitude at this field: how many stars
  a tile holds at a sparse field, which owner decisions 2, 16 and 21 work through.
- Q: What could not be checked? → A: Four things, each a field check (Assumptions):
  how ASTAP's star measure behaves on a small region of a raw colour mosaic; how much a
  reading scatters from frame to frame at a fixed focus; whether a tile of a one-second
  exposure holds five measurable stars at a sparse field, which the research note's
  own figures predict it does not at the galactic poles (owner decision 16 shows the
  arithmetic); and how long one reading
  takes at the telescope, which the view reports as `cadence_ms` for that purpose
  (`docs/research/2026-11-15-imaging-camera.md` § 13. Not established).
- Q: Where does this trio depart from the plan document and from `GLOSSARY.md`? → A: In
  ten places, each an owner decision or approval below. The plan document on `main`
  carries the first six, and the tenth's correction of its § 033, since 030's flip,
  `55e41f5` (plan.md § Prerequisites 9); the glossary's rewordings are written into
  `GLOSSARY.md` beside this trio, now that the owner has settled them (plan.md
  § Prerequisites 6).
  The plan document's "Settled decisions" table is not among them: its row for Focus
  stands as written. Of the last four places, three (7, 9 and 10) are this trio's
  readings of that row where it is silent, never changes to it, and one (8) is about
  the focus region, of which the row says nothing. The first six add to the plan
  document's § 033, its route table, its storage line and its module tree; the last
  four reword the glossary's entries (owner approval 11) and the plan document's § 033,
  and a different answer from the owner drops the reading:
  1. `POST /api/imaging/focus` takes a JSON body, and a focus check is the same
     request with `check` (owner decision 12).
  2. The focus error control is a simulate-only route, `/api/sim/focus`, which the
     plan document's route table did not list until the series audit added it (owner
     approval 2).
  3. The imaging folder also holds `focus.json` (owner approval 4).
  4. `focus.rs` has two child files (owner approval 3).
  5. The focus aid's imaging frames are removed once they are measured (owner decision
     13).
  6. `get_focus_reading` reads; the tool that takes a reading is `set_focus_aid` with
     `check` (owner decision 19).
  7. *The unit.* The row says "The focus reading is the median star size in the focus
     region, in arcseconds", and the glossary "as a half-flux diameter in arcseconds".
     This trio reads it as arcseconds whenever the console knows a scale. The row says
     nothing of a console that knows none, and there the reading is in sensor pixels;
     the record is always kept in sensor pixels (owner decision 6).
  8. *The region.* The glossary and the plan document's § 033 say the focus region is
     "the tile of a full frame with the smallest reading"; the row does not speak of
     it. Here it may also be a tile fixed by hand (owner decision 4), and while the
     stars are too far out to choose by, the aid reads a tile for the time being that
     is not kept and is not the focus region (owner decision 24).
  9. *The record.* The row judges a reading against the focus record, and the glossary
     calls it "The smallest focus reading this imaging camera has given on this
     telescope, tonight and on earlier nights". The record here is still the smallest
     reading: the smallest settled reading, kept in two parts, tonight's and the
     earlier nights', and set only by settled readings of the focus aid. The smaller
     of the two parts judges, as the glossary has it, until the aid has passed through
     focus tonight; then tonight's part judges alone (owner decisions 7 and 8).
  10. *A fifth verdict.* The row's verdicts are good, slightly soft, and clearly out or
      unmeasurable, each a comparison with the focus record. Where there is no record
      to judge against, the row gives no outcome; this trio's reading is `no_record`
      (owner decision 23). The plan document's § 033, which said "the three-way
      verdict", names it since the series audit.

  Two terms are new as well, focus check and settled reading, and both reach the page
  and the MCP tools' words. Both are in `GLOSSARY.md` on `main` since 030's flip
  (`55e41f5`), in owner approval 11's words; owner approval 11 words them and the three
  reworded entries, which are written into `GLOSSARY.md` beside this trio, so plan.md
  § Prerequisites 6 is met by the pull request that flips this spec.

### Session 2026-11-15 (the plan document's amendments, and the reviewed trios of 030, 031, 032 and 034)

- Q: What changed after the first draft of this trio, and what did this revision do
  about it? → A: The plan document gained "Mount-neutral by design", "Amendments after
  030, 031, 032 and 034 were drafted and reviewed" and two rows of settled decisions; 030, 031,
  032 and 034 were reviewed and fixed; and the drafting worktree moved from dbc2b5b to
  49bb210. Six things changed here:
  1. The focus aid holds the imaging camera's lease (amendment 1). The first draft had
     the aid end when another party's exposure or settings change slipped in, with the
     endings `camera_busy` and `settings_changed`; both are gone, because nothing else
     can expose or change the camera while the aid holds it (owner decision 14, FR-020,
     FR-021, FR-027, US5-S4).
  2. `SolveError` is used as 032's reviewed plan gives it: `TooFewStars`, `NotSolved`
     and `NoDatabase` carry a `detail`.
  3. Every `path:line` was read again at 49bb210. Three had moved: the every-tool test
     (`crates/finder/tests/mcp/motion.rs:517`), the request with a `Host` header
     (`crates/finder/tests/console/recording.rs:426`) and the support folder's list
     (`crates/finder/tests/layout/main.rs:126`). `Scratch`, which the first draft named
     without a line, has one.
  4. The far-out limit says what it is on a colour mosaic (owner decision 5).
  5. The research is quoted in the research note's own words.
  6. Owner decision 10 is weighed again against the "Another telescope later" row.
- Q: Which of the plan document's amendments reach this spec? → A: Six of the seven.
  - *1, one holder of the camera at a time*: the aid and the check take 031's lease
    (031 US9) under the name `the focus aid` and release it when they have ended;
    this spec adds no lock of its own (FR-020, FR-027, SC-007).
  - *3, the storage root*: `focus.json` lies in the imaging folder itself, beside
    032's `relation.json`; the aid's imaging frames are 031's, under `frames/`.
  - *4, two stand-in cameras*: every rig here has the simulator crate's
    `SimImagingCamera`, which has stars. Nothing uses the scope node's built-in one.
  - *5, the forwarding rate limit is a console setting*: the aid leaves it as the
    observer set it. A limit slows every reading, and `cadence_ms` shows it (Edge
    Cases).
  - *6, a region of interest is recorded with the frame*: the aid reads each imaging
    frame's own `roi` and `binning` from 031's view of that frame, never from the
    settings (FR-001).
  - *7, dependencies*: `depends_on_landed` names 030, 031 and 032, which is all this spec
    builds on. 034 names this spec in its own.
  - *2, white balance*, does not reach it.
- Q: Which lease is this trio written against? → A: 031's, as its User Story 9 gives
  it. When this session ran, 031's reviewed plan.md had no lease: its `take` and `set`
  refused only while an imaging frame was in flight, and its `SettingsEdit` named a
  `RoiEdit` it never defined. So this trio wrote down the smallest shape that says
  what the amendment says (one lease on 031's imaging state, naming its holder; a
  second taker refused with the holder's name; taken and released by each later
  spec), and 035's and 037's drafts assumed the same. The series audit (the fifth
  session) gave that shape to 031's trio unchanged, as its US9 with `RoiEdit`
  defined: the same four functions, `Imaging::lease`, `Imaging::holder`, `Lease::set`
  and `Lease::take`, the same six rules, and the refusal words `the imaging camera is
  held by {holder}`. plan.md § The camera lease now cites it. No story here creates
  it, and none may: it is 031's imaging state. A scenario that tests the lease asserts
  only what the amendment promises: a 409 that names the holder.
- Q: The imaging run (037) needs a focus check inside it. Who holds the lease during
  the check? → A: The check does. 037's draft has since chosen a hand-over (its owner
  decision 15): the run holds the lease as `the imaging run` while it slews, takes
  its own imaging frames, solves and centres, gives it up just before it asks for a
  focus check, and takes it again afterwards. That fits this trio as it stands: a
  focus check takes the lease itself, under the name `the focus aid`, and 037 calls
  `Imaging::focus_start` with `check` and awaits `Imaging::focus_ended()`. The other
  answer, a lease that 031 lets its holder lend to a step, stays 037's alternative;
  it would give `focus_start` a variant that takes a lease handed to it, a change to
  this trio made before the flip (plan.md § Shapes that existing tests pin, last
  bullet; § Risks).

### Session 2026-11-15 (the review of this trio by a second agent)

- Q: What did the review find, and what did this revision do about each point? → A:
  Eighteen findings; the validator had none. In the review's order:
  1. *The lease is in no plan of 031 (the blocker).* Not cleared in that session,
     because clearing it is an edit to 031's trio, which that revision could not make.
     It wrote the amendment's text for 031 instead, with `RoiEdit`. The series audit
     cleared it on 031's side (the fifth session below).
  2. *The open aid's verdict judged a reading against itself, and the first settled
     reading of a night hid a smaller record of earlier nights.* Fixed: a settled
     reading is judged against the record as it stood before it was noted, and
     tonight's part judges alone only once the aid has passed through focus tonight;
     until then the smaller of the two parts judges, as the glossary says (owner
     decision 8, FR-011, FR-012, FR-025, US3-S2, US5-S1, US8-S1).
  3. *Four departures from the plan document and the glossary were not listed, and two
     new terms are not in the glossary.* Listed (the second session above, departures
     7 to 10; owner approval 11; plan.md § Prerequisites 6 and 9). Writing the entries
     into `GLOSSARY.md` is the series' author's, not a story's.
  4. *Far out of focus at first light every tile was "unmeasurable" and the aid ended
     with the wrong words.* Fixed: the aid runs on the far-out tile with the smallest
     size for the time being, shows the size, and searches again at the first settled
     reading (owner decision 24, FR-007, FR-037, US2-S3, US9-S1).
  5. *The research predicts fewer than five stars a tile at a sparse field with the
     defaults.* The arithmetic is now in owner decisions 2, 16 and 21 and in plan.md
     § Risks, and the page and the README name the cure (FR-033, FR-030, US6-S4).
  6. *User Story 5 was over the 64 KiB budget.* Split: US5 keeps the loop, the routes,
     the view and the lease; a new story, US8, carries the focus check, the pass
     through focus on the simulator, the binary and the README section. US8 is built
     after US5 and before US6 and US7; it has the number 8 so that no other story or
     scenario of US6 and US7 is renumbered (Work Graph).
  7. *A start while a holder's own frame was exposing got the generic words.* Fixed:
     the frame-in-flight check is made only while nobody holds the lease (FR-020,
     US5-S4).
  8. *"No await between the checks" was claimed to make two starts exclusive.* Reworded:
     the lease does that (plan.md § The aid on the console).
  9. *No scenario read `focus` from `GET /api/session`.* Added (US5-S2).
  10. *An imaging solve asked for during the aid could lose its frame or leave a solve
      sidecar without one.* Fixed: a frame whose imaging solve is running when its
      turn to be removed comes is left in place (FR-039, US9-S2, owner decision 13).
  11. *Findings 11 to 16: wording, citations and places a builder would have had to
      guess.* Fixed where they stand; among them, the margin by which the binary's
      check finds its tile is 7.2% and not 22% (US8-S3), and 037's plan already calls
      this trio's `focus_start` and `focus_ended` and reads its five verdict words.
  12. *Finding 17: a record made at the focal-length flag's scale became "another
      telescope's" at the first imaging solve.* Fixed: only a scale an imaging solve
      measured is kept with the record or compared (owner decision 10, FR-013, US3-S4,
      US5-S1).
  13. *Finding 18: US2 to US4 are library slices, and US7 waited for US6 for no
      reason.* The first is stated and argued in the Work Graph; the second edge is
      gone.

### Session 2026-11-15 (the series audit of 030 to 037)

- Q: What did the audit of the seven trios the series then had ask of this one, and what
  was done? → A:
  Three findings, each applied in all three files:
  1. *The lease.* 031's trio now carries the imaging camera's lease as its User Story
     9 (its owner approval 12), with `RoiEdit` defined in its US1, in the shape this
     trio had written down: the four functions, the six rules, and the refusal words
     `the imaging camera is held by {holder}`. plan.md § The camera lease is a
     citation of 031 US9, with nothing
     changed in shape; the rule that a single imaging frame holds no lease (its point
     6) is 031's rule now, not an assumption of this trio. US5-S2, US5-S4, US5-S5 and
     the test setup's "The lease" keep their assertions. plan.md § Prerequisites 11 is
     met in the trios as drafted and is checked in code at the re-anchoring.
  2. *The plan document's "Settled decisions".* That table is not changed by any spec.
     Its row for Focus stands, and owner decisions 6, 7, 8 and 23 are declared as
     readings of it: arcseconds whenever a scale is known; the record still "the
     smallest reading"; `no_record` where the row has no record to judge against
     (the second session's departures 7, 9 and 10). Only the plan document's § 033,
     its route table, its storage line and its module tree are corrected, by the
     series' author (plan.md § Prerequisites 9).
  3. *Two wrong pointers.* `imaging_optics` and `set_imaging_optics` are on
     `optics::Simulator` in `crates/simulator/src/optics.rs`, not in
     `crates/simulator/src/imaging.rs` (the second session's list). And 036 bounds only
     a stack's exposure by the tracker's longest exposure, so the Out of scope line
     about the focus exposure no longer says that 036 defines a bound for it
     (Assumptions).
- Q: Did any citation move? → A: No. The worktree's HEAD is still 49bb210, which has
  all of 028. Every `path:line` in spec.md and plan.md was read there again, the files
  that 028 changed after dbc2b5b among them (`crates/finder/tests/console/recording.rs`,
  `crates/finder/tests/mcp/motion.rs`, `crates/finder/tests/support/mod.rs`), and each
  still names the line it describes.

### Session 2026-11-18 (round 1 probes)

- Q: What did the probe builds find? → A: Every story was built from this trio in a
  throwaway worktree, in the Work Graph's order, on top of 031's and 032's round-1
  probe builds, tests first, with both gates, `cargo deny check` and `cargo machete`
  (plan.md § Refinement Record, round 1). Every interface built as plan.md writes it,
  with private additions only, and every story's tests passed. Two defects were
  blocking. US1-S4 asserted that the simulated solver adapter without its builders
  answers 2.825784 for a whole frame at binning 2, where its star measure is in the
  frame's own pixels, as 032 specifies: 1.412892. And US5 measured 70,010 bytes
  against the 64 KiB budget, and the cut plan.md § Risks wrote in advance left it 397
  bytes under. Forty defects were reported in all. Their fixes are in this trio, and
  none adds a requirement.
- Q: How is US5 cut? → A: As the owner chose, in two. US5 keeps the loop, the routes,
  the view, the lease and the endings, with US5-S1 to US5-S5. A new story, US9, takes
  the tile read for the time being (FR-037, step 4j) and the exception for a frame an
  imaging solve is running on, now FR-039, split out of FR-024, with the scenarios that
  were US5-S6 and US5-S7, now US9-S1 and US9-S2, in a module of its own,
  `imaging_focus_far_out`. US9 is built after US5 and before US8, since US8-S1's check
  on a rig whose every tile is far out runs on the tile read for the time being. Until
  US9 lands, a search with no best tile ends `no_region` whether or not a tile was far
  out, and the aid removes its previous frame even while an imaging solve is running
  on it. The round-1 probe measured US5 so cut at about 63,284 bytes, 2.2 KiB under the
  budget, and put US9 at about 17 to 19 KiB before round 1's two additions to its
  second scenario (plan.md § Risks). Round 2, on `main`, measured them at 64,773 and
  21,701 bytes.
- Q: Which assertions did the probes add or correct? → A: Each where a build broken on
  purpose passed every test (the probes' bite checks), or where a scenario as worded
  could not pass:
  - US1-S3: `(25.0, 3)` is too few stars, so the five-star rule comes before the
    20-pixel one; and `(15.0, 9)` at binning 2 is a reading of 30.0, so the limit is
    read in frame pixels;
  - US1-S4: the plain adapter answers 1.412892 for the frame at binning 2, in its own
    pixels; and the sharpest point is tried on a frame whose region of interest, 824,
    0, 412, 274, is off the sensor's corner;
  - US2-S5: a file of another `format` holds nothing, and `keep_region` before
    `describe` writes no file;
  - US3-S2: a night is counted from its start, not from its record's time, on a second
    `Kept` whose first `note` also shows that a night's first reading carries the mark
    it is given; and a later night whose record is larger leaves the earlier nights' as
    they were;
  - US3-S4: a scale 6.4% away is another telescope's; and each `Kept` is "described for
    camera A and then given" its `note`, in place of "after", with the `record` call
    after the second `note` named;
  - US4-S1: a change keeps the trend (after samples 20 and 21);
  - US4-S3: `readings()` counts an unmeasurable sample too;
  - US9-S1 (US5-S6 when probed): a second view, kept at 3 readings, has no `verdict`
    and no `reference`, because its frame could be measured and the readings held are
    fewer than three (FR-025); the When clause keeps that view;
  - US9-S2 (US5-S7 when probed): on a rig of its own, a frame whose solve has ended
    when its turn comes goes with its solve sidecar (FR-024); and `last.at` is the
    middle of the frame's exposure;
  - US8-S1: `imaging.focus_ended()` is awaited right after the `POST` and returns only
    once three exposures have passed, so that US8 proves its own wait, which US7, a
    later story that does not write `run.rs`, could not repair; the When clause awaits
    it in place of polling the view;
  - US6-S2: no `Use the whole sensor` while the aid runs;
  - US6-S3: the reading carries the Display size's classes; and the `clearly_out` case
    is 7.68, 35%, so that a floor and a ceiling would each give another number;
  - US6-S4: no turning line after a check;
  - US6-S5: the history's `height` is 40.
- Q: Which behaviours did the probes settle where the plan left a choice? → A: These,
  each written where plan.md gives the behaviour: `Kept`'s save creates the imaging
  folder when it is missing, since a tile fixed by hand is kept before the aid has
  saved an imaging frame; the start's checks 1 to 7 read the imaging state's own
  fields under one lock, and the aid's steps 4f to 4h are one critical section,
  because `Imaging::view`, `Imaging::solver` and `Imaging::holder` take that lock
  themselves; the measured scale is 032's kept scale, read under that lock; only
  `Imaging::view` fills `focus` (on `main` the answers of the settings, frame and solve
  routes are `Imaging::view`'s own, so they carry it too: plan.md § The imaging state);
  no watch is woken for a change of the focus state; both waits on the
  focus state's `Notify` make the `Notified` future before they read the lifecycle;
  every `Option` of the focus view is skipped while `None`; the history `svg` is 40
  high at any count; `get_focus_reading` writes 031's not-ready words again, since the
  imaging module keeps them private; and US7 makes the README's sentence above the
  tool list count six tools.
- Q: Did any requirement's words change? → A: FR-037's, with its meaning kept. A tile
  read for the time being keeps no focus region, and its settled readings are noted in
  the record as FR-025 says and owner decision 7 has it (the record is the smallest
  settled reading of the aid); that is what the probe built, and US9-S1's oracle allows
  it. And a second search that finds no best tile, whether or not it finds a far-out
  one, goes on with the tile it had and does not end `no_region`, as plan.md's step 4j
  already said; the first probe build had ended there, by FR-022's ending beside it.
- Q: What is still caught by no test? → A: That a drop of the last outside `Arc` of the
  `Imaging` ends the aid only once the frame in flight has ended; step 5's wait for a
  frame in flight, which no ending of this spec reaches; a tile of a sensor whose corner
  is not 0, 0, and a search on a frame that is not the whole sensor, which no caller in
  this spec makes; and a focus check whose camera leaves `ready` before it ends, after
  which `set_focus_aid` answers the not-ready words.

### Session 2026-11-18 (the re-anchoring onto `main`)

- Q: What does the focus aid do when the answer to its take carries no frame? → A: It
  ends the run `failed` with 031's words for the camera's state. On `main` the view
  that `Lease::take` answers is built once the lock is released, and it carries `frame`
  only while the camera is `ready`, so a link that drops between the take and its
  answer leaves no frame id. The owner chose this ending on 2026-11-18. The alternative,
  waiting for the camera and taking again, would need a scenario of its own. No
  scenario reaches the case, and no test can (plan.md § The aid on the console, step 4b
  and the search). Step 1 ends the same way when the view it remembers has no
  `settings`.
- Q: What did the round-2 probe build find? → A: US1 to US4 from round 1, brought across
  to `main` at 670ddcf, then US5 as cut and US9 built from this trio, tests first, with
  every gate: no defect was blocking. US5 measured 64,773 bytes, 763 under the budget,
  and US9 21,701. Its clarity fixes are in: step 1's ending above, `run.rs` writing out
  031's four not-ready texts itself, the visibility of a `run.rs` function that takes
  the imaging state, T019 without the slow measure and solve, and plan trap 13's
  order, which no test can observe.
- Q: Should FR-038 move from US5 to US8 to give US5 more margin? → A: No. The owner kept
  US5 as cut on 2026-11-18. The move stays the next cut if the factory's US5 passes the
  budget (plan.md § Risks).
- Q: Are FR-037's "a search with no best goes on with the tile it had" and FR-029's "no
  provisional tile once the run has ended" pinned by a scenario? → A: They are now, by
  the owner's answer of 2026-11-18: US9-S1 gained a case on a rig of its own, worked by
  the round-2 probe and run on its build.

## Owner decisions

Each is a design or taste call the plan document leaves open. All take their recommended
answers: the owner approved the imaging series' recommendations on 2026-11-17 ("im good
with all of the info in the decisions doc") and flipped this spec on 2026-11-18. None is
left to a story.

1. **What the focus aid exposes.** Recommended: the focus region alone, as a region of
   interest, at the camera's smallest binning, whatever binning the settings held; the
   settings are put back when the aid ends. Instead: the whole sensor binned 2 × 2,
   which shortens nothing on the cable and halves the sampling.
2. **The tiles.** Recommended: three across and three down, each a third of the sensor,
   so one tile lies on the sensor's centre and each holds a ninth of the field's stars.
   A ninth is already thin at a sparse field. On the worked example a field holds 30
   to 36 stars brighter than magnitude G 14 at the galactic poles and 78 to 97 at
   galactic latitude 30° (`docs/research/2026-11-15-imaging-camera.md` § 4. Star
   counts and limiting magnitude at this field), so a tile holds 3 to 4 at the poles,
   under the five a reading needs, and 9 to 11 at latitude 30°. How deep a focus frame
   reaches is owner decision 16's; the two are chosen together. Instead: five by five,
   whose tiles hold 3 to 4 such stars at latitude 30° and 1 to 2 at the poles; two by
   two, whose tiles hold 7 to 9 at the poles but none of which lies on the sensor's
   centre, where the optical axis usually is; or a window slid in half-tile steps, 25
   star measures for one choice.
3. **How long a found focus region is kept.** Recommended: until the observer asks for
   it to be found again; it is kept with the camera across restarts and nights, so the
   aid starts without a full imaging frame. A region found while focus was still well
   out is chosen with a thin margin (this spec's inference, for a field check to
   test): defocus enlarges every tile alike and the off-axis growth adds to it in
   quadrature, so the sharpest tile is still the smallest, but by a few percent,
   against a few percent of scatter. The README therefore says to choose `Find the
   sharpest tile again` once focus is close.
   Instead: found at every start, one more whole-sensor frame each time; or found
   again by the aid itself the first time it reports that focus was passed and
   regained, one more whole-sensor frame in the middle of focusing.
4. **Fixing the focus region by hand.** The glossary's focus region is only "the tile
   of a full frame with the smallest reading". Recommended: the observer may also pick
   one of the nine tiles; it stays until the region is found again, and the glossary's
   entry gains "or a tile fixed by hand" (owner approval 11). Instead: no fixing by
   hand, as the glossary has it, which leaves no way round a search that keeps a poor
   tile; or a free rectangle in pixels.
5. **The size above which a star measure is not a reading.** Recommended: 20 pixels of
   the imaging frame as the solver adapter reports it; such a frame is "far out" and
   its verdict is clearly out whatever the record. ASTAP's stated limit is 20 pixels of
   the picture it is given. For a colour mosaic the ASTAP adapter gives it a copy
   averaged 2 × 2 and doubles the answer (032), so on such a frame this limit is half
   of ASTAP's own, and on a mono frame it is ASTAP's own: on the safe side either way,
   and about four times an in-focus star on the worked example. Instead: another figure;
   a limit the solver adapter reports for itself, which adds a method to the solver
   port; or showing the number as a reading.
6. **With no scale known.** The plan document's "Settled decisions" row for Focus and
   the glossary give the focus reading in arcseconds, and the row stands as written.
   The scale comes from the first imaging solve or from `--focal-length-mm` (032), and
   an unfocused frame does not solve, so the row is silent about a console that knows
   no scale yet. Recommended: read the row as arcseconds whenever a scale is known;
   with none, the reading, the aid and the record still work, in sensor pixels, and
   the page says so; and the glossary's entry gains "or in sensor pixels until the
   console knows the scale" (owner approval 11). Instead: arcseconds only, refusing
   until a scale is known, which leaves a camera that cannot solve with no way to be
   focused unless the flag was given.
7. **What sets the focus record.** The row judges a reading against the focus record
   and the glossary calls the record the smallest focus reading; both stand, and this
   decision is how this trio reads them. Recommended: the record is still the smallest
   reading, the smallest settled reading of the focus aid (the median of three imaging
   frames at one focuser position), and nothing else sets it: not a single imaging
   frame, whose scatter would set a bar no later frame meets, and not a focus check,
   which would become its own reference. The record is kept in two parts, tonight's
   and the earlier nights', each the smallest settled reading of its time; the
   glossary's entry is reworded to say so (owner approval 11). Instead: every focus
   reading; or only the low point of a pass through focus the aid has seen rise on
   both sides, under which a night on which the observer stopped the aid early leaves
   no record at all.
8. **Which part of the record judges a reading.** The row says only "Verdict against
   the focus record", and it stands; the glossary promises one value today: "The
   smallest focus reading this imaging camera has given on this telescope, tonight and
   on earlier nights. A reading is judged against it". Recommended, as this trio's
   reading of both: just that, the smaller of tonight's part and the earlier nights',
   with one exception: once the focus aid has passed through focus tonight (it has
   reported that the reading fell, rose on both sides and came back), tonight's part
   judges alone, even when an earlier night was sharper, because seeing changes from
   night to night and the aid has then shown what tonight allows. And a settled reading of the open aid
   is judged against the record as it stood before that reading was noted, so no
   reading is its own reference. Instead: tonight's part as soon as the aid has given
   one settled reading tonight (the first draft of this trio: the first settled
   reading of a night, however soft, then hides a smaller record of earlier nights,
   every new low reads good against itself, and a check after a short, unfinished
   session reads good); always the smaller of the two, under which one night of rare
   seeing makes every later night read clearly out however carefully it is focused;
   or no record at all until a pass through focus has been seen, under which the
   first night's checks answer `no_record` and 037's run pauses until the aid has been
   through focus once.
9. **A night.** Recommended: 12 hours from the first settled reading of the night; a
   settled reading after that starts a new night and the old one joins the earlier
   nights. Instead: the local date at the session's site, which makes focus code read
   the site.
10. **Another telescope.** The owner plans a second one (the plan document's "Another
    telescope later" row), and the console tells cameras apart, not telescopes.
    Recommended: the record is kept with the scale an imaging solve measured, and a
    record whose scale differs from the measured scale now in force by more than 5% is
    another telescope's: it judges nothing and the next settled reading replaces it.
    Only a measured scale counts, on both sides. The scale worked out from
    `--focal-length-mm` is a nominal figure, and one that leaves out a corrector or a
    reducer in the light path can be more than 5% from the truth, so a record made
    before the first imaging solve would otherwise be thrown away by that solve, and
    the next focus check would answer `no_record`. A record
    made before any imaging solve therefore carries no scale and is no telescope's in
    particular; it takes the scale of the first solve that follows. Moving the camera
    between two telescopes costs the earlier nights' record each time; one pass of the
    focus aid makes tonight's, which judges alone once the aid has been through focus
    (owner decision 8). Instead: `focus.json` keeps one record and one focus region
    for each scale it has seen, so both telescopes' are kept; that is a list in a file
    whose format is pinned once written, so it is chosen before the flip or not at
    all. Or: the scale in force whatever its source, with the README telling the
    observer to give `--focal-length-mm` to within 5% of the truth. Or: one record a
    camera whatever the scale.
11. **The focus aid's figures.** Recommended: the reading shown is the median of the
    last three imaging frames; a frame more than 15% from that median is a change, and
    it and the next frame are set aside while the telescope settles; the trend compares
    with the settled reading before the change, and 5% either way is "the same"; a
    settled reading within 10% of the best is near it and one 20% or more above is far
    from it; "found" is far, near, far, near in that order; the best is kept over the
    last 600 settled readings. Instead: other figures.
12. **A focus check.** Recommended: `POST /api/imaging/focus` with `check` true takes
    imaging frames of the focus region until the first settled reading, at most nine,
    judges it, puts the settings back and stops by itself; one frame that cannot be
    measured ends it at once. It is what the imaging run (037) and an MCP client ask
    for. Instead: a route of its own; or judging the latest imaging frame as it is,
    which compares a binned full frame with a record made on a region at full
    resolution.
13. **What happens to the focus aid's imaging frames.** The solver port measures a
    saved file, and 031 saves every imaging frame. Recommended: the aid's frames are
    saved like any other, with the purpose label `focus`, and each one's file is
    removed once the next has been saved; the last stays. A quarter hour of focusing
    leaves one file. One exception: a frame on which an imaging solve (032) is running
    when its turn to be removed comes is left in place for good, with the solve
    sidecar that solve writes, so that a solve never loses its frame and no sidecar is
    left without one. Instead: keep them all; a path through 031's `take` that saves
    no file, which the solver port could not measure; or, for the exception, removing
    such a frame later, once its solve has ended.
14. **How the focus aid holds the imaging camera.** That it takes 031's lease (031
    US9) is the plan document's amendment 1 and not open. What is open is how. Recommended: one
    holder's name, `the focus aid`, for the aid and for a focus check; the lease is
    taken at the start, after the request has passed every other check, and held
    through finding the region, every imaging frame and putting the settings back,
    then released; a start while another holds the camera is refused with the lease's
    own status and words, unchanged, also while that holder's own imaging frame is
    exposing; and while the aid holds it, a single imaging
    frame and a settings change from the page or an MCP client are refused by the
    lease, with the aid's name, and the aid runs on. Instead: two names, `the focus
    aid` and `the focus check`; or releasing the lease between imaging frames, which
    lets another party's exposure in and brings back the endings the first draft had.
15. **Stopping.** Recommended: `DELETE /api/imaging/focus` aborts the exposure in
    flight, waits until the settings are back and then answers. Instead: answer at once
    and let the exposure finish.
16. **The focus exposure.** Recommended: its own figure, 1000 ms unless the request
    says otherwise, clamped to nothing: a value outside the camera's range is refused.
    The Imaging page has a field for it. What that default reaches is an estimate, and
    it is thin at a sparse field. The research note puts the worked example's limit at
    G 15.6 to 15.8 in 2 s at a signal-to-noise ratio of 10, uncertain by about a
    magnitude and true only near the centre of the field
    (`docs/research/2026-11-15-imaging-camera.md` § 4. Star counts and limiting
    magnitude at this field). This spec's inference from it, which no source states:
    half the exposure costs 0.4 to 0.75 of a magnitude, and the measure's floor of 30
    rather than 10 (owner decision 21) costs about 1.2 more, so a one-second focus
    frame measures stars to about G 14. To G 14 a tile holds 3 to 4 stars at the
    galactic poles, 9 to 11 at galactic latitude 30°, and about 14 in the Orion
    Nebula's field away from the Trapezium (owner decision 2). So with these defaults
    the aid and the check are expected to work at the showcase target and over most of
    the sky, and in the sparsest fields, towards the galactic poles, to end `no_region`
    or to read too few stars. The cure is the observer's: a longer focus exposure
    (three seconds reach 0.6 to 1.2 magnitudes deeper than one, to about G 15, where a
    polar tile holds 6 to 7 stars) or focusing on a richer field first. The page and
    the README say so (FR-033, FR-030), and 037's run can give its focus check a
    longer `exposure_ms` than the default. The one-second
    default is kept because the research reports that "the focusing exposures of 1s or
    even less give better results" and because it sets the pace of the loop. Instead:
    a default of 2000 or 3000 ms, which measures a polar tile at the price of a slower
    loop; the imaging settings' exposure; or 1000 ms with the measure's floor lowered
    (owner decision 21).
17. **Checks of the page.** Recommended: Vitest only. The block renders, idle, on the
    simulated console's Imaging page, so 031's own full-page night check counts its
    pixels. Instead: a Playwright check of a running aid, which edits
    `playwright.config.ts` and leaves the shared simulated console on a region of its
    sensor.
18. **The size of the reading on the page.** Recommended: the Display size, which
    `DESIGN.md` gives to RA and Dec today, because it is the one number the observer
    reads from a step away with a hand on the focuser. Instead: the Headline size.
19. **What the two MCP tools do.** Recommended: `get_focus_reading` reads the focus
    object and changes nothing; `set_focus_aid` starts or stops the aid and, with
    `check`, waits for the verdict. Instead: `get_focus_reading` takes a focus check
    itself, which makes a tool named "get" move the camera's settings.
20. **The simulated focuser under `finder --simulate`.** Recommended: the simulated
    solver adapter measures with 2% of seeded noise and stars that grow away from a
    point at 0.60 of the sensor's width and 0.45 of its height, so the region is worth
    finding at a desk. Instead: no noise and a flat field.
21. **The floor given to ASTAP's star measure** (032's owner decision 24).
    Recommended: keep 032's figure, a signal-to-noise ratio of 30, the figure ASTAP's
    own page uses; the cost at a sparse field is worked through in owner decision 16.
    Instead: 10, which measures stars about 1.2 magnitudes fainter, to about G 15 in
    one second, where a tile holds 6 to 7 stars at the galactic poles in place of 3 to
    4, and measures them more noisily. How ASTAP's size for a star at a ratio of 10
    scatters is not known and is a field check (Assumptions), which is why the floor
    stays where 032 put it until that check is made.
22. **The words to the observer.** Recommended: the standing paragraph and the verdict
    lines of FR-032. Instead: other words, with the same three things said: smaller is
    sharper, the judgement is against this camera's own record, and seeing, collimation
    and a warm mirror enlarge stars too.
23. **A reading with no record to compare it with.** The plan document's "Settled
    decisions" row names three outcomes, each a comparison with the focus record:
    good, slightly soft, and clearly out or unmeasurable. The row stands. A new
    camera, or one moved to another telescope, has no record, and there the row has no
    record to judge against. Recommended, as this trio's reading of the row where it is
    silent: a fifth verdict, `no_record`, with words of its own on the page, so that
    nothing is called good or out without something to compare it with; what the
    imaging run does with it is 037's (its owner decision 21 pauses and opens the focus
    aid). Instead:
    `unmeasurable`, which says something false about the frame; or `good`, which says
    something the data cannot support.
24. **Far out of focus when the region is to be found.** At first light the focuser
    may be a millimetre or more from focus, where stars are 34 pixels across on the
    worked example (`docs/research/2026-11-15-imaging-camera.md` § 6. Focus): every
    tile is then far out, none is a candidate, and a search has nothing to choose.
    Recommended: when no tile can be measured but at least one is far out, the focus
    aid runs on the far-out tile with the smallest size, for the time being: that tile
    is not kept and is not the focus region, each imaging frame shows its rough size
    so the observer can see it shrink, and at the first settled reading the aid
    searches again on a whole-sensor frame and keeps what it finds as `found`. A focus
    check in the same state takes one imaging frame of that tile and ends with clearly
    out. Only when no tile is far out either does a start end `no_region`, and its
    words are then about too few stars. Instead: the centre tile for the time being,
    whatever its size; or ending `no_region` with words of its own and leaving the
    observer to fix a tile by hand, which is what the first draft did without saying
    so.

## Owner approvals

Flipping this spec to `ready` approves the settled answer to every owner decision above,
and the following. Each is named in the FR or success criterion that makes it.

1. **Two routes and one part of the imaging object**: `POST /api/imaging/focus` and
   `DELETE /api/imaging/focus` (FR-020, FR-026, FR-028), and `focus` in `GET
   /api/imaging` and
   in the `imaging` object of `GET /api/session`, which the MCP tools that return either
   carry too (FR-029).
2. **One simulate-only route**: `GET` and `POST /api/sim/focus` (FR-004).
3. **Three module files**, which the plan document's module tree names since the series
   audit: `crates/finder/src/imaging/focus.rs` and its children
   `crates/finder/src/imaging/focus/aid.rs` (FR-015) and
   `crates/finder/src/imaging/focus/run.rs` (FR-020).
4. **One file format on disk**, `focus.json`, which the plan document's storage line
   names since the series audit: format `imaging-focus/1`, in the imaging folder
   (FR-008, FR-014).
5. **Saved imaging frames are removed**: the focus aid removes the file of each of its
   own imaging frames, and that frame's solve sidecar if one was written, once the next
   is saved, but for a frame an imaging solve is running on at that moment, which
   stays (FR-024, FR-039). Nothing else is ever removed.
6. **A change under `crates/simulator`**: `Simulator::focus_error` and
   `Simulator::set_focus_error` in `crates/simulator/src/optics.rs` (FR-003). It is the
   focus error control the plan document's § Simulator names.
7. **New public names in files 031 and 032 create**: `SimulatedSolver::with_measure_noise`
   and `SimulatedSolver::with_sharpest_at` (FR-005); on `Imaging`, the methods
   `focus_start`, `focus_stop`, `focus_running` and `focus_ended` (FR-020, FR-028,
   FR-029, FR-026); and `focus` in `ImagingView`. The lease is not among them, and
   neither is `RoiEdit`: both are 031's (the lease is 031 US9), approved with 031's
   trio, and flipping this spec approves no change to it (plan.md § Prerequisites 11).
8. **Two MCP tools**: `get_focus_reading` and `set_focus_aid` (FR-035, FR-036).
9. **`DESIGN.md`**: one new entry, Focus block; the Display line of its Hierarchy gains
   "and the focus reading"; one sentence added to 031's Imaging page entry, with the
   words of plan.md § `DESIGN.md`; and one component in `.impeccable/design.json`
   (FR-034).
10. **Document sections**: `## Focus` in `crates/finder/README.md`, two rows of its API
    table, one row of its simulate table and two lines of its MCP tool list, and two
    route lines in `crates/finder/src/lib.rs` (FR-004, FR-030, FR-036, FR-038).
11. **No story writes `GLOSSARY.md` or the plan document.** The terms used here (focus
    reading, focus region, focus record, focus aid, imaging solver, solver port, solver
    adapter) are written by the plan document's own change to `GLOSSARY.md`, which
    reached `main` with 030's flip (`55e41f5`). That change must also carry the five
    things below, and it is the series' author who writes them there, beside the other
    imaging terms, before the flip (plan.md § Prerequisites 6, met by the change that
    re-anchored this trio, which ships in the pull request that flips it). The two new
    entries are in `GLOSSARY.md` on `main` since 030's flip, in these words; the three
    rewordings, which waited for the owner's answers to decisions 4, 6, 7 and 8, are
    written into `GLOSSARY.md` beside this trio, in these words, and reach `main` with
    it, as 030's flip carried its glossary.
    Two are new entries, for words the page and the MCP tools show:
    - **Focus check**: "Up to nine imaging frames of the focus region taken without the
      observer, ended by the first settled reading and a verdict against the focus
      record. _Avoid_: autofocus (there is no motor), focus test".
    - **Settled reading**: "The median of three focus readings taken one after another
      at one focuser position. The focus aid shows it, and only it can set the focus
      record. _Avoid_: average, smoothed HFD".

    Three are entries reworded, each a departure listed in Clarifications:
    - **Focus reading** (owner decision 6): "The median size of the stars in an
      imaging frame's focus region, as a half-flux diameter: in arcseconds, or in
      sensor pixels until the console knows the scale. Smaller is sharper."
    - **Focus region** (owner decision 4): "The part of the imaging camera's frame
      where focus readings are taken: the tile of a full frame with the smallest
      reading, or a tile fixed by hand."
    - **Focus record** (owner decisions 7 and 8): "The smallest settled reading this
      imaging camera has given on this telescope: tonight's, and the earlier nights'.
      A reading is judged against the smaller of the two, or against tonight's alone
      once the focus aid has passed through focus tonight; never against an absolute
      number."

    An owner who settles decision 4, 6, 7 or 8 the other way drops that rewording,
    and the trio is changed to match before the flip. "Lease" and "holder" are the
    plan document's words for 031's one holder of the imaging camera (amendment 1;
    031 US9). The glossary's entry Lease is 031's, not this spec's.
12. **Edits to existing test files** (SC-002), and no others: one `mod` line for each
    new finder test module, in alphabetical order, in
    `crates/finder/tests/console/main.rs` (`mod imaging_focus_aid;`,
    `mod imaging_focus_arithmetic;`, `mod imaging_focus_check;`,
    `mod imaging_focus_far_out;`, `mod imaging_focus_reading;`,
    `mod imaging_focus_record;`, `mod imaging_focus_region;`),
    `crates/finder/tests/cli/main.rs` (`mod imaging_focus;`) and
    `crates/finder/tests/mcp/main.rs` (`mod imaging_focus;`), as 027's layout asks of
    every new finder test.

## User Scenarios & Testing *(mandatory)*

Every scenario shares these definitions unless it says otherwise.

- **The gate command.** Every Rust scenario runs in
  `cargo nextest run --workspace --all-targets --all-features --locked`, the `test`
  gate's first command, each test in its own process. The page's scenarios run in
  `npm test`, a later command of the same gate, in `crates/finder/web`.
- **Paused time.** A test of the in-process console runs under
  `#[tokio::test(start_paused = true)]`, so every wait is in virtual time. US8-S3 starts
  the built `finder` and is a plain `#[test]`, in real time.
- **Sensor pixels and frame pixels.** A frame pixel is one delivered sample of an
  imaging frame; at binning b it covers b sensor pixels on a side. A region is `x`, `y`,
  `width`, `height`: its top-left corner and its size. The solver port takes a region in
  frame pixels; a focus reading, a tile and the focus region are in sensor pixels.
- **The simulator's star size.** With e the simulator's focus error, the simulated
  solver adapter's star measure is `H(e) = 2·√(2·ln 2)·√(1.2² + (1.0·e)²)` sensor
  pixels (032 FR-008, from 030's point-spread constants): H(0) = 2.825784, H(1) =
  3.678346, H(2) = 5.492337, H(3) = 7.608656. The sign of e does not matter.
- **The scale of the rig.** The simulated imaging camera is 1236 × 822 pixels of
  19.12 µm, with binnings 1 and 2 and exposures from 1 to 3600000 ms (030 US2). A
  console built with `with_focal_length_mm(1800.0)` and no imaging solve has the scale
  `206264.806 × 0.01912 / 1800` = 2.1909906″ a sensor pixel (032 FR-018). A value in
  arcseconds below is the stated sensor pixels times that.
- **The focus rig** is an in-process console: a `Simulator` over the test database at
  the site 40° N 75° W on a `VirtualClock` from Unix 1768528800
  (`2026-01-16T02:00:00Z`), built as `crates/finder/tests/support/live.rs:41` builds it
  but pointed with `push_to(40.0, 0.0)`, at the celestial pole, where the imaging field
  moves by under a pixel in three seconds; a `SimNode` `node` over it with a 500 ms
  frame interval and the token `test-token`, served over a fresh duplex for each
  connection with `memory::serve_routes_on` and `node.service().routes()` (`SimNode`
  has no `routes()` of its own; its `service()` is at
  `crates/simulator/src/node.rs:76`), as
  `crates/finder/tests/support/mcp.rs:228` serves its node; a session paired to it with
  that site; then `AppState::shared` with `with_session`, `with_clock` on the
  simulator's clock, `with_link`, `with_live()`, `with_imaging(R)`,
  `with_imaging_solver(S)` and `with_focal_length_mm(1800.0)`. **S** is the solver the
  scenario names. **R** is the child path `imaging` of the test's scratch folder. The
  rig's imaging camera is the simulator crate's `SimImagingCamera`, which the
  `SimNode` carries (030 US3). No mount is connected: nothing here moves or reads one.
  A focus-rig scenario first waits until `GET /api/imaging` has `state` `ready`.
- **The scripted solver** is an `ImagingSolver` of the test's own. Its `name()` is
  `scripted`; its `solve` answers `Failed("not used")`; its `measure` keeps a note of
  each call (the frame's `roi`, `binning`, `width` and `height`, and the region) and
  answers the next entry of its list at once, repeating the last entry once the list
  is used up. An entry written `(2.60, 12)` is a star measure of 2.60 frame pixels and
  12 stars; any other entry is the `SolveError` named, as 032 gives it: `TooFewStars`
  and `NoDatabase` written alone are those variants with `detail: None`. A scripted
  solver "with a slow measure of m ms" awaits `tokio::time::sleep` of m ms before
  each `measure` answers, and one "with a slow solve of s ms" does the same before
  each `solve` answers; without those words neither waits.
- **The lease** is 031's (031 US9), as plan.md § The camera lease cites it. In a
  scenario, `imaging` is `state.imaging().unwrap()`. "The test takes the lease as `a
  test`" means it calls `imaging.lease("a test")`, requires `Ok`, and keeps the value
  until the scenario says it is dropped. An answer "names" a holder when it is a 409
  whose `error` contains the holder's name, or a `Refusal` with the status 409 whose
  message contains it. 031 words the refusal `the imaging camera is held by {holder}`;
  no scenario here asserts the rest of that sentence, which stays 031's to word.
- **Scratch folders.** A finder test module takes its folder from
  `crate::support::Scratch::new(name)`
  (`crates/finder/tests/support/mod.rs:12`), which makes the folder under
  `CARGO_TARGET_TMPDIR` and removes it when dropped.
- **Requests** go through the router in-process, as `request` in
  `crates/finder/tests/support/live.rs:90` sends them, with `Content-Type:
  application/json` and no `Origin`. A scenario that names an `Origin` builds its
  request with a helper of the module's own, with a `Host` of localhost on port 8080,
  as `crates/finder/tests/console/recording.rs:426` does. A foreign `Origin` is
  `http://elsewhere.example`.
- **Waiting until** a condition means reading `GET /api/imaging` every 100 ms until it
  holds, for at most 300 s, and failing otherwise. The view a scenario then asserts on
  is the one in which the condition first held. An imaging frame of the focus aid
  takes at least its exposure, 1000 ms unless the scenario says otherwise, so a count
  of readings is seen at every value it passes through.
- **A sample** in a US4 scenario is a reading of the stated sensor pixels with 12 stars
  unless the scenario says otherwise, pushed into a new `focus::aid::Aid` in the order
  given. "After n" means after the n-th sample was pushed.
- **Reading files.** A test parses `focus.json` itself with `serde_json` and lists
  folders with `std::fs`. **The frames folder** is `frames/2026-01-16` under R.
- **Comparing a view.** A test compares `focus` and its parts key by key, never whole,
  because later specs add keys.
- **Oracles.** Each scenario names its oracle: the simulator's truth and constants, a
  worked example whose arithmetic is shown, the published SplitMix64 stream, or, for a
  scenario that checks only what the console says and writes, the words, statuses and
  keys the scenario itself states, which the test compares exactly. No scenario's
  expected ratio lies on a threshold: a verdict is asked for at 1.097 and 1.103, never
  at 1.100.

### User Story 1 - The focus reading, and a focuser to turn on the simulator (Priority: P1)

As the console, I ask the imaging solver how large the stars of an imaging frame, or of
a region of one, are, and turn its answer into a focus reading: in sensor pixels, in
arcseconds when I know the scale, and refused when fewer than five stars were measured
or the stars are too large to measure. At a desk, the simulator has a focuser I can
turn.

**Why this priority**: Every later story reads this value, and nothing about focus can
be tested until the simulator's focus error can be set and measured.

**Independent Test**: `crates/finder/tests/console/imaging_focus_reading.rs`, the module
`imaging_focus_reading` of finder's `console` test binary. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development,
`cargo nextest run -p finder --locked --test console imaging_focus_reading::`.

**Acceptance Scenarios**:

1. **Given** a `SimulatedSolver::new(sim)` over a simulator built as the focus rig's,
   and imaging frames of the test's own description (no file is read) whose exposure
   starts at Unix 1768528810 and lasts 1 s, **When** a test calls `focus::read` with the
   frame id 7, no region and the scale 2.0, **Then** a committed test asserts that:
   - a whole frame at binning 1 (1236 × 822, region of interest 0, 0, 1236, 822)
     answers a reading with `px` 2.825784 within 1e-6, `arcsec` 5.651568 within 1e-5,
     at least 30 `stars`, `frame_id` 7, `region` 0, 0, 1236, 822 and `at` 1768528810.5;
   - after `sim.set_focus_error(2.0)`, `sim.focus_error()` is 2.0 and
     `sim.imaging_optics().focus_error` is 2.0, and the reading's `px` is 5.492337
     within 1e-4; after `sim.set_focus_error(-2.0)` it is the same within 1e-4;
   - with the focus error still −2.0, a whole frame at binning 2 (618 × 411) answers
     `px` 5.492337 within 1e-4: the star measure's 2.746168 frame pixels times the
     binning;
   - with no scale, `arcsec` is absent;
   - oracle: the simulator's star size, H(0) and H(2).
2. **Given** the scripted solver, **Then** a committed test asserts that:
   - with the entry `(3.10, 12)`, a frame at binning 1 whose region of interest is 824,
     0, 412, 274, no region and the scale 0.548, `read` answers `px` 3.10, `arcsec`
     1.6988 within 1e-9, `stars` 12 and `region` 824, 0, 412, 274, and the solver was
     called once with that frame and no region;
   - with the entry `(1.55, 12)`, a whole frame at binning 2 and the region 100, 50,
     200, 100 in frame pixels, it answers `px` 3.10 and `region` 200, 100, 400, 200,
     and the solver was called with exactly that region;
   - oracle: a worked example (3.10 × 0.548 = 1.6988; 1.55 × 2 = 3.10).
3. **Given** the scripted solver and a whole frame at binning 1, **Then** a committed
   test asserts what `read` answers for each entry:
   - `(3.0, 4)`: unmeasurable, `TooFewStars` with 4 stars; `(3.0, 5)`: a reading;
   - the error `TooFewStars`: unmeasurable, `TooFewStars` with 0 stars;
   - `(25.0, 3)`: unmeasurable, `TooFewStars` with 3 stars: the five-star rule comes
     first;
   - `(20.0, 9)`: a reading of 20.0; `(20.5, 9)` on a frame at binning 2:
     unmeasurable, `FarOut` with `px` 41.0 and 9 stars; `(15.0, 9)` on a frame at
     binning 2: a reading of `px` 30.0 (the limit is 20 frame pixels, not sensor
     pixels);
   - the errors `TimedOut { seconds: 30 }`, `NoDatabase` and `Failed("x")`: each is
     `read`'s own error, unchanged;
   - oracle: the stated rule of five stars and 20 frame pixels.
4. **Given** `SimulatedSolver::new(sim).with_measure_noise(0, 0.02)` with the focus
   error 0 and a whole frame at binning 1, **Then** a committed test asserts that:
   - its first three `measure` calls answer 2.869110, 2.818045 and 2.772256 frame
     pixels, each within 1e-5, and a second solver built the same way answers the same
     three;
   - fifty more answers each lie from 2.769268 to 2.882300, 2% either side of H(0);
   - `SimulatedSolver::new(sim).with_sharpest_at(0.8, 0.2, 3.0)` answers 5.094255
     within 1e-5 for that frame, 2.547127 within 1e-5 for the whole frame at binning 2,
     4.538578 within 1e-5 for the region 0, 0, 1236, 412 of the frame at binning 1,
     and 2.864762 within 1e-5 for a frame at binning 1 whose region of interest is 824,
     0, 412, 274, with no region (its centre, 1030, 137, is 0.0667 of the
     half-diagonal from 988.8, 164.4: `2·√(2·ln 2)·√(1.44 + 0.2²)`);
   - `SimulatedSolver::new(sim)` answers 2.825784 within 1e-6 for the whole frame at
     binning 1 and for its region 0, 0, 1236, 412, and 1.412892 within 1e-6 for the
     whole frame at binning 2 (its own pixels: 2.825784 / 2), as 032 specifies;
   - oracle: the published SplitMix64 stream for the seed 0, whose first three outputs
     are `0xe220a8397b1dcdaf`, `0x6e789e6aa1b965f4` and `0x06c45d188009454f`, and the
     worked sizes: a region whose centre is 0.6 of the sensor's half-diagonal from the
     sharpest point gives `2·√(2·ln 2)·√(1.44 + (3.0·0.6)²)` = 5.094255.
5. **Given** a simulated console built in process, as
   `crates/finder/tests/console/live/whole_loop.rs:12` builds one, **Then** a committed
   test asserts that `GET /api/sim/focus` answers 200 and exactly `{"focus_error":0.0}`;
   that `POST /api/sim/focus` with `{"focus_error":1.5}` answers 200 and
   `{"focus_error":1.5}`, after which the state's simulator has the focus error 1.5 and
   the `GET` answers 1.5; that `{"focus_error":101}`, `{}` and
   `{"focus_error":1,"x":2}` each answer 400 with a JSON `error` and change nothing,
   the first with `focus_error must be a number from -100 to 100`; that the `POST` with
   a foreign `Origin` answers 403; and that on a console built without a simulation
   both answer 404. Oracle: the stated bodies, statuses and words.

---

### User Story 2 - The focus region, found from a full imaging frame or fixed by hand (Priority: P1)

As the console, I cut a full imaging frame into nine tiles, measure each, and keep the
one with the smallest reading as the focus region, so that focus is read where this
telescope is sharpest tonight and not where the sensor's centre happens to be. The
observer can fix a tile by hand instead. Either is kept for the camera.

**Why this priority**: A reading taken off the optical axis is large whatever the
focus, and a verdict made there would be wrong every night.

**Independent Test**: `crates/finder/tests/console/imaging_focus_region.rs`, the module
`imaging_focus_region` of finder's `console` test binary. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development,
`cargo nextest run -p finder --locked --test console imaging_focus_region::`.

**Acceptance Scenarios**:

1. **Given** `focus::tile_region`, **Then** a committed test asserts that, for the
   sensor 0, 0, 1236, 822 at binning 1, the tile at column 0, row 0 is 0, 0, 412, 274;
   column 2, row 0 is 824, 0, 412, 274; column 1, row 1 is 412, 274, 412, 274; and
   column 2, row 2 is 824, 548, 412, 274; that for the sensor 0, 0, 4944, 3284 at
   binning 1 the tile at column 2, row 2 is 3296, 2188, 1648, 1094; that for 0, 0,
   1236, 822 at binning 2 the tile at column 1, row 1 is 412, 272, 412, 272; and that
   `Tile::new(3, 0)` and `Tile::new(0, 3)` are `None`. Oracle: a worked example: a tile
   is a third of the sensor each way, rounded down to a multiple of twice the binning
   (1236/3 = 412; 822/3 = 274, and 272 at binning 2; 4944/3 = 1648; 3284/3 = 1094.67,
   so 1094).
2. **Given** the scripted solver with the nine entries `(4.0, 12)`, `(3.5, 12)`,
   `(2.6, 12)`, `(3.6, 12)`, `(2.9, 12)`, `(3.4, 12)`, `(4.1, 12)`, `(2.2, 3)`,
   `(4.4, 12)`, and a whole frame at binning 1, **When** a test calls
   `focus::find_region`, **Then** a committed test asserts that `best` is column 2,
   row 0; that `tiles` holds nine entries in row-major order, the third a reading of
   `px` 2.6 and the eighth unmeasurable with 3 stars; and that the solver was called
   nine times, with the regions of the nine tiles in row-major order (the first 0, 0,
   412, 274, the last 824, 548, 412, 274). **And** for a whole frame at binning 2
   (618 × 411) the committed test asserts that the fifth call's region is 206, 136,
   206, 136, the tile's sensor pixels divided by the binning. Oracle: a worked example;
   the smallest of the eight tiles with five stars is 2.6, and the smaller 2.2 has only
   three stars.
3. **Given** the scripted solver and a whole frame at binning 1, **Then** a committed
   test asserts that:
   - nine entries of `(3.0, 2)` give a `best` of `None` and a `far_out` of `None`;
   - nine entries of `(3.0, 12)` give the `best` column 0, row 0, the first of the
     tied tiles, and a `far_out` of `None`;
   - a list whose second entry is the error `TooFewStars` and whose others are
     `(3.0, 12)` leaves that tile out and still answers a `best`;
   - a list whose every entry is `(25.0, 12)` gives a `best` of `None` and the
     `far_out` column 0, row 0, the first of the tied far-out tiles;
   - the nine entries `(3.0, 2)`, `(25.0, 12)`, `(25.0, 12)`, `(25.0, 12)`,
     `(22.0, 12)`, `(25.0, 12)`, `(25.0, 12)`, `(25.0, 12)`, `(25.0, 12)` give a `best`
     of `None` and the `far_out` column 1, row 1, the smallest of the far-out tiles;
   - the same nine with the first entry `(3.0, 12)` give the `best` column 0, row 0
     and a `far_out` of `None`: a far-out tile is named only when no tile can be
     measured;
   - a list whose third entry is the error `Failed("x")` makes `find_region` answer
     that error after exactly three calls;
   - oracle: the stated rules.
4. **Given** a simulator built as the focus rig's and
   `SimulatedSolver::new(sim).with_sharpest_at(0.8, 0.2, 3.0)`, and a whole frame at
   binning 1 whose exposure starts at Unix 1768528810 and lasts 1 s, **When** a test
   first measures the tile at column 2, row 0 itself through the solver and requires at
   least 5 stars, and then calls `find_region`, **Then** a committed test asserts that
   `best` is column 2, row 0 and that its reading's `px` is 2.864762 within 1e-5. With
   `with_sharpest_at(0.5, 0.5, 3.0)`, after the same check of the tile at column 1,
   row 1, it asserts that `best` is column 1, row 1 with `px` 2.825784 within 1e-5.
   Oracle: the simulator's truth: the tile whose centre is nearest the sharpest point
   has the smallest star size, and the nearest other tile reads 3.69 or more.
5. **Given** a `focus::Kept::new(path)` for a file in a scratch folder and a `CameraId`
   A (model `simulated imaging camera`, serial `SIM-0001`, 1236 × 822, 19.12 µm),
   **Then** a committed test asserts that after `describe(A)` its `region()` is `None`
   and no file exists; that after `keep_region` of the found tile at column 2, row 0
   (`px` 2.6, 12 stars, frame 3, at 1768528803.5) the file parses with `format`
   `imaging-focus/1`, `camera` equal to A, and a `region` with `col` 2, `row` 0, `x`
   824, `y` 0, `width` 412, `height` 274, `source` `found`, `px` 2.6, `stars` 12,
   `frame_id` 3 and `at` 1768528803.5, and the folder holds no name ending `.part`;
   that a second `Kept::new(path)` after `describe(A)` answers that region; that after
   `keep_region` of `KeptRegion::fixed` for column 1, row 1 the file's `region` has
   `source` `fixed`, `x` 412, `y` 274 and no `px`, `stars` or `frame_id`; that
   `describe(B)`, B being A with the serial `SIM-0002`, answers no region; that a
   `Kept` whose file is that file with `format` `imaging-focus/2` answers no region
   after `describe(A)`; that a `Kept::new(path)` given `keep_region` before any
   `describe` writes no file; and that a `Kept` whose file holds the text `not json`
   answers no region after `describe(A)` and writes a readable file at the next
   `keep_region`. Oracle: the stated keys and values, compared exactly.

---

### User Story 3 - The focus record and the verdict (Priority: P1)

As the observer, I am never told that focus is good, only how a reading compares with
the sharpest this camera has been on this telescope: the smaller of tonight's record
and the earlier nights', or tonight's alone once the focus aid has passed through focus
tonight and so shown what tonight's sky allows. The console keeps both across restarts.

**Why this priority**: The imaging run of 037 decides on this verdict whether to carry
on, warn or pause, and a verdict against a fixed number would be wrong on most nights.

**Independent Test**: `crates/finder/tests/console/imaging_focus_record.rs`, the module
`imaging_focus_record` of finder's `console` test binary. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development,
`cargo nextest run -p finder --locked --test console imaging_focus_record::`.

**Acceptance Scenarios**:

1. **Given** `focus::judge`, **Then** a committed test asserts that against the
   reference 3.00 a reading of 2.50 is `good`, 3.29 `good`, 3.31 `slightly_soft`, 3.89
   `slightly_soft` and 3.91 `clearly_out`; that with no reference each of those is
   `no_record`; that `TooFewStars` is `unmeasurable` and `FarOut` is `clearly_out`,
   each with the reference 3.00 and with none; and that `Verdict::as_str` gives exactly
   those five words. Oracle: a worked example: 3.29/3.00 = 1.097, 3.31/3.00 = 1.103,
   3.89/3.00 = 1.297, 3.91/3.00 = 1.303, against the plan document's 10% and 30%.
2. **Given** a `Kept` described for camera A, with T = 1768528800, no scale given, and
   `note`'s last argument, `passed`, false unless it is written, **When** a test calls
   `note` in this order, **Then** a committed test asserts each result:
   - before any `note`, `record(T, None)` has neither part and `passed` false, and
     `reference(T, None)` is `None`;
   - `note(3.40, T, None, false)` answers true; `note(3.10, T + 60, None, false)`
     true; `note(3.30, T + 120, None, false)` false; then `record(T + 200, None)` has
     `tonight` 3.10 at T + 60, no `earlier` and `passed` false, and
     `reference(T + 200, None)` is 3.10 of tonight, the only part there is;
   - `record(T + 43201, None)` has no `tonight` and `earlier` 3.10 at T + 60, and
     `reference(T + 43201, None)` is 3.10 of earlier;
   - on a second `Kept`, described for camera A with a file of its own and given the
     same first three `note`s but the first with `passed` true, `record(T + 1, None)`
     has `passed` true after that first `note`: a night's first reading carries the
     mark it is given; and `note(3.50, T + 43230, None, false)` answers true and
     `record(T + 43231, None)` has `tonight` 3.50 at T + 43230 and `earlier` 3.10 at
     T + 60: the night began at T, more than 12 hours before, though its record was
     read at T + 60;
   - `note(3.60, T + 50000, None, false)` answers true; then `record(T + 50001, None)`
     has `tonight` 3.60, `earlier` 3.10 and `passed` false, and the reference is 3.10
     of earlier: tonight's part has not been passed through, so the smaller judges;
   - `note(3.70, T + 50030, None, true)` answers true, because the record changed;
     then `record(T + 50031, None)` has `tonight` 3.60 at T + 50000 still, `earlier`
     3.10 and `passed` true, and the reference is 3.60 of tonight: tonight's part now
     judges alone;
   - `note(3.65, T + 50040, None, true)` answers false: nothing changed;
   - `note(2.90, T + 50060, None, false)` answers true; then `record(T + 50061, None)`
     has `tonight` 2.90 at T + 50060 and `passed` still true, and the reference is
     2.90 of tonight;
   - `record(T + 100000, None)` has no `tonight`, `earlier` 2.90 at T + 50060 and
     `passed` false, and the reference is 2.90 of earlier;
   - `note(3.80, T + 100000, None, false)` answers true; then `record(T + 100001,
     None)` has `tonight` 3.80 at T + 100000 and `earlier` 2.90 at T + 50060; and after
     `note(3.90, T + 150000, None, false)`, `record(T + 150001, None)` has `earlier`
     still 2.90 at T + 50060: the old night's 3.80 is not the smaller, so it does not
     join;
   - oracle: a worked example of the 12-hour night and of the rule of owner decision
     8: 43201 s after a night's first settled reading the night is over, counted from
     the night's start and not from its record's time; the smaller of 3.10 and 3.60 is
     3.10; the smaller of 3.10 and 2.90 is the earlier nights' record once the second
     night is over; and a night whose record, 3.80, is larger than the earlier nights'
     leaves the earlier nights' as they were.
3. **Given** the `Kept` of US3-S2 after its first three calls, with a region kept as in
   US2-S5, **Then** a committed test asserts that the file's `record` has `tonight`
   with `px` 3.10, `at` 1768528860 and `since` 1768528800 and no `passed` key, no
   `earlier` and no `arcsec_per_px`; that the file still has the `region`; that a
   second `Kept::new(path)` after `describe(A)` answers the same `record` and the same
   `region`; that after `describe(B)` both are empty; that a `keep_region` after the
   notes leaves the `record` in the file; and that after
   `note(3.20, T + 130, None, true)` the file's `record.tonight` has `passed` true and
   `px` 3.10 still, and a third `Kept::new(path)` after `describe(A)` answers a
   `record(T + 200, None)` whose `passed` is true. Oracle: the stated keys and values.
4. **Given** a `Kept` described for camera A and then given `note(3.00, T, Some(2.19),
   false)`, **Then** a committed test asserts that the file's `record.arcsec_per_px` is
   2.19; that `reference(T + 10, Some(2.28))` is 3.00 of tonight (the scale is 4.1%
   larger); that `reference(T + 10, Some(2.33))` is `None` (6.4% larger); that
   `reference(T + 10, None)` is 3.00 of tonight; that `reference(T + 10, Some(1.10))`
   is `None` and `record(T + 10, Some(1.10))` has neither part; and that after
   `note(4.00, T + 20, Some(1.10), false)`, `record(T + 25, Some(1.10))` has `tonight`
   4.00 and no `earlier`, and the file's `record.arcsec_per_px` is 1.10. **And** for a
   second `Kept`, described for camera A and then given `note(3.00, T, None, false)`, a
   record made with no measured scale, the committed test asserts that the file's
   `record` has no `arcsec_per_px`; that `reference(T + 10, Some(1.10))` is 3.00 of
   tonight: such a record is no telescope's in particular; that `note(3.20, T + 20, Some(1.10),
   false)` answers false and the file's `record` still has no `arcsec_per_px`; that
   `note(2.90, T + 30, Some(1.10), false)` answers true and the file's
   `record.arcsec_per_px` is then 1.10; and that `reference(T + 40, Some(2.19))` is
   then `None`, while `reference(T + 40, Some(1.12))` is 2.90 of tonight. Oracle: a
   worked example of the 5% rule (2.28/2.19 = 1.041; 2.33/2.19 = 1.064; 1.10/2.19 =
   0.502; 2.19/1.10 = 1.991; 1.12/1.10 = 1.018).

---

### User Story 4 - The focus aid's arithmetic (Priority: P1)

As the observer with a hand on the focuser, I want one steady number, the best it has
been, whether my last turn helped, and an honest "you have been through focus and are
back at it". The console works these out from the readings alone, because the focuser
has no scale it can read.

**Why this priority**: These rules decide what the observer is told to do with the
focuser; they are worth proving on worked examples before any camera is in the loop.

**Independent Test**: `crates/finder/tests/console/imaging_focus_arithmetic.rs`, the
module `imaging_focus_arithmetic` of finder's `console` test binary. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development,
`cargo nextest run -p finder --locked --test console imaging_focus_arithmetic::`.

**Acceptance Scenarios**:

1. **Given** the 39 samples 6.0, 6.1, 5.9, 6.0; 4.5, 4.5, 4.4, 4.6, 4.5; 3.2, 3.2, 3.3,
   3.1, 3.2; 2.8, 2.8, 2.9, 2.7, 2.8; 3.3, 3.3, 3.2, 3.4, 3.3; 4.0, 4.0, 4.1, 3.9, 4.0;
   3.3, 3.3, 3.3, 3.3, 3.3; 2.85, 2.85, 2.8, 2.9, 2.85, **Then** a committed test
   asserts, each number within 1e-9:
   - after 2: the median 6.05 from 2 readings, no best, no trend, `push` answered no
     settled reading;
   - after 3: the median 6.0 from 3, best 6.0, no trend, and `push` answered the
     settled reading 6.0;
   - after 5 and after 6: the phase `settling` and no median; after 8: the median 4.5
     from 2 readings; after 9: settled 4.5, best 4.5, trend `better`;
   - after 15: the phase `reading` (2.8 is 12.5% from 3.2, not a change) and the
     settled reading 3.1; after 16: settled 2.8, best 2.8, trend `better`;
   - after 20 and after 21: the phase `settling` and the trend still `better` (a change
     drops the readings held and the level, not the trend);
   - after 24: settled 3.3 and trend `worse`; after 29: settled 4.0, trend `worse`,
     best 2.8, found false;
   - after 34: settled 3.3, trend `better`, found false; after 35: found false;
   - after 36: settled 2.85, best 2.8, trend `better`, found true; after 39: found
     true;
   - `push` answered a settled reading after exactly the samples 3, 4, 9, 14 to 19, 24,
     29 and 34 to 39;
   - oracle: a worked example. Far from the best 2.8 is 3.36 or more and near is 3.08
     or less; the settled readings run far (6.0, 4.5), near (2.8), far (4.0), near
     (2.85).
2. **Given** the 18 samples 6.0, 6.0, 6.0, 5.9, 5.8, 5.6, 5.6, 5.6, 5.5, 5.4, 5.3, 5.3,
   5.3, 5.4, 5.5, 5.6, 5.6, 5.6, none of which is a change, **Then** a committed test
   asserts no trend after 6 (the settled reading 5.8 is within 5% of the level 6.0);
   trend `better` after 7 (5.6 against 6.0); `better` after 12 (5.3 against 5.6);
   `same` after 15 and 16 (5.4 and 5.5 against 5.6); `worse` after 17 (5.6 against the
   level 5.3); the phase `reading` after every sample; and best 5.3 at the end. Oracle:
   a worked example of the 5% rule for a slow turn.
3. **Given** the 11 samples 3.0, 3.0, 3.0, then a sample of 2 stars, then 3.1, 3.1,
   3.1, then a far-out sample of 41.0, then 3.6, 3.6, 3.6, **Then** a committed test
   asserts the phase `too_few_stars` and no median after 4; the median 3.1 from 1
   reading after 5; trend `same` after 7 (3.1 against 3.0); the phase `far_out` and no
   median after 8; trend `worse` after 11 (3.6 against 3.1); best 3.0 throughout from
   3; that `history()` is the eleven values in order, with `None` at the fourth and
   41.0 at the eighth; and that `readings()` is 11, since it counts every sample
   pushed, an unmeasurable one included. Oracle: a worked example.
4. **Given** an `Aid::new().with_best_kept(4)` and the 17 samples 5.0 four times, 3.0
   five times and 5.0 eight times, **Then** a committed test asserts best 3.0 after 9
   and after 16, and best 5.0 after 17, when the last four settled readings are all
   5.0. **And** for an `Aid::new()` given 130 readings of 3.0, the committed test
   asserts that `history()` holds 120 values and `readings()` is 130. Oracle: a worked
   example: the settled readings are those after samples 3, 4, 9, 14, 15, 16 and 17.
5. **Given** three sequences, **Then** a committed test asserts found false at the end
   of 6.0 four times, 4.5 five times, 2.8 five times (the reading only fell); found
   false at the end of that followed by 4.0 five times (it rose and did not come
   back); and found true, with best 2.7, at the end of 6.0 four times, 2.8 five times,
   4.0 five times, 2.7 five times (2.8 is within 10% of the new best 2.7). Oracle: a
   worked example of far, near, far, near.

---

### User Story 5 - The focus aid on the console (Priority: P1)

As the observer, I start the focus aid and the console takes short exposures of the
focus region one after another, finding the region first if it has none, and shows each
reading with the best, the trend and the verdict until I stop it; then it puts the
camera's settings back. While it runs nothing else can expose or change the camera.

**Why this priority**: This is the loop the observer focuses with. The focus check of
User Story 8, which the imaging run needs before its first imaging solve, is this loop
with one more way to end.

**Independent Test**: `crates/finder/tests/console/imaging_focus_aid.rs`, the module
`imaging_focus_aid` of finder's `console` test binary. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test console imaging_focus_aid::`.

Until User Story 9, a search with no best tile ends `no_region` whether or not a tile
was far out, and the aid removes its previous frame even while an imaging solve is
running on it.

**Acceptance Scenarios**:

1. **Given** the focus rig with the scripted solver whose list is the nine entries of
   US2-S2 followed by `(2.60, 12)`, `(2.62, 12)`, `(2.58, 12)` and `(2.60, 12)`,
   **When** a test sets `{"exposure_ms":2000,"binning":2}` with `PUT
   /api/imaging/settings`, sends `POST /api/imaging/focus` with `{}`, waits until
   `focus.aid.readings` is 3 and keeps that view as the first view, waits until
   `focus.aid.readings` is at least 5 and keeps that view as the second, and then
   sends `DELETE /api/imaging/focus`, **Then** a committed test asserts that:
   - the `POST` answered 200 with `focus.aid.state` `starting`, `finding` or `running`;
   - the solver's first nine calls had the region of interest 0, 0, 1236, 822, binning
     1 and the nine tiles' regions in row-major order; every later call had the region
     of interest 824, 0, 412, 274, binning 1, a frame 412 × 274 and no region;
   - in the first view, `settings.exposure_ms` is 1000, `settings.binning` 1,
     `settings.roi` 824, 0, 412, 274 and `settings.save_at_node` false;
     `frame.purpose` is `focus`;
   - in the first view, `focus.unit` is `arcsec` and `focus.arcsec_per_px` 2.1909906
     within 1e-6; `focus.region` has `col` 2, `row` 0, `x` 824, `y` 0, `width` 412,
     `height` 274 and `source` `found`;
   - in the first view, `focus.aid` has `state` `running`, `exposure_ms` 1000, `median`
     with `value` 5.696576 within 1e-5 and `readings` 3, `best` 5.696576 within 1e-5,
     `found` false, no `trend`, `last.state` `reading` and `last.stars` 12, a `history`
     of exactly 5.696576, 5.740395 and 5.652756, each within 1e-5, and a `cadence_ms`
     of at least 1000; its `verdict` is `no_record` and it has no `reference`, because
     the first settled reading of a camera with no record is judged before it is
     noted; and `focus.record.tonight.value` is 5.696576 within 1e-5, because it has
     been noted by then;
   - in the second view, `focus.aid.verdict` is `good`, `focus.aid.reference` has
     `value` 5.696576 within 1e-5 and `of` `tonight`, and `focus.aid.median.value` is
     5.696576 within 1e-5;
   - the `DELETE` answered 200 with `focus.aid.state` `ended` and `ended_reason`
     `stopped`, and `settings` back at `exposure_ms` 2000, `binning` 2, `roi` 0, 0,
     1236, 822;
   - afterwards the frames folder holds exactly one file ending `.fits` and none
     ending `.part`; and `focus.json` under R has `region.col` 2, `region.px` 2.6,
     `region.stars` 12 and `record.tonight.px` 2.6, no `passed` key in
     `record.tonight`, and no `arcsec_per_px` key in `record`, because the rig's scale
     comes from the focal length and no imaging solve has measured one;
   - oracle: a worked example (2.60, 2.62 and 2.58 sensor pixels times the rig's
     scale; the fourth and fifth settled readings are 2.60 again, judged against the
     2.60 noted before them) and the stated settings.
2. **Given** consoles built as the focus rig with the scripted solver `(3.0, 12)`,
   **Then** a committed test asserts that:
   - before any start `GET /api/imaging` has a `focus` with `unit` `arcsec`, an
     `arcsec_per_px` and no `aid`, `region` or `record`, and the `imaging` object of
     `GET /api/session` has a `focus` with the same `unit` and `arcsec_per_px` and no
     `aid`; on a rig built without `with_focal_length_mm` the `focus` of `GET
     /api/imaging` is exactly `{"unit":"px"}`; on a rig built without
     `with_imaging_solver` neither `GET /api/imaging` nor the `imaging` object of `GET
     /api/session` has a `focus` key;
   - `POST /api/imaging/focus` answers 409 and `this console takes no imaging frames`
     on a console built without `with_imaging`; 409 and `this console has no imaging
     solver` without `with_imaging_solver`; 409 and `the scope node is not connected`
     after the simulator's `set_unresponsive(true)`
     (`crates/simulator/src/controls.rs:115`) and a wait until `state` is `no_link`;
     409 and `an exposure is in progress` while an imaging frame of 10000 ms, asked
     for with `POST /api/imaging/frame`, is exposing and `imaging.holder()` is `None`;
     and 409 and `the focus aid is already running` 1 s after a first start;
   - it answers 400 for `{"x":1}`; 400 and `give find or tile, not both` for
     `{"find":true,"tile":{"col":0,"row":0}}`; 400 and `tile col and row must each be
     0, 1 or 2` for `{"tile":{"col":3,"row":0}}`; 400 and `exposure_ms 0.5 is outside
     the camera's range 1–3600000 ms` for `{"exposure_ms":0.5}`; and 403 with a foreign
     `Origin`; and after each of these `focus` has no `aid`;
   - `DELETE /api/imaging/focus` with nothing running answers 200 and a view whose
     `focus` has no `aid`;
   - `imaging.focus_running()` is false before a start and true 1 s after one started
     with `{"tile":{"col":1,"row":1}}`; and at that moment the `imaging.focus` of `GET
     /api/session` has the `unit` `arcsec` and an `aid.state` equal to the
     `focus.aid.state` of `GET /api/imaging` read just after it, which is `running`;
   - oracle: the stated words and statuses, compared exactly; the words for a camera
     that is not ready, an exposure in progress and a missing solver are 031's and
     032's own.
3. **Given** the focus rig with the scripted solver `(3.0, 12)`, **When** a test starts
   the aid with `{"tile":{"col":1,"row":1}}`, waits for a reading and stops it; starts
   it with `{}`, waits and stops it; and starts it with `{"find":true}`, waits for a
   reading and stops it, **Then** a committed test asserts that the first start made no
   whole-sensor call: every call had the region of interest 412, 274, 412, 274, and
   `focus.region` had `source` `fixed`; that the second start's calls had that region
   of interest too; and that the third start made nine calls on a whole-sensor frame
   and then calls with the region of interest 0, 0, 412, 274, the first of nine tied
   tiles, with `focus.region.source` `found`. Oracle: the stated regions.
4. **Given** the focus rig with the scripted solver `(3.0, 12)` and `exposure_ms` first
   set to 2000 with `PUT /api/imaging/settings`, **Then** a committed test asserts,
   each on a rig of its own:
   - *the aid holds the camera.* Before the start `imaging.holder()` is `None`. After
     a start with `{"tile":{"col":1,"row":1}}`, once `aid.readings` is at least 1:
     `imaging.holder()` is `Some("the focus aid")`; `POST /api/imaging/frame` names
     `the focus aid`; `PUT /api/imaging/settings` with `{"roi":null}` names `the focus
     aid`; and `imaging.lease("a test")` is an `Err` that names `the focus aid`. After
     a further wait until `aid.readings` has grown by 2, `aid.state` is still
     `running`, `settings.roi` is still 412, 274, 412, 274 and every call of the
     solver had that region of interest. After the `DELETE`, `imaging.holder()` is
     `None` and `POST /api/imaging/frame` answers 200;
   - *another holds the camera.* On a rig where, after that `PUT`, the test has taken
     the lease as `a test`, and no aid was started before: `POST /api/imaging/focus`
     with `{"tile":{"col":1,"row":1}}` names `a test`; `focus` has no `aid`;
     `settings.exposure_ms` is still 2000 and `focus.json` does not exist under R.
     After the test drops its lease the same request answers 200;
   - *another holds the camera and its own imaging frame is exposing.* On a rig where
     the test has set `exposure_ms` to 10000 with `PUT /api/imaging/settings`, taken
     the lease as `a test` and called `take("test")` on it: while `frame.state` is
     `exposing`, `POST /api/imaging/focus` with `{"tile":{"col":1,"row":1}}` names `a
     test`, its `error` is not `an exposure is in progress`, and `focus` has no `aid`;
   - oracle: the plan document's amendment 1 and 031 US9's rules (a second taker is
     refused with the holder's name, and the lease is released).
5. **Given** the focus rig with the scripted solver, `exposure_ms` first set to 2000
   with `PUT /api/imaging/settings`, an aid started with `{"tile":{"col":1,"row":1}}`
   unless stated, and entries of `(3.0, 12)` unless stated, **Then** a committed test
   asserts, each on a rig of its own:
   - with the second entry the error `TimedOut { seconds: 30 }`: `ended_reason`
     `failed`, `error` `the imaging solver gave no answer within 30 s`,
     `settings.exposure_ms` back at 2000, `settings.roi` back at 0, 0, 1236, 822, no
     `restore_error`, and `imaging.holder()` `None`;
   - on a rig whose aid was started with `{"exposure_ms":10000,"tile":{"col":1,
     "row":1}}`: a `DELETE` sent while the frame is `exposing` answers, less than 5 s
     later on the simulator's clock, with `aid.state` `ended`, `ended_reason`
     `stopped`, `settings.exposure_ms` 2000 and `imaging.holder()` `None`; and on
     another such rig `POST /api/imaging/abort` ends the aid with `ended_reason`
     `stopped`;
   - with nine entries of `(3.0, 2)` and a start of `{}` on a rig with no kept region:
     `ended_reason` `no_region`, `error` `no tile of the imaging frame had five stars
     to measure`, no `focus.region`, `settings.exposure_ms` back at 2000,
     `imaging.holder()` `None`, and no `focus.json` under R;
   - after `set_unresponsive(true)` during a running aid: `imaging.focus_running()`
     becomes false within 300 s, and `imaging.holder()` is then `None`; and after
     `set_unresponsive(false)` and a wait until `state` is `ready`, `focus.aid` has
     `ended_reason` `failed` and a `restore_error` that is not empty;
   - oracle: the stated reasons and words, compared exactly, and the plan document's
     amendment 1 with 031 US9's rules for the lease being free after every ending.
---

### User Story 6 - The Focus block on the Imaging page (Priority: P2)

As the observer at the telescope with one hand on the focuser, I read one large number
from the Imaging page, with the best so far, whether my last turn helped, a small
history, and a plain statement of how the reading compares with this camera's record. At
night it is red like everything else.

**Why this priority**: The page is the only thing to hand at the focuser; the console's
arithmetic is of no use until it is in front of the observer.

**Independent Test**: `npm test` in `crates/finder/web` runs
`src/components/FocusBlock.test.tsx`.

**Acceptance Scenarios**:

1. **Given** `ImagingPage` rendered with `fetch` answering `GET /api/imaging` with a
   `ready` view holding a `saved` frame and `focus` `{"unit":"arcsec",
   "arcsec_per_px":2.19}`, **Then** a committed test asserts a region named `Focus`
   after the region named `Imaging solve` in document order, holding:
   - a paragraph reading exactly `A focus reading is the size of the stars in the focus
     region: smaller is sharper. The console judges it against this camera's own focus
     record and never against a fixed number, because seeing, collimation and a warm
     mirror enlarge stars too. Use a field of stars away from any bright nebula.`;
   - a select named `Focus region` whose options are, in order, `Kept, or the sharpest
     tile`, `Find the sharpest tile again`, `Top left`, `Top centre`, `Top right`,
     `Middle left`, `Centre`, `Middle right`, `Bottom left`, `Bottom centre` and
     `Bottom right`; a field named `Focus frame time` holding 1000; and buttons named
     `Start focus aid` and `Check focus`;
   - no `status`, no `alert` and no button named `Stop focus aid`.
   **And** with a view that has no `focus`, the committed test asserts no region named
   `Focus`. Oracle: the stated names, order and words, compared exactly; the region
   named `Imaging solve` is the one 032's own page scenarios find.
2. **Given** `FocusBlock` rendered with that view, `fetch` answering 200 and a view,
   **Then** a committed test asserts that:
   - `Start focus aid` calls `fetch` with `/api/imaging/focus` and
     `{"method":"POST","headers":{"Content-Type":"application/json"},"body":"{\"exposure_ms\":1000}"}`,
     then `refresh`;
   - with `Focus region` set to `Find the sharpest tile again` and `Focus frame time`
     to 2000 the body is `{"exposure_ms":2000,"find":true}`; with `Top right` and 1000
     it is `{"exposure_ms":1000,"tile":{"col":2,"row":0}}`; with `Bottom left`,
     `{"exposure_ms":1000,"tile":{"col":0,"row":2}}`;
   - `Check focus` sends `{"exposure_ms":1000,"check":true}`;
   - a 409 answer with `{"error":"this console has no imaging solver"}` shows an alert
     with exactly that text and calls no `refresh`;
   - with `focus.aid.state` `running` the region holds a button `Stop focus aid`, which
     calls `fetch` with `/api/imaging/focus` and `{"method":"DELETE"}` and then
     `refresh`, and holds no select, no field and neither start button, nor, with
     `settings.roi` a region of the sensor, a button `Use the whole sensor`;
   - oracle: the stated requests, compared exactly.
3. **Given** `FocusBlock` with `focus` `{"unit":"arcsec","arcsec_per_px":2.19,
   "region":{"col":2,"row":0,"x":824,"y":0,"width":412,"height":274,"source":"found"},
   "record":{"tonight":{"value":5.70,"at":1},"earlier":{"value":5.41,"at":0}}}` and an
   `aid` of `state` `running`, `readings` 9, `last` `{"frame_id":12,"at":2,
   "state":"reading","value":5.74,"stars":12}`, `median` `{"value":5.7046,
   "readings":3}`, `best` 5.6512, `trend` `better`, `found` false, `verdict` `good`,
   `reference` `{"value":5.70,"of":"tonight"}` and `history` `[5.70,5.74,5.65]`,
   **Then** a committed test asserts, each text exact:
   - the text `Focus reading` and the reading `5.70″`, in an element whose classes
     include `text-[2.35rem]` and `sm:text-[2.75rem]`;
   - a status `Median of the last 3 imaging frames · 12 stars`;
   - `Best so far 5.65″`; `Sharper than before the last change`; and `Turn the focuser
     one way until the reading has passed its lowest and risen again, then come back to
     the lowest.`;
   - `Within 10% of the focus record of 5.70″ tonight.`;
   - `Focus record: 5.70″ tonight · 5.41″ on earlier nights.` and `Focus region: the
     top right tile, found from an imaging frame.`
   **And** the committed test asserts these variants:
   - `found` true: `Passed through focus and came back to the best so far.` in place of
     the turning line;
   - `trend` `worse`: `Softer than before the last change`; `same`: `About the same as
     before the last change`; no `trend`: none of the three;
   - `median.readings` 2: a status `2 of 3 imaging frames since the last change · 12
     stars`;
   - `verdict` `slightly_soft` with `median.value` 6.50 and `reference` 5.70 of
     `earlier`: `Slightly soft: 14% larger than the focus record of 5.70″ from earlier
     nights.`; `clearly_out` with 7.68: `Clearly out: 35% larger than the focus record
     of 5.70″ from earlier nights.`; `no_record`: `No focus record for this camera yet.
     Run the focus aid through focus to make one.`;
   - `unit` `px`, with the same numbers: `5.70 px`, `Best so far 5.65 px`, and a
     paragraph `No imaging scale is known yet, so sizes are in sensor pixels. Solve an
     imaging frame, or start the console with --focal-length-mm.`;
   - a `record` with only `earlier`: `Focus record: 5.41″ on earlier nights.`; a
     `region` with `col` 1, `row` 1 and `source` `fixed`: `Focus region: the centre
     tile, set by hand.`; the same `region` with `source` `provisional`: `Focus region:
     the centre tile for now. The sharpest tile is found once the stars are small
     enough to measure.`, and no text holding `set by hand` or `found from an imaging
     frame`;
   - oracle: the stated words; each number is the view's to two decimals, and the
     percentages are (6.50/5.70 − 1) and (7.68/5.70 − 1), rounded: 14.04% and 34.74%,
     which a ceiling would make 15 and a floor 34.
4. **Given** `FocusBlock` with each `aid` below, **Then** a committed test asserts:
   - `state` `starting`: a status `Starting the focus aid`; `finding`: a status
     `Finding the focus region on a full imaging frame`; `running` with `readings` 0
     and no `last`: the reading `—` and a status `Waiting for the first imaging frame`;
   - `last.state` `settling`: the reading `—` and a status `Settling after a change`;
     `too_few_stars` with `stars` 2: a status `Not measurable: 2 stars in the focus
     region, and 5 are needed`, a paragraph `Try a longer Focus frame time, or a field
     with more stars.`, and with `verdict` `unmeasurable` the line `Not
     measurable: too few stars in the focus region.`; `far_out` with `value` 22.30: a
     status `Far out of focus: stars about 22.30″ across`, and with `verdict`
     `clearly_out` and no `reference` the line `Clearly out: the stars are too large to
     measure.`;
   - `state` `ended` with `ended_reason` `stopped`: a status `Focus aid stopped.` and
     the two start buttons; `done` with `verdict` `good`: a status `Focus check done.`
     and the verdict line, and no paragraph that starts `Turn the focuser`; `done` with
     `verdict` `unmeasurable` and `last.state` `reading`: the line `Not measurable: the
     reading did not settle.`;
   - `ended_reason` `failed` with `error` `the imaging solver gave no answer within
     30 s`: an alert `Focus aid stopped: the imaging solver gave no answer within
     30 s.`, and no paragraph `Try a longer Focus frame time, or a field with more
     stars.`; with a `restore_error` `the scope node is not connected`, a second alert
     `The camera's settings were not put back: the scope node is not connected.`;
   - `ended_reason` `no_region` with `error` `no tile of the imaging frame had five
     stars to measure`: an alert `Focus aid stopped: no tile of the imaging frame had
     five stars to measure.` and the paragraph `Try a longer Focus frame time, or a
     field with more stars.`;
   - with no aid running, `settings.roi` `{"x":412,"y":274,"width":412,"height":274}`
     and a camera 1236 × 822: a paragraph `The camera is reading a region of its
     sensor.` and a button `Use the whole sensor`, which calls `fetch` with
     `/api/imaging/settings` and
     `{"method":"PUT","headers":{"Content-Type":"application/json"},"body":"{\"roi\":null}"}`,
     then `refresh`; with `settings.roi` the whole sensor, neither;
   - oracle: the stated words and request, compared exactly.
5. **Given** `FocusBlock` with a running `aid` whose `history` is `[2, 4, null, 3]`,
   **Then** a committed test asserts one element with the attribute
   `data-focus-history`, the role `img`, the name `Focus readings of the last 4 imaging
   frames`, the `viewBox` `0 0 16 40` and the `height` `40`, holding exactly three
   `rect` elements whose `x` are `0`, `4` and `12`, whose `height` are `18.0`, `36.0`
   and `27.0`, whose `y` are `22.0`, `4.0` and `13.0`, and whose `fill` is
   `currentColor`; that with a `history` of 70 values it holds the last 60; and that no
   element of the block has a `style` attribute, or a `fill` or `stroke` other than
   `currentColor` or `none`. Oracle: a worked example: a bar is 36 units tall at the
   largest value shown, so 2, 4 and 3 give 18, 36 and 27.

---

### User Story 7 - Two MCP tools (Priority: P2)

As an MCP client, I read the focus reading with its verdict, start and stop the focus
aid, and take a focus check that answers with its verdict, through the same console
actions and refusals as the page.

**Why this priority**: The owner's goal is an ask from an MCP client, and the client is
the one that has to tell the observer to refocus.

**Independent Test**: `crates/finder/tests/mcp/imaging_focus.rs`, the module
`imaging_focus` of finder's `mcp` test binary. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test mcp imaging_focus::`.

**Acceptance Scenarios**:

1. **Given** a console with MCP on, **When** a test calls `tools/list`, **Then** a
   committed test asserts that `get_focus_reading` and `set_focus_aid` are listed, each
   with `outputSchema` `{"type":"object"}`; that only the first has
   `annotations.readOnlyHint` true; that the first's description is `Read the focus
   reading: the focus region, the focus record, and the reading, best, trend and
   verdict of the focus aid or the last focus check.`; that the second's is `Start or
   stop the focus aid, as POST and DELETE /api/imaging/focus do. With check true it
   takes a focus check, waits until it ends and returns its verdict against the focus
   record.`; and that the second's `inputSchema.properties` has exactly the five keys
   `on`, `check`, `find`, `tile` and `exposure_ms`, with `required` exactly `["on"]`.
   Oracle: the stated names, schema and words, compared exactly.
2. **Given** the focus rig of US8-S1 (its file and its three entries) with MCP on,
   served as `crates/finder/tests/support/mcp.rs:28` serves its app, **When** a test
   reads the simulator's clock, calls `set_focus_aid` with `{"on":true,"check":true}`
   and reads the clock again, **Then** a committed test asserts that the clock moved by
   at least 3.0 s, so the answer came only once the check had ended; that
   `structuredContent.aid.state` is `ended` with `ended_reason` `done`, `verdict`
   `good` and `median.value` 7.011170 within 1e-5; and that `get_focus_reading` with
   `{}` then answers a `structuredContent` whose `unit`, `aid.verdict`,
   `aid.median.value` and `record.tonight.value` equal those of the `focus` of `GET
   /api/imaging` read just after. Oracle: three exposures of 1 s on the virtual clock,
   US8-S1's worked verdict, and the route's own view.
3. **Given** the focus rig with the scripted solver `(3.0, 12)` and MCP on, **Then** a
   committed test asserts that `set_focus_aid` with `{"on":true,"tile":{"col":1,
   "row":1}}` answers a `structuredContent` whose `aid.state` is not `ended`; that a
   second such call answers `isError` true and `the focus aid is already running`; that
   `set_focus_aid` with `{"on":false}` answers `aid.state` `ended` and `ended_reason`
   `stopped`; that `{}` answers `isError` true and `on must be true or false`; that
   `{"on":false,"check":true}` answers `isError` true and `set_focus_aid with on false
   takes no other argument`; that `{"on":true,"find":true,"tile":{"col":0,"row":0}}`
   answers `isError` true and `give find or tile, not both`; and that
   `get_focus_reading` with `{"x":1}` answers `isError` true and `get_focus_reading
   takes no arguments`. Oracle: the stated words, which are the route's own where the
   route has them.
4. **Given** a console with MCP on and no `with_imaging`, as
   `crates/finder/tests/support/mcp.rs:24` builds its state, **Then** a committed test
   asserts that `get_focus_reading` with `{}` answers at once with `isError` true and
   `this console takes no imaging frames`; that `set_focus_aid` with `{}` answers at
   once with `isError` true and `on must be true or false`, and with `{"on":true}`,
   `this console takes no imaging frames`; that on the focus rig built without
   `with_imaging_solver`, `get_focus_reading` answers `isError` true and `this console
   has no imaging solver`; and that no result of either tool on the focus rig contains
   `test-token`. Oracle: the stated words, and the token the rig paired with.

---

### User Story 8 - The focus check, and a pass through focus on the simulator (Priority: P1)

As the imaging run of spec 037, or an MCP client, I ask for a focus check: the console
takes a few imaging frames of the focus region, judges the first settled reading
against the focus record, puts the camera's settings back and stops by itself with a
verdict, without changing the record. And as the owner at a desk, I turn the simulated
focuser through focus and back and see the focus aid say so.

This story is built after User Story 9 and before User Stories 6 and 7 (Work Graph). It
has the number 8 because it was split from User Story 5 in review, and numbering it so
left every scenario of User Stories 6 and 7 with its number.

**Why this priority**: The imaging run needs the check before its first imaging solve,
and the pass through focus on the simulator is the one proof, short of the telescope,
that the reading, the region, the record and the arithmetic work together.

**Independent Test**: `crates/finder/tests/console/imaging_focus_check.rs`, the module
`imaging_focus_check` of finder's `console` test binary, and, for US8-S3, which starts
the built `finder`, `crates/finder/tests/cli/imaging_focus.rs`, the module
`imaging_focus` of `cli`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs both; during
development, `cargo nextest run -p finder --locked --test console imaging_focus_check::`
and `cargo nextest run -p finder --locked --test cli imaging_focus::`.

**Acceptance Scenarios**:

1. **Given** the focus rig built after the test has written `focus.json` under R with
   exactly `{"format":"imaging-focus/1","camera":{"model":"simulated imaging
   camera","serial":"SIM-0001","width":1236,"height":822,"pixel_um":19.12},
   "region":{"col":1,"row":1,"x":412,"y":274,"width":412,"height":274,
   "source":"fixed","at":1768528000.0},"record":{"tonight":{"px":3.0,
   "at":1768528000.0,"since":1768528000.0}}}`, and the scripted solver `(3.20, 12)`,
   `(3.25, 12)`, `(3.15, 12)`, **When** a test sends `POST /api/imaging/focus` with
   `{"check":true}`, awaits `imaging.focus_ended()` and then reads `GET /api/imaging`,
   **Then** a committed test asserts `ended_reason` `done`, `check` true, `readings` 3,
   `median.value` 7.011170 within 1e-5, `verdict` `good`, `reference` with `value`
   6.572972 within 1e-5 and `of` `tonight`; that the solver was called exactly three
   times, each with the region of interest 412, 274, 412, 274; that `settings.roi` is
   back at 0, 0, 1236, 822 and `imaging.holder()` is `None`; that
   `imaging.focus_ended().await`, awaited right after the `POST` answers and before any
   read of the view, returns only once the simulator's clock has moved by at least
   3.0 s since the `POST` answered, with a view whose `aid.state` is `ended` and whose
   `aid.verdict` is `good`; and that the file's `record.tonight.px` is still 3.0 and
   its `record.tonight` has no `passed` key. **And** on fresh rigs, each with that file
   changed as stated, the committed test asserts:
   - with the file's `px` 2.8: `verdict` `slightly_soft`; with 2.4: `clearly_out`;
     with no `record` key: `no_record` and no `reference`, and the file has no `record`
     key afterwards either;
   - with `at` and `since` 1768482000, 13 hours before the rig's start: `reference.of`
     `earlier`, `verdict` `good`, and `focus.record` with an `earlier` and no
     `tonight`;
   - with the `record` `{"tonight":{"px":3.2,"at":1768528000.0,"since":1768528000.0},
     "earlier":{"px":2.4,"at":1768400000.0}}`: `reference` with `value` 5.258377
     within 1e-5 and `of` `earlier`, and `verdict` `clearly_out`: tonight's part has
     not been passed through, so the smaller part judges; and with `"passed":true`
     added to that `tonight`: `reference` with `value` 7.011170 within 1e-5 and `of`
     `tonight`, and `verdict` `good`;
   - with the solver `(3.0, 2)`: `readings` 1, `verdict` `unmeasurable`, `last.state`
     `too_few_stars`; with `(25.0, 9)`: `readings` 1, `verdict` `clearly_out`,
     `last.state` `far_out`;
   - with the nine entries `(3.0, 12)`, `(4.0, 12)` alternating: `readings` 9,
     `ended_reason` `done`, `verdict` `unmeasurable` and no `median` of 3 readings;
   - on a rig with no `focus.json`, with the solver's nine entries of `(25.0, 12)`
     followed by `(25.0, 9)`: `ended_reason` `done`, `verdict` `clearly_out`,
     `readings` 1 and `last.state` `far_out`; the solver was called exactly ten
     times, the tenth with the region of interest 0, 0, 412, 274, the first of the
     tied far-out tiles; `focus` has no `region` afterwards; and `focus.json` does not
     exist under R;
   - oracle: a worked example (3.20/3.00 = 1.067; 3.20/2.80 = 1.143; 3.20/2.40 =
     1.333; 3.20/3.20 = 1.000; 2.4 and 3.2 sensor pixels times the rig's scale are
     5.258377″ and 7.011170″), the hand-written file, and three exposures of 1 s on the
     virtual clock before the check can end.
2. **Given** the focus rig with `SimulatedSolver::new(sim).with_measure_noise(7, 0.02)`
   and the simulator's focus error set to 3.0, **When** a test first measures the tile
   at column 1, row 1 itself through a `SimulatedSolver::new(sim)` and requires at
   least 8 stars, starts the aid with `{"tile":{"col":1,"row":1}}`, and then, for each
   focus error of 3.0, 2.0, 1.0, 0.0, −1.0, −2.0, −1.0 and 0.0 in turn, sets it with
   `set_focus_error` and waits until `focus.aid.readings` has grown by 6, **Then** a
   committed test asserts that:
   - after the fourth step (the first 0.0) `median.value` is within 3% of 6.191266,
     `trend` is `better`, `found` is false and `check` is false, and `focus.json`'s
     `record.tonight` has no `passed` key;
   - after the sixth step `trend` is `worse` and `found` is false;
   - after the last step `found` is true, `trend` is `better`, `best` is within 3% of
     6.191266, `verdict` is `good`, `focus.record.tonight.value` is within 3% of
     6.191266, and `focus.json`'s `record.tonight.passed` is true;
   - oracle: the simulator's truth, H(0) × the rig's scale = 6.191266″, with 2% of
     seeded noise on each imaging frame; H(1)/H(0) = 1.30, so every step is a change
     and every neighbour of best focus is far from it.
3. **Given** the built `finder` run with `--simulate`, `--sim-start
   2026-01-16T02:00:00Z`, `--state S` and `--listen` on port 0 of `127.0.0.1`, S a
   scratch folder, **When** a test waits until `GET /api/imaging` has `state` `ready`,
   sends `POST /api/sim/focus` with `{"focus_error":2.0}` and `POST /api/imaging/focus`
   with `{"check":true,"exposure_ms":100}`, and waits up to 120 s of real time until
   `focus.aid.state` is `ended`, **Then** a committed test asserts `ended_reason`
   `done`, `readings` 3, `verdict` `no_record`, `focus.unit` `px`,
   `focus.region.source` `found` with `col` 1 and `row` 1, a `median.value` from 5.38
   to 6.00, and a `history` of three values that are not all equal. Oracle: the
   simulator's truth: under `--simulate` the stars are sharpest at 0.60, 0.45 of the
   sensor, in the middle tile, where the star size at a focus error of 2.0 is 5.6305
   sensor pixels, against 6.0386 in the tile above it, the next smallest: a margin of
   7.2%, where 2% of seeded noise either way can close at most 4.1%; no scale is
   known, so the unit is pixels, and no record exists.

---

### User Story 9 - The tile read for the time being, and a frame kept for a solve (Priority: P1)

As the observer at first light, with stars too far out of focus for any tile to be
measured, I want the aid to run anyway on the smallest far-out tile and find the region
by itself once the stars come in; and when I ask for an imaging solve while the aid
runs, I want the frame kept for that solve.

This story is built after User Story 5 and before User Story 8 (Work Graph). It has the
number 9 because it was cut from User Story 5 once the round-1 probe build measured that
story over the 64 KiB budget, and numbering it so left every other story and scenario
with its number.

**Why this priority**: At first light every tile is far out, and without this story the
aid ends `no_region` and leaves the observer no number to turn the focuser by; and a
solve asked for while the aid runs would lose its frame, or leave a solve sidecar beside
nothing.

**Independent Test**: `crates/finder/tests/console/imaging_focus_far_out.rs`, the module
`imaging_focus_far_out` of finder's `console` test binary. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development,
`cargo nextest run -p finder --locked --test console imaging_focus_far_out::`.

**Acceptance Scenarios**:

1. **Given** the focus rig with no kept region and the scripted solver whose list is:
   nine entries, each `(25.0, 12)` but the fifth, which is `(22.0, 12)`; then
   `(22.0, 12)`, `(21.0, 12)`, `(12.0, 12)`, `(12.0, 12)`, `(12.0, 12)`; then nine
   entries, each `(12.5, 12)` but the third, which is `(11.0, 12)`; then `(11.0, 12)`,
   **When** a test sends `POST /api/imaging/focus` with `{}`, waits until
   `focus.aid.readings` is 1 and keeps that view, waits until `focus.aid.readings` is 3
   and keeps that view too, waits until `focus.region.source` is `found`, waits until
   `focus.aid.readings` is at least 8 and sends `DELETE /api/imaging/focus`, **Then** a
   committed test asserts that:
   - in the view kept at 1 reading `focus.aid.state` is `running`; `focus.region` has
     `col` 1, `row` 1, `x` 412, `y` 274, `width` 412, `height` 274 and `source`
     `provisional`; `settings.roi` is 412, 274, 412, 274; `focus.aid.last.state` is
     `far_out` and `focus.aid.last.value` 48.201793 within 1e-5; `focus.aid.verdict`
     is `clearly_out`; `focus.aid` has no `median`; and `focus.json` does not exist
     under R: a tile read for the time being is not kept;
   - in the view kept at 3 readings, `focus.aid.last.state` is `reading` and
     `focus.aid` has no `verdict` and no `reference`: that frame could be measured and
     the readings held are 1, so the `clearly_out` of the two far-out frames before it
     is not kept;
   - once `focus.region.source` is `found`, `focus.region` has `col` 2 and `row` 0;
   - the solver's calls were, in order: nine on a frame with the region of interest
     0, 0, 1236, 822; five with the region of interest 412, 274, 412, 274; nine more
     on a frame with the region of interest 0, 0, 1236, 822; and every later call with
     the region of interest 824, 0, 412, 274;
   - the view before the `DELETE` has a `history` beginning 48.201793, 46.010803,
     26.291887, 26.291887, 26.291887, 24.100897, each within 1e-5;
   - afterwards `focus.json` under R has `region.col` 2, `region.row` 0,
     `region.source` `found` and `region.px` 11.0, and `record.tonight.px` 11.0; the
     `DELETE` answered `ended_reason` `stopped`, and `focus.region.source` is then
     `found`;
   - oracle: a worked example. No tile of the first search can be measured, and the
     smallest far-out tile is the centre one, at 22.0 frame pixels (22.0 × the rig's
     scale is 48.201793″). The first settled reading there is 12.0, after the fifth
     imaging frame, and starts the second search, whose smallest tile is the top right
     one at 11.0. The settled readings after that are 12.0 and then 11.0 (11.0 is 8.3%
     below 12.0, not a change), so tonight's record is 11.0 once seven imaging frames
     have been read;
   - and, on a rig of its own whose list is the first nine entries above, then
     `(12.0, 12)` three times, then nine entries `(25.0, 12)`, then `(9.0, 12)`, a test
     that sends `POST /api/imaging/focus` with `{}` and waits until
     `focus.aid.readings` is 5 finds `focus.aid.state` `running` with no
     `ended_reason`, and `focus.region` with `col` 1, `row` 1 and `source`
     `provisional`; the solver's calls were, in order: nine on a frame with the region
     of interest 0, 0, 1236, 822; three with the region of interest 412, 274, 412, 274;
     nine more on a frame with the region of interest 0, 0, 1236, 822; and two more with
     the region of interest 412, 274, 412, 274; when the test then sends
     `DELETE /api/imaging/focus`, the answer has `ended_reason` `stopped` and no
     `focus.region`, and `focus.json` under R has no `region`. Oracle: the rules of
     FR-037 and FR-029. The first settled reading, 12.0 after the third frame of the
     centre tile, starts the second search; no tile of that search can be measured, so
     it has no best, and the aid goes on with the tile it had instead of ending
     `no_region`. A tile read for the time being is shown only while the aid runs, and
     it is never kept.
2. **Given** the focus rig with the scripted solver `(3.0, 12)` with a slow measure of
   300 ms and a slow solve of 2500 ms, and an aid started with
   `{"tile":{"col":1,"row":1}}`, **When** a test waits until `frame.purpose` is `focus`
   and `frame.state` is `saved`, at once sends `POST /api/imaging/solve`, waits until
   `solve.state` is `failed`, waits until `focus.aid.readings` has grown by 2 and
   sends `DELETE /api/imaging/focus`, **Then** a committed test asserts that the `POST`
   answered 200 with `solve.state` `solving`; that once `solve.state` is `failed`,
   with S the view's `solve.file` and F the same path ending `.fits` in place of
   `.solve.json`, both S and F exist under R, although the aid has saved at least one
   imaging frame since; and that after the `DELETE` the frames folder holds exactly
   two files ending `.fits`, of which F is one, exactly one file ending `.solve.json`,
   which is S, and none ending `.part`. **And** once `focus.aid.last.frame_id` is the
   id of the frame the solve was asked for, `focus.aid.last.at` is that frame's
   `start`, as the view showed it when the frame was `saved`, plus half its
   `exposure_ms` in seconds (`start + exposure_ms/2000`), within 1e-6. **And** on a rig
   of its own with a slow measure of 300 ms and a slow solve of 100 ms, a test that
   asks for the solve the same way, waits until `solve.state` is `failed`, waits until
   `focus.aid.readings` has grown by 2 and sends the `DELETE` finds exactly one file
   ending `.fits` and none ending `.solve.json` or `.part`: the solve had ended when
   its frame's turn came, so its sidecar went with it. Oracle: the stated rules of
   FR-039 and FR-024: a frame an imaging solve is running on when its turn to be
   removed comes is left in place, and the solve sidecar 032 writes for it, also for a
   solve that fails, is never left without its frame, while a frame whose solve has
   ended by then goes with its sidecar; and the middle of the exposure (FR-001). The
   slow measure is what gives the test a moment in which the aid's latest frame is
   `saved`, as ASTAP's own measuring time does at the telescope; 032 refuses a solve
   of a frame that is still arriving.

---

### Edge Cases

- **No imaging solver.** Without `--astap` and outside `--simulate` the imaging view has
  no `focus`, the page shows no Focus block, and a start is refused with `this console
  has no imaging solver`.
- **No scale yet.** Before the first imaging solve, and with no `--focal-length-mm`,
  every size is in sensor pixels and `focus.unit` is `px`. The record is kept in sensor
  pixels either way, so it does not change when a scale arrives or is refined.
- **Fewer than five stars in the focus region.** The reading is unmeasurable. In the aid
  the number shown goes blank until an imaging frame is measurable again, and is settled
  again after three; a focus check ends with `unmeasurable`. The page then says `Try a
  longer Focus frame time, or a field with more stars.`
- **A sparse field.** With the defaults a tile is expected to hold fewer than five
  measurable stars in the sparsest fields, towards the galactic poles (owner decisions
  2, 16 and 21). A start with no kept region then ends `no_region`, and an aid or a
  check on a kept region reads too few stars. The cure is a longer focus exposure, or
  focusing on a richer field nearby first and coming back. The page says so in both
  cases and the README says why.
- **Far out of focus.** Stars more than 20 frame pixels across are beyond what ASTAP
  measures correctly on a mono frame, and are taken so on a colour mosaic too, whose
  2 × 2 averaged copy doubles that limit (owner decision 5); an obstructed telescope's
  stars are rings by then. A frame of
  them is "far out": the aid shows the rough size so the observer can see it shrink,
  and uses it for nothing else. When the region is to be found and every tile that has
  stars is far out, as at first light, the aid runs on the far-out tile with the
  smallest size for the time being, keeps no region, and searches again at its first
  settled reading (owner decision 24). A focus check in that state takes one imaging
  frame and answers clearly out.
- **A region found while focus was still well out.** The search that follows the first
  settled reading may run while the stars are still several times their in-focus size,
  where the sharpest tile leads the others by little. The region it keeps is `found`
  like any other. The observer chooses `Find the sharpest tile again` once focus is
  close; the README says so (owner decision 3).
- **A bright nebula in the focus region.** ASTAP asks for no large bright object in a
  picture it measures. The focus region is a part of the sensor, not of the sky, so
  when the target's core fills that tile the reading may be poor or unmeasurable. The
  page tells the observer to focus on a field of stars; where the imaging run takes
  its focus check is 037's (plan.md § Risks).
- **The telescope is not tracking.** The aid does not look at the mount. Trailed stars
  read as they are measured; ASTAP may read a streak small. The README says to track
  first.
- **The observer touches the telescope.** A frame taken while it shakes reads large, so
  it is a change: it and the next frame are set aside. That is the pause after a
  change, and it is also how a turn of the focuser is noticed at all.
- **A slow turn of the focuser.** No single frame is 15% from the median, so nothing is
  set aside; the median follows, and the trend moves once the settled reading is 5%
  from where it stood.
- **Seeing changes while the aid runs.** The best so far is the smallest of the last
  600 settled readings, so a best the sky no longer allows is forgotten, in about ten
  minutes at one imaging frame a second.
- **One night of rare seeing.** It lowers the earlier nights' record. A later night is
  judged against its own record once the aid has passed through focus that night;
  until then the smaller of the two judges, so a check may read clearly out against
  the rare night, and the page shows both figures and says which one it compared with.
- **A short session that never reached focus.** The observer starts the aid, turns a
  little and stops it. Tonight's record is then whatever was seen, and it is not marked
  as passed. A later check is judged against the smaller of it and the earlier nights'
  record, so a soft tonight's record does not hide a sharper one. On a camera's first
  night there is no earlier record, and the check is judged against tonight's: the
  page says `of the focus record of … tonight`, which is all the console knows.
- **The first settled reading with no record.** Its verdict is `no_record`: it is
  judged before it is noted, so it is never compared with itself. The next settled
  reading is judged against it.
- **The record after a change of telescope or reducer.** A record made at a measured
  scale more than 5% from the measured scale now in force judges nothing, and the next
  settled reading replaces it.
- **A record made before any imaging solve.** It carries no scale, because the scale
  worked out from `--focal-length-mm` is never kept with it. The first imaging solve
  therefore does not set it aside, however far the measured scale is from the nominal
  one; the next settled reading that lowers the record writes the measured scale with
  it. Such a record carried to another telescope before either was solved on is not
  recognised as the other telescope's; the README says to remove `focus.json` then.
- **Another camera.** `focus.json` is for one camera. A camera with another model,
  serial, size or pixel size starts with no region and no record, and its first kept
  value replaces the file.
- **Someone asks for an imaging frame, or changes the settings, during the aid.** The
  aid holds the camera's lease, so 031 refuses the request with the aid's name and the
  aid runs on. The observer stops the aid first.
- **Something else holds the camera when the aid is asked for.** A set of darks or
  flats (034), a stack (035) or an imaging run (037) will each hold the lease under
  its own name. The start is refused with the lease's words, which name the holder,
  also while that holder's own imaging frame is exposing, and nothing is changed: no
  setting, no file.
- **Two starts at the same instant.** The console answers requests on several threads,
  so two starts can both pass the check for a running aid. The lease lets one through.
  The other is refused with `the focus aid is already running`, or, when the two
  arrive in the same instant, with the lease's words, which name `the focus aid`: a
  409 either way.
- **A forwarding limit is set** (031's `forward_kib_per_s`). The aid leaves it alone.
  Every imaging frame then takes longer to arrive, and `cadence_ms` says how long one
  reading takes. The README says to clear the limit while focusing.
- **The scope link drops, or the console exits, during the aid.** The camera keeps the
  focus region as its region of interest, since nothing could put it back. The aid
  ends and releases the lease. The Focus block then offers `Use the whole sensor`; the
  exposure and binning are on the Settings form.
- **The telescope is changed.** The focus record of the other telescope judges nothing
  once both have been solved on (owner decision 10). The kept focus region is still
  the other telescope's sharpest tile; the observer chooses `Find the sharpest tile
  again`. The README says so.
- **An imaging solve asked for during the aid.** 032 refuses a solve while the latest
  imaging frame is still exposing or arriving, which during the aid is nearly always.
  In the gap while the aid measures a saved frame a solve can start, on that frame, a
  small region. If it is still running when the aid's next frame is saved, the aid
  leaves that frame in place for good, and the solve writes its solve sidecar beside
  it. If it has ended by then, the aid removes the sidecar with the frame. Either way
  no solve loses its frame and no sidecar is left without one. 032 takes no aim reading
  from a frame that is not the whole sensor.
- **Frame ids after a restart.** 031 counts on from the largest file number in the
  day's folder. The aid's last frame is the one it keeps, so no id is used twice.
- **An imaging run (037) that needs a focus check.** The check takes the lease itself,
  so the run cannot hold the camera across it. 037's draft gives the lease up just
  before it asks for the check and takes it again afterwards (its owner decision 15),
  which needs nothing more of this spec. Its alternative, a lease that can be lent,
  would (the third session of Clarifications).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: `focus::read` SHALL ask the solver port's `measure` for the star measure
  of a saved imaging frame, or of a region of it given in frame pixels, and answer a
  focus reading: `px`, the star measure's half-flux diameter times the frame's binning,
  in sensor pixels; `arcsec`, `px` times the scale given, absent without one; the star
  count; the region measured, in sensor pixels; the frame's id; and `at`, the middle of
  the exposure.
- **FR-002**: `focus::read` SHALL answer unmeasurable, `too_few_stars`, when the star
  measure has fewer than five stars or the solver answers `TooFewStars`; unmeasurable,
  `far_out`, with the size in sensor pixels, when the star measure is above 20 frame
  pixels (owner decision 5); and any other error of the solver unchanged.
- **FR-003**: The simulator SHALL offer `focus_error()` and `set_focus_error(e)`, which
  read and set the focus error of its imaging optics and nothing else (owner approval
  6).
- **FR-004**: A console built with a simulation SHALL serve `GET /api/sim/focus`,
  answering `{"focus_error": e}`, and `POST /api/sim/focus`, which takes exactly
  `{"focus_error": e}` with e from −100 to 100 under the same-origin rule, sets it and
  answers the same. Any other JSON body SHALL answer 400 and change nothing. Without a
  simulation both SHALL answer 404. `crates/finder/README.md` SHALL list the route in
  its simulate table (owner approvals 2 and 10).
- **FR-005**: The simulated solver adapter SHALL offer `with_measure_noise(seed,
  fraction)`, which multiplies each star measure by a seeded factor from `1 − fraction`
  to `1 + fraction`, and `with_sharpest_at(x, y, growth)`, under which the star size
  grows with the distance between the measured region's centre and the point (x, y) of
  the sensor, as plan.md § The simulated solver adapter gives both. Without either its
  answers SHALL be what 032 specifies (owner approval 7).
- **FR-006**: A tile SHALL be one of three columns and three rows. Its width and height
  SHALL be a third of the sensor's, each rounded down to a multiple of twice the
  binning, and its corner `(col × width, row × height)` from the sensor's corner (owner
  decision 2).
- **FR-007**: `focus::find_region` SHALL measure the nine tiles of a full imaging frame
  in row-major order, each through `focus::read`, and answer every tile's result; as
  `best`, the tile with the smallest reading, the first of equal ones; and, only when
  there is no `best`, as `far_out`, the far-out tile with the smallest size, the first
  of equal ones. A tile that is unmeasurable is not a candidate for `best`, and with no
  candidate there is none. A solver error other than `TooFewStars` SHALL end the
  search with that error (owner decision 24).
- **FR-008**: `focus::Kept` SHALL keep, for one camera, the focus region: its tile, its
  sensor pixels, whether it was `found` or `fixed`, and for a found one the reading,
  star count, frame and time it was found from. It SHALL write `focus.json` in the
  imaging folder, format `imaging-focus/1`, whole under a `.part` name and renamed, at
  every change; read it when the camera is described; and hold nothing when the file is
  missing, unreadable, of another format or of another camera (owner approval 4).
- **FR-009**: A focus region fixed by hand SHALL be one of the nine tiles, and SHALL be
  kept until the region is found again (owner decision 4).
- **FR-010**: `focus::judge` SHALL answer `unmeasurable` for a frame with too few stars
  and `clearly_out` for one that is far out, whatever the record; otherwise `no_record`
  without a reference (owner decision 23); otherwise `good` when the reading is at most
  1.10 times the reference, `slightly_soft` when at most 1.30 times, and `clearly_out`
  beyond.
- **FR-011**: The focus record SHALL have two parts, tonight's and the earlier nights',
  each a size in sensor pixels and its time, and tonight's part SHALL carry whether the
  focus aid has passed through focus tonight. `Kept::note` SHALL take a settled reading
  and whether the aid reports `found` at it: with no record of tonight, or one whose
  night began more than 12 hours before the reading, it starts a night with that
  reading, the old night's record joining the earlier nights' if it is the smaller;
  otherwise it replaces tonight's record only when smaller. A `note` with `found` true
  SHALL mark tonight's part as passed, and the mark SHALL stay for that night (owner
  decisions 7, 8 and 9).
- **FR-012**: The reference a reading is judged against SHALL be tonight's record
  alone, while its night began at most 12 hours ago and it is marked as passed;
  otherwise the smaller of tonight's record, while its night began at most 12 hours
  ago, and the earlier nights' record, with which a past night's counts; and none when
  there is neither (owner decision 8).
- **FR-013**: The record SHALL be kept with the measured scale in force when it was
  last noted: the scale an imaging solve gave, never one worked out from the focal
  length alone. While both that scale and the measured scale now in force are known
  and differ by more than 5%, the record SHALL judge nothing and show nothing, and the
  next `note` SHALL replace it. A record noted while no measured scale was known
  carries none, is never set aside by this rule, and takes the first measured scale a
  later `note` brings (owner decision 10).
- **FR-014**: The record SHALL be written to `focus.json` with the focus region,
  neither write losing what the other wrote, and read back when the camera is
  described.
- **FR-015**: The focus aid's reading SHALL be the median of the last three focus
  readings since the last change: the reading itself with one, the mean of two, the
  middle of three. With three it is a settled reading.
- **FR-016**: A focus reading more than 15% from the median of the readings held SHALL
  be a change: the readings held are dropped, and that reading and the next one are set
  aside. A frame that is unmeasurable SHALL drop the readings held and set nothing
  aside (owner decision 11).
- **FR-017**: The trend SHALL compare each settled reading with the settled reading
  that stood before the last change, or before the settled reading last moved more
  than 5% from its level: `better` when more than 5% smaller, `worse` when more than 5%
  larger, else `same`; absent until there is one to compare with.
- **FR-018**: The best so far SHALL be the smallest of the last 600 settled readings.
  `found` SHALL be true when the latest settled reading is within 10% of the best and
  those readings hold, in order, one 20% or more above the best, a later one within
  10% of it and a later one 20% or more above it.
- **FR-019**: The aid SHALL keep the last 120 samples in order for the history: the
  size of each reading and of each far-out frame, and nothing for a frame with too few
  stars.
- **FR-020**: `POST /api/imaging/focus` SHALL accept a JSON body with any of `find`
  (true or false), `tile` (`{col, row}`, each 0, 1 or 2) and `exposure_ms`, under the
  same-origin rule. A body with any other key, or a value of the wrong kind, SHALL
  answer 400 (a request whose `Content-Type` is not `application/json` answers 415)
  before anything else is looked at. Then it SHALL answer, in this order: 409 with
  031's words without imaging or while the camera is not `ready`; 409 `this console
  has no imaging solver`; 409 `the focus aid is already running`; 409 `an exposure is
  in progress` while an imaging frame is exposing or arriving and nobody holds 031's
  lease; 400 `give find or tile, not both`; 400 `tile col and row must each be 0, 1
  or 2`; 400 with 031's words for an `exposure_ms` outside the camera's range; then,
  when 031's lease is refused because another holds the imaging camera, the lease's own
  status and words, which name that holder, also while that holder's own imaging frame
  is exposing; and otherwise 200 with the imaging view, the aid started and the lease
  held under the name `the focus aid`. A refused request SHALL change no setting and
  write no file (owner approval 1, owner decision 14).
- **FR-021**: A started aid SHALL hold 031's lease, taken in FR-020, until it has
  ended and the settings are back (FR-027), and SHALL make every settings change and
  take every imaging frame through it; it SHALL add no lock of its own on the imaging
  camera. It SHALL remember the settings' exposure, binning, region of interest and
  node-copy switch; set the camera's smallest binning, the focus exposure (1000 ms
  unless given) and no node copy; and choose the focus region: the tile given, kept as
  `fixed`; else, with `find` or with none kept, the region found (FR-022); else the
  kept one (owner decisions 1, 3, 14 and 16).
- **FR-022**: To find the region the aid SHALL take one imaging frame of the whole
  sensor with the purpose label `focus`, run `focus::find_region` on it and keep the
  best tile as `found`. With no best tile and a far-out one it SHALL go on as FR-037
  says. With neither it SHALL end with `no_region` and the words `no tile of the
  imaging frame had five stars to measure`.
- **FR-023**: The aid SHALL then set the focus region as the region of interest and
  take imaging frames with the purpose label `focus` one after another, as fast as they
  are saved and measured, reading each whole with `focus::read`, and SHALL report the
  time between the last two as `cadence_ms`.
- **FR-024**: The aid SHALL remove the file of each imaging frame it took, and that
  frame's solve sidecar if there is one, once the next of its frames has been saved, so
  that the file of the last frame it saved stays. It SHALL remove no other file (owner
  approval 5, owner decision 13).
- **FR-025**: While the aid runs, its verdict SHALL be `judge` of each settled reading,
  or of an unmeasurable frame, against the reference (FR-012) as it stood before that
  reading was noted; the reference SHALL be kept with the verdict; and both SHALL be
  absent while the readings held are fewer than three and the latest frame could be
  measured. Each settled reading SHALL then be noted in the focus record (FR-011), with
  the aid's `found` of that moment and the measured scale (FR-013).
- **FR-026**: `POST /api/imaging/focus` SHALL also accept `check` (true or false), and
  the aid's part of the view SHALL carry it. With `check` true the aid SHALL note
  nothing in the record, and SHALL end by itself with `done`: at its first settled
  reading, with that reading's verdict; at its first unmeasurable frame, with that
  frame's verdict; or after nine imaging frames with neither, with `unmeasurable`. It
  SHALL end, put the settings back and release the lease as FR-027 says of every
  ending. `Imaging::focus_ended()` SHALL answer the focus view once no aid or check is
  running (owner decision 12, owner approval 7).
- **FR-027**: The aid SHALL end with `stopped` when asked to or when its frame is
  aborted; `failed`, with the words, when a frame, the solver or a settings change
  fails; and `no_region` as FR-022 says. However it ends, it SHALL wait out an imaging
  frame in flight, put the remembered settings back, reporting a failure to do so as
  `restore_error`, and then release the lease, so that the next taker is not refused.
  While the aid holds the lease, an imaging frame or a settings change asked for by
  anyone else is refused by 031 with the aid's name and does not end the aid (owner
  decision 14).
- **FR-028**: `DELETE /api/imaging/focus` SHALL, under the same-origin rule, abort the
  aid's exposure in flight, wait until the aid has ended and answer 200 with the
  imaging view; with nothing running it SHALL answer 200 and change nothing (owner
  decision 15).
- **FR-029**: `GET /api/imaging`, and the `imaging` object of `GET /api/session`, SHALL
  carry `focus` while the camera is `ready` and the console has an imaging solver, and
  omit it otherwise: `unit` (`arcsec` with a scale, else `px`), `arcsec_per_px`,
  `region`, `record` and `aid`, with every size in that unit, as plan.md § The focus
  view gives them. `region` SHALL be the kept focus region, or, while a running aid
  reads a tile for the time being (FR-037), that tile with the `source` `provisional`.
  The aid's part SHALL stay after it ends, until the next start or the console's exit.
  `Imaging::focus_running()` SHALL say whether an aid or a check is running, whatever
  the camera's state (owner approvals 1 and 7).
- **FR-030**: Under `--simulate` the `finder` binary SHALL build its simulated solver
  adapter with 2% of measure noise and stars sharpest at 0.60, 0.45 of the sensor
  (owner decision 20). `crates/finder/README.md` SHALL gain `## Focus`, saying what a
  focus reading, the focus region, the focus record and the verdict are, that no
  verdict is absolute and why, how the aid and a check run, what the aid does while
  the stars are too far out of focus to find a region by, that they hold the imaging
  camera while they run, what they do to the camera's settings and files, to track
  first and focus on a field of stars, that a sparse field wants a longer focus
  exposure or a richer field first, to clear a forwarding limit while focusing, and to
  find the focus region again once focus is close and after a change of telescope or
  collimation. The route's line in `crates/finder/src/lib.rs` and its row of the
  README's API table (FR-038) SHALL name `check` from this story on (owner approval
  10).
- **FR-031**: The Imaging page SHALL show a section headed Focus after the Imaging
  solve block whenever the view has `focus`: while no aid runs, a select `Focus
  region`, a field `Focus frame time` and the buttons `Start focus aid` and `Check
  focus`, which send the request of FR-020 and FR-026; while one runs, the button
  `Stop focus aid`. A refusal SHALL show its words as an alert.
- **FR-032**: The block SHALL always show the standing paragraph of US6-S1, which says
  that smaller is sharper, that the reading is judged against this camera's own record
  and never a fixed number, and that seeing, collimation and a warm mirror enlarge
  stars too. While an aid runs or after a check it SHALL show the settled reading in
  the Display size, with the status, best, trend, found or turning line, verdict,
  record, region and history of US6-S3 to US6-S5, each size in the view's unit; a tile
  read for the time being SHALL be named as that and never as found or set by hand. No
  line SHALL say that focus is good without naming the record it is compared with
  (owner decisions 18 and 22).
- **FR-033**: The block SHALL say how an aid ended, as an alert when it did not end by
  a stop or a finished check, and SHALL show a failure to put the settings back. While
  the latest imaging frame had too few stars, and after an aid ended `no_region`, it
  SHALL say `Try a longer Focus frame time, or a field with more stars.` While no aid
  runs and the settings' region of interest is not the whole sensor, it SHALL offer
  `Use the whole sensor`.
- **FR-034**: The block SHALL carry no colour of its own: existing classes, ink tokens
  and `currentColor` only. `DESIGN.md` SHALL describe it and give the Display size to
  the focus reading, `.impeccable/design.json` SHALL list it as a component, and
  `crates/finder/web/dist` SHALL be the build of its source (owner approval 9).
- **FR-035**: The MCP tool `get_focus_reading` SHALL be read-only, take no arguments
  and answer the focus view; without imaging, while the camera is not ready or without
  a solver it SHALL answer the error a start would be refused with (owner approval 8,
  owner decision 19).
- **FR-036**: The MCP tool `set_focus_aid` SHALL take `on` and, with `on` true, the
  keys of FR-020 and FR-026: it starts the aid as the route does and answers the focus
  view at once, or, with `check` true, once the check has ended. With `on` false it
  SHALL stop the aid as FR-028 does. A refusal SHALL be a tool error with the route's
  words. `crates/finder/README.md` SHALL list both tools (owner approvals 8 and 10).
- **FR-037**: When a search (FR-022) finds no tile that can be measured and at least
  one that is far out, the aid SHALL run on the far-out tile `find_region` names, for
  the time being: it SHALL keep no focus region (its settled readings are noted in the
  record as FR-025 says), show that tile as the view's `region` with the `source`
  `provisional`, and read imaging frames of it as FR-023 says. At each settled reading
  of such a run it SHALL search again, taking and measuring one whole-sensor frame as
  FR-022 does: with a best tile it SHALL keep that tile as `found` and go on there, no
  longer for the time being; without one, whether or not that search found a far-out
  tile, it SHALL go on with the tile it had, and SHALL NOT end with `no_region` (owner
  decision 24).
- **FR-038**: `crates/finder/src/lib.rs` SHALL list the two routes of FR-020 and
  FR-028, and the API table of `crates/finder/README.md` SHALL gain a row for each
  (owner approval 10).
- **FR-039**: The aid SHALL leave in place, for good, a frame on which an imaging solve
  (032) is running when its turn to be removed comes (FR-024), so that a solve keeps its
  frame and no solve sidecar is left without one (owner approval 5, owner decision
  13).

### Key Entities

- **Focus reading**: the median half-flux diameter of the stars in the focus region of
  one imaging frame, in sensor pixels and, with a scale, in arcseconds.
- **Focus region**: one of nine tiles of the sensor: the one with the smallest reading
  on a full imaging frame, or one fixed by hand.
- **A tile read for the time being**: the far-out tile with the smallest size, which
  the aid reads while no tile can be measured. It is never kept and is not the focus
  region.
- **Settled reading**: the median of three focus readings at one focuser position.
- **Focus record**: the smallest settled reading tonight, with whether the aid has
  passed through focus tonight, and the smallest on earlier nights, for one camera at
  one measured scale.
- **Reference**: the part of the record a reading is judged against: tonight's alone
  once the aid has passed through focus tonight, else the smaller of the two.
- **Verdict**: how a reading compares with the reference: `good`, `slightly_soft`,
  `clearly_out`, `unmeasurable` or `no_record`.
- **Focus aid**: the loop of imaging frames of the focus region with, for each, the
  reading, the best so far, the trend and whether focus was passed and regained.
- **Focus check**: at most nine imaging frames of the focus region, ended by the first
  settled reading and its verdict.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario has a committed test that asserts it, and the
  `lint` and `test` gates pass.
- **SC-002**: No existing test, golden, `scripts/ci/` file, `.github/` file,
  `CODEOWNERS` entry, `ergane.yaml`, `.specify/` file or `playwright.config.ts` is
  edited. The only edits to existing files under `crates/finder/tests` are the `mod`
  lines of owner approval 12.
- **SC-003**: No `Cargo.toml`, `Cargo.lock` or `deny.toml` changes, no `unsafe` is
  written, nothing changes under `crates/scope-node`, `crates/scope-link` or `proto/`,
  and under `crates/simulator` only `crates/simulator/src/optics.rs` changes.
- **SC-004**: On the simulator, with the focus turned from an error of 3.0 through
  best to −2.0 and back, the focus aid reports that focus was passed and regained, and
  a best within 3% of the true star size (US8-S2).
- **SC-005**: No file under `crates/finder/src/imaging/focus.rs` or
  `crates/finder/src/imaging/focus/` imports or calls the mount, an axis position, an
  altitude, an azimuth or the `ephemeris` module, and none holds a size in pixels or
  arcseconds of one camera or telescope (the mount-neutral rules). A comment that says
  the aid does not look at the mount breaks nothing.
- **SC-006**: Every line the page shows about a verdict names the focus record it is
  compared with, or says there is none or that the frame could not be measured
  (US6-S3, US6-S4).
- **SC-007**: While an aid or a check runs it holds 031's lease under the name `the
  focus aid`, and the lease is free again however the aid ends. No story adds another
  lock, flag or queue that keeps a second taker from the imaging camera (US5-S4,
  US5-S5, US8-S1; the plan document's amendment 1).

## Assumptions

- **Earlier features are in place.** 030-imaging-camera-link, 031-imaging-frames-on-console and
  032-imaging-solve-and-aim-point have landed, with the interfaces Clarifications list,
  031 among them with its User Story 9, the imaging camera's lease of the plan
  document's amendment 1, and a defined `RoiEdit` (plan.md § Prerequisites 11). Every
  `path:line` here was read at 49bb210, the drafting worktree's HEAD, checked there
  again in the fifth session, and read again on `main` at 670ddcf on 2026-11-18, when
  the trio was re-anchored after all three landed (plan.md § Prerequisites 2); every
  file of 030, 031 and 032 is named without a line.
- **Out of scope:**
  - autofocus, a focuser motor or any focuser position (the plan document defers it);
  - star detection or a half-flux measure of this project's own;
  - a model of how star size varies over the field, a tilt or collimation report, or a
    map of the nine tiles kept from night to night;
  - what the imaging run does with a verdict, the "go anyway", where on the sky it
    takes its focus check, and what focus exposure it gives the check (037);
  - dropping subs that measure poorly (035), which calls `focus::read`;
  - the imaging camera's lease itself (031 US9) and `RoiEdit`, which are 031's, and
    how 034's sets, 035's stack and 037's run take the lease; this spec only takes and
    releases it;
  - a lease that its holder can lend to a focus check (037's alternative to its owner
    decision 15);
  - a route, a flag or a button that forgets the focus record;
  - a focus region other than one of the nine tiles;
  - finding the focus region again without being asked, once focus has been passed
    and regained (owner decision 3's alternative);
  - one focus record and focus region for each telescope a camera has been on (owner
    decision 10's alternative);
  - any change to 031's `take`, `set`, `abort` or their refusals;
  - a Playwright check of the block (owner decision 17);
  - checking that the telescope tracks: the focus exposure is not bounded by the
    tracker's longest exposure, and the aid does not wait for the tracker to be
    steady; 036 defines the interface and bounds a stack's exposure only.
- **Outside the factory:** the field checks, each at the telescope with the real camera
  and ASTAP:
  - the reading falls and rises through focus as the knob turns, and the aid reports
    that focus was passed and regained on a real pass;
  - the in-focus value for this telescope and camera is written down, with the night's
    seeing;
  - the scatter of the reading from frame to frame at a fixed focus, which is what the
    15%, 5%, 10% and 20% of owner decision 11 are set against;
  - which tile the region search keeps, on two nights and after a re-collimation, and
    whether a search made while focus is still well out keeps the same tile as one
    made in focus;
  - starting the aid a millimetre or more out of focus, as at first light: that some
    tile reads far out and not too few stars, and that the rough size falls as the
    knob turns;
  - how many measurable stars a tile of a one-second exposure holds at a rich field,
    at galactic latitude 30° and near a galactic pole, and what focus exposure the
    sparsest needs: the test of the arithmetic in owner decision 16;
  - `cadence_ms` with the real camera and link, at a tile and at a full frame;
  - ASTAP's star measure on a tile of a raw colour mosaic, at a signal-to-noise floor
    of 30 and of 10;
  - a focus check with the Orion Nebula's core in the focus region;
  - a night check of the Focus block while the aid runs.
- **The factory's gates and CI:** the gates are `lint` and `test`, copied in plan.md
  § Verification. CI's other jobs run only in the merge queue, so no scenario's Then
  clause rests on them.

## Work Graph

```yaml
US1:
  depends_on: []
  implements: [FR-001, FR-002, FR-003, FR-004, FR-005]
US2:
  depends_on: []
  depends_on_merged: [US1]
  implements: [FR-006, FR-007, FR-008, FR-009]
US3:
  depends_on: []
  depends_on_merged: [US2]
  implements: [FR-010, FR-011, FR-012, FR-013, FR-014]
US4:
  depends_on: []
  depends_on_merged: [US3]
  implements: [FR-015, FR-016, FR-017, FR-018, FR-019]
US5:
  depends_on: []
  depends_on_merged: [US1, US4]
  implements: [FR-020, FR-021, FR-022, FR-023, FR-024, FR-025, FR-027, FR-028, FR-029, FR-038]
US6:
  depends_on: []
  depends_on_merged: [US8]
  implements: [FR-031, FR-032, FR-033, FR-034]
US7:
  depends_on: []
  depends_on_merged: [US8]
  implements: [FR-035, FR-036]
US8:
  depends_on: []
  depends_on_merged: [US5, US9]
  implements: [FR-026, FR-030]
US9:
  depends_on: []
  depends_on_merged: [US5]
  implements: [FR-037, FR-039]
```

Every edge is a merge edge. US1 to US5, US9 and US8 run one after another; US6 and US7
both follow US8 and share no file, so they can run side by side.

US2 measures its tiles with US1's `read` and adds to US1's `focus.rs`. US3 adds the
record to the file and the `Kept` US2 creates. US4 adds its module line to `focus.rs`
after US3 and feeds on US1's readings; it shares no logic with US3 and is ordered after
it only for that file and for the `mod` lines of the test binary. US5 runs US4's
arithmetic on US1's readings, in US2's region, against US3's record; its edge to US1 is
for `crates/finder/src/imaging/mod.rs` and the README, which US2 to US4 do not touch.

US8 was part of US5 until the review, whose count put the one story over the 64 KiB a
story's diff may be. US5 keeps the loop, the lease, the two routes, the view and the
endings; US8 adds `check` to the request and to the view, the one more way to end,
`Imaging::focus_ended`, the binary's simulated solver and the README's `## Focus`
section, with the pass through focus on the simulator and the built binary's check as
its proofs. It writes `run.rs`, `lib.rs`, `main.rs` and the README after US5, and a test
module of its own, so that no story edits another's tests.

US9 was part of US5 until the round-1 probe build measured US5 at 70,010 bytes, over the
budget. It adds to US5's `run.rs` the tile read for the time being (step 2's far-out
branch, step 4j and the provisional tile in the view) and the exception that leaves a
frame in place for an imaging solve running on it, with a test module of its own; US5
keeps the rest. US8 follows US9 as well as US5, because US8-S1's check on a
rig whose every tile is far out runs on the tile read for the time being, and because
both write `run.rs` and the `console` binary's `main.rs`.

US6 shows what US5, US9 and US8 put in the imaging view and calls US5's routes with US8's
`check`; it waits for US8 so that the page never offers a `Check focus` the console
would refuse. US7 calls US5's actions and waits on US8's `focus_ended`; its edge to US8
is also for the README, where US8 writes the section and US7 the tool list. US7 has no
edge to US6: the two share no file (the page, `DESIGN.md` and `.impeccable/design.json`
against `tools.rs`, the README and the `mcp` tests), and the first draft's edge between
them only made them wait for each other.

US2, US3 and US4 are library slices on purpose: each adds functions that no route, page
or tool reaches until US5. The plan document asks for vertical slices, and these are
the exception this trio argues for. Each is a rule the observer is told something by
(which tile is sharpest, how a reading is judged, whether the last turn helped) that
is exact arithmetic and can be proved on worked examples with no camera, clock or
console in the test; folded into US5 they would put one story at three times the
budget, and their proofs behind a camera loop. Each lands with its own committed tests,
which is what makes it a story the factory can judge from its diff.
