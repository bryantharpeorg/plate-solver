# Implementation Plan: The Imaging Run

**Branch**: `037-imaging-run` | **Date**: 2026-11-15 | **Spec**: [spec.md](spec.md)

This plan records every design decision for the feature. A story that would change one
escalates rather than re-deciding it. The binding decisions above it are
`docs/plans/2026-11-15-specs-030-037-imaging.md` (the plan document),
`docs/adr/0002-imaging-cameras-through-indi-frames-relayed-console-stacks.md` and
`GLOSSARY.md`.

## Summary

Add a `Runner` to the console's imaging module. It carries one imaging run through five
parts it is handed, each a small trait: the mount, the camera, the focus check, the
solver and the stack. US1 writes the machine and a scripted stand-in for the parts, and
tests, in paused time, the path on which every step answers, the start's refusals, a
stop and the mount's endings. US2 adds what the machine does when a step fails or the
stack reports: the retries, the table of failed steps, the counts, the tracker's pause
and the warnings. US3 serves the runner over three routes, puts its view in the imaging
view and keeps `run.json`. US4 writes the console's own mount, camera and solver parts:
pointing through Track, the lease of the imaging camera, and 032's solve. US5 adds
centring to the machine and, to the mount part, the wait for a live pointing that a
re-centre needs. US6 adds the focus gate and its pause over 033; US7 the limits and the
stack's own endings; US8 the stack part over 035 and 036 and the binary's wiring. US9
adds the run block to the Imaging page, US10 four MCP tools, US11 the README section and
the field checklist. The run asks Track and nothing else to move the mount, asks nothing
of the mount when it ends, and holds no number sized to one telescope.

Eleven stories, where a first draft had eight: a second agent's review judged the
machine, the centring and the stacking stories each likely to pass 64 KiB, and a split
after a probe build would have meant a changed spec and a relaunch (§ Plan traps 5).
Each of the three is two stories here (§ Decisions and the alternatives rejected).

## Technical Context

**Language/Version**: Rust (MSRV 1.96) for US1 to US8, US10 and US11; TypeScript with
React 19 for US9.

**Primary Dependencies**: nothing new. `finder` already has `tokio` with `sync` and
`time`, `serde` and `serde_json` (`crates/finder/Cargo.toml:32`), and `rmcp`. The page
adds no npm package.

**Storage**: `run.json` under the console's imaging folder (031: `imaging/` in the state
folder, or `--imaging-dir`): in the stack's folder, `runs/<stack id>/`, beside 035's
`stack.json`, `stack.fits` and `stack.png`, or in `runs/<run id>/` for a run that ends
before it stacks. Subs stay under `frames/`, where 031 saves every imaging frame.
Nothing about a run is kept in `session.json`: it is run-only, like Track.

**Testing**:
- US1: `crates/finder/tests/console/imaging_run.rs` (new), the module `imaging_run` of
  finder's `console` test binary: a `Runner` over the script, in paused time.
- US2: `crates/finder/tests/console/imaging_run_steps.rs` (new), the module
  `imaging_run_steps` of `console`: the same, for failed steps, counts and warnings.
- US3: `crates/finder/tests/console/imaging_run_routes.rs` (new), the module
  `imaging_run_routes` of `console`: the router over the scripted console.
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
- US9: `crates/finder/web/src/components/ImagingRunBlock.test.tsx` (new), Vitest. No new
  Playwright test (spec owner decision 17).
- US10: `crates/finder/tests/mcp/imaging_run.rs` (new), the module `imaging_run` of
  finder's `mcp` test binary.
- US11: `crates/finder/tests/console/imaging_run_docs.rs` (new), the module
  `imaging_run_docs` of `console`.
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
  the 900 s of a focus pause, the 60 s of a time limit and the 5 s a re-centre may wait
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
- No lock of the run's own on the imaging camera: 031's one lease (031 US6), taken and
  given back (the plan document, amendment 1).
- `web/dist` is committed and must be the build of its source (US9).
- Budgets are things a test measures; this plan states none for the real telescope.

**Scale/Scope**: one module with three children, three routes, one page block, four MCP
tools, one README section, in eleven stories.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Nothing touches the solver, detection or `crates/math-core`. The order of the steps, the centring tolerances in use elsewhere and the forced stop of a solve loop that never converges are grounded in `docs/research/2026-11-15-imaging-camera.md` § 12. Prior art and what was not adopted; the 60″ of GoTo and the tracker's endings in the code cited in spec.md § Clarifications. The run's own counts and limits are owner decisions, named as such. |
| II. Never weaken a gate | No existing test, golden, fixture, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS`, `ergane.yaml` or `playwright.config.ts` is edited. The only edits to existing test files are the `mod` lines spec owner approval 8 names. |
| III. Tests first, independent expectations | Oracles: for the machine, the words, states and call lists each scenario states, compared with what a scripted stand-in recorded; for the real parts, the simulator's truth (the true aim point, the true centre of the imaging frame, the mount's true motion), the session, and the answers of 031's and 033's own routes. `run.json` and `stack.json` are parsed by the tests themselves. Clocks are injected; virtual time only in tests and `finder --simulate`. |
| IV. Critical paths | No `unsafe`, no dependency or `Cargo.toml` change, no database-format change, nothing under `proto/`, nothing in `crates/scope-link`. |
| V. Standalone Rust, honest licensing | The finder camera's solver is untouched. The run reaches ASTAP only through 032's solver port; the constitution's reading of principle V for that is the plan document's first item before any flip. |

## Project Structure

```text
crates/finder/
├── README.md                             # US11: three API rows, four tools, ## An imaging run
├── src/lib.rs                            # US3: three route lines
├── src/api.rs                            # US3: imaging_runner, with_imaging_run_steps; US8: with_imaging_run; US10: the tool name in start_motion
├── src/main.rs                           # US8: .with_imaging_run()
├── src/imaging/mod.rs                    # US1: pub mod run; US3: run in ImagingView, attach_run
├── src/imaging/routes.rs                 # US3: three routes
├── src/imaging/run.rs                    # US1: new, the machine; US2: failed steps, counts; US3: the record; US4: pub mod console; US5: centring; US6: the pause; US7: limits
├── src/imaging/run/script.rs             # US1: new, the scripted stand-in
├── src/imaging/run/store.rs              # US3: new, run.json
├── src/imaging/run/console.rs            # US4: new, mount, camera and solver parts; US5: the wait of repoint; US6: focus part; US8: stack part
├── src/mcp/tools.rs                      # US10: four tools
├── tests/console/main.rs                 # US1, US2, US3, US5, US6, US7, US11: one mod line each
├── tests/console/imaging_run.rs          # US1: new
├── tests/console/imaging_run_steps.rs    # US2: new
├── tests/console/imaging_run_routes.rs   # US3: new
├── tests/console/imaging_run_centring.rs # US5: new
├── tests/console/imaging_run_focus.rs    # US6: new
├── tests/console/imaging_run_limits.rs   # US7: new
├── tests/console/imaging_run_docs.rs     # US11: new
├── tests/motion/main.rs                  # US4, US5, US8: one mod line each
├── tests/motion/imaging_run.rs           # US4: new
├── tests/motion/imaging_run_centring.rs  # US5: new
├── tests/motion/imaging_run_stack.rs     # US8: new
├── tests/cli/main.rs                     # US8: mod imaging_run
├── tests/cli/imaging_run.rs              # US8: new
├── tests/mcp/main.rs                     # US10: mod imaging_run
├── tests/mcp/imaging_run.rs              # US10: new
└── web/
    ├── src/api.ts                        # US9: ImagingRun, three calls
    ├── src/components/ImagingRunBlock.tsx       # US9: new
    ├── src/components/ImagingRunBlock.test.tsx  # US9: new
    ├── src/components/ImagingPage.tsx    # US9: placement
    └── dist/                             # US9: rebuilt
DESIGN.md                                 # US9: the block
.impeccable/design.json                   # US9: the Imaging run component
```

## Interfaces

Everything 030 to 036 create is named as their plan.md gives it in the drafting
worktree: each of the seven reviewed by a second agent, and all the trios of the series
audited side by side. The lease of the imaging camera, which the plan document's
amendment 1 gives 031, is 031's US6; 033, 034 and 035 take the same lease. All of it
is re-read on `main` before the flip (§ Prerequisites 2 and 3).

### What 030 to 036 offer (read, never written, by this spec)

| What a part needs | From | Name | Where it is written |
|---|---|---|---|
| the camera's state, description, settings and latest imaging frame | 031 | `Imaging::view()` → `ImagingView { state, camera, settings, frame, .. }`; `SettingsView::roi`; `CameraView::{width, height}` | 031 plan.md § The console's imaging state |
| one imaging frame, awaited, and its abort | 031 | `Imaging::take(purpose)`, `Imaging::ended(id)` → `FrameView`, `Imaging::abort()` | 031 plan.md § Receiving and saving |
| why a request was not carried out | 031 | `crate::imaging::Refusal { status, message }` | 031 plan.md § The console's imaging state |
| the one lease of the imaging camera, and an imaging frame taken by its holder | 031 | `Imaging::lease(holder)` → `Lease`, released when dropped; `Lease::take(purpose)`; a refusal `the imaging camera is held by {holder}` | 031 US6 (031 plan.md, its section on the lease); the plan document, amendment 1 |
| the UTC text of a time | 031 | `crate::imaging::fits::utc(unix_s) -> (String, String)` | 031 plan.md § FITS at the console |
| a target's name as a folder's name takes it | 035 | `crate::imaging::stack::service::label(name: &str) -> String` | 035 spec.md FR-035, US8-S3; 035 plan.md § The stack on the console |
| the imaging folder | 034 | `Imaging::root()` | 034 plan.md § 031's imaging state |
| whether there is an imaging solver | 032 | `Imaging::solver()` | 032 plan.md § Solving on the console |
| solve the latest imaging frame, awaited | 032 | `Imaging::solve()`, `Imaging::solved(frame_id)` → `SolveView { state, ra_deg, dec_deg, arcsec_per_px, reason, error, .. }` | 032 plan.md § Solving on the console |
| whether a solve moved the aim point | 032 | `SessionView::aim` → `Option<AimView { x, y, source }>`, read before the solve and after it; `Session::held_aim()` → `Option<Aim>`, the measured aim point held back while Track is on, read after it; 032's task hands a measured aim point to `Session::set_measured_aim`, which holds it back while Track is on, before the solve reads `solved` | 032 plan.md § The session (the held aim point), § The relation, § Solving on the console |
| the focus check, awaited | 033 | `Imaging::focus_start(FocusAsk { check: true, .. })`, `Imaging::focus_ended()` → `FocusView { unit, aid, .. }`, `AidView { check, state, verdict, median, last, reference, ended_reason, error, .. }` | 033 plan.md § The aid on the console, § The focus view |
| the focus aid, opened and closed | 033 | `Imaging::focus_start(FocusAsk::default())`, `Imaging::focus_stop()` | 033 plan.md § The aid on the console |
| the live stack | 035 | `AppState::stack()` → `Stacker`; `Stacker::start(StartRequest, Option<&str>)`, `Stacker::stop()`, `Stacker::view()` → `StackView { state, id, stacked, dropped, ended_reason, error, save_error, .. }` | 035 plan.md § The stack on the console, § The console |
| whether the stack waits for the tracker, and the set point | 036 | `Imaging::steering()` → `Steering::waiting()`, `Steering::set_set_point(None)` | 036 plan.md § The steering, § The imaging state |
| whether the tracker is steady enough to expose | 036 | `Session::steered()` → `Steered::steady()` | 036 plan.md § The session, § The tracking error and the interface |

### The machine (`crates/finder/src/imaging/run.rs`, new, US1)

```rust
//! The imaging run (037): one procedure from a chosen target to a saved stack, carried
//! through five parts the console hands it. It moves the mount only by asking its mount
//! part for Track, and asks nothing of the mount when it ends.
pub mod script;
// US3: pub mod store;        US4: pub mod console;

/// How often a run reads the mount and the stack.
pub const POLL: Duration = Duration::from_millis(250);
/// The time limit of a run whose request gives none, in seconds of stacking.
pub const DEFAULT_DURATION_S: f64 = 7200.0;
pub const MAX_DURATION_S: f64 = 43_200.0;
pub const MAX_SUBS: u32 = 100_000;

/// The state words, in the order a run can pass through them.
pub const STATES: [&str; 10] = ["slewing", "tracking", "first_frame", "focus_check",
    "focus_pause", "solving", "centring", "stacking", "paused_by_tracker", "ended"];
/// The ending reasons.
pub const ENDINGS: [&str; 17] = ["stopped", "count_limit", "duration_limit", "too_low",
    "sun_up", "link_lost", "mount_failed", "target_changed", "pushed_away", "camera_failed",
    "solver_unavailable", "solver_failing", "save_failed", "focus_timeout", "not_centred",
    "subs_failing", "stack_failed"];

