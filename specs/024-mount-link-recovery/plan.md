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
and the board's identity on `DescribeMountReply`, and the console refuses GoTo, second
sync and Track after a reset until a new sync. A last story retries wrong-length replies
to status and position reads and keeps their bytes.

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
- `SkyWatcher::connect` and its `Mount` methods keep their signatures, and their
  exchanges when every reply is valid; existing tests in
  `crates/scope-node/tests/skywatcher.rs` and `crates/scope-node/tests/skywatcher_stop.rs`
  build it directly.
- `EmulatorConfig` gains no field: tests build it with struct literals.
- The `Error` enum of the Sky-Watcher module gains no variant.
- Proto changes are additions only.

**Scale/Scope**: one new module, four trait or handle additions, three proto fields, one
console refusal.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Nothing touches the solver or `crates/math-core`. |
| II. Never weaken a gate | No existing test, golden, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS` or `ergane.yaml` is edited. New tests go in new files. |
| III. Tests first, independent expectations | The oracles are the emulator's own board state (`auxiliary_enabled`, positions set with `set_position`, the exchanges it recorded) and fixed numbers in the scenarios, never the link's own arithmetic. |
| IV. Critical paths | No `unsafe`, no dependency change, no database-format change. The proto change adds three fields to two messages; nothing is renumbered. |
| V. Standalone Rust, honest licensing | Unaffected. |

## Project Structure

```text
crates/scope-link/proto/scope_node/v1/scope_node.proto   # US2: AxisState.mount_resets; US3: DescribeMountReply.firmware, .board
crates/scope-node/
├── README.md                                   # US1: starting without the mount; US2: resets; US4: invalid replies
├── src/main.rs                                 # US1: MountLink instead of a one-shot connect
├── src/capture.rs                              # US1: no counts while disconnected
├── src/service.rs                              # US1: connected checks; US2: mount_resets; US3: identity
├── src/mount/mod.rs                            # US1: connected, resets, identity defaults; MountIdentity
├── src/mount/skywatcher/mod.rs                 # US1: pub mod mount_link, pub use MountLink, Opener
├── src/mount/skywatcher/mount_link.rs          # US1: new; US2: loss and resets; US4: health
├── src/mount/skywatcher/emulator.rs            # US1: reopen, power, set_position, inject_reply
├── src/mount/skywatcher/protocol.rs            # US2: the init bit; US4: retried reads, goto path
├── tests/mount_link.rs                         # US1: new
├── tests/mount_reset.rs                        # US2: new
└── tests/mount_garble.rs                       # US4: new
crates/finder/
├── src/api.rs                                  # US3: the console clock to the mount poll; docs
├── src/live.rs                                 # US3: identity and resets from the node
├── src/session.rs                              # US3: MountView fields, sync-needed flag, Track's refusal
├── src/goto.rs                                 # US3: GoTo and second-sync refusal, abort on a reset
├── src/tracking.rs                             # US3: stop on a reset
└── tests/mount_identity.rs                     # US3: new
```

## Interfaces

Every story was probe-built from this section on 2026-11-14; where a probe had to differ,
the section now says what compiled and passed. The reviews after the probes added parts
that no probe has compiled yet: in US1, Stop through the queue while disconnected; in
US2, the bit rule's need for a set bit first, releasing `io` before an in-line connect
and waking the connect task when that connect fails; in US3, the frame cutoff for
clearing the refusal and the console clock it needs.

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

Tests import it as `scope_node::mount::MountIdentity`.

### `MountLink` (`crates/scope-node/src/mount/skywatcher/mount_link.rs`, new)

```rust
/// Opens a fresh byte stream to the board, e.g. the serial device.
pub type Opener<S> = Box<dyn Fn() -> std::io::Result<S> + Send + Sync>;

pub struct MountLink<S: BaudStream + 'static> {
    /* a std::sync::Mutex snapshot read by the synchronous methods, and the
       connect task's JoinHandle, aborted when the link is dropped */
}

