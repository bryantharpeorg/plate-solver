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
- US3: `crates/finder/tests/motion/mount_identity.rs` (new), a module of finder's
  `motion` test binary declared in `crates/finder/tests/motion/main.rs`: an in-process
  console over a memory transport, plus a `describe_mount` check on the node's service.
- US4: `crates/scope-node/tests/mount_garble.rs` (new).
- No Playwright or Vitest test: the page is unchanged.

**Target Platform**: `scope-node` on the Pi; `finder` on any desk machine.

**Constraints**:
- `SkyWatcher::connect` and its `Mount` methods keep their signatures, and their
  exchanges when every reply is valid; existing tests in
  `crates/scope-node/tests/skywatcher.rs` and `crates/scope-node/tests/skywatcher_stop.rs`
  build it directly.
- `EmulatorConfig` gains no field: its `Default` and field set are pinned by the tests'
  configs.
- The `Error` enum of the Sky-Watcher module gains no variant.
- Proto changes are additions only.

**Scale/Scope**: one new module, four trait or handle additions, three proto fields, one
console refusal.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Nothing touches the solver or `crates/math-core`. |
| II. Never weaken a gate | No existing test, golden, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS` or `ergane.yaml` is edited. New tests go in new files; US3's module adds one `mod mount_identity;` line to `crates/finder/tests/motion/main.rs`, which spec.md owner approval 7 names. |
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
├── src/mount/skywatcher/mount_link.rs          # US1: new; US2: loss, resets, Stop over the last stream; US4: health
├── src/mount/skywatcher/emulator.rs            # US1: reopen, power, set_position, inject_reply
├── src/mount/skywatcher/protocol.rs            # US2: the init bit, halt_once; US4: retried reads, goto path
├── tests/mount_link.rs                         # US1: new
├── tests/mount_reset.rs                        # US2: new
└── tests/mount_garble.rs                       # US4: new
crates/finder/
├── src/api.rs                                  # US3: the console clock to the mount poll; docs
├── src/live.rs                                 # US3: identity and resets from the node
├── src/session.rs                              # US3: MountView fields, sync-needed flag, Track's refusal, tracking's baseline
├── src/goto.rs                                 # US3: GoTo and second-sync refusal, abort on a reset while slewing or correcting
├── src/tracking.rs                             # US3: start_with_resets; stop on a reset, at the tick read and in the keyhole's waits
├── tests/motion/main.rs                        # US3: one line, mod mount_identity;
└── tests/motion/mount_identity.rs              # US3: new
```

## Interfaces

Every story was probe-built from this section three times on 2026-11-14, each time from
the trio as it then stood, and every signature here compiled as given, except that
round 3 had to make `MountReading`'s fields `pub(crate)`, as written below. The second
round also built the parts the first round's reviews had added (Stop through the queue
while disconnected; the bit rule's need for a set bit first; `io` released before the
re-initialisation's connect and the connect task woken after a failed one; the frame
cutoff and its console clock). The third built the parts the owner's answers of
2026-11-15 added: Stop over the last connect's stream (US2's **Stop** bullet and
`halt_once`), GoTo's read after each correction's plan, and the count in tracking's
`at_rest` (US3). Where a probe had to add to the text, this section says what it built:
the re-initialisation runs in a spawned task, and the helpers and accessors named below.
Tracking's baseline from the session's count (`Tracker::start_with_resets`, US3), added
on 2026-11-15 when the owner closed gap (d), was probe-built in round 4 on 027's layout:
every signature here compiled as given.

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
    /// Added by US4, not US1: invalid-reply count, last invalid bytes, GoTo path per axis.
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
  baselines, the failure count, a pending cause and Stop's board (below), and US4 the
  health. The snapshot is
  cloned out and never held across an `await`; the `SkyWatcher`'s own lock still
  serialises the board. The opener, the clock and the snapshot sit in a private inner
  struct behind an `Arc`, and the connect task holds only that `Arc`, never the
  `Arc<MountLink>`: otherwise the link's `Drop`, which aborts the task, never runs. US2's
  re-initialisation reaches the opener through it too.
- **Connecting (US1).** The connect task calls `open()`, then `SkyWatcher::connect`. On
  success it logs FR-003's line and then the board's warnings as `mount warning: <text>`,
  and stores the `SkyWatcher`. On failure it logs `mount not connected: <error>` when
  the error text differs from the last one since the last successful connect, so the
  first failure after each loss is logged, and waits 2 s. Against a silent board one
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
  nothing and log `mount not connected: move dropped`; before the first connect
  `stop_all` sends nothing and `stop_confirmed` answers false at once. From US2 a link
  that has connected once still sends Stop while it is not connected (**Stop**, below).
- **`identity` (US1)** is none while not connected, and otherwise comes from
  `report().firmware`: `format!("{}.{:02}", major, minor)` and
  `format!("0x{:02X}", board)`.
