---
state: draft
depends_on_landed: ["014-goto", "015-tracking", "016-mcp", "017-console-stop-confirm", "030-imaging-camera-link", "031-imaging-frames-on-console", "032-imaging-solve-and-aim-point", "033-focus-reading-and-aid", "034-dark-and-flat-frames", "035-live-stack", "036-tracking-from-subs", "037-imaging-run", "038-track-follows-a-push", "040-stack-waits-for-steady"]
# The imaging run, second of three pieces: the console's own mount, camera, focus, solver
# and stack parts, the centring, the focus gate and its pause, the limits and the stack's
# own endings, and the whole run in the binary (US4 to US8), on 037-imaging-run's machine.
# Depends on 014 to 017, 030 to 038 and 040 having landed. State: draft, re-anchored to
# `main` after 037 lands and before the flip. history.md has how it got here; flip.md what
# the flip needs.
---

# Feature Specification: The Imaging Run on the Console's Own Parts

**Feature Branch**: `041-imaging-run-on-the-rig`

**Created**: 2026-11-15, as part of `037-imaging-run`; a spec of its own since 2026-11-20

**Status**: Draft

**Input**: `037-imaging-run` writes the imaging run's machine, its routes and its record,
and proves them on a script that stands in for the five parts a run is handed. This spec
gives the run the console's own parts: the mount part, which points through Track and
never moves the mount any other way; the camera part, over 031's imaging frames and
lease; the solver part, over 032's imaging solve; the focus part, over 033's focus check
and focus aid; and the stack part, over 035's stack and the steering of 036 and 040. It
adds to 037's machine the centring on the imaging camera's sensor, the focus gate and its
pause, and the limits and the stack's own endings, and wires the whole run into the
`finder` binary. `042-imaging-run-page-and-mcp` then puts the run on the Imaging page, in
four MCP tools and in the README.

## User Scenarios & Testing *(mandatory)*

Every scenario shares these definitions unless it says otherwise.

- **The gate command.** Every Rust scenario runs in
  `cargo nextest run --workspace --all-targets --all-features --locked`, the `test`
  gate's first command, each test in its own process. The page's scenarios run in
  `npm test`, a later command of the same gate, in `crates/finder/web`.
- **Paused time.** A test of a `Runner` or of the in-process console runs under
  `#[tokio::test(start_paused = true)]`, so every wait is in virtual time. A test that
  starts the built `finder` is a plain `#[test]`.
- **The script**, **a scripted run**, **settling**, **the straight path**, **the scripted
  console** and **R** are 037's, and plan.md § What 037-imaging-run provides gives each
  as 037 defines it: the script's default answers, controls and call words; a `Runner`
  over its parts on a `FixedClock` at Unix 1768528800, started with the default request
  by the page; a settle of 1 s of virtual time; the calls of a scripted run that reaches
  `stacking` with nothing changed; and 031's console rig with the script's parts added
  last, R being the child path `imaging` of the test's scratch folder.
- **The run rig** is 032's solving rig (032's spec.md § User Scenarios: the same
  simulator with a `SimMount` connected first and `push_to(50.0, 120.0)` after, the
  simulated solver adapter, no aim point) with three changes: the state is built with
  `with_live_solver(Arc::new(crate::support::goto::Stars))` in place of `with_live()`,
  never after it, because each of the two starts a live loop and a GoTo of its own
  (`crates/finder/src/api.rs:247`), so that a finder solve never leaves the paused
  runtime (`crates/finder/tests/support/goto.rs:42`); the session's target is M42; and
  the test waits until the pointing model holds a sync before its steps. Its parts are
  named by each scenario: the console's own from `finder::imaging::run::console`, or
  the script's. The test keeps its own handle of the `SimMount`.
- **The simulator's truth.** The true aim point is `sim.optics().eyepiece_px`, (700,
  420) in the simulated finder camera's frame, which is 29.78″ a pixel (032's spec.md
  § User Scenarios). The true centre of the imaging camera's frame at a time t is
  `sim.imaging_truth_at(t)` (030 US2). **The true offset** is the separation between
  that centre at the console's time and the target's catalogue place, in arcseconds.
  The test computes it with `math_core::angular_distance` over
  `UnitVector::from_radec`, which both work in radians: each RA and Dec goes in as
  `to_radians()`, and the answer comes out as `to_degrees() * 3600.0`. The simulated
  imaging frame is 1236 × 822 pixels at about 2.1845″ a pixel, so 1/20 of its shorter
  side is 89.8″ and the tolerance of owner decision 2 is 90.0″.
- **At rest.** The simulated mount is at rest when `Mount::state` of the test's
  `SimMount` handle reports `moving` false for both of its axes, as
  `crates/finder/tests/motion/stop/watchdog.rs:121` reads it. **At rest for
  10 s** means that, sampled every 100 ms of virtual time for 10 s, every sample has
  `moving` false for both axes and each axis's `position` is the one of the first
  sample.
- **Scratch folders.** A finder test module takes its folder from
  `crate::support::Scratch::new(name)` (`crates/finder/tests/support/mod.rs:12`), which
  makes the folder under the directory cargo names and removes it when dropped.
- **Requests** go through the router in-process, as `request` in
  `crates/finder/tests/support/live.rs:90` sends them, with `Content-Type:
  application/json` and no `Origin`. A scenario that names an `Origin` builds its
  request itself with a `Host` of localhost on port 8080. A foreign `Origin` is
  `http://elsewhere.example`.
- **Waiting until** a condition means reading `GET /api/imaging` every 100 ms until it
  holds, for at most 900 s of virtual time, and failing otherwise.
- **Comparing a view.** A test compares `run` key by key, never whole, because later
  specs may add keys. `elapsed_s` is compared within 0.5 s, twice the run's poll.
- **Oracles.** Each scenario names its oracle: the simulator's truth; the session; the
  answer of another route of 030 to 036 at the same moment; or, for a scenario that
  checks only what the run says, calls and writes, the words, states, reasons and call
  lists the scenario itself states, which the test compares exactly. A test parses
  `run.json` and `stack.json` itself with `serde_json`, never through the console's
  code. A count or a word has no tolerance; every other tolerance is stated where the
  number is.


### User Story 4 - Pointing through Track, on the console's own mount, camera and solver parts (Priority: P1)

As the observer, I start a run and the console points the telescope by asking Track, as
I would from the page, holds the imaging camera while it does, and solves what the
camera sees. When I press Stop, or a guard ends the motion, the run ends with it and
never restarts the mount.

**Why this priority**: This is where the run first moves real metal: the three parts
through which it meets Track, 031's imaging camera and 032's imaging solver.

**Independent Test**: `crates/finder/tests/motion/imaging_run.rs`, the module
`imaging_run` of finder's `motion` test binary, on the run rig. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test motion imaging_run::`.

In this story **the centred rig** is the run rig with the console's own mount, camera
and solver parts, the script's focus and stack parts, and the session's aim point set
first to the true aim point with `Session::set_aim`. **A centred run** is a run started
on it with `POST /api/imaging/run` and `{}`, on which the test has waited until
`run.state` is `stacking`. No scenario of this story re-centres: the machine gains that
in US5.

**Acceptance Scenarios**:

1. **Given** the centred rig, **When** a test sends `POST /api/imaging/run` with `{}`
   and waits until `run.state` is `stacking`, sampling the session every 100 ms,
   **Then** a committed test asserts, against the simulator's truth and the session,
   that:
   - the start answered 200 with `run.state` `slewing`, and the session's `motion` then
     read `by` `page` and `tool` `imaging_run`; and `run.state` read `slewing` at every
     sample at which `track.state` read `armed`, of which there was at least one;
   - `run.centring.tries` is 0, `run.centring.tolerance_arcsec` is 90.0, the floor of
     owner decision 2 (1/20 of the simulated sensor's shorter side alone gives 89.8″),
     and `run.centring.offset_arcsec` is at most `run.centring.tolerance_arcsec`: Track
     brought the target inside the tolerance with no re-centre, which is also what
     keeps this scenario true once US5 adds the centring loop;
   - the true offset, read when the test first sees `stacking`, is at most 110″: the
     tolerance and 20″ more, which allows the tracker's 10″ deadband twice over
     (`crates/finder/src/tracking.rs:31`); and `run.centring.offset_arcsec` is within
     30″ of that true offset, the deadband three times over;
   - the imaging view's `frame.purpose` read `run` when the test first saw `stacking`;
   - at no sample in the 11.5 s after `track.state` first read `tracking` did
     `run.state` read anything but `slewing` or `tracking`: the mount part reads steady
     only once 040's tracker has settled and then been steady for the whole of 032's
     fit reach, 10 s at the simulator's exposure of 1 s (FR-019; 12.1 s on the
     simulator);
   - `track.state`, once it first read `tracking`, never read anything else.
