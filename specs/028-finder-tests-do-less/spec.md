---
state: draft
depends_on_landed: ["012-targets-and-push-to", "013-live-finder", "014-goto", "015-tracking", "016-mcp", "017-console-stop-confirm", "018-live-chart-steady", "019-night-legibility", "020-live-chart-calm", "024-mount-link-recovery", "025-frame-capture", "027-finder-tests-by-system"]
# Drafted 2026-11-15 against `main` at 06a7d4e, after spec 027 landed, from the test-layer
# audit of 2026-11-14 and the owner's answers of 2026-11-15. It builds after 024 and 025
# have landed (plan.md § Prerequisites). It stays draft until the owner flips it to
# `ready`, which gives the approvals listed under "Owner approvals" and dispatches it.
# The three optional stories (US11, US12, US13) are each kept or deleted by the owner
# before the flip; nothing depends on them.
# Refined 2026-11-15 (round 2) by probe builds of all thirteen stories from the trio as it
# stood, each in a throwaway worktree, then all thirteen merged in one tree; the scenarios
# the probes reworded are adopted, and the operator's decisions on what they found are in
# plan.md § Refinement Record. The same day it was
# re-anchored to `main` at f0ea2d5 (024 landed): every citation in a file 024 changed was
# re-read there. It still builds after 025, which re-anchors it again.
# Refined again 2026-11-15 (round 3) after a review of the folded trio: a delta probe
# rebuilt US2, US3, US5, US8, US9 and US10 on the round-2 tree, and its findings and the
# review's fixes are folded in (plan.md § Refinement Record). US8 now also covers 024's
# `motion/mount_identity.rs`. The trio is still anchored at f0ea2d5.
# Re-anchored 2026-11-15 to 025-frame-capture's US1 and US2 (main 9d8cc4e), and 025 joins
# `depends_on_landed`: every citation in a file 025 changed was re-read there and moved
# with its line, US6's shared write of `crates/finder/tests/cli/main.rs` names 025 US1,
# and US9 also takes the scratch folders of 025's four new test modules, which build them
# under `../../target`. None of 025's tests holds an inline rig solver. 025's US3, which
# landed at 820c558 and completed 025, writes only web files this trio does not cite;
# `main` there was merged in. Citations are read at 9d8cc4e, unchanged at 820c558
# (plan.md § Prerequisites 1 and § Refinement Record).
---

# Feature Specification: Finder's Tests Do Less Work

**Feature Branch**: `028-finder-tests-do-less`

**Created**: 2026-11-15

**Status**: Draft

**Input**: After spec 027 the `test` gate on GitHub spends more time running Rust tests than
compiling them. In the GitHub `test` job of ergane gates run 43683194483 (on `main` at
b8dceaa) the test build took 4m 04s, the nextest summary reads 339.674 s for 621 tests, and
the per-test durations in that log add up to 1,150 s, of which `finder::motion` is 615 s
and `finder::mcp` 361 s. The audit found why: simulated time is free, but every simulated
camera frame is rendered pixel by pixel, detected and solved, and most tests read few of
the frames they pay for. Some tests also reach their starting state through a long slew,
start the built binary to check library code, or depend on how fast the machine is. This
spec makes finder's Rust tests, and a few of their neighbours in other crates, do less of
that work. Every saving comes with an assertion, or a line in the diff, that shows if the
waste comes back.

## Clarifications

### Session 2026-11-15 (review of `main` at 06a7d4e)

- Q: What does this trio read besides itself? → A: Nothing. `spec.md` holds every
  requirement and scenario, and `plan.md` holds every design decision, the edit each
  story makes, the measurements and the alternatives rejected. A story that would change
  a decision escalates rather than re-deciding it.
- Q: Where does the time go? → A: In run 43683194483, the longest tests are the whole
  night (`finder::motion night::a_whole_night_syncs_five_gotos_hand_push_stop_and_dropped_link`,
  181.548 s), the MCP ten-minute hold (`finder::mcp
  tracking::track_first_goto_keeps_mcp_source_and_holds_m42_for_ten_minutes`, 173.646 s)
  and the five-target GoTo (`finder::motion
  goto::sim::five_catalog_targets_converge_after_two_syncs_and_report_progress`,
  80.944 s). The whole night runs its node at the production 200 ms frame interval
  (`crates/finder/src/simulate.rs:66` builds the node with no interval; the default is
  `crates/scope-node/src/service.rs:56`), against 500 ms in every test rig.
- Q: What did the owner decide? → A: On 2026-11-15:
  1. Two specs. This one is the Rust side; the page's Playwright tests are spec 029.
     This one builds after 024 and 025 have landed.
  2. The cedar oracle stays put: no change to
     `crates/plate-solver/tests/reference_parity.rs`, `image_parity.rs`,
     `crates/grpc-service/tests/e2e_service.rs`, `crates/star-detection/tests/parity.rs`,
     `e2e-eval` or `e2e-canary`.
  3. Four coverage trades are approved (owner approvals 5, 6, 10 and 12 below).
  4. Three trims of landed scenarios are optional stories, kept or deleted at the flip.
  5. Not now: CI cache fixes, a merge-queue-only slow tier, zero-test targets in
     `Cargo.toml`, and a lower per-test time limit.
- Q: What does a story's evidence look like? → A: A committed test whose new assertion
  fails if the work comes back ("the console received no more than one frame after the
  camera stopped" is such an assertion; "it got faster" is not one the judge can see), or
  a line the judge can read from the diff. Measured seconds are not evidence for the
  judge; they are for the owner, in plan.md § Measurements.
- Q: Does a deleted or shortened test weaken a gate? → A: The constitution's rule II
  says it does unless the spec names it for the owner's approval. Every deletion, every
  edit to a test body and every coverage given up is named under "Owner approvals", with
  the coverage-lost text of the audit's skeptic.
- Q: Why does each story's Independent Test name `cargo nextest run`? → A: It is the
  first command of the `test` gate, word for word. Under nextest every test is its own
  process, so a test's cost is its own duration: removing work from any test counts,
  wherever it is.

## Owner approvals

Flipping this spec to `ready` approves the following. The constitution's rule II lets a
story edit an existing test only where its spec names the edit for the owner's approval;
these are those edits. No oracle file, golden, `Cargo.toml`, `Cargo.lock`, `scripts/ci/`
file, workflow, `CODEOWNERS` entry, `ergane.yaml` or `.config/nextest.toml` is edited.

1. **Test bodies are edited, as each FR says and no further.** The edits are listed per
   story in plan.md § Interfaces.
2. **The camera stops in six MCP direct-move tests, one MCP Track test starts centred on
   M42, and the simulator crate's Stop and watchdog rig uses the synthetic camera** (US1,
   FR-001 to FR-003; findings mcp-S2, mcp-S5, node-S5). Coverage lost: none.
   `frame_positions.rs` stays on the catalog camera, so it still exercises the rendered
   camera's exposure times with mount sampling.