impl<S: BaudStream + 'static> MountLink<S> {
    /// Returns at once and connects in a background task.
    pub fn start(open: Opener<S>, clock: Arc<dyn NodeClock>) -> Arc<Self>;
    /// US4: invalid-reply count, last invalid bytes, GoTo path per axis.
    pub fn health(&self) -> MountHealth;
}
impl<S: BaudStream + 'static> Mount for MountLink<S> { /* delegates to the SkyWatcher */ }
```

`crates/scope-node/src/mount/skywatcher/mod.rs` gains `pub mod mount_link;` and
`pub use mount_link::{MountLink, Opener};`, so tests import
`scope_node::mount::skywatcher::{MountLink, Opener}`.

- **State (US1).** `connected`, `resets`, `identity`, `axes` and `health` are
  synchronous, so they read a `std::sync::Mutex` snapshot: the stored
  `Arc<SkyWatcher<S>>` and the last state per axis; US2 adds the reset count, the jump
  baselines, the failure count and a pending cause, and US4 the health. The snapshot is
  cloned out and never held across an `await`; the `SkyWatcher`'s own lock still
  serialises the board.
- **Connecting (US1).** The connect task calls `open()`, then `SkyWatcher::connect`. On
  success it logs FR-003's line and then the board's warnings as `mount warning: <text>`,
  and stores the `SkyWatcher`. On failure it logs `mount not connected: <error>` when
  the error text differs from the last one, and waits 2 s. Against a silent board one
  attempt takes 3 s (two bauds, `crates/scope-node/src/mount/skywatcher/protocol.rs:81`,
  each tried five times, `crates/scope-node/src/mount/skywatcher/transport.rs:43`, with
  a 300 ms timeout, `crates/scope-node/src/mount/skywatcher/transport.rs:6`), so
  attempts start 5 s apart and a board that comes on is connected within about 4 s (the
  US1 probe measured 2.9 to 3.8 s). US1's task returns after its first connect.
- **FR-003's line (US1)** is written by the connect task, never by `main.rs`:
  `mount connected: <firmware> at <baud> baud; azimuth <cpr> counts/rev, timer <hz> Hz,
  high-speed ratio <n>, auxiliary encoders, :W sent; altitude ...`, with
  `no auxiliary encoders, no :W` for an axis without them, where `<firmware>` is
  `Firmware`'s display (`3.40 (board 0x90)`). The connect sends `:W` on exactly the axes
  that report auxiliary encoders (`crates/scope-node/src/mount/skywatcher/protocol.rs:333`),
  so the line reads it from the report.
- **Connected (US1).** `axes`, `state`, `goto`, `rate`, `stop_all` and `stop_confirmed`
  delegate. While not connected: `axes` returns
  `AxisInfo { counts_per_revolution: 0, top_rate_deg_per_s: 0.0, can_goto: false }` for
  both axes, which `describe_mount` never sends because it checks `connected()` first;
  `state` returns the last known state with `moving: true` (the node's existing rule,
  `crates/scope-node/src/mount/skywatcher/protocol.rs:396`); `goto` and `rate` send
  nothing and log `mount not connected: move dropped`; `stop_all` sends nothing and
  `stop_confirmed` answers false at once.
- **`identity` (US1)** is none while not connected, and otherwise comes from
  `report().firmware`: `format!("{}.{:02}", major, minor)` and
  `format!("0x{:02X}", board)`.
- **Lost (US2).** Three failed `state` reads in a row, counted across both axes and
  cleared by any good read, drop the stored `SkyWatcher` and wake the connect task. A
  read failed when `SkyWatcher::initialised` (below) is none after it. A read while the
  link has no board, from the capture loop or `axis_state`, is not a failed read: it
  reaches no board and counts toward nothing. From US2 the connect task loops: connect,
  wait on a `tokio::sync::Notify` for a wake, connect again. Wakes use `notify_one`, which
  keeps a wake that comes before the task waits.
- **Resets (US2).** A `tokio::sync::Mutex` (`io`) is held through each `state` read and
  by `goto` and `rate`, so no move is under way on the board when a reset takes it out.
  After each good read the link checks that axis:
  - **the init bit:** a reset when the bit reads clear and an earlier read of that axis
    since the last connect read it set. A board whose bit never reads set never trips
    this rule;
  - **the jump rule:** a reset when the counts moved from the axis's last good reading
    on this board by more than 1.5 × `top_rate_deg_per_s` × elapsed seconds ×
    `counts_per_revolution` / 360 + 1,000, taking both from the board's `axes()` and
    wrapping the difference at 24 bits.

  One read counts at most one reset, and `not initialised` wins over
  `counts jumped on <axis>`. The read that found the reset is returned as read. On a
  reset the link first takes the board out of the snapshot, and the read drops its own
  handle, so `connected()` reads false and `goto` and `rate` drop their moves (FR-008);
  then it releases `io` and runs `SkyWatcher::connect` on a fresh `open()` stream,
  awaited by the read that found the reset. The count goes up when the
  re-initialisation succeeds, with the log line
  `mount reset (<cause>): re-initialised <firmware> at <baud> baud`; if it fails, the
  cause is kept, the link wakes the connect task, and that task's next successful
  connect counts the cause, so one event counts once. A connect after the first with no
  cause kept counts as `link lost`. While the in-line connect runs the link has no
  board, so no read can count toward a loss and the connect task stays waiting: only
  one connect is ever under way.
  Every connect clears the jump baselines, the failure count and the init-bit history.
  Stop does not take `io`: while the link has no board it answers at once (owner
  decision 4). A Stop that took the old board just before still finishes on it.

### `SkyWatcher` additions (`crates/scope-node/src/mount/skywatcher/protocol.rs`)

- **US2:** `Status` gains `initialised: bool` from the third digit's lowest bit
  (`crates/scope-node/src/mount/skywatcher/protocol.rs:150`). `Protocol` records it per
  axis on each full `state` read, and `Mount::state` sets it to none when the read fails
  (`crates/scope-node/src/mount/skywatcher/protocol.rs:394`); the status reads inside
  `goto`, `rate` and `stop_wait` don't record it. `SkyWatcher` gains
  `pub async fn initialised(&self, axis: Axis) -> Option<bool>`: none before the first
  read and after a read that failed. `Mount::state` otherwise behaves as before.
- **US4:** a private `read(command, axis, len)`. Each try is one transmission
  (`Transport::command` with `safe` false), and `read` retries a timeout, a reply the
  transport could not frame and a framed reply of the wrong length, up to five
  transmissions in all, so the transport's own retries never nest inside it. `status`
  (also used inside `goto`, `rate` and `stop_wait`) and `position` use it. Each framed
  reply of the wrong length is counted, kept as `last_invalid`, rebuilt as `=`, its
  digits and `\r` (`Protocol` sees only the digits; the transport has already checked
  the `=` and the `\r`) and escaped with `escape_ascii`, and logged as
  `mount reply invalid for :<command><axis>: <bytes>`. Replies the transport can't frame
  are retried but not counted: their bytes never leave `transport.rs`, and `Error` gains
  no variant. `goto` records `GotoPath::Absolute` or `GotoPath::Relative` per axis once
  `:S`, or `:H` then `:M`, is accepted, before `:J`, and logs
  `mount goto path on axis <n>: :S` (or `:H/:M`) when the axis's path changes, so once
  per board. `GotoPath` displays as `:S` and `:H/:M`. `SkyWatcher` gains
  `pub async fn health(&self) -> MountHealth`.
- **US4, `MountLink::health`:** after each delegated `state`, `goto` and `rate`, the link
  copies `board.health()` into its snapshot, unless the board was replaced meanwhile. A
  new connect folds the old board's health into a retired total, so `invalid_replies`
  sums over every board the link has had, and `last_invalid` and `goto_path` prefer the
  current board's. `mount_link.rs` re-exports both types
  (`pub use super::protocol::{GotoPath, MountHealth};`), because `protocol` is a private
  module (`crates/scope-node/src/mount/skywatcher/mod.rs:4`); tests import
  `scope_node::mount::skywatcher::mount_link::{GotoPath, MountHealth}`.

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

A powered-off board neither replies nor carries out commands. `inject_reply` carries out
the command and replaces only its reply. Faults injected before a power cycle survive
it. `set_position` overwrites the counts and does not cancel a GoTo in flight.
`reopen` returns a stream at 9600 baud with empty buffers that is not recorded in
`baud_attempts()`, like the stream `Emulator::new` returns. The handle gains no position
getter: tests read positions through the link and set them with `set_position`.
`inject_reply` and the power methods are test tooling in the emulator's source, not
tests; no existing test calls them.

### The service (`crates/scope-node/src/service.rs`)

- **US1:** `connected_mount` keeps `no mount connected` when there is no `[mount]`, and
  fails with `Status::failed_precondition("mount not connected")` when
  `mount().connected()` is false. `moving_mount` calls `connected_mount` before the
  watchdog's `heartbeat required after link loss` check
  (`crates/scope-node/src/service.rs:205`): the watchdog trips 1 s after the service is
  built when no heartbeat comes (`crates/scope-node/src/mount/watchdog.rs:42`), and a
  disconnected link must still answer `mount not connected`. `describe_mount` returns no
  axes while not connected. `stop_all` still goes through the queue's `stop_all`
  (`crates/scope-node/src/mount/queue.rs:115`), so Stop clears queued moves whatever
  the link's state; the link's `stop_confirmed` answers false while it has no board, so
  the handler answers `Status::unavailable("the mount did not confirm the stop")`.
- **US2:** `AxisState.mount_resets` is `mount().resets()`, read after the axes, as
  `watchdog_stops` is.
- **US3:** `DescribeMountReply.firmware` and `.board` from `mount().identity()` while
  the mount is connected (the same filter as the axes), empty strings otherwise.

### The capture (`crates/scope-node/src/capture.rs`, US1)

`positions` returns none when `mount.connected()` is false
(`crates/scope-node/src/capture.rs:45`).

### The binary (`crates/scope-node/src/main.rs`, US1)

`serve` builds `MountLink::start(Box::new(move || Serial::open(&device)), clock.clone())`
instead of awaiting `SkyWatcher::connect`
(`crates/scope-node/src/main.rs:82`), and passes it as the `Arc<dyn Mount>`. It logs
nothing per connect; the link does. `--probe-mount` is unchanged.

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

The live loop records a sync from every solved frame at rest or tracking slowly
(`crates/finder/src/live.rs:223`, through `crates/finder/src/session.rs:1052`), so with
frames running a refusal lasts until the next solved frame. That sync is the one the
spec asks for, and it opens the new epoch.

- **`live.rs`**: `MountMonitor::start` (`crates/finder/src/live.rs:286`) gains a third
  argument, the console clock `clock: Arc<dyn Clock>`. Each mount poll
  (`crates/finder/src/live.rs:296`) passes `set_mount` a `MountReading` with the reply's
  `firmware` and `board` (none when empty) and the azimuth reply's `mount_resets`, and
  the poll's console time `clock.now()`. A failed poll passes `MountReading::default()`
  (`crates/finder/src/live.rs:320`), which leaves the last-seen count as it was.
- **`api.rs`**: the two builders that start the monitor pass `self.clock.clone()`:
  `with_simulation` (`crates/finder/src/api.rs:196`), just after it sets the simulation's
  clock (`crates/finder/src/api.rs:194`), and `with_link`
  (`crates/finder/src/api.rs:217`), whose callers already set the clock first
  (`crates/finder/src/api.rs:210`). The clock has to come from here: `Session` holds
  none (`crates/finder/src/session.rs:662`), `LinkManager` keeps its own clock private,
  and `LinkStatus.last_frame_at` is the last frame's arrival, not the poll's time. The
  module docs' second-sync and Track paragraphs (`crates/finder/src/api.rs:3`,
  `crates/finder/src/api.rs:17`) gain the refusal after a reset.
- **`session.rs`**: `set_mount` stays `pub(crate)` and takes a
  `pub(crate) struct MountReading { counts, connected, moving, firmware, board, resets }`
  and `time: f64`, the poll's console time, in place of its three arguments. `MountView`
  gains `#[serde(skip_serializing_if = "is_zero")] resets: u64`. The session keeps the
  last count a good poll saw (none before the first, so the first poll only learns it),
  and a `sync_needed` flag with the `time` of the poll that set it, set when a good
  poll's count is above the last seen. `record_frame_sync` clears the flag only when it
  records a sync from a frame that started after that time (the frame's start on the
  console clock is its sync time less half the exposure,
  `frame.end_s - frame.start_s`), as `crates/finder/src/goto.rs:950` waits for an
  exposure starting after settle.
  `Session::sync_needed()` is `pub`, and `pub(crate) const POWER_CYCLED` holds
  `The mount was power-cycled: sync again first`. `Session::track_target`
  (`crates/finder/src/session.rs:759`) returns it as its error when `sync_needed()`,
  right after its "the night has no sync yet" check; the API maps that to `409`.
