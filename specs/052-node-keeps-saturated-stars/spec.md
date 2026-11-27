---
state: ready
depends_on_landed: ["013-live-finder", "025-frame-capture", "045-node-runs-finder-over-indi", "047-console-uses-the-database-that-fits", "048-node-detects-stars-on-a-dark-sky", "051-tools-page-focuses-either-camera"]
# Drafted 2026-11-27 against `main` at f6d4f5fd, at the owner's ask of 2026-11-27 ("Your
# rec. Proceed", on the seat's ranked list of a field night's defects, this first), from
# a probe of that day on the branch `probe/052` cut from f6d4f5fd; folded the same day
# with cynical review round 1 (0 blockers, 2 majors, 11 minors), the fold's probe commits
# on the same branch (tip 62b814ef: the three stories built, every scenario's test and
# both gates' Rust commands passing, each new test failing on the state before its
# story, each gate mutated once; plan.md § What was run and what was read). Probed
# from this trio alone the same day (round 1, branch `probe/052-r1`, tip 491b7655:
# every scenario passing, 0 blocking and 10 clarity defects) and folded: five choices
# no scenario saw are now steps of US1-S1, US1-S2 and US1-S4 (commit ae80ff09, each
# bitten by its mutation), the rest words; the gates' Rust commands pass at ae80ff09.
# Every `path:line` here was read at f6d4f5fd. The owner answered the seven decisions on
# 2026-11-27, each with its recommendation, which the scenarios were already written to. 013 is named
# because its stories built the node's detection, the simulator's finder camera and the
# live loop, and last wrote `crates/scope-node/tests/detect.rs`; 025 because its raw
# frames replay through detection; 045 because its US2 last wrote the node's README and
# its INDI finder camera adapter reads the 8 bits whose top is 255; 047 because its US1
# last wrote the console's `solve.rs`, and its narrow test database is the pattern US2's
# is built on; 048 because its stories last wrote the node's detection,
# star-detection's `lib.rs` and `solve_image`; 051 because it last wrote the focus aid's
# `finder_focus.rs`, the console's test binary, the console's README and the glossary,
# and its simulated focuser widens the stars these tests saturate (plan.md § Refinement
# Record).
# Flipped to `ready` on 2026-11-27 at the owner's ask ("Flip 052 now", in reply to the seat's
# seven recommendations), with every prerequisite in plan.md met and none waived. `main`
# still at f6d4f5fd.
---

# Feature Specification: The Node Keeps Saturated Stars

**Feature Branch**: `052-node-keeps-saturated-stars`

**Created**: 2026-11-27

**Status**: Ready

**Input**: On a field night the finder camera (a ZWO ASI120MM Mini, 1280 × 960, about
1.62° wide, read as 8 bits by the scope node) gave `no_match` on the Orion Nebula's
field with Track on: 55 to 70 stars detected at 1.5 s, and 39 to 43 at 1 s with the
nebula at the aim point. Before Track was first on, with the mount still and each star
trailed about 9 pixels, it gave `no_match` on the Pleiades' and other fields at 3 s with
54 to 185; the Pleiades with Track on at 1.5 s matched. Fields whose brightest stars
were missing failed, whatever their count. So the aim point got no readings, GoTo ended
"no frame solved after settling" and Track "failed after 5 corrections". The seat's
triage, offline, on the Orion frame: 10 of its brightest stars were missing from the
detected list; with all 10 put first it solved at once, 30 of 30 matched, with the 7 of
them that are round and whole it still solved, and with only the 3 largest it did not.
Make the scope node's detection keep saturated stars, round or trailed along the frame's
common direction, and set aside hot pixels, nebulae, merged doubles and a frame mostly
saturated; detect a frame uploaded to the console the same way; say so in the README.

## Clarifications

### Session 2026-11-27 (the probe on `main` at f6d4f5fd, and its fold)

- Q: What does this trio read besides itself? → A: Nothing about the feature.
  `spec.md` holds every requirement and scenario, and `plan.md` every design decision,
  the interfaces, the shapes that existing tests pin and the alternatives rejected. The
  probe's branch and logs are not in the repository; what they showed is written here.
- Q: Why are the brightest stars lost? → A: By reading, and shown on drawn frames. The
  node keeps the first 200 of `star_detection::detect_stars`
  (`crates/scope-node/src/detect.rs:43`, `crates/scope-node/src/detect.rs:67`), which
  ports cedar-detect's gates. A candidate must stand strictly above its pixels two to
  either side (`crates/star-detection/src/detect.rs:180`), which a flat top wider than
  three binned pixels fails; a core wider than `max_size`, 7 binned pixels on a 1280
  wide frame at binning 2 (`crates/star-detection/src/binning.rs:108-114`), is refused
  (`crates/star-detection/src/detect.rs:293`), and so is a perimeter whose range passes
  3σ·noise (`crates/star-detection/src/detect.rs:361`). cedar-detect says so itself: "if
  a bright star is overexposed to a degree that it bleeds into too many adjacent pixels,
  CedarDetect will reject the bright star" and leaves exposure to its caller
  (`reference-solutions/cedar-detect/src/algorithm.rs:81-84`). The console sends the
  first 30 centroids to the solver (`crates/finder/src/solve.rs:68`,
  `crates/finder/src/solve.rs:427`), and a pattern database is built from catalog stars
  taken brightest first (`crates/database-generation/src/patterns.rs:34`,
  `crates/database-generation/src/thinning.rs:28`). On drawn frames, cedar's detector
  found no saturated star with a flat top of 32 to 401 pixels (run).
- Q: Were the night's failures all saturation? → A: No. The Orion failures were with
  Track on, untrailed: the case this pass answers. The 3 s failures all came before
  Track was first on, so each star trailed about 9 px (13.7″ a second at Dec +24°, 3 s,
  4.55″ a pixel), and a saturated top drawn out by about half its own width or more is
  longer than 1.5 times its width: the pass as first drafted set exactly those aside.
  Owner decision 7 is that case. On drawn frames at the finder camera's own geometry
  over the narrow Tycho database (Moffat and Gaussian stars, 9 px trails), the pass as
  drafted solved 2 of 12 trailed frames and the recommended rule 10 of 12, where
  cedar's detection alone solved none (run).
