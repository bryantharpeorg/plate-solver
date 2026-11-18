---
state: draft
depends_on_landed: ["025-frame-capture", "028-finder-tests-do-less", "030-imaging-camera-link"]
# Drafted 2026-11-15 against `main` at dbc2b5b from
# docs/plans/2026-11-15-specs-030-037-imaging.md, by the grilling session of that day,
# as the console's half of one trio, 030-imaging-camera-link. Reviewed that day and the
# review's findings applied; then audited beside the other trios of the series and the
# audit's findings applied (US9, the camera's lease; the plan document's Amendments 1
# and 6). Split out the same day, at the owner's ask, so that a builder reads a shorter
# spec: 030 keeps the scope node's side and the link, and this trio is the console's
# side (§ Clarifications, the last session).
# Probe-built on 2026-11-17 (round 1, every story), on top of 030's round-1 probe
# builds: the round-1 probe fixes are in, and the draft's second story, which measured
# over the budget, is split in four as measured, so the trio holds nine stories (plan.md
# § Refinement Record). It builds on code that 030-imaging-camera-link created; every
# interface of 030 named here was taken from 030's plan.md and read again on `main`
# after 030 landed.
# Re-anchored 2026-11-18 to `main` at 49f8dcd (49f8dcd2e82de22b0335590d4fd60d286e2429dc),
# where 030's eight stories have landed: every citation was compared line for line with
# 49bb210, and the ones that moved are corrected (plan.md § Prerequisites 2).
# Revised 2026-11-17 with a note on the owner's answer to 035's owner decision 13: a
# stack's subs are saved in its own folder, through a folder 035 lets a lease name
# (§ Clarifications, the last session); nothing this spec builds changes.
# It stays draft until the owner settles the owner decisions and flips it to `ready`,
# which gives the approvals listed under "Owner approvals" and dispatches it.
---

# Feature Specification: Imaging Frames on the Console

**Feature Branch**: `031-imaging-frames-on-console`

**Created**: 2026-11-15

**Status**: Draft

**Input**: Spec 030 gives the scope node an imaging camera and the scope link a service,
`ScopeNodeImaging`, that describes the camera, changes its settings and answers one
exposure with the imaging frame, untouched, as a header and chunks. This spec is the
console's side of that link, and with 030 the base of the other six of the series
(specs 030 to 037). The console opens a second connection to the scope node for that
service alone, so that frame bytes never share the heartbeat's connection; keeps the
camera's state in an `imaging` object; saves each imaging frame as a FITS file; shows
the latest on a new Imaging page, red at night with colour while a control is held; and
offers three MCP tools. Its imaging state holds the camera for one holder at a time, so
that the later specs' focus aid, calibration sets, stack and imaging run share it
without a lock of their own. The simulated imaging camera of 030 stands in for tests
and `finder --simulate`. Nothing here solves, focuses, calibrates or stacks.

## Clarifications

### Session 2026-11-15 (the interview's decisions, from the plan document)

- Q: What does this trio read besides itself? → A: Nothing about the feature, apart from
  the interfaces of 030 that the last session below names. `spec.md` holds every
  requirement and scenario, and `plan.md` holds every design decision, the interfaces,
  the shapes that existing tests pin and the alternatives rejected. The binding
  decisions come from `docs/plans/2026-11-15-specs-030-037-imaging.md`,
  `docs/adr/0002-imaging-cameras-through-indi-frames-relayed-console-stacks.md` and
  `GLOSSARY.md`; facts come from `docs/research/2026-11-15-imaging-camera.md`. A story
  that would change a decision escalates rather than re-deciding it.
- Q: Where does the work run? → A: Relay-first. The scope node takes exposures and
  forwards each imaging frame untouched (030); the console saves it, its samples
  untouched.
- Q: Is anything keyed to one camera model? → A: No. Sizes, ranges, the Bayer pattern
  and the cooler are read from the camera's description, which 030's camera adapter
  fills from the camera. The owner's camera, a 4944 × 3284 colour camera, appears below
  only as a size in a worked example.
- Q: What does "half resolution" mean? → A: `binning` 2 in the imaging settings, and
  nothing else. The console sends the setting on; neither it nor the scope node bins
  (030's Clarifications say what is and is not established about a binned frame).
- Q: How do frames reach the console? → A: By 030's service, `ScopeNodeImaging`, on a
  second connection that the console opens only for it. An exposure is answered by a
  stream: one header, then the samples in chunks of at most 256 KiB (262,144 bytes). The
  250 ms heartbeat, whose 1 s of silence stops the mount, never shares a connection with
  frame bytes. One 16-bit 4944 × 3284 frame is 32,472,192 bytes, 124 chunks
  (`docs/research/2026-11-15-imaging-camera.md` § 11. Frame transport).
- Q: What is kept? → A: Every imaging frame, untouched, as a 16-bit FITS file under the
  console's imaging folder. A frame is never rewritten.
- Q: What does the page show at night? → A: Every imaging picture takes the red-only
  filter, with a hold-to-see-colour control: the one written exception to the Red Torch
  Rule. MCP clients get colour.
- Q: What about the three `imaging_camera` numbers of 015? → A: They keep their shape and
  their tests. This spec does not read or write them (owner decision 14).

### Session 2026-11-15 (review of `main` at dbc2b5b, re-read at 49bb210)

- Q: What was found in the code? → A: Four things on this side of the link, each read at
  dbc2b5b and its citation re-read at 49bb210:
  1. The console builds every client from clones of one channel
     (`crates/finder/src/link.rs:377`), sends a heartbeat every 250 ms and treats 1 s of
     silence as a lost link (`crates/finder/src/link.rs:29`,
     `crates/finder/src/link.rs:30`); the node's watchdog stops the mount after 1 s
     without a heartbeat (`crates/scope-node/src/mount/watchdog.rs:42`). 025 refused raw
     frames while anything moves for this reason. Imaging frames run while the mount
     tracks, so they need their own connection.
  2. `LinkManager::start`, and `start_with_filter`, which it calls since 026, keep
     neither their transport nor their session
     (`crates/finder/src/link.rs:236`), and `Transport::connect` is private
     (`crates/finder/src/link.rs:165`). Under `--simulate` the link is paired through a
     private session the console's own session never sees
     (`crates/finder/src/simulate.rs:104`). So the second connection is opened by the
     link manager itself, from the pairing it was started with.
  3. `run` in the link already takes eight parameters, under an `allow` of clippy's
     limit (`crates/finder/src/link.rs:402`). This spec adds none.
  4. An existing test calls every listed MCP tool with empty arguments and checks the
     result for the token (`crates/finder/tests/mcp/motion.rs:517`); the three new
     tools answer such a call at once on a console that takes no imaging frames
     (US8-S5).
- Q: Does the constitution's "detection input is 8-bit grayscale" bind imaging frames? →
  A: No. An imaging frame is never detection input in this spec: the finder camera's
  frames and star detection are untouched.
- Q: Which other drafts write the same files? → A: Three are known, each `state: draft`
  on a branch of this repository and none on `main`: `026-mount-sky`,
  `021-chart-orientation` and `022-altaz-guidance`. Their Refinement Records write files
  this spec writes too. 026 writes nine of them: `crates/finder/src/api.rs`,
  `crates/finder/src/link.rs`, `crates/finder/src/lib.rs`, `crates/finder/src/main.rs`,
  `crates/finder/README.md`, `crates/finder/web/src/api.ts`,
  `crates/finder/web/src/App.tsx`, `crates/finder/web/dist` and `DESIGN.md`. 021 writes
  six: that README, `crates/finder/src/mcp/tools.rs`, `api.ts`, `App.tsx`, `dist` and
  `DESIGN.md`. 022 writes four: the README, `api.ts`, `dist` and `DESIGN.md`. None names
  this spec in its `depends_on_landed`, this spec names none of them, and the factory
  runs four epics at once (`ergane.yaml:196`), so this spec and one of them flipped
  together would build on the same files together. The rule: whichever of this spec and
  one of those is flipped second adds the other to its `depends_on_landed` and is
  re-anchored after the other lands (plan.md § Prerequisites 8). All three have since
  landed (026 at 6cc8724, 021 at 2f6a777, 022 at 20ce2fe), and this trio is re-anchored
  on top of them at 49f8dcd, so nothing is left to order. No draft numbered 023
  was found on any branch read here; whether one exists elsewhere is the owner's to say
  before the flip. Specs 032 to 037 build on this one and write its files after it.
- Q: Where does this trio depart from the plan document's § Interfaces shared across the
  specs? → A: Nowhere now. The review found one departure on this side of the link, and
  the plan document's § Amendments after 030, 031, 032 and 034 were drafted and reviewed
  settle it as this trio has it: frames go under `imaging/` in the state folder, named
  by the console's own frame id (Amendment 3; owner decisions 3 and 4). The three on the
  scope node's side are 030's; the plan document now carries all three, and one of them
  is still an open owner decision there (030's owner decision 9, `save` on
  `ExposeRequest`).

  The console's routes live in `crates/finder/src/imaging/routes.rs`, a file the plan
  document's module list lacked when the review read it and now names; 032 to 037 add
  their routes there. That section's list of an imaging frame's FITS cards now holds the
  cards this trio adds (owner decisions 5 and 16). The keys of the view's `frame` this
  trio adds (FR-009) are an addition, which the amendments' preamble covers ("where a
  trio's reviewed text is more exact than this plan, the trio's `plan.md` is the
  interface"), not a departure.
- Q: Which FITS keywords beyond the plan document's first list were checked, and against
  what? → A: On 2026-11-15. `XORGSUBF` and `YORGSUBF`, in the SBIG FITS extension
  document, are a sub-frame's origin "relative to whole frame in binned pixel units".
  `SET-TEMP`, in MaxIm DL's FITS header definitions, is the set temperature in degrees
  Celsius, absent when none was set. `ROWORDER` is among the cards INDI writes
  (`docs/research/2026-11-15-imaging-camera.md` § 2. INDI), and `BSCALE` 1 goes with
  `BZERO` in the FITS convention for unsigned samples. `FRAMEID`, `PURPOSE` and
  `SERIALNO` are this project's own: no convention read gives a camera's serial a
  keyword. Both documents are in the research note's source list (plan.md
  § Prerequisites 5). The plan document's list now holds all eight.

### Session 2026-11-15 (the audit of 030 to 037, and the plan document's amendments)

- Q: Who holds the imaging camera when later specs take frames one after another? → A:
  One holder at a time, on the console's imaging state, as the plan document's
  Amendment 1 has it: "The console's imaging state (031) holds one lease naming the
  holder; a second taker is refused with the holder's name. Each later spec takes and
  releases that lease and adds no lock of its own." The reviewed draft had no holder:
  its `set` and `take` refused only while an imaging frame was in flight, and 033, 035,
  036 and 037 each wrote down the same stand-in for a lease that none of them may
  create. US9 builds it, name for name as those trios assume it (`Lease`,
  `Imaging::lease`, `Imaging::holder`, `Lease::set`, `Lease::take`), after every story
  that writes the console's imaging state, so that it is the last writer of each file
  it shares (owner approval 12).
- Q: Where does a frame's region of interest come from? → A: From the scope node, with
  the frame, as Amendment 6 has it: "A region of interest is recorded with the frame (its
  origin and size on the sensor, in the header and the FITS file)." 030's `ImagingFrame`
  and the header carry the region in force for the exposure; the console's file writes
  its origin as `XORGSUBF` and `YORGSUBF`, as the scope node's own copy does; the size
  in the file is `NAXIS1` and `NAXIS2` with `XBINNING` (owner decision 16). The view's
  `frame.roi` keeps its shape, so 032 to 037 change nothing.