2. **Given** the centred rig, **Then** a committed test asserts that
   `POST /api/imaging/run` with `{}` answers 409: with `no target is selected` after
   `PUT /api/session` with `{"target":null}`; with the target set to M8, which
   `finder::pushto::class` gives as below the horizon there and then, as
   `crates/finder/tests/motion/tracking/routes.rs:182` checks it, with an `error`
   containing `M8 is below the horizon`, Track's own words; and, after
   `PUT /api/imaging/settings` has set the `roi` `{"x":0,"y":0,"width":412,"height":274}`,
   the top-left of 033's nine tiles of the simulated sensor, with
   `the imaging camera has a region of interest set: clear it before an imaging run`;
   and, after `Simulator::set_unresponsive(true)` and once the imaging view's `state`
   is no longer `ready`, with `the scope node is not connected`.
   After each the session's `track` is absent or its `track.wanted` is false, the
   imaging view has no `run`,
   R holds no `runs` folder, and, once the region is cleared, `POST /api/imaging/frame`
   answers 200: the lease was given back (oracle: 031's own route). The closing checks
   of the unresponsive case are made after `set_unresponsive(false)`, once the view is
   `ready` again.
3. **Given** a centred run, **When** a test sends `POST /api/stop`, **Then** a committed
   test asserts that the run ended `stopped`, that the script's `calls()` hold
   `stack.stop` once, and that from 2 s after the stop the simulated mount is at rest
   for 10 s (oracle: the simulator's mount).
4. **Given** a centred run, on a rig of its own each, **Then** a committed test asserts
   that:
   - after `PUT /api/session` with `{"min_alt_deg":80.0}`, as
     `crates/finder/tests/motion/tracking/min_alt.rs:67` raises it, the run ends
     `too_low` and the simulated mount is at rest 2 s later;
   - after `Simulator::set_unresponsive(true)`, the run ends `link_lost`;
   - after `PUT /api/session` with `{"target":"M45"}`, the run ends `target_changed`
     and `track.state` is still `tracking` with `track.target.messier` `M42`;
   - after `rig.mount.push_by(0.0, -2.0)` (`crates/simulator/src/mount.rs:269`), down and
     so behind the rising target, while
     the run stacks, the run ends `pushed_away` within 60 s, and `track.state` is still
     `tracking` with `track.push.state` `moving` or `following` (oracle: the session,
     whose `push` is 038's);
   - after `rig.mount.push_by(0.0, -0.1)` once a second while the run stacks, until the
     run ends or 20 times, the run ends `pushed_away` between 10 s and 15 s after the
     first push: a push that keeps reading `moving` ends the run once it has lasted
     `PUSH_GRACE` (10.4 s on the simulator; oracle: the session, whose `push` is 038's);
   - on a centred rig whose clock starts 120 s before the Sun's centre reaches −0.83°
     at the site, found by the test by bisection of `crate::support::goto::sun_truth`
     (`crates/finder/tests/support/goto.rs:91`), with the target M81, a run started at
     once ends `sun_up`, in whatever state it had reached, and the simulated mount is at
     rest 2 s later;
   - on a centred rig, right after a start is answered and while `run.state` is
     `slewing`, `POST /api/imaging/frame` answers 409 with the `error`
     `the imaging camera is held by the imaging run`; and after `POST /api/stop` has
     ended that run it answers 200 (oracle: 031's own route under 031 US9's lease,
     whose refusal names the holder);
   - on a centred rig whose clock starts 15 s before M42's transit at the site, at Unix
     1768528800 + 3242.7, where 036's tracker is not steady around the transit
     (flip.md § Prerequisites 2), a run started at once still
     reads `run.state` `tracking`, with the message
     `Tracking M42. Waiting for the mount to settle; nothing to do.`, 30 s after it
     first read
     `tracking` (oracle: 036's tracker, whose answer the mount part passes on);
   - after `rig.mount.push_by(0.0, -0.1)`, 6′ down, while the run stacks, and
     `rig.mount.push_by(0.0, 0.1)` 3 s later, the session's `track.push.state` read
     `moving` at a sample between the two (the test fails otherwise), and 30 s after
     the first push `run.state` is `stacking` or `paused_by_tracker` and the session's
     `track` has no `push`: a push that springs back within 10 s does not end the run
     (oracle: the session, whose `push` is 038's);
   - on a centred rig, `DELETE /api/imaging/run` sent right after a start is answered,
     while `run.state` is `slewing`, answers a run ended `stopped` with `ended.message`
     `Stopped: the mount was stopped.` and `ended.track_on` false; the session's
     `track.state` is then `off`, and from 2 s after it the simulated mount is at rest
     for 10 s: a stop of the run before anything was exposed also stops the mount
     (oracle: the session and the simulator's mount).
5. **Given** the centred rig and its mount part alone, made by the test with
   `finder::imaging::run::console::mount`, no run being started, **Then** a committed
   test asserts that:
   - `point(By::Page)` answers `Ok(())`, and `state()`, read every 100 ms, is
     `MountState::Tracking { steady: true }` within 900 s;
   - `repoint()` then answers `Ok(())`; `state()` is `Tracking { steady: true }` again
     within 120 s; and the session's `motion` then reads `by` `page` and `tool`
     `imaging_run` (oracle: the session);
   - after `POST /api/stop`, `state()` is `Ended(Ending::Stopped)`; `repoint()` answers
     `Err(Failure::Stopped)`; the session's `track.state` is `off` right after it; and
     from 2 s after the stop the simulated mount is at rest for 10 s: a re-centre asked
     for after a Stop turns nothing on (oracle: the session and the simulator's mount);
   - on a centred rig of its own, after `point(By::Page)` has answered, `state()` has
     read `Tracking { steady: true }` and `PUT /api/session` has set
     `{"target":"M45"}`, `repoint()` answers `Ok(())`, the session's `track.state` is
     `off` right after it and still `off` 10 s later, and `state()` is
     `Ended(Ending::TargetChanged)`: a re-centre does not ask Track for a target the
     run was not started for (oracle: the session);
   - on a centred rig of its own, after `target()` has answered M42 and
     `PUT /api/session` has set `{"target":"M45"}`, `point(By::Page)` answers
     `Err("the target was changed")` and the session's `track` is absent or its
     `track.wanted` is not true: the part asks Track for no target but the one it
     answered (oracle: the session);
   - on a centred rig of its own, after `point(By::Page)` has answered and `state()`
     has read `Tracking { steady: true }`, then `PUT /api/session` with
     `{"target":"M45"}`, `POST /api/track` with `{"on":true}`, answered 200, and
     `PUT /api/session` with `{"target":"M42"}` again, `state()` is
     `Ended(Ending::TargetChanged)`: Track on another target ends the motion whatever
     the session names (oracle: the session);
   - on a centred rig of its own, after `point(By::Page)` has answered, `state()` has
     read `Tracking { steady: true }`, `POST /api/stop` has been sent and, 1 s later,
     `POST /api/track` with `{"on":true}` has answered 200, `state()` is
     `Ended(Ending::Stopped)` 1 s after that while `track.state` is not `off`: Track
     turned on again after a Stop does not bring the run back (oracle: the session);
   - on a centred rig of its own, after `point(By::Page)` has answered, `state()` has
     read `Tracking { steady: true }` and `PUT /api/session` has set
     `{"min_alt_deg":80.0}`, `repoint()` asked 1 s later answers
     `Err(Failure::Mount("M42 is below the minimum altitude"))`, Track's own refusal
     (oracle: Track's words);
   - on a centred rig of its own, after `point(By::Page)` has answered and `state()`
     has read `Tracking { steady: true }`, one imaging frame taken 12 s later by
     `console::camera`'s `hold` and `take`, given back with `release`, and solved by
     `console::solver` gives `near_arcsec` 449″ within 1″, a quarter of the simulated
     sensor's shorter side at its scale (448.9″ on the simulator; oracle: the
     simulator's sensor).
6. **Given** the centred rig, on which `PUT /api/imaging/settings` has set
   `{"exposure_ms":10000}`, **When** a test sends `POST /api/imaging/frame`, sleeps 1 s
   and sends `POST /api/imaging/run` with `{}`, **Then** a committed test asserts 409
   with the `error` `an exposure is in progress`; that the session's `track` is absent
   or its `track.wanted` is false and the imaging view has no `run`; and that, once that
   imaging frame is saved, `POST /api/imaging/frame` answers 200: the camera part took
   the lease, saw the imaging frame in flight and gave the lease back (FR-021; oracle:
   031's own route).

---

### User Story 5 - Centring on the sensor (Priority: P1)

As the observer, I start a run on a telescope whose imaging camera the console has never
related to the finder camera, and the run brings the target to the middle of the imaging
camera's sensor by asking Track again with the aim point it has just measured. If I
press Stop while it does so, Stop still wins.

**Why this priority**: This is where "nothing to tap" is proven.

**Independent Test**: `crates/finder/tests/console/imaging_run_centring.rs`, the module
`imaging_run_centring` of `console`, for the scripted scenarios US5-S1 to US5-S3, and
`crates/finder/tests/motion/imaging_run_centring.rs`, the module `imaging_run_centring`
of `motion`, for US5-S4 to US5-S8 on the run rig. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs both; during
development, `cargo nextest run -p finder --locked --test console imaging_run_centring::`
and `cargo nextest run -p finder --locked --test motion imaging_run_centring::`.

**Acceptance Scenarios**:

1. **Given** a script whose `push_solve` gives
   `Centre { offset_arcsec: 400.0, tolerance_arcsec: 90.0, near_arcsec: 450.0, aim_moved: true }`
   and then the same with `offset_arcsec` 50.0, and which holds `repoint`, **When** a test
   starts a scripted run and settles, **Then** a committed test asserts the state
   `centring`, the message
   `Centring M42: 400″ from the sensor's centre, try 1 of 3.` and `centring`
   `{"tries":1,"offset_arcsec":400.0,"tolerance_arcsec":90.0}`; and, after `repoint` is
   released and the test settles, the state `stacking`, `centring`
   `{"tries":1,"offset_arcsec":50.0,"tolerance_arcsec":90.0}`, and `calls()` after
   `solver.solve 2` equal to `mount.repoint`, `camera.take`, `solver.solve 3`,
   `camera.release`, `stack.start M42`.
2. **Given** a script whose `push_solve` gives an offset of 600″, beyond its
   `near_arcsec` of 450″, with `aim_moved` true four times, **When** a test starts a
   scripted run and settles, **Then** a committed test asserts that the run ended
   `not_centred` with
   `Ended: M42 is still 10.0′ from the sensor's centre after 3 tries. Check the aim point on the console page, then start again.`,
   an offset of 600″ or more being written in arcminutes,
   that `calls()` holds `mount.repoint` three times and `solver.solve` four times, and
   that it holds no `stack.start`.
3. **Given** a script, **Then** a committed test asserts that:
   - with `solve` held, and an offset of 400″ with `aim_moved` false, then 50″: once
     the first solve waits, the test sets the mount to tracking and not steady and
     releases `solve`; after a settle the state is `centring`, `centring.tries` is 0
     and the message is
     `Centring M42: 400″ from the sensor's centre, no new aim point yet. Waiting for the mount to settle; nothing to do.`;
     and after the mount is set steady and a settle the run is `stacking` with
     `centring.tries` 0 and no `mount.repoint` in `calls()`: a solve that measured no
     new aim point is not a try;
   - an offset of 400″, inside the `near_arcsec` of 450″, with `aim_moved` false four
     times reaches `stacking` with `centring.tries` 0, four `solver.solve` and no
     `mount.repoint` in `calls()`, and `warnings`
     `["M42 is 400″ from the sensor's centre and no new aim point could be measured; it is stacked where it is."]`,
     400″ being over the 180″ up to which 036 steers a stack;
     and an offset of 600″ with `aim_moved` false four times ends `not_centred` with
     `Ended: M42 is 10.0′ from the sensor's centre and no new aim point could be measured.`,
     `centring.tries` 0, four `solver.solve`, and neither `mount.repoint` nor
     `stack.start` in `calls()`;
   - an offset of 400″ with `aim_moved` false three times, true once, false three
     times more, and then 50″, reaches `stacking` with `centring.tries` 1, one
     `mount.repoint`, `centring.offset_arcsec` 50, eight `solver.solve` and no
     `warnings`: a re-centre starts the count of such solves again;
   - with `repoint` held, an offset of 400″ with `aim_moved` true, then 400″ with
     `aim_moved` false, then 50″ with `aim_moved` false: the message is
     `Centring M42: 400″ from the sensor's centre, try 1 of 3.` while `repoint` is
     held; and with `solve` held, `repoint` released and a settle, then the mount set
     to tracking and not steady and `solve` released, after a settle the state is
     `centring` and the message is
     `Centring M42: 400″ from the sensor's centre, no new aim point yet. Waiting for the mount to settle; nothing to do.`:
     after a try as before one, a solve that measured nothing reads the second message;
   - an offset of 400″, inside the `near_arcsec` of 450″, with `aim_moved` true four
     times reaches `stacking` with `centring.tries` 3, `mount.repoint` three times in
     `calls()`, and `warnings`
     `["M42 is 400″ from the sensor's centre after 3 tries; it is stacked where it is."]`;
   - an offset of exactly 90″ with `aim_moved` true reaches `stacking` with
     `centring.tries` 0 and no `mount.repoint`; and an offset of exactly 450″, the
     `near_arcsec`, with `aim_moved` true four times reaches `stacking` with `warnings`
     `["M42 is 450″ from the sensor's centre after 3 tries; it is stacked where it is."]`:
     both bounds hold the target that sits on them;
   - an offset of 180″, outside the tolerance of 90″ and not over 036's 180″, with
     `aim_moved` true four times reaches `stacking` with `warnings`
     `["M42 is 180″ from the sensor's centre after 3 tries; the stack's steering will bring it in."]`,
     and an offset of 150″ with `aim_moved` false four times with `warnings`
     `["M42 is 150″ from the sensor's centre and no new aim point could be measured; the stack's steering will bring it in."]`:
     each of the two warnings has two forms, by the offset against the 180″ beyond which
     036 steers no stack (FR-024);
   - an offset of 400″ followed by `fail_repoint(Failure::Mount("the mount is already moving"))`
     ends `mount_failed` with
     `Ended: Track ended and the run with it: the mount is already moving. Check the mount, then start again.`;
   - an offset of 400″ followed by `fail_repoint(Failure::Stopped)` ends `stopped`;
   - after an offset of 400″, with the mount set to tracking and not steady while
     `repoint` is held, no `camera.take` follows `mount.repoint` in `calls()` 10 s
     after `repoint` is released, and one follows once the mount is set steady and the
     test settles;
   - after an offset of 400″, with `repoint` held, a `runner.stop()` awaited beside a
     sleep of 10 s has not answered when the sleep ends; once `repoint` is released it
     answers a view that ended `stopped`, and `calls()` holds no `camera.take` after
     `mount.repoint`: a re-centre is never abandoned half-way, and a stop that arrives
     during one is acted on as soon as it has answered (037's FR-005).
4. **Given** the run rig with the console's own mount, camera and solver parts and the
   script's focus and stack parts, and no aim point, **When** a test sends
   `POST /api/imaging/run` with `{}` and waits until `run.state` is `stacking`, **Then**
   a committed test asserts, against the simulator's truth, that:
   - `run.centring.tries` is at least 1 and `run.centring.tolerance_arcsec` is 90.0,
     the floor of owner decision 2 (1/20 of the simulated sensor's shorter side alone
     gives 89.8″);
   - the true offset, read when the test first sees `stacking`, is at most the
     tolerance plus 20″, which allows the tracker's 10″ deadband twice over
     (`crates/finder/src/tracking.rs:31`);
   - the session's `aim.source` is `measured` and its `aim` is within 2.0 pixels of the
     true aim point in each axis: the 1.5 pixels 032 allows the same oracle with the
     mount at rest (032 US4-S1), and 0.5 pixel more for the tracker's 10″ deadband,
     which is 0.34 pixel at 29.78″ a pixel;
   - sampling `GET /api/session` every 100 ms, the first sample whose `track.state` is
     not `off`, after the first sample that read `off` while its `imaging.run.state`
     was `centring`, came at most 2 s later: the re-centre waits for the finder
     camera's first live pointing with Track off, which came within two polls
     (0.50 s) on the simulator, and no longer (oracle: the session);
   - on a rig of its own, the same run with `Simulator::set_frame_interval` set to
     2 s once the rig is ready reaches `stacking` with `run.centring.tries` 1 and the
     true offset at most the tolerance plus 20″: a re-centre waits for a finder camera
     slower than one poll (5.59″ on the simulator).
5. **Given** the run rig with the console's own mount and camera parts and the script's
   solver, focus and stack parts, the script's `push_solve` given an offset of 400″ with
   `aim_moved` true four times, all such a run can ask for (US5-S2), **When** a test
   sends `POST /api/imaging/run` with
   `{}`, reads `GET /api/imaging` every 100 ms and sends `POST /api/stop` as soon as it
   reads the state `centring`, **Then** a committed test asserts that the run ended
   `stopped` and not `mount_failed`, wherever in the re-centre the Stop landed; that
   the script's `calls()` hold no `stack.start`; and that from 2 s after the stop the
   simulated mount is at rest for 10 s: no re-centre restarts the mount (oracle: the
   simulator's mount).
6. **Given** the rig of US5-S4, on which `PUT /api/imaging/settings` has first set
   `{"exposure_ms":20000}`, **When** a test sends `POST /api/imaging/run` with `{}` and
   waits until `run.state` is `stacking`, **Then** a committed test asserts that
   `run.centring.tries` is at least 1; that the true offset, read when the test first
   sees `stacking`, is at most the tolerance plus 20″; and that at none of the reads
   the wait made did the imaging view's `relation.last.outcome` read `moving`: the
   mount part's look-back is the exposure when that is longer than 032's 10 s, so no
   aim reading's fit reaches into a slew (FR-019; oracle: the simulator's truth and
   032's own view).
7. **Given** the rig of US5-S5, **When** a test sends `POST /api/imaging/run` with
   `{}`, reads `GET /api/imaging` every 100 ms and sends `PUT /api/session` with
   `{"target":"M45"}` as soon as it reads the state `centring`, **Then** a committed
   test asserts that the run ended `target_changed` with `ended.track_on` false; that
   the session's `track.state` is `off` 10 s later; and that from 2 s after the ending
   the simulated mount is at rest for 10 s: the re-centre did not ask Track for the new
   target (oracle: the session and the simulator's mount).
8. **Given** the run rig with the session's aim point set first to the true aim point
   with `Session::set_aim`, and its mount part alone, made by the test with
   `finder::imaging::run::console::mount`, no run being started, **Then** a committed
   test asserts that, after `point(By::Page)` has answered, `state()` has read
   `Tracking { steady: true }` and `Simulator::stop_frames()` has been called,
   `repoint()` answers `Ok(())` between 4.9 s and 5.5 s after it was asked: with no
   live pointing, a re-centre goes on after `FRESH_POLLS` polls (FR-025; 5.000 s on the
   simulator; oracle: the simulator's clock).

---

### User Story 6 - The focus gate and its pause (Priority: P1)

As the observer at a hand focuser, I am stopped before the run wastes a solve on a
picture that is clearly out of focus. The focus aid opens, I turn the knob, and I ask the
run to check again, or to go on anyway.

**Why this priority**: With a manual focuser the first imaging frame of a night is often
far out, and an unfocused imaging frame does not solve.

**Independent Test**: `crates/finder/tests/console/imaging_run_focus.rs`, the module
`imaging_run_focus` of `console`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test console imaging_run_focus::`.

**Acceptance Scenarios**:

1. **Given** a script whose `push_focus` gives `Verdict::ClearlyOut` with reading 9.8 and
   record 3.1 in arcseconds, **When** a test starts a scripted run and settles, **Then**
   a committed test asserts the state `focus_pause`, `focus`
   `{"verdict":"clearly_out","reading":9.8,"record":3.1,"unit":"arcsec"}`, the message
   `Focus is clearly out: stars are 9.8″ across against a record of 3.1″. Turn the focuser while watching the focus aid below, then check focus again or go on anyway.`,
   `pause_ends_at` 1768529700.0, 900 s after the pause began on the console's clock,
   `calls()` ending `camera.take`, `camera.release`, `focus.check`, `focus.open_aid`,
   and no `solver.solve` in it.
2. **Given** the paused run of US6-S1, **When** a test calls `runner.resume(false)` and
   settles, **Then** a committed test asserts that `calls()` goes on with
   `focus.close_aid`, `focus.check`, `camera.hold`, `camera.take`, `solver.solve 2`,
   that the state is `stacking`, and that `pause_ends_at` is absent.
3. **Given** the paused run of US6-S1, **When** a test calls `runner.resume(true)` and
   settles, **Then** a committed test asserts that `calls()` goes on with
   `focus.close_aid`, `camera.hold`, `camera.take`, `solver.solve 2`, with no second
   `focus.check`;
   that the state is `stacking`; that `warnings` is
   `["Focus was not checked again: the observer went on anyway."]`; and that
   `pause_ends_at` is absent.
4. **Given** the paused run of US6-S1, **Then** a committed test asserts that 899.5 s
   after the start the state is still `focus_pause`; that 1 s later the run has ended
   `focus_timeout` with
   `Ended: focus was left paused for 15 minutes. Start again when ready.`; that
   `focus.close_aid` was called once; and that `calls()` holds no `stack.start`.
5. **Given** the paused run of US6-S1, in a run of its own each, **Then** a committed
   test asserts that `runner.stop()` ends it `stopped`, and the mount set to
   `Ended(Ending::TooLow)` ends it `too_low`, each with `focus.close_aid` called once
   before `camera.release` and the ended view with no `pause_ends_at`; that with
   `fail_open_aid(Failure::Other("the focus aid is busy"))` the run still pauses, with
   `warnings` `["The focus aid could not be opened: the focus aid is busy."]`, and a
   `runner.stop()` on that run still calls `focus.close_aid` once, before
   `camera.release`: the aid is closed on every way out of a pause, whether or not it
   could be opened; and that `resume(true)` on a run that is stacking is refused with
   409 and `the imaging run is not paused`.
6. **Given** the scripted console with a run paused as in US6-S1, **When** a test sends
   `POST /api/imaging/run/continue` with `{"anyway":false}`, **Then** a committed test
   asserts 200 and, once settled, `run.state` `stacking`; and that on another such run
   `{"anyway":true}` answers 200 and leaves the warning of US6-S3 in `run.warnings`.
7. **Given** 033's focus rig (033's spec.md § User Scenarios) with the simulated solver
   adapter as its solver, on which the test has first run the focus aid at the
   simulator's focus error 0.0, with `POST /api/imaging/focus` and `{}`, until the
   imaging view's `focus.record.tonight` is present, and stopped it with
   `DELETE /api/imaging/focus`; and the rig's state given
   `with_imaging_run_steps(steps)`, the camera and focus parts of `steps` being the
   console's own and the mount, solver and stack parts the script's, the test taking
   the runner from `AppState::imaging_runner`, **Then** a
   committed test asserts, against the simulator's focus error
   (`Simulator::set_focus_error`, 033 US1), that:
   - at the focus error 0.0 a run reaches `stacking` with `focus.verdict` `good` and
     `focus.reading` at most 1.10 times `focus.record`, 033's bound for good, and
     `focus.unit` `arcsec`;
   - with the focus error raised from 0.0 in steps of 1.0, at most ten of them, until a
     check the test starts itself, with `POST /api/imaging/focus` and
     `{"check":true}`, ends with the verdict `clearly_out` (the test fails if none has
     by the focus error 10.0), a run pauses with `focus.verdict` `clearly_out`, and the
     imaging view's `focus.aid` has `state` `running` and `check` false: the pause
     opened the focus aid;
   - with the focus error then set back to 0.0, `resume(false)` reaches `stacking`, and
     `focus.aid.state` is `ended`;
   - a start while a focus aid the test started is running is refused with 409 and the
     `error` `the imaging camera is held by the focus aid`, the script's `calls()` hold
     no `mount.point` after the refused start, and the imaging view's `focus.aid.state`
     is still `running`
     (oracle: 031 US9's lease, whose refusal names its holder, and 033's name for the
     aid; the camera part asks for the lease before it looks for an imaging frame in
     flight, FR-021);
   - on a focus rig whose R is fresh, so that the camera has no focus record, a run
     pauses with `focus.verdict` `no_record`;
   - on a focus rig built with no imaging solver, and no focus aid run on it, a run
     ends `solver_unavailable` at its focus check, with no `focus_pause` (oracle: 033's
     refusal `this console has no imaging solver`);
   - with a `DELETE /api/imaging/focus` sent, and not waited for, while a run's own
     check runs (the imaging view's `focus.aid.state` `running` and `run.state`
     `focus_check`), the run ends `stopped` with `run.ended.message`
     `Ended: the focus check was stopped.`, the script's `calls()` hold no
     `solver.solve`, and the imaging view's `focus.aid.state` is then `ended`: a check
     stopped from 033's own route ends the run, as a stack stopped from its own route
     does. The test spawns the `DELETE` and does not await it.
8. **Given** a script, in a run of its own each, **Then** a committed test asserts
   that:
   - with `push_focus` giving the `Verdict::ClearlyOut` check of US6-S1 once and
     `unsteady(1)`, a run reaches `stacking` after a settle with `focus.verdict`
     `good`, `focus.check` twice and no `focus.open_aid` in `calls()`, and no
     `warnings`: a check that would pause the run, and during which the tracker was
     not steady, is repeated once with no pause;
   - with that check given twice and `unsteady(2)`, the run pauses after two
     `focus.check` and one `focus.open_aid`, with `warnings`
     `["The tracker was not steady during the focus check."]`;
   - with `unsteady(1)` and the script's default check, which is good, the run reaches
     `stacking` with one `focus.check`: a check that goes on is not repeated;
   - with `push_focus` giving a `Verdict::NoRecord` check with reading 3.2 in
     arcseconds and no record, and `unsteady(1)`, the run pauses after one
     `focus.check`: only a check that is clearly out or unmeasurable is repeated;
   - with the check of US6-S1 given three times and `unsteady(1)`, the run pauses
     after two `focus.check`; after `unsteady(1)` again and a `resume(false)`, it
     reaches `stacking` after four `focus.check` and with no `warnings`: every check
     the run makes after a resume starts the rule again;
   - with the check of US6-S1 once and `unsteady(1)`, the run reaches `stacking` and
     `calls()` holds no `camera.release` between the two `focus.check`: a repeat gives
     the lease up no second time.
9. **Given** the paused run of US6-S1, in a run of its own each, **Then** a committed
   test asserts that:
   - a `resume(true)` called right after a `resume(false)`, with no settle between, is
     refused with 409 and `the imaging run is not paused`, and after a settle
     `calls()` holds a second `focus.check` and the run has no `warnings`: the first
     resume is the one acted on;
   - with `close_aid` held (`Step::CloseAid`), 901 s after the start, when the limit
     has fired and the run waits for the aid to close, `resume(true)` is refused with
     409 and `the imaging run is not paused`, and once `close_aid` is released the run
     has ended `focus_timeout`: a continue that comes too late is not answered yes.
10. **Given** a script whose `push_focus` gives one of these checks, in a run of its own
    each, **When** a test starts a scripted run and settles, **Then** a committed test
    asserts a pause with these messages:
    - `Verdict::ClearlyOut` with reading 21.0 in pixels and no record:
      `Focus is clearly out: stars are 21.0 px across. Turn the focuser while watching the focus aid below, then check focus again or go on anyway.`;
    - `Verdict::Unmeasurable` with no reading:
      `Focus could not be measured: too few stars. Check for cloud or dew, then check focus again or go on anyway.`;
    - `Verdict::NoRecord` with reading 3.2 in arcseconds:
      `There is no focus record for this camera yet. Focus with the focus aid, then check focus again or go on anyway.`;
    - `Err(Failure::NotSolved("the focus check ended: no_region"))`, a check that
      found no stars to measure: the message of `Verdict::Unmeasurable`,
      `focus.verdict` `unmeasurable`, and `warnings`
      `["The focus check did not finish: the focus check ended: no_region."]`;
    - `Err(Failure::Other("the focus check ended: failed: the camera stopped answering"))`:
      `The focus check did not finish: the focus check ended: failed: the camera stopped answering. Check the imaging camera, then check focus again or go on anyway.`,
      `focus.verdict` `unmeasurable`, and `warnings`
      `["The focus check did not finish: the focus check ended: failed: the camera stopped answering."]`:
      a check that failed for another reason does not tell the observer to look
      for cloud;
    - `Err(Failure::Other("the focus check ended: failed: the camera stopped answering"))`
      and then the `Verdict::ClearlyOut` check of US6-S1: after the first pause and a
      `resume(false)`, the run pauses again with the message of US6-S1: each pause
      words its own verdict, not the failure of a check before it.
11. **Given** a script whose `push_focus` gives the `Verdict::ClearlyOut` check of
    US6-S1 twice, **When** a test starts a scripted run, settles, calls
    `runner.resume(false)` and settles, **Then** a committed test asserts that the state
    is `focus_pause` again, that `focus.open_aid` was called twice, and that `calls()`
    holds no `camera.release` between the two `focus.check`: the lease stays given up
    through a pause and a check that is repeated (037's FR-002).

---

### User Story 7 - Stacking to a limit, and the endings a stack can cause (Priority: P1)

As the observer, I ask for fifty subs or twenty minutes and walk away. The run stops the
stack at the limit and saves it. If the camera fails while it stacks, or the sky clouds
over and sub after sub is dropped, the run ends and says why, and what it had stacked is
saved.

**Why this priority**: Without a limit a forgotten run writes subs until a guard or the
disk ends it, and without these endings a stack that has stopped by itself leaves a run
that says it is still stacking.

**Independent Test**: `crates/finder/tests/console/imaging_run_limits.rs`, the module
`imaging_run_limits` of `console`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test console imaging_run_limits::`.

**Acceptance Scenarios**:

1. **Given** a scripted run started with `subs` 3 that is stacking, **When** a test calls
   `set_stack(2, &[("not_solved", 1)], false)` and settles, and then
   `set_stack(3, &[("not_solved", 1)], false)` and settles, **Then** a committed test
   asserts `stacking` after the first, where three subs were taken and two stacked, and,
   after the second, an ending `count_limit` with
   `Finished: the 3 subs asked for are stacked.` and `stack.stop keep` called once:
   a dropped sub does not count toward the number asked for.
2. **Given** a script holding `solve`, and a scripted run started with `duration_s` 60,
   **When** a test waits 100 s, releases `solve`, and settles, **Then** a committed test
   asserts that 58.6 s later the state is still `stacking`, and that 1 s after that the
   run has ended `duration_limit` with `Finished: the 1-minute limit was reached.` and
   `stack.stop keep` called once: the time before stacking does not count.
3. **Given** a scripted run that is stacking, **When** a test calls
   `set_stack(4, &[], false)` and settles, and then calls
   `set_stack(4, &[("behind", 30), ("not_solved", 7), ("too_few_stars", 3)], false)`,
   **Then** a committed test asserts that 598.6 s later the state is still `stacking`,
   and that 1 s after that the run has ended `subs_failing` with
   `Ended: no sub was stacked for 10 minutes (not solved 7, too few stars 3). Cloud, dew or a tree is the usual cause; check the sky and start again.`
   and `stack.stop keep` called once: the rule is time without a stacked sub, the
   reasons are written with spaces, and a sub the stack let go because the console was
   behind is not named; that on a run of its own, after
   `set_stack(4, &[("not_solved", 2)], false)`, a settle and
   `set_stack(4, &[("not_solved", 9)], false)`, the run has ended `subs_failing` 601 s
   later with a message holding `(not solved 7)`: the words give each reason's rise
   since the last stacked sub, not its total; and that on a run of its own, after
   `set_stack(4, &[], false)`, a settle and `set_stack(4, &[("not_solved", 1)], false)`,
   the run has ended `subs_failing` 601 s later: a single failed sub is enough.
4. **Given** a scripted run that is stacking, in a run of its own each, **When** a test
   calls `end_stack(Some(failure))` and settles, **Then** a committed test asserts, for
   `Failure::Camera("no imaging frame header arrived within 152 s")`,
   `Failure::Save("no space left on device")`,
   `Failure::SolverUnavailable("the imaging solver has no star database it can read")`,
   `Failure::NotSolved("the imaging solver found no solution")` and
   `Failure::Other("the stack's grid could not be built")`, the reasons `camera_failed`,
   `save_failed`, `solver_unavailable`, `solver_failing` and `stack_failed`, the last
   with `Ended: the stack stopped: the stack's grid could not be built.`; that after
   `end_stack(None)` and a settle a run has ended `stopped` with
   `Ended: the stack was stopped.`; that each of the six
   runs called `stack.stop keep` once, so that the stack is saved whoever ended it; and
   that a run started with `subs` 3, given `set_stack(3, &[], false)` and then
   `end_stack(Some(Failure::Camera("the camera stopped answering")))` with no settle
   between, has ended `camera_failed` after a settle: a stack's failure is judged
   before the count limit.
5. **Given** a scripted run that is stacking, **When** a test calls
   `end_stack(Some(Failure::Camera("the scope node is not connected")))` and, with no
   settle between, sets the mount to `Ended(Ending::LinkLost)`, and settles, **Then** a
   committed test asserts that the run ended `link_lost` with
   `Ended: the mount or the scope link stopped answering. Check the scope node and the mount's cable, then start again.`
   and `stack.stop keep` called once: the mount's
   ending is judged before the stack's failure, so a lost link that both have seen
   reads `link_lost`.
6. **Given** a scripted run that is stacking, after `set_stack(4, &[], false)` and a
   settle, in a run of its own each, **Then** a committed test asserts that:
   - 700 s after `set_stack(4, &[("behind", 30), ("unprocessed", 2)], false)` the state
     is still `stacking`: neither reason is a failed sub;
   - 700 s after `set_stack(4, &[("not_solved", 3)], true)` the state is still
     `paused_by_tracker`; and after `set_stack(4, &[("not_solved", 3)], false)` it is
     `stacking` 598.6 s later and has ended `subs_failing` 1 s after that: time in
     which the tracker pauses the stack does not count.
7. **Given** a scripted run started with `subs` 3 and `keep_subs` false that is
   stacking, **When** a test calls `set_stack(3, &[("not_solved", 1)], false)` and
   settles, **Then** a committed test asserts an ending `count_limit`,
   `stack.stop discard` called once and never `stack.stop keep`.
8. **Given** a script holding `solve`, and a scripted run started with `duration_s` 60,
   **When** a test waits 100 s, releases `solve`, calls `set_stack(0, &[], true)` right
   after and settles, **Then** a committed test asserts that 58.6 s later the state is
   `paused_by_tracker`, and that 1 s after that the run has ended `duration_limit`: the
   time the tracker pauses the stack does count.
9. **Given** a scripted run that is stacking, **When** a test calls
   `set_stack(4, &[], false)` and settles, calls
   `set_stack(4, &[("not_solved", 9)], false)` and waits 400 s, calls
   `set_stack(5, &[("not_solved", 9)], false)` and settles, and calls
   `set_stack(5, &[("not_solved", 18)], false)` and waits 400 s, **Then** a committed
   test asserts that the state is still `stacking`: a stacked sub starts the time again;
   and that on a run of its own, after `set_stack(4, &[("not_solved", 9)], false)`, a
   settle and `set_stack(5, &[("not_solved", 9)], false)`, the state is still
   `stacking` 700 s later: a stacked sub keeps the drops it saw, so they end nothing
   later.

---

### User Story 8 - The whole run on the console's own parts, and in the binary (Priority: P1)

As the observer, I ask for the Orion Nebula and three subs, and the console I started
from the command line does all of it with its own five parts: it tracks, checks focus,
centres, stacks, stops at three, and leaves the stack and the run's record in one
folder, with Track still on.

**Why this priority**: This story joins the last part, the stack, and makes the binary
run the whole procedure.

**Independent Test**: `crates/finder/tests/motion/imaging_run_stack.rs`, the module
`imaging_run_stack` of `motion`, for US8-S1, US8-S2 and US8-S4 to US8-S8 on the run rig
and for US8-S9, which
reads four source files, and
`crates/finder/tests/cli/imaging_run.rs`, the module `imaging_run` of `cli`, for US8-S3.
The `test` gate's `cargo nextest run --workspace --all-targets --all-features --locked`
runs both; during development,
`cargo nextest run -p finder --locked --test motion imaging_run_stack::` and
`cargo nextest run -p finder --locked --test cli imaging_run::`.

**Acceptance Scenarios**:

1. **Given** the run rig built with `with_imaging_run()`, so that every part is the
   console's own, on which the test has first run 033's focus aid until the imaging
   view's `focus.record.tonight` is present, and stopped it, **When** a test sends
   `POST /api/imaging/run` with
   `{"subs":3}` and waits until `run.state` is `ended`, **Then** a committed test
   asserts that:
   - `run.ended.reason` is `count_limit`, `run.ended.message` is
     `Finished: the 3 subs asked for are stacked.`, `run.ended.saved` is true and
     `run.subs.stacked` is at least 3;
   - `run.folder` is `runs/` followed by the imaging view's `stack.id`, which ends
     `-M42`, and R joined with it holds `run.json`, whose `ended.reason` is
     `count_limit`, and 035's `stack.json`, `stack.fits` and `stack.png`, and no
     `.reserve`;
   - `run.subs.stacked`, the `subs.stacked` of `run.json` and the `stacked` of
     `stack.json` are equal, and equal to the number of subs `stack.json` lists with
     the `outcome` `stacked`; and the sum of `run.subs.dropped` equals the sum of
     `stack.json`'s `dropped`: the counts are read again after the stack is stopped;
   - that `stack.json` lists at least three subs with the `outcome` `stacked`, and the
     `file` of each, a path inside the run's folder, exists there;
   - 2 s after the ending the session's `track.state` is `tracking` with
     `track.target.messier` `M42`, and `run.ended.track_on` is true: the run stopped
     nothing (oracle: the session).
2. **Given** the same rig, **When** a test sends `{"subs":2,"keep_subs":false}` and
   waits until the run has ended, **Then** a committed test asserts `count_limit`, a
   `stack.fits` in the run's folder, and that no `file` that its `stack.json` lists
   exists in the run's folder any more, nor that file's name with `.fits` made
   `.solve.json`.
3. **Given** the built `finder` binary run with `--simulate`, `--state S` and `--listen`
   on port 0 of `127.0.0.1`, S a fresh scratch folder, **When** a test reads
   `GET /api/imaging` every 200 ms, for at most 60 s, until its `state` is `ready`, and
   then sends `POST /api/imaging/run` with `{}`, no target being set, **Then** a
   committed test asserts 409 with the `error` `no target is selected`: the binary runs
   imaging runs, or the answer would be `this console runs no imaging runs`.
4. **Given** the rig of US8-S1 with a run that is stacking and whose imaging view's
   `stack.stacked` is at least 1, **When** a test sends `DELETE /api/imaging/stack`,
   035's own route, **Then** a committed test asserts that the run ends `stopped` and
   that no reason in `run.subs.dropped` has a count of 0 (oracle: 035's route; the Edge
   Case of a stack stopped from its own route).
5. **Given** the rig of US8-S1 whose R already holds a regular file named `runs`,
   **When** a test starts a run with `{}` and waits until it has ended, **Then** a
   committed test asserts `ended.reason` `save_failed` and an `ended.message` beginning
   `Ended: the console could not save to its imaging folder: cannot make the stack folder`.
6. **Given** the rig of US8-S1 with a run that is stacking, **When** a test calls
   `Simulator::set_unresponsive(true)` and waits until the run has ended, **Then** a
   committed test asserts that `run.ended.reason` is `link_lost` or `camera_failed`,
   and that with `camera_failed` its `run.ended.message` holds
   `the scope node is not connected`: with the console's own stack, either the tracker
   or the stack sees a lost link first (oracle: the session's `track.ended_reason` and
   035's `stack.error`).
7. **Given** the rig of US8-S1 with a run that is stacking and whose `run.folder` is
   present, so that the stack part's start has answered, **When** a test reads that
   folder and then sends `DELETE /api/imaging/run`, **Then** a committed test asserts
   that the folder held a file `.reserve` whose size in bytes was 15 × W × H +
   16777216, W and H being the imaging view's `camera.width` and `camera.height`, each
   divided by `settings.binning` and rounded up, which is 32017096 on this rig's 1236 ×
   822 at binning 1; and that the folder holds none once the run has ended, beside its
   `stack.json` (oracle: the imaging view, and 035's files of FR-033); and that, in a
   plain `#[test]` of its own,
   `finder::imaging::run::console::reserve_bytes(4944, 3284, 1)` is 260318656,
   `reserve_bytes(4944, 3284, 3)` is 43845616 (15 × 1648 × 1095 + 16777216; rounded
   down it would be 43820896), and `reserve_bytes(1236, 822, 0)` is 32017096: a binned
   side is rounded up, and a binning of 0 counts as 1 (oracle: the arithmetic typed
   into the test).
8. **Given** the rig of US8-S1 with a run that is stacking, **When** a test sends
   `DELETE /api/imaging/run`, **Then** a committed test asserts `stopped`, the imaging
   view's `stack.state` `ended`, a `run.json` in the run's folder, `track.state` still
   `tracking`, and that `POST /api/imaging/frame` then answers 200.
9. **Given** `src/imaging/run.rs`, `src/imaging/run/script.rs`,
   `src/imaging/run/store.rs` and `src/imaging/run/console.rs`, read by the test from
   `CARGO_MANIFEST_DIR`, **Then** a committed test asserts that none contains
   `crate::goto`, `crate::tracking`, `crate::ephemeris`, `scope_link` or `unsafe`, that
   only `console.rs` contains `crate::model`, and that every line of it that does
   contains `separation`: the run's code reaches the mount through Track alone (037's FR-001,
   SC-006).

---


### Edge Cases

- **The observer turns Track off, or presses Stop, during a run.** Both are the
  console's Stop; the run ends `stopped` (US4-S3).
- **An MCP client calls `set_tracking` or `goto_target` during a run.** `set_tracking`
  on for the same target changes nothing; off is Stop. `goto_target` is refused with
  `Track is on`, as it is today (`crates/finder/src/api.rs:613`), but for the seconds
  in which a re-centre has Track off (below).
- **Someone asks for an imaging frame, the focus aid, a set of darks or flats, or a
  stack during a run.** Each is refused. The lease refuses with the name of whoever
  holds the camera then, in 031 US9's words `the imaging camera is held by` and the
  name: the imaging run, the focus aid or the stack (US4-S4, US6-S7). The starts of
  033, 034 and 035, as drafted, look for an imaging frame in flight only while nobody
  holds the lease, so they too answer with the holder's name while the holder is
  exposing, and never `an exposure is in progress`.
- **A run is asked for while one of those holds the camera.** The start is refused with
  the lease's words, which name the holder, before Track is asked, even while the
  holder's own imaging frame is exposing: the camera part asks for the lease before it
  looks for an imaging frame in flight (FR-021, US6-S7).
- **A run is asked for while a single imaging frame, taken under no lease, is in
  flight.** The camera part takes the lease, sees the imaging frame, gives the lease
  back and refuses with `an exposure is in progress` (FR-021, US4-S6).
- **The stack is stopped from its own route or tool during a run.** The run sees a stack
  that no longer runs and ends `stopped`, saying `Ended: the stack was stopped.`
  (US7-S4, US8-S4).
- **Stop lands while the run is re-centring.** The run asks for Stop once more and ends
  `stopped`, never `mount_failed`, whatever Track answered meanwhile; the mount stays
  at rest (FR-020, US5-S5).
- **The target is changed while the run re-centres.** The re-centre does not ask Track
  again; the run ends `target_changed` with Track off (FR-023, US4-S5, US5-S7).
- **Another motion is started while the run re-centres.** Track is off for some
  seconds then, so `goto_target` is not refused; Track's start finds the mount moving
  and the run ends `mount_failed` with Track's words.
- **The telescope is bumped.** One live pointing more than 5′ off makes 038's `push`
  read `moving`; a pointing back within 5′ clears it. The run ends `pushed_away` only
  when the push reads `following`, or has read `moving` for 10 s; until then the
  mount part reads not steady (FR-023, US4-S4).
- **The aim point was tapped, or measured on an earlier night.** The first imaging solve
  replaces or refines it (032). Centring starts from whatever Track brought to the aim
  point in force; with a good aim point it needs no try (US4-S1).
- **The target is outside the imaging frame at the first solve.** The offset is the
  separation of the solve's centre from the target, which needs no pixel of the target;
  a wide first search is 032's.
- **An imaging solve that does not move the aim point**: 032 took no aim reading from it
  (`moving`, `no_live_pointing`, `off_frame`, `set_aside`), or took one whose median
  lies within 10″ of the aim point in force and left that alone, and 032 holds no
  measured aim point back. A re-centre would bring the same aim point back, so the run
  takes another imaging frame instead. It does not count as a try. After four in a row
  the run stacks with a warning when the
  target is within a quarter of the imaging frame's shorter side of the centre, and
  otherwise ends `not_centred`, saying that no new aim point could be measured (FR-024,
  US5-S3). The first run after the finder or the imaging camera was re-seated meets
  this: 032 sets its first two readings aside. So does an imaging exposure shorter
  than the finder camera's, with no finder solve inside it. An aim point
  032 holds back while Track is on, which is where every aim reading of a run goes, does
  move it: the re-centre's Track off puts it in force (FR-022; 032's FR-036).
- **No finder live pointing arrives after the re-centre's Track off.** After 5 s the
  run asks Track anyway (US5-S8), and Track decides with the pointing it has, taken at
  the old aim point; an aim point 032 held back stays held under the new Track, so the
  next solve asks for another re-centre, and at worst a try is spent. A finder camera
  that gives a live pointing less often than every 5 s is already outside what 015's
  tracker is built for (`crates/finder/src/tracking.rs:248`).
- **A newly measured aim point while Track is on.** 032 holds it back until Track is
  off, so the tracker never chases it and, since 038, it is never read as a push by
  hand that marks the pointing model's slip (032's FR-036; 032's owner decision 25, which
  037's owner decision 26 cites). The re-centre's Track off puts it in force, and the live
  pointing the re-centre then waits for is the first taken at it (FR-025).
- **Track is still stopping when the re-centre asks it again.** GoTo refuses with
  `the mount is already moving` while an axis still reports motion
  (`crates/finder/src/goto.rs:739`), and the run ends `mount_failed` with those words.
  Track itself waits up to 2 s for both axes to rest whenever it replaces a tracker,
  before it asks GoTo. Whether the real mount can be slower than that to stop is not
  established (plan.md § Risks).
- **A focus pause while tracking.** Track stays on through the pause, so the target
  stays in the focus aid's view; Track's guards can still end the run (US6-S5).
- **The focus check is stopped from 033's own route or tool during a run.** The run
  ends `stopped`, saying `Ended: the focus check was stopped.`, as it does when the
  stack is stopped from its own route (FR-029, US6-S7).
- **The tracker was not steady during a focus check** whose verdict would pause the
  run. Trailed stars read as out of focus, so the check is repeated once before the
  observer is told to turn the focuser; a second unsteady one pauses with a warning
  (FR-026, US6-S8).
- **A continue arrives twice, or as the pause ends.** The first resume is the one
  acted on; a second, and one that arrives once a stop, the mount's ending or the
  15-minute limit has ended the pause, is refused with `the imaging run is not paused`
  (FR-027, US6-S9).
- **The target is plainly in the frame but not centred**, after three tries or after
  four solves in a row that measured no new aim point. Within a quarter of the imaging
  frame's shorter side of the centre, the run stacks with a warning (FR-024, US5-S3). Up
  to 180″ from the centre 036's steering then brings the target in over the following
  subs, slowly where an axis would have to back up, and the warning says so; beyond 180″
  036 does not steer that stack, and the warning says that the target is stacked where
  it is.
- **A run started while Track is already on the target.** Track answers at once, and
  the run takes its first imaging frame as soon as the tracker has been steady for the
  whole of FR-019's look-back, which it already has when Track was left alone that
  long.
- **No matching master dark or flat.** The run does not look: 035's stack says what is
  missing in its own `stack` object, beside `run` in the same view.
- **The kept subs fill the disk.** The first failed save ends the run `save_failed`; the
  default time limit of 037's owner decision 8 is what keeps a forgotten run from getting
  there, and the start form says how much the subs will take. The stack part holds back,
  in the stack's folder, as much room as the stacked picture's files can take for the
  camera's frame, and frees it just before it stops the stack, so the ending's own save
  has room (FR-033, US8-S7); an ending whose stack could not
  be saved says so and where the subs are (042's FR-035).
- **No sub stacks for ten minutes** (cloud, dew, a branch). The run ends
  `subs_failing` once 600 s of stacking have passed with a failed sub and none
  stacked; a pause by the tracker does not count, and neither does a sub the console
  was too far behind to process (FR-031, US7-S3, US7-S6, US7-S9). A single exposure longer
  than the ten minutes does not end the run: with no failed sub there is nothing to
  end it on.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-019**: The console's own mount part SHALL say the session's target, or refuse with
  `no target is selected`, and SHALL point by calling the function the Track route
  calls, passing its refusals on unchanged. It SHALL report `slewing` while Track is
  `armed`; tracking, with the tracker's own answer to "steady" (040), held for the
  whole of 032's fit reach before now, in every other state of Track but `off`; and an
  ending with the reason of plan.md § The console's mount part when Track is off. The
  reach is the larger of 032's 10 s and the imaging camera's exposure setting: an aim
  reading fits the finder camera's solves from that far before its exposure, and a
  fit that reaches into a GoTo or a re-centre takes no aim reading. The part SHALL
  carry out the console's Stop when asked, say whether Track is on, and pass on the
  tracker's answer to whether it was steady over a past window.
- **FR-020**: The console's mount part SHALL point again by turning Track off with
  `Session::stop_tracking` and calling the function of FR-019 once more under
  `start_motion`. It SHALL remember the stop generation of `GoTo::stops` when it points,
  and report the ending `stopped` whenever that generation has changed. Pointing again
  SHALL be refused as stopped when it has changed beforehand. When it has changed by
  the time Track has answered, whatever Track answered, the part SHALL carry out the
  console's Stop once more and answer that it was stopped; only with the generation
  unchanged SHALL an error from Track be a failure of the mount. Track read `off` with
  the reason `stopped` SHALL be the ending `stopped`, except between the part's own
  Track off and Track's answer.
- **FR-021**: The console's own camera part SHALL refuse a start with 031's words while
  the imaging camera is not ready. It SHALL take 031's lease of the imaging camera
  (031 US9) as `the imaging run`, passing a refusal's words on unchanged, before it
  looks at anything else of the camera (037's owner approval 2). Holding the lease, it SHALL refuse with
  `an exposure is in progress` while an imaging frame is in flight, and with
  `the imaging camera has a region of interest set: clear it before an imaging run`
  while the settings hold one (owner decision 24), giving the lease back each time; and
  it SHALL give the lease back when asked. A take SHALL ask 031, as the holder of the
  lease, for one imaging frame with the purpose `run` and wait for it: `saved` answers
  its id and file; an error beginning `could not save the imaging frame` is a save
  failure; any other failure, a refused take, a take answered with no frame and an
  aborted exposure are camera failures.
- **FR-022**: The console's own solver part SHALL refuse a start with
  `this console has no imaging solver` when 032 has none. A solve SHALL ask 032 to solve
  the imaging frame and wait for it. Solved, it SHALL answer the separation of the
  solve's centre from the target in arcseconds; the tolerance, the larger of 1/20 of the
  imaging frame's shorter side times the solve's scale and 90″ (owner decision 2); and
  whether the solve moved the aim point: whether, after the solve, the session's aim
  point is another than it was before it, or the session holds a measured aim point
  back while Track is on (032's FR-036), which the re-centre's Track off puts in force
  (037's owner decision 26). Failed with 032's reason `no_database`, or
  `failed`, it is a solver that cannot run, unless the error begins
  `could not save the imaging solve`, which is a save failure; with any other reason,
  no solution.
- **FR-023**: The console's mount part SHALL report the ending `target_changed` when the
  session's target is no longer the one the run started with (owner decision 12), and
  the ending `pushed_away` when the session's `track` view carries 038's `push` that
  is `following`, or that has stayed `moving` for 10 s; a `moving` push younger than
  that SHALL read as tracking and not steady (owner decision 27). The part SHALL read
  the target once, when the run asks for it, and refuse to ask Track when the
  session's target is no longer that one. A target changed while a re-centre has
  Track off SHALL leave Track off: the mount part SHALL not ask Track for the new
  target.
- **FR-024**: After a solve the run SHALL keep its offset and tolerance in `centring`.
  Outside the tolerance, when the solve moved the aim point as FR-022 tests it, it
  SHALL count a try, show `centring`, ask the mount part to point again, wait until
  the mount tracks and is steady, take another imaging frame and solve it. When the
  solve moved no aim point it SHALL count no try and ask for no re-centre, and
  take and solve another imaging frame, at most four such solves in a row. After three
  tries without success, and after the fourth such solve in a row, it SHALL start the
  stack with a warning when the target is within a quarter of the imaging frame's
  shorter side of the sensor's centre, and SHALL otherwise end `not_centred` without
  starting the stack, saying after the fourth such solve that no new aim point could be
  measured (owner decisions 3 and 4). The warning SHALL say that the stack's steering
  will bring the target in when the offset is at most 180″, the bound beyond which 036
  steers no stack (036 FR-014), and that the target is stacked where it is when the
  offset is over it.
- **FR-025**: After it turns Track off and before it asks Track again, the mount part's
  pointing again SHALL wait, with Track off, for a live pointing newer than the one it
  finds once Track is off: at most twenty polls of 250 ms, 5 s. That live pointing is
  the first the session records with Track off, which puts a measured aim point 032
  held back in force and is taken at it (032's FR-036, 037's owner decision 26). It SHALL be
  refused as stopped when the stop generation changes during the wait, and SHALL go on
  to ask Track when no live pointing has come.
- **FR-026**: A focus verdict of clearly out, unmeasurable or no record, and a focus
  check that could not finish, SHALL put the run in `focus_pause`, open the focus aid
  through the focus part, and say so in the words of 037's plan.md § The run's words (owner
  decision 21). A focus aid that cannot be opened SHALL add a warning and leave the run
  paused. Before it pauses for a verdict of clearly out or unmeasurable, the run SHALL
  ask the mount part whether the tracker was steady for the whole of that check; when
  it was not, the check SHALL be repeated once with no pause, and a repeat that was not
  steady either SHALL pause with the warning
  `The tracker was not steady during the focus check.` A check that could not finish
  SHALL pause with the words of unmeasurable only when it found no stars to measure, and
  otherwise with words that say that it did not finish, why, and to check the imaging
  camera.
- **FR-027**: `Runner::resume` SHALL be refused outside `focus_pause`. In it, with
  `anyway` false the focus check SHALL be repeated, and with `anyway` true the run SHALL
  go on to the solve unchecked with the warning
  `Focus was not checked again: the observer went on anyway.` (owner decision 5).
  A pause SHALL take resumes from the moment the state is `focus_pause`. The first
  resume SHALL be the one acted on; a later one, and one that arrives after a stop,
  the mount's ending or the limit has ended the pause, SHALL be refused as outside a
  pause.
- **FR-028**: A focus pause 900 s old SHALL end the run `focus_timeout`. Every way out of
  a pause, a resume or an ending, SHALL close the focus aid through the focus part
  first, whether or not the aid could be opened (owner decision 6). While paused the
  view SHALL hold `pause_ends_at`, the console's time at which the pause ends by
  itself.
- **FR-029**: The console's own focus part SHALL check focus with 033's focus check and
  answer its verdict, its focus reading and the focus record it was judged against, in
  the unit 033 shows; SHALL answer a check that 033 ended as `stopped` as stopped, so
  that the run ends `stopped` when somebody stops its check from 033's own route;
  SHALL answer a check that ended any other way than with a verdict as one that could
  not finish, with 033's reason, telling a check that found no stars to measure (033's
  `no_region`) from every other; and SHALL open and close 033's focus
  aid, which also stops a check that is running.
- **FR-030**: While stacking the run SHALL end `count_limit` when the subs stacked reach
  the request's `subs`, and `duration_limit` when the request's `duration_s`, 7200 s
  when the request gave none, have passed on the clock since the stack started. Time in
  which the tracker pauses the stack SHALL count; time before the stack started SHALL
  not (037's owner decision 8).
- **FR-031**: While stacking, when 600 s of the run's clock have passed since a sub was
  last stacked, or since stacking began, and at least one sub was dropped in them for
  a reason other than `behind` and `unprocessed`, the run SHALL end `subs_failing`,
  naming each such reason that rose and by how much, with a space for each
  underscore. Time in which the tracker pauses the stack SHALL not count. `behind`
  and `unprocessed` say that the console fell behind and nothing about a sub. The
  600 s SHALL be a constant: no request key and no page field sets it (037's owner decision 9).
- **FR-032**: A stack that reports a failure SHALL end the run `camera_failed`,
  `save_failed`, `solver_unavailable` or `solver_failing` by the failure's kind, and
  `stack_failed` otherwise; a stack that stops with none SHALL end it `stopped`, saying
  that the stack was stopped. In each case the run SHALL still ask the stack to stop,
  which saves it. At each poll the mount's ending and a stop of the run SHALL be
  judged before anything the stack reports, so that a lost link, which also fails the
  stack's next take, ends the run `link_lost` when Track has seen it.
- **FR-033**: The console's own stack part SHALL tell 036's steering that the set point
  is the sensor's centre and start 035's stack for the run's target, answering the
  stack's folder; SHALL answer its counts, with the reasons whose count is above 0,
  whether the steering is waiting, for the tracker or for its exposure limit to rise
  (040 FR-022), and why it ended by itself; and
  SHALL stop it, which saves it. With `keep_subs` false, a saved stack of at least one
  sub and a stack the run itself stopped, it SHALL then
  remove each sub's file and solve sidecar that `stack.json` lists; otherwise it SHALL
  remove none (037's owner decision 22).
  It SHALL stop only the stack it started: a stack another client has started since is
  left running, the run keeps the counts it last read, and no sub is removed. A start
  that 035 answers with a stack somebody else had already started SHALL be a failure,
  and the part SHALL keep nothing of that stack. When its own stack has started the part
  SHALL write a file `.reserve` in the stack's folder, as large as the most 035 writes
  there when the stack ends: 15 bytes for each pixel of a sub, twelve for `stack.fits`
  and three for `stack.png`, the pixels being the imaging view's `camera.width` and
  `camera.height`, each divided by `settings.binning` and rounded up, plus 16 MiB for
  the two files' headers and framing, `stack.json` and `run.json`. That is about 260 MB
  at the worked camera's full size. The part SHALL write it 4 MiB at a time, letting the
  console's other tasks run between two writes, and SHALL remove it just before it stops
  the stack, so that the stack's save has that room. A part that cannot write it SHALL
  remove what it wrote, stop the stack it has just started and fail the start as a
  failure to save (037's owner decision 7, US8-S7).
- **FR-034**: `AppState::with_imaging_run` SHALL give the console its own five parts,
  and the `finder` binary SHALL call it after 032's imaging solver is set, under
  `--simulate` too.

### Key Entities

- **Part**: one of the five things a run asks: the mount, the camera, the focus check,
  the solver, the stack.
- **Lease**: 031's one hold on the imaging camera (031 US9), which a run takes for its
  own imaging frames and hands over to the focus check and the stack (the plan
  document, amendment 1).
- **Set point**: where on the sensor the run wants the target, today its centre (036's
  glossary term).


## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario has a committed test that asserts it, and the
  `lint` and `test` gates pass.
- **SC-002**: No existing test, golden, `scripts/ci/` file, `.github/` file, `CODEOWNERS`
  entry, `ergane.yaml` or `playwright.config.ts` is edited, 037's test files among the
  existing tests. The only edits to existing files under `crates/finder/tests` are one
  `mod` line for each new test module, in alphabetical order, in its binary's `main.rs`:
  `imaging_run_centring`, `imaging_run_focus` and `imaging_run_limits` in
  `crates/finder/tests/console/main.rs`; `imaging_run`, `imaging_run_centring` and
  `imaging_run_stack` in `crates/finder/tests/motion/main.rs`; and `imaging_run` in
  `crates/finder/tests/cli/main.rs`. No other line of those files and no other existing
  test changes.
- **SC-003**: No `Cargo.toml`, `Cargo.lock` or `deny.toml` changes, no scope-link message,
  config key, console flag or glossary term is added, and no `unsafe` is written.
- **SC-004**: On the simulator, a run started with no aim point reaches stacking with the
  target within the centring tolerance plus 20″ of the sensor's centre (US5-S4).
- **SC-005**: On the simulator, after Stop the mount is at rest and stays at rest,
  whatever the run was doing (US4-S3, US5-S5), and no ending but a guard's, Stop's, a
  stop of the run while it still slews, a re-centre that Track refused or a target
  changed during a re-centre changes what Track is doing (US8-S1, US8-S8, US4-S4,
  US5-S7).
- **SC-006**: No file of this spec under `crates/finder/src/imaging/` holds an
  identifier, an import or arithmetic for an axis, an altitude, an azimuth, a rate or
  axis counts, and none holds a focal length, a field size or a scale as a number. The
  rule is about code: the words an observer reads and the doc comments are exempt, so
  the `too_low` message may name the minimum altitude and a comment may say that
  nothing here knows an axis (037's FR-001; plan.md § Plan traps 18). US8-S9 reads the
  four files for what they import.
- **SC-007**: Every story's diff, measured as plan.md § Plan traps 3 says, is under
  64 KiB.


## Assumptions

- **Earlier features are in place.** Every spec this one names in `depends_on_landed` has
  landed before it is built, 037 among them (flip.md § Prerequisites 1). Every
  `path:line` here was read at 49bb210. A file that 030 to 037 create is named without a
  line, with the spec that creates it.
- **Out of scope:**
  - an equatorial mount, meridian flips, dithering and any tracker but 015's as 036
    and 040 leave it; the run is written so that none of them needs a change here;
  - autofocus, and any change to 033's verdict or its thresholds;
  - choosing the exposure, the gain or the binning: they are the imaging camera's
    settings (031), capped by 036 for a stack's subs only; capping the run's own
    imaging frames too, which needs a getter 036 does not offer;
  - any change to GoTo, Track, Stop or their guards, and any new mount command; what
    the tracker does with a newly measured aim point is 032's and 015's, and the owner
    settled it as 032's owner decision 25: 032 holds it back while Track is on (owner
    decision 26 of 037);
  - the lease itself, and how 033 to 035 take it: they are 031's and theirs;
  - a free-space check (037's owner decision 7);
- **Outside the factory:** the field checks at the telescope, which follow the checklist
  of 042's FR-041 (flip.md § Field checks). GoTo and Track must be proven on the real mount
  before any of it (the plan document, § Before any flip, 4).
- **The factory's gates and CI:** the gates are `lint` and `test`, copied in plan.md
  § Verification. CI's other jobs run only in the merge queue, so no scenario's Then
  clause rests on them.


## Work Graph

```yaml
US4:
  depends_on: []
  implements: [FR-019, FR-020, FR-021, FR-022, FR-023]
US5:
  depends_on: []
  depends_on_merged: [US4]
  implements: [FR-024, FR-025]
US6:
  depends_on: []
  depends_on_merged: [US5]
  implements: [FR-026, FR-027, FR-028, FR-029]
US7:
  depends_on: []
  depends_on_merged: [US6]
  implements: [FR-030, FR-031, FR-032]
US8:
  depends_on: []
  depends_on_merged: [US5, US6, US7]
  implements: [FR-033, FR-034]
```

Every edge is a merge edge, and the stories run one after another, after the whole of
`037-imaging-run` has landed. An edge names each earlier story of this spec that last
wrote a file the story writes next, so that the later story builds on the merged file; a
file 037 last wrote is on `main` when this spec starts, so no edge names 037.

- **US4** writes the console's own mount, camera and solver parts in
  `crates/finder/src/imaging/run/console.rs` and drives them through 037's routes; it
  adds one line to 037's `run.rs`. It names no story.
- **US5** adds the centring loop to the machine and the wait for a live pointing to the
  mount part of US4, and is the first story to ask for a re-centre. It names US4 for
  `run.rs`, `console.rs` and `crates/finder/tests/motion/main.rs`; it is this spec's
  first story to write `crates/finder/tests/console/main.rs`, which 037 last wrote.
- **US6** adds the pause to the machine and the focus part to `console.rs`, after US5,
  and uses 037's `continue` route and US4's camera part.
- **US7** adds the limits and the stack's own endings to the machine, after US6.
- **US8** adds the stack part, assembles the five parts and wires the binary. Its whole
  run needs every earlier story: it names US7 for the count limit its scenarios end on,
  US6 for `console.rs` and US5 for `crates/finder/tests/motion/main.rs`, and it writes
  `crates/finder/src/api.rs` next after 037's US3.

042's US9 to US11, in `042-imaging-run-page-and-mcp`, build on this spec once it has
landed.
