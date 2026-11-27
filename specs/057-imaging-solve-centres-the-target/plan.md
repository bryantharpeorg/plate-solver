# Implementation Plan: An Imaging Solve Centres the Target

**Branch**: `057-imaging-solve-centres-the-target` | **Date**: 2026-11-27 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/057-imaging-solve-centres-the-target/spec.md`

## Summary

A centring brings a place, the session's target or the sky at a tapped pixel of the
latest solved imaging frame, to the imaging camera's centre from that frame's imaging
solve alone. A new module, `crates/finder/src/centre.rs`, serves
`POST /api/imaging/centre`, reads the latest solve and frame, checks the guards, and
hands Track the place to centre and the solve's centre. `Tracker::centre` makes the
place Track holds the place to centre and has Track's next turn make the step from the
solve's centre to it, the step 055's nudge makes, so it needs no live pointing, moves no
aim point and is never a sync. `nudge::track_step`, the Track branch of 055's `nudge`
made shared, reserves the move and reports it in `motion` and `axis_move`. The imaging
run's mount part gains `centre`, a default method the console's part implements, and the
run asks it when a solve moved no aim point. MCP gains a `centre` tool through the same
function; the README and the glossary gain the words; the Imaging page gains a Centre
block. Five stories: the target's centring and Track's step; the tap; the run and the
stack; MCP and the words; the page.

## Technical Context

**Language/Version**: Rust, the factory's toolchain; TypeScript and React for the page
(`crates/finder/web`).

**Primary Dependencies**: none new. `axum` routes, `rmcp` tools, the console's own
`tracking`, `nudge`, `imaging` and `model` modules.

**Testing**: `cargo nextest` (the `motion` and `mcp` test binaries), Vitest and
Playwright, all under the `test` gate.

**The general rule in code.** A stack's tracking error is the sky at the set point less
the place Track holds (`tracking_error`, `crates/finder/src/imaging/steering.rs:32`,
called with the tracker's target at `crates/finder/src/imaging/steering.rs:569`). An
imaging solve gives the sky at the imaging camera's centre, `(ra_deg, dec_deg)`, at the
frame's time. A centring makes the place Track holds the place to centre and steps the
telescope by the difference between the two, so the step and every later tracking
error agree: after it, the place Track holds is what the imaging camera's centre should
show. The place to centre is the session's target's RA and Dec, or the sky at a pixel by
the solve's own matrix (`ImagingSolve::from_cd`, `crates/finder/src/imaging/solver.rs:150`,
and `sky_at`, `crates/finder/src/imaging/solver.rs:210`).

**The tap reads a saved, solved frame; 058's preview frames.** A tap names a pixel of
the latest imaging frame that is `saved` and that the latest solve solved (FR-002,
FR-007), and the Centre block draws that frame's own picture to tap. Spec 058, drafted
beside this one and not read here (it is on no branch this probe had), adds unsaved
preview frames on the Imaging page that are never solved (032 refuses to solve a shown
frame). A preview frame has no solve of its own, so a tap on its picture cannot be a
centring: it could only be a step in the frame's up and right from the last solve, as
055's `imaging_camera` view reads them, which is a nudge, not this spec's centring. This
spec centres from saved, solved frames only. At the re-anchor on 058 (§ Prerequisites),
read whether a preview frame becomes the imaging view's latest `frame`: if it does,
FR-002's check "the latest frame is `saved` and the solve's" would refuse every
centring after a preview with `imaging frame N is not solved yet: solve it first`, and
the check is to read the latest saved frame instead; the Centre block's picture stays
the solved frame's either way.

**Track's step.** 055's `Tracker::nudge` (`crates/finder/src/tracking.rs:393`) reads
the held place, works out the moved place, reads the counts, plans the step with
`step_path` (the counts plus the model's inverse at the moved place on arrival less
that at the held place now, each moved axis 0.5° past against Track's way, a GoTo's
guards), takes the session's model, sets the held place, clears the correction's memory
and marks the step pending (`stepping = Some(from)`,
`crates/finder/src/tracking.rs:446`); the rate loop makes it and runs the quiet window
(055's FR-010, FR-011). Everything after the moved place is the same for a centring, with
`from` the solve's centre instead of the held place. So that body becomes
`step_between(from, to, model)`, which `nudge` and the new `centre` call. The step's end
is planned as a difference of the model's inverses, so the model's zero-point errors
cancel to first order; what is left is the model's scale, about 2 % of the step on the
simulator (055's Risks), 17.5″ on a 726″ step and 23.2″ on an 834″ tap.

**The steadiness.** An imaging solve says where the imaging camera's centre was during
its frame. 040's steady marks (`Tracker::steady_between`,
`crates/finder/src/tracking.rs:526`) are false for any moment Track was settling,
pushed, stalled, measuring backlash, or stepping (a nudge's or a centring's step), and
before Track first tracked. So "Track steady from the solved frame's start until now"
is the one guard that covers a push, a nudge, Track's settle, a reversal and an earlier
centring, and a second centring from the same solve is refused by it.

**The run.** `MountSteps` (`crates/finder/src/imaging/run.rs:243`) gains
`fn centre(&self) -> StepFuture<'_, Result<bool, Failure>>` with a default of
`Ok(false)`; `Script` keeps the default, so 037's and 041's scripted tests see the run
as before. The console's part centres the session's target through `centre::centre`
with the run's `by`. In the procedure, after the aim point's re-centre branch
(`crates/finder/src/imaging/run.rs:1251`) and before the idle branch
(`crates/finder/src/imaging/run.rs:1265`), a solve outside the tolerance that moved no
aim point, with fewer than three tries made, asks the part; a step made counts a try,
shows `centring` with the try's words, and the loop's next frame waits for the mount to
be steady (`RunTask::frame` calls `steady`, `crates/finder/src/imaging/run.rs:1045`).
After three steps the offset counts as measured: the warning or the ending is 041's for
three tries.

