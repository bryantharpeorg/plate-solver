---
state: draft
depends_on_landed: ["030-imaging-camera-link", "031-imaging-frames-on-console", "032-imaging-solve-and-aim-point", "033-focus-reading-and-aid"]
# Drafted 2026-11-15 from docs/plans/2026-11-15-specs-030-037-imaging.md, by the
# grilling session of that day, first against `main` at dbc2b5b
# (dbc2b5bf15103e59e7166d9b7a79b421f9d96ce4), and revised the same day after a second
# agent's review and then after the series audit of 030 to 037 (§ Clarifications, the
# fourth session): a set holds the imaging camera's lease of the plan document's
# amendment 1, white balance is settled by its amendment 2, `match` passes over the
# focus aid's frames, and the shared writes name 033's stories.
# Citations checked against 49bb210fe4860dd98ba24727074e7bd3cc0d7124, the worktree's
# HEAD, on 2026-11-15 in that fourth session: every `path:line` in spec.md and plan.md
# was read there again; three were corrected (plan.md § Refinement Record).
# Revised on 2026-11-17 for the owner's answer to owner decision 7, the wider match of a
# master flat (§ Clarifications, the session of that day).
# Probe-built on 2026-11-18 (round 1, every story), on top of 031's, 032's and 033's
# round-1 probe builds: the round-1 probe fixes are in, and User Story 7, which measured
# over the budget, is cut as the owner chose: User Story 10 takes its three scenarios on
# the simulated camera and the imaging angle kept with the flats (§ Clarifications, the
# session of 2026-11-18; plan.md § Risks, § Refinement Record).
# It builds on 030, 031, 032 and 033, in the plan document's settled
# order, all four drafts when it was written: every interface of theirs named here was
# taken from that draft's plan.md, 031's lease and its frame's set temperature included;
# and this trio was re-anchored against `main` after the four landed and before it is
# flipped (plan.md § Prerequisites).
# Re-anchored 2026-11-19 to `main` at 80f414f (80f414ff9b369cfb8d5a2d51a0924c1bab3fdf1b),
# where 030's eight stories, 031's nine, 032's seven and 033's nine have landed: every
# citation was compared line for line with 49bb210 and read again by its content, and
# the ones that moved are corrected, the glossary's among them; every name and behaviour
# taken from 030 to 033 was read in their code, and the few that differed in name, path
# or place are corrected; a saved file's `EXPTIME` is the frame's own exposure and an
# exposure-only `Lease::set` waits on nothing, both now written as facts; the imaging
# routes' shared write names 033 US8, their last writer; and the Constitution Check says
# why the calibration arithmetic is this project's own (plan.md § Prerequisites 2 and 5,
# § Constitution Check).
# The owner approved the imaging series' recommended answers on 2026-11-17.
# It stays draft until the owner settles the owner decisions and flips it to `ready`,
# which gives the approvals listed under "Owner approvals" and dispatches it.
---

# Feature Specification: Dark and Flat Frames

**Feature Branch**: `034-dark-and-flat-frames`

**Created**: 2026-11-15

**Status**: Draft

**Input**: The console keeps every imaging frame the scope node relays (specs 030 and 031)
and will stack subs into a picture (spec 035). Before a sub is turned into colour it has to
lose the sensor's own signal and the telescope's uneven light: the offset level and any
fixed pattern, hot pixels, vignetting and dust shadows. This spec takes darks and flats,
averages them into master darks and master flats, keeps those in a calibration library on
the console's disk, finds a bad-pixel map in each master dark, and offers one function
that calibrates a sub on its mosaic and says which steps ran and which were missing. The
Imaging page gains a Calibration block and MCP clients three tools. Nothing here needs a
real camera: the simulated imaging camera gains a cover, a flat panel, vignetting, a dust
shadow, hot pixels and a column pattern, and its known patterns are the oracles.

## Clarifications

### Session 2026-11-15 (the interview's decisions, from the plan document)

- Q: What does this trio read besides itself? → A: Nothing about the feature. `spec.md`
  holds every requirement and scenario, and `plan.md` holds every design decision, the
  interfaces, the shapes that existing tests pin and the alternatives rejected. The
  binding decisions come from `docs/plans/2026-11-15-specs-030-037-imaging.md`,
  `docs/adr/0002-imaging-cameras-through-indi-frames-relayed-console-stacks.md` and
  `GLOSSARY.md`; facts come from `docs/research/2026-11-15-imaging-camera.md`. A story
  that would change a decision escalates rather than re-deciding it.
- Q: What is in this pass? → A: Taking darks and flats, master darks and master flats, a
  calibration library, the bad-pixel map, and applying them to a sub. Without a matching
  master a sub is still calibrated as far as it can be, and the report says what was
  missing.
- Q: Where does the work run? → A: On the console. The scope node takes each exposure and
  relays the imaging frame untouched, as 030 has it. This spec changes nothing on the
  scope node or the scope link: a dark is an exposure asked for with the purpose label
  `dark`, a flat with `flat`, and 030 already carries both.
- Q: In what order are the steps applied? → A: The master dark is subtracted, the result
  is divided by the master flat, bad pixels are repaired, and only then is the frame
  turned into colour (`docs/research/2026-11-15-imaging-camera.md` § 9. Calibration
  frames and § 8. Live stacking). Colour is 035's; the first three are this spec's.
- Q: Why are bad pixels repaired on the mosaic? → A: On an alt-az mount a hot pixel lands
  on a different part of the sky in every sub, so in a stack it draws a coloured arc, and
  turning the frame into colour first would spread it into its neighbours. It is replaced
  from samples of its own colour before any colour work
  (`docs/research/2026-11-15-imaging-camera.md` § 9. Calibration frames).
- Q: What does a dark remove at exposures of seconds? → A: The offset level, hot and warm
  pixels and any fixed pattern. Thermal signal is negligible on a cooled sensor at these
  lengths (`docs/research/2026-11-15-imaging-camera.md` § 1. The camera and § 9.
  Calibration frames). A dark of the sub's own exposure already contains the offset
  level, so a sub needs no separate offset frame.
- Q: What does a master flat need? → A: Its own dark taken off first, and the same
  optical train and camera orientation as the sub: turn the camera in the focuser and the
  flat is stale (`docs/research/2026-11-15-imaging-camera.md` § 9. Calibration frames).
- Q: Is anything keyed to the owner's camera? → A: No. Sizes, the Bayer pattern, the
  ranges and the full scale come from the frames and the camera's description. The
  owner's camera appears only as a worked example. Every threshold is relative to a
  measured level or noise, or is a share of the frame or of full scale.
- Q: Does this spec need a real camera or real frames? → A: No. Every scenario's oracle
  is the simulator's known pattern, a worked example, or a camera of the test's own. The
  owner takes real darks and flats later (Assumptions, Outside the factory).

### Session 2026-11-15 (review of `main` at dbc2b5b and of the 030, 031 and 032 drafts)

- Q: What exists today? → A: Nothing of this feature. At dbc2b5b the console has no
  imaging module at all; every file this spec extends under `crates/finder/src/imaging/`
  and `crates/simulator/src/imaging.rs` is created by 031 or 030 and is named here as a plain
  path with the story that creates it. Seven things were read in the code at
  dbc2b5b, and read again at 49bb210 in the fourth session, where item 5's line and,
  in the uncommitted glossary, item 6's last line had moved, and read again on `main`
  at 80f414f, whose line numbers are cited:
  1. The simulator's shared state is a `Copy` struct
     (`crates/simulator/src/optics.rs:65`, `crates/simulator/src/optics.rs:66`), where
     030 puts the imaging optics. A list of hot pixels or dust shadows cannot go in a
     `Copy` struct, so this spec's sensor pattern and light path live in a second shared
     cell beside it (`crates/simulator/src/optics.rs:76`).
  2. The console's router has one `DELETE` of its own (`crates/finder/src/api.rs:542`),
     and its catch-all answers only `GET` (`crates/finder/src/api.rs:547`); on `main`
     031's route table holds a second, 033's `DELETE /api/imaging/focus`. This spec's
     `DELETE /api/imaging/calibration/{id}` is added to 031's route table in
     `crates/finder/src/imaging/routes.rs` (031 US1).
  3. `finder` already depends on the simulator (`crates/finder/Cargo.toml:14`), on
     `image` (`crates/finder/Cargo.toml:27`) and on `serde_json`
     (`crates/finder/Cargo.toml:32`), and its tests on the scope node's crate
     (`crates/finder/Cargo.toml:36`). This spec adds no dependency and changes no
     `Cargo.toml`.
  4. A finder test is a module of one of the test binaries, and a file at the top level
     of `crates/finder/tests` fails the layout test
     (`crates/finder/tests/layout/main.rs:102`). The new Rust tests are modules of
     `console` and `mcp`, each declared by one `mod` line
     (`crates/finder/tests/console/main.rs:9`, `crates/finder/tests/mcp/main.rs:7`).
  5. An existing test calls every listed MCP tool with empty arguments and checks the
     result for the token (`crates/finder/tests/mcp/motion.rs:517`), on a console that
     takes no imaging frames. The three new tools answer such a call at once (US8-S4).
  6. `GLOSSARY.md` defines dark, flat, master dark, master flat and calibration library
     (`GLOSSARY.md:169`, `GLOSSARY.md:173`, `GLOSSARY.md:177`) and hot pixel
     (`GLOSSARY.md:341`). These entries were first read in the glossary as the plan
     document and the series audit rewrote it, a file then changed in the drafting
     worktree and not yet committed, which also defines the **Lease** (on the imaging
     camera) and the **Purpose label** this trio uses. At dbc2b5b and at 49bb210 the
     Dark, Flat and Calibration library entries did not exist, line 159 was the Camera
     port entry and hot pixel was at line 231. The rewrite reached `main` with 030's
     flip (`55e41f5`), and the four lines cited are of `main` at 80f414f (plan.md
     § Prerequisites 6). Neither version has an entry for a bad-pixel map, the offset
     level or a flat label (owner approval 6).
  7. The constitution's "detection input is 8-bit grayscale"
     (`.specify/memory/constitution.md:106`) does not bind here: no imaging frame is
     detection input in this spec, and star detection is untouched.
- Q: Which of 030's and 031's interfaces does this spec build on? → A: These, each from
  `specs/031-imaging-frames-on-console/plan.md` § Interfaces as drafted, but for the
  last, which is from `specs/030-imaging-camera-link/plan.md`, and each read again in
  their code on `main` at 80f414f, now that both have landed: `Imaging::take`,
  `Imaging::ended`, `Imaging::set`, `Imaging::abort` and `Imaging::view`, with
  `ImagingView`, `FrameView`, `SettingsEdit` and `Refusal`
  (`crates/finder/src/imaging/mod.rs`, and `crates/finder/src/imaging/frames.rs` for
  `FrameView`, 031 US1 and US2); two things the series audit gave 031's trio, which
  031's plan.md now carries as well as its spec.md: the imaging camera's lease (031 US9,
  its plan.md § The camera lease), `Imaging::lease`, `Imaging::holder`, `Lease::set`
  and `Lease::take`, whose refusal is 409 `the imaging camera is held by {holder}`, in
  the shape `specs/033-focus-reading-and-aid/plan.md` § The camera lease quotes, and
  `FrameView::set_temp_c` (031's FR-009), the set temperature 031 keeps beside a frame
  for its file's `SET-TEMP` card (plan.md § The camera's lease, § Prerequisites 11);
  the saved file
  `frames/<UTC date>/<id>.fits` and its cards, read with `fits::decode`
  (`crates/finder/src/imaging/fits.rs`, 031 US2 and US3); the route table
  (`crates/finder/src/imaging/routes.rs`, 031 US1); the Imaging page
  (`crates/finder/web/src/components/ImagingPage.tsx`, 031 US7); the MCP tools' place
  (`crates/finder/src/mcp/tools.rs:264`); and `SimImagingCamera` with its numbered steps
  for making a frame (`crates/simulator/src/imaging.rs`, 030 US2). 030 and 031 landed
  with these as named here but for a few in name, path or place, which are corrected
  (plan.md § Prerequisites 2).
- Q: Which of 032's and 033's interfaces does this spec build on? → A: Of 032, from
  `specs/032-imaging-solve-and-aim-point/plan.md` § Interfaces as drafted, and read
  again in its code on `main` at 80f414f: the imaging
  view's `relation` and its `angle_deg`, the imaging angle the console last measured,
  present only while the session's aim point is the measured one
  (`crates/finder/src/imaging/relation.rs`, 032 US3 to US5); and, in one scenario
  (US10-S2, which was US7-S6 until round 1's cut), 032's solving rig,
  `SimulatedSolver`, `AppState::with_imaging_solver` and
  `POST /api/imaging/solve` (032 US2). Of 033, from
  `specs/033-focus-reading-and-aid/plan.md` § The aid on the console, and read again in
  its code on `main` at 80f414f: the purpose label `focus`
  (`crate::imaging::focus::run::PURPOSE`) of every imaging frame its focus aid takes,
  which `match` passes over (owner decision 22), and the aid's holder name
  `the focus aid` (`crate::imaging::focus::run::HOLDER`), which a set meets only as the
  lease's refusal. 033 is in `depends_on_landed` because the plan document settles the
  order 030, 031, 032, 033, 034, and because it writes files this spec writes; what was
  read in its draft, and again in its code, is plan.md § Prerequisites 5.
- Q: What does 031 leave to this spec? → A: One thing. 031's `Imaging::take` always
  saves a file, so every dark and flat of a set is kept as an ordinary imaging frame
  (owner decision 2). Before 030 was split in two, the one trio's owner decision 24 also
  handed the camera's other controls, white balance among them, to this spec; the plan
  document's amendment 2 settles that instead, and 030's owner decision 13 now records
  it: 030's INDI adapter sets white balance neutral each time it connects, and no spec
  records those controls per frame (owner decision 17, settled).
- Q: Who else takes imaging frames, and who holds the camera meanwhile? → A: The
  observer's own `POST /api/imaging/frame` (031), 033's focus aid, and later 035's stack
  and 037's run. The plan document's amendment 1 settles who holds the camera: 031's
  imaging state holds one lease naming its holder, a second taker is refused with the
  holder's name, and each later spec takes and releases that lease and adds no lock of
  its own. A set of darks, and each step of a set of flats, holds it under the name
  `a set of darks or flats` (FR-042, FR-031; owner decision 23, settled).
- Q: Where does this trio depart from the plan document? → A: Nowhere now. It departed
  in four places when this session was held, each an owner decision below, and the
  plan document now says each as this trio has it: three in its § Interfaces shared
  across the specs, the fourth in its paragraph on this spec.
  1. Its module list names `calibration.rs` with the two child modules
     `calibration/library.rs` and `calibration/service.rs`, which this trio adds so that
     no story's diff passes 64 KiB (owner decision 21).
  2. It describes `fits.rs` as writing and reading 8- and 16-bit FITS, to which "034
     adds 32-bit real images for masters": a master is a mean (owner decision 3).
  3. Its route table gives `DELETE /api/imaging/calibration/{id}` as "remove a master,
     or stop or drop the set with that id", so the route also stops a set that is being
     taken and drops flats that wait for their darks (owner decision 10).
  4. It says darks are "refused when the frames are not dark, unless the observer says
     to take them anyway": this trio's refusal and its way past it, the request key
     `anyway`, for an observer who knows the telescope is covered (owner decision 4).

  The library's key is not among them: a master flat was then matched by the plan
  document's key as it was written (owner decision 7). The owner's answer of 2026-11-17
  widens the flat's part of that key, a departure he approves, and the plan document now
  writes the wider key (§ Clarifications, the session of 2026-11-17).
- Q: Which numbers here come from a source, and which are design values? → A: The
  research note gives four things (`docs/research/2026-11-15-imaging-camera.md` § 9.
  Calibration frames): one stacker maps the pixels of a master dark that are more than
  3.5 σ hot; a flat's histogram should keep little above 80 or below 30 percent, inside
  which this spec's 40 to 50 percent sits; one maker's routine accepts darks on their
  mean level against a low threshold; and one maker suggests retaking darks beyond
  8 °C. Every other number is a design value of this spec, named as a constant in
  plan.md § Interfaces and listed for the owner in owner decisions 4 to 15 and 24. None
  was measured on a camera.
- Q: Which other drafts write the same files? → A: All of them. 032 and 033 landed before
  this spec and are in its `depends_on_landed`, so the factory, which runs up to four
  epics at once (`ergane.yaml:196`), could not dispatch this one beside either. 035 to 037 land
  after it and name it in theirs. The files are `crates/finder/src/imaging/mod.rs`,
  `crates/finder/src/imaging/routes.rs`, `crates/finder/src/mcp/tools.rs`,
  `crates/finder/README.md`, the Imaging page and the simulator's imaging camera
  (plan.md § Prerequisites 5).

### Session 2026-11-15 (a second agent's review of this trio)

- Q: What did the review change? → A: Ten things, each checked against the plan
  document, the drafts of 030, 031 and 032 and the research note before it was taken.
  1. The stories were renumbered: the library story was split into the library on disk
     (US3) and the library in the console (US4), so the old US4 to US8 are US5 to US9.
  2. 032 and 033 joined `depends_on_landed`, the plan document's settled order, and the
     imaging angle is read from 032's view where an earlier copy gave a setter no spec
     called (owner decision 20).
  3. A master flat is matched by the plan document's key as it is written; the wider
     match is now the alternative, shown as a departure (owner decision 7). The owner's
     answer of 2026-11-17 reverses this: the wider match is settled, and the plan
     document's first key is the alternative (the session of that day).
  4. The dark test gained a third part, because its level test passed an uncovered
     camera at this spec's own simulated settings, and a way past it, `anyway` (owner
     decision 4).
  5. The search for hot pixels in a sub gained a third test, because the first two took
     star cores for hot pixels at binning 2 (owner decision 14).
  6. A set of darks or flats now holds its frames to the exposure it asked for, fails on
     a sensor that has not reached its set temperature, and waits out a frame in flight
     before it sets the exposure back (FR-027, FR-042, FR-044). The series audit (the
     next session) replaced the waiting restore with the imaging camera's lease.
  7. A request made while a set runs is told that a set is running: the two taking
     routes look for a set before they look for a frame in flight (FR-024).
  8. `match` stays on the latest sub while later frames expose (owner decision 22).
  9. The refusal no longer says "calibration set", which the glossary avoids, and
     **Flat label** joined the glossary terms put to the owner (owner approval 6).
  10. The out-of-tries text of the flat search names the exposure that was taken
      (US7-S2), and a scenario that left its key sample unstated says it (US3-S5).
- Q: What did it find that this trio cannot settle? → A: Three things, each written then
  as a prerequisite that was not met: who records the camera's other controls (owner
  decision 17), who holds the imaging camera while frames are taken one after another
  (owner decision 23), and what 033, whose draft did not exist yet, writes to the shared
  files. All three are settled now: the first two by the plan document's amendments 2
  and 1, the third by reading 033's draft (the next session).

### Session 2026-11-15 (the series audit of 030 to 037, and the re-anchoring at 49bb210)

- Q: What did the audit change? → A: Seven findings, each checked against the plan
  document and the drafts of 030, 031, 033 and 035 before it was taken.
  1. **The lease** (amendment 1). A set no longer keeps a lock of its own:
     `Calibration::busy`, `Calibration::restore_exposure` and `RESTORE_TRIES` are gone,
     with the scenario that called the restore and the edge case of a frame that slips
     in between two of a set's frames. `take_darks` and `take_flats` keep their own
     refusals, look for a frame in flight only while nobody holds the lease, and then,
     last, take 031's lease under `HOLDER`, `a set of darks or flats`, passing a refusal
     of it on unchanged. The set's task owns the `Lease` from start to end: it sets
     every exposure and takes every frame through it, puts the observer's exposure back
     through it, and only then drops it and writes the set's final state. Flats that
     wait for their darks hold no lease; the darks step takes it again (FR-024, FR-031,
     FR-042, US6-S9; owner decision 23).
  2. **The shared writes** of plan.md § Refinement Record name 033's stories where 033
     writes a file after 030, 031 or 032.
  3. **White balance** (amendment 2): 030's INDI adapter sets it neutral each time it
     connects, and no spec records the camera's other controls; owner decision 17 is
     closed, with no `controls` map, no card and no test.
  4. **`match` passes over focus frames.** 033's focus aid saves its imaging frames with
     the purpose `focus` and removes each when the next is saved, so `match` follows the
     latest saved imaging frame whose purpose is none of `dark`, `flat` and `focus`
     (US4-S3, FR-017).
  5. **The frame's own set temperature.** The audit's finding on 031 gives `FrameView`
     the set temperature 031 keeps for the frame's file, and `FrameKey::from_view` reads
     `frame.set_temp_c`; owner decision 22 keeps only its first half.
  6. **033 is read**: plan.md § Prerequisites 5 holds the four answers.
  7. **Words**: the darks' refusals say `the darks are not dark`, and the search's
     `the flat's level does not rise with the exposure`, since the glossary's Imaging
     frame entry avoids "frame" alone (FR-025, FR-030).