- Q: Which set temperature does a saved frame carry? → A: The one the console's settings
  held, with the cooler on, when the frame was asked for: the value its `SET-TEMP` card
  is written from, now also in the view as `frame.set_temp_c` (FR-009), so that 034 reads
  a frame's set temperature from the frame and not from settings that may have changed
  since (034's owner decision 22).
- Q: Is the hold-to-see-colour control one control or one per picture? → A: One. US7
  builds it as a hook, `crates/finder/web/src/useColourHold.ts`, and 035's stack picture
  uses the same hook. The exception in `DESIGN.md` names both pictures (owner
  approval 6).
- Q: Can a test in the `console` binary call an MCP tool? → A: Yes: every test binary
  includes the shared support tree, and `crates/finder/tests/motion/mount_identity.rs:4`
  already binds `crate::support::mcp` from another binary. US9's test reaches
  `set_imaging_camera` and `take_imaging_frame` through `crate::support::mcp`'s `app`
  (`crates/finder/tests/support/mcp.rs:28`), `call`, `send` and `body`, so that the lease's
  refusal is proved at the MCP door without a new module in `mcp`.
- Q: What is `RoiEdit`? → A: The region of interest in a settings change, defined by US1
  beside `SettingsEdit`, which names it: sensor pixels, unbinned, the body's
  `{"x":…,"y":…,"width":…,"height":…}` (plan.md § The console's imaging state). 033 builds
  `Some(Some(RoiEdit { .. }))` for a tile.

### Session 2026-11-15 (the split into two trios)

- Q: Why are there two trios where one was drafted? → A: The owner asked for 030 to
  be split if that helps its builders. One trio held thirteen stories on both sides of
  the scope link, more than a builder should read for one story. It was cut where the
  scope link is. This trio holds the console's side, six stories: the console sees the
  imaging camera (the second connection, the session's `imaging` object, the settings
  route); taking an imaging frame and saving it as FITS; the colour preview; the Imaging
  page with the night exception; the three MCP tools; and the camera's one holder.
  `030-imaging-camera-link` keeps the scope node's side and the link, seven stories,
  and this trio names it in `depends_on_landed`. (Those are the counts at the split.
  Round 1's probes split 030's fourth story in two and this trio's second in four, so
  030 landed with eight stories and this trio holds nine.) No decision changed in the split. One
  story straddled the link, the scope node's own copy of a frame, and was cut there:
  what the scope node does is 030's US7, and what the console shows of it is US5-S4
  here.
- Q: What does this spec take from 030? → A: These, by name, from 030's plan.md as
  drafted. 030 has landed, and each was read on `main` at 49f8dcd and found as written;
  each is named without a line:
  - `crates/scope-link/proto/scope_node/v1/scope_node.proto` (030 US3): the service
    `ScopeNodeImaging` with its calls `DescribeImaging`, `ImagingStatus`,
    `ConfigureImaging`, `Expose` and `AbortExposure`, and its messages `ImagingInfo`,
    `ImagingRange`, `ImagingRoi`, `ImagingSettings`, `ImagingStatusReply`,
    `ExposeRequest`, `ExposeEvent` and `ImagingFrameHeader`; and what the node answers
    with: `Unimplemented` when it was built before 030, `FailedPrecondition` and
    `no imaging camera` when it has none, `InvalidArgument`, `ResourceExhausted`,
    `Aborted` and `Internal` with the camera's words;
  - `crates/scope-node/src/imaging/mod.rs` (030 US1): the trait `ImagingCamera` and its
    types `ImagingInfo`, `ImagingSettings`, `ImagingStatus`, `Exposure`, `ImagingFrame`,
    `Roi`, `BayerPattern` and `ImagingError`, which a test's own camera implements;
  - `crates/scope-node/src/imaging/synthetic.rs` (030 US1): `SyntheticImagingCamera`,
    with its description, starting settings, sample rule and cooler's rule;
  - `crates/scope-node/src/service.rs` (030 US3 and US7): `NodeService::with_imaging`
    and `with_imaging_save_dir`;
  - `crates/simulator/src/imaging.rs` and `crates/simulator/src/node.rs` (030 US2 and
    US3): `SimImagingCamera` with `with_stars` and `with_noise`, `SimStar`,
    `Simulator::imaging_truth_at`, and `SimNode::new`, which attaches a
    `SimImagingCamera`.

  plan.md § What this trio takes from 030 gives each as this trio uses it.

### Session 2026-11-17 (the owner's answers)

- Q: Is every imaging frame saved under `frames/<UTC date>/`? → A: Every frame this
  spec takes is, but a stack's subs are not. The owner settled 035's owner decision 13
  as a folder per run: a stack's subs are saved in `runs/<stack id>/subs/`, beside the
  stack's own files. 035's US5 adds `Lease::save_in` to this spec's lease and the save
  of a frame taken under a lease that names a folder; no story here writes or calls
  it, and FR-011, the id rule and every scenario here stand as written (plan.md
  § Receiving and saving, **A stack's subs are saved elsewhere**).

### Session 2026-11-17 (round 1 probes)

- Q: What did the probe builds find? → A: Every story was built from this trio in a
  throwaway worktree, on top of 030's round-1 probe builds, tests first, with both
  gates, `cargo deny check` and `cargo machete` (plan.md § Refinement Record, round 1).
  Every interface built as plan.md writes it, with private additions only, and every
  story's tests passed. Three defects were blocking: the draft's second story measured
  85,621 bytes against the 64 KiB budget, and the split prepared in plan.md § Risks
  still left 71,496; one of its scenarios waits 152 s for a header, past the 60 s the
  shared setup allowed a wait; and the lease's story could not make its check under
  `take`'s lock without writing `crates/finder/src/imaging/frames.rs`, which it did not
  declare. Forty-one defects were reported in all; their fixes are in this trio, and
  none adds a requirement.
- Q: How is that story split? → A: In four, cut where its measured code divides: US2
  takes an imaging frame and saves it (the draft story's first, second, sixth, seventh
  and eighth scenarios, now US2-S1 to US2-S5); US3 reads a saved file back (`decode`,
  the tenth, now US3-S1); US4 holds the checks on what the scope node sends (the fifth
  and eleventh, now US4-S1 and US4-S2, the two that need a test's own
  `ScopeNodeImaging`); and US5 the refusals, abort and the scope node's copy (the third,
  fourth, ninth and twelfth, now US5-S1 to US5-S4). Each has a test module of its own:
  `imaging_frames`, `imaging_fits`, `imaging_checks` and `imaging_control`. The stories
  after them are renumbered: the preview is US6, the page US7, the MCP tools US8 and
  the lease US9 (owner decision 1).
- Q: What changed in the code this trio stands on? → A: 026 landed after the trio was
  read. `LinkManager::start` now calls `start_with_filter`, which spawns `run`, and
  `run` takes eight parameters under an `allow` of clippy's limit. The second
  connection's clones are kept in `start_with_filter`, and no signature changes
  (plan.md § The link). This was read on the probe base; the citations are re-anchored
  with the rest (plan.md § Prerequisites 2).
- Q: Which assertions did the probes add? → A: Each where a build broken on purpose
  passed every test (the probes' bite checks), or where a scenario as worded could not
  pass:
  - US1-S3: `{"forward_kib_per_s":100.5}`, a fraction inside the range, where 2.5 was
    refused for being below 64;
  - US1-S5: the connections are counted once star frames arrive, since `node.stars` is
    still null when imaging is first `ready`;
  - US1-S6: 1.5 s before the silence, so that a status read is in flight when the node
    stops answering;
  - US2-S2: a fourth frame, with the cooler off, has no `SET-TEMP`;
  - US2-S4: the time in hand within 0.1 s, where 1 s let a rate in thousands of bytes
    pass;
  - US2-S6, new: the purposes `dark` and `flat` give `IMAGETYP` `DARK` and `FLAT`;
  - US3-S1: a whole real, `GAIN` 100.0, pins how a real is written, and the shared setup
    compares a card's number as a number;
  - US4-S1: the stall's time, 30.0 to 30.2 s, and each frame waited for up to 160 s;
  - US5-S2: `status.exposing` true 1 s into the exposure, before the abort;
  - US6-S1 and US6-S2: green the mean of both its samples, the Bayer pattern read, a
    block's mean and not its first sample, and the span's floor of 1;
  - US6-S3: a frame with no whole 2 × 2 cell is saved with no preview;
  - US6-S4: every saturated pixel near the star, since "the brightest" had no one
    meaning;
  - US7-S1: the top bar is the first element of role `banner`;
  - US7-S4: while `receiving`, `Take imaging frame` is disabled and `Abort exposure`
    shown;
  - US7-S6: a failed read changes nothing shown;
  - US8-S1: the seven keys that refuse null are published as not nullable;
  - US8-S2: `get_session` carries `imaging` over MCP, and a parse error has the route's
    words;
  - US8-S4: the aborted case waits for the page's frame to end first;
  - US8-S5: "at once" read on tokio's clock;
  - US9-S5: the test's own `Imaging` takes a `ConsoleClock` over the rig's clock.
- Q: Which behaviours did the probes settle where the plan left a choice? → A: These,
  each written where plan.md gives the behaviour: `roi: null` is sent as the whole
  sensor's region; `status.exposing` is the console's, from its latest frame (FR-009);
  `EXPTIME` is the header's exposure; an error on the stream, or an event that is not a
  chunk, after the header is an incomplete frame; `ended` answers the frame with its id
  as it ended, which the console keeps beside the latest; the id rule lists the folder
  of the take's UTC date; `SettingsEdit` has deserialize helpers of its own; `jpeg`
  answers `None` for a picture with no pixels, and a Bayer string other than the four
  is read as mono; the page draws its checkbox in ink, seeds its drafts on the first
  render and goes home from `/imaging` when a frame is picked; `ColourHoldBind`'s
  `onPointerDown` takes no event; `set_imaging_camera` parses with the route's own
  `sim_json`; the README's three tools are a short list in `## MCP clients`; and the
  lease's check is made in `frames.rs`, which US9 now writes (plan.md § The camera
  lease).
- Q: What is still caught by no test? → A: Which percentile the preview reads the
  sky's noise from, for which no exact check was found, and the JPEG's quality; both
  belong to the preview's look, which owner decision 7 puts outside the gates. And that
  a dropped lease clears only its own grant, which no order of calls can show, since a
  lease is granted only while none is out.

## Owner decisions

Each is a design or taste call the plan document leaves open. Decisions 3, 4 and 8 are
settled by the plan document's Amendments 3, 3 and 5, as recommended here, and the
region of decision 16 by Amendment 6; each says so, and keeps its other answer only as
the record of what was closed. The rest are open: the owner settles each before the
flip, and a different answer is written into the trio and re-validated, never left to a
story.

1. **Nine stories on this side of the link.** Recommended: nine, so that each stays
   under 64 KiB: the console's own part is six stories (sees the camera; takes and saves
   a frame; reads a saved file back; the checks on what the scope node sends; refusals,
   abort and the scope node's copy; the preview), the page and the MCP tools are one
   each, and the camera's lease (the plan document's Amendment 1) is a story of its own,
   US9, after every story that writes the console's imaging state. The draft had six,
   with the second to fifth in one story, which the round-1 probe build measured at
   85,621 bytes; the split prepared in plan.md § Risks still left 71,496. Instead:
   three, as the plan document's first list had this side (the console; the page; the
   tools), with two stories likely to pass the limit and the lease folded into one.
2. **`IMAGETYP` in a saved file.** Recommended: `DARK` for the purpose label `dark`,
   `FLAT` for `flat` and `LIGHT` for any other, as the scope node's own copy has it
   (030's owner decision 7, which makes the label itself: 1 to 16 lower-case letters,
   free). `LIGHT` is a named exception to the glossary's "avoid light": it is the value
   other programs read in that card, not this project's word for a sub. Later specs add
   labels with no change to the writer. Instead: the two-word values some programs
   write.
3. **Where frames go** (settled by the plan document's Amendment 3, as recommended; the
   review had it as a departure, since § Interfaces shared across the specs then read as
   `frames/` directly under the state folder). Recommended: `imaging/` inside the
   console's state folder, so `--state` moves it and `--simulate` keeps simulated nights
   apart, or the folder `--imaging-dir` names. Instead: `frames/` directly in the state
   folder.
