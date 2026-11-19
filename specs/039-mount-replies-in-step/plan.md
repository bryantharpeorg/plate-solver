# Implementation Plan: The Mount's Replies Stay in Step

**Branch**: `039-mount-replies-in-step` | **Date**: 2026-11-19 | **Spec**: [spec.md](spec.md)

This plan records every design decision for the feature. A story that would change one
escalates rather than re-deciding it.

## Summary

Two changes inside the scope node's Sky-Watcher mount adapter, and tests of both under
the console.

The transport learns that an exchange can be cut short. It remembers the frame it sent
until that frame's reply has been read. Before each transmission it reads a reply still
owed off the line, waiting no longer than what is left of that reply's own 300 ms, and
then discards whatever input has arrived. A left-over reply can then never answer the
next command, so one late reply, one read the console gave up on, or one Stop that
cancelled a move no longer leaves the line out of step.

The protocol stops treating a spoiled acknowledgement of a motion start as a failed
start. It reads the axis's status once; a running axis took the start. The start is
still sent once.

Both events are counted in a new `LineHealth` beside 024's `MountHealth` and logged with
the command and the axis. No RPC, no message, no console file and no part of the mount
port changes.

## Technical Context

**Language/Version**: Rust, MSRV 1.96, stable in CI.

**Primary Dependencies**: nothing new. `rustix` already has `termios` and `fs`; the
discard uses `rustix::io::read`, as `read_byte` does.

**Storage**: none. Line health lives for the node's process, as 024's health does.

**Testing**:
- US1: `crates/scope-node/tests/mount_step.rs` (new), over the emulator behind the new
  test line, and one new test in the `tests` module of `serial.rs`, over a socket pair.
- US2: `crates/scope-node/tests/mount_start.rs` (new), over the same rig.
- US3: `crates/finder/tests/motion/tracking/mount_line.rs` (new), a module of finder's
  `motion` test binary: the console's `Tracker`, `LinkManager` and `MountClient` over
  the in-memory link, the node's real service, `MountLink` and adapter, and the
  emulator.
- All under `#[tokio::test(start_paused = true)]` but the `serial.rs` test, which runs
  on the real clock as the test beside it does.
- No Vitest or Playwright test: the page is unchanged.

**Target Platform**: `scope-node` on the Pi. `finder` is touched only in its tests.

**Constraints**:
- `Transport::new` and `Transport::command` keep their signatures; existing tests call
  both (§ Shapes that existing tests pin).
- `MountHealth`, `EmulatorConfig`, `Exchange`, `AxisState` and the Sky-Watcher `Error`
  gain no field and no variant: tests build or match them.