**Performance Goals**: a centring answers at once (the route hands Track the step and
returns). On the simulator the telescope is within 25″ of the place 12 s after the step
ends.

**Constraints**: no new dependency; no change to the pointing model, the live solve,
the aim point's measure, `session.rs`, `goto.rs` or the steering; `tracking.rs` gains
`centre` and moves `nudge`'s body into `step_between` without changing what a nudge
does.

**Scale/Scope**: about 1,640 added lines across five stories (the probe's, without
`dist` and the probe-only tests), about 1,130 of them tests.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Detection, the plate solver and the live solve are untouched; the imaging solve is read as 032 gives it (its centre and `cd_deg`). |
| II. Never weaken a gate | No existing test, golden, fixture, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS` or `ergane.yaml` is edited. `playwright.config.ts` gains two list entries, an owner approval as 050's, 051's and 055's were. The only edits to existing test files are the `mod` lines of `crates/finder/tests/motion/main.rs` and `crates/finder/tests/mcp/main.rs` (SC-002). Every existing test passed unchanged on the probe's tip (run, § What was run). |
| III. Tests first, independent expectations | The oracles are the simulator's imaging truth, eyepiece and push, M42's catalogue place, the solve's own matrix at a pixel (032's), and the words of 041 and the scenarios. Each test file writes its own copy of PARTS; no test reads a constant of `centre.rs`. Time comes from a paused tokio clock and the simulator's virtual clock. US1's, US2's, US3-S1's, US4's and US5's tests failed on the code before them (run); US3-S2 proves US1's step under a stack and passes from US1 on (§ Plan traps 4). |
| IV. Read the critical paths twice | None is touched: no `Cargo.toml` or `Cargo.lock` change, no `unsafe`, nothing under `proto/`, `crates/math-core`, `crates/pattern-database` or `crates/database-generation`, no `build.rs` or `.cargo/`. The mount's motion is a safety path: a centring is Track's own step through the same `step_path` and guards as 055's Track-on nudge, behind the same reservation. |
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
crates/finder/src/centre.rs                      # new (US1, the target); US2 the tap and PARTS
crates/finder/src/tracking.rs                    # US1: `Tracker::centre`, `step_between`
crates/finder/src/nudge.rs                       # US1: `track_step`, the nudge's Track branch through it
crates/finder/src/api.rs                         # US1: one `merge`, `centre` in the reservation
crates/finder/src/lib.rs                         # US1: `pub mod centre;`
crates/finder/src/imaging/run.rs                 # US3: `MountSteps::centre`, the procedure's step
crates/finder/src/imaging/run/console.rs         # US3: `ConsoleMount::centre`
crates/finder/src/mcp/tools.rs                   # US4: the `centre` tool
crates/finder/README.md, GLOSSARY.md             # US4
crates/finder/tests/motion/centre.rs             # US1, new (its rig and helpers `pub(super)`)
crates/finder/tests/motion/centre_tap.rs         # US2, new
crates/finder/tests/motion/centre_run.rs         # US3, new
crates/finder/tests/mcp/centre.rs                # US4, new
crates/finder/web/src/api.ts                     # US5: `ImagingFrame`'s size, `Centred`, `postCentre`
crates/finder/web/src/components/CentreBlock.tsx, CentreBlock.test.tsx  # US5, new
crates/finder/web/src/components/ImagingPage.tsx # US5: one import, the block after the solve block
crates/finder/web/e2e/solve-centre.spec.ts       # US5, new
crates/finder/web/playwright.config.ts           # US5: two entries
crates/finder/web/dist                           # US5: the build
```