4. **Whose frame id names a file** (settled by the plan document's Amendment 3, "Files
   are named by the console's own frame id", as recommended). Recommended: the console's
   own count, six digits,
   starting after the largest id already in that UTC date's folder, so no file is ever
   replaced; the node's id restarts with the node. Instead: the node's id.
5. **The FITS writer and its cards.** Recommended: hand-written in the console, one
   image, the cards the plan document lists, which now include `BSCALE`, `ROWORDER`,
   `FRAMEID` and `PURPOSE` and those of owner decision 16. Instead: a FITS crate
   (`fitsio` wraps a C library; the pure-Rust writers are under a year old).
6. **An 8-bit camera.** Recommended: its frames are saved with `BITPIX` 8, untouched.
   Instead: widened to 16 bits.
7. **The preview.** Recommended: superpixel colour, each channel's median to black,
   the larger of the 99.9th percentile and 30 times the sky's noise to white, an asinh
   curve of strength 10, longest side at most 1280 px, JPEG quality 85, the last eight
   kept in memory. Its look is for the design review, not a gate. Instead: another
   stretch; previews saved beside the frames.
8. **The forwarding rate limit** (settled by the plan document's Amendment 5, "a
   console setting sent with each exposure, not a node config key", as recommended).
   Recommended: a console setting, `forward_kib_per_s`, off by default, set from the
   Imaging page's `Forwarding limit` field or by an MCP client, and sent with each
   exposure as `max_bytes_per_s` (030's owner decision 8). Instead: a key in the node's
   config.
9. **Asking for the scope node's copy.** Recommended: a console setting,
   `save_at_node`, off by default, set from the Imaging page's checkbox or by an MCP
   client, and sent with each exposure as `save`; the view names the node's file, or
   says why there is none (US5-S3, US5-S4). Whether the scope node keeps a copy at all
   is 030's owner decision 9. Instead: no such setting, if that decision drops the
   copy.
10. **The Imaging link in the top bar.** Recommended: shown while a scope node is paired,
    so a console with no scope node keeps today's bar. Instead: always.
11. **Hold to see colour.** Recommended: shown only at night, colour only while held, at
    most 10 s a hold, built once as the hook `useColourHold` that every picture from the
    imaging camera uses (this spec's imaging frame preview, 035's stack). Instead: a
    switch; or no time limit; or the control written into each page block.
12. **How the Imaging page keeps up.** Recommended: it reads `GET /api/imaging` once a
    second while it is open. Instead: a stream.
13. **`take_imaging_frame` waits.** Recommended: it answers when the frame is saved, with
    the same preview the page shows. Instead: answer at once with an id; a smaller JPEG.
14. **015's `imaging_camera` numbers.** Recommended: untouched by this spec. Instead: fill
    them from the described camera when they are unset.
15. **A browser check of the night exception.** Recommended: one Playwright file in the
    simulate project, which needs the edit of `playwright.config.ts` that owner approval
    8 names. Instead: component tests only.
16. **What a file says about where and how its frame was taken.** A frame taken with a
    region of interest must say which part of the sensor it is, and files are never
    rewritten, so this is settled before the first file exists. For the region it is
    settled by the plan document's Amendment 6 ("A region of interest is recorded with
    the frame (its origin and size on the sensor, in the header and the FITS file)"):
    the scope node records the region in force for the exposure and sends it in the
    header as `roi` (030's owner decision 12); the console's `frame.roi` starts as the
    region its settings held and is replaced by the header's when the header arrives;
    `XORGSUBF` and `YORGSUBF`, the origin in binned pixels, are written from it, always;
    the size in the file is `NAXIS1` and `NAXIS2` with `XBINNING`. A camera changed by
    another INDI client is therefore stamped as it really was. Open, for the rest of
    what a file says: Recommended: the console stamps `SET-TEMP`, the set temperature
    its settings held when the cooler was on with one, and `SERIALNO`, the camera's
    serial when it has one, from what it held when it asked for the exposure; the
    node's copy carries neither. Instead: the scope node stamps those two as well, from
    the camera's own state, which adds two more fields to the header of 030's US3 and
    puts both cards in the node's copy. (The review's recommendation, the console
    stamping the region too with nothing on the scope link, and its alternative (c),
    leaving the region to 033, are closed by the amendment.)

## Owner approvals

Flipping this spec to `ready` approves the settled answer to every owner decision above,
and the following. Each is named in the FR or success criterion that makes it.

1. **One new module tree**: `crates/finder/src/imaging/` (FR-001).
2. **One flag of `finder`**: `--imaging-dir` (FR-007).
3. **Five routes and one session field**: `GET /api/imaging` and
   `PUT /api/imaging/settings` (FR-002 to FR-004), `POST /api/imaging/frame` and
   `POST /api/imaging/abort` (FR-008, FR-013),
   `GET /api/imaging/frames/{id}/preview.jpg` (FR-018), and the `imaging` object of
   `GET /api/session`, which the MCP tools that return the session carry too (FR-005).
4. **Three MCP tools**: `get_imaging_camera`, `set_imaging_camera` and
   `take_imaging_frame` (FR-025 to FR-027).
5. **A third page route, `/imaging`**, and its link in the top bar (FR-020).
6. **`DESIGN.md`: the exception to the Red Torch Rule**, with these words exactly
   (FR-023). The rule's paragraph becomes:

   > **The Red Torch Rule.** At night every pixel is red or black: no white, no other
   > hue, and no token brighter than Night Ink. Anything new, images and embedded views
   > included, must take a night token or the red-only filter before it ships. There is
   > one exception. A picture from the imaging camera, an imaging frame's preview or a
   > stack's picture, shows its own colours while the observer holds the Hold to see
   > colour control under it, for at most 10 seconds a hold, and is red again the moment
   > the control is let go. Nothing else on the page changes, nothing keeps colour on,
   > and no other picture has the control.

   The matching line under Don't becomes:

   > - **Don't** show a white or non-red pixel in night mode, including in images,
   >   previews, embedded views and the first paint. The only exception is the Red
   >   Torch Rule's held colour.

   The Top bar line gains `the Imaging link while a scope node is paired,` after
   `the Setup link,`, and one component entry, Imaging page, is added after Recording
   with the words of plan.md § `DESIGN.md`; `.impeccable/design.json` lists the
   component.
7. **No story writes `GLOSSARY.md` or the plan document.** The terms this spec uses
   (imaging frame, and the changed imaging camera) are written by the plan document's
   own change to `GLOSSARY.md`, which landed with 030's flip. That change also carries
   one entry it lacked when this trio was drafted (plan.md § Prerequisites 5):
   **Preview** widened to "An occasional small JPEG of a star frame's image, for the
   page to show under the chart, or a colour JPEG of an imaging frame, for the Imaging
   page and MCP clients", since this spec uses the word for both. (The **Purpose
   label** entry is 030's owner approval 6.)
8. **Edits to existing test and gate files** (SC-002), and no others:
   - one `mod` line for each new finder test module, in alphabetical order, in
     `crates/finder/tests/console/main.rs` (`mod imaging;`, `mod imaging_checks;`,
     `mod imaging_control;`, `mod imaging_fits;`, `mod imaging_frames;`,
     `mod imaging_lease;`, `mod imaging_preview;`), `crates/finder/tests/cli/main.rs`
     (`mod imaging;`, `mod imaging_frames;`) and `crates/finder/tests/mcp/main.rs`
     (`mod imaging;`), as 027's layout asks of every new finder test;
   - in `crates/finder/web/playwright.config.ts`, the string `"**/imaging.spec.ts"`
     added to the default project's `testIgnore` list and to the simulate project's
     `testMatch` list. The second entry takes that list's line past Prettier's 120
     columns (`crates/finder/web/.prettierrc.json:2`), so `npm run format:check`, a
     command of the `test` gate, requires the `testMatch` list written one entry a
     line; that re-wrap is part of the approved edit. Nothing else in the file changes
     (owner decision 15).
9. **Document sections** in `crates/finder/README.md`: `## Imaging frames` with its
   paragraph on the camera's one holder, five rows of the API table, a short list of the
   three imaging tools in `## MCP clients`, before `### MCP under WSL2`, and one sentence
   in `## Night mode` on the held colour (FR-007, FR-015,
   FR-019, FR-023, FR-027, FR-034).
10. **Three widened visibilities**: `Transport::connect`, `SessionWrite` and `sim_json`
    become `pub(crate)` (FR-001).
11. **The cards of a saved imaging frame**, which no later spec can change in files
    already written: the plan document's list, which now holds `BSCALE`, `ROWORDER`,
    `FRAMEID` and `PURPOSE` (owner decision 5) and `XORGSUBF`, `YORGSUBF`, `SET-TEMP` and
    `SERIALNO` (owner decision 16); in the order of FR-012. (The cards of the scope
    node's own copy are 030's owner approval 10.)
12. **The imaging camera's one holder** (the plan document's Amendment 1; FR-028 to
    FR-033): the public names `Lease`, `Imaging::lease`, `Imaging::holder`, `Lease::set`
    and `Lease::take` in `crates/finder/src/imaging/mod.rs`, name for name as 033 to 037
    call them; and the refusal 409 `the imaging camera is held by {holder}`,
    which `PUT /api/imaging/settings` and `POST /api/imaging/frame` answer while a lease
    is out, and `set_imaging_camera` and `take_imaging_frame` as a tool error with the
    same words. Specs 033 to 037 take and release the lease and add no lock of their
    own.

## User Scenarios & Testing *(mandatory)*

Every scenario shares these definitions unless it says otherwise. The cameras and the
node they name are 030's; plan.md § What this trio takes from 030 gives their
signatures.

- **The gate command.** Every Rust scenario runs in
  `cargo nextest run --workspace --all-targets --all-features --locked`, the `test`
  gate's first command, each test in its own process.
- **Paused time.** A Rust test of the in-process console runs under
  `#[tokio::test(start_paused = true)]`, so every wait is in virtual time and an elapsed
  time can be asserted exactly. A test that starts a built binary is a plain `#[test]`
  and waits in real time.
- **The stand-in camera** is 030's `SyntheticImagingCamera::new(clock)`: 64 × 48,
  16 bits, RGGB, exposure 1 to 60000 ms, gain 0 to 300, offset 0 to 100, binnings
  `[1, 2]`, a cooler, with gain 100 and offset 10 in force. Its **sample rule**, with
  `binning` b, region of interest origin (x0, y0) and offset o in force: the sample at
  column c and row r of a frame is `256 + 16·o + ((x0/b + c + 3·(y0/b + r)) mod 251)`.
  With the settings it starts with, the sample at column 5, row 2 is 427, the bytes
  `AB 01`. Its **cooler's rule**, which the simulated camera shares: with the cooler
  off, or on with no set temperature, the sensor reads 20.0 °C and power 0.0; on with a
  set temperature T, it reads T at once and power `clamp(2.5·(20 − T), 0, 100)`.
- **A frame's samples** are little-endian, two bytes a sample for 16 bits, row by row
  from the top-left; column c, row r is at byte `2·(r·width + c)`.
- **The sim rig** is a `Simulator` over the test database at the site 40° N 75° W on a
  `VirtualClock` from Unix 1768528800 (`2026-01-16T02:00:00Z`), given `push_to(50.0,
  120.0)`, with no mount unless a scenario connects one, as
  `crates/finder/tests/support/live.rs:41` builds it, and
  `SimImagingCamera::new(sim.clone())` over it. `with_stars` is a builder, so a
  scenario that takes more than one frame of a star "at the truth's centre for
  mid-exposure" builds a camera for each frame, its star placed from
  `sim.imaging_truth_at` for the middle of that frame's own exposure.
- **The node rig** is `NodeService::new` over a `SyntheticCamera` with the token
  `test-token` and default settings (`crates/scope-node/src/service.rs:117`), given
  `.with_imaging(the stand-in camera)`, and served with `routes()` over one in-memory
  stream for each connection.
- **The console rig** is an in-process console as 025's shared setup builds one: the sim
  rig's `Simulator` with a `SimMount::new` connected first and `push_to(50.0, 120.0)`
  after; a `SimNode` over it with a 500 ms frame interval and the token `test-token`,
  served over a fresh duplex for each connection with `memory::serve_routes_on` and its
  `routes()`, as `crates/finder/tests/support/mcp.rs:228` serves its node; a session
  paired to it with the site 40° N 75° W; then `AppState::shared` with `with_session`,
  `with_clock` on the simulator's clock, `with_link`, `with_live()` and
  `with_imaging(R)`. **R** is the child path `imaging` of the test's scratch folder
  (below): a path that does not exist until the console makes it. **The
  stand-in console rig** is the same console paired instead to the node rig, whose
  clocks read the simulator's `VirtualClock` (`crates/simulator/src/clock.rs:28` for the
  node), so its frames follow the sample rule.
- **Scratch folders.** A test's files live under the directory cargo names in
  `CARGO_TARGET_TMPDIR`, and no test builds a path from `CARGO_MANIFEST_DIR` and
  `../../target`: that is 028's rule (`specs/028-finder-tests-do-less/spec.md:413`). A
  finder test module takes its folder from `crate::support::Scratch::new(name)`, the
  helper 028's US5 put in `crates/finder/tests/support/mod.rs:12`, which makes the
  folder and removes it when dropped. Wherever a scenario needs a folder that does not
  exist yet (R, S or D), it is a child path of the scratch folder.
- **Requests** go through the router in-process, as `request` in
  `crates/finder/tests/support/live.rs:90` sends them, with `Content-Type:
  application/json` and no `Origin`. That helper sends no `Host`. A scenario that names
  an `Origin` or another `Content-Type` builds its request with a helper of the module's
  own, with a `Host` of localhost on port 8080, as
  `crates/finder/tests/console/recording.rs:426` does. A foreign `Origin` is
  `http://elsewhere.example`.
- **The MCP door.** A scenario that calls an MCP tool from a module outside the `mcp`
  binary builds a second router over the same state with `crate::support::mcp::app(state,
  "127.0.0.1")` (`crates/finder/tests/support/mcp.rs:28`) and calls a tool with that
  module's `call`, `send` and `body`; MCP is on for localhost by default. A tool error is
  a result whose `isError` is true and whose first text block is the error's words.
- **Waiting until** a condition means reading `GET /api/imaging` every 100 ms until it
  holds, for at most 60 s or the longer time a scenario names, and failing otherwise. A
  console-rig scenario first waits until `state` is `ready`.
- **Reading a FITS file in a test.** A test reads a saved file with code of its own, never
  through the console's or the scope node's FITS code: 80-byte cards up to `END`, the
  header padded to 2880 bytes, then big-endian samples, each 16-bit sample being the
  stored signed value plus `BZERO`. A card's number is compared as a number, not as its
  text; only a scenario that quotes a card's text compares the text.
- **Comparing a view.** A test compares the `camera` and `status` objects of an imaging
  view whole, and the `settings` and `frame` objects key by key, never whole, because
  later stories and specs add keys to those two. A test that builds a scope-link message
  or an `ImagingFrame` ends its literal with `..Default::default()`.
- **The lease** (US9) is `imaging.lease("the test")` on `imaging`, the console's
  `Imaging` as `AppState::imaging` hands it out, and **the holder** is the code that
  holds the `Lease` it answers. A test that "holds no lease" calls `imaging` itself, the
  routes or the MCP door. A settings edit in a Rust call is the `SettingsEdit` that
  `serde_json::from_value` makes of the JSON the scenario gives.
- **A test's own camera, and a test's own node.** Where a scenario says a node's imaging
  camera is "one of the test's own", the test implements 030's `ImagingCamera` and
  attaches it with `with_imaging`; where it says a node's `ScopeNodeImaging` is "the
  test's own", the test implements that service by hand and serves it in the node's
  place. Either ends every `ImagingFrame` and scope-link message literal with
  `..Default::default()`.
- **Oracles.** Each scenario names its oracle: the sample rule, a worked example given
  here, the simulator's truth, the bytes a test's own node sent, or a scenario of 030
  that proves the same value at the scope node.

### User Story 1 - The console sees the imaging camera (Priority: P1)

As the observer at the console, once the scope node is connected I see what imaging
camera it has, its temperature and its settings, and I can change the settings; when
there is no camera, or the scope node is too old, the console says which.

**Why this priority**: Every later imaging action starts from this state, and it is the
first thing checked at the telescope.

**Independent Test**: `crates/finder/tests/console/imaging.rs`, the module `imaging` of
finder's `console` test binary, and, for US1-S7, which starts the built `finder`,
`crates/finder/tests/cli/imaging.rs`, the module `imaging` of `cli`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs both; during
development, `cargo nextest run -p finder --locked --test console imaging::` and
`cargo nextest run -p finder --locked --test cli imaging::`.

**Acceptance Scenarios**:

1. **Given** the console rig, **When** a test waits until `state` is `ready` and reads
   `GET /api/imaging` and `GET /api/session`, **Then** a committed test asserts that the
   first answers 200 with `state` `ready`; `camera` exactly
   `{"model":"simulated imaging camera","serial":"SIM-0001","width":1236,"height":822,"pixel_um":19.12,"bits":16,"bayer":"RGGB","exposure_ms":{"min":1.0,"max":3600000.0},"gain":{"min":0.0,"max":300.0},"offset":{"min":0.0,"max":100.0},"binnings":[1,2],"cooler":true}`;
   `status` exactly
   `{"temp_c":20.0,"cooler_power":0.0,"set_temp_c":null,"exposing":false}`; and
   `settings` with `exposure_ms` 1000.0, `gain` 120.0, `offset` 10.0, `binning` 1, `roi`
   `{"x":0,"y":0,"width":1236,"height":822}`, `cooler_on` false, `set_temp_c` null,
   `forward_kib_per_s` null and `save_at_node` false; that it has no `frame` key; and
   that the session's `imaging` equals that body (oracle: the simulated camera's
   description, 030's US2-S1).
2. **Given** the console rig, **When** a test sends `PUT /api/imaging/settings` with
   `{"exposure_ms":500,"gain":200,"binning":2}`, **Then** a committed test asserts 200
   and `settings` with `exposure_ms` 500.0, `gain` 200.0, `binning` 2 and the other keys
   as in US1-S1; and that `ConfigureImaging` with an empty message, sent by the test on
   a channel of its own to the same node, answers gain 200.0 and binning 2 (oracle: the
   camera itself). **When** it then sends `{"roi":{"x":400,"y":200,"width":400,"height":400}}`
   and later `{"roi":null}`, **Then** `settings.roi` is that region and then the whole
   sensor again. **When** it sends `{"forward_kib_per_s":4096,"save_at_node":true}` and
   later `{"forward_kib_per_s":null}`, **Then** the view shows 4096 and true, then null
   and true. `{}` answers 200 and changes nothing.
3. **Given** the console rig, **Then** a committed test asserts that each of these bodies
   answers 400 with a JSON `error`, the exact text where one is given, and leaves the
   settings as they were:
   - `{"gain":301}`: `gain 301 is outside the camera's range 0–300`;
   - `{"binning":3}`: `binning 3 is not one of the camera's binnings 1, 2`;
   - `{"exposure_ms":0.5}`: `exposure_ms 0.5 is outside the camera's range 1–3600000 ms`;
   - `{"forward_kib_per_s":10}` and `{"forward_kib_per_s":100.5}`:
     `forward_kib_per_s must be a whole number from 64 to 1048576, or null`;
   - `{"set_temp_c":60}`: `set_temp_c must be between -50 and 50`;
   - `{"gain":null}`, `{"gian":1}` and `{"binning":"2"}`: any `error`;
   - `{"roi":{"x":400,"y":200,"width":400}}`, a region without `height`, and
     `{"roi":{"x":400,"y":200,"width":400,"height":400,"z":1}}`, one with a key
     `RoiEdit` does not know: any `error`;
   and that `{"gain":150}` with a foreign `Origin` answers 403, the same body sent with
   `Content-Type: text/plain` answers 415, and the body `not json` sent as
   `application/json` answers 400.
4. **Given** consoles built as the console rig but paired otherwise, **Then** a committed
   test asserts that:
   - with the node served by `into_server()` alone, `state` becomes `no_service`, the
     view has no `camera` key, and `PUT /api/imaging/settings` with `{"gain":150}`
     answers 409 and `this scope node has no imaging service`;
   - with a `NodeService` that has no imaging camera, served with `routes()`, `state`
     becomes `no_camera` and the same request answers 409 and
     `this scope node has no imaging camera`;
   - with a transport whose connector always answers an I/O error, `state` is
     `no_link` and the request answers 409 and `the scope node is not connected`;
   - with a node whose imaging camera, one of the test's own, answers `describe` with
     `Failed("cannot reach the INDI server at 127.0.0.1:7624: refused")`, `state`
     becomes `error`, the view's `error` is that text, and the request answers 409 and
     `the imaging camera is not answering: cannot reach the INDI server at
     127.0.0.1:7624: refused`;
   - built without `with_imaging`, `GET /api/session` has no `imaging` key,
     `GET /api/imaging` answers 200 and exactly `{"state":"off"}`, and the request
     answers 409 and `this console takes no imaging frames`.
5. **Given** the console rig with a connector that counts the streams it hands out,
   **When** `state` is `ready` and the session's `node.stars` is not null (each read
   every 100 ms for at most 60 s), **Then** a committed test asserts that the count is
   exactly 2 and `node.connected` is true, and that after 10 s more the count is still 2.
