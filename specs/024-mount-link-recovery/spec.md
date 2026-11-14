---
state: draft
depends_on_landed: ["013-live-finder", "014-goto", "015-tracking", "017-console-stop-confirm"]
# Drafted 2026-11-14 by the plate-solver operator against `main` at c1ed295, from the
# lake-night field findings of 13-14 November (gap 1: the mount link after a power cycle).
# It stays draft until the owner has reviewed it and given the approvals listed under
# "Owner approvals". Flipping it to `ready` is the act that dispatches it.
---

# Feature Specification: The Mount Link Survives a Power Cycle

**Feature Branch**: `024-mount-link-recovery`

**Created**: 2026-11-14

**Status**: Draft

**Input**: On the first dark-site night with the real Sky-Watcher board, the mount was
power-cycled at about 22:40 and the scope node never noticed. It had initialised the
board once, at 20:37, and never again: the board came back with its own encoders on,
hand pushes then moved the counts, and the pointing model stayed on one epoch through
an 80° push. Before that, with `[mount]` configured, the node would not start at all
until the mount answered (44 restarts in 20 minutes). Through the night the link also
saw garbled replies, which nothing logged. Make the node start without the mount and
connect when it answers, notice a power cycle and initialise the board again before any
move, report the board's identity and its resets to the console, and stop GoTo and
Track from using zero points the reset made stale.

## Clarifications

### Session 2026-11-14 (review of `main` at c1ed295)

- Q: What does this trio read besides itself? → A: Nothing about the feature. `spec.md`
  holds every requirement and scenario, and `plan.md` holds every design decision, the
  interfaces, the shapes that existing tests pin and the alternatives rejected. A story
  that would change a decision escalates rather than re-deciding it.
- Q: What was found? → A: Seven things, each checked against the code at c1ed295:
  1. The node connects to the board once, at start, and exits if it can't
     (`crates/scope-node/src/main.rs:82`, the `?` after `SkyWatcher::connect`). Under
     systemd that is a restart loop until the mount answers.
  2. The connect sequence finds the baud, sends `:F3` (initialisation done) and, on axes
     that report auxiliary encoders, `:W` with `050000` to turn them off
     (`crates/scope-node/src/mount/skywatcher/protocol.rs:321`,
     `crates/scope-node/src/mount/skywatcher/protocol.rs:324`,
     `crates/scope-node/src/mount/skywatcher/protocol.rs:335`). Nothing runs it again.
  3. The board's status reply has three hex digits, and the third one's lowest bit says
     whether initialisation was done. The node reads only the first two
     (`crates/scope-node/src/mount/skywatcher/protocol.rs:150`); the emulator already
     sends the third (`crates/scope-node/src/mount/skywatcher/emulator.rs:204`) and
     clears it only on a fresh board.
  4. The mount is fixed when the service is built
     (`crates/scope-node/src/service.rs:157`): a node started without one can never gain
     one.
  5. The transport retries timeouts and replies it can't frame
     (`crates/scope-node/src/mount/skywatcher/transport.rs:49`), but a framed reply of
     the wrong length fails above it, in the status and position reads
     (`crates/scope-node/src/mount/skywatcher/protocol.rs:152`,
     `crates/scope-node/src/mount/skywatcher/protocol.rs:195`), and is never retried or
     logged with its bytes.
  6. The console's session always reports the board and firmware as null
     (`crates/finder/src/session.rs:1001`): `DescribeMountReply` carries only the axes
     (`crates/scope-link/proto/scope_node/v1/scope_node.proto:126`).
  7. A GoTo already aborts when the node's watchdog stops the mount mid-move
     (`crates/finder/src/goto.rs:930`). Nothing does the same for a power cycle, which
     moves every count's meaning.
- Q: How is a power cycle recognised? → A: Three ways, any one enough: a status reply
  says initialisation was not done; the link was lost and comes back (the node can't
  tell an unplugged cable from a power cycle, so it takes the safe reading); or an
  axis's counts move further between two reads than the top rate allows.
- Q: What does a reset cost the observer? → A: One sync. The counts after a power cycle
  start from the board's power-on zero, so the night's zero points no longer apply. The
  console refuses GoTo and Track until a sync made after the reset, and the 014 model
  then opens a new epoch at that sync as it already does for a slip.

