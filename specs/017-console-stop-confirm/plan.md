# Implementation Plan: A Stop the Console Can Confirm

**Branch**: `017-console-stop-confirm` | **Date**: 2026-11-08 | **Spec**: [spec.md](spec.md)

This plan records every design decision for the feature. A story that would change one
escalates rather than re-deciding it.

## Summary

Put one retrying stop in the console's `LinkManager` and send every stop through it:
first attempt at once, 200 ms per attempt, 50 ms between them, for at most 2 s, in a task
of its own. `POST /api/stop` and the MCP stops report whether the node answered, leaving
the acknowledged body exactly as it is. A direct move whose last stop isn't answered ends
`halted`, never `done`. On the node, a stop that has started runs to its end whether or
not the console is still waiting, and an OK from `StopAll` means the motor board
confirmed it. A new count on `AxisState` lets a running GoTo see that the watchdog
stopped the mount. The page keeps Stop on screen, with a message, until a fresh mount
reading proves rest.

Nothing here changes how a GoTo plans, how tracking computes rates, or what the door
checks.

## Technical Context

**Language/Version**: Rust, MSRV 1.96, stable in CI. TypeScript with React 19 for the
page.

**Primary Dependencies**: nothing new. `tokio`, `tonic` and `axum` as they are.

**Storage**: nothing new is persisted. No field is added to the session.

**Testing**:
- Rust integration tests in new files, with a shared module
  `crates/finder/tests/stop/support.rs`, in the shape of `crates/finder/tests/mcp/support.rs`.
- Simulator rigs on a `VirtualClock`, under `#[tokio::test(start_paused = true)]`.
- Vitest for the page, in a new file. No new Playwright test: the simulator can't stall
  a link, and the logic is the component's.

**Target Platform**: the `finder` binary in WSL2 on the Surface, and `scope-node` on the
Pi.

**Constraints**:
- `RUSTFLAGS="-D warnings"`, `--all-features` clippy and rustdoc, and the red-only night
  mode.
- `web/dist` is committed and must be the build of its source.
- No test moves real hardware.

**Scale/Scope**: seven stop sites, one scope link, one Stop button.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Nothing touches the solver or `crates/math-core`. |
| II. Never weaken a gate | No existing test, golden, `scripts/ci/`, `ci.yml`, `deny.toml` or `CODEOWNERS` is edited. New tests go in new files. `crates/finder/tests/stop/support.rs` is this spec's own shared test module; US2 may add new items and may not change or remove any item US1 put there. |
| III. Tests first, independent expectations | Oracles are the attempt log, the recording mount's command log, the simulator's axis state and true eyepiece direction, and the emulator's serial bytes. |
| IV. Critical paths | No `unsafe`, no dependency change, no database-format change, and nothing under `proto/`. The one proto change is additive, in `crates/scope-link/proto`, named for the owner (spec approval 3). |
| V. Standalone Rust, honest licensing | Unaffected. |

## Project Structure

```text
crates/scope-link/proto/scope_node/v1/scope_node.proto   # US4: AxisState.watchdog_stops
crates/scope-node/
├── src/mount/mod.rs                     # US3: provided method stop_confirmed
├── src/mount/queue.rs                   # US3: stop_all returns whether it was confirmed
├── src/mount/skywatcher/protocol.rs     # US3: stop_confirmed from halt()
├── src/mount/watchdog.rs                # US4: the count
├── src/service.rs                       # US3: StopAll in a task, UNAVAILABLE; US4: the field
└── tests/skywatcher_stop.rs             # US3
crates/simulator/tests/
├── stop_finishes.rs                     # US3
└── watchdog_report.rs                   # US4
crates/finder/
├── README.md                            # US1: the reply; US4: the halt reason
├── src/link.rs                          # US1: the retrying stop and its constants
├── src/api.rs                           # US1: stop; docs
├── src/lib.rs                           # US1: docs
├── src/goto.rs                          # US1: stop; US2: run; US4: motion
├── src/mcp/door.rs, src/mcp/tools.rs    # US1: the real reply reaches the tool
├── src/mcp/guard.rs                     # US2: both stops, and `halted`
├── src/tracking.rs, src/session.rs      # US2: both stops, one shared deadline
├── tests/stop/support.rs                # US1 creates; US2 only adds
├── tests/stop_retry.rs                  # US1
├── tests/stop_motion_ends.rs            # US2
├── tests/watchdog_goto.rs               # US4
└── web/
    ├── src/api.ts                       # US5: postStop
    ├── src/components/StopButton.tsx    # US5
    ├── src/components/StopConfirm.test.tsx  # US5
    └── dist/                            # US5: rebuilt
DESIGN.md, .impeccable/design.json       # US5
```

## Interfaces

### The retrying stop (`crates/finder/src/link.rs`)