6. **Given** the console rig, **When** a test applies `{"gain":200}`, waits 1.5 s, calls
   the simulator's `set_unresponsive(true)` (`crates/simulator/src/controls.rs:115`), waits
   until `state` is `no_link`, calls `set_unresponsive(false)` and waits until `state`
   is `ready`, **Then** a committed test asserts that `no_link` came within 3 s and
   `ready` within 15 s, that the view between them had no `camera` key, and that
   `settings.gain` is 200.0 afterwards.
7. **Given** the built `finder` run with `--simulate`, `--state S` and `--listen` on port
   0 of `127.0.0.1`, S a child path of the test's scratch folder, **When** a test reads
   `GET /api/imaging` every 200 ms for at most 20 s, **Then** a committed test asserts
   that `state` becomes `ready` with `camera.model` `simulated imaging camera`. **Given**
   the same with `--imaging-dir D`, **Then** the binary starts and answers the same; and
   `finder --help` names `--imaging-dir`.
8. **Given** the console rig, **When** a test sends
   `{"cooler_on":true,"set_temp_c":-10}`, **Then** a committed test asserts that within
   5 s `status` is exactly
   `{"temp_c":-10.0,"cooler_power":75.0,"set_temp_c":-10.0,"exposing":false}` and
   `settings.cooler_on` is true (oracle: the cooler's rule of the shared
   setup).
