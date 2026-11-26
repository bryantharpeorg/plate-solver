---
state: draft
depends_on_landed: ["013-live-finder", "015-tracking", "016-mcp", "021-chart-orientation", "031-imaging-frames-on-console", "032-imaging-solve-and-aim-point", "033-focus-reading-and-aid", "035-live-stack", "036-tracking-from-subs", "037-imaging-run", "038-track-follows-a-push", "041-imaging-run-on-the-rig", "042-imaging-run-page-and-mcp", "047-console-uses-the-database-that-fits", "049-search-finds-whole-names"]
# Drafted 2026-11-25 against `main` at e2eee793, at the owner's ask that day ("a spec to
# get the finder and the imager roughly aligned, worried about the small FOV and getting
# them aligned. we should probably just generally have some utility pages in the
# system"), from a probe of that day on the branch `probe/050` cut from e2eee793.
# Folded the same day, round 1, from a cynical review (0 blockers, 5 majors, 15 minors,
# each folded here or in plan.md) against `main` at ea3d6a9b
# (ea3d6a9b0db7fa26f81a0e90d8705ff1ffe8a7d7), which moved only under `crates/scope-node/`
# since e2eee793. Round 1 ran: the probe's fold commits 935a0699 and f351c4da with every
# scenario's test and both gates' commands passing on f351c4da, ASTAP's command-line
# solver timed on starless, trailed and failing copies of the saved frame, and each new
# or changed FR's check mutated once. Every `path:line` here was read at ea3d6a9b.
# State: draft. Two owner decisions are settled (the optic, the live scope) and ten are
# open; the scenarios and the record are written to the recommended answers. 031 and 032
# are named because the alignment holds 031's lease and solves through 032's solve and
# hint and hands each solve to 032's aim point; 033 because a scenario starts its focus
# aid; 013 and 016 for the live loop and MCP; 015 and 038 because a start is refused
# while Track is on, for 038's push check; 021, 035, 036, 037, 041, 042, 047 and 049
# because each last changed a file this spec writes (plan.md § Refinement Record). What
# rests on a run and what on reading is said where it is used (plan.md § What was run
# and what was read).
---

# Feature Specification: The Tools Page Aligns the Finder Camera

**Feature Branch**: `050-tools-page-aligns-finder-camera`

**Created**: 2026-11-25

**Status**: Draft

**Input**: The finder camera looks through the SVBONY SV165 guide scope's own objective,
40 mm at f/4 (160 mm), in front of the ZWO ASI120MM Mini (1280 × 960 at 3.75 µm): about
1.72° by 1.29° at about 4.8″ a pixel, solved with a narrow Tycho database (spec 047's
choice by range). It sits in six-point rings whose screws the owner turns by hand. The
imaging camera, the ASI071 behind the 1800 mm telescope, sees about 0.75° by 0.50°. So
the imaging camera's centre leaves the finder camera's frame at any misalignment over
about 0.86° across it (0.64° up and down), and 032's measured aim point then says only
`off_frame`. Give the console a Tools page for utilities used once a setup, and on it a
finder alignment: while the owner turns the ring screws, it shows, live, how far and
which way the imaging camera points from the finder camera, also off the finder
camera's frame, and says plainly when it is done, which is when the imaging camera's
centre holds in the middle of the finder camera's frame. Spec 051 adds focusing to the
same page. The code ties no constant to this optic: another finder optic changes
nothing here.

## Clarifications

### Session 2026-11-25 (the probe on `main` at e2eee793, folded at ea3d6a9b)

- Q: What does this trio read besides itself? → A: Nothing about the feature. `spec.md`
  holds every requirement and scenario, and `plan.md` every design decision, the
  interfaces, the shapes that existing tests pin and the alternatives rejected. A story
  that would change a decision escalates rather than re-deciding it. The probe's branch
  and logs are not in the repository; what they showed is written here.
- Q: Which finder optic is this for? → A: Settled by the owner: the SV165's own
  objective, 40 mm f/4, about 1.72° by 1.29°, as **Input** says. The trio's words, the
  README and plan.md § Outside the factory are written for it; the code is
  optic-neutral, as the console already is.
- Q: A live alignment, or one press per turn of the screws? → A: Settled by the owner:
  the live alignment of four stories, chosen over a one-press version.
- Q: What does the console do today when the two cameras point apart? → A: After an
  imaging solve, 032's `read_aim` (`crates/finder/src/imaging/relation.rs:284`)
  projects the imaging centre through the finder fix inside the exposure nearest its
  middle and returns `OffFrame` when it falls outside the finder camera's frame
  (`crates/finder/src/imaging/relation.rs:306`, `crates/finder/src/imaging/relation.rs:309`),
  as 032's FR-020 says (`specs/032-imaging-solve-and-aim-point/spec.md:2032`). The
  console then says only `off_frame`: not how far, nor which way. Run on the probe: with
  the simulator's imaging camera 6.6° and 8.5° from the finder camera's centre, every
  imaging solve handed to 032 through `Imaging::note_solve` read `off_frame`.
- Q: How does an imaging solve find its field today? → A: Through 032's hint
  (`Imaging::hint`, `crates/finder/src/imaging/solving.rs:629`): the session's pointing
  within `HINT_AGE_S`, 30 s (`crates/finder/src/imaging/solving.rs:21`), with a radius of
  at most `WIDE_RADIUS_DEG`, 3° (`crates/finder/src/imaging/solving.rs:23`), and a field
  height from the kept scale, else the focal length, else none. So an imaging camera
  more than 3° off misses 032's first hints. After three misses in a row
  (`SCALE_MISSES`, `crates/finder/src/imaging/relation.rs:25`) the hint walks down its
  sources (`crates/finder/src/imaging/relation.rs:686`) and in the end to `none`, the
  whole sky (`crates/finder/src/imaging/solving.rs:148`), which can solve; but 032 then
  answers `off_frame`, with no direction and no distance. The tool's case rests on that
  (by reading).
- Q: When does 032 take an aim reading? → A: Only when a finder fix's time, the middle
  of its exposure, lies inside the imaging exposure
  (`crates/finder/src/imaging/relation.rs:299`), else `no_live_pointing`
  (`crates/finder/src/imaging/relation.rs:302`); and only while the imaging centre's
  place moves under 60″ in the exposure (`MAX_TRAVEL_ARCSEC`,
  `crates/finder/src/imaging/relation.rs:41`), else `moving`. A short exposure from a
  slow finder camera therefore often gives `no_live_pointing`, while the alignment's
  place needs only a finder fix near the frame (owner decisions 3 and 5).
