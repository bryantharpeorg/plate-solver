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
a second, waits while the tracker is not steady (a take-up, the keyhole pass, a first
reversal's measurement, a push by hand, 2 s of settling after the first three, and a
turn that finds an axis stopped) or its limit is under half the exposure held, says why
it waits in a word the tracker gives, and tells the stack to drop a sub taken across a
time the tracker was not steady. Wire it into 035's taker and worker, the taker setting
the capped exposure through the stack's own lease on the camera and putting the
observer's back as the stack ends; show the steering in the imaging view as `steering`,
set its four settings, the switch, the corner smear and the control law's two shares,
with `PUT /api/imaging/steering` or the MCP tool `set_steering`, and give the Imaging
page a block for the readout. Two stories of tests only, cut from the two largest, test
the tracker at the edges of a claim and of a time not steady.

## Technical Context

**Language/Version**: Rust (MSRV 1.96) for US1 to US7, US8's readout and US9 to US11;
TypeScript with React 19 for US8's page.

**Primary Dependencies**: nothing new, and no `Cargo.toml` changes. `finder` already has
`tokio` with `sync` and `time`, `serde` and `axum`; the simulator already has what its
memory needs in `std`.

**Storage**: none. The four settings and the readout are run-only.

**Testing**:
- US1: `crates/simulator/tests/imaging_history.rs` (new).
- US2: `crates/finder/tests/console/steering_error.rs` (new).
- US3: `crates/finder/tests/motion/steering.rs` (new).
- US4: `crates/finder/tests/console/steering_gap.rs` and, for US4-S5 and US4-S6,
  `crates/finder/tests/motion/steering_gap.rs` (new).
- US5: `crates/finder/tests/console/steering_cap.rs` and, for US5-S6, US5-S8, US5-S11
  and US5-S14, `crates/finder/tests/motion/steering_cap.rs` (new).
- US6: `crates/finder/tests/motion/steering_steady.rs` and
  `crates/finder/tests/console/steering_steady.rs` (new).
- US7: `crates/finder/tests/motion/steering_stack.rs` (new).
- US8: `crates/finder/tests/console/steering_readout.rs` and
  `crates/finder/web/src/components/SteeringBlock.test.tsx` (new, Vitest).
- US9: `crates/finder/tests/motion/steering_claim.rs` (new): tests of US3's tracker.
- US10: `crates/finder/tests/motion/steering_unsteady.rs` (new): tests of US6's tracker.
- US11: `crates/finder/tests/mcp/steering.rs` (new), through the MCP door.
- Where a finder test goes is 027's rule: `console` calls the console in process and
  moves no mount (`crates/finder/tests/console/main.rs:2`), `motion` drives the
  simulated mount in paused time (`crates/finder/tests/motion/main.rs:1`). Each module
  is declared by one `mod` line, in alphabetical order, in its binary's `main.rs`
  (`crates/finder/tests/layout/main.rs:156`), and writes its own setup:
  `crates/finder/tests/support` takes no new file
  (`crates/finder/tests/layout/main.rs:127`). A module may use what the existing support
  files already offer: `crate::support::tracker` (the backlash rig, US6),
  `crate::support::goto` (US9-S9, US4-S6, US6-S6) and `crate::support::Scratch`
  (`crates/finder/tests/support/mod.rs:14`). The tracker rig, with its wait for the first
  rates, and the scripted tracker of spec.md are written out in each module that uses
  them, only the parts its own tests use, because an unused helper fails `clippy -D
  warnings` and a story may not edit an earlier story's test file. 035's rigs live in
  its `console` binary, which the `motion` binary cannot import: the stand-in stacking
  rig, `ScriptedSolver`, `wait_stack` and `cards` in `stack_loop`, the scripted camera
  in `stack_camera` and the sim stacking rig in `stack_gate` (035 plan.md § Test rigs,
  since its split of US5). US7 writes the stacking rig of spec.md itself. `mcp` goes
  through the MCP door and can import neither binary's rigs: US11's module builds its
  own console, as 031's and 035's `mcp` modules do.
- No Playwright test (spec owner decision 12).

**Target Platform**: the `finder` console on the console machine. Nothing changes on the
scope node or the scope link.

**Constraints**:
- No change under `crates/scope-node`, `crates/scope-link` or `proto/`; no `unsafe`; no
  dependency.
- No file under `crates/finder/src/imaging/` computes an altitude, an azimuth, a
  parallactic angle or a field rotation, or reads an axis count (the plan document,
  § Mount-neutral by design, rule 1; US2-S6 guards `steering.rs`, and US7-S6 035's
  `stack/service.rs`, the file under `crates/finder/src/imaging/` that US7 edits most,
  each for thirteen names).
- Nothing is sized to one telescope or one sub length (rules 3 and 6): the deadband is
  in the mount's own counts, and the freshness rule, the readout's window, the
  exposures the steering remembers and the gaps the tracker remembers are counted in
  subs. The times this spec fixes, the half-second pulse, the quarter-second poll, the
  2 s a tracker settles for, the hour ahead a tracker's limit is asked for and the 1 s
  an exposure is not halved under, belong to the mount, the gap and the sky and not to a
  sub.
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
  lease it already holds (§ The stack; § What 035 must provide).
- Every rate is sent by the tracker's own task (§ Plan traps 1).
- While a stack holds the tracker, no rate changes between two of its gaps but the
  once-a-second feed-forward, whether the subs steer or have lapsed (§ Plan traps 27);
  the switch turned off (spec owner decision 8) and a stack the steering stands back
  from (spec FR-014) are the two declared exceptions.
- `web/dist` is committed and must be the build of its source (US8).