- Q: Against what were the citations checked? → A: `main` at 49bb210, this worktree's
  HEAD, which 028's US5 moved past dbc2b5b in test files only. Every `path:line`
  of spec.md and plan.md was read there. One had moved: the MCP token test is now at
  `crates/finder/tests/mcp/motion.rs:517`. One was wrong at both commits: the `## API`
  table's last row was line 184 of `crates/finder/README.md`, line 167 being its
  heading. And the uncommitted glossary, to which the audit added entries, held hot
  pixel at line 331. The scratch-folder helper 028's US5 added is cited at
  `crates/finder/tests/support/mod.rs:12`. All were read again on `main` at 80f414f, at
  the re-anchoring, whose line numbers are cited: the `## API` heading is now
  `crates/finder/README.md:204` and its last row `crates/finder/README.md:229`, and hot
  pixel is at `GLOSSARY.md:341`.

### Session 2026-11-17 (the owner's answers)

- Q: What did the owner's answer to owner decision 7 change? → A: The match of a master
  flat. The owner chose the wider match, a departure from the plan document's key that
  he approves: a master flat matches a sub on the camera's model and serial, the frame's
  size and origin, bits, Bayer pattern and binning only, and gain, offset and set
  temperature no longer restrict it, since a master flat has its own dark taken off. A
  flat's exposure and sensor temperature are still kept with it and shown, the newest
  still wins, and a caller may still ask for one flat label. A master dark keeps its key
  as written, and owner decision 8's warning is unchanged. Changed here: owner decision
  7, FR-015, US3-S4 and two edge cases (a sensor still cooling; a camera with no gain,
  offset or temperature). In plan.md: § The library's **Matching**, the README's
  paragraph, the note on the page's flat line, § Decisions, § Prerequisites 3 and
  § Refinement Record (US3-S4's assertion, and what this answer changed). In tasks.md:
  the owner-decisions prerequisite and T012. And the plan document's paragraph on this
  spec, which now keys a master flat the same way. 035, 036 and 037 say only that a
  master may be missing, so they do not change.

### Session 2026-11-18 (round 1 probes)

- Q: What did the round-1 probe build find? → A: Three agents built every story in
  order, on top of 031's, 032's and 033's round-1 probe builds, with no ASTAP, INDI
  driver or real camera, and measured each story's diff as plan.md § Plan traps 3 says.
  Every story built inside its declared writes, and every existing test passed
  unchanged after each. They reported 51 defects. One was blocking: US7 measured 67,264
  bytes, 1,728 over the 64 KiB budget. The other 50 were clarity defects, most of them a
  scenario that a build broken on purpose still passed; each case the probes proposed
  for one was run on the probe's build, passed there and failed under the break. US6
  measured 61,351 bytes, and about 62,938 with its new cases. The lit test of US6-S3
  counted 1275 lit samples when US2's functions were called on the uncovered simulated
  frame and 1306 through the console's own set, against 101 allowed, where the trio had
  worked about 1200 with a model of the camera; and every worked number that rests on
  the simulator's noise or its seeded field held, the same in three runs (plan.md
  § Prerequisites 4).
- Q: How is US7 cut? → A: As the owner chose on 2026-11-18, by scenario. US7, "Take a
  set of flats", keeps US7-S1 to US7-S4 and the scripted camera: the route, the search,
  the two steps, their refusals and the master flat, whose entry carries
  `imaging_angle_deg` null. A new story, US10, takes the scenarios that were US7-S5 to
  US7-S7, now US10-S1 to US10-S3, on the simulated camera, in a module of its own,
  `calibration_flats_sim`, after `calibration_flats` in the test binary; its code is the
  imaging angle kept with the flats (`relation.angle_deg` read when the last flat is in
  hand, the entry's `imaging_angle_deg`), now FR-045, split out of FR-032. US7's module
  declares `pub(crate)` the helpers US10 imports. Every other story and scenario keeps
  its number, which is why the new story is 10. On US7's code US10-S1 and US10-S3
  already pass, and only US10-S2, the angle, fails first (tasks.md, US10's test task).
  US8 and US9 share no file with US10, so all three may run together after US7 (§ Work
  Graph). The probe built the cut and measured US7 at 54,871 bytes and US10 at 14,888;
  with round 1's cases folded in, US7 is about 60 KB, the nearest to the budget after
  US6 (plan.md § Risks). Owner decision 1's nine stories are ten so; its text is kept.
- Q: What was the owner's second answer? → A: That every assertion or case the three
  reports propose where a break survived goes into its scenario, in the report's words;
  those that extend a scenario of the old US7 go with that scenario after the cut, so
  US7-S3 keeps its two new cases.
- Q: Which assertions did the probes add or correct? → A: These, each where a build
  broken on purpose passed every test, or where a worked example was wrong:
  - US1-S3: a hot pixel is not vignetted; US1-S4 and US1-S5: under a region of
    interest the column pattern and V follow the sensor pixel; US1-S6: a sample whose
    centre lies on a dust shadow's rim is inside it;
  - US2-S1: the 8-bit mosaic is 2 × 2; US2-S4: the mono repair reads the samples one
    away, which now differ from those two away; US2-S5: the first test scales with the
    colour's noise; US2-S9: all eight samples that touch the 0.8-sample star are lit
    (the worked example said four), and two cases pin the share and the noises that
    make a sample lit;
  - US3-S3: `FrameKey::from_image` names the first card it lacks; US3-S4: a turn of
    exactly 1.0° is not warned of; US3-S5: a master of another size is not used, and
    the flat still is; US3-S6: an index of another format is unreadable, in its words;
  - US4-S1: the answers of the settings, the frame and the `DELETE` carry the whole
    calibration object; US4-S2: a console that is not ready answers the library's own
    view, its `error` included;
  - US5-S1: the map is row by row; US5-S5: `attach_bad_pixels` refuses an id that is no
    master dark, and a map of another format or size is not used, in its words;
  - US6-S3: its worked example gives round 1's count; US6-S4 and US6-S7: the exposure is
    put back after a set that ends at its offset frame; US6-S6: a `DELETE` before the
    set's first frame stops it by the stop flag; US6-S7: a frame at another exposure
    fails the set, in place of a sentence no test asserted, and the master's `temp_c` is
    the mean of the darks alone;
  - US7-S1: the flats' darks count in `darks_taken` while `taken` stays 3, and the master
    keeps the flats' sensor temperature; US7-S2: a bright first frame halves the
    exposure, and a search frame is held to the first; US7-S3: a setting changed
    between the two steps returns the set to `needs_darks`, and a flat's failed frame
    reads `a flat failed:`; US7-S4: the flats' darks are a set being taken, and the
    `DELETE` stops them;
  - US10-S1 (US7-S5 when probed): the hot pixel (600, 400), where a master flat whose
    darks were not subtracted shows;
  - US8-S2: `list_calibration` equals the route once the set is done; US8-S3: a
    malformed argument reads as the route's 400; US8-S4: without imaging the arguments
    are not read;
  - US9-S1: the block comes after 033's Focus block; US9-S2: each item
    holds a line and a button, and `Remove` deletes the master's id while a set runs.

  In the shared setup, **Waiting until** reads the route the condition is about, a
  scripted camera's frames may carry an exposure, and **Rigs are written once** names
  US7's helpers for US10.
- Q: Which behaviours did the probes settle where the plan left a choice? → A: These,
  each written where plan.md gives the behaviour: the library's `calibrate` words a
  master of another size `it is {W} × {H} and the sub {w} × {h}`, uses no map of
  another format or size, and still names in the selection a master it could not use;
  `remove` deletes a map's file from US3 on, so US5 adds nothing to it;
  `Calibration::remove` takes `self: &Arc<Self>` and reaches the blocking pool through
  `spawn_blocking`; a view that is not `ready` is refused with 031's words written again
  in `service.rs`, since 031 keeps them inside `set` and `take`; a `Refusal` of another
  status is built from its public fields; `Mosaic::from_samples`'s `None` reads
  `its samples do not fit its size`; `solve` is among the answers `Imaging` builds its
  view for; the MCP tools work out the route's answer again and parse their arguments
  as 031's `set_imaging_camera` does; US8 makes the README count nine tools; the page's
  two count fields are `type="number"`, as 033's Focus frame time was on the probe base,
  and it lists the masters in the view's order; and US4's console rig is built over a
  `Simulator` its caller keeps.
- Q: What did the probes find that 031, 032 and 033 do otherwise? → A: On the probe base
  `Imaging` built its view in two places: `set`, `take`, `solve` and the focus run
  answered the imaging state's own view, and only `Imaging::view` the calibration's. On
  `main`, where 031 and 032 have landed, `set` and `take` answer `Imaging::view`, as
  `abort` and `solve` do, which is what plan.md § 031's imaging state asks. `main`'s
  `Refusal` has `bad_request` and `conflict` and public fields; 031's not-ready words are
  private to its imaging module; 031's `set_imaging_camera` parses its arguments with the
  route's own `sim_json`; and the probe base's Focus frame time of 033 was a
  `type="number"` field. 032's
  aim window gave US10-S2's three points on the probe code and on `main` alike. Every
  other seam built as written. Read again on `main` at 80f414f, where 033 has landed
  too: its `focus_start` and `focus_stop` answer `Imaging::view` as well, and its Focus
  frame time is a text field with `inputMode="decimal"`, no longer `type="number"`; the
  page's two count fields stay `type="number"` (plan.md § Prerequisites 2).
- Q: What is not folded? → A: One defect: the two `GLOSSARY.md` lines that plan.md
  § `crates/finder/README.md` and `GLOSSARY.md` cites for the **Flat** and
  **Calibration library** entries, which the probe base's glossary holds at other lines.
  In the drafting worktree they read as cited; the re-anchoring on `main` at 80f414f
  cites them at the lines that hold them there, `GLOSSARY.md:173` and
  `GLOSSARY.md:177` (plan.md § Prerequisites 2). The probes' notes on memory are in plan.md
  § Risks as measured and add no requirement.

### Session 2026-11-19 (the re-anchoring onto `main`)

- Q: What does a set do when the answer to its take carries no frame? → A: It fails
  the step, with 031's words for the camera's state (`a dark failed: …` or
  `a flat failed: …`, as every failed step is worded). On `main` the view that
  `Lease::take` answers is built once the lock is released, and it carries `frame` only
  while the camera is `ready`, so a link that drops between the take and its answer
  leaves no frame id. The owner chose this ending on 2026-11-19, as for 033's focus aid
  on 2026-11-18. The alternative, waiting for the camera and taking again, would need a
  scenario of its own. No scenario reaches the case (plan.md § **A set's frame**,
  step 1).
- Q: Is the Calibration block the Imaging page's last? → A: No: it comes after 033's Focus
  block, and 035's Stack block follows it. 035's round-1 probe found that a test asserting
  the page's last region (`regions.at(-1)`) fails once 035's US9 adds its block, and an
  existing test may not be edited, so US9-S1 asserts the order after the regions
  `Imaging frame` and `Focus`, which still catches a block put before them. The owner
  chose this on 2026-11-19 (plan.md § The page).

## Owner decisions

Each is a design or taste call the plan document leaves open, or, where its heading says
so, one that departed from the plan document and that the plan document now writes as
recommended here (§ Clarifications, the second session). Two, 17 and 23, have since
been settled by the plan document's amendments and are kept, under their numbers, so
that every reference
to them still reads; they ask nothing of the owner. One, 7, has since been settled by the
owner, against this trio's recommendation, and its answer is written into the trio
(§ Clarifications, the session of 2026-11-17). None of the others is settled yet:
the owner settles each before the flip, and a different answer is written into the trio
and re-validated, never left to a story.

1. **Nine stories, where the brief suggested six.** Recommended: nine, so that each
   stays under 64 KiB: the simulator's additions are a story of their own, the library
   on disk and the library in the console (its state, its view and two routes) are two,
   and the MCP tools and the page block are two. Instead: six, with the simulator's
   additions inside the first story, one story for the whole library, and the page and
   the tools in one; a story found too large mid-epic then needs a relaunch.
2. **Every dark and flat of a set is kept as an ordinary imaging frame.** Recommended:
   a set takes its frames through 031's frame path (`Lease::take`, which is
   `Imaging::take` for the lease's holder), which saves each under
   `frames/<UTC date>/`, and reads them back to average them. Nothing in 031's path
   changes, and a set can be averaged again with another program. The cost is disk: a set
   of 20 at the worked example's full resolution is about 650 MB
   (`docs/research/2026-11-15-imaging-camera.md` § 11. Frame transport). Instead: a path
   in 031 that takes a frame without saving it, which 031 does not have.
3. **A master is a 32-bit real FITS file, and the library's list is one JSON file** (a
   departure once, when the plan document's `fits.rs` was 16-bit; it now says "034 adds
   32-bit real images for masters"). Recommended: `encode_real` and
   `decode_real` in `crates/finder/src/imaging/fits.rs`, one image of `BITPIX` −32, read
   by any astronomy program; `index.json` beside the masters. 031's `decode` still
   refuses `BITPIX` −32, as its test pins. Instead: masters rounded to 16 bits, which
   adds a quarter of a count of noise to every sub.
4. **The dark test, and the way past it** (a departure once, when the plan document said
   only "refused when the frames are not dark"; it now adds "unless the observer says to
   take them anyway"). Recommended: a set first takes one exposure of the camera's shortest
   exposure, its offset frame, whose median is the offset level and whose spread is the
   noise. Three things then refuse the set:
   - an offset level above 20 percent of full scale (daylight on an open tube);
   - a dark whose median is more than twice the noise above the offset level;
   - a dark in which more than one sample in 10,000, and more than 8 samples, stand more
     than 8 noises above their colour's median without being hot pixels. A covered
     sensor's bright samples are hot pixels, which stand alone; stars and leaking light
     spread over samples that touch.

   The third is there because the level alone passes an uncovered camera whenever the
   sky adds less than two noises in the exposure, as it does on this spec's own simulated
   camera at gain 0 and 200 ms (US6-S3). The request key `anyway: true` takes the set
   whatever the three say, and the master's entry then says `checked` false. It is there
   because a refusal of real darks must not end a night's calibration: a warm sensor at
   a long exposure reads above its offset level although covered, the plan document's
   rule 6 forbids assuming subs of a few seconds, and whether a camera's shortest
   exposure reads the level of a multi-second dark is not established
   (`docs/research/2026-11-15-imaging-camera.md` § 13. Not established). Every number
   here is a design value; whether the owner's real darks pass at his exposures is a
   field check. Instead: no `anyway` key, a departure from the plan document's sentence
   as it now reads; no third test; other numbers.
5. **How many frames a set may hold.** Recommended: 3 to 64, the same number for a flat
   set's flats and for their darks. Instead: other bounds.
6. **When a master dark matches a sub.** Recommended: the same camera model and serial,
   frame size and origin, bits, Bayer pattern and binning; gain and offset each equal
   within 0.001; exposure within 1 ms; and the set temperatures within 1.0 °C. A dark
   taken with no set temperature matches a sub with none when their sensor temperatures
   are within 8.0 °C, the figure one maker gives for retaking darks
   (`docs/research/2026-11-15-imaging-camera.md` § 9. Calibration frames), or when
   either has no reading. A cooled dark never matches an uncooled sub. Among several,
   the nearest in temperature wins, then the newest. Instead: other tolerances.
7. **When a master flat matches a sub.** **Settled by the owner on 2026-11-17: the wider
   match.** A master flat matches a sub with the same camera model and serial, frame
   size and origin, bits, Bayer pattern and binning, and on nothing else: gain, offset
   and set temperature do not restrict it. A master flat is a ratio with its own dark
   taken off, so gain, offset and temperature do not change it, and flats made once
   serve every gain, with the cooler on or off. A flat's exposure and its sensor
   temperature are kept with it and shown, and do not restrict the match either. The
   newest wins; a caller may ask for one flat label only. This is a departure from the
   plan document's key that the owner approves, and the plan document now writes it.
   Instead, the plan document's key as it was first written, "camera, gain, offset,
   binning, set temperature and, for darks, exposure", which this trio recommended until
   then: gain and offset each equal within 0.001 as well, and the set temperatures
   within 1.0 °C, or neither frame with one. Its cost: flats taken at one gain, or with
   the cooler off, serve no sub taken at another gain or with the cooler on, and the
   match says `flat` is missing.
8. **A camera turned in the focuser.** Recommended: when the imaging angle is known now
   and was known when the flats were taken, and the two differ by more than 1.0°, the
   match carries the difference as a warning and the flat is still applied. At the worked
   example's corner 1° is 52 pixels, about the width of a dust shadow (inferred). When
   either angle is unknown there is no warning and the flat label is all the observer has
   to go by; the angle is known only while the session's aim point is the one the
   console measured (owner decision 20). Instead: a turned flat counts as missing;
   another angle.
9. **Finding a flat's exposure.** Recommended: the level of a flat is the largest of its
   colours' medians; the aim is 45 percent of full scale, accepted from 40 to 50; full
   scale is the largest value a sample of the frame's bits can hold; the search starts
   at the exposure in force, takes at most 8 frames, follows a straight line through its
   last two, and never asks for more than 30 s. If the owner's camera clips below that
   full scale the search fails and says where the level stopped rising; that is a field
   check. Instead: a full-scale setting; other numbers.
10. **Flats are taken in two steps** (the `DELETE` route's second and third uses were a
    departure once, when the plan document listed it for removing a master; its route
    table now reads "remove a master, or stop or drop the set with that id").
    Recommended: `POST /api/imaging/calibration/flats` with a count and a label finds the
    exposure and takes the flats with the panel lit; the flats then wait, in memory,
    until the same route is sent `{"darks":true}` with the telescope covered.
    `DELETE /api/imaging/calibration/{id}` with a waiting or running set's id drops or
    stops it. Flats that wait are lost when the console restarts; their files stay.
    Instead: the flats' darks first, at an exposure the observer enters; or a master
    flat less the offset level alone, which the glossary's "less its own dark" rules out.
11. **A dark is subtracted without clipping at zero.** Recommended: a calibrated sub is
    real numbers and keeps values below zero, so that a faint background's mean is not
    pushed up. Instead: clip at zero as one open stacker does
    (`docs/research/2026-11-15-imaging-camera.md` § 9. Calibration frames).
12. **A master flat is normalised by each colour's median**, the two greens together.
    Recommended. Instead: by the largest value, as the same open stacker does; or each
    of the four positions of the Bayer cell by itself.
13. **The bad-pixel map.** Recommended: the samples of a master dark that are more than
    3.5 times its spread above its median, the spread being the median less the 16th
    percentile and at least one count; hot samples only; and no map at all when it would
    hold more than 1 percent of the frame. Instead: other numbers; dead samples from a
    master flat as well.
14. **Hot pixels found in the sub itself.** Recommended: when no bad-pixel map applies, a
    sample is taken as hot when three things hold. It stands more than 8 times the noise
    above the largest of its four same-colour neighbours. It stands more than 5 times as
    far above its colour's median as that neighbour does. And at most one of the eight
    samples that touch it is lit, a sample being lit when it stands above its own
    colour's median by more than 3 noises and by more than a tenth of what the hot
    candidate stands above its own. The third test is what tells a star from a hot
    pixel at any sampling: the first two compare samples two apart, which a star at
    binning 2, or on a telescope with a coarser scale, passes as a hot pixel does, while
    a star always lights the samples that touch its peak and a hot pixel does not. The
    limits, in samples of the frame as delivered, whatever its binning: a star at least
    1.4 samples across at half its height is kept; a narrower one, or one whose peak is
    under about 30 noises, can lose its brightest sample; two hot pixels that touch are
    both found, three or more that touch are left for the bad-pixel map; and inside
    nebulosity a hot pixel is found only when it stands more than ten times as far above
    the frame's median as the nebula around it does (plan.md § Interfaces). All five
    numbers are design values. Instead: other numbers; the first two tests alone, which
    is safe only where stars are at least 2.6 samples across; no repair without a map.
15. **A flat too dark to divide by.** Recommended: where a master flat is under 0.1 the
    sample is not divided and is repaired as a bad pixel. Instead: another floor.
16. **The two taking tools answer at once.** Recommended: `take_darks` and `take_flats`
    answer when the set has started, and the client reads its progress with
    `list_calibration`; a set of 20 ten-second darks runs for minutes. An MCP client
    cannot stop a set or remove a master: the plan document names three tools. It can
    send `anyway`, since a tool passes through to the route's own action. Instead: wait
    for the set; a `stop` key.
17. **The camera's other controls, white balance among them.** Settled by the plan
    document's amendment 2: where a camera offers white-balance controls, 030's INDI
    adapter sets them neutral each time it connects, so every frame of every night is on
    the same footing for calibration, and no spec records them per frame. This spec
    therefore has no `controls` map in `FrameKey` or in `MasterEntry`, no card for them
    in a master's file and no test of them; it changes nothing on the scope node or the
    scope link (owner approval 10), and those controls are in no frame the console
    receives. The line this gives the plan document's Deferred list ("recording a
    camera's other controls with each imaging frame, and matching masters on them") is
    there now, written by the series audit's shared edit of that document, not by this
    trio.
18. **No cover or flat panel under `finder --simulate`.** Recommended: the simulator's
    cover and panel are controls for tests. At a desk `Take darks` is refused as not
    dark, by the level or by the stars in the simulated field, which is the refusal
    working. Instead: a simulate-only route, `POST /api/sim/imaging`, that switches them,
    so the page's whole flow can be rehearsed at a desk.
19. **No calibrated picture in this spec.** Recommended: `calibrate` has no route of its
    own; 035's stack is its first caller, and the page says which masters the latest
    imaging frame would get. The plan document's field check, a calibrated sub compared
    with the raw one, then waits for 035. Instead: one more route that answers the latest
    imaging frame calibrated as a JPEG, and a switch for it under the page's picture.
20. **Where the imaging angle comes from.** Recommended: 032 lands before this spec, as
    the plan document orders them, and carries the imaging angle in the imaging view as
    `relation.angle_deg`. The console reads it there: when the last flat of a set is in
    hand, for the master's entry, and whenever it works out a match, for the warning.
    There is no setter and nothing for a later spec to wire up. The angle is present only
    while the session's aim point is the one the console measured, so on a night with a
    tapped aim point flats record no angle and nothing warns. Instead: depend on 030 and 031
    alone and give the console a setter that 035 calls, which leaves the warning dead
    until 035 lands and lets the factory run this epic beside 032 or 033.
21. **Three source files** (a departure once, from the plan document's one; its module
    list now names all three). Recommended:
    `crates/finder/src/imaging/calibration.rs` for the arithmetic, with
    `calibration/library.rs` for the files and the match and `calibration/service.rs`
    for the console's state and the sets. Instead: one file.
22. **What `match` describes.** Recommended: the latest imaging frame the console has
    seen saved whose purpose label is none of `dark`, `flat` and `focus`: 033's focus
    aid labels its imaging frames `focus`, takes them about once a second and removes
    each when the next is saved, so a match on one would name a file about to go. It
    stays while later frames are exposing, or are darks, flats or focus frames, so that
    the page does not lose it for most of every exposure once 035 takes subs one after
    another. A frame's real size is known only from the frame (030 FR-003), so the
    console does not guess a match from the settings alone. The frame's set temperature
    is the frame's own, `frame.set_temp_c` of 031's view, the one 031 keeps for the
    file's `SET-TEMP` card (§ Clarifications, the fourth session; that half of this
    decision is closed). Instead: a match worked out from the settings in force; a match
    only while the very latest frame is a saved sub; a match on focus frames too.
23. **Who holds the imaging camera while frames are taken one after another.** Settled
    by the plan document's amendment 1: 031's imaging state holds one lease naming its
    holder, a second taker is refused with the holder's name, and each later spec takes
    and releases that lease and adds no lock of its own. A set of darks, and each of the
    two steps of a set of flats, takes it last of all its checks under `HOLDER`,
    `a set of darks or flats`, sets every exposure and takes every frame through it,
    puts the observer's exposure back through it, and only then drops it (FR-042,
    FR-031). So the observer's `POST /api/imaging/frame`, a settings change, 033's focus
    aid, 035's stack and 037's run are refused while a set is taken, with the set's
    holder name; a set is refused while the focus aid, a stack or a run holds the
    camera, with that holder's name, and while the observer's own single frame, which
    holds no lease, is in flight, with `an exposure is in progress`. Flats that wait for
    their darks hold nothing, so the camera is free between the two steps.
24. **A sensor that has not reached its set temperature.** Recommended: a set fails when
    one of its frames reads a sensor temperature more than 1.0 °C from the set
    temperature it was taken under, and says so; `anyway` does not pass this, because
    the master would be kept under a set temperature it was not made at. The observer
    waits for the cooler, or sets a temperature it can hold. This is inferred from how a
    cooler behaves; the research note does not cover settling. Instead: no check;
    another tolerance; waiting for the sensor inside the set.

## Owner approvals

Flipping this spec to `ready` approves the settled answer to every owner decision above,
and the following. Each is named in the FR or success criterion that makes it.

1. **Four routes and one part of the imaging view**: `GET /api/imaging/calibration` and
   `DELETE /api/imaging/calibration/{id}` (FR-018, FR-019),
   `POST /api/imaging/calibration/darks` (FR-024) and
   `POST /api/imaging/calibration/flats` (FR-029), the request key `anyway` on both
   `POST` routes (FR-043; owner decision 4), and the `calibration` object of
   `GET /api/imaging` and of `imaging` in `GET /api/session`, which the MCP tools that
   return either carry too (FR-017).
2. **Three MCP tools**: `list_calibration`, `take_darks` and `take_flats`, the last two
   taking `anyway` as the routes do (FR-034, FR-035).
3. **Three new source files**, `crates/finder/src/imaging/calibration.rs`,
   `crates/finder/src/imaging/calibration/library.rs` and
   `crates/finder/src/imaging/calibration/service.rs` (owner decision 21), and these
   edits to files 031 creates: `ImagingView` gains `calibration`, `Imaging` gains
   `root` and `attach_calibration`, `AppState::with_imaging` builds the calibration state
   and `AppState` gains `calibration()` (FR-017); `fits.rs` gains `RealImage`,
   `encode_real` and `decode_real` (FR-012); `ImagingPage.tsx` renders the block
   (FR-037). The console's calibration state has one public name beyond the requests,
   the constant `HOLDER` of `crates/finder/src/imaging/calibration/service.rs`,
   `a set of darks or flats`: the holder name a set takes 031's lease under, which
   031's refusals then show to every other taker and which 035's and 037's words may
   quote (FR-042; owner decision 23).
4. **The simulator's new public names**: `HotPixel`, `SensorPattern`, `FlatPanel`,
   `DustShadow` and `LightPath`, exported from `crates/simulator`, and
   `Simulator::sensor_pattern`, `set_sensor_pattern`, `light_path` and `set_light_path`
   (FR-001).
5. **The calibration library's folder and files**, which later nights read:
   `calibration/index.json` (format `finder-calibration/1`),
   `calibration/darks/<id>.fits`, `calibration/darks/<id>.badpixels.json` (format
   `finder-bad-pixels/1`) and `calibration/flats/<id>.fits` under the imaging folder,
   with the cards of plan.md § The library, `DARKTEST` among them (FR-013, FR-022).
6. **Three new `GLOSSARY.md` terms** (FR-023, FR-028, FR-033), with these words:

   > **Bad-pixel map**:
   > The samples of an imaging camera's sensor that read high whatever the light, found in a master dark and kept with it. A sub's bad pixels are replaced from neighbours of their own colour before the sub is turned into colour.
   > _Avoid_: defect map, hot-pixel list, cosmetic correction

   > **Offset level**:
   > What a sample reads with no light and no time to gather any: the median of the **offset frame**, a dark of the camera's shortest exposure that the console takes before the darks it means to average. Those darks are checked against it.
   > _Avoid_: bias (alone), bias frame, pedestal, black level

   > **Flat label**:
   > The short name the observer gives the flats taken through one optical train, kept with their master flat.
   > _Avoid_: flat name, profile, train (alone)

   "Calibration set" stays a term the glossary avoids: this spec's routes, page and
   documents say "a set of darks" and "a set of flats".
7. **`DESIGN.md`: one component entry, Calibration block**, after the Imaging page entry
   that 031 adds, with the words of plan.md § `DESIGN.md`, and its entry in
   `.impeccable/design.json` (FR-039). No rule of `DESIGN.md` changes: the block holds no
   picture and uses existing tokens only.
8. **Document sections**: `## Calibration frames` in `crates/finder/README.md`, four rows
   of its API table and three lines of its MCP tool list (FR-020, FR-028, FR-033,
   FR-036), and four lines of the route list in `crates/finder/src/lib.rs`.
9. **Edits to existing test files** (SC-002), and no others: one `mod` line for each new
   finder test module, in alphabetical order, in `crates/finder/tests/console/main.rs`
   (`mod calibration;`, `mod calibration_bad_pixels;`, `mod calibration_console;`,
   `mod calibration_darks;`, `mod calibration_flats;`, `mod calibration_flats_sim;`,
   `mod calibration_library;`) and in
   `crates/finder/tests/mcp/main.rs` (`mod calibration;`), as 027's layout asks of every
   new finder test.
10. **No dependency, no scope-link change and no config key.** Nothing under
    `crates/scope-node` or `crates/scope-link` changes, and no `Cargo.toml` or
    `Cargo.lock`.

## User Scenarios & Testing *(mandatory)*

Every scenario shares these definitions unless it says otherwise.

- **The gate command.** Every Rust scenario runs in
  `cargo nextest run --workspace --all-targets --all-features --locked`, the `test`
  gate's first command, each test in its own process.
- **Paused time.** Every Rust test here that waits runs under
  `#[tokio::test(start_paused = true)]`, so every wait is in virtual time. A test of a
  pure function is a plain `#[test]`.
- **The sim rig** is 030's: a `Simulator` over the test database at the site 40° N 75° W
  on a `VirtualClock` from Unix 1768528800 (`2026-01-16T02:00:00Z`), given
  `push_to(50.0, 120.0)`, as `crates/finder/tests/support/live.rs:41` builds it, and a
  `SimImagingCamera::new(sim.clone())` over it. **sim** below is that `Simulator`, which
  the test keeps: it switches the cover, the panel and the patterns on it.
- **Counts on the simulated camera.** With gain 0 applied one electron is 0.25 counts,
  and with offset 10 in force the offset level is 160 counts
  (`specs/030-imaging-camera-link/plan.md` § The simulated imaging camera). The sky adds
  20 electrons a second to every sample, 5 counts a second at gain 0. With noise on and
  gain 0, a covered frame's samples have a standard deviation of 0.80 counts (worked
  example: 3 electrons of read noise at 0.25, and rounding).
