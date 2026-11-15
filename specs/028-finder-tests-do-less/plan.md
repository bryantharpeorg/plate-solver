# Implementation Plan: Finder's Tests Do Less Work

**Branch**: `028-finder-tests-do-less` | **Date**: 2026-11-15 | **Spec**: [spec.md](spec.md)

This plan records every design decision for the feature. A story that would change one
escalates rather than re-deciding it.

## Summary

Each story removes work a test does without being checked, and adds the assertion that
shows if the work comes back.

- **US1, the camera off:** six MCP direct-move tests stop the camera after their first
  sync, one MCP Track test starts centred, and the simulator crate's Stop and watchdog rig
  runs on the synthetic camera without the pattern database.
- **US2, centred starts:** nine tests whose claim is Track's refusals, its Stop or a later
  GoTo start with M42 in the eyepiece instead of reaching it through a GoTo.
- **US3, cheaper GoTo setups:** the five-target test loads the far sync of its two-sync
  model from a state file; the cable-wind test samples its 350° move every 2 s.
- **US4, the whole night at 2 frames a second:** a new `Simulator::set_frame_interval`.
- **US5, Track's restart in paused time:** the tracking happens in process; the built
  binary is started once.
- **US6, the binary only for `main.rs`:** the node-settings restart becomes an in-process
  test, and the backlash restart test takes over the fixed reset.
- **US7, previews and the unsolved interval:** two console nodes at slower intervals.
- **US8, hazards:** no wall-clock bound in the hour of tracking, a solve deadline that
  cannot fire in rigs, a real-time check that excludes solve latency, a child that cannot
  hang the gate, and paused clocks for three unit tests.
- **US9, scratch folders:** one helper under `CARGO_TARGET_TMPDIR`, removed on drop.
- **US10, duplicates:** the deletions and reductions of thirteen findings, as one story.
- **US11 to US13, optional trims** of landed scenarios, each a leaf of the Work Graph.

## Technical Context

**Language/Version**: Rust 2021, in test files, four test modules of product source files,
and one method of the `simulator` crate.

**Primary Dependencies**: nothing new. `Cargo.toml` and `Cargo.lock` are not edited.
`tokio`'s `test-util` feature, which `start_paused` needs, is already a dev-dependency of
`scope-node` (`crates/scope-node/Cargo.toml:32`). `CARGO_TARGET_TMPDIR` is set by cargo for
every integration test.

**Storage**: nothing.

**Testing**: every story edits or adds tests; nothing else is the product of this spec.
The new assertions are listed per story in § Interfaces.

**Target Platform**: the workspace's test builds.

**Constraints**:
- The gates build with `-D warnings`: an import a deletion leaves unused fails the build.
- Simulated time is tokio's paused clock. What costs wall time is the work done in it,
  above all each rendered, detected and solved frame.
- Under nextest every test is its own process and loads the pattern database itself if
  it uses one.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | No oracle file is touched: `crates/plate-solver/tests/reference_parity.rs`, `crates/plate-solver/tests/image_parity.rs`, `crates/grpc-service/tests/e2e_service.rs`, `crates/star-detection/tests/parity.rs`, and the `e2e-eval` and `e2e-canary` scripts. No solver, detection or database code changes. |
| II. Never weaken a gate | Every deleted test, edited test body, renamed test and coverage given up is named under spec.md § Owner approvals with the skeptic's coverage-lost text. No tolerance is widened. A test that moves work out of its body keeps the assertion in a stricter place, named in § Interfaces. |
| III. Tests first, independent expectations | Each new assertion has an oracle outside the code under test: the node's own frame ids, the simulator's truth, the state file on disk, the command log. Each fails before its story's change, where the change is a saving. |
| IV. Critical paths | No `unsafe`, `Cargo.toml`, `Cargo.lock`, `build.rs`, `proto/` or `.cargo/` change. `crates/pattern-database` and `crates/database-generation` lose one duplicate unit test, in `crates/database-generation/src/patterns.rs`; no format code changes. |
| V. Standalone Rust, honest licensing | Unaffected. |

## Project Structure

| Story | Files written |
|---|---|
| US1 | `crates/finder/tests/mcp/axis_moves.rs`, `crates/simulator/tests/support/mod.rs`, `crates/simulator/tests/frame_positions.rs` |
| US2 | `crates/finder/tests/motion/tracking/routes.rs`, `crates/finder/tests/motion/tracking/min_alt.rs`, `crates/finder/tests/motion/stop/motion_ends.rs`, `crates/finder/tests/motion/stop/retry.rs`, `crates/finder/tests/motion/stop/watchdog.rs` |
| US3 | `crates/finder/tests/support/goto.rs`, `crates/finder/tests/motion/goto/sim.rs` |
| US4 | `crates/simulator/src/controls.rs`, `crates/finder/tests/motion/night.rs` |
| US5 | `crates/finder/tests/cli/track.rs` |
| US6 | `crates/finder/tests/console/session/durable.rs` (new), `crates/finder/tests/console/session/mod.rs`, `crates/finder/tests/cli/session_node.rs` (deleted), `crates/finder/tests/cli/main.rs`, `crates/finder/tests/cli/aim.rs`, `crates/finder/tests/cli/backlash.rs`, `crates/finder/tests/motion/tracking/backlash.rs` |
| US7 | `crates/finder/tests/support/live.rs`, `crates/finder/tests/console/aim.rs`, `crates/finder/tests/console/live/loop_.rs` |
| US8 | `crates/finder/tests/motion/tracking/night.rs`, `crates/finder/tests/support/goto.rs`, `crates/finder/tests/support/mcp.rs`, `crates/finder/tests/support/stop.rs`, `crates/finder/tests/motion/night.rs`, `crates/finder/tests/cli/simulate.rs`, `crates/scope-node/tests/cli.rs`, `crates/scope-node/src/rpicam/metadata.rs` |
| US9 | `crates/finder/tests/support/mod.rs` and the twenty files of § Interfaces "US9" |
| US10 | the twelve files of § Interfaces "US10" |
| US11 | `crates/finder/tests/motion/night.rs` |
| US12 | `crates/finder/tests/mcp/tracking.rs` |
| US13 | `crates/finder/tests/motion/tracking/tracker.rs`, `crates/finder/tests/motion/tracking/routes.rs` |

## Interfaces

Line numbers are at 06a7d4e. A story re-reads each line before it edits it; the text is
given as well as the number.

### Shared helpers this plan names

- **The newest frame id (MCP rig).** `LinkManager::frames()`
  (`crates/finder/src/link.rs:217`) is a watch that every frame from the node replaces
  (`crates/finder/src/link.rs:486`). Its `id` is the node's capture counter, which counts
  every capture, solved or not. `crates/finder/tests/mcp/frames.rs:180` already reads it.
  US1 adds to `crates/finder/tests/mcp/axis_moves.rs`:

  ```rust
  /// The id of the newest frame the console's link holds.
  fn newest_frame(rig: &Rig) -> u64 {
      rig.state.link().unwrap().frames().borrow().as_ref().map(|f| f.id).unwrap()
  }
  /// Stop the camera and return the newest frame id at that moment. One capture
  /// already under way may still arrive, so callers allow one more frame.
  fn camera_off(rig: &Rig) -> u64 {
      rig.controls.stop_frames();
      newest_frame(rig)
  }
  ```

  and at the end of each test that calls it:
  `assert!(newest_frame(&rig) <= stopped + 1, "frames arrived with the camera off");`.
  `rig.controls` is the node's own controls (`crates/finder/tests/support/mcp.rs:218`);
  `rig.sim.stop_frames()` would do nothing here, because the node was built with its own
  `NodeControls` (`crates/finder/tests/support/mcp.rs:214`). With frames stopped the
  capture loop waits before its next capture (`crates/scope-node/src/service.rs:227`), so
  nothing is rendered.

- **A centred start.** As `already_centred_track_is_immediate_and_repeated_on_is_idempotent`
  does (`crates/finder/tests/motion/tracking/routes.rs:383`): point the eyepiece at M42
  by hand before the first sync (`crates/finder/tests/motion/tracking/routes.rs:386`,
  `rig.sim.point_eyepiece_at(target.ra_deg, target.dec_deg);`), then sync. Track then
  finds the pointing within 1′ of the target (`crates/finder/src/session.rs:816`) and
  begins at once (`crates/finder/src/session.rs:842`): its reply reads `tracking` and
  holds no `goto`, because the session shows `goto` only after a GoTo was requested.
  Post Track within a few ticks of the sync: M42 drifts out of 1′ in about 4 s at a fixed
  mount.

### US1: the camera off

1. In `crates/finder/tests/mcp/axis_moves.rs` add the two helpers above. Then:
   - `cable_wind_refuses_one_point_two_turns_and_accepts_point_eight`
     (`crates/finder/tests/mcp/axis_moves.rs:319`) and
     `braking_distance_is_checked_at_the_cable_limit`
     (`crates/finder/tests/mcp/axis_moves.rs:597`): after `let rig = ready().await;`,
     `let stopped = camera_off(&rig);`; after the last existing assertion, the bound.
   - `rate_duration_and_acceleration_ramp_match_the_simulator_and_bad_duration_moves_nothing`
     (`crates/finder/tests/mcp/axis_moves.rs:347`),
     `disconnected_client_does_not_end_heartbeats_or_leave_a_rate_running`
     (`crates/finder/tests/mcp/axis_moves.rs:374`) and
     `a_zero_rate_still_reserves_motion_until_its_duration_ends`
     (`crates/finder/tests/mcp/axis_moves.rs:651`): the same.
   - `sunrise_halts_with_the_page_guard_reason_within_one_second`
     (`crates/finder/tests/mcp/axis_moves.rs:394`) builds its own rig and syncs at
     `crates/finder/tests/mcp/axis_moves.rs:397`; `camera_off` goes right after that
     sync. Its Sun guard is driven by the clock, not by frames.
   - `track_through_the_page_blocks_all_three_mcp_moves_and_keeps_tracking`
     (`crates/finder/tests/mcp/axis_moves.rs:522`): replace `let rig = ready().await;`
     with the centred start: `Rig::new(START, 50.0, 120.0, true)`, point the eyepiece at
     M42 (`finder::targets::find("M42")`), `rig.sync()`, and the connected check
     `ready()` makes. After the page's Track request, assert
     `session(&rig.app).await["track"]["state"] == "tracking"` and that the session has
     no `goto`, before the existing wait loop. Everything after is unchanged.

   Why it loses nothing: these tests assert axis counts, command times, heartbeats and
   refusals. The cable-limit refusals are re-checked from axis counts every 50 ms during
   a direct move (`crates/finder/src/mcp/guard.rs:155`), and the wind is observed from
   counts there too (`crates/finder/src/goto.rs:745`). Heartbeats do not wait for frames.
   With the camera off the model stays the first sync's, which makes the cable margins
   more repeatable, not less.