**055's files.** This spec is stacked on 055's code: `nudge.rs` exists only there, and
`tracking.rs`, `api.rs`, `mcp/tools.rs`, the README, the glossary, `api.ts`,
`ImagingPage.tsx` and `playwright.config.ts` carry 055's additions, which this spec
places its own after. In `tracking.rs` and `nudge.rs` the lines that move keep their
order and words; no nudge answers otherwise (055's tests pass unchanged, run).

**The other drafts' shared files.** 052, 053 and 054 also write the README, the
glossary and the test `main.rs` files; 054 writes `run/console.rs` and rewrites
`ImagingPage.tsx`; 053 owns syncs, the model and `session.rs`, which this spec reads
only. They land in the order 052, 053, 054, 055, and the seat re-anchors this trio on
whatever has landed before its flip (§ Prerequisites).

## Interfaces

### `crates/finder/src/centre.rs` (US1 new; US2 adds)

```rust
//! Centring from an imaging solve (057): the session's target, or the point
//! the observer taps on the latest solved imaging frame, is brought to the
//! imaging camera's centre with Track's own step (055). ...

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

`centre` checks, in order: (US2) the ask's parts, `(None, None, None)` the target and
`(Some, Some, Some)` with finite `x` and `y` a point, else 400 PARTS; the latest solve's
`state` (`solved`, else `solving`, `failed` or none with FR-002's words); the latest
frame `saved` and the solve's (`imaging frame N is not solved yet: solve it first`); the
solve's `ra_deg`, `dec_deg` (and from US2 `cd_deg`) and the frame's `width` and
`height`, else `the imaging camera has not solved a frame yet`; the place to centre
((US2) the point's frame and bounds, then `sky_at`; or the session's target); `track`
wanted; the session's tracker `tracking`; `tracker.steady_between(frame.start, now)`.
Then `offset_arcsec` is `model::separation` of the solve's centre and the place times
3,600, and `nudge::track_step(state, by, "centre", |tracker, model| tracker.centre(centre, place, model))`.

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
    /// The step both make: plan it from the counts, take the session's `model`,
    /// hold `to` and mark the step pending from `from`.
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
reserved list (`crates/finder/src/api.rs:844`); `lib.rs` gains `pub mod centre;` after
`pub mod catalog;` (`crates/finder/src/lib.rs:86`).

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

The trait doc's sentence at `crates/finder/src/imaging/run.rs:377` gains: "but
`MountSteps::centre` (057), whose default makes no step." The procedure gains
`let mut stepped = false;` and, after the re-centre branch:

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
and `Ending::NotCentred`. `ConsoleMount::centre` answers `Err(Failure::Stopped)` when
stopped and otherwise `Ok(centre::centre(&self.state, by, &CentreAsk::default()).await.is_ok())`,
`by` the run's.

### `crates/finder/src/mcp/tools.rs` (US4)

`CentreArguments { frame_id: Option<u64>, x: Option<f64>, y: Option<f64> }`, schema
only, after `NudgeArguments` (`crates/finder/src/mcp/tools.rs:274`), and the tool
`centre` before `set_tracking` (`crates/finder/src/mcp/tools.rs:525`): it reads its
arguments as a `CentreAsk` (a failure answers PARTS), takes `limit_move`, calls
`centre::centre(&state, "mcp", &ask)` and logs the call as `centre`. Its description:
"Move a real telescope so the session's target, or the point at pixel x, y of imaging
frame frame_id, comes to the imaging camera's centre, from that frame's imaging solve
alone, as the Imaging page's Centre block does. Needs Track tracking and the latest
imaging frame solved with nothing moving the telescope since it began; the place Track
holds becomes that place and Track makes the step. It moves no aim point and is never a
sync. Answers frame_id, offset_arcsec and moved: track."

### The README (`crates/finder/README.md`, US4)

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

In `## API`, before the `POST /api/goto` row (`crates/finder/README.md:326`):

```markdown
| `POST /api/imaging/centre` | JSON `{}`, or `{"frame_id":N,"x":X,"y":Y}` with X and Y a pixel of imaging frame N. Bring the session's target, or that pixel's place, to the imaging camera's centre from the latest imaging solve, with Track on; answers 200 with `frame_id`, `offset_arcsec` (how far that place was from the frame's centre) and `moved` (`track`), or 400, 409 or 415 with a JSON `error`. Same-origin writes only. |
```

After `## Nudging the telescope`'s text, before `## Recording a night`
(`crates/finder/README.md:670`):

```markdown
## Centring from an imaging solve

A centring brings a place to the imaging camera's centre from the latest imaging
solve alone: the session's target, or a point tapped on the solved frame's
picture in the Imaging page's Centre block. It needs no live pointing, so it
works when the finder camera cannot solve. Take an imaging frame and solve it
with Track on, then centre: the place Track holds becomes that place and Track
makes the step, as a nudge's. The solve's frame must be the latest, and nothing
may have moved the telescope since it began, or the centring is refused with
`the telescope moved after imaging frame N: take another and solve it`. A
centring moves no aim point and is never a sync. An imaging run centres the same
way when a solve measures no new aim point. Without the finder camera's solves
Track holds the place on the mount's rates alone and may drift: take another
frame and centre again, or let a stack's steering hold it. When the finder
camera solves again with the aim point off, Track brings the aim point back onto
the place and the target off with it; an imaging run measures the aim point
then.
```

### The glossary (`GLOSSARY.md`, US4)

**Centring** after **Nudge** (`GLOSSARY.md:277`), before **Purpose label**
(`GLOSSARY.md:281`), with FR-012's words and its `_Avoid_` line.

### The page (`crates/finder/web/src/`, US5)

`api.ts`: `ImagingFrame` gains `width?: number; height?: number;` after `start?`;
`interface Centred { frame_id: number; offset_arcsec: number; moved: "track" }` and
`postCentre(point?: { frame_id: number; x: number; y: number }): Promise<Centred>`,
posting `JSON.stringify(point ?? {})` with `Content-Type: application/json`, before
`postRecord`. `CentreBlock({ view, session, onChange })`: a `section` labelled by its
`h2` `Centre`; the quiet line "With Track on, take an imaging frame and solve it, then
centre the target or tap a point of the solved frame. It needs no solve of the finder
camera."; the button; the picture in a `border border-ink bg-photo` box with the
`photo` class (019's night filter); `role="status"` and `role="alert"` lines.
`ImagingPage.tsx` imports it after `ImagingSolveBlock` and renders
`<CentreBlock view={view} session={session} onChange={() => { onChange?.(); void refresh(); }} />`
right after `<ImagingSolveBlock …/>` (`crates/finder/web/src/components/ImagingPage.tsx:309`).

## What was run and what was read

All on the probe branch `probe/057`, cut from `probe/055-r1` at 8b7212bb, on the
simulator, with the factory's toolchain, `--locked`, and npm from its cache. Logs are
outside the repository.

**The stories.** 979a2573 and eb958a11 (the target and the tap, tests then code),
8872fb84 and 6b81cb65 (the run), 3e60c411 and 212c053e (MCP and the words), 3c234092 and
7ddbaf0d (the page), then 00c4a55a (the tests moved into this spec's five-story layout,
the browser test renamed to sort after `goto.spec.ts`) and 1a3f06be (the README's
section in these words). 3a46ca58 is PROBE ONLY: the candidates' measurements. Each
story's tests failed on the code before it: US1's route answered 405; the run stacked
387″ off with tries 0 and the warning; the MCP tool was not listed and the README had no
section; the Vitest file found no `CentreBlock`.

**The candidates (3a46ca58, simulated).** Four trials at imaging angles 17°, 100°, 250°
and 17°, 120″ to 729″ off, each with the telescope pushed and with the aim point off,
the finder camera's frames stopped: the general rule's step left 4.2″ to 17.5″ and 6.9″
to 14.1″; moving the held place by the offset left 3.7″ to 17.6″ and 6.2″ to 12.9″; one
check frame reached 90″ in every trial. 60 s later, with no solves, 30.2″ to 52.1″. With
the finder camera's frames back for 45 s: the general rule held the pushed cases at 5.4″
to 8.5″ and let the aim cases go back to 42.7″ to 235.7″; moving the held place did the
reverse. Stacks of 25 subs from 120″: the general rule 4.8″ to 1.4″ and 9.0″ to 1.3″;
the moved place 3.7″ to 74.0″ and 7.4″ to 76.4″. The aim point moved by the offset with
no solves: 394″ to 403″ and 481″; 113″ to 95″ and 91″. Today's run: 386″, 242″, 114″
with tries 0; with US3's step: 13.8″, 14.7″, 5.3″ after one try. Taps of 834″, 1,047″
and 679″: 23.2″, 13.2″, 7.6″.

**This text's tests (00c4a55a).** The six motion tests in this spec's layout passed; US1's
tests passed on US1's code alone (the tap, the run and the tool taken out: `centre.rs`
without the point, `run.rs`, `run/console.rs` and `mcp/tools.rs` at 8b7212bb), with
`cargo fmt` and clippy clean; US2's on US2's code alone; US3-S1 failed on US2's code
(tries 0) and US3-S2 passed there (§ Plan traps 4).

**The gates (7ddbaf0d, then 00c4a55a and 1a3f06be).** `lint` and `test` on 7ddbaf0d:
fmt, clippy, both doc builds, nextest 1,558 of 1,558, doc tests, `npm ci`, typecheck,
Prettier, Vitest, the build with `dist` clean, and `CI=1 npm run e2e` 38 of 39, one
failing: `goto.spec.ts`'s status locator found the guidance block's `Tracking ended`
line left by `centre.spec.ts`, which then sorted before it. Renamed to
`solve-centre.spec.ts` (00c4a55a), the whole browser suite passed, 40 of 40. On
1a3f06be the README's and the other specs' README tests passed (41 of 41).

**Read, not run.** A centring refused for a latest frame that did not solve, for Track
armed, in its keyhole wait or measuring backlash, during a direct axis move, or across
a GoTo's guards; Stop during the step and the `Centring stopped:` status; the run's
three steps without reaching the tolerance; a centring during a stack or a run; a
mirrored imaging frame; a push or a nudge after the solved frame began (each makes
Track unsteady); a real mount and cameras.

## Read from the diff, not from a test

- `tracking.rs`'s `step_between` is `nudge`'s body from the axes' read onwards, in the
  same order; `nudge` only works out `to` first.
- `nudge.rs`'s `track_step` is the nudge's Track branch with its tool a parameter.
- `run.rs` changes only the trait's one default method, its doc sentence, and the
  procedure's lines of § Interfaces; `Script` is not written.
- The README's new paragraph, row and section keep every sentence at 8b7212bb.
- `CentreBlock` posts only `postCentre`; `ImagingPage` gains one import and one element.

## Shapes that existing tests pin

- **055's nudge with Track on.** `crates/finder/tests/motion/nudge_track.rs` and
  `crates/finder/tests/motion/nudge.rs` pass unchanged with `step_between` and
  `track_step` (run).
- **The run's centring.** `crates/finder/tests/motion/imaging_run_centring.rs` (041) and
  the library's `imaging::run` tests pass unchanged: `Script` keeps the default, so a
  solve that moved no aim point is idle as before (run).
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
  specs after it pass (run, 40 of 40).

## Decisions and the alternatives rejected

| Decision | Chosen | Rejected, and why |
|---|---|---|
| What moves | the place Track holds becomes the place to centre; Track steps (decision 3) | the place moved by the offset (a stack pulls it to 74″); the aim point (nothing moves without solves); a sync (053's) |
| The step | `step_between`, 055's step | a GoTo with Track off and on (the run's re-centre: it needs a new aim point) |
| The guard for movement | Track steady since the solved frame began | a time limit on the solve (a push inside it passes); none (a stale solve centres the wrong way) |
| The solve read | the latest, of the latest saved frame | any solved frame (its pixel may no longer be where the solve says) |
| Track off | refused (decision 6) | a direct move (stale in seconds); Track on first (its GoTo moves the frame) |
| The run | the mount part's new default method | a new part or a new `Steps` field (more API; `Script` would need it) |
| The tool | through `centre::centre` | through the route (the tool needs `by` `mcp`) |
| The page | a Centre block with its own picture (decision 7) | a tap on the Imaging frame block's picture |

## User Story 1 - The target's centring and Track's step

`centre.rs` without the tap, `Tracker::centre` and `step_between`, `track_step`, the
route's merge, the reservation, `pub mod centre;`, and the tests of spec.md US1.

## User Story 2 - The tap

`CentreAsk`'s three fields, `PARTS`, the point's checks and `sky_at`, and the tests of
spec.md US2.

## User Story 3 - The run and the stack

`MountSteps::centre`, the procedure's step, `ConsoleMount::centre`, and the tests of
spec.md US3.

## User Story 4 - MCP and the words

The tool, the README's paragraph, row and section, the glossary, and the tests of
spec.md US4.

## User Story 5 - The page

`ImagingFrame`'s size, `postCentre`, `CentreBlock`, its line in `ImagingPage`, the config
entries, `dist`, and the tests of spec.md US5.

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

- 052, 053, 054 and 055 have landed, in that order, and this trio is re-anchored on that
  `main`: every `path:line` read again; 052 to 055 added to `depends_on_landed`;
  `tracking.rs`'s and `nudge.rs`'s moves placed against 055's landed text; the README,
  `GLOSSARY.md` and the test `main.rs` additions placed after theirs; `run/console.rs`'s
  `centre` placed against 054's; the Centre block placed on 054's rewritten
  `ImagingPage.tsx` after its Imaging solve block; and the Refinement Record's last
  writers read again.
- 058's preview frames read against FR-002 and FR-013 (§ Technical Context: the tap
  reads a saved, solved frame), if 058 lands before this spec.
