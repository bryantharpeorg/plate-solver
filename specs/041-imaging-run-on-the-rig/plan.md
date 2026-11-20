# Implementation Plan: The Imaging Run on the Console's Own Parts

**Branch**: `041-imaging-run-on-the-rig` | **Date**: 2026-11-15, a spec of its own since 2026-11-20 | **Spec**: [spec.md](spec.md)

This plan records every design decision for the feature. A story that would change one
escalates rather than re-deciding it. The binding decisions above it are
`docs/plans/2026-11-15-specs-030-037-imaging.md` (the plan document),
`docs/adr/0002-imaging-cameras-through-indi-frames-relayed-console-stacks.md` and
`GLOSSARY.md`.

## Summary

`037-imaging-run` has landed a `Runner` that carries one imaging run through five parts
it is handed, each a small trait: the mount, the camera, the focus check, the solver and
the stack, with three routes, the run's record and a scripted stand-in for the parts.
This spec gives the run the console's own parts and the steps of the machine that need
them. US4 writes the console's own mount, camera and solver parts: pointing through
Track, the lease of the imaging camera, and 032's solve. US5 adds centring to the machine
and, to the mount part, the wait for a live pointing that a re-centre needs. US6 adds the
focus gate and its pause over 033; US7 the limits and the stack's own endings; US8 the
stack part over 035 and the steering of 036 and 040, and the binary's wiring.
`042-imaging-run-page-and-mcp` then adds the run block to the Imaging page, four MCP tools
and the README section. The run asks Track and nothing else to move the mount, asks
nothing of the mount when it ends but a Stop when it is itself stopped while still
slewing, and holds no number sized to one telescope.

## Technical Context

**Language/Version**: Rust (MSRV 1.96).

**Primary Dependencies**: nothing new. `finder` already has `tokio` with `sync` and
`time`, `serde` and `serde_json` (`crates/finder/Cargo.toml:32`), and `rmcp`.

**Storage**: `run.json` under the console's imaging folder (031: `imaging/` in the state
folder, or `--imaging-dir`): in the stack's folder, `runs/<stack id>/`, beside 035's
`stack.json`, `stack.fits` and `stack.png`, or in `runs/<run id>/` for a run that ends
before it stacks. The stack's subs are in its folder's `subs/` (035's owner decision
13); the run's own imaging frames stay under `frames/`, where 031 saves them.
Nothing about a run is kept in `session.json`: it is run-only, like Track.

**Testing**:
- US4: `crates/finder/tests/motion/imaging_run.rs` (new), the module `imaging_run` of
  `motion`, on the run rig, where the simulated mount moves: the console's own mount,
  camera and solver parts under a run and, for US4-S5, the mount part alone.
- US5: `crates/finder/tests/console/imaging_run_centring.rs` (new), the module
  `imaging_run_centring` of `console`, scripted; and
  `crates/finder/tests/motion/imaging_run_centring.rs` (new), the module
  `imaging_run_centring` of `motion`, on the run rig. The two modules share a name and
  no code: each test binary has its own module tree.
- US6: `crates/finder/tests/console/imaging_run_focus.rs` (new), the module
  `imaging_run_focus` of `console`: scripted, and for US6-S7 the console's own camera
  and focus parts with a scripted mount, so no mount moves.
- US7: `crates/finder/tests/console/imaging_run_limits.rs` (new), the module
  `imaging_run_limits` of `console`, scripted.
- US8: `crates/finder/tests/motion/imaging_run_stack.rs` (new), the module
  `imaging_run_stack` of `motion`, on the run rig with all five of the console's own
  parts; and `crates/finder/tests/cli/imaging_run.rs` (new), the module `imaging_run`
  of `cli`, against the built binary.
- A rig test writes its own rig. US4, US5 and US8 each build the run rig in their own
  module, because a test file an earlier story created is an existing test and
  `crates/finder/tests/support` takes no new file; the rig is about fifty lines.
- Where a finder test goes is 027's rule, by what it needs to run: `console` calls the
  router in process and moves no mount (`crates/finder/tests/console/main.rs:2`), `cli`
  starts the built `finder` (`crates/finder/tests/cli/main.rs:1`), `mcp` goes through
  the MCP door, and `motion` drives the simulated mount in paused time
  (`crates/finder/tests/motion/main.rs:1`). Each module is one file in its binary's
  folder and one `mod` line, in alphabetical order, in that binary's `main.rs`. A module
  writes its own setup; `crates/finder/tests/support` takes no new file
  (`crates/finder/tests/layout/main.rs:127`), and an unused helper fails
  `clippy -D warnings`. It takes its scratch folder from `crate::support::Scratch`
  (`crates/finder/tests/support/mod.rs:12`).
- Every Rust test but the `cli` one runs under `#[tokio::test(start_paused = true)]`, so
  the 900 s of a focus pause, the 60 s of a time limit, the 600 s without a stacked
  sub, the 5 s between two takes and the 5 s a re-centre may wait
  are asserted exactly and none is waited for in real time.
- Every rig test uses the simulated imaging camera at its small default size, 1236 ×
  822 pixels; no test of this spec uses a real camera's size.

**Target Platform**: the `finder` console on the console machine. Nothing changes on the
scope node.

**Constraints**:
- No `unsafe`, no dependency, no `Cargo.toml` change, no scope-link change.
- No change under `crates/simulator`, `crates/scope-node` or `crates/scope-link`.
- Mount-neutral (the plan document, § Mount-neutral by design):
  `crates/finder/src/imaging/run.rs` and the files under `crates/finder/src/imaging/run/`
  call Track on, Track off, Stop and the imaging module, and nothing of
  `crate::goto::Planner`, `crate::tracking`, `crate::ephemeris`, `crate::model` but its
  great-circle `separation`, or `scope_link::proto`. None holds an identifier, an import
  or arithmetic for an axis, an altitude, an azimuth, a rate or axis counts, or a focal
  length, a field size or a scale as a number (§ Plan traps 18).
- No lock of the run's own on the imaging camera: 031's one lease (031 US9), taken and
  given back (the plan document, amendment 1).
- Budgets are things a test measures; this plan states none for the real telescope.