- **H** is three hot pixels, `HotPixel { x, y, electrons_per_s }`: (100, 50, 4000.0),
  (101, 51, 2000.0) and (600, 400, 40000.0). At gain 0 they add 1000, 500 and 10000
  counts for each second of exposure.
- **V and D.** With the sensor W × H, `corner_light` k and the centre of sample (x, y) at
  (x + 0.5, y + 0.5): V(x, y) = 1 − (1 − k)·((x + 0.5 − W/2)² + (y + 0.5 − H/2)²) /
  ((W/2)² + (H/2)²), and D(x, y) is the product, over the dust shadows whose centre is
  within its radius of the sample's centre, of 1 − depth. A test computes both itself.
- **A sample's colour** in an `RGGB` frame: red at even x and even y, blue at odd x and
  odd y, green otherwise.
- **The console rig** is 031's console rig: the sim rig's `Simulator` with a
  `SimMount::new` connected first and `push_to(50.0, 120.0)` after; a `SimNode` over it
  with a 500 ms frame interval and the token `test-token`, served over a fresh duplex for
  each connection with `memory::serve_routes_on` and `node.service().routes()`
  (`SimNode` has no `routes()` of its own; its `service()` is at
  `crates/simulator/src/node.rs:76`), as
  `crates/finder/tests/support/mcp.rs:228` serves its node; a session paired to it with
  the site 40° N 75° W; then `AppState::shared` with `with_session`, `with_clock` on the
  simulator's clock, `with_link`, `with_live()` and `with_imaging(R)`. **R** is the child
  path `imaging` of the test's scratch folder. The simulated camera starts with exposure
  1000 ms, gain 120, offset 10, binning 1, the whole 1236 × 822 sensor and the cooler
  off. A console-rig scenario first waits until `GET /api/imaging` has `state` `ready`.
- **The scripted camera** is an `ImagingCamera` of the tests' own that wraps 030's
  stand-in camera, `SyntheticImagingCamera` (64 × 48, 16 bits, `RGGB`, exposure 1 to
  60000 ms, gain 100 and offset 10 in force): `describe`, `status`, `apply` and `abort`
  pass through; `expose` passes through, notes the exposure it was asked for in a list
  the test can read, and then replaces every sample of the frame with one value,
  `level(n, exposure_ms)` rounded half away from zero and clamped to 0 to 65535, where
  `level` is a function the scenario gives and n counts that camera's exposures from 1.
  Where a scenario says a scripted camera's frames "carry" a gain, a sensor
  temperature or an exposure, it also overwrites that field of the frame it answers, by
  a second function of n the scenario gives; where a scenario says its nth `expose` answers
  `Failed`, it answers that in place of a frame. **The scripted console rig** is 031's
  stand-in console rig with the scripted camera in the stand-in camera's place: the
  console rig paired instead to a `NodeService::new` over a `SyntheticCamera` with the
  token `test-token`, given `.with_imaging(the scripted camera)`, whose clocks read the
  simulator's `VirtualClock`. A scenario that needs several sets builds a rig for each.
- **Rigs are written once.** The FITS reader of a test's own, the console rig, the
  request helper for `DELETE` and a foreign `Origin`, the wait, the scripted camera, the
  scripted console rig and US7's flats helpers are each needed by more than one module
  of `console`. Each is written in the first module whose story needs it, declared
  `pub(crate)`, and imported by the later modules with `use crate::<module>::…`: the
  FITS reader in `calibration_library` (US3); the console rig, the request helper and
  the wait in `calibration_console` (US4); the scripted camera and the scripted console
  rig in `calibration_darks` (US6); and the flats helpers, a settings request, a request
  on a calibration route, the waits for a set's state and for a set to settle, a
  comparison of numbers, a master's file read and the three route paths, in
  `calibration_flats` (US7), for US10. A module declares only what its own tests use,
  since an unused helper fails `clippy -D warnings`, and a later story adds nothing to an
  earlier story's module.
- **The solving rig** is 032's: the console rig with
  `with_imaging_solver(Arc::new(SimulatedSolver::new(sim.clone())))` added after
  `with_imaging(R)`, a session with no target and no aim point, and the simulated
  imaging camera turned 17° from the finder camera's frame, read as
  `sim.imaging_optics().angle_deg`. **Solving a frame** is `POST /api/imaging/solve` and
  waiting until `GET /api/imaging` has `solve.frame_id` equal to that frame's id and a
  `solve.state` other than `solving`, as 032's shared setup has it.
- **F** is full scale, the largest value a sample can hold: 65535 for 16 bits.
- **Scratch folders.** A test's files live under the directory cargo names in
  `CARGO_TARGET_TMPDIR`. A finder test module takes its folder from
  `crate::support::Scratch::new(name)`, the helper 028's US5 put in
  `crates/finder/tests/support/mod.rs:12`, which makes the folder, names it by its
  public field `.0`, and removes it when dropped.
- **The lease** is 031's (031 US9; plan.md § The camera's lease):
  `state.imaging().unwrap().holder()` is the name of whoever holds the imaging camera,
  or `None`; `state.imaging().unwrap().lease(name)` takes it for the test, or answers a
  `Refusal` of status 409 whose `message` contains the holder's name; dropping the
  `Lease` gives it back. **HOLDER** is `finder::imaging::calibration::service::HOLDER`,
  `a set of darks or flats`. A scenario asserts only that a refusal is 409 and contains
  the holder's name, never 031's whole sentence.
- **Requests** go through the router in-process, as `request` in
  `crates/finder/tests/support/live.rs:90` sends them, with `Content-Type:
  application/json` and no `Origin`. A scenario that names an `Origin`, another
  `Content-Type` or the method `DELETE` builds its request with the request helper of
  **Rigs are written once**, with a `Host` of localhost on port 8080. A foreign `Origin`
  is `http://elsewhere.example`.
- **Waiting until** a condition means reading, every 100 ms, the route the condition
  is about (`GET /api/imaging` for an imaging frame's state, its `status` or its
  `settings`, otherwise `GET /api/imaging/calibration`) until it holds, for at most
  600 s of virtual time, and failing otherwise. **The set** is the `set` of
  `GET /api/imaging/calibration`'s body, and **the masters** its `masters`.
- **Reading files in a test.** A test reads a saved imaging frame as 031's shared setup
  says, and a master with code of its own too, never through the console's FITS code:
  80-byte cards up to `END`, the header padded to 2880 bytes, then, for `BITPIX` −32,
  big-endian IEEE 754 single-precision numbers, row by row from the top.
- **Comparing numbers.** Two real numbers are equal when they differ by at most 0.001,
  unless a scenario gives another tolerance. Counts of things and texts are exact.
- **The mean of files.** The mean of some saved imaging frames is taken by the test,
  sample by sample, from the files it read.
- **Oracles.** Each scenario names its oracle: a worked example given here, the
  simulator's known pattern (H, V, D, the column pattern, the cover and the panel), the
  scripted camera, or files the test read itself.

### User Story 1 - A simulated sensor with its own pattern, a cover and a flat panel (Priority: P1)

As a test of any calibration story, I can give the simulated imaging camera hot pixels at
known places, a column pattern, vignetting and a dust shadow, cover it, and light a flat
panel in front of it, and I know exactly what each adds to a frame.

**Why this priority**: Every later story's oracle is one of these known patterns. The
real camera's own are unknown until the owner takes real frames.

**Independent Test**: `crates/simulator/tests/imaging_calibration.rs`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p simulator --locked --test imaging_calibration`.

**Acceptance Scenarios**:

1. **Given** the sim rig, **Then** a committed test asserts that `sim.sensor_pattern()`
   is `SensorPattern::default()`, with no hot pixel and `column_amplitude` 0.0, and
   `sim.light_path()` is `LightPath::default()`, with `cover` false, no `flat_panel`,
   `corner_light` 1.0 and no `dust`. **When** two cameras built alike with noise off, no
   stars (`with_stars(vec![])`) and gain 0 each take a 1000 ms frame, the second after
   `sim.set_sensor_pattern(SensorPattern::default())` and
   `sim.set_light_path(LightPath::default())`, **Then** the committed test asserts that
   every sample of both is exactly 165 and the two frames are equal byte for byte (worked
   example: 160 of offset and 20 electrons of sky at 0.25).
2. **Given** the sim rig with `cover` true, noise off, gain 0 and one white star of
   magnitude 5 at the truth's centre, **When** a test takes a 1000 ms frame, **Then** a
   committed test asserts that every sample is exactly 160. **Given** the same with noise
   on and no stars, **Then** over all samples the mean is 160.0 ± 0.05 and the standard
   deviation is between 0.76 and 0.85 (worked example: √(0.75² + 1/12) = 0.804).
3. **Given** the sim rig with the hot pixels H, `cover` true, noise off and gain 0,
   **Then** a committed test asserts that:
   - a 1000 ms frame reads 1160 at (100, 50), 660 at (101, 51) and 10160 at (600, 400),
     and exactly three of its samples are not 160;
   - a 500 ms frame reads 660, 410 and 5160 there;
   - with `corner_light` 0.5 as well, a 1000 ms frame still reads 1160 at (100, 50), 660
     at (101, 51) and 10160 at (600, 400) (worked example: a hot pixel's charge is the
     sensor's own, added after the light and its vignetting);
   - with `cover` false and no stars, a 1000 ms frame reads 1165 at (100, 50) and 165 at
     (0, 0);
   - at binning 2 with `cover` true, a 1000 ms frame is 618 × 411 and reads 410 at
     (50, 24), 285 at (51, 25) and 2660 at (300, 200), and exactly three of its samples
     are not 160 (worked example: each binned sample is the mean of four of one colour,
     so 160 + 1000/4, 160 + 500/4 and 160 + 10000/4).
4. **Given** the sim rig with `column_amplitude` 14.0, `cover` true and noise off,
   **Then** a committed test asserts that every sample (x, y) of a 1000 ms frame at gain
   0 is exactly 160 + 2·(x mod 8) − 7; that a 100 ms frame at gain 200 is the same frame;
   and that at binning 2 the samples of row 0 at columns 0 to 4 are 155, 157, 163, 165
   and 155 (worked example: the pattern adds 14·((x mod 8)/7 − 0.5) counts to sensor
   column x, after the gain, and binned column 2 is the mean of sensor columns 4 and 6);
   and that with the region x 4, y 0, width 16, height 2, every sample (x, y) of a
   1000 ms frame at gain 0 is 160 + 2·((x + 4) mod 8) − 7, so (0, 0) reads 161 (worked
   example: the pattern follows the sensor column).
5. **Given** the sim rig with `flat_panel`
   `FlatPanel { electrons_per_s: 40000.0, colour: [0.8, 1.0, 0.6] }`, `corner_light` 0.5,
   noise off, gain 0 and one white star of magnitude 5 at the truth's centre, **When** a
   test takes a 1000 ms frame, **Then** a committed test asserts, each within 1 count,
   that it reads 10160 at (618, 411), 8160 at (618, 410), 6160 at (619, 411), 4167 at
   (0, 0) and 3166 at (1235, 821); and that a 500 ms frame reads 5160 at (618, 411)
   (worked example: 160 + 0.25·40000·seconds·colour·V, with V 1.0000 at the centre and
   0.5009 at the corners; the panel takes the place of the sky and the stars, which would
   have added 5 counts and the star). **Given** a white `flat_panel` of 40000 electrons a
   second, `corner_light` 0.5, noise off, gain 0 and the region x 600, y 400, width 40,
   height 30, **Then** a 1000 ms frame reads 10156 at its (0, 0), 10160 at (18, 11) and
   10153 at (39, 29), each within 1 count (worked example: V is taken at the sensor
   pixels (600, 400), (618, 411) and (639, 429), not at the region's).
6. **Given** the sim rig with a white panel of 40000 electrons a second, `corner_light`
   1.0, `dust` `[DustShadow { x: 300.0, y: 200.0, radius_px: 10.0, depth: 0.3 }]`, noise
   off and gain 0, **When** a test takes a 1000 ms frame, **Then** a committed test
   asserts that it reads 7160 at (300, 200) and at (309, 200), and 10160 at (310, 200)
   and at (300, 215) (worked example: the centre of (309, 200) is 9.51 from the shadow's
   centre and that of (310, 200) is 10.51). **Given** the same with the shadow at
   x 300.5, y 200.5, **Then** a 1000 ms frame reads 7160 at (310, 200) and 10160 at
   (311, 200) (worked example: the centre of (310, 200) lies exactly on the rim, which is
   inside). **Given** instead no panel, no dust,
   `corner_light` 0.5, no stars, noise off and gain 0, **Then** a 4000 ms frame reads 180
   at (618, 411) and 170 at (0, 0) (worked example: 20 counts of sky, halved at the
   corner). **Given** the panel and the dust of the first case with `cover` true, noise
   off and gain 0, **Then** every sample of a 1000 ms frame is exactly 160.

---

### User Story 2 - Calibrate a sub on its mosaic (Priority: P1)

As the stack of spec 035, I hand over one sub's samples and whatever masters match it,
and get back the sub with its dark subtracted, its flat divided out and its bad pixels
repaired from neighbours of their own colour, in that order, with a report of which steps
ran and which were missing. With no master at all it still finds and repairs hot pixels
in the sub itself.

**Why this priority**: It is the one thing 035 calls. Everything else in this spec makes
or keeps what it takes in.

**Independent Test**: `crates/finder/tests/console/calibration.rs`, the module
`calibration` of finder's `console` test binary. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test console calibration::`.

