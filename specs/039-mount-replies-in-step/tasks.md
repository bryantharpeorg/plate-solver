---
description: "Task list for 039-mount-replies-in-step: The Mount's Replies Stay in Step"
---

# Tasks: The Mount's Replies Stay in Step

**Prerequisites** (plan.md § Prerequisites):
- every spec in `depends_on_landed` has landed on `main`: met at 52785403;
- the owner decisions in spec.md are settled: met, the owner settled all six on
  2026-11-20; its owner approvals are given by the flip;
- every story has been probe-built from this text: met by probe round 1 for the text as
  it stood on 2026-11-19; not met for what the fold of 2026-11-20 then added to US1,
  FR-016 in one build with `set_baud` as on `main`, and US1-S15;
- no operator-staged file is needed.

**Rules for every story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml`,
  `CODEOWNERS`, `ergane.yaml` or `.specify/`. If an existing test fails, stop and
  escalate with its exact output. The two exceptions are the ones your story's tasks
  name: US1 adds one test function to the `tests` module of `serial.rs`, beside the
  function there, which it does not edit (spec.md owner approval 6); US3 adds one `mod`
  line in alphabetical order (spec.md owner approval 5). A test file an earlier story
  of this spec created is an existing test.
- Write only the files your story's tasks name; they are its `writes` in plan.md
  § Refinement Record. A story that needs another file escalates.
- `crates/math-core`, `crates/simulator`, `crates/scope-link`, `proto/`, every file
  under `crates/finder/src`, and the node's `mount/mod.rs`, `mount/queue.rs`,
  `mount/watchdog.rs` and `service.rs` are never touched.
- Copy the four log lines from spec.md FR-004, FR-012 and FR-013 character for
  character; no test reads them.
- No test moves real hardware. Run no tool the gates don't run, and do not set
  `CARGO_TARGET_DIR`.

## Phase 1: User Story 1 - A reply left on the line never answers the next command (Priority: P1)

### Tests first

- [ ] T001 [US1] Create `crates/scope-node/tests/mount_step.rs` with the node rig as
  spec.md § User Scenarios defines it, and tests for US1-S1, US1-S2, US1-S3, US1-S4,
  US1-S5, US1-S6, US1-S7, US1-S9, US1-S10, US1-S11, US1-S12, US1-S13, US1-S14 and
  US1-S15; US1-S10 builds a link of its own over the bare emulator. They use `Line`,
  `LineHandle` and `line_health()`, which T003 and T005 add. `SkyWatcher::line_health`
  reads `Transport::stale()`, which is T004's: until T004 is done, let `stale()` answer
  `(0, None)`. With those in place and T004 not yet done, each of the fourteen fails on
  today's transport: the first seven were seen to fail in the probe, and each of the
  others asserts a `stale_replies` of 1, or US1-S15's 600 ms, that today's transport
  cannot give. Every exchange list is compared whole (plan.md § Plan traps 11), and
  every "no `:K`" stands beside a positive (plan.md § Plan traps 12).
- [ ] T002 [US1] Add one test function for US1-S8 to the `tests` module at the foot of
  `crates/scope-node/src/mount/skywatcher/serial.rs`, built on a socket pair as the
  function there builds one. Do not edit that function.

### Implementation

- [ ] T003 [US1] Add `BaudStream::discard_input` in
  `crates/scope-node/src/mount/skywatcher/transport.rs` with its implementation for
  `Emulator` there; `Emulator::discard_input`, `Line` and `LineHandle` in
  `crates/scope-node/src/mount/skywatcher/emulator.rs`; and `Serial`'s implementation
  in `serial.rs`, reading until the device has no more, at most 256 bytes a call
  (plan.md § The byte stream, § The emulator and the test line).
- [ ] T004 [US1] In `transport.rs`: the open exchange, `clear_line` before every try,
  the retry kept open for one reply more (FR-016), `line`, the stale count and its log
  line, and `stale()`. Leave `set_baud` as it is on `main`. Keep `Transport::new`'s and
  `command`'s signatures (plan.md § The transport, with its seven rules).
- [ ] T005 [US1] Declare `LineHealth` with `stale_replies` and `last_stale` in
  `crates/scope-node/src/mount/skywatcher/protocol.rs`, add `SkyWatcher::line_health`,
  and in `crates/scope-node/src/mount/skywatcher/mount_link.rs` re-export it and add
  `MountLink::line_health` with its copy and its fold. Leave `MountHealth` and
  `MountLink::stop` as they are (plan.md § Line health).
- [ ] T006 [US1] Add the paragraph on the line in step to `## The mount link` in
  `crates/scope-node/README.md` (FR-009; plan.md § README).