- The owner's eight decisions are answered.
- A probe round from this text alone, folded.
- The page half has run once on the re-anchored base (`CI=1 npm run e2e`).
- No operator-staged file is needed.

## Outside the factory

Written here and not run by any gate. **On the next field night**, with the owner at the
telescope: GoTo M42, turn Track on, open the Imaging page, take an imaging frame and
solve it, and press `Centre M42`; take another frame and solve it, and repeat until the
Imaging solve block puts M42 within 90″ of the centre. Note how far each solve put M42
from the centre, how many frames it took, and whether any centring was refused and its
words. Then tap a star near a corner of the solved frame and note how far the next
solve puts it from the centre. Then start an imaging run on M42 and note its
`centring.tries` and its offset at `stacking`. The counts and the notes go in the
night's report; nothing in the repository waits for them.

## Plan traps

1. **The story sizes.** Measured on the probe's commits in this spec's layout, each
   diff without `dist` (§ What was run): US1 28,192 B (43.0 % of 65,536; 58.0 % at
   1.35 times), US2 9,595 B (14.6 %; 19.8 %), US3 7,497 B (11.4 %; 15.4 %), US4
   20,502 B (31.3 %; 42.2 %), US5 19,409 B plus `dist`'s 697 B (30.7 %; 41.4 %). US1 is
   the tightest, 38,059 B at 1.35 against 39,321: its `centre.rs` without the tap was
   cut from the probe's by hand and run alone (§ What was run). Should it grow, its
   refusal test's last three asks (about 1.1 KB) move to US2's refusal test.
