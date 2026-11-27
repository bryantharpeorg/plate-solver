# Implementation Plan: An Imaging Solve Centres the Target

**Branch**: `057-imaging-solve-centres-the-target` | **Date**: 2026-11-27 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/057-imaging-solve-centres-the-target/spec.md`

## Summary

A centring brings a place, the session's target or the sky at a tapped pixel of the
latest solved imaging frame, to the imaging camera's set point, the sensor's centre,
from that frame's imaging solve alone. A new module, `crates/finder/src/centre.rs`,
serves `POST /api/imaging/centre`, reads the latest solve and frame, checks the guards,
and hands Track the place to centre and the sky at the set point. `Tracker::centre`
makes the place Track holds the place to centre and has Track's next turn make the step
from the set point's sky to it, the step 055's nudge makes, so it needs no live pointing
and is never a sync. `nudge::track_step`, the Track branch of 055's `nudge` made shared,
reserves the move and reports it in `motion` and `axis_move`. After a centring,
`Session::centred` marks the running Track for a minute, and the finder camera's first
solve in it takes the aim point from the place Track holds, so its return moves nothing.
The imaging run's mount part gains `centre`, a default method the console's part
implements, and the run asks it when a solve moved no aim point. MCP gains a `centre`
tool through the same function; the README and the glossary gain the words; the Imaging
page gains a Centre block. Six stories: the target's centring and Track's step; the tap,
the refusals and the region of interest; the run and the stack; the finder camera's
return; MCP and the words; the page.

## Technical Context

**Language/Version**: Rust, the factory's toolchain; TypeScript and React for the page
(`crates/finder/web`).

**Primary Dependencies**: none new. `axum` routes, `rmcp` tools, the console's own
`tracking`, `nudge`, `session`, `imaging` and `model` modules.

**Testing**: `cargo nextest` (the `motion` and `mcp` test binaries), Vitest and
Playwright, all under the `test` gate.

**The general rule in code.** A stack's tracking error is the sky at the set point less
the place Track holds (`tracking_error`, `crates/finder/src/imaging/steering.rs:32`,
the set point from `set_point_in_frame` at `crates/finder/src/imaging/steering.rs:568`).
The set point is the sensor's centre in the frame's own pixels
(`set_point_in_frame((sensor width, sensor height), None, roi, binning)`,
`crates/finder/src/imaging/steering.rs:58`). An imaging solve's `ra_deg` and `dec_deg`
are the frame's own centre, which is the sensor's only with no region of interest; its
matrix (`ImagingSolve::from_cd`, `crates/finder/src/imaging/solver.rs:150`, and
`sky_at`, `crates/finder/src/imaging/solver.rs:210`) gives the sky at any pixel. A
centring takes the sky at the set point by that matrix, makes the place Track holds the
place to centre and steps the telescope by the difference, so the step and every later
tracking error agree: after it, the place Track holds is what the set point should
show. The place to centre is the session's target's RA and Dec, or the sky at a tapped
pixel by the same matrix.

**The solved frame, and 058's preview frames.** A centring reads the imaging view's
latest `frame` and latest `solve`, and refuses unless the frame is the solve's (owner
decision 9). A stack's subs are the view's `frame` (the review's E4: during a stack the
draft's words named sub 4, then sub 6), and so are 058's preview frames: 058, drafted
beside this spec (its trio read at the seat's worktree, not on `main`), makes a preview
frame the view's `frame` with state `shown` and purpose `preview`, never saved, refused
by `POST /api/imaging/solve` with `the latest imaging frame was not saved`
(`crates/finder/src/imaging/solving.rs:558`), taking the console's next id, and kept
among the same last eight previews as saved frames. So after 058 a centring after any
preview frame is refused with `a newer imaging frame was taken after frame N`, the
Centre block disables its button and draws no picture until a saved frame is solved
again, and a solved frame's picture is gone after eight preview frames in any case.
The re-anchor on 058 checks this shape (§ Prerequisites). During a stack, Track stays
steady from before it while the steering moves the telescope (run, § What was run), so
a solve before a stack is stale even though Track never turned unsteady: refusing any
newer frame is the plain rule, and its words say what to do.

**Track's step.** 055's `Tracker::nudge` (`crates/finder/src/tracking.rs:393`) reads
the held place, works out the moved place, reads the counts, plans the step with
`step_path` (the counts plus the model's inverse at the moved place on arrival less
that at the held place now, each moved axis 0.5° past against Track's way, a GoTo's
guards), takes the session's model, sets the held place, clears the correction's memory
and marks the step pending (`stepping = Some(from)`,
`crates/finder/src/tracking.rs:446`); the rate loop makes it and runs the quiet window
(055's FR-010, FR-011). Everything after the moved place is the same for a centring, with
`from` the set point's sky instead of the held place. So that body becomes
`step_between(from, to, model)`, which `nudge` and the new `centre` call. The step's end
is planned as a difference of the model's inverses, so the model's zero-point errors
cancel to first order; what is left is the model's scale, about 2 % of the step on the
simulator (055's Risks), 17.5″ on a 726″ step and 23.2″ on an 834″ tap.

**What the guard sees.** An imaging solve says where the set point was during its
frame. 040's steady marks (`Tracker::steady_between`,
`crates/finder/src/tracking.rs:526`) are false for any moment Track was settling,
stepping (a nudge's or a centring's step), reversing or measuring backlash, before Track
first tracked, and while a push is read. A push is read only from the finder camera's
live pointings, and only past 5′ (038's `track_live_pointing`,
`crates/finder/src/session.rs:1178`, and `PUSH_THRESHOLD_DEG`,
`crates/finder/src/session.rs:109`). So "Track steady from the solved frame's start
until now" covers Track's own moves, an earlier centring among them, and a push only
when the finder camera sees it past 5′: with the finder camera not solving, a push by
hand passes (the review's E1: a 200″ push after the solve, centred from the stale solve,
left M42 192″ off), and with it solving a push under 5′ is corrected by Track and not
refused (E1b: 34″ off). The age bound limits how stale a solve can be: a frame that
ended more than `AGE_S`, 60 s, before the press is refused (owner decision 3).

**The finder camera's return.** A centring moves the telescope and not the aim point,
so with the aim point wrong, the finder camera's next pointings put the target off again:
under 5′ Track corrects the centring away, and past 5′ 038 reads a push, follows it and
syncs (spec.md Edge Cases). `Session::record_live_pointing`
(`crates/finder/src/session.rs:1629`) is where every live pointing reaches Track. With a
centring's mark on the running Track (`TrackRun`, `crates/finder/src/session.rs:161`),
the first pointing recorded before the mark's time, with the tracker `tracking` and
steady, first sets the aim point to the pixel of that live frame at which the place
Track holds lies (`Frame::project`), persisted as measured as 032's FR-022 persists an
aim reading, and in force at once; the pointing then read from it is the place Track
holds, so Track has nothing to correct and 038 nothing to read. A held aim point (032's
FR-036, `Session::set_measured_aim`, `crates/finder/src/session.rs:1730`) is dropped,
since this one moves nothing. A pointing after the mark's time clears it. Track turned
on again builds a new `TrackRun`, with no mark.

**The run.** `MountSteps` (`crates/finder/src/imaging/run.rs:243`) gains
`fn centre(&self) -> StepFuture<'_, Result<bool, Failure>>` with a default of
`Ok(false)`; `Script` keeps the default, so 037's and 041's scripted tests see the run
as before. The console's part answers `Stopped` after the console's Stop, as its
`repoint` does (`crates/finder/src/imaging/run/console.rs:247`), and otherwise centres
the session's target through `centre::centre` with the run's `by`. In the procedure,
after the aim point's re-centre branch (`crates/finder/src/imaging/run.rs:1251`) and
before the idle branch (`crates/finder/src/imaging/run.rs:1265`), a solve outside the
tolerance that moved no aim point, with fewer than three tries made, asks the part; a
step made counts a try, shows `centring` with the try's words, and the loop's next frame
waits for the mount to be steady (`RunTask::frame`, `crates/finder/src/imaging/run.rs:1042`,
calls `steady` at `crates/finder/src/imaging/run.rs:1045`). After three steps the offset
counts as measured: the warning or the ending is 041's for three tries.

**Performance Goals**: a centring answers at once (the route hands Track the step and
returns). On the simulator the telescope is within 25″ of the place 12 s after the step
ends.

**Constraints**: no new dependency; no change to the pointing model, the live solve,
`goto.rs` or the steering. `session.rs` gains only `TrackRun`'s mark, `Session::centred`
and the aim reading after `record_live_pointing`'s held aim point's block; `tracking.rs`
gains `centre` and moves `nudge`'s body into `step_between` without changing what a
nudge does.

