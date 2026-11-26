---
state: draft
depends_on_landed: ["013-live-finder", "016-mcp", "025-frame-capture", "033-focus-reading-and-aid", "047-console-uses-the-database-that-fits", "048-node-detects-stars-on-a-dark-sky", "050-tools-page-aligns-finder-camera"]
# Drafted 2026-11-26 against `main` at c75a6e76, at the owner's ask of 2026-11-25
# ("we should probably just generally have some utility pages in the system. another
# one for focusing an imaging node (finder or the imager itself)"), from a probe of that
# day on the branch `probe/051` cut from c75a6e76, and folded the same day through a
# cynical review (round 1: a blocker, six majors and fourteen minors) with the probe
# extended to 43c7ef94 (plan.md § What was run and what was read), then probe-built
# from this text alone on `probe/051-r1`, cut from 9b139bac (round 1: tip 77491a47,
# every scenario and gate passing, 0 blocking and 10 clarity defects), whose fold
# added 1a87ce8a, 9b691e75, d577c82d and 9ef38442 there (tip 9ef38442, every gate
# passing). Every `path:line` here was read at c75a6e76. State: draft. The ten owner
# decisions are open, each with a recommendation the scenarios are written to; the flip
# is the owner's. 025 is named because the aid reads its raw frames; 033 because it
# judges every reading with 033's arithmetic and reuses 033's Focus block; 048 because
# the aid measures at the stars the scope node's detection finds; 013 for the
# simulator's finder camera and the live loop; 016 for MCP; 047 and 050 because each last
# changed a file this spec writes (plan.md § Refinement Record).
---

# Feature Specification: The Tools Page Focuses Either Camera

**Feature Branch**: `051-tools-page-focuses-either-camera`

**Created**: 2026-11-26

**Status**: Draft

**Input**: The owner focuses the finder camera, a ZWO ASI120MM Mini (1280 × 960 at
3.75 µm, mono, read as 8-bit by the scope node, 045) behind the SVBONY SV165's own 40 mm
f/4 objective (160 mm, an achromat), about 1.72° × 1.29° at about 4.83″ a pixel, by hand
with the guide scope's helical focuser, and will refocus it after each spacer the owner
tries, and after a focal reducer if one is ever added (that is not settled, and nothing
here assumes one). Today nothing tells the owner how sharp the finder camera's stars
are, beyond solving or not. The imaging camera already has a focus aid (033) on the
Imaging page. Give the Tools page (050) a Focus block where the owner picks the camera,
turns the focuser and watches one number, its trend and the best so far get better,
with a plain line once they have passed the best and come back to it, for either
camera, without building 033's aid twice.

## Clarifications

### Session 2026-11-26 (the probe on `main` at c75a6e76)

- Q: What does this trio read besides itself? → A: Nothing about the feature. `spec.md`
  holds every requirement and scenario, and `plan.md` every design decision, the
  interfaces, the shapes that existing tests pin and the alternatives rejected. The
  probe's branch and logs are not in the repository; what they showed is written here.
