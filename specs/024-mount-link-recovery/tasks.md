---
description: "Task list for 024-mount-link-recovery: The Mount Link Survives a Power Cycle"
---

# Tasks: The Mount Link Survives a Power Cycle

**Prerequisites** (plan.md § Prerequisites):
- `013-live-finder`, `014-goto`, `015-tracking`, `017-console-stop-confirm` and
  `027-finder-tests-by-system` have landed on `main` (plan.md § Prerequisites 1);
- `main` is still as read: every citation was read at c1ed295 and again at b8dceaa,
  where only the finder test paths 027 moved had changed, and a later `main` that moves
  in 024's files needs them re-read (plan.md § Prerequisites 2);
- the owner decisions in spec.md are settled, including the owner's 2026-11-15 answers
  and gap (d) closed, and the owner approvals given (plan.md § Prerequisites 3);
- no operator-staged file is needed (plan.md § Prerequisites 4);
- the drafts `025-frame-capture` and `026-mount-sky`, which also write 024's files, wait
  for this spec to land and then name it themselves, so this spec names neither (plan.md
  § Prerequisites 5);
- `027-finder-tests-by-system` has landed (b8dceaa), and US3's test and citations are
  re-anchored to its layout: the test is a module of finder's `motion` binary (plan.md
  § Prerequisites 6). Met.

**Rules for every story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml`,
  `CODEOWNERS`, `ergane.yaml`, `.specify/` or `crates/finder/web/playwright.config.ts`,
  but for US3's one `mod mount_identity;` line in `crates/finder/tests/motion/main.rs`
  (spec.md owner approval 7). If an existing test fails, stop and escalate with its
  exact output. Do not edit it.
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
  `Ok(handle.reopen(clock.clone()))`, paused time, a copy of the `Clock` in
  `crates/scope-node/tests/skywatcher.rs:18`) and failing tests for US1-S1 and US1-S2.
- [ ] T002 [US1] Add failing tests to `crates/scope-node/tests/mount_link.rs` for the
  service and the frame over a disconnected link (US1-S3, US1-S4), for the built
  binary with a missing device on a free loopback port (US1-S5), and for the emulator
  handle's `set_position`, `inject_reply` and `power_cycle` on a bare `SkyWatcher`
  (US1-S6).

### Implementation

- [ ] T003 [US1] In `crates/scope-node/src/mount/mod.rs`, add `MountIdentity` and the
  `connected`, `resets` and `identity` defaults of plan.md § Interfaces.
- [ ] T004 [US1] In `crates/scope-node/src/mount/skywatcher/emulator.rs`, add `reopen`,
  `power_off`, `power_on`, `power_cycle`, `set_position` and `inject_reply` to
  `EmulatorHandle` exactly as plan.md § Interfaces gives them, with the behaviour it
  states for a powered-off board, injected replies, faults across a cycle and
  `set_position`. US2 and US4 depend on all six and may not write this file.
- [ ] T005 [US1] Create `crates/scope-node/src/mount/skywatcher/mount_link.rs` with
  `MountLink`, its `std::sync::Mutex` snapshot, its connect task (aborted on drop, and
  holding only the link's inner state) and FR-003's line, and its `Mount` impl
  including `connected` and `identity()` from `report().firmware`, but not `health`,
  which is US4's; add `pub mod mount_link;` and
  `pub use mount_link::{MountLink, Opener};` to
  `crates/scope-node/src/mount/skywatcher/mod.rs`.
- [ ] T006 [US1] In `crates/scope-node/src/service.rs`, refuse and describe as plan.md
  § Interfaces says while not connected, with the connection check before the
  watchdog's in `moving_mount` and Stop still through the queue; in
  `crates/scope-node/src/capture.rs`, carry no counts then; in
  `crates/scope-node/src/main.rs`, start a `MountLink` instead of awaiting
  `SkyWatcher::connect`, logging nothing per connect.
- [ ] T007 [US1] Add the `## The mount link` section to `crates/scope-node/README.md`
  with US1's part only: starting without the mount, and FR-003's line; say that Stop
  answers unconfirmed at once before the link's first connect, so the sentence stays
  true after US2 (plan.md § Interfaces).
- [ ] T008 [US1] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US1-S1, US1-S2, US1-S3, US1-S4, US1-S5, US1-S6).

## Phase 2: User Story 2 — A power cycle initialises the board again (Priority: P1)

### Tests first

