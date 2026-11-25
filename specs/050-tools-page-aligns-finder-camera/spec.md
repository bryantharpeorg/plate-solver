---
state: draft
depends_on_landed: ["013-live-finder", "016-mcp", "021-chart-orientation", "031-imaging-frames-on-console", "032-imaging-solve-and-aim-point", "033-focus-reading-and-aid", "035-live-stack", "036-tracking-from-subs", "037-imaging-run", "042-imaging-run-page-and-mcp", "047-console-uses-the-database-that-fits", "049-search-finds-whole-names"]
# Drafted 2026-11-25 against `main` at e2eee793 (e2eee793d9dff8b519a7038b6be2b36fc7d225d2),
# at the owner's ask that day ("a spec to get the finder and the imager roughly aligned,
# worried about the small FOV and getting them aligned. we should probably just generally
# have some utility pages in the system"), from a probe of that day on the branch
# `probe/050` cut from e2eee793: a trial build of all four stories on the simulator with
# every scenario's test and both gates passing, ASTAP's command-line solver run on a saved
# frame at hint offsets of 0° to 132°, and the page measured in a browser at three phone
# widths. Every `path:line` here was read at e2eee793. State: draft. The eight owner
# decisions are open; the scenarios and the record are written to the recommended
# answers. 031 and 032 are named because the alignment holds 031's lease and solves
# through 032's solve and hint and hands each solve to 032's aim point; 033 because a
# scenario starts its focus aid; 013 and 016 for the live loop and MCP; 021, 035, 036,
# 037, 042, 047 and 049 because each last changed a file this spec writes (plan.md
# § Refinement Record). What rests on a run and what on reading is said where it is used
# (plan.md § What was run and what was read).
---

# Feature Specification: The Tools Page Aligns the Finder Camera

**Feature Branch**: `050-tools-page-aligns-finder-camera`

**Created**: 2026-11-25

**Status**: Draft

**Input**: The finder camera may soon look through a 40 mm guide scope, about 1.72° by
1.29°, from six-point rings whose screws the owner turns by hand, while the imaging
camera on the main telescope sees about 0.75° by 0.50°. 032's measured aim point takes a
reading only when the imaging camera's centre falls inside the finder camera's frame
(`off_frame` otherwise), and a finder camera in rings is rarely that close at first.
Give the console a Tools page for utilities used once a setup, and on it a finder
alignment: while the owner turns the ring screws, it shows, live, how far and which way
the imaging camera points from the finder camera, also off the finder camera's frame,
and says plainly when he is done, which is when 032's aim point has taken over. Spec 051
adds focusing to the same page. Whichever finder camera or lens is fitted, nothing here
changes.

## Clarifications

### Session 2026-11-25 (the probe on `main` at e2eee793)

- Q: What does this trio read besides itself? → A: Nothing about the feature. `spec.md`
  holds every requirement and scenario, and `plan.md` every design decision, the
  interfaces, the shapes that existing tests pin and the alternatives rejected. A story
  that would change a decision escalates rather than re-deciding it. The probe's branch
  and logs are not in the repository; what they showed is written here.
- Q: What does the console do today when the two cameras point apart? → A: After an
  imaging solve, 032's `read_aim` (`crates/finder/src/imaging/relation.rs:284`)
  projects the imaging centre through the finder fix nearest the exposure and returns
  `OffFrame` when it falls outside the finder camera's frame
  (`crates/finder/src/imaging/relation.rs:306`, `crates/finder/src/imaging/relation.rs:309`),
  as 032's FR-020 says (`specs/032-imaging-solve-and-aim-point/spec.md:2032`). The
  console then says only `off_frame`: not how far, nor which way. Run on the trial: with
  the simulator's imaging camera 6.6° and 8.5° from the finder camera's centre, every
  imaging solve handed to 032 through `Imaging::note_solve` read `off_frame`.
- Q: How does an imaging solve find its field today? → A: Through 032's hint
  (`Imaging::hint`, `crates/finder/src/imaging/solving.rs:629`): the session's pointing
  within `HINT_AGE_S`, 30 s (`crates/finder/src/imaging/solving.rs:21`), with a radius of
  at most `WIDE_RADIUS_DEG`, 3° (`crates/finder/src/imaging/solving.rs:23`), and a field
  height from the kept scale, else the focal length, else none. The simulated imaging
  solver refuses a hint whose place is farther from the answer than its radius, or whose
  field height is more than 5 % off (`crates/finder/src/imaging/simulated.rs:122`). So
  today an imaging camera more than 3° off the finder camera's pointing never solves
  (by reading; plan.md § What was run and what was read).
- Q: What lets a later spec solve imaging frames itself and still feed the aim point?
  → A: 032's FR-037 (`specs/032-imaging-solve-and-aim-point/spec.md:2120`):
  `Imaging::note_solve` (`crates/finder/src/imaging/solving.rs:497`) takes an aim reading
  from a solve made outside `Imaging::solve`, for a whole-sensor frame, "exactly as FR-020
  to FR-022 and FR-036 take one", and changes neither the imaging view's `solve`, the
  kept scale nor the misses. This spec is such a later spec; it changes no word of
  FR-037.
- Q: How does a procedure take the imaging camera? → A: Through 031's lease (FR-028,
  `specs/031-imaging-frames-on-console/spec.md:1545`): `Imaging::lease(holder)`
  (`crates/finder/src/imaging/mod.rs:761`) never waits and answers
  `the imaging camera is held by {holder}` while another holds it, and `Lease::set` and
  `Lease::take` are the camera's settings and frames for the holder (031's FR-030). The
  focus aid holds it as `the focus aid` (`crates/finder/src/imaging/focus/run.rs:28`),
  remembers the settings, takes frames and puts the settings back
  (`crates/finder/src/imaging/focus/run.rs:553`); the alignment follows that pattern.
