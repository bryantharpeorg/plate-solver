# Implementation Plan: The Nudge Arrows Follow the Frame

**Branch**: `055-nudge-arrows-follow-the-frame` | **Date**: 2026-11-27 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/055-nudge-arrows-follow-the-frame/spec.md`

## Summary

A nudge moves the telescope one step on the sky, fine 30″ or coarse 4′, up, right,
down or left in one of three views: the mount's own, the finder camera's live views as
the page turns them, or the imaging camera's frame from its latest imaging solve. A new
module, `crates/finder/src/nudge.rs`, turns a press into an offset in the mount's view,
`[up, right]` arcseconds, and hands it on. With Track off it is a direct axis move of
both axes: a new `AxisMove::Offset` is checked by `Planner::direct` into a new
`AxisMove::Through`, two waypoints, the second the end and the first 0.5° below it on
any axis that moves down, through every guard MCP's direct moves pass, and
`mcp::guard` drives the axes through them. With Track on, `Tracker::nudge` moves the
place Track holds by the step and adds the step to the rate loop's next commands. A
nudge writes nothing to the model. MCP gains a `nudge` tool through the same function;
the README and the glossary gain the words; the guidance block gains a Nudge block
under Track. Four stories: the step and the guards; the imaging camera's view and
Track; MCP and the words; the page.

## Technical Context

**Language/Version**: Rust, the factory's toolchain; TypeScript and React for the page
(`crates/finder/web`).

**Primary Dependencies**: none new. `axum` routes, `rmcp` tools, the console's own
`goto`, `mcp::guard`, `tracking` and `sky` modules.

**Testing**: `cargo nextest` (the `motion` and `mcp` test binaries), Vitest and
Playwright, all under the `test` gate.

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
pixel right (column 0) and one pixel down (column 1); the frame's φ is the pixel
direction `[sin φ, −cos φ]`; east and north become across and up with the parallactic
angle q at the solve's place and time (q from `sky::alt_az` of the place and of a place
10⁻⁵° north of it), across = −cos q·east + sin q·north and up = sin q·east + cos q·north,
as `tracking.rs`'s `axis_frame` and `axis_error` turn them, normalised. No equatorial
direction is used.

**Track off.** `Planner::direct` (`crates/finder/src/goto.rs:220`) predicts the altitude
and azimuth at the axes' counts, adds the step (azimuth divided by the altitude's
cosine), and asks the model's inverse at both places; the end is the counts plus the
difference, so the model's own errors cancel to first order. Each axis whose counts
fall goes first to 0.5° (014's `APPROACH`, `crates/finder/src/goto.rs:23`, 2,250 counts
at 1,620,000 a turn) below its end. Both waypoints pass `ready` (Sun, mount),
the minimum altitude (or the altitude now, when lower) and `safe_path`
(`crates/finder/src/goto.rs:426`). `mcp::guard` drives both axes to the first waypoint
with `goto_axis`, then to the last, and reports `axis_move` as MCP's moves do.

**Track on.** `Tracker::nudge` turns `[up, right]` into east and north at the moment,
moves the held place's RA and Dec by them, and adds their axis arcseconds
(`axis_error`) to a new `Control::nudge`, which the rate loop drives over its next
seconds on top of the sky's rate and correction, through `commanded`
(`crates/finder/src/tracking.rs:1540`), which never reverses an axis: against the gears
the axis waits for the sky. The correction's memory is cleared as `follow` clears it
(038's FR-004), and no live pointing is used until 1 s plus `SETTLE`
(`crates/finder/src/tracking.rs:31`) after the step is done, so the frames of the move
do not feed the loop.

**Performance Goals**: a press answers at once (the route starts the move and returns).
With Track on and the gears, the telescope is at the new place within seconds (spec.md
SC-003).

**Constraints**: no new dependency; no change to the model, the live solve, GoTo's
correction solve or `session.rs` (053's); `goto.rs` gains only the two variants, one
arm and one helper.

**Scale/Scope**: about 1,700 added lines across four stories (the probe's, without
`dist` and the probe-only tests), more than half of them tests.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Detection, the plate solver and the live solve are untouched; the finder camera's view reads the live pointing's `up_deg` as the live loop gives it, and the imaging camera's view the imaging solve's `cd_deg` as 032 gives it. |
| II. Never weaken a gate | No existing test, golden, fixture, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS` or `ergane.yaml` is edited. `playwright.config.ts` gains two list entries, an owner approval as 050's and 051's were. The only edits to existing test files are the `mod` lines of `crates/finder/tests/motion/main.rs` and `crates/finder/tests/mcp/main.rs` (SC-002). Every existing test passed unchanged on the probe's tip (run, § What was run). |
| III. Tests first, independent expectations | The oracles are the simulator's true tube direction, eyepiece and imaging truth and its own horizon transform, the steps' sizes and angles, the mount's counts a turn, and the words of 014, 016, 038 and the scenarios. No test calls `offset` for an expected value or reads `FINE_ARCSEC`, `COARSE_ARCSEC` or `PUSH_QUIET_S`. Time comes from a paused tokio clock and the simulator's virtual clock. Each story's tests failed on f6d4f5fd's source where run (§ What was run). |
| IV. Read the critical paths twice | None is touched: no `Cargo.toml` or `Cargo.lock` change, no `unsafe`, nothing under `proto/`, `crates/math-core`, `crates/pattern-database` or `crates/database-generation`, no `build.rs` or `.cargo/`. The mount's motion is a safety path: every press goes through the guards MCP's direct moves already pass (FR-006) or through Track's own commands (FR-008). |
| V. Proven open source first, with honest licensing | Looked for, in local sources only (no download): the vendored `reference-solutions/` (cedar-detect, cedar-solve: no mount control) and the console's own MCP `move_axis`. **INDI's telescope motion** (`TELESCOPE_MOTION_NS` and `_WE`, held while a button is held, with a slew rate) and the **KStars/Ekos mount panel** built on it move the axes by direction and rate while held, in equatorial or the mount's own directions; neither reads a camera's frame, the scope node speaks its own protocol to the mount (045's INDI path is the camera's), and a held move is owner decision 3 (b). **SkySafari's and a hand controller's arrows** are the same held moves. **N.I.N.A.'s and Ekos's "centre on a solved frame"** move to a solved place, not by a step (out of scope). Nothing found reads a frame's own up and right for a press, so the step is written fresh, on the console's existing guards and tracker. |

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
crates/finder/src/nudge.rs                  # new (US1); US2 adds the imaging view and Track
crates/finder/src/goto.rs                   # US1: AxisMove::Offset, ::Through, one arm, `through` (053's file: small)
crates/finder/src/mcp/guard.rs              # US1: `start_by`, driving `Through`
crates/finder/src/api.rs                    # US1: one `merge` line
crates/finder/src/lib.rs                    # US1: `pub mod nudge;`
crates/finder/src/tracking.rs               # US2: `Tracker::nudge`, the loop's step
crates/finder/src/mcp/tools.rs              # US3: the `nudge` tool
crates/finder/README.md, GLOSSARY.md        # US3
crates/finder/tests/motion/nudge.rs         # US1, new
crates/finder/tests/motion/nudge_imaging.rs # US2, new
crates/finder/tests/motion/nudge_track.rs   # US2, new
crates/finder/tests/mcp/nudge.rs            # US3, new
crates/finder/web/src/api.ts                # US4: `postNudge`
crates/finder/web/src/components/NudgeBlock.tsx, NudgeBlock.test.tsx  # US4, new
crates/finder/web/src/components/GuidanceBlock.tsx                    # US4: one import, one line
crates/finder/web/e2e/nudge.spec.ts         # US4, new
crates/finder/web/playwright.config.ts      # US4: two entries
crates/finder/web/dist                      # US4: the build
```

**053's files.** Of `crates/finder/src/{model,goto,live,session}.rs`, only `goto.rs` is
written, and only by adding: two `AxisMove` variants after `Rate`, one arm in
`Planner::direct`'s `match`, and the private helper `through` after `direct`. No line of
GoTo's planning, correction solve or sync is changed. `session.rs` is read only
(`Session::tracker`, `crates/finder/src/session.rs:931`; `PushView`,
`crates/finder/src/session.rs:136`). This spec is re-anchored after 053 lands.

## Interfaces

### `crates/finder/src/nudge.rs` (US1 new; US2 adds)

```rust
//! Nudges: one press of an arrow moves the telescope one step on the sky, the
//! way the arrow points in the view the observer picked (055). With Track off it
//! is a direct axis move through every guard a GoTo passes; with Track on the
//! place Track holds moves by the same step, and the telescope with it.