2. In `crates/simulator/tests/support/mod.rs` the rig (`crates/simulator/tests/support/mod.rs:96`)
   becomes:
   - `Rig::new()`: a `VirtualClock` at the same start, a `SimMount` on it with the same
     configuration, and `NodeService::with_mount` over
     `scope_node::synthetic::SyntheticCamera`, as `crates/simulator/tests/stop_finishes.rs:101`
     builds it, with the same settings. No `Simulator` and no pattern database. Before it
     returns it asserts
     `rig.camera.describe(DescribeRequest {}).await.unwrap().into_inner().model == "simulated"`;
     the synthetic camera's model is `crates/scope-node/src/synthetic.rs:34`, the catalog
     camera's is `crates/simulator/src/camera.rs:79`.
   - `Rig::rendered()`: what `Rig::new()` builds today, on `SimCamera` and the database
     (`crates/simulator/tests/support/mod.rs:24`).
   - The field `pub sim: Simulator` becomes `pub clock: VirtualClock`. Nothing in
     `stop_and_watchdog.rs` or `watchdog_report.rs` reads `rig.sim`.
3. In `crates/simulator/tests/frame_positions.rs`, `Rig::new()` becomes `Rig::rendered()`
   and the three `rig.sim.clock()` reads (`crates/simulator/tests/frame_positions.rs:22`
   and the two after it) become `rig.clock`. Its 0.2 s exposure assertion
   (`crates/simulator/tests/frame_positions.rs:29`) is unchanged.

### US2: centred starts

1. `crates/finder/tests/motion/tracking/routes.rs`: add beside `accepted()`
   (`crates/finder/tests/motion/tracking/routes.rs:85`):

   ```rust
   /// Track running on M42 with no GoTo: for tests whose claim is not the handoff.
   async fn centred() -> Rig {
       let rig = Rig::new(START, 50.0, 120.0, true).await;
       let target = targets::find("M42").unwrap();
       rig.sim.point_eyepiece_at(target.ra_deg, target.dec_deg);
       rig.sync().await;
       assert_eq!(rig.view().await["mount"]["connected"], true);
       let body = track(&rig, true).await;
       assert_eq!(body["track"]["state"], "tracking", "{body}");
       assert!(body.get("goto").is_none(), "a centred Track ran a GoTo: {body}");
       moving(&rig).await;
       rig
   }
   ```

   and call it instead of `accepted()` in
   `below_horizon_and_foreign_origin_refuse_after_positive_track_control`
   (`crates/finder/tests/motion/tracking/routes.rs:148`),
   `foreign_bodiless_stop_ends_tracking_and_no_rate_restarts_it`
   (`crates/finder/tests/motion/tracking/routes.rs:178`),
   `track_names_missing_mount_sync_site_target_and_sun_refusals`
   (`crates/finder/tests/motion/tracking/routes.rs:304`) and
   `dropping_console_cancels_tracking_without_retaining_the_session`
   (`crates/finder/tests/motion/tracking/routes.rs:407`). `accepted()` and `tracking()`
   stay, for the handoff tests and the readout test. `centred()` cannot call `tracking()`:
   it asserts the GoTo finished (`crates/finder/tests/motion/tracking/routes.rs:57`).
2. `crates/finder/tests/motion/tracking/min_alt.rs`: in
   `raised_minimum_altitude_ends_running_tracking_as_too_low`
   (`crates/finder/tests/motion/tracking/min_alt.rs:28`), point the eyepiece at M42
   between `Rig::new` (`crates/finder/tests/motion/tracking/min_alt.rs:29`) and the sync,
   and assert the Track reply (`crates/finder/tests/motion/tracking/min_alt.rs:31`, whose
   reply is discarded today) reads `tracking` with no `goto`. The poll for `tracking`
   then ends at once; the rest is unchanged.
3. `crates/finder/tests/motion/stop/motion_ends.rs`: `tracking()`
   (`crates/finder/tests/motion/stop/motion_ends.rs:282`) builds its rig as `ready()`
   (`crates/finder/tests/motion/stop/motion_ends.rs:18`) does, but points the eyepiece
   at M42 before the sync; `ready()` itself is shared with tests that need a slew and
   does not change. After the wait for `tracking`, assert that `rig.commands` holds no
   `MountCommand::GoTo`. The azimuth-moving check
   (`crates/finder/tests/motion/stop/motion_ends.rs:294`) still holds: M42's azimuth
   rate is not zero, and the tracker sends its first rate at once.
4. `crates/finder/tests/motion/stop/retry.rs`: in
   `retained_tracker_cannot_delay_goto_cancellation_or_first_attempt`
   (`crates/finder/tests/motion/stop/retry.rs:198`), point the eyepiece at M42 between
   `Rig::new` (`crates/finder/tests/motion/stop/retry.rs:199`) and the sync, and after the
   wait for `tracking` assert that `rig.commands` holds no `MountCommand::GoTo`. Its later
   GoTo to M45 is unchanged.
5. `crates/finder/tests/motion/stop/watchdog.rs`: in
   `past_watchdog_firing_is_the_baseline_for_a_successful_later_goto`
   (`crates/finder/tests/motion/stop/watchdog.rs:157`), replace `ready()`
   (`crates/finder/tests/motion/stop/watchdog.rs:158`) with `Rig::new()`, a push of the
   mount to 5° below M42 (as `crates/finder/tests/motion/tracking/routes.rs:121` does,
   `rig.mount.push_to(alt - 5.0, az)`), then the sync, before the watchdog. Before the
   GoTo (`crates/finder/tests/motion/stop/watchdog.rs:192`) assert that the telescope's
   true direction is within 6° of M42. The final assertions
   (`crates/finder/tests/motion/stop/watchdog.rs:206` onward) are unchanged; a GoTo of a
   few degrees still sends GoTo commands.

The stop rig starts at azimuth 120°, altitude 50° (`crates/finder/tests/support/stop.rs:281`),
and records GoTo, Rate and StopAll commands (`crates/finder/tests/support/stop.rs:260`).

### US3: cheaper GoTo setups

1. `crates/finder/tests/support/goto.rs`: `Rig::new(start, alt, az, connected)`
   (`crates/finder/tests/support/goto.rs:109`) builds its session at
   `crates/finder/tests/support/goto.rs:153` (`let session = Session::default();`). Add
   `Rig::with_session(start, alt, az, connected, session: Session)`, which is the body of
   `new` with that line replaced by the parameter, and make `new` call it with
   `Session::default()`. Its session edit and aim (the lines after `crates/finder/tests/support/goto.rs:153`)
   still apply, and write to the state file when the session has one.
2. Add to the same file, beside `Rig::second` (`crates/finder/tests/support/goto.rs:243`):
   - `pub async fn far_sync(path: &Path, start: f64)`: writes a state file holding the
     far sync of `second()`, computed, not typed. A second `SimMount` built from the
     rig's own mount configuration (`crates/finder/tests/support/goto.rs:112`) is driven
     in paused time by positive moves to axis counts (580000, 4500), the counts of
     `crates/finder/tests/support/goto.rs:250`, so its gears sit on the same flank. Its
     `true_direction()` gives the sync's `az_deg`; `finder::ephemeris::apparent_from_true`
     (`crates/finder/src/ephemeris.rs:129`) of its altitude gives `alt_deg`, because
     solved frames record apparent altitude. The file holds `site`, `min_alt_deg` and a
     `model` with that one sync (`time` = `start`, `counts_per_revolution` as the mount's,
     `epoch` 0, `rms_arcsec` 1), one epoch whose zero points are the rig's on the positive
     flank, no `base_tilt`, no `fixed` and no `backlash`.
   - `pub async fn second_from_file(&self)`: makes the first 1° move of `second()`
     (`crates/finder/tests/support/goto.rs:246`), 15 ticks, and asserts what `second()`
     asserts (`crates/finder/tests/support/goto.rs:255` and the line after it), plus
     that the azimuth axis reads 4500 counts and the model holds at least two syncs.
   Loading a state file does not re-fit anything (`crates/finder/src/session.rs:1457`);
   the base tilt is fitted by the product when the live sync of the 1° move joins the
   stored sync in the same epoch, 129° apart (`crates/finder/src/model.rs:519`).
3. `crates/finder/tests/motion/goto/sim.rs`:
   `five_catalog_targets_converge_after_two_syncs_and_report_progress`
   (`crates/finder/tests/motion/goto/sim.rs:9`) writes the file with `far_sync` into a
   scratch folder, builds the rig with
   `Rig::with_session(START, 50.0, 120.0, true, Session::with_state_file(path).unwrap())`
   and calls `second_from_file()` instead of `second()`
   (`crates/finder/tests/motion/goto/sim.rs:11`). The target list
   (`crates/finder/tests/motion/goto/sim.rs:12`) and every assertion after it are
   unchanged. The folder is built as the file's neighbours build theirs today; US9 moves
   it to the shared helper later.
4. Same file: `continuously_observed_cable_wind_survives_real_simulator_power_cycle`
   (`crates/finder/tests/motion/goto/sim.rs:255`): before
   `rig.move_to([350 * 4500, 0])` (`crates/finder/tests/motion/goto/sim.rs:258`) read the
   frame id of the session's pointing and call
   `rig.controls.set_frame_interval(Duration::from_secs(2))`; after it read the frame id
   again, call `rig.controls.set_frame_interval(Duration::from_millis(500))`, and assert
   the advance is at least 20 and at most 60. The power cycle
   (`crates/finder/tests/motion/goto/sim.rs:263`) and the rest are unchanged; the cycle
   needs a sync within 1.5 s afterwards, which is why 500 ms comes back first. At 2 s the
   350° move is sampled about every 7°, inside the half-turn the wind monitor needs
   between observations (`crates/finder/src/goto.rs:60`).

### US4: the whole night at 2 frames a second

1. `crates/simulator/src/controls.rs`: beside `stop_frames`
   (`crates/simulator/src/controls.rs:105`):

   ```rust
   pub fn set_frame_interval(&self, interval: Duration) {
       self.controls.set_frame_interval(interval);
   }
   ```

   with `std::time::Duration` imported. The simulator's controls are the node's
   (`crates/simulator/src/node.rs:45`); `NodeControls::set_frame_interval`
   (`crates/scope-node/src/service.rs:71`) refuses zero; the capture loop reads the
   interval before every frame (`crates/scope-node/src/service.rs:229`).
2. `crates/finder/tests/motion/night.rs`: after
   `let sim = state.simulator().unwrap().clone();` (`crates/finder/tests/motion/night.rs:136`),
   `sim.set_frame_interval(Duration::from_millis(500));`. After the first-sync block
   (`crates/finder/tests/motion/night.rs:153`), read the session about 30 times, one tick
   apart, collect the distinct `(pointing.frame_id, pointing.time)` pairs, skip the first,
   and assert at least four remain and that for each consecutive pair the time step
   divided by the frame-id step is 0.5 s within 0.05 s. At the 200 ms default it is
   0.2 s. Nothing else in the test changes.

### US5: Track's restart in paused time