- **`goto.rs`**: `GoTo::check_request` (`crates/finder/src/goto.rs:645`) returns
  `POWER_CYCLED` as its `String` error when `sync_needed()`, after its "a GoTo is already
  running" and "no mount is connected" checks; `crates/finder/src/api.rs:706` already
  maps it to `409`. `check_request` also serves second sync
  (`crates/finder/src/api.rs:725`) and `GoTo::refusal`, so `GET /api/session`'s GoTo
  refusal shows the same words. `GoTo::check_axis_move` is not changed (owner
  decision 2). A running GoTo takes as its baseline the session's last-seen count when
  it was accepted (its first read's count before the console's first poll); its first
  read (`crates/finder/src/goto.rs:907`) and each slewing read beside `watchdog_stops`
  (`crates/finder/src/goto.rs:930`) end it `failed` with
  `mount power-cycled during the move` when `mount_resets` differs. The existing failure
  path then sends `StopAll`. A reset during the solving or correcting phases is seen at
  the next slew, if any.
- **`tracking.rs`**: the rate loop (`crates/finder/src/tracking.rs:735`) reads
  `axis_state` for the azimuth on each 1 s tick; the first read sets the baseline, and a
  change ends tracking with `the mount was power-cycled` (owner decision 5). The
  keyhole and take-up waits don't check it.