- Q: What does Track do while the owner turns a screw? → A: A turned screw moves the
  session's pointing, read through the finder camera. With Track on, 038's push check
  acts on it: at or under 5′ (`crates/finder/src/session.rs:109`) the error goes to the
  tracker, which drives the mount back under the owner's hands
  (`crates/finder/src/session.rs:1279`); over 5′ a first push marks the model's slip and
  Track follows the new place (`crates/finder/src/session.rs:1292`). 032's FR-036
  (`specs/032-imaging-solve-and-aim-point/spec.md:2110`) exists for this mechanism (by
  reading; owner decision 7).
- Q: Does an imaging solve need a field height? → A: ASTAP does. On the saved frame,
  with no field height the on-target and the 8° hints failed, after 8.7 times the
  on-target solve, and a field height 20 % high failed (run). The simulated imaging
  solver refuses a hint whose place is farther from the answer than its radius, or whose
  field height is more than 5 % off (`crates/finder/src/imaging/simulated.rs:122`), and
  solves with none; so the gates see a wrong field height but not a missing one, and a
  start with none is refused (owner decision 9).
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
  `the imaging camera is held by {holder}` while another holds it. The focus aid holds
  it as `the focus aid` (`crates/finder/src/imaging/focus/run.rs:28`), remembers the
  settings, takes frames and puts the settings back
  (`crates/finder/src/imaging/focus/run.rs:553`); the alignment follows that pattern.
  `Imaging::solve`, the Imaging page's Solve, holds no lease.
- Q: Can a test point the simulator's imaging camera off the finder camera's frame?
  → A: Not at ea3d6a9b. The simulator's imaging camera points along the eyepiece pixel's
  ray (`Optics::eyepiece_px`, default (700, 420) in a 1456 × 1088 frame 12° wide,
  `crates/simulator/src/optics.rs:28`), and `eyepiece_ray` asserts that the pixel lies
  inside the frame (`crates/simulator/src/optics.rs:52`), so a pixel off the frame
  panics there (by reading the assert). US1 relaxes it to a finite pixel; with that, the
  probe's off-frame scenarios ran.
- Q: What navigation does the page have, and what does a new page need? → A: `App.tsx`
  switches on `window.location.pathname` among `/setup`, `/imaging` and the console
  (`crates/finder/web/src/App.tsx:187`); the top bar holds a Setup chip and, while a
  scope node is paired, an Imaging chip (`crates/finder/web/src/App.tsx:178`); Setup's
  header holds `Back to console` (`crates/finder/web/src/components/SetupPage.tsx:117`).
  The server answers every path without a file extension with `index.html`
  (`crates/finder/src/api.rs:1698`), so `/tools` needs no server change (run).
- Q: What do the words "align" and "alignment" mean in the glossary today? → A: Each is
  an _Avoid_ word: under **Sync** (`GLOSSARY.md:57`), **Pointing model**
  (`GLOSSARY.md:61`), **Zero-point epoch** (`GLOSSARY.md:73`) and **Relation**
  (`GLOSSARY.md:299`), with "offset" under Relation too. This spec adds the term
  **Finder alignment** and marks the bare word in those four entries (owner decision 10);
  its key for the place is `centre`, not `offset`.
- Q: Can the alignment be used by day? → A: No. Both cameras find their field by
  plate-solving stars, and neither solves a daytime scene. A rough start by day is the
  owner's, outside this tool (plan.md § Outside the factory).

## Owner decisions

Two are settled by the owner; ten are open. Each open one keeps its options, its
evidence and its recommendation, the smallest design that works; the scenarios and the
record are written to the recommendations, and a different answer changes them before
any flip. One general rule is behind decisions 4 to 6: **the alignment adds no rule of
its own to 032.** It gives every frame the same hint, hands every solve through 032's own
door (FR-037) and shows 032's own word, so 032's module, its FRs and its tests are not
touched. Its own words (the place, Done, the refusals) are its own, and are said apart
from 032's. The candidates were measured on data where they could be (decisions 3, 4).

**Settled.**

