---
state: ready
depends_on_landed: ["013-live-finder", "030-imaging-camera-link", "044-node-stops-mount-before-exit", "048-node-detects-stars-on-a-dark-sky"]
# Drafted 2026-11-25 against `main` at be7f3ff6 (be7f3ff686adc431878819f7e30ee56a9c5d48ea),
# at the owner's ask ("a spec to allow asi120mm to be used with the pi"), from a probe of
# that day on the branch `probe/045` cut from be7f3ff6: a trial build of both stories with
# every scenario's test passing and the gates' Rust commands passing, the brief's leads
# as runs, and one run of the trial node against a real `indiserver` with INDI's camera
# simulators, in a container with no network. Every `path:line` here was read at
# be7f3ff6. State: draft. Folded the same day with cynical review round 1 (1 blocker,
# 1 major, 10 minors) and the owner's answers of 19:21 UTC, every decision as
# recommended and as the review amended it (§ Owner decisions); the fold's runs are on
# `probe/045` at 3aa0bb38, bd2e382f, 57844515, b2b19ba4 and df2ea0c9 (plan.md § What was
# run and what was read). The record is written to the answers (plan.md § Refinement
# Record). 030 is named because US1 changes its INDI client and both stories' tests use
# its stand-in server; 044 because it last changed `lib.rs` and `main.rs`; 048 because it
# last changed `config.rs` and the node's README; 013 because FR-007 of its spec gains the
# third adapter. What rests on a run and what on reading is said where it is used
# (plan.md § What was run and what was read).
# Flipped to `ready` on 2026-11-25 at the owner's ask ("flip to ready"), while probe
# round 1 (both stories built from this text alone) was still running, so the owner's ask
# waives plan.md's prerequisite 4; its merge, on the owner's go, gives the owner
# approvals. `main` still at be7f3ff6.
---

# Feature Specification: The Node Runs the Finder Camera over INDI

**Feature Branch**: `045-node-runs-finder-over-indi`

**Created**: 2026-11-25

**Status**: Ready

**Input**: A ZWO ASI120MM Mini (a 1280 × 960 mono sensor of 3.75 µm pixels, 12 bits, on
USB 2) replaces the Pi's ribbon camera as the finder camera, on the Pi 5 beside the ZWO
ASI071 imaging camera. The scope node reaches the imaging camera through an INDI server;
make it reach the finder camera through the same server, as the finder camera's third
camera adapter, built on the node's INDI client, so both cameras share one `indiserver`
and one ZWO device program. Linking ZWO's SDK was turned down on 2026-11-09: it ends
the node's static musl build. Keep the optics open: the field comes from the database
the console chooses (047).

## Clarifications

### Session 2026-11-25 (the probe on `main` at be7f3ff6)

- Q: What does this trio read besides itself? → A: Nothing about the feature. `spec.md`
  holds every requirement and scenario, and `plan.md` every design decision, the
  interfaces, the shapes that existing tests pin and the alternatives rejected. A story
  that would change a decision escalates rather than re-deciding it. The probe's branch
  and logs are not in the repository; what they showed is written here.
- Q: What finder cameras does the node have today? → A: Two adapters, `rpicam` and
  `simulated` (`crates/scope-node/src/config.rs:8`), behind the camera port
  (`crates/scope-node/src/camera.rs:31`), which yields 8-bit gray frames (013's FR-004,
  `specs/013-live-finder/spec.md:720`). The imaging side has an INDI client,
  `IndiCamera` (`crates/scope-node/src/imaging/indi/mod.rs:1835` for its TCP
  constructor), behind the imaging port, which drives the ASI071 through
  `indiserver -p 7624 indi_asi_ccd` on the Pi. 013's FR-007 reads, in part: "The scope node
  SHALL read its listen address, token, camera adapter (`rpicam` or `simulated`)"
  (`specs/013-live-finder/spec.md:739`); this spec adds `indi` to that list and changes
  nothing else in it.
