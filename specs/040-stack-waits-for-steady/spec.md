---
state: ready
depends_on_landed: ["015-tracking", "030-imaging-camera-link", "031-imaging-frames-on-console", "032-imaging-solve-and-aim-point", "033-focus-reading-and-aid", "034-dark-and-flat-frames", "035-live-stack", "038-track-follows-a-push", "036-tracking-from-subs"]
# The second of two pieces of tracking steered by the subs: the stack waits while the
# tracker is not steady and drops what was taken across it (US6, and US10, which tests
# US6's tracker), asks the steering before every exposure (US7), shows the readout (US8)
# and takes set_steering over MCP (US11). Story and FR numbers are 036's, kept.
# Depends on 036-tracking-from-subs (US1 to US5, US9 and US12) and on what 036 depends on.
# State: ready, flipped on 2026-11-20 under the owner's standing permission for the
# imaging specs (flip.md). history.md has how it got here.
# Re-anchored on 2026-11-20 to `main` at `fc161a54`, after 036 landed (#327 to #333), and
# probe round 2's findings folded: 3 blocking and 20 clarity, US6 cut by tests into US10.
---

# Feature Specification: The Stack Waits for a Steady Tracker, and Shows the Steering

**Feature Branch**: `040-stack-waits-for-steady`

**Created**: 2026-11-20, from `036-tracking-from-subs` (created 2026-11-15)

**Status**: Ready

**Input**: Spec 036 lets the subs of a live stack steer the tracker through `Steering`,
the console's part between the stack and the tracker: the tracker's rates change only in
the gaps between exposures, and a stack's exposure is capped at the tracker's limit for a
chosen smear. This spec finishes that work: pause exposures while the tracker is not
steady and drop a sub whose exposure crossed such a time, have 035's stack ask the
steering before every exposure and hand over every sub it accepts, show what steers, the
tracking error and the drift on the Imaging page and in the session, and let an MCP
client set what the page sets. 015 lists "guiding from the imaging camera" as out of
scope; 036 and this spec are that work.

**This piece.** This spec is the second of two, and lists `036-tracking-from-subs` in its
`depends_on_landed`. Its stories keep 036's numbers: US6 (not steady: pause and drop),
US10 (US6's tracker at its edges, tests only), US7 (the stack asks before every
exposure), US8 (the readout) and US11 (`set_steering`). What it uses of 036 is listed in
plan.md § What 036 provides. A cite such as "036's US4-S12" is to that spec.

**Owner decisions and owner approvals** are cited here by their numbers in flip.md, and
for those 036's flip approves, in 036's flip.md; the rule each one settles is stated in
the requirement, scenario or interface that cites it.

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
  `Simulator` as a field only where it reads it (036's US4-S5's does; 036's US3's does not, and a
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
  are sent by its task's first turn, not by `start` (036's rate task, as landed:
  `crates/finder/src/tracking.rs:1367`), and a gap asked
  for before them moves and claims nothing. The 3.5 s are for US6, under which a tracker
  settles for 2 s after its first rates and turns a gap away until then (FR-021): 036's US3's
  scenarios must pass before US6 and after it. Every scenario on the tracker rig begins
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
- **Playing a sub late** is playing a sub with step 6 changed: for the first sub the
  test hands over nothing; for every later sub it hands over the em and tm it kept
  from the sub before it. So each tracking error reaches the tracker one sub after its
  own, as 035's worker delivers them (plan.md § The stack).
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

### User Story 6 - Not steady: pause, and drop what was taken across it (Priority: P2)

As the stack, I start no exposure while the tracker says it is not steady: during a
backlash take-up, a first reversal that is still being measured, or the keyhole pass;
while a hand is moving the telescope; for the 2 s the tube is given to settle after a
take-up, the keyhole pass and a tracker's first rates; and in a turn that finds an axis
stopped. A sub whose exposure crossed such a time is dropped before it is solved, with
that reason. While it is not steady the tracker corrects itself as 015 has it, even
after the subs have lapsed, so that a first reversal's measurement still closes. I also
start none while the tracker's limit is under half the exposure I hold: I let the
tracker go and wait for the limit to rise.

The fourteen scenarios of User Story 10, which follows this story on the chain (Work
Graph), test this story's code; five of them were cut from this story by tests (below its
scenarios).

**Why this priority**: Altitude reverses at transit, where a target is highest and a
run will sit. It is P2 because it is rare within a night.

**Independent Test**: `crates/finder/tests/motion/steering_steady.rs`, the module
`steering_steady` of `motion`, on the backlash rig; and
`crates/finder/tests/console/steering_steady.rs`, the
module `steering_steady` of `console`, with the scripted tracker. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs both; during
development, `cargo nextest run -p finder --locked --test motion steering_steady::` and
`cargo nextest run -p finder --locked --test console steering_steady::`.

**Acceptance Scenarios**:

1. **Given** the backlash rig, followed to 60 s before the transit with
   `rig.follow(transit - 60.0, 60.0)`, and one `gap` awaited then, **When** a test runs
   three futures to their ends with `tokio::join!` on its own task, each borrowing the
   rig (the rig holds its `Tracker` by value, so nothing can be spawned over it): the
   follower, `rig.follow(transit + 120.0, 60.0)`; the reader, which every 50 ms until
   the rig's clock passes transit + 120 s notes the console time,
   `rig.tracker.steady()`, `rig.tracker.pause()` and `rig.tracker.steered_by_subs()`;
   and the asker, which looks every 10 ms, until the rig's clock passes transit + 120 s,
   for the reader's first false reading of `steady()`, then awaits
   `rig.tracker.gap()` once and notes the console time of its answer, **Then** a
   committed test asserts, with F and L the times of the reader's first and last
   false readings, that:
   - `steady()` read false at least once, every reading from F to L is false with
     `pause()` `Some("reversing")`, and the reading 1 s before F has `pause()` `None`;
   - the altitude rate of magnitude 0.5°/s in the rate log, and the zero rate that ends
     it, both lie between F − 0.05 s and L + 0.05 s (oracle: the rig's rate log, as
     `crates/finder/tests/support/tracker.rs:285` reads it);
   - `steady_between(F - 20.0, F - 1.0)` and `steady_between(L + 1.0, L + 20.0)` are
     true, and `steady_between(F - 5.0, F + 0.1)`, `steady_between(L - 0.1, L + 5.0)`
     and `steady_between(F - 5.0, L + 5.0)` are false, all asked once the three
     futures have ended;
   - the reader's `steered_by_subs()` is true at its reading 1 s before F and false at
     L;
   - the asker's `gap` answers exactly (0.0, 0.0), no earlier than the zero rate that
     ends the take-up and at least 1.5 s before L: a gap asked for during a take-up is
     served once the take-up's turn is over, never inside it, and is turned away then,
     because the tracker settles for 2 s more (oracle: the rig's rate log and clock.
     What the mount is left running at the first gap after the settling is US10-S1).
2. **Given** `crate::support::tracker::Rig` for M42 from 120 s before its transit on a
   fresh session, so that no backlash is measured, **When** a test joins
   `rig.follow(transit + 120.0, 60.0)` with a reader as in US6-S1, **Then** a committed
   test asserts, with Z the time of the altitude axis's first zero rate in the rate
   log, that every reading of `steady()` from 3.5 s after the rig's start to Z − 0.1 s
   is true, that the reading nearest Z + 5 s is false, with `pause()`
   `Some("measuring")`, and that `steady_between(Z + 0.1, Z + 10.0)`, asked at the end,
   is false (oracle: the rig's rate log and clock. The measurement's close, a start and
   a stop, what a take-up forgets, the trait and the keyhole pass are US10-S2 to
   US10-S7).
3. **Given** a scripted steering whose tracker says it is not steady, with the `pause`
   word `reversing`, and whose `gap` waits 0.1 s, **When** a test awaits
   `before_exposure` with the worked ask in a task,
   waits 10.1 s, sets the tracker steady and waits 1 s, **Then** a committed test
   asserts that the call had not answered after the 10.1 s, with `view()`'s `steady`
   false, `waiting` true, `pause` `reversing`, `Steering::waiting()` true and no `gap`
   in the log; and that it answered within 0.5 s of the change, after exactly one `gap`,
   with `steady` true, `waiting` false, no `pause` and `Steering::waiting()` false (the
   five later cases this scenario held are US6-S12 and US6-S6 to US6-S9).
