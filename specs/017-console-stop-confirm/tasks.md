---
description: "Task list for 017-console-stop-confirm: A Stop the Console Can Confirm"
---

# Tasks: A Stop the Console Can Confirm

**Prerequisites** (plan.md § Prerequisites):
- `014-goto`, `015-tracking` and `016-mcp` have landed on `main`;
- the owner approvals in spec.md are given;
- no operator-staged file is needed.

**Rules for every story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml` or
  `CODEOWNERS`.
- Write only the files your story's tasks name; they are its `writes` in plan.md
  § Refinement Record. A story that needs another file escalates.
- New tests go in new files, or in-module. `crates/finder/tests/stop/support.rs` starts
  with `#![allow(dead_code)]`, as `tests/mcp/support.rs` does. US1 creates it, and US2 may
  add new items to it and may not change or remove any item US1 put there.
- Exact attempt counts and times are asserted only when nothing else is stopping the
  mount (plan.md § Plan traps, trap 8).
- `crates/math-core`, `proto/` and `crates/simulator/src` are never touched.
- No test moves real hardware or calls a real LLM.
- Run no tool the gates don't run: no `cargo deny`, `cargo machete`, Rust 1.96 or
  `ergane`.

## Phase 1: User Story 1 — Stop is repeated until the scope node answers, and the reply says whether it did (Priority: P1)

### Tests first

- [ ] T001 [US1] Create `crates/finder/tests/stop/support.rs` with a simulator `Rig` in
  the shape of `tests/mcp/support.rs`: a `VirtualClock`, a recording mount around
  `SimMount` that logs go-to, rate and stop all with their virtual time, the routes built
  as `tests/tracking_sim.rs` builds them so that the stop service is a `StopGate`, a node
  clock that adds an atomic millisecond offset to `node_now()`, a `hold_state` flag on the
  recording mount that makes `state` wait until cleared, and a console through
  `finder::router`. `StopGate` has modes `Pass`, `Hold(n)` (hold the next `n` calls, then
  pass), `HoldAll` and `Refuse`, and an attempt log of virtual times. A held call awaits
  `std::future::pending()`.
- [ ] T002 [US1] Write failing tests in a new `crates/finder/tests/stop_retry.rs`:
  - one attempt on a passing gate, with the body exactly `{"stopped":true}` (US1-S1);
  - two held then one passed, at 0, 0.25 and 0.5 s (US1-S2);
  - every attempt held: 8 attempts, the reply at 1.95 s ± 0.1 s, the body as JSON equal
    to `{"stopped":true,"acknowledged":false}`, and the axis still moving (US1-S3);
  - every attempt refused: at most 41, none closer than 0.045 s (US1-S4);
  - the request's future dropped after 0.3 s, then the gate passing (US1-S5);
  - heartbeats silent until `LinkStatus.connected` is false (US1-S6);
  - a console with no scope link (US1-S7);
  - a GoTo with a retained tracker, halted within 0.1 s under a held gate (US1-S8).
- [ ] T003 [US1] Add failing tests to `crates/finder/tests/stop_retry.rs`:
  - MCP `stop_all` under a held gate, in `structuredContent`, `content[0]` and the call
    log, and the two tools' descriptions in `tools/list` (US1-S9);
  - a foreign-`Origin` batch naming `stop` and `stop_all`: 403 and one round of 8
    attempts (US1-S10).
- [ ] T004 [US1] Add a failing test to `crates/finder/tests/stop_retry.rs` that posts
  `POST /api/goto` with `hold_state` set, then `POST /api/stop`, and asserts that Stop
  answers within 0.1 s and before the GoTo request does (US1-S11).

### Implementation

- [ ] T005 [US1] Add `STOP_ATTEMPT`, `STOP_PAUSE`, `STOP_WINDOW`, `stop_deadline` and
  `LinkManager::stop_until_acknowledged` to `crates/finder/src/link.rs`, exactly as in
  plan.md § Interfaces.
- [ ] T006 [US1] Make `GoTo::stop` in `crates/finder/src/goto.rs` return the round's
  result, and rewrite `stop` in `crates/finder/src/api.rs` with `tokio::join!` as in
  plan.md § Interfaces, with the `acknowledged` body from FR-004.