```rust
/// One attempt's limit, the pause between attempts and the whole round.
pub(crate) const STOP_ATTEMPT: Duration = Duration::from_millis(200);
pub(crate) const STOP_PAUSE: Duration = Duration::from_millis(50);
pub(crate) const STOP_WINDOW: Duration = Duration::from_secs(2);

/// The end of a stop round that starts now.
pub(crate) fn stop_deadline() -> Instant {
    Instant::now() + STOP_WINDOW
}

impl LinkManager {
    /// Send StopAll until the node answers OK or `deadline` passes. The first
    /// attempt starts at once. It runs in a task of its own, so a caller that
    /// goes away can't end it. A caller holding a client passes it: it's used
    /// when the manager no longer offers one.
    pub(crate) async fn stop_until_acknowledged(
        self: Arc<Self>,
        client: Option<MountClient>,
        deadline: Instant,
    ) -> bool {
        tokio::spawn(async move {
            let mut client = client;
            loop {
                if let Some(current) = self.mount() {
                    client = Some(current);
                }
                if let Some(stop) = &mut client {
                    if matches!(time::timeout(STOP_ATTEMPT, stop.stop_all()).await, Ok(Ok(()))) {
                        return true;
                    }
                }
                if Instant::now() + STOP_PAUSE >= deadline {
                    return false;
                }
                time::sleep(STOP_PAUSE).await;
            }
        })
        .await
        .unwrap_or(false)
    }
}
```

An attempt that has been timed out is dropped, and tonic resets its stream. That is
harmless once US3 has landed, and it's why US3 exists. A held attempt starts at
(k − 1) × 0.25 s, so a round with every attempt timing out is 8 attempts, ending at
1.95 s. A deadline already passed still makes exactly one attempt.

`MountClient::stop_all` keeps its signature, since `tests/mount_calls.rs` and
`tests/mcp_session_tools.rs` call it.

### `api::stop` (`crates/finder/src/api.rs`)

```rust
pub(crate) async fn stop(state: &AppState) -> serde_json::Value {
    // Neither side waits for the other: a GoTo is cancelled and the first attempt
    // sent even while tracking is still finishing.
    let (_, acknowledged) = tokio::join!(state.session.stop_tracking(), async {
        match (&state.goto, &state.link) {
            (Some(goto), _) => goto.stop().await,
            (None, Some(link)) => link.clone().stop_until_acknowledged(None, stop_deadline()).await,
            (None, None) => true,
        }
    });
    if acknowledged {
        serde_json::json!({"stopped": true})
    } else {
        serde_json::json!({"stopped": true, "acknowledged": false})
    }
}
```

`stop_tracking` is polled first, so its synchronous part (track generation and arming)
still comes first. `join!` is what fixes the order: at 032b34d `goto.stop()` ran only after
`stop_tracking` had awaited the tracker and `track_gate`.

`stop_mount` and `set_track` are unchanged. Stop has no extractor and takes no lock on
`state.motion`.

### MCP (`crates/finder/src/mcp/door.rs`, `tools.rs`)

`StopsCarriedOut` becomes `StopsCarriedOut(pub(crate) serde_json::Value)`, holding what
`api::stop` answered, and `stop_result` uses that value in place of today's literal
`json!({"stopped":true})`. Everything before and after it in the door is unchanged. The
two tools' descriptions gain this sentence: `A reply with acknowledged: false means the
scope node did not answer and the mount may still be moving; call it again.`

### GoTo (`crates/finder/src/goto.rs`)

US1 changes `GoTo::stop` to return the round's result:

```rust
pub async fn stop(&self) -> bool {
    self.cancel.send_modify(|n| *n = n.wrapping_add(1));
    { /* unchanged: mark a running GoTo halted */ }
    self.link.clone().stop_until_acknowledged(None, stop_deadline()).await
}
```

US2 changes `run`. The ending is published, then the round runs, then `busy` clears:

```rust
if let Err((halted, reason)) = result {
    let progress = self.progress().unwrap();
    self.update(generation, if halted { "halted" } else { "failed" },
        progress.correction, progress.error_arcsec, Some(reason));
    self.link.clone().stop_until_acknowledged(Some(client), stop_deadline()).await;
}
```

US4 changes `motion` and `run`:

```rust
const WATCHDOG_HALT: &str = "halted by the scope node's watchdog";
// before the first leg:
let watchdog_stops = client.axis_state(AXES[0]).await
    .map_err(|_| "mount did not answer")?.watchdog_stops;
// in the wait loop, after reading az and alt, before testing `moving`:
if az.watchdog_stops != watchdog_stops || alt.watchdog_stops != watchdog_stops {
    return Err(WATCHDOG_HALT.into());
}
```

In `run`, `motion`'s error maps to `(reason == WATCHDOG_HALT, reason)`, so that this halt
is `halted` and every other motion error stays `failed`.

### Direct moves (`crates/finder/src/mcp/guard.rs`)

Both stops call `state.link()`'s `stop_until_acknowledged(Some(client), stop_deadline())`.
The final one sets `axis_move` as:

```rust
let (name, reason) = match (result, acknowledged) {
    (Err(reason), _) => ("halted", Some(reason)),
    (Ok(()), true) => ("done", None),
    (Ok(()), false) => ("halted", Some("the mount did not confirm the stop".to_owned())),
};
```

`Progress` doesn't change shape. `begin` has `state.link()` in hand from `goto`'s
existence; `ok_or_else` with today's `no mount is connected` covers the impossible case.

### Tracking (`crates/finder/src/tracking.rs`, `session.rs`)

`Tracker::stop(reason)` keeps its signature, because `tests/tracking_sim.rs`,
`tracking_night.rs`, `backlash_sim.rs` and `keyhole_sim.rs` call it. It becomes
`self.stop_by(reason, stop_deadline())`. A new `pub(crate) async fn stop_by(&self, reason,
deadline)` holds today's body, and its no-task branch calls the retrying stop with `None`
and that deadline. `Shared::run`'s exit calls it with `Some(client)` and a fresh deadline
after `self.end(reason)`. `STOP_TIMEOUT` goes. `Session::stop_tracking` computes one
`stop_deadline()` at entry and passes it to both of its `stop_by` calls, so the second
can't start a fresh window. The keyhole's `client.stop_all()` stays: it fails the run on
an error and its own end path stops the axes.

### The node (`crates/scope-node/src/`)

```rust
// mount/mod.rs, in `trait Mount`:
/// Cancel motion on both axes and say whether the mount confirmed it. Adapters
/// with nothing to report keep this default.
fn stop_confirmed(&self) -> MountFuture<'_, bool> {
    Box::pin(async move {
        self.stop_all().await;
        true
    })
}