- Q: What does 033 measure, and can it run on the finder camera's frames? → A: 033's
  focus reading is the imaging solver's star measure (`focus::read`,
  `crates/finder/src/imaging/focus.rs:55`, asks the solver's `measure`,
  `crates/finder/src/imaging/focus.rs:65`): ASTAP's `-analyse` median half-flux diameter
  of a saved imaging frame, or of a tile of it, with fewer than `MIN_STARS`, five
  (`crates/finder/src/imaging/focus.rs:16`), saying nothing and above `FAR_OUT_PX`, 20
  (`crates/finder/src/imaging/focus.rs:21`), far out. The simulated imaging solver's
  measure does not read pixels: it answers from the simulator's imaging truth and the
  imaging optics' focus error (`crates/finder/src/imaging/simulated.rs:149`,
  `crates/finder/src/imaging/simulated.rs:156`), so on the gates 033's measure of a
  finder camera frame would report the imaging camera. 033's arithmetic is apart from
  its measure: `Aid` (`crates/finder/src/imaging/focus/aid.rs:118`) takes `Sample`s
  (`crates/finder/src/imaging/focus/aid.rs:33`), a size and a star count or why there is
  none, with no clock, camera or file, and gives the median of three, a change at 15 %
  (`crates/finder/src/imaging/focus/aid.rs:13`), the trend, the best of the last 600
  settled readings (`crates/finder/src/imaging/focus/aid.rs:27`), `found` and the last
  120 samples (`crates/finder/src/imaging/focus/aid.rs:29`). It runs unchanged on any
  sizes (run: the probe fed it the finder camera's readings).
- Q: What does the console have of the finder camera's frames? → A: Each star frame
  carries at most 200 centroids, brightest first, each `x`, `y` and `brightness`
  (`crates/scope-link/proto/scope_node/v1/scope_node.proto:65`,
  `crates/scope-link/proto/scope_node/v1/scope_node.proto:72`), and the exposure and
  gain actually used, automatic or not
  (`crates/scope-link/proto/scope_node/v1/scope_node.proto:60`): the scope node's
  detection drops the peak value and the saturated count that `star_detection` measures
  (`crates/scope-node/src/detect.rs:68`), so a star frame says where the stars are and
  not how large. 025's raw frame (`crates/scope-link/proto/scope_node/v1/scope_node.proto:153`)
  answers the next star frame's 8-bit pixels, `width × height` bytes
  (`crates/scope-link/proto/scope_node/v1/scope_node.proto:162`), with that star
  frame's id. The node sends the star frame first and then answers every waiting raw
  request from the same pixels (`crates/scope-node/src/service.rs:324`,
  `crates/scope-node/src/service.rs:339`): a raw request costs the node no exposure and
  no detection, and the star frames go on. The console reaches it through
  `LinkManager::raw` (`crates/finder/src/link.rs:347`) and the star frames through
  `LinkManager::frames` (`crates/finder/src/link.rs:338`). The preview JPEG comes at
  most every 5 s and is box-averaged to 640 pixels on its long side, too coarse for a
  star of one or two pixels.
- Q: Can a test defocus the simulator's finder camera? → A: Not at c75a6e76. Its stars
  are drawn with a fixed Gaussian width, `PSF_SIGMA_PX`, 1.2 px
  (`crates/simulator/src/render.rs:10`), in a stamp of radius 8
  (`crates/simulator/src/render.rs:74`); 033's focus error is the imaging optics'
  (`crates/simulator/src/optics.rs:299`). `render::render` is called by an existing test
  outside the simulator, so its signature stays. The smallest change (owner approval 1)
  is a finder camera focus error kept beside the pointing
  (`crates/simulator/src/optics.rs:74`) and a `render_with_sigma` that `render` calls at
  1.2 px, drawing exactly the pixels it drew before (run: the FNV-1a hash of
  c75a6e76's own pixels, computed at c75a6e76, US1-S4).
- Q: How does a focus run end? → A: By a stop, or `failed` when the scope node is not
  connected, cannot send raw frames, or sends none for 15 s: 025's limit and words
  (`crates/finder/src/recording.rs:33`, `crates/finder/src/recording.rs:369`,
  `crates/finder/src/recording.rs:371`, `crates/finder/src/recording.rs:377`); or when
  the frames it measures change exposure or gain, or a measure fails (owner decision 3).
  The silence and the changes of exposure and of gain are run; the rest are read
  (plan.md § What was run and what was read).
- Q: Can the aid be used by day? → A: No: it measures stars. A rough focus by day on a
  distant object is the owner's, outside this tool (plan.md § Outside the factory).

### Session 2026-11-26 (cynical review, round 1, folded)

- Q: How large is a focused star on this optic? → A: About a pixel. A 40 mm objective's
  Airy disc is about 2.9″, and with 2″ to 3″ of seeing a star is about 3.8″ across, 0.8
  px at 4.83″ a pixel, about 1.05 px once the pixel itself is added: σ about 0.33 to
  0.45 px. The critical focus zone at f/4 is ±2.44·λ·N², about ±21.5 µm. The draft's
  scenarios looked only from σ 1.2 px up, with stars on pixel centres. The draft's
  measure read radii between pixel centres, so below σ 1.2 px it stepped with a star's
  place on the pixel grid (run: 0.000 px at σ 0.35 to 0.5 on a pixel centre, 2.000 px at
  σ 0.7 to 1.0, and 1.414 px from σ 0.35 to 0.85 on a pixel corner). The measure is now a
  sub-pixel one (owner decision 1).
- Q: Does the reading depend on the exposure? → A: The draft's did: its three-noise cut
  trimmed a faint star's wings, so it read +24 % from 200 to 1000 ms at one focus (run).
  The folded measure sums every pixel of a star's circle with no cut and moved at most
  3.5 % from half to four times one exposure on the model of this optic, and at most
  2.3 % from 200 to 1000 ms on the simulator at focus errors 0 to 2 (run). The node's
  automatic exposure still steps by a quarter to four times whenever the star count
  leaves 25 to 40 (`crates/scope-node/src/exposure.rs:26`,
  `crates/scope-node/src/exposure.rs:29`), which turning the focuser does, and the
  star count shown beside the reading (owner decision 5) means something only at one
  exposure. So a run asks for a fixed exposure and gain, and ends when they change
  (owner decision 3).
- Q: Does any of this come from the real camera behind the real objective? → A: No.
  The real frames read so far are an earlier finder camera's (an OV5647 behind a 16 mm
  lens, about 36″ a pixel); the simulator's are about 30″ a pixel with σ 1.2 px; the
  rest is the review's model of this optic. Owner decision 10 says when a night series
  of the real optic comes.

### Session 2026-11-26 (probe round 1 from this text, folded)

- Q: Where are a star's offsets taken from, and when are two distances one? → A: From
  the centre of the box's centre pixel, and distances within 10⁻⁹ px of the one before
  are one (FR-002). Built in cedar's coordinates with exact ties, six identical
  noise-free stars at σ 0.5 and phase 0 read 1.2332 or 1.2649 px by their place in the
  frame, since float rounding splits or joins equal distances there (run).

## Owner decisions

All ten are open. Each has its options, its evidence and a recommendation; the
scenarios and the record are written to the recommendations. One general rule is
behind decisions 1 to 3 and 7: **the finder camera's aid adds no rule of its own to
033's arithmetic, takes nothing from the live solve and changes no setting; it asks only
that the one setting its readings depend on, a fixed exposure and gain, be set on Setup,
and it ends when that changes.** It reads only what the scope link already carries, the
raw frames of 025 and each star frame's centroids, exposure and gain, measures each raw
frame and hands the result to 033's `Aid` as a `Sample`, with 033's own five stars and
20 pixels. So it holds no lease, remembers and puts back no setting, needs no rule for
Track, a GoTo or a change of optics, and keeps no record and gives no verdict; 033's
module and its tests are not touched. Its own parts are the measure, the run that feeds
it and its words.

1. **The finder camera's measure** (FR-002). Recommended (a): **a sub-pixel half-flux
   diameter in circles, the method of SEP's `flux_radius` written fresh**: at the
   centroids of each raw frame's own star frame, the median over at most 15 of the
   brightest stars whose box of 33 × 33 pixels lies inside the frame and holds no other
   centroid, and whose own pixels (those more than three noise levels above the box
   edge's median, joined to the star's brightest pixel) are under 250; each star's size
   twice the radius, round the flux-weighted centre of its own pixels, within which the
   box's pixels, less the background and split into 5 × 5 parts, sum to half of what a
   circle one pixel past its farthest own pixel holds. Evidence (run, plan.md § What was
   run): on drawn stars integrated over each pixel, 1.002 to 1.060 times a Gaussian's own
   half-flux diameter from σ 1.2 to 3.0 at phases 0, 0.25 and 0.5 of a pixel; at σ 0.5
   and 0.8 rising with σ at every phase (1.233, 1.493 and 1.577 px and 2.124, 2.111 and
   1.984 px at phases 0, 0.25 and 0.5), its phase spread 28 % and 7 % (the draft's: from
   0 to 2.19 times); the median of 15 stars at random phases rises from σ 0.3 (1.18,
   1.29, 1.35, 1.52, 1.70 and 2.11 px at σ 0.3, 0.35, 0.4, 0.5, 0.6 and 0.8). On the
   review's model of this optic at a fixed exposure, inside the critical focus zone (0
   and 10 µm) the reading moves 3.3 % and 4.0 % from frame to frame and 033's `Aid`
   settles 38 of 40 frames at each, every frame after the first two (the draft's: 17.1 %
   and 15.5 %, settling 1 and 10), and outside it 1.4 % to 2.2 %, settling 38; it rises
   with the focus error (1.29, 1.35, 1.53, 1.80, 2.19 and 3.06 px at 0, 10, 21, 32, 43
   and 64 µm).
   On the simulator at 1000 ms, 1.021, 1.016, 1.003 and 0.995 times H(e) at focus errors
   0 to 3, 15 stars each. On the owner's eight 3 s frames of 2026-11-09 from the earlier
   finder camera (outside the repository), 3.16 to 3.64 px with 15 stars each, where the
   owner's own script read diameters of 2.42 to 3.12 and ASTAP 3.4 to 4.5. Instead: (b)
   **the draft's measure**, the joined pixels' half-flux diameter read between pixel
   radii: steps with a star's phase below σ 1.2 px, where a focused star of this optic
   lies; (c) **033's measure**, ASTAP's `-analyse` on the raw frame saved as a picture:
   on those eight frames, with 033's signal-to-noise floor of 30
   (`crates/finder/src/imaging/astap.rs:28`), it found 3 to 7 stars, under five in five
   of them; it needs an imaging solver on the console, and the gates' simulated one reads
   the imaging camera's truth, not the frame; (d) **the star count alone**: on a real
   sky at one exposure it peaks at focus (`docs/research/2026-11-15-imaging-camera.md:208`;
   the model: 29.4, 28.6, 25.8, 21.0 and 16.4 stars detected at 0, 10, 21, 32 and 43 µm),
   but it moves 3 % across the zone where the size moves 5 %, and it follows the sky as
   much as the focus; the simulator cannot show it rising toward focus, since it draws
   only stars brighter than its exposure's limit (`crates/simulator/src/render.rs:20`,
   `crates/simulator/src/render.rs:39`), all of them detected near focus; it is shown
   beside the reading instead (decision 5); (e) **a fitted width or second moments**:
   free of the grid in principle, but a fit needs a solver, a start and a stop rule two
   builds would have to share, and second moments weigh the noisy wings most; (f) **a
   size measured by the scope node** and sent on each star frame: the cheapest on the
   link, but a change to the proto and the node, out of this spec's scope.
2. **Where it is computed, and its cost** (FR-005). Recommended (a): **on the console,
   one raw frame per reading**: `width × height` bytes on the scope link, 1,228,800 B
   for the ASI120MM's 1280 × 960, and nothing more on the node, which already holds each
   frame's pixels and answers a raw request from them. The measure took 0.23 % of the
   simulator's 500 ms frame interval on a 1456 × 1088 frame of 141 centroids, on the
   probe's host and not on the Pi. The cadence is one reading per raw frame the link
   delivers, with no cap of its own. The console runs beside the scope node on the Pi,
   where the link is the Pi's own loopback, as the field kit is built (plan.md § Target
   Platform). Over a slower link, such as a phone's hotspot, one raw frame can hold up
   the link's heartbeat, every 250 ms, past its 1 s of silence
   (`crates/finder/src/link.rs:31`, `crates/finder/src/link.rs:32`), and drop the link
   (read, not run); a cap on the cadence would not shorten that one frame, so the README
   says where to run the console instead. Instead: (b) **on the scope node**, decision 1's
   (f); (c) **from the preview JPEG**, at most one every 5 s and box-averaged to 640
   pixels.