- Q: Does the finder camera's frame really have a flat top? → A: By reading: the INDI
  finder camera adapter keeps a 16-bit sample's high byte (`crates/scope-node/src/indi_finder.rs:100-107`),
  so a sample at the camera's full scale reads 255. That the Mini's clipped samples
  read full scale, at every gain, is not measured; a top that clips below 250 is not
  helped (§ Edge Cases).
- Q: Does the simulator draw such a frame? → A: Yes, unchanged. The simulator's finder
  camera holds every pixel to 0 to 255 (`crates/simulator/src/render.rs:122`), and 051's
  focus error widens its stars (`crates/simulator/src/camera.rs:121`). Pointed at the
  Orion Nebula on the finder camera's pixel grid at a 10.5° field (the bundled
  database's narrowest field is 10°), at focus error 3, 500 ms and gain 2, its first
  frame gives 69 centroids, none within 3 px of the 10 brightest drawn stars, and the
  console's solve gives `no_match`; the Pleiades' field gives 56 and `no_match` (run).
  Those frames are at about 30″ a pixel and Gaussian; US2-S4 draws the finder camera's
  own 1.617° over a narrow database built in the test.
- Q: What does the fix do? → A: After cedar's own pass, the node finds each 8-connected
  region of pixels at 250 or above, the finder camera's one saturation level, which
  051's focus aid already uses, and keeps one cedar did not already find as a star when
  it has at least 5 pixels, its ring 3 pixels out from its box lies inside the frame, it
  is no longer than 1.5 times its width or lies within 10° of the frame's common trail
  direction (when at least 3 long regions share it), and its ring averages less than
  half of the way from the frame's median to 250. A frame with more than 24,576 such
  pixels is set aside whole. Each kept star is centred on the mean of its pixels'
  centres and placed before the first of cedar's stars that is fainter, so the list
  stays brightest first (owner decisions 2 to 4 and 7). The console's Solve page and
  MCP detect an uploaded frame the same way (owner decision 1).
- Q: Which earlier requirements does this change? → A: 048's FR-002 reads:
  "`scope_node::detect::detect` SHALL call `detect_stars` while it is absent and
  `detect_stars_with_noise_floor` with it while present, keeping the brightest 200 as
  today" (`specs/048-node-detects-stars-on-a-dark-sky/spec.md:425-427`); it now also
  places the saturated stars cedar set aside (FR-004). 013's FR-006 reads: "for the same
  frame and the same detection settings the node's centroids equal the console's"
  (`specs/013-live-finder/spec.md:735-737`); it still holds between the node and the
  console's Solve page, which both run the pass (FR-005). 025 made a raw recording a
  detection fixture: a frame line's settings, run through `detect_stars` and cut to 200,
  give its centroids (`specs/025-frame-capture/spec.md:572-575`); for a frame holding a
  kept saturated star they no longer do, and `solve_image --saturated` replays it as the
  node detected it (owner decision 5, owner approval 2). 013's FR-014, "Each star frame
  SHALL report the settings it was made with" (`specs/013-live-finder/spec.md:777-779`),
  still holds: the pass is no setting. 048 left saturated stars out ("Bright saturated
  stars were not detected either; that is not this spec's",
  `specs/048-node-detects-stars-on-a-dark-sky/spec.md:71-72`), and so did 020
  (`specs/020-live-chart-calm/spec.md:292-294`).
- Q: Do the cedar parity tests change? → A: No. `detect_stars` and
  `detect_stars_with_noise_floor` keep their code and every result (FR-002), so
  `cedar_detect_parity_on_all_reference_images` (`crates/star-detection/tests/parity.rs:107`)
  and the e2e evaluation, which runs the plate solver's and the gRPC service's own
  tests, see what they saw. One existing test changes: the node's centroids on the saved
  finder camera frame no longer equal `detect_stars`'s, because that real frame holds a
  saturated star cedar sets aside (owner approval 3).
- Q: What rests on a run, and what on reading? → A: Every scenario's test was written
  on the probe as the stories would write it and passed; each failed on the state before
  its story, and each gate's mutation failed a scenario (run). With all three stories
  built, every other test of the workspace passed unchanged. A second probe built the
  three stories from this trio alone and every scenario passed; the choices it found
  no scenario saw are steps or words here now (run). The field night's frames
  were not replayed: only a 640 × 480 preview JPEG of them survives, which is not a
  fixture. plan.md § What was run and what was read lists each claim.

### Session 2026-11-27 (the flip)

- Q: How were the seven open decisions answered? → A: Each with its recommendation, by
  the owner on 2026-11-27, in reply to the seat's question whether to flip with its
  recommended answer to each: "Flip 052 now".

## Owner decisions

All seven are settled by the owner on 2026-11-27, each answered with its recommendation:
(d) for decision 1, (a) for decisions 2, 3, 5 and 6, and (b) for decisions 4 and 7. Each
keeps its options and its evidence; the scenarios and the record were written to the
recommendations, so no scenario changed for them. One general rule is behind decisions 1 to 4 and 7: **a saturated star is the
top of a star, so every finder camera frame the console solves keeps the regions of
saturated pixels cedar's gates set aside when they look like a star's top, round or
trailed with the rest of the frame, places them by their brightness and touches nothing
cedar keeps.** So cedar's detector, its parity and its order stay as they are, no
setting or link field is added, and a frame with no such region gives exactly today's
centroids. plan.md § Decisions and the alternatives rejected gives the measures.

