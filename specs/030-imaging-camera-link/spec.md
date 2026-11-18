---
state: ready
depends_on_landed: ["025-frame-capture", "028-finder-tests-do-less"]
# Flipped to ready 2026-11-17 by the operator seat on the owner's answer the same day ("Yes"
# to flipping it now, after round 1's probes), which gives the owner approvals listed under
# "Owner approvals". Every owner decision takes its settled answer: 3, 8, 12 and the white
# balance of 13 by the plan document's amendments, and the rest the recommended one, which
# the owner approved on 2026-11-17 with the imaging series' other decisions. Re-anchored
# first to `main` at 86ebeb8: two `crates/finder/src/link.rs` citations moved, after
# 026's US1, and are corrected; no other cited file changed. Validated there.
# Drafted 2026-11-15 against `main` at dbc2b5b from
# docs/plans/2026-11-15-specs-030-037-imaging.md, by the grilling session of that day.
# Reviewed on 2026-11-15 and the review's findings applied; then
# audited beside the other trios of the series the same day and the audit's findings
# applied (the plan document's Amendments 2 and 6). Split the same day, at the owner's
# ask, from the one trio that held both sides of the scope link: this trio keeps the
# scope node's side and the link, and 031-imaging-frames-on-console holds the console's
# side (§ Clarifications, the last session).
# Citations checked against `main` at 49bb210
# (49bb210fe4860dd98ba24727074e7bd3cc0d7124), the worktree's HEAD, which completes 028;
# none of the cited files changed by `main` at 2f6a777.
# Probe-built on 2026-11-17 (round 1, every story): the round-1 probe fixes (defects 1
# to 44) are in, and the first INDI story is split in two as measured, US4 connected
# and described and US5 configured, so the trio holds eight stories (plan.md
# § Refinement Record).
---

# Feature Specification: The Imaging Camera on the Scope Link

**Feature Branch**: `030-imaging-camera-link`

**Created**: 2026-11-15

**Status**: Draft

**Input**: The console finds, reaches and follows a target with the finder camera. The
owner wants an imaging camera at the main telescope's focuser, and in the end one ask
that returns a stacked colour picture (specs 030 to 037). This spec is the first step and
the base of the other seven: the scope node's side, and the link. The scope node takes
exposures from an imaging camera through an INDI server on the same machine and relays
each imaging frame untouched over the scope link, by a new service that its caller gives
a connection of its own. It can also keep its own copy of each frame, and describe and
time the camera by itself. A simulated imaging camera stands in for tests and
`finder --simulate`. The console's side (its second connection, each imaging frame saved
as a FITS file, the Imaging page, the MCP tools and the camera's one holder) is
`031-imaging-frames-on-console`, which builds on this spec. Nothing here solves,
focuses, calibrates or stacks.

## Clarifications

### Session 2026-11-15 (the interview's decisions, from the plan document)

- Q: What does this trio read besides itself? → A: Nothing about the feature. `spec.md`
  holds every requirement and scenario, and `plan.md` holds every design decision, the
  interfaces, the shapes that existing tests pin and the alternatives rejected. The
  binding decisions come from `docs/plans/2026-11-15-specs-030-037-imaging.md`,
  `docs/adr/0002-imaging-cameras-through-indi-frames-relayed-console-stacks.md` and
  `GLOSSARY.md`; facts come from `docs/research/2026-11-15-imaging-camera.md`. A story
  that would change a decision escalates rather than re-deciding it.
- Q: Where does the work run? → A: Relay-first. The scope node takes exposures and
  forwards each imaging frame untouched; the console saves it. The scope node does no
  pixel work of its own: it converts a FITS container's big-endian samples to the link's
  little-endian samples and nothing else.
- Q: How is the camera reached? → A: Through INDI. The scope node is a client, over TCP,
  of an INDI server on the same machine (by default 127.0.0.1, port 7624), behind the
  camera port. Nothing is keyed to one camera model: sizes, ranges, the Bayer pattern and the
  cooler are read from the camera (`docs/research/2026-11-15-imaging-camera.md` § 2.
  INDI). The owner's camera, a 4944 × 3284 colour camera, appears below only as a worked
  example and a size.
- Q: What does "half resolution" mean? → A: `binning` 2 in the imaging settings, and
  nothing else. On the owner's make of camera 2 × 2 binning is done in software after the
  whole frame has crossed USB. That a colour camera's binned frame keeps the Bayer
  mosaic is inferred, not established: the maker does not say so, and the note marks it
  "test on the camera" (`docs/research/2026-11-15-imaging-camera.md` § 1. The camera
  and § 13. Not established). It is a field check (Assumptions, Outside the factory);
  the two stand-ins keep the mosaic by a rule of their own. The scope node never bins.
- Q: How do frames cross the scope link? → A: By a new service, `ScopeNodeImaging`, on a
  second connection that the console opens only for it. An exposure is answered by a
  stream: one header, then the samples in chunks of at most 256 KiB (262,144 bytes). The
  250 ms heartbeat, whose 1 s of silence stops the mount, never shares a connection with
  frame bytes. One 16-bit 4944 × 3284 frame is 32,472,192 bytes, 124 chunks
  (`docs/research/2026-11-15-imaging-camera.md` § 11. Frame transport).

### Session 2026-11-15 (review of `main` at dbc2b5b, re-read at 49bb210)

- Q: What was found in the code? → A: Nine things on this side of the link, each read at
  dbc2b5b and its citation re-read at 49bb210:
  1. The camera port is one trait that yields 8-bit gray frames from one camera
     (`crates/scope-node/src/camera.rs:31`, `crates/scope-node/src/camera.rs:17`), and the
     node's service owns one camera and one capture task
     (`crates/scope-node/src/service.rs:149`). The imaging side is therefore a second
     trait and a second object beside them, not a change to either.
  2. A new call on the `ScopeNode` service would stop two hand-written implementations
     compiling (`crates/scope-link/tests/link.rs:55`, `crates/finder/tests/cli/aim.rs:22`).
     A new service does not: 025 added `ScopeNodeRaw` that way
     (`crates/scope-link/proto/scope_node/v1/scope_node.proto:151`).
  3. The console builds every client from clones of one channel
     (`crates/finder/src/link.rs:377`), sends a heartbeat every 250 ms and treats 1 s of
     silence as a lost link (`crates/finder/src/link.rs:29`,
     `crates/finder/src/link.rs:30`); the node's watchdog stops the mount after 1 s
     without a heartbeat (`crates/scope-node/src/mount/watchdog.rs:42`). 025 refused raw
     frames while anything moves for this reason. Imaging frames run while the mount
     tracks, so they need their own connection.
  4. The scope node's crate forbids `unsafe` (`crates/scope-node/src/lib.rs:2`), and so
     does the simulator's (`crates/simulator/src/lib.rs:2`). An INDI client over TCP
     needs none.
  5. `Cargo.lock` holds no XML reader. It holds `base64` 0.23.1 for the console
     (`Cargo.lock:241`, `crates/finder/Cargo.toml:11`); the scope node's manifest has
     neither (`crates/scope-node/Cargo.toml:10`). See owner decision 4.
  6. The scope node's binary takes one adapter from the config
     (`crates/scope-node/src/main.rs:55`), its config refuses unknown keys
     (`crates/scope-node/src/config.rs:87`), and `adapter = "simulated"` there means the
     node's own seeded stand-in, not the simulator crate's camera, because the simulator
     depends on the scope node (`crates/simulator/src/node.rs:40`). The imaging side
     follows the same split (owner decision 3).
  7. The simulator draws the pattern database's stars: about one star a square degree,
     so two imaging fields in three would be empty; its frames are 8-bit, mono, with one
     position for each star in an exposure (`crates/simulator/src/render.rs:27`,
     `crates/simulator/src/render.rs:58`, `crates/simulator/src/camera.rs:106`). The
     simulated imaging camera therefore has a renderer and a star field of its own.
  8. The main telescope's axis is already simulator state: the eyepiece pixel of the
     finder frame (`crates/simulator/src/optics.rs:13`,
     `crates/simulator/src/optics.rs:43`), and a connected mount's true direction is
     that direction (`crates/simulator/src/controls.rs:35`). The simulated imaging
     camera looks along it.
  9. Per-pixel rendering is already most of the `test` gate (`Cargo.toml:15`). The
     simulated imaging camera is small by default, 1236 × 822, and draws one Gaussian
     deviate a sample.