3. **One exposure and gain for a run** (FR-004, FR-007, FR-008). Recommended (a): **the
   start refuses while the session's finder camera exposure or gain is automatic**, 409
   `the finder camera's exposure or gain is automatic: fix both on Setup`, **and a run
   ends `failed`**, `the finder camera's exposure or gain changed during the run`, **when
   a measured star frame's exposure or gain differs from the run's first**. It holds no
   lease and changes no setting: the live solve goes on with the same frames, and Track
   may stay on. Evidence: decision 1's measure moves at most 3.5 % with the exposure, but
   the automatic exposure steps whenever the star count leaves 25 to 40, which turning
   the focuser does (the model: 29.4 stars detected at focus and 16.4 at 43 µm); a step
   changes which stars saturate and are measured, and the detected count beside the
   reading (decision 5) is held at 25 to 40 by the automatic exposure and means nothing
   under it. Replayed through 033's arithmetic, the owner's 3 s focus logs of the earlier
   camera, of 15.0 and 13.9 stars measured a frame, settled 95 of 104 and 44 of 62
   readings, and the 1 s logs, of 12.6 and 11.4, settled 21 of 224 and 15 of 214, most
   set aside as changes: an earlier camera and the draft's measure, but the README still
   says to start from the exposure the automatic control has chosen. Instead: (b) **start
   033's arithmetic again on a change**: the best so far is lost at every step, which
   comes while the owner turns through focus, so `found` would rarely come; (c) **hold a
   fixed exposure for the run**, remembered and put back, a lease-like rule of its own
   over the session's settings, and a console stopped mid-run leaves it held; (d) **any
   exposure, as the draft had it**: the count then means nothing and the star set moves
   under the reading.
4. **The imaging camera from Tools** (FR-015). Recommended (a): **033's Focus block
   itself**, `FocusBlock` rendered unchanged on Tools when the imaging camera is picked,
   over the Tools page's own imaging view; it stays on the Imaging page too. Built in the
   probe with `FocusBlock.tsx` untouched (run, Vitest and the browser). Instead: (b) **a
   link** to the Imaging page's Focus block, a second place to go; (c) **moving** the
   block to Tools, which changes 033's FR-031 and its tests.
5. **The display** (FR-014). Recommended (a): **033's lines and words for the finder
   camera, with the star count, and no record or verdict**: the median as the large
   number in the finder camera's pixels to two decimals; 033's status words with
   "finder camera frames" where 033 says "imaging frames"; `Stars detected` and the
   count of the measured frame's own star frame (free: it is that star frame's centroid
   count); `Best so far`; 033's trend words, "than before the last change"; the line once
   the reading has passed through focus and come back; the last 60 readings as bars. With
   too few stars measured, the page says to turn the focuser slowly only when the node
   itself detected fewer than five, and else that the rest are saturated, crowded or at
   the frame's edge. The status says the aid stopped only after a stop; a failed run is
   its alert alone. 033's own heading `Focus` sits inside the region `Focus a camera`
   when the imaging camera is picked: accepted, since `FocusBlock` is not edited (FR-017)
   and each is a region of its own name. Instead: (b) **arcseconds** from the live
   solve's scale, which an optic changes; pixels compare exactly for one camera behind
   one optic; (c) **033's record and verdict for the finder camera**, a second
   `focus.json` and its rules; (d) **the draft's words** ("since the last turn",
   "frames"), two sets of words in one block.
6. **MCP** (FR-010). Recommended (a): **two tools**, `get_finder_focus` (read-only) and
   `set_finder_focus` (`on`), as 033 and 050 have theirs, since an agent drives a night
   over the tunnel. Instead: (b) **none**; (c) **a `camera` argument on 033's tools**, a
   change to 033's FR-035 and FR-036.
7. **A change of optics mid-run** (FR-008). Recommended (a): **nothing of its own**:
   the first readings after a spacer are a change by 033's 15 % rule and the run goes
   on; the best so far stays the old optic's until the observer stops and starts again,
   which the README says to do. An optic that moves the focal plane leaves the frames out
   of focus and the live solve failing until refocused (read, not run). Instead: (b)
   **end the run** when the live solve's field width moves more than 5 % from the run's
   first; (c) **start the arithmetic again** on such a move.
8. **A browser test** (FR-016). Recommended (a): **`e2e/tools-focus.spec.ts` in the
   simulate project**, at 412 by 915, by day and by night, its locators held to the
   block's region, with two list entries in `playwright.config.ts` (owner approval 6). It
   fixes the exposure first through `PUT /api/session`, 200 ms at gain 1, as
   `crates/finder/web/e2e/goto.spec.ts:126` and three other specs do, and stops its run
   through `DELETE /api/finder-camera/focus` in its `finally`. The block draws bars, and
   only a browser paints its night colours. Run in the probe, 34 of 34 with it. Instead:
   (b) **none**, Vitest alone.
9. **The route's name** (FR-004). Recommended (a): **`/api/finder-camera/focus`**.
   Instead: (b) `/api/finder/focus`, where "finder" alone is the glossary's _Avoid_ for
   the camera; (c) `/api/focus/finder`.
