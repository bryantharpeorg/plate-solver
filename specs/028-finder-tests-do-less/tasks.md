---
description: "Task list for 028-finder-tests-do-less: Finder's Tests Do Less Work"
---

# Tasks: Finder's Tests Do Less Work

**Prerequisites** (plan.md § Prerequisites):
- `027-finder-tests-by-system`, `024-mount-link-recovery` (at f0ea2d5) and
  `025-frame-capture` have landed on `main`: 025's US1 and US2 at 9d8cc4e, where this
  trio was re-anchored, and its US3 at 820c558, which writes only web files;
- the owner has kept or deleted the optional US6, or any one of its three trims;
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

## Phase 1: User Story 1 — Frames nobody reads are not made (Priority: P1)

### Camera off: tests first

- [ ] T001 [US1] In `crates/finder/tests/mcp/axis_moves.rs`, add `newest_frame` as
  plan.md § Interfaces "Shared helpers" gives it. In each of the six direct-move tests of
  FR-001, read `newest_frame` right after the first sync and assert at the end that the
  newest frame id is at most one more. The bound fails while frames still run (US1-S1,
  US1-S2).

### Camera off: implementation

- [ ] T002 [US1] In `crates/finder/tests/mcp/axis_moves.rs`, add `camera_off` and make
  each of the six reads of T001 a call to it, placed as plan.md § Interfaces "US1, camera
  off" item 1 says, including the sunrise test's own rig, where the binding is
  `last_frame` (FR-001). Change no other line of those tests.
- [ ] T003 [US1] In `crates/finder/tests/mcp/axis_moves.rs`, give
  `track_through_the_page_blocks_all_three_mcp_moves_and_keeps_tracking` the centred
  start of plan.md § Interfaces "Shared helpers", with its wait for the first frame solved
  on M42, and assert that the session reads `tracking` with no `goto` after the page's
  Track request, before the wait loop (FR-002, US1-S3).
- [ ] T004 [US1] In `crates/simulator/tests/support/mod.rs`, make `Rig::new` build the
  node on the synthetic camera with no `Simulator` and no pattern database, and assert
  the `Describe` model `simulated` before it returns; add `Rig::rendered` with what
  `Rig::new` builds today, both through the private `on` of plan.md § Interfaces "US1,
  camera off" item 2; replace the field `sim` with `clock` (FR-003, US1-S4).
  `crates/simulator/tests/stop_and_watchdog.rs` and
  `crates/simulator/tests/watchdog_report.rs` stay unchanged.
- [ ] T005 [US1] In `crates/simulator/tests/frame_positions.rs`, call `Rig::rendered`
  and read `rig.clock`, and change nothing else (US1-S5).

### The whole night: tests first

- [ ] T006 [US1] In `crates/finder/tests/motion/night.rs`, after the first sync, add the
  cadence assertion of plan.md § Interfaces "US1, the whole night" item 2, and see it
  fail at the 200 ms default (US1-S6).

### The whole night: implementation

- [ ] T007 [US1] In `crates/simulator/src/controls.rs`, add
  `Simulator::set_frame_interval`, one call to the shared `NodeControls` (FR-011,
  US1-S7).
- [ ] T008 [US1] In `crates/finder/tests/motion/night.rs`, call it with 500 ms right
  after the test takes the simulator from the console's state. The whole-night test
  function's only changes are that call and the cadence assertion of T006; the solver
  edit of T012 is outside it (FR-012, US1-S7).

### Console nodes: tests first

- [ ] T009 [US1] In `crates/finder/tests/console/aim.rs`, add the preview-stride
  assertion of US1-S8; in `crates/finder/tests/console/live/loop_.rs`, count the frames
  and add the bound of US1-S9. Both fail at 500 ms.

### Console nodes: implementation

- [ ] T010 [US1] In `crates/finder/tests/support/live.rs`, add `state_with_interval` and
  make `state` call it with 500 ms (FR-018, US1-S10).
