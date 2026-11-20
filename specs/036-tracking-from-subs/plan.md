# Implementation Plan: Tracking Steered by the Subs

**Branch**: `036-tracking-from-subs` | **Date**: 2026-11-15 | **Spec**: [spec.md](spec.md)

This plan records every design decision for the feature. A story that would change one
escalates rather than re-deciding it.

## Summary

Give the simulator a memory of where its telescope pointed, so the truth of a sub's
mid-exposure is right on a moving mount, and a switch that makes its mount move in whole
counts. Define, in `crates/finder/src/imaging/steering.rs`, the tracking error (the sky
at the set point less the target, from a sub's imaging solve) and `Steered`, the one
interface between the stack and a tracker. Implement `Steered` for 015's tracker: once a
stack claims it, the finder camera's pointings change no rate; in each gap the tracker
takes half of each axis's error out as a half-second pulse when the error is two counts
or more, takes 0.3 of the drift seen between two subs out of the rate, by 0.1″/s a gap
at most and never from a pair of errors with a large step between them, sends both axes
their rates and answers when they are acknowledged. Because 035's worker solves a sub
while the next is exposing, a tracking error arrives a gap or more late: the tracker
remembers what its last gaps did and judges each error against them. After eight gaps
with no tracking error it goes back to its own corrections and stays with them until a
sub is accepted again; while the stack still holds it, the step those corrections ask
for is held for the next gap and applied there, and the rate is left as the subs set it,
so that no rate changes during an exposure whichever steers. Add `Steering`, the
console's part between the two: it calls the gap before every exposure, hands over the
tracking error of each sub the stack accepts, stands back from a stack that begins more
than 3′ from its set point, caps a stack's exposure at the tracker's shortest limit of
the hour ahead for a chosen smear, by halving it before the first sub and never to under
a second. Show the steering in the imaging view as `steering`, and set its four settings,
the switch, the corner smear and the control law's two shares, with
`PUT /api/imaging/steering`. One story of tests only, cut from US3, tests the tracker at
the edges of a claim. `040-stack-waits-for-steady` builds the rest on this spec as
landed: the waits while the tracker is not steady or its limit is low, the drop of a sub
taken across such a time, the wiring into 035's stack, the page's block and the MCP tool.

## Technical Context

**Language/Version**: Rust (MSRV 1.96) for every story.

**Primary Dependencies**: nothing new, and no `Cargo.toml` changes. `finder` already has
`tokio` with `sync` and `time`, `serde` and `axum`; the simulator already has what its
memory needs in `std`.

**Storage**: none. The four settings are run-only.

**Testing**:
- US1: `crates/simulator/tests/imaging_history.rs` (new).
- US2: `crates/finder/tests/console/steering_error.rs` (new).
- US3: `crates/finder/tests/motion/steering.rs` (new).
- US4: `crates/finder/tests/console/steering_gap.rs` and, for US4-S5 and US4-S6,
  `crates/finder/tests/motion/steering_gap.rs` (new).
- US5: `crates/finder/tests/console/steering_cap.rs` and, for US5-S6, US5-S8, US5-S11
  and US5-S14, `crates/finder/tests/motion/steering_cap.rs` (new).
- US9: `crates/finder/tests/motion/steering_claim.rs` (new): tests of US3's tracker.
- Where a finder test goes is 027's rule: `console` calls the console in process and
  moves no mount (`crates/finder/tests/console/main.rs:2`), `motion` drives the
  simulated mount in paused time (`crates/finder/tests/motion/main.rs:1`). Each module
  is declared by one `mod` line, in alphabetical order, in its binary's `main.rs`
  (`crates/finder/tests/layout/main.rs:156`), and writes its own setup:
  `crates/finder/tests/support` takes no new file
  (`crates/finder/tests/layout/main.rs:127`). A module may use what the existing support
  files already offer: `crate::support::goto` (US9-S9, US4-S6) and
  `crate::support::Scratch` (`crates/finder/tests/support/mod.rs:14`). The tracker rig,
  with its wait for the first rates, and the scripted tracker of spec.md are written out
  in each module that uses them, only the parts its own tests use, because an unused
  helper fails `clippy -D warnings` and a story may not edit an earlier story's test
  file.

**Target Platform**: the `finder` console on the console machine. Nothing changes on the
scope node or the scope link.

**Constraints**:
- No change under `crates/scope-node`, `crates/scope-link` or `proto/`; no `unsafe`; no
  dependency.
- No file under `crates/finder/src/imaging/` computes an altitude, an azimuth, a
  parallactic angle or a field rotation, or reads an axis count (the plan document,
  § Mount-neutral by design, rule 1; US2-S6 guards `steering.rs` for thirteen names, as
  040's US7-S6 guards 035's `stack/service.rs`).
- Nothing is sized to one telescope or one sub length (rules 3 and 6): the deadband is
  in the mount's own counts, and the freshness rule, the exposures the steering
  remembers and the gaps the tracker remembers are counted in subs. The times this spec
  fixes, the half-second pulse, the hour ahead a tracker's limit is asked for and the
  1 s an exposure is not halved under, belong to the mount, the gap and the sky and not
  to a sub.
- The imaging path's words are mount-neutral (rule 5): `steering.rs` and the view say
  "the tracker", never "the finder camera", an axis or field rotation.
- The steering takes no exposure and holds no lock on the camera (the plan document,
  § Amendments, 1), and never keeps a tracker alive (§ Plan traps 15).
- Every rate is sent by the tracker's own task (§ Plan traps 1).
- While a stack holds the tracker, no rate changes between two of its gaps but the
  once-a-second feed-forward, whether the subs steer or have lapsed (§ Plan traps 27);
  the switch turned off (owner decision 8) and a stack the steering stands back
  from (spec FR-014) are the two declared exceptions.

**Scale/Scope**: one new module, one route, about 450 lines added to the tracker by US3,
and more by US5, and about 100 to the simulator.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Nothing touches the finder camera's solver, detection or `crates/math-core`. Console work is grounded in the cited sources: the research note for the mount's counts and the finder camera's scatter, 015's own functions for the field rotation, and the simulator's truth. The control law's constants are this spec's own design, named for the owner (owner decisions 4 and 5). |
| II. Never weaken a gate | No existing test, golden, fixture, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS`, `ergane.yaml` or `playwright.config.ts` is edited. New tests are new files; each finder module adds one `mod` line to its binary's `main.rs`, which holds no test (owner approval 9). No existing assertion changes. |
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
├── src/imaging/steering.rs              # US2: new; US4: Steering; US5: the cap, the settings
├── src/imaging/mod.rs                   # US2: pub mod steering; US4: the view's part, Imaging::steering; US5: the view's ask
├── src/imaging/routes.rs                # US5: PUT /api/imaging/steering
├── src/tracking.rs                      # US3: Steered for Tracker; US5: the two limits, tune
├── src/session.rs                       # US4: Session::steered
├── src/lib.rs                           # US5: the route's doc line
├── README.md                            # US5: the route's row
├── tests/console/main.rs                # US2, US4, US5: one mod line each
├── tests/console/steering_error.rs      # US2: new
├── tests/console/steering_gap.rs        # US4: new
├── tests/console/steering_cap.rs        # US5: new
├── tests/motion/main.rs                 # US3, US4, US5, US9: one mod line each
├── tests/motion/steering.rs             # US3: new
├── tests/motion/steering_claim.rs       # US9: new
├── tests/motion/steering_gap.rs         # US4: new
└── tests/motion/steering_cap.rs         # US5: new
GLOSSARY.md                              # US2: Tracking error, Set point
```

040-stack-waits-for-steady writes after this spec, in `steering.rs`, `tracking.rs`,
`session.rs`, the README and the three test binaries' `main.rs`, and in files this spec
does not touch (its plan.md § Project Structure).

## Interfaces

Coordinates, sky offsets and the worked examples are as spec.md § User Scenarios defines
them. Everything 030 to 035 create is named as their plan.md gives it and is re-read on
`main` before the flip (flip.md § Prerequisites 2 and 3). What this spec uses or changes of
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

/// A gap's answer. It owns only the answer's receiver and borrows nothing of the
/// tracker, so awaiting it keeps no tracker alive.
pub type GapFuture = Pin<Box<dyn Future<Output = Stepped> + Send>>;

/// The one interface between the stack and whatever tracker is running.
pub trait Steered: Send + Sync {
    /// The J2000 place, in degrees, the tracker keeps at the set point.
    fn target(&self) -> Option<(f64, f64)>;
    /// Hand over a tracking error. It changes nothing until the next gap.
    fn steer(&self, error: TrackingError);
    /// The stack is between two exposures. The tracker makes its correction now and
    /// answers once the mount has acknowledged its rates. The gap is asked for when
    /// this is called, not when the future is first polled.
    fn gap(&self) -> GapFuture;
    /// The stack no longer steers.
    fn release(&self);
    /// Whether tracking errors, and not its own corrections, steer it now.
    fn steered_by_subs(&self) -> bool;
    /// First answer: the longest exposure during which the field turns by at most
    /// `turn_rad` about the target, in seconds; `None` when nothing limits it.
    fn longest_exposure_s(&self, _turn_rad: f64) -> Option<f64> { None }
    /// The same for an exposure decided now and held: the shortest that answer will be
    /// over the hour ahead.
    fn longest_exposure_ahead_s(&self, _turn_rad: f64) -> Option<f64> { None }
    /// Second answer: whether it is steady enough to expose.
    fn steady(&self) -> bool { true }
    /// Whether it was steady for all of a window of console Unix seconds.
    fn steady_between(&self, _start: f64, _end: f64) -> bool { true }
    /// Why it is not steady, in one word of its own for the page; `None` while it is
    /// steady or has nothing to say.
    fn pause(&self) -> Option<&'static str> { None }
    /// The share of a tracking error to take out as a step, and of a drift rate to take
    /// out of the rate, each from 0 to 1. A tracker with no such law ignores them.
    fn tune(&self, _step_share: f64, _rate_share: f64) {}
}
```

`ImagingSolve` and `Region` are 032's (`crates/finder/src/imaging/solver.rs`, 032 US1).
The trait has five required methods and six provided ones, all of them written by US2:
later stories give the tracker bodies for the provided ones (US5 the two limits and
`tune`, 040's US6 the rest) and add none, so the scripted trackers of US4 and US5, and of
040's US6 to US8, never need a method they did not have. No doc comment in this file names an axis, the sky's turning
or the finder camera: US2-S6 reads its text.

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
/// The share of an axis's tracking error taken out in the gap after it (036), until
/// the steering tunes it (US5).
const SUB_STEP_SHARE: f64 = 0.5;
/// The share of the drift rate measured between two subs taken out of the axis's rate,
/// until the steering tunes it (US5).
const SUB_RATE_SHARE: f64 = 0.3;
/// The most one gap changes an axis's tracking offset, in arcseconds a second.
const SUB_RATE_STEP: f64 = 0.1;
/// An axis error under this many of that axis's counts moves nothing,
const SUB_DEADBAND_COUNTS: f64 = 2.0;
/// nor one under this many arcseconds on the sky, whichever is larger.
const SUB_DEADBAND_FLOOR_ARCSEC: f64 = 1.0;
/// The most one gap moves an axis, in arcseconds on the sky.
const SUB_STEP_LIMIT_ARCSEC: f64 = 30.0;
/// How long a gap's step takes.
const SUB_STEP_TIME: Duration = Duration::from_millis(500);
/// Gaps in a row with no tracking error before the finder camera steers again, in the
/// gaps, and goes on steering until a tracking error comes: three, 035's queue of four
/// waiting subs and the one in work, so that an error that is only late does not lapse
/// the claim.
const STALE_GAPS: u32 = 8;
/// How many of its last gaps the tracker remembers.
const CORRECTIONS: usize = 16;

impl Tracker {
    /// The J2000 place being tracked; `None` when off.
    pub fn target(&self) -> Option<(f64, f64)>;
    pub fn steer(&self, error: TrackingError);
    pub fn gap(&self) -> GapFuture;
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
/// True from the start until the rate task has sent its first rates. (040's US6 also sets
/// it for a take-up and the keyhole pass.)
settling: bool,
/// A tracking error handed over and not yet applied.
pending: Option<TrackingError>,
/// A tracking error steers only when its time is later than this console time: the
/// last `follow` (from 040's US6 also the last time the tracker stopped being steady). 0.0
/// when built.
fresh_after: f64,
/// Present while the subs steer.
subs: Option<Subs>,
/// The claim lapsed for want of tracking errors, and none has been applied since. A
/// stack still holds the tracker.
lapsed: bool,
/// While the claim has lapsed: the step the finder camera's newest pointing asks for,
/// held for the next gap.
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
/// What 015's proportional term would have done for the finder camera's newest
/// pointing since the last gap, on each axis: the step, in arcseconds of axis angle.
/// 015's integral is not held: the tracking offset stays as the subs left it.
#[derive(Debug, Default, Clone, Copy)]
struct Held { step: [f64; 2] }
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

**`steer`.** Under the control lock: nothing unless the state is `tracking`, all three
numbers are finite and the error's time is later than `fresh_after` (040's US6: and
`control.steady` is true). Then:
- while `subs` is `Some`: nothing when `previous` holds a time not earlier than the
  error's, or `pending` holds one not earlier; otherwise `pending = Some(error)`;
- while `subs` is `None` and `lapsed` is true: nothing when `pending` holds a time not
  earlier than the error's; otherwise `pending = Some(error)`;
- while `subs` is `None` and `lapsed` is false: nothing.

So an error handed over before a stack's first gap or after a release is dropped; of two
handed over between two gaps the later is kept; and after the claim has lapsed the
newest error is kept for the gap that takes the tracker back. An error measured at or
before the last `follow` is dropped whenever it is handed over: its sub was taken at the
old place, the steering judges it against the target as it is when the worker hands it
over, which is the new place, and 035's worker runs a sub or more behind and keeps four
waiting, so such an error can arrive after the stack's next gap has claimed again (EE2).

**`gap`.** `pub fn gap(&self) -> GapFuture`. Under the control lock: when the state is
`off`, the future it returns answers `Stepped::default()` at once. Otherwise it pushes a
new sender onto `gaps`, releases the lock and calls `wake.notify_one()`, all before it
returns; the future owns only the receiver and awaits it, and a dropped sender answers
`Stepped::default()`. So whoever asks may let go of its `Arc` of the tracker while it
waits (§ The steering, **`before_exposure`**).

**`release`.** `subs = None`, `pending = None`, `lapsed = false` and
`held = Held::default()`. The tracking offset, the remembered corrections and
`fresh_after` are kept.

**`follow`.** 038's `Tracker::follow`, after it clears the samples, also sets
`subs = None`, `pending = None`, `lapsed = false` and `held = Held::default()`, as
`release` does, and `fresh_after = clock.now()`, under a two-line comment: a tracking
error measured from the old place, or a step held for it, would otherwise steer the
tracker back toward that place at up to 30″ a gap (spec FR-011, US9-S12, US9-S13).
`lapsed` is cleared as `release` clears it, so the stack's next gap claims again. From
040's US6 `follow` also clears `pushed` and calls `mark` (040's plan.md § Steady, the tracker's second answer, **A push by
hand**). The
push it ends was by then a time the tracker was not steady, so no exposure the stack
keeps is in flight at a `follow`: the stack is waiting, and a sub that began before the
push is dropped as `not_steady`. Until the stack's next gap claims, the finder camera's
pointings correct the tracker as 015 has them. The rest of `follow` is 038's.

**`steered_by_subs`.** `subs.is_some()`.

**The rate task.** In `run` (`crates/finder/src/tracking.rs:736`), the wait at the top of
the loop (`crates/finder/src/tracking.rs:755`) becomes

```rust
tokio::select! {
    biased;
    _ = self.wake.notified() => {
        if let Err(reason) = self.serve_gaps(&mut movement).await {
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
was. `serve_gaps` reads the target and the counts from `Control` under the lock in its
step 1: at the top of the loop the turn has not read the target, and 038's `follow` may
have changed it since the last turn. `select!` is biased toward the wake, and `start`
answers before the task's first turn (`crates/finder/src/tracking.rs:207`), so a gap
asked for the moment `start` answers is served before any rate has been sent, with
`direction` still `[0.0, 0.0]`: `serve_gaps` step 1 turns that gap away (§ Plan traps
22). Just before the loop that sends the rates (`crates/finder/src/tracking.rs:868`),
the task stores its `direction` in `control.direction`; right after that loop, at the
end of every turn, it sets `control.settling = false`. The body of that loop, the rule
that turns a requested rate into the one sent (`crates/finder/src/tracking.rs:871`),
moves into

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
waiting gap with no step: one that waits in the queue behind a take-up or the keyhole
pass is answered by nothing else, and a stack whose gap was never answered would wait
for ever (040's US6-S10).

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
2. For each axis i, with `s = [cos E, 1.0]`, the sky angle one arcsecond of that axis
   makes at the target, E being the target's apparent altitude at the time now, from the
   private function `axis_error` uses (step 6): `deadband[i] =
   (SUB_DEADBAND_COUNTS·1_296_000.0/f64::from(counts[i])).max(SUB_DEADBAND_FLOOR_ARCSEC/s[i])`
   and `limit[i] = SUB_STEP_LIMIT_ARCSEC/s[i]`, both in arcseconds of axis angle. The 1″
   and the 30″ are angles on the sky: an azimuth angle is a sky angle over cos E, so at
   80° of altitude 30″ of azimuth is 5″ of sky and 1″ is under any solve's noise
   (reanalysis FF14); an equatorial axis has the same pole. When `subs` is still `None`
   (the claim has lapsed and no error has come), apply what is held: take `held`,
   leaving `Held::default()`, and for each axis set
   `want[i] = held.step[i].clamp(-limit[i], limit[i])` and `changed[i] = 0.0`: the
   tracking offset is not touched (§ **`Shared::pointing`**). Set
   `proportional = [0.0; 2]` and `proportional_until = 0.0`, as the claim does. No
   deadband is applied: 015's own 10″ was applied when the pointing came. Then step 3.
   Otherwise take `pending`. With none, and when this gap did not claim, `idle += 1`; at
   `STALE_GAPS` set `subs = None` and `lapsed = true` (`held` is empty then: the steered
   pointings held nothing), and this gap sends nothing (step 4). With an error at the
   time `t1`, `idle = 0` and, with `a = axis_error(target, t1, east, north)`
   (`crates/finder/src/tracking.rs:357`) and C the remembered corrections
   (`control.corrections`), for each axis i:
   - the rate: when `previous` is `Some((t0, a0))` and `dt = t1 - t0 > 0`: `between` is
     the sum of `c.step[i]` over the c in C with `t0 < c.time <= t1`; `unseen` is the
     sum of `c.rate[i] * (c.time.min(t1) - t0)` over the c in C with `c.time > t0`;
     `drift_rate = (a[i] - a0[i] - between + unseen)/dt`; when
     `drift_rate.abs() <= 2.0·OFFSET_LIMIT` and `between.abs() <= 2.0·deadband[i]`, `new
     = (tracking_offset[i] - (SUB_RATE_SHARE·drift_rate).clamp(-SUB_RATE_STEP,
     SUB_RATE_STEP)).clamp(-OFFSET_LIMIT, OFFSET_LIMIT)`
     (`crates/finder/src/tracking.rs:42`), `changed[i] = new - tracking_offset[i]` and
     `tracking_offset[i] = new`; otherwise `changed[i] = 0.0`;
   - the step wanted: `after` is the sum of `c.step[i]` over the c in C with
     `c.time > t1`; `seen = a[i] + after`; `want[i] = if seen.abs() >= deadband[i] {
     (-SUB_STEP_SHARE·seen).clamp(-limit[i], limit[i]) } else { 0.0 }`.

   `unseen` is what makes a late error honest: a change of the tracking offset made
   after `t0` was not in force for `c.time.min(t1) - t0` of the interval the drift rate
   was measured over, and the axis would have moved that much more with it. With every
   error applied in the gap right after its sub, `after` is nothing and `unseen` is the
   part of the interval before the last change, so each drift rate measured is the rate
   error as it now stands, and 0.3 of it is taken. The corrections are the tracker's,
   not the claim's, so the error that takes a lapsed claim back is judged against the
   steps the held corrections made after its time too.

   The two further tests on the rate are for a mount that is not the simulator.
   `between` is the step the tracker asked for; where the tube made a little more or
   less, the difference reads as drift, and while a large offset is taken out at 30″ a
   gap a 10% error in the pulse is 0.5″/s of it. So a pair of errors with more than two
   deadbands of steps between them changes no rate on that axis, and the rate is
   measured once the steps are small. And one bad error, a bump or a poor solve that the
   jump test lets through because it is a rate and so grows with the sub length, changes
   the rate by `SUB_RATE_STEP` and no more. In a model of this law a mount that makes
   80% of each step winds the offset to 0.03″/s while 120″ are taken out, where the
   drafted law reached 0.6″/s, and a rate error of 1% is taken out in the same number of
   subs as before (reanalysis AA6, AA7, FF7).

   US3 writes `SUB_STEP_SHARE` and `SUB_RATE_SHARE` into this step as they stand. From
   US5 the step reads the two shares from `Control`, where `tune` puts them (§ The
   tracker's two answers, **Tuning**).
3. Outside the lock: `rates = self.rates(&target, counts)?`, with the target and counts
   step 1 read (`crates/finder/src/tracking.rs:401`). For each axis,
   `base[i] = commanded(rates[i], direction[i], tracking_offset[i]/3600.0)` and
   `pulsed[i] = commanded(rates[i], direction[i], (tracking_offset[i] +
   want[i]/0.5)/3600.0)`; the step planned is `(pulsed[i] - base[i])·3600.0·0.5`, which
   is `want[i]` unless the axis may not turn that way: a step against the axis's motion
   is at most the axis held still for the half second.
4. If either planned step is not zero: send both axes `pulsed`, telling
   `model.motion.moving` as the loop does, wait `SUB_STEP_TIME`, and compute `base`
   again at the time then. Then send both axes `base`, at every gap but the one at which
   the claim lapsed in step 2, whether or not anything changed: while the subs steer and
   after a lapse alike. A failed send returns `link_lost`, as in the loop. The step made
   is `step[i] = (pulsed[i] - base[i])·3600.0·d[i]`, with step 3's `base` and `d[i]` the
   seconds, on tokio's `Instant`, from the acknowledgement of axis i's `pulsed` to that
   of its `base`; it is zero when no pulse was sent. In paused time on the simulated
   link `d[i]` is exactly 0.5, so no scenario's figure depends on it; on a real link a
   pulse lasts the half second and its two commands, and a step booked at a flat 0.5 s
   would leave the rest to be read as drift (reanalysis FF9).

   The gap at which the claim lapses sends nothing: the subs steered until this gap, so
   the mount's last rates hold no proportional term (US9-S4). At every later gap while
   the claim stays lapsed both axes are sent `base`, changed or not, because `Control`
   cannot say what the mount was last sent. From 040's US6 a pointing while the tracker is not
   steady runs 015's block whole even after a lapse, and the next turn sends its
   proportional rate, 0.6 of the pointing's axis error over its interval. A pointing
   that then arrives steady and lapsed zeroes `proportional` and sends nothing, and the
   rate's time can run out before the next turn. Sending `base` only when
   `proportional` is not zero and `proportional_until` not yet reached misses both
   (D25, EE1). 040's US10-S1,
   040's US6-S8 and 040's US10-S9 assert the rate in force at the answer, and US3-S11 that a held
   step's pulse has ended.
5. Under the lock, when an error was applied and `subs` is still `Some`: `previous =
   Some((t1, a))`. Whenever this gap applied an error or a held step, `Correction {
   time: clock.now(), step, rate: changed }` is pushed onto
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
- while `subs` is `None` and `lapsed` is true (040's US6: and `control.steady`), runs as it
  stands except for what it writes inside the deadband test. Beside its own
  `*proportional = [0.0; 2]` it sets `held.step = [0.0; 2]`; in place of
  `proportional[i] = -PROPORTIONAL_GAIN * axis_error[i] / interval` it sets
  `held.step[i] = -PROPORTIONAL_GAIN * axis_error[i]`, that rate over its interval, the
  distance 015's term would move the axis; and it adds nothing to
  `state.tracking_offset[i]` and holds no change of it. So the newest pointing's step
  replaces an older one's, as 015's newest proportional rate replaces the last;
  `proportional` stays zero, and the tracking offset does not change until a sub is
  accepted or the stack lets go;
- otherwise (no stack holds the tracker), runs whole, as 015 has it.

The cable watch, the keyhole's resume pointing, the samples and `settle` are as they
were in all three cases. The claim set `previous_pointing` to `None` and the skipped
block never sets it, so the first finder pointing after the subs let go or lapse has no
earlier one, whatever pointings came before the claim: it takes 1 s as its interval and,
where 015's block runs whole, adds nothing to the integral, as a first pointing does.
The keyhole's own call of `pointing` with its resume pointing
(`crates/finder/src/tracking.rs:642`) takes the same path as any other: in US3 it is
held while a lapsed claim stands; from 040's US6 the keyhole pass is a time the tracker is not
steady, so it runs whole.

The integral is not held: while a stack holds a lapsed tracker the tracking offset stays
as the subs left it. Held and summed, one gap would add the changes of all its
pointings at once, each made for the same uncorrected error, so the loop's gain would
grow with the square of the gap: past about 25 s between gaps it is unstable, and on a
finder scatter of 15″ it moves the offset by 0.2″/s to 0.5″/s a gap, which spoils the
rate the subs had learned and smears the very sub that would take the tracker back
(reanalysis AA1, FF4, from models of this section; a simulator rig with 6 s gaps and
the truth as its finder pointings does not show it). The proportional
term acts once a gap instead of once a solve, and 0.6 of the newest error is taken out
then, between two exposures.

### The tracker's two answers (`crates/finder/src/tracking.rs`, US5; 040's US6 gives the second)

```rust
impl Tracker {
    // US5:
    pub fn longest_exposure_s(&self, turn_rad: f64) -> Option<f64>;
    pub fn longest_exposure_ahead_s(&self, turn_rad: f64) -> Option<f64>;
    pub fn tune(&self, step_share: f64, rate_share: f64);
}
```

and `impl Steered for Tracker` gains these three, each calling the inherent one.

**`longest_exposure_s`** (US5): `None` when off or when `turn_rad` is not a finite
number above 0. Otherwise, with `(alt, az) = self.place(target, clock.now())`
(`crates/finder/src/tracking.rs:345`) and `rate =
ephemeris::altaz_rates_from_alt_az(site.lat_deg, alt, az).field_rotation_deg_per_h
.to_radians()/3600.0` (`crates/finder/src/ephemeris.rs:65`), as the session's readout
takes it (`crates/finder/src/session.rs:142`): `None` when `rate.abs() < 1e-9`, else
`Some(turn_rad/rate.abs())`.

**`longest_exposure_ahead_s`** (US5): the same answer for the worst moment of the hour
ahead.

```rust
/// How far ahead a limit is asked for an exposure that will be held, and how often it
/// is sampled: an hour, each minute, as the keyhole's forecast samples its window.
const LIMIT_AHEAD_S: f64 = 3600.0;
const LIMIT_SAMPLE_S: f64 = 60.0;
```

`None` when off or when `turn_rad` is not a finite number above 0. Otherwise `rate` as
above at `clock.now() + k·LIMIT_SAMPLE_S` for k from 0 to 60, both ends included, and
with `worst` the largest `rate.abs()` of the 61: `None` when `worst < 1e-9`, else
`Some(turn_rad/worst)`. A stack holds one exposure for its whole life, and before its
transit a target's limit only falls: decided from the limit at the instant of the start,
a stack asked for 30 s on M31 an hour before transit held 7.5 s and trailed 12 px in the
corners at transit (reanalysis FF5; the figures are for the worked example's camera at
35° N and 6 px, by the formula this function uses). An hour is as far as the answer
looks. A stack older than that can still pass its limit: `over_limit` says so, and under
half the exposure held the steering waits (040's plan.md § The steering's wait and its check, **Steady**). The other way
about, a stack started just after the keyhole decides from a limit that then only rises,
and holds a short exposure for good; the view's `would_take_s` shows what a new stack
would hold.

**Tuning** (US5). `Control` gains `step_share: f64` and `rate_share: f64`,
`SUB_STEP_SHARE` and `SUB_RATE_SHARE` in `Control::default()` and in the `Control` that
`start_with_resets` builds. `pub fn tune(&self, step_share: f64, rate_share: f64)`,
under the control lock, stores each number that is finite, clamped to 0.0 to 1.0, and
leaves the other as it was. `serve_gaps` step 2 reads `control.step_share` where US3
wrote `SUB_STEP_SHARE` and `control.rate_share` where it wrote `SUB_RATE_SHARE`; nothing
else of the step changes, and the held step after a lapse is 015's own 0.6 as before. A
share of 0 turns its term off: the rate share at 0 leaves steps alone, which is one of
the comparisons the field wants (reanalysis FF15). The steering calls `tune` before
every gap it asks for (§ The steering, **`before_exposure`**), so a tracker made by a
GoTo or a re-centre has the settings by its first gap; a tracker nobody tunes, as in
US3's scenarios, runs on the two constants.

040's US6 gives the tracker its second answer, whether it is steady enough to expose
(040's plan.md § Steady, the tracker's second answer).

### The steering (`crates/finder/src/imaging/steering.rs`, US4; US5 adds to it, and 040's US6 and US8 add more)

```rust
/// Where a `Steering` finds the running tracker.
pub type Source = Arc<dyn Fn() -> Option<Arc<dyn Steered>> + Send + Sync>;

/// How many of the last exposures the steering remembers.
pub const EXPOSURES_KEPT: usize = 32;
/// A stack whose first tracking error is longer than this, in arcseconds, is not
/// steered: the tracker's own reference and the set point then disagree by too much
/// to steer between them.
pub const FAR_ARCSEC: f64 = 180.0;
// US5:
/// An exposure is not halved to under this many seconds.
pub const HALVING_FLOOR_S: f64 = 1.0;
/// The two shares the steering hands the tracker, until they are set.
pub const STEP_SHARE: f64 = 0.5;
pub const RATE_SHARE: f64 = 0.3;

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
    pub fn set_step_share(&self, share: f64) -> Result<(), String>;
    pub fn set_rate_share(&self, share: f64) -> Result<(), String>;
    /// `view`, for a console whose camera would take `ask` now. With an ask it is also
    /// made, used or not, while a tracker runs.
    pub fn view_for(&self, ask: Option<ExposureAsk>) -> Option<SteeringView>;
}

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
    /// While the steering stands back from a stack that began far from its set point:
    /// the length of that stack's first tracking error.
    pub far_arcsec: Option<f64>,
    /// Whether a stack holds the steering: a `Hold` is alive.
    pub holding: bool,
    // US5:
    pub smear_px: f64,
    pub on: bool,
    pub step_share: f64,
    pub rate_share: f64,
    pub exposure_s: Option<f64>,
    pub limited: bool,
    pub longest_exposure_s: Option<LimitView>,
    pub over_limit: bool,
    /// The exposure a stack started now would hold.
    pub would_take_s: Option<f64>,
}
```

`Clock` is the console's (`crates/finder/src/session.rs:24`). Each story adds only the
fields and items named for it; a field written and never read fails `clippy -D
warnings`. The state behind the lock, a `std::sync::Mutex`, holds: whether the steering
was used; the set point; `subs` and `last`; `far`, the length of the first tracking
error of a stack the steering stands back from, or none; the tracker `before_exposure`
last found, as a `Weak`; how many `Hold`s are alive; a `VecDeque` of at most
`EXPOSURES_KEPT` **exposure records**, newest last; and, from US5, the settings and the
exposure held with the ask it was decided for (040's US6 and US8 add more).

**Lock order.** `Steering`'s lock is a leaf. It is never held while the source closure,
any method of a tracker, or `Imaging` is called: the source reads the session, which
takes the session's own lock, and a tracker's methods take the tracker's. `view`,
`before_exposure`, `accepted` and `release` each copy what they need under the
lock, release it, make their calls, and take it again only to write what they learned.

An exposure record is

```rust
struct Taken {
    /// The console's clock when `before_exposure` answered.
    go_time: f64,
    /// The tracker found, if any. Weak: the steering never keeps a tracker alive.
    tracker: Option<Weak<dyn Steered>>,
    sensor: (u32, u32),
}
```

**The record of a frame** is the newest record whose `go_time` is not after the frame's
mid-exposure, `frame.start + frame.exposure_s/2.0`; none when there is no such record.
The mid-exposure is used, not the start, because a frame's `start` is the scope node's
clock mapped to the console's and may fall a few milliseconds either side of the
answer; the middle of an exposure lies half an exposure from both its own answer and
the next.

**`before_exposure`** (US4), with the lock never held across an await:
1. Mark the steering used. `tracker = source()`, or none when no source is set. Keep it,
   weak, as the tracker last found.
2. With none: push `Taken { go_time: clock.now(), tracker: None, sensor: ask.sensor }`
   and answer `Go { exposure_s: ask.wanted_s, limited: false }`.
3. Unless the steering stands back (**`accepted`**, below):
   `let answer = tracker.gap();`, then the `Arc` is dropped, and then `answer.await`.
   `GapFuture` owns only the answer's receiver, so the steering keeps no tracker alive
   while a gap waits, which can be a whole keyhole pass: if the console's owner drops
   the tracker meanwhile, the tracker's own task stops the mount and answers the gap
   (reanalysis AA11). Then the tracker last found is upgraded again; when it has gone,
   the call goes on as step 2.
4. Push `Taken { go_time: clock.now(), tracker: Some(Arc::downgrade(&tracker)), sensor:
   ask.sensor }`, dropping the oldest past `EXPOSURES_KEPT`, and answer
   `Go { exposure_s: ask.wanted_s, limited: false }`.

US5 changes step 3 and the answers. `on` is read where the gap would be asked for, not
where the call began, so that a switch turned while the call waits is obeyed, and with
`on` false no `gap` is called. Right before a `gap` the steering calls
`tracker.tune(step_share, rate_share)` (§ The tracker's two answers, **Tuning**). The
call keeps its ask, for the view. And the answer is the exposure held (below). 040's US6 puts
a wait before step 3 and a check after it, and 040's US8 keeps in the record the sum of
what the gaps served in the call answered (040's plan.md § The steering's wait and
its check).

**`accepted`** (US4): the record of the frame; with none, nothing. Its tracker,
upgraded; with none, nothing. `target = tracker.target()`; with none, nothing. `set =
set_point_in_frame(record.sensor, set point, frame.roi, frame.binning)`, `error =
tracking_error(solve, set, target, frame.start + frame.exposure_s/2.0)`; with none,
nothing. While `far` is `Some`, nothing. When no tracking error has been handed over
since the last `release` (`subs` is 0) and this one is longer than `FAR_ARCSEC`,
`error.east_arcsec.hypot(error.north_arcsec) > FAR_ARCSEC`, the steering stands back: it
sets `far` to that length, hands nothing over, and calls `tracker.release()` once its
lock is released. Otherwise `tracker.steer(error)`, `subs += 1`, `last = error`. US5:
with `on` false nothing is handed over and the test of `FAR_ARCSEC` is not made.

**Standing back.** While `far` is `Some`, `before_exposure` calls no `gap` and
`accepted` hands nothing over, until the steering's own next `release`; the cap, and
from 040's US6 the wait and `check`, go on as they do with the switch off. Two references hold
the telescope. 015 keeps the finder camera's pointing, read at the aim point in force,
on the target; the subs keep the set point on it. The two differ by the aim point's
error, which 032 measures and holds back for as long as Track is on (032 FR-036). Where
that is more than 038's 5′, the subs' own centring carries the finder camera's pointing
past 038's threshold: the session reads a push nobody made and follows it, the target
moves, the subs find the same error again, and the telescope walks off the target in
hops. The simulator's aim point is exact, so no simulator rig meets it (reanalysis BB5;
read from the code, not run). 180″ is under 038's 300″ with room for the finder camera's
scatter, and by the time a stack's first error is known no step has been made: its first
gap claims and moves nothing. The tracker is then let go for the rest of the stack, and
its own corrections run as 015 has them, inside exposures too.

**Letting the tracker go.** One private function, which three callers share. Under the
lock it takes the tracker last found, upgraded, and sets `subs` to 0 and `last` to
`None` (040's US8: and empties the window); once the lock is released it calls that tracker's
`release`. It keeps the tracker last found, the exposure records, the exposure held,
`far` and the count of drops: the stack goes on, capped and checked, and its next gap,
where one is asked for, claims afresh. The first tracking error after it is then a first
error once more: no drift rate is measured across the break, and `FAR_ARCSEC` is asked
again. Its callers are `set_set_point` with a set point that differs from the one held
(US4), `set_on(false)` (US5), and the wait for the limit (040's US6). Without it a
claim outlives what should have ended it: no gap is asked for, a claim lapses only at a
gap, and while a claim stands the finder camera's pointings change no rate either, so
nothing steers at all. Were `set_on` only to store a flag, a switch turned off during a
stack, which is how the field check with and without the feedback is run, would compare
the feedback with a mount nothing corrected (reanalysis AA5, CC1, DD5, FF2; AA12 for
the set point).

**`set_set_point`** stores the set point, and lets the tracker go when it differs from
the one held: the tracker's last applied error was measured from the old place, and a
set point moved by 3 px between two subs would read as about 1″/s of drift. `None` is
the sensor's centre; 037 names it so as a run starts its stack, before any gap, where
there is usually nothing to change and nothing is called (spec US4-S12).

**`hold`** counts one more `Hold` alive (040's US6: and sets `dropped_not_steady` to 0, a
stack having begun). `Hold`'s `Drop` counts one less and calls `release`. The view's
`holding` is whether any is alive; the page uses it to tell a stack that has stopped
steering from no stack at all (040's plan.md § The page).

**`release`** takes the tracker last found, upgraded, and calls its `release`; then
forgets it, empties the records and sets `subs` to 0, `last` to `None` and `far` to
`None` (US5: and forgets the exposure held and the ask kept with it; 040's US6: and sets
`steady` true and `waiting` false and forgets the pause's word; 040's US8: and empties the
window). It leaves the settings and `dropped_not_steady` as they are. The tracker's
`release` is called after `Steering`'s lock is released (**Lock order**). `Hold`'s
`Drop` calls it. It works from the tracker last found and not from a record, so that a
stack stopped while its first `before_exposure` waits still lets the tracker go.

**`view`**: `None` until used. The state is copied under the lock and the lock released;
then `source` is `none` when `source()` answers none (or no source is set), `subs` when
that tracker's `steered_by_subs()` is true, else `tracker`, asked each time a view is
made. `far_arcsec` is `far`, and `holding` whether a `Hold` is alive.

**The cap** (US5). The settings are `smear_px` 6.0, `on` true, `step_share` `STEP_SHARE`
and `rate_share` `RATE_SHARE` when built. `set_smear_px` refuses a value that is not
finite, under 0.1 or over 20.0 with exactly
`the corner smear must be a number from 0.1 to 20`; `set_step_share` and
`set_rate_share` refuse one that is not finite, under 0.0 or over 1.0 with exactly
`the step share must be a number from 0 to 1` and
`the rate share must be a number from 0 to 1`. `set_on` and an accepted setter mark the
steering used; a refused one changes nothing, the mark included. `set_on(false)` also
lets the tracker go (**Letting the tracker go**); `set_on(true)` calls nothing, and the
stack's next `before_exposure` asks for a gap, which claims. The state gains
`held: Option<(f64, bool)>`, the exposure a stack holds, in seconds, and whether it was
cut when it was decided, and `asked: Option<ExposureAsk>`, the ask of the latest
`before_exposure`. In `before_exposure`, after the gap (or at once, with `on` false or
no tracker):

```rust
let turn = |ask: &ExposureAsk| {
    let set = set point, or (f64::from(sensor.0)/2.0, f64::from(sensor.1)/2.0);
    let radius_px = the largest of hypot(cx - set.0, cy - set.1) over the four corners
        (roi.x, roi.y), (roi.x + roi.width, roi.y), (roi.x, roi.y + roi.height),
        (roi.x + roi.width, roi.y + roi.height), divided by binning;
    smear_px / radius_px
};
let decide = |ask: &ExposureAsk, ahead: Option<f64>| {
    let floor = ask.shortest_s.max(HALVING_FLOOR_S);
    let mut exposure = ask.wanted_s;
    if let Some(limit) = ahead {
        while exposure > limit && exposure / 2.0 >= floor { exposure /= 2.0; }
    }
    (exposure, exposure < ask.wanted_s)
};
let (exposure, limited) = match held {
    Some(held) => held,                      // decided at this stack's first exposure
    None => {
        let ahead = tracker.as_ref().and_then(|t| t.longest_exposure_ahead_s(turn(&ask)));
        let decided = decide(&ask, ahead);
        held = Some(decided);
        decided
    }
};
asked = Some(ask);
Go { exposure_s: exposure, limited }
```

The set point's default is half the sensor's size as `f64`, 618.5 for a sensor 1237
wide, not half rounded down (spec US5-S1). The exposure is decided from the tracker's
answer for the hour ahead, not for now: a stack holds it for its whole life, and before
a target's transit the limit only falls (§ The tracker's two answers,
**`longest_exposure_ahead_s`**; reanalysis FF5). The halving stops where one more would
pass 1 s, or the camera's shortest where that is longer: stopped only at the camera's
shortest, a 30 s setting near the zenith would become subs of a quarter of a second,
130 frames of 31 MiB for each 30 s asked for. An exposure the observer set under
1 s is not halved at all. `limited` is decided with the exposure and held with it: a
later call answers both as they were decided, whatever `wanted_s` it brings, until
`release` forgets them (spec US5-S2: a wanted 10 s against a held 30 s still answers
30 s, limited). Halving and not the limit itself, so that a night needs master darks at
a few exposures (034 matches one within 1 ms). Held and not followed, because 035 drops
a sub whose exposure differs from its reference sub's (035 plan.md § The settings test).

**`view_for`** (US5). `view()` is `view_for(None)`, and US4's tests go on calling it.
The state is copied under the lock and the lock released, as in `view`; then, with
`tracker = source()`:
- it is `None` while the steering was never used, as `view` has it, unless a tracker
  runs and an ask is given: then the view is made all the same, and making it marks
  nothing. The observer must be able to read what a stack would take before one has run;
- `longest_exposure_s` is `Seconds(limit)` or `Text("no limit")` from
  `tracker.longest_exposure_s(turn(a))`, a being `asked`, the stack's own ask, or with
  none the ask given; absent with no tracker or neither ask. It is asked as the view is
  made, like `steered_by_subs`. Kept from the latest `before_exposure`, it would show no
  limit before a stack, and after one, or with Track off, a limit an hour old
  (reanalysis FF17);
- `over_limit` is whether that limit is a number under the exposure held, false while
  none is held;
- `exposure_s` and `limited` are the exposure held and whether it was cut, absent and
  false while none is held, before a stack's first exposure and after `release` alike;
- `would_take_s`, only with a tracker and an ask given, is
  `decide(g, tracker.longest_exposure_ahead_s(turn(g))).0`, g being the ask given: what
  `before_exposure` would decide now. It is given whether or not a stack holds an
  exposure, so that a stack started while the limit was low can be seen to hold less
  than a new one would.

`Imaging` gives the ask (§ The imaging state). A call with no ask, as every scripted
steering's `view()` is, shows the limit for the stack's own ask and no `would_take_s`.

### The session (`crates/finder/src/session.rs`, US4; 040's US6 adds the push)

```rust
/// The running tracker as the stack sees it (036): `None` while Track is off, armed or ended.
pub fn steered(&self) -> Option<Arc<dyn crate::imaging::steering::Steered>>;
```

`self.tracker()` (`crates/finder/src/session.rs:773`), kept when its
`state().state != "off"`. 037 asks it `steady()` and `steady_between` over a window that
ends now.

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

US5 changes that one call to `self.steering.view_for(ask)`, `ask` being `Some` of the
`ExposureAsk` the console's camera and settings give, built as 040's US7's taker builds its
own (040's plan.md § The stack: `wanted_s` and `shortest_s` from `settings.exposure_ms` and
`camera.exposure_ms.min`, the sensor, the region and the binning), and `None` while the
view has no `camera` or no `settings`. So from US5 a console whose steering has a source
carries `steering` whenever a tracker runs and its camera is ready, with
`longest_exposure_s` and `would_take_s`; until 040's US7 gives the console's steering its
source, only a test that calls `set_source` meets that.

### The route (`crates/finder/src/imaging/routes.rs`, US5)

`PUT /api/imaging/steering`, handled by `put_steering`, taking `SessionWrite` and
`Result<Json<SteeringEdit>, JsonRejection>` through `sim_json`, as 031's `put_settings`
does:

```rust
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SteeringEdit {
    pub on: Option<bool>, pub smear_px: Option<f64>,
    pub step_share: Option<f64>, pub rate_share: Option<f64>,
}
```

Each field uses the `deserialize_with` 031's `SettingsEdit` uses, so a JSON null is an
error and not "absent". Without imaging: 409 `this console takes no imaging frames`.
The three numbers are checked first, `smear_px`, `step_share` and `rate_share` in that
order, each against its setter's range, and the first out of range answers 400 with that
setter's words and changes nothing, `on` and the other numbers included. Then all that
are present are set, `on` last. It answers `Json(imaging.view())` with 200.
It is not refused while the camera is not `ready`: the settings are the console's own,
and the view carries `steering` whatever its `state`. The checks and the setting are one
function, `pub(crate) fn apply_steering(imaging: &Imaging, edit: SteeringEdit) ->
Result<ImagingView, String>`, which `put_steering` calls and answers from, the `Err`
being a refusal's words; 040's US11's tool calls the same function, so that the tool and the
route cannot come to differ.

`crates/finder/src/lib.rs` gains, after the last route line earlier specs left, the
`GET /api/imaging/stack/preview.jpg` line (`crates/finder/src/lib.rs:35` today),
written as it stands here:
`* PUT /api/imaging/steering: {"on":true|false,"smear_px":N,"step_share":N,"rate_share":N}, any of them; whether the subs steer the tracker, the corner smear that caps the exposure, and the two shares of the control law.`

`crates/finder/README.md` gains a row in the `## API` table
(`crates/finder/README.md:167`), after the last `/api/imaging` row earlier specs left,
the `GET /api/imaging/stack/preview.jpg` row:

```markdown
| `PUT /api/imaging/steering` | JSON with any of `on` (true or false), `smear_px` (0.1 to 20), `step_share` and `rate_share` (each 0 to 1). Whether the subs steer the tracker during a stack, how many pixels the field may turn at the corner of a sub in one exposure, and the shares of a tracking error and of a drift that one gap takes out. Answers 200 with the imaging view. Run-only: on, 6 px, 0.5 and 0.3 at every start. Same-origin writes only. |
```

### What 035 must provide (035's files; read as landed before the flip)

031 has landed with the camera's lease, and 035's eleven stories have landed on it.
035's US9 and US10 wrote the page, the README and the MCP tools, and
`stack/service.rs` is still as 035's US8 left it.
This section lists everything this spec assumes of 035, and through it of 031's lease,
so that the re-anchoring has one list to check (flip.md § Prerequisites 3). 035's Prerequisites
10 and 037's plan.md point here. 035 says that "a sub taken at an exposure that changes
within a stack is not offered" (035 plan.md § Prerequisites 10); this spec does not ask
for one. It changes the setting once, before a stack's first sub.

What 040's US7 and 040's US8 rely on:

1. **The taker's loop** has one place before each `lease.take("sub")`, after its look at
   the stopping mark, and the taker, `Stacker::take_subs`, reaches the console's imaging
   state as `self.imaging`, from which `self.imaging.steering()` comes; the worker
   reaches it as `self.stacker.imaging`.
2. **The stopping mark** (035 plan.md § The stack on the console, **The stopping
   mark**): one `tokio::sync::watch` channel of `bool`, made with the run, `false`
   until the stack is ending and `true` from then on, set once by whoever ends the
   stack; the taker and the worker each hold a receiver (the taker's by value: 040's US7 makes
   it `mut`, 040's plan.md § The stack). The taker's wait races it as
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
   way a stack ends; the taker returns only at the top of its loop, from the gap's wait,
   on a refused take, on a take answered with no frame, or after `imaging.ended` has
   answered for its last frame, so no frame is in flight as it returns, and the lease
   drops with it. 035 puts the observer's exposure back "where the taker returns and
   before the lease is dropped" (the seam, 2), which is where 040's plan.md § The stack does it.
6. **The worker**: `Worker::process` builds the sub's `SolveFrame` and then makes the
   settings test, which answers `Err("settings")` for a sub whose `exposure_ms` differs
   from the reference sub's by more than 1 ms, before it sets `last`; it answers
   `Ok(Accepted { frame, solve, .. })` for a sub it accepts. `Worker::run` counts the
   outcome, calls `note_solve` for an accepted sub with no stack lock held, and then may
   make a picture.
7. **`Dropped`** derives `Default`, writes every key always and in order, and has
   `total()` and `count(outcome)`, whose match names every outcome the worker answers
   and is `unreachable!` for any other; 040's US7 adds `not_steady` to the struct, to
   `total()` and to that match (040's plan.md § The stack). `stack.json`'s `subs` writes the outcome a
   sub was counted under, and `LastView::outcome` that of a sub that reached the solve.
   No test of 035 counts the keys, compares a whole `dropped` object with a literal, a
   fixed list of keys, a golden file or a schema that lists its keys (a comparison of
   two serializations of the same `Dropped`, as
   `stack_console.rs` makes between `stack.json` and the view, is not one), or builds
   `Dropped { .. }` without `..Default::default()` (035 spec.md § User Scenarios: "a
   test never asserts how many there are").
8. **`StackBlock.tsx`** builds its `Dropped {sum}: {parts}` line from the counts above
   0, in `StackDropped`'s order, each with its words. Its filter of the counts above 0
   reads an optional count with `?? 0` once `not_steady` is optional (040's plan.md § The page).
9. **`Stacker::start`** refuses a camera that is not `ready`, so the taker's first read
   of the view finds `camera` and `settings`.

What they change in 035's two files, and nothing else:

| File | Change | Story |
|---|---|---|
| `crates/finder/src/imaging/stack/service.rs` | `Dropped` gains `not_steady`, last, `total()` adds it and `count` gains its arm | 040's US7 |
| `crates/finder/src/imaging/stack/service.rs` | the taker keeps a `Hold`, awaits `before_exposure` before each take raced against the stopping mark, and before its first take sets the exposure through the lease, ending the stack `camera` on a refusal | 040's US7 |
| `crates/finder/src/imaging/stack/service.rs` | the taker's loop runs in one `async` block, and after it the taker puts the observer's exposure back through its lease before the lease drops with the task | 040's US7 |
| `crates/finder/src/imaging/stack/service.rs` | the worker's `process` asks `check` before the settings test and answers its drop as `Err`, and `run` calls `accepted` right after `note_solve` | 040's US7 |
| `crates/finder/web/src/components/StackBlock.tsx` | the `Dropped` line gains `{count} while the mount was not steady`, last | 040's US8 |

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
  `set_imaging_camera`'s nine keys are 031's (031 spec.md US1-S2, US7, US8-S1). The four
  settings here have their own route. `get_imaging_camera`'s description in the tool
  listing is 031's too, and 031's MCP test asserts its words: 040's US11 leaves it as it is.
- **The imaging view without steering.** `steering` is skipped until the steering is
  first used. No test of 030, 031, 032 or 034 starts a stack or calls this spec's route, so
  every body they read is what it was. From 040's US7 on a stack uses the steering, and the
  imaging view of a console that ran one carries `steering`: 035's tests read a view key
  by key (035 spec.md § User Scenarios, Comparing) and are not disturbed. From 040's US7 on
  the view of a console that tracks with its camera ready carries it too, used or not
  (§ The steering, **`view_for`**). 032's `imaging_track.rs`, the one landed test that
  tracks with an imaging solver, reads its solve and no whole view; flip.md § Prerequisites 3
  checks that no landed test compares an imaging view, or the session's `imaging`
  object, whole on a console that tracks.
- **Finder's test layout.** `crates/finder/tests/layout/main.rs` fails a file at the top
  level of `crates/finder/tests` (`crates/finder/tests/layout/main.rs:109`), a file
  added to `support` (`crates/finder/tests/layout/main.rs:127`) and a `main.rs` holding
  anything but its support include and `mod` lines
  (`crates/finder/tests/layout/main.rs:156`).
- **This spec's own shapes, for its later stories and for 037.** `SteeringView` is
  `#[non_exhaustive]` and tests read it key by key. `Steered`'s required methods are
  fixed in US2: the scripted trackers of US4 and US5, and of 040's US6 to US8,
  implement them, so no later story
  adds a required method, only bodies for the provided ones. `ExposureAsk` and `Go` have
  every field from US4 on, and `before_exposure`, `accepted`, `hold`, `release`,
  `set_set_point` and `view` keep their signatures: US5 adds `view_for` beside `view`
  and does not change it, because US4's tests call `view()` and no story edits an
  earlier story's test file. A case of an early story asserts nothing a later story
  changes: US5's cases hold no limit under half the exposure they answer, which 040's US6
  would wait on, and every test file passes unedited under every later story (§ Plan
  traps 22 and 34).
- **What 037 reads.** Two of the things 037 reads of the steering are this spec's, each
  kept as it is here (037 plan.md § What 030 to 036 offer, flip.md § Prerequisites 2):
  - `Steering::set_set_point(None)`, called as a run starts its stack (spec US4-S12);
  - the view's `steering.far_arcsec`, present while the steering stands back from a
    stack that began more than 180″ from its set point.

  It also reaches the tracker through `Session::steered()` (§ The session). The other
  three, `Steered::steady()`, `Steered::steady_between` and `Steering::waiting()`, are
  040's (its plan.md § Shapes that existing tests pin).
  What 037 may not lean on: that the steering brings in a target stacked off the centre.
  Beyond 180″ nothing steers it; inside, a step against an axis's motion is at most that
  axis's rate times half a second a gap (spec FR-008).

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

Each binds every story.

1. **A rate after a Stop.** The scope node drops a move queued before a Stop and accepts
   one that arrives after. `gap` runs in the stack's task. *Check:* `gap` only queues a
   sender and wakes the rate task; `serve_gaps` is called from `run`'s loop alone;
   US3-S7 asserts that a gap after a stop sends nothing.
2. **The simulator's truth on a moving mount.** Asked after the fact, it used the
   direction of now. *Check:* US4 names US1 in its `depends_on_merged`; US4-S5 and 040's US7
   read the truth of past instants and fail without it.
3. **Refraction in the simulator.** The tracker follows the apparent place and the
   simulator draws the geometric sky, 65″ apart at M42's altitude in the rigs. *Check:*
   scenarios on the tracker rig measure against the apparent place; US4-S5's rig and
   040's stacking rig start on the catalogue place, so their first tracking error is
   small (§ Plan traps 17 says why it must be).
4. **A step that the axis may not make.** A step against an axis's motion is cut by
   `commanded`. *Check:* the remembered correction keeps the step made, not the step
   wanted, and `Stepped` reports the step made.
5. **The claim gap is not an idle gap.** *Check:* `idle` counts gaps after the claim;
   US9-S4 asserts the claim stands at the seventh gap after it and lapses at the eighth,
   and that a ninth leaves it lapsed.
6. **Finder pointings while steered.** Skipping all of `Shared::pointing` would lose the
   cable watch, the keyhole's resume and the backlash measure. *Check:* only the block
   that sets the proportional rate and the offset is skipped; US3-S6 asserts the samples
   still grow, and US9-S9 that the pointing model still takes syncs.
7. **An oracle copied from the code proves nothing.** *Check:* axis parts come from the
   simulator's horizon transform, differentiated by the test; drifts from the
   simulator's truth; the cap from 015's `longest_exposure`; rates from a wrapper round
   the mount.
8. **`SteeringView` grows by story.** *Check:* tests read it key by key; no test
   compares a whole `steering` object.
9. **A named command is read literally.** *Check:* commands are copied from
   § Verification.
10. **One evidence budget.** *Check:* each story keeps its diff under 64 KiB, measured
    with `git diff <base> -- . ':(exclude)crates/finder/web/dist' | wc -c`. A story that
    measures over it says its size when it hands off, and drops or shortens no case to
    get under: 64 KiB is the size the judge prefers, a cut is made in the trio and not
    in a build, and a missing case is worse than a large diff (reanalysis DD12).
11. **A spec fixed mid-epic needs a relaunch.** *Check:* no builder edits
    `specs/036-tracking-from-subs/` or `.specify/`.
12. **027's layout.** *Check:* each new finder test is the module § Technical Context
    names, declared by one alphabetical `mod` line; no file is added to `support`.
14. **The lock across an await.** `Steering`'s and the tracker's locks are
    `std::sync::Mutex`. *Check:* neither is held across `gap().await`, a sleep or a
    send.
15. **A tracker kept alive.** Dropping a `Tracker` stops the mount
    (`crates/finder/src/tracking.rs:112`). *Check:* `Steering`'s state holds only `Weak`
    references; the one strong reference is the local of a `before_exposure` call, and
    it is not held across `gap().await`: `GapFuture` borrows nothing, and the call lets
    its `Arc` go before it awaits the answer; and 040's US6-S4 drops the test's `Arc` and sees
    the sub dropped.
16. **An error older than the corrections.** 035's worker hands over sub N's tracking
    error while sub N + 1 or later is exposing. *Check:* `serve_gaps` step 2 adds the
    steps made after the error's time and the rate changes it did not see; US3-S9 and
    US3-S10 fail for a tracker that treats the error as fresh, and US3-S1 and US4-S5
    play their subs late.
17. **A step against an axis's motion.** `commanded` sends zero in place of a rate
    against the axis's direction, so such a step is at most the axis's own rate times
    the half second. *Check:* the scenarios that expect a full step hand over errors
    whose step runs with the axis (US3-S4, US3-S5, US3-S9), US9-S3 expects the cut, and
    the rigs of US4-S5 and 040's US7 start on the catalogue place so that no
    scenario needs the altitude axis to back up 65″.
18. **A gap nobody waits for.** A stack that stops while `before_exposure` awaits a gap
    drops the future, and the `Hold` releases at once; a gap served afterwards would
    claim a tracker no stack will ever release. *Check:* `serve_gaps` drops senders
    whose receiver has gone before it claims, and `Steering::release` releases the
    tracker last found even when no exposure was recorded.
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
    still `[0.0, 0.0]`; and once 040's US6 lands it would find the tracker not steady and
    claim nothing, so a scenario that began with it would pass in US3 and fail in 040's US6,
    which may not edit US3's test file. *Check:* `serve_gaps` step 1 turns a gap away
    while `settling` is true; the tracker rig waits for the first rates, and 3.5 s more,
    in every module that writes it out, because from 040's US6 a tracker also settles for 2 s
    after them and turns a gap away until then; US9-S7 asserts what a gap before the
    first rates does, and the assertion holds under US3's test and under 040's US6's; 040's US6's
    development checks run US3's and US9's modules.
24. **A lapsed claim taken back too soon.** *Check:* `serve_gaps` step 1 claims a
    lapsed tracker only when an error is pending; `steer` keeps the newest error while
    lapsed; `release` and `end` clear `lapsed`, so a new stack's first gap always
    claims; US9-S4 asserts that a ninth idle gap leaves the finder camera steering.
26. **`Steering`'s lock held across a call out.** The source reads the session under
    the session's lock, and a tracker's methods take the tracker's. *Check:* § The
    steering, **Lock order**: no call to the source, a tracker or `Imaging` is made
    with `Steering`'s lock held.
27. **A finder correction inside an exposure after a lapse.** Once eight gaps bring no
    tracking error, a stack still holds the tracker and is still exposing. *Check:*
    while `lapsed`, `Shared::pointing` writes the step to `held` and touches neither
    `proportional` (beyond zeroing it, as 015 does) nor the tracking offset, which
    nothing changes until a sub is accepted; only `serve_gaps` step 2 applies `held`;
    US3-S11 asserts that the pointings change no rate when they come, US9-S11 that no
    rate between two gaps differs from the feed-forward plus the offset with a noisy
    finder task running, and US9-S4 that the claim taken back forgets what was held.
33. **A claim that outlives what should end it.** A claim lapses only at a gap. Where
    the steering stops asking for gaps, it must release the tracker itself, or nothing
    steers: the finder camera's pointings change no rate under a claim. *Check:* § The
    steering, **Letting the tracker go**, is called by `set_on(false)`, by a
    `set_set_point` that changes the set point and by the wait for the limit; US5-S9,
    US4-S12 and 040's US6-S17 each see one `release` in the scripted tracker's log.
34. **A case of an early story that a later story's wait would hold.** From 040's US6 a
    `before_exposure` waits while the limit is under half the exposure held, and a test
    of US5's that answered such an exposure would then never end, in a file 040's US6 may not
    edit. *Check:* no case of US5's two modules has a limit under half the exposure it
    answers (spec US5-S1 says so of its own); 040's US6's development checks run both.
35. **A first tracking error the steering stands back from.** A scenario that expects a
    `steer` must hand over a first error under 180″. *Check:* US4-S4 names the set point
    (668.0, 386.0), 110″ out, where it named one 230″ out; US4-S8 and 040's US7-S7 are the
    only scenarios whose first error is over the limit.
36. **A story of tests only.** US9 writes nothing under `crates/finder/src`, and its
    tests pass on the code of US3, the story it was cut from, as soon as they are
    written: there is no failing test to write first. *Check:* its tasks say so; a test
    of its that fails is a finding about US3's code and is escalated with its output,
    not answered by an edit of that code or of the test's bound.
37. **The view before the steering was used.** `view()` answers `None` until the
    steering is used, and US4's tests assert it; from US5 `view_for` with an ask answers
    a view while a tracker runs, used or not. *Check:* only `view_for` with `Some` makes
    that view, it marks nothing used, and US5-S12 asserts both.
38. **A guarded word in a comment.** US2-S6 and 040's US7-S6 read the whole text of
    `steering.rs` and of 035's `stack/service.rs` for thirteen names, and a doc comment
    that explains a pause would use four of them. *Check:* the pause's words and their
    reasons are written in `tracking.rs` and in the page only; `steering.rs` passes
    `pause` through unread.

## Risks

| Risk | Mitigation |
|---|---|
| The bounds were worked from a model, and probe round 1 ran them on the simulator alone. | flip.md § Prerequisites 5: met. Every bound held on three runs each; the narrowest is 040's US6-S5's, 41.414″ against 015's 60″ (1.45×), and the drift bound's is 4.4× at the 12th sub played late (0.0340″ against 0.15″, where the model gave 0.030″). The real mount is a field check. |
| The owner prefers arcseconds for the corner smear (owner decision 1). | The steering then needs a scale before a stack's first solve, 032's measured one or the one from the focal length setting; FR-018, US5 and the page's field are redrafted before the flip. |
| A sub dropped as soft carries a good tracking error that is not used, and a drift that makes subs soft is then taken out by the tracker's own corrections only. | Owner decision 15; a field check. |
| The limit falls under a long stack's exposure near the meridian. | The exposure is decided from the shortest limit of the hour ahead (34.3 s on the worked rig at 2 px, against 37.28 s at its start, measured in probe round 1), so a stack under an hour old does not meet it unless the floor of 1 s held its exposure up. The readout says `over_limit`, and under half the exposure held the stack waits. The hour, its sampling each minute and the figures for other targets are arithmetic on 015's formula, not a build: probe round 2 runs US5-S11. |
| The default smear of 6 px lets a corner star trail three times as far as the drafted 2 px. | It is the owner's choice of 2026-11-19, on the research note's figures for this telescope (28 px of coma in that corner uncorrected; 20 s to 30 s subs recalled). It stays a setting, and the block's `Longest exposure` names the smear in force. A field check on a bright field. |
| A half-second pulse of one count does not move the real tube (stiction, backlash). | The rate then takes it up: an undelivered step of under two deadbands reads as a drift, and 0.3 of it goes into the rate, 0.1″/s a gap at most. A field check (spec § Assumptions). |
| The real mount makes more or less than the step asked for. | A pair of errors with more than two deadbands of steps between them changes no rate, and a step is booked for the time its pulse really lasted (§ The tracker, `serve_gaps` steps 2 and 4). Models of the law, not builds, say what that buys. A field check: the tube's motion per commanded pulse of 1″, 10″ and 30″. |
| The real mount's rate commands are coarser than the model. | Field check; the constants are in one place. |
| An error that needs a slow axis to back up leaves slowly. | 015's rule that no correction reverses an axis stands; near a reversal the take-up crosses it. A field check through the meridian. |
| The real solver is slower than a sub, so errors arrive several gaps late. | The tracker's memory of 16 gaps covers 035's queue of four, and the claim lapses only after eight gaps with no error, so a worker a whole queue behind still measures a drift rate. `took_ms` in 035's view, against the sub cadence, is the figure to read: a field check. |
| Accepted subs come more than eight gaps apart (cloud, a gate dropping most subs, a console that needs more than eight sub lengths for one sub). | Each tracking error is then the first after a claim and sets no rate, and since the reanalysis the finder camera's corrections change no rate after a lapse either: a rate error stays until subs come closer together, and only steps correct the place. The readout shows `source` changing. A field check with subs dropped; keeping the last applied error through a lapse is the remedy if the field shows it, and wants a probe of its own. |
| An imaging solve is wrong and 035 accepts it, or the tube is knocked between two subs. | One gap moves an axis 30″ on the sky at most; a jump changes no rate, and anything under the jump test changes it by 0.1″/s at most. |
| The finder camera's aim point and the sensor's centre disagree by more than 038's 5′. | The steering stands back from a stack whose first tracking error is over 180″, and the page says so. How large the aim point's error is on the real finder is not measured; under 5′ and over a few arcseconds, each lapse hands the telescope to the finder camera's reference and each return takes it back. A field check. |
| After a lapse the finder camera corrects once a gap, not once a solve, and corrects the place only. | Each gap takes 0.6 of the newest pointing's error out as a step, and the tracking offset stays as the subs left it. A lapse means subs are being dropped (cloud, a lost field), which the readout shows. The step is from one finder pointing, so while lapsed the frame is moved by the finder camera's scatter each gap, about 13″ to 21″ in a model, between exposures. A field check with subs dropped. |
| A story's diff nears 64 KiB. | Round 1 measured every story of the spec this one was split from: US1 19,045 B, US2 14,094 B, US3 50,893 B (24,984 B of tracker and 25,583 B of test), US4 31,946 B and US5 38,804 B. With its cases folded in and the reanalysis's first pass, US3 stood at about 61,300 B (93%), and it was cut by tests into US9 (history.md). Estimated from those measures and the length of what the plan and the scenarios now ask for, nothing of it built: US1 about 24,400 B (37%), US2 15,600 B (24%), US3 49,100 B (75%), US4 38,700 B (59%), US5 53,100 B (81%) and US9 23,900 B (36%). The factory's builder has landed large stories 2% to 4% over such estimates. US5 is now the nearest to the budget: the hour ahead, the view's ask, the two shares and the switch's release are about 10 KB of it. If a build measures US5 over, it is cut as US3 was, by tests: its four motion scenarios (US5-S6, US5-S8, US5-S11 and US5-S14) and their module go to a story of their own after US5, about 9 KB. No story drops or shortens a case to get under (§ Plan traps 10). |
| `view_for` makes the imaging view carry `steering` on any console that tracks with a ready camera, from 040's US7. | 035's tests read views key by key, and 032's tracking test reads its solve. flip.md § Prerequisites 3 checks the landed tests for a whole-view comparison on such a console before the flip. |
| The two shares can be set to values that make the loop ring or stall. | They are bounded to 0 to 1, run-only and back at 0.5 and 0.3 at every start; the rate's limit a gap and the two-deadband rule still hold whatever they are. Models of the law, not builds, say 0.5 and 0.3 are sound: a field check, and the reason they are settings. |
| The caps of 16 remembered gaps, 32 remembered exposures and 64 times not steady are asserted by no scenario. | Two of each pass every test (reanalysis CC15). A case for each was weighed against the size of US3 and 040's US6 and left out; each is one named constant. |
| 037's trio says the stack's steering brings in a target stacked up to a quarter of the frame from the centre. | It does not beyond 180″, and inside it is slow against an axis's motion. 037's sentences and its two warnings are corrected in 037's trio; the fold's report lists them. |
| 030 to 035 and 037 write `mod.rs`, `routes.rs`, the page and the README too. | The six land first; 037 names this spec. flip.md § Prerequisites 2 re-anchors. |

## Refinement Record

Writes come from § Project Structure. Shared writes name the merge edge that orders two
stories of this spec on one file, under the earlier story, or the story that, by the
drafts of 030 to 035, last writes the file before this spec does. A test
file that an earlier spec's draft or an earlier story of this spec creates cannot be
listed under `tests` as existing, because the validator resolves every such path in the
drafting worktree; each is named under `preserved_behavior` with what it must still
assert, and moves to `tests` at the re-anchoring. A file last written on `main` by a
spec outside this spec's `depends_on_landed` (`crates/simulator/src/controls.rs`) has no
entry for that spec, because the validator refuses one. `crates/finder/src/tracking.rs`
was such a file, last written by 024, until 038's US1 wrote `Tracker::follow` into it;
038 is now in `depends_on_landed`, and US3 names that story.
`crates/finder/tests/motion/main.rs`, last written on `main` by 025, is
written after it by 032's US4 (`mod imaging_track`), and US3 names that story. The
stories of `040-stack-waits-for-steady` that write a file after this spec name, in that
spec's record, the story of this spec that last wrote it; this record names none of
them. The record's `open_questions` are empty because the validator refuses a record that
holds any; what is not established is in § Risks and in flip.md § Prerequisites 2 and 3,
and in owner decisions 2 and 15 (flip.md). Nothing in this record is an owner approval:
those are in flip.md, and flipping to `ready` gives them. history.md has the record's
account of how the trio was drafted, probe-built and folded.
```yaml
version: 1
reviewed_revision: 49bb210fe4860dd98ba24727074e7bd3cc0d7124
stories:
  US1:
    tests:
    - path: crates/simulator/tests/imaging_history.rs
      status: new
      disposition: add
      assertion: "on a mount turning at a commanded 0.02 deg/s with a 0.25 rate error, the imaging truth asked 30 s later for three instants of an exposure has the centre of the eyepiece read live at each and the angle of the truth read then, the azimuth rose 0.05 deg in 2 s, the centre moved, and the star is elongated by the distance between its true places"
      decision: "spec.md:US1-S1"
    - path: crates/simulator/tests/imaging_history.rs
      status: new
      disposition: add
      assertion: "the truth for an instant between two samples has the centre of a twin's eyepiece and the angle of its truth read live at that instant, with both axes turning, and across 0 deg of azimuth"
      decision: "spec.md:US1-S2"
    - path: crates/simulator/tests/imaging_history.rs
      status: new
      disposition: add
      assertion: "after a push by hand, a mount connected, the optics set or the eyepiece pointed, the truth for an earlier instant is the one directions_at gives, and stays so after a later read"
      decision: "spec.md:US1-S3"
    - path: crates/simulator/tests/imaging_history.rs
      status: new
      disposition: add
      assertion: "with no mount the truth for a past instant equals directions_at's eyepiece"
      decision: "spec.md:US1-S4"
    - path: crates/simulator/tests/imaging_history.rs
      status: new
      disposition: add
      assertion: "count steps are off when built, when the true azimuth takes over 100 values in 20 s, and when on the true azimuth and altitude each take 8 to 11 values, each a whole count from the first, with a drive under one count moving nothing"
      decision: "spec.md:US1-S5"
    - path: crates/simulator/tests/imaging_history.rs
      status: new
      disposition: add
      assertion: "the truth read at t is answered again after 3000 s of reads every 100 s, and after 3700 s the sample is dropped and the answer is directions_at's"
      decision: "spec.md:US1-S6"
    - path: crates/simulator/tests/imaging_history.rs
      status: new
      disposition: add
      assertion: "a push made on the mount itself keeps the memory of the instants before it"
      decision: "spec.md:US1-S7"
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
    - "032's motion/imaging_track.rs passes unchanged: its solve on a tracking mount now reads the mid-exposure direction, and its aim stays within its 1.5 px"
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
      assertion: "the set point in a frame's pixels for the whole sensor, a binned region, a named set point and a sensor of odd size"
      decision: "spec.md:US2-S3"
    - path: crates/finder/tests/console/steering_error.rs
      status: new
      disposition: add
      assertion: "with a rendered star's centroid as the set point the error is nothing, and with the frame's centre it is the star's offset reversed"
      decision: "spec.md:US2-S4"
    - path: crates/finder/tests/console/steering_error.rs
      status: new
      disposition: add
      assertion: "a target at the far side of the sky, a time, a set point or a target that is not finite, or a solve whose matrix is zero gives none"
      decision: "spec.md:US2-S5"
    - path: crates/finder/tests/console/steering_error.rs
      status: new
      disposition: add
      assertion: "steering.rs holds none of the thirteen guarded names"
      decision: "spec.md:US2-S6"
    - path: crates/finder/tests/console/steering_error.rs
      status: new
      disposition: add
      assertion: "a tracker with only the required methods names no limit, now or for the hour ahead, is always steady, gives no pause word and ignores tuning"
      decision: "spec.md:US2-S7"
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
      assertion: "on a mount 1% fast, the drift within a 5 s sub is at least 0.6 arcsec at first and at most 0.15 from the 12th sub on with the subs steering, whether each error is handed over at once or a sub late, the offset reaches the rate error, and without feedback every sub drifts at least 0.6, every truth error from the 12th sub on is at most 3 arcsec, and the subs steer from the first gap"
      decision: "spec.md:US3-S1"
    - path: crates/finder/tests/motion/steering.rs
      status: new
      disposition: add
      assertion: "with finder pointings 15 arcsec noisy and no feedback, the median drift of 5 s subs is above 3 arcsec"
      decision: "spec.md:US3-S2"
    - path: crates/finder/tests/motion/steering.rs
      status: new
      disposition: add
      assertion: "an axis error of 1.9 counts moves nothing and sets no rate"
      decision: "spec.md:US3-S3"
    - path: crates/finder/tests/motion/steering.rs
      status: new
      disposition: add
      assertion: "a first error steps and sets no rate; a pair of errors with more than two deadbands of steps between them sets none on either axis, though it shows a drift; and a drift between two errors with a small step between them takes 0.3 of itself out of each axis's rate"
      decision: "spec.md:US3-S4"
    - path: crates/finder/tests/motion/steering.rs
      status: new
      disposition: add
      assertion: "a jump over twice the offset's limit changes no rate, and one gap moves an axis 30 arcsec on the sky at most, 41 of azimuth here"
      decision: "spec.md:US3-S5"
    - path: crates/finder/tests/motion/steering.rs
      status: new
      disposition: add
      assertion: "finder pointings change no rate while the subs steer and still fill the samples, and a claim ends the finder camera's proportional rate"
      decision: "spec.md:US3-S6"
    - path: crates/finder/tests/motion/steering.rs
      status: new
      disposition: add
      assertion: "between gaps every rate is the feed-forward plus the offset the gap left, every gap sends both axes a rate, and after a stop the subs no longer steer and a gap answers no step and sends nothing"
      decision: "spec.md:US3-S7"
    - path: crates/finder/tests/motion/steering.rs
      status: new
      disposition: add
      assertion: "a drift whose 0.3 share is over 0.1 arcsec/s changes the azimuth offset by 0.1 and no more"
      decision: "spec.md:US3-S8"
    - path: crates/finder/tests/motion/steering.rs
      status: new
      disposition: add
      assertion: "an error measured before the last gap's step is stepped by half of what that step left, not by half of itself, and sets no rate; of two errors handed over between two gaps the later is used, and one no later than the last applied is ignored"
      decision: "spec.md:US3-S9"
    - path: crates/finder/tests/motion/steering.rs
      status: new
      disposition: add
      assertion: "an error measured before the last gap's rate change takes 0.3 of the drift that change left, so two equal drifts give 1.7 and not 2 times the first change"
      decision: "spec.md:US3-S10"
    - path: crates/finder/tests/motion/steering.rs
      status: new
      disposition: add
      assertion: "after the claim lapses at the eighth idle gap, two finder pointings of 30 arcsec east change no rate when they come, and the next gap steps -0.6 of the newest, leaves the tracking offset exactly as it was and has ended its pulse at its answer"
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
    - "crates/finder/tests/motion/tracking/push.rs and push_back.rs, 038's test files, pass unchanged: follow still follows a push as 038 has it, and with no stack holding the tracker it has nothing more to end (plan.md The tracker, follow)"
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
    - path: crates/finder/src/tracking.rs
      other_spec: 038-track-follows-a-push
      other_story: US1
      disposition: merge_order
      reason: "036 depends_on_landed 038, whose US1 last writes tracking.rs, with Tracker::follow"
    - path: crates/finder/tests/motion/main.rs
      other_spec: 036-tracking-from-subs
      other_story: US9
      disposition: merge_order
      reason: "US9 depends_on_merged US3, and adds mod steering_claim after mod steering"
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
      assertion: "with no tracker nothing waits and nothing is handed over"
      decision: "spec.md:US4-S2"
    - path: crates/finder/tests/console/steering_gap.rs
      status: new
      disposition: add
      assertion: "a frame from before the stack's exposures steers nothing, the view's holding follows the Hold, dropping the Hold releases the tracker found even when no exposure was recorded, nothing is handed over after a release, and a frame is matched by its middle, not its start"
      decision: "spec.md:US4-S3"
    - path: crates/finder/tests/console/steering_gap.rs
      status: new
      disposition: add
      assertion: "a named set point 110 arcsec out, and a binned region's frame, give the worked tracking errors"
      decision: "spec.md:US4-S4"
    - path: crates/finder/tests/motion/steering_gap.rs
      status: new
      disposition: add
      assertion: "thirty subs played late through Steering on a mount 1% fast, with solves from the simulator's truth, start within 5 arcsec, end within 3 arcsec and 0.15 of drift from the 14th sub on, with no rate change during an exposure, the first sub drifting at least 0.6"
      decision: "spec.md:US4-S5"
    - path: crates/finder/tests/motion/steering_gap.rs
      status: new
      disposition: add
      assertion: "Session::steered answers the running tracker with M42's place, and none before Track and after it is turned off"
      decision: "spec.md:US4-S6"
    - path: crates/finder/tests/console/steering_gap.rs
      status: new
      disposition: add
      assertion: "the imaging view has no steering part until the console's steering is first used through Imaging::steering, and then carries its source and its count"
      decision: "spec.md:US4-S7"
    - path: crates/finder/tests/console/steering_gap.rs
      status: new
      disposition: add
      assertion: "a first tracking error of 200 arcsec makes the steering release the tracker, hand nothing over, call no further gap and show far_arcsec until its next release, and one of 100 arcsec steers"
      decision: "spec.md:US4-S8"
    - path: crates/finder/tests/console/steering_gap.rs
      status: new
      disposition: add
      assertion: "an exposure is remembered from when before_exposure answered, not from when it was called"
      decision: "spec.md:US4-S9"
    - path: crates/finder/tests/console/steering_gap.rs
      status: new
      disposition: add
      assertion: "a tracker with no target is handed nothing"
      decision: "spec.md:US4-S10"
    - path: crates/finder/tests/console/steering_gap.rs
      status: new
      disposition: add
      assertion: "the steering keeps no tracker alive: its last Arc dropped, the tracker is gone and no sub is counted"
      decision: "spec.md:US4-S11"
    - path: crates/finder/tests/console/steering_gap.rs
      status: new
      disposition: add
      assertion: "a set point changed mid-stack lets the tracker go once, None puts it back at the sensor's centre, and an unchanged one calls nothing"
      decision: "spec.md:US4-S12"
    - path: crates/finder/tests/layout/main.rs
      status: existing
      disposition: preserve
      assertion: "console's and motion's main.rs hold only their support include and mod lines, with steering_gap added to each"
      decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
    - "the imaging view has no steering part until the steering is first used (spec.md FR-016)"
    - "no stack calls the steering yet: 035's stack behaves as it did"
    - "crates/finder/tests/console/steering_error.rs, US2's test file, passes unchanged: the tracking error, the trait's defaults and the guard on steering.rs's text still hold"
    - "crates/finder/tests/motion/steering.rs and steering_claim.rs, US3's and US9's test files, pass unchanged"
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
    - path: crates/finder/src/imaging/mod.rs
      other_spec: 036-tracking-from-subs
      other_story: US5
      disposition: merge_order
      reason: "US5 depends_on_merged US4, and gives the view its ask"
    open_questions: []
  US5:
    tests:
    - path: crates/finder/tests/console/steering_cap.rs
      status: new
      disposition: add
      assertion: "the turn asked about is the smear, 6 px by default, over the farthest corner's distance as the test computes it, for the whole sensor, binning, a region, a named set point and an odd sensor, and the exposure is halved to fit the limit, not past the camera's shortest, not to under 1 s, and not again when it equals the limit"
      decision: "spec.md:US5-S1"
    - path: crates/finder/tests/console/steering_cap.rs
      status: new
      disposition: add
      assertion: "the exposure first answered, and whether it was cut, are held whatever the limit and the exposure asked for become, over_limit says when the limit is under it and not when it equals it, a release leaves the view with no exposure, and the next call decides again"
      decision: "spec.md:US5-S2"
    - path: crates/finder/tests/console/steering_cap.rs
      status: new
      disposition: add
      assertion: "the smear setting changes the turn, and values outside 0.1 to 20 are refused with the stated words"
      decision: "spec.md:US5-S3"
    - path: crates/finder/tests/console/steering_cap.rs
      status: new
      disposition: add
      assertion: "the view carries the smear, the switch, the two shares, the exposure, whether it was limited or is over the limit, and the running tracker's limit as seconds or no limit, absent when no tracker runs"
      decision: "spec.md:US5-S4"
    - path: crates/finder/tests/console/steering_cap.rs
      status: new
      disposition: add
      assertion: "PUT /api/imaging/steering sets any of the four settings, refuses bad bodies, nulls among them, with 400 and the stated words and changes nothing then, the switch included, refuses a foreign origin with 403, another content type with 415 and a console without imaging with 409"
      decision: "spec.md:US5-S5"
    - path: crates/finder/tests/motion/steering_cap.rs
      status: new
      disposition: add
      assertion: "the tracker's longest exposure equals 015's longest_exposure for the equivalent sensor at the same instant, for a field turning either way and through the trait, and is none for a turn that is not a positive number and after a stop"
      decision: "spec.md:US5-S6"
    - path: crates/finder/tests/console/steering_cap.rs
      status: new
      disposition: add
      assertion: "with the switch off nothing is claimed, tuned or handed over while the cap still applies, and the switch alone marks the steering used"
      decision: "spec.md:US5-S7"
    - path: crates/finder/tests/motion/steering_cap.rs
      status: new
      disposition: add
      assertion: "the tracker's longest exposure is none at the instant its target is due east, and a number again 10 s later"
      decision: "spec.md:US5-S8"
    - path: crates/finder/tests/console/steering_cap.rs
      status: new
      disposition: add
      assertion: "the switch turned off mid-stack releases the tracker once and keeps the exposure held, no further gap is asked for, and turned on again the next call asks for one"
      decision: "spec.md:US5-S9"
    - path: crates/finder/tests/console/steering_cap.rs
      status: new
      disposition: add
      assertion: "the exposure is decided from the limit of the hour ahead, 15 s where the limit now allows 30, and the view shows the limit now"
      decision: "spec.md:US5-S10"
    - path: crates/finder/tests/motion/steering_cap.rs
      status: new
      disposition: add
      assertion: "the tracker's limit for the hour ahead is the smallest of 015's answers at each minute of it as the test computes them, under its limit now before a transit, the same through the trait, and none for a turn that is not positive and after a stop"
      decision: "spec.md:US5-S11"
    - path: crates/finder/tests/console/steering_cap.rs
      status: new
      disposition: add
      assertion: "view_for with an ask answers the limit and what a stack started now would take while a tracker runs, used or not, and neither with none; view stays None until used; and the console's view carries both"
      decision: "spec.md:US5-S12"
    - path: crates/finder/tests/console/steering_cap.rs
      status: new
      disposition: add
      assertion: "the two shares go to the tracker before every gap, 0.5 and 0.3 until set, values outside 0 to 1 are refused with the stated words, and the view carries them"
      decision: "spec.md:US5-S13"
    - path: crates/finder/tests/motion/steering_cap.rs
      status: new
      disposition: add
      assertion: "a tuned tracker takes its tuned shares of a drift and of an error, and a rate share of 0 changes no rate"
      decision: "spec.md:US5-S14"
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
    - "motion/steering_gap.rs, US4's, passes unchanged: its limit for the hour ahead, about 103 s at 6 px, is over its 5 s exposure"
    - "motion/steering.rs and steering_claim.rs pass unchanged: an untuned tracker uses 0.5 and 0.3"
    checks:
      development:
      - "cargo nextest run -p finder --locked --test console steering_cap::"
      - "cargo nextest run -p finder --locked --test motion steering_cap::"
      - "cargo nextest run -p finder --locked --test motion steering::"
      - "cargo nextest run -p finder --locked --test motion steering_claim::"
      - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
      - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
    - crates/finder/README.md
    - crates/finder/src/imaging/mod.rs
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
  US9:
    tests:
    - path: crates/finder/tests/motion/steering_claim.rs
      status: new
      disposition: add
      assertion: "beside finder pointings 15 arcsec noisy, subs with feedback drift at most 0.15 arcsec from the 12th on"
      decision: "spec.md:US9-S1"
    - path: crates/finder/tests/motion/steering_claim.rs
      status: new
      disposition: add
      assertion: "an axis error of 2.1 counts is half taken out in a gap of half a second, on that axis alone, for each axis"
      decision: "spec.md:US9-S2"
    - path: crates/finder/tests/motion/steering_claim.rs
      status: new
      disposition: add
      assertion: "a step against the axis's motion is cut to the axis held still for half a second"
      decision: "spec.md:US9-S3"
    - path: crates/finder/tests/motion/steering_claim.rs
      status: new
      disposition: add
      assertion: "eight idle gaps hand the tracker back and seven do not, the eighth sending nothing, and it stays handed back until a tracking error comes; finder pointings after the lapse change no rate when they come; the error that takes it back sets no rate and forgets what was held, also at the next lapse"
      decision: "spec.md:US9-S4"
    - path: crates/finder/tests/motion/steering_claim.rs
      status: new
      disposition: add
      assertion: "a release hands the tracker back at once, forgets an error not yet applied and lets the next gap claim, even after a lapse"
      decision: "spec.md:US9-S5"
    - path: crates/finder/tests/motion/steering_claim.rs
      status: new
      disposition: add
      assertion: "a steer that is not finite keeps no claim, and one before any gap changes nothing"
      decision: "spec.md:US9-S6"
    - path: crates/finder/tests/motion/steering_claim.rs
      status: new
      disposition: add
      assertion: "a gap before a tracker's first rates answers no step, claims nothing and sends no zero"
      decision: "spec.md:US9-S7"
    - path: crates/finder/tests/motion/steering_claim.rs
      status: new
      disposition: add
      assertion: "a gap nobody waits for claims nothing"
      decision: "spec.md:US9-S8"
    - path: crates/finder/tests/motion/steering_claim.rs
      status: new
      disposition: add
      assertion: "while the subs steer a tracker started through the session, the pointing model's newest sync still advances and the finder's samples still grow"
      decision: "spec.md:US9-S9"
    - path: crates/finder/tests/motion/steering_claim.rs
      status: new
      disposition: add
      assertion: "a step held after a lapse moves an axis 30 arcsec on the sky at most"
      decision: "spec.md:US9-S10"
    - path: crates/finder/tests/motion/steering_claim.rs
      status: new
      disposition: add
      assertion: "after a lapse, under noisy finder pointings, every rate between two gaps is the feed-forward plus the offset the last gap left, the offset never changes, and a gap steps"
      decision: "spec.md:US9-S11"
    - path: crates/finder/tests/motion/steering_claim.rs
      status: new
      disposition: add
      assertion: "a followed push ends the claim at once and forgets the error measured from the old place, and the next gap claims again; an error from before the follow that is handed over after that gap steers nothing, and one measured after it steers"
      decision: "spec.md:US9-S12"
    - path: crates/finder/tests/motion/steering_claim.rs
      status: new
      disposition: add
      assertion: "a followed push under a lapsed claim forgets the step held and lets the next gap claim"
      decision: "spec.md:US9-S13"
    - path: crates/finder/tests/layout/main.rs
      status: existing
      disposition: preserve
      assertion: "motion's main.rs holds only its support include and mod lines, with steering_claim added"
      decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
    - "no file under crates/finder/src changes: each scenario tests US3's tracker and passes on it as soon as it is written"
    - "motion/steering.rs, US3's test file, passes unedited: steering_claim writes the tracker rig out again"
    - "015's and 038's tracking tests pass unchanged"
    checks:
      development:
      - "cargo nextest run -p finder --locked --test motion steering_claim::"
      - "cargo nextest run -p finder --locked --test motion steering::"
      - "cargo nextest run -p finder --locked --test layout"
      handoff: declared_gates
      independent: declared_pipeline
      release:
      - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
    - crates/finder/tests/motion/main.rs
    - crates/finder/tests/motion/steering_claim.rs
    no_write_reason: ""
    shared_writes:
    - path: crates/finder/tests/motion/main.rs
      other_spec: 036-tracking-from-subs
      other_story: US4
      disposition: merge_order
      reason: "US4 depends_on_merged US9, and adds mod steering_gap after mod steering_claim"
    open_questions: []
```