- Q: Which INDI names were checked, and against what? → A: Against INDI's
  standard-properties page on 2026-11-15: `CONNECTION` (`CONNECT`, `DISCONNECT`),
  `DRIVER_INFO` (`DRIVER_INTERFACE`), `CCD_EXPOSURE` (`CCD_EXPOSURE_VALUE`, seconds),
  `CCD_ABORT_EXPOSURE` (`ABORT`), `CCD_FRAME` (`X`, `Y`, `WIDTH`, `HEIGHT`),
  `CCD_BINNING` (`HOR_BIN`, `VER_BIN`), `CCD_INFO` (`CCD_MAX_X`, `CCD_MAX_Y`,
  `CCD_PIXEL_SIZE`, `CCD_BITSPERPIXEL`), `CCD_CFA` (`CFA_TYPE`), `CCD_TEMPERATURE`
  (`CCD_TEMPERATURE_VALUE`), `CCD_COOLER` (`COOLER_ON`, `COOLER_OFF`), `CCD_FRAME_TYPE`
  (`FRAME_LIGHT`, `FRAME_BIAS`, `FRAME_DARK`, `FRAME_FLAT`), `CCD_COMPRESSION`
  (`INDI_ENABLED`, `INDI_DISABLED`), `CCD_TRANSFER_FORMAT` (`FORMAT_FITS`),
  `UPLOAD_MODE` (`UPLOAD_CLIENT`) and the BLOB `CCD1`. Four names are not on that page
  and were read in the source of the two device programs that define them:
  `CCD_COOLER_POWER` (`CCD_COOLER_VALUE`), `CCD_CONTROLS` (one number for each control,
  named by the camera maker's library; the program itself looks up `Gain`) and
  `Serial Number` (`SN`) in the ZWO device program, and `CCD_GAIN` (`GAIN`) and
  `CCD_OFFSET` (`OFFSET`) in INDI's camera simulator. The camera bit of
  `DRIVER_INTERFACE` is 2. Not checked on a camera: that the ZWO program's offset
  control is named `Offset` on the owner's model (plan.md § Risks). The review of this
  draft re-read the ZWO program's source for its failure handling: it starts an exposure
  up to three times, 100 ms apart, and restarts a failed exposure once, two attempts in
  all, before it reports failure (its retry limit is 2 and is compared before the
  restart); its USB reset is off unless switched on. The research note's § 2 first worded
  the restart as "up to 2 times" and now reads as here (plan.md § Prerequisites 6).
- Q: Can the INDI adapter be tested with no camera? → A: Yes, against a scripted stand-in
  for the INDI server that the tests run over an in-memory stream. Hand-written scripts
  come from INDI's documents and cover every path. One script is a saved capture,
  recorded from the INDI project's own camera simulator: the grilling session could not
  run downloaded programs, and the capture was recorded on 2026-11-17 from INDI 1.9.9's
  camera simulator with plan.md's own script and staged in `fixtures/scope-node/indi/`
  (plan.md § Prerequisites 4). plan.md § The script format and § Recording the
  conversation define both exactly.
- Q: Does the constitution's "detection input is 8-bit grayscale" bind imaging frames? →
  A: No. An imaging frame is never detection input in this spec: the finder camera's
  frames and star detection are untouched.
- Q: Which other drafts write the same files? → A: None that is known. Three other
  drafts are known, each `state: draft` on a branch of this repository and none on
  `main`: `026-mount-sky`, `021-chart-orientation` and `022-altaz-guidance`. Every file
  their Refinement Records share with this series is under `crates/finder`, or is
  `DESIGN.md`: files of 031, which lists them and says how the flips are ordered (its
  plan.md § Prerequisites 8). None of those files is one this trio writes. No draft
  numbered 023 was found on any branch read here; whether one exists elsewhere is the
  owner's to say before the flip. Specs 031 to 037 build on this one, and those that
  write its files write them after it.
- Q: Where does this trio depart from the plan document's § Interfaces shared across the
  specs? → A: Nowhere now. The review found three on this side of the link; the
  plan document's § Amendments after 030, 031, 032 and 034 were drafted and reviewed settle
  two of them as this trio has them, its § Interfaces shared across the specs now
  carries the third as this trio has it, and the owner decisions say so:
  1. `[imaging] adapter = "simulated"` on the node's binary builds the scope node's own
     stand-in, and `SimImagingCamera` in `crates/simulator` serves the tests that need
     stars: settled by Amendment 4 (owner decision 3).
  2. `ExposeRequest` carries `max_bytes_per_s`, the forwarding rate limit as a console
     setting sent with each exposure: settled by Amendment 5 (owner decision 8).
  3. `ExposeRequest` also carries `save`, the ask for a copy at the scope node, and the
     node's config table holds `save_dir`: the plan document's `Expose` now takes
     "whether the node also keeps the frame", and its `[imaging]` table lists
     `save_dir`. How the node keeps a copy is owner decision 9, settled at the flip on its
     recommended answer.

  Had the owner settled owner decision 9 otherwise, the plan document would have been
  corrected in the same change (plan.md § Prerequisites 9). The review's fourth finding, where the console
  puts its files, is 031's, and Amendment 3 settles it there.
- Q: Which FITS keywords does the scope node's own copy write, and against what were
  they checked? → A: On 2026-11-15. `XORGSUBF` and `YORGSUBF`, in the SBIG FITS
  extension document, are a sub-frame's origin "relative to whole frame in binned pixel
  units". `ROWORDER` is among the cards INDI writes
  (`docs/research/2026-11-15-imaging-camera.md` § 2. INDI), and `BSCALE` 1 goes with
  `BZERO` in the FITS convention for unsigned samples. `FRAMEID`, `PURPOSE` and
  `NODE-T0` are this project's own. The SBIG document is in the research note's source
  list (plan.md § Prerequisites 6).

### Session 2026-11-15 (the audit of 030 to 037, and the plan document's amendments)

- Q: What does the INDI adapter do with a camera's white-balance controls? → A: What
  Amendment 2 says: "Where a camera offers white-balance controls they scale the raw
  samples. The INDI adapter (030) sets each to its neutral value once, when it connects,
  so every frame of every night is on the same footing for calibration. No spec records
  them per frame." No control may be keyed to a maker's name in code, so the scope
  node's config names them: `[imaging] neutral`, a table of `"<property>.<element>" =
  value` (owner decision 13, FR-020, FR-021). The draft's bring-up step and its hand-off
  of recording to 034, which 034 declined (its owner decision 17), are gone.
- Q: Where does a frame's region of interest come from? → A: From the scope node, with
  the frame, as Amendment 6 has it: "A region of interest is recorded with the frame (its
  origin and size on the sensor, in the header and the FITS file)." `ImagingFrame` and
  the header carry the region in force for the exposure; the scope node's own copy
  writes its origin as `XORGSUBF` and `YORGSUBF`, as 031's file does from the header;
  the size in the file is `NAXIS1` and `NAXIS2` with `XBINNING` (owner decision 12).

### Session 2026-11-15 (the split into two trios)

- Q: Why are there two trios where one was drafted? → A: The owner asked for 030 to
  be split if that helps its builders. One trio held thirteen stories on both sides of
  the scope link, more than a builder should read for one story. It was cut where the
  scope link is. This trio keeps the scope node's side and the link, seven stories then
  (eight since the round-1 probe builds split the first INDI story: the last session): the
  imaging side of the camera port, the scope node's stand-in camera, `SimImagingCamera`
  in the simulator, the `ScopeNodeImaging` service, the INDI adapter, the scope node's
  own copy of a frame and the camera probe. `031-imaging-frames-on-console` holds the
  console's side, six stories, and names this trio in its `depends_on_landed`. No
  decision changed in the split. One story straddled the link, the scope node's own
  copy, and was cut there: what the scope node does is US7 here, and what the console
  shows of it is 031's US5-S4.
- Q: What does 031 take from this trio? → A: Names only, each from plan.md
  § Interfaces: the service `ScopeNodeImaging` and its messages (§ The scope link);
  `NodeService::with_imaging` and `with_imaging_save_dir` (§ The node's service, § The
  node's copy); the trait `ImagingCamera` with `ImagingInfo`, `ImagingSettings`,
  `ImagingStatus`, `Exposure`, `ImagingFrame`, `Roi`, `Range`, `BayerPattern` and
  `ImagingError`, which its tests implement and build (§ The port);
  `SyntheticImagingCamera` and its sample rule (§ The stand-in camera); and
  `SimImagingCamera`, `SimStar`, the imaging truth and `SimNode::new` (§ The simulated
  imaging camera, § `SimNode`). This trio reads nothing
  of 031.

### Session 2026-11-17 (the round-1 probe builds)

- Q: What did the probe builds find? → A: Every story was built from this trio in a
  throwaway worktree, tests first, with both gates, `cargo deny check` and
  `cargo machete` (plan.md § Refinement Record, round 1). Every interface built as
  plan.md writes it, and every story passed as written. One did not fit: the first
  INDI story as drafted measured 86,286 bytes against the 64 KiB budget, and the split
  prepared in plan.md § Risks still left 69,756. Forty-four defects were reported, that
  one blocking; their fixes are in this trio, and none adds a requirement.
- Q: How is that story split? → A: As the probe measured it, 53,460 and 38,979 bytes:
  US4, an INDI camera connected and described (the draft story's first, second and
  fifth scenarios), and US5, an INDI camera configured (its third, fourth, sixth,
  seventh and eighth).
  The stories after them are renumbered: exposures through INDI are US6, the node's
  copy US7 and the probe US8. The two worked scripts are written once, in
  `crates/scope-node/tests/support/mod.rs` beside the stand-in, so every INDI test file
  shares them (owner decision 1).
- Q: Which assertions did the probes add? → A: Each where a build broken on purpose
  passed every test (the probes' bite checks), so that the scenario now catches it:
  - US1-S2: the cooler's `apply` answers gain 200 and binning 2 still in force;
  - US2-S6: the mount part's moments over 41 × 41 samples, and its centroid within
    0.2 px of the midpoint, where 0.5 px could not tell the smear's points apart;
  - US2-S8: no two stars of the field have the same magnitude;
  - US3-S4: a node made unresponsive does not answer `DescribeImaging`;
  - US3-S8: the limited stream's last chunk between 7.68 s and 7.70 s, where 7.5 to
    8.0 s let a pacing rule that counts the chunk about to leave pass;
  - US5-S3: a `neutral` key is split at its first `.`, shown with `CCD_CONTROLS.WB.R`;
  - US5-S5: a control the camera lacks fails the connection, so a second `describe()`
    connects again;
  - US6-S1: two `Busy` answers, as INDI's camera simulator sends more than one, and
    `start_s` at the first;
  - US6-S2: a region another INDI client sets during an exposure leaves the frame's
    `roi` as it stood when the exposure was asked for, and a FITS header's `XBINNING`
    wins over `HOR_BIN` while `CCD_CFA` stands in for a missing `BAYERPAT`;
  - US6-S4: the wait for an imaging frame counts from the request, not from `Busy`;
  - US6-S6: the late BLOB is another picture, so one handed to the next `expose` fails;
  - US7-S2: an 8-bit mono frame with no gain, offset or temperature is saved with none
    of their cards and no `BZERO` or `BSCALE`;
  - US8-S3: the built probe takes `--probe-exposure-ms`, so it exits within 2.5 s.
- Q: Which behaviours did the probes settle where the plan left a choice? → A: These,
  each written where plan.md gives the behaviour: `check_settings` checks the region
  only when an edit names `roi` or `binning`, so a gain-only edit is never refused for a
  region another INDI client set; a set temperature asked for while the cooler is off
  is remembered, and sent when the cooler goes on; a connection is ready only once
  `CCD_INFO` reports a sample size other than 0, since INDI's camera simulator first
  defines it with zeros; an `Alert` exposure's text takes only that answer's own
  `message`; a BLOB that is not base64 or holds no `oneBLOB`, and a reconnect that
  fails before the second try, have their own texts; and the simulated camera's frame
  number counts the frames it delivered.
- Q: What is still caught by no test? → A: The binary's attaching of the imaging camera
  and its `save_dir` in `serve` (plan.md § Risks); that frames need their own
  connection, which US3-S8 cannot show in paused time over memory links, so it stays a
  design rule; the probe's flush; and whatever a live INDI server's timing would show,
  since the saved capture holds no times.

## Owner decisions

Each is a design or taste call the plan document leaves open, or, where its heading
says so, a place where this trio departs from the plan document. Decisions 3 and 8 are
settled by the plan document's Amendments 4 and 5, as recommended here, and decision 12
and the white balance of decision 13 by Amendments 6 and 2; each says so, and keeps its
other answer only as the record of what was closed. The rest take their recommended
answers: the owner approved the imaging series' recommendations on 2026-11-17
("im good with all of the info in the decisions doc") and flipped this spec the same day. None is left to a story.

1. **Eight stories on this side of the link.** Recommended: eight, so that each stays
   under 64 KiB: the port and the simulated camera are two stories, the INDI adapter is
   three (connected and described; configured; exposures), and the service, the scope
   node's own copy and the probe are one each. The draft had seven, with the adapter's
   first two in one story, which the round-1 probe build measured at 86,286 bytes; the
   two halves measured 53,460 and 38,979. Instead: five, as the plan document's first
   list had this side, with the port and the simulated camera in one story and the INDI
   adapter in one, two stories likely to pass the limit.
2. **The port is asynchronous, and settings are a partial edit.** Recommended: every
   method of the imaging side of the camera port returns a future, since an INDI camera
   is a network away; `apply` takes only the settings to change and answers all the
   settings in force. Instead: synchronous `describe` and `status`, and a whole settings
   object each time.
3. **Two stand-in cameras** (settled by the plan document's Amendment 4, as
   recommended; the review had it as a departure from § Interfaces shared across the
   specs, which then named `SimImagingCamera` in `crates/simulator` as the `simulated`
   adapter). Recommended:
   `[imaging] adapter = "simulated"` on the node's binary is a small seeded stand-in in
   the scope node's crate (64 × 48), as `adapter = "simulated"` already is for the
   finder camera, because the simulator depends on the scope node and the node's binary
   cannot build the simulator's camera; the sky-rendering `SimImagingCamera` lives in
   `crates/simulator` and serves tests and `finder --simulate`. Instead: move the
   simulator's camera into the scope node's crate.
4. **Reading INDI's XML.** Recommended: the `quick-xml` crate, pinned `=0.42.0`, MIT,
   with its `async-tokio` feature, and `base64` `=0.23.1`, MIT OR Apache-2.0, already in
   `Cargo.lock`; both for `scope-node` only, each added by the story whose code first
   uses it, `quick-xml` in US4 and `base64` in US6 (owner approval 3). Instead: a
   hand-written reader and decoder, about 300 more lines in the adapter and no
   dependency.
5. **One INDI connection.** Recommended: one TCP connection with `enableBLOB` `Also`.
   Instead: a second connection with `Only` for BLOBs; or INDI's same-machine fast BLOBs,
   which pass a file descriptor and so need a new crate or `unsafe`.
6. **A failed exposure.** Recommended: the adapter waits for its frame the exposure and
   60 s more, counted from its request, treats that or INDI's Alert as a failure,
   aborts, waits 1 s, tries once more and then answers an error. It relies on what the
   ZWO device program always does (three starts 100 ms apart, and one restart of a
   failed exposure) and never on its USB reset, which is off by default. Instead: other
   numbers, or no retry.
7. **The purpose label.** Recommended: 1 to 16 lower-case letters, free; `dark` and
   `flat` choose the camera's frame type, any other label its normal type; in the scope
   node's own copy the FITS card `IMAGETYP` is `DARK`, `FLAT` or `LIGHT`, as it is in
   the console's file (031's owner decision 2). `LIGHT` is a named exception to the
   glossary's "avoid light": it is the value other programs read in that card, not this
   project's word for a sub. Later specs add labels with no change to the scope node.
   Instead: a fixed list on the wire.
8. **The forwarding rate limit** (settled by the plan document's Amendment 5, "a
   console setting sent with each exposure, not a node config key", as recommended).
   Recommended: asked for with each exposure, as `max_bytes_per_s` on `ExposeRequest`,
   0 for no limit; the scope node's config has no key for it, and the console's setting
   that fills it is 031's (its owner decision 8). Instead: a key in the node's config.
9. **The copy at the scope node** (no longer a departure from the plan document: its
   § Interfaces shared across the specs now has `Expose` also take whether the node
   keeps the frame, as this trio's `save` does, and the node's config `save_dir`).
   Recommended: the node writes a FITS file of its own, `frame-<id>.fits`, with the
   node clock's start time as `NODE-T0`, since the node has no wall clock, asked for
   with each exposure as `save`. Instead: tell INDI to save (`UPLOAD_BOTH`); or raw dumps; or no copy at the
   scope node, which drops US7 and `save` and `save_dir` with it, here and in the plan
   document, and with them 031's `save_at_node` setting and its US5-S4.
10. **The simulated sky.** Recommended: a seeded field of the simulator's own, 320 stars
    a square degree brighter than magnitude 16, and the constants of plan.md § The
    simulated imaging camera. Instead: a real deep catalogue file.
11. **A dropped `Expose` call.** Recommended: the node finishes the exposure and discards
    the frame; only `AbortExposure` aborts. Instead: abort when the caller goes away.
12. **Where a frame sat on the sensor** (settled by the plan document's Amendment 6: "A
    region of interest is recorded with the frame (its origin and size on the sensor, in
    the header and the FITS file)"). A frame taken with a region of interest must say
    which part of the sensor it is, and files are never rewritten, so this is settled
    before the first file exists, as the review's alternatives (a) and (b) together: the
    scope node records the region in force for the exposure as `ImagingFrame.roi`,
    unbinned sensor pixels (the INDI adapter reads `CCD_FRAME`, the two stand-ins their
    settings), and sends it in the header as `roi`; `XORGSUBF` and `YORGSUBF`, the
    origin in binned pixels, are written from it, always, in the scope node's own copy
    (US7), as 031 writes them in the console's file; the size in the file is `NAXIS1`
    and `NAXIS2` with `XBINNING`. A camera changed by another INDI client is therefore
    stamped as it really was. The node's copy carries neither `SET-TEMP` nor
    `SERIALNO`: the console stamps those two in its own file, and whether the scope
    node should stamp them instead is open in 031's owner decision 16, whose other
    answer adds two more fields to US3's header and puts both cards in the node's copy.
    (The review's recommendation, the console stamping the region too with nothing on
    the scope link, and its alternative (c), leaving the region to 033, are closed by
    the amendment.)
13. **The camera's white-balance controls** (settled by the plan document's Amendment 2:
    "Where a camera offers white-balance controls they scale the raw samples. The INDI
    adapter (030) sets each to its neutral value once, when it connects, so every frame
    of every night is on the same footing for calibration. No spec records them per
    frame."). The research note says that the red and blue white-balance controls of the
    owner's make of camera scale the raw samples and that the camera's own "neutral"
    values colour a long exposure's background, and advises 50 and 50
    (`docs/research/2026-11-15-imaging-camera.md` § 1. The camera and § 2. INDI). They
    are one maker's names, so no code may be keyed to them: the scope node's config
    names them. `[imaging]` gains, for the `indi` adapter only, an optional table
    `neutral` of `"<property>.<element>" = value`; the worked example for the research
    note's camera is `neutral = { "CCD_CONTROLS.WB_R" = 50, "CCD_CONTROLS.WB_B" = 50 }`.
    On each new connection, after its three set-up switches, the adapter sends one
    `newNumberVector` for each property the table names, the elements named at their
    values and every other element at the device's reported value, and awaits the
    device's answer; every control the adapter is not asked to change, by the table or
    by an `apply`, goes back at the device's latest value (US5-S1, US5-S5). No spec
    records any control with a frame. Open, for a name the device does not define:
    Recommended: the connection fails with `the camera has no control {name}`, so a
    misspelt key is seen at the first call and never silently skipped. Instead: skip it
    and connect. (The review's recommendation, a bring-up step for the observer with an
    INDI client and the recording of every control handed to 034, and its alternative,
    every control carried with each frame, are closed by the amendment.)

## Owner approvals

Flipping this spec to `ready` approves the settled answer to every owner decision above,
and the following. Each is named in the FR or success criterion that makes it.

1. **An additive scope-link service, `ScopeNodeImaging`**, with five calls and its
   messages (FR-013 to FR-019), the frame header's `roi` among its fields (FR-016). No
   existing message, field or call changes. The scope node must be rebuilt before the
   console sees an imaging camera.
2. **Two new module trees**: `crates/scope-node/src/imaging/` and
   `crates/simulator/src/imaging.rs` (FR-001, FR-006).
3. **Two dependencies of `scope-node`**: `quick-xml = { version = "=0.42.0",
   default-features = false, features = ["async-tokio"] }` (MIT) and
   `base64 = "=0.23.1"` (MIT OR Apache-2.0), with the `Cargo.lock` change they bring
   (FR-020 and FR-028, owner decision 4). `deny.toml` allows both licences. No other
   manifest changes.
4. **The scope node's `[imaging]` config table**: `adapter`, `address`, `device` and
   `neutral`, the table of controls the INDI adapter sets on connecting (FR-020, owner
   decision 13), and `save_dir` (FR-034).
5. **Two flags of `scope-node`**: `--probe-camera` and `--probe-exposure-ms` (FR-037).
6. **No story writes `GLOSSARY.md` or the plan document.** The terms this spec uses
   (imaging frame, and the changed imaging camera, camera port and camera adapter) are
   written by the plan document's own change to `GLOSSARY.md`, which lands with this
   trio. That change also carries one entry it lacked when this trio was drafted
   (plan.md § Prerequisites 6): a new **Purpose label**, worded as the audit's edit of
   `GLOSSARY.md` words it, which adds `probe` (this spec's US8) and `run` (037): "The
   short word an exposure is asked for with, saying what it is for: `frame` and `probe`,
   and later `focus`, `dark`, `flat`, `sub` and `run`. _Avoid_: frame type (that is
   INDI's), kind". The plan document's § Interfaces shared across the specs already
   carries `save` and `save_dir` as this trio has them (plan.md § Prerequisites 9).
7. **One document section**: `## The imaging camera` in `crates/scope-node/README.md`
   (FR-026, FR-032, FR-036, FR-038).
8. **Two operator-staged fixture files**, written by no story:
   `fixtures/scope-node/indi/ccd-simulator.script` and
   `fixtures/scope-node/indi/ccd-simulator.expect.json` (plan.md § Prerequisites 4).
9. **Two widened visibilities and the simulator's new public names**:
   `NodeControls::ready` becomes `pub(crate)`, `SplitMix64::uniform` becomes `pub`, and
   `crates/simulator` exports `SimImagingCamera`, `ImagingOptics`, `ImagingTruth` and
   `SimStar` (FR-006, FR-014).
10. **The cards of the scope node's own copy**, which no later spec can change in files
    already written: those of FR-035, `XORGSUBF` and `YORGSUBF` among them, and no
    `SET-TEMP` or `SERIALNO` (owner decisions 9 and 12).

No existing test or gate file is edited by any story of this trio (SC-002).

## User Scenarios & Testing *(mandatory)*

Every scenario shares these definitions unless it says otherwise.

- **The gate command.** Every Rust scenario runs in
  `cargo nextest run --workspace --all-targets --all-features --locked`, the `test`
  gate's first command, each test in its own process.
- **Paused time.** A Rust test of the scope node or the simulator runs under
  `#[tokio::test(start_paused = true)]`, so every wait is in virtual time and an elapsed
  time can be asserted exactly. A test that starts a built binary is a plain `#[test]`
  and waits in real time.
- **The test clock** is a `NodeClock` (`crates/scope-node/src/clock.rs:3`) of the test's
  own that reads 1000.0 plus the tokio time elapsed since it was built.
- **The stand-in camera** is `SyntheticImagingCamera::new(clock)` on the test clock
  (plan.md § The stand-in camera): 64 × 48, 16 bits, RGGB, gain 100 and offset 10 in
  force. Its **sample rule**, with `binning` b, region of interest origin (x0, y0) and
  offset o in force: the sample at column c and row r of a frame is
  `256 + 16·o + ((x0/b + c + 3·(y0/b + r)) mod 251)`. With the settings it starts with,
  the sample at column 5, row 2 is 427, the bytes `AB 01`.
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
- **A position in a frame.** The sample at column c, row r covers the square from
  (c, r) to (c + 1, r + 1), so its centre is at (c + 0.5, r + 0.5). **The centroid** of a
  star is the mean of those centres weighted by (sample − background) over the 25 × 25
  samples about the brightest sample, and **its second moments** are the weighted
  covariance of the same samples. Background is the value the scenario gives, and every
  scenario that uses a centroid or second moments gives one.
- **The node rig** is `NodeService::new` over a `SyntheticCamera` on the test clock with
  the token `test-token` and default settings (`crates/scope-node/src/service.rs:114`),
  given `.with_imaging(the stand-in camera)`, and served with `routes()` over one
  `scope_link::memory::channel_pair` for each connection
  (`crates/scope-link/src/memory.rs:25`, `crates/scope-link/src/memory.rs:70`). A test
  calls it through a `ScopeNodeImagingClient` with the bearer token `test-token`.
- **Scratch folders.** A test's files live under the directory cargo names in
  `CARGO_TARGET_TMPDIR`, and no test builds a path from `CARGO_MANIFEST_DIR` and
  `../../target`: that is 028's rule (`specs/028-finder-tests-do-less/spec.md:413`).
  The scope node's tests have no shared helper for it: each new scope-node test file
  that writes a file holds a private guard of its own, a struct over a path under
  `CARGO_TARGET_TMPDIR` named from the test, the process id and a counter, whose `Drop`
  removes the file or folder, as `crates/scope-node/tests/cli.rs:8` does. Wherever a
  scenario needs a folder that does not exist yet (D), it is a child path of the
  scratch folder.
- **Reading a FITS file in a test.** A test reads a saved file with code of its own, never
  through the scope node's FITS code: 80-byte cards up to `END`, the header padded to
  2880 bytes, then big-endian samples, each 16-bit sample being the stored signed value
  plus `BZERO`.
- **An INDI script** is text in the format of plan.md § The script format, played by the
  tests' stand-in for the INDI server over an in-memory stream. It fails the test if the
  adapter sends anything the script does not expect next, if an element the script
  expects at a stated time comes at another, or if expectations are left at the end.
  **The colour-camera script** and **the mono-camera script** are the two
  hand-written scripts plan.md § The two worked scripts gives in full, written once, as
  `colour()` and `mono()` in `crates/scope-node/tests/support/mod.rs`, so that every
  INDI test file shares them; their values are a worked example written from INDI's
  documents, not a claim about any camera. The mono-camera script's device carries the
  name of INDI's camera simulator, `CCD Simulator`, but not its values: the saved
  capture is that simulator.
- **The camera of US4-S1** is an `IndiCamera` built as US4-S1 builds it, over the
  colour-camera script followed by the scenario's own lines, on which `describe()` has
  already returned before the scenario's When. Its connection is therefore made and
  settled, and every time a scenario states is counted from the call it names, with no
  part of connecting in it. A scenario that states a time makes its first call 1 s
  after `describe()` returned, so "in the instant of the call" is `@ expect after 1000`
  after the colour-camera script's last line, whose stamp is the moment `describe()`
  returned. **The mono camera** is the same for US4-S2's adapter over the mono-camera
  script. A scenario that times the adapter's first call says so.
- **Literals.** A test that builds a scope-link message or an `ImagingFrame` ends its
  literal with `..Default::default()`, because later stories and specs add fields.
- **Oracles.** Each scenario names its oracle: the sample rule, a worked example given
  here, the simulator's truth, INDI's documents through a script, or the saved capture.

### User Story 1 - The imaging side of the camera port (Priority: P1)

As the builder of every later story, I have one hardware-neutral way to describe an
imaging camera, change its settings, take one exposure on request and abort it, and a
small stand-in camera that honours it, so that the service, the adapters and the probe
are written against one port.

**Why this priority**: Every other story of this spec, and of 031 to 037, calls this
port.

**Independent Test**: `crates/scope-node/tests/imaging_port.rs`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p scope-node --locked --test imaging_port`.

**Acceptance Scenarios**:

1. **Given** the stand-in camera, **When** a test calls `describe()` and `status()`,
   **Then** a committed test asserts, against plan.md § The stand-in camera, that the
   description is exactly: model `synthetic imaging camera`, no serial, 64 × 48, pixel
   4.78 µm, 16 bits, `BayerPattern::Rggb`, exposure 1 to 60000 ms, gain 0 to 300, offset
   0 to 100, binnings `[1, 2]`, a cooler; and that the status is 20.0 °C, cooler power
   0.0, no set temperature, not exposing.
2. **Given** the stand-in camera, **When** a test calls `apply` with an empty
   `ImagingSettings`, then with gain 200 and binning 2, then with an empty one again,
   **Then** a committed test asserts that the first answer is gain 100, offset 10,
   binning 1, the region (0, 0, 64, 48), cooler off and no set temperature; that the
   second and third are equal and differ from the first only in gain 200 and binning 2;
   that `apply` with the cooler on and −10 °C then answers the second answer with
   `cooler_on` true and set temperature −10.0, so gain 200 and binning 2 stay in force;
   and that after it `status()` reads −10.0 °C, power 75.0 and set temperature −10.0.
3. **Given** the stand-in camera, **When** a test calls `expose` for 500 ms with the
   purpose `frame`, **Then** a committed test asserts that the call returns after exactly
   500 ms of virtual time; that the frame is 64 × 48, 16 bits, RGGB, binning 1, gain
   100.0, offset 10.0, 20.0 °C and 6144 bytes, with `roi` the region in force,
   (0, 0, 64, 48); that `start_s` is the test clock's
   reading at the call and `end_s` is `start_s + 0.5`, each exactly; and that every
   sample equals the sample rule (oracle: the rule, computed by the test).
4. **Given** the stand-in camera with binning 2 and the region (8, 4, 16, 8) applied,
   **When** a test takes a frame, **Then** a committed test asserts 8 × 4 samples,
   binning 2, `roi` (8, 4, 16, 8) in unbinned sensor pixels, and every sample equal to
   the sample rule, the first being 426.
5. **Given** the stand-in camera, **Then** a committed test asserts that each of these
   `apply` calls answers `ImagingError::Invalid` with exactly this text, and that an
   empty `apply` afterwards answers the settings as they were:
   - gain 301: `gain 301 is outside the camera's range 0–300`;
   - offset −1: `offset -1 is outside the camera's range 0–100`;
   - binning 3: `binning 3 is not one of the camera's binnings 1, 2`;
   - the region (2, 0, 64, 48) with binning 1:
     `roi must lie inside the 64 × 48 sensor with x and y multiples of 2`;
   - the region (2, 2, 16, 8), which is accepted at binning 1, then binning 2 alone: the
     second answers `roi must lie inside the 64 × 48 sensor with x and y multiples of 4`;
   - the region (0, 0, 80, 48): `roi must lie inside the 64 × 48 sensor with x and y
     multiples of 2`;
   - the cooler on, on a camera of the test's own description without one, through
     `check_settings`: `this camera has no cooler`;
   - set temperature 60: `set_temp_c must be between -50 and 50`.
6. **Given** the stand-in camera, **Then** a committed test asserts that `expose` answers
   `Invalid` with `exposure_ms 0.5 is outside the camera's range 1–60000 ms` for 0.5 ms,
   the same form for 60001 and for a NaN, and
   `purpose must be 1 to 16 lower-case letters` for the purposes `` (empty), `Dark` and
   a 17-letter one, each without waiting.
7. **Given** the stand-in camera with an `expose` of 10 s started in a task of its own,
   **When** 1 s later a test reads `status()`, calls `apply` with an empty
   `ImagingSettings` and then with gain 150, starts a second `expose`, calls `abort()`
   and reads `status()` again, **Then** a committed test asserts that the first status is
   exposing; that the empty `apply` answers the settings in force and the one with gain
   150 answers `ImagingError::Busy`, an empty `apply` after it still answering gain
   100; that the second `expose` answers `ImagingError::Busy` at once; that the
   first `expose` answers `ImagingError::Aborted` at that same virtual instant; that the
   second status is not exposing; that `abort()` with nothing exposing answers `Ok`; and
   that a new 100 ms `expose` then succeeds.

---

### User Story 2 - A simulated imaging camera along the main telescope's axis (Priority: P1)

As a test of any later imaging story, I get 16-bit Bayer frames from a simulated camera
that looks where the main telescope looks: seeded stars, wider when out of focus, smeared
when the sky or the mount moves during the exposure, on a sky background with noise, and
I can read the truth of where each frame pointed.

**Why this priority**: The solving, focus, stacking and tracking specs are tested against
this camera's frames and truth. The finder simulator's stars are too sparse for a
0.75° field.

**Independent Test**: `crates/simulator/tests/imaging.rs`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p simulator --locked --test imaging`.

**Acceptance Scenarios**:

1. **Given** the sim rig, **Then** a committed test asserts, against plan.md § The
   simulated imaging camera, that `describe()` is exactly: model
   `simulated imaging camera`, serial `SIM-0001`, 1236 × 822, pixel 19.12 µm, 16 bits,
   RGGB, exposure 1 to 3600000 ms, gain 0 to 300, offset 0 to 100, binnings `[1, 2]`, a
   cooler; that an empty `apply` answers gain 120, offset 10, binning 1 and the region
   (0, 0, 1236, 822); and that `ImagingOptics::default()` is 1236 × 822, 0.75°, angle
   17.0°, focus error 0.0 and pixel 19.12 µm.
2. **Given** the sim rig, **Then** a committed test asserts of
   `sim.imaging_truth_at(t)` at t = 1768528800:
   - its scale is 2.18450 ± 0.0001 ″/px (worked example: 2·tan(0.375°)/1236 radians);
   - its RA and Dec equal `sim.directions_at(t).eyepiece`'s within 10⁻⁹ degrees (oracle:
     the simulator's existing truth, `crates/simulator/src/optics.rs:120`);
   - with the finder optics' eyepiece pixel set to the finder frame's centre, (728.0,
     544.0), and the imaging angle 0, its `up_east_of_north_deg` is within 0.01° of the
     position angle, east of north, from P0 to P1, where P0 and P1 are the J2000
     directions of altitude 50° and 50.01° at azimuth 120°, each made by the test with
     `sky::horizontal_to_date` and `sky::to_j2000`
     (`crates/simulator/src/sky.rs:46`, `crates/simulator/src/sky.rs:81`) and compared
     with the formula `atan2(sin Δα·cos δ1, cos δ0·sin δ1 − sin δ0·cos δ1·cos Δα)`;
   - with the imaging angle 17 it is that value plus 17°, within 0.01°, modulo 360.
3. **Given** the sim rig's camera built `.with_noise(false).with_stars(vec![A, B])`,
   where both stars have magnitude 5 and colour `[1, 1, 1]`, A is at the truth's RA and
   Dec for the middle of the exposure and B is 300″ north of A at the same RA, with gain
   0 applied, **When** a test takes a 20 ms frame, **Then** a committed test asserts,
   with background 160 and θ and s the truth's `up_east_of_north_deg` and scale at that
   instant: the frame is 1236 × 822; A's centroid is within 0.3 px of (618.0, 411.0);
   and B's centroid less A's is within 0.3 px of (d·sin θ, −d·cos θ), where d = 300/s
   (oracle: the simulator's truth, checked independently in US2-S2).
4. **Given** the sim rig's camera with no stars (`with_stars(vec![])`) and gain 0,
   **When** a test takes three 1000 ms frames, the first two from two cameras built
   alike and the third as the second frame of one of them, **Then** a committed test
   asserts that the first two are identical byte for byte and the third differs; and
   that over all samples of one frame the mean is 165.0 ± 0.05 and the standard
   deviation is between 1.31 and 1.45 (worked example: offset 10 gives 160; 20
   electrons of sky at 0.25 a count give 5; the noise is √(20 + 3²) electrons, 1.346
   counts, and 1.377 with rounding).
5. **Given** the sim rig's camera with noise off, gain 0 and one white star of magnitude
   5 at the truth's centre for mid-exposure, **When** a test takes a 20 ms frame with
   the optics' focus error 0 and another with 3.0, **Then** a committed test asserts,
   with background 160, that the star's second moments give a width, the square root
   of half their trace, of 1.20 ± 0.05 px and 3.23 ± 0.12 px (worked example:
   √(1.2² + 3.0²) = 3.231; the background is 160 counts of offset and 0.1 of sky).
6. **Given** the sim rig's camera with noise off, gain 0 and one white star of magnitude
   9 at the truth's centre for mid-exposure, **When** a test takes a 2 s frame with no
   mount, **Then** a committed test asserts, with background 170 and over the 41 × 41
   samples about the brightest, that the larger eigenvalue of the star's second moments
   less the smaller is within 8% of L²/12, where L = 15.041·cos δ·2/s px, δ is the
   truth's Dec and s its scale (worked example: the sky turns 15.041″ in a second of
   time at the equator; near Dec 15°, where this rig points, that is about 14.5″, so L
   is about 13.3 px, and the test computes L from the truth; the background is 160 of
   offset and 40 electrons of sky at 0.25 counts each). **Given** instead a
   `SimMount::new(sim.clock().clone())` connected with `sim.connect_mount`, then
   `sim.push_to(50.0, 120.0)`, and `Mount::rate` of `Axis::Azimuth` at 0.03 °/s, with
   60 s waited, so that the take-up is done, gain 0 as before, and one
   white star of magnitude 8 at the truth's centre then, **When** a test reads
   `sim.imaging_truth()`, takes a 500 ms frame and reads the truth again, **Then** a
   committed test asserts, with background 163 (160 of offset and 2.5 of sky, rounded
   half away from zero), over the 41 × 41 samples about the brightest, that the star's
   two true positions, the frame's centre and its
   place in the second truth's frame, are more than 10 px apart (the sky alone moves it
   3.3); that its centroid is within 0.2 px of their midpoint (a smear drawn at the
   points k/n, not plan.md's (k + 0.5)/n, is 0.47 px off); that the difference of the
   eigenvalues is within 10% of the squared distance between them over 12; and that
   the long axis lies within 5° of the line between them.
7. **Given** the sim rig's camera with noise off, gain 0 and one star of magnitude 5 and
   colour `[1, 0, 0]` at the truth's centre for mid-exposure, **When** a test takes a
   20 ms frame at binning 1 and another at binning 2, **Then** a committed test asserts
   that the second is 618 × 411 with `binning` 2, RGGB and `roi` (0, 0, 1236, 822); that
   in each frame the
   brightest sample is above 1000 and at an even column and an even row, within 2 px of
   the star's place, (618, 411) and (309, 205.5); that every sample at an odd
   column or an odd row within 9 samples of it is exactly 160; and that every sample of
   the binned frame's last row, row 410, is exactly 160 (worked example: 822 is not a
   multiple of 4, so sensor row 822 does not exist and row 410 is the mean of the two
   samples of sensor row 820 alone, 160.1 each; a missing sample counted as zero would
   give 80). **Given** instead the same star 409.5 px below the frame's centre, on
   sensor row 820 (the truth's centre moved 409.5·s arcseconds toward position angle
   θ + 180°, θ and s the truth's `up_east_of_north_deg` and scale), **Then** with A the
   sum of (sample − 160) over the even columns of row 820 of the binning-1 frame and B
   the same sum over the even columns of row 410 of the binning-2 frame, A is above
   10,000, B is within 5% of A/2, and every sample at an odd column of row 410 is
   exactly 160 (worked example: each red sample of sensor row 820 enters exactly one
   mean, and each mean is of two). **Given** a white star at the truth's centre instead
   and the region (400, 200, 400, 400) at binning 1, **Then**, with background 160, the
   frame is 400 × 400 with `roi` (400, 200, 400, 400) and the star's centroid is within
   0.3 px of (218.0, 211.0).
8. **Given** `deep_field(FIELD_SEED, 83.8, -5.4, 2.0)`, **Then** a committed test asserts
   that it is the same list when asked twice; that it holds between 3780 and 4262 stars
   (worked example: 320 a square degree over 12.566 square degrees, 4021), of which
   between 768 and 977 are brighter than magnitude 14; that every magnitude is from 8
   to 16 and the list is in rising magnitude; that no two of its stars have the same
   magnitude (were the cell stream not seeded by the band, neighbouring bands would
   share them); that every star is within 2.0° of the centre; and that every star of
   `deep_field(FIELD_SEED, 83.8, -5.4, 0.3)` within 0.3° of (84.0, −5.4) is also in
   `deep_field(FIELD_SEED, 84.0, -5.4, 0.3)` with identical
   RA, Dec, magnitude and colour.
9. **Given** the sim rig's default camera with noise off, **When** a test takes a 1 s
   frame, **Then** a committed test asserts that the brightest star of
   `deep_field(FIELD_SEED, ra, dec, 0.2)` about the truth's centre for mid-exposure has,
   within 10 samples of its place by the truth, a sample at least 300 above the
   background of 180. **And** a committed test asserts the port's behaviour of US1-S5 to
   US1-S7 on this camera with its own ranges: a second `expose` is `Busy`, an `apply`
   with gain 150 made during an exposure is `Busy` while an empty one answers the
   settings in force, `abort()` ends a 10 s exposure with `Aborted` at once, gain 301
   and binning 3 are `Invalid` with the texts of US1-S5, and a frame's `start_s` is the
   simulator clock's `node_now()` at the call with `end_s` later by the exposure.

---

### User Story 3 - The imaging service on the scope link (Priority: P1)

As the console, I can ask the scope node to describe its imaging camera, report its
status, change its settings, take one exposure and abort it, and I receive the frame as
a header and chunks on a connection that carries nothing else.

**Why this priority**: It is the wire every imaging frame crosses, and the reason the
heartbeat stays safe while frames move.

**Independent Test**: `crates/scope-node/tests/imaging_service.rs` and
`crates/simulator/tests/imaging_link.rs`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs both; during
development, `cargo nextest run -p scope-node --locked --test imaging_service` and
`cargo nextest run -p simulator --locked --test imaging_link`.

**Acceptance Scenarios**:

1. **Given** the node rig, **Then** a committed test asserts that `DescribeImaging`
   answers model `synthetic imaging camera`, no `serial`, width 64, height 48, pixel
   4.78, bits 16, bayer `RGGB`, `exposure_ms` 1 to 60000, `gain` 0 to 300, `offset` 0 to
   100, binnings `[1, 2]` and `cooler` true; that `ImagingStatus` answers 20.0, 0.0, no
   set temperature and `exposing` false; that `ConfigureImaging` with an empty message
   answers gain 100, offset 10, binning 1 and the region (0, 0, 64, 48) with `cooler_on`
   false; that with gain 200 it answers gain 200 and the rest unchanged; and that with
   gain 301 it answers `InvalidArgument` and
   `gain 301 is outside the camera's range 0–300`.
2. **Given** the node rig, **When** a test calls `Expose` with 500 ms and `frame`, twice,
   **Then** a committed test asserts of the first stream: its first message is a header
   with id 1, 64 × 48, bits 16, bayer `RGGB`, exposure 500.0, gain 100.0, offset 10.0,
   binning 1, 20.0 °C, purpose `frame`, `byte_count` 6144, `roi` x 0, y 0, width 64 and
   height 48, `start_s` the test clock's reading at the call and `end_s` 0.5 later;
   every later message is a chunk of at most 262,144 bytes; the chunks together are
   6144 bytes equal to the sample rule; and the stream then ends. The second stream's
   header has id 2. **Given** the node rig with `ConfigureImaging` of binning 2 and the
   region (8, 4, 16, 8) sent first, **Then** the next header's `roi` is x 8, y 4, width
   16 and height 8, its `binning` 2 and its size 8 × 4 (oracle: the stand-in's frame,
   US1-S4).
3. **Given** the node rig and a client with the token `wrong-token`, **Then** a committed
   test asserts that each of the five calls answers `Unauthenticated`.
4. **Given** the node rig built without `with_imaging`, **Then** a committed test asserts
   that each of the five calls answers `FailedPrecondition` with `no imaging camera`,
   `Expose` as the first item of its stream. **Given** the same service served by
   `into_server()` alone, as a node built before this spec, **Then** `DescribeImaging`
   answers `Unimplemented`. **Given** the node rig with `controls().set_unresponsive(true)`,
   **Then** `DescribeImaging` does not answer within 1 s, as the node's `Describe` does
   not.
5. **Given** the node rig with an `Expose` of 10 s open, **When** 1 s later a test calls
   `ImagingStatus`, a second `Expose`, then `AbortExposure`, **Then** a committed test
   asserts `exposing` true; that the second stream's first item is `ResourceExhausted`
   with `an exposure is already in progress`; that the first stream's first item is
   `Aborted` with `the exposure was aborted`; that `ImagingStatus` then reads `exposing`
   false; and that `AbortExposure` with nothing exposing answers OK.
6. **Given** the node rig, **Then** a committed test asserts that `Expose` with 0.5 ms
   returns a stream whose first and only item is `InvalidArgument` with
   `exposure_ms 0.5 is outside the camera's range 1–60000 ms`, and with the purpose
   `Dark`, in the same way, `InvalidArgument` with
   `purpose must be 1 to 16 lower-case letters`.
   **Given** a node whose imaging camera is one of the test's own that answers `expose`
   with `ImagingError::Failed("the camera fell over")`, **Then** the stream's first item
   is `Internal` with `the camera fell over`. **Given** the node rig and `Expose` with
   `save` true, **Then** the header's `node_file` is empty, its `node_save_error` is
   `this scope node has no save_dir`, and the whole frame follows.
7. **Given** the node rig, **When** a test opens an `Expose` of 2 s, drops the stream
   after 1 s, waits 2 s more and calls `Expose` with 100 ms, **Then** a committed test
   asserts that `ImagingStatus` read 0.5 s after the drop still says `exposing` true,
   and that the new stream's header has id 2 (owner decision 11).
8. **Given** a `NodeService` on the sim rig's clock with a `SimMount` attached and an
   imaging camera of the test's own that answers a 100 ms exposure with a 4944 × 3284,
   16-bit frame whose i-th sample is `i mod 65521`, served with `routes()` over two
   connections; the mount's azimuth rate set to 0.5°/s on the first connection; a task
   sending `Heartbeat` on the first connection every 250 ms, as
   `crates/finder/src/link.rs:547` paces it, and keeping each reply; **When** a test
   calls `Expose` on the second connection with `max_bytes_per_s` 4,194,304 and reads
   the whole stream, and then again with 0, **Then** a committed test asserts that:
   - each stream's header has `byte_count` 32,472,192 and is followed by exactly 124
     chunks, the first 123 of 262,144 bytes and the last of 228,480, whose bytes are the
     camera's samples;
   - the limited stream's last chunk arrives between 7.68 s and 7.70 s of virtual time
     after its header (worked example: the last chunk leaves when 123 × 262,144 bytes
     have gone, 7.6875 s after the first, which tokio's timer rounds up to 7.688 s;
     counting the chunk about to leave would give 7.742 s), and the unlimited stream's
     in the same virtual instant as its header;
   - at least 30 heartbeats were answered between the limited stream's header and its
     last chunk, every reply's `t1` is 0.250 s ± 1 ms after the one before, and every
     reply arrived in the virtual instant it was sent (in paused time over memory links
     this proves the pacing and that a paced stream never holds a heartbeat; it cannot
     show bandwidth sharing, and the second connection is a design rule, which 031's
     link manager keeps);
   - afterwards `AxisState` for azimuth reads `moving` true and `watchdog_stops` 0.
9. **Given** a `SimNode::new` over the sim rig's simulator and a channel from
   `memory_channel()` (`crates/simulator/src/node.rs:62`), **Then** a committed test
   asserts that `DescribeImaging` answers model `simulated imaging camera`, 1236 × 822
   and bayer `RGGB`; that an `Expose` of 100 ms delivers a header with `byte_count`
   2,031,984 and that many bytes in 8 chunks; and that `Describe` on the same channel
   still answers the finder camera, model `simulator`, 1456 × 1088. **Given** instead a
   `SimNode::from_cameras` over the same simulator, with a `SimCamera::new` of it and an
   imaging camera built `.with_stars(vec![]).with_noise(false)`, **Then** an `Expose` of
   100 ms delivers 2,031,984 bytes whose every sample is exactly 162, so the caller's
   camera is the one attached (worked example: with no star and no noise a sample is
   160 of offset and 2 electrons of sky at the starting gain of 120, which is
   0.25 × 10^0.6 counts an electron: 161.99).

---

### User Story 4 - An INDI camera connected and described (Priority: P1)

As the observer setting up the scope node, I point the node at an INDI server, and it
connects, chooses the camera that server offers and describes it, with no code of ours
for the camera's model, and it connects again by itself after a lost connection.

**Why this priority**: The real camera is reached only this way (ADR-0002), and every
later INDI story stands on this connection and on the stand-in server it brings.

**Independent Test**: `crates/scope-node/tests/indi.rs` with the stand-in server and the
two worked scripts of `crates/scope-node/tests/support/mod.rs`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p scope-node --locked --test indi`.

**Acceptance Scenarios**:

1. **Given** an `IndiCamera` named for the device `ZWO CCD ASI071MC Pro` over the
   colour-camera script, **When** a test calls `describe()` and `status()`, **Then** a
   committed test asserts that the adapter sent exactly, in order, `getProperties` with
   `version="1.7"` (which the script's pair `@version=1.7` checks), `newSwitchVector`
   `CONNECTION` with `CONNECT` `On`, and `enableBLOB`
   with the text `Also`, each naming the device where the script says; that
   `enableBLOB` came exactly 200 ms after the script sent its last definition, the
   adapter's settle time, which the script's own timed expectation checks; and that the
   description is model `ZWO CCD ASI071MC Pro`, serial `0a1b2c3d4e5f6071`, 4944 × 3284,
   pixel 4.78, 16 bits, RGGB, exposure 0.064 to 2000000 ms, gain 0 to 240, offset 0 to
   100, binnings `[1, 2, 3, 4]`, a cooler; and the status 12.5 °C, power 0.0, no set
   temperature, not exposing (oracle: INDI's documents, through the script).
2. **Given** an `IndiCamera` with no device named over the mono-camera script, whose
   first device is a mount and whose second is a camera that is already connected,
   **Then** a committed test asserts that the adapter sent `getProperties`, then
   `enableBLOB` `Also`, then `newSwitchVector` `CCD_COMPRESSION` with `INDI_DISABLED`
   `On`, then `newSwitchVector` `UPLOAD_MODE` with `UPLOAD_CLIENT` `On`, every one after
   the first naming `CCD Simulator`, and no `CONNECTION`; and that the description is
   model `CCD Simulator`, no serial, 1280 × 1024, pixel 5.2, 16 bits, mono, exposure 10
   to 3600000 ms, gain 0 to 300, offset 0 to 6000, binnings `[1]`, no cooler.
3. **Given** an `IndiCamera` whose connector answers an I/O error, **Then** a committed
   test asserts that `describe()` answers `Failed` with a text beginning
   `cannot reach the INDI server at 127.0.0.1:7624: `. **Given** the device
   `No Such Camera` over the colour-camera script's first part, **Then** `describe()`,
   the adapter's first call, answers `Failed` with
   `the INDI server at 127.0.0.1:7624 has no device No Such Camera` exactly 5 s after it
   was made. **Given** no device named and a script that defines only a mount,
   **Then** `describe()`, again the first call, answers `Failed` with
   `the INDI server at 127.0.0.1:7624 offers no camera` exactly 5 s after it was made.
   **Given** the colour-camera script with `CCD_BITSPERPIXEL` 12 in place of 16,
   **Then** `describe()` answers `Failed` with `unsupported sample size 12 bits`.
   **Given** a script that closes the connection after the description, and a second
   script for the next connection, **Then** `describe()` succeeds, the next `status()`
   answers from the second connection, and the adapter sent `getProperties` twice.

---

### User Story 5 - An INDI camera configured (Priority: P1)

As the observer setting up the scope node, I name an INDI server and the camera's
neutral controls in the node's config, and the node applies my settings to that camera,
sets those controls on each connection and attaches the camera to its service.

**Why this priority**: Without its settings and its config table the real camera
cannot be used from the console.

**Independent Test**: `crates/scope-node/tests/indi_settings.rs`, with the stand-in
server and the two worked scripts of US4, and
`crates/scope-node/tests/imaging_config.rs`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs both; during
development, `cargo nextest run -p scope-node --locked --test indi_settings` and
`cargo nextest run -p scope-node --locked --test imaging_config`.

**Acceptance Scenarios**:

1. **Given** the camera of US4-S1, **When** a test applies gain 120 and binning 2, then
   the region (1000, 800, 1930, 1200), then the cooler on with −10 °C, **Then** a
   committed test asserts that the three applies sent these vectors, in this order:
   `newNumberVector` `CCD_BINNING` with `HOR_BIN` 2 and `VER_BIN` 2, then
   `newNumberVector` `CCD_CONTROLS` with `Gain` 120, `Offset` 20, `WB_R` 52 and `WB_B` 95
   (the elements it was not asked to change go back at the values the device last
   reported: owner decision 13); then `newNumberVector` `CCD_FRAME` with `X` 1000,
   `Y` 800, `WIDTH` 1930 and `HEIGHT` 1200; then `newSwitchVector` `CCD_COOLER` with
   `COOLER_ON` `On`, then `newNumberVector` `CCD_TEMPERATURE` with
   `CCD_TEMPERATURE_VALUE` −10. It asserts the answers: after the first, gain 120.0,
   offset 20.0, binning 2; after the second, the region (1000, 800, 1928, 1200), the
   width the script's camera answered; after the third, `cooler_on` true and set
   temperature −10.0. After the script's later `setNumberVector`s, `status()` reads
   5.0 °C, power 80.0 and set temperature −10.0.
2. **Given** the camera of US4-S1, **Then** a committed test asserts that gain 300
   answers `Invalid` with `gain 300 is outside the camera's range 0–240` and sends
   nothing; and that binning 3, which the script's camera answers with the state `Alert`
   and the message `Binning 3x3 is not supported.`, answers `Failed` with
   `the camera refused CCD_BINNING: Binning 3x3 is not supported.` **Given** the camera
   of US4-S1 and a script that never answers `CCD_BINNING`, **Then** `apply` with
   binning 2 answers `Failed` with `the camera did not answer CCD_BINNING within 5 s`
   exactly 5 s after that call.
3. **Given** `Config::parse`, **Then** a committed test asserts that:
   - a config with no `[imaging]` table has `imaging` `None`;
   - `[imaging]` with `adapter = 'indi'` alone has the address 127.0.0.1 with port 7624
     and no device; with `address = '127.0.0.1:7700'` and `device = 'CCD Simulator'` it has
     those;
   - `[imaging]` with `adapter = 'simulated'` parses, and with `address` or `device`
     beside it is refused with an error containing `imaging.address` or
     `imaging.device`;
   - an empty `[imaging]` table is refused with an error containing
     `missing imaging.adapter`; `adapter = 'usb'`, `device = ''` and an unknown key
     `exposure = 5` are each refused with an error naming the key, and no error
     contains `test-token`;
   - `[imaging]` with `adapter = 'indi'` and no `neutral` has an empty `neutral`; with
     `neutral = { "CCD_CONTROLS.WB_R" = 50, "CCD_CONTROLS.WB_B" = 50 }` its `neutral`
     holds those two at 50.0 and `neutral_controls()` answers
     `[("CCD_CONTROLS", "WB_B", 50.0), ("CCD_CONTROLS", "WB_R", 50.0)]`, in the table's
     sorted order, each key split at its first `.`, so that
     `neutral = { "CCD_CONTROLS.WB.R" = 1 }` answers `[("CCD_CONTROLS", "WB.R", 1.0)]`;
   - that `neutral` beside `adapter = 'simulated'` is refused with exactly
     `imaging.neutral applies only to the indi adapter`; `neutral = { "WB_R" = 50 }`
     with exactly `imaging.neutral key WB_R must be PROPERTY.ELEMENT`; and
     `neutral = { "CCD_CONTROLS.WB_R" = nan }` with exactly
     `imaging.neutral CCD_CONTROLS.WB_R must be a finite number`.
4. **Given** the built `scope-node` run with `--check-config` on a config holding
   `[imaging]`, `adapter = 'indi'`, `device = 'CCD Simulator'` and
   `neutral = { "CCD_CONTROLS.WB_R" = 50, "CCD_CONTROLS.WB_B" = 50 }`, as `run` in
   `crates/scope-node/tests/cli.rs:15` runs it, the config file kept as the shared
   setup's scratch rule says, **Then** a committed test asserts that
   its output contains `[imaging]`, `adapter = "indi"`, `address = "127.0.0.1:7624"`,
   `device = "CCD Simulator"` and
   `neutral = { "CCD_CONTROLS.WB_B" = 50, "CCD_CONTROLS.WB_R" = 50 }`; and, on a config
   with no such table, the line `# no imaging camera configured`.
5. **Given** an `IndiCamera` named for the device `ZWO CCD ASI071MC Pro` with the
   neutral controls `[("CCD_CONTROLS", "WB_B", 50.0), ("CCD_CONTROLS", "WB_R", 50.0)]`,
   as US5-S3's table gives them, over the colour-camera script followed by
   `@ expect newNumberVector CCD_CONTROLS Gain=90 Offset=20 WB_R=50 WB_B=50` and a
   `@ send` of `setNumberVector` `CCD_CONTROLS` in the state `Ok` with those four
   values, **When** a test calls `describe()` and then applies gain 120, **Then** a
   committed test asserts, through the script played to its end, that after
   `enableBLOB` the adapter sent exactly one more element before `describe()` answered,
   that `newNumberVector`, holding every element of `CCD_CONTROLS` in its defined order,
   the two named at 50 and `Gain` and `Offset` at the device's reported 90 and 20; that
   the `apply` then sent `CCD_CONTROLS` with `Gain` 120, `Offset` 20, `WB_R` 50 and
   `WB_B` 50 and answered gain 120.0; and that, with the first script ending in
   `@ close` after the `apply`'s answer and a second script, the colour-camera script
   with the same expectation and answer, for the next connection, the next `status()`
   connects again and the second script is played to its end too, so the controls are
   set on each new connection (oracle: INDI's documents and the worked table, through the
   script). **Given** instead the neutral control `("CCD_CONTROLS", "WB_G", 50.0)` over
   two copies of the colour-camera script, **Then** `describe()` answers `Failed` with
   `the camera has no control CCD_CONTROLS.WB_G`, a second `describe()` answers the
   same, and the adapter sent `getProperties` twice and nothing after either
   `enableBLOB`, so the failure failed the connection; with `("CCD_WB", "R", 50.0)`,
   `the camera has no control CCD_WB.R`.
   **Given** the two controls of the first case and a script that answers their
   `newNumberVector` with `CCD_CONTROLS` in the state `Alert` and the message
   `no such control`, **Then** `describe()` answers `Failed` with
   `the camera refused CCD_CONTROLS: no such control`.

---

### User Story 6 - Exposures through INDI (Priority: P1)

As the observer, I ask for an exposure and the scope node takes it through INDI, hands
back the camera's samples untouched with the times of the exposure, and when an exposure
fails or never arrives it tries once more and then says so.

**Why this priority**: Without it the real camera gives no frame; with a silent failure
a night's run would stall.

**Independent Test**: `crates/scope-node/tests/indi_expose.rs`, with the stand-in server
and the two worked scripts of US4 and the saved capture
`fixtures/scope-node/indi/ccd-simulator.script`. The `test`
gate's `cargo nextest run --workspace --all-targets --all-features --locked` runs it;
during development, `cargo nextest run -p scope-node --locked --test indi_expose`.

**Acceptance Scenarios**:

1. **Given** the camera of US4-S1 and a script that answers `CCD_EXPOSURE` with the state
   `Busy` after 200 ms, again with `Busy` and 0.2 s left 300 ms after that, as INDI's
   camera simulator does (the saved capture's three `Busy` answers), and, 500 ms after
   the first, a `setBLOBVector` `CCD1` whose `.fits`
   BLOB is the worked FITS frame of plan.md § Reading a FITS BLOB (4 × 2, `BITPIX` 16,
   `BZERO` 32768, `BAYERPAT` `RGGB`, `XBINNING` 1, `CCD-TEMP` −9.5, stored samples
   `8000 8001 8002 8003 0000 1C40 7FFE 7FFF`), then `CCD_EXPOSURE` `Ok`, **When** a test
   calls `expose` for 500 ms with the purpose `frame`, **Then** a committed test asserts
   that the adapter sent `newSwitchVector` `CCD_FRAME_TYPE` with `FRAME_LIGHT` `On`,
   then `newNumberVector` `CCD_EXPOSURE` with `CCD_EXPOSURE_VALUE` 0.5, both in the
   instant of the call; that the frame
   is 4 × 2, 16 bits, RGGB, binning 1, −9.5 °C, gain 90.0, offset 20.0, exposure 500.0,
   with `roi` (0, 0, 4944, 3284), the script's `CCD_FRAME` when the exposure was asked
   for (the BLOB of a worked example is smaller than the region; the frame's own size is
   what counts, FR-003);
   that its samples are, as numbers, 0, 1, 2, 3, 32768, 40000, 65534, 65535 and, as
   bytes, `00 00 01 00 02 00 03 00 00 80 40 9C FE FF FF FF`; that `start_s` is the test
   clock's reading 200 ms after the call and `end_s` is 0.5 later, each exactly; and
   that the call returned 700 ms after it was made.
2. **Given** the same camera, **Then** a committed test asserts that the purpose `dark`
   sends `FRAME_DARK` `On` and `flat` sends `FRAME_FLAT` `On`; and that over the
   mono-camera script, whose BLOB is 3 × 2 with `BITPIX` 8 and the bytes `00 01 02 FD FE
   FF`, the frame is 3 × 2, 8 bits, mono, 6 bytes, the same bytes, with gain 90.0 and
   offset 0.0 from `CCD_GAIN` and `CCD_OFFSET` and `roi` (0, 0, 1280, 1024) from its
   `CCD_FRAME`. **Given** the camera of US4-S1 with the region (1000, 800, 1930, 1200)
   applied as in US5-S1, whose answer sets `WIDTH` 1928, **Then** the next frame's `roi`
   is (1000, 800, 1928, 1200), the device's reported region. **Given** instead a script
   that sends `setNumberVector` `CCD_FRAME` with `X` 8 and `WIDTH` 64 between the
   exposure's `Busy` and its BLOB, as another client would, **Then** the frame's `roi` is
   still (0, 0, 4944, 3284). **And** a colour frame whose FITS has no `BAYERPAT` and
   `XBINNING` 2, on a camera whose `HOR_BIN` is 1, is RGGB (from `CCD_CFA`) with binning
   2 (from the header).
3. **Given** the camera of US4-S1 and a script that answers the first `CCD_EXPOSURE` at
   once with the state `Alert` and the message `Exposure failed after 3 attempts.`, and
   the second as US6-S1's script does, **When** a test calls `expose` for 500 ms,
   **Then** a committed test asserts, through the script's timed expectations, that the
   adapter sent exactly these elements: `newSwitchVector` `CCD_FRAME_TYPE` with
   `FRAME_LIGHT` `On` and `newNumberVector` `CCD_EXPOSURE` with `CCD_EXPOSURE_VALUE`
   0.5, in the instant of the call; `newSwitchVector` `CCD_ABORT_EXPOSURE` with `ABORT`
   `On`, 0 ms after the `Alert` was sent; `CCD_FRAME_TYPE` with `FRAME_LIGHT` `On`
   again, exactly 1000 ms after the abort; and `CCD_EXPOSURE` 0.5 again, 0 ms after
   that; and that `expose` answers the frame of US6-S1 1700 ms after it was called
   (worked example: 1000 of pause, 200 to `Busy`, 500 to the BLOB). **Given** a script
   that answers both with that `Alert`, **Then** the adapter sent the same five elements
   and then `CCD_ABORT_EXPOSURE` once more, 0 ms after the second `Alert`, and `expose`
   answers `Failed` with `the exposure failed twice: the camera reported a failed
   exposure: Exposure failed after 3 attempts.` 1000 ms after it was called.
4. **Given** the camera of US4-S1 and a script that answers each `CCD_EXPOSURE` 300 ms
   later with `Busy` and nothing more, **When** a test calls `expose` for 500 ms, **Then** a
   committed test asserts, through the script's timed expectations, that the adapter
   sent `CCD_FRAME_TYPE` with `FRAME_LIGHT` `On` and `CCD_EXPOSURE` 0.5 in the instant
   of the call; `CCD_ABORT_EXPOSURE` with `ABORT` `On` exactly 60,500 ms after the
   first `CCD_EXPOSURE` (60,200 ms after its `Busy`); `CCD_FRAME_TYPE` and
   `CCD_EXPOSURE` 0.5 again exactly 1000 ms after that abort; and `CCD_ABORT_EXPOSURE`
   exactly 60,500 ms after the second `CCD_EXPOSURE` (60,200 ms after its `Busy`), the
   wait being counted from the request and not from `Busy`;
   that `expose` answers `Failed` with `the exposure failed twice: no imaging frame
   arrived within 60 s of the exposure's end` exactly 122 s after the call (worked
   example: 60.5, 1 and 60.5); and that a `status()` made 30 s into the call answered
   in that instant with `exposing` true.
5. **Given** the camera of US4-S1 and a script that closes the connection 100 ms after
   `CCD_EXPOSURE`, with a second script that sets up and answers the frame of US6-S1,
   **Then** a committed test asserts that `expose` answers the frame and that the
   adapter sent `getProperties` twice. **Given** a second script that closes too,
   **Then** it answers `Failed` with
   `the exposure failed twice: the INDI server closed the connection`.
6. **Given** the camera of US4-S1 with an `expose` of 30 s in a task of its own and a
   script that answers its `CCD_EXPOSURE` at once with `Busy`, **When** 2 s after that
   call a test calls `expose` again, then `apply` with gain 100, then `apply` with an
   empty `ImagingSettings`, then `abort()`, **Then** a committed test asserts that the
   second `expose` and the `apply` with gain 100 each answer `Busy` and send nothing,
   and the empty `apply` answers the settings in force and sends nothing; that the
   adapter sent `CCD_FRAME_TYPE` with `FRAME_LIGHT` `On` and `CCD_EXPOSURE` 30 in the
   instant of the first call, and `newSwitchVector` `CCD_ABORT_EXPOSURE` with `ABORT`
   `On` exactly 2000 ms after the `Busy` was sent, which the script's timed expectation
   checks; that the first `expose` answers `Aborted` in the virtual instant `abort()`
   was called, which the test reads from tokio's clock; that a BLOB of another picture
   (US6-S2's 3 × 2, 8-bit one) the script sends 1 s later is discarded; and that a
   following `expose` of 100 ms, made after that
   BLOB has come, for which the adapter sends `CCD_FRAME_TYPE` and `CCD_EXPOSURE` 0.1
   and the script answers the frame of US6-S1, succeeds.
7. **Given** the camera of US4-S1, **Then** a committed test asserts that a BLOB whose
   format is `.fits.fz` answers `Failed` with `the camera sent .fits.fz, not .fits`; one
   whose FITS has `NAXIS` 3 answers `Failed` with
   `the camera sent a 3-axis FITS picture`; one with `BITPIX` −32 answers `Failed` with
   `unsupported FITS BITPIX -32`; and one whose data is shorter than its header says
   answers `Failed` with `the camera's FITS data is short`, each without a second try.
8. **Given** an `IndiCamera` named for the device of
   `fixtures/scope-node/indi/ccd-simulator.script` over that script, **When** a test
   calls `describe()`, applies the region (0, 0, 64, 48) and calls `expose` for 1000 ms
   with `frame`, **Then** a committed test asserts that the script was satisfied to its
   end; that the description's model, width, height, pixel size, bits, exposure range,
   gain range, offset range and binnings equal those of
   `fixtures/scope-node/indi/ccd-simulator.expect.json`; and that the frame's width,
   height and bits, its first eight samples and the sum of all its samples equal that
   file's (oracle: the saved capture, and the values a separate program read from it
   when it was recorded).

---

### User Story 7 - A copy of each frame kept at the scope node (Priority: P3)

As the observer on a poor link, I can ask the scope node to keep its own copy of every
imaging frame on its disk, so that a frame lost on the way is not lost.

**Why this priority**: A safety net the first nights can do without; it adds a config
key and a file writer to the node.

**Independent Test**: `crates/scope-node/tests/imaging_save.rs`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p scope-node --locked --test imaging_save`. What the
console shows of the copy is 031's US5-S4, tested there.

**Acceptance Scenarios**:

1. **Given** the node rig given `.with_imaging_save_dir(D)`, D a child path of the
   test's scratch folder that does not exist yet, **When** a test calls `Expose` with 100 ms, `frame` and
   `save` true, **Then** a committed test asserts that the header's `node_file` is
   `frame-000001.fits` and its `node_save_error` is empty; that D holds exactly that
   file, 11,520 bytes, and no `.part` file, before the header arrives; and that, read as
   the shared setup says, its cards are in order `SIMPLE` T, `BITPIX` 16, `NAXIS` 2,
   `NAXIS1` 64, `NAXIS2` 48, `BZERO` 32768, `BSCALE` 1, `EXPTIME` 0.1, `GAIN` 100.0,
   `OFFSET` 10.0, `XBINNING` 1, `YBINNING` 1, `XORGSUBF` 0, `YORGSUBF` 0, `CCD-TEMP`
   20.0, `BAYERPAT` `RGGB`, `INSTRUME` `synthetic imaging camera`, `IMAGETYP` `LIGHT`,
   `ROWORDER` `TOP-DOWN`, `FRAMEID` 1, `PURPOSE` `frame`, `NODE-T0` the header's
   `start_s`, then `END`, with no `SET-TEMP` or `SERIALNO`, and its samples equal the
   sample rule and the stream's bytes.
2. **Given** the same node, **Then** a committed test asserts that a second saved frame
   is `frame-000002.fits`; that with a file `frame-000003.fits` already in D the third is
   saved as `frame-000003-2.fits` and the earlier file's bytes are unchanged; that
   the purposes `dark` and `flat` write `IMAGETYP` `DARK` and `FLAT`; that, after
   `ConfigureImaging` of binning 2 and the region (8, 4, 16, 8), the next saved file has
   `NAXIS1` 8, `NAXIS2` 4, `XBINNING` 2, `XORGSUBF` 4 and `YORGSUBF` 2 (worked example:
   the frame's region origin over its binning, 8/2 and 4/2); and that `save` false then
   writes nothing and answers both fields empty (ids count every frame, saved or not).
   **Given** a node given `.with_imaging_save_dir(D)` whose imaging camera is one of the
   test's own that answers a 3 × 2, 8-bit, mono frame with no gain, offset or
   temperature, **Then** the saved file has `BITPIX` 8, no `BZERO`, `BSCALE`, `GAIN`,
   `OFFSET`, `CCD-TEMP` or `BAYERPAT`, and the six bytes as they are.
3. **Given** the node rig with no save folder, **Then** a committed test asserts that
   `save` true answers a header with `node_file` empty and `node_save_error`
   `this scope node has no save_dir`, followed by the whole frame. **Given** the node
   rig whose save folder is a regular file, **Then** `node_save_error` begins
   `could not save the imaging frame: ` and the whole frame still follows.
4. **Given** `Config::parse`, **Then** a committed test asserts that `[imaging]` with
   `adapter = 'simulated'` and `save_dir = '/var/lib/scope-node/frames'` holds that
   path, that `save_dir = ''` is refused with an error containing `imaging.save_dir`,
   and that the built `scope-node --check-config` prints
   `save_dir = "/var/lib/scope-node/frames"` under `[imaging]`.

---

### User Story 8 - Probe the camera from the scope node (Priority: P3)

As the observer bringing up a scope node, I run one command on it that describes the
imaging camera, takes three exposures and prints each one's size and how long after the
end of the exposure the frame was in hand, so that I learn the download time before the
console is involved.

**Why this priority**: It is a bring-up tool. Its numbers set the budgets later specs
assume, and nothing depends on its code.

**Independent Test**: `crates/scope-node/tests/imaging_probe.rs`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p scope-node --locked --test imaging_probe`.

**Acceptance Scenarios**:

1. **Given** `scope_node::imaging::probe::run` with the stand-in camera, the test clock
   and 500 ms, **Then** a committed test asserts that it writes exactly these lines to the writer it
   is given:
   `imaging camera: synthetic imaging camera`, `serial: none`,
   `sensor: 64 x 48 px, 4.78 um pixels, 16 bits, RGGB`, `exposure: 1 to 60000 ms`,
   `gain: 0 to 300`, `offset: 0 to 100`, `binnings: 1, 2`, `cooler: yes`, and for n from
   1 to 3 `exposure n: 64 x 48, 6144 bytes, in hand 0 ms after the end of the exposure`;
   and that it took exactly 1.5 s of virtual time.
2. **Given** `run` with a camera of the test's own that describes a mono camera with a
   serial `A1`, no gain or offset control and no cooler, and answers each 100 ms
   exposure 350 ms after it was asked with a 3 × 2, 8-bit frame, **Then** a committed
   test asserts the lines `serial: A1`,
   `sensor: 3 x 2 px, 5.2 um pixels, 8 bits, mono`, `gain: none`, `offset: none`,
   `cooler: no` and `exposure 1: 3 x 2, 6 bytes, in hand 250 ms after the end of the
   exposure` (worked example: 350 − 100). **Given** a camera whose second exposure
   answers `Failed("the camera fell over")`, **Then** `run` answers that error, having written the
   description and the first exposure's line and nothing after.
3. **Given** the built `scope-node` run, as `run` in `crates/scope-node/tests/cli.rs:15`
   runs it and with its config file kept as the shared setup's scratch rule says,
   with `--probe-camera --probe-exposure-ms 50` on a config holding `[imaging]` and
   `adapter = 'simulated'`, **Then** a committed test asserts that it exits 0 and that
   its output has the eight description lines of US8-S1 and three lines matching
   `exposure \d: 64 x 48, 6144 bytes, in hand \d+ ms after the end of the exposure`,
   and that it exited within 2.5 s of real time, which three exposures of the default
   1000 ms cannot.
   **Given** a config with no `[imaging]` table, **Then** it exits non-zero with
   `scope-node: no [imaging] table in the config` on standard error; and
   `--probe-camera` with `--check-config` is refused by the argument parser.

---

### Edge Cases

- **A scope node built before this spec.** It lacks the service, and a caller gets
  `Unimplemented` (US3-S4); the console then says to rebuild it (031). A console built
  before 031 never calls the new service.
- **A night without an imaging camera.** With no `[imaging]` table the node answers
  `FailedPrecondition` to every call of the service, and nothing else about the node
  changes.
- **The INDI server is not running, or the camera is unplugged.** The node has a camera
  configured but cannot describe it: every call answers the adapter's words, and the
  adapter reconnects by itself at the next call. The node starts all the same.
- **The caller goes away during an exposure**, as when the link drops. The node finishes
  its exposure and discards the frame (owner decision 11). Nothing resumes.
- **The link drops while bytes are in flight.** The stream ends; a copy kept at the
  node, when asked for, is already on its disk, since it is written before the header.
- **The scope node restarts.** Its frame ids start again at 1. Its own copies are never
  replaced: a name already taken gets `-2`, `-3` and so on (US7-S2).
- **A camera with no gain, offset or cooler, or a mono camera.** The description says so
  (an absent range, `cooler` false, an empty `bayer`), the settings omit them, and the
  node's copy omits the cards.
- **An exposure longer than the heartbeat's silence limit.** The heartbeat is on the
  other connection and is unaffected: a 60 s exposure is one call that answers after
  60 s.
- **A full or missing save folder at the scope node.** The header says why there is no
  copy, and the whole frame still follows (US7-S3).
- **A frame taken with a region of interest.** The scope node sends the region in force
  with the frame, and its own copy says where the frame sat on the sensor (`XORGSUBF`,
  `YORGSUBF`; owner decision 12).
- **A camera that delivers less than the region over the binning.** The frame's own
  width and height are used everywhere; nothing assumes the stand-ins' rule (FR-003).
- **The camera's white balance and other controls.** The controls `[imaging] neutral`
  names are set on each connection, so after a camera's power cycle they are set again
  at the next connection; every other control stays as the device reports it; none is
  in any file (owner decision 13).
- **A `neutral` key the camera does not have**, misspelt or from another camera. The
  connection fails with `the camera has no control {name}`, and every call answers
  those words until the table is corrected.
- **The scope node restarts with the cooler running.** The adapter remembers only the
  set temperature it was asked for in this run, so the status has none until one is
  set again.
- **Two callers ask for an exposure at once.** One exposure runs; the other call's
  stream answers `ResourceExhausted` with `an exposure is already in progress`
  (US3-S5).
- **Settings changed on the camera by another INDI client.** The adapter answers the
  settings as the device reports them, and a frame's `roi` is the device's `CCD_FRAME`
  as it stood when the exposure was asked for, so the frame is stamped as it really
  was (owner decision 12; US6-S2). An edit that names neither `roi` nor `binning` is
  never refused for a region another client set; only a new region gets out of one
  that breaks the rule.
- **The simulated camera on an untracked mount.** Stars trail at the sky's rate, about
  7 px a second at the default scale, as they would.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: `crates/scope-node` SHALL gain the module `imaging`, holding the imaging
  side of the camera port as the trait `ImagingCamera`: `describe`, `status`, `apply`,
  `expose` and `abort`, each taking `&self` and answering a future, so that one camera
  can be asked its status or told to abort while it exposes (owner approval 2, owner
  decision 2). The finder camera's `Camera` trait and `GrayFrame` SHALL not change.
- **FR-002**: The port's types SHALL be `ImagingInfo` (model, serial when the camera
  gives one, sensor width and height in pixels, pixel size in µm, bits per sample, the
  Bayer pattern or none, exposure range in ms, gain range and offset range when the
  camera has those controls, the binnings it supports, whether it has a cooler),
  `ImagingSettings` (gain, offset, binning, region of interest, cooler on, set
  temperature, each optional), `ImagingStatus` (sensor temperature, cooler power, set
  temperature, whether an exposure is in progress), `Exposure` (milliseconds and a
  purpose label) and `ImagingFrame` (the samples as delivered, little-endian, 8 or 16
  bits; width, height, bits, Bayer pattern; exposure start and end on the node's clock;
  exposure, gain, offset, binning and sensor temperature; and the region of interest in
  force for the exposure, in unbinned sensor pixels, as the plan document's Amendment 6
  asks). `ImagingFrame` and `Roi` SHALL derive `Default`, so that a test's literal ends
  with `..Default::default()`.
- **FR-003**: `apply` SHALL change only the settings present and answer every setting in
  force. A value outside what the camera describes SHALL be refused with
  `ImagingError::Invalid` and the texts of US1-S5 and US1-S6, changing nothing. A region
  of interest is in unbinned sensor pixels, SHALL lie inside the sensor and SHALL have x
  and y that are multiples of twice the binning in force, so a colour mosaic keeps its
  pattern. On the two stand-in cameras the delivered frame is the region's width and
  height each divided by the binning, rounded down. A real camera may deliver less (the
  worked-example camera gives 1232 × 820 at binning 4, not 1236 × 821:
  `docs/research/2026-11-15-imaging-camera.md` § 1. The camera), so everywhere the
  frame's own width and height are what count. A purpose label SHALL be 1 to 16
  lower-case letters.
- **FR-004**: The module SHALL hold `SyntheticImagingCamera`, the stand-in camera of
  plan.md § The stand-in camera, whose frames follow the sample rule and carry the
  region in force as their `roi`, and whose description, starting settings and cooler
  follow US1-S1 and US1-S2.
- **FR-005**: An exposure SHALL take its exposure time on the node's clock; its
  `start_s` SHALL be the clock's reading when it began and its `end_s` that plus the
  exposure. While one runs, on every camera adapter, `status` SHALL say so, a second
  `expose` SHALL answer `ImagingError::Busy` at once, an `apply` that names any setting
  SHALL answer `Busy` and change nothing while an empty `apply` still answers the
  settings in force, and `abort` SHALL end the exposure with `ImagingError::Aborted`;
  `abort` with nothing exposing SHALL answer `Ok`.
- **FR-006**: `crates/simulator` SHALL gain the module `imaging` and export
  `SimImagingCamera`, `ImagingOptics`, `ImagingTruth` and `SimStar` (owner approvals 2
  and 9). The imaging optics (sensor size, field of view, imaging angle, focus error,
  pixel size) SHALL be simulator state beside the finder optics, defaulting to
  1236 × 822 pixels over 0.75°, an imaging angle of 17°, no focus error and 19.12 µm.
  The existing `Optics`, `SimCamera` and every existing public name SHALL not change.
- **FR-007**: The simulated imaging camera SHALL look along the main telescope's axis,
  the direction of the finder optics' eyepiece pixel, with its frame turned from the
  finder frame by the imaging angle about that axis, positive turning frame-up toward
  the finder frame's left. `imaging_truth` and `imaging_truth_at` SHALL answer the
  frame centre's J2000 RA and Dec, the angle of frame-up east of celestial north and
  the scale in arcseconds a pixel, to the tolerances of US2-S2 and US2-S3.
- **FR-008**: `deep_field` SHALL answer the stars of a seeded field that depends only on
  the seed and the place on the sky: 320 stars a square degree brighter than magnitude
  16, magnitudes 8 to 16 rising in number by a factor of 10^0.33 a magnitude, each with
  a colour, brightest first (US2-S8). A camera built with `with_stars` SHALL draw those
  stars and no others.
- **FR-009**: A simulated frame SHALL be 16-bit RGGB samples made of stars, a sky
  background and noise by the constants of plan.md § The simulated imaging camera,
  repeatable from the camera's seed and the frame's number, with `with_noise(false)`
  giving the noiseless frame (US2-S4).
- **FR-010**: A simulated star SHALL be a Gaussian whose width is √(1.2² + e²) pixels for
  a focus error e, drawn along the straight line from its place at the exposure's start
  to its place at its end, so that the sky's and the mount's motion within the exposure
  smear it (US2-S5, US2-S6).
- **FR-011**: The simulated camera SHALL honour `binning` 2 by averaging samples of the
  same colour, keeping the mosaic: a delivered sample is the mean of those of its four
  same-colour sensor samples that lie inside the region, a sample outside the region
  being left out of the mean, so that a region whose width or height is not a multiple
  of 4 still delivers its width and height each halved, rounded down (618 × 411 from
  1236 × 822). It SHALL honour a region of interest by rendering only it, and every
  frame SHALL carry the region in force as its `roi` (US2-S7).
- **FR-012**: The simulated camera SHALL honour the port as FR-003 and FR-005 say, on the
  simulator's clock, with the description, starting settings and cooler of US2-S1.
- **FR-013**: The scope link SHALL gain a service `ScopeNodeImaging`, under the pairing
  token, with the calls `DescribeImaging`, `ImagingStatus`, `ConfigureImaging`, `Expose`
  and `AbortExposure` and the messages of plan.md § The scope link. No existing message,
  field or call SHALL change (owner approval 1).
- **FR-014**: The scope node SHALL serve the service among its routes whether or not it
  has an imaging camera. Without one, every call SHALL answer `FailedPrecondition` and
  `no imaging camera`. `NodeService::with_imaging` SHALL attach one.
- **FR-015**: `DescribeImaging`, `ImagingStatus` and `ConfigureImaging` SHALL answer the
  port's `describe`, `status` and `apply`. `ImagingError::Invalid` SHALL become
  `InvalidArgument`, `Busy` `ResourceExhausted` with
  `an exposure is already in progress`, `Aborted` `Aborted` with
  `the exposure was aborted`, and `Failed` `Internal`, each with the error's text.
- **FR-016**: `Expose` SHALL answer a stream: one header with the frame's id, size, bits,
  Bayer pattern, times, exposure, gain, offset, binning, temperature, purpose, byte
  count and region of interest (`roi`, the frame's own, unbinned sensor pixels), then
  the samples in order in chunks of at most 262,144 bytes, then the end. Ids
  SHALL count the node's frames from 1. Every failure after the token check (no camera,
  a refused request, `Busy`, `Aborted`, a camera failure) SHALL be the stream's first
  and only item, never the call's own status.
- **FR-017**: The node SHALL run an exposure to its end whether or not the caller is
  still there, and discard a frame nobody is reading; only `AbortExposure` aborts
  (owner decision 11).
- **FR-018**: With `max_bytes_per_s` above 0 the node SHALL send a chunk no sooner than
  the bytes already sent divided by that rate after the first chunk; with 0 it SHALL
  not pace (US3-S8).
- **FR-019**: `SimNode::new` SHALL attach a `SimImagingCamera` over its simulator, and
  `SimNode::from_cameras` SHALL take one of the caller's (US3-S9). `Expose` with `save` true on
  a node with no save folder SHALL answer a header whose `node_save_error` is
  `this scope node has no save_dir`, and the frame.
- **FR-020**: The scope node's config SHALL accept an optional `[imaging]` table with
  `adapter` (`"indi"` or `"simulated"`, required), and for `indi` only `address` (by
  default 127.0.0.1, port 7624), `device` and `neutral`, a table of
  `"<property>.<element>" = number` (by default empty; owner decision 13); unknown keys
  and the errors of US5-S3 SHALL be refused without printing the token, and
  `--check-config` SHALL print `neutral` when it is not empty (US5-S4). `scope-node`
  SHALL depend on `quick-xml` as owner approval 3 gives it, from the story whose reader
  first calls it (US4; owner approvals 3 and 4); `base64` waits for the story whose code
  uses it (FR-028).
- **FR-021**: The INDI adapter SHALL be a client of the server at that address. On each
  new connection it SHALL send `getProperties` for protocol 1.7; choose the named
  device, or the first whose `DRIVER_INFO` `DRIVER_INTERFACE` has the camera bit, of
  value 2; send `CONNECT`
  when the device is not connected; send `enableBLOB` `Also`; and, for each of
  `CCD_COMPRESSION`, `CCD_TRANSFER_FORMAT` and `UPLOAD_MODE` the device defines, switch
  on `INDI_DISABLED`, `FORMAT_FITS` and `UPLOAD_CLIENT` (US4-S1, US4-S2). After those
  three set-up switches it SHALL send one `newNumberVector` for each property the
  `neutral` table names, holding the elements named at their values and every other
  element of that property at the device's reported value, and await the device's
  answer to each as `apply` awaits its own; a name the device does not define SHALL be
  `Failed("the camera has no control {name}")`, checked before any of them is sent, and
  any of these failures SHALL fail the connection, so that the next call connects again
  (US5-S5; the plan document's Amendment 2).
- **FR-022**: `describe` SHALL answer from the device's own properties: the device name
  as the model, `Serial Number` `SN` as the serial, `CCD_INFO`, `CCD_CFA` `CFA_TYPE`,
  the range of `CCD_EXPOSURE_VALUE` in ms, gain and offset from `CCD_CONTROLS` `Gain`
  and `Offset` or from `CCD_GAIN` `GAIN` and `CCD_OFFSET` `OFFSET`, every whole binning
  in `HOR_BIN`'s range, and a cooler when `CCD_COOLER` is defined. No value SHALL be
  taken from a camera's name or model. A `CCD_BITSPERPIXEL` other than 8 or 16 SHALL
  answer `Failed` with `unsupported sample size <n> bits` (US4-S3).
- **FR-023**: `status` SHALL answer the latest `CCD_TEMPERATURE` and `CCD_COOLER_POWER`
  the device has sent, the set temperature the adapter last asked for, and whether an
  exposure is in progress, without waiting on the device once the adapter is connected;
  a first call connects, and fails as `describe` does when it cannot.
- **FR-024**: `apply` SHALL send, for each setting that is present, one vector, in the
  order plan.md § The INDI adapter's **`apply`** gives (binning, the region, gain and
  offset, the cooler, the set temperature; US5-S1 spreads them over three applies),
  wait up to 5 s for the device's answer to each, and answer the
  settings in force as the device then reports them. An element of a vector that the
  edit does not name SHALL go back at the device's latest value, which for a control the
  `neutral` table names is the neutral value set on connecting (owner decision 13). An
  answer in the state `Alert`
  SHALL be `Failed` with `the camera refused <property>: <message>`, and no answer
  `Failed` with `the camera did not answer <property> within 5 s`.
- **FR-025**: The adapter SHALL connect when first used and again after a lost
  connection, with the errors of US4-S3. The `scope-node` binary SHALL build the adapter
  the `[imaging]` table names and attach it to its service, and `--check-config` SHALL
  print the table or `# no imaging camera configured` (US5-S4).
- **FR-026**: `crates/scope-node/README.md` SHALL gain `## The imaging camera`: what must
  be installed and running on the scope node (an INDI server and the camera's device
  program, by the owner), the `[imaging]` table, how the device is chosen, and that the
  node must be rebuilt (owner approval 7). It SHALL name INDI's server and device
  programs and their licence, LGPL-2.1-or-later
  (`docs/research/2026-11-15-imaging-camera.md` § 2. INDI), and say that the node is
  their client over a socket and links none of them (the constitution's Principle V,
  1.3.0). It SHALL say that `neutral` is how a
  camera's white balance is made neutral: on a camera whose white-balance controls scale
  the raw samples, the table names them with their neutral values, the research note's
  camera's as the worked example, and the adapter sets them on each connection; that the
  adapter leaves every control neither the table nor an `apply` names as the device
  reports it; and that it records no control with a frame (owner decision 13).
- **FR-027**: `expose` SHALL set `CCD_FRAME_TYPE` to `FRAME_DARK` for the purpose `dark`,
  `FRAME_FLAT` for `flat` and `FRAME_LIGHT` for any other, when the device defines it,
  then set `CCD_EXPOSURE_VALUE` in seconds. The frame's `start_s` SHALL be the node
  clock's reading when the device first reports the exposure `Busy`, or when the request
  was sent if it never does, and `end_s` that plus the exposure.
- **FR-028**: The frame SHALL be made from the `.fits` BLOB of `CCD1`: its width, height
  and bits from the FITS header, its samples converted from big-endian with `BZERO` to
  little-endian and otherwise untouched, its Bayer pattern from `BAYERPAT` or `CCD_CFA`,
  its binning and temperature from the header or the device's properties, its gain
  and offset from the device's properties, and its `roi` from the device's `CCD_FRAME`
  as it stood when the exposure was asked for, or the whole sensor of `CCD_INFO` when
  the device defines no `CCD_FRAME` (US6-S1, US6-S2). The BLOBs of US6-S7 SHALL be
  refused. From
  this story on `scope-node` SHALL depend on `base64` as owner approval 3 gives it, for
  the BLOB's text.
- **FR-029**: An exposure SHALL fail when the device reports `Alert`, when no frame has
  arrived the exposure plus 60 s after the request was sent, or when the connection
  closes. After a failure
  the adapter SHALL send `CCD_ABORT_EXPOSURE`, wait 1 s and try once more, reconnecting
  first if it must; after a second failure it SHALL answer `Failed` with
  `the exposure failed twice: ` and the reason, as US6-S3 to US6-S5 word it (owner
  decision 6).
- **FR-030**: `abort` SHALL send `CCD_ABORT_EXPOSURE` and end the exposure with
  `Aborted`; a BLOB that arrives when no exposure is waiting SHALL be discarded; a
  second `expose`, and an `apply` that names any setting, SHALL answer `Busy` and send
  nothing (US6-S6).
- **FR-031**: The adapter SHALL satisfy the saved capture
  `fixtures/scope-node/indi/ccd-simulator.script` to its end and answer the values of
  `fixtures/scope-node/indi/ccd-simulator.expect.json` (US6-S8, owner approval 8).
- **FR-032**: `## The imaging camera` SHALL say what the adapter does on a failed
  exposure, which of the device program's own handling it relies on (up to three
  starts, and one restart of a failed exposure), and that it does not rely on the
  device program's USB reset.