- [ ] T009 [US2] Create `crates/scope-node/tests/mount_reset.rs` with the shared setup
  and failing tests for a power cycle (US2-S1), a lost and recovered link after a move
  (US2-S2) and a counts jump (US2-S3), and a test for a GoTo that is not a jump
  (US2-S4). US2-S4 guards behaviour the story must keep (travel within the top rate is
  no reset): it is a preserving test, not a failing one, although the file does not
  compile before T013 adds `mount_resets`.
- [ ] T010 [US2] Add failing tests to `crates/scope-node/tests/mount_reset.rs` for
  `mount_resets` on `axis_state` (US2-S5), moves dropped while disconnected (US2-S6),
  one reset for a cycle after a move (US2-S7), a failed re-initialisation with a Stop
  the board acknowledges (US2-S9) and a dropped read during one (US2-S10), Stop over a
  lost link to a powered board running a `:J` move (US2-S11), to a powered-off board
  (US2-S12) and through the service's `stop_all` (US2-S13), and a test for a bit that
  never read set (US2-S8), which like US2-S4 is a preserving test, not a failing one,
  although the file does not compile before T013 adds `mount_resets`.

### Implementation

- [ ] T011 [US2] In `crates/scope-node/src/mount/skywatcher/protocol.rs`, read the init
  bit into `Status`, record it on each `state` read and clear it on a failed one, add
  `SkyWatcher::initialised`, make `wrapped_delta` `pub(super)` for the link's jump rule,
  and add `SkyWatcher::halt_once`, which sets the board's baud and sends `halt`'s order
  with one transmission a command, leaving `halt` and `stop_confirmed` unchanged
  (plan.md § Interfaces).
- [ ] T012 [US2] In `crates/scope-node/src/mount/skywatcher/mount_link.rs`, add the loss
  rule (three failed `state` reads in a row across both axes; a read while the link has
  no board is not a failed read) and the connect task's loop, the `io` lock, the bit
  rule and the jump rule of spec.md FR-006, the re-initialisation that takes the board
  out first, runs in a spawned task the finding read awaits through its `JoinHandle`
  (so a dropped read leaves it running) and wakes the connect task when it fails, one
  count per event, and the loss, reset and failed re-initialisation log lines; and Stop
  over the last connect's stream: the kept `stop_board`, the `device` hold that every
  connect attempt takes from `open()` until its board is stored, the `give_way`
  notification and the `stopping` count that make an attempt drop its connect, and
  `stop_confirmed` and `stop_all` that answer false at once before the first connect
  without notifying or taking `device`, and otherwise raise `stopping`, notify, take
  `device`, and then delegate to a connected board or call `stop_board.halt_once()`
  (plan.md § Interfaces, **Stop**).
- [ ] T013 [US2] Add `mount_resets = 5` to `AxisState` in
  `crates/scope-link/proto/scope_node/v1/scope_node.proto`, and fill it in
  `crates/scope-node/src/service.rs`.
- [ ] T014 [US2] Add US2's part to the `## The mount link` section of
  `crates/scope-node/README.md`: resets, FR-007's line, the loss and failed
  re-initialisation lines, and that a console restarted after a power cycle needs a
  sync before GoTo. The console's refusal is US3's, so this
  part does not describe it.
- [ ] T015 [US2] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US2-S1, US2-S2, US2-S3, US2-S4, US2-S5, US2-S6,
  US2-S7, US2-S8, US2-S9, US2-S10, US2-S11, US2-S12, US2-S13).

## Phase 3: User Story 3 — The console knows the board and its resets (Priority: P2)

### Tests first

- [ ] T016 [US3] Create `crates/finder/tests/motion/mount_identity.rs`, a module of
  finder's `motion` test binary, and add `mod mount_identity;` to
  `crates/finder/tests/motion/main.rs` in its alphabetical place, after
  `mod mount_calls;`, changing no other line (spec.md owner approval 7). Give the module
  a copy of the `Rig` of `crates/finder/tests/support/mcp.rs:179`, built as
  `crates/finder/tests/support/mcp.rs:188` builds it, whose test `Mount` wraps the
  `SimMount`, records its commands and reports an identity and a reset count the test
  sets, and failing tests for US3-S1 and US3-S2. The module includes no file with
  `#[path]`; a helper it reuses from `crates/finder/tests/support/` it reaches with
  `use crate::support::mcp as support;` (or `stop`), and it adds no file there, since
  027's layout guard pins that folder's files (`crates/finder/tests/layout/main.rs:102`).
  The `DB` static, `NodeTime`, `Stars` and `Recorded` in `support/mcp.rs` are private, so
  the module keeps its own copies with the `Rig`; `support/stop.rs`'s `post` always sends
  a JSON body, so US3-S3 and US3-S4's requests with no body need the module's own `post`.
