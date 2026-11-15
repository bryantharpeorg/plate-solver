---
state: draft
depends_on_landed: ["013-live-finder", "014-goto", "015-tracking", "017-console-stop-confirm"]
# Drafted 2026-11-14 by the plate-solver operator against `main` at c1ed295, from the
# dark-site field findings of 13-14 November (gap 1: the mount link after a power cycle).
# Refined 2026-11-14 from four probe builds of its stories and two reviews (anchors, scenarios),
# then once more the same day from a completeness review of that refinement.
# The owner settled the owner decisions on 2026-11-14 by accepting every recommended
# answer.
# Refined 2026-11-14 (round 2) by probe builds of all four stories from the trio as it
# stood and two reviews; the scenarios this refinement added were run on the probe branches.
# On 2026-11-15 the owner answered the round-2 questions: owner decision 4 now sends Stop
# over the last stream even while the link is lost or initialising again, and owner
# approval 6 closes its first two gaps.
# Refined 2026-11-14 local time (round 3, after those answers) by probe builds of all four
# stories from the trio as it stood and two reviews (anchors, scenarios); the clauses this
# refinement added were run on copies of the round-3 probe branches. It waits for
# 027-finder-tests-by-system to land (plan.md § Prerequisites 6). Flipping it to `ready`
# gives the approvals listed under "Owner approvals" and dispatches it.
---

# Feature Specification: The Mount Link Survives a Power Cycle

**Feature Branch**: `024-mount-link-recovery`

**Created**: 2026-11-14

**Status**: Draft

**Input**: On the first dark-site night with the real Sky-Watcher board, the mount was
power-cycled at about 22:40 and the scope node never noticed. It had initialised the
board once, at 20:37, and never again: the board probably came back with its own
encoders on (inferred, not measured), hand pushes then moved the counts, and the
pointing model stayed on one epoch through an 80° push. Before that, with `[mount]`
configured, the node would not start at all until the mount answered (44 restarts in 20
minutes). Through the night the link also saw garbled replies, logged without their
bytes. Make the node start without the mount and connect when it answers, notice a
power cycle and initialise the board again before any move, report the board's identity
and its resets to the console, and stop GoTo and Track from using zero points the reset
made stale.

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
  says initialisation was not done on an axis where an earlier read since the connect
  said it was; the link was lost and comes back (the node can't tell an unplugged cable
  from a power cycle, so it takes the safe reading); or an axis's counts move further
  between two reads than the top rate allows.
- Q: What does a reset cost the observer? → A: One sync. The counts after a power cycle
  start from the board's power-on zero, so the night's zero points no longer apply. The
  console refuses GoTo, second sync and Track until it records a sync from a frame whose
  exposure started after it saw the reset, and the 014 model then opens a new epoch at
  that sync as it already does for a slip.

### Session 2026-11-14 (probe builds and review)

- Q: How long does the console's refusal last in practice? → A: Until the next solved
  frame at rest or tracking slowly. The live loop records a sync from every such frame
  (`crates/finder/src/live.rs:223`), and that sync is the one this spec asks for. A
  test can see the refusal only with the node's frames stopped, so US3-S3 and US3-S6
  stop them.
- Q: Can the link see a failed read? → A: Yes, through `SkyWatcher::initialised`, which
  reads none after a state read that failed. `Mount::state` itself still answers the
  last state with `moving: true` and logs the error
  (`crates/scope-node/src/mount/skywatcher/protocol.rs:395`), as its tests pin.
- Q: Does one power cycle that shows two causes count twice? → A: No. One read counts at
  most one reset, and a reset whose re-initialisation fails is counted once, when a
  later connect succeeds.

### Session 2026-11-14 (round 2: probe builds and review)

- Q: What if the caller gives up on the read that found a reset? → A: The
  re-initialisation goes on. The console bounds each mount poll at 200 ms
  (`crates/finder/src/live.rs:318`), less than a connect at 9600 baud may take, and a
  dropped `axis_state` handler drops its read. So the re-initialisation runs in its own
  task, which the read waits for (US2-S10), as the node's Stop already does
  (`crates/scope-node/src/service.rs:489`).

### Session 2026-11-15 (the owner's answers to round 2)

- Q: Does Stop reach a board the link has lost, or one it is initialising again? → A: Yes
  (owner decision 4, changed). Stop and the node's watchdog send 017's halt over the
  stream of the link's last connect, which the link keeps for them, and a connect under
  way gives the device up to them. Only before the first connect, or with the device
  gone, does Stop answer at once (US2-S11, US2-S12, US2-S13).
- Q: Which of owner approval 6's gaps stay open? → A: Only the third (owner approval 6).
  A GoTo reads the count after each correction's plan, before that correction's slew
  (US3-S7), and tracking's waits for rest in the keyhole read it (US3-S8). A console
  restarted after a reset still does not refuse; the README covers it.

### Session 2026-11-14 (round 3, local time: probe builds and review)

- Q: Does a Stop before the link's first connect make that connect give way? → A: No.
  With no stream opened yet there is nothing to stop over, so Stop answers at once
  without touching the device (US1-S3). The node's watchdog, which trips 1 s after the
  service starts when no heartbeat comes, and the console's repeated Stops therefore
  never delay the first connect.