- **Lost (US2).** Three failed `state` reads in a row, counted across both axes and
  cleared by any good read, take the stored `SkyWatcher` out of the snapshot (Stop's
  board stays, below), log `mount link lost: 3 failed reads in a row` and wake the
  connect task. A
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
    on this board by more than 1.5 × `top_rate_deg_per_s` × elapsed seconds (on the
    link's `NodeClock`, read after each good read) × `counts_per_revolution` / 360 +
    1,000, taking both from the board's `axes()` and wrapping the difference at 24 bits
    with `protocol.rs`'s private `wrapped_delta`
    (`crates/scope-node/src/mount/skywatcher/protocol.rs:312`), which US2 makes
    `pub(super)`.

  One read counts at most one reset, and `not initialised` wins over
  `counts jumped on <axis>`. The read that found the reset is returned as read. On a
  reset the link first takes the board out of the snapshot, and the read drops its own
  handle, so `connected()` reads false and `goto` and `rate` drop their moves (FR-008);
  then it releases `io` and runs `SkyWatcher::connect` on a fresh `open()` stream in a
  task of its own (`tokio::spawn`), whose `JoinHandle` the read that found the reset
  awaits. A read dropped by its caller leaves that task running, as the node's Stop
  handler already does (`crates/scope-node/src/service.rs:489`): the console bounds each
  mount poll at 200 ms (`crates/finder/src/live.rs:318`), a connect at 9600 baud may
  take longer, and a connect cancelled with the read would leave the link with no
  board and no connect under way until the node restarts (US2-S10). No `await` comes
  between taking the board out and the spawn. The count goes up when the
  re-initialisation succeeds, with the log line
  `mount reset (<cause>): re-initialised <firmware> at <baud> baud`, after FR-003's
  connect line and with `<firmware>` as that line gives it; if it fails, the task logs
  `mount reset (<cause>): re-initialisation failed: <error>`, keeps the cause and wakes
  the connect task, and that task's next successful connect counts the cause, so one
  event counts once (US2-S9). A connect after the first with no cause kept counts as
  `link lost`. While the spawned re-initialisation runs the link has no board, so no
  read can count toward a loss and the connect task stays waiting: only one connect is
  ever under way. A read that comes meanwhile answers at once with the last state,
  `moving: true`. The read that found the reset waits about 0.3 s for a connect that
  succeeds and about 3 s for one that fails; when that read is the capture loop's, no
  frame is captured meanwhile.
  Every connect clears the jump baselines, the failure count and the init-bit history.
- **Stop (US2, owner decision 4).** The snapshot also keeps `stop_board`, the
  `Arc<SkyWatcher<S>>` of the last successful connect. A loss or a reset never takes it
  out; the next successful connect replaces it, which drops the old handle and closes
  its stream. So once the link has connected, it always holds the last open stream to
  the board. The inner struct gains `device: tokio::sync::Mutex<()>`,
  `give_way: tokio::sync::Notify` and `stopping: std::sync::atomic::AtomicUsize`:
  - **a connect attempt gives way.** Every attempt, the connect task's and the spawned
    re-initialisation's, first creates `give_way.notified()`, then races the rest of the
    attempt against that notification in a `tokio::select!` that is `biased` toward it:
    taking `device`, which it holds from `open()` until the new board is stored, `open()`
    and `SkyWatcher::connect`. When the notification wins, the attempt drops the
    connect, which closes its stream, and fails with the error
    `connect gave way to a stop` (an attempt's error is its text, so `Error` gains no
    variant): the connect task logs it and tries again 2 s later, and a
    re-initialisation keeps its cause and wakes the connect task, as for any failed
    re-initialisation (US2-S9). A `Notified` created before `notify_waiters()` receives
    it even before it is first polled. An attempt that finds `stopping` above 0 once it
    holds `device` gives way the same way, so one that began just after a Stop's
    notification, on the node's other worker thread, can't hold that Stop either;
  - **Stop takes the device.** Before the first connect (no `stop_board`),
    `stop_confirmed` answers false at once and `stop_all` sends nothing, without
    notifying `give_way` or taking `device`, so the node's watchdog, which trips 1 s
    after the service is built when no heartbeat comes
    (`crates/scope-node/src/mount/watchdog.rs:42`), and the console's repeated Stops never
    delay the first connect. Otherwise `stop_confirmed` and `stop_all` raise `stopping`
    through a guard that lowers it when Stop ends or its future is dropped, call
    `give_way.notify_waiters()` and then take `device`, which an attempt gives up within
    a task switch, so Stop never waits seconds on a connect and never runs beside one.
    Holding it, Stop reads the snapshot: with a board connected it delegates to it as
    before (017's halt, with the transport's five tries); with none it calls
    `stop_board.halt_once()` (below). Stop never takes `io`, and an in-flight
    move is already cancelled by the queue (`crates/scope-node/src/mount/queue.rs:115`)
    before the link's `stop_confirmed` runs. The node's watchdog stops through that same
    queue call (`crates/scope-node/src/mount/watchdog.rs:51`), so it takes this path
    too;
  - **one descriptor read at a time.** On the serial device a reconnect's stream and the
    kept one are two descriptors on one tty, which share its input queue and its baud.
    Only one is ever read: reads and moves use only a connected board, the connect task
    and a re-initialisation never run at once, and Stop holds `device`, which no attempt
    can hold beside it and under which every new board is stored. `halt_once` sets the
    kept stream back to the board's baud first, because a discovery that gave way may
    have left the tty at 115200 baud
    (`crates/scope-node/src/mount/skywatcher/protocol.rs:81`);
  - **the device gone.** A kept stream whose device has gone fails its first call at
    once, so Stop answers false at once, as it does before the first connect.

### `SkyWatcher` additions (`crates/scope-node/src/mount/skywatcher/protocol.rs`)

- **US2:** `Status` gains `initialised: bool` from the third digit's lowest bit
  (`crates/scope-node/src/mount/skywatcher/protocol.rs:150`). `Protocol` records it per
  axis on each full `state` read, and `Mount::state` sets it to none when the read fails
  (`crates/scope-node/src/mount/skywatcher/protocol.rs:394`); the status reads inside
  `goto`, `rate` and `stop_wait` don't record it. `SkyWatcher` gains
  `pub async fn initialised(&self, axis: Axis) -> Option<bool>`: none before the first
  read and after a read that failed. `Mount::state` otherwise behaves as before.
  `wrapped_delta` (`crates/scope-node/src/mount/skywatcher/protocol.rs:312`) becomes
  `pub(super)` for the link's jump rule.
- **US2, Stop's path with no board connected:** `SkyWatcher` gains
  `pub async fn halt_once(&self) -> bool`. Under the protocol lock it first sets the
  stream back to the board's baud (`self.report.baud`, through `Transport::set_baud`,
  which on the serial device also flushes stale input), then sends `halt`'s sequence
  (`crates/scope-node/src/mount/skywatcher/protocol.rs:297`: `:K1`, `:K2`, then `:L` on
  each axis whose `:K` was not acknowledged) with `Transport::command`'s `safe` false
  (`crates/scope-node/src/mount/skywatcher/transport.rs:35`), so each command is one
  transmission and a board that answers nothing costs four 300 ms timeouts, 1.2 s, not
  the 6 s of five tries each. It answers true when every axis acknowledged `:K` or
  `:L`, and otherwise logs `mount stop failed: <error>` and answers false; a failed
  `set_baud` answers false at once. `halt`, `stop_all` and `stop_confirmed` are
  unchanged, so a connected board still gets 017's halt with five tries a command.
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
  no variant. A refusal (`!n`) or an I/O error returns at once, neither retried nor
  counted, as the transport returned them before. `goto` records `GotoPath::Absolute`
  or `GotoPath::Relative` per axis once `:S`, or `:H` then `:M`, is accepted, before
  `:J`, and logs
  `mount goto path on axis <n>: :S` (or `:H/:M`) when the axis's path changes, so once
  per board. `GotoPath` displays as `:S` and `:H/:M`. `SkyWatcher` gains
  `pub async fn health(&self) -> MountHealth`.