- `SkyWatcher::connect` and `SkyWatcher`'s `Mount` methods keep their signatures, and
  their exchanges when every reply is valid and in time (024's FR-004).
- `RUSTFLAGS="-D warnings"`, `--all-features` clippy, and rustdoc with warnings denied.
- No test moves real hardware.

**Scale/Scope**: one adapter, three stories, six source files, three new test files.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Nothing touches the solver, star detection or `crates/math-core`. The behaviour is grounded in Sky-Watcher's motor-controller command set as the adapter already speaks it, in 024's requirements, and in what the probe of 2026-11-19 ran on the emulator; the parts that rest on reading are listed in § What was run and what was read, and none is invented beyond them. |
| II. Never weaken a gate | No existing test, golden, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS` or `ergane.yaml` is edited. New tests go in new files, but for one new function in the `tests` module of `serial.rs`, beside a test that is not edited (spec owner approval 6), and one `mod` line (spec owner approval 5). § Shapes that existing tests pin names every existing test file near the change and says what each story does to what it observes. |
| III. Tests first, independent expectations | The oracles are the emulator's own record of what the board was sent and what it answered, the board's counts, and fixed numbers in the scenarios; never the transport's or the protocol's own arithmetic. Time is paused Tokio time, supplied to the board, the node and the console through their clocks. The one test on the real clock asserts no duration. |
| IV. Critical paths | No `unsafe`, no dependency change, no `Cargo.toml` or `Cargo.lock` change, nothing under `proto/`, no `build.rs`. The serial shell stays safe `rustix` calls. |
| V. Proven open source first, honest licensing | Nothing is designed that existing software could supply: the change is two rules inside this project's own adapter for one vendor's serial protocol. The adapter's own sources, Sky-Watcher's motor-controller command set and `docs/research/2026-10-21-synscan-mount-control.md`, are not changed. No reused code, no new license. |

**Mount neutrality.** The mount port, `crates/scope-node/src/mount/mod.rs`, is not
touched: the console still sees two axes that report, go to a position, move at a rate
and stop. Every change sits below it, in `crates/scope-node/src/mount/skywatcher/`.
Nothing here reads an altitude, an azimuth or a mount type; the same adapter on an
equatorial mount with this board keeps its line in step the same way. A second mount
adapter with its own serial line would need its own transport rule; `LineHealth` is the
Sky-Watcher link's, not the port's.

## Project Structure

```text
crates/scope-node/
├── README.md                                 # US1: the line; US2: the start
├── src/mount/skywatcher/
│   ├── transport.rs                          # US1: the open exchange, read-out, discard, stale count
│   ├── serial.rs                             # US1: discard_input, and one new test
│   ├── emulator.rs                           # US1: Emulator::discard_input, Line, LineHandle
│   ├── protocol.rs                           # US1: LineHealth, SkyWatcher::line_health; US2: start, the counters, the log lines
│   └── mount_link.rs                         # US1: line_health, its copy and fold; US2: two more fields in the fold
└── tests/
    ├── mount_step.rs                         # US1, new
    └── mount_start.rs                        # US2, new
crates/finder/tests/motion/tracking/
├── mod.rs                                    # US3: one mod line
└── mount_line.rs                             # US3, new
```

## Interfaces

### The byte stream (`crates/scope-node/src/mount/skywatcher/transport.rs`, US1)

```rust
pub trait BaudStream: Send {
    fn set_baud(&mut self, baud: u32) -> io::Result<()>;
    /// Take every byte that has arrived and was not read off the stream, and return
    /// them in order. It never waits.
    fn discard_input(&mut self) -> io::Result<Vec<u8>>;
    fn write(&mut self, bytes: &[u8]) -> impl Future<Output = io::Result<()>> + Send;
    fn read_byte(&mut self) -> impl Future<Output = io::Result<u8>> + Send;
}
```

The three implementations:

- **`Emulator`** (`transport.rs`, beside its other methods): `Ok(self.discard_input())`,
  the inherent method below, as `set_baud` there calls the inherent `set_baud`.
- **`Serial`** (`serial.rs`): reads with `rustix::io::read` into a 256-byte buffer until
  the device answers `AGAIN`, a read of 0 bytes, or the buffer is full; `INTR` is tried
  again; any other error is returned. It returns the bytes read. It is not `async` and
  never sleeps. `set_baud` keeps its `tcflush`.
- **`Line`** (`emulator.rs`): below.

### The transport (`crates/scope-node/src/mount/skywatcher/transport.rs`, US1)

```rust
pub struct Transport<S> {
    stream: S,
    /// The frame of an exchange that was begun and not finished: when it was sent, and
    /// its bytes. Its reply may still be on the line or on its way.
    open: Option<(tokio::time::Instant, Vec<u8>)>,
    stale_replies: u64,
    last_stale: Option<String>,
}

impl<S: BaudStream> Transport<S> {
    pub fn new(stream: S) -> Self;                       // signature unchanged
    pub async fn command(&mut self, command: u8, axis: u8, data: &[u8], safe: bool)
        -> Result<Vec<u8>, Error>;                       // signature unchanged
    pub fn set_baud(&mut self, baud: u32) -> Result<(), Error>;  // also closes an open exchange
    /// Transmissions that stale input preceded, and the last such bytes, escaped.
    pub fn stale(&self) -> (u64, Option<String>);
}
```

`command` builds the frame and loops over its tries as today
(`crates/scope-node/src/mount/skywatcher/transport.rs:43`). The one change in the loop
is a first step in every try:

```rust
for attempt in 0..tries {
    self.clear_line(command, axis).await?;
    let result = tokio::time::timeout(REPLY_TIMEOUT, self.exchange(&frame))
        .await
        .unwrap_or(Err(Error::Timeout));
    // ... as today
}

/// Take a reply still owed off the line, then whatever has arrived.
async fn clear_line(&mut self, command: u8, axis: u8) -> Result<(), Error> {
    let mut stale = Vec::new();
    if let Some((sent, frame)) = self.open.clone() {
        let deadline = sent + REPLY_TIMEOUT;
        if let Ok(Ok(line)) = tokio::time::timeout_at(deadline, self.line(&frame)).await {
            stale = line;
        }
        self.open = None;
    }
    stale.extend(self.stream.discard_input()?);
    if !stale.is_empty() {
        let escaped = stale.escape_ascii().to_string();
        eprintln!("mount stale input before :{}{axis}: {escaped}", char::from(command));
        self.stale_replies += 1;
        self.last_stale = Some(escaped);
    }
    Ok(())
}

async fn exchange(&mut self, frame: &[u8]) -> Result<Vec<u8>, Error> {
    self.open = Some((tokio::time::Instant::now(), frame.to_vec()));
    self.stream.write(frame).await?;
    let line = self.line(frame).await;
    self.open = None;
    codec::reply(&line?)
}

/// One line off the stream, its carriage return included. A leading line feed and an
/// echo of `frame` are skipped, and a line over 16 bytes is `Error::InvalidReply`.
async fn line(&mut self, frame: &[u8]) -> Result<Vec<u8>, Error>;
```

`line` is today's loop (`crates/scope-node/src/mount/skywatcher/transport.rs:57`) with
its last step, `codec::reply`, moved to `exchange`. The rules this code carries, each of
which a scenario or an existing test holds:

- **The exchange is open on the transport, not in the future.** A caller that goes away
  drops `exchange`'s future between the write and the read, or in the middle of the
  read; `open` stays set because nothing in a dropped future runs. The 300 ms
  `timeout` drops it the same way. So all three routes of spec Clarifications leave an
  open exchange (US1-S1, S3, S5).
- **The deadline is the abandoned frame's own.** The read-out stops at `sent + 300 ms`,
  not 300 ms after it began. After a try that timed out, the deadline has passed: the
  read-out adds no wait, and five tries still take 1.5 s
  (`crates/scope-node/tests/skywatcher.rs:268` allows 10 ms; US1-S2).
- **`timeout_at` polls its future before its deadline.** With the deadline passed, a
  reply line that has wholly arrived is still read by that one poll, and anything else
  that has arrived is taken by the discard that follows. FR-004 counts once for the
  transmission and keeps the bytes of both in order, so the count and the bytes are the
  same whichever of the two took them (US1-S2's 25 bytes).
- **A dropped `clear_line` loses nothing.** `open` is cleared only after the read-out
  has ended, so a caller that goes away while the node waits leaves the exchange open
  with its first deadline, and the next transmission waits on to that same moment.
- **A partial line is taken but not kept.** Bytes of a left-over line that had not all
  arrived when the read-out ended are consumed and dropped; they are stale either way.
- **Nothing else changes.** No extra transmission, no wait when no exchange is open and
  nothing has arrived. `set_baud` sets `open` to `None` before changing the baud: the
  device is flushed there and the old reply is not owed any more.

### The emulator and the test line (`crates/scope-node/src/mount/skywatcher/emulator.rs`, US1)

```rust
impl Emulator {
    /// Take the reply bytes that were produced and never read, as reading the serial
    /// input queue dry does. Not a baud change: nothing is recorded for it.
    pub fn discard_input(&mut self) -> Vec<u8>;
}

/// A serial line in front of the emulator, for tests: it can deliver one reply late
/// and lose one command. Bytes keep their order, and a reply nobody reads stays on
/// the line, as in the kernel's input queue.
pub struct Line {
    board: Emulator,
    faults: LineHandle,
    /// Each byte on its way to the reader, with the moment it arrives.
    coming: VecDeque<(tokio::time::Instant, u8)>,
}

#[derive(Clone, Default)]
pub struct LineHandle(Arc<Mutex<LineFaults>>);

impl Line {
    pub fn new(board: Emulator, faults: LineHandle) -> Self;
}

impl LineHandle {
    /// The reply to the next transmission that starts with `command`, written as
    /// ":f1", reaches the reader `by` later than it otherwise would. One use each.
    pub fn delay_next(&self, command: &str, by: Duration);
    /// The next transmission that starts with `command` never reaches the board: it is
    /// not carried out, not answered and not recorded. One use each.
    pub fn lose_next(&self, command: &str);
}
```

`Line` implements `BaudStream` in `emulator.rs`, where it can reach the emulator's own
reply buffer. This is the line the probe ran, moved into the library so that both
crates' tests share one copy; in this place it reads the emulator's buffer directly:

```rust
impl BaudStream for Line {
    fn set_baud(&mut self, baud: u32) -> io::Result<()> {
        self.arrived(); // a flush takes what has arrived, not what is on its way
        self.board.set_baud(baud)
    }
    fn discard_input(&mut self) -> io::Result<Vec<u8>> {
        Ok(self.arrived())
    }
    async fn write(&mut self, bytes: &[u8]) -> io::Result<()> {
        let now = tokio::time::Instant::now();
        let delay = {
            let mut faults = self.faults.0.lock().unwrap();
            if let Some(i) = faults.lost.iter().position(|c| bytes.starts_with(c)) {
                faults.lost.remove(i);
                return Ok(());
            }
            match faults.late.iter().position(|(c, _)| bytes.starts_with(c)) {
                Some(i) => faults.late.remove(i).1,
                None => Duration::ZERO,
            }
        };
        self.board.write_all(bytes).await?;
        // The emulator answers inside the write. Its reply reaches the reader `delay`
        // later, and never before the bytes ahead of it.
        let mut due = now + delay;
        if let Some((last, _)) = self.coming.back() {
            due = due.max(*last);
        }
        let reply: Vec<u8> = self.board.output.drain(..).collect();
        self.coming.extend(reply.into_iter().map(|byte| (due, byte)));
        Ok(())
    }
    async fn read_byte(&mut self) -> io::Result<u8> {
        let Some(&(due, _)) = self.coming.front() else {
            // Nothing is coming until the next transmission.
            return std::future::pending().await;
        };
        tokio::time::sleep_until(due).await;
        Ok(self.coming.pop_front().expect("a byte was waiting").1)
    }
}
```

`arrived` removes and returns, in order, every byte of `coming` whose moment is now or
earlier. `LineFaults` holds `late: Vec<(Vec<u8>, Duration)>` and `lost: Vec<Vec<u8>>`.
`Line` and `LineHandle` are test tooling in the emulator's source, as its handle's
`inject_reply` is; no existing test uses them, and the `Emulator` type behaves as it
does today for every existing caller. `EmulatorHandle` gains nothing.

### Line health (`protocol.rs` and `mount_link.rs`, US1; two fields more in US2)

```rust
/// What the serial line did that the console is not told.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LineHealth {
    /// Transmissions before which input left over from an earlier exchange was taken
    /// off the line (US1).
    pub stale_replies: u64,
    /// The bytes last taken off, escaped as `MountHealth::last_invalid` is (US1).
    pub last_stale: Option<String>,
    /// Motion starts whose acknowledgement was missing, unframeable or carried data,
    /// whatever the status read then found (US2).
    pub unacknowledged_starts: u64,
    /// Rate and GoTo commands that failed on the node, after which it sent the stop to
    /// both axes (US2).
    pub halted_moves: u64,
}
```

- US1 declares the struct with its first two fields; US2 adds the other two. No test
  builds it by literal (§ Plan traps 5).
- It is declared in `protocol.rs` beside `MountHealth`
  (`crates/scope-node/src/mount/skywatcher/protocol.rs:34`) and re-exported from
  `mount_link.rs` on the line that re-exports `MountHealth`
  (`crates/scope-node/src/mount/skywatcher/mount_link.rs:2`), because `protocol` is a
  private module. Tests import
  `scope_node::mount::skywatcher::mount_link::LineHealth` and
  `scope_node::mount::skywatcher::emulator::{Line, LineHandle}`.
- `SkyWatcher` gains `pub async fn line_health(&self) -> LineHealth`: under the protocol
  lock, the transport's `stale()` and, from US2, the protocol's own two counts.
- `MountLink` gains `pub fn line_health(&self) -> LineHealth`. Its snapshot keeps a
  current and a retired `LineHealth` beside the two `MountHealth`
  (`crates/scope-node/src/mount/skywatcher/mount_link.rs:31`). `copy_health`
  (`crates/scope-node/src/mount/skywatcher/mount_link.rs:106`) copies
  `board.line_health()` under the same condition as `board.health()`, so after every
  delegated `state`, `goto` and `rate`. A new connect folds the current into the
  retired and clears the current, where it does so for `MountHealth`
  (`crates/scope-node/src/mount/skywatcher/mount_link.rs:207`). The fold sums the three
  counts and prefers the current board's `last_stale`, else the retired one.
- `MountLink::stop` is not changed. Stale input found by a Stop is counted on the
  transport at once and shows in `MountLink::line_health()` after the next read or
  move, which is where US1-S5 reads it.

### The start (`crates/scope-node/src/mount/skywatcher/protocol.rs`, US2)

```rust
/// Send the start once, never again (024's FR-016). When its acknowledgement is
/// missing or garbled, one status read decides: a running axis took the start.
async fn start(&mut self, axis: u8) -> Result<(), Error> {
    let error = match self.ack(b'J', axis, b"").await {
        Err(error @ (Error::Timeout | Error::InvalidReply)) => error,
        result => return result,
    };
    self.line.unacknowledged_starts += 1;
    match self.status(axis).await {
        Ok(status) if status.moving => {
            eprintln!("mount start :J{axis} not acknowledged ({error}): the axis is running");
            Ok(())
        }
        Ok(_) => {
            eprintln!("mount start :J{axis} not acknowledged ({error}): the axis is not running");
            Err(error)
        }
        Err(unreadable) => {
            eprintln!("mount start :J{axis} not acknowledged ({error}): its status could not be read");
            Err(unreadable)
        }
    }
}
```

- `rate` and `goto` end with `self.start(axis).await` in place of
  `self.motion(b'J', axis, b"").await`
  (`crates/scope-node/src/mount/skywatcher/protocol.rs:312`,
  `crates/scope-node/src/mount/skywatcher/protocol.rs:360`). For `:J`, `motion` is
  `ack` and nothing more (`crates/scope-node/src/mount/skywatcher/protocol.rs:243`), so
  nothing is lost.
- `Error::Timeout` is no reply in 300 ms; `Error::InvalidReply` is a line that cannot be
  framed, a line over 16 bytes, or a framed reply that carries data
  (`crates/scope-node/src/mount/skywatcher/protocol.rs:223`). `Error::Refused` and
  `Error::Io` return as today.
- `status` is 024's retried read
  (`crates/scope-node/src/mount/skywatcher/protocol.rs:207`): up to five transmissions,
  wrong-length replies counted in `invalid_replies` as ever.
- **`moving` alone is the test, on purpose.** `goto` brings a moving axis to rest before
  it starts (`crates/scope-node/src/mount/skywatcher/protocol.rs:315`), and so does
  `rate` for every start but one: a slow rate in the direction the axis already tracks
  (`crates/scope-node/src/mount/skywatcher/protocol.rs:291`). So an axis found moving
  after the start either took it, or was already running the way it was asked to. No
  check of direction or mode is added.
- `Protocol` gains `line: LineHealth` for its two counts.
- `log_motion` (`crates/scope-node/src/mount/skywatcher/protocol.rs:479`) takes the
  command's name and its axis. On an error it adds 1 to `halted_moves`, logs
  `mount rate on axis 1 failed (<error>): stopping both axes` (or `goto`, or axis 2),
  and sends `halt` as today. `Mount::goto` and `Mount::rate` pass `"goto"` and `"rate"`.
  The error is the one the command returned; `mount stop failed: <error>` still follows
  when the stop itself fails.

### `crates/scope-node/README.md` (US1, US2)

Both stories write under `## The mount link`, after the paragraph that ends "A motion
start (`:J`) is always sent once, even when its acknowledgement is lost."

