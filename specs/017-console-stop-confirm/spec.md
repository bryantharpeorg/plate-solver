---
state: draft
depends_on_landed: ["014-goto", "015-tracking", "016-mcp"]
# Drafted 2026-11-08 against `main` at 032b34d, from a review of the console's Stop. The
# owner decided that day to do this as its own spec, before the first powered GoTo at
# the telescope, whether or not the voice mode work goes ahead. It stays draft until the
# owner has reviewed it and given the approvals listed under "Owner approvals". Flipping
# it to `ready` is the act that dispatches it.
---

# Feature Specification: A Stop the Console Can Confirm

**Feature Branch**: `017-console-stop-confirm`

**Created**: 2026-11-08

**Status**: Draft

**Input**: The console's Stop makes one attempt to reach the scope node, gives it 200 ms,
ignores what happened and answers `{"stopped":true}`. A stall on the scope link of 200 ms
or more, but under the scope node's 1 s watchdog, can lose the stop while heartbeats stay
healthy, and the mount runs on. Before the first powered GoTo at the telescope, Stop has
to keep trying for a bounded time, say whether the mount acknowledged, and show the
observer when rest is not confirmed. This is a hardening spec, not a redesign of motion.

## Clarifications

### Session 2026-11-08 (review of `main` at 032b34d)

- Q: What does this trio read besides itself? → A: Nothing about the feature. `spec.md`
  holds every requirement and scenario, and `plan.md` holds every design decision, the
  interfaces, the shapes that existing tests pin and the alternatives rejected. A story
  that would change a decision escalates rather than re-deciding it.
- Q: What was found? → A: Six things, each checked against the code at 032b34d:
  1. One attempt, 200 ms, result discarded, at `crates/finder/src/api.rs:778`,
     `crates/finder/src/goto.rs:847`, `crates/finder/src/goto.rs:877`,
     `crates/finder/src/mcp/guard.rs:55`, `crates/finder/src/mcp/guard.rs:72`,
     `crates/finder/src/tracking.rs:216` and `crates/finder/src/tracking.rs:859`; the
     route then answers `{"stopped":true}` whatever happened.
  2. A rate move has no end on the motor board, so if the last stop of a `move_axis` is
     lost, `axis_move` reports `done` and the axis keeps going.
  3. `mount.moving` reads false on a telemetry timeout, so on its own it can't prove rest.
  4. A watchdog firing on the node isn't reported to the console. A throwaway probe at
     032b34d made the node ignore heartbeats for 1.1 s in the middle of a slew while the
     console kept getting its replies. The watchdog stopped the mount, the GoTo read the
     stopped axes as arrival, sent its next waypoint 1.15 s later and ended `done`.
  5. The 200 ms limit also cancels the node's own work. A second throwaway probe gave the
     mount a stop that takes 1 s and dropped the call at 200 ms: tonic dropped the node's
     handler, the mount's stop never finished, the axis was still moving 3 s later, and
     the next move was refused with `mount command discarded by Stop`, because
     `MountQueue::stop_all` left `stopping` set. One unanswered serial command costs
     300 ms (`REPLY_TIMEOUT`), so this needs no broken link, only one lost reply.
  6. The node's `StopAll` answers OK even when the motor board never confirmed. The
     Sky-Watcher adapter logs `mount stop failed` and discards the error.
- Q: Where does the retry live? → A: In the console, in one function that every stop
  goes through, and in a task of its own, so that neither a request's end nor a phone's
  lost connection can end it (FR-001, FR-002). The page and MCP clients already share
  `api::stop`.
- Q: Why 2 s, 200 ms and 50 ms? → A: The attempt keeps today's 200 ms. Attempts run one
  at a time with a 50 ms pause, so a timed-out attempt is followed by the next at 250 ms
  intervals and an attempt is nearly always in flight when a stall ends. The window has to
  outlast the 1 s watchdog with room for an answer. Past 1 s of silence the node's
  watchdog has stopped the mount anyway, so 2 s is enough. A stall longer than the
  window is the case the reply reports.