- **US4, `MountLink::health`:** after each delegated `state` (before its loss and reset
  checks, both of which take the board out), `goto` and `rate`, the link copies
  `board.health()` into its snapshot while the snapshot still holds that board (the copy
  is kept only when the snapshot's board is that `Arc`, by `Arc::ptr_eq`), so the read
  that finds a reset or a loss still counts its own wrong-length replies (US4-S6). A new
  connect folds the old board's health into a retired total, so `invalid_replies` sums
  over every board the link has had, and `last_invalid` and `goto_path` prefer the
  current board's: `goto_path[i]` is the current board's path on axis `i`, or else the
  last retired one, so after a reset the old board's path stands until the new board's
  first GoTo.
  `mount_link.rs` re-exports both types
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

A powered-off board neither replies nor carries out commands, its motors stop, and it
consumes no injected fault. `power_on` builds each motor again as `Emulator::new` builds
it (`crates/scope-node/src/mount/skywatcher/emulator.rs:333`: step mode 2, so a fresh
board's `:f` reads `000`), not as `Motor::default()` would (mode 0, which reads `400`).
`inject_reply` carries out the command and replaces only its reply. Faults
injected before a power cycle survive it: `power_on` keeps them. `set_position`
overwrites the counts and does not cancel a GoTo in flight; the board holds no clock, so
on a moving axis the motion since the last exchange adds to the new counts, and tests
call it only on an axis at rest. US1-S6 proves these helpers on the board itself, through
a bare `SkyWatcher`, because US2 and US4 use them and may not write `emulator.rs`.
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
  the link's state; before the link's first connect its `stop_confirmed` answers false
  at once, so the handler answers
  `Status::unavailable("the mount did not confirm the stop")`. From US2 the handler is
  unchanged and its answer follows the link's Stop over the last connect's stream: OK
  when the board acknowledges (US2-S13).
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
  `crates/finder/src/api.rs:17`) gain the refusal after a reset, and the second-sync
  paragraph one sentence that `POST /api/goto` refuses the same way: that route's own
  docs are in `crates/finder/src/lib.rs:21`, which US3 does not write.
- **`session.rs`**: `set_mount` stays `pub(crate)` and takes a
  `#[derive(Default)] pub(crate) struct MountReading {
  pub(crate) counts: Option<[u32; 2]>, pub(crate) connected: bool,
  pub(crate) moving: bool, pub(crate) firmware: Option<String>,
  pub(crate) board: Option<String>, pub(crate) resets: Option<u64> }` (its fields are
  `pub(crate)` because `live.rs` builds it by literal) and `time: f64`, the poll's console
  time, in place of its three arguments; its default is a failed poll, and its
  `resets: None` leaves the last-seen count. `MountView` gains, after `moving`,
  `#[serde(skip_serializing_if = "is_zero")] pub resets: u64`, with a private
  `fn is_zero(n: &u64) -> bool` in `session.rs` (none exists in the workspace). The
  session keeps the last count a good poll saw (none before the first, so the first poll
  only learns it), readable as
  `pub(crate) fn Session::mount_resets(&self) -> Option<u64>`, and a `sync_needed` flag
  with the `time` of the poll that set it, set when a good poll's count is above the
  last seen. `record_frame_sync` clears the flag only when it
  records a sync from a frame that started after that time (the frame's start on the
  console clock is its sync time less half the exposure,
  `frame.end_s - frame.start_s`), as `crates/finder/src/goto.rs:950` waits for an
  exposure starting after settle.
  `Session::sync_needed()` is `pub`, and `pub(crate) const POWER_CYCLED` holds
  `The mount was power-cycled: sync again first`. `Session::track_target`
  (`crates/finder/src/session.rs:759`) returns it as its error when `sync_needed()`,
  right after its "the night has no sync yet" check; the API maps that to `409`.
  `Session::begin_tracking` (`crates/finder/src/session.rs:879`) starts the tracker it
  builds with `tracker.start_with_resets(target, self.mount_resets())` in place of
  `tracker.start(target)` (`crates/finder/src/session.rs:905`), so tracking's baseline is
  the session's last-seen count when tracking begins, as a GoTo's is the count when it
  was accepted (spec.md owner approval 6, gap (d)). `POST /api/track` and MCP
  `set_tracking` both start tracking through it.
- **`goto.rs`**: `GoTo::check_request` (`crates/finder/src/goto.rs:645`) returns
  `POWER_CYCLED` as its `String` error when `sync_needed()`, after its "a GoTo is already
  running" check and its `self.link.mount().ok_or("no mount is connected")?` line
  (`crates/finder/src/goto.rs:649`) and before `prepare` (whose `prepare_with` has a
  second "no mount is connected" check); `crates/finder/src/api.rs:706` already
  maps it to `409`. `check_request` also serves second sync
  (`crates/finder/src/api.rs:725`) and `GoTo::refusal`, so `GET /api/session`'s GoTo
  refusal shows the same words. `GoTo::check_axis_move` is not changed (owner
  decision 2). A running GoTo takes as its baseline the session's last-seen count when
  it was accepted, `Session::mount_resets()` read in `GoTo::start_request` after
  `check_request` and passed to the private `run` and `motion` as a trailing
  `resets: Option<u64>` (its first read's count when that is none, before the console's
  first poll); its first read (`crates/finder/src/goto.rs:907`) and each slewing read beside `watchdog_stops`
  (`crates/finder/src/goto.rs:930`) end it `failed` with
  `mount power-cycled during the move` when `mount_resets` differs. The existing failure
  path then sends `StopAll`. After each correction's plan
  (`plan = self.prepare(target, client, true).await?.0`,
  `crates/finder/src/goto.rs:1010`), while `goto.state` still reads `correcting`,
  `motion` reads the azimuth's `axis_state` once more and ends the GoTo the same way
  when its `mount_resets` differs from the baseline, before the loop sends that
  correction's `goto_axis` pair (`crates/finder/src/goto.rs:917`): a reset during the
  solving phase or the correction's plan never reaches the wire as a slew (owner
  approval 6, US3-S7). The read is `motion`'s own, after `prepare`, so `prepare_with`,
  which `check_request` and `check_axis_move` share, is unchanged. A reset between that
  read and the pair is seen at the slew's first read, as before.