- [ ] T011 [US1] Call `state_with_interval` with 3 s in
  `crates/finder/tests/console/aim.rs` (FR-019) and with 5 s in
  `crates/finder/tests/console/live/loop_.rs` (FR-020). Every other assertion of the two
  tests stays.

### The night's solver: implementation

- [ ] T012 [US1] Pass `timeout_ms: u64::MAX` from the inline solver in
  `crates/finder/tests/motion/night.rs`, as plan.md § Interfaces "US1, the night's
  solver" says (FR-040, US1-S11).
- [ ] T013 [US1] Run `cargo fmt --all`, then the `lint` and `test` gates. Commit the
  green story.

## Phase 2: User Story 2 — Tests start where their claim starts (Priority: P1)

### Centred starts: tests first

- [ ] T014 [US2] In `crates/finder/tests/motion/tracking/routes.rs`, add `centred()` as
  plan.md § Interfaces "US2, centred starts" item 1 gives it, with the centred-start wait
  of "Shared helpers" and its assertions that Track's reply reads `tracking`, names M42
  with `wanted` true and holds no `goto`, and check that the reply no longer reads
  `tracking` with no `goto` when the `point_eyepiece_at` line is removed (US2-S1).

### Centred starts: implementation

- [ ] T015 [US2] In `crates/finder/tests/motion/tracking/routes.rs`, call `centred()`
  instead of `accepted()` in the four tests FR-004 names. Leave `accepted()`,
  `tracking()`, the three handoff tests and the readout test unchanged (US2-S5).
- [ ] T016 [US2] In `crates/finder/tests/motion/tracking/min_alt.rs`, centre on M42
  before the sync, wait as the centred-start recipe says, and assert the Track reply, as
  plan.md § Interfaces "US2, centred starts" item 2 says (FR-005, US2-S2).
- [ ] T017 [US2] In `crates/finder/tests/motion/stop/motion_ends.rs` (its `tracking()`
  helper only; `ready()` is unchanged) and in
  `crates/finder/tests/motion/stop/retry.rs`, centre on M42 before the sync, wait as the
  centred-start recipe says and, once tracking runs, assert that the command log holds no
  `MountCommand::GoTo` (FR-006, US2-S3).
- [ ] T018 [US2] In `crates/finder/tests/motion/stop/watchdog.rs`, start
  `past_watchdog_firing_is_the_baseline_for_a_successful_later_goto` 5° below M42, tick
  after the sync until the session's pointing is within 6° of M42 and assert it before
  the watchdog, and assert the true direction within 6° before its GoTo, as plan.md
  § Interfaces "US2, centred starts" item 5 says, keeping every later assertion (FR-007,
  US2-S4).

### Stored syncs: tests first

- [ ] T019 [US2] In `crates/finder/tests/support/goto.rs`, add `Rig::with_session` and
  make `Rig::new` call it with `Session::default()`, keeping its signature (FR-008,
  US2-S9).
- [ ] T020 [US2] In `crates/finder/tests/support/goto.rs`, add `mount_config`,
  `far_sync` (writing the near and far syncs) and `second_from_file` as plan.md
  § Interfaces "US2, stored syncs" item 2 gives them, with the assertions of US2-S6: 4500
  azimuth counts, a base tilt within 1′ of the simulator's 1° toward 130°, three syncs
  (the two stored and the live one) and one epoch.

### Stored syncs: implementation

- [ ] T021 [US2] In `crates/finder/tests/motion/goto/sim.rs`, build the five-target
  test's rig from the stored near and far syncs and call `second_from_file`, as plan.md
  § Interfaces "US2, stored syncs" item 3 says. Its five GoTos and every assertion after
  the setup stay unchanged (FR-009, US2-S6, US2-S7).
- [ ] T022 [US2] In `crates/finder/tests/motion/goto/sim.rs`, first assert the frame-id
  advance of US2-S8 during the 350° move and see it fail at the default interval; then
  set 2 s before the move and 500 ms after it (FR-010).

### Track restart: tests first