1. **Where the pass runs** (FR-004, FR-005). Answered **(d) in the scope node's
   detection and in the console's `solve_frame_with_frame`**, through a new module of
   `star-detection`; `detect_stars` keeps its code and results, so the parity tests, the
   e2e evaluation, the gRPC service and the plate solver's own detection are as before.
   Evidence (run): the Solve page and MCP call `solve_frame_with_frame`
   (`crates/finder/src/api.rs:1571`, `crates/finder/src/mcp/tools.rs:656`,
   `crates/finder/src/databases.rs:196`); with the pass there, an uploaded bright frame
   detects exactly as the node does and solves (US2-S5), and no existing test changes;
   it writes `crates/finder/src/solve.rs`, which neither spec 053 nor 054 writes, about
   1 KB of diff. (a) The node only, as drafted: a finder camera frame uploaded to the
   Solve page stays `no_match` where the live solve matches. (b) Inside `detect_stars`,
   for every caller: of cedar's 13 reference images one gains a kept star at binning 1,
   the parity test's setting (`test_5mp_g100_e100ms.jpg`, run), so a parity result
   moves. (c) As (a), with a `[detection]` key to turn it off: a config key, a
   `--check-config` line (048's US1-S6 pins today's print) and Configure's handling
   (048's FR-007), for a switch with nothing to switch off.
2. **The centroid of a saturated star** (FR-001). Answered **(a) the mean of its
   saturated pixels' centres**, tetra3's centre of mass with every weight equal.
   Evidence (run): drawn Gaussian and Moffat stars within 0.1 px of their place (US1-S1);
   520 kept stars of the simulator's bright fields, 90th percentile 0.29 px; at the
   finder camera's own geometry every untrailed kept star within 1.37 px of its catalog
   place, and in the review's runs every node match within 2″ of truth. (b) Flux-weighted inside the ring, less the ring's
   mean: 90th percentile 1.15 px, largest 3.81 px, as a neighbour's light pulls it. (c)
   A profile fitted to its unsaturated wings: no reference does it, and more code.
3. **The gates that set a region aside** (FR-001). Answered **(a)**: the level 250,
   8-connected; fewer than 5 pixels (tetra3's `min_area`); a frame holding more than
   24,576 pixels at the level (2 % of the finder camera's frame) set aside whole as
   `Extended`; a ring 3 px out from the box that crosses the frame's edge; longer than
   1.5 times its width by its second moments (tetra3's `max_axis_ratio` in its own
   example), unless decision 7's trail keeps it; a ring averaging more than 0.5 of the
   way from the frame's median to 250 (nebulae); and a region holding a star cedar found.
   Evidence (run): the levels 255, 250 and 240 gave the same solves and no false star on
   every drawn frame (12 untrailed, 18 with nebulae, 12 trailed 4.5 px at the finder
   camera's geometry; 120 simulator frames); on the night's stretched preview, a proxy
   only, 250 kept 4 of the 10 missing stars, the brightest among them, where 255 kept 1
   and left 23 fragments beside another region, and the saved frame's top holds 254s
   inside it. 4- and 8-connectivity were the same on drawn frames; 4 left twice the
   proxy's fragments. Of the 105 regions set aside as small at 9 pixels, 99 were barely
   saturated stars cedar missed, within 2 px of a drawn star with no other region near,
   5 were fragments of a larger top and 1 lay 2.3 px from a star; at 5 pixels one more
   untrailed frame solved (12 of 12) and no false star was kept. The most
   saturated star frame drawn held 7,164 pixels at 255 (0.58 %: Sirius saturated from
   magnitude 10 with heavy wings) and its largest top 6,872; the Moon's disc 117,079 to
   133,693, a twilight sky 97,263, a white frame all 1,228,800, which the pass as drafted
   took about 145 times cedar's detection to follow and with the bound about 0.2 times.
   The ring 3 px out with a share of 0.5 kept every drawn star and set aside every drawn
   nebula; at twice the region's radius it kept smooth and lobed nebulae, 3 px with 0.25
   set aside soft stars, 6 px with 0.5 kept a core in a faint halo. (b) As drafted: 255
   and 9 pixels, no bound; one untrailed frame fewer, the proxy's tops broken, a white
   frame followed pixel by pixel. (c) A per-region cap of 10,000 pixels in place of the
   bound: a drawn Moon beside stars then solves on the 17 stars kept beside it, where the
   bound gives cedar's `no_match`, as today; but a white frame and a twilight sky are
   followed pixel by pixel again (about 137 and 5 times cedar's detection) unless a frame
   share bounds them too, a second constant. (d) tetra3's gates alone, an area window
   and the ratio: its default cap of 100 pixels refuses the brightest tops (up to 2,680
   pixels kept at the finder camera's geometry). (e) Relax cedar's own gates: changes
   `detect_stars` (as 1 (b)).
4. **Where a saturated star ranks** (FR-001). Answered **(b) before the first of
   cedar's stars that is fainter than it**, its brightness the sum, over every pixel
   strictly inside its ring, of the pixel less the ring's mean, so the list stays
   brightest first and cedar's stars keep their own order. Evidence (run): with
   decision 3's gates, (a) and (b) solved the same frames (50 of 53 frames with a kept
   star at the finder camera's geometry, 115 of 116 of the simulator's); but (a), every kept star first, broke the star frame's "brightest
   first" (`crates/scope-link/proto/scope_node/v1/scope_node.proto:64`) on 38 and 48 of
   them, and failed `ten_dense_frames_have_all_fields_top_200_and_ids_count_dropped_exposures`
   (`crates/simulator/tests/node.rs:111`) once a 5-pixel top was kept. (a) Every kept star
   first, the most saturated pixels first, as drafted. (c) After cedar's list: 79 of 115.
5. **Whether the live report names what was set aside** (FR-006, FR-009). Answered
   **(a) not in the live report; `solve_image --saturated` prints, for a recorded raw
   frame, how many regions were kept and set aside and why**, and the README says a star
   frame carries neither. The live report would need a field on the star frame, a scope
   link protocol change (Principle IV), written by the node's capture and service and
   read in `crates/finder/src/live.rs`, which spec 053 is changing now: a further story.
   (b) That, as a later spec once 053 has landed; the next field night then learns the
   counts only from the raw frames it records. (c) Nothing at all: a raw frame recorded
   on a field night replays through `solve_image` as cedar detects it.
6. **An exposure hint that keeps the brightest stars just under saturation**. Answered
   **(a) later, and only if a field night shows saturated frames still fail** once this
   lands: 8 bits cannot hold a magnitude 2.8 star under saturation and detect magnitude
   10, about 1,600 times fainter, and cedar-detect leaves exposure to its caller. (b)
   Here: needs 5 (b)'s link field, and a rule for "too many" saturated stars that no
   field night has measured. (c) Auto-exposure aiming below saturation: changes 013's
   FR-015, out of scope.
7. **Trailed frames** (FR-001). Answered **(b) a trail-aware ratio**: a region longer
   than 1.5 times its width is a trailed star when its major axis lies within 10° of the
   frame's common trail direction, the axial median of the frame's long regions'
   directions, which stands only when at least 3 of them lie within 10° of it; it is then
   kept or set aside by its ring as a round one is. Evidence (run), on frames drawn at
   the finder camera's geometry over the narrow Tycho database (the Orion Nebula's and
   the Pleiades' fields, Gaussian σ 1.3, Moffat β 2.5 and β 1.8, saturating from
   magnitude 7 and 8.5; trails at 20°): trailed 9 px (3 s with the mount still), cedar
   alone 0 of 12, the pass as drafted 2, (b) with decision 3's gates 10; trailed 4.5 px
   (1.5 s), cedar 5, both 11. On the pass as drafted, (b) changed nothing on the
   untrailed frames, the simulator's 120 frames, the drawn objects of US1-S2 and US1-S3
   or the proxies; its one misplaced star, a close pair (a magnitude 4.8 star and a
   companion 5 px off) kept as one at its middle, 2.3 to 3.2 px from the brighter, came in
   8 of the 24 trailed frames, and no solve was lost. US1-S4 and US2-S4 draw it. (a) Out
   of scope: the README and the field plan keep the finder camera's exposure at or under
   about 1.5 s while the mount is still (the first sync, a GoTo's settling), and the 3 s
   failures stay open. (c) A looser ratio: 2.0 solved 6 of the 12 9 px frames and 3.0
   solved 9; from 2.0 the drawn doubles 8 to 10 px apart, from 2.5 all of US1-S2's, are
   kept as one star at their middle, with up to three false stars on the trailed
   frames.

## Owner approvals

Flipping this spec to `ready` approves the answer to every owner decision above, and
the following. Each is named in the FR that makes it. No golden, `scripts/ci/` or CI
file changes, and no existing fixture.

1. **New public API** (FR-001): the module `star_detection::saturated` with
   `SATURATED`, `MIN_PIXELS`, `MAX_SATURATED_PIXELS`, `MAX_AXIS_RATIO`,
   `TRAIL_TOLERANCE_DEG`, `TRAIL_MIN_REGIONS`, `RING_MARGIN`, `MAX_RING_SHARE`,
   `SetAside`, `saturated_stars` and `with_saturated_stars`. No existing signature
   changes; `finder::finder_focus::SATURATED` keeps its name and its value 250 (FR-003).
2. **The node's and the Solve page's centroids** (FR-004, FR-005): for a frame holding a
   saturated star cedar sets aside, they no longer equal `detect_stars`'s, and a raw
   recording's frame line no longer replays through `detect_stars` alone (025); with
   none, nothing changes.
3. **One existing test replaced** (FR-004; Principle II):
   `saved_finder_frame_has_identical_nonempty_centroids_in_console_order`
   (`crates/scope-node/tests/detect.rs:26`) holds the node's centroids on
   `fixtures/scope-node/finder-frame.png` equal to `detect_stars`'s; that frame holds a
   saturated star of 29 pixels at 250 or above that cedar sets aside, so it fails with
   the pass (run). US2 replaces it with
   `saved_finder_frame_keeps_its_saturated_star_first_then_the_consoles_centroids_in_order`
   (US2-S1), which keeps every one of its checks and holds the rest of the list to
   `detect_stars`'s exactly. It also breaks a promise: the fixture's README says that
   when the frame is swapped for the finder camera's own "no test changes"
   (`fixtures/scope-node/README.md:3-7`); US2-S1 pins this frame's 29 pixels, so a swap
   changes it. The frame is the bring-up camera's, not the Mini's.
4. **The glossary** (FR-008): **Saturated star**, with the one saturation level.
5. **The README sections and `solve_image --saturated`** (FR-007, FR-009).
6. **A Principle I exception** (FR-001): the pass is grounded in tetra3's spot extraction
   for its labelled regions, its least area of 5 pixels, its axis ratio of 1.5 and its
   centre of mass plus 0.5 (plan.md § Constitution Check, row I), and these are this
   spec's own, each with the measurement behind it in owner decision 3 or 7:
   8-connectivity (tetra3 labels with scipy's default 4-connectivity); the level 250
   (cedar-detect counts a pixel saturated at exactly 255; 250 is 051's focus aid's);
   the frame bound of 24,576 pixels; the trail's 10° and 3 regions; the ring 3 frame
   pixels out from the box (cedar-detect's perimeter lies three binned pixels out from
   its core, 6 frame pixels at binning 2) and its share of 0.5.
7. **A new fixture** (FR-006): `fixtures/finder/tycho-pleiades.csv`, the 203 stars of
   the kit's narrow Tycho database (`tyc_main`, proper motion to 2026.9, fields 1.6° to
   1.85°) within 1.3° of the Pleiades' field, brightest first, 5,329 bytes, from which
   US2-S4 builds a narrow database in the test's own folder.

## User Scenarios & Testing *(mandatory)*

Every scenario shares these definitions.

- **The gate command.** Every scenario runs in
  `cargo nextest run --workspace --all-targets --all-features --locked`, the `test`
  gate's first command, each test in its own process.
- **A drawn frame** is a 640 × 480 frame of the test's own making: at each pixel's
  centre (column + 0.5, row + 0.5), background 10 plus each light the scenario names,
  rounded and held to 0 to 255. **A Gaussian of σ and peak P at (x, y)** adds
  P·exp(−r²/2σ²), r the distance from (x, y); **a Moffat of α, β and P** adds
  P·(1 + r²/α²)^−β; **trailed L px** means the mean of the light over 2L centres evenly
  spaced along a segment of length L at 20° (direction (cos 20°, sin 20°)), its middle
  at (x, y). **cedar's list** is `detect_stars` of the frame at sigma 8, binning 2, rows
  normalised and hot pixels rejected, the node's settings.
- **A bright frame** is the first frame of a `SimCamera` at 500 ms and gain 2 over a
  simulator on `crate::support::live::DB` with `Site::default()` and a `VirtualClock`
  from `crate::support::live::START`, its optics 1280 × 960 at 10.5° with the eyepiece
  at (640, 480), `point_eyepiece_at` the field, and `set_finder_focus_error(3.0)`: the
  finder camera's pixel grid at a 10.5° field. **The Orion Nebula's field** is RA
  83.82°, Dec −5.39°; **the Pleiades'** RA 56.75°, Dec 24.12°. **Its drawn stars** are
  `field_stars(500.0, 2.0)` read before the frame, in their order; **its truth** is
  `telescope().camera`.
- **The Pleiades' narrow database** is built by the test from the 203 rows of
  `fixtures/finder/tycho-pleiades.csv` (degrees, brightest first), each a `CatalogEntry`
  with `ra` and `dec` in radians, as a `CatalogEntry` takes them
  (`crates/database-generation/src/catalog.rs:19-22`), `id` `CatalogId::Hip` of its row
  number from 1 and no proper motion, with `database_generation`'s `clean_and_limit`,
  `enumerate_patterns` for fields 1.6° to 1.85° (150 stars a field, oversampling 20, 50
  patterns a field) and `build_pattern_catalog` (error 0.001), and kept as one complete
  file, `052-tycho-pleiades.npz` in `CARGO_TARGET_TMPDIR`, with 047's
  `DatabaseProperties` but `min_fov` 1.6, `max_fov` 1.85, `star_catalog` `tyc_main` and
  `epoch_proper_motion` 2026.9, as 047's narrow test database is kept. Like 047's, it is
  built only when the file is missing (`crates/finder/tests/console/databases/mod.rs:64`),
  so a file built wrong stays for every later run until it is deleted. **A field
  frame** is 1280 × 960: background 10 plus, for each of `field_stars(1e9, 1.0)` of a
  simulator on that database with optics 1280 × 960 at 1.617°, eyepiece (640, 480), pointed at the
  Pleiades' field, a light of peak 255·10^(−0.4(m − 8.5)) within a reach of its place,
  rounded and held to 0 to 255: the finder camera's own geometry.
- **The console's solve** is `finder::solve::solve_centroids` with
  `SolveOptions::default()` and the frame's size, as the live loop calls it
  (`crates/finder/src/live.rs:40`); it sends the first 30 centroids.
- **Oracles.** The drawn places and shapes; the simulator's drawn stars and true
  direction; the saved finder camera frame's own pixels; cedar's list, which this spec
  does not change; the constants as the README states them. No expected place comes from
  the code under test.

### User Story 1 - The pass keeps saturated stars (Priority: P1)

As the observer, when the finder camera's brightest stars saturate, I need detection to
keep them as stars, round or trailed with the rest of the frame, placed by their
brightness, so that the solve finds the field the patterns were built for, and hot
pixels, nebulae, merged doubles and a washed-out frame stay out.

**Why this priority**: The node and the console (US2) need the pass; without it a field
whose brightest stars saturate gives `no_match`.

**Independent Test**: `crates/star-detection/tests/saturated.rs`, new (US1-S1 to
US1-S5). The `test` gate runs it; during development,
`cargo nextest run -p star-detection --locked --test saturated`.

**Acceptance Scenarios**:

1. **Given** drawn frames each of one Gaussian at (p, p) for p = 200.5, 200.75 and 201.0, of σ 1.5, 3, 3 and 5 and P 3000, 3000, 30000 and 3000, two drawn frames of one Moffat at (200.5, 200.5) of α 2, P 25500 and β 2.5 and 1.8, and one of a Gaussian at (200.5, 200.5) of σ 1.5 and P 340, a top of five pixels, each also holding a Gaussian at (400.5, 300.5) of σ 1.5 and P 100, **When** a committed test calls `saturated_stars` on each with cedar's list and `with_saturated_stars` with it and with a list of two stars of brightness 10⁶ at (50.5, 50.5) and 10 at (600.5, 50.5), **Then** it asserts that each keeps exactly one star, within 0.1 px of (p, p) in x and y (of (200.5, 200.5) for the Moffats and the small one), with `peak_value` 255 and `num_saturated` the frame's count of pixels at 250 or above, and sets nothing aside; that `with_saturated_stars` gives that star followed by cedar's list unchanged, and the two-star list with that star between them; that cedar's list holds a star within 0.5 px of (400.5, 300.5) and none within 3 px of the kept star's place; and that for the Gaussian of σ 3 and P 3000 at (200.5, 200.5) the kept star's brightness is within 10⁻⁶ of the sum, over the pixels from 2 px outside its saturated pixels' box on each side, of each pixel less the mean of the one-pixel perimeter 3 px outside that box.
2. **Given** drawn frames of a segment from (150, 100) to (450, 340) at 400 within 1, 2 and 3 px of it; of two Gaussians of σ 3 at (300.5 ∓ d/2, 240.5), P 3000 and 3000 for d = 8, 10 and 12 and P 3000 and 800 for d = 8 and 12, and P 3000 and 3000 for d = 12 under a Gaussian of σ 40 and P 200 at (300.5, 240.5), a long region with a bright ring; of a Gaussian of σ 40 and P h at (320.5, 240.5) plus one of σ s and P c there, for (h, s, c) = (200, 8, 400), (60, 8, 400), (0, 25, 400) and (150, 3, 3000); of three Gaussians at (300.5, 230.5), (330.5, 250.5) and (315.5, 270.5) of σ 30, 25 and 20 and P 180, 180 and 150; of a Gaussian of σ 3 and P 3000 at (5.5, 240.5); of one Gaussian at (200.5, 200.5) of σ 1 and P 1000, a top of nine pixels cedar keeps; three frames of background 10 with 255 in a block of 1 × 1, 2 × 2 and 4 wide by 1 high from (320, 240); and one of background 10 with 255 in two 3 × 3 blocks from (300, 240) and (303, 243), touching at one corner, **When** a committed test calls `saturated_stars` on the first six kinds with cedar's list and on the blocks with no stars, **Then** it asserts that none keeps a star; that the segments and the pairs, the one under the halo too, are each set aside as `[Elongated]`, the five nebulae as `[Extended]`, the star at 5.5 as `[Edge]`, the top cedar keeps as `[]` with cedar's list holding a star within 0.5 px of (200.5, 200.5), the 1 × 1 block as `[]`, the other two blocks as `[Small]` and the two touching blocks as `[Elongated]`; and that `with_saturated_stars` gives each of the first six kinds' cedar's list unchanged.
3. **Given** drawn frames of two Gaussians of σ 3 and P 3000 at (300.5 ∓ d/2, 240.5) for d = 4, 6, 16 and 24, **When** a committed test calls `saturated_stars` on each with cedar's list, **Then** it asserts that none sets anything aside, every kept star is within 0.1 px of y 240.5, at d = 4 and 6 one star is kept within 0.1 px of x 300.5, and at d = 16 and 24 two are kept within 0.1 px of 300.5 ∓ d/2.
4. **Given** drawn frames holding, trailed 9 px, Gaussians of σ 1.5 and P 3000 at the first 5, 3 and 2 of (100.5, 100.5), (300.5, 150.5), (500.5, 100.5), (150.5, 350.5) and (450.5, 380.5), and in each two untrailed Gaussians of σ 3 and P 3000 at (295.5, 300.5) and (305.5, 300.5), and a drawn frame holding only such trails at the first 2 and at (5.5, 240.5), **When** a committed test calls `saturated_stars` on each with cedar's list, **Then** it asserts that the frames of 5 and 3 trails keep exactly their 5 and 3 trailed stars, each within 0.1 px of its place in x and y, and set the pair aside as `[Elongated]`; that the frame of 2 trails keeps none and sets aside `[Elongated, Elongated, Elongated]`; and that the frame with a trail at (5.5, 240.5) keeps none and sets aside `[Elongated, Elongated, Edge]`, the trail set aside at the edge not counted toward the frame's trail.
5. **Given** drawn frames of a disc, every pixel whose centre lies within r of (320, 240) 1000 above the background, for r = 88 and 89, and a frame of 640 × 480 pixels all 255, **When** a committed test calls `saturated_stars` on each with no stars and `with_saturated_stars` on the white frame with none, **Then** it asserts that the disc of 88 keeps one star within 0.1 px of (320, 240) and sets nothing aside, that the disc of 89 and the white frame keep none and set aside `[Extended]`, and that `with_saturated_stars` gives the white frame no star.

### User Story 2 - The scope node and the console keep saturated stars (Priority: P1)

As the observer, I need the scope node to send the saturated stars with the rest and
the console's Solve page to detect an uploaded frame the same way, so that the live
solve, GoTo and Track find a bright field, at the finder camera's own scale too, and a
frame I upload solves as the live loop solved it.

**Why this priority**: It is the fix the field night needs; it rests on US1.

**Independent Test**: the replaced test of `crates/scope-node/tests/detect.rs` (US2-S1)
and `crates/finder/tests/console/saturated_field.rs`, new (US2-S2 to US2-S5). The `test`
gate runs them; during development,
`cargo nextest run -p scope-node --locked --test detect` and
`cargo nextest run -p finder --locked --test console -E 'test(saturated_field)'`.

**Acceptance Scenarios**:

1. **Given** `fixtures/scope-node/finder-frame.png` as the existing test reads it, **When** a committed test lists its pixels at 250 or above and detects it with `scope_node::detect::detect` and `DetectSettings::cedar()` and with `detect_stars` at `DetectParams::cedar()`'s settings, **Then** it asserts that the frame and its size are as the fixture's `fixtures.json` says, that its pixels at 250 or above are exactly (20, 186), (19 to 24, 187), (19 to 24, 188), (19 to 25, 189), (20 to 24, 190) and (21 to 24, 191) as (x, y), that `detect_stars` gives a non-empty list with no star within 5 px of (22.5, 189.5), that the node gives one centroid more, the first within 10⁻⁹ px of (632/29 + 0.5, 5473/29 + 0.5), and that every later one equals `detect_stars`'s in order, x, y and brightness exactly.
2. **Given** a bright frame of the Orion Nebula's field and one of the Pleiades', **When** a committed test detects each with `scope_node::detect::detect` and `DetectSettings::cedar()` and with cedar's list cut to 200 as `Centroid`s, and runs the console's solve over `crate::support::live::DB` on both, **Then** it asserts that each of the 5 first drawn stars at least 20 px inside the frame has a node centroid within 1.5 px among the node's first 10 and no centroid of cedar's list within 3 px; that the console's solve of cedar's list answers `NoMatch`; and that of the node's centroids `MatchFound`, its pointing within 30″ of the frame's truth.
3. **Given** `AppState::shared` over `crate::support::live::DB` with simulation at `crate::support::live::START` over `SimulationTransport::Memory` and the live loop, its simulator's frames stopped, then set as a bright frame's simulator is for the Orion Nebula's field, `PUT /api/session` with `camera_settings` `{"exposure_ms": 500, "gain": 2}` answered 200, and the frames started again, **When** a committed test under `#[tokio::test(start_paused = true)]` reads live reports until three are 1280 wide, **Then** it asserts that each of those three is `MatchFound` with its pointing within 60″ of the simulator's camera direction read then, and that they come within 30 s of `START` on the simulator's clock.
4. **Given** the Pleiades' narrow database built from its 203 rows and two field frames of the Pleiades, one of Moffats of α 2.5 and β 1.8 within a reach of 64 px and one of Gaussians of σ 1.3 trailed 9 px within a reach of 16 px, **When** a committed test detects each as a `GrayFrame` with `scope_node::detect::detect` and `DetectSettings::cedar()` and with cedar's list cut to 200, and runs the console's solve over that database on both, **Then** it asserts that the console's solve of cedar's list answers `NoMatch` on each frame, and of the node's centroids `MatchFound`, its pointing within 30″ of the simulator's `telescope().camera`.
5. **Given** a bright frame of the Orion Nebula's field and one of the Pleiades', each encoded as a PNG, **When** a committed test decodes each with `finder::solve::decode_gray` and solves it with `finder::solve::solve_frame` over `crate::support::live::DB` and `SolveOptions::default()`, as the Solve page does, and detects the frame with `scope_node::detect::detect`, **Then** it asserts that the report's first 200 detected places equal the node's centroids' (x, y) in order, and that the report is `MatchFound` with its pointing within 30″ of the frame's truth.

### User Story 3 - The README says which saturated regions are stars (Priority: P2)

As the observer, I need the node's README to say that saturated stars are kept, which
regions are set aside and why, that a star frame does not say so, that the Solve page
detects the same way, and how to replay a recorded raw frame as the node detected it.

**Why this priority**: The behaviour works without it; the next field night's checks
and the replay of its raw frames need it.

**Independent Test**: `crates/scope-node/tests/saturated_readme.rs`, new. The `test`
gate runs it; during development,
`cargo nextest run -p scope-node --locked --test saturated_readme`.

**Acceptance Scenarios**:

1. **Given** the committed `crates/scope-node/README.md`, `GLOSSARY.md` and `crates/plate-solver/examples/solve_image.rs`, **When** a committed test reads the README's section from its line `## Bright stars: saturated stars` to the next `## ` heading, its whitespace runs read as one space, **Then** it asserts that the section holds `its brightest pixels read 250 or more`, `at least 5 pixels, tetra3's smallest spot`, `its length at most 1.5 times its width`, `which is kept when at least 3 of the frame's long regions lie within 10 degrees of their common direction`, `the ring 3 pixels out from it averaging less than 0.5 of the way from the frame's median to 250`, `whole, that ring inside the frame`, `A frame holding more than 24576 saturated pixels`, `placed before the first of cedar-detect's stars that is fainter, so the list stays brightest first`, `the node keeps the first 200 as before`, `A star frame does not say which centroids are saturated, nor how many regions were set aside.`, `The console's Solve page detects an uploaded frame as the node does.`, `--saturated <frame>` and ``It does not combine with `--dark` ``, each number formatted from `SATURATED`, `MIN_PIXELS`, `MAX_AXIS_RATIO`, `TRAIL_MIN_REGIONS`, `TRAIL_TOLERANCE_DEG`, `RING_MARGIN`, `MAX_RING_SHARE` and `MAX_SATURATED_PIXELS`; that the glossary holds a line `**Saturated star**:` followed by a line beginning `A star whose brightest pixels read 250 or more`; and that the example holds `[--noise-floor F] [--saturated] [--stars N]`, a doc line beginning ``//! * `--saturated` —`` and `--saturated does not combine with --dark`.

### Edge Cases

- **Two stars run into one round top** closer than about the top's own radius are kept
  as one star at their middle (US1-S3 at 4 and 6 px); 8 to 12 px apart they are set
  aside as elongated (US1-S2, US1-S4); farther apart, two stars (US1-S3). In a trailed
  frame a close pair lying along the trail is kept as one star at its middle (decision
  7's one false star, about 3 px from each).
- **A star embedded in nebulosity** whose ring stays bright is set aside with the
  nebula (US1-S2's (150, 3, 3000)); a smooth round knot with dark surroundings is kept,
  since by shape it is a star.
- **A saturated star near the frame's edge** whose ring would cross it is set aside
  (US1-S2), as cedar refuses a blob near the edge.
- **A planet** is a round saturated top and is kept as a star; drawn into the Pleiades'
  field at the finder camera's geometry as a false star brighter than all but a few, the
  frame still solved (run).
- **The Moon, a white frame or a twilight sky**: a frame with more than 24,576 pixels at
  250 or above is set aside whole (US1-S5), and the node sends cedar's list as before;
  the live report then shows cedar's result, for a white frame `too_few` with its hint.
  A drawn Moon beside stars thus gives `no_match`, as today, where keeping the stars
  beside it would have solved.
- **A top that clips below 250** (a camera, a conversion, or a low gain where the full
  well fills before the converter) gets no help: the pass looks only at 250 or above.
- **Hot pixels**: a lone one is no region; a block of 2 to 4 is small (US1-S2). A round
  block of 5 or more, dark around, that cedar does not keep would be kept as a star (not
  drawn; § Assumptions).
- **More than 200 stars**: the kept stars take their place by brightness, so the node's
  cut to 200 drops the faintest, saturated or not.
- **A frame with no pixel at 250 or above** gives exactly cedar's list, as today; the
  pass costs one scan of the frame (plan.md § What was run gives the cost as a ratio).
- **The finder camera's focus aid** (051) measures no star with a pixel at
  `finder_focus::SATURATED` or above, now the same constant as the pass's level; a
  saturated star now in the list is skipped by it as before, and its neighbours within
  the aid's box are no longer measured. 051's tests pass unchanged (run).

## Requirements *(mandatory)*

### Functional Requirements

**The pass** (US1):

- **FR-001**: `star-detection` SHALL gain `pub mod saturated` with `SATURATED` 250,
  `MIN_PIXELS` 5, `MAX_SATURATED_PIXELS` 24576, `MAX_AXIS_RATIO` 1.5,
  `TRAIL_TOLERANCE_DEG` 10, `TRAIL_MIN_REGIONS` 3, `RING_MARGIN` 3, `MAX_RING_SHARE`
  0.5, the enum `SetAside` (`Small`, `Edge`, `Elongated`, `Extended`), and
  `saturated_stars(image, width, height, stars) -> (Vec<Star>, Vec<SetAside>)`, which
  answers no star and `[Extended]` when the frame holds more than `MAX_SATURATED_PIXELS`
  pixels at `SATURATED` or above, and otherwise takes each 8-connected region of such
  pixels in reading order of its first pixel; passes over a region of one pixel or whose
  box holds a star of `stars`; sets a region aside as `Small` below `MIN_PIXELS` pixels
  and `Edge` when its box widened by `RING_MARGIN` on each side leaves the frame; takes
  it as long when the root of its second moments' major over minor eigenvalue exceeds
  `MAX_AXIS_RATIO`, and sets it aside as `Elongated`, whatever its ring, when the minor
  is not positive or when it is long and no trail stands or its major axis lies more
  than `TRAIL_TOLERANCE_DEG` from the frame's trail; sets aside as `Extended` a region
  whose widened box's one-pixel perimeter (its ring) averages more than `MAX_RING_SHARE`
  of the way from the median of every 64th pixel of the frame to `SATURATED`; and keeps
  every other region as a `Star` at the mean of its pixels' centres, `peak_value` its
  brightest pixel, `num_saturated` its pixel count, and brightness the sum, over every
  pixel strictly inside the ring, of the pixel less the ring's mean, the kept stars as
  `Star` orders them. The frame's trail is the axial median of the major axes of the
  regions found long (none passed over or set aside before its shape is read): half the
  sum of their doubled angles' circular mean and the median of each doubled angle's
  difference from that mean, taken into −180° to 180°, of an even count the mean of the
  two middle differences; it stands only when at least `TRAIL_MIN_REGIONS` of them lie
  within `TRAIL_TOLERANCE_DEG` of it.
  `with_saturated_stars(image, width, height, stars)` SHALL give `stars` with each kept
  star placed before the first of them that is fainter, and `stars` itself when none is
  kept (owner decisions 2 to 4 and 7, owner approvals 1 and 6).
- **FR-002**: `detect_stars` and `detect_stars_with_noise_floor` SHALL keep their
  signatures, code and results, so that cedar's list still sets a wide saturated star
  aside (US1-S1) and cedar's parity tests pass unchanged (owner decision 1).
- **FR-003**: `finder::finder_focus::SATURATED` SHALL be
  `star_detection::saturated::SATURATED`, the finder camera's one saturation level, its
  name, value and the focus aid's behaviour unchanged (owner decision 3).

**The node and the console** (US2):

- **FR-004**: `scope_node::detect::detect` SHALL pass the stars it detects, with or
  without a noise floor, through `with_saturated_stars` before keeping the first 200,
  and its doc line SHALL say so (048's FR-002 changed; owner decision 1, owner approvals
  2 and 3).
- **FR-005**: `finder::solve::solve_frame_with_frame` SHALL pass the stars
  `detect_stars` gives through `with_saturated_stars`, so that the Solve page and MCP's
  `solve_frame` detect an uploaded frame as the node does (owner decision 1, owner
  approval 2).
- **FR-006**: Nothing else SHALL change: the scope link's protocol and the star frame,
  `DetectSettings` and the node's config, auto-exposure, every file of
  `crates/finder/src` but `solve.rs` and `finder_focus.rs`, the simulator's `src/`,
  `Cargo.toml` and `Cargo.lock`, every existing test but US2-S1's, and every existing
  fixture; the one new file outside `crates/` is `fixtures/finder/tycho-pleiades.csv`,
  its places in degrees where a `CatalogEntry` takes radians; US2-S4's test builds its
  narrow database from it only when the database's file is missing (owner decision 5,
  owner approval 7).

**Words and replay** (US3):

- **FR-007**: The node's README SHALL gain the section `## Bright stars: saturated
  stars`, before `## The mount link`, in plan.md § Interfaces' words; and the console's
  README's live stream section a paragraph, before `` `GET /api/live` is
  `text/event-stream`. ``, saying the live solve takes the node's centroids with the
  saturated stars among them and an uploaded frame is detected the same way.
- **FR-008**: `GLOSSARY.md` SHALL gain **Saturated star**, after **Centroid**, in plan.md
  § Interfaces' words (owner approval 4).
- **FR-009**: The `solve_image` example SHALL take `--saturated` and, when given, pass
  its detected stars through `with_saturated_stars`, print `<image>: saturated stars kept
  <k>, set aside <a> (small <s>, edge <e>, elongated <l>, extended <x>)` before its result
  line, or add `"saturated_kept"` and `"saturated_set_aside"` to its JSON, and refuse
  `--saturated` with `--dark` with the usage message `--saturated does not combine with
  --dark: the scope node subtracts no dark`; its usage line and doc comment SHALL list
  the flag (owner decision 5, owner approval 5).

### Key Entities

- **Saturated star**: a star whose brightest pixels read 250 or more, the finder
  camera's one saturation level, so its top is flat; the node and the Solve page keep
  one cedar's gates set aside, by its brightness, when it is round or lies along the
  frame's trail, dark around and whole in the frame.
- **Region**: an 8-connected set of pixels at 250 or above.
- **Ring**: the one-pixel perimeter of a region's box widened by 3 pixels on each side,
  standing for the sky around it, as cedar's perimeter does for a core.
- **The frame's trail**: the axial median of its long regions' major axes, standing when
  at least 3 of them lie within 10° of it.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario has a committed test that asserts it, and the
  `lint` and `test` gates pass.
- **SC-002**: Every existing test passes unchanged but the one owner approval 3
  replaces; no golden, existing fixture, `scripts/ci/` file, `.github/` file,
  `CODEOWNERS` entry, `ergane.yaml` or `playwright.config.ts` is edited, and the one
  new fixture is owner approval 7's.
- **SC-003**: On drawn frames at the finder camera's own 1.617° over a narrow database
  of Tycho stars around the Pleiades, untrailed with heavy wings and trailed 9 px, and on
  the simulator's Orion Nebula and Pleiades fields on the finder camera's pixel grid at a
  10.5° field, the node's centroids solve within 30″ and the live solve within 60″,
  where cedar's detection alone gives `no_match` (US2-S2 to US2-S4). These are drawn or
  simulated frames, not the finder camera's; frames trailed by more than about half a
  saturated top solve only through decision 7's trail rule, and a real frame, trailed or
  not, is the next field night's (plan.md § Outside the factory).

## Assumptions

- **Earlier features are in place.** Every spec in `depends_on_landed` has landed. Every
  citation here was read at f6d4f5fd.
- **The scope** is the scope node's detection and the console's frame detection, with
  reaches into `star-detection` (the new module), the focus aid's constant, the
  console's test binary, a new fixture, the plate solver's example (US3) and the
  glossary. The live report, the scope link, sync intake (spec 053), the solver's
  matching, the databases, the node's 8-bit read and auto-exposure are out.
- **The drawn and simulated frames stand for the field night's** in their symptom: the
  brightest stars flat and missing, tens of stars detected, `no_match`. US2-S4 draws the
  finder camera's own scale over a narrow database of 203 stars; the bright frames are
  at about 30″ a pixel. Whether a real frame solves is the next field night's (plan.md
  § Outside the factory).
- **Hot pixels on the finder camera are single**: not measured on the Mini. A round
  block of 5 or more saturated hot pixels that cedar does not keep would be kept.

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
  implements: [FR-007, FR-008, FR-009]
```

Three stories, each under 60 % of 65,536 B (39,321 B) as the probe from this trio
measured them, with the fold's steps added to US1: US1 28,931 B (44 %), US2 25,069 B
(38 %), US3 14,397 B (22 %); at the factory's worst overrun seen, 1.35 times, 39,057 B,
33,843 B and 19,436 B, still under the line, US1 by 264 B.
The draft had two; the fold's gates and tests pushed its first story to about 50 KB, so
the node and the console's use of the pass is now a story of its own. US2 merges after
US1 (`depends_on_merged`): it calls US1's `with_saturated_stars`; US3 merges after US2:
its README test reads US1's constants, `solve_image` calls US1's functions, and its words
describe the node and the Solve page as US2 leaves them.