**Scale/Scope**: one new module, one route, one MCP tool, one page block, about 450
lines added to the tracker by US3, and more by US5 and US6, about 100 to the simulator
and about 90 to 035's stack (probe round 1 measured US3's at +454 −38 lines of
`tracking.rs`, US1's simulator source at +97 −3 and US7's change to 035's stack at +88
−5, ignoring whitespace).

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
├── src/imaging/steering.rs              # US2: new; US4: Steering; US5: the cap, the settings; US6: steady, check; US8: the readout
├── src/imaging/mod.rs                   # US2: pub mod steering; US4: the view's part, Imaging::steering; US5: the view's ask
├── src/imaging/routes.rs                # US5: PUT /api/imaging/steering
├── src/mcp/tools.rs                     # US11: set_steering
├── src/imaging/stack/service.rs         # US7: the taker's and the worker's calls, Dropped::not_steady (035's file)
├── src/imaging/solving.rs               # US7: set_solver gives the steering its source (032's file)
├── src/tracking.rs                      # US3: Steered for Tracker; US5: the two limits, tune; US6: steady, pause
├── src/session.rs                       # US4: Session::steered; US6: tells the tracker of a push (038's track_live_pointing)
├── src/lib.rs                           # US5: the route's doc line
├── README.md                            # US5: the route's row; US7: the section; US8: the readout's keys; US11: the tool's lines
├── tests/console/main.rs                # US2, US4, US5, US6, US8: one mod line each
├── tests/console/steering_error.rs      # US2: new
├── tests/console/steering_gap.rs        # US4: new
├── tests/console/steering_cap.rs        # US5: new
├── tests/console/steering_steady.rs     # US6: new
├── tests/console/steering_readout.rs    # US8: new
├── tests/motion/main.rs                 # US3 to US7, US9, US10: one mod line each
├── tests/motion/steering.rs             # US3: new
├── tests/motion/steering_claim.rs       # US9: new
├── tests/motion/steering_gap.rs         # US4: new
├── tests/motion/steering_cap.rs         # US5: new
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
`tune`, US6 the rest) and add none, so the scripted trackers of US4 to US8 never need a
method they did not have. No doc comment in this file names an axis, the sky's turning
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
/// True from the start until the rate task has sent its first rates. (US6 also sets
/// it for a take-up and the keyhole pass.)
settling: bool,
/// A tracking error handed over and not yet applied.
pending: Option<TrackingError>,
/// A tracking error steers only when its time is later than this console time: the
/// last `follow` (from US6 also the last time the tracker stopped being steady). 0.0
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
numbers are finite and the error's time is later than `fresh_after` (US6: and
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
waiting, so such an error can arrive after the stack's next gap has claimed again. The
recheck of round 1 ran that case and saw a step of 7.5″ where nothing should move (EE2).

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
US6 `follow` also clears `pushed` and calls `mark` (§ The tracker's two answers). The
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
for ever (US6-S10).

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
   cannot say what the mount was last sent. From US6 a pointing while the tracker is not
   steady runs 015's block whole even after a lapse, and the next turn sends its
   proportional rate, 0.6 of the pointing's axis error over its interval. A pointing
   that then arrives steady and lapsed zeroes `proportional` and sends nothing, and the
   rate's time can run out before the next turn. Round 1's fix sent `base` only when
   `proportional` was not zero and `proportional_until` not yet reached, and missed
   both: the recheck of round 1 measured +25.3″/s and +27.8″/s on the azimuth axis still
   in force at the gap's answer for a pointing with no earlier one, where round 1 itself
   had measured +0.632″/s for a pointing 40 s after the one before (D25, EE1). US10-S1,
   US6-S8 and US10-S9 assert the rate in force at the answer, and US3-S11 that a held
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
- while `subs` is `None` and `lapsed` is true (US6: and `control.steady`), runs as it
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
held while a lapsed claim stands; from US6 the keyhole pass is a time the tracker is not
steady, so it runs whole.

The integral is not held: while a stack holds a lapsed tracker the tracking offset stays
as the subs left it. Held and summed, as this trio first had it, one gap added the
changes of all its pointings at once, each made for the same uncorrected error, so the
loop's gain grew with the square of the gap: past about 25 s between gaps it was
unstable, and on a finder scatter of 15″ it moved the offset by 0.2″/s to 0.5″/s a gap,
which spoils the rate the subs had learned and smears the very sub that would take the
tracker back (reanalysis AA1, FF4, from models of this section; round 1's simulator,
with 6 s gaps and the truth as its finder pointings, did not show it). The proportional
term acts once a gap instead of once a solve, and 0.6 of the newest error is taken out
then, between two exposures.

### The tracker's two answers (`crates/finder/src/tracking.rs`, US5 and US6)

```rust
impl Tracker {
    // US5:
    pub fn longest_exposure_s(&self, turn_rad: f64) -> Option<f64>;
    pub fn longest_exposure_ahead_s(&self, turn_rad: f64) -> Option<f64>;
    pub fn tune(&self, step_share: f64, rate_share: f64);
    // US6:
    pub fn steady(&self) -> bool;
    pub fn steady_between(&self, start: f64, end: f64) -> bool;
    pub fn pause(&self) -> Option<&'static str>;
    /// The session says whether the telescope is being moved by hand (038's `moving`).
    pub fn pushed(&self, moving: bool);
}
```

and `impl Steered for Tracker` gains the first six, each calling the inherent one; `pushed` is the tracker's own, for the session, and no part of `Steered`.

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
half the exposure held the steering waits (§ The steering, **Steady**). The other way
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

**Steady** (US6). `Control` gains `steady: bool`;
`unsteady: VecDeque<(f64, Option<f64>)>`, the times, on the console's clock, the tracker
stopped and started being steady, the last 64;
`settle_until: Option<tokio::time::Instant>`; `pushed: bool`; `stalled: bool`; and
`sent: Option<(f64, f64)>`, the rate the task last sent the azimuth axis, in degrees a
second, with the console time it sent it. `settling` is US3's: true from the start until
the first rates are sent. US6 also sets it true at the start of a take-up and of the
keyhole pass, and keeps it true for `SETTLE` after the rates that follow each of the
three:

```rust
/// How long the tube is given to settle after a tracker's first rates, a take-up and
/// the keyhole pass, before the tracker reads steady.
const SETTLE: Duration = Duration::from_secs(2);
```

Wherever `settling` is set true, `settle_until` is set to `None`. At the end of every
turn of the rate task, after its rates are sent, in place of US3's `settling = false`:
when `settling` is true and `settle_until` is `None`,
`settle_until = Some(Instant::now() + SETTLE)`; when `settling` is true and
`Instant::now()` has reached `settle_until`, `settling = false` and
`settle_until = None`. So the tracker is steady again at the end of the first turn to
end at least 2 s after those rates: 2 s after them in paused time on the simulated link,
where a turn takes no time, and between 2 s and 3 s after them on a real one. The settle
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
steady and never on becoming steady: US3-S9 and US3-S10 hand over errors dated before
the tracker's first rates, and US3's file passes unedited after US6. For the same reason
US6 makes `steer` do nothing while `control.steady` is false. `start_with_resets` calls
nothing: the `Control` it builds has `steady: false` beside US3's `settling: true`, and
its list holds `(f64::NEG_INFINITY, None)`, the tracker having been not steady at every
time before it started. `mark` is called:
- at the start of `take_up` and of `keyhole` (`crates/finder/src/tracking.rs:697`,
  `crates/finder/src/tracking.rs:551`), after setting `settling = true`;
- where the rate task opens a measurement (`crates/finder/src/tracking.rs:829`: on the
  simulated link the end-of-turn `mark` comes at the same instant; this one matters
  where the link's round trips take time) and where `settle` closes one
  (`crates/finder/src/tracking.rs:668`);
- at the end of every turn of the rate task, after its rates are sent and the settling
  rule above;
- in `pushed` and in `follow` (**A push by hand**, below);
- where a turn finds an axis stopped (**A stopped axis**, below);
- in `end`, where the state is `off`.

**A push by hand** (US6). `pub fn pushed(&self, moving: bool)`: under the control lock,
nothing when the state is `off`; otherwise `control.pushed = moving` and `mark`.
`follow` sets `pushed = false` and calls `mark`, after US3's lines, and `end` sets it
false. The session calls `pushed` (§ The session). 038 keeps a push in the session and
hands the tracker nothing while it is `moving`, so as this trio first had it the tracker
read steady with a hand on the tube: exposures went on, every sub taken across the push
was kept, and a sub taken after the hand had left and before 038's five still seconds
were up was accepted with the whole push as its error, which the next gaps stepped back
toward the old place at 30″ a gap. With gaps under about 2.5 s apart those steps could
keep the push from ever settling (reanalysis AA2, BB3, CC2, DD3, FF1; read from the
code, not run). Not steady, the claim and the error not yet applied are dropped as for a
take-up, `before_exposure` waits and `check` drops the sub in flight; that the stack
waits while a push is moving is also what 037's run takes for granted. One finder solve
more than 5′ wrong sets 038's `moving` for one pointing, and so costs one sub and the
claim; the offset is kept.

**A stopped axis** (US6). The scope node stops both axes when a rate command fails on
the mount's side, and still acknowledges the command, so the console is not told
(reanalysis FF3; its own handling is another spec's). The rate task already reads the
azimuth axis's state at the top of every turn. Wherever it sends the azimuth axis a
rate, in the turn's loop and in `serve_gaps`, it stores that rate and `clock.now()` in
`control.sent`. When a turn's read answers `moving` false while `sent` holds a rate that
is not zero, the turn sets `stalled = true`, calls `mark`, and sets the start of the
entry `mark` has just pushed back to `sent`'s time; at the end of that turn, after its
rates are sent, it sets `stalled = false`, before the settling rule and `mark`. So the
tracker reads not steady from the last azimuth rate it sent to the rates that start the
axes again, about a second: the sub in flight is dropped, the claim is taken back at the
next gap, and no drift rate is measured across the stop. It does not settle afterwards:
the owner's answer is that turn alone. A state read the node could not make answers the
last counts with `moving` true, so it raises no false alarm, and an axis that was sent
zero is not looked at.

**Why it is not steady** (US6). `Control` gains `cause: &'static str`, written wherever
`settling` is set true: `"starting"` in `start_with_resets`, `"reversing"` at the start
of `take_up` and `"overhead"` at the start of `keyhole`.
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
instant does not touch it, as `steady()` read at that instant is true. Round 1 built
both bounds inclusive, and a stack started at the instant of a tracker's first rates
lost its first sub (D38). With 2 s of settling the stack's own quarter-second poll can
fall on the instant the settling ends, whichever instant the stack began at, so the end
is not counted and the stacking rig of spec.md US7 no longer waits a second more. US6
changes the test in `serve_gaps` step 1 from `settling` to `!control.steady`, which is
the same before the first rates and also turns away a gap served while a measurement is
open, while the tracker settles, during a push and in a turn that found an axis stopped:
it answers no step and claims nothing. US3's scenarios pass under either test, because
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
steady, and the sub it crossed is dropped (US6-S4). US3's scenarios pass under either
test, because none of them has a time the tracker is not steady once its rig's wait is
over.

### The steering (`crates/finder/src/imaging/steering.rs`, US4; US5, US6 and US8 add to it)

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
    // US6:
    /// For every sub, before it is solved: whether the tracker was steady for it.
    pub fn check(&self, frame: &SolveFrame) -> Verdict;
    /// Whether a `before_exposure` is waiting, for the tracker to be steady or (US6) for
    /// its limit to rise. 037 reads it.
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

`Clock` is the console's (`crates/finder/src/session.rs:24`). Each story adds only the
fields and items named for it; a field written and never read fails `clippy -D
warnings`. The state behind the lock, a `std::sync::Mutex`, holds: whether the steering
was used; the set point; `subs` and `last`; `far`, the length of the first tracking
error of a stack the steering stands back from, or none; the tracker `before_exposure`
last found, as a `Weak`; how many `Hold`s are alive; a `VecDeque` of at most
`EXPOSURES_KEPT` **exposure records**, newest last; and, by story, the settings, the
exposure held with the ask it was decided for, the pause's word, the count and the
window.

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
    // US8: the sum of what the gaps served in this call answered.
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
call keeps its ask, for the view. And the answer is the exposure held (below). US6 puts
a wait before step 3 and a check after it
(below). US8 keeps in the record the sum of what the gaps served in this call answered.

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
from US6 the wait and `check`, go on as they do with the switch off. Two references hold
the telescope. 015 keeps the finder camera's pointing, read at the aim point in force,
on the target; the subs keep the set point on it. The two differ by the aim point's
error, which 032 measures and holds back for as long as Track is on (032 FR-036). Where
that is more than 038's 5′, the subs' own centring carries the finder camera's pointing
past 038's threshold: the session reads a push nobody made and follows it, the target
moves, the subs find the same error again, and the telescope walks off the target in
hops. The simulator's aim point is exact, so no rig of round 1 met it (reanalysis BB5;
read from the code, not run). 180″ is under 038's 300″ with room for the finder camera's
scatter, and by the time a stack's first error is known no step has been made: its first
gap claims and moves nothing. The tracker is then let go for the rest of the stack, and
its own corrections run as 015 has them, inside exposures too.

**Letting the tracker go.** One private function, which three callers share. Under the
lock it takes the tracker last found, upgraded, and sets `subs` to 0 and `last` to
`None` (US8: and empties the window); once the lock is released it calls that tracker's
`release`. It keeps the tracker last found, the exposure records, the exposure held,
`far` and the count of drops: the stack goes on, capped and checked, and its next gap,
where one is asked for, claims afresh. The first tracking error after it is then a first
error once more: no drift rate is measured across the break, and `FAR_ARCSEC` is asked
again. Its callers are `set_set_point` with a set point that differs from the one held
(US4), `set_on(false)` (US5), and the wait for the limit (US6, **Steady**). Without it a
claim outlives what should have ended it: no gap is asked for, a claim lapses only at a
gap, and while a claim stands the finder camera's pointings change no rate either, so
nothing steers at all. As drafted `set_on` only stored a flag, and a switch turned off
during a stack, which is how the field check with and without the feedback would be run,
compared the feedback with a mount nothing corrected (reanalysis AA5, CC1, DD5, FF2;
AA12 for the set point).

**`set_set_point`** stores the set point, and lets the tracker go when it differs from
the one held: the tracker's last applied error was measured from the old place, and a
set point moved by 3 px between two subs would read as about 1″/s of drift. `None` is
the sensor's centre; 037 names it so as a run starts its stack, before any gap, where
there is usually nothing to change and nothing is called (spec US4-S12).

**`hold`** counts one more `Hold` alive (US6: and sets `dropped_not_steady` to 0, a
stack having begun). `Hold`'s `Drop` counts one less and calls `release`. The view's
`holding` is whether any is alive; the page uses it to tell a stack that has stopped
steering from no stack at all (§ The page).

**`release`** takes the tracker last found, upgraded, and calls its `release`; then
forgets it, empties the records and sets `subs` to 0, `last` to `None` and `far` to
`None` (US5: and forgets the exposure held and the ask kept with it; US6: and sets
`steady` true and `waiting` false and forgets the pause's word; US8: and empties the
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
pass 1 s, or the camera's shortest where that is longer: as drafted it stopped only at
the camera's shortest, and a 30 s setting near the zenith became subs of a quarter of a
second, 130 frames of 31 MiB for each 30 s asked for. An exposure the observer set under
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
  made, like `steered_by_subs`. As drafted it was kept from the latest
  `before_exposure`, so before a stack the page showed no limit, and after one, or with
  Track off, a limit an hour old (reanalysis FF17);
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

**Steady** (US6). `POLL` is 250 ms of tokio time. The state's `steady` is true and its
`waiting` false when the steering is built, and they are that in the view until a wait
changes them. In `before_exposure`, with a tracker: while `!tracker.steady()`, set
`steady` false, `waiting` true and `pause` to `tracker.pause()` in the state, sleep
`POLL`, and read `source()` again (a tracker that ended or was replaced is taken as
found: none sets `steady` true and `waiting` false, forgets the pause's word and answers
as step 2). Then the limit: whenever an exposure is held and
`tracker.longest_exposure_s(turn(&ask))` is a number under half of it, the call goes no
further. On the first such round it lets the tracker go (**Letting the tracker go**); it
sets `waiting` true and `pause` to `Some("limit")`, leaving `steady` as the tracker has
it, sleeps `POLL` and begins again from the look at `source()`. It looks before it asks
for a gap and, at a stack's first call, once more when the cap has decided the exposure,
before it answers. A stack never stacks subs smeared by more than twice the smear
chosen, and goes on by itself once the limit has risen, which after a transit it does;
meanwhile the tracker's own corrections steer, as they do with the switch off (spec
owner decision 2; reanalysis FF5). A limit equal to half the exposure does not wait, and
US5's scenarios hold no case that would (spec US5-S1). Then the gap, where step 3 calls
one. While its answer is awaited the steering goes on looking: every `POLL` it upgrades
the tracker last found, reads `steady()` and lets the `Arc` go again, and it sets
`steady` false and `waiting` true while that reads false, and back when it reads true. A
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
which stays in that stack's view too. As drafted it counted since the console started,
and from a console's second stack on it disagreed with the count shown beside it
(reanalysis FF18, whose own fix cleared it in `release`; the two would then have
disagreed after every stop).

**The readout** (US8). The state keeps the last 10 errors handed over since the last
release, each with its frame's `exposure_s` and its solve's `arcsec_per_px`. With n
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

### The session (`crates/finder/src/session.rs`, US4; US6 adds the push)

```rust
/// The running tracker as the stack sees it (036): `None` while Track is off, armed or ended.
pub fn steered(&self) -> Option<Arc<dyn crate::imaging::steering::Steered>>;
```

`self.tracker()` (`crates/finder/src/session.rs:773`), kept when its
`state().state != "off"`. 037 asks it `steady()` and `steady_between` over a window that
ends now.

**The push** (US6). In 038's `track_live_pointing` the session tells the tracker it read
in its step 1, with its own lock released: `tracker.pushed(true)` wherever it makes the
run's push `Push::Moving` (its steps 4 and 6, and its step 7 where a still stretch
starts again, where the call changes nothing), and `tracker.pushed(false)` wherever a
`Moving` push ends with no follow, before it hands that pointing over (its steps 2 and
5). `Tracker::follow`, which step 7 calls, clears the flag itself, and so does the end
of tracking, which `stop_tracking` brings. Nothing else of the session changes, and
`TrackView` gains nothing: 038's `track.push` already says `moving`.

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
`ExposureAsk` the console's camera and settings give, built as US7's taker builds its
own (§ The stack: `wanted_s` and `shortest_s` from `settings.exposure_ms` and
`camera.exposure_ms.min`, the sensor, the region and the binning), and `None` while the
view has no `camera` or no `settings`. So from US5 a console whose steering has a source
carries `steering` whenever a tracker runs and its camera is ready, with
`longest_exposure_s` and `would_take_s`; until US7 gives the console's steering its
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
being a refusal's words; US11's tool calls the same function, so that the tool and the
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

### The stack (`crates/finder/src/imaging/stack/service.rs`, US7)

035's stack runs two tasks (035 plan.md § The stack on the console): the taker,
`Stacker::take_subs`, which calls `lease.take("sub")` back to back on the camera's
lease, which it owns and which drops when it returns, and the worker, `Worker::run`,
which takes each saved sub off the queue and awaits `Worker::process` for it. US7 adds
to both and to `Dropped`, and nothing else to the file. The places below are named as
035's `stack/service.rs` has them on `main` since its US8 landed (read on 2026-11-19;
round 1's probe built on an earlier 035, whose worker counted in numbered steps); each
is given its line when the trio is re-anchored (§ Prerequisites 3; § What 035 must
provide).

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
`LastView` does not carry it. Round 1's words for this drop, "counted inline, as the
settings drop is, and not through `Dropped::count`, whose match sends a key it does not
name to `unprocessed`", were true of the probe's 035 and are false of the one that
landed (reanalysis BB1, CC10, DD4, EE3; BB2 for the place of `accepted`).

**The guard** (spec US7-S6). The file, as US7 leaves it, holds none of the thirteen
names US2-S6 reads `steering.rs` for: `ephemeris`, `alt_az`, `parallactic`,
`counts_per_revolution`, `AxisInfo`, `MountClient`, `azimuth`, `altitude`,
`field_rotation`, `hour_angle`, `meridian`, `backlash` and `keyhole`. The last seven
were added on 2026-11-19: with six, a file could have said `azimuth` or `keyhole` and
passed (reanalysis FF20). Nothing US7 adds names one: the taker and the
worker reach the tracker only through `Steering`, `ExposureAsk` and `Verdict`, in sky
and sensor terms. § Prerequisites 3 checks that 035's landed file holds none either, so
that the guard rests on US7's diff.

### What 035 must provide (035's files; read as landed before the flip)

031 has landed with the camera's lease, and 035's eleven stories have landed on it: its
US9 and US10 on 2026-11-19, after § The stack was read against `main`. They wrote the
page, the README and the MCP tools, and `stack/service.rs` is still as its US8 left it.
This section lists everything this spec assumes of 035, and through it of 031's lease,
so that the re-anchoring has one list to check (§ Prerequisites 3). 035's Prerequisites
10 and 037's plan.md point here. 035 says that "a sub taken at an exposure that changes
within a stack is not offered" (035 plan.md § Prerequisites 10); this spec does not ask
for one. It changes the setting once, before a stack's first sub.

What US7 and US8 rely on:

1. **The taker's loop** has one place before each `lease.take("sub")`, after its look at
   the stopping mark, and the taker, `Stacker::take_subs`, reaches the console's imaging
   state as `self.imaging`, from which `self.imaging.steering()` comes; the worker
   reaches it as `self.stacker.imaging`.
2. **The stopping mark** (035 plan.md § The stack on the console, **The stopping
   mark**): one `tokio::sync::watch` channel of `bool`, made with the run, `false`
   until the stack is ending and `true` from then on, set once by whoever ends the
   stack; the taker and the worker each hold a receiver (the taker's by value: US7 makes
   it `mut`, § The stack). The taker's wait races it as
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
   before the lease is dropped" (the seam, 2), which is where § The stack does it.
6. **The worker**: `Worker::process` builds the sub's `SolveFrame` and then makes the
   settings test, which answers `Err("settings")` for a sub whose `exposure_ms` differs
   from the reference sub's by more than 1 ms, before it sets `last`; it answers
   `Ok(Accepted { frame, solve, .. })` for a sub it accepts. `Worker::run` counts the
   outcome, calls `note_solve` for an accepted sub with no stack lock held, and then may
   make a picture.
7. **`Dropped`** derives `Default`, writes every key always and in order, and has
   `total()` and `count(outcome)`, whose match names every outcome the worker answers
   and is `unreachable!` for any other; US7 adds `not_steady` to the struct, to
   `total()` and to that match (§ The stack). `stack.json`'s `subs` writes the outcome a
   sub was counted under, and `LastView::outcome` that of a sub that reached the solve.
   No test of 035 counts the keys, compares a whole `dropped` object with a literal, a
   fixed list of keys, a golden file or a schema that lists its keys (a comparison of
   two serializations of the same `Dropped`, as
   `stack_console.rs` makes between `stack.json` and the view, is not one), or builds
   `Dropped { .. }` without `..Default::default()` (035 spec.md § User Scenarios: "a
   test never asserts how many there are").
8. **`StackBlock.tsx`** builds its `Dropped {sum}: {parts}` line from the counts above
   0, in `StackDropped`'s order, each with its words. Its filter of the counts above 0
   reads an optional count with `?? 0` once `not_steady` is optional (§ The page).
9. **`Stacker::start`** refuses a camera that is not `ready`, so the taker's first read
   of the view finds `camera` and `settings`.

What they change in 035's two files, and nothing else:

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

US8 adds the readout's four keys to that list of the `steering` part's keys (§ The
steering, **The readout**), and US11 the tool's lines to the README's list of tools
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

Its words, exactly, with `s = view.steering`. They were reworded on 2026-11-19: as
drafted one sentence said `Track is off` while Track was armed or had ended by itself
with a stack still exposing, one covered four states of which only one needs the
observer, and a pause gave no cause and no end (reanalysis FF16).
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
  (`crates/finder/web/src/components/TrackControl.tsx:62`), only with `s`:
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
  could be typed; set once, it would never show a change made elsewhere. Round 1 saw
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

One struct that only describes the tool's input, beside 035's `StackingArguments` and
written as it is, with
`#[expect(dead_code, reason = "schema only; SteeringEdit parses the actual arguments")]`:

```rust
#[derive(rmcp::schemars::JsonSchema)]
struct SteeringArguments {
    on: Option<bool>,
    smear_px: Option<f64>,
    step_share: Option<f64>,
    rate_share: Option<f64>,
}
```

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
  3. `crate::imaging::routes::apply_steering(&imaging, edit)` (§ The route): its `Err`
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
- **The tracker with no stack.** `Tracker::new`, `start`, `start_with_resets`, `stop`,
  `state` and `pointing` keep their signatures, and `State` gains no field
  (`crates/finder/src/tracking.rs:58`): 015's tests build and read them
  (`crates/finder/tests/motion/tracking/pi_loop.rs:184`,
  `crates/finder/tests/support/tracker.rs:194`). With no `gap` ever called nothing is
  claimed, `Shared::pointing` runs whole, and every rate is what it was: the tests that
  compare each rate with the feed-forward rate
  (`crates/finder/tests/motion/tracking/pi_loop.rs:267`) and the order of a reversal's
  commands (`crates/finder/tests/support/tracker.rs:285`) pass unchanged.
- **038's push.** `track_live_pointing` keeps its steps, its three push states and the
  `track.push` view (038's tests in `crates/finder/tests/motion/tracking/`). US6 adds
  the calls of `Tracker::pushed` and changes no branch of it; with no stack holding the
  tracker, one that reads not steady during a push corrects, follows and reports as
  before.
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
  listing is 031's too, and 031's MCP test asserts its words: US11 leaves it as it is.
- **The imaging view without steering.** `steering` is skipped until the steering is
  first used. No test of 030, 031, 032 or 034 starts a stack or calls this spec's route, so
  every body they read is what it was. From US7 on a stack uses the steering, and the
  imaging view of a console that ran one carries `steering`: 035's tests read a view key
  by key (035 spec.md § User Scenarios, Comparing) and are not disturbed. From US7 on
  the view of a console that tracks with its camera ready carries it too, used or not
  (§ The steering, **`view_for`**). 032's `imaging_track.rs`, the one landed test that
  tracks with an imaging solver, reads its solve and no whole view; § Prerequisites 3
  checks that no landed test compares an imaging view, or the session's `imaging`
  object, whole on a console that tracks.
- **035's stack with no tracker.** 035's rigs do not track. `before_exposure` then
  answers at once with the observer's exposure, `Lease::set` is not called and no
  setting changes, `check` keeps every sub and `accepted` hands nothing over (spec
  US7-S4): every count and every sub of 035's scenarios is what it was, with
  `dropped.not_steady` 0. 035 says `dropped` has eleven keys, each always written, that
  its tests sum whatever keys `dropped` holds, and that "a test never asserts how many
  there are, since a later spec adds reasons" (035 spec.md § User Scenarios); a
  twelfth key at 0 disturbs neither. § Prerequisites 3 checks the landed tests for a
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
  `#[non_exhaustive]` and tests read it key by key. `Steered`'s required methods are
  fixed in US2: the scripted trackers of US4 to US8 implement them, so no later story
  adds a required method, only bodies for the provided ones. `ExposureAsk` and `Go` have
  every field from US4 on, and `before_exposure`, `accepted`, `hold`, `release`,
  `set_set_point` and `view` keep their signatures: US5 adds `view_for` beside `view`
  and does not change it, because US4's tests call `view()` and no story edits an
  earlier story's test file. A case of an early story asserts nothing a later story
  changes: US5's cases hold no limit under half the exposure they answer, which US6
  would wait on, and every test file passes unedited under every later story (§ Plan
  traps 22 and 34).
- **What 037 reads.** Five things, each kept as it is here (037 plan.md § What 030 to
  036 offer, § Prerequisites 2):
  - `Steered::steady()` through `Session::steered()`;
  - `Steered::steady_between(now - reach, now)` through the same, `reach` being the
    larger of 10 s and the exposure: false for a window that touches a time from a
    tracker's start to 2 s after its first rates, a take-up, a keyhole pass or a
    measurement with the 2 s that follow the first two, a push the session reads as
    moving, or a turn that found an axis stopped; and true for a window that begins at
    the instant such a time ended (spec US10-S3 pins the window after a start, US10-S6
    after a take-up, US10-S7 across the keyhole pass, US6-S6 across a push);
  - `Steering::waiting()`, or the view's `steering.waiting`: true while a
    `before_exposure` waits, for the tracker or for its limit, the view's
    `steering.pause` saying which;
  - `Steering::set_set_point(None)`, called as a run starts its stack (spec US4-S12);
  - the view's `steering.far_arcsec`, present while the steering stands back from a
    stack that began more than 180″ from its set point.

  What 037 may not lean on: that the steering brings in a target stacked off the centre.
  Beyond 180″ nothing steers it; inside, a step against an axis's motion is at most that
  axis's rate times half a second a gap (spec FR-008).

## Decisions and the alternatives rejected

| Decision | Rejected | Why |
|---|---|---|
| A step in the gap and a rate from the drift between subs | 015's proportional-integral law with other gains | Its proportional term is a rate held until the next solve: a 2″ error is about 1″ of deliberate drift in the next sub. Its integral winds up while a first offset of tens of arcseconds is taken out, and unwinds over several smeared subs. Here the step takes the offset and the rate takes only the drift rate, so the two never work against each other on a mount that makes the step asked for; for one that does not, the next two rows. |
| The rate from the difference of two errors, less the step made between | The rate from the error itself | The difference measures drift whatever the offset; the tracker knows the step it made. |
| No rate change when the drift rate is over twice the offset's limit, and at most 0.1″/s of change in one gap | Always; the jump test alone | A jump that large is a wrong solve, a gust or a bump, not a rate error the offset could hold. But the test is of a rate, so the jump it lets through grows with the sub length: 9″ at 6 s between subs, 47″ at 31 s. A smaller one is believed, and 0.3 of it went into the offset; 0.1″/s a gap bounds what one bad error costs at any sub length, and a 1% rate error, whose first change is 0.06″/s, converges as before (spec owner decision 4, changed 2026-11-19). |
| No rate change from a pair of errors with more than two deadbands of steps between them | Believing the step asked for was the step made | The simulator makes every step exactly. A real pulse is two rate commands half a second apart over a serial link, and a 10% error in a 30″ step read as 0.5″/s of drift, inside the jump test. The rate now waits until the steps are small (spec owner decision 4, changed 2026-11-19). |
| A step booked for the time its pulse really lasted | A flat half second | On the simulated link a pulse lasts exactly 0.5 s. On a real one it lasts as long as its two commands take as well, and the unbooked part would be read as drift. |
| The deadband's 1″ and the step's 30″ as angles on the sky | Angles of the axis | An axis angle is worth less sky toward the mount's pole: 30″ of azimuth is 5″ of sky at 80° of altitude, and 1″ of it is under any solve's noise (spec owner decision 5, reworded 2026-11-19). |
| A deadband of two counts or 1″ on each axis's step | One figure in arcseconds; one count | A step of under a count cannot be commanded; half of two counts is one. 1″ keeps a fine-geared mount from chasing solve noise. The tracker reads the counts from the mount, so nothing is sized to one. |
| The step as a half-second pulse inside the gap | A proportional rate over the next exposure | Nothing but the feed-forward then changes while the shutter is open, however late the next gap is. |
| Gaps served by the rate task | `gap` sending rates itself, under a lock | The scope node accepts a rate that arrives after a Stop. Only the rate task is ended before Stop is sent (`crates/finder/src/tracking.rs:885`). |
| Freshness counted in gaps | An age in seconds | The plan document's rule 6; a sub may last minutes on another mount. |
| The finder camera changes no rate while the subs steer | A finder error over some bound takes the tracker back | Its scatter is the fault being removed; a real loss of the target shows as subs that do not solve, which hands the tracker back after eight gaps. |
| The steering stands back from a stack whose first tracking error is over 180″ | Steering it in; holding the field wherever the stack began | The finder camera holds the target on its aim point and the subs on the set point. Where they differ by more than 038's 5′, the subs' centring reads as a push by hand and Track follows it, again and again. 180″ leaves room under 300″ for the finder camera's scatter. Holding the field where it began would change what `set_set_point(None)` means for 037 (spec owner decision 7, added 2026-11-19). |
| Eight gaps with no tracking error before the claim lapses | Three; counting sub verdicts | 035 keeps four subs waiting and one in work, so with a slow worker errors come several gaps apart with nothing dropped; at three the claim lapsed between every two and no drift rate was ever measured. Counting the worker's verdicts exactly needs a third call from 035's worker and more code in the largest story (spec owner decision 6, changed 2026-11-19). |
| Not steady drops the claim | Steering through a take-up | 015's handling of a reversal and of the keyhole is tested as it stands; the subs come back once it is over. |
| A push the session reads as moving is a time the tracker is not steady | Steering stopped and exposures going on; as drafted, with 037 ending its run on any push | 038 leaves a push alone until it has settled, and a running stack did not: it stepped the telescope back at 30″ a gap. Not steady, one mechanism pauses the exposures, drops the smeared sub and the claim, and gives 037 the wait its own design assumes (spec owner decision 17, changed 2026-11-19). |
| 2 s of settling after a tracker's first rates, a take-up and the keyhole pass, kept by the tracker | None; a look-back in the steering's wait | Steady was commands acknowledged, not a tube at rest. Kept by the tracker, the page, 037 and `steady_between` all see the same time; a wait after a large step in a gap was not taken (spec owner decision 17, changed 2026-11-19). |
| The tracker reads not steady for a turn that finds the azimuth axis at rest after a rate that was not zero | A field check only | The scope node stops both axes on a failed rate command and still acknowledges it; the trailed sub was kept and its error steered. The node's own fix is another spec (spec owner decision 17, changed 2026-11-19). |
| An error measured before a follow, or before the tracker stopped being steady, is dropped by its time | Forgetting only what is pending at that instant | 035's worker hands a sub over one or more subs late, so the error can arrive after the claim has been taken again. |
| The cap as a turn of the field in radians | Passing pixels and a sensor size to the tracker | The tracker then needs nothing of the camera, and an equatorial tracker answers "no limit" with no arithmetic. |
| Halving the exposure | Following the limit to the millisecond | 034 matches a master dark within 1 ms of the sub's exposure; a few exposures can each have one. |
| A route of this spec's own | Keys of 031's settings | 031's tests pin the settings body and the MCP tool to nine keys. |
| `Steering` as a part of its own, with a source closure | The stack calling the tracker | One place holds the set point, the settings and the readout; a test gives it a scripted tracker; 037 reads it. |
| The simulator remembers its telescope's direction | The simulated solver keeping its own record | The truth is the simulator's to say; 032's adapter and every later test then need no change. |
| Count steps as a setter | A field of `MountConfig` | Tests build `MountConfig` by literal. |
| Each error judged against the gaps since its mid-exposure | Applying each error as if it were measured now | 035's worker solves a sub while the next is exposing, so every error is at least one gap old. Applied as fresh, each offset is stepped out twice and the rate loop runs with a delay, which rings. Remembering the last 16 gaps costs a small list. |
| Only an accepted sub steers | Every solved sub | The plan document's own words. A solve 035 refuses never reaches the mount; the cost, that a sub dropped as soft steers nothing, is spec owner decision 15. |
| `check` before the solve, `accepted` once the worker has accepted the sub, right after `note_solve` | One call after the solve | A sub taken across a take-up is smeared: solving it costs time and would move the stack's own hint. The tracking error must wait for the stack's verdict, and must not wait for the picture 035 may render after it. |
| The exposure decided once and held | Following the limit sub by sub | 035 drops a sub whose exposure differs from the reference sub's, and a take exposes for the console's setting. |
| The cap applied by changing the console's exposure setting through the stack's own lease, and put back through it as the stack ends | A second exposure parameter on the take; 034's `Calibration::restore_exposure` | 031's take and its refusals are pinned by 031's tests, and 035's seam for 036 has a stack's exposure set through the taker's lease (035 plan.md § Shapes that existing tests pin, **The seam for 036 and 037**, 2). `restore_exposure` called `Imaging::set`, which the stack's own lease refuses, so the subs would have been taken at the observer's exposure; 034 has since removed it (034 A1). |
| A stack that cannot set its capped exposure ends with `camera` | Taking the subs at the observer's exposure and saying `over_limit` | The view would show the exposure held while longer subs were taken; and a refusal there means the camera is no longer ready. |
| `limited` decided with the exposure and held with it | Comparing the held exposure with each call's wanted one | A later call may ask for less than is held; whether this stack's exposure was cut does not change with what is asked later. |
| A lapsed claim is taken back only once a tracking error has come | Taking the tracker back at the next gap | While subs are dropped, the next-gap rule leaves the finder camera steering during one exposure in nine and nothing steering during the other eight. This one hands the tracker to its own corrections whole (spec owner decision 6). |
| After a lapse, the finder camera's step held for the next gap and applied there, and the tracking offset left as the subs set it | 015's corrections at once, as written; the integral's changes held too and added up | The settled row says rate corrections change only between exposures, and a stack that has lapsed is still exposing: a sub taken then may be accepted and is the one that takes the tracker back. Held, every sub's exposure sees only the feed-forward and the offset. The integral is not held: summed over a gap's pointings its gain grows with the square of the gap, it is unstable past about 25 s between gaps, and it writes the finder camera's scatter over the rate the subs learned (spec owner decision 6, changed 2026-11-19). |
| A gap served under a lapsed claim always sends each axis its rate | Sending only when the offset changed, or when `proportional` says a rate is in force | `Control` cannot say what the mount was last sent: a later pointing can zero `proportional`, and its time can run out, with the rate still on the mount until the next turn. The recheck of round 1 ran the rule with the whole workspace passing. |
| Not steady runs 015's corrections whole, even after a lapse | Holding them through a time the tracker is not steady | Gaps are turned away while the tracker is not steady, so a held correction would wait for it to end, and 015 closes a first reversal's measurement from its own corrections, which would then never come (spec owner decision 17). No sub the stack keeps is exposed then. |
| The taker puts the observer's exposure back as it returns | 035's ending, after the taker has returned | The taker owns the lease, which drops as it returns: after that there is no lease to set through, and another holder may already have the camera. 035's seam says the same. |
| A gap before a tracker's first rates is turned away | Serving it like any other | `direction` is not set until the task's first turn, so `commanded` would send both axes zero. From US6 such a tracker is not steady and the same gap claims nothing, so the two stories would disagree about every scenario that began with it. |
| Not steady while a first reversal's backlash is measured | Steady, with the subs steering through it | 015 closes that measurement from the finder camera's pointings and its own corrections; changing what steers during it would need 015's backlash tests proven again (spec owner decision 17). |
| The steering keeps weak references to trackers | Strong ones | Dropping a `Tracker` stops the mount (`crates/finder/src/tracking.rs:112`); a record must not put that off. |
| A sub is matched to its exposure by its mid-exposure time | A ticket carried through 035's queue | It needs no change to 035's hand-over, and half an exposure of margin either side is far more than the clocks' mapping error. |
| `source` says `tracker` | `finder` | The plan document's rule 5: another telescope may have no finder camera. |
| The exposure decided from the tracker's shortest limit of the hour ahead | The limit at the instant the stack starts; the wait alone; ending the stack with a reason of its own | A stack holds one exposure for life, and before its transit a target's limit only falls: a stack asked for 30 s on M31 an hour before transit held 7.5 s and trailed 12 px at transit, at the default smear. An hour, sampled each minute, is what 015's own forecasts look at (spec owner decision 2, changed 2026-11-19). |
| No halving to under 1 s | Halving down to the camera's shortest | Near the zenith a 30 s setting became subs of a quarter of a second, 130 frames for each 30 s asked for, most of them dropped as `behind`. |
| A wait while the limit is under half the exposure held, the tracker let go for it | Stacking on with `over_limit` shown; ending the stack | The stack never stacks subs smeared by more than twice the smear chosen, and goes on by itself once the limit has risen, which after a transit it does. Let go, the tracker's own corrections steer through a wait that can last many minutes; held, nothing would. |
| A default smear of 6 px | 2 px | The 2 px was sourced nowhere and cut a 20 s setting on M42 to 5 s. The research note gives 28 px of coma in the same corner, a published 7 px, and the owner's 20 s to 30 s subs with this camera (spec owner decision 1, changed 2026-11-19). |
| The limit asked as the view is made, with what a stack started now would take | Kept from the latest `before_exposure` | Kept, it was absent before a stack, when it decides which darks to take, and stale after one. |
| `view_for` beside `view` | Changing `view`'s signature; keeping the ask in the steering | US4's tests call `view()`, and US5 may not edit them. The ask is the camera's and the settings', which `Imaging` holds. |
| Why a tracker is not steady as the tracker's own word, through `Steered::pause` and the view | One sentence for every pause; the steering naming the causes | The causes last from under a second to a quarter of an hour, and only a push needs the observer. The words are a mount's; kept in the tracker and the page, the imaging path names no axis. |
| A word of its own, `stalled`, for a turn that finds an axis stopped | No word, the page's general sentence serving | The other pauses each say why; this one is the only sign on the page that the node stopped the mount. It lasts under the steering's poll, so it is seldom seen, and the README and the node's journal say the rest. |
| `dropped_not_steady` set to 0 where a stack takes its `Hold` | Since the console started; cleared in `release` | It sits beside 035's count of the same drops, which is one stack's and stays after the stack has ended. Cleared in `release`, the two would disagree after every stop. |
| The two shares as run-only settings, handed to the tracker before every gap | Constants in code; a second argument of `steer` | The telescope is two hours from where a console is rebuilt, and the switch alone compares all of the feedback with none. Handed before every gap, a tracker a GoTo has just made has them by its first step, and `steer` keeps its shape. |
| The switch turned off, a set point that changes and the wait for the limit each let the tracker go | Storing the flag or the set point only | A claim lapses only at a gap, and none is asked for then: the claim stood, the finder camera's pointings changed no rate under it, and nothing steered. |
| `set_steering` as a story of its own, built last | A part of US5; a part of US8 | Spec owner decision 11 named it "one more story after US5". US5 is the second largest story with this fold's cases, and a tool, its schema and a module in a third test binary are about 9 KB more. |
| `get_imaging_camera`'s description left as 031 has it | Naming the stack and the steering in it | 031's MCP test asserts its words, and no existing test is edited. |
| US3 and US6 cut by tests, into US9 and US10 | Building at 93% and 92% of the budget; the code cut § Risks once named for US3 | A cut by tests moves whole scenarios and no line of code, so the story that keeps the code is graded on fewer criteria and builds as it would have. The code cut was never built. A cut found mid-epic needs a relaunch. |

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
   `034-dark-and-flat-frames`, `035-live-stack` and `038-track-follows-a-push`, all in
   this spec's `depends_on_landed`. Not met: 015 and 038 have landed; the six others are
   drafts. 033 is named because it writes `imaging/mod.rs`, `imaging/routes.rs`,
   `lib.rs`, the README and the Imaging page before this spec does, and 035 depends on
   it; nothing of 033's is called here.
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
   § Refinement Record; and the validator is re-run on `main`. Probe round 1 found that
   038, on `main`, moved most of this trio's citations of
   `crates/finder/src/tracking.rs` from line 83 on (D7): the lines still hold content,
   so the validator passes every one but the citation of `impl Drop for Tracker`, and
   each is read again then. The same holds for the last route line of
   `crates/finder/src/lib.rs` and the last
   `/api/imaging` row of the README, after which US5 inserts its own and which § The
   route now also names by content (D36), and for the `dl` of `TrackControl.tsx`, at
   line 69 on round 1's base.
3. **035 is read as landed, against § What 035 must provide.** Not met. 035's trio was
   read whole when the review of this one was applied: its plan.md with the camera's
   lease, its tasks.md and its Refinement Record. It was then still a draft and not
   probe-built, and the lease it uses is 031's US9, which 031's draft carries and `main`
   does not yet (035 plan.md § Prerequisites 8). Since then 035's own probe round 1 has
   built it, and its trio has been re-anchored and flipped with its US5 split in two
   (its US5-S6 is now US11-S3); this trio's probe round 1 built US7 and US8 on 035's
   probe code and found the nine points provided, with the three words § The stack and
   § The page now give (§ Prerequisites 5). Before the flip each of the
   nine points of § What 035 must provide is read in the landed code, and these are
   settled:
   - the taker's loop, its returns, and the worker's `process` and `run` are where § The
     stack puts US7's calls, by name and line (read by name on `main` on 2026-11-19,
     after 035's US8 landed: § The stack says what was found);
   - how the worker counts and lists a dropped sub, and what `Dropped::count` does with
     an outcome it does not name: on `main` it is `unreachable!`, and US7 gives
     `not_steady` its arm;
   - the stopping mark is the `tokio::sync::watch` of `bool` that 035's plan.md names
     (point 2), and the taker holds a receiver of it;
   - the taker owns the run's `Lease` and it drops as the taker's task ends, and the
     holder's `set` behaves as point 3 says. This replaces the first draft's use of
     034's `Calibration::restore_exposure`, which called `Imaging::set`, would be refused
     while the stack holds the camera and has since been removed from 034 (034 A1), and
     the second draft's restore in 035's ending, after the taker had returned and the
     lease with it;
   - 035's landed `crates/finder/src/imaging/stack/service.rs` holds none of the
     thirteen names US2-S6 lists, so that US7-S6 rests on US7's own diff (read on `main`
     at `52785403` on 2026-11-19: none). If it holds one, the trio goes back to the
     owner before any story is dispatched;
   - no landed test compares an imaging view, or the session's `imaging` object, whole
     on a console that tracks with its camera ready: from US7 such a view carries
     `steering` (§ Shapes that existing tests pin);
   - 031's MCP test still asserts `get_imaging_camera`'s description word for word, so
     US11 leaves it, and the test that calls every listed tool with `{}` still does so
     on a console with no imaging;
   - no test of 035 counts `dropped`'s keys, compares the whole object with a literal, a
     fixed list of keys, a golden file or a schema that lists its keys (a comparison of
     two serializations of the same `Dropped`, as
     `stack_console.rs` makes between `stack.json` and the view, is not one), or builds
     `Dropped { .. }` by a struct literal without `..Default::default()`. If one does,
     the trio goes back to the owner for an approval that names that test, before any
     story is dispatched: no approval here covers an unnamed test;
   - 035's `StackBlock` builds its `Dropped` line as § The page assumes;
   - no landed test of 031 to 035 reads a region of the Imaging page by its place from
     the page's end (`getAllByRole("region").at(-1)` or `.at(-2)`): 036's block becomes
     the page's last region. 034's US9-S1 places its block after `Imaging frame` and
     `Focus`, and 035's US9-S6 its region after Calibration;
   - the worker still drops a sub whose exposure differs from the reference sub's. If
     035 comes to weight subs by exposure instead, spec owner decision 2's last
     alternative becomes possible and is put to the owner.
4. **The owner decisions are settled and the approvals given** (spec § Owner decisions,
   § Owner approvals). Not met: all seventeen decisions are open.
5. **A probe build.** Met on 2026-11-19 for the trio as round 1 found it, and not met
   for the trio as the reanalysis of the same day left it: nothing the two passes of
   that second fold wrote has been built (the end of this point says what a second round
   must show). Probe round 1 built all eight stories on 2026-11-18 and 19, tests first
   and with both gates, on probe builds of 030 to 035
   (spec.md § Clarifications, the session of round 1 probes). Before it, the bounds of
   US3-S1, US3-S2, US4-S5 and US7-S1 came from arithmetic on a model of the loop
   (§ Refinement Record), and US6-S5's rested on 015's own follow staying within 60″
   with the finder camera's corrections held for gaps 6 s apart before and after the
   measurement. US3 and US6 were probe-built together, since US6 changes the test
   `serve_gaps` step 1 makes and the held mode's test in `Shared::pointing`. What the
   round settled:
   - US3's test file passed before and after US6, unedited (§ Plan traps 22).
   - Both rigs of US7 ran. The stacking rig tracks and stacks on top of 035, 18 subs in
     about 97 s of virtual time, once its drift oracle reads each saved sub from its
     FITS file and it waits 1 s more before the stack starts (D37 and D38; spec.md
     § Clarifications, the session of round 1 probes); US7-S5's scripted tracker, given
     with `set_source`, saw its link dropped within 1 s, well inside its 10 s gap.
   - The nine points of § What 035 must provide hold in 035's probe code, with the three
     words § The stack and § The page now give: the taker's receiver of the stopping
     mark is made `mut` (point 2); the `not_steady` drop is counted inline, as the
     settings drop is, and `LastView` does not carry it (point 7: true of that probe
     code and not of the 035 that landed, where the drop is answered from `process` and
     `Dropped::count` gets an arm for it, § The stack); and the Stack block's
     filter reads an optional count with `?? 0` (point 8).
   - Every story's diff is under 64 KiB (§ Risks).
   - Every bound held on three runs each, with the same numbers each time. US3-S1: at
     most 0.0220″ from the 12th sub with each error handed over at once, and 0.0340″
     played late, against 0.15″ (4.4× at the late sub; the model's 0.030″ was 13% under
     it).
   - US3-S2: a median drift of 17.19″ without feedback, against "above 3.0″" (5.7×).
   - US4-S5: at most 0.01552″ from the 14th sub, against 0.15″ (9.7×).
   - US6-S5: a worst true error of 41.414″, against 015's 60″ (1.45×), with the play
     measured at 0.299879°. With the held mode's test left at `lapsed` alone, 015's
     bound broke at transit + 319.7 s, as § Plan traps 28 says it would.
   - US7-S1: at most 0.01558″ from the 14th sub, against 0.15″ (9.6×; 0.0317″ at the
     12th), and 0.746″ on every sub with the switch off.

   **What the reanalysis settled, and what a second probe round must show.** Six reviews
   of the folded trio found 78 more issues, and the owner took the recommended answer to
   each of the fifteen choices they came to (spec.md § Clarifications, the session of
   the reanalysis). The re-run among them built round 1's 60 folded cases on all eight
   stories: 59 passed on three runs each, with the five bounds unchanged to the digit,
   and the sixth lacked only D25's code fix, which that probe never had. It also ran two
   of the fixes now in the trio, a lapsed gap that always sends each axis its rate, with
   the whole workspace passing, and an error from before a follow dropped by its time.
   Everything else of the second fold rests on reading and on models of this plan's law:
   two of them reproduce round 1's two measured drifts and give, with the rate's limit a
   gap and the two-deadband rule added, the same 12th-sub figures to four places. Probe
   round 2 builds all eleven stories on `main`, with 035 landed, and must show:
   - every bound of round 1 again under the changed law: US3-S1 and US3-S2 with the
     rig's 3.5 s, US4-S5 and US7-S1 from the 14th sub, and US6-S5 with eight gaps and
     the offset left alone after a lapse;
   - a bound for US10-S8, which has none, written into the scenario with its margin;
   - the scenarios nobody has run, the least trusted first: the stopped-axis test
     raising no false alarm in any motion module (§ Plan traps 32) and US6-S7; US6-S6's
     push on the goto rig; US7-S7's far stack; US10-S9's timing, which is worked and not
     measured; US6-S17's wait for the limit and US5-S11's hour ahead; US10-S1, US6-S8
     and US6-S9; and the settle's times in US6-S1 and US10-S3;
   - that every test file passes unedited under every later story: US3's and US9's under
     US6, US4's and US5's under US6, and US9's and US10's on the code of the story each
     was cut from, with no test of theirs failing first;
   - each story's diff against its estimate in § Risks, US3, US5 and US6 above all;
   - the points of § Prerequisites 3 that 035's US9 and US10 settle, now that they have
     landed.
6. **Outside the factory, before this spec is used at the telescope:** GoTo and Track
   proven on the real mount (the plan document, § Before any flip, 4), and its backlash
   measured on both axes: altitude reverses at transit, and azimuth too for a target
   that culminates north of the zenith. Not met.
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
   route and MCP tool" (owner decision 11). That last answer changed on 2026-11-19: 036
   has one tool, `set_steering`, and the plan document's list gains it in the same
   change as the flip; it is not edited by this fold. Its § Simulator names what US1
   adds, a memory of where the telescope pointed and count steps, where its first draft
   had "rate error within an exposure (036)" (spec.md § Clarifications). If the owner
   settles any of those decisions otherwise, the plan document is corrected in the same
   change: the route's row, the part, the module, and the tool or the exemption. Rule 1
   is not edited: it gives its two times a tracker is not steady as today's examples,
   and spec owner decision 17 declares the three further times on its own, and since
   2026-11-19 three more. 035's Prerequisites 10 does the same for its own departures.
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
12. **038's followed push.** Added 2026-11-16; met on 2026-11-19.
    `038-track-follows-a-push` makes Track follow a push by hand of more than 5′ through
    `Tracker::follow`, which changes the place the tracker holds. 038 has landed
    (`ad09efeb`), so this trio is re-anchored to it: `follow` ends any claim the subs
    hold on the tracker and forgets any tracking error or held correction not yet
    applied, as `release` does (spec FR-011; § The tracker, **`follow`**), with one
    scenario in US3, US3-S12 [now US9-S12]; `038-track-follows-a-push` joins
    `depends_on_landed` and US3's shared write of the tracker; and the validator is
    re-run. Without that, a sub's error, measured from the old place, would steer the
    tracker back toward it at up to 30″ a gap. The reanalysis completed it on
    2026-11-19: `follow` also drops, by its time, an error from the old place that is
    handed over after the claim was taken
    again; it forgets what a lapsed claim held, with a scenario of its own (US9-S13);
    and the stretch before the follow, while 038 reads the push as moving, is a time the
    tracker is not steady (§ The tracker's two answers, **A push by hand**; US6-S6).

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
13. **The bundle must be rebuilt.** *Check:* US8 commits `crates/finder/web/dist` after
    `npm run build`.
14. **The lock across an await.** `Steering`'s and the tracker's locks are
    `std::sync::Mutex`. *Check:* neither is held across `gap().await`, a sleep or a
    send.
15. **A tracker kept alive.** Dropping a `Tracker` stops the mount
    (`crates/finder/src/tracking.rs:112`). *Check:* `Steering`'s state holds only `Weak`
    references; the one strong reference is the local of a `before_exposure` call, and
    it is not held across `gap().await`: `GapFuture` borrows nothing, and the call lets
    its `Arc` go before it awaits the answer; and US6-S4 drops the test's `Arc` and sees
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
    the rigs of US4-S5 and US7 start on the catalogue place so that no
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
    while `settling` is true; the tracker rig waits for the first rates, and 3.5 s more,
    in every module that writes it out, because from US6 a tracker also settles for 2 s
    after them and turns a gap away until then; US9-S7 asserts what a gap before the
    first rates does, and the assertion holds under US3's test and under US6's; US6's
    development checks run US3's and US9's modules.
23. **The stack's own lease refuses outsiders.** `Imaging::set` is refused while the
    stack holds the camera, as 034's `Calibration::restore_exposure`, which went through
    it, would have been (034 has since removed it, 034 A1). *Check:* both changes of the exposure are `Lease::set` on the run's lease;
    US7-S2 reads 30000 ms while the stack runs and 60000 ms after it, and fails for a
    set that was refused.
24. **A lapsed claim taken back too soon.** *Check:* `serve_gaps` step 1 claims a
    lapsed tracker only when an error is pending; `steer` keeps the newest error while
    lapsed; `release` and `end` clear `lapsed`, so a new stack's first gap always
    claims; US9-S4 asserts that a ninth idle gap leaves the finder camera steering.
25. **A wait left showing.** A stack stopped while `before_exposure` waits for a steady
    tracker drops that future with `waiting` true in the state, and 037 reads
    `Steering::waiting()`. *Check:* `Steering::release`, which the stack's `Hold` calls
    as it drops, sets `steady` true and `waiting` false; US6-S14 asserts it.
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
    is not looked at, `sent` is written wherever the task sends the azimuth axis a rate,
    and US3's, US9's and US4's motion modules, whose bounds a false alarm would break
    by dropping the claim, run in US6's development checks.
33. **A claim that outlives what should end it.** A claim lapses only at a gap. Where
    the steering stops asking for gaps, it must release the tracker itself, or nothing
    steers: the finder camera's pointings change no rate under a claim. *Check:* § The
    steering, **Letting the tracker go**, is called by `set_on(false)`, by a
    `set_set_point` that changes the set point and by the wait for the limit; US5-S9,
    US4-S12 and US6-S17 each see one `release` in the scripted tracker's log.
34. **A case of an early story that a later story's wait would hold.** From US6 a
    `before_exposure` waits while the limit is under half the exposure held, and a test
    of US5's that answered such an exposure would then never end, in a file US6 may not
    edit. *Check:* no case of US5's two modules has a limit under half the exposure it
    answers (spec US5-S1 says so of its own); US6's development checks run both.
35. **A first tracking error the steering stands back from.** A scenario that expects a
    `steer` must hand over a first error under 180″. *Check:* US4-S4 names the set point
    (668.0, 386.0), 110″ out, where it named one 230″ out; US4-S8 and US7-S7 are the
    only scenarios whose first error is over the limit.
36. **A story of tests only.** US9 and US10 write nothing under `crates/finder/src`, and
    their tests pass on the code of the story they were cut from as soon as they are
    written: there is no failing test to write first. *Check:* their tasks say so; a
    test of theirs that fails is a finding about US3's or US6's code and is escalated
    with its output, not answered by an edit of that code or of the test's bound.
37. **The view before the steering was used.** `view()` answers `None` until the
    steering is used, and US4's tests assert it; from US5 `view_for` with an ask answers
    a view while a tracker runs, used or not. *Check:* only `view_for` with `Some` makes
    that view, it marks nothing used, and US5-S12 asserts both.
38. **A guarded word in a comment.** US2-S6 and US7-S6 read the whole text of
    `steering.rs` and of 035's `stack/service.rs` for thirteen names, and a doc comment
    that explains a pause would use four of them. *Check:* the pause's words and their
    reasons are written in `tracking.rs` and in the page only; `steering.rs` passes
    `pause` through unread.

## Risks

| Risk | Mitigation |
|---|---|
| The bounds were worked from a model, and probe round 1 ran them on the simulator alone. | § Prerequisites 5: met. Every bound held on three runs each; the narrowest is US6-S5's, 41.414″ against 015's 60″ (1.45×), and the drift bound's is 4.4× at the 12th sub played late (0.0340″ against 0.15″, where the model gave 0.030″). The real mount is a field check. |
| 035 changes before it lands, or 031's lease lands in another shape or not at all. | § What 035 must provide lists the nine points this spec leans on, which probe round 1 found in 035's probe code with D39 to D41's words, and § Prerequisites 3 reads each as landed; only US7 and one line of US8 touch 035's files, and US1 to US6 are tested without a stack. |
| US7-S5's rig: a scripted tracker given to a console's steering, and the link dropped while its gap waits. | It ran in probe round 1: the link dropped 1 s into the 10 s gap was seen within 1 s, against 031's 3 s, and its words, its `taken` 0, its `gap`-then-`release` log and the restored setting held. The scenario's only timing is 031's 3 s for a lost link against a gap of 10 s. |
| The owner prefers arcseconds for the corner smear (spec owner decision 1). | The steering then needs a scale before a stack's first solve, 032's measured one or the one from the focal length setting; FR-018, US5 and the page's field are redrafted before the flip. |
| The stacking rig of US7: a tracked simulated mount, live finder solves, a pointing model fitted from syncs, the simulated solver and 035's stack together. | It ran in probe round 1, 18 subs in about 97 s of virtual time, once its drift oracle read each saved sub from its FITS file and it waited 1 s more before the stack started (D37, D38); US3-S1 and US4-S5 carry the claim of SC-003 on rigs of this spec's own, so a correction to US7-S1's bound does not touch it. |
| A sub dropped as soft carries a good tracking error that is not used, and a drift that makes subs soft is then taken out by the tracker's own corrections only. | Spec owner decision 15; a field check. |
| The limit falls under a long stack's exposure near the meridian. | The exposure is decided from the shortest limit of the hour ahead (34.3 s on the worked rig at 2 px, against 37.28 s at its start, measured in probe round 1), so a stack under an hour old does not meet it unless the floor of 1 s held its exposure up. The readout says `over_limit`, and under half the exposure held the stack waits. The hour, its sampling each minute and the figures for other targets are arithmetic on 015's formula, not a build: probe round 2 runs US5-S11. |
| The default smear of 6 px lets a corner star trail three times as far as the drafted 2 px. | It is the owner's choice of 2026-11-19, on the research note's figures for this telescope (28 px of coma in that corner uncorrected; 20 s to 30 s subs recalled). It stays a setting, and the block's `Longest exposure` names the smear in force. A field check on a bright field. |
| The wait for the limit lets the tracker go, and the finder camera then steers for as long as the wait lasts. | That is the tracker's state with the switch off, and no exposure is taken in it. When the limit has risen the next gap claims afresh, and where the finder camera's aim point and the set point differ by more than 3′ the steering then stands back (spec FR-014). Not run: US6-S17 is a scripted tracker. |
| A half-second pulse of one count does not move the real tube (stiction, backlash). | The rate then takes it up: an undelivered step of under two deadbands reads as a drift, and 0.3 of it goes into the rate, 0.1″/s a gap at most. A field check (spec § Assumptions). |
| The real mount makes more or less than the step asked for. | A pair of errors with more than two deadbands of steps between them changes no rate, and a step is booked for the time its pulse really lasted (§ The tracker, `serve_gaps` steps 2 and 4). Models of the law, not builds, say what that buys. A field check: the tube's motion per commanded pulse of 1″, 10″ and 30″. |
| The real mount's rate commands are coarser than the model. | Field check; the constants are in one place. |
| An error that needs a slow axis to back up leaves slowly. | 015's rule that no correction reverses an axis stands; near a reversal the take-up crosses it. A field check through the meridian. |
| The real solver is slower than a sub, so errors arrive several gaps late. | The tracker's memory of 16 gaps covers 035's queue of four, and the claim lapses only after eight gaps with no error, so a worker a whole queue behind still measures a drift rate. `took_ms` in 035's view, against the sub cadence, is the figure to read: a field check. |
| Accepted subs come more than eight gaps apart (cloud, a gate dropping most subs, a console that needs more than eight sub lengths for one sub). | Each tracking error is then the first after a claim and sets no rate, and since the reanalysis the finder camera's corrections change no rate after a lapse either: a rate error stays until subs come closer together, and only steps correct the place. The readout shows `source` changing. A field check with subs dropped; keeping the last applied error through a lapse is the remedy if the field shows it, and wants a probe of its own. |
| An imaging solve is wrong and 035 accepts it, or the tube is knocked between two subs. | One gap moves an axis 30″ on the sky at most; a jump changes no rate, and anything under the jump test changes it by 0.1″/s at most. |
| The finder camera's aim point and the sensor's centre disagree by more than 038's 5′. | The steering stands back from a stack whose first tracking error is over 180″, and the page says so. How large the aim point's error is on the real finder is not measured; under 5′ and over a few arcseconds, each lapse hands the telescope to the finder camera's reference and each return takes it back. A field check. |
| A hand on the tube during a stack. | While 038 reads the push as moving the tracker is not steady. Read from 038's code on `main`, not run: probe round 2 runs US6-S6. One finder solve more than 5′ wrong costs one sub. |
| The scope node stops both axes on a failed rate command and acknowledges it. | The tracker notices a turn later and drops the sub. How often a rate command fails on the real board is not known; the lake night logged a bad mount reply every 1 to 8 minutes of polling. A field check: 30 minutes of Track with the node's journal read afterwards. The node's own fix is another spec. |
| The simulated mount reads `moving` false where the tracker last sent a rate that was not zero, with nothing stopped. | Then the stopped-axis test drops claims in scenarios that do not expect it. No build has run it: probe round 2 shows it in US3's and US4's motion modules (§ Plan traps 32). |
| Rate updates once a second during an exposure. | They are 015's feed-forward and change by thousandths of an arcsecond a second; unchanged here. The first sub after a time not steady would have begun on 015's proportional rate, +0.632″/s for 0.15 s measured in round 1 and +25.3″/s in its recheck, had every gap served under a lapsed claim not sent each axis its rate (§ The tracker, `serve_gaps` step 4; US10-S1, US6-S8, US10-S9). A field check. |
| The first reversal on a mount pauses the stack for minutes. | Once per mount and axis: the backlash is kept with the fixed terms. The page says exposures are paused. Probe round 1 measured about 312 s of the simulator's time with 0.3° of play, and 0.85 s for a take-up, to which the 2 s of settling are now added. With a real finder camera's scatter the measurement closes later than the simulator's 312 s: a field check. |
| A real tube rings after a take-up, a slew or a start from rest for longer than 2 s. | `SETTLE` is one constant. A field check: the first sub after a take-up. |
| After a lapse the finder camera corrects once a gap, not once a solve, and corrects the place only. | Each gap takes 0.6 of the newest pointing's error out as a step, and the tracking offset stays as the subs left it. A lapse means subs are being dropped (cloud, a lost field), which the readout shows. The step is from one finder pointing, so while lapsed the frame is moved by the finder camera's scatter each gap, about 13″ to 21″ in a model, between exposures. A field check with subs dropped. |
| US6-S5 has not been run since the lapse moved to eight gaps and the offset stopped changing after one. | Probe round 1 ran it with three gaps and the integral held (41.414″ against 015's 60″). The 41.4″ comes while the measurement is open, when the tracker is not steady and 015's corrections run whole, with a stack or without; what changes is the stretch after the close, 10.9″ in round 1, where the offset 015 built during the measurement is now left as it is. It must be probed again before the flip. |
| US10-S8's bound is not written (it was US6-S11). | No build has steered through a reversal with a rate error. Probe round 2 measures it and the number is written into the scenario, with its margin, before the flip. It is in a story of tests only, so no story's code waits on it. If it shows the offset winding where `commanded` sends zero, `serve_gaps` step 2 gains "an axis whose `base` is zero where its feed-forward is not changes no offset at this gap". |
| A story's diff nears 64 KiB. | Round 1 measured every story: US1 19,045 B, US2 14,094 B, US3 50,893 B (24,984 B of tracker and 25,583 B of test), US4 31,946 B, US5 38,804 B, US6 32,913 B, US7 35,434 B and US8 34,698 B. With its cases folded in and the reanalysis's first pass, US3 stood at about 61,300 B (93%) and US6 at about 55,200 B (84%); the second pass took US6 to about 60,300 B (92%). Both were cut by tests on 2026-11-19 (spec.md § Clarifications, the session of the reanalysis). Estimated from those measures and the length of what the plan and the scenarios now ask for, nothing of it built: US1 about 24,400 B (37%), US2 15,600 B (24%), US3 49,100 B (75%), US4 38,700 B (59%), US5 53,100 B (81%), US6 48,400 B (74%), US7 43,500 B (66%), US8 43,700 B (67%), US9 23,900 B (36%), US10 15,300 B (23%) and US11 8,700 B (13%). The factory's builder has landed large stories 2% to 4% over such estimates. US5 is now the nearest to the budget: the hour ahead, the view's ask, the two shares and the switch's release are about 10 KB of it. If a build measures US5 over, it is cut as the others were, by tests: its four motion scenarios (US5-S6, US5-S8, US5-S11 and US5-S14) and their module go to a story of their own after US5, about 9 KB. No story drops or shortens a case to get under (§ Plan traps 10). |
| The Refinement Record nears its 65,536 B. | It is the fence's text the validator measures. Each scenario costs an entry of about 300 B, and this fold's 59 new ones took it from 43,878 B to the figure the fold's report gives. The one lever left is formatting, two spaces off the record's list items; no assertion is cut to make room. |
| `view_for` makes the imaging view carry `steering` on any console that tracks with a ready camera, from US7. | 035's tests read views key by key, and 032's tracking test reads its solve. § Prerequisites 3 checks the landed tests for a whole-view comparison on such a console before the flip. |
| The two shares can be set to values that make the loop ring or stall. | They are bounded to 0 to 1, run-only and back at 0.5 and 0.3 at every start; the rate's limit a gap and the two-deadband rule still hold whatever they are. Models of the law, not builds, say 0.5 and 0.3 are sound: a field check, and the reason they are settings. |
| US10-S9's timing is worked, not measured. | It rests on the take-up starting 1.99 s before the rig's transit, as the recheck of round 1 saw it, and on the settling ending 3.0 s after that. Its first assertion fails loudly if the case was not met, and probe round 2 confirms the 5.4 s or moves it. The story is tests only. |
| The caps of 16 remembered gaps, 32 remembered exposures and 64 times not steady are asserted by no scenario. | Two of each pass every test (reanalysis CC15). A case for each was weighed against the size of US3 and US6 and left out; each is one named constant. |
| 037's trio says the stack's steering brings in a target stacked up to a quarter of the frame from the centre. | It does not beyond 180″, and inside it is slow against an axis's motion. 037's sentences and its two warnings are corrected in 037's trio; the fold's report lists them. |
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
steady they run as 015 has them (US3-S6 [its lapse now US9-S4], US3-S11, US6-S5, § The
tracker, § Plan traps 27 and 28); the stopping mark is named, and the observer's
exposure is put back where the taker returns (§ The stack, § What 035 must provide);
rule 1 is no longer to be edited (§ Prerequisites 9); US7-S6 guards 035's
`stack/service.rs`, and owner decision 1's widening is worked; and a bound on the rate
one tracking error may ask for waited on 032's owner decision, which the owner settled
on 2026-11-17 with no bound (§ Prerequisites 11). Every `path:line` in spec.md
and plan.md was then read again at 49bb210, the worktree's HEAD; none had moved. **Until
probe round 1 no probe build had been run:** no story had been compiled, and no scenario
had been seen to pass or fail, before the review, after it, or after the audit.

What stood in for a probe until round 1, and what it did not show:
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
  (`crates/finder/tests/support/tracker.rs:323`) and had not been run.
- **The worked numbers** of US2, US4-S4, US5 and US8 were recomputed with a short
  script; the limit of 37.15 s and the 65″ of refraction at the rig's start likewise
  (probe round 1 measured the limit on the rig as 37.28 s, at 14.91°/h, 0.36% off: D32).
  The review recomputed them independently, and its corrections are in: the turns of
  US5 are now the test's own `hypot` expressions, US3-S10's signs are right, US5-S6's
  due-east instant is worked out (9.7″ of azimuth a second against a 3.2″ window), and
  US8-S4 no longer rests on how a tie rounds.
- **Anchors** were read at 49bb210, read again there after the audit, and checked by
  the validator.

Before round 1, for the probe build and the re-anchoring: every bound named in
§ Prerequisites 5; whether `serve_gaps` can be woven into `run` as described without
disturbing the keyhole's `second.reset()`; whether a gap that takes 0.5 s lets the next
once-a-second turn slip past 015 US3-S1's "at least once a second" in any existing test
(no existing test calls `gap`); US3's test file under US6's test in `serve_gaps` step 1
(the tracker rig waits for the first rates, and US3-S7 [now US9-S7] pins what a gap
before them does, so the two stories should agree, but only a build of both shows it);
whether the wait for the first rates ever takes more than a few milliseconds of virtual
time; US3's test file under US6's held-mode test, `lapsed && control.steady`; whether
US6-S5's follow keeps 015's 60″ bound with the finder camera's corrections held for gaps
6 s apart; US7-S5's rig; and all of § Prerequisites 3.

**What probe round 1 changed** (2026-11-18 and 19; spec.md § Clarifications, the session
of round 1 probes). Four agents built the eight stories from this trio, in order, on
probe builds of 030 to 035, tests first and with both gates, and found 3 blocking and 44
clarity defects, D1 to D47, all folded on 2026-11-19 with the owner's answers of that
day. Of the questions above, all but § Prerequisites 3, which waits for 035 to land,
were answered: every bound held (§ Prerequisites 5); `serve_gaps` was woven into `run`
as described, with the target and counts read in its step 1 (D9); 015's tests, and every
other existing test but the one 034 test of D43, passed under both gates on every story;
US3's test file passed unedited before and after US6; US6-S5's follow kept 015's 60″
bound; and US7-S5's rig ran. The changes that reach across stories: US6-S1's gap answers
at L or after it (D24), US7's drift oracle reads each saved sub from its FITS file
(D37), and the stacking rig waits 1 s more before a stack starts (D38), the three
blocking defects; a lapsed gap that zeroes a proportional rate still in force sends the
zeroed rate (D25: `serve_gaps` steps 2 and 4, FR-010, US6-S1 [now US10-S1]); 038's
`follow` ends the claim as `release` does (D6: FR-011, US3-S12 [now US9-S12],
§ Prerequisites 12), and 038 joins `depends_on_landed`; the owner trigger of
§ Prerequisites 3 is narrowed to a comparison
with a literal or a fixed list of keys (D8), and that prerequisite gains a check of the
regions read from the page's end (D43); three scenarios are new, US1-S6, US3-S12 [now
US9-S12] and US4-S7; every case the round proposed where a mutation survived is in its
scenario, in the round's words; and § What 035 must provide and § The stack gain three
words (D39 to D41). The records below change with them: new tests for US1-S6, US3-S12
[now US9-S12] and US4-S7 and widened assertions where cases were added; US3's shared
write of the tracker names 038's US1, and its preserved behaviour 038's tests; US7's
preserved tests name 035's `stack_camera.rs`; and US8 writes the README, after US7's
section. No story was over 64 KiB, and none is on the folded figures (§ Risks).

**What the reanalysis changed** (2026-11-19; spec.md § Clarifications, the session of
the reanalysis). Six reviews of the folded trio found 78 more issues, AA1 to FF20, and
the owner took the recommended answer to all fifteen choices they came to. They were
folded the same day in two passes; § Prerequisites 5 says what was run and what was not.
The first pass changed the tracker and the stack's wiring: a push the session reads as
moving, 2 s of settling and a turn that finds an axis stopped are times the tracker is
not steady (FR-021, US6-S6, US6-S7); the steering stands back from a stack whose first
tracking error is over 180″ (FR-014, US4-S8, US7-S7); the claim lapses after eight gaps,
and a lapse holds the finder camera's step and not its integral (FR-011); no gap changes
a rate by more than 0.1″/s, a pair of errors with more than two deadbands of steps
between them changes none, and a step is booked for the time its pulse lasted (FR-008,
FR-009); an error from before a follow or a time not steady is dropped by its time, and
a lapsed gap always sends each axis its rate (FR-010, FR-011); and US7's calls are
written for the 035 that landed (§ The stack). It added the edge from US6 to US4, for
`session.rs`. The second pass changed the cap, the view, the page and MCP: the exposure
is decided from the hour ahead, not halved to under 1 s, and waited on while the limit
is under half of it (FR-017, FR-018, FR-022); the default smear is 6 px; the view
carries the limit and what a stack started now would take whenever a tracker runs
(`view_for`); a pause says why, in the tracker's word (FR-021, FR-022); the page's
sentences are reworded (§ The page); the two shares are settings (FR-030);
`set_steering` is an MCP tool (FR-031); and the switch turned off, or a set point
changed, during a stack lets the tracker go (FR-014, FR-019). It split every scenario
that held a second Given, or a When on another rig, and it cut US3 and US6 by tests: US9
and US10 hold the scenarios that left them, write no code and implement no FR, and US11
is the tool. The records below change with all of it: three new blocks; 59 new entries,
each a scenario that is new or was the later half of another, whose own line is trimmed
to what it still asserts; US4's and US7's edges to US9 and US10, for
`crates/finder/tests/motion/main.rs`; US5's write of `imaging/mod.rs`; and the
development checks of US6, which run US9's module beside US3's. Two scenarios were found
broken by the fold's own rules and mended: US4-S4's set point and a case of US5-S1
(spec.md § Clarifications).

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
record's `open_questions` are empty because the validator refuses a record that holds any; what
is not established is in § Risks and § Prerequisites 2 and 3, and in spec.md's
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
    - path: crates/finder/src/session.rs
      other_spec: 036-tracking-from-subs
      other_story: US6
      disposition: merge_order
      reason: "US6 depends_on_merged US4, and adds the calls of Tracker::pushed to 038's track_live_pointing"
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
      assertion: "both axes stopped behind the tracker's back make it not steady from its last azimuth rate to its next rates, which start the axes again, and drop the claim"
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
      assertion: "a tracker that stops being steady while its gap is awaited shows steady false and waiting true"
      decision: "spec.md:US6-S11"
    - path: crates/finder/tests/console/steering_steady.rs
      status: new
      disposition: add
      assertion: "a gap that ends on a tracker no longer steady does not count: before_exposure answers after a second gap"
      decision: "spec.md:US6-S12"
    - path: crates/finder/tests/console/steering_steady.rs
      status: new
      disposition: add
      assertion: "before any wait the view has steady true, waiting false, no pause word and no drop, with a tracker and with none"
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
      assertion: "while the limit is under half the exposure held before_exposure waits with the word limit, the tracker released once and no gap asked, and answers once the limit is half of it again"
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
    - "a scripted tracker that is always steady is never waited for (US4's and US5's scenarios)"
    - "crates/finder/tests/motion/steering.rs, US3's test file, passes unchanged: its tracker rig waits 3.5 s after the first rates before a scenario asks for a gap, by when the tracker has settled, and no scenario of it, US3-S11 with its held step among them, has a time the tracker is not steady after that wait, so the held mode's new test and the errors dropped by their time change nothing there"
    - "crates/finder/tests/motion/steering_gap.rs, US4's test file, passes unchanged: its rig waits 3.5 s after the first rates too, and it meets no reversal and no push"
    - "motion/steering_claim.rs, US9's, passes unchanged as US3's does: its rig waits 3.5 s, US9-S7 expects what a tracker that is not steady answers, US9-S9 waits 4.5 s, and its lapsed scenarios meet no time not steady"
    - "US5's console and motion steering_cap.rs pass unchanged: no case has a limit under half the exposure it answers, so none waits"
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
      other_story: US8
      disposition: merge_order
      reason: "US8 depends_on_merged US6, and adds the readout's figures"
    - path: crates/finder/tests/motion/main.rs
      other_spec: 036-tracking-from-subs
      other_story: US10
      disposition: merge_order
      reason: "US10 depends_on_merged US6, and adds mod steering_unsteady after mod steering_steady"
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
      assertion: "a sub during which Track was turned off and on is dropped as not_steady, counted in the stack's twelfth count, in the steering's and in stack.json's subs, the stack's counts still add up, and total() adds the twelfth"
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
    - "the camera's lease and its refusals are 031's as 035 assumes them: the stack changes the exposure through the lease it already holds and takes no other (plan.md What 035 must provide)"
    - "crates/finder/tests/console/steering_gap.rs, steering_cap.rs and steering_steady.rs, US4's, US5's and US6's test files, pass unchanged: Steering's calls behave as before the stack used them"
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
    - path: crates/finder/README.md
      other_spec: 036-tracking-from-subs
      other_story: US8
      disposition: merge_order
      reason: "US8 depends_on_merged US7, and adds the readout's four keys to US7's section"
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
      assertion: "three worked tracking errors with one step between give a drift of 0.58583 arcsec and an error of 0.90370 RMS, the second's 1/sqrt(2) written as FRAC_1_SQRT_2, and their pixels at the newest error's scale, 2.16 arcsec a pixel or half of it; a step before the first error is not taken out; and one error gives no figure"
      decision: "spec.md:US8-S1"
    - path: crates/finder/tests/console/steering_readout.rs
      status: new
      disposition: add
      assertion: "the window is the last ten errors, neither nine nor eleven, and a release empties it"
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
      assertion: "Save tracking settings PUTs on, smear_px, step_share and rate_share to /api/imaging/steering, and a refusal is an alert with the answer's words"
      decision: "spec.md:US8-S5"
    - path: crates/finder/web/src/components/SteeringBlock.test.tsx
      status: new
      disposition: add
      assertion: "the Imaging page shows the block after the stack block"
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
      assertion: "the form's fields are drafts: a render with the same settings leaves a typed value, and changed settings replace it"
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
      other_spec: 036-tracking-from-subs
      other_story: US11
      disposition: merge_order
      reason: "US11 depends_on_merged US8, and adds the tool's lines to the README's list of tools"
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
      assertion: "the tracker is not steady before its first rates nor for 2 s after them, saying starting, nor in any window before its start; a 10 s window ending now is false 6 s after the start and true 14 s after it; and no window past its stop is steady"
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
      other_spec: 036-tracking-from-subs
      other_story: US7
      disposition: merge_order
      reason: "US7 depends_on_merged US10, and adds mod steering_stack before mod steering_steady"
    open_questions: []
  US11:
    tests:
    - path: crates/finder/tests/mcp/steering.rs
      status: new
      disposition: add
      assertion: "set_steering is listed with its exact description, an object output schema, no read-only hint and the four optional arguments"
      decision: "spec.md:US11-S1"
    - path: crates/finder/tests/mcp/steering.rs
      status: new
      disposition: add
      assertion: "set_steering sets any of the four settings and answers the imaging view, get_imaging_camera and get_session carry the same steering, and empty arguments change nothing"
      decision: "spec.md:US11-S2"
    - path: crates/finder/tests/mcp/steering.rs
      status: new
      disposition: add
      assertion: "bad arguments answer a tool error with the route's words and change nothing, and a console with no imaging answers at once"
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
      reason: "036 depends_on_landed 035, whose US10 last writes the MCP tools"
    - path: crates/finder/tests/mcp/main.rs
      other_spec: 035-live-stack
      other_story: US10
      disposition: merge_order
      reason: "036 depends_on_landed 035, whose US10 last writes mcp/main.rs, with mod stack"
    open_questions: []
```