`crates/finder/tests/cli/track.rs`, `tracking_that_moved_the_mount_is_off_and_quiet_after_restart`
(`crates/finder/tests/cli/track.rs:164`), becomes `#[tokio::test(start_paused = true)]`,
as `crates/finder/tests/cli/backlash.rs:119` is, and:

1. In process, on the test's folder:
   `goto::Rig::with_session(goto::START, 50.0, 120.0, true, Session::with_state_file(folder/state).unwrap())`
   (US3's constructor), a centred start on M42, `rig.sync()`, then `POST /api/track`
   `{"on":true}` with no Origin header (allowed for writes), asserting the reply reads
   `tracking` with no `goto`. Then about 12 ticks, and assert the mount's true direction
   moved. Then `drop(rig)`.
2. The checks that follow are today's from the state-file read on
   (`crates/finder/tests/cli/track.rs:189`): no `track` in the stored file, one start of
   the built binary (`crates/finder/tests/cli/track.rs:192`), connected, no `track` in its
   session, 2 s of no motion and the eyepiece unchanged. The binary's blocking calls run
   inside the paused test as `crates/finder/tests/cli/backlash.rs:140` already does.
3. What goes: the first start of the binary, the `POST /api/sim/telescope` push, the two
   real-time waits (`crates/finder/tests/cli/track.rs:179` and
   `crates/finder/tests/cli/track.rs:183`), and the 1.2 s sleep
   (`crates/finder/tests/cli/track.rs:185`).
4. A comment above the test says: Track on the built binary is owned by Playwright's
   `tracking.spec.ts`; this test owns "Track is off and nothing moves after a restart".

The stored file never holds `track` whatever tracking did: `StoredSession`
(`crates/finder/src/session.rs:1396`) has no such field and refuses unknown ones. The
other tests of the file are unchanged.

### US6: the binary only for `main.rs`

1. **New** `crates/finder/tests/console/session/durable.rs`, declared by `mod durable;` in
   `crates/finder/tests/console/session/mod.rs`, with one test,
   `durable_fields_survive_reopening_the_state_file_in_owner_only_mode`, on
   `#[tokio::test]` with a fixed clock:
   - a folder holding a state file that it sets to mode 0644, and a stale
     `session.json.tmp` beside it;
   - for exposure 200 ms and then 2000 ms: `Session::with_state_file`
     (`crates/finder/src/session.rs:1224`), one `Session::edit`
     (`crates/finder/src/session.rs:1197`) setting site, minimum altitude, target,
     imaging camera, node address and token, and camera settings (exposure, gain 4,
     sigma 5), and `Session::set_aim` (`crates/finder/src/session.rs:1180`) to (700, 420);
     drop it; reopen with `Session::with_state_file`; assert every value equal, the
     view's `token_set` true and the token absent from the view's JSON;
   - after the first write: the file's mode is 0600 (`crates/finder/src/session.rs:1479`)
     and the stale `.tmp` is gone (`crates/finder/src/session.rs:1471`).
2. **Delete** `crates/finder/tests/cli/session_node.rs` and `mod session_node;`
   (`crates/finder/tests/cli/main.rs:11`). Its stderr check
   (`crates/finder/tests/cli/session_node.rs:80`) moves to item 3; its restart, mode and
   exposure checks are item 1's.
3. `crates/finder/tests/cli/aim.rs`: its server pipes the child's stderr
   (`crates/finder/tests/cli/aim.rs:108`) and its reader thread
   (`crates/finder/tests/cli/aim.rs:113`) only looks for the address. It keeps every line
   in a shared `logs` string, as `crates/finder/tests/cli/session_node.rs:16` does, and
   when the server is dropped asserts `!logs.contains("test-token")`. Its test
   (`crates/finder/tests/cli/aim.rs:165`) is otherwise unchanged and stays the restart
   check on the real binary.
4. `crates/finder/tests/cli/backlash.rs`: its `Server` gains a request with a method and
   path, as `crates/finder/tests/cli/track.rs:93` has. The restart loop
   (`crates/finder/tests/cli/backlash.rs:140`) is unrolled so the second server stays up
   after its read of the measured backlash; the test then posts `{"what":"fixed"}` to
   `/api/model/reset`, asserts the reply has no `backlash` and the next session read has
   none, drops the server, and asserts the state file has no `backlash`.
5. `crates/finder/tests/motion/tracking/backlash.rs`: delete
   `first_transit_measures_then_fixed_reset_removes_the_observed_value`
   (`crates/finder/tests/motion/tracking/backlash.rs:9`). Its value check is the cli
   test's; its reset checks (`crates/finder/tests/motion/tracking/backlash.rs:20` and the
   line after) are item 4's. `another_target_on_the_same_mount_takes_up_the_measured_altitude_play`
   stays: it is the only test where a value measured in the running session is taken up
   without a reload.

`crates/finder/tests/cli/session.rs` is unchanged: it keeps `--state` over
`XDG_STATE_HOME` (`crates/finder/tests/cli/session.rs:153`) and the other two folders on
the real binary.

### US7: previews and the unsolved interval

1. `crates/finder/tests/support/live.rs`: `state` (`crates/finder/tests/support/live.rs:60`)
   sets 500 ms at `crates/finder/tests/support/live.rs:71`. Add `state_with_interval`,
   the same body with the interval as a parameter, and make `state` call it with
   500 ms. Its eight callers are unchanged.
2. `crates/finder/tests/console/aim.rs`:
   `aim_writes_enforce_origin_json_and_strict_shape_and_preview_retention_is_eight`
   (`crates/finder/tests/console/aim.rs:63`) calls `state_with_interval(.., 3 s)`. The
   node sends a preview when 5 s have passed since the last
   (`crates/scope-node/src/preview.rs:12`), so at 3 s previews fall on every other frame.
   After its loop (`crates/finder/tests/console/aim.rs:108`) assert
   `previews.windows(2).all(|w| w[1] - w[0] == 2)`. Every retention assertion after it is
   unchanged. The wait for each frame allows 10 s of virtual time
   (`crates/finder/tests/support/live.rs:121`), more than 3 s.
3. `crates/finder/tests/console/live/loop_.rs`:
   `rich_frames_failing_for_thirty_seconds_report_the_whole_unsolved_interval`
   (`crates/finder/tests/console/live/loop_.rs:240`) calls `state_with_interval(.., 5 s)`,
   counts the frames its loop (`crates/finder/tests/console/live/loop_.rs:248`) receives,
   and asserts at most 7. The unsolved interval is measured from the first unsolved rich
   frame, so the frame count does not change it
   (`crates/finder/tests/console/live/loop_.rs:255` is unchanged).

### US8: hazards

1. `crates/finder/tests/motion/tracking/night.rs`: in
   `an_hour_covers_transit_too_low_and_keyhole_in_under_thirty_wall_seconds`
   (`crates/finder/tests/motion/tracking/night.rs:328`), delete `let wall = Instant::now();`
   (`crates/finder/tests/motion/tracking/night.rs:329`) and the wall-time assertion
   (`crates/finder/tests/motion/tracking/night.rs:407`); rename the test
   `an_hour_covers_transit_too_low_and_keyhole`; drop `Instant` from the import
   (`crates/finder/tests/motion/tracking/night.rs:27`) and ", and 30 s wall time" from the
   module comment (`crates/finder/tests/motion/tracking/night.rs:4`). A runaway is ended
   by nextest (`.config/nextest.toml:23`, slow at 60 s, ended after five periods).
2. The inline rig solvers: `Stars` in `crates/finder/tests/support/goto.rs:39`, in
   `crates/finder/tests/support/mcp.rs:113`, in `crates/finder/tests/support/stop.rs:64`
   and in `crates/finder/tests/motion/night.rs:21` each call
   `solve::solve_centroids(.., &options)` with the 5000 ms default
   (`crates/finder/src/solve.rs:69`), which the solver checks against the real clock
   (`crates/plate-solver/src/status.rs:125`). Each passes
   `&SolveOptions { timeout_ms: u64::MAX, ..options }` instead. Nothing adds the timeout
   to an `Instant`, so `u64::MAX` cannot overflow.
3. `crates/finder/tests/cli/simulate.rs`: `let first = ...` is read at
   `crates/finder/tests/cli/simulate.rs:130`, before a wait for the first live pointing
   and two edits; the 5 s window starts at `crates/finder/tests/cli/simulate.rs:142`.
   Read the session time again immediately before `let at = Instant::now();` and compare
   against that reading at `crates/finder/tests/cli/simulate.rs:155`. `first` stays for
   the epoch check that uses it.
4. `crates/scope-node/tests/cli.rs`: `run` (`crates/scope-node/tests/cli.rs:7`) calls
   `.output()` (`crates/scope-node/tests/cli.rs:21`), which waits forever. It spawns the
   child with piped output, polls `try_wait` every 50 ms for up to 10 s, and on timeout
   kills it and panics naming the arguments. The refusal test
   (`crates/scope-node/tests/cli.rs:62`) is otherwise unchanged and still starts the node
   without `--check-config`, so it still checks the serving path's refusal.
5. `crates/scope-node/src/rpicam/metadata.rs`: the three tests at
   `crates/scope-node/src/rpicam/metadata.rs:151`, `crates/scope-node/src/rpicam/metadata.rs:166`
   and `crates/scope-node/src/rpicam/metadata.rs:192` become
   `#[tokio::test(start_paused = true)]`. They read in-memory duplex streams only, so a
   paused clock changes nothing but makes their 100 ms timeouts independent of load;
   `crates/scope-node/src/synthetic.rs:101` is a precedent.

### US9: scratch folders

1. `crates/finder/tests/support/mod.rs` gains, below its `pub mod` lines:

   ```rust
   /// A folder for one test's files, under the target directory cargo names, removed
   /// when dropped. The name, process id and a counter keep it unique under nextest and
   /// under `cargo test`.
   pub struct Scratch(pub std::path::PathBuf);
   impl Scratch {
       pub fn new(name: &str) -> Self {
           static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
           let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
           let path = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"))
               .join(format!("{name}-{}-{n}", std::process::id()));
           let _ = std::fs::remove_dir_all(&path);
           std::fs::create_dir_all(&path).unwrap();
           Self(path)
       }
   }
   impl Drop for Scratch {
       fn drop(&mut self) {
           let _ = std::fs::remove_dir_all(&self.0);
       }
   }
   ```

   The `layout` tests pin the file list of `support/`, not its contents, and this holds no
   test.