- Q: Is any reset left that a running Track does not see? → A: Yes, one narrow window,
  recorded under owner approval 6 as gap (d) and open to the owner: tracking takes its
  baseline from its first tick's read, so a reset after the console's last mount poll and
  before that read becomes the baseline. The recommended default leaves it open.

## Owner decisions

Each is a design or safety call. *Settled 2026-11-14: the owner accepted the recommended
answer to all five; on 2026-11-15 the owner changed decision 4 (below).* The alternatives
stay listed for the record.

1. **How sure the node must be before it acts on a reset** (FR-006). Recommended: one
   reading is enough, as drafted. A false reset costs a re-initialisation, an aborted
   GoTo or Track, and one sync. The risks: a garbled reply of the right length (an even
   third status digit, a flipped position digit) passes US4's length check; a board
   that refuses `:q` is never sent `:W`
   (`crates/scope-node/src/mount/skywatcher/protocol.rs:105`), keeps its encoders on,
   and every fast hand push then counts as a reset (one reading of that night); and
   the 3.4°/s top rate (`crates/scope-node/src/mount/skywatcher/protocol.rs:14`) was
   never measured, so a board faster than 5.1°/s would trip the jump rule on every long
   GoTo. Instead: act only when a second read straight after shows the same cause; turn
   the jump rule off on an axis where the connect sent no `:W`, logging that once;
   measure the real board's goto speed before trusting the jump rule.
2. **Which moves a reset blocks** (FR-014, FR-015). Recommended: GoTo, Track and second
   sync refuse, and a slewing GoTo and running Track stop. The MCP direct axis moves
   `goto_axis_position` and `move_axis` stay allowed and are not stopped, although their
   minimum-altitude guard plans through the stale model
   (`crates/finder/src/goto.rs:163`). Instead: refuse them too, in
   `GoTo::check_axis_move` (`crates/finder/src/goto.rs:767`), and halt a running one in
   the MCP guard's halt loop (`crates/finder/src/mcp/guard.rs:97`).
3. **A move that waited across a re-initialisation** (FR-008). Recommended: a `goto` or
   `rate` that reaches the link while it re-initialises is dropped, and one that
   reaches it after goes to the new board; the console's abort (FR-015) and its
   `StopAll` follow within one poll. Instead: the link also drops a move that was
   queued before the reset and runs after it, logging it.
4. **Stop while the link is lost or initialising again** (FR-002, FR-018). *Settled
   2026-11-15: the owner chose "Always send stop".* Stop and the node's mount watchdog
   send the stop command to both axes over the last open stream to the board, the stream
   of the link's last successful connect, even while the link is lost (three failed
   reads) or initialising again, keeping 017's halt rule: `:K` on both axes before `:L`
   on either (`crates/scope-node/src/mount/skywatcher/protocol.rs:297`). `StopAll`
   answers OK only if the board acknowledges, and otherwise `UNAVAILABLE` with "the mount
   did not confirm the stop". Only when no stream to the board has been opened (no connect
   has succeeded since the node started, because `SkyWatcher::connect` takes the stream
   and drops it when it fails, `crates/scope-node/src/mount/skywatcher/protocol.rs:321`),
   or when the device can't be opened (it is gone, and the kept stream fails at once),
   does Stop answer `UNAVAILABLE` at once without the board hearing it. A connect attempt
   under way, the connect task's or a spawned re-initialisation's, gives the device up to
   Stop, which never waits seconds for it, and with no board connected Stop sends each
   command once, so a board that answers nothing costs it 1.2 s (plan.md § Interfaces).
   Rejected: the round-2 answer, Stop answering at once while the link has no board,
   because after a lost link the board may still be powered and running a `:J` move,
   such as an MCP `move_axis`; and Stop waiting for the connect sequence to finish, which
   takes seconds on a silent board.