10. **Evidence from the real optic** (plan.md § Outside the factory, step 1).
    Recommended (b): **flip on the simulator and the model, with a through-focus night
    series of the ASI120MM behind the SV165 as the first step outside the factory**.
    Instead: (a) **that series before the flip**, a plan prerequisite. The series: 5 to 7
    focuser positions through the best, about a twentieth of a turn apart, about 10 raw
    frames each at one fixed exposure (025's raw frames, at most 10 a request,
    `crates/finder/src/recording.rs:31`, or any capture program), on a tripod near
    Polaris, where a star trails under a pixel a second; each frame measured with
    decision 1's measure. Evidence for (b): nothing in the code depends on this optic (the
    measure's numbers are pixels and stars: a 33-pixel box, 15 stars, 033's five and 20);
    on the first night with the aid its own readings, history and count are the series,
    and 025's raw frames keep it for later; the model predicts the measure settles at
    focus. What would change the design, each a follow-up spec and not this one's code: a
    reading at the best position that moves more than 7 % from frame to frame, or that
    033's `Aid`, replayed over the position's 10 readings, settles on fewer than 4 of the
    8 that can settle (033's 15 % rule would then need a longer window, a rule of the
    finder camera's own); fewer than five measurable stars at every fixed exposure that
    keeps the brightest under 250; or a detected count at the sharpest position below
    two thirds of its highest a little off it (decision 5's count would then need a
    target on the page, not only in the README). A best reading above 5 px, a softer
    objective than modelled, changes nothing: the measure is then where it is easiest.

## Owner approvals

Flipping this spec to `ready` approves the answer to every owner decision above, and the
following. Each is named in the FR that makes it. No existing test, golden, fixture or CI
file changes.

1. **A change to the simulator** (FR-001): `Simulator::finder_focus_error` and
   `Simulator::set_finder_focus_error`, the focus error kept beside the pointing,
   `render::render_with_sigma`, `render::render` calling it at `PSF_SIGMA_PX`, and
   `SimCamera` drawing with the width the focus error gives. Nothing else in
   `crates/simulator` changes.
2. **New public API in `crates/finder`'s library** (FR-002, FR-004 to FR-007): the
   module `finder::finder_focus` with `STARS_MEASURED`, `BOX_PX`, `SATURATED`,
   `THRESHOLD_NOISE`, `SUBPIXELS`, `APERTURE_MARGIN_PX`, `RAW_WAIT`, `measure`,
   `FocusAsk`, `FocusView`, `LastView`, `MedianView`, `Refusal` and `FinderFocus`
   (`view`, `start`, `stop`); and `AppState::finder_focus`. No existing signature
   changes.
3. **New routes and tools** (FR-004, FR-010): `GET`, `POST` and
   `DELETE /api/finder-camera/focus`, and the MCP tools `get_finder_focus` and
   `set_finder_focus`.
4. **The glossary** (FR-011): **Focus reading**, **Focus aid** and **Raw frame** name
   the finder camera's focus aid.
5. **The finder README's section, rows and MCP paragraph** (FR-011). The pinned sentence
   `The imaging camera has sixteen tools of its own` (`crates/finder/README.md:88`)
   stays true: the two tools are in a paragraph of their own, after the finder
   alignment's.
6. **Two list entries in `crates/finder/web/playwright.config.ts`** (FR-016):
   `"**/tools-focus.spec.ts"` in the default project's `testIgnore` and in the simulate
   project's `testMatch`, each on a line of its own after `"**/tools.spec.ts"`
   (`crates/finder/web/playwright.config.ts:44`, `crates/finder/web/playwright.config.ts:63`).
7. **The page** (FR-014, FR-015): the Tools page's block `Focus a camera`.

## User Scenarios & Testing *(mandatory)*

Every scenario shares these definitions.

- **The gate commands.** Every Rust scenario runs in
  `cargo nextest run --workspace --all-targets --all-features --locked`, the `test`
  gate's first command, each test in its own process. The page's Vitest scenarios run in
  `npm test`, and the browser scenario in `CI=1 npm run e2e`, later commands of the same
  gate, in `crates/finder/web`.
- **H(e)**, the oracle of a simulated star's size, is a Gaussian's own half-flux
  diameter, 2·√(2 ln 2)·σ, at σ = √(1.2² + e²) px, the simulator's finder camera star
  width at focus error e: H(0) = 2.826 px and H(3) = 7.609 px.
- **A drawn frame** is a 640 × 480 frame of the test's own making, its pixel at column
  c and row r covering c ≤ x < c + 1 and r ≤ y < r + 1 (cedar's coordinates): background
  10, plus, for each star at (x, y) of density D, the integral over that square of
  D·exp(−((u − x)² + (v − y)²)/2σ²), which is D times the product of the two
  one-dimensional integrals, each to within 10⁻⁶ of the true value; rounded and held to
  0 to 255. D is 200 unless a scenario says otherwise. Each star's centroid is (x, y)
  with D as brightness. **The six places at phase p** are (100.5 + p, 100.5 + p),
  (300.5 + p, 100.5 + p), (500.5 + p, 100.5 + p), (100.5 + p, 300.5 + p),
  (300.5 + p, 300.5 + p) and (500.5 + p, 300.5 + p); **the six places** are those at
  phase 0, each on a pixel's centre.
- **A simulated frame at e** is the next frame of a `SimCamera` over a simulator on
  `crate::support::live::DB` with `Site::default()` and a `VirtualClock` from
  `crate::support::live::START`, pushed to altitude 50° and azimuth 120°, with
  `set_finder_focus_error(e)`, an exposure of 1000 ms and gain 1, measured at the
  centroids `scope_node::detect::detect` finds in it with `DetectSettings::cedar()`, the
  scope node's own detection.
- **The focus rig** is that simulator served by a `SimNode` with the token `test-token`
  and a 500 ms frame interval, reached over an in-memory duplex as
  `crates/finder/tests/console/imaging_align.rs` reaches its node; a `Session` paired
  with it whose `camera_settings` are `{"exposure_ms": 500, "gain": 1}`; and
  `AppState::shared` over that database with `with_session`, `with_clock` of the
  simulator's clock, `with_link` and `with_live()`, served by `finder::router`. **Live**
  means `GET /api/session` reads `pointing.source` `live`. **The settled reading after
  n** is `focus` of `GET /api/finder-camera/focus` once its `readings` is at least
  n + 3, its `median.readings` 3 and its `last.state` `reading`, read every 100 ms.
  **Turning the focuser to e** is `set_finder_focus_error(e)` mid-run. **A foreign
  write** is a request whose `Host` header is localhost on port 8080 and whose `Origin`
  header is `http://elsewhere.example`, as `crates/finder/tests/console/imaging_focus_aid.rs`
  sends one. The tests run under `#[tokio::test(start_paused = true)]`, and every state
  awaited comes within 120 s of paused time.
- **The MCP rig** is the focus rig served with `/mcp`, the tools called with
  `tools/call` as `crates/finder/tests/mcp/` calls them; **the bare console** is
  `AppState::shared` alone, with no link.
- **The page rig** is the component rendered by Testing Library with `fetch` stubbed to
  answer the views the test writes, as the page's existing tests do.
- **The simulated console** is the simulate project's server of
  `crates/finder/web/playwright.config.ts`, the release `finder` with `--simulate`.
- **Oracles.** H(e) and a drawn star's own 2·√(2 ln 2)·σ; a uniform disc's half-flux
  diameter, √2 times its radius; the drawn frames' own places and densities; for drawn
  stars too small for a closed form under the measure, the order of their sizes and the
  bounds the scenarios write; the FNV-1a hash of c75a6e76's own pixels, computed at
  c75a6e76; 025's and 033's words; and the words of these scenarios. No expected value
  comes from the code under test: no test calls `measure` for an expected value or reads
  `STARS_MEASURED`, `BOX_PX`, `SATURATED`, `THRESHOLD_NOISE`, `SUBPIXELS`,
  `APERTURE_MARGIN_PX` or `RAW_WAIT`.

### User Story 1 - The console reads the finder camera's focus, and the simulator turns it (Priority: P1)

As the owner, I need the console to measure how large the finder camera's stars are in
one of its raw frames, down to the pixel-sized stars of this optic, and the simulator's
finder camera to go out of focus so that this can be tested.

**Why this priority**: Without the measure and a focuser to turn on the simulator, the
aid has nothing to show and nothing to test.

**Independent Test**: `crates/finder/tests/console/finder_focus_measure.rs`, new. The
`test` gate's nextest command runs it; during development,
`cargo nextest run -p finder --locked --test console -E 'test(finder_focus_measure)'`.

**Acceptance Scenarios**:

1. **Given** drawn frames of six stars at the six places at phase 0, 0.25 and 0.5, with
   σ 1.2, 2.0 and 3.0, a drawn frame of six stars at the six places of σ 1.2, 1.6, 2.0,
   2.4, 2.8 and 3.2, and the σ 3.0 frame at phase 0 again with 2 added to each pixel
   whose column and row sum to an odd number, so that each star's box edge holds 64
   pixels of 10 and 64 of 12, **When** a committed test calls `measure` on each with its
   six centroids, **Then** it asserts that each answers `Sample::Reading` with `stars` 6;
   that each of the first frames has `px` within 8 % of 2·√(2 ln 2)·σ; that the mixed
   frame's `px` is within 3 % of the mean of 2·√(2 ln 2)·2.0 and 2·√(2 ln 2)·2.4; and that
   the last frame's `px` is within 1 % of the σ 3.0 frame's at phase 0.
2. **Given** a drawn frame of σ 2.0 with the six places, a star at (10.5, 240.5) whose
   box crosses the frame's edge, a star at (624.5, 240.5) whose box reaches column 640,
   two at (400.5, 400.5) and (410.5, 405.5) within 16 px of each other, and one at
   (300.5, 420.5) of density 250, whose top reaches 255, each with its centroid; the same
   frame with, for each of the six stars, a pixel of 200 ten pixels to its right and a
   pixel of 255 ten pixels below it; and the same frame with, for each of the six stars,
   a pixel of 200 eight pixels to its right and a pixel of 255 sixteen pixels below it;
   none of these pixels with a centroid, **When** a committed test calls `measure` on the
   three, **Then** it asserts that each answers `Sample::Reading` with `stars` 6, and that
   the second's `px` and the third's each equal the first's within 10⁻⁹.
3. **Given** a drawn frame of four stars of σ 2.0 at the first four of the six places, a
   frame of background 10 alone with no centroid, and a frame of background 10 with five
   uniform discs at the first five places, each pixel whose centre lies within 15 px of
   the place at level 60, **When** a committed test calls `measure` on each, **Then** it
   asserts that the first answers `Sample::TooFewStars` with `stars` 4, the second
   `Sample::TooFewStars` with `stars` 0, and the third `Sample::FarOut` with `stars` 5
   and `px` within 5 % of 15·√2.
4. **Given** a simulator as a simulated frame's definition gives it, **When** a
   committed test reads `finder_focus_error()`, renders with `render`, before any frame
   is taken, the 147 stars `field_stars(1000.0, 1.0)` gives at
   `crate::support::live::START`, at 1000 ms, gain 1, seed 13 and frame 0, and with
   `render_with_sigma` at `PSF_SIGMA_PX`, measures simulated frames at e = 0, 1, 2, 3,
   −3 and 8, and then calls `set_finder_focus_error(f64::NAN)`, **Then** it asserts that
   the error reads 0 before any is set and each e after it is set; that `render` gives
   1,584,128 bytes whose FNV-1a 64-bit hash is `0xb1d308b97a09eb40`, the hash of
   c75a6e76's own pixels computed at c75a6e76, and that `render_with_sigma` gives the
   same bytes; that at e = 0 to 3 each answers `Sample::Reading` with `stars` 15 and
   `px` within 10 % of H(e), each larger than the one before; that at e = −3 `px` is
   within 5 % of e = 3's; that at e = 2 a simulated frame at 200 ms reads within 4 % of
   e = 2's at 1000 ms; that at e = 8 it answers `Sample::TooFewStars` with fewer than 5
   `stars`; and that the last call panics.
5. **Given** `Optics` of 64 × 64 pixels, otherwise the default, and one `RenderStar` at
   (32.5, 32.5) of magnitude 8, **When** a committed test sums, for each of frames 0 to
   63 at 200 ms, gain 1 and seed 13, the pixels `render_with_sigma` draws with the star
   less those it draws with no star, at `PSF_SIGMA_PX` and at σ 3.0, **Then** it asserts
   that no pixel drawn with the star is at 250 or above, that the mean sum at
   `PSF_SIGMA_PX` is above 50, and that the mean sum at 3.0 is within 5 % of it.
6. **Given** drawn frames of six stars at the six places at phase 0, 0.25 and 0.5, with
   σ 0.5, 0.8 and 1.2, **When** a committed test calls `measure` on each with its six
   centroids, **Then** it asserts that each answers `Sample::Reading` with `stars` 6;
   that at each phase the reading at σ 0.8 is at least 1.1 times the one at σ 0.5, and
   the one at σ 1.2 at least 1.1 times the one at σ 0.8; and that the largest of the
   three phases' readings is at most 1.32 times the smallest at σ 0.5 and at most 1.1
   times at σ 0.8.

### User Story 2 - The finder camera's focus aid runs beside the live solve (Priority: P1)

As the owner, with a fixed exposure set on Setup, I start the finder camera's focus aid
and it reads one raw frame after another while the live solve goes on, and gives me the
reading, the stars detected, the trend, the best so far and a sign once I have passed the
best and come back to it.

**Why this priority**: It is the tool; the page and MCP show what it answers.

**Independent Test**: `crates/finder/tests/console/finder_focus_aid.rs`, new. The `test`
gate's nextest command runs it; during development,
`cargo nextest run -p finder --locked --test console -E 'test(finder_focus_aid)'`.

**Acceptance Scenarios**:

1. **Given** the focus rig, live, its session read, **When** a committed test sends
   `GET /api/finder-camera/focus`, then `POST /api/finder-camera/focus` with `{}`, and
   reads the settled reading after 0, **Then** it asserts that the first answers 200
   `{"focus": null}`; that the start answers 200 with `focus.state` `running`,
   `focus.readings` 0, `focus.found` false and `focus.history` `[]`; that the settled
   reading's `median.value` is within 10 % of H(0), its `last.stars` at least 5, its
   `last.detected` above its `last.stars` and its `history` as long as its
   `readings`; and that `GET /api/session` then reads the same `camera_settings` as
   before the start and a `pointing.seq` above the one read before.
2. **Given** the focus rig, live, with the focuser turned to 3 and the aid started with
   `{}` and its settled reading after 0 read, **When** a committed test turns the
   focuser to 0, reads until a settled reading has `trend` `better`, turns it to −3,
   reads until one has `trend` `worse`, and turns it to 0, reading until `found` is
   true, **Then** it asserts that the readings with `trend` `better` and `worse` each
   have `found` false; that the last has `found` true, `state` `running`, `best` within
   10 % of H(0) and `median.value` at most 1.10 times `best`.
3. **Given** the focus rig, live, the aid started with `{}` and its settled reading after
   0 read, **When** a committed test turns the focuser to 10 and reads until
   `last.state` is `too_few_stars`, then sets the simulator's optics to a 24° field, twice
   the default, with the focuser at 0 and reads the settled reading after n, n being that
   `too_few_stars` view's `readings`, then sends `DELETE /api/finder-camera/focus`, waits
   5 s and sends it again, **Then** it asserts that the `too_few_stars` view has
   `last.stars` under 5, `last.detected` under 5, no `last.value`, no `median`, a
   `history` ending in `null` and `state` `running`; that the settled reading after n has
   `state` `running`; that the first stop answers 200 with `focus.state` `ended` and
   `focus.ended` `stopped`; that the second stop answers 200 with the same `readings` and
   `ended` `stopped`; and that `GET /api/session` reads the `camera_settings` read before
   the start.
4. **Given** the bare console, a focus rig whose session was never given camera settings,
   live, and the focus rig, live, **When** a committed test sends
   `POST /api/finder-camera/focus` `{}` to the bare console and to the rig without camera
   settings, and to the focus rig a foreign write of `POST` and of `DELETE`,
   `GET /api/finder-camera/focus`, a `POST` of `{"x": 1}`, the body `{}` sent as
   `Content-Type: text/plain`, then `PUT /api/session`
   `{"camera_settings": {"exposure_ms": "auto"}}`, a `POST` of `{}`, `PUT /api/session`
   `{"camera_settings": {"exposure_ms": 500}}`, `{}`, `{}` again, then reads the settled
   reading after 0, stops the node's frames with `controls().stop_frames()`, reads the
   view 10 s later and reads until `focus.state` is `ended`, **Then** it asserts these
   answers in order: 409 `{"error": "the scope node is not connected"}`; 409
   `{"error": "the finder camera's exposure or gain is automatic: fix both on Setup"}`;
   403 `{"error": "writes require the console's own origin and an allowed host"}` for
   each foreign write; 200 `{"focus": null}`; 400; 415; 200; 409
   `{"error": "the finder camera's exposure or gain is automatic: fix both on Setup"}`;
   200; 200; 409 `{"error": "the finder camera's focus aid is already running"}`; that
   10 s after the frames stopped `focus.state` is still `running`; and that the run then
   ends with `focus.ended` `failed` and `focus.error` exactly
   `no raw frame arrived within 15 s`.
5. **Given** the focus rig, live, **When** a committed test starts the aid with `{}`,
   reads its settled reading after 0, sends `PUT /api/session`
   `{"camera_settings": {"exposure_ms": 250}}` and reads until `focus.state` is `ended`,
   then starts it again with `{}`, reads its settled reading after 0, sends
   `PUT /api/session` `{"camera_settings": {"gain": 2}}` and reads until `focus.state` is
   `ended`, **Then** it asserts that each run ended with `focus.ended` `failed`,
   `focus.error` exactly `the finder camera's exposure or gain changed during the run`
   and `readings` at least its settled reading's, and that `GET /api/session` then reads
   `camera_settings.exposure_ms` 250.

### User Story 3 - MCP and the README carry the finder camera's focus aid (Priority: P2)

As the owner, an MCP client can run the same aid for me, and the README and the glossary
say how, where to run the console and what to set first, and name it.

**Why this priority**: The routes and the page work without them; they are how an agent
and a reader find the aid.

**Independent Test**: `crates/finder/tests/mcp/finder_focus.rs`, new. The `test` gate's
nextest command runs it; during development,
`cargo nextest run -p finder --locked --test mcp -E 'test(finder_focus)'` and
`cargo nextest run -p finder --locked --test console -E 'test(docs)'`.

**Acceptance Scenarios**:

1. **Given** the bare console and the MCP rig, **When** a committed test calls, on the
   bare console, `set_finder_focus` with `{"on": true}`, and on the MCP rig
   `get_finder_focus` with `{}`, `set_finder_focus` with `{}`, with `{"x": 1}` and with
   `{"on": true, "x": 1}`, `get_finder_focus` with `{"x": 1}`, then, once live,
   `set_finder_focus` with `{"on": true}` twice, `get_finder_focus` until its
   `focus.median.readings` is 3, `set_finder_focus` with `{"on": false}`, then, after
   `PUT /api/session` `{"camera_settings": {"gain": "auto"}}`, `set_finder_focus` with
   `{"on": true}`, and lists the tools, **Then** it asserts that the bare console's call
   is a tool error with exactly `the scope node is not connected`; that the rig's first
   read answers `structuredContent` `{"focus": null}`; that the next four are tool errors
   with exactly `on must be true or false`, `on must be true or false`,
   `set_finder_focus takes on and nothing else` and `get_finder_focus takes no arguments`;
   that the first start's `focus.state` is `running` and the second is a tool error with
   exactly `the finder camera's focus aid is already running`; that a settled reading is
   read; that the stop's `focus.ended` is `stopped`; that the start after the `PUT` is a
   tool error with exactly
   `the finder camera's exposure or gain is automatic: fix both on Setup`; and that the
   list holds both tools, `get_finder_focus` with `readOnlyHint` true.
2. **Given** the finder README, **When** a committed test reads
   `crates/finder/README.md`, **Then** it asserts that it still holds
   `The imaging camera has sixteen tools of its own`; that the line
   `The Tools page's focus aid for the finder camera has two tools:` comes after
   `The Tools page's finder alignment has two tools:` and before `### MCP under WSL2`,
   with a list entry for each of `get_finder_focus` and `set_finder_focus` between them;
   that the heading `## Focusing the finder camera` comes after `## Finder alignment`
   and before `## Calibration frames`; and that between that heading and
   `## Calibration frames` it holds
   `Set a fixed exposure and gain for the finder camera on Setup first.`

### User Story 4 - The Tools page's Focus block picks the camera (Priority: P2)

As the owner, at the telescope with a phone, I open Tools, pick the finder camera or the
imaging camera, start its focus aid and watch the reading, the stars detected, the best
so far and the trend while I turn the focuser.

**Why this priority**: The routes and MCP work without the page; the page is how the
owner uses the aid at the telescope.

**Independent Test**: `crates/finder/web/src/components/FocusTool.test.tsx` and
`crates/finder/web/src/components/FocusToolPage.test.tsx`, new, under `npm test`, and
`crates/finder/web/e2e/tools-focus.spec.ts`, new, under `CI=1 npm run e2e`; during
development, `npm test -- FocusTool` and
`npx playwright test --project=simulate e2e/tools-focus.spec.ts` in `crates/finder/web`.

**Acceptance Scenarios**:

1. **Given** the page rig, **When** a committed test calls `finderStatus` for a running
   aid with no `last`, with a `reading` of 15 stars and three readings held, with a
   `reading` of 9 stars and two held, `settling`, `too_few_stars` with 2 stars of 3
   detected, `too_few_stars` with 2 stars of 31 detected, `far_out` of 21.2 px, and
   `too_few_stars` with 2 stars of 5 detected,
   **Then** it asserts these texts exactly:
   `Waiting for the first finder camera frame`,
   `Median of the last 3 finder camera frames · 15 stars`,
   `2 of 3 finder camera frames since the last change · 9 stars`,
   `Settling after a change`,
   `No reading: 2 stars measured, and 5 are needed. Far out of focus the stars vanish: turn the focuser slowly until they show.`,
   `No reading: 2 of the 31 stars detected could be measured, and 5 are needed: the rest are saturated, crowded or at the frame's edge.`,
   `Far out of focus: stars about 21.20 px across` and
   `No reading: 2 of the 5 stars detected could be measured, and 5 are needed: the rest are saturated, crowded or at the frame's edge.`
2. **Given** the page rig with `FocusTool` and no imaging view, `GET` answering
   `{"focus": null}`, **When** a committed test presses
   `Start the finder camera's focus aid`, then renders a running aid with `median` 2.83
   of 3 readings, `last` a reading of 15 stars of 31 detected, `best` 2.7, `trend`
   `better`, `found` true and `history` `[5.9, 6.1, null, 2.8, 2.83]`, presses
   `Stop the finder camera's focus aid`, and renders the block again over a running aid
   whose `history` holds 61 readings, **Then** it asserts that before the first read
   the region `Focus a camera` shows `Reading the finder camera's focus aid.`; that the
   start is `POST /api/finder-camera/focus` with the header
   `Content-Type: application/json` and the body `{}`; that the region shows `2.83 px`,
   the status `Median of the last 3 finder camera frames · 15 stars`,
   `Stars detected 31`, `Best so far 2.70 px`, `Sharper than before the last change`,
   `Passed through focus and came back to the best so far.` and the image
   `Focus readings of the last 5 finder camera frames` with 4 bars; that after the stop
   it shows `The finder camera's focus aid stopped.`; and that the block over 61 readings
   shows the image `Focus readings of the last 60 finder camera frames` with 60 bars.
3. **Given** the page rig with `FocusTool` over an aid ended `failed` with the error
   `no raw frame arrived within 15 s`, **When** a committed test renders it, **Then** it
   asserts that the alert reads exactly
   `The finder camera's focus aid stopped: no raw frame arrived within 15 s.`, that the
   button reads `Start the finder camera's focus aid`, and that no status shows.
4. **Given** the page rig with `FocusTool`, first with no imaging view and then with an
   imaging view whose `focus` has `unit` `px`, **When** a committed test picks
   `Imaging camera` in the select `Camera`, **Then** it asserts that with no view the
   region shows `The imaging camera's focus aid shows here once the imaging camera is ready.`;
   and that with the view the region holds 033's level-two heading `Focus`, its button
   `Start focus aid` and its words
   `A focus reading is the size of the stars in the focus region: smaller is sharper.`,
   and no button `Start the finder camera's focus aid`.
5. **Given** the page rig with `ToolsPage` and no imaging view, **When** a committed test
   renders it, **Then** it asserts that the region `Align the finder camera` comes
   before the region `Focus a camera` in the document, and that the select `Camera`
   reads `Finder camera`.
6. **Given** the simulated console in a 412 by 915 viewport, day mode, **When** a
   committed Playwright test sends `PUT /api/session`
   `{"camera_settings": {"exposure_ms": 200, "gain": 1}}`, opens `/tools`, presses
   `Start the finder camera's focus aid` in the region `Focus a camera` within 15 s,
   takes a full-page screenshot once a reading shows, turns night mode on with the
   switch `Night mode: red and black`, takes another, picks `Imaging camera` in that
   region's select `Camera`, picks `Finder camera` again, presses
   `Stop the finder camera's focus aid`, and in its `finally` sends
   `DELETE /api/finder-camera/focus`, **Then** it asserts that the `PUT` is answered
   200; that within 20 s the region's status contains
   `Median of the last 3 finder camera frames` and it shows a number of pixels with two
   decimals and the image of its readings; that the document's `scrollWidth` is at most
   412 by day and by night; that the day screenshot holds more than 10,000 pixels off
   night red and the night one none, counted as `crates/finder/web/e2e/tools.spec.ts`
   counts them (any green or blue, or red above 226); that the imaging camera's
   `Start focus aid` shows in the region within 15 s; and that
   `Start the finder camera's focus aid` shows again within 15 s of the stop.

### Edge Cases

- **No stars at all** (far out of focus, clouds, the cap on): every frame is
  `too_few_stars`, its history entry `null`, and the run goes on (US2-S3); with the node
  itself detecting fewer than five, the page says to turn the focuser slowly until stars
  show (US4-S1). Far out, the scope node's detection finds no stars before the size
  reaches 20 px (run on the simulator: at most one star at focus error 5 and beyond at
  200 ms to 1000 ms), so the aid helps once stars are in reach.
- **Stars detected but not measured** (saturated, crowded, at the frame's edge): the
  page says so, with the count (US4-S1); a lower fixed exposure gives more unsaturated
  stars, after which the aid is started again.
- **The exposure or gain automatic, or changed**: the start is refused (US2-S4), and a
  run ends `failed` when a measured star frame's exposure or gain differs from its
  first (US2-S5). A stored exposure outside the camera's range is sent to the node as
  automatic (`crates/finder/src/link.rs:407`): a run then ends at the automatic
  exposure's first step. Read, not run.
- **The sharpest stars taken for hot pixels**: the node's detection (cedar-detect's
  rule) can drop faint stars of under about 0.4 px σ at the sharpest focus
  (`reference-solutions/cedar-detect/src/algorithm.rs:89`); the count beside the
  reading falls there, and the README says to come back to the highest count within a
  tenth of the best. The rule's arithmetic was run on drawn stars (plan.md § What was
  run and what was read); its effect on the node's detection is read, not run.
- **A change of optics** (a spacer): the run goes on (US2-S3); the first readings after
  it are a change by 033's rule; the best so far is the old optic's until a stop and a
  start (owner decision 7). The README says so.
- **Track on, or a GoTo**: allowed. Track keeps the stars still. A GoTo smears them for
  a frame or two, which 033's rule sets aside as a change. Read, not run.
- **A recording's raw frames at the same time**: the node answers every waiting raw
  request from the same pixels (`crates/scope-node/src/service.rs:351`); each gets the
  frame. Read, not run.
- **A raw frame whose star frame does not come** within 15 s: it is skipped, measured
  nothing and counted nothing. Read, not run.
- **A scope node built before 025**, which answers `UNIMPLEMENTED`: the run ends
  `failed` with `this scope node cannot send raw frames`; any other error of the raw
  request, or a raw frame whose length is not its width times its height, with
  `the raw frame request failed`; the link dropped between frames with
  `the scope node is not connected`; a measure that panics with
  `the focus reading of a raw frame failed`. Read, not run.
- **Two pages open**: one run, shared; a second start is refused (US2-S4).
- **The imaging camera picked while the finder camera's aid runs**: that run goes on; the
  block shows 033's Focus block, and the finder camera's part again when picked back
  (US4-S6).
- **The console stops mid-run**: the run is in memory and is not resumed; no setting was
  changed, so none is left behind. Read, not run.

## Requirements *(mandatory)*

### Functional Requirements

**The measure and the simulator's focuser** (US1):

- **FR-001**: The simulator SHALL keep a finder camera focus error, 0 at start, with
  `Simulator::finder_focus_error()` and `Simulator::set_finder_focus_error(e)`, which
  panics on a value that is not finite. `SimCamera` SHALL draw each frame with stars of
  Gaussian width √(`render::PSF_SIGMA_PX`² + (`imaging::FOCUS_SIGMA_PX`·e)²) through a
  new `render::render_with_sigma`, which keeps each star's flux (its peak times
  (`PSF_SIGMA_PX`/σ)²) and draws it within a radius of 8·σ/`PSF_SIGMA_PX` rounded up;
  `render::render` SHALL call it at `PSF_SIGMA_PX` and draw exactly the pixels it drew
  at c75a6e76. Nothing else in `crates/simulator` SHALL change (owner approval 1).