pub type StepFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum By { Page, Mcp }
impl By { pub fn as_str(self) -> &'static str; }          // "page", "mcp"

/// Why a run is over. The whole enum, its reasons and its messages are here from US1;
/// a later story adds only the code that reaches a variant.
#[derive(Debug, Clone, PartialEq)]
pub enum Ending {
    Stopped, CountLimit, DurationLimit, TooLow, SunUp, LinkLost,
    /// With the words Track or GoTo gave.
    MountFailed(String),
    TargetChanged,
    /// 038's Track view shows a push by hand.
    PushedAway,
    CameraFailed(String), SolverUnavailable(String), SolverFailing(String), SaveFailed(String),
    FocusTimeout,
    NotCentred { offset_arcsec: f64 },
    /// With the reasons that rose, as `not_solved 7, too_few_stars 3`.
    SubsFailing(String),
    StackFailed(String),
}
impl Ending {
    /// Its word in `ENDINGS`.
    pub fn reason(&self) -> &'static str;
    /// The sentence of § The run's words. `name` is the target's.
    pub fn message(&self, name: &str, stacked: u64, duration_s: f64) -> String;
}

/// Why a part did not do what it was asked.
#[derive(Debug, Clone, PartialEq)]
pub enum Failure {
    /// The observer's Stop arrived.
    Stopped,
    Camera(String),
    Save(String),
    /// The imaging solver cannot run at all.
    SolverUnavailable(String),
    /// The imaging solver ran and found no solution.
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

/// The five parts. The five traits and this struct are complete from US1: no later
/// story or spec adds a method or a field, because the script implements them all.
#[derive(Clone)]
pub struct Steps {
    pub mount: Arc<dyn MountSteps>,
    pub camera: Arc<dyn FrameSteps>,
    pub focus: Arc<dyn FocusSteps>,
    pub solver: Arc<dyn SolveSteps>,
    pub stack: Arc<dyn StackSteps>,
}

/// The body of `POST /api/imaging/run`.
#[derive(Debug, Default, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunRequest {
    pub subs: Option<u32>,
    pub duration_s: Option<f64>,
    pub keep_subs: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RunSettings { pub subs: Option<u32>, pub duration_s: f64, pub keep_subs: bool }
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct SubCounts { pub stacked: u64, pub dropped: BTreeMap<String, u64> }
/// `reading` and `record` are skipped while `None`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FocusNote { pub verdict: &'static str, pub reading: Option<f64>, pub record: Option<f64>, pub unit: &'static str }
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CentringNote { pub tries: u32, pub offset_arcsec: f64, pub tolerance_arcsec: f64 }
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EndedNote { pub reason: &'static str, pub at: f64, pub message: String }

/// `run` in the imaging view. Serialized in this field order; `focus`, `centring`,
/// `ended` and `folder` are skipped while `None`, `warnings` while empty;
/// `settings.subs` is `null` when no number was asked for.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct RunView {
    pub id: String,
    pub state: &'static str,
    pub by: &'static str,
    pub target: RunTarget,
    pub started_at: f64,
    pub elapsed_s: f64,
    pub settings: RunSettings,
    pub subs: SubCounts,
    pub focus: Option<FocusNote>,
    pub centring: Option<CentringNote>,
    pub message: String,
    pub warnings: Vec<String>,
    pub ended: Option<EndedNote>,
    /// Relative to the imaging folder.
    pub folder: Option<String>,
}

/// A target's name as a folder name takes it: every character that is not an ASCII
/// letter or digit made `-`. Calls 035's `crate::imaging::stack::service::label`.
pub fn label(name: &str) -> String;
/// `<UTC start as YYYY-MM-DDTHH-MM-SSZ>-<label(name)>`.
pub fn run_id(started_at: f64, name: &str) -> String;

pub struct Runner {
    /* steps, clock, the task's handle, and an inner `Arc` it shares with the run's
       task: one std Mutex over the latest run */
}
/// Dropping the runner aborts the run's task. The task holds no `Arc<Runner>`.
impl Drop for Runner { fn drop(&mut self); }
impl Runner {
    pub fn new(steps: Steps, clock: Arc<dyn Clock>) -> Arc<Self>;
    /// Check, take the lease, ask Track, and start the run's task. Answers the new
    /// run's view.
    pub async fn start(self: &Arc<Self>, request: RunRequest, by: By) -> Result<RunView, Refusal>;
    /// End the run that is on as `stopped` and wait until it has ended. The latest
    /// view, or `None` before the first start.
    pub async fn stop(&self) -> Option<RunView>;
    /// Go on from a focus pause (US6). Until then always refused.
    pub fn resume(&self, anyway: bool) -> Result<RunView, Refusal>;
    /// The latest run, on or ended; `None` before the first accepted start.
    pub fn view(&self) -> Option<RunView>;
}
```

`Clock` is the console's (`crates/finder/src/session.rs:24`) and `Refusal` is 031's
`crate::imaging::Refusal { status, message }`.

**`label`** is one line, `crate::imaging::stack::service::label(name)`: the pure
function 035 offers for the rule its stack uses to name its folder, and which 035's
`AppState::target_label` calls on the session's target (035 plan.md § The stack on the
console, § The console). The rule is written once, in 035; the machine passes the name
it was handed and reads no session, so it stays mount-neutral (§ Plan traps 18). US1-S1
pins the answer to the worked example `NGC 1976` → `NGC-1976`.
**`run_id`** takes the second string of 031's `crate::imaging::fits::utc(started_at)`
(`YYYY-MM-DDTHH:MM:SS.mmm`), keeps its first nineteen characters, makes each `:` a `-`,
and adds `Z`, `-` and `label(name)`. `run_id(1768528800.0, "M42")` is
`2026-01-16T02-00-00Z-M42`, and with the name `NGC 1976` it ends `-NGC-1976`.

**`start`**, in this order, each refusal a `Refusal`:
1. `subs` of 0 or above `MAX_SUBS`: 400 `subs must be a whole number from 1 to 100000`.
   `duration_s` not finite, below 1.0 or above `MAX_DURATION_S`: 400
   `duration_s must be a number from 1 to 43200`.
2. A run that has not ended: 409 `an imaging run is already running`.
3. `camera.ready()`, then `solver.ready()`: the first `Err(words)` is 409 with the
   words.
4. `mount.target()`: `Err(words)` is 409 with the words.
5. `camera.hold()`: `Err(words)` is 409 with the words.
6. `mount.point(by).await`: `Err(words)` calls `camera.release()` and is 409 with the
   words.
7. The new run replaces the latest one: `id` from `run_id(clock.now(), &target.name)`,
   no `folder`, `settings` with `duration_s` or `DEFAULT_DURATION_S` and `keep_subs` or
   true, the state `slewing`. Its task is spawned and its view answered.

Checks 2 to 7 run under one async lock, so two starts cannot both pass check 2. The
lease is taken before Track is asked, so the mount never moves for a run that cannot
expose.

**The runner's own lock.** The latest run (the fields of its view, its events, its stop
signal) lies under one std `Mutex`, in the inner `Arc` the runner and the run's task
share. Nothing is awaited and no part is called while that lock is held: a step copies
what it needs out of the lock, releases it, calls the part, and locks again to record
the answer. The console's parts read the imaging view, and the imaging view reads
`Runner::view` (§ `crates/finder/src/imaging/mod.rs`), which takes that same lock; a
part called under it would wait on itself for ever (§ Plan traps 23). The async lock of
`start` is another lock, and parts are called under it.

**Dropping the runner.** `Runner` implements `Drop`, which aborts the run's task. The
task holds the inner `Arc` and never an `Arc<Runner>`, so the runner lives exactly as
long as its owners, the console's state among them, keep it. An aborted task runs no
ending: it asks nothing more of any part, and its parts are dropped with it, with
whatever they hold (US1-S6). That is what keeps 015's rule for a dropped console
(`crates/finder/tests/motion/tracking/routes.rs:456`) true while a run is on: the
console's own parts each hold a clone of the console's state, and that clone holds the
owner of Track.

**The task.** Times shown to the observer (`started_at`, `ended.at`) are the console
clock's; everything measured (`elapsed_s`, and from US6 and US7 the pause and the time
limit) is `tokio::time::Instant`, which a paused test clock drives. Three helpers:

- `guarded(step)`: `tokio::select!`, `biased`, over the stop signal, an interval of
  `POLL` at which `mount.state()` is read, and the step. A stop answers `Stopped`; an
  `Ended(e)` answers `e`; either drops the step's future. Otherwise the step's value.
- `steady(show)`: first at once and then every `POLL`, `mount.state()`: `Ended(e)` ends
  the run with `e`; `Tracking { steady: true }` returns; `Slewing` and
  `Tracking { steady: false }` wait, and when `show` is `Some((a, b))` the state is set
  to `a` for the first and `b` for the second, with its message. A stop ends the run
  `Stopped`. `steady(None)` leaves the state as it is.
- `frame()`: `steady(None)`, then `guarded(camera.take())`. Given up by a stop or an
  ending: `camera.abort().await`, then the run ends that way. `Ok` is the answer.
  `Err(Stopped)` ends the run `Stopped`. In US1 any other `Err(f)` ends it
  `CameraFailed` with `f`'s words; US2 replaces that arm (§ Failed steps and counts in
  the machine). `frame()` never changes the state.

The task, in US1:
1. `steady(Some(("slewing", "tracking")))`.
2. The state is `first_frame`; `frame()`. The imaging frame is not used again.
3. The state is `focus_check`; `steady(None)`; `camera.release()`, because the focus
   check takes the lease itself; `guarded(focus.check())`. Given up by a stop or an
   ending: the run notes that a check was given up and ends that way, and the ending
   stops the check. `Ok(check)` sets `focus` from it, and the run goes on whatever the
   verdict: US2 adds the warning of `SlightlySoft`, and US6 puts the pause where
   `ClearlyOut`, `Unmeasurable` and `NoRecord` go on. `Err(Stopped)` ends the run
   `Stopped`. Any other `Err(f)` sets `focus` to `unmeasurable` with no reading, no
   record and the unit `px`, adds the warning
   `The focus check did not finish: {words}.`, and goes on as `Unmeasurable` does; US2
   first sorts two kinds of failure out of it. Whenever the run goes on from here to
   step 4, it first takes the lease again: `camera.hold()`, whose `Err(words)` ends the
   run `CameraFailed(words)`.
4. The state is `solving`. `frame()`, then `guarded(solver.solve(&frame, &target))`.
   `Ok(centre)` sets `centring` to `{tries: 0, offset_arcsec, tolerance_arcsec}` and
   goes on: US5 adds the loop here. `Err(Stopped)` ends the run `Stopped`. In US1 any
   other `Err(f)` ends it `SolverFailing` with `f`'s words; US2 replaces that arm.
5. The state is `stacking`, with its message; `camera.release()`, because the stack
   takes the lease itself; then `stack.start(&target).await`, not guarded. `Ok(folder)`
   sets the view's `folder`. `Err(Stopped)` ends the run `Stopped`. In US1 any other
   `Err(f)` ends it `StackFailed` with `f`'s words; US2 replaces that arm. A stack whose
   start failed is not stopped.
6. Every `POLL`: `mount.state()` `Ended(e)` ends the run with `e`; a stop ends it
   `Stopped`. In US1 the counts stay at zero and the state stays `stacking`: US2 reads
   `stack.progress()` here, and US7 adds the limits and the stack's own endings.

`stack.start`, `stack.stop`, `focus.open_aid`, `focus.close_aid` and `mount.repoint`
are awaited unguarded: dropping one half-way would leave a stack, a focus aid or Track
in a state nobody owns. Each is bounded by the part behind it: `repoint` by
`FRESH_POLLS` polls, one Track off and one Track on (§ The console's mount part), the
other four by 033's and 035's own starts and stops. A stop of the run that arrives
during one is acted on as soon as the call has answered: the stop signal stays set, and
the next `guarded` or `steady` ends the run `Stopped` (spec FR-005, US5-S3).

**Ending**, in this order: `focus.close_aid()`, once, if the run gave up a check or,
from US6, is ending from a focus pause, whether or not the aid could be opened;
`stack.stop(settings.keep_subs)` if step 5 returned `Ok`, whoever ended the stack, a
failure of which is ignored in US1 and from US2 adds the warning
`The stack did not end cleanly: {words}.`; `camera.release()`, always and last, whether
or not the run still holds the lease; then `ended` is set with the reason, the console
clock's time and the message, the state is `ended`, `elapsed_s` is fixed, and whoever
waits in `stop` is woken. Nothing is asked of the mount part after `point` and
`repoint`. A run that ends drops its task; nothing restarts it.

**`stop`** sets the stop signal of a run that is on and waits for its ending; with none
on it answers `view()` at once. **`view`** copies the run under the runner's lock,
releases it, and sets `elapsed_s` from the instant of the start to now, or to the
ending.

### Failed steps and counts in the machine (`crates/finder/src/imaging/run.rs`, US2)

```rust
/// Imaging frames in a row the camera may fail before the run ends.
pub const FRAME_TRIES: u32 = 3;
/// Solves in a row that may find no solution before the run ends.
pub const SOLVE_TRIES: u32 = 3;
```

US2 replaces the arms US1 gave a failed take, a failed solve and a failed `stack.start`
with this table, sorts the failures of a focus check by it, and reads the stack's
progress. US5 and US7 use its last two rows.

| A failed step | `Stopped` | `Camera` | `Save` | `SolverUnavailable` | `NotSolved` | `Mount`, `Other` |
|---|---|---|---|---|---|---|
| `camera.take` | `Stopped` | counted | `SaveFailed` | counted | counted | counted |
| `focus.check` | `Stopped` | unmeasurable | `SaveFailed` | `SolverUnavailable` | unmeasurable | unmeasurable |
| `solver.solve` | `Stopped` | counted | `SaveFailed` | `SolverUnavailable` | counted | counted |
| `stack.start` (US2), and `StackProgress::failed` (US7) | `Stopped` | `CameraFailed` | `SaveFailed` | `SolverUnavailable` | `SolverFailing` | `StackFailed` |
| `mount.repoint` (US5) | `Stopped` | `MountFailed` | `MountFailed` | `MountFailed` | `MountFailed` | `MountFailed` |

- **A take that is counted** is one more failure. At `FRAME_TRIES` in a row the run ends
  `CameraFailed` with the last one's words; otherwise `frame()` goes round again:
  `steady(None)`, then another take. `Ok` zeroes the count.
- **A solve that is counted** is one more failure. At `SOLVE_TRIES` in a row the run
  ends `SolverFailing` with the last one's words; otherwise `frame()` and the solve
  again. No focus check is repeated. `Ok` zeroes the count.
- **A focus check** that fails with `Save(e)` or `SolverUnavailable(e)` ends the run
  that way. Every other failure is US1's: unmeasurable, with its warning.
- **`stack.start`** that fails ends the run by its row; the stack is not stopped.
- **`stack.stop`** that fails at an ending adds the warning
  `The stack did not end cleanly: {words}.` and changes no reason.
- **`SlightlySoft`** adds its focus sentence as a warning and goes on.
- **Step 6**, every `POLL`, before the mount's state and the stop are read:
  `stack.progress()` is copied into `subs`, and the state is `paused_by_tracker` while
  `waiting_for_tracker`, else `stacking`, each with its message. Nothing in US2 reads
  `running` or `failed`: US7 does.

### The run's words

Every sentence is exactly this, with `{name}` the target's name, arcseconds of an offset
written with no decimal (`{:.0}`), and a whole number of seconds written as one (`60`,
by `{}` of the `f64`). A focus reading or record is written with one decimal and its
unit: `3.6″` for `Unit::Arcsec`, `6.6 px` for `Unit::Px`.

| State | `message` |
|---|---|
| `slewing` | `Slewing to {name}.` |
| `tracking` | `Tracking {name}. Waiting for the tracker to be steady.` |
| `first_frame` | `Taking a first imaging frame.` |
| `focus_check` | `Checking focus.` |
| `focus_pause` (US6) | the focus sentence below, then ` Turn the focuser, then check focus again or go on anyway.`; for `no_record`, then ` Focus with the focus aid, then check focus again or go on anyway.` |
| `solving` | `Solving an imaging frame.` |
| `centring` (US5) | `Centring {name}: {offset}″ from the sensor's centre, try {n} of 3.` |
| `stacking` | `Stacking {name}: {stacked} stacked, {dropped} dropped.`, `{dropped}` the sum over the reasons |
| `paused_by_tracker` | `Paused by the tracker: {stacked} stacked so far.` |
| `ended` | the ending's message |

| Verdict | The focus sentence |
|---|---|
| `slightly_soft`, with a reading and a record | `Focus is slightly soft: stars are {reading} across against a record of {record}.` |
| `slightly_soft`, without either | `Focus is slightly soft.` |
| `clearly_out`, with a reading and a record | `Focus is clearly out: stars are {reading} across against a record of {record}.` |
| `clearly_out`, with a reading only | `Focus is clearly out: stars are {reading} across.` |
| `clearly_out`, with no reading | `Focus is clearly out.` |
| `unmeasurable` | `Focus could not be measured.` |
| `no_record` | `There is no focus record for this camera yet.` |

| Ending | `reason` | `message` |
|---|---|---|
| `Stopped` | `stopped` | `Stopped by the observer.` |
| `CountLimit` | `count_limit` | `Finished: {stacked} subs stacked, the number asked for.` |
| `DurationLimit` | `duration_limit` | `Finished: the {duration_s} s limit was reached.` |
| `TooLow` | `too_low` | `Ended: {name} is below the minimum altitude.` |
| `SunUp` | `sun_up` | `Ended: the Sun is up.` |
| `LinkLost` | `link_lost` | `Ended: the scope link was lost.` |
| `MountFailed(w)` | `mount_failed` | `Ended: the mount could not reach or follow {name}: {w}.` |
| `TargetChanged` | `target_changed` | `Ended: the target was changed. Start a new imaging run for the new target.` |
| `PushedAway` | `pushed_away` | `Ended: the telescope was pushed away from {name}. Track follows where it is left.` |
| `CameraFailed(w)` | `camera_failed` | `Ended: the imaging camera failed: {w}.` |
| `SolverUnavailable(w)` | `solver_unavailable` | `Ended: the imaging solver cannot run: {w}.` |
| `SolverFailing(w)` | `solver_failing` | `Ended: the imaging solver keeps failing: {w}.` |
| `SaveFailed(w)` | `save_failed` | `Ended: the console could not save to its imaging folder: {w}. Check the disk.` |
| `FocusTimeout` | `focus_timeout` | `Ended: focus was left paused for 15 minutes.` |
| `NotCentred` | `not_centred` | `Ended: {name} is still {offset}″ from the sensor's centre after 3 tries.` |
| `SubsFailing(w)` | `subs_failing` | `Ended: the last 10 subs were all dropped ({w}).` |
| `StackFailed(w)` | `stack_failed` | `Ended: the stack stopped: {w}.` |

| Warning | Words |
|---|---|
| verdict `slightly_soft` (US2) | its focus sentence |
| a focus check that did not finish (US1) | `The focus check did not finish: {words}.` |
| going on anyway (US6) | `Focus was not checked again: the observer went on anyway.` |
| the focus aid (US6) | `The focus aid could not be opened: {words}.` |
| the stack at an ending (US2) | `The stack did not end cleanly: {words}.` |
| the record at an ending (US3) | `The run's record could not be saved: {words}.` |

The three tables are whole in `run.rs` from US1, as functions of the state, the verdict
and the ending; a later story calls what an earlier one wrote. The one sentence whose
subject is an altitude, the `too_low` message, is an observer's word and not code
(§ Plan traps 18).

### The script (`crates/finder/src/imaging/run/script.rs`, new, US1)

```rust
//! Scripted stand-ins for the five parts of an imaging run, as `FixedClock` stands in
//! for a clock: tests queue the answers and read back the calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Part { Camera, Solver, Target, Hold, Point }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Step { Take, Check, Solve, Repoint, StackStart }

/// The folder the script's stack answers.
pub const STACK_FOLDER: &str = "runs/2026-01-16T02-00-10Z-M42";

pub struct Script { /* one std Mutex over its answers and calls; one Notify for holds */ }
impl Script {
    pub fn new() -> Arc<Self>;
    /// All five parts are this script.
    pub fn steps(self: &Arc<Self>) -> Steps;
    /// What `MountSteps::state` answers from now on.
    pub fn set_mount(&self, state: MountState);
    /// Make `camera.ready()`, `solver.ready()`, `mount.target()`, `camera.hold()` or
    /// `mount.point()` refuse with these words.
    pub fn refuse(&self, part: Part, words: &str);
    /// Queue the next answer of a take, a focus check or a solve. An empty queue
    /// answers the default.
    pub fn push_frame(&self, answer: Result<FrameRef, Failure>);
    pub fn push_focus(&self, answer: Result<FocusCheck, Failure>);
    pub fn push_solve(&self, answer: Result<Centre, Failure>);
    /// Make the next `repoint`, `open_aid`, `stack.start` or `stack.stop` fail once.
    pub fn fail_repoint(&self, failure: Failure);
    pub fn fail_open_aid(&self, failure: Failure);
    pub fn fail_stack_start(&self, failure: Failure);
    pub fn fail_stack_stop(&self, failure: Failure);
    /// The counts and the tracker's pause that `StackSteps::progress` answers from now
    /// on. `dropped` is reason and count; a count of 0 is left out. Whether the stack
    /// runs, and why it ended, are not touched.
    pub fn set_stack(&self, stacked: u64, dropped: &[(&str, u64)], waiting_for_tracker: bool);
    /// Make the stack end by itself: it no longer runs, and `failed` is as given.
    pub fn end_stack(&self, failed: Option<Failure>);
    /// Make every call of a step wait, after it is listed in `calls`, until it is
    /// released.
    pub fn hold(&self, step: Step);
    /// Let the calls of a step that wait go on, and end its hold.
    pub fn release(&self, step: Step);
    /// The calls the run made, in order.
    pub fn calls(&self) -> Vec<String>;
}
```

Defaults: the target `RunTarget { id: "NGC 1976", name: "M42", ra_deg: 83.82, dec_deg:
-5.39 }`; a camera and a solver that are ready and a lease that is free; the mount
`Tracking { steady: true }`; a take answers `FrameRef { id: n, file:
"frames/2026-01-16/00000n.fits" }` for n = 1, 2, 3 … (six digits), a failed take using
no n; a focus check `FocusCheck { verdict: Good, reading: Some(3.0), record: Some(3.0),
unit: Arcsec }`; a solve `Centre { offset_arcsec: 0.0, tolerance_arcsec: 90.0, aim_moved:
true }`; `stack.start` answers `STACK_FOLDER`.

The stack's progress, as `StackSteps::progress` answers it:
- `running` is the script's own. It is false before `stack.start`, true from the moment
  `stack.start` answers `Ok`, and false again once `stack.stop` is called or
  `end_stack` is. No other call changes it.
- `failed` is `None` until `end_stack(Some(failure))`.
- `stacked`, `dropped` and `waiting_for_tracker` are 0, empty and false until
  `set_stack`, and then what its last call gave.

So `set_stack` can never end a stack. That matters across stories: US2's and US3's
tests set counts on a stacking run, and US7 then makes a stack that no longer runs end
the run; were `running` part of what `set_stack` sets, a test written two stories
earlier would start to fail, and its story could not be reopened to fix it (§ Plan
traps 24).

The call words, one string a call: `camera.ready`, `solver.ready`, `mount.target`,
`camera.hold`, `mount.point page` or `mount.point mcp`, `camera.take`, `camera.abort`,
`camera.release`, `focus.check`, `focus.open_aid`, `focus.close_aid`,
`solver.solve {frame id}`, `mount.repoint`, `stack.start {target name}`,
`stack.stop keep` or `stack.stop discard`. `state()` and `progress()` are not listed. A
refused `ready()`, `target()`, `hold()` or `point()` is listed.

It is public product code that only tests call (spec owner decision 16, owner approval
5). It holds no logic of the run: it answers what it was told and writes down what it
was asked. It is whole from US1, with the controls only later stories use, so that no
later story writes this file.

### `crates/finder/src/imaging/mod.rs` (US1; US3)

US1 adds `pub mod run;` in alphabetical order among the module lines 030 to 036 put
there. US3 adds, after the view's last part:

```rust
// ImagingView gains:
#[serde(skip_serializing_if = "Option::is_none")]
pub run: Option<run::RunView>,
// Imaging gains a field `run: std::sync::OnceLock<std::sync::Weak<run::Runner>>` and:
/// Let every view this state answers carry the run (037). The first call wins.
pub(crate) fn attach_run(&self, runner: &Arc<run::Runner>);
```

It follows 035's `attach_stack` (035 plan.md § 031's imaging state): wherever `Imaging`
builds the `ImagingView` it answers, after its own lock is released and where 035 sets
`stack`, it sets `run` from `attached.view()` when a runner is attached and still alive,
**whatever the view's `state`**. `ImagingView::off()` has `run` `None`: a console
without imaging has no runner either. The reference is weak because the runner's parts
hold the `Arc<Imaging>`. `attached.view()` takes the runner's lock, which is why the
runner never calls a part while it holds that lock (§ The machine, § Plan traps 23).

### The record (`crates/finder/src/imaging/run/store.rs`, new, US3)

```rust
//! The run's record, `run.json` (037).
pub const FORMAT: &str = "imaging-run/1";

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Event { pub at: f64, pub state: &'static str }

pub struct Store { /* the imaging folder, and the run's folder once it has one */ }
impl Store {
    pub fn new(root: PathBuf) -> Self;
    /// The run's folder, relative to the imaging folder, once it has one.
    pub fn folder(&self) -> Option<&str>;
    /// Keep the record in this folder, relative to the imaging folder, from now on.
    pub fn adopt(&mut self, folder: &str);
    /// With no folder yet, make `runs/<id>/`, as `runs/<id>-2/`, `-3` … when the name
    /// is taken, and adopt it.
    pub fn make(&mut self, id: &str) -> std::io::Result<()>;
    /// Write `run.json` whole, as `run.json.part`, then rename it. `Ok` without a
    /// folder.
    pub fn write(&self, view: &RunView, events: &[Event]) -> std::io::Result<()>;
}
```

`make` uses `create_dir_all(root/runs)` and then `create_dir` of the run's folder,
trying the next name on `AlreadyExists`, as 025's recorder names its folders
(`crates/finder/src/recording.rs:235`). `write` first calls `create_dir_all` of the
folder, which 035 has already made for a real stack and which a scripted stack never
makes, and is synchronous `std::fs`: the file is a few hundred bytes, and a
blocking-pool hop would let a paused test clock run on.

`run.json`, its keys in this order: `format`, `finder` (`CARGO_PKG_VERSION`), `id`,
`by`, `target`, `started_at`, `started_utc` (`YYYY-MM-DDTHH:MM:SSZ`, from the same
`fits::utc` string as the id), `settings`, `state`, `events`, `subs`, then `focus`,
`centring`, `warnings` and `ended` when the view has them. No `message`, `elapsed_s` or
`folder`, and nothing of the site, the node's address or its token.

`Runner` gains, in US3:

```rust
/// As `new`, keeping each run's `run.json` under the imaging folder `root`.
pub fn with_folder(steps: Steps, clock: Arc<dyn Clock>, root: PathBuf) -> Arc<Self>;
```

Every run, with a folder or not, keeps a list of `Event`s, one pushed with the console
clock's time each time the state word changes, `slewing` at the start and `ended` at the
end included. With a folder:
- nothing is written before the run stacks;
- when `stack.start` answers `Ok(folder)`: `store.adopt(&folder)`, then `write`;
- at every later change of the state word: `write`;
- a `write` that fails while the run is on ends the run `SaveFailed(e.to_string())`;
- at the ending, after the lease is given back: with no folder yet, `store.make(&id)`,
  and the view's `folder` becomes the store's; then `write`, with the stack's counts as
  last read. A failure of either adds the warning of § The run's words, changes no
  reason, and after a failed `make` leaves the view without a `folder`.

The stack's counts are written at changes of state and at the ending, not at every
poll.

### The console (`crates/finder/src/api.rs`, US3; US8; US10)

`AppState` (`crates/finder/src/api.rs:101`) gains
`imaging_runner: Option<Arc<crate::imaging::run::Runner>>` and, in US3:

```rust
/// Give the console the parts of its imaging runs (037). Call after `with_imaging`;
/// without imaging it changes nothing.
pub fn with_imaging_run_steps(mut self, steps: crate::imaging::run::Steps) -> Self;
/// The owner of the console's imaging runs, when it has one.
pub fn imaging_runner(&self) -> Option<Arc<crate::imaging::run::Runner>>;
```

`with_imaging_run_steps` builds `Runner::with_folder(steps, self.clock.clone(),
imaging.root().to_path_buf())` (`Imaging::root`, 034 US4), calls
`imaging.attach_run(&runner)` and keeps it. `SessionResponse`
(`crates/finder/src/api.rs:652`) does not change: its `imaging` is 031's view, which now
carries `run`. `start_motion` (`crates/finder/src/api.rs:593`) and `stop`
(`crates/finder/src/api.rs:904`) are already `pub(crate)`.

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
them. That is what `Runner`'s `Drop` is for (§ The machine).
`crates/finder/src/main.rs` calls it once, after the lines 032 and 033 put after
`.with_live()` (`crates/finder/src/main.rs:114`), `.with_imaging_solver(…)` and
`.with_focal_length_mm(…)`, on both the `--simulate` and the real path.

US10 changes one line of `start_motion`, the refusal of an MCP Track while a direct axis
move runs (`crates/finder/src/api.rs:623`): `tool == "set_tracking"` becomes
`matches!(tool, "set_tracking" | "start_imaging_run")`.

### The routes (`crates/finder/src/imaging/routes.rs`, US3)

```rust
// routes() gains:
//   POST   /api/imaging/run            start_run
//   DELETE /api/imaging/run            stop_run
//   POST   /api/imaging/run/continue   continue_run
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ContinueEdit { anyway: bool }
```

- `start_run` takes `SessionWrite` and `Result<Json<RunRequest>, JsonRejection>` through
  `sim_json` (`crates/finder/src/api.rs:570`), so a `Content-Type` other than
  `application/json` is 415, a body that is not JSON 400, and JSON of another shape 400.
  Then, with no runner, 409 `this console runs no imaging runs`. Then
  `api::start_motion(&state, "page", "imaging_run", || …)`, whose closure awaits
  `runner.start(request, By::Page)` and turns a `Refusal` into
  `ApiError::new(status, message)`; a refusal leaves the motion record as it was
  (`crates/finder/src/api.rs:644`). It answers `Json` of the imaging view.
- `stop_run` takes `SessionWrite` and no body: no runner is the same 409; otherwise
  `runner.stop().await`, and the imaging view.
- `continue_run` takes `SessionWrite` and `Result<Json<ContinueEdit>, JsonRejection>`
  through `sim_json`: no runner is the same 409; a `Refusal` of `runner.resume(anyway)`
  becomes its status and words; otherwise the imaging view.

`resume`, from US1 until US6 gives it its work, refuses with 409
`no imaging run is running` when no run is on and 409 `the imaging run is not paused`
otherwise.

`crates/finder/src/lib.rs` gains, after the last `/api/imaging` route line then there,
three lines, written as they stand here, with no backticks:
`* POST /api/imaging/run: {"subs":N,"duration_s":S,"keep_subs":true}, each optional; start an imaging run on the session's target.`
`* DELETE /api/imaging/run: no body; end the imaging run and save its stack. It does not stop the mount.`
`* POST /api/imaging/run/continue: {"anyway":false} checks focus again, {"anyway":true} goes on; from a focus pause only.`

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

pub fn mount(state: &AppState) -> Arc<dyn MountSteps>;
pub fn camera(state: &AppState) -> Arc<dyn FrameSteps>;
pub fn solver(state: &AppState) -> Arc<dyn SolveSteps>;
// US6: pub fn focus(state: &AppState) -> Arc<dyn FocusSteps>;
// US8: pub fn stack(state: &AppState) -> Arc<dyn StackSteps>;
```

Each part keeps a clone of the `AppState` it was made from. The mount part also keeps,
from `point` on, the stop generation, the target's id and `by`; and a flag,
`repointing`, that is true only between its own Track off and Track's answer.

- **`target`**: `state.session().snapshot(state.clock.now()).target`
  (`crates/finder/src/session.rs:1002`). None: `Err("no target is selected")`. Else
  `RunTarget { id, name: messier or id, ra_deg, dec_deg }`
  (`crates/finder/src/targets.rs:39`).
- **`point(by)`**: without `state.goto()` (`crates/finder/src/api.rs:296`),
  `Err("no mount is connected")`. It reads `*goto.stops().borrow()`
  (`crates/finder/src/goto.rs:801`), then awaits `api::start_tracking(&state)`
  (`crates/finder/src/api.rs:831`), the function the Track route and the MCP
  `set_tracking` call; its error's `message` is the refusal, unchanged. On `Ok` it keeps
  the generation, the target's id and `by`. It does not call `start_motion`: the route
  already holds the motion lock around `Runner::start`, and that lock is not re-entrant.
- **`state`**, the first that holds:

  | What the part reads | It answers |
  |---|---|
  | the stop generation differs from the one kept | `Ended(Stopped)` |
  | the session's target is none, or its id is not the one kept | `Ended(TargetChanged)` |
  | `track.push` present (038's FR-009), while `repointing` is not set | `Ended(PushedAway)` |
  | no `track` in the snapshot | `Ended(MountFailed("tracking is off"))` |
  | `track.state` `armed` | `Slewing` |
  | `track.state` `off`, `ended_reason` `stopped`, while `repointing` is set | `Slewing` |
  | `off`, `stopped` otherwise, or `the observer stopped it` | `Ended(Stopped)` |
  | `off`, `link_lost` or `halted by a lost link` | `Ended(LinkLost)` |
  | `off`, `sun_up` or `halted by the Sun guard` | `Ended(SunUp)` |
  | `off`, `too_low` | `Ended(TooLow)` |
  | `off`, any other reason, or none | `Ended(MountFailed(reason or "tracking ended"))` |
  | any other `track.state` | `Tracking { steady }`, `steady` being `state.session().steered().is_some_and(\|t\| t.steady())` (036) |

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
- **`repoint`**, in this order. US4 writes steps 1, 2 and 4 to 6; US5 adds step 3 and
  `FRESH_POLLS`.
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
  4. `let asked = api::start_motion(&state, by, tool, || api::start_tracking(&state)).await`,
     with `tool` `imaging_run` for `page` and `start_imaging_run` for `mcp`;
     `repointing` is cleared, whatever `asked` is.
  5. The generation differs now, whatever `asked` is: `api::stop(&state).await`
     (`crates/finder/src/api.rs:904`), then `Err(Stopped)`.
  6. `asked` is `Err(error)`: `Err(Mount(message))`, the error's `message`. Otherwise
     `Ok(())`.

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
  which spec owner decision 26 cites, settled by the owner on 2026-11-17). Right after
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
  measured aim point; during step 3 Track is off. As 032 was first drafted, the aim
  point was set at once and this wait came before Track off: the tracker took the whole
  step from the old aim point as its error and closed it at a rate nothing bounds
  (`crates/finder/src/tracking.rs:304`), under Track's own guards and outside GoTo's,
  for up to 5 s (spec § Clarifications). The owner's answer to 032's decision 25 closed
  that in 032. The probe of US5-S4 confirms that the simulator's mount does not move
  toward the new aim point between the imaging solve and step 2 (§ Prerequisites 5).

  `stop_tracking` waits for the mount's acknowledgement of its stop, not for rest. If
  an axis still reports motion when step 4's Track asks GoTo, GoTo refuses with
  `the mount is already moving` (`crates/finder/src/goto.rs:739`) and the run ends
  `mount_failed` (§ Risks).

### The console's camera and solver parts (`crates/finder/src/imaging/run/console.rs`, US4)

**Camera.**

```rust
/// The name a run holds the imaging camera's lease under.
pub const HOLDER: &str = "the imaging run";
```

It keeps one `Mutex<Option<Arc<Lease>>>`. The lease is 031's US6, which 033, 034 and
035 take as well: `Imaging::lease(holder)` answers a `Lease` or a 409 `Refusal` whose
message is `the imaging camera is held by {holder}`, naming whoever holds the camera,
and does not wait; while a lease is out, `Imaging::take` and `Imaging::set` by anyone
else are refused the same way; `Lease::take(purpose)`, which borrows the lease and is
awaited, is `Imaging::take` for the holder; `Imaging::abort`, `ended`, `view` and 032's
`solve` are open to anyone; and dropping the `Lease` gives the camera back. `HOLDER` and
the purpose `run` are the two words this spec gives 031's records and refusals (spec
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
  `Err(Camera(message))`; then `imaging.ended(id)` for the id of the answered view's
  `frame`: `saved` is `Ok(FrameRef { id, file })`; `failed` whose `error` begins
  `could not save the imaging frame` is `Err(Save(error))`, any other `failed`
  `Err(Camera(error))`; `aborted` is `Err(Camera("the exposure was aborted"))`.
- `abort`: `imaging.abort().await`.

**The lease comes before the look for an imaging frame in flight.** 033's focus aid
takes one exposure after another and 035's stack one sub after another, so while either
holds the camera an imaging frame is almost always `exposing`. A part that looked for
that first would answer a start with `an exposure is in progress`; asked for the lease
first, it answers with the lease's words, which name the holder (US6-S7). The words
`an exposure is in progress` are left for a single imaging frame taken under no lease.

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
the stars lost in the corners is counted as any other (§ Failed steps and counts in the
machine). This is a known limit (spec § Clarifications), which the README states and
the field checklist's step 8 has the observer look for.

The focus part and the stack part never touch the lease: 033's `focus_start` takes it
as `the focus aid` and 035's `Stacker::start` as `the stack`, each for as long as it
runs, which is why the machine gives the run's own lease up just before it calls them
(spec owner decision 15). The lock around the kept lease is a std `Mutex` and is never
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
   `id` is still `frame.id`, and `CENTRE_FLOOR_ARCSEC` alone otherwise. `aim_moved` is
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
```

Step 4 of the task becomes a loop. `tries` starts at 0. After each `Ok(centre)`,
`centring` is set to `{tries, offset_arcsec, tolerance_arcsec}`, and then:
- `offset_arcsec <= tolerance_arcsec`: go on to stacking;
- else, `tries == CENTRE_TRIES`: the run ends `NotCentred { offset_arcsec }`;
- else `tries += 1`, `centring.tries` is set to it, the state is `centring` with its
  message, whose `{n}` is `tries`, and: when `centre.aim_moved`,
  `mount.repoint().await`, not guarded, whose `Err` ends the run by the table of
  § Failed steps and counts in the machine; then `frame()`, which waits until the mount
  tracks and is steady, and the solve again, the state staying `centring` throughout. A
  take or a solve that fails inside the loop is counted as US2 counts it.

A stop of the run that arrives while `repoint` is awaited is not lost: the signal stays
set, `repoint` answers within its bound, and the `steady` at the head of `frame()` ends
the run `Stopped` before another take (US5-S3).

### The focus gate in the machine (`crates/finder/src/imaging/run.rs`, US6)

```rust
/// How long a focus pause may last.
pub const FOCUS_PAUSE_LIMIT: Duration = Duration::from_secs(900);
```

In step 3, `ClearlyOut`, `Unmeasurable`, `NoRecord` and a check that did not finish no
longer go on (the warning `The focus check did not finish: {words}.` stays). Instead:
1. the state is `focus_pause` with the verdict's message; the run notes that it is in a
   pause; `focus.open_aid().await`, an `Err` adding the warning of § The run's words
   and changing nothing else;
2. it waits, with `tokio::select!`, for the first of: a resume; a stop; an `Ended` from
   `mount.state()`, read every `POLL`; `FOCUS_PAUSE_LIMIT` since the pause began. The
   last three end the run, as `Stopped`, as the mount's ending and as `FocusTimeout`;
   the ending closes the aid because the note is set, whether or not the aid opened:
   033's `focus_stop` is safe when nothing runs, and one rule is easier to test than
   two;
3. a resume: `focus.close_aid().await`, on the same rule, and the note cleared. With
   `anyway` false the run goes back to step 3, the focus check, the lease still given
   up. With `anyway` true
   the warning `Focus was not checked again: the observer went on anyway.` is added and
   the run goes to step 4, taking the lease again first, as every way on from step 3
   does.

`Runner::resume(anyway)` now: no run on, 409 `no imaging run is running`; a run not in
`focus_pause`, 409 `the imaging run is not paused`; else it hands `anyway` to the
waiting task and answers the view.

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