- **US1**: a paragraph on the line in step. Before each transmission the node reads a
  reply still owed by an exchange that was cut short, for at most what is left of its
  300 ms, then discards any input that has arrived; what this costs Stop; that such
  input is counted in `MountLink::line_health().stale_replies`, kept escaped as
  `last_stale`, and logged, with the line of FR-004 in a `text` block as the README
  shows its other lines.
- **US2**: a paragraph on the start. An acknowledgement that is missing or garbled is
  followed by one status read; a running axis is accepted and nothing is stopped;
  otherwise the command fails and both axes are stopped. `unacknowledged_starts` and
  `halted_moves`, the three lines of FR-012 and the line of FR-013, and that the node
  still answers the console OK.

## What was run and what was read

**Run** on 2026-11-19, on the emulator in paused time, three times each with identical
output, on a branch of `main` at 6a519d49; the adapter's files are the same at
6e506ec6:

- today's behaviour of both faults, on the node's service and under the console's
  tracker (spec Clarifications);
- three trial fixes: the status read after an unacknowledged start; a discard of
  arrived input before each transmission; the read-out of an abandoned exchange. With
  all three, every test of `scope-node` (161) and of finder's `motion` binary (127)
  passed unchanged;
- with the fixes, every observation the scenarios of US1-S1 to US1-S7, US2-S1 to US2-S7
  and US3-S1 to US3-S4 state about answers, times, exchanges, positions, `moving`,
  `invalid_replies`, `connected` and `resets`;
