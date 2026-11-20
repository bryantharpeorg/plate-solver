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
view and keeps `run.json`. `041-imaging-run-on-the-rig` then writes the console's own
mount, camera, focus, solver and stack parts and adds the centring, the focus gate and the
limits to the machine, and `042-imaging-run-page-and-mcp` adds the run block to the
Imaging page, four MCP tools and the README section. The run asks Track and nothing else
to move the mount, asks nothing of the mount when it ends but a Stop when it is itself
stopped while still slewing, and holds no number sized to one telescope.

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

**Scale/Scope**: one module and two of its three children (`script.rs`, `store.rs`),
three routes, in four stories.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Nothing touches the solver, detection or `crates/math-core`. The order of the steps, the centring tolerances in use elsewhere and the forced stop of a solve loop that never converges are grounded in `docs/research/2026-11-15-imaging-camera.md` § 12. Prior art and what was not adopted; the 60″ of GoTo and the tracker's endings in the code cited in history.md § Clarifications. The run's own counts and limits are owner decisions, named as such. |
| II. Never weaken a gate | No existing test, golden, fixture, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS`, `ergane.yaml` or `playwright.config.ts` is edited. The only edits to existing test files are the `mod` lines spec.md SC-002 names. |
| III. Tests first, independent expectations | Oracles: for the machine, the words, states and call lists each scenario states, compared with what a scripted stand-in recorded; for the real parts, the simulator's truth (the true aim point, the true centre of the imaging frame, the mount's true motion), the session, and the answers of 031's and 033's own routes. `run.json` and `stack.json` are parsed by the tests themselves. Clocks are injected; virtual time only in tests and `finder --simulate`. |
| IV. Critical paths | No `unsafe`, no dependency or `Cargo.toml` change, no database-format change, nothing under `proto/`, nothing in `crates/scope-link`. |
| V. Proven open source first, honest licensing | The finder camera's solver is untouched. The run reaches ASTAP only through 032's solver port; principle V of constitution 1.3.0 allows that. |

## Project Structure

```text
crates/finder/
├── src/lib.rs                            # US3: three route lines
├── src/api.rs                            # US3: imaging_runner, with_imaging_run_steps
├── src/imaging/mod.rs                    # US12: pub mod run; US3: run in ImagingView, attach_run
├── src/imaging/routes.rs                 # US3: three routes
├── src/imaging/run.rs                    # US12: new, the parts and the words; US1: the view and the machine; US2: failed steps, counts; US3: the record
├── src/imaging/run/script.rs             # US12: new, the scripted stand-in
├── src/imaging/run/store.rs              # US3: new, run.json
├── tests/console/main.rs                 # US12, US1, US2, US3: one mod line each
├── tests/console/imaging_run.rs          # US1: new
├── tests/console/imaging_run_steps.rs    # US2: new
├── tests/console/imaging_run_routes.rs   # US3: new
└── tests/console/imaging_run_words.rs    # US12: new
```

`041-imaging-run-on-the-rig` adds `src/imaging/run/console.rs` and more of `run.rs`, and
`042-imaging-run-page-and-mcp` the page, the MCP tools and the README.

## Interfaces

Everything 030 to 036 create is named as their plan.md gives it in the drafting
worktree: each of the seven reviewed by a second agent, and all the trios of the series
audited side by side. All of it is re-read on `main` before the flip (flip.md
flip.md § Prerequisites 2).

### What 030 to 036 offer (read, never written, by this spec)

| What a part needs | From | Name | Where it is written |
|---|---|---|---|
| why a request was not carried out | 031 | `crate::imaging::Refusal { status, message }` | 031 plan.md § The console's imaging state |
| the UTC text of a time | 031 | `crate::imaging::fits::utc(unix_s) -> (String, String)` | 031 plan.md § FITS at the console |
| a target's name as a folder's name takes it | 035 | `crate::imaging::stack::service::label(name: &str) -> String` | 035 spec.md FR-035, US8-S3; 035 plan.md § The stack on the console |
| the imaging folder | 034 | `Imaging::root()` | 034 plan.md § 031's imaging state |
| how far from the set point a stack may begin and still be steered | 036 | `crate::imaging::steering::FAR_ARCSEC`, 180.0 | 036 plan.md § The steering; 036 spec.md FR-014 |

The rest of what 030 to 038 and 040 offer is read by the console's own parts, which
`041-imaging-run-on-the-rig` writes.

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
// US3: pub mod store;        041's US4: pub mod console;

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
    /// `focus_pause` (041's US6).
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
    /// Go on from a focus pause (041's US6). Until then always refused.
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
clock's; everything measured (`elapsed_s`, and from 041's US6 and 041's US7 the pause and the time
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
  yet, and the only thing the run has done is start that motion (owner decision 14). `steady(None)` leaves the state word as it is and,
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
   verdict: US2 adds the warning of `SlightlySoft`, and 041's US6 puts the pause where
   `ClearlyOut`, `Unmeasurable` and `NoRecord` go on. `Err(Stopped)` ends the run
   `CheckStopped`: `guarded` has already taken the run's own stop and the mount's
   ending, so a `Stopped` that reaches the machine from `focus.check` is a stop from
   outside. Any other `Err(f)` sets `focus` to `unmeasurable` with no reading, no
   record and the unit `px`, adds the warning
   `The focus check did not finish: {words}.`, keeps the failure, for the words of 041's US6's
   pause, and goes on as `Unmeasurable` does; US2 first sorts two kinds of failure out
   of it. Whenever the run goes on from here to
   step 4, it first takes the lease again: `camera.hold()`, whose `Err(words)` ends the
   run `CameraFailed(words)`.
4. The state is `solving`. `frame()`, then `guarded(solver.solve(&frame, &target))`.
   `Ok(centre)` sets `centring` to `{tries: 0, offset_arcsec, tolerance_arcsec}` and
   goes on: 041's US5 adds the loop here. `Err(Stopped)` ends the run `Stopped`. In US1 any
   other `Err(f)` ends it `SolverFailing` with `f`'s words; US2 replaces that arm.
5. The state is `stacking`, with its message; `camera.release()`, because the stack
   takes the lease itself; then `stack.start(&target).await`, not guarded. `Ok(folder)`
   sets the view's `folder`, its `stacking_since`, the console clock's time then, and
   its `limit_at`, that time plus `settings.duration_s`: where the limit will fall on
   the console's clock, the limit itself being measured on tokio's (041's plan.md § Limits and the
   stack's endings in the machine).
   `Err(Stopped)` ends the run `Stopped`. In US1 any other
   `Err(f)` ends it `StackFailed` with `f`'s words; US2 replaces that arm. A stack whose
   start failed is not stopped.
6. Every `POLL`: `mount.state()` `Ended(e)` ends the run with `e`; a stop ends it
   `RunStopped`. In US1 the counts stay at zero and the state stays `stacking`: US2 reads
   `stack.progress()` here, and 041's US7 adds the limits and the stack's own endings.

`stack.start`, `stack.stop`, `focus.open_aid`, `focus.close_aid`, `mount.repoint` and
`mount.stop` are awaited unguarded: dropping one half-way would leave a stack, a focus
aid or Track in a state nobody owns. Each is bounded by the part behind it: `repoint`
by `FRESH_POLLS` polls, Track's own wait of up to 2 s for the axes to rest, one Track
off and one Track on, about 7 s in all (041's plan.md § The console's mount part); `mount.stop` by the
console's Stop; the other four by 033's and 035's own starts and stops, and
`stack.start` on the console's own part also by the writing of its reserve, which takes
as long as the disk needs for that file (041's plan.md § The console's stack part). A stop of
the run that arrives during one is acted on as soon as the call has answered: the stop
signal stays set, and the next `guarded` or `steady` ends the run `RunStopped` (spec
FR-005, US5-S3).

**Ending**, in this order: from US2 the run notes that it is ending, which changes its
`message` only, to `Saving the stack: {stacked} stacked.` when step 5 returned `Ok`
(§ The run's words); `focus.close_aid()`, once, if the run gave up a check or,
from 041's US6, is ending from a focus pause, whether or not the aid could be opened;
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

**`resume`**, from US1 until 041's US6 gives it its work, refuses with 409
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
progress. 041's US5 and 041's US7 use its last two rows.

| A failed step | `Stopped` | `Camera` | `Save` | `SolverUnavailable` | `NotSolved` | `Mount`, `Other` |
|---|---|---|---|---|---|---|
| `camera.take` | `Stopped` | counted | `SaveFailed` | counted | counted | counted |
| `focus.check` | `CheckStopped` | unmeasurable | `SaveFailed` | `SolverUnavailable` | unmeasurable | unmeasurable |
| `solver.solve` | `Stopped` | counted | `SaveFailed` | `SolverUnavailable` | counted | counted |
| `stack.start` (US2), and `StackProgress::failed` (041's US7) | `Stopped` | `CameraFailed` | `SaveFailed` | `SolverUnavailable` | `SolverFailing` | `StackFailed` |
| `mount.repoint` (041's US5) | `Stopped` | `MountFailed` | `MountFailed` | `MountFailed` | `MountFailed` | `MountFailed` |

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
  mount and the stop come before anything the stack reports (041's US7 puts its rules after
  them), so that a lost link, which also fails the stack's next take, reads
  `link_lost` when Track has seen it. Nothing in US2 reads `running` or `failed`: 041's US7
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
| `focus_pause` (041's US6) | the focus sentence below, then ` Turn the focuser while watching the focus aid below, then check focus again or go on anyway.`; for `unmeasurable`, then ` Check for cloud or dew, then check focus again or go on anyway.`; for `no_record`, then ` Focus with the focus aid, then check focus again or go on anyway.`; and for a check that did not finish, unless its failure is `NotSolved`, which reads as `unmeasurable` does, in place of both: `The focus check did not finish: {w}. Check the imaging camera, then check focus again or go on anyway.`, `{w}` the failure's words |
| `solving` | `Solving an imaging frame.` |
| `centring` (041's US5) | `Centring {name}: {offset} from the sensor's centre, try {n} of 3.` after a re-centre was asked for; `Centring {name}: {offset} from the sensor's centre, no new aim point yet.` after a solve that measured no new aim point |
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
| going on anyway (041's US6) | `Focus was not checked again: the observer went on anyway.` |
| the focus aid (041's US6) | `The focus aid could not be opened: {words}.` |
| the stack at an ending (US2) | `The stack did not end cleanly: {words}.` |
| subs not removed because nothing was stacked (US2) | `The subs were kept: nothing was stacked.` |
| the record at an ending (US3) | `The run's record could not be saved: {words}.` |
| a target stacked off the centre after the last try (041's US5) | `{name} is {offset} from the sensor's centre after 3 tries; the stack's steering will bring it in.` while the offset is at most 036's `FAR_ARCSEC`, 180″; over it, `{name} is {offset} from the sensor's centre after 3 tries; it is stacked where it is.` |
| a target stacked off the centre after four solves that measured no new aim point (041's US5) | `{name} is {offset} from the sensor's centre and no new aim point could be measured; the stack's steering will bring it in.` while the offset is at most `FAR_ARCSEC`; over it, `{name} is {offset} from the sensor's centre and no new aim point could be measured; it is stacked where it is.` |
| a focus check that would pause, twice not steady (041's US6) | `The tracker was not steady during the focus check.` |

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
sides; the steering's block on the page then says what the steering does (040 plan.md
§ The page).

**`paused_by_tracker` names no cause.** 040's stack waits before an
exposure for more than a mount that settles: its view's `steering.pause` has a word for
a start, a take-up, a measurement, the keyhole pass, a push and a stopped axis, and
`limit` while the tracker is steady and its exposure limit is under half the exposure
held (040 FR-021, FR-022). The stack part hands the machine `Steering::waiting()` alone,
so the run reads no such word, and its one sentence is true of every one of them. 040's
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
tests set counts on a stacking run, and 041's US7 then makes a stack that no longer runs end
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

It is public product code that only tests call (owner decision 16, owner approval
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

The words `stacking` and `paused_by_tracker` can change places for hours: the steering
waits before an exposure whenever the tracker is not steady, and whenever its exposure
limit is under half the exposure held (040 FR-022). So a change between those
two is listed as an event, and written, only when no such change was listed in the
`RECORD_EVERY` before it, on tokio's clock. One that is held back is neither an event
nor a write; the view's `state` and `message` change at once all the same, and the
record's `state` is then the view's as of its last write (US3-S11). The first pause
after the stack has started is therefore always written (US3-S3).

### The console (`crates/finder/src/api.rs`, US3)

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

`041-imaging-run-on-the-rig` adds `with_imaging_run` here, and
`042-imaging-run-page-and-mcp` one line of `start_motion`.

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

## Shapes that existing tests pin

These are not changed by any story:
- **`GET /api/session` and the MCP session bodies without a run.** `run` is skipped when
  absent, so every existing body is byte for byte what it was.
- **031's imaging view and its tests.** `ImagingView` is `#[non_exhaustive]` and its
  tests compare it key by key (031's spec.md § User Scenarios), so a `run` key that
  appears only after a start changes none of them. `Imaging::take`, `ended`, `abort`
  and `set`, and their refusals, are 031's and unchanged; so is the lease (031 US9).
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
    US2-S6 to US2-S10, which assert the machine as US1 leaves it: their tests pass
    before US2's code is written and after it.
  - US3 adds `Runner::with_folder` and leaves `Runner::new` writing nothing.
- **Finder's test layout.** `crates/finder/tests/layout/main.rs` fails a file at the top
  level of `crates/finder/tests` (`crates/finder/tests/layout/main.rs:109`) and any file
  added to `crates/finder/tests/support` (`crates/finder/tests/layout/main.rs:127`).

The stories of 041 and 042 change what the task does in the same way, and
leave every scenario of this spec passing.

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
   `Store`; and no test builds an expected message by calling `Ending::message`: US12-S1
   calls it, and compares what it answers with a sentence typed into the test.
10. **The view outside `ready`.** 031's view holds only its `state` then. *Check:* `run`
    is set where the view is built, after the state's own parts, in every state;
    US3-S5 reads it under `no_link`.
11. **Paused time and the blocking pool.** Tokio's paused clock runs on while a blocking
    task works. *Check:* `Store`'s writes and reads, and the stack part's removals and
    the writes of its reserve, are synchronous `std::fs`, the reserve written a chunk at
    a time with a yield between two (041's plan.md § The console's stack part), and no file of this
    spec calls `spawn_blocking`.
12. **A held step must be listed first.** A test that holds `take` and then reads
    `calls()` expects `camera.take` there. *Check:* the script lists a call when it is
    made and holds after.
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
    scale. The rule is about code: 041's US8-S9 reads the four files for what they
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
    take after the focus gate. US1-S1, US1-S2, US2-S7, US2-S8, US2-S10 and 041's US6-S11
    assert it on the script, and 041's US4-S2, 041's US4-S4, 041's US4-S6 and 041's US8-S8 through 031's own
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
    041's US4, 041's US5, 041's US6 and 041's US8 show it only as a test nextest ends after 5 minutes. *Check:*
    while the runner's lock is held no part is called, nothing that reads the imaging
    view is called and nothing is awaited: a step copies what it needs out of the lock,
    lets it go, calls the part, and locks again to record the answer (§ The machine).
24. **A later rule must not break an earlier story's test.** A story may not edit a test
    file an earlier story created. *Check:* the script's `set_stack` leaves `running`
    and `failed` alone and only `end_stack` ends a stack (§ The script); 041's US4-S1 asserts
    its solved offset inside the tolerance, so the loop of 041's US5 does not enter it; each
    story's `preserved_behavior` in § Refinement Record names what of the earlier
    stories it leaves passing; and a story that finds an earlier test failing
    escalates.

## Risks

| Risk | Mitigation |
|---|---|
| Another taker gets the lease in the instant of a hand-over. | The run's next step is refused with that holder's name and the run says so: a pause, `stack_failed` or `camera_failed` (spec § Edge Cases, US1-S2, US2-S4). Owner decision 15 names the other answer, a change to 033 and 035. |
| A story's diff passes 64 KiB. | Estimated from probe round 1's measured bytes, each within about 1 KB but for US12 and US1, which are within about 2 KB: US12 43.4 KB (66 %), US1 30.0 KB (46 %), US2 35.2 KB (54 %) and US3 47.1 KB (72 %). A factory build has run to 1.35 times its probe build: at that US12 would be about 89 % and US3 about 97 %, so US3 is the next story to cut. The second probe round measures all of it (flip.md § Prerequisites 5). |
| US1's refusals of a start, its stop, the mount's endings and its `Drop` are written in US1 and first tested in US2 (US2-S6 to US2-S10). | A fault in `run.rs` that one of those tests shows is US2's to correct, since US2 writes that file. A fault in `script.rs` is not: no story after US12 writes it, so US2 escalates. US12's own scenarios assert every control of the script with no run (US12-S3, US12-S4), which is what keeps that unlikely. |
| US12 writes public items that nothing in the crate calls until US1 lands: the traits, `Ending::message`, the script. | They are public items of a library, which the lints do not count as dead code, and US12's test module calls each of them. |
| A console dropped in the middle of a run keeps Track on through something the imaging state of 030 to 036 holds. | 037 adds no delay of its own: a dropped `Runner` aborts its task and lets its parts go (US2-S9). 035's stack holds the session until its take ends, so a test that drops a console while a stack runs waits more than 015's 2 s for the session. A console dropped during a focus pause leaves 033's aid, opened by the run, with no owner. In the binary the state lives until exit, so only tests are touched. |
| A stop of the run waits up to about 7 s when it arrives during a re-centre. | Said in spec FR-005 and owner decision 14, and asserted on the script (041's US5-S3). The console's Stop does not wait: it stops the mount at once, and the run then ends `stopped`. |
| The first imaging frame costs an exposure and a transfer the run does not use again. | Owner decision 20 names the two other answers. The first imaging frame is what tells a camera fault from poor focus. |
| Track left on after an ending is forgotten. | Track's own guards end it at the minimum altitude and at sunrise, and the top bar's Stop stays visible while Track is on (`DESIGN.md:308`). Owner decision 1 is where it changes. |
| The console exits mid-run and the stacked picture is lost: 035 saves it only when the stack ends. | Said in spec § Edge Cases and the README. `run.json` is refreshed each minute, the subs stay on the disk, and the page says after the restart that the last run did not finish. Saving the stack as it goes is 035's, and is asked of it. |

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
  - "the README is not written: its rows are 042's US11's (plan.md Decisions and the alternatives rejected)"
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
    reason: "037 depends_on_landed 035-live-stack, whose 041's US8 last writes this file in the drafts; re-read at the re-anchoring"
  - path: crates/finder/src/imaging/routes.rs
    other_spec: 036-tracking-from-subs
    other_story: US5
    disposition: merge_order
    reason: "037 depends_on_landed 036-tracking-from-subs, whose 041's US5 last writes this file in the drafts; re-read at the re-anchoring"
  - path: crates/finder/src/lib.rs
    other_spec: 036-tracking-from-subs
    other_story: US5
    disposition: merge_order
    reason: "037 depends_on_landed 036-tracking-from-subs, whose 041's US5 last writes this file in the drafts; re-read at the re-anchoring"
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
    reason: "037 depends_on_landed 036-tracking-from-subs, whose 041's US4 last writes this file in the drafts; re-read at the re-anchoring"
  - path: crates/finder/src/imaging/run.rs
    other_spec: 037-imaging-run
    other_story: US1
    disposition: merge_order
    reason: "US1 depends_on_merged US12, and adds the view, the Runner and its task after what US12 wrote"
  - path: crates/finder/tests/console/main.rs
    other_spec: 040-stack-waits-for-steady
    other_story: US8
    disposition: merge_order
    reason: "037 depends_on_landed 040-stack-waits-for-steady, whose 041's US8 last writes this file in the drafts; re-read at the re-anchoring"
  - path: crates/finder/tests/console/main.rs
    other_spec: 037-imaging-run
    other_story: US1
    disposition: merge_order
    reason: "US1 depends_on_merged US12, and adds mod imaging_run"
  open_questions: []
```
