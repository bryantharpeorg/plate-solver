---
description: "Task list for 044-node-stops-mount-before-exit: The Node Stops the Mount Before It Exits"
---

# Tasks: The Node Stops the Mount Before It Exits

**Prerequisites** (plan.md § Prerequisites):
- every spec in `depends_on_landed` has landed on `main`: met at 7164e87f;
- the owner decisions in spec.md are settled: met on 2026-11-21, the recommended answers;
- the factory's gate boundary gives a test a pseudo-terminal, for US2: met, by a run in a
  sandbox with its `/dev`;
- every story has been probe-built from this text: met, probe round 1 on 2026-11-21;
- no operator-staged file is needed.

**Rules for every story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml`,
  `CODEOWNERS`, `ergane.yaml` or `.specify/`. If an existing test fails, stop and
  escalate with its exact output. A test file an earlier story of this spec created is
  an existing test.
- Write only the files your story's tasks name; they are its `writes` in plan.md
  § Refinement Record. A story that needs another file escalates.
- `crates/finder`, `crates/simulator`, `crates/scope-link`, `proto/`, `Cargo.lock`, and
  the node's `service.rs`, `mount/mod.rs`, `mount/watchdog.rs`, `mount/skywatcher/` and
  `imaging/` are never touched.
- Copy the README paragraph, the unit line and the two rows of
  `docs/voice/03-safety.md` from plan.md § Interfaces; no test reads them.
- No test moves real hardware, and no test signals a process it did not start. Run no
  tool the gates don't run, and do not set `CARGO_TARGET_DIR`.

## Phase 1: User Story 1 - On SIGTERM or SIGINT the node stops the mount before it exits (Priority: P1)

### Tests first

- [ ] T001 [US1] Create `crates/scope-node/tests/stop_before_exit.rs` with the node rig,
  the powered-off rig and the stand-in mount as spec.md § User Scenarios defines them,
  and one test for each of US1-S1, US1-S2, US1-S3, US1-S4, US1-S5, US1-S6, US1-S7 and
  US1-S8, under `#[tokio::test(start_paused = true)]`. They call
  `NodeService::shut_down` and name `Shutdown`, which T004 adds; until then the file
  does not build, as on the probe.
- [ ] T002 [US1] Create `crates/scope-node/tests/exit_on_signal.rs` with the binary's
  helpers (start it, send a signal with the `kill` program, await its exit and read its
  standard error) and one test for each of US1-S9 and US1-S10, under `#[tokio::test]`.
  On today's binary both fail: the node dies by the signal, with no status and no line,
  as the probe saw.

### Implementation

- [ ] T003 [US1] In `crates/scope-node/src/mount/queue.rs`, add `closed` to `State`, the
  refusal in `submit` and `MountQueue::close` (FR-003; plan.md § Interfaces). No other
  line of the file changes.
- [ ] T004 [US1] Create `crates/scope-node/src/shutdown.rs` with `STOP_BOUND`,
  `Shutdown` and its `Display`, and `NodeService::shut_down`, and add
  `pub mod shutdown;` to `crates/scope-node/src/lib.rs` (FR-004, FR-005, FR-007).
- [ ] T005 [US1] Add `signal` to tokio's features in `crates/scope-node/Cargo.toml`
  (FR-010), and in `crates/scope-node/src/main.rs` take the two signals at the start of
  `serve`, serve with `serve_with_shutdown`, and on the first signal run the shutdown,
  write the line and exit (FR-001, FR-002, FR-006, FR-008, FR-009; plan.md
  § Interfaces). `Cargo.lock` must not change.
- [ ] T006 [US1] In `crates/scope-node/README.md`, add `TimeoutStopSec=10` to the unit and
  the paragraph after the `dialout` sentence (FR-011), and in `docs/voice/03-safety.md`
  split the row at line 177 into the two rows (FR-012), both from plan.md § Interfaces.
- [ ] T007 [US1] Run the `lint` and `test` gates with every existing test unchanged and
  green, then commit the green story (FR-001 to FR-013; US1-S1, US1-S2, US1-S3, US1-S4,
  US1-S5, US1-S6, US1-S7, US1-S8, US1-S9, US1-S10).

## Phase 2: User Story 2 - The built node stops a moving mount on a signal (Priority: P2)

### Tests

- [ ] T008 [US2] Add the rustix line under `[dev-dependencies]` in
  `crates/scope-node/Cargo.toml` (FR-014; plan.md § Interfaces). `Cargo.lock` must not
  change.
- [ ] T009 [US2] Create `crates/scope-node/tests/exit_stops_the_mount.rs` with its own
  copy of the binary's helpers, the terminal rig, the board's word on an axis and
  moving, as spec.md § User Scenarios defines them, and one test for each of US2-S1,
  US2-S2, US2-S3 and US2-S4, under `#[tokio::test]`. They pass on arrival with US1
  merged; on the node's code before US1 all four fail, as the probe saw.
- [ ] T010 [US2] Run the `lint` and `test` gates with every existing test unchanged and
  green, US1's two test files among them, then commit the green story (FR-014; US2-S1,
  US2-S2, US2-S3, US2-S4).

## Execution Order

```text
US1 -> US2
```

- **US1** starts from a `main` that holds every spec of `depends_on_landed`.
- **US2** starts once US1 has merged: it tests US1's binary, and both write
  `crates/scope-node/Cargo.toml`.
- **`[P]`** marks nothing here: each story's implementation follows its tests.