- **FR-033**: With `save` true and a save folder, the scope node SHALL write each frame
  to that folder before it sends the header, as `frame-<id, six digits>.fits`, under a
  `.part` name and renamed, adding `-2`, `-3` and so on when the name is taken, and
  SHALL name the file in the header. With `save` false it SHALL write nothing.
- **FR-034**: The `[imaging]` table SHALL accept `save_dir`, which
  `NodeService::with_imaging_save_dir` gives the service and `--check-config` prints
  (owner approval 4).
- **FR-035**: The node's file SHALL be one FITS image with the cards of US7-S1, the
  exposure's start on the node's clock as `NODE-T0` and no `DATE-OBS`; its `XORGSUBF`
  and `YORGSUBF` SHALL be the frame's `roi` origin divided by its binning, rounded down
  (US7-S2), and it SHALL carry no `SET-TEMP` or `SERIALNO`. A failure to
  write SHALL be reported in the header as `could not save the imaging frame: <error>`
  and SHALL not stop the frame being sent (owner decision 9).
- **FR-036**: `## The imaging camera` SHALL document `save_dir`, the files and that the
  node never deletes them.
- **FR-037**: `scope-node --probe-camera` SHALL build the camera the `[imaging]` table
  names, write its description and, for three exposures of `--probe-exposure-ms`
  (default 1000), each frame's size and the time from the end of the exposure to the
  frame in hand, in the lines of US8-S1, and exit; with no table it SHALL exit
  non-zero with `no [imaging] table in the config` (owner approval 5).
