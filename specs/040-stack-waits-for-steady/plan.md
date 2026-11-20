# Implementation Plan: The Stack Waits for a Steady Tracker, and Shows the Steering

**Branch**: `040-stack-waits-for-steady` | **Date**: 2026-11-20 | **Spec**: [spec.md](spec.md)

This plan records every design decision for the feature. A story that would change one
escalates rather than re-deciding it.

## Summary

036 has made the subs steer the tracker through `Steering`, in the gaps between
exposures, and capped a stack's exposure (§ What 036 provides). Make the tracker say when
it is not steady: during a backlash take-up, a first reversal's measurement and the
keyhole pass, while a hand moves the telescope, for 2 s of settling after its first
rates, a take-up and the keyhole pass, in a turn that finds an axis stopped, and after
tracking ends; it says why in a word of its own, and while it is not steady its own
corrections run as 015 has them, even after a lapse. Make the steering wait before an
exposure while the tracker is not steady or its limit is under half the exposure held,
letting the tracker go for the second, and tell the stack to drop a sub taken across a
time the tracker was not steady. Wire it into 035's taker and worker, the taker setting
the capped exposure through the stack's own lease on the camera and putting the
observer's back as the stack ends; give the Imaging page a block for the readout of the
tracking error and the drift, with the steering's four settings; and give an MCP client
`set_steering`, which sets what the route sets. One story of tests only, cut from US6,
tests the tracker at the edges of a time not steady.

## Technical Context

**Language/Version**: Rust (MSRV 1.96) for US6, US7, US8's readout, US10 and US11;
TypeScript with React 19 for US8's page.

**Primary Dependencies**: nothing new, and no `Cargo.toml` changes. `finder` already has
`tokio` with `sync` and `time`, `serde` and `axum`.

**Storage**: none. The readout is run-only.

**Testing**:
- US6: `crates/finder/tests/motion/steering_steady.rs` and
  `crates/finder/tests/console/steering_steady.rs` (new).
- US7: `crates/finder/tests/motion/steering_stack.rs` (new).
- US8: `crates/finder/tests/console/steering_readout.rs` and
  `crates/finder/web/src/components/SteeringBlock.test.tsx` (new, Vitest).
- US10: `crates/finder/tests/motion/steering_unsteady.rs` (new): tests of US6's tracker.
- US11: `crates/finder/tests/mcp/steering.rs` (new), through the MCP door.
- Where a finder test goes is 027's rule: `console` calls the console in process and
  moves no mount (`crates/finder/tests/console/main.rs:2`), `motion` drives the
  simulated mount in paused time (`crates/finder/tests/motion/main.rs:1`). Each module
  is declared by one `mod` line, in alphabetical order, in its binary's `main.rs`
  (`crates/finder/tests/layout/main.rs:156`), and writes its own setup:
  `crates/finder/tests/support` takes no new file
  (`crates/finder/tests/layout/main.rs:127`). A module may use what the existing support
  files already offer: `crate::support::tracker` (the backlash rig, US6 and US10),
  `crate::support::goto` (US6-S6) and `crate::support::Scratch`
  (`crates/finder/tests/support/mod.rs:14`). The rigs and the scripted tracker of
  spec.md are written out in each module that uses them, only the parts its own tests
  use, because an unused helper fails `clippy -D warnings` and a story may not edit an
  earlier story's test file, 036's included. 035's rigs live in its `console` binary,
  which the `motion` binary cannot import: the stand-in stacking rig, `ScriptedSolver`,
  `wait_stack` and `cards` in `stack_loop`, the scripted camera in `stack_camera` and
  the sim stacking rig in `stack_gate` (035 plan.md § Test rigs, since its split of US5).
  US7 writes the stacking rig of spec.md itself. `mcp` goes through the MCP door and can
  import neither binary's rigs: US11's module builds its own console, as 031's and 035's
  `mcp` modules do.
- No Playwright test (owner decision 12).

**Target Platform**: the `finder` console on the console machine. Nothing changes on the
scope node or the scope link.

**Constraints**:
- No change under `crates/scope-node`, `crates/scope-link`, `crates/simulator` or
  `proto/`; no `unsafe`; no dependency.
- No file under `crates/finder/src/imaging/` computes an altitude, an azimuth, a
  parallactic angle or a field rotation, or reads an axis count (the plan document,
  § Mount-neutral by design, rule 1; 036's US2-S6 guards `steering.rs`, which US6 and US8
  add to, and US7-S6 035's `stack/service.rs`, the file under `crates/finder/src/imaging/`
  that US7 edits most, each for thirteen names).
- Nothing is sized to one telescope or one sub length (rules 3 and 6): the readout's
  window is counted in subs. The times this spec fixes, the quarter-second poll and the
  2 s a tracker settles for, belong to the mount and the gap and not to a sub.
- The imaging path's words are mount-neutral (rule 5): `steering.rs`, the view and the
  page say "the tracker", never "the finder camera", an axis or field rotation. Why a
  tracker is not steady is the tracker's own word, passed through `Steered::pause` and
  the view; the page has a sentence for each word it knows and a general one for any
  other, so another mount's tracker can say `flipping` with no change to the imaging
  path. One sentence of the page names the aim point, a term of 032's that the same page
  already shows.
- The steering takes no exposure and holds no lock on the camera (the plan document,
  § Amendments, 1), and never keeps a tracker alive (§ Plan traps 15). The one camera
  setting this spec changes, a stack's exposure, is changed by the stack through the
  lease it already holds (§ The stack; § What 035 provides).
- Every rate is sent by the tracker's own task (§ Plan traps 1).
- `web/dist` is committed and must be the build of its source (US8).

**Scale/Scope**: one MCP tool and one page block; US6 adds to the tracker and the
steering, and US7 about 90 lines to 035's stack.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Nothing touches the finder camera's solver, detection or `crates/math-core`. Console work is grounded in the cited sources: 015's own take-up, backlash measure and keyhole pass for the times not steady, 038's push, and the simulator's truth. |
| II. Never weaken a gate | No existing test, golden, fixture, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS`, `ergane.yaml` or `playwright.config.ts` is edited, 036's tests included. New tests are new files; each finder module adds one `mod` line to its binary's `main.rs`, which holds no test (owner approval 9). No existing assertion changes: 035's tests read `dropped` key by key and never count its keys (035 spec.md § User Scenarios), and flip.md § Prerequisites 3 checks that in the landed tests. |
| III. Tests first, independent expectations | Oracles: the simulator's truth, read by each test; worked examples whose arithmetic is in spec.md; 015's own measurement, keyhole pass and bounds; 038's `track.push`; the rate log of the backlash rig. Every scenario names its oracle and tolerance. Time is virtual throughout. |
| IV. Critical paths | No `unsafe`, no `Cargo.toml` or `Cargo.lock` change, nothing under `proto/` or `crates/math-core`. |
| V. Proven open source first, honest licensing | The wait for a steady tracker, the drop of a sub taken across a time it was not and the readout are this project's own code, and principle V of constitution 1.3.0 asks a plan to name what it looked at and why it turned it down. PHD2 (BSD-3-Clause), the proven guider, which also tells an imaging program when its guiding has settled, was looked at and turned down, as 036's plan.md turns it down: it guides a mount through ASCOM, INDI or ST-4, and this mount is driven by the scope node, which has no INDI mount driver; and it expects a guide camera's steady stream, not imaging subs that may come minutes apart (history.md § Decisions and the alternatives rejected). No solver is touched and nothing is copied. |

## Project Structure

```text
crates/finder/
├── src/imaging/steering.rs              # US6: steady, check, the wait; US8: the readout (036's file)
├── src/imaging/stack/service.rs         # US7: the taker's and the worker's calls, Dropped::not_steady (035's file)
├── src/imaging/solving.rs               # US7: set_solver gives the steering its source (032's file)
├── src/mcp/tools.rs                     # US11: set_steering
├── src/tracking.rs                      # US6: steady, pause, pushed (015's file, as 036 left it)
├── src/session.rs                       # US6: tells the tracker of a push (038's track_live_pointing)
├── README.md                            # US7: the section; US8: the readout's keys; US11: the tool's lines
├── tests/console/main.rs                # US6, US8: one mod line each
├── tests/console/steering_steady.rs     # US6: new
├── tests/console/steering_readout.rs    # US8: new
├── tests/motion/main.rs                 # US6, US7, US10: one mod line each
├── tests/motion/steering_steady.rs      # US6: new
├── tests/motion/steering_unsteady.rs    # US10: new
├── tests/motion/steering_stack.rs       # US7: new
├── tests/mcp/main.rs                    # US11: one mod line
├── tests/mcp/steering.rs                # US11: new
└── web/
    ├── src/api.ts                       # US8: ImagingSteering, putImagingSteering, StackDropped.not_steady
    ├── src/components/SteeringBlock.tsx       # US8: new
    ├── src/components/SteeringBlock.test.tsx  # US8: new
    ├── src/components/StackBlock.tsx    # US8: the Dropped line's words for not_steady (035's file)
    ├── src/components/ImagingPage.tsx   # US8: places the block
    └── dist/                            # US8: rebuilt