- **`tracking.rs`**: `Tracker` gains one method,
  `pub async fn start_with_resets(&self, target: Target, resets: Option<u64>) -> Result<(), String>`,
  `start` with the rate loop's baseline given. `start`'s body
  (`crates/finder/src/tracking.rs:156`) moves into it and passes `resets` on to the
  private `run` (`crates/finder/src/tracking.rs:718`) as a trailing argument; `start`
  keeps its signature and calls `self.start_with_resets(target, None)`, so the existing
  tracker tests and US3-S8 start as before. The rate loop
  (`crates/finder/src/tracking.rs:735`) gains an `axis_state` read of the azimuth on
  every 1 s tick, right after the tick and so before that tick's keyhole check,
  take-ups and rates (today it reads `axis_state` only in two conditional branches,
  `crates/finder/src/tracking.rs:796` and `crates/finder/src/tracking.rs:820`). The
  baseline is the given count, or the first read's when none was given (through
  `start`, or from `begin_tracking` before the console's first poll). A reset after the
  console's last poll and before tracking's first read therefore ends tracking at that
  read, before the loop sends any rate (spec.md owner approval 6, gap (d), US3-S9). A
  count other than the baseline ends tracking with `the mount was power-cycled` (owner
  decision 5), and a failed read ends it `link_lost`, as the loop's other reads do. The
  keyhole's waits for rest check it too (owner approval 6, US3-S8): the rate loop passes
  its baseline, a `u64` once the first tick has read the count, to
  `keyhole(&self, target, client, axes, resets: u64)`
  (`crates/finder/src/tracking.rs:534`, called at `crates/finder/src/tracking.rs:744`),
  which passes it to `at_rest(&self, client, counts, resets: u64)`
  (`crates/finder/src/tracking.rs:509`). `at_rest` returns
  `Err("the mount was power-cycled")` when either reading's `mount_resets` differs, so
  the wait after the keyhole's `stop_all` (`crates/finder/src/tracking.rs:558`) and the
  wait after each keyhole waypoint (`crates/finder/src/tracking.rs:585`) end tracking,
  through the rate loop's existing return and `StopAll`, before the next waypoint's
  `goto_axis` pair (`crates/finder/src/tracking.rs:581`). The wait for the target to
  leave the circle (`crates/finder/src/tracking.rs:593`) and the take-up's
  `stop_axis` reads (`crates/finder/src/tracking.rs:671`) don't check it; the next
  tick's read does, after at most a take-up pulse and one second of tracking rates,
  neither of them a slew.
- MCP `goto_target` and `set_tracking` go through `GoTo::check_request` and
  `Session::track_target`, so they refuse with the same words without a change of their
  own; no scenario reads them.

### `crates/scope-node/README.md` (US1, US2, US4)

A `## The mount link` section, written by the story that builds each part. US1: the node
starts without the mount and connects when it answers, and FR-003's line; its sentence
on Stop says that Stop answers unconfirmed at once, sending nothing, before the link's
first connect, so it stays true after US2. US2: a power cycle is recognised and the
board is initialised again, the reset line of FR-007, the loss line and the failed
re-initialisation line, and that a console restarted after a power cycle needs a sync
before GoTo. US4: FR-017's two lines. The console's refusal
is US3's and is documented in the console's own API docs
(`crates/finder/src/api.rs`), not in the node's README.

### Read from the diff, not from a test

- **The log lines** of FR-001, FR-003, FR-007 and FR-017, and the loss and failed
  re-initialisation lines above: no test reads the node's standard error in process,
  and US1-S5 reads the binary's only for `mount not connected`.
- **The frame cutoff in `record_frame_sync` (US3).** No scenario tells it apart from
  "any recorded sync clears the flag": US3-S3 records no sync, and every sync after
  US3-S4 restarts frames comes from a frame exposed after the rise. A frame in flight
  across the rise can't be staged with the rig's 500 ms frames and 200 ms exposure. The
  round-3 probe confirmed it: with "any recorded sync" all eight US3 tests stay green.
- **US3's other paths no scenario reaches.** GoTo's first-read comparison before the
  first slew (the first slewing read catches the same reset one pair later); the
  baselines GoTo and tracking take from their first read when the console has not
  polled yet; `at_rest` right after the keyhole's `stop_all` (US3-S8 reaches the one
  after a waypoint); the tick read's `link_lost` branch; and `MountReading::default()`
  leaving the last-seen count.
- **That `begin_tracking` hands the session's count to its tracker (US3).** US3-S9 gives
  a `Tracker` the count through `start_with_resets` itself. Staged through
  `POST /api/track`, the reset would have to land between the console's mount poll,
  250 ms apart (`crates/finder/src/live.rs:323`), and tracking's first read, a race with
  that poll's timer in the test's one paused runtime; and a Track off its target starts
  a GoTo first, whose own baseline would end it the same way. The round-4 probe
  confirmed it: with `begin_tracking` calling `start(target)`, all nine US3 tests stay
  green; with `start_with_resets` ignoring its count, US3-S9 fails.
- **That every connect clears the jump baselines and the init-bit history (US2).** The
  probe showed no scenario fails without either: US2-S2's reconnect comes about 10 s
  after the last good read, when the jump limit is far above the 100,000 counts the
  power-on zero moves.