- Q: How does a finder camera over INDI give its description before the node serves?
  → A: The port's `info` is synchronous and the service reads it once, before it serves
  (`crates/scope-node/src/service.rs:162`), while the INDI client connects at its first
  call. So the adapter describes the camera once, at start, as `RpicamCamera::discover`
  does (`crates/scope-node/src/main.rs:83`): a camera it cannot describe ends the start
  with `scope-node: <message>` and exit 1 (`crates/scope-node/src/main.rs:46`), and
  systemd starts the node again a second later (`Restart=always`, `RestartSec=1` in the
  README's unit, `crates/scope-node/README.md:85`) until the unit's start limit: by
  default 5 starts within 10 s (`systemctl show`: `DefaultStartLimitIntervalUSec=10s`,
  `DefaultStartLimitBurst=5`), after which, per `man systemd.unit`, a unit that reaches
  the limit is "not attempted to be restarted anymore". With no server the node exits at
  once, so a server away for about 5 s while the node starts would leave it failed, and
  the mount and the imaging camera with it, until `systemctl reset-failed`. The node's
  unit therefore takes `StartLimitIntervalSec=0` in its `[Unit]` ("may be set to 0 to
  disable any kind of rate limiting", the same page): in the README's new section
  (FR-011) and on the Pi (plan.md § Outside the factory). Run against a real server
  (plan.md § What was run and what was read): no server, exit 1 at once; a named device
  the server lacks, exit 1 after 5 s; a device that is not a camera, exit 1 after 10 s,
  each with the client's own words. Owner decision 8 weighs waiting instead.
- Q: What does the ASI120MM's 12-bit sample become? → A: Detection reads 8 bits (the
  constitution's product limits). Through INDI the camera sends 16-bit FITS (RAW16, its
  12 bits shifted up by 4) or 8-bit FITS when its video format is RAW8 (by reading; not
  seen on this camera). Owner decision 1 weighs the high byte, RAW8 on the camera, a
  stretch, a smaller shift and the camera's `Offset`, measured on synthetic frames: the
  high byte finds fewer of the faintest stars at the lowest noise, about as many as a
  stretch once the noise is half an 8-bit level, and never a false one, at an offset
  level of 64 in 12-bit units and, in review round 1, of 3.
- Q: What gain does the camera report, and does automatic gain work on it? → A: ZWO's
  `CCD_CONTROLS` `Gain` is in the camera's own units from 0, where rpicam's is an analogue
  multiplier from 1. The node refuses a gain of 0 or less (`crates/scope-node/src/camera.rs:74`,
  and `crates/scope-node/src/config.rs:29` for the config), and automatic gain multiplies
  (`crates/scope-node/src/exposure.rs:34`), so from 0 it never moves. Run on the probe
  with a camera whose range is the device's 0–100 and whose gain is 0: the service sent
  one frame in 30 s and logged `cannot apply camera settings: gain 0 is outside the
  camera's range 0–100` every second (`crates/scope-node/src/service.rs:284`). With the
  range from 1, gain climbed 1, 4, 16, 64, 100. Review round 1 found the same stall
  after a server restart: the client's retry runs on a new connection to a freshly
  started device program, whose gain is its own (0 on the ASI071 by the bench), and the
  frame reported it (run: frames at gain 1 and then 0, and none after). Owner decision 2
  now passes on no frame at a gain the adapter did not send.
- Q: How long can automatic exposure go? → A: The controller caps it at the larger of the
  node's frame interval, 200 ms by default (`crates/scope-node/src/service.rs:58`), and
  the active mode's 1000 / fps (`crates/scope-node/src/service.rs:308`), and at the
  camera's longest. An INDI camera does not stream, so the adapter chooses the fps it
  reports: at 1 frame/s, automatic exposure from 100 ms went 400 ms, then 1000 ms, and
  held there; with no mode the cap is the camera's 2000 s, and it went 1600 ms,
  6400 ms, and so on to 1,638,400 ms by the eighth frame (run). Owner decision 3. With
  `exposure_ms = 1000`, as the Pi's config has it today, the owner gets 1 s exposures
  whatever this decision says.
- Q: What are the frame's times, and what do they cost? → A: The INDI client takes
  `start_s` at the node clock's reading when the device first reports the exposure
  `Busy`, and `end_s` as that plus the exposure (`crates/scope-node/src/imaging/indi/mod.rs:1468`,
  `crates/scope-node/src/imaging/indi/mod.rs:1733`). The capture loop reads the mount's
  axes before and after `next_frame` (`crates/scope-node/src/capture.rs:80`,
  `crates/scope-node/src/capture.rs:82`), and over INDI `next_frame` holds the request,
  the exposure and the download. The console uses the frame's times for the
  mid-exposure time of a live solve and a sync (`crates/finder/src/live.rs:249`,
  `crates/finder/src/session.rs:1554`), for the 0.2°/s still-axes rule of a sync
  (`crates/finder/src/model.rs:258`) and for the first exposure starting after a GoTo
  settles (`crates/finder/src/goto.rs:966`). plan.md § Frame timing states each error in
  arcseconds and pixels at 48″ and 4.8″ a pixel, among them the one device program
  serving both cameras through one stream, which by reading can make `start_s` late by
  up to about 1 s behind an imaging frame; all are estimates, and none is measured on the
  camera, whose bench check (plan.md § Outside the factory) measures them.
- Q: How do two cameras on one server stay apart? → A: With no device named, the INDI
  client takes the first device whose `DRIVER_INTERFACE` has the camera bit
  (`crates/scope-node/src/imaging/indi/mod.rs:760`), so two clients of one server take the
  same camera (run on the probe's stand-in server). Owner decision 4: when the finder
  camera and the imaging camera are both INDI with the same `address`, the config must
  name both devices, and differently. Each client then has its own connection, its own
  device and its own
  BLOBs; on the probe's stand-in their two exposures ran at once, and against a real
  `indiserver` with two simulator cameras both sent frames side by side for 30 s.
- Q: What will the ASI120MM's device be called? → A: Expected `ZWO CCD ASI120MM Mini`:
  the ZWO device program named the ASI071 `ZWO CCD ASI071MC Pro` on the bench on
  2026-11-20. Not verified on this camera; no test or requirement here depends on the
  string, and the README says how to read the real one.
- Q: What happens when `indiserver` restarts? → A: Each client loses its connection. An
  exposure in flight is tried once more 1 s later on a new connection
  (`crates/scope-node/src/imaging/indi/mod.rs:45`, 030's FR-029); when that fails too,
  the finder camera's capture loop logs and tries again every second
  (`crates/scope-node/src/capture.rs:69`), and the imaging camera's next call connects
  again (030's FR-025). On a new connection the client sends only its neutral controls
  (`crates/scope-node/src/imaging/indi/mod.rs:915`), so the retried exposure runs at a
  freshly started device program's own settings. The finder camera's frame at a gain
  the adapter did not send is not passed on, and the next exposure sends the gain
  (FR-004; run: US1-S7, US2-S9); the imaging camera's retried and later exposures run at
  the device program's own (gain 0 and the cooler off on the ASI071 by the bench), a
  follow-up for the owner (plan.md § Outside the factory). Run against a real server
  stopped for 4 s and for 6 s, before the fold: both cameras sent frames again within
  about one exposure and a second of the server's return, the finder camera's gain as
  configured: the server was away longer than the retry's 1 s pause, so each retry
  failed and the gain was sent on the next connection; the retry meets a fresh device
  program when the server is back within that second, as after the udev rule's
  restart. On the Pi a udev rule restarts the server on every ZWO plug-in,
  because the device program finds cameras only when it starts; with two cameras that
  costs the other camera its exposure in flight. Owner decision 6.
- Q: The bench saw the client keep a placeholder exposure range. Is it fixed? → A: Yes,
  in US1, owner decision 5. `indi_asi_ccd` defines `CCD_EXPOSURE` 0–3600 s and at once
  answers with a `setNumberVector` whose `oneNumber` carries `min="3.2e-05" max="2000"`;
  the client reads a range only from `defNumber` (`crates/scope-node/src/imaging/indi/mod.rs:213`)
  and takes only the value from a `setNumberVector` (`crates/scope-node/src/imaging/indi/mod.rs:255`),
  so a freshly started device program's camera reported 0–3,600,000 ms (bench log of
  2026-11-20). The rule that a set vector's `min` and `max` replace the range is read
  from that log; INDI's white paper and libindi's source were not read here (review
  round 1 found `IUUpdateMinMax`, the driver call that sends a number's new range on a
  set vector, exported by the simulator image's libindi 1.9.9). 030's FR-022, "the range of
  `CCD_EXPOSURE_VALUE` in ms" (`specs/030-imaging-camera-link/spec.md:1391`), then reads
  the latest range; its text does not change.
- Q: What rests on a run, and what on reading? → A: Every scenario was run on the probe
  with the trial build of plan.md § Interfaces: before the fold both gates' Rust commands
  passed there, and in the fold the node's lint and tests passed at its last commit,
  which touched the node alone. The lead runs, the real-server run and the fold's runs
  are listed in plan.md § What was run and what was read. Nothing was run on the
  ASI120MM, the ASI071, a Pi or `indi_asi_ccd`.
- Q: What is left for later? → A: The items under "Out of scope" in § Assumptions.

### Session 2026-11-25 (the owner's answers)

- Q: Owner decisions 1 to 8, each the recommended option (a) as review round 1 amended
  it? → A: The owner, at 19:21 UTC: "agree with all of your recommendations".

## Owner decisions

All eight are answered: the owner agreed with every recommendation on 2026-11-25 at
19:21 UTC (§ Clarifications), each the recommended option as review round 1 amended it.
Each keeps its options, its evidence and its recommendation, the smallest design that
works; the scenarios are written to the answers.

1. **Turning 16-bit samples into 8** (FR-003). **Answered: (a), for the finder camera
   only, with (e) as the lever on the offset level.** Recommended: (a) the high byte of
   each 16-bit sample, and an 8-bit sample as it is. The camera's gain brings faint stars
   up, as it does on rpicam's 8-bit frames: the node never stretches. Measured on
   synthetic 1280 × 960 frames (150 stars, peaks log-uniform from twice the noise to
   1500, 12-bit units, at an offset level of 64 in 12-bit units, four 8-bit levels, two
   seeds each), stars found of 150 with cedar's settings, none false in any case, with no
   noise floor and with 048's `noise_floor = 0.5`: at noise 1 (a sixteenth of an 8-bit
   level after the high byte), high byte 57 and 70, at the floor 48 and 64, shift by 6
   bits 90 and 101, at the floor 84 and 95, stretch 112 and 126 at either; at noise 3,
   high byte 78 and 91, at the floor 63 and 81, shift by 6 bits 102 and 106 at either,
   stretch 110 and 120 at either; at noise 10 (0.6 of a level), high byte 90 and 102, at
   the floor 85 and 96, shift by 6 bits 100 and 102 at either, stretch 99 and 106 at
   either. Review round 1 ran the high byte again at an offset level of 3 in 12-bit
   units, as a ZWO camera at its own low `Offset` sits (the ASI071's dark read 44.1 of
   65535 at `Offset` 8 on the bench, a high byte of 0): the high byte then zeroes 89 to
   99.9 % of the background, yet found 68 and 82 (noise 1), 84 and 95 (noise 3) and 107
   and 124 (noise 10) with no floor, and 48 and 65, 64 and 81, 86 and 97 at the floor,
   none false.
   The frames are Gaussian: they hold no hot pixel or row pattern, the ingredients of
   048's real night on a background clipped to 0. Weighed against: (b) RAW8 on the
   camera: the same top 8 bits by reading, half the download, but a switch the client
   does not send today and a property not seen on this camera; the bench check can show
   it, as a later change. (c) A per-frame stretch, black at the median, 4 levels a noise
   sigma: the most stars at low noise, but each frame's levels differ, 048's floor (in
   8-bit levels) loses its meaning, the preview's brightness moves frame to frame, and
   stars above about 60 sigma saturate, losing the brightness order the solver ranks by.
   (d) A fixed shift by 6: four times the high byte, saturating at a quarter of full
   scale. (e) With (a), the camera's `Offset` raised through `[indi] neutral` (decision
   4 (e)): the only lever on the offset level, which the client already sets on each
   connection; its value is the bench check's (plan.md § Outside the factory, step 6).
2. **Gain units** (FR-002, FR-004). **Answered: (a), with no frame passed on at a gain
   the adapter did not send.** Recommended: (a) the camera's own units, its range as the
   device gives it with the minimum raised to 1, and the camera's own gain raised to 1
   before the first exposure when it is below; and no frame passed on at a gain the
   adapter did not send: a frame whose gain is not the camera's last answer fails with
   `the finder camera exposed at gain <g>, not at the gain <s> sent`, and the next
   exposure sends the gain again. Review round 1 found why: after a server restart the
   client's retry runs on a new connection to a freshly started device program at its
   own gain, 0 on the ASI071 by the bench; automatic gain took that 0 on, `apply`
   refused it, and the service stalled for good (run). One step of a ZWO gain is 0.1 dB
   on the ASI071 by the bench (gain 60 gave 1.99 times the signal); the ASI120MM's step
   is not known here. Automatic gain then multiplies on a logarithmic scale, so its steps
   are fine near 1 and coarse near the top: from 1 with no star, 1, 4, 16, 64 and 100,
   where ×4 from 1 is about 1.035 times the signal at 0.1 dB a step and 16 to 64 about
   1.74 times. Weighed against: (b) the range from 0 as the device gives it: the service
   stalls on its own gain of 0 (run: one frame in 30 s); (c) a linear multiplier made from
   decibels: needs the camera's step, which INDI does not give, and a value from a
   camera's model, which 030's FR-022 forbids; (d) fixed gain only over INDI: refuses the
   console's default of automatic gain, a console change. For the frame after a restart
   (review round 1's missed option, every frame's gain checked against the gain sent,
   measured on the probe with automatic gain over star frames, plan.md § Decisions): the
   check with the exposure taken again at once (the second star frame 1 s sooner, but
   the axes the capture loop reads then span two exposures and a reconnection); the gain
   sent before every exposure with no check (still a frame at gain 0, since the retry
   runs inside the client, and the stall); the client putting its caller's settings back
   on a new connection (both cameras' client, so the imaging side's follow-up).
3. **The longest automatic exposure** (FR-002, FR-003). **Answered: (a).** Recommended:
   (a) the one mode reports 1 frame/s, so automatic exposure goes no longer than 1000 ms
   (or the frame interval, if longer), and the first automatic exposure is 1000 ms; a
   fixed `exposure_ms` overrides. At 4.8″ a pixel a 1 s exposure on the untracked Dob
   smears about 3 px near the equator; the console's live pointing goes stale 5 s after
   its mid-exposure (`crates/finder/src/pushto.rs:184`), which by reading exposures up to
   about 3 s avoid. Weighed against: (b) 0.5 frame/s, 2000 ms; (c) no mode, the camera's
   longest (run: 1,638,400 ms by the eighth frame without stars); (d) a config key: a key
   for the owner to set per optic.
4. **The config's shape** (FR-007, FR-008). **Answered: (a) with (e).** Recommended: (a)
   `adapter = "indi"` and an optional `[indi]` table of `address` (INDI's default
   127.0.0.1:7624) and `device`, as `[imaging]` has them; when `[imaging]` is also `indi`
   with the same `address`, both devices must be named and must differ, checked where the
   config is validated, so the node and `--check-config` refuse it alike. The rule
   compares the two addresses, not the server they reach: the IPv6 and the IPv4
   loopback on one port, or the scope node's own network address and loopback, pass
   while reaching one server, so the README says to give both tables the same `address`. With (e): `[indi]` also takes
   `neutral`, as `[imaging]` does (say `CCD_CONTROLS.Offset`), which the client already
   sets on each connection: the only lever on the finder camera's offset level (decision
   1). Measured on the probe: about 2.9 KB of US2's diff (the config's 1.9 KB, the
   binary's one line, the tests' 0.7 KB, the README's 0.3 KB) and no existing test
   changed. Weighed against: (b) flat keys (`indi_address`, `indi_device`) beside `mode`:
   no table, but two keys that mean nothing to the other adapters; (c) `device` always
   required: refuses the one-camera case that works today on the first camera; (d) no
   rule, a README warning: two clients of one server take the same first camera (run).
5. **The client's range from a set vector** (FR-001). **Answered: (a).** Recommended: (a)
   fix it here, in US1: a `min` or `max` on a `oneNumber` of a `setNumberVector` replaces
   the element's range, and one that does not parse is kept. The finder camera needs the
   real exposure range: from a placeholder minimum of 0, automatic exposure could fall
   below the camera's 32 µs, and each such exposure would fail. The imaging camera's
   description changes the same way on a freshly started device program, which is the
   bench's finding. No existing test changes: the saved capture's set vectors carry
   ranges only for `CCD_FRAME`, `CCD_STREAM_FRAME` and `GUIDER_FRAME`, which no
   description reads (plan.md § Shapes that existing tests pin). Weighed against: (b)
   leave it: the finder camera's exposure range stays 0–3600 s on a freshly started device
   program; (c) a spec of its own: a second epic for 13 lines.
6. **The udev rule** (outside the factory, plan.md § Outside the factory). **Answered:
   (a), with the full cost of a restart stated; the imaging camera's settings after a
   reconnection are the owner's follow-up spec.** Today the Pi restarts `indiserver` on
   every ZWO plug-in. Recommended: (a) decide at the bench check: if `indi_asi_ccd` 2.2
   finds a camera plugged in after it starts, drop the rule; otherwise keep it, knowing
   what each restart costs. The finder camera loses its exposure in flight and the frame
   the retry brings at the device program's own gain, and then sends star frames again
   at its gain, about 1 s later than an exposure taken at once (US1-S7, US2-S7, US2-S9).
   The imaging camera loses its exposure in flight, and its retried exposure and those
   after it run at the device program's own settings, gain 0 and the cooler off on the
   ASI071 by the bench, because 030's client sends only the neutral controls on a new
   connection (`crates/scope-node/src/imaging/indi/mod.rs:915`) and its retry's path
   re-applies nothing else (`crates/scope-node/src/imaging/indi/mod.rs:1066` to `crates/scope-node/src/imaging/indi/mod.rs:1075`).
   A plug-in mid-run (a cable fault) costs both. Weighed against: (b) keep it as is,
   unchecked; (c) drop it and plug both cameras in before the Pi starts; (d) a rule that
   restarts only when no exposure is in flight: udev has no such state.
7. **A saved capture of the real camera** (US1). **Answered: (a).** Recommended: (a) a
   follow-up: the hand-written finder script stands for the camera, its values marked as
   expected, and after the bench check the operator stages the camera's own definitions
   as a data-only swap through a `fixtures.json`, with the owner's OK, as test data.
   Weighed against: (b) a prerequisite: the flip waits for the camera on the bench and a
   capture staged first.
8. **Starting without the camera** (FR-010). **Answered: (a), with
   `StartLimitIntervalSec=0` on the node's unit.** Recommended: (a) as rpicam: a camera
   the node cannot describe ends the start, exit 1, and systemd starts the node again a
   second later, with `StartLimitIntervalSec=0` in the unit's `[Unit]`, since by default
   systemd stops starting a unit after 5 starts within 10 s (§ Clarifications); the mount
   and the imaging camera are not served meanwhile, as with a missing ribbon camera
   today. Weighed against: (b) wait at start, retrying every second, the node serving
   nothing meanwhile all the same, or only for the server that cannot be reached, keeping
   (a) for a missing device or a colour camera; (c) serve without a description: a change
   to the service's one reading of `info` and to what the console is told.

## Owner approvals

Flipping this spec to `ready` approves the answer to every owner decision above, and the
following. Each is named in the FR that makes it. No existing test, golden, fixture or CI
file changes.

1. **New public API in the node's library** (FR-002, FR-007): the module
   `scope_node::indi_finder` with `IndiFinder`, `gray`, `FRAME_MS` and `PURPOSE`;
   `config::Adapter::Indi`; `config::FinderIndiConfig` with its `neutral` and
   `neutral_controls`, and `Config::indi`. No existing signature changes.
2. **The imaging camera's description changes on a freshly started ZWO device program**
   (FR-001): its exposure range is the device's answer, 0.032 to 2,000,000 ms on the
   ASI071 by the bench, where it was 0 to 3,600,000 ms.
3. **The node refuses a colour camera as the finder camera** (FR-002): a device with a
   colour mosaic ends the start with its name and pattern.
4. **The README's new section** (FR-011), with the node unit's `StartLimitIntervalSec=0`.

## User Scenarios & Testing *(mandatory)*

Every scenario shares these definitions.

- **The gate command.** Every scenario runs in
  `cargo nextest run --workspace --all-targets --all-features --locked`, the `test`
  gate's first command.
- **The stand-in server** is `Scripts` of `crates/scope-node/tests/support/mod.rs:577`,
  with its builders (`def_number`, `set_number` and the rest), its colour-camera script
  `colour()` (`crates/scope-node/tests/support/mod.rs:835`, device `ZWO CCD ASI071MC Pro`)
  and its mono-camera script `mono()` (`crates/scope-node/tests/support/mod.rs:956`,
  device `CCD Simulator`), each connection playing the next script and `finish` failing
  on any mismatch, any script not played to its end, or any element no `@ expect` took.
  It is used, not edited.
- **The shared test module** `crates/scope-node/tests/finder_script/mod.rs` is US1's,
  new: it holds the finder script, the gain step, the exposures, the FITS writer, the
  worked 16-bit frame, the test clock and the finder rig's camera, which both stories'
  tests use. US2's test file uses it and does not edit it.
- **The test clock** reads 1000.0 plus the tokio time elapsed since it was built;
  **reading(ms)** is its reading `ms` milliseconds after. US1's tests and US2-S7 to
  US2-S9 run under `#[tokio::test(start_paused = true)]`.
- **The finder script at gain g** is the server's side of the camera, written by the test:
  `@ device Finder CCD` (a name of the test's own: no test uses a real device program's
  name), `@ expect getProperties @version=1.7`, then `CONNECTION` (`CONNECT` Off,
  `DISCONNECT` On) and `DRIVER_INFO` with `DRIVER_INTERFACE` 2;
  `@ expect newSwitchVector CONNECTION CONNECT=On`; then `CONNECTION` answered Ok with
  `CONNECT` On, `CCD_EXPOSURE` defined with `CCD_EXPOSURE_VALUE` 1 of 0–3600, at once a
  `setNumberVector` of `CCD_EXPOSURE`, state Ok, whose `oneNumber` is
  `<oneNumber name="CCD_EXPOSURE_VALUE" min="3.2e-05" max="2000" step="1">1</oneNumber>`,
  `CCD_ABORT_EXPOSURE`, `CCD_FRAME` 0, 0, 1280, 960, `CCD_INFO` 1280, 960, 3.75 µm and
  16 bits, `CCD_CONTROLS` with `Gain` g of 0–100 and `Offset` 10 of 0–100,
  `CCD_FRAME_TYPE` (`FRAME_LIGHT` On, `FRAME_DARK` Off) and the BLOB `CCD1`; then
  `@ expect after 200 enableBLOB text=Also`.
- **The gain step g** is `@ expect newNumberVector CCD_CONTROLS Gain=g Offset=10`, then a
  `setNumberVector` of `CCD_CONTROLS`, state Ok, `Gain` g and `Offset` 10.
- **An exposure of s seconds with a frame** is
  `@ expect newSwitchVector CCD_FRAME_TYPE FRAME_LIGHT=On`,
  `@ expect newNumberVector CCD_EXPOSURE CCD_EXPOSURE_VALUE=s`, then 200 ms later
  `CCD_EXPOSURE` Busy, s later the frame's FITS as the `.fits` BLOB of `CCD1` in standard
  base64, then `CCD_EXPOSURE` Ok. **The next exposure's two vectors** are the first two
  lines of that, unanswered.
- **The frames.** **The worked 16-bit frame** is a FITS image of 4 × 2 samples, `BITPIX`
  16, `BZERO` 32768, whose samples are 0x0000, 0x0010, 0x00FF, 0x0100, 0x1230, 0x8000,
  0xFFF0 and 0xFFFF. **The flat frame** is 64 × 48 samples of 16 bits, every one 0x1000:
  it holds no star. **An 8-bit frame of v** is 3 × 2 samples, `BITPIX` 8, every one v;
  **the 8-bit ramp** is 3 × 2 samples 0, 1, 2, 253, 254, 255. **The star field** is the
  simulated finder camera's frame at its default settings (`SyntheticCamera` of
  `crates/scope-node/src/synthetic.rs:9`, 640 × 480, 8 bits), taken by the test before
  its rig is built. Each is written by the tests' own FITS writer, as
  `crates/scope-node/tests/indi_expose.rs:87` builds its frames.