3. **Track tests start centred on M42 where their claim is not the GoTo handoff** (US2,
   FR-004 to FR-007; findings trk-S2, trk-S3, trk-S4, trk-S9). Coverage lost: none; the
   four routes tests no longer make `tracking()`'s handoff checks (GoTo done, Track
   within 2 s of it), which stay in the three tests about the handoff.
4. **The five-target GoTo test loads both syncs of its old setup from a state file, and
   the cable-wind test samples its 350° move every 2 s** (US3, FR-008 to FR-010; findings
   simA-S2, simA-S4). Coverage lost: two of the model's three syncs, the near one and the
   far one, come from a file written from the simulator's own mount, not from frames
   solved in this session, so "GoTo accuracy would no longer follow from a model the
   product fitted itself from two solved frames" holds for those two; the third, from the
   first 1° move, is still a solved frame, the product still fits the base tilt from the
   three itself, and the five GoTos stay chained on one session with today's cable-wind
   state. During the 350° move "the 'continuously observed' sampling goes from 1.7 deg to
   about 7 deg steps"; the direction assertion after the power cycle is unchanged.
5. **The whole night runs at a 500 ms frame interval** (US4, FR-011, FR-012; finding
   simA-S1), through a new `Simulator::set_frame_interval` that forwards to the existing
   `NodeControls` setter. Coverage lost: "The night is the only paused-time GoTo /
   second-sync scenario that runs at the production 5 frames/s cadence; at 500 ms it runs
   at the same cadence as goto_sim, leaving the default cadence to simulate_cli and
   Playwright only."
6. **Track's restart test tracks in paused time in process, then starts the built binary
   once** (US5, FR-013; finding trk-S5). Coverage lost: "The built binary would no longer
   start Track (and its GoTo handoff) through main.rs wiring; and 'no track in
   session.json' would be checked on a file written by an in-process Session dropped
   cleanly rather than by a real process killed mid-track." The test says that
   Playwright's `tracking.spec.ts` owns Track on the real binary.
7. **The node-settings restart is checked in process; the token-in-stderr check moves to
   `cli/aim.rs`; the backlash fixed reset joins the binary restart test** (US6, FR-014 to
   FR-017; findings api-D3, simA-D6). `cli/session_node.rs` and its one test, and
   `tracking::backlash::first_transit_measures_then_fixed_reset_removes_the_observed_value`,
   are deleted. Coverage lost: node credentials and both camera settings survive a
   restart through `Session::with_state_file`, which the binary's `main.rs` calls, rather
   than through a restart of the built binary; `cli/aim.rs` remains the restart check on
   the real binary, with the default binary pairing with a node over TCP. The fixed reset
   loses nothing: it is posted to the restarted binary after the two restarts.
8. **One new test module and one fewer.** `crates/finder/tests/console/session/durable.rs`
   is new and adds `mod durable;` to `crates/finder/tests/console/session/mod.rs`, a file
   other specs also edit; `mod session_node;` leaves `crates/finder/tests/cli/main.rs`.
9. **Two console tests run their node at a slower frame interval** (US7, FR-018 to
   FR-020; findings api-S1, api-D1, api-S2). Coverage lost: none. At 3 s the aim test's
   previews fall on alternate frames, so it still tells "eight previews" from "eight
   frames".
10. **The hour of tracking no longer bounds its own wall time** (US8, FR-021; finding
    trk-H1). The assertion that the hour ran in under 30 wall seconds is removed and the
    test's name loses `_in_under_thirty_wall_seconds`; nextest's per-test limit in
    `.config/nextest.toml` (ended after 300 s) catches a runaway. Spec 015's US9-S1,
    FR-020 and SC-003 keep their text; no test asserts their 30 s any more.
11. **Four hazards are removed without changing what is asserted** (US8, FR-022 to
    FR-025; findings mcp-H1, simA-H2, node-H1, node-H3): the test rigs' inline solvers,
    and the one 024 added in `crates/finder/tests/motion/mount_identity.rs`, get a solve
    deadline that cannot fire; `cli/simulate.rs` reads the session time just
    before its 5 s window; scope-node's CLI helper kills a child that has not exited
    after 10 s; three `rpicam::metadata` unit tests run on a paused clock. Coverage lost:
    none.
12. **Duplicates and tests that cannot fail are deleted or reduced** (US10, FR-029 to
    FR-035), as one story. Confirmed by the skeptic with nothing lost: core-D3, core-D4,
    core-D7, node-D1, node-D3, api-D9, simA-D5. Applied with the skeptic's correction
    only: core-D5, core-D6, node-D2, simA-D4, simA-D9, mcp-D3. Coverage lost: the 8×8
    image size of `inline_image_bytes_used_when_shmem_absent`, whose twin
    `extract_centroids_reports_wire_x_as_column` sends inline bytes of another size; and
    one statement kept on record: `SolveStatus::Cancelled` has no test and no public way
    to trigger it (core-D5). Of core-D6, `detection_parameters_do_not_perturb_solve_math`
    stays: it is the only test outside the oracle that detects isolated peaks on a zero
    background. Three test functions are renamed for what they check, and one is moved
    into its neighbouring module: `brightest_first_requirement` becomes
    `exhausted_search_of_an_empty_pattern_catalog_is_no_match`, `cancellation_honored`
    becomes `solve_context_reports_a_set_cancel_flag`, `both_stops_precede_emergency_fallback`
    becomes `stop_all_stops_both_axes`, and
    `patterns::bfc_order_tests::matches_cedar_order_and_visits_each_combination_once` moves
    to `patterns::tests::matches_cedar_order_and_visits_each_combination_once` (plan.md
    § Interfaces "US10").
13. **Scratch folders move under `CARGO_TARGET_TMPDIR` and are removed on drop** (US9,
    FR-026 to FR-028; findings api-H2, mcp-H2, simA-H1, trk-H4, node-H2), including
    those of US6's `console/session/durable.rs`, of 024's
    `crates/scope-node/tests/mount_link.rs`, and of 025's `cli/recording.rs`,
    `console/recording.rs`, `console/recording_raw.rs` and `motion/recording_raw.rs`.
    Coverage lost: none. Where a test's removal of its folder was itself a check, the
    check stays as an assertion, and where a test asserts its folder absent, the folder
    is a path inside the scratch folder that does not exist yet.