- a GoTo under each fault, with the fixes and without (US1-S7, US2-S5 to US2-S7). On
  `main`: a spoiled `:J1` left azimuth at 0 or 1,215 counts of 90,000; a spoiled `:J2`
  stopped both axes short; with the line out of step no `:S` and no `:J` reached the
  board.

**Read, not run**:

- `LineHealth`, its four values in every scenario, and its copy and fold: the probe's
  discard counted bytes and its trial fixes counted nothing else;
- the three log lines of FR-004 and FR-012 and the line of FR-013: the probe logged
  `mount start on axis <n> not acknowledged (<error>): the axis is running` and kept
  `mount command failed`;
- `discard_input` returning its bytes, and `Serial` reading in place of the probe's
  `tcflush(TCIFLUSH)`, which was compiled and never run (US1-S8 is new);
- `Line` in the library reading the emulator's buffer: the probe's line was in its test
  files and polled the emulator's reads;
- `clear_line` as one function: the probe had the read-out and the discard as two calls
  in the same order;
- US1-S6's two added assertions (both axes at rest after the Stop, `stale_replies` 0),
  and that US2-S4's call answers after 300 ms, which is why that scenario states no
  time;
- that no test outside `scope-node` and finder's `motion` binary reaches the adapter:
  a search of the workspace finds the emulator and `MountLink` only in the files
  § Shapes names.

**Not run at all**: a real serial device, the real board, the camera loop, the page; a
console GoTo, whose wait for rest ends on one reading of both axes at rest
(`crates/finder/src/goto.rs:947`); two left-over replies still on their way; the
back-to-back case of spec Edge Cases.

## Read from the diff, not from a test

- **The four log lines.** No test reads the node's standard error in process. Copy them
  from FR-004, FR-012 and FR-013, character for character.
- **`Serial::discard_input` on a serial device.** US1-S8 runs it over a socket pair. On
  a tty it relies on the same `AGAIN` that `read_byte` already relies on
  (`crates/scope-node/src/mount/skywatcher/serial.rs:69`).
- **The retired fold of `LineHealth`.** No scenario reconnects after counting. It is the
  shape of `combined_health`
  (`crates/scope-node/src/mount/skywatcher/mount_link.rs:140`).