- **The finder rig** is `IndiFinder::discover` of an `IndiCamera::new` over the stand-in's
  connector, the address text 127.0.0.1:7624, the device the scenario names, no
  neutral controls and the test clock, with `CameraMode::Auto` unless the scenario names a
  size, unwrapped unless the scenario awaits its failure. **Taking a frame** is
  `next_frame` awaited, after `apply` of the scenario's settings.
- **The service rig** is `NodeService::new` of a finder rig, the test clock, the token
  `test-token` and the scenario's settings, its routes served over
  `scope_link::memory::channel_pair` and its star frames read with
  `ScopeNodeClient::frames`, as `crates/scope-node/tests/raw_frame.rs:29` builds its rig.
- **The node rig** (US2) is the built `scope-node`, `env!("CARGO_BIN_EXE_scope-node")`, on
  a config file under `CARGO_TARGET_TMPDIR` that names the test case and the process id,
  with standard input null and standard output and error piped, killed after at most
  10 s.
- **Oracles.** The scripts' own definitions and answers, the frames' bytes the tests
  write, the INDI client's documented waits (`SETUP_WAIT` 5 s, `SETTLE` 200 ms,
  `RETRY_PAUSE` 1 s: `crates/scope-node/src/imaging/indi/mod.rs:40` to `crates/scope-node/src/imaging/indi/mod.rs:45`) and the
  capture loop's 1 s (`crates/scope-node/src/capture.rs:70`), the process's exit status
  and standard error, and the words of these scenarios. No expected value comes from the
  code under test: no test calls `gray` or `FRAME_MS` for an expected value or formats an
  expected message from the code. The 25 centroids of US2-S9 are the controller's band
  (`crates/scope-node/src/exposure.rs:26`), a condition of the scenario, not a count
  computed for it.