- Q: Can a test point the simulator's imaging camera off the finder camera's frame?
  → A: Not at e2eee793. The simulator's imaging camera points along the eyepiece pixel's
  ray (`Optics::eyepiece_px`, default (700, 420) in a 1456 × 1088 frame 12° wide,
  `crates/simulator/src/optics.rs:28`), and `eyepiece_ray` asserts that the pixel lies
  inside the frame (`crates/simulator/src/optics.rs:52`), so a pixel off the frame
  panics there (by reading the assert). US1 relaxes it to a finite pixel; with that, the
  trial's off-frame scenarios ran.
- Q: What navigation does the page have, and what does a new page need? → A: `App.tsx`
  switches on `window.location.pathname` among `/setup`, `/imaging` and the console
  (`crates/finder/web/src/App.tsx:187`); the top bar holds a Setup chip and, while a
  scope node is paired, an Imaging chip (`crates/finder/web/src/App.tsx:178`); Setup's
  header holds `Back to console` (`crates/finder/web/src/components/SetupPage.tsx:117`).
  The server answers every path without a file extension with `index.html`
  (`crates/finder/src/api.rs:1698`), so `/tools` needs no server change (run: the
  trial's `/tools` loaded in the browser).
- Q: What do the words "alignment" and "offset" mean in the glossary today? → A:
  `GLOSSARY.md:297` lists them under **Relation**'s _Avoid_, since the relation is
  what the console keeps between the cameras. This spec adds the term **Finder
  alignment**, the procedure, whose _Avoid_ names relation; its key for the place is
  `centre`, not `offset`.
- Q: Can the alignment be used by day? → A: No. Both cameras find their field by
  plate-solving stars, and neither solves a daytime scene. A rough start by day, on a
  distant object through the main telescope's eyepiece and the finder camera's preview,
  is the owner's, outside this tool (plan.md § Outside the factory).

## Owner decisions

All eight are open. Each keeps its options, its evidence and its recommendation, the
smallest design that works; the scenarios and the record are written to the
recommendations, and a different answer changes them before any flip. One general rule
is behind decisions 4 to 7: **the alignment adds no rule of its own to 032.** It gives
every frame the same hint, hands every solve through 032's own door (FR-037) and reads
032's own word back, so 032's module, its FRs and its tests are not touched and no side
effect on the aim point, the kept scale or the misses needs an approval. The candidates
were measured on data (decision 4).

1. **The page's name and place** (FR-015, FR-016). Recommended: (a) **a page named
   Tools at `/tools`, linked from Setup's header** while a scope node is paired, under
   `Back to console`, with one block per utility; 051 adds its block below. Measured in
   the browser on the trial (Playwright's Chromium at 360, 390 and 412 px wide): a
   Tools chip in the top bar beside Setup and Imaging made the page 416 px wide at 412
   and 390 and 373 px at 360, a sideways scroll on every phone; the Setup link left the
   page exactly the viewport's width, and the bar today already reaches 348 px of 360.
   Instead: (b) a **Tools chip in the top bar** (the overflow above, or a smaller chip
   for every page); (c) an **icon-only chip**, smaller but still one more item in that
   bar; (d) **a section on Setup**, which leaves 051 no page and makes Setup longer.
2. **How the direction is given** (FR-002, FR-017). Recommended: (a) **along the finder
   camera's own frame**: `right_deg` and `up_deg` along its rows and columns, as the
   finder camera sees it, with the chart rotation not applied, and `separation_deg` on
   the sky. The rings hold the finder camera, so its frame turns with the screws and not
   with the night, and the owner reads the words beside the finder camera's own preview.
   Instead: (b) **north and east on the sky**, which turn against the rings as an alt-az
   mount moves; (c) **ring-screw words** ("turn the upper screw"), which depend on how
   the rings and the camera sit, which the console does not know; (d) **the chart's
   rotated frame** (021), which follows a setting made for the eyepiece view.
3. **The exposure** (FR-005). Recommended: (a) **1000 ms unless the request names
   `exposure_ms`, the binning left as set.** At about 0.55″ a pixel (1800 mm and
   4.78 µm, by reading) an untracked star drifts up to 15″ a second, about 27 px, so a
   longer frame smears with Track off; 1 s gave the simulator's frames their stars on
   the trial and is inside the ASI071's range (045, by reading) and the simulated
   camera's. A 2 × 2 binning, if set, sends a quarter of the bytes.
   Instead: (b) **2000 ms**, more stars and twice the smear; (c) **force 2 × 2**, a
   fourth setting to remember and put back.
4. **The search** (FR-005). Recommended: (a) **one hint for every frame: the paired
   finder fix's centre with a 10° radius and the field height 032 would give.** Measured
   with ASTAP's command-line solver on a saved frame 0.5° high (times as shares of the
   on-target solve at a 10° radius): hint offsets inside their radius (2.5° at 3°, 8°
   and 9.9° at 10°, 12° at 15°) solved at 1.0 to 1.22 times, and the on-target solve at
   3° at 1.12 times; 20° at a 30° radius 1.66 times; 12° at a 10° radius failed, after
   1.13 times; a whole-sky search from a hint about 130° away solved at 12.2 times; with
   no field height the on-target and the 8° hints failed, after 8.7 times; a field
   height 20 % high failed, and the right one solved at 8°. Instead: (b) **032's 3°**,
   which misses an imaging camera 3° to 10° off; (c) **a follow mode**, a narrow radius
   round the last solve once one solves, a second rule for no measured gain; (d) **the
   whole sky**, twelve times slower here and failing without a field height.
5. **What "done" means** (FR-010, FR-017). Recommended: (a) **the imaging camera's
   centre in the middle half of the finder camera's frame** (within a quarter of its
   width and of its height of its centre) **and 032's aim point measured or replaced
   from that frame.** The middle half scales with the optic: about ±0.43° by ±0.32° for
   the guide scope, about ±4.3° by ±3.3° for the 16 mm lens, by arithmetic. "Measured or
   replaced" is 032's own word, so a reading taken while a screw still turns, which 032
   sets aside or calls `moving`, never says Done (run: after a move into the middle,
   `set_aside` and Done false, then `replaced` and Done true). Instead: (b) **on the
   frame**, where 032 can measure, but at the edge, where a bump puts it off again;
   (c) **a fixed angle**, such as 0.25°, which is a different job for each optic.
