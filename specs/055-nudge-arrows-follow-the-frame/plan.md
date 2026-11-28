# Implementation Plan: The Nudge Arrows Follow the Frame

**Branch**: `055-nudge-arrows-follow-the-frame` | **Date**: 2026-11-27 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/055-nudge-arrows-follow-the-frame/spec.md`

## Summary

A nudge moves the telescope one step on the sky, fine 30″ or coarse 4′, up, right,
down or left in one of three views: the mount's own, the finder camera's live views as
the page turns them, or the imaging camera's frame from its last solved imaging solve. A
new module, `crates/finder/src/nudge.rs`, turns a press into an offset in the mount's
view, `[up, right]` arcseconds, and hands it on. With Track off it is a direct axis move
of both axes: a new `AxisMove::Offset` is checked by `Planner::direct` into a new
`AxisMove::Through`, two waypoints, the second the end and the first 0.5° below it on
every axis that moves, through every guard MCP's direct moves pass, and `mcp::guard`
drives the axes through them. `start_motion`'s reservation gains `nudge` and `track`.
With Track on, `Tracker::nudge` moves the place Track holds and the rate loop's next
turn makes the step, each moved axis going 0.5° past against Track's own way and coming
back; while the step is made and for 1 s plus Track's settle after, the push check and
the loop read no pointing. A nudge writes nothing to the model. MCP gains a `nudge` tool
through the same function; the README and the glossary gain the words; the guidance
block and the Imaging page gain a Nudge block. Five stories: the mount's view, the
approach and the guards; the camera views; Track's step; MCP and the words; the page.

## Technical Context

**Language/Version**: Rust, the factory's toolchain; TypeScript and React for the page
(`crates/finder/web`).

**Primary Dependencies**: none new. `axum` routes, `rmcp` tools, the console's own
`goto`, `mcp::guard`, `tracking` and `sky` modules.

**Testing**: `cargo nextest` (the `motion` and `mcp` test binaries and the `finder`
library's own tests), Vitest and Playwright, all under the `test` gate.

**The step on an alt-az mount.** A camera fixed to the tube of an alt-az mount keeps its
orientation to altitude and azimuth: the sky turns in the frame (field rotation), the
axes do not. So the frame's up and right on the axes are read once and hold for every
later press until the camera is turned in its holder. In the mount's view the press's
unit is `[1, 0]`, `[0, 1]`, `[-1, 0]` or `[0, -1]` as `[up, right]`. In the finder
camera's view the page draws the live frame turned counter-clockwise by the chart
rotation R (021), so the page's φ is φ + R clockwise of the frame's up, and the live
pointing's `up_deg` u is the zenith's angle clockwise of the frame's up, so the press is
φ + R − u clockwise of the zenith: `[cos a, sin a]`, a = φ + R − u. In the imaging
camera's view the solve's `cd_deg` gives degrees east (row 0) and north (row 1) for one
pixel right (column 0) and one pixel down (column 1); its determinant's sign is the
frame's handedness; the frame's φ is the pixel direction `[sin φ, −cos φ]`; east and
north become across and up with the parallactic angle q at the solve's place and the
frame's mid-exposure time (q from `sky::alt_az` of the place and of a place 10⁻⁵° north
of it), across = −cos q·east + sin q·north and up = sin q·east + cos q·north, as
`tracking.rs`'s `axis_frame` and `axis_error` turn them
(`crates/finder/src/tracking.rs:733`, `crates/finder/src/tracking.rs:752`), normalised.
No equatorial direction is used.

**One movement rule.** Every moved axis ends its step moving the way it is next driven.
With Track off that is a GoTo's way: `Planner::direct` (`crates/finder/src/goto.rs:222`)
predicts the altitude and azimuth at the axes' counts, adds the step (azimuth divided by
the altitude's cosine), and asks the model's inverse at both places; the end is the
counts plus the difference, so the model's own errors cancel to first order; every axis
whose counts change first goes to 0.5° (014's `APPROACH`,
`crates/finder/src/goto.rs:23`, 2,250 counts at 1,620,000 a turn) below its end, as a
GoTo does (`crates/finder/src/goto.rs:377`). With Track on it is Track's way: the
waypoint is 0.5° past the end against the way Track drives that axis
(`Control::direction`), so the gears end on Track's flank and Track's next command
takes up nothing. GoTo's approach from below on a setting target, where Track drives
altitude down, was measured to leave 529″ (§ What was run).

**Track on.** `Tracker::nudge` turns `[up, right]` into east and north at the moment
(`axis_frame`'s q, the inverse of `axis_error`'s turn), checks the step's path through a
`Planner` (the guards of FR-006) from the axes' counts, takes the session's model (as a
followed push hands it over), moves the held place's RA and Dec, clears the
correction's memory as `follow` does (038's FR-004) and marks the step pending. The
rate loop's next turn stops both axes, waits for rest, plans again from the counts it
reads, and drives both axes to the two waypoints with `goto_axis`, telling the model's
motion flanks of each move (`motion.moving`) and holding the frames out of the fit
(`motion.crossing`), as the keyhole's slew does; then `quiet_until` is now + 1 s +
`SETTLE` (`crates/finder/src/tracking.rs:31`), the steady mark settles with the cause
`nudged`, and the loop goes on from the counts with its forecasts reset. Until
`quiet_until`, `pointing` returns before the loop and `checks_push` is false
(`crates/finder/src/tracking.rs:331`), so 038's check (`crates/finder/src/session.rs:1281`)
sees none of the step's frames. When the step ends, `motion` reads again what it read
before the press (FR-010). The step is made from the counts, so it carries any Track
error the telescope had at the press, and Track's own loop corrects that error after
the window as it corrects any error at f6d4f5fd, swinging past first when the error is
large (spec.md § Clarifications, probe round 1).

**Performance Goals**: a press answers at once (the route starts the move and returns).
With Track on the telescope is at the new place within the 10 s US3-S1 allows.

**Constraints**: no new dependency; no change to the model, the live solve, GoTo's
correction solve or `session.rs` (053's); `goto.rs` gains only the two variants, two
arms, one helper and one refusal.

**Scale/Scope**: about 2,500 added lines across five stories (the probe's, without
`dist` and the probe-only tests), more than half of them tests.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Detection, the plate solver and the live solve are untouched; the finder camera's view reads the live pointing's `up_deg` as the live loop gives it, and the imaging camera's view the imaging solve's `cd_deg` as 032 gives it. |
| II. Never weaken a gate | No existing test, golden, fixture, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS` or `ergane.yaml` is edited. `playwright.config.ts` gains two list entries, an owner approval as 050's and 051's were. The only edits to existing test files are the `mod` lines of `crates/finder/tests/motion/main.rs` and `crates/finder/tests/mcp/main.rs` (SC-002). Every existing test passed unchanged on the probe's tip (run, § What was run). |
| III. Tests first, independent expectations | The oracles are the simulator's true tube direction, counts, eyepiece and imaging truth and its own horizon transform, the steps' sizes and angles, the mount's counts a turn, and the words of 014, 016, 038 and the scenarios. No test reads `FINE_ARCSEC`, `COARSE_ARCSEC` or `PUSH_QUIET_S`; the unit tests call `offset` for the value under test, never for an expected one. Time comes from a paused tokio clock and the simulator's virtual clock. Every story's tests failed on the story before it (run, § What was run). |
| IV. Read the critical paths twice | None is touched: no `Cargo.toml` or `Cargo.lock` change, no `unsafe`, nothing under `proto/`, `crates/math-core`, `crates/pattern-database` or `crates/database-generation`, no `build.rs` or `.cargo/`. The mount's motion is a safety path: every press goes through the guards MCP's direct moves already pass (FR-006), and Track's step through the same `Planner` path check (FR-010). |
| V. Proven open source first, with honest licensing | Looked for, in local sources only (no download): the vendored `reference-solutions/` (cedar-detect, cedar-solve: no mount control) and the console's own MCP `move_axis`. **INDI's telescope motion** (`TELESCOPE_MOTION_NS` and `_WE`, held while a button is held, with a slew rate, and `TELESCOPE_TIMED_GUIDE_NS`, timed pulses) and the **KStars/Ekos mount panel** move the axes by direction and rate while held or for a time; INDI's `TELESCOPE_REVERSE_MOTION` shows users expect either arrow sense (owner decision 9); none reads a camera's frame, and the scope node speaks its own protocol to the mount (045's INDI path is the camera's). **N.I.N.A.'s and Ekos's "centre on a solved frame"** move to a solved place, not by a step (owner decision 1 (e), a follow-up). Nothing found reads a frame's own up and right for a press, so the step is written fresh, on the console's existing guards and tracker. |

## Project Structure

### Documentation (this feature)

```text
specs/055-nudge-arrows-follow-the-frame/
├── spec.md
├── plan.md
└── tasks.md
```

### Source Code (repository root)