- **The finder optic** (owner, 2026-11-25): "the finder optic is the SVBONY SV165's own
  objective, 40 mm f/4 (160 mm), in front of the ZWO ASI120MM Mini", about 1.72° by 1.29°
  at about 4.8″ a pixel, solved with a narrow Tycho database (047's choice by range); the
  tool is the one that gets the two cameras together, since 032 says only `off_frame`
  beyond about 0.86°. The 16 mm lens's arguments are dropped.
- **The scope** (owner, 2026-11-25): "the live alignment, as drafted (option A)", four
  stories chained by `depends_on_merged`, chosen over a one-press version.

**Open.**

1. **The page and its link** (FR-017, FR-018). Recommended: (a) **a page named Tools at
   `/tools`, linked from Setup's header** while a scope node is paired, under
   `Back to console`, with one block per utility; 051 adds its block below. Measured in
   the browser on the probe (Playwright's Chromium at 360, 390 and 412 px wide): with a
   Tools chip beside Setup and Imaging the top bar measured 416 px at 390 and 412, where
   `Set site` showed (no site was set), and 373 px at 360, where it did not; without
   `Set site`, the bar is about 373 px at every width (416 less `Set site`'s 39 px and its
   4 px gap, by arithmetic), so it fits at 390 and 412 and scrolls sideways at 360. The
   Setup link left the page exactly the viewport's width at all three. Instead: (b) a
   **Tools chip in the top bar** (the overflow at 360 px, and at every width until a site
   is set); (c) an **icon-only chip**, smaller but one more item in that bar; (d) **a
   section on Setup**, which leaves 051 no page and makes Setup longer; (e) **a link on
   the Imaging page**, beside the camera it uses, which 051's finder-camera focus block
   would not suit.
2. **The direction words** (FR-002, FR-019). Recommended: (a) **along the finder
   camera's own frame**: `right_deg` and `up_deg` along its rows and columns, as the
   finder camera sees it, with the chart rotation not applied, and `separation_deg` on
   the sky. The rings hold the finder camera, so its frame turns with the screws and not
   with the night or the mount, and the same words hold on any mount. Instead: (b)
   **north and east on the sky**, which turn against the rings as an alt-az mount moves;
   (c) **ring-screw words** ("turn the upper screw"), which depend on how the rings and
   the camera sit, which the console does not know; (d) **the chart's rotated frame**
   (021), which follows a setting made for the eyepiece view; (e) **alt-az words**
   (up toward the zenith, left), fixed against the rings on this dob as the frame is and
   the words of the console's push guidance, but turning against them on an equatorial
   mount; (f) **a zenith arrow** drawn in the block from the session's pointing, beside
   the frame-axis words, one more drawing to keep right.
3. **Exposure and binning** (FR-005). Recommended: (a) **500 ms unless the request names
   `exposure_ms`, the binning left as set.** With Track off a star drifts 15.04″ a second
   at the celestial equator (times cos δ elsewhere). ASTAP solves a prepared copy that
   averages each 2 × 2 Bayer cell of the ASI071's frame
   (`crates/finder/src/imaging/astap.rs:524`), 2472 × 1642 at about 1.10″ a pixel, under
   `MAX_ROWS` and so not halved again (`crates/finder/src/imaging/astap.rs:30`,
   `crates/finder/src/imaging/astap.rs:529`): a trail of 13.7 px a second in that copy
   (27 px on the sensor), so about 7 px at 500 ms, 14 px at 1000 ms and 27 px at
   2000 ms (0.55″ a sensor pixel from 4.78 µm at 1800 mm, by arithmetic). Measured with
   ASTAP on the saved 0.5° frame (about 1.0″ a pixel), smeared along its rows (times as
   shares of the on-target solve at a 10° radius): a 7 px trail solved at 0.55 times, a
   15 px trail at 1.54 times, and a 30 px trail failed, after 1.37 times. 2 × 2 binning
   sends a quarter of the bytes and leaves ASTAP's scale as it is. The finder cadence
   assumed is at least one live solve a second: the node's frame interval is 200 ms by
   default (`crates/scope-node/src/service.rs:58`), slower only with a longer fixed
   exposure set in Setup. Instead: (b) **1000 ms**, more stars and a 14 px trail, which
   solved but slower; (c) **force 2 × 2**, a fourth setting to remember and put back for
   no gain at ASTAP's scale; (d) **2000 ms**, a trail that failed.
4. **The search radius** (FR-005). Recommended: (a) **one hint for every frame: the
   paired finder fix's centre with a 10° radius and the field height 032 would give.**
   Measured with ASTAP on the saved frame (shares of the on-target solve at 10°): a
   solve's time follows the hint's offset, not its radius (on target, 1.0 at 10° and
   1.00 at 180°; 2.5° off at 3°, 1.03; 8° and 9.9° off at 10°, 1.10 and 1.13; 12° off at
   15°, 1.22; 20° off at 30°, 1.67; about 130° off at 180°, 12.2). A failing frame's time
   follows the radius: with a field height 20 % high, it failed after 1.07 times at 10°,
   1.67 at 30° and 11.0 at 180°; a starless frame failed after 0.01 times at each; a hint
   12° off at 10° failed after 1.13. A guide scope in a shoe or rings sits within a few
   degrees of the main tube by build (by reading, not measured), so 10° covers it with
   room and bounds a failing frame to about one solve. Instead: (b) **032's 3°**, which
   misses an imaging camera 3° to 10° off; (c) **30°**, a failing frame half as long
   again; (d) **a follow mode**, a narrow radius round the last solve once one solves, a
   second rule for no measured gain; (e) **the whole sky**, eleven times as long to fail.
5. **Done** (FR-010, FR-019). Recommended: (a) **the alignment's own word: the imaging
   camera's centre in the middle half of the finder camera's frame** (within a quarter of
   its width and of its height of its centre) **for `DONE_FRAMES`, two, frames in a row,
   with 032's word for the aim point shown beside it** (`measured`, `replaced`,
   `set_aside`, `moving`, `no_live_pointing`, `off_frame`). The place needs only a finder
   fix within 5 s of the frame's middle; with Track off the sky drifts at most 75″ (0.02°)
   in that gap, against a middle half of about ±0.43° by ±0.32° for this optic, so the gap
   does not move the place at the degree scale. Two frames, because the frame in flight
   while a screw turns may land in the middle by chance and the next one, taken after
   the turn, says whether it holds. 032's word then says whether the aim point took this
   frame's reading or a later imaging solve measures it, as it does today (held back
   while Track is on, its FR-036). Run: Done on the second frame in the middle while 032
   still set the frame's reading aside, and 032's `replaced` on the third; and Done with
   `no_live_pointing` from a finder camera six times slower than the exposure. Instead:
   (b) **Done only with 032's `measured` or `replaced` from that frame**, the draft's: at
   500 ms 032's anchor (`crates/finder/src/imaging/relation.rs:299`) needs a finder fix
   inside the half-second exposure, about one frame in two with a live solve a second,
   and after a move 032 sets two readings aside before it replaces them on the third
   (`REPLACE_AFTER`, `crates/finder/src/imaging/relation.rs:56`), so Done would come no
   sooner than the third frame in the middle and never from a slow finder camera; (c)
   **on the frame**, where 032 can measure, but at the edge, where a bump puts it off
   again; (d) **a fixed angle**, such as 0.25°, a different job for each optic.
6. **The `note_solve` handover** (FR-010). Recommended: (a) **every solve through
   `Imaging::note_solve`**, 032's FR-037, and `aim` read from 032's
   `relation.last.outcome` for that frame: once the centre is on the frame, 032 measures
   the aim point as from any imaging solve. Run: at the end the session's aim point was
   measured at the simulator's eyepiece pixel within 1.5 px. Instead: (b) **no
   handover**, the owner then solving on the Imaging page; (c) **count the alignment's
   solves as 032's route solves**, keeping their scale and counting their misses: a
   change to 032's rules; (d) **the alignment sets the aim point itself** from its own
   pairing, a second aim rule beside 032's.
7. **Track** (FR-011). Recommended: (a) **a start is refused while Track is on, naming
   Track, and a run ends `failed` when Track is turned on**, read before every frame;
   the README says to turn Track off while turning the finder camera's screws and back
   on after. Track off costs round stars, which the 500 ms exposure keeps short (decision
   3), and the field drifts 0.25° a minute at the equator, which changes the stars both
   cameras see and not the place between them. Instead: (b) **Track allowed, with its
   side effects written**: each turned screw moves the session's pointing; under 5′ the
   tracker drives the mount back under the owner's hands, over 5′ the model's slip is
   marked and Track follows the new place, and 032 holds every aim reading back until
   Track is off (the Clarifications above).
8. **A browser test** (FR-020). Recommended: (a) **`e2e/tools.spec.ts` in the simulate
   project**, at 412 by 915, from Setup's link to Done on the simulated console, by day
   and by night, its locators held to the block's region, Done awaited within 20 s, with
   the page no wider than the viewport: two list entries in `playwright.config.ts`
   (owner approval 6), as 035 added its stack check. It first turns Track off and keeps
   a scale through the API, since the simulated console starts with no focal length
   (decisions 7 and 9). The width is what decision 1 measured, and only a browser lays
   the page out or paints its night colours. Instead: (b) **none**, as 042 chose,
   Vitest alone driving the block.