- **Stop's hold on `device`, its `stopping` count and its baud (US2).** The emulator
  gives each stream its own buffers and baud, so no scenario fails if Stop uses the kept
  stream while a connect attempt still holds the device, or skips setting the board's
  baud first, and the tests' single-threaded runtime can't stage an attempt that takes
  the device just after a Stop's notification, which only `stopping` catches. These
  matter only on a real serial port, whose descriptors share one input queue and one
  baud, and on the node's two worker threads. The give-way itself is tested: US2-S9,
  US2-S11 and US2-S12 call Stop while a connect attempt holds the device, and US2-S13's
  spawned `stop_all` lets one start first, so all four fail without it.
- **Stop over the kept `stop_board`, not a fresh open (US2).** The emulator's streams
  all reach one board and its `Exchange` records no stream, so no scenario can tell
  them apart; § Decisions rejects the fresh open.
- **Stop with the device gone (US2).** A failed `set_baud` or first write answers false
  at once; the emulator has no gone mode.
- **The watchdog's Stop over a lost link (US2).** It takes the same path through the
  queue's `stop_all` (`crates/scope-node/src/mount/watchdog.rs:51`) as the service's,
  which US2-S13 shows; no scenario trips the watchdog over a lost link.
- **Stop before the first connect leaves that connect alone (US2).** US1-S3 shows that
  it answers at once and sends nothing; no scenario times the first connect around a
  Stop.

## Shapes that existing tests pin

These are not changed by any story:
- **`SkyWatcher::connect`'s exchanges.** `baud_discovery_initialization_and_auxiliary_encoders`
  (`crates/scope-node/tests/skywatcher.rs:136`) reads them, and `probe_order_and_report`
  (`crates/scope-node/tests/skywatcher.rs:244`) reads `probe()`'s. The link calls
  `connect` as it is; US2 adds no command to it.
- **`state` on a failed read** returns the last state with `moving: true`
  (`position_wrap_zero_rate_and_lost_telemetry` in `crates/scope-node/tests/skywatcher.rs`).
  US4's retries happen before that rule, and five bad replies still reach it (US4-S2).
  The same test (`crates/scope-node/tests/skywatcher.rs:303`) also keeps US4's `read`
  from nesting the transport's retries: with `safe` true inside `read`, a sixth
  transmission answers and it fails, and so do US2-S11 and US2-S13.
- **A motion start is sent once** (`transport_timeouts_and_replay_safety`). US4 retries
  only reads, and US4-S5 guards it again through the link.
- **017's halt on a connected board** (`crates/scope-node/tests/skywatcher_stop.rs`):
  `SkyWatcher::stop_confirmed` keeps the transport's five tries a command. US2's
  `halt_once` is a new method beside it, used only by the link with no board connected.
- **GoTo's corrections and tracking's keyhole with no reset**
  (`crates/finder/tests/motion/goto/sim.rs`,
  `crates/finder/tests/motion/tracking/keyhole.rs`): US3's added read and comparisons
  see an unchanged count and send nothing.
- **Finder's test layout** (`crates/finder/tests/layout/main.rs:102`, 027's guard): no
  file at the top level of `crates/finder/tests`, the same helper files under
  `crates/finder/tests/support/`, and nothing in a binary's `main.rs` after its support
  tree but `mod` lines. US3's test is a module of `motion` with its own copy of the
  `Rig`, declared by one `mod mount_identity;` line (spec.md owner approval 7), and adds
  no helper file.
- **`EmulatorConfig`'s fields** and `Emulator::new`'s signature.
- **`GET /api/session` with no reset.** `resets` is skipped at 0 and `firmware` and
  `board` stay null with nodes that report none, so existing bodies are unchanged.
- **A node with no `[mount]`** behaves exactly as before: no link, no axes,
  `axis_state` and `goto_axis` refused, Stop answers OK
  (`crates/finder/tests/motion/mount_calls.rs:239`); `--check-config` still reports the
  mount device without opening it (`crates/scope-node/tests/mount_config.rs:31`).

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
| A session flag, not a persisted one | Store the count with each sync | It changes the session file's format; a console restart after a cycle is rare and the README covers it (owner approval 6, gap (c)). |
| Stop still goes through the queue while disconnected | Answer without the queue | Stop must clear queued moves whatever the link's state. |
| Stop and the watchdog reach a lost or re-initialising board over the last connect's stream | Answer at once while the link has no board (the round-2 answer); wait for the connect to finish; open the device afresh for each Stop | Owner decision 4. A lost board may still run a `:J` move; a connect takes seconds on a silent board; and a fresh stream needs the board's baud, which only a connect finds, and a halt outside `protocol.rs`, where the kept board already has both. |
| A connect attempt gives the device up to Stop | Stop waits for the attempt, or runs beside it | Waiting costs up to 3 s on a silent board; on a real serial port two descriptors read at once split the replies, and they share one baud. |
| With no board connected Stop sends each command once (`halt_once`) | The transport's five tries a command, or a timeout around `halt` | Five tries make a board that answers nothing hold Stop for 6 s; a timeout around `halt` can cut it off before `:K2`, breaking 017's order. Once each, both axes hear `:K` within 0.6 s and the whole stop ends within 1.2 s. |
| GoTo reads the count after each correction's plan, before its slew | Catch it at the next slew's first read | Owner approval 6: that slew's pair, planned through the stale zero points, would already be on the wire. |
| `at_rest` compares the count on every read | Leave the keyhole's waits unchecked | Owner approval 6: a keyhole slew could run to its end through stale zero points. |
| Tracking's baseline is the session's last-seen count when it begins, handed over by one new `Tracker` method | The first tick read's count (the round-3 default) | Owner approval 6, gap (d): a reset between the console's last poll and that read would become the baseline, and tracking would go on with the old zero points. `start` keeps its signature, so no existing caller or test changes. |
| The re-initialisation runs in a spawned task the finding read awaits | Await `SkyWatcher::connect` inside the read | The read is often a console `axis_state` call bounded at 200 ms; a dropped read must not cancel the connect (US2-S10). |
| US1-S6 proves the emulator helpers through a bare `SkyWatcher` | Through the link | Later stories change what the link does after a jump or a cleared bit, and may not edit US1's test. |
| Retry reads in `protocol.rs`, one transmission a try | Retry in the transport | The transport can't tell a short status from a valid empty acknowledgement, and nested retries would allow 25 transmissions. |
| Keep bytes in `health()` | Only log them | A test can read a value; it can't read the node's standard error in process. |