4. **Given** a scripted steering after a `before_exposure` with the worked ask, whose
   tracker answers `steady_between` false, **Then** a committed test asserts that
   `check` with the
   worked frame (`start` 1768528800.0, `exposure_s` 5.0) answers
   `Verdict::Drop("not_steady")`, and that `view()`'s `dropped_not_steady` is 1, and, on
   another such steering after the same two calls, that after `release` it is still 1
   and after a new `Steering::hold` it is 0: the count is one stack's, and lasts until
   the next stack begins; that with `steady_between` true it answers `Verdict::Keep` and
   the count stays 1; that the tracker's log shows the window it was asked about as
   (1768528800.0, 1768528805.0); that after the test has made the source answer no
   tracker and dropped its last `Arc` of the scripted one, `check` with the same frame
   answers `Drop("not_steady")`: a tracker that has gone was not steady; and that on a
   `Steering` whose source answers no tracker, `check` after a `before_exposure` answers
   `Keep`; and, on another scripted steering as in the first case, that `check` with the
   worked frame started 0.01 s earlier (`start` 1768528799.99) also answers
   `Drop("not_steady")`: the record is found by the frame's mid-exposure, not its start;
   and, on another such steering after `set_on(false)` and a `before_exposure` with the
   worked ask, that `check` with the worked frame answers `Drop("not_steady")` too: the
   switch turns off the steering, not the drop.
5. **Given** `crate::support::tracker::Rig` for M42 from 600 s before its transit on a
   fresh session, as `crates/finder/tests/support/tracker.rs:326` builds one, so that
   no backlash is measured; a first `gap` awaited once `steady()` reads true (read every
   10 ms, for at most 5 s: the tracker settles for 2 s after its first rates); and eight
   more gaps, each after 6 s of waiting and with no `steer`, so that the claim has
   lapsed, **When** a test runs with `tokio::join!` on its own task the follower,
   `rig.follow(transit + 600.0, 60.0)`, and an asker that, until the rig's clock
   passes transit + 600 s, waits 6 s, awaits `rig.tracker.gap()` and notes
   `rig.tracker.steady()` and `rig.tracker.steered_by_subs()` just after its answer,
   **Then** a committed test asserts that the follower's own bound held (it asserts
   the true error is at most 60″ at every step, `crates/finder/tests/support/tracker.rs:269`);
   that `GET /api/session`'s `backlash.alt_deg` is within 30″ of 0.3°, as
   `crates/finder/tests/support/tracker.rs:332` asserts after the same follow with no
   stack; that at least one of the asker's notes of `steady()` is false and the last
   is true; that every note of `steered_by_subs()` is false; and that `steady()` is
   true once the two futures have ended (oracle: the mount's configured 0.3° of play
   and 015's own bounds. While the first reversal's measurement is open the tracker is
   not steady, its gaps are turned away, and the finder camera's pointings correct it
   as 015 has them though the claim had lapsed; held for a gap instead, they would
   never be applied, and the measurement would not close).
6. **Given** a scripted steering whose tracker is steady, before any `before_exposure`,
   given only `set_smear_px(6.0)` so that it has a view, and after one; and a
   `Steering` whose source answers no tracker, after one, **Then** a committed
   test asserts that each `view()` has `steady` true, `waiting` false, no `pause` and
   `dropped_not_steady` 0.
7. **Given** a scripted steering whose tracker is not steady, a `Hold` taken from it
   and a `before_exposure` awaited in a task for 2 s, **When** the test aborts that
   task and drops the `Hold`, **Then** a committed test asserts that
   `Steering::waiting()` is false and that `view()` has `waiting` false, `steady` true
   and no `pause`: a wait abandoned with its stack does not go on showing.
8. **Given** a scripted steering whose tracker is not steady, after `set_on(false)`,
   **When** a test awaits `before_exposure` with the worked ask in a task, **Then** a
   committed test asserts that it has not answered 1 s after it was called, with
   `steady` false and `waiting` true, and that once the tracker is steady it answers
   with no `gap` in the log: the switch turns off the steering, not the wait. **And**
   on another such steering with the switch still on, where `set_on(false)` is called
   0.5 s into the wait, the same holds: the switch is read where a gap would be asked
   for, not when the call is made.
9. **Given** a scripted steering whose tracker is not steady and a `before_exposure`
   waiting in a task for 1.1 s, **When** the test makes the source answer no tracker,
   **Then** a committed test asserts that the call answers within 0.25 s, and that
   `view()` has `steady` true and `waiting` false.
10. **Given** a scripted steering whose tracker answers a longest exposure of 37.15 s,
    after a first `before_exposure` with the worked ask and a wanted 60 s that answered
    30.0, **When** the tracker's longest exposure becomes 14.0 s and a test awaits
    `before_exposure` again in a task, waits 1.1 s, sets the longest exposure to 15.0 s
    and waits 0.5 s, **Then** a committed test asserts that the call had not answered
    after the 1.1 s, with `view()`'s `waiting` true, `steady` true, `pause` `limit` and
    `over_limit` true, `Steering::waiting()` true, and the tracker's log holding one
    more `release` and no second `gap`; and that it answered within 0.5 s of the change,
    30.0 with `limited` true, after a second `gap`, with `waiting` false and no `pause`
    (oracle: FR-022. While the limit is under half the exposure a stack holds, its subs
    would be smeared by more than twice the smear chosen, so it takes none, lets the
    tracker go to its own corrections and goes on by itself once the limit has risen:
    15.0 s is half of 30.0 s and is not under it). **And** on another such steering
    whose tracker answers a longest exposure of 0.4 s from the start, a first
    `before_exposure` with the worked ask and a wanted 5 s has not answered after 1 s,
    with `pause` `limit` and one `release` in the log: the cap's 1 s floor is more than
    twice the limit, and the first call looks at the limit again once the cap has
    decided.
11. **Given** a scripted steering whose tracker is steady and whose `gap` waits 5 s,
    **When** a test awaits `before_exposure` with the worked ask in a task, sets the
    tracker not steady, with the `pause` word `overhead`, 0.5 s later and waits 0.6 s
    more, **Then** a committed test asserts that `view()` has `steady` false, `waiting`
    true and `pause` `overhead`, the tracker's word, that
    `Steering::waiting()` is true, and that the log holds exactly one `gap`: a tracker
    that stops being steady while its gap is awaited, as one does whose keyhole pass
    begins just then, shows as a wait.
12. **Given** a scripted steering whose tracker's `gap` waits 1 s, **When** a test
    awaits `before_exposure` with the worked ask in a task, sets the tracker not steady
    0.5 s later and steady again 5 s after that, **Then** a committed test asserts that
    the call has not answered 4 s after it was made, and that it answers between 6.4 s
    and 6.8 s after it was made, with exactly two `gap`s in the log: the first gap ended
    on a tracker that was not steady, so it does not count.

Its five motion scenarios that were US6-S6 to US6-S10 (the push by hand on the goto rig,
the stall, a quiet pointing before a lapsed gap, errors from before and inside a time not
steady, and a gap queued behind a take-up) are US10-S10 to US10-S14: their tests are
committed in User Story 10's file, and the judge reads each story's scenarios against
its own diff. US6-S13 to US6-S17 took the numbers 6 to 10.

---

### User Story 7 - The stack asks before every exposure (Priority: P2)

As the observer with Track on and a stack running, I get subs whose stars are round:
the stack's taker asks the steering before every exposure and uses the exposure it
allows, its worker drops a sub the tracker was not steady for and hands over every sub
it accepts, and the tracker is let go when the stack stops.

**Why this priority**: It is what makes the earlier stories reach a picture. It is P2
because it consumes them, and it is the one story that edits 035's stack.