6. **How it relates to 032's aim point** (FR-010). Recommended: (a) **every solve
   through `Imaging::note_solve`**, 032's FR-037, and `done` read from 032's
   `relation.last.outcome` for that frame: once the centre is on the frame 032 measures
   the aim point as from any imaging solve. Run: at the stop the session's aim point was
   measured at the simulator's eyepiece pixel within 1.5 px. Instead: (b) **no handover**,
   the owner then solving on the Imaging page; (c) **count the alignment's solves as
   032's route solves**, keeping their scale and counting their misses: a change to
   032's rules; (d) **the alignment sets the aim point itself** from its own pairing, a
   second aim rule beside 032's.
7. **Track, and the camera's other holders** (FR-004). Recommended: (a) **Track may be
   on; another holder of the lease refuses the start with its name**, as 031 words it.
   Track moves both cameras together and keeps 1 s stars round; while it is on, 032
   holds the aim point back until Track is off (FR-036,
   `specs/032-imaging-solve-and-aim-point/spec.md:2110`; read, not run). An imaging run,
   the stack, the focus aid and a set of darks or flats hold the lease, so each refuses
   the start. Instead: (b) **refuse while Track is on**, which costs round stars and
   the owner a step.
8. **A browser test** (FR-018). Recommended: (a) **`e2e/tools.spec.ts` in the simulate
   project**, at 412 by 915, from Setup's link to Done on the simulated console, with
   the page no wider than the viewport: two list entries in `playwright.config.ts`
   (owner approval 6), as 035 added its stack check. The width is what decision 1
   measured, and only a browser lays the page out. Instead: (b) **none**, as 042 chose,
   Vitest alone driving the block.

## Owner approvals

Flipping this spec to `ready` approves the answer to every owner decision above, and the
following. Each is named in the FR that makes it. No existing test, golden, fixture or CI
file changes.

1. **A change to the simulator** (FR-001): `Optics::eyepiece_px` may lie off the finder
   camera's frame, a relaxed assert in `eyepiece_ray`. Nothing else in
   `crates/simulator` changes.
2. **New public API in `crates/finder`'s library** (FR-002, FR-004 to FR-007, FR-010): the
   module `finder::imaging::align` with `HOLDER`, `PURPOSE`, `ALIGN_EXPOSURE_MS`,
   `PAIR_S`, `SEARCH_RADIUS_DEG`, `MIDDLE_SHARE`, `AlignAsk`, `Centre`, `centre_of`,
   `AlignLast` and `AlignView`; `Imaging::align_start`, `Imaging::align_stop` and
   `Imaging::align_running`; and `ImagingView::align`. No existing signature changes.
3. **New routes and tools** (FR-004, FR-007, FR-011): `POST` and
   `DELETE /api/imaging/align`, and the MCP tools `get_finder_alignment` and
   `set_finder_alignment`.
4. **The glossary** (FR-012): the new term **Finder alignment**, and the finder
   alignment added to **Lease**'s holders and `align` to **Purpose label**'s words.
5. **The finder README's section, rows and MCP paragraph** (FR-012).
6. **Two list entries in `crates/finder/web/playwright.config.ts`** (FR-018): `"**/tools.spec.ts"`
   in the default project's `testIgnore` and in the simulate project's `testMatch`,
   each on a line of its own. Nothing else in that file changes.
7. **The page** (FR-015, FR-016): the Tools page at `/tools` and Setup's `Tools` link.

## User Scenarios & Testing *(mandatory)*

Every scenario shares these definitions.

- **The gate commands.** Every Rust scenario runs in
  `cargo nextest run --workspace --all-targets --all-features --locked`, the `test`
  gate's first command, each test in its own process. The page's Vitest scenarios run in
  `npm test`, and the browser scenario in `CI=1 npm run e2e`, later commands of the same
  gate, in `crates/finder/web`.
- **The centre frame at (α, δ)** is a `FinderFix` (`crates/finder/src/imaging/relation.rs:204`)
  of time 0 and frame id 1 whose frame is `Frame::from_solution` of a solution at RA α
  and Dec δ, roll 0, with `PinholeCamera::new(1456.0, 1088.0, 12° in radians)`, as
  032's tests build theirs (`crates/finder/tests/console/imaging_relation.rs:248`):
  north up and east left. **f** is 728 / tan 6°, the finder camera's focal length in
  pixels.
- **The pinhole oracle of a pixel (x, y)** is right = atan((x − 728) / f),
  up = atan((544 − y) / f) and all = atan(hypot(x − 728, 544 − y) / f), in degrees. For
  the pixels used: (700, 420) is −0.23°, 1.03° and 1.05°; (−60, 420) is −6.49°, 1.03°
  and 6.57°; (−300, 420) is −8.44°, 1.03° and 8.50°; (1800, 100) is 8.80°, 3.67° and
  9.51°; (900, 700) is 1.42°, −1.29° and 1.92°; (−1100, 420) is 14.8° in all.
- **The alignment rig at e** is the simulator over `crate::support::live::DB` with
  `Site::default()` on a `VirtualClock` from `crate::support::live::START`, a mount
  connected and pushed to altitude 50° and azimuth 120°, and `set_optics` with
  `eyepiece_px` e and the rest `Optics::default()`; a `SimNode` over it with the token
  `test-token` and a 500 ms frame interval, reached over an in-memory duplex as
  `crates/finder/tests/console/imaging_relation.rs` reaches its node; a `Session` paired
  with that node and given `Site::default()`'s latitude and longitude as its site; and
  `AppState::shared` over that
  database with `with_session`, `with_clock` of the simulator's clock, `with_link`,
  `with_live()` (left out where the scenario says **without the live loop**),
  `with_imaging` of a folder under a `crate::support::Scratch`, and `with_imaging_solver`
  of the simulated imaging solver over the simulator (left out where the scenario says
  **without a solver**), served by `finder::router`. **Ready** means `GET /api/imaging`
  reads `state` `ready`, `GET /api/session` reads `pointing.source` `live`, and 5 s more
  have passed. **Turning a screw to e** is `set_optics` with `eyepiece_px` e mid-run.
  **The first measured frame after n** is `align.last` once its `outcome` is `measured`
  and its `frame_id` exceeds n, read from `GET /api/imaging` every 100 ms. The tests run
  under `#[tokio::test(start_paused = true)]`, and every state awaited comes within
  120 s of paused time.