9. **The field-height refusal** (FR-012). Recommended: (a) **a start is refused while
   the hint would carry no field height** (`Relation::height_inputs()` answers neither a
   scale nor a focal length, `crates/finder/src/imaging/relation.rs:648`), naming what to
   give: `--focal-length-mm`, or a solve on the Imaging page. Without one ASTAP failed
   each frame after 8.7 times an on-target solve (decision 4's frame), and a placed hint
   is held to 120 s (`SOLVE_LIMIT`, `crates/finder/src/imaging/astap.rs:22`), which is
   also how long a stop waits for a solve in flight (FR-007). Instead: (b) **the README
   alone**, frames that never solve until the owner reads it.
10. **The tool's name** (FR-014). Recommended: (a) **Finder alignment**, the owner's own
    word ("roughly aligned"), with the bare word marked in the four _Avoid_ entries that
    hold it: Sync's "align, alignment (alone: the finder alignment turns the finder camera
    on the tube)", Pointing model's and Zero-point epoch's "alignment (alone: see Finder
    alignment)", and Relation's "alignment (the finder alignment is the procedure, not
    what is kept)". Instead: (b) **another name that clashes with no entry**, such as
    sighting in the finder camera (the rifleman's word for bringing a sight onto the
    barrel's line; Boresight is already an entry), which renames the routes, the holder,
    the purpose label and the MCP tools.

## Owner approvals

Flipping this spec to `ready` approves the answer to every owner decision above, and the
following. Each is named in the FR that makes it. No existing test, golden, fixture or CI
file changes.

1. **A change to the simulator** (FR-001): `Optics::eyepiece_px` may lie off the finder
   camera's frame, a relaxed assert in `eyepiece_ray`. Nothing else in
   `crates/simulator` changes.
2. **New public API in `crates/finder`'s library** (FR-002, FR-004 to FR-007, FR-010):
   the module `finder::imaging::align` with `HOLDER`, `PURPOSE`, `ALIGN_EXPOSURE_MS`,
   `PAIR_S`, `SEARCH_RADIUS_DEG`, `MIDDLE_SHARE`, `DONE_FRAMES`, `AlignAsk`, `Centre`,
   `centre_of`, `AlignLast` and `AlignView`; `Imaging::align_start`,
   `Imaging::align_stop` and `Imaging::align_running`; and `ImagingView::align`. No
   existing signature changes.
3. **New routes and tools** (FR-004, FR-007, FR-013): `POST` and
   `DELETE /api/imaging/align`, and the MCP tools `get_finder_alignment` and
   `set_finder_alignment`.
4. **The glossary** (FR-014): the new term **Finder alignment**; the finder alignment
   added to **Lease**'s holders and `align` to **Purpose label**'s words; and the bare
   word marked in the _Avoid_ lines of **Sync**, **Pointing model**, **Zero-point
   epoch** and **Relation** (owner decision 10).
5. **The finder README's section, rows and MCP paragraphs** (FR-014). The pinned
   sentence `The imaging camera has sixteen tools of its own`
   (`crates/finder/README.md:88`, asserted by
   `crates/finder/tests/console/imaging_run_docs.rs:139`) stays true because the two new
   tools are in a paragraph of their own, after the imaging camera's list, and are not
   counted in it.
6. **Two list entries in `crates/finder/web/playwright.config.ts`** (FR-020):
   `"**/tools.spec.ts"` in the default project's `testIgnore` and in the simulate
   project's `testMatch`, each on a line of its own. Nothing else in that file changes.
7. **The page** (FR-017, FR-018): the Tools page at `/tools` and Setup's `Tools` link.
8. **Two refusals of a start** (FR-011, FR-012): while Track is on, and without a field
   height; and a run ended by Track turned on.

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
  `test-token` and a 500 ms frame interval (3000 ms where the scenario says so), reached
  over an in-memory duplex as `crates/finder/tests/console/imaging_relation.rs` reaches
  its node; a `Session` paired with that node and given `Site::default()`'s latitude and
  longitude as its site; and `AppState::shared` over that database with `with_session`,
  `with_clock` of the simulator's clock, `with_link`, `with_live()` (left out where the
  scenario says **without the live loop**), `with_imaging` of a folder under a
  `crate::support::Scratch` (left out where it says **with no imaging state**),
  `with_imaging_solver` of the simulated imaging solver over the simulator (left out
  where it says **without a solver**) and `with_focal_length_mm(1800.0)` (2200.0 where
  it says so, left out where it says **with no focal length**), served by
  `finder::router`. The simulator's imaging optics are 1236 px over 0.75° at 19.12 µm,
  a focal length of 1805.6 mm, so 1800 mm gives a field height 0.3 % off and 2200 mm one
  18 % off, beyond the simulated solver's 5 %. **Ready** means `GET /api/imaging` reads
  `state` `ready`, `GET /api/session` reads `pointing.source` `live`, and 5 s more have
  passed. **Turning a screw to e** is `set_optics` with `eyepiece_px` e mid-run. **The
  first measured frame after n** is `align.last` once its `outcome` is `measured` and its
  `frame_id` exceeds n, read from `GET /api/imaging` every 100 ms. The tests run under
  `#[tokio::test(start_paused = true)]`, and every state awaited comes within 120 s of
  paused time.
- **The Track rig** is the alignment rig at (700, 420) with the session's target `M42`
  and the finder camera's solves made on the paused runtime by
  `crate::support::goto::Stars` (`with_live_solver`) in place of `with_live()`, as
  `crates/finder/tests/motion/imaging_track.rs` builds its rig, in the motion test
  binary, whose tests move the mount. **Ready** there means `GET /api/imaging` reads
  `state` `ready` and the session's pointing model holds a sync; every state awaited
  comes within 300 s of paused time.
- **The MCP rig** is the alignment rig at (700, 420) served with `/mcp`, the tools called
  with `tools/call` as `crates/finder/tests/mcp/` calls them; **the bare MCP console** is
  `AppState::shared` alone, with no imaging state, served the same way.
- **The page rig** is the component or `App` rendered by Testing Library with `fetch`
  stubbed to answer the views the test writes, and `EventSource` stubbed, as the page's
  existing tests do.
- **The simulated console** is the simulate project's server of
  `crates/finder/web/playwright.config.ts:54`, the release `finder` with `--simulate`,
  whose finder camera and imaging camera are the simulator's default optics: the eyepiece
  pixel (700, 420), in the middle half of the frame. It has no focal length, so the test
  first sends `POST /api/track` with `{"on": false}`, then `POST /api/imaging/frame`,
  and once the frame is saved `POST /api/imaging/solve`, whose solve keeps a scale (032's
  FR-016), each answered 200 and the solve `solved` within 15 s.
