---
state: ready
depends_on_landed: ["015-tracking", "030-imaging-camera-link", "031-imaging-frames-on-console", "032-imaging-solve-and-aim-point", "033-focus-reading-and-aid", "034-dark-and-flat-frames", "035-live-stack", "038-track-follows-a-push"]
# Tracking steered by the subs, the first of two pieces: the simulator's memory, the
# tracking error, the tracker steered by it, corrections only in the gap and the exposure
# cap (US1 to US5; US9 tests US3's tracker, and US12 US5's). 040-stack-waits-for-steady
# holds the rest (US6, US7, US8, US10, US11) and lists this spec in its depends_on_landed.
# Depends on 015, 030 to 035 and 038 (depends_on_landed), all landed. State: ready,
# flipped on 2026-11-20 under the owner's standing permission for the imaging specs (flip.md).
# Re-anchored on 2026-11-20 to `main` at 52785403, after 035 landed, and probe-built there
# again (round 2): its 21 clarity defects are folded, and US5 is cut by tests into US12.
# history.md has how it got here.
---

# Feature Specification: Tracking Steered by the Subs

**Feature Branch**: `036-tracking-from-subs`

**Created**: 2026-11-15

**Status**: Ready

**Input**: Spec 015 keeps a target at the aim point with the finder camera's solves. That
is enough for an eyepiece and not for an imaging camera: a finder solve wanders by tens of
arcseconds, more than 015's 10″ deadband, and each wander becomes a rate change that
smears a sub. Spec 035 solves every sub, and an imaging solve places the target on the
sensor to a fraction of an arcsecond. While a stack runs, take the tracking error from
each sub's imaging solve in place of the finder camera's live pointing, change the
mount's rates only between exposures for as long as a stack holds the tracker,
whichever of the two steers it, cap the exposure at the tracker's limit for a
chosen smear, pause exposures while the tracker is not steady, and show the drift and the
error. 015 lists "guiding from the imaging camera" as out of scope; this spec is that
work.