### User Story 1 - The finder camera is described and framed over INDI (Priority: P1)

As the observer, I fit the ASI120MM as the finder camera on the Pi 5, and the node
describes it from the camera's own properties and gives detection 8-bit frames of it at
the exposure and gain I set or leave automatic, never at a gain it did not send, and
keeps doing so when the INDI server restarts.

**Why this priority**: Until 045 lands the Pi 5 has no finder camera.

**Independent Test**: `crates/scope-node/tests/indi_finder.rs`, new, over the shared test
module `crates/scope-node/tests/finder_script/mod.rs`, new. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p scope-node --locked --test indi_finder`.

**Acceptance Scenarios**:

1. **Given** the finder rig naming `Finder CCD` over the finder script at gain 0, **When**
   a committed test reads its `info`, **Then** it asserts that it equals a `CameraInfo` of
   model `Finder CCD`, width 1280, height 960, `modes` one mode of 1280 × 960 at 1 fps,
   `active_mode` that mode, `exposure_min_ms` 0.032, `exposure_max_ms` 2000000,
   `gain_min` 1 and `gain_max` 100, and that the script plays to its end.
2. **Given** the finder rig naming `Finder CCD` over the finder script at gain 0, the gain
   step 1 and an exposure of 1 s with the worked 16-bit frame, **When** a committed test
   takes a frame with default settings, **Then** it asserts that its pixels are 0x00,
   0x00, 0x00, 0x01, 0x12, 0x80, 0xFF and 0xFF, that it is 4 by 2, that its `exposure_ms`
   is 1000 and its `gain` 1, that its `start_s` is reading(400) and its `end_s`
   reading(400) plus 1.0, exactly, and that the script plays to its end.
3. **Given** the finder rig naming `Finder CCD` over the finder script at gain 50, the gain
   step 30, two exposures of 0.5 s with the 8-bit ramp, the gain step 30 and a third
   exposure of 0.5 s with the 8-bit ramp, **When** a committed test takes two frames with
   `exposure_ms` 500 and `gain` 30, calls `restart` and takes a third, **Then** it asserts
   that each frame's pixels are 0, 1, 2, 253, 254 and 255 with `exposure_ms` 500 and
   `gain` 30, and that the script, which holds a gain step before the first and the third
   exposures only, plays to its end.
4. **Given** the finder rig naming `Finder CCD` over the finder script at gain 0, **When**
   a committed test applies `gain` 101, then `gain` 0.5, then `exposure_ms` 0.01, each
   alone, **Then** it asserts that each is refused with exactly
   `gain 101 is outside the camera's range 1–100`,
   `gain 0.5 is outside the camera's range 1–100` and
   `exposure_ms 0.01 ms is outside the camera's range 0.032–2000000 ms`, and that the
   script, which expects nothing after `enableBLOB`, plays to its end.
