---
state: ready
depends_on_landed: ["013-live-finder", "030-imaging-camera-link", "044-node-stops-mount-before-exit"]
# Drafted 2026-11-22 (UTC) against `main` at b7e983a0, at the owner's ask to review "how
# you plan to tweak dark sky detection", from a probe of that day on the branch
# `probe/048` cut from b7e983a0. Folded the same day after review round 1 and the
# owner's answers to decisions 4 and 5, and re-anchored at `main` 28f436df
# (28f436dfbdd6b9e7aead37928f52a980027560fc): every `path:line` here was read there.
# Folded again after review round 2 and the owner's answers to decisions 1, 2, 3, 6 and
# 7 and approval 3: the sky camera now has the finder camera's listed ranges, and US2
# was rerun on `probe/048` with them. Every owner decision is answered and the approvals
# are the owner's (Clarifications, the owner's answers); the flip gives them.
# Folded a third time after review round 3, in wording only: a town sky's noise on the
# finder camera is not measured, so the README leaves the floor out in town; drawn-sky
# results are said to be the drawn sky's; tuning frames at gains 16, 32 and the top.
# Folded a fourth time after review round 4, in wording only: sigma's likeness to the
# floor is the clipped background's, "Finding the value" is one rule, the field's causes
# are said to be read, the floor is tuned on the Pi that observes (046 reads the Pi 5).
# Round 4 reran the validator only; the fold's probe commit is the README's, which no
# gate reads.
# Folded a fifth time after probe round 1, which built both stories from this trio alone
# at 4ea98bcc (the trio committed on 28f436df): every scenario passed on the first build
# and both gates' Rust commands passed, with 0 blocking defects and 8 of clarity. The
# fold adds US2-S3 (Configure takes the link's four keys) and a third frame to US1-S2
# (`detect_stars`'s own floor), each a throwaway the probe ran, and runs no cargo.
# 047 landed in between and wrote only `crates/finder/**` and one `Cargo.lock` line; no
# 048 story writes those files or puts a test beside them, so 047 is not in
# `depends_on_landed` (046's convention); two cites into `crates/finder` moved and are
# updated. 013 is named because its stories built the node's detection, auto-exposure
# and config (`crates/scope-node/src/detect.rs`, `exposure.rs`) and its tests sit beside
# these; 030 because its US7 last changed `config.rs` and `service.rs`; 044 because its
# US1 last changed the node's README and `main.rs`, the binary these config tests run.
# 027's test layout is not named: no story writes a finder test. What rests on a run and
# what on reading, the fold's run among them: plan.md § What was run and what was read.
# Flipped to `ready` on 2026-11-22 at the owner's ask ("then bring me the flip"); its
# merge, on the owner's go, gives the owner approvals. `main` had moved to 2ab39799
# (#360, docs only), which changes no file cited here.
---

# Feature Specification: The Node Detects Stars on a Dark Sky

**Feature Branch**: `048-node-detects-stars-on-a-dark-sky`

**Created**: 2026-11-22

**Status**: Ready

**Input**: At a dark site the finder camera's 1 s frames did not solve: the background
sat near 0 of 255, the noise estimate collapsed, the node reported 200 detections and
nothing matched; 3000 ms at gain 16 solved. Give detection a floor under its noise
estimate, keep auto-exposure from being driven darker by a black sky, with today's
config files accepted as they are. The owner: "i want to get it working on the pi
camera as well. when we add in asi120mm support, i want it to be more of an
adapter/config pattern where we reference the type we want to use." The second wish is
left to spec 045 (owner decision 4).

## Clarifications

### Session 2026-11-22 (the probe on `main` at b7e983a0)

- Q: What does this trio read besides itself? → A: Nothing about the feature. `spec.md`
  holds every requirement and scenario, and `plan.md` every design decision, the
  interfaces, the frames the tests draw, the shapes that existing tests pin and the
  alternatives rejected. A story that would change a decision escalates rather than
  re-deciding it. The probe's branch and notes are not in the repository; what they
  showed is written here.
- Q: What did the dark night show? → A: From the operator's handoff, as the field note
  of 2026-11-14 records it: at the dark site, with the Pi 3B+'s OV5647 at 1296x972, a 1 s
  exposure failed with the background near 0, the noise estimate collapsed and 200
  detections, nothing matching; 3000 ms at gain 16 with sigma 8 solved every frame at
  about 170 to 200 detections. How those settings were set is not recorded. Bright
  saturated stars were not detected either; that is not this spec's. No frame from that
  night survives. The 3B+'s image processor is the VC4; since 046 the node reads the
  Pi 5's frames, the same OV5647 through the PiSP
  (`specs/046-node-reads-pi-5-frames/spec.md:47`,
  `specs/046-node-reads-pi-5-frames/spec.md:50`), whose tuning may treat a dark
  background otherwise, so the floor is tuned on the Pi the node will observe with
  (plan.md § Outside the factory). 025 left "dark-sky detection, the noise floor and
  auto-exposure" to a spec then numbered 023 (`specs/025-frame-capture/spec.md:886`); 023 was closed undrafted.
