---
description: "Task list for 028-finder-tests-do-less: Finder's Tests Do Less Work"
---

# Tasks: Finder's Tests Do Less Work

**Prerequisites** (plan.md § Prerequisites):
- `027-finder-tests-by-system` and `024-mount-link-recovery` have landed on `main` (024
  at f0ea2d5, where this trio was re-anchored), and so has 025;
- the owner has kept or deleted the optional stories US11, US12 and US13;
- the owner approvals in spec.md are given;
- no operator-staged file is needed.

**Rules for every story:**
- Write only the files plan.md § Refinement Record lists as your story's `writes`. A
  story that needs another file escalates. A file a task names as unchanged stays
  unchanged.
- Make only the edits plan.md § Interfaces names for your story. Delete, rename or
  reduce no test it does not name. Widen no tolerance and drop no assertion it does not
  name. Where a test moves work out of its body, the assertion it names takes the work's
  place.
- Each new assertion is written first and seen to fail, against the old code or with the
  saving's line removed, before the change that makes it pass.
- Never edit a `Cargo.toml`, `Cargo.lock`, a golden, a fixture's data, an oracle file
  (plan.md § Constitution Check, I), `scripts/`, `.github/`, `.config/`, `deny.toml`,
  `CODEOWNERS`, `ergane.yaml` or anything under `specs/`. If the build needs one of them
  changed, stop and escalate with the exact output.
- Run no tool the gates don't run, except `cargo fmt --all` to apply the formatting that
  `cargo fmt --all --check` checks, and the commands of your story's development checks.
  Do not set `CARGO_TARGET_DIR`.

## Phase 1: User Story 1 — The camera stops where no assertion reads a frame (Priority: P1)

### Tests first

- [ ] T001 [US1] In `crates/finder/tests/mcp/axis_moves.rs`, add `newest_frame` as
  plan.md § Interfaces "Shared helpers" gives it. In each of the six direct-move tests of
  FR-001, read `newest_frame` right after the first sync and assert at the end that the
  newest frame id is at most one more. The bound fails while frames still run (US1-S1,
  US1-S2).

### Implementation

- [ ] T002 [US1] In `crates/finder/tests/mcp/axis_moves.rs`, add `camera_off` and make
  each of the six reads of T001 a call to it, placed as plan.md § Interfaces "US1" item 1
  says, including the sunrise test's own rig, where the binding is `last_frame` (FR-001).
  Change no other line of those tests.
- [ ] T003 [US1] In `crates/finder/tests/mcp/axis_moves.rs`, give
  `track_through_the_page_blocks_all_three_mcp_moves_and_keeps_tracking` the centred
  start of plan.md § Interfaces "Shared helpers", with its wait for the first frame solved
  on M42, and assert that the session reads `tracking` with no `goto` after the page's
  Track request, before the wait loop (FR-002, US1-S3).
- [ ] T004 [US1] In `crates/simulator/tests/support/mod.rs`, make `Rig::new` build the
  node on the synthetic camera with no `Simulator` and no pattern database, and assert
  the `Describe` model `simulated` before it returns; add `Rig::rendered` with what
  `Rig::new` builds today, both through the private `on` of plan.md § Interfaces "US1"
  item 2; replace the field `sim` with `clock` (FR-003, US1-S4).
  `crates/simulator/tests/stop_and_watchdog.rs` and
  `crates/simulator/tests/watchdog_report.rs` stay unchanged.
- [ ] T005 [US1] In `crates/simulator/tests/frame_positions.rs`, call `Rig::rendered`
  and read `rig.clock`, and change nothing else (US1-S5).
- [ ] T006 [US1] Run `cargo fmt --all`, then the `lint` and `test` gates. Commit the
  green story.

## Phase 2: User Story 2 — Track tests start where their claim starts (Priority: P1)

### Tests first

- [ ] T007 [US2] In `crates/finder/tests/motion/tracking/routes.rs`, add `centred()` as
  plan.md § Interfaces "US2" item 1 gives it, with the centred-start wait of "Shared
  helpers" and its assertions that Track's reply reads `tracking`, names M42 with
  `wanted` true and holds no `goto`, and check that the reply no longer reads `tracking`
  with no `goto` when the `point_eyepiece_at` line is removed (US2-S1).