**Scale/Scope**: about 2,150 added lines across six stories (round 1's with this
fold's tests, without `dist`), about 1,550 of them tests.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Detection, the plate solver and the live solve are untouched; the imaging solve is read as 032 gives it (its centre, `cd_deg` and the frame's size), and the live frame's `project` as 032's aim reading reads it. |
| II. Never weaken a gate | No existing test, golden, fixture, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS` or `ergane.yaml` is edited. `playwright.config.ts` gains two list entries, an owner approval as 050's, 051's and 055's were. The only edits to existing test files are the `mod` lines of `crates/finder/tests/motion/main.rs` and `crates/finder/tests/mcp/main.rs` (SC-002). Every existing test passed unchanged at each story's tip (run, § What was run). |
| III. Tests first, independent expectations | The oracles are the simulator's imaging truth, eyepiece and push, M42's catalogue place, the solve's own matrix at a pixel (032's), and the words of 041 and the scenarios. Each test file writes its own copy of PARTS and of the route's words; no test reads a constant of `centre.rs`. Time comes from a paused tokio clock and the simulator's virtual clock. Every story's new tests failed on the story before it (run, § What was run), but US2-S3, US2-S4, US2-S5 and US3-S2, which pass from US1 on and pin US1's FR-003 and FR-002 (§ Plan traps 4). |
| IV. Read the critical paths twice | None is touched: no `Cargo.toml` or `Cargo.lock` change, no `unsafe`, nothing under `proto/`, `crates/math-core`, `crates/pattern-database` or `crates/database-generation`, no `build.rs` or `.cargo/`. The mount's motion is a safety path: a centring is Track's own step through the same `step_path` and guards as 055's Track-on nudge, behind the same reservation; the aim point it leaves moves nothing by construction (its pointing is the place held). |
| V. Proven open source first, with honest licensing | Looked for, in local sources only (no download): the vendored `reference-solutions/` (cedar-detect and cedar-solve: no mount control), the cargo registry caches (no mount or astronomy-control crate), and the repository's own research of 2026-10-21 and 2026-11-15 (`docs/research/`), which read **N.I.N.A.**'s centering solver (solve, then a sync or an offset, then a slew, up to 10 times within 1′), **KStars/Ekos** align (solve, sync, slew, up to 10 times within 30″) and **SharpCap** (one solve, sync and GoTo). All three are programs the console cannot link (and N.I.N.A. runs on Windows only); their method, a solve and a move by its offset, is the one taken here, without their sync (053's) and with Track's own step in place of a slew. Nothing to adopt, so the step is the console's existing one. |

## Project Structure

### Documentation (this feature)

```text
specs/057-imaging-solve-centres-the-target/
├── spec.md
├── plan.md
└── tasks.md
```

### Source Code (repository root)

```text
crates/finder/src/centre.rs                      # new (US1, the target); US2 the tap and PARTS; US4 the mark
crates/finder/src/tracking.rs                    # US1: `Tracker::centre`, `step_between`
crates/finder/src/nudge.rs                       # US1: `track_step`, the nudge's Track branch through it
crates/finder/src/api.rs                         # US1: one `merge`, `centre` in the reservation
crates/finder/src/lib.rs                         # US1: `pub mod centre;`
crates/finder/src/imaging/run.rs                 # US3: `MountSteps::centre`, the doc sentence, the procedure's step
crates/finder/src/imaging/run/console.rs         # US3: `ConsoleMount::centre`
crates/finder/src/session.rs                     # US4: `TrackRun::centred`, `Session::centred`, the aim reading
crates/finder/src/mcp/tools.rs                   # US5: the `centre` tool
crates/finder/README.md, GLOSSARY.md             # US5
crates/finder/tests/motion/centre.rs             # US1, new (its rig and helpers `pub(super)`)
crates/finder/tests/motion/centre_tap.rs         # US2, new
crates/finder/tests/motion/centre_run.rs         # US3, new
crates/finder/tests/motion/centre_aim.rs         # US4, new
crates/finder/tests/mcp/centre.rs                # US5, new
crates/finder/web/src/api.ts                     # US6: `ImagingFrame`'s size, `Centred`, `postCentre`
crates/finder/web/src/components/CentreBlock.tsx, CentreBlock.test.tsx  # US6, new
crates/finder/web/src/components/ImagingPage.tsx # US6: one import, the block after the solve block
crates/finder/web/e2e/solve-centre.spec.ts       # US6, new
crates/finder/web/playwright.config.ts           # US6: two entries
crates/finder/web/dist                           # US6: the build
```

**055's files.** This spec is stacked on 055's code: `nudge.rs` exists only there, and
`tracking.rs`, `api.rs`, `mcp/tools.rs`, the README, the glossary, `api.ts`,
`ImagingPage.tsx` and `playwright.config.ts` carry 055's additions, which this spec
places its own after. In `tracking.rs` and `nudge.rs` the lines that move keep their
order and words; no nudge answers otherwise (055's tests pass unchanged, run).

**The other drafts' shared files.** 052, 053 and 054 also write the README, the
glossary and the test `main.rs` files; 054 writes `run/console.rs` and rewrites
`ImagingPage.tsx`; 053 owns syncs and the model and also writes `session.rs`, where US4
adds its mark and its aim reading. They land in the order 052, 053, 054, 055, and the
seat re-anchors this trio on whatever has landed before its flip (§ Prerequisites).

## Interfaces

### `crates/finder/src/centre.rs` (US1 new; US2 and US4 add)

```rust
//! Centring from an imaging solve (057): the session's target, or the point
//! the observer taps on the latest solved imaging frame, is brought to the
//! imaging camera's set point, the sensor's centre, with Track's own step
//! (055). The place Track holds becomes that place, and the telescope moves by
//! what the latest frame's solve says lies between it and the sky at the set
//! point. Nothing here needs a live pointing or writes the pointing model.