// mount/skywatcher/protocol.rs, in `impl Mount for SkyWatcher<S>`:
fn stop_confirmed(&self) -> MountFuture<'_, bool> {
    Box::pin(async move {
        match self.protocol.lock().await.halt().await {
            Ok(()) => true,
            Err(error) => {
                eprintln!("mount stop failed: {error}");
                false
            }
        }
    })
}

// mount/queue.rs: `stop_all` returns the adapter's answer.
pub async fn stop_all(&self) -> bool { /* unchanged, then */
    let confirmed = self.shared.mount.stop_confirmed().await;
    self.shared.state.lock().unwrap().stopping = false;
    confirmed
}

// service.rs, ScopeNodeStop::stop_all:
if let Some(mount) = self.mount_queue() {
    // A task of its own: a console that gives up on this call must not cancel a
    // stop that is under way, half way through the axes or with `stopping` set.
    let mount = mount.clone();
    let confirmed = tokio::spawn(async move { mount.stop_all().await })
        .await
        .unwrap_or(false);
    if !confirmed {
        return Err(Status::unavailable("the mount did not confirm the stop"));
    }
}
Ok(Response::new(proto::StopAllReply {}))
```

`Watchdog` discards the bool. Its `queue.stop_all().await;` compiles unchanged.

### The watchdog count

```proto
message AxisState {
  int32 position = 1;
  double rate_deg_per_s = 2;
  bool moving = 3;
  // Times the node's watchdog has stopped the mount since the node started.
  uint64 watchdog_stops = 4;
}
```

`Watchdog`'s `State` gains `stops: u64`, incremented where `lost` is set, before
`queue.stop_all()` is commanded. `pub fn stops(&self) -> u64` reads it.
`ScopeNodeMount::axis_state` reads it after reading the axis:
`watchdog_stops: self.runtime.watchdog.as_ref().map_or(0, Watchdog::stops)`. So a reply
that shows axes the watchdog stopped always shows the new count.

### The page (`crates/finder/web/src/`)

`api.ts` adds:

```ts
export interface StopReply {
  stopped: boolean;
  acknowledged?: boolean;
}
/** Stop deliberately carries no bearer token. */
export async function postStop(): Promise<StopReply> {
  return json(await fetch("/api/stop", { method: "POST" }));
}
```

`postMotion` is unchanged. `StopButton.tsx`:
- gains a `waiting` state, set after a press has finished, holding the `mount` object
  current when the reply arrived (read through a ref, so that the closure sees the latest
  prop) and whether the reply was confirmed;
- a request failure and `acknowledged === false` each set `error` and `waiting`;
- `if (!waiting && !mountMayMove(...)) return null`;
- an effect clears `waiting` and `error` when `mount` is a different object from the one
  held, with `connected === true` and `moving === false`;
- when `waiting` holds a confirmed reply, `mount?.connected !== true` and there's no
  `error`, the note of FR-015 is a plain `<p>` in the same absolutely placed position and
  classes as the alert;
- a new press clears `error` and leaves `waiting` in place, so the button can't vanish
  while the second request is pending.

`mountMayMove` and every existing prop are unchanged. `TopBar` and `App` are unchanged.

## Shapes that existing tests pin

These are not changed by any story:
- **The reply.** `{"stopped":true}` when acknowledged:
  `crates/finder/tests/mcp_motion.rs:444`, `crates/finder/tests/mcp_motion.rs:489`,
  `crates/finder/tests/mcp_motion.rs:493`, `crates/finder/tests/mcp_tracking.rs:309` and
  `crates/finder/tests/mcp_axis_moves.rs:437` compare it exactly. The first does so on a
  console with no scope link.
- **One stop on a healthy link.** `crates/finder/tests/mcp_motion.rs:375` wants exactly
  one `StopAll` for a whole request with no GoTo and no tracker. The first OK ends the round.
- **Stop's speed.** `crates/finder/tests/mcp_motion.rs:243` and
  `crates/finder/tests/mcp_motion.rs:246` want a `StopAll` and the answer within 0.1 s on
  a healthy link.
- **Proto literals.** `HeartbeatReply` has struct literals in
  `crates/scope-link/tests/link.rs:83` and `crates/finder/tests/aim_cli.rs:51`, so it
  gains no field. `proto::AxisState` is built only in `crates/scope-node/src/service.rs`.
- **The `Mount` port.** `stop_all` is implemented in `tests/tracking_sim.rs`,
  `tests/tracking_loop.rs`, `tests/keyhole_sim.rs`, `tests/backlash_sim.rs`,
  `tests/goto_api.rs`, `tests/mcp/support.rs` and `crates/simulator/tests/support/mod.rs`,
  and keeps its `MountFuture<'_, ()>` type. The new method is provided.