**Independent Test**: `crates/finder/tests/motion/steering_stack.rs`, the module
`steering_stack` of `motion`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test motion steering_stack::`.

**The stacking rig** is 032's solving rig with these changes: the mount is
`SimMount::with_config` with `MountConfig { base_tilt_deg: 0.0, base_tilt_toward_deg:
0.0, zero_points_deg: [120.0, 50.0], backlash_deg: 0.0 }`, the azimuth and altitude of
the rig's `push_to(50.0, 120.0)`, connected before that push; the session has the
target M42 and the aim point (700.0, 420.0), the true aim point; the telescope is
pointed with `sim.point_eyepiece_at` at M42's catalogue place; the test waits until the
pointing model has a sync and the session's pointing is within 1′ of M42, as
`crates/finder/tests/motion/tracking/routes.rs:96` does, and turns Track on with
`POST /api/track`; then it waits until the session's `track.state` is `tracking` and the
imaging view's `state` is `ready`. (The tracker settles for 2 s after its first rates,
the stack's first `before_exposure` waits that out, and a sub that starts at the
instant the tracker became steady is kept: plan.md § Steady, the tracker's second
answer.) **Waiting until** a
condition means reading `GET /api/imaging`, or `GET /api/session` for the session's
parts, every 100 ms until it holds, for at most 300 s of virtual time, and failing
otherwise. **A saved sub** is one whose FITS file is in the stack's folder
`runs/<id>/subs/`, `<id>` being the `stack.id` that `POST /api/imaging/stack` answers;
subs are numbered in the order of their file names, which are their imaging frame ids.
**A sub's drift** is the separation between `sim.imaging_truth_at` at the sub's start
and at its start plus its exposure, both read from that file: its `DATE-OBS`, UTC to the
millisecond, and its `EXPTIME`, in seconds. (031's `frame` part is not used: the taker
takes the next sub at the instant one is saved, so a read every 100 ms sees few of them
`saved`.) **The stack's counts** are the `stack` part of the imaging view (035); **Σ
dropped** is the sum of the twelve counts of its `dropped`.

**Acceptance Scenarios**:

1. **Given** the stacking rig with `set_rate_error(0.01)` on its mount and
   `PUT /api/imaging/settings` with `{"exposure_ms":5000}`, **When** a test starts a
   stack with `POST /api/imaging/stack` and waits until `steering.subs` is at least 18,
   **Then** a committed test asserts that `steering.source` is `subs`; that every
   sub's drift from the 14th sub on is at most 0.15″; that `stack.dropped.not_steady`
   is 0; and that after `DELETE /api/imaging/stack`, `steering.source` is `tracker` and
   `steering.subs` 0 (oracle: the simulator's truth. The same rig with the switch off is
   US7-S8).
2. **Given** the stacking rig, `PUT /api/imaging/steering` with `{"smear_px":2}` and
   `PUT /api/imaging/settings` with `{"exposure_ms":60000}`, **When** a test reads
   `GET /api/imaging`, starts a stack and waits until its first sub is saved, **Then** a
   committed test asserts that the read before the start has `steering.would_take_s`
   30.0 and no `steering.exposure_s`; that the sub's `EXPTIME` is 30.0; that
   `settings.exposure_ms` is 30000.0 while the stack runs; that `steering.limited` is
   true, `steering.exposure_s` 30.0, `steering.over_limit` false and
   `steering.longest_exposure_s` within 2% of the figure the test computes as 036's US12-S1
   does for the console time of its read; and that `settings.exposure_ms` is 60000.0
   once `DELETE /api/imaging/stack` has answered (the stack set the exposure through its
   own lease and put the observer's back through it before letting the camera go. Worked
   example: at the default smear of 6 px the rig's limit is about 112 s and a setting of
   60 s is not cut, so the scenario sets 2 px; the limit is then 37 s now and 34.3 s at
   its shortest in the hour ahead, at M42's transit, and 60 s is halved once).
3. **Given** the stacking rig with `{"exposure_ms":5000}`, a stack running and
   `stack.stacked` at least 2, **When** a test waits until an imaging frame is
   `exposing`, sends `POST /api/track` with `{"on":false}` and then `{"on":true}`,
   waits until `stack.taken` has risen by 3 more and stops the stack, **Then** a
   committed test asserts `stack.dropped.not_steady` is at least 1,
   `steering.dropped_not_steady` equals it, and `stack.taken` equals `stack.stacked`
   plus Σ dropped; and that `stack.json`'s `subs` lists as many subs with `outcome`
   `not_steady` as `stack.dropped.not_steady`, and that
   `Dropped { not_steady: 2, ..Default::default() }.total()` is 2; and that at no read
   is `stack.last.outcome` `not_steady`, and that no sub `stack.json` lists as
   `not_steady` has its `subs/<frame_id, six digits>.solve.json`: the check comes before
   the solve and before `last`.
4. **Given** 032's solving rig as it stands, with no target and Track off, and
   `{"exposure_ms":2000}`, **When** a test starts a stack, waits until `stack.taken` is
   at least 3 and stops it, **Then** a committed test asserts that `steering.source` is
   `none` and `steering.subs` 0 throughout, that every saved sub's `EXPTIME` is 2.0, and
   that `stack.dropped.not_steady` is 0: with no tracker the stack is
   neither capped nor paused, and nothing is dropped for the tracker's sake.
5. **Given** 032's solving rig as it stands, `{"exposure_ms":60000}`, and the console's
   steering given a source of the test's own with
   `state.imaging().unwrap().steering().set_source(…)`: a scripted tracker whose target
   is the worked target, which names a longest exposure of 37.15 s and whose `gap`
   waits 10 s, **When** a test starts a stack, calls the simulator's
   `set_unresponsive(true)` (`crates/simulator/src/controls.rs:117`) 1 s later, while
   the stack's first `before_exposure` waits in that gap, and waits until
   `stack.state` is `ended`, **Then** a committed test asserts that
   `stack.ended_reason` is `camera`; that `stack.error` is exactly
   `the exposure could not be set to 30000 ms: the scope node is not connected`; that
   `stack.taken` is 0; that the scripted tracker's log holds exactly one `gap` and,
   after it, one `release`; and that, after `set_unresponsive(false)` and a wait until
   the imaging view's `state` is `ready`, `settings.exposure_ms` is 60000.0 (oracle:
   031's own words for a camera that is not ready, which the stack's lease passes on;
   the link is lost within 3 s by 031 US1-S6, well inside the gap's 10 s. A stack that
   cannot shorten its exposure takes no sub at the observer's. A stop while the first
   gap waits, and a link lost while the second waits, are US7-S9 and US7-S10).
6. **Given** the text of `crates/finder/src/imaging/stack/service.rs`, 035's file as
   this story leaves it, read by the test from `CARGO_MANIFEST_DIR`, **Then** a
   committed test asserts it holds none of `ephemeris`, `alt_az`, `parallactic`,
   `counts_per_revolution`, `AxisInfo`, `MountClient`, `azimuth`, `altitude`,
   `field_rotation`, `hour_angle`, `meridian`, `backlash` and `keyhole` (oracle: the
   plan document's rule 1, read as 036's US2-S6 reads it of `steering.rs`: the stack reaches
   the tracker only through `Steering`, and the calls this story adds to it say nothing
   of axes).
7. **Given** the stacking rig with `{"exposure_ms":5000}` and one change, made right
   after its `sim.point_eyepiece_at`: the test reads `c = sim.telescope().camera`, calls
   `sim.set_optics` with `Optics { eyepiece_px: (716.0, 420.0), ..Optics::default() }`,
   and calls `sim.push_to(c.alt, c.az)`, so that the finder camera still has M42 at the
   session's aim point, (700.0, 420.0), while the eyepiece and the imaging camera look
   16 finder pixels from it, about 8′ (the simulated finder camera's 12° across 1456 px
   are 29.8″ a pixel): the session's aim point is that far from the true one, **When** a
   test starts a stack, reads `GET /api/session`, `GET /api/imaging` and
   `sim.telescope().eyepiece` every 100 ms until `stack.taken` is at least 12, and stops
   the stack, **Then** a committed test asserts that at the last read before the stop
   `steering.far_arcsec` is between 420 and 540, `steering.source` is `tracker` and
   `steering.subs` is 0; that no read of the session has a `track.push`; and that the RA
   and Dec of every reading of the eyepiece are within 60″ of the first (oracle: the
   simulator's truth and 038's own `track.push`. Worked example: 16 px × 29.8″ is 477″,
   over the 180″ beyond which the steering stands back. Steered, the subs would have
   stepped the telescope 30″ a gap toward centring M42, and once the finder camera's
   pointing was 5′ from the target 038 would have read a push nobody made).
8. **Given** the stacking rig with `set_rate_error(0.01)` on its mount and
   `PUT /api/imaging/settings` with `{"exposure_ms":5000}`, **When** a test calls
   `sim.stop_frames()` (`crates/simulator/src/controls.rs:108`) as soon as the session
   reports `tracking`, so that no finder pointing reaches the tracker and its own
   corrections change no rate, sends `PUT /api/imaging/steering` with `{"on":false}`,
   starts a stack and waits until 12 subs are saved, **Then** a committed test asserts
   that `steering.subs` stays 0, that `steering.source` is `tracker`, and that at least
   8 of the 12 subs drift more than 0.5″ (oracle: the simulator's truth; worked example:
   with nothing correcting it, a mount 1% fast drifts 0.749″ in a 5 s sub. The finder
   camera's frames are stopped because 015's own loop would otherwise start to correct
   once the error passed its 10″ deadband, about 67 s after Track began, and the bound
   would then rest on how long the rig's waits took).
9. **Given** 032's solving rig as it stands, `{"exposure_ms":60000}`, and the console's
   steering given US7-S5's scripted tracker as its source, **When** a test starts a
   stack and sends `DELETE /api/imaging/stack` 1 s into the first gap, **Then** a
   committed test asserts that the `DELETE` answers within 0.5 s of virtual time, that
   the stack ends `stopped` with `taken` 0, and that the tracker's log holds exactly
   `gap` then `release` (oracle: 035's own stop; the stopping mark ends the steering's
   wait).
10. **Given** the rig, the setting and the scripted tracker of US7-S9, **When** a test
    starts a stack, calls the simulator's `set_unresponsive(true)` once `stack.taken` is
    1, while the second gap waits, and waits until `stack.state` is `ended`, **Then** a
    committed test asserts that `stack.ended_reason` is `camera`, `stack.taken` 1 and
    `stack.error` exactly `the scope node is not connected`, the take's own refusal: the
    exposure is set once, before the first sub; and that, after
    `set_unresponsive(false)` and a wait until the imaging view's `state` is `ready`,
    `settings.exposure_ms` is 30000.0: the camera refused the stack's change back to the
    observer's 60 s, and a refusal there changes nothing else (oracle: FR-023, and 031's
    own words for a camera that is not ready).

---

### User Story 8 - The readout on the Imaging page and in the session (Priority: P3)

As the observer, I see on the Imaging page what steers the tracker, how large the
tracking error is, how far the target drifts during a sub, in arcseconds and in pixels,
whether my exposure was cut, and whether the stack is waiting for the tracker. I set the
corner smear, the switch and the two tuning shares there.

**Why this priority**: It is how the observer knows the feedback works, and what the
field check reads. It is P3 because the feedback works without it.

**Independent Test**: `crates/finder/tests/console/steering_readout.rs`, the module
`steering_readout` of `console`, in the `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` (during
development, `cargo nextest run -p finder --locked --test console steering_readout::`);
and `npm test` in `crates/finder/web`, which runs
`src/components/SteeringBlock.test.tsx`.

**A set clock** is a `Clock` of the test module's own whose time the test sets. **A
solve for the tracking error (e, n)** is `ImagingSolve::from_cd` with the worked
solve's matrix and size and the centre RA 83.81867° + e/(3600 × cos 5.38967°), Dec
−5.38967° + n/3600, so that with the worked target and the centre as set point the
tracking error is (e, n) within 10⁻⁴″.

**Acceptance Scenarios**:

1. **Given** a scripted steering on a set clock, whose tracker's target is the worked
   target and answers `Stepped` (0, 0) at its first two gaps and (−0.5, 0.0) at its
   third, **When** a test, for the times T = 1768528800.0, T + 6.5 and T + 13.0 in
   turn, sets the clock to the time, awaits `before_exposure` with the worked ask, and
   calls `accepted` with the worked frame (`start` that time, `exposure_s` 5.0) and a
   solve for the tracking
   error (0.0, 0.0), (0.6, 0.8) and (0.1, 1.2), **Then** a committed test asserts that
   after the second sub `drift_arcsec` is 0.76923 ± 10⁻⁴ and `error_rms_arcsec` 0.70711
   ± 10⁻⁴ (1/√2, written `std::f64::consts::FRAC_1_SQRT_2` in the test: clippy's
   `approx_constant` refuses the literal); and after the third `drift_arcsec` is 0.58583
   ± 10⁻⁴, `drift_px` 0.27122 ± 10⁻⁴, `error_rms_arcsec` 0.90370 ± 10⁻⁴ and
   `error_rms_px` 0.41838 ± 10⁻⁴ (worked example: the drifts are 1.0″ × 5/6.5 and 0.4″ ×
   5/6.5, the second after the step of (−0.5, 0.0) is taken out; the readout is their
   root mean square, and the errors'; the scale is 2.16″ a pixel). After one sub, none
   of the four keys is present. **And** on another such steering, with the second sub's
   solve at half the worked scale, after the second sub `error_rms_px` is
   `error_rms_arcsec` / 1.08 within 10⁻⁴; and on another, with the first gap answering
   `Stepped` (0.3, 0.0), before the first error, the second sub's `drift_arcsec` is
   still 0.76923 ± 10⁻⁴. **And** on another, where the fourth gap, at T + 19.5 and
   answering (0.4, 0.0), is asked before the third sub is handed over, the third sub's
   `drift_arcsec` is still 0.58583 ± 10⁻⁴. On another, whose tracker reads not steady
   once just after its third gap answers (0.3, 0.0) and then serves a fourth answering
   (−0.8, 0.0) in the same call, it is 0.58583 too: a call's steps are summed. On
   another with the errors (0.0, 0.0), (0.8, 0.6) and (1.2, 0.1) and a third gap
   answering (0.0, −0.5), it is 0.58583 too.
2. **Given** the same steering after 12 subs 6.5 s apart whose errors are (1.0, 0.0) for
   the first two, (2.0, 0.0) for the third and (0.0, 0.0) for the last nine, **Then** a
   committed test asserts `error_rms_arcsec` is 0.63246 ± 10⁻⁴ (√(4/10): the window is
   the last ten, of which the third error is the oldest; a window of nine would give 0.0
   and one of eleven 0.67420); and `drift_arcsec` is 0.51282 ± 10⁻⁴ (2.0 × 5/6.5/3: the
   nine drifts between the last ten errors, the window's first error's own drift left
   out; with it, 0.57335). **And** after `release` and a new `before_exposure`, none
   of the four keys is present. **And** on another after three subs, once the tracker's
   limit falls under half the held exposure and `before_exposure` waits, with `waiting`
   true, none of the four keys is present: letting the tracker go empties the window.
3. **Given** `SteeringBlock` rendered with a view whose `steering` is
   `{"smear_px":6,"on":true,"step_share":0.5,"rate_share":0.3,"source":"subs","holding":true,"steady":true,"waiting":false,"subs":12,"dropped_not_steady":0,"exposure_s":5,"limited":false,"over_limit":false,"longest_exposure_s":9.29,"would_take_s":5,"error_rms_arcsec":0.9037,"error_rms_px":0.41838,"drift_arcsec":0.58583,"drift_px":0.27122}`,
   **Then** a committed test asserts one region named `Tracking from the subs` whose
   status reads exactly `Steered by the subs · 12 used`, and terms reading
   `Tracking error from the subs (RMS, last 10)` with `0.9″ · 0.4 px`, `Drift per sub`
   with `0.6″ · 0.3 px`, `Exposure` with `5.0 s`, `Longest exposure (6 px corner smear)`
   with `9.3 s` and `A stack started now would take` with `5.0 s`; that the readout's
   terms are exactly those five; and no alert.
4. **Given** the same with `limited` true, `exposure_s` 30 and `longest_exposure_s`
   37.2, **Then** a committed test asserts that `Exposure` reads exactly
   `30.0 s, shortened to keep corner stars round` and
   `Longest exposure (6 px corner smear)` reads exactly `37.2 s` (37.2 and not the
   worked limit of 37.15, which a binary float holds a hair under and JavaScript's
   `toFixed(1)` prints as `37.1`: the test is of the words, not of rounding a tie); that
   with `smear_px` 1.5 that term is named `Longest exposure (1.5 px corner smear)`; that
   with `longest_exposure_s` `"no limit"` it reads `none`; that with `over_limit` true
   and `longest_exposure_s` 25.1 an alert reads exactly
   `Corner stars are trailing: the longest exposure for round stars is now 25.1 s, and this stack takes 30.0 s. It keeps running. To shorten the exposure, stop it and start a new stack.`;
   that without `would_take_s` there is no term `A stack started now would take`, and
   with it 15 and no `exposure_s` it reads `15.0 s` and there is no term `Exposure`;
   that with `error_rms_px` absent there is no term
   `Tracking error from the subs (RMS, last 10)`; that with `dropped_not_steady` 2 no
   term names it, 035's own `Dropped` line being where it is shown; and that with no
   `steering` in the view the region shows the form and no status: `Corner smear (px)`
   reads `6`, the box is ticked, `Step share` reads `0.5` and `Rate share` reads `0.3`.
5. **Given** `SteeringBlock` with the view of US8-S3, **When** a test sets the field
   `Corner smear (px)` to 1.5, unticks `Steer the tracker from the subs` and clicks
   `Save tracking settings`, with `fetch` answering 200 and a view, **Then** a committed
   test asserts `fetch` was called with `/api/imaging/steering` and
   `{"method":"PUT","headers":{"Content-Type":"application/json"},"body":"{\"on\":false,\"smear_px\":1.5,\"step_share\":0.5,\"rate_share\":0.3}"}`,
   then `onChange`; and that on another such block, with `Step share` set to 0.25 and
   `Rate share` to 0.1, the body is
   `{"on":true,"smear_px":6,"step_share":0.25,"rate_share":0.1}`; and that a 400 answer
   with `{"error":"the corner smear must be a number from 0.1 to 20"}` shows an alert
   with exactly that text and calls no `onChange`.
6. **Given** `ImagingPage` rendered as 031's own test renders it, with a view that holds
   the `steering` of US8-S3, **Then** a committed test asserts the region
   `Tracking from the subs` is present and comes after 035's stack block in document
   order, and that a click on `Save tracking settings` there reads `/api/imaging` once
   more.
7. **Given** `SteeringBlock` with the view of US8-S3, **Then** a committed test asserts
   that the status reads exactly: with `source` `tracker` and `holding` false,
   `Steered by the tracker's own corrections`; with `source` `tracker`, `holding` true
   and `on` true,
   `No sub accepted lately: the tracker's own corrections steer until one is. See Dropped in the stack.`;
   with `source` `tracker`, `holding` true and `on` false,
   `The switch is off: the tracker's own corrections steer`; with `source` `tracker` and
   `far_arcsec` 476.4,
   `The target is 7.9′ from the set point: the tracker steers on its own. Centre it, or turn Track off and on to apply the measured aim point.`;
   and with `source` `none`, `Not tracking: nothing steers`, with no alert while
   `holding` is false and, while it is true, an alert that reads exactly
   `Nothing is tracking, and the stack is still exposing. Turn Track on or stop the stack.`