DESIGN.md                                # US8: the block
.impeccable/design.json                  # US8: the component
```

## What 036 provides (read as landed on 2026-11-20)

`036-tracking-from-subs` has landed (#327 to #333, the last `fc161a54`). This section
lists what this spec's stories call, change or must leave passing of it, by name and
with the behaviour they rely on. The re-anchoring of 2026-11-20 read each point in 036's
code on `main` at `fc161a54` and corrected this section where it differed; the lines
cited are that code's (flip.md § Prerequisites 2).

1. **`Steered`** (`crates/finder/src/imaging/steering.rs:81`, 036's US2), the one interface
   between the stack and a tracker, with `TrackingError` (`time`, `east_arcsec`,
   `north_arcsec`), `Stepped` (`east_arcsec`, `north_arcsec`, `Default`) and
   `GapFuture`, which owns only a gap's answer and borrows nothing of the tracker:

   ```rust
   pub trait Steered: Send + Sync {
       fn target(&self) -> Option<(f64, f64)>;
       fn steer(&self, error: TrackingError);
       fn gap(&self) -> GapFuture;
       fn release(&self);
       fn steered_by_subs(&self) -> bool;
       fn longest_exposure_s(&self, _turn_rad: f64) -> Option<f64> { None }
       fn longest_exposure_ahead_s(&self, _turn_rad: f64) -> Option<f64> { None }
       fn steady(&self) -> bool { true }
       fn steady_between(&self, _start: f64, _end: f64) -> bool { true }
       fn pause(&self) -> Option<&'static str> { None }
       fn tune(&self, _step_share: f64, _rate_share: f64) {}
   }
   ```

   The five required methods are fixed. US6 gives the tracker bodies for `steady`,
   `steady_between` and `pause` and adds no method, so a scripted tracker of this spec
   implements the five and the provided ones its tests set. 036's US2-S6 reads the text
   of `steering.rs` for thirteen names (`ephemeris`, `alt_az`, `parallactic`,
   `counts_per_revolution`, `AxisInfo`, `MountClient`, `azimuth`, `altitude`,
   `field_rotation`, `hour_angle`, `meridian`, `backlash` and `keyhole`), so nothing US6
   or US8 writes there may name one, in code or in a comment.
2. **The tracker** (`crates/finder/src/tracking.rs`, 036's US3 and US5). `impl Steered for
   Tracker` (`crates/finder/src/tracking.rs:484`), each method calling the inherent one
   of the same name; `longest_exposure_s` and `longest_exposure_ahead_s`, the limit now
   and the shortest of the hour ahead; `tune`. Under the control lock, `Control`
   (`crates/finder/src/tracking.rs:109`) holds, of 036's:
   - `settling`: true from the start until the rate task has sent its first rates
     (`start_with_resets` builds it true; the rate task sets it false at the end of every
     turn, after its rates);
   - `fresh_after`: a tracking error steers only when its time is later than this; 038's
     `Tracker::follow` sets it to the time now, and ends a claim, clears a lapse and
     forgets what was pending or held, as `release` does;
   - `subs` (present while the subs steer), `pending` (an error not yet applied),
     `lapsed` (the claim lapsed for want of errors while a stack still holds the
     tracker), `held` (the step a lapsed claim holds for the next gap), `corrections`
     (what the last 16 gaps did) and `gaps` (the gaps waiting, each with its answer's
     sender);
   - `direction`, which the rate task stores before it sends its rates.

   `steer` keeps an error only while the state is `tracking`, its three numbers are
   finite and its time is later than `fresh_after`. The rate task serves gaps between two
   of its turns, in `serve_gaps` (`crates/finder/src/tracking.rs:975`): its step 1, under
   the lock, answers every waiting gap with no step and claims nothing while the state is
   not `tracking` or `settling` is true (`crates/finder/src/tracking.rs:986`); its steps
   1 and 2 set `proportional` and `proportional_until` to zero at every
   gap that claims or applies a held step; its step 4 sends both axes their rate, `base`, at
   every gap served under a claim or a lapsed claim but the one at which the claim
   lapses. `commanded(rate, direction, extra)` (`crates/finder/src/tracking.rs:1393`)
   turns a requested rate into the one sent, zero where it would turn the axis against
   `direction`. The turn's loop and `serve_gaps` send the azimuth axis its tracking
   rate; `take_up`, `stop_axis` and `keyhole` command it too, and US6's `sent` is
   written only where the turn's loop and `serve_gaps` send. `Shared::pointing`
   (`crates/finder/src/tracking.rs:512`) skips 015's block of the proportional rate and
   the offset while `subs` is `Some`, holds the step 015's proportional term would make in
   `held` while `lapsed` is true, and runs it whole otherwise. `end`
   (`crates/finder/src/tracking.rs:595`) clears the claim, `lapsed`, `held`, the
   corrections and the waiting gaps, answering each with no step.

   036's FR-011 lists the tracking errors the tracker ignores. Two of its clauses are
   this spec's to build, in US6: an error whose time is not later than the last time the
   tracker stopped being steady, and one handed over while the tracker is not steady.
   036's FR-007 has a tracker steered from a stack's first `gap` that comes while it is
   steady (FR-021).
3. **`Steering`** (`crates/finder/src/imaging/steering.rs:252`, 036's US4 and US5):
   `Steering::new(clock)`, `set_source`, `hold`, `before_exposure(ask) -> Go`,
   `accepted(frame, solve)`, `release`, `set_set_point`, `view` (which is
   `view_for(None)`), `view_for(ask)`, `set_smear_px`, `set_on`, `set_step_share` and
   `set_rate_share`; with `Source`, `ExposureAsk` (`wanted_s`, `shortest_s`, `sensor`,
   `roi`, `binning`), `Go` (`exposure_s`, `limited`), `Hold`, `EXPOSURES_KEPT` (32),
   `FAR_ARCSEC` (180.0), `HALVING_FLOOR_S` (1.0), `ErrorView` and `LimitView`. What this
   spec relies on:
   - **Lock order.** The state is behind a `std::sync::Mutex` that is a leaf: never held
     across an await, nor while the source closure, a tracker's method or `Imaging` is
     called. Each call copies what it needs under the lock, releases it, makes its calls
     and takes it again only to write what it learned.
   - **`before_exposure`** (`crates/finder/src/imaging/steering.rs:284`), in four
     steps: (1) mark the steering used, find the tracker with `source()` and keep it,
     weak, as the tracker last found; (2) with none, push an exposure record with no
     tracker and answer at once; (3) unless the steering stands back, and with `on` read
     true at this point, call `tracker.tune(step_share, rate_share)` and `tracker.gap()`,
     drop the `Arc`, await the answer and upgrade the tracker last found again, going on
     as step 2 when it has gone; (4) push `Taken { go_time, tracker, sensor }`, the oldest
     dropped past `EXPOSURES_KEPT`, and answer the exposure held: decided at a stack's
     first call by halving the wanted exposure until it is no longer than
     `longest_exposure_ahead_s(turn_for(&ask, set_point, smear_px))`, never to under
     `HALVING_FLOOR_S` or the camera's shortest, and held with `limited` until `release`.
     `turn_for(&ask, set_point, smear_px)`, a private function of `steering.rs`, is the
     smear over the distance, in delivered pixels, from the set point to the frame's
     farthest corner.
   - **The record of a frame** is the newest exposure record whose `go_time` is not after
     the frame's mid-exposure, `frame.start + frame.exposure_s/2.0`.
   - **Standing back.** While `far` is set (a stack whose first tracking error was over
     `FAR_ARCSEC`), `before_exposure` calls no `gap` and `accepted` hands nothing over,
     until the steering's own next `release`; the cap goes on.
   - **Letting the tracker go**: one private function, `let_go`
     (`crates/finder/src/imaging/steering.rs:409`), that, under the lock, copies the
     tracker last found, still a `Weak`, and sets `subs` to 0 and `last` to `None`, and
     once the lock is released upgrades it and calls that tracker's `release`. It keeps
     the tracker last found, the exposure records, the exposure held and `far`. 036
     calls it from a `set_set_point` that changes the set point and from
     `set_on(false)`; US6's wait for the limit is its third caller.
   - **`hold`** counts one more `Hold` alive; `Hold`'s drop counts one less and calls
     `release`. **`release`**, under the lock, takes the tracker last found, still a
     `Weak`, so that it is forgotten, empties the records, sets `subs` to 0, `last` and
     `far` to `None`, and forgets the exposure held and its ask; once the lock is
     released it upgrades that tracker and calls its `release`. It leaves the settings.
   - **The view.** `view_for` answers `None` until the steering is used, unless a
     tracker runs and an ask is given. `SteeringView` is `#[non_exhaustive]`, serialized
     with every `Option` skipped while `None`, and holds 036's keys `source`, `subs`,
     `last`, `far_arcsec`, `holding`, `smear_px`, `on`, `step_share`, `rate_share`,
     `exposure_s`, `limited`, `longest_exposure_s`, `over_limit` and `would_take_s`.
     `longest_exposure_s` is the running tracker's answer for now, asked as the view is
     made; `over_limit` is whether it is a number under the exposure held.
   - **The switch.** With `on` false the steering calls neither `gap`, `tune` nor
     `steer`, and still caps (036's FR-019 says it still pauses and drops from US6).
4. **`Session::steered()`** (`crates/finder/src/session.rs:918`, 036's US4): the running
   tracker as an `Arc<dyn Steered>`, `None` while Track is off, armed or ended. 037 asks
   it `steady()` and `steady_between` over a window that ends now.
5. **`Imaging::steering()`** and the view's part (`crates/finder/src/imaging/mod.rs:458`,
   036's US4 and US5): the console's `Arc<Steering>`, made in `Imaging::start`. Wherever
   `Imaging` builds the view it sets `steering` to `self.steering.view_for(ask)` after
   its own lock is released (`crates/finder/src/imaging/mod.rs:541`), `ask` being the
   `ExposureAsk` the console's camera and settings give (`wanted_s` and `shortest_s` from
   `settings.exposure_ms` and `camera.exposure_ms.min`, the sensor, the region and the
   binning), and `None` while the view has no `camera` or no `settings`.
6. **The route** (`crates/finder/src/imaging/routes.rs`, 036's US5):
   `PUT /api/imaging/steering`, with `SteeringEdit`
   (`crates/finder/src/imaging/routes.rs:45`; `on`, `smear_px`, `step_share`,
   `rate_share`, each optional, unknown keys refused and a JSON null an error, by a
   `Deserialize` of its own that also refuses anything but a JSON object), and
   `pub(crate) fn apply_steering(imaging: &Imaging, edit: SteeringEdit) ->
   Result<ImagingView, String>` (`crates/finder/src/imaging/routes.rs:81`), which checks
   the three numbers in that order, answers the first out of range with its setter's words
   (`the corner smear must be a number from 0.1 to 20`,
   `the step share must be a number from 0 to 1`,
   `the rate share must be a number from 0 to 1`) and changes nothing then, sets the rest
   with `on` last, and answers the view. Without imaging the route answers 409
   `this console takes no imaging frames`. The four settings are run-only: 6.0, true,
   0.5 and 0.3 at every start.
7. **The README** (`crates/finder/README.md`): 036 adds the route's row to the `## API`
   table (`crates/finder/README.md:243`). US7's section goes after 035's `## Live stack`.
8. **036's test files**, which no story here edits and every story leaves passing:
   `crates/finder/tests/console/steering_error.rs` (US2's), `steering_gap.rs` (US4's) and
   `steering_cap.rs` (US5's), and `crates/finder/tests/motion/steering.rs` (US3's),
   `steering_claim.rs` (US9's), `steering_gap.rs` (US4's) and `steering_cap.rs` (US12's,
   cut from US5 by tests). Their tracker rigs wait 3.5 s after a
   tracker's first rates before they ask for a gap, by when a tracker that settles for
   2 s (US6) has settled, and none of their scenarios has a time the tracker is not
   steady after that wait; no case of 036's US5 or US12 has a limit under half the
   exposure it answers (§ Plan traps 22 and 34). Their `mod` lines in `console/main.rs` and
   `motion/main.rs` are 036's, in alphabetical order, and this spec's go among them.

## Interfaces

Coordinates, sky offsets and the worked examples are as spec.md § User Scenarios defines
them. Everything 030 to 035 create is named as their plan.md gives it, and everything
036 creates as § What 036 provides gives it; all seven have landed, and both were
re-read in their code on `main` at `fc161a54` on 2026-11-20 (flip.md § Prerequisites 2
and 3). What this spec uses or changes of 035's is gathered in
§ What 035 provides.

### Steady, the tracker's second answer (`crates/finder/src/tracking.rs`, US6)

036's tracker (§ What 036 provides, 2) gains:

```rust
impl Tracker {
    pub fn steady(&self) -> bool;
    pub fn steady_between(&self, start: f64, end: f64) -> bool;
    pub fn pause(&self) -> Option<&'static str>;
    /// The session says whether the telescope is being moved by hand (038's `moving`).
    pub fn pushed(&self, moving: bool);
}
```

and `impl Steered for Tracker` gains `steady`, `steady_between` and `pause`, each calling
the inherent one; `pushed` is the tracker's own, for the session, and no part of
`Steered`.

**Steady** (US6). `Control` gains `steady: bool`;
`unsteady: VecDeque<(f64, Option<f64>)>`, the times, on the console's clock, the tracker
stopped and started being steady, the last 64;
`settle_until: Option<tokio::time::Instant>`; `pushed: bool`; `stalled: bool`; and
`sent: Option<(f64, f64)>`, the rate the task last sent the azimuth axis, in degrees a
second, with the console time it sent it. `settling` is 036's US3's: true from the start until
the first rates are sent. US6 also sets it true at the start of a take-up and of the
keyhole pass, and keeps it true for `SETTLE` after the rates that follow each of the
three:

```rust
/// How long the tube is given to settle after a tracker's first rates, a take-up and
/// the keyhole pass, before the tracker reads steady.
const SETTLE: Duration = Duration::from_secs(2);
```

Wherever `settling` is set true, `settle_until` is set to `None`. At the end of every
turn of the rate task, after its rates are sent, in place of 036's US3's `settling = false`:
when `settling` is true and `settle_until` is `None`,
`settle_until = Some(Instant::now() + SETTLE)`; when `settling` is true and
`Instant::now()` has reached `settle_until`, `settling = false` and
`settle_until = None`. So the tracker is steady again at the end of the first turn to
end at least 2 s after those rates: exactly 2 s after a start's first rates in paused
time, and for a take-up, whose turn runs 0.8 s of paused time, 3.0 s after its pulse
(probe round 2); between 2 s and 3 s after them on a real link. The settle
belongs to the mount, as the half-second pulse does, and not to a sub. Steady was
defined by commands acknowledged, not by a tube at rest: a take-up is a 0.5°/s move
stopped dead, the keyhole pass ends in a slew, and first rates start two motors from
rest, and a real tube rings for a second or more after each, where the simulated one
follows its drive exactly (reanalysis AA10, FF8).

A private `fn mark(&self, control: &mut Control)` computes

```rust
let now_steady = state == "tracking" && !settling && !pushed && !stalled
    && measurement[0].is_none() && measurement[1].is_none();
```

and when it differs from `control.steady`: stores it; on becoming not steady pushes
`(clock.now(), None)`, dropping the oldest past 64, and sets `subs = None`,
`pending = None`, `held = Held::default()` and `fresh_after = clock.now()`; on becoming
steady sets the last entry's end to `clock.now()`. `lapsed` is left as it is, so a claim
dropped by a take-up is taken back at the next gap, and one that had already lapsed
stays lapsed. `fresh_after` does what `pending = None` cannot: 035's worker runs a sub
or more behind, so an error measured before this time can be handed over after it, and
with a lapsed claim `steer` would keep it for the gap that takes the tracker back,
minutes of 015's own corrections later (reanalysis AA4). It is set on becoming not
steady and never on becoming steady: 036's US3-S9 and 036's US3-S10 hand over errors dated before
the tracker's first rates, and 036's US3's file passes unedited after US6. For the same reason
US6 makes `steer` do nothing while `control.steady` is false. `start_with_resets` calls
nothing: the `Control` it builds has `steady: false` beside 036's US3's `settling: true`, and
its list holds `(f64::NEG_INFINITY, None)`, the tracker having been not steady at every
time before it started. `mark` is called:
- at the start of `take_up` and of `keyhole` (`crates/finder/src/tracking.rs:1168`,
  `crates/finder/src/tracking.rs:834`), after setting `settling = true`;
- where the rate task opens a measurement (`crates/finder/src/tracking.rs:1326`: on the
  simulated link the end-of-turn `mark` comes at the same instant; this one matters
  where the link's round trips take time) and where `settle` closes one
  (`crates/finder/src/tracking.rs:955`);
- at the end of every turn of the rate task, after its rates are sent and the settling
  rule above;
- in `pushed` and in `follow` (**A push by hand**, below);
- where a turn finds an axis stopped (**A stopped axis**, below);
- in `end`, where the state is `off`.

**A push by hand** (US6). `pub fn pushed(&self, moving: bool)`: under the control lock,
nothing when the state is `off`; otherwise `control.pushed = moving` and `mark`.
`follow` sets `pushed = false` and calls `mark`, after 036's US3's lines, and `end` sets it
false. The session calls `pushed` (§ The session's push). 038 keeps a push in the session and
hands the tracker nothing while it is `moving`, so without `pushed` the tracker
would read steady with a hand on the tube: exposures would go on, every sub taken across
the push would be kept, and a sub taken after the hand had left and before 038's five
still seconds were up would be accepted with the whole push as its error, which the next
gaps would step back toward the old place at 30″ a gap. With gaps under about 2.5 s apart those steps could
keep the push from ever settling (reanalysis AA2, BB3, CC2, DD3, FF1; read from the
code, not run). Not steady, the claim and the error not yet applied are dropped as for a
take-up, `before_exposure` waits and `check` drops the sub in flight; that the stack
waits while a push is moving is also what 037's run takes for granted. One finder solve
more than 5′ wrong sets 038's `moving` for one pointing, and so costs one sub and the
claim; the offset is kept.

**A stopped axis** (US6). The scope node stops both axes when a rate command fails on
the mount's side, and still acknowledges the command, so the console is not told
(reanalysis FF3; its own handling is another spec's). The rate task already reads the
azimuth axis's state at the top of every turn. Where it sends the azimuth axis its
tracking rate, in the turn's loop and in `serve_gaps`, it stores that rate and
`clock.now()` in `control.sent`; `take_up`, `stop_axis` and `keyhole` command the axis
too, and write nothing there. When a turn's read answers `moving` false while `sent`
holds a rate that is not zero, the turn sets `stalled = true`, calls `mark`, and sets
the start of the open entry to the earlier of its own start and `sent`'s time (a tracker
already not steady has an open entry that began earlier, and `mark` pushes none). At the
end of that turn, after its rates are sent, it sets `stalled = false`, before the
settling rule and `mark`. So the tracker reads not steady from the last azimuth rate it
sent to the rates that start the axes again, about a second: the sub in flight is
dropped, the claim is taken back at the next gap, and no drift rate is measured across
the stop. It does not settle afterwards: the owner's answer is that turn alone. A state
read the node could not make answers the last counts with `moving` true, so it raises no
false alarm, and an axis that was sent zero is not looked at.

**Why it is not steady** (US6). `Control` gains `cause: &'static str`, written wherever
`settling` is set true: `"starting"` in `start_with_resets`, `"reversing"` at the start
of `take_up` while the state is `tracking`, and `"overhead"` at the start of `keyhole`:
the take-up that ends the keyhole pass is the pass's own, and keeps `"overhead"`.
`pub fn pause(&self) -> Option<&'static str>`, under the control lock: `None` when
`control.steady` is true or the state is `off`; otherwise `Some("pushed")` when
`pushed`; else `Some("measuring")` when either axis's measurement is open; else
`Some(cause)` when `settling`; else `Some("stalled")` when `stalled`; else `None`. So
the 2 s of settling keep the word of what they follow, and a push, which needs the
observer's hand off the tube, is named before anything it coincides with. `stalled`
lasts for the one turn that found an axis stopped, under the steering's poll, so the
page seldom shows it, and no scenario of the tracker reads it: in paused time that turn
takes no time. The six words are the tracker's own, and the only place this spec writes
them is here and in the page's sentences (§ The page): `steering.rs` and the view pass
the word through unread, so the imaging path names no axis and no gear, and another
mount's tracker may say a word of its own (reanalysis FF16, FF20).

`Control::default()`, which a tracker never started holds, has an empty list and
`steady: false`. `steady()` is `control.steady`. `steady_between(start, end)` is false
when the list is empty, or when any entry `(from, to)` has `from <= end` and
`to.is_none_or(|to| to > start)`; after tracking ends the last entry is open, so every
window that reaches past the end is false and one wholly before it is judged on the
entries alone. A time not steady runs from the instant it began up to, and not
including, the instant the tracker was steady again: a window that begins at that
instant does not touch it, as `steady()` read at that instant is true. With both bounds
inclusive a stack started at the instant of a tracker's first rates would lose its
first sub (D38). With 2 s of settling the stack's own quarter-second poll can
fall on the instant the settling ends, whichever instant the stack began at, so the end
is not counted and the stacking rig of spec.md US7 needs no second more. US6
changes the test in `serve_gaps` step 1 from `settling` to `!control.steady`, which is
the same before the first rates and also turns away a gap served while a measurement is
open, while the tracker settles, during a push and in a turn that found an axis stopped:
it answers no step and claims nothing. 036's US3's scenarios pass under either test, because
the tracker rig waits 3.5 s after the first rates, by when the tracker has settled, and
none of them opens a measurement (§ Plan traps 22).

US6 also changes the held mode's test in `Shared::pointing` from `lapsed` to
`lapsed && control.steady`. While the tracker is not steady its gaps are turned away, so
a step held then would wait as long as the tracker stays not steady; and 015 closes a
first reversal's measurement from its own corrections, so held steps would keep it open
for good (§ Plan traps 28). Not steady, a pointing therefore runs 015's block whole even
after a lapse; `mark` has forgotten what was held as the tracker stopped being steady,
and once it is steady again the next pointing zeroes `proportional` before it holds
anything, and the next gap zeroes it too (step 2) and sends `base`, as every gap served
under a lapsed claim does (step 4). No exposure is started while the tracker is not
steady, and the sub it crossed is dropped (US6-S4). 036's US3's scenarios pass under either
test, because none of them has a time the tracker is not steady once its rig's wait is
over.

### The steering's wait and its check (`crates/finder/src/imaging/steering.rs`, US6; US8 adds the readout)

036's `Steering` (§ What 036 provides, 3) gains:

```rust
impl Steering {
    // US6:
    /// For every sub, before it is solved: whether the tracker was steady for it.
    pub fn check(&self, frame: &SolveFrame) -> Verdict;
    /// Whether a `before_exposure` is waiting, for the tracker to be steady or for its
    /// limit to rise. 037 reads it.
    pub fn waiting(&self) -> bool;
}

// US6:
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict { Keep, Drop(&'static str) }
/// The reason a sub taken while the tracker was not steady is dropped with.
pub const NOT_STEADY: &str = "not_steady";

/// 036's `steering` part of the imaging view gains, after its own keys:
pub struct SteeringView {
    // US6:
    pub steady: bool,
    pub waiting: bool,
    /// While `waiting`: the tracker's word for why it is not steady, or `limit`.
    pub pause: Option<&'static str>,
    /// One stack's drops: 0 again when the next stack takes its `Hold`.
    pub dropped_not_steady: u64,
    // US8:
    pub error_rms_arcsec: Option<f64>, pub error_rms_px: Option<f64>,
    pub drift_arcsec: Option<f64>, pub drift_px: Option<f64>,
}
```

and 036's exposure record, `Taken`, gains `stepped: Stepped` in US8: the sum of what the
gaps served in its `before_exposure` answered. Each story adds only the fields and items
named for it; a field written and never read fails `clippy -D warnings`. The state behind
036's lock gains, by story, `steady`, `waiting`, the pause's word and the count (US6), and
the window (US8). `check` keeps 036's **Lock order**.

US6 puts a wait before 036's `before_exposure` step 3 and a check after it (**Steady**,
below), and US8 keeps in each record the sum of what the gaps served in its call
answered. US6 also adds to three of 036's calls: `hold` sets `dropped_not_steady` to 0, a
stack having begun; `release` sets `steady` true and `waiting` false and forgets the
pause's word, and leaves `dropped_not_steady` as it is; and while the switch is off, or
the steering stands back, the wait and `check` go on as they do with it on. US8 makes
`release`, and 036's letting the tracker go, empty the window.

**Steady** (US6). `POLL` is 250 ms of tokio time. The state's `steady` is true and its
`waiting` false when the steering is built, and they are that in the view until a wait
changes them. In `before_exposure`, with a tracker: while `!tracker.steady()`, set
`steady` false, `waiting` true and `pause` to `tracker.pause()` in the state, sleep
`POLL`, and read `source()` again (a tracker that ended or was replaced is taken as
found: none sets `steady` true and `waiting` false, forgets the pause's word and answers
as step 2). Then the limit: whenever an exposure is held and
`tracker.longest_exposure_s(turn_for(&ask, set_point, smear_px))` is a number under half
of it, the call goes no further. On the first such round it lets the tracker go (**Letting
the tracker go**); it sets `waiting` true and `pause` to `Some("limit")`, leaving `steady`
as the tracker has it, sleeps `POLL` and begins again from the look at `source()`. It
looks before it asks for a gap and, at a stack's first call, once more when the cap has
decided the exposure, before it answers. A stack never stacks subs smeared by more than
twice the smear chosen, and goes on by itself once the limit has risen, which after a
transit it does; meanwhile the tracker's own corrections steer, as they do with the switch
off (036's owner decision 2; reanalysis FF5). A limit equal to half the exposure does not
wait, and 036's US5's and US12's scenarios hold no case that would (036's spec.md US5-S1).
Then the gap, where step 3 calls one. While its answer is awaited the steering goes on
looking: every `POLL` it upgrades the tracker last found, reads `steady()` and lets the
`Arc` go again, and it sets `steady` false, `waiting` true and `pause` to
`tracker.pause()` while that reads false, and back, with no word, when it reads true. A
gap asked for just as a keyhole pass begins is held by the tracker for the whole pass,
and without this the view said `steady` true and `waiting` false for all of it, to the
page and to 037 (reanalysis FF11). If `!tracker.steady()` after the gap, go round again:
that gap does not count. Then `steady` true, `waiting` false, no pause's word, the
record and the answer. `pause` is in the view only while `waiting` is true: the
tracker's word (starting, reversing, measuring, overhead, pushed or stalled on this
mount), `limit`, or nothing where the tracker gives none, and the page has a sentence
for each (§ The page). A `before_exposure` whose future is dropped while it waits (the
stack stopped) leaves `waiting` true in the state; `release`, which the stack's `Hold`
calls as it drops, sets `steady` true and `waiting` false and forgets the pause's word.
**`check`**: the record of the frame; with none, or one with no tracker, `Keep`. Its
tracker, upgraded; when it no longer exists, or when
`!tracker.steady_between(frame.start, frame.start + frame.exposure_s)`:
`dropped_not_steady += 1` and `Drop(NOT_STEADY)`. Otherwise `Keep`. It asks the tracker
that was running when the exposure began, not the one running now, so a sub fully
exposed before Track was turned off is kept and one that crossed the end is not.
`dropped_not_steady` counts one stack's drops: `hold` sets it to 0 and `release` leaves
it, so that after a stack has ended it still agrees with 035's own `dropped.not_steady`,
which stays in that stack's view too. Counted since the console started, it would
disagree with the count shown beside it from a console's second stack on, and cleared
in `release`, after every stop (reanalysis FF18).

**The readout** (US8). The state keeps the last 10 errors handed over since the last
release, each with its solve's `arcsec_per_px` and the drift worked out when it was
handed over (below). With n
errors, n at least 2: `error_rms_arcsec = sqrt(sum(east² + north²)/n)`; for each error k
after the window's first, `drift_k = hypot(east_k - east_{k-1} - step_east, north_k -
north_{k-1} - step_north)·exposure_k/(time_k - time_{k-1})`, the step being the sum of
the `stepped` of the records whose `go_time` lies after `time_{k-1}` and not after
`time_k`; `drift_arcsec = sqrt(sum(drift_k²)/(n - 1))`; and the two `_px` keys are those
divided by the newest error's `arcsec_per_px`. With fewer than 2, all four are `None`.
Each drift is worked out when its error is handed over and kept with it: worked out when
a view is made, the records of the gaps between two errors may be gone, since 32 are
kept and with one sub in five accepted ten errors span fifty exposures.
US8 adds the four keys to the README's list of the `steering` part's keys: the root mean
square of the last ten tracking errors and of the drifts between them, in arcseconds and
pixels, once two errors have been handed over since the tracker was last let go.

### The session's push (`crates/finder/src/session.rs`, US6)

**The push** (US6). In 038's `track_live_pointing` the session tells the tracker it read
in its step 1, with its own lock released: `tracker.pushed(true)` wherever it makes the
run's push `Push::Moving` (its steps 4 and 6, and its step 7 where a still stretch
starts again, where the call changes nothing), and `tracker.pushed(false)` wherever a
`Moving` push ends with no follow, before it hands that pointing over (its steps 2 and
5). `Tracker::follow`, which step 7 calls, clears the flag itself, and so does the end
of tracking, which `stop_tracking` brings. Nothing else of the session changes, and
`TrackView` gains nothing: 038's `track.push` already says `moving`.

### The stack (`crates/finder/src/imaging/stack/service.rs`, US7)

035's stack runs two tasks (035 plan.md § The stack on the console): the taker,
`Stacker::take_subs`, which calls `lease.take("sub")` back to back on the camera's
lease, which it owns and which drops when it returns, and the worker, `Worker::run`,
which takes each saved sub off the queue and awaits `Worker::process` for it. US7 adds
to both and to `Dropped`, and nothing else to the file. The places below are named as
035's `stack/service.rs` has them on `main` since its US8 landed; the re-anchoring of
2026-11-20 read each on `main` at `fc161a54` and gave it its line in § What 035
provides (flip.md § Prerequisites 3).

**`Dropped`** gains a twelfth field, last, `pub not_steady: u32`; `total()` adds it; and
`count`'s match gains the arm `"not_steady" => &mut self.not_steady`. That match names
every outcome the worker answers and ends in `unreachable!`, so an outcome without an
arm panics the worker's task while it holds the counts' lock, and the stack never ends.

**The stopping mark** is 035's (035 plan.md § The stack on the console, **The stopping
mark**): one `tokio::sync::watch` channel of `bool`, made with the run, `false` until
the stack is ending and `true` from then on; the taker holds a receiver of it,
`stopping` below, as a `mut` binding: `wait_for` takes `&mut self`, so US7 makes the
taker's parameter `mut stopping`, the one change to its signature. "The stack is
stopping" is `*stopping.borrow()`, and the taker's
wait races `stopping.wait_for(|stopping| *stopping)` in a `tokio::select!` that is
`biased` toward it, as 035's seam for 036 says (035 plan.md § Shapes that existing
tests pin, **The seam for 036 and 037**, 1). `wait_for` also completes, with an error,
if the sender has gone; either way the taker returns.

**The taker.** Where `take_subs` begins:

```rust
let steering = self.imaging.steering();
let _hold = steering.hold();          // dropped when the taker returns: the tracker is let go
let view = self.imaging.view();
// `start` refused a camera that was not ready, so both parts are there. If the camera
// has stopped being ready since, there is no ask: the steering is not asked, and the
// first `lease.take` below is refused with 031's words and ends the stack as 035 has it.
let ask = match (&view.camera, &view.settings) {
    (Some(camera), Some(settings)) => Some((settings.exposure_ms, ExposureAsk {
        wanted_s: settings.exposure_ms / 1000.0,
        shortest_s: camera.exposure_ms.min / 1000.0,
        sensor: (camera.width, camera.height),
        roi: Region::from(settings.roi),
        binning: settings.binning,
    })),
    _ => None,
};
let mut first = true;
// The observer's exposure, in milliseconds, once this stack has set a shorter one.
let mut restore_ms: Option<f64> = None;
```

035's loop then runs inside one `async` block, awaited in place, so that each of its
`return`s leaves the block and not the task (below, **The taker's return**). At the top
of the loop, after "if the stack is stopping: return" and before `lease.take("sub")`,
where 035's code says "036 can wait here under this lease":

```rust
if let Some((observer_ms, ask)) = ask {
    let go = tokio::select! {
        biased;
        _ = stopping.wait_for(|stopping| *stopping) => return,
        go = steering.before_exposure(ask) => go,
    };
    if first {
        first = false;
        let wanted_ms = go.exposure_s * 1000.0;
        if (wanted_ms - observer_ms).abs() > 0.0005 {
            let edit = SettingsEdit { exposure_ms: Some(wanted_ms), ..Default::default() };
            match lease.set(edit).await {
                Ok(_) => restore_ms = Some(observer_ms),
                Err(refusal) => {
                    // end the stack: camera, error = format!(
                    //     "the exposure could not be set to {wanted_ms} ms: {}", refusal.message)
                    return;
                }
            }
        }
    }
}
```

Nothing panics: the taker never unwraps a part of the view. `before_exposure` may wait
as long as the tracker is not steady; the stopping mark ends that wait by dropping the
future, and the `Ref` that `wait_for` answers is dropped with the arm, before anything
else is awaited. `{wanted_ms}` is the `f64` written with `{}`, so 30000.0 reads
`30000`.

**Why through the lease.** The stack is the camera's holder. While its lease is out,
`Imaging::set` by anyone else is refused with the stack's name (035 plan.md § The
camera's lease, 2), and 034's `Calibration::restore_exposure`, which an earlier draft of
this trio used, was such a caller: it would have answered false and the subs would have
been taken at the observer's exposure; 034 has since removed it (034 A1). `Lease::set` is
the holder's own `set`. With only `exposure_ms` present it stores the console's
exposure and calls nothing on the scope node (031 plan.md § The console's imaging
state, **`set`**), and of 031's refusals only one can reach it here: no frame is in
flight between two takes, and the exposure answered is never outside the camera's
range, because the halving stops at `shortest_s` and `wanted_s` was the setting
itself. So a refusal means the camera is no longer `ready`, and the stack ends as it
does for a refused take: the reason `camera`, the error naming the exposure, nothing
taken at the observer's longer exposure (spec US7-S5).

**The taker's return.** The taker owns the run's `Lease`, which drops when the taker's
task ends (035 plan.md § The stack on the console, **The taker**), so there is no
moment after the taker has returned at which the lease is still held. 035 puts the
observer's exposure back "where the taker returns and before the lease is dropped"
(035 plan.md § Shapes that existing tests pin, **The seam for 036 and 037**, 2). So,
right after the loop's `async` block has been awaited, and before the task ends:

```rust
if let Some(ms) = restore_ms.take() {
    let edit = SettingsEdit { exposure_ms: Some(ms), ..Default::default() };
    let _ = lease.set(edit).await;   // refused only by a camera that is no longer ready
}
// the task ends here, and `lease` and `_hold` drop with it
```

A refusal is ignored: a camera that is no longer `ready` refuses, and the setting stays
at the exposure the stack used. The change goes through the lease and before the lease
drops, so that nobody else's `set` comes between and whoever finds the camera free
finds the setting as it will stay. No frame is in flight then: the taker returns only at
the top of its loop, from the steering's wait, on a refused `take` or `set`, on a take
answered with no frame, or after `imaging.ended` has answered for its last frame. Every
way a stack ends passes that point, because 035's ending waits for the taker to return.
US7 adds nothing to 035's ending itself.

**The worker.** `Worker::run` awaits `self.process(&sub, &mut last)`, a
`Result<Accepted, &'static str>`, and counts an `Err(reason)` with
`view.dropped.count(reason)`; 035's own settings drop is `return Err("settings")` from
`process`. US7 adds two calls:

- in `process`, after the `SolveFrame` is built and before the settings test: `if let
  Verdict::Drop(reason) = self.stacker.imaging.steering().check(&frame) { return
  Err(reason); }`. It comes before the solve so that a sub smeared by a take-up costs no
  solve and leaves the stack's own hint alone, and before `*last = Some(…)`, so that
  `LastView` is left as the settings drop leaves it;
- in `run`, in the `if let Ok(accepted) = result` block, right after `note_solve` and
  before the picture is made, with no stack lock held:
  `self.stacker.imaging.steering().accepted(&accepted.frame, &accepted.solve);`. Inside
  the counts' block it would call the tracker under 035's counts lock, and 035 holds
  neither stack lock across a call out; after `make_picture` every tracking error would
  wait for a render, with the queue empty that is every sub, and an error that should
  arrive one gap late would arrive two late.

`run` writes the outcome it counted into the sub's record, so `stack.json`'s `subs`
lists a sub dropped here with the outcome `not_steady` with no further change;
`LastView` does not carry it (reanalysis BB1, CC10, DD4, EE3; BB2 for the place of
`accepted`).

**The guard** (spec US7-S6). The file, as US7 leaves it, holds none of the thirteen
names 036's US2-S6 reads `steering.rs` for: `ephemeris`, `alt_az`, `parallactic`,
`counts_per_revolution`, `AxisInfo`, `MountClient`, `azimuth`, `altitude`,
`field_rotation`, `hour_angle`, `meridian`, `backlash` and `keyhole`. Nothing US7 adds names one: the taker and the
worker reach the tracker only through `Steering`, `ExposureAsk` and `Verdict`, in sky
and sensor terms. flip.md § Prerequisites 3 checks that 035's landed file holds none either, so
that the guard rests on US7's diff.

### What 035 provides (035's files; read as landed on 2026-11-20)

035's eleven stories have landed, on 031's camera lease. This section lists what this
spec's stories rely on of 035, and through it of 031's lease, so that the re-anchoring has
one list to check (flip.md § Prerequisites 3). 036's re-anchoring read the same points
in 035's code at `52785403` and then took them out of 036's plan.md; this spec's
re-anchoring read each again on `main` at `fc161a54`, where 035's `stack/service.rs`
and `StackBlock.tsx` are as 035 left them, and the lines below are that code's. 035
says that "a sub taken at an exposure that changes within a stack is not offered" (035
plan.md § Prerequisites 10); this spec changes the setting once, before a stack's first
sub.

1. **The taker's loop**, `Stacker::take_subs`
   (`crates/finder/src/imaging/stack/service.rs:769`), has one place before each
   `lease.take("sub")`, after its look at the stopping mark, where 035's comment says
   036 can wait (`crates/finder/src/imaging/stack/service.rs:775`). The taker reaches the
   console's imaging state as `self.imaging`, from which `self.imaging.steering()` comes;
   the worker reaches it as `self.stacker.imaging`.
2. **The stopping mark** (035 plan.md § The stack on the console, **The stopping
   mark**): one `tokio::sync::watch` channel of `bool`, made with the run, `false` until
   the stack is ending and `true` from then on, set once by whoever ends the stack
   (`crates/finder/src/imaging/stack/service.rs:488`, set in `end` at
   `crates/finder/src/imaging/stack/service.rs:765`); the
   taker and the worker each hold a receiver, the taker's by value (US7 makes it `mut`).
   The taker's wait races it as `stopping.wait_for(|stopping| *stopping)` in a
   `tokio::select!` that is `biased` toward it, as the worker's own wait does
   (`crates/finder/src/imaging/stack/service.rs:972`), and returns when it wins (035 plan.md
   § Shapes that existing tests pin, **The seam for 036 and 037**, 1).
3. **The run's `Lease`**, which 035's `start` moves into the taker
   (`crates/finder/src/imaging/stack/service.rs:508`) and which drops when
   the taker's task ends, with `Lease::take(purpose)` as 035 uses it and
   `Lease::set(SettingsEdit)` (`crates/finder/src/imaging/mod.rs:909`,
   `crates/finder/src/imaging/mod.rs:901`). Of `set`, whose checks are 031's
   `Imaging::set_as` (`crates/finder/src/imaging/mod.rs:556`): with only `exposure_ms`
   present it stores the console's exposure and calls nothing on the scope node; it
   answers 409 with 031's words when the camera is not `ready`, 409
   `an exposure is in progress` while a frame is in flight, and 400 outside the camera's
   range.
4. **While the lease is out**, `Imaging::set` by anyone else is refused with the
   holder's name, `PUT /api/imaging/settings` included (035 plan.md § The camera's
   lease, 2; `held_by`, `crates/finder/src/imaging/mod.rs:935`, asked at
   `crates/finder/src/imaging/mod.rs:583`). Spec.md's edge case on editing the exposure
   during a stack rests on it.