- **Rust signatures:** `MountClient::stop_all`, `Tracker::stop`.
- **`Progress` and `axis_move`** gain no field. A new halt reason adds a text; every
  existing one stays byte-for-byte.
- **The page.** `tests` find Stop with `getByRole("button", { name: "Stop" })`, expect
  `fetch` with `("/api/stop", { method: "POST" })`, want `null` for an idle or absent
  mount (`StopButton.test.tsx`), and `mountMayMove`'s exported shape
  (`McpNote.test.tsx`). `crates/finder/web/src/components/GoToControl.test.tsx:103` uses an
  unscoped `getByRole("status")`, so FR-015's note has no `status` role.

## Decisions and the alternatives rejected

- **One retrying function, in `LinkManager`.** It already owns the current client and is
  held in an `Arc` everywhere a stop is sent. *Rejected:* a retry at each site, which
  would drift, and a new module, which would have nothing else in it.
- **A task of its own.** The direct-move guard's rule is that a client's lifetime never
  owns the stop timer, and axum drops a handler when the client goes away. *Rejected:* a
  retry inside the handler.
- **Rounds aren't shared between callers.** Two presses make two rounds, each bounded.
  *Rejected:* one global round that callers join. A joiner would have to ignore any
  attempt sent before its own cancellation, which is a generation scheme, for a case
  that's rare and harmless.
- **The reply waits for the round.** *Rejected:* answering after the first attempt and
  reporting through the session, which would add a session field, a poll, and a reply
  that can't say whether the mount answered.
- **`acknowledged` only when false** (spec § Clarifications, approval 1). *Rejected for
  now:* always sending it, which needs five assertions edited. If the owner prefers it,
  spec.md's approval 1 and FR-004 change, and those five assertions join the approvals.
- **`AxisState`, not `HeartbeatReply`** (spec § Clarifications). A heartbeat-carried count
  also leaves a window between the node's clearing `lost` and the console's reading the
  reply, in which a next leg could go out. *Rejected as well:* a new RPC, which needs
  every test fake of the service to grow, and a position check at arrival, because
  arrival accuracy isn't measured on real hardware yet.
- **Two reasons stay apart.** A GoTo halted by the count says `halted by the scope node's
  watchdog`, not `halted by a lost link`, so that the night's log can tell them apart.
- **The node runs each stop in a spawned task.** *Rejected:* giving the console's attempt
  a longer timeout. The serial layer can take up to 6 s on a silent board.
- **`StopAll` errors with `UNAVAILABLE`.** *Rejected:* a new `StopAllReply` field, which
  would need its own handling on both sides, when the RPC already has an error channel
  and the retry already treats an error as unanswered.
- **The page's latch is local state.** *Rejected:* a session field. See spec § Assumptions.

## User Story 1 — The retry and the reply

`Rig` in `tests/stop/support.rs` mirrors `tests/mcp/support.rs`'s simulator rig, with
`Routes` built as `tests/tracking_sim.rs` builds them, so the stop service is the gate.
Its node clock adds an atomic offset in milliseconds to `node_now()`, used by US4.
`StopGate::set(Hold(n) | HoldAll | Pass | Refuse)` and `StopGate::log()` are all a test
needs. A held call is `std::future::pending()`, as `StopLink` does. The recording mount
has a `hold_state` flag for US1-S11: `check_request` waits on `axis_state` under a 1 s
timeout, inside `start_motion`'s lock.

US1-S8 needs a retained tracker. Turn Track on, post `/api/stop`, and start a GoTo to a
different target: `Session::tracker()` still returns the ended tracker.

## User Story 2 — The ends of motions

The order in `GoTo::run` matters for `crates/finder/tests/goto_sim.rs:206`, which reads
`halted` after 3 s of silence. The round mustn't hide that state for 2 s.

US2-S6 adds a `hold_rate` flag to the recording mount in `tests/stop/support.rs`.

## User Story 3 — The node's half

`stop_finishes.rs` builds its own node with a `Mount` wrapper around `SimMount` whose
stop sleeps 1 s and counts completions. `support::Adapter` can't be edited. The probe that
found this (spec § Clarifications, finding 5) took about 40 lines.

## User Story 4 — The watchdog count

Advancing only the node's clock fires the real `Watchdog` with no test hook in it. The
jump also shifts exposure times against the console's clock map for 8 s, so US4-S2 and S4
assert only the halt. US4-S3 uses a real silence instead, because it has to finish a GoTo.

## User Story 5 — The page

Stop stays rendered by `TopBar` across routes, so `waiting` survives moving between the
console and Setup. Page reloads lose it (spec § Assumptions).

## Verification (every story, before proposing)

- **The factory gates** (`ergane.yaml` at 032b34d):
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
- **CI in the merge queue** runs every required `ci.yml` job, including `cargo machete`,
  `cargo deny` and the build on Rust 1.96.

