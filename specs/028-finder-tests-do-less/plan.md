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
- **US3, cheaper GoTo setups:** the five-target test loads the two syncs of its old setup
  from a state file and makes only its first 1° move; the cable-wind test samples its
  350° move every 2 s.
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
| US8 | `crates/finder/tests/motion/tracking/night.rs`, `crates/finder/tests/support/goto.rs`, `crates/finder/tests/support/mcp.rs`, `crates/finder/tests/support/stop.rs`, `crates/finder/tests/motion/night.rs`, `crates/finder/tests/motion/mount_identity.rs`, `crates/finder/tests/cli/simulate.rs`, `crates/scope-node/tests/cli.rs`, `crates/scope-node/src/rpicam/metadata.rs` |
| US9 | `crates/finder/tests/support/mod.rs` and the twenty-six files of § Interfaces "US9" |
| US10 | the twelve files of § Interfaces "US10" |
| US11 | `crates/finder/tests/motion/night.rs` |
| US12 | `crates/finder/tests/mcp/tracking.rs` |
| US13 | `crates/finder/tests/motion/tracking/tracker.rs`, `crates/finder/tests/motion/tracking/routes.rs` |

## Interfaces

Line numbers are at 9d8cc4e, where 025's US1 and US2 landed, except where a line says it
is as an earlier story here leaves the file. A story re-reads each line before it edits
it; the text is given as well as the number. Each story's list includes what the
round-2 and round-3 probes of it had to do differently from the earlier text
(§ Refinement Record).

### Shared helpers this plan names