5. **Tracking's end reason after a reset** (FR-015). Recommended: `the mount was
   power-cycled`, which the page shows as it is
   (`crates/finder/web/src/components/TrackControl.tsx:78`). Instead: a code such as
   `power_cycled` with an entry in the page's list, which changes the page.

## Owner approvals

Flipping this spec to `ready` approves the settled answer to every owner decision
above, and the following. Each is named in the FR that makes it. No existing test,
golden or CI file changes.

1. **The node starts without the mount** and tries again 2 s after each failed attempt
   (FR-001, FR-002). Until it first connects, Stop still clears queued moves and answers
   "the mount did not confirm the stop", as for any unconfirmed stop.
2. **Every reconnect after a lost link counts as a reset**, and so do a cleared init
   bit on an axis that read it set, and a jump in counts beyond 1.5 times the top
   rate's travel plus 1,000 counts (FR-006, FR-007). The first connect after start is
   not a reset.
3. **Three fields added to the node protocol**: `AxisState.mount_resets` (5) and
   `DescribeMountReply.firmware` (2) and `.board` (3) (FR-009, FR-012). All are
   additions; old consoles ignore them.
4. **GoTo, second sync and Track refuse to start after a reset until a new sync**, with
   the words "The mount was power-cycled: sync again first", and a slewing GoTo or
   running Track stops when a reset is seen (FR-014, FR-015).
5. **Wrong-length replies to status and position reads are retried**, sharing five
   transmissions in all, and each one's bytes are kept for the log (FR-016, FR-017).
6. **Two of the three gaps after a reset are closed, and one stays open** (FR-014,
   FR-015). *Settled 2026-11-15: the owner chose "Close (a) and (b)".* Both are closed
   within US3's writes:
   - (a) a reset seen while a GoTo is solving or correcting. GoTo compares the node's
     `mount_resets` with its baseline on a read it makes after each correction's plan
     (`crates/finder/src/goto.rs:1010`) and before sending that correction's
     `goto_axis` pair (`crates/finder/src/goto.rs:917`), and a difference ends the GoTo
     `failed` with a reason containing `power-cycled` and sends `StopAll` (US3-S7);
   - (b) tracking's keyhole slews (`crates/finder/src/tracking.rs:581`) and their waits
     in `at_rest` (`crates/finder/src/tracking.rs:509`). `at_rest` compares the count
     with the rate loop's baseline on every read, and a difference stops tracking with
     the power-cycled end reason before any further keyhole waypoint (US3-S8).

   Gap (c) stays open: the console's sync-needed flag is not persisted, so a console
   restarted after a reset never refuses; the node's README says to sync first (FR-007).
   Rejected: the round-2 default, leaving all three open.

   A narrower gap (d), found in round 3, also stays open by default (open owner question):
   tracking takes its baseline from the rate loop's first tick read, so a reset that
   lands after the console's last mount poll (every 250 ms) and before that read becomes
   the baseline and never stops that tracking, which then tracks on the old zero points,
   and may slew through the keyhole on them, until it is stopped. From its next poll the
   console still refuses GoTo, second sync and a new Track until a new sync. Instead: start the
   tracker's baseline from the session's last-seen count, as a GoTo takes its own
   (plan.md § Interfaces), which needs one new `Tracker` method within US3's writes.

## User Scenarios & Testing *(mandatory)*

The Rust scenarios of US1, US2 and US4 share one setup unless a scenario says otherwise:
- A `MountLink` (plan.md § Interfaces) over the emulator: one `Emulator` built with
  `EmulatorConfig::default()` (firmware 3.40 on board 0x90, auxiliary encoders,
  1,620,000 counts per revolution), and an opener that returns
  `Ok(handle.reopen(clock.clone()))`, so every reconnect reaches the same board. The
  stream `Emulator::new` returns is not used.
- Tokio's clock is paused (`#[tokio::test(start_paused = true)]`) and the node clock is a
  copy of the `Clock` in `crates/scope-node/tests/skywatcher.rs:18` (copied rather than
  included with `#[path]`, which would run `skywatcher.rs`'s tests again); "after N s"
  means `tokio::time::sleep` of N seconds.
- "Connected" means the test let 3 s pass after the link started and
  `link.connected()` reads true.
- "Polled" means `state(Axis::Azimuth)` and `state(Axis::Altitude)` once each, as the
  capture loop does for every frame. "Polls N times, sleeping 1 s before each" means N
  rounds of a 1 s sleep and then a poll.
- "After a GoTo on axis 1 to N counts" means the test called
  `link.goto(Axis::Azimuth, N)` and polled every 100 ms until the azimuth read at rest.
- "Moves on the wire" are the emulator's exchanges whose command starts with `:G`, `:S`,
  `:H`, `:M`, `:I` or `:J`. Commands are recorded with their carriage return, as
  `:F3\r`.

The console scenarios of US3 (S2 to S8) share one setup:
- An in-process console built as `crates/finder/tests/mcp/support.rs:188` builds its
  `Rig`, copied into the new test file because that `Rig` hard-wires its own private
  mount, with a test `Mount` that wraps the `SimMount`, records the commands it
  receives, and reports firmware `3.40`, board `0x90` and a reset count the test sets.
  It is built as `Rig::new(START, 50.0, 120.0, true)` with `START` 1,768,528,800
  (`crates/finder/tests/mcp/support.rs:97`) and target M42
  (`crates/finder/tests/mcp/support.rs:237`), as the existing GoTo and Track tests build
  it. Time is paused.
- The copied `Rig` has no helper for `/api` writes, so each `POST` is built as the
  `post` helper in `crates/finder/tests/stop/support.rs:232` builds it: a `host` header
  naming `localhost` on port 8080, and a JSON body, or an empty body where a scenario
  says so.
- "After a sync" means the model holds a sync, as `Rig::sync` waits for
  (`crates/finder/tests/mcp/support.rs:278`).
- "With frames stopped" means the test called `controls.stop_frames()`
  (`crates/scope-node/src/service.rs:62`) and then let 2 s pass, so no frame is still in
  flight.

### User Story 1 - The node starts without the mount (Priority: P1)

As the observer setting up in the dark, I power the Pi before the mount. The node starts
at once and serves the camera; when I switch the mount on, it connects by itself.

**Why this priority**: The node restarted 44 times before the mount answered, and every
other story needs a link that can connect after start.

**Independent Test**: `cargo test -p scope-node --test mount_link --locked`.

**Acceptance Scenarios**:

1. **Given** the setup with the emulator powered off (`handle.power_off()`) before the
   link starts, **When** 10 s pass, **Then** a committed test asserts that
   `link.connected()` is false, that at least one exchange was recorded, and that no
   exchange has a reply.
2. **Given** the same link, **When** the test calls `handle.power_on()` and 5 s pass,
   **Then** it asserts that `link.connected()` is true, that `link.axes()` reports
   1,620,000 counts per revolution on both axes, that the exchanges after power-on
   include `:F3\r`, `:W1050000\r` and `:W2050000\r` in that order, that
   `handle.auxiliary_enabled(1)` and `handle.auxiliary_enabled(2)` are false, and that
   `link.identity()` is `Some(MountIdentity { firmware: "3.40", board: "0x90" })`.
3. **Given** a `NodeService` built with `with_mount` from
   `SyntheticCamera::new(clock.clone())`, the clock, `"test-token"`, `Settings::default()`
   and a link whose emulator is powered off, after 5 s, **When** the test calls the
   service's `describe_mount`, `axis_state`, `goto_axis`, `rate_axis` and `stop_all`
   directly, `stop_all` inside `tokio::time::timeout(Duration::from_millis(10), ...)`,
   **Then** it asserts that `describe_mount` returns no axes, that `axis_state`,
   `goto_axis` and `rate_axis` fail with `FAILED_PRECONDITION` and the message
   `mount not connected`, that `stop_all`'s timeout returns `Ok` and the call fails with
   `UNAVAILABLE`, and that no move and no exchange whose command starts with `:K` or `:L`
   is on the wire: before the first connect Stop answers at once and sends nothing.
4. **Given** the same service, **When** the test takes the first frame from the service's
   `frames` stream, **Then** it asserts that the frame has neither `axis_start` nor
   `axis_end`.
5. **Given** the built `scope-node` binary on the real clock, with a config whose
   `listen` is a free loopback port (one the test bound and released), whose adapter is
   `simulated` and whose `[mount]` device is a path that does not exist, **When** a
   test starts it and waits 5 s, **Then** it asserts that the process is still running
   and that standard error contains `mount not connected`.
6. **Given** an `Emulator` on `EmulatorConfig::default()` and a `SkyWatcher` connected
   directly over `handle.reopen(clock.clone())`, with no link, **When** the test calls
   `handle.set_position(1, 500_000)` and reads the azimuth state, then calls
   `handle.inject_reply(b'f', 1, "=101\r", 2)` and reads it once, then calls
   `handle.power_cycle()` and reads it twice, **Then** it asserts that the first read's
   position is 500,000; that `handle.auxiliary_enabled(1)` is false before the cycle and
   true straight after it; that the `:f1\r` replies recorded from the injection on are
   `=101\r`, `=101\r` and `=000\r` (an injected reply survives the cycle, and the board
   then reads not initialised); and that the last `:j1\r` reply is `=000080\r`
   (position 0).

---

### User Story 2 - A power cycle initialises the board again (Priority: P1)

As the observer who switched the mount off and on, I find the node has initialised the
board again before anything moves, turned its encoders off again, and counted the reset.

**Why this priority**: An uninitialised board with its encoders on is how the night's
model probably went wrong without anyone seeing it.

**Independent Test**: `cargo test -p scope-node --test mount_reset --locked`.

**Acceptance Scenarios**:

1. **Given** the setup, connected and polled once, **When** the test calls
   `handle.power_cycle()` and the link is polled after 1 s, **Then** a committed test
   asserts that the exchanges after the cycle include `:F3\r`, `:W1050000\r` and
   `:W2050000\r`, that `link.resets()` is 1, that `handle.auxiliary_enabled(1)` is
   false, and that no move is on the wire after the cycle.
2. **Given** the setup, connected, after a GoTo on axis 1 to 100,000 counts, **When** the
   test calls `handle.power_off()`, polls five times sleeping 1 s before each, then
   calls `handle.power_on()` and polls six times sleeping 1 s before each, **Then** it
   asserts that `link.connected()` was false after the fifth poll, is true after the
   last, and that `link.resets()` is 1.
3. **Given** the setup, connected and polled once, with both axes at rest, **When** the
   test calls `handle.set_position(1, 500_000)` and polls after 1 s, **Then** it asserts
   that `link.resets()` is 1 and that the exchanges after the jump include `:F3\r`.
4. **Given** the setup, connected, **When** the test calls
   `link.goto(Axis::Azimuth, 16_200)` and polls every 100 ms until the azimuth reads at
   rest, **Then** it asserts that the azimuth reads 16,200, that `:J1\r` was sent, and
   that `link.resets()` is 0: travel within the top rate is not a jump.
5. **Given** a `NodeService` built with `with_mount` over the link once US2-S1's cycle
   and poll are done, **When** the test reads `axis_state` for both axes, **Then** it
   asserts that each reply's `mount_resets` is 1.
6. **Given** the link of US2-S2 after its fifth poll, before `power_on`, while
   `link.connected()` is false, **When** the test calls `link.goto(Axis::Azimuth, 1000)`
   and `link.rate(Axis::Altitude, 0.5)`, **Then** it asserts that no move is on the
   wire after `power_off` (the GoTo before it put its own on the wire).
7. **Given** the setup, connected, after a GoTo on axis 1 to 100,000 counts, **When** the
   test calls `handle.power_cycle()` and polls four times sleeping 1 s before each,
   **Then** it asserts that `link.resets()` is 1 and that `:F3\r` appears exactly once
   in the exchanges after the cycle: a cleared bit and a jump on the same read are one
   reset.
8. **Given** the setup, connected, **When** the test calls
   `handle.inject_reply(b'f', 1, "=000\r", 5)` before the link's first poll and polls
   five times sleeping 1 s before each, **Then** it asserts that `link.resets()` is 0
   and that `:F3\r` was sent exactly once in all, by the connect: a bit that has not
   read set on this board since the connect is not a reset.
9. **Given** the setup, connected and polled once, with `handle.drop_replies(b'e', 1, 10)`
   (enough to fail the re-initialisation and the connect task's next attempt), **When**
   the test calls `handle.power_cycle()`, polls 1 s later on a spawned task, and 0.5 s
   after that notes whether the poll is still running and then calls
   `link.stop_confirmed()`, `link.goto(Axis::Azimuth, 1000)` and
   `link.rate(Axis::Altitude, 0.5)`, each inside
   `tokio::time::timeout(Duration::from_millis(10), ...)`, **Then** it asserts that the
   poll was still running and `link.connected()` false before those calls, that each
   timeout returns `Ok`, `stop_confirmed` with true, that `:K1\r` and `:K2\r` were sent
   after the cycle and each answered `=\r`, that `link.resets()` is 0 when the poll
   returns, that 10 s later `link.connected()` is true and `link.resets()` is 1, and that
   no move is on the wire after the cycle: Stop reaches the board while a
   re-initialisation is under way, and a failed re-initialisation wakes the connect task
   and is counted once.
10. **Given** the setup, connected and polled once, **When** the test calls
    `handle.power_cycle()` and `handle.drop_replies(b'e', 1, 2)` (the re-initialisation
    then takes over 0.6 s), and 1 s later polls inside
    `tokio::time::timeout(Duration::from_millis(200), ...)`, as the console bounds its
    mount poll (`crates/finder/src/live.rs:318`), **Then** it asserts that the timeout
    elapsed, and that 5 s later `link.connected()` is true, `link.resets()` is 1 and
    `:F3\r` appears exactly once in the exchanges after the cycle: a read dropped by its
    caller does not cancel the re-initialisation it started.
11. **Given** the setup, connected and polled once, with `link.rate(Axis::Azimuth, 1.0)`
    sent (a `:J` move that runs until it is stopped) and then
    `handle.drop_replies(b'e', 1, 5)` (so the link can't connect again at once),
    `handle.drop_replies(b'f', 1, 10)` and `handle.drop_replies(b'f', 2, 5)`, **When**
    the test polls twice, so that three reads in a row fail and the link is lost while
    the board stays powered and the azimuth keeps moving, and 0.1 s later (the connect
    task's attempt then holds the device) calls `link.stop_confirmed()` inside
    `tokio::time::timeout(Duration::from_millis(100), ...)`, **Then** it asserts that
    `link.connected()` was false before the call, that the timeout returns `Ok(true)`,
    that the exchanges after the injection include `:K1\r` and `:K2\r`, each answered
    `=\r`, and that 10 s later `link.connected()` is true and two polls 1 s apart read the
    azimuth not `moving`, at the same position: Stop reaches a board the link has lost,
    and the connect attempt under way gives the device up to it.
12. **Given** the setup, connected and polled once, **When** the test calls
    `handle.power_off()`, polls twice (three reads in a row fail and the link is lost)
    and 0.1 s later (the connect task's attempt then holds the device) calls
    `link.stop_confirmed()` inside
    `tokio::time::timeout(Duration::from_millis(1500), ...)`, **Then** it asserts that
    the timeout returns `Ok(false)`, and that the exchanges after the power-off whose
    command starts with `:K` or `:L` are exactly `:K1\r`, `:K2\r`, `:L1\r` and `:L2\r` in
    that order, none answered: with no board connected Stop sends each command once, so a
    board that answers nothing costs it 1.2 s, not the 6 s of five tries each.
13. **Given** a second emulator and link, made lost by US2-S11's setup and two polls and
    not yet stopped, and, once it is lost, a `NodeService` built over that link with
    `with_mount` as US2-S5 builds one (built earlier, its watchdog would stop the board
    1 s later while it is still connected), **When** the test at once calls the service's
    `stop_all` directly inside `tokio::time::timeout(Duration::from_millis(500), ...)`,
    **Then** it asserts that `link.connected()` was false before the call, that the call
    answers OK and that `:K1\r` and `:K2\r` were sent after the injection, each answered
    `=\r`: the node's `StopAll` over a lost link answers OK when the board acknowledges.

---

### User Story 3 - The console knows the board and its resets (Priority: P2)

As the observer at the page, I see the board's firmware and identity, and after a power
cycle the console tells me to sync before it will GoTo or Track.

**Why this priority**: The session showed null for the board all night, and a GoTo
after an unnoticed reset aims with zero points from before it.

**Independent Test**: `cargo test -p finder --test mount_identity --locked`.

**Acceptance Scenarios**:

1. **Given** a `NodeService` built with `with_mount` over a `MountLink` whose opener
   returns `Ok(handle.reopen(clock.clone()))` of an `Emulator` built with
   `EmulatorConfig::default()`, with paused time and the test's own node clock, after
   3 s with `link.connected()` true, **When** the test calls the service's
   `describe_mount`, **Then** it asserts that the reply's `firmware` is `3.40` and its
   `board` is `0x90`; and for a `NodeService` with no mount, that both are empty
   strings.
2. **Given** the US3 console, **When** the test reads `GET /api/session` after 0.5 s,
   **Then** it asserts that `mount.firmware` is `3.40`, `mount.board` is `0x90`, and
   that `mount` has no `resets` key while the count is 0.
3. **Given** the US3 console after a sync, with frames stopped, **When** the test sets
   the reset count to 1 and reads the session after 0.5 s, **Then** it asserts that
   `mount.resets` is 1, that `POST /api/goto` (the session's target is already M42) and
   `POST /api/model/second-sync`, each with no body, each answer `409` with the error
   exactly `The mount was power-cycled: sync again first`, and that the test `Mount`
   recorded no `GotoAxis`.
4. **Given** the console of US3-S3, **When** the test starts frames again
   (`controls.start_frames()`) and waits until the model's latest sync time is later
   than the console time at which US3-S3's `POST /api/goto` was refused, **Then** it
   asserts that `POST /api/goto` answers `200` with `{"started":true}`.
5. **Given** the US3 console after a sync, with `POST /api/goto` accepted and
   `goto.state` `slewing` while the `SimMount`'s axes move, **When** the test sets the
   reset count to 1, **Then** it asserts that within 3 s `goto.state` is `failed` with a
   reason containing `power-cycled`, and that the test `Mount` recorded a `StopAll`
   after the count was set.
6. **Given** the US3 console after a sync, with `POST /api/track` `{"on":true}` accepted
   and `track.state` `tracking`, then frames stopped, **When** the test sets the reset
   count to 1, **Then** it asserts that within 2 s `track.state` is `off` with
   `track.ended_reason` containing `power-cycled` and the session shows `mount.resets`
   1, and that `POST /api/track` `{"on":true}` then answers `409` with the error exactly
   `The mount was power-cycled: sync again first`.
7. **Given** the US3 console after a sync, with `POST /api/goto` accepted, **When**
   `goto.state` first reads `solving` and the test pushes the `SimMount` 0.1° in azimuth
   (`rig.mount.push_by(0.1, 0.0)`, as `crates/finder/tests/goto_sim.rs:21` does) so that
   a correction follows, and then, when `goto.state` first reads `correcting` (the test
   reads `GET /api/session` every 10 ms), sets the reset count to 1, **Then** it asserts
   that within 3 s `goto.state` is `failed` with a reason containing `power-cycled`, and
   that the test `Mount` recorded a `StopAll` and no `GotoAxis` after the count was set:
   a correction planned through the stale zero points is never sent.
8. **Given** the US3 console built as the setup builds it but at the first whole minute
   after `START` at which M42 moved to declination 39° (as
   `crates/finder/tests/keyhole_sim.rs:115` moves it) stands at least 88.5° high by
   `simulator::sky::horizontal` with `simulator::Site::default()`, after a sync, and a
   `Tracker` built with `Tracker::new` over the console's own link (`AppState::link`,
   `crates/finder/src/api.rs:225`), a console clock on the rig's virtual clock, the
   session's model, the site at latitude 40° and longitude −75° and a 15° minimum
   altitude, then started on that target, which stands inside the zenith circle, **When**
   its state reads `keyhole_wait` and the test `Mount` has recorded the keyhole slew's
   `GotoAxis` pair, and the test sets the reset count to 1 while that slew runs, **Then**
   it asserts that within 2 s the tracker's state is `off` with an end reason containing
   `power-cycled`, and that the test `Mount` recorded a `StopAll` and no `GotoAxis` after
   the count was set: a keyhole slew's wait for rest reads the count, and no further
   waypoint is sent.

---

### User Story 4 - Garbled replies are retried and kept (Priority: P3)

As the owner reading the node's log after a night, I can see the bytes of every
wrong-length reply to a status or position read, and one garbled read no longer fails a
poll.

**Why this priority**: Garbled replies came every 6 to 8 minutes, and then every 1 to 2;
their cause is unknown until their bytes are seen.

**Independent Test**: `cargo test -p scope-node --test mount_garble --locked`.

**Acceptance Scenarios**:

1. **Given** the setup, connected, **When** the test calls
   `handle.inject_reply(b'f', 1, "=10\r", 1)` (a status two digits long) and polls,
   **Then** a committed test asserts that the azimuth state is not `moving`, that
   `link.health().invalid_replies` is 1, and that `link.health().last_invalid` is
   `Some("=10\\r")`: the five characters `=`, `1`, `0`, a backslash and `r`.
2. **Given** the setup, connected, **When** the test injects that reply 5 times for `f`
   on axis 1 and polls, **Then** it asserts that the azimuth state reads `moving: true`
   (a lost reading never looks like rest), that `invalid_replies` is 5, and that `:f1\r`
   was sent exactly five times after the injection.
3. **Given** the setup, connected, with `handle.set_position(2, 123_456)` called before
   the link's first poll, **When** the test injects `"=12\r"` once for `j` on axis 2 (a
   position two digits long) and polls, **Then** it asserts that the altitude position
   is 123,456 and not `moving`, that the `:j2\r` replies the emulator recorded after the
   injection are `=12\r` then `=40E281\r`, and that `invalid_replies` is 1.
4. **Given** a link over `EmulatorConfig { refuse_absolute: true, ..Default::default() }`,
   connected, **When** the test calls `link.goto(Axis::Azimuth, 16_200)`, **Then** it
   asserts that `link.health().goto_path[0]` is `None` before the GoTo and
   `Some(GotoPath::Relative)` after; and, after the same GoTo over a second link on the
   default board, `Some(GotoPath::Absolute)`.
5. **Given** the setup, connected, **When** the test calls
   `handle.drop_replies(b'J', 1, 1)` and `link.goto(Axis::Azimuth, 16_200)`, **Then** it
   asserts that `:J1\r` was sent exactly once: a motion start is never retried.
6. **Given** the setup, connected, with `"=10\r"` injected once for `f` on axis 1 and
   polled, then `link.goto(Axis::Azimuth, 16_200)` and a poll 2 s later, **When** the
   test calls `handle.power_cycle()`, injects `"=1\r"` once for `f` on axis 1 and polls
   1 s later, **Then** it asserts that `link.resets()` is 1, that
   `link.health().invalid_replies` is 2, that `last_invalid` is `Some("=1\\r")`, and
   that `goto_path` is `[Some(GotoPath::Absolute), None]`: the read that found the reset
   still counts its wrong-length reply, and the old board's path stands until the new
   board's first GoTo.

---

### Edge Cases

- **The serial adapter itself is unplugged.** Opening the device fails; the link tries
  again 2 s after each failure and logs the error once per change, not every try.
- **A brief power blip.** The board may answer the next read uninitialised without any
  timeout; the status bit catches it, because the bit read set before (US2-S1).
- **Firmware that never sets the bit.** The bit rule never fires on it, because the bit
  never read set (US2-S8). A blip too short to lose the link and too early for any move
  goes unseen; any longer one loses the link, and every reconnect counts as a reset
  (owner approval 2). One extra sync is the cost.
- **The console restarts after a reset.** It learns the count at its first poll and
  can't tell it from an old one, so it does not refuse; the next sync's slip check
  (014) still opens a new epoch. The node's README says a console restarted after a
  power cycle needs a sync before GoTo.
- **The Pi and the mount share one power supply.** A cycle restarts the node too; its
  count starts at 0 and its first connect is not a reset, so the console does not
  refuse. The console-restart advice in the README covers it.
- **A hand push with the board's encoders on.** It moves the counts faster than any
  motor could; the jump rule counts it as a reset and initialises the board again,
  which turns the encoders off when the board reports them (US2-S3). A board that
  refuses `:q` is never sent `:W`, keeps them on, and every fast push counts as a reset
  (owner decision 1).
- **A garbled reply of the right length.** It passes US4's length check and can read
  as a reset (owner decision 1).
- **A move in flight when the mount loses power.** The motors stop with the power. While
  the node's link is lost or initialising again it refuses `axis_state` with
  `mount not connected`, so a GoTo or Track whose read comes then ends
  `mount did not answer` (`crates/finder/src/goto.rs:925`) or `link_lost`; one whose read
  comes back across the reset ends with the power-cycled reason (US3-S5, US3-S6). Either
  way the motion stops, and once the node has connected again the console refuses until
  a new sync.
- **A reset just before tracking starts.** One that lands after the console's last mount
  poll and before tracking's first tick read becomes tracking's baseline and does not
  stop it; the console refuses from its next poll (owner approval 6, gap (d)).
- **A reset while a GoTo solves or corrects.** The read after the correction's plan sees
  it before that correction's slew is sent (US3-S7). A GoTo whose last solve lands within
  60″ ends `done` and sends nothing more.
- **A reset during a keyhole slew.** The keyhole's waits for rest read the count, so
  tracking stops before the next waypoint (US3-S8). The wait for the target to leave the
  circle reads nothing; the rate loop's next tick read stops tracking, after at most a
  backlash take-up and one second of tracking rates, neither of them a slew.
- **Stop while disconnected.** It clears the node's queued moves. Before the link's first
  connect it answers "the mount did not confirm the stop" at once and leaves the connect
  under way alone (US1-S3); the page already treats that as an unconfirmed stop (017).
  After a lost link, when the board may still be powered and running a `:J` move, or
  while the link initialises the board again, Stop still goes over the last connect's
  stream: OK when the board acknowledges (US2-S9, US2-S11, US2-S13), unconfirmed within
  1.2 s when it answers nothing (US2-S12), and unconfirmed at once when the device is
  gone (owner decision 4).
- **The console gives up on a read mid-re-initialisation.** The re-initialisation runs on
  in its own task and the link connects (US2-S10).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: With `[mount]` configured, `scope-node` SHALL start and serve the camera
  whether or not the mount answers, and SHALL try to open the device and run the
  connect sequence again 2 s after each failed attempt until it succeeds.
- **FR-002**: While the link is not connected, `DescribeMount` SHALL return no axes,
  `AxisState`, `GotoAxis` and `RateAxis` SHALL fail with `FAILED_PRECONDITION` and
  `mount not connected`, `StopAll` SHALL clear queued moves, frames SHALL carry no axis
  counts, and nothing SHALL be sent to the board but the connect sequence and Stop's
  commands (FR-018). Before the link's first connect, `StopAll` and the node's watchdog
  SHALL send nothing, and `StopAll` SHALL fail with `UNAVAILABLE` at once.
- **FR-003**: Each successful connect SHALL log one line naming the firmware, the baud,
  each axis's counts per revolution, timer, high-speed ratio and auxiliary-encoder
  support, and whether `:W` was sent, followed by the existing warnings.
- **FR-004**: `SkyWatcher::connect` and `SkyWatcher`'s `Mount` methods SHALL keep their
  signatures, and their exchanges when every reply is valid; the link wraps them.
- **FR-005**: `crates/scope-node/README.md` SHALL describe starting without the mount
  and FR-003's line.
- **FR-006**: The link SHALL treat as a reset: a status reply whose third digit's lowest
  bit is 0 on an axis where an earlier read since the last connect found it 1; any
  reconnect after the link was lost, which three failed state reads in a row mean; and
  an axis's counts moving between two good reads on the same board by more than
  1.5 × top rate × elapsed seconds × counts per degree + 1,000 counts. The first connect
  after start SHALL NOT count.
- **FR-007**: On a reset the link SHALL run the connect sequence again before sending any
  other command, count one reset per event however many causes one read shows, and log
  it with its cause. `crates/scope-node/README.md` SHALL describe resets, this line, and
  that a console started after a power cycle needs a sync before GoTo.
- **FR-008**: While connecting or initialising again, `goto` and `rate` SHALL send
  nothing to the board. Stop is FR-018's.
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
- **FR-014**: After the console has seen the node's `mount_resets` rise, `POST /api/goto`,
  `POST /api/model/second-sync` and `POST /api/track` SHALL answer `409` with
  `The mount was power-cycled: sync again first`, until the console records a sync from
  a frame whose exposure started after it saw the rise.
- **FR-015**: A GoTo SHALL end `failed` with a reason containing `power-cycled` and send
  `StopAll` when a read it makes while slewing, or the read it makes after a correction's
  plan and before sending that correction's `goto_axis` pair, carries a `mount_resets`
  other than the count the console had seen when it accepted the GoTo. Running tracking
  SHALL stop, with an end reason containing `power-cycled`, when the count changes on the
  rate loop's tick read or on any read of the keyhole's waits for rest, and then SHALL
  send no further keyhole waypoint (owner approval 6).
- **FR-016**: A status or position read whose framed reply has the wrong length SHALL be
  sent again, its tries sharing five transmissions in all; a motion start (`:J`) SHALL
  never be sent again.
- **FR-017**: The link SHALL count framed replies of the wrong length to status and
  position reads, keep the last one's bytes escaped, log each with its command, and
  record per axis whether GoTo used `:S` or the `:H`/`:M` fallback, logging the path
  when it changes on each board. `crates/scope-node/README.md` SHALL name both lines.
- **FR-018**: After the link's first connect, while it is lost or initialising again,
  `StopAll` and the node's watchdog SHALL send the stop to both axes over the stream of
  the link's last successful connect, in 017's order (`:K` on both axes before `:L` on
  either, `crates/scope-node/src/mount/skywatcher/protocol.rs:297`) and with one
  transmission per command, once any connect attempt under way has given the device up.
  `StopAll` SHALL answer OK only if each axis acknowledged, and otherwise `UNAVAILABLE`
  with `the mount did not confirm the stop`. Stop SHALL never wait on a connect, and a
  board that answers nothing SHALL cost it at most four reply timeouts (1.2 s); when the
  device is gone it SHALL answer at once (owner decision 4).

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
  poll of the board answering after a blip (US2-S1), and within 6 s of it answering
  after a lost link (US2-S2).

## Assumptions

- **Earlier features are in place.** 013, 014, 015 and 017 have landed. Every citation here
  was read at c1ed295.
- **Out of scope:**
  - dark-sky detection and exposure (spec 023);
  - saving live frames (drafted separately as 025-frame-capture);
  - finding the cause of the garbled replies: this spec only records them;
  - replies the transport cannot frame: they are retried as before, not counted;
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
  implements: [FR-006, FR-007, FR-008, FR-009, FR-018]
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
the node's service and `crates/finder`; US4 touches only the Sky-Watcher module, its
tests and the node's README, so they may run beside each other once US2 has merged.