## Prerequisites before this spec is flipped to `ready`

1. **The earlier specs have landed:** `014-goto`, `015-tracking` and `016-mcp`. Met:
   016's last story landed at bcf3765, before 032b34d.
2. **`main` is still as read.** Every citation here is at 032b34d. If `main` has moved
   in these files, re-read them and re-run the validator.
3. **The owner approvals are given** (spec § Owner approvals). Flipping to `ready`
   records them.
4. **No operator-staged files are needed.**

## Plan traps

Lessons from building 012 to 016 on this factory, each with its mechanism and the check
that keeps it from recurring. They bind every story.

1. **Gate evidence or nothing.** The judge credits only gate results and the story's
   diff. *Check:* every Then clause names a committed test or a command a gate runs.
2. **A named command is read literally.** *Check:* commands in scenarios are copied from
   § Verification; the `--test` lines are development commands only.
3. **One evidence budget.** *Check:* a story keeps its diff under 64 KiB, measured with
   `git diff <base> -- . ':(exclude)crates/finder/web/dist' | wc -c`.
4. **The sandbox has only the gates' tools.** *Check:* no task runs a tool outside them.
5. **A spec fixed mid-epic needs a relaunch.** The judge grades the spec as dispatched.
   *Check:* no builder edits `specs/017-console-stop-confirm/`.
6. **A struct literal pins its fields.** *Check:* no field is added to `HeartbeatReply`
   or `Progress` (§ Shapes that existing tests pin).
7. **A negative assertion passes when nothing happens.** A "still moving" or "no go-to
   after" passes on a mount that was never moving. *Check:* each shows its other side
   in the same test, as spec.md's shared setup requires.
8. **The gate counts every caller.** During a GoTo or Track, more than one round runs at
   once, so exact attempt counts are only meaningful with nothing else stopping the
   mount. *Check:* the exact scenarios start the axis on `SimMount` directly.
9. **Silence isn't a held Stop.** `NodeControls::set_unresponsive` delays heartbeats and
   frames, but the node still answers `StopAll`. *Check:* a scenario that needs a stop
   held uses the gate.
10. **JSON key order isn't part of the contract.** `serde_json` sorts keys unless
    `preserve_order` is on. *Check:* the two-key reply is compared as parsed JSON, and
    only the one-key body is compared exactly as text.
11. **A cancelled handler keeps no promises.** tonic drops a handler when its caller goes
    away, which is what left `stopping` set. *Check:* anything on the node that has to
    finish runs in its own task, and US3-S1 proves it.
12. **The last `moving` read can be stale.** The page reads a session that is up to a
    poll old. *Check:* only a `mount` object received after the reply clears the message
    (US5-S5).
13. **Each worktree builds on its own.** *Check:* no story sets `CARGO_TARGET_DIR`.

## Risks

| Risk | Mitigation |
|---|---|
| A retry lands after the observer has started a new GoTo. | Rare and in Stop's direction. Noted in spec § Edge Cases. |
| A Stop on a dead board makes the node run several slow halts in turn. | Attempts are bounded at about 8, and moves are refused while they run. Nothing moves in the meantime. |
| The new `scope-node` isn't installed on the Pi. | The console works with an old node (spec § Edge Cases). Listed as outside the factory. |
| A test that used to answer in 200 ms now waits 2 s of virtual time. | `start_paused` makes it instant. The two real-time tests that post Stop have a healthy link or no scope link. |

## Refinement Record

Drafted on 2026-11-08 by a plate-solver spec session against `main` at 032b34d, after two
throwaway probes (spec § Clarifications). This plan's code was then applied to a throwaway
copy of 032b34d, with no existing test edited: `cargo test -p finder -p scope-node -p
simulator -p scope-link --locked --all-targets` passed 454 tests, and the page's Vitest
suite and typecheck passed. Playwright, `lint` and the other workspace crates were not
run. In that copy a stop gate gave the attempt times US1-S2, US1-S3 and US1-S4 name, and
US2-S4's and US4-S2's outcomes. Writes come from the task text. Shared writes
name the merge edge that orders them, and the 014, 015 and 016 stories that last wrote
the same file on `main`. Nothing in this record is an owner approval: those are in
spec.md, and flipping to `ready` gives them.