14. **Code outside test files**: `crates/simulator/src/controls.rs` gains
    `Simulator::set_frame_interval` (FR-011). Test modules inside four product source
    files change: `crates/scope-node/src/rpicam/metadata.rs` (FR-025),
    `crates/scope-node/src/mount/mod.rs`, `crates/finder/src/session.rs` and
    `crates/database-generation/src/patterns.rs` (FR-031 to FR-033). No product code
    outside a test module changes.
15. **Optional, US11: the whole night is shortened** (FR-036; finding simA-D3). Kept, it
    amends spec 014 US9's "five GoTos" to one GoTo far from M42 (M81) after the one-tap
    second sync, then the hand push and its GoTo, the Stop and the dropped link. Coverage
    lost: "Four of the five GoTo convergences (M31, M45, M81, M1 within 60 arcsec by true
    eyepiece) on the production simulate wiring with the default mount mechanics and a
    model fitted by the one-tap second sync; the hand push after five GoTos (state
    carried across the whole night)." As drafted M81 is kept, so the lost convergences
    are M42 as the first GoTo, M31, M45 and M1. The test is renamed
    `a_whole_night_syncs_gotos_hand_push_stop_and_dropped_link`.
16. **Optional, US12: the MCP hold is shortened from ten minutes to 75 s** (FR-037;
    findings mcp-D1, trk-D6). Kept, it amends spec 016 US6-S1's "10 min, sampled every
    1 s". Coverage lost: "The only ten-minute 1 arcminute full-pipeline hold, and 016
    US6-S1's '10 min, sampled every 1 s'. A 60-75 s window cannot show slow closed-loop
    drift." The test is renamed
    `track_first_goto_keeps_mcp_source_and_holds_m42_for_seventy_five_seconds`.
17. **Optional, US13: the sixty-second window is proven below the route** (FR-038;
    finding trk-D2). Kept, it amends how spec 015 US7-S7 is proven. Coverage lost: "At
    the route with live rendered solves: samples older than 60 s excluded and at least
    50 s retained; the readout RMS compared when older samples exist; and RMS <= 20
    arcsec over a full minute of closed-loop tracking rather than ~16 s." The route keeps
    a proof of the window with one injected sample.

Deleting an optional story before the flip deletes its User Story section, its phase in
tasks.md, its entry and row in the Work Graph, its line in tasks.md § Execution Order,
its story in the Refinement Record, its FR, its approval, its row in plan.md
§ Measurements, its row in plan.md § Project Structure and its section of plan.md
§ Interfaces; the other approvals keep their numbers. Other sentences that name it stay
as the record of what was offered. Nothing else changes.

## User Scenarios & Testing *(mandatory)*

Every scenario's tests run in
`cargo nextest run --workspace --all-targets --all-features --locked`, the first command
of the `test` gate. A test is named by its binary and module path, as nextest prints it.
"The newest frame id" is the id of the newest frame the console's scope link holds,
`rig.state.link().unwrap().frames()`. A scenario that begins "Given the story's diff" is
read from the diff.

### User Story 1 - The camera stops where no assertion reads a frame (Priority: P1)

As the maintainer, I have the tests that move the mount directly, and the simulator
crate's Stop and watchdog tests, run without rendering frames nobody reads.

**Why this priority**: Two MCP slews alone took 60 s in run 43683194483, and the change
loses nothing.

**Independent Test**:
`cargo nextest run --workspace --all-targets --all-features --locked` runs
`crates/finder/tests/mcp/axis_moves.rs`, `crates/simulator/tests/stop_and_watchdog.rs`,
`crates/simulator/tests/watchdog_report.rs` and `crates/simulator/tests/frame_positions.rs`.

**Acceptance Scenarios**:

1. **Given** the `mcp` binary, **When** `axis_moves::braking_distance_is_checked_at_the_cable_limit` and `axis_moves::cable_wind_refuses_one_point_two_turns_and_accepts_point_eight` make their long azimuth moves, **Then** each committed test stops the camera after its first sync and asserts that the newest frame id at its end is at most one more than the newest frame id when the camera stopped, and keeps every assertion it made before.
2. **Given** the `mcp` binary, **When** `axis_moves::rate_duration_and_acceleration_ramp_match_the_simulator_and_bad_duration_moves_nothing`, `axis_moves::disconnected_client_does_not_end_heartbeats_or_leave_a_rate_running`, `axis_moves::sunrise_halts_with_the_page_guard_reason_within_one_second` and `axis_moves::a_zero_rate_still_reserves_motion_until_its_duration_ends` run, **Then** each committed test stops the camera after its first sync and asserts the same bound on the newest frame id.
3. **Given** the `mcp` binary, **When** `axis_moves::track_through_the_page_blocks_all_three_mcp_moves_and_keeps_tracking` starts, **Then** the committed test centres the eyepiece on M42 before its first sync, waits for the first frame solved on M42, and asserts that the session reads `tracking` with no `goto` after the page's Track request, before its three refusals.
4. **Given** the simulator crate's shared test rig, **When** `Rig::new` builds the node, **Then** it builds it on the synthetic camera with no pattern database and asserts that the node's `Describe` names the camera `simulated`, and every test of `stop_and_watchdog.rs` and `watchdog_report.rs` passes unchanged on it.
5. **Given** the story's diff, **When** it is read, **Then** `frame_positions.rs` builds its rig with `Rig::rendered`, on the catalog camera, and its only changed lines are that call and its clock reads.

---

### User Story 2 - Track tests start where their claim starts (Priority: P1)

As the maintainer, I have tests whose claim is about running Track, its refusals or its
Stop start with M42 already in the eyepiece, so they do not pay for a GoTo first.

**Why this priority**: Nine tests take 64.6 s together in run 43683194483, mostly in a
GoTo none of them asserts, and the change loses nothing.

**Independent Test**:
`cargo nextest run --workspace --all-targets --all-features --locked` runs
`crates/finder/tests/motion/tracking/routes.rs`, `crates/finder/tests/motion/tracking/min_alt.rs`,
`crates/finder/tests/motion/stop/motion_ends.rs`, `crates/finder/tests/motion/stop/retry.rs`
and `crates/finder/tests/motion/stop/watchdog.rs`.

**Acceptance Scenarios**:

1. **Given** the `motion` binary, **When** `tracking::routes::below_horizon_and_foreign_origin_refuse_after_positive_track_control`, `tracking::routes::foreign_bodiless_stop_ends_tracking_and_no_rate_restarts_it`, `tracking::routes::track_names_missing_mount_sync_site_target_and_sun_refusals` and `tracking::routes::dropping_console_cancels_tracking_without_retaining_the_session` reach tracking, **Then** each committed test starts with the eyepiece centred on M42 before the first sync and asserts that Track's reply reads `tracking`, names M42 with `wanted` true and holds no `goto`, then keeps every assertion of its own body; only `tracking()`'s GoTo-handoff checks (GoTo done, Track within 2 s of it) are no longer made.
2. **Given** the `motion` binary, **When** `tracking::min_alt::raised_minimum_altitude_ends_running_tracking_as_too_low` reaches tracking, **Then** the committed test starts with the eyepiece centred on M42 before the first sync, asserts that Track's reply reads `tracking` and holds no `goto`, and keeps its off, at-rest and `too_low` assertions.
3. **Given** the `motion` binary, **When** `stop::motion_ends::track_stop_starts_promptly_and_retries_to_the_exact_acknowledged_body`, `stop::motion_ends::track_stop_shares_one_deadline_across_both_tracker_calls` and `stop::retry::retained_tracker_cannot_delay_goto_cancellation_or_first_attempt` reach tracking, **Then** each committed test centres the eyepiece on M42 before the first sync and asserts that the command log holds no GoTo command when tracking starts.
4. **Given** the `motion` binary, **When** `stop::watchdog::past_watchdog_firing_is_the_baseline_for_a_successful_later_goto` starts its later GoTo, **Then** the committed test asserts that the telescope's true direction is within 6° of M42 before it, and asserts that the session's pointing is within 6° of M42 before the watchdog fires, and keeps its assertions that the GoTo ends `done` within 60″ by the true eyepiece, that the watchdog count is unchanged and that a GoTo command was recorded.
5. **Given** the story's diff, **When** it is read, **Then** `tracking::routes::track_arms_during_goto_then_starts_within_two_seconds`, `tracking::routes::five_degree_error_starts_goto_and_off_leaves_telescope_where_it_is`, `tracking::routes::armed_stop_cancels_handoff_and_failed_goto_keeps_its_reason` and `tracking::routes::readout_uses_live_errors_last_sixty_seconds_and_fr001_fr002` are unchanged and still reach tracking through the GoTo handoff.

---

### User Story 3 - The GoTo simulator tests reach their start cheaply (Priority: P1)

As the maintainer, I have the five-target GoTo test start from a two-sync model without
the 129° slew that builds it, and the cable-wind test sample its long move less often.

**Why this priority**: The two tests take 112.5 s in run 43683194483.

**Independent Test**:
`cargo nextest run --workspace --all-targets --all-features --locked` runs
`crates/finder/tests/motion/goto/sim.rs`.

**Acceptance Scenarios**:

1. **Given** the `motion` binary, **When** `goto::sim::five_catalog_targets_converge_after_two_syncs_and_report_progress` builds its rig, **Then** the committed test starts from a state file holding the two syncs of the old setup, at axis counts (4500, 4500) and (580000, 4500), whose directions are computed from a simulated mount with the rig's own configuration, makes only the first 1° move of the old setup, and asserts before the first GoTo that the azimuth axis reads 4500 counts and that the model has a base tilt within 1′ of the simulator's 1° toward 130°, three syncs and one epoch.
2. **Given** the same test, **When** its five GoTos run, **Then** it asserts for each target in the same order on the same session everything it asserted before: the ordered progress states, at most five corrections, a correction on M42, and an error under 60″ reported and by the true eyepiece.
3. **Given** the `motion` binary, **When** `goto::sim::continuously_observed_cable_wind_survives_real_simulator_power_cycle` makes its 350° move, **Then** the committed test sets a 2 s frame interval before the move and 500 ms after it, asserts that the frame id of the session's pointing advanced by at least 20 and at most 60 during the move, and keeps its assertions after the power cycle.
4. **Given** the story's diff, **When** it is read, **Then** `crates/finder/tests/support/goto.rs` gains `Rig::with_session`, `Rig::new` keeps its signature and its body is one call of it with `Session::default()`, and no test outside `crates/finder/tests/motion/goto/sim.rs` changes.

---

### User Story 4 - The whole night runs at two frames a second (Priority: P1)

As the maintainer, I have the whole-night scenario run its node at the 500 ms frame
interval every other rig uses.

**Why this priority**: It is the longest test, 181.548 s in run 43683194483.

**Independent Test**:
`cargo nextest run --workspace --all-targets --all-features --locked` runs
`crates/finder/tests/motion/night.rs`.

**Acceptance Scenarios**:

1. **Given** the `motion` binary, **When** `night::a_whole_night_syncs_five_gotos_hand_push_stop_and_dropped_link` takes the simulator from the console's state, **Then** the committed test sets a 500 ms frame interval on it and, after the first sync, asserts over at least four solved frames that the session's pointing time advances by 0.5 s per frame id, within 0.05 s.
2. **Given** the story's diff, **When** it is read, **Then** `Simulator::set_frame_interval` is one call to the `NodeControls` the simulator shares with its node, and the night test's only changes are that call, right after it takes the simulator, and the cadence assertion of US4-S1.

---

### User Story 5 - Track's restart test tracks in paused time (Priority: P1)

As the maintainer, I have the test that Track is off after a restart do its tracking in
paused time, and start the built binary only to check the restart.

**Why this priority**: The test waits on real time, 27.032 s in run 43683194483.

**Independent Test**:
`cargo nextest run --workspace --all-targets --all-features --locked` runs
`crates/finder/tests/cli/track.rs`.

**Acceptance Scenarios**:

1. **Given** the `cli` binary, **When** `track::tracking_that_moved_the_mount_is_off_and_quiet_after_restart` runs, **Then** the committed test tracks M42 in paused time in process on a `Session::with_state_file` in its folder, from a centred start, and asserts that Track's reply reads `tracking` with no `goto` and that the mount's true direction moved.
2. **Given** that session dropped, **When** the test starts the built binary on the folder, **Then** it keeps its assertions: the state file holds no `track`, the restarted console reports no `track`, nothing moves for 2 s and the eyepiece is unchanged.
3. **Given** the story's diff, **When** it is read, **Then** the test starts the built `finder` once, sends it no `POST /api/track`, and says that Playwright's `tracking.spec.ts` owns Track on the real binary.

---

### User Story 6 - The built binary is started only for what `main.rs` does (Priority: P2)

As the maintainer, I have restart tests that check library code do it in process, and the
backlash measured once for both its restart and its reset.

**Why this priority**: It removes two tests (4.99 s in run 43683194483) and a measurement
that ran twice, without giving up the real binary's restart check.

**Independent Test**:
`cargo nextest run --workspace --all-targets --all-features --locked` runs
`crates/finder/tests/console/session/durable.rs`, `crates/finder/tests/cli/aim.rs` and
`crates/finder/tests/cli/backlash.rs`.

**Acceptance Scenarios**:

1. **Given** the `console` binary, **When** `session::durable::durable_fields_survive_reopening_the_state_file_in_owner_only_mode` runs, **Then** the committed test writes the site, minimum altitude, target, node address and token, camera settings with exposures of 200 ms and 2000 ms, imaging camera and aim through `Session` on a `Session::with_state_file`, drops it, reopens the file and asserts each value equal (the token is compared in the stored file, because the view only says a token is set), and asserts that the file's mode is 0600 after a write over a 0644 file, that a stale `.tmp` beside it is gone after the next write, and that the token never appears in the session's view.
2. **Given** the `cli` binary, **When** `aim::calibrated_aim_survives_binary_restart_exactly_and_reset_removes_it` runs, **Then** the committed test collects every line the built binary writes to stderr and asserts that none contains the node token.
3. **Given** the `cli` binary, **When** `backlash::measured_altitude_backlash_survives_two_next_day_binary_restarts_exactly` has restarted the binary twice and read the measured backlash from it, **Then** the committed test posts `{"what":"fixed"}` to `/api/model/reset` on that running binary and asserts that the reply and the next session read hold no `backlash`, and that the state file holds none after the binary stops.
4. **Given** the story's diff, **When** it is read, **Then** `crates/finder/tests/cli/session_node.rs` and its `mod` line are removed, `tracking::backlash::first_transit_measures_then_fixed_reset_removes_the_observed_value` is removed, and `crates/finder/tests/console/session/mod.rs` gains `mod durable;`.

---

### User Story 7 - Preview retention and the unsolved interval need fewer frames (Priority: P2)

As the maintainer, I have the two console tests that wait on the node's clock run their
node at a frame interval that fits what they assert.

**Why this priority**: 17.9 s in run 43683194483, nothing lost.

**Independent Test**:
`cargo nextest run --workspace --all-targets --all-features --locked` runs
`crates/finder/tests/console/aim.rs` and `crates/finder/tests/console/live/loop_.rs`.

**Acceptance Scenarios**:

1. **Given** the `console` binary, **When** `aim::aim_writes_enforce_origin_json_and_strict_shape_and_preview_retention_is_eight` waits for nine previews, **Then** the committed test runs its node at a 3 s frame interval, asserts that consecutive preview frame ids differ by exactly 2, and keeps every retention assertion, including the 409 for the evicted preview.
2. **Given** the `console` binary, **When** `live::loop_::rich_frames_failing_for_thirty_seconds_report_the_whole_unsolved_interval` runs, **Then** the committed test runs its node at a 5 s frame interval, counts the frames its wait loop receives after the first and asserts at most 7, and keeps its assertions that the unsolved interval is at least 30 s and the field is null.
3. **Given** the story's diff, **When** it is read, **Then** `state` in `crates/finder/tests/support/live.rs` keeps its signature and its 500 ms interval, so its other callers are unchanged.

---

### User Story 8 - No test depends on how fast the machine is (Priority: P2)

As the maintainer, I have no test fail, hang or flake because the machine running it is
busy.

**Why this priority**: Nothing fails today; these are what break first under load.

**Independent Test**:
`cargo nextest run --workspace --all-targets --all-features --locked` runs
`crates/finder/tests/motion/tracking/night.rs`, `crates/finder/tests/cli/simulate.rs`,
`crates/scope-node/tests/cli.rs` and the `rpicam::metadata` tests of `scope-node`.

**Acceptance Scenarios**:

1. **Given** the `motion` binary, **When** `tracking::night::an_hour_covers_transit_too_low_and_keyhole` runs, **Then** the committed test makes every assertion it made before except its own wall-clock bound, and `crates/finder/tests/motion/tracking/night.rs` no longer reads `std::time::Instant`.
2. **Given** the story's diff, **When** it is read, **Then** each inline rig solver, in `crates/finder/tests/support/goto.rs`, `crates/finder/tests/support/mcp.rs`, `crates/finder/tests/support/stop.rs`, `crates/finder/tests/motion/night.rs` and `crates/finder/tests/motion/mount_identity.rs`, passes `timeout_ms: u64::MAX` to the solver.
3. **Given** the `cli` binary, **When** `simulate::simulated_epoch_frames_and_real_rate_never_connect_to_the_stored_node` checks that 5 s of real time is 5 s of simulated time within 2 s, **Then** the committed test reads the session time it compares against immediately before its 5 s window.
4. **Given** the story's diff, **When** it is read, **Then** `run` in `crates/scope-node/tests/cli.rs` spawns the child, kills it if it has not exited 10 s after it started, and fails the test naming the child's command-line arguments.
5. **Given** the `scope-node` library tests, **When** the three `rpicam::metadata` tests run, **Then** each runs on a paused clock, and the two that wrap a read in a 100 ms timeout keep it, with every assertion.

---

### User Story 9 - Scratch folders are made and removed in one place (Priority: P3)

As the maintainer, I have every scratch folder of finder's integration tests come from one
helper, under the target directory cargo names, and go away when the test ends, passed or
failed.

**Why this priority**: A hazard, not a cost; it waits for the stories that edit the same
files.

**Independent Test**:
`cargo nextest run --workspace --all-targets --all-features --locked` runs every test that
uses a scratch folder, and `crates/finder/tests/layout/main.rs`.

**Acceptance Scenarios**:

1. **Given** `crates/finder/tests/support/mod.rs`, **When** a test asks it for a scratch folder, **Then** the helper creates one under `CARGO_TARGET_TMPDIR` named by the caller's name, the process id and a counter, and removes it when dropped, without panicking if removal fails.
2. **Given** the story's diff, **When** it is read, **Then** no file under `crates/finder/tests` builds a folder from `CARGO_MANIFEST_DIR` and `../../target`, and every site plan.md § Interfaces "US9" lists uses the helper.
3. **Given** the `console` binary, **When** `targets::search_works_from_an_empty_working_directory` and `session::edits::library_without_a_state_file_leaves_home_and_xdg_empty` have run their child, **Then** each committed test still asserts that its folder is empty.
4. **Given** the story's diff, **When** it is read, **Then** `crates/scope-node/tests/cli.rs`, `crates/scope-node/tests/mount_config.rs`, `crates/scope-node/tests/rpicam.rs` and `crates/scope-node/tests/mount_link.rs` write their scratch files under `CARGO_TARGET_TMPDIR` and remove them on drop.
5. **Given** the `layout` binary, **When** its tests read `crates/finder/tests/support`, **Then** they pass unchanged: the folder holds the same six files.

---

### User Story 10 - Duplicates with a stricter twin, and tests that cannot fail, go (Priority: P3)

As the maintainer, I have each behaviour asserted once, by its strictest test, and no test
whose name claims coverage it does not have.

**Why this priority**: Most of these cost nothing to run; the owner asked whether tests
are duplicative. Three of them (`model::sim`, `goto::routes`, `mcp::motion`) do save time.