## Owner approvals

Flipping this spec to `ready` approves the following. Each is named in the FR that makes
it. No existing test, golden or CI file changes.

1. **The node starts without the mount** and tries again every 2 s (FR-001, FR-002).
   Until it connects, Stop answers "the mount did not confirm the stop", as for any
   unconfirmed stop.
2. **Every reconnect after a lost link counts as a reset**, and so does a jump in
   counts beyond 1.5 times the top rate's travel plus 1,000 counts (FR-006, FR-007).
   The first connect after start is not a reset.
3. **Two fields added to the node protocol**: `mount_resets` on `AxisState` and the
   board's `firmware` and `board` on `DescribeMountReply` (FR-009, FR-012). Both are
   additions; old consoles ignore them.
4. **GoTo and Track refuse to start after a reset until a new sync**, with the words
   "The mount was power-cycled: sync again first", and a running GoTo or Track stops
   when a reset is seen (FR-014, FR-015).
5. **Wrong-length replies to safe reads are retried**, up to the transport's five
   transmissions in all, and each one's bytes are kept for the log (FR-016, FR-017).

## User Scenarios & Testing *(mandatory)*

The Rust scenarios of US1, US2 and US4 share one setup unless a scenario says otherwise:
- A `MountLink` (plan.md § Interfaces) over the emulator: its opener returns
  `handle.reopen()` of one `Emulator` built with `EmulatorConfig::default()` (firmware
  3.40 on board 0x90, auxiliary encoders, 1,620,000 counts per revolution), so every
  reconnect reaches the same board.
- Tokio's clock is paused (`#[tokio::test(start_paused = true)]`) and the node clock is
  the test's `Clock` of `crates/scope-node/tests/skywatcher.rs`; "after N s" means
  `tokio::time::sleep` of N seconds.
- "Polled" means `state(Axis::Azimuth)` and `state(Axis::Altitude)` once each, as the
  capture loop does for every frame.
- "Moves on the wire" are the emulator's exchanges whose command starts with `:G`, `:S`,
  `:H`, `:M`, `:I` or `:J`. Commands are recorded with their carriage return, as
  `:F3\r`.

### User Story 1 - The node starts without the mount (Priority: P1)

As the observer setting up in the dark, I power the Pi before the mount. The node starts
at once and serves the camera; when I switch the mount on, it connects by itself.

**Why this priority**: The node restarted 44 times before the mount answered, and every
other story needs a link that can connect after start.

**Independent Test**: `cargo test -p scope-node --test mount_link --locked`.

**Acceptance Scenarios**:

1. **Given** the setup with the emulator powered off (`handle.power_off()`) before the
   link starts, **When** 10 s pass, **Then** a committed test asserts that
   `link.connected()` is false and that no exchange has a reply.
2. **Given** the same link, **When** the test calls `handle.power_on()` and 3 s pass,
   **Then** it asserts that `link.connected()` is true, that `link.axes()` reports
   1,620,000 counts per revolution on both axes, that the exchanges after power-on
   include `:F3\r`, `:W1050000\r` and `:W2050000\r` in that order, and that
   `handle.auxiliary_enabled(1)` and `handle.auxiliary_enabled(2)` are false.
3. **Given** a `NodeService` built with `with_mount` over a link whose emulator is
   powered off, **When** the test calls the service's `describe_mount`, `axis_state`,
   `goto_axis` and `stop_all` directly, **Then** it asserts that `describe_mount` returns
   no axes, that `axis_state` and `goto_axis` fail with `FAILED_PRECONDITION` and the
   message `mount not connected`, that `stop_all` fails with `UNAVAILABLE`, and that
   no move is on the wire.
4. **Given** the same service, **When** one frame is captured, **Then** the test asserts
   that the frame has neither `axis_start` nor `axis_end`.
5. **Given** the built `scope-node` binary with a config whose adapter is `simulated`
   and whose `[mount]` device is a path that does not exist, **When** a test starts it
   and waits 5 s, **Then** it asserts that the process is still running and that
   standard error contains `mount not connected`.

---