**Acceptance Scenarios**:

1. **Given** `finder::imaging::calibration`, **Then** a committed test asserts that
   `Mosaic::from_samples` of the bytes `A0 00 88 13 FF FF 00 00`, 2 × 2, 16 bits, `RGGB`,
   has the samples 160.0, 5000.0, 65535.0 and 0.0, with `colour` 0 at (0, 0), 1 at (1, 0)
   and (0, 1), and 2 at (1, 1), `colours()` 3 and `step()` 2; that for `GRBG` the colours
   at (0, 0), (1, 0), (0, 1) and (1, 1) are 1, 0, 2, 1, for `BGGR` 2, 1, 1, 0 and for
   `GBRG` 1, 2, 0, 1; that the 8-bit bytes `00 7F FF 10`, 2 × 2 with no pattern, give
   0.0, 127.0, 255.0 and 16.0 with every colour 0, `colours()` 1 and `step()` 1; that 7
   bytes for a 2 × 2 16-bit frame, and `bits` 12, give `None`; and that `percentile` of
   `[5.0, 1.0, 4.0, 2.0, 3.0]` at 0.5 is 3.0, of `[1.0, 2.0, 3.0, 4.0]` at 0.5 is 2.0
   and at 0.16 is 1.0, and of no values is `None` (worked example: the value at index
   ⌊p·(n − 1)⌋ of the values sorted).
2. **Given** an 8 × 8 `RGGB` sub whose every sample is 560.0, a dark of 64 samples of
   160.0, a flat of 64 samples of 0.5 and an empty bad-pixel map, **Then** a committed
   test asserts that `calibrate` with all three makes every sample 800.0 and reports
   `dark` `applied`, `flat` `applied`, `bad_pixels` `map`, `repaired` 0, `not_repaired`
   0 and `low_flat` 0 (worked example: (560 − 160)/0.5; the flat first would give 960);
   and, in each of these with no map given, so that `bad_pixels` is `sub` and `repaired`
   0: that with the dark alone every sample is 400.0 and `flat` is `missing`; with the
   flat alone 1120.0 and `dark` is `missing`; with neither 560.0 and both are `missing`;
   and that a sub of 150.0 with that dark alone becomes −10.0, not 0.
3. **Given** a 4 × 4 `RGGB` mosaic whose green samples are all 4000.0, whose red samples
   at (0, 0), (2, 0), (0, 2) and (2, 2) are 2000.0, 2200.0, 1800.0 and 2000.0, and whose
   blue samples at (1, 1), (3, 1), (1, 3) and (3, 3) are 1000.0, 1000.0, 500.0 and
   1500.0, **Then** a committed test asserts that `normalise_flat` answers the medians
   2000.0, 4000.0 and 1000.0 and leaves the reds 1.0, 1.1, 0.9 and 1.0, every green 1.0
   and the blues 1.0, 1.0, 0.5 and 1.5; and that `calibrate` of a 4 × 4 `RGGB` sub of
   1000.0 with that flat and an empty map gives 1000.0, 909.091, 1111.111 and 1000.0 at
   the reds, 1000.0 at every green and 1000.0, 1000.0, 2000.0 and 666.667 at the blues
   (worked example: each colour is divided by its own median, so the panel's colour does
   not reach the sub).
4. **Given** an 8 × 8 `RGGB` mosaic whose red samples are 100.0, green 200.0 and blue
   300.0, **Then** a committed test asserts of `repair`, each on a fresh copy:
   - with (4, 4) set to 5000.0 and the list `[(4, 4)]`, (4, 4) becomes 100.0 and the
     answer is (1, 0);
   - with (0, 0) set to 7000.0 and the list `[(0, 0)]`, (0, 0) becomes 100.0 (worked
     example: its two neighbours inside the frame, (2, 0) and (0, 2));
   - with (2, 4), (6, 4), (4, 2) and (4, 6) set to 140.0, 100.0, 120.0 and 180.0 and the
     list `[(4, 4)]`, (4, 4) becomes 130.0 (worked example: the mean of the middle two
     of four);
   - with the same and (6, 4) set to 9000.0 and the list `[(4, 4), (6, 4)]`, (4, 4)
     becomes 140.0, the median of its three good neighbours, and (6, 4) becomes 100.0,
     the mean of (6, 2) and (6, 6);
   - with (3, 4) set to 900.0 and the list `[(3, 4)]`, (3, 4) becomes 200.0, from the
     green samples two away;
   - with the list `[(4, 4), (4, 4), (99, 99)]`, the answer is (1, 0);
   - on a 4 × 4 `RGGB` mosaic with all four reds in the list, no sample changes and the
     answer is (0, 4);
   - on a 5 × 5 mono mosaic of 50.0 with (2, 2) set to 900.0, (2, 0), (0, 2), (4, 2) and
     (2, 4) set to 70.0, and the list `[(2, 2)]`, (2, 2) becomes 50.0, from the samples
     one away.
5. **Given** a 16 × 16 `RGGB` mosaic whose red samples are 100.0, green 200.0 and blue
   300.0, with (6, 6) set to 400.0; (10, 10) set to 4100.0 and its four red neighbours
   two away to 1100.0; and (10, 2) set to 6100.0 and its four red neighbours two away to
   1100.0, **Then** a committed test asserts that `find_hot_pixels` answers exactly
   `[(10, 2), (6, 6)]`; that `calibrate` with no master and no map reports `dark`
   `missing`, `flat` `missing`, `bad_pixels` `sub` and `repaired` 2, leaves (6, 6) at
   100.0 and (10, 2) at 1100.0, and leaves (10, 10) at 4100.0 (worked example: the
   colour's median is 100 and its noise the floor of 1; (10, 10) stands 4000 above the
   median and its neighbour 1000, a ratio of 4, under 5; (10, 2) stands 6000 above
   against 1000, over 5; and no sample that touches (6, 6) or (10, 2) stands above its
   own colour's median, so the third test passes both); and that `calibrate` of the same
   mosaic with an empty map changes nothing and reports `bad_pixels` `map` and
   `repaired` 0. **Given** the 16 × 16 mosaic of US2-S9 with the reds of rows 0 and 8 set
   to 90.0 and (8, 4) raised by 60.0, **Then** `find_hot_pixels` answers empty (worked
   example: the red median is 100.0 and its 16th percentile 90.0, a noise of 10, so
   (8, 4) stands 60 above its neighbours two away, under 8 noises).
6. **Given** an 8 × 8 `RGGB` sub of 1000.0 and a flat of 1.0 but for 0.05 at (4, 4) and
   `f32::NAN` at (2, 2), **Then** a committed test asserts that `calibrate` with that flat
   and an empty map leaves every sample 1000.0 and reports `low_flat` 2 and `repaired` 2
   (worked example: a division would have made (4, 4) 20000).
7. **Given** an 8 × 8 sub and a dark of 63 samples, **Then** a committed test asserts that
   `calibrate` answers a `SizeMismatch` that displays exactly
   `the master dark holds 63 samples and the sub 64`, and leaves the sub as it was; and
   the same for a flat of 65 samples, with `the master flat holds 65 samples and the
   sub 64`.
8. **Given** the sim rig with H, `column_amplitude` 14.0, `corner_light` 0.5 and the dust
   shadow of US1-S6, its camera built `.with_stars(vec![])` with noise on and gain 0,
   **When** a test takes a 10000 ms frame and calibrates it with masters of its own
   making, a dark of 160 + 2·(x mod 8) − 7 plus 10000, 5000 and 100000 at H, a flat of
   V·D, and the three places of H as the map, **Then** a committed test asserts that the
   report is `applied`, `applied`, `map`, `repaired` 3, `not_repaired` 0 and `low_flat`
   0; that the medians of the samples in the 100 × 100 block at (0, 0), in the 100 × 100
   block at (568, 361) and in the 10 × 10 block at (295, 195) are each 50.0 ± 3.0; that
   before calibration the first two of those medians differ by more than 15; and that
   each of the three samples at H equals the median of its four same-colour neighbours
   as the test computes it from the calibrated mosaic (oracle: the simulator's known
   patterns; 200 electrons of sky are 50 counts, halved toward the corner until the flat
   restores them).
9. **Given** a 16 × 16 `RGGB` mosaic whose red samples are 100.0, green 200.0 and blue
   300.0, **Then** a committed test asserts of `find_hot_pixels`, each on a fresh copy:
   - with 2000·exp(−((x − 8)² + (y − 8)²)/1.28) added to every sample (x, y), a star 0.8
     samples wide centred on (8, 8), the answer is empty, and `calibrate` with no master
     and no map reports `repaired` 0 and leaves (8, 8) at 2100.0 (worked example: (8, 8)
     stands 2000 above its median and its red neighbours two away 87.9, a ratio of 22.8,
     so the first two tests pass; but the four samples beside it stand 915.7 above their
     medians and the four at its corners 419.2, each more than a tenth of 2000, so all
     eight that touch it are lit);
   - with 2000·exp(−((x − 8)² + (y − 8)²)/0.5) added instead, a star 0.5 samples wide,
     the answer is empty (worked example: the four beside it stand 270.7 above);
   - with (4, 4) raised by 5000.0 and (5, 5) by 2000.0, the answer is exactly
     `[(4, 4), (5, 5)]` (worked example: each has one lit sample touching it, which the
     rule allows);
   - with every sample within 3.0 of (8, 8), (8, 8) included, raised by 50.0 and (8, 8)
     by 5000.0 more, the answer is exactly `[(8, 8)]` (worked example: each sample that
     touches it stands 50 above its median, above 3 noises but under a tenth of 5050, so
     none is lit);
   - with (4, 4) raised by 20.0 and (5, 4) and (4, 5) by 2.5, the answer is exactly
     `[(4, 4)]` (worked example: a tenth of 20 is 2, but 2.5 is under 3 noises of 1, so
     neither is lit);
   - with (4, 4) raised by 5000.0, (5, 4) by 3000.0 and (5, 5) by 2000.0, the answer is
     empty (worked example: each has two lit samples touching it).
10. **Given** `finder::imaging::calibration`, **Then** a committed test asserts that
    `lit_samples` is 21 for the mosaic of US2-S9's first star (worked example: the
    samples with (x − 8)² + (y − 8)² of 0, 1, 2, 4 or 5 stand more than 8 above their
    medians, 1 + 4 + 4 + 4 + 8 of them, and none is a hot pixel); 0 for the same
    background with only (6, 6) set to 400.0; 0 for US2-S9's pair; 9 for the mosaic of
    US2-S5 (worked example: eleven samples stand more than 8 above their medians and two
    of them are hot pixels); and 0 for a 16 × 16 `RGGB` mosaic of 160.0.
11. **Given** the sim rig with H set on sim, its camera built with noise on and
    `.with_stars` of one star of magnitude 9.0 and `colour` `[1.0, 1.0, 1.0]` at the
    truth's centre for mid-exposure, with gain 0 and binning 2 applied, **When** a test
    takes a 1000 ms frame, **Then** a committed test asserts that the frame's largest
    sample is above 5000 and is not at (50, 24), (51, 25) or (300, 200); that
    `find_hot_pixels` of the frame's mosaic answers exactly
    `[(50, 24), (51, 25), (300, 200)]`; and that `calibrate` with no master and no map
    reports `bad_pixels` `sub` and `repaired` 3 and leaves that largest sample as it was
    (oracle: the simulator's H at US1-S3's binned places, and a star the test placed; by
    the first two tests alone the star's brightest samples, whose same-colour neighbours
    are four sensor pixels away, would be taken for hot pixels).

---

### User Story 3 - A calibration library on the console's disk (Priority: P1)

As the observer, the masters I make stay on the console's disk from night to night, each
with the camera and settings it was made with. For any imaging frame the library finds
the master dark and master flat that match it, says what is missing, and warns when the
camera has turned since the flats were taken.

**Why this priority**: A master made once in daylight serves many nights only if it is
kept and matched by rule.

**Independent Test**: `crates/finder/tests/console/calibration_library.rs`, the module
`calibration_library` of `console`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test console calibration_library::`.
Every scenario here is a plain `#[test]` on a scratch folder: none builds a console.

**Acceptance Scenarios**:

1. **Given** `finder::imaging::fits`, **Then** a committed test asserts that
   `encode_real` of a 2 × 2 `RealImage` with the samples 1.0, −2.5, 0.0 and 160.25 and
   the cards `IMAGETYP` text `MASTER DARK` and `NFRAMES` integer 4 is 5760 bytes; that,
   read as the shared setup says, its cards are in order `SIMPLE` T, `BITPIX` -32,
   `NAXIS` 2, `NAXIS1` 2, `NAXIS2` 2, `IMAGETYP` `MASTER DARK`, `NFRAMES` 4, `END`, with
   no `BZERO` and no `BSCALE`; that its data begins
   `3F 80 00 00 C0 20 00 00 00 00 00 00 43 20 40 00`; that `decode_real` of those bytes
   equals the image; that `decode_real` of a 16-bit image `encode` wrote answers a
   `FitsError` displaying `not a 32-bit real FITS image: BITPIX 16`, and of those 5760
   bytes cut off 8 bytes after the header `the FITS data is short: 8 of 16 bytes`; and
   that 031's `decode` of the real image still answers `unsupported FITS BITPIX -32`.
2. **Given** `Library::open(D)`, D a child path of the scratch folder that does not exist,
   **Then** a committed test asserts that `masters()` is empty and `error()` is `None`;
   that `new_id(Kind::Dark, 1768528803.7)` is `dark-20260116T020003Z`; and that, after
   `add` of the master dark A of US3-S3 under that id with 8 samples of its own, 4 × 2:
   - D holds `darks/dark-20260116T020003Z.fits`, 5760 bytes, whose cards after `NAXIS2`
     are in order `IMAGETYP` `MASTER DARK`, `CALID` `dark-20260116T020003Z`, `NFRAMES`
     20, `DARKTEST` T, `EXPTIME` 5.0, `GAIN` 120.0, `OFFSET` 10.0, `XBINNING` 1,
     `YBINNING` 1, `XORGSUBF` 0, `YORGSUBF` 0, `CCD-TEMP` -10.0, `SET-TEMP` -10.0,
     `BAYERPAT` `RGGB`, `INSTRUME` `simulated imaging camera`, `SERIALNO` `SIM-0001`,
     `ROWORDER` `TOP-DOWN` and `DATE` `2026-01-16T02:00:03.700`, and whose data is the 8
     samples;
   - D holds `index.json`, which the test parses: `format` is `finder-calibration/1` and
     `masters` is one object equal to `serde_json::to_value` of the entry `add`
     answered, whose `file` is `darks/dark-20260116T020003Z.fits` and whose `checked` is
     true;
   - D holds no file whose name ends `.part`;
   - a second `Library::open(D)` has the same `masters()`;
   - `new_id(Kind::Dark, 1768528803.2)` is now `dark-20260116T020003Z-2`; and a second
     `add` under the first id answers an error of kind `AlreadyExists` and leaves the
     first file's bytes as they were.
3. **Given** a library holding the master dark **A**: model `simulated imaging camera`,
   serial `SIM-0001`, 4 × 2, origin (0, 0), 16 bits, `RGGB`, gain 120.0, offset 10.0,
   binning 1, set temperature −10.0, sensor temperature −10.0, exposure 5000.0 ms, 20
   frames, `checked` true, made at 1768528803.7, and the frame key **K** with the same
   values, **Then** a committed test asserts of `select(&key, None, None)`:
   - K gives `dark` A's id, `flat` `None`, `bad_pixels` `None` and `missing`
     `["flat", "bad_pixels"]`;
   - K with the set temperature −10.9, and K with the exposure 5000.9, each give A;
   - each of these gives no dark and `missing` `["dark", "flat", "bad_pixels"]`: K with
     the set temperature −11.1; with no set temperature; with gain 121.0; with no gain;
     with offset 11.0; with binning 2; with exposure 5001.1; with no serial; with the
     serial `SIM-0002`; with the model `another camera`; with width 6; with origin
     (2, 0); with no Bayer pattern; with 8 bits;
   - with a second master **B** like A but with no set temperature and a sensor
     temperature of 20.0, K with no set temperature and a sensor temperature of 27.9
     gives B, with 28.1 gives no dark, and with no sensor temperature gives B;
   - with **A2** like A but set −10.5 and made 1000 s later, and **A3** like A and made
     2000 s later, K gives A3 and K with the set temperature −10.4 gives A2 (worked
     example: the nearest set temperature, then the newest);
   - and of `FrameKey::agrees`, the rule that holds a set's frames to its first:
     `K.agrees(&K, 5000.0)` is true, and so are K with the sensor temperature 3.0
     against K, and `K.agrees(&K, 5000.9)`; `K.agrees(&K, 5001.1)` is false, and so,
     against K at 5000.0, are K with gain 121.0, with no gain, with offset 11.0, with
     binning 2, with the set temperature −10.5, with no set temperature, with the serial
     `SIM-0002`, with width 6 and with origin (2, 0);
   - and that `FrameKey::from_image` of a 4 × 2 16-bit `FitsImage` with no cards answers
     exactly `the imaging frame's file has no INSTRUME card`.
4. **Given** a library holding the master flat **F1**, like A but of kind flat with the
   label `desk`, the imaging angle 17.0 and made at 1768528803.7, **Then** a committed
   test asserts that K gives `flat` F1's id; that K with exposure 2.0, with the sensor
   temperature 20.0, with gain 121.0, with no gain, with offset 11.0, with the set
   temperature −11.1 and with no set temperature each give F1 (worked example: a flat's
   gain, offset, set temperature, exposure and sensor temperature do not restrict it);
   that K with binning 2, with no serial, with the serial `SIM-0002`, with the model
   `another camera`, with width 6, with origin (2, 0), with no Bayer pattern and with 8
   bits each give no flat; that in a library holding only **F3**, like F1 but with no
   set temperature and a sensor temperature of 20.0, K gives F3, and so does K with no
   set temperature and a sensor temperature of 35.0 (worked example: a flat taken with
   the cooler off serves a sub taken with it on); that with **F2**, like F1 but labelled
   `corrector`, with gain 0.0 and made 1000 s later, K gives F2 (worked example: the
   newest wins, whatever its gain), `select(&K, Some("desk"), None)` gives F1 and
   `select(&K, Some("other"), None)` gives none; and that with F1 alone
   `select(&K, None, Some(17.9))` has no `flat_turned_deg`,
   `select(&K, None, Some(18.0))` has no `flat_turned_deg` (worked example: a turn of
   exactly 1.0° is not above it), `select(&K, None, Some(19.5))` has `flat_turned_deg`
   2.5 and still gives F1, and, for a flat taken at 179.5, an angle now of −179.6 gives none and −178.4 gives 2.1 (worked
   example: the difference is taken the short way round, and warned of above 1.0°). A
   flat with no angle, or no angle now, gives none.