### Implementation

- [ ] T008 [US2] In `crates/finder/tests/motion/tracking/routes.rs`, call `centred()`
  instead of `accepted()` in the four tests FR-004 names. Leave `accepted()`,
  `tracking()`, the three handoff tests and the readout test unchanged (US2-S5).
- [ ] T009 [US2] In `crates/finder/tests/motion/tracking/min_alt.rs`, centre on M42
  before the sync, wait as the centred-start recipe says, and assert the Track reply, as
  plan.md § Interfaces "US2" item 2 says (FR-005, US2-S2).
- [ ] T010 [US2] In `crates/finder/tests/motion/stop/motion_ends.rs` (its `tracking()`
  helper only; `ready()` is unchanged) and in
  `crates/finder/tests/motion/stop/retry.rs`, centre on M42 before the sync, wait as the
  centred-start recipe says and, once tracking runs, assert that the command log holds no
  `MountCommand::GoTo` (FR-006, US2-S3).
- [ ] T011 [US2] In `crates/finder/tests/motion/stop/watchdog.rs`, start
  `past_watchdog_firing_is_the_baseline_for_a_successful_later_goto` 5° below M42, tick
  after the sync until the session's pointing is within 6° of M42 and assert it before
  the watchdog, and assert the true direction within 6° before its GoTo, as plan.md
  § Interfaces "US2" item 5 says, keeping every later assertion (FR-007, US2-S4).
- [ ] T012 [US2] Run `cargo fmt --all`, then the `lint` and `test` gates. Commit the
  green story.

## Phase 3: User Story 3 — The GoTo simulator tests reach their start cheaply (Priority: P1)

### Tests first

- [ ] T013 [US3] In `crates/finder/tests/support/goto.rs`, add `Rig::with_session` and
  make `Rig::new` call it with `Session::default()`, keeping its signature (FR-008,
  US3-S4).
- [ ] T014 [US3] In `crates/finder/tests/support/goto.rs`, add `mount_config`,
  `far_sync` (writing the near and far syncs) and `second_from_file` as plan.md
  § Interfaces "US3" item 2 gives them, with the assertions of US3-S1: 4500 azimuth
  counts, a base tilt within 1′ of the simulator's 1° toward 130°, three syncs (the two
  stored and the live one) and one epoch.

### Implementation

- [ ] T015 [US3] In `crates/finder/tests/motion/goto/sim.rs`, build the five-target
  test's rig from the stored near and far syncs and call `second_from_file`, as plan.md
  § Interfaces "US3" item 3 says. Its five GoTos and every assertion after the setup stay
  unchanged (FR-009, US3-S1, US3-S2).
- [ ] T016 [US3] In `crates/finder/tests/motion/goto/sim.rs`, first assert the frame-id
  advance of US3-S3 during the 350° move and see it fail at the default interval; then
  set 2 s before the move and 500 ms after it (FR-010).
- [ ] T017 [US3] Run `cargo fmt --all`, then the `lint` and `test` gates. Commit the
  green story.

## Phase 4: User Story 4 — The whole night runs at two frames a second (Priority: P1)

### Tests first

- [ ] T018 [US4] In `crates/finder/tests/motion/night.rs`, after the first sync, add the
  cadence assertion of plan.md § Interfaces "US4" item 2, and see it fail at the 200 ms
  default (US4-S1).

### Implementation

- [ ] T019 [US4] In `crates/simulator/src/controls.rs`, add
  `Simulator::set_frame_interval`, one call to the shared `NodeControls` (FR-011,
  US4-S2).
- [ ] T020 [US4] In `crates/finder/tests/motion/night.rs`, call it with 500 ms right
  after the test takes the simulator from the console's state. The night test's only
  changes are that call and the cadence assertion of T018 (FR-012, US4-S2).
- [ ] T021 [US4] Run `cargo fmt --all`, then the `lint` and `test` gates. Commit the
  green story.

## Phase 5: User Story 5 — Track's restart test tracks in paused time (Priority: P1)

### Tests first