9. **Given** the console rig paired to a node whose imaging camera, one of the test's
   own, describes itself as the stand-in camera does and answers an empty `apply` with
   the stand-in's starting settings, **Then** a committed test asserts that, when the
   camera answers `apply` with gain 150 with
   `Failed("the camera refused CCD_CONTROLS: no")`, `PUT /api/imaging/settings` with
   `{"gain":150}` answers 502 and
   `the scope node did not apply the settings: the camera refused CCD_CONTROLS: no`;
   that, when the camera never answers that `apply`, the same request answers 502 and
   `the scope node did not apply the settings: no answer within 30 s` exactly 30 s
   after it was sent; and that `settings.gain` is still 100.0 after each (oracle: the
   test's own camera).
10. **Given** the console rig paired to a node whose imaging camera, one of the test's
    own, notes tokio's clock at each `describe` and answers the first two with
    `Failed("not yet")` and the third as the stand-in camera does, **Then** a committed
    test asserts that the view's `state` is `error` with `error` `not yet` until the
    third call and `ready` after it, and that the second call came exactly 5 s after
    the first and the third exactly 5 s after the second, although the link's status
    changed with every heartbeat meanwhile. **Given** a camera whose `describe` never
    answers, **Then** `state` becomes `error` with `error`
    `the scope node did not answer within 5 s`, between 5.0 and 5.2 s after the
    camera's first `describe` was called.

---

### User Story 2 - Take an imaging frame and keep it as FITS (Priority: P1)

As the observer, I ask for one imaging frame; the console has the scope node take the
exposure, receives the samples on the imaging connection while the mount keeps tracking,
and saves them untouched as a FITS file I can open in any astronomy program.

**Why this priority**: It is the first end-to-end proof that a frame from the focuser
reaches the console's disk, and everything later reads these files.

**Independent Test**: `crates/finder/tests/console/imaging_frames.rs`, the module
`imaging_frames` of `console`, and, for US2-S5, `crates/finder/tests/cli/imaging_frames.rs`,
the module `imaging_frames` of `cli`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs both; during
development, `cargo nextest run -p finder --locked --test console imaging_frames::` and
`cargo nextest run -p finder --locked --test cli imaging_frames::`.

**Acceptance Scenarios**:

1. **Given** the stand-in console rig with `{"exposure_ms":500}` applied, **When** a test
   reads the session's `time`, sends `POST /api/imaging/frame` and waits until
   `frame.state` is `saved`, **Then** a committed test asserts that:
   - the answer was 200 with `frame.id` 1, `frame.state` `exposing`, `frame.purpose`
     `frame`, `frame.exposure_ms` 500.0 and `frame.roi`
     `{"x":0,"y":0,"width":64,"height":48}`, the region the settings held;
   - the saved view has `frame.width` 64, `height` 48, `bits` 16, `bayer` `RGGB`,
     `gain` 100.0, `offset` 10.0, `binning` 1, `temp_c` 20.0, `bytes` 6144, `received`
     6144, `roi` `{"x":0,"y":0,"width":64,"height":48}`, now the header's, `file`
     `frames/2026-01-16/000001.fits` and no `error` or `set_temp_c` key; `frame.start` is
     within 0.001 s of the `time` read before the request;
     `finder::simulate::parse_start` of `frame.date_obs` with `Z` appended is within
     0.0006 s of `frame.start`; and `frame.received_at` and `frame.saved_at` are each
     from 0.49 s to 1.5 s after `frame.start`, `saved_at` not before `received_at`;
   - R holds `frames/2026-01-16/000001.fits`, 11,520 bytes long, and no file whose name
     ends `.part`;
   - read as the shared setup says, the file's cards are, in this order and with these
     values: `SIMPLE` T, `BITPIX` 16, `NAXIS` 2, `NAXIS1` 64, `NAXIS2` 48, `BZERO` 32768,
     `BSCALE` 1, `DATE-OBS` the text of `frame.date_obs`, `EXPTIME` 0.5, `GAIN` 100.0,
     `OFFSET` 10.0, `XBINNING` 1, `YBINNING` 1, `XORGSUBF` 0, `YORGSUBF` 0, `CCD-TEMP`
     20.0, `BAYERPAT` `RGGB`, `INSTRUME` `synthetic imaging camera`, `IMAGETYP` `LIGHT`,
     `ROWORDER` `TOP-DOWN`, `FRAMEID` 1, `PURPOSE` `frame`, then `END`, with no
     `SET-TEMP` and no `SERIALNO`; and its 3072 samples equal the sample rule (oracle:
     the rule, computed by the test).
2. **Given** the console rig with `{"exposure_ms":200}` applied, **When** a test takes a
   frame, applies `{"binning":2}` and takes another, then applies
   `{"roi":{"x":400,"y":200,"width":400,"height":400},"cooler_on":true,"set_temp_c":-10}`
   and takes a third, **Then** a committed test asserts that the first file is
   2,036,160 bytes with `NAXIS1` 1236, `NAXIS2` 822, `XORGSUBF` 0, `YORGSUBF` 0,
   `INSTRUME` `simulated imaging camera`, `SERIALNO` `SIM-0001` and no `SET-TEMP`, its
   view having no `frame.set_temp_c` key; that the second, `000002.fits`, is 512,640
   bytes with `NAXIS1` 618, `NAXIS2` 411, `XBINNING` 2 and `YBINNING` 2; and that the
   third, `000003.fits`, is 83,520 bytes with `NAXIS1` 200, `NAXIS2` 200, `XORGSUBF`
   200, `YORGSUBF` 100, `CCD-TEMP` -10.0 and `SET-TEMP` -10.0, its view's `frame.roi`
   being `{"x":400,"y":200,"width":400,"height":400}` and its `frame.set_temp_c` -10.0
   both while `exposing` and once saved (worked example: 2880 bytes of header and the
   samples padded to a multiple of 2880; the origin cards are the header's region's
   origin over the binning, 400/2 and 200/2). **When** it then applies
   `{"cooler_on":false}` and takes a fourth, **Then** `000004.fits` has no `SET-TEMP` and
   its view no `frame.set_temp_c`.
3. **Given** the stand-in console rig whose R already holds a file
   `frames/2026-01-16/000007.fits` of the test's making, **When** a test takes two
   frames, **Then** a committed test asserts that they are saved as `000008.fits` and
   `000009.fits` with `frame.id` 8 and 9, and that the first file's bytes are unchanged.
   **When** the test then writes a file `000010.fits` of its own into that folder and
   takes a third frame, **Then** that frame has `frame.id` 10 and ends `failed` with
   `could not save the imaging frame: frames/2026-01-16/000010.fits already exists`,
   the test's file is unchanged, the folder holds no `.part` file, and a fourth frame is
   saved as `000011.fits`.
4. **Given** the console rig with `{"exposure_ms":200,"forward_kib_per_s":64}` applied
   and a `watch` on the link's status (`crates/finder/src/link.rs:195`), **When** a test
   takes a frame and waits until it is saved, **Then** a committed test asserts that the
   frame was `receiving` for between 27 s and 31 s of virtual time (worked example: the
   eighth and last chunk leaves when 7 × 262,144 bytes have gone, 28 s after the first,
   at 65,536 bytes a second), with `frame.received` rising and `frame.bytes` 2,031,984
   throughout; that in the saved view `frame.received_at − frame.start − 0.2` is
   28 s ± 0.1 s, the time from the end of the exposure to the last byte in hand, which is
   what the field check reads, and `frame.saved_at` is from 0 to 1 s after
   `frame.received_at`; that the link's `heartbeats` count rose by at least 104 in that
   time; that the session's `node.connected` was true at every read;
   and that the saved file is 2,036,160 bytes.
5. **Given** the built `finder` run with `--simulate`, `--state S`, `--imaging-dir D` and
   `--listen` on port 0 of `127.0.0.1`, **When** a test waits for `state` `ready`,
   applies `{"exposure_ms":100}`, sends `POST /api/imaging/frame` and waits until
   `frame.state` is `saved`, for at most 30 s, **Then** a committed test asserts that D
   holds exactly one file under `frames/`, at the path `frame.file` names, 2,036,160
   bytes long, and that S holds no `imaging` folder. S and D are child paths of the
   test's scratch folder.
6. **Given** the stand-in console rig with `{"exposure_ms":100}` applied, **When** a test
   calls `take("dark")` on the console's `Imaging`, as `AppState::imaging` hands it out,
   waits until that frame is saved, then calls `take("flat")` and waits again, **Then** a
   committed test asserts that `frames/2026-01-16/000001.fits` has `IMAGETYP` `DARK` and
   `PURPOSE` `dark`, that `000002.fits` has `IMAGETYP` `FLAT` and `PURPOSE` `flat`, and
   that each frame's view has its label as `frame.purpose` (oracle: owner decision 2).

---

### User Story 3 - Read a saved imaging frame back (Priority: P1)

As a later spec's solver, I read a FITS file this console wrote back into its samples and
cards, and a file it did not write is refused with a plain reason.

**Why this priority**: 032 reads every saved imaging frame through `decode`. Nothing in this
spec calls it, so it is proved here, beside the writer it reads (FR-016).

**Independent Test**: `crates/finder/tests/console/imaging_fits.rs`, the module
`imaging_fits` of `console`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test console imaging_fits::`.

**Acceptance Scenarios**:

1. **Given** `finder::imaging::fits`, **Then** a committed test asserts that:
   - `encode` of a 4 × 2, 16-bit image whose samples are 0, 1, 2, 3, 32768, 40000,
     65534 and 65535, with the cards `EXPTIME` real 0.5, `GAIN` real 100.0, `XBINNING`
     integer 1, `BAYERPAT` text `RGGB` and `IMAGETYP` text `LIGHT`, is 5760 bytes; that,
     read as the shared setup says, its cards are in order `SIMPLE` T, `BITPIX` 16,
     `NAXIS` 2, `NAXIS1` 4, `NAXIS2` 2, `BZERO` 32768, `BSCALE` 1, `EXPTIME` 0.5, `GAIN`
     100.0, `XBINNING` 1, `BAYERPAT` `RGGB`, `IMAGETYP` `LIGHT`, `END`; that its `GAIN`
     card begins `GAIN    =                100.0`; that its data begins
     `80 00 80 01 80 02 80 03 00 00 1C 40 7F FE 7F FF` (worked example: each sample
     less 32768 as a big-endian signed number); and that `decode` of those bytes
     equals the image, its `GAIN` being `Real(100.0)`;
   - `encode` of a 3 × 2, 8-bit image with the bytes `00 01 02 FD FE FF` and no cards
     is 5760 bytes with `BITPIX` 8, no `BZERO` and no `BSCALE`, its data beginning with
     those six bytes, and `decode` of it equals the image;
   - `decode` of the worked FITS frame of plan.md § FITS at the console, built by a
     helper of the test's own from the cards and bytes given there, is 4 × 2, 16 bits,
     the samples 0, 1, 2, 3, 32768, 40000, 65534 and 65535, and the cards `XBINNING`
     integer 1, `YBINNING` integer 1, `CCD-TEMP` real -9.5 and `BAYERPAT` text `RGGB`,
     in that order;
   - `decode` answers a `FitsError` that displays exactly `not a FITS image` for 2880
     zero bytes; `a 3-axis FITS file is not one image` for the worked frame with
     `NAXIS` 3; `unsupported FITS BITPIX -32` for it with `BITPIX` -32;
     `unsupported FITS BZERO 0` for it with `BZERO` 0; and
     `the FITS data is short: 8 of 16 bytes` for it cut off 8 bytes after its header.

---

### User Story 4 - An imaging frame that breaks the rules, and an 8-bit mono one (Priority: P1)

As the observer, when the scope node sends an imaging frame that is short, too large, out of
order or late, I am told which, and no file or half-written file is left; and a frame from
an 8-bit mono camera, or of a region the console did not ask for, is saved as the scope
node sent it.

**Why this priority**: A wrong file, or one stamped with the wrong place on the sensor, would
mislead every later spec that reads it; these checks keep it from being written.

**Independent Test**: `crates/finder/tests/console/imaging_checks.rs`, the module
`imaging_checks` of `console`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test console imaging_checks::`.

**Acceptance Scenarios**:

1. **Given** the console rig paired to a node whose `ScopeNodeImaging` is the test's own
   and answers `Expose` in turn with each of these, **Then** a committed test asserts
   the frame ends `failed` with exactly this `error`, and that R holds no `.fits` and no
   `.part` file after any of them. Each frame is waited for for at most 160 s:
   - a header for 4 × 4, 16 bits, `byte_count` 32, then 16 bytes, then the end of the
     stream: `the imaging frame was incomplete: 16 of 32 bytes`;
   - a header for 4 × 4, 16 bits, `byte_count` 30:
     `the imaging frame's header does not add up`;
   - a header for 16384 × 8193, 16 bits, `byte_count` 268,468,224, which adds up but is
     above 256 MiB (268,435,456 bytes): the same text;
   - a chunk before any header: `the scope node sent no imaging frame header`;
   - a header for 4 × 4, 16 bits, `byte_count` 32, then 40 bytes:
     `the scope node sent more bytes than the header said`;
   - the status `Internal` with `the camera fell over`:
     `the exposure failed: the camera fell over`;
   - a header and 16 bytes, then nothing: `the imaging frame stalled`, between 30.0 and
     30.2 s after the 16 bytes were sent;
   - no message at all, the stream staying open, at the console's starting exposure of
     1000 ms: `no imaging frame header arrived within 152 s`, between 152.0 and 152.2 s
     after the request (worked example: twice the exposure and 150 s).
2. **Given** the console rig paired to a node whose `ScopeNodeImaging` is the test's own,
   as in US4-S1, which describes a camera with the model `test mono camera`, no serial,
   a 16 × 8 sensor, 8 bits, no Bayer pattern, exposure 1 to 60000 ms, no gain or offset range,
   binnings `[1]` and no cooler, answers `ConfigureImaging` with binning 1 and the
   region (0, 0, 3, 2), and answers `Expose` with a header for 3 × 2, 8 bits, an empty
   `bayer`, no `gain`, `offset` or `temp_c`, binning 1, exposure 100.0, the purpose
   `frame`, `byte_count` 6 and `roi` x 8, y 4, width 3, height 2 (a region the
   console's settings did not hold, as when another INDI client has moved it), then the
   bytes `00 01 02 FD FE FF`, **When** a test takes a frame, **Then** a committed test
   asserts that the answer to `POST /api/imaging/frame` has `frame.roi`
   `{"x":0,"y":0,"width":3,"height":2}`, the settings' region; that the frame is
   `saved` with `frame.roi` `{"x":8,"y":4,"width":3,"height":2}`, the header's; that its
   view has `bits` 8 and no `bayer`, `gain`, `offset` or `temp_c` key; that the file is
   5760 bytes; that its cards are in order `SIMPLE` T, `BITPIX` 8, `NAXIS` 2, `NAXIS1` 3,
   `NAXIS2` 2, `DATE-OBS`, `EXPTIME` 0.1, `XBINNING` 1, `YBINNING` 1, `XORGSUBF` 8,
   `YORGSUBF` 4, `INSTRUME` `test mono camera`, `IMAGETYP` `LIGHT`, `ROWORDER`
   `TOP-DOWN`, `FRAMEID` 1, `PURPOSE` `frame`, `END`, with no `BZERO`, `BSCALE`,
   `GAIN`, `OFFSET`, `CCD-TEMP`, `SET-TEMP`, `BAYERPAT` or `SERIALNO`; and that its
   data begins with those six bytes unchanged (oracle: the bytes the test's node
   sent).

---

### User Story 5 - Refuse a second imaging frame, abort one, and ask for the scope node's copy (Priority: P1)

As the observer, a second frame or a settings change during an exposure is refused with a
plain reason, I can abort an exposure at any time, and I can ask the scope node to keep its
own copy and see what became of it.

**Why this priority**: An exposure of minutes must be stoppable, and the scope node's copy is
the one kept when the console cannot save a frame.

**Independent Test**: `crates/finder/tests/console/imaging_control.rs`, the module
`imaging_control` of `console`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test console imaging_control::`.

**Acceptance Scenarios**:

1. **Given** the stand-in console rig, **Then** a committed test asserts that
   `POST /api/imaging/frame` answers 403 with a foreign `Origin`; that with an exposure
   of 10 s applied, a second request 1 s after the first answers 409 and
   `an exposure is already in progress`, and `PUT /api/imaging/settings` with
   `{"gain":150}` then answers 409 and `an exposure is in progress`; and that on each
   console of US1-S4 the request answers 409 with that scenario's text for its state.
2. **Given** the stand-in console rig with an exposure of 30 s applied, **When** a test
   takes a frame, 1 s later sends `POST /api/imaging/abort`, and then applies 100 ms and
   takes another, **Then** a committed test asserts that 1 s into the exposure, before
   the abort, `status.exposing` is true; that the abort answers 200; that
   within 1 s `frame.state` is `aborted` with `frame.id` 1 and `status.exposing` false;
   that R then holds no file under `frames/`; that the next frame is saved as
   `000002.fits`; and that `POST /api/imaging/abort` with nothing exposing answers 200
   and changes nothing.
3. **Given** the stand-in console rig as it is, whose node has no save folder, with
   `{"exposure_ms":100,"save_at_node":true}` applied, **When** a test takes a frame and
   waits until it is saved, **Then** a committed test asserts that the frame is `saved`
   with `frame.node_save_error` `this scope node has no save_dir` and no `node_file`
   key, and that R holds `frames/2026-01-16/000001.fits`, 11,520 bytes. **When** it
   applies `{"save_at_node":false}` and takes another, **Then** that frame's view has
   neither key (oracle: 030's US3-S6, the node's answer to `save` with no folder).
4. **Given** the stand-in console rig whose node was given `.with_imaging_save_dir(D)`
   (030's US7), D a child path of the test's scratch folder that does not exist yet,
   **When** a test applies `{"exposure_ms":100,"save_at_node":true}` and takes a frame,
   **Then** a committed test asserts that the saved view's `frame.node_file` is
   `frame-000001.fits` with no `node_save_error` key; that D holds that file; and that
   its samples, read as the shared setup says, equal the samples of the console's
   `frames/2026-01-16/000001.fits` (oracle: the two files, each read with the test's
   own code; a node with no save folder is US5-S3).

---

### User Story 6 - A colour preview of each imaging frame (Priority: P2)

As the observer or an MCP client, I get a small colour JPEG of each saved imaging frame,
stretched so that faint stars show, without opening the FITS file.

**Why this priority**: The page and the MCP tools show this picture; the FITS file is
the record.

**Independent Test**: `crates/finder/tests/console/imaging_preview.rs`, the module
`imaging_preview` of `console`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test console imaging_preview::`.

**Acceptance Scenarios**:

1. **Given** `finder::imaging::preview::rgb` and the 6 × 6, 16-bit RGGB frame whose rows
   are `100 100 400 400 700 100`, `100 50 100 50 100 50`, `900 100 100 100 100 100`,
   `100 200 100 350 100 50`, `100 100 100 100 100 100` and `100 50 100 50 100 50`,
   **Then** a committed test asserts a 3 × 3 picture whose first five pixels, left to
   right and top to bottom, are (0, 0, 0), (197, 140, 0), (255, 0, 0), (255, 0, 140) and
   (0, 0, 197), and whose other four are (0, 0, 0) (worked example, plan.md § The
   preview: red's median is 100 and its 99.9th percentile 700, blue's are 50 and 200,
   green's median and 99.9th percentile are both 100, so the white span is 600; a half
   maps to 255·asinh(5)/asinh(10) = 197 and a quarter to 255·asinh(2.5)/asinh(10) = 140,
   which is the second pixel's green, the mean of 400 and 100); and that the same samples
   read as `BGGR` give every pixel with its red and blue swapped.
2. **Given** `rgb` and frames of the test's making, **Then** a committed test asserts
   that a mono 8-bit frame of 5200 × 100 gives a picture of 1040 × 20 (worked example:
   the factor is ⌈5200/1280⌉ = 5); that a 16-bit RGGB frame of 2600 × 1400 gives one of
   650 × 350 (1300 × 700 superpixels, halved); that a uniform frame gives a picture of all zeros; and that a mono frame
   gives equal red, green and blue in every pixel; that a mono 8-bit frame of 2560 × 2
   whose 2 × 2 blocks each hold 0 at their top-left and the block's column index mod 200
   in their other three samples gives a 1280 × 1 picture that is not all zeros (k = 2:
   only the mean of a block, not its first sample, varies); and that a 16-bit RGGB frame
   of 6 × 2 whose rows are `0 100 0 100 0 101` and `100 0 100 0 100 0` gives the pixels
   (0, 0, 0), (0, 0, 0) and (0, 197, 0) (the greens are 100, 100 and 100.5, so the span
   is its floor, 1).
3. **Given** the stand-in console rig with `{"exposure_ms":100}` applied, **When** a test
   takes a frame and waits until it is saved, **Then** a committed test asserts that
   `frame.preview` is `/api/imaging/frames/1/preview.jpg`; that `GET` of it answers 200
   with `Content-Type` `image/jpeg` and a body that `image` decodes to 32 × 24; that
   `GET /api/imaging/frames/999/preview.jpg` answers 404 and
   `the console no longer holds that preview`; that
   `GET /api/imaging/frames/abc/preview.jpg` answers 400; and that after eight more
   frames the first frame's preview answers 404 while the ninth's answers 200. **When**
   it then applies `{"roi":{"x":0,"y":0,"width":64,"height":1}}` and takes a frame,
   **Then** that frame is `saved` with no `preview` key (a 64 × 1 mosaic has no whole
   2 × 2 cell).
4. **Given** the sim rig's camera with noise on and one star of magnitude 5 and colour
   `[1, 0, 0]` at the truth's centre for mid-exposure, **When** a test takes a 20 ms
   frame through the port and passes it to `rgb`, **Then** a committed test asserts a
   618 × 411 picture in which at least one pixel has red 255, every pixel with red 255 is
   within 2 pixels of (309, 205) in x and in y, and none of them has green or blue above
   100; and with the colour `[0, 0, 1]`, the same with blue 255 and red at most 100
   (oracle: the simulator's star; worked example: a sample four times the sky's noise
   above the median maps to 93).

---

### User Story 7 - The Imaging page, red at night with colour while held (Priority: P2)

As the observer with only the page to hand, I open Imaging, see the camera and its
temperature, set the exposure, gain, offset, binning and cooler, take a frame and see
it. At night the picture is red like everything else, and I can hold a control to see
its colours for a moment.

**Why this priority**: The page is how frames are taken at the telescope; the night
exception is the one place the page may show colour in the dark, so it is specified and
checked exactly.

**Independent Test**: `npm test` in `crates/finder/web` runs
`src/components/ImagingPage.test.tsx`; `CI=1 npm run e2e` runs `e2e/imaging.spec.ts` in
the simulate project (owner approval 8). Both are commands of the `test` gate.

**Acceptance Scenarios**:

1. **Given** `App` rendered at the path `/imaging` with `fetch` answering
   `/api/session` with a session holding a connected `node`, and `/api/imaging` with the
   body of US1-S1, and `EventSource` stubbed as
   `crates/finder/web/src/components/MountSection.test.tsx:13` stubs it, **Then** a
   committed test asserts a heading `Imaging`, a link `Back to console`, and in the top
   bar, the first element of role `banner`, a link `Imaging` with `aria-current="page"`,
   placed after the link `Setup`.
   **When** the test clicks `Back to console`, **Then** the path is `/` and the heading
   is gone. **Given** `App` at `/` with a session that has no `node`, **Then** the
   top bar has no link named `Imaging`; with a `node`, clicking it sets the path to
   `/imaging`.
2. **Given** `ImagingPage` rendered with each view, **Then** a committed test asserts
   exactly these words and no region named `Settings` or `Imaging frame`:
   - `{"state":"no_link"}` and `{"state":"off"}`: a status
     `The scope node is not connected.`;
   - `{"state":"no_service"}`: a status
     `This scope node has no imaging service. Rebuild it to match this console.`;
   - `{"state":"no_camera"}`: a status `This scope node has no imaging camera.`;
   - `{"state":"error","error":"cannot reach the INDI server"}`: an alert
     `The imaging camera is not answering: cannot reach the INDI server.`
3. **Given** `ImagingPage` with the view of US1-S1, **Then** a committed test asserts a
   region `Camera` reading
   `simulated imaging camera · 1236 × 822 px · 19.12 µm · 16-bit · RGGB` and
   `Sensor 20.0 °C · cooler off`; and a region `Settings` holding fields named
   `Exposure`, `Gain`, `Offset`, `Binning` with the options `1 × 1` and `2 × 2`, a
   switch `Cooler`, a field `Set temperature`, a field `Forwarding limit`, a checkbox
   `Also keep each imaging frame at the scope node` and a button
   `Save imaging settings`.
   **When** the test sets `Exposure` to 2000 and `Binning` to `2 × 2` and clicks the
   button, with `fetch` answering 200 and a view, **Then** `fetch` was called with
   `/api/imaging/settings` and
   `{"method":"PUT","headers":{"Content-Type":"application/json"},"body":"{\"exposure_ms\":2000,\"gain\":120,\"offset\":10,\"binning\":2,\"cooler_on\":false,\"forward_kib_per_s\":null,\"save_at_node\":false}"}`.
   **When** it sets `Forwarding limit` to 4096 and saves again, the body's
   `forward_kib_per_s` is 4096; and with `settings.forward_kib_per_s` 4096 in the view
   the field shows 4096.
   A 400 answer with `{"error":"gain 301 is outside the camera's range 0–300"}` shows an
   alert with exactly that text. With `status`
   `{"temp_c":-10.0,"cooler_power":75.0,"set_temp_c":-10.0,"exposing":false}` and
   `cooler_on` true the line reads `Sensor -10.0 °C · cooler on at 75% · set -10.0 °C`;
   with `status` `{"temp_c":5.0,"cooler_power":40.0,"set_temp_c":null,"exposing":false}`
   and `cooler_on` true, as after the scope node restarts with the cooler running, it
   reads `Sensor 5.0 °C · cooler on at 40%`.
   With `camera.gain` null, `camera.offset` null, `camera.cooler` false and
   `camera.bayer` null, the fields `Gain`, `Offset`, `Cooler` and `Set temperature` are
   absent and the camera line ends `mono`.
4. **Given** `ImagingPage` with the view of US1-S1 and each `frame`, **Then** a committed
   test asserts in the region `Imaging frame`:
   - no `frame`: a button `Take imaging frame`, which calls `fetch` with
     `/api/imaging/frame` and `{"method":"POST"}`; a 409 answer shows its `error` as an
     alert;
   - `state` `exposing`, `exposure_ms` 2000: a status `Exposing for 2000 ms`,
     `Take imaging frame` disabled, and a button `Abort exposure`, which calls `fetch`
     with `/api/imaging/abort` and `{"method":"POST"}`;
   - `receiving`, `received` 262144, `bytes` 2031984: a status
     `Receiving 262144 of 2031984 bytes`, `Take imaging frame` disabled, and the button
     `Abort exposure`;
   - `saved`, `file` `frames/2026-01-16/000001.fits`, `exposure_ms` 2000, `start`
     1768528800.0 and `received_at` 1768528830.3: a status
     `Saved frames/2026-01-16/000001.fits · in hand 28.3 s after the exposure` (worked
     example: 30.3 s less the 2 s exposure); with `node_file` `frame-000001.fits` the
     status also holds ` · also at the scope node as frame-000001.fits`; with
     `node_save_error` `this scope node has no save_dir`, an alert
     `The scope node kept no copy: this scope node has no save_dir.`;
   - `failed`, `error` `the imaging frame stalled`: an alert
     `Imaging frame failed: the imaging frame stalled.`;
   - `aborted`: a status `Exposure aborted.`
5. **Given** `ImagingPage` with a saved frame whose `preview` is
   `/api/imaging/frames/1/preview.jpg`, **Then** a committed test asserts a picture
   named `Imaging frame 1` with that source and the class `photo`. With `night` true it
   asserts a button `Hold to see colour` with `aria-pressed="false"` and the text
   `The picture is red at night. Hold the control to see its colours for up to 10
   seconds.`; that a `pointerdown` on the button removes the class `photo` and sets
   `aria-pressed="true"`; that a `pointerup`, a `pointerleave`, a `pointercancel` and a
   `blur` each restore both; that holding the space key does the same as the pointer
   and releasing it restores both; that, with fake timers, 10 s after a
   `pointerdown` with no release both are restored; and that a `keydown` of the space
   key with `repeat` true, 5 s into a hold begun with that key, does not start the
   limit again, both being restored 10 s after the first `keydown`. With `night` false
   there is no such button. **And** a committed test renders the hook the page uses,
   `useColourHold` of `crates/finder/web/src/useColourHold.ts`, with `renderHook` from
   `@testing-library/react`, and asserts that with `night` true `held` starts false,
   `bind.onPointerDown` makes it true and `bind.onPointerUp` false again; that with
   `night` false `held` stays false after `bind.onPointerDown`; and that a hold begun
   with `night` true ends, `held` false, when the hook is rendered again with `night`
   false.
6. **Given** `ImagingPage` fed by its hook with fake timers and `fetch` answering
   `/api/imaging`, **Then** a committed test asserts that `/api/imaging` is read once
   when the page opens, again after each 1000 ms, at once after a successful save or
   `Take imaging frame`, and never after the page is unmounted; and that a read
   answered 500 changes nothing the page shows.
7. **Given** the simulate project's console, **When** a Playwright test opens
   `/imaging`, sets `Exposure` to 200, saves and clicks `Take imaging frame`, **Then** a
   committed test asserts the camera line contains `simulated imaging camera`; that
   within 15 s a picture named `Imaging frame` is visible whose natural size is
   618 × 411; and that the banner's `Imaging` link has `aria-current="page"`.
8. **Given** that page with its picture on a 412 × 915 viewport at a device scale of 2,
   **When** a Playwright test counts off-red pixels as
   `crates/finder/web/e2e/session.spec.ts:45` counts them (red above 226, or any green
   or blue), **Then** a committed test asserts more than 100,000 by day; exactly 0 in a
   full-page screenshot once Night is on; more than 100 in a screenshot of the picture
   while the mouse is held down on `Hold to see colour`, and exactly 0 in a full-page
   screenshot taken during that hold with the picture masked in black; and exactly
   0 in a full-page screenshot after the mouse is released (oracle: `DESIGN.md`'s Night
   Ink, `#e00000`, and the exception of owner approval 6).

---

### User Story 8 - Three MCP tools (Priority: P2)

As an MCP client, I can read the imaging camera's state, change its settings and take a
frame, and I get the picture back in colour, through the same console actions and
refusals as the page.

**Why this priority**: The owner's goal is an ask from an MCP client; these are its
first three steps.

**Independent Test**: `crates/finder/tests/mcp/imaging.rs`, the module `imaging` of
finder's `mcp` test binary. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test mcp imaging::`.

**Acceptance Scenarios**:

1. **Given** a console with MCP on, **When** a test calls `tools/list`, **Then** a
   committed test asserts that `get_imaging_camera`, `set_imaging_camera` and
   `take_imaging_frame` are listed, each with `outputSchema` `{"type":"object"}`; that
   only the first has `annotations.readOnlyHint` true; that the first's description is
   `Read the imaging camera's state, description, status, settings and latest imaging frame.`;
   that the second's contains `as PUT /api/imaging/settings does` and its
   `inputSchema.properties` has exactly the nine keys `exposure_ms`, `gain`, `offset`,
   `binning`, `roi`, `cooler_on`, `set_temp_c`, `forward_kib_per_s` and `save_at_node`,
   with no `required` list or an empty one, that `inputSchema.properties.gain.type` is
   `number`, and that `forward_kib_per_s`'s type allows `null`; and that the third's
   contains `colour JPEG` and `waits`.
2. **Given** the stand-in console rig with MCP on, **Then** a committed test asserts that
   `get_imaging_camera` with `{}` answers a `structuredContent` equal to the body of
   `GET /api/imaging` read just after, and `get_session` over MCP carries `imaging` equal
   to that body; that `set_imaging_camera` with
   `{"gain":200,"exposure_ms":100}` answers a `structuredContent` whose `settings.gain`
   is 200.0 and `settings.exposure_ms` 100.0, which `GET /api/imaging` then shows too;
   that with `{"gain":301}` it answers `isError` true and the text
   `gain 301 is outside the camera's range 0–300`; and that with `{"gian":1}` it answers
   `isError` true with exactly the `error` that `PUT /api/imaging/settings` answers for
   the same body.
3. **Given** the same console with 100 ms applied, **When** a test calls
   `take_imaging_frame` with `{}`, **Then** a committed test asserts that the answer
   comes only after the frame is saved; that `structuredContent.frame.state` is `saved`
   with `file` `frames/2026-01-16/000001.fits`, which exists under R; that `content`
   holds one text block and one image block with `mimeType` `image/jpeg`; and that the
   image block's `data`, decoded from standard base64, equals byte for byte the body of
   `GET /api/imaging/frames/1/preview.jpg` and decodes to 32 × 24.
4. **Given** the same console, **Then** a committed test asserts that
   `take_imaging_frame` with `{"exposure_ms":5}` answers `isError` true and
   `take_imaging_frame takes no arguments`; that with an exposure of 10 s applied and a
   frame started through the page 1 s before, it answers `isError` true and
   `an exposure is already in progress`; that, once that frame is saved,
   `take_imaging_frame` called again answers `isError` true and `the exposure was aborted`
   when its own 10 s frame is aborted 1 s later through `POST /api/imaging/abort`; and
   that, on a console with MCP on paired to a node
   whose imaging camera, one of the test's own, describes itself as the stand-in camera
   does and answers `expose` with `Failed("the camera fell over")`, it answers
   `isError` true and `the exposure failed: the camera fell over`, the failed frame's
   own `error`.
5. **Given** a console with MCP on and no `with_imaging`, as
   `crates/finder/tests/support/mcp.rs:24` builds its state, **Then** a committed test
   asserts that `get_imaging_camera` with `{}` answers `structuredContent`
   `{"state":"off"}`, and that `set_imaging_camera` and `take_imaging_frame` with `{}`
   each answer at once, tokio's clock reading the same instant before and after under
   paused time, with `isError` true and `this console takes no imaging frames`.
6. **Given** the stand-in console rig with MCP on, **Then** a committed test asserts that
   no result of the three tools, called with `{}`, contains `test-token`.

---

### User Story 9 - One holder of the imaging camera (Priority: P2)

As a later spec's focus aid, set of darks or flats, stack or imaging run, I take the
imaging camera for myself while I take frames one after another, and nothing else
changes its settings or starts an exposure until I let it go; as the observer, I can
still see the camera and stop an exposure at any time.

**Why this priority**: Nothing in this spec takes frames one after another, so the
first nights can do without it; but specs 033 to 037 each call it and may add no lock of
their own (the plan document's Amendment 1), so it lands with this spec, after every
story that writes the console's imaging state.

**Independent Test**: `crates/finder/tests/console/imaging_lease.rs`, the module
`imaging_lease` of finder's `console` test binary. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test console imaging_lease::`.

**Acceptance Scenarios**:

1. **Given** the stand-in console rig, **When** a test waits until `state` is `ready`,
   calls `imaging.lease("the test")`, then `imaging.lease("a second taker")`, then drops
   the first lease and calls `imaging.lease("a second taker")` again, **Then** a
   committed test asserts that the first answers `Ok` and `imaging.holder()` is then
   `Some("the test")`; that the second answers `Err` with a `Refusal` of status 409 and
   the message `the imaging camera is held by the test`, tokio's clock reading the same
   instant before and after it, and `holder()` still `Some("the test")`; and that the
   third answers `Ok`, `holder()` then being `Some("a second taker")`. **Given** the
   console of US1-S4 whose transport's connector always answers an I/O error, its
   `state` `no_link`, **Then** `imaging.lease("the test")` answers `Ok`: a lease looks at
   nothing but the lease (oracle: the rule of FR-028, one holder by name).
2. **Given** the stand-in console rig, ready, with `{"exposure_ms":10000}` applied and
   the lease taken, **When** a test holding no lease makes each of the requests below,
   first with no frame in flight and again 1 s after the holder has called
   `lease.take("frame")`, **Then** a committed test asserts that every one answers 409
   with exactly `the imaging camera is held by the test`, never the frame-in-flight
   words `an exposure is already in progress` or `an exposure is in progress`:
   - `imaging.set` of `{"gain":150}` and `imaging.take("frame")`, each an `Err` with a
     `Refusal` of status 409;
   - `PUT /api/imaging/settings` with `{"gain":150}` and `POST /api/imaging/frame`, each
     with that text as its JSON `error`;
   - over the MCP door, `set_imaging_camera` with `{"gain":150}` and
     `take_imaging_frame` with `{}`, each a tool error with that text;
   and that afterwards `settings.gain` is still 100.0, the view's `frame.id` is 1 with
   `frame.state` `exposing`, and `status.exposing` is true (oracle: the holder's own
   frame, which no refused request could have started). **Given** instead the console of
   US1-S4 whose connector always fails, `state` `no_link`, and the lease taken, **Then**
   `imaging.set` of `{"gain":150}`, `imaging.take("frame")` and the holder's `lease.set`
   of `{"gain":150}` each answer 409 `the scope node is not connected`: the state's
   check comes before the lease's.
3. **Given** the stand-in console rig, ready, and the lease taken, **When** the holder
   calls `lease.set` of `{"exposure_ms":100,"gain":200}`, `lease.set` of `{"gain":301}`,
   `lease.take("frame")` and then `imaging.ended(1)`, **Then** a committed test asserts
   that the first answers `Ok` with `settings.exposure_ms` 100.0 and `settings.gain`
   200.0, which `GET /api/imaging` then shows; that the second answers a `Refusal` of
   status 400 with `gain 301 is outside the camera's range 0–300`; that `lease.take`
   answers `Ok` with `frame.id` 1, `frame.state` `exposing` and `frame.purpose` `frame`;
   and that `ended(1)` answers `state` `saved` with `file` `frames/2026-01-16/000001.fits`,
   which R holds, 11,520 bytes long. **When** the holder then calls `lease.set` of
   `{"exposure_ms":10000}`, `lease.take("frame")` and, 1 s later, `lease.take("frame")`
   and `lease.set` of `{"gain":150}`, **Then** those last two answer 409
   `an exposure is already in progress` and 409 `an exposure is in progress`: the holder
   meets every check of `Imaging::take` and `Imaging::set` but the lease's (oracle:
   US1-S3, US2-S1 and US5-S1, the same answers without a lease).
4. **Given** the stand-in console rig, ready, with `{"exposure_ms":30000}` applied, the
   lease taken and the holder's frame started with `lease.take("frame")`, **When** 1 s
   later a test holding no lease reads `GET /api/imaging`, calls `get_imaging_camera`
   with `{}` over the MCP door and `imaging.view()`, sends `POST /api/imaging/abort` and
   calls `imaging.ended(1)`, **Then** a committed test asserts that the read, the tool and
   `view()` each answer the view with `frame.id` 1 and `frame.state` `exposing`; that the
   abort answers 200; and that `ended(1)` answers `state` `aborted` within 1 s of the
   abort. **When** the holder then takes a frame of 100 ms with `lease.set` and
   `lease.take` and it is saved, **Then** `GET /api/imaging/frames/2/preview.jpg`, sent
   by a caller holding no lease while the lease is still out, answers 200 with
   `Content-Type` `image/jpeg` (oracle: US5-S2 and US6-S3, the same answers without a
   lease).
5. **Given** the stand-in console rig, ready, with `{"exposure_ms":10000}` applied,
   **When** a test notes `Arc::strong_count(&imaging)` as N, spawns a task that, on a
   clone of `imaging`, takes `lease("the test")`, calls `lease.take("frame")` and awaits
   `ended` of its frame, waits until `holder()` is `Some("the test")`, and 1 s after the
   task's `take` aborts the task and awaits its handle, **Then** a committed test asserts
   that `strong_count` read N + 1 while the task held the lease, the task's own clone
   and nothing for the lease; that the handle answers a `JoinError` whose
   `is_cancelled()` is true; that `holder()` is `None` in that same virtual instant; that
   `imaging.lease("a second taker")` then answers `Ok`, and once that lease is dropped
   `holder()` is `None` again with no time passing; that `POST /api/imaging/frame` answers
   409 `an exposure is already in progress`, the aborted task's frame running on with no
   holder; and that `ended(1)` answers `saved`. **Given** instead an `Imaging` the test
   starts itself with `Imaging::start(R, link, clock)`, `link` being `AppState::link` of
   the rig and `clock` a `ConsoleClock` over the rig's simulator clock, as the rig passes
   to `with_clock`, a `Weak` of it, a lease of it taken and
   then the test's one `Arc` of it dropped, **Then** the `Weak` no longer upgrades; the
   lease's `take("frame")` and its `set` of `{}` each answer 409
   `this console takes no imaging frames`; and dropping the lease afterwards neither
   panics nor changes `holder()` of the rig's own `imaging`, which stays `None`.
6. **Given** the stand-in console rig, ready, with `{"exposure_ms":10000}` applied and no
   lease out, **When** a test sends `POST /api/imaging/frame` and 1 s later calls
   `imaging.holder()` and `imaging.lease("the test")`, **Then** a committed test asserts
   that `holder()` is `None` although the frame is `exposing`; that the lease is granted;
   that with it held the holder's `lease.take("frame")` answers 409
   `an exposure is already in progress` and its `lease.set` of `{"gain":150}` 409
   `an exposure is in progress`, so each holder looks for a frame in flight itself; that
   `POST /api/imaging/frame` from a caller holding no lease now answers 409
   `the imaging camera is held by the test`; and that `ended(1)` answers `saved` with
   `file` `frames/2026-01-16/000001.fits` while the lease is still out. **Given** the
   same rig with the lease dropped and the frame saved, **When** a task calls
   `take_imaging_frame` with `{}` over the MCP door, **Then** `holder()` read 1 s later,
   while that call waits for its frame of 10 s, is `None`, and the call answers with
   `frame.state` `saved` (oracle: the rule of FR-033, a single imaging frame holds no
   lease).

---

### Edge Cases

- **A scope node built before 030.** It lacks the service: the console's state is
  `no_service` and the page says to rebuild it. A console built before this spec never
  calls the service.
- **A night without an imaging camera.** A node with no `[imaging]` table answers
  `FailedPrecondition`; the console's state is `no_camera`, it asks again every 5 s, and
  nothing else about the console changes.
- **The INDI server is not running, or the camera is unplugged.** The node has a camera
  configured but cannot describe it; the console's state is `error` with the camera
  adapter's words, and it asks again every 5 s.
- **The link drops during an exposure.** The frame ends `failed` at the stream's error or
  the header wait. Nothing resumes: the observer takes another frame.
- **The link drops while bytes are in flight.** The frame ends `failed` as incomplete,
  the `.part` file is never made (a file is written only from a whole frame), and a copy
  kept at the node, when asked for, is already on its disk.
- **The scope node restarts.** Its frame ids start again at 1. The console names files by
  its own count, so nothing is replaced.
- **The console restarts.** Its count resumes after the largest id in that UTC date's
  folder. Previews are gone; the files stay.
- **Midnight UTC.** A frame goes into the folder of its exposure's start date; the count
  carries on rising within a run.
- **A camera with no gain, offset or cooler, or a mono camera.** The description says so
  (`null`, `false`), the settings show `null` for them, the FITS file omits the cards, and the page
  omits the fields.
- **An exposure longer than the heartbeat's silence limit.** The heartbeat is on the
  other connection and is unaffected: a 60 s exposure is one call that answers after
  60 s.
- **A frame larger than 256 MiB.** The console refuses its header as not adding up.
- **A full disk at the console.** The frame ends `failed` with
  `could not save the imaging frame: ` and the system's words; the frame is lost unless
  the node kept a copy.
- **A file already under the next name.** Another program, or a copy made by hand, has
  put a file where the next frame would go. The frame ends `failed` and names the file;
  that file is untouched, and the next frame takes the next id (US2-S3).
- **A frame taken with a region of interest.** The scope node sends the region in force
  with the frame, and the console's file says where it sat on the sensor (`XORGSUBF`,
  `YORGSUBF`; owner decision 16).
- **A camera that delivers less than the region over the binning.** The frame's own
  width and height, from the header, are used everywhere; nothing works a frame's size
  out from the settings.
- **A `neutral` key the camera does not have** in the scope node's config (030). The
  console's state is `error` with the camera adapter's words,
  `the camera has no control {name}`, until the node's table is corrected.
- **The scope node restarts with the cooler running.** The status then has no set
  temperature; the page says `cooler on at <power>%` and no set temperature until one
  is saved again.
- **Two pages, or a page and an MCP client, take a frame at once.** One exposure runs;
  the other request is refused with `an exposure is already in progress`.
- **A later spec's focus aid, set, stack or run holds the camera.** A frame or a
  settings change from the page or an MCP client is refused with
  `the imaging camera is held by …` and the holder's name; the view, the previews and
  Abort stay open, so an exposure can always be stopped (US9).
- **The holder's task is cancelled.** The lease goes with it, at once (US9-S5); a frame
  it had started runs on, holding nothing, and is saved.
- **Settings changed on the camera by another INDI client.** The console shows them at
  its next settings read, which follows every settings request and every reconnection,
  not sooner. A frame's file is right meanwhile, because its region comes from the scope
  node with the frame (owner decision 16); only the set temperature it is stamped with
  is the console's.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: `crates/finder` SHALL gain the module tree `imaging`, and the link manager
  SHALL open, on request, a second connection to the node it is paired with, for
  `ScopeNodeImaging` only, from the transport and pairing it was started with
  (`LinkManager::connect_imaging`). No heartbeat, star frame, mount call or raw frame
  SHALL use that connection, and `LinkManager::start`'s signature SHALL not change
  (owner approvals 1 and 10).
- **FR-002**: `GET /api/imaging` SHALL answer 200 with the imaging view: `state`, one of
  `off`, `no_link`, `no_service`, `no_camera`, `error` and `ready`; with `error`, the
  node's words as `error`; and with `ready`, `camera`, `status` and `settings` in the
  shapes of US1-S1. A console built without `AppState::with_imaging` SHALL answer
  `{"state":"off"}`.
- **FR-003**: `PUT /api/imaging/settings` SHALL accept, under the same-origin rule, a
  JSON object with any of `exposure_ms`, `gain`, `offset`, `binning`, `roi` (a region,
  or null for the whole sensor), `cooler_on`, `set_temp_c`, `forward_kib_per_s` (a whole
  number from 64 to 1048576, or null for no limit) and `save_at_node`, and answer 200
  with the view. The console SHALL keep `exposure_ms` (1000 until set, and within the
  camera's range), `forward_kib_per_s` and `save_at_node`; it SHALL send the rest to the
  node and show the settings the node answers.
- **FR-004**: The route SHALL answer 400 with a JSON `error` and no change for an unknown
  key, a wrong type, a null where none is allowed, a body that is not JSON, or a value
  the console or the camera refuses, with the texts of US1-S3; 403 for a foreign origin
  and 415 for a `Content-Type` other than `application/json`; 409 while the state is not
  `ready`, with the texts of US1-S4; 409 with `an exposure is in progress` during one;
  and 502 with `the scope node did not apply the settings: <message>` when the node's
  call fails otherwise, the message being the node's words or, after 30 s without an
  answer, `no answer within 30 s` (US1-S9).
- **FR-005**: `GET /api/session`, and the MCP tools that return the session, SHALL carry
  the same view as `imaging` whenever the console was built with `with_imaging`, and
  omit it otherwise.
- **FR-006**: While the scope link is connected the console SHALL describe the camera on
  the second connection, read its settings, and read its status every 2 s; in the states
  `no_service`, `no_camera` and `error` it SHALL ask again every 5 s, and no sooner
  because a heartbeat was answered. A call of its own that is not answered within 5 s
  SHALL count as failed with `the scope node did not answer within 5 s` (US1-S10). When
  the link drops, the state SHALL be `no_link` within 3 s and the connection dropped;
  when it returns, the console SHALL connect again and read the camera's settings,
  which the camera keeps (US1-S6).
- **FR-007**: The `finder` binary SHALL take `--imaging-dir DIR` and otherwise use
  `imaging/` in its state folder, and SHALL start imaging over its scope link or its
  simulated node. `crates/finder/README.md` SHALL gain `## Imaging frames` and the two
  routes' rows (owner approvals 2 and 9).
- **FR-008**: `POST /api/imaging/frame` SHALL, under the same-origin rule and while the
  state is `ready` and no exposure runs, ask the node for one exposure of the settings'
  `exposure_ms` with the purpose `frame`, the settings' forwarding rate and
  `save_at_node` (US5-S3, US5-S4), and answer 200 with the view at once. It SHALL answer 409
  with the state's text of US1-S4 otherwise, or with
  `an exposure is already in progress`.
- **FR-009**: The view's `frame` SHALL report the latest frame: `id`, `state` (one of
  `exposing`, `receiving`, `saved`, `failed`, `aborted`), `purpose`, `exposure_ms`,
  `requested_at`; `roi`, at first the region the console's settings held when the frame
  was asked for, replaced by the header's `roi` when the header carries one (owner
  decision 16, US4-S2); `set_temp_c`, the set temperature the settings held with the
  cooler on when the frame was asked for, absent otherwise, the value `SET-TEMP` is
  written from (US2-S2); from the header on, `width`, `height`, `bits`, `bayer`,
  `gain`, `offset`, `binning`, `temp_c`, `start` (console Unix seconds), `date_obs`,
  `bytes` and `received`, with `node_file` or `node_save_error` when the node sent one
  (US5-S3, US5-S4); once the last byte is in hand, `received_at`, and when saved, `saved_at`
  (both console Unix seconds) and `file`; when failed, `error`. `status.exposing` SHALL
  be the console's: true while its latest frame is `exposing`, false otherwise.
- **FR-010**: The console SHALL accept a frame only whose header comes first, whose byte
  count is its width times its height times its bytes a sample and at most 256 MiB, and
  whose chunks add up to exactly that; it SHALL wait for the header twice the exposure
  plus 150 s and for each chunk 30 s; and it SHALL end any other frame as `failed` with
  the texts of US4-S1, the header wait's being
  `no imaging frame header arrived within <n> s` with n that wait in whole seconds,
  rounded up.
- **FR-011**: A whole frame SHALL be saved as
  `frames/<UTC date of the exposure's start>/<id, six digits>.fits` under the imaging
  folder, written under a `.part` name and renamed. The id is the console's own count,
  which starts after the largest id in the folder of the UTC date the frame is asked
  for, rises with every frame asked
  for, saved or not, and never falls within a run. No file SHALL be replaced or
  rewritten: when the name is already taken the frame SHALL end `failed` with
  `could not save the imaging frame: <file> already exists` and the file there SHALL be
  left as it is (US2-S3; owner decisions 3 and 4).
- **FR-012**: The file SHALL be one FITS image with the samples untouched: 16-bit samples
  stored with `BZERO` 32768, 8-bit samples as they are with no `BZERO` or `BSCALE`
  (US4-S2), and the cards of US2-S1 in that order, with `SET-TEMP` after `CCD-TEMP` and
  `SERIALNO` after `INSTRUME` when they are written. `GAIN`, `OFFSET`, `CCD-TEMP` and
  `BAYERPAT` SHALL be left out when the frame has none. `XORGSUBF` and `YORGSUBF` SHALL
  be the origin of the frame's `roi` as it stands when the frame is saved, the header's,
  each divided by the frame's binning and rounded down; the region's size is `NAXIS1`
  and `NAXIS2` with `XBINNING`; `SET-TEMP` SHALL be the frame's `set_temp_c`, written
  only when it has one; `SERIALNO` SHALL be the camera's serial, written only when it
  has one (US2-S2, US4-S2; owner decision 16, owner approval 11).
  `DATE-OBS` SHALL be the exposure's start on the node's clock less the link's clock
  offset, as UTC to the millisecond; `IMAGETYP` SHALL be `DARK` for the purpose `dark`,
  `FLAT` for `flat` and `LIGHT` otherwise (owner decisions 2, 5 and 6).
- **FR-013**: `POST /api/imaging/abort` SHALL, under the same-origin rule, abort the
  exposure in progress through `AbortExposure`, or stop receiving a frame in flight, end
  the frame as `aborted`, and answer 200 with the view; with nothing in progress it
  SHALL answer 200 and change nothing.
- **FR-014**: Imaging frames SHALL cross only the second connection, so that the scope
  link's heartbeat keeps its 250 ms period and the link stays connected while a frame
  is in flight at any forwarding rate (US2-S4).
- **FR-015**: `crates/finder/README.md` SHALL document the two routes, the `frame`
  object, the files and their cards, that an imaging frame is never rewritten, and how
  the time a frame took to reach the console is read from `received_at`, `start` and
  `exposure_ms`.
- **FR-016**: `finder::imaging::fits` SHALL read back, with `decode`, any image `encode`
  writes, 8-bit or 16-bit, with the cards after the structural ones in their order, and
  SHALL refuse anything else with a `FitsError` that displays the texts of US3-S1.
- **FR-017**: The console SHALL make a preview of each saved frame as plan.md § The
  preview gives it: superpixel colour for a mosaic and gray for mono, box-averaged by a
  whole factor until its longer side is at most 1280 pixels, stretched, and encoded as
  a JPEG of quality 85 (owner decision 7).
- **FR-018**: `GET /api/imaging/frames/{id}/preview.jpg` SHALL answer the preview of that
  frame as `image/jpeg` while it is among the last eight, 404 with
  `the console no longer holds that preview` otherwise, and 400 for an id that is not a
  number. A saved frame's view SHALL carry the route as `frame.preview`.
- **FR-019**: `crates/finder/README.md` SHALL document the preview route and how the
  preview is made.
- **FR-020**: The page SHALL gain the route `/imaging` and, while the session has a
  `node`, a link `Imaging` in the top bar after `Setup`, marked current on that route
  (owner approval 5, owner decision 10).
- **FR-021**: The Imaging page SHALL show, from `GET /api/imaging` read when it opens,
  every second and after each of its own requests, the words of US7-S2 for each state
  and, when ready, the camera, its status and a settings form with the fields the
  camera has and the forwarding limit, saved with one `PUT /api/imaging/settings`
  (owner decisions 8 and 12).
- **FR-022**: The page SHALL offer `Take imaging frame` and, during a frame,
  `Abort exposure`; show the frame's progress, file, time in hand after the exposure,
  node copy and failure with the words of US7-S4; and show the latest preview as a
  picture.
- **FR-023**: At night the picture SHALL take the red-only filter. Under it the page
  SHALL offer `Hold to see colour`, at night only: the picture shows its colours only
  while the control is held, for at most 10 s counted from the first press, and nothing
  else changes. The control SHALL be built once, as the hook `useColourHold` in
  `crates/finder/web/src/useColourHold.ts`, which the Imaging page uses and 035's stack
  picture will use (US7-S5). `DESIGN.md` SHALL carry the exception with the words of owner approval
  6, `.impeccable/design.json` SHALL list the Imaging page, and `## Night mode` in
  `crates/finder/README.md` SHALL gain one sentence on the held colour (owner approval
  9).
- **FR-024**: `crates/finder/web/dist` SHALL be the build of its source, the page SHALL
  add no npm package, and `crates/finder/web/playwright.config.ts` SHALL gain the two
  entries of owner approval 8, with the simulate project's `testMatch` list written one
  entry a line as Prettier then requires, and nothing else.
- **FR-025**: The console SHALL offer the MCP tools `get_imaging_camera`, which answers
  the imaging view and changes nothing, and `set_imaging_camera`, which takes the body
  of `PUT /api/imaging/settings`, publishes its nine keys as the tool's input schema,
  and answers the view or the route's refusal as a tool error (US8-S1, owner approval
  4).