- **FR-002**: `crates/finder` SHALL gain the module `finder_focus` with
  `pub fn measure(pixels: &[u8], width: u32, height: u32, centroids: &[Centroid]) -> Sample`,
  033's `Sample`, its centroids in cedar's coordinates (pixel column c covers c ≤ x <
  c + 1). Walking the centroids in order, it SHALL measure a star when its box, the
  square of side 2·`BOX_PX` + 1 (`BOX_PX` 16) centred on the pixel holding the centroid
  (column ⌊x⌋, row ⌊y⌋), lies inside the frame and no other centroid lies within
  `BOX_PX` of it in both x and y, and SHALL stop once `STARS_MEASURED` (15) are
  measured. A star's size SHALL be its half-flux diameter, computed so: the background
  is the median of the box's 128 edge pixels (of the two middle values, their mean), and
  the noise 1.4826 times their median absolute deviation, at least 0.5; the star's own
  pixels are those more than `THRESHOLD_NOISE` (3) noise levels above the background
  joined, by a side or a corner and through such pixels, to the start pixel, the
  brightest of the 5 × 5 pixels centred on the box's centre (the first in reading order,
  top row first and each row left to right, on a tie), and there are none when the start
  pixel is not above that level; a star none of whose pixels are its own, or one of
  whose own pixels is at `SATURATED` (250) or above, is not measured; offsets are taken
  from the centre of the box's centre pixel: the box pixel at column ⌊x⌋ + i and row
  ⌊y⌋ + j has its centre at (i, j) and its parts at (i − 0.5 + (a + 0.5)/5, j − 0.5 +
  (b + 0.5)/5), a and b from 0 to 4; the centre is the star's own pixels' centroid
  weighted by their values less the background, each pixel at its own centre; the circle
  round the centre has a radius `APERTURE_MARGIN_PX` (1) more than the distance from the
  centre to its farthest own pixel's centre, and at most `BOX_PX` − 1; every pixel of
  the box, less the background and with no threshold, is split into `SUBPIXELS` ×
  `SUBPIXELS` (5 × 5) equal squares, the parts, each carrying a 25th of that value at
  its own centre, and the squares whose centres lie within the circle (at a distance at
  most its radius) are kept; their total T, when not above 0, leaves the star
  unmeasured; distances within 10⁻⁹ px of the one before are one distance; for each
  distance d of a kept square from the centre, E(d) is the sum of the kept squares at d
  or nearer, and the half-flux radius is where the line through (0, 0) and the points
  (d, E(d)) in order of d first reaches T/2, read linearly between the two points around
  it; the size is twice that radius. The reading SHALL be the median of the sizes (of
  two middle values, their mean): fewer than 033's `MIN_STARS` stars SHALL answer
  `TooFewStars` with the count, a median above 033's `FAR_OUT_PX` `FarOut` with it and
  the count, and else `Reading` (owner decision 1; owner approval 2).
