# Implementation Plan: The Mount Link Survives a Power Cycle

**Branch**: `024-mount-link-recovery` | **Date**: 2026-11-14 | **Spec**: [spec.md](spec.md)

This plan records every design decision for the feature. A story that would change one
escalates rather than re-deciding it.

## Summary

Put a supervisor, `MountLink`, between the node's service and the Sky-Watcher board. It
owns the device path, connects in the background and again after the link is lost,
reads the board's "initialisation done" bit and the counts on every poll, and runs the
existing connect sequence again on a reset. The service asks it whether it is connected
before reaching the board. The node then reports the reset count on every `AxisState`
and the board's identity on `DescribeMountReply`, and the console refuses GoTo and Track
after a reset until a new sync. A last story retries wrong-length replies to safe reads
and keeps their bytes.

## Technical Context

**Language/Version**: Rust (MSRV 1.96).

**Primary Dependencies**: nothing new.

**Storage**: none. The reset count lives for the node process; the console's "sync
needed" flag lives for the console process.

**Testing**:
- US1: `crates/scope-node/tests/mount_link.rs` (new), over the emulator and the built binary.
- US2: `crates/scope-node/tests/mount_reset.rs` (new).
- US3: `crates/finder/tests/mount_identity.rs` (new), in-process console over a memory
  transport, plus a `describe_mount` check on the node's service.
- US4: `crates/scope-node/tests/mount_garble.rs` (new).
- No Playwright or Vitest test: the page is unchanged.

**Target Platform**: `scope-node` on the Pi; `finder` on any desk machine.

**Constraints**:
- `SkyWatcher::connect` and its `Mount` methods keep their signatures; existing tests in
  `crates/scope-node/tests/skywatcher.rs` and `crates/scope-node/tests/skywatcher_stop.rs`
  build it directly.
- `EmulatorConfig` gains no field: tests build it with struct literals.
- The `Error` enum of the Sky-Watcher module gains no variant.
- Proto changes are additions only.

**Scale/Scope**: one new module, four trait or handle additions, two proto fields, one
console refusal.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Nothing touches the solver or `crates/math-core`. |
| II. Never weaken a gate | No existing test, golden, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS` or `ergane.yaml` is edited. New tests go in new files. |
| III. Tests first, independent expectations | The oracles are the emulator's own board state (`auxiliary_enabled`, positions, the exchanges it recorded) and fixed numbers in the scenarios, never the link's own arithmetic. |
| IV. Critical paths | No `unsafe`, no dependency change, no database-format change. The proto change adds two fields and one message field number; nothing is renumbered. |
| V. Standalone Rust, honest licensing | Unaffected. |

## Project Structure

```text
crates/scope-link/proto/scope_node/v1/scope_node.proto   # US2: AxisState.mount_resets; US3: DescribeMountReply.firmware, .board
crates/scope-node/
├── README.md                                   # US1: starting without the mount, resets, log lines
├── src/main.rs                                 # US1: MountLink instead of a one-shot connect
├── src/capture.rs                              # US1: no counts while disconnected
├── src/service.rs                              # US1: connected checks; US2: mount_resets; US3: identity
├── src/mount/mod.rs                            # US1: connected, resets, identity defaults; MountIdentity
├── src/mount/skywatcher/mod.rs                 # US1: pub mod mount_link
├── src/mount/skywatcher/mount_link.rs          # US1: new; US2: resets; US4: health
├── src/mount/skywatcher/emulator.rs            # US1: reopen, power, set_position, inject_reply
├── src/mount/skywatcher/protocol.rs            # US2: the init bit; US4: retried reads, goto path
├── tests/mount_link.rs                         # US1: new
├── tests/mount_reset.rs                        # US2: new
└── tests/mount_garble.rs                       # US4: new
crates/finder/
├── src/live.rs                                 # US3: identity and resets from the node
├── src/session.rs                              # US3: MountView fields, sync-needed flag
├── src/goto.rs                                 # US3: refusal, abort on a reset
├── src/tracking.rs                             # US3: refusal, stop on a reset
└── tests/mount_identity.rs                     # US3: new
```

## Interfaces

### The `Mount` trait (`crates/scope-node/src/mount/mod.rs`, US1)

Three methods with defaults, so `SimMount`, `SkyWatcher` and every test mount compile
unchanged:

```rust
/// The board's own name for itself, when the adapter can read it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MountIdentity {
    /// Version as major.minor with two minor digits, e.g. "3.40".
    pub firmware: String,
    /// Board code in hex, e.g. "0x90".
    pub board: String,
}