5. **Given** four discoveries: naming `ZWO CCD ASI071MC Pro` over the colour-camera
   script, naming `Finder CCD` over the finder script at gain 0 with the mode 640 × 480,
   naming `Finder CCD` over the finder script at gain 0 whose answer's
   `min="3.2e-05" max="2000"` reads `min="3000" max="2000"`, and naming `Finder CCD` over
   a stand-in with no script, **When** a committed test awaits each, **Then** it asserts
   that they fail with exactly
   `the INDI camera ZWO CCD ASI071MC Pro is a colour camera (RGGB); the finder camera must be mono`,
   `camera has no 640x480 mode; it lists 1280x960`,
   `camera has invalid exposure_ms range` and
   `cannot reach the INDI server at 127.0.0.1:7624: no script left to play`, and that the
   first three scripts play to their end.
6. **Given** a discovery naming `Other CCD` over the finder script at gain 0 with its
   device line made `@ device Other CCD` and its expectations of `CONNECT` and of
   `enableBLOB` removed, **When** a committed test awaits it, **Then** it asserts that it
   fails exactly 5 s after it began with
   `the INDI server at 127.0.0.1:7624 has no device Other CCD`, and that the script plays
   to its end.
7. **Given** the finder rig naming `Finder CCD` over two scripts, the first the finder
   script at gain 0, the gain step 30, an exposure of 0.5 s with the worked 16-bit frame
   and the next exposure's two vectors followed by `@ close`, the second the finder script
   at gain 0, an exposure of 0.5 s with the worked 16-bit frame, the gain step 30 and an
   exposure of 0.5 s with the worked 16-bit frame, **When** a committed test takes three
   frames with `exposure_ms` 500 and `gain` 30, **Then** it asserts that the first frame's
   gain is 30, that the second fails with exactly
   `the finder camera exposed at gain 0, not at the gain 30 sent`, that the third's gain
   is 30, that the stand-in made 2 connections, and that both scripts play to their end.