- **FR-026**: `take_imaging_frame` SHALL take no arguments, do what
  `POST /api/imaging/frame` does, wait until the frame ends, and answer the view with
  the frame's preview as a JPEG image block in colour; a refusal and an aborted frame
  SHALL be tool errors with the texts of US8-S4, and a failed frame a tool error with
  the frame's own `error` (owner decision 13).
- **FR-027**: On a console that takes no imaging frames the three tools SHALL answer at
  once, as US8-S5 says. No tool's result SHALL hold the node's token.
  `crates/finder/README.md` SHALL list the three tools.
- **FR-028**: The console's imaging state SHALL hold the imaging camera for at most one
  holder at a time (the plan document's Amendment 1; owner approval 12).
  `Imaging::lease(holder)` SHALL never wait and SHALL look at nothing but the lease: it
  hands out a `Lease` when none is out, and otherwise answers a `Refusal` of 409 with
  `the imaging camera is held by {holder}`, the name of the one who holds it.
  `Imaging::holder` SHALL answer that name while a lease is out (US9-S1).
- **FR-029**: While a lease is out, `Imaging::set` and `Imaging::take` SHALL answer that
  409 to anyone but the holder, after the check of the state and before the check for
  an imaging frame in flight; and so `PUT /api/imaging/settings` and
  `POST /api/imaging/frame` SHALL answer it as their 409, and `set_imaging_camera` and
  `take_imaging_frame` as a tool error with its words (US9-S2).
- **FR-030**: `Lease::set` and `Lease::take` SHALL be `Imaging::set` and `Imaging::take`
  less the lease's check: the same checks, refusals, words and order otherwise
  (US9-S3).
- **FR-031**: `Imaging::abort`, `Imaging::ended`, `Imaging::view` and `Imaging::preview`,
  and so `GET /api/imaging`, `POST /api/imaging/abort`,
  `GET /api/imaging/frames/{id}/preview.jpg` and `get_imaging_camera`, SHALL stay open
  to anyone, lease or no lease (US9-S4).
- **FR-032**: Dropping a `Lease` SHALL release the camera at once, also when the task
  that owns it is aborted. A `Lease` SHALL hold no strong reference to the `Imaging`;
  once the `Imaging` is gone, its `set` and `take` SHALL answer 409
  `this console takes no imaging frames` and its drop SHALL do nothing (US9-S5).
- **FR-033**: An imaging frame asked for with `Imaging::take`, from the page or an MCP
  client, SHALL hold no lease: `holder()` stays `None` while it is in flight and a lease
  may still be granted, so each holder looks for a frame in flight itself (US9-S6).
- **FR-034**: `## Imaging frames` in `crates/finder/README.md` SHALL gain one paragraph
  on the camera's one holder: what is refused while a lease is out and with which words,
  what stays open, and that a single imaging frame holds no lease (owner approval 9).

### Key Entities

- **Imaging frame**: one exposure from the imaging camera as the scope node relays it.
- **Imaging settings**: gain, offset, binning, region of interest and cooler, held by
  the camera; exposure, forwarding rate and the node copy, held by the console.
- **Imaging view**: the console's `imaging` object: state, camera, status, settings and
  the latest frame.
- **Purpose label**: why an exposure is taken; `frame` here, more in later specs (the
  label is 030's).
- **Lease** (of the imaging camera): the console's one holder of the imaging camera, by
  name; while it is out, nothing else changes the camera's settings or starts an
  exposure.
- **Preview** (of an imaging frame): a small colour JPEG of a saved imaging frame, kept
  in memory for the page and MCP clients.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario has a committed test that asserts it, and the
  `lint` and `test` gates pass.
- **SC-002**: No existing test, golden, `scripts/ci/` file, `.github/` file, `CODEOWNERS`
  entry, `ergane.yaml` or `.specify/` file is edited. The only edits to existing test or
  gate files are the `mod` lines and the two `playwright.config.ts` entries of owner
  approval 8, with the re-wrap of one list that Prettier requires.
- **SC-003**: A frame in flight for 28 s leaves the scope link connected, with its
  heartbeats answered meanwhile (US2-S4).
- **SC-004**: A saved FITS file holds the camera's samples exactly (US2-S1).
- **SC-005**: No file under `crates/finder` names a camera model outside tests,
  documents and comments, and `crates/finder` gains no `unsafe`.
- **SC-006**: At night the Imaging page shows no off-red pixel except in the picture
  while the control is held (US7-S8).
- **SC-007**: While a lease is out, no settings change and no imaging frame reaches the
  camera except through that lease, and an exposure can still be aborted by anyone
  (US9-S2, US9-S4).

## Assumptions

- **Earlier features are in place.** `025-frame-capture` and
  `028-finder-tests-do-less` have landed on `main`, 028 completely at 49bb210, and with
  them every spec they name. `030-imaging-camera-link` has landed too, all eight
  stories, at 49f8dcd. Every citation was drafted at dbc2b5b, checked again at 49bb210
  and re-anchored at 49f8dcd, the worktree's HEAD; the places that moved are corrected,
  and the scratch-folder helper 028 adds is cited (plan.md § Prerequisites 2). Every
  file of 030 is named without a line.
- **Specs 032 to 037 build on this one** and are re-anchored after it lands.
- **Out of scope:**
  - everything on the scope node's side of the scope link: the camera port, the camera
    adapters, the service, the scope node's own copy and the probe (030);
  - solving an imaging frame, the aim point and the imaging angle (032);
  - any focus reading or focus aid (033);
  - darks, flats and the calibration library, beyond writing `IMAGETYP` from the purpose
    label (034);
  - stacking, and any stretch beyond the preview's (035);
  - tracking from imaging frames (036) and the imaging run (037);
  - any change to 015's `imaging_camera` numbers (owner decision 14);
  - any change to the finder camera, star detection, the live loop or recordings;
  - listing, fetching or deleting saved frames through the API;
  - previews saved to disk, and a preview of a frame the console no longer holds;
  - any calibration card;
  - a lease that waits for its holder, one lent from a holder to another, and any
    holder of this spec's own: nothing here takes frames one after another;
  - a region of interest control on the page;
  - MCP anywhere but on the console.
- **Outside the factory:** rebuilding the console, after the scope node has been rebuilt
  from 030; the field checks at the telescope: a frame from the real camera saved by the
  console and opened in another astronomy program; the time across the link, read on
  the Imaging page as `in hand … s after the exposure` less the figure of 030's probe;
  heartbeats keeping time while frames cross the real link; and a design review of the
  preview's look and of the night exception on a phone in the dark.
- **The factory's gates and CI:** the gates are `lint` and `test`, copied in plan.md
  § Verification. CI's other jobs run only in the merge queue, so no scenario's Then
  clause rests on them. No story of this trio adds a dependency.

## Work Graph

```yaml
US1:
  depends_on: []
  implements: [FR-001, FR-002, FR-003, FR-004, FR-005, FR-006, FR-007]
US2:
  depends_on: []
  depends_on_merged: [US1]
  implements: [FR-008, FR-009, FR-011, FR-012, FR-014, FR-015]
US3:
  depends_on: []
  depends_on_merged: [US2]
  implements: [FR-016]
US4:
  depends_on: []
  depends_on_merged: [US2, US3]
  implements: [FR-010]
US5:
  depends_on: []
  depends_on_merged: [US2, US4]
  implements: [FR-008, FR-009, FR-013, FR-015]
US6:
  depends_on: []
  depends_on_merged: [US5]
  implements: [FR-017, FR-018, FR-019]
US7:
  depends_on: []
  depends_on_merged: [US6]
  implements: [FR-020, FR-021, FR-022, FR-023, FR-024]
US8:
  depends_on: []
  depends_on_merged: [US7]
  implements: [FR-025, FR-026, FR-027]
US9:
  depends_on: []
  depends_on_merged: [US6, US8]
  implements: [FR-028, FR-029, FR-030, FR-031, FR-032, FR-033, FR-034]
```

Every edge is a merge edge, because each dependent imports or extends what its
predecessor creates. Nothing here is ordered against a story of 030: that whole trio
has landed before US1 starts (`depends_on_landed`). US2 and US5 share three
requirements, each in its own half: FR-008's exposure, refused while the state is not
`ready`, is US2's, and its frame-in-flight refusal and `save_at_node` are US5's, whose
US5-S1 asserts both refusals; FR-009's frame is US2's and `status.exposing`, `node_file` and
`node_save_error` US5's; FR-015's rows are US2's but for the abort row, which is US5's.

- **US2 after US1**: frames extend US1's state, client and routes.
- **US3 after US2**: `decode` reads what US2's `encode` writes, in the file US2 created.
- **US4 after US2 and US3**: the checks extend US2's receiving in `frames.rs`, and US4
  adds its `mod` line to `crates/finder/tests/console/main.rs` after US3's.
- **US5 after US2 and US4**: refusals, abort and the node's copy extend US2's state,
  routes and README, and `frames.rs` and `crates/finder/tests/console/main.rs` after
  US4. The edge to US2 follows from the one to US4; it is written out because US5
  writes the files US2 last wrote.
- **US6 after US5**: the preview is made from US2's frames and adds `frame.preview`,
  in the files US5 last wrote.
- **US7 after US6**: the page shows US6's preview and the frame words of US2 and US5.
  It writes only the page, `DESIGN.md`, `.impeccable/design.json` and the README.
- **US8 after US7**: the tools return US6's preview; the edge to US7 orders their two
  edits of `crates/finder/README.md`.
- **US9 after US6 and US8**: the lease guards `Imaging::set` and `Imaging::take`, which
  US1, US2 and US5 write, and its test calls US6's preview route and US8's tools, so it
  is the last writer of each file it shares: `crates/finder/src/imaging/mod.rs`,
  `crates/finder/src/imaging/frames.rs` and `crates/finder/tests/console/main.rs` after
  US6, and `crates/finder/README.md` after US8. The edge to US6 follows from the one to
  US8, which comes after US6; it is written out because US9 writes the files US6 last
  wrote.

The nine stories form one chain, so no two run together.