5. **The taker's return.** 035's ending sets the stopping mark, awaits
   `imaging.abort()` when `stop()` asked (`crates/finder/src/imaging/stack/service.rs:591`),
   and waits for the taker to return, on every way a stack ends
   (`crates/finder/src/imaging/stack/service.rs:529`); the taker returns only at the top
   of its loop, from the gap's wait, on a refused take, on a take answered with no frame,
   or after `imaging.ended` has answered for its last frame, so no frame is in flight as
   it returns, and the lease drops with it. 035 puts the observer's exposure back "where
   the taker returns and before the lease is dropped" (the seam, 2), which is where § The
   stack does it.
6. **The worker**: `Worker::process` (`crates/finder/src/imaging/stack/service.rs:1043`)
   builds the sub's `SolveFrame` (`crates/finder/src/imaging/stack/service.rs:1058`) and
   then makes the settings test, which answers `Err("settings")` for a sub whose
   `exposure_ms` differs from the reference sub's by more than 1 ms
   (`crates/finder/src/imaging/stack/service.rs:1085`), before it sets `last`; it answers
   `Ok(Accepted { frame, solve, .. })` for a sub it accepts. `Worker::run` counts the
   outcome (`crates/finder/src/imaging/stack/service.rs:1020`), calls `note_solve` for an
   accepted sub with no stack lock held (`crates/finder/src/imaging/stack/service.rs:1034`),
   and then may make a picture.