8. **Given** two `IndiCamera`s naming `Finder CCD`, one over the finder script at gain 0
   and one over the finder script at gain 0 whose answer's `min="3.2e-05"` reads
   `min="x"`, **When** a committed test calls `describe` on each, **Then** it asserts
   that the first's `exposure_ms` range is 0.032 to 2000000 and the second's 0 to
   2000000, and that both scripts play to their end.
9. **Given** a finder rig naming `CCD Simulator` over the mono-camera script whose first
   `@ send` block starts with the colour-camera script's first block of lines, then
   `@ expect newNumberVector CCD_EXPOSURE CCD_EXPOSURE_VALUE=1`, 100 ms later `CCD_EXPOSURE`
   Busy and 1 s later an 8-bit frame of 7, and an `IndiCamera` naming
   `ZWO CCD ASI071MC Pro` over its own stand-in playing the colour-camera script whose
   first `@ send` block starts with the mono-camera script's, then the expectations of
   `FRAME_LIGHT` and of `CCD_EXPOSURE_VALUE=1`, 100 ms later Busy and 1 s later an 8-bit
   frame of 9, the imaging camera described, **When** a committed test takes a frame with
   default settings and exposes the imaging camera for 1000 ms with the purpose `light`,
   both at once, **Then** it asserts that both answer 1100 ms after they began, that the
   finder camera's pixels are six 7s and the imaging frame's samples six 9s, that the two
   descriptions name their own devices, and that both scripts, each of whose `@ device`
   lines names its own device, play to their end.

### User Story 2 - The node runs the finder camera over INDI beside the imaging camera (Priority: P1)

As the observer, I set `adapter = "indi"` in the node's config, name the ASI120MM's
device beside the ASI071's, and the node serves star frames from the one and exposures
from the other through one INDI server; a config that would put both on one camera is
refused before the node starts.

**Why this priority**: Without it US1's adapter is not reachable from the config, and two
INDI cameras of one server take the same camera.