### User Story 2 - A power cycle initialises the board again (Priority: P1)

As the observer who switched the mount off and on, I find the node has initialised the
board again before anything moves, turned its encoders off again, and counted the reset.

**Why this priority**: An uninitialised board with its encoders on is how the night's
model went wrong without anyone seeing it.

**Independent Test**: `cargo test -p scope-node --test mount_reset --locked`.

**Acceptance Scenarios**:

1. **Given** the setup, connected, **When** the test calls `handle.power_cycle()` and the
   link is polled after 1 s, **Then** a committed test asserts that the exchanges after
   the cycle include `:F3\r`, `:W1050000\r` and `:W2050000\r`, that `link.resets()` is 1, that
   `handle.auxiliary_enabled(1)` is false, and that no move is on the wire after the
   cycle.
2. **Given** the setup, connected, **When** the test calls `handle.power_off()`, polls
   once a second for 5 s, then calls `handle.power_on()` and polls once a second for
   3 s, **Then** it asserts that `link.connected()` was false at the fifth poll, is true
   at the end, and that `link.resets()` is 1.
3. **Given** the setup, connected, with both axes at rest, **When** the test calls
   `handle.set_position(1, 500_000)` and polls after 1 s, **Then** it asserts that
   `link.resets()` is 1 and that the exchanges after the jump include `:F3\r`.
4. **Given** the setup, connected, **When** the test starts a GoTo on axis 1 to 16,200
   counts and polls every 100 ms until it is at rest, **Then** it asserts that
   `link.resets()` is 0: travel within the top rate is not a jump.
5. **Given** a `NodeService` over the link of US2-S1 after its cycle, **When** the test
   reads `axis_state` for both axes, **Then** it asserts that each reply's
   `mount_resets` is 1.
6. **Given** the link of US2-S2 while it is disconnected, **When** the test calls
   `link.goto(Axis::Azimuth, 1000)` and `link.rate(Axis::Altitude, 0.5)`, **Then** it
   asserts that no move is on the wire.

---

### User Story 3 - The console knows the board and its resets (Priority: P2)

As the observer at the page, I see the board's firmware and identity, and after a power
cycle the console tells me to sync before it will GoTo or Track.

**Why this priority**: The session showed null for the board all night, and a GoTo
after an unnoticed reset aims with zero points from before it.

**Independent Test**: `cargo test -p finder --test mount_identity --locked`.

**Acceptance Scenarios**:

1. **Given** a `NodeService` over a connected `MountLink` and the emulator's default
   board, **When** a test calls `describe_mount`, **Then** it asserts that the reply's
   `firmware` is `3.40` and its `board` is `0x90`; and given a node with no mount, that
   both are empty strings.
2. **Given** an in-process console built as `crates/finder/tests/mcp/support.rs:188`
   builds its `Rig`, but with a test `Mount` that wraps the `SimMount` and reports
   firmware `3.40`, board `0x90` and a reset count the test sets, **When** the test
   reads `GET /api/session`, **Then** it asserts that `mount.firmware` is `3.40`,
   `mount.board` is `0x90`, and that `mount` has no `resets` key while the count is 0.
3. **Given** that console after two syncs, **When** the test sets the reset count to 1
   and reads the session again, **Then** it asserts that `mount.resets` is 1, and that
   `POST /api/goto` to M42 answers `409` with `The mount was power-cycled: sync again
   first` and that the `SimMount` received no `GotoAxis`.
4. **Given** the console of US3-S3, **When** a sync completes after the reset and the
   test asks for the same GoTo, **Then** it asserts that the GoTo is accepted.
5. **Given** that console with a GoTo to M42 running, **When** the test sets the reset
   count to 1, **Then** it asserts that the GoTo ends `failed` with a reason containing
   `power-cycled`, and that a `StopAll` reached the node.
6. **Given** that console tracking after two syncs, **When** the test sets the reset
   count to 1, **Then** it asserts that tracking reads stopped within 2 s, and that
   `POST /api/track` answers `409` with `The mount was power-cycled: sync again first`.

---

### User Story 4 - Garbled replies are retried and kept (Priority: P3)

As the owner reading the node's log after a night, I can see the exact bytes of every
garbled reply, and a garbled read no longer fails a poll.