- [ ] T007 [US1] Make `StopsCarriedOut` in `crates/finder/src/mcp/door.rs` carry
  `api::stop`'s value, and use it in `stop_result` in `crates/finder/src/mcp/tools.rs`.
  Add the sentence from plan.md § Interfaces to both tools' descriptions.
- [ ] T008 [US1] Describe the reply and the retry in the `POST /api/stop` row of
  `crates/finder/README.md` and in the module documentation of `crates/finder/src/lib.rs`
  and `crates/finder/src/api.rs`.
- [ ] T009 [US1] Run the `lint` and `test` gates with every existing test unchanged and
  green, then commit the green story (FR-001 to FR-006, FR-017; US1-S1 to US1-S11).

## Phase 2: User Story 2 — A motion's own end is repeated the same way, and a rate move never reports done on a lost stop (Priority: P1)

### Tests first

- [ ] T010 [US2] Add a `hold_rate` flag to the recording mount in
  `crates/finder/tests/stop/support.rs`, which makes `rate` never complete.
- [ ] T011 [US2] Write failing tests in a new `crates/finder/tests/stop_motion_ends.rs`:
  - a GoTo halted by silent heartbeats reads `halted` before the third attempt (US2-S1);
  - `POST /api/goto` is refused during that round (US2-S2);
  - a rate move's final stop held twice ends `done` (US2-S3);
  - held throughout, `axis_move` equals the object in US2-S4, never read `done`, and the
    axis is still moving (US2-S4);
  - a Stop during a rate move keeps `the observer stopped it` (US2-S5);
  - a `rate` call that never completes answers `mount did not answer` after two attempts
    (US2-S6);
  - Track with a held-twice Stop (US2-S7);
  - Track with every attempt held, answered within 2.5 s (US2-S8);
  - a `Tracker` never started, at 0, 0.25 and 0.5 s (US2-S9).

### Implementation

- [ ] T012 [US2] In `crates/finder/src/goto.rs`, publish the ending in `run` before its
  final stop and send that stop through `stop_until_acknowledged` with its client, as in
  plan.md § Interfaces.
- [ ] T013 [US2] In `crates/finder/src/mcp/guard.rs`, send both stops through
  `stop_until_acknowledged` with the client the move holds, and set `axis_move` from the
  result and the acknowledgement as in plan.md § Interfaces.
- [ ] T014 [US2] In `crates/finder/src/tracking.rs`, add `Tracker::stop_by`, keep
  `Tracker::stop`'s signature, send both of its stops through `stop_until_acknowledged`,
  and remove `STOP_TIMEOUT`. In `crates/finder/src/session.rs`, compute one
  `stop_deadline()` in `stop_tracking` and pass it to both `stop_by` calls.
- [ ] T015 [US2] Run the `lint` and `test` gates with every existing test unchanged and
  green, then commit the green story (FR-007 to FR-009; US2-S1 to US2-S9).

## Phase 3: User Story 3 — The scope node finishes every stop it starts, and acknowledges only what the motor board confirmed (Priority: P1)

### Tests first

- [ ] T016 [US3] Write failing tests in a new `crates/simulator/tests/stop_finishes.rs`,
  with its own node and a `Mount` wrapper around `SimMount` whose stop sleeps 1 s and
  counts completions:
  - a `StopAll` dropped at 200 ms still finishes and the next `rate_axis` is accepted
    (US3-S1);
  - three overlapping `StopAll` calls all finish (US3-S2);
  - a mount whose `stop_confirmed` is false answers `Unavailable` with
    `the mount did not confirm the stop`; a default mount and a node with no mount answer
    OK (US3-S6).
- [ ] T017 [US3] Write failing tests in a new `crates/scope-node/tests/skywatcher_stop.rs`
  on the emulator: `stop_confirmed` on a healthy board (US3-S3), with
  `silent_stop_axis: Some(1)` (US3-S4), and with `drop_replies` on `K` and `L` for axis 2,
  five of each (US3-S5).

### Implementation

- [ ] T018 [US3] Add the provided method `stop_confirmed` to `Mount` in
  `crates/scope-node/src/mount/mod.rs`. Override it for `SkyWatcher` in
  `crates/scope-node/src/mount/skywatcher/protocol.rs`.
- [ ] T019 [US3] Make `MountQueue::stop_all` in `crates/scope-node/src/mount/queue.rs`
  return the adapter's answer, and run it in a spawned task with the `UNAVAILABLE` answer
  in `ScopeNodeStop::stop_all` in `crates/scope-node/src/service.rs`, as in plan.md
  § Interfaces.