- **The newest frame id (MCP rig).** `LinkManager::frames()`
  (`crates/finder/src/link.rs:250`) is a watch that every frame from the node replaces
  (`crates/finder/src/link.rs:537`). Its `id` is the node's capture counter, which counts
  every capture, solved or not. `crates/finder/tests/mcp/frames.rs:180` already reads it.
  US1 adds to `crates/finder/tests/mcp/axis_moves.rs`:

  ```rust
  /// The id of the newest frame the console's link holds.
  fn newest_frame(rig: &Rig) -> u64 {
      rig.state
          .link()
          .unwrap()
          .frames()
          .borrow()
          .as_ref()
          .map(|f| f.id)
          .unwrap()
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
  In `sunrise_halts_with_the_page_guard_reason_within_one_second` alone the binding is
  `let last_frame = camera_off(&rig);`, and its bound reads `last_frame + 1`, because
  `stopped` is already its stop time (`crates/finder/tests/mcp/axis_moves.rs:402`,
  `let stopped = stop_time(&rig, start);`).
  `rig.controls` is the node's own controls (`crates/finder/tests/support/mcp.rs:218`);
  `rig.sim.stop_frames()` would do nothing here, because the node was built with its own
  `NodeControls` (`crates/finder/tests/support/mcp.rs:214`). With frames stopped the
  capture loop waits before its next capture (`crates/scope-node/src/service.rs:240`), so
  nothing is rendered.

- **A centred start.** As `already_centred_track_is_immediate_and_repeated_on_is_idempotent`
  does (`crates/finder/tests/motion/tracking/routes.rs:383`): point the eyepiece at M42
  by hand before the first sync (`crates/finder/tests/motion/tracking/routes.rs:386`,
  `rig.sim.point_eyepiece_at(target.ra_deg, target.dec_deg);`), sync, then tick (at most
  10 ticks) until the session's `pointing` is within 1′ of M42 (`separation`), and post
  Track. The node starts capturing in `Rig::new`, so its first frame, and the first
  sync, come from the start direction, 28.7° from M42. Frame 2, solved at about 0.8 s,
  is the first on M42. Track then finds the pointing within 1′ of the target
  (`crates/finder/src/session.rs:844`) and begins at once
  (`crates/finder/src/session.rs:868`): its reply reads `tracking` and holds no `goto`,
  because the session shows `goto` only after a GoTo was requested. Posted before that
  frame has solved, Track finds the start direction and arms a GoTo. The wait ends well
  inside the about 4 s in which M42 drifts out of 1′ at a fixed mount. This is the one
  recipe for every centred start (US1-S3, the four US2 sites and US5); each site writes
  the wait in its own file, right after its sync and the connected or at-rest check that
  follows the sync, because the rigs' shared files are not those stories' writes.
  `separation` is `finder::model::separation`.

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
     with the centred start of "Shared helpers": `Rig::new(START, 50.0, 120.0, true)`,
     point the eyepiece at M42 (`finder::targets::find("M42")`, written in full),
     `rig.sync()`, the connected check `ready()` makes, then the wait of the recipe,
     reading `session(&rig.app).await["pointing"]`. After the page's Track request,
     assert `session(&rig.app).await["track"]["state"] == "tracking"` and that the
     session has no `goto`, before the existing wait loop. Everything after is unchanged.

   Why it loses nothing: these tests assert axis counts, command times, heartbeats and
   refusals. The cable-limit refusals are re-checked from axis counts every 50 ms during
   a direct move (`crates/finder/src/mcp/guard.rs:155`), and the wind is observed from
   counts there too (`crates/finder/src/goto.rs:750`). Heartbeats do not wait for frames.
   With the camera off the model stays the first sync's, which makes the cable margins
   more repeatable, not less.

2. In `crates/simulator/tests/support/mod.rs` the rig (`crates/simulator/tests/support/mod.rs:96`)
   becomes:
   - `Rig::new()`: a `VirtualClock` at the same start, a `SimMount` on it with the same
     configuration, and `scope_node::synthetic::SyntheticCamera::new(Arc::new(Clock(clock.clone())))`,
     the camera `crates/simulator/tests/stop_finishes.rs:101` builds its service over,
     with the same settings. No `Simulator` and no pattern database. Before it returns it
     asserts
     `rig.camera.describe(DescribeRequest {}).await.unwrap().into_inner().model == "simulated"`,
     on `let mut rig`, since the client call takes `&mut`;
     the synthetic camera's model is `crates/scope-node/src/synthetic.rs:34`, the catalog
     camera's is `crates/simulator/src/camera.rs:79`.
   - `Rig::rendered()`: what `Rig::new()` builds today, on `SimCamera` and the database
     (`crates/simulator/tests/support/mod.rs:24`).
   - Both call one private `async fn on<C: Camera + 'static>(camera: C, clock: VirtualClock, mount: SimMount) -> Self`,
     the rest of today's constructor, which still builds the node with
     `NodeService::with_mount`. The imports gain `synthetic::SyntheticCamera` and
     `Camera` from `scope_node`.
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
       // Track reads the newest solved pointing, and the rig's first frame was
       // exposed before the push: wait for the first one solved on M42.
       for _ in 0..10 {
           let p = rig.view().await["pointing"].clone();
           let (ra, dec) = (
               p["ra_deg"].as_f64().unwrap(),
               p["dec_deg"].as_f64().unwrap(),
           );
           if separation(ra, dec, target.ra_deg, target.dec_deg) < 1.0 / 60.0 {
               break;
           }
           rig.tick().await;
       }
       let body = track(&rig, true).await;
       assert_eq!(body["track"]["state"], "tracking", "{body}");
       assert!(
           body.get("goto").is_none(),
           "a centred Track ran a GoTo: {body}"
       );
       assert_eq!(body["track"]["target"]["messier"], "M42", "{body}");
       assert_eq!(body["track"]["wanted"], true, "{body}");
       moving(&rig).await;
       rig
   }
   ```

   The last two assertions are the two of `tracking()`'s checks
   (`crates/finder/tests/motion/tracking/routes.rs:61` and the line after) that are not
   about the handoff; round 3 saw each fail all four tests under a mutation.

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
   The wait after the connected check is the centred-start recipe of "Shared helpers".
2. `crates/finder/tests/motion/tracking/min_alt.rs`: in
   `raised_minimum_altitude_ends_running_tracking_as_too_low`
   (`crates/finder/tests/motion/tracking/min_alt.rs:28`), point the eyepiece at M42
   between `Rig::new` (`crates/finder/tests/motion/tracking/min_alt.rs:29`) and the sync,
   wait after the sync as the centred-start recipe says, reading `rig.view()`, and assert
   the Track reply (`crates/finder/tests/motion/tracking/min_alt.rs:31`, whose reply is
   discarded today) reads `tracking` with no `goto`. The poll for `tracking` then ends
   at once; the rest is unchanged.
3. `crates/finder/tests/motion/stop/motion_ends.rs`: `tracking()`
   (`crates/finder/tests/motion/stop/motion_ends.rs:282`) builds its rig as `ready()`
   (`crates/finder/tests/motion/stop/motion_ends.rs:18`) does, but points the eyepiece
   at M42 before the sync, and after the sync and `ready()`'s at-rest check waits as the
   centred-start recipe says, reading `session(&rig.app)`; `ready()` itself is shared with
   tests that need a slew and does not change. After the wait for `tracking`, assert that
   `rig.commands` holds no `MountCommand::GoTo`. The azimuth-moving check
   (`crates/finder/tests/motion/stop/motion_ends.rs:294`) still holds: M42's azimuth
   rate is not zero, and the tracker sends its first rate at once.
4. `crates/finder/tests/motion/stop/retry.rs`: in
   `retained_tracker_cannot_delay_goto_cancellation_or_first_attempt`
   (`crates/finder/tests/motion/stop/retry.rs:198`), point the eyepiece at M42 between
   `Rig::new` (`crates/finder/tests/motion/stop/retry.rs:199`) and the sync, wait after
   the sync as the centred-start recipe says, and after the wait for `tracking` assert
   that `rig.commands` holds no `MountCommand::GoTo`. Its later GoTo to M45 is unchanged.
5. `crates/finder/tests/motion/stop/watchdog.rs`: in
   `past_watchdog_firing_is_the_baseline_for_a_successful_later_goto`
   (`crates/finder/tests/motion/stop/watchdog.rs:157`), replace `ready()`
   (`crates/finder/tests/motion/stop/watchdog.rs:158`) with `Rig::new()`, a push of the
   mount to 5° below M42 (as `crates/finder/tests/motion/tracking/routes.rs:121` does,
   `rig.mount.push_to(alt - 5.0, az)`, with `alt` and `az` from
   `simulator::sky::horizontal`), then the sync, then tick (at most 10 ticks) until the
   session's pointing is within 6° of M42, and assert it, with the message "no frame
   solved after the push", before the watchdog. Without that wait the only sync before
   the later GoTo is the rig's first frame, exposed before the push, and the GoTo plans
   from the start direction: its first slew lands 33.5° off and needs two
   corrections. Before the GoTo (`crates/finder/tests/motion/stop/watchdog.rs:192`)
   assert that the telescope's true direction is within 6° of M42. The final assertions
   (`crates/finder/tests/motion/stop/watchdog.rs:205` onward) are unchanged; a GoTo of a
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
   - `fn mount_config(alt: f64, az: f64) -> MountConfig`, private: the `MountConfig`
     literal of `crates/finder/tests/support/goto.rs:112` (base tilt 1° toward 130°,
     zero points `[az, alt]`, backlash 0.3°) moves here, and `with_session` and
     `far_sync` both call it. `std::path::Path` joins the file's `use std::{...}` group.
   - `pub async fn far_sync(path: &Path, start: f64, alt: f64, az: f64)`: writes a state
     file holding both syncs of `second()`, computed, not typed. A second `SimMount`,
     `SimMount::with_config(VirtualClock::new(start), mount_config(alt, az))`, is driven
     in paused time with `Mount::goto` directly, by positive moves, first to axis counts
     (4500, 4500), the counts of `crates/finder/tests/support/goto.rs:246`, then to
     (580000, 4500), those of `crates/finder/tests/support/goto.rs:250`, so its gears sit
     on the flank both syncs use; after each move it waits for both axes to stop and
     asserts the counts. At each, its `true_direction()` gives the sync's `az_deg`, and
     `finder::ephemeris::apparent_from_true` (`crates/finder/src/ephemeris.rs:129`) of its
     altitude gives `alt_deg` (0.82′ of refraction at the far sync), because solved
     frames record apparent altitude. The syncs are `finder::model::Sync` literals: the
     near one first, with `time` = `start - 120.0`, then the far one, with `time` =
     `start`; each with `axis` its counts, `counts_per_revolution` from `mount.axes()`
     (1620000 on both axes), `roll_deg` null, `rms_arcsec` 1 and `epoch` 0. The near sync
     is 120 s older so the live sync at the same counts does not thin it
     (`crates/finder/src/model.rs:512` to `crates/finder/src/model.rs:517`). One
     `finder::model::Epoch` holds the rig's zero points on the positive flank, in TPoint
     signs: `ia` = backlash / 2 − the azimuth zero point (−119.85), `ie` = the altitude
     zero point − backlash / 2 (49.85). The file holds top-level `site` (40, −75) and
     `min_alt_deg` 15, and a `model` with the two syncs, that epoch and its own
     `rms_arcsec` 1 (the model has no serde default for it); no `base_tilt`, `fixed`,
     `expires_at` or `backlash`.
   - `pub async fn second_from_file(&self)`: makes the first 1° move of `second()`
     (`crates/finder/tests/support/goto.rs:246`), 15 ticks, and asserts what `second()`
     asserts (`crates/finder/tests/support/goto.rs:255` and the line after it), plus
     that the azimuth axis reads 4500 counts, that the model holds three syncs, the two
     stored and the live one at (4500, 4500) (`assert_eq!`, not "at least two", which
     passes with the near sync missing), and that the fitted base tilt is within 1′ of
     the simulator's 1° toward 130° (`(deg − 1) × 60` and `(toward − 130) × 60` each
     under 1 in magnitude). Why the tilt:
     a wrong fixture (true instead of apparent altitude, the negative flank, azimuth or
     altitude 0.3° off, or an untilted mount) passes the five GoTos silently, because
     they correct themselves, with the fitted tilt off by up to 663′; round 2 saw the
     tilt check fail each.
   Loading a state file does not re-fit anything (`crates/finder/src/session.rs:1511`);
   the base tilt is fitted by the product when the live sync of the 1° move joins the
   stored syncs in the same epoch, the far one 129° away (`crates/finder/src/model.rs:519`).
   Why both syncs are stored: GoTo starts its cable-wind count from the model's first
   sync (`crates/finder/src/goto.rs:748`; the wind monitor's is
   `crates/finder/src/goto.rs:564`). With the far sync alone it is `syncs[0]`, and the
   M81 → M1 GoTo then turns 118° the short way to 1,736,902 counts, 386° from power-on,
   past the planner's ±360° (`crates/finder/src/goto.rs:479`), and nothing notices. With
   the near sync first, M1 unwinds to 118,535 counts, as the old setup's model makes it.
3. `crates/finder/tests/motion/goto/sim.rs`:
   `five_catalog_targets_converge_after_two_syncs_and_report_progress`
   (`crates/finder/tests/motion/goto/sim.rs:9`) writes the file with
   `api::far_sync(&path, START, 50.0, 120.0)` into a scratch folder, builds the rig with
   `Rig::with_session(START, 50.0, 120.0, true, Session::with_state_file(path).unwrap())`
   and calls `second_from_file()` instead of `second()`
   (`crates/finder/tests/motion/goto/sim.rs:11`). The target list
   (`crates/finder/tests/motion/goto/sim.rs:12`) and every assertion after it are
   unchanged. The folder is built as the file's neighbours build theirs today, and the
   test removes it at its end; US9 moves it to the shared helper later. The imports gain
   `finder::session::Session` and `std::time::Duration`.
4. Same file: `continuously_observed_cable_wind_survives_real_simulator_power_cycle`
   (`crates/finder/tests/motion/goto/sim.rs:255`): before
   `rig.move_to([350 * 4500, 0])` (`crates/finder/tests/motion/goto/sim.rs:258`) read the
   frame id of the session's pointing and call
   `rig.controls.set_frame_interval(Duration::from_secs(2))`; after it read the frame id
   again, call `rig.controls.set_frame_interval(Duration::from_millis(500))`, and assert
   the advance is at least 20 and at most 60. The frame id is read by a helper in the
   file, `fn pointing_frame(rig: &Rig) -> u64`, from
   `rig.state.session().snapshot(rig.now()).pointing`'s `frame_id`. The power cycle
   (`crates/finder/tests/motion/goto/sim.rs:263`) and the rest are unchanged; the cycle
   needs a sync within 1.5 s afterwards, which is why 500 ms comes back first. At 2 s the
   350° move is sampled about every 7°, inside the half-turn the wind monitor needs
   between observations (`crates/finder/src/goto.rs:61`).

### US4: the whole night at 2 frames a second

1. `crates/simulator/src/controls.rs`: after `start_frames`, next to `stop_frames`
   (`crates/simulator/src/controls.rs:105`):

   ```rust
   pub fn set_frame_interval(&self, interval: Duration) {
       self.controls.set_frame_interval(interval);
   }
   ```

   with `time::Duration` added to the file's existing `use std::{...}` group. The
   simulator's controls are the node's (`crates/simulator/src/node.rs:45`);
   `NodeControls::set_frame_interval` (`crates/scope-node/src/service.rs:72`) refuses
   zero; the capture loop reads the interval before every frame
   (`crates/scope-node/src/service.rs:242`).
2. `crates/finder/tests/motion/night.rs`: after
   `let sim = state.simulator().unwrap().clone();` (`crates/finder/tests/motion/night.rs:136`),
   `sim.set_frame_interval(Duration::from_millis(500));` (the file already imports
   `Duration`). After the first-sync block's last line
   (`crates/finder/tests/motion/night.rs:164`, `assert!(first["model"]["base_tilt"].is_null());`;
   the block opens at `crates/finder/tests/motion/night.rs:153`), read the session 30
   times, one tick apart, collect the distinct `(pointing.frame_id, pointing.time)`
   pairs, skip the first, and assert at least four remain and that for each consecutive
   pair the time step divided by the frame-id step is 0.5 s within 0.05 s. At the 200 ms
   default it is 0.2 s. The test's only other change is the call above.

### US5: Track's restart in paused time

`crates/finder/tests/cli/track.rs`, `tracking_that_moved_the_mount_is_off_and_quiet_after_restart`
(`crates/finder/tests/cli/track.rs:164`), becomes `#[tokio::test(start_paused = true)]`,
as `crates/finder/tests/cli/backlash.rs:119` is, and:

1. In process, on the test's folder:
   `goto::Rig::with_session(goto::START, 50.0, 120.0, true, Session::with_state_file(folder/state).unwrap())`
   (US3's constructor), the eyepiece pointed at M42, `rig.sync()`, the wait of the
   centred-start recipe ("Shared helpers"), reading the pointing through `rig.view()` as
   US2's `centred()` does, then `POST /api/track` `{"on":true}` with no Origin header
   (allowed for writes), asserting the reply reads `tracking` with no `goto`. The test
   makes that request itself, a JSON `Request::post("/api/track")` with `host` and
   `content-type` headers sent through `rig.app` with `tower::ServiceExt::oneshot`,
   because `Rig::call` sends an empty body; `axum`'s `Body`, `Request` and `to_bytes`,
   `finder::model::separation`, `finder::session::Session` and `tower::ServiceExt` join
   the file's imports.
   Then it ticks until the mount's true direction changes, at most 100 ticks, and
   asserts that it changed: from a centred start the gears sit mid-backlash (0.3°), so
   Track's rates take some seconds to take up the play before the tube turns, and a
   fixed 12 ticks is not enough. Then an explicit `drop(rig)` at the end of the
   in-process block.
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
(`crates/finder/src/session.rs:1450`) has no such field and refuses unknown ones. The
other tests of the file are unchanged.

### US6: the binary only for `main.rs`

1. **New** `crates/finder/tests/console/session/durable.rs`, declared by `mod durable;` in
   `crates/finder/tests/console/session/mod.rs`, with one test,
   `durable_fields_survive_reopening_the_state_file_in_owner_only_mode`, on
   `#[tokio::test]` with a fixed clock:
   - a folder holding a state file that it sets to mode 0644, and a stale
     `session.json.tmp` beside it;
   - for exposure 200 ms and then 2000 ms: `Session::with_state_file`
     (`crates/finder/src/session.rs:1278`), one `Session::edit`
     (`crates/finder/src/session.rs:1251`) setting site, minimum altitude, target,
     imaging camera, node address and token, and camera settings (exposure, gain 4,
     sigma 5), and `Session::set_aim` (`crates/finder/src/session.rs:1234`) to (700, 420);
     drop it; reopen with `Session::with_state_file`; assert every value equal, the
     view's `token_set` true and the token absent from the view's JSON; the token itself
     is compared in the stored file (`stored["node"]["token"] == "test-token"`), since
     the view holds only `token_set`; the whole reopened view equals the view the write
     returned, and the mode is 0600 again after each reopen;
   - after the first write: the file's mode is 0600 (`crates/finder/src/session.rs:1536`,
     `options.mode(0o600);`) and the stale `.tmp` is gone
     (`crates/finder/src/session.rs:1526`, `match fs::remove_file(&temporary)`).

   Its scratch folder is `env!("CARGO_TARGET_TMPDIR")` joined with
   `session-durable-<pid>`, held by a local `Folder` guard whose `Drop` removes it; US9
   replaces that guard with `crate::support::Scratch`. The 0644 file is made by
   `Session::with_state_file` on the empty folder (a fresh default session), dropped, and
   then set to mode 0644 before the stale `.tmp` is written beside it.
2. **Delete** `crates/finder/tests/cli/session_node.rs` and `mod session_node;`
   (`crates/finder/tests/cli/main.rs:12`). Its stderr check
   (`crates/finder/tests/cli/session_node.rs:80`) moves to item 3; its restart, mode and
   exposure checks are item 1's.
3. `crates/finder/tests/cli/aim.rs`: its server pipes the child's stderr
   (`crates/finder/tests/cli/aim.rs:108`) and its reader thread
   (`crates/finder/tests/cli/aim.rs:113`) only looks for the address. It keeps every line
   in a shared `logs` string, as `crates/finder/tests/cli/session_node.rs:16` does, and
   when the server is dropped asserts `!logs.contains("test-token")`. Its test
   (`crates/finder/tests/cli/aim.rs:165`) is otherwise unchanged and stays the restart
   check on the real binary.
4. `crates/finder/tests/cli/backlash.rs`: its `Server::request(edit: Option<Value>)` is
   replaced by `request(method, path, body)`, as `crates/finder/tests/cli/track.rs:93`
   has, and its two session reads become `request("GET", "/api/session", Value::Null)`;
   `serde_json::json` joins the import. The restart loop
   (`crates/finder/tests/cli/backlash.rs:140`) becomes a `restart` closure that starts
   the server, makes the loop's read and assertions, and returns the server:
   `drop(restart());` then `let server = restart();`, so the second server stays up
   after its read of the measured backlash. The test then posts `{"what":"fixed"}` to
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
   500 ms. Its other six callers are unchanged.
2. `crates/finder/tests/console/aim.rs`:
   `aim_writes_enforce_origin_json_and_strict_shape_and_preview_retention_is_eight`
   (`crates/finder/tests/console/aim.rs:63`) calls `state_with_interval(.., 3 s)`,
   written `std::time::Duration::from_secs(3)` in full. The
   node sends a preview when 5 s have passed since the last
   (`crates/scope-node/src/preview.rs:12`), so at 3 s previews fall on every other frame.
   After its loop's closing brace (`crates/finder/tests/console/aim.rs:115`; the loop
   opens at `crates/finder/tests/console/aim.rs:108`), before
   `assert!(live.preview(id).is_none())`, assert
   `previews.windows(2).all(|w| w[1] - w[0] == 2)`. Every retention assertion after it is
   unchanged. The wait for each frame allows 10 s of virtual time
   (`crates/finder/tests/support/live.rs:121`), more than 3 s.