2. **US1's helpers.** US2's and US3's tests use US1's `pub(super)` rig and helpers
   (`Rig`, `Cause`, `refused`, `sep`, `m42`, `CENTRE`) from `super::centre`, and
   US2's the `send` of 055's `crates/finder/tests/motion/nudge.rs`; US1 writes them
   `pub(super)` from the start, as no story may edit an earlier story's test file.
3. **Track must settle before the frame.** A frame begun within Track's settle after
   Track on, or during a step, is refused as moved. The rigs wait 30 s after Track on
   and 12 s after a step; the browser test waits 12 s after `tracking` (on the probe a
   frame taken at once after Track on was refused).
4. **US3-S2 passes from US1.** It is US1's step under a stack (decision 3's evidence),
   placed in US3 for US1's size; it fails on 8b7212bb, where there is no route, and
   passes on US2's code. US3-S1 is the story's failing test.
5. **The finder camera's frames.** The rigs stop the node's star frames (`stop_frames`)
   to stand for a finder camera that cannot solve; the imaging camera's frames go on.
   With frames back and the aim point off, Track pulls a centring off within a minute
   (Edge Cases); US3-S2's second case starts them again only once a stack's subs steer.
6. **The browser suite's order.** The simulate project runs its files in name order on
   one server. A file that leaves Track's `Tracking ended` words in the guidance block
   breaks `goto.spec.ts`'s single status locator if it sorts before it; the file is
   `solve-centre.spec.ts` for that reason.
