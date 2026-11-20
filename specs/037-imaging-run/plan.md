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
solver and the stack. US12, built first, writes what a run is handed and what it can say
before any run exists: the five traits, the endings with their words, and a scripted
stand-in for the parts, each tested with no run. US1 writes the machine on them and
tests, in paused time, the path on which every step answers and two failures on it. US2
adds what the machine does when a step fails or the stack reports: the retries, the
table of failed steps, the counts, the tracker's pause and the warnings; its test module
also holds the tests of US1's refusals of a start, of a stop, of the mount's endings and
of a dropped runner, moved there to keep US1 under 64 KiB (§ Risks). US3 serves the
runner over three routes, puts its view in the imaging
view and keeps `run.json`. US4 writes the console's own mount, camera and solver parts:
pointing through Track, the lease of the imaging camera, and 032's solve. US5 adds
centring to the machine and, to the mount part, the wait for a live pointing that a
re-centre needs. US6 adds the focus gate and its pause over 033; US7 the limits and the
stack's own endings; US8 the stack part over 035 and 036 and the binary's wiring. US9
adds the run block to the Imaging page, US10 four MCP tools, US11 the README section and
the field checklist. The run asks Track and nothing else to move the mount, asks nothing
of the mount when it ends but a Stop when it is itself stopped while still slewing, and
holds no number sized to one telescope.

Twelve stories, where a first draft had eight: a second agent's review judged the
machine, the centring and the stacking stories each likely to pass 64 KiB, and a split
after a probe build would have meant a changed spec and a relaunch (§ Plan traps 5).
Each of the three is two stories here, and the first of the six was cut once more,
before any dispatch, when the owner split US12 from US1 on 2026-11-19 (§ Decisions and
the alternatives rejected).

## Technical Context

**Language/Version**: Rust (MSRV 1.96) for US1 to US8 and US10 to US12; TypeScript with
React 19 for US9.

**Primary Dependencies**: nothing new. `finder` already has `tokio` with `sync` and
`time`, `serde` and `serde_json` (`crates/finder/Cargo.toml:32`), and `rmcp`. The page
adds no npm package.

**Storage**: `run.json` under the console's imaging folder (031: `imaging/` in the state
folder, or `--imaging-dir`): in the stack's folder, `runs/<stack id>/`, beside 035's
`stack.json`, `stack.fits` and `stack.png`, or in `runs/<run id>/` for a run that ends
before it stacks. The stack's subs are in its folder's `subs/` (035's owner decision
13); the run's own imaging frames stay under `frames/`, where 031 saves them.
Nothing about a run is kept in `session.json`: it is run-only, like Track.

**Testing**:
- US12: `crates/finder/tests/console/imaging_run_words.rs` (new), the module
  `imaging_run_words` of finder's `console` test binary: the endings' words, the
  request, `run_id` and the script's own contract, with no `Runner`.
- US1: `crates/finder/tests/console/imaging_run.rs` (new), the module `imaging_run` of
  finder's `console` test binary: a `Runner` over the script, in paused time.
- US2: `crates/finder/tests/console/imaging_run_steps.rs` (new), the module
  `imaging_run_steps` of `console`: the same, for failed steps, counts and warnings,
  and for the scenarios of US1's machine that were moved to US2 (US2-S6 to US2-S10).
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
- `web/dist` is committed and must be the build of its source (US9).
- Budgets are things a test measures; this plan states none for the real telescope.

**Scale/Scope**: one module with three children, three routes, one page block, four MCP
tools, one README section, in twelve stories.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Nothing touches the solver, detection or `crates/math-core`. The order of the steps, the centring tolerances in use elsewhere and the forced stop of a solve loop that never converges are grounded in `docs/research/2026-11-15-imaging-camera.md` § 12. Prior art and what was not adopted; the 60″ of GoTo and the tracker's endings in the code cited in spec.md § Clarifications. The run's own counts and limits are owner decisions, named as such. |
| II. Never weaken a gate | No existing test, golden, fixture, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS`, `ergane.yaml` or `playwright.config.ts` is edited. The only edits to existing test files are the `mod` lines spec owner approval 8 names. |
| III. Tests first, independent expectations | Oracles: for the machine, the words, states and call lists each scenario states, compared with what a scripted stand-in recorded; for the real parts, the simulator's truth (the true aim point, the true centre of the imaging frame, the mount's true motion), the session, and the answers of 031's and 033's own routes. `run.json` and `stack.json` are parsed by the tests themselves. Clocks are injected; virtual time only in tests and `finder --simulate`. |
| IV. Critical paths | No `unsafe`, no dependency or `Cargo.toml` change, no database-format change, nothing under `proto/`, nothing in `crates/scope-link`. |
| V. Proven open source first, honest licensing | The finder camera's solver is untouched. The run reaches ASTAP only through 032's solver port; principle V of constitution 1.3.0 allows that. |

## Project Structure

```text
crates/finder/
├── README.md                             # US11: three API rows, four tools, ## An imaging run
├── src/lib.rs                            # US3: three route lines
├── src/api.rs                            # US3: imaging_runner, with_imaging_run_steps; US8: with_imaging_run; US10: the tool name in start_motion
├── src/main.rs                           # US8: .with_imaging_run()
├── src/imaging/mod.rs                    # US12: pub mod run; US3: run in ImagingView, attach_run
├── src/imaging/routes.rs                 # US3: three routes
├── src/imaging/run.rs                    # US12: new, the parts and the words; US1: the view and the machine; US2: failed steps, counts; US3: the record; US4: pub mod console; US5: centring; US6: the pause; US7: limits
├── src/imaging/run/script.rs             # US12: new, the scripted stand-in
├── src/imaging/run/store.rs              # US3: new, run.json
├── src/imaging/run/console.rs            # US4: new, mount, camera and solver parts; US5: the wait of repoint; US6: focus part; US8: stack part
├── src/mcp/tools.rs                      # US10: four tools
├── tests/console/main.rs                 # US12, US1, US2, US3, US5, US6, US7, US11: one mod line each
├── tests/console/imaging_run.rs          # US1: new
├── tests/console/imaging_run_steps.rs    # US2: new
├── tests/console/imaging_run_routes.rs   # US3: new
├── tests/console/imaging_run_centring.rs # US5: new
├── tests/console/imaging_run_focus.rs    # US6: new
├── tests/console/imaging_run_limits.rs   # US7: new
├── tests/console/imaging_run_docs.rs     # US11: new
├── tests/console/imaging_run_words.rs    # US12: new
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
amendment 1 gives 031, is 031's US9; 033, 034 and 035 take the same lease. All of it
is re-read on `main` before the flip (§ Prerequisites 2 and 3).

### What 030 to 036 offer (read, never written, by this spec)

| What a part needs | From | Name | Where it is written |
|---|---|---|---|
| the camera's state, description, settings and latest imaging frame | 031 | `Imaging::view()` → `ImagingView { state, camera, settings, frame, .. }`; `SettingsView::roi`; `CameraView::{width, height}` | 031 plan.md § The console's imaging state |
| one imaging frame, awaited, and its abort | 031 | `Imaging::take(purpose)` → the whole `ImagingView`, whose `frame` may be absent; `Imaging::ended(id)` → `FrameView`; `Imaging::abort()` | 031 plan.md § Receiving and saving |
| why a request was not carried out | 031 | `crate::imaging::Refusal { status, message }` | 031 plan.md § The console's imaging state |
| the one lease of the imaging camera, and an imaging frame taken by its holder | 031 | `Imaging::lease(holder)` → `Lease`, released when dropped; `Lease::take(purpose)`; a refusal `the imaging camera is held by {holder}` | 031 US9 (031 plan.md, its section on the lease); the plan document, amendment 1 |
| the UTC text of a time | 031 | `crate::imaging::fits::utc(unix_s) -> (String, String)` | 031 plan.md § FITS at the console |
| a target's name as a folder's name takes it | 035 | `crate::imaging::stack::service::label(name: &str) -> String` | 035 spec.md FR-035, US8-S3; 035 plan.md § The stack on the console |
| the imaging folder | 034 | `Imaging::root()` | 034 plan.md § 031's imaging state |
| whether there is an imaging solver | 032 | `Imaging::solver()` | 032 plan.md § Solving on the console |
| solve the latest imaging frame, awaited | 032 | `Imaging::solve()`, `Imaging::solved(frame_id)` → `SolveView { state, ra_deg, dec_deg, arcsec_per_px, reason, error, .. }` | 032 plan.md § Solving on the console |
| whether a solve moved the aim point | 032 | `SessionView::aim` → `Option<AimView { x, y, source }>`, read before the solve and after it; `Session::held_aim()` → `Option<Aim>`, the measured aim point held back while Track is on, read after it; 032's task hands a measured aim point to `Session::set_measured_aim`, which holds it back while Track is on, before the solve reads `solved` | 032 plan.md § The session (the held aim point), § The relation, § Solving on the console |
| the focus check, awaited | 033 | `Imaging::focus_start(FocusAsk { check: true, .. })`, `Imaging::focus_ended()` → `FocusView { unit, aid, .. }`, `AidView { check, state, verdict, median, last, reference, ended_reason, error, .. }` | 033 plan.md § The aid on the console, § The focus view |
| the focus aid, opened and closed | 033 | `Imaging::focus_start(FocusAsk::default())`, `Imaging::focus_stop()` | 033 plan.md § The aid on the console |
| the live stack | 035 | `AppState::stack()` → `Option<Arc<Stacker>>`; `Stacker::start(StartRequest, Option<&str>)`; `Stacker::stop()` and `Stacker::view()` → `Option<StackView>`, `StackView { state, id, started_at, stacked, dropped, ended_reason, error, save_error, .. }`; `dropped`'s keys `behind` and `unprocessed`, which are not failed subs | 035 plan.md § The stack on the console, § The console |
| whether the stack waits, for the tracker or for its exposure limit, and the set point | 036 | `Imaging::steering()` → `Steering::waiting()`, `Steering::set_set_point(None)` | 036 plan.md § The steering, § The imaging state |
| how far from the set point a stack may begin and still be steered | 036 | `crate::imaging::steering::FAR_ARCSEC`, 180.0 | 036 plan.md § The steering; 036 spec.md FR-014 |
| whether the tracker is steady enough to expose, and was over a past window | 036 | `Session::steered()` → `Option<Arc<dyn Steered>>`; `Steered::steady()`, `Steered::steady_between(start, end)` over console Unix seconds | 036 plan.md § The session, § The tracking error and the interface |
| how far back an aim reading fits the finder camera's solves | 032 | `crate::imaging::relation::FIT_REACH_S`, 10 s, or the exposure when that is longer | 032 plan.md § The relation |
| a push by hand, and whether it has settled | 038 | `TrackView::push` → `moving` or `following`, absent with no push | 038 spec.md FR-009 |

### The parts and the words (`crates/finder/src/imaging/run.rs`, new, US12)

US12 writes this half of `run.rs`: every public item of the module but the view's types
and the `Runner`. Nothing here needs a run, US12-S1 and US12-S2 assert it with none, and
no later story changes any of it.

```rust
//! The imaging run (037): one procedure from a chosen target to a saved stack, carried
//! through five parts the console hands it. It moves the mount only by asking its mount
//! part for Track, and asks nothing of the mount when it ends, but a Stop when it is
//! stopped while it still slews.
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

/// Why a run is over. The whole enum, its reasons and its messages are here from US12;
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
    /// The sentence of § The run's words. `name` is the target's.
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

/// The five parts. The five traits and this struct are complete from US12: no later
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

/// A target's name as a folder name takes it: every character that is not an ASCII
/// letter or digit made `-`. Calls 035's `crate::imaging::stack::service::label`.
pub fn label(name: &str) -> String;
/// `<UTC start as YYYY-MM-DDTHH-MM-SSZ>-<label(name)>`.
pub fn run_id(started_at: f64, name: &str) -> String;
```

**`label`** is one line, `crate::imaging::stack::service::label(name)`: the pure
function 035 offers for the rule its stack uses to name its folder, and which 035's
`AppState::target_label` calls on the session's target (035 plan.md § The stack on the
console, § The console). The rule is written once, in 035; the machine passes the name
it was handed and reads no session, so it stays mount-neutral (§ Plan traps 18). US12-S2
pins the answer to the worked example `NGC 1976` → `NGC-1976`.
**`run_id`** takes the second string of 031's `crate::imaging::fits::utc(started_at)`
(`YYYY-MM-DDTHH:MM:SS.mmm`), keeps its first nineteen characters, makes each `:` a `-`,
and adds `Z`, `-` and `label(name)`. `run_id(1768528800.0, "M42")` is
`2026-01-16T02-00-00Z-M42`, and with the name `NGC 1976` it ends `-NGC-1976`.

**`Ending::message`** words an ending by § The run's words, whose table of endings
it holds whole, with the three rules that table uses: an offset in arcseconds or
arcminutes, the minutes of a time limit, and `1 sub` for one. `settings` gives it the
number of subs asked for and the time limit. US12-S1 types every sentence.

### The machine (`crates/finder/src/imaging/run.rs`, US1)

US1 adds the rest of `run.rs` after what US12 wrote, and changes none of that: the view,
which only a run fills (`RunView` is `#[non_exhaustive]`, so a test outside the crate
reads one and cannot build one), and the `Runner` with its task.

```rust
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct SubCounts { pub stacked: u64, pub dropped: BTreeMap<String, u64> }
/// `reading` and `record` are skipped while `None`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FocusNote {
    pub verdict: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reading: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub record: Option<f64>,
    pub unit: &'static str,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CentringNote { pub tries: u32, pub offset_arcsec: f64, pub tolerance_arcsec: f64 }
/// `saved`: the run started a stack and `stack.stop` answered `Ok`. `track_on`: Track
/// was on at the ending and has been ever since.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EndedNote {
    pub reason: &'static str, pub at: f64, pub message: String,
    pub saved: bool, pub track_on: bool,
}

/// `run` in the imaging view. Serialized in this field order; `focus`, `centring`,
/// `stacking_since`, `limit_at`, `pause_ends_at`, `ended` and `folder` are skipped
/// while `None`,
/// `warnings` while empty; `settings.subs` is `null` when no number was asked for.
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focus: Option<FocusNote>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub centring: Option<CentringNote>,
    /// Console Unix seconds at which the stack started.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stacking_since: Option<f64>,
    /// Console Unix seconds at which the time limit will end the run: `stacking_since`
    /// plus `settings.duration_s`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_at: Option<f64>,
    /// Console Unix seconds at which a focus pause ends by itself; only in
    /// `focus_pause` (US6).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pause_ends_at: Option<f64>,
    pub message: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended: Option<EndedNote>,
    /// Relative to the imaging folder.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder: Option<String>,
}

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
    /// End the run that is on as `stopped` and wait until it has ended; a run still
    /// slewing has its mount stopped too. The latest view, or `None` before the first
    /// start.
    pub async fn stop(&self) -> Option<RunView>;
    /// Go on from a focus pause (US6). Until then always refused.
    pub fn resume(&self, anyway: bool) -> Result<RunView, Refusal>;
    /// The latest run, on or ended; `None` before the first accepted start.
    pub fn view(&self) -> Option<RunView>;
}
```

`Clock` is the console's (`crates/finder/src/session.rs:24`) and `Refusal` is 031's
`crate::imaging::Refusal { status, message }`.

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
7. The new run replaces the latest one: `started_at` is `clock.now()`, read once, and
   `id` is `run_id(started_at, &target.name)`; no `folder`, `settings` with
   `duration_s` or `DEFAULT_DURATION_S` and `keep_subs` or true, the state `slewing`.
   Its task is spawned and its view answered.

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
whatever they hold (US2-S9). That is what keeps 015's rule for a dropped console
(`crates/finder/tests/motion/tracking/routes.rs:456`) true while a run is on: the
console's own parts each hold a clone of the console's state, and that clone holds the
owner of Track.

**The task.** Times shown to the observer (`started_at`, `stacking_since`, `limit_at`,
`pause_ends_at`, `ended.at`), and the two ends of a window asked of `steady_between`,
are the console
clock's; everything measured (`elapsed_s`, and from US6 and US7 the pause and the time
limit) is `tokio::time::Instant`, which a paused test clock drives. Three helpers:

- `guarded(step)`: `tokio::select!`, `biased`, over the stop signal, an interval of
  `POLL` at which `mount.state()` is read, and the step. A stop answers `RunStopped`; an
  `Ended(e)` answers `e`; either drops the step's future. Otherwise the step's value.