- **FR-038**: `## The imaging camera` SHALL document the probe and what its times mean.

### Key Entities

- **Imaging frame**: one exposure from the imaging camera as the scope node relays it.
- **Imaging settings**: gain, offset, binning, region of interest and cooler, held by
  the camera. The exposure, its purpose label, the forwarding rate and the ask for a
  copy travel with each `Expose` call.
- **Purpose label**: why an exposure is taken; `frame` and `probe` here, more in later
  specs.
- **Neutral controls**: the camera controls `[imaging] neutral` names, set to their
  values on each connection; white balance on a camera whose controls scale the raw
  samples.
- **The node's copy**: the FITS file the scope node writes of a frame when asked, in
  its `save_dir`.
- **INDI script**: one side of a conversation with an INDI server, for the tests.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario has a committed test that asserts it, and the
  `lint` and `test` gates pass.
- **SC-002**: No existing test, golden, `scripts/ci/` file, `.github/` file, `CODEOWNERS`
  entry, `ergane.yaml`, `.specify/` file or `playwright.config.ts` is edited. Every
  test of this trio is a new file under `crates/scope-node/tests` or
  `crates/simulator/tests`, which needs no line in any existing file.
- **SC-003**: A frame of 32,472,192 bytes crosses the scope link in 124 chunks while
  every heartbeat is answered in its 250 ms slot (US3-S8).