- [ ] T022 [US5] In `crates/finder/tests/cli/track.rs`, make
  `tracking_that_moved_the_mount_is_off_and_quiet_after_restart` a paused-clock test
  whose first half tracks M42 in process on a state-file session, using the goto rig's
  `Rig::with_session`, a centred start with its wait, and its own JSON
  `POST /api/track`, and asserts the reply and, within 100 ticks, the motion of US5-S1,
  as plan.md § Interfaces "US5" item 1 says (FR-013).

### Implementation

- [ ] T023 [US5] In `crates/finder/tests/cli/track.rs`, keep only the second start of
  the built binary and the assertions of US5-S2; remove the first start, the telescope
  push, the two real-time waits and the sleep; add the comment of US5-S3. The file's other
  tests stay unchanged.
- [ ] T024 [US5] Run `cargo fmt --all`, then the `lint` and `test` gates. Commit the
  green story.

## Phase 6: User Story 6 — The built binary is started only for what `main.rs` does (Priority: P2)

### Tests first

- [ ] T025 [US6] Create `crates/finder/tests/console/session/durable.rs` with
  `durable_fields_survive_reopening_the_state_file_in_owner_only_mode` as plan.md
  § Interfaces "US6" item 1 gives it, comparing the token in the stored file, and add
  `mod durable;` to `crates/finder/tests/console/session/mod.rs` (FR-014, US6-S1).
- [ ] T026 [US6] In `crates/finder/tests/cli/aim.rs`, keep the built binary's stderr
  lines and assert on drop that none contains the node token (FR-016, US6-S2).
- [ ] T027 [US6] In `crates/finder/tests/cli/backlash.rs`, replace `Server::request` with
  `request(method, path, body)`, post the fixed reset to the second restarted binary and
  assert the reply, the session and the state file hold no `backlash`, as plan.md
  § Interfaces "US6" item 4 says (FR-017, US6-S3).

### Implementation

- [ ] T028 [US6] Delete `crates/finder/tests/cli/session_node.rs` and its `mod` line in
  `crates/finder/tests/cli/main.rs` (FR-015). In
  `crates/finder/tests/motion/tracking/backlash.rs`, delete
  `first_transit_measures_then_fixed_reset_removes_the_observed_value` and any import
  only it used (FR-017, US6-S4).
- [ ] T029 [US6] Run `cargo fmt --all`, then the `lint` and `test` gates. Commit the
  green story.

## Phase 7: User Story 7 — Preview retention and the unsolved interval need fewer frames (Priority: P2)

### Tests first

- [ ] T030 [US7] In `crates/finder/tests/console/aim.rs`, add the preview-stride
  assertion of US7-S1; in `crates/finder/tests/console/live/loop_.rs`, count the frames
  and add the bound of US7-S2. Both fail at 500 ms.

### Implementation

- [ ] T031 [US7] In `crates/finder/tests/support/live.rs`, add `state_with_interval` and
  make `state` call it with 500 ms (FR-018, US7-S3).
- [ ] T032 [US7] Call `state_with_interval` with 3 s in
  `crates/finder/tests/console/aim.rs` (FR-019) and with 5 s in
  `crates/finder/tests/console/live/loop_.rs` (FR-020). Every other assertion of the two
  tests stays.
- [ ] T033 [US7] Run `cargo fmt --all`, then the `lint` and `test` gates. Commit the
  green story.

## Phase 8: User Story 8 — No test depends on how fast the machine is (Priority: P2)

### Implementation

- [ ] T034 [US8] In `crates/finder/tests/motion/tracking/night.rs`, remove the
  wall-clock read and assertion, rename the test, and update the import and the module
  comment, as plan.md § Interfaces "US8" item 1 says (FR-021, US8-S1).
- [ ] T035 [US8] Pass `timeout_ms: u64::MAX` from the inline solver in
  `crates/finder/tests/support/goto.rs`, `crates/finder/tests/support/mcp.rs`,
  `crates/finder/tests/support/stop.rs`, `crates/finder/tests/motion/night.rs` and
  `crates/finder/tests/motion/mount_identity.rs` (FR-022, US8-S2).
- [ ] T036 [US8] In `crates/finder/tests/cli/simulate.rs`, read the session time again
  immediately before the 5 s window and compare against it (FR-023, US8-S3).