- Q: Why does the reply carry `acknowledged` only when it's false? → A: Five existing
  assertions pin the acknowledged body exactly: `crates/finder/tests/mcp_motion.rs:444`,
  `crates/finder/tests/mcp_motion.rs:489`, `crates/finder/tests/mcp_motion.rs:493`,
  `crates/finder/tests/mcp_tracking.rs:309` and
  `crates/finder/tests/mcp_axis_moves.rs:437`. Leaving the reply byte-for-byte unchanged
  when the node answered means no existing client and no existing test changes, and the
  case the page and an MCP client need to catch is the negative one (owner approval 1).
- Q: What does `acknowledged` mean? → A: That the scope node answered `StopAll` OK, and,
  once US3 has landed, that the motor board confirmed the stop on each axis, or that the
  node has no mount. It doesn't mean the axes are at rest: a stop decelerates.
- Q: What proves rest? → A: A session read, received after Stop's reply, with
  `mount.connected` true and `mount.moving` false. `moving` is only ever true evidence
  when `connected` is true (finding 3).
- Q: Why is the watchdog count on `AxisState` and not on `HeartbeatReply`? → A:
  `HeartbeatReply` is built with a struct literal in two existing tests
  (`crates/scope-link/tests/link.rs:83`, `crates/finder/tests/aim_cli.rs:51`), so a new
  field there would need those tests edited. `AxisState` is built only in `service.rs`.
  It's also the right place: the GoTo reads the count in the same reply that says the
  axes are still, so a firing can't be mistaken for arrival.
- Q: What is left for later? → A: The items under "Out of scope" in § Assumptions.

## Owner approvals

Flipping this spec to `ready` approves the following. Each is named in the FR that makes
it. No existing test, golden or CI file changes.

1. **`acknowledged` appears only when false.** `POST /api/stop` and the MCP `stop` and
   `stop_all` tools answer exactly `{"stopped":true}` when the node acknowledged, and
   `{"stopped":true,"acknowledged":false}` when it didn't. Always sending the field would
   give a reader positive evidence instead of an absence, but it needs the five
   assertions named in § Clarifications edited (FR-004, FR-005).
2. **Stop's reply can take about 2 s on a bad link.** It usually answered within about
   200 ms before. The first attempt is still sent at once, and the page's Stop stays enabled
   while the reply is pending. The 2 s window is one constant (FR-001, FR-003).
3. **An additive scope link field, `AxisState.watchdog_stops`** (FR-012).
4. **`StopAll` can now answer an error.** The node answers `UNAVAILABLE` when the motor
   board doesn't confirm a stop. It answered OK in every case before (FR-011).
5. **GoTo's halt is published before its last stop is confirmed.** The session reads
   `halted` at once, and the GoTo stays busy until the stop is acknowledged or the window
   closes (FR-007).
6. **The page's two messages**, in FR-014 and FR-015. The alert tells the observer to
   switch the mount's power off if it keeps moving.

## User Scenarios & Testing *(mandatory)*

All Rust scenarios share one setup:
- Every scenario that names the simulator runs it as 016 does: over an in-memory
  transport, on a `VirtualClock`, under `#[tokio::test(start_paused = true)]`, with the
  console reached through `finder::router` and `tower::ServiceExt::oneshot`. The simulated
  mount is wrapped in a recording mount, as `tests/mcp/support.rs` does: that is "the
  command log".
- "The stop gate" is a test `ScopeNodeStop` service in front of the real node's, as
  `StopLink` in `tests/tracking_sim.rs` is. It records the virtual time of every `StopAll`
  it receives ("the attempt log"), and for each one it passes it to the node, holds it
  unanswered, or answers `UNAVAILABLE`. `hold` keeps the node from seeing the call at all.
- A scenario gives exact attempt times only when nothing else is stopping the mount. In
  those scenarios the axis is started by calling `rate` on the simulated mount directly,
  so that no console motion runs. Attempt times have a tolerance of ±0.05 s.
- Every scenario that asserts a mount still moves or never moved also shows, in the same
  test, the opposite outcome on the same rig.
- No test moves real hardware or calls a real LLM.

### User Story 1 - Stop is repeated until the scope node answers, and the reply says whether it did (Priority: P1)

As the observer, I press Stop on a link that has stalled. The console sends it at once,
and keeps sending it until the scope node answers or a bounded time has passed. The reply
tells me whether the node answered, and an MCP client gets the same reply.

**Why this priority**: The first powered GoTo must not depend on one 200 ms attempt.