7. **The rate limit in US4-S1.** A refused call takes a slot; wait 1.1 s after the first
   call.

## Risks

| Risk | Effect | Mitigation |
|---|---|---|
| The one-sync model's scale | a long step misses by about 3 % of its size | within the bounds; the next solve shows it, and the run takes another try |
| An aim point that is off while the finder camera solves | Track pulls the centring off | the README says so; the run measures the aim point when the finder camera solves |
| Drift with no finder camera solves | 30″ to 50″ a minute on the simulator | a stack's steering holds it (US3-S2); the README says to centre again |
| 054 rewrites `ImagingPage.tsx` and `run/console.rs` | the block's place and `centre`'s place move | one element and one method; re-anchored before the flip |
| 055 changes before it lands | the moved bodies move with it | re-anchored on 055's landed text before the flip |

## Refinement Record

Drafted on 2026-11-27 by the plate-solver operator seat's spec session against 055's
probe code (`probe/055-r1` at 8b7212bb, `main` at 9c4a0a15 under it), from a probe of
that day on `probe/057` (five stories built with every scenario's test, both gates
passing on the tip, every story's tests but US3-S2 failing on the code before it, the
candidates measured on the simulator, the page run in a browser by day and by night),
and a reading of the imaging solve and the aim point, Track and its steady marks, 055's
step, the steering, the run's centring and its parts, MCP, the Imaging page, the
constitution, the glossary and the ADRs. Writes come from the task text. Shared writes
name the spec story that last wrote the same file on `main`; within the spec, each
story's files written by an earlier one are ordered by `depends_on_merged`. 055's own
writes (`nudge.rs` and its additions to the shared files) are not on `main`; they are
named again at the re-anchor (plan.md § Prerequisites).