7. **`Dropped`** (`crates/finder/src/imaging/stack/service.rs:83`) derives `Default`,
   writes every key always and in order, and has `total()` and `count(outcome)`, whose
   match names every outcome the worker answers and is `unreachable!` for any other
   (`crates/finder/src/imaging/stack/service.rs:123`). `stack.json`'s `subs` writes the
   outcome a sub was counted under, and `LastView::outcome` that of a sub that reached the
   solve. No test of 035 counts the keys, compares a whole `dropped` object with a
   literal, a fixed list of keys, a golden file or a schema that lists its keys (a
   comparison of two serializations of the same `Dropped`, as `stack_console.rs` makes
   between `stack.json` and the view, is not one), or builds `Dropped { .. }` without
   `..Default::default()` (035 spec.md § User Scenarios: "a test never asserts how many
   there are").
8. **`StackBlock.tsx`** builds its `Dropped {sum}: {parts}` line from the counts above
   0, in `StackDropped`'s order, each with its words
   (`crates/finder/web/src/components/StackBlock.tsx:62`,
   `crates/finder/web/src/components/StackBlock.tsx:78`).
9. **`Stacker::start`** refuses a camera that is not `ready`
   (`crates/finder/src/imaging/stack/service.rs:394`), so the taker's first read
   of the view finds `camera` and `settings`.

What this spec changes in 035's two files, and nothing else:

| File | Change | Story |
|---|---|---|
| `crates/finder/src/imaging/stack/service.rs` | `Dropped` gains `not_steady`, last, `total()` adds it and `count` gains its arm | US7 |
| `crates/finder/src/imaging/stack/service.rs` | the taker keeps a `Hold`, awaits `before_exposure` before each take raced against the stopping mark, and before its first take sets the exposure through the lease, ending the stack `camera` on a refusal | US7 |
| `crates/finder/src/imaging/stack/service.rs` | the taker's loop runs in one `async` block, and after it the taker puts the observer's exposure back through its lease before the lease drops with the task | US7 |
| `crates/finder/src/imaging/stack/service.rs` | the worker's `process` asks `check` before the settings test and answers its drop as `Err`, and `run` calls `accepted` right after `note_solve` | US7 |
| `crates/finder/web/src/components/StackBlock.tsx` | the `Dropped` line gains `{count} while the mount was not steady`, last | US8 |

### `set_solver` (`crates/finder/src/imaging/solving.rs`, US7)

032's `Imaging::set_solver(solver, session, live)` gains one statement:
`self.steering().set_source(Arc::new(move || session.steered()))`, on a clone of the
session. So `AppState::with_imaging_solver`, the binary and `--simulate` need no change
(032 plan.md § The builders).

### `crates/finder/README.md` (US7)

A section `## Tracking steered by the subs`, after 035's `## Live stack`, saying, on a
mount whose field turns as it tracks:

- **What steers.** That while a stack runs with Track on, each sub the stack accepts
  gives a tracking error at the set point, and that it, not the finder camera, corrects
  the tracker; that corrections are made only between exposures, a step of half the
  error when it is two counts or more and 0.3 of the drift rate out of the rate; that
  the two shares are the settings `step_share` and `rate_share`, for tuning in the
  field; that an error arrives a sub or more late and is judged against the corrections
  made since; that one gap changes a rate by 0.1″/s at most, and a pair of subs with a
  large step between them by nothing; that an axis is never reversed by a correction, so
  a step against an axis's motion is at most that axis's own rate times half a second a
  gap, and an error that way leaves slowly; and that the finder camera still feeds the
  pointing model.
- **When the tracker's own corrections steer again.** The stack stopped; the switch
  turned off, at once, during a stack too; eight gaps in a row with no accepted sub; the
  tracker not steady; a stack that began more than 3′ from its set point. That after
  eight such gaps they go on steering until a sub is accepted, applied only in the
  stack's gaps and only as steps, the rate staying as the subs left it, so that no rate
  changes during an exposure; that a gap with no accepted sub counts toward those eight
  whatever the reason, a sub dropped as `behind`, `settings`, `frame_failed`,
  `unreadable` or `solver_failed` or one still being solved, so that a console that
  needs more than eight sub lengths for one sub takes no drift out of the rate; that
  turning the switch off lets the finder camera correct as before, during exposures too;
  and that a stack whose first tracking error is over 3′ is not steered at all, since
  the finder camera's aim point and the sensor's centre then disagree by more than
  Track's own push threshold, and what to do about it (centre the target, or turn Track
  off and on to apply the measured aim point).
- **The cap.** The corner smear, 6 px unless set; that the exposure is decided before a
  stack's first sub from the shortest limit of the hour ahead, by halving the setting,
  never to under 1 s; why it is then held (a stack's subs share one exposure, and a
  master dark matches one); that the stack sets the exposure setting to the exposure in
  use, that the setting cannot be edited while the stack holds the camera and is put
  back as the stack ends, and that a stack whose exposure cannot be set ends with
  `camera`; what `over_limit` means; that under half the exposure held the stack waits
  for the limit to rise; and that `would_take_s` says what a stack started now would
  hold, which a stack started while the limit was low may be well under.
- **The pause.** When the tracker is not steady: before its first rates, during a
  take-up, a first reversal's backlash measurement and the keyhole pass, for 2 s of
  settling after the first rates, a take-up and the keyhole pass, while the telescope is
  moved by hand and until Track follows the push or it comes back, for the second in
  which the tracker finds an axis stopped, and after tracking ends; the `not_steady`
  drop; and that the scope node stops both axes when a rate command fails and logs
  `mount command failed`, which the console is not told of and the tracker can only
  notice a second later.
- **The `steering` part's keys**, one line a key, with these meanings:
  - `source`: `subs` while tracking errors steer; `tracker` while the tracker's own
    corrections do (no stack, the switch off, a stack begun too far out, or no sub
    accepted in the last eight gaps); `none` while nothing tracks;
  - `subs`: tracking errors used since the tracker was last let go; `last`: the newest
    one, its `time` in Unix seconds at the sub's mid-exposure;
  - `holding`: a stack holds the steering. With `source` `tracker`, `on` true and no
    `far_arcsec` it means no sub was accepted lately;
  - `far_arcsec`: present while a stack is not steered because it began this far from
    its set point;
  - `smear_px`, `on`, `step_share`, `rate_share`: the four settings, all run-only;
  - `exposure_s`, `limited`: the exposure the stack holds, and whether it was shortened
    when the stack began;
  - `longest_exposure_s`: the longest exposure that keeps corner stars within the smear
    now, or `no limit`; absent while nothing tracks. `over_limit`: it is under the
    exposure held;
  - `would_take_s`: the exposure a stack started now would hold;
  - `steady`, `waiting`, `pause`: `waiting` is true while the stack waits before an
    exposure, `steady` false when it waits for the tracker, and `pause` says why in one
    word: `starting`, `reversing`, `measuring`, `overhead`, `pushed`, `stalled`, or
    `limit` when it waits for the longest exposure to rise;
  - `dropped_not_steady`: this stack's subs dropped because the tracker was not steady
    during them, the same count as `stack.dropped.not_steady`.
- **Why subs are dropped.** One paragraph: `stack.dropped` counts a stack's dropped subs
  by reason, 035's reasons and this spec's `not_steady`, for a sub taken across a pause
  or a time the tracker was not steady.
- That `Drift per sub` includes the scatter between two solves, so it does not read 0 on
  a real sky; that the four settings are run-only; and that 015's "guiding from the
  imaging camera" is no longer out of scope.

In 035's `## Live stack`, the sentence `The eleven dropped-sub reasons are:` becomes
`The twelve dropped-sub reasons are:`, and its list gains, last:
`` - `not_steady`: the tracker was not steady during the sub, which was dropped before its solve (040). ``