**Independent Test**: `crates/scope-node/tests/indi_finder_node.rs`, new, over US1's
shared test module. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p scope-node --locked --test indi_finder_node`.
US2-S1 to US2-S6 prove US2's own diff. US2-S7 to US2-S9 run US1's adapter inside the
node's service, which the node serves from US2 on: they pass on US1 alone, and guard
at US2's merge what US1 built (US1-S3 and US1-S7 hold the same rules on the adapter
alone).

**Acceptance Scenarios**:

1. **Given** a config of `listen`, the token `test-token` and `adapter = 'indi'`, and the
   same with an `[indi]` table of `address = '127.0.0.1:7700'`, `device = 'Finder CCD'`
   and `neutral = { 'CCD_CONTROLS.Offset' = 20 }`, **When** a committed test parses each
   with `Config::parse`, **Then** it asserts that the first's adapter is `Adapter::Indi`
   and its `indi` is `None`, and that the second's `indi` has the address 127.0.0.1:7700,
   the device `Finder CCD` and the neutral controls `CCD_CONTROLS`, `Offset`, 20 alone.
2. **Given** configs of that head with `adapter = 'rpicam'` and `[indi] device = 'Finder CCD'`,
   with `adapter = 'indi'` and `[indi] device = ''`, with `adapter = 'indi'` and
   `[indi] neutral = { 'Offset' = 20 }`, and with `adapter = 'indi'` and
   `[indi] neutral = { 'CCD_CONTROLS.Offset' = nan }`, **When** a committed test parses
   each, **Then** it asserts that they fail with exactly
   `the [indi] table applies only to adapter = "indi"`,
   `indi.device must not be empty`,
   `indi.neutral key Offset must be PROPERTY.ELEMENT` and
   `indi.neutral CCD_CONTROLS.Offset must be a finite number`.
3. **Given** configs of that head with `adapter = 'indi'` and an `[imaging]` table of
   `adapter = 'indi'`: with `[indi] device = 'Finder CCD'` and no imaging device; with no
   `[indi]` table and `imaging.device = 'Imaging CCD'`; with both devices `Same CCD`; with
   `imaging.address = '127.0.0.1:7625'`; with the devices `A` and `B`; and one with
   `[imaging] adapter = 'simulated'`, **When** a committed test parses each, **Then** it
   asserts that the first two fail with exactly
   `the finder camera and the imaging camera share the INDI server at 127.0.0.1:7624: set indi.device and imaging.device`,
   that the third fails with exactly
   `indi.device and imaging.device both name Same CCD: the finder camera and the imaging camera need a device each`,
   and that the last three parse.
4. **Given** the node rig on a config of that head with `adapter = 'indi'`,
   `exposure_ms = 1000`, `gain = 30` and `[indi]` of `device = 'Finder CCD'` and
   `neutral = { 'CCD_CONTROLS.Offset' = 20 }`, and on one with `adapter = 'simulated'`,
   each run with `--check-config`, **When** a committed test reads each one's exit and
   standard output, **Then** it asserts that the first exits 0, that its output holds
   `adapter = "indi"` and
   `\n[indi]\naddress = "127.0.0.1:7624"\ndevice = "Finder CCD"\nneutral = { "CCD_CONTROLS.Offset" = 20 }\n`
   and not `test-token`, and that the second's output holds no `[indi]`.
5. **Given** the node rig on a config of that head with `adapter = 'indi'` and
   `[indi] address` a `127.0.0.1` port on which nothing listens, **When** a committed test
   waits for it to exit, **Then** it asserts that it exits within 5 s with status 1 and
   that the last line of its standard error is
   `scope-node: cannot reach the INDI server at 127.0.0.1:<port>: Connection refused (os error 111)`.
6. **Given** a TCP listener on a free `127.0.0.1` port whose one connection the test
   bridges to the stand-in playing the mono-camera script with its
   `@ expect after 200 enableBLOB` made `@ expect enableBLOB`, then
   `@ expect newNumberVector CCD_OFFSET OFFSET=20` answered at once with `CCD_OFFSET`,
   state Ok, `OFFSET` 20, then
   `@ expect newNumberVector CCD_EXPOSURE CCD_EXPOSURE_VALUE=0.1`, 100 ms later
   `CCD_EXPOSURE` Busy, 100 ms later the 8-bit ramp as the BLOB of `CCD1`, and the next
   `CCD_EXPOSURE_VALUE=0.1`, and the node rig on a config of a free `listen` port, the
   token, `adapter = 'indi'`, `exposure_ms = 100`, `gain = 90` and `[indi]` of `address`
   that listener and `neutral = { 'CCD_OFFSET.OFFSET' = 20 }`, **When** a committed test
   calls `Describe` and reads the first star frame over the node's port within 10 s,
   waits 1 s and kills the node, **Then** it asserts that the description is model
   `CCD Simulator`, 1280 by 1024, one mode of 1280 × 1024 at 1 fps, active, exposure 10
   to 3600000 ms and gain 1 to 300; that the frame is 3 by 2 with `exposure_ms` 100 and
   `gain` 90, its `end_s` 0.1 after its `start_s` within 1e-9; and that the script, which
   holds the neutral vector and no gain vector, plays to its end.
7. **Given** the service rig at `exposure_ms` 500 and `gain` 30 over a finder rig naming
   `Finder CCD` whose connector refuses its 2nd, 3rd and 4th dials with
   `ConnectionRefused` and serves the 1st and the 5th from two scripts, the first the
   finder script at gain 0, the gain step 30, an exposure of 0.5 s with the flat frame and
   the next exposure's two vectors followed by `@ close`, the second the finder script at
   gain 0, the gain step 30, an exposure of 0.5 s with the flat frame and the next
   exposure's two vectors, **When** a committed test reads two star frames, each within
   60 s of paused time, **Then** it asserts that the first has id 1, gain 30 and
   `start_s` reading(400), that the second has id 2, gain 30 and `start_s`
   reading(5300), that the connector was dialled 5 times, and that both scripts play to
   their end.
8. **Given** the service rig with default settings over a finder rig naming `Finder CCD`
   over the finder script at gain 0 followed, for each g of 1, 4, 16, 64 and 100, by the
   gain step g and an exposure of 1 s with the flat frame, and then the next exposure's
   two vectors, **When** a committed test reads five star frames, each within 60 s of
   paused time, and then waits 100 ms, **Then** it asserts that their `exposure_ms` and
   `gain` are 1000 and 1, 1000 and 4, 1000 and 16, 1000 and 64, and 1000 and 100, and
   that the script, whose sixth exposure has no gain step before it, plays to its end.
9. **Given** the service rig at `exposure_ms` 500 with automatic gain over a finder rig
   naming `Finder CCD` over two scripts, the first the finder script at gain 0, the gain
   step 1, an exposure of 0.5 s with the star field and the next exposure's two vectors
   followed by `@ close`, the second the finder script at gain 0, an exposure of 0.5 s
   with the star field, the gain step 1, an exposure of 0.5 s with the star field and the
   next exposure's two vectors, **When** a committed test reads two star frames, each
   within 60 s of paused time, and then waits 100 ms, **Then** it asserts that each holds
   at least 25 centroids, that the first has id 1, gain 1 and `start_s` reading(400),
   that the second has id 2, gain 1 and `start_s` reading(4000), and that both scripts
   play to their end.

### Edge Cases

- **A camera with no gain control** reports gain 1 to 1, is sent no gain, and its frames
  report gain 1 (FR-002, FR-003). Read, not run.
- **A camera whose longest exposure is under 1000 ms** takes its longest as the first
  automatic exposure (FR-003). Read, not run.
- **A fixed gain of 0** stays refused, by the config (`crates/scope-node/src/config.rs:29`)
  and by the console (`crates/finder/src/session.rs:1899`), as for rpicam; the camera's
  0.1 dB (on the ASI071) below gain 1 is out of reach.
- **The camera's own gain below 1 with automatic gain**: the first exposure raises it to 1
  (US1-S2), and after a server restart the frame the client's retry brings at a freshly
  started device program's own gain is not passed on, the next exposure sending the gain
  again (US1-S7, US2-S9), so no star frame reports a gain the node would refuse; the
  console's drawn sky takes frames of a gain above 0 only
  (`crates/finder/src/mount_sky.rs:119`).
- **Another INDI client changes the camera's gain**: the next frame at that gain is not
  passed on, and the next exposure sends the adapter's gain again (FR-004). **Its region
  or binning**: the frames take the size the camera sends; the description stays as read
  at start. Read, not run.
- **A named device that is not a camera** fails the start after 10 s with
  `the INDI camera <name> did not come ready within 10 s`
  (`crates/scope-node/src/imaging/indi/mod.rs:875`): run against a real server, not in a
  scenario.
- **One INDI camera, no device named**: the node takes the first device with the camera
  bit, as the imaging camera does (run against a real server whose first device was a
  mount).
- **The server restarts while the imaging camera exposes**: the exposure is tried once
  more on a new connection and fails `the exposure failed twice: ...` when the server is
  still down (run); when it is back, the retried exposure and those after it run at the
  device program's own settings (by reading; a follow-up, plan.md § Outside the factory);
  the console's imaging run then sees that failure or those frames (not run).
- **Exposures over about 3 s, fixed**: by reading, the console's live pointing can go
  stale between frames (`crates/finder/src/pushto.rs:184`); the node takes them as set.
- **The dark-sky floor** (048) applies to these 8-bit frames as to rpicam's. With the high
  byte, a camera at its own low `Offset` can leave the background at 0; `[indi] neutral`
  raises the camera's `Offset` (owner decision 4 (e)). Read, not run on the camera.

## Requirements *(mandatory)*

### Functional Requirements

**The client and the adapter** (US1):

- **FR-001**: The INDI client's record of a `setNumberVector`
  (`crates/scope-node/src/imaging/indi/mod.rs:241`) SHALL take a `oneNumber`'s `min` and
  `max` attributes, where present and parsed as numbers, as that element's new range, and
  keep the range it had where they are absent or do not parse; the value SHALL be taken
  as today (owner decision 5).
- **FR-002**: `crates/scope-node` SHALL gain the module `indi_finder` with
  `pub struct IndiFinder<C>` and
  `pub async fn discover(camera: C, mode: CameraMode) -> Result<Self, CameraError>` for
  `C: ImagingCamera`, which SHALL describe the camera once and answer, as the port's
  `info`: the device's name as the model; its width and height; one mode of that size at
  `1000 / FRAME_MS` fps, `FRAME_MS` being 1000, as `modes` and `active_mode`; the
  device's exposure range in ms; and its gain range with each end raised to at least 1,
  or 1 to 1 without a gain control (owner decisions 2 and 3). It SHALL fail with the
  description's own error text, with
  `the INDI camera <model> is a colour camera (<pattern>); the finder camera must be mono`
  for a device with a colour mosaic, with
  `camera has no <w>x<h> mode; it lists <width>x<height>` for a mode size other than the
  sensor's (owner approval 3), and with `camera has invalid <name> range`, `<name>` being
  `exposure_ms` or `gain`, for a range whose ends are not finite or whose maximum is under
  its minimum.
- **FR-003**: `IndiFinder` SHALL implement the camera port. Each `next_frame` SHALL take
  one exposure through the imaging port with the purpose `finder`, at the settings'
  exposure or, with automatic exposure, at `FRAME_MS` limited to the camera's range; its
  frame SHALL carry the exposure's size, `start_s`, `end_s` and `exposure_ms`, its gain as
  the camera reported it, which FR-004 makes the gain sent (1 without a gain control),
  and as pixels each 16-bit sample's high byte or each 8-bit sample as it is (owner
  decision 1); `pub fn gray` SHALL make those pixels and fail on another sample size or
  a sample count other than width times height.
- **FR-004**: Before an exposure, when the gain wanted, the settings' gain or with
  automatic gain the camera's last answered gain limited to the reported range, differs
  from the camera's last answered gain, the adapter SHALL send it through the imaging
  port's `apply` and await its answer, whose gain becomes the last answered gain. No frame
  SHALL be passed on at a gain the adapter did not send: when the camera has a gain
  control and a frame's gain is not the last answered gain, `next_frame` SHALL fail with
  `the finder camera exposed at gain <g>, not at the gain <s> sent`, and the last
  answered gain SHALL be none, as after `restart`, so the next exposure sends the gain
  again (owner decision 2).
- **FR-005**: `IndiFinder::apply` SHALL refuse settings outside the reported ranges with
  `check_range` (`crates/scope-node/src/camera.rs:62`, 013's FR-004) and send nothing.
- **FR-006**: Nothing else SHALL change in US1: the imaging port and its types, the
  signatures of `IndiCamera`, the scope link's protocol, `camera.rs`, `capture.rs`,
  `service.rs`, `exposure.rs`, `detect.rs`, `rpicam/`, `synthetic.rs`, `crates/finder`, and
  every existing test and fixture. A frame FR-004 refuses takes the capture loop's
  existing failure path: `camera capture failed: ` and its text, then 1 s and `restart`
  before the next frame (`crates/scope-node/src/capture.rs:69` to `crates/scope-node/src/capture.rs:77`).

**The node** (US2):

- **FR-007**: The config's `adapter` SHALL also take `indi` (013's FR-007), and the config
  SHALL take an optional `[indi]` table of `address` (by default 127.0.0.1:7624),
  `device` and `neutral` (controls set on each connection, keyed `PROPERTY.ELEMENT`, as
  `[imaging]`'s), refusing unknown keys as every table does, and refusing with
  `the [indi] table applies only to adapter = "indi"` a table with another adapter, with
  `indi.device must not be empty` an empty device, with
  `indi.neutral key <key> must be PROPERTY.ELEMENT` a key that does not split into two
  names, and with `indi.neutral <key> must be a finite number` a value that is not finite
  (owner decision 4).
- **FR-008**: When the adapter is `indi` and `[imaging]` is `indi` with the same
  `address`, the config SHALL be refused unless both devices are named and differ, with
  `the finder camera and the imaging camera share the INDI server at <address>: set indi.device and imaging.device`
  or `indi.device and imaging.device both name <name>: the finder camera and the imaging camera need a device each`
  (owner decision 4).
- **FR-009**: `--check-config` SHALL print `adapter = "indi"` for the adapter and, for
  `indi` only, `[indi]` after the `[detection]` table with `address`, its default
  included, `device` when set and `neutral` when not empty, as `[imaging]` prints it; its
  output for `rpicam` and `simulated` SHALL not change.
- **FR-010**: The binary SHALL build, for `adapter = "indi"`, `IndiFinder::discover` of
  `IndiCamera::tcp` at the table's address and device, with the table's neutral controls,
  and the config's `mode`, and serve it as it serves the other adapters; a discovery
  failure SHALL end the start with `scope-node: <message>` and exit 1, systemd starting
  the node again under a unit with `StartLimitIntervalSec=0` (owner decision 8).
- **FR-011**: `crates/scope-node/README.md` SHALL gain `## The finder camera over INDI`
  before `## The imaging camera`, as plan.md § Interfaces gives it (owner approval 4).