- Q: Why does a black background give 200 detections? → A: By reading. The node calls
  `star_detection::detect_stars` with its settings and keeps the first 200
  (`crates/scope-node/src/detect.rs:39`, `crates/scope-node/src/detect.rs:49`). Detection estimates the noise on the
  binned detection image (`crates/star-detection/src/detect.rs:47`) from three cuts of
  its middle row, each at most 50 px (`crates/star-detection/src/noise.rs:46`, `crates/star-detection/src/noise.rs:50`),
  taking the deviation of the darkest cut, at least `NOISE_FLOOR`, 0.2
  (`crates/star-detection/src/noise.rs:10`, `crates/star-detection/src/noise.rs:91`), as cedar-detect does
  (`reference-solutions/cedar-detect/src/algorithm.rs:992-995`, `reference-solutions/cedar-detect/src/algorithm.rs:1070-1071`). On a
  background clipped to 0 the darkest cut is all 0, so the noise is 0.2, and the gates
  `max(2, round(2·σ·noise))` and `max(3, round(3·σ·noise))`
  (`crates/star-detection/src/detect.rs:48-49`) are 3 and 5 at σ = 8: a lift of 2 levels
  over a black ring passes. The probe drew such a frame (plan.md § The skies): 88.9 % of
  its pixels 0, 200 detections, 14 of them true stars, 8 among the 30 brightest.