**Independent Test**:
`cargo nextest run --workspace --all-targets --all-features --locked` runs every file this
story edits.

**Acceptance Scenarios**:

1. **Given** the story's diff, **When** it is read, **Then** exactly these test functions are removed, each with plan.md § Interfaces "US10" naming its stricter twin: `pixel_center_convention_top_left`, `inline_image_bytes_used_when_shmem_absent` and `coordinate_swap_inbound_and_outbound` in `crates/grpc-service/tests/service_acceptance.rs`; `explicit_detection_parameters_honored` in `crates/plate-solver/tests/solve_acceptance.rs`; `breadth_first_yields_brightest_first` in `crates/database-generation/src/patterns.rs`; `backlash_changes_truth_by_point_three_at_the_same_axis_reading` in `crates/simulator/tests/mount.rs`; `two_axes_expose_capabilities_positions_signed_rates_and_stop` with its fake in `crates/scope-node/src/mount/mod.rs`; the two inline tests `fresh_session_has_no_site_target_or_pointing_and_fifteen_degree_minimum` and `fixed_clock_reads_the_same_unix_time_twice` in `crates/finder/src/session.rs`; `second_sync_fits_the_simulator_base_without_opening_an_epoch` in `crates/finder/tests/motion/model/sim.rs`; and `m8_below_horizon_is_refused_without_motion`, `approach_to_fifteen_point_two_degrees_is_refused_without_motion`, `sun_five_degrees_up_is_refused_without_motion` and `absent_mount_and_missing_sync_each_name_the_reason` in `crates/finder/tests/motion/goto/routes.rs`.
2. **Given** `crates/plate-solver/tests/solve_acceptance.rs`, **When** its tests run, **Then** the committed test that was `cancellation_honored` asserts only that a context with its cancel flag set reports it, under the name `solve_context_reports_a_set_cancel_flag`, and the committed test that was `brightest_first_requirement` keeps its assertions under the name `exhausted_search_of_an_empty_pattern_catalog_is_no_match`.
3. **Given** `crates/database-generation/src/patterns.rs` and `crates/scope-node/tests/skywatcher.rs`, **When** their tests run, **Then** `matches_cedar_order_and_visits_each_combination_once` runs in the `tests` module unchanged, and the committed test that was `both_stops_precede_emergency_fallback` asserts, under the name `stop_all_stops_both_axes`, that `stop_all` sends both axis stops and leaves both axes at rest.
4. **Given** the `motion` binary, **When** `model::sim::durable_night_survives_local_midnight_then_expires_at_solar_noon` has made its second sync, **Then** the committed test asserts the fitted base tilt within 1′ of the simulator's, as the removed test did; and `goto::refusals::no_mount_omits_refusal_and_second_sync_omits_it_while_running` asserts that the refusal without a mount names `no mount`.
5. **Given** the `motion` binary, **When** `model::routes::resets_clear_only_the_requested_terms_and_persist_across_restart` posts a foreign-origin reset, **Then** the committed test does so for `fixed` only and keeps its 403 and unchanged-model assertions.
6. **Given** the `mcp` binary, **When** the nine cases of `motion::second_client_stop_and_sdk_rejections_cannot_block_stop` and `motion::security_rejections_cannot_block_stop` stop a running GoTo, **Then** each committed case asserts StopAll within 0.1 s, the halt reason, motion cleared and both axes at rest within 2 s, and only the second-client case also watches 5 s of unchanged axes and checks that no command but StopAll was sent.

---

### User Story 11 - Optional: the whole night is shortened (Priority: P3)

As the owner, I can keep the whole-night scenario's claim, that the stories work together
in one session, with fewer GoTos to done. Kept or deleted before the flip.

**Why this priority**: Optional; it amends spec 014 US9.

**Independent Test**:
`cargo nextest run --workspace --all-targets --all-features --locked` runs
`crates/finder/tests/motion/night.rs`.

**Acceptance Scenarios**:

1. **Given** the `motion` binary, **When** `night::a_whole_night_syncs_gotos_hand_push_stop_and_dropped_link` runs, **Then** the committed test makes the first sync and the one-tap second sync, then one GoTo to M81 that it asserts ends within 60″ by the true eyepiece, then the hand push and its GoTo to M42, the Stop mid-slew and the dropped link mid-slew, with every assertion each of those steps made before.
2. **Given** the story's diff, **When** it is read, **Then** the completed GoTos to M42 as the first target, M31, M45 and M1 are gone, the slews the Stop and the dropped link interrupt stay, and nothing else in the test changes except its name.

---

### User Story 12 - Optional: the MCP hold is shortened (Priority: P3)

As the owner, I can keep the MCP tracking scenario's own claims over 75 s instead of ten
minutes. Kept or deleted before the flip.

**Why this priority**: Optional; it amends spec 016 US6-S1.

**Independent Test**:
`cargo nextest run --workspace --all-targets --all-features --locked` runs
`crates/finder/tests/mcp/tracking.rs`.

**Acceptance Scenarios**:

1. **Given** the `mcp` binary, **When** `tracking::track_first_goto_keeps_mcp_source_and_holds_m42_for_seventy_five_seconds` runs, **Then** the committed test samples every second for 75 s of tracking and asserts on every sample the MCP attribution and M42 within 1′ by the true eyepiece, then more than 30 tracker samples and the readout equal to the page route's.
2. **Given** the story's diff, **When** it is read, **Then** the only changes are the hold's span, the test's name, the comment that states the span, and `ten-minute` in its closing wall-time message.

---

### User Story 13 - Optional: the sixty-second window is proven below the route (Priority: P3)

As the owner, I can have the tracker test own both edges of the sixty-second window, and
the route test track only as long as its other claims need. Kept or deleted before the
flip.

**Why this priority**: Optional; it amends how spec 015 US7-S7 is proven.

**Independent Test**:
`cargo nextest run --workspace --all-targets --all-features --locked` runs
`crates/finder/tests/motion/tracking/tracker.rs` and
`crates/finder/tests/motion/tracking/routes.rs`.

**Acceptance Scenarios**:

1. **Given** the `motion` binary, **When** `tracking::tracker::pointing_samples_age_on_console_time_and_are_ignored_after_ending` has fed one sample, **Then** the committed test asserts that the sample is still held 59.9 s later and gone 60.1 s after it was fed.
2. **Given** the `motion` binary, **When** `tracking::routes::readout_uses_live_errors_last_sixty_seconds_and_fr001_fr002` has tracked until the tracker holds more than 30 live samples, **Then** the committed test keeps its RMS, field-rotation and longest-exposure assertions, then stops the camera, lets the live samples age out, injects one sample through the tracker and asserts that the readout's RMS equals that sample's error 59.9 s later and is zero 60.1 s after it was injected.