- **FR-003**: Nothing else SHALL change in US1: no route, view, page or test of another
  story or spec, and 033's module is used, not changed.

**The run** (US2):

- **FR-004**: `GET /api/finder-camera/focus` SHALL answer 200 `{"focus": ...}`, null
  before the first start in this process. `POST /api/finder-camera/focus` SHALL take
  the JSON body `{}` alone (any key 400, a body that is not JSON 415, through
  `sim_json` as 050's route takes its body), under the same-origin rule
  (`SessionWrite`, `crates/finder/src/api.rs:530`: a foreign write answers 403
  `writes require the console's own origin and an allowed host` before the body is
  read), and refuse, in this order: a run already running, 409
  `the finder camera's focus aid is already running`; a console with no scope link or
  no raw-frame client on it, 409 `the scope node is not connected`; and session camera
  settings (`GET /api/session`'s `camera_settings`) whose `exposure_ms` or `gain` is
  `auto`, or which are absent, the session's default being automatic, 409
  `the finder camera's exposure or gain is automatic: fix both on Setup`. Otherwise it
  SHALL start a run and answer 200 `{"focus": ...}`. `DELETE /api/finder-camera/focus`
  (no body, the same rule) SHALL stop a running run and answer 200 `{"focus": ...}`, the
  view unchanged when none runs (owner decisions 3 and 9; owner approvals 2 and 3).
- **FR-005**: A run SHALL ask the scope node for one raw frame after another (025's
  `RawFrame`, through `LinkManager::raw`), each within `RAW_WAIT`, 15 s; keep the star
  frames it sees meanwhile, the last eight, each with its id, centroids, `exposure_ms`
  and `gain`, through `LinkManager::frames`; and measure each raw frame at the centroids
  of the star frame of its id, waiting up to 15 s more for that star frame and skipping
  the raw frame when it does not come. Each measure SHALL run off the async runtime and
  its `Sample` SHALL go to one 033 `Aid` made at the start (owner decision 2).
- **FR-006**: `focus` SHALL carry `state` (`running` or `ended`), `since` (console Unix
  seconds), `readings` (033's samples pushed), `cadence_ms` (between the last two, once
  there are two), `last` (`frame_id`, `state`: 033's phase word, `value` for a reading
  or a far-out frame, `stars` measured, and `detected`, the centroid count of its star
  frame), `median` (`value`, `readings`), `best`, `trend`, `found` and `history`, each
  as 033's `Aid` answers it, every size in the finder camera's pixels; and, once ended,
  `ended` (`stopped` or `failed`) and, when failed, `error`.
- **FR-007**: A stop SHALL end the run at once, the raw request in flight dropped,
  `ended` `stopped`. A run SHALL end `failed` with `error`
  `the scope node is not connected` when the link has no raw-frame client or its star
  frames end; `this scope node cannot send raw frames` when the raw request answers
  `UNIMPLEMENTED`; `the raw frame request failed` for any other error, or a raw frame
  whose length is not its width times its height; `no raw frame arrived within 15 s`
  when none comes within `RAW_WAIT`;
  `the finder camera's exposure or gain changed during the run` when a star frame about
  to be measured has an `exposure_ms` or a `gain` other than the first measured star
  frame's, before that frame is measured; and `the focus reading of a raw frame failed`
  when a measure panics (owner decision 3).
- **FR-008**: A run SHALL change no setting of the finder camera or the session, take
  no lease, write no file, move nothing, and leave the live loop as it is; it SHALL read
  the session's camera settings only at its start, SHALL not read Track, the mount or
  the optics, and SHALL go on through a change of them (owner decisions 3 and 7).
- **FR-009**: Nothing else SHALL change in US2: 033's module, 025's recording, the link,
  the live loop and every existing test, US1's among them.

**MCP and the words** (US3):

- **FR-010**: MCP SHALL gain `get_finder_focus`, read-only, which takes no arguments
  (else the tool error `get_finder_focus takes no arguments`) and answers
  `{"focus": ...}`; and `set_finder_focus`, which takes `on`, a boolean (else
  `on must be true or false`), and nothing else
  (`set_finder_focus takes on and nothing else`), and starts or stops the aid as
  FR-004 does, the session's camera settings among its refusals, answering
  `{"focus": ...}`, a refusal's words being its tool error (owner decision 6; owner
  approval 3).
- **FR-011**: The finder README SHALL gain the three route rows after
  `DELETE /api/imaging/align`'s, the MCP paragraph after the finder alignment's, and
  the section `## Focusing the finder camera` before `## Calibration frames`, holding
  the fixed exposure first, the console beside the scope node, and the hot-pixel limit
  and its aim, as plan.md § Interfaces gives them; `The imaging camera has sixteen tools
  of its own` SHALL stay as it is. `GLOSSARY.md`'s **Focus reading**, **Focus aid** and
  **Raw frame** SHALL name the finder camera's aid as plan.md § Interfaces gives them
  (owner decisions 2, 3 and 5; owner approvals 4 and 5).
- **FR-012**: Nothing else SHALL change in US3: the routes, the run and the view of US2,
  033's tools, and every existing test, the earlier stories' among them.

**The page** (US4):

- **FR-013**: `crates/finder/web/src/api.ts` SHALL gain `FinderFocus` (its `last` with
  `detected`), `getFinderFocus`, `postFinderFocus` (body `{}`,
  `Content-Type: application/json`) and `deleteFinderFocus`.
- **FR-014**: `components/FocusTool.tsx` SHALL be a region named by its heading
  `Focus a camera`, with the select `Camera` (`Finder camera`, the default, and
  `Imaging camera`). With the finder camera picked it SHALL read
  `GET /api/finder-camera/focus` once a second, show `Reading the finder camera's focus aid.`
  until the first read, then one line on what it does, the button
  `Start the finder camera's focus aid`, or `Stop the finder camera's focus aid` while a
  run runs, and, once there is a run: `Focus reading`, the median as the large number
  with two decimals and ` px`, or `—` without one; while it runs, the status
  `finderStatus` gives (US4-S1's words, the `too_few_stars` hint chosen by whether
  `last.detected` is under 5); after a stop, the status
  `The finder camera's focus aid stopped.`; `Stars detected <detected>` once there is a
  `last`; `Best so far <best> px`; the trend's words (`Sharper than before the last change`,
  `Softer than before the last change`, `About the same as before the last change`);
  `Passed through focus and came back to the best so far.` while `found`; the last 60
  readings as bars, an image named `Focus readings of the last N finder camera frames`;
  for a run ended `failed` the alert `The finder camera's focus aid stopped: <error>.`
  and no status; and a refused start or stop's words as an alert. It SHALL use
  DESIGN.md's existing classes and `currentColor`, so night mode stays red only (owner
  decision 5; owner approval 7).