**Why this priority**: Garbled replies came every 6 to 8 minutes, and then every 1 to 2;
their cause is unknown until their bytes are seen.

**Independent Test**: `cargo test -p scope-node --test mount_garble --locked`.

**Acceptance Scenarios**:

1. **Given** the setup, connected, **When** the test calls
   `handle.inject_reply(b'f', 1, "=10\r", 1)` (a status two digits long) and polls,
   **Then** a committed test asserts that the azimuth state is not `moving`, that
   `link.health().invalid_replies` is 1, and that `link.health().last_invalid` is
   `Some("=10\\r")`.
2. **Given** the same, **When** the test injects that reply 5 times for `f` on axis 1
   and polls, **Then** it asserts that the azimuth state reads `moving: true` (a lost
   reading never looks like rest) and that `invalid_replies` is 5.
3. **Given** the same, **When** the test injects `"=12\r"` once for `j` on axis 2 (a
   position two digits long) and polls, **Then** it asserts that the altitude position
   equals `handle`'s position for axis 2 and that `invalid_replies` is 1.
4. **Given** an emulator with `refuse_absolute: true`, **When** the test starts a GoTo on
   axis 1 through the link, **Then** it asserts that `link.health().goto_path[0]` is
   `Some(GotoPath::Relative)`; and with the default board, `Some(GotoPath::Absolute)`.
5. **Given** the same, **When** the test injects a dropped reply for `J` on axis 1 once
   and starts a GoTo, **Then** it asserts that `:J1\r` was sent exactly once: a motion
   start is never retried.

---

### Edge Cases

- **The serial adapter itself is unplugged.** Opening the device fails; the link keeps
  trying every 2 s and logs the error once per change, not every try.
- **A brief power blip.** The board may answer the next read uninitialised without any
  timeout; the status bit catches it (US2-S1).
- **Firmware that never sets the bit.** A blip too short to lose the link and too
  early for any move goes unseen; any longer one loses the link, and every reconnect
  counts as a reset (owner approval 2). One extra sync is the cost.
- **The console restarts after a reset.** It learns the count at its first poll and
  can't tell it from an old one, so it does not refuse; the next sync's slip check
  (014) still opens a new epoch. The node's README says a console restarted after a
  power cycle needs a sync before GoTo.
- **A hand push with the board's encoders on.** It moves the counts faster than any
  motor could; the jump check counts it as a reset and initialises the board again,
  which turns the encoders off (US2-S3).
- **A move in flight when the mount loses power.** The motors stop with the power; the
  console's GoTo or Track stops when it sees the reset (US3-S5, US3-S6).
- **Stop while disconnected.** Answers "the mount did not confirm the stop" (US1-S3);
  the page already treats that as an unconfirmed stop (017).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: With `[mount]` configured, `scope-node` SHALL start and serve the camera
  whether or not the mount answers, and SHALL try to open the device and run the
  connect sequence every 2 s until it succeeds.
- **FR-002**: Until the link is connected, `DescribeMount` SHALL return no axes,
  `AxisState`, `GotoAxis` and `RateAxis` SHALL fail with `FAILED_PRECONDITION` and
  `mount not connected`, `StopAll` SHALL fail with `UNAVAILABLE`, frames SHALL carry no
  axis counts, and nothing SHALL be sent to the board but the connect sequence.
- **FR-003**: Each successful connect SHALL log one line naming the firmware, the baud,
  each axis's counts per revolution, timer, high-speed ratio and auxiliary-encoder
  support, and whether `:W` was sent, followed by the existing warnings.
- **FR-004**: `SkyWatcher::connect` and `SkyWatcher`'s `Mount` methods SHALL keep their
  signatures and behaviour; the link wraps them.
- **FR-005**: `crates/scope-node/README.md` SHALL describe starting without the mount,
  power cycles and the new log lines.
- **FR-006**: The link SHALL treat as a reset: a status reply whose third digit's lowest
  bit is 0; any reconnect after the link was lost; and an axis's counts moving between
  two reads by more than 1.5 × top rate × elapsed seconds × counts per degree + 1,000
  counts. The first connect after start SHALL NOT count.