**This piece.** This spec is the first of two. It holds the simulator's memory (US1),
the tracking error and the one interface between the stack and a tracker (US2), the
tracker steered by tracking errors (US3, whose claim's edges US9 tests), the steering
that corrects only in the gap (US4), and the exposure cap with the steering's settings
(US5, whose limits and tuning US12 tests on the tracker). `040-stack-waits-for-steady`
builds on it as landed: the pause while the tracker
is not steady, the stack that asks the steering before every exposure, the readout on
the Imaging page and the MCP tool `set_steering`. A cite such as "040's US6-S5" is to
that spec.

**Owner decisions and owner approvals** are cited here by their numbers in flip.md, which
lists them for the owner; the rule each one settles is stated in the requirement,
scenario or interface that cites it.

## User Scenarios & Testing *(mandatory)*

Every scenario shares these definitions unless it says otherwise.

- **The gate command.** Every Rust scenario runs in
  `cargo nextest run --workspace --all-targets --all-features --locked`, the `test`
  gate's first command, each test in its own process. The page's scenarios run in
  `npm test`, a later command of the same gate, in `crates/finder/web`.
- **Paused time.** Every Rust test here runs under
  `#[tokio::test(start_paused = true)]`, so every wait is in virtual time.
- **A position in an imaging frame**, angles "east of north" and **a separation** are as
  032 spec.md § User Scenarios defines them: (x, y) in pixels from the frame's top-left
  corner, the centre of a frame W wide and H high at (W/2, H/2), up toward smaller y.
- **A sky offset** of a place P from a place Q is east = (RA of P less RA of Q, reduced
  into −180° to 180°) × cos(Dec of Q) × 3600 and north = (Dec of P less Dec of Q) ×
  3600, in arcseconds, as `crates/finder/tests/support/tracker.rs:89` computes it.
- **The worked solve** is `ImagingSolve::from_cd(83.8221, -5.3911,
  [[-5.196152422706632e-4, -3.0e-4], [3.0e-4, -5.196152422706632e-4]], 1236, 822)`: a
  frame 1236 × 822 centred on RA 83.8221°, Dec −5.3911°, up 30° east of north, 2.16″ a
  pixel, not mirrored (032's worked answer B). **The worked target** is M42's catalogue
  place, RA 83.81867°, Dec −5.38967° (`crates/finder/data/openngc.csv:7666`). **The
  worked frame** is a `SolveFrame` 1236 × 822, 16 bits, `RGGB`, binning 1, whose `roi`
  is the whole sensor (0, 0, 1236, 822), with the `start` its scenario gives, an
  `exposure_s` of 5.0 unless it gives another, and any path.
- **The tracker rig** is the rig `crates/finder/tests/motion/tracking/pi_loop.rs:104`
  builds, written out again in the module that uses it, with two changes: the base tilt
  is 0° in the mount and in the model, and it keeps its `Tracker` in an `Arc`, so that a
  second task or a `Steering`'s source can share it. A module keeps the rig's
  `Simulator` as a field only where it reads it (US4-S5's does; US3's does not, and a
  field nothing reads fails the `lint` gate). So: a
  `VirtualClock` from Unix 1768528800 (`2026-01-16T02:00:00Z`); the target M42; a
  `SimMount` with no backlash whose zero points put its axes on M42's apparent place at
  the start, and whose truth is loaded into the model with `load_known_terms`
  (`crates/finder/src/model.rs:351`); a wrapper round the mount that logs every rate
  command with the console's time and its axis, as
  `crates/finder/tests/motion/tracking/pi_loop.rs:41` does: **the rate log**; frames
  stopped; and a `Tracker` on the console's clock at the site 40° N 75° W with a
  minimum altitude of 15°, started on M42. A scenario gives the mount's rate error.
  **The rig waits for the first rates:** after `start` answers, it looks at the rate log
  every 10 ms of tokio time until the log holds at least one rate for each axis, and
  fails if that takes more than 5 s; then it waits 3.5 s more. A tracker's first rates
  are sent by its task's first turn, not by `start` (plan.md § The tracker, **The rate task**), and a gap asked
  for before them moves and claims nothing. The 3.5 s are for 040's US6, under which a tracker
  settles for 2 s after its first rates and turns a gap away until then (040's FR-021): US3's
  scenarios must pass before 040's US6 and after it. Every scenario on the tracker rig begins
  after that wait, unless it says the rig is built without it. Each module that writes
  the rig out writes the wait too.
- **Windows on the rate log.** Its times are console Unix seconds near 1.77 × 10⁹, where
  an `f64` resolves about 2.4 × 10⁻⁷ s: a test that tells two instants apart, or opens a
  window just after one, adds at least 1 ms, never 10⁻⁹ s, which is lost in the sum.
  Where a scenario says "every rate" in a window, its test also asserts that the window
  holds at least one rate for each axis it speaks of.
- **The truth error** on the tracker rig is the sky offset of the mount's true
  direction, as a J2000 place, from M42's apparent place, as
  `crates/finder/tests/motion/tracking/pi_loop.rs:217` computes it. **The feed-forward
  rate** of an axis at a time is the test's own, as
  `crates/finder/tests/motion/tracking/pi_loop.rs:252` computes it.
- **Playing a sub** of exposure T on the tracker rig is: (1) await `tracker.gap()` and
  keep what it answers; (2) read the truth error e0; (3) wait T/2 in 50 ms steps and
  read the truth error em and the console time tm; (4) wait T/2 and read e1; (5) wait
  1 s; (6) call `tracker.steer` with the time tm and em's east and north. The sub's
  **true drift** is the length of e1 − e0. **Playing a sub without feedback** is steps 2
  to 5 alone.
- **An axis displacement** of (a, b) arcseconds at time t is the sky offset, computed by
  the test with the simulator's transforms (`crates/finder/tests/support/tracker.rs:76`,
  `crates/finder/tests/support/tracker.rs:83`), of the direction at altitude
  `alt + b/3600` and azimuth `az + a/3600` from the direction at (alt, az), M42's
  apparent place at t. **One count** of an axis is 1,296,000 divided by the counts a
  turn the mount reports for it: 0.8″ on the simulated mount
  (`crates/simulator/src/mount.rs:12`).
- **Playing a sub late** is playing a sub with step 6 changed: for the first sub the
  test hands over nothing; for every later sub it hands over the em and tm it kept
  from the sub before it. So each tracking error reaches the tracker one sub after its
  own, as 035's worker delivers them (plan.md § Summary).
- **The backlash rig** is `crate::support::tracker::Rig`
  (`crates/finder/tests/support/tracker.rs:98`) for M42 from 120 s before its transit
  (`crates/finder/tests/support/tracker.rs:237`), on a session file that already holds
  0.3° of backlash for both axes, as
  `crates/finder/tests/motion/tracking/backlash.rs:71` writes one. `Rig::follow`
  (`crates/finder/tests/support/tracker.rs:269`) feeds it the truth as finder pointings.
  **The lapsed backlash rig** is the backlash rig followed to 100 s before the transit
  with `rig.follow(transit - 100.0, 60.0)`, with one `gap` awaited then and eight more
  6 s apart with no `steer`: its claim lapsed at the ninth gap, 52 s before the transit,
  and `steered_by_subs()` is false.
- **The scripted tracker** is a type of the test module's own that implements `Steered`:
  it answers a target the test sets; logs every `steer`, `gap`, `release` and, where its
  module's tests read them, `tune`, with the console's time; makes `gap` wait a time the
  test sets, on tokio time, and answer a `Stepped` the test sets; says it is steered by
  the subs from its first `gap` to its next `release`; answers the longest exposure,
  `steady`, `steady_between` and the `pause` word the test sets; and keeps the last
  window `steady_between` was asked about. Unless a scenario says otherwise its target
  is the worked target, its `gap` waits 0 s and answers (0.0, 0.0), it names no longest
  exposure, `steady` and `steady_between` are true and `pause` is none. It answers
  `longest_exposure_ahead_s` what it answers `longest_exposure_s`, unless a scenario
  sets a longest exposure for the hour ahead apart. **A
  scripted steering** is a `Steering` whose source answers that tracker from a cell the
  test can empty, so that the test's own `Arc` is the last one; it runs on the console's
  `FixedClock` (`crates/finder/src/session.rs:41`) at 1768528800.0, or on the rig's
  clock where a scenario moves a mount, or on a clock that follows tokio's where a
  scenario says so. **The worked ask** is `ExposureAsk { wanted_s: 5.0,
  shortest_s: 0.001, sensor: (1236, 822), roi: the whole sensor (0, 0, 1236, 822),
  binning: 1 }`, with the `wanted_s` its scenario gives where that is not 5 s.
- **A star's centroid** in a simulated imaging frame taken with noise off is the mean
  position of the samples within 10 px of the brightest sample, in x and in y, each
  weighted by its value less the background the scenario states, a sample's position
  being its centre, (column + 0.5, row + 0.5). 030 US2-S3 holds the same quantity
  within 0.3 px of the truth.
- **The tracking console** is the console `crate::support::goto::Rig` builds
  (`crates/finder/tests/support/goto.rs:113`) at altitude 50°, azimuth 120°, pointed at
  M42 with `point_eyepiece_at`, synced, and tracking M42 through `POST /api/track` with
  no GoTo, as `crates/finder/tests/motion/tracking/routes.rs:96` does. A scenario that
  says "before Track is turned on" takes it before that last step.
- **Scratch folders** come from `crate::support::Scratch::new(name)`
  (`crates/finder/tests/support/mod.rs:14`).
- **Requests** go through the router in process, as `request` in
  `crates/finder/tests/support/live.rs:90` sends them, with
  `Content-Type: application/json` and no `Origin`. A foreign `Origin` is
  `http://elsewhere.example`, sent with a `Host` of localhost on port 8080.
- **Comparing a view.** A test reads `steering` key by key, never whole: later stories
  add keys.
- **Oracles.** Each scenario names its oracle: the simulator's truth, a worked example
  whose arithmetic is given, 015's own functions where a scenario says this spec must
  agree with them, or the words, statuses and keys the scenario itself states.

### User Story 1 - The simulator remembers where the telescope pointed (Priority: P1)

As a test of tracking from subs, I ask the simulator where the imaging camera pointed
at the middle of an exposure that ended seconds ago, while the mount has gone on moving,
and I get the truth of that moment. I can also make the simulated mount move in whole
counts.

**Why this priority**: Without it every simulated imaging solve on a tracking mount is
wrong by more than the error this spec corrects, and no later scenario means anything.

**Independent Test**: `crates/simulator/tests/imaging_history.rs`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p simulator --locked --test imaging_history`.

**Acceptance Scenarios**:

1. **Given** a `Simulator` over the test database at 40° N 75° W on a `VirtualClock`
   from 1768528800, a `SimMount::with_config` with no backlash and no base tilt
   connected, `push_to(50.0, 120.0)`, `set_rate_error(0.25)` called on the mount and
   only then its azimuth axis commanded at 0.02°/s (a rate error applies to the rate
   commands that follow it, `crates/simulator/src/mount.rs:302`), 5 s waited, and a
   `SimImagingCamera` with noise off, gain 0 and one white star of magnitude 8 at the
   centre of `sim.imaging_truth()` as read then, **When** a test starts a 2 s exposure
   in a task with no wait in between, reads `sim.imaging_truth()` and
   `sim.telescope().eyepiece` (its RA, Dec and azimuth) at its start, 1 s later and 2 s
   later, waits 30 s more with the axis still turning, and asks `sim.imaging_truth_at`
   for those three instants, **Then** a committed test asserts that:
   - each answer's centre is within 0.02″ of `sim.telescope().eyepiece`'s RA and Dec
     read at that instant, and its angle within 0.001° of the `imaging_truth()` reading
     taken then (oracle: the simulator's own truth, read live);
   - the eyepiece's azimuth rose by 0.0500° ± 0.0005° over the 2 s (worked example:
     0.02°/s × 1.25 × 2 s);
   - the first and third answers' centres are more than 60″ apart, so the solve's centre
     moves with the drift;
   - the star's trail in the frame is as long as the truth says. The star sits on the
     first truth's centre, so it starts at the frame's centre, (618.0, 411.0). With d
     the separation of the first and third answers' centres in arcseconds over the
     truth's scale, A the position angle from the third answer's centre to the first's
     (`atan2(sin Δα·cos δ1, cos δ3·sin δ1 − sin δ3·cos δ1·cos Δα)`, Δα the first's RA
     less the third's, east of north) and θ the third answer's angle, it ends at
     P = (618.0 − d·sin(A − θ), 411.0 − d·cos(A − θ)). The test asserts that d is above
     20; and, with background 170 (160 counts of offset and 2 s of sky at 5 counts a
     second, by 030's constants) and over the samples whose centres lie within
     d/2 + 10 px, in x and in y, of the midpoint of (618.0, 411.0) and P, that the
     larger eigenvalue of the second moments of those samples, each weighted by its
     value less the background, less the smaller is within 10% of d²/12, and that
     their weighted mean position is within 1.0 px of that midpoint (oracle: the
     simulator's truth; the measure is 030 US2-S6's second half, with a window sized
     to the trail, since 41 × 41 samples do not hold one of 40 px or more; the test
     computes the separation and the angle itself, from the truths' RA and Dec).
2. **Given** two simulators built as in US1-S1 together (the two builds awaited with
   `tokio::join!`, so that both clocks read the same instant; built one after the other,
   the second starts after the first's 5 s wait), each with its altitude axis also
   commanded at 0.01°/s after its azimuth axis,
   **When** a test reads the first's `imaging_truth()` at t and t + 2 s only, and the
   second's `imaging_truth()` and `telescope().eyepiece` at t, t + 1 s and t + 2 s,
   **Then** a committed test asserts that the first's `imaging_truth_at(t + 1.0)`, asked
   at t + 2 s, has its centre within 0.02″ of the second's `telescope().eyepiece` RA and
   Dec read at t + 1 s, and its angle within 0.001° of the second's `imaging_truth()`
   read then (oracle: a twin read live, its centre through 030's `directions_at`, which
   FR-001 leaves alone, since `imaging_truth()` itself is answered from the memory; a
   constant rate makes a straight line between samples exact to well under 0.02″).
   **And** for twins pushed to (50.0, 359.85) in place of (50.0, 120.0), whose camera
   azimuth crosses 0° between t and t + 2 s, the same holds.
3. **Given** the simulator of US1-S1 after 10 s of turning, **When** a test reads the
   truth at t, waits 1 s, calls `push_to(40.0, 200.0)`, and asks `imaging_truth_at(t)`,
   **Then** a committed test asserts the answer's centre equals
   `sim.directions_at(t).eyepiece`'s RA and Dec within 10⁻⁹ degrees: a push by hand
   starts the memory again, and the answer is the one 030 gives (oracle:
   `crates/simulator/src/optics.rs:127`); and that 2 s after the push, after one read of
   `imaging_truth()` 1 s after it, `imaging_truth_at(t)` still equals
   `sim.directions_at(t).eyepiece`'s RA and Dec within 10⁻⁹ degrees. The same holds,
   each on a fresh simulator of US1-S1, when in place of the push the test calls
   `sim.set_optics` with `Optics { eyepiece_px: (600.0, 500.0), ..Optics::default() }`,
   `sim.connect_mount` with a second mount built as US1-S1's, or
   `sim.point_eyepiece_at(83.81867, -5.38967)`.
4. **Given** a simulator with no mount, pointed with `push_to(50.0, 120.0)`, **When** a
   test waits 20 s, reading `imaging_truth()` every second, and asks
   `imaging_truth_at(t)` for t 7.5 s back, **Then** a committed test asserts the answer
   equals `sim.directions_at(t).eyepiece`'s RA and Dec within 10⁻⁹ degrees (oracle: 030
   US2-S2's own).
5. **Given** a `SimMount::with_config` with no backlash and no base tilt, **Then** a
   committed test asserts that `count_steps()` is false; that with the azimuth axis at 0.0001°/s for
   20 s, `true_direction().az` read every 100 ms takes more than 100 distinct values;
   and that after `set_count_steps(true)` on a mount built the same way,
   `count_steps()` is true and the same 200 readings take between 8 and 11 distinct
   values, each a whole multiple of
   360/1,620,000° from the first within 10⁻⁹ degrees (worked example: 0.36″/s for 20 s
   is 7.2″, nine counts of 0.8″); and that, with count steps on, the reading 2.0 s after
   the rate command still equals the first: 0.72″ of drive is under one count of 0.8″,
   and the tube follows its drive rounded down; and that on a third mount built the same
   way, with count steps on and the altitude axis at 0.0001°/s, `true_direction().alt`
   read every 100 ms for 20 s takes between 8 and 11 distinct values.
6. **Given** the simulator of US1-S1, **When** a test reads `imaging_truth()` at t and
   then every 100 s for 3000 s with the axis still turning, **Then** a committed test
   asserts that `imaging_truth_at(t)` is within 0.02″ of the reading at t; and that
   after further reads every 100 s up to t + 3700 s, `imaging_truth_at(t)` equals
   `sim.directions_at(t).eyepiece`'s RA and Dec within 10⁻⁹ degrees: a sample older than
   the hour is dropped, and the answer is then the one 030 gives (oracle: the
   simulator's own truth, read live, and `crates/simulator/src/optics.rs:127`; owner
   decision 16's hour).
7. **Given** the simulator of US1-S1 after 10 s of turning, **When** a test reads
   `imaging_truth()` at t, waits 1 s, pushes the tube on the mount itself with
   `push_by(0.0, -1.0)` (`SimMount::push_by`, a degree down), reads `imaging_truth()`
   once more and asks `imaging_truth_at(t)`, **Then** a committed test asserts that the
   answer's centre is within 0.02″ of the reading at t and more than 1000″ from the
   reading after the push: a push made on the mount is a motion like any other and keeps
   the memory, where a push made through the simulator starts it again, as US1-S3 has it
   (oracle: the simulator's own truth, read live; FR-001). **And** on a fresh simulator
   of US1-S1, a read of `imaging_truth()`, `push_by(0.0, -1.0)` on the mount with no
   wait, and a second read give centres more than 1000″ apart: a read at the newest
   sample's instant replaces that sample (plan.md § The simulator's memory).

---

### User Story 2 - The tracking error from a sub (Priority: P1)

As the stack, I turn a sub's imaging solve into a tracking error: how far the sky at
the set point is from the target, east and north in arcseconds, at the sub's
mid-exposure. I hand it to whatever tracker is running through one interface that says
nothing of axes.

**Why this priority**: It is the signal everything else acts on, and the interface is
what keeps the imaging path fit for another mount.

**Independent Test**: `crates/finder/tests/console/steering_error.rs`, the module
`steering_error` of finder's `console` test binary. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test console steering_error::`.

**Acceptance Scenarios**:

1. **Given** the worked solve and the worked target, **When** a test calls
   `tracking_error(&solve, (618.0, 411.0), (83.81867, -5.38967), 1768528802.5)`,
   **Then** a committed test asserts `time` 1768528802.5 exactly, `east_arcsec`
   12.2934 ± 0.001 and `north_arcsec` −5.1480 ± 0.001 (worked example: the frame's
   centre is 0.00343° of right ascension east of M42 at Dec −5.39°, and 0.00143° south).
2. **Given** the same, with the set point (718.0, 361.0), **Then** a committed test
   asserts `east_arcsec` −120.768 ± 0.001 and `north_arcsec` 196.383 ± 0.001 (worked
   example: 100 px right and 50 px up of the centre is 3600 × (−0.0369615°, 0.0559808°)
   by the solve's matrix, added to US2-S1's answer).
3. **Given** a sensor 1236 × 822, **Then** a committed test asserts that
   `set_point_in_frame` answers (618.0, 411.0) for no set point, the whole sensor and
   binning 1; (109.0, 105.5) for no set point, the region (400, 200, 400, 400) and
   binning 2; and (718.0, 361.0) for the set point (718.0, 361.0), the whole sensor and
   binning 1; and (618.5, 411.5) for no set point on a sensor 1237 × 823, its whole
   region and binning 1 (the centre is half the size, not half rounded down).
4. **Given** a `Simulator` as `crates/finder/tests/support/live.rs:41` builds it, its
   truth at t, an `ImagingSolve::from_cd` the test builds from that truth with the
   matrix `[[−s·cos θ, −s·sin θ], [s·sin θ, −s·cos θ]]` (s the truth's scale in degrees,
   θ its angle), and a place T 100″ from the truth's centre at position angle 70°,
   **When** a `SimImagingCamera` with noise off, gain 0 and one white star of magnitude
   5 at T takes a 20 ms frame, **Then** a committed test asserts that with the star's
   centroid, taken with background 160, as the set point and T as the target the
   tracking error is within 0.7″ of (0, 0) (oracle: the rendered frame: the target's
   own image is where the error is nothing; 0.3 px at 2.18″ a pixel); and that with
   the frame's centre, (618.0, 411.0), as the set point it is (−93.97, −34.20) ± 0.05″
   (worked example: −100″ × (sin 70°, cos 70°)).
5. **Given** the worked solve, **Then** a committed test asserts that the target at the
   far side of the sky from the solve's centre, (263.8221, 5.3911), answers `None` (032's
   `pixel_of` answers none only where the cosine of the distance from the centre is
   zero or less; a target exactly a quarter turn away sits on that edge and is not
   used here), and so does the worked target with a time that is not finite, and so does
   the worked target with the set point (NaN, 411.0), and the target (NaN, −5.38967)
   with the set point (618.0, 411.0); and so does the worked target on
   `ImagingSolve::from_cd(83.8221, -5.3911, [[0.0, 0.0], [0.0, 0.0]], 1236, 822)`, whose
   results are not finite.
6. **Given** the text of `crates/finder/src/imaging/steering.rs`, read by the test from
   `CARGO_MANIFEST_DIR`, **Then** a committed test asserts it holds none of `ephemeris`,
   `alt_az`, `parallactic`, `counts_per_revolution`, `AxisInfo`, `MountClient`,
   `azimuth`, `altitude`, `field_rotation`, `hour_angle`, `meridian`, `backlash` and
   `keyhole` (oracle: the plan document's rule 1).
7. **Given** a type of the test's own that implements the five required methods of
   `Steered` and no other, **Then** a committed test asserts that
   `longest_exposure_s(0.001)` and `longest_exposure_ahead_s(0.001)` are `None`, that
   `steady()` is true, `steady_between(0.0, 1.0)` is true and `pause()` is `None`, and
   that `tune(0.5, 0.3)` returns: a tracker that says nothing limits nothing, is always
   steady and takes no tuning (oracle: FR-005).

---

### User Story 3 - The tracker steered by tracking errors (Priority: P1)

As the tracker, once a stack claims me I take my corrections from the tracking errors it
hands me: in each gap I move each axis by half of its error when that is two counts or
more, and I take 0.3 of the drift rate I measure between two subs out of its rate. An
error that reaches me late is judged against what my gaps did since it was measured. I
change nothing else until the next gap, and while a stack holds me the finder camera's
pointings change no rate between my gaps. Once the stack has gone quiet for eight gaps I
take my own corrections again, but only in its gaps and only as steps: the step a finder
pointing asks for waits for the next gap, and my rate stays as the subs left it. I stay
so until the stack hands me a tracking error again, or lets me go.

Thirteen scenarios that test this story's code are User Story 9's, which follows this
story on the chain (Work Graph).

**Why this priority**: This is the change that stops the smear.

**Independent Test**: `crates/finder/tests/motion/steering.rs`, the module `steering` of
finder's `motion` test binary. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test motion steering::`.

**Acceptance Scenarios**:

1. **Given** the tracker rig with a rate error of 0.01, **When** a test plays 40 subs of
   5 s, plays 40 subs of 5 s late on a second rig built the same way, and plays 20 subs
   of 5 s without feedback on a third, **Then** a committed test asserts that:
   - on the first rig, the first sub's true drift is at least 0.6″ (worked example: 1%
     of M42's 14.97″/s across the sky is 0.1497″/s, 0.749″ in 5 s);
   - on the first and on the second rig, every sub's true drift from the 12th on is at
     most 0.15″, and the length of every truth error em from the 12th sub on is at most
     3.0″ (worked example: each drift rate measured cuts the rate error to 0.7 of itself,
     and the 12th sub played late is the 9th such cut, 0.749″ × 0.7⁹ = 0.030″);
   - on the first rig, after the 40th sub, `state().tracking_offset[0]` is within 10%
     of −0.0099 times the azimuth feed-forward rate then, in arcseconds a second
     (worked example: an axis that runs 1.01 times its command needs its command cut by
     1/1.01), and `steered_by_subs()` is true after the first gap;
   - on the third rig, every true drift is at least 0.6″ and the tracking offset stays
     `[0.0, 0.0]` (oracle: the simulator's truth).
2. **Given** the tracker rig with a rate error of 0.01 and a second task that calls
   `tracker.pointing` every 2 s with the truth error plus Gaussian noise of σ 15″ on
   each of east and north, from a seeded generator of the test's own as
   `crates/finder/tests/motion/tracking/pi_loop.rs:289` has, **When** a test plays 20
   subs of 5 s without feedback, **Then** a committed test asserts that the median true
   drift is above 3.0″ (oracle: the simulator's truth; the noise is the research note's
   0.3′ to 0.5′ a finder solve. US9-S1 plays subs with feedback beside the same task).
3. **Given** the tracker rig with no rate error, **When** a test awaits one `gap`, hands
   the axis displacement (0.0, 1.9 counts) as a tracking error at the time now, and
   awaits a second `gap`, **Then** a committed test asserts that the second gap answers
   a `Stepped` of exactly (0.0, 0.0), that the tracking offset is exactly `[0.0, 0.0]`
   (a first error sets no rate), and that the truth error just after it is within 0.05″
   of the one just before (oracle: the simulator's truth and its transforms; the
   deadband is the larger of two counts, 1.6″ here, and 1″ on the sky, which at M42's
   43° of altitude is 1.37″ of azimuth and 1″ of altitude: two counts on both axes.
   US9-S2 has an error over it, on each axis).
4. **Given** the tracker rig with no rate error, **When** a test awaits a `gap`, hands
   the tracking error (12.0, −16.0) and awaits a second, **Then** a committed test
   asserts a `Stepped` within 0.1″ of (−6.0, 8.0) and a tracking offset of `[0.0, 0.0]`:
   a first error sets no rate. **When** it then waits 6 s, hands (6.0 + 2.0, −8.0) at
   the time now, and awaits a third gap, **Then** the `Stepped` is within 0.1″ of (−4.0,
   4.0) and the tracking offset is exactly `[0.0, 0.0]`: the second error shows 2″ of
   drift to the east since the first, but the step made between the two, about 5.2″ of
   azimuth and 9.2″ of altitude, is over two deadbands (3.2″ here) on both axes, and
   such a pair sets no rate. A mount that made a little more or less than the step asked
   for would show the same. **When** it waits 6 s more, hands (4.0, −4.0) at the time
   now, where that step left the target, and awaits a fourth, **Then** the `Stepped` is
   within 0.1″ of (−2.0, 2.0) and the tracking offset is still exactly `[0.0, 0.0]`: the
   step between, about 3.9″ and 4.9″, is over two deadbands on both axes again. **When**
   it waits 6 s more, hands (2.0 + 1.3, −2.0) at the time now and awaits a fifth,
   **Then** each axis's tracking offset is within 0.002″/s of −0.3 × d/Δt, where Δt is
   the time between the fourth and third errors' times as the test handed them, and d is
   that axis's part of a sky offset of (1.3, 0.0), by the differencing of
   `crates/finder/tests/motion/tracking/pi_loop.rs:481` (oracle: the simulator's horizon
   transform, differentiated by the test. The step between these two errors, about 2.0″
   of azimuth and 2.4″ of altitude, is under two deadbands on both axes, and the two
   changes, about +0.079″/s and +0.015″/s, are under the 0.1″/s one gap may make).
5. **Given** the tracker rig with no rate error, a first gap, the error (0.0, 0.0)
   handed over at the time now, a second gap, 6 s waited and then the axis displacement
   (−20.0″, 0.0) handed as an error at the time now, **When** a third gap is awaited,
   **Then** a committed test asserts that the tracking offset is within 0.002″/s of
   `[0.0, 0.0]` (a jump of about 3″/s on the azimuth axis is over twice the 0.752″/s the
   offset may hold, so it is not taken for a drift rate; the tolerance is for the
   altitude axis, where the tracker's own resolving of the same displacement leaves a
   few hundredths of an arcsecond, which is a drift rate of thousandths) and that the
   `Stepped` is within 0.2″ of −0.5 times the error. **When** after 6 s more the axis
   displacement (−100.0″, 0.0) is handed over at the time now and a fourth gap awaited,
   **Then** the `Stepped`'s length is within 3% of the length of the axis displacement
   (30.0″/cos a, 0.0), a being the altitude of M42's apparent place then: one gap moves
   an axis 30″ on the sky at most, which is 41.0″ of azimuth here; were the limit an
   angle of the axis, the step would be 22.0″ on the sky (oracle: the simulator's truth
   and transforms; the displacements are negative so that the step runs with the azimuth
   axis, which turns forward at M42's place then. US9-S3 has a step against the axis's
   motion).
6. **Given** the tracker rig with no rate error and a first gap, **Then** a committed
   test asserts that `tracker.pointing(30.0, 0.0)` called every 2 s for 20 s changes no
   rate: every rate in the log over those 20 s equals its axis's feed-forward rate
   within 0.001″/s, and the tracking offset stays `[0.0, 0.0]`; that `state().samples`
   holds at least 9 samples of 30″; and that on another rig,
   `tracker.pointing(30.0, 0.0)` called just before the first gap leaves every rate the
   log gains in the 2 s after that gap's answer equal to its axis's feed-forward rate
   within 0.001″/s: the claim ends the finder camera's proportional rate (oracle: the
   rig's rate log and the test's own feed-forward rate. The lapse, a release and the
   errors that keep no claim are US9-S4, US9-S5 and US9-S6).
7. **Given** the tracker rig with a rate error of 0.01 and ten subs of 5 s played,
   **Then** a committed test asserts of the rate log that between each gap's answer and
   the next gap's call every rate equals its axis's feed-forward rate plus the tracking
   offset read just after that answer, within 0.001″/s; that every gap put at least one
   rate for each axis into the log between its call and its answer; and that `target()`
   is `Some` before `tracker.stop("stopped")` and `None` after it, `steered_by_subs()`
   is false, a `gap` answers (0.0, 0.0) at once and the log gains no rate (oracle:
   015's own rule that nothing follows a stop, `specs/015-tracking/spec.md:229`. A gap
   before a tracker's first rates and a gap nobody waits for are US9-S7 and US9-S8).
8. **Given** the tracker rig with no rate error, a first gap, the error (0.0, 0.0)
   handed over at the time now, a second gap and 6 s waited, **When** the axis
   displacement (−6.0″, 0.0) is handed as an error at the time now and a third gap
   awaited, **Then** a committed test asserts that the azimuth tracking offset is within
   0.001″/s of +0.1 and the altitude tracking offset within 0.002″/s of 0.0 (worked
   example: −6″ of azimuth in 6.0 s is a drift rate of −1.0″/s, under the 1.504″/s of
   the jump test, and 0.3 of it is 0.3″/s; one gap changes an axis's offset by 0.1″/s at
   most. No step lies between the two errors, so the two-deadband rule does not stop
   it).
9. **Given** the tracker rig with no rate error and a first gap, **When** a test reads
   the console time t, hands the tracking error (12.0, −16.0) with the time t − 3.0,
   awaits a second gap, hands (12.0, −16.0) again with the time t − 1.0, and awaits a
   third gap, **Then** a committed test asserts that the second gap's `Stepped` is
   within 0.1″ of (−6.0, 8.0); that the third's is within 0.1″ of (−3.0, 4.0), not
   (−6.0, 8.0): the second error was measured before the first step, which has since
   halved it; and that the tracking offset is exactly `[0.0, 0.0]` after the second gap
   and within 0.002″/s of it after the third (worked example: (12, −16) + (−6, 8) =
   (6, −8), and half of that is taken out; no step lies between the two errors' times,
   so no drift rate was measured but the turn of the sky's axes in 2 s). **And** on
   another rig, after a first gap, the test reads the console time t, hands (12.0,
   −16.0) with the time t − 2.0 and then (6.0, −8.0) with the time t − 1.0, and awaits a
   gap: its `Stepped` is within 0.1″ of (−3.0, 4.0): of two errors handed over between
   two gaps the later is used. **And** on another rig, after a first gap, the test hands
   (12.0, −16.0) with the time t and awaits a gap, then hands (12.0, −16.0) with the
   time t − 1.0 and awaits another: that gap answers exactly (0.0, 0.0): an error no
   later than the last applied is ignored.
10. **Given** the tracker rig with no rate error and a first gap, **When** a test reads
    the console time t, hands the tracking error (0.0, 0.0) with the time t − 20.0 and
    awaits a gap; hands (0.5, 0.0) with the time t − 14.0 and awaits a gap, reading the
    tracking offset o1 after it; and hands (1.0, 0.0) with the time t − 8.0 and awaits a
    gap, reading the tracking offset o2, **Then** a committed test asserts that for each
    axis o1 is within 0.002″/s of −0.3 × d/6, d being that axis's part of a sky offset
    of (0.5, 0.0) by the differencing of
    `crates/finder/tests/motion/tracking/pi_loop.rs:481`; that o2 is within 0.003″/s of
    1.7 × o1 on each axis, and its azimuth part differs from 2 × o1's by more than
    0.006″/s; and that every `Stepped` is exactly (0.0, 0.0) (worked example: the third
    error shows the same drift rate as the second, but it was measured before the gap
    that took 0.3 of that drift rate out, so 0.7 of it is left and 0.3 of that is
    taken: 0.3 + 0.21 = 0.51 of d/6. M42 is east of the meridian at the rig's start, where a place
    to the east has the smaller azimuth: on the azimuth axis d is about −0.66″, so o1
    is about +0.033″/s, 1.7 × o1 +0.056″/s and 2 × o1 +0.066″/s. Both errors are under
    the deadband of two counts, 1.6″: 1.0″ east is about 1.33″ in azimuth and 0.24″ in
    altitude, in size, at M42's place then).
11. **Given** the tracker rig with no rate error, a first `gap`, and eight more gaps,
    each after 6 s of waiting and with no `steer` between them, so that
    `steered_by_subs()` is false after the ninth, **When** a test reads the tracking
    offset o and the truth error e0, calls `tracker.pointing(30.0, 0.0)`, waits 2 s,
    calls it again, waits 1 s and awaits a gap, reading the truth error e1 just after
    that gap's answer, **Then** a committed test asserts that:
    - from the first call to that gap's call every rate in the log equals its axis's
      feed-forward rate within 0.001″/s, o is exactly `[0.0, 0.0]`, and the tracking
      offset read just before the gap's call is exactly o: neither pointing changed a
      rate when it came;
    - the gap answers a `Stepped` within 0.1″ of (−18.0, 0.0), and e1 − e0 is within
      0.5″ of that `Stepped` (worked example: 015's proportional term would have taken
      0.6 of the newest pointing's error out over its interval, and the gap takes it as
      one step, −0.6 × (30.0, 0.0). At M42's place then, 30″ east is about −39.8″ of
      azimuth and −7.3″ of altitude, and both axes turn forward, so the step of about
      +23.9″ and +4.4″ runs with both and is under the 30″ on the sky, 41.0″ of azimuth
      here, that one gap may move an axis);
    - each axis's tracking offset after the gap is exactly o: after a lapse the finder
      camera's pointings step the telescope and change no rate, so the offset stays as
      the subs left it; and the last rate the log holds for each axis at or before that
      gap's answer equals its feed-forward rate plus the tracking offset within 0.01″/s:
      the pulse has ended;
    - `steered_by_subs()` is still false after that gap;
    - **And** a `steer` of (36.0, 0.0) with the console time read with o and e0, handed
      after that gap, makes the next gap answer a `Stepped` within 0.3″ of
      −0.5 × ((36.0, 0.0) + that gap's `Stepped`), about (−9.0, 0.0): the held step was
      remembered, and the error that takes a lapsed claim back is judged against it
      (oracle: the simulator's truth and its horizon transform, differentiated by the
      test, and 015's constants. A held step over one gap's limit is US9-S10, and a
      noisy finder camera after a lapse US9-S11).

---

### User Story 4 - Corrections only in the gap (Priority: P1)

As the stack, before every exposure I tell the console's steering that I am in a gap,
and I start the exposure only when it answers: by then the tracker has made its
correction and the mount has acknowledged its rates. For every sub I accept I hand over
its solve, and when I stop I let the tracker go.

**Why this priority**: A correction during an exposure is a smear. The gap is the only
time one is free.

**Independent Test**: `crates/finder/tests/console/steering_gap.rs`, the module
`steering_gap` of `console`, with the scripted tracker; and, for US4-S5 and US4-S6, which move
the simulated mount, `crates/finder/tests/motion/steering_gap.rs`, the module
`steering_gap` of `motion`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs both; during
development, `cargo nextest run -p finder --locked --test console steering_gap::` and
`cargo nextest run -p finder --locked --test motion steering_gap::`.

**Acceptance Scenarios**:

1. **Given** a scripted steering whose tracker's target is the worked target and whose
   `gap` waits 0.7 s, **When** a test awaits `before_exposure` with the worked ask (a
   wanted exposure of 5 s on the whole sensor at binning 1), calls `accepted` with the
   worked frame
   (`start` 1768528800.0, `exposure_s` 5.0) and the worked solve, and calls `release`,
   **Then** a committed test asserts that:
   - `before_exposure` answered 0.7 s of tokio time after it was called, with
     `exposure_s` 5.0 and `limited` false, and the tracker's log then held exactly one
     `gap`;
   - after `accepted` the log gained exactly one `steer`, with the time 1768528802.5
     and US2-S1's east and north within 0.001″;
   - `release` put one `release` in the log;
   - `view()` was `None` before the first call; after `before_exposure` its `source` is
     `subs` and `subs` 0; after `accepted`, `subs` is 1 and `last` is
     `{"time":1768528802.5,"east_arcsec":…,"north_arcsec":…}` with those numbers; after
     `release`, `source` is `tracker`, `subs` 0 and `last` absent.
2. **Given** a `Steering` whose source answers no tracker, **Then** a committed test
   asserts that `before_exposure` with the worked ask answers with no tokio time passed,
   `exposure_s` 5.0 and `limited` false; and that after `accepted` with the worked frame
   (`start` 1768528800.0, `exposure_s` 5.0) and the worked solve, `view()`'s `source` is
   `none`, its `subs` 0 and its `last` absent.
3. **Given** a scripted steering after one `before_exposure`, **Then** a committed test
   asserts that `accepted` with the worked frame at `start` 1768528700.0, a hundred
   seconds before that call, puts no `steer` in the log and leaves `subs` 0: no exposure
   of this stack holds its middle; that `view()`'s `holding` is false before
   `Steering::hold` is called, true while the `Hold` it answered lives and false once it
   is dropped; that dropping that `Hold` puts one `release` in the log; and that
   `accepted` with the worked frame after it puts no
   `steer` in the log. **And** on another scripted steering whose `gap` waits 0.7 s, a
   `before_exposure` dropped after 0.1 s, then the drop of the `Hold`, leave the log
   holding `gap` and then `release`: the tracker found is released even when no exposure
   was recorded. **And** on another, after one `before_exposure`, `accepted` with the
   worked frame at `start` 1768528799.99, 10 ms before the answer, puts one `steer` in
   the log: an exposure is found by its middle, not its start. **And** on another, after
   one `before_exposure`, the source's cell given a second scripted tracker, a second
   `before_exposure` and `accepted` with the worked frame at `start` 1768528800.0 put
   one `steer` in the second tracker's log and none in the first's: a frame goes to the
   tracker of the newest record that holds its middle.
4. **Given** a scripted steering with `set_set_point(Some((668.0, 386.0)))`, **Then** a
   committed test asserts that the `steer` after a `before_exposure` and `accepted` with
   the worked frame and solve carries −54.2373 ± 0.001 east and 95.6174 ± 0.001 north
   (worked example: 50 px right and 25 px up of the centre is 3600 × (−0.0184808°,
   0.0279904°) by the solve's matrix, half of US2-S2's offset, added to US2-S1's answer.
   US2-S2's own set point, (718.0, 361.0), would give a first error 230″ long, which the
   steering stands back from: US4-S8); and that
   with no set point, a `before_exposure` with the worked ask and a frame 200 × 200
   whose `roi` is (400, 200, 400, 400) at binning 2, solved by a solve 200 × 200 with
   the worked solve's centre and twice its matrix, it carries −33.2577 ± 0.001 east and
   −6.2848 ± 0.001 north (worked example: that frame's centre is the sensor's point
   (600, 400), so the sensor's centre, the set point, is (109.0, 105.5) in the frame,
   18 sensor pixels right of and 11 below the frame's centre; 3600 × the worked matrix
   × (18, 11) is (−45.5511, −1.1368), added to US2-S1's answer).
5. **Given** the tracker rig with a rate error of 0.01, built with one change: the
   mount's zero points put its axes on M42's catalogue place at the start, which the
   test computes with `simulator::sky::horizontal` and no refraction, so that the
   simulated imaging frame starts centred on the target; the rig's wait for its first
   rates; and a `Steering` on the rig's clock whose source answers the rig's tracker,
   **When** a test plays 30 subs of 5 s through it late, as 035's worker would (for
   each: await `before_exposure` with the worked ask; read the console time as the
   frame's `start`, and `sim.imaging_truth()`; wait 5 s; read `sim.imaging_truth()`
   again, which also leaves US1's memory the two samples it needs; wait 1 s; and, for
   every sub but the first, call `accepted` for the sub before this one, with the
   worked frame at that sub's `start` and `exposure_s` 5.0 and an `ImagingSolve` built
   with `from_cd` from `sim.imaging_truth_at` for that sub's mid-exposure, its `start`
   plus 2.5 s, as US2-S4 builds one), **Then** a committed test asserts, reading each
   tracking error handed over from `view()`'s `last` right after its `accepted`, that
   the first is at most 5.0″ long; that the first sub's drift, the separation between
   the centres of its two readings, is at least 0.6″; that from the 14th sub on every
   tracking error is at most 3.0″ long and every sub's drift is at most 0.15″; and
   that the rate log holds no rate between a `before_exposure`'s answer and 5 s after
   it that differs from its axis's feed-forward rate plus the tracking offset read at
   that answer by more than 0.001″/s (oracle: the simulator's truth, through US1's
   memory. The rig
   starts on the catalogue place because the simulator draws no refraction: started on
   the apparent place, 65″ higher, the first error would need the altitude axis to
   back up, which no correction may do).
6. **Given** the tracking console before Track is turned on, **Then** a committed test
   asserts that `session.steered()` is `None`; that once the session reports tracking it
   answers a tracker whose `target()` is M42's catalogue place, (83.81867, −5.38967),
   exactly; and that after `POST /api/track` with `{"on":false}` it is `None` again
   (oracle: the session's own Track state).
7. **Given** a console built as
   `crate::support::live::state(&crate::support::live::sim(), None, false).with_imaging(root)`,
   root a `Scratch` folder (`with_imaging` does nothing without the link that `state`
   gives it, `crates/finder/src/api.rs:342`), **Then** a committed test asserts that
   its imaging view has no `steering` key; and that after
   `state.imaging().unwrap().steering()` is given a source that answers no tracker and
   awaits one `before_exposure` with the worked ask, the view's `steering.source` is
   `none` and `steering.subs` 0 (oracle: FR-016).
8. **Given** a scripted steering, **When** a test awaits `before_exposure` with the
   worked ask and calls `accepted` with the worked frame (`start` 1768528800.0) and a
   solve whose tracking error is (200.0, 0.0), `ImagingSolve::from_cd` with the worked
   solve's matrix and size and the centre RA 83.81867° + 200/(3600 × cos 5.38967°), Dec
   −5.38967°, **Then** a committed test asserts that the tracker's log holds `gap` and
   then `release` and no `steer`; that `view()`'s `far_arcsec` is within 0.01 of 200.0,
   its `source` `tracker`, its `subs` 0 and its `last` absent; that a second
   `before_exposure` puts no second `gap` in the log, and `accepted` with the worked
   frame and the worked solve no `steer`; and that after `release`, one more
   `before_exposure` puts a `gap` in the log, `view()` has no `far_arcsec`, and
   `accepted` with the worked frame and a solve built the same way for (100.0, 0.0) puts
   one `steer` in the log and leaves `far_arcsec` absent (oracle: FR-014: the steering
   stands back from a stack whose first tracking error is over 180″, until its next
   `release`, and steers one whose first error is under it). **And** a further
   `before_exposure`, and `accepted` with a solve built the same way for (200.0, 0.0),
   put a second `steer` in the log and leave `far_arcsec` absent: only the first
   tracking error after a `release` is tested against 180″.
9. **Given** a `Steering` on a clock that follows tokio's time from 1768528800.0, whose
   source answers a scripted tracker whose `gap` waits 0.7 s, **When** a test awaits one
   `before_exposure` with the worked ask, **Then** a committed test asserts that
   `accepted` with the worked solve and a frame whose `start` is 1768528800.1 and
   `exposure_s` 0.4 puts no `steer` in the log, and that one with the worked frame at
   `start` 1768528800.7 puts one: an exposure is remembered from when `before_exposure`
   answered, not from when it was called (oracle: FR-013).
10. **Given** a scripted steering whose tracker has no target, **When** a test awaits
    `before_exposure` with the worked ask and calls `accepted` with the worked frame
    (`start` 1768528800.0) and the worked solve, **Then** a committed test asserts that
    the log holds no `steer` and that `view()`'s `subs` is 0, and that `view()` has no
    `far_arcsec`, and the log holds no `release` (oracle: FR-014).
11. **Given** a scripted steering after one `before_exposure` with the worked ask,
    **When** the test empties the source's cell and drops its own `Arc` of the tracker,
    **Then** a committed test asserts that a `Weak` of it taken before the drop no
    longer upgrades, and that `accepted` with the worked frame and solve leaves `subs`
    0: the steering keeps no tracker alive (oracle: FR-013).
12. **Given** a scripted steering with `set_set_point(Some((668.0, 386.0)))`, after one
    `before_exposure` with the worked ask and one `accepted` with the worked frame
    (`start` 1768528800.0) and the worked solve, so that its log holds one `gap` and one
    `steer` and `view()`'s `subs` is 1, **When** a test calls `set_set_point(None)`,
    **Then** a committed test asserts that the log gains exactly one `release`, and that
    `view()` has `subs` 0 and `last` absent; that after one more `before_exposure`,
    which puts a second `gap` in the log, `accepted` with the same frame and solve puts
    a `steer` in the log that carries US2-S1's east and north within 0.001″; and that a
    second `set_set_point(None)` puts no further `release` in the log (oracle: FR-014. A
    set point that changes lets the tracker go, so that no drift rate is measured across
    the move, and `None` puts it back at the sensor's centre, as 037 asks when a run
    starts its stack).

---

### User Story 5 - The exposure cap (Priority: P2)

As the observer, I choose how many pixels the field may turn at the corner of a sub
during one exposure. A stack then starts with no exposure longer than the tracker allows
for that over the hour ahead, halving the exposure I set as often as needed but never to
under a second, keeps that exposure while it runs, and the readout says when my exposure
was cut, when the limit has since fallen under it, and what a stack started now would
take. I can turn the feedback off, and set the two shares of its control law, for the
field.

Four scenarios that test this story's code on the tracker are User Story 12's, which
follows this story on the chain (Work Graph).

**Why this priority**: Field rotation is the one limit no correction removes. It is P2
because short exposures chosen by hand already stay under it.

**Independent Test**: `crates/finder/tests/console/steering_cap.rs`, the module
`steering_cap` of `console`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test console steering_cap::`.

**Acceptance Scenarios**:

1. **Given** a scripted steering whose tracker logs the turn it is asked about and
   answers a longest exposure of 37.15 s, **When** a test awaits `before_exposure` with
   the worked ask (the whole 1236 × 822 sensor at binning 1, a shortest exposure of
   0.001 s) and the wanted exposure each case gives, calling `release` between the
   cases, **Then** a committed test asserts that:
   - the turn asked about equals the test's own `6.0 / 618f64.hypot(411.0)` radians
     within 10⁻¹² (worked example: the corner is hypot(618, 411) = 742.189 px from the
     centre, and the smear is the default, 6 px);
   - a wanted exposure of 60 s answers 30.0 with `limited` true, and 20 s answers 20.0
     with `limited` false;
   - at binning 2 the turn is `6.0 / 309f64.hypot(205.5)`; for the region (400, 200,
     400, 400) at binning 1 it is `6.0 / 218f64.hypot(211.0)`; with the set point
     (718.0, 361.0) on the whole sensor it is `6.0 / 718f64.hypot(461.0)`; and for a
     sensor 1237 × 823, whole and at binning 1, it is `6.0 / 618.5f64.hypot(411.5)`, the
     centre being half the size and not half rounded down; each within 10⁻¹² (worked
     example: the farthest corners are 371.095 px, 303.389 px, 853.256 px and 742.889 px
     from the set point; the figures here are rounded and are not what the test compares
     with);
   - with a longest exposure of 6.0 s, a wanted 60 s answers 3.75; with a longest
     exposure of 8.0 s and a shortest exposure of 10 s it answers 15.0, `limited` true;
     with no longest exposure it answers 60.0, `limited` false; with a longest exposure
     of 30.0 s, a wanted 60 s answers 30.0, `limited` true: an exposure equal to the
     limit is not halved again; and with a longest exposure of 0.7 s, a wanted 5 s
     answers 1.25, `limited` true: the exposure is not halved to under 1 s, where two
     more halvings would have fitted it (no case here has a limit under half the
     exposure it answers: from 040's US6 such a call waits, 040's US6-S17).
2. **Given** the same steering and a first `before_exposure` that answered 30.0 for a
   wanted 60 s, **When** the tracker's longest exposure becomes 25.0 s and a test awaits
   `before_exposure` again without a `release`, for a wanted 60 s and then for a wanted
   10 s, **Then** a committed test asserts that both answer 30.0 with `limited` true:
   the exposure, and whether it was cut when it was decided, are held until `release`,
   whatever is asked for meanwhile; that `view()` has `exposure_s` 30.0, `limited` true,
   `longest_exposure_s` 25.0 and `over_limit` true; that right after `release`, before
   any further call, `view()` has no `exposure_s`, `limited` false and `over_limit`
   false; and that a `before_exposure` for a wanted 60 s then answers 15.0 with
   `limited` true, and `view()` has `over_limit` false; and, on another such steering,
   after its first `before_exposure`, when the tracker's longest exposure becomes
   exactly 30.0 s and `before_exposure` is awaited again, `view()` has
   `longest_exposure_s` 30.0 and `over_limit` false.
3. **Given** the same after `set_smear_px(1.0)`, **Then** a committed test asserts the
   turn asked about equals `1.0 / 618f64.hypot(411.0)` within 10⁻¹², and that
   `set_smear_px` refuses 0.05, 20.5 and a value that is not finite with
   `the corner smear must be a number from 0.1 to 20`. **And** on a steering never used,
   `set_smear_px(0.05)`, `set_step_share(1.5)` and `set_rate_share(-0.1)` are each
   refused and `view()` stays `None`: a refused value changes nothing, its use included.
4. **Given** a scripted steering whose tracker answers a longest exposure of 37.15 s,
   after a first `before_exposure` with the worked ask and a wanted exposure of 60 s,
   **Then** a committed test asserts `view()` has `smear_px` 6.0, `on` true,
   `step_share` 0.5, `rate_share` 0.3, `exposure_s` 30.0, `limited` true, `over_limit`
   false and `longest_exposure_s` 37.15, and after a further `before_exposure` whose
   source answers no tracker, no `longest_exposure_s`: the limit is the running
   tracker's, asked as the view is made, and no tracker is running; and on another,
   whose tracker names no longest exposure, after the same call, `longest_exposure_s` is
   the string `no limit`, `exposure_s` 60.0 and `limited` false.
5. **Given** 032's solving rig (032 spec.md § User Scenarios; 032's own test builds it
   at `crates/finder/tests/console/imaging_solve.rs:131`), **Then** a committed test
   asserts that:
   - `PUT /api/imaging/steering` with `{"smear_px":1.5}` answers 200 with the imaging
     view, whose `steering.smear_px` is 1.5 and `steering.on` true, and
     `GET /api/session`'s `imaging.steering` says the same;
   - `{"on":false}` answers 200 with `steering.on` false and `smear_px` still 1.5,
     `{"step_share":0.25,"rate_share":0}` answers 200 with `steering.step_share` 0.25
     and `steering.rate_share` 0.0, and `{}` answers 200 and changes nothing;
   - `{"smear_px":0}`, `{"smear_px":"2"}`, `{"on":1}`, `{"smear":2}`,
     `{"step_share":1.5}` and `{"rate_share":-0.1}` each answer 400 with a JSON `error`,
     the first exactly `the corner smear must be a number from 0.1 to 20`, the fifth
     exactly `the step share must be a number from 0 to 1` and the sixth exactly
     `the rate share must be a number from 0 to 1`, and change nothing; after
     `{"on":false}`, `{"on":true,"smear_px":0}` answers 400
     and `on` is still false; and `{"on":null}` and `{"smear_px":null}` each answer 400;
     `{"rate_share":-0.1,"step_share":1.5,"smear_px":0}` answers 400 with the corner
     smear's words, and `{"rate_share":-0.1,"step_share":1.5}` with the step share's:
     the three are checked in the order smear, step share, rate share;
   - `{"on":true}` with a foreign `Origin` answers 403; the same body sent with
     `Content-Type: text/plain` answers 415; the body `not json` sent as
     `application/json` answers 400; and none of the three changes anything: `on` is
     still false;
   - a console built the same way without `with_imaging` answers 409 with
     `this console takes no imaging frames`.
6. **Given** a scripted steering never used, whose tracker answers a longest exposure
   of 37.15 s and, for the hour ahead, of 20.0 s, **Then** a committed test asserts
   that `view()` is `None`; that `view_for(Some(ask))`, the ask being the worked ask
   with a wanted 60 s, has `source` `tracker`, `longest_exposure_s` 37.15,
   `would_take_s` 15.0, no `exposure_s` and `limited` false, and leaves `view()`
   `None`; that when the tracker names no longest exposure it has `longest_exposure_s`
   `no limit` and `would_take_s` 60.0; that when the source answers no tracker
   `view_for(Some(ask))` is `None`; and that after a `before_exposure` for a wanted
   60 s that found the first tracker, `view_for` with the worked ask and a wanted 10 s
   has `exposure_s` 15.0 and `would_take_s` 10.0: what a stack started now would take
   is worked from the ask given, whatever a running stack holds. **And** on 032's
   solving rig, with the console's steering given a source that answers a scripted
   tracker naming no longest exposure and no other call made, `GET /api/imaging`'s
   `steering.longest_exposure_s` is `no limit` and its `steering.would_take_s` equals
   that view's `settings.exposure_ms` over 1000: the console asks with its own camera
   and settings (oracle: FR-018). **And** on another scripted steering, whose tracker
   answers a longest exposure of 37.15 s, after a `before_exposure` with the worked ask
   and a wanted 60 s, `view_for` with the worked ask at binning 2 and a wanted 10 s asks
   the tracker's limit now for the turn `6.0 / 618f64.hypot(411.0)` within 10⁻¹²: the
   limit shown is for the stack's own ask, and only `would_take_s` for the ask given.
   **And** on 032's solving rig, after `PUT /api/imaging/settings` with `exposure_ms`
   8000, with the console's steering given a scripted tracker whose limit is 1000 s now
   and 0.3 s for the hour ahead, `GET /api/imaging`'s `steering.would_take_s` is 1.0,
   and the turn the tracker is asked about is `6.0 / (w / 2.0).hypot(h / 2.0)` within
   10⁻¹², w and h being that view's `camera.width` and `camera.height`: the console asks
   with its camera's own shortest exposure and sensor.
7. **Given** a scripted steering whose tracker answers a longest exposure of 37.15 s,
   with `set_on(false)`, **When** a test awaits `before_exposure` with the worked ask
   and a wanted 60 s and calls `accepted` with the worked frame and solve, **Then** a
   committed test asserts that the log holds no `gap`, no `tune` and no `steer`, that
   the call answered 30.0 with `limited` true, and that `view()`'s `source` is `tracker`
   and its `on` false: the switch turns off the steering and not the cap. **And** on a
   steering never used, `set_on(false)` alone makes `view()` answer, with `on` false.
8. **Given** a scripted steering, **When** a test awaits `before_exposure` with the
   worked ask, calls `set_step_share(0.25)` and `set_rate_share(0.0)`, and awaits
   `before_exposure` again, **Then** a committed test asserts that the tracker's log
   holds `tune(0.5, 0.3)` right before its first `gap` and `tune(0.25, 0.0)` right
   before its second; that `view()` has `step_share` 0.25 and `rate_share` 0.0; that
   `set_step_share` refuses −0.1, 1.5 and a value that is not finite with
   `the step share must be a number from 0 to 1`, and `set_rate_share` the same three
   with `the rate share must be a number from 0 to 1`, each leaving the view as it was;
   and that both accept 0.0 and 1.0 (oracle: FR-030).
9. **Given** a scripted steering after one `before_exposure` with the worked ask and one
   `accepted` with the worked frame (`start` 1768528800.0) and the worked solve, so that
   its log holds one `gap` and one `steer` and `view()` has `source` `subs` and `subs`
   1, **When** a test calls `set_on(false)`, **Then** a committed test asserts that the
   log gains exactly one `release`; that `view()` has `source` `tracker`, `subs` 0, no
   `last` and `exposure_s` still 5.0; that a second `before_exposure` puts no second
   `gap` in the log and answers 5.0; and that after `set_on(true)` a third puts one
   (oracle: FR-019. Turned off while a stack runs, the switch lets the tracker go at
   once, so that its own corrections steer).
10. **Given** a scripted steering whose tracker answers a longest exposure of 37.15 s
    and, for the hour ahead, of 20.0 s, **When** a test awaits `before_exposure` with
    the worked ask and a wanted 60 s, **Then** a committed test asserts that it answers
    15.0 with `limited` true, and that `view()` has `exposure_s` 15.0,
    `longest_exposure_s` 37.15 and `over_limit` false: the exposure is decided from the
    shortest limit of the hour ahead and held, and the view shows the limit as it is now
    (oracle: FR-018; worked example: 60 s halved twice is 15 s, the first value not over
    20 s).

---

### User Story 9 - The claim's edges, tested on the tracker (Priority: P1)

As the factory that builds User Story 3, I test in a story of their own what that
story's tracker does at the edges of a claim: an error over the deadband and one against
an axis's motion, the lapse and what it holds, a release, a gap before the first rates,
a followed push, and the finder camera beside all of it.

This story is built after User Story 3 and before User Story 4 (Work Graph). Its thirteen
scenarios test User Story 3's code: this story writes nothing under `crates/finder/src`,
and on User Story 3's code its scenarios already pass (tasks.md, US9's test tasks). Its
module writes the tracker rig out again, as every module that uses it does.

**Why this priority**: A claim that outlived its stack, a lapse that changed a rate
inside an exposure or a followed push that steered back to the old place would each
smear subs with no error shown. It follows User Story 3, whose code it tests, and comes
before User Story 4, since both add a `mod` line to
`crates/finder/tests/motion/main.rs`.

**Independent Test**: `crates/finder/tests/motion/steering_claim.rs`, the module
`steering_claim` of finder's `motion` test binary. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test motion steering_claim::`.

**Acceptance Scenarios**:

1. **Given** the tracker rig with a rate error of 0.01 and the second task of US3-S2,
   which calls `tracker.pointing` every 2 s with the truth error plus Gaussian noise of
   σ 15″ on each of east and north, **When** a test plays 40 subs of 5 s with feedback,
   **Then** a committed test asserts that every true drift from the 12th on is at most
   0.15″: the finder camera's noise steers nothing while the subs steer (oracle: the
   simulator's truth; the noise is the research note's 0.3′ to 0.5′ a finder solve, and
   US3-S2 shows what the same pointings do to subs played without feedback).
2. **Given** the tracker rig with no rate error, **When** a test awaits one `gap`, hands
   the axis displacement (0.0, 2.1 counts) as a tracking error at the time now, and
   awaits a second `gap`, **Then** a committed test asserts that the second gap's
   `Stepped` is within 0.05″ of −0.5 times the error handed over, that the truth error
   changes across the gap by that `Stepped` within 0.05″, that the gap takes between
   0.5 s and 0.6 s, and that every azimuth rate the rate log holds from the gap's call
   to its answer equals the azimuth feed-forward rate at its time within 0.001″/s.
   **And** on another tracker rig, for the axis displacement (3.0 counts, 0.0), the same
   holds with the two axes exchanged: the altitude rates are the ones that stay at the
   altitude feed-forward rate (oracle: the simulator's truth and its transforms; the
   deadband is the larger of two counts, 1.6″ here, and 1″ on the sky, which at M42's
   43° of altitude is 1.37″ of azimuth and 1″ of altitude: two counts on both axes, so
   no figure here moved when the 1″ became an angle on the sky. US3-S3 has an error
   under it).
3. **Given** the tracker rig with no rate error and a first gap, **When** the axis
   displacement (60.0″, 0.0) is handed as an error at the time now and a second gap
   awaited, **Then** a committed test asserts that the `Stepped`'s length is within 0.3″
   of the length of the axis displacement (r/2, 0.0), r being the azimuth feed-forward
   rate then in arcseconds a second, about 19.9: a step against the axis's motion can
   only hold the axis still for the half second (oracle: the simulator's truth and
   transforms; the azimuth axis turns forward at M42's place then, and the step this
   error asks for runs back against it. US3-S5 has the steps that run with the axis).
4. **Given** the tracker rig with no rate error and a first gap, **When** a test awaits
   eight more gaps with no `steer` between them, the eighth called 0.2 s after the
   seventh answers, and then a ninth, **Then** a committed test asserts that
   `steered_by_subs()` is true after the seventh and false after the eighth, that the
   rate log gains no rate from the eighth gap's call to its answer, and that the ninth
   leaves it false and answers exactly (0.0, 0.0): a claim that lapsed stays lapsed
   while no tracking error comes. **When** the test then calls
   `tracker.pointing(30.0, 0.0)` twice 2 s apart and waits 1 s after the second,
   **Then** neither changes a rate: every
   rate the log gains from the first call until the next gap's call equals its axis's
   feed-forward rate within 0.001″/s, and the tracking offset stays exactly `[0.0, 0.0]`
   (US3-S11 says what a gap does with those pointings). **When** it then calls
   `tracker.steer` with (0.0, 0.0) at the time now and awaits one more gap, **Then**
   `steered_by_subs()` is true again, that gap answers exactly (0.0, 0.0), and the
   tracking offset read after it is the one read just before it, exactly: the error that
   takes the tracker back sets no rate, and the step held from the two pointings is
   forgotten. **When** it awaits eight more gaps with no `steer` and one after them,
   **Then** `steered_by_subs()` is false again and that last gap answers exactly (0.0,
   0.0): nothing held before the claim was taken back survives it (oracle: the rig's
   rate log and the test's own feed-forward rate; the eight are 035's queue of four
   waiting subs, the one in work and the three of before). **And** on another rig, a
   first gap, seven gaps with no `steer`, a `steer` of (0.0, 0.0) at the time now, a
   gap and seven more gaps with no `steer` leave `steered_by_subs()` true: the eight
   are eight in a row with no tracking error.
5. **Given** the tracker rig with no rate error and a first gap, **When** a test calls
   `release()`, **Then** a committed test asserts that `steered_by_subs()` is false at
   once; that a `steer` of (12.0, −16.0) at the time now, made after it, is dropped, so
   the next gap answers exactly (0.0, 0.0); and that this gap makes `steered_by_subs()`
   true again, as a new stack's first gap does. **And** on another rig, after a first
   gap, a `steer` of (12.0, −16.0) at the time now and then `release()` leave the next
   gap answering exactly (0.0, 0.0): `release` forgets an error not yet applied. **And**
   on another rig whose claim has lapsed as in US9-S4, `release()` and one more gap make
   `steered_by_subs()` true: a released tracker is claimed by the next gap, lapsed or
   not (oracle: FR-011). **And** on another rig, `tracker.pointing(30.0, 0.0)` called,
   2 s waited, a first gap, `release()` and `tracker.pointing(30.0, 0.0)` again leave
   the tracking offset exactly `[0.0, 0.0]`: the claim forgot the pointing before it, so
   the first pointing after a release adds nothing to 015's integral (plan.md § The
   tracker, **`Shared::pointing`**).
6. **Given** the tracker rig with no rate error and a first gap, **When** a test awaits
   eight more gaps, each preceded by a `steer` at the time now whose east is NaN,
   **Then** a committed test asserts that `steered_by_subs()` is false after the eighth:
   an error that is not finite is ignored and keeps no claim. **And** on another rig a
   `steer` of (12.0, −16.0) at the time now, made before any gap, leaves the next gap
   answering exactly (0.0, 0.0) with a tracking offset of exactly `[0.0, 0.0]`: an error
   handed over before a stack's first gap is dropped (oracle: FR-011).
7. **Given** a tracker rig with no rate error, built without the wait for its first
   rates, **When** a test awaits `gap` as the first thing after `start` answers,
   **Then** a committed test asserts that it answers exactly (0.0, 0.0), that
   `steered_by_subs()` is false after it, and that, 0.1 s later, the first rate the log
   holds for each axis equals that axis's feed-forward rate at its time within 0.001″/s:
   a gap that comes before a tracker's first rates claims nothing and sends nothing, so
   no axis is sent a zero before it has started (oracle: the rig's rate log and the
   test's own feed-forward rate).
8. **Given** the tracker rig with no rate error, **When** a test drops a `gap` future
   after one poll (a timeout of zero) and waits 0.1 s, **Then** a committed test asserts
   that `steered_by_subs()` is false: a gap nobody waits for claims nothing (oracle:
   plan.md § Plan traps 18).
9. **Given** the tracking console, **When** a test waits 4.5 s after the session first
   reports `tracking`, so that the tracker has sent its first rates and, from 040's US6,
   settled, takes the session's tracker (`state.session().tracker()`), awaits `gap`, and
   for 20 s hands the error (0.0, 0.0) at the time now and awaits `gap` every 2 s,
   **Then** a committed test asserts that the newest sync's `time` in the session's
   pointing model advanced by at least 15 s over those 20 s (the model keeps one sync of
   a place and replaces it, `crates/finder/src/model.rs:524`), that the tracker's
   `state().samples` grew, that `steered_by_subs()` is true throughout, and that the
   tracking offset at the end is the one read after the first gap within 0.002″/s
   (oracle: the session's own model; the finder camera still feeds it).
10. **Given** the tracker rig with no rate error, a first `gap` and eight more gaps,
    each after 6 s of waiting and with no `steer` between them, so that the claim has
    lapsed, **When** a test calls `tracker.pointing(100.0, 0.0)`, waits 2 s, calls it
    again, waits 1 s and awaits a gap, **Then** a committed test asserts that the gap's
    `Stepped` has a length within 3% of that of the axis displacement (30.0″/cos a, −0.6
    × d₁), a being the altitude of M42's apparent place then and d₁ the altitude part of
    a sky offset of (100.0, 0.0) by the differencing of
    `crates/finder/tests/motion/tracking/pi_loop.rs:481`: a held step moves an axis 30″
    on the sky at most, about 33″ in all here, against 60″ unclamped (oracle: the
    simulator's truth and its horizon transform, differentiated by the test, and 015's
    constants. US3-S11 has a held step under the limit). **And** on another rig lapsed
    the same way, `tracker.pointing(30.0, 0.0)`, 2 s waited,
    `tracker.pointing(3.0, 4.0)`, 1 s waited and a gap: the gap answers exactly (0.0,
    0.0): a pointing inside 015's 10″ deadband clears the held step, as it zeroes 015's
    proportional rate.
11. **Given** the tracker rig with a rate error of 0.01 and the second task of US3-S2,
    which calls `tracker.pointing` every 2 s with the truth error plus Gaussian noise of
    σ 15″ on each of east and north, **When** a test awaits a first gap, eight more 6 s
    apart with no `steer`, and then ten more 6 s apart, **Then** a committed test
    asserts that `steered_by_subs()` is false after the ninth gap and after every later
    one; that from the answer of each of the ten last gaps to the next gap's call, and
    after the last of them for the 6 s that follow its answer, every rate in the log
    equals its axis's feed-forward rate plus the tracking offset read just after that
    answer within 0.001″/s; that the tracking offset read after the last of the ten is
    exactly the one read after the ninth gap; and that at least one of the ten gaps
    answered a `Stepped` longer than 1.0″: once the subs have lapsed, the finder
    camera's corrections reach the mount in the gaps, and only there, and only as steps
    (oracle: the rig's rate log and the test's own feed-forward rate; with σ 15″ on each
    coordinate a pointing falls inside 015's 10″ deadband at most about one time in
    five, 1 − e^(−100/450), so the chance that the newest pointing before each of those
    ten gaps was inside it every time is under 10⁻⁶, whatever the seed).
12. **Given** the tracker rig with no rate error, **Then** a committed test asserts that
    after a first gap, 6 s waited and a `steer` of (12.0, −16.0) at the time now,
    `follow` to a place 10′ east (of M42's catalogue place) answers Ok,
    `steered_by_subs()` is false at once, and the next gap answers exactly (0.0, 0.0)
    and claims again; that a `steer` of (−600.0, 0.0) whose time is 2.5 s before the
    `follow`, handed over after that gap, leaves the gap after it answering exactly
    (0.0, 0.0); and that, 1 s later, a `steer` of (12.0, −16.0) at the time now makes
    the next gap answer within 0.1″ of (−6.0, 8.0) (oracle: 038's `Tracker::follow` and
    FR-011: an error measured from the old place steers nothing, whether it was waiting
    at the `follow`, as `release` forgets one, or is handed over after the claim was
    taken again, as 035's worker hands over a sub taken before the push. −600″ east is
    such a sub's error against the new place).
13. **Given** the tracker rig with no rate error, a first gap and eight more with no
    `steer`, so that the claim has lapsed, and `tracker.pointing(30.0, 0.0)` called
    twice 2 s apart, **When** the test calls `follow` to a place 10′ east of M42's
    catalogue place and awaits a gap, **Then** a committed test asserts that `follow`
    answers Ok, and that the gap answers exactly (0.0, 0.0) and makes
    `steered_by_subs()` true: `follow` forgets the step a lapsed claim held and clears
    the lapse, so the stack's next gap claims (oracle: FR-011; without it that gap would
    step about (−18.0, 0.0), as in US3-S11, and claim nothing).

---

### User Story 12 - The cap's limits and tuning, tested on the tracker (Priority: P2)

As the factory that builds User Story 5, I test in a story of their own what that
story's tracker answers on the simulated mount: its longest exposure now and over the
hour ahead, against 015's own functions, its answer at the instant a target is due east,
and the shares it takes when it is tuned.

This story is built after User Story 5 (Work Graph). Its four scenarios test User Story
5's code: this story writes nothing under `crates/finder/src`, and on User Story 5's code
its scenarios already pass (tasks.md, US12's test tasks). Its module writes the tracker
rig out again, as every module that uses it does.

**Why this priority**: A limit that disagreed with 015's, or a tracker that ignored its
tuning, would cap or steer every stack wrongly with no error shown. It is P2, as User
Story 5 is, and follows that story, whose code it tests.

**Independent Test**: `crates/finder/tests/motion/steering_cap.rs`, the module
`steering_cap` of finder's `motion` test binary. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test motion steering_cap::`.

**Acceptance Scenarios**:

1. **Given** the tracker rig with no rate error, **Then** a committed test asserts that
   `tracker.longest_exposure_s(2.0 / 618f64.hypot(411.0))` equals, within one part in a
   million, the seconds `ephemeris::longest_exposure(23.63232, 15.71664, 19.12, r, 2.0)`
   answers (`crates/finder/src/ephemeris.rs:105`), r being the field rotation the test
   computes for M42 with `sky::alt_az` and `ephemeris::altaz_rates_from_alt_az`, as
   `crates/finder/tests/motion/tracking/routes.rs:312` does, in radians a second, at
   the console time it reads immediately before the call with no await between the
   two; and that it is between 30 s and 45 s (worked example: 1236 × 822 pixels of
   19.12 µm are 23.63232 × 15.71664 mm; at 14.91°/h the answer is 37.28 s). The same
   holds, with the same oracle, for M81 (`targets::find("M81")`), whose field rotation
   at the rig's start is negative; through `&dyn Steered` the answer is the same;
   `longest_exposure_s` answers `None` for a turn of 0.0, −1.0 and NaN; and after
   `stop("stopped")` it answers `None`.
2. **Given** a tracker rig whose target is due east at an instant the test chooses, t* =
   1768528830.0, 30 s after the rig's clock starts: with Dec fixed at +18.75° the test
   steps RA through 0°, 1°, … 359°, takes the first two neighbours whose azimuths at t*
   by `sky::alt_az` for 40° N 75° W lie either side of 90° with both altitudes above 0°,
   and bisects RA between them 60 times, so that the azimuth at t* is 90° within 10⁻⁹
   degrees (the altitude is then near 30°); the rig's target is a clone of M42's
   `Target` with that `ra_deg` and `dec_deg`, and its mount's zero points put its axes
   on that place's apparent direction at the start, as the tracker rig does for M42;
   **When** the test sleeps until the rig's clock reads t* to within 2 ms (tokio's timer
   counts in milliseconds) and calls `longest_exposure_s(0.0027)` with no await after
   the sleep, **Then** a committed test asserts it is `None`; and that after 10 s more
   it is `Some` of more than 3600 s (oracle: 015's own function and its 10⁻⁹ rad/s rule.
   Worked example: due east at this place the azimuth moves 9.7″ a second, and the field
   rotation passes 10⁻⁹ rad/s 3.2″ of azimuth from due east, a third of a second either
   side of t*, so the instant matters and 2 ms is 0.02″; 10 s later the azimuth is 97″
   past east, the rotation 3 × 10⁻⁸ rad/s and the answer about 89,000 s).
3. **Given** the tracker rig with no rate error, **Then** a committed test asserts that
   `tracker.longest_exposure_ahead_s(2.0 / 618f64.hypot(411.0))` equals, within one
   part in a million, the smallest of the 61 values `turn / r.abs()` the test computes,
   r as US12-S1 computes it, for M42 at the console time it reads immediately before the
   call, with no await between the two, and at each whole minute after that time up to
   the sixtieth; that it is less than `tracker.longest_exposure_s` of the same turn
   asked at that instant, and between 33 s and 36 s; that through `&dyn Steered` the
   answer is the same; that it is `None` for a turn of 0.0, −1.0 and NaN; and that
   after `stop("stopped")` it is `None` (oracle: 015's own functions, sampled by the
   test as 015 samples its forecasts. Worked example: the rig starts 54 minutes before
   M42's transit, where the field turns fastest, 16.19°/h against 14.91°/h at the
   start, so the shortest limit of the hour is the transit's: 34.33 s for this turn,
   against 37.28 s at the start).
4. **Given** the tracker rig with no rate error and a first gap, **When** a test calls
   `tracker.tune(1.0, 0.6)`, hands the error (0.0, 0.0) at the time now, awaits a gap,
   waits 6 s, hands (0.5, 0.0) at the time now and awaits a gap, reading the tracking
   offset o after it, waits 6 s more, hands (12.0, −16.0) at the time now and awaits
   one more gap, **Then** a committed test asserts that for each axis o is within
   0.002″/s of −0.6 × d/6, d being that axis's part of a sky offset of (0.5, 0.0) by
   the differencing of `crates/finder/tests/motion/tracking/pi_loop.rs:481`, and that
   its azimuth part is above +0.05″/s; that the last gap's `Stepped` is within 0.2″ of
   (−12.0, 16.0); and that on another rig, after `tracker.tune(0.5, 0.0)`, the same
   first two errors leave the tracking offset exactly `[0.0, 0.0]` (oracle: the
   simulator's truth and its horizon transform, differentiated by the test. Worked
   example: 0.5″ east is about −0.66″ of azimuth, under the deadband, so no step lies
   between the first two errors; with a rate share of 0.6 the azimuth offset is about
   +0.066″/s, where the default 0.3 gives +0.033″/s; and with a step share of 1.0 the
   whole of (12, −16) is stepped out, about 10.4″ and 18.5″ of axis angle, which runs
   with both axes and is under one gap's limit). **And** on another rig, after a first
   gap, `tune(2.0, f64::NAN)` then `tune(f64::NAN, f64::NAN)` called through
   `&dyn Steered`, and the error (12.0, −16.0) at the time now, make the next gap
   answer within 0.2″ of (−12.0, 16.0): a share is held to 0 to 1, a NaN leaves it as it
   was, and the trait reaches the tracker's own `tune`.

---

### Edge Cases

- **Stop.** Stop ends tracking as before. A gap that is waiting answers at once with no
  step, one still queued behind a take-up included, and no rate follows the Stop
  (US3-S7, 040's US6-S10).
- **A followed push.** 038's `follow`, as `release` does, ends the claim, clears a lapse
  and forgets what was pending or held: an error measured from the old place steers
  nothing, whenever it is handed over, and the stack's next gap claims again (US9-S12,
  US9-S13).
- **Subs that are not accepted.** Each is a gap with no tracking error. After eight gaps
  in a row with none, the tracker's own corrections steer again, with the tracking
  offset as it stood, and go on steering through every later gap until a sub is
  accepted, applied only in the gaps and only as steps: a finder pointing between two
  gaps holds its step for the next and changes no rate (US3-S11). The gap after that
  sub's tracking error takes the tracker back, forgets what was held and steps without
  setting a rate: no drift rate is measured across the break (US9-S4).
- **The switch off.** No stack claims the tracker, so its own corrections run as 015 has
  them, during exposures too: the observer's declared exception (owner decision 8).
  Turned off while a stack runs, it lets the tracker go at once; turned on again, the
  stack's next gap claims it (US5-S9).
- **A set point named during a stack.** A set point that changes lets the tracker go in
  the same way, so that no drift rate is measured across the move, and the stack's next
  gap claims afresh (US4-S12).
- **A gap before a tracker's first rates.** It answers at once, moves nothing and claims
  nothing (US9-S7); from 040's US6 the stack does not ask for one, because such a
  tracker is not steady yet.
- **A tracking error that arrives late.** It is judged against the steps and rate
  changes made since its mid-exposure (US3-S9, US3-S10). Of two that arrive between two
  gaps, the newer is used.
- **The worker falls behind, or is slow.** A gap with no tracking error counts toward
  the lapse whatever the reason: a sub dropped as `behind`, `settings`, `frame_failed`,
  `unreadable` or `solver_failed`, or a sub still being solved. The lapse comes at the
  eighth such gap in a row, so a worker a whole queue behind (035 keeps four subs
  waiting and one in work) does not lapse the claim. A console that needs more than
  eight sub lengths for one sub lapses between every two tracking errors: each error
  then steps the telescope and takes no drift out of the rate.
- **A wrong imaging solve, a bump.** 035 does not accept a sub whose solve differs from
  the reference sub's in scale or lies off the field, so it steers nothing. For one that
  passes, and for a tube that was knocked between two subs, one gap moves an axis 30″ on
  the sky at most, a jump between two subs faster than twice the largest tracking offset
  changes no rate, and no gap changes an axis's rate by more than 0.1″/s (US3-S5,
  US3-S8).
- **A mount that does not make the step asked for.** A pair of errors between which the
  gaps stepped an axis by more than two deadbands changes no rate on that axis. So while
  a large offset is taken out, a pulse that moves the tube a little more or less than
  asked is not read as a drift; the rate is measured once the steps are small (US3-S4).
- **A stack that starts far from its set point.** When a stack's first tracking error is
  over 180″ the steering stands back for the rest of that stack: it lets the tracker go
  and calls neither `gap` nor `steer`, the tracker's own corrections run as 015 has
  them, the cap, the pause and the drop still apply, and the view says how far out the
  target is. The finder camera holds the target on its aim point and the subs on the set
  point; steering between two that differ by more than 038's 5′ would read as a push by
  hand (owner decision 7; US4-S8, 040's US7-S7).
- **A correction against an axis's motion.** An axis is never reversed by a step or a
  rate: a step against its motion can at most hold the axis still for the half second,
  and what is left waits for the next gap (FR-008; US9-S3). That is the axis's own rate
  times half a second a gap: about 10″ of azimuth and 1.8″ of altitude on the tracker
  rig, and nearly nothing where an axis crawls near its reversal. A target left 120″ too
  high east of the meridian is so brought in over about 67 gaps, not four.
- **A sub of a region, or binned.** The set point is a place on the sensor; it need not
  lie inside the frame (US4-S4).
- **The limit falls under the exposure in use.** The exposure was decided from the
  shortest limit of the hour ahead (US5-S10, US12-S3), so this comes to a stack more
  than an hour old, or to one whose exposure the floor of 1 s kept above that limit. The
  stack keeps its exposure and the readout says `over_limit`; a new stack starts shorter
  (US5-S2).
- **The cap near the zenith.** The limit shrinks toward the keyhole. The exposure is not
  halved to under 1 s, nor past the camera's shortest where that is longer, and may then
  be both `limited` and `over_limit` (US5-S1); with the limit under half of it the stack
  waits (040's US6-S17).
- **A stack started while the limit is low.** It holds a short exposure for its whole
  life while the limit climbs. The block's `A stack started now would take` says what a
  new stack would hold (US5-S6, 040's US8-S4).
- **Master darks.** A halved exposure needs a master dark of its own length; without
  one 034's fallback applies and says so.
- **Count steps.** With the simulator's count steps on, the truth read at an instant
  jitters by up to a count; the scenarios that bound a drift leave them off.

## Requirements *(mandatory)*

### Functional Requirements

**The simulator** (US1):

- **FR-001**: The simulator SHALL remember the direction of its telescope for the last
  3600 s of its clock, one sample for every read of it, and begin again when the
  telescope is pushed by hand through the simulator (`push_to`, `push_by`,
  `point_eyepiece_at`), a mount is connected or the finder's optics are set
  (`set_optics`); a push made on the mount itself (`SimMount::push_by`,
  `SimMount::push_to`) is a motion like any other and keeps the memory. 030's
  `imaging_truth_at` and the imaging camera's own rotation SHALL
  take the direction for an instant between two remembered samples by a straight line
  between them, azimuth the short way round, and otherwise the direction now, as before
  (owner approval 5). `directions_at` SHALL not change.
- **FR-002**: The simulated mount SHALL take `set_count_steps`, false when built, under
  which each axis's tube follows its drive rounded down to a whole count. With it false
  nothing SHALL change.

**The tracking error** (US2):

- **FR-003**: `tracking_error` SHALL answer, for an imaging solve, a set point in that
  frame's pixels, a target's J2000 place and a time, the sky at the set point less the
  target: the solve's matrix applied to the set point less the target's pixel, as east
  and north arcseconds, with the time given; and `None` when the target has no pixel in
  that solve or a number is not finite.
- **FR-004**: `set_point_in_frame` SHALL place a set point given in sensor pixels, or
  the sensor's centre when none is given, in a frame's own pixels from its region and
  binning.
- **FR-005**: `crates/finder/src/imaging/steering.rs` SHALL define `Steered`, the one
  interface between the stack and a tracker: the target's place, `steer`, `gap`,
  `release`, `steered_by_subs`, and the two answers, `longest_exposure_s` for a turn of
  the field, with `longest_exposure_ahead_s` for the hour ahead (none by default), and
  `steady` with `steady_between` (true by default); with them `pause`, the tracker's own
  word for why it is not steady (none by default), and `tune`, which takes the two
  shares of FR-030 (nothing by default). The file SHALL hold none of `ephemeris`,
  `alt_az`, `parallactic`, `counts_per_revolution`, `AxisInfo`, `MountClient`,
  `azimuth`, `altitude`, `field_rotation`, `hour_angle`, `meridian`, `backlash` and
  `keyhole`: it computes no altitude, azimuth, parallactic angle or field rotation and
  reads no axis count, as the plan document's rule 1 asks of every file under
  `crates/finder/src/imaging/` (040's FR-023 holds 035's `stack/service.rs` to the same).
- **FR-006**: `GLOSSARY.md` SHALL define **Tracking error** and **Set point** (owner
  approval 4).

**The tracker** (US3):

- **FR-007**: A tracker SHALL be steered by the subs from a stack's first `gap` that
  comes while it is tracking and after its first rates (from 040's US6: while it is steady,
  040's FR-021). While it is, a live pointing from the finder camera SHALL change no rate
  and no tracking offset; it SHALL still be watched for the cable's wind, taken as the
  keyhole's resume pointing, kept among the 60 s of samples and given to the backlash
  measure. The live loop's syncs SHALL not change (owner approvals 1 and 2).
- **FR-008**: At a gap that follows a new tracking error, the tracker SHALL resolve it
  into its two axes as it resolves a finder error, add to each axis's error the steps
  its gaps made after the error's time, and for each axis whose sum is at least the
  larger of two of that axis's counts and 1″ on the sky, move that axis by the step
  share of the sum, −0.5 of it unless tuned (FR-030), at most 30″ on the sky, as one
  rate pulse of 0.5 s added to the axis's rate, never commanding a rate against the
  axis's direction: a step against an axis's motion is so at most that axis's rate times
  0.5 s. It SHALL take the step it made to be the pulse's rate over the time the pulse
  really lasted, from the acknowledgement
  of its start to that of its end.
- **FR-009**: At the same gap, for each axis, the tracker SHALL add to the tracking
  offset the rate share of the drift rate, −0.3 of it unless tuned (FR-030), and at most
  0.1″/s either way: this error less the last
  one applied, less the steps made between their two times, plus what every change of
  the tracking offset made after the earlier time would have moved the axis had it been
  in force from that time to the later one, all over the time between the two errors. It
  SHALL change no offset for the first error after a claim, nor when that drift rate is
  more than twice the offset's limit, nor when the steps made between the two errors'
  times add up to more than two of that axis's deadbands, and SHALL keep the offset
  within 015's limit. It SHALL remember what its last 16 gaps did.
- **FR-010**: At every gap served while the tracker is steered or its claim has lapsed,
  except the gap at which the claim lapses (FR-011), the tracker SHALL send each axis
  its rate and answer only when both are acknowledged; the gap at which the claim lapses
  sends nothing. Between a gap's answer and the next gap, while it stays steered or its
  claim stays lapsed, each once-a-second rate SHALL be the axis's feed-forward rate plus
  the tracking offset as that gap left it, or zero where 015 sends zero in place of a
  rate against the axis's direction; a take-up and the keyhole pass send what 015 has
  them send. Every rate SHALL be sent by the task that sends the once-a-second rates, a
  gap SHALL be served between two of its turns, and none SHALL be sent after tracking
  ends. A gap that comes before a tracker's first rates SHALL answer no step, send
  nothing and claim nothing.
- **FR-011**: The tracker SHALL stop being steered by the subs on `release`, at the
  eighth gap in a row with no new tracking error, and when tracking ends. It SHALL keep
  its tracking offset on a `release` and on such a lapse; the end of tracking and a
  keyhole pass zero it, as 015 has them. After a `release` or the end of tracking the
  finder camera's pointings correct it again as in 015. After such an eighth gap the
  claim has lapsed: the tracker SHALL stay so, whatever gaps follow, until a tracking
  error is handed over, and while it stays so a live pointing SHALL change no rate when
  it arrives and SHALL never change the tracking offset. The step that 015's
  proportional term would make for it, −0.6 of its axis error, SHALL be held instead, a
  newer pointing's step replacing an older one's, and the next gap SHALL apply it as one
  0.5 s pulse of at most 30″ on the sky an axis, added to the axis's rate, never
  commanding a rate against the axis's direction. The gap after a tracking error is
  handed over SHALL take the tracker back, forget what was held and treat that error as
  the first after a claim; `release` and the end of tracking SHALL also forget what was
  held. `follow`, as `release` does, ends the claim, clears a lapse and forgets what was
  pending or held. A tracking error SHALL be ignored when it is not finite, when it is
  handed over before a stack's first gap or after a `release`, when it is no later than
  the last applied, when its time is not later than the last `follow` or, from 040's US6, than
  the last time the tracker stopped being steady, and, from 040's US6, when it is handed over
  while the tracker is not steady; of two handed over between two gaps the later SHALL
  be used.
- **FR-012**: `gap` SHALL answer the step it made, as east and north arcseconds on the
  sky, and zero when it made none.

**The steering** (US4):

- **FR-013**: `Steering::before_exposure` SHALL ask its source for the running tracker
  and, with one, await its `gap` before answering; with none it SHALL answer at once.
  It SHALL answer the exposure to take and whether it is shorter than asked, and SHALL
  remember, for the last 32 exposures, when it answered and which tracker it found,
  without keeping that tracker alive.
- **FR-014**: `Steering::accepted` SHALL, for a sub whose mid-exposure falls in an
  exposure it remembers, whose tracker still exists and has a target, hand that tracker
  the tracking error at the sub's mid-exposure from the set point; and hand nothing
  otherwise. When the first tracking error it works out after a `release`, or the first
  of all, is longer than 180″, it SHALL hand over nothing, release the tracker and,
  until its own next `release`, call neither `gap` nor `steer`: the steering does not
  steer a stack that began far from its set point (owner decision 7). The set point
  SHALL be the sensor's centre unless `set_set_point` has named another, and
  `set_set_point(None)` SHALL put it back there. A `set_set_point` that changes the set
  point SHALL let go a tracker a stack holds, as the switch turned off does (FR-019), so
  that the first tracking error from the new place is a first error.
- **FR-015**: `Steering::release`, and the drop of the `Hold` a stack keeps, SHALL
  release the tracker and forget the exposures remembered.
- **FR-016**: The imaging view, and the session's `imaging` object, SHALL carry
  `steering` from the first use of the steering in a run of the console, and before it
  only as FR-018 has it while a tracker runs: `source` (`subs`, `tracker` or `none`),
  `subs` and `last`, `holding`, true while a stack keeps a `Hold`, and `far_arcsec`, the
  length of that first tracking error, while the steering stands back (FR-014; owner
  approval 3). `Session::steered` SHALL answer the running tracker, and
  `Imaging::steering` the console's steering.

**The exposure cap** (US5):

- **FR-017**: The tracker's `longest_exposure_s` SHALL answer the turn it is asked about
  divided by the field rotation at the target now, and none when that rotation is under
  10⁻⁹ rad/s, in agreement with 015's `longest_exposure`. Its `longest_exposure_ahead_s`
  SHALL answer the shortest of those answers for now and for each whole minute of the
  hour ahead, as 015 samples its forecasts, and none when each of them is none.
- **FR-018**: `before_exposure` SHALL ask the tracker about a turn of the smear setting
  divided by the distance, in delivered pixels, from the set point to the farthest
  corner of the frame to be taken. The first call after a `release`, or the first of
  all, SHALL answer the wanted exposure halved until it is no longer than the tracker's
  answer for the hour ahead (FR-017), or until one more halving would pass 1 s or the
  camera's shortest, whichever is longer, with `limited` true when that is shorter than
  the exposure it was asked for; every later call SHALL answer that same exposure and
  that same `limited`, whatever exposure it is asked for, until the next `release`. The
  view SHALL carry `smear_px` and `on`; `exposure_s` and `limited`, the exposure held
  and whether it was cut, absent and false while none is held; `longest_exposure_s`, a
  number or `no limit`, the tracker's answer for now, asked as the view is made for the
  frame the stack takes or, with no stack, the frame the console would take, and absent
  while no tracker runs; `over_limit`, true while that answer is under the exposure
  held; and `would_take_s`, the exposure a stack started now would hold, wherever a
  tracker runs and the console's camera and settings are known. Where they are, the view
  SHALL be carried while a tracker runs even before the steering's first use (FR-016).
- **FR-019**: `PUT /api/imaging/steering` SHALL accept, under the same-origin rule, a
  JSON object with any of `on` (a boolean), `smear_px` (a number from 0.1 to 20),
  `step_share` and `rate_share` (each a number from 0 to 1), or with none of them, and
  answer 200 with the imaging view; 400 for any other JSON body and for a body that is
  not JSON, with `the corner smear must be a number from 0.1 to 20`,
  `the step share must be a number from 0 to 1` or
  `the rate share must be a number from 0 to 1` for a number out of range; 415 for a
  `Content-Type` other than `application/json`; 403 for a foreign origin; and 409 with
  `this console takes no imaging frames` without imaging. The four settings are
  run-only: 6.0, true, 0.5 and 0.3 at every start. With `on` false the steering SHALL
  call neither `gap` nor `steer`, and SHALL still cap and, from 040's US6, still pause and
  drop. Turning `on` to false SHALL let go at once a tracker a stack holds: the tracker
  is released and the count of tracking errors handed over begins again, while the
  exposure held and the exposures remembered are kept; turned on again, the stack's next
  gap claims it (owner approval 3).
- **FR-020**: `crates/finder/README.md` SHALL document the route in its `## API` table,
  and `crates/finder/src/lib.rs` SHALL list it (owner approval 8).

**Tuning** (US5):

- **FR-030**: The steering SHALL hold a step share and a rate share, each a number from
  0 to 1, 0.5 and 0.3 when built, and SHALL hand both to the tracker through `tune`
  before every `gap` it asks for. The tracker SHALL use them from its next gap on in
  place of FR-008's 0.5 and FR-009's 0.3, each held to 0 to 1 and a number that is not
  finite ignored; a tracker nobody has tuned SHALL use 0.5 and 0.3. The view SHALL carry
  `step_share` and `rate_share` (owner decision 3).

### Key Entities

- **Tracking error**: how far the sky at the set point is from the target, east and
  north in arcseconds, at a stated time.
- **Set point**: where on the imaging camera's sensor the run wants the target; the
  centre unless named.
- **Gap**: the time between two exposures of a stack, the only time a correction is
  made.
- **Step**: what a gap moves the telescope by, on the sky.
- **Drift**: how far the target moved on the sensor during one sub's exposure, a
  distance in arcseconds or pixels; what the readout shows.
- **Drift rate**: how fast an axis's tracking error grows between two subs once the
  steps made between them are taken out, in arcseconds a second; what the tracker
  takes 0.3 of out of that axis's rate.
- **Steering**: the console's part between the stack and the tracker; it holds the set
  point, the four settings, the exposure a stack holds and the readout.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario has a committed test that asserts it, and the
  `lint` and `test` gates pass.
- **SC-002**: No existing test, golden, `scripts/ci/` file, `.github/` file,
  `CODEOWNERS` entry, `ergane.yaml` or `playwright.config.ts` is edited. The only edit
  to an existing file under `crates/finder/tests` is one `mod` line for each new
  module, in alphabetical order, in `crates/finder/tests/console/main.rs` and
  `crates/finder/tests/motion/main.rs` (owner approval 9).
- **SC-003**: On a simulated mount that runs 1% fast, with tracking errors arriving a
  sub late, the drift within a 5 s sub is at most 0.15″ once the subs steer (from the
  12th sub in US3-S1 and the 14th in US4-S5; 040's US7-S1 holds a stack to the same),
  against 0.75″ without.
- **SC-004**: Every test of 015 passes unchanged, and
  `crates/finder/src/imaging/steering.rs` holds no alt-az arithmetic (US2-S6).
- **SC-005**: While a stack runs with the switch on (owner decision 8), the steering not
  standing back (FR-014) and the tracker steady (040's US6-S5), no rate is sent during an
  exposure the stack keeps other than each axis's feed-forward rate plus the tracking
  offset the last gap left, whether the subs steer or have lapsed (US3-S7, US3-S11,
  US9-S11, US4-S5).

## Assumptions

- **Earlier features are in place.** 015, 030, 031, 032, 033, 034, 035 and 038 have
  landed (flip.md § Prerequisites 1), 031 with the camera's lease of the plan document's
  amendment 1 as its US9. This trio does not reach the lease: 040's stories do. It was
  re-anchored to their code on 2026-11-20 (flip.md § Prerequisites 2 and 3): every
  `path:line` here was read by its content on `main` at 52785403, whose line numbers
  are cited, the lines of `GLOSSARY.md` among them; those the drafts held were first
  read at 49bb210, the drafting worktree's HEAD.
- **Out of scope:**
  - an equatorial mount, a tracker that corrects in small steps, meridian flips,
    dithering and guiding with a second camera (the plan document, § Deferred);
  - moving the set point: this spec keeps one and lets 037 or a later spec name it;
  - telling a wrong imaging solve from a right one, and star-shape gates (035);
  - changing a stack's exposure while it runs, and weighting subs by exposure (035);
  - retuning 015's gains for the finder camera, and any change to GoTo;
  - persisting the four settings (owner decision 3);
  - what `040-stack-waits-for-steady` builds on this spec: the pause while the tracker is
    not steady, the stack's calls of the steering, the readout and `set_steering`.
- **Outside the factory:** GoTo and Track proven on the real mount (the plan document,
  § Before any flip, 4), and its backlash measured on both axes (azimuth reverses for a
  target that culminates north of the zenith), before this spec is used at the
  telescope; the field checks flip.md lists; and rebuilding the console on the console
  machine.
- **The factory's gates and CI:** the gates are `lint` and `test`, copied in plan.md
  § Verification. CI's other jobs run only in the merge queue, so no scenario's Then
  clause rests on them.

## Work Graph

```yaml
US1:
  depends_on: []
  implements: [FR-001, FR-002]
US2:
  depends_on: []
  implements: [FR-003, FR-004, FR-005, FR-006]
US3:
  depends_on: []
  depends_on_merged: [US2]
  implements: [FR-007, FR-008, FR-009, FR-010, FR-011, FR-012]
US4:
  depends_on: []
  depends_on_merged: [US1, US2, US3, US9]
  implements: [FR-013, FR-014, FR-015, FR-016]
US5:
  depends_on: []
  depends_on_merged: [US3, US4]
  implements: [FR-017, FR-018, FR-019, FR-020, FR-030]
US9:
  depends_on: []
  depends_on_merged: [US3]
  implements: []
US12:
  depends_on: []
  depends_on_merged: [US5]
  implements: []
```

Every edge is a merge edge, because each dependent builds on what its predecessor
creates. US3 implements US2's `Steered` for the tracker. US9 tests US3's tracker: it
writes no code of its own and implements no FR, and it follows US3 on the chain because
both add a `mod` line to `crates/finder/tests/motion/main.rs`. US4 creates `Steering` in
US2's file over the tracker US3 made steerable, and its US4-S5 reads US1's memory; it
names US9 as well as US3, as the next story after US9 to write that `main.rs`. US5 adds
to US4's `steering.rs` and `imaging/mod.rs` and to US3's `tracking.rs`. US12 tests US5's
tracker: it writes no code of its own and implements no FR, and it follows US5, whose
code it tests; its `mod` line in `crates/finder/tests/motion/main.rs` comes after US4's,
which US5's edge to US4 orders. Where a story names two predecessors and one already
follows the other, the second edge is there because both wrote a file the story writes.
US1 touches no file in common with US2 or US3 (`crates/simulator` against
`crates/finder` and `GLOSSARY.md`) and may run beside them. The stories that follow US5
and US12 are `040-stack-waits-for-steady`'s, which lists this spec in its
`depends_on_landed`.