- **The status read that fails after an unacknowledged start** (FR-011's second case),
  and FR-012's third line.
- **A caller that goes away during the read-out.** No scenario drops a call while the
  node waits for a left-over reply.

## Shapes that existing tests pin

Every existing test passes unchanged with all three stories built. The files near the
change, and what each story does to what they observe:

- **`crates/scope-node/tests/skywatcher.rs`**. `transport_timeouts_and_replay_safety`
  (`crates/scope-node/tests/skywatcher.rs:258`) builds a `Transport` with
  `Transport::new`, calls `command` with four arguments, and asserts that four dropped
  replies cost 1.2 s within 10 ms, five transmissions and no more, and one `:J`. US1
  keeps both signatures, adds no transmission, and adds no wait after a try that timed
  out. `connection_and_motion_failures`
  (`crates/scope-node/tests/skywatcher.rs:324`) still sees `:K2` after a rate that is
  not finite: US2 changes that failure's log line and count, not its stop.
- **`crates/scope-node/tests/skywatcher_stop.rs`**. 017's halt on a connected board:
  `:K1`, `:K2`, `:L` after a silent `:K`, five tries a command. No exchange is open
  when these stops begin, so US1 adds no wait and no byte.
- **`crates/scope-node/tests/mount_link.rs`**. Its last test
  (`crates/scope-node/tests/mount_link.rs:239`) builds a `Transport` over
  `handle.reopen`, asserts `handle.baud_attempts()` is empty, and reads replies after a
  command that timed out. `Emulator::discard_input` records no baud attempt, and the
  read-out after that timed-out command finds nothing and waits for nothing.
- **`crates/scope-node/tests/mount_reset.rs`**. Stop over a lost link within 1.2 s
  (`crates/scope-node/tests/mount_reset.rs:327`), within 100 ms while a connect holds
  the device (`crates/scope-node/tests/mount_reset.rs:296`) and within 10 ms during a
  re-initialisation (`crates/scope-node/tests/mount_reset.rs:232`). `halt_once` sets the
  baud first, which closes any open exchange, and each of its commands that times out
  leaves a deadline already passed. US1 adds no wait to any of them.
- **`crates/scope-node/tests/mount_garble.rs`**. It builds `MountHealth` by literal
  twice (`crates/scope-node/tests/mount_garble.rs:144`,
  `crates/scope-node/tests/mount_garble.rs:266`) and compares it whole with
  `MountHealth::default()`; `MountHealth` is not changed. With US2,
  `motion_start_with_a_lost_acknowledgement_is_sent_once`
  (`crates/scope-node/tests/mount_garble.rs:121`) puts one more `:f1` on the wire and no
  `:K`; it counts `:J1`, still 1. In `goto_and_rate_copy_health_including_stop_wait_reads`
  (`crates/scope-node/tests/mount_garble.rs:199`) the GoTo whose `:J1` reply is dropped
  is now accepted and keeps slewing; the status read after it gets a valid reply, so
  `invalid_replies` is still 2 there, and the altitude commands that follow count 3 and
  4 as before. `refused_motion_start_is_also_sent_once`
  (`crates/scope-node/tests/mount_garble.rs:190`) is a refusal: no status read.
- **The `tests` modules of `emulator.rs`, `codec.rs` and `serial.rs`.** The emulator's
  two tests use a bare `Emulator`, unchanged. `serial.rs`'s test is not edited; US1 adds
  a function beside it.
- **`crates/finder/tests/motion/mount_identity.rs`**. It builds a `MountLink` over a
  bare `Emulator` (`crates/finder/tests/motion/mount_identity.rs:21`), which still
  implements `BaudStream`.
- **The node's other tests** (`mount_config.rs`, `cli.rs`, the camera and imaging
  files) and **the simulator's tests** do not reach the adapter: no config key, flag or
  port method changes.
- **Every other finder test** drives the simulator's `SimMount` through the mount port
  and never reaches this adapter. `crates/finder/tests/layout/main.rs` holds: US3's
  module sits in `motion/tracking/` with one `mod` line and adds no helper file.
- **US1's `mount_step.rs` is an existing test for US2.** US2 changes nothing it
  observes: its scenarios spoil no start, so `start` behaves as `ack` did, and they read
  `line_health()` by field.

## Decisions and the alternatives rejected