- MCP `goto_target` and `set_tracking` go through `GoTo::check_request` and
  `Session::track_target`, so they refuse with the same words without a change of their
  own; no scenario reads them.

### `crates/scope-node/README.md` (US1, US2, US4)

A `## The mount link` section, written by the story that builds each part. US1: the node
starts without the mount and connects when it answers, and FR-003's line. US2: a power
cycle is recognised and the board is initialised again, the reset line of FR-007, and
that a console restarted after a power cycle needs a sync before GoTo. US4: FR-017's two
lines. The console's refusal is US3's and is documented in the console's own API docs
(`crates/finder/src/api.rs`), not in the node's README.

### Log lines

The log lines of FR-001, FR-003, FR-007 and FR-017 are read from the story's diff, not
from a test: no test reads the node's standard error in process, and US1-S5 reads the
binary's only for `mount not connected`.

## Shapes that existing tests pin

These are not changed by any story:
- **`SkyWatcher::connect`'s exchanges.** `baud_discovery_initialization_and_auxiliary_encoders`
  (`crates/scope-node/tests/skywatcher.rs:136`) reads them, and `probe_order_and_report`
  (`crates/scope-node/tests/skywatcher.rs:244`) reads `probe()`'s. The link calls
  `connect` as it is; US2 adds no command to it.