**Independent Test**: `cargo test -p finder --locked --test stop_retry` runs
`crates/finder/tests/stop_retry.rs` against the simulator in virtual time.

**Acceptance Scenarios**:

1. **Given** a connected simulator with an axis turning and no console motion running,
   **When** a committed test posts `POST /api/stop` with the gate passing, **Then** it
   asserts that:
   - the status is 200 and the body is exactly `{"stopped":true}`;
   - the attempt log has one entry, within 0.1 s, and the command log one `StopAll`;
   - the axis is at rest within 2 s.
2. **Given** the same rig with the gate holding the first two attempts, **When** a
   committed test posts `POST /api/stop`, **Then** it asserts that the attempt log has
   entries at 0, 0.25 and 0.5 s and no more, the body is exactly `{"stopped":true}`, the
   command log has one `StopAll`, and the axis is at rest within 2 s of the third.
3. **Given** the same rig with the gate holding every attempt, **When** a committed test
   posts `POST /api/stop`, **Then** it asserts that:
   - the attempt log has 8 entries, the k-th at (k − 1) × 0.25 s;
   - the status is 200, the body as JSON equals
     `{"stopped":true,"acknowledged":false}`, and the reply arrives at 1.95 s ± 0.1 s;
   - the axis is still moving (oracle: the simulator's axis state), which scenario 2's
     passing gate shows is the gate's doing.
4. **Given** the same rig with the gate answering `UNAVAILABLE` to every attempt, **When**
   a committed test posts `POST /api/stop`, **Then** it asserts that the body as JSON
   equals `{"stopped":true,"acknowledged":false}`, that no two attempts are closer than
   0.045 s, and that there are at most 41.
5. **Given** the same rig with the gate holding every attempt, **When** a committed test
   drops the `POST /api/stop` request's future after 0.3 s of virtual time and sets the
   gate passing, **Then** it asserts that within the next 0.5 s the attempt log gains an
   entry, the command log has one `StopAll` and the axis comes to rest.
6. **Given** a connected simulator whose heartbeats have gone silent until
   `LinkStatus.connected` is false, **When** a committed test posts `POST /api/stop`,
   **Then** it asserts that the body as JSON equals
   `{"stopped":true,"acknowledged":false}` and that the reply arrives within 2.05 s.
7. **Given** a console built with no scope link, **When** a committed test posts
   `POST /api/stop`, **Then** it asserts that the body is exactly `{"stopped":true}` and
   that no virtual time passed.
8. **Given** a tracker retained from an ended Track run and a GoTo slewing, with the gate
   holding every attempt, **When** a committed test posts `POST /api/stop` without
   awaiting it and reads the session 0.1 s of virtual time later, **Then** it asserts that
   `goto.state` is `halted` with the reason `the observer stopped it`, and that the
   attempt log has at least one entry from before that read.
9. **Given** the gate holding every attempt, **When** a committed test calls the MCP tool
   `stop_all`, **Then** it asserts that `structuredContent` and the JSON in `content[0]`
   both equal `{"stopped":true,"acknowledged":false}`, that the call log's `stop_all` line
   carries `"acknowledged":false`, and that `tools/list`'s description of `stop` and of
   `stop_all` each mentions `acknowledged`.
10. **Given** the gate holding every attempt, **When** a committed test sends a JSON-RPC
    batch naming `stop` and `stop_all` from a foreign `Origin`, **Then** it asserts that
    the door answers 403 and that the attempt log has the same 8 entries as scenario 3:
    one round of attempts, not two.
11. **Given** a recording mount whose `state` call is held, and a `POST /api/goto` waiting
    on it, which holds the console's motion lock, **When** a committed test posts
    `POST /api/stop`, **Then** it asserts that Stop's reply arrives within 0.1 s of virtual
    time, and before the `POST /api/goto` answers.

---

### User Story 2 - A motion's own end is repeated the same way, and a rate move never reports done on a lost stop (Priority: P1)

As the console, I end a GoTo, a direct axis move and a tracking run with the same retried
stop as the Stop button. A rate move that I couldn't stop is not reported as finished.

**Why this priority**: The motor board never ends a rate move on its own, so a lost last
stop leaves an axis turning with nothing watching it.

**Independent Test**: `cargo test -p finder --locked --test stop_motion_ends` runs
`crates/finder/tests/stop_motion_ends.rs` against the simulator in virtual time.

**Acceptance Scenarios**:

1. **Given** a GoTo slewing with the gate holding the first two attempts, **When** a
   committed test silences the heartbeats, **Then** it asserts that `goto.state` is
   `halted` with a reason containing `lost link` before the third attempt arrives, and
   that the third attempt reaches the node.
2. **Given** a GoTo halted by a lost link with the gate holding every attempt and its
   retry still running, **When** a committed test posts `POST /api/goto`, **Then** it
   asserts 409 with `a GoTo is already running`.
3. **Given** a `move_axis` rate move of 1 s accepted, with the gate holding the first two
   attempts of its final stop, **When** the move's duration passes, **Then** a committed
   test asserts that `axis_move.state` is `moving` until the third attempt, then `done`
   with no `reason`, and that the command log has one `StopAll` and the axis is at rest.
4. **Given** the same move with the gate holding every attempt, **When** the retry window
   closes, **Then** a committed test asserts that `axis_move` equals
   `{"tool":"move_axis","state":"halted","reason":"the mount did not confirm the stop"}`,
   that it never read `done`, and that the simulated axis is still moving, which
   scenario 3 shows is the gate's doing.
5. **Given** a `move_axis` rate move running with the gate holding every attempt, **When**
   a committed test posts `POST /api/stop`, **Then** it asserts that `axis_move` ends
   `halted` with the reason `the observer stopped it`.
6. **Given** a recording mount whose `rate` call never completes, **When** a committed
   test calls `move_axis` with the gate holding the first attempt, **Then** it asserts
   that the tool answers `mount did not answer` and that the attempt log has two entries.
7. **Given** Track running with the gate holding the first two attempts, **When** a
   committed test posts `POST /api/stop`, **Then** it asserts that the body is exactly
   `{"stopped":true}`, that the first attempt is within 0.1 s, that at least three
   attempts arrive, that the axes come to rest and that `track.ended_reason` is `stopped`.
8. **Given** Track running with the gate holding every attempt, **When** a committed test
   posts `POST /api/stop`, **Then** it asserts that the body as JSON equals
   `{"stopped":true,"acknowledged":false}` and that the reply arrives within 2.5 s of
   virtual time.
9. **Given** a `Tracker` built and never started, with the gate holding the first two
   attempts, **When** a committed test awaits `Tracker::stop`, **Then** it asserts that
   the attempt log has entries at 0, 0.25 and 0.5 s.

---

### User Story 3 - The scope node finishes every stop it starts, and acknowledges only what the motor board confirmed (Priority: P1)

As the console, I can rely on an OK from `StopAll`. A stop I gave up waiting for still
finishes on the node, and an OK means the motor board answered.

**Why this priority**: A retry that cancels the node's half-done stop every 200 ms can
never get past a board that is slow to answer, and an OK that means nothing makes the
reply in US1 untrue.

**Independent Test**: `cargo test -p simulator --locked --test stop_finishes` and
`cargo test -p scope-node --locked --test skywatcher_stop`.

**Acceptance Scenarios**:

1. **Given** a node whose mount takes 1 s of virtual time to stop, **When** a committed
   test calls `StopAll` under a 200 ms timeout and lets it expire, **Then** it asserts
   that the mount's stop finishes, that both axes come to rest, and that a `rate_axis`
   sent after it is accepted.
2. **Given** that mount, **When** a committed test sends three `StopAll` calls under a
   200 ms timeout, 250 ms apart, **Then** it asserts that every stop finishes and that a
   later `rate_axis` is accepted.
3. **Given** the Sky-Watcher adapter on its emulator, turning on both axes, **When** a
   committed test awaits `stop_confirmed`, **Then** it asserts `true`, with `:K1\r` and
   `:K2\r` on the wire.
4. **Given** the emulator with `silent_stop_axis: Some(1)`, **When** a committed test
   awaits `stop_confirmed`, **Then** it asserts `true`, and that `:L1\r` follows `:K2\r`.
5. **Given** the emulator dropping every reply to `K` and `L` on axis 2, **When** a
   committed test awaits `stop_confirmed`, **Then** it asserts `false`, and that axis 1
   still received `:K1\r`.
6. **Given** a node with a mount whose `stop_confirmed` is `false`, **When** a committed
   test calls `StopAll`, **Then** it asserts code `Unavailable` and the message
   `the mount did not confirm the stop`. A mount that keeps the trait's default answers
   OK, and so does a node with no mount.

---

### User Story 4 - The scope node reports a watchdog firing, and a running GoTo halts (Priority: P1)

As the console, I learn that the node's watchdog stopped the mount in the same reply that
says the axes are still, so I don't take the stop for arrival and send the next leg.

**Why this priority**: The owner's first powered GoTo will run over Wi-Fi. A GoTo that
continues after the node stopped the mount for losing the link is moving a telescope on
a link just shown to be unreliable.

**Independent Test**: `cargo test -p simulator --locked --test watchdog_report` and
`cargo test -p finder --locked --test watchdog_goto`.

**Acceptance Scenarios**:

1. **Given** a node with a mount, **When** a committed test reads `AxisState`, lets the
   heartbeats go silent for 1.1 s, reads it again, restores the heartbeats, and repeats
   the silence, **Then** it asserts that `watchdog_stops` reads 0, 1 and 2, with an
   explicit `StopAll` between the readings leaving it unchanged.
2. **Given** a GoTo slewing with the console's heartbeats healthy, **When** a committed
   test advances only the node's injected clock by 1.2 s, **Then** it asserts that:
   - the command log gains the watchdog's `StopAll`, and the axes come to rest;
   - `goto.state` is `halted` with the reason `halted by the scope node's watchdog`;
   - the command log has no go-to after that `StopAll`;
   - `LinkStatus.connected` stayed true throughout.
3. **Given** a node whose watchdog fired during a silent link and whose link has come
   back, **When** a committed test starts a GoTo, **Then** it asserts that the GoTo ends
   `done` within 1′ of the target (oracle: the simulator's true eyepiece direction).
4. **Given** Track armed and waiting for a GoTo, **When** a committed test advances only
   the node's clock by 1.2 s, **Then** it asserts that Track ends with `ended_reason`
   `halted by the scope node's watchdog`.

---

### User Story 5 - The page says when the mount has not confirmed Stop (Priority: P2)

As the observer, I can see when Stop was not confirmed, the Stop button stays where it is
until rest is proved, and the page tells me what to do.

**Why this priority**: Today the button disappears exactly when rest isn't confirmed:
`mount.moving` reads false on a telemetry timeout, `mountMayMove` turns false and Stop
returns nothing, taking its error message with it.

**Independent Test**: `npm test` in `crates/finder/web` runs
`src/components/StopConfirm.test.tsx`.

**Acceptance Scenarios**:

1. **Given** a `StopButton` pressed while `mount.moving` was true, with a reply of
   `{stopped: true, acknowledged: false}`, **When** a committed Vitest test rerenders it
   with `{connected: false, moving: false}`, **Then** it asserts that an element with role
   `alert` contains exactly FR-014's message, and that Stop is present and not disabled.
2. **Given** a `StopButton` pressed while `mount.moving` was true, whose request failed,
   **When** a committed Vitest test rerenders it with `{connected: false, moving: false}`,
   **Then** it asserts that the alert reads the error's message and that Stop is present.
3. **Given** a `StopButton` pressed while `mount.moving` was true, with a reply of
   `{stopped: true}`, **When** a committed Vitest test rerenders it with
   `{connected: false, moving: false}`, **Then** it asserts that FR-015's message is
   present with no `alert` or `status` role, and that Stop is present.
4. **Given** a `StopButton` pressed while `{connected: true, moving: true}`, with a reply
   of `{stopped: true}`, **When** a committed Vitest test reads it, **Then** it asserts
   that neither message is present and that Stop is present.
5. **Given** an unconfirmed `StopButton`, **When** a committed Vitest test rerenders it
   with a new `mount` object reading `{connected: true, moving: false}`, **Then** it
   asserts that the alert and Stop are gone. Rerendering with the object it held when the
   reply arrived leaves both in place, and so does a new object reading
   `{connected: false, moving: false}`.
6. **Given** an unconfirmed `StopButton`, **When** a committed Vitest test presses Stop
   again while the first request is pending, **Then** it asserts that Stop is not disabled
   and that `fetch` was called twice with `("/api/stop", { method: "POST" })`.
7. **Given** the rebuilt page, **When** the `test` gate runs `npm run build` and
   `git status --porcelain -- dist | (! grep .)`, **Then** the gate passes, and
   `DESIGN.md` § Stop names both messages and the rule that keeps Stop visible.

---

### Edge Cases

- **Two presses** start two rounds of attempts, each bounded. `StopAll` is safe to repeat.
- **A newer GoTo.** If a round from an earlier Stop reaches the node after a newer GoTo
  has started, Stop wins and that GoTo reads the stop as arrival, as with a `StopAll`
  from another console today. The window for this is a stalled link's last 250 ms. Not
  handled here.
- **The link drops during a round.** Attempts go through the newest client the link
  manager offers. A caller that holds a client keeps using it after the manager has
  given the link up, as `GoTo::run` does today. No client at all means no attempt: the
  round waits for one or for its window to close.
- **An older scope node** with a newer console sends `watchdog_stops` as 0 forever, and
  answers `StopAll` as before, so GoTo behaves as it does today. A newer node with an
  older console is ignored. Both halves are needed for US3 and US4 to take effect.
- **The node restarts during a GoTo.** The count goes back to 0, which differs from the
  baseline, so the GoTo halts. The link would already have dropped.
- **Track off** answers with the session and doesn't carry `acknowledged`. Its stop is
  still repeated, because it runs the same `api::stop`.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The console SHALL send every stop through one function that sends
  `StopAll` until the node answers OK or the window closes. The first attempt starts at
  once. An attempt gets 200 ms. Attempts run one at a time with a 50 ms pause between
  them. No attempt starts within 50 ms of the end of a 2 s window. The function reports
  whether an attempt was answered OK.
- **FR-002**: That function SHALL run in a task of its own, so that the end of the HTTP
  request or the MCP call that started it can't end it.
- **FR-003**: `api::stop` SHALL end tracking and cancel a GoTo without either waiting on
  the other, and SHALL start its first attempt before waiting on anything. Its answer
  waits for its round to end.
- **FR-004**: `POST /api/stop` SHALL always answer 200. The body SHALL be exactly
  `{"stopped":true}` when an attempt was answered OK or the console has no scope link,
  and otherwise a JSON object equal to `{"stopped":true,"acknowledged":false}`.
- **FR-005**: The MCP tools `stop` and `stop_all` SHALL answer with the same JSON as the
  route, whether the door or the tool carried out the stop. A batch makes one round of
  attempts. Each tool's description SHALL say what `acknowledged: false` means.
- **FR-006**: Stop SHALL stay outside the door's checks and SHALL take no motion lock.
- **FR-007**: A GoTo that ends with a halt or a failure SHALL publish that state before
  its final stop's round begins, and SHALL stay busy until the round ends.
- **FR-008**: A direct axis move's final stop, and the stop after a failed start, SHALL
  use that function. A move whose result was success and whose final stop wasn't answered
  SHALL end `halted` with the reason `the mount did not confirm the stop`. A move that
  already had a reason keeps it. `axis_move.state` SHALL read `moving` until the round
  ends.
- **FR-009**: `Tracker::stop` and the tracking run's exit SHALL use that function.
  `Session::stop_tracking` SHALL share one deadline across its two calls to the tracker.
- **FR-010**: The node SHALL carry a `StopAll` it has started through to the end even if
  the caller goes away.
- **FR-011**: The `Mount` port SHALL gain a provided method, `stop_confirmed`, that the
  Sky-Watcher adapter overrides to report whether the board answered on each axis.
  `StopAll` SHALL answer `UNAVAILABLE` with `the mount did not confirm the stop` when it
  reports `false`, and OK otherwise, including with no mount.
- **FR-012**: `AxisState` SHALL gain `uint64 watchdog_stops`, the number of times the
  node's watchdog has stopped the mount since the node started. The node SHALL read it
  after reading the axis, and SHALL increment it before commanding the stop.
- **FR-013**: A running GoTo SHALL read a baseline `watchdog_stops` before its first leg,
  and compare every `AxisState` it reads while waiting for arrival against it, before
  treating the axes as arrived. A different value SHALL end the GoTo as `halted` with
  the reason `halted by the scope node's watchdog`.
- **FR-014**: When a Stop reply carries `acknowledged: false`, the page SHALL show this
  as an alert, in the place Stop's errors appear today:

  `The mount did not confirm Stop and may still be moving. Press Stop again. If it keeps moving, switch off the mount's power.`

  A failed request SHALL keep today's error text.
- **FR-015**: When a Stop reply didn't say `acknowledged: false` and the latest session
  read has no live mount reading (`connected` isn't true), the page SHALL show this in
  the same place, with no `alert` or `status` role:

  `Stop sent. Waiting for the mount to report that it has stopped.`
- **FR-016**: After a press, Stop SHALL stay visible whatever `mountMayMove` says, until
  a session read received after the reply shows `connected` true and `moving` false. That
  read clears the messages. `mountMayMove`'s signature and rule SHALL not change.
- **FR-017**: `crates/finder/README.md`, the module documentation in
  `crates/finder/src/lib.rs` and `crates/finder/src/api.rs` SHALL describe the reply and
  the retry. The README's list of halt reasons SHALL gain the watchdog's in US4.
- **FR-018**: `DESIGN.md` § Stop and `.impeccable/design.json` SHALL record the messages
  and the rule that keeps Stop visible, and `crates/finder/web/dist` SHALL be the build of
  its source.

### Key Entities

- **Stop round**: the attempts one caller makes through FR-001's function.
- **Acknowledged**: the node answered `StopAll` OK in a round.
- **Watchdog stops**: `AxisState.watchdog_stops`, a count kept by the node for as long
  as it runs.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario above has a committed test that asserts it.
  `cargo test --workspace --all-targets --all-features --locked` and `npm test` pass.
- **SC-002**: With every `StopAll` unanswered, a Stop answers within 2.5 s of virtual time
  and says it wasn't acknowledged. No Stop waits unbounded on the mount.
- **SC-003**: A stall that ends inside the window loses no stop: an attempt reaches the
  node and the axes come to rest.
- **SC-004**: No existing test, golden, `scripts/ci/` file, `.github/workflows/ci.yml` or
  `CODEOWNERS` entry is edited, and workspace line coverage stays at or above 80%.
- **SC-005**: In `crates/finder/src`, the only `MountClient::stop_all` call left outside
  FR-001's function is the keyhole's in `tracking.rs`, which already fails the run on an
  error.

## Assumptions

- **Earlier features are in place.** 014, 015 and 016 have landed. Every citation here
  was read at 032b34d.
- **Out of scope:**
  - direct moves and tracking reacting to a watchdog firing. They're at rest or
    harmlessly refreshed afterwards, and neither sends a next leg;
  - a `goto.reason` that says a GoTo's final stop wasn't acknowledged;
  - reporting `acknowledged` on the Track off route;
  - sharing the unconfirmed state with other phones and laptops, or keeping it across a
    page reload;
  - repeating the watchdog's single stop on the node;
  - making `MountMonitor` report an unknown `moving` on a timeout;
  - the voice mode work.
- **Outside the factory:** building and installing the new `scope-node` on the Pi, and
  the owner's first powered GoTo at the telescope.
- **The factory's gates and CI:**
  - The factory's gates are `lint` and `test`, copied in plan.md § Verification.
  - CI's other jobs run only in the merge queue, so no scenario's Then clause rests on
    them.

## Work Graph

```yaml
US1:
  depends_on: []
  implements: [FR-001, FR-002, FR-003, FR-004, FR-005, FR-006, FR-017]
US2:
  depends_on: []
  depends_on_merged: [US1]
  implements: [FR-007, FR-008, FR-009]
US3:
  depends_on: []
  implements: [FR-010, FR-011]
US4:
  depends_on: []
  depends_on_merged: [US1, US2, US3]
  implements: [FR-012, FR-013]
US5:
  depends_on: []
  depends_on_merged: [US1]
  implements: [FR-014, FR-015, FR-016, FR-018]
```

Every edge is a merge edge, because each dependent uses what its predecessor creates:

| Story | Builds on |
|---|---|
| US2 | US1's retrying function in `link.rs`, and its stop gate in `tests/stop/support.rs` |
| US4 | US2's `GoTo::run` and `motion`, US3's `service.rs`, US1's README text and test support |
| US5 | US1's reply, which the page reads |

US3 touches only `crates/scope-node`, so it may run beside US1 and US2. US5 touches only
`crates/finder/web/`, `DESIGN.md` and `.impeccable/design.json`.
