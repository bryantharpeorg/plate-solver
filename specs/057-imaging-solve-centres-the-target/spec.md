---
state: draft
depends_on_landed: ["013-live-finder", "015-tracking", "016-mcp", "019-night-legibility", "032-imaging-solve-and-aim-point", "035-live-stack", "036-tracking-from-subs", "037-imaging-run", "038-track-follows-a-push", "040-stack-waits-for-steady", "041-imaging-run-on-the-rig", "042-imaging-run-page-and-mcp", "050-tools-page-aligns-finder-camera", "051-tools-page-focuses-either-camera"]
# Drafted 2026-11-27 against 055's probe code, the branch `probe/055-r1` at 8b7212bb
# (`main` at 9c4a0a15 under it), at the owner's pick of 2026-11-27 ("Centre from
# imaging solve", from the seat's ranked list), from a probe of that day on the branch
# `probe/057` cut from 8b7212bb (plan.md § What was run and what was read). Every
# `path:line` here was read at 8b7212bb. None of the eight owner decisions is answered
# yet; the scenarios are written to each recommendation. 015, 038 and 040 are named for
# Track, its push check and its steady marks; 016 for MCP, `motion` and `axis_move`; 032
# for the imaging solve and the aim point; 035 and 036 for the stack and its steering;
# 037, 041 and 042 for the imaging run, its parts and its page; 013 for the simulator's
# finder camera and the live loop; 019 for the night rule; 050 and 051 because each last
# changed on `main` a file this spec writes (plan.md § Refinement Record). 052, 053, 054
# and 055 are drafted beside this spec and land before it, in that order; this spec is
# re-anchored on them before any flip, and they join `depends_on_landed` then.
---

# Feature Specification: An Imaging Solve Centres the Target

**Feature Branch**: `057-imaging-solve-centres-the-target`

**Created**: 2026-11-27

**Status**: Draft

**Input**: On 2026-11-27 the owner picked "Centre from imaging solve" from the seat's
ranked list after a field night. The list's item read: "Let an imaging-camera solve
centre the target on its own, or tap a star on the preview to centre it. Every M42 stack
was about 6.5′ off because the finder [camera] couldn't solve with M42 centred." 055
records the same night: every stack of M42 sat 6.2′ to 6.6′ off the imaging camera's
centre, and a tap on the imaging camera's frame to centre a point was the field driver's
own note, recommended there as a follow-up spec (055's owner decision 1 (e)). Today an
imaging solve moves the telescope only through the aim point, which needs the finder
camera's fixes; with the finder camera not solving, nothing centres the target. That
pick is the owner's only logged word on the feature.

## Clarifications

### Session 2026-11-27 (the probe on `probe/057`, cut from 8b7212bb)

- Q: What does this trio read besides itself? → A: Nothing about the feature. `spec.md`
  holds every requirement and scenario, and `plan.md` every design decision, the
  interfaces, the shapes that existing tests pin and the alternatives rejected. The
  probe's branch and logs are not in the repository; what they showed is written here.
- Q: What brings the target to the imaging camera's centre today? → A: Only the aim
  point. 041's solver part says whether a solve moved the aim point (041's FR-022), and
  only then does the run turn Track off and on to the new aim point
  (`crates/finder/src/imaging/run.rs:1251`). The aim point is measured from an imaging
  solve together with the finder camera's live pointings (032). With the finder camera
  not solving no aim point moves: the run takes four solves (`CENTRE_IDLE`,
  `crates/finder/src/imaging/run.rs:1483`) and stacks where it is, with a warning, or
  ends. On the probe, at 8b7212bb, a run with the finder camera's frames stopped stacked
  386″, 242″ and 114″ off M42 with that warning. The page has no centring; 055's arrows
  step 30″ or 4′.
