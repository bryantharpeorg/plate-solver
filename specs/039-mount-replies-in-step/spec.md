---
state: draft
depends_on_landed: ["014-goto", "015-tracking", "017-console-stop-confirm", "024-mount-link-recovery", "027-finder-tests-by-system", "030-imaging-camera-link", "038-track-follows-a-push"]
# Drafted 2026-11-19 against `main` at 6e506ec6 (6e506ec6864fcaa5d5474a203357481dedc5ae81),
# at the owner's ask the same day ("Yes, draft 039"), from two probes of that day: a
# reading of the scope node's Sky-Watcher adapter and a run of that reading on the
# emulator, with three trial fixes. Every `path:line` here was read at 6e506ec6.
# It is a draft: nothing here is settled or approved. The owner decisions are open, no
# story has been probe-built from this text, and the operator asks the owner before any
# flip. What rests on a run and what on reading is said where it is used (Clarifications;
# plan.md § What was run and what was read).
---

# Feature Specification: The Mount's Replies Stay in Step

**Feature Branch**: `039-mount-replies-in-step`

**Created**: 2026-11-19

**Status**: Draft

**Input**: Two faults in the scope node's Sky-Watcher mount adapter, both confirmed on
the emulator on 2026-11-19. First, when the acknowledgement of a motion start (`:J`) is
garbled or lost, the node stops both axes and still answers the console OK, and counts
nothing. Second, one reply that arrives late, or whose reader has gone away, leaves the
serial line one reply out of step for good: every later answer is the reply to the
command before it, every rate and GoTo command then fails inside the node and stops both
axes while the console is told OK, and Track reads `tracking` over a mount that stands
still. Make the node check an unacknowledged start before it calls it failed, keep the
line in step by taking a left-over reply off it before the next transmission, and count
and log both.

## Clarifications

### Session 2026-11-19 (the two probes, and a review of `main` at 6e506ec6)

- Q: What does this trio read besides itself? → A: Nothing about the feature. `spec.md`
  holds every requirement and scenario, and `plan.md` holds every design decision, the
  interfaces, the shapes that existing tests pin and the alternatives rejected. A story
  that would change a decision escalates rather than re-deciding it. The probes' notes
  and branch are not in the repository; what they showed is written here.
- Q: What happens today when a start's acknowledgement is spoiled? → A: The start is sent
  once and never again, by 024's FR-016. `Protocol::ack` sends `:J` with one transmission
  (`crates/scope-node/src/mount/skywatcher/protocol.rs:221`), and an answer that is
  missing, cannot be framed or carries data comes back as an error. `rate` and `goto`
  end with that start (`crates/scope-node/src/mount/skywatcher/protocol.rs:312`,
  `crates/scope-node/src/mount/skywatcher/protocol.rs:360`), so the error reaches
  `log_motion`, which logs `mount command failed: <error>` and sends the stop to both
  axes (`crates/scope-node/src/mount/skywatcher/protocol.rs:481`). `Mount::rate` and
  `Mount::goto` return nothing (`crates/scope-node/src/mount/mod.rs:84`), so the node's
  `RateAxis` and `GotoAxis` answer OK (`crates/scope-node/src/service.rs:583`). The probe
  ran it: with both axes tracking and the board itself having taken the start, the node
  answered OK, sent `:K1` and `:K2`, and both axes stood still until another command
  came. Under the console's tracker, Track stayed `tracking` and the axis rested until
  the next one-second refresh; a GoTo's axis stopped short of where it was sent.
- Q: What puts the line out of step? → A: `Transport::exchange` writes one frame and
  reads one line (`crates/scope-node/src/mount/skywatcher/transport.rs:55`). When its
  future is dropped between the two, or its 300 ms run out
  (`crates/scope-node/src/mount/skywatcher/transport.rs:45`), the reply stays in the
  serial input queue. Nothing empties that queue but a baud change
  (`crates/scope-node/src/mount/skywatcher/serial.rs:41`), so the next exchange reads the
  earlier reply, and so does every exchange after it. Three things cut an exchange
  short: its own 300 ms, when the reply is only late; the console giving up on a read,
  which its mount monitor does after 200 ms (`crates/finder/src/live.rs:330`), because
  the node's handler runs the read inside its own future
  (`crates/scope-node/src/service.rs:544`); and Stop, which makes the queue drop the
  move it is carrying out (`crates/scope-node/src/mount/queue.rs:153`). The second and
  third need no fault on the line, only a reply that arrives after its reader has gone.
- Q: What did the probe see once the line was out of step? → A: Each of these, the same
  on three runs in paused time:
  1. A slewing axis reported at rest: its status read was answered by the other axis's
     left-over reply.
  2. An axis at rest reported moving, then three failed reads in a row with the board
     answering every one, the log line `mount link lost: 3 failed reads in a row`, a
     reconnect and `mount_resets` 1, which the console takes for a power cycle (024's
     FR-014 and FR-015).
  3. Stop's `:K1` answered by a status reply, so the node sent `:L1`, an emergency stop
     nobody needed.
  4. Every later rate command failing in the node: in 5 s of the tracker's refreshes,
     ten `RateAxis` calls all answered OK while 0 `:J` and 20 `:K` reached the board. With
     Track on, Track still read `tracking` 32.5 s later with both counts unchanged.
  5. A GoTo of both axes answered OK twice while no `:S` and no `:J` reached the board and
     neither axis moved.