- **SC-004**: The INDI adapter hands on a FITS BLOB's samples exactly (US6-S1).
- **SC-005**: No file under `crates/scope-node`, `crates/scope-link` or
  `crates/simulator` names a camera model outside tests, documents and comments, and
  neither `crates/scope-node` nor `crates/simulator` gains `unsafe`.

## Assumptions

- **Earlier features are in place.** `025-frame-capture` and
  `028-finder-tests-do-less` have landed on `main`, 028 completely at 49bb210, and with
  them every spec they name. Every citation here was drafted at dbc2b5b and checked
  again at 49bb210, the worktree's HEAD; the places 49bb210 moved are corrected, and
  none of the cited files changed by `main` at 2f6a777 (plan.md § Prerequisites 2).
- **Specs 031 to 037 build on this one** and are re-anchored after it lands.
- **Out of scope:**
  - everything on the console's side of the scope link: its second connection, the
    files it saves, the preview, the Imaging page, the MCP tools and the camera's one
    holder (031);
  - solving an imaging frame, the aim point and the imaging angle (032);
  - any focus reading or focus aid (033);
  - darks, flats and the calibration library, beyond carrying the purpose label (034);
  - stacking (035), tracking from imaging frames (036) and the imaging run (037);
  - any change to the finder camera, star detection, the live loop or recordings;
  - a camera reached any other way than INDI or the stand-ins;
  - INDI's fast BLOBs, its compression, and a second INDI connection;
  - a mount, focuser or filter wheel through INDI;
  - the set temperature and the serial in the scope node's own copy (owner decision
    12);
  - recording any camera control with a frame, and setting any control but gain,
    offset, binning, the region, the cooler and those `[imaging] neutral` names (owner
    decision 13);
  - listing, fetching or deleting the scope node's copies through the scope link;
  - MCP on the scope node.