  With any other `ended_reason` (`stopped`, `failed`, `no_region`):
  `Err(Other(words))`, the words being
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
/// Subs dropped in a row, with none stacked between them, that end a run.
pub const DROP_RUN: u64 = 10;
```

Step 6 of the task, every `POLL`, after the counts are copied and before the state is
shown, the first that holds:
1. `progress.failed` is `Some(f)`: the run ends by the row of `StackProgress::failed`
   in the table of § Failed steps and counts in the machine;
2. `progress.running` is false: the run ends `Stopped`;
3. `settings.subs` is `Some(n)` and `stacked >= n`: `CountLimit`;
4. the instant now is `settings.duration_s` or more after the instant `stack.start`
   answered `Ok`: `DurationLimit`. This is the clock's time, on tokio's clock: a
   stretch in which the stack waits for the tracker counts, because the limit is there
   to end a forgotten run and not to measure exposure (spec owner decision 8, US7-S2);
5. the drop run: the task keeps the `dropped` map as it stood when `stacked` last rose,
   or when stacking began. When the sum of `dropped` now, less that map's sum, is
   `DROP_RUN` or more, the run ends `SubsFailing(words)`, the words being, for each
   reason whose count rose, `{reason} {rise}`, in the map's order, joined by `, `.

A failed `stack.start` is US2's, unchanged. Every one of these endings still calls
`stack.stop`, as § The machine's Ending has it: the stack is saved whoever ended it.

Rule 2 is why the script's `set_stack` leaves `running` alone (§ The script): US2's,
US3's and US10's tests set counts on a stacking run and must go on passing once this
rule is in the machine.

### The console's stack part and the wiring (`crates/finder/src/imaging/run/console.rs`, `crates/finder/src/api.rs`, `crates/finder/src/main.rs`, US8)

`stack(state)` keeps the id of the stack it started. With `stacker` being
`state.stack()` (035), which is there whenever imaging is:
- `start(target)`: `imaging.steering().set_set_point(None)` (036): the run says where
  on the sensor it wants the target, today the centre. Then
  `stacker.start(StartRequest::default(), Some(&run::label(&target.name))).await`. A
  `Refusal` whose message begins `cannot make the stack folder` is `Err(Save(message))`;
  any other is `Err(Other(message))`. `Ok(view)` keeps `view.id` and answers
  `format!("runs/{}", view.id)`.
- `progress`: from `stacker.view()`; `None`, or a view whose `id` is not the one kept,
  answers `StackProgress::default()`. Otherwise `running` is `state == "stacking"`;
  `stacked` is the view's; `dropped` is the view's `dropped` object as
  `serde_json::to_value` gives it, keeping each key whose number is above 0, so a
  reason a later spec adds is carried without a change here; `waiting_for_tracker` is
  `imaging.steering().waiting()` (036); and `failed`, for an `ended` view, is
  `Some(Camera(error))` when `ended_reason` is `camera`, else `Some(Save(words))` when
  the view has a `save_error`, else `None`.
- `stop(keep_subs)`: `stacker.stop().await`. A view with a `save_error` is
  `Err(Save(save_error))`, and no sub is removed. Otherwise, with `keep_subs` false: it
  reads `stack.json` in the stack's folder under `imaging.root()`, and for every entry
  of its `subs` array that has a `file`, removes that file and the file whose name ends
  `.solve.json` in place of `.fits`, a missing file being no failure; the first other
  failure, after every entry was tried, is
  `Err(Other(format!("could not remove {file}: {error}")))`; a `stack.json` that cannot
  be read or parsed is `Err(Other(format!("could not read stack.json: {error}")))`.
  The removals are synchronous `std::fs`, as the record's writes are.

035's stack takes its own subs and 036's steering corrects the tracker from them inside
035's loop (036 plan.md § What 035 must provide, and the stack's three calls); the part
adds nothing to either.

`with_imaging_run` and the binary's line are in § The console.

### The page (`crates/finder/web/src/`, US9)

`api.ts`, after the imaging types 030 to 036 put there:

```ts
export interface ImagingRun {
  id: string;
  state: "slewing" | "tracking" | "first_frame" | "focus_check" | "focus_pause" | "solving"
    | "centring" | "stacking" | "paused_by_tracker" | "ended";
  by: "page" | "mcp";
  target: { id: string; name: string; ra_deg: number; dec_deg: number };
  started_at: number;
  elapsed_s: number;
  settings: { subs: number | null; duration_s: number; keep_subs: boolean };
  subs: { stacked: number; dropped: Record<string, number> };
  focus?: {
    verdict: "good" | "slightly_soft" | "clearly_out" | "unmeasurable" | "no_record";
    reading?: number; record?: number; unit: "arcsec" | "px";
  };
  centring?: { tries: number; offset_arcsec: number; tolerance_arcsec: number };
  message: string;
  warnings?: string[];
  ended?: { reason: string; at: number; message: string };
  folder?: string;
}
// ImagingView gains: run?: ImagingRun;
export interface ImagingRunRequest { subs?: number; duration_s?: number; keep_subs?: boolean }
export async function postImagingRun(request: ImagingRunRequest): Promise<ImagingView>;
export async function deleteImagingRun(): Promise<ImagingView>;        // fetch(url, { method: "DELETE" })
export async function postImagingRunContinue(anyway: boolean): Promise<ImagingView>;
```

The two posts are written as `postTrack` is (`crates/finder/web/src/api.ts:312`), with
`{ method: "POST", headers: { "Content-Type": "application/json" }, body:
JSON.stringify(…) }`, and each call throws an `Error` with the answer's `error` on a
refusal. `postImagingRun` builds its body with the keys in the order `subs`,
`duration_s`, `keep_subs`, each only when given.

`components/ImagingRunBlock.tsx`, `ImagingRunBlock({ view, refresh }: { view:
ImagingView; refresh: () => Promise<void> | void })`: nothing unless `view.state` is
`ready` or `view.run` is present; else a `section` labelled by an `h2` `Imaging run`,
with these words exactly, each phrase a test asserts on one line of JSX:

- **No run, or an ended one**: a `quiet` paragraph
  `Goes to the target, checks focus, centres it on the imaging camera's sensor and stacks subs until you stop it or a limit is reached.`;
  number fields labelled `Subs to stack` and `Minutes to stack`, both empty; a checkbox
  `Keep every sub`, checked; and the button `Start imaging run` (`button
  button-primary`). A click calls `postImagingRun` with `subs` when the first field
  holds a whole number above 0, `duration_s` as the second field's number times 60 when
  it holds one above 0, and `keep_subs` always; then `refresh`. A refusal shows its
  words in an `alert` and calls no `refresh`. Drafts in the fields survive the
  one-second reads of 031's `useImaging`.
- **A run that is on**: the run's `message` in a `status`, or in an `alert` while the
  state is `focus_pause`; a line `{stacked} stacked · {dropped} dropped`, `{dropped}`
  the sum, followed when it is above 0 by ` ({reason with each _ made a space} {n}, …)`
  in the object's order; a line `Elapsed {m}:{ss}`, from `elapsed_s` rounded down to
  whole seconds; when `by` is `mcp`, the line `Started by an MCP client`, with the
  classes of `McpNote` (`crates/finder/web/src/components/McpNote.tsx:5`); each of
  `warnings` as a `quiet` paragraph; and the button `Stop imaging run` (`button`), which
  calls `deleteImagingRun` then `refresh`. In `focus_pause`, also the buttons
  `Check focus again` (`button button-primary`), which calls
  `postImagingRunContinue(false)`, and `Go anyway` (`button`), which calls it with
  `true`, each then `refresh`. No fields and no `Start imaging run`.
- **An ended run**, above the start form: `{ended.message}` followed by
  ` {stacked} subs stacked · saved in {folder}.` when `stacked` is above 0 and by
  ` No sub was stacked.` otherwise, in a `status` when the reason is `stopped`,
  `count_limit` or `duration_limit` and in an `alert` for every other reason; and each
  warning.

Only existing classes and tokens are used; `index.css` does not change. The block holds
no picture, so the night rule and the hold-to-see-colour control need nothing of it.
`ImagingPage.tsx` (031 US4) renders `<ImagingRunBlock view={view} refresh={refresh} />`
right after its title block, before the not-ready line and the Camera section, whenever
it has a view.

### `DESIGN.md` and `.impeccable/design.json` (US9)

After the last imaging entry then in `DESIGN.md` (the Recording entry is at
`DESIGN.md:350` today, and 030 to 036 add theirs after it), a plain `###` heading and
one paragraph. The `>` marks only set the text apart here:

> ### Imaging run — built
> First on the Imaging page, while the imaging camera is ready or a run has been
> started: a block titled Imaging run. With no run on, one quiet line says what a run
> does, two optional ruled fields take a number of subs and of minutes, a checkbox keeps
> every sub, and one primary button reads Start imaging run. While a run is on, one
> status line carries the run's own message, with the counts of subs stacked and
> dropped, the elapsed time and Stop imaging run as a secondary button; a run an MCP
> client started says so in Second Ink, as the MCP motion note does. A focus pause turns
> the message into an alert and adds Check focus again as the primary button and Go
> anyway beside it. After a run the block says why it ended and where the stack was
> saved: a status for a stop or a limit, an alert for anything else. It never stops the
> mount: the top bar's Stop does. Ink on paper with existing tokens only; at night it is
> red like every other block.

`.impeccable/design.json` gains one entry in `components`, after the last imaging
component then there, formatted as its neighbours: `name` `Imaging run`, `kind`
`custom`, `refersTo` `button`, a `description` saying the same in one sentence each, and
`html` and `css` snippets under the class `ds-imaging-run`, using only
`var(--color-ink)`.

### The MCP tools (`crates/finder/src/mcp/tools.rs`, US10)

Two structs that only describe a tool's input, beside `TargetArguments`
(`crates/finder/src/mcp/tools.rs:31`):

```rust
#[derive(rmcp::schemars::JsonSchema)]
#[expect(dead_code, reason = "schema only; RunRequest parses the actual arguments")]
struct ImagingRunArguments { subs: Option<u32>, duration_s: Option<f64>, keep_subs: Option<bool> }
#[derive(serde::Deserialize, rmcp::schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct ImagingRunContinueArguments { anyway: bool }
```

Four tools in the `#[tool_router]` block (`crates/finder/src/mcp/tools.rs:165`), each
with `output_schema = object_schema()` (`crates/finder/src/mcp/tools.rs:104`). Each
answer is `field_result("run", …)` (`crates/finder/src/mcp/tools.rs:110`) of the run's
view as JSON, or of `null`; each refusal is `CallToolResult::error` with the words.

- `start_imaging_run`, description
  `Move a real telescope to the session's target and start an imaging run there, as POST /api/imaging/run does: GoTo, tracking, a first imaging frame, a focus check, centring on the imaging camera's sensor, and stacking until it is stopped or reaches a limit.`,
  with `input_schema` from `schema_for_input::<ImagingRunArguments>()`, as `set_target`
  declares its own (`crates/finder/src/mcp/tools.rs:354`). It takes `arguments:
  JsonObject`. In this order: `self.state.limit_move()?`
  (`crates/finder/src/api.rs:305`); the arguments parsed as `RunRequest` the way
  `set_target` parses its edit, a parse error being the tool's error; no runner, the
  error `this console runs no imaging runs`; then
  `api::start_motion(&self.state, "mcp", "start_imaging_run", || …)` around
  `runner.start(request, By::Mcp)`, as `goto_target` wraps its start
  (`crates/finder/src/mcp/tools.rs:216`). It writes one call-log line with
  `self.state.log_call("start_imaging_run", …)` whatever the result
  (`crates/finder/src/api.rs:322`).
- `stop_imaging_run`, description
  `End the imaging run and save its stack, as DELETE /api/imaging/run does. It does not stop the mount: call stop for that.`:
  takes `arguments: JsonObject`; any key is the error
  `stop_imaging_run takes no arguments`; no runner, the same error as above; otherwise
  `runner.stop().await`.
- `continue_imaging_run`, description
  `Go on from a focus pause of the imaging run, as POST /api/imaging/run/continue does: with anyway false, focus is checked again; with anyway true, the run goes on unchecked.`,
  with `input_schema` from `schema_for_input::<ImagingRunContinueArguments>()`: takes
  `arguments: JsonObject` and parses them itself with `serde_json::from_value`, so a
  missing or extra key is a tool error with serde's words and not a protocol error; no
  runner, the same error; a `Refusal` of `runner.resume(anyway)` is the error.
- `get_imaging_run`, read-only (`annotations(read_only_hint = true)`), description
  `Read the imaging run: its state, target, counts of subs stacked and dropped by reason, elapsed time, and its message for the observer.`:
  takes nothing, as `get_session` does (`crates/finder/src/mcp/tools.rs:347`), and
  answers the view, or `null` with no runner or no run.

Only `start_imaging_run` is a moving tool. The door needs no change: it stops on `stop`
and `stop_all` only (`crates/finder/src/mcp/door.rs:33`), and `stop_imaging_run` is not
a Stop. The MCP session tools carry `imaging.run` because they serialize
`session_response`; no other MCP code changes. No run tool returns a picture: the
stack's picture is 035's `get_stack`.

### The README: rows, tools, the section and the field checklist (`crates/finder/README.md`, US11)

US11 is the one story of this spec that writes the README, so that every line it gains
is asserted by the story that writes it (US11-S1 to US11-S3).

Three rows after the last `/api/imaging` row of the `## API` table
(`crates/finder/README.md:167`):

```markdown
| `POST /api/imaging/run` | JSON with optional `subs` (1 to 100000), `duration_s` (1 to 43200, default 7200) and `keep_subs` (default true). Start an imaging run on the session's target: GoTo, tracking, a first imaging frame, a focus check, centring on the imaging camera's sensor and stacking. Answers 200 with the imaging view, whose `run` says where it has got to; 400 or 409 with a JSON `error`. Same-origin writes only. |
| `DELETE /api/imaging/run` | No body. End the imaging run as `stopped` and save its stack; answers 200 with the imaging view once it has ended. It does not stop the mount: `POST /api/stop` does, and ends the run too. Same-origin writes only. |
| `POST /api/imaging/run/continue` | JSON `{"anyway":false}` or `{"anyway":true}`. From a focus pause, check focus again, or go on unchecked; 409 with a JSON `error` when no run is paused. Same-origin writes only. |
```

In `## MCP clients` (`crates/finder/README.md:57`): four lines after the imaging tools
then there, worded like their neighbours, the first saying that `start_imaging_run`
moves the telescope, is rate-limited and is logged.

A section `## An imaging run`, after the last imaging section then there. In prose: what
a run does, in order; that the imaging camera is held throughout, by the run, the focus
aid or the stack in turn, so a single imaging frame, the focus aid, a set of darks or
flats and a stack asked for meanwhile are refused with the holder's name; that it moves
the mount only by asking Track, and never stops it, so after a run Track is still on
unless a guard or Stop ended the motion; a table of the ten states with their words
between backticks and one line each; a table of the seventeen endings with their words
between backticks, what each means and what the mount is doing afterwards; the limits
(`subs`, `duration_s` and its 7200 s default, counted on the clock from the start of
stacking) and the counts (three imaging frames, three solves, three centring tries, ten
subs, 15 minutes at a focus pause, 5 s for the finder camera after a re-centre's Track
off); the centring tolerance; that the run's own imaging frames are taken at the
camera's exposure setting and only the stack's subs are capped by the tracker's limit,
so that a long setting on an alt-az mount trails the corner stars of the imaging frames
the run solves; the three routes and the four tools between backticks; where
`run.json` lies and its keys; what `keep_subs` false removes; and what `POST /api/stop`
and the MCP `stop` do to a run.

Then `### Field checklist: the first imaging run`, exactly fourteen numbered steps, each
one or two sentences with what to write down (US11-S2 reads the word `corner` in the
text from the line that begins `8.` to the line that begins `9.`):

1. Before dark: the imaging camera on the focuser, the scope node and the console
   running, and the Imaging page showing the camera as ready.
2. After dark: pair, set the site, and wait for the first sync; write down the model's
   RMS.
3. Choose a bright target well above the minimum altitude.
4. Take one imaging frame by hand and solve it by hand; write down the scale and the
   time the solve took.
5. Press `Start imaging run` with 20 subs asked for.
6. Write down the time from the press to `tracking`, and GoTo's last error.
7. At the focus check, write down the verdict, the focus reading and the focus record;
   if the run pauses, turn the focuser and press `Check focus again`.
8. At centring, write down each try's offset and the tolerance, how many tries it took,
   and whether the telescope stayed still between the first imaging solve and the first
   re-centre's Track off: a newly measured aim point waits until Track is off.
   Open the last imaging frame the run solved and write down whether its corner stars
   are trailed, since the run's own imaging frames are not capped as subs are.
9. While stacking, write down the subs stacked and dropped by reason after 5 minutes,
   and whether the run was ever paused by the tracker.
10. Let the run end at its 20 subs; check that Track is still on and that the folder
    holds `run.json` and the stack, and that the subs it lists are on the disk.
11. Start a second run and press the top bar's Stop while it stacks; check that the
    mount is at rest and stays there, and that the run ended `stopped`.
12. From an MCP client: `set_target`, then `start_imaging_run` with a time limit of 300
    s; read `get_imaging_run` until it ends; check that the page said who started it.
13. Start a third run and end it with `stop_imaging_run`; check that Track is still on.
14. Copy the three `run.json` files and this filled-in list into the night's field note.

## Shapes that existing tests pin

These are not changed by any story:
- **Track, GoTo and Stop.** `Session::start_tracking`, `Session::stop_tracking`,
  `GoTo`, `Tracker`, `api::stop`, `api::start_tracking` and the routes over them keep
  their signatures, words and behaviour; the run only calls them. Their tests are
  finder's `motion` binary, among them
  `crates/finder/tests/motion/tracking/routes.rs:136` and the Stop tests under
  `crates/finder/tests/motion/stop`.
- **`start_motion`'s rules for the existing tools.** US10's one changed line adds a tool
  name to a `matches!`; every existing pairing of `by` and `tool` is refused or accepted
  as before (`crates/finder/tests/mcp/motion.rs:87`).
- **`GET /api/session` and the MCP session bodies without a run.** `run` is skipped when
  absent, so every existing body is byte for byte what it was.
- **The MCP tool list's existing entries**, and the test that calls every listed tool
  with empty arguments (`crates/finder/tests/mcp/motion.rs:528`): on its rig, which has
  no runner, `start_imaging_run` and `stop_imaging_run` answer
  `this console runs no imaging runs`, `continue_imaging_run` a parse error and
  `get_imaging_run` `{"run":null}`, none with the token. `tools/list` is read by name
  (`crates/finder/tests/mcp/endpoint.rs:30`), not counted.
- **031's imaging view and its tests.** `ImagingView` is `#[non_exhaustive]` and its
  tests compare it key by key (031's spec.md § User Scenarios), so a `run` key that
  appears only after a start changes none of them. `Imaging::take`, `ended`, `abort`
  and `set`, and their refusals, are 031's and unchanged; so is the lease (031 US6).
- **032's solve and relation, 033's check and aid, 035's stack, 036's steering.** The
  parts call them as their plans give them, and change none.
- **The scripted tests of earlier stories, for the later ones.** A test file an earlier
  story of this spec created is an existing test, and no later story may edit it.
  `crates/finder/tests/console/imaging_run.rs` and the scripted modules after it build
  `Steps` by `script.steps()` and read `RunView` field by field. So no story adds a
  method to a part's trait or a field to `Steps`, changes `Runner::new`, `start`,
  `stop`, `resume` or `view`, or writes the script after US1. Each story changes what
  the task does between two states, and what it replaces is asserted by nothing before
  it:
  - US2 replaces what US1 does with a failed take, solve or `stack.start`, an ending at
    once, and reads the stack's progress. No US1 scenario fails a step or sets a count;
    the zero counts of US1-S1 are the script's defaults.
  - US3 adds `Runner::with_folder` and leaves `Runner::new` writing nothing.
  - US5 adds the centring loop. The script's default offset of 0″ never enters it, and
    US4-S1 asserts its solved offset inside the tolerance, so its run enters none
    either; US4-S5's `repoint` on the part alone gains only a wait of at most 5 s.
  - US6 replaces the going-on of a verdict that pauses. No earlier scenario gives one,
    and the refusals of `resume` outside a pause keep their words (US3-S4).
  - US7 adds the limits and the stack's own endings. The default request has no count
    and 7200 s, which no earlier scenario stacks for, and the script's stack runs until
    it is stopped or `end_stack` is called, whatever `set_stack` gave it (US2-S3,
    US3-S3). That last point is why `set_stack` does not set `running` (§ The script).
- **Finder's test layout.** `crates/finder/tests/layout/main.rs` fails a file at the top
  level of `crates/finder/tests` (`crates/finder/tests/layout/main.rs:109`) and any file
  added to `crates/finder/tests/support` (`crates/finder/tests/layout/main.rs:127`).
- **031's Imaging page tests, and those 032 to 036 add.** With a `ready` view and no run
  the block adds one region, two number fields, one checkbox and one button, and no
  `status` or `alert`. The probe build confirms that the earlier page tests find their
  own controls by name (§ Risks).
- **The binary's start-up lines and flags.** Nothing new is printed and no flag is
  added.

## Decisions and the alternatives rejected

| Decision | Rejected | Why |
|---|---|---|
| The run points by asking Track, and re-centres by Track off and Track on | Calling `GoTo::start` from the run; feeding the tracker an offset; a small axis move | GoTo refuses while the mount tracks (`crates/finder/src/goto.rs:739`); Track already decides whether a GoTo is needed and carries every guard. An axis move is a second way to move the mount, which the plan document forbids (rules 1 and 5). |
| A re-centre turns Track off, then waits for the finder camera's next live pointing, for at most 5 s, before it asks Track again | Asking Track at once after Track off; waiting with Track still on; a count of the finder camera's frames | While Track is on 032 holds a newly measured aim point back and applies it at the first live solve the session records with Track off, before that solve's pointing is taken (032's FR-036; spec owner decision 26). The pointing the session holds was taken at the aim point of its own moment (`crates/finder/src/session.rs:1192`): asked at once, Track's 1′ test would read one taken at the old aim point, start no GoTo, and a try would be spent. A wait made with Track still on would see no pointing at the new aim point at all. The bound is in seconds because a count of star frames has no end on a finder camera that has stopped solving, and 5 s is the interval within which 015's tracker already expects a live pointing (`crates/finder/src/tracking.rs:248`). |
| Stop is told from the run's own Track off by GoTo's stop generation | Reading `track.ended_reason` | The run's own `stop_tracking` leaves `stopped` there too. Every real Stop goes through `GoTo::stop`, which advances the generation first. |
| The mount part asks for Stop again when the generation changed across its restart, and compares the generation before it reads Track's answer | Trusting Track's own check; reading Track's error first | A Stop that lands between Track off and Track's start is seen by neither. One that lands during Track's start makes Track answer `stopped` (`crates/finder/src/session.rs:854`) or GoTo answer `the observer stopped it` (`crates/finder/src/goto.rs:626`): read first, that is a `mount_failed` after the observer pressed Stop. |
| The mount part names only `armed` and `off` of Track's states, and takes `steady` from the tracker | Listing `tracking` and `keyhole_wait` | The keyhole is one mount's way of being unsteady; a later tracker has others (rule 1). |
| `run.rs` with three child modules | One file, as the plan document first named it (it now names the three children too) | Spec owner decision 25: the machine in `run.rs` then imports nothing of the session, the mount or the camera, which is what keeps it mount-neutral and testable on the script alone, and each story's diff stays small. |
| Five small traits and a struct of them | One `Steps` trait | US4, US6 and US8 each add real parts; a test of one story uses that real part with the script for the rest. One trait would make every story implement all of it. |
| The traits and the script are complete from US1, and the machine grows by story | The whole machine in US1; a trait or a script that grows | US1's test implements nothing itself, so later stories never touch it; a machine written whole would put the retries, the focus pause, the centring loop and the limits in one story with no real part to prove them against, well over 64 KiB. |
| A scripted stand-in in the library | A copy in each of nine test modules; a new support file | Spec owner decision 16. |
| The lease is two calls of the camera part, `hold` and `release` | A sixth part; a lock in the `Runner` | The plan document's amendment 1: one lease, 031's, and no lock of the run's own. The camera part is where the run meets 031. |
| The lease is taken before Track is asked | After the mount has arrived | A run that cannot expose should not move the telescope. |
| The run holds the lease while it slews, exposes, solves and centres, and hands it over to the focus check and the stack | The run holding it for its whole length; the run taking none | Spec owner decision 15: 033's `focus_start` and 035's `Stacker::start` take the lease themselves, under their own names, so a run that kept it would refuse its own steps. Taking none would let a single imaging frame or a set of darks in while the run slews, solves or re-centres. |
| `camera.release()` is called at every ending whether or not the lease is held | Tracking in the machine whether it is held | One rule a test can read off the call list: the last call of every run gives the camera back. |
| A first imaging frame, then 033's own focus check, then an imaging frame for the solve | Measuring focus on the first imaging frame; no first imaging frame | Spec owner decision 20: the plan document's order, with 033's check as its draft gives it. `Imaging::solve` solves the latest imaging frame, which after a check is one of the check's. |
| The run's view rides in the imaging view, in every state | A top-level `run` in the session; only while `ready` | The plan document puts `run` in `imaging`. A run that ended because the link was lost must still be readable; 035's `stack` does the same. |
| The run's record lies in the stack's folder, and a run that never stacks makes its own | The run makes the folder and tells the stack to save there | 035's `Stacker::start` names its folder and takes none (spec owner decision 23). One folder then holds the stack and what the run did. |
| `run.json` rewritten whole at each change of state | JSON Lines of events | It is small, changes about ten times, and one whole file is what the field note copies. |
| Subs that are not kept are removed when the stack has ended, from `stack.json` | 035 removing each as it goes | Spec owner decision 22: 035's draft keeps every sub and lists it; removing by its list needs no change to 035. |
| An ending asks nothing of the mount | Stop at every ending | Spec owner decision 1: Track's own guards keep watching, and the target stays in view. |
| The offset is the separation of the solve's centre from the target | The target's pixel in the imaging frame | It needs no pixel, so it works when the target is outside the imaging frame, and it uses the same great-circle function as Track's 1′ test. |
| A region of interest refuses a start | Measuring at the sensor's centre through the solve's map | Spec owner decision 24: 032 takes aim readings from whole-sensor imaging frames only, so such a run could not re-centre. |
| The tolerance has a floor of 90″ | A share of the imaging frame alone | GoTo arrives at 60″ (`crates/finder/src/goto.rs:1013`); on a small or long-focus imaging frame a share alone would ask for less than GoTo can give and never be met. |
| A solve that did not move the aim point is retried without a re-centre, and whether it moved is read from the session, an aim point it holds back counting as moved | Re-centre anyway; deciding by 032's outcome word | The aim point in force did not change and none is held back, so Track off and on would bring the mount where it already is, and the restart would spoil the next aim reading. 032's outcome `measured` does not say the aim point moved: 032 leaves an aim point alone when its median is within 10″ of it (032 plan.md § The relation). |
| `repoint`, `stack.start`, `stack.stop` and the aid's open and close are awaited unguarded | Guarded like the other steps; the runner's stop signal handed to `repoint` | Dropping one half-way would leave Track off, or a stack or a focus aid running, with the run already ended. A stop of the run therefore waits for them: for a re-centre at most 5 s, one Track off and one Track on, after which Track is on, as after any stop of the run (spec FR-005, owner decision 14). Handing the signal in would add a parameter to a trait for a wait that short. |
| Durations are measured on tokio's clock, stamps on the console's | Both on the console's clock | The tracker does the same (`crates/finder/src/tracking.rs:4`); `FixedClock` in a scripted test never moves. |
| The run waits for a steady tracker without a time limit | A timeout | Rule 6 and rule 1: how long a tracker is unsteady is the mount's affair, a meridian flip among them; the observer can always stop. |
| Save failures are one ending | A free-space check | Spec owner decision 7: the words reach the run as text, and a new dependency is the owner's to approve. |
| `start_imaging_run` alone is rate-limited and logged | All four | Spec owner decision 19: it is the one that moves the telescope. |
| `continue_imaging_run` parses its own arguments | `Parameters<T>` | A missing key would then be a protocol error; the route's 400 has words, and the tool should too. |
| No Playwright check | An `imaging-run.spec.ts` | Spec owner decision 17. |
| The camera part asks for the lease before it looks for an imaging frame in flight | The in-flight check in `ready`, before the lease | The focus aid and the stack expose almost without a gap, so an in-flight check first answers `an exposure is in progress` where the lease would have named the holder (US6-S7). |
| `run::label` calls 035's `crate::imaging::stack::service::label` | A copy of 035's rule in `run.rs` | The run's id and the stack's folder must end alike; one function cannot drift from itself, and the machine still reads no session (spec § Clarifications, the audit). |
| The run's own imaging frames are taken at the exposure setting, uncapped, a known limit | The camera part asking 036's steering for the capped exposure before `hold` answers | 036 offers no getter for the capped exposure outside a stack's `before_exposure`, and adding one is an edit to 036. The smear falls on the corners and not on the solve's centre, which is all the run reads (`docs/research/2026-11-15-imaging-camera.md` § 7. Field rotation, sub length and the drive); the README and the field checklist say so (spec § Clarifications). |
| The script's `set_stack` sets the counts and the tracker's pause; `end_stack` ends the stack | One call that replaces the whole `StackProgress` | A test of US2 or US3 that set counts with `running` at its default would be ended by US7's rule for a stack that no longer runs, in a file US7 may not edit. |
| A dropped `Runner` aborts its task | Leaving the task to the runtime; a weak reference to the state in each part | The console's parts hold clones of the console's state, the owner of Track among them. 015 lets Track go when the console is dropped (`crates/finder/tests/motion/tracking/routes.rs:456`); a run must not keep it. One `Drop` does that; weak references would put an upgrade in every call of every part. |
| The time limit counts the clock from the start of stacking, pauses included | Counting only the time the stack is not paused | Its job is to end a forgotten run (spec owner decision 8). An unsteady tracker has no timeout of its own, and spec § Edge Cases tells the two apart. |
| Eleven stories: the machine, the centring and the stacking each in two | Eight stories, split after the probe if one passes 64 KiB | A split after dispatch is a changed spec and a relaunch (§ Plan traps 5). US1 and US2 are cut where every step answers against where one fails; US4 and US5 where the run points against where it points again; US7 and US8 where the script suffices against where the simulator is needed. |
| The console's own mount, camera and solver parts land one story before the loop that re-centres | The loop first, on the script alone; both in one story | US4 proves the parts on the moving mount with runs that need no re-centre and with the mount part alone (US4-S5). US5 then adds the loop, and the wait a re-centre needs, and proves a re-centre end to end. In one story the two passed 64 KiB by the review's estimate; the loop first would have left a story of rig tests with no product code. |
| The README is written by one story, the last | Rows with the routes and tools with the MCP story | Every README line is then asserted by the story that writes it (US11-S3). |
| The `DESIGN.md` entry and the `design.json` component are asserted by a Vitest test | Unasserted, as the page stories of 030 to 036 leave theirs | `readWebFile` reaches the repository's root from the page's tests (`crates/finder/web/src/test/files.ts:11`), so the story that writes the entry can prove it (US9-S7). |

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

## Prerequisites before this spec is flipped to `ready`

1. **The earlier specs have landed.** Met for `014-goto`, `015-tracking`, `016-mcp` and
   `017-console-stop-confirm`. Not met for `030-imaging-camera-link`, `031-imaging-frames-on-console`,
   `032-imaging-solve-and-aim-point`, `033-focus-reading-and-aid`,
   `034-dark-and-flat-frames`, `035-live-stack`, `036-tracking-from-subs` and
   `038-track-follows-a-push`, all of them drafts in the drafting worktree. 038 can land
   before 030: it changes only 015's Track. The `pushed_away` ending reads its `push`. 032 itself waits on the constitution's reading
   of principle V (the plan document, § Before any flip, 1).
2. **This trio is re-anchored to its seven predecessors after they land.** Not met. Every
   name in § What 030 to 036 offer is re-read on `main`, and the last story of each
   spec that writes a shared file replaces the one named in § Refinement Record. 030, 031,
   032 and 034 were reviewed and fixed before this text was written; 033, 035 and 036
   were drafts then, and have since been reviewed too. Where one of them lands
   offering something else than the table says, the part that calls it changes, and where it offers nothing that a part needs, the
   gap goes to the owner before the flip. Seven points are known to need that reading,
   and an eighth is closed:
   - 033 writes its focus check as taking its own exposures and restoring the settings
     afterwards; the run's first imaging frame and the imaging frame for the solve rest
     on that (spec owner decision 20);
   - 035's `Stacker::start` names its own folder, keeps every sub, and has no limit;
     the record's place and the removal of subs rest on that (spec owner decisions 22
     and 23);
   - 035's view ends with `stopped` or `camera` and a `save_error`; if it gains other
     reasons, `StackProgress::failed` sorts them;
   - 035's `Stacker::stop` is safe on a stack that has already ended: the run calls it
     at every ending after a start, whoever ended the stack (§ The machine, Ending);
   - closed: the rule for a folder's name is written once. 035 offers it as the pure
     function `crate::imaging::stack::service::label`, which its `AppState::target_label`
     calls, and `run::label` calls it too (§ The machine); US1-S1 holds it to the worked
     example `NGC 1976` → `NGC-1976`. The re-anchoring re-reads only the function's path;
   - 036's `Steering::waiting` is true only inside 035's loop; `paused_by_tracker`
     rests on it;
   - 036's `Tracker::steady` is false while a tracker settles after a start; the run's
     first exposure waits on it;
   - 032's task sets a measured aim point in the session before it shows a solve as
     `solved`, and sets it only when `should_apply` says so; while Track is on it
     holds it back instead, `Session::held_aim` answers it, and the first live solve
     the session records with Track off applies it before that pointing is taken
     (032's FR-036). `aim_moved` and the wait of `repoint`'s step 3 rest on all of it
     (§ The console's camera and solver parts, § The console's mount part).
3. **The lease of the imaging camera exists.** Met in the drafts, by the audit round in
   which this trio was revised. The plan document's amendment 1 gives 031's imaging
   state one lease that names its holder, and says each later spec takes and gives it
   back and adds no lock of its own. 031's trio gains it as its US6 (the audit's
   finding A1 on 031), and 034 takes it for a set of darks or flats in place of a check
   of its own (the audit's finding A1 on 034), as 033 and 035 do for the focus aid and
   the stack; 033's plan already words the refusal as this trio does. 031's plan.md
   § The camera lease now writes US6 out, and the five things below hold there; this
   trio is read again against it when 031 lands, for the same five
   things: the names `Imaging::lease`, `Lease` and `Lease::take`; that a refusal is a
   409 whose words are `the imaging camera is held by {holder}`; that `Lease::take`
   borrows the lease, so that the kept slot and a take in flight can share one
   `Arc<Lease>`; that
   `Imaging::abort`, `ended`, `view` and `solve` stay open to anyone; and that 033's
   check and aid and 035's stack take the lease themselves, as `the focus aid` and
   `the stack`, which is what the hand-over of spec owner decision 15 rests on. This
   trio does not need 031 to put its lease check before its own check for an imaging
   frame in flight: the camera part asks for the lease first (§ The console's camera and
   solver parts). US1-S2, US4-S2, US4-S4, US6-S7 and US8-S2 are the scenarios that rest
   on the lease. Two state its refusal word for word:
   `the imaging camera is held by the imaging run` (US4-S4) and
   `the imaging camera is held by the focus aid` (US6-S7).
4. **The owner decisions are settled and the approvals given** (spec § Owner decisions,
   § Owner approvals). Not met: none of the first twenty-five is settled. Decision 26,
   the tracker and a newly measured aim point, is not this trio's: it cites 032's owner
   decision 25, which the owner settles in 032's trio, because the behaviour is
   reachable from 032 on, with or without a run (§ The console's mount part, "The
   tracker is handed no step"). Met for decision 26: the owner settled 032's decision
   25 on 2026-11-17 with the hold, which 032 writes as its FR-036 and US4-S8, and this
   trio is written for it. Every aim reading of a run is taken while Track is on and
   held back, so FR-022's test counts a held aim point as moved and FR-025's wait comes
   after the re-centre's Track off, whose first live pointing puts it in force; US5-S4,
   SC-004 and the whole runs of US8-S1 and US8-S2 rest on both. This trio is
   re-validated with 032's before the flip.
5. **A probe build of every story.** Not met. No story has been compiled or run
   (§ Refinement Record). The probe settles, at the least:
   - each story's diff against 64 KiB, US1, US4 and US8 first (spec SC-007);
   - whether US4-S1 leaves the target inside 90″ with the true aim point set, and
     whether its solved offset agrees with the truth within 30″;
   - whether US5-S4 centres within its bound on the simulator and how many tries it
     takes; that the simulator's mount does not move toward the newly measured aim
     point between the first imaging solve and the re-centre's Track off (032's owner
     decision 25, which spec owner decision 26 cites); and that the first live
     pointing after that Track off comes within the 5 s of `repoint`'s step 3;
   - whether the simulator's mount is at rest by the time a re-centre's Track asks
     GoTo, or GoTo refuses with `the mount is already moving` (§ Risks);
   - the timing of US6-S4 and US7-S2 against the machine's poll, and of US3-S6's two
     runs inside one second;
   - that `Imaging` builds its view in one place, so that `run` can be set in every
     state, and that no part is reached under the runner's lock (§ Plan traps 23);
   - that a console dropped while a run is stacking on the run rig lets the session go
     and brings the simulated mount to rest. 015's own test of that
     (`crates/finder/tests/motion/tracking/routes.rs:456`) builds no runner and passes
     whatever this spec does, so the probe drops a rig in the middle of a run by hand.
     US1-S6 proves the runner's half on the script; whether the imaging state of 030 to
     036 lets go as well is theirs to show;
   - that the run rig syncs, slews and solves at dawn for US4-S4's Sun scenario;
   - that US8-S1 reaches three stacked subs on the simulator with 036's steering in the
     loop;
   - and that the earlier page tests still find their controls.
6. **The plan document, ADR-0002, the glossary's imaging terms and the research note
   are on `main`.** Not met: they are uncommitted in the drafting worktree.
7. **`main` is still as read.** Met on 2026-11-15: every `path:line` here was read at
   49bb210, and read again at 49bb210 after the audit, when `main` had not moved; no
   cited line had moved, and `crates/finder/src/targets.rs:39` replaced a citation one
   line above the `struct` it names. It is re-read at the flip, because 030 to 036
   change most of the files cited.
8. **The real mount.** Not met, and outside the factory: a GoTo to a target and Track
   have not run on the real mount (the plan document, § Before any flip, 4). The spec
   can be built before that; the run is not used at the telescope until it is.
9. **No operator-staged files are needed.** Met.

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
   `specs/037-imaging-run/` or `.specify/`.
6. **An oracle copied from the code proves nothing.** *Check:* a scripted test compares
   with the words, reasons and call lists spec.md states, typed into the test; a rig
   test compares with the simulator's truth, the session and the answers of 031's and
   033's own routes; `run.json` and `stack.json` are parsed by the test, never through
   `Store`; and no test builds an expected message by calling `Ending::message`.
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
10. **The view outside `ready`.** 031's view holds only its `state` then. *Check:* `run`
    is set where the view is built, after the state's own parts, in every state;
    US3-S5 reads it under `no_link`.
11. **Paused time and the blocking pool.** Tokio's paused clock runs on while a blocking
    task works. *Check:* `Store`'s writes and the stack part's removals are synchronous,
    and no file of this spec calls `spawn_blocking`.
12. **A held step must be listed first.** A test that holds `take` and then reads
    `calls()` expects `camera.take` there. *Check:* the script lists a call when it is
    made and holds after.
13. **The rate limit in tool tests.** Two `start_imaging_run` calls in one virtual
    second are both counted, refused or not. *Check:* the tests of US10 leave 1 s
    between starts, except US10-S4.
14. **`useImaging` reads every second.** A draft typed into a field would be lost on
    each read. *Check:* the block keeps its fields in its own state; US9-S1 types, then
    clicks.
15. **The bundle must be rebuilt.** *Check:* US9 commits `crates/finder/web/dist` after
    `npm run build`.
16. **027's layout.** *Check:* each new test is a module file in its binary's folder
    with one alphabetical `mod` line, taking its folder from `crate::support::Scratch`,
    and holding only the setup its own tests use.
17. **Each worktree builds on its own.** *Check:* no story sets `CARGO_TARGET_DIR`.
18. **Mount-neutral code.** *Check:* `crates/finder/src/imaging/run.rs`, `script.rs` and
    `store.rs` import nothing of `crate::goto`, `crate::tracking`, `crate::session` but
    `Clock`, `crate::ephemeris`, `crate::model` or `scope_link`, and of 035 `run.rs`
    calls only `crate::imaging::stack::service::label`; `console.rs` imports
    `crate::model::separation` and no other item of those, reaches Track and Stop
    through `crate::api` and `Session::stop_tracking` only, reads the session through
    `Session::snapshot` and `Session::steered`, and names no state of Track but `armed`
    and `off`. In the four files no identifier (a constant, a field, a variable, a
    function or a type), no import and no arithmetic is about an axis, an altitude, an
    azimuth, a rate or axis counts, and no number is a focal length, a field size or a
    scale. The rule is about code, and a reviewer checks imports and identifiers, not
    words. What an observer reads and what a doc comment says are exempt:
    `Ending::message` writes `Ended: {name} is below the minimum altitude.`, the module
    doc of `console.rs` says that nothing there knows an axis, and `SubCounts` counts
    subs.
19. **A cycle of `Arc`s, and a state kept alive.** The runner holds the parts, the parts
    hold a state, and the state holds the runner; and a task that held the parts for
    ever would hold the owner of Track for ever. *Check:* `with_imaging_run` clones the
    state for the parts before the runner is kept, and `Imaging` holds the runner
    weakly; the run's task holds the runner's inner `Arc` and never the `Runner`, and
    `Runner`'s `Drop` aborts the task (US1-S6).
20. **The lease must come back, and must not be kept across a hand-over.** A run that
    ends by any path, a refused `point` included, would otherwise lock the imaging
    camera until the console restarts; a run that still held the lease when it called
    033's check or 035's stack would refuse its own step. *Check:* `camera.release()`
    is the last call of every ending and of a refused `point`, and comes before every
    `focus.check` and `stack.start`; `camera.hold()` comes before the first take after
    the focus gate. US1-S1, US1-S2, US1-S4 and US1-S5 assert it on the script, and
    US4-S2, US4-S4 and US8-S2 through 031's own route.
21. **Nothing waits a fixed time for an exposure.** *Check:* the only durations in the
    four files are `POLL`, `FOCUS_PAUSE_LIMIT` and the request's `duration_s`.
    `FRESH_POLLS` times `POLL` is 5 s: a bound on the finder camera's next live
    pointing, said as such, and on nothing of the imaging camera. Every other bound is
    a count of imaging frames, solves, tries or subs.
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
    lets it go, calls the part, and locks again to record the answer (§ The machine).
24. **A later rule must not break an earlier story's test.** A story may not edit a test
    file an earlier story created. *Check:* the script's `set_stack` leaves `running`
    and `failed` alone and only `end_stack` ends a stack (§ The script); US4-S1 asserts
    its solved offset inside the tolerance, so the loop of US5 does not enter it; each
    story's `preserved_behavior` in § Refinement Record names what of the earlier
    stories it leaves passing; and a story that finds an earlier test failing
    escalates.

## Risks

| Risk | Mitigation |
|---|---|
| 033, 035 or 036 lands offering something other than its draft. | The run touches them in three small parts and nowhere else; the states, routes, page and tools do not move. § Prerequisites 2 re-anchors, and a gap goes to the owner. |
| 031 lands the lease under other names, words or rules than its US6 gives. | § Prerequisites 3 lists the five points. Only the camera part's `hold`, `release` and `take` touch it, and five scenarios, two of which state its refusal word for word. |
| Another taker gets the lease in the instant of a hand-over. | The run's next step is refused with that holder's name and the run says so: a pause, `stack_failed` or `camera_failed` (spec § Edge Cases, US1-S2, US2-S4). Spec owner decision 15 names the other answer, a change to 033 and 035. |
| The simulator leaves the target just outside 90″, with the true aim point set or after a re-centre, or needs more than three tries. | Unprobed (§ Prerequisites 5). US4-S1 and US5-S4 assert the bounds against the truth, so a miss is seen in the probe of the story that owns the test, and owner decisions 2 and 3 are where the numbers change. |
| A story still passes 64 KiB. | Unmeasured (§ Prerequisites 5, spec SC-007). Three stories were each cut in two on the review's estimate, and the script is in the library so that nine test modules do not each carry one. US1 is the largest: the machine, the whole script and six scenarios. If the probe shows it over, its scenario of the mount's endings (US1-S5) moves to US2 at the re-anchoring, a move of tests only. US4 is next: if it is over, its scenario of the guards (US4-S4) becomes a story of its own. |
| A re-centre interrupts tracking for a second or two, so the next aim reading reads `moving`. | 032 takes no aim reading then and keeps the one in force; the run still solves and compares, and asks for no further re-centre from a solve that did not move the aim point. Seen in the probe of US5-S4. |
| 015's tracker chases a newly measured aim point at a rate nothing bounds, outside GoTo's checks. | Closed in 032, where the seam between 032 and 015 lies: the owner settled 032's owner decision 25 on 2026-11-17 with the hold (032's FR-036, US4-S8), which spec owner decision 26 cites, and FR-022 and FR-025 here are written for it (§ Prerequisites 4). The probe of US5-S4 confirms that the simulator's mount does not move toward the new aim point before the re-centre's Track off (§ Prerequisites 5), and the field checklist has the observer watch the first centring (step 8). |
| The observer's exposure setting is longer than the tracker's limit, and the run's own imaging frames, which 036 does not cap, trail their corner stars. | A known limit (spec § Clarifications; § The console's camera and solver parts). The solve's centre, all the run reads, is not moved; a solve that fails for want of stars is counted (US2-S2). The README says so and step 8 of the field checklist has the observer look. The other answer is a getter in 036 for the capped exposure, an edit to 036. |
| A re-centre asks Track again while the mount still reports motion after Track off; GoTo refuses with `the mount is already moving` and the run ends `mount_failed`. | Unprobed (§ Prerequisites 5). `stop_tracking` waits for the mount's acknowledgement of the stop, not for rest (`crates/finder/src/tracking.rs:219`). `repoint`'s step 3 now lies between Track off and Track's start, which gives the mount until the next live pointing, but it is not a wait for rest. If the probe or the telescope shows it, `repoint` waits, for a counted number of polls, until the session's mount view no longer says the mount is moving (`crates/finder/src/session.rs:508`) before it asks Track; that reads no axis, and is written in at the re-anchoring. |
| A console dropped in the middle of a run keeps Track on through something the imaging state of 030 to 036 holds. | US1-S6 proves that the runner lets its parts go; the probe drops a rig in the middle of a run (§ Prerequisites 5). In the binary the state lives until exit, so only tests are touched. |
| A stop of the run waits up to about 5 s when it arrives during a re-centre. | Said in spec FR-005 and owner decision 14, and asserted on the script (US5-S3). The console's Stop does not wait: it stops the mount at once, and the run then ends `stopped`. |
| The first imaging frame costs an exposure and a transfer the run does not use again. | Spec owner decision 20 names the two other answers. The first imaging frame is what tells a camera fault from poor focus. |
| Track left on after an ending is forgotten. | Track's own guards end it at the minimum altitude and at sunrise, and the top bar's Stop stays visible while Track is on (`DESIGN.md:308`). Owner decision 1 is where it changes. |
| The kept subs fill the disk. | The default time limit (owner decision 8) and `save_failed`. |
| Subs that are not kept still fill the disk while the run stacks. | Spec owner decision 22 names the other answer, an edit to 035. |
| A second client stops or restarts the stack under a run. | The lease refuses a second stack; a stop of the run's own stack from 035's route ends the run `stopped` (US7-S4). |
| An earlier page test queries a control the block also has. | The block's controls have their own names and it shows no `status` or `alert` before a run; the probe build of US9 runs every page test. |
| The real GoTo and Track are unproven. | § Prerequisites 8. The checklist of FR-041 starts with an imaging frame and a solve by hand before the first run. |