- **The MCP rig** is the alignment rig at (700, 420) served with `/mcp`, the tools called
  with `tools/call` as `crates/finder/tests/mcp/` calls them.
- **The page rig** is the component or `App` rendered by Testing Library with `fetch`
  stubbed to answer the views the test writes, and `EventSource` stubbed, as the page's
  existing tests do.
- **The simulated console** is the simulate project's server of
  `crates/finder/web/playwright.config.ts:54`, the release `finder` with `--simulate`,
  whose finder camera and imaging camera are the simulator's default optics: the eyepiece
  pixel (700, 420), in the middle half of the frame.
- **Oracles.** The simulator's eyepiece pixel, which is where its imaging camera points in
  the finder camera's frame, and the pinhole oracle above; the centre frame's own
  arithmetic; 031's and 032's words; and the words of these scenarios. No expected value
  comes from the code under test: no test calls `centre_of` or reads `MIDDLE_SHARE`,
  `PAIR_S` or `SEARCH_RADIUS_DEG` for an expected value.

### User Story 1 - The console places the imaging camera's centre off the finder camera's frame (Priority: P1)

As the owner, I need the console to say where the imaging camera points in the finder
camera's frame however far apart they start, and the simulator to point its imaging
camera off the finder camera's frame so that this can be tested.

**Why this priority**: Without the place off the frame, the alignment has nothing to
show until the cameras are already close.

**Independent Test**: `crates/finder/tests/console/imaging_align_centre.rs`, new. The
`test` gate's `cargo nextest run --workspace --all-targets --all-features --locked` runs
it; during development,
`cargo nextest run -p finder --locked --test console -E 'test(imaging_align_centre)'`.

**Acceptance Scenarios**:

1. **Given** the centre frame at (100°, 20°), **When** a committed test calls
   `centre_of` for RA 100° and Dec 21°, **Then** it asserts that `x` is 728 and `y` is
   544 − f·tan 1°, each within 10⁻⁶ px, that `right_deg` is 0, `up_deg` 1 and
   `separation_deg` 1, each within 10⁻⁹°, that `on_frame` and `middle` are true, and
   that `finder_width` is 1456 and `finder_height` 1088.
2. **Given** the centre frame at (100°, 20°), **When** a committed test calls
   `centre_of` for RA 100° and Dec 25°, **Then** it asserts that `y` is
   544 − f·tan 5° within 10⁻⁶ px, below 0, that `up_deg` is 5 within 10⁻⁹°, and that
   `on_frame` and `middle` are false.
3. **Given** the centre frame at (100°, 0°), **When** a committed test calls `centre_of`
   for RA 108° and for RA 96°, both at Dec 0°, **Then** it asserts that the first's
   `right_deg` is −8 and `up_deg` 0 within 10⁻⁹°, its `x` 728 − f·tan 8° within
   10⁻⁶ px and its `on_frame` false, and that the second's `right_deg` and
   `separation_deg` are 4 within 10⁻⁹°, its `on_frame` true and its `middle` false.
4. **Given** the centre frame at (100°, 20°), **When** a committed test calls
   `centre_of` for RA 280° and Dec −20°, the point behind the camera, **Then** it
   asserts that the answer is `None`.
5. **Given** a simulator over `crate::support::live::DB` pushed to altitude 50° and
   azimuth 120°, **When** a committed test calls `set_optics` with `eyepiece_px`
   (−300, 420) and the rest `Optics::default()`, **Then** it asserts that no panic
   occurs and that the angle between `imaging_truth()`'s RA and Dec and
   `telescope().camera`'s is atan(hypot(1028, 124) / f) within 10⁻⁶°.

### User Story 2 - The finder alignment runs and shows the place frame by frame (Priority: P1)

As the owner, I start the finder alignment and it takes imaging frames one after another,
each solved and paired with the finder camera's live solve, and shows where the imaging
camera's centre lies from the finder camera's centre while I turn the ring screws, until
I stop it, when it puts the imaging camera's settings back.

**Why this priority**: It is the tool.

**Independent Test**: `crates/finder/tests/console/imaging_align.rs`, new. The `test`
gate's nextest command runs it; during development,
`cargo nextest run -p finder --locked --test console -E 'test(/^imaging_align::/)'`.

**Acceptance Scenarios**:

1. **Given** the alignment rig at (700, 420), ready, **When** a committed test sends
   `POST /api/imaging/align` with `{}` and reads the first measured frame after 0,
   **Then** it asserts that the answer is 200 whose `align` is exactly `state`
   `running`, `since`, `exposure_ms` 1000.0 and `frames` 0; that the frame's `centre`
   has `x` and `y` within 2 px of 700 and 420, `right_deg`, `up_deg` and
   `separation_deg` within 0.02° of the pinhole oracle, `finder_width` 1456,
   `finder_height` 1088 and `on_frame` true; and that `GET /api/imaging` then reads the
   frame's `purpose` `align`, `settings.exposure_ms` 1000.0, `settings.roi`
   `{"x":0,"y":0,"width":1236,"height":822}` and `settings.save_at_node` false.
2. **Given** three alignment rigs, at (−60, 420), (−300, 420) and (1800, 100), each
   ready, **When** a committed test starts the alignment on each and reads its first
   measured frame after 0, **Then** it asserts for each that `x` and `y` are within 2 px
   of the eyepiece pixel, `right_deg`, `up_deg` and `separation_deg` within 0.02° of the
   pinhole oracle, and `on_frame` false.
3. **Given** the alignment rig at (−1100, 420), 14.8° out, ready, **When** a committed
   test starts the alignment and reads two frames whose `outcome` is `not_solved`, the
   second after the first, **Then** it asserts that `align.state` is still `running`,
   that the second's `error` is exactly `the imaging solver found no solution`, and that
   it has no `centre`.
