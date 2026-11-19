# Implementation Plan: Tracking Steered by the Subs

**Branch**: `036-tracking-from-subs` | **Date**: 2026-11-15 | **Spec**: [spec.md](spec.md)

This plan records every design decision for the feature. A story that would change one
escalates rather than re-deciding it.

## Summary

Give the simulator a memory of where its telescope pointed, so the truth of a sub's
mid-exposure is right on a moving mount, and a switch that makes its mount move in
whole counts. Define, in `crates/finder/src/imaging/steering.rs`, the tracking error (the
sky at the set point less the target, from a sub's imaging solve) and `Steered`, the one
interface between the stack and a tracker. Implement `Steered` for 015's tracker: once a
stack claims it, the finder camera's pointings change no rate; in each gap the tracker
takes half of each axis's error out as a half-second pulse when the error is two counts
or more, takes 0.3 of the drift seen between two subs out of the rate, sends both axes
their rates and answers when they are acknowledged. Because 035's worker solves a sub
while the next is exposing, a tracking error arrives a gap or more late: the tracker
remembers what its last gaps did and judges each error against them. After three gaps
with no tracking error it goes back to its own corrections and stays with them until a
sub is accepted again; while the stack still holds it, those corrections are held for
the next gap and applied there, so that no rate changes during an exposure whichever
steers. Add `Steering`,
the console's part between the two: it calls the gap before every exposure, hands over
the tracking error of each sub the stack accepts, caps a stack's exposure at the
tracker's limit for a chosen smear by halving it once, waits while the tracker is not
steady and tells the stack to drop a sub taken across such a time. Wire it into 035's
taker and worker, the taker setting the capped exposure through the stack's own lease
on the camera and putting the observer's back as the stack ends; show the steering in
the imaging view as `steering`, set its two settings with `PUT /api/imaging/steering`,
and give the Imaging page a block for the readout.

## Technical Context

**Language/Version**: Rust (MSRV 1.96) for US1 to US7 and US8's readout; TypeScript with
React 19 for US8's page.

**Primary Dependencies**: nothing new, and no `Cargo.toml` changes. `finder` already has
`tokio` with `sync` and `time`, `serde` and `axum`; the simulator already has what its
memory needs in `std`.

**Storage**: none. The two settings and the readout are run-only.

**Testing**:
- US1: `crates/simulator/tests/imaging_history.rs` (new).
- US2: `crates/finder/tests/console/steering_error.rs` (new).
- US3: `crates/finder/tests/motion/steering.rs` (new).
- US4: `crates/finder/tests/console/steering_gap.rs` and, for US4-S5 and US4-S6,
  `crates/finder/tests/motion/steering_gap.rs` (new).
- US5: `crates/finder/tests/console/steering_cap.rs` and, for US5-S6,
  `crates/finder/tests/motion/steering_cap.rs` (new).
- US6: `crates/finder/tests/motion/steering_steady.rs` and
  `crates/finder/tests/console/steering_steady.rs` (new).
- US7: `crates/finder/tests/motion/steering_stack.rs` (new).
- US8: `crates/finder/tests/console/steering_readout.rs` and
  `crates/finder/web/src/components/SteeringBlock.test.tsx` (new, Vitest).
- Where a finder test goes is 027's rule: `console` calls the console in process and
  moves no mount (`crates/finder/tests/console/main.rs:2`), `motion` drives the
  simulated mount in paused time (`crates/finder/tests/motion/main.rs:1`). Each module
  is declared by one `mod` line, in alphabetical order, in its binary's `main.rs`
  (`crates/finder/tests/layout/main.rs:156`), and writes its own setup:
  `crates/finder/tests/support` takes no new file
  (`crates/finder/tests/layout/main.rs:127`). A module may use what the existing support
  files already offer: `crate::support::tracker` (the backlash rig, US6),
  `crate::support::goto` (US3-S8, US4-S6) and `crate::support::Scratch`
  (`crates/finder/tests/support/mod.rs:14`). The tracker rig, with its wait for the first
  rates, and the scripted tracker of spec.md are written out in each module that uses
  them, only the parts its own tests use, because an unused helper fails `clippy -D
  warnings` and a story may not edit an earlier story's test file. 035's rigs live in its `console` module `stack_loop`, which
  the `motion` binary cannot import; US7 writes the stacking rig of spec.md itself.
- No Playwright test (spec owner decision 12).

**Target Platform**: the `finder` console on the console machine. Nothing changes on the
scope node or the scope link.

**Constraints**:
- No change under `crates/scope-node`, `crates/scope-link` or `proto/`; no `unsafe`; no
  dependency.
- No file under `crates/finder/src/imaging/` computes an altitude, an azimuth, a
  parallactic angle or a field rotation, or reads an axis count (the plan document,
  § Mount-neutral by design, rule 1; US2-S6 guards `steering.rs`, and US7-S6 035's
  `stack/service.rs`, the file under `crates/finder/src/imaging/` that US7 edits most).
- Nothing is sized to one telescope or one sub length (rules 3 and 6): the deadband is
  in the mount's own counts, and the freshness rule, the readout's window, the
  exposures the steering remembers and the gaps the tracker remembers are counted in
  subs. The two times this spec fixes, the half-second pulse and the quarter-second
  poll, belong to the gap and not to a sub.
- The imaging path's words are mount-neutral (rule 5): `steering.rs`, the view and the
  page say "the tracker", never "the finder camera", an axis or field rotation.
- The steering takes no exposure and holds no lock on the camera (the plan document,
  § Amendments, 1), and never keeps a tracker alive (§ Plan traps 15). The one camera
  setting this spec changes, a stack's exposure, is changed by the stack through the
  lease it already holds (§ The stack; § What 035 must provide).
- Every rate is sent by the tracker's own task (§ Plan traps 1).
- While a stack holds the tracker, no rate changes between two of its gaps but the
  once-a-second feed-forward, whether the subs steer or have lapsed (§ Plan traps 27);
  the switch turned off is the one declared exception (spec owner decision 8).
- `web/dist` is committed and must be the build of its source (US8).

**Scale/Scope**: one new module, one route, one page block, about 340 lines added to the
tracker, 60 to the simulator and 40 to 035's stack.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Nothing touches the finder camera's solver, detection or `crates/math-core`. Console work is grounded in the cited sources: the research note for the mount's counts and the finder camera's scatter, 015's own functions for the field rotation, and the simulator's truth. The control law's constants are this spec's own design, named for the owner (spec owner decisions 4 and 5). |
| II. Never weaken a gate | No existing test, golden, fixture, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS`, `ergane.yaml` or `playwright.config.ts` is edited. New tests are new files; each finder module adds one `mod` line to its binary's `main.rs`, which holds no test (spec owner approval 9). No existing assertion changes: 035's tests read `dropped` key by key and never count its keys (035 spec.md § User Scenarios), and § Prerequisites 3 checks that in the landed tests. |
| III. Tests first, independent expectations | Oracles: the simulator's truth, read by each test; worked examples whose arithmetic is in spec.md; the simulator's own horizon transform, differentiated by the test, for the axes; 015's `longest_exposure` for the cap; the rate log of a wrapper the test puts round the mount. Every scenario names its oracle and tolerance. Time is virtual throughout. |
| IV. Critical paths | No `unsafe`, no `Cargo.toml` or `Cargo.lock` change, nothing under `proto/` or `crates/math-core`. |
| V. Proven open source first, honest licensing | Unaffected: no solver is touched and nothing is copied. 032's use of ASTAP is 032's to settle (its plan.md § Prerequisites 4). |

## Project Structure

```text
crates/simulator/
├── src/optics.rs                        # US1: the memory, direction_at
├── src/controls.rs                      # US1: remember on every read, forget on a push
├── src/mount.rs                         # US1: set_count_steps
└── tests/imaging_history.rs             # US1: new
crates/finder/
├── src/imaging/steering.rs              # US2: new; US4: Steering; US5: the cap; US6: steady, check; US8: the readout
├── src/imaging/mod.rs                   # US2: pub mod steering; US4: the view's part, Imaging::steering
├── src/imaging/routes.rs                # US5: PUT /api/imaging/steering
├── src/imaging/stack/service.rs         # US7: the taker's and the worker's calls, Dropped::not_steady (035's file)
├── src/imaging/solving.rs               # US7: set_solver gives the steering its source (032's file)
├── src/tracking.rs                      # US3: Steered for Tracker; US5: longest_exposure_s; US6: steady
├── src/session.rs                       # US4: Session::steered
├── src/lib.rs                           # US5: the route's doc line
├── README.md                            # US5: the route's row; US7: the section
├── tests/console/main.rs                # US2, US4, US5, US6, US8: one mod line each
├── tests/console/steering_error.rs      # US2: new
├── tests/console/steering_gap.rs        # US4: new
├── tests/console/steering_cap.rs        # US5: new
├── tests/console/steering_steady.rs     # US6: new
├── tests/console/steering_readout.rs    # US8: new
├── tests/motion/main.rs                 # US3 to US7: one mod line each
├── tests/motion/steering.rs             # US3: new
├── tests/motion/steering_gap.rs         # US4: new
├── tests/motion/steering_cap.rs         # US5: new
├── tests/motion/steering_steady.rs      # US6: new
├── tests/motion/steering_stack.rs       # US7: new
└── web/
    ├── src/api.ts                       # US8: ImagingSteering, putImagingSteering, StackDropped.not_steady
    ├── src/components/SteeringBlock.tsx       # US8: new
    ├── src/components/SteeringBlock.test.tsx  # US8: new
    ├── src/components/StackBlock.tsx    # US8: the words `not steady` (035's file)
    ├── src/components/ImagingPage.tsx   # US8: places the block
    └── dist/                            # US8: rebuilt
GLOSSARY.md                              # US2: Tracking error, Set point
DESIGN.md                                # US8: the block
.impeccable/design.json                  # US8: the component
```

## Interfaces

Coordinates, sky offsets and the worked examples are as spec.md § User Scenarios defines
them. Everything 030 to 035 create is named as their plan.md gives it and is re-read on
`main` before the flip (§ Prerequisites 2 and 3). What this spec uses or changes of
035's is gathered in § What 035 must provide.

### The simulator's memory (`crates/simulator/src/optics.rs`, `crates/simulator/src/controls.rs`, US1)

`optics::Simulator` (`crates/simulator/src/optics.rs:74`) gains one field, a shared
`VecDeque` of samples `(unix_s, alt, az)`, the state's `alt` and `az` as `Pointing`
holds them (`crates/simulator/src/optics.rs:65`), and:

```rust
/// How long the simulator remembers where its telescope pointed, in seconds of its
/// clock: an hour, the simulated imaging camera's longest exposure (030).
pub const HISTORY_S: f64 = 3600.0;

impl Simulator {
    /// Remember the direction now. A sample at the newest sample's instant replaces it;
    /// samples older than `HISTORY_S` are dropped.
    pub(crate) fn remember(&self);
    /// Forget every sample: the telescope was moved by hand.
    pub(crate) fn forget(&self);
    /// The camera centre's altitude and azimuth at an instant.
    fn direction_at(&self, unix_s: f64) -> (f64, f64);
}
```

- `direction_at(t)`: with a and b two neighbouring samples such that `a.unix_s <= t <=
  b.unix_s` (found with `partition_point` over the samples' instants), and `f = (t - a.unix_s)/(b.unix_s - a.unix_s)` (0 when the two instants are
  equal): `alt = a.alt + f·(b.alt - a.alt)` and `az = (a.az + f·(((b.az - a.az +
  180).rem_euclid(360)) - 180)).rem_euclid(360)`. With no such pair (an empty memory, t
  before the oldest sample or after the newest), the state's `alt` and `az` now.
- 030's `imaging_truth_at(t)` and `imaging_rotation_at(t)` (030 plan.md § The simulated
  imaging camera) take the finder's rotation from `sky::camera_rotation(alt, az, site,
  t)` with `(alt, az) = direction_at(t)` in place of the state's own. Nothing else of
  them changes. `directions_at` (`crates/simulator/src/optics.rs:120`), `telescope` and
  `field_stars` are not touched.
- `remember` is called in `controls::Simulator`'s `deref`
  (`crates/simulator/src/controls.rs:121`), after `follow_mount`. So every read of the
  telescope through the control handle leaves a sample, a still mount included, and a
  read at the clock's now always finds its own sample as the newest.
- `forget` is called by `optics::Simulator::push_to` and `set_optics`
  (`crates/simulator/src/optics.rs:108`, `crates/simulator/src/optics.rs:103`), and in
  `controls::Simulator` by `connect_mount` and `point_eyepiece_at`
  (`crates/simulator/src/controls.rs:35`, `crates/simulator/src/controls.rs:97`).
  `optics::Simulator::point_eyepiece_at` itself does not forget: `follow_mount` calls it
  for every motion of the mount (`crates/simulator/src/controls.rs:54`).

### Count steps (`crates/simulator/src/mount.rs`, US1)

```rust
impl SimMount {
    /// Make each axis's tube follow its drive in whole counts, as a real controller's
    /// reference does in slow motion, or smoothly again. False when built.
    pub fn set_count_steps(&self, on: bool);
    pub fn count_steps(&self) -> bool;
}
```

`set_count_steps` advances the mechanics to now, as `set_slip_per_move` does
(`crates/simulator/src/mount.rs:291`), and stores the flag. `Motor::travel`
(`crates/simulator/src/mount.rs:97`) clamps the tube round `(drive / step).floor() *
step` in place of `drive` when the flag is set, `step` being `360.0 /
f64::from(COUNTS_PER_REVOLUTION)` (`crates/simulator/src/mount.rs:12`). With the flag
false the line is as it stands. `MountConfig` gains no field: tests build it by literal
(`crates/finder/tests/motion/tracking/pi_loop.rs:113`).

### The tracking error and the interface (`crates/finder/src/imaging/steering.rs`, new, US2)

```rust
//! Tracking steered by the subs (036): the tracking error a sub's imaging solve gives,
//! the one interface between the stack and a tracker, and the console's steering.
//! Nothing here knows an axis: sky terms only.

/// How far the sky at the set point is from the target.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct TrackingError {
    /// Console Unix seconds: the sub's mid-exposure.
    pub time: f64,
    /// East (RA times cos Dec) and north, arcseconds.
    pub east_arcsec: f64,
    pub north_arcsec: f64,
}

/// The tracking error of an imaging solve: the sky at `set_point`, a position in that
/// frame's pixels, less `target`, J2000 degrees. `None` when the target has no pixel in
/// the solve, or a number is not finite.
pub fn tracking_error(solve: &ImagingSolve, set_point: (f64, f64), target: (f64, f64),
    time: f64) -> Option<TrackingError>;

/// A set point given in sensor pixels, or the sensor's centre for `None`, in the pixels
/// of a frame that covers `roi` at `binning`.
pub fn set_point_in_frame(sensor: (u32, u32), set_point: Option<(f64, f64)>, roi: Region,
    binning: u32) -> (f64, f64);

/// What a gap moved the telescope by, on the sky, arcseconds.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Stepped { pub east_arcsec: f64, pub north_arcsec: f64 }

pub type GapFuture<'a> = Pin<Box<dyn Future<Output = Stepped> + Send + 'a>>;