US8 adds the readout's four keys to that list of the `steering` part's keys (§ The
steering's wait and its check, **The readout**), and US11 the tool's lines to the README's list of tools
(§ The MCP tool).

### The page (`crates/finder/web/src/`, US8)

`api.ts`, after 035's types:

```ts
export interface ImagingSteering {
  smear_px: number; on: boolean; step_share: number; rate_share: number;
  source: "subs" | "tracker" | "none"; holding: boolean;
  steady: boolean; waiting: boolean; pause?: string;
  subs: number; dropped_not_steady: number; far_arcsec?: number;
  last?: { time: number; east_arcsec: number; north_arcsec: number };
  exposure_s?: number; limited: boolean; over_limit: boolean;
  longest_exposure_s?: number | "no limit"; would_take_s?: number;
  error_rms_arcsec?: number; error_rms_px?: number; drift_arcsec?: number; drift_px?: number;
}
// ImagingView gains: steering?: ImagingSteering;
// 035's StackDropped gains: not_steady?: number;
export async function putImagingSteering(edit: {
  on: boolean; smear_px: number; step_share: number; rate_share: number;
}): Promise<ImagingView>;
```

`putImagingSteering` is written as 031's `putImagingSettings` is and throws an `Error`
with the answer's `error` on a refusal. Its body's keys are `on`, `smear_px`,
`step_share` and `rate_share`, in that order. `pause` is a `string`, not a list of the
words this page knows: the word is the tracker's. `not_steady` is optional in
`StackDropped` so that 035's tests, whose views have eleven counts, still typecheck.

`components/SteeringBlock.tsx`:

```tsx
export function SteeringBlock({ view, onChange }: { view: ImagingView; onChange?: () => void }) {
  // <section aria-labelledby="steering-title" className="space-y-3">
  //   <h2 id="steering-title" className="section-title">Tracking from the subs</h2>
}
```

Its words, exactly, with `s = view.steering`.
- **The status**, one `role="status"` element, only with `s`. The first that applies:
  1. `s.waiting`: a pause, by `s.pause`:
     - `starting`: `Exposures paused: tracking is starting`
     - `reversing`: `Exposures paused for a moment: the mount is reversing`
     - `measuring`:
       `Exposures paused: the mount is measuring its gear play, once, for a few minutes`
     - `overhead`:
       `Exposures paused: the target is too near overhead to follow. They resume when it has passed`
     - `pushed`: `Exposures paused: the telescope is being moved by hand`
     - `stalled`:
       `Exposures paused for a moment: the mount stopped and is being started again`
     - `limit`:
       `Exposures paused: corner stars would trail. They resume when the longest exposure for round stars reaches {(exposure_s / 2).toFixed(1)} s`
     - any other word, or none: `Exposures paused: the tracker is not steady`
  2. `source` `subs`: `Steered by the subs · {subs} used`
  3. `source` `tracker`:
     - with `far_arcsec`:
       `The target is {(far_arcsec / 60).toFixed(1)}′ from the set point: the tracker steers on its own. Centre it, or turn Track off and on to apply the measured aim point.`
     - else, not `holding`: `Steered by the tracker's own corrections`
     - else, `on` false: `The switch is off: the tracker's own corrections steer`
     - else:
       `No sub accepted lately: the tracker's own corrections steer until one is. See Dropped in the stack.`
  4. `source` `none`: `Not tracking: nothing steers`
- **The readout**, a `dl` with existing classes, as `TrackControl` has one
  (`crates/finder/web/src/components/TrackControl.tsx:69`), only with `s`:
  - `Tracking error from the subs (RMS, last 10)`:
    `{error_rms_arcsec.toFixed(1)}″ · {error_rms_px.toFixed(1)} px`, only when both are
    present. The Track control beside it shows 015's
    `Tracking error (RMS, last minute)`, from the finder camera, at the same time: the
    two names must not read as one figure;
  - `Drift per sub`: `{drift_arcsec.toFixed(1)}″ · {drift_px.toFixed(1)} px`, likewise;
  - `Exposure`, only with `exposure_s`: `{exposure_s.toFixed(1)} s`, followed when
    `limited` by `, shortened to keep corner stars round`;
  - `Longest exposure ({smear_px} px corner smear)`, only with `longest_exposure_s`:
    `{longest_exposure_s.toFixed(1)} s`, or `none` for `"no limit"`. `{smear_px}` is the
    number as JavaScript prints it, `6` or `1.5`;
  - `A stack started now would take`, only with `would_take_s`:
    `{would_take_s.toFixed(1)} s`.

  `dropped_not_steady` has no term: 035's `Dropped` line, in the block above, says it.
- **The warnings**, each a `role="alert"` on one line:
  - only when `s.over_limit` and `longest_exposure_s` is a number:
    `Corner stars are trailing: the longest exposure for round stars is now {longest_exposure_s.toFixed(1)} s, and this stack takes {exposure_s.toFixed(1)} s. It keeps running. To shorten the exposure, stop it and start a new stack.`
  - only when `source` is `none` and `s.holding`:
    `Nothing is tracking, and the stack is still exposing. Turn Track on or stop the stack.`
- **The form**, always: a number field labelled `Corner smear (px)`, from `s.smear_px`
  or 6; a checkbox labelled `Steer the tracker from the subs`, from `s.on` or true; a
  line of plain text,
  `The corner smear is used when a stack starts. After a restart of the console it is 6 px again and the switch is on.`;
  under an `h3` reading `Tuning`, two number fields labelled `Step share` and
  `Rate share`, from `s.step_share` or 0.5 and `s.rate_share` or 0.3, and the line
  `For tuning in the field: the share of a tracking error one gap steps out, and of a drift it takes out of the rate. After a restart of the console they are 0.5 and 0.3 again.`;
  and the button `Save tracking settings` (`button`), which calls
  `putImagingSteering({ on, smear_px, step_share, rate_share })` and then `onChange`. A
  refusal shows `role="alert"` with the answer's `error` and calls no `onChange`.

  The four fields are drafts of the block's own. They take the view's four values when
  the block first has a `steering`, and again whenever one of those values in the view
  changes; a view that brings the same four, as the page's one-second read does, leaves
  a draft alone. Bound to the view, a field would snap back each second and no setting
  could be typed; set once, it would never show a change made elsewhere. A probe saw
  031's exposure field lose a typed value to that one-second read (reanalysis DD6; spec
  US8-S9).

Write each phrase a test asserts on one line of JSX. `ImagingPage.tsx` places
`<SteeringBlock view={view} onChange={refresh} />` right after 035's `StackBlock`.

`components/StackBlock.tsx` (035 plan.md § The page): the parts of its
`Dropped {sum}: {parts}` line gain a twelfth, last,
`{count} while the mount was not steady`, from `dropped.not_steady ?? 0`, which also
joins the sum; the filter of counts above 0 reads `(stack.dropped[key] ?? 0) > 0`, since
the key is optional. Nothing else of the block changes.

### `DESIGN.md` and `.impeccable/design.json` (US8)

After 035's entry for its stack block, a plain `###` heading and one paragraph, like its
neighbours (the `>` marks only set the text apart here):

> ### Tracking from the subs — built
> On the Imaging page, after the stack: a section titled Tracking from the subs. One
> status line says what steers the tracker, the subs or the tracker's own corrections,
> and why when it is not the subs, or that exposures are paused and for what. A short
> readout gives the tracking error and the drift per sub in arcseconds and pixels, the
> exposure, the longest exposure that keeps corner stars round and what a stack started
> now would take; an alert says when corner stars are trailing, and another when a
> stack is exposing with nothing tracking. A small form sets the corner smear in pixels
> and whether the subs steer, and under Tuning the two shares of the control law. Text
> only, with existing tokens; at night it is red like every other section.

and in `.impeccable/design.json`'s `components`, formatted as its neighbours:

```json
{"name": "Tracking from the subs", "kind": "custom", "refersTo": "button",
 "description": "Built in SteeringBlock.tsx on the Imaging page after the stack block. One status says what steers the tracker, or why exposures are paused; a definition list gives tracking error, drift per sub, exposure, the longest exposure and what a stack started now would take; a form with three number fields, a checkbox and Save tracking settings PUTs /api/imaging/steering. Existing ink, paper and red-only night tokens; no new class.",
 "html": "<section class=\"ds-steering\"><h2>Tracking from the subs</h2><p role=\"status\">Steered by the subs · 12 used</p><dl><dt>Drift per sub</dt><dd>0.6″ · 0.3 px</dd></dl><button>Save tracking settings</button></section>",
 "css": ".ds-steering{color:var(--color-ink)}.ds-steering button{border:1px solid var(--color-ink);background:transparent;color:inherit;min-height:44px}"}
```

### The MCP tool (`crates/finder/src/mcp/tools.rs`, US11)

One struct that only describes the tool's input, beside 035's `StackingArguments`,
written as 031's `ImagingSettingsArguments` writes its keys that refuse null, with
`#[expect(dead_code, reason = "schema only; SteeringEdit parses the actual arguments")]`:

```rust
#[derive(rmcp::schemars::JsonSchema)]
struct SteeringArguments {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    on: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "f64")]
    smear_px: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "f64")]
    step_share: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "f64")]
    rate_share: Option<f64>,
}
```

Written with plain `Option`s, the schema would say every argument takes null, which
`SteeringEdit` refuses (spec US11-S1).

One tool in the `#[tool_router]` block, after 035's `get_stack`, with
`output_schema = object_schema()` and `input_schema` from `SteeringArguments`, as
`set_stacking` declares its own:

- `set_steering`, description
  `Set what steers the tracker during a live stack, as PUT /api/imaging/steering does: on, smear_px, step_share and rate_share, each optional. Read the result in the steering part of get_imaging_camera.`
  It takes `arguments: JsonObject` and judges them in this order, each failure a
  `CallToolResult::error` with the words given:
  1. `serde_json::from_value::<SteeringEdit>(Value::Object(arguments))`; an error is
     serde's own words (`error.to_string()`), which no test asserts. That covers a key
     that is none of the four, a wrong type and a null;
  2. no imaging: `this console takes no imaging frames`. This is its answer to `{}` on a
     console with no imaging, at once;
  3. `crate::imaging::routes::apply_steering(&imaging, edit)` (036's, as landed:
     `crates/finder/src/imaging/routes.rs:81`; § What 036 provides, 6): its `Err`
     is the error's words, and its `Ok` is answered with `CallToolResult::structured` of
     the view.

It is not a moving tool: no rate limit and no call-log line. `get_imaging_camera` and
the session tools carry `steering` already, because they serialize the imaging view and
the session's response. `get_imaging_camera`'s own description is left as 031 wrote it,
`Read the imaging camera's state, description, status, settings and latest imaging frame.`:
031's MCP test asserts those words, and no existing test is edited. `set_steering`'s
description is where a tool listing says the steering can be read (reanalysis FF19).

In `crates/finder/README.md`, in the list of the imaging camera's tools: the count in
the sentence before the list, eleven since 035, becomes twelve; the line for
`get_imaging_camera` gains, at its end, that with a stack the view carries its `stack`
and `steering` parts; and after `get_stack`'s line comes `set_steering`'s, one sentence:
that it takes any of `on`, `smear_px`, `step_share` and `rate_share`, as
`PUT /api/imaging/steering` does, and answers the imaging view or the route's refusal as
a tool error.

## Shapes that existing tests pin