- [ ] T037 [US8] In `crates/scope-node/tests/cli.rs`, make `run` spawn the child with
  stdin null, kill it if it is still running after 10 s, remove its config file and
  panic naming its command-line arguments (FR-024, US8-S4).
- [ ] T038 [US8] In `crates/scope-node/src/rpicam/metadata.rs`, put the three tests on
  `#[tokio::test(start_paused = true)]` (FR-025, US8-S5).
- [ ] T039 [US8] Run `cargo fmt --all`, then the `lint` and `test` gates. Commit the
  green story.

## Phase 9: User Story 9 — Scratch folders are made and removed in one place (Priority: P3)

### Tests first

- [ ] T040 [US9] In `crates/finder/tests/support/mod.rs`, add `Scratch` as plan.md
  § Interfaces "US9" item 1 gives it (FR-026, US9-S1). Add no file to
  `crates/finder/tests/support` (US9-S5).

### Implementation

- [ ] T041 [US9] Use `Scratch` in `crates/finder/tests/cli/aim.rs`,
  `crates/finder/tests/cli/session.rs`, `crates/finder/tests/cli/backlash.rs`,
  `crates/finder/tests/cli/simulate.rs` and `crates/finder/tests/cli/track.rs`, removing
  each file's own folder type (FR-027).
- [ ] T042 [US9] Use `Scratch` in `crates/finder/tests/console/session/writes.rs`,
  `crates/finder/tests/console/targets.rs`, `crates/finder/tests/console/session/edits.rs`
  and `crates/finder/tests/console/session/durable.rs` (as
  `Scratch::new("session-durable")`, dropping its `Folder` type, its first
  `remove_dir_all` and its `create_dir_all`), keeping, in `targets.rs` and `edits.rs`, the
  explicit assertion that the child left its folder empty (FR-027, US9-S3).
- [ ] T043 [US9] Use `Scratch` in `crates/finder/tests/mcp/endpoint.rs`,
  `crates/finder/tests/mcp/motion.rs` and `crates/finder/tests/mcp/axis_moves.rs`
  (FR-027).
- [ ] T044 [US9] Use `Scratch` in `crates/finder/tests/motion/model/sim.rs`,
  `crates/finder/tests/motion/model/routes.rs`,
  `crates/finder/tests/motion/tracking/backlash.rs`,
  `crates/finder/tests/motion/tracking/keyhole.rs`,
  `crates/finder/tests/motion/tracking/night.rs` and
  `crates/finder/tests/motion/goto/sim.rs`; the keyhole and tracking-night rigs hold
  their `Scratch` for their lifetime (FR-027).
- [ ] T045 [US9] In `crates/scope-node/tests/cli.rs`,
  `crates/scope-node/tests/mount_config.rs`, `crates/scope-node/tests/rpicam.rs` and
  `crates/scope-node/tests/mount_link.rs`, build each scratch path under
  `CARGO_TARGET_TMPDIR` and remove it on drop, with the guards plan.md § Interfaces "US9"
  item 3 describes; `mount_link.rs`'s guard is a file-local `Scratch`, and its own
  `remove_dir_all` goes (FR-028, US9-S4).
- [ ] T046 [US9] Check that no file under `crates/finder/tests` still joins
  `CARGO_MANIFEST_DIR` with `../../target` (US9-S2). Run `cargo fmt --all`, then the
  `lint` and `test` gates. Commit the green story.

## Phase 10: User Story 10 — Duplicates with a stricter twin, and tests that cannot fail, go (Priority: P3)

### Implementation

- [ ] T047 [US10] In `crates/grpc-service/tests/service_acceptance.rs`, delete the three
  tests of FR-029 and any helper or import only they used.
- [ ] T048 [US10] In `crates/plate-solver/tests/solve_acceptance.rs`, delete
  `explicit_detection_parameters_honored` and the solve half of `cancellation_honored`,
  and rename the two survivors, as plan.md § Interfaces "US10" says; leave
  `detection_parameters_do_not_perturb_solve_math` unchanged (FR-030, US10-S2).