- **`state` on a failed read** returns the last state with `moving: true`
  (`position_wrap_zero_rate_and_lost_telemetry` in `crates/scope-node/tests/skywatcher.rs`).
  US4's retries happen before that rule, and five bad replies still reach it (US4-S2).
- **A motion start is sent once** (`transport_timeouts_and_replay_safety`). US4 retries
  only reads, and US4-S5 guards it again through the link.
- **`EmulatorConfig`'s fields** and `Emulator::new`'s signature.
- **`GET /api/session` with no reset.** `resets` is skipped at 0 and `firmware` and
  `board` stay null with nodes that report none, so existing bodies are unchanged.
- **A node with no `[mount]`** behaves exactly as before: no link, no axes,
  `axis_state` and `goto_axis` refused, Stop answers OK
  (`crates/finder/tests/mount_calls.rs:239`); `--check-config` still reports the mount
  device without opening it (`crates/scope-node/tests/mount_config.rs:31`).

## Decisions and the alternatives rejected

| Decision | Rejected | Why |
|---|---|---|
| A supervisor around `SkyWatcher` | Reconnect inside `SkyWatcher` | Its tests build it directly over one stream; a supervisor leaves it as tested. |
| Retry the open 2 s after each failure | Exit and let systemd restart | 44 restarts lost the camera each time, and a restart can't keep a reset count. |
| The link's state in a `std::sync::Mutex` snapshot | One tokio Mutex around everything | `connected`, `resets`, `identity`, `axes` and `health` are synchronous; a tokio Mutex can only be tried from them, and that fails while a read is in flight. |
| `initialised()` reads none after a failed read | A fallible `read_state` on `SkyWatcher` | The probe build showed it is enough to count failed reads, and `Mount::state` stays as tested. |
| Every reconnect counts as a reset | Only reconnects that find the bit cleared | A reconnect runs `:F3` before anything can read the bit, and some firmware may not set it; one extra sync is cheap. |
| The bit rule needs the bit to have read set first | Any clear bit is a reset | A board that never sets the bit would count a reset on every poll. |
| Jump rule at 1.5 × top rate + 1,000 counts | Any change while at rest | Polls are not instants, and the board may still be finishing a stop. |
| One reading is enough | Confirm with a second read | Owner decision 1. |
| Refuse GoTo, second sync and Track until a sync | Re-zero the model from the new counts | The counts after power-on are an unknown offset from the old ones; only a sync measures it. |
| Only a frame whose exposure started after the rise was seen clears the flag | Any recorded sync clears it | A frame exposed before the cycle carries the old zero points. |
| The monitor gets the console clock from `api.rs` | Time the rise from `LinkStatus.last_frame_at` or `SystemTime` | The last frame's arrival can be older than the poll, and `SystemTime` is not the clock the tests and the simulation run on. |
| A session flag, not a persisted one | Store the count with each sync | It changes the session file's format; a console restart after a cycle is rare and the README covers it. |
| Stop still goes through the queue while disconnected | Answer without the queue | Stop must clear queued moves whatever the link's state. |
| Retry reads in `protocol.rs`, one transmission a try | Retry in the transport | The transport can't tell a short status from a valid empty acknowledgement, and nested retries would allow 25 transmissions. |
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
2. **`main` is still as read.** Every citation here is at c1ed295. `main` at 0a33c19
   adds only `specs/020-live-chart-calm/`, whose stories write `crates/finder/web`,
   `crates/finder/src/solve.rs`, a finder test and `DESIGN.md`, none of 024's files. If
   `main` has moved in these files since, re-read them and re-run the validator.