5. **Given** a library holding a 4 × 4 master dark of 160.0 but for 1160.0 at (2, 2), and
   a 4 × 4 master flat of 0.5, both matching the key K4, K with width 4 and height 4,
   **When** a test calls `calibrate(&K4, &mut sub, None, None)` on a 4 × 4 `RGGB` sub of
   560.0 but for 1560.0 at (2, 2), **Then** a committed test asserts that every sample
   is 800.0, that `selection.dark` and `selection.flat` are the two ids, that the report
   is `applied`, `applied`, `sub` with `repaired` 0, and that `problems` is empty. **Given**
   an empty library, **Then** the same call makes every sample 560.0, reports `missing`,
   `missing`, `sub` and `repaired` 1, and has `selection.missing`
   `["dark", "flat", "bad_pixels"]` (worked example: (2, 2) stands 1000 above its two
   in-frame red neighbours and above the red median, and nothing that touches it is lit,
   so it is found in the sub and replaced by the median of the two). **Given** the first
   library with the dark's file deleted by the test before any call, **Then** the
   report's `dark` is `missing`, `flat` is `applied`, and `problems` holds one line that
   begins `the master dark ` and contains ` could not be used: `. **Given** the first
   library with the dark's file replaced by the test with a 2 × 2 real image, and
   reopened, **Then** the report's `dark` is `missing`, `flat` is `applied`, and
   `problems` holds exactly one line that begins `the master dark ` and contains
   ` could not be used: `.
6. **Given** a library with A and F1, **Then** a committed test asserts that
   `remove` of A's id answers true, deletes its file, leaves F1 listed and in
   `index.json`, and that a second `remove` answers false. **Given** a folder whose
   `index.json` holds the text `not json`, **Then** `Library::open` has no masters and an
   `error()` that begins `the calibration index could not be read: `; and after one
   `add` the folder holds `index.json.unreadable` with the text `not json`, a readable
   `index.json` with one master, and `error()` is `None`. **Given** a folder whose
   `index.json` holds `{"format":"finder-calibration/2","masters":[]}`, **Then**
   `Library::open` has no masters and `error()` is exactly
   `the calibration index could not be read: its format is finder-calibration/2`.

---

### User Story 4 - The library in the console (Priority: P1)

As the observer, I can list the calibration library and remove a master from the page or
an MCP client's view of the console, and for the latest imaging frame I am told which
masters it would get and what is missing.

**Why this priority**: It is what puts the library within reach of the page, the tools
and the sets, and it is the seam the later stories extend.

**Independent Test**: `crates/finder/tests/console/calibration_console.rs`, the module
`calibration_console` of `console`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test console calibration_console::`.

**Acceptance Scenarios**:

1. **Given** the console rig, **Then** a committed test asserts that
   `GET /api/imaging/calibration` answers 200 and exactly `{"masters":[]}`, and that
   `calibration` in `GET /api/imaging` and in the `imaging` of `GET /api/session` equal
   it. **When** the test adds, through `state.calibration().unwrap().library()`, a master
   dark for the simulated camera's starting settings, 1236 × 822 with exposure 200.0 and
   samples of 160.0, applies `{"exposure_ms":200}` (the answer's `calibration` equals
   `GET /api/imaging/calibration` read after it, and so does that of the
   `POST /api/imaging/frame`), sends `POST /api/imaging/frame` and waits until the frame
   is saved, **Then** `masters` holds that one entry, equal to
   `serde_json::to_value` of what `add` answered, and `match` is exactly
   `{"frame":1,"dark":"<its id>","flat":null,"bad_pixels":null,"missing":["flat","bad_pixels"]}`.
   **When** it applies `{"gain":0}` and takes another frame, **Then** `match` is
   `{"frame":2,"dark":null,"flat":null,"bad_pixels":null,"missing":["dark","flat","bad_pixels"]}`.
   **When** it sends `DELETE /api/imaging/calibration/<its id>`, **Then** the answer is
   200 and equals `GET /api/imaging/calibration` read after it, with `masters` empty and
   `match.frame` 2, and R holds no file under `calibration/darks`; the same
   request again answers 404 and `no master or set has the id <its id>`; the ids `A`,
   `a%2Fb` and one of 65 letters answer 404 and touch no file; and the request with a
   foreign `Origin` answers 403.
2. **Given** a console built as the console rig but without `with_imaging`, **Then** a
   committed test asserts that `GET /api/imaging/calibration` answers 200 and exactly
   `{"masters":[]}`, that `DELETE /api/imaging/calibration/dark-20260116T020003Z` answers
   409 and `this console takes no imaging frames`, the same request with a foreign
   `Origin` answers 403, and `GET /api/imaging` is still exactly `{"state":"off"}`.
   **Given** the console rig with its node served by `into_server()` alone, so that the
   imaging state is `no_service`, **Then** `GET /api/imaging` has no `calibration` key
   and `GET /api/imaging/calibration` answers 200 with `masters`. **Given** the same
   console with R's `calibration/index.json` written as `not json` before it is built,
   **Then** `GET /api/imaging/calibration` answers `masters` `[]` and an `error` that
   begins `the calibration index could not be read: ` (worked example: the imaging state
   is not `ready`, so the route answers `view(None)`).
3. **Given** the console rig with `{"cooler_on":true,"set_temp_c":-10,"exposure_ms":200}`
   applied, **Then** a committed test asserts that `GET /api/imaging/calibration` has no
   `match` key; and, once one imaging frame is saved, that `match.frame` is 1 and that
   `FrameKey::from_image` of the saved file, which the test reads and passes through
   031's `decode`, agrees field by field with `FrameKey::from_view` of the imaging view,
   and that its model is `simulated imaging camera`, serial `SIM-0001`, size 1236 × 822,
   origin (0, 0), 16 bits, `RGGB`, gain 120.0, offset 10.0, binning 1, set temperature
   −10.0, sensor temperature −10.0 and exposure 200.0, and that the imaging view's
   `frame.set_temp_c` is −10.0. **When** the test then calls
   `state.imaging().unwrap().take("dark")` and waits until that frame is saved, **Then**
   `match.frame` is still 1. **When** it applies `{"exposure_ms":10000}`, sends
   `POST /api/imaging/frame` and reads the calibration while `status.exposing` is true,
   **Then** `match.frame` is still 1; and once that frame is saved it is 3. **When** it
   calls `state.imaging().unwrap().take("focus")`, the purpose of 033's focus aid, and
   waits until that frame, 4, is saved, **Then** `match.frame` is still 3; and **when**
   it calls `take("flat")` and waits until frame 5 is saved, `match.frame` is still 3.
   **Given** a new console rig whose first frame is taken with `take("dark")` and second
   with `take("focus")`, **Then** once both are saved there is still no `match` key
   (worked example: `match` describes the latest imaging frame seen saved whose purpose
   is none of `dark`, `flat` and `focus`, and stays until a newer one).

---

### User Story 5 - A bad-pixel map from each master dark (Priority: P2)

As the observer, every master dark I make also yields a bad-pixel map: the samples that
read high whatever the light. The console keeps it with the master and repairs those
samples in each matching sub.

**Why this priority**: Without it a stack on an alt-az mount carries coloured arcs. The
fallback of US2 works with no map, but a map from a master dark finds warm pixels a
single sub cannot.

**Independent Test**: `crates/finder/tests/console/calibration_bad_pixels.rs`, the module
`calibration_bad_pixels` of `console`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development,
`cargo nextest run -p finder --locked --test console calibration_bad_pixels::`.

**Acceptance Scenarios**:

1. **Given** a 20 × 20 master dark of 160.0 but for 163.6 at (3, 2), 163.4 at (7, 7) and
   400.0 at (5, 5), **Then** a committed test asserts that `bad_pixel_map` answers the
   pixels `[(3, 2), (5, 5)]`, `level` 160.0, `spread` 1.0 and `too_many` false (worked
   example: nothing spreads below the median, so the spread is the floor of one count
   and the line is 160 + 3.5); and with 1000.0 at (9, 1) and (2, 6) of a 20 × 20 master
   of 160.0, the pixels are `[(9, 1), (2, 6)]` (worked example: row by row from the
   top).
2. **Given** a 16 × 8 master dark whose sample (x, y) is 160 + 2·(x mod 8) − 7, but for
   200.0 at (9, 3), **Then** a committed test asserts the pixels `[(9, 3)]`, `level`
   161.0 and `spread` 6.0 (worked example: the median is 161 and the 16th percentile
   155; the line is 161 + 21 = 182, above the pattern's highest column, 167).
3. **Given** a 10 × 10 master dark of 160.0 with one sample of 1000.0, **Then** a
   committed test asserts one pixel and `too_many` false; with two samples of 1000.0, no
   pixels and `too_many` true (worked example: more than 1 percent of 100 samples); and
   with one sample of `f32::NAN`, that sample is in the map.
4. **Given** the sim rig with H, `cover` true and gain 0, its camera as built, **When** a
   test takes eight 1000 ms frames and averages them itself, **Then** a committed test
   asserts that `bad_pixel_map` of the mean answers exactly `[(100, 50), (101, 51),
   (600, 400)]` (oracle: the simulator's known places).
5. **Given** a library to which the 20 × 20 master dark of US5-S1 is added, matching a
   key K20, **When** a test calls `attach_bad_pixels` with that master's id and its
   `bad_pixel_map`, **Then** a committed test asserts that the entry it answers, and the
   one `masters()` and `index.json` then hold, have `bad_pixels` 2; that the folder holds
   `darks/<id>.badpixels.json` whose `format` is `finder-bad-pixels/1`, `width` 20,
   `height` 20, `level` 160.0, `spread` 1.0, `sigmas` 3.5 and `pixels` `[[3,2],[5,5]]`;
   that `select` gives `bad_pixels` 2 and `missing` `["flat"]`; that `calibrate` of a
   20 × 20 `RGGB` sub of 560.0 with 5000.0 at (3, 2) reports `bad_pixels` `map` and
   `repaired` 2 and leaves 400.0 at (3, 2) and at (5, 5); and that `remove` deletes the
   map's file too, and that `attach_bad_pixels` with a master flat's id, or with
   `dark-20260116T020009Z`, which no master has, answers an error of kind `NotFound`,
   writes no file and changes no entry. **When**, before that `remove`, the map's file is
   replaced with one whose `format` is `finder-bad-pixels/2`, or whose `width` is 10,
   and the library is reopened, **Then** `calibrate` of that sub reports `bad_pixels`
   `sub` and `problems` exactly
   `["the bad-pixel map of <id> could not be used: its format is finder-bad-pixels/2"]`,
   or
   `["the bad-pixel map of <id> could not be used: it is 10 × 20 and the sub 20 × 20"]`.
   **Given** the 10 × 10 master dark of US5-S3 with two samples of
   1000.0, added and attached the same way, **Then** the entry's `bad_pixels` is null, no
   map file is written, `select` has `missing` `["flat", "bad_pixels"]` and `calibrate`
   reports `bad_pixels` `sub`. A master dark that was only added, with no map attached,
   keeps `bad_pixels` null, as US3's scenarios have it.

---

### User Story 6 - Take a set of darks (Priority: P1)

As the observer with the telescope covered, I ask for a number of darks. The console
takes them at the exposure, gain, offset, binning and set temperature in force, checks
that they really are dark, averages them into a master dark, finds its bad-pixel map and
adds it to the library. If the darks are not dark it says so and adds nothing, unless I
tell it to take them anyway.

**Why this priority**: The master dark is the first master a stack needs, and it carries
the bad-pixel map.

**Independent Test**: `crates/finder/tests/console/calibration_darks.rs`, the module
`calibration_darks` of `console`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test console calibration_darks::`.

**Acceptance Scenarios**:

1. **Given** the console rig with H and `cover` true set on sim and
   `{"gain":0,"exposure_ms":200}` applied, **When** a test sends
   `POST /api/imaging/calibration/darks` with `{"count":4}` and waits until the set's
   `state` is `done`, **Then** a committed test asserts that:
   - `state.imaging().unwrap().holder()` was `None` before the request, is
     `Some(HOLDER)` as soon as the request has answered, and is `None` again once the
     set is `done`;
   - the answer was 200 with a `set` of `kind` `dark`, `state` `offset`, `wanted` 4,
     `taken` 0, `exposure_ms` 200.0 and an `id` of the form `dark-20260116T02MMSSZ`;
   - the finished set has `taken` 4, `offset_level` 160.0, `level` 160.0, no `error` and
     no `not_dark` key;
   - the masters are one entry with that `id`, `kind` `dark`, `frames` 4, `checked`
     true, `model` `simulated imaging camera`, `serial` `SIM-0001`, `width` 1236,
     `height` 822, `origin_x` 0, `origin_y` 0, `bits` 16, `bayer` `RGGB`, `gain` 0.0,
     `offset` 10.0, `binning` 1, `set_temp_c` null, `temp_c` 20.0, `exposure_ms` 200.0,
     `level` 160.0 ± 0.5, `bad_pixels` 3, `label` null and `file` `darks/<id>.fits`;
   - `settings.exposure_ms` in `GET /api/imaging` is 200.0 again;
   - R holds exactly five files under `frames/2026-01-16/`, `000001.fits` to
     `000005.fits`, each with `IMAGETYP` `DARK` and `PURPOSE` `dark`, the first with
     `EXPTIME` 0.001 and the others 0.2;
   - the master file under R's `calibration/` is 1236 × 822 reals, equal sample by
     sample to the mean of files 2 to 5, with the cards `IMAGETYP` `MASTER DARK`,
     `NFRAMES` 4, `DARKTEST` T and `EXPTIME` 0.2; it reads 360 ± 20 at (100, 50),
     260 ± 12 at (101, 51), 2160 ± 60 at (600, 400) and 160 ± 2 at (0, 0);
   - `calibration/darks/<id>.badpixels.json` has `pixels` `[[100,50],[101,51],[600,400]]`
     (oracle: the simulator's H, 200, 100 and 2000 counts in 0.2 s, and the files the
     test read; the three hot pixels are bright samples that stand alone, so no dark is
     refused for them).