These are not changed by any story:
- **038's push.** `track_live_pointing` keeps its steps, its three push states and the
  `track.push` view (038's tests in `crates/finder/tests/motion/tracking/`). US6 adds
  the calls of `Tracker::pushed` and changes no branch of it; with no stack holding the
  tracker, one that reads not steady during a push corrects, follows and reports as
  before.
- **The tracker with no stack.** 015's tests of the take-up, the backlash measure and the
  keyhole pass (`crates/finder/tests/motion/tracking/backlash.rs`,
  `crates/finder/tests/motion/tracking/keyhole.rs`) pass unchanged: being not steady
  sends no command of its own, and with no stack no gap is asked for. Every rate is still
  sent by the rate task (§ Plan traps 1).
- **031's pinned words.** `get_imaging_camera`'s description in the tool listing is
  031's, and 031's MCP test asserts its words: US11 leaves it as it is.
- **The imaging view with steering.** From US7 on a stack uses the steering, and the
  imaging view of a console that ran one carries `steering`: 035's tests read a view key
  by key (035 spec.md § User Scenarios, Comparing) and are not disturbed. From US7 on the
  view of a console that tracks with its camera ready carries it too, used or not
  (036's `view_for`). 032's `imaging_track.rs`, the one landed test that tracks with an
  imaging solver, reads its solve and no whole view; flip.md § Prerequisites 3 checks
  that no landed test compares an imaging view, or the session's `imaging` object, whole
  on a console that tracks.
- **035's stack with no tracker.** 035's rigs do not track. `before_exposure` then
  answers at once with the observer's exposure, `Lease::set` is not called and no
  setting changes, `check` keeps every sub and `accepted` hands nothing over (spec
  US7-S4): every count and every sub of 035's scenarios is what it was, with
  `dropped.not_steady` 0. 035 says `dropped` has eleven keys, each always written, that
  its tests sum whatever keys `dropped` holds, and that "a test never asserts how many
  there are, since a later spec adds reasons" (035 spec.md § User Scenarios); a
  twelfth key at 0 disturbs neither. flip.md § Prerequisites 3 checks the landed tests for a
  comparison of the whole object with a literal, a fixed list of keys, a golden file or
  a schema that lists its keys (a comparison of two serializations of the same
  `Dropped`, as `stack_console.rs` makes between
  `stack.json` and the view, is not one) and for a `Dropped` built by a literal without
  `..Default::default()`.
- **The camera's lease and its refusals.** They are 031's, as 035 assumes them (035
  plan.md § The camera's lease). US7 takes no lease and changes no refusal: it calls
  `Lease::set` on the lease the run already holds, twice at most in a stack's life,
  and only when a tracker is running and its limit is under the observer's exposure.
  035's US11-S3, which reads a refusal by the stack's name, is not disturbed.
- **035's `StackBlock` tests.** Their views carry eleven counts; `not_steady` is
  optional in the type and adds nothing to the line when absent or 0 (spec US8-S10).
- **The MCP tool list as tests read it.** They look tools up by name, and one calls
  every listed tool with `{}` on a console with no imaging and reads its result for the
  node's token (`crates/finder/tests/mcp/motion.rs`; 035 plan.md § Shapes that existing
  tests pin). So `set_steering` answers `{}` there at once, with
  `this console takes no imaging frames` (spec US11-S3), and no existing test is edited
  for it.
- **Finder's test layout.** `crates/finder/tests/layout/main.rs` fails a file at the top
  level of `crates/finder/tests` (`crates/finder/tests/layout/main.rs:109`), a file
  added to `support` (`crates/finder/tests/layout/main.rs:127`) and a `main.rs` holding
  anything but its support include and `mod` lines
  (`crates/finder/tests/layout/main.rs:156`).
- **This spec's own shapes, for its later stories and for 037.** `SteeringView` is
  `#[non_exhaustive]` and tests read it key by key; US6 and US8 add keys to it and change
  none of 036's. `Steered` gains no method: US6 gives the tracker bodies for provided
  ones, so the scripted trackers of US6 to US8 implement 036's five required methods.
  `before_exposure`, `accepted`, `hold`, `release`, `set_set_point`, `view` and
  `view_for` keep 036's signatures, and `check` and `waiting` are added beside them. A
  case of an early story asserts nothing a later story changes, and every test file
  passes unedited under every later story, 036's among them (§ Plan traps 22 and 34).
- **What 037 reads.** Three things of this spec's, each kept as it is here (037 plan.md
  § What 030 to 036 offer, flip.md § Prerequisites 2); the other two, `Steering::set_set_point`
  and `steering.far_arcsec`, are 036's (its plan.md § Shapes that existing tests pin):
  - `Steered::steady()` through 036's `Session::steered()`;
  - `Steered::steady_between(now - reach, now)` through the same, `reach` being the
    larger of 10 s and the exposure: false for a window that touches a time from a
    tracker's start to 2 s after its first rates, a take-up, a keyhole pass or a
    measurement with the 2 s that follow the first two, a push the session reads as
    moving, or a turn that found an axis stopped; and true for a window that begins at
    the instant such a time ended (spec US10-S3 pins the window after a start, US10-S6
    after a take-up, US10-S7 across the keyhole pass, US6-S6 across a push);
  - `Steering::waiting()`, or the view's `steering.waiting`: true while a
    `before_exposure` waits, for the tracker or for its limit, the view's
    `steering.pause` saying which.

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

  Every story runs both, and the judge sees their results. The stories' development
  checks run one finder module with
  `cargo nextest run -p finder --locked --test <binary> <module>::`.
- **CI in the merge queue** runs every `ci.yml` job, including the finder-page
  build-freshness check and the build on Rust 1.96. Coverage runs only by hand.

## Plan traps

Each binds every story. They keep 036's numbers, so that a cite means the same trap in
both pieces; the ones that bind only 036's stories are 036's.

1. **A rate after a Stop.** The scope node drops a move queued before a Stop and accepts
   one that arrives after. `gap` runs in the stack's task. *Check:* `gap` only queues a
   sender and wakes the rate task; `serve_gaps` is called from `run`'s loop alone;
   036's US3-S7 asserts that a gap after a stop sends nothing.
2. **The simulator's truth on a moving mount.** Asked after the fact, it used the
   direction of now. *Check:* 036's US1 gives the simulator its memory; US7's drift
   oracle reads the truth of past instants and fails without it.
3. **Refraction in the simulator.** The tracker follows the apparent place and the
   simulator draws the geometric sky, 65″ apart at M42's altitude in the rigs. *Check:*
   the stacking rig starts on the catalogue place, as 036's US4-S5's rig does, so its
   first tracking error is small (§ Plan traps 17 says why it must be).
7. **An oracle copied from the code proves nothing.** *Check:* axis parts come from the
   simulator's horizon transform, differentiated by the test; drifts from the
   simulator's truth; the cap from 015's `longest_exposure`; rates from a wrapper round
   the mount.
8. **`SteeringView` grows by story.** *Check:* this spec's tests read it key by key.
   036's US5-S5 compares whole `steering` objects, at
   `crates/finder/tests/console/steering_cap.rs:602`,
   `crates/finder/tests/console/steering_cap.rs:633` and
   `crates/finder/tests/console/steering_cap.rs:658`: reads before and after refused or
   empty edits on one console, which does not track. It still passes from US6 and US8
   on, because the keys they add are equal in both reads: `steady` true, `waiting`
   false, no `pause` and `dropped_not_steady` 0, with no wait and no check between the
   reads, and the readout's four absent, no tracking error having been handed over.
9. **A named command is read literally.** *Check:* commands are copied from
   § Verification.
10. **One evidence budget.** *Check:* each story keeps its diff under 64 KiB, measured
    with `git diff <base> -- . ':(exclude)crates/finder/web/dist' | wc -c`. A story that
    measures over it says its size when it hands off, and drops or shortens no case to
    get under: 64 KiB is the size the judge prefers, a cut is made in the trio and not
    in a build, and a missing case is worse than a large diff (reanalysis DD12).
11. **A spec fixed mid-epic needs a relaunch.** *Check:* no builder edits
    `specs/040-stack-waits-for-steady/`, `specs/036-tracking-from-subs/` or `.specify/`.
12. **027's layout.** *Check:* each new finder test is the module § Technical Context
    names, declared by one alphabetical `mod` line; no file is added to `support`.
13. **The bundle must be rebuilt.** *Check:* US8 commits `crates/finder/web/dist` after
    `npm run build`.
14. **The lock across an await.** `Steering`'s and the tracker's locks are
    `std::sync::Mutex`. *Check:* neither is held across `gap().await`, a sleep or a
    send.
15. **A tracker kept alive.** Dropping a `Tracker` stops the mount
    (`crates/finder/src/tracking.rs:180`). *Check:* `Steering`'s state holds only `Weak`
    references; the one strong reference is the local of a `before_exposure` call, and
    it is not held across `gap().await`: `GapFuture` borrows nothing, and the call lets
    its `Arc` go before it awaits the answer; and US6-S4 drops the test's `Arc` and sees
    the sub dropped.
17. **A step against an axis's motion.** `commanded` sends zero in place of a rate
    against the axis's direction, so such a step is at most the axis's own rate times
    the half second. *Check:* the stacking rig of US7 starts on the catalogue place so
    that no scenario needs the altitude axis to back up 65″.
19. **The exposure setting left changed.** The taker owns the lease, and it drops as
    the taker's task ends. *Check:* the taker's loop runs in one `async` block, and
    after it, on every return, the taker puts the observer's exposure back through its
    lease before the task ends; nothing in 035's ending sets it; US7-S2 reads the
    setting after the stack has stopped.
22. **A gap before a tracker's first rates.** `start` answers before the rate task's
    first turn, and the task's `select!` serves a waiting gap before its first tick.
    Served like any other, that gap would send both axes zero, since `direction` is
    still `[0.0, 0.0]`; and once US6 lands it would find the tracker not steady and
    claim nothing, so a scenario that began with it would pass in 036's US3 and fail in US6,
    which may not edit 036's US3's test file. *Check:* `serve_gaps` step 1 turns a gap away
    while `settling` is true; the tracker rig waits for the first rates, and 3.5 s more,
    in every module that writes it out, because from US6 a tracker also settles for 2 s
    after them and turns a gap away until then; 036's US9-S7 asserts what a gap before the
    first rates does, and the assertion holds under 036's US3's test and under US6's; US6's
    development checks run 036's US3's and 036's US9's modules.
23. **The stack's own lease refuses outsiders.** `Imaging::set` is refused while the
    stack holds the camera, as 034's `Calibration::restore_exposure`, which went through
    it, would have been (034 has since removed it, 034 A1). *Check:* both changes of the exposure are `Lease::set` on the run's lease;
    US7-S2 reads 30000 ms while the stack runs and 60000 ms after it, and fails for a
    set that was refused.
25. **A wait left showing.** A stack stopped while `before_exposure` waits for a steady
    tracker drops that future with `waiting` true in the state, and 037 reads
    `Steering::waiting()`. *Check:* `Steering::release`, which the stack's `Hold` calls
    as it drops, sets `steady` true and `waiting` false; US6-S14 asserts it.
26. **`Steering`'s lock held across a call out.** The source reads the session under
    the session's lock, and a tracker's methods take the tracker's. *Check:* 036's
    **Lock order**, as landed (§ What 036 provides, 3): no call to the source, a tracker
    or `Imaging` is made with `Steering`'s lock held.
28. **A held correction while a measurement is open.** Gaps are turned away while the
    tracker is not steady, and 015 closes a first reversal's measurement from its own
    corrections. Held then, nothing would correct the tracker and the measurement would
    never close, with the stack waiting on it. *Check:* from US6 the held mode's test
    is `lapsed && control.steady`, and `mark` forgets `held` as the tracker stops being
    steady; US6-S5 lapses a claim before a first reversal and sees the backlash
    measured.
29. **A push the tracker is not told of.** 038 keeps a push in the session and hands the
    tracker nothing while it is `moving`. *Check:* the session calls `Tracker::pushed`
    wherever it sets or ends `Push::Moving`, with its own lock released; `follow` and
    `end` clear the flag; US6-S6 pushes the tube during a claim, sees the tracker not
    steady until the push is followed, and sees a push that comes back end it with no
    follow.
30. **A gap asked for while the tracker settles.** From US6 a gap served in the 2 s
    after a take-up is turned away, so a test that asks `gap` as soon as `steady()`
    reads false proves nothing of what the tracker forgot or sends. *Check:* the
    scenarios that mean the gap after a take-up await it once `steady()` reads true
    again (US10-S1, US10-S4, US10-S5, US10-S9, US6-S8, US6-S9); the one that means the
    gap during it asserts that it is turned away (US6-S1).
31. **A proportional rate that does not outlast the settling.** A pointing while the
    tracker is not steady, with no earlier one, sets a rate for 1 s, and a turn drops it
    inside the 2 s of settling. *Check:* the scenarios that need 015's proportional rate
    still on the mount at the first steady gap give the pointing an earlier one about
    50 s before (US10-S1, US6-S8); US10-S9 gives it one 5.4 s before the transit, so
    that the rate's time runs out between the turn that ends the settling and the gap.
32. **A stopped axis read where none stopped.** The stopped-axis test reads `moving` of
    the azimuth axis a second after a rate that was not zero. *Check:* an axis sent zero
    is not looked at, `sent` is written where the turn's loop and `serve_gaps` send the
    azimuth axis its tracking rate and not where `take_up`, `stop_axis` or `keyhole`
    command it, and 036's US3's, 036's US9's and 036's US4's motion modules, whose
    bounds a false alarm would break by dropping the claim, run in US6's development
    checks.
33. **A claim that outlives what should end it.** A claim lapses only at a gap. Where
    the steering stops asking for gaps, it must release the tracker itself, or nothing
    steers: the finder camera's pointings change no rate under a claim. *Check:* the wait
    for the limit calls 036's letting the tracker go (§ What 036 provides, 3), as
    `set_on(false)` and a `set_set_point` that changes the set point do; US6-S17 sees
    one `release` in the scripted tracker's log.
34. **A case of an early story that a later story's wait would hold.** From US6 a
    `before_exposure` waits while the limit is under half the exposure held, and a test
    of 036's US5's or US12's that answered such an exposure would then never end, in a
    file US6 may not edit. *Check:* no case of their two modules, US5's `steering_cap` of
    `console` and US12's of `motion`, has a limit under half the exposure it
    answers (036's spec.md US5-S1 says so of its own); US6's development checks run both.
35. **A first tracking error the steering stands back from.** A scenario that expects a
    `steer` must hand over a first error under 180″. *Check:* 036's US4-S4 names the set point
    (668.0, 386.0), 110″ out, where it named one 230″ out; 036's US4-S8 and US7-S7 are the
    only scenarios whose first error is over the limit.
36. **A story of tests only.** US10 writes nothing under `crates/finder/src`, and its
    tests pass on the code of US6, the story it was cut from, as soon as they are
    written: there is no failing test to write first. *Check:* its tasks say so; a test
    of its that fails is a finding about US6's code and is escalated with its output,
    not answered by an edit of that code or of the test's bound.
38. **A guarded word in a comment.** 036's US2-S6 and US7-S6 read the whole text of
    `steering.rs` and of 035's `stack/service.rs` for thirteen names, and a doc comment
    that explains a pause would use four of them. *Check:* the pause's words and their
    reasons are written in `tracking.rs` and in the page only; `steering.rs` passes
    `pause` through unread.

## Risks

| Risk | Mitigation |
|---|---|
| 035's stack or 031's lease, as landed, is not what § What 035 provides says. | Both have landed. § What 035 provides lists the nine points this spec leans on, and flip.md § Prerequisites 3 read each in their code on `main` at `fc161a54`, where each holds as written; only US7 and one line of US8 touch 035's files, and US6 and 036's stories are tested without a stack. |
| US7-S5's rig: a scripted tracker given to a console's steering, and the link dropped while its gap waits. | It ran in probe round 1: the link dropped 1 s into the 10 s gap was seen within 1 s, against 031's 3 s, and its words, its `taken` 0, its `gap`-then-`release` log and the restored setting held. The scenario's only timing is 031's 3 s for a lost link against a gap of 10 s. |
| The stacking rig of US7: a tracked simulated mount, live finder solves, a pointing model fitted from syncs, the simulated solver and 035's stack together. | It ran in probe round 1, 18 subs in about 97 s of virtual time, once its drift oracle read each saved sub from its FITS file and it waited 1 s more before the stack started (D37, D38); 036's US3-S1 and 036's US4-S5 carry the claim of SC-003 on rigs of this spec's own, so a correction to US7-S1's bound does not touch it. Probe round 2 ran it again: 18 subs, every drift from the 14th at most 0.014181″ against 0.15″ (10.6×). |
| The wait for the limit lets the tracker go, and the finder camera then steers for as long as the wait lasts. | That is the tracker's state with the switch off, and no exposure is taken in it. When the limit has risen the next gap claims afresh, and where the finder camera's aim point and the set point differ by more than 3′ the steering then stands back (036's spec.md FR-014). Not run: US6-S17 is a scripted tracker. |
| A hand on the tube during a stack. | While 038 reads the push as moving the tracker is not steady. Probe round 2 ran US6-S6 on the goto rig, and it passed as written on three runs; a real push reads `moving` from real solves, a field check. One finder solve more than 5′ wrong costs one sub. |
| The scope node stops both axes on a failed rate command and acknowledges it. | The tracker notices a turn later and drops the sub. How often a rate command fails on the real board is not known; the lake night logged a bad mount reply every 1 to 8 minutes of polling. A field check: 30 minutes of Track with the node's journal read afterwards. The node's own fix is another spec. |
| The simulated mount reads `moving` false where the tracker last sent a rate that was not zero, with nothing stopped. | Then the stopped-axis test drops claims in scenarios that do not expect it. Probe round 2 printed every stall the test found: in the 165 motion tests at US6's tip and the 174 at US10's, 036's US3's and 036's US4's modules among them, the one stall was US6-S7's own (§ Plan traps 32). |
| Rate updates once a second during an exposure. | They are 015's feed-forward and change by thousandths of an arcsecond a second; unchanged here. The first sub after a time not steady would have begun on 015's proportional rate, +0.632″/s for 0.15 s measured in round 1 and +25.3″/s in its recheck, had every gap served under a lapsed claim not sent each axis its rate (§ The tracker, `serve_gaps` step 4; US10-S1, US6-S8, US10-S9). Probe round 2 measured the feed-forward plus the offset in force at the answer in all three, to 0.000000″/s. A field check. |
| The first reversal on a mount pauses the stack for minutes. | Once per mount and axis: the backlash is kept with the fixed terms. The page says exposures are paused. Probe round 1 measured about 312 s of the simulator's time with 0.3° of play (round 2 the same), and 0.85 s for a take-up, to which the 2 s of settling are now added. With a real finder camera's scatter the measurement closes later than the simulator's 312 s: a field check. |
| A real tube rings after a take-up, a slew or a start from rest for longer than 2 s. | `SETTLE` is one constant. A field check: the first sub after a take-up. |
| US6-S5's margin, with the lapse at eight gaps and the offset left as it is after one. | Probe round 1 ran it with three gaps and the integral held (41.414″ against 015's 60″). The 41.4″ comes while the measurement is open, when the tracker is not steady and 015's corrections run whole, with a stack or without; what changes is the stretch after the close, 10.9″ in round 1, where the offset 015 built during the measurement is now left as it is. Probe round 2 ran it as written, three runs with the same digits: 41.412530″ against 60″ (1.449×) while the measurement is open, and 10.078″ after the close. |
| US10-S8's bound is not written (it was US6-S11). | No build has steered through a reversal with a rate error. Probe round 2 measures it and the number is written into the scenario, with its margin, before the flip. It is in a story of tests only, so no story's code waits on it. If it shows the offset winding where `commanded` sends zero, `serve_gaps` step 2 gains "an axis whose `base` is zero where its feed-forward is not changes no offset at this gap". |
| A story's diff nears 64 KiB. | Round 1 measured US6 at 32,913 B, US7 at 35,434 B and US8 at 34,698 B. With its cases folded in, US6 stood at about 55,200 B (84%) after the reanalysis's first pass and about 60,300 B (92%) after its second, and it was cut by tests into US10 (history.md). Estimated from those measures and the length of what the plan and the scenarios now ask for, nothing of it built: US6 about 48,400 B (74%), US7 43,500 B (66%), US8 43,700 B (67%), US10 15,300 B (23%) and US11 8,700 B (13%). Probe round 2 built US7 at 43,790 B (67%), US8 at 47,784 B (73%, `dist/` counted as binary) and US11 at 12,354 B (19%). The factory's builder has landed large stories 2% to 4% over such estimates. No story drops or shortens a case to get under (§ Plan traps 10). |
| US10-S9's timing was worked before it was measured. | It rests on the take-up starting 1.99 s before the rig's transit, as the recheck of round 1 saw it, and on the settling ending 3.0 s after that. Its first assertion fails loudly if the case was not met. Probe round 2 confirmed the 5.4 s: the interval was 3.410 s, the last azimuth rate before the gap 7.4189″/s off the feed-forward plus the offset (bound: over 1″/s) and the rate in force at its answer 0.000000″/s off (bound 0.01″/s). The story is tests only. |
| 030 to 036 and 037 write `mod.rs`, `routes.rs`, the page and the README too. | 036 and the six before it have landed, and the trio was re-anchored on their code on 2026-11-20 (flip.md § Prerequisites 2); 037 lands after this spec. |

## Refinement Record

Writes come from § Project Structure. Shared writes name the merge edge that orders two
stories of this spec on one file, under the earlier story, or the story that last wrote
the file on `main` before this spec does, read with `git log -1` at `fc161a54` by the
re-anchoring of 2026-11-20. A test file that 030 to 036 or an earlier story of
this spec created is not listed under `tests` as existing, because the validator
resolved every such path in the drafting worktree, where none of them existed; each is
named under `preserved_behavior` with what it must still assert, and the re-anchoring
left them there, as 036's left its own. The record's `open_questions` are empty because
the validator refuses a record that holds any; what is not established is in § Risks and
in flip.md § Prerequisites 2 and 3. Nothing in this record is an owner approval: those are
in flip.md, and flipping to `ready` gives them. history.md says where this record's
stories came from.
```yaml
version: 1
reviewed_revision: 49bb210fe4860dd98ba24727074e7bd3cc0d7124
stories:
  US6:
    tests:
    - path: crates/finder/tests/motion/steering_steady.rs
      status: new
      disposition: add
      assertion: "through a take-up at transit the tracker is not steady for one unbroken time that holds the take-up's commands, with the word reversing, steady_between is false for windows that touch it and true for those that do not, the claim is dropped, and a gap asked during it is served once the take-up's turn is over and turned away while the tracker settles"
      decision: "spec.md:US6-S1"
    - path: crates/finder/tests/motion/steering_steady.rs
      status: new
      disposition: add
      assertion: "the tracker is steady from 3.5 s after its start to a first reversal, and not steady while that reversal is measured, with the word measuring"
      decision: "spec.md:US6-S2"
    - path: crates/finder/tests/console/steering_steady.rs
      status: new
      disposition: add
      assertion: "before_exposure waits with no gap while the tracker is not steady, shows steady false, waiting true and the tracker's word, and answers within half a second of it being steady, after one gap, with the word gone"
      decision: "spec.md:US6-S3"
    - path: crates/finder/tests/console/steering_steady.rs
      status: new
      disposition: add
      assertion: "check drops a sub the tracker was not steady for, or whose tracker no longer exists, as not_steady and counts it, in a count a release leaves and the next Hold clears, asks about the sub's own exposure window, finds its record by its mid-exposure, keeps a sub taken with no tracker, and still drops with the switch off"
      decision: "spec.md:US6-S4"
    - path: crates/finder/tests/motion/steering_steady.rs
      status: new
      disposition: add
      assertion: "with the claim lapsed before a first reversal and gaps asked every 6 s, the tracker is not steady while the altitude backlash is measured, the subs steer at no gap, the follow keeps 015's 60 arcsec bound, the measured play is 0.3 deg within 30 arcsec, and the tracker is steady again at the end"
      decision: "spec.md:US6-S5"
    - path: crates/finder/tests/motion/steering_steady.rs
      status: new
      disposition: add
      assertion: "a push the session reads as moving makes the tracker not steady, with the word pushed, and drops the claim, a gap then claims nothing, a push that comes back ends it with no follow, and a followed push ends it, with steady_between false across it and the next gap claiming"
      decision: "spec.md:US6-S6"
    - path: crates/finder/tests/motion/steering_steady.rs
      status: new
      disposition: add
      assertion: "both axes stopped behind the tracker's back, after a turn's own azimuth rate taken 2.5 s after the gap, make it not steady from that rate to its next rates, which start the axes again, and drop the claim"
      decision: "spec.md:US6-S7"
    - path: crates/finder/tests/motion/steering_steady.rs
      status: new
      disposition: add
      assertion: "a pointing inside 015's deadband between the settling and a lapsed gap leaves no proportional rate in force at that gap's answer"
      decision: "spec.md:US6-S8"
    - path: crates/finder/tests/motion/steering_steady.rs
      status: new
      disposition: add
      assertion: "a tracking error handed over while the tracker is not steady, or measured before it stopped being steady, steers nothing and takes no lapsed tracker back, and a fresh one does"
      decision: "spec.md:US6-S9"
    - path: crates/finder/tests/motion/steering_steady.rs
      status: new
      disposition: add
      assertion: "a gap queued behind a take-up is answered with no step when tracking ends"
      decision: "spec.md:US6-S10"
    - path: crates/finder/tests/console/steering_steady.rs
      status: new
      disposition: add
      assertion: "a tracker that stops being steady while its gap is awaited shows steady false, waiting true and the tracker's pause word"
      decision: "spec.md:US6-S11"
    - path: crates/finder/tests/console/steering_steady.rs
      status: new
      disposition: add
      assertion: "a gap that ends on a tracker no longer steady does not count: before_exposure answers after a second gap"
      decision: "spec.md:US6-S12"
    - path: crates/finder/tests/console/steering_steady.rs
      status: new
      disposition: add
      assertion: "before any wait the view has steady true, waiting false, no pause word and no drop, with a tracker before any before_exposure and after one, and with none"
      decision: "spec.md:US6-S13"
    - path: crates/finder/tests/console/steering_steady.rs
      status: new
      disposition: add
      assertion: "a wait abandoned with its stack no longer shows"
      decision: "spec.md:US6-S14"
    - path: crates/finder/tests/console/steering_steady.rs
      status: new
      disposition: add
      assertion: "the switch off still waits for a steady tracker and asks for no gap, also when turned off during the wait"
      decision: "spec.md:US6-S15"
    - path: crates/finder/tests/console/steering_steady.rs
      status: new
      disposition: add
      assertion: "a source that no longer answers a tracker ends the wait within a poll"
      decision: "spec.md:US6-S16"
    - path: crates/finder/tests/console/steering_steady.rs
      status: new
      disposition: add
      assertion: "while the limit is under half the exposure held before_exposure waits with the word limit, the tracker released once and no gap asked, and answers once the limit is half of it again; and a first call whose 1 s floor is more than twice the limit waits once the cap has decided"
      decision: "spec.md:US6-S17"
    - path: crates/finder/tests/motion/tracking/backlash.rs
      status: existing
      disposition: preserve
      assertion: "take-ups and the first reversal's measure are unchanged"
      decision: "plan.md:Shapes that existing tests pin"
    - path: crates/finder/tests/motion/tracking/keyhole.rs
      status: existing
      disposition: preserve
      assertion: "the keyhole pass is unchanged"
      decision: "plan.md:Shapes that existing tests pin"
    - path: crates/finder/tests/layout/main.rs
      status: existing
      disposition: preserve
      assertion: "console's and motion's main.rs hold only their support include and mod lines, with steering_steady added to each"
      decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
    - "015's take-up, backlash measure and keyhole pass send the commands they sent (spec.md FR-021)"
    - "a scripted tracker that is always steady is never waited for (036's US4's and 036's US5's scenarios)"
    - "crates/finder/tests/motion/steering.rs, 036's US3's test file, passes unchanged: its tracker rig waits 3.5 s after the first rates before a scenario asks for a gap, by when the tracker has settled, and no scenario of it, 036's US3-S11 with its held step among them, has a time the tracker is not steady after that wait, so the held mode's new test and the errors dropped by their time change nothing there"
    - "crates/finder/tests/motion/steering_gap.rs, 036's US4's test file, passes unchanged: its rig waits 3.5 s after the first rates too, and it meets no reversal and no push"
    - "motion/steering_claim.rs, 036's US9's, passes unchanged as 036's US3's does: its rig waits 3.5 s, 036's US9-S7 expects what a tracker that is not steady answers, 036's US9-S9 waits 4.5 s, and its lapsed scenarios meet no time not steady"
    - "036's US5's console steering_cap.rs and US12's motion steering_cap.rs pass unchanged: no case has a limit under half the exposure it answers, so none waits"
    - "crates/finder/tests/motion/tracking/push.rs and push_back.rs, 038's test files, pass unchanged: no stack holds their tracker, so a push that reads not steady changes nothing they read"
    checks:
      development:
      - "cargo nextest run -p finder --locked --test motion steering_steady::"
      - "cargo nextest run -p finder --locked --test console steering_steady::"
      - "cargo nextest run -p finder --locked --test motion steering::"
      - "cargo nextest run -p finder --locked --test motion steering_claim::"
      - "cargo nextest run -p finder --locked --test motion steering_cap::"
      - "cargo nextest run -p finder --locked --test console steering_cap::"
      - "cargo nextest run -p finder --locked --test motion steering_gap::"
      - "cargo nextest run -p finder --locked --test motion tracking::"
      - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
      - "release owner: the repository owner, through the merge queue's ci.yml jobs, and a run through the meridian at the telescope"
    writes:
    - crates/finder/src/imaging/steering.rs
    - crates/finder/src/session.rs
    - crates/finder/src/tracking.rs
    - crates/finder/tests/console/main.rs
    - crates/finder/tests/console/steering_steady.rs
    - crates/finder/tests/motion/main.rs
    - crates/finder/tests/motion/steering_steady.rs
    no_write_reason: ""
    shared_writes:
    - path: crates/finder/src/imaging/steering.rs
      other_spec: 036-tracking-from-subs
      other_story: US5
      disposition: merge_order
      reason: "040 depends_on_landed 036, whose US5 last writes steering.rs, with the cap and the settings"
    - path: crates/finder/src/tracking.rs
      other_spec: 036-tracking-from-subs
      other_story: US5
      disposition: merge_order
      reason: "040 depends_on_landed 036, whose US5 last writes tracking.rs, with the two limits and tune"
    - path: crates/finder/src/session.rs
      other_spec: 036-tracking-from-subs
      other_story: US4
      disposition: merge_order
      reason: "040 depends_on_landed 036, whose US4 last writes session.rs, with Session::steered"
    - path: crates/finder/tests/console/main.rs
      other_spec: 036-tracking-from-subs
      other_story: US5
      disposition: merge_order
      reason: "040 depends_on_landed 036, whose US5 last writes console/main.rs, with mod steering_cap"
    - path: crates/finder/tests/motion/main.rs
      other_spec: 036-tracking-from-subs
      other_story: US12
      disposition: merge_order
      reason: "040 depends_on_landed 036, whose US12 last wrote motion/main.rs, with mod steering_cap (fc161a54)"
    - path: crates/finder/src/imaging/steering.rs
      other_spec: 040-stack-waits-for-steady
      other_story: US8
      disposition: merge_order
      reason: "US8 depends_on_merged US6, and adds the readout's figures"
    - path: crates/finder/tests/motion/main.rs
      other_spec: 040-stack-waits-for-steady
      other_story: US10
      disposition: merge_order
      reason: "US10 depends_on_merged US6, and adds mod steering_unsteady after mod steering_steady"
    - path: crates/finder/tests/console/main.rs
      other_spec: 040-stack-waits-for-steady
      other_story: US8
      disposition: merge_order
      reason: "US8 depends_on_merged US6, and adds mod steering_readout"
    open_questions: []
  US7:
    tests:
    - path: crates/finder/tests/motion/steering_stack.rs
      status: new
      disposition: add
      assertion: "a stack on a tracked mount 1% fast, each saved sub read from its FITS file, is steered by the subs, drifts at most 0.15 arcsec a sub from the 14th on, drops nothing as not_steady and lets the tracker go when stopped"
      decision: "spec.md:US7-S1"
    - path: crates/finder/tests/motion/steering_stack.rs
      status: new
      disposition: add
      assertion: "with the smear set to 2 px the view says before the stack that it would take 30 s of a 60 s setting, and the stack takes 30 s under the tracker's limit, set through its own lease, shown as limited and written as the sub's EXPTIME, the setting reading 30 s while it runs and 60 s after"
      decision: "spec.md:US7-S2"
    - path: crates/finder/tests/motion/steering_stack.rs
      status: new
      disposition: add
      assertion: "a sub during which Track was turned off and on is dropped as not_steady, counted in the stack's twelfth count, in the steering's and in stack.json's subs, the stack's counts still add up, and total() adds the twelfth; and such a sub reaches neither the stack's last outcome nor a solve sidecar"
      decision: "spec.md:US7-S3"
    - path: crates/finder/tests/motion/steering_stack.rs
      status: new
      disposition: add
      assertion: "a stack with no tracker is neither capped nor paused, hands nothing over and drops nothing as not_steady"
      decision: "spec.md:US7-S4"
    - path: crates/finder/tests/motion/steering_stack.rs
      status: new
      disposition: add
      assertion: "a stack whose change of the exposure is refused ends with the reason camera and the stated words, takes no sub, lets the tracker go and leaves the observer's exposure as it was"
      decision: "spec.md:US7-S5"
    - path: crates/finder/tests/motion/steering_stack.rs
      status: new
      disposition: add
      assertion: "035's stack/service.rs, as this story leaves it, holds none of the thirteen guarded names"
      decision: "spec.md:US7-S6"
    - path: crates/finder/tests/motion/steering_stack.rs
      status: new
      disposition: add
      assertion: "with the session's aim point 16 finder pixels from the true one, the steering stands back from the stack: far_arcsec is about 477, no push is read, and the eyepiece stays within 60 arcsec of where it was"
      decision: "spec.md:US7-S7"
    - path: crates/finder/tests/motion/steering_stack.rs
      status: new
      disposition: add
      assertion: "with the switch off and the finder camera's frames stopped, a stack hands nothing over and most subs drift over 0.5 arcsec"
      decision: "spec.md:US7-S8"
    - path: crates/finder/tests/motion/steering_stack.rs
      status: new
      disposition: add
      assertion: "a stop while the first gap waits ends the stack at once with nothing taken and the tracker let go"
      decision: "spec.md:US7-S9"
    - path: crates/finder/tests/motion/steering_stack.rs
      status: new
      disposition: add
      assertion: "a link lost while the second gap waits ends the stack as camera with the take's own words, and the refused change back leaves the setting at the stack's exposure"
      decision: "spec.md:US7-S10"
    - path: crates/finder/tests/layout/main.rs
      status: existing
      disposition: preserve
      assertion: "motion's main.rs holds only its support include and mod lines, with steering_stack added"
      decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
    - "a stack with Track off takes, solves and stacks its subs as 035 has it: nothing is capped, paused or dropped by the steering (spec.md US7-S4)"
    - "settings.exposure_ms is the observer's own after a stack (US7-S2)"
    - "the test files 035 creates (stack_loop.rs, stack_camera.rs, stack_gate.rs, stack_display.rs, stack_console.rs, the cli and mcp stack modules) pass unchanged: they read dropped key by key and never count its keys, their rigs do not track, and no exposure is changed for them (plan.md Shapes that existing tests pin, Prerequisites 3)"
    - "the camera's lease and its refusals are 031's as 035 assumes them: the stack changes the exposure through the lease it already holds and takes no other (plan.md What 035 provides)"
    - "crates/finder/tests/console/steering_gap.rs, steering_cap.rs and steering_steady.rs, 036's US4's, 036's US5's and US6's test files, pass unchanged: Steering's calls behave as before the stack used them"
    checks:
      development:
      - "cargo nextest run -p finder --locked --test motion steering_stack::"
      - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
      - "release owner: the repository owner, through the merge queue's ci.yml jobs, and a stack with and without the switch at the telescope"
    writes:
    - crates/finder/README.md
    - crates/finder/src/imaging/solving.rs
    - crates/finder/src/imaging/stack/service.rs
    - crates/finder/tests/motion/main.rs
    - crates/finder/tests/motion/steering_stack.rs
    no_write_reason: ""
    shared_writes:
    - path: crates/finder/README.md
      other_spec: 036-tracking-from-subs
      other_story: US5
      disposition: merge_order
      reason: "040 depends_on_landed 036, whose US5 last writes the README, with the route's row"
    - path: crates/finder/src/imaging/solving.rs
      other_spec: 032-imaging-solve-and-aim-point
      other_story: US5
      disposition: merge_order
      reason: "040 depends_on_landed 032, whose US5 last writes solving.rs"
    - path: crates/finder/README.md
      other_spec: 040-stack-waits-for-steady
      other_story: US8
      disposition: merge_order
      reason: "US8 depends_on_merged US7, and adds the readout's four keys to US7's section"
    - path: crates/finder/src/imaging/stack/service.rs
      other_spec: 035-live-stack
      other_story: US8
      disposition: merge_order
      reason: "040 depends_on_landed 035, whose US8 last wrote the stack's service (6a519d49)"
    open_questions: []
  US8:
    tests:
    - path: crates/finder/tests/console/steering_readout.rs
      status: new
      disposition: add
      assertion: "three worked tracking errors with one step between give a drift of 0.58583 arcsec and an error of 0.90370 RMS, the second's 1/sqrt(2) written as FRAC_1_SQRT_2, and their pixels at the newest error's scale, 2.16 arcsec a pixel or half of it; a step before the first error is not taken out; one error gives no figure; and a gap asked before the latest sub is handed over, two gaps of one call and a north step each count as the drift says"
      decision: "spec.md:US8-S1"
    - path: crates/finder/tests/console/steering_readout.rs
      status: new
      disposition: add
      assertion: "the window is the last ten errors, neither nine nor eleven, its drifts those after the window's first error, and a release and letting the tracker go for the limit each empty it"
      decision: "spec.md:US8-S2"
    - path: crates/finder/web/src/components/SteeringBlock.test.tsx
      status: new
      disposition: add
      assertion: "the block reads Steered by the subs with the count used, the error, the drift, the exposure, the longest exposure with its smear and what a stack started now would take, to one decimal, those five terms alone, and no alert"
      decision: "spec.md:US8-S3"
    - path: crates/finder/web/src/components/SteeringBlock.test.tsx
      status: new
      disposition: add
      assertion: "a shortened exposure, the smear in the term's name, no limit, trailing corner stars, what a stack would take, a missing pixel figure and a drop count with no term each read their own words, and without steering only the form shows, with its four defaults"
      decision: "spec.md:US8-S4"
    - path: crates/finder/web/src/components/SteeringBlock.test.tsx
      status: new
      disposition: add
      assertion: "Save tracking settings PUTs on, smear_px and the typed step_share and rate_share to /api/imaging/steering, and a refusal is an alert with the answer's words"
      decision: "spec.md:US8-S5"
    - path: crates/finder/web/src/components/SteeringBlock.test.tsx
      status: new
      disposition: add
      assertion: "the Imaging page shows the block after the stack block, and a save there reads the view again"
      decision: "spec.md:US8-S6"
    - path: crates/finder/web/src/components/SteeringBlock.test.tsx
      status: new
      disposition: add
      assertion: "the status says why the subs do not steer: no stack, no sub accepted lately, the switch off, a target too far out, or nothing tracking, with an alert when a stack is exposing then"
      decision: "spec.md:US8-S7"
    - path: crates/finder/web/src/components/SteeringBlock.test.tsx
      status: new
      disposition: add
      assertion: "a wait reads the sentence of its pause word, the general one for an unknown word or none, and nothing is a pause that is not a wait"
      decision: "spec.md:US8-S8"
    - path: crates/finder/web/src/components/SteeringBlock.test.tsx
      status: new
      disposition: add
      assertion: "the form's fields are drafts: a render with the same settings leaves a typed value, changed settings replace it, and a block first given a steering takes its values"
      decision: "spec.md:US8-S9"
    - path: crates/finder/web/src/components/SteeringBlock.test.tsx
      status: new
      disposition: add
      assertion: "the stack block's Dropped line names subs dropped while the mount was not steady, and reads a view without that count as before"
      decision: "spec.md:US8-S10"
    - path: crates/finder/web/src/components/TrackControl.test.tsx
      status: existing
      disposition: preserve
      assertion: "015's Track control and its readout read as before"
      decision: "plan.md:Shapes that existing tests pin"
    - path: crates/finder/tests/layout/main.rs
      status: existing
      disposition: preserve
      assertion: "console's main.rs holds only its support include and mod lines, with steering_readout added"
      decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
    - "the page adds no npm package (plan.md Technical Context)"
    - "red-only night mode: the block uses existing tokens and classes; 031's night check of the Imaging page still counts no off-red pixel"
    - "track.readout and the Track control are unchanged"
    - "StackBlock.test.tsx, 035's test file, passes unchanged: a dropped object without not_steady reads as it did (spec.md US8-S10)"
    - "CalibrationBlock.test.tsx and ImagingPage.test.tsx pass unchanged: they read no region by its place from the page's end"
    checks:
      development:
      - "cargo nextest run -p finder --locked --test console steering_readout::"
      - "npm test"
      - "npm run typecheck"
      - "npm run build"
      handoff: declared_gates
      independent: declared_pipeline
      release:
      - "release owner: the repository owner, through the merge queue's ci.yml jobs: finder-page, and the Impeccable review of the block"
    writes:
    - .impeccable/design.json
    - DESIGN.md
    - crates/finder/README.md
    - crates/finder/src/imaging/steering.rs
    - crates/finder/tests/console/main.rs
    - crates/finder/tests/console/steering_readout.rs
    - crates/finder/web/dist
    - crates/finder/web/src/api.ts
    - crates/finder/web/src/components/ImagingPage.tsx
    - crates/finder/web/src/components/StackBlock.tsx
    - crates/finder/web/src/components/SteeringBlock.test.tsx
    - crates/finder/web/src/components/SteeringBlock.tsx
    no_write_reason: ""
    shared_writes:
    - path: crates/finder/README.md
      other_spec: 040-stack-waits-for-steady
      other_story: US11
      disposition: merge_order
      reason: "US11 depends_on_merged US8, and adds the tool's lines to the README's list of tools"
    - path: .impeccable/design.json
      other_spec: 035-live-stack
      other_story: US9
      disposition: merge_order
      reason: "040 depends_on_landed 035, whose US9 last wrote it (6e506ec6)"
    - path: DESIGN.md
      other_spec: 035-live-stack
      other_story: US9
      disposition: merge_order
      reason: "040 depends_on_landed 035, whose US9 last wrote it (6e506ec6)"
    - path: crates/finder/web/dist
      other_spec: 035-live-stack
      other_story: US9
      disposition: merge_order
      reason: "040 depends_on_landed 035, whose US9 last wrote it (6e506ec6)"
    - path: crates/finder/web/src/api.ts
      other_spec: 035-live-stack
      other_story: US9
      disposition: merge_order
      reason: "040 depends_on_landed 035, whose US9 last wrote it (6e506ec6)"
    - path: crates/finder/web/src/components/ImagingPage.tsx
      other_spec: 035-live-stack
      other_story: US9
      disposition: merge_order
      reason: "040 depends_on_landed 035, whose US9 last wrote it (6e506ec6)"
    - path: crates/finder/web/src/components/StackBlock.tsx
      other_spec: 035-live-stack
      other_story: US9
      disposition: merge_order
      reason: "040 depends_on_landed 035, whose US9 last wrote it (6e506ec6)"
    open_questions: []
  US10:
    tests:
    - path: crates/finder/tests/motion/steering_unsteady.rs
      status: new
      disposition: add
      assertion: "a lapsed gap, once a take-up has settled, leaves no proportional rate in force at its answer"
      decision: "spec.md:US10-S1"
    - path: crates/finder/tests/motion/steering_unsteady.rs
      status: new
      disposition: add
      assertion: "while a first reversal is measured the tracker is not steady, says measuring and a gap claims nothing, and it is steady once the backlash is measured"
      decision: "spec.md:US10-S2"
    - path: crates/finder/tests/motion/steering_unsteady.rs
      status: new
      disposition: add
      assertion: "the tracker is not steady before its first rates nor for exactly 2 s after them, saying starting, nor in any window before its start, and a window that begins as it is steady again is steady; a 10 s window ending now is false 6 s after the start and true 14 s after it; and no window past its stop is steady"
      decision: "spec.md:US10-S3"
    - path: crates/finder/tests/motion/steering_unsteady.rs
      status: new
      disposition: add
      assertion: "a take-up forgets a step held under a lapsed claim"
      decision: "spec.md:US10-S4"
    - path: crates/finder/tests/motion/steering_unsteady.rs
      status: new
      disposition: add
      assertion: "a take-up drops the claim and the error not yet applied, and the next steady gap claims again"
      decision: "spec.md:US10-S5"
    - path: crates/finder/tests/motion/steering_unsteady.rs
      status: new
      disposition: add
      assertion: "the trait answers steady, steady_between and pause as the tracker does, before its first rates and still after a take-up"
      decision: "spec.md:US10-S6"
    - path: crates/finder/tests/motion/steering_unsteady.rs
      status: new
      disposition: add
      assertion: "the keyhole pass is a time not steady, with the word overhead and steady_between false across its start, and the tracker is steady after it"
      decision: "spec.md:US10-S7"
    - path: crates/finder/tests/motion/steering_unsteady.rs
      status: new
      disposition: add
      assertion: "subs played late on a mount 1% fast through the take-up at transit drift at most the bound probe round 2 measures, from the fourth sub after the tracker is steady again"
      decision: "spec.md:US10-S8"
    - path: crates/finder/tests/motion/steering_unsteady.rs
      status: new
      disposition: add
      assertion: "a proportional rate whose time ran out between the settling's last turn and a lapsed gap is on the mount before that gap and gone at its answer"
      decision: "spec.md:US10-S9"
    - path: crates/finder/tests/layout/main.rs
      status: existing
      disposition: preserve
      assertion: "motion's main.rs holds only its support include and mod lines, with steering_unsteady added"
      decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
    - "no file under crates/finder/src changes: each scenario tests US6's tracker and passes on it as soon as it is written, US10-S8 once it has its bound"
    - "motion/steering_steady.rs, US6's test file, passes unedited"
    - "015's backlash and keyhole tests pass unchanged"
    checks:
      development:
      - "cargo nextest run -p finder --locked --test motion steering_unsteady::"
      - "cargo nextest run -p finder --locked --test motion steering_steady::"
      - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
      - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
    - crates/finder/tests/motion/main.rs
    - crates/finder/tests/motion/steering_unsteady.rs
    no_write_reason: ""
    shared_writes:
    - path: crates/finder/tests/motion/main.rs
      other_spec: 040-stack-waits-for-steady
      other_story: US7
      disposition: merge_order
      reason: "US7 depends_on_merged US10, and adds mod steering_stack before mod steering_steady"
    open_questions: []
  US11:
    tests:
    - path: crates/finder/tests/mcp/steering.rs
      status: new
      disposition: add
      assertion: "set_steering is listed with its exact description, an object output schema, no read-only hint and the four optional arguments, typed boolean or number and none nullable"
      decision: "spec.md:US11-S1"
    - path: crates/finder/tests/mcp/steering.rs
      status: new
      disposition: add
      assertion: "set_steering sets any of the four settings and answers the imaging view, get_imaging_camera and get_session carry the same steering, and empty arguments change nothing"
      decision: "spec.md:US11-S2"
    - path: crates/finder/tests/mcp/steering.rs
      status: new
      disposition: add
      assertion: "bad arguments answer a tool error with the route's words and change nothing, and a console with no imaging answers at once, judging bad arguments before it says it takes no imaging frames"
      decision: "spec.md:US11-S3"
    - path: crates/finder/tests/mcp/motion.rs
      status: existing
      disposition: preserve
      assertion: "every listed tool, set_steering among them, answers empty arguments on a console with no imaging, and no result holds the node token"
      decision: "plan.md:Shapes that existing tests pin"
    - path: crates/finder/tests/layout/main.rs
      status: existing
      disposition: preserve
      assertion: "mcp's main.rs holds only its support include and mod lines, with steering added"
      decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
    - "get_imaging_camera's description and every other tool are as they were: 031's to 035's mcp test files pass unchanged"
    - "PUT /api/imaging/steering answers as before: the tool calls the route's own function"
    checks:
      development:
      - "cargo nextest run -p finder --locked --test mcp"
      - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
      - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
    - crates/finder/README.md
    - crates/finder/src/mcp/tools.rs
    - crates/finder/tests/mcp/main.rs
    - crates/finder/tests/mcp/steering.rs
    no_write_reason: ""
    shared_writes:
    - path: crates/finder/src/mcp/tools.rs
      other_spec: 035-live-stack
      other_story: US10
      disposition: merge_order
      reason: "040 depends_on_landed 035, whose US10 last writes the MCP tools"
    - path: crates/finder/tests/mcp/main.rs
      other_spec: 035-live-stack
      other_story: US10
      disposition: merge_order
      reason: "040 depends_on_landed 035, whose US10 last writes mcp/main.rs, with mod stack"
    open_questions: []
```