---

### Edge Cases

- **A frame already on its way when the camera stops.** The node checks its controls
  before each capture, so one capture already started still arrives. The bound is
  therefore "at most one more frame", not "none".
- **A centred start that waits too long, or not at all.** A mount at rest holds a fixed
  altitude and azimuth, so M42 leaves the 1′ that counts as on target after about 4 s.
  But the node starts capturing when the rig is built, so the first frame, and the first
  sync, come from the start direction, 28.7° from M42; Track posted at once would start
  a GoTo. A centred test ticks after its sync, at most 10 times, until the session's
  pointing is within 1′ of M42, then posts Track (plan.md § Interfaces "A centred
  start").
- **A frame interval changed mid-run.** The node reads it before each capture, so the
  first frame after a change can still follow the old interval. The cadence and count
  assertions allow for one such frame.
- **A test under `cargo test` instead of nextest.** Every change here keeps a test
  passing when its binary runs all its tests in one process: scratch folders carry a
  counter as well as the process id.
- **Specs 024 and 025 land first.** 024 added a module to `motion/main.rs` and edited
  `crates/scope-node/src/mount/mod.rs` and `crates/finder/src/session.rs`; 025 added
  four modules, `cli/recording.rs`, `console/recording.rs`, `console/recording_raw.rs`
  and `motion/recording_raw.rs`, declared in their binaries' `main.rs`, and each builds
  a scratch folder that US9 moves. Both are in `depends_on_landed`. Every citation in a
  file 024 changed was re-read at f0ea2d5, and every citation in a file 025 changed was
  re-read at 9d8cc4e, where 025's US1 and US2 landed; 025's US3 (820c558) writes only web
  files this spec does not cite (plan.md § Prerequisites).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The six direct-move tests of `crates/finder/tests/mcp/axis_moves.rs` named
  in US1-S1 and US1-S2 SHALL stop the camera after their first sync and SHALL assert, at
  their end, that the newest frame id is at most one more than when the camera stopped.
- **FR-002**: `axis_moves::track_through_the_page_blocks_all_three_mcp_moves_and_keeps_tracking`
  SHALL centre the eyepiece on M42 before its first sync and SHALL assert that Track
  starts without a GoTo.
- **FR-003**: The simulator crate's shared test rig SHALL build its node on the synthetic
  camera with no pattern database, SHALL assert that the node describes its camera as
  `simulated`, and SHALL keep a second constructor on the catalog camera for
  `frame_positions.rs`.
- **FR-004**: The four tests of `crates/finder/tests/motion/tracking/routes.rs` named in
  US2-S1 SHALL reach tracking from a centred start and SHALL assert Track's reply reads
  `tracking` with no `goto`.
- **FR-005**: `tracking::min_alt::raised_minimum_altitude_ends_running_tracking_as_too_low`
  SHALL do the same.
- **FR-006**: The three Stop tests named in US2-S3 SHALL centre on M42 before the first
  sync and SHALL assert that the command log holds no GoTo when tracking starts.
- **FR-007**: `stop::watchdog::past_watchdog_firing_is_the_baseline_for_a_successful_later_goto`
  SHALL start 5° below M42 in altitude, SHALL assert that the session's pointing is
  within 6° of M42 before the watchdog fires, and SHALL assert the true direction is
  within 6° of M42 before its later GoTo.
- **FR-008**: The goto rig SHALL gain `Rig::with_session`, taking a `Session`; `Rig::new`
  SHALL call it with `Session::default()` and keep its signature.
- **FR-009**: The five-target GoTo test SHALL start from a state file holding the near
  and far syncs of plan.md § Interfaces "US3", SHALL make only the first 1° move, and SHALL assert
  the start of US3-S1 before its first GoTo; its five GoTos and their assertions SHALL
  not change.
- **FR-010**: The cable-wind test SHALL set a 2 s frame interval for its 350° move and
  500 ms after it, and SHALL assert the bound of US3-S3.
- **FR-011**: `Simulator` SHALL gain `set_frame_interval`, which forwards to the
  `NodeControls` it shares with its node.
- **FR-012**: The whole-night test SHALL set a 500 ms frame interval on its simulator and
  SHALL assert the cadence of US4-S1.
- **FR-013**: Track's restart test SHALL track in paused time in process on a
  `Session::with_state_file` from a centred start, then start the built binary once on
  that folder, keeping its restart assertions.
- **FR-014**: A new module, `crates/finder/tests/console/session/durable.rs`, SHALL hold
  the in-process test of US6-S1, and `crates/finder/tests/console/session/mod.rs` SHALL
  declare it.
- **FR-015**: `crates/finder/tests/cli/session_node.rs` and its `mod` line in
  `crates/finder/tests/cli/main.rs` SHALL be removed.
- **FR-016**: `crates/finder/tests/cli/aim.rs` SHALL collect the built binary's stderr and
  assert it never contains the node token.
- **FR-017**: The backlash restart test SHALL post the fixed reset to the restarted binary
  and assert US6-S3, and `first_transit_measures_then_fixed_reset_removes_the_observed_value`
  SHALL be removed.
- **FR-018**: `crates/finder/tests/support/live.rs` SHALL gain `state_with_interval`, and
  `state` SHALL call it with 500 ms.
- **FR-019**: The aim test SHALL run its node at 3 s and assert US7-S1.
- **FR-020**: The thirty-second unsolved test SHALL run its node at 5 s and assert
  US7-S2.
- **FR-021**: The hour of tracking SHALL drop its wall-clock read and assertion, and its
  name SHALL drop `_in_under_thirty_wall_seconds`.
- **FR-022**: Each of the five inline rig solvers named in US8-S2, including the one in
  `crates/finder/tests/motion/mount_identity.rs`, SHALL pass `timeout_ms: u64::MAX`.
- **FR-023**: `cli/simulate.rs` SHALL take the time it compares against immediately
  before its 5 s window.
- **FR-024**: The child helper of `crates/scope-node/tests/cli.rs` SHALL kill a child
  still running after 10 s and fail the test naming the child's arguments.
- **FR-025**: The three `rpicam::metadata` tests SHALL run on a paused clock.
- **FR-026**: `crates/finder/tests/support/mod.rs` SHALL hold one scratch-folder helper as
  US9-S1 describes, and SHALL hold no test.
- **FR-027**: Every scratch folder plan.md § Interfaces "US9" lists under
  `crates/finder/tests` SHALL come from that helper; where a test's removal of its folder
  was a check, the check SHALL stay as an assertion.
- **FR-028**: The four scope-node test files of US9-S4 SHALL write under
  `CARGO_TARGET_TMPDIR` and remove their files on drop.
- **FR-029**: The three `service_acceptance.rs` tests of US10-S1 SHALL be removed.
- **FR-030**: In `solve_acceptance.rs`, the test of US10-S1 SHALL be removed, the two of
  US10-S2 reduced or renamed as it says, and
  `detection_parameters_do_not_perturb_solve_math` SHALL stay unchanged.
- **FR-031**: In `patterns.rs`, `breadth_first_yields_brightest_first` SHALL be removed
  and `bfc_order_tests` folded into `tests`.
- **FR-032**: The simulator and scope-node duplicates of US10-S1 and US10-S3 SHALL be
  removed or reduced as they say.
- **FR-033**: The two inline session tests of US10-S1 SHALL be removed.
- **FR-034**: The `motion` duplicates of US10-S1, US10-S4 and US10-S5 SHALL be removed,
  with the assertions those scenarios name moved to the tests that stay.
- **FR-035**: The MCP Stop cases SHALL assert as US10-S6 says.
- **FR-036**: Optional, US11: The whole night SHALL run the steps of US11-S1.
- **FR-037**: Optional, US12: The MCP hold SHALL span 75 s.
- **FR-038**: Optional, US13: The window SHALL be proven as US13-S1 and US13-S2 say.

### Key Entities

- **Newest frame id**: the id of the newest frame the console's scope link holds. The node
  counts every capture, solved or not.
- **Centred start**: the eyepiece pointed at M42 by hand before the first sync, so that
  Track begins at once with no GoTo, and Track posted once a frame exposed after the push
  has solved.
- **Inline rig solver**: a test rig's solver that solves each frame on the test's paused
  runtime.
- **Scratch folder**: a folder a test writes state into and must leave nothing behind.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario above is asserted by a committed test or can be
  read from the story's diff. The `lint` and `test` gates pass.
- **SC-002**: No oracle file, golden, `Cargo.toml`, `Cargo.lock`, `scripts/ci/` file,
  workflow, `CODEOWNERS` entry, `ergane.yaml` or `.config/nextest.toml` is edited, and no
  product source changes outside a test module except `crates/simulator/src/controls.rs`.
- **SC-003**: With every story kept, 16 test functions are removed and one is added, and
  no assertion is removed except those owner approvals 3, 6, 7, 10, 12 and 15 to 17 name.
- **SC-004**: The owner reads the per-test durations of the first GitHub `test` job after
  the last story lands and compares them with plan.md § Measurements. This is not a gate.

## Assumptions

- **Earlier features are in place.** 012 through 020, 024, 025 and 027 have landed:
  025's US1 and US2 at 9d8cc4e, and its US3, the page's recording controls, writes only
  web files. Every citation here holds at 9d8cc4e: those in files 025 changed were
  re-read there, those in files 024 changed were re-read at f0ea2d5 and are unchanged
  since, and the rest are in files unchanged since 06a7d4e, where the trio was drafted.
- **Out of scope:**
  - the page's tests, Playwright and its servers (spec 029);
  - the oracle files and the audit's findings core-S1 to core-S3, which would edit them;
  - CI cache fixes, a merge-queue-only slow tier, zero-test targets in `Cargo.toml`, a
    lower per-test time limit, and any change to the test profile;
  - the organization of the other crates' tests and of the page's test files;
  - the two scratch folders made by unit tests in `crates/finder/src` (`session.rs` and
    `samples.rs`), where cargo does not set `CARGO_TARGET_TMPDIR`;
  - the other 5 s solver deadlines, which tests still reach: the production live
    solver's, through `AppState::with_live` or, in 025's `console/recording.rs`,
    `Live::start` with `DbSolver`; uploaded-frame solves; and direct calls of
    `solve::solve_frame_with_frame`;
  - the 15 s wait for the first live pointing in `cli/simulate.rs`'s `Server::pointing`
    (`crates/finder/tests/cli/simulate.rs:97`), another wall-clock bound;
  - audit findings outside the handoff's list for this spec (plan.md § Decisions);
  - the text of landed specs, including 014 US9, 015 US7-S7 and US9, and 016 US6-S1:
    they keep their text, and an optional story's approval records its amendment.
- **The factory's gates and CI:** the gates are `lint` and `test`, copied in plan.md
  § Verification. CI's other jobs run only in the merge queue.

## Work Graph

```yaml
US1:
  depends_on: []
  implements: [FR-001, FR-002, FR-003]
US2:
  depends_on: []
  implements: [FR-004, FR-005, FR-006, FR-007]
US3:
  depends_on: []
  implements: [FR-008, FR-009, FR-010]
US4:
  depends_on: []
  implements: [FR-011, FR-012]
US5:
  depends_on: []
  depends_on_merged: [US3]
  implements: [FR-013]
US6:
  depends_on: []
  implements: [FR-014, FR-015, FR-016, FR-017]
US7:
  depends_on: []
  implements: [FR-018, FR-019, FR-020]
US8:
  depends_on: []
  depends_on_merged: [US3, US4]
  implements: [FR-021, FR-022, FR-023, FR-024, FR-025]
US9:
  depends_on: []
  depends_on_merged: [US1, US3, US5, US6, US8]
  implements: [FR-026, FR-027, FR-028]
US10:
  depends_on: []
  depends_on_merged: [US9]
  implements: [FR-029, FR-030, FR-031, FR-032, FR-033, FR-034, FR-035]
US11:
  depends_on: []
  depends_on_merged: [US4, US8]
  implements: [FR-036]
US12:
  depends_on: []
  implements: [FR-037]
US13:
  depends_on: []
  depends_on_merged: [US2]
  implements: [FR-038]
```

Every edge is a merge edge, because the later story edits a file the earlier one edits or
uses what it adds:

| Story | Waits for | Why |
|---|---|---|
| US5 | US3 | `Rig::with_session` in `support/goto.rs` |
| US8 | US3, US4 | `support/goto.rs`; `motion/night.rs` |
| US9 | US1, US3, US5, US6, US8 | `mcp/axis_moves.rs`; `motion/goto/sim.rs`; `cli/track.rs`; `console/session/durable.rs`, `cli/aim.rs`, `cli/backlash.rs`, `motion/tracking/backlash.rs`; `motion/tracking/night.rs`, `cli/simulate.rs`, scope-node's `tests/cli.rs` |
| US10 | US9 | `motion/model/sim.rs`, `motion/model/routes.rs`, `mcp/motion.rs` |
| US11 | US4, US8 | `motion/night.rs` |
| US13 | US2 | `motion/tracking/routes.rs` |

US1, US2, US3, US4, US6 and US7 touch disjoint files and may run side by side. No story
depends on US11, US12 or US13, so deleting any of them leaves a valid graph.