- [ ] T023 [US2] In `crates/finder/tests/cli/track.rs`, make
  `tracking_that_moved_the_mount_is_off_and_quiet_after_restart` a paused-clock test
  whose first half tracks M42 in process on a state-file session, using the goto rig's
  `Rig::with_session`, a centred start with its wait, and its own JSON
  `POST /api/track`, and asserts the reply and, within 100 ticks, the motion of US2-S10,
  as plan.md § Interfaces "US2, Track restart" item 1 says (FR-013).

### Track restart: implementation

- [ ] T024 [US2] In `crates/finder/tests/cli/track.rs`, keep only the second start of
  the built binary and the assertions of US2-S11; remove the first start, the telescope
  push, the two real-time waits and the sleep; add the comment of US2-S12. The file's
  other tests stay unchanged.

### The goto rig's solver: implementation

- [ ] T025 [US2] Pass `timeout_ms: u64::MAX` from the inline solver in
  `crates/finder/tests/support/goto.rs`, as plan.md § Interfaces "US2, the goto rig's
  solver" says (FR-039, US2-S13).
- [ ] T026 [US2] Run `cargo fmt --all`, then the `lint` and `test` gates. Commit the
  green story.

## Phase 3: User Story 3 — The built binary only for `main.rs`, and no test depends on how fast the machine is (Priority: P2)

### The binary only for `main.rs`: tests first

- [ ] T027 [US3] Create `crates/finder/tests/console/session/durable.rs` with
  `durable_fields_survive_reopening_the_state_file_in_owner_only_mode` as plan.md
  § Interfaces "US3, the binary only for `main.rs`" item 1 gives it, comparing the token
  in the stored file, and add `mod durable;` to
  `crates/finder/tests/console/session/mod.rs` (FR-014, US3-S1).
- [ ] T028 [US3] In `crates/finder/tests/cli/aim.rs`, keep the built binary's stderr
  lines and assert on drop that none contains the node token (FR-016, US3-S2).
- [ ] T029 [US3] In `crates/finder/tests/cli/backlash.rs`, replace `Server::request` with
  `request(method, path, body)`, post the fixed reset to the second restarted binary and
  assert the reply, the session and the state file hold no `backlash`, as plan.md
  § Interfaces "US3, the binary only for `main.rs`" item 4 says (FR-017, US3-S3).

### The binary only for `main.rs`: implementation

- [ ] T030 [US3] Delete `crates/finder/tests/cli/session_node.rs` and its `mod` line in
  `crates/finder/tests/cli/main.rs` (FR-015). In
  `crates/finder/tests/motion/tracking/backlash.rs`, delete
  `first_transit_measures_then_fixed_reset_removes_the_observed_value` and any import
  only it used (FR-017, US3-S4).

### Hazards: implementation

- [ ] T031 [US3] In `crates/finder/tests/motion/tracking/night.rs`, remove the
  wall-clock read and assertion, rename the test, and update the import and the module
  comment, as plan.md § Interfaces "US3, hazards" item 1 says (FR-021, US3-S5).
- [ ] T032 [US3] Pass `timeout_ms: u64::MAX` from the inline solver in
  `crates/finder/tests/support/mcp.rs`, `crates/finder/tests/support/stop.rs` and
  `crates/finder/tests/motion/mount_identity.rs` (FR-022, US3-S6).
- [ ] T033 [US3] In `crates/finder/tests/cli/simulate.rs`, read the session time again
  immediately before the 5 s window and compare against it (FR-023, US3-S7).
- [ ] T034 [US3] In `crates/scope-node/tests/cli.rs`, make `run` spawn the child with
  stdin null, kill it if it is still running after 10 s, remove its config file and
  panic naming its command-line arguments (FR-024, US3-S8).
- [ ] T035 [US3] In `crates/scope-node/src/rpicam/metadata.rs`, put the three tests on
  `#[tokio::test(start_paused = true)]` (FR-025, US3-S9).