4. **Given** the alignment rig at (−300, 420), ready, the alignment started and its first
   measured frame read, **When** a committed test turns a screw to (1800, 100), skips the
   next measured frame (taken before the turn) and reads the one after, and then does the
   same for (900, 700), **Then** it asserts that each frame read has `x` and `y` within
   2 px of its new eyepiece pixel and `right_deg`, `up_deg` and `separation_deg` within
   0.02° of its pinhole oracle, with `on_frame` false at (1800, 100) and true at
   (900, 700).
5. **Given** the alignment rig at (700, 420), ready, the alignment started and its first
   measured frame read, **When** a committed test sends `PUT /api/session` with
   `{"camera_settings": {"exposure_ms": 1, "gain": 1}}`, which leaves the finder
   camera's frames without stars, **Then** it asserts that the answer is 200, and that a
   later frame's `outcome` is `no_live_solve` with no `centre` while `align.state` is
   `running`.
6. **Given** an alignment rig at (700, 420) without the live loop, its camera ready, one
   without a solver, ready, and one with both, ready, **When** a committed test sends
   `POST /api/imaging/align` to each as follows, **Then** it asserts these answers in
   order: without the live loop, 409 `{"error": "the finder camera has no live solve"}`;
   without a solver, 409 `{"error": "this console has no imaging solver"}`; with both,
   `{"exposure_ms": 1e9}` 400
   `{"error": "exposure_ms 1000000000 is outside the camera's range 1–3600000 ms"}` and
   `{"x": 1}` 400; with the focus aid started by `POST /api/imaging/focus` `{}` (200),
   409 `{"error": "the imaging camera is held by the focus aid"}`; with the focus aid
   stopped (200), `{}` 200, and then a second `{}` 409
   `{"error": "the finder alignment is already running"}`, and
   `POST /api/imaging/focus` `{}` and `POST /api/imaging/frame` each 409
   `{"error": "the imaging camera is held by the finder alignment"}`.
7. **Given** the alignment rig at (700, 420), ready, **When** a committed test sends
   `DELETE /api/imaging/align` with nothing running, sets
   `PUT /api/imaging/settings` `{"exposure_ms": 2000, "roi": {"x": 0, "y": 0, "width": 618, "height": 410}, "save_at_node": true}`,
   reads the mount's true direction, starts the alignment, reads three measured frames,
   one after another, and sends `DELETE /api/imaging/align`, **Then** it asserts that
   the first stop answers 200 with no `align`, that the second answers 200 with
   `align.state` `ended`, `align.ended` `stopped` and no `align.error`, that its
   `settings` equal those the settings request answered, that the mount's true direction
   is unchanged, that exactly one `.fits` file is left under the imaging folder's
   `frames`, and that `POST /api/imaging/frame` then answers 200.

### User Story 3 - The alignment says Done once the aim point has taken over, and MCP and the README carry it (Priority: P2)

As the owner, I am told Done when the imaging camera's centre is in the middle of the
finder camera's frame and the console has measured the aim point there; an MCP client
can run the same alignment for me; and the README says how.

**Why this priority**: The alignment works without Done; Done is what makes it a tool
that ends.

**Independent Test**: `crates/finder/tests/console/imaging_align_aim.rs` and
`crates/finder/tests/mcp/imaging_align.rs`, new. The `test` gate's nextest command runs
them; during development,
`cargo nextest run -p finder --locked --test console --test mcp -E 'test(imaging_align)'`.

**Acceptance Scenarios**:

1. **Given** the alignment rig at (700, 420), ready, **When** a committed test starts the
   alignment and reads the first measured frame after 0, **Then** it asserts that its
   `aim` is `measured`, its `centre.middle` true and its `done` true, and that
   `GET /api/session` reads `aim.source` `measured` with `aim.x` within 1.5 px of 700
   and `aim.y` within 1.5 px of 420.
2. **Given** the alignment rig at (700, 420), ready, the alignment started and three
   measured frames read, **When** a committed test turns a screw to (−300, 420), reads
   measured frames until one has `aim` `off_frame`, then turns a screw to (900, 700) and
   reads measured frames until one has `done` true, keeping the `aim` and `done` of each
   whose `centre.middle` is true, **Then** it asserts that the `off_frame` frame's `done`
   is false, that the first frame kept is `set_aside` with `done` false and the last
   `replaced` with `done` true, and that `GET /api/session` reads `aim.x` within 1.5 px
   of 900 and `aim.y` within 1.5 px of 700.
3. **Given** the MCP rig, **When** a committed test calls `get_finder_alignment` with
   `{}`, then `set_finder_alignment` with `{}` and with `{"on": false, "exposure_ms": 5}`
   and `get_finder_alignment` with `{"x": 1}`, then, once ready, `set_finder_alignment`
   with `{"on": true}` twice, then `get_finder_alignment` once a frame is measured, then
   `set_finder_alignment` with `{"on": false}`, and lists the tools, **Then** it asserts
   that the first answers `structuredContent` `{"align": null}`; that the next three are
   tool errors with exactly `on must be true or false`,
   `set_finder_alignment with on false takes no other argument` and
   `get_finder_alignment takes no arguments`; that the first start's `align.state` is
   `running` and the second is a tool error with exactly
   `the finder alignment is already running`; that the read's `align.last.centre.x` is
   within 2 px of 700; that the stop's `align.ended` is `stopped`; and that the list
   holds both tools, `get_finder_alignment` with `readOnlyHint` true.

### User Story 4 - The Tools page holds the finder alignment (Priority: P2)

As the owner, at the telescope with a phone, I open Tools from Setup, start the finder
alignment, read which way and how far the imaging camera points and see it drawn against
the finder camera's frame, and stop it when it says Done.

**Why this priority**: The routes and MCP work without the page; the page is how the
owner uses it at the telescope.

