# Implementation Plan: The Imaging Run on the Page, over MCP and in the README

**Branch**: `042-imaging-run-page-and-mcp` | **Date**: 2026-11-15, a spec of its own since 2026-11-20 | **Spec**: [spec.md](spec.md)

This plan records every design decision for the feature. A story that would change one
escalates rather than re-deciding it. The binding decisions above it are
`docs/plans/2026-11-15-specs-030-037-imaging.md` (the plan document),
`docs/adr/0002-imaging-cameras-through-indi-frames-relayed-console-stacks.md` and
`GLOSSARY.md`.

## Summary

`037-imaging-run` and `041-imaging-run-on-the-rig` have landed the imaging run: a `Runner`
on the console's own five parts, three routes, the run in the imaging view, and its record,
`run.json`. This spec puts the run where the observer and an agent reach it. US9 adds the
run block to the Imaging page, with its entries in `DESIGN.md` and
`.impeccable/design.json`; US10 four MCP tools, and the tool name `start_motion` refuses
beside a direct axis move; US11 the README's three API rows, four tool lines and section
`## An imaging run`, with the field checklist of the first imaging run. No story here
changes the run: what it does, its states, its endings and its words are 037's and 041's.

## Technical Context

**Language/Version**: Rust (MSRV 1.96) for US10 and US11; TypeScript with React 19 for
US9.

**Primary Dependencies**: nothing new. `finder` already has `serde` and `serde_json`
(`crates/finder/Cargo.toml:32`) and `rmcp`. The page adds no npm package.

**Storage**: none of its own. The page and the tools read the run's view; the README says
where 037 keeps `run.json`.

**Testing**:
- US9: `crates/finder/web/src/components/ImagingRunBlock.test.tsx` (new), Vitest. No new
  Playwright test (owner decision 17).
- US10: `crates/finder/tests/mcp/imaging_run.rs` (new), the module `imaging_run` of
  finder's `mcp` test binary.
- US11: `crates/finder/tests/console/imaging_run_docs.rs` (new), the module
  `imaging_run_docs` of `console`.
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
- `web/dist` is committed and must be the build of its source (US9).
- Budgets are things a test measures; this plan states none for the real telescope.

**Scale/Scope**: one page block, four MCP tools, one line of `start_motion`, one README
section, in three stories.

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
├── README.md                             # US11: three API rows, four tools, ## An imaging run
├── src/api.rs                            # US10: the tool name in start_motion
├── src/mcp/tools.rs                      # US10: four tools
├── tests/console/main.rs                 # US11: mod imaging_run_docs
├── tests/console/imaging_run_docs.rs     # US11: new
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

The run's code, its routes and its record are 037's and 041's, and no story here writes
them.

## What 037-imaging-run provides (read as landed before the flip)

`037-imaging-run` has landed all of this before any story here is built, and no story
here changes any of it: a test file 037 created is an existing test. 037's plan.md gives
each item in full, under the section named; the flip re-reads this section against the
code on `main` (flip.md § Prerequisites 2).