```text
crates/finder/src/nudge.rs                  # new (US1, mount view); US2 the camera views; US3 Track's branch
crates/finder/src/goto.rs                   # US1: AxisMove::Offset, ::Through, two arms, `through`, the zenith refusal
crates/finder/src/mcp/guard.rs              # US1: `start_by`, driving `Through`
crates/finder/src/api.rs                    # US1: the reservation's two names, one `merge`; US2: `nudge_kept`; US3: `motion` pub(crate)
crates/finder/src/lib.rs                    # US1: `pub mod nudge;`
crates/finder/src/tracking.rs               # US3: `Tracker::nudge`, the loop's step, the quiet window
crates/finder/src/mcp/tools.rs              # US4: the `nudge` tool
crates/finder/README.md, GLOSSARY.md        # US4
crates/finder/tests/motion/nudge.rs         # US1, new (its helpers `pub(super)` for US2 and US3)
crates/finder/tests/motion/nudge_views.rs   # US2, new
crates/finder/tests/motion/nudge_track.rs   # US3, new
crates/finder/tests/mcp/nudge.rs            # US4, new
crates/finder/web/src/api.ts                # US5: `postNudge`, `axis_move`
crates/finder/web/src/useSession.ts         # US5: copies `axis_move`
crates/finder/web/src/App.tsx               # US5: hands `ImagingPage` the session
crates/finder/web/src/components/NudgeBlock.tsx, NudgeBlock.test.tsx  # US5, new
crates/finder/web/src/components/GuidanceBlock.tsx                    # US5: one import, one line
crates/finder/web/src/components/ImagingPage.tsx                      # US5: two props, one line (054's page)
crates/finder/web/e2e/nudge.spec.ts         # US5, new
crates/finder/web/playwright.config.ts      # US5: two entries
crates/finder/web/dist                      # US5: the build
```

**053's files.** Of `crates/finder/src/{model,goto,live,session}.rs`, only `goto.rs` is
written, and only by adding: two `AxisMove` variants after `Rate`, two arms in
`Planner::direct`'s `match`, and the helper `through` after `direct`. No line of GoTo's
planning, correction solve or sync is changed. `session.rs` is read only
(`Session::tracker`, `crates/finder/src/session.rs:931`; `PushView`,
`crates/finder/src/session.rs:136`; `PUSH_REOPEN_S`, `crates/finder/src/session.rs:113`).

**The other drafts' shared files.** 052, 053 and 054 also write
`crates/finder/README.md`, `GLOSSARY.md` and the `console` and `motion` test `main.rs`
files; 053 changes `goto.rs`; 054 rewrites `ImagingPage.tsx`. They land in the order 052,
053, 054, 055, and the seat re-anchors this trio on whatever has landed before its flip
(§ Prerequisites). This spec adds only: one README paragraph, row and section, two
glossary entries, `mod` lines, and on the Imaging page two props and one line after its
imaging frame.

## Interfaces

### `crates/finder/src/nudge.rs` (US1 new; US2 and US3 add)

```rust
//! Nudges: one press of an arrow moves the telescope one step on the sky, the
//! way the arrow points in the view the observer picked (055). With Track off it
//! is a direct axis move through every guard a GoTo passes; with Track on the
//! place Track holds moves by the same step and Track makes the step. Either
//! way each axis the step moves goes 0.5° past its end and comes back.

pub const FINE_ARCSEC: f64 = 30.0;
pub const COARSE_ARCSEC: f64 = 240.0;
/// Seconds after Track follows a push by hand in which a press is refused:
/// 038's `PUSH_REOPEN_S` in `session.rs`, copied here; change both together. (US3)
pub const PUSH_QUIET_S: f64 = 10.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction { Up, Right, Down, Left }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Step { Fine, Coarse }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum View { Mount, FinderCamera /* US2 */, ImagingCamera /* US2 */ }
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NudgeAsk { pub direction: Direction, pub step: Step, pub view: View }
#[derive(Debug, Serialize)]
pub struct NudgeView { pub up_arcsec: f64, pub right_arcsec: f64, pub moved: &'static str }

/// The imaging camera's frame on the axes, kept from the latest solved imaging
/// solve until another solves: `[across, up]` rows, pixel x and y columns. (US2)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Kept { pub frame_id: u64, pub axes: [[f64; 2]; 2] }

/// `[up, right]` arcseconds in the mount's view. (US1: `offset(ask)`; US2 adds the
/// session, the imaging view and the kept frame.)
pub fn offset(ask: &NudgeAsk, session: &SessionView, imaging: Option<&ImagingView>,
    kept: &Mutex<Option<Kept>>) -> Result<[f64; 2], String>;
/// `POST /api/mount/nudge`: `SessionWrite`, `sim_json`, then `nudge(state, "page", ..)`.
pub fn routes() -> Router<AppState>;
/// One press, from the page (`by` "page") or MCP ("mcp").
pub async fn nudge(state: &AppState, by: &'static str, ask: &NudgeAsk)
    -> Result<NudgeView, ApiError>;
```

The mount view's units are exact, so the answer is exactly the step and 0. `nudge`
reads the session snapshot and, from US2, the imaging view; computes the offset (its
refusals first). With Track off, or before US3, it calls
`mcp::guard::start_by(state, by, "nudge", AxisMove::Offset { .. })` and answers `moved`
`axes`. From US3, with `track.wanted`: while `track.push` is `Moving`, or `Following`
less than `PUSH_QUIET_S` after its `at`, it refuses with FR-012's words; else
`api::start_motion(state, by, "nudge", ..)` runs `Session::tracker()` (none: `Track is
not tracking yet`), `Tracker::nudge(up, right, session.model(now))`, sets `axis_move`
to `{tool: "nudge", state: "moving"}` and spawns a watcher that every `goto::TICK` reads
the tracker: off ends `halted` with its `ended_reason`, `!stepping()` ends `done`; it
answers `moved` `track`. Before `start_motion` the Track branch reads the `motion`
record (`AppState::motion`, `crates/finder/src/api.rs:132`, made `pub(crate)` in US3),
and the watcher, ending the step, sets `axis_move` and puts that record back under
`motion`'s lock, so `motion` names the nudge only while its step is made (FR-010).

US2's imaging arm: of the latest solve, one reading `solved` replaces `kept`, when
nothing is kept or the kept `frame_id` is not the solve's, with
`frame_axes(solve, site, time)`, `time` the held frame's `start + exposure_ms / 2000`
when the imaging view's frame has the solve's `frame_id`, else `requested_at`; no site:
`a site is needed`. With nothing kept, FR-005's three words by the latest solve's state.
The press's pixel direction `[sin φ, −cos φ]` through `axes` gives `[across, up]`,
normalised (not normal: `the imaging camera has not solved a frame yet`), answered as
`[up, across]`. `AppState` gains `pub(crate) nudge_kept: Arc<Mutex<Option<Kept>>>`
(`crates/finder/src/api.rs:102`), `None` at start.

### `crates/finder/src/goto.rs` (US1)

```rust
pub enum AxisMove {
    // … `Position` and `Rate` as at f6d4f5fd …
    /// Move the telescope by an offset on the sky from where it points: up in
    /// altitude, and across the sky the way azimuth grows, in arcseconds (055).
    /// Checked into `Through`.
    Offset { up_arcsec: f64, right_arcsec: f64 },
    /// Move both axes to `first`, then to `last`: an offset as checked, ending
    /// on each axis the way a GoTo's approach ends (055).
    Through { first: [i32; 2], last: [i32; 2] },
}
```

`Planner::direct` gains the arm `Offset`: refuse a non-finite offset with `the offset
must be finite`; `ready()`; predict (alt, az) at the current counts; at or above 88°,
refuse with FR-009's words; to = (alt + up/3600, az + right/3600/cos alt); the model's
`inverse` at both, each with `ephemeris::apparent_from_true` of its altitude (none: `the
night has no sync yet`); `last = current + round(there − here)`; for each axis whose
`last` differs from `current`, `first = last − round(APPROACH · counts / 360)`, else
`first = last`; `through(first, last)?`; answer `Through { first, last }`. The arm
`Through` re-checks with `through`. `pub(crate) fn through(&self, first, last) ->
Result<f64, String>`: `ready()`; a floor of the lower of the predicted altitude now and
`min_alt`, less 10⁻⁶; `last` below it: `the position is below the minimum altitude`;
`first` below it: `approach would go below the minimum altitude`; `safe_path` of
`[last]` or `[first, last]` (`crates/finder/src/goto.rs:433`), none: 014's path words;
answers the path's duration.

### `crates/finder/src/mcp/guard.rs` and `api.rs` (US1)

`start` becomes `start_by(state, "mcp", tool, request)`; `pub(crate) async fn
start_by(state, by, tool, request)` calls `api::start_motion(state, by, tool, ..)`.
`begin` sends `goto_axis` of both axes to `first` for a `Through`; the watcher `path`,
when neither axis is moving, ends the move if `first == last` and otherwise sends both
axes to `last` and goes on with `Through { first: last, last }`. Stop and the guards'
halts are the watcher's as at f6d4f5fd. In `api::start_motion`
(`crates/finder/src/api.rs:835`) the reserved list becomes `"goto" | "second_sync" |
"goto_target" | "goto_axis_position" | "move_axis" | "nudge" | "track"`, and
`routes = routes.merge(crate::nudge::routes());` follows the `finder_focus` merge
(`crates/finder/src/api.rs:661`).

### `crates/finder/src/tracking.rs` (US3)