- Q: What does Track hold, and what does a stack steer to? → A: Track holds a place
  (`Tracker::target`, `crates/finder/src/tracking.rs:457`) at the aim point, on the
  model's rates; with no live pointing no correction reaches it. A stack's steering hands
  the tracker the tracking error, the sky at the set point less the place Track holds
  (036's FR-003 and FR-014; `crates/finder/src/imaging/steering.rs:569`). So the place
  Track holds is what a stack brings to the imaging camera's centre.
- Q: Can a nudge's step serve? → A: Yes. 055's `Tracker::nudge`
  (`crates/finder/src/tracking.rs:393`) moves the held place and has Track's next turn
  make the step, planned as the counts plus the model's inverse at the new place on
  arrival less that at the old place now, each moved axis going 0.5° past against
  Track's own way, through a GoTo's guards, with a quiet window after it in which no
  live pointing feeds Track and no push is read (055's FR-010, FR-011). A centring is
  that step from the solve's centre to the place to centre, with the held place set to
  the place to centre.
- Q: Which of the other drafts' files does this spec write? → A: It is stacked on 055's
  code: it writes 055's `crates/finder/src/nudge.rs` (its Track branch moves into a
  shared function), `tracking.rs` (`Tracker::nudge`'s body moves into a shared step),
  `api.rs`, `lib.rs`, `mcp/tools.rs`, `crates/finder/README.md`, `GLOSSARY.md`, the
  `motion` and `mcp` test `main.rs` files, `ImagingPage.tsx`, `api.ts` and
  `playwright.config.ts`. 054 also writes `crates/finder/src/imaging/run/console.rs` and
  rewrites `ImagingPage.tsx`; 053 owns syncs and is not written (plan.md
  § Prerequisites).

## Owner decisions

None is answered yet. Each keeps its options and its evidence, and the scenarios and the
record are written to the recommendations. One general rule is behind decisions 3, 5 and
6 and the guards: **a centring is one of Track's steps, sized by the latest imaging
solve. The place Track holds becomes the place to centre (the session's target, or the
sky at a tapped pixel of the solved frame), and Track steps the telescope by what that
solve says lies between the imaging camera's centre and that place, as it makes a
nudge's step. It needs nothing of the finder camera, moves no aim point and is never a
sync, and it is refused unless Track has been steady since the solved frame began.**
So a stack's steering, which steers the place Track holds to the set point, keeps a
centred target where the centring put it; the run's centring with no new aim point is
the same call; and a push, a nudge, Track's settle and an earlier centring need no rule
of their own, as each makes Track unsteady (040's marks).

1. **Which actions** (FR-001, FR-006, FR-013). Recommended (a): **both, through one
   route and one tool**: `Centre <target>` for the session's target and a tap on the
   solved frame's picture for any point of it. Evidence (run): one centring brought M42
   from 115″ to 726″ off to 4.2″ to 17.5″ (four trials, three imaging angles); taps
   834″, 1,047″ and 679″ from the centre came to 23.2″, 13.2″ and 7.6″. Instead: (b) the
   target only (a tap is a later spec); (c) the tap only (the target is then a tap on
   it, and a target outside the frame cannot be centred).
2. **One step or repeated, and the tolerance** (FR-003, FR-009). Recommended (a): **one
   step a press outside a run; within a run, repeated to the run's tolerance**: the
   page's and MCP's centring makes one step and says how far it was, and the page says
   to take another imaging frame to check; the run repeats it, at most three tries
   (`CENTRE_TRIES`, `crates/finder/src/imaging/run.rs:1481`), to its tolerance, 1/20 of
   the imaging frame's shorter side and at least 90″ (041's FR-022). Evidence (run): in
   every trial one step and one check frame reached the run's 90″; every run with the
   finder camera's frames stopped stacked after one try, 13.8″, 14.7″ and 5.3″ off.
   Instead: (b) the route repeats until within a tolerance of its own, as Ekos (30″, up
   to 10 tries) and N.I.N.A. (1′, up to 10 slews) do (the repository's research of
   2026-10-21), with a stop and a status of its own; (c) a page tolerance such as 30″.
3. **What a centring moves** (FR-003). Recommended (a): **the place Track holds becomes
   the place to centre, and Track steps the telescope by the solve's offset** (the
   general rule). Evidence (run, start 120″ to 729″): one step 4.2″ to 17.5″ when a
   push had moved the telescope and 6.9″ to 14.1″ when the aim point was off; a stack
   of 25 subs after it held M42 at 1.4″ and 1.3″. Instead: (b) the place Track holds
   moves by the offset (a nudge of measured size): the same first step (3.7″ to 17.6″),
   but a stack's steering then brings the moved place, not M42, to the centre: 3.7″ to
   74.0″ and 7.4″ to 76.4″ over 25 subs; (c) the aim point moves by the offset: with no
   live pointing nothing moves (394″ to 403″ in 30 s, 481″ after Track off and on;
   113″ to 95″ and 91″); (d) a sync from the imaging solve (053's, decision 5).
4. **Whether the run uses an imaging solve alone** (FR-008, FR-009). Recommended (a):
   **yes, when a solve measured no new aim point**: the run asks its mount part to
   centre from the solve and counts a try; the aim point's re-centre stays the path
   when the finder camera's fixes move the aim point. Evidence (run): today the run
   stacked 386″, 242″ and 114″ off with the warning; with this, after one try, 13.8″,
   14.7″ and 5.3″. Instead: (b) the run always centres from the imaging solve, never
   turning Track off and on to a new aim point (the aim point stays wrong for the next
   GoTo); (c) the run as it is.
5. **Whether an imaging solve ever becomes a sync** (FR-005). Recommended (a): **never
   here**: a centring writes nothing to the pointing model or the aim point; 053 owns
   syncs, whatever it decides. Evidence (run): after every centring the model's syncs
   and epoch and the aim point were as before and `slip_pending` false. Instead: (b) a
   centring also records a sync from the imaging solve, so later GoTos land nearer
   (053's rule to write).
6. **Track off** (FR-002). Recommended (a): **refuse with `Track is off: turn Track on
   to centre`**. With Track off the sky moves past the still telescope about 15″ a
   second, so a solve's offset is stale within seconds, and Track turned on later GoTos
   the session's target and undoes a Track-off move (015's FR-014, 055's owner
   decision 10). Instead: (b) a direct axis move by the offset with Track off; (c) the
   centring turns Track on first (its GoTo moves the frame the solve read).
7. **What is shown** (FR-013, FR-014). Recommended (a): **a Centre block after the
   Imaging solve block, with its own picture of the solved frame**: `Centre <target>`,
   the solved frame's picture to tap, and words while Track makes the step. The tapped
   pixel must belong to the solved frame, which the Imaging frame block's picture is
   not once a newer frame is taken, and 054 rewrites that picture's lines. Instead:
   (b) a tap on the Imaging frame block's picture; (c) the button only.
8. **MCP** (FR-010). Recommended (a): **a `centre` tool with the route's arguments**,
   logged and rate-limited as every moving tool is. Instead: (b) no tool.

## Owner approvals

Flipping this spec to `ready` approves the answer to every owner decision above, and the
following. Each is named in the FR that makes it. No existing test, golden, fixture or CI
file changes.

1. **New public API in `crates/finder`'s library** (FR-001 to FR-007): the module
   `finder::centre` with `CentreAsk`, `CentreView`, `PARTS`, `routes` and `centre`, and
   `Tracker::centre`. `nudge::track_step` is `pub(crate)`. No existing signature
   changes.
2. **A new route and tool** (FR-001, FR-010): `POST /api/imaging/centre` and the MCP tool
   `centre`.
3. **A default method on the run's mount part** (FR-008): `MountSteps::centre`, against
   the trait doc's "The five traits and this struct are complete from US12: no later
   story or spec adds a method or a field, because the script implements them all"
   (`crates/finder/src/imaging/run.rs:377`) and 037's FR-042 (quoted below). The
   method has a default, so `Script` and every other implementation compile and answer
   as before; the doc's sentence names the exception.
4. **041's FR-024** (FR-009): a solve outside the tolerance that moved no aim point asks
   the mount part to centre before it counts as an idle solve.
5. **A page action in `motion` and `axis_move`** (FR-004): 016's FR-014 names the page
   actions `goto`, `second_sync` and `track`, and 055 adds `nudge`; a centring adds
   `centre`, and its step is reported in `axis_move` with `tool` `centre`.
6. **`start_motion`'s reservation** (FR-004): `centre` joins the tools refused with `the
   mount is already moving` while a direct axis move runs (055's FR-008 list).
7. **The glossary** (FR-012): **Centring** is added after **Nudge**.
8. **`crates/finder`'s README: its section, row and MCP paragraph** (FR-011). The pinned
   sentence `The imaging camera has sixteen tools of its own`
   (`crates/finder/README.md:88`) stays true: the tool is in a paragraph of its own.
9. **Two list entries in `crates/finder/web/playwright.config.ts`** (FR-016):
   `"**/solve-centre.spec.ts"` in the default project's `testIgnore` and in the simulate
   project's `testMatch`, each after `"**/nudge.spec.ts"`
   (`crates/finder/web/playwright.config.ts:46`,
   `crates/finder/web/playwright.config.ts:67`).
10. **The page** (FR-013 to FR-016): the Imaging page's Centre block, and `width` and
    `height` on the page's `ImagingFrame`, which the console's imaging view already
    serves.

### Other specs' requirements this spec changes

- 041's FR-024, which reads in part: "When the solve moved no aim point it SHALL count
  no try and ask for no re-centre, and take and solve another imaging frame, at most four
  such solves in a row." With this spec the run first asks the mount part to centre from
  the solve; a step made counts a try (FR-009). The rest of FR-024 stands.
- 037's FR-042, which reads in part: "`finder::imaging::run` SHALL offer, before any
  `Runner` exists and unchanged by every later story: the five parts' traits with every
  type they exchange, and `Steps`". `MountSteps` gains one method with a default
  (owner approval 3).
- 016's FR-014, which reads: "While a GoTo, a direct axis move or tracking runs, the
  session SHALL hold `motion`, with three fields: `by`: `page` or `mcp`; `tool`: the
  page action (`goto`, `second_sync`, `track`) or the MCP tool that started it;
  `since`: when it started." A centring holds `motion` with `tool` `centre` while its
  step is made (FR-004).
- 016's FR-022, which reads: "The session SHALL report the latest direct axis move as
  `axis_move: {tool, state, reason?}` beside 014's `goto`". A centring's step is
  reported there with `tool` `centre` (FR-004).
- 055's FR-008, which reads in part: "While a direct axis move runs, `api::start_motion`
  SHALL refuse the tools `nudge` and `track`, beside those it refuses at f6d4f5fd".
  `centre` joins them (FR-004).

No other spec's FR changes. 032's aim point and its FR-036, 036's steering and its
FR-014, 038's push check, 040's steady marks and 055's FR-010 and FR-011 stand as
written: a centring reads them and uses 055's step.

## User Scenarios & Testing *(mandatory)*

Every scenario shares these definitions.

- **The gate commands.** Every Rust scenario runs in
  `cargo nextest run --workspace --all-targets --all-features --locked`, the `test`
  gate's first command, each test in its own process. The page's Vitest scenarios run in
  `npm test`, and the browser scenarios in `CI=1 npm run e2e`, later commands of the
  same gate, in `crates/finder/web`.
- **A centring** is `POST /api/imaging/centre` whose `Host` header is localhost on port
  8080, with `Content-Type: application/json` and the body the scenario gives; **a
  foreign write** is one whose `Origin` is `http://elsewhere.example`. **PARTS** is
  exactly `centre takes nothing, to centre the target, or frame_id, x and y together`.
- **The centring rig at θ** is `crates/finder/tests/motion/centre.rs`'s rig, which US1
  writes: a `Simulator` on `crate::support::live::DB` with `Site::default()` and a
  `VirtualClock` from `crate::support::live::START`, its `SimMount` connected and
  pushed to altitude 50° and azimuth 120°, its imaging optics `ImagingOptics::default()`
  at angle θ; a `SimNode` with the token `test-token` and a 500 ms frame interval,
  served with `scope_link::memory::serve_routes_on` over an in-memory duplex; a session
  with that node, site 40° N 75° W and target M42; the console with the simulator's
  clock, the `crate::support::goto::Stars` live solver, `with_imaging` in the test's
  scratch folder, the simulated imaging solver, and imaging run steps of the console's
  own mount, camera and solver parts with `Script`'s focus and stack; read until the
  imaging view is `ready` and the model has a sync. Tokio's clock is paused.
- **Put off by a push of D at ψ**: the session's aim point set to the simulator's true
  eyepiece pixel; Track turned on with `POST /api/track` `{"on": true}` and read until
  `tracking`; 30 s; the node's star frames stopped (`stop_frames`; the imaging camera
  goes on); 2 s; the simulated mount pushed by hand D arcseconds at position angle ψ
  (`push_by(D cos ψ / 3600 / cos A, D sin ψ / 3600)`, A the tube's true altitude); 2 s.
  **Put off by the aim point by D at ψ**: the same with the aim point set D arcseconds
  from the true eyepiece pixel at ψ in the finder camera's pixels, 29.67″ a pixel (12°
  over 1,456), and no push: Track holds the wrong pixel on M42, as an aim point that
  could not be measured does.
- **A solve** is `POST /api/imaging/frame`, read every 100 ms until that frame is
  `saved`, `POST /api/imaging/solve`, and read until its solve is no longer `solving`,
  asserted `solved`. **Rests** means `GET /api/session` reads `axis_move.state` other
  than `moving` and `track.state` `tracking`, read every 100 ms, then 12 s more.
- **Off P** is the angular distance, in arcseconds, of the simulator's `imaging_truth()`
  (the imaging camera's true centre) from the place P. **M42** is its catalogue place,
  `finder::targets::find("M42")`. **The place Track holds** is
  `state.session().tracker()`'s `target()`.
- **The bare console** is `finder::router` over `AppState::shared` alone, with no link.
- **The MCP rig** is `crates/finder/tests/mcp/centre.rs`'s rig: the centring rig's
  simulator, node and session at the default imaging angle, 17°, with no run steps,
  served by `crate::support::mcp`'s `app` at `127.0.0.1` with `with_call_log` of a file
  the test reads, its tools called with `tools/call` as `crates/finder/tests/mcp/` calls
  them.
- **The page rig** is `CentreBlock` rendered by Testing Library from the recorded
  session `tracking` of `crates/finder/web/src/test/fixtures/session-track.ts` and a
  `ready` imaging view whose frame 3, 1236 × 822, is `saved` with its solve `solved`,
  with `fetch` stubbed to answer what the test writes.
- **The simulated console** is the simulate project's server of
  `crates/finder/web/playwright.config.ts`, the release `finder` with `--simulate`, at
  412 × 915 CSS pixels and a device scale of 2, with the site 40° N 75° W, the target
  M42, a minimum altitude of 15°, the finder camera's exposure 200 ms at gain 1, the
  simulated telescope at altitude 50° and azimuth 120°, the night reset, frames
  running until `model.syncs` is above 0, then Track turned on with `POST /api/track`,
  read until `tracking`, and 12 s, and the Imaging page open. **The Centre block** is
  the region named exactly `Centre`; every page locator of these scenarios is held to
  it, to the region `Imaging frame` or to the region `Imaging solve`, but the top bar's
  night switch.
- **Oracles.** The simulator's imaging truth, eyepiece and push; M42's catalogue place;
  the solve's own matrix at a pixel (032's `ImagingSolve::sky_at`); the route's and
  041's words and those of these scenarios; and DESIGN.md's night ink. No expected
  value comes from the code under test: no test reads `PARTS` from the library (each
  test file writes its own copy of the words).

### User Story 1 - An imaging solve centres the target with Track's own step (Priority: P1)

As the owner, I need the latest imaging solve alone to bring the session's target to
the imaging camera's centre with Track on, when the finder camera cannot solve, without
touching the aim point or the pointing model.

**Why this priority**: Every M42 stack of the field night was about 6.5′ off; this is
the centring.

**Independent Test**: `crates/finder/tests/motion/centre.rs`, new. The `test` gate's
nextest command runs it; during development,
`cargo nextest run -p finder --locked --test motion -E 'test(/^centre::/) | test(/^nudge_track::/)'`.

**Acceptance Scenarios**:

1. **Given** the centring rig at 17°, put off by a push of 390″ at 30°; at 100°, by a push of 120″ at 120°; at 250°, by a push of 240″ at 210°; and at 100°, put off by the aim point by 240″ at 300°, each a new rig, **When** a committed test checks that off M42 is within 15″ of D, makes a solve, posts a centring `{}`, reads `GET /api/session` at once, reads until it rests and makes another solve, **Then** it asserts that the centring answers 200 with the solve's `frame_id`, `moved` `track` and `offset_arcsec` within 5″ of off M42 before it; that the session read at once has `axis_move` exactly `{"state": "moving", "tool": "centre"}` and `motion.tool` `centre`; that once it rests off M42 is at most 25″, `axis_move` is exactly `{"state": "done", "tool": "centre"}`, `motion` is as before the centring, the place Track holds is within 0.01″ of M42, `aim`, `model.syncs` and `model.epoch` are as before and `slip_pending` is false; and that the second solve's centre is within 25″ of M42.
2. **Given** the centring rig at 17°, **When** a committed test posts a centring `{}` before any solve, makes a solve with Track off and posts `{}`, turns Track on, reads until `tracking` and posts `{}`, waits 15 s, posts `POST /api/imaging/frame` and posts `{}` at once, reads until that frame is `saved`, posts `POST /api/imaging/solve` and posts `{}` at once, reads until it is `solved`, posts `{}` twice one after another, sets the session's target to none with `PUT /api/session` `{"target": null}` and posts `{}`, **Then** it asserts that the answers are, in order, 409 with exactly `{"error": "the imaging camera has not solved a frame yet"}`, `{"error": "Track is off: turn Track on to centre"}`, `{"error": "the telescope moved after imaging frame N: take another and solve it"}` with N the first solve's frame, `{"error": "imaging frame M is not solved yet: solve it first"}` with M the new frame, and `{"error": "the imaging camera's solve is still running"}`; then 200; then 409 with exactly `{"error": "the telescope moved after imaging frame M: take another and solve it"}`; and last 409 with exactly `{"error": "no target is selected"}`.

### User Story 2 - A tap on the solved frame centres that point (Priority: P1)

As the owner, I need to name any point of the latest solved imaging frame by its pixel
and have Track bring it to the imaging camera's centre and hold it there.

**Why this priority**: The pick names the tap; a framing other than the catalogue
centre, or a star beside the target, needs it.

**Independent Test**: `crates/finder/tests/motion/centre_tap.rs`, new. The `test`
gate's nextest command runs it; during development,
`cargo nextest run -p finder --locked --test motion -E 'test(/^centre_tap::/) | test(/^centre::/)'`.

**Acceptance Scenarios**:

1. **Given** the centring rig at 17°, 100° and 250°, each put off by a push of 0″ (Track on at the true eyepiece, the star frames stopped), **When** a committed test makes a solve, takes P, the sky at pixel (300, 200), (1000, 700) and (618, 100) respectively, by the solve's own `ra_deg`, `dec_deg` and `cd_deg` with `ImagingSolve::from_cd(…, 1236, 822).sky_at`, posts a centring `{"frame_id": F, "x": X, "y": Y}` with F the solve's frame and X, Y that pixel, and reads until it rests, **Then** it asserts that the centring answers 200 with `offset_arcsec` within 5″ of off P before it; that once it rests off P is at most 30″ and the place Track holds is within 0.01″ of P; and that the session's `track.target.id` is still `NGC 1976`.
2. **Given** the bare console, and then the centring rig at 17°, **When** a committed test posts a centring `{}` to the bare console; posts to the rig a foreign write of `{}`, `{}` with `Content-Type: text/plain`, and `{"z": 1}`; makes a solve A with Track off and posts `{"frame_id": A}` and `{"frame_id": A, "x": 1.0}`; turns Track on, waits 15 s and makes a solve B; posts `{"frame_id": B, "x": 1237.0, "y": 10.0}` and `{"frame_id": A, "x": 10.0, "y": 10.0}`; sets the session's target to none; and posts `{"frame_id": B, "x": 618.0, "y": 300.0}` twice one after another, **Then** it asserts that the bare console answers 409 with exactly `{"error": "the imaging camera has not solved a frame yet"}`; that the rig answers 403, 415 and 400; that both partial asks answer 400 with exactly `{"error": PARTS}`; that the two points answer 409 with exactly `{"error": "the point is outside imaging frame B"}` and `{"error": "imaging frame A is not the latest solved frame"}`; and that the last two answer 200 and then 409 with exactly `{"error": "the telescope moved after imaging frame B: take another and solve it"}`.

### User Story 3 - The imaging run centres from an imaging solve alone, and a stack holds it (Priority: P1)

As the owner, I need an imaging run whose solve moved no aim point to centre the target
from that solve, so it stacks centred when the finder camera cannot solve, and I need
a stack after a centring to keep the target there.

**Why this priority**: The field night's stacks were the run's; a centring that a stack
then pulls away would not help.

**Independent Test**: `crates/finder/tests/motion/centre_run.rs`, new. The `test` gate's
nextest command runs it; during development,
`cargo nextest run -p finder --locked --test motion -E 'test(/^centre_run::/) | test(/^imaging_run_centring::/) | test(/^imaging_run::/)'`
and `cargo nextest run -p finder --locked --lib -E 'test(/imaging::run::/)'`.

**Acceptance Scenarios**:

1. **Given** the centring rig at 17°, once put off by a push of 390″ at 30° and once, on a new rig, by the aim point by 240″ at 210°, **When** a committed test reads `GET /api/session`, starts an imaging run with `POST /api/imaging/run` `{}` and reads `GET /api/imaging` every 100 ms until `run.state` is `stacking` or `ended`, **Then** it asserts that the start answers 200; that `run.state` is `stacking`; that `run.centring.tries` is 1 and `run.centring.offset_arcsec` at most 25; that the run has no `warnings`; that off M42 is at most 25″; and that `aim`, `model.epoch` and `model.syncs` are as before the run.
2. **Given** the centring rig at 17°, once put off by a push of 120″ at 120° with the star frames left stopped, and once, on a new rig, put off by the aim point by 120″ at 120° with the star frames started again (`start_frames`) after the centring rests, **When** a committed test makes a solve, posts a centring `{}`, reads until it rests, sets the imaging exposure with `PUT /api/imaging/settings` `{"exposure_ms": 2000}`, starts a stack with `POST /api/imaging/stack` `{}` and reads `GET /api/imaging` until `stack.stacked` is at least 15, **Then** it asserts that the centring answers 200; that `steering.source` is `subs`; and that off M42 is at most 10″.

### User Story 4 - MCP and the README carry the centring (Priority: P2)

As an MCP client, I need the page's centring as a tool with the route's answers, and as
the owner, I need the README and the glossary to say what a centring is.

**Why this priority**: The field night's work was also done over MCP; the route alone
carries the feature.

**Independent Test**: `crates/finder/tests/mcp/centre.rs`, new. The `test` gate's
nextest command runs it; during development,
`cargo nextest run -p finder --locked --test mcp -E 'test(/^centre::/)'`.

**Acceptance Scenarios**:

1. **Given** the MCP rig with the call log, **When** a committed test reads `tools/list`; calls `centre` with `{}`; waits 1.1 s; calls it with `{"frame_id": 1}` and with `{"q": 1}`; sets the aim point to the simulator's true eyepiece pixel, turns Track on with `POST /api/track` `{"on": true}`, reads until `tracking`, waits 30 s, stops the star frames, pushes the mount by hand 240″ in azimuth (`push_by(240/3600/cos 50°, 0)`), waits 2 s and makes a solve; calls `centre` with `{}`, reads `GET /api/session` at once, and calls `centre` with `{}` twice more one after another; reads until `axis_move.state` is `done` and waits 12 s, **Then** it asserts that `centre` is listed with a description containing "real telescope", an `outputSchema` of type `object` and `inputSchema` properties `frame_id` of type `["integer", "null"]` and `x` and `y` of type `["number", "null"]`; that the first call answers `isError` true with the text `the imaging camera has not solved a frame yet`; that the next two answer `isError` true with exactly PARTS; that the centring is not an error, its `structuredContent` has `moved` `track` and `offset_arcsec` within 5″ of off M42 before it; that the session read at once has `motion.by` `mcp` and `motion.tool` `centre`; that the two calls after it answer `isError` true with exactly `the telescope moved after imaging frame N: take another and solve it`, N the centring's `frame_id`, and `the rate limit of 2 moving calls a second was reached`; that off M42 is then at most 25″; and that the call log has a line containing `mcp centre` and no line containing `test-token`.
2. **Given** the committed `crates/finder/README.md` and `GLOSSARY.md`, **When** a committed test reads them, **Then** it asserts that the README still contains `The imaging camera has sixteen tools of its own`; that `Centring from an imaging solve has one tool:` lies after `The page's nudge arrows have one tool:` and before `### MCP under WSL2`, with a line starting ``- `centre` takes nothing`` after it; that the `## API` section has a row starting ``| `POST /api/imaging/centre` |``; that `## Centring from an imaging solve` lies after `## Nudging the telescope` and before `## Recording a night` and, with its whitespace made single spaces, contains `It needs no live pointing`, ``the telescope moved after imaging frame N: take another and solve it` `` and `A centring moves no aim point and is never a sync.`; and that the glossary's **Centring** entry follows **Nudge** and starts with FR-012's words and `_Avoid_: recentre`.

### User Story 5 - The Centre block on the Imaging page (Priority: P2)

As the owner at the telescope with a phone, I need a button that centres the target, a
picture of the solved frame I can tap, words while Track makes the step, and the
console's words when a centring is refused.

**Why this priority**: The pick asks for a tap on the preview; the route and the tool
are not one.

**Independent Test**: `crates/finder/web/src/components/CentreBlock.test.tsx` and
`crates/finder/web/e2e/solve-centre.spec.ts`, new. The `test` gate's `npm test` and
`CI=1 npm run e2e` run them; during development, `npm test -- CentreBlock` and
`npx playwright test --project=simulate e2e/solve-centre.spec.ts` in `crates/finder/web`.

**Acceptance Scenarios**:

1. **Given** the page rig with `fetch` answering `{"frame_id": 3, "offset_arcsec": 386.6, "moved": "track"}`, **When** a committed test clicks `Centre M42` in the Centre block and re-renders the block from the same session with `axis_move` `{"state": "moving", "tool": "centre"}` and then `{"state": "done", "tool": "centre"}`, **Then** it asserts that `fetch` was called with `/api/imaging/centre`, method `POST`, the header `Content-Type: application/json` and the body `{}`; that `onChange` was called once; that the block's status reads exactly `Centring M42: 387″ from the centre…` after the answer and after the `moving` session; and that it reads exactly `Centred M42 · Track holds it. Take another imaging frame to check.` after the `done` one.
2. **Given** the page rig with `fetch` answering `{"frame_id": 3, "offset_arcsec": 834, "moved": "track"}` and the block's picture laid out at left 10, top 20, 618 × 411 CSS pixels, **When** a committed test finds the image named `Solved imaging frame 3: tap a point to centre it` and clicks it at client (319, 122.75), **Then** it asserts that its `src` is `/api/imaging/frames/3/preview.jpg`; that `fetch` was called with `/api/imaging/centre`, method `POST`, that header and the body `{"frame_id":3,"x":618,"y":205.5}`; and that the status reads exactly `Centring the tapped point: 13.9′ from the centre…`.
3. **Given** the page rig with `fetch` answering 409 with `{"error": "Track is off: turn Track on to centre"}`, then the view with its solve of frame 2, then the session without its `target`, **When** a committed test clicks `Centre M42` and re-renders the block from each, **Then** it asserts that the block's alert reads exactly `Track is off: turn Track on to centre` and it has no status; that with the solve of frame 2 `Centre M42` is disabled and the block has no image; and that without a target it has no button and still the image.
4. **Given** `ImagingPage` rendered by Testing Library from the recorded session `tracking`, with `fetch` answering the page rig's imaging view, **When** a committed test finds the region `Imaging solve`, **Then** it asserts that the region `Centre` follows it in the document and holds the button `Centre M42`.
5. **Given** the simulated console by day, **When** a committed browser test takes an imaging frame and solves it with the `Take imaging frame` and `Solve imaging frame` buttons of their regions, reading each region's status until `Saved` and `Solved`, clicks `Centre M42`, waits 12 s, takes and solves another frame, and clicks the Centre block's picture at a quarter of its width and half its height, **Then** it asserts that the first centring's response is 200 with `moved` `track`, that the block's status comes to read exactly `Centred M42 · Track holds it. Take another imaging frame to check.` within 20 s and the session's `track.state` is `tracking`; and that the tap's request body has exactly the keys `frame_id`, `x` and `y` with `x` within 3 of 309 and `y` within 3 of 411, its response is 200, and the status comes to read exactly `Centred the tapped point · Track holds it. Take another imaging frame to check.` within 20 s.
6. **Given** the simulated console by day with a frame taken and solved as in US5-S5, **When** a committed browser test screenshots the Centre block with its picture shown, turns on `Night mode: red and black`, reads `data-theme` `night` on `html`, waits 1.5 s and screenshots the block again, **Then** it asserts with 012's off-red count (a pixel whose red is above 226 or whose green or blue is above 0) that the day screenshot has more than 1,000 off-red pixels and the night one none.

### Edge Cases

- **The finder camera solves again with the aim point off.** Track's corrections bring
  the aim point back onto the place Track holds, so a centring made with a wrong aim
  point is pulled off again: on the probe 44″, 113″, 236″ and 43″ off M42 45 s after
  the frames came back (run, not committed). While a stack's subs steer, the finder
  camera's corrections are off and the stack holds it (US3-S2). An imaging run takes
  the aim point's path whenever the finder camera's fixes move the aim point.
- **No finder camera solves at all.** Track holds the place on the model's rates alone:
  on the simulator M42 was 30″ to 52″ off 60 s after a centring (run, not committed).
  Take another frame and centre again, or let a stack's steering hold it (US3-S2).
- **A push, a nudge, Track's settle or an earlier centring after the frame began.**
  Refused with `the telescope moved after imaging frame N: take another and solve it`:
  Track's start and a centring are run (US1-S2, US2-S2); a push and a nudge are read,
  not run, as both mark Track unsteady (040's marks, 055's FR-011).
- **Track wanted but not `tracking`** (armed, in its keyhole wait or measuring
  backlash). Refused with `Track is not tracking yet` (FR-002). Read, not run.
- **A direct axis move running.** Refused with `the mount is already moving` by the
  reservation (FR-004). Read, not run.
- **Stop.** The top bar's Stop and MCP's `stop` end Track, so the step ends `halted`
  with Track's reason and the block reads `Centring stopped: ` and that reason. Read,
  not run.
- **A tap near a corner of the frame.** The step is up to half the frame's diagonal,
  through a GoTo's guards (Sun, minimum altitude, cable wind) with their words, as a
  nudge's is (055's FR-010). Read, not run.
- **Track turned off and on after a centring or a tap.** Track on GoTos the session's
  target and holds it at the aim point (015's FR-014), undoing the centring. Read, not
  run.
- **The run's three steps without reaching the tolerance.** The run then warns or ends
  as after three tries of the aim point's path, with 041's words for a measured offset
  (FR-009). Read, not run.
- **A centring during a stack or an imaging run.** The step makes Track unsteady, so
  040 drops the subs made during it, and the steering's next error is from the new
  place. Read, not run.
- **A preview frame (058, drafted beside this spec).** An unsaved preview frame is never
  solved, so it cannot be centred from or tapped; a tap names a pixel of a saved, solved
  frame only. How a preview frame meets FR-002's latest-frame check is read at the
  re-anchor (plan.md § Technical Context). Read, not run.
- **A mirrored imaging frame.** The tapped pixel's sky comes from the solve's own
  matrix, whose determinant carries the handedness (032). Read, not run: the
  simulator's imaging camera is not mirrored.

## Requirements *(mandatory)*

### Functional Requirements

**The route and Track's step** (US1):

- **FR-001**: The console SHALL serve `POST /api/imaging/centre`, a same-origin write as
  `POST /api/mount/nudge` is, taking a JSON object: `{}` centres the session's target
  (from US2, `frame_id`, `x` and `y` centre a pixel, FR-006). It SHALL answer 200 with
  `{"frame_id": N, "offset_arcsec": O, "moved": "track"}`, N the solved imaging frame
  and O the angular distance in arcseconds of the place to centre from the solve's
  centre, or 409 with a JSON `error` naming the refusal (owner decision 1).
- **FR-002**: A centring SHALL read the latest imaging solve and the latest imaging
  frame, and SHALL refuse with 409, checked in this order: no solve, or one without a
  solution's place, with `the imaging camera has not solved a frame yet`; a solve still
  running with `the imaging camera's solve is still running`; a latest frame that did
  not solve with `the imaging camera's latest frame did not solve`; a latest frame
  that is not `saved` or not the solve's frame with `imaging frame N is not solved yet:
  solve it first`, N the latest frame; for the target, no target with `no target is
  selected`; Track not wanted with `Track is off: turn Track on to centre` (owner
  decision 6); Track wanted but not `tracking` with `Track is not tracking yet`; and
  Track not steady at any moment from the solved frame's start until now
  (`Tracker::steady_between`, `crates/finder/src/tracking.rs:526`) with `the telescope
  moved after imaging frame N: take another and solve it`, N the solved frame.
- **FR-003**: Otherwise `Tracker::centre` SHALL make the place Track holds the place to
  centre (its RA and Dec; the rest of Track's target kept), clear the correction's
  memory as a nudge clears it, and have Track's next turn make the step from the
  solve's centre to that place as 055's FR-010 and FR-011 make a nudge's: the counts
  plus the model's inverse at the place to centre on arrival less that at the solve's
  centre now, each moved axis 0.5° past against Track's way, through a GoTo's guards,
  with the quiet window after it. One centring SHALL be one step (owner decisions 2
  and 3).
- **FR-004**: While the step is to be made or being made the session SHALL hold
  `motion` with `by` the caller's (`page`, `mcp` or the run's) and `tool` `centre`, and
  report `axis_move` with `tool` `centre` and `state` `moving`, then `done` once made or
  `halted` with Track's reason, `motion` reading again what it read before. A centring
  SHALL be refused with `the mount is already moving` while a direct axis move runs
  (`api::start_motion`'s list, `crates/finder/src/api.rs:844`).
- **FR-005**: A centring SHALL write nothing to the pointing model or the aim point:
  the model's syncs, its zero-point epoch and `slip_pending` and the session's `aim`
  stay as they were (owner decision 5).

**The tap** (US2):

- **FR-006**: The route SHALL also take `frame_id`, `x` and `y` together, `x` and `y`
  finite pixels of that imaging frame (right from its left edge and down from its top,
  in the frame's own width and height); any one or two of them SHALL answer 400 with
  exactly PARTS. Any other key SHALL answer 400, a body that is not JSON 415, and a
  foreign write 403.
- **FR-007**: A tap SHALL refuse with 409 `imaging frame F is not the latest solved
  frame` when F is not the solve's frame, and `the point is outside imaging frame F`
  when `x` lies outside 0 to the frame's width or `y` outside 0 to its height. The
  place to centre SHALL be the sky at that pixel by the solve's matrix
  (`ImagingSolve::from_cd` and `sky_at`, `crates/finder/src/imaging/solver.rs:150`,
  `crates/finder/src/imaging/solver.rs:210`); a tap needs no session target, and the
  session's target stays as it was.

**The run** (US3):

- **FR-008**: `MountSteps` SHALL gain `centre`, with a default that makes no step and
  answers `Ok(false)`, and the trait's doc SHALL name it as the one later method. The
  console's mount part SHALL answer `Stopped` when the run is stopped, and otherwise
  centre the session's target as the route does, by the run's `by`: `true` when a step
  was made, `false` when the centring was refused (owner approval 3).
- **FR-009**: After a solve outside the tolerance that moved no aim point, while fewer
  than three tries are made, the run SHALL ask the mount part to centre. A step made
  SHALL count a try, show `centring` with the try's words, and take and solve another
  imaging frame once the mount is steady. A step not made SHALL leave 041's FR-024 as
  it was (an idle solve). After three steps the run SHALL warn or end as after three
  tries of the aim point's path (owner decisions 2 and 4).

**MCP and the words** (US4):

- **FR-010**: The MCP endpoint SHALL offer `centre`, taking `frame_id`, `x` and `y`,
  each optional, as the route does, through the same function, rate-limited and
  written to the call log as every moving tool is, answering the route's view as
  `structuredContent` or its refusal as a tool error, and any call the route's ask
  cannot read with exactly PARTS. Its description SHALL say it moves a "real
  telescope" (owner decision 8).
- **FR-011**: `crates/finder/README.md` SHALL gain the paragraph `Centring from an
  imaging solve has one tool:` with its `centre` entry after the nudge's paragraph and
  before `### MCP under WSL2`; the `## API` row of `POST /api/imaging/centre` before
  `POST /api/goto`'s; and the section `## Centring from an imaging solve` after `##
  Nudging the telescope` and before `## Recording a night`, with the words of plan.md
  § Interfaces.
- **FR-012**: `GLOSSARY.md` SHALL define **Centring**, after **Nudge**: "Bringing a
  place, the session's target or a point tapped on the latest solved imaging frame, to
  the imaging camera's centre from that frame's imaging solve alone: the place Track
  holds becomes that place, and Track steps the telescope by what the solve says lies
  between them. It needs no live pointing, moves no aim point and is never a sync."
  with _Avoid_: recentre (the imaging run's Track off and on to a new aim point), slew
  to solve, plate-solve sync.

**The page** (US5):

- **FR-013**: The Imaging page SHALL show a Centre block, the region named `Centre`,
  after the Imaging solve block whenever the imaging view is `ready`: a quiet line
  saying what it needs, a `Centre <name>` button when the session has a target (its
  Messier name, else its id), disabled unless the latest frame is `saved` and solved
  by the latest solve, and while a centring is asked; and, when the latest frame is
  solved, that frame's picture `/api/imaging/frames/<id>/preview.jpg` named `Solved
  imaging frame <id>: tap a point to centre it` (owner decisions 1 and 7).
- **FR-014**: A click on the picture SHALL post the clicked pixel, its offset in the
  picture scaled to the frame's `width` and `height`, with the frame's id; the button
  SHALL post `{}`. The status SHALL read `Centring <what>: <offset> from the centre…`
  until a later session reads `axis_move` with `tool` `centre` other than `moving`,
  then `Centred <what> · Track holds it. Take another imaging frame to check.`, or
  `Centring stopped: <reason>` when `halted`; `<what>` is the target's name or `the
  tapped point`, and `<offset>` the answer's offset in whole arcseconds under 600″,
  else in arcminutes to one decimal. A refusal SHALL show its `error` in the block's
  alert. The block SHALL ask the page to refresh after a centring.
- **FR-015**: The Centre block SHALL follow the red-only night rule.
- **FR-016**: `playwright.config.ts` SHALL gain only owner approval 9's two entries; the
  page's `ImagingFrame` SHALL gain the optional `width` and `height` the imaging view
  serves; and `crates/finder/web/dist` SHALL be the build of its source.

### Key Entities

- **Centring**: one step of Track from the latest imaging solve; a place to centre and
  the solved frame it was read from (`GLOSSARY.md`, FR-012).
- **The place to centre**: the session's target, or the sky at a tapped pixel of the
  solved frame.
- **The place Track holds**: 038's followed place; a centring makes it the place to
  centre.
- **The solved frame**: the latest imaging frame when it is `saved` and the latest
  solve solved it.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario has a committed test that asserts it, and the
  `lint` and `test` gates pass.
- **SC-002**: No existing test, golden, fixture, `scripts/ci/` file, `.github/` file,
  `CODEOWNERS` entry or `ergane.yaml` is edited, and `playwright.config.ts` gains only
  its two list entries. Every existing test passes unchanged (plan.md § Shapes that
  existing tests pin). The only edits to existing test files are the `mod` lines of
  `crates/finder/tests/motion/main.rs` and `crates/finder/tests/mcp/main.rs`.
- **SC-003**: One centring brings M42 from 120″ to 390″ off to within 25″ of the
  imaging camera's centre at three imaging angles, after a push or with the aim point
  off (US1-S1); a tap brings its point within 30″ (US2-S1); a run with no finder camera
  solve stacks within 25″ after one try (US3-S1); and a stack after a centring keeps
  M42 within 10″ (US3-S2).
- **SC-004**: No centring is a sync or moves the aim point (US1-S1, US3-S1); every
  refusal is the route's (US1-S2, US2-S2, US4-S1), and the page shows the console's own
  words (US5-S3).

## Assumptions

- **Earlier features are in place.** Every spec in `depends_on_landed` has landed, and
  052 to 055 have landed before the flip (plan.md § Prerequisites). Every citation here
  was read at 8b7212bb.
- **The simulator stands for the mount and cameras** in the gates: its mount's
  backlash, counts and rates, its finder camera, whose frames a test stops while the
  imaging camera goes on, and its imaging camera at the angle a test sets, solved by the
  simulated imaging solver from the imaging truth.
- **The mount is alt-az.** Nothing here is equatorial; the step is Track's, and the
  future equatorial mount is not designed for.
- **Out of scope:**
  - guiding;
  - syncs from imaging solves (053's);
  - a tap on the finder camera's frame;
  - solar-system targets;
  - a centring that repeats by itself outside an imaging run (owner decision 2 (b));
  - the field night's centring by the owner (plan.md § Outside the factory).
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
  implements: [FR-006, FR-007]
US3:
  depends_on: []
  depends_on_merged: [US2]
  implements: [FR-008, FR-009]
US4:
  depends_on: []
  depends_on_merged: [US3]
  implements: [FR-010, FR-011, FR-012]
US5:
  depends_on: []
  depends_on_merged: [US4]
  implements: [FR-013, FR-014, FR-015, FR-016]
```

Five stories, each under 60 % of 65,536 B (39,321 B) at the factory's worst overrun
seen, 1.35 times the probe's measure (plan.md § Plan traps 1): the target's centring and
Track's step; the tap; the run and the stack; MCP, the README and the glossary; the
page. The cut: the probe's first story held the target and the tap, 35,224 B, 47,552 B
at 1.35, so the tap and the route's ask are a story of their own, and the stack's
scenario sits with the run. Each merges after the one before (`depends_on_merged`): US2
adds the tap to US1's `centre` module and uses US1's test rig; US3's mount part calls
`centre` and its tests use US1's rig; US4's tool reads the ask US2 completes; the page
posts the tap and its browser test needs the route. Plain `depends_on` would order them
without putting the earlier code in the later story's base.