- **Oracles.** The simulator's eyepiece pixel, which is where its imaging camera points in
  the finder camera's frame, and the pinhole oracle above; the centre frame's own
  arithmetic and the drawing's (160 wide, a 12 margin); 031's and 032's words; and the
  words of these scenarios. No expected value comes from the code under test: no test
  calls `centre_of` or reads `MIDDLE_SHARE`, `PAIR_S`, `SEARCH_RADIUS_DEG`,
  `ALIGN_EXPOSURE_MS` or `DONE_FRAMES` for an expected value.

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
   `running`, `since`, `exposure_ms` 500.0 and `frames` 0; that the frame's `centre`
   has `x` and `y` within 2 px of 700 and 420, `right_deg`, `up_deg` and
   `separation_deg` within 0.02° of the pinhole oracle, `finder_width` 1456,
   `finder_height` 1088 and `on_frame` true; and that `GET /api/imaging` then reads the
   frame's `purpose` `align`, `settings.exposure_ms` 500.0, `settings.roi`
   `{"x":0,"y":0,"width":1236,"height":822}` and `settings.save_at_node` false.
2. **Given** three alignment rigs, at (−60, 420), (−300, 420) and (1800, 100), each
   ready, **When** a committed test starts the alignment on each and reads its first
   measured frame after 0, **Then** it asserts for each that `x` and `y` are within 2 px
   of the eyepiece pixel, `right_deg`, `up_deg` and `separation_deg` within 0.02° of the
   pinhole oracle, and `on_frame` false.
3. **Given** the alignment rig at (−1100, 420), 14.8° out, beyond the 10° search, ready,
   and the alignment rig at (700, 420) with the focal length 2200 mm, ready, **When** a
   committed test starts the alignment on each and reads `align` until `last.unsolved`
   is 3, **Then** it asserts for each that `align.state` is still `running`, that
   `align.frames` is 3, that `last.outcome` is `not_solved` with the `error` exactly
   `the imaging solver found no solution`, and that `last` has no `centre`.
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
6. **Given** an alignment rig at (700, 420) with no imaging state, one without the live
   loop, its camera ready, one without a solver, ready, and one with all of them, ready,
   **When** a committed test sends `POST /api/imaging/align` to each as follows,
   **Then** it asserts these answers in order: with no imaging state, `{}` 409
   `{"error": "this console takes no imaging frames"}`; without the live loop, 409
   `{"error": "the finder camera has no live solve"}`; without a solver, 409
   `{"error": "this console has no imaging solver"}`; with all, the body `{}` sent as
   `Content-Type: text/plain` 415, `{"exposure_ms": 1e9}` 400
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
   is unchanged, that exactly two files are left under the imaging folder's `frames`, one
   `.fits` and the `.solve.json` of the same name beside it, and that
   `POST /api/imaging/frame` then answers 200.
8. **Given** the alignment rig at (700, 420), ready, the alignment started with
   `{"exposure_ms": 60000}`, **When** a committed test waits until the imaging view's
   frame has `purpose` `align` and `state` `exposing` and sends
   `DELETE /api/imaging/align`, **Then** it asserts that the answer comes within 5 s of
   paused time, before the 60 s exposure would end, and is 200 with `align.ended`
   `stopped` and `align.frames` 0, and that the imaging view's frame reads `state`
   `aborted`.

### User Story 3 - The alignment says Done, refuses Track and a missing field height, and MCP and the README carry it (Priority: P2)

As the owner, I am told Done when the imaging camera's centre has held in the middle of
the finder camera's frame, with what the aim point made of the frame beside it; the
alignment will not run with Track on or without a field height; an MCP client can run
the same alignment for me; and the README says how.

**Why this priority**: The alignment works without Done; Done is what makes it a tool
that ends, and the two refusals keep a night from going wrong.

**Independent Test**: `crates/finder/tests/console/imaging_align_aim.rs`,
`crates/finder/tests/motion/imaging_align_track.rs` and
`crates/finder/tests/mcp/imaging_align.rs`, new. The `test` gate's nextest command runs
them; during development,
`cargo nextest run -p finder --locked --test console --test motion --test mcp -E 'test(imaging_align)'`.

**Acceptance Scenarios**:

1. **Given** the alignment rig at (700, 420), ready, the alignment started with
   `{"exposure_ms": 1000}` so that a finder fix falls inside every exposure, **When** a
   committed test reads the first measured frame after 0 and the measured frame after
   it, **Then** it asserts that the first has `centre.middle` true, `done` false and
   `aim` `measured`, that the second has `done` true, and that `GET /api/session` reads
   `aim.source` `measured` with `aim.x` within 1.5 px of 700 and `aim.y` within 1.5 px
   of 420.
2. **Given** the alignment rig at (700, 420), ready, the alignment started with
   `{"exposure_ms": 1000}` and three measured frames read, **When** a committed test
   turns a screw to (−300, 420), reads measured frames until one has `aim` `off_frame`,
   then turns a screw to (900, 700) and reads measured frames, keeping the `aim` and
   `done` of each whose `centre.middle` is true, until one has `aim` `replaced`,
   **Then** it asserts that the `off_frame` frame's `done` is false, that the frames
   kept are exactly `set_aside` with `done` false, `set_aside` with `done` true and
   `replaced` with `done` true, and that `GET /api/session` reads `aim.source`
   `measured` with `aim.x` within 1.5 px of 900 and `aim.y` within 1.5 px of 700.
3. **Given** the alignment rig at (700, 420) whose node's frame interval is 3000 ms, six
   times the 500 ms exposure, ready, **When** a committed test starts the alignment with
   `{}` and reads measured frames, at most 20, until one has `aim` `no_live_pointing` and
   `done` true, **Then** it asserts that such a frame comes, and that its `centre` has
   `x` and `y` within 2 px of 700 and 420, `right_deg`, `up_deg` and `separation_deg`
   within 0.02° of the pinhole oracle, `finder_width` 1456, `finder_height` 1088 and
   `on_frame` true.
4. **Given** the alignment rig at (700, 420) with no focal length, ready, **When** a
   committed test sends `POST /api/imaging/align` with `{}`, **Then** it asserts that the
   answer is 409
   `{"error": "the imaging camera's field height is unknown: start the console with --focal-length-mm, or solve an imaging frame on the Imaging page"}`.
5. **Given** the Track rig, ready, **When** a committed test sends `POST /api/track`
   with `{"on": true}` (200), waits until `GET /api/session` reads `track.wanted` true
   and sends `POST /api/imaging/align` with `{}`, **Then** it asserts that the answer is
   409 `{"error": "Track is on; turn it off while the finder camera's screws are turned"}`.