pub trait Mount: Send + Sync {
    // ... existing methods unchanged ...
    /// False while the adapter has no working link to the board.
    fn connected(&self) -> bool { true }
    /// Resets seen since the node started (spec FR-006).
    fn resets(&self) -> u64 { 0 }
    /// Firmware and board, once known.
    fn identity(&self) -> Option<MountIdentity> { None }
}
```

### `MountLink` (`crates/scope-node/src/mount/skywatcher/mount_link.rs`, new)

```rust
/// Opens a fresh byte stream to the board, e.g. the serial device.
pub type Opener<S> = Box<dyn Fn() -> std::io::Result<S> + Send + Sync>;

pub struct MountLink<S: BaudStream + 'static> { /* Arc'd state behind one tokio Mutex */ }

impl<S: BaudStream + 'static> MountLink<S> {
    /// Returns at once and connects in a background task, retrying every 2 s.
    pub fn start(open: Opener<S>, clock: Arc<dyn NodeClock>) -> Arc<Self>;
    /// US4: invalid-reply count, last invalid bytes, GoTo path per axis.
    pub fn health(&self) -> MountHealth;
}
impl<S: BaudStream + 'static> Mount for MountLink<S> { /* delegates to the SkyWatcher */ }
```

- **Connecting.** The task calls `open()`, then `SkyWatcher::connect`. On success it logs
  FR-003's line and stores the `SkyWatcher`; on failure it logs `mount not connected:
  <error>` when the error text differs from the last one, and sleeps 2 s.
- **Connected.** `axes`, `state`, `goto`, `rate`, `stop_all` and `stop_confirmed`
  delegate. `axes` before the first connect returns counts per revolution 0, which
  `describe_mount` never sends because it checks `connected()` first.
- **Lost.** Three consecutive failed `state` reads, or any `Io` error, drop the stored
  `SkyWatcher` and restart the connect task. `state` while not connected returns the
  last known state with `moving: true` (the node's existing rule,
  `crates/scope-node/src/mount/skywatcher/protocol.rs:396`). `goto` and `rate` while not
  connected send nothing and log `mount not connected: move dropped`.
- **US2, resets.** After each successful `state` read the link checks the board's init
  bit (`SkyWatcher::initialised`, below) and the jump rule of FR-006 against its
  previous reading of that axis. A reset holds the lock, re-runs
  `SkyWatcher::connect` on a fresh `open()` stream, increments the count and logs
  `mount reset (<cause>): re-initialised <firmware> at <baud> baud`, where cause is
  `not initialised`, `link lost` or `counts jumped on <axis>`. Any reconnect after the
  first counts as `link lost`.
- **`identity`** comes from `report().firmware`: `format!("{}.{:02}", major, minor)` and
  `format!("0x{:02X}", board)`.

### `SkyWatcher` additions (`crates/scope-node/src/mount/skywatcher/protocol.rs`)

- **US2:** `Status` gains `initialised: bool` from the third digit's lowest bit
  (`crates/scope-node/src/mount/skywatcher/protocol.rs:150`). `Protocol` keeps the last
  value per axis, and `SkyWatcher` gains
  `pub async fn initialised(&self, axis: Axis) -> Option<bool>` returning it (none
  before the first read).
- **US4:** a private `read(command, axis)` that sends a safe read and parses it, and on
  `InvalidReply` from a length or hex check sends it again, up to five transmissions in
  all. `status` and `position` use it. Each invalid reply's bytes go to a counter and a
  `last_invalid` string with `\r` written as `\\r`, and to the log as
  `mount reply invalid for :<command><axis>: <bytes>`. `goto` records `GotoPath::Absolute`
  or `GotoPath::Relative` per axis and logs `mount goto path on axis <n>: :S` or
  `:H/:M` the first time. `SkyWatcher` gains `pub async fn health(&self) -> MountHealth`.

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GotoPath { Absolute, Relative }

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MountHealth {
    pub invalid_replies: u64,
    pub last_invalid: Option<String>,
    /// Azimuth first.
    pub goto_path: [Option<GotoPath>; 2],
}
```