- **FR-012**: Nothing else SHALL change in US2: the `[imaging]` table, its code and its
  texts, the imaging adapter's construction, the mount, the scope link's protocol,
  `crates/finder`, and every existing test and fixture, US1's among them.

### Key Entities

- **The finder camera over INDI**: the finder camera as the scope node reaches it through
  an INDI server's device, beside the imaging camera.
- **The high byte**: a 16-bit sample's top 8 bits, the 8-bit gray value detection reads.
- **The gain floor**: 1, the least gain the node reports for a camera whose own range
  starts at 0.
- **The gain sent**: the camera's last answer to the adapter's gain, the only gain at
  which the adapter passes a frame on.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario has a committed test that asserts it, and the
  `lint` and `test` gates pass.
- **SC-002**: No existing test, golden, fixture, `scripts/ci/` file, `.github/` file,
  `CODEOWNERS` entry, `ergane.yaml` or `playwright.config.ts` is edited. Every existing
  test passes unchanged (plan.md § Shapes that existing tests pin).
- **SC-003**: A node configured with `adapter = "indi"` describes the camera from its
  device and sends its star frames at the set or automatic exposure and gain, 16-bit
  samples as their high bytes and 8-bit samples as they are (US1-S1 and US1-S2 for the
  description and the high bytes, US1-S3 and US2-S6 for 8-bit samples, US2-S8 for
  automatic gain).
- **SC-004**: Two INDI cameras of one server are each on their own device or refused
  before the node starts (US2-S3, US1-S9), and the finder camera sends star frames again
  after the server returns, none at a gain the adapter did not send (US1-S7, US2-S7,
  US2-S9).

## Assumptions

- **Earlier features are in place.** Every spec in `depends_on_landed` has landed. Every
  citation here was read at be7f3ff6.
- **The finder script stands for the ZWO device program** for its definitions: the
  ASI071's, as the bench and 030's colour script read them, with the ASI120MM's sensor
  (1280 × 960, 3.75 µm) and a gain range of 0–100 that is expected, not read from the
  camera (owner decision 7).
- **The device program sends `min` and `max` on a set vector's `oneNumber`**, as the
  bench log shows for `indi_asi_ccd` 2.2; FR-001 relies on it.
- **A freshly started device program has its own settings**: the ASI071's were gain 0
  and the cooler off on the bench; FR-004's rule does not depend on the values.
- **Out of scope:**
  - the console, the page, MCP, the scope link's protocol, the mount, the solver and the
    database;
  - RAW8 on the camera, binning or a region of interest for the finder camera, and
    debayering a colour finder camera;
  - the imaging camera's settings after a reconnection, a follow-up spec of the owner's
    (plan.md § Outside the factory);
  - linking ZWO's SDK;
  - the udev rule, the Pi's deploy and the bench and field checks, which are the owner's
    (plan.md § Outside the factory).
- **Outside the factory:** the bench check of the real camera, the Pi's config and the
  field check, in plan.md § Outside the factory. A new Pi kit is needed once 045 lands.
- **The factory's gates and CI:** the gates are `lint` and `test`, copied in plan.md
  § Verification. CI's other jobs run only in the merge queue, so no scenario's Then
  clause rests on them.

## Work Graph

```yaml
US1:
  depends_on: []
  implements: [FR-001, FR-002, FR-003, FR-004, FR-005, FR-006]
US2:
  depends_on: []
  depends_on_merged: [US1]
  implements: [FR-007, FR-008, FR-009, FR-010, FR-011, FR-012]
```

Two stories, each under 60 % of 65,536 B as the probe built them (plan.md § Plan traps 2):
the adapter, the client's fix and their tests, then the node's config, binary and README
with the adapter inside the node's service. US2 merges after US1 (`depends_on_merged`):
its binary constructs US1's `IndiFinder` and its tests use US1's shared test module, and
plain `depends_on` would order the two without putting US1's code in US2's base.