- [ ] T036 [US3] Run `cargo fmt --all`, then the `lint` and `test` gates. Commit the
  green story.

## Phase 4: User Story 4 — Duplicates with a stricter twin, and tests that cannot fail, go (Priority: P3)

### Implementation

- [ ] T037 [US4] In `crates/grpc-service/tests/service_acceptance.rs`, delete the three
  tests of FR-029 and any helper or import only they used.
- [ ] T038 [US4] In `crates/plate-solver/tests/solve_acceptance.rs`, delete
  `explicit_detection_parameters_honored` and the solve half of `cancellation_honored`,
  and rename the two survivors, as plan.md § Interfaces "US4" says; leave
  `detection_parameters_do_not_perturb_solve_math` unchanged (FR-030, US4-S2).
- [ ] T039 [US4] In `crates/database-generation/src/patterns.rs`, delete
  `breadth_first_yields_brightest_first` and move the seven-element order test into
  `tests` unchanged (FR-031, US4-S3).
- [ ] T040 [US4] Delete the duplicate in `crates/simulator/tests/mount.rs`, reduce and
  rename the test in `crates/scope-node/tests/skywatcher.rs`, and delete the test module
  and its fake in `crates/scope-node/src/mount/mod.rs` (FR-032, US4-S3).
- [ ] T041 [US4] In `crates/finder/src/session.rs`, delete the two inline tests of
  FR-033, and the test module if it is left empty.
- [ ] T042 [US4] Move the tilt assertions into the durable night and delete the
  standalone second-sync test in `crates/finder/tests/motion/model/sim.rs`; delete the
  four refusal tests and their unused imports in
  `crates/finder/tests/motion/goto/routes.rs`; assert `no mount` in
  `crates/finder/tests/motion/goto/refusals.rs`; narrow the foreign-origin loop in
  `crates/finder/tests/motion/model/routes.rs` (FR-034, US4-S4, US4-S5).
- [ ] T043 [US4] In `crates/finder/tests/mcp/motion.rs`, give `stopped()` its `watch`
  flag and pass `true` only for the second-client case (FR-035, US4-S6).
- [ ] T044 [US4] Check that the story's diff deletes exactly the functions US4-S1
  lists. Run `cargo fmt --all`, then the `lint` and `test` gates. Commit the green story.

## Phase 5: User Story 5 — Scratch folders are made and removed in one place (Priority: P3)

### Tests first

- [ ] T045 [US5] In `crates/finder/tests/support/mod.rs`, add `Scratch` as plan.md
  § Interfaces "US5" item 1 gives it (FR-026, US5-S1). Add no file to
  `crates/finder/tests/support` (US5-S5).

### Implementation

- [ ] T046 [US5] Use `Scratch` in `crates/finder/tests/cli/aim.rs`,
  `crates/finder/tests/cli/session.rs`, `crates/finder/tests/cli/backlash.rs`,
  `crates/finder/tests/cli/simulate.rs` and `crates/finder/tests/cli/track.rs`, removing
  each file's own folder type (FR-027). In `crates/finder/tests/cli/recording.rs`, give
  the binary's `--state` a path inside a `Scratch` that does not exist yet, declared
  before the `Console` guard, which keeps killing and reaping the child and loses its
  `remove_dir_all` (FR-027; plan.md § Interfaces "US5" item 2).
- [ ] T047 [US5] Use `Scratch` in `crates/finder/tests/console/session/writes.rs`,
  `crates/finder/tests/console/targets.rs`, `crates/finder/tests/console/session/edits.rs`
  and `crates/finder/tests/console/session/durable.rs` (as
  `Scratch::new("session-durable")`, dropping its `Folder` type, its first
  `remove_dir_all` and its `create_dir_all`), keeping, in `targets.rs` and `edits.rs`, the
  explicit assertion that the child left its folder empty (FR-027, US5-S3). Use it too in
  `crates/finder/tests/console/recording.rs` and
  `crates/finder/tests/console/recording_raw.rs`, dropping their `Folder` types; in
  `console/recording.rs`'s refusal test the recordings root stays a path inside its
  `Scratch` that does not exist yet, so its absence assertions and its regular-file case
  stay (FR-027; plan.md § Interfaces "US5" item 2).