3. `crates/finder/tests/console/live/loop_.rs`:
   `rich_frames_failing_for_thirty_seconds_report_the_whole_unsolved_interval`
   (`crates/finder/tests/console/live/loop_.rs:240`) calls `state_with_interval(.., 5 s)`,
   counts the frames its wait loop (`crates/finder/tests/console/live/loop_.rs:248`)
   receives after the first, and asserts at most 7 (round 2's probe received 6). The unsolved interval is measured from the first unsolved rich
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
2. The inline rig solvers: `Stars` in `crates/finder/tests/support/goto.rs:42` (its line
   as US3 leaves the file; on `main` `pub struct Stars;` is one line up), in
   `crates/finder/tests/support/mcp.rs:113`, in `crates/finder/tests/support/stop.rs:65`,
   in `crates/finder/tests/motion/night.rs:21` and in
   `crates/finder/tests/motion/mount_identity.rs:58` (024 US3's file, whose call is at
   `crates/finder/tests/motion/mount_identity.rs:62`) each call
   `solve::solve_centroids(.., &options)` with the 5000 ms default
   (`crates/finder/src/solve.rs:69`), which the solver checks against the real clock
   (`crates/plate-solver/src/status.rs:125`). Each passes
   `&SolveOptions { timeout_ms: u64::MAX, ..options }` instead. Nothing adds the timeout
   to an `Instant`: the deadline check compares the elapsed time with it
   (`crates/plate-solver/src/status.rs:125`), so `u64::MAX` cannot overflow.
   Out of scope, and named so: the other 5 s solver deadlines tests still reach, which
   are the production live solver's through `AppState::with_live`
   (`crates/finder/src/api.rs:247`), uploaded-frame solves (whose `timeout_ms` the upload
   route parses, `crates/finder/src/api.rs:1244`), and direct calls of
   `solve::solve_frame_with_frame` (`crates/finder/src/solve.rs:382`). 025's new test
   modules hold no inline rig solver: they solve through `AppState::with_live`, and one
   test of `crates/finder/tests/console/recording.rs` builds `Live::start` with the
   production `DbSolver` (`crates/finder/tests/console/recording.rs:411`), so all of
   them are on the first of those paths. Round 3's solve-stall mutation, a deadline that
   starts 5 s early, failed, on round 3's tree before 025:

   | State | finder's tests (351) | `mcp` and `motion` (164) |
   |---|---|---|
   | Before US8 | 122 | 93 |
   | Round 2's US8, without `mount_identity.rs` | 45 | 17 |
   | US8 with `mount_identity.rs` | 38 | 10 |

   Each of the 38 left is on one of the out-of-scope paths above. `mount_identity.rs`'s
   own nine tests fail 7 before the change and none after.
3. `crates/finder/tests/cli/simulate.rs`: `let first = ...` is read at
   `crates/finder/tests/cli/simulate.rs:130`, before a wait for the first live pointing
   and two edits; the 5 s window starts at `crates/finder/tests/cli/simulate.rs:142`.
   Read the session time again immediately before `let at = Instant::now();` and compare
   against that reading at `crates/finder/tests/cli/simulate.rs:155`. `first` stays for
   the epoch check that uses it. Out of scope, and named so: the 15 s wait for the first
   live pointing in the file's `Server::pointing`
   (`crates/finder/tests/cli/simulate.rs:97`), another wall-clock bound, which this
   story leaves alone.
4. `crates/scope-node/tests/cli.rs`: `run` (`crates/scope-node/tests/cli.rs:7`) calls
   `.output()` (`crates/scope-node/tests/cli.rs:21`), which waits forever. It spawns the
   child with `.stdin(Stdio::null())`, as `.output()` did, and piped stdout and stderr,
   polls `try_wait` every 50 ms for up to 10 s, and on timeout kills and reaps it,
   removes the config file, and panics naming the child's command-line arguments
   (`command.get_args()`), not the config text, which holds the token. `Stdio`,
   `Duration` and `Instant` join the file's imports. The refusal test
   (`crates/scope-node/tests/cli.rs:62`) is otherwise unchanged and still starts the node
   without `--check-config`, so it still checks the serving path's refusal.
5. `crates/scope-node/src/rpicam/metadata.rs`: the three tests at
   `crates/scope-node/src/rpicam/metadata.rs:151`, `crates/scope-node/src/rpicam/metadata.rs:166`
   and `crates/scope-node/src/rpicam/metadata.rs:192` become
   `#[tokio::test(start_paused = true)]`. They read in-memory duplex streams only, so a
   paused clock changes nothing but makes their 100 ms timeouts independent of load;
   `crates/scope-node/src/synthetic.rs:101` is a precedent. Two of them wrap a read in a
   100 ms timeout; `malformed_or_missing_metadata_is_not_a_frame` never had one. Only the
   file's `#[cfg(test)]` module changes.

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
   | `crates/finder/tests/console/session/durable.rs` (US6's new file) | `session-durable` | by its own `Folder` guard |
   | `crates/finder/tests/cli/recording.rs:37` (025 US1's) | `recording-binary` | on drop, by its `Console` guard |
   | `crates/finder/tests/console/recording.rs:36` (025 US1's) | `recording-<name>` | on drop |
   | `crates/finder/tests/console/recording_raw.rs:28` (025 US2's) | `raw-<name>` | on drop |
   | `crates/finder/tests/motion/recording_raw.rs:24` (025 US2's) | `raw-moving` | on drop |

   The keyhole and tracking-night rigs build their folders in `Rig::new`
   (`crates/finder/tests/motion/tracking/keyhole.rs:149`,
   `crates/finder/tests/motion/tracking/night.rs:139`); each rig gains a field
   `_folder: crate::support::Scratch`, so the folder lives as long as the rig, and
   `keyhole.rs` loses its early `remove_dir_all`
   (`crates/finder/tests/motion/tracking/keyhole.rs:270`). `writes.rs` builds its folder
   as `repo.join("target")`, where `repo` is `CARGO_MANIFEST_DIR` joined with `../..`
   (`crates/finder/tests/console/session/writes.rs:182`); `repo` stays for the database
   path, the `catch_unwind` stays because it reaps the child, and only the
   `remove_dir_all` (`crates/finder/tests/console/session/writes.rs:248`) goes. In
   `durable.rs`, `Scratch::new("session-durable")` replaces its `Folder` type, its first
   `remove_dir_all` and its `create_dir_all`. In
   `crates/finder/tests/console/targets.rs` the `remove_dir` after the child
   (`crates/finder/tests/console/targets.rs:124`) also proved the child left the folder
   empty; it becomes `assert_eq!(std::fs::read_dir(&empty.0).unwrap().count(), 0)`.
   `crates/finder/tests/console/session/edits.rs` already counts the entries before it
   removes the folder, and keeps that count.

   025's four modules each name a path under `../../target` that does not exist yet and
   let the recorder or the binary create it. In `crates/finder/tests/console/recording_raw.rs`
   and `crates/finder/tests/motion/recording_raw.rs`, and in every test of
   `crates/finder/tests/console/recording.rs` but one, the `Scratch` is the recordings
   folder itself: `Folder::new(..)` becomes `crate::support::Scratch::new` of the prefix
   above, the `root.0` reads are unchanged, and the file's `Folder` type goes with its
   stale-folder assertion and its `Drop`; `console/recording_raw.rs`'s
   `fn folder(root: &Folder, ..)` (`crates/finder/tests/console/recording_raw.rs:156`)
   takes `&crate::support::Scratch`. The folder now exists before the first start, which
   the recorder's `create_dir_all` (`crates/finder/src/recording.rs:231`) accepts. The
   exception is `invalid_requests_and_missing_recording_sources_leave_no_recording`
   (`crates/finder/tests/console/recording.rs:429`), which asserts that refused requests
   leave no recordings folder (`crates/finder/tests/console/recording.rs:463` and
   `crates/finder/tests/console/recording.rs:481`) and then writes a regular file in its
   place (`crates/finder/tests/console/recording.rs:482`). Its root is
   `scratch.0.join("root")`, after `let scratch = crate::support::Scratch::new("recording-refusals");`,
   a path that does not exist yet; the test's `root.0` reads become `root`, and those
   assertions stay. `crates/finder/tests/cli/recording.rs` gives the binary's `--state`
   the same kind of path, `scratch.0.join("state")` of a
   `crate::support::Scratch::new("recording-binary")` declared before its `Console`
   guard, so the folder is removed after the guard has killed and reaped the child.
   `assert!(!root.exists())` (`crates/finder/tests/cli/recording.rs:41`) stays, so the
   binary still creates its state folder, and only the guard's `remove_dir_all`
   (`crates/finder/tests/cli/recording.rs:20`) goes. Its `current_dir`,
   `CARGO_MANIFEST_DIR` joined with `../..` (`crates/finder/tests/cli/recording.rs:43`),
   is the repository root, not a scratch folder, and stays. The three in-process modules
   alias `crate::support::live` as `support` (`crates/finder/tests/console/recording.rs:3`),
   so the helper is written `crate::support::Scratch` in full. Built against 9d8cc4e by
   the round-4 probe, these edits make a diff of 12,349 B, and US9's whole diff is
   53,272 B.

   Imports a removed type or call leaves unused are removed.
3. The scope-node files have no shared helper and get none: in
   `crates/scope-node/tests/cli.rs:9`, `crates/scope-node/tests/mount_config.rs:9`,
   `crates/scope-node/tests/rpicam.rs:123` and `crates/scope-node/tests/mount_link.rs:159`
   the path is built under `env!("CARGO_TARGET_TMPDIR")` instead of `CARGO_MANIFEST_DIR`
   and `../../target`, and held by a small guard in the file whose `Drop` removes it.
   `cli.rs` and `mount_config.rs` scratch a file, so their guards call `remove_file`;
   `rpicam.rs` and `mount_link.rs` scratch a folder. `rpicam.rs`'s guard is
   `#[cfg(unix)]`, as its one user is. `cli.rs`'s timeout branch (US8) loses its own
   `remove_file`, because the guard runs while the panic unwinds. `mount_link.rs` is
   024's: `binary_keeps_running_with_a_missing_mount_device` builds
   `../../target/mount-link-process-<pid>` and removes it only near its end
   (`crates/scope-node/tests/mount_link.rs:189`), so a panic before that line leaves it.
   It gets a file-local `struct Scratch(PathBuf)` whose `Drop` calls `remove_dir_all`,
   holding `CARGO_TARGET_TMPDIR` joined with `mount-link-process-<pid>`, and the
   `remove_dir_all` at `crates/scope-node/tests/mount_link.rs:189` goes. Round 3 saw a
   full run leave nothing behind, and neither this guard nor `durable.rs`'s `Scratch`
   leaves anything when its test is made to panic.

The two unit tests in `crates/finder/src` that make folders are left alone: cargo sets
`CARGO_TARGET_TMPDIR` only for integration tests and benches.

### US10: duplicates

Every edit below was read at f0ea2d5, and 025 changed none of these files, so each holds
at 9d8cc4e; none touches an oracle file.

| Finding | File | Edit | The stricter twin |
|---|---|---|---|
| core-D3 | `crates/grpc-service/tests/service_acceptance.rs:240` | delete `pixel_center_convention_top_left` | `extract_centroids_reports_wire_x_as_column` (`crates/grpc-service/tests/service_acceptance.rs:404`): asymmetric pixel, axis order, one star |
| core-D4 | `crates/grpc-service/tests/service_acceptance.rs:187` and `crates/grpc-service/tests/service_acceptance.rs:325` | delete `inline_image_bytes_used_when_shmem_absent` and `coordinate_swap_inbound_and_outbound` | for the first, `extract_centroids_reports_wire_x_as_column` (`crates/grpc-service/tests/service_acceptance.rs:404`), which also sends inline image bytes with no shared memory and checks the centroid it gets back; only the 8×8 image size is lost. For the second, `all_four_rpcs_available` (`crates/grpc-service/tests/service_acceptance.rs:82`) makes the same calls, checking only `is_ok`. Neither deleted test could fail |
| core-D5 | `crates/plate-solver/tests/solve_acceptance.rs:287` | delete the solve half of `cancellation_honored`; rename the rest `solve_context_reports_a_set_cancel_flag` and say in its comment that `SolveStatus::Cancelled` has no test and no public trigger, and that `timeout_bounds_the_search` covers the zero-timeout solve | `timeout_bounds_the_search` (`crates/plate-solver/tests/solve_acceptance.rs:266`) |
| core-D6 | `crates/plate-solver/tests/solve_acceptance.rs:88` | delete `explicit_detection_parameters_honored`; rename `brightest_first_requirement` (`crates/plate-solver/tests/solve_acceptance.rs:43`) `exhausted_search_of_an_empty_pattern_catalog_is_no_match`, assertions unchanged. `detection_parameters_do_not_perturb_solve_math` (`crates/plate-solver/tests/solve_acceptance.rs:159`) stays: its image solve must find at least four of five single-pixel peaks on an all-zero 100×100 image (`NO_MATCH`, not `TOO_FEW`), and no test outside the oracle detects on a zero background | E2's image parity, and `TOO_FEW` on a blank image over the wire |
| core-D7 | `crates/database-generation/src/patterns.rs:177` | delete `breadth_first_yields_brightest_first`; move `matches_cedar_order_and_visits_each_combination_once` into `tests` and delete `bfc_order_tests` | the seven-element order test pins the five-element sequence |
| node-D1 | `crates/simulator/tests/mount.rs:84` | delete `backlash_changes_truth_by_point_three_at_the_same_axis_reading` | `defaults_keep_zero_rate_error_and_point_three_backlash_on_both_axes` (`crates/simulator/tests/rate_and_backlash.rs:71`): 1″ against 0.01°, and exact positions |
| node-D2 | `crates/scope-node/tests/skywatcher.rs:222` | reduce `both_stops_precede_emergency_fallback` to `stop_all_stops_both_axes`: default emulator, both axes running, `stop_all`, assert K1 and K2 sent and both axes at rest | `a_silent_first_axis_confirms_with_emergency_stop_after_the_second_axis` in `crates/scope-node/tests/skywatcher_stop.rs`; the reduced test stays the only caller of `SkyWatcher::stop_all` |
| node-D3 | `crates/scope-node/src/mount/mod.rs:99` | delete the `tests` module and its `FakeMount` | the port contract on real adapters, `crates/simulator/tests/mount.rs` and `crates/scope-node/tests/skywatcher.rs` |
| api-D9 | `crates/finder/src/session.rs:1599` and `crates/finder/src/session.rs:1609` | delete the two inline tests | `fresh_session_and_fixed_clock_are_exact_on_repeated_reads` (`crates/finder/tests/console/session/edits.rs:54`) |
| simA-D5 | `crates/finder/tests/motion/model/sim.rs:200` | move the tilt assertions (`crates/finder/tests/motion/model/sim.rs:207` to the line before the test's end) into `durable_night_survives_local_midnight_then_expires_at_solar_noon` in place of its `base_tilt.is_some()` (`crates/finder/tests/motion/model/sim.rs:228`), then delete `second_sync_fits_the_simulator_base_without_opening_an_epoch` | the durable night runs the same setup and second sync |
| simA-D4 | `crates/finder/tests/motion/goto/routes.rs:49` to `crates/finder/tests/motion/goto/routes.rs:101` | delete `approach_to_fifteen_point_two_degrees_is_refused_without_motion`, `sun_five_degrees_up_is_refused_without_motion`, `m8_below_horizon_is_refused_without_motion` and `absent_mount_and_missing_sync_each_name_the_reason`, and the `ephemeris` and `sky` imports only they used (`crates/finder/tests/motion/goto/routes.rs:6`); in `no_mount_omits_refusal_and_second_sync_omits_it_while_running` assert the no-mount refusal's error names `no mount` (`crates/finder/tests/motion/goto/refusals.rs:119`, which checks only 409 today) | each has a stricter copy in `crates/finder/tests/motion/goto/refusals.rs` that also checks `goto_refusal`; `foreign_goto_is_forbidden_without_motion` and `simulate_startup_connects_the_default_mount_and_serves_motion_and_stop` stay |
| simA-D9 | `crates/finder/tests/motion/model/routes.rs:107` | the foreign-origin loop keeps `"fixed"` only, unrolled to one request | `foreign_model_writes_leave_a_working_mount_and_populated_model_unchanged`, in the same file, covers the foreign `night` reset; the guard runs before the body is read |
| mcp-D3 | `crates/finder/tests/mcp/motion.rs:226` | `stopped()` takes `watch: bool`, with a doc comment saying what each call checks: always StopAll within 0.1 s, the halt reason, motion cleared and at rest within 20 ticks (2 s); only with `watch`, today's 5 s at rest, 5 s unchanged and only-StopAll checks. The `"second client"` case (`crates/finder/tests/mcp/motion.rs:272`) passes `true`, the other eight `false` | the spec asks for the 5 s watch in the second-client case only (016 US4-S6) |

Why node-D1 deletes only: the skeptic's tidy-up, moving two default-value assertions to
`default_base_tilt_and_zero_points_are_present`, would leave the survivor's name claiming
"zero rate error" it no longer checks; the assertions already sit in the survivor and
`crates/simulator/tests/mount.rs:196` already checks the 0.3° default.

The story removes 14 test functions, renames three and moves one, as round 2's probe
counted them and round 3 confirmed with `detection_parameters_do_not_perturb_solve_math`
kept byte-identical: `brightest_first_requirement` becomes
`exhausted_search_of_an_empty_pattern_catalog_is_no_match`, `cancellation_honored`
becomes `solve_context_reports_a_set_cancel_flag`, and
`both_stops_precede_emergency_fallback` becomes `stop_all_stops_both_axes`;
`patterns::bfc_order_tests::matches_cedar_order_and_visits_each_combination_once` moves
to `patterns::tests::matches_cedar_order_and_visits_each_combination_once`. With every
story kept, nextest runs 645 tests: the 660 of `main` at f0ea2d5, less 16, plus one
(SC-003). 025's US1 and US2 add 17 test functions, none of which a story here removes,
so on 9d8cc4e the same sum is 677 less 16 plus one, 662; that count is taken from the
source, not from a run. `stop_all_stops_both_axes`, like the test it reduces, has no
positive control that the axes were moving before the stop. In `simA-D4`, the deleted
no-mount ticks could not fail: a rig built with `connected` false has no mount. The
probe found each new or moved assertion failing under a mutation of what it checks;
undoing a saving fails nothing, so the savings are read from the diff.

### US11 (optional): the whole night shortened

`crates/finder/tests/motion/night.rs`: the five GoTos (`crates/finder/tests/motion/night.rs:174`,
`["M42", "M31", "M45", "M81", "M1"]`; the loop is at line 198 once US4 and US8 have landed) become
one, `night.goto("M81")`, with the same assertions; the first sync, the one-tap second
sync, the hand push and its GoTo to M42, the Stop mid-slew to M31 and the dropped link
mid-slew to M81 are unchanged. The test is renamed
`a_whole_night_syncs_gotos_hand_push_stop_and_dropped_link`. No assertion guards the
saving: the diff is the evidence (US11-S2). Spec 014 US9's "five GoTos"
(`specs/014-goto/spec.md:677`) keeps its text; approval 15 records the amendment.

### US12 (optional): the MCP hold shortened

`crates/finder/tests/mcp/tracking.rs`: the hold loop spans 75 s, through its two literals:
`for second in 0..=600` (`crates/finder/tests/mcp/tracking.rs:181`) and
`if second < 600` (`crates/finder/tests/mcp/tracking.rs:188`) both become 75. The test
(`crates/finder/tests/mcp/tracking.rs:163`) is renamed
`track_first_goto_keeps_mcp_source_and_holds_m42_for_seventy_five_seconds`; the comment
above the loop says "through t=75". The closing `eprintln!`
(`crates/finder/tests/mcp/tracking.rs:206`) may say the new span in place of
`ten-minute` (`crates/finder/tests/mcp/tracking.rs:207`). Every assertion in and after the
loop is unchanged. No assertion guards the saving, and none could: the tracker holds 120
samples at either span (its window is 60 s), so "more than 30 samples" cannot catch a
revert; the diff is the evidence (US12-S2). Spec 016 US6-S1's "10 min"
(`specs/016-mcp/spec.md:695`) keeps its text; approval 16 records the amendment.

### US13 (optional): the window below the route

1. `crates/finder/tests/motion/tracking/tracker.rs`:
   `pointing_samples_age_on_console_time_and_are_ignored_after_ending`
   (`crates/finder/tests/motion/tracking/tracker.rs:480`) feeds one sample and then runs
   1202 ticks of 50 ms (`crates/finder/tests/motion/tracking/tracker.rs:500`). Split that
   run at 1198 ticks (59.9 s): assert the sample is still held, then run the last 4 ticks
   and keep today's check that it is gone.
2. `crates/finder/tests/motion/tracking/routes.rs`:
   `readout_uses_live_errors_last_sixty_seconds_and_fr001_fr002`
   (`crates/finder/tests/motion/tracking/routes.rs:244`) ends its 750 ticks
   (`crates/finder/tests/motion/tracking/routes.rs:259`) as soon as the tracker holds
   more than 30 samples, and drops the two window assertions
   (`crates/finder/tests/motion/tracking/routes.rs:265` and the line after). The loop
   keeps 750 ticks as its upper bound, so a regression fails at the kept "more than 30"
   assertion instead of hanging; `let tracker` moves above the loop. Its RMS,
   field-rotation and longest-exposure assertions stay. Then it stops the camera
   (`rig.controls.stop_frames()`), ticks 610 times (61 s) so the live samples age out,
   injects one sample, 12″ east and −9″ north (an error of 15″), with
   `rig.state.session().tracker().unwrap().pointing(east, north)`
   (`crates/finder/src/tracking.rs:250`), and asserts that the readout's `rms_arcsec`
   equals that sample's error within 1e-9″ after 599 ticks (59.9 s) and is 0 two ticks
   later (60.1 s). Missing solves do not end Track, so the readout stays present. The
   readout's own filter is `crates/finder/src/session.rs:153`.

Spec 015 US7-S7 (`specs/015-tracking/spec.md:416`) keeps its text; approval 17 records
the amendment.

## Measurements

"Before" is each test's duration in the GitHub `test` job of ergane gates run 43683194483
on b8dceaa (nextest, optimized test build). "Work" is what round 2's probe counted on its
build of each story, and round 3 on the stories it rebuilt, in units that do not depend
on the machine: frames rendered, detected and solved, ticks, GoTos, requests and
processes started, and simulated seconds. None of it is part of any scenario. Wall
seconds are not recorded here: the owner reads the first GitHub `test` job after the
last story lands (SC-004).

| Story | Tests changed | Before (s) | Work, before → after (round 2 and 3 probes) |
|---|---|---|---|
| US1 | `axis_moves::braking_distance_…` 32.515, `cable_wind_refuses_…` 27.516, `track_through_the_page_…` 8.384, `disconnected_client_…` 7.405, `rate_duration_…` 5.202, `sunrise_halts_…` 4.076, `a_zero_rate_…` 2.039; simulator `silent_link_…` 2.584, `stop_cancels_…` 2.457, `watchdog_count_…` 2.086, `a_full_adapter_queue_…` 0.396 | 94.660 | frames after the first sync in the six direct-move tests 485 → 0 (braking 212, cable wind 174, disconnected 42, rate duration 24, sunrise 23, zero rate 10); Track test frames 52 → 18 and GoTo handoffs 1 → 0; simulator Stop and watchdog catalog renders 52 → 0 and pattern-database loads 4 → 0 |
| US2 | `tracking::routes::track_names_…` 13.720, `foreign_bodiless_…` 8.706, `below_horizon_…` 6.740, `dropping_console_…` 5.518; `min_alt::…` 6.139; `motion_ends::track_stop_shares_…` 6.521, `track_stop_starts_…` 5.530; `retry::retained_tracker_…` 6.032; `watchdog::past_watchdog_…` 5.651 | 64.557 | frames in the nine tests 425 → 99 (track names 96 → 28, foreign bodiless 56 → 22, below horizon 44 → 10, dropping console 36 → 2, min alt 40 → 6, track stop shares 40 → 6, track stop starts 36 → 2, retry 40 → 6, watchdog 37 → 17); the three Stop tests send no GoTo command |
| US3 | `goto::sim::five_catalog_…` 80.944, `continuously_observed_cable_wind_…` 31.589 | 112.533 | five-target frames 552 → 446 with both syncs stored, as round 3 counted (374 with the far sync alone, which loses today's cable-wind state); cable-wind frames 218 → 62 (the 350° move 208 → 52) |
| US4 | `night::a_whole_night_…` | 181.548 | frames rendered and solved 1511 → 617; `/api/goto` requests 8 → 8; simulated time 306.8 s → 312.8 s |
| US5 | `track::tracking_that_moved_…` | 27.032 | built-binary starts 2 → 1; `POST /api/track` to the binary 1 → 0; real-time waits before the restart 3 → 0; 14 frames in process, in paused time |
| US6 | removed: `session_node::…` 1.554, `tracking::backlash::first_transit_…` 3.438; changed: `cli backlash::…` 4.098, `cli aim::…` 1.373 | 10.463 | built `finder` processes 9 → 5; paused-time M42 transit measurements 3 → 2; test functions −2 +1 |
| US7 | `aim::aim_writes_…` 9.591, `live::loop_::rich_frames_…` 8.336 | 17.927 | aim test frames 81 → 17 (previews 1, 11, …, 81 → 1, 3, …, 17); unsolved-interval frames 61 → 7, its unsolved interval 30.0 s both |
| US8 | `tracking::night::an_hour_…` 2.915, `cli simulate::simulated_epoch_…` 5.775, scope-node `start_refuses_…` 0.010 | no saving intended | frames and solves unchanged; scope-node still starts 3 children; `cli/simulate.rs` makes one more `GET /api/session` |
| US9 | none intended | n/a | scratch entries left under `target/` after a full run 2 → 0, and `CARGO_TARGET_TMPDIR` empty after each run |
| US10 | removed: `model::sim::second_sync_fits_…` 11.974, `goto::routes::absent_mount_…` 2.590, `approach_to_…` 2.130, `m8_below_…` 1.926, `sun_five_…` 1.487, and nine tests of 0.010 s or less each; changed: `mcp motion::security_rejections_…` 15.371, `second_client_…` 12.246 | 47.783 | five goto rigs no longer built (each with a solved first sync, four with an accepted GoTo); one second-sync setup gone; MCP Stop cases' simulated time after the Stop 90 s → 26 s (900 → 260 ticks, about 180 → 52 frames) |
| US11 (optional) | `night::a_whole_night_…` (before US4) | 181.548 | on top of US4: frames 617 → 192 (1511 → 192 with US4); `/api/goto` requests 8 → 4; GoTos to done 6 → 2, halted 2 → 2; simulated time 312.8 s → 100.3 s |
| US12 (optional) | `mcp tracking::track_first_goto_…_ten_minutes` | 173.646 | frames 1,236 → 186 (the hold 1,200 → 150); the hold's simulated time 600 s → 75 s |
| US13 (optional) | `tracking::routes::readout_uses_…` 27.232, `tracking::tracker::pointing_samples_…` 0.483 | 27.715 | readout test frames 186 → 67 (simulated time 92.9 s → 154.4 s, 121 s of it with the camera off) |

For scale, the same log's per-test durations add up to 1,150 s across 621 tests. Across
finder's own tests, round 2's merged tree (`probe/028-r2-all` at c015cb7) renders 2,646
frames against 6,751 on f0ea2d5, and runs 2,736 solves against 6,840. That tree stored
US3's far sync alone (374 frames in the five-target test, against 446 with both).

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
| The five-target test stores both syncs of the old setup, the near one first and 120 s older, and keeps a live sync from the first 1° move. | Store only the far sync (round 1's decision); load the fixture from `motion/model/routes.rs`. | Round 2 showed that GoTo starts its cable-wind count from the model's first sync: with the far sync alone, the M81 → M1 GoTo reaches 386° from power-on, past the planner's ±360° limit, and nothing notices. With both stored, the wind state is the old setup's. The first 1° move still puts the gears on the flank the syncs use and gives a solved sync, so the product fits the tilt itself, from its own frame and the stored syncs. The model-route fixture carries fixed terms and a sync this mount does not have. |
| The stored syncs are computed from a second `SimMount`, and `second_from_file` checks the fitted tilt within 1′. | Type their numbers; check only that a tilt exists. | Numbers typed by hand would be an oracle nobody can check; the simulator's mount is the truth the test already uses. Round 2 found that a wrong fixture passes the five GoTos, which correct themselves, so only the tilt check catches it. |
| One centred-start recipe: after the sync, tick at most 10 times until the session's pointing is within 1′ of M42, then post Track. | Post Track at once after the sync (round 1's text). | The node starts capturing when the rig is built, so the first sync comes from the start direction and Track posted at once arms a GoTo; round 2 saw that at every centred site. |
| US10 keeps `detection_parameters_do_not_perturb_solve_math`. | Delete it with `explicit_detection_parameters_honored` (round 1's core-D6). | Round 2 found it holds the only assertion outside the oracle that detection finds isolated peaks on a zero background. The owner's approval covers only deletions that lose nothing or lose only what the skeptic's correction names. |
| US9 also moves the scratch folders of US6's `durable.rs`, 024's `crates/scope-node/tests/mount_link.rs` and 025's four recording test modules. | Leave them on their own paths. | With `durable.rs`'s own `Folder`, FR-026's one helper would not be literally true of finder's tests; `mount_link.rs` and 025's modules landed after the audit with the same hazard, and 025's would fail US9-S2's `../../target` check. |
| US11 and US12 add no guard assertion. | Assert the shortened span. | Their savings are read from the diff (US11-S2, US12-S2); US12's "more than 30 samples" holds at either span. |
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

1. **024 and 025 have landed. Met.** The owner's order is 024, 025, then 028 and 029.
   024 has landed (`main` at f0ea2d5) and is in `depends_on_landed`; `main` was merged
   into this branch there, every citation in a file 024 changed was re-read at f0ea2d5,
   and the Refinement Record's `shared_writes` name 024's stories where they last wrote a
   file this spec writes: `crates/scope-node/src/mount/mod.rs` (024 US1, which added trait
   defaults and left the test module US10 deletes alone; that module now opens at line
   99), `crates/finder/src/session.rs` (024 US3),
   `crates/finder/tests/motion/mount_identity.rs` (024 US3, new; US8 gives its inline
   solver a deadline that cannot fire) and `crates/scope-node/tests/mount_link.rs` (024
   US1, new; US9 moves its scratch folder). 024 also moved lines this spec cites in
   `crates/scope-node/src/service.rs`, `crates/finder/src/goto.rs`,
   `crates/finder/src/tracking.rs` and `crates/finder/src/session.rs`. 025 has landed
   its Rust stories, its spec flipped `ready` at ed7f581, its US1 at 101fa04 (#228) and
   its US2 at 9d8cc4e (#229), and is in `depends_on_landed`. `main` was merged into this
   branch at 9d8cc4e and every citation in a file 025 changed was re-read there: lines
   moved in `crates/finder/src/link.rs`, `crates/finder/src/api.rs`,
   `crates/scope-node/src/service.rs` and `crates/finder/tests/cli/main.rs`, and no cited
   fact changed. 025 US1 last wrote `crates/finder/tests/cli/main.rs` (`mod recording;`),
   which US6 writes to remove `mod session_node;`, and US9 now also moves the scratch
   folders of 025's four new test modules, US1's `crates/finder/tests/cli/recording.rs`
   and `crates/finder/tests/console/recording.rs` and US2's
   `crates/finder/tests/console/recording_raw.rs` and
   `crates/finder/tests/motion/recording_raw.rs`; the Refinement Record's
   `shared_writes` name those 025 stories. No 025 test holds an inline rig solver. 025's
   US3, the page's recording controls, landed at 820c558 (#230), completing 025, and
   `main` there was merged into this branch; it writes only web files (`DESIGN.md`, `.impeccable/design.json` and files under
   `crates/finder/web`), none of which this spec writes or cites by line. Both specs add
   modules to finder's binaries, which this spec does not reorder.
2. **The owner keeps or deletes US11, US12 and US13**, and the approvals they carry.
3. **The owner approvals are given.** Flipping gives them.
4. **Round 2's probe has built every kept story** in a throwaway worktree and measured
   each diff. Done on 2026-11-15 for all thirteen, and round 3 rebuilt US2, US3, US5,
   US8, US9 and US10 on their merged tree the same day (§ Refinement Record);
   § Measurements holds the probes' machine-independent counts. A story whose files 025
   changed is probe-built again on 9d8cc4e or later: US6, for
   `crates/finder/tests/cli/main.rs`, and US9, with 025's four recording modules, which
   round 4 built on 9d8cc4e.
5. **`crates/finder/tests` is still as read.** If a file there changes after 9d8cc4e,
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
5. **A centred start that waits too long, or not at all.** *Check:* after the sync the
   test ticks, at most 10 times, until the session's pointing is within 1′ of M42 (6° in
   the watchdog test, which then lets the watchdog fire), then posts Track, and its reply
   is asserted, not just the later state.
6. **Unused imports after a deletion.** *Check:* `cargo clippy` with `-D warnings`;
   `routes.rs` loses `ephemeris` and `sky`, `tracking/night.rs` loses `Instant`.
7. **A spec fixed mid-epic needs a relaunch.** *Check:* no builder edits
   `specs/028-finder-tests-do-less/`.
8. **Each worktree builds on its own.** *Check:* no story sets `CARGO_TARGET_DIR`.
9. **A new test file at the top of `crates/finder/tests` fails `layout`.** *Check:* the
   one new module is `console/session/durable.rs`, declared in its folder's `mod.rs`.
10. **The stored syncs must be the simulator's truth.** If US3's live sync opens a second
    epoch, the stored zero points or flank are wrong; if the fitted tilt is off, the
    stored directions are. *Check:* `second_from_file` asserts one epoch and the tilt
    within 1′ of the simulator's 1° toward 130°.
11. **Formatting is the repository's.** *Check:* `cargo fmt --all`, then
    `cargo fmt --all --check`.

## Risks

| Risk | Mitigation |
|---|---|
| US3's stored syncs do not match what solved frames would record (apparent altitude, flank). | They are computed from the simulator's mount, not typed; `second_from_file` asserts one epoch and a base tilt within 1′ of the simulator's, which round 2 saw fail for each wrong fixture it tried. If it still fails, US3 escalates rather than loosening a bound. |
| A frame interval change takes one frame to apply. | Every count and cadence bound allows one frame. |
| A change landing on `main` after 9d8cc4e touches a file this spec cites. | Prerequisites 1 and 5: re-read on `main` and re-probe. 025's US3 writes only web files. |
| An optional story is deleted after round 2 built on it. | Nothing depends on an optional story; its files are its own or come after the stories it waits for. |
| US9 and US10 are long lists across crates. | Each is a list of small, mechanical edits; the probes measured their diffs at 41 KB (US9, round 3) and 26 KB (US10, round 2, before it kept a test), under the 64 KiB preferred size, and US9's two added files are a few lines each. 025's four recording modules add 12,349 B of such edits to US9, whose whole diff is 53,272 B, under 64 KiB. |

## Refinement Record

Drafted on 2026-11-15 by the plate-solver operator against `main` at 06a7d4e. Round 1
drafted only: no story had been built. Writes come from the task text. Shared writes name
the landed story that last wrote each file, and, for a file two stories here write, the
later story under the earlier one. Nothing in this record is an owner approval: those are
in spec.md, and flipping to `ready` gives them.

**Round 2 (probe builds), 2026-11-15.** Each of the thirteen stories was probe-built in a
throwaway worktree from the trio as it stood at 9563141, stacked as the Work Graph orders
them, on `main` as 024 landed story by story: 06a7d4e, de9bfe6 (024 US1) or fc7d2bf (024
US2), with US9 and US10 ending on f0ea2d5. Every probe built, passed its story's tests and
the workspace's tests, was clean under clippy and fmt, wrote only its declared files and
failed no existing test. Diffs are each story's own, against the branch it was built on:

| Story | Branch | Built on | Diff |
|---|---|---|---|
| US1 | `probe/028-r2-us1` at 654f0de | de9bfe6 | 11,041 B |
| US2 | `probe/028-r2-us2` at 49b7482 | de9bfe6 | 10,664 B |
| US3 | `probe/028-r2-us3` at 4828efd | 06a7d4e | 8,104 B |
| US4 | `probe/028-r2-us4` at 28432e9 | de9bfe6 | 2,526 B |
| US5 | `probe/028-r2-us5` at 294a9a5 | US3 | 4,629 B |
| US6 | `probe/028-r2-us6` at dee6f91 | de9bfe6 | 16,600 B |
| US7 | `probe/028-r2-us7` at 1c48c3c | fc7d2bf | 3,925 B |
| US8 | `probe/028-r2-us8` at cda4f71 | US3 and US4 on de9bfe6, then fc7d2bf merged | 7,789 B |
| US9 | `probe/028-r2-us9` at 47d2d04 | US1, US5, US6 and US8 on fc7d2bf, then f0ea2d5 merged | 37,169 B |
| US10 | `probe/028-r2-us10` at 1259323 | US9 | 25,543 B |
| US11 (optional) | `probe/028-r2-us11` at 54b6623 | US4 | 1,026 B |
| US12 (optional) | `probe/028-r2-us12` at 5b72961 | fc7d2bf | 1,480 B |
| US13 (optional) | `probe/028-r2-us13` at a124e06 | US2 | 2,964 B |

All thirteen were then merged into one tree on f0ea2d5 (`probe/028-r2-all` at c015cb7)
with no conflicts: fmt, clippy and the doc tests were clean, and nextest passed 644 of 644
tests twice, against 660 on `main` by the same count, so the round nets −16 tests (−15
once US10 keeps `detection_parameters_do_not_perturb_solve_math`). In that tree every
file two stories write has a `depends_on_merged` path between them in the Work Graph,
and US7 and US12 share no file with any other story.

The probes, and the operator's decisions on what they found, changed this trio as
follows. None of them changes the owner's settled answers: two specs, the oracle
untouched, the approved trades and the optional stories as leaves of the graph.

- **One centred-start recipe** (US1-S3, the four US2 sites, US5). Round 1's text failed
  at every site: the node starts capturing in `Rig::new`, so the first sync comes from
  the start direction and Track arms a GoTo. After the sync each site now ticks at most
  10 times until the session's pointing is within 1′ of M42, then posts Track; frame 2,
  at about 0.8 s, is the first on M42. The recipe is written once (§ Interfaces "A
  centred start"), and the edge case and Key Entities' "Centred start" say so.
- **US1:** the sunrise test's binding is `last_frame`, because `stopped` is already its
  stop time; the simulator rig's two constructors share a private
  `async fn on<C: Camera + 'static>`.
- **US2:** the watchdog test ticks until the session's pointing is within 6° of M42 and
  asserts it ("no frame solved after the push"); without that its later GoTo plans from
  a frame exposed before the push. US2-S4 and FR-007 say so.
- **US3:** `second_from_file` checks the fitted base tilt within 1′ of the simulator's
  1° toward 130°, since a wrong fixture passes the five GoTos (US3-S1 reworded). The
  state file now holds the near sync too, at (4500, 4500) and 120 s before the start:
  with one stored sync the M81 → M1 GoTo reaches 386°, past the planner's ±360° limit.
  That reverses round 1's decision; § Decisions and owner approval 4 now say two of the
  three syncs come from a file and the live 1° sync still solves. The helpers are
  `far_sync(path, start, alt, az)`, `mount_config(alt, az)`, and a model with its own
  `rms_arcsec`.
- **US4:** US4-S2 and T020 reworded so the cadence assertion is part of the diff; it
  goes after `crates/finder/tests/motion/night.rs:164`.
- **US5:** the test makes its own JSON `POST /api/track` and ticks, at most 100 times,
  until the true direction changes, because the gears start mid-backlash.
- **US6:** `durable.rs` scratches under `CARGO_TARGET_TMPDIR`; `request(method, path,
  body)` replaces the backlash test's old helper; the token is compared in the stored
  file (US6-S1); two session.rs citations were off at drafting and are fixed.
- **US7:** the preview assertion goes after the loop's closing brace; US7-S2 takes the
  probe's clearer wording.
- **US8:** US8-S4 is read from the diff and names the child's arguments, not its config,
  which holds the token; US8-S5 takes the probe's wording; the other 5 s solver
  deadlines and `cli/simulate.rs`'s 15 s first-pointing wait are named out of scope.
- **US9:** T042 and § Interfaces name the keyhole and tracking-night rigs, `writes.rs`'s
  path and `catch_unwind`, the file guards of scope-node's `cli.rs` and
  `mount_config.rs`, and `rpicam.rs`'s `#[cfg(unix)]`. US9 also takes US6's
  `console/session/durable.rs` (as `Scratch::new("session-durable")`) and 024's
  `crates/scope-node/tests/mount_link.rs` (a fourth scope-node file under US9-S4 and
  FR-028), both added to its writes.
- **US10:** `detection_parameters_do_not_perturb_solve_math` stays (US10-S1, FR-030, the
  tasks and owner approval 12 no longer delete it), so the story removes 14 tests;
  approval 12 says three renamed and one moved and names them; the twin of
  `inline_image_bytes_used_when_shmem_absent` and of the foreign `night` reset are
  named. SC-003 now counts 16 removed.
- **US11:** US11-S2 reworded so the interrupted slews visibly stay; no guard assertion.
- **US12:** US12-S2 and T057 allow `ten-minute` in the closing message to change; both
  loop literals and spec 016's line are named; no guard assertion.
- **US13:** the 750-tick bound and the injected sample's values are stated.

Round 2's counts of the work each story removes are in § Measurements; its wall-clock
timings are not part of this trio.

**Re-anchored to `main` at f0ea2d5, 2026-11-15**, once 024 had landed: `main` was merged
into this branch, every citation in a file 024 changed was re-read at f0ea2d5 and moved
with its line (in `crates/finder/src/session.rs`, `goto.rs` and `tracking.rs`, and
`crates/scope-node/src/service.rs` and `mount/mod.rs`), and the shared writes of
`session.rs` and `mount/mod.rs` now name the 024 stories that last wrote them. 025 is
still to land and re-anchors the trio again (§ Prerequisites 1).

**Round 3 (delta probe), 2026-11-15.** A review of the folded trio (c5a314e) re-read its
210 cited lines and found one blocker and thirteen smaller fixes. The stories those
touch, and the round-2 decisions no committed probe had yet built, were then rebuilt as
one stack on round 2's merged tree: `probe/028-r3-delta`, head f00d4e9 on c015cb7, one
commit per story:

| Commit | Story | What it builds |
|---|---|---|
| 3180d41 | US3 | both syncs stored, the near one first and 120 s older; the fitted tilt checked |
| e668983 | US5 | the centred-start wait, reading the pointing through `rig.view()` |
| e37c8e2 | US9 | `durable.rs` on `Scratch`; a file-local guard in `mount_link.rs` |
| 2de1d0b | US10 | `detection_parameters_do_not_perturb_solve_math` restored, byte-identical |
| de7419d | US8 | `mount_identity.rs`'s inline solver on `timeout_ms: u64::MAX` |
| f00d4e9 | US2 | `centred()` also asserts M42 and `wanted` |

nextest passed 645 of 645 tests twice, and the doc tests, clippy and fmt were clean;
sources were touched before every run and the junit test list was checked against the
source. The review and the probe changed this trio as follows. None of it changes the
owner's settled answers.

- **Optional stories (the review's blocker):** two intra-spec `merge_order` shared writes
  named an optional story (US8 → US11 on `motion/night.rs`, US2 → US13 on
  `motion/tracking/routes.rs`), so deleting either story made the validator refuse.
  Both are gone; the `depends_on_merged` edges still order those merges. spec.md's
  deletion rule now also names the story's Work Graph row, its line in tasks.md
  § Execution Order and its row in § Measurements, and § Execution Order gives each
  optional story a line of its own. Deleting US11, US12 and US13 from scratch copies by
  that rule, one at a time and then all three, validates with 0 findings.
- **US2:** `centred()` also asserts that Track's reply names M42 with `wanted` true, the
  two checks of `tracking()` that are not about the handoff; each mutation fails all
  four tests. US2-S1 and approval 3 say which handoff checks the four tests no longer
  make, SC-003 counts approval 3, and US2-S2 states the min_alt test's assertions
  itself.
- **US3:** when US3-S1 is asserted the model holds three syncs (the near, the far and
  the live one, all in epoch 0, with a fitted tilt of 0.99951° toward 130.00012°), and
  M1 ends at 26.3° of azimuth. With the far sync alone "at least two syncs" still
  passed and M1 ended at 386°; `assert_eq!` on three fails (2 against 3). US3-S1,
  § Interfaces "US3", T014 and this record now say three syncs. A tilt target of 131°
  fails the tilt check. The five-target test renders 446 frames and the cable-wind test
  62, as § Measurements says.
- **US5:** the recipe's wait ends after 5 ticks and the true direction changes after 61
  of the 100 ticks allowed; 14 frames are rendered in process. Without the wait Track
  arms a GoTo from frame 1, 28.7° away, and the test fails. § Interfaces "US5" reads the
  pointing through `rig.view()` and keeps "some seconds" for the take-up of the play.
- **US8:** 024 US3's `crates/finder/tests/motion/mount_identity.rs` holds a fifth inline
  solver with the 5 s default. It joins US8-S2, FR-022, approval 11, § Project Structure,
  § Interfaces "US8", T035 and US8's writes, with a shared write naming 024 US3. The
  solve-stall figures in § Interfaces "US8" are round 3's.
- **US9:** `durable.rs` takes `Scratch::new("session-durable")`; `mount_link.rs` gets a
  file-local `Scratch` and loses its `remove_dir_all`, which § Interfaces "US9" now says
  outright. A full run leaves nothing behind, and nothing is left when either test is
  made to panic. US9's diff is 40,923 B.
- **US10:** with `detection_parameters_do_not_perturb_solve_math` back, the story removes
  exactly its 14 tests, and the workspace runs 645 = 660 − 16 + 1.
- **The review's other fixes:** approval 12 names the 8×8 image size as lost; approvals 15
  and 16 give their tests' new names; US3-S4 says `Rig::new`'s body is one call of
  `with_session` and that no test outside `motion/goto/sim.rs` changes; trap 5 says the
  watchdog test's wait lets the watchdog fire; tasks.md's rules allow every command of
  a story's development checks; and four citations were corrected
  (`LinkManager::frames()` in `crates/finder/src/link.rs`,
  `crates/finder/tests/motion/stop/watchdog.rs:205`, the night's loop at line 198 once
  US4 and US8 have landed, and `state`'s other six callers).
- **Counts:** § Measurements gives finder's frame and solve totals, counted on round 2's
  merged tree, and US10's "before" is the GitHub log's sum, 47.783 s.

Round 3's wall-clock timings are not part of this trio either.

**Re-anchored to `main` at 9d8cc4e, 2026-11-15**, once 025's US1 and US2 had landed:
`main` was merged into this branch, and every citation in a file 025 changed was re-read
at 9d8cc4e and moved with its line: in `crates/finder/src/link.rs` (218 to 250, 486 to
537), `crates/finder/src/api.rs` (243 to 247, 1130 to 1244),
`crates/scope-node/src/service.rs` (55 to 56, 71 to 72, 233 to 240, 235 to 242) and
`crates/finder/tests/cli/main.rs` (11 to 12). No cited fact changed. 025 joins
`depends_on_landed`, `reviewed_revision` is 9d8cc4e, and US6's shared write of
`cli/main.rs` names 025 US1, its last writer. 025's four new test modules build their
scratch folders under `../../target`, so US9 takes them: its writes, shared writes naming
025 US1 and US2, four rows and a paragraph in § Interfaces "US9", T041, T042 and T044,
and owner approval 13. Two of their tests assert their folder absent, so there the folder
is a path inside the `Scratch` that does not exist yet. None of them holds an inline rig
solver, so US8 gains no site; § Interfaces "US8" and spec.md's Assumptions name the
`Live::start` with `DbSolver` that one of them builds among the out-of-scope deadlines.
025's US3 writes only web files, none of which this trio cites; it landed at 820c558
(#230), completing 025, and `main` there was merged in with no cited line moved.

**Round 4 (probe on 025's code), 2026-11-15.** Every story's round-3 probe code, merged
with `main` at 9d8cc4e (probe/028-r4-025, no conflicts), passed the workspace's 662
tests twice, with fmt, clippy and the doc tests clean: 645 of this spec's and 025's 17
new ones. US9's edits to 025's four recording modules were then built as § Interfaces
"US9" words them (probe/028-r4-us9-025): their 15 tests and the workspace's 662 pass
twice, nothing is left under the worktree's `target/` or in `CARGO_TARGET_TMPDIR`, a test
in each file panicked on purpose after making its folder leaves nothing, and the refusal
test's two absent-root assertions still fail when the root is made first. Those four
files add 12,349 B, and US9's whole diff is 53,272 B across its 27 writes. No scenario
or § Interfaces wording changed.

```yaml
version: 1
reviewed_revision: 9d8cc4e78c7a0762976303d5295caafd7fbada2f
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
      assertion: "four tests reach tracking from a centred start and assert the Track reply is tracking, names M42 with wanted true and holds no goto"
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
      assertion: "five-target test starts from the stored near and far syncs with 4500 azimuth counts, a base tilt within 1 arcminute of the simulator's, three syncs and one epoch, then its five GoTos unchanged; cable-wind move advances the frame id by 20 to 60"
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
    - path: crates/finder/tests/console/session/durable.rs
      other_spec: 028-finder-tests-do-less
      other_story: US9
      disposition: merge_order
      reason: "US9 depends_on_merged US6 and moves this new file's scratch folder to the shared helper"
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
      other_spec: 025-frame-capture
      other_story: US1
      disposition: merge_order
      reason: "028 depends_on_landed 025-frame-capture, whose US1 last wrote this file, adding mod recording"
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
      assertion: "a child still running after 10 s is killed and fails the test naming its arguments; both tests' assertions unchanged"
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
    - crates/finder/tests/motion/mount_identity.rs
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
    - path: crates/finder/tests/motion/mount_identity.rs
      other_spec: 024-mount-link-recovery
      other_story: US3
      disposition: merge_order
      reason: "028 depends_on_landed 024-mount-link-recovery, whose US3 added this file"
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
    - "025's console/recording.rs still asserts that refused requests leave no recordings folder, and cli/recording.rs that the binary creates its state folder"
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
    - crates/finder/tests/console/session/durable.rs
    - crates/scope-node/tests/cli.rs
    - crates/scope-node/tests/mount_config.rs
    - crates/scope-node/tests/rpicam.rs
    - crates/scope-node/tests/mount_link.rs
    - crates/finder/tests/cli/recording.rs
    - crates/finder/tests/console/recording.rs
    - crates/finder/tests/console/recording_raw.rs
    - crates/finder/tests/motion/recording_raw.rs
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
    - path: crates/scope-node/tests/mount_link.rs
      other_spec: 024-mount-link-recovery
      other_story: US1
      disposition: merge_order
      reason: "028 depends_on_landed 024-mount-link-recovery, whose US1 added this file"
    - path: crates/finder/tests/cli/recording.rs
      other_spec: 025-frame-capture
      other_story: US1
      disposition: merge_order
      reason: "028 depends_on_landed 025-frame-capture, whose US1 added this file"
    - path: crates/finder/tests/console/recording.rs
      other_spec: 025-frame-capture
      other_story: US1
      disposition: merge_order
      reason: "028 depends_on_landed 025-frame-capture, whose US1 added this file"
    - path: crates/finder/tests/console/recording_raw.rs
      other_spec: 025-frame-capture
      other_story: US2
      disposition: merge_order
      reason: "028 depends_on_landed 025-frame-capture, whose US2 added this file"
    - path: crates/finder/tests/motion/recording_raw.rs
      other_spec: 025-frame-capture
      other_story: US2
      disposition: merge_order
      reason: "028 depends_on_landed 025-frame-capture, whose US2 added this file"
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
      assertion: "one test deleted; the getter check and the empty-catalog NO_MATCH kept under honest names; detection of isolated peaks on a zero background kept"
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
    - "detection_parameters_do_not_perturb_solve_math is unchanged (spec.md FR-030)"
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
      other_spec: 024-mount-link-recovery
      other_story: US1
      disposition: merge_order
      reason: "028 depends_on_landed 024-mount-link-recovery, whose US1 last wrote this file, adding trait defaults before US10 deletes its test module"
    - path: crates/finder/src/session.rs
      other_spec: 024-mount-link-recovery
      other_story: US3
      disposition: merge_order
      reason: "028 depends_on_landed 024-mount-link-recovery, whose US3 last wrote this file before US10 deletes two of its inline tests"
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