## Refinement Record

Drafted on 2026-11-15 by the grilling session against `main` at 49bb210, from
`docs/plans/2026-11-15-specs-030-037-imaging.md` with its § Mount-neutral by design and
its amendments, the reviewed trios of 030, 031, 032 and 034 and the drafts of 033, 035 and
036 in the same worktree, a read of `crates/finder/src/goto.rs`, `tracking.rs`,
`session.rs`, `api.rs` and the `mcp` module, the specs 014 to 017 and 025, and the
research on prior art with its verification. A first draft of this trio, written at
dbc2b5b before 033, 035 and 036 had drafts, was read whole and replaced.

A second agent then reviewed the trio against the plan document, the code at 49bb210
and the drafts of 030 to 036, and returned twenty-two findings: one blocker, six majors
and fifteen minors. Each was checked again against the same sources and all were
applied; spec.md's last session under Clarifications lists what a builder writes
differently because of them. The largest change is the Work Graph: eleven stories for
eight, with the machine, the centring and the stacking each cut in two, so every story
id, scenario id and FR number of the reviewed draft above US1-S5 and FR-006 has moved.

An audit then compared the seven trios the series then had, side by side, and gave this
one four
minor findings, all applied (spec.md § Clarifications, the last session): owner
decision 26 now cites 032's owner decision 25 (A1); `run::label` calls 035's
`crate::imaging::stack::service::label` (A2); the lease is 031's US6, its names no
longer marked as assumed, US4-S4 and US6-S7 state its refusal word for word, and owner
approval 2 names the purpose `run` and the holder's name `the imaging run` (A3); and the
run's own imaging frames, which 036 does not cap, are a known limit that the README and
the field checklist's step 8 state, with US11-S2 reading it (A4). No story id, scenario
id or FR number moved. Every `path:line` was read again at 49bb210, where `main` still
was: none had moved, and one was made exact (§ Prerequisites 7).