- Q: What does auto-exposure do with them? → A: Above 40 detections it lowers gain first
  and then exposure, by `(30 / N)^0.7` clamped to a quarter
  (`crates/scope-node/src/exposure.rs:23-42`). It never sets an exposure above its cap:
  the larger of the node's frame interval and the camera mode's frame time, within the
  camera's range (`crates/scope-node/src/exposure.rs:12-18`, `crates/scope-node/src/service.rs:305-313`). The node's binary
  builds its service with the default controls (`crates/scope-node/src/main.rs:166`), a
  frame every 200 ms (`crates/scope-node/src/service.rs:58`), and the OV5647's 1296x972
  mode runs at 46.34 frames/s (`fixtures/scope-node/list-cameras.txt:5`), so the cap is
  200 ms. The first frame's exposure, while automatic, is the camera's own: the node has
  none to set yet (`crates/scope-node/src/service.rs:267`, `crates/scope-node/src/service.rs:276-279`), and the rpicam
  adapter starts `rpicam-vid` without a shutter at 5 frames/s at most
  (`crates/scope-node/src/rpicam/mod.rs:174-186`), so that frame is at most 200 ms too,
  by reading. The probe ran the node's service at that interval over a camera drawing
  the dark sky at whatever exposure and gain it was set to, with the finder camera's
  listed ranges (US2's rig, six seed bases): without a floor, from 200 ms and gain 16,
  gain fell 16 → 4.24 → 1.12 → 1 at 200 ms and then exposure 200 → 53 → 14 → 3.7 → 1 ms
  and on to its least, 0.13 ms, every frame at 150 to 200 detections and none after the
  first solving. With a floor of 0.5 levels, after pairing, from gain 1, exposure stayed at
  200 ms and gain rose to between 9.9 and 14.6; the 6th to 12th frames held 23 to 43
  detections, and every frame from the fourth solved within 26″. The drawn sky's gain is
  noiseless (§ Assumptions), so that run shows auto-exposure's path, not what a real
  200 ms frame holds.
- Q: Does the console's camera settings reach the node's detection? → A: Yes, by
  reading. On pairing the console sends Configure with Setup's exposure, gain and
  detection threshold (sigma; Setup labels it "Detection threshold", in σ,
  `crates/finder/web/src/components/ScopeNodeSection.tsx:106-107`), binning 2, rows
  normalised and hot pixels rejected (`crates/finder/src/link.rs:407`,
  `crates/finder/src/link.rs:441-448`, `crates/finder/src/link.rs:476-477`), and again on every Setup change (`crates/finder/src/link.rs:619-622`). The node takes
  the four detection keys from the wire and fills the rest from `DetectSettings::cedar()`
  (`crates/scope-node/src/service.rs:407-435`, `crates/scope-node/src/service.rs:432`). A key the link does not carry,
  such as a floor, would be lost the moment the console pairs. The probe's mutation
  showed it: with the floor reset by Configure, US2-S2 failed; with the node's sigma or
  binning kept by Configure, US2-S3 failed (probe round 1).
- Q: What changes, by owner decision? → A: The owner's answers, as the probe built them:
  `noise_floor` in `[detection]`, in 8-bit levels, from 0.2 to 5, off unless set
  (owner decisions 1, 2); the floor applied inside `star-detection` by a new function
  beside `detect_stars`, which keeps its signature and its results, with Setup's sigma
  documented beside it as the no-code option (owner decision 3); Configure leaving the
  node's own detection keys as the config set them (FR-007); no change to auto-exposure
  (owner decision 5); `solve_image --noise-floor` for tuning the floor from recorded
  frames (owner decision 6); nothing in the console (owner decision 7). No table per
  camera adapter (owner decision 4: spec 045's).
- Q: Which earlier requirements does this change? → A: 013's FR-006 reads: "for the same
  frame and the same detection settings the node's centroids equal the console's"
  (`specs/013-live-finder/spec.md:735`). Without a floor that holds as today; with one,
  the node's settings hold a key the console's detector has not, and the console never
  detects the node's frames itself. 013's FR-014 reads: "The node SHALL detect with
  cedar's detection settings unless the console sends others ... Each star frame SHALL
  report the settings it was made with" (`specs/013-live-finder/spec.md:777-779`). With this spec the node also detects
  with its config's floor, which the console cannot send, and a star frame reports the
  four keys the link carries, not the floor (owner approval 3). 025 made raw recordings
  detection fixtures: a frame line's detection settings, run through `detect_stars`,
  give its centroids (`specs/025-frame-capture/spec.md:105-106`, `specs/025-frame-capture/spec.md:572-575`); with a floor set
  they no longer do (owner approval 3). 013's FR-007 (`specs/013-live-finder/spec.md:739`)
  gains the new key. 013's FR-015, auto-exposure, does not change.
- Q: Do the cedar parity tests change? → A: No. `detect_stars` keeps its signature and
  every result (US1-S1), so `cedar_detect_parity_on_all_reference_images`
  (`crates/star-detection/tests/parity.rs:107`) and every other caller pass unchanged.
- Q: What about the ASI120MM's 12 bits and its offset? → A: Detection reads 8-bit gray
  frames (the constitution's product limits). Spec 045's adapter will make 8-bit frames
  from the camera's 12, and its offset can lift the background off 0, which may make a
  floor unneeded there. Where that camera's own detection settings live is 045's to
  decide (owner decision 4; plan.md § Outside the factory). This spec writes no INDI
  finder code.
- Q: What rests on a run, and what on reading? → A: Every scenario's test was run on the
  probe with the fix, three times each with the same outcome; US2's at the node's own
  frame interval in the first fold and with the finder camera's listed ranges in the
  second. On `main`'s code, US1-S3 and US1-S5's solve without a floor
  passed, three times each, and with `main`'s `service.rs`, once, US2-S1 passed and US2-S2
  failed: they state today's behaviour. The rest name what `main` lacks and do not build there.
  Probe round 1 then built both stories from this trio alone, and every scenario passed
  on the first build; US1-S2's third frame and US2-S3, added in its fold, ran there as
  throwaway tests on the built code and under the mutations they catch.
  plan.md § What was run and what was read lists each claim. No real camera, recorded
  frame or Pi was used: the dark sky is drawn to show the field's symptoms, not measured.

### Session 2026-11-22 (the owner's answers)

- Q: Owner decision 4, a table of detection settings per camera adapter? → A: leave the
  per-camera tables to spec 045 (the owner took the seat's recommendation). This spec has no such table: the draft's third story,
  its six scenarios, its five requirements, its owner approval and its share of the new
  public API are gone, and the last requirement is renumbered FR-009. plan.md § Outside
  the factory keeps the design work for 045: the shapes considered, what the console's
  pairing leaves of them, and the probe's trial.
- Q: Owner decision 5, auto-exposure? → A: (a), no auto-exposure change; the cap stays as
  it is (the owner took the seat's recommendation). On the node, auto-exposure never goes above 200 ms; a longer exposure, such as
  the 3000 ms at gain 16 that solved on the dark night, needs a fixed exposure set in
  Setup. US2's rig runs at the node's own frame interval (review round 1 found the
  draft's 3000 ms interval was not the node's), and US2-S2 holds the floor of 0.5 that
  the README now gives.
- Q: Owner approval 3, star frames and recordings without the floor? → A: "Approval 3:
  approved. Frames and recordings don't carry the floor, so record [the dark night's]
  frames with the floor unset. Don't add the floor to star frames or link messages in
  048." The recording step of plan.md § Outside the factory records with the floor
  unset, and FR-007 keeps the link's messages and star frames as they are.
- Q: Owner decision 1, the setting's name and unit? → A: "Decision 1: noise_floor, 8-bit
  levels, range 0.2 to 5." (FR-003.)
- Q: Owner decision 2, its default? → A: "Decision 2: off by default; README says start
  at 0.5." (FR-002, FR-004.)
- Q: Owner decision 3, where the floor applies? → A: "Decision 3: a new function in
  star-detection, cedar's detector untouched, with Setup's sigma documented beside it as
  the no-code option." Cedar's detector untouched means that `detect_stars` keeps its
  signature and every result (US1-S1). The README section US1 writes documents Setup's
  detection threshold (sigma) beside the floor (FR-004).
- Q: Owner decision 6, a tool for tuning the floor? → A: "Decision 6: solve_image
  --noise-floor." (FR-006.)
- Q: Owner decision 7, the floor in the console? → A: "Decision 7: no floor in the
  console in 048." (FR-009.)

## Owner decisions

Every decision is answered (Clarifications, the owner's answers). plan.md § Decisions
and the alternatives rejected records the alternatives and why each lost.

1. **The setting's name and unit** (FR-003). **Answered:** `noise_floor`, in the 8-bit
   levels of the frame detection reads, from 0.2, cedar's own, to 5.
2. **Its default** (FR-002, FR-003, FR-004). **Answered:** off; the README gives 0.5 as
   the starting point for a dark site.
3. **Where the floor applies** (FR-001, FR-002, FR-004). **Answered:** a new
   `detect_stars_with_noise_floor` in `star-detection`, with `detect_stars` keeping its
   signature and every result (the owner's "cedar's detector untouched"), and Setup's
   detection threshold (sigma) documented beside it in the node's README as the no-code
   option, (e) of plan.md § Decisions.
4. **A table of detection settings per camera adapter**. **Answered: left to spec 045**.
   plan.md § Outside the factory keeps the options for 045.
5. **Auto-exposure** (FR-008). **Answered: (a)**, no change; the cap stays as it is. A
   longer exposure than 200 ms needs a fixed exposure set in Setup.
6. **A tool for tuning the floor from recorded frames** (FR-006). **Answered:**
   `solve_image --noise-floor`, so US2 writes a `crates/plate-solver` example.
7. **Showing the floor in the console** (FR-009). **Answered:** not in 048. A later
   spec may carry the floor on the link's `Detection`, a `proto/` change, so that Setup,
   MCP, star frames and recordings show it ((b) of plan.md § Decisions).

## Owner approvals

Flipping this spec to `ready` approves the following; the owner approved approval 3 on
2026-11-22 (Clarifications). Each is named in the FR that makes it. No existing test,
golden, fixture or CI file changes.

1. **New public API**: `star_detection::detect_stars_with_noise_floor`;
   `star_detection::noise::NOISE_FLOOR` made public; `DetectSettings::noise_floor`
   (FR-001, FR-002). No existing signature changes.
2. **Configure keeps the node's own detection keys** (FR-007), where today it resets
   every key the link does not carry to cedar's. Today no config can set those keys to
   anything but cedar's, so no observable changes until a floor is set.
3. **With a floor set, star frames and raw recordings no longer say how their centroids
   were found** (FR-002, FR-007). A star frame reports the four detection keys the link
   carries and not the floor, so it no longer reports every setting it was made with
   (013's FR-014, `specs/013-live-finder/spec.md:777-779`); and a raw recording's frame line, whose settings
   025 made reproduce its centroids (`specs/025-frame-capture/spec.md:572-575`), no longer does. 048 adds
   the floor to neither, and until a later spec carries the floor on the link (owner
   decision 7), the recording step of plan.md § Outside the factory records with the
   floor unset, as the owner said. With no floor set, nothing changes.

## User Scenarios & Testing *(mandatory)*

Every scenario shares these definitions.

- **The gate command.** Every scenario runs in
  `cargo nextest run --workspace --all-targets --all-features --locked`, the `test`
  gate's first command.
- **The skies** are drawn by the test, as plan.md § The skies gives them exactly: the
  simulator's catalog field at altitude 50°, azimuth 120°, on a 1296x972 frame 13° wide,
  its stars taken once; levels scaled by exposure in seconds times gain; read noise
  shared over 2x2 pixels; a level the camera subtracts, then rounding and clipping to 8
  bits, so that a pixel with no light reads 0. **The dark-sky frame** is the dark sky at
  1 s and gain 1 from seed 13; **the city frame** the city sky's.
- **A true star** is a centroid within 1.5 px of a star the frame drew.
- **The console's solve** is `solve_from_centroids` of a frame's 30 brightest centroids
  as `(y, x)`, with no field estimate, 10° field error, 0.01 match radius, 1e-5 match
  threshold, a 5000 ms timeout, distortion 0 and 0.002 match error, the frame's size
  `(1296, 972)`, over the bundled
  cedar database, as `crates/simulator/tests/sky.rs:69` calls it; **its error** is the
  angle between its RA and Dec and the simulator's true camera direction. A frame
  **solves** when the console's solve finds a match.
- **The sky camera** is a camera adapter in the test with the finder camera's listed
  ranges: one mode, 1296x972 at 46.34 frames/s (`fixtures/scope-node/list-cameras.txt:5`),
  exposure 0.13 to 3066.985 ms and gain 1 to 63.9375, the OV5647's `ExposureTime` in
  microseconds and `AnalogueGain` (`fixtures/scope-node/list-cameras.txt:27`,
  `fixtures/scope-node/list-cameras.txt:17`) as the rpicam adapter reads them
  (`crates/scope-node/src/rpicam/listing.rs:68-80`,
  `crates/scope-node/src/rpicam/listing.rs:96-97`); each frame the dark sky drawn at the
  exposure and gain it is set to, from seed 13 plus its frame number, counted from 1;
  with exposure automatic it
  takes 200 ms, the longest frame the rpicam adapter's 5 frames/s allows
  (`crates/scope-node/src/rpicam/mod.rs:174`), and with gain automatic the gain the
  scenario names.
- **The node rig** is `NodeService::new` over the sky camera, which takes the default
  controls, a frame every 200 ms (`crates/scope-node/src/service.rs:58`), as the node's
  binary does (`crates/scope-node/src/main.rs:166`), so that auto-exposure's cap is
  200 ms; with the system clock, token `test-token`, exposure and gain automatic and the
  detection settings the scenario names, served over `scope_link::memory` to a
  `ScopeNodeClient` with the bearer token `test-token`, under
  `#[tokio::test(start_paused = true)]`. **The pairing settings** are what the console
  sends with Setup's defaults: exposure and gain absent, sigma 8, binning 2, rows
  normalised, hot pixels rejected.
- **The config rig** is the built `scope-node`, `env!("CARGO_BIN_EXE_scope-node")`, run
  with `--config` naming a file under `CARGO_TARGET_TMPDIR` and `--check-config`; its
  exit status, standard output and standard error. **The head** is the three lines
  `listen = '127.0.0.1:50051'`, `token = 'test-token'` and `adapter = '<adapter>'`.
- **Today's `[detection]` print** is
  `[detection]\nsigma = 8.0\nbinning = 2\nnormalize_rows = true\ndetect_hot_pixels = true\nreturn_binned = false\nuse_binned_for_star_candidates = true\n\n# no mount configured\n`,
  as `main`'s binary prints it for a config with no `[detection]` table (the probe ran it).
- **Oracles.** The drawn stars and the simulator's truth; cedar-detect's reference
  images; cedar's threshold rule written as arithmetic; `main`'s own output, captured by
  the probe and written here as literals; the words in these scenarios. One is partly
  the code under test: US1-S1 holds `detect_stars_with_noise_floor` to `detect_stars`,
  which the same diff rewrites to call it. At binning 1 without row normalisation
  `parity.rs` still pins `detect_stars` to cedar-detect's goldens
  (`crates/star-detection/tests/parity.rs:113`); at binning 2 with row normalisation
  nothing pins it directly, only `image_parity`'s solve through it
  (`crates/plate-solver/tests/image_parity.rs:152`) and the existing tests of plan.md
  § Shapes. Nor does US1-S1 pin `detect_stars`'s own floor of 0.2: by inference every
  reference image's estimate is above 0.3 at both settings, and a `detect_stars` that
  passed 0.3 passed US1-S1 and US1-S2's first two frames on the probe. US1-S2's third
  frame pins it by cedar's arithmetic: `2·2 = 4` against the gate
  `max(2, round(2·σ·noise))`, 3 at 0.2 and 5 at 0.3. Every other expected value comes
  from outside the code under test. Two of US2-S2's clauses cannot fail: the pairing
  settings are the node's initial settings' wire form
  (`crates/scope-node/src/service.rs:395`), so `Configure` answering them and every
  frame reporting them hold for a `Configure` that does nothing. FR-007 rests on
  US2-S2's 200 ms, solve and count clauses for the floor kept, and on US2-S3, which
  sends settings other than the node's own, for the four keys taken from the link;
  today's `Configure` fails the first of US2-S2's at the fourth frame (plan.md § Plan
  traps 5).

### User Story 1 - Detection has a noise floor (Priority: P1)

As the observer at a dark site, I set a floor under the node's noise estimate in its
config, and on the drawn dark sky its noise points are gone while its stars stay; on
the drawn city sky nothing changes.

**Why this priority**: Without it a dark sky gave 200 detections a frame and nothing
matched (the field note); by reading, mostly noise. Nothing else in this spec has a
floor to work with.

**Independent Test**: `crates/star-detection/tests/noise_floor.rs` (US1-S1, US1-S2),
`crates/simulator/tests/dark_sky.rs` (US1-S3 to US1-S5, with the skies in
`crates/simulator/tests/dark/mod.rs`) and `crates/scope-node/tests/noise_floor.rs`
(US1-S6, US1-S7), all new. The `test` gate runs them; during development,
`cargo nextest run -p star-detection --locked --test noise_floor`,
`cargo nextest run -p simulator --locked --test dark_sky` and
`cargo nextest run -p scope-node --locked --test noise_floor`.

**The floors used**: US1-S4 uses 1.0, not the README's 0.5, because at 1.0 every
detection on the 1 s dark-sky frame is a true star, which shows the mechanism exactly; at
0.5 that frame gives 20 detections, 7 of them true. The README's 0.5 is for the node's own
200 ms frames at high gain, where US2-S2 holds it on the drawn sky.

**Acceptance Scenarios**:

1. **Given** each of the 13 images of `crates/star-detection/tests/data/cedar_goldens_bin1_sigma8.json`, read as luma from `reference-solutions/cedar-detect/test_data`, **When** a committed test calls `detect_stars_with_noise_floor` on each at sigma 8 with hot pixels rejected, at binning 1 without row normalisation and at binning 2 with it, with floors 0.0, 0.1 and 0.2, **Then** it asserts that each result equals `detect_stars` at the same settings star for star and in order, and that no `detect_stars` result is empty.
2. **Given** three 32x32 frames of 0, one with levels 3, 6 and 3 at x 15, 16 and 17 of row 10, one with 9, 18 and 9 there and a third with 1, 2 and 1 there, **When** a committed test detects each at sigma 8, binning 1, no row normalisation and no hot-pixel rejection, **Then** it asserts that the first gives one star with `detect_stars` and with floors 0.2 and 0.7 and none with floor 1.0, the second one star within 0.1 px of (16.5, 10.5) with floor 1.0 and none with floor 2.5, and the third one star with `detect_stars` and with floor 0.2, and none with floor 0.3.
3. **Given** the dark-sky frame, **When** a committed test detects it with `scope_node::detect::detect` and `DetectSettings::cedar()`, **Then** it asserts that at least 80 % of its pixels are 0, that it gives 200 centroids, that at most 15 of them are true stars, and that at most 10 of its 30 brightest are.
4. **Given** the dark-sky frame, **When** a committed test detects it with `scope_node::detect::detect` and cedar's settings with `noise_floor` 1.0, **Then** it asserts that it gives at least 4 centroids and that every one is a true star.
5. **Given** the city frame, **When** a committed test detects it with `scope_node::detect::detect` and cedar's settings, and again with `noise_floor` 0.5 and with 1.0, **Then** it asserts that the console's solve of the first is within 30″ and that the other two results equal the first centroid for centroid.
6. **Given** the config rig on the head with adapter `simulated` and no `[detection]` table, and on the same head followed by `[detection]` and `noise_floor = 1.0`, **When** a committed test runs each, **Then** it asserts that both exit 0, that the first's standard output holds today's `[detection]` print and the second's holds `[detection]\nsigma = 8.0\nnoise_floor = 1.0\nbinning = 2\n`, and that `Config::parse` of the two gives settings whose `detection.noise_floor` is `None` and `Some(1.0)`.
7. **Given** the config rig on the head with adapter `simulated`, followed by `[detection]` on line 4 and `noise_floor` on line 5 set to `0.1`, `5.5`, `nan`, `'high'`, `0.2` and `5` in turn, **When** a committed test runs each, **Then** it asserts that the first three exit 1 with standard error exactly `scope-node: detection.noise_floor must be between 0.2 and 5\n`, that the fourth exits 1 with exactly `scope-node: invalid noise_floor at line 5\n`, and that the last two exit 0.

### User Story 2 - Auto-exposure finds the stars on a dark sky (Priority: P1)

As the observer at a dark site, with the floor set, I pair the console, and, on the
drawn sky, the node's auto-exposure, which never goes above 200 ms, holds 200 ms and
raises gain until it finds the stars, and the live solve holds from the fourth frame;
and I can find the floor's value from frames I recorded.

**Why this priority**: The console pairs with every node it uses, and without this the
pairing would undo the floor; on the drawn sky the floor is what lets auto-exposure work
on a black sky.

**Independent Test**: `crates/simulator/tests/dark_sky_exposure.rs` (US2-S1 to US2-S3),
new, with US1's `crates/simulator/tests/dark/mod.rs`. The `test` gate runs it; during
development, `cargo nextest run -p simulator --locked --test dark_sky_exposure`.

**Acceptance Scenarios**:

1. **Given** the node rig with cedar's settings and the sky camera at gain 16 while gain is automatic, **When** a committed test reads 8 star frames, **Then** it asserts that each holds at least 100 centroids, that the first holds 200 at 200 ms and gain 16 and solves within 30″, that neither gain nor exposure ever rises from one frame to the next, that exposure stays 200 ms while gain is above 1, that the eighth is at gain 1 and under 10 ms, and that no frame from the third solves.
2. **Given** the node rig with cedar's settings and `noise_floor` 0.5, and the sky camera at gain 1 while gain is automatic, **When** a committed test sends the pairing settings with `Configure` and then reads 12 star frames, **Then** it asserts that `Configure` answers the pairing settings, that every frame reports them and is at 200 ms, that each frame from the fourth solves within 30″, that each frame from the sixth holds 20 to 50 centroids at a gain above 8, and that at least 5 of those 7 frames hold 25 to 40.
3. **Given** the node rig with cedar's settings and `noise_floor` 0.5, and the sky camera at gain 1 while gain is automatic, **When** a committed test sends `Configure` with exposure and gain absent, sigma 20, binning 4, rows not normalised and hot pixels not rejected, and then reads 3 star frames, **Then** it asserts that `Configure` answers those settings and that the third frame reports them.

### Edge Cases

- **A floor below a sky's own noise** changes nothing: detection takes the larger of the
  two (US1-S5: on the city frame floors of 0.5 and 1 change nothing, so its estimate is
  above 1; inferred, the estimate itself is not printed). A town sky's noise on the
  finder camera is not measured, and at binning 2 the detection image averages each 2x2
  block (`crates/star-detection/src/binning.rs:118`), which halves independent pixel
  noise, so a real town sky's estimate may sit near 0.5: the README leaves the floor out
  in town, or has the observer compare star counts with and without it (FR-004).
- **A floor of 0.2 or less** is cedar's own: 0.2 is accepted (US1-S7), below it refused,
  since it would do nothing. **A floor above 5** is refused (US1-S7): at sigma 50 a
  floor above 5.1 would let no star pass.
- **The 200 cap** is unchanged: the node still keeps the brightest 200
  (`crates/scope-node/src/detect.rs:49`), so a crowded field still reads 200 and still
  steps exposure down, as 013 built it.
- **A lone dark-sky frame may still solve** on the few true stars among its 30
  brightest: the probe's dark-sky frame did, within 17″, on 8 of them; plan.md § What
  was run gives the other seeds. What fails every time on the drawn sky is auto-exposure
  (US2-S1). Not asserted.
- **The console's Setup sigma** replaces the config's sigma once the console pairs, as
  today (013 FR-014, US2-S3); the floor stays the config's (FR-007, US2-S2).
- **A config without a floor at a dark site** still drives auto-exposure to its least on
  the drawn sky (US2-S1), as today; the README's floor is the remedy, or Setup's sigma
  (owner decision 3 (e)).
- **Auto-exposure's cap**: on the node no automatic exposure exceeds 200 ms (US2-S1 and
  US2-S2 hold every frame at or under it). A longer exposure, such as 3000 ms at gain 16,
  needs a fixed exposure set in Setup, which auto-exposure leaves alone (013 FR-015).
- **A floor too high for the node's 200 ms frames** drives gain to its top (63.9 on the
  OV5647) below the band: on the drawn sky the probe's floors of 4 and 5 held gain at its
  top from the fourth frame with 18 to 22 and 11 to 15 detections, every frame from the
  fourth solving, while 1.0 settled at gains 23.8 to 32.3 in the band (plan.md § What
  was run). Not asserted. The model's gain is noiseless (§ Assumptions), so on a real
  camera a lower floor may reach gain's top; the README's tuning takes the least floor that works,
  with a fixed longer exposure in Setup as the fallback.
- **`--check-config` and a star frame**: the first prints the floor (US1-S6), the second
  does not report it (US2-S2, US2-S3): the link has no field for it (owner decision 7, owner
  approval 3).

## Requirements *(mandatory)*

### Functional Requirements

**The floor** (US1):

- **FR-001**: `star-detection` SHALL gain `pub fn detect_stars_with_noise_floor`, taking
  `detect_stars`'s arguments and then `noise_floor: f64`, which detects as `detect_stars`
  does with the detection image's noise estimate raised to at least `noise_floor`.
  `detect_stars` SHALL keep its signature and call it with `noise::NOISE_FLOOR`, now
  public and still 0.2, so that its results do not change (owner decision 3, owner
  approval 1).
- **FR-002**: `DetectSettings` SHALL gain `noise_floor: Option<f64>`, absent by default
  and in `DetectSettings::cedar()`, and left out when serialised while absent.
  `scope_node::detect::detect` SHALL call `detect_stars` while it is absent and
  `detect_stars_with_noise_floor` with it while present, keeping the brightest 200 as
  today (owner decisions 2, 3; owner approvals 1, 3).
- **FR-003**: The config's `[detection]` SHALL take `noise_floor`, a number from 0.2 to 5
  in 8-bit levels. Any other number, NaN included, SHALL stop the node with
  `detection.noise_floor must be between 0.2 and 5`, and a value that is not a number
  with `invalid noise_floor at line <n>`. `--check-config` SHALL print it under
  `[detection]`, after `sigma`, when set, and print `[detection]` exactly as today when
  not (owner decision 1).
- **FR-004**: The node's README, in the section US1 writes, SHALL say what the floor does,
  its unit and range, that it is off unless set, and give 0.5 as a starting point for a
  dark site; that a town sky's noise on the finder camera is not measured, so the floor
  is left out in town or star counts compared with and without it; and beside the floor,
  as the no-code option, Setup's detection threshold (sigma), as Setup labels it
  (`crates/finder/web/src/components/ScopeNodeSection.tsx:106-107`): that on a
  background clipped to 0, as on the simulated dark sky, sigma 20 acts as a floor of 0.5
  and 40 as 1.0, and that, like 0.5, about 20 to 40 is a starting point at a dark site;
  that it costs stars in town, where a city frame's detections fall, and that it needs
  no config change (owner decision 3).

**The console's settings and auto-exposure** (US2):

- **FR-005**: The node's README SHALL say that the console's camera settings leave the
  floor as the config set it; that neither a star frame nor a raw recording's frame line
  reports it, so a recording should be made with the floor unset or noted beside it;
  that auto-exposure on the node never goes above 200 ms, holds 200 ms and raises gain
  while it finds fewer than 25 stars, and that a fixed longer exposure set in Setup is
  the fallback, without promising a star count; and how to find the floor's value from
  the 200 ms raw frames, at gains 16, 32 and the camera's top, recorded with
  `POST /api/record/raw` on the Pi the node will observe with, retuning if that changes,
  and replayed with `solve_image --noise-floor` over the database the console reports it
  uses: the least floor at which the gain-16 frames hold tens of stars, not hundreds,
  and solve, left unset if they already do with none, then checked on the gain-32 and
  top frames, with the live check that follows.
- **FR-006**: The `solve_image` example SHALL take `--noise-floor <levels>` and, when it is
  given, detect with `detect_stars_with_noise_floor`; its usage line and its doc comment
  SHALL list the flag (owner decision 6).
- **FR-007**: `Configure` SHALL take sigma, binning, row normalisation and hot-pixel
  rejection from the link and keep every other detection key, the floor among them, as
  the node's config set it, so that a floor applies to every frame after the console
  pairs. The link's messages, and the settings a star frame reports, SHALL not change
  (owner approvals 2, 3). US2-S3 holds the four keys taken from the link, and US2-S2
  the floor kept.
- **FR-008**: Auto-exposure SHALL not change: 013's FR-015 and
  `crates/scope-node/src/exposure.rs` stand as they are, and so does its cap (owner
  decision 5).

**Everything else** (US1, US2):

- **FR-009**: Nothing else SHALL change: the scope link's protocol, `crates/finder`,
  `Cargo.toml` and `Cargo.lock`, the node's `main.rs`, `capture.rs`, `camera.rs`,
  `exposure.rs`, `rpicam/`, `mount/` and `imaging/`, the simulator's `src/`, and every
  existing test and fixture.

### Key Entities

- **Noise floor**: the least noise, in 8-bit levels of the detection image, that
  detection assumes when it sets its thresholds; detection uses the larger of the floor
  and the noise it measures. cedar-detect's own is 0.2; the config's `noise_floor` raises
  it.
- **Clipped background**: a finder camera frame's background that the camera's
  processing has cut to 0, so that the noise detection measures there is next to none.
- **Auto-exposure cap**: the longest exposure auto-exposure sets, the larger of the
  node's frame interval and the camera mode's frame time; 200 ms on the node.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario has a committed test that asserts it, and the
  `lint` and `test` gates pass.
- **SC-002**: No existing test, golden, fixture, `scripts/ci/` file, `.github/` file,
  `CODEOWNERS` entry, `ergane.yaml` or `playwright.config.ts` is edited. Every existing
  test passes unchanged (plan.md § Shapes that existing tests pin).
- **SC-003**: On the drawn sky, at the node's own frame interval with the finder
  camera's listed ranges and after the console pairs, with a floor of 0.5 auto-exposure
  holds every frame at 200 ms, each frame from the fourth solves within 30″, and from the
  sixth each holds 20 to 50 stars at a gain above 8, at least 5 of 7 holding 25 to 40
  (US2-S2); without the floor gain and then exposure fall to 1 and under 10 ms and no
  frame from the third solves (US2-S1). Gain is noiseless there (§ Assumptions): this
  shows auto-exposure's path and the floor surviving pairing, not that a real 200 ms
  frame holds enough stars.
- **SC-004**: Today's config files print as before (US1-S6), and cedar's `detect_stars`
  finds what it found (US1-S1).

## Assumptions

- **Earlier features are in place.** Every spec in `depends_on_landed` has landed. Every
  citation here was read at 28f436df.
- **The scope** is the scope node's detection, with two reaches outside its crate: US1
  writes `crates/star-detection`, whose detector the node and the console share, and US2
  a `crates/plate-solver` example. Neither changes what the console does.
- **The dark sky stands for the field's** in its symptoms only: a background clipped to
  0, the noise estimate at 0.2, 200 detections at 1 s, and about 160 true detections that
  solve at 3000 ms and gain 16 (the probe drew it from six seeds: 161 to 163 true stars,
  each frame solving within 6″). Its levels were chosen to show them, not measured. Its
  read noise is shared over 2x2 pixels because independent noise at the same levels
  gives 75 detections on the 1 s frame, and that frame solves within 5.3″: only the
  shared noise shows the field's failure. The model is linear, while the Pi camera's
  frames are `rpicam-vid`'s YUV, which pass through its image processor's tone curve; the
  node's arguments turn off denoise and sharpening, not the curve
  (`crates/scope-node/src/rpicam/mod.rs:150-167`). The model also treats gain as
  noiseless: the sky and the flux scale with exposure times gain, and the level the
  camera subtracts and the read noise do not, so 200 ms at gain 14.5 draws the same frame
  as 2.9 s at gain 1, where a sensor's analog gain amplifies its own shot and read noise
  too. US2-S2 therefore shows auto-exposure's path, holding 200 ms and raising gain, and
  the floor surviving pairing; it does not show that a 200 ms frame on a real dark sky
  holds enough stars, which the field note never measured: it records 1 s failing and
  3000 ms at gain 16 solving, nothing at 200 ms. How far a real OV5647 frame differs is
  the field check's.
- **The floor's value for the real camera** is not known; 0.5 is what the README's
  tuning procedure gives on the probe's drawn frames. The owner finds the real one from
  recorded frames (plan.md § Outside the factory).
- **Out of scope:**
  - detecting saturated stars for the aim tap;
  - any change to the console, the page, MCP, the scope link, the mount or imaging;
  - showing the floor in Setup (owner decision 7);
  - the INDI finder adapter, its 12-bit conversion and any table of detection settings
    per camera adapter, which are spec 045's (owner decision 4);
  - a change to auto-exposure or its cap (owner decision 5);
  - lifting the Pi camera's background off 0 at the source (plan.md § Outside the
    factory).
- **Outside the factory:** recording twenty raw frames at five settings on the next dark
  night with the floor unset, trying Setup's detection threshold (sigma) there, tuning
  the floor from the frames, an optional town frame, the field check at the dark site,
  and what 045 must add, in plan.md § Outside the factory, by the owner.
- **The factory's gates and CI:** the gates are `lint` and `test`, copied in plan.md
  § Verification. CI's other jobs run only in the merge queue, so no scenario's Then
  clause rests on them.

## Work Graph

```yaml
US1:
  depends_on: []
  implements: [FR-001, FR-002, FR-003, FR-004, FR-009]
US2:
  depends_on: []
  depends_on_merged: [US1]
  implements: [FR-005, FR-006, FR-007, FR-008, FR-009]
```

US2 merges after US1: its test reads US1's `crates/simulator/tests/dark/mod.rs` and
sets US1's floor, and both write the node's README.