- Q: What is the fix for the start? → A: When a start's acknowledgement is missing, cannot
  be framed or carries data, the node reads that axis's status once, through the read
  024 already retries. An axis that reads running took the start: the command succeeded
  and nothing is stopped. Otherwise the command fails as today. `:J` is still sent once.
- Q: What is the fix for the line? → A: Before every transmission the transport does two
  things. It reads out the reply of an exchange that was begun and not finished, waiting
  for it no longer than what is left of that exchange's own 300 ms. Then it discards
  whatever input has already arrived and was not read. Neither is enough alone: the
  discard cannot see a reply still on its way, and the read-out does not wait for a
  reply whose 300 ms have already run out.
- Q: What does the fix cost? → A: One wait, paid by whatever is sent next after an
  exchange was cut short, and only until that exchange's reply arrives (30 ms in
  US1-S5, where the reply takes 50 ms). When that reply is lost, the wait lasts until
  300 ms after the abandoned frame was sent. The probe measured Stop's `:K1` leaving 280 ms
  after Stop was asked, where today it leaves at once (owner decision 1).
- Q: Is the console told? → A: No. No RPC changes shape or answer. Both events are
  counted in the node's own `MountLink::line_health()` and logged with the command and
  the axis (owner decision 2).
- Q: Does this change 024's FR-016? → A: No. It reads, in full: "A status or position
  read whose framed reply has the wrong length SHALL be sent again, its tries sharing
  five transmissions in all; a motion start (`:J`) SHALL never be sent again"
  (`specs/024-mount-link-recovery/spec.md:700`). The start is still sent once. What is
  added after it is a status read, which that requirement already lets the node repeat,
  and 024's own test of the rule
  (`crates/scope-node/tests/mount_garble.rs:121`) passes unchanged.
- Q: Does anything here know about alt-az mounts? → A: No. The mount port
  (`crates/scope-node/src/mount/mod.rs:66`) is not changed. Everything is inside the
  Sky-Watcher mount adapter, between its protocol and its serial stream, and would
  serve the same board on an equatorial mount unchanged.
- Q: What rests on a run, and what on reading? → A: The behaviours of both faults on
  `main`, and of the three trial fixes, were run on the emulator, three times each with
  identical output; so were a GoTo under each fault, with and without the fixes. The
  stories' own form was not run: the counters of `line_health()`, the log lines, the
  discard that returns its bytes, the real device's discard by reading, and the test
  line's place in the library are designed here from that code. plan.md § What was run
  and what was read lists each. Nothing was run on a real serial device or the real
  board.
- Q: What is left for later? → A: The items under "Out of scope" in § Assumptions.

## Owner decisions

Each is a design or safety call, with a recommended answer. None is settled.