On 2026-11-17 the owner settled 032's owner decision 25, which owner decision 26 cites,
with the hold: FR-022, FR-024 and FR-025, `repoint`'s steps (its wait is now step 3,
after Track off) and `aim_moved` were rewritten for it (spec.md § Clarifications, the
last session). No story id, scenario id or FR number moved, and no scenario's text
changed.

**No probe build has been run.** No story of this trio has been compiled, linted or
tested, no interface above has been built, and no diff has been measured: both sessions
wrote documents only. Every `path:line` was read at 49bb210. Every number in a scenario
is either stated by the scenario as its own oracle or worked by hand: 1/20 of the
simulated imaging frame's 822 pixels at 2.1845″ a pixel is 89.8″, under the 90″ floor;
the centre (728, 544) of the simulated finder camera's frame is 127 pixels from the true
aim point (700, 420), about 3800″ at 29.78″ a pixel.

What a first refinement round must do is § Prerequisites 2, 3 and 5, and its findings
replace this paragraph. Until then the record below is the draft's own account: its
tests are the scenarios, its writes are the files § Project Structure names, and its
shared writes name the merge edge that orders two stories of this spec on one file, or
the story of 030 to 036 that last writes the file among the drafts as they stood on
2026-11-15, read from each draft's own record. Each of those names is checked again at
the re-anchoring. A test file an earlier story of this trio creates does not exist yet
and cannot be listed as an existing test; what a later story must leave passing in it
is under that story's `preserved_behavior`. The record's `open_questions` are empty
because the validator refuses a record that holds any; what is not established is in
§ Risks and § Prerequisites, and what the owner must settle is in spec.md § Owner
decisions, with the seam between 032 and 015 settled as 032's owner decision 25, which
decision 26 cites. Nothing in this record is an owner
approval: those are in spec.md, and flipping to `ready` gives them.