### The emulator handle (`crates/scope-node/src/mount/skywatcher/emulator.rs`, US1)

```rust
impl EmulatorHandle {
    /// A new stream on the same board, as reopening the serial device would give.
    pub fn reopen(&self, clock: Arc<dyn NodeClock>) -> Emulator;
    /// The board stops answering; commands are recorded with no reply.
    pub fn power_off(&self);
    /// The board answers again as at power-on: positions 0, motors at rest,
    /// not initialised, auxiliary encoders as configured.
    pub fn power_on(&self);
    /// `power_off` then `power_on`.
    pub fn power_cycle(&self);
    /// Moves an axis's counts, as a hand push with the board's encoders on would.
    pub fn set_position(&self, axis: u8, counts: i32);
    /// The next `count` replies to `command` on `axis` are `reply` exactly.
    pub fn inject_reply(&self, command: u8, axis: u8, reply: &str, count: usize);
}
```

`inject_reply` and the power methods are test tooling in the emulator's source, not
tests; no existing test calls them.

### The service (`crates/scope-node/src/service.rs`)

- **US1:** `connected_mount` and `moving_mount` fail with
  `Status::failed_precondition("mount not connected")` when `mount().connected()` is
  false. `describe_mount` returns no axes then. `stop_all` returns
  `Status::unavailable("the mount did not confirm the stop")` then, without calling the
  link.
- **US2:** `AxisState.mount_resets` is `mount().resets()`, read after the axes, as
  `watchdog_stops` is.
- **US3:** `DescribeMountReply.firmware` and `.board` from `mount().identity()`, empty
  strings when none.

### The capture (`crates/scope-node/src/capture.rs`, US1)

`positions` returns none when `mount.connected()` is false
(`crates/scope-node/src/capture.rs:45`).

### The binary (`crates/scope-node/src/main.rs`, US1)

`serve` builds `MountLink::start(Box::new(move || Serial::open(&device)), clock.clone())`
instead of awaiting `SkyWatcher::connect`
(`crates/scope-node/src/main.rs:82`), and passes it as the `Arc<dyn Mount>`.
`--probe-mount` is unchanged.

### The proto (`crates/scope-link/proto/scope_node/v1/scope_node.proto`)

```proto
message AxisState {
  // ... fields 1 to 4 unchanged ...
  // Resets of the motor board the node has seen since it started (US2).
  uint64 mount_resets = 5;
}
message DescribeMountReply {
  repeated AxisInfo axes = 1;
  // Firmware as major.minor and board code, empty when unknown (US3).
  string firmware = 2;
  string board = 3;
}
```

### The console (`crates/finder`, US3)

- **`live.rs`**: the mount poll (`crates/finder/src/live.rs:296`) passes the reply's
  `firmware` and `board` (none when empty) and the azimuth reply's `mount_resets` to
  `Session::set_mount`.
- **`session.rs`**: `set_mount` takes a `MountReading { counts, connected, moving,
  firmware, board, resets }` in place of its three arguments. `MountView` gains
  `#[serde(skip_serializing_if = "is_zero")] resets: u64`. The session keeps the last
  seen count and a `sync_needed` flag, set when the count rises above the last seen
  value and cleared by `record_sync` when it records a sync. `Session::sync_needed()`
  reads it.
- **`goto.rs`**: a GoTo start refuses with `GotoRefusal` text
  `The mount was power-cycled: sync again first` when `sync_needed()`, mapped to `409`
  as the existing refusals are. The running GoTo reads `mount_resets` beside
  `watchdog_stops` (`crates/finder/src/goto.rs:930`) and ends `failed` with
  `mount power-cycled during the move`, after `StopAll`.
- **`tracking.rs`**: Track's start refuses with the same words when `sync_needed()`; the
  tracking loop stops when the count changes.
- MCP tools that start GoTo or Track call the same functions, so they refuse with the
  same words without their own change.

### `crates/scope-node/README.md` (US1)