/// The one interface between the stack and whatever tracker is running.
pub trait Steered: Send + Sync {
    /// The J2000 place, in degrees, the tracker keeps at the set point.
    fn target(&self) -> Option<(f64, f64)>;
    /// Hand over a tracking error. It changes nothing until the next gap.
    fn steer(&self, error: TrackingError);
    /// The stack is between two exposures. The tracker makes its correction now and
    /// answers once the mount has acknowledged its rates.
    fn gap(&self) -> GapFuture<'_>;
    /// The stack no longer steers.
    fn release(&self);
    /// Whether tracking errors, and not its own corrections, steer it now.
    fn steered_by_subs(&self) -> bool;
    /// First answer: the longest exposure during which the field turns by at most
    /// `turn_rad` about the target, in seconds; `None` when nothing limits it.
    fn longest_exposure_s(&self, _turn_rad: f64) -> Option<f64> { None }
    /// Second answer: whether it is steady enough to expose.
    fn steady(&self) -> bool { true }
    /// Whether it was steady for all of a window of console Unix seconds.
    fn steady_between(&self, _start: f64, _end: f64) -> bool { true }
}
```

`ImagingSolve` and `Region` are 032's (`crates/finder/src/imaging/solver.rs`, 032 US1).

**`tracking_error`.** `p = solve.pixel_of(target.0, target.1)?`; `(dx, dy) =
(set_point.0 - p.0, set_point.1 - p.1)`; `east_arcsec = 3600·(cd[0][0]·dx + cd[0][1]·dy)`
and `north_arcsec = 3600·(cd[1][0]·dx + cd[1][1]·dy)`, `cd` being `solve.cd_deg`. `None`
when `time`, either result or either input pair is not finite. It is the tracker's
convention, the aim point less the target (`crates/finder/src/tracking.rs:44`), with the
set point in the aim point's place. 032's `pixel_of` answers none where the cosine of
the target's distance from the solve's centre is zero or less (032 plan.md § The
tangent plane); this function adds no guard of its own. A target exactly a quarter turn
away sits on that edge, where rounding decides, so US2-S5 uses the far side of the sky.

**`set_point_in_frame`.** With `(sx, sy)` the set point, or `(sensor.0/2, sensor.1/2)`
as `f64` for none: `((sx - roi.x)/binning, (sy - roi.y)/binning)`, all as `f64`.

`crates/finder/src/imaging/mod.rs` gains `pub mod steering;` in its alphabetical place.

### `GLOSSARY.md` (US2)

After the **Tracking** entry (`GLOSSARY.md:89`):

```markdown
**Tracking error**:
How far the sky at the set point is from the target, as east and north arcseconds at a stated time. While a stack runs it comes from each sub's imaging solve and steers the tracker; otherwise the tracker steers on the finder camera's live pointing.
_Avoid_: guide error, guiding, drift (that is how far the target moves during one sub)
```

After the **Imaging angle** entry (`GLOSSARY.md:97`):

```markdown
**Set point**:
Where on the imaging camera's sensor the imaging run wants the target, and where a sub's tracking error is measured from. The sensor's centre unless the run names another place.
_Avoid_: lock position, guide position, aim point (that is a spot in the finder camera's frame)
```

### The tracker (`crates/finder/src/tracking.rs`, US3)

```rust
/// The share of an axis's tracking error taken out in the gap after it (036).
const SUB_STEP_SHARE: f64 = 0.5;
/// The share of the drift rate measured between two subs taken out of the axis's rate.
const SUB_RATE_SHARE: f64 = 0.3;
/// An axis error under this many of that axis's counts moves nothing,
const SUB_DEADBAND_COUNTS: f64 = 2.0;
/// nor one under this many arcseconds, whichever is larger.
const SUB_DEADBAND_FLOOR_ARCSEC: f64 = 1.0;
/// The most one gap moves an axis, in arcseconds of axis angle.
const SUB_STEP_LIMIT_ARCSEC: f64 = 30.0;
/// How long a gap's step takes.
const SUB_STEP_TIME: Duration = Duration::from_millis(500);
/// Gaps in a row with no tracking error before the finder camera steers again, in the
/// gaps, and goes on steering until a tracking error comes.
const STALE_GAPS: u32 = 3;
/// How many of its last gaps the tracker remembers.
const CORRECTIONS: usize = 16;

impl Tracker {
    /// The J2000 place being tracked; `None` when off.
    pub fn target(&self) -> Option<(f64, f64)>;
    pub fn steer(&self, error: TrackingError);
    pub async fn gap(&self) -> Stepped;
    pub fn release(&self);
    pub fn steered_by_subs(&self) -> bool;
}
impl Steered for Tracker { /* each method calls the inherent one of the same name */ }
```

The module's first line (`crates/finder/src/tracking.rs:1`) becomes
`//! Feed-forward tracking, corrected by the finder camera or steered by the subs, with latched safe endings.`

`Control` (`crates/finder/src/tracking.rs:83`) gains:

```rust
/// The mount's counts a turn, azimuth then altitude, from the start.
counts: [u32; 2],
/// The direction each axis's gear is loaded in, as the rate task last left it.
direction: [f64; 2],
/// True from the start until the rate task has sent its first rates. (US6 also sets
/// it for a take-up and the keyhole pass.)
settling: bool,
/// A tracking error handed over and not yet applied.
pending: Option<TrackingError>,
/// Present while the subs steer.
subs: Option<Subs>,
/// The claim lapsed for want of tracking errors, and none has been applied since. A
/// stack still holds the tracker.
lapsed: bool,
/// While the claim has lapsed: the finder camera's correction, held for the next gap.
held: Held,
/// What the last gaps did, oldest first, at most `CORRECTIONS`, whatever steered them.
corrections: VecDeque<Correction>,
/// Gaps waiting to be served, each with the sender of its answer.
gaps: Vec<tokio::sync::oneshot::Sender<Stepped>>,

#[derive(Default)]
struct Subs {
    /// The last error applied: its time and its two axis errors.
    previous: Option<(f64, [f64; 2])>,
    /// Gaps in a row with no new tracking error, not counting the one that claimed.
    idle: u32,
}
/// What 015's proportional and integral terms would have done for the finder camera's
/// pointings since the last gap, on each axis: the step, in arcseconds of axis angle,
/// and the change of the tracking offset, in arcseconds a second.
#[derive(Debug, Default, Clone, Copy)]
struct Held { step: [f64; 2], offset: [f64; 2] }
/// What one gap did: when it ended on the console's clock, the step it made on each
/// axis in arcseconds, and what it added to each axis's tracking offset in arcseconds
/// a second.
struct Correction { time: f64, step: [f64; 2], rate: [f64; 2] }
```

and `Shared` (`crates/finder/src/tracking.rs:94`) gains `wake: tokio::sync::Notify`.
`start_with_resets` puts `counts` and `settling: true` into the `Control` it builds
(`crates/finder/src/tracking.rs:197`); `Control::default()`, which a tracker never
started holds, has `settling` false, and nothing reads it there because its state is
`off`.

**`steer`.** Under the control lock: nothing unless the state is `tracking` and all
three numbers are finite. Then:
- while `subs` is `Some`: nothing when `previous` holds a time not earlier than the
  error's, or `pending` holds one not earlier; otherwise `pending = Some(error)`;
- while `subs` is `None` and `lapsed` is true: nothing when `pending` holds a time not
  earlier than the error's; otherwise `pending = Some(error)`;
- while `subs` is `None` and `lapsed` is false: nothing.

So an error handed over before a stack's first gap or after a release is dropped; of
two handed over between two gaps the later is kept; and after the claim has lapsed the
newest error is kept for the gap that takes the tracker back.

**`gap`.** Under the control lock: when the state is `off`, answer `Stepped::default()`
at once. Otherwise push a new sender onto `gaps`, release the lock, call
`wake.notify_one()` and await the receiver; a dropped sender answers
`Stepped::default()`.

**`release`.** `subs = None`, `pending = None`, `lapsed = false` and `held =
Held::default()`. The tracking offset and the remembered corrections are kept.

**`steered_by_subs`.** `subs.is_some()`.

**The rate task.** In `run` (`crates/finder/src/tracking.rs:736`), the wait at the top of
the loop (`crates/finder/src/tracking.rs:755`) becomes

```rust
tokio::select! {
    biased;
    _ = self.wake.notified() => {
        if let Err(reason) = self.serve_gaps(&target, &mut movement, counts).await {
            return reason;
        }
        continue;
    }
    _ = second.tick() => {}
}
```

so a gap is served between two turns, never inside a take-up or the keyhole pass, and
by the task the guard ends before Stop is sent
(`crates/finder/src/tracking.rs:885`). With no gap ever asked for, the loop is as it
was. `select!` is biased toward the wake, and `start` answers before the task's first
turn (`crates/finder/src/tracking.rs:207`), so a gap asked for the moment `start`
answers is served before any rate has been sent, with `direction` still `[0.0, 0.0]`:
`serve_gaps` step 1 turns that gap away (§ Plan traps 22). Just before the loop that
sends the rates (`crates/finder/src/tracking.rs:868`), the task stores its `direction`
in `control.direction`; right after that loop, at the end of every turn, it sets
`control.settling = false`. The body of that loop, the rule that turns a requested rate
into the one sent (`crates/finder/src/tracking.rs:871`), moves into

```rust
/// The rate to send an axis: its feed-forward rate plus `extra`, in degrees a second,
/// or zero where that would turn the axis against `direction`.
fn commanded(rate: f64, direction: f64, extra: f64) -> f64 {
    let requested = rate + extra;
    if rate.signum() != direction || requested * direction < 0.0 { 0.0 } else { requested }
}
```

which the loop and `serve_gaps` both call. `end` (`crates/finder/src/tracking.rs:326`)
also sets `subs` and `pending` to `None`, `lapsed` to false and `held` to
`Held::default()`, empties `corrections`, and empties `gaps`, which answers every
waiting gap with no step.

**`serve_gaps`** takes every sender from `gaps` and drops those whose receiver has gone
(`is_closed()`: a stack that stopped while it waited); with none left it returns. Then,
once for them all:

1. Under the control lock, if the state is not `tracking`, or `settling` is true (the
   rate task has not sent its first rates), answer every sender `Stepped::default()`
   and return: nothing is claimed and nothing is sent. Otherwise, when `subs` is
   `None`, claim unless `lapsed` is true and `pending` is `None`. To claim: `subs =
   Some(Subs::default())`, `lapsed = false`, `held = Held::default()` (a tracking
   error takes over from what the finder camera's pointings held), `proportional =
   [0.0; 2]`, `proportional_until = 0.0` and `previous_pointing = None`. `pending` is
   left as it is: `None` at a stack's first gap, and the newest error after a lapse.
2. When `subs` is still `None` (the claim has lapsed and no error has come), apply what
   is held: take `held`, leaving `Held::default()`, and for each axis i set `new =
   (tracking_offset[i] + held.offset[i]).clamp(-OFFSET_LIMIT, OFFSET_LIMIT)`,
   `changed[i] = new - tracking_offset[i]`, `tracking_offset[i] = new` and `want[i] =
   held.step[i].clamp(-SUB_STEP_LIMIT_ARCSEC, SUB_STEP_LIMIT_ARCSEC)`; set
   `proportional = [0.0; 2]` and `proportional_until = 0.0`, as the claim does. No
   deadband is applied: 015's own 10″ was applied when the pointing came. Then step 3.
   Otherwise take `pending`. With none, and when this gap did not claim, `idle += 1`;
   at `STALE_GAPS` set `subs = None` and `lapsed = true` (`held` is empty then: the
   steered pointings held nothing). With an error at the time `t1`, `idle = 0` and,
   with `a = axis_error(target, t1, east, north)` (`crates/finder/src/tracking.rs:357`)
   and C the remembered corrections (`control.corrections`), for each axis i:
   - the rate: when `previous` is `Some((t0, a0))` and `dt = t1 - t0 > 0`:
     `between` is the sum of `c.step[i]` over the c in C with `t0 < c.time <= t1`;
     `unseen` is the sum of `c.rate[i] * (c.time.min(t1) - t0)` over the c in C with
     `c.time > t0`; `drift_rate = (a[i] - a0[i] - between + unseen)/dt`; when
     `drift_rate.abs() <= 2.0·OFFSET_LIMIT`, `new = (tracking_offset[i] -
     SUB_RATE_SHARE·drift_rate).clamp(-OFFSET_LIMIT, OFFSET_LIMIT)`
     (`crates/finder/src/tracking.rs:42`), `changed[i] = new - tracking_offset[i]` and
     `tracking_offset[i] = new`; otherwise `changed[i] = 0.0`;
   - the step wanted: `after` is the sum of `c.step[i]` over the c in C with `c.time >
     t1`; `seen = a[i] + after`; `deadband =
     (SUB_DEADBAND_COUNTS·1_296_000.0/f64::from(counts[i])).max(SUB_DEADBAND_FLOOR_ARCSEC)`;
     `want[i] = if seen.abs() >= deadband { (-SUB_STEP_SHARE·seen)
     .clamp(-SUB_STEP_LIMIT_ARCSEC, SUB_STEP_LIMIT_ARCSEC) } else { 0.0 }`.

   `unseen` is what makes a late error honest: a change of the tracking offset made
   after `t0` was not in force for `c.time.min(t1) - t0` of the interval the drift rate
   was measured over, and the axis would have moved that much more with it. With every
   error applied in the gap right after its sub, `after` is nothing and `unseen` is the
   part of the interval before the last change, so each drift rate measured is the rate
   error as it now stands, and 0.3 of it is taken. The corrections are the tracker's,
   not the claim's, so the error that takes a lapsed claim back is judged against the
   steps the held corrections made after its time too.
3. Outside the lock: `rates = self.rates(target, counts)?`
   (`crates/finder/src/tracking.rs:401`). For each axis, `base[i] = commanded(rates[i],
   direction[i], tracking_offset[i]/3600.0)` and `pulsed[i] = commanded(rates[i],
   direction[i], (tracking_offset[i] + want[i]/0.5)/3600.0)`; the step made is `step[i]
   = (pulsed[i] - base[i])·3600.0·0.5`, which is `want[i]` unless the axis may not turn
   that way: a step against the axis's motion is at most the axis held still for the
   half second.
4. If either step is not zero: send both axes `pulsed`, telling `model.motion.moving` as
   the loop does, wait `SUB_STEP_TIME`, and compute `base` again at the time then. Send
   both axes `base`. A failed send returns `link_lost`, as in the loop. While `subs` is
   `Some`, both axes are sent `base` even when nothing changed; when `subs` is `None`
   (the claim has lapsed, at this gap or before), `base` is sent when either axis's
   `changed` is not zero, and nothing is sent otherwise.
5. Under the lock, when an error was applied and `subs` is still `Some`: `previous =
   Some((t1, a))`. Whenever this gap applied an error or a held correction,
   `Correction { time: clock.now(), step, rate: changed }` is pushed onto
   `corrections`, the oldest dropped past `CORRECTIONS`.
6. Answer every sender the step on the sky: with q and the apparent altitude E as
   `axis_error` takes them at the time now, `x = step[0]·cos E`, `y = step[1]`,
   `east = -cos q·x + sin q·y`, `north = sin q·x + cos q·y` (the matrix of `axis_error`
   is its own inverse). `axis_error`'s q and E move into a small private function both
   use.

**`Shared::pointing`** (`crates/finder/src/tracking.rs:256`): the block that sets
`previous_pointing`, `proportional`, `proportional_until` and the tracking offset
(`crates/finder/src/tracking.rs:292` to `crates/finder/src/tracking.rs:313`):

- while `subs` is `Some`, is skipped;
- while `subs` is `None` and `lapsed` is true (US6: and `control.steady`), runs as it
  stands except for its two writes inside the deadband test. Beside its own
  `*proportional = [0.0; 2]` it sets `held.step = [0.0; 2]`; in place of
  `proportional[i] = -PROPORTIONAL_GAIN * axis_error[i] / interval` it sets
  `held.step[i] = -PROPORTIONAL_GAIN * axis_error[i]`, that rate over its interval,
  the distance 015's term would move the axis; and in place of adding to
  `state.tracking_offset[i]`, under the same `elapsed` test
  (`crates/finder/src/tracking.rs:305`), it adds the same amount,
  `-PROPORTIONAL_GAIN * axis_error[i] / INTEGRAL_TIME_S`, to `held.offset[i]`, with no
  clamp (step 2 of `serve_gaps` clamps the sum). So the newest pointing's step replaces
  an older one's, as 015's newest proportional rate replaces the last, and the
  offset's changes add up, as 015's integral does; `proportional` stays zero and the
  tracking offset unchanged until a gap applies them;
- otherwise (no stack holds the tracker), runs whole, as 015 has it.

The cable watch, the keyhole's resume pointing, the samples and `settle` are as they
were in all three cases. The claim set `previous_pointing` to `None` and the skipped
block never sets it, so the first finder pointing after the subs let go or lapse has no
earlier one, whatever pointings came before the claim: it takes 1 s as its interval and
adds nothing to the integral, as a first pointing does. The keyhole's own call of
`pointing` with its resume pointing (`crates/finder/src/tracking.rs:642`) takes the
same path as any other: in US3 it is held while a lapsed claim stands; from US6 the
keyhole pass is a time the tracker is not steady, so it runs whole.

Holding the correction keeps 015's integral gain per second: three pointings between
two gaps 6 s apart add three changes, as 015 adds one every 2 s. The proportional term
acts once a gap instead of once a solve, and 0.6 of the newest error is taken out then.

### The tracker's two answers (`crates/finder/src/tracking.rs`, US5 and US6)

```rust
impl Tracker {
    // US5:
    pub fn longest_exposure_s(&self, turn_rad: f64) -> Option<f64>;
    // US6:
    pub fn steady(&self) -> bool;
    pub fn steady_between(&self, start: f64, end: f64) -> bool;
}
```

and `impl Steered for Tracker` gains the three, each calling the inherent one.

**`longest_exposure_s`** (US5): `None` when off or when `turn_rad` is not a finite
number above 0. Otherwise, with `(alt, az) = self.place(target, clock.now())`
(`crates/finder/src/tracking.rs:345`) and `rate =
ephemeris::altaz_rates_from_alt_az(site.lat_deg, alt, az).field_rotation_deg_per_h
.to_radians()/3600.0` (`crates/finder/src/ephemeris.rs:65`), as the session's readout
takes it (`crates/finder/src/session.rs:142`): `None` when `rate.abs() < 1e-9`, else
`Some(turn_rad/rate.abs())`.

**Steady** (US6). `Control` gains `steady: bool` and `unsteady: VecDeque<(f64,
Option<f64>)>`, the times, on the console's clock, the tracker stopped and started
being steady, the last 64. `settling` is US3's: true from the start until the first
rates are sent; US6 also sets it true at the start of a take-up and of the keyhole
pass. A private `fn mark(&self, control: &mut Control)` computes `now_steady = state ==
"tracking" && !settling && measurement[0].is_none() && measurement[1].is_none()`, and
when it differs from `control.steady`: stores it; on becoming not steady pushes
`(clock.now(), None)`, dropping the oldest past 64, and sets `subs = None`, `pending
= None` and `held = Held::default()` (`lapsed` is left as it is, so a claim dropped by a
take-up is taken back at the next gap, and one that had already lapsed stays lapsed);
on becoming steady sets the last entry's end to `clock.now()`. It is called:
- in `start_with_resets`, with `steady: false` beside US3's `settling: true` in the new
  `Control`, and the list holding `(f64::NEG_INFINITY, None)`: the tracker was not
  steady at any time before it started;
- at the start of `take_up` and of `keyhole`
  (`crates/finder/src/tracking.rs:697`, `crates/finder/src/tracking.rs:551`), after
  setting `settling = true`;
- where the rate task opens a measurement (`crates/finder/src/tracking.rs:829`) and
  where `settle` closes one (`crates/finder/src/tracking.rs:668`);
- at the end of every turn of the rate task, after its rates are sent and US3's
  `settling = false`;
- in `end`, where the state is `off`.

`Control::default()`, which a tracker never started holds, has an empty list and
`steady: false`. `steady()` is `control.steady`. `steady_between(start, end)` is false
when the list is empty, or when any entry `(from, to)` has `from <= end` and
`to.is_none_or(|to| to >= start)`; after tracking ends the last entry is open, so every
window that reaches past the end is false and one wholly before it is judged on the
entries alone. US6 changes the test in `serve_gaps` step 1 from `settling` to
`!control.steady`, which is the same before the first rates and also turns away a gap
served while a measurement is open: it answers no step and claims nothing. US3's
scenarios pass under either test, because the tracker rig waits for the first rates
and none of them opens a measurement (§ Plan traps 22).

US6 also changes the held mode's test in `Shared::pointing` from `lapsed` to `lapsed &&
control.steady`. While the tracker is not steady its gaps are turned away, so a
correction held then would wait as long as the tracker stays not steady; and 015 closes
a first reversal's measurement from its own corrections, so held corrections would keep
it open for good (§ Plan traps 28). Not steady, a pointing therefore runs 015's block
whole even after a lapse; `mark` has forgotten what was held as the tracker stopped
being steady, and once it is steady again the next pointing zeroes `proportional`
before it holds anything, and the next gap zeroes it too (step 2). No exposure is
started while the tracker is not steady, and the sub it crossed is dropped (US6-S4).
US3's scenarios pass under either test, because none of them has a time the tracker
is not steady after its first rates.

### The steering (`crates/finder/src/imaging/steering.rs`, US4; US5, US6 and US8 add to it)

```rust
/// Where a `Steering` finds the running tracker.
pub type Source = Arc<dyn Fn() -> Option<Arc<dyn Steered>> + Send + Sync>;

/// How many of the last exposures the steering remembers.
pub const EXPOSURES_KEPT: usize = 32;

/// The exposure the stack is about to take.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExposureAsk {
    /// The exposure the observer set, and the camera's shortest, in seconds.
    pub wanted_s: f64,
    pub shortest_s: f64,
    /// The sensor's size, and the region and binning of the frame to be taken.
    pub sensor: (u32, u32),
    pub roi: Region,
    pub binning: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Go { pub exposure_s: f64, pub limited: bool }

/// Releases the tracker when dropped.
pub struct Hold { /* Arc<Steering> */ }

pub struct Steering { /* clock, Mutex<state>, the source */ }
impl Steering {
    pub fn new(clock: Arc<dyn Clock>) -> Arc<Self>;
    pub fn set_source(&self, source: Source);
    /// For the life of a stack's taker.
    pub fn hold(self: &Arc<Self>) -> Hold;
    /// In the gap, before every exposure.
    pub async fn before_exposure(&self, ask: ExposureAsk) -> Go;
    /// For every sub the stack accepts, with its imaging solve.
    pub fn accepted(&self, frame: &SolveFrame, solve: &ImagingSolve);
    pub fn release(&self);
    /// The set point in sensor pixels; `None` is the sensor's centre.
    pub fn set_set_point(&self, sensor_px: Option<(f64, f64)>);
    /// `None` until the steering is first used.
    pub fn view(&self) -> Option<SteeringView>;
    // US5:
    pub fn set_smear_px(&self, px: f64) -> Result<(), String>;
    pub fn set_on(&self, on: bool);
    // US6:
    /// For every sub, before it is solved: whether the tracker was steady for it.
    pub fn check(&self, frame: &SolveFrame) -> Verdict;
    /// Whether a `before_exposure` is waiting for the tracker to be steady. 037 reads it.
    pub fn waiting(&self) -> bool;
}

// US6:
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict { Keep, Drop(&'static str) }
/// The reason a sub taken while the tracker was not steady is dropped with.
pub const NOT_STEADY: &str = "not_steady";

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ErrorView { pub time: f64, pub east_arcsec: f64, pub north_arcsec: f64 }

/// Serialized as a number of seconds, or the string `no limit`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(untagged)]
pub enum LimitView { Seconds(f64), Text(&'static str) }

/// The `steering` part of the imaging view. Every `Option` is skipped while `None`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct SteeringView {
    // US4:
    /// `subs`, `tracker` or `none`.
    pub source: &'static str,
    /// Tracking errors handed over since the tracker was last released.
    pub subs: u64,
    pub last: Option<ErrorView>,
    // US5:
    pub smear_px: f64,
    pub on: bool,
    pub exposure_s: Option<f64>,
    pub limited: bool,
    pub longest_exposure_s: Option<LimitView>,
    pub over_limit: bool,
    // US6:
    pub steady: bool,
    pub waiting: bool,
    pub dropped_not_steady: u64,
    // US8:
    pub error_rms_arcsec: Option<f64>, pub error_rms_px: Option<f64>,
    pub drift_arcsec: Option<f64>, pub drift_px: Option<f64>,
}
```

`Clock` is the console's (`crates/finder/src/session.rs:24`). Each story adds only the
fields and items named for it; a field written and never read fails `clippy -D
warnings`. The state behind the lock, a `std::sync::Mutex`, holds: whether the steering
was used; the set point; `subs` and `last`; the tracker `before_exposure` last found,
as a `Weak`; a `VecDeque` of at most `EXPOSURES_KEPT` **exposure records**, newest
last; and, by story, the settings, the exposure held, the counts and the window.

**Lock order.** `Steering`'s lock is a leaf. It is never held while the source closure,
any method of a tracker, or `Imaging` is called: the source reads the session, which
takes the session's own lock, and a tracker's methods take the tracker's. `view`,
`before_exposure`, `accepted`, `check` and `release` each copy what they need under the
lock, release it, make their calls, and take it again only to write what they learned.

An exposure record is

```rust
struct Taken {
    /// The console's clock when `before_exposure` answered.
    go_time: f64,
    /// The tracker found, if any. Weak: the steering never keeps a tracker alive.
    tracker: Option<Weak<dyn Steered>>,
    sensor: (u32, u32),
    // US8: what that tracker's gap answered.
    stepped: Stepped,
}
```

**The record of a frame** is the newest record whose `go_time` is not after the frame's
mid-exposure, `frame.start + frame.exposure_s/2.0`; none when there is no such record.
The mid-exposure is used, not the start, because a frame's `start` is the scope node's
clock mapped to the console's and may fall a few milliseconds either side of the
answer; the middle of an exposure lies half an exposure from both its own answer and
the next.

**`before_exposure`** (US4), with the lock never held across an await:
1. Mark the steering used. `tracker = source()`, or none when no source is set. Keep
   it, weak, as the tracker last found.
2. With none: push `Taken { go_time: clock.now(), tracker: None, sensor: ask.sensor }`
   and answer `Go { exposure_s: ask.wanted_s, limited: false }`.
3. `tracker.gap().await`.
4. Push `Taken { go_time: clock.now(), tracker: Some(Arc::downgrade(&tracker)), sensor:
   ask.sensor }`, dropping the oldest past `EXPOSURES_KEPT`, and answer `Go {
   exposure_s: ask.wanted_s, limited: false }`.

US5 changes step 3 and the answers: with `on` false no `gap` is called; and the answer
is the exposure held (below). US6 puts a wait before step 3 and a check after it
(below). US8 keeps the gap's answer in the record.

**`accepted`** (US4): the record of the frame; with none, nothing. Its tracker, upgraded;
with none, nothing. `target = tracker.target()`; with none, nothing. `set =
set_point_in_frame(record.sensor, set point, frame.roi, frame.binning)`, `error =
tracking_error(solve, set, target, frame.start + frame.exposure_s/2.0)`; with one:
`tracker.steer(error)`, `subs += 1`, `last = error`. US5: with `on` false nothing is
handed over.

**`release`** takes the tracker last found, upgraded, and calls its `release`; then
forgets it, empties the records and sets `subs` to 0 and `last` to `None` (US5: and
forgets the exposure held; US6: and sets `steady` true and `waiting` false; US8: and
empties the window). The tracker's `release` is called after `Steering`'s lock is
released (**Lock order**). `Hold`'s `Drop` calls it. It
works from the tracker last found and not from a record, so that a stack stopped while
its first `before_exposure` waits still lets the tracker go.

**`view`**: `None` until used. The state is copied under the lock and the lock released;
then `source` is `none` when `source()` answers none (or no source is set), `subs` when
that tracker's `steered_by_subs()` is true, else `tracker`, asked each time a view is
made.

**The cap** (US5). The settings are `smear_px` 2.0 and `on` true when built.
`set_smear_px` refuses a value that is not finite, under 0.1 or over 20.0 with exactly
`smear_px must be a number from 0.1 to 20`; both setters mark the steering used. The
state gains `held: Option<(f64, bool)>`: the exposure a stack holds, in seconds, and
whether it was cut when it was decided. In `before_exposure`, after the gap (or at
once, with `on` false or no tracker):

```rust
let longest = match &tracker {
    None => None,
    Some(tracker) => {
        let set = set point, or (sensor.0/2, sensor.1/2) as f64;
        let radius_px = the largest of hypot(cx - set.0, cy - set.1) over the four corners
            (roi.x, roi.y), (roi.x + roi.width, roi.y), (roi.x, roi.y + roi.height),
            (roi.x + roi.width, roi.y + roi.height), divided by binning;
        tracker.longest_exposure_s(smear_px / radius_px)
    }
};
let (exposure, limited) = match held {
    Some(held) => held,                      // decided at this stack's first exposure
    None => {
        let mut exposure = ask.wanted_s;
        if let Some(limit) = longest {
            while exposure > limit && exposure / 2.0 >= ask.shortest_s { exposure /= 2.0; }
        }
        let decided = (exposure, exposure < ask.wanted_s);
        held = Some(decided);
        decided
    }
};
Go { exposure_s: exposure, limited }
```

`limited` is decided with the exposure and held with it: a later call answers both as
they were decided, whatever `wanted_s` it brings, until `release` forgets them (spec
US5-S2: a wanted 10 s against a held 30 s still answers 30 s, limited). The view's
`exposure_s` and `limited` are that answer's; `longest_exposure_s` is
`Seconds(limit)` or `Text("no limit")` from the latest call that found a tracker, absent
before one; and `over_limit` is whether that limit is a number under `exposure_s`.
Halving and not the limit itself, so that a night needs master darks at a few exposures
(034 matches one within 1 ms). Held and not followed, because 035 drops a sub whose
exposure differs from its reference sub's (035 plan.md § The settings test).

**Steady** (US6). `POLL` is 250 ms of tokio time. The state's `steady` is true and its
`waiting` false when the steering is built, and they are that in the view until a wait
changes them. In `before_exposure`, with a tracker:
while `!tracker.steady()`, set `steady` false and `waiting` true in the state, sleep
`POLL`, and read `source()` again (a tracker that ended or was replaced is taken as
found: none sets `steady` true and `waiting` false and answers as step 2). Then the
gap. If `!tracker.steady()` after it, go round again: that gap does not count. Then
`steady` true, `waiting` false, the record and the answer. A `before_exposure` whose
future is dropped while it waits (the stack stopped) leaves `waiting` true in the
state; `release`, which the stack's `Hold` calls as it drops, sets `steady` true and
`waiting` false.
**`check`**: the record of the frame; with none, or one with no tracker, `Keep`. Its
tracker, upgraded; when it no longer exists, or when
`!tracker.steady_between(frame.start, frame.start + frame.exposure_s)`:
`dropped_not_steady += 1` and `Drop(NOT_STEADY)`. Otherwise `Keep`. It asks the tracker
that was running when the exposure began, not the one running now, so a sub fully
exposed before Track was turned off is kept and one that crossed the end is not.
`dropped_not_steady` counts since the console started; `release` does not clear it.

**The readout** (US8). The state keeps the last 10 errors handed over since the last
release, each with its frame's `exposure_s` and its solve's `arcsec_per_px`. With n
errors, n at least 2: `error_rms_arcsec = sqrt(sum(east² + north²)/n)`; for each error k
after the window's first, `drift_k = hypot(east_k - east_{k-1} - step_east, north_k -
north_{k-1} - step_north)·exposure_k/(time_k - time_{k-1})`, the step being the sum of
the `stepped` of the records whose `go_time` lies after `time_{k-1}` and not after
`time_k`; `drift_arcsec = sqrt(sum(drift_k²)/(n - 1))`; and the two `_px` keys are those
divided by the newest error's `arcsec_per_px`. With fewer than 2, all four are `None`.

### The session (`crates/finder/src/session.rs`, US4)

```rust
/// The running tracker as the stack sees it (036): `None` while Track is off, armed or ended.
pub fn steered(&self) -> Option<Arc<dyn crate::imaging::steering::Steered>>;
```

`self.tracker()` (`crates/finder/src/session.rs:773`), kept when its
`state().state != "off"`. 037 asks it `steady()`.

### The imaging state (`crates/finder/src/imaging/mod.rs`, US4)

`Imaging` (031 plan.md § The console's imaging state) gains a field `steering:
Arc<Steering>`, made with `Steering::new(clock.clone())` in `Imaging::start`, and

```rust
/// The console's steering (036). 035's stack and 037's run reach it here.
pub fn steering(&self) -> Arc<steering::Steering>;
```

`ImagingView` gains, after `stack`, the last part 035 gives it,

```rust
#[serde(skip_serializing_if = "Option::is_none")]
pub steering: Option<steering::SteeringView>,   // US4
```

`None` in `ImagingView::off()`. Wherever `Imaging` builds the view it answers, after its
own lock is released and where 035 sets `stack`, it sets `steering` to
`self.steering.view()`, whatever the view's `state`, as 035 does for `stack`.
`Steering`'s lock is never held while `Imaging` is called.

### The route (`crates/finder/src/imaging/routes.rs`, US5)

`PUT /api/imaging/steering`, handled by `put_steering`, taking `SessionWrite` and
`Result<Json<SteeringEdit>, JsonRejection>` through `sim_json`, as 031's `put_settings`
does:

```rust
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SteeringEdit { pub on: Option<bool>, pub smear_px: Option<f64> }
```

Each field uses the `deserialize_with` 031's `SettingsEdit` uses, so a JSON null is an
error and not "absent". Without imaging: 409 `this console takes no imaging frames`.
`smear_px` is checked first, and a refusal answers 400 with `set_smear_px`'s words and
changes nothing, `on` included. Then `on`. It answers `Json(imaging.view())` with 200.
It is not refused while the camera is not `ready`: the two settings are the console's
own, and the view carries `steering` whatever its `state`.

`crates/finder/src/lib.rs` gains, after the last route line earlier specs left
(`crates/finder/src/lib.rs:35` today), written as it stands here:
`* PUT /api/imaging/steering: {"on":true|false,"smear_px":N}, either or both; whether the subs steer the tracker, and the corner smear that caps the exposure.`

`crates/finder/README.md` gains a row in the `## API` table
(`crates/finder/README.md:167`), after the last `/api/imaging` row earlier specs left:

```markdown
| `PUT /api/imaging/steering` | JSON with either or both of `on` (true or false) and `smear_px` (0.1 to 20). Whether the subs steer the tracker during a stack, and how many pixels the field may turn at the corner of a sub in one exposure. Answers 200 with the imaging view. Run-only: 2 px and on at every start. Same-origin writes only. |
```

### The stack (`crates/finder/src/imaging/stack/service.rs`, US7)

035's stack runs two tasks (035 plan.md § The stack on the console): the taker, which
calls `lease.take("sub")` back to back on the camera's lease, which it owns and which
drops when it returns, and the worker, which handles each saved sub in eight steps.
US7 adds to both and to `Dropped`, and nothing else to the file. Each place is put
against 035's code by name and line when the trio is re-anchored (§ Prerequisites 3;
§ What 035 must provide).

**`Dropped`** gains a twelfth field, last, `pub not_steady: u32`, and `total()` adds it.

**The stopping mark** is 035's (035 plan.md § The stack on the console, **The stopping
mark**): one `tokio::sync::watch` channel of `bool`, made with the run, `false` until
the stack is ending and `true` from then on; the taker holds a receiver of it,
`stopping` below. "The stack is stopping" is `*stopping.borrow()`, and the taker's
wait races `stopping.wait_for(|stopping| *stopping)` in a `tokio::select!` that is
`biased` toward it, as 035's seam for 036 says (035 plan.md § Shapes that existing
tests pin, **The seam for 036 and 037**, 1). `wait_for` also completes, with an error,
if the sender has gone; either way the taker returns.

**The taker.** Where the task begins:

```rust
let steering = imaging.steering();
let _hold = steering.hold();          // dropped when the taker's task ends: the tracker is let go
let view = imaging.view();
// `start` refused a camera that was not ready, so both parts are there. If the camera
// has stopped being ready since, there is no ask: the steering is not asked, and the
// first `lease.take` below is refused with 031's words and ends the stack as 035 has it.
let ask = match (&view.camera, &view.settings) {
    (Some(camera), Some(settings)) => Some((settings.exposure_ms, ExposureAsk {
        wanted_s: settings.exposure_ms / 1000.0,
        shortest_s: camera.exposure_ms.min / 1000.0,
        sensor: (camera.width, camera.height),
        roi: Region::from(settings.roi.clone()),
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
of the loop, after "if the stack is stopping: return" and before `lease.take("sub")`:

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
finds the setting as it will stay. No frame is in flight then: the taker returns only
at the top of its loop, from the steering's wait, on a refused `take` or `set`, or
after `imaging.ended` has answered for its last frame. Every way a stack ends passes
that point, because 035's ending waits for the taker to return. US7 adds nothing to
035's ending itself.

**The worker**, for each sub:

- in step 2, before 035's settings test: `if let Verdict::Drop(reason) =
  steering.check(&frame) { … }`, the outcome being `reason`, `not_steady`, counted in
  `dropped.not_steady` and listed like any other outcome. It comes before the solve so
  that a sub smeared by a take-up costs no solve and leaves the stack's own hint alone;
- in step 8, where the sub is counted as stacked: `steering.accepted(&frame, &solve)`,
  with the `SolveFrame` of step 1 and the `ImagingSolve` of step 4.

`LastView::outcome` and `stack.json`'s `subs` carry `not_steady` with no further change:
both write the key of `Dropped` a sub was counted under.

**The guard** (spec US7-S6). The file, as US7 leaves it, holds none of `ephemeris`,
`alt_az`, `parallactic`, `counts_per_revolution`, `AxisInfo` and `MountClient`, the six
names US2-S6 reads `steering.rs` for. Nothing US7 adds names one: the taker and the
worker reach the tracker only through `Steering`, `ExposureAsk` and `Verdict`, in sky
and sensor terms. § Prerequisites 3 checks that 035's landed file holds none either, so
that the guard rests on US7's diff.

### What 035 must provide (035's files; read as landed before the flip)

035's trio is a draft, and so is 031's, which carries the camera's lease as its US9
(031 plan.md § The camera lease; 035 plan.md § Prerequisites 8). This section lists
everything this spec assumes of 035, and through it of 031's lease, so that the
re-anchoring has one list to check (§ Prerequisites 3). 035's Prerequisites 10 and
037's plan.md point here. 035 says that "a sub taken at an exposure that changes within
a stack is not offered" (035 plan.md § Prerequisites 10); this spec does not ask for
one. It changes the setting once, before a stack's first sub.

What US7 and US8 rely on:

1. **The taker's loop** has one place before each `lease.take("sub")`, after its look at
   the stopping mark, and the taker can reach `imaging`, an `Arc<Imaging>`, from which
   `imaging.steering()` comes.
2. **The stopping mark** (035 plan.md § The stack on the console, **The stopping
   mark**): one `tokio::sync::watch` channel of `bool`, made with the run, `false`
   until the stack is ending and `true` from then on, set once by whoever ends the
   stack; the taker and the worker each hold a receiver. The taker's wait races it as
   `stopping.wait_for(|stopping| *stopping)` in a `tokio::select!` that is `biased`
   toward it, and returns when it wins (035 plan.md § Shapes that existing tests pin,
   **The seam for 036 and 037**, 1).
3. **The run's `Lease`**, which 035's `start` moves into the taker and which drops when
   the taker's task ends, with
   `Lease::take(purpose)` as 035 uses it and `Lease::set(SettingsEdit)`, which 035 does
   not call and 033's aid does. Of `set`: with only `exposure_ms` present it stores
   the console's exposure and calls nothing on the scope node; it answers 409 with
   031's words when the camera is not `ready`, 409 `an exposure is in progress` while
   a frame is in flight, and 400 outside the camera's range.
4. **While the lease is out**, `Imaging::set` by anyone else is refused with the
   holder's name, `PUT /api/imaging/settings` included (035 plan.md § The camera's
   lease, 2). Spec.md's edge case on editing the exposure during a stack rests on it.
5. **The taker's return.** 035's ending sets the stopping mark, awaits
   `imaging.abort()` when `stop()` asked, and waits for the taker to return, on every
   way a stack ends; the taker returns only at the top of its loop, from the gap's
   wait, on a refused take, or after `imaging.ended` has answered for its last frame,
   so no frame is in flight as it returns, and the lease drops with it. 035 puts the
   observer's exposure back "where the taker returns and before the lease is dropped"
   (the seam, 2), which is where § The stack does it.
6. **The worker's steps**: a `SolveFrame` built in step 1; the settings test in step 2,
   which drops a sub whose `exposure_ms` differs from the reference sub's by more than
   1 ms; the `ImagingSolve` of step 4; and step 8, "the one place a sub is accepted".
7. **`Dropped`** derives `Default`, writes every key always and in order, and has
   `total()`; `LastView::outcome` and `stack.json`'s `subs` write the key a sub was
   counted under. No test of 035 counts the keys, compares a whole `dropped` object,
   or builds `Dropped { .. }` without `..Default::default()` (035 spec.md § User
   Scenarios: "a test never asserts how many there are").
8. **`StackBlock.tsx`** builds its `Dropped {sum}: {parts}` line from the counts above
   0, in `StackDropped`'s order, each with its words.
9. **`Stacker::start`** refuses a camera that is not `ready`, so the taker's first read
   of the view finds `camera` and `settings`.

What they change in 035's two files, and nothing else:

| File | Change | Story |
|---|---|---|
| `crates/finder/src/imaging/stack/service.rs` | `Dropped` gains `not_steady`, last, and `total()` adds it | US7 |
| `crates/finder/src/imaging/stack/service.rs` | the taker keeps a `Hold`, awaits `before_exposure` before each take raced against the stopping mark, and before its first take sets the exposure through the lease, ending the stack `camera` on a refusal | US7 |
| `crates/finder/src/imaging/stack/service.rs` | the taker's loop runs in one `async` block, and after it the taker puts the observer's exposure back through its lease before the lease drops with the task | US7 |
| `crates/finder/src/imaging/stack/service.rs` | the worker calls `check` before the settings test and `accepted` at step 8 | US7 |
| `crates/finder/web/src/components/StackBlock.tsx` | the `Dropped` line gains `{count} not steady`, last | US8 |

If 031 lands with no lease (the other answer to 035's owner decision 21), the two
`lease.set` calls become `imaging.set`, which then refuses only a frame in flight, a
camera that is not ready and an exposure out of range; and spec.md's edge case on
editing the exposure is rewritten, because 035's settings test would then drop the subs
as `settings`.

### `set_solver` (`crates/finder/src/imaging/solving.rs`, US7)

032's `Imaging::set_solver(solver, session, live)` gains one statement:
`self.steering().set_source(Arc::new(move || session.steered()))`, on a clone of the
session. So `AppState::with_imaging_solver`, the binary and `--simulate` need no change
(032 plan.md § The builders).

### `crates/finder/README.md` (US7)

A section `## Tracking steered by the subs`, after 035's `## Live stack`, saying: that
while a stack runs with Track on, each sub the stack accepts gives a tracking error at
the set point, and that it, not the finder camera, corrects the tracker; that
corrections are made only between exposures, a step of half the error when it is two
counts or more and 0.3 of the drift rate out of the rate; that an error arrives a sub
or more late and is judged against the corrections made since; that the finder camera
still feeds the pointing model; when the tracker's own corrections steer again (the
stack stopped, three gaps in a row with no accepted sub, the tracker not steady), and
that after three such gaps they go on steering until a sub is accepted, applied only in
the stack's gaps, so that no rate changes during an exposure; that turning the switch
off lets the finder camera correct as before, during exposures too; the corner
smear, the halving before a stack's first sub, why the exposure is then held (a
stack's subs share one exposure, and a master dark matches one), that the stack sets
the exposure setting to the exposure in use, that the setting cannot be edited while
the stack holds the camera and is put back as the stack ends, that a stack whose
exposure cannot be set ends with `camera`, and what `over_limit` means; the pause,
when the tracker is not steady (before its first rates, during a take-up, a first
reversal's backlash measurement and the keyhole pass, and after tracking ends) and
the `not_steady` drop; that an axis is never reversed by a correction; the `steering`
part's keys; that both settings are run-only; and that 015's "guiding from the imaging
camera" is no longer out of scope.

### The page (`crates/finder/web/src/`, US8)

`api.ts`, after 035's types:

```ts
export interface ImagingSteering {
  smear_px: number; on: boolean; source: "subs" | "tracker" | "none";
  steady: boolean; waiting: boolean; subs: number; dropped_not_steady: number;
  last?: { time: number; east_arcsec: number; north_arcsec: number };
  exposure_s?: number; limited: boolean; over_limit: boolean;
  longest_exposure_s?: number | "no limit";
  error_rms_arcsec?: number; error_rms_px?: number; drift_arcsec?: number; drift_px?: number;
}
// ImagingView gains: steering?: ImagingSteering;
// 035's StackDropped gains: not_steady?: number;
export async function putImagingSteering(edit: { on: boolean; smear_px: number }): Promise<ImagingView>;
```

`putImagingSteering` is written as 031's `putImagingSettings` is and throws an `Error`
with the answer's `error` on a refusal. Its body's keys are `on` then `smear_px`.
`not_steady` is optional in `StackDropped` so that 035's tests, whose views have eleven
counts, still typecheck.

`components/SteeringBlock.tsx`:

```tsx
export function SteeringBlock({ view, onChange }: { view: ImagingView; onChange?: () => void }) {
  // <section aria-labelledby="steering-title" className="space-y-3">
  //   <h2 id="steering-title" className="section-title">Tracking from the subs</h2>
}
```

Its words, exactly, with `s = view.steering`:
- **The status**, one `role="status"` element, only with `s`: when `s.waiting` and not
  `s.steady`, `Exposures paused: the tracker is not steady`; else for `source` `subs`,
  `Steered by the subs · {subs} subs`; for `tracker`,
  `Steered by the tracker's own corrections`; for `none`,
  `Track is off: nothing steers`.
- **The readout**, a `dl` with existing classes, as `TrackControl` has one
  (`crates/finder/web/src/components/TrackControl.tsx:62`), only with `s`:
  - `Tracking error (RMS, last 10 subs)`: `{error_rms_arcsec.toFixed(1)}″ ·
    {error_rms_px.toFixed(1)} px`, only when both are present;
  - `Drift per sub`: `{drift_arcsec.toFixed(1)}″ · {drift_px.toFixed(1)} px`, likewise;
  - `Exposure`, only with `exposure_s`: `{exposure_s.toFixed(1)} s`, followed when
    `limited` by `, cut to fit the tracker's limit`;
  - `Tracker's limit`, only with `longest_exposure_s`: `{longest_exposure_s.toFixed(1)}
    s`, or `none` for `"no limit"`;
  - `Dropped while not steady`: `{dropped_not_steady}`, only above 0.
- **The warning**, a `role="alert"`, only when `s.over_limit` and `longest_exposure_s`
  is a number, on one line:
  `The exposure is longer than the tracker's limit of {longest_exposure_s.toFixed(1)} s. Stop the stack and start it again to shorten it.`
- **The form**, always: a number field labelled `Corner smear (px)`, from `s.smear_px` or
  2; a checkbox labelled `Steer the tracker from the subs`, from `s.on` or true; and the
  button `Save tracking settings` (`button`), which calls `putImagingSteering({ on,
  smear_px })` and then `onChange`. A refusal shows `role="alert"` with the answer's
  `error` and calls no `onChange`.

Write each phrase a test asserts on one line of JSX. `ImagingPage.tsx` places
`<SteeringBlock view={view} onChange={refresh} />` right after 035's `StackBlock`.

`components/StackBlock.tsx` (035 plan.md § The page): the parts of its `Dropped {sum}:
{parts}` line gain a twelfth, last, `{count} not steady`, from `dropped.not_steady ?? 0`,
which also joins the sum. Nothing else of the block changes.

### `DESIGN.md` and `.impeccable/design.json` (US8)

After 035's entry for its stack block, a plain `###` heading and one paragraph, like its
neighbours (the `>` marks only set the text apart here):

> ### Tracking from the subs — built
> On the Imaging page, after the stack: a section titled Tracking from the subs. One
> status line says what steers the tracker, the subs or the tracker's own corrections,
> or that exposures are paused because the tracker is not steady. A short readout gives
> the tracking error and the drift per sub in arcseconds and pixels, the exposure, and
> the tracker's limit; an alert says when the exposure is longer than that limit. A
> small form sets the corner smear in pixels and whether the subs steer. Text only,
> with existing tokens; at night it is red like every other section.

and in `.impeccable/design.json`'s `components`, formatted as its neighbours:

```json
{"name": "Tracking from the subs", "kind": "custom", "refersTo": "button",
 "description": "Built in SteeringBlock.tsx on the Imaging page after the stack block. One status says what steers the tracker or that exposures are paused; a definition list gives tracking error, drift per sub, exposure and the tracker's limit; a form with a number field, a checkbox and Save tracking settings PUTs /api/imaging/steering. Existing ink, paper and red-only night tokens; no new class.",
 "html": "<section class=\"ds-steering\"><h2>Tracking from the subs</h2><p role=\"status\">Steered by the subs · 12 subs</p><dl><dt>Drift per sub</dt><dd>0.6″ · 0.3 px</dd></dl><button>Save tracking settings</button></section>",
 "css": ".ds-steering{color:var(--color-ink)}.ds-steering button{border:1px solid var(--color-ink);background:transparent;color:inherit;min-height:44px}"}
```

## Shapes that existing tests pin

These are not changed by any story:
- **The tracker with no stack.** `Tracker::new`, `start`, `start_with_resets`, `stop`,
  `state` and `pointing` keep their signatures, and `State` gains no field
  (`crates/finder/src/tracking.rs:58`): 015's tests build and read them
  (`crates/finder/tests/motion/tracking/pi_loop.rs:184`,
  `crates/finder/tests/support/tracker.rs:194`). With no `gap` ever called nothing is
  claimed, `Shared::pointing` runs whole, and every rate is what it was: the tests that
  compare each rate with the feed-forward rate
  (`crates/finder/tests/motion/tracking/pi_loop.rs:267`) and the order of a reversal's
  commands (`crates/finder/tests/support/tracker.rs:285`) pass unchanged.
- **Nothing after a stop.** 015's and 017's tests assert that no rate follows a stop
  (`crates/finder/tests/motion/tracking/tracker.rs`,
  `crates/finder/tests/motion/stop/motion_ends.rs`). Every rate this spec adds is sent
  by the rate task (§ Plan traps 1).
- **`track.readout`.** Its three keys and the samples behind `rms_arcsec` are 015's
  (`crates/finder/tests/motion/tracking/routes.rs:306`); the samples still come from the
  finder camera. This spec's readout is a part of the imaging view.
- **The simulator on a still mount, and at now.** `imaging_truth()` and
  `imaging_truth_at` on a mount that did not move give what 030's, 032's and 035's
  tests read (030 spec.md US2-S2, US2-S6; 032's simulated solver; 035's sim stacking
  rig, whose telescope does not track). `directions_at`, the finder camera's frames and
  `MountConfig`'s four fields are untouched
  (`crates/simulator/tests/rate_and_backlash.rs`).
- **031's pinned settings.** `PUT /api/imaging/settings`, the page's settings body and
  `set_imaging_camera`'s nine keys are 031's (031 spec.md US1-S2, US7, US8-S1). The
  two settings here have their own route.
- **The imaging view without steering.** `steering` is skipped until the steering is
  first used. No test of 030, 031, 032 or 034 starts a stack or calls this spec's route, so
  every body they read is what it was. From US7 on a stack uses the steering, and the
  imaging view of a console that ran one carries `steering`: 035's tests read a view
  key by key (035 spec.md § User Scenarios, Comparing) and are not disturbed.
- **035's stack with no tracker.** 035's rigs do not track. `before_exposure` then
  answers at once with the observer's exposure, `Lease::set` is not called and no
  setting changes, `check` keeps every sub and `accepted` hands nothing over (spec
  US7-S4): every count and every sub of 035's scenarios is what it was, with
  `dropped.not_steady` 0. 035 says `dropped` has eleven keys, each always written, that
  its tests sum whatever keys `dropped` holds, and that "a test never asserts how many
  there are, since a later spec adds reasons" (035 spec.md § User Scenarios); a
  twelfth key at 0 disturbs neither. § Prerequisites 3 checks the landed tests for a comparison of
  the whole object and for a `Dropped` built by a literal without
  `..Default::default()`.
- **The camera's lease and its refusals.** They are 031's, as 035 assumes them (035
  plan.md § The camera's lease). US7 takes no lease and changes no refusal: it calls
  `Lease::set` on the lease the run already holds, twice at most in a stack's life,
  and only when a tracker is running and its limit is under the observer's exposure.
  035's US5-S6, which reads a refusal by the stack's name, is not disturbed.
- **035's `StackBlock` tests.** Their views carry eleven counts; `not_steady` is
  optional in the type and adds nothing to the line when absent or 0 (spec US8-S6).
- **Finder's test layout.** `crates/finder/tests/layout/main.rs` fails a file at the top
  level of `crates/finder/tests` (`crates/finder/tests/layout/main.rs:109`), a file
  added to `support` (`crates/finder/tests/layout/main.rs:127`) and a `main.rs` holding
  anything but its support include and `mod` lines
  (`crates/finder/tests/layout/main.rs:156`).
- **This spec's own shapes, for its later stories and for 037.** `SteeringView` is
  `#[non_exhaustive]` and tests read it key by key. `Steered`'s required methods are
  fixed in US2: the scripted trackers of US4 to US8 implement them, so no later story
  adds a required method, only bodies for the provided ones. `ExposureAsk` and `Go`
  have every field from US4 on, and `before_exposure`, `accepted`, `hold`, `release` and
  `set_set_point` keep their signatures. 037 reads `Steered::steady` through
  `Session::steered`, and `Steering::waiting` or the view's `steering.waiting` (037
  plan.md § Prerequisites 2), and calls `Steering::set_set_point(None)` as a run starts
  its stack (037 plan.md § What 030 to 036 offer).

## Decisions and the alternatives rejected

| Decision | Rejected | Why |
|---|---|---|
| A step in the gap and a rate from the drift between subs | 015's proportional-integral law with other gains | Its proportional term is a rate held until the next solve: a 2″ error is about 1″ of deliberate drift in the next sub. Its integral winds up while a first offset of tens of arcseconds is taken out, and unwinds over several smeared subs. Here the step takes the offset and the rate takes only the drift rate, so the two never work against each other. |
| The rate from the difference of two errors, less the step made between | The rate from the error itself | The difference measures drift whatever the offset; the tracker knows the step it made. |
| No rate change when the drift rate is over twice the offset's limit | Always | A jump that large is a wrong solve, a gust or a bump, not a rate error the offset could hold. |
| A deadband of two counts or 1″ on each axis's step | One figure in arcseconds; one count | A step of under a count cannot be commanded; half of two counts is one. 1″ keeps a fine-geared mount from chasing solve noise. The tracker reads the counts from the mount, so nothing is sized to one. |
| The step as a half-second pulse inside the gap | A proportional rate over the next exposure | Nothing but the feed-forward then changes while the shutter is open, however late the next gap is. |
| Gaps served by the rate task | `gap` sending rates itself, under a lock | The scope node accepts a rate that arrives after a Stop. Only the rate task is ended before Stop is sent (`crates/finder/src/tracking.rs:885`). |
| Freshness counted in gaps | An age in seconds | The plan document's rule 6; a sub may last minutes on another mount. |
| The finder camera changes no rate while the subs steer | A finder error over some bound takes the tracker back | Its scatter is the fault being removed; a real loss of the target shows as subs that do not solve, which hands the tracker back after three. |
| Not steady drops the claim | Steering through a take-up | 015's handling of a reversal and of the keyhole is tested as it stands; the subs come back once it is over. |
| The cap as a turn of the field in radians | Passing pixels and a sensor size to the tracker | The tracker then needs nothing of the camera, and an equatorial tracker answers "no limit" with no arithmetic. |
| Halving the exposure | Following the limit to the millisecond | 034 matches a master dark within 1 ms of the sub's exposure; a few exposures can each have one. |
| A route of this spec's own | Keys of 031's settings | 031's tests pin the settings body and the MCP tool to nine keys. |
| `Steering` as a part of its own, with a source closure | The stack calling the tracker | One place holds the set point, the settings and the readout; a test gives it a scripted tracker; 037 reads it. |
| The simulator remembers its telescope's direction | The simulated solver keeping its own record | The truth is the simulator's to say; 032's adapter and every later test then need no change. |
| Count steps as a setter | A field of `MountConfig` | Tests build `MountConfig` by literal. |
| Each error judged against the gaps since its mid-exposure | Applying each error as if it were measured now | 035's worker solves a sub while the next is exposing, so every error is at least one gap old. Applied as fresh, each offset is stepped out twice and the rate loop runs with a delay, which rings. Remembering the last 16 gaps costs a small list. |
| Only an accepted sub steers | Every solved sub | The plan document's own words. A solve 035 refuses never reaches the mount; the cost, that a sub dropped as soft steers nothing, is spec owner decision 15. |
| `check` before the solve, `accepted` at the worker's step 8 | One call after the solve | A sub taken across a take-up is smeared: solving it costs time and would move the stack's own hint. The tracking error must wait for the stack's verdict. |
| The exposure decided once and held | Following the limit sub by sub | 035 drops a sub whose exposure differs from the reference sub's, and a take exposes for the console's setting. |
| The cap applied by changing the console's exposure setting through the stack's own lease, and put back through it as the stack ends | A second exposure parameter on the take; 034's `Calibration::restore_exposure` | 031's take and its refusals are pinned by 031's tests, and 035's seam for 036 has a stack's exposure set through the taker's lease (035 plan.md § Shapes that existing tests pin, **The seam for 036 and 037**, 2). `restore_exposure` called `Imaging::set`, which the stack's own lease refuses, so the subs would have been taken at the observer's exposure; 034 has since removed it (034 A1). |
| A stack that cannot set its capped exposure ends with `camera` | Taking the subs at the observer's exposure and saying `over_limit` | The view would show the exposure held while longer subs were taken; and a refusal there means the camera is no longer ready. |
| `limited` decided with the exposure and held with it | Comparing the held exposure with each call's wanted one | A later call may ask for less than is held; whether this stack's exposure was cut does not change with what is asked later. |
| A lapsed claim is taken back only once a tracking error has come | Taking the tracker back at the next gap | While subs are dropped, the next-gap rule leaves the finder camera steering during one exposure in four and nothing steering during the other three. This one hands the tracker to its own corrections whole (spec owner decision 6). |
| After a lapse, the finder camera's corrections held for the next gap and applied there | 015's corrections at once, as written | The settled row says rate corrections change only between exposures, and a stack that has lapsed is still exposing: a sub taken then may be accepted and is the one that takes the tracker back. Held, every sub's exposure sees only the feed-forward and the offset. The newest pointing's step and the sum of the integral's changes keep 015's integral gain per second; only the proportional term acts less often, once a gap (spec owner decision 6). |
| Not steady runs 015's corrections whole, even after a lapse | Holding them through a time the tracker is not steady | Gaps are turned away while the tracker is not steady, so a held correction would wait for it to end, and 015 closes a first reversal's measurement from its own corrections, which would then never come (spec owner decision 17). No sub the stack keeps is exposed then. |
| The taker puts the observer's exposure back as it returns | 035's ending, after the taker has returned | The taker owns the lease, which drops as it returns: after that there is no lease to set through, and another holder may already have the camera. 035's seam says the same. |
| A gap before a tracker's first rates is turned away | Serving it like any other | `direction` is not set until the task's first turn, so `commanded` would send both axes zero. From US6 such a tracker is not steady and the same gap claims nothing, so the two stories would disagree about every scenario that began with it. |
| Not steady while a first reversal's backlash is measured | Steady, with the subs steering through it | 015 closes that measurement from the finder camera's pointings and its own corrections; changing what steers during it would need 015's backlash tests proven again (spec owner decision 17). |
| The steering keeps weak references to trackers | Strong ones | Dropping a `Tracker` stops the mount (`crates/finder/src/tracking.rs:112`); a record must not put that off. |
| A sub is matched to its exposure by its mid-exposure time | A ticket carried through 035's queue | It needs no change to 035's hand-over, and half an exposure of margin either side is far more than the clocks' mapping error. |
| `source` says `tracker` | `finder` | The plan document's rule 5: another telescope may have no finder camera. |

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

## Prerequisites before this spec is flipped to `ready`

1. **The earlier specs have landed:** `015-tracking`, `030-imaging-camera-link`, `031-imaging-frames-on-console`,
   `032-imaging-solve-and-aim-point`, `033-focus-reading-and-aid`,
   `034-dark-and-flat-frames` and `035-live-stack`, all in this spec's
   `depends_on_landed`. Not met: 015 has landed; the six others are drafts. 033 is
   named because it writes `imaging/mod.rs`, `imaging/routes.rs`, `lib.rs`, the README
   and the Imaging page before this spec does, and 035 depends on it; nothing of
   033's is called here.
2. **The trio is re-anchored against `main` after they land.** Not met. Every
   `path:line` here was read at 49bb210, in files that exist there. After the six
   land, every name taken from their plan.md is re-read as landed and corrected here
   where it differs (030: `imaging_truth_at`, `imaging_rotation_at` and
   `SimImagingCamera`; 031: `Imaging`, `Imaging::start`, `set`, `view`, `ImagingView`,
   `CameraView`, `SettingsView`, `RoiView`, `FrameView`, `SessionWrite`, `sim_json`,
   `put_settings`, `SettingsEdit`, `Refusal`, `putImagingSettings`, the simulator's
   `set_unresponsive` as 031 US1-S6 uses it, and the camera's lease, `Imaging::lease`,
   `Lease::take` and `Lease::set`, as 031's US9 gives them (031 plan.md § The camera
   lease); 032: `ImagingSolve`,
   `from_cd`, `pixel_of`, `cd_deg`, `SolveFrame`, `Region`, `set_solver`,
   `solve_frame`, `SimulatedSolver`, the solving rig; 034: that a master dark matches
   a sub's exposure within 1 ms; 035: § What 035 must provide); every file they create
   is cited with a line; the story that last wrote each shared file is put right in
   § Refinement Record; and the validator is re-run on `main`.
3. **035 is read as landed, against § What 035 must provide.** Not met. 035's trio was
   read whole when the review of this one was applied: its plan.md with the camera's
   lease, its tasks.md and its Refinement Record. It is still a draft and it is not
   probe-built, and the lease it uses is 031's US9, which 031's draft carries and
   `main` does not yet (035 plan.md § Prerequisites 8). Before the flip each of the
   nine points of § What 035 must provide is read in the landed code, and these are
   settled:
   - the taker's loop, its returns and the worker's eight steps are where § The stack
     puts US7's calls, by name and line;
   - the stopping mark is the `tokio::sync::watch` of `bool` that 035's plan.md names
     (point 2), and the taker holds a receiver of it;
   - the taker owns the run's `Lease` and it drops as the taker's task ends, and the
     holder's `set` behaves as point 3 says. This replaces the first draft's use of
     034's `Calibration::restore_exposure`, which called `Imaging::set`, would be refused
     while the stack holds the camera and has since been removed from 034 (034 A1), and
     the second draft's restore in 035's ending, after the taker had returned and the
     lease with it;
   - 035's landed `crates/finder/src/imaging/stack/service.rs` holds none of the six
     names US2-S6 lists, so that US7-S6 rests on US7's own diff. If it holds one, the
     trio goes back to the owner before any story is dispatched;
   - no test of 035 counts `dropped`'s keys, compares the whole object, or builds
     `Dropped { .. }` by a struct literal without `..Default::default()`. If one does,
     the trio goes back to the owner for an approval that names that test, before any
     story is dispatched: no approval here covers an unnamed test;
   - 035's `StackBlock` builds its `Dropped` line as § The page assumes;
   - the worker still drops a sub whose exposure differs from the reference sub's. If
     035 comes to weight subs by exposure instead, spec owner decision 2's last
     alternative becomes possible and is put to the owner;
   - if 031 landed with no lease, the last paragraph of § What 035 must provide is
     written into § The stack and into spec.md's edge cases.
4. **The owner decisions are settled and the approvals given** (spec § Owner decisions,
   § Owner approvals). Not met: all seventeen decisions are open.
5. **A probe build.** Not met. No story has been compiled. The bounds of US3-S1,
   US3-S2, US4-S5 and US7-S1 come from arithmetic on a model of the loop (§ Refinement
   Record), and US6-S5's rests on 015's own follow staying within 60″ with the finder
   camera's corrections held for gaps 6 s apart before and after the measurement; a
   probe build of US1 to US4 on the simulator confirms or corrects them before the
   flip, one of US6 confirms US6-S5, and one of US7 on top of 035 confirms that the
   stacking rig tracks and stacks at all. US3 and US6 are probe-built together: US6
   changes the test `serve_gaps` step 1 makes and the held mode's test in
   `Shared::pointing`, and US3's test file must pass before and after it, unedited
   (§ Plan traps 22).
6. **Outside the factory, before this spec is used at the telescope:** GoTo and Track
   proven on the real mount (the plan document, § Before any flip, 4), and its altitude
   backlash measured. Not met.
7. **The documents this trio cites are on `main`.** Not met: the plan document,
   ADR-0002, the changed `GLOSSARY.md` and the research note are in the drafting
   worktree, uncommitted. US2 inserts one of its entries after **Imaging angle**, which
   only the rewrite has.
8. **No operator-staged files are needed.** Met.
9. **The plan document agrees with the settled decisions.** Not met on `main`, which
   does not hold the plan document yet (§ Prerequisites 7). In this worktree
   `docs/plans/2026-11-15-specs-030-037-imaging.md` § Interfaces shared across the specs
   already carries this trio's recommended answers: its route table has the row
   `PUT /api/imaging/steering` for 036 (spec owner decision 3); its list of the parts of
   the session's `imaging` object names `steering`, and its module tree
   `crates/finder/src/imaging/steering.rs` (owner decision 14); and its list of MCP
   tools has none for 036, whose `steering` part is read through `get_imaging_camera`
   and the session tools, beside the "Who runs it" row's "Each step also has its own
   route and MCP tool" (owner decision 11). Its § Simulator names what US1 adds, a
   memory of where the telescope pointed and count steps, where its first draft had
   "rate error within an exposure (036)" (spec.md § Clarifications). If the owner
   settles any of those decisions otherwise, the plan document is corrected in the same
   change: the route's row, the part, the module, and the tool or the exemption. Rule 1
   is not edited: it gives its two times a tracker is not steady as today's examples,
   and spec owner decision 17 declares the three further times on its own. 035's Prerequisites 10 does the same for its own departures.
10. **031 carries the camera's lease.** Met in the drafts: 031's trio in this worktree
    carries it as its US9 (031 plan.md § The camera lease), with `Imaging::lease`,
    `Lease::take` and `Lease::set` as this trio names them. Not met on `main`, where 031
    has not landed; 035 tracks it (035 plan.md § Prerequisites 8), and § Prerequisites 2
    and 3 read it as landed. This trio reaches the lease only through § What 035 must
    provide, points 3 to 5.
11. **A bound on the rate one tracking error may ask for.** Met on 2026-11-17, with
    nothing to add here. 032's owner decision on a newly measured aim point while Track
    is on (032's owner decision 25, raised by the audit's finding A4 on 032; 037's
    owner decision 26 cites it) could have taken the answer that the tracker bounds the
    rate one tracking error may ask for, which US3 would then have carried. The owner
    chose the other answer: 032 holds a newly measured aim point back while Track is on
    and uses it once Track is off (032's FR-036). So no bound is written, and nothing in
    this trio changes.
12. **038's followed push.** Added 2026-11-16. Draft `038-track-follows-a-push` makes
    Track follow a push by hand of more than 5′ through `Tracker::follow`, which changes
    the place the tracker holds. If 038 lands first, this trio is re-anchored to it:
    `follow` ends any claim the subs hold on the tracker and forgets any tracking error
    or held correction not yet applied, as a tracker that stops being steady does
    (spec FR-011), with one scenario in US3; `038-track-follows-a-push` joins
    `depends_on_landed`; and the validator is re-run. Without that, a sub's error,
    measured from the old place, would steer the tracker back toward it at up to 30″ a
    gap. Not met: 038 is a draft.

## Plan traps

Each binds every story.

1. **A rate after a Stop.** The scope node drops a move queued before a Stop and accepts
   one that arrives after. `gap` runs in the stack's task. *Check:* `gap` only queues a
   sender and wakes the rate task; `serve_gaps` is called from `run`'s loop alone;
   US3-S7 asserts that a gap after a stop sends nothing.
2. **The simulator's truth on a moving mount.** Asked after the fact, it used the
   direction of now. *Check:* US4 names US1 in its `depends_on_merged`; US4-S5 and US7
   read the truth of past instants and fail without it.
3. **Refraction in the simulator.** The tracker follows the apparent place and the
   simulator draws the geometric sky, 65″ apart at M42's altitude in the rigs. *Check:*
   scenarios on the tracker rig measure against the apparent place; US4-S5's rig and
   the stacking rig start on the catalogue place, so their first tracking error is
   small (§ Plan traps 17 says why it must be).
4. **A step that the axis may not make.** A step against an axis's motion is cut by
   `commanded`. *Check:* the remembered correction keeps the step made, not the step
   wanted, and `Stepped` reports the step made.
5. **The claim gap is not an idle gap.** *Check:* `idle` counts gaps after the claim;
   US3-S6 asserts the claim lapses at the third gap after it, and that a fourth
   leaves it lapsed.
6. **Finder pointings while steered.** Skipping all of `Shared::pointing` would lose the
   cable watch, the keyhole's resume and the backlash measure. *Check:* only the block
   that sets the proportional rate and the offset is skipped; US3-S6 asserts the samples
   still grow, and US3-S8 that the pointing model still takes syncs.
7. **An oracle copied from the code proves nothing.** *Check:* axis parts come from the
   simulator's horizon transform, differentiated by the test; drifts from the
   simulator's truth; the cap from 015's `longest_exposure`; rates from a wrapper round
   the mount.
8. **`SteeringView` grows by story.** *Check:* tests read it key by key; no test
   compares a whole `steering` object.
9. **A named command is read literally.** *Check:* commands are copied from
   § Verification.
10. **One evidence budget.** *Check:* each story keeps its diff under 64 KiB, measured
    with `git diff <base> -- . ':(exclude)crates/finder/web/dist' | wc -c`.
11. **A spec fixed mid-epic needs a relaunch.** *Check:* no builder edits
    `specs/036-tracking-from-subs/` or `.specify/`.
12. **027's layout.** *Check:* each new finder test is the module § Technical Context
    names, declared by one alphabetical `mod` line; no file is added to `support`.
13. **The bundle must be rebuilt.** *Check:* US8 commits `crates/finder/web/dist` after
    `npm run build`.
14. **The lock across an await.** `Steering`'s and the tracker's locks are
    `std::sync::Mutex`. *Check:* neither is held across `gap().await`, a sleep or a
    send.
15. **A tracker kept alive.** Dropping a `Tracker` stops the mount
    (`crates/finder/src/tracking.rs:112`). *Check:* `Steering`'s state holds only `Weak`
    references; the one strong reference is the local of a `before_exposure` call; and
    US6-S4 drops the test's `Arc` and sees the sub dropped.
16. **An error older than the corrections.** 035's worker hands over sub N's tracking
    error while sub N + 1 or later is exposing. *Check:* `serve_gaps` step 2 adds the
    steps made after the error's time and the rate changes it did not see; US3-S9 and
    US3-S10 fail for a tracker that treats the error as fresh, and US3-S1 and US4-S5
    play their subs late.
17. **A step against an axis's motion.** `commanded` sends zero in place of a rate
    against the axis's direction, so such a step is at most the axis's own rate times
    the half second. *Check:* the scenarios that expect a full step hand over errors
    whose step runs with the axis (US3-S4, US3-S5, US3-S9), US3-S5's last case expects
    the cut, and the rigs of US4-S5 and US7 start on the catalogue place so that no
    scenario needs the altitude axis to back up 65″.
18. **A gap nobody waits for.** A stack that stops while `before_exposure` awaits a gap
    drops the future, and the `Hold` releases at once; a gap served afterwards would
    claim a tracker no stack will ever release. *Check:* `serve_gaps` drops senders
    whose receiver has gone before it claims, and `Steering::release` releases the
    tracker last found even when no exposure was recorded.
19. **The exposure setting left changed.** The taker owns the lease, and it drops as
    the taker's task ends. *Check:* the taker's loop runs in one `async` block, and
    after it, on every return, the taker puts the observer's exposure back through its
    lease before the task ends; nothing in 035's ending sets it; US7-S2 reads the
    setting after the stack has stopped.
20. **A sub matched to the wrong exposure.** *Check:* the record of a frame is found by
    the frame's mid-exposure, never its start; US4-S3 hands over a frame from before the
    stack and sees nothing steered.
21. **The simulator's memory needs samples.** The truth of a past instant is right only
    between two reads of the simulator. *Check:* the simulated imaging camera reads it
    at the start and the end of every exposure (030), and US4-S5's test reads it there
    itself, since its rig has no camera running.
22. **A gap before a tracker's first rates.** `start` answers before the rate task's
    first turn, and the task's `select!` serves a waiting gap before its first tick.
    Served like any other, that gap would send both axes zero, since `direction` is
    still `[0.0, 0.0]`; and once US6 lands it would find the tracker not steady and
    claim nothing, so a scenario that began with it would pass in US3 and fail in US6,
    which may not edit US3's test file. *Check:* `serve_gaps` step 1 turns a gap away
    while `settling` is true; the tracker rig waits for the first rates in every
    module that writes it out; US3-S7 asserts what such a gap does, and the assertion
    holds under US3's test and under US6's; US6's development checks run US3's module.
23. **The stack's own lease refuses outsiders.** `Imaging::set` is refused while the
    stack holds the camera, as 034's `Calibration::restore_exposure`, which went through
    it, would have been (034 has since removed it, 034 A1). *Check:* both changes of the exposure are `Lease::set` on the run's lease;
    US7-S2 reads 30000 ms while the stack runs and 60000 ms after it, and fails for a
    set that was refused.
24. **A lapsed claim taken back too soon.** *Check:* `serve_gaps` step 1 claims a
    lapsed tracker only when an error is pending; `steer` keeps the newest error while
    lapsed; `release` and `end` clear `lapsed`, so a new stack's first gap always
    claims; US3-S6 asserts that a fourth idle gap leaves the finder camera steering.
25. **A wait left showing.** A stack stopped while `before_exposure` waits for a steady
    tracker drops that future with `waiting` true in the state, and 037 reads
    `Steering::waiting()`. *Check:* `Steering::release`, which the stack's `Hold` calls
    as it drops, sets `steady` true and `waiting` false; US6-S3 asserts it.
26. **`Steering`'s lock held across a call out.** The source reads the session under
    the session's lock, and a tracker's methods take the tracker's. *Check:* § The
    steering, **Lock order**: no call to the source, a tracker or `Imaging` is made
    with `Steering`'s lock held.
27. **A finder correction inside an exposure after a lapse.** Once three gaps bring no
    tracking error, a stack still holds the tracker and is still exposing. *Check:*
    while `lapsed`, `Shared::pointing` writes the step and the offset change to `held`
    and touches neither `proportional` (beyond zeroing it, as 015 does) nor the
    tracking offset; only `serve_gaps` step 2 applies `held`; US3-S11 asserts that no
    rate between two gaps differs from the feed-forward plus the offset, with a noisy
    finder task running, and US3-S6 that the claim taken back forgets what was held.
28. **A held correction while a measurement is open.** Gaps are turned away while the
    tracker is not steady, and 015 closes a first reversal's measurement from its own
    corrections. Held then, nothing would correct the tracker and the measurement would
    never close, with the stack waiting on it. *Check:* from US6 the held mode's test
    is `lapsed && control.steady`, and `mark` forgets `held` as the tracker stops being
    steady; US6-S5 lapses a claim before a first reversal and sees the backlash
    measured.

## Risks

| Risk | Mitigation |
|---|---|
| The bounds were worked from a model, not run. | § Prerequisites 5: a probe build before the flip. The model leaves a factor of five on the drift bound (0.030″ against 0.15″ at the 12th sub played late). |
| 035 changes before it lands, or 031's lease lands in another shape or not at all. | § What 035 must provide lists the nine points this spec leans on and § Prerequisites 3 reads each as landed; only US7 and one line of US8 touch 035's files, and US1 to US6 are tested without a stack. |
| US7-S5's rig has never run: a scripted tracker given to a console's steering, and the link dropped while its gap waits. | The probe build of US7. The scenario's only timing is 031's 3 s for a lost link against a gap of 10 s. |
| The owner prefers arcseconds for the corner smear (spec owner decision 1). | The steering then needs a scale before a stack's first solve, 032's measured one or the one from the focal length setting; FR-018, US5 and the page's field are redrafted before the flip. |
| The stacking rig of US7 has never run: a tracked simulated mount, live finder solves, a pointing model fitted from syncs, the simulated solver and 035's stack together. | The probe build of US7; US3-S1 and US4-S5 carry the claim of SC-003 on rigs of this spec's own, so a correction to US7-S1's bound does not touch it. |
| A sub dropped as soft carries a good tracking error that is not used, and a drift that makes subs soft is then taken out by the tracker's own corrections only. | Spec owner decision 15; a field check. |
| The limit falls under a long stack's exposure near the meridian. | The readout says so; halving usually leaves margin (30 s held against a limit of 34 s to 39 s in the worked rig). Ending the stack is spec owner decision 2's alternative. |
| A half-second pulse of one count does not move the real tube (stiction, backlash). | The rate then takes it up: an undelivered step reads as a drift, and 0.3 of it goes into the rate. A field check (spec § Assumptions). |
| The real mount's rate commands are coarser than the model. | Field check; the constants are in one place. |
| An error that needs a slow axis to back up leaves slowly. | 015's rule that no correction reverses an axis stands; near a reversal the take-up crosses it. A field check through the meridian. |
| The real solver is slower than a sub, so errors arrive several gaps late. | The tracker's memory of 16 gaps covers 035's queue of four; after three gaps with no error the tracker's own corrections steer. `took_ms` in 035's view is the figure to read. |
| An imaging solve is wrong and 035 accepts it. | One gap moves an axis 30″ at most; a jump changes no rate. |
| Rate updates once a second during an exposure. | They are 015's feed-forward and change by thousandths of an arcsecond a second; unchanged here. A field check. |
| The first reversal on a mount pauses the stack for minutes. | Once per mount: the backlash is kept with the fixed terms. The page says exposures are paused. |
| After a lapse the finder camera corrects once a gap, not once a solve. | The integral's gain per second is 015's, and each gap takes 0.6 of the newest error out. A lapse means subs are being dropped (cloud, a lost field), which the readout shows. A field check with subs dropped. |
| A story's diff nears 64 KiB. | US3 is the largest: about 16 KiB of tracker and 38 KiB of test with its rig. If the probe build measures it over, US3-S9, US3-S10 and the remembered corrections move to a story of their own between US3 and US4, and US3-S11 with the held correction to another. |
| 030 to 035 and 037 write `mod.rs`, `routes.rs`, the page and the README too. | The six land first; 037 names this spec. § Prerequisites 2 re-anchors. |

## Refinement Record

Drafted on 2026-11-15 by the grilling session, from the plan document, a read of
`crates/finder/src/tracking.rs`, 015's trio and tests, the simulator, and the drafts of
030, 031, 032, 034, 035 and 037. A first drafter wrote the trio against `main` at dbc2b5b
before 035 had a draft and was cut off. A second finished it the same day against
`main` at 49bb210: it re-read every cited line there; took 030's, 031's, 032's and 034's
interfaces from their reviewed plan.md; read 035's draft plan.md, which changed US4 to
US7 (two tasks and a queue, so tracking errors arrive late, a sub is checked before it
is solved and steers only once accepted, and the exposure is held for a stack's life);
brought the words in line with the plan document's § Mount-neutral by design; and
corrected two scenarios (US3-S5, US4-S5) whose step would have been cut by the rule
that no correction reverses an axis. A second agent then reviewed the trio, and its
twenty-three findings were each checked against the files, the code and the research
and applied the same day, against 035's revised draft (the camera's lease,
`lease.take`, its tasks.md and its Refinement Record) and 033's. The changes that
reach across stories: the cap is set through the stack's own lease and a refused set
ends the stack (US7, § The stack, § What 035 must provide); the tracker rig waits for
a tracker's first rates and a gap before them is turned away (US3, US6, § Plan traps
22); a lapsed claim is taken back only once a tracking error has come (US3); `limited`
is held with the exposure (US5); and six scenarios whose numbers could not pass as
written were corrected (US2-S5, US5-S1, US5-S2, US5-S6, US7-S1, US8-S4). An audit of
the seven trios the series then had, side by side, followed the same day, and its five findings
were applied (spec.md § Clarifications, the audit's session): after a lapse the
finder camera's corrections are held for the next gap, and while the tracker is not
steady they run as 015 has them (US3-S6, US3-S11, US6-S5, § The tracker, § Plan traps
27 and 28); the stopping mark is named, and the observer's exposure is put back where
the taker returns (§ The stack, § What 035 must provide); rule 1 is no longer to be
edited (§ Prerequisites 9); US7-S6 guards 035's `stack/service.rs`, and owner
decision 1's widening is worked; and a bound on the rate one tracking error may ask
for waited on 032's owner decision, which the owner settled on 2026-11-17 with no bound
(§ Prerequisites 11). Every `path:line` in spec.md
and plan.md was then read again at 49bb210, the worktree's HEAD; none had moved. **No
probe build has been run:** no story has been compiled, and no scenario has been seen
to pass or fail, before the review, after it, or after the audit.

What stands in for a probe, and what it does not show:
- **A model of the loop**, worked by hand for this draft: two axes at M42's rates for
  2026-01-16T02:00Z at 40° N (azimuth 19.9″/s, altitude 3.6″/s), a 1% rate error, 5 s
  subs, errors read at mid-exposure. With each error judged against the gaps since its
  mid-exposure, every drift measured is the rate error as it then stands and 0.3 of it
  is taken, so the drift within a sub falls by 0.7 a sub from the first drift measured:
  0.749″ at first and 0.030″ at the 12th sub when errors arrive a sub late. The spec's
  bound of 0.15″ is that figure with margin. The model has no once-a-second refresh, no
  link, no pulse shape and no real axis resolution, and it was not run as a program.
- **The finder-noise scenario** (US3-S2) rests on arithmetic alone: a 15″ scatter on
  each coordinate every 2 s puts a proportional rate of about 9″/s on each axis for
  most intervals, which is tens of arcseconds in 5 s.
- **The held correction** (US3-S11) was worked by hand from 015's constants and a
  spherical-trigonometry check of M42's place at the rig's start (altitude 43.0°,
  azimuth 161.8°, an hour angle of −13.3°): 30″ east is −39.8″ of azimuth and −7.2″ of
  altitude, so the step is about +23.9″ and +4.3″ and runs with both axes. US6-S5's
  measurement under a lapsed claim mirrors 015's own `measured` helper
  (`crates/finder/tests/support/tracker.rs:323`) and has not been run.
- **The worked numbers** of US2, US4-S4, US5 and US8 were recomputed with a short
  script; the limit of 37.15 s and the 65″ of refraction at the rig's start likewise.
  The review recomputed them independently, and its corrections are in: the turns of
  US5 are now the test's own `hypot` expressions, US3-S10's signs are right, US5-S6's
  due-east instant is worked out (9.7″ of azimuth a second against a 3.2″ window), and
  US8-S4 no longer rests on how a tie rounds.
- **Anchors** were read at 49bb210, read again there after the audit, and checked by
  the validator.

For the probe build and the re-anchoring: every bound named in § Prerequisites 5;
whether `serve_gaps` can be woven into `run` as described without disturbing the
keyhole's `second.reset()`; whether a gap that takes 0.5 s lets the next once-a-second
turn slip past 015 US3-S1's "at least once a second" in any existing test (no existing
test calls `gap`); US3's test file under US6's test in `serve_gaps` step 1 (the tracker
rig waits for the first rates, and US3-S7 pins what a gap before them does, so the two
stories should agree, but only a build of both shows it); whether the wait for the
first rates ever takes more than a few milliseconds of virtual time; US3's test file
under US6's held-mode test, `lapsed && control.steady`; whether US6-S5's follow keeps
015's 60″ bound with the finder camera's corrections held for gaps 6 s apart; US7-S5's
rig; and all of § Prerequisites 3.

Writes come from § Project Structure. Shared writes name the merge edge that orders two
stories of this spec on one file, under the earlier story, or the story that, by the
drafts of 030 to 035, last writes the file before this spec does. A test
file that an earlier spec's draft or an earlier story of this spec creates cannot be
listed under `tests` as existing, because the validator resolves every such path in the
drafting worktree; each is named under `preserved_behavior` with what it must still
assert, and moves to `tests` at the re-anchoring. A file last written on `main` by a
spec outside this spec's `depends_on_landed` (`crates/finder/src/tracking.rs`, by 024;
`crates/simulator/src/controls.rs`) has no entry for that spec, because the validator
refuses one. `crates/finder/tests/motion/main.rs`, last written on `main` by 025, is
written after it by 032's US4 (`mod imaging_track`), and US3 names that story. The
record's `open_questions` are empty because the validator refuses a record that holds any; what
is not established is in § Risks and § Prerequisites 2, 3, 5 and 11, and in spec.md's
owner decisions 2 and 15. Nothing in this record is an owner approval: those are in spec.md,
and flipping to `ready` gives them.

```yaml
version: 1
reviewed_revision: 49bb210fe4860dd98ba24727074e7bd3cc0d7124
stories:
  US1:
    tests:
      - path: crates/simulator/tests/imaging_history.rs
        status: new
        disposition: add
        assertion: "on a mount turning at a commanded 0.02 deg/s with a 0.25 rate error, the imaging truth asked 30 s later for three instants of an exposure equals the truth read at each, the azimuth rose 0.05 deg in 2 s, the centre moved, and the star is elongated by the distance between its true places"
        decision: "spec.md:US1-S1"
      - path: crates/simulator/tests/imaging_history.rs
        status: new
        disposition: add
        assertion: "the truth for an instant between two samples equals a twin simulator's reading at that instant"
        decision: "spec.md:US1-S2"
      - path: crates/simulator/tests/imaging_history.rs
        status: new
        disposition: add
        assertion: "after a push by hand the truth for an earlier instant is the one directions_at gives"
        decision: "spec.md:US1-S3"
      - path: crates/simulator/tests/imaging_history.rs
        status: new
        disposition: add
        assertion: "with no mount the truth for a past instant equals directions_at's eyepiece"
        decision: "spec.md:US1-S4"
      - path: crates/simulator/tests/imaging_history.rs
        status: new
        disposition: add
        assertion: "count steps are off when built, and when on the true azimuth takes 8 to 11 values, each a whole count from the first"
        decision: "spec.md:US1-S5"
      - path: crates/simulator/tests/rate_and_backlash.rs
        status: existing
        disposition: preserve
        assertion: "the simulated mount's rate error and backlash behave as before with count steps off"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/simulator/tests/frame_positions.rs
        status: existing
        disposition: preserve
        assertion: "the finder camera's frames and directions_at are unchanged"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "directions_at, the finder camera's frames and MountConfig's fields are unchanged (spec.md FR-001, FR-002)"
      - "imaging_truth_at on a still mount, and at the clock's now, answers what 030 has it answer"
      - "crates/simulator/tests/imaging.rs, a test file 030 creates, passes unchanged: on a still telescope every simulated imaging frame and truth is what 030 makes (plan.md Shapes that existing tests pin)"
      - "the simulator test files 034 and 035 create (imaging_calibration.rs, imaging_scene.rs) pass unchanged: neither moves a mount during an exposure"
    checks:
      development:
        - "cargo nextest run -p simulator --locked --test imaging_history"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - crates/simulator/src/controls.rs
      - crates/simulator/src/mount.rs
      - crates/simulator/src/optics.rs
      - crates/simulator/tests/imaging_history.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/simulator/src/mount.rs
        other_spec: 015-tracking
        other_story: US2
        disposition: merge_order
        reason: "036 depends_on_landed 015, whose US2 last changed mount.rs"
      - path: crates/simulator/src/optics.rs
        other_spec: 035-live-stack
        other_story: US1
        disposition: merge_order
        reason: "036 depends_on_landed 035, whose US1 last changes optics.rs by its draft, after 030's US2 and 034's US1"
    open_questions: []
  US2:
    tests:
      - path: crates/finder/tests/console/steering_error.rs
        status: new
        disposition: add
        assertion: "the worked solve and target give a tracking error of 12.2934 east and -5.1480 north at the time given"
        decision: "spec.md:US2-S1"
      - path: crates/finder/tests/console/steering_error.rs
        status: new
        disposition: add
        assertion: "a set point 100 px right and 50 px up of the centre gives -120.768 east and 196.383 north"
        decision: "spec.md:US2-S2"
      - path: crates/finder/tests/console/steering_error.rs
        status: new
        disposition: add
        assertion: "the set point in a frame's pixels for the whole sensor, a binned region and a named set point"
        decision: "spec.md:US2-S3"
      - path: crates/finder/tests/console/steering_error.rs
        status: new
        disposition: add
        assertion: "with a rendered star's centroid as the set point the error is nothing, and with the frame's centre it is the star's offset reversed"
        decision: "spec.md:US2-S4"
      - path: crates/finder/tests/console/steering_error.rs
        status: new
        disposition: add
        assertion: "a target at the far side of the sky or a time that is not finite gives none, and a tracker that says nothing limits nothing"
        decision: "spec.md:US2-S5"
      - path: crates/finder/tests/console/steering_error.rs
        status: new
        disposition: add
        assertion: "steering.rs names no alt-az arithmetic and no axis count"
        decision: "spec.md:US2-S6"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and console's main.rs holds only its support include and mod lines, with steering_error added"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "nothing calls the new module yet: every console behaviour is as it was"
      - "the console test modules 031, 032, 034 and 035 create pass unchanged: this story adds pure functions, a trait and one pub mod line"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console steering_error::"
        - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - GLOSSARY.md
      - crates/finder/src/imaging/mod.rs
      - crates/finder/src/imaging/steering.rs
      - crates/finder/tests/console/main.rs
      - crates/finder/tests/console/steering_error.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/imaging/steering.rs
        other_spec: 036-tracking-from-subs
        other_story: US4
        disposition: merge_order
        reason: "US4 depends_on_merged US2, and adds Steering to the module"
      - path: crates/finder/src/imaging/mod.rs
        other_spec: 036-tracking-from-subs
        other_story: US4
        disposition: merge_order
        reason: "US4 depends_on_merged US2, and adds the view's part after US2's mod line"
      - path: crates/finder/tests/console/main.rs
        other_spec: 036-tracking-from-subs
        other_story: US4
        disposition: merge_order
        reason: "US4 depends_on_merged US2, and adds mod steering_gap after mod steering_error"
      - path: crates/finder/src/imaging/mod.rs
        other_spec: 035-live-stack
        other_story: US8
        disposition: merge_order
        reason: "036 depends_on_landed 035, whose US8 last writes mod.rs by its draft"
      - path: crates/finder/tests/console/main.rs
        other_spec: 035-live-stack
        other_story: US8
        disposition: merge_order
        reason: "036 depends_on_landed 035, whose US8 last writes console/main.rs by its draft"
      - path: GLOSSARY.md
        other_spec: 035-live-stack
        other_story: US5
        disposition: merge_order
        reason: "036 depends_on_landed 035, whose US5 last writes GLOSSARY.md by its draft"
    open_questions: []
  US3:
    tests:
      - path: crates/finder/tests/motion/steering.rs
        status: new
        disposition: add
        assertion: "on a mount 1% fast, the drift within a 5 s sub is at least 0.6 arcsec at first and at most 0.15 from the 12th sub on with the subs steering, whether each error is handed over at once or a sub late, the offset reaches the rate error, and without feedback every sub drifts at least 0.6"
        decision: "spec.md:US3-S1"
      - path: crates/finder/tests/motion/steering.rs
        status: new
        disposition: add
        assertion: "with finder pointings 15 arcsec noisy the median drift is above 3 arcsec, and with the subs steering beside the same pointings it is at most 0.15 from the 12th sub on"
        decision: "spec.md:US3-S2"
      - path: crates/finder/tests/motion/steering.rs
        status: new
        disposition: add
        assertion: "an axis error of 1.9 counts moves nothing and one of 2.1 counts is half taken out in a gap of half a second, on that axis alone"
        decision: "spec.md:US3-S3"
      - path: crates/finder/tests/motion/steering.rs
        status: new
        disposition: add
        assertion: "a first error steps and sets no rate, an error where the step left the target sets none, and a drift between two errors takes 0.3 of itself out of each axis's rate"
        decision: "spec.md:US3-S4"
      - path: crates/finder/tests/motion/steering.rs
        status: new
        disposition: add
        assertion: "a jump over twice the offset's limit changes no rate, one gap moves an axis 30 arcsec at most, and a step against the axis's motion is cut to the axis held still for half a second"
        decision: "spec.md:US3-S5"
      - path: crates/finder/tests/motion/steering.rs
        status: new
        disposition: add
        assertion: "finder pointings change no rate while the subs steer and still fill the samples; three idle gaps hand the tracker back and it stays handed back until a tracking error comes; finder pointings after the lapse change no rate when they come; the error that takes it back sets no rate and forgets what was held; a release hands it back at once; a steer that is not finite or comes before any gap changes nothing"
        decision: "spec.md:US3-S6"
      - path: crates/finder/tests/motion/steering.rs
        status: new
        disposition: add
        assertion: "between gaps every rate is the feed-forward plus the offset the gap left, every gap sends both axes a rate, after a stop a gap answers no step and sends nothing, and a gap before a tracker's first rates answers no step, claims nothing and sends no zero"
        decision: "spec.md:US3-S7"
      - path: crates/finder/tests/motion/steering.rs
        status: new
        disposition: add
        assertion: "while the subs steer a tracker started through the session, the pointing model's newest sync still advances and the finder's samples still grow"
        decision: "spec.md:US3-S8"
      - path: crates/finder/tests/motion/steering.rs
        status: new
        disposition: add
        assertion: "an error measured before the last gap's step is stepped by half of what that step left, not by half of itself, and sets no rate"
        decision: "spec.md:US3-S9"
      - path: crates/finder/tests/motion/steering.rs
        status: new
        disposition: add
        assertion: "an error measured before the last gap's rate change takes 0.3 of the drift that change left, so two equal drifts give 1.7 and not 2 times the first change"
        decision: "spec.md:US3-S10"
      - path: crates/finder/tests/motion/steering.rs
        status: new
        disposition: add
        assertion: "after the claim lapses, two finder pointings of 30 arcsec east change no rate when they come, and the next gap steps -0.6 of the newest and adds 015's integral of the second to the offset; with noisy finder pointings no rate between two gaps differs from the feed-forward plus the offset the last gap left"
        decision: "spec.md:US3-S11"
      - path: crates/finder/tests/motion/tracking/pi_loop.rs
        status: existing
        disposition: preserve
        assertion: "015's closed loop on finder pointings, its deadband, its rates and its integral are unchanged when no gap is asked for"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/motion/tracking/tracker.rs
        status: existing
        disposition: preserve
        assertion: "feed-forward rates once a second, the four endings and nothing after a stop are unchanged"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/motion/tracking/backlash.rs
        status: existing
        disposition: preserve
        assertion: "the first reversal's measure and later take-ups are unchanged"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/motion/tracking/keyhole.rs
        status: existing
        disposition: preserve
        assertion: "the keyhole pass and its resume are unchanged"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/motion/tracking/night.rs
        status: existing
        disposition: preserve
        assertion: "an hour of tracking through a transit and a keyhole is unchanged"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "motion's main.rs holds only its support include and mod lines, with steering added"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "a tracker no stack has claimed behaves in every way as 015 has it (spec.md FR-007)"
      - "no rate is sent after tracking ends (spec.md FR-010)"
      - "the live loop's syncs for the pointing model are unchanged"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test motion steering::"
        - "cargo nextest run -p finder --locked --test motion tracking::"
        - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs, and the field check of drift per sub at the telescope"
    writes:
      - crates/finder/src/tracking.rs
      - crates/finder/tests/motion/main.rs
      - crates/finder/tests/motion/steering.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/tracking.rs
        other_spec: 036-tracking-from-subs
        other_story: US5
        disposition: merge_order
        reason: "US5 depends_on_merged US3, and adds longest_exposure_s"
      - path: crates/finder/tests/motion/main.rs
        other_spec: 036-tracking-from-subs
        other_story: US4
        disposition: merge_order
        reason: "US4 depends_on_merged US3, and adds mod steering_gap after mod steering"
      - path: crates/finder/tests/motion/main.rs
        other_spec: 032-imaging-solve-and-aim-point
        other_story: US4
        disposition: merge_order
        reason: "036 depends_on_landed 032, whose US4 last writes motion/main.rs by the drafts, with mod imaging_track"
    open_questions: []
  US4:
    tests:
      - path: crates/finder/tests/console/steering_gap.rs
        status: new
        disposition: add
        assertion: "before_exposure answers only after the tracker's gap, accepted hands over the worked tracking error at mid-exposure, release lets go, and the view's source, subs and last follow"
        decision: "spec.md:US4-S1"
      - path: crates/finder/tests/console/steering_gap.rs
        status: new
        disposition: add
        assertion: "with no tracker, or a tracker with no target, nothing waits and nothing is handed over"
        decision: "spec.md:US4-S2"
      - path: crates/finder/tests/console/steering_gap.rs
        status: new
        disposition: add
        assertion: "a frame from before the stack's exposures steers nothing, dropping the Hold releases, and nothing is handed over after a release"
        decision: "spec.md:US4-S3"
      - path: crates/finder/tests/console/steering_gap.rs
        status: new
        disposition: add
        assertion: "a named set point, and a binned region's frame, give the worked tracking errors"
        decision: "spec.md:US4-S4"
      - path: crates/finder/tests/motion/steering_gap.rs
        status: new
        disposition: add
        assertion: "thirty subs played late through Steering on a mount 1% fast, with solves from the simulator's truth, start within 5 arcsec, end within 3 arcsec and 0.15 of drift from the 14th sub on, with no rate change during an exposure"
        decision: "spec.md:US4-S5"
      - path: crates/finder/tests/motion/steering_gap.rs
        status: new
        disposition: add
        assertion: "Session::steered answers the running tracker with M42's place, and none before Track and after it is turned off"
        decision: "spec.md:US4-S6"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "console's and motion's main.rs hold only their support include and mod lines, with steering_gap added to each"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "the imaging view has no steering part until the steering is first used (spec.md FR-016)"
      - "no stack calls the steering yet: 035's stack behaves as it did"
      - "crates/finder/tests/console/steering_error.rs, US2's test file, passes unchanged: the tracking error, the trait's defaults and the guard on steering.rs's text still hold"
      - "crates/finder/tests/motion/steering.rs, US3's test file, passes unchanged"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console steering_gap::"
        - "cargo nextest run -p finder --locked --test motion steering_gap::"
        - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - crates/finder/src/imaging/mod.rs
      - crates/finder/src/imaging/steering.rs
      - crates/finder/src/session.rs
      - crates/finder/tests/console/main.rs
      - crates/finder/tests/console/steering_gap.rs
      - crates/finder/tests/motion/main.rs
      - crates/finder/tests/motion/steering_gap.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/imaging/steering.rs
        other_spec: 036-tracking-from-subs
        other_story: US5
        disposition: merge_order
        reason: "US5 depends_on_merged US4, and adds the cap and the settings"
      - path: crates/finder/tests/console/main.rs
        other_spec: 036-tracking-from-subs
        other_story: US5
        disposition: merge_order
        reason: "US5 depends_on_merged US4, and adds mod steering_cap before mod steering_error"
      - path: crates/finder/tests/motion/main.rs
        other_spec: 036-tracking-from-subs
        other_story: US5
        disposition: merge_order
        reason: "US5 depends_on_merged US4, and adds mod steering_cap after mod steering"
      - path: crates/finder/src/session.rs
        other_spec: 032-imaging-solve-and-aim-point
        other_story: US4
        disposition: merge_order
        reason: "036 depends_on_landed 032, whose US4 last writes session.rs by the drafts"
    open_questions: []
  US5:
    tests:
      - path: crates/finder/tests/console/steering_cap.rs
        status: new
        disposition: add
        assertion: "the turn asked about is the smear over the farthest corner's distance as the test computes it, for the whole sensor, binning, a region and a named set point, and the exposure is halved to fit the limit but not past the camera's shortest"
        decision: "spec.md:US5-S1"
      - path: crates/finder/tests/console/steering_cap.rs
        status: new
        disposition: add
        assertion: "the exposure first answered, and whether it was cut, are held whatever the limit and the exposure asked for become, over_limit says when the limit is under it, and a release lets the next call decide again"
        decision: "spec.md:US5-S2"
      - path: crates/finder/tests/console/steering_cap.rs
        status: new
        disposition: add
        assertion: "the smear setting changes the turn, and values outside 0.1 to 20 are refused with the stated words"
        decision: "spec.md:US5-S3"
      - path: crates/finder/tests/console/steering_cap.rs
        status: new
        disposition: add
        assertion: "the view carries the smear, the switch, the exposure, whether it was limited or is over the limit, and the limit as seconds or no limit; with the switch off nothing is claimed or handed over while the cap still applies"
        decision: "spec.md:US5-S4"
      - path: crates/finder/tests/console/steering_cap.rs
        status: new
        disposition: add
        assertion: "PUT /api/imaging/steering sets either setting, refuses bad bodies with 400, a foreign origin with 403, another content type with 415 and a console without imaging with 409"
        decision: "spec.md:US5-S5"
      - path: crates/finder/tests/motion/steering_cap.rs
        status: new
        disposition: add
        assertion: "the tracker's longest exposure equals 015's longest_exposure for the equivalent sensor at the same instant, is none at the instant its target is due east, and is a number again 10 s later"
        decision: "spec.md:US5-S6"
      - path: crates/finder/tests/console/rates.rs
        status: existing
        disposition: preserve
        assertion: "015's rates and longest exposure are unchanged"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "console's and motion's main.rs hold only their support include and mod lines, with steering_cap added to each"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "PUT /api/imaging/settings and its nine keys are unchanged (plan.md Shapes that existing tests pin)"
      - "track.readout.longest_exposure_s is still 015's, for 1 px on the session's three numbers"
      - "crates/finder/tests/console/steering_gap.rs, US4's test file, passes unchanged: a scripted tracker that names no limit still answers the wanted exposure, not limited"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console steering_cap::"
        - "cargo nextest run -p finder --locked --test motion steering_cap::"
        - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - crates/finder/README.md
      - crates/finder/src/imaging/routes.rs
      - crates/finder/src/imaging/steering.rs
      - crates/finder/src/lib.rs
      - crates/finder/src/tracking.rs
      - crates/finder/tests/console/main.rs
      - crates/finder/tests/console/steering_cap.rs
      - crates/finder/tests/motion/main.rs
      - crates/finder/tests/motion/steering_cap.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/imaging/steering.rs
        other_spec: 036-tracking-from-subs
        other_story: US6
        disposition: merge_order
        reason: "US6 depends_on_merged US5, and adds the wait and check"
      - path: crates/finder/src/tracking.rs
        other_spec: 036-tracking-from-subs
        other_story: US6
        disposition: merge_order
        reason: "US6 depends_on_merged US5, and adds steady"
      - path: crates/finder/tests/console/main.rs
        other_spec: 036-tracking-from-subs
        other_story: US6
        disposition: merge_order
        reason: "US6 depends_on_merged US5, and adds mod steering_steady"
      - path: crates/finder/tests/motion/main.rs
        other_spec: 036-tracking-from-subs
        other_story: US6
        disposition: merge_order
        reason: "US6 depends_on_merged US5, and adds mod steering_steady"
      - path: crates/finder/README.md
        other_spec: 036-tracking-from-subs
        other_story: US7
        disposition: merge_order
        reason: "US7 depends_on_merged US5, and adds the section"
      - path: crates/finder/README.md
        other_spec: 035-live-stack
        other_story: US10
        disposition: merge_order
        reason: "036 depends_on_landed 035, whose US10 last writes the README by its draft"
      - path: crates/finder/src/imaging/routes.rs
        other_spec: 035-live-stack
        other_story: US8
        disposition: merge_order
        reason: "036 depends_on_landed 035, whose US8 last writes routes.rs by its draft"
      - path: crates/finder/src/lib.rs
        other_spec: 035-live-stack
        other_story: US8
        disposition: merge_order
        reason: "036 depends_on_landed 035, whose US8 last writes lib.rs by its draft"
    open_questions: []
  US6:
    tests:
      - path: crates/finder/tests/motion/steering_steady.rs
        status: new
        disposition: add
        assertion: "through a take-up at transit the tracker is not steady for one unbroken time that holds the take-up's commands, steady_between is false for windows that touch it and true for those that do not, the claim is dropped and a gap asked during it answers after"
        decision: "spec.md:US6-S1"
      - path: crates/finder/tests/motion/steering_steady.rs
        status: new
        disposition: add
        assertion: "the tracker is not steady while a first reversal is measured and before its first rates, where a gap claims nothing, nor for any window before it started or any window that reaches past its stop"
        decision: "spec.md:US6-S2"
      - path: crates/finder/tests/console/steering_steady.rs
        status: new
        disposition: add
        assertion: "before_exposure waits with no gap while the tracker is not steady, shows steady false and waiting true, answers within half a second of it being steady after one gap, waits again for a tracker that stops being steady during its gap, and a wait abandoned with its stack no longer shows"
        decision: "spec.md:US6-S3"
      - path: crates/finder/tests/console/steering_steady.rs
        status: new
        disposition: add
        assertion: "check drops a sub the tracker was not steady for, or whose tracker no longer exists, as not_steady and counts it, asks about the sub's own exposure window, and keeps a sub taken with no tracker"
        decision: "spec.md:US6-S4"
      - path: crates/finder/tests/motion/steering_steady.rs
        status: new
        disposition: add
        assertion: "with the claim lapsed before a first reversal and gaps asked every 6 s, the tracker is not steady while the altitude backlash is measured, the follow keeps 015's 60 arcsec bound, the measured play is 0.3 deg within 30 arcsec, and the tracker is steady again at the end"
        decision: "spec.md:US6-S5"
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
      - "a scripted tracker that is always steady is never waited for (US4's and US5's scenarios)"
      - "crates/finder/tests/motion/steering.rs, US3's test file, passes unchanged: its tracker rig waits for the first rates before a scenario asks for a gap, and the one scenario that asks before them (US3-S7) expects no step and no claim, which is also what a tracker that is not steady answers; and no scenario of it, US3-S6 and US3-S11 with their held corrections among them, has a time the tracker is not steady after its first rates, so the held mode's new test changes nothing there"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test motion steering_steady::"
        - "cargo nextest run -p finder --locked --test console steering_steady::"
        - "cargo nextest run -p finder --locked --test motion steering::"
        - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs, and a run through the meridian at the telescope"
    writes:
      - crates/finder/src/imaging/steering.rs
      - crates/finder/src/tracking.rs
      - crates/finder/tests/console/main.rs
      - crates/finder/tests/console/steering_steady.rs
      - crates/finder/tests/motion/main.rs
      - crates/finder/tests/motion/steering_steady.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/imaging/steering.rs
        other_spec: 036-tracking-from-subs
        other_story: US8
        disposition: merge_order
        reason: "US8 depends_on_merged US6, and adds the readout's figures"
      - path: crates/finder/tests/motion/main.rs
        other_spec: 036-tracking-from-subs
        other_story: US7
        disposition: merge_order
        reason: "US7 depends_on_merged US6, and adds mod steering_stack"
      - path: crates/finder/tests/console/main.rs
        other_spec: 036-tracking-from-subs
        other_story: US8
        disposition: merge_order
        reason: "US8 depends_on_merged US6, and adds mod steering_readout"
    open_questions: []
  US7:
    tests:
      - path: crates/finder/tests/motion/steering_stack.rs
        status: new
        disposition: add
        assertion: "a stack on a tracked mount 1% fast drifts at most 0.15 arcsec a sub from the 14th on, drops nothing as not_steady, lets the tracker go when stopped, and with the switch off and the finder camera's frames stopped hands nothing over and most subs drift more than 0.5"
        decision: "spec.md:US7-S1"
      - path: crates/finder/tests/motion/steering_stack.rs
        status: new
        disposition: add
        assertion: "an exposure setting of 60 s is taken as 30 s under the tracker's limit, set through the stack's own lease, shown as limited, and the setting reads 30 s while the stack runs and 60 s after it"
        decision: "spec.md:US7-S2"
      - path: crates/finder/tests/motion/steering_stack.rs
        status: new
        disposition: add
        assertion: "a sub during which Track was turned off and on is dropped as not_steady, counted in the stack's twelfth count and in the steering's, and the stack's counts still add up"
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
        assertion: "035's stack/service.rs, as this story leaves it, names none of ephemeris, alt_az, parallactic, counts_per_revolution, AxisInfo and MountClient"
        decision: "spec.md:US7-S6"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "motion's main.rs holds only its support include and mod lines, with steering_stack added"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "a stack with Track off takes, solves and stacks its subs as 035 has it: nothing is capped, paused or dropped by the steering (spec.md US7-S4)"
      - "settings.exposure_ms is the observer's own after a stack (US7-S2)"
      - "the test files 035 creates (stack_loop.rs, stack_gate.rs, stack_display.rs, stack_console.rs, the cli and mcp stack modules) pass unchanged: they read dropped key by key and never count its keys, their rigs do not track, and no exposure is changed for them (plan.md Shapes that existing tests pin, Prerequisites 3)"
      - "the camera's lease and its refusals are 031's as 035 assumes them: the stack changes the exposure through the lease it already holds and takes no other (plan.md What 035 must provide)"
      - "crates/finder/tests/console/steering_gap.rs and steering_steady.rs, US4's and US6's test files, pass unchanged: Steering's calls behave as before the stack used them"
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
      - path: crates/finder/src/imaging/solving.rs
        other_spec: 032-imaging-solve-and-aim-point
        other_story: US5
        disposition: merge_order
        reason: "036 depends_on_landed 032, whose US5 last writes solving.rs"
      - path: crates/finder/src/imaging/stack/service.rs
        other_spec: 035-live-stack
        other_story: US8
        disposition: merge_order
        reason: "036 depends_on_landed 035, whose US8 last writes the stack's service by its draft"
    open_questions: []
  US8:
    tests:
      - path: crates/finder/tests/console/steering_readout.rs
        status: new
        disposition: add
        assertion: "three worked tracking errors with one step between give a drift of 0.58583 arcsec and an error of 0.90370 RMS, and their pixels at 2.16 arcsec a pixel, and one error gives no figure"
        decision: "spec.md:US8-S1"
      - path: crates/finder/tests/console/steering_readout.rs
        status: new
        disposition: add
        assertion: "the window is the last ten errors, and a release empties it"
        decision: "spec.md:US8-S2"
      - path: crates/finder/web/src/components/SteeringBlock.test.tsx
        status: new
        disposition: add
        assertion: "the block reads Steered by the subs with the count, the error, the drift, the exposure and the tracker's limit to one decimal, and no alert"
        decision: "spec.md:US8-S3"
      - path: crates/finder/web/src/components/SteeringBlock.test.tsx
        status: new
        disposition: add
        assertion: "a cut exposure, no limit, an exposure over the limit, the tracker's own corrections, Track off, a pause and dropped subs each read their own words, and without steering only the form shows"
        decision: "spec.md:US8-S4"
      - path: crates/finder/web/src/components/SteeringBlock.test.tsx
        status: new
        disposition: add
        assertion: "Save tracking settings PUTs on and smear_px to /api/imaging/steering, and a refusal is an alert with the answer's words"
        decision: "spec.md:US8-S5"
      - path: crates/finder/web/src/components/SteeringBlock.test.tsx
        status: new
        disposition: add
        assertion: "the Imaging page shows the block after the stack block, and the stack block's Dropped line names subs dropped as not steady and reads a view without that count as before"
        decision: "spec.md:US8-S6"
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
      - "StackBlock.test.tsx, 035's test file, passes unchanged: a dropped object without not_steady reads as it did (spec.md US8-S6)"
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
      - path: .impeccable/design.json
        other_spec: 035-live-stack
        other_story: US9
        disposition: merge_order
        reason: "036 depends_on_landed 035, whose US9 last writes it by its draft"
      - path: DESIGN.md
        other_spec: 035-live-stack
        other_story: US9
        disposition: merge_order
        reason: "036 depends_on_landed 035, whose US9 last writes it by its draft"
      - path: crates/finder/web/dist
        other_spec: 035-live-stack
        other_story: US9
        disposition: merge_order
        reason: "036 depends_on_landed 035, whose US9 last writes it by its draft"
      - path: crates/finder/web/src/api.ts
        other_spec: 035-live-stack
        other_story: US9
        disposition: merge_order
        reason: "036 depends_on_landed 035, whose US9 last writes it by its draft"
      - path: crates/finder/web/src/components/ImagingPage.tsx
        other_spec: 035-live-stack
        other_story: US9
        disposition: merge_order
        reason: "036 depends_on_landed 035, whose US9 last writes it by its draft"
      - path: crates/finder/web/src/components/StackBlock.tsx
        other_spec: 035-live-stack
        other_story: US9
        disposition: merge_order
        reason: "036 depends_on_landed 035, whose US9 last writes it by its draft"
    open_questions: []
```