None of the eight owner decisions of spec.md is answered; this record is written to the
recommendations. Its `open_questions` are empty because the validator refuses a record
that holds any; what is open is spec.md's owner decisions 1 to 8 and § Prerequisites
(the re-anchoring after 052 to 055). Nothing in this record is an owner approval: those
are in spec.md, and flipping to `ready` gives them.

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
      - path: crates/finder/tests/motion/centre.rs
        status: new
        disposition: add
        assertion: "the centring rig at 17: {} before a solve 409 exactly the imaging camera has not solved a frame yet; after a solve with Track off 409 Track is off: turn Track on to centre; Track on then 409 the telescope moved after imaging frame N: take another and solve it; 15 s, a new frame M posted, at once 409 imaging frame M is not solved yet: solve it first; saved, solve posted, at once 409 the imaging camera's solve is still running; solved: 200, then at once 409 the telescope moved after imaging frame M: take another and solve it; target null: 409 no target is selected"
        decision: "spec.md:US1-S2"
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
        assertion: "the centring rig at 17, 100 and 250, each pushed 0 (Track on at the true eyepiece, frames stopped): a solve; P the sky at pixel (300, 200), (1000, 700) and (618, 100) by ImagingSolve::from_cd of the solve's ra_deg, dec_deg and cd_deg with 1236 x 822 and sky_at; centre {frame_id F, x X, y Y} answers 200 with offset_arcsec within 5 of off P; once it rests off P at most 30, the place Track holds within 0.01 arcsec of P, track.target.id still NGC 1976"
        decision: "spec.md:US2-S1"
      - path: crates/finder/tests/motion/centre_tap.rs
        status: new
        disposition: add
        assertion: "the bare console {} 409 exactly the imaging camera has not solved a frame yet; the rig at 17: a foreign write 403, text/plain 415, {z 1} 400; solve A with Track off: {frame_id A} and {frame_id A, x 1.0} 400 exactly PARTS; Track on, 15 s, solve B: {frame_id B, x 1237.0, y 10.0} 409 exactly the point is outside imaging frame B; {frame_id A, x 10.0, y 10.0} 409 exactly imaging frame A is not the latest solved frame; target null; {frame_id B, x 618.0, y 300.0} 200, then at once 409 exactly the telescope moved after imaging frame B: take another and solve it"
        decision: "spec.md:US2-S2"
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
        assertion: "the centring rig at 17 pushed 120 at 120 with frames left stopped, and a new one with the aim point 120 off at 120 with frames started again after the centring rests: a solve, centre {} 200, rests, PUT /api/imaging/settings {exposure_ms 2000}, POST /api/imaging/stack {}, read until stack.stacked at least 15: steering.source subs and off M42 at most 10"
        decision: "spec.md:US3-S2"
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
        - "cargo nextest run -p finder --locked --lib -E 'test(/imaging::run::/)'"
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
      - path: crates/finder/tests/mcp/centre.rs
        status: new
        disposition: add
        assertion: "the MCP rig with the call log: tools/list has centre with a description containing real telescope, outputSchema type object, inputSchema frame_id [integer, null], x and y [number, null]; centre {} isError the imaging camera has not solved a frame yet; after 1.1 s {frame_id 1} and {q 1} isError exactly PARTS; aim at the true eyepiece, Track on, 30 s, frames stopped, pushed 240 in azimuth, 2 s, a solve: centre {} not an error, structuredContent moved track and offset_arcsec within 5 of off M42; at once motion.by mcp and motion.tool centre; the next two calls isError the telescope moved after imaging frame N: take another and solve it and the rate limit of 2 moving calls a second was reached; after axis_move done and 12 s off M42 at most 25; the call log has a line containing mcp centre and none containing test-token"
        decision: "spec.md:US4-S1"
      - path: crates/finder/tests/mcp/centre.rs
        status: new
        disposition: add
        assertion: "the committed README keeps The imaging camera has sixteen tools of its own; Centring from an imaging solve has one tool: lies after The page's nudge arrows have one tool: and before ### MCP under WSL2 with a line starting - `centre` takes nothing after it; the ## API section has a row starting | `POST /api/imaging/centre` |; ## Centring from an imaging solve lies after ## Nudging the telescope and before ## Recording a night and, whitespace made single, contains It needs no live pointing, the telescope moved after imaging frame N: take another and solve it (with its closing backtick) and A centring moves no aim point and is never a sync.; the glossary's **Centring** follows **Nudge** and starts with FR-012's words and _Avoid_: recentre"
        decision: "spec.md:US4-S2"
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
  US5:
    tests:
      - path: crates/finder/web/src/components/CentreBlock.test.tsx
        status: new
        disposition: add
        assertion: "the page rig with fetch answering {frame_id 3, offset_arcsec 386.6, moved track}: Centre M42 posts /api/imaging/centre, POST, Content-Type application/json, body {}; onChange once; the status exactly Centring M42: 387″ from the centre… after the answer and after axis_move {state moving, tool centre}, exactly Centred M42 · Track holds it. Take another imaging frame to check. after {state done, tool centre}"
        decision: "spec.md:US5-S1"
      - path: crates/finder/web/src/components/CentreBlock.test.tsx
        status: new
        disposition: add
        assertion: "fetch answering {frame_id 3, offset_arcsec 834, moved track}, the picture laid out at left 10, top 20, 618 x 411: the image Solved imaging frame 3: tap a point to centre it has src /api/imaging/frames/3/preview.jpg; a click at client (319, 122.75) posts the body {\"frame_id\":3,\"x\":618,\"y\":205.5}; the status exactly Centring the tapped point: 13.9′ from the centre…"
        decision: "spec.md:US5-S2"
      - path: crates/finder/web/src/components/CentreBlock.test.tsx
        status: new
        disposition: add
        assertion: "fetch answering 409 {error Track is off: turn Track on to centre}: Centre M42 shows that alert exactly and no status; with the solve of frame 2 Centre M42 disabled and no image; without a target no button and still the image"
        decision: "spec.md:US5-S3"
      - path: crates/finder/web/src/components/CentreBlock.test.tsx
        status: new
        disposition: add
        assertion: "ImagingPage from the recorded session tracking with the page rig's view: the region Centre follows the region Imaging solve in the document and holds the button Centre M42"
        decision: "spec.md:US5-S4"
      - path: crates/finder/web/e2e/solve-centre.spec.ts
        status: new
        disposition: add
        assertion: "the simulated console by day, Track tracking and 12 s, the Imaging page: Take imaging frame and Solve imaging frame in their regions until Saved and Solved; Centre M42 response 200 moved track, the status exactly Centred M42 · Track holds it. Take another imaging frame to check. within 20 s, track.state tracking; 12 s, another frame solved; a click on the Centre block's picture at a quarter of its width and half its height posts exactly the keys frame_id, x and y with x within 3 of 309 and y within 3 of 411, response 200, the status exactly Centred the tapped point · Track holds it. Take another imaging frame to check. within 20 s"
        decision: "spec.md:US5-S5"
      - path: crates/finder/web/e2e/solve-centre.spec.ts
        status: new
        disposition: add
        assertion: "a frame solved as in US5-S5: the Centre block's day screenshot has more than 1000 off-red pixels; Night mode: red and black on, data-theme night on html, 1.5 s: the night screenshot has none"
        decision: "spec.md:US5-S6"
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
        - "npx playwright test --project=simulate e2e/solve-centre.spec.ts e2e/goto.spec.ts e2e/imaging.spec.ts"
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