- [ ] T048 [US5] Use `Scratch` in `crates/finder/tests/mcp/endpoint.rs`,
  `crates/finder/tests/mcp/motion.rs` and `crates/finder/tests/mcp/axis_moves.rs`
  (FR-027).
- [ ] T049 [US5] Use `Scratch` in `crates/finder/tests/motion/model/sim.rs`,
  `crates/finder/tests/motion/model/routes.rs`,
  `crates/finder/tests/motion/tracking/backlash.rs`,
  `crates/finder/tests/motion/tracking/keyhole.rs`,
  `crates/finder/tests/motion/tracking/night.rs`,
  `crates/finder/tests/motion/goto/sim.rs` and
  `crates/finder/tests/motion/recording_raw.rs`; the keyhole and tracking-night rigs hold
  their `Scratch` for their lifetime (FR-027).
- [ ] T050 [US5] In `crates/scope-node/tests/cli.rs`,
  `crates/scope-node/tests/mount_config.rs`, `crates/scope-node/tests/rpicam.rs` and
  `crates/scope-node/tests/mount_link.rs`, build each scratch path under
  `CARGO_TARGET_TMPDIR` and remove it on drop, with the guards plan.md § Interfaces "US5"
  item 3 describes; `mount_link.rs`'s guard is a file-local `Scratch`, and its own
  `remove_dir_all` goes (FR-028, US5-S4).
- [ ] T051 [US5] Check that no file under `crates/finder/tests` still joins
  `CARGO_MANIFEST_DIR` with `../../target` (US5-S2). Run `cargo fmt --all`, then the
  `lint` and `test` gates. Commit the green story.

## Phase 6: User Story 6 — Optional: three trims of landed scenarios (Priority: P3)

### The whole night shortened: implementation

- [ ] T052 [US6] In `crates/finder/tests/motion/night.rs`, replace the five GoTos with
  one to M81 under the same assertions and rename the test, as plan.md § Interfaces
  "US6, the whole night shortened" says (FR-036, US6-S1, US6-S2).

### The MCP hold shortened: implementation

- [ ] T053 [US6] In `crates/finder/tests/mcp/tracking.rs`, make the hold 75 s at both
  of its literals, rename the test and restate the span in the comment above the loop;
  the closing wall-time message may say the new span in place of `ten-minute`; change
  nothing else (FR-037, US6-S3, US6-S4).

### The window below the route: tests first

- [ ] T054 [US6] In `crates/finder/tests/motion/tracking/tracker.rs`, split the
  1202-tick run at 1198 ticks and assert the sample is still held there (FR-038,
  US6-S5).

### The window below the route: implementation

- [ ] T055 [US6] In `crates/finder/tests/motion/tracking/routes.rs`, end the readout
  test's tracking once the tracker holds more than 30 samples, keeping 750 ticks as the
  loop's bound, drop its two window assertions, and add the injected-sample window check
  of plan.md § Interfaces "US6, the window below the route" item 2, with its sample of
  12″ east and −9″ north (FR-038, US6-S6).
- [ ] T056 [US6] Run `cargo fmt --all`, then the `lint` and `test` gates. Commit the
  green story.

## Execution Order

```text
US1, US2, US3, US4                start at once (round 1)
US1, US2, US3, US4 ───────────────► US5 (round 2)
US1, US2 ─────────────────────────► US6 (optional, round 2)
```

- **US1, US2, US3 and US4** start at once: no two write one file.
- **US5** waits for all four: it moves the scratch folders of files US1, US2 and US3
  edit, and US4 edits three of its files first.
- **US6** is optional and waits for US1 and US2: it edits `motion/night.rs` after US1 and
  `motion/tracking/routes.rs` after US2. No story waits for it, so the owner can delete
  it, or any one of its trims, before the flip.