**Independent Test**: `crates/finder/web/src/components/AlignmentBlock.test.tsx` and
`crates/finder/web/src/components/ToolsPage.test.tsx`, new, under `npm test`, and
`crates/finder/web/e2e/tools.spec.ts`, new, under `CI=1 npm run e2e`; during
development, `npm test -- AlignmentBlock ToolsPage` and
`npx playwright test --project simulate e2e/tools.spec.ts` in `crates/finder/web`.

**Acceptance Scenarios**:

1. **Given** the page rig, **When** a committed test calls `centreLine` with four
   centres and `done`, **Then** it asserts these texts exactly: for right −6.49°, up
   1.03°, 6.57° in all, off the frame, `The imaging camera points 6.5° left and 1.0° up of the finder camera's centre, as the finder camera sees it: 6.6° in all, off the finder camera's frame.`;
   for 4.72°, −3.77° and 6.03° on the frame,
   `The imaging camera points 4.7° right and 3.8° down of the finder camera's centre, as the finder camera sees it: 6.0° in all, on the finder camera's frame.`;
   for 1.42°, −1.29° and 1.92° in the middle,
   `The imaging camera points 1.4° right and 1.3° down of the finder camera's centre, as the finder camera sees it: 1.9° in all, in the middle of the finder camera's frame. Done once the aim point is measured from a frame taken while the finder camera holds still.`;
   and for −0.23°, 1.03° and 1.05° in the middle with `done` true,
   `Done: the imaging camera's centre is in the middle of the finder camera's frame, and the aim point is measured. The imaging camera points 0.2° left and 1.0° up of the finder camera's centre, as the finder camera sees it: 1.1° in all.`
2. **Given** the page rig with `AlignmentBlock` over a running alignment, **When** a
   committed test renders its last frame as `not_solved` frame 7 with the error
   `the imaging solver found no solution`, and then as `no_live_solve` frame 8,
   **Then** it asserts that the status reads exactly
   `Imaging frame 7 did not solve: the imaging solver found no solution. The next frame looks again.`
   and then `The finder camera had no live solve for imaging frame 8.`
3. **Given** the page rig with `AlignmentBlock` over a ready camera with no alignment,
   **When** a committed test presses `Start aligning`, renders a running alignment whose
   last frame is measured off the frame, and presses `Stop aligning`, **Then** it asserts
   that the first request is `POST /api/imaging/align` with the header
   `Content-Type: application/json` and the body `{}`, that the view is refreshed after
   each press, that the image named
   `The imaging camera's centre off the finder camera's frame` is shown, and that the
   last request is `DELETE /api/imaging/align`.
4. **Given** the page rig with `AlignmentBlock` over an alignment ended `failed` with the
   error `the frame failed`, **When** a committed test renders it, **Then** it asserts
   that the alert reads exactly `The finder alignment stopped: the frame failed.` and
   that the button reads `Start aligning`.
5. **Given** the page rig with `App` at `/setup` and a session with a paired scope node,
   **When** a committed test clicks the link `Tools`, and then `Back to console`,
   **Then** it asserts that the path becomes `/tools` with the level-one heading `Tools`
   and the button `Start aligning`, and then `/`.
6. **Given** the page rig with `App` at `/setup` and a session with no scope node,
   **When** a committed test waits for the heading `Setup`, **Then** it asserts that no
   link named `Tools` is shown.
7. **Given** the simulated console in a 412 by 915 viewport, day mode, **When** a
   committed Playwright test opens `/setup`, clicks `Tools`, presses `Start aligning`,
   and after the Done check presses `Stop aligning`, **Then** it asserts that the URL
   ends in `/tools` with the heading `Tools`, that within 60 s the status contains
   `Done: the imaging camera's centre is in the middle`, that the image named
   `The imaging camera's centre on the finder camera's frame` is visible, that the
   document's `scrollWidth` is at most 412, and that `Start aligning` shows again within
   15 s of the stop.

### Edge Cases

- **Track on.** Allowed (owner decision 7); the stars stay round and the two cameras move
  together. 032 holds an aim reading taken with Track on back until the first live
  solve with Track off (its FR-036); `last.aim` still reads 032's word. Read, not run.
- **Another holder of the camera** (an imaging run, the stack, the focus aid, a set of
  darks or flats) refuses the start with its name (US2-S6 for the focus aid; the others
  by 031's FR-028, read). While the alignment runs, each of them, a frame and the
  settings are refused with `the imaging camera is held by the finder alignment`
  (US2-S6 for the focus aid and the frame).
- **A frame in flight with no lease** refuses the start with `an exposure is in progress`,
  and a camera that is not ready with 031's own words (`the scope node is not connected`
  and the rest, `crates/finder/src/imaging/mod.rs:591`). Read, not run.