- `steady(show)`: first at once and then every `POLL`, `mount.state()`: `Ended(e)` ends
  the run with `e`; `Tracking { steady: true }` returns; `Slewing` and
  `Tracking { steady: false }` wait, and when `show` is `Some((a, b))` the state is set
  to `a` for the first and `b` for the second, with its message. A stop ends the run
  `RunStopped`; but a stop that arrives while the state word is `slewing` first awaits
  `mount.stop()`, not guarded, and ends the run `Stopped`: nothing has been exposed
  yet, and the only thing the run has done is start that motion (spec owner decision
  14, as changed on 2026-11-19). `steady(None)` leaves the state word as it is and,
  from US2, notes while it waits that the run waits for the tracker, which the message
  shows (§ The run's words).
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
   `CheckStopped`: `guarded` has already taken the run's own stop and the mount's
   ending, so a `Stopped` that reaches the machine from `focus.check` is a stop from
   outside. Any other `Err(f)` sets `focus` to `unmeasurable` with no reading, no
   record and the unit `px`, adds the warning
   `The focus check did not finish: {words}.`, keeps the failure, for the words of US6's
   pause, and goes on as `Unmeasurable` does; US2 first sorts two kinds of failure out
   of it. Whenever the run goes on from here to
   step 4, it first takes the lease again: `camera.hold()`, whose `Err(words)` ends the
   run `CameraFailed(words)`.
4. The state is `solving`. `frame()`, then `guarded(solver.solve(&frame, &target))`.
   `Ok(centre)` sets `centring` to `{tries: 0, offset_arcsec, tolerance_arcsec}` and
   goes on: US5 adds the loop here. `Err(Stopped)` ends the run `Stopped`. In US1 any
   other `Err(f)` ends it `SolverFailing` with `f`'s words; US2 replaces that arm.
5. The state is `stacking`, with its message; `camera.release()`, because the stack
   takes the lease itself; then `stack.start(&target).await`, not guarded. `Ok(folder)`
   sets the view's `folder`, its `stacking_since`, the console clock's time then, and
   its `limit_at`, that time plus `settings.duration_s`: where the limit will fall on
   the console's clock, the limit itself being measured on tokio's (§ Limits and the
   stack's endings in the machine).
   `Err(Stopped)` ends the run `Stopped`. In US1 any other
   `Err(f)` ends it `StackFailed` with `f`'s words; US2 replaces that arm. A stack whose
   start failed is not stopped.
6. Every `POLL`: `mount.state()` `Ended(e)` ends the run with `e`; a stop ends it
   `RunStopped`. In US1 the counts stay at zero and the state stays `stacking`: US2 reads
   `stack.progress()` here, and US7 adds the limits and the stack's own endings.

`stack.start`, `stack.stop`, `focus.open_aid`, `focus.close_aid`, `mount.repoint` and
`mount.stop` are awaited unguarded: dropping one half-way would leave a stack, a focus
aid or Track in a state nobody owns. Each is bounded by the part behind it: `repoint`
by `FRESH_POLLS` polls, Track's own wait of up to 2 s for the axes to rest, one Track
off and one Track on, about 7 s in all (§ The console's mount part); `mount.stop` by the
console's Stop; the other four by 033's and 035's own starts and stops, and
`stack.start` on the console's own part also by the writing of its reserve, which takes
as long as the disk needs for that file (§ The console's stack part). A stop of
the run that arrives during one is acted on as soon as the call has answered: the stop
signal stays set, and the next `guarded` or `steady` ends the run `RunStopped` (spec
FR-005, US5-S3).

**Ending**, in this order: from US2 the run notes that it is ending, which changes its
`message` only, to `Saving the stack: {stacked} stacked.` when step 5 returned `Ok`
(§ The run's words); `focus.close_aid()`, once, if the run gave up a check or,
from US6, is ending from a focus pause, whether or not the aid could be opened;
`stack.stop(settings.keep_subs)` if step 5 returned `Ok`, whoever ended the stack, a
failure of which is ignored in US1 and from US2 adds the warning
`The stack did not end cleanly: {words}.`; from US2, after `stack.stop` answers,
`stack.progress()` is read once more and its counts copied, so that a sub stacked
after the last poll is in the view, the ending's message and the record, no rule
being judged on that read; from US2, with `keep_subs` false and nothing stacked by
that read, the warning `The subs were kept: nothing was stacked.`;
`camera.release()`, always and last, whether
or not the run still holds the lease; then `mount.track_on()` is read, and a
`RunStopped` ending that finds Track off becomes `Stopped`, because the mount was
stopped meanwhile; then `ended` is set with the reason, the console
clock's time, the message, `saved` (step 5 returned `Ok` and `stack.stop` answered
`Ok`) and `track_on` as just read, the state is `ended` and `elapsed_s` is fixed; from US3
the `ended` event is pushed and the record made and written (§ The record); only then
is the run over: `start`'s check 2 lets a new run in, and whoever waits in `stop` is
woken. Nothing is asked of the mount part after `point` and `repoint` but the Stop of
a run stopped while it slews; `state`, `track_on` and `steady_between` only read. A
run that ends drops its task; nothing restarts it.

**`stop`** first takes and lets go the async lock of `start`, so that a stop which
arrives while a start is still asking Track acts on the run that start makes, or finds
none; then it sets the stop signal of a run that is on and waits for its ending; with
none on it answers `view()` at once. **`view`** copies the run under the runner's lock,
releases it, and sets `elapsed_s` from the instant of the start to now, or to the
ending. For an ended run whose `ended.track_on` is true it then reads
`mount.track_on()`, outside the lock; once that reads false, `ended.track_on` is false
in this answer and, written back under the lock, in every later one. So the field
says that Track has been on ever since the ending, and the page's line about it goes
away when Track ends. `run.json` keeps the value of the ending.

**`resume`**, from US1 until US6 gives it its work, refuses with 409
`no imaging run is running` when no run is on and with 409
`the imaging run is not paused` otherwise. No scenario of US1 or US2 calls it: US3-S4 is
its oracle.

### Failed steps and counts in the machine (`crates/finder/src/imaging/run.rs`, US2)

```rust
/// Imaging frames in a row the camera may fail before the run ends.
pub const FRAME_TRIES: u32 = 3;
/// Solves in a row that may find no solution before the run ends.
pub const SOLVE_TRIES: u32 = 3;
/// How long the run waits after a failed take before it asks again.
pub const RETRY_WAIT: Duration = Duration::from_secs(5);
```

US2 replaces the arms US1 gave a failed take, a failed solve and a failed `stack.start`
with this table, sorts the failures of a focus check by it, and reads the stack's
progress. US5 and US7 use its last two rows.

| A failed step | `Stopped` | `Camera` | `Save` | `SolverUnavailable` | `NotSolved` | `Mount`, `Other` |
|---|---|---|---|---|---|---|
| `camera.take` | `Stopped` | counted | `SaveFailed` | counted | counted | counted |
| `focus.check` | `CheckStopped` | unmeasurable | `SaveFailed` | `SolverUnavailable` | unmeasurable | unmeasurable |
| `solver.solve` | `Stopped` | counted | `SaveFailed` | `SolverUnavailable` | counted | counted |
| `stack.start` (US2), and `StackProgress::failed` (US7) | `Stopped` | `CameraFailed` | `SaveFailed` | `SolverUnavailable` | `SolverFailing` | `StackFailed` |
| `mount.repoint` (US5) | `Stopped` | `MountFailed` | `MountFailed` | `MountFailed` | `MountFailed` | `MountFailed` |

- **A take that is counted** is one more failure. At `FRAME_TRIES` in a row the run ends
  `CameraFailed` with the last one's words; otherwise it waits `RETRY_WAIT`, guarded,
  so that a stop or the mount's ending is acted on first, and `frame()` goes round
  again: `steady(None)`, then another take. `Ok` zeroes the count. 031 refuses a take
  at once while the camera is not ready, so without the wait a link that drops for a
  second would spend all three takes in one poll; with it Track has 10 s to report
  `link_lost` before the third take, and the camera 10 s to come back.
- **A solve that is counted** is one more failure. At `SOLVE_TRIES` in a row the run
  ends `SolverFailing` with the last one's words; otherwise `frame()` and the solve
  again. No focus check is repeated. `Ok` zeroes the count.
- **A focus check** that fails with `Save(e)` or `SolverUnavailable(e)` ends the run
  that way. Every other failure is US1's: unmeasurable, with its warning.
- **`stack.start`** that fails ends the run by its row; the stack is not stopped.
- **`stack.stop`** that fails at an ending adds the warning
  `The stack did not end cleanly: {words}.` and changes no reason.
- **`SlightlySoft`** adds its focus sentence as a warning and goes on.
- **Step 6**, every `POLL`, in this order: `stack.progress()` is copied into `subs`;
  then the mount's `Ended(e)` ends the run with `e` and a stop ends it `RunStopped`;
  then the state is `paused_by_tracker` while `waiting_for_tracker`, else `stacking`,
  each with its message. The counts come first so that an ending holds them. The
  mount and the stop come before anything the stack reports (US7 puts its rules after
  them), so that a lost link, which also fails the stack's next take, reads
  `link_lost` when Track has seen it. Nothing in US2 reads `running` or `failed`: US7
  does.
- **Waiting for the tracker.** While `steady(None)` waits in `first_frame`,
  `focus_check`, `solving` or `centring`, the run's message is the state's own
  followed by ` Waiting for the mount to settle; nothing to do.`; it is the state's
  own again as soon as the wait is over.
- **An ending** notes that it has begun, reads the counts once more after
  `stack.stop`, and adds the warning for kept subs, as § The machine's Ending has
  them.

### The run's words

Every sentence is exactly this, with `{name}` the target's name. An offset, `{offset}`,
is written in arcseconds with no decimal while it is below 600″ (`400″`, by `{:.0}`),
and in arcminutes with one decimal from 600″ on (`10.0′`, `63.2′`). The minutes of a
time limit, `{minutes}`, are `duration_s` divided by 60, with one decimal and never
below `0.1`, a whole number written without its `.0` (`120`, `1.5`). A focus reading or
record is written with one decimal and its
unit: `3.6″` for `Unit::Arcsec`, `6.6 px` for `Unit::Px`. A number of subs that is 1
is written `1 sub`, with `is` where the sentence has `are`.

| State | `message` |
|---|---|
| `slewing` | `Slewing to {name}.` |
| `tracking` | `Tracking {name}. Waiting for the mount to settle; nothing to do.` |
| `first_frame` | `Taking a first imaging frame.` |
| `focus_check` | `Checking focus.` |
| `focus_pause` (US6) | the focus sentence below, then ` Turn the focuser while watching the focus aid below, then check focus again or go on anyway.`; for `unmeasurable`, then ` Check for cloud or dew, then check focus again or go on anyway.`; for `no_record`, then ` Focus with the focus aid, then check focus again or go on anyway.`; and for a check that did not finish, unless its failure is `NotSolved`, which reads as `unmeasurable` does, in place of both: `The focus check did not finish: {w}. Check the imaging camera, then check focus again or go on anyway.`, `{w}` the failure's words |
| `solving` | `Solving an imaging frame.` |
| `centring` (US5) | `Centring {name}: {offset} from the sensor's centre, try {n} of 3.` after a re-centre was asked for; `Centring {name}: {offset} from the sensor's centre, no new aim point yet.` after a solve that measured no new aim point |
| `stacking` | `Stacking {name}: {stacked} stacked, {dropped} dropped.`, `{dropped}` the sum over the reasons |
| `paused_by_tracker` | `Exposures paused: {stacked} stacked so far. They resume by themselves.` |
| `first_frame`, `focus_check`, `solving` or `centring`, while the run waits for the tracker (US2) | the state's own message, then ` Waiting for the mount to settle; nothing to do.` |
| any, once the run is ending (US2) | `Saving the stack: {stacked} stacked.` when step 5 returned `Ok`, else the state's own message |
| `ended` | the ending's message |

| Verdict | The focus sentence |
|---|---|
| `slightly_soft`, with a reading and a record | `Focus is slightly soft: stars are {reading} across against a record of {record}.` |
| `slightly_soft`, without either | `Focus is slightly soft.` |
| `clearly_out`, with a reading and a record | `Focus is clearly out: stars are {reading} across against a record of {record}.` |
| `clearly_out`, with a reading only | `Focus is clearly out: stars are {reading} across.` |
| `clearly_out`, with no reading | `Focus is clearly out.` |
| `unmeasurable`, and a check that did not finish because it found no stars to measure | `Focus could not be measured: too few stars.` |
| `no_record` | `There is no focus record for this camera yet.` |
| `good` | none: no step words a good verdict, which adds no warning and no pause |

| Ending | `reason` | `message` |
|---|---|---|
| `Stopped` | `stopped` | `Stopped: the mount was stopped.` |
| `RunStopped` | `stopped` | `Imaging run stopped. Track is still on.` |
| `StackStopped` | `stopped` | `Ended: the stack was stopped.` |
| `CheckStopped` | `stopped` | `Ended: the focus check was stopped.` |
| `CountLimit` | `count_limit` | `Finished: the {n} subs asked for are stacked.`, `{n}` the request's `subs`; `Finished: the 1 sub asked for is stacked.` for one |
| `DurationLimit` | `duration_limit` | `Finished: the {minutes}-minute limit was reached.` |
| `TooLow` | `too_low` | `Ended: {name} is below the minimum altitude.` |
| `SunUp` | `sun_up` | `Ended: the Sun is up.` |
| `LinkLost` | `link_lost` | `Ended: the mount or the scope link stopped answering. Check the scope node and the mount's cable, then start again.` |
| `MountFailed(w)` | `mount_failed` | `Ended: Track ended and the run with it: {w}. Check the mount, then start again.`, which is true of a guard this spec does not know as of a fault |
| `TargetChanged` | `target_changed` | `Ended: the target was changed. Start a new imaging run for the new target.` |
| `PushedAway` | `pushed_away` | `Ended: the telescope was pushed away from {name}. Track follows where it is left. Start a new imaging run to go back to {name}.` |
| `CameraFailed(w)` | `camera_failed` | `Ended: the imaging camera failed: {w}.` |
| `SolverUnavailable(w)` | `solver_unavailable` | `Ended: the imaging solver cannot run: {w}.` |
| `SolverFailing(w)` | `solver_failing` | `Ended: 3 imaging frames in a row could not be solved: {w}. Check for cloud, dew and focus.` |
| `SaveFailed(w)` | `save_failed` | `Ended: the console could not save to its imaging folder: {w}. Check the disk.` |
| `FocusTimeout` | `focus_timeout` | `Ended: focus was left paused for 15 minutes. Start again when ready.` |
| `NotCentred`, `measured` true | `not_centred` | `Ended: {name} is still {offset} from the sensor's centre after 3 tries. Check the aim point on the console page, then start again.` |
| `NotCentred`, `measured` false | `not_centred` | `Ended: {name} is {offset} from the sensor's centre and no new aim point could be measured.` |
| `SubsFailing(w)` | `subs_failing` | `Ended: no sub was stacked for 10 minutes ({w}). Cloud, dew or a tree is the usual cause; check the sky and start again.`, `{w}` the reasons with a space for each underscore |
| `StackFailed(w)` | `stack_failed` | `Ended: the stack stopped: {w}.` |

| Warning | Words |
|---|---|
| verdict `slightly_soft` (US2) | its focus sentence |
| a focus check that did not finish (US1) | `The focus check did not finish: {words}.` |
| going on anyway (US6) | `Focus was not checked again: the observer went on anyway.` |
| the focus aid (US6) | `The focus aid could not be opened: {words}.` |
| the stack at an ending (US2) | `The stack did not end cleanly: {words}.` |
| subs not removed because nothing was stacked (US2) | `The subs were kept: nothing was stacked.` |
| the record at an ending (US3) | `The run's record could not be saved: {words}.` |
| a target stacked off the centre after the last try (US5) | `{name} is {offset} from the sensor's centre after 3 tries; the stack's steering will bring it in.` while the offset is at most 036's `FAR_ARCSEC`, 180″; over it, `{name} is {offset} from the sensor's centre after 3 tries; it is stacked where it is.` |
| a target stacked off the centre after four solves that measured no new aim point (US5) | `{name} is {offset} from the sensor's centre and no new aim point could be measured; the stack's steering will bring it in.` while the offset is at most `FAR_ARCSEC`; over it, `{name} is {offset} from the sensor's centre and no new aim point could be measured; it is stacked where it is.` |
| a focus check that would pause, twice not steady (US6) | `The tracker was not steady during the focus check.` |

The table of endings is whole in `run.rs` from US12, as `Ending::reason` and
`Ending::message`, with the rules above for an offset, the minutes of a time limit and a
number of subs. The other three tables are made from a run's own state and are whole
from US1, as functions of the state, the verdict and the warning; a later story calls
what an earlier one wrote, and the story named beside a warning is the one whose code
first adds it. A warning already in the
list is not added again. The one sentence whose
subject is an altitude, the `too_low` message, is an observer's word and not code
(§ Plan traps 18).

**Stacked off the centre, in two forms.** Each of the two warnings for a target stacked
off the centre ends `; the stack's steering will bring it in.` while
`offset_arcsec <= crate::imaging::steering::FAR_ARCSEC`, and
`; it is stacked where it is.` over it. `FAR_ARCSEC`, 180″, is 036's: its steering
stands back from a stack whose first tracking error is longer, and the view then carries
`steering.far_arcsec` (036 FR-014). The function that words the warning reads the
constant and writes no 180 of its own, as `label` calls 035's rule. It judges by the
offset of the run's last solve, before the stack has taken a sub, and 036 by the stack's
first tracking error, so within a few arcseconds of 180″ the two can fall on different
sides; 036's block on the page then says what the steering does (036 plan.md § The
page).

**`paused_by_tracker` names no cause.** 036's stack waits before an
exposure for more than a mount that settles: its view's `steering.pause` has a word for
a start, a take-up, a measurement, the keyhole pass, a push and a stopped axis, and
`limit` while the tracker is steady and its exposure limit is under half the exposure
held (036 FR-021, FR-022). The stack part hands the machine `Steering::waiting()` alone,
so the run reads no such word, and its one sentence is true of every one of them. 036's
block, on the same page, shows the sentence of the word.

### The script (`crates/finder/src/imaging/run/script.rs`, new, US12)

```rust
//! Scripted stand-ins for the five parts of an imaging run, as `FixedClock` stands in
//! for a clock: tests queue the answers and read back the calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Part { Camera, Solver, Target, Hold, Point }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Step { Take, Check, Solve, Repoint, StackStart, StackStop, CloseAid }

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
    /// `mount.point()` refuse with these words. A refusal lasts: every later call of
    /// that part refuses with the words.
    pub fn refuse(&self, part: Part, words: &str);
    /// Queue the next answer of a take, a focus check or a solve. An empty queue
    /// answers the default. A pushed answer, `Ok` or `Err`, is answered as given and
    /// uses no n; n counts only the takes answered by default.
    pub fn push_frame(&self, answer: Result<FrameRef, Failure>);
    pub fn push_focus(&self, answer: Result<FocusCheck, Failure>);
    pub fn push_solve(&self, answer: Result<Centre, Failure>);
    /// Make the next n calls of `steady_between` answer false.
    pub fn unsteady(&self, n: u32);
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
"frames/2026-01-16/00000n.fits" }` for n = 1, 2, 3 … (six digits), a pushed take, `Ok`
or `Err`, using no n; a focus check `FocusCheck { verdict: Good, reading: Some(3.0), record: Some(3.0),
unit: Arcsec }`; a solve `Centre { offset_arcsec: 0.0, tolerance_arcsec: 90.0,
near_arcsec: 450.0, aim_moved: true }`; `stack.start` answers `STACK_FOLDER`.
`steady_between` answers true, but for the next n calls after `unsteady(n)`.
`track_on` answers from the mount's state as `set_mount` left it: false for
`Ended(Stopped)`, `Ended(TooLow)`, `Ended(SunUp)`, `Ended(LinkLost)` and
`Ended(MountFailed(_))`, true for every other. `mount.stop` is listed and changes
nothing: the script does not model a mount.

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
`solver.solve {frame id}`, `mount.repoint`, `mount.stop`, `stack.start {target name}`,
`stack.stop keep` or `stack.stop discard`. `state()`, `track_on()`, `steady_between()`
and `progress()` are not listed. A
refused `ready()`, `target()`, `hold()` or `point()` is listed.

It is public product code that only tests call (spec owner decision 16, owner approval
5). It holds no logic of the run: it answers what it was told and writes down what it
was asked. It is whole from US12, with the controls only later stories use, so that no
later story writes this file, and US12-S3 and US12-S4 assert its defaults, its call
words and every one of its controls on the script alone, with no run.

### `crates/finder/src/imaging/mod.rs` (US12; US3)

US12 adds `pub mod run;` in alphabetical order among the module lines 030 to 036 put
there. US3 adds, after the view's last part:

```rust
// ImagingView gains:
#[serde(skip_serializing_if = "Option::is_none")]
pub run: Option<run::RunView>,
/// The folder of the newest run record with no `ended`, until the first start (037).
#[serde(skip_serializing_if = "Option::is_none")]
pub unfinished_run: Option<String>,
// Imaging gains a field `run: std::sync::OnceLock<std::sync::Weak<run::Runner>>` and:
/// Let every view this state answers carry the run (037). The first call wins.
pub(crate) fn attach_run(&self, runner: &Arc<run::Runner>);
```

It follows 035's `attach_stack` (035 plan.md § 031's imaging state): wherever `Imaging`
builds the `ImagingView` it answers, after its own lock is released and where 035 sets
`stack`, it sets `run` from `attached.view()` when a runner is attached and still alive,
**whatever the view's `state`**, and `unfinished_run` from `attached.unfinished()` in
the same place. On `main` 034 sets `calibration` there only while the view is
`ready`; `run` and `unfinished_run`, like 035's `stack`, are set outside that test
(US3-S5). `ImagingView::off()` has `run` `None`: a console
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
`centring`, `warnings` and `ended` when the view has them, `ended` with `saved` and
with `track_on` as read at the ending. No `message`, `elapsed_s`, `stacking_since`,
`limit_at`, `pause_ends_at` or `folder`, and nothing of the site, the node's address or
its token.

`Runner` gains, in US3:

```rust
/// As `new`, keeping each run's `run.json` under the imaging folder `root`.
pub fn with_folder(steps: Steps, clock: Arc<dyn Clock>, root: PathBuf) -> Arc<Self>;
/// The folder, relative to the imaging folder, of a run this console did not finish
/// before it last exited. `None` from the first accepted start on.
pub fn unfinished(&self) -> Option<String>;
/// How often, at most, the record is refreshed while the stacked count rises.
pub const RECORD_EVERY: Duration = Duration::from_secs(60);
```

`with_folder` reads the imaging folder once, synchronously: among the folders of
`root/runs` that hold a `run.json`, it takes the one whose name is greatest, which is
the newest because every name begins with its UTC start, and when that record parses
as a JSON object with no `ended` key it keeps `runs/<name>` as the unfinished run. A
`runs` that is missing or cannot be read, and a record that does not parse, leave
none. The first accepted start clears it. Nothing else is read from an old record, and
nothing resumes (spec § Edge Cases, FR-016).

Every run, with a folder or not, keeps a list of `Event`s, one pushed with the console
clock's time each time the state word changes, `slewing` at the start and `ended` at the
end included, but for the changes held back below. With a folder:
- nothing is written before the run stacks;
- when `stack.start` answers `Ok(folder)`: `store.adopt(&folder)`, then `write`;
- at every later change of the state word that is listed as an event: `write`;
- a `write` that fails while the run is on ends the run `SaveFailed(e.to_string())`;
- at the ending, after the lease is given back: with no folder yet, `store.make(&id)`,
  and the view's `folder` becomes the store's; then `write`, with the stack's counts as
  read after `stack.stop` (§ The machine, Ending). A failure of either adds the warning
  of § The run's words, changes no
  reason, and after a failed `make` leaves the view without a `folder`.

The record is also written when `subs.stacked` has risen since the last write and that
write is `RECORD_EVERY` old or more on tokio's clock, so that a run cut short by a
console that exits says how far it got. Such a write pushes no event, and one that
fails ends the run `SaveFailed` as any write while the run is on. So the stack's
counts are written at changes of state, at most once a minute while they rise, and at
the ending, not at every poll.

The words `stacking` and `paused_by_tracker` can change places for hours: 036's steering
waits before an exposure whenever the tracker is not steady, and whenever its exposure
limit is under half the exposure held (036 FR-022). So a change between those
two is listed as an event, and written, only when no such change was listed in the
`RECORD_EVERY` before it, on tokio's clock. One that is held back is neither an event
nor a write; the view's `state` and `message` change at once all the same, and the
record's `state` is then the view's as of its last write (US3-S11). The first pause
after the stack has started is therefore always written (US3-S3).

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

`resume`'s two refusals outside a pause are US1's (§ The machine).

`crates/finder/src/lib.rs` gains, after the last `/api/imaging` route line then there,
three lines, written as they stand here, with no backticks:
`* POST /api/imaging/run: {"subs":N,"duration_s":S,"keep_subs":true}, each optional; start an imaging run on the session's target.`
`* DELETE /api/imaging/run: no body; end the imaging run and save its stack. It stops the mount only while the run still slews.`
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
  | any other `track.state` | `Tracking { steady }`, `steady` being `state.session().steered().is_some_and(\|t\| t.steady() && t.steady_between(now - reach, now))` (036), with `now` the console clock and `reach` the larger of `crate::imaging::relation::FIT_REACH_S` and the imaging view's `settings.exposure_ms` in seconds |

  **Steady means steady for the whole of 032's reach.** 032's aim reading fits the
  finder camera's solves from `reach` before the imaging exposure, and a fit that
  reaches into a GoTo, a re-centre or a take-up reads `moving`, or is biased: round 1
  saw every solve after a re-centre take no aim reading for that reason. 036's
  `steady_between` is false from a tracker's start until 2 s after its first rates,
  across every take-up and keyhole pass with the 2 s that follow each, across a
  measurement, while 038's push reads `moving`, and for a turn that finds an axis
  stopped; a window that begins at the instant such a time ended is not touched by it
  (036 FR-021). Every GoTo and every re-centre starts a new tracker, so the row needs no
  clock and no field of its own. The window
  is in console Unix seconds, which is what 036 keeps its list in. The cost is the
  larger of 10 s and the exposure, after the 2 s to 3 s for which a new tracker settles
  (036's US10-S3 pins this window on 036's own rig: false asked 6 s after a tracker's
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
  while it stacks 036's tracker is itself not steady for as long as the push reads
  `moving`: the stack waits before its next exposure, `Steering::waiting()` is true and
  the run reads `paused_by_tracker`, and the sub in flight is dropped as `not_steady`
  (036 FR-021, FR-022). The instant
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

  Steps 1, 3's check and 5 guard one window between them; a mutation of any one alone
  survives on the simulator, and US5-S5 catches two removed together.

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
  for up to 5 s (spec § Clarifications). Since 038, a step of more than 5′ is read as
  a push by hand instead, which hands the tracker nothing but marks the pointing
  model's slip; a smaller one is still chased. The owner's answer to 032's decision 25
  avoids both in 032. Probe round 1 confirmed on
  US5-S4 that the simulator's mount does not move toward the new aim point between
  the imaging solve and step 2, and that the model's slip is not marked, which since 038
  is what tells the hold from no hold
  (§ Prerequisites 5).

  A re-centre therefore takes at most the 5 s of step 3, Track's own wait of up to 2 s
  for the axes to rest, and the acknowledgements of one Track off and one Track on:
  about 7 s. In that time Track is off, so another client's `goto_target` is not
  refused with `Track is on`; step 4's Track then finds the mount moving and the run
  ends `mount_failed` with Track's words (spec § Edge Cases).

  `stop_tracking` waits for the mount's acknowledgement of its stop, not for rest.
  `Session::start_tracking` itself waits up to 2 s for both axes to rest whenever it
  replaces a tracker, before it asks GoTo, and the probe of round 1 saw the simulated
  axes at rest from the re-centre's Track off on. If
  an axis still reports motion after that, when step 4's Track asks GoTo, GoTo refuses
  with `the mount is already moving` (`crates/finder/src/goto.rs:739`) and the run ends
  `mount_failed` (§ Risks).
- **`stop`**: `api::stop(&state).await` (`crates/finder/src/api.rs:904`), the console's
  Stop, its answer ignored. It moves the stop generation, so the next `state()`
  answers `Ended(Stopped)`.
- **`track_on`**: whether the session's snapshot has a `track` whose `state` is not
  `off`.
- **`steady_between(start, end)`**:
  `state.session().steered().is_some_and(|t| t.steady_between(start, end))` (036).

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
  table of § Failed steps and counts in the machine; then `frame()`, which waits until
  the mount tracks and is steady, and the solve again;
- `centre.aim_moved`, the three tries being spent: when
  `offset_arcsec <= near_arcsec` the run adds the first of the two warnings that § The
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
  (spec owner decision 4). Below `CENTRE_IDLE` the state is `centring` with its second
  message, and `frame()` and the solve again, `tries` unchanged and no re-centre asked
  for.

The state stays `centring` throughout. A take or a solve that fails inside the loop is
counted as US2 counts it. A solve that measured no new aim point is not a try: 032
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
`Unmeasurable`, it asks `mount.steady_between` over those two times. When the tracker
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
   pause, and takes resumes from this moment, the opening of the aid included;
   `focus.open_aid().await`, an `Err` adding the warning of § The run's words
   and changing nothing else;
2. it waits, with `tokio::select!`, `biased` in this order, for the first of: a stop; a
   resume; an `Ended` from
   `mount.state()`, read every `POLL`; `FOCUS_PAUSE_LIMIT` since the pause began. The
   stop, the mount's ending and the limit end the run, as `RunStopped`, as the mount's
   ending and as `FocusTimeout`;
   the ending closes the aid because the note is set, whether or not the aid opened:
   033's `focus_stop` is safe when nothing runs, and one rule is easier to test than
   two. Whichever of the four comes first answers the pause: from then on a resume is
   refused, though the state word is still `focus_pause` until the aid has closed;
3. a resume: `focus.close_aid().await`, on the same rule, the note and `pause_ends_at`
   cleared. With
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
and a stop are judged (§ Failed steps and counts in the machine), and before the state
is shown, the first that holds:
1. `progress.failed` is `Some(f)`: the run ends by the row of `StackProgress::failed`
   in the table of § Failed steps and counts in the machine;
2. `progress.running` is false: the run ends `StackStopped`;
3. `settings.subs` is `Some(n)` and `stacked >= n`: `CountLimit`;
4. the instant now is `settings.duration_s` or more after the instant `stack.start`
   answered `Ok`: `DurationLimit`. This is the clock's time, on tokio's clock: a
   stretch in which the stack waits for the tracker counts, because the limit is there
   to end a forgotten run and not to measure exposure (spec owner decision 8, US7-S2);
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

A failed `stack.start` is US2's, unchanged. Every one of these endings still calls
`stack.stop`, as § The machine's Ending has it: the stack is saved whoever ended it.

Rule 2 is why the script's `set_stack` leaves `running` alone (§ The script): US2's,
US3's and US10's tests set counts on a stacking run and must go on passing once this
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
  the machine awaits unguarded (§ The machine); the ending then calls `stop`, which
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
  `imaging.steering().waiting()` (036), true while 036's stack waits before an exposure,
  for the tracker or for its exposure limit to rise; and `failed`, for an `ended` view, is
  `Some(Camera(error))` when `ended_reason` is `camera`, else `Some(Save(words))` when
  the view has a `save_error`, else `None`.
- `stop(keep_subs)`: first it removes `.reserve` from the folder of the stack it
  started, a missing file being no failure, so that 035's save has that room. Then,
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
  nothing was stacked (§ The machine, Ending).
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
  stacking_since?: number;
  limit_at?: number;
  pause_ends_at?: number;
  message: string;
  warnings?: string[];
  ended?: { reason: string; at: number; message: string; saved: boolean; track_on: boolean };
  folder?: string;
}
// ImagingView gains: run?: ImagingRun; unfinished_run?: string;
export interface ImagingRunRequest { subs?: number; duration_s?: number; keep_subs?: boolean }
export async function postImagingRun(request: ImagingRunRequest): Promise<ImagingView>;
export async function deleteImagingRun(): Promise<ImagingView>;        // fetch(url, { method: "DELETE" })
export async function postImagingRunContinue(anyway: boolean): Promise<ImagingView>;
```

The two posts are written as `postTrack` is (`crates/finder/web/src/api.ts:312`), with
`{ method: "POST", headers: { "Content-Type": "application/json" }, body:
JSON.stringify(…) }`, and each call throws an `Error` with the answer's `error` on a
refusal. When `fetch` itself rejects, each of the three throws an `Error` with the
words `The console did not answer. The run itself is not affected; try again.`, so
that the block never shows a browser's own words. `postImagingRun` builds its body
with the keys in the order `subs`, `duration_s`, `keep_subs`, each only when given.

`components/ImagingRunBlock.tsx`, `ImagingRunBlock({ view, refresh }: { view:
ImagingView; refresh: () => Promise<void> | void })`: nothing unless `view.state` is
`ready` or `view.run` is present; else a `section` labelled by an `h2` `Imaging run`,
with these words exactly, each phrase a test asserts on one line of JSX:

- **No run, or an ended one**: a `quiet` paragraph
  `Goes to the target, checks focus, centres it on the imaging camera's sensor and stacks subs until you stop it or a limit is reached (two hours unless you give another).`;
  number fields labelled `Subs to stack` and `Minutes to stack`, both empty, the second
  with the placeholder `120`; a checkbox
  `Keep every sub`, checked; while the view has a `camera` and its `settings`, a
  `quiet` line `About {n} GB of subs in {m} minutes at the current exposure.`; and the
  button `Start imaging run` (`button
  button-primary`). A click calls `postImagingRun` with `subs` when the first field
  holds a whole number above 0, `duration_s` as the second field's number times 60 when
  it holds one above 0, and `keep_subs` always; then `refresh`. Drafts in the fields
  survive the one-second reads of 031's `useImaging`. In the size line `{m}` is the
  second field's number when it holds one above 0, else 120, and `{n}` is, with one
  decimal, `camera.width` × `camera.height` ÷ `settings.binning`² × 2 bytes, 031's
  sixteen-bit pixels, × the number of exposures of `settings.exposure_ms` that fit in
  `{m}` minutes, ÷ 10⁹. It is an estimate at the exposure as set: 036 may cap a sub
  shorter, which makes more of them. With no run and `view.unfinished_run` present,
  the block also shows, in an `alert`,
  `The last imaging run did not finish: the console stopped while it ran. Its subs are in {unfinished_run}/subs.`
- **A run that is on**: the run's `message` in a `status`, or in an `alert` while the
  state is `focus_pause`; a line `{stacked} stacked · {dropped} dropped`, `{dropped}`
  the sum, followed when it is above 0 by ` ({reason with each _ made a space} {n}, …)`
  in the object's order; a line `Elapsed {m}:{ss}`, from `elapsed_s` rounded down to
  whole seconds; once `stacking_since` is present, a line
  `Stacking for {m}:{ss} of {M}:{SS}`, the first from `started_at` + `elapsed_s` −
  `stacking_since` and the second from `settings.duration_s`, each rounded down to
  whole seconds, because the limit counts from the stack's start and `Elapsed` from
  the press; in `focus_pause` with `pause_ends_at`, a line
  `Focus aid open below · {m}:{ss} left`, from `pause_ends_at` − (`started_at` +
  `elapsed_s`), rounded down and never below zero; when `by` is `mcp`, the line
  `Started by an MCP client`, with the
  classes of `McpNote` (`crates/finder/web/src/components/McpNote.tsx:5`); and each of
  `warnings` as a `quiet` paragraph. In `focus_pause`, one row of its own holds the
  buttons
  `Check focus again` (`button button-primary`), which calls
  `postImagingRunContinue(false)`, and `Go anyway` (`button`), which calls it with
  `true`, each then `refresh`. Last, in a row of its own, the button `Stop imaging run`
  (`button`): a run cannot be resumed, and with `Keep every sub` unticked a stop also
  removes the subs, so the first click only turns it into `Tap again to stop the run`
  for 5 s, after which it reads `Stop imaging run` again; a click on
  `Tap again to stop the run` calls `deleteImagingRun` then `refresh`. The button is
  disabled while the run's `message` begins `Saving the stack`. No fields and no
  `Start imaging run`.
- **Every button of the block.** While a call of the block is unanswered, every button
  of the block is `disabled`, and the one clicked reads `Starting…`, `Checking…` or
  `Stopping…` (`Go anyway` keeps its words). A refusal shows its words in an `alert`
  of the block's own and calls no `refresh`; a call that got no answer shows the words
  `api.ts` throws for it. That alert is cleared when another call of the block starts,
  and when `run.id` or `run.state` changes.
- **An ended run**, above the start form: `{ended.message}` followed by
  ` Saved on the console in {folder}.` when the reason is `count_limit`, `ended.saved`
  is true and
  `stacked` is above 0, the message already holding the number; by
  ` {stacked} subs stacked · saved on the console in {folder}.` when `ended.saved` is
  true and
  `stacked` is above 0, for every other reason; by
  ` {stacked} subs were stacked but the stack was not saved: the subs are in {folder}/subs.`
  when `stacked` is above 0 and `ended.saved` is false; and by
  ` No sub was stacked.` otherwise. One sub is written `1 sub stacked` and
  `1 sub was stacked`. It is in a `status` when the reason is `stopped`,
  `count_limit` or `duration_limit`, and in an `alert` for every other reason and
  whenever subs were stacked and the stack was not saved. Under it, while
  `ended.track_on` is true, a `quiet` paragraph
  `Track is still on: the top bar's Stop stops the mount.`; and each
  warning.

Only existing classes and tokens are used; `index.css` does not change. The block holds
no picture, so the night rule and the hold-to-see-colour control need nothing of it.
`ImagingPage.tsx` (031 US7) renders `<ImagingRunBlock view={view} refresh={refresh} />`
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
> every sub, a quiet line estimates the size of the subs, and one primary button reads
> Start imaging run. While a run is on, one
> status line carries the run's own message, with the counts of subs stacked and
> dropped, the elapsed time and, once it stacks, the time stacked against the limit; a
> run an MCP
> client started says so in Second Ink, as the MCP motion note does. Stop imaging run
> is a secondary button on a row of its own, last, and asks for a second tap within
> five seconds. A focus pause turns
> the message into an alert, says how long the pause may still last, and adds Check
> focus again as the primary button and Go
> anyway beside it, on their own row. Every button is disabled while its request is
> out. After a run the block says why it ended, whether the stack was saved, and
> where: a status for a stop or a limit, an alert for anything else and for a stack
> that was not saved. While Track is still on it says so: the run's own stop stops the
> mount only while the run still slews, and the top bar's Stop does at any time. Ink
> on paper with existing tokens only; at night it is
> red like every other block.

`.impeccable/design.json` gains one entry in `components`, after the last imaging
component then there, formatted as its neighbours: `name` `Imaging run`, `kind`
`custom`, `refersTo` `button`, a `description` of one sentence for each of the block's
three faces (no run, a run
that is on, an ended run), and
`html` and `css` snippets under the class `ds-imaging-run`, using no colour but
`var(--color-ink)`, `transparent` and `inherit`, as its neighbours do.

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
  `Move a real telescope to the session's target and start an imaging run there, as POST /api/imaging/run does: GoTo, tracking, a first imaging frame, a focus check, centring on the imaging camera's sensor, and stacking until it is stopped or reaches a limit. Set the target first with set_target. Answers at once with the run in state slewing; read get_imaging_run every few seconds until state is ended. In state focus_pause the run waits up to 15 minutes for continue_imaging_run.`,
  with `input_schema` from `schema_for_input::<ImagingRunArguments>()`, as `set_target`
  declares its own (`crates/finder/src/mcp/tools.rs:354`). It takes `arguments:
  JsonObject`. In this order: `self.state.limit_move()?`
  (`crates/finder/src/api.rs:305`); the arguments parsed as `RunRequest` the way
  `set_target` parses its edit, a parse error being the tool's error; no runner, the
  error `this console runs no imaging runs`; then
  `api::start_motion(&self.state, "mcp", "start_imaging_run", || …)` around
  `runner.start(request, By::Mcp)`, as `goto_target` wraps its start
  (`crates/finder/src/mcp/tools.rs:216`); it answers `field_result("run", …)` of
  `runner.view()` read once `start_motion` has answered, and writes that same
  `{"run": …}` object, or the refusal's words, after `->` in its call-log line. It
  writes one call-log line with
  `self.state.log_call("start_imaging_run", …)` whatever the result
  (`crates/finder/src/api.rs:322`).
- `stop_imaging_run`, description
  `End the imaging run and save its stack, as DELETE /api/imaging/run does. It stops the mount only while the run still slews: call stop for that. Answers once the run has ended, which takes the stack's save and, during a re-centre, up to about 7 s more; with no run on it answers the last run unchanged.`:
  takes `arguments: JsonObject`; any key is the error
  `stop_imaging_run takes no arguments`; no runner, the same error as above; otherwise
  `runner.stop().await`.
- `continue_imaging_run`, description
  `Go on from a focus pause of the imaging run, as POST /api/imaging/run/continue does: with anyway false, focus is checked again; with anyway true, the run goes on unchecked. Only valid in state focus_pause. An agent with nobody at the focuser should ask the observer, or pass anyway true.`,
  with `input_schema` from `schema_for_input::<ImagingRunContinueArguments>()`: takes
  `arguments: JsonObject` and parses them itself with `serde_json::from_value`, so a
  missing or extra key is a tool error with serde's words and not a protocol error; no
  runner, the same error; a `Refusal` of `runner.resume(anyway)` is the error.
- `get_imaging_run`, read-only (`annotations(read_only_hint = true)`), description
  `Read the imaging run: state (focus_pause needs continue_imaging_run; ended is final), ended.reason and ended.message once over, warnings, the focus verdict, the centring offset, counts of subs stacked and dropped by reason, limit_at for when the time limit ends it, and its message for the observer. null before the first run.`:
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
| `DELETE /api/imaging/run` | No body. End the imaging run as `stopped` and save its stack; answers 200 with the imaging view once it has ended. It stops the mount only while the run still slews: `POST /api/stop` stops it at any time, and ends the run too. Same-origin writes only. |
| `POST /api/imaging/run/continue` | JSON `{"anyway":false}` or `{"anyway":true}`. From a focus pause, check focus again, or go on unchecked; 409 with a JSON `error` when no run is paused. Same-origin writes only. |
```

In `## MCP clients` (`crates/finder/README.md:57`): four lines after `get_stack`'s, the
last 035 adds, worded like their neighbours, the first saying that `start_imaging_run`
moves the telescope, is rate-limited and is logged; and the count in the sentence
before the list made four higher, in the words that sentence has on `main` then.

A section `## An imaging run`, after the last imaging section then there. In prose: what
a run does, in order; that the imaging camera is held throughout, by the run, the focus
aid or the stack in turn, so a single imaging frame, the focus aid, a set of darks or
flats and a stack asked for meanwhile are refused with the holder's name; that it moves
the mount only by asking Track, and stops it only when the run is itself stopped while
it still slews, so after a run Track is still on
unless a guard or Stop ended the motion; a table of the ten states with their words
between backticks and one line each; a table of the seventeen endings with their words
between backticks, what each means, what the mount is doing afterwards and, in a
fourth column headed `What to do next`, what to do next (start again; check focus,
cloud or the aim point first; call `stop`; nothing), the row of `stopped` giving its
four sentences and what the mount does after each (the run's own
stop: Track on; the console's Stop, or a stop of the run while it slewed: at rest; the
stack or the focus check stopped from its own route, or an aborted sub: Track on), the
row of `mount_failed` saying that Track or GoTo ended for a reason of its own, a
guard's among them, whose words the message carries, and that after a re-centre
that Track refused Track is off, the row of `target_changed` that
Track is off when the target was changed during a re-centre, and the rows of
`link_lost` and `camera_failed` that a link lost while stacking may read either; the
limits
(`subs`, `duration_s` and its 7200 s default, counted on the clock from the start of
stacking) and the counts and waits (three imaging frames, 5 s apart; three solves;
three centring tries, and four solves in a row that measure no new aim point; 10
minutes of stacking with failed subs and none stacked, a pause by the tracker and subs
the console was too far behind to process not counting; 15 minutes at a focus pause;
5 s for the finder camera after a re-centre's Track off; before every imaging frame of
its own, a tracker steady for 10 s or the exposure, whichever is longer; 10 s for a
push by hand to spring back); that the imaging exposure should be set at least as
long as the finder camera's exposure, so that a finder solve falls inside each
imaging frame, without which a solve can measure no aim point; the centring
tolerance, and that a target left within a quarter of the frame's shorter side after
three tries, or after four solves in a row that measured no new aim point, is stacked
with a warning, which says that the stack's steering will bring it in when it is within
3′ of the centre, and otherwise that it is stacked where it is; that the run's own
imaging frames are taken at the
camera's exposure setting and only the stack's subs are capped by the tracker's limit,
so that a long setting on an alt-az mount trails the corner stars of the imaging frames
the run solves; the three routes and the four tools between backticks; where
`run.json` lies and its keys; what `keep_subs` false removes, and that it removes
nothing when no sub was stacked or the stack ended by itself; that the stacked picture
is written only when the stack ends, so that a console that exits mid-run leaves the
subs in `subs/`, a `run.json` with no `ended` that is at most a minute old, and no
picture, and that the page then says the last imaging run did not finish; that the
stack's folder holds a `.reserve` file while the run stacks, as large as the stacked
picture's files can be, about 260 MB for the worked camera's full frame, which the run
removes before it saves the stack; that an
exposure somebody aborts is a failed take of the run's own, and ends the stack and so
the run as `stopped` when it was a sub; and what `POST /api/stop`
and the MCP `stop` do to a run.

Then `### Field checklist: the first imaging run`, exactly eighteen numbered steps, each
one or two sentences with what to write down (US11-S2 reads the word `corner` in the
text from the line that begins `8.` to the line that begins `9.`):

1. Before dark: the imaging camera on the focuser, the scope node and the console
   running, and the Imaging page showing the camera as ready.
2. After dark: pair, set the site, and wait for the first sync; write down the model's
   RMS.
3. Choose a bright target well above the minimum altitude. Turn Track on for it and
   leave it for 30 minutes; write down whether Track ended and with what reason,
   since an imaging run cannot outlast Track.
4. Set the imaging exposure at least as long as the finder camera's exposure, so that
   a finder solve falls inside each imaging frame. Take one imaging frame by hand and
   solve it by hand; write down how long the imaging frame took to arrive, the scale
   and the time the solve took.
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
11. Start a second run and press the top bar's Stop while it still slews; check that
    the mount is at rest and stays there, and that the run ended `stopped`.
12. Start a third run. While it stacks, tap the tube lightly; write down whether the run
    went on and what the page said.
13. Still in the third run, hold a hand over the front of the tube for a minute; write
    down the subs dropped by reason and whether the run went on.
14. Press the top bar's Stop while the third run stacks; check that the mount is at rest
    and stays there, and that the run ended `stopped`.
15. From an MCP client: `set_target`, then `start_imaging_run` with a time limit of 300
    s; read `get_imaging_run` until it ends; check that the page said who started it.
16. Defocus the telescope on purpose and start a fifth run from the MCP client. At the
    focus pause, refocus and go on with `continue_imaging_run` and `anyway` false;
    write down the verdict before and after.
17. End the fifth run with `stop_imaging_run` while it stacks; check that Track is still
    on.
18. Copy the five `run.json` files and this filled-in list into the night's field note.

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
  and `set`, and their refusals, are 031's and unchanged; so is the lease (031 US9).
- **032's solve and relation, 033's check and aid, 035's stack, 036's steering.** The
  parts call them as their plans give them, and change none.
- **The scripted tests of earlier stories, for the later ones.** A test file an earlier
  story of this spec created is an existing test, and no later story may edit it.
  `crates/finder/tests/console/imaging_run.rs` and the scripted modules after it build
  `Steps` by `script.steps()` and read `RunView` field by field. So no story adds a
  method to a part's trait or a field to `Steps`, changes `Runner::new`, `start`,
  `stop`, `resume` or `view`, or writes the script after US12. Each story changes what
  the task does between two states, and what it replaces is asserted by nothing before
  it:
  - US1 adds the view and the `Runner` to `run.rs` after what US12 wrote, and changes
    no item of it: no trait, no type, no reason and no sentence of an ending, and
    nothing of the script. US12's four scenarios make no run, so they hold whatever
    a later story adds to the machine; no later story changes an ending's reason or
    sentence, a call word or a default of the script either.
  - US2 replaces what US1 does with a failed take, solve or `stack.start`, an ending at
    once, and reads the stack's progress. No US1 scenario fails a take, a solve or a
    stack start, or sets a count; the one that fails a focus check (US1-S2) gives it
    `Failure::Other`, which US2 leaves as US1 has it; the zero counts of US1-S1 are the
    script's defaults. US2 also adds the words of a wait for the tracker to the
    messages of four states, and the message of an ending in progress: US1's scenarios
    read a message only where the run does not wait for the tracker after `tracking`
    and only before or after an ending, never during one. The same holds for
    US2-S6 to US2-S10, which were US1's scenarios until 2026-11-19 and assert the
    machine as US1 leaves it: their tests pass before US2's code is written and
    after it.
  - US3 adds `Runner::with_folder` and leaves `Runner::new` writing nothing.
  - US5 adds the centring loop. The script's default offset of 0″ never enters it, and
    US4-S1 asserts its solved offset inside the tolerance, so its run enters none
    either; US4-S5's `repoint` on the part alone gains only a wait of at most 5 s.
  - US6 replaces the going-on of a verdict that pauses and of a check that did not
    finish. One earlier scenario has such a run, US1-S2's bullet for a check that did
    not finish, and it reads only `focus` and `warnings`, which US6 leaves as they are;
    the refusals of `resume` outside a pause keep their words (US3-S4).
  - US7 adds the limits and the stack's own endings. The default request has no count
    and 7200 s, which no earlier scenario stacks for; no earlier scenario stacks for
    600 s with a dropped sub; and the script's stack runs until
    it is stopped or `end_stack` is called, whatever `set_stack` gave it (US2-S3,
    US3-S3). That last point is why `set_stack` does not set `running` (§ The script).
- **Finder's test layout.** `crates/finder/tests/layout/main.rs` fails a file at the top
  level of `crates/finder/tests` (`crates/finder/tests/layout/main.rs:109`) and any file
  added to `crates/finder/tests/support` (`crates/finder/tests/layout/main.rs:127`).
- **031's Imaging page tests, and those 032 to 036 add.** With a `ready` view and no run
  the block adds one region, two number fields, one checkbox and one button, and no
  `status` or `alert`. Probe round 1 confirmed that the earlier page tests find
  their own controls by name (§ Prerequisites 5).
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
| The traits and the script are complete from US12, and the machine grows by story | The whole machine in US1; a trait or a script that grows | No scripted test implements a part itself, so later stories never touch an earlier test; a machine written whole would put the retries, the focus pause, the centring loop and the limits in one story with no real part to prove them against, well over 64 KiB. |
| A scripted stand-in in the library | A copy in each of nine test modules; a new support file | Spec owner decision 16. |
| The lease is two calls of the camera part, `hold` and `release` | A sixth part; a lock in the `Runner` | The plan document's amendment 1: one lease, 031's, and no lock of the run's own. The camera part is where the run meets 031. |
| The lease is taken before Track is asked | After the mount has arrived | A run that cannot expose should not move the telescope. |
| The run holds the lease while it slews, exposes, solves and centres, and hands it over to the focus check and the stack | The run holding it for its whole length; the run taking none | Spec owner decision 15: 033's `focus_start` and 035's `Stacker::start` take the lease themselves, under their own names, so a run that kept it would refuse its own steps. Taking none would let a single imaging frame or a set of darks in while the run slews, solves or re-centres. |
| `camera.release()` is called at every ending whether or not the lease is held | Tracking in the machine whether it is held | One rule a test can read off the call list: the last call of every run gives the camera back. |
| A first imaging frame, then 033's own focus check, then an imaging frame for the solve | Measuring focus on the first imaging frame; no first imaging frame | Spec owner decision 20: the plan document's order, with 033's check as its draft gives it. `Imaging::solve` solves the latest imaging frame, which after a check is one of the check's. |
| The run's view rides in the imaging view, in every state | A top-level `run` in the session; only while `ready` | The plan document puts `run` in `imaging`. A run that ended because the link was lost must still be readable; 035's `stack` does the same. |
| The run's record lies in the stack's folder, and a run that never stacks makes its own | The run makes the folder and tells the stack to save there | 035's `Stacker::start` names its folder and takes none (spec owner decision 23). One folder then holds the stack and what the run did. |
| `run.json` rewritten whole at each change of state | JSON Lines of events | It is small, changes about ten times and after that at most twice a minute (§ The record), and one whole file is what the field note copies. |
| Subs that are not kept are removed when the stack has ended, from `stack.json` | 035 removing each as it goes | Spec owner decision 22: 035's draft keeps every sub and lists it; removing by its list needs no change to 035. |
| An ending asks nothing of the mount, but a stop of the run while it still slews, which asks for Stop | Stop at every ending; no Stop at any | Spec owner decision 1: Track's own guards keep watching, and the target stays in view. Before anything was exposed there is no target in view to keep, and a run that says it stopped while the tube still swings misleads (spec owner decision 14, as changed on 2026-11-19). |
| The mount part reads steady only when 036's tracker was steady for the whole of 032's fit reach (`steady_between`) | Round 1's fixed wait of 10 s from the first reading of a working Track; no wait | A fixed wait starts at the wrong moment, since the tracker is not steady when Track first works, leaves out exposures longer than 10 s, is a clock inside a getter, and does not start again after a keyhole pass or a take-up. 036 already keeps the list that answers the question. The lasting fix is in 032 (§ Risks). |
| A push by hand ends the run when it has settled or lasted `PUSH_GRACE` | Ending on the first `moving` reading | 038 itself forgives a push that springs back; the first reading is one live pointing (spec owner decision 27, as changed on 2026-11-19). |
| Subs that keep failing end the run by time, 600 s of stacking with a failed sub and none stacked | A count of ten subs; a page setting for the minutes | A count is under a minute of cloud at short subs and ten minutes at long ones. The owner took the constant and no setting (spec owner decision 9, as changed on 2026-11-19). |
| Four endings share the reason `stopped`, each with its own sentence | One sentence, `Stopped by the observer.` | The one sentence covered a run stopped with Track left on, a mount at rest, an MCP client's stop and a stack or a focus check stopped from its own route; the observer could not tell whether the telescope still moved. The reason word stays one, so `ENDINGS` and every client's switch on it are unchanged. |
| The offset is the separation of the solve's centre from the target | The target's pixel in the imaging frame | It needs no pixel, so it works when the target is outside the imaging frame, and it uses the same great-circle function as Track's 1′ test. |
| A region of interest refuses a start | Measuring at the sensor's centre through the solve's map | Spec owner decision 24: 032 takes aim readings from whole-sensor imaging frames only, so such a run could not re-centre. |
| The tolerance has a floor of 90″ | A share of the imaging frame alone | GoTo arrives at 60″ (`crates/finder/src/goto.rs:1013`); on a small or long-focus imaging frame a share alone would ask for less than GoTo can give and never be met. |
| A solve that did not move the aim point is retried without a re-centre and without counting a try, four in a row ending the wait (a target near the centre is then stacked, and beyond that the run ends `not_centred`), and whether it moved is read from the session, an aim point it holds back counting as moved | Re-centre anyway; counting it as a try, as this plan did until 2026-11-19; deciding by 032's outcome word | The aim point in force did not change and none is held back, so Track off and on would bring the mount where it already is, and the restart would spoil the next aim reading. 032's outcome `measured` does not say the aim point moved: 032 leaves an aim point alone when its median is within 10″ of it (032 plan.md § The relation). |
| `repoint`, `stack.start`, `stack.stop`, the aid's open and close and the Stop of a run stopped while it slews are awaited unguarded | Guarded like the other steps; the runner's stop signal handed to `repoint` | Dropping one half-way would leave Track off, or a stack or a focus aid running, with the run already ended. A stop of the run therefore waits for them: for a re-centre about 7 s (the 5 s of its wait, Track's own 2 s for the axes to rest, one Track off and one Track on), after which Track is on, as after any stop of the run that has exposed (spec FR-005, owner decision 14). Handing the signal in would add a parameter to a trait for a wait that short. |
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
| Twelve stories: the machine, the centring and the stacking each in two, and the machine's parts, words and script ahead of it (the next row but one) | Eight stories, split after the probe if one passes 64 KiB | A split after dispatch is a changed spec and a relaunch (§ Plan traps 5). US1 and US2 are cut where every step answers against where one fails; US4 and US5 where the run points against where it points again; US7 and US8 where the script suffices against where the simulator is needed. |
| US1's tests of a refused start, a mount that is not steady, a stop, the mount's endings and a dropped runner are written by US2, in US2's test module | Leaving them in US1; a story of its own for the script | US1 was 93.5 % of 64 KiB as probed, and the owner's answers of 2026-11-19 added code to it. Tests can move with no change to the Work Graph, where a new story cannot (spec § Clarifications, the owner's answer 16; § Risks). The move left US1 at 86 %, and the owner then split it as well (the next row). |
| The run's parts, its words and its script are a story of their own, US12, built before the machine; the view's types and the words made from a run's state stay with the machine, in US1 | US1 as one story of about 56 KB; the script alone as a story; every word and every type in US12, the state's messages and the view among them | The owner's split of 2026-11-19: US1 was estimated at 86 % of 64 KiB with about 50 KB of code, and factory builds have run to 1.35 times their probe build. The cut is by what can be proven without a run. An ending's reason and sentence, the request, `run_id` and the script's contract need none, and US12 asserts them directly. A state's message, a focus sentence and a warning are made from a run's own state, and `RunView` is `#[non_exhaustive]`, so a test outside the crate cannot build one: with those in US12 it would hold code its own tests could not reach, or new public functions written only to be tested, and would be estimated at about 55 KB, 84 %, against 66 % as cut. The script alone would have left the machine with every type, at about 44 KB. The story has the number 12 so that no other id moves, as 035's US11 has. |
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

1. **The earlier specs have landed.** Met for `014-goto`, `015-tracking`, `016-mcp`
   and `017-console-stop-confirm`, and, by 2026-11-19, for `030-imaging-camera-link`,
   `031-imaging-frames-on-console`, `032-imaging-solve-and-aim-point`,
   `033-focus-reading-and-aid`, `034-dark-and-flat-frames` and
   `038-track-follows-a-push`, whose `push` the `pushed_away` ending reads. Not met for
   `035-live-stack`, which is flipped and being built, and `036-tracking-from-subs`, a
   draft beside this one.
2. **This trio is re-anchored to its seven predecessors after they land.** Not met. Every
   name in § What 030 to 036 offer is re-read on `main`, and the last story of each
   spec that writes a shared file replaces the one named in § Refinement Record. 030, 031,
   032 and 034 were reviewed and fixed before this text was written; 033, 035 and 036
   were drafts then, and have since been reviewed too. Where one of them lands
   offering something else than the table says, the part that calls it changes, and where it offers nothing that a part needs, the
   gap goes to the owner before the flip. Thirteen points are known to need that
   reading, and a fourteenth is closed:
   - 033 writes its focus check as taking its own exposures and restoring the settings
     afterwards; the run's first imaging frame and the imaging frame for the solve rest
     on that (spec owner decision 20);
   - 035's `Stacker::start` names its own folder, keeps every sub, and has no limit;
     the record's place and the removal of subs rest on that (spec owner decisions 22
     and 23);
   - 035's view ends with `stopped` or `camera` and a `save_error`; if it gains other
     reasons, `StackProgress::failed` sorts them;
   - 035's `Stacker::stop` is safe on a stack that has already ended: the machine asks
     the stack part to stop at every ending after a start, whoever ended the stack
     (§ The machine, Ending), and the part calls `Stacker::stop` only while its own
     stack still reads `stacking`, so the point matters for a stack that ends between
     that read and the call (§ The console's stack part);
   - 035 writes, when a stack ends, `stack.fits` as three planes of 32-bit reals,
     `stack.png` as an 8-bit RGB picture of the stack's full size, and `stack.json`
     again whole; the stack part's reserve is sized to those three (§ The console's
     stack part);
   - closed: the rule for a folder's name is written once. 035 offers it as the pure
     function `crate::imaging::stack::service::label`, which its `AppState::target_label`
     calls, and `run::label` calls it too (§ The parts and the words); US12-S2 holds it
     to the worked
     example `NGC 1976` → `NGC-1976`. The re-anchoring re-reads only the function's path;
   - 036's `Steering::waiting` is true only inside 035's loop; `paused_by_tracker` rests
     on it. It is also true while the stack waits for its exposure limit to rise, with
     `steering.pause` `limit` and the tracker steady (036 FR-022), so the run's sentence
     for `paused_by_tracker` names no cause, and the run does not read
     `steering.pause` (§ The run's words);
   - 036's `Steered::steady`, read through `Session::steered`, is false while a tracker
     settles after a start; the run's
     first exposure waits on it;
   - 036's tracker is not steady for at least 60 s around M42's transit on the
     simulator; US4-S4's unsteady bullet rests on it;
   - 036's `Steered::steady_between` is false from a tracker's start until 2 s after its
     first rates, across every take-up and keyhole pass with the 2 s that follow each,
     across a measurement, while 038's push reads `moving`, and for a turn that finds an
     axis stopped, over a window of console Unix seconds; a window that begins at the
     instant such a time ended is not touched by it (036 FR-021). 036's US10-S3 pins the
     window the mount part asks, ending now and 10 s long: false asked 6 s after a
     tracker's start and true asked 14 s after it. The mount part's `steady` and the
     repeat of an unsteady focus check rest on it;
   - 035's `StackView::started_at` is set once, at the start that made the stack, and
     035's `start` on a running stack answers `Ok` with that stack's view; the stack
     part tells its own stack from another's by it;
   - 035's `dropped` holds `behind` and `unprocessed` for subs the console did not
     get to, and no other reason means that; the rule for subs that keep failing
     leaves those two out;
   - 038's `push` reads `moving` from the first live pointing more than 5′ off and
     `following` after 5 s of stillness, and is absent otherwise; `pushed_away` and
     its 10 s rest on it. 036's tracker reads not steady while a push is `moving` (036
     FR-021, US6-S6);
   - 036's `crate::imaging::steering::FAR_ARCSEC` is public and 180.0, and its steering
     stands back from a stack whose first tracking error is longer (036 FR-014); the two
     forms of the warnings for a target stacked off the centre rest on it. 036's list of
     what this spec reads names the view's `steering.far_arcsec` and not the constant,
     so the re-anchoring reads the constant's path as it reads `label`'s;
   - 036's `Steering::set_set_point` lets go a tracker a stack holds when the set point
     changes (036 FR-014); the stack part calls it before the stack's start, where it
     changes nothing a stack holds;
   - 032's task sets a measured aim point in the session before it shows a solve as
     `solved`, and sets it only when `should_apply` says so; while Track is on it
     holds it back instead, `Session::held_aim` answers it, and the first live solve
     the session records with Track off applies it before that pointing is taken
     (032's FR-036). `aim_moved` and the wait of `repoint`'s step 3 rest on all of it
     (§ The console's camera and solver parts, § The console's mount part).
3. **The lease of the imaging camera exists.** Met in the drafts, by the audit round in
   which this trio was revised. The plan document's amendment 1 gives 031's imaging
   state one lease that names its holder, and says each later spec takes and gives it
   back and adds no lock of its own. 031's trio gains it as its US9 (the audit's
   finding A1 on 031), and 034 takes it for a set of darks or flats in place of a check
   of its own (the audit's finding A1 on 034), as 033 and 035 do for the focus aid and
   the stack; 033's plan already words the refusal as this trio does. 031's plan.md
   § The camera lease now writes US9 out, and the five things below hold there; this
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
   solver parts). US1-S2, US2-S10, US4-S2, US4-S4, US4-S6, US6-S7 and
   US8-S8 are the scenarios that rest on the lease. Two state its refusal word for word:
   `the imaging camera is held by the imaging run` (US4-S4) and
   `the imaging camera is held by the focus aid` (US6-S7).
4. **The owner decisions are settled and the approvals given** (spec § Owner decisions,
   § Owner approvals). Not met: none of the first twenty-five is settled as a
   whole. On 2026-11-19 the owner answered the seventeen choices of the reanalysis
   (spec.md § Clarifications), which changed decisions 1, 3, 4, 7, 9, 12, 14, 22 and
   27; each carries a dated note. Decision 26,
   the tracker and a newly measured aim point, is not this trio's: it cites 032's owner
   decision 25, which the owner settles in 032's trio, because the behaviour is
   reachable from 032 on, with or without a run (§ The console's mount part, "The
   tracker is handed no step"). Met for decision 26: the owner settled 032's decision
   25 on 2026-11-17 with the hold, which 032 writes as its FR-036 and US4-S8, and this
   trio is written for it. Every aim reading of a run is taken while Track is on and
   held back, so FR-022's test counts a held aim point as moved and FR-025's wait comes
   after the re-centre's Track off, whose first live pointing puts it in force; US5-S4,
   SC-004 and the whole runs of US8-S1, US8-S2 and US8-S8 rest on both. This trio is
   re-validated with 032's before the flip.
5. **A probe build of every story.** Met on 2026-11-19 by probe round 1, for the trio
   as it then stood. Four agents built all eleven stories from it, tests first and with
   both gates, each starting where the one before stopped (US1 to US3, US4 to US6, US7
   and US8, US9 to US11), on `main`'s 030 to 034 and 038 and on probe builds of 035's
   flipped trio and of 036's folded draft (spec.md § Clarifications, the session of
   round 1 probes and the reanalysis). Every story built and passed both gates, two of
   them only with a marked deviation (D13 and D14 below), and each wrote only its
   declared files; every bound but those two held, on three runs each. What the round
   settled:
   - each story's diff against 64 KiB (spec SC-007): US1 61,307 B, 93.5 %; US2 17,566;
     US3 40,205; US4 32,441; US5 23,300; US6 31,886; US7 11,298; US8 20,250; US9
     25,710; US10 20,747; US11 16,425. All were under, and US1 alone was above 85 %.
     The fold of 2026-11-19 added code and cases to every story, so § Risks carries
     the figures as they are estimated now;
   - US4-S1 leaves the target 4.43″ from the centre by the truth with the true aim
     point set, against its 110″, and its solved offset is 0.07″ from the truth,
     against 30″;
   - US5-S4 centres in one try, 8.67″ by the truth, with the aim point 0.017 and 0.009
     pixels from the true one, against 2.0. It did so only once the mount part read
     steady no sooner than 032's fit reach after Track first worked: an imaging
     exposure started 8.2 s after `tracking` read `moving` and took no aim reading,
     and one started 9.2 s after it read `measured` (D14, blocking). The round built
     that as a fixed wait of 10 s; the fold wrote `steady_between` in its place (the
     owner's answer 2), which no probe has run;
   - the simulator's mount did not move toward the newly measured aim point between
     the first imaging solve and the re-centre's Track off (0.00″), the pointing
     model's slip was not marked then, and the first live pointing after that Track
     off came 0.30 s later, against the 5 s of `repoint`'s step 3;
   - the simulated axes are at rest by the time a re-centre's Track asks GoTo, and
     Track itself waits up to 2 s for that; GoTo did not refuse (§ Risks);
   - US6-S4's pause ended at 900.000 s and US7-S2's limit at 60.000 s, each on a poll.
     Their reads were a second either side, and a limit a second short survived
     them, so both now read half a second before and after (D22, D25). US3-S6's two
     runs took 200 ms of their 950 ms;
   - `Imaging` builds its view in one place, `Imaging::view`, which `Imaging::solve`
     answers on `main` too; `run` is set there in every state, and no part was reached
     under the runner's lock (§ Plan traps 23);
   - a console dropped while a run of its own five parts stacks has its mount at rest
     within 0.05 s and lets its session go 2.95 s after the drop, all of that time
     035's stack's (§ Risks). US2-S9 proves the runner's half on the script;
   - the run rig syncs, slews and solves at dawn: US4-S4's run stacked for 31 s before
     `sun_up`;
   - US8-S1 reached three stacked subs on the simulator with 036's steering in the
     loop, none dropped, on all three runs;
   - the earlier page tests still find their controls: all 380 earlier Vitest tests
     passed unchanged with the block on the page, and so did the Playwright suite,
     031's night check of the Imaging page among it;
   - two more defects blocked a story as written: US4-S5 read rest the instant after a
     Stop, while the axes still braked (D13: it now reads from 2 s after), and US10's
     continue read the tool's own answer, which is still `focus_pause` (D30: it is
     now read back with `get_imaging_run` after a settle, US10-S8).

   Not settled, and for the second probe round the owner asked for (spec.md
   § Clarifications, the owner's answer 17): everything the fold of 2026-11-19 added,
   none of which has been run (§ Refinement Record, "What probe round 1 and the
   reanalysis changed"); each story's diff on the folded trio, US12's and US1's first,
   because the owner's split of 2026-11-19 came after the round and no probe has built
   either as it now is (§ Risks); what 036's own fold changed underneath this trio the
   same day (§ Refinement Record, "036's second fold"): the two forms of the warnings in
   US5-S3, and every wait for a steady tracker 2 s longer on the simulator, inside which
   US4-S5's 900 s and 120 s must still hold;
   and the margin of US4-S4's "within 60 s" for `pushed_away`, the one bound round 1
   did not time.
6. **The plan document, ADR-0002, the glossary's imaging terms and the research note
   are on `main`.** Met by 2026-11-19: all four are on `main`.
7. **`main` is still as read.** Met on 2026-11-15: every `path:line` here was read at
   49bb210, and read again at 49bb210 after the audit, when `main` had not moved; no
   cited line had moved, and `crates/finder/src/targets.rs:39` replaced a citation one
   line above the `struct` it names. It is re-read at the flip, because 030 to 036
   change most of the files cited.
8. **The real mount.** Not met, and outside the factory: a GoTo to a target and Track
   have not run on the real mount (the plan document, § Before any flip, 4). The spec
   can be built before that; the run is not used at the telescope until it is. The
   field checklist's step 3 leaves Track alone on the target for 30 minutes before the
   first run: a night at the telescope logged a garbled reply from the mount every
   few minutes, and if one of those ends Track, no run outlasts it.
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
   `Store`; and no test builds an expected message by calling `Ending::message`: US12-S1
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
10. **The view outside `ready`.** 031's view holds only its `state` then. *Check:* `run`
    is set where the view is built, after the state's own parts, in every state;
    US3-S5 reads it under `no_link`.
11. **Paused time and the blocking pool.** Tokio's paused clock runs on while a blocking
    task works. *Check:* `Store`'s writes and reads, and the stack part's removals and
    the writes of its reserve, are synchronous `std::fs`, the reserve written a chunk at
    a time with a yield between two (§ The console's stack part), and no file of this
    spec calls `spawn_blocking`.
12. **A held step must be listed first.** A test that holds `take` and then reads
    `calls()` expects `camera.take` there. *Check:* the script lists a call when it is
    made and holds after.
13. **The rate limit in tool tests.** Two `start_imaging_run` calls in one virtual
    second are both counted, refused or not. *Check:* the tests of US10 leave 1 s
    between starts, except US10-S4.
14. **`useImaging` reads every second.** A draft typed into a field would be lost on
    each read. *Check:* the block keeps its fields in its own state; US9-S1 types,
    renders the block again with a new view, then clicks.
15. **The bundle must be rebuilt.** *Check:* US9 commits `crates/finder/web/dist` after
    `npm run build`.
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
    `Runner`'s `Drop` aborts the task (US2-S9).
20. **The lease must come back, and must not be kept across a hand-over.** A run that
    ends by any path, a refused `point` included, would otherwise lock the imaging
    camera until the console restarts; a run that still held the lease when it called
    033's check or 035's stack would refuse its own step. *Check:* `camera.release()`
    is the last call of every ending and of a refused `point`, and comes before the
    first `focus.check` and before `stack.start`; `camera.hold()` comes before the first
    take after the focus gate. US1-S1, US1-S2, US2-S7, US2-S8, US2-S10 and US6-S11
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
    lets it go, calls the part, and locks again to record the answer (§ The machine).
24. **A later rule must not break an earlier story's test.** A story may not edit a test
    file an earlier story created. *Check:* the script's `set_stack` leaves `running`
    and `failed` alone and only `end_stack` ends a stack (§ The script); US4-S1 asserts
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
| 033, 035 or 036 lands offering something other than its draft. | The run touches them in three small parts and nowhere else; the states, routes, page and tools do not move. § Prerequisites 2 re-anchors, and a gap goes to the owner. |
| 031 lands the lease under other names, words or rules than its US9 gives. | § Prerequisites 3 lists the five points. Only the camera part's `hold`, `release` and `take` touch it, and seven scenarios, two of which state its refusal word for word. |
| Another taker gets the lease in the instant of a hand-over. | The run's next step is refused with that holder's name and the run says so: a pause, `stack_failed` or `camera_failed` (spec § Edge Cases, US1-S2, US2-S4). Spec owner decision 15 names the other answer, a change to 033 and 035. |
| The simulator leaves the target just outside 90″, with the true aim point set or after a re-centre, or needs more than three tries. | Probed in round 1 (§ Prerequisites 5): 4.43″ by the truth with the true aim point set, and 8.67″ after one try with none. US4-S1 and US5-S4 assert the bounds against the truth, and owner decisions 2 and 3 are where the numbers change. The steady rule the fold wrote in the place of round 1's wait has not been run. |
| A story's diff passes 64 KiB. | Measured in probe round 1 (§ Prerequisites 5): all eleven were under, US1 at 61,307 B, 93.5 %, and no other above 62 %. The fold of 2026-11-19 added code and cases to every story, and the owner then split US1, so what follows is estimated from the round's measured bytes, each within about 1 KB but for the two halves of the split, which are within about 2 KB: US12 43.4 KB (66 %), US1 30.0 KB (46 %), US2 35.2 KB (54 %), US3 47.1 KB (72 %), US4 40.9 KB (62 %), US5 29.1 KB (44 %), US6 37.4 KB (57 %), US7 14.8 KB (23 %), US8 28.2 KB (43 %), US9 37.7 KB (58 %), US10 23.9 KB (36 %) and US11 20.3 KB (31 %). **No story is estimated above 72 %.** Before the split US1 was the one story above 85 %, at about 56.4 KB, even after US1-S3 to US1-S6 and the start's refusals of US1-S2, about 12.5 KB of tests, had become US2-S6 to US2-S10 (spec.md § Clarifications, the owner's answer 16, widened by the fold): what was left was about 50 KB of code and two scenarios. The split (spec.md § Clarifications, the last question of the session of 2026-11-19) gives US12 the parts' traits, the endings' words and the request, about 15.8 KB as round 1 built them with what the fold added, the script, about 12.4 KB, and four new scenarios that make no run, about 14.4 KB of tests; and leaves US1 the view and the machine, about 23.4 KB, and its two scenarios, about 6.2 KB. The two halves come to about 17 KB more than US1 did, all of it US12's new tests. A factory build has run to 1.35 times its probe build: at that US12 would be about 89 % and US3 about 97 %, so US3 is the next story to cut, and after it US4, whose scenario of the guards (US4-S4) could become a story of its own. The second probe round measures all of it; US12 and US1 as they now are have been built by none. |
| US1's refusals of a start, its stop, the mount's endings and its `Drop` are written in US1 and first tested in US2 (US2-S6 to US2-S10). | A fault in `run.rs` that one of those tests shows is US2's to correct, since US2 writes that file. A fault in `script.rs` is not: no story after US12 writes it, so US2 escalates. US12's own scenarios assert every control of the script with no run (US12-S3, US12-S4), which is what keeps that unlikely. |
| US12 writes public items that nothing in the crate calls until US1 lands: the traits, `Ending::message`, the script. | They are public items of a library, which the lints do not count as dead code, and US12's test module calls each of them. Not built: the second probe round builds US12 by itself, with both gates. |
| A Stop that lands while a re-centre's Track is being asked is covered by step 5 of `repoint` alone, which no test can reach by itself on the simulator. | Steps 1 and 3 cover the windows either side, and US5-S5 catches two of the three removed together. The field checklist's steps 11 and 14 press Stop during a run. |
| 032's aim reading fits the finder camera's solves from 10 s, or the exposure when that is longer, before the imaging exposure, so an imaging frame taken sooner than that after a GoTo or a re-centre reads `moving` and gives no aim reading. Round 1 saw it after every re-centre. | The mount part reads steady only when 036's tracker was steady for that whole look-back (`steady_between`, spec FR-019). The cost is the larger of 10 s and the exposure, after the 2 s to 3 s for which 036's tracker settles, after each GoTo and each re-centre. Not probed: round 1 built a fixed wait of 10 s instead, and no run had an exposure above 1 s; US4-S1's bullet and US5-S6, at 20 s, are for the second probe round. The lasting fix is a small follow-up to 032, which has landed: drop from the fit every finder solve older than the session's last end of a GoTo or start of Track. When it lands the look-back shrinks to `steady()`. |
| A solve measures no new aim point, again and again: the finder camera's exposure is longer than the imaging camera's, so no finder solve falls inside the imaging frame, or 032 sets readings aside after a camera was re-seated. | Such a solve is not a try; after four in a row the run stacks a target that is near the centre, with a warning, and ends `not_centred` with their own sentence beyond that (spec FR-024). The README and step 4 of the field checklist say to set the imaging exposure at least as long as the finder camera's. The real finder's cadence is a field check. |
| A push that reads `moving` for less than 10 s no longer ends the run, and while the run stacks nothing of this spec pauses the stack for it. | 036's tracker reads not steady while the push is `moving`, so the stack waits and the sub in flight is dropped as `not_steady`. Not run: 036's second probe round runs its US6-S6. US4-S4's bullet shows only that the run goes on. |
| The reserve is worked out from the files 035 writes at a stack's end as they are today: 15 bytes a pixel of a sub and a margin of 16 MiB, about 260 MB at the worked camera's full size (`reserve_bytes`; the owner's answer of 2026-11-19, in place of a fixed 64 MiB, which could not hold a full-size `stack.fits` of about 195 MB). | If 035 comes to write more at a stack's end, a fourth plane or a 16-bit picture, the figure for a pixel is one constant, and the re-anchoring reads 035's save again (§ Prerequisites 2). A `stack.json` that lists more than about fifty thousand subs outgrows the margin. The reserve helps only an ending the run makes: a stack that ends by itself saves while the reserve is still there. It is itself room the subs cannot use, and a disk that cannot hold it ends the run `save_failed` before its first sub. While it is written, 4 MiB at a time with a yield between, a stop of the run waits for `stack.start` to answer, for as long as the disk needs. Not run: the second probe round builds it (US8-S7). |
| A target changed in the instant after a re-centre has asked Track again is not seen by step 3a. | Track then goes on with the old target, the next `state()` reads `target_changed` by its second row, and Track is left on the old target, as outside a re-centre. US5-S7's timing rests on the first live pointing with Track off coming 0.30 s after it on the simulator; not run. |
| With `keep_subs` false, the two conditions under which the stack part removes subs are asserted on the script only, by the warning. | No rig scenario stops a run before its first sub is stacked at a moment a test can fix. The second probe round looks for one. |
| A newly measured aim point under Track: as 032 was first drafted, 015's tracker chased it at a rate nothing bounds, outside GoTo's checks; since 038, a step of more than 5′ is read as a push by hand instead and marks the pointing model's slip. | Closed in 032, where the seam between 032 and 015 lies: the owner settled 032's owner decision 25 on 2026-11-17 with the hold (032's FR-036, US4-S8), which spec owner decision 26 cites, and FR-022 and FR-025 here are written for it (§ Prerequisites 4). Probe round 1 saw that the simulator's mount does not move toward the new aim point before the re-centre's Track off (0.00″) and that the model's slip is not marked (§ Prerequisites 5), and the field checklist has the observer watch the first centring (step 8). |
| The observer's exposure setting is longer than the tracker's limit, and the run's own imaging frames, which 036 does not cap, trail their corner stars. | A known limit (spec § Clarifications; § The console's camera and solver parts). The solve's centre, all the run reads, is not moved; a solve that fails for want of stars is counted (US2-S2). The README says so and step 8 of the field checklist has the observer look. The other answer is a getter in 036 for the capped exposure, an edit to 036. |
| A re-centre asks Track again while the mount still reports motion after Track off; GoTo refuses with `the mount is already moving` and the run ends `mount_failed`. | Probed in 037's round 1: the simulated axes are at rest from the re-centre's Track off on, and `Session::start_tracking` waits up to 2 s for both axes to rest whenever it replaces a tracker, before it asks GoTo; GoTo did not refuse. The field checklist's step 8 still watches the first centring. `stop_tracking` itself waits for the mount's acknowledgement of the stop, not for rest (`crates/finder/src/tracking.rs:219`). If the telescope shows it all the same, `repoint` waits, for a counted number of polls, until the session's mount view no longer says the mount is moving (`crates/finder/src/session.rs:508`) before it asks Track; that reads no axis. |
| A console dropped in the middle of a run keeps Track on through something the imaging state of 030 to 036 holds. | Probed in 037's round 1. A console dropped while a run of its own five parts stacks has its mount at rest within 0.05 s, which stays at rest. Its session is let go 2.95 s after the drop, exactly as when a stack started by hand runs with no run. 035's stack holds it until its take ends. 037 adds no delay of its own. A test that drops a console while a stack runs waits more than 015's 2 s for the session. US2-S9 proves that the runner lets its parts go. Unprobed: a console dropped during a focus pause, which leaves 033's aid, opened by the run, with no owner. In the binary the state lives until exit, so only tests are touched. |
| A stop of the run waits up to about 7 s when it arrives during a re-centre. | Said in spec FR-005 and owner decision 14, and asserted on the script (US5-S3). The console's Stop does not wait: it stops the mount at once, and the run then ends `stopped`. |
| The first imaging frame costs an exposure and a transfer the run does not use again. | Spec owner decision 20 names the two other answers. The first imaging frame is what tells a camera fault from poor focus. |
| Track left on after an ending is forgotten. | Track's own guards end it at the minimum altitude and at sunrise, and the top bar's Stop stays visible while Track is on (`DESIGN.md:308`). Owner decision 1 is where it changes. |
| The kept subs fill the disk. | The default time limit (owner decision 8), the size shown on the start form, `save_failed`, and the reserve the stack part frees before the stack's save. |
| The console exits mid-run and the stacked picture is lost: 035 saves it only when the stack ends. | Said in spec § Edge Cases and the README. `run.json` is refreshed each minute, the subs stay on the disk, and the page says after the restart that the last run did not finish. Saving the stack as it goes is 035's, and is asked of it. |
| Subs that are not kept still fill the disk while the run stacks. | Spec owner decision 22 names the other answer, an edit to 035. |
| A second client stops or restarts the stack under a run. | The lease refuses a second stack; a stop of the run's own stack from 035's route ends the run `stopped` (US7-S4 on the script, US8-S4 on the rig). A stack another client starts in the instant of the hand-over, which 035's `start` joins and answers `Ok` for, is told from the run's own by its `started_at` and ends the run `stack_failed`, with nothing of that stack stopped or removed; that race was not run either. A stack started in the one poll between the run's stack ending and the run's own ending is left running; the run keeps its last counts and its subs (§ The console's stack part). That race was not run. |
| On the simulator no run meets 036's wait (`paused_by_tracker` on the console's own stack part), a stack's `camera` ending, a `save_error` at `stop`, or a dropped sub; and no test can see the set point. | Those five rows of the stack part are asserted on the script only (US2-S3, US2-S4, US7-S4). Round 1's mutations of each survived the rig's tests. |
| An earlier page test queries a control the block also has. | The block's controls have their own names and it shows no `status` or `alert` before a run; probe round 1 ran every earlier page test with the block on the page, and all passed (§ Prerequisites 5). |
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
`crate::imaging::stack::service::label` (A2); the lease is 031's US9, its names no
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

**Until probe round 1 no probe build had been run.** No story of this trio had been
compiled, linted or tested, no interface above had been built, and no diff had been
measured: the sessions before it wrote documents only. Every `path:line` was read at
49bb210. Every number in a scenario was either stated by the scenario as its own oracle
or worked by hand: 1/20 of the
simulated imaging frame's 822 pixels at 2.1845″ a pixel is 89.8″, under the 90″ floor;
the centre (728, 544) of the simulated finder camera's frame is 127 pixels from the true
aim point (700, 420), about 3800″ at 29.78″ a pixel.

**What probe round 1 and the reanalysis changed** (2026-11-19; spec.md
§ Clarifications, the session of round 1 probes and the reanalysis). Four agents built
the eleven stories from this trio, in order, tests first and with both gates, and found
3 blocking and 35 clarity defects, D1 to D38 (§ Prerequisites 5 says what the round
settled). Six reviews of the round, the trio and the code then raised 71 more points
(A1 to A14, B1 to B9, C1 to C19, D101 to D110, E1 to E3, F1 to F16), and the owner
answered their seventeen choices with "Take all 17". All of it was folded on
2026-11-19, in three passes. The changes that reach across stories:

- **The three blocking defects.** US4-S5 reads rest from 2 s after a Stop (D13). The
  mount part reads steady only when 036's tracker was steady for the whole of 032's
  fit reach, by `steady_between` and not by round 1's fixed wait (D14 and the owner's
  answer 2: FR-019, US4-S1, US5-S6). US10's continue is read back with
  `get_imaging_run` after a settle (D30, now US10-S8).
- **What ends a run.** A solve that measured no new aim point is no try, and a target
  near the centre is stacked with a warning whichever way centring gives up (FR-024);
  a push ends the run only once it has settled or lasted 10 s, and a target changed
  during a re-centre leaves Track off (FR-023); a failed take is asked again 5 s
  later, and the mount's ending is judged before the stack's failure (FR-009, FR-032);
  subs that keep failing end the run by time, not by a count (FR-031); and a focus
  check stopped from 033's own route ends the run (FR-029).
- **What a run keeps, and says.** Subs are removed only after a saved stack of at least
  one sub, another client's stack is left alone, and room for the stacked picture's
  files is held back for the ending's save (FR-033); `run.json` is refreshed while the
  count rises, lists the
  tracker's pauses at most once a minute, and an unfinished run is said after a
  restart (FR-016, FR-017); four endings share the reason `stopped`, each with its
  sentence, an ending says whether the stack was saved and whether Track is still on,
  and a stop of the run while it still slews stops the mount (FR-004 to FR-006); the
  sentences an observer could not act on are reworded (§ The run's words).
- **The focus pause** repeats a check during which the tracker was not steady, shows
  its deadline, and acts on the first resume only (FR-026 to FR-028).
- **The interfaces of US1** (US12's since the split). `MountSteps` gained `stop`,
  `track_on` and
  `steady_between`, `Centre` `near_arcsec`, and the script `unsteady`,
  `Step::StackStop` and `Step::CloseAid`; the view gained `stacking_since`,
  `limit_at`, `pause_ends_at`, `ended.saved` and `ended.track_on`, and the imaging
  view `unfinished_run`.
- **The page, the tools and the README.** The block asks for a second tap before it
  stops a run, disables its buttons while a request is out, and shows the size of the
  subs, the time stacked against the limit and the time a pause has left (FR-035);
  the tools' descriptions say what an agent needs to run a night (§ The MCP tools);
  the README's table of endings says what to do next, and its field checklist has
  eighteen steps (§ The README).
- **Tests moved, with no code.** US1-S3 to US1-S6 are US2-S6 to US2-S9, and the start's
  refusals of US1-S2 are US2-S10, so that US1's diff stays under 64 KiB (the owner's
  answer 16, widened by the fold when the other answers added code to US1; § Risks).
- **Scenarios.** The trio had 90 at the fold where the probed draft had 58, and has
  93 since the split of US1. Each second half of a
  scenario has its own number (US3-S10, US6-S10, US6-S11, US7-S7 to US7-S9, US8-S8,
  US9-S13, US10-S8); three rules that no scenario asserted have one each (US3-S10,
  US4-S6, US8-S9), and so have the two word lists (US2-S11, since the split a bullet of
  US12-S1); and every other new case
  is a bullet of the scenario it belongs to or a new scenario at the end of its
  story, so no scenario id moved but US1-S3 to US1-S6.

The records below change with them: every case added has a clause in its scenario's
`assertion`; the moved scenarios are listed under US2, in US2's test module; and US11's
shared write of the README names 036's US8, which writes it after US7 in 036's folded
record. Two strings the record repeats were shortened, with no change of meaning, to
keep it under the 64 KiB a record may be. No story was over 64 KiB as built. On the
folded figures US1 was estimated at about 56 KB, 86 % (§ Risks). None of what the fold
added has been run: the second probe round builds the changed stories before the flip.

**The split of US1** (2026-11-19, after the fold; spec.md § Clarifications, the last
question of that session). The owner answered "Yes, split US1", and the record gained
US12, which is built first. It holds four scenarios: the two word lists, which was
US2-S11, with every ending's reason and sentence; `label` and `run_id`, which was a
bullet of US1-S1, with the request and the settings; and two for the script. US1 lost
its writes of `crates/finder/src/imaging/mod.rs` and `script.rs`, and gained a line for
US12's scenarios, which it must leave passing. US12 and US3 both write
`crates/finder/src/imaging/mod.rs`, and US3 follows US12 through US1; the record has no
shared write for the two, because the validator takes a `merge_order` entry only between
two stories joined by an edge of their own, and the Work Graph gained no edge but US1
after US12. The same day the stack part's reserve was sized to the stacked picture
(US8-S7) and the pause after a focus check that did not finish got words of its own
(US6-S10). To make room for US12's block the record was re-indented, and nothing else:
its list items stand under their keys, and every level under `stories` is one space
deep. The record parses to the same object before and after each of the two steps, no
`checks` list was narrowed and no clause of an assertion dropped. Nothing of the split
has been built.

**036's second fold** (2026-11-19, after the split; spec.md § Clarifications, the fourth
point of that session's last question). 036's fold of its own reanalysis changed four
things this trio said of it, and they are corrected here with no choice put to the
owner; the two new sentences are the fold's wording. Its steering stands back from a
stack that begins more than 180″ from the set point, so each of the two warnings for a
target stacked off the centre has a second form, `; it is stacked where it is.`, chosen
by 036's own `FAR_ARCSEC` (FR-024; US5-S3, whose two examples at 400″ now read the
second form, and which gained a bullet at 180″ and 150″ for the first). Its tracker
settles for 2 s after its first rates, so each wait for a steady tracker is that much
longer (§ The console's mount part). It is not steady while 038's push reads `moving`,
which § Risks had as not known. And its stack also waits for its exposure limit to rise,
with the tracker steady, so the sentence of `paused_by_tracker` names the tracker and no
cause (US2-S3). No story, scenario or FR number moved and no scenario was added. In the
record below US5-S3's assertion gained one clause, and nothing else changed: it is
65,440 B of the 65,536 a record may be. None of it has been run.

§ Prerequisites 2 and 3, the re-anchoring, are still to do. Until then the record
below names what the drafts named: its
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
    assertion: "a scripted run held at each step reads slewing, first_frame, focus_check, solving and stacking with their messages, its view carries the id, by, target, settings and neither folder, stacking_since nor limit_at until the stack has started, when limit_at is 7200 s after stacking_since, the calls are the straight path in order with the lease given up before the focus check and before the stack and taken again between them, a held take is the last call listed while it waits, and the view written as json holds only the keys whose values are there"
    decision: "spec.md:US1-S1"
  - path: crates/finder/tests/console/imaging_run.rs
    status: new
    disposition: add
    assertion: "a lease that cannot be taken back after the focus check ends camera_failed with the lease's words and no solve, its ended written as json with exactly its five keys, and a focus check that fails with another failure leaves focus unmeasurable in px with the warning of a check that did not finish, the test reading nothing of the state or of the calls after it"
    decision: "spec.md:US1-S2"
  - path: crates/finder/tests/layout/main.rs
    status: existing
    disposition: preserve
    assertion: "no file at the top level of crates/finder/tests, no new file in its support folder, and each main.rs only its support include and mod lines"
    decision: "plan.md:Shapes that existing tests pin"
  preserved_behavior:
  - "no Cargo.toml, Cargo.lock or deny.toml changes and no unsafe is written (spec.md SC-003)"
  - "nothing under crates/finder/src/imaging/run.rs imports goto, tracking, ephemeris, model or scope_link, holds an identifier or arithmetic for an axis, or holds a number sized to one telescope (plan.md Plan traps 18)"
  - "the imaging state of 030 to 036 and their tests are untouched but for one mod line"
  - "every scenario of US12 still holds: US1 adds to run.rs after what US12 wrote, changes no item of it, and does not write script.rs (plan.md Shapes that existing tests pin)"
  checks:
   development:
   - "cargo nextest run -p finder --locked --test console imaging_run::"
   - "cargo nextest run -p finder --locked --test console imaging_run_words::"
   - "cargo nextest run -p finder --locked --test layout"
   handoff: declared_gates
   independent: declared_pipeline
   release:
   - "release owner: the repository owner, through the merge queue's ci.yml jobs"
  writes:
  - crates/finder/src/imaging/run.rs
  - crates/finder/tests/console/imaging_run.rs
  - crates/finder/tests/console/main.rs
  no_write_reason: ""
  shared_writes:
  - path: crates/finder/src/imaging/run.rs
    other_spec: 037-imaging-run
    other_story: US2
    disposition: merge_order
    reason: "US2 depends_on_merged US1, and replaces what a failed take, solve and stack start do with the table of failed steps, and reads the stack's progress"
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
    assertion: "two failed takes are retried 5 s apart, one take listed 4.5 s after the start and two 1 s later, three in a row end camera_failed with the last error before any focus check, a stop of the run inside the wait between two takes ends it stopped after one take, and a save failure ends save_failed at the first take"
    decision: "spec.md:US2-S1"
  - path: crates/finder/tests/console/imaging_run_steps.rs
    status: new
    disposition: add
    assertion: "a solve without a solution takes a new imaging frame and solves again with no second focus check, three in a row end solver_failing, a solver that cannot run ends solver_unavailable at once, a solve that cannot save ends save_failed after one solve, and a focus check that cannot save ends save_failed and one whose solver cannot run solver_unavailable"
    decision: "spec.md:US2-S2"
  - path: crates/finder/tests/console/imaging_run_steps.rs
    status: new
    disposition: add
    assertion: "while stacking the view mirrors the counts set_stack gave with its message, reads paused_by_tracker while the stack waits for the tracker and stacking again after, and a slightly soft verdict leaves its warning in arcseconds or pixels and goes on"
    decision: "spec.md:US2-S3"
  - path: crates/finder/tests/console/imaging_run_steps.rs
    status: new
    disposition: add
    assertion: "a stack that cannot start ends the run by its failure's kind without being stopped and with no folder, a stack start that fails as no solution ends solver_failing, as a camera camera_failed and as a solver that cannot run solver_unavailable, a stack that does not end cleanly at an ending leaves a warning and the ending's reason, a run stopped with keep_subs false and nothing stacked asks the stack to discard and warns that the subs were kept, with no such warning once subs are stacked, and while stack.stop is held an ending run still reads stacking with the message Saving the stack and its count, and refuses a new start"
    decision: "spec.md:US2-S4"
  - path: crates/finder/tests/console/imaging_run_steps.rs
    status: new
    disposition: add
    assertion: "a run that goes on from its focus check to a mount that is not steady reads solving, its message followed by the words of a wait for the tracker, with one take made, and reaches stacking once the mount is steady"
    decision: "spec.md:US2-S5"
  - path: crates/finder/tests/console/imaging_run_steps.rs
    status: new
    disposition: add
    assertion: "a mount that tracks but is not steady shows tracking with no take and no focus check, and the run goes on once it is steady"
    decision: "spec.md:US2-S6"
  - path: crates/finder/tests/console/imaging_run_steps.rs
    status: new
    disposition: add
    assertion: "a stop in each of five states ends the run as stopped, saying that the imaging run stopped and Track is still on, with track_on true, in the four states after slewing, and that the mount was stopped in the run still slewing, which alone asks the mount part for Stop, before the lease is given back; saved is true only for the run stopped while stacking; the stop aborts a take in flight, stops a focus check in flight, stops a started stack once and no other, gives the lease back as the last call, lists no call after the ending and never repoints; and track_on reads false once the mount is found stopped after the ending"
    decision: "spec.md:US2-S7"
  - path: crates/finder/tests/console/imaging_run_steps.rs
    status: new
    disposition: add
    assertion: "each of seven endings of the mount's motion while stacking stops the stack once, gives the lease back, and ends the run with its reason and exact message, the first being that the mount was stopped, with saved true and track_on false after the five endings that stop the mount and true after target_changed and pushed_away; a mount ending while a take is held at first_frame ends too_low with the take aborted once, and one while the run waits for a slewing mount ends sun_up with the lease given back as the last call"
    decision: "spec.md:US2-S8"
  - path: crates/finder/tests/console/imaging_run_steps.rs
    status: new
    disposition: add
    assertion: "dropping the only Arc of a runner whose run is stacking leaves the script with one owner and adds no call in the next 10 s"
    decision: "spec.md:US2-S9"
  - path: crates/finder/tests/console/imaging_run_steps.rs
    status: new
    disposition: add
    assertion: "a refusal of the camera, the solver, the target, the lease or Track stops the start at that part with 409 and its words, a refused lease is followed by neither Track nor a release, a refused Track gives the lease back, bad subs and duration_s are 400 with their words, a second start is refused while a run is on and accepted after a stop"
    decision: "spec.md:US2-S10"
  - path: crates/finder/tests/layout/main.rs
    status: existing
    disposition: preserve
    assertion: "no file at the top level of crates/finder/tests, no new file in its support folder, and each main.rs only its support include and mod lines"
    decision: "plan.md:Shapes that existing tests pin"
  preserved_behavior:
  - "every scenario of US1 still holds: none fails a take, a solve or a stack start or sets a count, the focus check US1-S2 fails gives Failure::Other, which US2 leaves as it was, and what US2 replaces, a failed take, solve or stack start ending the run at once, is asserted by none (plan.md Shapes that existing tests pin)"
  - "US2-S6 to US2-S10 assert the machine as US1 leaves it, and pass before US2's code as after it (plan.md Shapes that existing tests pin)"
  - "no trait of a part gains a method, Steps gains no field, and the script is not written again"
  - "nothing under crates/finder/src/imaging/run.rs holds an identifier or arithmetic for an axis (plan.md Plan traps 18)"
  - "no scenario of US1 reads a message while a run waits for the tracker after tracking, or while an ending is in progress, so the words US2 adds to both change none (plan.md Shapes that existing tests pin)"
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
    assertion: "POST /api/imaging/run answers 200 with the run in the imaging view and in the session, never in the session.json the scripted console keeps, and once the run stacks run.json lies in the stack's folder with the format, version, id, by, target, times, settings, state and the events in order"
    decision: "spec.md:US3-S1"
  - path: crates/finder/tests/console/imaging_run_routes.rs
    status: new
    disposition: add
    assertion: "bad bodies answer 400, another content type 415, a foreign origin 403, a part's refusal and a second start 409 with their words, a console given no parts 409 on all three routes, each sent a body it would otherwise accept, and no refusal leaves a run or a folder"
    decision: "spec.md:US3-S2"
  - path: crates/finder/tests/console/imaging_run_routes.rs
    status: new
    disposition: add
    assertion: "a pause by the tracker is written to run.json when it happens, with paused_by_tracker as its state and its last event, DELETE ends the run as stopped and answers once it has ended, run.json then holds the ending, the count set_stack gave just before the DELETE with no poll between and ended as its last event, a second DELETE changes nothing and DELETE with no run answers 200 with no run"
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
    assertion: "two runs started in one whole second that end at first_frame share an id, have no folder until their ending and then runs/<id> and runs/<id>-2, each holding a run.json that ended stopped"
    decision: "spec.md:US3-S6"
  - path: crates/finder/tests/console/imaging_run_routes.rs
    status: new
    disposition: add
    assertion: "run.json holds no token, node address or site, and no message, elapsed_s or site key"
    decision: "spec.md:US3-S7"
  - path: crates/finder/tests/console/imaging_run_routes.rs
    status: new
    disposition: add
    assertion: "while a run stacks, run.json reads 3 stacked 61 s after the count rose to 3, still 3 thirty seconds after it rose to 4, and 4 thirty-one seconds after that, each time with the state stacking and no event beyond the five of the start"
    decision: "spec.md:US3-S8"
  - path: crates/finder/tests/console/imaging_run_routes.rs
    status: new
    disposition: add
    assertion: "a console built over an imaging folder whose newest run record has no ended carries unfinished_run with that record's folder and no run, drops the key at the first accepted start, and never has it when the record holds an ended"
    decision: "spec.md:US3-S9"
  - path: crates/finder/tests/console/imaging_run_routes.rs
    status: new
    disposition: add
    assertion: "with a regular file named runs in the imaging folder, a stacking run ends save_failed with the stack stopped, the lease given back and the warning that its record could not be saved, and a run stopped at first_frame ends stopped with no folder and that warning"
    decision: "spec.md:US3-S10"
  - path: crates/finder/tests/console/imaging_run_routes.rs
    status: new
    disposition: add
    assertion: "of three changes between stacking and paused_by_tracker within seconds run.json lists and writes the first only, the view showing each at once, and one 60 s later is listed and written"
    decision: "spec.md:US3-S11"
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
    assertion: "no file at the top level of crates/finder/tests, no new file in its support folder, and each main.rs only its support include and mod lines"
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
    reason: "037 depends_on_landed 035-live-stack, whose US8 last writes this file in the drafts; re-read at the re-anchoring"
  - path: crates/finder/src/api.rs
    other_spec: 037-imaging-run
    other_story: US8
    disposition: merge_order
    reason: "US8 depends_on_merged US3, and adds with_imaging_run"
  - path: crates/finder/src/imaging/routes.rs
    other_spec: 036-tracking-from-subs
    other_story: US5
    disposition: merge_order
    reason: "037 depends_on_landed 036-tracking-from-subs, whose US5 last writes this file in the drafts; re-read at the re-anchoring"
  - path: crates/finder/src/imaging/run.rs
    other_spec: 037-imaging-run
    other_story: US4
    disposition: merge_order
    reason: "US4 depends_on_merged US3, and adds pub mod console"
  - path: crates/finder/src/lib.rs
    other_spec: 036-tracking-from-subs
    other_story: US5
    disposition: merge_order
    reason: "037 depends_on_landed 036-tracking-from-subs, whose US5 last writes this file in the drafts; re-read at the re-anchoring"
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
    assertion: "on the simulator with the true aim point set a run reaches stacking with no try, the tolerance exactly 90.0, its solved offset inside the tolerance and within 30 arcseconds of the true offset, the true offset at most 110 arcseconds, the run reading slewing at every sample at which Track read armed and nothing later than tracking in the 9.5 s after Track first read tracking, its imaging frame's purpose run, Track never leaving tracking, and the motion record reading page and imaging_run"
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
    assertion: "a raised minimum altitude ends the run too_low, an unresponsive node link_lost, a changed target target_changed with Track still on the old target, a push by hand pushed_away with Track following it, dawn sun_up, an imaging frame asked for while the run holds the lease is refused with the imaging camera is held by the imaging run and accepted after the run, a tracker that is not steady near the transit still holds the run in tracking with its message 30 s after it first read tracking, a push of 6 arcminutes undone 3 s later is read as moving and does not end the run, and a DELETE while the run still slews ends it stopped with the sentence that the mount was stopped, track_on false, Track off and the simulated mount at rest for 10 s"
    decision: "spec.md:US4-S4"
  - path: crates/finder/tests/motion/imaging_run.rs
    status: new
    disposition: add
    assertion: "the mount part alone points, reads tracking and steady, points again leaving the motion record as page and imaging_run, and after POST /api/stop reads ended stopped and refuses to point again with Track off and the simulated mount at rest for 10 s from 2 s after the stop; and on a rig of its own, after the target is changed, a re-centre answers Ok with Track left off for 10 s and the part reading target_changed"
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
  - "the run sends no axis command and reads no axis position (spec.md FR-001)"
  - "no trait of a part gains a method and Steps gains no field"
  - "the lease is 031's US9: the camera part takes it, gives it back and adds no lock (plan.md Plan traps 20)"
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
    reason: "037 depends_on_landed 036-tracking-from-subs, whose US7 last writes this file in the drafts; re-read at the re-anchoring"
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
    assertion: "four solves 600 arcseconds off, beyond the near bound of 450, repoint three times and end not_centred with the offset written as 10.0 arcminutes and the words to check the aim point, with no stack started"
    decision: "spec.md:US5-S2"
  - path: crates/finder/tests/console/imaging_run_centring.rs
    status: new
    disposition: add
    assertion: "a solve that measured no new aim point is retried without a repoint and is not a try, its message saying so while the run waits for the tracker; four such solves in a row stack a target 400 arcseconds off with their warning and end not_centred 600 arcseconds off with their sentence, with no try and no repoint; a re-centre starts that count again; a target 400 arcseconds off after three tries, inside the near bound, is stacked with its warning; at 400 arcseconds both warnings say it is stacked where it is, and at 180 and 150, not over 036's 180, that the stack's steering will bring it in; a failed repoint ends mount_failed or stopped by its failure, no take follows a repoint until the mount is steady, and a stop of the run during a repoint is answered only after it and ends the run stopped with no further take"
    decision: "spec.md:US5-S3"
  - path: crates/finder/tests/motion/imaging_run_centring.rs
    status: new
    disposition: add
    assertion: "on the simulator with no aim point a run reaches stacking after at least one try with the tolerance exactly 90.0, the true offset within the tolerance plus 20 arcseconds, the session's aim point measured within 2.0 pixels of the true one, and Track read as no longer off within 2 s of the first sample that read it off while centring"
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
    assertion: "a clearly out verdict pauses the run with its message, focus note, pause_ends_at 900 s after the pause began and the focus aid opened and no solve"
    decision: "spec.md:US6-S1"
  - path: crates/finder/tests/console/imaging_run_focus.rs
    status: new
    disposition: add
    assertion: "resume(false) closes the aid and checks focus again, taking the lease back and going on to the solve when it is good"
    decision: "spec.md:US6-S2"
  - path: crates/finder/tests/console/imaging_run_focus.rs
    status: new
    disposition: add
    assertion: "resume(true) closes the aid, takes the lease back and goes on to the solve with no second check and the go-anyway warning"
    decision: "spec.md:US6-S3"
  - path: crates/finder/tests/console/imaging_run_focus.rs
    status: new
    disposition: add
    assertion: "a pause is still on 899.5 s after the start and has ended focus_timeout 1 s later with the aid closed and no stack started"
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
    assertion: "on the simulator the console's own focus part passes a run at the focus error 0.0 with its reading in arcsec, pauses one that 033's own check calls clearly out with the focus aid running, goes on after refocusing, refuses a start with the imaging camera is held by the focus aid while the aid already runs, with no mount.point after the refused start, pauses with no_record on a fresh folder, and ends stopped with the sentence that the focus check was stopped, no solve and 033's aid ended when the run's own check is stopped from 033's route"
    decision: "spec.md:US6-S7"
  - path: crates/finder/tests/console/imaging_run_focus.rs
    status: new
    disposition: add
    assertion: "a clearly out check during which the tracker was not steady is repeated once with no pause and no warning, a second one that was not steady either pauses with the warning that the tracker was not steady, and a good check is not repeated"
    decision: "spec.md:US6-S8"
  - path: crates/finder/tests/console/imaging_run_focus.rs
    status: new
    disposition: add
    assertion: "a second resume right after a first is refused as outside a pause and the first is the one acted on, and a resume that arrives after the limit has fired, while the aid is still closing, is refused and the run ends focus_timeout"
    decision: "spec.md:US6-S9"
  - path: crates/finder/tests/console/imaging_run_focus.rs
    status: new
    disposition: add
    assertion: "clearly out without a record, unmeasurable and no record each pause with their own words; a check that did not finish because it found no stars to measure pauses with the words of unmeasurable, and one that failed otherwise with the words that it did not finish, its reason and to check the imaging camera, each of the two with its warning"
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
  - "every scenario of US1 to US5 still holds: none gives a verdict that pauses, the one run whose check did not finish (US1-S2) is read only for focus and warnings, which US6 leaves as they are, and US3's continue refusals outside a pause are unchanged"
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
    reason: "US7 depends_on_merged US6, and adds the limits, the rule for subs that keep failing and the stack's own endings"
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
    assertion: "a run asked for 3 subs keeps stacking at 2 stacked with 1 dropped and ends count_limit at 3, saying that the 3 subs asked for are stacked, with the stack stopped once and its subs kept"
    decision: "spec.md:US7-S1"
  - path: crates/finder/tests/console/imaging_run_limits.rs
    status: new
    disposition: add
    assertion: "a 60 s limit counts from the start of stacking, not of the run: still stacking 59.5 s in and ended duration_limit by 60.5 s"
    decision: "spec.md:US7-S2"
  - path: crates/finder/tests/console/imaging_run_limits.rs
    status: new
    disposition: add
    assertion: "with a failed sub and none stacked the run still stacks 599.5 s after the last stacked sub and has ended subs_failing 1 s later, naming each failed reason's rise with spaces and not the subs the console was behind on"
    decision: "spec.md:US7-S3"
  - path: crates/finder/tests/console/imaging_run_limits.rs
    status: new
    disposition: add
    assertion: "each kind of stack failure ends the run with its own reason, an unknown one as stack_failed with its words, a stack that ends with no failure as stopped with the sentence that the stack was stopped, and each still stops the stack once"
    decision: "spec.md:US7-S4"
  - path: crates/finder/tests/console/imaging_run_limits.rs
    status: new
    disposition: add
    assertion: "a stack that fails as a camera in the same poll in which the mount's motion ends link_lost ends the run link_lost, with the stack stopped once"
    decision: "spec.md:US7-S5"
  - path: crates/finder/tests/console/imaging_run_limits.rs
    status: new
    disposition: add
    assertion: "700 s with only behind and unprocessed subs leave the run stacking, 700 s paused by the tracker leave it paused, and after the pause it still stacks 590 s on and has ended subs_failing 12 s later"
    decision: "spec.md:US7-S6"
  - path: crates/finder/tests/console/imaging_run_limits.rs
    status: new
    disposition: add
    assertion: "with keep_subs false the count limit asks the stack once to discard its subs, never to keep them"
    decision: "spec.md:US7-S7"
  - path: crates/finder/tests/console/imaging_run_limits.rs
    status: new
    disposition: add
    assertion: "the time limit counts the time the tracker pauses the stack: paused_by_tracker 59.5 s in and ended duration_limit by 60.5 s"
    decision: "spec.md:US7-S8"
  - path: crates/finder/tests/console/imaging_run_limits.rs
    status: new
    disposition: add
    assertion: "a stacked sub between two stretches of 400 s with failed subs starts the time again, and the run still stacks"
    decision: "spec.md:US7-S9"
  - path: crates/finder/tests/layout/main.rs
    status: existing
    disposition: preserve
    assertion: "no file at the top level of crates/finder/tests, no new file in its support folder, and each main.rs only its support include and mod lines"
    decision: "plan.md:Shapes that existing tests pin"
  preserved_behavior:
  - "every scenario of US1 to US6 still holds: the default request has no count and 7200 s, none stacks for 600 s with a dropped sub, and the script's stack runs until it is stopped or end_stack is called, whatever set_stack gave it (plan.md The script, Plan traps 24)"
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
    assertion: "once the stack part's start has answered, the run's folder holds a reserve file of 15 bytes for each binned pixel of the imaging view's camera plus 16777216, 32017096 bytes on the rig, and none once the run has been stopped"
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
    reason: "037 depends_on_landed 033-focus-reading-and-aid, whose US8 last writes this file in the drafts; re-read at the re-anchoring"
  - path: crates/finder/tests/cli/main.rs
    other_spec: 035-live-stack
    other_story: US8
    disposition: merge_order
    reason: "037 depends_on_landed 035-live-stack, whose US8 last writes this file in the drafts; re-read at the re-anchoring"
  open_questions: []
 US9:
  tests:
  - path: crates/finder/web/src/components/ImagingRunBlock.test.tsx
    status: new
    disposition: add
    assertion: "with no run the block offers Start imaging run, two empty limit fields and a checked Keep every sub with no status or alert"
    decision: "spec.md:US9-S1"
  - path: crates/finder/web/src/components/ImagingRunBlock.test.tsx
    status: new
    disposition: add
    assertion: "a stacking run reads its message, counts by reason, with no brackets when none was dropped, and elapsed time rounded down, offers Stop imaging run which reads Tap again to stop the run after one click and sends DELETE at the second, and neither button of a focus pause, and says Started by an MCP client only when by is mcp"
    decision: "spec.md:US9-S2"
  - path: crates/finder/web/src/components/ImagingRunBlock.test.tsx
    status: new
    disposition: add
    assertion: "a focus pause shows its message as an alert with Check focus again and Go anyway posting anyway false and true, the two in an element that does not hold Stop imaging run"
    decision: "spec.md:US9-S3"
  - path: crates/finder/web/src/components/ImagingRunBlock.test.tsx
    status: new
    disposition: add
    assertion: "a centring run reads its message as a status and each warning as a paragraph"
    decision: "spec.md:US9-S4"
  - path: crates/finder/web/src/components/ImagingRunBlock.test.tsx
    status: new
    disposition: add
    assertion: "an ended run says why and what was saved, the count limit's line in its short form, as a status for a stop, a count limit and a time limit and as an alert for a guard's ending, shows each of its warnings as a paragraph, and offers Start imaging run again"
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
  - path: crates/finder/web/src/components/ImagingRunBlock.test.tsx
    status: new
    disposition: add
    assertion: "an ended run whose stack was not saved shows an alert, for a stop as for a save failure, that says the subs were stacked but not saved and where they are, one sub is written 1 sub, and the line that Track is still on shows only while ended.track_on is true"
    decision: "spec.md:US9-S8"
  - path: crates/finder/web/src/components/ImagingRunBlock.test.tsx
    status: new
    disposition: add
    assertion: "one click on Stop imaging run sends nothing and asks for a second, which sends DELETE only within 5 s, the button reads Stopping and is disabled while the request is out, and it is disabled while the run saves its stack"
    decision: "spec.md:US9-S9"
  - path: crates/finder/web/src/components/ImagingRunBlock.test.tsx
    status: new
    disposition: add
    assertion: "a refused Check focus again shows its words in a second alert with no refresh, the alert goes when the run's state changes, the pause buttons and the stop button are disabled while the request is out, and a request that got no answer shows the block's own words"
    decision: "spec.md:US9-S10"
  - path: crates/finder/web/src/components/ImagingRunBlock.test.tsx
    status: new
    disposition: add
    assertion: "the start form shows 120 as the placeholder of Minutes to stack, the quiet line with the two-hour default, and the size of the subs for the minutes typed or 120, and a view with unfinished_run shows the alert that the last imaging run did not finish, with Start imaging run still offered"
    decision: "spec.md:US9-S11"
  - path: crates/finder/web/src/components/ImagingRunBlock.test.tsx
    status: new
    disposition: add
    assertion: "a focus pause shows the time it has left from pause_ends_at, and a stacking run the time stacked against its limit from stacking_since"
    decision: "spec.md:US9-S12"
  - path: crates/finder/web/src/components/ImagingRunBlock.test.tsx
    status: new
    disposition: add
    assertion: "a click on Start imaging run posts the request given, sends no limit typed as 0, keeps a draft across a new view, shows a refusal as an alert with no refresh, and drops that alert once a run appears"
    decision: "spec.md:US9-S13"
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
    reason: "037 depends_on_landed 036-tracking-from-subs, whose US8 last writes this file in the drafts; re-read at the re-anchoring"
  - path: DESIGN.md
    other_spec: 036-tracking-from-subs
    other_story: US8
    disposition: merge_order
    reason: "037 depends_on_landed 036-tracking-from-subs, whose US8 last writes this file in the drafts; re-read at the re-anchoring"
  - path: crates/finder/web/dist
    other_spec: 036-tracking-from-subs
    other_story: US8
    disposition: merge_order
    reason: "037 depends_on_landed 036-tracking-from-subs, whose US8 last writes this file in the drafts; re-read at the re-anchoring"
  - path: crates/finder/web/src/api.ts
    other_spec: 036-tracking-from-subs
    other_story: US8
    disposition: merge_order
    reason: "037 depends_on_landed 036-tracking-from-subs, whose US8 last writes this file in the drafts; re-read at the re-anchoring"
  - path: crates/finder/web/src/components/ImagingPage.tsx
    other_spec: 036-tracking-from-subs
    other_story: US8
    disposition: merge_order
    reason: "037 depends_on_landed 036-tracking-from-subs, whose US8 last writes this file in the drafts; re-read at the re-anchoring"
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
    assertion: "bad arguments, an unknown key of continue_imaging_run, a run that is not on and a console given no parts, each tool called with arguments it would otherwise accept, are tool errors with the routes' words"
    decision: "spec.md:US10-S3"
  - path: crates/finder/tests/mcp/imaging_run.rs
    status: new
    disposition: add
    assertion: "two starts in one second answer the page route's own refusal and the third the rate limit, each logged"
    decision: "spec.md:US10-S4"
  - path: crates/finder/tests/mcp/imaging_run.rs
    status: new
    disposition: add
    assertion: "stop_imaging_run ends the run as stopped with the counts set_stack gave and no log line"
    decision: "spec.md:US10-S5"
  - path: crates/finder/tests/mcp/imaging_run.rs
    status: new
    disposition: add
    assertion: "start_imaging_run is refused with a direct axis move is running while move_axis runs, with nothing asked of any part, and accepted after stop, and stop_imaging_run then ends that run stopped while the recording mount receives no StopAll in the next second"
    decision: "spec.md:US10-S6"
  - path: crates/finder/tests/mcp/imaging_run.rs
    status: new
    disposition: add
    assertion: "no answer of the four tools and no call-log line holds the node's token"
    decision: "spec.md:US10-S7"
  - path: crates/finder/tests/mcp/imaging_run.rs
    status: new
    disposition: add
    assertion: "after continue_imaging_run with anyway true and a settle get_imaging_run answers a run no longer in focus_pause with the go-anyway warning"
    decision: "spec.md:US10-S8"
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
    assertion: "no file at the top level of crates/finder/tests, no new file in its support folder, and each main.rs only its support include and mod lines"
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
    reason: "037 depends_on_landed 035-live-stack, whose US10 last writes this file in the drafts; re-read at the re-anchoring"
  - path: crates/finder/tests/mcp/main.rs
    other_spec: 035-live-stack
    other_story: US10
    disposition: merge_order
    reason: "037 depends_on_landed 035-live-stack, whose US10 last writes this file in the drafts; re-read at the re-anchoring"
  open_questions: []
 US11:
  tests:
  - path: crates/finder/tests/console/imaging_run_docs.rs
    status: new
    disposition: add
    assertion: "the README's An imaging run section, read to the next section or to the end of the file, names every state word, every ending reason, the three routes, the four tools and run.json between backticks, and its prose before the field checklist contains corner, 7200 and What to do next, the heading of the fourth column of its table of endings"
    decision: "spec.md:US11-S1"
  - path: crates/finder/tests/console/imaging_run_docs.rs
    status: new
    disposition: add
    assertion: "the field checklist, read to the next heading or to the end of the file, has exactly eighteen steps numbered in order, names Start imaging run, start_imaging_run, continue_imaging_run, Stop and run.json, and its step 8 has the observer look at the corners of the run's own solved imaging frames"
    decision: "spec.md:US11-S2"
  - path: crates/finder/tests/console/imaging_run_docs.rs
    status: new
    disposition: add
    assertion: "the API table has a row for each of the three routes and MCP clients names the four tools"
    decision: "spec.md:US11-S3"
  - path: crates/finder/tests/layout/main.rs
    status: existing
    disposition: preserve
    assertion: "no file at the top level of crates/finder/tests, no new file in its support folder, and each main.rs only its support include and mod lines"
    decision: "plan.md:Shapes that existing tests pin"
  preserved_behavior:
  - "no code changes: the story writes the README and its test"
  - "the README's existing sections are unchanged but for three rows of the API table, four lines of MCP clients and the count in the sentence before them"
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
    other_story: US8
    disposition: merge_order
    reason: "037 depends_on_landed 036-tracking-from-subs, whose US8 last writes this file in the drafts; re-read at the re-anchoring"
  open_questions: []
 US12:
  tests:
  - path: crates/finder/tests/console/imaging_run_words.rs
    status: new
    disposition: add
    assertion: "STATES is exactly the ten state words and ENDINGS the seventeen ending reasons, each in its order; each of the twenty-one endings answers its reason and its exact sentence, with 1 sub for one, a time limit's minutes as 90, 1.5 and 0.1, and an offset in arcseconds below 600 and in arcminutes from it on; the reasons and ENDINGS cover each other; and By, Verdict, Unit and Failure answer their words"
    decision: "spec.md:US12-S1"
  - path: crates/finder/tests/console/imaging_run_words.rs
    status: new
    disposition: add
    assertion: "label and run_id turn NGC 1976 into NGC-1976; a request parses from an empty object and from its three keys and refuses a key it does not know; the settings are written with subs null and the target with its four keys; and the four constants have their values"
    decision: "spec.md:US12-S2"
  - path: crates/finder/tests/console/imaging_run_words.rs
    status: new
    disposition: add
    assertion: "a new script's five parts answer their defaults, takes numbered from 1; calls() lists one string a call in the order made, nineteen in all, with page and mcp, keep and discard; the mount's three reads and the stack's progress are not listed; and the stack runs from its start to its stop with zero counts"
    decision: "spec.md:US12-S3"
  - path: crates/finder/tests/console/imaging_run_words.rs
    status: new
    disposition: add
    assertion: "a pushed take, focus check or solve is answered as pushed, in order and using no frame number, then the default; each of the four fail controls fails its call once; each of the five refusals lasts and is listed; unsteady(2) makes two answers false and the third true; track_on is false after the five endings that stop the mount and true otherwise; set_stack sets the counts and the tracker's pause, leaves out a zero count and never ends the stack, which end_stack does with its failure; and a held take is listed at once, answers only once released, holds no other step and is not held again, as with stack.stop and focus.close_aid"
    decision: "spec.md:US12-S4"
  - path: crates/finder/tests/layout/main.rs
    status: existing
    disposition: preserve
    assertion: "no file at the top level of crates/finder/tests, no new file in its support folder, and each main.rs only its support include and mod lines"
    decision: "plan.md:Shapes that existing tests pin"
  preserved_behavior:
  - "no Cargo.toml, Cargo.lock or deny.toml changes and no unsafe is written (spec.md SC-003)"
  - "nothing under crates/finder/src/imaging/run.rs or run/script.rs imports goto, tracking, ephemeris, model or scope_link, holds an identifier or arithmetic for an axis, or holds a number sized to one telescope (plan.md Plan traps 18)"
  - "the imaging state of 030 to 036 and their tests are untouched but for one mod line"
  - "the rule for a folder's name is 035's crate::imaging::stack::service::label, called by run::label and not copied (plan.md The parts and the words)"
  checks:
   development:
   - "cargo nextest run -p finder --locked --test console imaging_run_words::"
   - "cargo nextest run -p finder --locked --test layout"
   handoff: declared_gates
   independent: declared_pipeline
   release:
   - "release owner: the repository owner, through the merge queue's ci.yml jobs"
  writes:
  - crates/finder/src/imaging/mod.rs
  - crates/finder/src/imaging/run.rs
  - crates/finder/src/imaging/run/script.rs
  - crates/finder/tests/console/imaging_run_words.rs
  - crates/finder/tests/console/main.rs
  no_write_reason: ""
  shared_writes:
  - path: crates/finder/src/imaging/mod.rs
    other_spec: 036-tracking-from-subs
    other_story: US4
    disposition: merge_order
    reason: "037 depends_on_landed 036-tracking-from-subs, whose US4 last writes this file in the drafts; re-read at the re-anchoring"
  - path: crates/finder/src/imaging/run.rs
    other_spec: 037-imaging-run
    other_story: US1
    disposition: merge_order
    reason: "US1 depends_on_merged US12, and adds the view, the Runner and its task after what US12 wrote"
  - path: crates/finder/tests/console/main.rs
    other_spec: 036-tracking-from-subs
    other_story: US8
    disposition: merge_order
    reason: "037 depends_on_landed 036-tracking-from-subs, whose US8 last writes this file in the drafts; re-read at the re-anchoring"
  - path: crates/finder/tests/console/main.rs
    other_spec: 037-imaging-run
    other_story: US1
    disposition: merge_order
    reason: "US1 depends_on_merged US12, and adds mod imaging_run"
  open_questions: []
```