## Verification (every story, before proposing)

- **The factory gates** (`ergane.yaml` at b8dceaa, unchanged since pull request 214):
  - `lint` = `cargo fmt --all --check`,
    `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`,
    `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features --locked`
    and `RUSTDOCFLAGS="-D warnings" cargo doc -p finder --no-deps`;
  - `test` = `cargo nextest run --workspace --all-targets --all-features --locked`,
    `cargo test --workspace --locked --doc`, and then, in `crates/finder/web`,
    `npm ci --no-audit --no-fund`, `npm run typecheck`, `npm run format:check`,
    `npm test`, `npm run build`, `git status --porcelain -- dist | (! grep .)` and
    `CI=1 npm run e2e`.

  Every story runs both, and the judge sees their results. nextest runs each test in a
  process of its own; every story's tests, run on copies of the round-3 probe branches,
  pass under it as under `cargo test`.
- **CI in the merge queue** runs every `ci.yml` job, including the build on Rust 1.96.
  Coverage runs only by hand (constitution 1.1.0).

## Prerequisites before this spec is flipped to `ready`

1. **The earlier specs have landed:** `013-live-finder`, `014-goto`, `015-tracking`,
   `017-console-stop-confirm` and `027-finder-tests-by-system`. Met: 017's last story
   landed at c1ed295, and 027's at b8dceaa.