```yaml
version: 1
reviewed_revision: 49bb210fe4860dd98ba24727074e7bd3cc0d7124
stories:
  US1:
    tests:
      - path: crates/finder/tests/console/imaging_run.rs
        status: new
        disposition: add
        assertion: "a scripted run held at each step reads slewing, first_frame, focus_check, solving and stacking with their messages, its view carries the id, by, target, settings and no folder until the stack has started, the calls are the straight path in order with the lease given up before the focus check and before the stack and taken again between them, and label and run_id turn NGC 1976 into NGC-1976"
        decision: "spec.md:US1-S1"
      - path: crates/finder/tests/console/imaging_run.rs
        status: new
        disposition: add
        assertion: "a refusal of the camera, the solver, the target, the lease or Track stops the start at that part with 409 and its words, a refused lease is followed by neither Track nor a release, a refused Track gives the lease back, bad subs and duration_s are 400 with their words, a second start is refused while a run is on, a start is accepted after a stop, and a lease that cannot be taken back after the focus check ends camera_failed with the lease's words"
        decision: "spec.md:US1-S2"
      - path: crates/finder/tests/console/imaging_run.rs
        status: new
        disposition: add
        assertion: "a mount that tracks but is not steady shows tracking with no take and no focus check, and the run goes on once it is steady"
        decision: "spec.md:US1-S3"
      - path: crates/finder/tests/console/imaging_run.rs
        status: new
        disposition: add
        assertion: "a stop in each of five states ends the run as stopped with its message, aborts a take in flight, stops a focus check in flight, stops a started stack once and no other, gives the lease back as the last call, lists no call after the ending and never repoints"
        decision: "spec.md:US1-S4"
      - path: crates/finder/tests/console/imaging_run.rs
        status: new
        disposition: add
        assertion: "each of seven endings of the mount's motion while stacking stops the stack once, gives the lease back, and ends the run with its reason and exact message"
        decision: "spec.md:US1-S5"
      - path: crates/finder/tests/console/imaging_run.rs
        status: new
        disposition: add
        assertion: "dropping the only Arc of a runner whose run is stacking leaves the script with one owner and adds no call in the next 10 s"
        decision: "spec.md:US1-S6"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and each binary's main.rs holds only its support include and mod lines"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "no Cargo.toml, Cargo.lock or deny.toml changes and no unsafe is written (spec.md SC-003)"
      - "nothing under crates/finder/src/imaging/run.rs or run/script.rs imports goto, tracking, ephemeris, model or scope_link, holds an identifier or arithmetic for an axis, or holds a number sized to one telescope (plan.md Plan traps 18)"
      - "the imaging state of 030 to 036 and their tests are untouched but for one mod line"
      - "the rule for a folder's name is 035's crate::imaging::stack::service::label, called by run::label and not copied (plan.md The machine)"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console imaging_run::"
        - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - crates/finder/src/imaging/mod.rs
      - crates/finder/src/imaging/run.rs
      - crates/finder/src/imaging/run/script.rs
      - crates/finder/tests/console/imaging_run.rs
      - crates/finder/tests/console/main.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/imaging/mod.rs
        other_spec: 036-tracking-from-subs
        other_story: US4
        disposition: merge_order
        reason: "037 depends_on_landed 036-tracking-from-subs, whose US4 last writes this file among the drafts of 030 to 036 as they stood; named again at the re-anchoring"
      - path: crates/finder/src/imaging/mod.rs
        other_spec: 037-imaging-run
        other_story: US3
        disposition: merge_order
        reason: "US3 depends_on_merged US1, and adds run to the imaging view and attach_run"
      - path: crates/finder/src/imaging/run.rs
        other_spec: 037-imaging-run
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and replaces what a failed take, solve and stack start do with the table of failed steps, and reads the stack's progress"
      - path: crates/finder/tests/console/main.rs
        other_spec: 036-tracking-from-subs
        other_story: US8
        disposition: merge_order
        reason: "037 depends_on_landed 036-tracking-from-subs, whose US8 last writes this file among the drafts of 030 to 036 as they stood; named again at the re-anchoring"
      - path: crates/finder/tests/console/main.rs
        other_spec: 037-imaging-run
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and adds mod imaging_run_steps"
    open_questions: []
  US2:
    tests:
      - path: crates/finder/tests/console/imaging_run_steps.rs
        status: new
        disposition: add
        assertion: "two failed takes are retried, three in a row end camera_failed with the last error before any focus check, and a save failure ends save_failed at the first take"
        decision: "spec.md:US2-S1"
      - path: crates/finder/tests/console/imaging_run_steps.rs
        status: new
        disposition: add
        assertion: "a solve without a solution takes a new imaging frame and solves again with no second focus check, three in a row end solver_failing, and a solver that cannot run ends solver_unavailable at once"
        decision: "spec.md:US2-S2"
      - path: crates/finder/tests/console/imaging_run_steps.rs
        status: new
        disposition: add
        assertion: "while stacking the view mirrors the counts set_stack gave with its message, reads paused_by_tracker while the stack waits for the tracker and stacking again after, and a slightly soft verdict leaves its warning in arcseconds or pixels and goes on"
        decision: "spec.md:US2-S3"
      - path: crates/finder/tests/console/imaging_run_steps.rs
        status: new
        disposition: add
        assertion: "a stack that cannot start ends the run by its failure's kind without being stopped and with no folder, and a stack that does not end cleanly at an ending leaves a warning and the ending's reason"
        decision: "spec.md:US2-S4"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and each binary's main.rs holds only its support include and mod lines"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "every scenario of US1 still holds: none fails a step or sets a count, and what US2 replaces, a failed take, solve or stack start ending the run at once, is asserted by none (plan.md Shapes that existing tests pin)"
      - "no trait of a part gains a method, Steps gains no field, and the script is not written again"
      - "nothing under crates/finder/src/imaging/run.rs holds an identifier or arithmetic for an axis (plan.md Plan traps 18)"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console imaging_run_steps::"
        - "cargo nextest run -p finder --locked --test console imaging_run::"
        - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - crates/finder/src/imaging/run.rs
      - crates/finder/tests/console/imaging_run_steps.rs
      - crates/finder/tests/console/main.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/imaging/run.rs
        other_spec: 037-imaging-run
        other_story: US3
        disposition: merge_order
        reason: "US3 depends_on_merged US2, and adds with_folder, the events and the writes of run.json"
      - path: crates/finder/tests/console/main.rs
        other_spec: 037-imaging-run
        other_story: US3
        disposition: merge_order
        reason: "US3 depends_on_merged US2, and adds mod imaging_run_routes"
    open_questions: []
  US3:
    tests:
      - path: crates/finder/tests/console/imaging_run_routes.rs
        status: new
        disposition: add
        assertion: "POST /api/imaging/run answers 200 with the run in the imaging view and in the session, never in session.json, and once the run stacks run.json lies in the stack's folder with the format, version, id, by, target, times, settings, state and the events in order"
        decision: "spec.md:US3-S1"
      - path: crates/finder/tests/console/imaging_run_routes.rs
        status: new
        disposition: add
        assertion: "bad bodies answer 400, another content type 415, a foreign origin 403, a part's refusal and a second start 409 with their words, a console given no parts 409 on all three routes, and no refusal leaves a run or a folder"
        decision: "spec.md:US3-S2"
      - path: crates/finder/tests/console/imaging_run_routes.rs
        status: new
        disposition: add
        assertion: "DELETE ends the run as stopped and answers once it has ended, run.json then holds the ending, the counts set_stack gave and ended as its last event, a second DELETE changes nothing and DELETE with no run answers 200 with no run"
        decision: "spec.md:US3-S3"
      - path: crates/finder/tests/console/imaging_run_routes.rs
        status: new
        disposition: add
        assertion: "continue answers 409 with no imaging run is running or the imaging run is not paused, 400 for other bodies and 403 for a foreign origin"
        decision: "spec.md:US3-S4"
      - path: crates/finder/tests/console/imaging_run_routes.rs
        status: new
        disposition: add
        assertion: "an imaging view whose state is no_link still carries the ended run"
        decision: "spec.md:US3-S5"
      - path: crates/finder/tests/console/imaging_run_routes.rs
        status: new
        disposition: add
        assertion: "two runs started in one whole second that end at first_frame share an id, have no folder until their ending and then runs/<id> and runs/<id>-2, and a runs folder that cannot be written ends a stacking run save_failed with the stack stopped and the lease given back"
        decision: "spec.md:US3-S6"
      - path: crates/finder/tests/console/imaging_run_routes.rs
        status: new
        disposition: add
        assertion: "run.json holds no token, node address or site, and no message, elapsed_s or site key"
        decision: "spec.md:US3-S7"
      - path: crates/finder/tests/console/session/edits.rs
        status: existing
        disposition: preserve
        assertion: "GET /api/session bodies without a run are unchanged"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/mcp/endpoint.rs
        status: existing
        disposition: preserve
        assertion: "get_session over MCP still returns the session body unchanged when no run exists"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and each binary's main.rs holds only its support include and mod lines"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "GET /api/session and GET /api/imaging without a run are byte for byte unchanged"
      - "nothing about a run is persisted in session.json (spec.md FR-016)"
      - "start_motion, SessionWrite and sim_json keep their behaviour; the routes only call them"
      - "Runner::new still writes nothing, so every scenario of US1 and US2 holds"
      - "the README is not written: its rows are US11's (plan.md Decisions and the alternatives rejected)"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console imaging_run_routes::"
        - "cargo nextest run -p finder --locked --test console imaging_run::"
        - "cargo nextest run -p finder --locked --test console imaging_run_steps::"
        - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - crates/finder/src/api.rs
      - crates/finder/src/imaging/mod.rs
      - crates/finder/src/imaging/routes.rs
      - crates/finder/src/imaging/run.rs
      - crates/finder/src/imaging/run/store.rs
      - crates/finder/src/lib.rs
      - crates/finder/tests/console/imaging_run_routes.rs
      - crates/finder/tests/console/main.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/api.rs
        other_spec: 035-live-stack
        other_story: US8
        disposition: merge_order
        reason: "037 depends_on_landed 035-live-stack, whose US8 last writes this file among the drafts of 030 to 036 as they stood; named again at the re-anchoring"
      - path: crates/finder/src/api.rs
        other_spec: 037-imaging-run
        other_story: US8
        disposition: merge_order
        reason: "US8 depends_on_merged US3, and adds with_imaging_run"
      - path: crates/finder/src/imaging/routes.rs
        other_spec: 036-tracking-from-subs
        other_story: US5
        disposition: merge_order
        reason: "037 depends_on_landed 036-tracking-from-subs, whose US5 last writes this file among the drafts of 030 to 036 as they stood; named again at the re-anchoring"
      - path: crates/finder/src/imaging/run.rs
        other_spec: 037-imaging-run
        other_story: US4
        disposition: merge_order
        reason: "US4 depends_on_merged US3, and adds pub mod console"
      - path: crates/finder/src/lib.rs
        other_spec: 036-tracking-from-subs
        other_story: US5
        disposition: merge_order
        reason: "037 depends_on_landed 036-tracking-from-subs, whose US5 last writes this file among the drafts of 030 to 036 as they stood; named again at the re-anchoring"
      - path: crates/finder/tests/console/main.rs
        other_spec: 037-imaging-run
        other_story: US5
        disposition: merge_order
        reason: "US5 depends_on_merged US3, and adds mod imaging_run_centring"
    open_questions: []
  US4:
    tests:
      - path: crates/finder/tests/motion/imaging_run.rs
        status: new
        disposition: add
        assertion: "on the simulator with the true aim point set a run reaches stacking with no try, its solved offset inside the tolerance and within 30 arcseconds of the true offset, the true offset at most 110 arcseconds, Track never leaving tracking, and the motion record reading page and imaging_run"
        decision: "spec.md:US4-S1"
      - path: crates/finder/tests/motion/imaging_run.rs
        status: new
        disposition: add
        assertion: "a start is refused with no target is selected, with Track's own words for a target below the horizon and with the region-of-interest words, each leaving Track off, no run, no folder and the lease given back"
        decision: "spec.md:US4-S2"
      - path: crates/finder/tests/motion/imaging_run.rs
        status: new
        disposition: add
        assertion: "POST /api/stop while stacking ends the run as stopped with the stack stopped once and the simulated mount at rest for 10 s"
        decision: "spec.md:US4-S3"
      - path: crates/finder/tests/motion/imaging_run.rs
        status: new
        disposition: add
        assertion: "a raised minimum altitude ends the run too_low, an unresponsive node link_lost, a changed target target_changed with Track still on the old target, a push by hand pushed_away with Track following it, dawn sun_up, and an imaging frame asked for while the run holds the lease is refused with the imaging camera is held by the imaging run and accepted after the run"
        decision: "spec.md:US4-S4"
      - path: crates/finder/tests/motion/imaging_run.rs
        status: new
        disposition: add
        assertion: "the mount part alone points, reads tracking and steady, points again leaving the motion record as page and imaging_run, and after POST /api/stop reads ended stopped and refuses to point again with Track off and the simulated mount at rest for 10 s"
        decision: "spec.md:US4-S5"
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
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and each binary's main.rs holds only its support include and mod lines"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "Session::start_tracking, Session::stop_tracking, GoTo, Tracker and api::stop keep their signatures, words and behaviour (plan.md Shapes that existing tests pin)"
      - "the run sends no axis command and reads no axis position (spec.md FR-001)"
      - "no trait of a part gains a method and Steps gains no field"
      - "the lease is 031's US6: the camera part takes it, gives it back and adds no lock (plan.md Plan traps 20)"
      - "the run's own imaging frames are taken at the camera's settings as they stand: the camera part sets nothing and asks nothing of 036's steering (plan.md The console's camera and solver parts)"
      - "every scenario of US1 to US3 still holds: the machine changes by one mod line"
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
        other_story: US5
        disposition: merge_order
        reason: "US5 depends_on_merged US4, and adds the centring loop"
      - path: crates/finder/src/imaging/run/console.rs
        other_spec: 037-imaging-run
        other_story: US5
        disposition: merge_order
        reason: "US5 depends_on_merged US4, and adds FRESH_POLLS and the wait for a live pointing after Track off to repoint"
      - path: crates/finder/tests/motion/main.rs
        other_spec: 036-tracking-from-subs
        other_story: US7
        disposition: merge_order
        reason: "037 depends_on_landed 036-tracking-from-subs, whose US7 last writes this file among the drafts of 030 to 036 as they stood; named again at the re-anchoring"
      - path: crates/finder/tests/motion/main.rs
        other_spec: 037-imaging-run
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
        assertion: "four solves outside the tolerance repoint three times and end not_centred with the offset, with no stack started"
        decision: "spec.md:US5-S2"
      - path: crates/finder/tests/console/imaging_run_centring.rs
        status: new
        disposition: add
        assertion: "a solve that did not move the aim point is retried without a repoint and counts as a try, a failed repoint ends mount_failed or stopped by its failure, no take follows a repoint until the mount is steady, and a stop of the run during a repoint is answered only after it and ends the run stopped with no further take"
        decision: "spec.md:US5-S3"
      - path: crates/finder/tests/motion/imaging_run_centring.rs
        status: new
        disposition: add
        assertion: "on the simulator with no aim point a run reaches stacking after at least one try with the true offset within the tolerance plus 20 arcseconds and the session's aim point measured within 2.0 pixels of the true one"
        decision: "spec.md:US5-S4"
      - path: crates/finder/tests/motion/imaging_run_centring.rs
        status: new
        disposition: add
        assertion: "POST /api/stop while centring ends the run as stopped and not mount_failed, with no stack started and the simulated mount at rest for 10 s"
        decision: "spec.md:US5-S5"
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
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and each binary's main.rs holds only its support include and mod lines"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "every scenario of US1 to US3 still holds: the script's default offset of 0 takes no centring branch (plan.md Shapes that existing tests pin)"
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
        - "release owner: the repository owner, through the merge queue's ci.yml jobs, and the first centring at the telescope, watched for the tube staying still after a newly measured aim point until the re-centre's Track off (032's owner decision 25, which spec.md owner decision 26 cites)"
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
        other_spec: 037-imaging-run
        other_story: US6
        disposition: merge_order
        reason: "US6 depends_on_merged US5, and replaces the going-on of a verdict that pauses with the focus pause and makes resume work"
      - path: crates/finder/src/imaging/run/console.rs
        other_spec: 037-imaging-run
        other_story: US6
        disposition: merge_order
        reason: "US6 depends_on_merged US5, and adds the focus part"
      - path: crates/finder/tests/console/main.rs
        other_spec: 037-imaging-run
        other_story: US6
        disposition: merge_order
        reason: "US6 depends_on_merged US5, and adds mod imaging_run_focus"
      - path: crates/finder/tests/motion/main.rs
        other_spec: 037-imaging-run
        other_story: US8
        disposition: merge_order
        reason: "US8 depends_on_merged US5, and adds mod imaging_run_stack"
    open_questions: []
  US6:
    tests:
      - path: crates/finder/tests/console/imaging_run_focus.rs
        status: new
        disposition: add
        assertion: "a clearly out verdict pauses the run with its message, focus note and the focus aid opened and no solve, and clearly out without a record, unmeasurable, no record and a check that did not finish each pause with their own words"
        decision: "spec.md:US6-S1"
      - path: crates/finder/tests/console/imaging_run_focus.rs
        status: new
        disposition: add
        assertion: "resume(false) closes the aid and checks focus again, taking the lease back and going on to the solve when it is good and pausing again with the aid reopened when it is still out"
        decision: "spec.md:US6-S2"
      - path: crates/finder/tests/console/imaging_run_focus.rs
        status: new
        disposition: add
        assertion: "resume(true) closes the aid, takes the lease back and goes on to the solve with no second check and the go-anyway warning"
        decision: "spec.md:US6-S3"
      - path: crates/finder/tests/console/imaging_run_focus.rs
        status: new
        disposition: add
        assertion: "a pause is still on at 899 s and ends focus_timeout by 901 s with the aid closed and no stack started"
        decision: "spec.md:US6-S4"
      - path: crates/finder/tests/console/imaging_run_focus.rs
        status: new
        disposition: add
        assertion: "a stop and a mount ending from a pause close the aid before the lease is given back, an aid that cannot open leaves the run paused with a warning, and resume outside a pause is refused"
        decision: "spec.md:US6-S5"
      - path: crates/finder/tests/console/imaging_run_focus.rs
        status: new
        disposition: add
        assertion: "POST /api/imaging/run/continue with anyway false or true takes a paused run on"
        decision: "spec.md:US6-S6"
      - path: crates/finder/tests/console/imaging_run_focus.rs
        status: new
        disposition: add
        assertion: "on the simulator the console's own focus part passes a run at the focus error 0.0, pauses one that 033's own check calls clearly out with the focus aid running, goes on after refocusing, refuses a start with the imaging camera is held by the focus aid while the aid already runs, and pauses with no_record on a fresh folder"
        decision: "spec.md:US6-S7"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and each binary's main.rs holds only its support include and mod lines"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "033's verdict, thresholds, focus record and focus aid are unchanged; the part only calls them"
      - "Track stays on through a focus pause (spec.md Edge Cases)"
      - "no trait of a part gains a method and Steps gains no field"
      - "every scenario of US1 to US5 still holds: none gives a verdict that pauses, and US3's continue refusals outside a pause are unchanged"
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
        other_spec: 037-imaging-run
        other_story: US7
        disposition: merge_order
        reason: "US7 depends_on_merged US6, and adds the limits, the drop run and the stack's own endings"
      - path: crates/finder/src/imaging/run/console.rs
        other_spec: 037-imaging-run
        other_story: US8
        disposition: merge_order
        reason: "US8 depends_on_merged US6, and adds the stack part"
      - path: crates/finder/tests/console/main.rs
        other_spec: 037-imaging-run
        other_story: US7
        disposition: merge_order
        reason: "US7 depends_on_merged US6, and adds mod imaging_run_limits"
    open_questions: []
  US7:
    tests:
      - path: crates/finder/tests/console/imaging_run_limits.rs
        status: new
        disposition: add
        assertion: "a run asked for 3 subs keeps stacking at 2 and ends count_limit at 3 with the stack stopped once, keeping subs or discarding them as the request said"
        decision: "spec.md:US7-S1"
      - path: crates/finder/tests/console/imaging_run_limits.rs
        status: new
        disposition: add
        assertion: "a 60 s limit counts from the start of stacking, not of the run: still stacking 59 s in, ended duration_limit by 62 s, and the same when the tracker pauses the stack throughout"
        decision: "spec.md:US7-S2"
      - path: crates/finder/tests/console/imaging_run_limits.rs
        status: new
        disposition: add
        assertion: "ten drops with none stacked between them end subs_failing naming each reason's rise, and a stacked sub between nine and nine does not"
        decision: "spec.md:US7-S3"
      - path: crates/finder/tests/console/imaging_run_limits.rs
        status: new
        disposition: add
        assertion: "each kind of stack failure ends the run with its own reason, an unknown one as stack_failed with its words, a stack that ends with no failure as stopped, and each still stops the stack once"
        decision: "spec.md:US7-S4"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and each binary's main.rs holds only its support include and mod lines"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "every scenario of US1 to US6 still holds: the default request has no count and 7200 s, and the script's stack runs until it is stopped or end_stack is called, whatever set_stack gave it (plan.md The script, Plan traps 24)"
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
    shared_writes:
      - path: crates/finder/tests/console/main.rs
        other_spec: 037-imaging-run
        other_story: US11
        disposition: merge_order
        reason: "US11 depends_on_merged US7, and adds mod imaging_run_docs"
    open_questions: []
  US8:
    tests:
      - path: crates/finder/tests/motion/imaging_run_stack.rs
        status: new
        disposition: add
        assertion: "on the simulator a run of the console's own five parts asked for 3 subs ends count_limit with run.json beside stack.json, stack.fits and stack.png in the stack's folder, whose name ends with the target's label, at least three kept subs on disk, and Track still tracking"
        decision: "spec.md:US8-S1"
      - path: crates/finder/tests/motion/imaging_run_stack.rs
        status: new
        disposition: add
        assertion: "keep_subs false leaves no listed sub or solve sidecar on disk, and DELETE while stacking ends stopped with the stack saved, 035's stack no longer running, Track still tracking and the lease given back"
        decision: "spec.md:US8-S2"
      - path: crates/finder/tests/cli/imaging_run.rs
        status: new
        disposition: add
        assertion: "the built binary under --simulate, once its imaging camera is ready, answers a start with no target with 409 no target is selected, so it runs imaging runs"
        decision: "spec.md:US8-S3"
      - path: crates/finder/tests/cli/simulate.rs
        status: existing
        disposition: preserve
        assertion: "the binary's refusals and address line are unchanged"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and each binary's main.rs holds only its support include and mod lines"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "035's stack and 036's steering are unchanged; the part only calls them"
      - "no ending but a guard's or Stop's changes what Track is doing (spec.md SC-005)"
      - "no flag and no start-up line is added to the binary"
      - "every scenario of US1 to US7 still holds: the machine is not written, and the parts of US4 to US6 are not changed"
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
        other_story: US10
        disposition: merge_order
        reason: "US10 depends_on_merged US8, and adds the tool's name to start_motion's refusal"
      - path: crates/finder/src/main.rs
        other_spec: 033-focus-reading-and-aid
        other_story: US8
        disposition: merge_order
        reason: "037 depends_on_landed 033-focus-reading-and-aid, whose US8 last writes this file among the drafts of 030 to 036 as they stood; named again at the re-anchoring"
      - path: crates/finder/tests/cli/main.rs
        other_spec: 035-live-stack
        other_story: US8
        disposition: merge_order
        reason: "037 depends_on_landed 035-live-stack, whose US8 last writes this file among the drafts of 030 to 036 as they stood; named again at the re-anchoring"
    open_questions: []
  US9:
    tests:
      - path: crates/finder/web/src/components/ImagingRunBlock.test.tsx
        status: new
        disposition: add
        assertion: "with no run the block offers Start imaging run, two empty limit fields and a checked Keep every sub with no status or alert, posts the request it was given, and shows a refusal as an alert"
        decision: "spec.md:US9-S1"
      - path: crates/finder/web/src/components/ImagingRunBlock.test.tsx
        status: new
        disposition: add
        assertion: "a stacking run reads its message, counts by reason and elapsed time, offers Stop imaging run which sends DELETE, and says Started by an MCP client only when by is mcp"
        decision: "spec.md:US9-S2"
      - path: crates/finder/web/src/components/ImagingRunBlock.test.tsx
        status: new
        disposition: add
        assertion: "a focus pause shows its message as an alert with Check focus again and Go anyway posting anyway false and true"
        decision: "spec.md:US9-S3"
      - path: crates/finder/web/src/components/ImagingRunBlock.test.tsx
        status: new
        disposition: add
        assertion: "a centring run reads its message as a status and each warning as a paragraph"
        decision: "spec.md:US9-S4"
      - path: crates/finder/web/src/components/ImagingRunBlock.test.tsx
        status: new
        disposition: add
        assertion: "an ended run says why and what was saved, as a status for a stop and as an alert for a guard's ending, and offers Start imaging run again"
        decision: "spec.md:US9-S5"
      - path: crates/finder/web/src/components/ImagingRunBlock.test.tsx
        status: new
        disposition: add
        assertion: "ImagingPage fed by useImaging shows the block before Camera, keeps it under no_link when a run exists, and shows none under no_link with no run"
        decision: "spec.md:US9-S6"
      - path: crates/finder/web/src/components/ImagingRunBlock.test.tsx
        status: new
        disposition: add
        assertion: "DESIGN.md holds the heading Imaging run, built, and design.json one component named Imaging run of kind custom referring to button"
        decision: "spec.md:US9-S7"
      - path: crates/finder/web/e2e/session.spec.ts
        status: existing
        disposition: preserve
        assertion: "the pages without a scope node are still red only at night; they never render the block"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "the page adds no npm package and index.css does not change"
      - "red-only night mode: the block holds no picture and uses existing tokens and classes (spec.md owner decision 17)"
      - "the top bar's Stop and the MCP motion note are unchanged"
      - "the Imaging page tests of 030 to 036 find their own controls, statuses and alerts with the block present (plan.md Shapes that existing tests pin)"
    checks:
      development:
        - "npm test"
        - "npm run typecheck"
        - "npm run build"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs: finder-page, and a design review of the block on a phone in the dark"
    writes:
      - .impeccable/design.json
      - DESIGN.md
      - crates/finder/web/dist
      - crates/finder/web/src/api.ts
      - crates/finder/web/src/components/ImagingPage.tsx
      - crates/finder/web/src/components/ImagingRunBlock.test.tsx
      - crates/finder/web/src/components/ImagingRunBlock.tsx
    no_write_reason: ""
    shared_writes:
      - path: .impeccable/design.json
        other_spec: 036-tracking-from-subs
        other_story: US8
        disposition: merge_order
        reason: "037 depends_on_landed 036-tracking-from-subs, whose US8 last writes this file among the drafts of 030 to 036 as they stood; named again at the re-anchoring"
      - path: DESIGN.md
        other_spec: 036-tracking-from-subs
        other_story: US8
        disposition: merge_order
        reason: "037 depends_on_landed 036-tracking-from-subs, whose US8 last writes this file among the drafts of 030 to 036 as they stood; named again at the re-anchoring"
      - path: crates/finder/web/dist
        other_spec: 036-tracking-from-subs
        other_story: US8
        disposition: merge_order
        reason: "037 depends_on_landed 036-tracking-from-subs, whose US8 last writes this file among the drafts of 030 to 036 as they stood; named again at the re-anchoring"
      - path: crates/finder/web/src/api.ts
        other_spec: 036-tracking-from-subs
        other_story: US8
        disposition: merge_order
        reason: "037 depends_on_landed 036-tracking-from-subs, whose US8 last writes this file among the drafts of 030 to 036 as they stood; named again at the re-anchoring"
      - path: crates/finder/web/src/components/ImagingPage.tsx
        other_spec: 036-tracking-from-subs
        other_story: US8
        disposition: merge_order
        reason: "037 depends_on_landed 036-tracking-from-subs, whose US8 last writes this file among the drafts of 030 to 036 as they stood; named again at the re-anchoring"
    open_questions: []
  US10:
    tests:
      - path: crates/finder/tests/mcp/imaging_run.rs
        status: new
        disposition: add
        assertion: "the four tools are listed with their descriptions and object output schemas, only get_imaging_run read-only, and their input schemas name their arguments"
        decision: "spec.md:US10-S1"
      - path: crates/finder/tests/mcp/imaging_run.rs
        status: new
        disposition: add
        assertion: "get_imaging_run answers null before a run, start_imaging_run answers a run by mcp with the settings asked for and points the mount as mcp, and exactly one call-log line is written, for the start"
        decision: "spec.md:US10-S2"
      - path: crates/finder/tests/mcp/imaging_run.rs
        status: new
        disposition: add
        assertion: "bad arguments, a run that is not on and a console given no parts are tool errors with the routes' words"
        decision: "spec.md:US10-S3"
      - path: crates/finder/tests/mcp/imaging_run.rs
        status: new
        disposition: add
        assertion: "two starts in one second answer the page route's own refusal and the third the rate limit, each logged"
        decision: "spec.md:US10-S4"
      - path: crates/finder/tests/mcp/imaging_run.rs
        status: new
        disposition: add
        assertion: "stop_imaging_run ends the run as stopped with the counts set_stack gave and no log line, and continue_imaging_run takes a paused run on"
        decision: "spec.md:US10-S5"
      - path: crates/finder/tests/mcp/imaging_run.rs
        status: new
        disposition: add
        assertion: "start_imaging_run is refused with a direct axis move is running while move_axis runs, with nothing asked of any part, and accepted after stop"
        decision: "spec.md:US10-S6"
      - path: crates/finder/tests/mcp/imaging_run.rs
        status: new
        disposition: add
        assertion: "no answer of the four tools and no call-log line holds the node's token"
        decision: "spec.md:US10-S7"
      - path: crates/finder/tests/mcp/motion.rs
        status: existing
        disposition: preserve
        assertion: "every listed tool called with empty arguments answers without the token, the four new ones included, and motion attribution of the existing tools is unchanged"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/mcp/tracking.rs
        status: existing
        disposition: preserve
        assertion: "set_tracking over MCP is refused and accepted as before"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and each binary's main.rs holds only its support include and mod lines"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "the existing tools, the door, the rate limit and the call log are unchanged"
      - "stop and stop_all are still the only tools the door carries out before its checks"
      - "no MCP tool is added on the scope node"
      - "the README is not written: its tool lines are US11's"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test mcp imaging_run::"
        - "cargo nextest run -p finder --locked --test mcp motion::"
        - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs, and one run started by an MCP client at the telescope"
    writes:
      - crates/finder/src/api.rs
      - crates/finder/src/mcp/tools.rs
      - crates/finder/tests/mcp/imaging_run.rs
      - crates/finder/tests/mcp/main.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/mcp/tools.rs
        other_spec: 035-live-stack
        other_story: US10
        disposition: merge_order
        reason: "037 depends_on_landed 035-live-stack, whose US10 last writes this file among the drafts of 030 to 036 as they stood; named again at the re-anchoring"
      - path: crates/finder/tests/mcp/main.rs
        other_spec: 035-live-stack
        other_story: US10
        disposition: merge_order
        reason: "037 depends_on_landed 035-live-stack, whose US10 last writes this file among the drafts of 030 to 036 as they stood; named again at the re-anchoring"
    open_questions: []
  US11:
    tests:
      - path: crates/finder/tests/console/imaging_run_docs.rs
        status: new
        disposition: add
        assertion: "the README's An imaging run section names every state word, every ending reason, the three routes, the four tools and run.json between backticks"
        decision: "spec.md:US11-S1"
      - path: crates/finder/tests/console/imaging_run_docs.rs
        status: new
        disposition: add
        assertion: "the field checklist has exactly fourteen steps numbered in order, names Start imaging run, start_imaging_run, Stop and run.json, and its step 8 has the observer look at the corners of the run's own solved imaging frames"
        decision: "spec.md:US11-S2"
      - path: crates/finder/tests/console/imaging_run_docs.rs
        status: new
        disposition: add
        assertion: "the API table has a row for each of the three routes and MCP clients names the four tools"
        decision: "spec.md:US11-S3"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and each binary's main.rs holds only its support include and mod lines"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "no code changes: the story writes the README and its test"
      - "the README's existing sections are unchanged but for three rows of the API table and four lines of MCP clients"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console imaging_run_docs::"
        - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, who fills in the checklist on the first night with the whole chain"
    writes:
      - crates/finder/README.md
      - crates/finder/tests/console/imaging_run_docs.rs
      - crates/finder/tests/console/main.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/README.md
        other_spec: 036-tracking-from-subs
        other_story: US7
        disposition: merge_order
        reason: "037 depends_on_landed 036-tracking-from-subs, whose US7 last writes this file among the drafts of 030 to 036 as they stood; named again at the re-anchoring"
    open_questions: []
```