- [ ] T017 [US3] Add failing tests to `crates/finder/tests/motion/mount_identity.rs` for
  the refusals with frames stopped and their end once frames start again (US3-S3,
  US3-S4), the GoTo abort (US3-S5) and tracking (US3-S6), whose test waits for the
  session's `mount.resets` to read 1 as well as for tracking to end before it posts
  Track again, a reset seen while a GoTo corrects (US3-S7), a reset during a keyhole
  slew of a `Tracker` built over the console's link at a time its target stands inside
  the zenith circle (US3-S8), and a `Tracker` started with `start_with_resets` and the
  count the session last saw while the node already reports one more (US3-S9).

### Implementation

- [ ] T018 [US3] Add `firmware = 2` and `board = 3` to `DescribeMountReply` in
  `crates/scope-link/proto/scope_node/v1/scope_node.proto`, and fill them from
  `identity()` while connected in `crates/scope-node/src/service.rs`.
- [ ] T019 [US3] In `crates/finder/src/live.rs` and `crates/finder/src/session.rs`, add
  `MountReading` (with `pub(crate)` fields) and the poll's console time to `set_mount`,
  the identity, the `resets` field and its `is_zero` helper, `Session::mount_resets`
  (also tracking's baseline, T020), the sync-needed flag that only a sync from a frame
  whose exposure started after the rise was seen clears, `POWER_CYCLED`, and Track's
  refusal in `Session::track_target`; in `crates/finder/src/api.rs`, pass
  `self.clock.clone()` to `MountMonitor::start` in `with_simulation` and `with_link`, and
  add the refusal after a reset to the module docs' second-sync and Track paragraphs,
  with one sentence that `POST /api/goto` refuses the same way (plan.md § Interfaces).
- [ ] T020 [US3] In `crates/finder/src/goto.rs`, add the refusal to
  `GoTo::check_request` and the abort against the count seen when the GoTo was
  accepted, on its first and slewing reads and on a read after each correction's plan,
  before that correction's `goto_axis` pair; in `crates/finder/src/tracking.rs`, add
  `Tracker::start_with_resets`, which takes `start`'s body and hands its count to `run`,
  with `start` keeping its signature and calling it with none, the azimuth read on
  every tick of the rate loop, right after the tick, whose baseline is the given count
  or else the first read's, and pass that baseline to `keyhole` and `at_rest`, stopping
  tracking when the count differs on any of them; and in `Session::begin_tracking`
  (`crates/finder/src/session.rs`), start the new tracker with
  `start_with_resets(target, self.mount_resets())` in place of `start(target)` (plan.md
  § Interfaces).
- [ ] T021 [US3] Run the `lint` and `test` gates with every existing test unchanged and
  green but for T016's one `mod` line. Commit the green story (US3-S1, US3-S2, US3-S3,
  US3-S4, US3-S5, US3-S6, US3-S7, US3-S8, US3-S9).

## Phase 4: User Story 4 — Garbled replies are retried and kept (Priority: P3)

### Tests first

- [ ] T022 [US4] Create `crates/scope-node/tests/mount_garble.rs` with the shared setup
  and failing tests for a short status (US4-S1), five in a row (US4-S2), a short
  position (US4-S3), the GoTo path (US4-S4) and health across a reset (US4-S6), and a
  test for a motion start sent once (US4-S5). US4-S5 guards behaviour the story must
  keep (a motion start is sent once): it is a preserving test, not a failing one,
  although the file does not compile before the story adds `health`.

### Implementation

- [ ] T023 [US4] In `crates/scope-node/src/mount/skywatcher/protocol.rs`, add the retried
  `read`, `MountHealth`, `GotoPath` and `SkyWatcher::health`, and in
  `crates/scope-node/src/mount/skywatcher/mount_link.rs`, `MountLink::health`, its
  snapshot copied before each read's loss and reset checks, the per-axis fold of the
  GoTo path and the re-export of both types (plan.md § Interfaces).
- [ ] T024 [US4] Add US4's part to the `## The mount link` section of
  `crates/scope-node/README.md`: FR-017's two lines.
- [ ] T025 [US4] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US4-S1, US4-S2, US4-S3, US4-S4, US4-S5, US4-S6).

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