3. **The owner decisions are settled and the owner approvals given** (spec § Owner
   decisions, § Owner approvals). Flipping to `ready` records them.
4. **No operator-staged files are needed.**

## Plan traps

Lessons from building 012 to 019 on this factory, each with its mechanism and the check
that keeps it from recurring. They bind every story.

1. **Gate evidence or nothing.** The judge credits only gate results and the story's
   diff. *Check:* every Then clause names a committed test that the `test` gate runs.
2. **A named command is read literally.** *Check:* commands are copied from
   § Verification.
3. **One evidence budget.** *Check:* each story keeps its diff under 64 KiB, measured
   with `git diff <base> -- . ':(exclude)crates/finder/web/dist' | wc -c`. The probe
   builds came to 28, 25, 29 and 16 KB.
4. **The sandbox has only the gates' tools.** *Check:* no task runs a tool outside them.
5. **A spec fixed mid-epic needs a relaunch.** The judge grades the spec as dispatched.
   *Check:* no builder edits `specs/024-mount-link-recovery/` or `.specify/`.
6. **An oracle copied from the code proves nothing.** *Check:* tests read the emulator's
   board state and recorded exchanges, and never call the link's jump rule or parser to
   get an expected value.
7. **Paused time and background tasks.** The link connects in a spawned task; with
   Tokio's clock paused, a test must sleep for the task to run. *Check:* every scenario
   says how long to wait, and the tests use `start_paused = true`, except US1-S5, which
   runs the binary on the real clock.
8. **A reset that the test caused itself.** A GoTo moves counts quickly; a jump rule
   that is too tight would count it. *Check:* US2-S4 runs a real GoTo and asserts no
   reset, and US2-S2 and US2-S7 move 100,000 counts first.