| Decision | Rejected | Why |
|---|---|---|
| Read out an abandoned reply, then discard what has arrived | The discard alone | Run in the probe: a reply still on its way when the next command is sent answers it, and nothing counts the misread (a slewing axis read at rest once; Stop's `:K1` read as unacknowledged). |
| The read-out waits only to the abandoned frame's own deadline | A fresh 300 ms; or wait until the line is quiet | A try that timed out has paid its 300 ms; a fresh wait would double every timeout and break 024's 1.2 s for a silent Stop. A quiet-line wait taxes every exchange. |
| The open exchange is the transport's state | A guard in the exchange's future | The future is what gets dropped. |
| Before every transmission, retries included | Only before the first try | The retry is the transmission most likely to follow a late reply. |
| Stop takes the same path as every command | Stop skips the read-out | Owner decision 1. Skipping it is what makes `:K1` read the left-over. |
| `discard_input` returns the bytes | A flush that returns nothing | The count and the bytes are the only evidence a night's journal will hold of a late reply; 024 kept wrong-length bytes for the same reason. |
| `Serial` reads until `AGAIN`, at most 256 bytes | `tcflush(TCIFLUSH)` | Owner decision 5. A read returns what it took, and runs in the gate over a socket pair, where `tcflush` fails. The bound keeps a line that never stops sending from holding the node in a loop with no await. |
| A `LineHealth` of its own | New fields in `MountHealth` | `mount_garble.rs` builds `MountHealth` by literal twice; a new field would not compile without editing an existing test (024's plan trap 9). |
| Counts per transmission, bytes of both routes together | Separate counts for the read-out and the discard | Which of the two takes an arrived reply depends on a poll order no requirement should fix. |
| `Line` in the library, beside the emulator | A copy in each test file, as the probe had | Two crates' tests need it; in the library it reads the emulator's buffer with no polling trick. |
| A line in front of the emulator | Delays inside `Emulator` | `Emulator`'s read path is what every existing test runs; a wrapper leaves it as tested. |
| `lose_next` on the line | A new emulator fault | The board must not see the command; the line is where it is lost. |
| One status read after an unacknowledged start | Send `:J` again | 024's FR-016; owner decision 3. A relative move started twice travels twice. |
| `moving` alone accepts the start | Check direction and mode too | § The start: the adapter's own sequence already guarantees them. |
| The node still answers OK | An error status | Owner decision 2. |
| US3 only tests | Fold its scenarios into US1 and US2 | Each would then write finder's tests and a `mod` line; kept apart, US1 and US2 touch one crate. |

## User Story 1 - The line

Tests first: `mount_step.rs` with its rig, then the `serial.rs` test. Then
`BaudStream::discard_input` and its three implementations with `Line`; the transport;
`LineHealth` and its two accessors; the README paragraph.

## User Story 2 - The start

Tests first: `mount_start.rs`, with a copy of the rig. Then `start`, the two counts,
`log_motion`, the fold's two fields, the README paragraph.

## User Story 3 - Under the console

`mount_line.rs` and its `mod` line. The scenarios pass on arrival; T013 says what the
probe saw each of them do on `main`, without US1 and US2.

## Verification (every story, before proposing)

- **The factory gates** (`ergane.yaml` at 6e506ec6):
  - `lint` = `cargo fmt --all --check`,
    `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`,
    `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features --locked`
    and `RUSTDOCFLAGS="-D warnings" cargo doc -p finder --no-deps`;
  - `test` = `cargo nextest run --workspace --all-targets --all-features --locked`,
    `cargo test --workspace --locked --doc`, and then, in `crates/finder/web`,
    `npm ci --no-audit --no-fund`, `npm run typecheck`, `npm run format:check`,
    `npm test`, `npm run build`, `git status --porcelain -- dist | (! grep .)` and
    `CI=1 npm run e2e`.

  Every story runs both, and the judge sees their results.
- **CI in the merge queue** runs every required `ci.yml` job.

## Prerequisites before this spec is flipped to `ready`

1. **The earlier specs have landed:** every spec in `depends_on_landed`. Met at
   6e506ec6.
2. **`main` is still as read.** Every citation here was read at 6e506ec6. If `main` has
   moved in the files § Project Structure names or the tests § Shapes names, re-read
   them and re-run the validator.
3. **The owner decisions are settled and the owner approvals given** (spec § Owner
   decisions, § Owner approvals). Not met: all five decisions are open.
4. **Probe builds.** Not met. No story has been built from this text. The parts of
   § What was run and what was read that were only read are what a probe build has to
   show first.
5. **No operator-staged files are needed.**

**Outside the factory: the field check.** The emulator cannot show a real line's
latency, and `Serial::discard_input` has never read a serial device. Before the fix is
relied on at the telescope, with the node rebuilt and installed and the mount powered:

1. With Track on and the page open, let it track for ten minutes. Then press Stop and
   turn Track on again, twenty times, a few seconds apart.
2. After each Stop, see that both axes come to rest and that Track then moves the
   mount: the axis counts in the session advance.
3. Read the node's journal for that stretch and count four things:
   - `mount stale input before` lines. Some after a Stop, as `before :K1`, are the fix
     at work. Their bytes show what was left.
   - `mount reply invalid for` lines. A line out of step shows as one for every read.
     A handful in the stretch is the line's own noise; one per read is a failure.
   - `mount start :J` lines and what each found.
   - `mount rate on axis` and `mount goto on axis` lines: commands the node halted.
   No `mount I/O` line should appear: it would mean the discard's read failed on the
   device.
4. Run one GoTo of more than 10° and see that it ends on its target without a
   correction more than usual.

A failure of 2, or one `mount reply invalid` for every read in 3, means the line is out
of step on the real device: stop, restart the node, and report the journal.

## Plan traps

They bind every story.

1. **Gate evidence or nothing.** The judge credits only gate results and the story's
   diff. *Check:* every Then names a committed test the `test` gate runs.
2. **One evidence budget.** *Check:* a story's diff stays under 64 KiB, measured with
   `git diff <base> -- . ':(exclude)crates/finder/web/dist' | wc -c`. Estimates from
   the probe (its three fixes came to 6.9 KB of source in all; its test files, with
   their printing and their two behaviours, to 46.8 and 20.2 KB): US1 about 33 KB, US2
   about 18 KB, US3 about 15 KB.
3. **A spec fixed mid-epic needs a relaunch.** *Check:* no builder edits
   `specs/039-mount-replies-in-step/` or `.specify/`.
4. **An earlier story's test file is an existing test.** *Check:* US2 writes
   `mount_start.rs` and never `mount_step.rs`; it copies the rig. US3 writes neither.
5. **Struct literals break on a new field.** *Check:* no test builds `LineHealth` by
   literal; tests read its fields, or compare it with `LineHealth::default()`. Nothing
   is added to `MountHealth`, `EmulatorConfig`, `Exchange`, `AxisState` or `Error`.
6. **The deadline is the abandoned frame's.** A read-out that waits a fresh 300 ms
   fails US1-S2 (1,500 ms) and US1-S6 (280 ms), and with them
   `transport_timeouts_and_replay_safety` and the 1.2 s Stop of `mount_reset.rs`.
   *Check:* `timeout_at(sent + REPLY_TIMEOUT, ...)`.
7. **The open exchange must outlive its caller.** *Check:* `open` is a field of
   `Transport`, set before the write and cleared only after the reply's line is read or
   the read-out has ended.
8. **Discard only what has arrived.** A discard that also drops bytes still on their
   way is not what a serial device does, and hides the case the read-out exists for.
   *Check:* `Line::discard_input` and `Line::set_baud` use `arrived`; otherwise US1-S1's
   first read answers after 300 ms, not 400.
9. **Paused time and held replies.** A held reply arrives only when the test sleeps or
   awaits a call. *Check:* every scenario states its waits, and every time it asserts
   is exact on paused time.
10. **The wire must be the test's alone.** The capture loop reads both axes for every
    frame, and the node's watchdog stops the mount 1 s after the last heartbeat.
    *Check:* the node rig stops frames and beats every 200 ms; the tracker rig stops
    frames.
11. **An exchange list is exact.** *Check:* each list in a scenario is compared whole,
    by the first three characters of each command, and US1-S7's list begins with the
    abandoned read's own `:f1`.
12. **A negative assertion passes when nothing happens.** *Check:* every "no `:K`"
    stands beside a positive in the same test: the `:J` count, both axes `moving`, or
    the final positions.
13. **An oracle copied from the code proves nothing.** *Check:* expected values are the
    scenarios' own numbers and the emulator's record; no test calls `Transport::stale`
    or the codec to get one.
14. **An unused helper fails clippy.** *Check:* each test file holds only the helpers
    its own tests use.
15. **A new finder test joins `motion`.** A file at the top of `crates/finder/tests` or
    under `support/` fails the `layout` test. *Check:* US3 adds `mount_line.rs` in
    `motion/tracking/` and one `mod` line.
16. **rustdoc denies warnings.** An angle bracket in a doc comment outside backticks is
    read as HTML. *Check:* `<error>`, `<axis>` and the like appear in doc comments only
    inside backticks.
17. **Each worktree builds on its own.** *Check:* no story sets `CARGO_TARGET_DIR`.
18. **The log lines are in no test.** *Check:* the four `eprintln!` strings are the
    FRs', compared by eye before proposing.
19. **Stop's path in the link is not touched.** *Check:* `MountLink::stop` and
    `SkyWatcher::halt_once` are byte for byte as on `main`; US1-S5 reads `line_health()`
    after later reads have copied it.

## Risks

| Risk | Mitigation |
|---|---|
| On the real device a reply later than 300 ms is followed at once by another command, and the line is out of step for a few exchanges (spec Edge Cases). | It ends at the first pause, where the discard takes the left-over; every such exchange is counted and logged. The field check counts them. |
| Stop is held up to 300 ms behind a lost reply. | Owner decision 1. It needs Stop to cancel a move part-way and that move's reply to be lost; the console's second attempt is answered. |
| The real board answers a status read "moving" for an axis whose start it did not take. | Only on the same-direction slow-rate path, where the axis runs the way it was asked (§ The start). |
| `Serial::discard_input` behaves differently on a tty than on a socket. | It relies on the `AGAIN` `read_byte` already relies on; the field check looks for `mount I/O`. |
| A status read is trusted after an unacknowledged start while the line is out of step. | US1 lands first; US2 depends on it. |
| The node's log grows: a line for every stale input. | One line per event, and an event needs an exchange cut short or a late reply: a Stop that cancels a move part-way, a read the console gave up on. |
| The form of the stories was not run (§ What was run and what was read). | Prerequisite 4: probe builds before the flip. |

## Refinement Record

Drafted on 2026-11-19 by a plate-solver spec session against `main` at 6e506ec6, from
the two probes of that day, a reading of the adapter, its tests, 024's and 038's trios
and the constitution, and one extension of the probe: a GoTo under each fault, with and
without the trial fixes. Writes come from the task text. Shared writes name the merge
edge that orders them, and the stories that last wrote the same file on `main`.

Nothing in this record is an owner approval or an owner's answer. The five owner
decisions of spec.md are open; `open_questions` is empty below because the text is
written to each recommended answer and no story waits on a fact, not because anything
was settled. No story has been probe-built from this text.

```yaml
version: 1
reviewed_revision: 6e506ec6864fcaa5d5474a203357481dedc5ae81
stories:
  US1:
    tests:
      - path: crates/scope-node/tests/mount_step.rs
        status: new
        disposition: add
        assertion: "a status reply held 400 ms is read by the retry after 400 ms with moving true over the exchanges f1 f1 j1; invalid_replies 0, stale_replies 1, last_stale =111\\r escaped; ten later rounds of reads take 4 exchanges each; a rate call is f1 I1 J1; five rounds of rate pairs answer OK with 10 J and no K or L"
        decision: "spec.md:US1-S1"
      - path: crates/scope-node/tests/mount_step.rs
        status: new
        disposition: add
        assertion: "a status reply held 1,600 ms fails one read after 1,500 ms with moving true; the next read of the axis at rest has moving false, rate 0.0 and position 0; the link stays connected with resets 0 and mount_resets 0; invalid_replies 0, stale_replies 1, last_stale the 25 bytes of five =111\\r"
        decision: "spec.md:US1-S2"
      - path: crates/scope-node/tests/mount_step.rs
        status: new
        disposition: add
        assertion: "after a read given up at 200 ms whose reply arrives at 250 ms, a slewing axis reads moving 100 ms later, as the board itself said, with stale_replies 1 and last_stale =001\\r; read at once, it answers 50 ms later with moving true, and five later rounds read azimuth at rest and altitude moving"
        decision: "spec.md:US1-S3"
      - path: crates/scope-node/tests/mount_step.rs
        status: new
        disposition: add
        assertion: "Stop 20 ms into a rate call whose status reply takes 50 ms: the call fails ABORTED, Stop answers OK after 30 ms over K1 K2 with no L, both axes rest, and five rounds of rate pairs then put 10 J and no K or L on the wire, with stale_replies 1 and last_stale =111\\r; when that reply never comes, Stop answers OK after 280 ms over K1 K2, both axes rest and stale_replies is 0"
        decision: "spec.md:US1-S5"
      - path: crates/scope-node/tests/mount_step.rs
        status: new
        disposition: add
        assertion: "a GoTo of both axes asked at once after a read given up at 200 ms answers OK twice within 50 ms over f1 f1 j1 G1 S1 J1 f2 j2 G2 S2 J2, ends at 90,000 and 45,000 counts with no K or L, invalid_replies 0 and stale_replies 1"
        decision: "spec.md:US1-S7"
      - path: crates/scope-node/src/mount/skywatcher/serial.rs
        status: new
        disposition: add
        assertion: "over a socket pair, discard_input returns the seven bytes waiting, then nothing, then 256 of 300 bytes, then 44, and the next two bytes read are = and a carriage return"
        decision: "spec.md:US1-S8"
      - path: crates/scope-node/tests/skywatcher.rs
        status: existing
        disposition: preserve
        assertion: "four dropped replies cost 1.2 s within 10 ms and five transmissions through Transport::new and command, a start is sent once, and a failed rate still sends K2"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/scope-node/tests/skywatcher_stop.rs
        status: existing
        disposition: preserve
        assertion: "017's halt on a connected board is K1, K2, then L after a silent K, with five tries a command"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/scope-node/tests/mount_link.rs
        status: existing
        disposition: preserve
        assertion: "a Transport over a reopened stream records no baud attempt and reads each reply after a command that timed out"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/scope-node/tests/mount_reset.rs
        status: existing
        disposition: preserve
        assertion: "Stop over a lost link ends within 1.2 s on a silent board, within 100 ms while a connect holds the device and within 10 ms during a re-initialisation"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/scope-node/tests/mount_garble.rs
        status: existing
        disposition: preserve
        assertion: "MountHealth is built by literal and compared whole, and wrong-length replies are counted as before"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/motion/mount_identity.rs
        status: existing
        disposition: preserve
        assertion: "a MountLink over a bare Emulator still connects and names its board"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "Transport::new and Transport::command keep their signatures, and their exchanges and timing when every reply is valid and in time (spec.md FR-008)"
      - "SkyWatcher::connect and its Mount methods keep their signatures (024 FR-004)"
      - "MountHealth, EmulatorConfig, Exchange and Error gain nothing"
      - "Stop over a lost link costs at most four reply timeouts (024 FR-018)"
    checks:
      development:
        - "cargo nextest run -p scope-node --locked --test mount_step"
        - "cargo nextest run -p scope-node --locked --lib serial::"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs, a node rebuilt on the Pi and the field check of plan.md § Prerequisites"
    writes:
      - crates/scope-node/README.md
      - crates/scope-node/src/mount/skywatcher/emulator.rs
      - crates/scope-node/src/mount/skywatcher/mount_link.rs
      - crates/scope-node/src/mount/skywatcher/protocol.rs
      - crates/scope-node/src/mount/skywatcher/serial.rs
      - crates/scope-node/src/mount/skywatcher/transport.rs
      - crates/scope-node/tests/mount_step.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/scope-node/README.md
        other_spec: 039-mount-replies-in-step
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and adds the start's paragraph after the line's"
      - path: crates/scope-node/src/mount/skywatcher/protocol.rs
        other_spec: 039-mount-replies-in-step
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and adds start, two fields of LineHealth and the new log_motion"
      - path: crates/scope-node/src/mount/skywatcher/mount_link.rs
        other_spec: 039-mount-replies-in-step
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and adds two fields to the fold"
      - path: crates/scope-node/README.md
        other_spec: 030-imaging-camera-link
        other_story: US8
        disposition: merge_order
        reason: "039 depends_on_landed 030, whose US8 last changed the node's README (49f8dcd2); 024's US4 last wrote its mount link section"
      - path: crates/scope-node/src/mount/skywatcher/emulator.rs
        other_spec: 024-mount-link-recovery
        other_story: US1
        disposition: merge_order
        reason: "039 depends_on_landed 024, whose US1 last changed the emulator (de9bfe6e)"
      - path: crates/scope-node/src/mount/skywatcher/mount_link.rs
        other_spec: 024-mount-link-recovery
        other_story: US4
        disposition: merge_order
        reason: "039 depends_on_landed 024, whose US4 last changed the link (f0ea2d54)"
      - path: crates/scope-node/src/mount/skywatcher/protocol.rs
        other_spec: 024-mount-link-recovery
        other_story: US4
        disposition: merge_order
        reason: "039 depends_on_landed 024, whose US4 last changed protocol.rs (f0ea2d54)"
      - path: crates/scope-node/src/mount/skywatcher/serial.rs
        other_spec: 014-goto
        other_story: US3
        disposition: merge_order
        reason: "039 depends_on_landed 014, whose US3 wrote serial.rs (726c1fdd)"
      - path: crates/scope-node/src/mount/skywatcher/transport.rs
        other_spec: 014-goto
        other_story: US3
        disposition: merge_order
        reason: "039 depends_on_landed 014, whose US3 wrote transport.rs (726c1fdd)"
    open_questions: []
  US2:
    tests:
      - path: crates/scope-node/tests/mount_start.rs
        status: new
        disposition: add
        assertion: "on a tracking axis a start acknowledged with =0, with bytes that cannot be framed, or not at all answers OK (after 0 ms, 0 ms and 300 ms) over f1 I1 J1 f1, with both axes moving for 3 s, no K or L, unacknowledged_starts 1, halted_moves 0 and invalid_replies 0"
        decision: "spec.md:US2-S1"
      - path: crates/scope-node/tests/mount_start.rs
        status: new
        disposition: add
        assertion: "a start lost on its way to an axis at rest answers OK over f1 G1 I1 f1 K1 K2, the board never sees J1, both axes rest, unacknowledged_starts 1 and halted_moves 1"
        decision: "spec.md:US2-S4"
      - path: crates/scope-node/tests/mount_start.rs
        status: new
        disposition: add
        assertion: "a GoTo of both axes with azimuth's start acknowledged =0 runs f1 j1 G1 S1 J1 f1 f2 j2 G2 S2 J2 and ends at 90,000 and 45,000; with altitude's acknowledgement lost it answers the second call 300 ms after the first, runs f1 j1 G1 S1 J1 f2 j2 G2 S2 J2 f2 and ends at 90,000 and 45,000; each with no K or L, unacknowledged_starts 1 and halted_moves 0"
        decision: "spec.md:US2-S5"
      - path: crates/scope-node/tests/mount_start.rs
        status: new
        disposition: add
        assertion: "a 100-count GoTo whose acknowledgement is lost answers OK after 300 ms over f1 j1 G1 S1 J1 f1 K1 K2, ends at 100 counts while the other axis's slew stops under 45,000, with unacknowledged_starts 1 and halted_moves 1"
        decision: "spec.md:US2-S7"
      - path: crates/scope-node/tests/mount_garble.rs
        status: existing
        disposition: preserve
        assertion: "a start whose acknowledgement is lost or refused is sent once, and the wrong-length counts around a GoTo whose start is unacknowledged are 2, 3 and 4 as before"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/scope-node/tests/skywatcher.rs
        status: existing
        disposition: preserve
        assertion: "a rate that is not finite and a twice-refused mode still stop the axes"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "a motion start is never sent twice (024 FR-016)"
      - "every failed rate or GoTo command sends the stop to both axes, and RateAxis and GotoAxis answer OK (spec.md FR-011, FR-013)"
      - "US1's mount_step.rs passes unchanged: its scenarios spoil no start"
    checks:
      development:
        - "cargo nextest run -p scope-node --locked --test mount_start"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs, a node rebuilt on the Pi and the field check of plan.md § Prerequisites"
    writes:
      - crates/scope-node/README.md
      - crates/scope-node/src/mount/skywatcher/mount_link.rs
      - crates/scope-node/src/mount/skywatcher/protocol.rs
      - crates/scope-node/tests/mount_start.rs
    no_write_reason: ""
    shared_writes: []
    open_questions: []
  US3:
    tests:
      - path: crates/finder/tests/motion/tracking/mount_line.rs
        status: new
        disposition: add
        assertion: "under Track on M42, a start on azimuth or on altitude acknowledged =0 or not at all leaves Track tracking with no ended reason, one spoiled J on the wire, no K or L in 4.5 s, all four counts within 1 of a control run's at 4.5 s and 14.5 s, unacknowledged_starts 1, halted_moves 0 and the console's link connected"
        decision: "spec.md:US3-S1"
      - path: crates/finder/tests/motion/tracking/mount_line.rs
        status: new
        disposition: add
        assertion: "under Track on M42, after one status reply 400 ms late, 30 s hold 60 J, 60 I and no K or L, azimuth's count rises by at least 700 and altitude's by at least 100, the board says both axes move, Track is tracking, invalid_replies is 0, stale_replies 1, resets 0 and both links connected"
        decision: "spec.md:US3-S3"
      - path: crates/finder/tests/motion/tracking/mount_line.rs
        status: new
        disposition: add
        assertion: "the console's own client giving up on axis_state after 200 ms leaves the next read of a slewing axis moving true, as the board said, with the node's stale_replies 1"
        decision: "spec.md:US3-S4"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no test file at the top level of crates/finder/tests, the same helper files under support/, and only mod lines in a folder's mod.rs"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "no file under crates/finder/src or crates/scope-node/src is written"
      - "every existing module of finder's motion binary passes unchanged"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test motion tracking::mount_line::"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - crates/finder/tests/motion/tracking/mod.rs
      - crates/finder/tests/motion/tracking/mount_line.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/tests/motion/tracking/mod.rs
        other_spec: 038-track-follows-a-push
        other_story: US2
        disposition: merge_order
        reason: "039 depends_on_landed 038, whose US2 last changed the module list (5b9f8b57)"
    open_questions: []
```