```yaml
version: 1
reviewed_revision: 032b34db4fab63db8eaf40ff32c088cc15f65b7f
stories:
  US1:
    tests:
      - path: crates/finder/tests/stop_retry.rs
        status: new
        disposition: add
        assertion: "with the gate passing one attempt, a body of exactly {\"stopped\":true}, one StopAll, at rest; holding two, attempts at 0, 0.25 and 0.5 s and the same body; holding all, 8 attempts, {\"stopped\":true,\"acknowledged\":false} at 1.95 s and the axis still moving; refusing all, no two attempts within 0.045 s and at most 41"
        decision: "spec.md:US1-S1"
      - path: crates/finder/tests/stop_retry.rs
        status: new
        disposition: add
        assertion: "dropping the request after 0.3 s leaves the round running, so a later attempt reaches the node and the axis stops; with a dropped link the body says acknowledged false within 2.05 s; with no scope link the body is exactly {\"stopped\":true} and no time passes; with a retained tracker the GoTo reads halted within 0.1 s while the gate holds"
        decision: "spec.md:US1-S5"
      - path: crates/finder/tests/stop_retry.rs
        status: new
        disposition: add
        assertion: "MCP stop_all's structuredContent, content and call log carry acknowledged false under a held gate; both tools' descriptions mention acknowledged; a foreign-Origin batch of two stops answers 403 after one round of 8 attempts"
        decision: "spec.md:US1-S9"
      - path: crates/finder/tests/stop_retry.rs
        status: new
        disposition: add
        assertion: "with a POST /api/goto waiting on a held mount state call and so holding the motion lock, POST /api/stop answers within 0.1 s and before the goto answers"
        decision: "spec.md:US1-S11"
      - path: crates/finder/tests/mcp_motion.rs
        status: existing
        disposition: preserve
        assertion: "the acknowledged reply stays exactly {\"stopped\":true}, a stop reaches both axes within 0.1 s through every door case, and a whole request sends exactly one StopAll on a healthy link"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/mcp_axis_moves.rs
        status: existing
        disposition: preserve
        assertion: "stop_all's structuredContent stays exactly {\"stopped\":true} on a healthy link"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/mcp_tracking.rs
        status: existing
        disposition: preserve
        assertion: "stop's structuredContent stays exactly {\"stopped\":true} on a healthy link and ends Track"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/mount_calls.rs
        status: existing
        disposition: preserve
        assertion: "MountClient::stop_all keeps its signature and result"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "POST /api/stop always answers 200, takes no body and has no origin restriction (crates/finder/README.md)"
      - "the door carries out stops before any check, and an MCP batch still runs its stops and nothing else (016 FR-012)"
      - "Stop takes no motion lock (spec.md FR-006)"
    checks:
      development:
        - "cargo test -p finder --locked --test stop_retry"
        - "cargo test -p finder --locked --test mcp_motion --test mcp_axis_moves --test mcp_tracking"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs: msrv, deny and machete"
    writes:
      - crates/finder/README.md
      - crates/finder/src/api.rs
      - crates/finder/src/goto.rs
      - crates/finder/src/lib.rs
      - crates/finder/src/link.rs
      - crates/finder/src/mcp/door.rs
      - crates/finder/src/mcp/tools.rs
      - crates/finder/tests/stop/support.rs
      - crates/finder/tests/stop_retry.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/goto.rs
        other_spec: 017-console-stop-confirm
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and changes GoTo::run"
      - path: crates/finder/src/goto.rs
        other_spec: 017-console-stop-confirm
        other_story: US4
        disposition: merge_order
        reason: "US4 depends_on_merged US1, and changes GoTo::motion"
      - path: crates/finder/README.md
        other_spec: 017-console-stop-confirm
        other_story: US4
        disposition: merge_order
        reason: "US4 depends_on_merged US1, and adds the watchdog's halt reason"
      - path: crates/finder/tests/stop/support.rs
        other_spec: 017-console-stop-confirm
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and adds hold_rate to the recording mount"
      - path: crates/finder/README.md
        other_spec: 016-mcp
        other_story: US7
        disposition: merge_order
        reason: "017 depends_on_landed 016, whose US7 last changed the README"
      - path: crates/finder/src/api.rs
        other_spec: 016-mcp
        other_story: US6
        disposition: merge_order
        reason: "017 depends_on_landed 016, whose US6 last changed api.rs"
      - path: crates/finder/src/goto.rs
        other_spec: 016-mcp
        other_story: US5
        disposition: merge_order
        reason: "017 depends_on_landed 016, whose US5 last changed goto.rs"
      - path: crates/finder/src/lib.rs
        other_spec: 016-mcp
        other_story: US1
        disposition: merge_order
        reason: "017 depends_on_landed 016, whose US1 last changed lib.rs"
      - path: crates/finder/src/link.rs
        other_spec: 014-goto
        other_story: US2
        disposition: merge_order
        reason: "017 depends_on_landed 014, whose US2 last changed link.rs"
      - path: crates/finder/src/mcp/door.rs
        other_spec: 016-mcp
        other_story: US4
        disposition: merge_order
        reason: "017 depends_on_landed 016, whose US4 last changed door.rs"
      - path: crates/finder/src/mcp/tools.rs
        other_spec: 016-mcp
        other_story: US6
        disposition: merge_order
        reason: "017 depends_on_landed 016, whose US6 last changed tools.rs"
    open_questions: []
  US2:
    tests:
      - path: crates/finder/tests/stop_motion_ends.rs
        status: new
        disposition: add
        assertion: "a GoTo halted by a lost link reads halted before the held round's third attempt, the third attempt reaches the node, and POST /api/goto is refused with a GoTo is already running while the round runs"
        decision: "spec.md:US2-S1"
      - path: crates/finder/tests/stop_motion_ends.rs
        status: new
        disposition: add
        assertion: "a rate move's final stop held twice ends done after the third attempt with one StopAll; held throughout, axis_move is halted with the mount did not confirm the stop, never done, and the axis is still moving; a Stop during the move keeps the observer stopped it; a rate call that never completes answers mount did not answer after two attempts"
        decision: "spec.md:US2-S3"
      - path: crates/finder/tests/stop_motion_ends.rs
        status: new
        disposition: add
        assertion: "with Track running, a Stop held twice answers {\"stopped\":true} after at least three attempts and ends Track as stopped; held throughout it answers acknowledged false within 2.5 s; a Tracker never started repeats at 0, 0.25 and 0.5 s"
        decision: "spec.md:US2-S7"
      - path: crates/finder/tests/goto_sim.rs
        status: existing
        disposition: preserve
        assertion: "silent heartbeats stop at one second and report a lost link, with halted readable as before"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/night_sim.rs
        status: existing
        disposition: preserve
        assertion: "the night still halts on Stop and on a lost link, with no restart"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/tracking_sim.rs
        status: existing
        disposition: preserve
        assertion: "Tracker::stop keeps its signature, the first StopAll lands within 0.5 s, and a silent StopAll still leaves the node's watchdog as the first receipt"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/mcp_axis_moves.rs
        status: existing
        disposition: preserve
        assertion: "a direct move still ends done on a healthy link, and the page's Stop still reports the observer stopped it"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "a GoTo refuses a new start while its last stop is outstanding, as busy already does"
      - "every existing halt and refusal text stays byte-for-byte (spec.md FR-013 adds one, FR-008 adds one)"
      - "the keyhole's own stop is unchanged (plan.md Tracking)"
    checks:
      development:
        - "cargo test -p finder --locked --test stop_motion_ends"
        - "cargo test -p finder --locked --test goto_sim --test tracking_sim --test mcp_axis_moves"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs: msrv, deny and machete"
    writes:
      - crates/finder/src/goto.rs
      - crates/finder/src/mcp/guard.rs
      - crates/finder/src/session.rs
      - crates/finder/src/tracking.rs
      - crates/finder/tests/stop/support.rs
      - crates/finder/tests/stop_motion_ends.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/goto.rs
        other_spec: 017-console-stop-confirm
        other_story: US4
        disposition: merge_order
        reason: "US4 depends_on_merged US2, and changes GoTo::motion"
      - path: crates/finder/src/mcp/guard.rs
        other_spec: 016-mcp
        other_story: US5
        disposition: merge_order
        reason: "017 depends_on_landed 016, whose US5 created guard.rs"
      - path: crates/finder/src/session.rs
        other_spec: 015-tracking
        other_story: US7
        disposition: merge_order
        reason: "017 depends_on_landed 015, whose US7 is the last story to change session.rs; main's later 2beb764 is an operator commit"
      - path: crates/finder/src/tracking.rs
        other_spec: 015-tracking
        other_story: US9
        disposition: merge_order
        reason: "017 depends_on_landed 015, whose US9 last changed tracking.rs"
    open_questions: []
  US3:
    tests:
      - path: crates/simulator/tests/stop_finishes.rs
        status: new
        disposition: add
        assertion: "a StopAll dropped at 200 ms still finishes on a 1 s mount, both axes rest and the next rate_axis is accepted; three overlapping StopAlls all finish; a mount reporting false answers Unavailable with the mount did not confirm the stop; a default mount and a node with no mount answer OK"
        decision: "spec.md:US3-S1"
      - path: crates/scope-node/tests/skywatcher_stop.rs
        status: new
        disposition: add
        assertion: "stop_confirmed is true with K1 and K2 on the wire; true after L1 follows K2 when axis 1's K is silent; false when axis 2's K and L replies are dropped, with K1 still sent"
        decision: "spec.md:US3-S3"
      - path: crates/simulator/tests/stop_and_watchdog.rs
        status: existing
        disposition: preserve
        assertion: "Stop cancels in-flight and queued moves within 100 ms, and the watchdog stops both axes once at one second and never resumes"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/scope-node/tests/skywatcher.rs
        status: existing
        disposition: preserve
        assertion: "both stops precede the emergency fallback, and stop_all's wire order is unchanged"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "Mount::stop_all keeps its type; every test fake compiles unchanged through the provided method (plan.md Shapes that existing tests pin)"
      - "the StopAll service still needs no token"
    checks:
      development:
        - "cargo test -p simulator --locked --test stop_finishes --test stop_and_watchdog"
        - "cargo test -p scope-node --locked --test skywatcher_stop --test skywatcher"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs: msrv, deny and machete"
    writes:
      - crates/scope-node/src/mount/mod.rs
      - crates/scope-node/src/mount/queue.rs
      - crates/scope-node/src/mount/skywatcher/protocol.rs
      - crates/scope-node/src/service.rs
      - crates/scope-node/tests/skywatcher_stop.rs
      - crates/simulator/tests/stop_finishes.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/scope-node/src/service.rs
        other_spec: 017-console-stop-confirm
        other_story: US4
        disposition: merge_order
        reason: "US4 depends_on_merged US3, and adds the count to axis_state"
      - path: crates/scope-node/src/service.rs
        other_spec: 014-goto
        other_story: US2
        disposition: merge_order
        reason: "017 depends_on_landed 014, whose US2 last changed service.rs"
      - path: crates/scope-node/src/mount/queue.rs
        other_spec: 014-goto
        other_story: US2
        disposition: merge_order
        reason: "017 depends_on_landed 014, whose US2 last changed queue.rs"
      - path: crates/scope-node/src/mount/mod.rs
        other_spec: 014-goto
        other_story: US3
        disposition: merge_order
        reason: "017 depends_on_landed 014, whose US3 last changed mod.rs"
      - path: crates/scope-node/src/mount/skywatcher/protocol.rs
        other_spec: 014-goto
        other_story: US3
        disposition: merge_order
        reason: "017 depends_on_landed 014, whose US3 last changed protocol.rs"
    open_questions: []
  US4:
    tests:
      - path: crates/simulator/tests/watchdog_report.rs
        status: new
        disposition: add
        assertion: "AxisState.watchdog_stops reads 0, then 1 after a 1.1 s silence, then 2 after a second, and an explicit StopAll leaves it unchanged"
        decision: "spec.md:US4-S1"
      - path: crates/finder/tests/watchdog_goto.rs
        status: new
        disposition: add
        assertion: "advancing only the node's clock by 1.2 s mid-slew leaves the link connected, stops the axes, and halts the GoTo with halted by the scope node's watchdog, with no go-to after the StopAll; a firing before a GoTo doesn't halt it and it ends done within 1 arcmin; armed Track ends with the same reason"
        decision: "spec.md:US4-S2"
      - path: crates/scope-link/tests/link.rs
        status: existing
        disposition: preserve
        assertion: "HeartbeatReply is still built with three fields; the scope link's existing shapes are unchanged"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/goto_sim.rs
        status: existing
        disposition: preserve
        assertion: "every GoTo with no firing completes and corrects as before, and a lost link still reports a lost link"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "a GoTo with no firing behaves as before: the count stays equal to its baseline"
      - "an older node sends 0 and an older console ignores the field (spec.md Edge Cases)"
    checks:
      development:
        - "cargo test -p simulator --locked --test watchdog_report"
        - "cargo test -p finder --locked --test watchdog_goto --test goto_sim"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs: msrv, deny and machete"
    writes:
      - crates/finder/README.md
      - crates/finder/src/goto.rs
      - crates/finder/tests/watchdog_goto.rs
      - crates/scope-link/proto/scope_node/v1/scope_node.proto
      - crates/scope-node/src/mount/watchdog.rs
      - crates/scope-node/src/service.rs
      - crates/simulator/tests/watchdog_report.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/scope-link/proto/scope_node/v1/scope_node.proto
        other_spec: 014-goto
        other_story: US2
        disposition: merge_order
        reason: "017 depends_on_landed 014, whose US2 last changed the scope link proto"
      - path: crates/scope-node/src/mount/watchdog.rs
        other_spec: 014-goto
        other_story: US2
        disposition: merge_order
        reason: "017 depends_on_landed 014, whose US2 created watchdog.rs"
    open_questions: []
  US5:
    tests:
      - path: crates/finder/web/src/components/StopConfirm.test.tsx
        status: new
        disposition: add
        assertion: "acknowledged false shows FR-014's alert and keeps Stop with mountMayMove false; a failed request keeps its error and Stop; a confirmed reply with no reading shows FR-015's note with no alert or status role; a confirmed reply with connected and moving shows nothing extra; a new mount object at rest clears them, the same object doesn't, and a new one with connected false doesn't; a second press while pending fetches again with Stop enabled"
        decision: "spec.md:US5-S1"
      - path: crates/finder/web/src/components/StopButton.test.tsx
        status: existing
        disposition: preserve
        assertion: "Stop stays in the top bar, stays enabled through pending and failed requests, posts without a token or body, and renders nothing for an idle or absent mount"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/src/components/StopTrack.test.tsx
        status: existing
        disposition: preserve
        assertion: "Stop shows while Track is armed, tracking or at the keyhole, and hides for an idle mount"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/src/components/McpNote.test.tsx
        status: existing
        disposition: preserve
        assertion: "mountMayMove keeps its shape and motion keeps Stop on screen"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "the page adds no npm package and no second Stop (DESIGN.md)"
      - "red-only night mode: the note and the alert use the existing ink tokens"
    checks:
      development:
        - "npm test"
        - "npm run typecheck"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs: finder-page"
    writes:
      - .impeccable/design.json
      - DESIGN.md
      - crates/finder/web/dist
      - crates/finder/web/src/api.ts
      - crates/finder/web/src/components/StopButton.tsx
      - crates/finder/web/src/components/StopConfirm.test.tsx
    no_write_reason: ""
    shared_writes:
      - path: .impeccable/design.json
        other_spec: 016-mcp
        other_story: US7
        disposition: merge_order
        reason: "017 depends_on_landed 016, whose US7 last recorded its views"
      - path: DESIGN.md
        other_spec: 016-mcp
        other_story: US7
        disposition: merge_order
        reason: "017 depends_on_landed 016, whose US7 last recorded its views"
      - path: crates/finder/web/dist
        other_spec: 016-mcp
        other_story: US7
        disposition: merge_order
        reason: "017 depends_on_landed 016, whose US7 last rebuilt the bundle"
      - path: crates/finder/web/src/api.ts
        other_spec: 016-mcp
        other_story: US7
        disposition: merge_order
        reason: "017 depends_on_landed 016, whose US7 last changed api.ts"
      - path: crates/finder/web/src/components/StopButton.tsx
        other_spec: 016-mcp
        other_story: US7
        disposition: merge_order
        reason: "017 depends_on_landed 016, whose US7 last changed Stop's rule"
    open_questions: []
```
