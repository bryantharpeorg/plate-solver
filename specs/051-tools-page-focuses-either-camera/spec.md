---
state: draft
depends_on_landed: ["013-live-finder", "016-mcp", "025-frame-capture", "033-focus-reading-and-aid", "047-console-uses-the-database-that-fits", "048-node-detects-stars-on-a-dark-sky", "050-tools-page-aligns-finder-camera"]
# Drafted 2026-11-26 against `main` at c75a6e76, at the owner's ask of 2026-11-25
# ("we should probably just generally have some utility pages in the system. another
# one for focusing an imaging node (finder or the imager itself)"), from a probe of that
# day on the branch `probe/051` cut from c75a6e76 (tip 9a60d558; plan.md § What was run
# and what was read). Every `path:line` here was read at c75a6e76. State: draft. The
# nine owner decisions are open, each with a recommendation the scenarios are written
# to; the flip is the owner's. 025 is named because the aid reads its raw frames; 033
# because it judges every reading with 033's arithmetic and reuses 033's Focus block;
# 048 because the aid measures at the stars the scope node's detection finds; 013 for
# the simulator's finder camera and the live loop; 016 for MCP; 047 and 050 because each
# last changed a file this spec writes (plan.md § Refinement Record).
---

# Feature Specification: The Tools Page Focuses Either Camera

**Feature Branch**: `051-tools-page-focuses-either-camera`

**Created**: 2026-11-26

**Status**: Draft

**Input**: The owner focuses the finder camera, a ZWO ASI120MM Mini (1280 × 960 at
3.75 µm, read as 8-bit by the scope node, 045) behind the SVBONY SV165's own 40 mm f/4
objective, by hand with the guide scope's helical focuser, and will refocus it after each
spacer the owner tries, and after a focal reducer if one is added, which changes the field
and the scale. Today nothing tells the owner how sharp the finder camera's stars are, beyond solving or not.
The imaging camera already has a focus aid (033) on the Imaging page. Give the Tools
page (050) a Focus block where the owner picks the camera, turns the focuser and watches one
number, its trend and the best so far get better, with a plain line once they have passed
the best and come back to it, for either camera, without building 033's aid twice.

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
  `crates/scope-link/proto/scope_node/v1/scope_node.proto:72`): the scope node's
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
  star of 2 to 3 pixels.
- Q: Can a test defocus the simulator's finder camera? → A: Not at c75a6e76. Its stars
  are drawn with a fixed Gaussian width, `PSF_SIGMA_PX`, 1.2 px
  (`crates/simulator/src/render.rs:10`), in a stamp of radius 8
  (`crates/simulator/src/render.rs:74`); 033's focus error is the imaging optics'
  (`crates/simulator/src/optics.rs:299`). `render::render` is called by an existing test
  outside the simulator, so its signature stays. The smallest change (owner approval 1)
  is a finder camera focus error kept beside the pointing
  (`crates/simulator/src/optics.rs:74`) and a `render_with_sigma` that `render` calls at
  1.2 px, drawing exactly the pixels it drew before (run).