pub const FINE_ARCSEC: f64 = 30.0;
pub const COARSE_ARCSEC: f64 = 240.0;
/// Seconds after Track follows a push by hand in which a press is refused (US2):
/// 038's `PUSH_REOPEN_S`.
pub const PUSH_QUIET_S: f64 = 10.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction { Up, Right, Down, Left }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Step { Fine, Coarse }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum View { Mount, FinderCamera, ImagingCamera /* US2 */ }

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NudgeAsk { pub direction: Direction, pub step: Step, pub view: View }

#[derive(Debug, Clone, Serialize)]
pub struct NudgeView { pub up_arcsec: f64, pub right_arcsec: f64, pub moved: &'static str }

/// `[up, right]` arcseconds in the mount's view.
pub fn offset(ask: &NudgeAsk, session: &SessionView, imaging: Option<&ImagingView>)
    -> Result<[f64; 2], String>;
/// `POST /api/mount/nudge`: `SessionWrite`, `sim_json`, then `nudge(state, "page", ..)`.
pub fn routes() -> Router<AppState>;
/// One press, from the page (`by` "page") or MCP ("mcp").
pub async fn nudge(state: &AppState, by: &'static str, ask: &NudgeAsk)
    -> Result<NudgeView, ApiError>;
```

The mount view's units are exact (no trigonometry), so the answer is exactly the step
and 0. `nudge` reads the session snapshot and, from US2, the imaging view; computes the
offset (its refusals first); with Track wanted (US2) refuses while a push settles
(FR-009) and otherwise calls `Session::tracker()` (none: `Track is not tracking yet`)
and `Tracker::nudge`, answering `moved` `track`; with Track off calls
`mcp::guard::start_by(state, by, "nudge", AxisMove::Offset { .. })` and answers `moved`
`axes`. In US1 `View` has `Mount` and `FinderCamera` only and the Track branch is absent:
a press with Track on goes to the direct move, which `GoTo`'s at-rest check refuses
with `the mount is already moving` while Track turns the axes.

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
must be finite`; `ready()`; predict (alt, az) at the current counts; to = (alt + up/3600,
az + right/3600/cos alt); the model's `inverse` at both, each with
`ephemeris::apparent_from_true` of its altitude (none: `the night has no sync yet`);
`last = current + round(there − here)`; for each axis whose `last` is below `current`,
`first = last − round(APPROACH · counts / 360)`, else `first = last`; `through(first,
last)?`; answer `Through { first, last }`. The arm `Through` re-checks with `through`.
`through`: `ready()`; a floor of the lower of the predicted altitude now and
`min_alt`, less 10⁻⁶; either waypoint's predicted altitude below it: `the position is
below the minimum altitude`; `safe_path` of `[last]` or `[first, last]`, none: 014's
path words.

### `crates/finder/src/mcp/guard.rs` (US1)

`start` becomes `start_by(state, "mcp", tool, request)`; `pub(crate) async fn
start_by(state, by, tool, request)` calls `api::start_motion(state, by, tool, ..)`.
`begin` sends `goto_axis` of both axes to `first` for a `Through`; the watcher `path`,
when neither axis is moving, ends the move if `first == last` and otherwise sends both
axes to `last` and goes on with `Through { first: last, last }`. Stop and the guards'
halts are the watcher's as at f6d4f5fd.

### `crates/finder/src/tracking.rs` (US2)

```rust
impl Tracker {
    /// Move the place held by `up` arcseconds of altitude and `right`
    /// arcseconds across the sky the way azimuth grows, and drive the axes by
    /// the same step from the rate loop's next turn, with no solve needed (055).
    /// Refused, changing nothing, unless the state is `tracking` and no
    /// backlash is being measured.
    pub fn nudge(&self, up: f64, right: f64) -> Result<(), String>;
}
```

Under the control lock: the refusal `Track is not tracking yet`; q from `axis_frame` at
now; east = −cos q·right + sin q·up, north = sin q·right + cos q·up; the axis step from
`axis_error`; the held target's RA and Dec moved by east and north; `followed` true and
the memory cleared as `follow` clears it (proportional, previous pointing, samples,
subs, pending, lapsed, held; `fresh_after` now); `Control::nudge` += the axis step;
`mark`. `Control` gains `nudge: [f64; 2]` and `quiet_until: f64`. Each rate-loop turn
commands `commanded(rate, direction, correction + nudge/3600/1 s)` and takes what that
moved beyond `commanded(rate, direction, correction)` off `nudge`, zeroing an axis
under 0.5″ or past its sign; when both are zero, `quiet_until` = now + 1 s + `SETTLE`,
the steady mark's settling restarts with the cause `nudged`. The tracker's `pointing`
returns before the loop while `nudge` is not zero or before `quiet_until`; `pause`
reports `nudged` while `nudge` is not zero; the steady mark needs `nudge` zero.

### `crates/finder/src/mcp/tools.rs` (US3)

```rust
#[derive(rmcp::schemars::JsonSchema)]
#[expect(dead_code, reason = "schema only; nudge parses its arguments as the route does")]
struct NudgeArguments {
    /// up, right, down or left
    direction: String,
    /// fine or coarse
    step: String,
    /// mount, finder_camera or imaging_camera
    view: String,
}
```

The tool `nudge`, after `move_axis` and before `set_tracking`, with the description:
"Move a real telescope one step on the sky, as the page's nudge arrows do: direction
up, right, down or left; step fine (30 arcseconds) or coarse (4 arcminutes); view mount
(up is higher altitude, right a clockwise turn), finder_camera (the finder camera's live
views as the page turns them) or imaging_camera (the imaging camera's frame, from its
latest imaging solve). With Track off the axes move through all console guards; with
Track on the place Track holds moves by the step and the telescope with it. Answers the
step as up_arcsec and right_arcsec, and moved: axes or track." It parses its arguments
into `NudgeAsk` (any failure: FR-011's words), then `limit_move()`, then
`nudge::nudge(&state, "mcp", &ask)`, `log_call("nudge", ..)` and `tool_result`.

### The README (`crates/finder/README.md`, US3)

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

After `## Drawn sky on the real mount` and before `## Recording a night`
(`crates/finder/README.md:637`), with two blank lines before each heading as there:

```markdown
## Nudging the telescope

A nudge moves the telescope one step on the sky: fine is 30″, coarse is 4′. The
arrows follow one of three views. `mount`: up is higher altitude and right a
clockwise turn. `finder_camera`: up, right, down and left as the page draws the
finder camera's live views, turned by the chart rotation; it needs a live
pointing. `imaging_camera`: as the imaging camera's frame shows them, from its
latest imaging solve; it needs one. Without them the press is refused with
`the finder camera has no live pointing yet` or `the imaging camera has not
solved a frame yet`.

With Track off a nudge is a direct axis move of both axes: it needs a sync and
passes every guard a GoTo passes, and an axis that moves down goes 0.5° past and
comes back up, as a GoTo does, so the gears end on the same side. With Track on
the place Track holds moves by the step and the telescope follows; a press is
refused with `a push by hand is still settling: press again in a few seconds`
while a push by hand is still being read. A step is under the 5′ push threshold, so
a nudge is never read as a push. A nudge is never a sync: the next solve syncs as
usual.
```

### The glossary (`GLOSSARY.md`, US3)

**Direct axis move** (`GLOSSARY.md:273`) takes FR-013's words; **Nudge** follows it,
with `_Avoid_: jog, slew, manual move`.

### The page (`crates/finder/web/src/`, US4)

`api.ts`, before `postRecord` (`crates/finder/web/src/api.ts:946`):

```ts
export type NudgeDirection = "up" | "right" | "down" | "left";
export type NudgeStep = "fine" | "coarse";
export type NudgeView = "mount" | "finder_camera" | "imaging_camera";
export interface Nudged { up_arcsec: number; right_arcsec: number; moved: "axes" | "track"; }
export async function postNudge(direction: NudgeDirection, step: NudgeStep, view: NudgeView): Promise<Nudged>;
```

posting `{direction, step, view}` with `Content-Type: application/json` through the
module's `json` reader, so a refusal throws its `error`. `NudgeBlock.tsx` exports
`NudgeBlock({ session, onChange })`: nothing without a connected mount; a
`<section aria-label="Nudge">` with a labelled `<select aria-label="Arrows follow">`
styled as `FocusTool.tsx`'s camera box, a `role="group"` `Step` of two `.button`s with
`aria-pressed`, a three-column grid of four `.button`s at least 44 CSS pixels
(`min-h-11 min-w-11`) labelled `Nudge up`, `Nudge right`, `Nudge down` and `Nudge
left`, drawn ↑, ←, →, ↓ in a cross, disabled while a press is pending; a
`<p role="status">` and a `<p role="alert">` as `TrackControl.tsx` has them. The view
and step live in the component's state. `GuidanceBlock.tsx` renders it after
`TrackControl` (`crates/finder/web/src/components/GuidanceBlock.tsx:95`).

## What was run and what was read

All on the probe branch `probe/055`, cut from f6d4f5fd, on the simulator, with the
factory's toolchain, `--locked`, and npm from its cache. Logs are outside the
repository.

**Commits.** 3fa08e64 (the route, the offset move and Track's nudge), 5352937d (PROBE
ONLY: measurements), a8b07645 (the push-settling refusal), a991ad07 (US1's tests),
c54d5837 (US2's Track tests), d5e16689 (PROBE ONLY: syncs after presses), be75e894
(US3: the tool, the README, the glossary), b379bffb (US4: the page, its tests, `dist`),
12d0770e (PROBE ONLY: Track turned on during a press), 8dd905ab (US1-S5, US2's imaging
test in its own file, US3's `set_tracking` refusal), 962b472b (US2-S5's last press
read for `moved`), fad20226 (US1-S1 reads the page's `motion`). Tip fad20226.

**Run.**

- Track off, every step each way (measurement tests): the mount view landed within
  0.42″ to 0.61″ for fine (2.0 % at most) and 1.1″ to 4.9″ for coarse (2.0 %), the
  coarse misses an azimuth scale error of the one-sync model; the finder camera's view
  at chart rotations 0°, 90° and 200° within 2.1 %, the live pointing's `up_deg` about
  0.26°; the imaging camera's view at imaging angles 17°, 100° and 250° within 0.19″ to
  5.6″ (2.9 %), direction within 1.2°. Each press's move took 1.2 s to 2.8 s of
  simulated time, the approach the longer. The first fine `up` and `right` presses on a
  fresh rig moved 0″: the mount's 0.3° backlash; after one press down and one left
  (the synced rig) every press moved.
- Track on, 30 s after Track started on M42 at altitude 50°, azimuth 120°: with the
  gears (up, right) within 10″ of the stepped place 0.7 s to 1.8 s after the press,
  then within 15.2″ of it; against the gears (down, left) fine within 3.6 s to 5.6 s,
  coarse about 66 s (down) and 80 s (left), the sky's own rate there (about 3.2″/s in
  altitude); the held place moved by exactly the step; with frames stopped, within 10″
  in about 1 s to 2 s, then drifting with the sky's error (about 1″/s), as Track does
  with no solves. Option 4 (b) (only the place, the loop to bring the telescope):
  coarse up in 30 s with solves; with none it never moved. 038's push never fired in
  any run.
- The model: after presses the epoch was the one before, `slip_pending` false, and the
  live solve went on recording syncs (the model keeps one per 120 s and 2°, so the
  count does not grow; the newest's time does).
- Track turned on from the page during a press: 409 `the mount is already moving`,
  the move ending `done` (US1-S5). From MCP: `a direct axis move is running`
  (US3-S1).
- Fails before: US1's and US2's nine tests of commits a991ad07 and c54d5837, run on
  f6d4f5fd's source, all failed (the route answered 405). US2's Track tests without the
  Track branch failed with `the mount is already moving`. Not run before: US1-S5, US3's
  and US4's tests (each meets a missing route, tool or component).
- Gates: at 12d0770e, `lint` (fmt, clippy, both docs) passed, `cargo nextest run
  --workspace --all-targets --all-features --locked` passed every test, the probe-only
  ones among them, and `cargo test --workspace --locked --doc` passed. The page's
  typecheck, Prettier check, Vitest (65 files) and build passed; the first `CI=1 npm
  run e2e` passed every browser test but the nudge spec's first, whose wait for rest
  read `mount.moving` before telemetry caught up; with the wait on `axis_move.state`
  `done` (US4-S4) it passed. On 962b472b, `npm run build` left `dist` unchanged and the
  full `CI=1 npm run e2e` passed, 37 of 37. On the tip fad20226, fmt and clippy passed
  and the nudge tests of the `motion` and `mcp` binaries passed; the later commits
  change test files only.

**Read, not run.** Stop during a press (the watcher's Stop path, unchanged); a press
refused across the Sun guard or the cable's turn; a press with Track armed or in its
keyhole wait; the imaging camera turned after a solve; a real mount and cameras.

## Read from the diff, not from a test

- `goto.rs` gains only additions (two variants, one arm, `through`); no line of GoTo's
  planning or correction is changed.
- `mcp::guard::start` keeps its signature and calls `start_by` with `"mcp"`.
- `Tracker::follow` and 038's push check are unchanged; `nudge` reuses `follow`'s
  clearing.
- The README's new section and the MCP paragraph keep every sentence at f6d4f5fd.
- `NudgeBlock` posts only `postNudge`; `GuidanceBlock` gains one import and one line.

## Shapes that existing tests pin

- **MCP's direct moves.** `crates/finder/tests/mcp/axis_moves.rs` (016) passes
  unchanged: `start` keeps its words and order (run).
- **038's push.** `crates/finder/tests/motion/tracking/` and the push tests pass
  unchanged: `follow`, the threshold and the reopen window are untouched (run).
- **The README's MCP count.** `crates/finder/tests/console/imaging_run_docs.rs:139`
  asserts "sixteen tools of its own" (`crates/finder/README.md:88`): the tool is a
  paragraph of its own (run). Its `section(&readme, "## API", "\n## ")` and
  `section(&readme, "## MCP clients", "\n## ")` still find their sections (run).
- **050's and 051's README checks.** `crates/finder/tests/mcp/imaging_align.rs:200` and
  `crates/finder/tests/mcp/finder_focus.rs` find their paragraphs before `### MCP under
  WSL2` and their sections' order: the new paragraph follows theirs, and the new
  section lies before `## Recording a night`, far from theirs (run).
- **The guidance block's Track tests.** `TrackControl.test.tsx`, `GuidancePush.test.tsx`
  and `tracking.spec.ts` pass unchanged: their locators are held to the Track region
  (run).
- **The simulated console's state.** `nudge.spec.ts` sorts after `mcp.spec.ts` and
  before `simulate.spec.ts` in the simulate project; it sets the session as
  `tracking.spec.ts` does, turns Track off, and puts `imaging_camera` and `target` back
  to null at the end, as `tracking.spec.ts` does, so the specs after it see the server
  as they set it (run: every browser test passed).
- **`playwright.config.ts`.** The two entries put `nudge.spec.ts` in the simulate
  project only (run).

## Decisions and the alternatives rejected

| Decision | Chosen | Rejected, and why |
|---|---|---|
| Where the step is planned | `Planner::direct`, beside MCP's moves | A new planner: would duplicate the guards (general rule) |
| How the end is found | the model's inverse at both places, added as a difference | the inverse alone (the model's error moves the end); counts per degree (wrong near the zenith) |
| The approach | GoTo's 0.5° from below | none (decision 6 (b)) |
| Track on | `Tracker::nudge` | a direct move (fights Track); `follow` alone (decision 4 (b)) |
| The push window | refuse while `moving` or 10 s after `following` | allow (038 would read the step as a second push) |
| The tool | through `nudge::nudge` | through the route (the tool needs `by` `mcp`) |
| The page | guidance block, under Track | the Imaging page (054's), Setup (decision 7) |

## User Story 1 - The step, the route and the guards

`nudge.rs` with the mount and finder camera views, the route, `AxisMove::Offset` and
`Through`, `start_by`, and the tests of spec.md US1.

## User Story 2 - The imaging camera's view and Track

The imaging view in `offset`, the Track branch and the push refusal in `nudge`,
`Tracker::nudge` and the loop's step, and the tests of spec.md US2.

## User Story 3 - MCP and the words

The tool, the README's paragraph, row and section, the glossary, and the tests of
spec.md US3.

## User Story 4 - The page

`postNudge`, `NudgeBlock`, its line in `GuidanceBlock`, the config entries, `dist`,
and the tests of spec.md US4.

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

- 052, 053 and 054 have landed, and this trio is re-anchored on that `main`: every
  `path:line` read again, `goto.rs`'s additions placed against 053's, and the
  `ImagingPage.tsx` decision (7 (b)) read against 054's page.
- The owner's nine decisions are answered.
- A cynical review and a probe round from this text alone, folded.
- The page half has run once on the re-anchored base (`CI=1 npm run e2e`).
- No operator-staged file is needed.

## Outside the factory

Written here and not run by any gate. **On the next field night**, with the owner at the
telescope: GoTo a target the imaging camera can solve (M42 or the Pleiades), turn Track
on, take an imaging frame and solve it, choose `the imaging camera` in `Arrows follow`,
and centre the target in the imaging camera's frame with the arrows, counting the
presses of each step and noting, for each, whether the frame moved the way the arrow
pointed and about how far. Then do the same once in `the finder camera`'s view. Note
any press refused and its words, and whether Track kept tracking without a push. The
counts and the notes go in the night's report; nothing in the repository waits for
them.

## Plan traps

1. **The story sizes.** Measured on the probe's commits, each story's diff without
   `dist`: US1 about 23,400 B (35.7 % of 65,536), US2 about 26,700 B (40.7 %), US3 about
   16,400 B (25.0 %), US4 about 17,100 B plus `dist`'s 1,200 B (27.9 %). At 1.35 times,
   US2 is about 36,000 B, under 39,321.
2. **The backlash in tests.** A fresh simulated mount's first press up or right moves
   0″ (§ What was run). US1's synced rig presses down and left first.
3. **The sky turns under a still mount.** The imaging camera's centre moves with the sky
   between the press and the read; US2-S1's oracle is where it would be.
4. **The rate limit in US3-S2.** A refused call takes a slot; wait 1.1 s before the
   three calls.
5. **`mount.moving` lags.** The browser test waits on `axis_move.state` `done` (read
   raw; the page's types have no `axis_move`) before `mount.moving` false.
6. **US1 has two views.** `imaging_camera` is US2's; US1's tests do not press it.

## Risks

| Risk | Effect | Mitigation |
|---|---|---|
| The camera turned after a solve | arrows turned with it until the next solve | the README says the view comes from the latest solve; take a frame after turning |
| A coarse press against a large Track error | 038 could read it as a push | the step is 4′ under 5′; none seen in the probe; the field night watches for it |
| Against the gears with Track on | a coarse press waits for the sky, about a minute | the README says the axes go with the gears; fine presses take seconds |
| The one-sync model's scale | coarse presses miss by up to about 2 % | within the step's own size; the next solve shows it |
| 053 changes `goto.rs` | the arm and helper move | additions only; re-anchored before the flip |

## Refinement Record

Drafted on 2026-11-27 by the plate-solver operator seat's spec session against `main` at
f6d4f5fd, from a probe of that day on `probe/055` (a trial build of the four stories
with every scenario's test and both gates passing, US1's and US2's tests failing before
them, the steps measured in every view with Track off and on, the push window and Track
during a press tried, and the page run in a browser by day and by night), and a reading
of MCP's direct moves and their guards, GoTo's planner, Track and 038's push check, the
live pointing's up and the chart rotation, the imaging solve, the guidance block, the
constitution, the glossary and the ADRs. Writes come from the task text. Shared writes
name the spec story that last wrote the same file on `main`; within the spec, each
story's files written by an earlier one are ordered by `depends_on_merged`.

None of the nine owner decisions of spec.md is answered; this record is written to the
recommendations. Its `open_questions` are empty because the validator refuses a record
that holds any; what is open is spec.md's owner decisions 1 to 9 and § Prerequisites
(the re-anchoring after 052 to 054). Nothing in this record is an owner approval: those
are in spec.md, and flipping to `ready` gives them.

```yaml
version: 1
reviewed_revision: f6d4f5fd482ccc9d5b634ef1aa5124c112769583
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
        assertion: "the synced rig with PUT /api/session chart_rotation_deg 200: every step each way in the finder_camera view answers 200 moved axes and lands within 3 % of the step at phi + 200 deg clockwise of the zenith"
        decision: "spec.md:US1-S2"
      - path: crates/finder/tests/motion/nudge.rs
        status: new
        disposition: add
        assertion: "the synced rig, fine down, fine up, fine left in the mount view, the moved axis's counts read every 100 ms until it rests: down (altitude) and left (azimuth) end at least 2240 counts above the lowest count read and below their start; up reads no count below its start"
        decision: "spec.md:US1-S3"
      - path: crates/finder/tests/motion/nudge.rs
        status: new
        disposition: add
        assertion: "the bare console: fine up finder_camera 409 exactly the finder camera has no live pointing yet, mount 409 exactly no mount is connected; the synced rig: a foreign write 403 exactly writes require the console's own origin and an allowed host, text/plain 415, the extra key x 400, direction north 400, two fine up presses one after the other 200 then 409 exactly the mount is already moving; min_alt_deg set to pointing.alt_deg - 0.2: coarse down 409 exactly the position is below the minimum altitude, coarse up 200; frames stopped and the night reset: fine up 409 exactly the night has no sync yet"
        decision: "spec.md:US1-S4"
      - path: crates/finder/tests/motion/nudge.rs
        status: new
        disposition: add
        assertion: "the motion rig synced with the target M42: coarse down in the mount view 200, 100 ms later POST /api/track on 409 exactly the mount is already moving, the move rests with axis_move.state done and track absent or off"
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
      - "MCP's goto_axis_position and move_axis answer, refuse and log as at f6d4f5fd"
      - "the pointing model, the live solve and session.rs are not written"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test motion -E 'test(nudge::)'"
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
        other_spec: 024-mount-link-recovery
        other_story: US3
        disposition: merge_order
        reason: "055 depends_on_landed 024, whose US3 last changed goto.rs (20839158)"
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
        other_spec: 050-tools-page-aligns-finder-camera
        other_story: US3
        disposition: merge_order
        reason: "055 depends_on_landed 050, whose US3 last changed motion/main.rs (6b70a4a2)"
    open_questions: []
  US2:
    tests:
      - path: crates/finder/tests/motion/nudge_imaging.rs
        status: new
        disposition: add
        assertion: "the bare console and the imaging rig at imaging angle 100 (imaging ready, a sync): fine up imaging_camera 409 exactly the imaging camera has not solved a frame yet on both; after fine down and fine left in the mount view, an imaging frame taken, saved and solved: every step each way in the imaging_camera view answers 200 and the frame's centre (imaging_truth) moves from where it would be (its alt az at the press turned back to RA Dec at the read) by the step within 4 % of the step, phi clockwise of the frame's up from up_east_of_north_deg"
        decision: "spec.md:US2-S1"
      - path: crates/finder/tests/motion/nudge_track.rs
        status: new
        disposition: add
        assertion: "the Track rig (eyepiece at M42, synced, Track on, 30 s): coarse up then coarse right in the mount view each answer 200 moved track, the true place comes within 10 arcsec of the stepped place within 5 s and stays within 30 arcsec over the next 20 s, track.state tracking with no push key; model epoch unchanged, slip_pending false"
        decision: "spec.md:US2-S2"
      - path: crates/finder/tests/motion/nudge_track.rs
        status: new
        disposition: add
        assertion: "the Track rig: fine down then fine left in the mount view each answer 200 moved track, within 10 arcsec of the stepped place within 15 s, within 30 arcsec over the next 20 s, track.state tracking with no push key"
        decision: "spec.md:US2-S3"
      - path: crates/finder/tests/motion/nudge_track.rs
        status: new
        disposition: add
        assertion: "the Track rig settled within 10 arcsec of the held place, frames stopped: coarse right in the mount view answers 200 and the true place comes within 10 arcsec of the stepped place within 5 s"
        decision: "spec.md:US2-S4"
      - path: crates/finder/tests/motion/nudge_track.rs
        status: new
        disposition: add
        assertion: "the Track rig, mount push_by(0.0, -2.0): fine up when track.push.state first reads moving 409 exactly a push by hand is still settling: press again in a few seconds; when it first reads following 409; 10 s later 200 moved track"
        decision: "spec.md:US2-S5"
      - path: crates/finder/tests/motion/tracking/push.rs
        status: existing
        disposition: preserve
        assertion: "038's push check, threshold, stillness, reopen window and follow as before 055"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/motion/tracking/pi_loop.rs
        status: existing
        disposition: preserve
        assertion: "015's rate loop and correction as before 055 when no nudge is pending"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "Tracker::follow, commanded and the push check are unchanged; with no nudge pending the rate loop commands what it commanded at f6d4f5fd"
      - "a press never reverses an axis against its gears"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test motion -E 'test(nudge_imaging::) | test(nudge_track::)'"
        - "cargo nextest run -p finder --locked --test motion -E 'test(tracking::)'"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs and the night of plan.md Outside the factory"
    writes:
      - crates/finder/src/nudge.rs
      - crates/finder/src/tracking.rs
      - crates/finder/tests/motion/main.rs
      - crates/finder/tests/motion/nudge_imaging.rs
      - crates/finder/tests/motion/nudge_track.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/tracking.rs
        other_spec: 040-stack-waits-for-steady
        other_story: US6
        disposition: merge_order
        reason: "055 depends_on_landed 040, whose US6 last changed tracking.rs (f1847647)"
    open_questions: []
  US3:
    tests:
      - path: crates/finder/tests/mcp/nudge.rs
        status: new
        disposition: add
        assertion: "the MCP rig synced with the call log: tools/list has nudge with a description containing real telescope, outputSchema type object, inputSchema properties direction, step and view each type string; nudge right coarse mount structuredContent exactly {up_arcsec 0.0, right_arcsec 240.0, moved axes}; the session at once motion.by mcp and motion.tool nudge; set_tracking on isError true with the text a direct axis move is running; azimuth counts grew, altitude counts unchanged; the call log has a line containing mcp nudge and none containing test-token"
        decision: "spec.md:US3-S1"
      - path: crates/finder/tests/mcp/nudge.rs
        status: new
        disposition: add
        assertion: "the MCP rig before its sync: nudge fine up mount isError the night has no sync yet; synced: direction north, no view, the extra key x each isError exactly nudge takes direction (up, right, down or left), step (fine or coarse) and view (mount, finder_camera or imaging_camera); after 1.1 s three coarse up mount calls one after another: the first not an error, the second the mount is already moving, the third the rate limit of 2 moving calls a second was reached; after the move ends and 1 s, fine up imaging_camera the imaging camera has not solved a frame yet"
        decision: "spec.md:US3-S2"
      - path: crates/finder/tests/mcp/nudge.rs
        status: new
        disposition: add
        assertion: "the committed README keeps The imaging camera has sixteen tools of its own; The page's nudge arrows have one tool: lies after The Tools page's focus aid for the finder camera has two tools: and before ### MCP under WSL2 with a line starting - `nudge` takes `direction` between; the ## API section has a row starting | `POST /api/mount/nudge` |; ## Nudging the telescope lies after ## Drawn sky on the real mount and before ## Recording a night and contains fine is 30 and coarse is 4 (with their marks), the finder camera has no live pointing yet and A nudge is never a sync; the glossary has **Nudge**: with FR-013's words and no Only MCP clients ask for one;"
        decision: "spec.md:US3-S3"
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
      - "every MCP tool at f6d4f5fd keeps its name, schema, description and answers"
      - "every README sentence and glossary entry at f6d4f5fd stays, but Direct axis move's words (owner approval 4)"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test mcp -E 'test(nudge::)'"
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
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US3
        disposition: merge_order
        reason: "055 depends_on_landed 051, whose US3 last changed GLOSSARY.md (9424a371)"
      - path: crates/finder/README.md
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US3
        disposition: merge_order
        reason: "055 depends_on_landed 051, whose US3 last changed the README (9424a371)"
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
  US4:
    tests:
      - path: crates/finder/web/src/components/NudgeBlock.test.tsx
        status: new
        disposition: add
        assertion: "GuidanceBlock from the recorded session tracking, fetch answering {up_arcsec 0, right_arcsec 240, moved axes}: finder_camera chosen in the Nudge region's Arrows follow box, Coarse 4 (arcmin) clicked then Nudge right: Coarse aria-pressed true; fetch called with /api/mount/nudge, POST, Content-Type application/json, body {direction right, step coarse, view finder_camera}; onChange once; the region's status exactly Nudged right 4 (arcmin)"
        decision: "spec.md:US4-S1"
      - path: crates/finder/web/src/components/NudgeBlock.test.tsx
        status: new
        disposition: add
        assertion: "fetch answering 409 {error the night has no sync yet}: Nudge up clicked, the Nudge region's alert exactly the night has no sync yet"
        decision: "spec.md:US4-S2"
      - path: crates/finder/web/src/components/NudgeBlock.test.tsx
        status: new
        disposition: add
        assertion: "the session without imaging_camera: Arrows follow offers exactly the mount and the finder camera; with it: the mount, the finder camera and the imaging camera; without mount: no Nudge region"
        decision: "spec.md:US4-S3"
      - path: crates/finder/web/e2e/nudge.spec.ts
        status: new
        disposition: add
        assertion: "the simulated console, Track off, by day: Arrows follow reads mount; Nudge right answers 200 exactly {up_arcsec 0, right_arcsec 30, moved axes} and the block's status Nudged right 30 (arcsec); after axis_move.state done and mount.moving false, Coarse then Nudge down answers exactly {up_arcsec -240, right_arcsec 0, moved axes} and Nudge up at once 409 with the block's alert the mount is already moving"
        decision: "spec.md:US4-S4"
      - path: crates/finder/web/e2e/nudge.spec.ts
        status: new
        disposition: add
        assertion: "the simulated console, Track on with the guidance block's switch until track.state tracking: Coarse then Nudge up answers 200 exactly {up_arcsec 240, right_arcsec 0, moved track}; the block's status Nudged up 4 (arcmin) with the words Track holds the new place; track.state tracking with no push"
        decision: "spec.md:US4-S5"
      - path: crates/finder/web/e2e/nudge.spec.ts
        status: new
        disposition: add
        assertion: "the Nudge block's own screenshot by day has more than 1000 off-red pixels (red above 226 or green or blue above 0); after Night mode: red and black, html data-theme night and 1.5 s, none"
        decision: "spec.md:US4-S6"
      - path: crates/finder/web/src/components/TrackControl.test.tsx
        status: existing
        disposition: preserve
        assertion: "Track's region, switch, readout and words in the guidance block as before 055"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/e2e/tracking.spec.ts
        status: existing
        disposition: preserve
        assertion: "Tracking M42 in the guidance block, its readout and its night rule, its locators held to the Track region"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "TrackControl.tsx, GoToControl.tsx, McpNote.tsx, the Imaging page and Setup are unchanged"
      - "playwright.config.ts gains only its two list entries (owner approval 6)"
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
      - crates/finder/web/src/api.ts
      - crates/finder/web/src/components/GuidanceBlock.tsx
      - crates/finder/web/src/components/NudgeBlock.test.tsx
      - crates/finder/web/src/components/NudgeBlock.tsx
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/web/dist
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US4
        disposition: merge_order
        reason: "055 depends_on_landed 051, whose US4 last built dist (f6d4f5fd)"
      - path: crates/finder/web/playwright.config.ts
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US4
        disposition: merge_order
        reason: "055 depends_on_landed 051, whose US4 last changed playwright.config.ts (f6d4f5fd)"
      - path: crates/finder/web/src/api.ts
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US4
        disposition: merge_order
        reason: "055 depends_on_landed 051, whose US4 last changed api.ts (f6d4f5fd)"
      - path: crates/finder/web/src/components/GuidanceBlock.tsx
        other_spec: 022-altaz-guidance
        other_story: US1
        disposition: merge_order
        reason: "055 depends_on_landed 022, whose US1 last changed GuidanceBlock.tsx (9e23cb1f)"
    open_questions: []
```