6. **Given** the Track rig, ready, its imaging view read, the alignment started with
   `{}` (200) and `align.frames` 1 read, **When** a committed test sends
   `POST /api/track` with `{"on": true}` (200) and waits until `align.state` is `ended`,
   **Then** it asserts that `align.ended` is `failed` with `align.error` exactly
   `Track is on; turn it off while the finder camera's screws are turned`, that the
   imaging view's `settings` equal those read before the start, and that after
   `POST /api/track` with `{"on": false}` (200) `POST /api/imaging/frame` answers 200.
7. **Given** the bare MCP console and the MCP rig, **When** a committed test calls, on
   the bare console, `get_finder_alignment` with `{"x": 1}` and `set_finder_alignment`
   with `{}`, and on the MCP rig `get_finder_alignment` with `{}`, then
   `set_finder_alignment` with `{}` and with `{"on": false, "exposure_ms": 5}` and
   `get_finder_alignment` with `{"x": 1}`, then, once ready, `set_finder_alignment` with
   `{"on": true}` twice, then `get_finder_alignment` once a frame is measured, then
   `set_finder_alignment` with `{"on": false}`, and lists the tools, **Then** it asserts
   that both calls on the bare console are tool errors with exactly
   `this console takes no imaging frames`, the imaging state read before the arguments;
   that the rig's first read answers `structuredContent` `{"align": null}`; that the
   next three are tool errors with exactly `on must be true or false`,
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
`npx playwright test --project=simulate e2e/tools.spec.ts` in `crates/finder/web`.

**Acceptance Scenarios**:

1. **Given** the page rig, **When** a committed test calls `centreLine` with four
   centres and `done`, and `aimLine` with two words, **Then** it asserts these texts
   exactly: for right −6.49°, up 1.03°, 6.57° in all, off the frame,
   `The imaging camera points 6.5° left and 1.0° up of the finder camera's centre, as the finder camera sees it: 6.6° in all, off the finder camera's frame.`;
   for 4.72°, −3.77° and 6.03° on the frame,
   `The imaging camera points 4.7° right and 3.8° down of the finder camera's centre, as the finder camera sees it: 6.0° in all, on the finder camera's frame.`;
   for 1.42°, −1.29° and 1.92° in the middle,
   `The imaging camera points 1.4° right and 1.3° down of the finder camera's centre, as the finder camera sees it: 1.9° in all, in the middle of the finder camera's frame. Done once the next frame holds it there.`;
   for −0.23°, 1.03° and 1.05° in the middle with `done` true,
   `Done: the imaging camera's centre has held in the middle of the finder camera's frame. The imaging camera points 0.2° left and 1.0° up of the finder camera's centre, as the finder camera sees it: 1.1° in all.`;
   for `replaced`, `The aim point is measured from this frame (replaced).`; and for
   `no_live_pointing`,
   `The aim point is not measured from this frame (no_live_pointing); a later imaging solve measures it.`
2. **Given** the page rig with `AlignmentBlock` over a running alignment, **When** a
   committed test renders its last frame as `not_solved` frame 7 with the error
   `the imaging solver found no solution`, then as `not_solved` frame 9 with the same
   error and `unsolved` 3, and then as `no_live_solve` frame 10, **Then** it asserts
   that the status reads exactly
   `Imaging frame 7 did not solve: the imaging solver found no solution. The next frame looks again.`,
   then
   `Imaging frame 9 did not solve: the imaging solver found no solution. 3 frames in a row have not solved: the imaging camera may be more than 10° from the finder camera's centre, the sky may hold too few stars, or the field height may be wrong. The next frame looks again.`,
   and then `The finder camera had no live solve for imaging frame 10.`
3. **Given** the page rig with `AlignmentBlock` rendered first with no imaging view and
   then over a ready camera with no alignment, **When** a committed test presses
   `Start aligning`, renders a running alignment whose last frame is measured at the
   finder camera's pixel (−300, −2000), off the frame, and presses `Stop aligning`,
   **Then** it asserts that before the view the block shows `Reading the imaging camera.`
   and no button; that the first request is `POST /api/imaging/align` with the header
   `Content-Type: application/json` and the body `{}`; that the view is refreshed after
   each press; that the image named
   `The imaging camera's centre off the finder camera's frame` is shown with its circle's
   `cy` 12 and `cx` 12 + 160·(728 − 1028·544/2544)/1456, each within 10⁻⁶, where the line
   from the frame's centre to that pixel meets the frame's top edge; and that the last
   request is `DELETE /api/imaging/align`.
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
7. **Given** the simulated console in a 412 by 915 viewport, day mode, Track turned off
   and a scale kept as its definition says, **When** a committed Playwright test opens
   `/setup`, clicks `Tools`, presses `Start aligning` in the region
   `Align the finder camera`, takes a full-page screenshot after the Done check, turns
   night mode on with the switch `Night mode: red and black`, takes another, and presses
   `Stop aligning`, **Then** it asserts that the URL ends in `/tools` with the heading
   `Tools`; that within 20 s the region's status contains
   `Done: the imaging camera's centre has held in the middle`; that the region's image
   named `The imaging camera's centre on the finder camera's frame` is visible; that the
   document's `scrollWidth` is at most 412; that the day screenshot holds more than
   10,000 pixels off night red and the night one none, counted as
   `crates/finder/web/e2e/imaging.spec.ts` counts them (any green or blue, or red above
   226); and that the region's `Start aligning` shows again within 15 s of the stop.

### Edge Cases

- **Track on.** A start is refused with Track's words, and Track turned on mid-run ends
  the run `failed` before its next frame (US3-S5, US3-S6). The README says to turn Track
  off while turning the screws and back on after.
- **No field height.** A start is refused, naming `--focal-length-mm` and a solve on the
  Imaging page (US3-S4). A wrong one makes every frame `not_solved` (US2-S3).
- **Another holder of the camera** (an imaging run, the stack, the focus aid, a set of
  darks or flats) refuses the start with its name (US2-S6 for the focus aid; the others
  by 031's FR-028, read). While the alignment runs, each of them, a frame and the
  settings are refused with `the imaging camera is held by the finder alignment`
  (US2-S6 for the focus aid and the frame).
- **Solve on the Imaging page during a run.** `Imaging::solve` holds no lease, so it
  solves the alignment's latest frame with 032's own hint and rules: a miss there counts
  as 032's, and three move the field height to its next source, which the alignment's
  next frames then get. The alignment does not remove a frame while 032's solve is
  `solving` it (FR-005), and leaves that frame's files in place. Read, not run.
- **A frame in flight with no lease** refuses the start with `an exposure is in progress`,
  and a camera that is not ready with 031's own words (`the scope node is not connected`
  and the rest, `crates/finder/src/imaging/mod.rs:591`). Read, not run.