**The words** (`crates/finder/src/imaging/run.rs`; 037's § The parts and the words,
037's § The run's words and 037's § Failed steps and counts in the machine). 037's § The run's words
holds every state's message and every ending's sentence, which the page shows as the run
gives them and the README's tables quote:

```rust
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum By { Page, Mcp }

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

/// Imaging frames in a row the camera may fail before the run ends.
pub const FRAME_TRIES: u32 = 3;
/// Solves in a row that may find no solution before the run ends.
pub const SOLVE_TRIES: u32 = 3;
/// How long the run waits after a failed take before it asks again.
pub const RETRY_WAIT: Duration = Duration::from_secs(5);
```

`Ending::reason` answers an ending's word in `ENDINGS`, four endings answering `stopped`
with a sentence each, and `Ending::message` its sentence.

**The view and the runner** (`run.rs`; 037's § The machine). The page's `ImagingRun` in
§ The page is this view as JSON:

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

**`resume`**, from 037's US1 until 041's US6 gives it its work, refuses with 409
`no imaging run is running` when no run is on and with 409
`the imaging run is not paused` otherwise. No scenario of 037's US1 or 037's US2 calls it: 037's US3-S4 is
its oracle.

As 041's US6 leaves it:

`Runner::resume(anyway)` now: no run on, 409 `no imaging run is running`; a run not in
`focus_pause`, or one whose pause has already been answered by a resume, a stop, the
mount's ending or the limit, 409 `the imaging run is not paused`; else it hands
`anyway` to the task and answers the view. The first resume is the one acted on: a
second tap, or the page and an MCP client together, cannot turn a check into a
go-anyway, and a `Go anyway` sent as the limit fires is not answered yes by a run that
then ends.

**The routes and the imaging view** (037's § The routes,
037's § `crates/finder/src/imaging/mod.rs` and 037's § The console).
`POST /api/imaging/run` takes a `RunRequest` through `sim_json`, so a `Content-Type`
other than `application/json` is 415 and a body of another shape 400;
`DELETE /api/imaging/run` takes no body and answers once the run has ended;
`POST /api/imaging/run/continue` takes `{"anyway": bool}`. Each is a `SessionWrite`,
answers the imaging view, and with no runner is 409 `this console runs no imaging runs`.
The route starts a run inside `api::start_motion(&state, "page", "imaging_run", …)`. The
imaging view carries `run` in every state of the view, and `unfinished_run`, the folder of
the newest record with no `ended`, until the first accepted start; both are skipped when
absent. `AppState::imaging_runner()` answers the runner when the console has one.

**The record** (`crates/finder/src/imaging/run/store.rs`; 037's § The record). `run.json`
lies in the stack's folder, `runs/<stack id>/` under the imaging folder, or in
`runs/<run id>/` for a run that ends before it stacks; it is refreshed at changes of state,
at most once a `RECORD_EVERY`, 60 s, while the stacked count rises, and at the ending.
`run.json`, its keys in this order: `format`, `finder` (`CARGO_PKG_VERSION`), `id`,
`by`, `target`, `started_at`, `started_utc` (`YYYY-MM-DDTHH:MM:SSZ`, from the same
`fits::utc` string as the id), `settings`, `state`, `events`, `subs`, then `focus`,
`centring`, `warnings` and `ended` when the view has them, `ended` with `saved` and
with `track_on` as read at the ending. No `message`, `elapsed_s`, `stacking_since`,
`limit_at`, `pause_ends_at` or `folder`, and nothing of the site, the node's address or
its token.

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

## What 041-imaging-run-on-the-rig provides (read as landed before the flip)

`041-imaging-run-on-the-rig` has landed all of this on 037's machine before any story
here is built, and no story here changes any of it: a test file 041 created is an existing
test. 041's plan.md gives each item in full, under the section named; the flip re-reads
this section against the code on `main` (flip.md § Prerequisites 2).

**The binary has a runner** (041's § The console's own parts in the console).
`AppState::with_imaging_run()` builds `Steps` from the console's own five parts and calls
037's `with_imaging_run_steps`; `crates/finder/src/main.rs` calls it on both the
`--simulate` and the real path. A console without imaging has no runner.

**The counts, waits and names the README gives** (`crates/finder/src/imaging/run.rs` and
`crates/finder/src/imaging/run/console.rs`; 041's sections of each):

```rust
// console.rs, 041's § The console's mount part
pub const CENTRE_SHARE: f64 = 0.05;
pub const CENTRE_FLOOR_ARCSEC: f64 = 90.0;
/// Polls a re-centre waits, at most, after its own Track off, for the finder camera's
/// next live pointing: 5 s at `POLL`.
pub const FRESH_POLLS: u32 = 20;
/// How long 038's push may read `moving` before it ends a run.
pub const PUSH_GRACE: Duration = Duration::from_secs(10);
/// The share of the imaging frame's shorter side within which a target that could not
/// be centred is stacked all the same.
pub const CENTRE_NEAR_SHARE: f64 = 0.25;
// console.rs, 041's § The console's camera and solver parts
/// The name a run holds the imaging camera's lease under.
pub const HOLDER: &str = "the imaging run";
// run.rs, 041's § Centring in the machine
/// Re-centres a run may ask for.
pub const CENTRE_TRIES: u32 = 3;
/// Solves in a row, outside the tolerance, that may measure no new aim point.
pub const CENTRE_IDLE: u32 = 4;
// run.rs, 041's § The focus gate in the machine
/// How long a focus pause may last.
pub const FOCUS_PAUSE_LIMIT: Duration = Duration::from_secs(900);
// run.rs, 041's § Limits and the stack's endings in the machine
/// Stacking time with a failed sub and none stacked that ends a run.
pub const NO_SUB_LIMIT: Duration = Duration::from_secs(600);
/// Reasons 035 drops a sub for that say the console fell behind, not that a sub failed.
pub const NOT_FAILURES: [&str; 2] = ["behind", "unprocessed"];
// console.rs, 041's § The console's stack part and the wiring
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

- **The centring tolerance** is
  `(CENTRE_SHARE * shorter * solve.arcsec_per_px).max(CENTRE_FLOOR_ARCSEC)`, `shorter`
  being the smaller of the `width` and `height` of the imaging frame solved, and
  `near_arcsec`, within which a target left off the centre after the last try, or after
  `CENTRE_IDLE` solves in a row that measured no new aim point, is stacked with a
  warning, is `CENTRE_NEAR_SHARE * shorter * solve.arcsec_per_px`; when that frame is no
  longer the view's, both are `CENTRE_FLOOR_ARCSEC` (041's § The console's camera and
  solver parts).
- **Steady before every imaging frame of the run's own**: the mount part reads steady
  only when the tracker has been steady for 032's `relation::FIT_REACH_S`, 10 s, or the
  exposure setting when that is longer (041's § The console's mount part).
- **The focus pause**: `focus_pause`, its `pause_ends_at` and `Runner::resume(anyway)` in
  a pause, as above (041's § The focus gate in the machine).
- **The stack part**, whose stop finishes on a scope node that has stopped answering
  (041-R17; with 043 landed, flip.md § Prerequisites 2): the `.reserve` file in the
  stack's folder while the run stacks,
  `reserve_bytes` long and removed before the stack's save, and the subs removed when
  `keep_subs` is false (041's § The console's stack part and the wiring).
- **The steering**: the stack part tells the steering of 036 and 040 the set point and
  reads `Steering::waiting()` for `paused_by_tracker`; the steering's own block on the
  Imaging page is 040's.

## Interfaces

Everything 030 to 036, 038 and 040 create is named as their plan.md gives it in the
drafting worktree, and 037's and 041's as their plan.md give it. All of it is re-read on
`main` before the flip (flip.md § Prerequisites 2).

### `start_motion`'s one line (`crates/finder/src/api.rs`, US10)

US10 changes one line of `start_motion`, the refusal of an MCP Track while a direct axis
move runs (`crates/finder/src/api.rs:623`): `tool == "set_tracking"` becomes
`matches!(tool, "set_tracking" | "start_imaging_run")`.

### The page (`crates/finder/web/src/`, US9)

`api.ts`, after the imaging types 030 to 036 and 040 put there:

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
  `{m}` minutes, ÷ 10⁹ (the whole sensor, whatever the region). It is an estimate at
  the exposure as set: 036 may cap a sub
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
`DESIGN.md:350` today, and 030 to 036 and 040 add theirs after it), a plain `###` heading and
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

US11 is the one story of the run's three specs that writes the README, so that every line it gains
is asserted by the story that writes it (US11-S1 to US11-S3).

Three rows after the last `/api/imaging` row of the `## API` table
(`crates/finder/README.md:167`):

```markdown
| `POST /api/imaging/run` | JSON with optional `subs` (1 to 100000), `duration_s` (1 to 43200, default 7200) and `keep_subs` (default true). Start an imaging run on the session's target: GoTo, tracking, a first imaging frame, a focus check, centring on the imaging camera's sensor and stacking. Answers 200 with the imaging view, whose `run` says where it has got to; 400 or 409 with a JSON `error`. Same-origin writes only. |
| `DELETE /api/imaging/run` | No body. End the imaging run as `stopped` and save its stack; answers 200 with the imaging view once it has ended. It stops the mount only while the run still slews: `POST /api/stop` stops it at any time, and ends the run too. Same-origin writes only. |
| `POST /api/imaging/run/continue` | JSON `{"anyway":false}` or `{"anyway":true}`. From a focus pause, check focus again, or go on unchecked; 409 with a JSON `error` when no run is paused. Same-origin writes only. |
```

In `## MCP clients` (`crates/finder/README.md:57`): four lines after `set_steering`'s,
the last tool 040 adds (040's US11), worded like their neighbours, the first saying
that `start_imaging_run` moves the telescope, is rate-limited and is logged; and the
count in the sentence before the list made four higher, in the words that sentence has
on `main` then.

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
four sentences, each between backticks, and what the mount does after each (the
run's own stop: Track on; the console's Stop, or a stop of the run while Track is off
(while it slewed, or once a guard or a lost link had ended Track): at rest; the
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
- **037's and 041's tests.** A test file 037 or 041 created is an existing test, and no
  story here edits it; no story here writes `run.rs`, its children or the routes, so every
  scenario of 037 and 041 holds as they left it.
- **Finder's test layout.** `crates/finder/tests/layout/main.rs` fails a file at the top
  level of `crates/finder/tests` (`crates/finder/tests/layout/main.rs:109`) and any file
  added to `crates/finder/tests/support` (`crates/finder/tests/layout/main.rs:127`).
- **031's Imaging page tests, and those 032 to 036 and 040 add.** With a `ready` view and
  no run the block adds one region, two number fields, one checkbox and one button, and
  no `status` or `alert`.
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
13. **The rate limit in tool tests.** Two `start_imaging_run` calls in one virtual
    second are both counted, refused or not. *Check:* the tests of US10 leave 1 s
    between starts, except US10-S4 and US10-S3's three starts in one second.
14. **`useImaging` reads every second.** A draft typed into a field would be lost on
    each read. *Check:* the block keeps its fields in its own state; US9-S1 types,
    renders the block again with a new view, then clicks.
15. **The bundle must be rebuilt.** *Check:* US9 commits `crates/finder/web/dist` after
    `npm run build`.
16. **027's layout.** *Check:* each new test is a module file in its binary's folder
    with one alphabetical `mod` line, taking its folder from `crate::support::Scratch`,
    and holding only the setup its own tests use.
17. **Each worktree builds on its own.** *Check:* no story sets `CARGO_TARGET_DIR`.
22. **A code span that ends in a colon and a number is read as a citation.** The
    validator resolves it as `path:line`. *Check:* times, ports and JSON fragments in
    this trio never end a code span that way.
24. **A later rule must not break an earlier story's test.** A story may not edit a test
    file an earlier story created. *Check:* the script's `set_stack` leaves `running`
    and `failed` alone and only `end_stack` ends a stack (037's plan.md § The script); 041's US4-S1 asserts
    its solved offset inside the tolerance, so the loop of 041's US5 does not enter it; each
    story's `preserved_behavior` in § Refinement Record names what of the earlier
    stories it leaves passing; and a story that finds an earlier test failing
    escalates.

## Risks

| Risk | Mitigation |
|---|---|
| A story's diff passes 64 KiB. | Probe round 2 built the three stories: US9 39,648 B (60.5 %) as a plain diff and 38,951 B (59.4 %) with `crates/finder/web/dist` excluded (§ Plan traps 3), US10 22,881 B (34.9 %) as written and US11 21,834 B (33.3 %), against estimates of 37.7, 23.9 and 20.3 KB from round 1. Factory builds of 036, 039 and 040 ran 0.85 to 1.17 times their probes' diffs, so each story is estimated at 1.17 times its build with the cases the round's fold added, as tests: US9 (39,648 + 2,623) × 1.17 = 49,457 B (75.5 %), US10 (22,881 + 2,415) × 1.17 = 29,596 B (45.2 %) and US11 (21,834 + 1,574) × 1.17 = 27,387 B (41.8 %). Each is under owner choice 14's 60 KB, so no story is cut. US9 is the story to watch: US9-S8 to S13 can become a story of their own if a factory build passes 60,000 B. No story drops or shortens a case to get under. |
| An earlier page test queries a control the block also has. | The block's controls have their own names and it shows no `status` or `alert` before a run; flip.md § Prerequisites 5 has what probe round 2 found. |
| The real GoTo and Track are unproven. | flip.md § Prerequisites 8. The checklist of FR-041 starts with an imaging frame and a solve by hand before the first run. |

## Refinement Record

The re-anchoring (flip.md § Prerequisites 2) is still to do. Until then the record below
names what the drafts named: its tests are the scenarios, its writes are the files
§ Project Structure names, and its shared writes name the merge edge that orders two
stories of this spec on one file, or the story of another spec that last writes the file
among the drafts as they stood on 2026-11-15, read from each draft's own record, a story
of 036 that went to 040 when 036 was split being named in 040. `tools.rs`,
`tests/mcp/main.rs` and the README name 040's US11, which writes them last in 040's trio
and in probe round 2's build (042-R2). Each of those names is
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
    assertion: "a refused Check focus again shows its words in a second alert with no refresh, the alert goes when the run's state changes and when another call of the block starts, the pause buttons and the stop button are disabled while the request is out, and a request that got no answer shows the block's own words"
    decision: "spec.md:US9-S10"
  - path: crates/finder/web/src/components/ImagingRunBlock.test.tsx
    status: new
    disposition: add
    assertion: "the start form shows 120 as the placeholder of Minutes to stack, the quiet line with the two-hour default, and the size of the subs for the minutes typed or 120, divided by the binning squared and for only the exposures that fit, and a view with unfinished_run shows the alert that the last imaging run did not finish, with Start imaging run still offered"
    decision: "spec.md:US9-S11"
  - path: crates/finder/web/src/components/ImagingRunBlock.test.tsx
    status: new
    disposition: add
    assertion: "a focus pause shows the time it has left from pause_ends_at, never below zero, and a stacking run the time stacked from stacking_since against its own limit, while Elapsed still counts from the press"
    decision: "spec.md:US9-S12"
  - path: crates/finder/web/src/components/ImagingRunBlock.test.tsx
    status: new
    disposition: add
    assertion: "a click on Start imaging run posts the request given, sends no limit typed as 0 and no fraction of a sub, reads Starting and is disabled while its request is out, keeps a draft across a new view, shows a refusal as an alert with no refresh, and drops that alert once a run appears"
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
    other_spec: 040-stack-waits-for-steady
    other_story: US8
    disposition: merge_order
    reason: "042 depends_on_landed 040-stack-waits-for-steady, whose US8 last writes this file in the drafts; re-read at the re-anchoring"
  - path: DESIGN.md
    other_spec: 040-stack-waits-for-steady
    other_story: US8
    disposition: merge_order
    reason: "042 depends_on_landed 040-stack-waits-for-steady, whose US8 last writes this file in the drafts; re-read at the re-anchoring"
  - path: crates/finder/web/dist
    other_spec: 040-stack-waits-for-steady
    other_story: US8
    disposition: merge_order
    reason: "042 depends_on_landed 040-stack-waits-for-steady, whose US8 last writes this file in the drafts; re-read at the re-anchoring"
  - path: crates/finder/web/src/api.ts
    other_spec: 040-stack-waits-for-steady
    other_story: US8
    disposition: merge_order
    reason: "042 depends_on_landed 040-stack-waits-for-steady, whose US8 last writes this file in the drafts; re-read at the re-anchoring"
  - path: crates/finder/web/src/components/ImagingPage.tsx
    other_spec: 040-stack-waits-for-steady
    other_story: US8
    disposition: merge_order
    reason: "042 depends_on_landed 040-stack-waits-for-steady, whose US8 last writes this file in the drafts; re-read at the re-anchoring"
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
    assertion: "get_imaging_run answers null before a run, start_imaging_run answers a run by mcp with the settings asked for and points the mount as mcp, the read after it answers the same run but for the keys that change, stacking_since and limit_at among them, and exactly one call-log line is written, for the start"
    decision: "spec.md:US10-S2"
  - path: crates/finder/tests/mcp/imaging_run.rs
    status: new
    disposition: add
    assertion: "bad arguments, an unknown key of continue_imaging_run, a run that is not on and a console given no parts, each tool called with arguments it would otherwise accept, are tool errors with the routes' words, a start's bad body in the route's 400 words and continue's missing key in serde's, and a third start in one second meets the rate limit whatever the first two asked"
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
    assertion: "after continue_imaging_run with anyway true and a settle get_imaging_run answers a run no longer in focus_pause with the go-anyway warning, while the tool's own answer is still in focus_pause"
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
  - path: crates/finder/src/api.rs
    other_spec: 041-imaging-run-on-the-rig
    other_story: US8
    disposition: merge_order
    reason: "042 depends_on_landed 041-imaging-run-on-the-rig, whose 041's US8 last writes this file; re-read at the re-anchoring"
  - path: crates/finder/src/mcp/tools.rs
    other_spec: 040-stack-waits-for-steady
    other_story: US11
    disposition: merge_order
    reason: "042 depends_on_landed 040-stack-waits-for-steady, whose US11 last writes this file with `set_steering`; re-read at the re-anchoring"
  - path: crates/finder/tests/mcp/main.rs
    other_spec: 040-stack-waits-for-steady
    other_story: US11
    disposition: merge_order
    reason: "042 depends_on_landed 040-stack-waits-for-steady, whose US11 last writes this file with `set_steering`; re-read at the re-anchoring"
  open_questions: []
 US11:
  tests:
  - path: crates/finder/tests/console/imaging_run_docs.rs
    status: new
    disposition: add
    assertion: "the README's An imaging run section, read to the next section or to the end of the file, names every state word, every ending reason, the three routes, the four tools and run.json between backticks, and its prose before the field checklist contains corner, 7200 and What to do next, the heading of the fourth column of its table of endings, the reserve file, the exposure rule, a console that exits mid-run and the four sentences of stopped"
    decision: "spec.md:US11-S1"
  - path: crates/finder/tests/console/imaging_run_docs.rs
    status: new
    disposition: add
    assertion: "the field checklist, read to the next heading or to the end of the file, has exactly eighteen steps numbered in order, names Start imaging run, start_imaging_run, continue_imaging_run, Stop and run.json, its step 4 sets the imaging exposure at least as long as the finder camera's, and its step 8 has the observer look at the corners of the run's own solved imaging frames"
    decision: "spec.md:US11-S2"
  - path: crates/finder/tests/console/imaging_run_docs.rs
    status: new
    disposition: add
    assertion: "the API table has a row for each of the three routes, the start's giving its default of 7200, and MCP clients names the four tools"
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
    other_spec: 040-stack-waits-for-steady
    other_story: US11
    disposition: merge_order
    reason: "042 depends_on_landed 040-stack-waits-for-steady, whose US11 last writes this file with `set_steering`'s tool line; re-read at the re-anchoring"
  - path: crates/finder/tests/console/main.rs
    other_spec: 041-imaging-run-on-the-rig
    other_story: US7
    disposition: merge_order
    reason: "042 depends_on_landed 041-imaging-run-on-the-rig, whose 041's US7 last writes this file; re-read at the re-anchoring"
  open_questions: []
```