A `## The mount link` section: the node starts without the mount and connects when it
answers; a power cycle is recognised, the board is initialised again, and the console
asks for a sync; the log lines of FR-003, FR-007 and FR-017; and that a console restarted
after a power cycle needs a sync before GoTo.

## Shapes that existing tests pin

These are not changed by any story:
- **`SkyWatcher::connect`'s exchanges.** `baud_discovery_initialization_and_auxiliary_encoders`
  and `probe_order_and_report` in `crates/scope-node/tests/skywatcher.rs` read them.
  The link calls `connect` as it is; US2 adds no command to it.
- **`state` on a failed read** returns the last state with `moving: true`
  (`position_wrap_zero_rate_and_lost_telemetry` in `crates/scope-node/tests/skywatcher.rs`).
  US4's retries happen before that rule, and five bad replies still reach it (US4-S2).
- **A motion start is sent once** (`transport_timeouts_and_replay_safety`). US4 retries
  only reads.
- **`EmulatorConfig`'s fields** and `Emulator::new`'s signature.
- **`GET /api/session` with no reset.** `resets` is skipped at 0 and `firmware` and
  `board` stay null with nodes that report none, so existing bodies are unchanged.
- **A node with no `[mount]`** behaves exactly as before: no link, no axes, Stop answers
  OK (`crates/scope-node/tests/mount_config.rs`).

## Decisions and the alternatives rejected

| Decision | Rejected | Why |
|---|---|---|
| A supervisor around `SkyWatcher` | Reconnect inside `SkyWatcher` | Its tests build it directly over one stream; a supervisor leaves it as tested. |
| Retry the open every 2 s | Exit and let systemd restart | 44 restarts lost the camera each time, and a restart can't keep a reset count. |
| Every reconnect counts as a reset | Only reconnects that find the bit cleared | A reconnect runs `:F3` before anything can read the bit, and some firmware may not set it; one extra sync is cheap. |
| Jump rule at 1.5 × top rate + 1,000 counts | Any change while at rest | Polls are not instants, and the board may still be finishing a stop. |
| Refuse GoTo and Track until a sync | Re-zero the model from the new counts | The counts after power-on are an unknown offset from the old ones; only a sync measures it. |
| A session flag, not a persisted one | Store the count with each sync | It changes the session file's format; a console restart after a cycle is rare and the README covers it. |
| Retry reads in `protocol.rs` | Retry in the transport | The transport can't tell a short status from a valid empty acknowledgement. |
| Keep bytes in `health()` | Only log them | A test can read a value; it can't read the node's standard error in process. |

## Verification (every story, before proposing)

- **The factory gates** (`ergane.yaml` at c1ed295):
  - `lint` = `cargo fmt --all --check`,
    `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`,
    `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features --locked`
    and `RUSTDOCFLAGS="-D warnings" cargo doc -p finder --no-deps`;
  - `test` = `cargo test --workspace --all-targets --all-features --locked`,
    `cargo test --workspace --locked --doc`, and then, in `crates/finder/web`,
    `npm ci --no-audit --no-fund`, `npm run typecheck`, `npm run format:check`,
    `npm test`, `npm run build`, `git status --porcelain -- dist | (! grep .)` and
    `CI=1 npm run e2e`.

  Every story runs both, and the judge sees their results.
- **CI in the merge queue** runs every `ci.yml` job, including the build on Rust 1.96.
  Coverage runs only by hand (constitution 1.1.0).

## Prerequisites before this spec is flipped to `ready`

1. **The earlier specs have landed:** `013-live-finder`, `014-goto`, `015-tracking` and
   `017-console-stop-confirm`. Met: 017's last story landed at c1ed295.
2. **`main` is still as read.** Every citation here is at c1ed295. If `main` has moved in
   these files (020-live-chart-calm touches `crates/finder/web` only), re-read them and
   re-run the validator.
3. **The owner approvals are given** (spec § Owner approvals). Flipping to `ready`
   records them.
4. **No operator-staged files are needed.**

## Plan traps

Lessons from building 012 to 019 on this factory, each with its mechanism and the check
that keeps it from recurring. They bind every story.

1. **Gate evidence or nothing.** The judge credits only gate results and the story's
   diff. *Check:* every Then clause names a committed test that the `test` gate runs.