**Scale/Scope**: the module's third child, `console.rs`, the steps of 037's machine that
need its parts, and the binary's line, in five stories.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Nothing touches the solver, detection or `crates/math-core`. The order of the steps, the centring tolerances in use elsewhere and the forced stop of a solve loop that never converges are grounded in `docs/research/2026-11-15-imaging-camera.md` § 12. Prior art and what was not adopted; the 60″ of GoTo and the tracker's endings in the code cited in 037's history.md § Clarifications. The run's own counts and limits are owner decisions, named as such. |
| II. Never weaken a gate | No existing test, golden, fixture, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS`, `ergane.yaml` or `playwright.config.ts` is edited. The only edits to existing test files are the `mod` lines spec.md SC-002 names. |
| III. Tests first, independent expectations | Oracles: for the machine, the words, states and call lists each scenario states, compared with what a scripted stand-in recorded; for the real parts, the simulator's truth (the true aim point, the true centre of the imaging frame, the mount's true motion), the session, and the answers of 031's and 033's own routes. `run.json` and `stack.json` are parsed by the tests themselves. Clocks are injected; virtual time only in tests and `finder --simulate`. |
| IV. Critical paths | No `unsafe`, no dependency or `Cargo.toml` change, no database-format change, nothing under `proto/`, nothing in `crates/scope-link`. |
| V. Proven open source first, honest licensing | The finder camera's solver is untouched. The run reaches ASTAP only through 032's solver port; principle V of constitution 1.3.0 allows that. |

## Project Structure

```text
crates/finder/
├── src/api.rs                            # US8: with_imaging_run
├── src/main.rs                           # US8: .with_imaging_run()
├── src/imaging/run.rs                    # US4: pub mod console; US5: centring; US6: the pause; US7: limits
├── src/imaging/run/console.rs            # US4: new, mount, camera and solver parts; US5: the wait of repoint; US6: focus part; US8: stack part
├── tests/console/main.rs                 # US5, US6, US7: one mod line each
├── tests/console/imaging_run_centring.rs # US5: new
├── tests/console/imaging_run_focus.rs    # US6: new
├── tests/console/imaging_run_limits.rs   # US7: new
├── tests/motion/main.rs                  # US4, US5, US8: one mod line each
├── tests/motion/imaging_run.rs           # US4: new
├── tests/motion/imaging_run_centring.rs  # US5: new
├── tests/motion/imaging_run_stack.rs     # US8: new
├── tests/cli/main.rs                     # US8: mod imaging_run
└── tests/cli/imaging_run.rs              # US8: new
```

`run.rs`, `script.rs`, `store.rs` and the routes are 037's; the page, the MCP tools and the
README are `042-imaging-run-page-and-mcp`'s.

## What 037-imaging-run provides (read as landed before the flip)

`037-imaging-run` has landed all of this before any story here is built, and no story
here changes any of it: a test file 037 created is an existing test. 037's plan.md gives
each item in full, under the section named; the flip re-reads this section against the
code on `main` (flip.md § Prerequisites 2).

**The parts and the words** (`crates/finder/src/imaging/run.rs`; 037's § The parts and
the words). The five traits and `Steps` are whole, and no story adds a method or a field;
the console's own parts implement them, as the script does:

```rust
pub type StepFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum By { Page, Mcp }
impl By { pub fn as_str(self) -> &'static str; }          // "page", "mcp"

/// Why a run is over. The whole enum, its reasons and its messages are here from 037's US12;
/// a later story adds only the code that reaches a variant.
#[derive(Debug, Clone, PartialEq)]
pub enum Ending {
    /// The mount was stopped: the console's Stop, or Track turned off.
    Stopped,
    /// The run's own stop, with the mount left as it was. Its reason is `stopped` too.
    RunStopped,
    /// The stack was stopped from its own route, or ended with no failure. `stopped`.
    StackStopped,
    /// The run's focus check was stopped from 033's own route. `stopped`.
    CheckStopped,
    CountLimit, DurationLimit, TooLow, SunUp, LinkLost,
    /// With the words Track or GoTo gave.
    MountFailed(String),
    TargetChanged,
    /// 038's Track view shows a push by hand that settled or lasted.
    PushedAway,
    CameraFailed(String), SolverUnavailable(String), SolverFailing(String), SaveFailed(String),
    FocusTimeout,
    /// `measured` is false when the run ended on solves that measured no new aim point.
    NotCentred { offset_arcsec: f64, measured: bool },
    /// With the reasons that rose, as `not solved 7, too few stars 3`.
    SubsFailing(String),
    StackFailed(String),
}
impl Ending {
    /// Its word in `ENDINGS`. Four variants answer `stopped`.
    pub fn reason(&self) -> &'static str;
    /// The sentence of 037's plan.md § The run's words. `name` is the target's.
    pub fn message(&self, name: &str, settings: &RunSettings) -> String;
}

/// Why a part did not do what it was asked.
#[derive(Debug, Clone, PartialEq)]
pub enum Failure {
    /// The part was stopped from outside the run: the console's Stop for the mount,
    /// 033's own stop for a focus check.
    Stopped,
    Camera(String),
    Save(String),
    /// The imaging solver cannot run at all.
    SolverUnavailable(String),
    /// The imaging solver ran and found no solution; or a focus check found no stars
    /// to measure.
    NotSolved(String),
    Mount(String),
    Other(String),
}
impl Failure { pub fn words(&self) -> &str; }              // "stopped" for `Stopped`

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RunTarget { pub id: String, pub name: String, pub ra_deg: f64, pub dec_deg: f64 }

#[derive(Debug, Clone, PartialEq)]
pub enum MountState {
    /// The mount is being brought to the target.
    Slewing,
    /// `steady` is false while the tracker says it is not steady enough to expose.
    Tracking { steady: bool },
    /// The motion is over, and why.
    Ended(Ending),
}
pub trait MountSteps: Send + Sync {
    /// The target a run would go to, or why there is none.
    fn target(&self) -> Result<RunTarget, String>;
    /// Ask Track for that target. A refusal's words, unchanged.
    fn point(&self, by: By) -> StepFuture<'_, Result<(), String>>;
    fn state(&self) -> MountState;
    /// Ask Track for the target again, so that it comes to the aim point in force.
    fn repoint(&self) -> StepFuture<'_, Result<(), Failure>>;
    /// Carry out the console's Stop. The run asks only when it is stopped while it
    /// still slews.
    fn stop(&self) -> StepFuture<'_, ()>;
    /// Whether Track is on now, in any state but off.
    fn track_on(&self) -> bool;
    /// Whether the tracker was steady for all of a window of console Unix seconds.
    fn steady_between(&self, start: f64, end: f64) -> bool;
}

#[derive(Debug, Clone, PartialEq)]
pub struct FrameRef { pub id: u64, pub file: String }
pub trait FrameSteps: Send + Sync {
    /// Whether the imaging camera can be used at all, or why not.
    fn ready(&self) -> Result<(), String>;
    /// Take the one lease of the imaging camera for this run. A refusal's words,
    /// unchanged.
    fn hold(&self) -> Result<(), String>;
    /// Give the lease back. Safe when it is not held.
    fn release(&self);
    /// Take one imaging frame and wait until it is saved.
    fn take(&self) -> StepFuture<'_, Result<FrameRef, Failure>>;
    /// Give up the exposure in flight.
    fn abort(&self) -> StepFuture<'_, ()>;
}

/// 033's five verdicts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict { Good, SlightlySoft, ClearlyOut, Unmeasurable, NoRecord }
impl Verdict {
    /// `good`, `slightly_soft`, `clearly_out`, `unmeasurable` or `no_record`.
    pub fn as_str(self) -> &'static str;
}
/// The unit a focus reading and a focus record are in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit { Arcsec, Px }
impl Unit { pub fn as_str(self) -> &'static str; }         // "arcsec", "px"
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FocusCheck {
    pub verdict: Verdict,
    pub reading: Option<f64>,
    pub record: Option<f64>,
    pub unit: Unit,
}
pub trait FocusSteps: Send + Sync {
    /// Check focus and wait for the verdict.
    fn check(&self) -> StepFuture<'_, Result<FocusCheck, Failure>>;
    fn open_aid(&self) -> StepFuture<'_, Result<(), Failure>>;
    /// Close the focus aid, or stop a check that is running. Safe when neither runs.
    fn close_aid(&self) -> StepFuture<'_, ()>;
}

/// How far the target is from the set point, by one imaging solve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Centre {
    pub offset_arcsec: f64,
    pub tolerance_arcsec: f64,
    /// A quarter of the imaging frame's shorter side on the sky: a target this near
    /// after the last try is stacked with a warning.
    pub near_arcsec: f64,
    /// Whether the solve moved the aim point, or left a measured one held back until
    /// Track is off: a re-centre is asked for only then.
    pub aim_moved: bool,
}
pub trait SolveSteps: Send + Sync {
    fn ready(&self) -> Result<(), String>;
    fn solve(&self, frame: &FrameRef, target: &RunTarget) -> StepFuture<'_, Result<Centre, Failure>>;
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct StackProgress {
    pub running: bool,
    pub stacked: u64,
    /// By the stack's own reason words; only reasons whose count is above 0.
    pub dropped: BTreeMap<String, u64>,
    pub waiting_for_tracker: bool,
    /// Why the stack ended by itself, when it did.
    pub failed: Option<Failure>,
}
pub trait StackSteps: Send + Sync {
    /// Start a stack on the target. Answers the stack's folder, relative to the
    /// imaging folder.
    fn start(&self, target: &RunTarget) -> StepFuture<'_, Result<String, Failure>>;
    fn progress(&self) -> StackProgress;
    /// Stop taking subs and save the stack; with `keep_subs` false, then remove the subs.
    fn stop(&self, keep_subs: bool) -> StepFuture<'_, Result<(), Failure>>;
}

/// The five parts. The five traits and this struct are complete from 037's US12: no later
/// story or spec adds a method or a field, because the script implements them all.
#[derive(Clone)]
pub struct Steps {
    pub mount: Arc<dyn MountSteps>,
    pub camera: Arc<dyn FrameSteps>,
    pub focus: Arc<dyn FocusSteps>,
    pub solver: Arc<dyn SolveSteps>,
    pub stack: Arc<dyn StackSteps>,
}
```

and `POLL`, 250 ms, how often a run reads the mount and the stack; `label(name)`, a
target's name as a folder's name takes it, which calls 035's
`crate::imaging::stack::service::label`; and `Ending::reason` and `Ending::message`, with
every ending's sentence.

**The machine** (`run.rs`; 037's § The machine and 037's plan.md § Failed steps and counts in the
machine). `Runner::new`, `Runner::with_folder`, `start`, `stop`, `resume` and `view` keep
their signatures. `RunView` is `#[non_exhaustive]` and holds the keys of 037's FR-004,
`centring`, `pause_ends_at` and `warnings` among them. The latest run lies under one std
`Mutex` under which no part is called and nothing is awaited (§ Plan traps 23). The task,
as 037 leaves it for the stories here to change:
- `guarded(step)` races a step against the stop signal and the mount's ending, read
  every `POLL`; `steady(show)` waits until the mount part reads `Tracking { steady:
  true }`, adding the words of a wait for the tracker to the state's message; `frame()`
  is `steady(None)` and then a take, a failed take counted up to `FRAME_TRIES` in a row,
  `RETRY_WAIT` apart.
- Step 3, the focus check: the state `focus_check`, `steady(None)`, `camera.release()`
  and `guarded(focus.check())`. In 037 every verdict goes on, `SlightlySoft` with its
  warning; a check that did not finish sets `focus` to `unmeasurable` in `px`, adds
  `The focus check did not finish: {words}.`, keeps the failure for the words of a pause
  and goes on as `Unmeasurable` does; `Save` and `SolverUnavailable` end the run, and
  `Stopped` ends it `CheckStopped`. Every way on to step 4 first takes the lease again
  with `camera.hold()`. US6 puts the pause here.
- Step 4, the solve: the state `solving`, `frame()`, then `guarded(solver.solve(..))`, a
  solve with no solution counted up to `SOLVE_TRIES` in a row; `Ok(centre)` sets
  `centring` to `{tries: 0, offset_arcsec, tolerance_arcsec}` and goes on to stacking.
  US5 makes this step a loop.
- Step 5: the state `stacking`, `camera.release()`, then `stack.start(&target)`,
  unguarded; `Ok(folder)` sets `folder`, `stacking_since` and `limit_at`, and the record
  adopts the folder; a failure ends the run by the table below, with no `stack.stop`.
- Step 6, every `POLL`: `stack.progress()` copied into `subs`; then the mount's
  `Ended(e)` and a stop of the run; then `paused_by_tracker` while
  `waiting_for_tracker`, else `stacking`, each with its message. US7 puts its rules
  after the stop and before the state.
- The ending, in its order: the message `Saving the stack: {stacked} stacked.` once the
  stack has started; `focus.close_aid()` once, when a check was given up; then, when step
  5 answered `Ok`, `stack.stop(keep_subs)`, whoever ended the stack, and the counts read
  once more; `camera.release()`, always and last; then `ended` with `saved` and
  `track_on`, and the record written. US6 adds an ending from a pause to the first rule.
  `stack.start`, `stack.stop`, `focus.open_aid`, `focus.close_aid`, `mount.repoint` and
  `mount.stop` are awaited unguarded, and a stop that arrives during one is acted on as
  soon as it has answered.
- Two rows of 037's table of failed steps are this spec's to reach:

| A failed step | `Stopped` | `Camera` | `Save` | `SolverUnavailable` | `NotSolved` | `Mount`, `Other` |
|---|---|---|---|---|---|---|
| `stack.start` (037's US2), and `StackProgress::failed` (US7) | `Stopped` | `CameraFailed` | `SaveFailed` | `SolverUnavailable` | `SolverFailing` | `StackFailed` |
| `mount.repoint` (US5) | `Stopped` | `MountFailed` | `MountFailed` | `MountFailed` | `MountFailed` | `MountFailed` |

- 037's § The run's words is whole from 037's US1, with the words this spec's code first
  reaches: both messages of `centring`; the message of `focus_pause` for each verdict
  that pauses and for a check that did not finish; the warnings for a target stacked off
  the centre, after the last try and after four solves that measured no new aim point,
  each in two forms chosen against 036's `FAR_ARCSEC`; and the warnings for going on
  anyway, for an aid that could not be opened and for a check twice not steady. A
  warning already in the list is not added again.

**The routes and the record** (037's § The console, 037's plan.md § The routes, 037's plan.md § The record).
`POST /api/imaging/run`, `DELETE /api/imaging/run` and `POST /api/imaging/run/continue`;
the imaging view's `run` in every state of the view; `AppState::with_imaging_run_steps`
and `AppState::imaging_runner`; and `run.json` in the stack's folder once the run
stacks, holding `ended` and the counts read after the stack stopped. The route starts a
run inside `api::start_motion` with `page` and `imaging_run`, which holds the motion lock
around `Runner::start`.

**The script and the fixtures of the scenarios** (`finder::imaging::run::script`;
037's § The script and 037's spec.md § User Scenarios), which every scripted test here
uses and no story here writes. A call is listed when it is made, before any hold:

- **The script** is `finder::imaging::run::script::Script` (037's US12): one stand-in for all
  five parts of a run. Left alone it answers: the target M42 (`id` `NGC 1976`, `name`
  `M42`, RA 83.82°, Dec −5.39°, the script's own constants); a camera and a solver that
  are ready and a lease that is free; a mount that is tracking and steady; imaging
  frames with ids 1, 2, 3 … and the file `frames/2026-01-16/00000N.fits`; a focus check
  that is good with a reading and a record of 3.0″; a solve 0″ from the sensor's centre
  with a tolerance of 90″ and a `near_arcsec` of 450″ that moved the aim point; a
  tracker that was steady over any past window; and a stack that starts in the
  folder `runs/2026-01-16T02-00-10Z-M42`, **the script's stack folder**, and runs with
  nothing stacked or dropped until it is stopped. A test changes one answer at a time:
  `set_mount`, `refuse`, `push_frame`, `push_focus`, `push_solve`, `fail_repoint`,
  `fail_open_aid`, `fail_stack_start`, `fail_stack_stop`, `unsteady`, `set_stack` and
  `end_stack`. `unsteady(n)` makes the mount answer the next n questions about a past
  window, `steady_between`, with no. The script's Track is on unless its mount is set
  to an ending that stops the mount (`Stopped`, `TooLow`, `SunUp`, `LinkLost`,
  `MountFailed`).
  `set_stack(stacked, dropped, waiting_for_tracker)` sets the stack's counts and whether
  it waits for the tracker, `dropped` a list of reason and count, and never ends the
  stack; `end_stack(failed)` makes it end by itself. `hold(step)` makes every call
  of a step wait, once it is listed, until `release(step)`, which also ends the hold.
  `calls()` is the list of calls the run made, in order, as 037's plan.md § The script words
  them; reading the mount's or the stack's state, whether Track is on, or whether the
  tracker was steady over a window is not listed.
- **A scripted run** is a `Runner::new(script.steps(), clock)` whose clock is a
  `FixedClock` at Unix 1768528800 (`2026-01-16T02:00:00Z`,
  `crates/finder/src/session.rs:41`), started with
  `runner.start(RunRequest::default(), By::Page)` unless the scenario gives another
  request. **Settling** is sleeping 1 s of virtual time, four of the run's polls, before
  reading `runner.view()`.
- **The straight path** is what `calls()` holds when a scripted run has reached
  `stacking` with nothing changed: `camera.ready`, `solver.ready`, `mount.target`,
  `camera.hold`, `mount.point page`, `camera.take`, `camera.release`, `focus.check`,
  `camera.hold`, `camera.take`, `solver.solve 2`, `camera.release`, `stack.start M42`.
  The first imaging frame has the id 1 and the one that is solved the id 2. The lease
  is given up around the focus check and before the stack, which take it themselves.
- **The scripted console** is 031's console rig (031's spec.md § User Scenarios: a
  `Simulator` at 40° N 75° W on a `VirtualClock` from Unix 1768528800, a `SimNode` with
  the token `test-token`, a paired session, `with_link`, `with_live()` and
  `with_imaging(R)`), its session kept in `session.json` under the test's scratch
  folder (`Session::with_state_file`), with `with_imaging_run_steps(script.steps())`
  added last. **R** is the child path `imaging` of the test's scratch folder.

## Interfaces

Everything 030 to 036, 038, 040 and 043 create is named as their plan.md gives it in the
drafting worktree, and 037's as its plan.md gives it. The lease of the imaging camera,
which the plan document's amendment 1 gives 031, is 031's US9; 033, 034 and 035 take the
same lease. All of it is re-read on `main` before the flip (flip.md § Prerequisites 2
and 3).

### What 030 to 036, 038, 040 and 043 offer the console's parts (read, never written, by this spec)

| What a part needs | From | Name | Where it is written |
|---|---|---|---|
| the camera's state, description, settings and latest imaging frame | 031 | `Imaging::view()` → `ImagingView { state, camera, settings, frame, .. }`; `SettingsView::roi`; `CameraView::{width, height}` | 031 plan.md § The console's imaging state |
| one imaging frame, awaited, and its abort | 031 | `Imaging::take(purpose)` → the whole `ImagingView`, whose `frame` may be absent; `Imaging::ended(id)` → `FrameView`; `Imaging::abort()` | 031 plan.md § Receiving and saving |
| why a request was not carried out | 031 | `crate::imaging::Refusal { status, message }` | 031 plan.md § The console's imaging state |
| the one lease of the imaging camera, and an imaging frame taken by its holder | 031 | `Imaging::lease(holder)` → `Lease`, released when dropped; `Lease::take(purpose)`; a refusal `the imaging camera is held by {holder}` | 031 US9 (031 plan.md, its section on the lease); the plan document, amendment 1 |
| the imaging folder | 034 | `Imaging::root()` | 034 plan.md § 031's imaging state |
| whether there is an imaging solver | 032 | `Imaging::solver()` | 032 plan.md § Solving on the console |
| solve the latest imaging frame, awaited | 032 | `Imaging::solve()`, `Imaging::solved(frame_id)` → `SolveView { state, ra_deg, dec_deg, arcsec_per_px, reason, error, .. }` | 032 plan.md § Solving on the console |
| whether a solve moved the aim point | 032 | `SessionView::aim` → `Option<AimView { x, y, source }>`, read before the solve and after it; `Session::held_aim()` → `Option<Aim>`, the measured aim point held back while Track is on, read after it; 032's task hands a measured aim point to `Session::set_measured_aim`, which holds it back while Track is on, before the solve reads `solved` | 032 plan.md § The session (the held aim point), § The relation, § Solving on the console |
| the focus check, awaited | 033 | `Imaging::focus_start(FocusAsk { check: true, .. })`, `Imaging::focus_ended()` → `FocusView { unit, aid, .. }`, `AidView { check, state, verdict, median, last, reference, ended_reason, error, .. }` | 033 plan.md § The aid on the console, § The focus view |
| the focus aid, opened and closed | 033 | `Imaging::focus_start(FocusAsk::default())`, `Imaging::focus_stop()` | 033 plan.md § The aid on the console |
| the live stack | 035 | `AppState::stack()` → `Option<Arc<Stacker>>`; `Stacker::start(StartRequest, Option<&str>)`; `Stacker::stop()` and `Stacker::view()` → `Option<StackView>`, `StackView { state, id, started_at, stacked, dropped, ended_reason, error, save_error, .. }`; `dropped`'s keys `behind` and `unprocessed`, which are not failed subs | 035 plan.md § The stack on the console, 037's plan.md § The console |
| whether the stack waits, for the tracker or for its exposure limit, and the set point | 036, 040 | `Imaging::steering()` → `Steering::set_set_point(None)` (036), `Steering::waiting()` (040) | 036 plan.md § The steering, § The imaging state; 040 plan.md, where it adds `waiting` to the steering |
| how far from the set point a stack may begin and still be steered | 036 | `crate::imaging::steering::FAR_ARCSEC`, 180.0 | 036 plan.md § The steering; 036 spec.md FR-014 |
| whether the tracker is steady enough to expose, and was over a past window | 036, 040 | `Session::steered()` → `Option<Arc<dyn Steered>>`; `Steered::steady()`, `Steered::steady_between(start, end)` over console Unix seconds, the trait's (036) and the tracker's answers to both (040 FR-021) | 036 plan.md § The session, § The tracking error and the interface; 040 plan.md, the tracker's two answers |
| how far back an aim reading fits the finder camera's solves | 032 | `crate::imaging::relation::FIT_REACH_S`, 10 s, or the exposure when that is longer | 032 plan.md § The relation |
| a push by hand, and whether it has settled | 038 | `TrackView::push` → `moving` or `following`, absent with no push | 038 spec.md FR-009 |
| an abort, and a stack's stop that awaits one, that answer on a scope node that has stopped answering | 043 | `Imaging::abort()` answers within `crate::imaging::ABORT_WAIT`, 5 s, whether or not the node answers, the frame having then ended `aborted`; so `Stacker::stop()` answers within that and the stack's save, the stack `ended` and `stopped`. The camera part's `abort` and the stack part's `stop` await them as this plan writes them, with no race or limit of their own, and a run's ending finishes on a lost link (US8-S6) | 043 plan.md § What this gives later specs |

### The console's mount part (`crates/finder/src/imaging/run/console.rs`, new, US4; the wait of `repoint`, US5)

```rust
//! The console's own parts of an imaging run (037): Track, 031's imaging frames and
//! lease, 032's imaging solve, 033's focus check and 035's stack. Sky terms only:
//! nothing here knows an axis.
pub const CENTRE_SHARE: f64 = 0.05;
pub const CENTRE_FLOOR_ARCSEC: f64 = 90.0;
// US5:
/// Polls a re-centre waits, at most, after its own Track off, for the finder camera's
/// next live pointing: 5 s at `POLL`.
pub const FRESH_POLLS: u32 = 20;
// US4:
/// How long 038's push may read `moving` before it ends a run.
pub const PUSH_GRACE: Duration = Duration::from_secs(10);
/// The share of the imaging frame's shorter side within which a target that could not
/// be centred is stacked all the same.
pub const CENTRE_NEAR_SHARE: f64 = 0.25;

pub fn mount(state: &AppState) -> Arc<dyn MountSteps>;
pub fn camera(state: &AppState) -> Arc<dyn FrameSteps>;
pub fn solver(state: &AppState) -> Arc<dyn SolveSteps>;
// US6: pub fn focus(state: &AppState) -> Arc<dyn FocusSteps>;
// US8: pub fn stack(state: &AppState) -> Arc<dyn StackSteps>;
```

Each part keeps a clone of the `AppState` it was made from. The mount part also keeps
the id of the target it last answered from `target`; from `point` on, the stop
generation and `by`; a flag, `repointing`, that is true only between its own Track off
and Track's answer; and the tokio instant at which `state` first read 038's push as
`moving`, cleared whenever `state` reads no push.

- **`target`**: `state.session().snapshot(state.clock.now()).target`
  (`crates/finder/src/session.rs:1002`). None: `Err("no target is selected")`. Else
  `RunTarget { id, name: messier or id, ra_deg, dec_deg }`
  (`crates/finder/src/targets.rs:39`). It keeps the id it answers.
- **`point(by)`**: without `state.goto()` (`crates/finder/src/api.rs:296`),
  `Err("no mount is connected")`. Before it asks Track it reads the session's target
  again: none is `Err("no target is selected")`, and an id other than the one `target`
  last answered, when it has answered one, is `Err("the target was changed")`. It reads
  `*goto.stops().borrow()`
  (`crates/finder/src/goto.rs:801`), then awaits `api::start_tracking(&state)`
  (`crates/finder/src/api.rs:831`), the function the Track route and the MCP
  `set_tracking` call; its error's `message` is the refusal, unchanged. On `Ok` it keeps
  the generation, `by`, and the id it read before it asked, not a new read. When
  Track's own `track.target.id` is then another, because the target was changed while
  Track was being asked, which can take 2 s, it still answers `Ok`: the first `state()`
  answers `Ended(TargetChanged)` by its second row. So the run never images one target
  under a session that names another. It does not call `start_motion`: the route
  already holds the motion lock around `Runner::start`, and that lock is not re-entrant.
- **`state`**, the first that holds:

  | What the part reads | It answers |
  |---|---|
  | the stop generation differs from the one kept | `Ended(Stopped)` |
  | the session's target is none, or its id is not the one kept, or `track.target.id` is not the one kept | `Ended(TargetChanged)` |
  | `track.push` is `following` (038's FR-009), or has read `moving` at every `state()` since one that first read it `PUSH_GRACE` or more ago on tokio's clock, while `repointing` is not set | `Ended(PushedAway)` |
  | `track.push` `moving` for less than that, while `repointing` is not set | `Tracking { steady: false }` |
  | no `track` in the snapshot | `Ended(MountFailed("tracking is off"))` |
  | `track.state` `armed` | `Slewing` |
  | `track.state` `off`, `ended_reason` `stopped`, while `repointing` is set | `Slewing` |
  | `off`, `stopped` otherwise, or `the observer stopped it` | `Ended(Stopped)` |
  | `off`, `link_lost` or `halted by a lost link` | `Ended(LinkLost)` |
  | `off`, `sun_up` or `halted by the Sun guard` | `Ended(SunUp)` |
  | `off`, `too_low` | `Ended(TooLow)` |
  | `off`, any other reason, or none | `Ended(MountFailed(reason or "tracking ended"))` |
  | any other `track.state` | `Tracking { steady }`, `steady` being `state.session().steered().is_some_and(\|t\| t.steady() && t.steady_between(now - reach, now))` (036, 040), with `now` the console clock and `reach` the larger of `crate::imaging::relation::FIT_REACH_S` and the imaging view's `settings.exposure_ms` in seconds |

  **Steady means steady for the whole of 032's reach.** 032's aim reading fits the
  finder camera's solves from `reach` before the imaging exposure, and a fit that
  reaches into a GoTo, a re-centre or a take-up reads `moving`, or is biased. 040's
  `steady_between` is false from a tracker's start until 2 s after its first rates,
  across every take-up and keyhole pass with the 2 s that follow each, across a
  measurement, while 038's push reads `moving`, and for a turn that finds an axis
  stopped; a window that begins at the instant such a time ended is not touched by it
  (040 FR-021). Every GoTo and every re-centre starts a new tracker, so the row needs no
  clock and no field of its own. The window
  is in console Unix seconds, which is what 036 keeps its list in. The cost is the
  larger of 10 s and the exposure, after the 2 s to 3 s for which a new tracker settles
  (040's US10-S3 pins this window on 040's own rig: false asked 6 s after a tracker's
  start, true asked 14 s after it), once after each GoTo and each re-centre: at most
  four times a run, a little over 2 minutes in all at exposures of 30 s. A run started
  while Track
  has long been steady on the target waits for nothing. The lasting fix is 032's
  (§ Risks).

  **A push that has not settled reads not steady.** 038 sets `moving` at the first
  live pointing more than 5′ from the place held and clears it at the next one back
  within 5′; only 5 s of stillness make it `following`. A run that ended on the first
  `moving` would end on a knee against the tube, a gust or a hand on the focuser,
  with nothing pushed away. So `moving` reads `Tracking { steady: false }` until it
  has lasted `PUSH_GRACE`: the run starts no take and no focus check meanwhile, and
  while it stacks 040's tracker is itself not steady for as long as the push reads
  `moving`: the stack waits before its next exposure, `Steering::waiting()` is true and
  the run reads `paused_by_tracker`, and the sub in flight is dropped as `not_steady`
  (040 FR-021, FR-022). The instant
  is kept by the part and compared on tokio's clock, not counted in calls, because
  tests and the machine read `state` at different paces.

  The part names two of Track's states, `armed` and `off`, and no other: whatever a
  tracker calls its working states, it is tracking, and whether it is steady is the
  tracker's own answer. The reasons are the tracker's
  (`crates/finder/src/tracking.rs:61`) and a halted or failed GoTo's, which Track passes
  on when it ends armed (`crates/finder/src/session.rs:893`,
  `crates/finder/src/goto.rs:813`, `crates/finder/src/goto.rs:817`,
  `crates/finder/src/goto.rs:848`).

  The part's own Track off leaves `off` and `stopped` behind without moving the stop
  generation, which is why that pair reads `Slewing` while `repointing` is set. The
  machine never reads `state()` while `repoint` runs, so that row is a guard and not a
  path. Without the flag, a Track that ended `stopped` some other way with the
  generation unmoved (a start of Track that lost a race inside the session, or the
  console letting go of Track) would read `Slewing` for ever, and the run has no
  timeout for that.
- **`repoint`**, in this order. US4 writes steps 1, 2, 3a and 4 to 6; US5 adds step 3
  and `FRESH_POLLS`.
  1. The generation differs from the one kept: `Err(Stopped)`.
  2. `repointing` is set; `state.session().stop_tracking().await`
     (`crates/finder/src/session.rs:950`).
  3. (US5) It reads the `seq` of the session's pointing
     (`crates/finder/src/session.rs:463`), then, at most `FRESH_POLLS` times, sleeps
     `POLL` and reads it again. It goes on as soon as the `seq` has changed, and goes
     on all the same when it has not after the last sleep. A generation that has
     changed meanwhile clears `repointing` and is `Err(Stopped)`: that Stop has
     stopped the mount, and nothing has been started since step 2. Track stays off
     through this wait.
  3a. (US4) The session's target is none, or its id is not the one kept: `repointing`
     is cleared and the part answers `Ok(())` without asking Track. Track stays off,
     and `state()` answers `Ended(TargetChanged)` by its second row. The run never
     asks Track for a target it was not started for (spec FR-023, US4-S5, US5-S7).
  4. `let asked = api::start_motion(&state, by, tool, || api::start_tracking(&state)).await`,
     with `tool` `imaging_run` for `page` and `start_imaging_run` for `mcp`;
     `repointing` is cleared, whatever `asked` is.
  5. The generation differs now, whatever `asked` is: `api::stop(&state).await`
     (`crates/finder/src/api.rs:904`), then `Err(Stopped)`.
  6. `asked` is `Err(error)`: `Err(Mount(message))`, the error's `message`. Otherwise
     `Ok(())`.

  Steps 1, 3's check and 5 guard one window between them. A mutation of any one alone
  survives on the simulator, and so do steps 1 and 5 removed together and steps 1 and
  3's check removed together: step 3 checks the generation at each poll, and step 5
  stands in for both. US5-S5 catches step 3's check and step 5 removed together, and
  US4-S5 and US5-S5 catch all three.

  **Step 5 comes before `asked` is read.** A Stop that lands while step 4 runs makes
  Track itself answer an error: `Session::start_tracking` answers `stopped` when the
  session's own generation moved under it (`crates/finder/src/session.rs:854`), and a
  GoTo it was starting answers `the observer stopped it`
  (`crates/finder/src/goto.rs:626`). Read first, either would end the run
  `mount_failed` after the observer pressed Stop. Every real Stop advances GoTo's stop
  generation before it sends anything (`crates/finder/src/goto.rs:842`), so comparing
  it tells a Stop from a mount that would not start, and whatever step 4 did start is
  stopped again before the part answers. A Stop asked twice is always safe.

  **Step 3 is there** because a latest pointing is taken at the aim point in force when
  its star frame was solved (`crates/finder/src/session.rs:1192`), and while Track is
  on 032 holds a newly measured aim point back (032's FR-036; 032's owner decision 25,
  which 037's owner decision 26 cites). Right after
  step 2 the pointing the session holds was still taken at the old aim point, and
  Track's 1′ test (`crates/finder/src/session.rs:845`) would start no GoTo. The first
  live solve the session records with Track off applies the held aim point before its
  pointing is taken (032 plan.md § The session, the held aim point), so that pointing
  is the first at the new aim point, and Track asked after it brings the target there.
  The `seq` is read after step 2 for that reason: a pointing recorded while Track was
  still on was taken at the old aim point. The wait is a bound in seconds, 5 s at
  `POLL`, and not a count of the finder camera's frames. 015's tracker already expects
  a live pointing at least every five seconds (`crates/finder/src/tracking.rs:248`), so
  a finder camera slower than that is outside what Track is built for; after the bound
  the part asks Track anyway, Track decides with the pointing it has, the held aim
  point stays held under the new Track, and at worst a try is spent.

  **The tracker is handed no step.** While Track is on, live pointings stay taken at
  the aim point Track started from, so 015's tracker, which is handed every one as the
  target's offset (`crates/finder/src/session.rs:972`), sees no step toward a newly
  measured aim point; during step 3 Track is off.

  A re-centre therefore takes at most the 5 s of step 3, Track's own wait of up to 2 s
  for the axes to rest, and the acknowledgements of one Track off and one Track on:
  about 7 s. In that time Track is off, so another client's `goto_target` is not
  refused with `Track is on`; step 4's Track then finds the mount moving and the run
  ends `mount_failed` with Track's words (spec § Edge Cases).

  `stop_tracking` waits for the mount's acknowledgement of its stop, not for rest.
  `Session::start_tracking` itself waits up to 2 s for both axes to rest whenever it
  replaces a tracker, before it asks GoTo. If
  an axis still reports motion after that, when step 4's Track asks GoTo, GoTo refuses
  with `the mount is already moving` (`crates/finder/src/goto.rs:739`) and the run ends
  `mount_failed` (§ Risks).
- **`stop`**: `api::stop(&state).await` (`crates/finder/src/api.rs:904`), the console's
  Stop, its answer ignored. It moves the stop generation, so the next `state()`
  answers `Ended(Stopped)`.
- **`track_on`**: whether the session's snapshot has a `track` whose `state` is not
  `off`.
- **`steady_between(start, end)`**:
  `state.session().steered().is_some_and(|t| t.steady_between(start, end))` (036, 040).

### The console's camera and solver parts (`crates/finder/src/imaging/run/console.rs`, US4)

**Camera.**

```rust
/// The name a run holds the imaging camera's lease under.
pub const HOLDER: &str = "the imaging run";
```

It keeps one `Mutex<Option<Arc<Lease>>>`. The lease is 031's US9, which 033, 034 and
035 take as well: `Imaging::lease(holder)` answers a `Lease` or a 409 `Refusal` whose
message is `the imaging camera is held by {holder}`, naming whoever holds the camera,
and does not wait; while a lease is out, `Imaging::take` and `Imaging::set` by anyone
else are refused the same way; `Lease::take(purpose)`, which borrows the lease and is
awaited, is `Imaging::take` for the holder; `Imaging::abort`, `ended`, `view` and 032's
`solve` are open to anyone; and dropping the `Lease` gives the camera back. `HOLDER` and
the purpose `run` are the two words this spec gives 031's records and refusals (037's
owner approval 2).
- `ready`: the imaging view's `state` other than `ready` refuses with the words 031's
  `Imaging::set` uses for it (`the scope node is not connected`,
  `this scope node has no imaging service`, `this scope node has no imaging camera`,
  `the imaging camera is not answering: {error}`). It looks at nothing else. A console
  without imaging is never given this part.
- `hold`: with a lease already kept, `Ok(())`. Otherwise `imaging.lease(HOLDER)`; a
  `Refusal` is `Err(message)`, unchanged. Then, holding the lease, it reads the imaging
  view once, and in this order: a `frame` that is `exposing` or `receiving` drops the
  lease and answers `Err("an exposure is in progress")`, the words 033's `focus_start`
  and 035's `Stacker::start` use for it; a `settings.roi` that is not
  `{x: 0, y: 0, width: camera.width, height: camera.height}` drops the lease and answers
  `Err("the imaging camera has a region of interest set: clear it before an imaging run")`.
  Otherwise the lease is kept, as `Some(Arc::new(lease))`.
- `release`: takes the kept lease out of the lock and drops it.
- `take`: clones the kept `Arc` out of the lock and lets the lock go; without one,
  `Err(Camera("the imaging run does not hold the imaging camera"))`. Then
  `lease.take("run").await` on its own clone, a `Refusal` being
  `Err(Camera(message))`; a view with no `frame` is `Err(Camera(words))`, the words of
  `ready` for the view's state, or `the imaging camera answered no frame` when the
  state is `ready`: 031 builds that view after its lock is let go, so a link that
  drops in between answers `Ok` with no frame, and 035's stack ends on the same answer;
  it is counted like any failed take; then `imaging.ended(id)` for the id of the
  answered view's
  `frame`: `saved` is `Ok(FrameRef { id, file })`; `failed` whose `error` begins
  `could not save the imaging frame` is `Err(Save(error))`, any other `failed`
  `Err(Camera(error))`; `aborted` is `Err(Camera("the exposure was aborted"))`.
- `abort`: `imaging.abort().await`.

**The lease comes before the look for an imaging frame in flight.** 033's focus aid
takes one exposure after another and 035's stack one sub after another, so while either
holds the camera an imaging frame is almost always `exposing`. A part that looked for
that first would answer a start with `an exposure is in progress`; asked for the lease
first, it answers with the lease's words, which name the holder (US6-S7). The words
`an exposure is in progress` are left for a single imaging frame taken under no lease
(US4-S6).

**The run's own imaging frames are taken at the exposure setting, uncapped.** `take`
asks for one imaging frame at the camera's settings as they stand, and the part asks
nothing of `Imaging::steering()`. 036 caps the exposure of a stack's subs only: its
`Steering::before_exposure` decides the capped exposure for a stack's taker, inside
035's loop, and 036 sets it through the stack's own lease and puts it back as the stack
ends (036 plan.md § The steering). 036 offers no getter that answers the capped
exposure outside a stack, and adding one is an edit to 036, which this trio may not
make. So on an alt-az mount an exposure setting longer than the tracker's limit smears
the corners of the run's first imaging frame and of each imaging frame it solves, and,
once the target is centred, not their centre
(`docs/research/2026-11-15-imaging-camera.md` § 7. Field rotation, sub length and the
drive). The run reads only the solve's centre; a solve that fails for
the stars lost in the corners is counted as any other (037's § Failed steps and counts in
the machine). This is a known limit, which the README states and the field checklist's
step 8 has the observer look for.

The focus part and the stack part never touch the lease: 033's `focus_start` takes it
as `the focus aid` and 035's `Stacker::start` as `the stack`, each for as long as it
runs, which is why the machine gives the run's own lease up just before it calls them
(037's owner decision 15). The lock around the kept lease is a std `Mutex` and is never
held across an await: `take` awaits on its own clone of the `Arc` and drops it when it
answers or is itself dropped. The machine never calls `release` while a take of its own
is awaited, so the camera is given back at the moment `release` drops the kept clone.

**Solver.** `ready`: `imaging.solver()` that is `None` refuses with
`this console has no imaging solver`. `solve(frame, target)`:
1. It reads `before`: the `aim` of `state.session().snapshot(state.clock.now())`, 032's
   `Option<AimView>`.
2. `imaging.solve().await`. A `Refusal` is `Err(NotSolved(message))`. A view whose
   `solve.frame_id` is not `frame.id` is
   `Err(NotSolved("another imaging frame was taken before the solve"))`.
3. `imaging.solved(frame.id).await`. `failed` with the `reason` `no_database` is
   `Err(SolverUnavailable(error))`; with `failed`, `Err(Save(error))` when the error
   begins `could not save the imaging solve` and `Err(SolverUnavailable(error))`
   otherwise; with any other reason (`not_solved`, `too_few_stars`, `timed_out`)
   `Err(NotSolved(error))`.
4. `solved`: the offset is `crate::model::separation(solve.ra_deg, solve.dec_deg,
   target.ra_deg, target.dec_deg) * 3600.0`, the great-circle function Track's own 1′
   test uses (`crates/finder/src/model.rs:312`). The tolerance is
   `(CENTRE_SHARE * shorter * solve.arcsec_per_px).max(CENTRE_FLOOR_ARCSEC)`, `shorter`
   being the smaller of the `width` and `height` of the imaging view's `frame` when its
   `id` is still `frame.id`, and `CENTRE_FLOOR_ARCSEC` alone otherwise. `near_arcsec`
   is `CENTRE_NEAR_SHARE * shorter * solve.arcsec_per_px` with the same `shorter`, and
   the tolerance when the frame is no longer that one. `aim_moved` is
   true when `state.session().held_aim()` is `Some`: a measured aim point 032 holds
   back while Track is on, which the re-centre's Track off puts in force (§ The
   console's mount part, step 3). Otherwise it is true when the session's `aim`, read
   again now, is there where `before` was not, or has another `x` or another `y` than
   `before`.

`aim_moved` is read from the session and not from 032's `relation.last`, because an aim
reading 032 takes does not always move the aim point: 032 sets the session's aim point
only when none is in force, when the one in force was tapped, or when its window's
median lies more than `AIM_STEP_ARCSEC`, 10″ on the sky, from the one in force (032
plan.md § The relation, `should_apply`); and while Track is on it sets none at all but
holds it back (032's FR-036), which `held_aim` answers. 032 smooths later aim readings
against the held one as the aim point in force, so an aim point is held back only when
one of those three tests passed. When the aim point neither moved nor is held back,
Track off and on would bring the mount back to where it is, and a try would be spent on
nothing.

The centre of a whole-sensor imaging frame is the set point, at any binning, which is
why `hold` refuses a region of interest and the offset needs no pixel arithmetic. 032's
task hands the measured aim point to the session before it shows the solve as `solved`
(032 plan.md § Solving on the console, the task's steps 1 to 3). With Track off the
session sets it, so the second read sees it, and from then on every live pointing is
taken at it. With Track on, as throughout a run, 032 holds it back (032 plan.md § The
session, the held aim point): the second read of `aim` does not see it and `held_aim`
does, and the first live pointing after the re-centre's Track off is taken at it.

### Centring in the machine (`crates/finder/src/imaging/run.rs`, US5)

```rust
/// Re-centres a run may ask for.
pub const CENTRE_TRIES: u32 = 3;
/// Solves in a row, outside the tolerance, that may measure no new aim point.
pub const CENTRE_IDLE: u32 = 4;
```

Step 4 of the task becomes a loop. `tries` and `idle` start at 0. After each
`Ok(centre)`, `centring` is set to `{tries, offset_arcsec, tolerance_arcsec}`, and then
the first that holds:
- `offset_arcsec <= tolerance_arcsec`: go on to stacking;
- `centre.aim_moved` and `tries < CENTRE_TRIES`: `tries += 1`, `idle` is 0,
  `centring.tries` is set, the state is `centring` with its first message, whose `{n}`
  is `tries`; `mount.repoint().await`, not guarded, whose `Err` ends the run by the
  table of 037's plan.md § Failed steps and counts in the machine; then `frame()`, which waits until
  the mount tracks and is steady, and the solve again;
- `centre.aim_moved`, the three tries being spent: when
  `offset_arcsec <= near_arcsec` the run adds the first of the two warnings that 037's plan.md § The
  run's words gives a target stacked off the centre, in the form its offset gives, and
  goes on to stacking. Up to 036's `FAR_ARCSEC`, 180″, its steering takes out half of
  what is left at each gap, at most 30″ on the sky an axis and, against an axis's
  motion, at most that axis's rate times 0.5 s (036 FR-008); beyond 180″ it stands back,
  the view carries `steering.far_arcsec`, and the target stays where it is. Otherwise
  the run ends `NotCentred { offset_arcsec, measured: true }`;
- else, the solve having measured no new aim point: `idle += 1`. At `CENTRE_IDLE`, when
  `offset_arcsec <= near_arcsec` the run adds the second of those warnings, in the form
  its offset gives, and goes on to stacking, and otherwise it ends
  `NotCentred { offset_arcsec, measured: false }`: it
  is the case of the branch above, a target plainly in the frame that is not thrown away
  (owner decision 4). Below `CENTRE_IDLE` the state is `centring` with its second
  message, and `frame()` and the solve again, `tries` unchanged and no re-centre asked
  for.

The state stays `centring` throughout. A take or a solve that fails inside the loop is
counted as 037's US2 counts it. A solve that measured no new aim point is not a try: 032
gives none for four outcomes (`moving`, `no_live_pointing`, `off_frame`, `set_aside`),
and after the finder or the imaging camera is re-seated it sets its first two readings
aside, so counting them would spend the three tries before one re-centre was judged.
`CENTRE_IDLE` is what still ends the wait of a run that can measure nothing: it
stacks when the target is near the centre and ends `not_centred` when it is not.

A stop of the run that arrives while `repoint` is awaited is not lost: the signal stays
set, `repoint` answers within its bound, and the `steady` at the head of `frame()` ends
the run `RunStopped` before another take (US5-S3).

### The focus gate in the machine (`crates/finder/src/imaging/run.rs`, US6)

```rust
/// How long a focus pause may last.
pub const FOCUS_PAUSE_LIMIT: Duration = Duration::from_secs(900);
```

In step 3, `ClearlyOut`, `Unmeasurable`, `NoRecord` and a check that did not finish no
longer go on (the warning `The focus check did not finish: {words}.` stays). A check
that did not finish pauses with the message of `Unmeasurable` only when its failure is
`NotSolved`, the focus part's word for a check that found no stars to measure. With any
other failure the message is
`The focus check did not finish: {words}. Check the imaging camera, then check focus again or go on anyway.`:
a camera that failed under the check is not cloud or dew, and the observer is not sent
to look for either (US6-S10).

**An unsteady check is repeated once.** The run reads the console clock before it
calls `focus.check` and again when it answers. For a verdict of `ClearlyOut` or
`Unmeasurable`, and not for a check that did not finish, it asks
`mount.steady_between` over those two times. When the tracker
was not steady, and this check is not itself such a repeat, the check is made again at
once, `steady(None)` and `guarded(focus.check())`, with no pause and no second
`camera.release()`: trailed stars read
as out of focus, and the observer would be told to turn a focuser that is right. A
repeat is judged as any check, but for this: when it too would pause and was not
steady either, the run pauses with the warning
`The tracker was not steady during the focus check.` Every check the run makes after
a resume starts the rule again.

Then, for a check that pauses:
1. the state is `focus_pause` with the verdict's message; `pause_ends_at` is the
   console clock's time plus `FOCUS_PAUSE_LIMIT`; the run notes that it is in a
   pause, and takes resumes from this moment, the opening of the aid included (no test
   holds `open_aid`; the order of the two lines is the rule);
   `focus.open_aid().await`, an `Err` adding the warning of 037's plan.md § The run's words
   and changing nothing else;
2. it waits, with `tokio::select!`, `biased` in this order, for the first of: a stop; a
   resume; an `Ended` from
   `mount.state()`, read every `POLL`; `FOCUS_PAUSE_LIMIT` since the pause began. The
   stop, the mount's ending and the limit end the run, as `RunStopped`, as the mount's
   ending and as `FocusTimeout`;
   the ending closes the aid because the note is set, whether or not the aid opened:
   033's `focus_stop` is safe when nothing runs, and one rule is easier to test than
   two. Whichever of the four comes first answers the pause: from then on a resume is
   refused, though the state word is still `focus_pause` until the aid has closed.
   The pause is answered by whoever takes it first: the task, as soon as its wait
   answers, or a resume. A resume that took it before the task did is acted on, even
   when the limit had fired in the same instant; a stop and the mount's ending are
   still read at the next step;
3. a resume: `focus.close_aid().await`, on the same rule, the note and `pause_ends_at`
   cleared; every ending clears `pause_ends_at` too. With
   `anyway` false the state is `focus_check` again, then `steady(None)` and
   `guarded(focus.check())` as in step 3, without another `camera.release()`: the
   lease is still given up (US6-S11). With `anyway` true
   the warning `Focus was not checked again: the observer went on anyway.` is added and
   the run goes to step 4, taking the lease again first, as every way on from step 3
   does.

`Runner::resume(anyway)` now: no run on, 409 `no imaging run is running`; a run not in
`focus_pause`, or one whose pause has already been answered by a resume, a stop, the
mount's ending or the limit, 409 `the imaging run is not paused`; else it hands
`anyway` to the task and answers the view. The first resume is the one acted on: a
second tap, or the page and an MCP client together, cannot turn a check into a
go-anyway, and a `Go anyway` sent as the limit fires is not answered yes by a run that
then ends.

### The console's focus part (`crates/finder/src/imaging/run/console.rs`, US6)

`focus(state)`:
- `check`: `imaging.focus_start(FocusAsk { check: true, ..Default::default() }).await`
  (033). A `Refusal` whose message is `this console has no imaging solver` is
  `Err(SolverUnavailable(message))`; any other is `Err(Other(message))`. Then
  `imaging.focus_ended().await`: `None`, or a view without `aid`, is
  `Err(Other("the focus check left no reading"))`. With `aid.ended_reason` `done`:

  | `FocusCheck` | From 033's view |
  |---|---|
  | `verdict` | `aid.verdict`, word for word (`good`, `slightly_soft`, `clearly_out`, `unmeasurable`, `no_record`); `Unmeasurable` when it is absent |
  | `reading` | `aid.median.value` when present, else `aid.last.value` |
  | `record` | `aid.reference.value` |
  | `unit` | `Arcsec` when the view's `unit` is `arcsec`, else `Px` |

  With the `ended_reason` `stopped`: `Err(Stopped)`. The run's own stop and the mount's
  ending never get here, because `guarded` drops the check first, so this is a stop
  from 033's own route or tool, and the run ends `stopped`, as it does when the stack
  is stopped from its own route. 033's `focus_stop` answers only once no aid runs at
  all; a run that paused here and opened the aid would hold that request for the whole
  pause.
  With the `ended_reason` `no_region`, 033's word for an imaging frame none of whose
  tiles had stars enough to measure: `Err(NotSolved(words))`. With any other
  `ended_reason` (`failed`): `Err(Other(words))`. Either way the words are
  `the focus check ended: {ended_reason}`, followed by `: {error}` when the view has an
  `error`.
- `open_aid`: `imaging.focus_start(FocusAsk::default()).await`; a `Refusal` is
  `Err(Other(message))`.
- `close_aid`: `imaging.focus_stop().await`, which 033 makes safe when nothing runs and
  which also ends a check in flight.

The sizes are whatever 033's view shows, in its unit: the run compares nothing itself
and holds no threshold of 033's.

### Limits and the stack's endings in the machine (`crates/finder/src/imaging/run.rs`, US7)

```rust
/// Stacking time with a failed sub and none stacked that ends a run.
pub const NO_SUB_LIMIT: Duration = Duration::from_secs(600);
/// Reasons 035 drops a sub for that say the console fell behind, not that a sub failed.
pub const NOT_FAILURES: [&str; 2] = ["behind", "unprocessed"];
```

Step 6 of the task, every `POLL`, after the counts are copied and the mount's ending
and a stop are judged (037's plan.md § Failed steps and counts in the machine), and before the state
is shown, the first that holds:
1. `progress.failed` is `Some(f)`: the run ends by the row of `StackProgress::failed`
   in the table of 037's plan.md § Failed steps and counts in the machine;
2. `progress.running` is false: the run ends `StackStopped`;
3. `settings.subs` is `Some(n)` and `stacked >= n`: `CountLimit`;
4. the instant now is `settings.duration_s` or more after the instant `stack.start`
   answered `Ok`: `DurationLimit`. This is the clock's time, on tokio's clock: a
   stretch in which the stack waits for the tracker counts, because the limit is there
   to end a forgotten run and not to measure exposure (037's owner decision 8, US7-S2);
5. subs that keep failing: the task keeps `quiet`, the time on tokio's clock the run
   has stacked since `stacked` last rose, or since stacking began, and the `dropped`
   map as it stood then. Each poll whose `waiting_for_tracker` is false adds the time
   since the poll before; a poll whose `stacked` is above the poll before's sets
   `quiet` to zero and keeps the map again. When `quiet` is `NO_SUB_LIMIT` or more and
   the sum of `dropped` now over every reason not in `NOT_FAILURES`, less the kept
   map's sum over the same reasons, is 1 or more, the run ends `SubsFailing(words)`,
   the words being, for each such reason whose count rose, the reason with each `_`
   made a space, a space and the rise, in the map's order, joined by `, `.

Rule 5 is time and not a count of subs because a count punishes short subs: ten drops
are under a minute of cloud at 5 s subs. It leaves out `behind` and `unprocessed`
because 035 drops those when its queue is full or still holds subs at the end, which
says the console is slower than the camera and nothing about the sky. And it asks for
one failed sub because nothing in a run waits a fixed time for an exposure: a sub
longer than the limit, on a later mount, drops nothing and must not end the run.

A failed `stack.start` is 037's US2's, unchanged. Every one of these endings still calls
`stack.stop`, as 037's plan.md § The machine's Ending has it: the stack is saved whoever ended it.

Rule 2 is why the script's `set_stack` leaves `running` alone (037's plan.md § The script): 037's US2's,
037's US3's and 042's US10's tests set counts on a stacking run and must go on passing once this
rule is in the machine.

### The console's stack part and the wiring (`crates/finder/src/imaging/run/console.rs`, `crates/finder/src/api.rs`, `crates/finder/src/main.rs`, US8)

```rust
/// Bytes 035 writes for each pixel of a sub when a stack ends: twelve in `stack.fits`,
/// three planes of 32-bit reals, and at most three in `stack.png`, 8-bit RGB.
pub const RESERVE_PER_PIXEL: u64 = 15;
/// For what does not grow with the frame: the FITS header and padding, the PNG's row
/// bytes and framing, `stack.json` with its list of subs, and `run.json`.
pub const RESERVE_MARGIN: u64 = 16 * 1024 * 1024;
/// How much of the reserve is written between two yields.
pub const RESERVE_CHUNK: usize = 4 * 1024 * 1024;
/// `RESERVE_PER_PIXEL` times the pixels of one sub, its width and its height each
/// divided by the binning and rounded up, plus `RESERVE_MARGIN`. A binning of 0
/// counts as 1.
pub fn reserve_bytes(width: u32, height: u32, binning: u32) -> u64;
```

`stack(state)` keeps the id of the stack it started. With `stacker` being
`state.stack()` (035), which is there whenever imaging is:
- `start(target)`: `imaging.steering().set_set_point(None)` (036): the run says where on
  the sensor it wants the target, today the centre. 036 lets go a tracker a stack holds
  when the set point changes (036 FR-014); called here, before the stack's start, the
  call changes nothing a stack holds. Then
  `stacker.start(StartRequest::default(), Some(&run::label(&target.name))).await`. A
  `Refusal` whose message begins `cannot make the stack folder` is `Err(Save(message))`;
  any other is `Err(Other(message))`. The part reads the console clock before that
  call: `Ok(view)` whose `started_at` is before that reading is a stack somebody else
  started, which 035's `start` joins and answers `Ok` for;
  it is `Err(Other("another stack is already running"))`, and the part keeps no id, so
  nothing of that stack is stopped or removed. Otherwise `Ok(view)` keeps `view.id`,
  writes the reserve, and answers `format!("runs/{}", view.id)`.
  **The reserve** is a file `.reserve` in the stack's folder, of
  `reserve_bytes(camera.width, camera.height, settings.binning)` bytes, the three read
  from `imaging.view()` once the stack has started; a view with no `camera` gives
  `RESERVE_MARGIN` alone. That is the most 035 writes there when the stack ends (035
  plan.md § The stack on the console, "The folder and the files", and its § FITS;
  `crates/finder/src/imaging/stack/service.rs` on `main`): `stack.fits` is a cube of
  three planes of 32-bit reals, twelve bytes a pixel of a sub; `stack.png` is an 8-bit
  RGB picture of the stack's full size, three bytes a pixel at most, since a PNG's
  compressed data is never more than a few parts in ten thousand larger than the pixels
  it holds; and the margin is for the FITS header and padding, the PNG's row bytes and
  framing, `stack.json`, which 035 writes again whole with about 0.3 KB for each sub it
  lists, so that the margin holds a list of some fifty thousand, and the run's own
  `run.json`. At the worked camera's full size, 4944 × 3284 at binning 1, the reserve is
  260,318,656 bytes, about 260 MB; on the simulated camera's 1236 × 822 it is
  32,017,096. The file is written as zeros, not by `set_len`, which reserves nothing
  where files are sparse, and not in one call: `RESERVE_CHUNK` bytes at a time with
  synchronous `std::fs`, and `tokio::task::yield_now().await` between two chunks, so
  that the console's other tasks run while a file of that size is written; no file of
  this spec calls `spawn_blocking` (§ Plan traps 11). The stack is already taking subs
  meanwhile. A stop of the run, or an ending of the mount's motion, that arrives while
  the reserve is being written is acted on when `start` has answered, as with every call
  the machine awaits unguarded (037's plan.md § The machine); the ending then calls `stop`, which
  removes the file first. The console's Stop itself waits for nothing. When a chunk
  cannot be written the part removes the file, a missing one being no failure, stops the
  stack it has just started, with `stacker.stop().await`, keeps no id, and answers
  `Err(Save(format!("could not reserve room for the stack's save: {error}")))`.
- `progress`: from `stacker.view()`; `None` answers `StackProgress::default()`; a view
  whose `id` is not the one kept answers the counts last read, with `running` false:
  another client's stack has replaced the run's own, and its counts are not the run's.
  Otherwise `running` is `state == "stacking"`;
  `stacked` is the view's; `dropped` is the view's `dropped` object as
  `serde_json::to_value` gives it, keeping each key whose number is above 0, so a
  reason a later spec adds is carried without a change here; `waiting_for_tracker` is
  `imaging.steering().waiting()` (040), true while 040's stack waits before an exposure,
  for the tracker or for its exposure limit to rise; and `failed`, for an `ended` view, is
  `Some(Camera(error))` when `ended_reason` is `camera`, else `Some(Save(words))` when
  the view has a `save_error`, else `None`.
- `stop(keep_subs)`: first it removes `.reserve` from the folder of the stack it
  started, so that 035's save has that room, a missing file being no failure and any
  other failure to remove it ignored: if the room is not freed, 035's save says so in
  its `save_error`. Then,
  when `stacker.view()` is the stack the part started:
  `stacker.stop().await` while its state is `stacking`, else that view. When it is
  another stack, or none: `Ok(())`, and no sub is removed, because the part can no
  longer read whether its own stack was saved; a stack another client has started
  since is not stopped. A view with a `save_error` is
  `Err(Save(save_error))`, and no sub is removed. Otherwise, with `keep_subs` false,
  when the view's `stacked` is above 0 and its `ended_reason` is `stopped`: it
  reads `stack.json` in the folder of the stack it started, under `imaging.root()`, and
  for every entry
  of its `subs` array that has a `file`, a path inside the stack's folder
  (`subs/<frame id>.fits`, 035's owner decision 13), removes that file in the stack's
  folder and the file whose name ends `.solve.json` in place of `.fits`, a missing file
  being no failure; the first other
  failure, after every entry was tried, is
  `Err(Other(format!("could not remove {file}: {error}")))`; a `stack.json` that cannot
  be read or parsed is `Err(Other(format!("could not read stack.json: {error}")))`.
  With nothing stacked, or a stack that ended by itself, no sub is removed: 035
  writes no `stack.fits` without a stacked sub, so the subs would be all the run
  produced, and after a failure they are what shows why. The machine says so when
  nothing was stacked (037's plan.md § The machine, Ending).
  The removals are synchronous `std::fs`, as the record's writes are.

035's stack takes its own subs and the steering of 036 and 040 corrects the tracker from
them inside 035's loop (036 plan.md § What 035 must provide, and the stack's three calls);
the part adds nothing to either.

`with_imaging_run` and the binary's line are in § The console's own parts in the console.

### The console's own parts in the console (`crates/finder/src/api.rs`, `crates/finder/src/main.rs`, US8)

`AppState` holds 037's `imaging_runner` and `with_imaging_run_steps`.
US8 adds:

```rust
/// Give the console its own parts: Track, 031's imaging frames and lease, 032's solve,
/// 033's focus check and 035's stack (037). Call after `with_live`, `with_imaging` and
/// `with_imaging_solver`; without imaging it changes nothing.
pub fn with_imaging_run(self) -> Self;
```

which builds `Steps` from `run::console::{mount, camera, focus, solver, stack}` over a
clone of the state taken before the runner is kept, and calls `with_imaging_run_steps`.
The parts' clones hold no runner, so there is no cycle; they do hold everything else of
the state, the owner of Track included, for as long as the runner or its task holds
them. That is what `Runner`'s `Drop` is for (037's plan.md § The machine).
`crates/finder/src/main.rs` calls it once, after the lines 032 and 033 put after
`.with_live()` (`crates/finder/src/main.rs:114`), `.with_imaging_solver(…)` and
`.with_focal_length_mm(…)`, on both the `--simulate` and the real path.

## Shapes that existing tests pin

These are not changed by any story:
- **Track, GoTo and Stop.** `Session::start_tracking`, `Session::stop_tracking`,
  `GoTo`, `Tracker`, `api::stop`, `api::start_tracking` and the routes over them keep
  their signatures, words and behaviour; the run only calls them. Their tests are
  finder's `motion` binary, among them
  `crates/finder/tests/motion/tracking/routes.rs:136` and the Stop tests under
  `crates/finder/tests/motion/stop`.
- **032's solve and relation, 033's check and aid, 035's stack, the steering of 036 and 040.** The
  parts call them as their plans give them, and change none.
- **037's scripted tests, and this spec's own, for the later stories.** A test file 037
  or an earlier story of this spec created is an existing test, and no later story may
  edit it. `crates/finder/tests/console/imaging_run.rs` and the scripted modules after it
  build `Steps` by `script.steps()` and read `RunView` field by field. So no story adds a
  method to a part's trait or a field to `Steps`, changes `Runner::new`, `start`, `stop`,
  `resume` or `view`, or writes the script. Each story changes what the task does between
  two states, and what it replaces is asserted by nothing before it:
  - US5 adds the centring loop. The script's default offset of 0″ never enters it, and
    US4-S1 asserts its solved offset inside the tolerance, so its run enters none
    either; US4-S5's `repoint` on the part alone gains only a wait of at most 5 s.
  - US6 replaces the going-on of a verdict that pauses and of a check that did not
    finish. One earlier scenario has such a run, 037's US1-S2's bullet for a check that did
    not finish, and it reads only `focus` and `warnings`, which US6 leaves as they are;
    the refusals of `resume` outside a pause keep their words (037's US3-S4).
  - US7 adds the limits and the stack's own endings. The default request has no count
    and 7200 s, which no earlier scenario stacks for; no earlier scenario stacks for
    600 s with a dropped sub; and the script's stack runs until
    it is stopped or `end_stack` is called, whatever `set_stack` gave it (037's US2-S3,
    037's US3-S3). That last point is why `set_stack` does not set `running` (037's plan.md § The script).
- **Finder's test layout.** `crates/finder/tests/layout/main.rs` fails a file at the top
  level of `crates/finder/tests` (`crates/finder/tests/layout/main.rs:109`) and any file
  added to `crates/finder/tests/support` (`crates/finder/tests/layout/main.rs:127`).
- **The binary's start-up lines and flags.** Nothing new is printed and no flag is
  added.

## Verification (every story, before proposing)

- **The factory gates** (`ergane.yaml` at 49bb210):
  - `lint` = `cargo fmt --all --check`,
    `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`,
    `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features --locked`
    and `RUSTDOCFLAGS="-D warnings" cargo doc -p finder --no-deps`;
  - `test` = `cargo nextest run --workspace --all-targets --all-features --locked`,
    `cargo test --workspace --locked --doc`, and then, in `crates/finder/web`,
    `npm ci --no-audit --no-fund`, `npm run typecheck`, `npm run format:check`,
    `npm test`, `npm run build`, `git status --porcelain -- dist | (! grep .)` and
    `CI=1 npm run e2e`.

  Every story runs both, and the judge sees their results. nextest runs every test as
  its own process and ends a test after 5 minutes (`.config/nextest.toml`). The stories'
  development checks run one finder module with
  `cargo nextest run -p finder --locked --test <binary> <module>::`.
- **CI in the merge queue** runs every `ci.yml` job, including the finder-page
  build-freshness check and the build on Rust 1.96.

## Plan traps

Each has its mechanism and the check that keeps it from recurring. They bind every
story.

1. **Gate evidence or nothing.** The judge credits only gate results and the story's
   diff. *Check:* every Then clause names a committed test that the `test` gate runs.
2. **A named command is read literally.** *Check:* commands are copied from
   § Verification.
3. **One evidence budget.** *Check:* each story keeps its diff under 64 KiB, measured
   with `git diff <base> -- . ':(exclude)crates/finder/web/dist' | wc -c`.
4. **The sandbox has only the gates' tools.** *Check:* no task runs a tool outside them.
5. **A spec fixed mid-epic needs a relaunch.** *Check:* no builder edits
   `specs/037-imaging-run/`, `specs/041-imaging-run-on-the-rig/`,
   `specs/042-imaging-run-page-and-mcp/` or `.specify/`.
6. **An oracle copied from the code proves nothing.** *Check:* a scripted test compares
   with the words, reasons and call lists spec.md states, typed into the test; a rig
   test compares with the simulator's truth, the session and the answers of 031's and
   033's own routes; `run.json` and `stack.json` are parsed by the test, never through
   `Store`; and no test builds an expected message by calling `Ending::message`: 037's US12-S1
   calls it, and compares what it answers with a sentence typed into the test.
7. **Stop must win.** A re-centre turns Track off and on again. A Stop between the two
   would be followed by the run's own start, and a Stop during Track's start makes
   Track answer an error that is not the mount's. *Check:* the mount part's `repoint`
   reads the stop generation before, during its wait and after, compares it before it
   reads what Track answered, and asks for Stop again (§ The console's mount part);
   US4-S5 asks the part alone for a re-centre after a Stop, and US5-S5 presses Stop
   during centring and watches the simulator's mount for 10 s.
8. **The motion lock is not re-entrant.** `start_motion` holds a tokio mutex while the
   start runs. *Check:* `point` calls `api::start_tracking` and never `start_motion`;
   `repoint`, which runs in the run's task and not inside a route, is the only place
   the mount part takes it.
9. **The run's own Track off reads as `stopped`.** So does a Track that ended that way
   for another reason. *Check:* the mount part answers `Slewing` for `off` with
   `stopped` only while its own `repointing` flag is set, and `Ended(Stopped)`
   otherwise; the machine does not poll during `repoint`.
11. **Paused time and the blocking pool.** Tokio's paused clock runs on while a blocking
    task works. *Check:* `Store`'s writes and reads, and the stack part's removals and
    the writes of its reserve, are synchronous `std::fs`, the reserve written a chunk at
    a time with a yield between two (§ The console's stack part), and no file of this
    spec calls `spawn_blocking`.
16. **027's layout.** *Check:* each new test is a module file in its binary's folder
    with one alphabetical `mod` line, taking its folder from `crate::support::Scratch`,
    and holding only the setup its own tests use.
17. **Each worktree builds on its own.** *Check:* no story sets `CARGO_TARGET_DIR`.
18. **Mount-neutral code.** *Check:* `crates/finder/src/imaging/run.rs`, `script.rs` and
    `store.rs` import nothing of `crate::goto`, `crate::tracking`, `crate::session` but
    `Clock`, `crate::ephemeris`, `crate::model` or `scope_link`, and of 035 `run.rs`
    calls only `crate::imaging::stack::service::label`, and of 036 it reads only the
    constant `crate::imaging::steering::FAR_ARCSEC`; `console.rs` imports
    `crate::model::separation` and no other item of those, reaches Track and Stop
    through `crate::api` and `Session::stop_tracking` only, reads the session through
    `Session::snapshot`, `Session::steered` and `Session::held_aim`, and GoTo's stop
    generation through `AppState::goto` and `GoTo::stops`, and names no state of Track
    but `armed` and `off`. In the four files no identifier (a constant, a field, a
    variable, a
    function or a type), no import and no arithmetic is about an axis, an altitude, an
    azimuth, a rate or axis counts, and no number is a focal length, a field size or a
    scale. The rule is about code: US8-S9 reads the four files for what they
    import, and a reviewer checks the identifiers, not words. What an observer reads and
    what a doc comment says are exempt:
    `Ending::message` writes `Ended: {name} is below the minimum altitude.`, the module
    doc of `console.rs` says that nothing there knows an axis, and `SubCounts` counts
    subs.
19. **A cycle of `Arc`s, and a state kept alive.** The runner holds the parts, the parts
    hold a state, and the state holds the runner; and a task that held the parts for
    ever would hold the owner of Track for ever. *Check:* `with_imaging_run` clones the
    state for the parts before the runner is kept, and `Imaging` holds the runner
    weakly; the run's task holds the runner's inner `Arc` and never the `Runner`, and
    `Runner`'s `Drop` aborts the task (037's US2-S9).
20. **The lease must come back, and must not be kept across a hand-over.** A run that
    ends by any path, a refused `point` included, would otherwise lock the imaging
    camera until the console restarts; a run that still held the lease when it called
    033's check or 035's stack would refuse its own step. *Check:* `camera.release()`
    is the last call of every ending and of a refused `point`, and comes before the
    first `focus.check` and before `stack.start`; `camera.hold()` comes before the first
    take after the focus gate. 037's US1-S1, 037's US1-S2, 037's US2-S7, 037's US2-S8, 037's US2-S10 and US6-S11
    assert it on the script, and US4-S2, US4-S4, US4-S6 and US8-S8 through 031's own
    route.
21. **Nothing waits a fixed time for an exposure.** *Check:* the only durations in the
    four files are `POLL`, `FOCUS_PAUSE_LIMIT`, the request's `duration_s`,
    `RETRY_WAIT` between two failed takes, `NO_SUB_LIMIT` of stacking with a failed sub
    and none stacked, `RECORD_EVERY` between two refreshes of the record and between two
    listed
    changes of `stacking` and `paused_by_tracker`, and, in
    `console.rs`, `PUSH_GRACE` and `relation::FIT_REACH_S`, 032's own constant, in the
    mount part's `steady`, where the exposure setting takes its place when it is
    longer. None is a wait for an exposure to finish, and `NO_SUB_LIMIT` ends nothing
    without a failed sub.
    `FRESH_POLLS` times `POLL` is 5 s: a bound on the finder camera's next live
    pointing, said as such, and on nothing of the imaging camera. Every other bound is
    a count of imaging frames, solves or tries.
22. **A code span that ends in a colon and a number is read as a citation.** The
    validator resolves it as `path:line`. *Check:* times, ports and JSON fragments in
    this trio never end a code span that way.
23. **The runner's lock and the imaging view.** `Imaging::view` calls `Runner::view`,
    which takes the runner's std `Mutex`; the console's camera, solver and stack parts
    call `Imaging::view`. A part called while that lock is held waits on itself. The
    scripted tests never show it, because the script reads no view; the rig tests of
    US4, US5, US6 and US8 show it only as a test nextest ends after 5 minutes. *Check:*
    while the runner's lock is held no part is called, nothing that reads the imaging
    view is called and nothing is awaited: a step copies what it needs out of the lock,
    lets it go, calls the part, and locks again to record the answer (037's plan.md § The machine).
24. **A later rule must not break an earlier story's test.** A story may not edit a test
    file an earlier story created. *Check:* the script's `set_stack` leaves `running`
    and `failed` alone and only `end_stack` ends a stack (037's plan.md § The script); US4-S1 asserts
    its solved offset inside the tolerance, so the loop of US5 does not enter it; each
    story's `preserved_behavior` in § Refinement Record names what of the earlier
    stories it leaves passing; and a story that finds an earlier test failing
    escalates.
25. **A part awaited by the test's own task is not polled while the test awaits a
    request.** A request that needs the session's Track gate then waits on a future
    nobody polls, and nextest ends the test after 5 minutes. *Check:* US4-S5 awaits
    each part call to its answer before it sends the next request, or spawns it.
26. **The target is read once.** `point` takes no target, Track reads the session
    itself, and a start of Track can take 2 s; a target edit is not under the motion
    lock. A part that read the session again after Track answered would keep the new
    target as the run's own, and `target_changed` would never fire. *Check:* the
    mount part keeps the id `target` answered, compares it before it asks Track in
    `point` and in `repoint`'s step 3a, and its `state` compares it with the
    session's target and with `track.target.id` (§ The console's mount part).
27. **A step the script holds must be released once.** `release` ends a hold; a test
    that needs one call of a step to wait and the next to go uses the mount's state
    or a queued answer, not a second `hold`. *Check:* US5-S3 makes its second solve
    wait by setting the mount not steady.

## Risks

| Risk | Mitigation |
|---|---|
| 033, 035 or 036 lands offering something other than its draft. | The run touches them in three small parts and nowhere else; the states, routes, page and tools do not move. flip.md § Prerequisites 2 re-anchors, and a gap goes to the owner. |
| 031 lands the lease under other names, words or rules than its US9 gives. | flip.md § Prerequisites 3 lists the five points. Only the camera part's `hold`, `release` and `take` touch it, and seven scenarios, two of which state its refusal word for word. |
| The simulator leaves the target just outside 90″, with the true aim point set or after a re-centre, or needs more than three tries. | US4-S1 and US5-S4 assert the bounds against the truth, and owner decisions 2 and 3 are where the numbers change; flip.md § Prerequisites 5 has what probe round 2 measured. |
| A story's diff passes 64 KiB. | Probe round 2 built the five stories: US4 41,483 B (63.3 %), US5 31,921 B (48.7 %), US6 41,188 B (62.8 %), US7 14,797 B (22.6 %) and US8 31,488 B (48.0 %) as written, against estimates of 40.9, 29.1, 37.4, 14.8 and 28.2 KB from round 1. Factory builds of 036, 039 and 040 ran 0.85 to 1.17 times their probes' diffs, so each story is estimated at 1.17 times its build with the cases the round's fold added, as tests: US4 (41,483 + 4,715) × 1.17 = 54,052 B (82 %), US5 (31,921 + 5,195) × 1.17 = 43,426 B (66 %), US6 (41,188 + 5,708) × 1.17 = 54,868 B (84 %), US7 (14,797 + 2,074) × 1.17 = 19,739 B (30 %) and US8 (31,488 + 406) × 1.17 = 37,316 B (57 %). Each is under owner choice 14's 60 KB, so no story is cut. US4 and US6 are the stories to watch: the scenario of the guards (US4-S4) or US4-S5's cases on the parts alone, and US6-S7's rig cases with their rig, are where a cut by tests would fall. No story drops or shortens a case to get under. |
| A Stop that lands while a re-centre's Track is being asked is covered by step 5 of `repoint` alone, which no test can reach by itself on the simulator. | Steps 1 and 3 cover the windows either side, and US5-S5 catches step 3's check and step 5 removed together. The field checklist's steps 11 and 14 press Stop during a run. |
| 032's aim reading fits the finder camera's solves from 10 s, or the exposure when that is longer, before the imaging exposure, so an imaging frame taken sooner than that after a GoTo or a re-centre reads `moving` and gives no aim reading. | The mount part reads steady only when 040's tracker was steady for that whole look-back (`steady_between`, spec FR-019). The cost is the larger of 10 s and the exposure, after the 2 s to 3 s for which 040's tracker settles, after each GoTo and each re-centre. US4-S1's bullet and US5-S6, at 20 s, assert it. The lasting fix is a small follow-up to 032, which has landed: drop from the fit every finder solve older than the session's last end of a GoTo or start of Track. When it lands the look-back shrinks to `steady()`. |
| A solve measures no new aim point, again and again: the finder camera's exposure is longer than the imaging camera's, so no finder solve falls inside the imaging frame, or 032 sets readings aside after a camera was re-seated. | Such a solve is not a try; after four in a row the run stacks a target that is near the centre, with a warning, and ends `not_centred` with their own sentence beyond that (spec FR-024). The README and step 4 of the field checklist say to set the imaging exposure at least as long as the finder camera's. The real finder's cadence is a field check. |
| A push that reads `moving` for less than 10 s does not end the run, and while the run stacks nothing of this spec pauses the stack for it. | 040's tracker reads not steady while the push is `moving` (040's US6-S6), so the stack waits and the sub in flight is dropped as `not_steady`. US4-S4's bullet shows only that the run goes on. |
| The reserve is worked out from the files 035 writes at a stack's end as they are today: 15 bytes a pixel of a sub and a margin of 16 MiB, about 260 MB at the worked camera's full size, where a `stack.fits` alone is about 195 MB (`reserve_bytes`). | If 035 comes to write more at a stack's end, a fourth plane or a 16-bit picture, the figure for a pixel is one constant, and the re-anchoring reads 035's save again (flip.md § Prerequisites 2). A `stack.json` that lists more than about fifty thousand subs outgrows the margin. The reserve helps only an ending the run makes: a stack that ends by itself saves while the reserve is still there. It is itself room the subs cannot use, and a disk that cannot hold it ends the run `save_failed` before its first sub. While it is written, 4 MiB at a time with a yield between, a stop of the run waits for `stack.start` to answer, for as long as the disk needs (US8-S7). |
| A target changed in the instant after a re-centre has asked Track again is not seen by step 3a. | Track then goes on with the old target, the next `state()` reads `target_changed` by its second row, and Track is left on the old target, as outside a re-centre. US5-S7's timing rests on the first live pointing with Track off coming within two polls (0.50 s) of it on the simulator. A target set between step 3a and Track's own read of it, two lock waits later, starts Track on the new target: `state()` reads `target_changed` by its second row and Track is left on the new target. No test reaches that window. |
| With `keep_subs` false, the two conditions under which the stack part removes subs are asserted on the script only, by the warning. | No rig scenario stops a run before its first sub is stacked at a moment a test can fix. |
| The observer's exposure setting is longer than the tracker's limit, and the run's own imaging frames, which 036 does not cap, trail their corner stars. | A known limit (§ The console's camera and solver parts). The solve's centre, all the run reads, is not moved; a solve that fails for want of stars is counted (037's US2-S2). The README says so and step 8 of the field checklist has the observer look. The other answer is a getter in 036 for the capped exposure, an edit to 036. |
| A re-centre asks Track again while the mount still reports motion after Track off; GoTo refuses with `the mount is already moving` and the run ends `mount_failed`. | `Session::start_tracking` waits up to 2 s for both axes to rest whenever it replaces a tracker, before it asks GoTo. The field checklist's step 8 watches the first centring. `stop_tracking` itself waits for the mount's acknowledgement of the stop, not for rest (`crates/finder/src/tracking.rs:219`). If the telescope shows it all the same, `repoint` waits, for a counted number of polls, until the session's mount view no longer says the mount is moving (`crates/finder/src/session.rs:508`) before it asks Track; that reads no axis. |
| The kept subs fill the disk. | The default time limit (037's owner decision 8), the size shown on the start form, `save_failed`, and the reserve the stack part frees before the stack's save. |
| Subs that are not kept still fill the disk while the run stacks. | Owner decision 22 names the other answer, an edit to 035. |
| A second client stops or restarts the stack under a run. | The lease refuses a second stack; a stop of the run's own stack from 035's route ends the run `stopped` (US7-S4 on the script, US8-S4 on the rig). A stack another client starts in the instant of the hand-over, which 035's `start` joins and answers `Ok` for, is told from the run's own by its `started_at` and ends the run `stack_failed`, with nothing of that stack stopped or removed. A stack started in the one poll between the run's stack ending and the run's own ending is left running; the run keeps its last counts and its subs (§ The console's stack part). Neither race is run by a test. |
| On the simulator no run meets 040's wait (`paused_by_tracker` on the console's own stack part), a stack's `camera` ending or a dropped sub, and no test can see the set point; US8-S6 ends `link_lost`. | Those rows are asserted on the script only (037's US2-S3, 037's US2-S4, US7-S4). A `save_error` is reached on the rig by a stack folder made read-only while stacking, at a stop of the run and at a stop from 035's route (probe round 2's throwaways); US8 can carry both. |
| The real GoTo and Track are unproven. | flip.md § Prerequisites 8. The checklist of 042's FR-041 starts with an imaging frame and a solve by hand before the first run. |

## Refinement Record

The re-anchoring (flip.md § Prerequisites 2) is still to do. Until then the record below
names what the drafts named: its tests are the scenarios, its writes are the files
§ Project Structure names, and its shared writes name the merge edge that orders two
stories of this spec on one file, or the story of another spec that last writes the file
among the drafts as they stood on 2026-11-15, read from each draft's own record, a story
of 036 that went to 040 when 036 was split being named in 040. Each of those names is
checked again at the re-anchoring. A test file an earlier story creates does not exist
yet and cannot be listed as an existing test; what a later story must leave passing in it
is under that story's `preserved_behavior`. The record's `open_questions` are empty
because the validator refuses a record that holds any; what is not established is in
§ Risks and flip.md, and what the owner must settle is in flip.md. Nothing in this record
is an owner approval: those are in flip.md, and flipping to `ready` gives them.

```yaml
version: 1
reviewed_revision: 49bb210fe4860dd98ba24727074e7bd3cc0d7124
stories:
 US4:
  tests:
  - path: crates/finder/tests/motion/imaging_run.rs
    status: new
    disposition: add
    assertion: "on the simulator with the true aim point set a run reaches stacking with no try, the tolerance exactly 90.0, its solved offset inside the tolerance and within 30 arcseconds of the true offset, the true offset at most 110 arcseconds, the run reading slewing at every sample at which Track read armed and nothing later than tracking in the 11.5 s after Track first read tracking, its imaging frame's purpose run, Track never leaving tracking, and the motion record reading page and imaging_run"
    decision: "spec.md:US4-S1"
  - path: crates/finder/tests/motion/imaging_run.rs
    status: new
    disposition: add
    assertion: "a start is refused with no target is selected, with Track's own words for a target below the horizon, with the region-of-interest words and with the scope node is not connected while the node is unresponsive, each leaving Track absent or not wanted, no run, no folder and the lease given back"
    decision: "spec.md:US4-S2"
  - path: crates/finder/tests/motion/imaging_run.rs
    status: new
    disposition: add
    assertion: "POST /api/stop while stacking ends the run as stopped with the stack stopped once and the simulated mount at rest for 10 s"
    decision: "spec.md:US4-S3"
  - path: crates/finder/tests/motion/imaging_run.rs
    status: new
    disposition: add
    assertion: "a raised minimum altitude ends the run too_low, an unresponsive node link_lost, a changed target target_changed with Track still on the old target, a push by hand pushed_away with Track following it, a push that keeps moving pushed_away 10 s to 15 s after its first push, dawn sun_up, an imaging frame asked for while the run holds the lease is refused with the imaging camera is held by the imaging run and accepted after the run, a tracker that is not steady near the transit still holds the run in tracking with its message 30 s after it first read tracking, a push of 6 arcminutes undone 3 s later is read as moving and does not end the run, and a DELETE while the run still slews ends it stopped with the sentence that the mount was stopped, track_on false, Track off and the simulated mount at rest for 10 s"
    decision: "spec.md:US4-S4"
  - path: crates/finder/tests/motion/imaging_run.rs
    status: new
    disposition: add
    assertion: "the mount part alone points, reads tracking and steady, points again leaving the motion record as page and imaging_run, and after POST /api/stop reads ended stopped and refuses to point again with Track off and the simulated mount at rest for 10 s from 2 s after the stop; and on a rig of its own, after the target is changed, a re-centre answers Ok with Track left off for 10 s and the part reading target_changed; and on rigs of their own, point refuses with the target was changed once the session names another target than target answered, Track turned to another target reads target_changed whatever the session names, Track turned on again after a Stop still reads stopped, a re-centre Track refuses fails as the mount in Track's own words, and one frame taken and solved by the console's camera and solver parts gives near_arcsec 449 arcseconds within 1"
    decision: "spec.md:US4-S5"
  - path: crates/finder/tests/motion/imaging_run.rs
    status: new
    disposition: add
    assertion: "a start while a single imaging frame is in flight is refused with an exposure is in progress and leaves no Track wanted and no run, and 031's own route takes a frame once that one is saved: the lease was given back"
    decision: "spec.md:US4-S6"
  - path: crates/finder/tests/motion/tracking/routes.rs
    status: existing
    disposition: preserve
    assertion: "Track's route, its refusals, its hand-over from GoTo and its drop behaviour are unchanged"
    decision: "plan.md:Shapes that existing tests pin"
  - path: crates/finder/tests/motion/stop/retry.rs
    status: existing
    disposition: preserve
    assertion: "Stop still retries and acknowledges as before"
    decision: "plan.md:Shapes that existing tests pin"
  - path: crates/finder/tests/layout/main.rs
    status: existing
    disposition: preserve
    assertion: "no file at the top level of crates/finder/tests, no new file in its support folder, and each main.rs only its support include and mod lines"
    decision: "plan.md:Shapes that existing tests pin"
  preserved_behavior:
  - "Session::start_tracking, Session::stop_tracking, GoTo, Tracker and api::stop keep their signatures, words and behaviour (plan.md Shapes that existing tests pin)"
  - "the run sends no axis command and reads no axis position (spec.md 037's FR-001)"
  - "no trait of a part gains a method and Steps gains no field"
  - "the lease is 031's US9: the camera part takes it, gives it back and adds no lock (plan.md Plan traps 20)"
  - "the run's own imaging frames are taken at the camera's settings as they stand: the camera part sets nothing and asks nothing of 036's steering (plan.md The console's camera and solver parts)"
  - "every scenario of 037's US1 to US3 still holds: the machine changes by one mod line"
  checks:
   development:
   - "cargo nextest run -p finder --locked --test motion imaging_run::"
   - "cargo nextest run -p finder --locked --test console imaging_run::"
   - "cargo nextest run -p finder --locked --test layout"
   handoff: declared_gates
   independent: declared_pipeline
   release:
   - "release owner: the repository owner, through the merge queue's ci.yml jobs, and a first run at the telescope after GoTo and Track are proven there"
  writes:
  - crates/finder/src/imaging/run.rs
  - crates/finder/src/imaging/run/console.rs
  - crates/finder/tests/motion/imaging_run.rs
  - crates/finder/tests/motion/main.rs
  no_write_reason: ""
  shared_writes:
  - path: crates/finder/src/imaging/run.rs
    other_spec: 037-imaging-run
    other_story: US3
    disposition: merge_order
    reason: "041 depends_on_landed 037-imaging-run, whose 037's US3 last writes this file; re-read at the re-anchoring"
  - path: crates/finder/src/imaging/run.rs
    other_spec: 041-imaging-run-on-the-rig
    other_story: US5
    disposition: merge_order
    reason: "US5 depends_on_merged US4, and adds the centring loop"
  - path: crates/finder/src/imaging/run/console.rs
    other_spec: 041-imaging-run-on-the-rig
    other_story: US5
    disposition: merge_order
    reason: "US5 depends_on_merged US4, and adds FRESH_POLLS and the wait for a live pointing after Track off to repoint"
  - path: crates/finder/tests/motion/main.rs
    other_spec: 040-stack-waits-for-steady
    other_story: US7
    disposition: merge_order
    reason: "041 depends_on_landed 040-stack-waits-for-steady, whose US7 last writes this file in the drafts; re-read at the re-anchoring"
  - path: crates/finder/tests/motion/main.rs
    other_spec: 041-imaging-run-on-the-rig
    other_story: US5
    disposition: merge_order
    reason: "US5 depends_on_merged US4, and adds mod imaging_run_centring"
  open_questions: []
 US5:
  tests:
  - path: crates/finder/tests/console/imaging_run_centring.rs
    status: new
    disposition: add
    assertion: "an offset outside the tolerance shows centring with its message and try, repoints once, takes and solves again, and goes on to stacking when the next offset is inside"
    decision: "spec.md:US5-S1"
  - path: crates/finder/tests/console/imaging_run_centring.rs
    status: new
    disposition: add
    assertion: "four solves 600 arcseconds off, beyond the near bound of 450, repoint three times and end not_centred with the offset written as 10.0 arcminutes and the words to check the aim point, with no stack started"
    decision: "spec.md:US5-S2"
  - path: crates/finder/tests/console/imaging_run_centring.rs
    status: new
    disposition: add
    assertion: "a solve that measured no new aim point is retried without a repoint and is not a try, its message saying so while the run waits for the tracker; four such solves in a row stack a target 400 arcseconds off with their warning and end not_centred 600 arcseconds off with their sentence, with no try and no repoint; a re-centre starts that count again, so that three more such solves and a 50 arcsecond one stack with no warning; after a try, a solve that measured no new aim point reads the second centring message; a target 400 arcseconds off after three tries, inside the near bound, is stacked with its warning; a target exactly at the tolerance is centred with no try, and one exactly at the near bound after three tries is stacked with its warning; at 400 arcseconds both warnings say it is stacked where it is, and at 180 and 150, not over 036's 180, that the stack's steering will bring it in; a failed repoint ends mount_failed or stopped by its failure, no take follows a repoint until the mount is steady, and a stop of the run during a repoint is answered only after it and ends the run stopped with no further take"
    decision: "spec.md:US5-S3"
  - path: crates/finder/tests/motion/imaging_run_centring.rs
    status: new
    disposition: add
    assertion: "on the simulator with no aim point a run reaches stacking after at least one try with the tolerance exactly 90.0, the true offset within the tolerance plus 20 arcseconds, the session's aim point measured within 2.0 pixels of the true one, and Track read as no longer off within 2 s of the first sample that read it off while centring; and with a finder frame every 2 s a run still centres with one try"
    decision: "spec.md:US5-S4"
  - path: crates/finder/tests/motion/imaging_run_centring.rs
    status: new
    disposition: add
    assertion: "POST /api/stop while centring ends the run as stopped and not mount_failed, with no stack started and the simulated mount at rest for 10 s"
    decision: "spec.md:US5-S5"
  - path: crates/finder/tests/motion/imaging_run_centring.rs
    status: new
    disposition: add
    assertion: "on the simulator with no aim point and an exposure of 20 s a run reaches stacking after at least one try with the true offset within the tolerance plus 20 arcseconds, and 032's last aim reading never reads moving on the way"
    decision: "spec.md:US5-S6"
  - path: crates/finder/tests/motion/imaging_run_centring.rs
    status: new
    disposition: add
    assertion: "a target changed as soon as the run reads centring ends it target_changed with track_on false, Track off 10 s later and the simulated mount at rest for 10 s"
    decision: "spec.md:US5-S7"
  - path: crates/finder/tests/motion/imaging_run_centring.rs
    status: new
    disposition: add
    assertion: "the mount part alone, on the run rig with the true aim point set, with no live pointing after its own Track off, answers a re-centre Ok between 4.9 s and 5.5 s after it was asked"
    decision: "spec.md:US5-S8"
  - path: crates/finder/tests/motion/tracking/routes.rs
    status: existing
    disposition: preserve
    assertion: "Track's route, its refusals, its hand-over from GoTo and its drop behaviour are unchanged"
    decision: "plan.md:Shapes that existing tests pin"
  - path: crates/finder/tests/motion/stop/retry.rs
    status: existing
    disposition: preserve
    assertion: "Stop still retries and acknowledges as before"
    decision: "plan.md:Shapes that existing tests pin"
  - path: crates/finder/tests/layout/main.rs
    status: existing
    disposition: preserve
    assertion: "no file at the top level of crates/finder/tests, no new file in its support folder, and each main.rs only its support include and mod lines"
    decision: "plan.md:Shapes that existing tests pin"
  preserved_behavior:
  - "every scenario of 037's US1 to US3 still holds: the script's default offset of 0 takes no centring branch (plan.md Shapes that existing tests pin)"
  - "every scenario of US4 still holds: US4-S1's solved offset is inside the tolerance, so its run enters no loop, and repoint on the mount part alone gains only a wait of at most 5 s (plan.md Plan traps 24)"
  - "Session::start_tracking, Session::stop_tracking, GoTo, Tracker and api::stop keep their signatures, words and behaviour"
  - "no trait of a part gains a method and Steps gains no field"
  checks:
   development:
   - "cargo nextest run -p finder --locked --test console imaging_run_centring::"
   - "cargo nextest run -p finder --locked --test motion imaging_run_centring::"
   - "cargo nextest run -p finder --locked --test motion imaging_run::"
   - "cargo nextest run -p finder --locked --test console imaging_run::"
   - "cargo nextest run -p finder --locked --test layout"
   handoff: declared_gates
   independent: declared_pipeline
   release:
   - "release owner: the repository owner, through the merge queue's ci.yml jobs, and the first centring at the telescope, watched for the tube staying still after a newly measured aim point until the re-centre's Track off (032's owner decision 25, which spec.md 037's owner decision 26 cites)"
  writes:
  - crates/finder/src/imaging/run.rs
  - crates/finder/src/imaging/run/console.rs
  - crates/finder/tests/console/imaging_run_centring.rs
  - crates/finder/tests/console/main.rs
  - crates/finder/tests/motion/imaging_run_centring.rs
  - crates/finder/tests/motion/main.rs
  no_write_reason: ""
  shared_writes:
  - path: crates/finder/src/imaging/run.rs
    other_spec: 041-imaging-run-on-the-rig
    other_story: US6
    disposition: merge_order
    reason: "US6 depends_on_merged US5, and replaces the going-on of a verdict that pauses with the focus pause and makes resume work"
  - path: crates/finder/src/imaging/run/console.rs
    other_spec: 041-imaging-run-on-the-rig
    other_story: US6
    disposition: merge_order
    reason: "US6 depends_on_merged US5, and adds the focus part"
  - path: crates/finder/tests/console/main.rs
    other_spec: 037-imaging-run
    other_story: US3
    disposition: merge_order
    reason: "041 depends_on_landed 037-imaging-run, whose 037's US3 last writes this file; re-read at the re-anchoring"
  - path: crates/finder/tests/console/main.rs
    other_spec: 041-imaging-run-on-the-rig
    other_story: US6
    disposition: merge_order
    reason: "US6 depends_on_merged US5, and adds mod imaging_run_focus"
  - path: crates/finder/tests/motion/main.rs
    other_spec: 041-imaging-run-on-the-rig
    other_story: US8
    disposition: merge_order
    reason: "US8 depends_on_merged US5, and adds mod imaging_run_stack"
  open_questions: []
 US6:
  tests:
  - path: crates/finder/tests/console/imaging_run_focus.rs
    status: new
    disposition: add
    assertion: "a clearly out verdict pauses the run with its message, focus note, pause_ends_at 900 s after the pause began and the focus aid opened and no solve"
    decision: "spec.md:US6-S1"
  - path: crates/finder/tests/console/imaging_run_focus.rs
    status: new
    disposition: add
    assertion: "resume(false) closes the aid and checks focus again, taking the lease back and going on to the solve when it is good, with pause_ends_at gone"
    decision: "spec.md:US6-S2"
  - path: crates/finder/tests/console/imaging_run_focus.rs
    status: new
    disposition: add
    assertion: "resume(true) closes the aid, takes the lease back and goes on to the solve with no second check and the go-anyway warning, with pause_ends_at gone"
    decision: "spec.md:US6-S3"
  - path: crates/finder/tests/console/imaging_run_focus.rs
    status: new
    disposition: add
    assertion: "a pause is still on 899.5 s after the start and has ended focus_timeout 1 s later with the aid closed and no stack started"
    decision: "spec.md:US6-S4"
  - path: crates/finder/tests/console/imaging_run_focus.rs
    status: new
    disposition: add
    assertion: "a stop and a mount ending from a pause close the aid before the lease is given back and leave no pause_ends_at, an aid that cannot open leaves the run paused with a warning, and resume outside a pause is refused"
    decision: "spec.md:US6-S5"
  - path: crates/finder/tests/console/imaging_run_focus.rs
    status: new
    disposition: add
    assertion: "POST /api/imaging/run/continue with anyway false or true takes a paused run on"
    decision: "spec.md:US6-S6"
  - path: crates/finder/tests/console/imaging_run_focus.rs
    status: new
    disposition: add
    assertion: "on the simulator the console's own focus part passes a run at the focus error 0.0 with its reading in arcsec, pauses one that 033's own check calls clearly out with the focus aid running, goes on after refocusing, refuses a start with the imaging camera is held by the focus aid while the aid already runs, with no mount.point after the refused start, pauses with no_record on a fresh folder, and ends stopped with the sentence that the focus check was stopped, no solve and 033's aid ended when the run's own check is stopped from 033's route, and ends solver_unavailable at its focus check on a rig with no imaging solver"
    decision: "spec.md:US6-S7"
  - path: crates/finder/tests/console/imaging_run_focus.rs
    status: new
    disposition: add
    assertion: "a clearly out check during which the tracker was not steady is repeated once with no pause and no warning, a second one that was not steady either pauses with the warning that the tracker was not steady, and a good check is not repeated; a no-record check is not repeated either, every check after a resume starts the rule again, and a repeat gives the lease up no second time"
    decision: "spec.md:US6-S8"
  - path: crates/finder/tests/console/imaging_run_focus.rs
    status: new
    disposition: add
    assertion: "a second resume right after a first is refused as outside a pause and the first is the one acted on, and a resume that arrives after the limit has fired, while the aid is still closing, is refused and the run ends focus_timeout"
    decision: "spec.md:US6-S9"
  - path: crates/finder/tests/console/imaging_run_focus.rs
    status: new
    disposition: add
    assertion: "clearly out without a record, unmeasurable and no record each pause with their own words; a check that did not finish because it found no stars to measure pauses with the words of unmeasurable, and one that failed otherwise with the words that it did not finish, its reason and to check the imaging camera, each of the two with its warning; and after a check that did not finish and resume(false), a clearly out check pauses with its own words"
    decision: "spec.md:US6-S10"
  - path: crates/finder/tests/console/imaging_run_focus.rs
    status: new
    disposition: add
    assertion: "a check still clearly out after resume(false) pauses again with the aid opened twice and no second release of the lease"
    decision: "spec.md:US6-S11"
  - path: crates/finder/tests/layout/main.rs
    status: existing
    disposition: preserve
    assertion: "no file at the top level of crates/finder/tests, no new file in its support folder, and each main.rs only its support include and mod lines"
    decision: "plan.md:Shapes that existing tests pin"
  preserved_behavior:
  - "033's verdict, thresholds, focus record and focus aid are unchanged; the part only calls them"
  - "Track stays on through a focus pause (spec.md Edge Cases)"
  - "no trait of a part gains a method and Steps gains no field"
  - "every scenario of 037's US1 to US3 and of US4 and US5 still holds: none gives a verdict that pauses, the one run whose check did not finish (037's US1-S2) is read only for focus and warnings, which US6 leaves as they are, and 037's US3's continue refusals outside a pause are unchanged"
  checks:
   development:
   - "cargo nextest run -p finder --locked --test console imaging_run_focus::"
   - "cargo nextest run -p finder --locked --test console imaging_run::"
   - "cargo nextest run -p finder --locked --test console imaging_run_routes::"
   - "cargo nextest run -p finder --locked --test layout"
   handoff: declared_gates
   independent: declared_pipeline
   release:
   - "release owner: the repository owner, through the merge queue's ci.yml jobs, and a focus pause at the telescope with the focuser turned by hand"
  writes:
  - crates/finder/src/imaging/run.rs
  - crates/finder/src/imaging/run/console.rs
  - crates/finder/tests/console/imaging_run_focus.rs
  - crates/finder/tests/console/main.rs
  no_write_reason: ""
  shared_writes:
  - path: crates/finder/src/imaging/run.rs
    other_spec: 041-imaging-run-on-the-rig
    other_story: US7
    disposition: merge_order
    reason: "US7 depends_on_merged US6, and adds the limits, the rule for subs that keep failing and the stack's own endings"
  - path: crates/finder/src/imaging/run/console.rs
    other_spec: 041-imaging-run-on-the-rig
    other_story: US8
    disposition: merge_order
    reason: "US8 depends_on_merged US6, and adds the stack part"
  - path: crates/finder/tests/console/main.rs
    other_spec: 041-imaging-run-on-the-rig
    other_story: US7
    disposition: merge_order
    reason: "US7 depends_on_merged US6, and adds mod imaging_run_limits"
  open_questions: []
 US7:
  tests:
  - path: crates/finder/tests/console/imaging_run_limits.rs
    status: new
    disposition: add
    assertion: "a run asked for 3 subs keeps stacking at 2 stacked with 1 dropped and ends count_limit at 3, saying that the 3 subs asked for are stacked, with the stack stopped once and its subs kept"
    decision: "spec.md:US7-S1"
  - path: crates/finder/tests/console/imaging_run_limits.rs
    status: new
    disposition: add
    assertion: "a 60 s limit counts from the start of stacking, not of the run: still stacking 59.6 s in and ended duration_limit by 60.6 s"
    decision: "spec.md:US7-S2"
  - path: crates/finder/tests/console/imaging_run_limits.rs
    status: new
    disposition: add
    assertion: "with a failed sub and none stacked the run still stacks 599.6 s after the last stacked sub and has ended subs_failing 1 s later, naming each failed reason's rise with spaces and not the subs the console was behind on; the words give each reason's rise since the last stacked sub, and a single failed sub is enough"
    decision: "spec.md:US7-S3"
  - path: crates/finder/tests/console/imaging_run_limits.rs
    status: new
    disposition: add
    assertion: "each kind of stack failure ends the run with its own reason, an unknown one as stack_failed with its words, a stack that ends with no failure as stopped with the sentence that the stack was stopped, and each still stops the stack once; a stack's failure in the poll that reaches the count is judged before the count"
    decision: "spec.md:US7-S4"
  - path: crates/finder/tests/console/imaging_run_limits.rs
    status: new
    disposition: add
    assertion: "a stack that fails as a camera in the same poll in which the mount's motion ends link_lost ends the run link_lost, with the stack stopped once"
    decision: "spec.md:US7-S5"
  - path: crates/finder/tests/console/imaging_run_limits.rs
    status: new
    disposition: add
    assertion: "700 s with only behind and unprocessed subs leave the run stacking, 700 s paused by the tracker leave it paused, and after the pause it still stacks 598.6 s on and has ended subs_failing 1 s later"
    decision: "spec.md:US7-S6"
  - path: crates/finder/tests/console/imaging_run_limits.rs
    status: new
    disposition: add
    assertion: "with keep_subs false the count limit asks the stack once to discard its subs, never to keep them"
    decision: "spec.md:US7-S7"
  - path: crates/finder/tests/console/imaging_run_limits.rs
    status: new
    disposition: add
    assertion: "the time limit counts the time the tracker pauses the stack: paused_by_tracker 59.6 s in and ended duration_limit by 60.6 s"
    decision: "spec.md:US7-S8"
  - path: crates/finder/tests/console/imaging_run_limits.rs
    status: new
    disposition: add
    assertion: "a stacked sub between two stretches of 400 s with failed subs starts the time again, and the run still stacks; and a stacked sub keeps the drops it saw, so that they end nothing 700 s later"
    decision: "spec.md:US7-S9"
  - path: crates/finder/tests/layout/main.rs
    status: existing
    disposition: preserve
    assertion: "no file at the top level of crates/finder/tests, no new file in its support folder, and each main.rs only its support include and mod lines"
    decision: "plan.md:Shapes that existing tests pin"
  preserved_behavior:
  - "every scenario of 037's US1 to US3 and of US4 to US6 still holds: the default request has no count and 7200 s, none stacks for 600 s with a dropped sub, and the script's stack runs until it is stopped or end_stack is called, whatever set_stack gave it (plan.md The script, Plan traps 24)"
  - "no trait of a part gains a method and Steps gains no field"
  - "nothing under crates/finder/src/imaging/run.rs waits a fixed time for an exposure (plan.md Plan traps 21)"
  checks:
   development:
   - "cargo nextest run -p finder --locked --test console imaging_run_limits::"
   - "cargo nextest run -p finder --locked --test console imaging_run::"
   - "cargo nextest run -p finder --locked --test console imaging_run_steps::"
   - "cargo nextest run -p finder --locked --test console imaging_run_routes::"
   - "cargo nextest run -p finder --locked --test layout"
   handoff: declared_gates
   independent: declared_pipeline
   release:
   - "release owner: the repository owner, through the merge queue's ci.yml jobs"
  writes:
  - crates/finder/src/imaging/run.rs
  - crates/finder/tests/console/imaging_run_limits.rs
  - crates/finder/tests/console/main.rs
  no_write_reason: ""
  shared_writes: []
  open_questions: []
 US8:
  tests:
  - path: crates/finder/tests/motion/imaging_run_stack.rs
    status: new
    disposition: add
    assertion: "on the simulator a run of the console's own five parts asked for 3 subs ends count_limit, saying that the 3 subs asked for are stacked, with saved true, run.json beside stack.json, stack.fits and stack.png and no reserve file in the stack's folder, the run's counts equal to stack.json's, whose name ends with the target's label, at least three kept subs in that folder, and Track still tracking with track_on true"
    decision: "spec.md:US8-S1"
  - path: crates/finder/tests/motion/imaging_run_stack.rs
    status: new
    disposition: add
    assertion: "keep_subs false ends count_limit with a saved stack and no listed sub or solve sidecar left on disk"
    decision: "spec.md:US8-S2"
  - path: crates/finder/tests/cli/imaging_run.rs
    status: new
    disposition: add
    assertion: "the built binary under --simulate, once its imaging camera is ready, answers a start with no target with 409 no target is selected, so it runs imaging runs"
    decision: "spec.md:US8-S3"
  - path: crates/finder/tests/motion/imaging_run_stack.rs
    status: new
    disposition: add
    assertion: "a stack stopped from 035's own route once a sub is stacked ends the run stopped, and the run's dropped counts hold no reason at 0"
    decision: "spec.md:US8-S4"
  - path: crates/finder/tests/motion/imaging_run_stack.rs
    status: new
    disposition: add
    assertion: "on a rig whose imaging folder holds a regular file named runs a run ends save_failed with a message beginning with the console could not save to its imaging folder: cannot make the stack folder"
    decision: "spec.md:US8-S5"
  - path: crates/finder/tests/motion/imaging_run_stack.rs
    status: new
    disposition: add
    assertion: "a scope node that stops answering while the run stacks ends it link_lost or camera_failed, the second with the words the scope node is not connected"
    decision: "spec.md:US8-S6"
  - path: crates/finder/tests/motion/imaging_run_stack.rs
    status: new
    disposition: add
    assertion: "once the stack part's start has answered, the run's folder holds a reserve file of 15 bytes for each binned pixel of the imaging view's camera plus 16777216, 32017096 bytes on the rig, and none once the run has been stopped; and reserve_bytes answers 260318656, 43845616 and 32017096 for the worked camera at binning 1 and 3 and the simulated one at binning 0"
    decision: "spec.md:US8-S7"
  - path: crates/finder/tests/motion/imaging_run_stack.rs
    status: new
    disposition: add
    assertion: "DELETE while stacking ends the run stopped with the stack saved, 035's stack no longer running, Track still tracking and the lease given back"
    decision: "spec.md:US8-S8"
  - path: crates/finder/tests/motion/imaging_run_stack.rs
    status: new
    disposition: add
    assertion: "run.rs, script.rs, store.rs and console.rs name no crate::goto, crate::tracking, crate::ephemeris, scope_link or unsafe, and crate::model only in console.rs, for separation"
    decision: "spec.md:US8-S9"
  - path: crates/finder/tests/cli/simulate.rs
    status: existing
    disposition: preserve
    assertion: "the binary's refusals and address line are unchanged"
    decision: "plan.md:Shapes that existing tests pin"
  - path: crates/finder/tests/layout/main.rs
    status: existing
    disposition: preserve
    assertion: "no file at the top level of crates/finder/tests, no new file in its support folder, and each main.rs only its support include and mod lines"
    decision: "plan.md:Shapes that existing tests pin"
  preserved_behavior:
  - "035's stack and 036's steering are unchanged; the part only calls them"
  - "no ending changes what Track is doing but those spec.md SC-005 names"
  - "no flag and no start-up line is added to the binary"
  - "every scenario of 037's US1 to US3 and of US4 to US7 still holds: the machine is not written, and the parts of US4 to US6 are not changed"
  checks:
   development:
   - "cargo nextest run -p finder --locked --test motion imaging_run_stack::"
   - "cargo nextest run -p finder --locked --test cli imaging_run::"
   - "cargo nextest run -p finder --locked --test motion imaging_run::"
   - "cargo nextest run -p finder --locked --test layout"
   handoff: declared_gates
   independent: declared_pipeline
   release:
   - "release owner: the repository owner, through the merge queue's ci.yml jobs, a rebuild of the console, and the field checklist's first run"
  writes:
  - crates/finder/src/api.rs
  - crates/finder/src/imaging/run/console.rs
  - crates/finder/src/main.rs
  - crates/finder/tests/cli/imaging_run.rs
  - crates/finder/tests/cli/main.rs
  - crates/finder/tests/motion/imaging_run_stack.rs
  - crates/finder/tests/motion/main.rs
  no_write_reason: ""
  shared_writes:
  - path: crates/finder/src/api.rs
    other_spec: 037-imaging-run
    other_story: US3
    disposition: merge_order
    reason: "041 depends_on_landed 037-imaging-run, whose 037's US3 last writes this file; re-read at the re-anchoring"
  - path: crates/finder/src/main.rs
    other_spec: 033-focus-reading-and-aid
    other_story: US8
    disposition: merge_order
    reason: "041 depends_on_landed 033-focus-reading-and-aid, whose US8 last writes this file in the drafts; re-read at the re-anchoring"
  - path: crates/finder/tests/cli/main.rs
    other_spec: 035-live-stack
    other_story: US8
    disposition: merge_order
    reason: "041 depends_on_landed 035-live-stack, whose US8 last writes this file in the drafts; re-read at the re-anchoring"
  open_questions: []
```