- [ ] T020 [US3] Run the `lint` and `test` gates with every existing test unchanged and
  green, then commit the green story (FR-010, FR-011; US3-S1 to US3-S6).

## Phase 4: User Story 4 — The scope node reports a watchdog firing, and a running GoTo halts (Priority: P1)

### Tests first

- [ ] T021 [US4] Write a failing test in a new `crates/simulator/tests/watchdog_report.rs`
  with `tests/support`'s `Rig`: `watchdog_stops` reads 0, 1 and 2 across two silences,
  and an explicit `StopAll` between them leaves it unchanged (US4-S1).
- [ ] T022 [US4] Write failing tests in a new `crates/finder/tests/watchdog_goto.rs` with
  `tests/stop/support.rs`'s `Rig`:
  - advance only the node's clock by 1.2 s in mid-slew (US4-S2);
  - a watchdog that fired during a real silence doesn't halt a later GoTo, which ends
    `done` within 1′ (US4-S3);
  - armed Track ends with the watchdog's reason (US4-S4).

### Implementation

- [ ] T023 [US4] Add `uint64 watchdog_stops = 4` to `AxisState` in
  `crates/scope-link/proto/scope_node/v1/scope_node.proto`. Add the count to
  `crates/scope-node/src/mount/watchdog.rs`, incremented where `lost` is set, with
  `pub fn stops`. Fill the field in `axis_state` in `crates/scope-node/src/service.rs`,
  after reading the axis.
- [ ] T024 [US4] In `crates/finder/src/goto.rs`, add `WATCHDOG_HALT`, read the baseline
  before the first leg of `motion`, compare in the wait loop before testing `moving`, and
  map that reason to `halted` in `run`, as in plan.md § Interfaces.
- [ ] T025 [US4] Add the watchdog's halt reason to the list of halt reasons in
  `crates/finder/README.md`.
- [ ] T026 [US4] Run the `lint` and `test` gates with every existing test unchanged and
  green, then commit the green story (FR-012, FR-013; US4-S1 to US4-S4).

## Phase 5: User Story 5 — The page says when the mount has not confirmed Stop (Priority: P2)

### Tests first

- [ ] T027 [US5] Write failing tests in a new
  `crates/finder/web/src/components/StopConfirm.test.tsx`, each pressing Stop while
  `mount.moving` is true and then rerendering with the next `mount`:
  - `acknowledged: false` with `{connected: false, moving: false}` (US5-S1);
  - a failed request (US5-S2);
  - a confirmed reply with no live reading (US5-S3);
  - a confirmed reply with `{connected: true, moving: true}` (US5-S4);
  - clearing by a new object at rest; not by the same object; not by
    `{connected: false, moving: false}` (US5-S5);
  - a second press while the first is pending (US5-S6).

### Implementation

- [ ] T028 [US5] Add `StopReply` and `postStop` to `crates/finder/web/src/api.ts`, leaving
  `postMotion` unchanged. Change `crates/finder/web/src/components/StopButton.tsx` as in
  plan.md § Interfaces, with FR-014's and FR-015's messages copied verbatim, leaving
  `mountMayMove` unchanged.
- [ ] T029 [US5] Record both messages and the rule that keeps Stop visible in `DESIGN.md`
  § Stop and in `.impeccable/design.json`.
- [ ] T030 [US5] Run the `lint` and `test` gates with every existing Vitest and Playwright
  test unchanged and green. Commit the rebuilt `crates/finder/web/dist`. Commit the green
  story (FR-014 to FR-016, FR-018; US5-S1 to US5-S7).

## Execution Order

The stories land in this order:

```text
US1 ─┬→ US2 ─┬→ US4
     │       │
US3 ─┼───────┘
     └→ US5
```

- **US2** starts from a `main` that holds US1's retrying function.
- **US3** touches only `crates/scope-node` and its own tests, so it may run beside US1
  and US2.
- **US4** starts from a `main` that holds US1, US2 and US3: it changes `goto.rs` after
  them, `service.rs` after US3 and the README after US1.
- **US5** needs only US1's reply. It touches only `crates/finder/web/`, `DESIGN.md` and
  `.impeccable/design.json`.
- **`[P]`** marks nothing here: every task within a story shares a file with another.