- **A frame that does not solve** (clouds, too few stars, more than 10° out, a wrong field
  height) is `not_solved` with the solver's words, counts no miss and changes no field
  height (032's FR-037); `last.unsolved` counts them in a row, and from the third the
  block names the causes (US2-S3, US4-S2).
- **A stop during a solve** waits for the solve to end; ASTAP holds a placed hint to
  120 s (owner decision 9). A stop during an exposure aborts it (US2-S8).
- **A bumped finder camera mid-run** is a turned screw: the next frames follow (US2-S4),
  Done waits for two frames in the middle, and 032 sets its readings aside until it
  replaces them (US3-S2).
- **A slow finder camera.** The place needs a finder fix within 5 s of the frame's
  middle; 032's reading needs one inside the exposure, so 032 may say
  `no_live_pointing` while the alignment says Done (US3-S3).
- **A failed exposure** ends the run `failed` with the frame's error, the settings put
  back and the camera freed (FR-007). Read, not run.
- **The console stops mid-run**: the run is in memory and is not resumed; the imaging
  settings are the alignment's until set again. Read, not run.
- **A frame picked from the top bar while on Tools** opens the console's page, as from
  Setup and Imaging (FR-017). Read, not run.

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
  `the finder alignment is already running`; then US3's two refusals (FR-011,
  FR-012); no finder fix within `HINT_AGE_S` of now, 409
  `the finder camera has no live solve`; an imaging frame exposing or receiving that no
  lease holds, 409 `an exposure is in progress`; an `exposure_ms` that is not finite or
  is outside the camera's range, 400
  `exposure_ms <N> is outside the camera's range <min>–<max> ms`; and last the lease,
  `Imaging::lease("the finder alignment")`, refused with
  `the imaging camera is held by <holder>`. It SHALL then answer 200 with the imaging
  view, its `align` running (owner approvals 2 and 3).
- **FR-005**: The run SHALL remember the imaging camera's exposure, region of interest
  and `save_at_node`; set, through the lease, the exposure to `exposure_ms` or
  `ALIGN_EXPOSURE_MS` (500), the whole sensor and `save_at_node` false, leaving the
  binning as it is; and then take one imaging frame after another under the purpose
  label `PURPOSE` (`align`) until stopped. Each saved frame SHALL be paired with the
  finder fix whose time lies nearest the middle of its exposure, no more than `PAIR_S`
  (5 s) from it; with none, the frame's `outcome` SHALL be `no_live_solve` and it SHALL
  not be solved. Otherwise it SHALL be solved with 032's `solve_frame` and a hint of
  source `pointing` at that fix's frame centre, radius `SEARCH_RADIUS_DEG` (10°), and
  the field height `Imaging::hint` gives the frame; a solve SHALL give `outcome`
  `measured` and the `centre` of `centre_of` for the solved centre, and a failed solve
  `outcome` `not_solved` with the solver's error message as `error` and `unsolved`, the
  frames in a row, this one included, that have not solved. The previous frame's
  `.fits` file and its `.solve.json` sidecar SHALL be removed once the next frame is
  saved, so that only the last is kept, unless 032's own solve (`Imaging::solve`) is
  `solving` that frame, when both stay (owner decisions 3 and 4).
- **FR-006**: The imaging view SHALL gain `align`, absent until the first start in this
  process and then the latest run's: `state` (`running` or `ended`), `since` (console
  Unix seconds), `exposure_ms`, `frames` (the frames saved), and, once there is one,
  `last` of `frame_id`, `at` (the middle of its exposure), `outcome`, and `error` with
  `unsolved`, or `centre`, as FR-005 gives; once ended, `ended` (`stopped` or `failed`)
  and, when failed, `error`. Every other key of the imaging view SHALL stay as it is.
- **FR-007**: `DELETE /api/imaging/align` (no body) SHALL stop the run: mark it stopped,
  abort the exposure in flight, wait until the run has ended (a solve in flight ends
  first), its three settings put back through the lease and the lease dropped, and
  answer 200 with the imaging view, `align.ended` `stopped`. With nothing running it
  SHALL answer 200 with the view unchanged. A frame that fails SHALL end the run the
  same way with `ended` `failed` and that frame's error as `error`.
- **FR-008**: The alignment SHALL never move the mount, change the session's pointing,
  target or settings, or write anything but its own imaging frames and their sidecars.
- **FR-009**: Nothing else SHALL change in US2: 031's lease, frames, settings and words,
  032's solve, hint and relation, the focus aid, and every existing test, US1's among
  them.

**Done, the night's refusals, MCP and the words** (US3):