- **FR-015**: With the imaging camera picked, the block SHALL render 033's
  `FocusBlock` unchanged, with the Tools page's imaging view and refresh, while the view
  has `focus`, and else `The imaging camera's focus aid shows here once the imaging camera is ready.`
  (owner decision 4).
- **FR-016**: `ToolsPage` SHALL render `<FocusTool view={view} refresh={refresh} />`
  after the alignment's block. `crates/finder/web/e2e/tools-focus.spec.ts` SHALL hold
  US4-S6, with `"**/tools-focus.spec.ts"` added to the default project's `testIgnore`
  and the simulate project's `testMatch` (owner decision 8; owner approval 6);
  `crates/finder/web/dist` SHALL be rebuilt and committed.
- **FR-017**: Nothing else on the page SHALL change: `FocusBlock.tsx`,
  `AlignmentBlock.tsx`, the Imaging page and Setup are untouched.

### Key Entities

- **The finder camera's focus aid**: raw frames of the finder camera read one after
  another beside the live solve at one fixed exposure and gain, each measured at its
  star frame's centroids and judged by 033's arithmetic, while the observer turns the
  focuser.
- **The finder camera's focus reading**: the median half-flux diameter, in the finder
  camera's pixels, of at most 15 isolated, unsaturated stars of one raw frame, each
  summed in sub-pixel circles.