/// How long a solve holds, in seconds after its frame ended: a centring
/// refuses an older one, and the finder camera's first solve within it after a
/// centring takes the aim point from the place Track holds.
pub const AGE_S: f64 = 60.0;
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CentreAsk {            // US1: no field; US2: the three below
    pub frame_id: Option<u64>,
    pub x: Option<f64>,
    pub y: Option<f64>,
}
#[derive(Debug, Serialize)]
pub struct CentreView { pub frame_id: u64, pub offset_arcsec: f64, pub moved: &'static str }
/// The words for a malformed ask, the route's and the MCP tool's (US2).
pub const PARTS: &str = "centre takes nothing, to centre the target, or frame_id, x and y together";
pub fn routes() -> Router<AppState>;   // POST /api/imaging/centre: SessionWrite, sim_json, centre(.., "page", ..)
pub async fn centre(state: &AppState, by: &'static str, ask: &CentreAsk) -> Result<CentreView, ApiError>;
```

`centre` reads `state.clock.now()`, the session's snapshot and the imaging view, and
checks, in order: (US2) the ask's parts, `(None, None, None)` the target and
`(Some, Some, Some)` with finite `x` and `y` a point, else 400 PARTS; no view or no
solve, `the imaging camera has not solved a frame yet`; the view's `frame` not the
solve's frame, `a newer imaging frame was taken after frame N` with N the solve's
`frame_id`; the solve `solving` or `failed`, FR-002's words; the solve's `ra_deg`,
`dec_deg`, `cd_deg`, the frame's `width`, `height` and `start`, and the view's `camera`,
any missing, `the imaging camera has not solved a frame yet`; the place to centre ((US2)
the point's frame, `imaging frame F is not the latest solved frame`, and bounds, `the
point is outside imaging frame F`, then `sky_at((x, y))`; or the session's target, `no
target is selected`); `track` wanted; the session's tracker `tracking`;
`tracker.steady_between(frame start, now)`; and
`now - (start + frame.exposure_ms / 1000.0) > AGE_S`, `imaging frame N is more than a
minute old: take another and solve it`. Then:

```rust
let set_point = set_point_in_frame((camera.width, camera.height), None, frame.roi.into(), frame.binning.unwrap_or(1));
let from = ImagingSolve::from_cd(ra, dec, cd, width, height).sky_at(set_point);
let offset_arcsec = separation(from.0, from.1, to.0, to.1) * 3600.0;
crate::nudge::track_step(state, by, "centre", |tracker, model| async move { tracker.centre(from, to, model).await }).await?;
state.session().centred(now + AGE_S);   // US4
```

### `crates/finder/src/tracking.rs` (US1)

```rust
impl Tracker {
    /// (055) Unchanged in what it does: works out `to` and calls `step_between`.
    pub async fn nudge(&self, up: f64, right: f64, model: Model) -> Result<(), String>;
    /// Hold `to`, a J2000 place, in place of the place held, and have the rate
    /// loop's next turn step the telescope by `to` less `from`, the J2000 place
    /// an imaging solve found at the imaging camera's centre (057): Track's own
    /// step, as a nudge's. Refused as a nudge is.
    pub async fn centre(&self, from: (f64, f64), to: (f64, f64), model: Model) -> Result<(), String>;
    /// The step both a nudge and a centring make: plan it from the counts, take
    /// the session's `model`, hold `to` and mark the step pending from `from`.
    async fn step_between(&self, from: Target, to: Target, model: Model) -> Result<(), String>;
}
```

`centre` checks `may_nudge` (`crates/finder/src/tracking.rs:849`) and builds both places
from the held target with only RA and Dec changed. `step_between` is `nudge`'s body
from the axes' read onwards, unchanged.

### `crates/finder/src/nudge.rs` and `api.rs` (US1)

```rust
/// A step Track makes, through `start_motion` with `tool` (055; 057's
/// centring too): `ask` hands the step to the tracker with the session's model,
/// then `axis_move` reads `moving` until the tracker has made it, `done`, or
/// `halted` with Track's reason, and `motion` reads again what it read before.
pub(crate) async fn track_step<F, Fut>(state: &AppState, by: &'static str, tool: &'static str, ask: F) -> Result<(), ApiError>
where
    F: FnOnce(Arc<Tracker>, Model) -> Fut,
    Fut: Future<Output = Result<(), String>>;
```

Its body is `nudge`'s Track branch at `crates/finder/src/nudge.rs:214` with `"nudge"`
replaced by `tool`; that branch becomes
`track_step(state, by, "nudge", |tracker, model| async move { tracker.nudge(up, right, model).await })`.
`api.rs` gains `routes = routes.merge(crate::centre::routes());` after the nudge's merge
(`crates/finder/src/api.rs:665`) and `| "centre"` after `| "nudge"` in `start_motion`'s
reserved list (`crates/finder/src/api.rs:844`), an entry that is defensive for `centre`;
`lib.rs` gains `pub mod centre;` after `pub mod catalog;` (`crates/finder/src/lib.rs:86`).

### `crates/finder/src/session.rs` (US4)

`TrackRun` gains, after `model_due` (`crates/finder/src/session.rs:172`),
`/// Until when a centring's place may give the aim point (057).` `centred: Option<f64>`,
`None` where Track on builds the run. `Session` gains:

```rust
/// A centring (057) has brought the place Track holds to the imaging
/// camera's set point: the first live pointing recorded before `until`
/// with Track steady takes the aim point from that place, persisted as
/// measured and in force at once, since its pointing is the place held.
pub fn centred(&self, until: f64);   // sets the running Track's `centred`; no Track, nothing
```

`record_live_pointing` gains, after its held aim point's block and before it reads the
direction: when the running Track has a mark, `held` is the tracker's `target()` while
it is `tracking` and `steady()`; the mark is cleared when `time > until` or `held` is
some; when not late, `frame.project(UnitVector::from_radec(held))` gives the pixel, and
an `Aim` that validates becomes `aim`, with `aim_measured` true and `held_aim` `None`,
written with `write_state` (a failed write printed as `could not persist the centred aim
point: …` and the aim point left as it was), then the pointing is read as before.

### `crates/finder/src/imaging/run.rs` and `run/console.rs` (US3)

```rust
pub trait MountSteps: Send + Sync {
    // ... the methods at 8b7212bb, unchanged ...
    /// Step the telescope so the target comes to the imaging camera's centre,
    /// from the latest imaging solve alone, with Track's own step (057).
    /// `false` when no step was made; the default makes none.
    fn centre(&self) -> StepFuture<'_, Result<bool, Failure>> { Box::pin(async { Ok(false) }) }
}
```

The trait doc at `crates/finder/src/imaging/run.rs:377` then reads: "The five parts. The
five traits and this struct are complete from US12: no later story or spec adds a method
or a field, because the script implements them all, but `MountSteps::centre` (057),
whose default makes no step." The procedure gains `let mut stepped = false;` and, after
the re-centre branch:

```rust
if !centre.aim_moved && tries < CENTRE_TRIES {
    match self.steps.mount.centre().await {
        Ok(true) => { tries += 1; stepped = true; idle = 0;
            self.update(|run| { run.aim_moved = true; run.view.centring.as_mut().unwrap().tries = tries; });
            self.set_state("centring")?; continue; }
        Ok(false) => {}
        Err(Failure::Stopped) => return Err(Ending::Stopped),
        Err(failure) => return Err(Ending::MountFailed(failure.words().into())),
    }
}
let measured = centre.aim_moved || (stepped && tries >= CENTRE_TRIES);
```

with `measured` in place of `centre.aim_moved` in the idle test, `Warning::OffCentre`
and `Ending::NotCentred`. `ConsoleMount::centre`, after its `steady_between`
(`crates/finder/src/imaging/run/console.rs:325`), answers `Err(Failure::Stopped)` when
`self.stopped()` (`crates/finder/src/imaging/run/console.rs:113`) and otherwise
`Ok(centre::centre(&self.state, by.as_str(), &CentreAsk::default()).await.is_ok())`,
`by` the run's from its memory.

### `crates/finder/src/mcp/tools.rs` (US5)

`CentreArguments { frame_id: Option<u64>, x: Option<f64>, y: Option<f64> }`, schema
only, after `NudgeArguments` (`crates/finder/src/mcp/tools.rs:274`), and the tool
`centre` before `set_tracking` (`crates/finder/src/mcp/tools.rs:525`): it reads its
arguments as a `CentreAsk` (a failure answers PARTS), takes `limit_move`, calls
`centre::centre(&state, "mcp", &ask)` and logs the call as `centre`. Its description:
"Move a real telescope so the session's target, or the point at pixel x, y of imaging
frame frame_id, comes to the imaging camera's centre, from that frame's imaging solve
alone, as the Imaging page's Centre block does. Needs Track tracking and the latest
imaging frame solved, ended within the last minute, with nothing Track knows of moving
the telescope since it began; the place Track holds becomes that place and Track makes
the step. It is never a sync; the finder camera's first solve within a minute after it
takes the aim point from that place. Answers frame_id, offset_arcsec and moved: track."

### The README (`crates/finder/README.md`, US5)

After the nudge's MCP paragraph, before `### MCP under WSL2`
(`crates/finder/README.md:167`):

```markdown
Centring from an imaging solve has one tool:

- `centre` takes nothing, to centre the session's target, or `frame_id`, `x`
  and `y` together, to centre that pixel of that imaging frame, as `POST
  /api/imaging/centre` does. It moves a real telescope with Track on, is
  rate-limited and logged, and answers `frame_id`, `offset_arcsec` and `moved`
  (`track`), or the route's refusal as a tool error.
```

In `## API`, right before the `POST /api/goto` row (`crates/finder/README.md:326`):

```markdown
| `POST /api/imaging/centre` | JSON `{}`, or `{"frame_id":N,"x":X,"y":Y}` with X and Y a pixel of imaging frame N. Bring the session's target, or that pixel's place, to the sensor's centre from the latest imaging solve, with Track on; answers 200 with `frame_id`, `offset_arcsec` (how far that place was from the sky at the sensor's centre) and `moved` (`track`), or 400, 409 or 415 with a JSON `error`. Same-origin writes only. |
```

After `## Nudging the telescope`'s text, before `## Recording a night`
(`crates/finder/README.md:670`):

```markdown
## Centring from an imaging solve

A centring brings a place to the sensor's centre from the latest imaging solve
alone: the session's target, or a point tapped on the solved frame's picture in
the Imaging page's Centre block. It needs no live pointing, so it works when the
finder camera cannot solve. Take an imaging frame and solve it with Track on,
then centre within a minute: the place Track holds becomes that place and Track
makes the step, as a nudge's. The solve's frame must be the latest frame, or the
centring is refused with `a newer imaging frame was taken after frame N`. Track
must have been steady since that frame began, or it is refused with
`the telescope moved after imaging frame N: take another and solve it`. Track
knows of its own settling and steps, and of a push only while the finder camera
solves. A centring is never a sync. The finder camera's first solve within a
minute after it takes the aim point from the place centred, so Track holds the
target where the centring put it, and a later GoTo lands there too. An imaging
run centres the same way when a solve measures no new aim point. Without the
finder camera's solves Track holds the place on the mount's rates alone and may
drift: take another frame and centre again, or let a stack's steering hold it.
```

### The glossary (`GLOSSARY.md`, US5)

After **Nudge** (`GLOSSARY.md:277`), before **Purpose label** (`GLOSSARY.md:281`):
`**Centring from an imaging solve**:`, then on the next line FR-013's words, then
`_Avoid_: recentre (the imaging run's Track off and on to a new aim point), slew to
solve, plate-solve sync`. The name is not **Centring** alone: the imaging run's
`centring` state and the **Imaging run** entry's "centring the target on the imaging
camera's sensor" name both of the run's paths.

### The page (`crates/finder/web/src/`, US6)

`api.ts`: `ImagingFrame` gains `width?: number; height?: number;` after `start?`;
`interface Centred { frame_id: number; offset_arcsec: number; moved: "track" }` and
`postCentre(point?: { frame_id: number; x: number; y: number }): Promise<Centred>`,
posting `JSON.stringify(point ?? {})` with `Content-Type: application/json`, before
`postRecord`. `CentreBlock({ view, session, onChange })`: a `section` labelled by its
`h2` `Centre`; the quiet line "With Track on, take an imaging frame and solve it, then
centre the target or tap a point of the solved frame. It needs no solve of the finder
camera."; the button; the picture in a `border border-ink bg-photo` box with the
`photo` class (019's night filter); `role="status"` and `role="alert"` lines. The
solved frame is `view.frame` when it is `saved` and the solve is `solved` for its id. An
answered centring is held as `{ what, offset, seen: false }`; an effect sets `seen` once
a session's `axis_move` with `tool` `centre` reads `moving`; the status reads `Centring
<what>: <offset> from the centre…` while not `seen` or while `moving`, then `Centring
stopped: <reason>` when `halted`, else `Centred <what> · Track holds it. Take another
imaging frame to check.` `ImagingPage.tsx` imports it after `ImagingSolveBlock` and
renders `<CentreBlock view={view} session={session} onChange={() => { onChange?.(); void refresh(); }} />`
right after `<ImagingSolveBlock …/>` (`crates/finder/web/src/components/ImagingPage.tsx:309`).

## What was run and what was read

All on the probe branches `probe/057`, cut from `probe/055-r1` at 8b7212bb, and
`probe/057-r1`, from this trio at 4279aeaf, on the simulator, with the factory's
toolchain, `--locked`, and npm from its cache. Logs are outside the repository. The
scenario numbers here are the trio's as it now stands: the earlier rounds' US1-S3 and
US1-S2 are US2-S3 and US2-S4.

**The draft's round (979a2573 to 1a3f06be).** Five stories built with every scenario's
test, then 00c4a55a (the tests in the draft's layout, the browser test renamed to sort
after `goto.spec.ts`) and 1a3f06be (the README's section). 3a46ca58 is PROBE ONLY: the
candidates' measurements. Four trials at imaging angles 17°, 100°, 250° and 17°, 120″ to
729″ off, each pushed and with the aim point off, the finder camera's frames stopped:
the general rule's step left 4.2″ to 17.5″ and 6.9″ to 14.1″; moving the held place by
the offset left 3.7″ to 17.6″ and 6.2″ to 12.9″; one check frame reached 90″ in every
trial. Stacks of 25 subs from 120″: the general rule 4.8″ to 1.4″ and 9.0″ to 1.3″; the
moved place 3.7″ to 74.0″ and 7.4″ to 76.4″. The aim point moved by the offset with no
solves: 394″ to 403″ and 481″. Today's run: 386″, 242″, 114″ with tries 0; with the
run's step 13.8″, 14.7″, 5.3″ after one try. Taps of 834″, 1,047″ and 679″: 23.2″,
13.2″, 7.6″. The gates passed on 7ddbaf0d but the browser suite's order (§ Plan traps 6).

**The cynical review's round 1 (on the draft, its own worktree).** E1: a 200″ push after
the solve with the frames stopped, centred from the stale solve, 192″ off; E1b: with the
frames running, 34″ off. E2: the finder camera's return at 390″ (below). E3: the
solve's age, 10.3″ to 90.2″ off after waits of 0 to 180 s. E4: during a stack a
centring named sub 4, then sub 6. E5: a frame with a region, 538.6″ off; A1, the step
from the set point's sky, 6.6″. E7: Track off and on after a centring, 38.6″ off with the
frames stopped and 244″ with them running. E9: Stop 0.5 s into the step, `axis_move`
`{"reason": "stopped", "state": "halted", "tool": "centre"}` and Track off. E11: a
nudge after the frame refused as moved. Mutations M5b, M6c, M6d, M16 and P5 survived.

**This fold (7e287fe2 to 99875422).** 7e287fe2 is PROBE ONLY: the fold's measurements.
Then each story rebuilt from 8b7212bb's files, each on the one before: 396fa3b6 US1,
65d9fefb US2, 01f3d401 US3, 70849fa7 US4, 0a6e103c US5, 99875422 US6 (with `dist`).

**The finder camera's return (7e287fe2, E2's setup: the centring rig at 17°, the aim
point 390″ or 240″ off at 30°, centred with the frames stopped, then started).**
- (i), as drafted, at 390″: 5 s after, a push `moving` and `slip_pending` true; 10 s
  after, followed, the place Track holds 382.4″ from M42, syncs 2 to 3, epoch 1 to 2;
  M42 7.4″ to 12.5″ off to 60 s; a stack of 8 subs then did not steer (source `tracker`,
  its error 390.3″); Track off and on landed 6.9″ off, the aim point measured from the
  subs. At 240″: 190.0″, 212.2″, 254.4″, 245.8″ and 237.7″ off 5, 10, 15, 30 and 60 s
  after; the stack did not steer (241.3″); Track off and on 7.2″. With the frames never
  stopped, at 240″: 202.1″ off once it rested and 242″ to 245″ 30 s later.
- (ii), 038's check re-armed only within 5′ of the new place, at 390″: 285.7″, 295.5″,
  322.8″, 391.5″ and 390.6″, no sync; the stack 391.3″; Track off and on 2.9″. At 240″
  as (i).
- (iii), the aim point from the place held, at 390″: 8.4″, 10.3″, 12.4″, 7.8″ and 8.0″;
  no push; syncs 2 and epoch 1 throughout; the aim point (700.24, 420.13), 0.27 pixels
  from the true (700, 420); a stack of 8 subs steered to 1.2″; Track off and on 9.6″.
  At 240″: 7.6″, 10.2″, 12.9″, 6.9″ and 5.5″; the aim point (700.21, 419.95); the stack
  1.8″; Track off and on 11.9″. With the frames never stopped: 4.6″ once it rested and
  3.5″ to 6.4″ after, the aim point (700.05, 420.10). With the frames back 60 s and
  180 s after the centring: 34.5″ and 95.8″ off by drift, the aim point 1.18 and 3.23
  pixels off, M42 34.8″ and 95.7″ off 30 s later (at 180 s a sync was recorded, the
  epoch unchanged: not the centring's, but a live solve's frame sync kept past a 120 s
  gap, round 1 below); (i) at those gaps followed, the place held 365.2″ and 337.5″ off.
- **The age (F3).** With no finder camera solve, pressed 0.2, 30.2, 45.2, 60.2, 90.2,
  120.2 and 180.2 s after the frame ended: M42 10.3″, 23.0″, 30.3″, 37.3″, 49.7″, 63.7″
  and 90.2″ off once it rested.
- **A solve before a stack (F4).** M42 114.8″ off before the stack and 80.5″ after 10
  subs, the steering's source `subs`, `steady_between` true throughout; the draft route
  answered `imaging frame 11 is not solved yet: solve it first`; a step from the
  pre-stack solve, made 20 s into the stack, left M42 20.3″ off, overshooting by the
  steering's 34″ before the steering pulled it back. On 99875422, after 10 subs (the
  view's frame 11, purpose `sub`), the route answered 409 `a newer imaging frame was
  taken after frame 1`.
- **A region of interest (F5).** The draft's step from the frame's centre: 538.6″ off at
  17° and 549.2″ at 250°; with A1, US2-S3 passes.

**Each new test failed on the story before it** (logs named `fold-r1-2*`): US1-S1,
US2-S3 and US2-S4 on 8b7212bb (405); US2-S1 and US2-S2 on US1 (400, unknown fields);
US3's on US2 (the file did not compile: no `MountSteps::centre`), and US3-S1's push and
aim cases each alone (tries 0, not 1); US4-S1 on US3 (at 390″ a push read) and US4-S2
(the aim point not measured); US5-S1 and US5-S2 on US4 (the tool unlisted, no README
section); the Vitest file on US5 (no `CentreBlock`), and the browser test on US5 (no
Centre block). US3-S2 passes from US1 (§ Plan traps 4).

**The gates at each story's tip** (`lint` and `test`, `npm ci --offline`, the dist
check against the tip's tree): every step exited 0 at each of the six
tips: fmt, clippy, both doc builds, nextest (1,548, 1,550, 1,554, 1,556, 1,558 and 1,558
tests, all passing), doc tests, typecheck, Prettier, Vitest (611 tests at US1 to US5,
615 at US6), the build with `dist` clean (and the gate's own `git status --porcelain --
dist` empty at US6), and `CI=1 npm run e2e` (38 of 38 at US1 to US5, 40 of 40 at US6).
Before US6's commit its tap bound of 3 failed once (3.117) and became 4 (§ Plan traps
8).

**Mutations on 99875422** (each reverted after). The review's survivors now bite: M5b
(Track steady now, not since the frame began) fails US2-S4; M6c (three steps not
counted as measured) and M6d (the try's words dropped) fail US3-S3; M16 (the console's
Stop ignored) fails US3-S4; P5 (no `halted` words) fails US6-S1. M4 (the held place
kept) fails US2-S1 and passes US1-S1 (§ Plan traps 4). The fold's checks: the
newer-frame check removed fails US2-S2; the age made ten minutes fails US2-S2; the
frame's centre in place of the set point fails US2-S3; no mark fails US4-S1 and
US4-S2; a mark never late fails US4-S2; the API row after GoTo's, and the glossary
entry under its old name, fail US5-S2; `Centred` before the step is seen fails US6-S1
and US6-S2.

**Round 1 of a probe from this text alone (854943aa to ba98d527).** Each story built
from the trio alone, tests first, each on the one before: 854943aa and eeeadb66 US1,
abf27e35 and e74fd932 US2, a558378f and fd3800a0 US3, 3a7f43a9 and cf23e9c8 US4,
1a359052 and c6bb5b34 US5, 5ec4df0e, ee576397 and ba98d527 US6. Every story's tests
failed before its code as tasks.md said and passed on its first build; every gate passed
at each tip (nextest 1,548, 1,550, 1,554, 1,556, 1,558 and 1,558; Vitest 611 at US1 to
US5 and 615 at US6; `dist` clean; `CI=1 npm run e2e` 38 of 38 at US1 to US5 and 40 of 40
at US6), and no earlier test was edited. Every claim of § Read from the diff held. Its
10 clarity defects are folded here: FR-002's order read literally (R1); the frame's
check before the solve's state (R2) and the held aim point dropped (R7), each a survivor
of its mutation and each passing on the code and failing under it as a throwaway, now
US2-S5 and US4-S3; a development command that ran no tests (R3); US1 at 29,852 B, 725 B
over the line (R4, § Plan traps 1); ψ's sense (R5); US6-S3's third view (R6); the
Independent Test commands (R8); `npx playwright test` beside the e2e script's lock (R9);
and US6-S5 reading the first frame's statuses on the second, which failed 1 run in 4
(R10, fixed in ba98d527, then 3 of 3). Its other survivors are as § Plan traps 4 says:
in US1 the reservation's entry (defensive), the age bound and the held place, which bite
US2-S2 and US2-S1. **The finder camera's return after the minute** (a throwaway at
cf23e9c8, the aim point off at 30°, the frames started again after a centring): at 390″
back about 15 s after, the aim point taken a frame later, syncs 2, epoch 1, no push, M42
6.9″ off; at 240″ back about 75 s after, nothing taken and no sync, M42 pulled back to
239.5″ off; at 240″ back about 195 s after, `model.syncs` 2 to 3 at the first pointing,
epoch 1, no slip, no push, M42 243.7″ off, because every live solve is a frame sync and
`Model::add_sync` (`crates/finder/src/model.rs:524`) replaces a same-epoch sync within
120 s and 2° but keeps one 120 s or more apart beside it; at 390″ back about 195 s
after, a push `moving` and `slip_pending` at the return, then `following`, syncs 3 and
epoch 2 about 5 s later, M42 97.9″ off.

**This fold of round 1 (2e651943 to f7d6779a, on `probe/057-r1`).** 2e651943 moves
US1-S3 and US1-S2 to `centre_tap.rs` as US2-S3 and US2-S4, unchanged, with `refused`,
`moved` and their words; 1488b564 adds US2-S5; f7d6779a adds US4-S3. US1's tree with the
fold's changes passes `cargo clippy -p finder --all-targets --all-features --locked --
-D warnings`; US2's test file on US1's code fails US2-S1 and US2-S2 (400, unknown
fields) and passes US2-S3, US2-S4, US2-S5 and US1-S1. With `session.rs` and `centre.rs`
as at US3's tip, US4-S1, US4-S2 and US4-S3 fail. Each step's mutation, made once, fails
its own test alone: the frame's check after the solve's state fails US2-S5; the frame's
centre as the set point fails US2-S3; Track steady now, not since the frame began, fails
US2-S4; the held aim point kept fails US4-S3. The sizes are § Plan traps 1's. At
f7d6779a every gate step passed: fmt, clippy, both doc builds, nextest 1,560 of 1,560,
doc tests, `cargo deny --offline`, `cargo machete`, `npm ci --offline`, typecheck,
Prettier, Vitest 615 of 615, the build with `git status --porcelain -- dist` empty, and
`CI=1 npm run e2e` 40 of 40; and each story's development commands of § Refinement
Record passed.

**Read, not run.** A centring refused for a latest frame that did not solve, for Track
armed, in its keyhole wait or measuring backlash, during a direct axis move (the
reservation's entry is defensive), or across a GoTo's guards; a mirrored imaging frame;
058's preview frames (058 is not built here); a centring's `the mount is already moving`;
a real mount and cameras.

## Read from the diff, not from a test

- `tracking.rs`'s `step_between` is `nudge`'s body from the axes' read onwards, in the
  same order; `nudge` only works out `to` first.
- `nudge.rs`'s `track_step` is the nudge's Track branch with its tool a parameter.
- `run.rs` changes only the trait's one default method, its doc sentence, and the
  procedure's lines of § Interfaces; `Script` is not written.
- `session.rs` changes only `TrackRun`'s field, its `None`, `Session::centred` and one
  block after `record_live_pointing`'s held aim point's block; the held aim point's
  block and everything after the new block are as at 8b7212bb.
- The MCP tool's description has § Interfaces' words; its test pins `real telescope`.
- The README's new paragraph, row and section keep every sentence at 8b7212bb.
- `CentreBlock` posts only `postCentre`; `ImagingPage` gains one import and one element.

## Shapes that existing tests pin

- **055's nudge with Track on.** `crates/finder/tests/motion/nudge_track.rs` and
  `crates/finder/tests/motion/nudge.rs` pass unchanged with `step_between` and
  `track_step` (run).
- **The run's centring.** `crates/finder/tests/motion/imaging_run_centring.rs` (041)
  passes unchanged: `Script` keeps the default, so a solve that moved no aim point is
  idle as before (run).
- **The aim point and the push.** 032's held aim reading
  (`crates/finder/tests/motion/imaging_track.rs`) and 038's push tests
  (`crates/finder/tests/motion/tracking/push.rs`) pass unchanged: with no centring no
  `TrackRun` has a mark, so `record_live_pointing` reads as at 8b7212bb (run).
- **The reservation.** 016's and 055's tests of `the mount is already moving` pass with
  `centre` added (run).
- **The README's MCP count.** `crates/finder/tests/console/imaging_run_docs.rs:139`
  asserts "sixteen tools of its own" (`crates/finder/README.md:88`): the tool is a
  paragraph of its own (run). 055's `crates/finder/tests/mcp/nudge.rs:209` finds
  `## Nudging the telescope` before `## Recording a night`, which stays true (run).
- **The Imaging page.** `ImagingPage.test.tsx` and `imaging.spec.ts` pass unchanged: the
  block's picture is named `Solved imaging frame N: …`, not `Imaging frame …`, which
  `crates/finder/web/e2e/imaging.spec.ts:42` reads (run).
- **The simulated console's state.** `solve-centre.spec.ts` sorts after
  `simulate.spec.ts` and before `stack.spec.ts`; it sets the session as
  `tracking.spec.ts` does, turns Track off and the target to null at the end, and the
  specs after it pass (run).

## Decisions and the alternatives rejected

| Decision | Chosen | Rejected, and why |
|---|---|---|
| What moves | the place Track holds becomes the place to centre; Track steps (decision 3) | the place moved by the offset (a stack pulls it to 74″); the aim point (nothing moves without solves); a sync (053's) |
| Where to | the sensor's set point in the frame's pixels (A1) | the frame's centre (538.6″ off with a region set) |
| The step | `step_between`, 055's step | a GoTo with Track off and on (the run's re-centre: it needs a new aim point); feeding imaging solves to Track (the review's E8: five frames against one step) |
| The guard for movement | Track steady since the solved frame began, and the frame ended within 60 s | the guard alone (a push the finder camera does not see passes, and drift grows 0.45″ a second); 30 s or 2 minutes (23.0″ and 63.7″) |
| The finder camera's return | its first solve within the minute takes the aim point from the place held (iii) | as drafted (i: a push, a sync and a new epoch past 5′, corrected away under it); re-arming only within 5′ (ii: pulled to 392″) |
| The solve read | the latest frame's; any newer frame refused plainly (decision 9) | the latest saved frame's (a stack's steering makes it stale while Track stays steady); the draft's words (untrue after a sub) |
| Track off | refused (decision 6) | a direct move (stale in seconds); Track on first (its GoTo moves the frame) |
| The run | the mount part's new default method | a new part or a new `Steps` field (more API; `Script` would need it) |
| The tool | through `centre::centre` | through the route (the tool needs `by` `mcp`) |
| The page | a Centre block with its own picture (decision 7) | a tap on the Imaging frame block's picture (054 rewrites it; `crates/finder/web/e2e/imaging.spec.ts:42` pins its name) |
| The page's status | `Centring …` until a session shows the step `moving` | from the answer's session (a read before the step reads `Centred` at once) |

## User Story 1 - The target's centring and Track's step

`centre.rs` without the tap and without the mark, `Tracker::centre` and
`step_between`, `track_step`, the route's merge, the reservation, `pub mod centre;`,
and the tests of spec.md US1.

## User Story 2 - The tap and the later refusals

`CentreAsk`'s three fields, `PARTS`, the point's checks and `sky_at`, and the tests of
spec.md US2.

## User Story 3 - The run and the stack

`MountSteps::centre` and its doc sentence, the procedure's step, `ConsoleMount::centre`,
and the tests of spec.md US3.

## User Story 4 - The finder camera's return

`TrackRun::centred`, `Session::centred`, the aim reading in `record_live_pointing`,
`centre`'s call of `centred`, and the tests of spec.md US4.

## User Story 5 - MCP and the words

The tool, the README's paragraph, row and section, the glossary, and the tests of
spec.md US5.

## User Story 6 - The page

`ImagingFrame`'s size, `postCentre`, `CentreBlock`, its line in `ImagingPage`, the config
entries, `dist`, and the tests of spec.md US6.

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

- Waits for 055 to land (052, 053 and 054 before it, in that order) and the seat's
  re-anchor of this trio on that `main`: every `path:line` read again; 052 to 055 added
  to `depends_on_landed`; `tracking.rs`'s and `nudge.rs`'s moves placed against 055's
  landed text; the README, `GLOSSARY.md` and the test `main.rs` additions placed after
  theirs; `run/console.rs`'s `centre` placed against 054's; the Centre block placed on
  054's rewritten `ImagingPage.tsx` after its Imaging solve block; US4's block placed in
  053's `record_live_pointing` and `TrackRun`, read again for any sync 053 makes there;
  and the Refinement Record's last writers read again.
- Waits for 058 only if it lands first: read its landed text for the shape § Technical
  Context settles (a preview frame is the view's `frame`, `shown`, never solved, the
  next id, the same eight previews). If it holds, FR-002 and FR-014 stand and the Edge
  Case on preview frames is run once by hand on the re-anchored base (a preview after a
  solve, then `{}`: 409 with `a newer imaging frame was taken after frame N`; the Centre
  block disabled and without its picture). If it does not, the solved frame's definition
  is reopened as owner decision 9's (b) before the flip.
- Waits for the owner's answers to the nine decisions, at the flip ask.
- Met: a probe round from this text alone, folded. Round 1 on `probe/057-r1` (tip
  ba98d527, 0 blocking, 10 clarity), folded here with its proofs at f7d6779a (§ What
  was run and what was read).
- Waits for the re-anchor: the page half run once on the re-anchored base (`CI=1 npm run
  e2e`). Met on `probe/057-r1` at each story's tip and at f7d6779a (40 of 40).
- Met: no operator-staged file is needed (every test builds its rig from the
  repository's own simulator and fixtures).

## Outside the factory

Written here and not run by any gate. **On the next field night**, with the owner at the
telescope: GoTo M42, turn Track on, open the Imaging page, take an imaging frame and
solve it, and press `Centre M42` within a minute; take another frame and solve it, and
repeat until the Imaging solve block puts M42 within 90″ of the centre. Note how far each
solve put M42 from the centre, how many frames it took, and whether any centring was
refused and its words. If the finder camera solves again, note whether the target
stayed centred and the aim point's `source`. Then tap a star near a corner of the solved
frame and note how far the next solve puts it from the centre. Then start an imaging run
on M42 and note its `centring.tries` and its offset at `stacking`. The counts and the
notes go in the night's report; nothing in the repository waits for them.

## Plan traps

1. **The story sizes.** Measured on `probe/057-r1` with this fold's changes in place at
   each story's tip, each diff against the story before without `dist` (US1 from
   4279aeaf): US1 27,559 B (42.1 % of 65,536; 56.8 % at 1.35 times, 37,205 B against
   39,321), US2 13,551 B (20.7 %; 27.9 %), US3 11,960 B (18.2 %; 24.6 %), US4 9,547 B
   (14.6 %; 19.7 %), US5 22,093 B (33.7 %; 45.5 %), US6 21,382 B plus `dist`'s 697 B
   (32.6 %; 44.0 %). Built from this text US1 measured 29,852 B, 725 B over 29,127 B
   (39,321 at 1.35): its rig's test file, and `nudge.rs`'s moved branch, which counts
   twice. Its fallback is taken: US1-S3 moved to US2 (28,961 B, 166 B under), and, to
   clear the line by at least 1 KiB, US1-S2 with the helpers only it used: US1 is now
   1,568 B under. US1 keeps its centring (US1-S1); the newer-frame, solving and
   no-target asks are US2-S2's, the region of interest US2-S3 and the guards' asks
   US2-S4. No scenario is left in US1 to move.
2. **US1's helpers.** US2's, US3's and US4's tests use US1's rig from `super::centre`,
   and US2's the `send` and `write` of 055's `crates/finder/tests/motion/nudge.rs`. US1
   writes all of these `pub(super)` from the start, as no story may edit an earlier
   story's test file: the struct `Rig` with its fields `app`, `state` and `sim`; its
   methods `new`, `now`, `call`, `get`, `wait`, `sleep`, `off`, `solve`, `track_on`,
   `rests`, `displace` and `held`; the enum `Cause`; the functions `sep` and `m42`; and
   the constants `CENTRE` and `FINDER_ARCSEC_PER_PX`. US1-S1 uses every one, so none is
   dead code in US1. US2's file writes its own `refused` (with 055's `write`), `moved`
   and words.
3. **Track must settle before the frame.** A frame begun within Track's settle after
   Track on, or during a step, is refused as moved. The rigs wait 30 s after Track on
   and 12 s after a step; the browser test waits 12 s after `tracking` (on the probe a
   frame taken at once after Track on was refused).
4. **US2-S3 to US2-S5 and US3-S2 pass from US1.** US2-S3, US2-S4 and US2-S5 pin US1's
   set point (FR-003), its guards (FR-002) and its frame's check before the solve's
   state, placed in US2 for US1's size (§ Plan traps 1): US1's code passes them, and
   US2-S1 and US2-S2 are US2's failing tests (run). US3-S2 pins FR-003 under a stack
   (decision 3's evidence: without the general rule's held place a stack pulls the
   target 65″ off), placed in US3 for US1's size; it fails on 8b7212bb, where there is
   no route. US3-S1, US3-S3 and US3-S4 are the story's failing tests. Likewise US2-S1
   pins FR-003's held place (the review's M4: a step that keeps the held place passes
   US1 and fails US2-S1).
5. **The finder camera's frames.** The rigs stop the node's star frames (`stop_frames`)
   to stand for a finder camera that cannot solve; the imaging camera's frames go on.
   US4's cases start them again (`start_frames`) after the centring rests, or never stop
   them (`aim_off`, with the aim point set off and Track on for 30 s).
6. **The browser suite's order.** The simulate project runs its files in name order on
   one server. A file that leaves Track's `Tracking ended` words in the guidance block
   breaks `goto.spec.ts`'s single status locator if it sorts before it; the file is
   `solve-centre.spec.ts` for that reason, and `--repeat-each` on one server breaks it
   the same way.
7. **The rate limit in US5-S1.** A refused call takes a slot; wait 1.1 s after the first
   call.
8. **The tap's bound in US6-S5.** A click lands on a whole CSS pixel, about 3.3 of the
   frame's at 412 CSS pixels wide, so the posted pixel is within 4 of the quarter point,
   not 3 (3.117 was seen).
9. **US3-S3's mount part.** A struct of the test's own wraps `Script::steps()`'s mount,
   forwarding `target`, `point`, `state`, `repoint`, `stop`, `track_on` and
   `steady_between`; its `centre` counts its calls, sets the script's mount
   `Tracking { steady: false }` on the first (`Script::set_mount`) and answers
   `Ok(true)`, so the run's view shows the try's words while it waits.
10. **US4's two causes.** With the frames stopped at 390″ the aim point is set off and
    the frames stopped (`displace`); with the frames running at 240″, under 5′, it is
    set off and never stopped, so Track corrects the centring away as the first
    pointing after the rest comes unless the mark has taken the aim point first.

## Risks

| Risk | Effect | Mitigation |
|---|---|---|
| The one-sync model's scale | a long step misses by about 3 % of its size | within the bounds; the next solve shows it, and the run takes another try |
| A push by hand the finder camera does not see | a centring from a stale solve | the minute bounds it; the README says what Track knows of |
| The finder camera back after more than a minute | (i)'s return: Track's correction undoes the centring; under 5′ corrected away (one more sync past a 120 s gap), past it a push followed, a sync and a new epoch | the next centring takes the aim point at once (US4-S2); the run measures it when the finder camera solves |
| The finder camera back within the minute after drift | the aim point taken off by the drift since (1.2 pixels at 60 s) | the drift is what a minute allows, against the night's 13 pixels |
| Drift with no finder camera solves | about 0.45″ a second on the simulator | a stack's steering holds it (US3-S2); the README says to centre again |
| A step made between two of the page's reads | the status stays `Centring …` | a step lasts several reads; on the simulated console it was always seen |
| 053 rewrites `record_live_pointing` or `TrackRun` | US4's block moves | one block and one field; re-anchored before the flip |
| 054 rewrites `ImagingPage.tsx` and `run/console.rs` | the block's place and `centre`'s place move | one element and one method; re-anchored before the flip |
| 055 changes before it lands | the moved bodies move with it | re-anchored on 055's landed text before the flip |

## Refinement Record

Drafted on 2026-11-27 by the plate-solver operator seat's spec session against 055's
probe code (`probe/055-r1` at 8b7212bb, `main` at 9c4a0a15 under it), from a probe of
that day on `probe/057`, and folded the same day with the cynical review's round 1: six
stories rebuilt on that branch, each on the one before, with every scenario's test,
both gates passing at each story's tip, every story's new tests failing on the story
before it, the fold's options measured on the simulator, the review's surviving
mutations biting, and the page run in a browser by day and by night; then probed from
this text alone (round 1 on `probe/057-r1`, tip ba98d527: 0 blocking, 10 clarity) and
folded, its proofs on that branch at f7d6779a. Writes come from
the task text. Shared writes name the spec story that last wrote the same file on
`main`; within the spec, each story's files written by an earlier one are ordered by
`depends_on_merged`. 055's own writes (`nudge.rs` and its additions to the shared files)
are not on `main`; they are named again at the re-anchor (§ Prerequisites).

None of the nine owner decisions of spec.md is answered; this record is written to the
recommendations. Its `open_questions` are empty because the validator refuses a record
that holds any; what is open is spec.md's owner decisions 1 to 9 and § Prerequisites
(the re-anchoring after 052 to 055, and 058's shape). Nothing in this record is an owner
approval: those are in spec.md, and flipping to `ready` gives them.

```yaml
version: 1
reviewed_revision: 8b7212bb8d4c52bb2daf56f9f379dd2f9116ab7f
stories:
  US1:
    tests:
      - path: crates/finder/tests/motion/centre.rs
        status: new
        disposition: add
        assertion: "the centring rig at 17 pushed 390 at 30, at 100 pushed 120 at 120, at 250 pushed 240 at 210, and at 100 with the aim point 240 off at 300, each new, frames stopped: off M42 within 15 of D; a solve; centre {} answers 200 with the solve's frame_id, moved track and offset_arcsec within 5 of off M42; at once axis_move exactly {state moving, tool centre} and motion.tool centre; once it rests off M42 at most 25, axis_move exactly {state done, tool centre}, motion as before, the place Track holds within 0.01 arcsec of M42, aim, model.syncs and model.epoch as before, slip_pending false; the next solve's centre within 25 of M42"
        decision: "spec.md:US1-S1"
      - path: crates/finder/tests/motion/nudge_track.rs
        status: existing
        disposition: preserve
        assertion: "055's Track-on nudge: the place held moves by the step, Track makes it within 10 s, never a push or a slip, the epoch unchanged, with nudge's body now in step_between and its Track branch in track_step"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/motion/nudge.rs
        status: existing
        disposition: preserve
        assertion: "055's presses, reservation and refusals as at 8b7212bb, start_motion's list gaining centre"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "Tracker::nudge answers, refuses and steps as at 8b7212bb; step_between is its body from the axes' read onwards"
      - "the nudge route's and the MCP nudge's answers, motion and axis_move as at 8b7212bb"
      - "the pointing model, the aim point, the live solve, session.rs, goto.rs and the steering are not written"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test motion -E 'test(/^centre::/) | test(/^nudge_track::/)'"
        - "cargo nextest run -p finder --locked --test motion -E 'test(/^nudge::/)'"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs and the night of plan.md Outside the factory"
    writes:
      - crates/finder/src/api.rs
      - crates/finder/src/centre.rs
      - crates/finder/src/lib.rs
      - crates/finder/src/nudge.rs
      - crates/finder/src/tracking.rs
      - crates/finder/tests/motion/centre.rs
      - crates/finder/tests/motion/main.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/api.rs
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US2
        disposition: merge_order
        reason: "057 depends_on_landed 051, whose US2 last changed api.rs on main (f9706cc0); 055 adds to it before 057 lands"
      - path: crates/finder/src/lib.rs
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US1
        disposition: merge_order
        reason: "057 depends_on_landed 051, whose US1 last changed lib.rs on main (4c39350e); 055 adds to it before 057 lands"
      - path: crates/finder/src/tracking.rs
        other_spec: 040-stack-waits-for-steady
        other_story: US6
        disposition: merge_order
        reason: "057 depends_on_landed 040, whose US6 last changed tracking.rs on main (f1847647); 055 adds to it before 057 lands"
      - path: crates/finder/tests/motion/main.rs
        other_spec: 050-tools-page-aligns-finder-camera
        other_story: US3
        disposition: merge_order
        reason: "057 depends_on_landed 050, whose US3 last changed motion/main.rs on main (6b70a4a2)"
    open_questions: []
  US2:
    tests:
      - path: crates/finder/tests/motion/centre_tap.rs
        status: new
        disposition: add
        assertion: "the centring rig at 17, 100 and 250, each pushed 0 (Track on at the true eyepiece, frames stopped): a solve; P the sky at pixel (300, 200), (1000, 700) and (618, 100) by ImagingSolve::from_cd of the solve's ra_deg, dec_deg and cd_deg with 1236 x 822 and sky_at; centre {frame_id F, x X, y Y} answers 200 with offset_arcsec within 5 of off P; once it rests off P at most 30, the place Track holds within 0.01 arcsec of P (FR-003's held place, which US1 does not discriminate), track.target.id still NGC 1976"
        decision: "spec.md:US2-S1"
      - path: crates/finder/tests/motion/centre_tap.rs
        status: new
        disposition: add
        assertion: "the bare console {} 409 exactly the imaging camera has not solved a frame yet; the rig at 17: a foreign write 403, text/plain 415, {z 1} 400; solve A with Track off: {frame_id A} and {frame_id A, x 1.0} 400 exactly PARTS; Track on, 15 s, solve B: {frame_id B, x 1237.0, y 10.0} 409 exactly the point is outside imaging frame B; {frame_id A, x 10.0, y 10.0} 409 exactly imaging frame A is not the latest solved frame; POST /api/imaging/frame C, at once {} 409 exactly a newer imaging frame was taken after frame B; C saved, solve posted, at once {} 409 exactly the imaging camera's solve is still running; C solved, target null, {} 409 exactly no target is selected; 61 s, {frame_id C, x 618.0, y 300.0} 409 exactly imaging frame C is more than a minute old: take another and solve it; solve D, {frame_id D, x 618.0, y 300.0} 200, then at once 409 exactly the telescope moved after imaging frame D: take another and solve it"
        decision: "spec.md:US2-S2"
      - path: crates/finder/tests/motion/centre_tap.rs
        status: new
        disposition: add
        assertion: "the centring rig at 17 pushed 0, PUT /api/imaging/settings roi 824 x 548 at (0, 0): a solve; centre {} answers 200 with offset_arcsec within 5 of off M42; once it rests off M42 (the sensor's true centre) at most 25; it pins US1's set point (FR-003) and passes from US1 on"
        decision: "spec.md:US2-S3"
      - path: crates/finder/tests/motion/centre_tap.rs
        status: new
        disposition: add
        assertion: "the centring rig at 17: {} before a solve 409 exactly the imaging camera has not solved a frame yet; a solve with Track off, 409 exactly Track is off: turn Track on to centre; Track on until tracking, 409 exactly the telescope moved after imaging frame N: take another and solve it with N the first solve's; a solve at once (begun in Track's settle), 15 s, 409 the same with N that solve's; a solve, 200, then at once 409 the same with N that solve's; it pins US1's guards (FR-002) and passes from US1 on"
        decision: "spec.md:US2-S4"
      - path: crates/finder/tests/motion/centre_tap.rs
        status: new
        disposition: add
        assertion: "the centring rig at 17: POST /api/imaging/frame for B, read until B is saved, POST /api/imaging/solve, POST /api/imaging/frame for C and {} at once: 409 exactly a newer imaging frame was taken after frame B, the frame's check before the solve's state (FR-002's order); it passes from US1 on"
        decision: "spec.md:US2-S5"
    preserved_behavior:
      - "a centring of {} answers and refuses as US1 left it, US1's centre.rs tests passing unchanged with CentreAsk's three fields added; the route's write rules are US1's"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test motion -E 'test(/^centre_tap::/) | test(/^centre::/)'"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs and the night of plan.md Outside the factory"
    writes:
      - crates/finder/src/centre.rs
      - crates/finder/tests/motion/centre_tap.rs
      - crates/finder/tests/motion/main.rs
    no_write_reason: ""
    shared_writes: []
    open_questions: []
  US3:
    tests:
      - path: crates/finder/tests/motion/centre_run.rs
        status: new
        disposition: add
        assertion: "the centring rig at 17 pushed 390 at 30, and a new one with the aim point 240 off at 210, frames stopped: POST /api/imaging/run {} 200; read until run.state stacking or ended: stacking, run.centring.tries 1, run.centring.offset_arcsec at most 25, no warnings, off M42 at most 25, aim, model.epoch and model.syncs as before the run"
        decision: "spec.md:US3-S1"
      - path: crates/finder/tests/motion/centre_run.rs
        status: new
        disposition: add
        assertion: "the centring rig at 17 pushed 120 at 120 with frames left stopped, and a new one with the aim point 120 off at 120 with frames started again after the centring rests: a solve, centre {} 200, rests, PUT /api/imaging/settings {exposure_ms 2000}, POST /api/imaging/stack {}, read until stack.stacked at least 15: steering.source subs and off M42 at most 10; it pins FR-003 under a stack and passes from US1 on"
        decision: "spec.md:US3-S2"
      - path: crates/finder/tests/motion/centre_run.rs
        status: new
        disposition: add
        assertion: "the script's run, four solves of 300 with tolerance 90, near 450 and aim_moved false, its mount the script's wrapped so centre answers Ok(true) without moving and after the first call the script's mount reads Tracking steady false: 1 s after the start the view is centring with the message exactly Centring M42: 300″ from the sensor's centre, try 1 of 3. Waiting for the mount to settle; nothing to do.; the mount set steady, 5 s: stacking after exactly three centre calls, centring.tries 3, warnings exactly [M42 is 300″ from the sensor's centre after 3 tries; it is stacked where it is.]"
        decision: "spec.md:US3-S3"
      - path: crates/finder/tests/motion/centre_run.rs
        status: new
        disposition: add
        assertion: "the centring rig at 17 and console::mount(&state): point(By::Page) Ok(()); tracking, 30 s, a solve; POST /api/stop; centre() answers Err(Failure::Stopped) and the session's axis_move.tool is not centre"
        decision: "spec.md:US3-S4"
      - path: crates/finder/tests/motion/imaging_run_centring.rs
        status: existing
        disposition: preserve
        assertion: "041's centring of the run: tolerance, tries, idle solves, warnings and endings with the script's parts, which keep MountSteps::centre's default"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "Script and every MountSteps implementation but the console's answer as at 8b7212bb; a run whose mount part makes no step centres, idles, warns and ends as 041's FR-024 reads"
      - "the run's states, endings, words and record as at 8b7212bb"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test motion -E 'test(/^centre_run::/) | test(/^imaging_run_centring::/) | test(/^imaging_run::/)'"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs and the night of plan.md Outside the factory"
    writes:
      - crates/finder/src/imaging/run.rs
      - crates/finder/src/imaging/run/console.rs
      - crates/finder/tests/motion/centre_run.rs
      - crates/finder/tests/motion/main.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/imaging/run.rs
        other_spec: 041-imaging-run-on-the-rig
        other_story: US7
        disposition: merge_order
        reason: "057 depends_on_landed 041, whose US7 last changed run.rs on main (8a880e4f)"
      - path: crates/finder/src/imaging/run/console.rs
        other_spec: 041-imaging-run-on-the-rig
        other_story: US8
        disposition: merge_order
        reason: "057 depends_on_landed 041, whose US8 last changed run/console.rs on main (61b54a47); 054 writes it before 057 lands and the re-anchor reads its last writer again"
    open_questions: []
  US4:
    tests:
      - path: crates/finder/tests/motion/centre_aim.rs
        status: new
        disposition: add
        assertion: "the centring rig at 17 with the aim point 390 off at 30 and frames stopped, and a new one with the aim point 240 off at 30 and frames running: a solve, centre {} 200, rests, frames started, the session read each second for 30 s: no track.push; off M42 at most 25, the place Track holds within 0.01 arcsec of M42, model.syncs and model.epoch as before, slip_pending false, aim source measured within 1 pixel of the true eyepiece pixel; Track off, 2 s, Track on until tracking, 30 s: off M42 at most 25"
        decision: "spec.md:US4-S1"
      - path: crates/finder/tests/motion/centre_aim.rs
        status: new
        disposition: add
        assertion: "the centring rig at 17 with the aim point 240 off at 30, frames stopped: a solve, centre {} 200, rests, 60 s, frames started, 30 s: aim exactly as before the centring; a solve, centre {} 200, rests: aim.source measured"
        decision: "spec.md:US4-S2"
      - path: crates/finder/tests/motion/centre_aim.rs
        status: new
        disposition: add
        assertion: "the centring rig at 17 with the aim point 240 off at 30 and frames running: a solve, centre {} 200, rests, 30 s, GET /api/session; Track off with POST /api/track {on false}, 2 s: aim exactly as before Track off (FR-010's held aim point dropped)"
        decision: "spec.md:US4-S3"
      - path: crates/finder/tests/motion/imaging_track.rs
        status: existing
        disposition: preserve
        assertion: "032's aim reading taken while Track is on held back and applied at the next Track on (FR-036), as at 8b7212bb, no TrackRun having a centring's mark"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/motion/tracking/push.rs
        status: existing
        disposition: preserve
        assertion: "038's push check, its follow, sync and epoch, as at 8b7212bb"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "record_live_pointing reads every pointing as at 8b7212bb while the running Track has no centring's mark"
      - "set_aim, set_measured_aim and 032's held aim point as at 8b7212bb"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test motion -E 'test(/^centre_aim::/) | test(/^centre::/) | test(/^tracking::push/) | test(/^imaging_track::/)'"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs and the night of plan.md Outside the factory"
    writes:
      - crates/finder/src/centre.rs
      - crates/finder/src/session.rs
      - crates/finder/tests/motion/centre_aim.rs
      - crates/finder/tests/motion/main.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/session.rs
        other_spec: 047-console-uses-the-database-that-fits
        other_story: US2
        disposition: merge_order
        reason: "057 depends_on_landed 047, whose US2 last changed session.rs on main (c15937e7); 053 writes it before 057 lands and the re-anchor reads its last writer again"
    open_questions: []
  US5:
    tests:
      - path: crates/finder/tests/mcp/centre.rs
        status: new
        disposition: add
        assertion: "the MCP rig with the call log: tools/list has centre with a description containing real telescope, outputSchema type object, inputSchema frame_id [integer, null], x and y [number, null]; centre {} isError the imaging camera has not solved a frame yet; after 1.1 s {frame_id 1} and {q 1} isError exactly PARTS; aim at the true eyepiece, Track on, 30 s, frames stopped, pushed 240 in azimuth, 2 s, a solve: centre {} not an error, structuredContent moved track and offset_arcsec within 5 of off M42; at once motion.by mcp and motion.tool centre; the next two calls isError the telescope moved after imaging frame N: take another and solve it and the rate limit of 2 moving calls a second was reached; after axis_move done and 12 s off M42 at most 25; the call log has a line containing mcp centre and none containing test-token"
        decision: "spec.md:US5-S1"
      - path: crates/finder/tests/mcp/centre.rs
        status: new
        disposition: add
        assertion: "the committed README keeps The imaging camera has sixteen tools of its own; Centring from an imaging solve has one tool: lies after The page's nudge arrows have one tool: and before ### MCP under WSL2 with a line starting - `centre` takes nothing after it; in ## API the row starting | `POST /api/imaging/centre` | comes before the row starting | `POST /api/goto` |; ## Centring from an imaging solve lies after ## Nudging the telescope and before ## Recording a night and, whitespace made single, contains It needs no live pointing, `a newer imaging frame was taken after frame N`, the telescope moved after imaging frame N: take another and solve it (with its closing backtick), A centring is never a sync. and The finder camera's first solve within a minute after it takes the aim point; the glossary's **Centring from an imaging solve** follows **Nudge** and starts with FR-013's words and _Avoid_: recentre"
        decision: "spec.md:US5-S2"
      - path: crates/finder/tests/console/imaging_run_docs.rs
        status: existing
        disposition: preserve
        assertion: "the README's ## API rows of the run and ## MCP clients' sixteen tools of its own"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/mcp/nudge.rs
        status: existing
        disposition: preserve
        assertion: "055's README paragraph before ### MCP under WSL2 and ## Nudging the telescope before ## Recording a night"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "every MCP tool at 8b7212bb keeps its name, schema, description and answers"
      - "every README sentence and glossary entry at 8b7212bb stays"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test mcp -E 'test(/^centre::/) | test(/^nudge::/)'"
        - "cargo nextest run -p finder --locked --test console -E 'test(imaging_run_docs::)'"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - GLOSSARY.md
      - crates/finder/README.md
      - crates/finder/src/mcp/tools.rs
      - crates/finder/tests/mcp/centre.rs
      - crates/finder/tests/mcp/main.rs
    no_write_reason: ""
    shared_writes:
      - path: GLOSSARY.md
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US3
        disposition: merge_order
        reason: "057 depends_on_landed 051, whose US3 last changed GLOSSARY.md on main (9424a371)"
      - path: crates/finder/README.md
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US3
        disposition: merge_order
        reason: "057 depends_on_landed 051, whose US3 last changed the README on main (9424a371)"
      - path: crates/finder/src/mcp/tools.rs
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US3
        disposition: merge_order
        reason: "057 depends_on_landed 051, whose US3 last changed mcp/tools.rs on main (9424a371)"
      - path: crates/finder/tests/mcp/main.rs
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US3
        disposition: merge_order
        reason: "057 depends_on_landed 051, whose US3 last changed mcp/main.rs on main (9424a371)"
    open_questions: []
  US6:
    tests:
      - path: crates/finder/web/src/components/CentreBlock.test.tsx
        status: new
        disposition: add
        assertion: "the page rig with fetch answering {frame_id 3, offset_arcsec 386.6, moved track}: Centre M42 posts /api/imaging/centre, POST, Content-Type application/json, body {}; onChange once, and twice after the second click; the status exactly Centring M42: 387″ from the centre… after the answer, after a session with no axis_move and after axis_move {state moving, tool centre}; exactly Centred M42 · Track holds it. Take another imaging frame to check. after {state done, tool centre}; after the second answer with the done session still shown, Centring M42: 387″ from the centre…; exactly Centring stopped: stopped after {state halted, tool centre, reason stopped}"
        decision: "spec.md:US6-S1"
      - path: crates/finder/web/src/components/CentreBlock.test.tsx
        status: new
        disposition: add
        assertion: "fetch answering {frame_id 3, offset_arcsec 834, moved track}, the picture laid out at left 10, top 20, 618 x 411: the image Solved imaging frame 3: tap a point to centre it has src /api/imaging/frames/3/preview.jpg; a click at client (319, 122.75) posts POST, that header and the body {\"frame_id\":3,\"x\":618,\"y\":205.5}; the status exactly Centring the tapped point: 13.9′ from the centre…"
        decision: "spec.md:US6-S2"
      - path: crates/finder/web/src/components/CentreBlock.test.tsx
        status: new
        disposition: add
        assertion: "fetch answering 409 {error Track is off: turn Track on to centre}: Centre M42 shows that alert exactly and no status; with the solve of frame 2 Centre M42 disabled and no image; without a target no button and still the image"
        decision: "spec.md:US6-S3"
      - path: crates/finder/web/src/components/CentreBlock.test.tsx
        status: new
        disposition: add
        assertion: "ImagingPage from the recorded session tracking with the page rig's view: the region Centre follows the region Imaging solve in the document and holds the button Centre M42"
        decision: "spec.md:US6-S4"
      - path: crates/finder/web/e2e/solve-centre.spec.ts
        status: new
        disposition: add
        assertion: "the simulated console by day, Track tracking and 12 s, the Imaging page: Take imaging frame and Solve imaging frame in their regions until Saved and Solved; Centre M42 response 200 moved track, the status exactly Centred M42 · Track holds it. Take another imaging frame to check. within 20 s, track.state tracking; 12 s, another frame solved; a click on the Centre block's picture at a quarter of its width and half its height posts exactly the keys frame_id, x and y with x within 4 of 309 and y within 4 of 411, response 200, the status exactly Centred the tapped point · Track holds it. Take another imaging frame to check. within 20 s"
        decision: "spec.md:US6-S5"
      - path: crates/finder/web/e2e/solve-centre.spec.ts
        status: new
        disposition: add
        assertion: "a frame solved as in US6-S5: the Centre block's day screenshot has more than 1000 off-red pixels; Night mode: red and black on, data-theme night on html, 1.5 s: the night screenshot has none"
        decision: "spec.md:US6-S6"
      - path: crates/finder/web/e2e/imaging.spec.ts
        status: existing
        disposition: preserve
        assertion: "the Imaging page's frame picture is the only image named Imaging frame …; the Centre block's is Solved imaging frame …"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/e2e/goto.spec.ts
        status: existing
        disposition: preserve
        assertion: "the GoTo test's single status in the region Target M42, with solve-centre.spec.ts sorting after it"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "every block of the Imaging page renders and answers as at 8b7212bb; ImagingPage gains one import and one element"
      - "the simulate project's other files see the server as they set it: solve-centre.spec.ts turns Track off and the target to null at the end"
    checks:
      development:
        - "npm test -- CentreBlock"
        - "CI=1 npm run e2e -- --project=simulate e2e/solve-centre.spec.ts e2e/goto.spec.ts e2e/imaging.spec.ts"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml finder-page job and the night of plan.md Outside the factory"
    writes:
      - crates/finder/web/dist
      - crates/finder/web/e2e/solve-centre.spec.ts
      - crates/finder/web/playwright.config.ts
      - crates/finder/web/src/api.ts
      - crates/finder/web/src/components/CentreBlock.test.tsx
      - crates/finder/web/src/components/CentreBlock.tsx
      - crates/finder/web/src/components/ImagingPage.tsx
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/web/dist
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US4
        disposition: merge_order
        reason: "057 depends_on_landed 051, whose US4 last built dist on main (f6d4f5fd)"
      - path: crates/finder/web/playwright.config.ts
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US4
        disposition: merge_order
        reason: "057 depends_on_landed 051, whose US4 last changed playwright.config.ts on main (f6d4f5fd)"
      - path: crates/finder/web/src/api.ts
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US4
        disposition: merge_order
        reason: "057 depends_on_landed 051, whose US4 last changed api.ts on main (f6d4f5fd)"
      - path: crates/finder/web/src/components/ImagingPage.tsx
        other_spec: 042-imaging-run-page-and-mcp
        other_story: US9
        disposition: merge_order
        reason: "057 depends_on_landed 042, whose US9 last changed ImagingPage.tsx on main (5d8f5353); 054 and 055 rewrite and add to it before 057 lands and the re-anchor reads its last writer again"
    open_questions: []
```