- Q: Does the reading need the live solve stopped, or the finder camera's settings held?
  → A: No (owner decision 3). The probe's aid read raw frames while the live solve went
  on (the session's pointing sequence kept rising) and left `camera_settings` as they
  were (run). The node's automatic exposure, when no exposure is set, keeps 25 to 40
  stars (`crates/scope-node/src/exposure.rs:26`).
- Q: How does a focus run end? → A: By a stop, or `failed` when the scope node is not
  connected, cannot send raw frames, or sends none for 15 s: 025's limit and words
  (`crates/finder/src/recording.rs:33`, `crates/finder/src/recording.rs:369`,
  `crates/finder/src/recording.rs:371`, `crates/finder/src/recording.rs:377`). The
  silence is run; the other two are read (plan.md § What was run and what was read).
- Q: Can the aid be used by day? → A: No: it measures stars. A rough focus by day on a
  distant object is the owner's, outside this tool (plan.md § Outside the factory).

## Owner decisions

All nine are open. Each has its options, its evidence and a recommendation, (a); the
scenarios and the record are written to the recommendations. One general rule is behind
decisions 1 to 3 and 7: **the finder camera's aid adds no rule of its own to 033's
arithmetic and takes nothing from the live solve.** It reads only what the scope link
already carries, the raw frames of 025 and the centroids of each star frame, measures
each raw frame and hands the result to 033's `Aid` as a `Sample`, with 033's own five
stars and 20 pixels. So it holds no lease, remembers and puts back no setting, needs no
rule for Track, a GoTo or a change of optics, and keeps no record and gives no verdict;
033's module and its tests are not touched. Its own parts are the measure, the run that
feeds it and its words.

1. **The finder camera's measure** (FR-002). Recommended (a): **the console's own
   half-flux diameter, measured on each raw frame at the centroids of that frame's star
   frame**: the median over at most 15 of the brightest stars whose box of 33 × 33
   pixels lies inside the frame, holds no other centroid and no pixel at 250 or above,
   of the diameter that holds half the flux above the box edge's median, counting only
   the pixels more than three noise levels above it that are joined to the star's
   brightest pixel. It follows the half-flux recipe the repository's survey sets out
   (`docs/research/2026-11-15-imaging-camera.md:210`). Measured in the probe: on the
   simulator's finder camera at 1000 ms, 0.94, 0.97, 0.97 and 0.96 times the Gaussian's
   own half-flux diameter at focus errors 0, 1, 2 and 3, rising with each, with 15
   stars; at 200 ms 0.94 to 0.77, still rising; on the owner's eight 3 s frames of
   2026-11-09 from an earlier finder camera, an OV5647 behind a 16 mm lens (outside the
   repository), 2.78 to 3.10 px with 15 stars each, where the owner's own script's focus log of
   that night printed half-flux radii of 1.21 to 1.56 px, diameters of 2.42 to 3.12.
   Instead: (b) **033's measure**, ASTAP's `-analyse` on the raw frame saved as a
   picture: on those eight frames it found 3 to 7 stars, under five in five of them, at
   3.4 to 4.5 px; it needs an imaging solver on the console, and the gates' simulated
   one reads the imaging camera's truth, not the frame; (c) **a measure from the star
   frame alone**, no raw frame: on the simulator at 200 ms the star count stayed at 36
   from focus error 0 to 2 and the brightest centroid's brightness rose and fell, so
   neither moves near focus; (d) **a size measured by the scope node** and sent on each
   star frame: the cheapest on the link, but a change to the proto and the node, out of
   this spec's scope.
2. **Where it is computed, and its cost** (FR-005). Recommended (a): **on the console,
   one raw frame per reading**: `width × height` bytes on the scope link,
   1,228,800 B for the ASI120MM's 1280 × 960, and nothing more on the node, which
   already holds each frame's pixels and answers a raw request from them. The measure
   took under 1 % of the simulator's 500 ms frame interval in the probe's test build.
   The cadence is one reading per raw frame the link delivers; the star frames share the
   link meanwhile. Instead: (b) **on the scope node**, decision 1's (d); (c) **from the
   preview JPEG**, at most one every 5 s and box-averaged to 640 pixels.
3. **Beside the live solve, or holding the finder camera** (FR-008). Recommended (a):
   **beside it, changing no setting**: the live solve goes on with the same frames and
   Track may stay on, which keeps the stars still. With no exposure set, the node's
   automatic exposure keeps 25 to 40 stars. Evidence: the probe's run left
   `camera_settings` unchanged while the pointing sequence rose; replayed through 033's
   arithmetic, the owner's 2026-11-09 3 s focus log settled 95 of 104 readings and set
   aside 4, while the owner's 1 s logs of 2026-11-04 and earlier on 2026-11-09, of 8 stars a
   frame, settled 21 of 224 and 15 of 214, most set aside as changes; so the README says
   to fix a longer exposure on Setup when readings rarely settle. Instead: (b) **hold a fixed
   exposure for the run**, remembered and put back, a lease-like rule of its own over
   the session's settings; (c) **stop the live solve while focusing**, losing the
   pointing and Track for nothing measured.
4. **The imaging camera from Tools** (FR-015). Recommended (a): **033's Focus block
   itself**, `FocusBlock` rendered unchanged on Tools when the imaging camera is picked,
   over the Tools page's own imaging view; it stays on the Imaging page too. Built in the
   probe with `FocusBlock.tsx` untouched (run, Vitest and the browser). Instead: (b) **a
   link** to the Imaging page's Focus block, a second place to go; (c) **moving** the
   block to Tools, which changes 033's FR-031 and its tests.
5. **The display** (FR-014). Recommended (a): **033's lines for the finder camera, with
   no record or verdict**: the median as the large number in the finder camera's pixels
   to two decimals, a status line, `Best so far`, the trend since the last turn, the line
   once the reading has passed through focus and come back, and the last 60 readings as
   bars. Instead: (b) **arcseconds** from the live solve's scale, which a reducer halves;
   pixels compare exactly for one camera behind one optic; (c) **033's record and
   verdict for the finder camera**, a second `focus.json` and its rules.
6. **MCP** (FR-010). Recommended (a): **two tools**, `get_finder_focus` (read-only) and
   `set_finder_focus` (`on`), as 033 and 050 have theirs. Instead: (b) **none**; (c) **a
   `camera` argument on 033's tools**, a change to 033's FR-035 and FR-036.
7. **A change of optics mid-run** (FR-008). Recommended (a): **nothing of its own**:
   the first readings after a spacer or a reducer are a change by 033's 15 % rule and
   the run goes on; the best so far stays the old optic's until the observer stops and
   starts again, which the README says to do. A reducer also moves the focal plane, so
   the frames after it are out of focus and the live solve fails until refocused (read,
   not run): a field change would be seen only near focus. Instead: (b) **end the run**
   when the live solve's field width moves more than 5 % from the run's first; (c)
   **start the arithmetic again** on such a move.
8. **A browser test** (FR-016). Recommended (a): **`e2e/tools-focus.spec.ts` in the
   simulate project**, at 412 by 915, by day and by night, its locators held to the
   block's region, with two list entries in `playwright.config.ts` (owner approval 6):
   the block draws bars, and only a browser paints its night colours. Run in the probe,
   34 of 34 with it. Instead: (b) **none**, Vitest alone.
9. **The route's name** (FR-004). Recommended (a): **`/api/finder-camera/focus`**.
   Instead: (b) `/api/finder/focus`, where "finder" alone is the glossary's _Avoid_ for
   the camera; (c) `/api/focus/finder`.

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
   `THRESHOLD_NOISE`, `RAW_WAIT`, `measure`, `FocusAsk`, `FocusView`, `LastView`,
   `MedianView`, `Refusal` and `FinderFocus` (`view`, `start`, `stop`); and
   `AppState::finder_focus`. No existing signature changes.
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
- **H(e)**, the oracle of a star's size, is a Gaussian's own half-flux diameter,
  2·√(2 ln 2)·σ, at σ = √(1.2² + e²) px, the simulator's finder camera star width at
  focus error e: H(0) = 2.826 px and H(3) = 7.609 px.
- **A drawn frame** is a 640 × 480 frame of the test's own making: background 10, plus,
  for each star at (x, y), peak·exp(−r²/2σ²) at each pixel, r measured from the pixel's
  centre (column + 0.5, row + 0.5), rounded and held to 0 to 255; each star's centroid
  is (x, y) with its peak as brightness. **The six places** are (100.5, 100.5),
  (300.5, 100.5), (500.5, 100.5), (100.5, 300.5), (300.5, 300.5) and (500.5, 300.5).
- **A simulated frame at e** is the next frame of a `SimCamera` over a simulator on
  `crate::support::live::DB` with `Site::default()` and a `VirtualClock` from
  `crate::support::live::START`, pushed to altitude 50° and azimuth 120°, with
  `set_finder_focus_error(e)`, an exposure of 1000 ms and gain 1, measured at the
  centroids `scope_node::detect::detect` finds in it with `DetectSettings::cedar()`, the
  scope node's own detection.
- **The focus rig** is that simulator served by a `SimNode` with the token `test-token`
  and a 500 ms frame interval, reached over an in-memory duplex as
  `crates/finder/tests/console/imaging_align.rs` reaches its node; a `Session` paired
  with it; and `AppState::shared` over that database with `with_session`, `with_clock`
  of the simulator's clock, `with_link` and `with_live()`, served by `finder::router`.
  **Live** means `GET /api/session` reads `pointing.source` `live`. **The settled
  reading after n** is `focus` of `GET /api/finder-camera/focus` once its `readings` is
  at least n + 3, its `median.readings` 3 and its `last.state` `reading`, read every
  100 ms. **Turning the focuser to e** is `set_finder_focus_error(e)` mid-run. The tests
  run under `#[tokio::test(start_paused = true)]`, and every state awaited comes within
  120 s of paused time.
- **The MCP rig** is the focus rig served with `/mcp`, the tools called with
  `tools/call` as `crates/finder/tests/mcp/` calls them; **the bare console** is
  `AppState::shared` alone, with no link.
- **The page rig** is the component rendered by Testing Library with `fetch` stubbed to
  answer the views the test writes, as the page's existing tests do.
- **The simulated console** is the simulate project's server of
  `crates/finder/web/playwright.config.ts`, the release `finder` with `--simulate`.
- **Oracles.** H(e) and a uniform disc's half-flux diameter, √2 times its radius; the
  drawn frames' own places; 025's and 033's words; and the words of these scenarios. No
  expected value comes from the code under test: no test calls `measure` for an expected
  value or reads `STARS_MEASURED`, `BOX_PX`, `SATURATED`, `THRESHOLD_NOISE` or
  `RAW_WAIT`.

### User Story 1 - The console reads the finder camera's focus, and the simulator turns it (Priority: P1)

As the owner, I need the console to measure how large the finder camera's stars are in
one of its raw frames, and the simulator's finder camera to go out of focus so that this
can be tested.

**Why this priority**: Without the measure and a focuser to turn on the simulator, the
aid has nothing to show and nothing to test.

**Independent Test**: `crates/finder/tests/console/finder_focus_measure.rs`, new. The
`test` gate's nextest command runs it; during development,
`cargo nextest run -p finder --locked --test console -E 'test(finder_focus_measure)'`.

**Acceptance Scenarios**:

1. **Given** drawn frames of six stars at the six places, peak 200, with σ 1.2, 2.0 and
   3.0, **When** a committed test calls `measure` on each with its six centroids,
   **Then** it asserts that each answers `Sample::Reading` with `stars` 6 and `px`
   within 8 % of 2·√(2 ln 2)·σ.
2. **Given** a drawn frame of σ 2.0 and peak 200 with the six places, a star at
   (10.5, 240.5) whose box crosses the frame's edge, two at (400.5, 400.5) and
   (410.5, 405.5) within 16 px of each other, and one at (300.5, 420.5) of peak 245,
   whose top reaches 255, each with its centroid, and the same frame with a pixel of 200
   ten pixels right of each of the six stars, with no centroid, **When** a committed test
   calls `measure` on both, **Then** it asserts that each answers `Sample::Reading` with
   `stars` 6, and that the second's `px` equals the first's within 10⁻⁹.
3. **Given** a drawn frame of four stars at the first four of the six places, a frame of
   background 10 alone with no centroid, and a frame of background 10 with five uniform
   discs of level 60 and radius 15 px at the first five places, **When** a committed
   test calls `measure` on each, **Then** it asserts that the first answers
   `Sample::TooFewStars` with `stars` 4, the second `Sample::TooFewStars` with `stars` 0,
   and the third `Sample::FarOut` with `stars` 5 and `px` within 5 % of 15·√2.
4. **Given** a simulator as a simulated frame's definition gives it, **When** a committed
   test reads `finder_focus_error()`, compares `render_with_sigma` at `PSF_SIGMA_PX` with
   `render` for that simulator's stars at 1000 ms, gain 1, seed 13 and frame 0, and
   measures simulated frames at e = 0, 1, 2, 3, −3 and 8, **Then** it asserts that the
   error reads 0 before any is set and each e after it is set, that the two renders are
   equal pixel for pixel, that at e = 0 to 3 each answers `Sample::Reading` with
   `stars` 15 and `px` within 10 % of H(e), each larger than the one before, that at
   e = −3 `px` is within 5 % of e = 3's, and that at e = 8 it answers
   `Sample::TooFewStars` with fewer than 5 `stars`.
5. **Given** `Optics` of 64 × 64 pixels, otherwise the default, and one `RenderStar` at
   (32.5, 32.5) of magnitude 8, **When** a committed test sums, for each of frames 0 to
   63 at 200 ms, gain 1 and seed 13, the pixels `render_with_sigma` draws with the star
   less those it draws with no star, at `PSF_SIGMA_PX` and at σ 3.0, **Then** it asserts
   that no pixel drawn with the star is at 250 or above, that the mean sum at
   `PSF_SIGMA_PX` is above 50, and that the mean sum at 3.0 is within 5 % of it.

### User Story 2 - The finder camera's focus aid runs beside the live solve (Priority: P1)

As the owner, I start the finder camera's focus aid and it reads one raw frame after
another while the live solve goes on, and gives me the reading, the trend, the best so
far and a sign once I have passed the best and come back to it.

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
   reading's `median.value` is within 10 % of H(0), its `last.stars` at least 5 and its
   `history` as long as its `readings`; and that `GET /api/session` then reads the same
   `camera_settings` as before the start and a `pointing.seq` above the one read before.
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
   the default (a reducer), with the focuser at 0 and reads the settled reading after
   that count, then
   sends `DELETE /api/finder-camera/focus`, waits 5 s and sends it again, **Then** it
   asserts that the first has `last.stars` under 5, no `last.value`, no `median`, a
   `history` ending in `null` and `state` `running`; that the second has `state`
   `running`; that the first stop answers 200 with `focus.state` `ended` and
   `focus.ended` `stopped`; that the second stop answers 200 with the same `readings` and
   `ended` `stopped`; and that `GET /api/session` reads the `camera_settings` read before
   the start.
4. **Given** the bare console and the focus rig, live, **When** a committed test sends
   `POST /api/finder-camera/focus` `{}` to the bare console, and to the rig `{"x": 1}`,
   the body `{}` sent as `Content-Type: text/plain`, `{}`, `{}` again, and then stops
   the node's frames with `controls().stop_frames()`, reading until `focus.state` is
   `ended`, **Then** it asserts these answers in order: 409
   `{"error": "the scope node is not connected"}`; 400; 415; 200; 409
   `{"error": "the finder camera's focus aid is already running"}`; and that the run
   ends with `focus.ended` `failed` and `focus.error` exactly
   `no raw frame arrived within 15 s`.

### User Story 3 - MCP and the README carry the finder camera's focus aid (Priority: P2)

As the owner, an MCP client can run the same aid for me, and the README and the glossary
say how and name it.

**Why this priority**: The routes and the page work without them; they are how an agent
and a reader find the aid.

**Independent Test**: `crates/finder/tests/mcp/finder_focus.rs`, new. The `test` gate's
nextest command runs it; during development,
`cargo nextest run -p finder --locked --test mcp -E 'test(finder_focus)'` and
`cargo nextest run -p finder --locked --test console -E 'test(docs)'`.

**Acceptance Scenarios**:

1. **Given** the bare console and the MCP rig, **When** a committed test calls, on the
   bare console, `set_finder_focus` with `{"on": true}`, and on the MCP rig
   `get_finder_focus` with `{}`, `set_finder_focus` with `{}` and with
   `{"on": true, "x": 1}`, `get_finder_focus` with `{"x": 1}`, then, once live,
   `set_finder_focus` with `{"on": true}` twice, `get_finder_focus` until its
   `focus.median.readings` is 3, `set_finder_focus` with `{"on": false}`, and lists the
   tools, **Then** it asserts that the bare console's call is a tool error with exactly
   `the scope node is not connected`; that the rig's first read answers
   `structuredContent` `{"focus": null}`; that the next three are tool errors with
   exactly `on must be true or false`, `set_finder_focus takes on and nothing else` and
   `get_finder_focus takes no arguments`; that the first start's `focus.state` is
   `running` and the second is a tool error with exactly
   `the finder camera's focus aid is already running`; that a settled reading is read;
   that the stop's `focus.ended` is `stopped`; and that the list holds both tools,
   `get_finder_focus` with `readOnlyHint` true.
2. **Given** the finder README, **When** a committed test reads
   `crates/finder/README.md`, **Then** it asserts that it still holds
   `The imaging camera has sixteen tools of its own`; that the line
   `The Tools page's focus aid for the finder camera has two tools:` comes after
   `The Tools page's finder alignment has two tools:` and before `### MCP under WSL2`,
   with a list entry for each of `get_finder_focus` and `set_finder_focus` between them;
   and that the heading `## Focusing the finder camera` comes after
   `## Finder alignment` and before `## Calibration frames`.

### User Story 4 - The Tools page's Focus block picks the camera (Priority: P2)

As the owner, at the telescope with a phone, I open Tools, pick the finder camera or the
imaging camera, start its focus aid and watch the reading, the best so far and the trend
while I turn the focuser.

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
   `reading` of 9 stars and two held, `settling`, `too_few_stars` with 2 stars, and
   `far_out` of 21.2 px, **Then** it asserts these texts exactly:
   `Reading the first raw frame of the finder camera.`,
   `Median of the last 3 frames · 15 stars`,
   `2 of 3 frames since the last change · 9 stars`, `Settling after a change`,
   `No reading: 2 stars measured, and 5 are needed. Far out of focus the stars vanish: turn the focuser slowly until they show.`
   and `Far out of focus: stars about 21.20 px across`.
2. **Given** the page rig with `FocusTool` and no imaging view, `GET` answering
   `{"focus": null}`, **When** a committed test presses
   `Start the finder camera's focus aid`, then renders a running aid with `median` 2.83
   of 3 readings, `last` a reading of 15 stars, `best` 2.7, `trend` `better`, `found`
   true and `history` `[5.9, 6.1, null, 2.8, 2.83]`, and presses
   `Stop the finder camera's focus aid`, **Then** it asserts that before the first read
   the region `Focus a camera` shows `Reading the finder camera's focus aid.`; that the
   start is `POST /api/finder-camera/focus` with the header
   `Content-Type: application/json` and the body `{}`; that the region shows `2.83 px`,
   the status `Median of the last 3 frames · 15 stars`, `Best so far 2.70 px`,
   `Sharper than before the last turn`,
   `Passed through focus and came back to the best so far.` and the image
   `Focus readings of the last 5 frames of the finder camera` with 4 bars; and that after
   the stop it shows `The finder camera's focus aid stopped.`
3. **Given** the page rig with `FocusTool` over an aid ended `failed` with the error
   `no raw frame arrived within 15 s`, **When** a committed test renders it, **Then** it
   asserts that the alert reads exactly
   `The finder camera's focus aid stopped: no raw frame arrived within 15 s.` and that
   the button reads `Start the finder camera's focus aid`.
4. **Given** the page rig with `FocusTool`, first with no imaging view and then with an
   imaging view whose `focus` has `unit` `px`, **When** a committed test picks
   `Imaging camera` in the select `Camera`, **Then** it asserts that with no view the
   region shows `The imaging camera's focus aid shows here once the imaging camera is ready.`;
   and that with the view the region holds 033's level-two heading `Focus` and its
   button `Start focus aid`, and no button `Start the finder camera's focus aid`.
5. **Given** the page rig with `ToolsPage` and no imaging view, **When** a committed test
   renders it, **Then** it asserts that the region `Align the finder camera` comes
   before the region `Focus a camera` in the document, and that the select `Camera`
   reads `Finder camera`.
6. **Given** the simulated console in a 412 by 915 viewport, day mode, **When** a
   committed Playwright test opens `/tools`, presses
   `Start the finder camera's focus aid` in the region `Focus a camera`, takes a
   full-page screenshot once a reading shows, turns night mode on with the switch
   `Night mode: red and black`, takes another, picks `Imaging camera` in that region's
   select `Camera`, picks `Finder camera` again and presses
   `Stop the finder camera's focus aid`, **Then** it asserts that within 20 s the
   region's status contains `Median of the last 3 frames` and it shows a number of pixels with two
   decimals and the image of its readings; that the document's `scrollWidth` is at most
   412 by day and by night; that the day screenshot holds more than 10,000 pixels off
   night red and the night one none, counted as `crates/finder/web/e2e/tools.spec.ts`
   counts them (any green or blue, or red above 226); that the imaging camera's
   `Start focus aid` shows in the region within 15 s; and that
   `Start the finder camera's focus aid` shows again within 15 s of the stop.

### Edge Cases

- **No stars at all** (far out of focus, clouds, the cap on): every frame is
  `too_few_stars`, its history entry `null`, and the run goes on (US2-S3); the page
  says to turn the focuser slowly until stars show (US4-S1). Far out, the scope node's
  detection finds no stars before the size reaches 20 px (run on the simulator: none at
  focus error 6 and beyond), so the aid helps once stars are in reach.
- **A change of optics** (a spacer, a reducer): the run goes on (US2-S3); the first
  readings after it are a change by 033's rule; the best so far is the old optic's until
  a stop and a start (owner decision 7). The README says so.
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
  `the scope node is not connected`. Read, not run.
- **Two pages open**: one run, shared; a second start is refused (US2-S4).
- **The imaging camera picked while the finder camera's aid runs**: that run goes on; the
  block shows 033's Focus block, and the finder camera's part again when picked back
  (US4-S6).
- **The console stops mid-run**: the run is in memory and is not resumed. Read, not run.
- **Saturated, crowded and edge stars**: not measured (US1-S2).

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
  before. Nothing else in `crates/simulator` SHALL change (owner approval 1).
- **FR-002**: `crates/finder` SHALL gain the module `finder_focus` with
  `pub fn measure(pixels: &[u8], width: u32, height: u32, centroids: &[Centroid]) -> Sample`,
  033's `Sample`. Walking the centroids in order, it SHALL measure a star when its box of
  side 2·`BOX_PX` + 1 (`BOX_PX` 16), centred on the pixel holding the centroid, lies
  inside the frame, when no other centroid lies within `BOX_PX` of it in both x and y,
  and when no pixel of the box is at `SATURATED` (250) or above, until `STARS_MEASURED`
  (15) are measured. A star's size SHALL be its half-flux diameter: the background the
  median of the box's edge pixels; the noise 1.4826 times their median absolute
  deviation, at least 0.5; the pixels more than `THRESHOLD_NOISE` (3) noise levels above
  the background that are joined, by a side or a corner, to the brightest pixel within
  two pixels of the centroid (none when that pixel is not above it); their flux-weighted
  centre; and twice the radius from that centre at which their flux, summed in order of
  radius, reaches half its total, interpolated between pixel radii. The reading SHALL be
  the median of the sizes: fewer than 033's `MIN_STARS` stars SHALL answer
  `TooFewStars` with the count, a median above 033's `FAR_OUT_PX` `FarOut` with it and
  the count, and else `Reading` (owner decision 1; owner approval 2).
- **FR-003**: Nothing else SHALL change in US1: no route, view, page or test of another
  story or spec, and 033's module is used, not changed.

**The run** (US2):

- **FR-004**: `GET /api/finder-camera/focus` SHALL answer 200 `{"focus": ...}`, null
  before the first start in this process. `POST /api/finder-camera/focus` SHALL take
  the JSON body `{}` alone (any key 400, a body that is not JSON 415, through
  `sim_json` as 050's route takes its body), under the same-origin rule, and refuse, in
  this order: a run already running, 409
  `the finder camera's focus aid is already running`; and a console with no scope link
  or no raw-frame client on it, 409 `the scope node is not connected`. Otherwise it
  SHALL start a run and answer 200 `{"focus": ...}`. `DELETE /api/finder-camera/focus`
  (no body, same-origin) SHALL stop a running run and answer 200 `{"focus": ...}`, the
  view unchanged when none runs (owner decision 9; owner approvals 2 and 3).
- **FR-005**: A run SHALL ask the scope node for one raw frame after another (025's
  `RawFrame`, through `LinkManager::raw`), each within `RAW_WAIT`, 15 s; keep the star
  frames it sees meanwhile, the last eight, through `LinkManager::frames`; and measure
  each raw frame at the centroids of the star frame of its id, waiting up to 15 s more
  for that star frame and skipping the raw frame when it does not come. Each measure
  SHALL run off the async runtime and its `Sample` SHALL go to one 033 `Aid` made at the
  start (owner decision 2).
- **FR-006**: `focus` SHALL carry `state` (`running` or `ended`), `since` (console Unix
  seconds), `readings` (033's samples pushed), `cadence_ms` (between the last two, once
  there are two), `last` (`frame_id`, `state`: 033's phase word, `value` for a reading
  or a far-out frame, `stars`), `median` (`value`, `readings`), `best`, `trend`, `found`
  and `history`, each as 033's `Aid` answers it, every size in the finder camera's
  pixels; and, once ended, `ended` (`stopped` or `failed`) and, when failed, `error`.
- **FR-007**: A stop SHALL end the run at once, the raw request in flight dropped,
  `ended` `stopped`. A run SHALL end `failed` with `error`
  `the scope node is not connected` when the link has no raw-frame client or its star
  frames end; `this scope node cannot send raw frames` when the raw request answers
  `UNIMPLEMENTED`; `the raw frame request failed` for any other error, or a raw frame
  whose length is not its width times its height; and `no raw frame arrived within 15 s`
  when none comes within `RAW_WAIT`.
- **FR-008**: A run SHALL change no setting of the finder camera or the session, take
  no lease, write no file, move nothing, and leave the live loop as it is; it SHALL not
  read Track, the mount or the optics, and SHALL go on through a change of them (owner
  decisions 3 and 7).
- **FR-009**: Nothing else SHALL change in US2: 033's module, 025's recording, the link,
  the live loop and every existing test, US1's among them.

**MCP and the words** (US3):

- **FR-010**: MCP SHALL gain `get_finder_focus`, read-only, which takes no arguments
  (else the tool error `get_finder_focus takes no arguments`) and answers
  `{"focus": ...}`; and `set_finder_focus`, which takes `on`, a boolean (else
  `on must be true or false`), and nothing else
  (`set_finder_focus takes on and nothing else`), and starts or stops the aid as
  FR-004 does, answering `{"focus": ...}`, a refusal's words being its tool error
  (owner decision 6; owner approval 3).
- **FR-011**: The finder README SHALL gain the three route rows after
  `DELETE /api/imaging/align`'s, the MCP paragraph after the finder alignment's, and
  the section `## Focusing the finder camera` before `## Calibration frames`, as plan.md
  § Interfaces gives them; `The imaging camera has sixteen tools of its own` SHALL stay
  as it is. `GLOSSARY.md`'s **Focus reading**, **Focus aid** and **Raw frame** SHALL
  name the finder camera's aid as plan.md § Interfaces gives them (owner approvals 4
  and 5).
- **FR-012**: Nothing else SHALL change in US3: the routes, the run and the view of US2,
  033's tools, and every existing test, the earlier stories' among them.

**The page** (US4):

- **FR-013**: `crates/finder/web/src/api.ts` SHALL gain `FinderFocus`,
  `getFinderFocus`, `postFinderFocus` (body `{}`, `Content-Type: application/json`) and
  `deleteFinderFocus`.
- **FR-014**: `components/FocusTool.tsx` SHALL be a region named by its heading
  `Focus a camera`, with the select `Camera` (`Finder camera`, the default, and
  `Imaging camera`). With the finder camera picked it SHALL read
  `GET /api/finder-camera/focus` once a second, show `Reading the finder camera's focus aid.`
  until the first read, then one line on what it does, the button
  `Start the finder camera's focus aid`, or `Stop the finder camera's focus aid` while a
  run runs, and, once there is a run: `Focus reading`, the median as the large number with two decimals and ` px`, or
  `—` without one; the status `finderStatus` gives while it runs, else
  `The finder camera's focus aid stopped.`; `Best so far <best> px`; the trend's words
  (`Sharper than before the last turn`, `Softer than before the last turn`,
  `About the same as before the last turn`);
  `Passed through focus and came back to the best so far.` while `found`; the last 60
  readings as bars, an image named `Focus readings of the last N frames of the finder camera`;
  for a run ended `failed` the alert
  `The finder camera's focus aid stopped: <error>.`; and a refused start or stop's
  words as an alert. It SHALL use DESIGN.md's existing classes and
  `currentColor`, so night mode stays red only (owner decision 5; owner approval 7).
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
  another beside the live solve, each measured at its star frame's centroids and judged
  by 033's arithmetic, while the observer turns the focuser.
- **The finder camera's focus reading**: the median half-flux diameter, in the finder
  camera's pixels, of at most 15 isolated, unsaturated stars of one raw frame.
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
- **SC-003**: The reading follows the simulator's star size within 10 % at focus errors
  0 to 3 (US1-S4, US2-S1), the simulator's stars keep their flux as they widen (US1-S5),
  and a pass through focus and back is reported as `found`
  with the best within 10 % of the truth (US2-S2).
- **SC-004**: The live solve goes on and the session's camera settings are unchanged
  while the aid runs (US2-S1, US2-S3), and the imaging camera's focus aid is reached on
  Tools through 033's own block (US4-S4, US4-S6).

## Assumptions

- **Earlier features are in place.** Every spec in `depends_on_landed` has landed. Every
  citation here was read at c75a6e76.
- **The simulator's finder camera stands for the real one** in the gates: Gaussian stars
  whose width grows with the focus error, flux kept, so that, as on a real camera, stars
  fade out of the detection as they spread.
- **The scope node's centroids are the stars**: the aid measures where the node's
  detection, 048's on a dark sky, found stars, as the live solve uses them.
- **Out of scope:**
  - the scope node, the scope link's protocol and the node's detection;
  - 033's measure, arithmetic, record, verdict and rules, and its Imaging page block;
  - a motor focuser, the mount and an equatorial mount's needs;
  - a record or verdict for the finder camera;
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

Four stories, each under 60 % of 65,536 B (39,321 B) with room for the factory's
overrun, as the probe's commits measure them (plan.md § Plan traps 2): the measure and
the simulator's focuser; the run and its routes; MCP, the README and the glossary; the
page. Each merges after the one before (`depends_on_merged`): US2's run calls US1's
`measure` and its tests turn US1's focuser; US3's tools start and stop US2's run; the
page reads US2's view, and its browser test needs US2's routes. Plain `depends_on`
would order them without putting the earlier code in the later story's base.
