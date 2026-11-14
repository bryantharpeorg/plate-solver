---
description: "Task list for 024-mount-link-recovery: The Mount Link Survives a Power Cycle"
---

# Tasks: The Mount Link Survives a Power Cycle

**Prerequisites** (plan.md § Prerequisites):
- `013-live-finder`, `014-goto`, `015-tracking` and `017-console-stop-confirm` have
  landed on `main`;
- the owner decisions in spec.md are settled and the owner approvals given;
- no operator-staged file is needed.

**Rules for every story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml`,
  `CODEOWNERS`, `ergane.yaml`, `.specify/` or `crates/finder/web/playwright.config.ts`.
  If an existing test fails, stop and escalate with its exact output. Do not edit it.
- Write only the files your story's tasks name; they are its `writes` in plan.md
  § Refinement Record. A story that needs another file escalates.
- Add no field to `EmulatorConfig`, the node's `AxisState`, the Sky-Watcher `Error` or
  any type a test builds by literal (plan.md § Plan traps 9), and add no dependency.
- Tests compute their expected values themselves from the emulator's state and the
  scenario's numbers (plan.md § Plan traps 6).
- Run no tool the gates don't run: no `cargo deny`, `cargo machete`, Rust 1.96 or
  `ergane`.

## Phase 1: User Story 1 — The node starts without the mount (Priority: P1)

### Tests first

- [ ] T001 [US1] Create `crates/scope-node/tests/mount_link.rs` with the shared setup of
  spec.md § User Scenarios (a `MountLink` whose opener returns
  `handle.reopen(clock.clone())`, paused time, a copy of the `Clock` in
  `crates/scope-node/tests/skywatcher.rs:18`) and failing tests for US1-S1 and US1-S2.
- [ ] T002 [US1] Add failing tests to `crates/scope-node/tests/mount_link.rs` for the
  service and the frame over a disconnected link (US1-S3, US1-S4), and for the built
  binary with a missing device on a free loopback port (US1-S5).

### Implementation

- [ ] T003 [US1] In `crates/scope-node/src/mount/mod.rs`, add `MountIdentity` and the
  `connected`, `resets` and `identity` defaults of plan.md § Interfaces.
- [ ] T004 [US1] In `crates/scope-node/src/mount/skywatcher/emulator.rs`, add `reopen`,
  `power_off`, `power_on`, `power_cycle`, `set_position` and `inject_reply` to
  `EmulatorHandle` exactly as plan.md § Interfaces gives them, with the behaviour it
  states for a powered-off board, injected replies, faults across a cycle and
  `set_position`.
- [ ] T005 [US1] Create `crates/scope-node/src/mount/skywatcher/mount_link.rs` with
  `MountLink`, its `std::sync::Mutex` snapshot, its connect task (aborted on drop) and
  FR-003's line, and its `Mount` impl including `connected` and `identity()` from
  `report().firmware`; add `pub mod mount_link;` and
  `pub use mount_link::{MountLink, Opener};` to
  `crates/scope-node/src/mount/skywatcher/mod.rs`.
- [ ] T006 [US1] In `crates/scope-node/src/service.rs`, refuse and describe as plan.md
  § Interfaces says while not connected, with the connection check before the
  watchdog's in `moving_mount` and Stop still through the queue; in
  `crates/scope-node/src/capture.rs`, carry no counts then; in
  `crates/scope-node/src/main.rs`, start a `MountLink` instead of awaiting
  `SkyWatcher::connect`, logging nothing per connect.
- [ ] T007 [US1] Add the `## The mount link` section to `crates/scope-node/README.md`
  with US1's part only: starting without the mount, and FR-003's line (plan.md
  § Interfaces).
- [ ] T008 [US1] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US1-S1, US1-S2, US1-S3, US1-S4, US1-S5).

## Phase 2: User Story 2 — A power cycle initialises the board again (Priority: P1)

### Tests first

- [ ] T009 [US2] Create `crates/scope-node/tests/mount_reset.rs` with the shared setup
  and failing tests for a power cycle (US2-S1), a lost and recovered link after a move
  (US2-S2), a counts jump (US2-S3) and a GoTo that is not a jump (US2-S4).
- [ ] T010 [US2] Add failing tests to `crates/scope-node/tests/mount_reset.rs` for
  `mount_resets` on `axis_state` (US2-S5), moves dropped while disconnected (US2-S6),
  one reset for a cycle after a move (US2-S7) and a bit that never read set (US2-S8).

### Implementation