- [ ] T007 [US1] Run the `lint` and `test` gates with every existing test unchanged and
  green, then commit the green story (FR-001 to FR-009 and FR-016; US1-S1, US1-S2,
  US1-S3, US1-S4, US1-S5, US1-S6, US1-S7, US1-S8, US1-S9, US1-S10, US1-S11, US1-S12,
  US1-S13, US1-S14, US1-S15).

## Phase 2: User Story 2 - A start whose acknowledgement is spoiled is checked, not assumed failed (Priority: P1)

### Tests first

- [ ] T008 [US2] Create `crates/scope-node/tests/mount_start.rs` with its own copy of
  the node rig from `mount_step.rs`, which is an existing test now and is not edited,
  and tests for US2-S1, US2-S2, US2-S3, US2-S4, US2-S5, US2-S6, US2-S7, US2-S8, US2-S9
  and US2-S10. They read `unacknowledged_starts` and `halted_moves`, which T009 adds;
  with the fields, their fold (T010) and `log_motion`'s count in place and `start` not
  yet written, each fails on today's protocol, which sends no status read after `:J`,
  but US2-S8: a refusal is handled as today, so that test passes before `start` is
  written and after.

### Implementation

- [ ] T009 [US2] In `crates/scope-node/src/mount/skywatcher/protocol.rs`: `start`, used
  by `rate` and `goto` in place of `motion` for `:J`; `Protocol::line` and the two new
  fields of `LineHealth`; `log_motion` with the command's name and axis, its count and
  its line; `SkyWatcher::line_health` returning all four fields (plan.md § The start).
- [ ] T010 [US2] In `crates/scope-node/src/mount/skywatcher/mount_link.rs`, sum the two
  new counts in the fold (plan.md § Line health).
- [ ] T011 [US2] Add the paragraph on the start to `## The mount link` in
  `crates/scope-node/README.md`, after US1's (FR-014; plan.md § README).
- [ ] T012 [US2] Run the `lint` and `test` gates with every existing test unchanged and
  green, `mount_step.rs` and `mount_garble.rs` among them, then commit the green story
  (FR-010 to FR-015; US2-S1, US2-S2, US2-S3, US2-S4, US2-S5, US2-S6, US2-S7, US2-S8,
  US2-S9, US2-S10).

## Phase 3: User Story 3 - Track and the console's own calls over the real node (Priority: P2)

### Tests

- [ ] T013 [US3] Create `crates/finder/tests/motion/tracking/mount_line.rs` and add
  `mod mount_line;` after `mod min_alt;` in
  `crates/finder/tests/motion/tracking/mod.rs`. Build the tracker rig as spec.md § User
  Scenarios defines it, and write the tests for US3-S1, US3-S2, US3-S3 and US3-S4.
  US3-S3's 30 s start 2,250 ms after its delay is armed, off the tracker's refresh
  instants, and its counts are read through `MountLink::state` (spec.md § User
  Scenarios, Track on M42).
  They pass on arrival, because US1 and US2 have landed, and this story writes no
  source file. On `main` before those stories the probe saw each fail: US3-S1 and
  US3-S2 with `:K1` and `:K2` on the wire and up to 25 counts lost, US3-S3 with no `:J`
  in the 30 s and both counts unchanged, US3-S4 with the slewing axis read at rest.
- [ ] T014 [US3] Run the `lint` and `test` gates with every existing test unchanged and
  green, then commit the green story (US3-S1, US3-S2, US3-S3, US3-S4).

## Execution Order

The stories land in this order:

```text
US1 → US2 → US3
```

- **US2** starts from a `main` that holds US1's transport, `Line` and `LineHealth`.
- **US3** starts from a `main` that holds both.
- **`[P]`** marks nothing here: every task within a story shares a file with another,
  or follows its tests.