2. **Given** the same with
   `{"gain":0,"exposure_ms":200,"binning":2,"cooler_on":true,"set_temp_c":-10}` applied,
   **When** a set of 3 is taken, **Then** a committed test asserts that the master's
   entry has `width` 618, `height` 411, `binning` 2, `set_temp_c` -10.0, `temp_c` -10.0
   and `frames` 3; that its file is 618 × 411 with `XBINNING` 2, `CCD-TEMP` -10.0 and
   `SET-TEMP` -10.0; and that its map's `pixels` are `[[50,24],[51,25],[300,200]]`
   (oracle: US1-S3's binned places).
3. **Given** the console rig as it starts, uncovered, **When** a test asks for 3 darks,
   **Then** a committed test asserts that the set becomes `failed` with `not_dark` true
   and an `error` that begins `the darks are not dark: one reads ` and ends
   `; cover the telescope`, with `offset_level` 160 ± 1, `level` 180 ± 2 and `taken` 0;
   that the masters are empty; that R holds exactly two files under `frames/`; and that
   `settings.exposure_ms` is 1000.0 (worked example: at gain 120 a count is 0.995
   electrons, so one second of sky adds 20 counts and the noise is 3; the line is
   160 + 2·3). **Given** a new console rig, uncovered, with
   `{"gain":0,"exposure_ms":200}` applied, **Then** the set becomes `failed` with
   `not_dark` true and an `error` that begins `the darks are not dark: one holds ` and
   ends ` bright samples that are not hot pixels; cover the telescope`, with
   `offset_level` 160 ± 1, `level` 161 ± 1 and `taken` 0, the masters are empty and R
   holds exactly two files under `frames/` (worked example: 0.2 s of sky adds one count,
   under the line of 160 + 2·1, so the level passes; but the simulated field's stars
   put about 1300 samples more than 8 noises above their medians, 1306 when round 1
   counted them through the console's set, and the frame's 1,015,992 samples allow
   101). **Given** a new rig with a white `flat_panel` of
   40,000,000 electrons a second and no cover, **Then** the set becomes `failed` with
   `not_dark` true and an `error` that begins
   `the camera is not covered: its shortest exposure reads ` and ends ` of 65535`, with
   `offset_level` between 39000 and 41000, and R holds exactly one file under `frames/`.
4. **Given** the scripted console rig, **Then** a committed test asserts, each on its own
   rig, with a set of 3:
   - `level` 160 for the first exposure and 162 after: the set is `done`, the camera was
     asked for the exposures 1.0, 1000.0, 1000.0 and 1000.0 ms, `offset_level` is 160.0,
     the master's `level` is 162.0, its `checked` true, its `model`
     `synthetic imaging camera` and its `serial` null, every sample of its file is
     162.0 and the file has `DARKTEST` T;
   - `level` 160, 162, then 163: the set is `failed` with `not_dark` true, exactly
     `the darks are not dark: one reads 163 against an offset level of 160; cover the telescope`
     and `taken` 1;
   - `level` 13108 for the first exposure: `failed` with `not_dark` true and exactly
     `the camera is not covered: its shortest exposure reads 13108 of 65535`, and the
     camera was asked for one exposure, and `settings.exposure_ms` is 1000.0 again
     (worked example: refused at its 1 ms offset frame, the set puts back the exposure it
     was asked at);
   - `level` 13107 for the first exposure and 13109 after: `done` (worked example: a
     uniform frame's noise is the floor of one count, so the line is two counts above
     the offset level; 20 percent of 65535 is 13107);
   - `level` 160 for the first exposure and 163 after, asked for with
     `{"count":3,"anyway":true}`: `done` with no `not_dark` key, `offset_level` 160.0,
     the master's `level` 163.0 and its `checked` false, and its file has `DARKTEST` F;
   - `level` 13108 for every exposure, asked for with `{"count":3,"anyway":true}`:
     `done`, the camera was asked for four exposures and the master's `checked` is
     false.
5. **Given** the console rig, **Then** a committed test asserts that
   `POST /api/imaging/calibration/darks` answers:
   - 400 and exactly `count must be a whole number from 3 to 64` for `{}`, `{"count":2}`,
     `{"count":65}` and `{"anyway":true}`; 400 with a JSON `error` for `{"count":2.5}`,
     `{"count":"4"}`, `{"count":4,"anyway":"yes"}` and `{"count":4,"force":true}`;
   - 403 for `{"count":4}` with a foreign `Origin`, and 415 for it sent as `text/plain`;
   - 409 and `an exposure is in progress` with an exposure of 10 s applied and a frame
     started through `POST /api/imaging/frame` 1 s before, `holder()` being `None` at
     that moment (worked example: 031's single frame holds no lease, so the set looks
     for the frame in flight itself);
   - 409 and `a set of darks or flats is already being taken` while a set of 3 at 10 s
     runs, asked 1 s after that set's `state` became `frames`, when its own dark is
     exposing (worked example: a set being taken is looked for before a frame in
     flight);
   - 409 and `this scope node has no imaging service` on the console of US4-S2 whose
     node is served by `into_server()` alone;
   - on a console built without `with_imaging`: 409 and
     `this console takes no imaging frames` for `{"count":4}` and for `{"count":2}`, 400
     for `{"count":"4"}`, and 403 for `{"count":4}` with a foreign `Origin` (worked
     example: the origin is judged first, then the body's shape, then whether the
     console takes imaging frames, then the count);
   and that none of the refused requests changes `set` or the masters.
6. **Given** the console rig with `cover` true and `{"exposure_ms":10000}` applied,
   **When** a test asks for 3 darks, waits until the set's `state` is `frames` and
   `status.exposing` in `GET /api/imaging` is true, and sends
   `DELETE /api/imaging/calibration/<the set's id>`, **Then** a committed test asserts
   that `holder()` was `Some(HOLDER)` just before the `DELETE`; that the answer is 200;
   that within 2 s the set's `state` is `stopped` with no `error` and the imaging view's
   `frame.state` is `aborted`; that `holder()` is then `None`; that the masters are
   empty; that `settings.exposure_ms` is 10000.0; and that the same `DELETE` again
   answers 404.
   **Given** a new rig and the same set, **When** the test sends
   `POST /api/imaging/abort` instead, **Then** the set's `state` becomes `stopped` within
   2 s. **Given** a new rig and the same set, **When** the test sends the `DELETE` as soon
   as the `POST` has answered, while the set's `state` is still `offset`, **Then** the
   answer is 200, and within 2 s the set is `stopped`, `holder()` is `None`, the masters
   are empty and `settings.exposure_ms` is 10000.0 (worked example: the `abort` may come
   before the set's first frame exists; the flag is what stops it).
7. **Given** the scripted console rig whose camera answers its third `expose` with
   `Failed("the camera fell over")`, **When** a set of 3 is asked for, **Then** a
   committed test asserts that it is `failed` with exactly
   `a dark failed: the exposure failed: the camera fell over`, `taken` 1 and no
   `not_dark` key. **Given** one whose frames carry `gain` 100.0 for its first two
   exposures and 150.0 after, **Then** the set is `failed` with exactly
   `the imaging settings changed during the set` and `taken` 1. In both the masters are
   empty and `settings.exposure_ms` is 1000.0. **Given** one with
   `{"cooler_on":true,"set_temp_c":-10}` applied whose frames carry a sensor temperature
   of −7.5, **Then** a set of 3, asked for with or without `anyway`, is `failed` with
   exactly `the sensor is at -7.5 °C, not yet at its set temperature -10.0 °C`, `taken`
   0 and one exposure asked for and `settings.exposure_ms` 1000.0; and with frames that
   carry −9.0, −9.2, −9.4 and −9.6 in turn, the set is `done` and its master has
   `set_temp_c` -10.0 and `temp_c` -9.4 (worked example: the mean of the three darks;
   the offset frame's −9.0 is not among them, which would give −9.3; and a frame may
   read up to 1.0 °C from the set temperature it was taken under). **Given** one whose
   frames carry the exposure they were asked for, but 1500.0 for the third, **Then** the
   set is `failed` with exactly `the imaging settings changed during the set` and
   `taken` 1 (worked example: the third exposure is the second dark; its `EXPTIME` of
   1.5 s is more than `EXPOSURE_TOLERANCE_MS` from the 1000 ms asked for, and `agrees`
   holds a frame to the exposure asked, not to the first frame's).
8. **Given** the console rig after a set of 3 as in US6-S1, **When** a test sends
   `POST /api/imaging/frame` and waits until the frame is saved, **Then** a committed
   test asserts that `match` is exactly
   `{"frame":5,"dark":"<the id>","flat":null,"bad_pixels":3,"missing":["flat"]}`.
   **When** it takes a second set of 3 with the same settings and then another frame,
   **Then** the masters are two and `match.dark` is the second set's id (worked example:
   of two that match, the newest).
9. **Given** the console rig with `cover` true and `{"exposure_ms":10000}` applied,
   **When** a test asks for 3 darks and waits until the set's `state` is `frames` and
   `status.exposing` in `GET /api/imaging` is true, **Then** a committed test asserts
   that `holder()` is `Some(HOLDER)`; that `POST /api/imaging/frame` and
   `PUT /api/imaging/settings` with `{"exposure_ms":1000}` each answer 409 with an
   `error` that contains `a set of darks or flats`; that
   `state.imaging().unwrap().lease("probe")` answers a `Refusal` of status 409 whose
   `message` contains `a set of darks or flats`; and that `settings.exposure_ms` is
   still 10000.0. **When** the test sends
   `DELETE /api/imaging/calibration/<the set's id>` and waits until the set is
   `stopped`, **Then** `holder()` is `None`. **Given** a new console rig with `cover`
   true and `{"exposure_ms":10000}` applied, **When** the test takes
   `let probe = state.imaging().unwrap().lease("probe").unwrap()`, awaits
   `probe.take("probe")` and waits 1 s, **Then** `POST /api/imaging/calibration/darks`
   with `{"count":3}` answers 409 with an `error` that contains `probe` and is not
   `an exposure is in progress`, and the calibration has no `set` key; **When** the test
   sends `POST /api/imaging/abort`, waits until the imaging view's `frame.state` is
   `aborted` and drops `probe`, **Then** the same request answers 200 with a `set` whose
   `state` is `offset`, and `holder()` is `Some(HOLDER)` (worked example: the set takes
   the lease last of its checks and holds it while it exposes; a frame in flight is
   looked for only while nobody holds the lease, and otherwise the lease's refusal is
   answered unchanged, naming its holder).

---

### User Story 7 - Take a set of flats (Priority: P2)

As the observer with a flat panel lit in front of the telescope, I ask for a number of
flats under a label. The console finds the exposure that puts the frame at 40 to 50
percent of full scale and takes the flats. Then I cover the telescope and ask for their
darks, and the console makes a master flat, normalised colour by colour, and adds it to
the library.

This story keeps no imaging angle with the flats: User Story 10, cut from it after
round 1, keeps it, and proves the master flat on the simulated camera. Until then a
master flat's `imaging_angle_deg` is null.

**Why this priority**: The flat evens the picture across a large sensor and removes dust
shadows. A stack is usable without one, which is why it follows the darks.

**Independent Test**: `crates/finder/tests/console/calibration_flats.rs`, the module
`calibration_flats` of `console`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test console calibration_flats::`.

**Acceptance Scenarios**:

1. **Given** the scripted console rig whose `level` is 160 + 80·exposure_ms, at most
   65535, until the test says the camera is covered and 160 after, with the scripted
   camera reporting `temp_c` 20.0 on every frame, with `{"exposure_ms":100}` applied,
   **When** a test sends
   `POST /api/imaging/calibration/flats` with `{"count":3,"label":"desk"}` and waits
   until the set's `state` is `needs_darks`, **Then** a committed test asserts that the
   answer was 200 with a `set` of `kind` `flat`, `state` `search`, `label` `desk` and
   `wanted` 3, and an `id` of the form `flat-20260116T02MMSSZ`; that the camera was
   asked for exactly the exposures 100.0, 200.0, 366.634, 366.634, 366.634 and 366.634
   ms (worked example: 8160 and 16160 counts give 80 counts a millisecond, and 45
   percent of 65535 is 29490.75, reached at 366.634 ms); that the set has `exposure_ms`
   366.634, `taken` 3, `level_share` 0.45 and no `error`; that
   `state.imaging().unwrap().holder()` was `Some(HOLDER)` as soon as the request had
   answered and is `None` now, as it is whenever flats wait; that the masters are
   empty; and that `settings.exposure_ms` is 100.0 again. **When** the test then covers
   the camera, sends `{"darks":true}` and waits until the set's `state` is `done`,
   **Then** the committed test asserts that:
   - the answer was 200 with `state` `flat_darks`, `holder()` was `Some(HOLDER)` as soon
     as it had answered and is `None` once the set is `done`, and the camera was asked
     next for 1.0, 366.634, 366.634 and 366.634 ms; and once `done` the set has
     `darks_taken` 3 and `taken` 3;
   - the masters are one entry with the set's `id`, `kind` `flat`, `label` `desk`,
     `frames` 3, `dark_frames` 3, `checked` true, `exposure_ms` 366.634, `level_share`
     0.45, `imaging_angle_deg` null, `gain` 100.0, `offset` 10.0, `width` 64, `height`
     48, `bad_pixels` null and `file` `flats/<id>.fits`, and the master's `temp_c` is
     20.0;
   - every sample of that file is 1.0, and it has the cards `IMAGETYP` `MASTER FLAT`,
     `NFRAMES` 3, `NDARKS` 3, `DARKTEST` T, `EXPTIME` 0.366634, `FLATLBL` `desk` and no
     `IMGANGLE` (the rig has no imaging solver, so the view carries no imaging angle);
   - R holds ten files under `frames/`, the first six with `IMAGETYP` `FLAT` and the
     last four `DARK`, and `settings.exposure_ms` is 100.0.
2. **Given** the scripted console rig, **Then** a committed test asserts, each on its own
   rig, of a request for 3 flats labelled `desk`:
   - with the `level` of US7-S1 and the console's starting exposure of 1000 ms, the
     exposures asked for begin 1000.0, 125.0, 250.0 and 366.634, and the set reaches
     `needs_darks` (worked example: a frame at 90 percent of full scale or more sends
     the search to an eighth of its exposure);
   - with `level` 160 + 40·exposure_ms and the starting 1000 ms, the exposures begin
     1000.0, 500.0 and 733.269, and the set reaches `needs_darks` (worked example:
     1000 ms reads 61.3 percent, below 90, so the second is half; the line through 40160
     and 20160 counts meets 29490.75 at 733.269 ms);
   - with `level` 160 + 0.01·exposure_ms and 100 ms applied, the exposures are 100.0,
     200.0 and 30000.0, and the set is `failed` with exactly
     `the flat is too dim: 30000 ms reads 0.7% of full scale; is the panel lit?`;
   - with `level` 65535, the exposures are 1000.0, 125.0, 15.625, 1.953 and 1.0, and the
     set is `failed` with exactly
     `the flat is too bright: 1 ms reads 100.0% of full scale; dim the panel`;
   - with `level` 20000 and 100 ms applied, the exposures are 100.0 and 200.0, and the
     set is `failed` with exactly
     `the flat's level does not rise with the exposure: 100 ms and 200 ms read 20000 and 20000`;
   - with `level` 19491 for odd n and 39491 for even n, and 100 ms applied, the
     exposures are 100.0, 200.0, 149.999, 174.999, 162.499, 168.749, 165.624 and
     167.186, and the set is `failed` with exactly
     `no exposure put the flat between 40 and 50% of full scale in 8 tries; the last, 167.186 ms, read 60.3%`
     (worked example: the text names the eighth exposure taken and its level, not the
     ninth the search would have asked for, 166.405 ms);
   - with the `level` of US7-S1, 100 ms applied and the camera reporting gain 150 from
     its second exposure, the exposures are exactly 100.0 and 200.0, and the set is
     `failed` with exactly `the imaging settings changed during the set`;
   and that after each failure the masters are empty and `settings.exposure_ms` is what
   it was before the request.
3. **Given** the rig of US7-S1 but with `level` 40000 for the camera's sixth exposure,
   **Then** a committed test asserts that the set is `failed` with exactly
   `a flat reads 61.0% of full scale, outside 40 to 50; is the panel steady?` and
   `taken` 2. **Given** the rig of US7-S1 with 100 ms applied and its camera failing its
   first exposure with `the camera fell over`, **Then** the set is `failed` with exactly
   `a flat failed: the exposure failed: the camera fell over`. **Given** the rig of
   US7-S1 at `needs_darks` with the camera still not
   covered, **When** the test sends `{"darks":true}`, **Then** the set returns to
   `needs_darks` with `darks_taken` 0 and exactly the `error`
   `the darks are not dark: one reads 29491 against an offset level of 240; cover the telescope`,
   with `not_dark` true; and **When** the test covers the camera and sends
   `{"darks":true}` again, **Then** the set becomes `done` with no `error` and no
   `not_dark` key, and the masters hold the master flat. **Given** the rig of US7-S1 at
   `needs_darks`, **When** the test applies `{"gain":200}`, covers the camera and sends
   `{"darks":true}`, **Then** the set returns to `needs_darks` with exactly the `error`
   `the imaging settings changed during the set`. **Given** a rig like US7-S1's
   whose `level`, once the test says the camera is covered, is 160 at an exposure of
   1 ms and 170 at any longer one, at `needs_darks` and covered, **When** the test sends
   `{"darks":true}`, **Then** the set returns to `needs_darks` with `not_dark` true and
   exactly
   `the darks are not dark: one reads 170 against an offset level of 160; cover the telescope`;
   and **When** it sends `{"darks":true,"anyway":true}`, **Then** the set becomes `done`,
   the master's `checked` is false, its file has `DARKTEST` F and every sample of the
   file is 1.0.
4. **Given** the rig of US7-S1, **Then** a committed test asserts that
   `POST /api/imaging/calibration/flats` answers:
   - 400 and exactly
     `send count and label to take flats, or darks: true to take their darks` for `{}`,
     `{"count":3}`, `{"label":"desk"}`, `{"count":3,"label":"desk","darks":true}`,
     `{"count":3,"label":"desk","anyway":true}` and `{"anyway":true}`;
   - 400 and `count must be a whole number from 3 to 64` for `{"count":2,"label":"desk"}`;
   - 400 and exactly
     `label must be 1 to 32 characters of a to z, 0 to 9 and -, starting with a letter or digit`
     for the labels `""`, `Desk`, `-a` and one of 33 letters;
   - 409 and `no flats are waiting for their darks` for `{"darks":true}` before any
     flats; 403 for a foreign `Origin`; and 409 and
     `this console takes no imaging frames` on a console built without `with_imaging`;
   - with flats waiting, 409 and
     `flats are waiting for their darks: take those or drop them first` for
     `{"count":3,"label":"again"}` and for `POST /api/imaging/calibration/darks` with
     `{"count":3}`;
   - with 10 s applied and a search running, 409 and
     `a set of darks or flats is already being taken` for a darks request, for
     `{"count":3,"label":"again"}` and for `{"darks":true}`;
   - just after `{"darks":true}` has answered, 409 and
     `a set of darks or flats is already being taken` for a darks request; and
     `DELETE /api/imaging/calibration/<id>` then answers 200 and the set ends `stopped`
     with no master;
   and that `DELETE /api/imaging/calibration/<the waiting set's id>` answers 200, leaves
   the set `stopped`, after which `{"darks":true}` answers 409 and
   `no flats are waiting for their darks` and a new request for flats is accepted.

---

### User Story 8 - Three MCP tools (Priority: P2)

As an MCP client, I can list the calibration library and start a set of darks or of
flats, through the same console actions and refusals as the page.

**Why this priority**: The owner's goal is an ask from an MCP client; a client that
prepares a night needs to know what the library holds and to fill it.

**Independent Test**: `crates/finder/tests/mcp/calibration.rs`, the module `calibration`
of finder's `mcp` test binary. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test mcp calibration::`.

**Acceptance Scenarios**:

1. **Given** a console with MCP on, **When** a test calls `tools/list`, **Then** a
   committed test asserts that `list_calibration`, `take_darks` and `take_flats` are
   listed, each with `outputSchema` `{"type":"object"}`; that only the first has
   `annotations.readOnlyHint` true; that the first's description is
   `List the calibration library's master darks and master flats, the set being taken, and what matches the latest imaging frame.`;
   that the second's contains `as POST /api/imaging/calibration/darks does` and
   `list_calibration`, and its `inputSchema.properties` has exactly the keys `count`
   and `anyway`; and that the third's contains
   `as POST /api/imaging/calibration/flats does`, and its `inputSchema.properties` has
   exactly the keys `count`, `label`, `darks` and `anyway`.
2. **Given** 031's stand-in console rig with MCP on, **Then** a committed test asserts
   that `list_calibration` with `{}` answers a `structuredContent` equal to the body of
   `GET /api/imaging/calibration` read just after; that `take_darks` with `{"count":3}`
   answers before the set is done, with `structuredContent.set.kind` `dark` and
   `wanted` 3; and that, once the set's `state` is `done`, `list_calibration`'s
   `structuredContent` equals the body of `GET /api/imaging/calibration` read just
   after, whose `set.state` is `done`, and shows one master with `frames` 3 and `model`
   `synthetic imaging camera` (oracle: the stand-in
   camera's frames, which read the same at every exposure and so pass as dark).
3. **Given** the same console, **Then** a committed test asserts that `take_darks` with
   `{}` and with `{"count":2}` answers `isError` true and
   `count must be a whole number from 3 to 64`; that `take_flats` with `{"count":3}`
   answers `isError` true and
   `send count and label to take flats, or darks: true to take their darks`; that
   `take_flats` with `{"darks":true}` answers `isError` true and
   `no flats are waiting for their darks`; that `take_flats` with
   `{"count":3,"label":"desk"}` answers a `structuredContent.set.kind` `flat`, and that
   set then becomes `failed` with an `error` that begins
   `the flat's level does not rise with the exposure`; and that, with an exposure of
   10 s applied and a `take_darks` set running, a second `take_darks` answers `isError`
   true and `a set of darks or flats is already being taken`; and that `take_darks` with
   `{"count":3,"cover":true}` and `take_flats` with `{"count":"three","label":"desk"}`
   answer `isError` true with exactly the `error` of the route's 400 for the same body.
4. **Given** a console with MCP on and no `with_imaging`, as
   `crates/finder/tests/support/mcp.rs:24` builds its state, **Then** a committed test
   asserts that `list_calibration` with `{}` answers `structuredContent`
   `{"masters":[]}`, and that `take_darks` and `take_flats` with `{}` each answer at once
   with `isError` true and `this console takes no imaging frames`, and so do `take_darks`
   with `{"count":"many"}` and `take_flats` with `{"label":7}`. **Given** the stand-in
   console rig with MCP on, **Then** no result of the three tools, called with `{}`,
   contains `test-token`.

---

### User Story 9 - The Calibration block on the Imaging page (Priority: P2)

As the observer with only the page to hand, on the Imaging page I see which masters the
latest imaging frame would get and what is missing, the list of masters, and two short
forms to take darks and flats. While a set is taken I see its progress and can stop it;
when flats wait for their darks the page tells me to cover the telescope.

**Why this priority**: Darks and flats are taken at the telescope, where the page is
often the only thing to hand.

**Independent Test**: `npm test` in `crates/finder/web` runs
`src/components/CalibrationBlock.test.tsx`. It is a command of the `test` gate.

**Acceptance Scenarios**:

1. **Given** `CalibrationBlock` rendered with the calibration `{"masters":[]}`, **Then** a
   committed test asserts one region named `Calibration` holding the texts
   `Take an imaging frame to see which masters match it.` and
   `No master darks or flats yet.`, a field `Dark frames` with the value 20, a button
   `Take darks`, fields `Flat frames` with the value 20 and `Flat label`, empty, a button
   `Take flats`, disabled while the label is empty, the texts
   `Cover the telescope first. Darks use the exposure, gain, offset, binning and set temperature in force.`
   and
   `Light the flat panel first. The console finds the exposure; the flats' darks follow with the telescope covered.`,
   and no `status` or `alert`. **Given** `ImagingPage` rendered with 031's ready view of
   its US1-S1, which has no `calibration`, and with each of 031's not-ready views,
   **Then** the committed test asserts no region named `Calibration`; with that ready
   view, 033's `focus` `{"unit":"arcsec","arcsec_per_px":2.19}` and `calibration`
   `{"masters":[]}`, one such region, after the regions `Imaging frame` and `Focus` in
   document order.
2. **Given** `CalibrationBlock` with the masters
   `{"id":"dark-20260116T020003Z","kind":"dark","made_at":1768528803.7,"frames":20,"checked":true,"gain":120.0,"offset":10.0,"binning":1,"set_temp_c":-10.0,"temp_c":-10.0,"exposure_ms":5000.0,"bad_pixels":137,"label":null,"imaging_angle_deg":null}`
   and
   `{"id":"flat-20260116T030000Z","kind":"flat","made_at":1768532400.0,"frames":20,"checked":true,"gain":120.0,"offset":10.0,"binning":1,"set_temp_c":-10.0,"temp_c":-10.0,"exposure_ms":132.93,"bad_pixels":null,"label":"desk","imaging_angle_deg":17.0}`,
   **Then** a committed test asserts a list of two items, each holding a line that reads
   exactly
   `Master dark · 5000 ms · gain 120 · offset 10 · 1 × 1 · set -10.0 °C · 20 frames · 137 bad pixels · 2026-01-16`
   and
   `Master flat desk · gain 120 · offset 10 · 1 × 1 · set -10.0 °C · 20 frames · 2026-01-16 · camera at 17.0°`,
   and a button named `Remove dark-20260116T020003Z` and
   `Remove flat-20260116T030000Z`; that clicking the first calls `fetch` with
   `/api/imaging/calibration/dark-20260116T020003Z` and `{"method":"DELETE"}` and then
   `onChange`; that a dark with `gain` null, `offset` null, `set_temp_c` null, `temp_c`
   20.0 and `bad_pixels` null reads
   `Master dark · 5000 ms · 1 × 1 · sensor 20.0 °C · 20 frames · no bad-pixel map · 2026-01-16`;
   that a flat with `imaging_angle_deg` null ends at its date; and that either master
   with `checked` false reads the same line followed by ` · dark test skipped`; and
   that, with a `set` in `frames` as well, clicking `Remove dark-20260116T020003Z` still
   calls `fetch` with `/api/imaging/calibration/dark-20260116T020003Z`.
3. **Given** `CalibrationBlock` with `{"masters":[]}`, **When** a test sets `Dark frames`
   to 5 and clicks `Take darks`, with `fetch` answering 200 and a calibration, **Then** a
   committed test asserts that `fetch` was called with `/api/imaging/calibration/darks`
   and
   `{"method":"POST","headers":{"Content-Type":"application/json"},"body":"{\"count\":5}"}`,
   then `onChange`; that a 409 answer with
   `{"error":"an exposure is in progress"}` shows an alert with exactly that text and
   calls no `onChange`; and that, with `Flat frames` 4 and `Flat label` `desk`,
   `Take flats` calls `fetch` with `/api/imaging/calibration/flats` and the body
   `{"count":4,"label":"desk"}`.
4. **Given** `CalibrationBlock` with each `set`, **Then** a committed test asserts:
   - `{"id":"dark-x","kind":"dark","state":"offset","wanted":4,"taken":0,"exposure_ms":200}`:
     a status `Taking darks: reading the offset level`, a button `Stop`, which calls
     `fetch` with `/api/imaging/calibration/dark-x` and `{"method":"DELETE"}`, and
     `Take darks` and `Take flats` disabled;
   - the same with `state` `frames` and `taken` 2: `Taking darks: 2 of 4 at 200 ms`;
   - `kind` `flat`, `state` `search`: `Taking flats: finding the exposure`;
   - `kind` `flat`, `state` `frames`, `taken` 1, `wanted` 3, `exposure_ms` 366.634:
     `Taking flats: 1 of 3 at 366.634 ms`;
   - `kind` `flat`, `state` `needs_darks`, `wanted` 3, `exposure_ms` 366.634: a status
     `Flats in hand: 3 at 366.634 ms. Cover the telescope, then take their darks.`, a
     button `Take the flats' darks`, which calls `fetch` with
     `/api/imaging/calibration/flats` and the body `{"darks":true}`, a button
     `Drop these flats`, which calls the `DELETE`, no button `Stop`, and `Take darks`
     and `Take flats` disabled; with `error` `the darks are not dark`, also an alert
     `The flats' darks stopped: the darks are not dark.`; and with `not_dark` true as
     well, also the text
     `Only if the telescope is covered: a warm sensor or a long exposure can read above the offset level.`
     and a button `Take the flats' darks anyway`, which calls `fetch` with
     `/api/imaging/calibration/flats` and the body `{"darks":true,"anyway":true}`;
   - `kind` `flat`, `state` `flat_darks`, `darks_taken` 1, `wanted` 3:
     `Taking the flats' darks: 1 of 3`;
   - `state` `done`: for `kind` `dark` and `wanted` 4, a status
     `Master dark saved from 4 darks.`; for `kind` `flat`, `label` `desk` and `wanted`
     3, `Master flat desk saved from 3 flats.`; no button `Stop`;
   - `state` `failed`, `error` `the camera fell over`: for `dark` an alert
     `Darks stopped: the camera fell over.`, for `flat` `Flats stopped: the camera fell over.`,
     and no button whose name ends `anyway`;
   - `kind` `dark`, `state` `failed`, `wanted` 4, `not_dark` true, `error`
     `the darks are not dark`: the alert `Darks stopped: the darks are not dark.`, the
     text
     `Only if the telescope is covered: a warm sensor or a long exposure can read above the offset level.`
     and a button `Take them anyway`, which calls `fetch` with
     `/api/imaging/calibration/darks` and the body `{"count":4,"anyway":true}`;
   - `state` `stopped`: a status `Darks stopped.` or `Flats stopped.`
5. **Given** `CalibrationBlock` with the two masters of US9-S2 and `match`
   `{"frame":12,"dark":"dark-20260116T020003Z","flat":"flat-20260116T030000Z","bad_pixels":137,"missing":[]}`,
   **Then** a committed test asserts the text
   `Imaging frame 12 would get: its master dark · the master flat desk · a bad-pixel map of 137`
   and no alert; with `match`
   `{"frame":12,"dark":null,"flat":null,"bad_pixels":null,"missing":["dark","flat","bad_pixels"]}`,
   the text
   `Imaging frame 12 would get: no master dark · no master flat · hot pixels found in the sub itself`;
   and with the first `match` and `flat_turned_deg` 2.5, an alert reading exactly
   `The camera has turned 2.5° since the flats desk were taken. Take new flats.` **Given**
   a calibration with `error` `the calibration index could not be read: expected value`,
   **Then** an alert reads exactly
   `The calibration library is unreadable: the calibration index could not be read: expected value.`
6. **Given** `ImagingPage` fed by its hook with fake timers, `fetch` answering
   `/api/imaging` with 031's ready view plus `calibration` `{"masters":[]}`, **When** a
   test clicks `Take darks`, with `fetch` answering the darks request 200, **Then** a
   committed test asserts that `/api/imaging` is read again at once after that answer,
   and that when the next read's `calibration` holds a `set` in `state` `frames` the
   region shows its status.

---

### User Story 10 - A master flat on the simulated camera, and the imaging angle kept with it (Priority: P2)

As the observer, I want a master flat made from the simulated camera's flats to remove
exactly its vignetting and dust, a sub calibrated with it and a master dark to have a
level background, and the master flat to keep the imaging angle the console measured
when its flats were taken, so that a later imaging frame warns me once the camera has
turned.

This story is built after User Story 7 (Work Graph). It has the number 10 because it was
cut from User Story 7 once the round-1 probe build measured that story over the 64 KiB
budget, and numbering it so left every other story and scenario with its number. Its
three scenarios were US7-S5 to US7-S7.

**Why this priority**: The simulator's known vignetting and dust are the one oracle a
whole master flat is checked against, and without the angle kept a turned camera is
never warned of. Flats are taken, made and matched without it, which is why it follows
US7.

**Independent Test**: `crates/finder/tests/console/calibration_flats_sim.rs`, the module
`calibration_flats_sim` of `console`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development,
`cargo nextest run -p finder --locked --test console calibration_flats_sim::`.

**Acceptance Scenarios**:

1. **Given** the console rig with, on sim, H, `column_amplitude` 14.0, `corner_light`
   0.6, the dust shadow of US1-S6 and a `flat_panel` of 1,000,000 electrons a second
   with the colour `[0.8, 1.0, 0.6]`, and `{"gain":0,"exposure_ms":100}` applied, **When**
   a test asks for 4 flats labelled `dob`, waits for `needs_darks`, sets `cover` true,
   sends `{"darks":true}` and waits for `done`, **Then** a committed test asserts that the
   set's `exposure_ms` is 132.9 ± 0.5 (worked example: 100 ms reads 33.9 percent, 200 ms
   67.6 percent, and the line through them meets 45 percent at 132.93 ms); that the
   master's `level_share` is 0.45 ± 0.005; that in its file each colour's median is
   1.000 ± 0.002; and, with m the median of V·D over a colour's samples as the test
   computes it, that the file's sample times m is V·D within 0.01 at the green samples
   (619, 410), (1, 0), (301, 200) and (901, 600), at the red samples (618, 410) and
   (0, 0), and at the blue samples (619, 411) and (1235, 821), and within 0.02 at the
   hot pixel (100, 50) and at the hot pixel (600, 400) (oracle: the simulator's V and D;
   the panel's colour, the offset level, the column pattern and the hot pixels all leave
   the master).
2. **Given** the solving rig with `{"gain":0}` applied, **When** a test waits until the
   session's `pointing` has the `source` `live`, waits 5 s more, takes an imaging frame
   at the starting exposure of 1000 ms and solves it, and reads `relation.angle_deg`
   from `GET /api/imaging` as **a0**; then sets on sim a white `flat_panel` of
   1,000,000 electrons a second, asks for 3 flats labelled `dob` and waits for
   `needs_darks`; then sets the light path to `cover` true with no panel, sends
   `{"darks":true}` and waits for `done`, **Then** a committed test asserts that a0 is
   within 0.25° of 17.0, the simulator's true imaging angle, as 032's US4-S1 has it;
   that the camera was asked for flats at 125.0 ms (worked example: 1000 ms saturates,
   and an eighth of it reads 31410 counts, 47.9 percent); that the master's
   `imaging_angle_deg` equals a0 within 1e-9 and its file has `IMGANGLE` equal to a0
   within 1e-6; and, after the light path is set back to its default, an imaging frame
   is taken and saved, that `match.flat` is the master's id and `match` has no
   `flat_turned_deg` key. **When** the test turns the simulated imaging camera with
   `sim.set_imaging_optics(ImagingOptics { angle_deg: 19.5, ..sim.imaging_optics() })`,
   takes and solves three more frames, and reads `relation.angle_deg` as **a1**,
   **Then** a1 is within 0.25° of 19.5 (worked example: 032's window then holds one aim
   reading near 17.0 and three near 19.5, whose median is near 19.5), and
   `match.flat_turned_deg` equals a1 − a0 within 1e-6 and is within 0.5 of 2.5. **When**
   the test sends `DELETE /api/aim`, **Then** `GET /api/imaging` has no
   `relation.angle_deg` and `match` has no `flat_turned_deg` key (oracle: the
   simulator's truth through 032's aim readings; the imaging angle is carried only
   while the aim point is the measured one).
3. **Given** the console rig with the sensor pattern and light path of US10-S1 and gain 0,
   **When** a test covers the camera and takes 3 darks at 10000 ms; lights the panel and
   takes 3 flats labelled `dob` from 100 ms, then their darks under the cover; switches
   the panel and the cover off, applies 10000 ms, sends `POST /api/imaging/frame`, reads
   the saved file and calls the library's `calibrate` on it with its
   `FrameKey::from_image`, **Then** a committed test asserts that `selection.dark` and
   `selection.flat` are the two masters' ids and `selection.missing` is empty; that the
   report is `applied`, `applied`, `map` and `problems` is empty; that, with m the median
   of V·D over all samples, the medians of the calibrated samples in the 100 × 100 blocks
   at (0, 0) and at (568, 361) are each 50·m ± 3; that the same two medians of the saved
   file's own samples differ by more than 12; and that `match` in
   `GET /api/imaging/calibration` names both ids with `bad_pixels` 3 and `missing` `[]`
   (oracle: the simulator's sky of 200 electrons, 50 counts, and its V and D).

---

### Edge Cases

- **No master matches.** A sub is still calibrated: nothing is subtracted or divided, hot
  pixels are found in the sub itself, and the report says `missing` for each absent
  master.
- **A cover left off under a dark sky with a short exposure.** Where the sky adds less
  than twice the noise in the exposure, the frame's level passes. The stars in the
  field then refuse the set: their samples stand bright and touch (US6-S3). What still
  passes is an uncovered frame of a field with almost no stars in it, fewer than one
  bright sample in 10,000; the page says to cover the telescope first, and nothing else
  guards that (owner decision 4).
- **A warm sensor at a long exposure.** Thermal signal raises the level of a covered
  frame. Where it passes twice the noise the set is refused although covered, and the
  observer takes it with `anyway`; the master then says `checked` false. A cooled camera
  at exposures of seconds is far from this; an uncooled one at a minute may meet it
  (owner decision 4).
- **A sensor with a glow, a bright column or many clustered hot pixels.** Samples that
  stand bright and touch are counted as light. A covered sensor with more than one such
  sample in 10,000 is refused as not dark, and is taken with `anyway` (owner decision
  4).
- **A sensor still cooling.** A frame whose sensor temperature is more than 1.0 °C from
  its set temperature fails the set, with or without `anyway`: the observer waits, or
  sets a temperature the cooler can hold. A sub taken while the sensor is still cooling
  is matched to a master dark by its set temperature all the same, and a master flat
  does not look at it (owner decision 7); holding subs back until the sensor
  has settled is left to the specs that take subs, and neither 035's nor 037's draft
  does it yet (owner decision 24).
- **The camera clips below its samples' full scale.** A flat can never reach 45 percent
  of a full scale the camera does not have: the search ends with
  `the flat's level does not rise with the exposure` or as out of tries, and nothing is
  made (owner decision 9).
- **The camera is turned between taking flats and a sub.** With both angles known the
  match warns; without them it cannot, and they are known only while the session's aim
  point is the one the console measured (owner decision 20). The flat is applied either
  way.
- **Two optical trains.** The newest matching master flat wins whatever its label. An
  observer who changes the train takes new flats, or removes the ones that no longer
  apply; a caller of the library may ask for one label (US3-S4).
- **The console restarts while flats wait for their darks.** The flats in hand are lost.
  Their files stay under `frames/`, and the observer takes the flats again.
- **Another frame is asked for during a set.** The set holds the imaging camera's lease
  from its start to its end, between two of its frames as much as during one: a
  `POST /api/imaging/frame`, a `PUT /api/imaging/settings`, 031's MCP tools, 033's focus
  aid and 035's stack are refused by 031 with the holder's name,
  `a set of darks or flats`, and the set goes on (US6-S9). A set asked for while another
  taker holds the camera is refused the same way, with that taker's name (owner
  decision 23).
- **Frames between a set of flats and its darks.** Flats that wait hold no lease, so the
  observer may take imaging frames, change settings or focus between the two steps. The
  darks step takes the lease again, and is refused with `an exposure is in progress`
  while a frame is in flight. A gain, offset, binning, region or set temperature changed
  in between fails the darks step as below, and the flats stay in hand.
- **The settings differ during a set.** While a set holds the lease no settings change
  reaches the camera through the console. A frame that still differs from the set's
  first in gain, offset, binning, region or set temperature, or was not taken at the
  exposure the set asked for (a camera that changed a setting by itself, or a change
  made between the two steps of a set of flats), fails the set with
  `the imaging settings changed during the set`, or, for the darks of a set of flats,
  returns the set to `needs_darks` with those words; nothing is added (US6-S7).
- **A request in the same instant as a frame.** A `POST /api/imaging/frame` that starts
  a frame on another thread after a set's request has looked for one, and before it
  holds the lease, makes the set's first exposure setting refused: the set ends `failed`
  with `a dark failed: an exposure is in progress` (for flats, `a flat failed: …`)
  before it has taken anything, and the setting is as it was. No test can time this, and
  none asserts it.
- **The link drops during a set.** The frame in flight ends `failed`, and so does the
  set; nothing is added. 031 refuses a settings change while the camera is not `ready`,
  so the exposure setting stays where the set left it, the shortest exposure or the
  flats' own, until the observer sets it again.
- **The disk fills.** A frame that cannot be saved fails the set. A master that cannot be
  written fails the set with `the master could not be saved: ` and the system's words,
  and the library's list is left as it was.
- **A master's file is removed by hand.** It stays in the list; a sub that would have
  used it is calibrated without it, and the report's `problems` name it.
- **An unreadable `index.json`.** The library is empty and says why; the next master made
  moves the bad file aside as `index.json.unreadable` and starts a new list. The master
  files it named stay on the disk, unlisted.
- **A mono camera.** There is one colour: a flat is normalised by one median, and a bad
  pixel is repaired from the samples next to it.
- **A frame taken with a region of interest.** A master matches only a sub of the same
  size and origin. Darks and flats may be taken with a region and serve subs of that
  region.
- **A camera with no gain, offset or temperature.** Those keys are null in the master. A
  master dark with no gain, offset or set temperature matches only a sub with none
  either; a master flat is not matched on them (owner decision 7).
- **Darks taken at binning 2.** A hot pixel is diluted by the three good samples it is
  averaged with, and still stands far above the line (US6-S2).
- **A hot pixel next to another of its colour.** The search in the sub itself sees
  neither stand above its neighbour, and repairs neither. A bad-pixel map finds both.
- **Hot pixels that touch.** Two that touch are both found in the sub itself. Three or
  more that touch look like a small star to the search and are left; a bad-pixel map
  finds them (US2-S9).
- **A hot pixel inside nebulosity.** The search in the sub itself compares it with the
  frame's median, not with the nebula around it, and finds it only when it stands more
  than ten times as far above that median as the samples that touch it do. A bad-pixel
  map does not have this limit, which is one more reason to take darks before a night
  on a bright nebula.
- **Stars sharper than 1.4 samples.** On a telescope whose stars are narrower than about
  1.4 samples of the delivered frame at half their height, the search in the sub itself
  can take a star's brightest sample for a hot pixel. A bad-pixel map avoids it (owner
  decision 14).
- **Many masters.** Nothing is ever removed by the console itself. A master at the
  worked example's full resolution is about 65 MB and a set of 20 about 650 MB of
  frames.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The simulator SHALL hold a `SensorPattern` (hot pixels and a column
  amplitude) and a `LightPath` (a cover, a flat panel, the light at the corner and dust
  shadows), read and set through `Simulator::sensor_pattern`, `set_sensor_pattern`,
  `light_path` and `set_light_path`, and exported from `crates/simulator` (owner
  approval 4). With both at their defaults every simulated imaging frame SHALL be byte
  for byte what it is without this spec (US1-S1).
- **FR-002**: With the cover on, no star, sky or panel light SHALL reach a simulated
  frame (US1-S2, US1-S6).
- **FR-003**: Each hot pixel SHALL add its electrons per second times the exposure to its
  own sensor sample, covered or not, before the noise and before binning (US1-S3).
- **FR-004**: The column pattern SHALL add `column_amplitude·((x mod 8)/7 − 0.5)` counts
  to every sample of sensor column x, after the gain and before binning (US1-S4).
- **FR-005**: A flat panel SHALL replace the sky and the stars with its electrons per
  second times the exposure times its colour for the sample's filter; and all light,
  from stars, sky or panel, SHALL be multiplied by V and D as the shared setup defines
  them (US1-S5, US1-S6).
- **FR-006**: `Mosaic` SHALL hold one frame's samples as real numbers with its size and
  Bayer pattern; `Mosaic::from_samples` SHALL build it from little-endian samples of 8
  or 16 bits; and a sample's colour SHALL follow the pattern's letters from the frame's
  top-left, the two greens being one colour and a mono frame having one (US2-S1).
- **FR-007**: `calibrate` SHALL, in this order, subtract the master dark when given,
  keeping values below zero; divide by the master flat when given; and repair bad
  pixels: those of the bad-pixel map when one is given, otherwise those `find_hot_pixels`
  finds in the sub. It SHALL report `dark` and `flat` as `applied` or `missing`,
  `bad_pixels` as `map` or `sub`, and the counts `repaired`, `not_repaired` and
  `low_flat` (US2-S2, US2-S5, US2-S8, US2-S11; owner decision 11).
- **FR-008**: `repair` SHALL replace each listed sample inside the frame with the median
  of those of its four same-colour neighbours along its row and column (two samples
  away on a mosaic, one on a mono frame) that are inside the frame and not listed, the
  median of an even number being the mean of the middle two; a sample with no such
  neighbour SHALL be left and counted as not repaired (US2-S4).
- **FR-009**: `find_hot_pixels` SHALL answer, row by row, each sample that stands more
  than 8 times its colour's noise above the largest of those four neighbours; stands
  more than 5 times as far above its colour's median as that neighbour does; and has at
  most one lit sample among the eight that touch it, a sample being lit when it stands
  above its own colour's median by more than 3 times its colour's noise and by more
  than a tenth of what the candidate stands above its own. The noise is the colour's
  median less its 16th percentile and at least one count (US2-S5, US2-S9, US2-S11;
  owner decision 14).
- **FR-010**: `normalise_flat` SHALL divide each colour of a flat by that colour's median
  and answer the medians; and `calibrate` SHALL not divide a sample whose flat is under
  0.1 or not a number, but repair it as a bad pixel and count it in `low_flat` (US2-S3,
  US2-S6; owner decisions 12 and 15).
- **FR-011**: `calibrate` SHALL refuse a master whose length is not the sub's with a
  `SizeMismatch` and leave the sub untouched (US2-S7).
- **FR-012**: `crates/finder/src/imaging/fits.rs` SHALL gain `RealImage`, `encode_real`
  and `decode_real`: one image of `BITPIX` −32, big-endian reals row by row from the top,
  with the cards given and no `BZERO` or `BSCALE`. 031's `decode` SHALL be unchanged
  (US3-S1; owner decision 3).
- **FR-013**: The calibration library SHALL live in `calibration/` under the imaging
  folder: each master as `darks/<id>.fits` or `flats/<id>.fits` with the cards of
  US3-S2, and `index.json`, format `finder-calibration/1`, listing every master with the
  keys of plan.md § The library. A master's id SHALL be `dark-` or `flat-` and the UTC
  second it was begun, `YYYYMMDDTHHMMSSZ`, with `-2`, `-3` and so on when taken. Files
  SHALL be written under a `.part` name and renamed; no master file SHALL be replaced;
  and an unreadable index SHALL be reported and moved aside at the next write (US3-S2,
  US3-S6; owner approval 5).
- **FR-014**: A master dark SHALL match a frame when their camera model and serial,
  width, height, origin, bits, Bayer pattern and binning are equal; their gain and their
  offset are each both absent or equal within 0.001; their exposures differ by at most
  1 ms; and either both have a set temperature and these differ by at most 1.0 °C, or
  neither has one and their sensor temperatures differ by at most 8.0 °C or one is
  absent. Of several, the one nearest in that temperature SHALL be chosen, then the
  newest (US3-S3; owner decision 6).
- **FR-015**: A master flat SHALL match a frame when their camera model and serial,
  width, height, origin, bits, Bayer pattern and binning are equal and, when the caller
  names a flat label, its label is that one. Its gain, offset, set temperature, exposure
  and sensor temperature SHALL not restrict the match; the newest SHALL be chosen. When
  the imaging angle now and the flat's are both known and differ, the short way round,
  by more than 1.0°, the selection SHALL carry the difference as `flat_turned_deg`, and
  the flat SHALL still be chosen (US3-S4; owner decisions 7 and 8).
- **FR-016**: A selection SHALL carry the chosen dark's and flat's ids and the chosen
  dark's `bad_pixels`, and name in `missing`, in this order, those of `dark`, `flat` and
  `bad_pixels` that it lacks. `Library::calibrate` SHALL select the masters for a frame
  key, read them, calibrate the sub with them and answer the selection, the report and,
  for each chosen master it could not read or whose size did not fit, one line of
  `problems`, that step being reported `missing` (US3-S3, US3-S5).
- **FR-017**: While the imaging state is `ready`, `GET /api/imaging`, and `imaging` in
  `GET /api/session` and in the MCP tools that return either, SHALL carry `calibration`:
  `masters`, the library's entries, oldest first; `error` when the index is unreadable;
  and `match`: the selection for the latest imaging frame the console has seen saved
  whose purpose label is none of `dark`, `flat` and `focus`, with that frame's id as
  `frame` and its set temperature the frame's own `set_temp_c`. It SHALL stay while
  later frames are exposing or are darks, flats or focus frames, and be absent until
  there is such a frame. The imaging angle given to the selection SHALL be the imaging
  view's `relation.angle_deg` when it has one (US4-S1, US4-S3, US10-S2; owner approval
  1, owner decisions 20 and 22).