- **A frame that does not solve** (clouds, too few stars, more than 10° out) is
  `not_solved` with the solver's words, counts no miss and changes no field height
  (032's FR-037); the next frame looks again (US2-S3).
- **No field height.** With no kept scale and no `--focal-length-mm`, the hint carries
  no field height: the simulated imaging solver solves, and ASTAP failed without one
  (owner decision 4). The README says to give one first; plan.md § Outside the factory
  says how.
- **A bumped finder camera mid-run** is a turned screw: the next frames follow (US2-S4),
  and 032 sets its readings aside until it replaces them (US3-S2).
- **A failed exposure** ends the run `failed` with the frame's error, the settings put
  back and the camera freed (FR-007). Read, not run.
- **The console stops mid-run**: the run is in memory and is not resumed; the imaging
  settings are the alignment's until set again. Read, not run.
- **A frame picked from the top bar while on Tools** opens the console's page, as from
  Setup and Imaging (FR-015). Read, not run.

## Requirements *(mandatory)*

### Functional Requirements

**The place** (US1):

- **FR-001**: The simulator's `Optics::eyepiece_px` MAY be any finite pixel, off the
  finder camera's frame too: `eyepiece_ray` (`crates/simulator/src/optics.rs:50`) SHALL
  assert only that it is finite, and `Simulator::set_optics` SHALL then point the
  imaging camera along that pixel's ray as it does for a pixel inside (owner approval 1).
  Nothing else in `crates/simulator` SHALL change.
- **FR-002**: `crates/finder` SHALL gain the module `imaging::align` with
  `pub const MIDDLE_SHARE: f64 = 0.25`, `pub struct Centre` of `x`, `y`,
  `finder_width`, `finder_height`, `right_deg`, `up_deg`, `separation_deg`, `on_frame`
  and `middle`, and
  `pub fn centre_of(fix: &FinderFix, ra_deg: f64, dec_deg: f64) -> Option<Centre>`.
  `x` and `y` SHALL be the imaging centre's place on the fix frame's pinhole model
  (`Frame::project_pinhole`), outside the frame when it falls outside;
  `right_deg` = atan((x − w/2) / f) and `up_deg` = atan((h/2 − y) / f), with w and h the
  frame's size and f = (w/2) / tan(fov/2); `separation_deg` the angle on the sky
  between the frame's centre and the imaging centre; `on_frame` 032's own test of
  FR-020, the place with distortion (`Frame::project`) inside the frame with no margin;
  and `middle` true when `on_frame` is and |x − w/2| ≤ `MIDDLE_SHARE`·w and
  |y − h/2| ≤ `MIDDLE_SHARE`·h. It SHALL answer `None` for a centre behind the finder
  camera (owner decisions 2 and 5; owner approval 2).
- **FR-003**: Nothing else SHALL change in US1: no route, view, page or test of another
  story or spec.

**The run** (US2):

- **FR-004**: `POST /api/imaging/align` SHALL take the JSON body `{}` or
  `{"exposure_ms": N}` (any other key 400, a body that is not JSON 415, as the focus
  aid's route takes its body) and, on a console with no imaging state, answer 409
  `this console takes no imaging frames`. `Imaging::align_start` SHALL refuse, in this
  order: a camera that is not `ready`, with 031's words for that state; no imaging
  solver, 409 `this console has no imaging solver`; a run already running, 409
  `the finder alignment is already running`; no finder fix within `HINT_AGE_S` of now,
  409 `the finder camera has no live solve`; an imaging frame exposing or receiving that
  no lease holds, 409 `an exposure is in progress`; an `exposure_ms` that is not finite
  or is outside the camera's range, 400
  `exposure_ms <N> is outside the camera's range <min>–<max> ms`; and last the lease,
  `Imaging::lease("the finder alignment")`, refused with
  `the imaging camera is held by <holder>`. It SHALL then answer 200 with the imaging
  view, its `align` running. Track, the mount and the session SHALL NOT be read for a
  refusal (owner decision 7; owner approvals 2 and 3).
- **FR-005**: The run SHALL remember the imaging camera's exposure, region of interest
  and `save_at_node`; set, through the lease, the exposure to `exposure_ms` or
  `ALIGN_EXPOSURE_MS` (1000), the whole sensor and `save_at_node` false, leaving the
  binning as it is; and then take one imaging frame after another under the purpose
  label `PURPOSE` (`align`) until stopped. Each saved frame SHALL be paired with the
  finder fix whose time lies nearest the middle of its exposure, no more than `PAIR_S`
  (5 s) from it; with none, the frame's `outcome` SHALL be `no_live_solve` and it SHALL
  not be solved. Otherwise it SHALL be solved with 032's `solve_frame` and a hint of
  source `pointing` at that fix's frame centre, radius `SEARCH_RADIUS_DEG` (10°), and
  the field height `Imaging::hint` gives the frame; a solve SHALL give `outcome`
  `measured` and the `centre` of `centre_of` for the solved centre, and a failed solve
  `outcome` `not_solved` with the solver's error message as `error`. The previous
  frame's file and solve sidecar SHALL be removed once the next frame is saved, so that
  only the last is kept (owner decisions 3 and 4).
- **FR-006**: The imaging view SHALL gain `align`, absent until the first start in this
  process and then the latest run's: `state` (`running` or `ended`), `since` (console
  Unix seconds), `exposure_ms`, `frames` (the frames taken), and, once there is one,
  `last` of `frame_id`, `at` (the middle of its exposure), `outcome`, and `error` or
  `centre` as FR-005 gives; once ended, `ended` (`stopped` or `failed`) and, when
  failed, `error`. Every other key of the imaging view SHALL stay as it is.
- **FR-007**: `DELETE /api/imaging/align` (no body) SHALL stop the run: mark it stopped,
  abort the exposure in flight, wait until the run has ended, its three settings put
  back through the lease and the lease dropped, and answer 200 with the imaging view,
  `align.ended` `stopped`. With nothing running it SHALL answer 200 with the view
  unchanged. A frame that fails SHALL end the run the same way with `ended` `failed`
  and that frame's error as `error`.
- **FR-008**: The alignment SHALL never move the mount, change the session's pointing,
  target or settings, or write anything but its own imaging frames and their sidecars.
- **FR-009**: Nothing else SHALL change in US2: 031's lease, frames, settings and words,
  032's solve, hint and relation, the focus aid, and every existing test, US1's among
  them.

**Done, MCP and the words** (US3):

- **FR-010**: Each solve SHALL be offered to the aim point through `Imaging::note_solve`
  (032's FR-037) before its line is written, and `last` SHALL gain `aim`, the word of
  the imaging view's `relation.last.outcome` when `relation.last` is of this frame, and
  `done`, true exactly when `centre.middle` is true and `aim` is `measured` or
  `replaced` (owner decisions 5 and 6).
- **FR-011**: MCP SHALL gain `get_finder_alignment`, read-only, which takes no arguments
  (else the tool error `get_finder_alignment takes no arguments`) and answers
  `{"align": ...}`, null before the first start; and `set_finder_alignment`, which
  takes `on`, a boolean (else `on must be true or false`), and with `on` true optionally
  `exposure_ms`, with `on` false nothing else
  (`set_finder_alignment with on false takes no other argument`), starts or stops the
  alignment as FR-004 and FR-007 do and answers `{"align": ...}`, a refusal's words
  being its tool error; on a console with no imaging state both answer the tool error
  `this console takes no imaging frames` (owner approval 3).
- **FR-012**: The finder README SHALL gain the two route rows, the MCP paragraph and the
  section `## Finder alignment` before `## Calibration frames`, as plan.md § Interfaces
  gives them; the sentence `The imaging camera has sixteen tools of its own`
  (`crates/finder/README.md:88`) SHALL stay as it is. `GLOSSARY.md` SHALL gain
  **Finder alignment** before **Focus check**, and **Lease** and **Purpose label**
  SHALL gain the finder alignment and `align`, as plan.md § Interfaces gives them
  (owner approvals 4 and 5).
- **FR-013**: Nothing else SHALL change in US3: 032's module, the routes and loop of US2
  but for `last`'s two keys, and every existing test, US1's and US2's among them.

**The page** (US4):

- **FR-014**: `crates/finder/web/src/api.ts` SHALL gain `ImagingAlignCentre`,
  `ImagingAlign`, `postImagingAlign` (body `{}`), `deleteImagingAlign` and
  `ImagingView`'s optional `align`.
- **FR-015**: The page SHALL gain the Tools page at `/tools`: the level-one heading
  `Tools`, a `Back to console` link to `/`, and below them the finder alignment's block,
  over 031's `useImaging`; a frame picked from the top bar while on `/tools` SHALL open
  the console's page, as from `/setup` and `/imaging` (owner decision 1; owner
  approval 7).
- **FR-016**: Setup's header SHALL show a link `Tools` to `/tools` under
  `Back to console` while the session has a scope node, and none without one.
- **FR-017**: The block SHALL hold the heading `Align the finder camera`, one line on
  what it does, and the button `Start aligning`, or `Stop aligning` while it runs. Its
  status SHALL read `Taking the first imaging frame.` before the first frame; for a
  measured frame the line of US4-S1, where the place is "The imaging camera points
  X° right (left) and Y° up (down) of the finder camera's centre, as the finder camera
  sees it: Z° in all", one decimal each; for `not_solved`
  `Imaging frame N did not solve: <error>. The next frame looks again.`; and for
  `no_live_solve` `The finder camera had no live solve for imaging frame N.` A measured
  frame SHALL also draw the finder camera's frame, its middle half dashed, and the
  imaging camera's centre, held to the frame's edge when off it, as an image named
  `The imaging camera's centre on (off) the finder camera's frame`. An ended `failed`
  run SHALL show the alert `The finder alignment stopped: <error>.`, and a refused start
  or stop the route's words as an alert. It SHALL use DESIGN.md's existing classes and
  `currentColor`, so night mode stays red only.
- **FR-018**: `crates/finder/web/e2e/tools.spec.ts` SHALL hold US4-S7, with
  `"**/tools.spec.ts"` added to the default project's `testIgnore` and the simulate
  project's `testMatch` in `crates/finder/web/playwright.config.ts` (owner decision 8;
  owner approval 6); `crates/finder/web/dist` SHALL be rebuilt and committed. Nothing
  else on the page SHALL change.