2. Every site below uses `crate::support::Scratch::new("<its old prefix>")` in place of
   its hand-built path, and drops its own `Folder` type and `remove_dir_all` calls:

   | File | Site | Removed today |
   |---|---|---|
   | `crates/finder/tests/cli/aim.rs:181` | `aim-cli` | on drop |
   | `crates/finder/tests/cli/session.rs:17` | `session-cli` | on drop |
   | `crates/finder/tests/cli/backlash.rs:18` | `backlash-cli` | on drop |
   | `crates/finder/tests/cli/simulate.rs:15` | `simulate-<name>` | on drop |
   | `crates/finder/tests/cli/track.rs:16` | `track-cli` | on drop |
   | `crates/finder/tests/console/session/writes.rs:182` | `write-hosts` | after a `catch_unwind` |
   | `crates/finder/tests/console/targets.rs:106` | `targets-empty-cwd` | `remove_dir`, a check |
   | `crates/finder/tests/console/session/edits.rs:170` | `session-library-home` | before the asserts |
   | `crates/finder/tests/mcp/endpoint.rs:50` | `mcp-state` | end of test only |
   | `crates/finder/tests/mcp/motion.rs:189` | `mcp-motion` | end of test only |
   | `crates/finder/tests/mcp/axis_moves.rs:623` | `us5-axis-session` | end of test only |
   | `crates/finder/tests/motion/model/sim.rs:217` | `model-session` | end of test only |
   | `crates/finder/tests/motion/model/routes.rs:54` | `model-api` | end of test only |
   | `crates/finder/tests/motion/tracking/backlash.rs:84` | `backlash-az` | end of test only |
   | `crates/finder/tests/motion/tracking/keyhole.rs:149` | `keyhole-<name>` | in the constructor |
   | `crates/finder/tests/motion/tracking/night.rs:139` | `tracking-night-<name>` | never |
   | `crates/finder/tests/motion/goto/sim.rs:9` | US3's folder | as US3 left it |

   Where a constructor removed its folder early (`keyhole.rs`), the rig keeps the
   `Scratch` so the folder lives as long as the rig. In
   `crates/finder/tests/console/targets.rs` the `remove_dir` after the child
   (`crates/finder/tests/console/targets.rs:124`) also proved the child left the folder
   empty; it becomes `assert_eq!(std::fs::read_dir(&empty.0).unwrap().count(), 0)`.
   `crates/finder/tests/console/session/edits.rs` already counts the entries before it
   removes the folder, and keeps that count.
3. The scope-node files have no shared helper and get none: in
   `crates/scope-node/tests/cli.rs:9`, `crates/scope-node/tests/mount_config.rs:9` and
   `crates/scope-node/tests/rpicam.rs:123` the path is built under
   `env!("CARGO_TARGET_TMPDIR")` instead of `CARGO_MANIFEST_DIR` and `../../target`, and
   held by a small guard in the file whose `Drop` removes it.

The two unit tests in `crates/finder/src` that make folders are left alone: cargo sets
`CARGO_TARGET_TMPDIR` only for integration tests and benches.

### US10: duplicates

Every edit below was read at 06a7d4e; none touches an oracle file.