- **FR-018**: `GET /api/imaging/calibration` SHALL answer 200 with that object in every
  imaging state, and with exactly `{"masters":[]}` on a console that takes no imaging
  frames (US4-S1, US4-S2).
- **FR-019**: `DELETE /api/imaging/calibration/{id}` SHALL, under the same-origin rule,
  remove the master with that id and its files and answer 200 with the calibration
  object; answer 404 with `no master or set has the id <id>` for any other id, without
  touching the disk when the id is not of the form FR-013 gives; and answer 409 with
  `this console takes no imaging frames` on a console without imaging, the origin being
  judged before that (US4-S1, US4-S2).
- **FR-020**: `crates/finder/README.md` SHALL gain `## Calibration frames`, documenting
  the library's folder, files and cards, the matching rules, the order of the steps and
  the two routes' rows; and `crates/finder/src/lib.rs` their two lines (owner approval
  8). No scenario asserts a document's words; the story's diff shows them.
- **FR-021**: `bad_pixel_map` SHALL answer, row by row, the samples of a master dark that
  are more than 3.5 times its spread above its median, or not a number, the spread being
  the median less the 16th percentile and at least one count; and SHALL answer no pixels
  and `too_many` when they are more than 1 percent of the frame (US5-S1 to US5-S4;
  owner decision 13).