### Key Entities

- **Finder alignment**: imaging frames taken while the observer turns the finder
  camera's ring screws, each showing where the imaging camera's centre lies from the
  finder camera's centre, also off the finder camera's frame, until it lies in the
  middle of the frame and the aim point is measured there.
- **Centre** (of the finder alignment): where an imaging solve's centre lies in the
  paired finder fix's frame: pixels, right and up along the frame, the angle in all,
  on the frame, in its middle.
- **The middle of the frame**: within a quarter of the finder camera's frame's width
  and of its height of its centre.
- **The Tools page**: the console's page of utilities used once a setup, one block each.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario has a committed test that asserts it, and the
  `lint` and `test` gates pass.
- **SC-002**: No existing test, golden, fixture, `scripts/ci/` file, `.github/` file,
  `CODEOWNERS` entry or `ergane.yaml` is edited, and `playwright.config.ts` gains only
  its two list entries. Every existing test passes unchanged (plan.md § Shapes that
  existing tests pin). The only edits to existing test files are the `mod` lines of
  `crates/finder/tests/console/main.rs` and `crates/finder/tests/mcp/main.rs`.
- **SC-003**: The place is reported off the finder camera's frame as on it, to 0.02° of
  the simulator's truth, up to 10° out (US2-S1, US2-S2), and follows a turned screw from
  the second frame after the turn (US2-S4).
- **SC-004**: Done is said only in the middle of the frame with 032's aim point measured
  or replaced from that frame, and the session's aim point is then within 1.5 px of the
  truth (US3-S1, US3-S2); the stop leaves the imaging settings as they were and the
  mount where it was (US2-S7).

## Assumptions

- **Earlier features are in place.** Every spec in `depends_on_landed` has landed. Every
  citation here was read at e2eee793.
- **The simulated imaging solver stands for ASTAP** in the gates: it refuses a hint
  place farther than its radius and a field height more than 5 % off, as ASTAP fails
  outside its search (owner decision 4, measured on ASTAP outside the gates).
- **The finder camera solves its own frames** with the database that fits it (047);
  this spec adds nothing to the live loop.
- **Out of scope:**
  - the scope node, the scope link's protocol, the mount's motion, the solver's
    algorithm, the databases, and 032's aim point rules;
  - focusing, which is spec 051's block on the same page;
  - ring-screw words, a motorised mount for the finder camera, and an equatorial mount's directions;
  - the first night on the real rig, which is the owner's (plan.md § Outside the
    factory).
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
  implements: [FR-010, FR-011, FR-012, FR-013]
US4:
  depends_on: []
  depends_on_merged: [US3]
  implements: [FR-014, FR-015, FR-016, FR-017, FR-018]
```

Four stories, each under 60 % of 65,536 B as the probe built them (plan.md § Plan
traps 2): the place and the simulator; the run; Done, MCP and the words; the page. Each
merges after the one before (`depends_on_merged`): US2's loop calls US1's `centre_of`
and its tests point the simulator off the frame; US3 adds two keys to US2's `last` and
its tests use US2's test helpers; the page reads US3's `done`. Plain `depends_on` would
order them without putting the earlier code in the later story's base.