8. **Given** `SteeringBlock` with the view of US8-S3 and `waiting` true, **Then** a
   committed test asserts that the status reads exactly: with `pause` `starting`,
   `Exposures paused: tracking is starting`; with `reversing`,
   `Exposures paused for a moment: the mount is reversing`; with `measuring`,
   `Exposures paused: the mount is measuring its gear play, once, for a few minutes`;
   with `overhead`,
   `Exposures paused: the target is too near overhead to follow. They resume when it has passed`;
   with `pushed`, `Exposures paused: the telescope is being moved by hand`; with
   `stalled`,
   `Exposures paused for a moment: the mount stopped and is being started again`;
   with `limit` and `exposure_s` 30,
   `Exposures paused: corner stars would trail. They resume when the longest exposure for round stars reaches 15.0 s`;
   and with no `pause`, or with one the page does not know, `flipping`,
   `Exposures paused: the tracker is not steady`. With `waiting` false and `steady`
   false the status is the one its `source` gives, `Steered by the subs · 12 used`: only
   a wait is a pause.
9. **Given** `SteeringBlock` with the view of US8-S3, **When** a test sets the field
   `Corner smear (px)` to 1.5 and renders the block again with the same view, **Then** a
   committed test asserts that the field still reads `1.5`; and that when it is then
   rendered with a view whose `smear_px` is 3, the field reads `3`: the fields are
   drafts of the block's own, which the page's one-second read of the same settings
   leaves alone and a change of the settings replaces. **And** with a view that has no
   `steering`, the field `Corner smear (px)` set to 3 reads `6` once the block is
   rendered with the view of US8-S3: a block first given a steering takes its values.