| Finding | File | Edit | The stricter twin |
|---|---|---|---|
| core-D3 | `crates/grpc-service/tests/service_acceptance.rs:240` | delete `pixel_center_convention_top_left` | `extract_centroids_reports_wire_x_as_column` (`crates/grpc-service/tests/service_acceptance.rs:404`): asymmetric pixel, axis order, one star |
| core-D4 | `crates/grpc-service/tests/service_acceptance.rs:187` and `crates/grpc-service/tests/service_acceptance.rs:325` | delete `inline_image_bytes_used_when_shmem_absent` and `coordinate_swap_inbound_and_outbound` | `all_four_rpcs_available` (`crates/grpc-service/tests/service_acceptance.rs:82`) makes the same calls; neither could fail |
| core-D5 | `crates/plate-solver/tests/solve_acceptance.rs:287` | delete the solve half of `cancellation_honored`; rename the rest `solve_context_reports_a_set_cancel_flag` and say in its comment that `SolveStatus::Cancelled` has no test and no public trigger | `timeout_bounds_the_search` (`crates/plate-solver/tests/solve_acceptance.rs:266`) |
| core-D6 | `crates/plate-solver/tests/solve_acceptance.rs:88` and `crates/plate-solver/tests/solve_acceptance.rs:159` | delete `explicit_detection_parameters_honored` and `detection_parameters_do_not_perturb_solve_math`; rename `brightest_first_requirement` (`crates/plate-solver/tests/solve_acceptance.rs:43`) `exhausted_search_of_an_empty_pattern_catalog_is_no_match`, assertions unchanged | E2's image parity, and `TOO_FEW` on a blank image over the wire |
| core-D7 | `crates/database-generation/src/patterns.rs:177` | delete `breadth_first_yields_brightest_first`; move `matches_cedar_order_and_visits_each_combination_once` into `tests` and delete `bfc_order_tests` | the seven-element order test pins the five-element sequence |
| node-D1 | `crates/simulator/tests/mount.rs:84` | delete `backlash_changes_truth_by_point_three_at_the_same_axis_reading` | `defaults_keep_zero_rate_error_and_point_three_backlash_on_both_axes` (`crates/simulator/tests/rate_and_backlash.rs:71`): 1″ against 0.01°, and exact positions |
| node-D2 | `crates/scope-node/tests/skywatcher.rs:222` | reduce `both_stops_precede_emergency_fallback` to `stop_all_stops_both_axes`: default emulator, both axes running, `stop_all`, assert K1 and K2 sent and both axes at rest | `a_silent_first_axis_confirms_with_emergency_stop_after_the_second_axis` in `crates/scope-node/tests/skywatcher_stop.rs`; the reduced test stays the only caller of `SkyWatcher::stop_all` |
| node-D3 | `crates/scope-node/src/mount/mod.rs:78` | delete the `tests` module and its `FakeMount` | the port contract on real adapters, `crates/simulator/tests/mount.rs` and `crates/scope-node/tests/skywatcher.rs` |
| api-D9 | `crates/finder/src/session.rs:1545` and `crates/finder/src/session.rs:1555` | delete the two inline tests | `fresh_session_and_fixed_clock_are_exact_on_repeated_reads` (`crates/finder/tests/console/session/edits.rs:54`) |
| simA-D5 | `crates/finder/tests/motion/model/sim.rs:200` | move the tilt assertions (`crates/finder/tests/motion/model/sim.rs:207` to the line before the test's end) into `durable_night_survives_local_midnight_then_expires_at_solar_noon` in place of its `base_tilt.is_some()` (`crates/finder/tests/motion/model/sim.rs:228`), then delete `second_sync_fits_the_simulator_base_without_opening_an_epoch` | the durable night runs the same setup and second sync |
| simA-D4 | `crates/finder/tests/motion/goto/routes.rs:49` to `crates/finder/tests/motion/goto/routes.rs:101` | delete `approach_to_fifteen_point_two_degrees_is_refused_without_motion`, `sun_five_degrees_up_is_refused_without_motion`, `m8_below_horizon_is_refused_without_motion` and `absent_mount_and_missing_sync_each_name_the_reason`, and the `ephemeris` and `sky` imports only they used (`crates/finder/tests/motion/goto/routes.rs:6`); in `no_mount_omits_refusal_and_second_sync_omits_it_while_running` assert the no-mount refusal's error names `no mount` (`crates/finder/tests/motion/goto/refusals.rs:119`, which checks only 409 today) | each has a stricter copy in `crates/finder/tests/motion/goto/refusals.rs` that also checks `goto_refusal`; `foreign_goto_is_forbidden_without_motion` and `simulate_startup_connects_the_default_mount_and_serves_motion_and_stop` stay |
| simA-D9 | `crates/finder/tests/motion/model/routes.rs:107` | the foreign-origin loop keeps `"fixed"` only | the second-sync route test covers the foreign `night` reset; the guard runs before the body is read |
| mcp-D3 | `crates/finder/tests/mcp/motion.rs:226` | `stopped()` takes `watch: bool`: always StopAll within 0.1 s, the halt reason, motion cleared and at rest within 20 ticks (2 s); only with `watch`, today's 5 s at rest, 5 s unchanged and only-StopAll checks. The `"second client"` case (`crates/finder/tests/mcp/motion.rs:272`) passes `true`, the other eight `false` | the spec asks for the 5 s watch in the second-client case only (016 US4-S6) |

Why node-D1 deletes only: the skeptic's tidy-up, moving two default-value assertions to
`default_base_tilt_and_zero_points_are_present`, would leave the survivor's name claiming
"zero rate error" it no longer checks; the assertions already sit in the survivor and
`crates/simulator/tests/mount.rs:196` already checks the 0.3° default.

### US11 (optional): the whole night shortened

`crates/finder/tests/motion/night.rs`: the five GoTos (`crates/finder/tests/motion/night.rs:174`,
`["M42", "M31", "M45", "M81", "M1"]`) become one, to M81, with the same assertions; the
first sync, the one-tap second sync, the hand push and its GoTo to M42, the Stop mid-slew
to M31 and the dropped link mid-slew to M81 are unchanged. The test is renamed
`a_whole_night_syncs_gotos_hand_push_stop_and_dropped_link`. Spec 014 US9's "five GoTos"
(`specs/014-goto/spec.md:677`) keeps its text; approval 15 records the amendment.

### US12 (optional): the MCP hold shortened

`crates/finder/tests/mcp/tracking.rs`: the hold loop
(`crates/finder/tests/mcp/tracking.rs:181`, `for second in 0..=600`) spans 75 s; the test
(`crates/finder/tests/mcp/tracking.rs:163`) is renamed
`track_first_goto_keeps_mcp_source_and_holds_m42_for_seventy_five_seconds`; the comment
above the loop states 75 s. Every assertion in and after the loop is unchanged. Spec 016
US6-S1 (`specs/016-mcp/spec.md:694`) keeps its text; approval 16 records the amendment.

### US13 (optional): the window below the route

1. `crates/finder/tests/motion/tracking/tracker.rs`:
   `pointing_samples_age_on_console_time_and_are_ignored_after_ending`
   (`crates/finder/tests/motion/tracking/tracker.rs:480`) feeds one sample and then runs
   1202 ticks of 50 ms (`crates/finder/tests/motion/tracking/tracker.rs:500`). Split that
   run at 1198 ticks (59.9 s): assert the sample is still held, then run the last 4 ticks
   and keep today's check that it is gone.
2. `crates/finder/tests/motion/tracking/routes.rs`:
   `readout_uses_live_errors_last_sixty_seconds_and_fr001_fr002`
   (`crates/finder/tests/motion/tracking/routes.rs:244`) replaces its 750 ticks
   (`crates/finder/tests/motion/tracking/routes.rs:259`) with ticks until the tracker
   holds more than 30 samples, and drops the two window assertions
   (`crates/finder/tests/motion/tracking/routes.rs:265` and the line after). Its RMS,
   field-rotation and longest-exposure assertions stay. Then it stops the camera
   (`rig.controls.stop_frames()`), ticks 61 s so the live samples age out, injects one
   sample with `rig.state.session().tracker().unwrap().pointing(east, north)`
   (`crates/finder/src/tracking.rs:240`), and asserts the readout's `rms_arcsec` equals
   that sample's error after 59.9 s and is 0 after 60.1 s. Missing solves do not end
   Track, so the readout stays present. The readout's own filter is
   `crates/finder/src/session.rs:153`.

Spec 015 US7-S7 (`specs/015-tracking/spec.md:416`) keeps its text; approval 17 records
the amendment.

## Measurements

"Before" is each test's duration in the GitHub `test` job of ergane gates run 43683194483
on b8dceaa (nextest, optimized test build). "After" is measured by round 2's probe, on the
post-027 tree, and is not part of any scenario.

| Story | Tests changed | Before (s) | After (s) |
|---|---|---|---|
| US1 | `axis_moves::braking_distance_…` 32.515, `cable_wind_refuses_…` 27.516, `track_through_the_page_…` 8.384, `disconnected_client_…` 7.405, `rate_duration_…` 5.202, `sunrise_halts_…` 4.076, `a_zero_rate_…` 2.039; simulator `silent_link_…` 2.584, `stop_cancels_…` 2.457, `watchdog_count_…` 2.086, `a_full_adapter_queue_…` 0.396 | 94.660 | round 2 |
| US2 | `tracking::routes::track_names_…` 13.720, `foreign_bodiless_…` 8.706, `below_horizon_…` 6.740, `dropping_console_…` 5.518; `min_alt::…` 6.139; `motion_ends::track_stop_shares_…` 6.521, `track_stop_starts_…` 5.530; `retry::retained_tracker_…` 6.032; `watchdog::past_watchdog_…` 5.651 | 64.557 | round 2 |
| US3 | `goto::sim::five_catalog_…` 80.944, `continuously_observed_cable_wind_…` 31.589 | 112.533 | round 2 |
| US4 | `night::a_whole_night_…` | 181.548 | round 2 |
| US5 | `track::tracking_that_moved_…` | 27.032 | round 2 |
| US6 | removed: `session_node::…` 1.554, `tracking::backlash::first_transit_…` 3.438; changed: `cli backlash::…` 4.098, `cli aim::…` 1.373 | 10.463 | round 2 |
| US7 | `aim::aim_writes_…` 9.591, `live::loop_::rich_frames_…` 8.336 | 17.927 | round 2 |
| US8 | `tracking::night::an_hour_…` 2.915, `cli simulate::simulated_epoch_…` 5.775, scope-node `start_refuses_…` 0.010 | no saving intended | round 2 |
| US9 | none intended | n/a | n/a |
| US10 | removed: `model::sim::second_sync_fits_…` 11.974, `goto::routes::absent_mount_…` 2.590, `approach_to_…` 2.130, `m8_below_…` 1.926, `sun_five_…` 1.487, and ten tests of 0.010 s or less each; changed: `mcp motion::security_rejections_…` 15.371, `second_client_…` 12.246 | 47.724 | round 2 |
| US11 (optional) | `night::a_whole_night_…` (before US4) | 181.548 | round 2, after US4 |
| US12 (optional) | `mcp tracking::track_first_goto_…_ten_minutes` | 173.646 | round 2 |
| US13 (optional) | `tracking::routes::readout_uses_…` 27.232, `tracking::tracker::pointing_samples_…` 0.483 | 27.715 | round 2 |

For scale, the same log's per-test durations add up to 1,150 s across 621 tests.

## Shapes that existing tests pin

- **Track's reply when no GoTo runs.** `already_centred_track_is_immediate_and_repeated_on_is_idempotent`
  asserts a centred Track replies `tracking` with no `goto`; US2's guard is that same
  shape.
- **The node's frame ids count every capture** and start at 1, whichever camera.
- **The goto rig's session edit** writes node, site, target, camera settings and aim; on a
  state-file session those land in the file, which the cli binary ignores under
  `--simulate` (it pairs its own session).
- **Spec texts that cite old paths** (`--test track_api`, `crates/finder/tests/night_sim.rs`
  and the like) stay as landed; spec 027's move map gives the new names.

## Decisions and the alternatives rejected

| Decision | Rejected | Why |
|---|---|---|
| Bound the newest frame id at one more than at the stop. | Assert no frame at all; count frames in the live loop. | A capture already under way still arrives. The live loop drops frames that arrive while it solves, so it under-counts. |
| A second constructor, `Rig::rendered`, for `frame_positions.rs`. | Move `frame_positions.rs` to the synthetic camera too. | The skeptic's coverage note: it would stop exercising the rendered camera's exposure times with mount sampling. |
| A new `centred()` helper in `routes.rs`; `accepted()` stays. | Change `accepted()`. | `accepted()` also serves the readout test, whose span US13 may change; keeping it leaves that test to US13 alone. |
| The five-target test keeps one live sync and loads only the far one. | Load both syncs; load the fixture from `motion/model/routes.rs`. | The first 1° move puts the gears on the flank both syncs use and gives a solved sync, so the product fits the tilt from its own frame and one stored sync. The model-route fixture carries fixed terms and a sync this mount does not have. |
| The far sync is computed from a second `SimMount`. | Type its numbers. | Numbers typed by hand would be an oracle nobody can check; the simulator's mount is the truth the test already uses. |
| Five GoTos stay chained in one test. | One test per target. | Chaining carries backlash, wind and correction state between targets, which only this test and the night check. |
| 2 s for the cable-wind move. | 5 s. | The skeptic's corrected figure keeps steps near 7°; 5 s would be 17°. |
| 3 s for the aim test. | 6 s, with preview-less synthetic frames added to `loop_.rs`. | At 3 s the aim test still tells previews from frames by itself, and `loop_.rs` is not touched. |
| One in-process table test for the node settings; `cli/aim.rs` is the restart check. | Convert `cli/aim.rs` too. | It is the only test where the default binary pairs with a node over TCP. |
| scope-node's child helper times out. | Run the refusal with `--check-config`. | The test is about starting; with a timeout it still starts the node, and a regression fails instead of hanging. |
| Scratch helper in `support/mod.rs`. | A new `support/scratch.rs`; `tempfile`. | `layout` pins the six files of `support/`; `tempfile` would change `Cargo.lock`. |
| Duplicates as one story. | One story per crate. | The owner asked for one; it measures well under 64 KiB. |
| The optional trims are leaves. | Fold them into US4 and US2. | The owner decides them at the flip. |

**Findings left out**, because the handoff did not list them for this spec or the owner
said not now: trk-S1 (the readout test's centred start and thinned frames; overlaps US13),
simA-S3 (the fixture for three more GoTo tests; US3's helper would serve them later),
mcp-S4, mcp-D6, mcp-S6, node-S1, node-S2, node-S6, trk-S6, trk-S7, trk-S8, simA-S6,
simA-S7, api-S3, api-D2, api-S5, api-S6, api-S7, simA-D7, simA-S8, simA-S9, node-D5,
node-D6, node-D7, api-D5, api-D6, api-D13, mcp-D2, core-D9, core-D10, the hazards
mcp-H3 to mcp-H5, core-H1 to core-H4, api-H3, api-H4, trk-H2, trk-H3, trk-H5, node-H4,
node-H5, simA-H3 (no change proposed or a runner choice), and the profile findings PR 214
already applied. api-H1 was fixed by 027.

## Verification (every story, before proposing)

- **The factory gates** (`ergane.yaml`):
  - `lint` = `cargo fmt --all --check`,
    `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`,
    `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features --locked`
    and `RUSTDOCFLAGS="-D warnings" cargo doc -p finder --no-deps`;
  - `test` = `cargo nextest run --workspace --all-targets --all-features --locked`,
    `cargo test --workspace --locked --doc`, and then, in `crates/finder/web`,
    `npm ci --no-audit --no-fund`, `npm run typecheck`, `npm run format:check`,
    `npm test`, `npm run build`, `git status --porcelain -- dist | (! grep .)` and
    `CI=1 npm run e2e`.
- **CI in the merge queue** runs every required `ci.yml` job, including the cedar e2e
  eval. The coverage floor runs only by hand.

## Prerequisites before this spec is flipped to `ready`

1. **024 and 025 have landed.** The owner's order is 024, 025, then 028 and 029.
   `024-mount-link-recovery` is in `depends_on_landed`. 025 is not in this tree, so it is
   not there yet: when its spec lands on `main`, add it. When each has landed, re-read
   every citation of this trio on `main` and update the Refinement Record's
   `shared_writes`. Files this spec writes that they also write:
   `crates/scope-node/src/mount/mod.rs` (024 US1, which adds trait defaults and leaves the
   test module US10 deletes alone), `crates/finder/src/session.rs` (024 US3) and
   `crates/finder/tests/cli/main.rs` (025's draft). Files this spec cites that 024 edits,
   so their line numbers will move: `crates/scope-node/src/service.rs`,
   `crates/finder/src/goto.rs`, `crates/finder/src/tracking.rs`,
   `crates/finder/src/live.rs` and `crates/finder/src/api.rs`. Both add modules to
   finder's binaries, which this spec does not reorder.
2. **The owner keeps or deletes US11, US12 and US13**, and the approvals they carry.
3. **The owner approvals are given.** Flipping gives them.
4. **Round 2's probe has built every kept story** in a throwaway worktree, measured each
   diff with the factory's output check, and filled § Measurements' "After" column.
5. **`crates/finder/tests` is still as read.** If a file there changed since 06a7d4e,
   re-read it and re-run the probe.

## Plan traps

1. **Gate evidence or nothing.** The judge sees the diff, the scenarios and each gate's
   command and exit code, not the test output. *Check:* every Then clause names a
   committed test in the diff, or something read from the diff.
2. **A named command is read literally.** *Check:* scenarios name the nextest line of
   § Verification, word for word.
3. **A saving with no guard.** *Check:* each speed change in § Interfaces carries the
   assertion its scenario names, and the assertion fails before the change.
4. **The wrong controls.** In the finder rigs `rig.sim.stop_frames()` does nothing;
   `rig.controls` is the node's. In the simulator rig the call would be
   `rig.service.controls()`. *Check:* the newest-frame bound fails if the stop is removed.
5. **A centred start that waits.** *Check:* Track is posted within a few ticks of the
   sync, and its reply is asserted, not just the later state.
6. **Unused imports after a deletion.** *Check:* `cargo clippy` with `-D warnings`;
   `routes.rs` loses `ephemeris` and `sky`, `tracking/night.rs` loses `Instant`.
7. **A spec fixed mid-epic needs a relaunch.** *Check:* no builder edits
   `specs/028-finder-tests-do-less/`.
8. **Each worktree builds on its own.** *Check:* no story sets `CARGO_TARGET_DIR`.
9. **A new test file at the top of `crates/finder/tests` fails `layout`.** *Check:* the
   one new module is `console/session/durable.rs`, declared in its folder's `mod.rs`.
10. **The far sync must be the simulator's truth.** If US3's live sync opens a second
    epoch, the stored zero points or flank are wrong. *Check:* `second_from_file` asserts
    one epoch.
11. **Formatting is the repository's.** *Check:* `cargo fmt --all`, then
    `cargo fmt --all --check`.

## Risks

| Risk | Mitigation |
|---|---|
| US3's stored sync does not match what a solved frame would record (apparent altitude, flank). | It is computed from the simulator's mount, not typed; `second_from_file` asserts one epoch and a base tilt; round 2's probe builds it first. If it still fails, US3 escalates rather than loosening a bound. |
| A frame interval change takes one frame to apply. | Every count and cadence bound allows one frame. |
| 024 or 025 changes a file this spec cites. | Prerequisite 1: re-read on `main` after they land. |
| An optional story is deleted after round 2 built on it. | Nothing depends on an optional story; its files are its own or come after the stories it waits for. |
| US9 and US10 are long lists across crates. | Each is a list of small, mechanical edits; round 2 measures both diffs against the 64 KiB preferred size. |

## Refinement Record

Drafted on 2026-11-15 by the plate-solver operator against `main` at 06a7d4e. Round 1
drafts only: no story has been built. Writes come from the task text. Shared writes name
the landed story that last wrote each file, and, for a file two stories here write, the
later story under the earlier one. Nothing in this record is an owner approval: those are
in spec.md, and flipping to `ready` gives them.

```yaml
version: 1
reviewed_revision: 06a7d4e0ef488f227f7faf3407f5a6e5e5b405cc
stories:
  US1:
    tests:
    - path: crates/finder/tests/mcp/axis_moves.rs
      status: existing
      disposition: replace
      assertion: "six direct-move tests stop the camera and bound the newest frame id; the Track test starts centred and asserts tracking with no goto"
      decision: "spec.md:FR-001"
    - path: crates/simulator/tests/support/mod.rs
      status: existing
      disposition: replace
      assertion: "Rig::new builds on the synthetic camera with no database and asserts the node describes it as simulated"
      decision: "spec.md:FR-003"
    - path: crates/simulator/tests/frame_positions.rs
      status: existing
      disposition: preserve
      assertion: "its exposure and position assertions unchanged, on Rig::rendered"
      decision: "plan.md:Interfaces"
    preserved_behavior:
    - "every assertion the edited tests made before (spec.md US1-S1, US1-S2)"
    - "stop_and_watchdog.rs and watchdog_report.rs pass unchanged (spec.md US1-S4)"
    checks:
      development:
      - "cargo nextest run -p finder --locked --test mcp axis_moves::"
      - cargo nextest run -p simulator --locked
      - cargo fmt --all --check
      handoff: declared_gates
      independent: declared_pipeline
      release:
      - "release owner: the repository owner, through the merge queue: test, lint and the ci.yml jobs"
    writes:
    - crates/finder/tests/mcp/axis_moves.rs
    - crates/simulator/tests/support/mod.rs
    - crates/simulator/tests/frame_positions.rs
    no_write_reason: ""
    shared_writes:
    - path: crates/finder/tests/mcp/axis_moves.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "028 depends_on_landed 027-finder-tests-by-system, whose US2 last wrote this file"
    - path: crates/finder/tests/mcp/axis_moves.rs
      other_spec: 028-finder-tests-do-less
      other_story: US9
      disposition: merge_order
      reason: "US9 depends_on_merged US1 and moves this file's scratch folder"
    - path: crates/simulator/tests/support/mod.rs
      other_spec: 014-goto
      other_story: US2
      disposition: merge_order
      reason: "028 depends_on_landed 014-goto, whose US2 last wrote this file"
    - path: crates/simulator/tests/frame_positions.rs
      other_spec: 014-goto
      other_story: US2
      disposition: merge_order
      reason: "028 depends_on_landed 014-goto, whose US2 last wrote this file"
    open_questions: []
  US2:
    tests:
    - path: crates/finder/tests/motion/tracking/routes.rs
      status: existing
      disposition: replace
      assertion: "four tests reach tracking from a centred start and assert the Track reply is tracking with no goto"
      decision: "spec.md:FR-004"
    - path: crates/finder/tests/motion/tracking/min_alt.rs
      status: existing
      disposition: replace
      assertion: "centred start; Track reply tracking with no goto; off, at rest and too_low unchanged"
      decision: "spec.md:FR-005"
    - path: crates/finder/tests/motion/stop/motion_ends.rs
      status: existing
      disposition: replace
      assertion: "tracking() centres before the sync and asserts the command log holds no GoTo"
      decision: "spec.md:FR-006"
    - path: crates/finder/tests/motion/stop/retry.rs
      status: existing
      disposition: replace
      assertion: "centred before the sync; no GoTo in the command log when tracking starts; its M45 GoTo unchanged"
      decision: "spec.md:FR-006"
    - path: crates/finder/tests/motion/stop/watchdog.rs
      status: existing
      disposition: replace
      assertion: "starts 5 degrees below M42 and asserts within 6 degrees before the later GoTo; its final assertions unchanged"
      decision: "spec.md:FR-007"
    preserved_behavior:
    - "the three handoff tests and the readout test still reach tracking through a GoTo (spec.md US2-S5)"
    - "every refusal, Stop and watchdog assertion of the edited tests"
    checks:
      development:
      - "cargo nextest run -p finder --locked --test motion tracking::"
      - "cargo nextest run -p finder --locked --test motion stop::"
      - cargo fmt --all --check
      handoff: declared_gates
      independent: declared_pipeline
      release:
      - "release owner: the repository owner, through the merge queue: test, lint and the ci.yml jobs"
    writes:
    - crates/finder/tests/motion/tracking/routes.rs
    - crates/finder/tests/motion/tracking/min_alt.rs
    - crates/finder/tests/motion/stop/motion_ends.rs
    - crates/finder/tests/motion/stop/retry.rs
    - crates/finder/tests/motion/stop/watchdog.rs
    no_write_reason: ""
    shared_writes:
    - path: crates/finder/tests/motion/tracking/routes.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "028 depends_on_landed 027-finder-tests-by-system, whose US2 last wrote this file"
    - path: crates/finder/tests/motion/tracking/routes.rs
      other_spec: 028-finder-tests-do-less
      other_story: US13
      disposition: merge_order
      reason: "US13 depends_on_merged US2 and edits the readout test in this file"
    - path: crates/finder/tests/motion/tracking/min_alt.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "028 depends_on_landed 027-finder-tests-by-system, whose US2 last wrote this file"
    - path: crates/finder/tests/motion/stop/motion_ends.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "028 depends_on_landed 027-finder-tests-by-system, whose US2 last wrote this file"
    - path: crates/finder/tests/motion/stop/retry.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "028 depends_on_landed 027-finder-tests-by-system, whose US2 last wrote this file"
    - path: crates/finder/tests/motion/stop/watchdog.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "028 depends_on_landed 027-finder-tests-by-system, whose US2 last wrote this file"
    open_questions: []
  US3:
    tests:
    - path: crates/finder/tests/motion/goto/sim.rs
      status: existing
      disposition: replace
      assertion: "five-target test starts from the stored far sync with 4500 azimuth counts, a base tilt and one epoch, then its five GoTos unchanged; cable-wind move advances the frame id by 20 to 60"
      decision: "spec.md:FR-009"
    preserved_behavior:
    - "the five GoTos, their order, their progress states and bounds (spec.md US3-S2)"
    - "every other test using Rig::new is unchanged (spec.md US3-S4)"
    checks:
      development:
      - "cargo nextest run -p finder --locked --test motion goto::"
      - cargo fmt --all --check
      handoff: declared_gates
      independent: declared_pipeline
      release:
      - "release owner: the repository owner, through the merge queue: test, lint and the ci.yml jobs"
    writes:
    - crates/finder/tests/support/goto.rs
    - crates/finder/tests/motion/goto/sim.rs
    no_write_reason: ""
    shared_writes:
    - path: crates/finder/tests/support/goto.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "028 depends_on_landed 027-finder-tests-by-system, whose US2 last wrote this file"
    - path: crates/finder/tests/support/goto.rs
      other_spec: 028-finder-tests-do-less
      other_story: US8
      disposition: merge_order
      reason: "US8 depends_on_merged US3 and changes this rig's solver options"
    - path: crates/finder/tests/motion/goto/sim.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "028 depends_on_landed 027-finder-tests-by-system, whose US2 last wrote this file"
    - path: crates/finder/tests/motion/goto/sim.rs
      other_spec: 028-finder-tests-do-less
      other_story: US9
      disposition: merge_order
      reason: "US9 depends_on_merged US3 and moves this file's scratch folder"
    open_questions: []
  US4:
    tests:
    - path: crates/finder/tests/motion/night.rs
      status: existing
      disposition: replace
      assertion: "sets 500 ms on the simulator and asserts 0.5 s per frame id after the first sync; the night unchanged"
      decision: "spec.md:FR-012"
    preserved_behavior:
    - "every step and assertion of the whole night (spec.md US4-S2)"
    checks:
      development:
      - "cargo nextest run -p finder --locked --test motion night::"
      - cargo nextest run -p simulator --locked
      - cargo fmt --all --check
      handoff: declared_gates
      independent: declared_pipeline
      release:
      - "release owner: the repository owner, through the merge queue: test, lint and the ci.yml jobs"
    writes:
    - crates/simulator/src/controls.rs
    - crates/finder/tests/motion/night.rs
    no_write_reason: ""
    shared_writes:
    - path: crates/simulator/src/controls.rs
      other_spec: 014-goto
      other_story: US6
      disposition: merge_order
      reason: "028 depends_on_landed 014-goto, whose US6 last wrote this file"
    - path: crates/finder/tests/motion/night.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "028 depends_on_landed 027-finder-tests-by-system, whose US2 last wrote this file"
    - path: crates/finder/tests/motion/night.rs
      other_spec: 028-finder-tests-do-less
      other_story: US8
      disposition: merge_order
      reason: "US8 depends_on_merged US4 and changes this file's solver options"
    open_questions: []
  US5:
    tests:
    - path: crates/finder/tests/cli/track.rs
      status: existing
      disposition: replace
      assertion: "tracks in paused time in process from a centred start, then starts the built binary once; no track and no motion after the restart"
      decision: "spec.md:FR-013"
    preserved_behavior:
    - "no track in the state file or the restarted session, 2 s without motion, eyepiece unchanged (spec.md US5-S2)"
    - "imaging_camera_survives_restart_and_null_clears_it is unchanged"
    checks:
      development:
      - "cargo nextest run -p finder --locked --test cli track::"
      - cargo fmt --all --check
      handoff: declared_gates
      independent: declared_pipeline
      release:
      - "release owner: the repository owner, through the merge queue: test, lint and the ci.yml jobs"
    writes:
    - crates/finder/tests/cli/track.rs
    no_write_reason: ""
    shared_writes:
    - path: crates/finder/tests/cli/track.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "028 depends_on_landed 027-finder-tests-by-system, whose US2 last wrote this file"
    - path: crates/finder/tests/cli/track.rs
      other_spec: 028-finder-tests-do-less
      other_story: US9
      disposition: merge_order
      reason: "US9 depends_on_merged US5 and moves this file's scratch folder"
    open_questions: []
  US6:
    tests:
    - path: crates/finder/tests/console/session/durable.rs
      status: new
      disposition: add
      assertion: "durable fields at exposures 200 and 2000 ms survive reopening the state file; mode 0600; stale .tmp gone; token absent from the view"
      decision: "spec.md:FR-014"
    - path: crates/finder/tests/cli/session_node.rs
      status: existing
      disposition: replace
      assertion: "deleted; its checks are the new in-process test's and cli/aim.rs's stderr check"
      decision: "spec.md:FR-015"
    - path: crates/finder/tests/cli/aim.rs
      status: existing
      disposition: replace
      assertion: "the built binary's stderr never contains the node token; the restart and reset assertions unchanged"
      decision: "spec.md:FR-016"
    - path: crates/finder/tests/cli/backlash.rs
      status: existing
      disposition: replace
      assertion: "after two restarts the fixed reset posted to the binary leaves no backlash in the reply, the session or the state file"
      decision: "spec.md:FR-017"
    - path: crates/finder/tests/motion/tracking/backlash.rs
      status: existing
      disposition: replace
      assertion: "first_transit_measures_then_fixed_reset_removes_the_observed_value is deleted; its other tests unchanged"
      decision: "spec.md:FR-017"
    preserved_behavior:
    - "cli/session.rs keeps --state precedence and the binary's real folders"
    - "another_target_on_the_same_mount_takes_up_the_measured_altitude_play still measures in the running session"
    checks:
      development:
      - "cargo nextest run -p finder --locked --test console session::"
      - cargo nextest run -p finder --locked --test cli
      - "cargo nextest run -p finder --locked --test motion tracking::backlash::"
      - cargo fmt --all --check
      handoff: declared_gates
      independent: declared_pipeline
      release:
      - "release owner: the repository owner, through the merge queue: test, lint and the ci.yml jobs"
    writes:
    - crates/finder/tests/console/session/durable.rs
    - crates/finder/tests/console/session/mod.rs
    - crates/finder/tests/cli/session_node.rs
    - crates/finder/tests/cli/main.rs
    - crates/finder/tests/cli/aim.rs
    - crates/finder/tests/cli/backlash.rs
    - crates/finder/tests/motion/tracking/backlash.rs
    no_write_reason: ""
    shared_writes:
    - path: crates/finder/tests/console/session/mod.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "028 depends_on_landed 027-finder-tests-by-system, whose US2 last wrote this file"
    - path: crates/finder/tests/cli/session_node.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "028 depends_on_landed 027-finder-tests-by-system, whose US2 last wrote this file"
    - path: crates/finder/tests/cli/main.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "028 depends_on_landed 027-finder-tests-by-system, whose US2 last wrote this file"
    - path: crates/finder/tests/cli/aim.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "028 depends_on_landed 027-finder-tests-by-system, whose US2 last wrote this file"
    - path: crates/finder/tests/cli/aim.rs
      other_spec: 028-finder-tests-do-less
      other_story: US9
      disposition: merge_order
      reason: "US9 depends_on_merged US6 and moves this file's scratch folder"
    - path: crates/finder/tests/cli/backlash.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "028 depends_on_landed 027-finder-tests-by-system, whose US2 last wrote this file"
    - path: crates/finder/tests/cli/backlash.rs
      other_spec: 028-finder-tests-do-less
      other_story: US9
      disposition: merge_order
      reason: "US9 depends_on_merged US6 and moves this file's scratch folder"
    - path: crates/finder/tests/motion/tracking/backlash.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "028 depends_on_landed 027-finder-tests-by-system, whose US2 last wrote this file"
    - path: crates/finder/tests/motion/tracking/backlash.rs
      other_spec: 028-finder-tests-do-less
      other_story: US9
      disposition: merge_order
      reason: "US9 depends_on_merged US6 and moves this file's scratch folder"
    open_questions: []
  US7:
    tests:
    - path: crates/finder/tests/console/aim.rs
      status: existing
      disposition: replace
      assertion: "node at 3 s; consecutive preview ids differ by exactly 2; retention assertions unchanged"
      decision: "spec.md:FR-019"
    - path: crates/finder/tests/console/live/loop_.rs
      status: existing
      disposition: replace
      assertion: "node at 5 s; at most 7 frames; unsolved interval at least 30 s"
      decision: "spec.md:FR-020"
    preserved_behavior:
    - "state keeps its signature and 500 ms for its other callers (spec.md US7-S3)"
    checks:
      development:
      - cargo nextest run -p finder --locked --test console
      - cargo fmt --all --check
      handoff: declared_gates
      independent: declared_pipeline
      release:
      - "release owner: the repository owner, through the merge queue: test, lint and the ci.yml jobs"
    writes:
    - crates/finder/tests/support/live.rs
    - crates/finder/tests/console/aim.rs
    - crates/finder/tests/console/live/loop_.rs
    no_write_reason: ""
    shared_writes:
    - path: crates/finder/tests/support/live.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "028 depends_on_landed 027-finder-tests-by-system, whose US2 last wrote this file"
    - path: crates/finder/tests/console/aim.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "028 depends_on_landed 027-finder-tests-by-system, whose US2 last wrote this file"
    - path: crates/finder/tests/console/live/loop_.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "028 depends_on_landed 027-finder-tests-by-system, whose US2 last wrote this file"
    open_questions: []
  US8:
    tests:
    - path: crates/finder/tests/motion/tracking/night.rs
      status: existing
      disposition: replace
      assertion: "the hour's assertions without its wall-clock bound, under the name an_hour_covers_transit_too_low_and_keyhole"
      decision: "spec.md:FR-021"
    - path: crates/finder/tests/cli/simulate.rs
      status: existing
      disposition: replace
      assertion: "the real-rate check compares against a time read just before its 5 s window"
      decision: "spec.md:FR-023"
    - path: crates/scope-node/tests/cli.rs
      status: existing
      disposition: replace
      assertion: "a child still running after 10 s is killed and fails the test; both tests' assertions unchanged"
      decision: "spec.md:FR-024"
    - path: crates/scope-node/src/rpicam/metadata.rs
      status: existing
      disposition: replace
      assertion: "the three metadata tests run on a paused clock with their assertions unchanged"
      decision: "spec.md:FR-025"
    preserved_behavior:
    - "every assertion of the hour of tracking except its wall-clock bound (spec.md US8-S1)"
    - "the rigs' solves and every test that uses them; only the deadline changes (spec.md US8-S2)"
    checks:
      development:
      - cargo nextest run -p finder --locked --all-targets
      - cargo nextest run -p scope-node --locked --all-targets
      - cargo fmt --all --check
      handoff: declared_gates
      independent: declared_pipeline
      release:
      - "release owner: the repository owner, through the merge queue: test, lint and the ci.yml jobs"
    writes:
    - crates/finder/tests/motion/tracking/night.rs
    - crates/finder/tests/support/goto.rs
    - crates/finder/tests/support/mcp.rs
    - crates/finder/tests/support/stop.rs
    - crates/finder/tests/motion/night.rs
    - crates/finder/tests/cli/simulate.rs
    - crates/scope-node/tests/cli.rs
    - crates/scope-node/src/rpicam/metadata.rs
    no_write_reason: ""
    shared_writes:
    - path: crates/finder/tests/motion/tracking/night.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "028 depends_on_landed 027-finder-tests-by-system, whose US2 last wrote this file"
    - path: crates/finder/tests/motion/tracking/night.rs
      other_spec: 028-finder-tests-do-less
      other_story: US9
      disposition: merge_order
      reason: "US9 depends_on_merged US8 and moves this file's scratch folder"
    - path: crates/finder/tests/support/mcp.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "028 depends_on_landed 027-finder-tests-by-system, whose US2 last wrote this file"
    - path: crates/finder/tests/support/stop.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "028 depends_on_landed 027-finder-tests-by-system, whose US2 last wrote this file"
    - path: crates/finder/tests/motion/night.rs
      other_spec: 028-finder-tests-do-less
      other_story: US11
      disposition: merge_order
      reason: "US11 depends_on_merged US8 and changes the night's GoTos"
    - path: crates/finder/tests/cli/simulate.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "028 depends_on_landed 027-finder-tests-by-system, whose US2 last wrote this file"
    - path: crates/finder/tests/cli/simulate.rs
      other_spec: 028-finder-tests-do-less
      other_story: US9
      disposition: merge_order
      reason: "US9 depends_on_merged US8 and moves this file's scratch folder"
    - path: crates/scope-node/tests/cli.rs
      other_spec: 013-live-finder
      other_story: US2
      disposition: merge_order
      reason: "028 depends_on_landed 013-live-finder, whose US2 last wrote this file"
    - path: crates/scope-node/tests/cli.rs
      other_spec: 028-finder-tests-do-less
      other_story: US9
      disposition: merge_order
      reason: "US9 depends_on_merged US8 and moves this file's scratch file"
    open_questions: []
  US9:
    tests:
    - path: crates/finder/tests/console/targets.rs
      status: existing
      disposition: replace
      assertion: "the child's working folder is still asserted empty after the child ran"
      decision: "spec.md:FR-027"
    - path: crates/finder/tests/layout/main.rs
      status: existing
      disposition: preserve
      assertion: "support/ still holds exactly its six files and no test"
      decision: "spec.md:US9"
    preserved_behavior:
    - "every assertion of every test whose scratch folder moves (spec.md FR-027)"
    - "the layout tests pass unchanged (spec.md US9-S5)"
    checks:
      development:
      - cargo nextest run -p finder --locked --all-targets
      - cargo nextest run -p scope-node --locked --all-targets
      - cargo test -p finder --locked --all-targets
      - cargo fmt --all --check
      handoff: declared_gates
      independent: declared_pipeline
      release:
      - "release owner: the repository owner, through the merge queue: test, lint and the ci.yml jobs"
    writes:
    - crates/finder/tests/support/mod.rs
    - crates/finder/tests/cli/aim.rs
    - crates/finder/tests/cli/session.rs
    - crates/finder/tests/cli/backlash.rs
    - crates/finder/tests/cli/simulate.rs
    - crates/finder/tests/cli/track.rs
    - crates/finder/tests/console/session/writes.rs
    - crates/finder/tests/console/targets.rs
    - crates/finder/tests/console/session/edits.rs
    - crates/finder/tests/mcp/endpoint.rs
    - crates/finder/tests/mcp/motion.rs
    - crates/finder/tests/mcp/axis_moves.rs
    - crates/finder/tests/motion/model/sim.rs
    - crates/finder/tests/motion/model/routes.rs
    - crates/finder/tests/motion/tracking/backlash.rs
    - crates/finder/tests/motion/tracking/keyhole.rs
    - crates/finder/tests/motion/tracking/night.rs
    - crates/finder/tests/motion/goto/sim.rs
    - crates/scope-node/tests/cli.rs
    - crates/scope-node/tests/mount_config.rs
    - crates/scope-node/tests/rpicam.rs
    no_write_reason: ""
    shared_writes:
    - path: crates/finder/tests/support/mod.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "028 depends_on_landed 027-finder-tests-by-system, whose US2 last wrote this file"
    - path: crates/finder/tests/cli/session.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "028 depends_on_landed 027-finder-tests-by-system, whose US2 last wrote this file"
    - path: crates/finder/tests/console/session/writes.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "028 depends_on_landed 027-finder-tests-by-system, whose US2 last wrote this file"
    - path: crates/finder/tests/console/targets.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "028 depends_on_landed 027-finder-tests-by-system, whose US2 last wrote this file"
    - path: crates/finder/tests/console/session/edits.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "028 depends_on_landed 027-finder-tests-by-system, whose US2 last wrote this file"
    - path: crates/finder/tests/mcp/endpoint.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "028 depends_on_landed 027-finder-tests-by-system, whose US2 last wrote this file"
    - path: crates/finder/tests/mcp/motion.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "028 depends_on_landed 027-finder-tests-by-system, whose US2 last wrote this file"
    - path: crates/finder/tests/mcp/motion.rs
      other_spec: 028-finder-tests-do-less
      other_story: US10
      disposition: merge_order
      reason: "US10 depends_on_merged US9 and changes this file's Stop helper"
    - path: crates/finder/tests/motion/model/sim.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "028 depends_on_landed 027-finder-tests-by-system, whose US2 last wrote this file"
    - path: crates/finder/tests/motion/model/sim.rs
      other_spec: 028-finder-tests-do-less
      other_story: US10
      disposition: merge_order
      reason: "US10 depends_on_merged US9 and removes a duplicate in this file"
    - path: crates/finder/tests/motion/model/routes.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "028 depends_on_landed 027-finder-tests-by-system, whose US2 last wrote this file"
    - path: crates/finder/tests/motion/model/routes.rs
      other_spec: 028-finder-tests-do-less
      other_story: US10
      disposition: merge_order
      reason: "US10 depends_on_merged US9 and narrows a loop in this file"
    - path: crates/finder/tests/motion/tracking/keyhole.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "028 depends_on_landed 027-finder-tests-by-system, whose US2 last wrote this file"
    - path: crates/scope-node/tests/mount_config.rs
      other_spec: 014-goto
      other_story: US3
      disposition: merge_order
      reason: "028 depends_on_landed 014-goto, whose US3 last wrote this file"
    - path: crates/scope-node/tests/rpicam.rs
      other_spec: 013-live-finder
      other_story: US2
      disposition: merge_order
      reason: "028 depends_on_landed 013-live-finder, whose US2 last wrote this file"
    open_questions: []
  US10:
    tests:
    - path: crates/grpc-service/tests/service_acceptance.rs
      status: existing
      disposition: replace
      assertion: "three tests that a stricter twin covers are deleted; the rest unchanged"
      decision: "spec.md:FR-029"
    - path: crates/plate-solver/tests/solve_acceptance.rs
      status: existing
      disposition: replace
      assertion: "two tests deleted; the getter check and the empty-catalog NO_MATCH kept under honest names"
      decision: "spec.md:FR-030"
    - path: crates/database-generation/src/patterns.rs
      status: existing
      disposition: replace
      assertion: "the five-element order test deleted; the seven-element test moved into tests unchanged"
      decision: "spec.md:FR-031"
    - path: crates/simulator/tests/mount.rs
      status: existing
      disposition: replace
      assertion: "the 0.01 degree backlash test deleted; its 1 arcsecond twin stays"
      decision: "spec.md:FR-032"
    - path: crates/scope-node/tests/skywatcher.rs
      status: existing
      disposition: replace
      assertion: "stop_all_stops_both_axes: both axis stops sent, both axes at rest"
      decision: "spec.md:FR-032"
    - path: crates/scope-node/src/mount/mod.rs
      status: existing
      disposition: replace
      assertion: "the test of a fake's own behaviour is deleted with the fake"
      decision: "spec.md:FR-032"
    - path: crates/finder/src/session.rs
      status: existing
      disposition: replace
      assertion: "two inline tests covered exactly by the route test are deleted"
      decision: "spec.md:FR-033"
    - path: crates/finder/tests/motion/model/sim.rs
      status: existing
      disposition: replace
      assertion: "the durable night asserts the base tilt within 1 arcminute; the standalone second-sync test is deleted"
      decision: "spec.md:FR-034"
    - path: crates/finder/tests/motion/goto/routes.rs
      status: existing
      disposition: replace
      assertion: "four refusal tests with stricter copies deleted; foreign 403 and simulate startup stay"
      decision: "spec.md:FR-034"
    - path: crates/finder/tests/motion/goto/refusals.rs
      status: existing
      disposition: replace
      assertion: "the no-mount refusal's error names no mount"
      decision: "spec.md:FR-034"
    - path: crates/finder/tests/motion/model/routes.rs
      status: existing
      disposition: replace
      assertion: "the foreign-origin reset loop runs for fixed only, 403 and unchanged model"
      decision: "spec.md:FR-034"
    - path: crates/finder/tests/mcp/motion.rs
      status: existing
      disposition: replace
      assertion: "nine Stop cases assert StopAll within 0.1 s, halt reason, motion cleared and rest within 2 s; the second-client case alone watches 5 s and only-StopAll"
      decision: "spec.md:FR-035"
    preserved_behavior:
    - "no oracle file is edited (spec.md SC-002)"
    - "each removed assertion is held by the stricter twin plan.md Interfaces US10 names"
    checks:
      development:
      - cargo nextest run --workspace --locked --all-targets --all-features
      - cargo fmt --all --check
      handoff: declared_gates
      independent: declared_pipeline
      release:
      - "release owner: the repository owner, through the merge queue: test, lint and the ci.yml jobs"
    writes:
    - crates/grpc-service/tests/service_acceptance.rs
    - crates/plate-solver/tests/solve_acceptance.rs
    - crates/database-generation/src/patterns.rs
    - crates/simulator/tests/mount.rs
    - crates/scope-node/tests/skywatcher.rs
    - crates/scope-node/src/mount/mod.rs
    - crates/finder/src/session.rs
    - crates/finder/tests/motion/model/sim.rs
    - crates/finder/tests/motion/goto/routes.rs
    - crates/finder/tests/motion/goto/refusals.rs
    - crates/finder/tests/motion/model/routes.rs
    - crates/finder/tests/mcp/motion.rs
    no_write_reason: ""
    shared_writes:
    - path: crates/simulator/tests/mount.rs
      other_spec: 014-goto
      other_story: US1
      disposition: merge_order
      reason: "028 depends_on_landed 014-goto, whose US1 last wrote this file"
    - path: crates/scope-node/tests/skywatcher.rs
      other_spec: 014-goto
      other_story: US3
      disposition: merge_order
      reason: "028 depends_on_landed 014-goto, whose US3 last wrote this file"
    - path: crates/scope-node/src/mount/mod.rs
      other_spec: 017-console-stop-confirm
      other_story: US3
      disposition: merge_order
      reason: "028 depends_on_landed 017-console-stop-confirm, whose US3 last wrote this file at the reviewed revision"
    - path: crates/scope-node/src/mount/mod.rs
      other_spec: 024-mount-link-recovery
      other_story: US1
      disposition: merge_order
      reason: "028 depends_on_landed 024-mount-link-recovery, whose US1 adds trait defaults to this file before US10 deletes its test module"
    - path: crates/finder/src/session.rs
      other_spec: 017-console-stop-confirm
      other_story: US2
      disposition: merge_order
      reason: "028 depends_on_landed 017-console-stop-confirm, whose US2 last wrote this file at the reviewed revision"
    - path: crates/finder/src/session.rs
      other_spec: 024-mount-link-recovery
      other_story: US3
      disposition: merge_order
      reason: "028 depends_on_landed 024-mount-link-recovery, whose US3 edits this file before US10 deletes two of its inline tests"
    - path: crates/finder/tests/motion/goto/routes.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "028 depends_on_landed 027-finder-tests-by-system, whose US2 last wrote this file"
    - path: crates/finder/tests/motion/goto/refusals.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "028 depends_on_landed 027-finder-tests-by-system, whose US2 last wrote this file"
    open_questions: []
  US11:
    tests:
    - path: crates/finder/tests/motion/night.rs
      status: existing
      disposition: replace
      assertion: "one GoTo to M81 after the one-tap second sync, then the hand push, Stop and dropped link with their assertions"
      decision: "spec.md:FR-036"
    preserved_behavior:
    - "the production simulate wiring, the one-tap second sync and a cross-sky GoTo on it (spec.md US11-S1)"
    checks:
      development:
      - "cargo nextest run -p finder --locked --test motion night::"
      - cargo fmt --all --check
      handoff: declared_gates
      independent: declared_pipeline
      release:
      - "release owner: the repository owner, through the merge queue: test, lint and the ci.yml jobs"
    writes:
    - crates/finder/tests/motion/night.rs
    no_write_reason: ""
    shared_writes: []
    open_questions: []
  US12:
    tests:
    - path: crates/finder/tests/mcp/tracking.rs
      status: existing
      disposition: replace
      assertion: "75 s hold sampled every second; MCP attribution, M42 within 1 arcminute, more than 30 samples and readout parity unchanged"
      decision: "spec.md:FR-037"
    preserved_behavior:
    - "every assertion of the hold except its span (spec.md US12-S1)"
    checks:
      development:
      - "cargo nextest run -p finder --locked --test mcp tracking::"
      - cargo fmt --all --check
      handoff: declared_gates
      independent: declared_pipeline
      release:
      - "release owner: the repository owner, through the merge queue: test, lint and the ci.yml jobs"
    writes:
    - crates/finder/tests/mcp/tracking.rs
    no_write_reason: ""
    shared_writes:
    - path: crates/finder/tests/mcp/tracking.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "028 depends_on_landed 027-finder-tests-by-system, whose US2 last wrote this file"
    open_questions: []
  US13:
    tests:
    - path: crates/finder/tests/motion/tracking/tracker.rs
      status: existing
      disposition: replace
      assertion: "the fed sample is held at 59.9 s and gone at 60.1 s"
      decision: "spec.md:FR-038"
    - path: crates/finder/tests/motion/tracking/routes.rs
      status: existing
      disposition: replace
      assertion: "the readout tracks until more than 30 samples, keeps its RMS and ephemeris assertions, and proves the window on one injected sample"
      decision: "spec.md:FR-038"
    preserved_behavior:
    - "the readout's RMS equality, field rotation and longest exposure (spec.md US13-S2)"
    checks:
      development:
      - "cargo nextest run -p finder --locked --test motion tracking::"
      - cargo fmt --all --check
      handoff: declared_gates
      independent: declared_pipeline
      release:
      - "release owner: the repository owner, through the merge queue: test, lint and the ci.yml jobs"
    writes:
    - crates/finder/tests/motion/tracking/tracker.rs
    - crates/finder/tests/motion/tracking/routes.rs
    no_write_reason: ""
    shared_writes:
    - path: crates/finder/tests/motion/tracking/tracker.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "028 depends_on_landed 027-finder-tests-by-system, whose US2 last wrote this file"
    open_questions: []
```