- **Outside the factory:** installing INDI and the camera's device program on the scope
  node, and running the server; recording the saved capture (done on 2026-11-17:
  plan.md § Prerequisites 4); rebuilding the scope node; the field checks at the
  telescope: the real camera described and exposing through the node; the probe's
  download time on the scope node
  (the hop across the link, which the probe cannot see, is read at the console: 031);
  a recording of the real camera's conversation; the names of the real camera's gain
  and offset controls; whether its binned frame keeps the Bayer mosaic, and what size
  it delivers; and the names of its white-balance controls read with an INDI client and
  written into `[imaging] neutral`, and the neutral values read back after a power
  cycle and a reconnection.
- **The factory's gates and CI:** the gates are `lint` and `test`, copied in plan.md
  § Verification. CI's other jobs run only in the merge queue, so no scenario's Then
  clause rests on them. `cargo-deny` and `cargo-machete` there judge US4's `quick-xml`
  and US6's `base64`; each is added by the story whose code first uses it, because
  `cargo-machete` refuses a dependency nothing uses.

## Work Graph

```yaml
US1:
  depends_on: []
  implements: [FR-001, FR-002, FR-003, FR-004, FR-005]
US2:
  depends_on: []
  depends_on_merged: [US1]
  implements: [FR-006, FR-007, FR-008, FR-009, FR-010, FR-011, FR-012]
US3:
  depends_on: []
  depends_on_merged: [US1, US2]
  implements: [FR-013, FR-014, FR-015, FR-016, FR-017, FR-018, FR-019]
US4:
  depends_on: []
  depends_on_merged: [US3]
  implements: [FR-020, FR-021, FR-022, FR-023, FR-025]
US5:
  depends_on: []
  depends_on_merged: [US4]
  implements: [FR-020, FR-021, FR-024, FR-025, FR-026]
US6:
  depends_on: []
  depends_on_merged: [US4, US5]
  implements: [FR-027, FR-028, FR-029, FR-030, FR-031, FR-032]
US7:
  depends_on: []
  depends_on_merged: [US3, US5, US6]
  implements: [FR-033, FR-034, FR-035, FR-036]
US8:
  depends_on: []
  depends_on_merged: [US7]
  implements: [FR-037, FR-038]
```