- [ ] T011 [US2] In `crates/scope-node/src/mount/skywatcher/protocol.rs`, read the init
  bit into `Status`, record it on each `state` read and clear it on a failed one, and
  add `SkyWatcher::initialised` (plan.md § Interfaces).
- [ ] T012 [US2] In `crates/scope-node/src/mount/skywatcher/mount_link.rs`, add the loss
  rule (three failed `state` reads in a row across both axes) and the connect task's
  loop, the `io` lock, the bit rule and the jump rule of spec.md FR-006, the
  re-initialisation that takes the board out first, one count per event, and the
  reset's log line (plan.md § Interfaces).
- [ ] T013 [US2] Add `mount_resets = 5` to `AxisState` in
  `crates/scope-link/proto/scope_node/v1/scope_node.proto`, and fill it in
  `crates/scope-node/src/service.rs`.
- [ ] T014 [US2] Add US2's part to the `## The mount link` section of
  `crates/scope-node/README.md`: resets, FR-007's line, the console's refusal until a
  sync, and that a console restarted after a power cycle needs a sync before GoTo.
- [ ] T015 [US2] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US2-S1, US2-S2, US2-S3, US2-S4, US2-S5, US2-S6,
  US2-S7, US2-S8).

## Phase 3: User Story 3 — The console knows the board and its resets (Priority: P2)

### Tests first

- [ ] T016 [US3] Create `crates/finder/tests/mount_identity.rs` with a copy of the `Rig`
  of `crates/finder/tests/mcp/support.rs:188` whose test `Mount` wraps the `SimMount`,
  records its commands and reports an identity and a reset count the test sets, and
  failing tests for US3-S1 and US3-S2.
- [ ] T017 [US3] Add failing tests to `crates/finder/tests/mount_identity.rs` for the
  refusals with frames stopped and their end once frames start again (US3-S3, US3-S4),
  the GoTo abort (US3-S5) and tracking (US3-S6).

### Implementation

- [ ] T018 [US3] Add `firmware = 2` and `board = 3` to `DescribeMountReply` in
  `crates/scope-link/proto/scope_node/v1/scope_node.proto`, and fill them from
  `identity()` while connected in `crates/scope-node/src/service.rs`.
- [ ] T019 [US3] In `crates/finder/src/live.rs` and `crates/finder/src/session.rs`, add
  `MountReading`, the identity, the `resets` field, the sync-needed flag that only a
  frame exposed after the rise clears, `POWER_CYCLED`, and Track's refusal in
  `Session::track_target` (plan.md § Interfaces).
- [ ] T020 [US3] In `crates/finder/src/goto.rs`, add the refusal to
  `GoTo::check_request` and the abort against the count seen when the GoTo was
  accepted; in `crates/finder/src/tracking.rs`, stop tracking when the count changes
  (plan.md § Interfaces).
- [ ] T021 [US3] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US3-S1, US3-S2, US3-S3, US3-S4, US3-S5, US3-S6).

## Phase 4: User Story 4 — Garbled replies are retried and kept (Priority: P3)

### Tests first

- [ ] T022 [US4] Create `crates/scope-node/tests/mount_garble.rs` with the shared setup
  and failing tests for a short status (US4-S1), five in a row (US4-S2), a short
  position (US4-S3) and the GoTo path (US4-S4), and a test for a motion start sent once
  (US4-S5). US4-S5 already passes before the story and guards that behaviour: it is a
  preserving test, not a failing one.

### Implementation

- [ ] T023 [US4] In `crates/scope-node/src/mount/skywatcher/protocol.rs`, add the retried
  `read`, `MountHealth`, `GotoPath` and `SkyWatcher::health`, and in
  `crates/scope-node/src/mount/skywatcher/mount_link.rs`, `MountLink::health`, its
  snapshot and the re-export of both types (plan.md § Interfaces).
- [ ] T024 [US4] Add US4's part to the `## The mount link` section of
  `crates/scope-node/README.md`: FR-017's two lines.
- [ ] T025 [US4] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US4-S1, US4-S2, US4-S3, US4-S4, US4-S5).

## Execution Order

```text
US1 ──► US2 ──► US3
           └──► US4
```

- **US1** comes first: every other story builds on `MountLink` and the emulator handle.
- **US2** comes next: US3 reads its `mount_resets`, and US4 changes the reads it watches.
- **US3** and **US4** share no file, so they may run beside each other once US2 has
  merged.
- **`[P]`** marks nothing here: every task shares a file with another in its story.