- [ ] T049 [US10] In `crates/database-generation/src/patterns.rs`, delete
  `breadth_first_yields_brightest_first` and move the seven-element order test into
  `tests` unchanged (FR-031, US10-S3).
- [ ] T050 [US10] Delete the duplicate in `crates/simulator/tests/mount.rs`, reduce and
  rename the test in `crates/scope-node/tests/skywatcher.rs`, and delete the test module
  and its fake in `crates/scope-node/src/mount/mod.rs` (FR-032, US10-S3).
- [ ] T051 [US10] In `crates/finder/src/session.rs`, delete the two inline tests of
  FR-033, and the test module if it is left empty.
- [ ] T052 [US10] Move the tilt assertions into the durable night and delete the
  standalone second-sync test in `crates/finder/tests/motion/model/sim.rs`; delete the
  four refusal tests and their unused imports in
  `crates/finder/tests/motion/goto/routes.rs`; assert `no mount` in
  `crates/finder/tests/motion/goto/refusals.rs`; narrow the foreign-origin loop in
  `crates/finder/tests/motion/model/routes.rs` (FR-034, US10-S4, US10-S5).
- [ ] T053 [US10] In `crates/finder/tests/mcp/motion.rs`, give `stopped()` its `watch`
  flag and pass `true` only for the second-client case (FR-035, US10-S6).
- [ ] T054 [US10] Check that the story's diff deletes exactly the functions US10-S1
  lists. Run `cargo fmt --all`, then the `lint` and `test` gates. Commit the green story.

## Phase 11: User Story 11 — Optional: the whole night is shortened (Priority: P3)

### Implementation

- [ ] T055 [US11] In `crates/finder/tests/motion/night.rs`, replace the five GoTos with
  one to M81 under the same assertions and rename the test, as plan.md § Interfaces
  "US11" says (FR-036, US11-S1, US11-S2).
- [ ] T056 [US11] Run `cargo fmt --all`, then the `lint` and `test` gates. Commit the
  green story.

## Phase 12: User Story 12 — Optional: the MCP hold is shortened (Priority: P3)

### Implementation

- [ ] T057 [US12] In `crates/finder/tests/mcp/tracking.rs`, make the hold 75 s at both
  of its literals, rename the test and restate the span in the comment above the loop;
  the closing wall-time message may say the new span in place of `ten-minute`; change
  nothing else (FR-037, US12-S1, US12-S2).
- [ ] T058 [US12] Run `cargo fmt --all`, then the `lint` and `test` gates. Commit the
  green story.

## Phase 13: User Story 13 — Optional: the sixty-second window is proven below the route (Priority: P3)

### Tests first

- [ ] T059 [US13] In `crates/finder/tests/motion/tracking/tracker.rs`, split the
  1202-tick run at 1198 ticks and assert the sample is still held there (FR-038,
  US13-S1).

### Implementation

- [ ] T060 [US13] In `crates/finder/tests/motion/tracking/routes.rs`, end the readout
  test's tracking once the tracker holds more than 30 samples, keeping 750 ticks as the
  loop's bound, drop its two window assertions, and add the injected-sample window check
  of plan.md § Interfaces "US13" item 2, with its sample of 12″ east and −9″ north
  (FR-038, US13-S2).
- [ ] T061 [US13] Run `cargo fmt --all`, then the `lint` and `test` gates. Commit the
  green story.

## Execution Order

```text
US1, US2, US3, US4, US6, US7      start at once
US3 ──────────────────────────────► US5
US3, US4 ─────────────────────────► US8
US1, US3, US5, US6, US8 ──────────► US9 ──► US10
US4, US8 ─────────────────────────► US11 (optional)
US12 (optional)                   starts at once
US2 ──────────────────────────────► US13 (optional)
```

- **US1, US2, US3, US4, US6 and US7** start at once: no two write one file.
- **US5** waits for US3, whose `Rig::with_session` it uses.
- **US8** waits for US3 and US4: it edits the goto rig and the whole night after them.
- **US9** waits for US1, US3, US5, US6 and US8: it moves the scratch folders of files
  those stories edit.
- **US10** waits for US9, which edits three of its files first.
- **The optional stories** are the lines marked optional above. No story waits for one,
  so the owner can delete any of them before the flip.