9. **Struct literals break on a new field.** *Check:* no field is added to
   `EmulatorConfig`, `AxisState` (the node's Rust struct), `Error` or any type a test
   builds by literal; proto messages are built only in `crates/scope-node/src/service.rs`.
10. **Each worktree builds on its own.** *Check:* no story sets `CARGO_TARGET_DIR`.
11. **Every solved frame is a sync.** A refusal that a sync clears is gone by the next
    solved frame. *Check:* US3-S3 and US3-S6 stop the node's frames before they set the
    reset count, and US3-S4 starts them again.

## Risks

| Risk | Mitigation |
|---|---|
| The real board (firmware 2.09) doesn't set the init bit. | The bit rule fires only after the bit read set on that axis since the connect, so such a board never trips it (US2-S8); a lost link and the jump rule still catch a cycle. |
| A garbled reply of the right length reads as a reset, and garbles came every 1 to 2 minutes. | US4 retries only wrong lengths. Owner decision 1 offers a confirming second read. |
| The board refuses `:q`, so no `:W` is sent and its encoders stay on: every fast hand push counts as a reset. | The FR-003 line says whether `:W` was sent. Owner decision 1 offers turning the jump rule off then. |
| The real board's goto speed was never measured; above 5.1°/s every long GoTo would trip the jump rule. | Owner decision 1; the first powered night should include a move over 10°. |
| A reset during a long slew isn't seen until the next poll. | The motors lose power with the board; the GoTo reads the count every 50 ms while slewing and the console's poll runs every 250 ms. |
| Reconnecting while a move is queued sends a stale move. | `goto` and `rate` send nothing while not connected (FR-008), and the console aborts on a reset (FR-015); owner decision 3. |
| Two open descriptors on one serial device split its replies. | A reset takes the old board out under `io` and drops every handle on it before `open()`; only a Stop already under way can still finish on it. Reads while there is no board are not failed reads, so the connect task wakes only for a loss or a failed in-line connect, never beside one. |
| The retry hides a real fault. | Every wrong-length reply is counted and logged with its bytes (FR-017). |

## Refinement Record

Drafted on 2026-11-14 by the plate-solver operator against `main` at c1ed295, from the
13-14 November field note and a read of the node's mount module, the service and the
console's mount poll. The same day each story was probe-built in a throwaway worktree
from the trio alone, stacked US1, US2, then US3 and US4 beside each other, and two
reviews read the trio against the code. Every probe passed its story's tests, its
crates' tests, clippy and fmt, with no undeclared write and no existing test touched, in
16 to 29 KB of diff. They showed what this revision fixes: the link's state must sit in a
`std::sync::Mutex` for the synchronous trait methods; the link could not see a failed
read until `initialised()` read none after one; `identity` had no story; the refusal
is cleared by the next solved frame, so its scenarios stop frames; the init-bit rule
would count a reset on every poll of a board that never sets the bit; and several
waits, imports and oracles needed fixing. A completeness review of that revision then
found that the rule clearing the console's refusal needs the console clock in the mount
poll, which only `crates/finder/src/api.rs` can pass, so US3 now writes it, and that a
failed in-line re-initialisation must wake the connect task. § Interfaces names the
parts no probe has compiled. Writes come from the task text. Shared writes name the
merge edge that orders them, and the 013, 014, 015 and 017 stories that last wrote the
same file on `main`. Nothing in this record is an owner approval: those are in
spec.md, and flipping to `ready` gives them.

```yaml
version: 1
reviewed_revision: c1ed295734253c7d6121052b0a14beb2ee64bc17
stories:
  US1:
    tests:
      - path: crates/scope-node/tests/mount_link.rs
        status: new
        disposition: add
        assertion: "a link to a powered-off board stays disconnected and sends no move; after power-on it connects, sends F3 and both W commands, the board's encoders read off, and the link names firmware 3.40 on board 0x90"
        decision: "spec.md:US1-S1"
      - path: crates/scope-node/tests/mount_link.rs
        status: new
        disposition: add
        assertion: "the service over a disconnected link describes no axes, refuses axis_state, goto_axis and rate_axis with mount not connected, answers stop_all UNAVAILABLE, and frames carry no counts"
        decision: "spec.md:US1-S3"
      - path: crates/scope-node/tests/mount_link.rs
        status: new
        disposition: add
        assertion: "the binary with a missing mount device keeps running and logs mount not connected"
        decision: "spec.md:US1-S5"
      - path: crates/scope-node/tests/skywatcher.rs
        status: existing
        disposition: preserve
        assertion: "SkyWatcher::connect's exchanges, probe()'s order and the lost-telemetry rule are unchanged"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/mount_calls.rs
        status: existing
        disposition: preserve
        assertion: "a node with no mount describes no axes, refuses axis_state and goto_axis, and answers Stop OK"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/scope-node/tests/mount_config.rs
        status: existing
        disposition: preserve
        assertion: "--check-config reports the mount device without opening it and rejects a bad [mount] table"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "SkyWatcher::connect and its Mount methods keep their signatures, and their exchanges when every reply is valid (spec.md FR-004)"
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
        reason: "US2 depends_on_merged US1, and adds loss and reset detection to the link"
      - path: crates/scope-node/src/service.rs
        other_spec: 024-mount-link-recovery
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and adds mount_resets to axis_state"
      - path: crates/scope-node/README.md
        other_spec: 024-mount-link-recovery
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and adds resets to the README's mount link section"
      - path: crates/scope-node/README.md
        other_spec: 013-live-finder
        other_story: US2
        disposition: merge_order
        reason: "024 depends_on_landed 013, whose US2 was the last spec story to change the node's README; #194 (9892319) has edited it since"
      - path: crates/scope-node/src/capture.rs
        other_spec: 014-goto
        other_story: US2
        disposition: merge_order
        reason: "024 depends_on_landed 014, whose US2 last changed capture.rs"
      - path: crates/scope-node/src/main.rs
        other_spec: 014-goto
        other_story: US3
        disposition: merge_order
        reason: "024 depends_on_landed 014, whose US3 was the last spec story to change main.rs; #194 (9892319) has edited it since"
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
        assertion: "a power cycle, a lost and recovered link and a counts jump each count one reset, and the cycle re-runs F3 and both W and leaves the encoders off"
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
      - path: crates/scope-node/tests/mount_reset.rs
        status: new
        disposition: add
        assertion: "a cycle after a move counts one reset and sends F3 once; a status bit that never read set counts none"
        decision: "spec.md:US2-S7"
    preserved_behavior:
      - "the first connect after start is not a reset"
      - "watchdog_stops is unchanged"
      - "SkyWatcher's Mount::state still logs a failed read and returns the last state with moving true"
    checks:
      development:
        - "cargo test -p scope-node --test mount_reset --locked"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs, and a power cycle at rest on the real board"
    writes:
      - crates/scope-link/proto/scope_node/v1/scope_node.proto
      - crates/scope-node/README.md
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
      - path: crates/scope-node/README.md
        other_spec: 024-mount-link-recovery
        other_story: US4
        disposition: merge_order
        reason: "US4 depends_on_merged US2, and adds its log lines to the README's mount link section"
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
        assertion: "after a reset GoTo, second sync and Track answer 409 with the power-cycled words and no GotoAxis reaches the node, until a sync from a later frame"
        decision: "spec.md:US3-S3"
      - path: crates/finder/tests/mount_identity.rs
        status: new
        disposition: add
        assertion: "a slewing GoTo ends failed with power-cycled and sends StopAll; running tracking ends off with power-cycled"
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
      - "MCP direct axis moves are not refused after a reset (owner decision 2)"
    checks:
      development:
        - "cargo test -p finder --test mount_identity --locked"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - crates/finder/src/api.rs
      - crates/finder/src/goto.rs
      - crates/finder/src/live.rs
      - crates/finder/src/session.rs
      - crates/finder/src/tracking.rs
      - crates/finder/tests/mount_identity.rs
      - crates/scope-link/proto/scope_node/v1/scope_node.proto
      - crates/scope-node/src/service.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/api.rs
        other_spec: 017-console-stop-confirm
        other_story: US1
        disposition: merge_order
        reason: "024 depends_on_landed 017, whose US1 last changed api.rs"
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
      - path: crates/scope-link/proto/scope_node/v1/scope_node.proto
        other_spec: 017-console-stop-confirm
        other_story: US4
        disposition: merge_order
        reason: "024 depends_on_landed 017, whose US4 last changed the proto"
      - path: crates/scope-node/src/service.rs
        other_spec: 017-console-stop-confirm
        other_story: US4
        disposition: merge_order
        reason: "024 depends_on_landed 017, whose US4 last changed service.rs"
    open_questions: []
  US4:
    tests:
      - path: crates/scope-node/tests/mount_garble.rs
        status: new
        disposition: add
        assertion: "a wrong-length status or position reply is retried, counted and kept escaped; five in a row still read as moving and use five transmissions"
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
      - crates/scope-node/README.md
      - crates/scope-node/src/mount/skywatcher/mount_link.rs
      - crates/scope-node/src/mount/skywatcher/protocol.rs
      - crates/scope-node/tests/mount_garble.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/scope-node/src/mount/skywatcher/protocol.rs
        other_spec: 017-console-stop-confirm
        other_story: US3
        disposition: merge_order
        reason: "024 depends_on_landed 017, whose US3 last changed protocol.rs"
    open_questions: []
```