- **FR-010**: Each solve SHALL be offered to the aim point through `Imaging::note_solve`
  (032's FR-037) before its line is written. `last` SHALL gain `aim`, the word of the
  imaging view's `relation.last.outcome` when `relation.last` is of this frame, and
  `done`, true exactly when this frame and the `DONE_FRAMES` − 1 before it, in a row,
  each had `centre.middle` true, with `pub const DONE_FRAMES: u64 = 2`; any other frame
  starts the count again. `done` SHALL NOT read `aim` (owner decisions 5 and 6).
- **FR-011**: The start SHALL refuse, after a run already running and before FR-012's
  refusal, while the session's `track.wanted` is true, with 409
  `Track is on; turn it off while the finder camera's screws are turned`, the session
  read with no imaging lock held; and the run SHALL read it before every frame and,
  while it is true, end as FR-007 ends a failed frame, `ended` `failed` with those words
  as `error` (owner decision 7; owner approval 8).
- **FR-012**: The start SHALL refuse, before the live-solve check, while
  `Relation::height_inputs()` answers neither a scale nor a focal length, with 409
  `the imaging camera's field height is unknown: start the console with --focal-length-mm, or solve an imaging frame on the Imaging page`
  (owner decision 9; owner approval 8).
- **FR-013**: MCP SHALL gain `get_finder_alignment`, read-only, which takes no arguments
  (else the tool error `get_finder_alignment takes no arguments`) and answers
  `{"align": ...}`, null before the first start; and `set_finder_alignment`, which
  takes `on`, a boolean (else `on must be true or false`), and with `on` true optionally
  `exposure_ms`, with `on` false nothing else
  (`set_finder_alignment with on false takes no other argument`), starts or stops the
  alignment as FR-004 and FR-007 do and answers `{"align": ...}`, a refusal's words
  being its tool error. On a console with no imaging state both SHALL answer the tool
  error `this console takes no imaging frames` before reading their arguments, as the
  imaging camera's existing tools do (`crates/finder/README.md:129`) (owner approval 3).
- **FR-014**: The finder README SHALL gain the two route rows, the MCP paragraphs and
  the section `## Finder alignment` before `## Calibration frames`, as plan.md
  § Interfaces gives them; the sentence `The imaging camera has sixteen tools of its own`
  (`crates/finder/README.md:88`) SHALL stay as it is. `GLOSSARY.md` SHALL gain
  **Finder alignment** before **Focus check**; **Lease** and **Purpose label** SHALL gain
  the finder alignment and `align`; and the _Avoid_ lines of **Sync**, **Pointing
  model**, **Zero-point epoch** and **Relation** SHALL mark the bare word, as plan.md
  § Interfaces gives them (owner decision 10; owner approvals 4 and 5).
- **FR-015**: Nothing else SHALL change in US3: 032's module, the routes and loop of US2
  but for `last`'s two keys and FR-011's and FR-012's refusals, and every existing test,
  US1's and US2's among them.

**The page** (US4):

- **FR-016**: `crates/finder/web/src/api.ts` SHALL gain `ImagingAlignCentre`,
  `ImagingAlign`, `postImagingAlign` (body `{}`), `deleteImagingAlign` and
  `ImagingView`'s optional `align`.
- **FR-017**: The page SHALL gain the Tools page at `/tools`, given the session: the
  level-one heading `Tools`, a `Back to console` link to `/`, and below them the finder
  alignment's block, over 031's `useImaging`, which the block, not the page, waits for;
  a frame picked from the top bar while on `/tools` SHALL open the console's page, as
  from `/setup` and `/imaging` (owner decision 1; owner approval 7).
- **FR-018**: Setup's header SHALL show a link `Tools` to `/tools` under
  `Back to console` while the session has a scope node, and none without one.
- **FR-019**: The block SHALL be a region named by its heading `Align the finder camera`,
  with one line on what it does and, once the imaging view is read, the button
  `Start aligning`, or `Stop aligning` while it runs; before the view it SHALL show
  `Reading the imaging camera.` and no button. Its status SHALL read
  `Taking the first imaging frame.` before the first frame; for a measured frame the
  lines of US4-S1, where the place is "The imaging camera points X° right (left) and Y°
  up (down) of the finder camera's centre, as the finder camera sees it: Z° in all", one
  decimal each, and, when `aim` is given, `aimLine`'s line beside it; for `not_solved`
  `Imaging frame N did not solve: <error>. The next frame looks again.`, naming the
  causes from `unsolved` 3 on as US4-S2 words them; and for `no_live_solve`
  `The finder camera had no live solve for imaging frame N.` A measured frame SHALL also
  draw the finder camera's frame, its middle half dashed, and the imaging camera's
  centre, held to the frame's edge along the line from the frame's centre when off it,
  as an image named `The imaging camera's centre on (off) the finder camera's frame`. An
  ended `failed` run SHALL show the alert `The finder alignment stopped: <error>.`, and a
  refused start or stop the route's words as an alert. It SHALL use DESIGN.md's existing
  classes and `currentColor`, so night mode stays red only.
- **FR-020**: `crates/finder/web/e2e/tools.spec.ts` SHALL hold US4-S7, with
  `"**/tools.spec.ts"` added to the default project's `testIgnore` and the simulate
  project's `testMatch` in `crates/finder/web/playwright.config.ts` (owner decision 8;
  owner approval 6); `crates/finder/web/dist` SHALL be rebuilt and committed. Nothing
  else on the page SHALL change.

### Key Entities

- **Finder alignment**: imaging frames taken while the observer turns the finder
  camera's ring screws, with Track off, each showing where the imaging camera's centre
  lies from the finder camera's centre, also off the finder camera's frame, until it
  holds in the middle of the frame.
- **Centre** (of the finder alignment): where an imaging solve's centre lies in the
  paired finder fix's frame: pixels, right and up along the frame, the angle in all,
  on the frame, in its middle.
- **The middle of the frame**: within a quarter of the finder camera's frame's width
  and of its height of its centre.
- **Done**: the centre in the middle for two frames in a row; the alignment's word, said
  beside 032's word for the aim point.
- **The Tools page**: the console's page of utilities used once a setup, one block each.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario has a committed test that asserts it, and the
  `lint` and `test` gates pass.
- **SC-002**: No existing test, golden, fixture, `scripts/ci/` file, `.github/` file,
  `CODEOWNERS` entry or `ergane.yaml` is edited, and `playwright.config.ts` gains only
  its two list entries. Every existing test passes unchanged (plan.md § Shapes that
  existing tests pin). The only edits to existing test files are the `mod` lines of
  `crates/finder/tests/console/main.rs`, `crates/finder/tests/motion/main.rs` and
  `crates/finder/tests/mcp/main.rs`.
- **SC-003**: The place is reported off the finder camera's frame as on it, to 0.02° of
  the simulator's truth, up to 10° out (US2-S1, US2-S2), and follows a turned screw from
  the second frame after the turn (US2-S4).
- **SC-004**: Done is said only once the centre has held in the middle for two frames in
  a row, with 032's word beside it, and the session's aim point is within 1.5 px of the
  truth once 032 has measured it (US3-S1 to US3-S3); a start is refused with Track on or
  without a field height (US3-S4, US3-S5); the stop leaves the imaging settings as they
  were and the mount where it was (US2-S7), and does not wait out an exposure (US2-S8).

## Assumptions

- **Earlier features are in place.** Every spec in `depends_on_landed` has landed. Every
  citation here was read at ea3d6a9b.
- **The simulated imaging solver stands for ASTAP** in the gates: it refuses a hint
  place farther than its radius and a field height more than 5 % off, as ASTAP fails
  outside its search (owner decisions 4 and 9, measured on ASTAP outside the gates).
- **The finder camera solves its own frames** with the database that fits it (047);
  this spec adds nothing to the live loop.
- **Out of scope:**
  - the scope node, the scope link's protocol, the mount's motion, the solver's
    algorithm, the databases, and 032's aim point rules;
  - focusing, which is spec 051's block on the same page;
  - ring-screw words, a motorised mount for the finder camera, and an equatorial
    mount's directions;
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
  implements: [FR-010, FR-011, FR-012, FR-013, FR-014, FR-015]
US4:
  depends_on: []
  depends_on_merged: [US3]
  implements: [FR-016, FR-017, FR-018, FR-019, FR-020]
```

Four stories, each under 60 % of 65,536 B as the probe built them (plan.md § Plan
traps 2): the place and the simulator; the run; Done, the night's two refusals, MCP and
the words; the page. Each merges after the one before (`depends_on_merged`): US2's loop
calls US1's `centre_of` and its tests point the simulator off the frame; US3 adds two
keys and two refusals to US2's code and its tests use US2's test helpers; the page reads
US3's `done` and `aim`, and its browser test needs US3's refusals to be met. Plain
`depends_on` would order them without putting the earlier code in the later story's
base.