- **FR-022**: `Library::attach_bad_pixels` SHALL keep a master dark's map beside it as
  `darks/<id>.badpixels.json`, format `finder-bad-pixels/1`, and its size as the entry's
  `bad_pixels`, writing nothing for a `too_many` map. `Library::calibrate` SHALL repair
  from the map when there is one, and removing the master SHALL remove the map
  (US5-S5).
- **FR-023**: `GLOSSARY.md` SHALL define **Bad-pixel map** with the words of owner
  approval 6, and `## Calibration frames` SHALL document the map and its rule.
- **FR-024**: `POST /api/imaging/calibration/darks` SHALL accept `{"count":N}`, N a
  whole number from 3 to 64, with or without `"anyway"`, under the same-origin rule, and
  answer 200 with the calibration object and a new `set`. A request SHALL be judged in
  this order, the first that applies answering: a foreign origin, 403; a body that is
  not JSON of that shape, 415 or 400; a console that takes no imaging frames, 409; a
  count absent or outside 3 to 64, 400 with `count must be a whole number from 3 to 64`;
  an imaging state that is not `ready`, 409 with 031's words for the state; a set being
  taken, 409 with `a set of darks or flats is already being taken`; a frame in flight
  while nobody holds the imaging camera's lease, 409 with `an exposure is in progress`;
  and last, the lease taken under `HOLDER`, whose refusal SHALL be answered unchanged,
  its status and its words (US6-S5, US6-S9; owner decisions 5 and 23).
- **FR-025**: A set of darks SHALL first take one dark of the camera's shortest exposure,
  the offset frame. Its median is the offset level and its noise is that less its 16th
  percentile, at least one count. Unless the set was asked for with `anyway`, it SHALL
  fail, adding nothing and with `not_dark` true in its view, with
  `the camera is not covered: its shortest exposure reads <level> of <full scale>` when
  the offset level is above 20 percent of full scale; with
  `the darks are not dark: one reads <level> against an offset level of <offset level>; cover the telescope`
  when any dark's median is more than twice that noise above the offset level; and with
  `the darks are not dark: one holds <n> bright samples that are not hot pixels; cover the telescope`
  when any dark's `lit_samples`, n, is above 8 and above one in 10,000 of its samples
  (US6-S3, US6-S4; owner decision 4).
- **FR-026**: A set of darks SHALL take N darks with the purpose label `dark` at the
  exposure in force, each kept as an imaging frame; average them sample by sample into a
  master dark; add it to the library with the camera, size, origin, gain, offset,
  binning, set temperature, mean sensor temperature and exposure its frames carry; and
  attach its bad-pixel map (US6-S1, US6-S2, US6-S8; owner decision 2).
- **FR-027**: The calibration object SHALL carry `set` from the first accepted set until
  the console exits: the latest set's `id`, `kind`, `state`, `wanted`, `taken`,
  `exposure_ms`, and when known `offset_level`, `level` and `error`. A set SHALL end
  `failed` with `a dark failed: <why>` when a frame fails or its next frame is refused,
  and with `the imaging settings changed during the set` when a frame does not agree, by
  FR-041, with the set's first frame and the exposure the set asked for; and `stopped`
  when its exposure is aborted or `DELETE /api/imaging/calibration/<its id>` is sent
  (US6-S6, US6-S7).
- **FR-028**: `GLOSSARY.md` SHALL define **Offset level** with the words of owner
  approval 6; `## Calibration frames` SHALL document the darks route, the dark test,
  `anyway` and that each dark is kept as an imaging frame; and
  `crates/finder/src/lib.rs` SHALL gain the route's line.
- **FR-029**: `POST /api/imaging/calibration/flats` SHALL accept exactly
  `{"count":N,"label":L}`, N from 3 to 64 and L 1 to 32 characters of `a` to `z`, `0` to
  `9` and `-` that begins with a letter or digit, or `{"darks":true}` with or without
  `"anyway"`, under the same-origin rule, and answer 200 with the calibration object. A
  request SHALL be judged in FR-024's order, with these differences: any other pair of
  keys answers 400 with
  `send count and label to take flats, or darks: true to take their darks`, and a label
  of another form 400 with the text of US7-S4; and after the check for a set being
  taken, `{"darks":true}` answers 409 with `no flats are waiting for their darks` unless
  the latest set is a flat set in `needs_darks`. While flats wait, a request for new
  flats, and a request on FR-024's route, SHALL each answer 409 with
  `flats are waiting for their darks: take those or drop them first`, at that same
  place in the order (US7-S4; owner decision 10).
- **FR-030**: The first step SHALL find the flat's exposure: a frame's level is the
  largest of its colours' medians; an exposure whose level is from 40 to 50 percent of
  full scale is taken; the search starts at the exposure in force, moves to an eighth of
  the exposure after a level of 90 percent or more, doubles or halves until it has two
  unsaturated frames, then follows the straight line through its last two to 45 percent;
  it asks for no exposure outside the camera's range or above 30 s, and takes at most 8
  frames. It SHALL fail with the texts of US7-S2 when the level does not rise, when the
  range's end is still too dim or too bright, or when 8 frames do not find it (US7-S1,
  US7-S2; owner decision 9).
- **FR-031**: The first step SHALL then take N flats with the purpose label `flat` at
  that exposure, fail when one's level is outside 40 to 50 percent, and hold their sum
  with the set in `needs_darks`. The second step SHALL take an offset frame, and then N
  darks at the flats' exposure, under FR-025's three tests unless it was asked for with
  `anyway`; a failure of that step SHALL return the set to `needs_darks` with the
  `error`, and `not_dark` when a test refused it, the flats still in hand.
  `DELETE /api/imaging/calibration/<the set's id>` SHALL drop them. Each of the two
  steps SHALL hold the imaging camera's lease as FR-042 has a set of darks hold it,
  taken last of its request's checks and dropped before its state becomes
  `needs_darks`, `done`, `failed` or `stopped`; flats that wait for their darks SHALL
  hold none (US7-S1, US7-S3, US7-S4).
- **FR-032**: The master flat SHALL be the mean of the flats less the mean of their
  darks, normalised by `normalise_flat`, with samples that are not above zero set to
  zero, added to the library with its label, its imaging angle as FR-045 gives it, the
  flats' level as a share of full scale, `checked` false when its darks were taken with
  `anyway`, and the keys of FR-026 (US7-S1, US7-S3, US10-S1, US10-S3).
- **FR-033**: `GLOSSARY.md` SHALL define **Flat label** with the words of owner approval
  6; `## Calibration frames` SHALL document the flats route, its two steps, the search
  and the label; and `crates/finder/src/lib.rs` SHALL gain the route's line.
- **FR-034**: The MCP tool `list_calibration` SHALL be read-only and answer the
  calibration object as `GET /api/imaging/calibration` does (US8-S1, US8-S2, US8-S4).
- **FR-035**: The MCP tools `take_darks` and `take_flats` SHALL take the bodies of
  FR-024 and FR-029, `anyway` included, answer once the set or step has started with the
  calibration
  object, and refuse with the routes' words; on a console that takes no imaging frames
  they SHALL answer at once with `this console takes no imaging frames` (US8-S2 to
  US8-S4; owner decision 16).
- **FR-036**: `crates/finder/README.md` SHALL list the three tools in `## MCP clients`.
- **FR-037**: The Imaging page SHALL show a Calibration block after its Imaging frame
  section while the view is ready and carries `calibration`: what the latest imaging
  frame would get, a warning when the camera has turned, the list of masters with a
  Remove button each, and the two forms, with the words of US9-S1, US9-S2 and US9-S5
  (owner approval 3).
- **FR-038**: The block SHALL send the darks, flats, flats' darks and delete requests of
  US9-S2 to US9-S4, read the imaging view again after each, show a refusal as an alert,
  show a set's progress, end and failure with the words of US9-S4, offer
  `Take them anyway` after a set of darks that the dark test refused and
  `Take the flats' darks anyway` when it refused a flat set's darks, and disable both
  Take buttons while a set runs or flats wait (US9-S3, US9-S4, US9-S6).
- **FR-039**: `DESIGN.md` SHALL describe the block, `.impeccable/design.json` SHALL list
  it as a component, and `crates/finder/web/dist` SHALL be the build of its source
  (owner approval 7).
- **FR-040**: `lit_samples` SHALL answer how many samples of a mosaic stand more than 8
  times their colour's noise above their colour's median and are not among those
  `find_hot_pixels` answers (US2-S10; owner decision 4). US2 implements it.
- **FR-041**: `FrameKey::agrees` SHALL be true of a key, another key and an exposure
  when the two keys' camera model and serial, width, height, origin, bits, Bayer
  pattern and binning are equal, their gain, offset and set temperature are each both
  absent or equal within 0.001, and the first key's exposure is within 1 ms of the
  exposure given; sensor temperature SHALL not be compared (US3-S3). US3 implements it.
- **FR-042**: A set of darks SHALL hold the imaging camera's lease (031 US9) under
  `HOLDER`, `a set of darks or flats`, a public constant of
  `crates/finder/src/imaging/calibration/service.rs`, from the moment its request is
  accepted until just before its state becomes `done`, `failed` or `stopped`. Its
  request SHALL take the lease last of FR-024's checks and answer a refusal of it
  unchanged. The set SHALL set every exposure and take every imaging frame through that
  lease (`Lease::set`, `Lease::take`), and, however it ends, put the exposure in force
  before the request back through it, a refusal there being ignored, then drop the
  lease, and only then write its final state. It SHALL add no lock of its own over the
  camera (US6-S1, US6-S6, US6-S9; owner decision 23). US6 implements it.
- **FR-043**: A set of darks asked for with `"anyway":true` SHALL be taken and averaged
  whatever FR-025's three tests would say. A master's entry SHALL say `checked` false,
  and its file `DARKTEST` F, when its darks were taken so, and `checked` true and
  `DARKTEST` T otherwise (US6-S1, US6-S4; owner decision 4). US6 implements it.
- **FR-044**: A set SHALL fail with
  `the sensor is at <t> °C, not yet at its set temperature <s> °C`, both to one decimal,
  when a frame of it has a set temperature and a sensor temperature more than 1.0 °C
  apart, with or without `anyway` (US6-S7; owner decision 24). US6 implements it.
- **FR-045**: A master flat's entry SHALL carry, as `imaging_angle_deg`, the imaging
  angle the imaging view carried as `relation.angle_deg` when the last flat of its set
  was in hand, and be null when the view carried none; its file then has the `IMGANGLE`
  card (US10-S2; owner decision 20). Until US10 a master flat's `imaging_angle_deg` is
  null (US7-S1). US10 implements it, split out of FR-032 when round 1 cut US7.

### Key Entities

- **Mosaic**: one frame's samples as real numbers, with its size and Bayer pattern.
- **Master dark, master flat**: the glossary's, each a 32-bit real FITS file with an
  entry in the library's index.
- **Bad-pixel map**: the places in a master dark that read high whatever the light.
- **Frame key**: what a frame is matched by: camera, size, origin, bits, pattern, gain,
  offset, binning, temperatures and exposure.
- **Selection**: the masters chosen for a frame key, what is missing and whether the
  camera has turned since the flats.
- **Report**: what `calibrate` did to one sub.
- **Set**: one request for darks or flats, from its first frame to its master. The
  glossary avoids "calibration set"; it is always a set of darks or a set of flats.
  While it is taken it holds 031's lease of the imaging camera under the holder name
  `a set of darks or flats`; flats that wait for their darks hold none.
- **Offset frame, offset level**: a set's first dark, of the camera's shortest exposure,
  and its median (owner approval 6).
- **Dark test**: the three checks that a set's darks are dark, and `anyway`, the
  observer's way past them.
- **Flat label**: the short name the observer gives the flats taken through one optical
  train (owner approval 6).
- **Sensor pattern, light path**: the simulator's hot pixels and column pattern, and its
  cover, flat panel, vignetting and dust.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario has a committed test that asserts it, and the
  `lint` and `test` gates pass.
- **SC-002**: No existing test, golden, `scripts/ci/` file, `.github/` file, `CODEOWNERS`
  entry, `ergane.yaml`, `.specify/` file or `playwright.config.ts` is edited. The only
  edits to existing test files are the `mod` lines of owner approval 9. Every test 030, 031,
  032 and 033 land passes unchanged.
- **SC-003**: A simulated sub with vignetting, a dust shadow, a column pattern and hot
  pixels, calibrated with masters the console made from simulated darks and flats, has a
  background level that is the same at the corner and the centre within 3 counts in 50
  (US10-S3).
- **SC-004**: A bad-pixel map found in a master dark of simulated frames is exactly the
  simulator's list of hot pixels (US5-S4, US6-S1).
- **SC-005**: No file under `crates/finder/src/imaging/calibration.rs` or
  `crates/finder/src/imaging/calibration/` names a camera model, a focal length, an axis
  or an altitude, and no `unsafe` is added.
- **SC-006**: A set of darks asked for with the cover off is refused on the simulated
  camera both where the sky raises the level (US6-S3, gain 120 and 1000 ms) and where
  it does not (US6-S3, gain 0 and 200 ms).
- **SC-007**: With no bad-pixel map, a simulated frame at binning 2 that holds three hot
  pixels and a star has exactly the three repaired and the star's brightest sample left
  as it was (US2-S11).
- **SC-008**: While a set of darks is exposing, an imaging frame and a settings change
  asked for through the console's routes are refused with the set's holder name, and a
  set asked for while another taker holds the camera is refused with that taker's name
  (US6-S9).

## Assumptions

- **030, 031, 032 and 033 have landed.** `030-imaging-camera-link`, `031-imaging-frames-on-console`,
  `032-imaging-solve-and-aim-point` and `033-focus-reading-and-aid` are on `main` at
  80f414f, with the interfaces their drafts gave, and 031 with the lease and the frame's
  set temperature that the series audit's findings on 031 added; this trio was
  re-anchored to their code on 2026-11-19 (plan.md § Prerequisites 1, 2, 5 and 11).
  Every `path:line` here was checked at 49bb210, the drafting worktree's HEAD, and read
  again by its content on `main` at 80f414f, whose line numbers are cited, the four lines
  of `GLOSSARY.md` among them (Clarifications); every file 030, 031, 032 or 033 created
  is named without a line.
- **The plan document's `GLOSSARY.md` rewrite is on `main`**, since 030's flip
  (`55e41f5`): two stories insert their entries after entries only the rewrite has
  (plan.md § Prerequisites 6).
- **Out of scope:**
  - turning a sub into colour, registering it and stacking it (035);
  - any change to the scope node, the scope link or the camera port;
  - an imaging frame taken without saving a file (owner decision 2);
  - recording or matching on the camera's other controls, white balance among them,
    which no spec of this series records: 030's INDI adapter sets white balance neutral
    each time it connects (owner decision 17, settled by amendment 2);
  - the imaging camera's lease itself, which is 031's (031 US9): this spec takes it and
    adds nothing to it (owner decision 23, settled by amendment 1);
  - waiting for the sensor to reach its set temperature, and holding subs back until it
    has (owner decision 24);
  - dead or cold samples, hot columns and any map made from a master flat;
  - scaling a dark to another exposure or temperature, and a separate master of offset
    frames;
  - a flat taken of the twilight sky, and any search that follows a changing light;
  - choosing a flat label in use, and renaming or editing a master;
  - adding new frames into an existing master;
  - a cover or flat panel control under `finder --simulate` (owner decision 18);
  - a calibrated picture, and saving a calibrated sub (owner decision 19);
  - stopping a set or removing a master from an MCP client (owner decision 16);
  - an imaging angle on a night whose aim point was tapped (owner decision 20);
  - a browser test of the block: it holds no picture and uses existing classes, and
    031's night check of the Imaging page already covers every pixel of the page.
- **Outside the factory:** rebuilding the console; the field checks at the telescope: a
  dark set and a flat set taken on the real camera; whether real darks pass the three
  parts of the dark test at the exposures used, with the offset level, the noise and
  the count of bright samples they read, and how often `anyway` is needed; whether the
  cooler holds the sensor within 1.0 °C of its set temperature; whether a
  saturated frame of the real camera reads the full scale of its samples, and whether
  the search finds an exposure for the owner's panel; how many bad pixels the map
  holds; how evenly the telescope lights the sensor, which the research note has not
  established (`docs/research/2026-11-15-imaging-camera.md` § 13. Not established); a
  calibrated sub compared with the raw one, through 035's stack (owner decision 19);
  the disk a set uses; and a design review of the block on a phone in the dark.
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
  implements: [FR-006, FR-007, FR-008, FR-009, FR-010, FR-011, FR-040]
US3:
  depends_on: []
  depends_on_merged: [US2]
  implements: [FR-012, FR-013, FR-014, FR-015, FR-016, FR-041]
US4:
  depends_on: []
  depends_on_merged: [US2, US3]
  implements: [FR-017, FR-018, FR-019, FR-020]
US5:
  depends_on: []
  depends_on_merged: [US3, US4]
  implements: [FR-021, FR-022, FR-023]
US6:
  depends_on: []
  depends_on_merged: [US4, US5]
  implements: [FR-024, FR-025, FR-026, FR-027, FR-028, FR-042, FR-043, FR-044]
US7:
  depends_on: []
  depends_on_merged: [US6]
  implements: [FR-029, FR-030, FR-031, FR-032, FR-033]
US8:
  depends_on: []
  depends_on_merged: [US7]
  implements: [FR-034, FR-035, FR-036]
US9:
  depends_on: []
  depends_on_merged: [US7]
  implements: [FR-037, FR-038, FR-039]
US10:
  depends_on: []
  depends_on_merged: [US7]
  implements: [FR-045]
```

Every edge is a merge edge, because each dependent imports or extends what its
predecessor creates. US1 to US7 run one after another; US8, US9 and US10 may run
together.
Where a story names two predecessors, the second edge follows from the first; it is
written out because the story writes a file the other one made.

- **US2 after US1**: US2-S8 and US2-S11 calibrate frames of the simulator's patterns.
- **US3 after US2**: the library's `calibrate` calls US2's, and US3 adds its `pub mod`
  line to US2's file and its `mod` line after US2's in the test binary.
- **US4 after US2 and US3**: the console's state holds US3's library, adds its own
  `pub mod` line after US3's, and adds the calibration object to the imaging module
  that US2 gave its `pub mod calibration;` line.
- **US5 after US3 and US4**: the map is kept by US3's library and added to US2's file,
  and its paragraph goes into the README section US4 writes.
- **US6 after US4 and US5**: a set of darks attaches US5's map and extends US4's console
  state, routes and route list. Its tests use US1's cover and US4's console rig.
- **US7 after US6**: flats reuse the set's runner and its dark test, write the same
  files, and import US6's scripted camera.
- **US8 after US7**: the tools pass through to US6's and US7's actions.
- **US9 after US7**: the block calls US4's, US6's and US7's routes and shows their
  words. It writes only the page, `DESIGN.md` and `.impeccable/design.json`, none of
  which US8 writes, so the two may run together.
- **US10 after US7**: it keeps the imaging angle in US7's flat set, in `service.rs`,
  adds its `mod` line after US7's in the `console` binary's `main.rs`, and imports the
  flats helpers US7's test module declares. It was part of US7 until the round-1 probe
  build measured US7 at 67,264 bytes, over the budget; the owner cut it by scenario, so
  US7 keeps US7-S1 to US7-S4 and US10 takes the old US7-S5 to US7-S7. It has no edge to
  US8 or US9, nor they to it: neither writes `service.rs`, the `console` binary's
  `main.rs` or US10's module (US8 writes the MCP tools, their tests and the README; US9
  the page, `DESIGN.md` and `.impeccable/design.json`), and neither reads the angle a
  master flat keeps, since US9's masters are fixtures of its own test. So the three may
  run together.