Every edge is a merge edge, because each dependent imports or extends what its
predecessor creates. US4 and US5 share three requirements, each in its own half:
FR-020's `quick-xml` is US4's and its `[imaging]` table US5's; FR-021's steps up to the
three set-up switches are US4's and the neutral controls US5's; FR-025's reconnecting is
US4's and the binary's attaching US5's.

- **US2 after US1**: the simulated camera implements US1's trait and types.
- **US3 after US1 and US2**: the service serves US1's port, and `SimNode` attaches US2's
  camera.
- **US4 after US3**: the INDI adapter is a camera for US3's service, and US4 adds its
  `pub mod indi` line to the imaging module US3 last wrote.
- **US5 after US4**: settings, the neutral controls and the config extend US4's adapter
  and use its stand-in server and worked scripts unchanged, and the binary attaches the
  adapter with US3's `NodeService::with_imaging`.
- **US6 after US4 and US5**: exposures extend the adapter US5 last wrote, use US4's
  stand-in server and worked scripts unchanged, and add `base64` to the manifest and
  lock file US4 changed. The edge to US4 follows from the one to US5; it is written out
  because US6 writes files US4 last wrote.
- **US7 after US3, US5 and US6**: it extends US3's service and writes US5's config and
  binary and the imaging module and README after US6 has. The edges to US3 and US5
  follow from the one to US6; they are written out because US7 writes files those two
  stories last wrote.
- **US8 after US7**: the probe uses US5's config and the adapters, and the edge orders
  the node's binary, README and imaging module after US7's edits.

The eight stories form one chain, so no two run together. The console's stories are
031's and start after this trio has landed.