```rust
impl Tracker {
    /// Move the place held `up` arcseconds in altitude and `right` across the
    /// sky the way azimuth grows, and have the rate loop's next turn make the
    /// step (055) with the session's `model`. A press the GoTo guards refuse
    /// changes nothing; so does one while not `tracking`, measuring backlash
    /// or with an earlier step still to make.
    pub async fn nudge(&self, up: f64, right: f64, model: Model) -> Result<(), String>;
    /// Whether a nudge's slew is still to be made or under way (055).
    pub fn stepping(&self) -> bool;
}
```

`Control` (`crates/finder/src/tracking.rs:111`) gains `axes: Option<[AxisInfo; 2]>`
(set by `start`), `stepping: Option<Target>` (the place moved from) and `quiet_until:
f64`. `may_nudge`: not `tracking` or a measurement running, `Track is not tracking yet`;
`stepping` some, `the mount is already moving`. `nudge`: under the lock `may_nudge`, the
target, axes and `direction`; `Shared::sky_step` gives east and north; both axes'
counts read with a 1 s timeout (`mount did not answer`); the moved place `moved(target,
east, north)`; `step_path` checks it; `set_model(model)`; under the lock `may_nudge`
again, the held target moved, `followed` true, the memory cleared as `follow` clears it
(proportional, previous pointing, samples, subs, pending, lapsed, held; `fresh_after`
now), `stepping = Some(from)`, `mark`. `step_path(model, axes, current, direction, from,
to, now)`: two passes so the end is at arrival: `last = current + round(inverse(to, now
+ duration) − inverse(from, now))`, azimuth's difference wrapped to the nearer way
round; for a moved axis `first = last ∓ round(0.5 ·
counts / 360)`, minus while `direction ≥ 0`, plus while below; `duration =
Planner { .. }.through(first, last)?`. The rate loop reads `stepping` with `followed`
and `target` at the top of each turn (`crates/finder/src/tracking.rs:1376`) and, when
some, runs `step`: `stop_all`, `at_rest`, `step_path` again, `motion.crossing(i, true)`,
for each waypoint `motion.moving(i, delta)` and `goto_axis` then `at_rest`,
`crossing(i, false)`; under the lock `stepping = None`, `quiet_until = now + 1 +
SETTLE`, settling with the cause `nudged`, `mark`; then its forecasts, `prepared` and
`refresh_at` reset and `second.reset()`, the rate loop's one-second interval restarted as
after the keyhole's slew (`crates/finder/src/tracking.rs:1404`). `Shared::pointing`
(`crates/finder/src/tracking.rs:579`) returns after the wind's observation while
`stepping` is some or before `quiet_until`; `checks_push` adds `&& stepping.is_none()
&& now >= quiet_until`; `pause` reports `nudged` while stepping; the steady mark needs
`stepping` none.

### `crates/finder/src/mcp/tools.rs` (US4)