2. **A named command is read literally.** *Check:* commands are copied from
   § Verification.
3. **One evidence budget.** *Check:* each story keeps its diff under 64 KiB, measured
   with `git diff <base> -- . ':(exclude)crates/finder/web/dist' | wc -c`.
4. **The sandbox has only the gates' tools.** *Check:* no task runs a tool outside them.
5. **A spec fixed mid-epic needs a relaunch.** The judge grades the spec as dispatched.
   *Check:* no builder edits `specs/024-mount-link-recovery/` or `.specify/`.
6. **An oracle copied from the code proves nothing.** *Check:* tests read the emulator's
   board state and recorded exchanges, and never call the link's jump rule or parser to
   get an expected value.
7. **Paused time and background tasks.** The link connects in a spawned task; with
   Tokio's clock paused, a test must sleep for the task to run. *Check:* every scenario
   says how long to wait, and the tests use `start_paused = true`.
8. **A reset that the test caused itself.** A GoTo moves counts quickly; a jump rule
   that is too tight would count it. *Check:* US2-S4 runs a real GoTo and asserts no
   reset.
9. **Struct literals break on a new field.** *Check:* no field is added to
   `EmulatorConfig`, `AxisState` (the node's Rust struct), `Error` or any type a test
   builds by literal; proto messages are built only in `crates/scope-node/src/service.rs`.
10. **Each worktree builds on its own.** *Check:* no story sets `CARGO_TARGET_DIR`.

## Risks

| Risk | Mitigation |
|---|---|
| The real board (firmware 2.09) doesn't set the init bit. | A lost link counts as a reset whatever the bit says; the jump rule catches hand pushes. |
| A reset during a long slew isn't seen until the next poll. | The motors lose power with the board; the console's poll runs every 250 ms. |
| Reconnecting while a move is queued sends a stale move. | `goto` and `rate` send nothing while not connected (FR-008), and the console aborts on a reset (FR-015). |
| The retry hides a real fault. | Every invalid reply is counted and logged with its bytes (FR-017). |

## Refinement Record

Drafted on 2026-11-14 by the plate-solver operator against `main` at c1ed295, from the
13-14 November field note and a read of the node's mount module, the service and the
console's mount poll. No code was probed: the interfaces were read from the cited
lines. Writes come from the task text. Shared writes name the merge edge that orders
them, and the 013, 014 and 017 stories that last wrote the same file on `main`. Nothing
in this record is an owner approval: those are in spec.md, and flipping to `ready` gives
them.

```yaml
version: 1
reviewed_revision: c1ed295734253c7d6121052b0a14beb2ee64bc17
stories:
  US1:
    tests:
      - path: crates/scope-node/tests/mount_link.rs
        status: new
        disposition: add
        assertion: "a link to a powered-off board stays disconnected and sends no move; after power-on it connects, sends F3 and both W commands, and the board's encoders read off"
        decision: "spec.md:US1-S1"
      - path: crates/scope-node/tests/mount_link.rs
        status: new
        disposition: add
        assertion: "the service over a disconnected link describes no axes, refuses axis_state and goto_axis with mount not connected, answers stop_all UNAVAILABLE, and frames carry no counts"
        decision: "spec.md:US1-S3"
      - path: crates/scope-node/tests/mount_link.rs
        status: new
        disposition: add
        assertion: "the binary with a missing mount device keeps running and logs mount not connected"
        decision: "spec.md:US1-S5"
      - path: crates/scope-node/tests/skywatcher.rs
        status: existing
        disposition: preserve
        assertion: "SkyWatcher::connect's exchanges, probe order and lost-telemetry rule are unchanged"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/scope-node/tests/mount_config.rs
        status: existing
        disposition: preserve
        assertion: "a node with no mount configured behaves as before"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "SkyWatcher::connect and its Mount methods keep their signatures and exchanges (spec.md FR-004)"
      - "a node without [mount] is unchanged"
      - "--probe-mount is unchanged"
    checks:
      development:
        - "cargo test -p scope-node --test mount_link --locked"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs, and a node rebuilt on the Pi"
    writes:
      - crates/scope-node/README.md
      - crates/scope-node/src/capture.rs
      - crates/scope-node/src/main.rs
      - crates/scope-node/src/mount/mod.rs
      - crates/scope-node/src/mount/skywatcher/emulator.rs
      - crates/scope-node/src/mount/skywatcher/mod.rs
      - crates/scope-node/src/mount/skywatcher/mount_link.rs
      - crates/scope-node/src/service.rs
      - crates/scope-node/tests/mount_link.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/scope-node/src/mount/skywatcher/mount_link.rs
        other_spec: 024-mount-link-recovery
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and adds reset detection to the link"
      - path: crates/scope-node/src/service.rs
        other_spec: 024-mount-link-recovery
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and adds mount_resets to axis_state"
      - path: crates/scope-node/README.md
        other_spec: 013-live-finder
        other_story: US2
        disposition: merge_order
        reason: "024 depends_on_landed 013, whose US2 last changed the node's README"
      - path: crates/scope-node/src/capture.rs
        other_spec: 014-goto
        other_story: US2
        disposition: merge_order
        reason: "024 depends_on_landed 014, whose US2 last changed capture.rs"
      - path: crates/scope-node/src/main.rs
        other_spec: 014-goto
        other_story: US3
        disposition: merge_order
        reason: "024 depends_on_landed 014, whose US3 last changed main.rs"
      - path: crates/scope-node/src/mount/mod.rs
        other_spec: 017-console-stop-confirm
        other_story: US3
        disposition: merge_order
        reason: "024 depends_on_landed 017, whose US3 last changed the Mount trait"
      - path: crates/scope-node/src/mount/skywatcher/emulator.rs
        other_spec: 014-goto
        other_story: US3
        disposition: merge_order
        reason: "024 depends_on_landed 014, whose US3 last changed the emulator"
      - path: crates/scope-node/src/mount/skywatcher/mod.rs
        other_spec: 014-goto
        other_story: US3
        disposition: merge_order
        reason: "024 depends_on_landed 014, whose US3 last changed the module"
      - path: crates/scope-node/src/service.rs
        other_spec: 017-console-stop-confirm
        other_story: US4
        disposition: merge_order
        reason: "024 depends_on_landed 017, whose US4 last changed service.rs"
    open_questions: []
  US2:
    tests:
      - path: crates/scope-node/tests/mount_reset.rs
        status: new
        disposition: add
        assertion: "a power cycle, a lost and recovered link, and a counts jump each re-run F3 and W, count one reset and leave the encoders off"
        decision: "spec.md:US2-S1"
      - path: crates/scope-node/tests/mount_reset.rs
        status: new
        disposition: add
        assertion: "a real GoTo within the top rate counts no reset"
        decision: "spec.md:US2-S4"
      - path: crates/scope-node/tests/mount_reset.rs
        status: new
        disposition: add
        assertion: "axis_state carries mount_resets, and a disconnected link sends no move"
        decision: "spec.md:US2-S5"
    preserved_behavior:
      - "the first connect after start is not a reset"
      - "watchdog_stops is unchanged"
    checks:
      development:
        - "cargo test -p scope-node --test mount_reset --locked"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs, and a power cycle at rest on the real board"
    writes:
      - crates/scope-link/proto/scope_node/v1/scope_node.proto
      - crates/scope-node/src/mount/skywatcher/mount_link.rs
      - crates/scope-node/src/mount/skywatcher/protocol.rs
      - crates/scope-node/src/service.rs
      - crates/scope-node/tests/mount_reset.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/scope-link/proto/scope_node/v1/scope_node.proto
        other_spec: 024-mount-link-recovery
        other_story: US3
        disposition: merge_order
        reason: "US3 depends_on_merged US2, and adds DescribeMountReply fields"
      - path: crates/scope-node/src/service.rs
        other_spec: 024-mount-link-recovery
        other_story: US3
        disposition: merge_order
        reason: "US3 depends_on_merged US2, and adds identity to describe_mount"
      - path: crates/scope-node/src/mount/skywatcher/mount_link.rs
        other_spec: 024-mount-link-recovery
        other_story: US4
        disposition: merge_order
        reason: "US4 depends_on_merged US2, and adds health to the link"
      - path: crates/scope-node/src/mount/skywatcher/protocol.rs
        other_spec: 024-mount-link-recovery
        other_story: US4
        disposition: merge_order
        reason: "US4 depends_on_merged US2, and changes the reads US2 watches"
      - path: crates/scope-link/proto/scope_node/v1/scope_node.proto
        other_spec: 017-console-stop-confirm
        other_story: US4
        disposition: merge_order
        reason: "024 depends_on_landed 017, whose US4 last changed the proto"
      - path: crates/scope-node/src/mount/skywatcher/protocol.rs
        other_spec: 017-console-stop-confirm
        other_story: US3
        disposition: merge_order
        reason: "024 depends_on_landed 017, whose US3 last changed protocol.rs"
    open_questions: []
  US3:
    tests:
      - path: crates/finder/tests/mount_identity.rs
        status: new
        disposition: add
        assertion: "describe_mount carries firmware 3.40 and board 0x90; the session shows them and omits resets at 0"
        decision: "spec.md:US3-S1"
      - path: crates/finder/tests/mount_identity.rs
        status: new
        disposition: add
        assertion: "after a reset GoTo and Track answer 409 with the power-cycled words and no GotoAxis reaches the node, until a new sync"
        decision: "spec.md:US3-S3"
      - path: crates/finder/tests/mount_identity.rs
        status: new
        disposition: add
        assertion: "a running GoTo ends failed with power-cycled and sends StopAll; running tracking stops"
        decision: "spec.md:US3-S5"
      - path: crates/finder/tests/goto_refusal_api.rs
        status: existing
        disposition: preserve
        assertion: "GoTo's existing refusals and their words are unchanged"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/watchdog_goto.rs
        status: existing
        disposition: preserve
        assertion: "a GoTo still aborts when the node's watchdog stops the mount"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "GET /api/session with no reset and a node with no identity is byte-for-byte unchanged"
      - "the 014 model's epochs are unchanged"
    checks:
      development:
        - "cargo test -p finder --test mount_identity --locked"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - crates/finder/src/goto.rs
      - crates/finder/src/live.rs
      - crates/finder/src/session.rs
      - crates/finder/src/tracking.rs
      - crates/finder/tests/mount_identity.rs
      - crates/scope-link/proto/scope_node/v1/scope_node.proto
      - crates/scope-node/src/service.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/goto.rs
        other_spec: 017-console-stop-confirm
        other_story: US4
        disposition: merge_order
        reason: "024 depends_on_landed 017, whose US4 last changed goto.rs"
      - path: crates/finder/src/live.rs
        other_spec: 015-tracking
        other_story: US7
        disposition: merge_order
        reason: "024 depends_on_landed 015, whose US7 last changed live.rs"
      - path: crates/finder/src/session.rs
        other_spec: 017-console-stop-confirm
        other_story: US2
        disposition: merge_order
        reason: "024 depends_on_landed 017, whose US2 last changed session.rs"
      - path: crates/finder/src/tracking.rs
        other_spec: 017-console-stop-confirm
        other_story: US2
        disposition: merge_order
        reason: "024 depends_on_landed 017, whose US2 last changed tracking.rs"
    open_questions: []
  US4:
    tests:
      - path: crates/scope-node/tests/mount_garble.rs
        status: new
        disposition: add
        assertion: "a wrong-length status or position reply is retried, counted and kept escaped; five in a row still read as moving"
        decision: "spec.md:US4-S1"
      - path: crates/scope-node/tests/mount_garble.rs
        status: new
        disposition: add
        assertion: "the GoTo path is Relative on a board that refuses absolute moves and Absolute otherwise; a motion start is sent once"
        decision: "spec.md:US4-S4"
      - path: crates/scope-node/tests/skywatcher.rs
        status: existing
        disposition: preserve
        assertion: "transport replay safety and the lost-telemetry rule are unchanged"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "a motion start is never sent twice"
      - "five failed reads still return the last state with moving true"
    checks:
      development:
        - "cargo test -p scope-node --test mount_garble --locked"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs, and a night's node log read for invalid replies"
    writes:
      - crates/scope-node/src/mount/skywatcher/mount_link.rs
      - crates/scope-node/src/mount/skywatcher/protocol.rs
      - crates/scope-node/tests/mount_garble.rs
    no_write_reason: ""
    shared_writes: []
    open_questions: []
```