- **FR-007**: On a reset the link SHALL run the connect sequence again before sending any
  other command, count the reset, and log it with its cause.
- **FR-008**: While connecting or initialising again, `goto` and `rate` SHALL send
  nothing to the board.
- **FR-009**: `AxisState` SHALL carry `uint64 mount_resets = 5`, the link's reset count.
- **FR-010**: The `Mount` trait SHALL gain `connected`, `resets` and `identity` methods
  with defaults (true, 0, none), so other adapters are unchanged.
- **FR-011**: The emulator SHALL support `reopen`, `power_off`, `power_on`,
  `power_cycle`, `set_position` and `inject_reply` on its handle, as plan.md
  § Interfaces gives them.
- **FR-012**: `DescribeMountReply` SHALL carry `string firmware = 2` and
  `string board = 3`, empty when unknown.
- **FR-013**: The console session's `mount.firmware` and `mount.board` SHALL come from
  `DescribeMountReply`, and `mount.resets` SHALL report the node's count, omitted
  while 0.
- **FR-014**: When the node's `mount_resets` is above its value at the model's latest
  sync, `POST /api/goto` and `POST /api/track` SHALL answer `409` with
  `The mount was power-cycled: sync again first`, and the MCP tools that start them
  SHALL refuse with the same words.
- **FR-015**: A running GoTo SHALL end `failed` with a reason containing `power-cycled`
  and send `StopAll`, and running tracking SHALL stop, when the node's `mount_resets`
  changes.
- **FR-016**: A safe read whose framed reply has the wrong length SHALL be sent again,
  up to five transmissions in all; a motion start (`:J`) SHALL never be sent again.
- **FR-017**: The link SHALL count invalid replies, keep the last one's bytes escaped,
  log each with its command, and record per axis whether GoTo used `:S` or the
  `:H`/`:M` fallback, logging the path the first time it is used on each axis.

### Key Entities

- **Mount link**: the node's supervisor around one Sky-Watcher board: connects, watches
  for resets, initialises again.
- **Reset**: a power cycle or anything that looks like one (FR-006).
- **Board identity**: firmware version and board code from `:e`.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario has a committed test that asserts it, and the
  `lint` and `test` gates pass.
- **SC-002**: No existing test, golden, `scripts/ci/` file, `.github/` file, `CODEOWNERS`
  entry, `ergane.yaml` or `playwright.config.ts` is edited.
- **SC-003**: A power cycle is recognised and the board initialised again within one
  poll of the board answering (US2-S1, US2-S2).

## Assumptions

- **Earlier features are in place.** 013, 014, 015 and 017 have landed. Every citation here
  was read at c1ed295.
- **Out of scope:**
  - dark-sky detection and exposure (spec 023);
  - saving live frames (spec 025);
  - finding the cause of the garbled replies: this spec only records them;
  - any change to the pointing model's epochs: the next sync after a reset opens one as
    a slip already does;
  - turning the board's encoders back on, or using them;
  - the page showing the reset count beyond what `GET /api/session` carries.
- **Outside the factory:** rebuilding and installing the node on the Pi, and the first
  real power cycle with the mount at rest, by the owner.
- **The factory's gates and CI:** the gates are `lint` and `test`, copied in plan.md
  § Verification. CI's other jobs run only in the merge queue, so no scenario's Then
  clause rests on them.

## Work Graph

```yaml
US1:
  depends_on: []
  implements: [FR-001, FR-002, FR-003, FR-004, FR-005, FR-010, FR-011]
US2:
  depends_on: []
  depends_on_merged: [US1]
  implements: [FR-006, FR-007, FR-008, FR-009]
US3:
  depends_on: []
  depends_on_merged: [US2]
  implements: [FR-012, FR-013, FR-014, FR-015]
US4:
  depends_on: []
  depends_on_merged: [US2]
  implements: [FR-016, FR-017]
```

Every edge is a merge edge. US2 builds on US1's `MountLink` and emulator handle; US3
reads US2's `mount_resets`; US4 changes the reads US2 watches. US3 touches the proto,
the node's service and `crates/finder`; US4 touches only the Sky-Watcher module and its
tests, so they may run beside each other once US2 has merged.