`NudgeArguments` (three `String` fields, documented `up, right, down or left`, `fine or
coarse`, `mount, finder_camera or imaging_camera`), `#[expect(dead_code, reason =
"schema only; nudge parses its arguments as the route does")]`, after
`FinderFocusArguments`. The tool `nudge`, after `move_axis`, with the description:
"Move a real telescope one step on the sky, as the page's nudge arrows do: direction
up, right, down or left; step fine (30 arcseconds) or coarse (4 arcminutes); view mount
(up is higher altitude, right a clockwise turn), finder_camera (the finder camera's live
views as the page turns them) or imaging_camera (the imaging camera's frame, from its
latest imaging solve). With Track off the axes move through all console guards; with
Track on the place Track holds moves by the step and Track makes it. Answers the step as
up_arcsec and right_arcsec, and moved: axes or track." It parses its arguments into
`NudgeAsk` (any failure: FR-014's words), then `limit_move()`, then
`nudge::nudge(&state, "mcp", &ask)`, `log_call("nudge", ..)` and `tool_result`.

### The README (`crates/finder/README.md`, US4)

After the finder camera's focus aid's two entries and before `### MCP under WSL2`
(`crates/finder/README.md:159`):

```markdown
The page's nudge arrows have one tool:

- `nudge` takes `direction` (`up`, `right`, `down` or `left`), `step` (`fine` or
  `coarse`) and `view` (`mount`, `finder_camera` or `imaging_camera`) and nothing
  else, as `POST /api/mount/nudge` does. It moves a real telescope one step, is
  rate-limited and logged, and answers `up_arcsec`, `right_arcsec` and `moved`
  (`axes` or `track`), or the route's refusal as a tool error.
```

After the `POST /api/goto` row (`crates/finder/README.md:318`):

```markdown
| `POST /api/mount/nudge` | JSON `{"direction":D,"step":S,"view":V}`: D is `up`, `right`, `down` or `left`, S `fine` (30″) or `coarse` (4′), V `mount`, `finder_camera` or `imaging_camera`. Move the telescope one step on the sky that way in that view; answers 200 with `up_arcsec` and `right_arcsec` (the step in the mount's view) and `moved` (`axes`, or `track` while Track is on), or 400, 409 or 415 with a JSON `error`. Same-origin writes only. |
```

Before `## Recording a night` (`crates/finder/README.md:665`), with two blank lines
before each heading as there:

```markdown
## Nudging the telescope

A nudge moves the telescope one step on the sky: fine is 30″, coarse is 4′. The
arrows follow one of three views. `mount`: up is higher altitude and right a
clockwise turn. `finder_camera`: up, right, down and left as the page draws the
finder camera's live views, turned by the chart rotation; it needs a live
pointing. `imaging_camera`: as the imaging camera's frame shows them, from the
latest imaging solve that solved; it needs one. Without them the press is
refused with `the finder camera has no live pointing yet` or `the imaging
camera has not solved a frame yet`.

Each axis a step moves goes 0.5° past its end and comes back, so its gears end
on a known side. With Track off a nudge is a direct axis move of both axes: it
needs a sync and passes every guard a GoTo passes, and each moved axis comes up
from below as a GoTo's does. From 88° altitude up a nudge is refused: there a
step's turn of azimuth no longer keeps its size. With Track on the place Track
holds moves by the step and Track makes it, each moved axis coming back the way
Track drives it; a press is refused with `a push by hand is still settling:
press again in a few seconds` while a push by hand is still being read. A nudge
is never read as a push and never a sync: the next solve syncs as usual.
Turning Track on after nudges made with Track off goes back to the target, as
Track on always does: centre with Track on.
```

### The glossary (`GLOSSARY.md`, US4)

**Direct axis move** (`GLOSSARY.md:273`) takes FR-016's words; **Nudge** follows it,
with `_Avoid_: jog, slew, manual move`.

### The page (`crates/finder/web/src/`, US5)

`api.ts`: `ObservingSession` gains `axis_move?: { state: string; tool: string; reason?:
string }` after `imaging_camera` (`crates/finder/web/src/api.ts:862`); before
`postRecord` (`crates/finder/web/src/api.ts:956`):

```ts
export type NudgeDirection = "up" | "right" | "down" | "left";
export type NudgeStep = "fine" | "coarse";
export type NudgeView = "mount" | "finder_camera" | "imaging_camera";
export interface Nudged { up_arcsec: number; right_arcsec: number; moved: "axes" | "track"; }
export async function postNudge(direction: NudgeDirection, step: NudgeStep, view: NudgeView): Promise<Nudged>;
```

posting `{direction, step, view}` with `Content-Type: application/json` through the
module's `json` reader, so a refusal throws its `error`. `useSession.ts` copies
`axis_move` after `motion` (`crates/finder/web/src/useSession.ts:46`).
`NudgeBlock.tsx` exports `NudgeBlock({ session, onChange, first = "mount" })`: nothing
without a connected mount; a `<section aria-label="Nudge">` with a labelled `<select
aria-label="Arrows follow">` styled as `FocusTool.tsx`'s camera box (its imaging option
only with `session.imaging_camera`; `first` chosen when offered), a `role="group"`
`Step` of two `.button`s with `aria-pressed`, a three-column grid of four `.button`s at
least 44 CSS pixels (`min-h-11 min-w-11`) labelled `Nudge up`, `Nudge right`, `Nudge
down` and `Nudge left`, drawn `↑`, `→`, `↓` and `←` in a cross (up; left and right;
down), disabled while a press is pending; a `<p role="status">` and a `<p
role="alert">` as `TrackControl.tsx` has them. A press
keeps `{way, track, over: session}`; the status is FR-017's one rule: `Nudging <way>…`
while the session is still `over` or its `axis_move` (tool `nudge`) is `moving`,
`Nudge stopped: <reason>` when `halted`, else the `Nudged` words, so a later session
without `axis_move` reads `Nudged` and `useSession.ts`'s copy keeps it `Nudging`
(US5-S5). `GuidanceBlock.tsx` renders it after
`TrackControl` (`crates/finder/web/src/components/GuidanceBlock.tsx:95`).
`ImagingPage` (`crates/finder/web/src/components/ImagingPage.tsx:46`) gains the optional
props `session?: ObservingSession` and `onChange?: () => void` and renders
`{session && <NudgeBlock session={session} onChange={onChange} first="imaging_camera" />}`
after the ready camera's sections, outside `{camera != null && settings != null && (...)}`
(`crates/finder/web/src/components/ImagingPage.tsx:174` to
`crates/finder/web/src/components/ImagingPage.tsx:302` at f72b77d2), so it shows with or
without a ready imaging camera (on 054's page, after its frame and outside its ready
camera's sections). `App.tsx` passes `session={session}
onChange={changed}` (`crates/finder/web/src/App.tsx:200`).

## What was run and what was read

All on the probe branch `probe/055`, cut from f6d4f5fd, on the simulator, with the
factory's toolchain, `--locked`, and npm from its cache. Logs are outside the
repository.

**Round 0, the draft (2026-11-27).** Commits 3fa08e64 to fad20226, the draft's four
stories; its measurements are summarised in spec.md's owner decisions 1, 4 and 5 (the
mount and camera views at three chart rotations and three imaging angles; Track on with
and against the gears; the model's epoch).

**Review round 1 (the cynical review's reruns, on a scratch worktree of fad20226).**
Review-only tests showed a Track-on press read as a push (two coarse presses 1 s apart;
one coarse press from a 1.5′ Track error, either way): slip, epoch 2 to 3; Track turned
on during a press on target accepted; a second press at the turnaround accepted and
lost; the finder camera's u untested; no zenith guard (a coarse right press kept 75″ at
89.5° and 11″ at 89.95°). Its rerun of the story tests on f6d4f5fd's source failed all
13 Rust tests; its two trial lines (`checks_push` false while a step is pending; `nudge`
and `track` reserved) made those cases pass with 106 tests green.

**Round 1, the fold (2026-11-27).** Commits on top of fad20226: b61a08c8 and a858a153
(PROBE ONLY: the draft's measurement tests retired), then the five stories staged
three times as the fold settled, each a full tree: da0f07d2 to 2052782f and 7a4fac97
(PROBE ONLY), 070768e1 to 0e0ef5ee (US2's and US3's tests on US1's helpers), 6d57f290 to
947852aa (US1-S4's second half), and 06504889 (US4's glossary read whole), c61d2c07 and
3d218932 (PROBE ONLY, the tip). The stories are 6d57f290 (US1), ecf9e52c (US2),
cb9cc8cc (US3), 06504889 (US4) and c61d2c07 (US5).

- **Track's step (owner decision 4).** The seat's lead (e), Track making the step with
  GoTo's plan and approach from below, passed every way at the Track rig and failed on
  the setting target (START + 4 h): 529″ off after a coarse press, a push, a slip and two
  new epochs, the backlash 0.153° measured. (e′), the approach against Track's own way,
  passed every press, fine and coarse, each way at the Track rig, near transit (M42 at
  altitude 44.6°, its altitude moving 0.55″/s) and on the setting target: within 10″ of
  the stepped place 2.3 s to 7.0 s of simulated time after the press, at most 11.8″ off
  over the next 20 s, no push, no slip, the epoch unchanged, `axis_move` `done`, Track
  `tracking`. With no solves, within 20″ in 5 s (13.8″ best: the 10″ dead band's error
  Track already had). (f), refusing a press against the gears: down and left are
  against them at the Track rig and near transit, up and left on the setting target.
- **The push scenarios (review A and B).** A second coarse `left` 1 s after the first:
  409 `the mount is already moving`; over 10 s no push, no slip, the epoch 2 to 2, 7.9″
  from the stepped place at 10 s. From a 1.5′ Track error, coarse `up` and `down`: no
  push, no slip, within 10″ after 7.5 s and 27.0 s of simulated time.
- **Track off.** Every moved axis now dips 2,245 to 2,250 counts below its end and comes
  up (US1-S2). A fresh rig's first fine `up` moved the tube 510″ down and its first
  fine `right` 319″ the wrong way: the sync was read with the gears on their falling
  flank (Edge Cases); after the synced rig's two presses every press landed within 3 %.
- **Track on after Track-off presses (owner decision 10).** Two coarse `right` presses
  with Track off left the eyepiece 258″ and then 119″ from M42 as the sky turned; Track on
  GoTo'd back (GoTo error 8.4″) and held it 5″ from M42 after 60 s.
- **The zenith (FR-009).** The simulator's base tilt puts the tube 1.14° to 1.15° below
  the rig's nominal altitude. Synced rigs at nominal 84°, 85°, 86° and 88.5° (true 82.86°,
  83.86°, 84.85°, 87.35°): coarse `right` short by 10.1 %, 11.9 %, 14.3 % and 29.6 %,
  coarse `up` by 2.5 % to 7.0 %; at nominal 89° and 89.5° (model 88.01° and 88.50°) every
  press refused with FR-009's words.
- **Fails before**, each story's new tests on the tree of the story before: US1's 5 on
  f6d4f5fd (405), US2's 2 motion tests on US1 (400 `unknown variant`) and its 3 unit
  tests added to US1's `nudge.rs` (24 compile errors: US1's `offset` takes one
  argument and has no camera views), US3's 6 on US2 (409 `the mount is already moving`
  where 200 or the settling words were due), US4's 3 on US3 (tool not listed, README),
  US5's 4 Vitest tests on US4 (no `Nudge` region) and its first browser test on US4's
  `dist` (no `Nudge` region; the other two serial tests did not run).
- **Mutations**, one per new or changed FR, each alone on the tip and its scenario run:
  FR-002 and FR-003 (right's sign), FR-004 (`+ u`), FR-005 (kept cleared; ask time for
  mid-exposure; a mirrored frame un-mirrored), FR-006 (approach on falling axes only; the
  approach's floor check), FR-007 (`move_axis` for `nudge`), FR-008 (the two names out of
  the reservation), FR-009 (88° to 95°), FR-010 (approach always from below), FR-011
  (`checks_push` left on during a step; pointings fed during the step; not the window
  alone, removed or cut to 1 s, which no scenario caught: probe round 1 found it), FR-012 (the
  settling refusal; the stepping refusal with the reservation's `nudge`, as the
  tracker's alone is covered by FR-008's), FR-014 to FR-016 (the tool's words, the
  README's last sentence, the glossary's last clause), FR-017 (`Nudged` at once) and
  FR-018 (no block on the Imaging page): every one failed its scenario. FR-004's
  mutation failed US2-S2 and, by 0.1 %, US2-S1.
- **Gates on the tip 3d218932**: `lint` (fmt, clippy with `-D warnings`, both doc
  runs) passed; `cargo nextest run --workspace --all-targets --all-features --locked`
  passed 1,555 of 1,555 (the probe-only tests among them); `cargo test --workspace
  --locked --doc` passed; `npm ci`, typecheck, the Prettier check, Vitest (65 files, 611
  tests) and the build passed, `dist` unchanged by the build; `CI=1 npm run e2e` passed
  37 of 37.

**Probe round 1 from this text (2026-11-27).** On `probe/055-r1` from 21ae53a4 (f6d4f5fd
and this trio), the five stories built from the trio alone, in order, tests first:
ce5c7429 (US1), ff9a05c0 (US2), 6d313c86 (US3), 0f4775c9 (US4) and a6a92960 (US5, the
tip), each after its tests' commit, each story's tests failing on the story before. Both
gates passed on the tip (nextest 1,544, Vitest 611, `CI=1 npm run e2e` 37). Three
blocking and fourteen clarity defects, folded here. These mutations passed every
scenario: FR-011's window removed or cut to 1 s; `axis_move` `done` at the watcher's
first tick; `App` passing the Imaging page no session; the up and down glyphs swapped;
the block above Track; `useSession` dropping `axis_move`; the arrows never disabled; the
tracker's own stepping refusal (plan trap 7). In US3-S4 a push down and coarse `up`
came within 10″ at 7.6 s, swung 72.6″ past and was back within 10″ by about 26 s.

**This fold, on `probe/055-r1`.** d55caf08 (US3), 9f3b103f and 8b7212bb (US5) on
a6a92960, the tip 8b7212bb.

- **The new steps**, each with one mutation alone on the tip and its story's focused
  command, each failing its step: US3-S7 (the window removed; cut to 1 s); US3-S3's
  `moving` at 1 s (`done` at the watcher's first tick); US3-S1's `motion` (not put back,
  as on a6a92960); US3-S4's last 10 s (the loop held out 30 s after the step: US3-S4's
  push-down half failed at that clause, its other clauses passing); US5-S2 (the arrows never disabled); US5-S3 (up
  and down swapped; the block above Track); US5-S4's `off` view (the block inside the
  ready camera's sections, as on a6a92960); US5-S5's `Nudging` (`useSession` dropping
  `axis_move`; the status ignoring it); US5-S8 (`App` passing no session).
- **US3-S4's swing** (spec.md § Clarifications), read every 0.5 s for 45 s with Track
  started at five times 0.1 s apart: the same 1.5′ push down with no press swung 19″ to
  70″ past the held place and was within 15″ for good after 4.5 s to 18 s; with coarse
  `up`, 30″ to 84″ past and 13 s to 28 s. US3-S4's own run: within 10″ at 7.6 s and
  27.1 s, at most 10.5″ and 8.5″ off over the last 10 s.
- **Gates on 8b7212bb**: `lint` (fmt, clippy with `-D warnings`, both doc runs)
  passed; nextest 1,545 of 1,545 (1,525 existing, 20 new); the doc tests passed; `npm
  ci`, typecheck, the Prettier check, Vitest (65 files, 611 tests), the build and the
  `dist` check passed; `CI=1 npm run e2e` passed 38 of 38 (34 existing, 4 new). Every
  existing test passed unchanged. US3's seven tests failed on US2's code (409 `the mount
  is already moving` where 200 or the settling words were due).

**Read, not run.** Stop during a press or a Track-on step (the watchers' Stop paths,
unchanged); a press refused across the Sun guard or the cable's turn; a press from below
the minimum altitude; a press with Track in its keyhole wait or measuring backlash; a
press during an imaging run or a live stack; the imaging camera turned after a solve; the
`Nudge stopped:` status; a real mount and cameras.

## Read from the diff, not from a test

- `goto.rs` gains only additions (two variants, two arms, `through`); no line of GoTo's
  planning or correction is changed.
- `mcp::guard::start` keeps its signature and calls `start_by` with `"mcp"`.
- `Tracker::follow` and 038's push check are unchanged; `nudge` reuses `follow`'s
  clearing; with no step pending the rate loop commands what it commanded before this spec.
- The README's new section and the MCP paragraph keep every sentence they held before this spec.
- `NudgeBlock` posts only `postNudge`; `GuidanceBlock` gains one import and one line;
  `ImagingPage` two imports, two optional props and one line.
- The README section's sentences that US4-S3 does not quote (the approach, the settling
  refusal) are plan § Interfaces' words.

Probe round 1 found these read only from its diff; each is now a step: FR-011's window
(US3-S7), Track-on `axis_move` timing (US3-S3), the glyphs and the block's order
(US5-S3), the disabled arrows (US5-S2), the `axis_move` copy (US5-S5) and App's wiring
(US5-S8). The `Nudge stopped:` words stay read, not run.

## Shapes that existing tests pin

- **MCP's direct moves.** `crates/finder/tests/mcp/axis_moves.rs` (016) passes
  unchanged: `start` keeps its words and order (run).
- **038's push and 015's loop.** `crates/finder/tests/motion/tracking/` passes
  unchanged: `follow`, the threshold and the reopen window are untouched; `checks_push`
  is as before whenever no step is pending (run).
- **The reservation.** 016's and 015's tests that turn Track on or start a move during a
  direct move pass unchanged with `nudge` and `track` added (run).
- **The README's MCP count.** `crates/finder/tests/console/imaging_run_docs.rs:139`
  asserts "sixteen tools of its own" (`crates/finder/README.md:88`): the tool is a
  paragraph of its own (run). Its `section(&readme, "## API", "\n## ")` and
  `section(&readme, "## MCP clients", "\n## ")` still find their sections (run).
- **050's and 051's README checks.** `crates/finder/tests/mcp/imaging_align.rs:200` and
  `crates/finder/tests/mcp/finder_focus.rs` find their paragraphs before `### MCP under
  WSL2` and their sections' order (run).
- **The guidance block's Track tests and the Imaging page's.** `TrackControl.test.tsx`,
  `GuidancePush.test.tsx`, `ImagingPage.test.tsx`, `tracking.spec.ts` and the imaging
  browser specs pass unchanged: their locators are held to their own regions, and
  `ImagingPage`'s new props are optional (run).
- **The simulated console's state.** `nudge.spec.ts` sorts after `mcp.spec.ts` and
  before `simulate.spec.ts` in the simulate project; it sets the session as
  `tracking.spec.ts` does, turns Track off, and puts `imaging_camera` and `target` back
  to null at the end, so the specs after it see the server as they set it (run).

## Decisions and the alternatives rejected

| Decision | Chosen | Rejected, and why |
|---|---|---|
| Where the step is planned | `Planner::direct`, beside MCP's moves | A new planner: would duplicate the guards (general rule) |
| How the end is found | the model's inverse at both places, added as a difference | the inverse alone (the model's error moves the end); counts per degree (wrong near the zenith) |
| The approach, Track off | GoTo's 0.5° from below on every moved axis | falling axes only (decision 6 (b)) |
| Track on | Track makes the step, against its own way (decision 4 (e′)) | the draft's added commands (66 s to unbounded); GoTo's approach (529″ on a setting target); refusing half the arrows (f) |
| The push check | off while a step is made and 1 s + `SETTLE` after | on (the step read as a push: review A and B) |
| A second press | refused while a step runs, by the reservation and the tracker | accepted (lost at the turnaround; a push with Track on) |
| `motion` with Track on | the nudge's while its step is made, Track's again after, as a direct move's is read only while it runs | kept as the nudge's (probe round 1: the MCP note named the nudge for the rest of the run) |
| The imaging orientation | kept from the last solve that solved, at mid-exposure | the latest solve only (refuses after a failed one); the ask's time (turns with the field) |
| The tool | through `nudge::nudge` | through the route (the tool needs `by` `mcp`) |
| The page | the guidance block and the Imaging page | one place only (decision 7) |

## User Story 1 - The mount's view, the approach and the guards

`nudge.rs` with the mount view, the route, `AxisMove::Offset` and `Through` with the
zenith refusal, `start_by`, the reservation, and the tests of spec.md US1.

## User Story 2 - The camera views

The finder camera's and the imaging camera's arms of `offset`, `Kept` and `nudge_kept`,
the unit tests and `nudge_views.rs` of spec.md US2.

## User Story 3 - Track's step

`Tracker::nudge`, the loop's step and the quiet window, the Track branch, its `motion`
put back and the settling refusal in `nudge`, and the tests of spec.md US3.

## User Story 4 - MCP and the words

The tool, the README's paragraph, row and section, the glossary, and the tests of
spec.md US4.

## User Story 5 - The page

`postNudge`, `axis_move` in the page's session, `NudgeBlock`, its lines in
`GuidanceBlock` and `ImagingPage`, `App`'s props, the config entries, `dist`, and the
tests of spec.md US5.

## Verification (every story, before proposing)

The gates, copied from `ergane.yaml`:

- `lint`: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets
  --all-features --locked -- -D warnings`, `RUSTDOCFLAGS="-D warnings" cargo doc
  --workspace --no-deps --all-features --locked` and `RUSTDOCFLAGS="-D warnings" cargo
  doc -p finder --no-deps`.
- `test`: `cargo nextest run --workspace --all-targets --all-features --locked`,
  `cargo test --workspace --locked --doc`, then in `crates/finder/web`: `npm ci
  --no-audit --no-fund`, `npm run typecheck`, `npm run format:check`, `npm test`, `npm
  run build`, `git status --porcelain -- dist` empty, and `CI=1 npm run e2e`.

## Prerequisites before this spec is flipped to `ready`

- 052, 053 and 054 have landed, in that order, and this trio is re-anchored on that
  `main`. Met at f72b77d2: every `path:line` read again; `goto.rs`'s additions placed against 053's;
  the README, `GLOSSARY.md` and test `main.rs` additions placed after theirs; and the
  Imaging page's Nudge block placed on 054's rewritten `ImagingPage.tsx`, after its
  imaging frame and outside its ready camera's sections, with US5-S4 and the
  Refinement Record's last writer read again. Every cite into a changed file moved with its
  line; the record's entries name 053 US2, 054 US3 and 054 US4; the Nudge block sits after
  054's imaging frame, outside its ready camera's sections; the probe's commits replayed
  onto f72b77d2 with only `dist` to rebuild.
- The owner's ten decisions are answered. Met on 2026-11-27: decision 9 (a) by the owner,
  the other nine with their recommendations.
- A probe round from this text alone, folded. Met: probe round 1 on `probe/055-r1`
  (tip a6a92960, 3 blocking and 14 clarity defects), folded here; its new steps pass
  and bite on 8b7212bb (§ What was run).
- The page half has run once on the re-anchored base (`CI=1 npm run e2e`). Met: on the
  probe's code replayed onto f72b77d2, it passed 38 of 38, with every other gate 0; on
  8b7212bb it passed 38 of 38.
- No operator-staged file is needed. Met: every story writes only code, tests, docs
  and `dist`.

## Outside the factory

Written here and not run by any gate. **On the next field night**, with the owner at the
telescope: GoTo a target the imaging camera can solve (M42 or the Pleiades), turn Track
on, take an imaging frame and solve it, open the Imaging page, and centre the target in
the imaging camera's frame with its arrows, counting the presses of each step and
noting, for each, whether the frame moved the way the arrow pointed (owner decision 9)
and about how far. Then do the same once in `the finder camera`'s view on the main page.
Note any press refused and its words, and whether Track kept tracking without a push.
The counts and the notes go in the night's report; nothing in the repository waits for
them.

## Plan traps

1. **The story sizes.** Probe round 1's story commits, each diff from the previous
   story's commit without `dist`, with this fold's commits added to their story: US1
   28,611 B (43.7 % of 65,536; 38,625 B at 1.35 times, 98.2 % of 39,321), US2 26,156 B
   (39.9 %; 35,311 B, 89.8 %), US3 27,451 + 6,336 = 33,787 B (51.6 %; 45,612 B, 116.0 %),
   US4 18,512 B (28.2 %; 24,991 B, 63.6 %), US5 23,269 + 6,132 + 704 B plus `dist`'s
   1,189 + 617 B, 31,911 B (48.7 %; 43,080 B, 109.6 %). Every story is under 39,321 B as
   measured; US3 and US5 go over it at 1.35 times and stay whole, as 054's US1 and 051's
   US2 did (the factory's builds of 050 and 051 ran 0.78 to 1.04 times their probes).
   US1 gains no scenario. Should US3 go over 39,321 B as measured, its push-settling
   refusal and that test (about 2.2 KB) move to US4.
2. **The backlash in tests.** A fresh simulated rig's first press up or right takes up
   the sync's flank (§ What was run). US1's synced rig presses down and left first; the
   Track rig and the imaging rig are past it by Track's and the priming presses' moves.
3. **US1's helpers.** US2's and US3's tests use US1's helpers from `super::nudge`, which
   US1 writes `pub(super)` from the start, as no story may edit an earlier story's test
   file, and each used by US1's own tests, as a `pub(super)` item its file does not use
   is dead code to clippy's `-D warnings`: `START: f64`, `WAYS: [(&str, f64); 4]`,
   `STEPS: [(&str, f64); 2]`, `fn wrap(angle: f64) -> f64`, `async fn write(app:
   &Router, method: &str, path: &str, body: Value) -> (StatusCode, Value)`, `async fn
   nudge(app: &Router, direction: &str, step: &str, view: &str) -> (StatusCode, Value)`,
   `async fn refused(app: &Router, direction: &str, step: &str, view: &str, words:
   &str)`, `async fn rest(rig: &Rig) -> Value`, `async fn synced(alt: f64) -> Rig`, `fn
   moved(before: MountDirection, after: MountDirection) -> [f64; 2]` and `fn miss(moved:
   [f64; 2], size: f64, angle_deg: f64) -> f64`. `Router` is `axum::Router`, so the
   imaging rig's router takes the pressing ones too; `Rig` is
   `crate::support::goto::Rig`.
4. **The sky turns under a still mount.** The imaging camera's centre moves with the sky
   between the press and the read; US2-S3's oracle is where it would be.
5. **The rate limit in US4-S2.** A refused call takes a slot; wait 1.1 s before the
   three calls.
6. **`mount.moving` lags.** The browser test waits on `axis_move.state` `done` before
   `mount.moving` false; the page's session copies `axis_move` (FR-018).
7. **Two guards for a second Track-on press.** The reservation refuses it first; the
   tracker's own check covers a press that reaches the tracker between them.

## Risks

| Risk | Effect | Mitigation |
|---|---|---|
| The camera turned after a solve | arrows turned with it until the next solve | the README says the view comes from the latest solve that solved; take a frame after turning |
| The real mount's backlash differs from its measure | a Track-on step's flank is still Track's own way | the approach goes 0.5° past, more than any backlash measured |
| The one-sync model's scale | coarse presses miss by up to about 2 % | within the step's own size; the next solve shows it |
| 053 changes `goto.rs` | the arm and helper move | additions only; re-anchored before the flip |
| 054 rewrites `ImagingPage.tsx` | the block's place moves | one line after the frame; re-anchored before the flip |

## Refinement Record

Drafted on 2026-11-27 by the plate-solver operator seat's spec session against `main` at
f6d4f5fd, from a probe of that day on `probe/055`, folded the same day from cynical
review round 1 on that branch (five stories built with every scenario's test, both gates
passing on the tip, every story's tests failing on the story before, one mutation per
new or changed FR biting, Track's step measured rising, near transit and setting, and the
page run in a browser by day and by night) and from probe round 1, built from this text
alone on `probe/055-r1` (tip a6a92960) with its new steps proven there on 8b7212bb,
and a reading of MCP's direct moves and their guards, GoTo's planner, Track and 038's push check, the live pointing's up and the
chart rotation, the imaging solve, the guidance block and the Imaging page, the
constitution, the glossary and the ADRs. Writes come from the task text. Shared writes
name the spec story that last wrote the same file on `main`; within the spec, each
story's files written by an earlier one are ordered by `depends_on_merged`.

The ten owner decisions of spec.md are answered (2026-11-27: decision 9 (a) by the
owner, the other nine with their recommendations), and this record was written to those
answers. Its `open_questions` are empty: nothing is open, and § Prerequisites are met at
f72b77d2. Nothing in this record is an owner approval: those are in spec.md, and the
flip to `ready` gives them.

```yaml
version: 1
reviewed_revision: f72b77d2b7386932b824700288677039e096864f
stories:
  US1:
    tests:
      - path: crates/finder/tests/motion/nudge.rs
        status: new
        disposition: add
        assertion: "the synced rig (goto Rig at START, alt 50 az 120, synced, then fine down and fine left in the mount view, each read until it rests): coarse right in the mount view answers 200 with exactly {up_arcsec 0.0, right_arcsec 240.0, moved axes} and the session read at once has motion.by page and motion.tool nudge; then every step (fine 30, coarse 240 arcsec) each way (up 0, right 90, down 180, left 270 deg clockwise of the zenith) answers 200 moved axes, rests with axis_move.state done and axis_move.tool nudge, and the true tube direction's altitude change and azimuth change times cos alt land within 3 % of the step at its angle; model epoch unchanged, slip_pending false, a sync whose time is after the moment those presses began"
        decision: "spec.md:US1-S1"
      - path: crates/finder/tests/motion/nudge.rs
        status: new
        disposition: add
        assertion: "the synced rig, fine up, right, down and left in the mount view, the moved axis's counts read every 100 ms until it rests: each end at least 2240 counts above the lowest count read; the end above the start for up and right, below it for down and left"
        decision: "spec.md:US1-S2"
      - path: crates/finder/tests/motion/nudge.rs
        status: new
        disposition: add
        assertion: "the bare console: fine up mount 409 exactly no mount is connected; the synced rig: a foreign write 403 exactly writes require the console's own origin and an allowed host, text/plain 415, the extra key x 400, direction north 400; two fine up presses 200 then 409 exactly the mount is already moving; fine down then fine up at the turnaround 200 then 409 the same; min_alt_deg pointing.alt_deg - 0.02: coarse down 409 exactly the position is below the minimum altitude; - 0.2: coarse down 409 exactly approach would go below the minimum altitude, coarse right 200; frames stopped and the night reset: fine up 409 exactly the night has no sync yet"
        decision: "spec.md:US1-S3"
      - path: crates/finder/tests/motion/nudge.rs
        status: new
        disposition: add
        assertion: "the motion rig with the eyepiece at M42 before its sync and target M42: coarse down 200 and Track on 100 ms later 409 exactly the mount is already moving; fine down 200 and Track on at the turnaround 409 the same; both rest done with track absent or off; the eyepiece then more than 240 arcsec from M42; Track on 200, within 120 s track.state tracking and the eyepiece within 30 arcsec of M42"
        decision: "spec.md:US1-S4"
      - path: crates/finder/tests/motion/nudge.rs
        status: new
        disposition: add
        assertion: "the synced rig at 86 (tube about 1.15 deg lower on the tilted base): coarse right lands within 25 %, coarse up within 4 %; the motion rig at 89.5 synced: fine up 409 exactly the telescope is too near the zenith: nudges stop at 88° altitude"
        decision: "spec.md:US1-S5"
      - path: crates/finder/tests/mcp/axis_moves.rs
        status: existing
        disposition: preserve
        assertion: "016's direct axis moves over MCP keep their guards, words, rate limit and call log: mcp::guard::start keeps its signature and calls start_by with mcp"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/motion/goto/refusals.rs
        status: existing
        disposition: preserve
        assertion: "014's GoTo, its approach and refusals as before 055: Planner::direct only gains the Offset and Through arms"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "AxisMove's Position and Rate, Planner::direct's arms for them and GoTo's planning and correction are unchanged (goto.rs gains only additions)"
      - "MCP's goto_axis_position and move_axis answer, refuse and log as before this spec; start_motion reserves its five tools as before and adds nudge and track"
      - "the pointing model, the live solve and session.rs are not written"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test motion -E 'test(/^nudge::/)'"
        - "cargo nextest run -p finder --locked --test mcp -E 'test(axis_moves::)'"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - crates/finder/src/api.rs
      - crates/finder/src/goto.rs
      - crates/finder/src/lib.rs
      - crates/finder/src/mcp/guard.rs
      - crates/finder/src/nudge.rs
      - crates/finder/tests/motion/main.rs
      - crates/finder/tests/motion/nudge.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/api.rs
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US2
        disposition: merge_order
        reason: "055 depends_on_landed 051, whose US2 last changed api.rs (f9706cc0)"
      - path: crates/finder/src/goto.rs
        other_spec: 053-second-sync-fits-base-tilt
        other_story: US2
        disposition: merge_order
        reason: "055 depends_on_landed 053, whose US2 last changed goto.rs (9e344373)"
      - path: crates/finder/src/lib.rs
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US1
        disposition: merge_order
        reason: "055 depends_on_landed 051, whose US1 last changed lib.rs (4c39350e)"
      - path: crates/finder/src/mcp/guard.rs
        other_spec: 017-console-stop-confirm
        other_story: US2
        disposition: merge_order
        reason: "055 depends_on_landed 017, whose US2 last changed mcp/guard.rs (4273369d)"
      - path: crates/finder/tests/motion/main.rs
        other_spec: 054-focus-pause-gives-camera-back
        other_story: US3
        disposition: merge_order
        reason: "055 depends_on_landed 054, whose US3 last changed motion/main.rs (f72b77d2)"
    open_questions: []
  US2:
    tests:
      - path: crates/finder/tests/motion/nudge_views.rs
        status: new
        disposition: add
        assertion: "the bare console: fine up finder_camera 409 exactly the finder camera has no live pointing yet; the synced rig with chart_rotation_deg 200: every step each way in the finder_camera view answers 200 moved axes and lands within 3 % at phi + 200 deg clockwise of the zenith (the simulator's up_deg near 0: the rotation only)"
        decision: "spec.md:US2-S1"
      - path: crates/finder/src/nudge.rs
        status: new
        disposition: add
        assertion: "unit: a session whose latest live pointing reads up_deg 40, chart rotation 0 and 200: offset of a coarse press each way in finder_camera is [240 cos a, 240 sin a] as [up, right] within 1e-9, a = phi + R - 40"
        decision: "spec.md:US2-S2"
      - path: crates/finder/tests/motion/nudge_views.rs
        status: new
        disposition: add
        assertion: "the bare console and the imaging rig at imaging angle 100 (imaging ready, a sync): fine up imaging_camera 409 exactly the imaging camera has not solved a frame yet on both; after fine down and fine left in the mount view, an imaging frame taken, saved and solved: every step each way in imaging_camera answers 200 and the frame's centre (imaging_truth) moves from where it would be by the step within 4 %, at the position angle up_east_of_north_deg - phi (an unmirrored frame)"
        decision: "spec.md:US2-S3"
      - path: crates/finder/src/nudge.rs
        status: new
        disposition: add
        assertion: "unit: a mirrored frame's up and down equal the unmirrored frame's, its right and left the unmirrored left and right; a solve asked at T = START of a held frame begun at T - 1200 s with a 2 s exposure answers as one asked at T - 1199 s and more than 1 deg from one asked at T"
        decision: "spec.md:US2-S4"
      - path: crates/finder/src/nudge.rs
        status: new
        disposition: add
        assertion: "unit: after a solved solve, later solves reading solving and failed answer the solved one's offset; with nothing kept: the imaging camera's solve is still running, the imaging camera's latest frame did not solve, the imaging camera has not solved a frame yet; no site: a site is needed"
        decision: "spec.md:US2-S5"
    preserved_behavior:
      - "the mount view's answers and every refusal of US1 are unchanged"
      - "the live pointing, the chart rotation and the imaging solve are read only"
    checks:
      development:
        - "cargo nextest run -p finder --locked --lib --test motion -E 'test(/^nudge_views::/) | test(/^nudge::tests::/)'"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs and the night of plan.md Outside the factory"
    writes:
      - crates/finder/src/api.rs
      - crates/finder/src/nudge.rs
      - crates/finder/tests/motion/main.rs
      - crates/finder/tests/motion/nudge_views.rs
    no_write_reason: ""
    shared_writes: []
    open_questions: []
  US3:
    tests:
      - path: crates/finder/tests/motion/nudge_track.rs
        status: new
        disposition: add
        assertion: "the Track rig (eyepiece at M42, synced, Track on, 30 s), settled before each: every step each way in the mount view answers 200 moved track, axis_move at once exactly {state moving, tool nudge}; read every 100 ms for 30 s: the true place within 10 arcsec of the stepped place (the held place moved by the step with the simulator's horizon transform) within 10 s and within 30 arcsec over the next 20 s, no push key, slip_pending never true, then axis_move done and track tracking; motion.tool nudge at once and, once axis_move reads done, motion exactly as read just before the press; model epoch unchanged"
        decision: "spec.md:US3-S1"
      - path: crates/finder/tests/motion/nudge_track.rs
        status: new
        disposition: add
        assertion: "the Track rig at START + 4 h (M42 setting, Track drives altitude down): coarse up then coarse down, each as US3-S1"
        decision: "spec.md:US3-S2"
      - path: crates/finder/tests/motion/nudge_track.rs
        status: new
        disposition: add
        assertion: "the Track rig settled: coarse left 200, axis_move.state moving 1 s later, coarse left then 409 exactly the mount is already moving; 9 s more with no push key and slip_pending false; at 10 s the true place within 10 arcsec of the first press's stepped place; epoch unchanged"
        decision: "spec.md:US3-S3"
      - path: crates/finder/tests/motion/nudge_track.rs
        status: new
        disposition: add
        assertion: "the Track rig settled, the mount pushed by hand 1.5 arcmin down then coarse up, and on a new Track rig 1.5 arcmin up then coarse down: each 200; read for 45 s: within 10 arcsec of the stepped place and within 15 arcsec of it at every read of the last 10 s, no push key, slip_pending false, epoch unchanged"
        decision: "spec.md:US3-S4"
      - path: crates/finder/tests/motion/nudge_track.rs
        status: new
        disposition: add
        assertion: "the Track rig settled, frames stopped: coarse right 200 and the true place within 20 arcsec of the stepped place in 5 s"
        decision: "spec.md:US3-S5"
      - path: crates/finder/tests/motion/nudge_track.rs
        status: new
        disposition: add
        assertion: "the Track rig, push_by(0.0, -2.0): fine up at the first push.state moving and the first following 409 exactly a push by hand is still settling: press again in a few seconds; 10 s later 200 moved track; the motion rig synced, target M42 elsewhere, Track on reading armed: fine up 409 exactly Track is not tracking yet"
        decision: "spec.md:US3-S6"
      - path: crates/finder/tests/motion/nudge_track.rs
        status: new
        disposition: add
        assertion: "the Track rig settled, coarse left in the mount view: the tracker's checks_push false at every 100 ms read while axis_move.state reads moving, false at the first read it does not and 2.5 s after that, true 3.5 s after that"
        decision: "spec.md:US3-S7"
      - path: crates/finder/tests/motion/tracking/push.rs
        status: existing
        disposition: preserve
        assertion: "038's push check, threshold, stillness, reopen window and follow as before 055"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/motion/tracking/pi_loop.rs
        status: existing
        disposition: preserve
        assertion: "015's rate loop and correction as before 055 when no step is pending"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "Tracker::follow, commanded and the push check are unchanged; with no step pending the rate loop commands what it commanded before this spec and checks_push is as before"
      - "motion reads as before this spec but while a Track-on step is made"
      - "Track's keyhole, backlash take-up and steady mark behave as before when no step is pending"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test motion -E 'test(/^nudge_track::/)'"
        - "cargo nextest run -p finder --locked --test motion -E 'test(/^tracking::/)'"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs and the night of plan.md Outside the factory"
    writes:
      - crates/finder/src/api.rs
      - crates/finder/src/nudge.rs
      - crates/finder/src/tracking.rs
      - crates/finder/tests/motion/main.rs
      - crates/finder/tests/motion/nudge_track.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/tracking.rs
        other_spec: 040-stack-waits-for-steady
        other_story: US6
        disposition: merge_order
        reason: "055 depends_on_landed 040, whose US6 last changed tracking.rs (f1847647)"
    open_questions: []
  US4:
    tests:
      - path: crates/finder/tests/mcp/nudge.rs
        status: new
        disposition: add
        assertion: "the MCP rig synced with the call log: tools/list has nudge with a description containing real telescope, outputSchema type object, inputSchema properties direction, step and view each type string; nudge right coarse mount structuredContent exactly {up_arcsec 0.0, right_arcsec 240.0, moved axes}; the session at once motion.by mcp and motion.tool nudge; set_tracking on isError true with the text a direct axis move is running; azimuth counts grew, altitude counts unchanged; the call log has a line containing mcp nudge and none containing test-token"
        decision: "spec.md:US4-S1"
      - path: crates/finder/tests/mcp/nudge.rs
        status: new
        disposition: add
        assertion: "the MCP rig before its sync: nudge fine up mount isError the night has no sync yet; synced: direction north, no view, the extra key x each isError exactly nudge takes direction (up, right, down or left), step (fine or coarse) and view (mount, finder_camera or imaging_camera); after 1.1 s three coarse up mount calls one after another: the first not an error, the second the mount is already moving, the third the rate limit of 2 moving calls a second was reached; after the move ends and 1 s, fine up imaging_camera the imaging camera has not solved a frame yet"
        decision: "spec.md:US4-S2"
      - path: crates/finder/tests/mcp/nudge.rs
        status: new
        disposition: add
        assertion: "the committed README keeps The imaging camera has sixteen tools of its own; The page's nudge arrows have one tool: lies after The Tools page's focus aid for the finder camera has two tools: and before ### MCP under WSL2 with a line starting - `nudge` takes `direction` between; the ## API section has a row starting | `POST /api/mount/nudge` |; ## Nudging the telescope lies after ## Drawn sky on the real mount and before ## Recording a night and contains fine is 30 and coarse is 4 (with their marks), the finder camera has no live pointing yet, is never read as a push and never a sync, and Track on always does: centre with Track on; the glossary holds **Direct axis move** and **Nudge** whole with FR-016's words and the nudge's _Avoid_ line"
        decision: "spec.md:US4-S3"
      - path: crates/finder/tests/console/imaging_run_docs.rs
        status: existing
        disposition: preserve
        assertion: "the README's ## API rows of the run and ## MCP clients' sixteen tools of its own, found with section(.., \"\\n## \")"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/mcp/finder_focus.rs
        status: existing
        disposition: preserve
        assertion: "051's README paragraph before ### MCP under WSL2 and its sections' order"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/mcp/imaging_align.rs
        status: existing
        disposition: preserve
        assertion: "050's README paragraph before ### MCP under WSL2 and its sections' order"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "every MCP tool before this spec keeps its name, schema, description and answers"
      - "every README sentence and glossary entry before this spec stays, but Direct axis move's words (owner approval 7)"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test mcp -E 'test(/^nudge::/)'"
        - "cargo nextest run -p finder --locked --test console -E 'test(imaging_run_docs::)'"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - GLOSSARY.md
      - crates/finder/README.md
      - crates/finder/src/mcp/tools.rs
      - crates/finder/tests/mcp/main.rs
      - crates/finder/tests/mcp/nudge.rs
    no_write_reason: ""
    shared_writes:
      - path: GLOSSARY.md
        other_spec: 054-focus-pause-gives-camera-back
        other_story: US3
        disposition: merge_order
        reason: "055 depends_on_landed 054, whose US3 last changed GLOSSARY.md (f72b77d2)"
      - path: crates/finder/README.md
        other_spec: 054-focus-pause-gives-camera-back
        other_story: US3
        disposition: merge_order
        reason: "055 depends_on_landed 054, whose US3 last changed the README (f72b77d2)"
      - path: crates/finder/src/mcp/tools.rs
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US3
        disposition: merge_order
        reason: "055 depends_on_landed 051, whose US3 last changed mcp/tools.rs (9424a371)"
      - path: crates/finder/tests/mcp/main.rs
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US3
        disposition: merge_order
        reason: "055 depends_on_landed 051, whose US3 last changed mcp/main.rs (9424a371)"
    open_questions: []
  US5:
    tests:
      - path: crates/finder/web/src/components/NudgeBlock.test.tsx
        status: new
        disposition: add
        assertion: "GuidanceBlock from the recorded session tracking, fetch answering {up_arcsec 0, right_arcsec 240, moved axes}: finder_camera chosen in the Nudge region's Arrows follow box, Coarse 4 (arcmin) clicked then Nudge right: Coarse aria-pressed true; fetch called with /api/mount/nudge, POST, Content-Type application/json, body {direction right, step coarse, view finder_camera}; onChange once; the status exactly Nudging right 4 (arcmin) with an ellipsis after the answer and after a session with axis_move {moving, nudge}, and exactly Nudged right 4 (arcmin) after one with {done, nudge}"
        decision: "spec.md:US5-S1"
      - path: crates/finder/web/src/components/NudgeBlock.test.tsx
        status: new
        disposition: add
        assertion: "fetch holding its answer, then 409 {error the night has no sync yet}: Nudge up clicked, all four arrows disabled until it answers and Nudge up enabled after; the Nudge region's alert exactly the night has no sync yet"
        decision: "spec.md:US5-S2"
      - path: crates/finder/web/src/components/NudgeBlock.test.tsx
        status: new
        disposition: add
        assertion: "the session without imaging_camera: Arrows follow offers exactly the mount and the finder camera; with it: the mount, the finder camera and the imaging camera, the region Nudge after the region Track in the document, and Nudge up, right, down and left reading the arrows up, right, down and left; without mount: no Nudge region"
        decision: "spec.md:US5-S3"
      - path: crates/finder/web/src/components/NudgeBlock.test.tsx
        status: new
        disposition: add
        assertion: "ImagingPage from the recorded session tracking without its target, fetch answering a ready imaging view: a region Nudge follows the region Imaging frame in the document and its Arrows follow box reads imaging_camera; rendered again with fetch answering state off, once The scope node is not connected. shows, a region Nudge is still shown"
        decision: "spec.md:US5-S4"
      - path: crates/finder/web/e2e/nudge.spec.ts
        status: new
        disposition: add
        assertion: "the simulated console, Track off, by day: Arrows follow reads mount; Nudge right answers 200 exactly {up_arcsec 0, right_arcsec 30, moved axes} and the block's status Nudged right 30 (arcsec); after axis_move.state done and mount.moving false, Coarse then Nudge down answers exactly {up_arcsec -240, right_arcsec 0, moved axes} and Nudge up at once 409 with the block's alert the mount is already moving; once at rest, Coarse and Nudge left, and 1.5 s later the session's axis_move.state moving and the block's status exactly Nudging left 4 (arcmin) with an ellipsis"
        decision: "spec.md:US5-S5"
      - path: crates/finder/web/e2e/nudge.spec.ts
        status: new
        disposition: add
        assertion: "the simulated console, Track on with the guidance block's switch until track.state tracking: Coarse then Nudge up answers 200 exactly {up_arcsec 240, right_arcsec 0, moved track}; the block's status comes to Nudged up 4 (arcmin) with the words Track holds the new place within 20 s; every read each 500 ms for 10 s has track.state tracking and no push"
        decision: "spec.md:US5-S6"
      - path: crates/finder/web/e2e/nudge.spec.ts
        status: new
        disposition: add
        assertion: "the Nudge block's own screenshot by day has more than 1000 off-red pixels (red above 226 or green or blue above 0); after Night mode: red and black, html data-theme night and 1.5 s, none"
        decision: "spec.md:US5-S7"
      - path: crates/finder/web/e2e/nudge.spec.ts
        status: new
        disposition: add
        assertion: "the simulated console with the session nudge.spec.ts sets: /imaging opened, a region Nudge visible within 20 s and its Arrows follow box reading imaging_camera"
        decision: "spec.md:US5-S8"
      - path: crates/finder/web/src/components/TrackControl.test.tsx
        status: existing
        disposition: preserve
        assertion: "Track's region, switch, readout and words in the guidance block as before 055"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/src/components/ImagingPage.test.tsx
        status: existing
        disposition: preserve
        assertion: "the Imaging page's regions, controls and words as before 055, rendered without the new optional props"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/e2e/tracking.spec.ts
        status: existing
        disposition: preserve
        assertion: "Tracking M42 in the guidance block, its readout and its night rule, its locators held to the Track region"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "TrackControl.tsx, GoToControl.tsx, McpNote.tsx and Setup are unchanged; the Imaging page gains only the block and two optional props"
      - "playwright.config.ts gains only its two list entries (owner approval 9)"
      - "dist is the build of its source"
    checks:
      development:
        - "npm test -- NudgeBlock"
        - "npx playwright test --project=simulate e2e/nudge.spec.ts"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml finder-page job and the night of plan.md Outside the factory"
    writes:
      - crates/finder/web/dist
      - crates/finder/web/e2e/nudge.spec.ts
      - crates/finder/web/playwright.config.ts
      - crates/finder/web/src/App.tsx
      - crates/finder/web/src/api.ts
      - crates/finder/web/src/components/GuidanceBlock.tsx
      - crates/finder/web/src/components/ImagingPage.tsx
      - crates/finder/web/src/components/NudgeBlock.test.tsx
      - crates/finder/web/src/components/NudgeBlock.tsx
      - crates/finder/web/src/useSession.ts
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/web/dist
        other_spec: 054-focus-pause-gives-camera-back
        other_story: US4
        disposition: merge_order
        reason: "055 depends_on_landed 054, whose US4 last built dist (337dd09f)"
      - path: crates/finder/web/playwright.config.ts
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US4
        disposition: merge_order
        reason: "055 depends_on_landed 051, whose US4 last changed playwright.config.ts (f6d4f5fd)"
      - path: crates/finder/web/src/App.tsx
        other_spec: 050-tools-page-aligns-finder-camera
        other_story: US5
        disposition: merge_order
        reason: "055 depends_on_landed 050, whose US5 last changed App.tsx (c75a6e76)"
      - path: crates/finder/web/src/api.ts
        other_spec: 054-focus-pause-gives-camera-back
        other_story: US4
        disposition: merge_order
        reason: "055 depends_on_landed 054, whose US4 last changed api.ts (337dd09f)"
      - path: crates/finder/web/src/components/GuidanceBlock.tsx
        other_spec: 022-altaz-guidance
        other_story: US1
        disposition: merge_order
        reason: "055 depends_on_landed 022, whose US1 last changed GuidanceBlock.tsx (9e23cb1f)"
      - path: crates/finder/web/src/components/ImagingPage.tsx
        other_spec: 054-focus-pause-gives-camera-back
        other_story: US4
        disposition: merge_order
        reason: "055 depends_on_landed 054, whose US4 last rewrote ImagingPage.tsx (337dd09f)"
      - path: crates/finder/web/src/useSession.ts
        other_spec: 032-imaging-solve-and-aim-point
        other_story: US6
        disposition: merge_order
        reason: "055 depends_on_landed 032, whose US6 last changed useSession.ts (b507edc8)"
    open_questions: []
```