2. **`main` is still as read.** Every citation here was read at c1ed295, and again at
   b8dceaa. Since c1ed295 `main` has added `specs/020-live-chart-calm/` and its two
   stories (`crates/finder/web`, `crates/finder/src/solve.rs`, the test now at
   `crates/finder/tests/console/star_detected.rs`, `DESIGN.md`); Playwright 1.64.1
   (#213: `.github/workflows/ci.yml`, `.github/workflows/ergane-gates.yml` and the page's
   package files); the nextest `test` gate (#214: `ergane.yaml`, the root `Cargo.toml`
   and `README.md`, `.config/nextest.toml`, both workflows, `docs/coverage.md`,
   `scripts/coverage.sh`); and spec 027 and its two stories (#216, #217, #218: every
   file under `crates/finder/tests` and the page's two recorder fixtures). Of 024's
   files and citations, only the finder test paths that 027 moved have changed
   (Prerequisite 6); every other file this trio cites or writes is as at c1ed295.
   `ergane.yaml`'s `test` gate runs `cargo nextest run` first, which § Verification
   copies. If `main` has moved in 024's files since b8dceaa, re-read them and re-run the
   validator.
3. **The owner decisions are settled and the owner approvals given** (spec § Owner
   decisions, § Owner approvals). The owner answered the round-2 questions on
   2026-11-15: owner decision 4 changed to always send Stop, and owner approval 6 closes
   gaps (a) and (b); later that day the owner closed gap (d) too ("Close it"). Owner
   approval 7 names US3's one `mod` line in an existing test file. Flipping to `ready`
   records every answer.
4. **No operator-staged files are needed.**
5. **The drafts that write 024's files wait for it.** Met. `025-frame-capture` writes
   three of them (its US1 `crates/finder/src/api.rs`; its US2 `crates/finder/src/api.rs`,
   the proto and `crates/scope-node/src/service.rs`), and the `026-mount-sky` draft two
   (`crates/finder/src/api.rs` and `crates/finder/src/session.rs`). The owner settled on
   2026-11-15 that 024 goes before 025, and 026 makes 024's landing its own first
   prerequisite. Both stay draft until 024 has landed and then add it to their own
   `depends_on_landed`, so 024 names neither, in `depends_on_landed` or in
   `shared_writes`.
6. **`027-finder-tests-by-system` has landed, and US3 is re-anchored to its layout.**
   Met: 027's US2 landed at b8dceaa (#218), the owner's order being 027, then 024, then
   025 (recorded in 027's frontmatter), and `main` at b8dceaa was merged into this
   branch. The finder tests this trio cites moved by 027's move map: `mcp/support.rs`
   and `stop/support.rs` to `support/mcp.rs` and `support/stop.rs`, unchanged;
   `goto_sim.rs` to `motion/goto/sim.rs`, `keyhole_sim.rs` to
   `motion/tracking/keyhole.rs`, `goto_refusal_api.rs` to `motion/goto/refusals.rs`,
   `watchdog_goto.rs` to `motion/stop/watchdog.rs` and `mount_calls.rs` to
   `motion/mount_calls.rs`. US3's test is a module of the `motion` binary,
   `crates/finder/tests/motion/mount_identity.rs`, declared by a `mod mount_identity;`
   line in `crates/finder/tests/motion/main.rs`, a US3 write shared with 027's US2 and
   named by spec.md owner approval 7. Every `crates/finder/tests` citation and the
   Refinement Record's finder test paths were re-read at b8dceaa, US3's development
   check is the nextest filter for that module, 027 is in `depends_on_landed`, and the
   validator was re-run. No other story writes `crates/finder/tests`.

## Plan traps

Lessons from building 012 to 019 on this factory, each with its mechanism and the check
that keeps it from recurring. They bind every story.

1. **Gate evidence or nothing.** The judge credits only gate results and the story's
   diff. *Check:* every Then clause names a committed test that the `test` gate runs.
2. **A named command is read literally.** *Check:* commands are copied from
   § Verification.
3. **One evidence budget.** *Check:* each story keeps its diff under 64 KiB, measured
   with `git diff <base> -- . ':(exclude)crates/finder/web/dist' | wc -c`. The round-3
   probe builds came to 29.7, 37.6, 43.1 and 19.7 KB (US2's and US3's are mostly their
   tests, US3's with its copy of the `Rig`); round 4's US3, with the new baseline and
   US3-S9, came to 46.5 KB.
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
9. **Struct literals break on a new field.** *Check:* no field is added to `AxisState`
   (the node's Rust struct, built by literal in `crates/simulator/src/mount.rs:189`),
   `Error` or any type a test builds by literal, and `EmulatorConfig` keeps its fields
   (§ Technical Context); proto messages are built only in
   `crates/scope-node/src/service.rs`.
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
| Two open descriptors on one serial device split its replies, and they share one baud. | The kept stream stays open beside a reconnect's, but only one is ever read: reads and moves use only a connected board, Stop makes any connect attempt give the device up and holds it while it uses the kept stream, every new board is stored under that hold, and `halt_once` sets the board's baud first. Reads while there is no board are not failed reads, so the connect task wakes only for a loss or a spawned re-initialisation that failed or gave way, never beside one; the spawned re-initialisation wakes it only after its own connect has ended. |
| A caller that gives up on the read that found a reset cancels the re-initialisation, leaving the link with no board and nothing to wake the connect task. | The re-initialisation runs in a spawned task that the read awaits through its `JoinHandle`, so a dropped read leaves it running (US2-S10). |
| After a lost link the board may still be powered and running a `:J` move. | Stop and the watchdog send the stop over the last connect's stream (owner decision 4, US2-S11). A pulled cable can't carry it; then the reconnect counts a reset, and the console stops a running GoTo or Track (FR-015). |
| A Stop during a reconnect makes it give way, so the console's repeated Stops in its 2 s window delay the reconnect. | A yielded attempt counts as a failed one: the connect task tries again 2 s later, and a re-initialisation hands its cause to it, so the reset is still counted once (US2-S9). |
| With no board connected, one transmission a command loses an acknowledgement on a garbled line and escalates to `:L`, or answers unconfirmed. | `:L` stops the axis too, and an unconfirmed Stop is what 017's page already shows; a connected board keeps the five tries. |
| One failed read now takes five wrong-length replies, and three failed reads in a row across both axes are a lost link, so a burst of about 15 wrong-length replies on alternating reads becomes a lost link and one reset. | FR-006 and FR-016 as written; every such reply is logged with its bytes (FR-017), so a night's log shows whether it happens. |
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
failed in-line re-initialisation must wake the connect task.

Round 2, the same day: each story was probe-built again in a throwaway worktree from the
trio as it then stood, stacked the same way. Every probe passed its story's tests, its
crates' tests, clippy and fmt, with no undeclared write and no existing test touched,
in 18 to 35 KB of diff, and every signature in § Interfaces compiled as given. Those
probes and two more reviews showed what this revision fixes: a read awaiting the
in-line re-initialisation could be dropped by a console that gives up on its 200 ms
poll, leaving the link disconnected until the node restarts, so the re-initialisation
now runs in a spawned task; US1's scenarios never reached the emulator helpers that US2
and US4 rely on and may not write; US4's health copy must come before the reset check;
and several setups, helpers and races needed naming. The scenarios this revision adds
(US1-S6, US2-S9, US2-S10, US4-S6) were run on the round-2 probe branches and pass, and
US2-S10 fails when the spawn is replaced by an in-line await. Writes come from the task
text. Shared writes name the merge edge that orders them, and the 013, 014, 015 and 017
stories that last wrote the same file on `main`. Nothing in this record is an owner
approval: those are in spec.md, and flipping to `ready` gives them.

Round 3 opened on 2026-11-15 with the owner's answers to the two round-2 questions,
applied here before any probe. Owner decision 4 changed: Stop and the watchdog now reach
a lost or re-initialising board over the last connect's stream, so US2 gains the kept
`stop_board`, the `device` hold and give-way, `SkyWatcher::halt_once`, FR-018 and three
scenarios (US2-S11, US2-S12 and US2-S13), and US2-S9's Stop now answers true. Owner approval 6
closes two gaps: US3 gains GoTo's read after each correction's plan and the count in
tracking's `at_rest`, and two scenarios (US3-S7, US3-S8). No story's writes changed.
Those parts are written from the code at c1ed295 and were not in the round-2 probes.

Round 3's probes ran the same night (2026-11-14, local time): each story was probe-built
again in a throwaway worktree from the trio with those answers, stacked as before. Every
probe passed its story's tests, its crates' tests, clippy and fmt, with no undeclared
write and no existing test touched, in 19.7 to 43.1 KB of diff, and every signature in
§ Interfaces compiled as given but `MountReading`'s field visibility. Mutations showed
each new part needed: without the kept-stream Stop US2-S9, US2-S11, US2-S12 and US2-S13
fail, without one transmission a command US2-S12, without GoTo's read after a
correction's plan US3-S7, and without `at_rest`'s count US3-S8. Those probes and two
reviews showed what this revision fixes: US2-S11 and US2-S12 called Stop before any
connect attempt held the device, so they passed without the give-way and now wait 0.1 s;
US2-S13 now builds its service once the link is lost and checks that it is; a Stop before
the first connect no longer makes that connect give way; a `stopping` count closes the
window in which an attempt on the node's other worker thread could take the device just
after a Stop's notification; US1-S3 now checks that Stop answers at once and sends
nothing, and US3-S8 that no waypoint follows the reset; the health copy comes before the
loss check too; § Read from the diff lists the paths no scenario reaches; Prerequisite 5
is met by the owner's order; and `main` moved to 1104b7d, whose `test` gate runs
nextest and whose 027 must land first (Prerequisite 6). The changed scenarios and Stop
were run on copies of the round-3 probe branches: all US1 and US2 tests pass under
`cargo test` and `cargo nextest`, as do US3's and US4's, and without the give-way
US2-S9, US2-S11, US2-S12 and US2-S13 fail. One owner question was then open, gap (d)
under spec.md owner approval 6, whose recommended default left it open.

Refreshed on 2026-11-15, once 027 had landed: `main` at b8dceaa was merged into this
branch, 027's move map was applied to every finder test path, and every citation in the
trio was re-read at b8dceaa. Of the files this trio cites or writes, only those under
`crates/finder/tests` had moved, and each cited line still says what the trio says of
it. US3's test became a module of finder's `motion` binary, which adds
`crates/finder/tests/motion/main.rs` to US3's writes, shared with 027's US2, and owner
approval 7 for its one `mod` line; US3's development check became the nextest filter for
that module, and 027's layout guard joined US3's preserved tests. The same day the owner
closed gap (d) under owner approval 6 ("Close it"): tracking's baseline is the session's
last-seen count when it begins, handed over by `Session::begin_tracking` through one new
method, `Tracker::start_with_resets`, within US3's writes, with FR-015, the edge case,
T019 and T020 to match and one scenario, US3-S9. The round-3 probes were built on the old
layout. Round 4 (2026-11-15) probe-built US3 again on 027's layout, on US1 and US2 as
round 3 built them, with the new baseline: all nine US3 tests, the layout guard and the
workspace's tests passed, clippy and fmt were clean, and the diff was 46.5 KB. With
`start_with_resets` ignoring its count, US3-S9 failed; with `begin_tracking` calling
`start(target)`, all nine passed, which is why that hand-over is read from the diff.

```yaml
version: 1
reviewed_revision: b8dceaa9370a77e7012dd91debc2bef192c411b1
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
        assertion: "the service over a disconnected link describes no axes, refuses axis_state, goto_axis and rate_axis with mount not connected, answers stop_all UNAVAILABLE at once with no stop sent, and frames carry no counts"
        decision: "spec.md:US1-S3"
      - path: crates/scope-node/tests/mount_link.rs
        status: new
        disposition: add
        assertion: "the binary with a missing mount device keeps running and logs mount not connected"
        decision: "spec.md:US1-S5"
      - path: crates/scope-node/tests/mount_link.rs
        status: new
        disposition: add
        assertion: "on a bare SkyWatcher, set_position reads back, an injected reply is recorded and survives a power cycle, and the cycled board reads not initialised at position 0 with its encoders on again"
        decision: "spec.md:US1-S6"
      - path: crates/scope-node/tests/skywatcher.rs
        status: existing
        disposition: preserve
        assertion: "SkyWatcher::connect's exchanges, probe()'s order and the lost-telemetry rule are unchanged"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/motion/mount_calls.rs
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
      - path: crates/scope-node/tests/mount_reset.rs
        status: new
        disposition: add
        assertion: "a failed re-initialisation drops moves at once, gives the device up to a Stop the board acknowledges, and is counted once by the connect task's next connect; a read dropped at 200 ms does not cancel the re-initialisation"
        decision: "spec.md:US2-S9"
      - path: crates/scope-node/tests/mount_reset.rs
        status: new
        disposition: add
        assertion: "over a lost link to a powered board running a J move, Stop makes the connect attempt under way give way, sends K1 and K2 over the last connect's stream, is acknowledged, and the axis comes to rest; the node's StopAll over such a link answers OK"
        decision: "spec.md:US2-S11"
      - path: crates/scope-node/tests/mount_reset.rs
        status: new
        disposition: add
        assertion: "over a lost link to a powered-off board, Stop sends K1, K2, L1 and L2 once each and answers false within 1.5 s"
        decision: "spec.md:US2-S12"
      - path: crates/scope-node/tests/skywatcher_stop.rs
        status: existing
        disposition: preserve
        assertion: "SkyWatcher's stop_confirmed on a connected board keeps 017's halt with the transport's retries"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "the first connect after start is not a reset"
      - "watchdog_stops is unchanged"
      - "with a board connected, Stop is SkyWatcher's 017 halt with five tries a command"
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
      - path: crates/finder/tests/motion/mount_identity.rs
        status: new
        disposition: add
        assertion: "describe_mount carries firmware 3.40 and board 0x90; the session shows them and omits resets at 0"
        decision: "spec.md:US3-S1"
      - path: crates/finder/tests/motion/mount_identity.rs
        status: new
        disposition: add
        assertion: "after a reset GoTo, second sync and Track answer 409 with the power-cycled words and no GotoAxis reaches the node, until a sync from a later frame"
        decision: "spec.md:US3-S3"
      - path: crates/finder/tests/motion/mount_identity.rs
        status: new
        disposition: add
        assertion: "a slewing GoTo ends failed with power-cycled and sends StopAll; running tracking ends off with power-cycled"
        decision: "spec.md:US3-S5"
      - path: crates/finder/tests/motion/mount_identity.rs
        status: new
        disposition: add
        assertion: "a reset seen while a GoTo corrects ends it failed with power-cycled and sends StopAll with no further GotoAxis; a reset during a keyhole slew ends tracking off with power-cycled and sends no further waypoint"
        decision: "spec.md:US3-S7"
      - path: crates/finder/tests/motion/mount_identity.rs
        status: new
        disposition: add
        assertion: "a tracker given the session's last-seen count 0 while the node already reports 1 ends off with power-cycled at its first read, sends StopAll and sends no RateAxis"
        decision: "spec.md:US3-S9"
      - path: crates/finder/tests/motion/goto/refusals.rs
        status: existing
        disposition: preserve
        assertion: "GoTo's existing refusals and their words are unchanged"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/motion/stop/watchdog.rs
        status: existing
        disposition: preserve
        assertion: "a GoTo still aborts when the node's watchdog stops the mount"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/motion/goto/sim.rs
        status: existing
        disposition: preserve
        assertion: "GoTo's numbered corrections converge unchanged with no reset"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/motion/tracking/keyhole.rs
        status: existing
        disposition: preserve
        assertion: "the keyhole's stop, slew, wait and resume are unchanged with no reset"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no test file at the top level of crates/finder/tests, the same helper files under support/, and only mod lines after the support tree in motion/main.rs"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "GET /api/session with no reset and a node with no identity is byte-for-byte unchanged"
      - "the 014 model's epochs are unchanged"
      - "MCP direct axis moves are not refused after a reset (owner decision 2)"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test motion mount_identity::"
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
      - crates/finder/tests/motion/main.rs
      - crates/finder/tests/motion/mount_identity.rs
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
      - path: crates/finder/tests/motion/main.rs
        other_spec: 027-finder-tests-by-system
        other_story: US2
        disposition: merge_order
        reason: "024 depends_on_landed 027, whose US2 created motion/main.rs and last changed it; US3 adds one mod line"
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
      - path: crates/scope-node/tests/mount_garble.rs
        status: new
        disposition: add
        assertion: "the read that finds a reset still counts its wrong-length reply, and the old board's GoTo path stands until the new board's first GoTo"
        decision: "spec.md:US4-S6"
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