1. **What Stop pays for a line in step** (FR-001). Recommended: accept the bounded wait.
   Whatever is sent after an exchange was cut short first reads that exchange's reply off
   the line, for no longer than what is left of its 300 ms. Stop pays it when it cancels a
   move between that move's transmission and its reply, which is the case that puts the
   line out of step today. The wait ends when the reply arrives; when the reply is lost
   it ends 300 ms after the abandoned frame was
   sent (280 ms after Stop was asked, in the probe's case). The console's first Stop
   attempt of 200 ms then runs out and its second, 250 ms after the press, is answered
   (`crates/finder/src/link.rs:384`). Instead: (b) Stop sends `:K` at once and may read
   the left-over as its answer, which the probe ran: the node then takes `:K1` as
   unacknowledged and sends `:L1`, an emergency stop with no deceleration, and the mount
   moves again afterwards; or (c) Stop sends at once and reads one more line when the
   first is not an empty acknowledgement, which was not tried.
2. **The node's answer when it had to stop a command** (FR-013). Recommended: leave it
   OK, count the event as `halted_moves` and log it with the command and the axis. An
   error answer changes what Track does: the tracker ends as `link_lost` on any failed
   `RateAxis` (`crates/finder/src/tracking.rs:951`), so one failed command would end
   Track where today the next refresh, one second later, sends it again. Instead: (b)
   answer an error status, and change the tracker and GoTo to read it, a spec of its
   own; or (c) add a field to the replies, which changes the RPC's shape.
3. **A start found not running is not sent again** (FR-011). Recommended: keep 024's
   FR-016 whole. Track's next refresh sends the start again within a second, and a GoTo
   spends one of its five corrections. Instead: send `:J` once more when the status read
   finds the axis at rest, on the rate path and the absolute `:S` path, where a second
   start does no harm. That changes FR-016 and was not tried.
4. **One axis's failed command still stops both axes** (FR-011). Recommended: unchanged.
   A command that failed on the node means the node does not know what the mount is
   doing, and 017's stop reaches both axes. Instead: stop only the axis whose command
   failed, which changes 014's and 017's behaviour and was not tried.
5. **The real device discards by reading, not by flushing** (FR-005). Recommended: on
   the serial device, read until the device has nothing more, at most 256 bytes a call,
   so that the bytes are counted, logged and kept, and so that the same code runs in the
   `test` gate over a socket pair. Instead: `tcflush` of the input queue, one call the
   probe compiled and never ran, which cannot say what it threw away, so on the real
   mount the count would stay 0 and only the field check could show a left-over reply.

## Owner approvals

Flipping this spec to `ready` would approve the answer to every owner decision above, and
the following. Each is named in the FR that makes it. No existing test, golden or CI
file changes but for the one line of approval 5.

1. **Stop, like every command, may wait for an abandoned reply**, at most until 300 ms
   after that reply's frame was sent (FR-001; owner decision 1).
2. **`BaudStream` gains a required method**, `discard_input`. The trait is public, and
   its three implementations are in this crate (FR-005).
3. **The node's log line `mount command failed: <error>` is replaced** by a line that
   names the command and the axis, and three lines are added (FR-004, FR-012, FR-013).
4. **Test tooling in the library**: `Line` and `LineHandle` beside the emulator, and
   `Emulator::discard_input` (FR-006).
5. **US3's test module is declared in an existing test file.** US3 adds one line,
   `mod mount_line;`, in its alphabetical place in
   `crates/finder/tests/motion/tracking/mod.rs`, as 027's layout asks of every new
   finder test. No other line of that file and no other existing test changes.
6. **A test function is added to the `tests` module of `serial.rs`**, beside the one
   there, which is not edited (US1-S8).

## User Scenarios & Testing *(mandatory)*

Every Rust scenario shares these definitions.

- **The gate command.** Every scenario runs in
  `cargo nextest run --workspace --all-targets --all-features --locked`, the `test`
  gate's first command.
- **The node rig** (US1, US2) is built under `#[tokio::test(start_paused = true)]`:
  - a node clock that is a copy of the `Clock` in
    `crates/scope-node/tests/mount_reset.rs:23`;
  - one `Emulator` built with `EmulatorConfig::default()`: `Emulator::new` returns a
    stream, which is not used, and the `EmulatorHandle` called `handle`; and a
    `LineHandle::default()` called `line`;
  - a `MountLink` whose opener returns
    `Ok(Line::new(handle.reopen(clock.clone()), line.clone()))`, so it is a
    `MountLink<Line>` and every reconnect reaches the same board through the same line;
  - 1 s passes, and `link.connected()` is true, `link.resets()` 0 and
    `link.line_health()` equal to `LineHealth::default()`;
  - a `NodeService` built with `NodeService::with_controls_and_mount` from
    `SyntheticCamera::new(clock.clone())`, the clock, `"test-token"`,
    `Settings::default()`, a `NodeControls::default()` on which `stop_frames()` was
    called (so the capture loop's own position reads stay off the wire), and
    `Some(link.clone())`;
  - a spawned task that calls the service's `heartbeat` every 200 ms until the test
    ends, so the node's watchdog never stops the mount;
  - then 10 ms pass.
- **A call** is the service's own handler called directly, as
  `crates/scope-node/tests/mount_reset.rs:168` calls `axis_state`: `axis_state`,
  `rate_axis`, `goto_axis` or `stop_all`. **Reading an axis** is `axis_state` for it,
  which must answer.
- **Tracking** means `rate_axis(azimuth, 0.004)` and `rate_axis(altitude, -0.003)` each
  answered OK, then 2 s, then both axes read `moving`.
- **The exchanges** of a stretch are the emulator's `handle.exchanges()` recorded in it,
  in order. A list such as `:f1`, `:I1`, `:J1` names each by the first three characters
  of its command, and is exact: nothing else was recorded in the stretch.
- **What the board itself last said of an axis** is the second digit of the last
  three-digit reply the emulator recorded for that axis's `:f`: `1` is moving.
- **Until both are at rest** means reading both axes every 100 ms until neither is
  `moving`, for at most 60 s, and failing otherwise.
- **The tracker rig** (US3) is built under `#[tokio::test(start_paused = true)]`:
  - a `VirtualClock::new(START)` with `START` from
    `crates/finder/tests/support/live.rs:34`, a node clock on its `node_now()`, and the
    `ConsoleClock` of `crates/finder/tests/support/live.rs:35`;
  - an `Emulator`, a `LineHandle` and a `MountLink<Line>` as in the node rig, on the node
    clock, connected after 1 s;
  - a `Model::default()` given `load_known_terms(1.0, 130.0, zeros)`, where `zeros` is
    what `model.inverse(az, h, [360; 2])` answers for M42's apparent place at the
    clock's time, from `crate::support::tracker::apparent`
    (`crates/finder/tests/support/tracker.rs:76`), after a first
    `load_known_terms(1.0, 130.0, [0.0; 2])`: the emulator's zero counts are then M42's
    place;
  - a `NodeService` built as in the node rig, with frames stopped;
  - a `LinkManager` over a `Transport::connector` that serves `node.routes()` with
    `memory::serve_routes_on` across `tokio::io::duplex(64 * 1024)`, as
    `crates/finder/tests/support/tracker.rs:148` builds its transport, on a
    `Session::default()` whose node address and token are set as
    `crates/finder/tests/support/tracker.rs:171` sets them, waited on until
    `link.status().borrow().connected` is true;
  - a `Tracker::new` over the console clock, that link, the model, the site at latitude
    40° and longitude −75°, and a 15° minimum altitude.
  No heartbeat task is needed: the console's link sends its own. No pointing is fed
  to the tracker, so only its feed-forward rates reach the mount.
- **Track on M42** means `tracker.start` on `targets::find("M42")` answered `Ok`, then
  the test run on to 5,500 ms after that call, where `tracker.state().state` is
  `tracking`.
- **Oracles.** The emulator's own record of what the board was sent and what it
  answered, the board's own counts read back through the node, and the fixed numbers in
  the scenarios. Times are on paused Tokio time and exact to the millisecond. A count or
  a word has no tolerance; every other tolerance is stated where the number is.

### User Story 1 - A reply left on the line never answers the next command (Priority: P1)

As the observer with Track on, I press Stop, or the page gives up on a slow read, and
afterwards the mount still does what it is told and the node still reads each axis as it
is.

**Why this priority**: After one late or abandoned reply the node cannot move the mount
until it is restarted, and tells the console nothing. The second story's status read can
be trusted only on a line in step.

**Independent Test**: `crates/scope-node/tests/mount_step.rs`, a new file, and one test
in the `tests` module of `crates/scope-node/src/mount/skywatcher/serial.rs`. During
development, `cargo nextest run -p scope-node --locked --test mount_step` and
`cargo nextest run -p scope-node --locked --lib serial::`.

**Acceptance Scenarios**:

1. **Given** the node rig, tracking, with
   `line.delay_next(":f1", Duration::from_millis(400))`, **When** a committed test reads
   azimuth once, then reads both axes ten times 300 ms apart, then calls
   `rate_axis(azimuth, 0.0041)`, then makes five rounds 1 s apart of
   `rate_axis(azimuth, 0.0041)` and `rate_axis(altitude, -0.003)`, reading both axes
   100 ms after each round, **Then** it asserts that:
   - the first read answers 400 ms after it was asked, with `moving` true, and its
     exchanges are `:f1`, `:f1`, `:j1`;
   - after it `link.health().invalid_replies` is 0, and `link.line_health()` has
     `stale_replies` 1 and `last_stale` `Some("=111\\r")`, the six characters `=`, `1`,
     `1`, `1`, a backslash and `r`;
   - each of the ten later rounds of reads recorded exactly 4 exchanges, read both axes
     `moving`, and left `invalid_replies` 0 and `stale_replies` 1;
   - the single `rate_axis` answers OK with the exchanges `:f1`, `:I1`, `:J1`;
   - all ten calls of the five rounds answer OK, both axes read `moving` after each
     round, and the rounds' exchanges hold 10 commands that start with `:J` and none
     that starts with `:K` or `:L`.
2. **Given** the node rig with `rate_axis(azimuth, 0.004)` answered OK and, 2 s later,
   azimuth read `moving` and altitude not, with
   `line.delay_next(":f1", Duration::from_millis(1600))`, **When** a committed test
   reads azimuth, waits 200 ms, reads altitude and then
   azimuth, waits 5 s and reads azimuth again, **Then** it asserts that:
   - the first read answers 1,500 ms after it was asked, with `moving` true: all five
     tries ran out, and a lost reading never looks like rest;
   - the altitude read has `moving` false, `rate_deg_per_s` 0.0 and `position` 0;
   - both later azimuth reads have `moving` true, and every reply's `mount_resets` is 0;
   - `link.connected()` is true after every read and `link.resets()` is 0 at the end;
   - `invalid_replies` is 0, `stale_replies` is 1, and `last_stale` is the 25 bytes
     `=111\r` five times over, escaped.
3. **Given** the node rig with azimuth at rest, `goto_axis(altitude, 90000)` answered OK
   and, 2 s later, altitude read `moving`, with
   `line.delay_next(":f1", Duration::from_millis(250))`, **When** a committed test reads
   azimuth inside
   `tokio::time::timeout(Duration::from_millis(200), ...)`, waits 100 ms and reads
   altitude twice, **Then** it asserts that the timeout elapsed, that both altitude
   reads have `moving` true, that the board itself last said altitude was moving, and
   that `invalid_replies` is 0, `stale_replies` 1 and `last_stale` `Some("=001\\r")`.
4. **Given** the setup of scenario 3, **When** a committed test reads azimuth inside the
   same 200 ms timeout and at once reads altitude, then reads both axes five times
   300 ms apart, **Then** it asserts that the timeout elapsed, that the altitude read
   answers 50 ms after it was asked with `moving` true (the left-over reply arrived
   then and was read out first), that each of the five later rounds reads azimuth not
   `moving` and altitude `moving`, and that `invalid_replies` is 0 and `stale_replies`
   1.
5. **Given** the node rig, tracking, with
   `line.delay_next(":f1", Duration::from_millis(50))`, **When** a committed test spawns
   `rate_axis(azimuth, 0.0041)`, 20 ms later calls `stop_all`,
   500 ms after its answer reads both axes, and then makes five rounds 1 s apart of
   `rate_axis(azimuth, 0.004)` and `rate_axis(altitude, -0.003)`, reading both axes
   100 ms after each round, **Then** it asserts that:
   - the spawned call fails with `ABORTED` and `mount command discarded by Stop`;
   - `stop_all` answers OK 30 ms after it was asked;
   - the exchanges from the moment Stop was asked to its answer are `:K1`, `:K2`, each
     answered `=\r`: no `:L`;
   - both axes read not `moving` 500 ms after the Stop;
   - all ten calls of the five rounds answer OK, both axes read `moving` after each
     round, and the rounds' exchanges hold 10 commands that start with `:J` and none
     that starts with `:K` or `:L`;
   - at the end `invalid_replies` is 0, `stale_replies` 1 and `last_stale`
     `Some("=111\\r")`.
6. **Given** the node rig, tracking, with `handle.drop_replies(b'f', 1, 1)`, **When** a
   committed test spawns `rate_axis(azimuth, 0.0041)`, 20 ms later calls `stop_all`,
   and 500 ms after its answer reads both axes, **Then** it asserts that `stop_all`
   answers OK 280 ms after it was asked, which is 300 ms after the abandoned `:f1` was
   sent; that the exchanges from the moment Stop was asked to its answer are `:K1`,
   `:K2`, each answered `=\r`; that both axes then read not `moving`; and that
   `stale_replies` is 0: the reply that was waited for never came.
7. **Given** the node rig with both axes at rest at position 0 and
   `line.delay_next(":f1", Duration::from_millis(250))`, **When** a committed test reads
   azimuth inside `tokio::time::timeout(Duration::from_millis(200), ...)`, at once calls
   `goto_axis(azimuth, 90000)` and `goto_axis(altitude, 45000)`, and then reads until
   both are at rest, **Then** it asserts that:
   - the timeout elapsed and both GoTo calls answer OK, the second 50 ms after the first
     was asked;
   - the exchanges from the abandoned read to the second answer are `:f1`, `:f1`, `:j1`,
     `:G1`, `:S1`, `:J1`, `:f2`, `:j2`, `:G2`, `:S2`, `:J2`;
   - at rest, azimuth's `position` is 90,000 and altitude's 45,000;
   - no exchange after the abandoned read starts with `:K` or `:L`;
   - `invalid_replies` is 0 and `stale_replies` 1.
8. **Given** a `Serial` over one end of a non-blocking `UnixStream` pair, made as the
   test at `crates/scope-node/src/mount/skywatcher/serial.rs:81` makes one, whose peer
   has written the seven bytes `=111\r=\r`, **When** a committed test in that file's
   `tests` module calls `discard_input()` twice, then has the peer write 300 bytes of
   `=` and calls it twice more, then has the peer write `=\r` and calls `read_byte()`
   twice, **Then** it asserts that the first call returns exactly those seven bytes,
   the second an empty vector, the third 256 bytes and the fourth 44, and that the two
   bytes read are `=` and `\r`: the real device's discard takes what has arrived, never
   waits, and stops at 256 bytes a call.

---

### User Story 2 - A start whose acknowledgement is spoiled is checked, not assumed failed (Priority: P1)

As the observer with Track on or a GoTo under way, I lose nothing when one
acknowledgement is garbled: the axis that started keeps running and the other is left
alone.

**Why this priority**: The start goes out twice a second under Track, and today one
spoiled acknowledgement stops both axes without a word to the console.

**Independent Test**: `crates/scope-node/tests/mount_start.rs`, a new file. During
development, `cargo nextest run -p scope-node --locked --test mount_start`.

**Acceptance Scenarios**:

1. **Given** the node rig, tracking, with `handle.inject_reply(b'J', 1, "=0\r", 1)` (an
   acknowledgement that carries a character), **When** a committed test calls
   `rate_axis(azimuth, 0.0041)` and then reads both axes every 50 ms for 3 s, **Then**
   it asserts that:
   - the call answers OK, 0 ms after it was asked;
   - its exchanges are `:f1`, `:I1`, `:J1`, `:f1`: the start once, then one status read;
   - every read of both axes in the 3 s has `moving` true, and no exchange in them
     starts with `:K` or `:L`;
   - `link.line_health()` has `unacknowledged_starts` 1 and `halted_moves` 0, and
     `link.health().invalid_replies` is 0.
2. **Given** the node rig, tracking, with `handle.inject_reply(b'J', 1, "?\r", 1)` (bytes
   that cannot be framed), **When** a committed test does as in scenario 1, **Then** it
   asserts everything scenario 1 asserts.
3. **Given** the node rig, tracking, with `handle.drop_replies(b'J', 1, 1)` (no reply at
   all), **When** a committed test does as in scenario 1, **Then** it asserts everything
   scenario 1 asserts, but that the call answers OK 300 ms after it was asked.
4. **Given** the node rig with `rate_axis(altitude, -0.003)` answered OK and, 1 s later,
   azimuth at rest, with `line.lose_next(":J1")` (the start never reaches the board),
   **When** a committed test calls `rate_axis(azimuth, 0.004)` and 100 ms after its
   answer reads both axes, **Then** it asserts that:
   - the call answers OK;
   - its exchanges are `:f1`, `:G1`, `:I1`, `:f1`, `:K1`, `:K2`: the board never saw
     `:J1`, the status read found the axis at rest, and both axes were sent the stop,
     as today;
   - both axes read not `moving`;
   - `unacknowledged_starts` is 1 and `halted_moves` 1.
5. **Given** the node rig with both axes at rest at position 0 and
   `handle.inject_reply(b'J', 1, "=0\r", 1)`, **When** a committed test calls
   `goto_axis(azimuth, 90000)` and `goto_axis(altitude, 45000)` and then reads until
   both are at rest, **Then** it asserts that:
   - both calls answer OK;
   - their exchanges are `:f1`, `:j1`, `:G1`, `:S1`, `:J1`, `:f1`, `:f2`, `:j2`, `:G2`,
     `:S2`, `:J2`;
   - at rest, azimuth's `position` is 90,000 and altitude's 45,000;
   - no exchange starts with `:K` or `:L`, `unacknowledged_starts` is 1 and
     `halted_moves` 0.
6. **Given** the node rig with both axes at rest at position 0 and
   `handle.drop_replies(b'J', 2, 1)`, **When** a committed test does as in scenario 5,
   **Then** it asserts that:
   - both calls answer OK, the second 300 ms after the first was asked;
   - their exchanges are `:f1`, `:j1`, `:G1`, `:S1`, `:J1`, `:f2`, `:j2`, `:G2`, `:S2`,
     `:J2`, `:f2`;
   - at rest, azimuth's `position` is 90,000 and altitude's 45,000: the azimuth slew
     already under way was not stopped;
   - no exchange starts with `:K` or `:L`, `unacknowledged_starts` is 1 and
     `halted_moves` 0.
7. **Given** the node rig with `goto_axis(altitude, 90000)` answered OK and, 1 s later,
   altitude read `moving`, with `handle.drop_replies(b'J', 1, 1)`, **When** a committed
   test calls `goto_axis(azimuth, 100)`, a move the emulator finishes in under 0.2 s,
   and then reads until both are at rest, **Then** it asserts that:
   - the call answers OK 300 ms after it was asked;
   - its exchanges are `:f1`, `:j1`, `:G1`, `:S1`, `:J1`, `:f1`, `:K1`, `:K2`;
   - at rest, azimuth's `position` is 100 and altitude's is under 45,000;
   - `unacknowledged_starts` is 1 and `halted_moves` 1.
   This is the fix's limit, kept on purpose: a move that is over before the status
   read reads as at rest, and is failed as today (Edge Cases).

---

### User Story 3 - Track and the console's own calls over the real node (Priority: P2)

As the owner, I want both fixes proven where they matter: under the console's real
tracker and its real mount client, over the node's real service and adapter.

**Why this priority**: The node's scenarios call its handlers directly. Only the
console's own client shows that giving up on a call drops the node's read part-way, and
only the tracker shows what Track loses.

This story is built after User Story 2. It writes no file under `crates/scope-node/src`
or `crates/finder/src`: on User Story 1's and User Story 2's code its scenarios already
pass (tasks.md, T013).

**Independent Test**: `crates/finder/tests/motion/tracking/mount_line.rs`, the module
`tracking::mount_line` of finder's `motion` test binary, a new file. During development,
`cargo nextest run -p finder --locked --test motion tracking::mount_line::`.

**Acceptance Scenarios**:

1. **Given** the tracker rig with Track on M42, with
   `handle.inject_reply(b'J', 1, "=0\r", 1)` in one case and
   `handle.drop_replies(b'J', 1, 1)` in another, each on a rig of its own, and a third
   rig with no fault, the control, **When** a committed
   test on each rig records the exchanges of the next 4.5 s, reads both axes' counts
   through `MountLink::state`, waits 10 s and reads them again, **Then** it asserts for
   each faulted rig that:
   - `tracker.state().state` is `tracking` and `ended_reason` is none, before the fault
     is armed and after the 4.5 s;
   - exactly one recorded `:J1` has a reply other than `=\r`;
   - no recorded exchange starts with `:K` or `:L`;
   - each of the four counts differs from the control's by at most 1 count, the
     emulator's rounding of a reading;
   - `unacknowledged_starts` is 1 and `halted_moves` 0, and the console's link is
     connected.
2. **Given** the same three rigs with the fault on altitude's start instead,
   `handle.inject_reply(b'J', 2, "=0\r", 1)` and `handle.drop_replies(b'J', 2, 1)`,
   **When** a committed test does as in scenario 1, **Then** it asserts everything
   scenario 1 asserts, reading `:J2` for `:J1`.
3. **Given** the tracker rig with Track on M42 and
   `line.delay_next(":f1", Duration::from_millis(400))`,
   **When** a committed test waits 2.5 s, reads both axes' counts, records the exchanges
   of the next 30 s and reads the counts again, **Then** it asserts that:
   - `tracker.state().state` is `tracking` and `ended_reason` is none;
   - the 30 s of exchanges hold 60 commands that start with `:J`, 60 that start with
     `:I` and none that starts with `:K` or `:L`;
   - azimuth's count rose by at least 700 and altitude's by at least 100 between the two
     reads (756 and 130 in the probe);
   - the board itself last said both axes were moving;
   - the node's `invalid_replies` is 0 and its `stale_replies` 1, `link.resets()` is 0,
     and both the node's link and the console's are connected.
4. **Given** the tracker rig with no Track, its mount client taken with
   `link.mount()`, `goto_axis(Altitude, 90000)` answered `Ok` on it and, 2 s later,
   altitude read `moving`, with `line.delay_next(":f1", Duration::from_millis(250))`,
   **When** a committed test calls the client's `axis_state(Azimuth)` inside
   `tokio::time::timeout(Duration::from_millis(200), ...)`, waits 100 ms and calls
   `axis_state(Altitude)`, **Then** it asserts that the timeout elapsed, that the
   altitude reply has `moving` true, that the board itself last said altitude was
   moving, and that the node's `stale_replies` is 1: the console's giving up dropped the
   node's read part-way, and the reply it left was read out.

---

### Edge Cases

- **A reply more than 300 ms late, with the retry's own answer behind it.** The retry
  reads the late reply, which is at least an answer to the same command, and the retry's
  own answer is discarded before the next transmission if it has arrived by then
  (US1-S1). If it has not, because the next command follows at once on a real line, that
  command reads it. The line is then out of step only until the first transmission
  that follows a pause long enough for the left-over to arrive, where the discard takes
  it. Each such misread of a status or position read is counted by 024's
  `invalid_replies`, and the discard by `stale_replies`. This case was not run.
- **A left-over reply of the right length for the same command.** It is taken as the
  answer, as in US1-S1. For a read that is a reading up to 300 ms old.
- **A move that is over before the status read.** A GoTo of under about 300 counts on the
  emulator ends within the 300 ms a lost acknowledgement costs, so the status read finds
  the axis at rest, the command fails and both axes are stopped, as today (US2-S7,
  observed). A garbled acknowledgement that arrives costs no wait, so the read comes at
  once and finds the axis running.
- **A start lost on its way to the board on an axis already running the same way.** The
  rate path sends the new period with `:I` and then `:J` to an axis that is already
  tracking in that direction. If that `:J` never arrives, the status read finds the axis
  running and the command is accepted. Whether the real board applies `:I` without `:J`
  is not known; if it does not, the axis keeps its old rate until the next refresh, one
  second later. Read from the code, not run.
- **The status read after an unacknowledged start is itself garbled.** It is 024's
  retried read: up to five transmissions, each wrong-length reply counted and logged. If
  all five fail, the command fails with that read's error and both axes are stopped.
- **A refused start.** `!n` is an answer, not a missing acknowledgement: it fails as
  today, with no status read (`crates/scope-node/tests/mount_garble.rs:190` still sees
  one `:J1`).
- **The caller gives up while the node is reading out a left-over reply.** The exchange
  stays open with its original deadline, and the next transmission goes on waiting to
  that same moment.
- **Stop over a lost link.** `halt_once` sets the baud first
  (`crates/scope-node/src/mount/skywatcher/protocol.rs:469`), which forgets any open
  exchange and flushes the device, so 024's bound of four reply timeouts, 1.2 s, stands
  (`crates/scope-node/tests/mount_reset.rs:327`).
- **A connect attempt that gives way to Stop on the real device.** The attempt's own
  stream is closed with an exchange possibly open, and its reply may reach the kept
  stream after the flush, since descriptors of one serial device share its input queue.
  Stop then sends `:L` on that axis, which also stops it. The emulator gives each stream
  its own queue, so this is not tested and not changed here.
- **An echoing board.** The read-out skips an echo of the abandoned frame as the
  transport does for every reply, and then reads the reply.
- **A garbled reply of the right length.** The protocol has no checksum. Unchanged.
- **Two left-over replies at once.** The read-out takes one line and the discard takes
  the rest that has arrived (US1-S2 has five). More than one still on its way was not
  run.

## Requirements *(mandatory)*

### Functional Requirements

**The line** (US1):

- **FR-001**: Before every transmission, each try of a retried command included, the
  transport SHALL read one reply line off the stream when an exchange is open, and SHALL
  stop waiting for it no later than 300 ms after that exchange's frame was sent. An
  exchange whose 300 ms have already run out SHALL add no wait (owner decision 1).
- **FR-002**: An exchange SHALL be open from the moment its frame is written until its
  reply line has been read by that exchange or read out under FR-001. It SHALL stay open
  when its caller goes away and when its own 300 ms run out. `Transport::set_baud` SHALL
  close it.
- **FR-003**: After FR-001's read-out and before writing the frame, the transport SHALL
  discard every byte that has arrived and was not read, through
  `BaudStream::discard_input`, without waiting.
- **FR-004**: When FR-001 and FR-003 together took at least one byte off the line before
  a transmission, the transport SHALL count one stale reply, keep those bytes in order,
  escaped with `escape_ascii`, as the last stale input, and log
  `mount stale input before :<command><axis>: <bytes>`, naming the command about to be
  sent, as `mount stale input before :j1: =111\r`.
- **FR-005**: `BaudStream` SHALL gain `fn discard_input(&mut self) -> io::Result<Vec<u8>>`,
  which takes off the stream and returns the bytes that have arrived and were not read,
  and never waits. `Serial` SHALL implement it by reading until the device has no more,
  at most 256 bytes a call (owner decision 5). `Emulator` SHALL return its unread reply
  bytes and SHALL record no baud attempt for it.
- **FR-006**: The emulator module SHALL provide `Line` and `LineHandle` (plan.md
  § Interfaces): a `BaudStream` in front of an `Emulator` that can deliver the reply to
  one chosen transmission late and lose one chosen transmission, keeps bytes in order,
  keeps a reply nobody reads, and discards only bytes that have arrived.
- **FR-007**: `MountLink::line_health()` SHALL return a `LineHealth` with `stale_replies`
  and `last_stale`, summed over every board the link has had, the last bytes being the
  current board's or else the last retired one's, copied where `MountLink::health()` is
  copied. `MountHealth` SHALL NOT change.
- **FR-008**: `Transport::new` and `Transport::command` SHALL keep their signatures. When
  every reply is valid and in time, the exchanges, their order and their timing SHALL be
  what they are today, and a command's tries SHALL still take 300 ms each and five at
  most.
- **FR-009**: `crates/scope-node/README.md` SHALL describe, under `## The mount link`,
  the read-out and its bound, the discard, `line_health()`'s two fields and FR-004's
  line.

**The start** (US2):

- **FR-010**: When the reply to a motion start (`:J`) is missing after its 300 ms, cannot
  be framed, or is a framed reply that carries data, the adapter SHALL count one
  unacknowledged start and SHALL read that axis's status once, through 024's retried
  read. When the status says the axis is moving, the command SHALL succeed and no stop
  SHALL be sent.
- **FR-011**: When that status says the axis is not moving, the command SHALL fail with
  the acknowledgement's error; when the status cannot be read, with the read's error.
  Either way the failure SHALL be handled as every failed rate or GoTo command is today:
  the stop is sent to both axes (owner decision 4). The start SHALL have been sent once
  and SHALL NOT be sent again (024's FR-016; owner decision 3). A refusal or an I/O
  error SHALL fail as today, with no status read.
- **FR-012**: The adapter SHALL log an unacknowledged start as
  `mount start :J<axis> not acknowledged (<error>): the axis is running`, or the same
  with `the axis is not running`, or with `its status could not be read`.
- **FR-013**: Every rate or GoTo command that fails on the node SHALL be counted as one
  halted move and logged as
  `mount <rate|goto> on axis <axis> failed (<error>): stopping both axes`, in place of
  today's `mount command failed: <error>`. `RateAxis` and `GotoAxis` SHALL answer as
  they do today (owner decision 2).
- **FR-014**: `LineHealth` SHALL carry `unacknowledged_starts` and `halted_moves`,
  summed over every board the link has had. `crates/scope-node/README.md` SHALL describe
  the status read after an unacknowledged start, both counters and the lines of FR-012
  and FR-013.

**Unchanged** (every story; the Work Graph lists it under US2):

- **FR-015**: The mount port, every message of the node protocol, every RPC's answer, and
  every file under `crates/finder/src` SHALL stay as they are.

### Key Entities

- **Open exchange**: a frame that was sent and whose reply has not been read: when it
  was sent, and its bytes. At most one, held by the transport.
- **Stale input**: bytes taken off the line before a transmission, by the read-out or
  the discard.
- **Unacknowledged start**: a `:J` whose reply was missing, unframeable or carried data.
- **Line health**: the three counts and the last stale bytes, kept by the mount link
  for the node's process.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario has a committed test that asserts it, and the
  `lint` and `test` gates pass.
- **SC-002**: No existing test, golden, `scripts/ci/` file, `.github/` file,
  `CODEOWNERS` entry, `ergane.yaml` or `playwright.config.ts` is edited, but for the one
  `mod mount_line;` line US3 adds (owner approval 5). Every existing test passes
  unchanged (plan.md § Shapes that existing tests pin).
- **SC-003**: After one late or abandoned reply, the next command the console sends is
  carried out and the next read of each axis agrees with what the board itself said
  (US1-S1 to US1-S7), where today no start reaches the board again.
- **SC-004**: One spoiled start acknowledgement stops no axis that took its start (US2-S1
  to US2-S3, US2-S5, US2-S6), and under Track loses no more than 1 count (US3-S1,
  US3-S2), where today both axes are stopped and Track loses up to a second of motion.

## Assumptions

- **Earlier features are in place.** Every spec in `depends_on_landed` has landed.
  Every citation here was read at 6e506ec6.
- **The emulator stands for the board.** It accepts `:J` on a running axis and applies
  `:I` at once. Whether firmware 2.09 does was not checked; FR-010's status read does not
  depend on it.
- **How often a real line delivers a late reply is not known.** The lake night of 14
  November neither shows nor excludes it: Track never ran.
- **Out of scope:**
  - telling the console that a command was halted, or changing what Track and GoTo do
    then (owner decision 2);
  - sending a start a second time (owner decision 3);
  - stopping only the failing axis (owner decision 4);
  - a GoTo that ends before the status read (Edge Cases);
  - GoTo's start check acting on a single reading
    (`crates/finder/src/goto.rs:739`), and anything else in the console;
  - a connect attempt's left-over reply reaching the kept stream (Edge Cases);
  - finding the cause of garbled replies: this spec records what is left on the line.
- **Outside the factory:** rebuilding and installing the node on the Pi, and the field
  check in plan.md § Prerequisites, by the owner. The real device's `discard_input` runs
  in the gate only over a socket pair, never on a serial device.
- **The factory's gates and CI:** the gates are `lint` and `test`, copied in plan.md
  § Verification. CI's other jobs run only in the merge queue, so no scenario's Then
  clause rests on them.

## Work Graph

```yaml
US1:
  depends_on: []
  implements: [FR-001, FR-002, FR-003, FR-004, FR-005, FR-006, FR-007, FR-008, FR-009]
US2:
  depends_on: []
  depends_on_merged: [US1]
  implements: [FR-010, FR-011, FR-012, FR-013, FR-014, FR-015]
US3:
  depends_on: []
  depends_on_merged: [US2]
  implements: []
```

Every edge is a merge edge, and the stories run one after another.

- **US2** adds its two counters to US1's `LineHealth`, writes
  `crates/scope-node/src/mount/skywatcher/protocol.rs`,
  `crates/scope-node/src/mount/skywatcher/mount_link.rs` and the README after US1, and
  uses US1's `Line` to lose a start. Its status read is trusted because US1 keeps the
  line in step.
- **US3** tests both stories' code together and writes none of its own.