- **The Focus block**: the Tools page's block that picks the camera and shows that
  camera's focus aid.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario has a committed test that asserts it, and the
  `lint` and `test` gates pass.
- **SC-002**: No existing test, golden, fixture, `scripts/ci/` file, `.github/` file,
  `CODEOWNERS` entry or `ergane.yaml` is edited, and `playwright.config.ts` gains only
  its two list entries. Every existing test passes unchanged (plan.md § Shapes that
  existing tests pin). The only edits to existing test files are the `mod` lines of
  `crates/finder/tests/console/main.rs` and `crates/finder/tests/mcp/main.rs`.
- **SC-003**: The reading follows a drawn star's size within 8 % from σ 1.2 px at every
  phase and rises with it from σ 0.5 px (US1-S1, US1-S6), follows the simulator's star
  size within 10 % at focus errors 0 to 3 (US1-S4, US2-S1), the simulator's stars keep
  their flux as they widen (US1-S5), and a pass through focus and back is reported as
  `found` with the best within 10 % of the truth (US2-S2).
- **SC-004**: The live solve goes on and the session's camera settings are unchanged
  while the aid runs (US2-S1, US2-S3), every reading of a run is at one exposure and
  gain (US2-S4, US2-S5), and the imaging camera's focus aid is reached on Tools through
  033's own block (US4-S4, US4-S6).

## Assumptions

- **Earlier features are in place.** Every spec in `depends_on_landed` has landed. Every
  citation here was read at c75a6e76.
- **The simulator's finder camera stands for the real one** in the gates: Gaussian stars
  whose width grows with the focus error, flux kept, so that, as on a real camera, stars
  fade out of the detection as they spread. Its stars are no smaller than σ 1.2 px, so
  the pixel-sized stars of this optic are the drawn frames' (US1-S6) and the model's.
- **The scope node's centroids are the stars**: the aid measures where the node's
  detection, 048's on a dark sky, found stars, as the live solve uses them.
- **Out of scope:**
  - the scope node, the scope link's protocol and the node's detection;
  - 033's measure, arithmetic, record, verdict and rules, and its Imaging page block;
  - a motor focuser, the mount and an equatorial mount's needs;
  - a record or verdict for the finder camera;
  - the simulator's noise, which repeats every second frame (plan.md § Risks): a
    follow-up of its own;
  - the first focus of the real finder camera at night, which is the owner's (plan.md
    § Outside the factory).
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
  implements: [FR-004, FR-005, FR-006, FR-007, FR-008, FR-009]
US3:
  depends_on: []
  depends_on_merged: [US2]
  implements: [FR-010, FR-011, FR-012]
US4:
  depends_on: []
  depends_on_merged: [US3]
  implements: [FR-013, FR-014, FR-015, FR-016, FR-017]
```

Four stories, each under 60 % of 65,536 B (39,321 B) as probe round 1's commits and its
fold's measure them, the largest US2 at 53.7 %; at the factory's worst overrun seen,
1.35 times, US1 and US2 would go over that line, still under 64 KiB (plan.md § Plan
traps 2): the measure and the simulator's focuser; the run and its routes; MCP, the
README and the glossary; the page. Each merges after the one before
(`depends_on_merged`): US2's run calls US1's `measure` and its tests turn US1's focuser;
US3's tools start and stop US2's run; the page reads US2's view, and its browser test
needs US2's routes. Plain `depends_on` would order them without putting the earlier code
in the later story's base.