10. **Given** 035's `StackBlock` rendered with a view whose `stack` is stacking with
    `dropped` holding `not_steady` 2 and every other count 0, **Then** a committed test
    asserts that its line reads exactly `Dropped 2: 2 while the mount was not steady`;
    and that with no `not_steady` key in `dropped` and `behind` 4 it reads
    `Dropped 4: 4 behind`, as before.

---

### User Story 10 - Not steady at its edges, tested on the tracker (Priority: P2)

As the factory that builds User Story 6, I test in a story of their own what that
story's tracker does around a time it is not steady: what the mount is left running at
the first gap after one, a first reversal's measurement and its close, a start and a
stop, what a take-up forgets, the keyhole pass, subs steered through a reversal, a push
by hand, both axes stopped behind the tracker's back, what a pointing and a tracking
error do around a time not steady, and a gap queued behind a take-up when tracking
ends.

This story is built after User Story 6 and before User Story 7 (Work Graph). Its
fourteen scenarios test User Story 6's code, the last five, US10-S10 to US10-S14, cut
from that story by tests, where they were US6-S6 to US6-S10: this story writes nothing
under `crates/finder/src`, and on User Story 6's code its scenarios already pass
(tasks.md, US10's test tasks).

**Why this priority**: The first exposure after a take-up, a measurement or the keyhole
pass is the one most likely to start on a rate nobody meant, and a stop that left a gap
unanswered would hang a stack for good. It follows User Story 6, whose tracker it tests,
and comes before User Story 7, since both add a `mod` line to
`crates/finder/tests/motion/main.rs`.

**Independent Test**: `crates/finder/tests/motion/steering_unsteady.rs`, the module
`steering_unsteady` of finder's `motion` test binary, on the backlash rig and, for
US10-S10, on `crate::support::goto::Rig`. The `test`
gate's `cargo nextest run --workspace --all-targets --all-features --locked` runs it;
during development,
`cargo nextest run -p finder --locked --test motion steering_unsteady::`.

**Acceptance Scenarios**:

1. **Given** the lapsed backlash rig and a `pointing(0.0, 0.0)` right after its ninth
   gap, **When** a `pointing(30.0, 0.0)` arrives once the take-up at transit has made
   `steady()` false and a `gap` is awaited as soon as `steady()` reads true again, both
   read every 10 ms, **Then** a committed test asserts that the azimuth rate in force
   when the gap answers, the last in the rate log at or before its answer, is the
   feed-forward rate (as `crates/finder/tests/motion/tracking/pi_loop.rs:252` computes
   it) plus the tracking offset read just after the answer, within 0.01″/s, and that so
   is the first rate after it (oracle: the rig's rate log. The first pointing gives the
   second an interval of about 50 s, so the proportional rate 015 sets for the second,
   about +0.5″/s of azimuth, outlasts the settling; with no earlier pointing it would
   last 1 s, and a turn would have dropped it before the tracker was steady again.
   US10-S12 and US10-S9 are the two ways the tracker's own state stops showing that rate
   while the mount still runs it).
2. **Given** `crate::support::tracker::Rig` for M42 from 120 s before its transit on a
   fresh session, so that no backlash is measured, followed to transit + 30 s, while the
   first reversal is measured, **Then** a committed test asserts that `steady()` is
   false, that `pause()` is `Some("measuring")`, and that a `gap` awaited then answers
   exactly (0.0, 0.0) and leaves `steered_by_subs()` false. **When** the rig is followed
   on to transit + 600 s with a reader that, every 50 ms, notes whether
   `GET /api/session`'s `backlash.alt_deg` is present and then `steady()`, with no await
   between the two, **Then** some reading has the backlash measured, and every reading
   that has it has `steady()` true (oracle: 015's own measurement, which the session
   reports once it has closed).
3. **Given** the backlash rig as `Rig::new` leaves it, with the console time s read as
   soon as it answers and no wait, **Then** a committed test asserts that the rate log
   holds no rate, `steady()` is false and `pause()` is `Some("starting")`; that a `gap`
   awaited then answers exactly (0.0, 0.0) and leaves `steered_by_subs()` false; that
   0.1 s later the log holds a rate for each axis and `steady()` is still false; that it
   is false 1.9 s after the first rate in the log and true 2.1 s after it, with
   `pause()` then `None`, and that `steady_between` from 2.0 s to 3.0 s after it is
   true: the tracker settles for 2 s after its first rates, which a turn sends at its
   start, and a time not steady ends at the instant it is steady again; that
   `steady_between(s - 20.0, s - 10.0)` is
   false: it was not tracking then; that `steady_between(now - 10.0, now)`, now being
   the console time when it is asked, is false asked 6 s after s and true asked 14 s
   after s: the window 037's mount part asks, ending now and reaching back the larger of
   10 s and the exposure; and that after `rig.follow(s + 20.0, 60.0)` and
   `stop("stopped")`, with e the console time read just before the stop, `steady()` is
   false, `steady_between(e - 3.0, e + 0.1)` and `steady_between(e + 1.0, e + 2.0)` are
   false, and `steady_between(e - 3.0, e - 1.0)` is still true (oracle: the rig's rate
   log and clock).
4. **Given** the lapsed backlash rig and, 30 s after its ninth gap, a
   `pointing(30.0, 0.0)` (held, since the tracker is steady and lapsed), **When** the
   take-up at transit has made `steady()` false and then true again, both read every
   10 ms, and a `gap` is awaited then, **Then** a committed test asserts that it answers
   exactly (0.0, 0.0): what was held when the tracker stopped being steady was forgotten
   (awaited during the take-up, the gap would be turned away while the tracker settles,
   and prove nothing).
5. **Given** the backlash rig followed to 60 s before the transit with
   `rig.follow(transit - 60.0, 60.0)`, one `gap` awaited then and, 50 s later, a `steer`
   of (12.0, −16.0) at the time then, **When** the take-up has made `steady()` false and
   then true again, both read every 10 ms, and a `gap` is awaited then, **Then** a
   committed test asserts that it answers exactly (0.0, 0.0) and that
   `steered_by_subs()` is true: the take-up dropped the claim and the error not yet
   applied, and that gap took the claim back.
6. **Given** the backlash rig at transit − 120 s with the console time s read at once,
   and `let steered: &dyn Steered = &rig.tracker`, **Then** a committed test asserts
   that `steered.steady()` is false and `steered.pause()` is `Some("starting")`; and
   that after `rig.follow(transit + 10.0, 60.0)`, which crosses the take-up,
   `steered.steady()` is true, `steered.pause()` is `None`,
   `steered.steady_between(s - 20.0, s - 10.0)` is still false,
   `steady_between(transit - 5.0, transit + 5.0)` is false and
   `steady_between(s + 5.0, s + 20.0)` is true: the trait answers as the tracker does,
   still, after a take-up.
7. **Given** the backlash session and a clone of M42 at Dec +39°, as
   `crates/finder/tests/motion/tracking/keyhole.rs` uses, from 300 s before the first
   whole minute at which its altitude reaches 88°, with the rig's true error given as a
   pointing every 2 s, **When** the test reads `state().state`, `steady()` and `pause()`
   every 50 ms, for at most 1800 s of the rig's clock, until the tracker is `tracking`
   and steady after a `keyhole_wait`, **Then** a committed test asserts that a
   `keyhole_wait` was seen, that every reading in it has `steady()` false and `pause()`
   `Some("overhead")`, that the tracker is steady at the end, and that `steady_between`
   over the 2 s either side of the first `keyhole_wait` reading, asked at the end, is
   false (oracle: 015's keyhole pass, as its own test drives it).
8. **Given** the backlash rig with `rig.mount.set_rate_error(0.01)` called as soon as
   `Rig::new` answers, **When** a test plays subs of 5 s late until the rig's clock
   passes transit + 120 s, with three changes to playing a sub: the truth error is
   `rig.truth_error()`; before each sub it waits, reading every 50 ms, until `steady()`
   is true; and a sub for which `steady_between`, asked as the sub ends for the console
   times of its e0 and e1, is false is left out, its drift not judged and its error not
   handed over, **Then** a committed test asserts that `steady()`, read at each of the
   player's 50 ms steps, read false at least once after the player's first sub; and
   that, of the subs whose e0 was read at or after its first true reading after the
   last of its false readings, there are at least four, and every one from the fourth
   on has a true drift of at most 0.15″ (oracle: the simulator's truth. Probe round 2
   measured at most 0.00654″ over 18 subs, three runs, so the margin is 22.9×; 0.15″ is
   036's US4-S5 bound for the same drift. The sub after a left-out one hands no error
   over).
9. **Given** the lapsed backlash rig and a `pointing(0.0, 0.0)` 5.4 s before the
   transit, **When** a `pointing(30.0, 0.0)` arrives once the take-up at transit has
   made `steady()` false, at the console time p, and a `gap` is awaited at the first
   10 ms step at which `steady()` reads true and the console time is at least p + (p −
   (transit − 5.4)) + 0.01, **Then** a committed test asserts that the last azimuth rate
   the log holds before that gap's call differs from the feed-forward rate plus the
   tracking offset by more than 1″/s, and that the azimuth rate in force when the gap
   answers, the last in the rate log at or before its answer, is the feed-forward rate
   plus the tracking offset read just after the answer, within 0.01″/s (oracle: the
   rig's rate log. The second pointing's proportional rate lasts for its interval, p −
   (transit − 5.4), about 3.4 s: the turn that ends the settling still sends it, and its
   time runs out before the gap, so the tracker's own state says no rate is in force
   while the mount runs one until the next turn. The first assertion shows that the case
   was met. The 5.4 s is worked: from the take-up's start as the recheck of round 1
   saw it, 1.99 s before the rig's transit, and the 3.0 s from there to the turn that
   ends the settling; probe round 2 measured the interval at 3.410 s and confirmed it).
10. **Given** the tracking console, its session's tracker taken once
    `steady()` reads true (read every 100 ms, for at most 10 s) and one `gap` awaited, so
    that `steered_by_subs()` is true, **When** a test pushes the tube with
    `rig.mount.push_by(0.0, -0.1)`, 6′ as `crates/finder/tests/motion/tracking/push.rs`
    pushes it, and from then on reads `GET /api/session` and the tracker every 100 ms;
    puts the tube back with `rig.mount.push_by(0.0, 0.1)` at the first read whose
    `track.push.state` is `moving`; and, once `track.push` is absent again, pushes it a
    second time with `rig.mount.push_by(0.0, -0.1)` and leaves it there, **Then** a
    committed test asserts that:
    - at the first read whose `track.push.state` is `moving`, `steady()` is false,
      `pause()` is `Some("pushed")` and `steered_by_subs()` is false, and a `gap` awaited
      then answers exactly (0.0, 0.0) and leaves `steered_by_subs()` false;
    - at the first read after it that has no `track.push`, `steady()` is true, and a
      `gap` awaited then makes `steered_by_subs()` true: a push that comes back ends the
      time not steady with no follow;
    - after the second push, with m the console time of the first read whose
      `track.push.state` is `moving` and f that of the first whose state is `following`,
      `steady()` is false at m and true at f, `steady_between(m, f)`, asked at f, is
      false, and a `gap` awaited at f makes `steered_by_subs()` true (oracle: 038's own
      `track.push`, as its tests read it. While the session reads a push as moving the
      tracker is not steady: the claim is dropped, no gap steps the telescope back toward
      the old place, and the stack waits).
11. **Given** the backlash rig followed to 60 s before the transit with
    `rig.follow(transit - 60.0, 60.0)` and one `gap` awaited then, so that
    `steered_by_subs()` is true, **When** a test waits 2.5 s, then waits until the rate
    log gains an azimuth rate, at the time r, waits 0.5 s, stops both axes on the
    simulated mount itself with `rig.mount.stop_all().await`, and waits 2 s, **Then** a
    committed test asserts that `steady_between(r + 0.1, r + 0.9)` is false; that
    `steady_between(r - 5.0, r - 0.1)` and `steady_between(r + 1.1, r + 1.9)` are true;
    that `steady()` is true and `steered_by_subs()` false at the end; and that the first
    azimuth rate the log holds after r is not zero (oracle: the rig's rate log and clock.
    Stopping the mount behind the tracker's back is what the scope node does when a rate
    command fails on the mount's side: both axes stop and the command is still
    acknowledged. The tracker's next turn finds the azimuth axis at rest although the
    rate it last sent was not zero, reads not steady from the time of that rate to its
    own next rates, which start the axes again, and so drops the claim).
12. **Given** the lapsed backlash rig and a `pointing(0.0, 0.0)` right after its ninth
    gap, **When** a `pointing(30.0, 0.0)` arrives once the take-up at transit has made
    `steady()` false, a `pointing(0.0, 0.0)` arrives as soon as `steady()` reads true
    again, both read every 10 ms, and a `gap` is awaited at once, **Then** a committed
    test asserts that the azimuth rate in force when the gap answers, the last in the
    rate log at or before its answer, is the feed-forward rate plus the tracking offset
    read just after the answer, within 0.01″/s, and so is the first rate after it: a
    pointing inside 015's deadband that arrives steady and lapsed zeroes the proportional
    rate in the tracker's own state and sends nothing, so only a gap that sends whatever
    that state says takes the rate off the mount (oracle: the rig's rate log).
13. **Given** the lapsed backlash rig, with b the console time read right after its ninth
    gap, **When** the take-up at transit has made `steady()` false and a test hands over
    a `steer` of (12.0, −16.0) at the time now; and, once `steady()` reads true again,
    both read every 10 ms, it hands over a `steer` of (12.0, −16.0) with the time b and
    awaits a `gap`, **Then** a committed test asserts that the gap answers exactly (0.0,
    0.0) and leaves `steered_by_subs()` false: a tracking error handed over while the
    tracker is not steady, or measured before it stopped being steady, steers nothing and
    takes no lapsed tracker back; and that a `steer` of (0.0, 0.0) at the time now and
    one more gap then make `steered_by_subs()` true (oracle: 036's FR-011 and FR-021).
14. **Given** the backlash rig followed to 3 s before the transit with
    `rig.follow(transit - 3.0, 60.0)`, **When** a test reads `steady()` every 10 ms
    until it is false, for at most 5 s, and then joins, on its own task, an asker that
    awaits `rig.tracker.gap()` under a timeout of 1.1 s and a stopper that waits 0.1 s
    and calls `rig.tracker.stop("stopped")`, **Then** a committed test asserts that the
    gap answered, exactly (0.0, 0.0): the take-up holds the rate task for most of a
    second, the gap waits in the tracker's queue, and tracking that ends answers every
    gap still waiting (oracle: 015's Stop; without it a stack's taker would wait in
    `before_exposure` for ever).

---

### User Story 11 - An MCP client sets the steering (Priority: P3)

As an agent that runs a night through the console's MCP tools, I set what the page's
form sets: whether the subs steer, the corner smear, and the two shares. I read the
steering where I already read the camera, in `get_imaging_camera` and the session tools.

It is built last (Work Graph), after the page's story, because it edits the README's list
of tools after that story's section.

**Why this priority**: 037 gives an agent the whole night, and without the tool an agent
could neither compare a stack with and without the feedback nor change the smear before
a restart. It is P3 because the page does the same for an observer.

**Independent Test**: `crates/finder/tests/mcp/steering.rs`, the module `steering` of
finder's `mcp` test binary, which goes through the MCP door. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test mcp steering::`.

**The imaging console** of this story is a console that takes imaging frames, built in
the module on a stand-in scope node whose camera is ready, as 031's own MCP tests build
theirs (031 spec.md US8), and reached through the MCP door as they reach it.

**Acceptance Scenarios**:

1. **Given** the imaging console, **Then** a committed test asserts that `tools/list`
   names `set_steering` with the description
   `Set what steers the tracker during a live stack, as PUT /api/imaging/steering does: on, smear_px, step_share and rate_share, each optional. Read the result in the steering part of get_imaging_camera.`,
   an `outputSchema` of `{"type":"object"}`, no `readOnlyHint`, and an `inputSchema`
   whose `properties` are exactly `on`, `rate_share`, `smear_px` and `step_share` and
   which requires none of them, and whose properties' types are `boolean` for `on` and
   `number` for the other three, none of them nullable.
2. **Given** the imaging console, **Then** a committed test asserts that `set_steering`
   with `{"smear_px":1.5}` answers the imaging view, whose `steering.smear_px` is 1.5
   and `steering.on` true; that with `{"on":false,"step_share":0.25,"rate_share":0}` it
   answers `steering.on` false, `steering.step_share` 0.25 and `steering.rate_share`
   0.0, with `smear_px` still 1.5; that `get_imaging_camera` then carries the same
   `steering`, and `get_session`'s `imaging.steering` too; and that `{}` answers the
   view and changes nothing (oracle: the route's own answers, 036's US5-S5).
3. **Given** the imaging console, **Then** a committed test asserts that `set_steering`
   with `{"smear_px":0}` answers a tool error whose text is exactly
   `the corner smear must be a number from 0.1 to 20`, with `{"step_share":1.5}` exactly
   `the step share must be a number from 0 to 1`, and with `{"smear":2}` and with
   `{"on":1}` a tool error, and that none of the four changes anything; and that on a
   console that takes no imaging frames `set_steering` with `{}` answers the tool error
   `this console takes no imaging frames`, with no tokio time passed, and `{"smear":2}`
   answers a tool error other than that one: the arguments are judged first, as the
   route judges its body (oracle: the route's own refusals; an existing test calls
   every listed tool with `{}` on such a console, and a tool that waited there would
   hang it).

---

### Edge Cases

- **No Track.** With Track off, or armed and waiting for a GoTo, there is no tracker to
  steer: the stack is neither capped nor paused and hands over nothing (US7-S4). Subs
  taken during a slew fail 035's own gates.
- **Tracking ends during a stack.** The sub in flight is dropped as `not_steady`; later
  subs find no tracker. Stopping the stack is 035's and 037's; until then the block says
  that nothing is tracking and the stack is still exposing (US8-S7).
- **A push by hand, not yet settled.** From the live pointing that makes 038's push
  `moving` until the push is followed or the telescope is back within 038's threshold,
  the tracker is not steady: exposures pause, the sub in flight is dropped as
  `not_steady`, and the claim and any error not yet applied are dropped as for a
  take-up. When the push is followed or comes back, the stack's next gap claims again.
  One finder solve more than 5′ wrong costs one sub in the same way (US10-S10).
- **A rate command that fails at the scope node.** The node stops both axes and still
  acknowledges the command. The tracker's next turn finds the azimuth axis at rest
  although the rate it last sent was not zero and reads not steady from that rate to its
  own next rates, which start the axes again: the sub in flight is dropped as
  `not_steady` and steers nothing (US10-S11). What the node does with a failed command is
  another spec's.
- **Settling.** For 2 s after the rates that follow a tracker's start, a take-up and the
  keyhole pass the tracker is not steady, and no exposure starts: a real tube rings
  after each (US6-S1, US10-S3).
- **The first reversal of an axis on this mount.** Its backlash is still to be measured
  and the measurement takes minutes (altitude reverses at transit, and azimuth too for a
  target that culminates north of the zenith); the tracker is not steady for all of it,
  exposures pause, and the finder camera steers as 015 has it, even when the subs had
  lapsed before it (owner decision 17, US6-S5).
- **A long gap.** The feed-forward rates still go out once a second. Nothing times a gap
  out: a stack that is waiting for a tracker in the keyhole waits until it leaves, and
  stopping the stack ends the wait. The block says why exposures are paused (US8-S8).
- **The limit falls under half the exposure held.** The stack takes no exposure until the
  limit has risen again, lets the tracker go meanwhile, and the block says so (US6-S10,
  US8-S8).
- **The observer edits the exposure during a stack.** The edit is refused: while the
  stack holds the camera's lease, `PUT /api/imaging/settings` answers 409 with the
  stack's name (035 plan.md § The camera's lease; 035 US11-S3). So the exposure a stack
  holds cannot change under it. The observer stops the stack, edits and starts again;
  the setting they then see is their own, put back when the stack ended.
- **The exposure cannot be shortened.** If the camera refuses the stack's change of the
  exposure before its first sub, which it does only when it is no longer ready, the
  stack ends with the reason `camera` and the refusal's words, and no sub is taken at
  the observer's longer exposure (US7-S5).
- **The observer's exposure cannot be put back.** If the camera is no longer ready when
  a stack ends, the change back is refused and the setting stays at the exposure the
  stack used; the stack's own ending is not changed by it (US7-S10). The page shows the
  setting.
- **`--simulate`.** The simulated solver and camera steer the simulated tracker as the
  real ones would, with no flag.

## Requirements *(mandatory)*

### Functional Requirements

**Not steady** (US6):

- **FR-021**: The tracker's `steady` SHALL be false from its start until 2 s after its
  first rates are sent; from the start of a take-up or of the keyhole pass until 2 s
  after the rates that follow it are sent, the 2 s ending with the first of the
  tracker's once-a-second turns to end at least 2 s after those rates; while an axis's
  backlash is being measured; while the session reports the telescope moving by hand
  (038), from the live pointing that makes the push `moving` until the push is followed
  or the telescope is back within 038's threshold; from one turn's rates to the next
  turn's, when that next turn finds the azimuth axis at rest although the rate last sent
  it was not zero; and after tracking ends (owner decision 17). `steady_between` SHALL
  be false for a window that touches any such time among the last 64, or that begins
  before the tracker was started, while that time is still among the last 64 (the list's
  first entry); a window that begins at the instant such a time ends does not touch it.
  A tracker that stops being steady SHALL stop being steered by the subs and forget any
  tracking error not yet applied and any correction held for a gap (036's FR-011 says which
  errors it then ignores); a gap served while the tracker is not steady SHALL answer no
  step and claim nothing, and one asked for during a take-up or the keyhole pass SHALL
  be served once that is over. While the tracker is not steady a live pointing SHALL
  correct it as in 015, even when its claim has lapsed. The tracker's `pause` SHALL
  answer, while it tracks and is not steady, one word for the reason: `pushed` while the
  telescope is moved by hand, else `measuring` while an axis's backlash is measured,
  else `starting`, `reversing` or `overhead` for a start, a take-up and the keyhole
  pass, each with the 2 s that follow it; else `stalled` in a turn that found an axis
  stopped; and none otherwise.
- **FR-022**: `before_exposure` SHALL not answer, and SHALL call no `gap`, while the
  running tracker is not steady, with `steady` false and `waiting` true in the view, and
  `pause` the tracker's word for the reason where it gives one; it SHALL answer only
  after a `gap`, where one is called, at whose end the tracker is steady, and the view
  SHALL then have `steady` true, `waiting` false and no `pause`, as it has before any
  wait. While a `gap` is awaited it SHALL go on reading the tracker, and show `steady`
  false and `waiting` true while the tracker is not steady. It SHALL also not answer,
  with `waiting` true and `pause` `limit`, while the tracker's answer for now (036's FR-017)
  is under half the exposure held, and SHALL let the tracker go for that wait as the
  switch turned off does (036's FR-019). `Steering::check` SHALL answer `Drop("not_steady")`
  for a sub when the tracker that `before_exposure` found before that sub's exposure no
  longer exists, or does not say it was steady for the whole of the exposure; and `Keep`
  otherwise, a sub taken with no tracker among them. The view SHALL count one stack's
  drops in `dropped_not_steady`, from 0 again when the next stack takes its `Hold`.

**The stack** (US7):

- **FR-023**: 035's taker SHALL keep a `Hold` while it runs and await `before_exposure`
  before every exposure, a stop of the stack ending the wait. Before its first exposure,
  when the exposure answered differs from the observer's, it SHALL set the console's
  exposure to the one answered through the stack's own lease on the camera; if that is
  refused it SHALL end the stack with the reason `camera` and the error
  `the exposure could not be set to {ms} ms: {the refusal's words}`, having taken
  nothing. The taker SHALL, on every return, put the observer's exposure back through
  the same lease before the lease drops with it, and a refusal there SHALL change
  nothing else. 035's worker SHALL ask `check` for every sub before it solves it and
  count a sub it is told to drop under `not_steady`, a twelfth count of `dropped`; and
  SHALL call `accepted` for every sub it stacks (owner approval 6).
  `crates/finder/src/imaging/stack/service.rs` SHALL hold none of the thirteen names
  036's FR-005 lists.
- **FR-024**: Giving the console its imaging solver SHALL give its steering the
  session's running tracker as its source, in the binary and under `--simulate` alike.
- **FR-025**: `crates/finder/README.md` SHALL gain a section on tracking from the subs:
  what steers when, the gap, the cap, the pause and its reasons, what each key of the
  `steering` part means, why subs are dropped, and the constants (owner approval 8).

**The readout** (US8):

- **FR-026**: The view SHALL carry, once two tracking errors have been handed over
  since the tracker was last released, `error_rms_arcsec` and `drift_arcsec`, each the
  root mean square over the last ten tracking errors, and the same in delivered pixels
  as `error_rms_px` and `drift_px`, by the latest solve's scale. A sub's drift SHALL be
  the change in tracking error since the sub before, less the steps answered by the
  gaps between their mid-exposures, times that sub's exposure over the time between the
  mid-exposures, worked out when its tracking error is handed over.
- **FR-027**: The Imaging page SHALL show a region `Tracking from the subs` after 035's
  stack block, with the words of plan.md § The page: what steers, and why when it is not
  the subs, the readout to 0.1″, 0.1 px and 0.1 s, the longest exposure and what a stack
  started now would take, why exposures are paused, and a form that sends the four
  settings, the two shares under a `Tuning` heading (owner approval 7).
- **FR-028**: `DESIGN.md` SHALL describe the block, `.impeccable/design.json` SHALL list
  it as a component, and `crates/finder/web/dist` SHALL be the build of its source.
- **FR-029**: 035's `StackBlock` SHALL name subs dropped as `not_steady` in its
  `Dropped` line with the words `while the mount was not steady`, last, and read a
  `dropped` without that key as before (owner approval 7).

**The MCP tool** (US11):

- **FR-031**: The console's MCP server SHALL offer `set_steering`, which takes the body
  of `PUT /api/imaging/steering` and answers the imaging view, a refusal of the route's
  as a tool error with the route's words, and `this console takes no imaging frames` at
  once without imaging. `crates/finder/README.md` SHALL name it among the imaging
  camera's tools, and SHALL say there that `get_imaging_camera` carries the `stack` and
  `steering` parts (owner decision 11; owner approvals 3 and 8).

### Key Entities

036's spec.md § Key Entities defines the tracking error, the set point, the gap, the
step, the drift, the drift rate and the steering. This spec adds:

- **Pause**: a time a stack takes no exposure because the tracker is not steady or its
  limit is under half the exposure held; the view says which with one word.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario has a committed test that asserts it, and the
  `lint` and `test` gates pass.
- **SC-002**: No existing test, golden, `scripts/ci/` file, `.github/` file,
  `CODEOWNERS` entry, `ergane.yaml` or `playwright.config.ts` is edited, 036's test files
  among them. The only edit to an existing file under `crates/finder/tests` is one `mod`
  line for each new module, in alphabetical order, in
  `crates/finder/tests/console/main.rs`, `crates/finder/tests/motion/main.rs` and
  `crates/finder/tests/mcp/main.rs` (owner approval 9).
- **SC-003**: On a simulated mount that runs 1% fast, a stack's drift within a 5 s sub is
  at most 0.15″ once the subs steer (from the 14th sub in US7-S1), against 0.75″ without.
- **SC-004**: Every test of 015 passes unchanged, and 035's
  `crates/finder/src/imaging/stack/service.rs` holds no alt-az arithmetic (US7-S6).
- **SC-005**: A time the tracker is not steady drops the claim, and from its end to the
  stack's next gap the tracker's own corrections run as 015 has them; the sub that time
  touched is dropped, and no kept exposure begins before that gap (FR-021, FR-022).

## Assumptions

- **Earlier features are in place.** `036-tracking-from-subs` has landed, and with it the
  specs it depends on, 015, 030 to 035 and 038 (flip.md § Prerequisites). What this spec
  uses of 036 is plan.md § What 036 provides. 035's stack holds the camera through 031's
  lease, which this trio reaches through 035 (plan.md § What 035 provides). It was
  re-anchored to 036's code on 2026-11-20 (flip.md § Prerequisites 2 and 3): every
  `path:line` here was read by its content on `main` at `fc161a54`, whose line numbers
  are cited; those the draft held were first read at 49bb210.
- **Out of scope:**
  - everything 036 builds, which this spec uses as landed (plan.md § What 036 provides);
  - stopping a stack when tracking ends, and the run's states (037);
  - measuring a mount's backlash any other way than 015 does;
  - changing the words `get_imaging_camera` gives itself in the tool listing, which a
    test of 031's pins (owner decision 11).
- **Outside the factory:** GoTo and Track proven on the real mount (the plan document,
  § Before any flip, 4), and its backlash measured on both axes (azimuth reverses for a
  target that culminates north of the zenith), before this spec is used at the
  telescope; the field checks flip.md lists; the Impeccable review of the block, after
  US8 lands; and rebuilding the console on the console machine.
- **The factory's gates and CI:** the gates are `lint` and `test`, copied in plan.md
  § Verification. CI's other jobs run only in the merge queue, so no scenario's Then
  clause rests on them.

## Work Graph

```yaml
US6:
  depends_on: []
  implements: [FR-021, FR-022]
US7:
  depends_on: []
  depends_on_merged: [US6, US10]
  implements: [FR-023, FR-024, FR-025]
US8:
  depends_on: []
  depends_on_merged: [US6, US7]
  implements: [FR-026, FR-027, FR-028, FR-029]
US10:
  depends_on: []
  depends_on_merged: [US6]
  implements: []
US11:
  depends_on: []
  depends_on_merged: [US8]
  implements: [FR-031]
```

Every edge is a merge edge, because each dependent builds on what its predecessor
creates. US6 adds to what 036 left in `steering.rs`, `tracking.rs` and `session.rs`, and
has no edge in this spec: 036 has landed before it. US10 tests US6's tracker as 036's US9
tests 036's US3's, with no code and no FR, and follows US6. US7 wires the finished
steering into 035's stack and writes the README section after 036's route row; it names
US10 as well as US6, as the next story after US10 to write
`crates/finder/tests/motion/main.rs`. US8 adds the readout's figures to `steering.rs`
after US6 and shows them after US7's block of the stack exists, and names them in the
README section US7 writes. US11 adds the MCP tool last, after US8, the story before it to
write the README. Where a story names two predecessors and one already follows the other,
the second edge is there because both wrote a file the story writes.
