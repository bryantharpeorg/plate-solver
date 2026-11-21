# Implementation Plan: The Node Stops the Mount Before It Exits

**Branch**: `044-node-stops-mount-before-exit` | **Date**: 2026-11-21 | **Spec**: [spec.md](spec.md)

This plan records every design decision for the feature. A story that would change one
escalates rather than re-deciding it.

## Summary

The scope node learns to stop the mount when it is told to exit. `serve` takes SIGTERM
and SIGINT before the listener binds and serves with tonic's `serve_with_shutdown`. On
the first signal it ends the serving and calls `NodeService::shut_down`, which closes
the mount queue for good and runs the queue's own stop, the one the watchdog and
`StopAll` use, for at most `STOP_BOUND`, 8 s. The node then writes one line saying how
the stop ended and exits, 0 when the mount confirmed or there is none and 1 otherwise.
The stop goes through `MountQueue` and the `Mount` trait only. The node's README gives
the unit `TimeoutStopSec=10`, and the voice safety table's row for a dying node is split
from a clean stop.

US2 adds the one test that runs the shipped binary with a moving mount, the emulator
behind a pseudo-terminal, and no code.

## Technical Context

**Language/Version**: Rust, MSRV 1.96, stable in CI.

**Primary Dependencies**: tokio's `signal` feature (US1) and rustix's `pty` feature in
`[dev-dependencies]` (US2), both features of crates the node already uses. `Cargo.lock`
does not change with either (run, § What was run and what was read).

**Storage**: none.

**Testing**:
- US1: `crates/scope-node/tests/stop_before_exit.rs` (new), in process under
  `#[tokio::test(start_paused = true)]`, over the emulator behind a `MountLink` and over
  a stand-in mount; and `crates/scope-node/tests/exit_on_signal.rs` (new), which runs the
  built binary under `#[tokio::test]` on the wall clock and sends signals with the
  `kill` program.
- US2: `crates/scope-node/tests/exit_stops_the_mount.rs` (new), the built binary with the
  emulator behind a pseudo-terminal, under `#[tokio::test]` on the wall clock.
- No Vitest or Playwright test: the page is unchanged.

**Target Platform**: `scope-node` on the Pi, under systemd.

**Constraints**:
- `MountQueue::stop_all`, `QueueError`, the `Mount` trait, `NodeService`'s constructors
  and every RPC's answer keep their shapes; existing tests call them (§ Shapes that
  existing tests pin).
- `RUSTFLAGS="-D warnings"`, `--all-features` clippy, and rustdoc with warnings denied.
- No test moves real hardware. No test sends a signal to a process it did not start.

**Scale/Scope**: two stories; one new library module, three changed source files, the
README, one doc row, three new test files.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Nothing touches the solver, star detection or `crates/math-core`. The behaviour is grounded in the node's own stop (017's FR-010 and FR-011, 024's FR-018), in systemd's documented stop sequence (man `systemd.service(5)`, `systemd.kill(5)`, `systemd.unit(5)`), and in what the probe of 2026-11-21 ran; what rests on reading is listed in § What was run and what was read, and none is invented beyond it. |
| II. Never weaken a gate | No existing test, golden, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS` or `ergane.yaml` is edited. The new tests go in new files; no existing file under `tests/` is touched. § Shapes that existing tests pin names every existing test file beside the change. |
| III. Tests first, independent expectations | The oracles are the emulator's own record of what the board was sent and answered, the board's own word on its axes, a stand-in's record, the process's exit status and standard error, and the scenarios' own numbers and words. No test reads `STOP_BOUND` or `Shutdown`'s words from the code to get an expected value. In-process time is paused Tokio time supplied to the board and the node; the binary's times are wall-clock bounds with stated tolerances. The binary's tests fail on `main` (§ What was run and what was read). |
| IV. Critical paths | Two dependency changes, both in `crates/scope-node/Cargo.toml` and neither in `Cargo.lock`: tokio's `signal` feature (spec owner approval 1) and rustix's `pty` feature for tests (owner approval 4). No `unsafe`: the signal handling is tokio's, and the pseudo-terminal is rustix's safe calls. Nothing under `proto/` or `crates/scope-link`, no `build.rs`. |
| V. Proven open source first, honest licensing | Nothing is designed that existing software could supply. The signals come from tokio's `signal` module, the end of serving from tonic's `serve_with_shutdown`, and the stop from the node's own queue. Considered: `signal-hook` and `ctrlc`, turned down because tokio's own feature does the job with no new crate. No reused code, no new license. |

**Mount neutrality.** The stop is `MountQueue::close`, which runs `stop_all`, which asks
the adapter through `Mount::stop_confirmed`. Nothing in `main.rs`, `queue.rs` or
`shutdown.rs` names Sky-Watcher, a command letter or an axis. The bound's value is
weighed against the one adapter there is (spec owner decision 1), and an equatorial
mount's adapter would be stopped the same way, within the same bound.

## Project Structure

```text
crates/scope-node/
├── Cargo.toml                                # US1: tokio's signal feature; US2: rustix's pty feature in [dev-dependencies]
├── README.md                                 # US1: TimeoutStopSec=10, the paragraph on the stop
├── src/
│   ├── lib.rs                                # US1: pub mod shutdown
│   ├── main.rs                               # US1: the signals, serve_with_shutdown, the line, the exit
│   ├── shutdown.rs                           # US1, new: STOP_BOUND, Shutdown, NodeService::shut_down
│   └── mount/queue.rs                        # US1: closed, MountQueue::close
└── tests/
    ├── stop_before_exit.rs                   # US1, new
    ├── exit_on_signal.rs                     # US1, new
    └── exit_stops_the_mount.rs               # US2, new
docs/voice/03-safety.md                       # US1: the row for a dying node split from a clean stop
```

## Interfaces

### `crates/scope-node/src/shutdown.rs` (US1, new)

```rust
//! What the node does with its mount when it is told to exit.
use crate::service::NodeService;
use std::{fmt, time::Duration};

/// How long the node waits, once told to exit, for the mount to confirm its
/// stop before it exits without that confirmation.
pub const STOP_BOUND: Duration = Duration::from_secs(8);

/// How the mount's stop ended when the node was told to exit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shutdown {
    /// The node has no mount.
    NoMount,
    /// The mount confirmed the stop.
    Confirmed,
    /// The mount's adapter could not confirm the stop.
    NotConfirmed,
    /// The stop had not ended when the bound ran out.
    TimedOut,
}
impl fmt::Display for Shutdown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoMount => f.write_str("no mount to stop"),
            Self::Confirmed => f.write_str("the mount confirmed the stop"),
            Self::NotConfirmed => f.write_str("the mount did not confirm the stop"),
            Self::TimedOut => write!(
                f,
                "the mount did not confirm the stop within {} s",
                STOP_BOUND.as_secs()
            ),
        }
    }
}

impl NodeService {
    /// Refuse every later move for good, invalidate the queued and in-flight
    /// ones, and wait up to `STOP_BOUND` for the mount to confirm its stop.
    pub async fn shut_down(&self) -> Shutdown {
        let Some(queue) = self.mount_queue() else {
            return Shutdown::NoMount;
        };
        match tokio::time::timeout(STOP_BOUND, queue.close()).await {
            Ok(true) => Shutdown::Confirmed,
            Ok(false) => Shutdown::NotConfirmed,
            Err(_) => Shutdown::TimedOut,
        }
    }
}
```

`crates/scope-node/src/lib.rs` gains `pub mod shutdown;` after `pub mod service;`.
`NodeService::mount_queue` is already public (`crates/scope-node/src/service.rs:217`),
so `service.rs` is not touched.

### `crates/scope-node/src/mount/queue.rs` (US1)

`State` gains `closed: bool`, documented `/// Set for good when the node is shutting
down.`, and `MountQueue::new` sets it `false`. `submit`'s refusal
(`crates/scope-node/src/mount/queue.rs:95`) becomes
`if state.stopping || state.closed { return Err(QueueError::Cancelled); }`. After
`stop_all`:

```rust
    /// Refuse every later move for good, then stop as `stop_all` does.
    /// Returns whether the adapter confirmed the stop.
    pub async fn close(&self) -> bool {
        self.shared.state.lock().unwrap().closed = true;
        self.stop_all().await
    }
```

`closed` is set before `stop_all` takes its lock, so a move submitted while an earlier
stop still holds that lock is refused too (US1-S3).

### `crates/scope-node/src/main.rs` (US1)

The imports gain `shutdown::Shutdown` in the `scope_node` list and
`use tokio::signal::unix::{signal, SignalKind};`. `serve` begins:

```rust
    // Taken before the listener binds, so a signal from then on stops the mount.
    let mut terminate = signal(SignalKind::terminate())?;
    let mut interrupt = signal(SignalKind::interrupt())?;
```

and its last statement, tonic's `serve(config.listen)` awaited
(`crates/scope-node/src/main.rs:188`), becomes:

```rust
    let (stop_serving, stopped) = tokio::sync::oneshot::channel::<()>();
    let mut server = tokio::spawn(
        tonic::transport::Server::builder()
            .add_routes(service.routes())
            .serve_with_shutdown(config.listen, async {
                let _ = stopped.await;
            }),
    );
    let name = tokio::select! {
        served = &mut server => return Ok(served??),
        _ = terminate.recv() => "SIGTERM",
        _ = interrupt.recv() => "SIGINT",
    };
    // Take no new call, refuse every move, and stop the mount within the
    // bound. A later signal is not read: the stop runs to its end or the bound.
    let _ = stop_serving.send(());
    let stopped = service.shut_down().await;
    eprintln!("scope-node: {name}: {stopped}");
    // Exit here: the server would wait for every open stream, such as the
    // console's frames, and the runtime for any blocking task still running.
    std::process::exit(match stopped {
        Shutdown::NoMount | Shutdown::Confirmed => 0,
        Shutdown::NotConfirmed | Shutdown::TimedOut => 1,
    })
```

A bind failure still returns its error through `served??` and `main`'s
`scope-node: <error>` and exit 1, as today. `start`, `run_probe` and the one-shot modes
are not touched.

### `crates/scope-node/Cargo.toml`

US1: tokio's feature list becomes
`["fs", "io-util", "macros", "net", "process", "rt-multi-thread", "signal", "sync", "time"]`.

US2: after the `[dev-dependencies]` tokio line,

```toml
# A pseudo-terminal for the binary's mount device in tests/exit_stops_the_mount.rs.
rustix = { version = "1.1.4", features = ["pty"] }
```

### `crates/scope-node/README.md` (US1)

In the unit of § Config and systemd, `TimeoutStopSec=10` after `RestartSec=1`
(`crates/scope-node/README.md:86`). After the sentence that ends `` and to
`SupplementaryGroups`. `` (`crates/scope-node/README.md:95`), a paragraph of its own,
with a blank line before and after it:

```markdown
On SIGTERM or SIGINT (`systemctl stop`, `poweroff`, a restart, Ctrl-C) the node stops
the mount before it exits. It takes no new call, refuses every move, sends the stop to
both axes and waits up to 8 s for the mount to confirm it. Its last line on standard
error says how the stop ended, one of:

- `scope-node: SIGTERM: the mount confirmed the stop`;
- `scope-node: SIGTERM: the mount did not confirm the stop`;
- `scope-node: SIGTERM: the mount did not confirm the stop within 8 s`;
- `scope-node: SIGTERM: no mount to stop`.

SIGINT's line names SIGINT. The node exits 0 when the stop was confirmed or there is no
mount, and 1 otherwise. A second signal during the stop changes nothing. The unit's
`TimeoutStopSec=10` leaves systemd's SIGKILL 2 s after the node's bound; without it
systemd waits 90 s. A crash, `kill -9` or a power cut still ends the node without a stop.
```

### `docs/voice/03-safety.md` (US1)

The row at `docs/voice/03-safety.md:177` becomes two rows, its other cells unchanged in
the first:

```markdown
| Scope node or Pi dies: a crash, a kill or a power cut | Console | **May keep moving.** A GoTo leg runs to its end, and Track's rate continues | May keep moving | Refused by the console | "The scope node isn't answering. The telescope may still be moving." Use the power switch |
| Scope node stopped, or the Pi shut down: SIGTERM or SIGINT | Scope node | Axes stopped before the node exits; it waits up to 8 s for the mount to confirm, and its last line says whether it did | Axes stopped | Refused by the console | Same as the row above: the console sees the node go |
```

## What was run and what was read

**Run on 2026-11-21, before the draft**, on the branch `probe/044` cut from `main` at
7164e87f, three times each with the same outcome:

- the binary's six tests (US1-S9, US1-S10, US2-S1 to US2-S4) on `main`'s node code: all
  six fail. With a GoTo and a rate under way, SIGTERM and SIGINT each ended the process
  by the signal, with no line and no `:K` or `:L` sent, and 2 s after the exit the board
  said both axes still moved. With no mount and with a missing device the process died
  by the signal. `stop_before_exit.rs` does not build on `main`: `shutdown` and
  `shut_down` do not exist;
- the trial fix as § Interfaces gives it, with the README and the two rows: all
  fourteen scenarios' tests pass;
- with the fix, both gates' Rust commands: `lint`'s four and `test`'s two, every
  existing test of the workspace passing unchanged;
- `Cargo.lock` unchanged by tokio's `signal` feature and by rustix's `pty` feature, under
  `--locked`;
- as exploratory cases, each once and not committed: a call on an open channel during
  the stop (`UNAVAILABLE`) and a new connection then (refused); `AxisState` and
  `StopAll` called in process after the shutdown (both answered, `StopAll` sending
  `:K1` and `:K2` again); an imaging exposure in progress with the
  simulated camera (its stream ended `UNKNOWN` at the exit); SIGHUP (the node died by
  it, as today).

**Read, not run**:
- what the console shows when the node exits, and that it cannot tell a clean exit
  from a kill (spec Clarifications);
- a `StopAll` asked after the shutdown's stop, cut short by the exit (spec Edge Cases);
- a signal during `rpicam` discovery, before `serve`; the one-shot modes;
- an INDI exposure in progress, the cooler, and a `.part` file left by an exit
  mid-save (spec Edge Cases);
- systemd: `TimeoutStopSec=` defaults to `DefaultTimeoutStopSec=`, 90 s
  (`systemd.service(5)`, `systemd-system.conf(5)`); `KillMode=` defaults to
  `control-group`, which sends SIGTERM to the node and to every process left in its
  unit, such as the `rpicam` subprocess, and SIGKILL after `TimeoutStopSec=`
  (`systemd.kill(5)`); exit 0 and death by SIGTERM or SIGINT are clean, any other exit
  status fails the unit (`SuccessExitStatus=`); `Restart=always` does not restart a unit
  stopped with `systemctl stop` (`systemd.service(5)`); and at shutdown a unit ordered
  `After=` another stops before it (`systemd.unit(5)`). Read from the installed man
  pages.

**Not run at all**: a real serial device, the real board, the Pi, systemd, the console,
the page, the page half of the `test` gate (`npm ci` installs packages, and no story
writes anything the page is built from), and the factory's gate boundary itself (US2's
tests ran in a sandbox with its `/dev`: § Prerequisites 4).

## Read from the diff, not from a test

- **The signals are taken before the bind** (FR-002). The binary's tests send a signal
  only once the node accepts a connection; a node that took them later might still pass.
  *Check:* `signal(...)` is the first statement of `serve`.
- **The exit does not wait** (FR-009). US2-S1 and US2-S2 hold it: their client's open
  channel would keep a server that waits for every connection from finishing, and the
  node would not exit within 1 s. US1's tests hold no connection, so in US1's diff it
  shows only as `std::process::exit`. *Check:* `serve` never awaits `server` after the
  signal.
- **The README paragraph and the two rows.** No test reads them. Copy them from
  § Interfaces.
- **`closed` is set before `stop_all`.** US1-S3 refuses no move, so the order shows only
  in the diff.

## Shapes that existing tests pin

Every existing test passes unchanged with both stories built (run on the probe). The
files beside the change, and what the stories do to what they observe:

- **`crates/scope-node/tests/mount_link.rs`**. `binary_keeps_running_with_a_missing_mount_device`
  (`crates/scope-node/tests/mount_link.rs:160`) runs the binary for 5 s and ends it with
  SIGKILL through `child.kill()`, which no handler sees: unchanged.
  `disconnected_service_refuses_moves_stops_at_once_and_serves_frames_without_counts`
  (`crates/scope-node/tests/mount_link.rs:87`) calls `stop_all` on an open queue:
  `closed` is false, so its answers are as today.
- **`crates/scope-node/tests/cli.rs`** and **`crates/scope-node/tests/mount_config.rs`**.
  `--check-config` and config refusals exit before `serve`: unchanged.
- **`crates/scope-node/tests/skywatcher_stop.rs`** and
  **`crates/scope-node/tests/mount_reset.rs`**. 017's halt and 024's stop over a lost
  link are the adapter's, which no story touches; the shutdown uses them as they are.
- **`crates/scope-node/tests/mount_step.rs`** and **`crates/scope-node/tests/mount_start.rs`**.
  039's Stop goes through `stop_all` on an open queue, whose `stopping` flag is as
  today.
- **`crates/simulator/tests/stop_and_watchdog.rs`** builds a `MountQueue` and calls
  `stop_all`; `close` is a new method only.
- **The simulator's node** (`crates/simulator/src/node.rs`) builds a `NodeService` and
  serves it itself: `main.rs` is not in its path.
- **Every finder test** drives the simulator's mount or the node's service in process,
  never the node's binary.
- **US1's test files are existing tests for US2.** US2 writes neither; it copies the
  process helpers it needs.

## Decisions and the alternatives rejected

| Decision | Rejected | Why |
|---|---|---|
| tokio's `signal` feature | `signal-hook` or `ctrlc`; `libc` with `unsafe` | Owner approval 1: no new crate, and its registry is already in the lockfile. `unsafe` is a critical path. |
| `serve_with_shutdown`, then exit without awaiting the server | Await the server's graceful end | The console holds a frame stream open for good, so a graceful end never comes (§ Read from the diff). |
| `std::process::exit` after the line | Return from `main` | Dropping the runtime waits for blocking tasks, such as a frame being saved. |
| Close the queue, then the queue's own stop | `Mount::stop_confirmed` called directly | The queue invalidates queued and in-flight moves first, and orders the stop behind a `StopAll` already running (US1-S3). |
| A `closed` flag | Leave `stopping` set | `stopping` is the running stop's; a second stop, the watchdog's, clears it at its end (US2-S3). |
| `QueueError::Cancelled` for a closed queue | A new variant and status | `QueueError` is public and matched; the message `mount command discarded by Stop` is true. |
| The bound around the whole stop | A bound per command | The adapter has its own per-command limits; the node's bound is a backstop for any adapter. |
| 8 s | 2 s or 5 s | Owner decision 1. |
| Exit 1 when not confirmed | 0 always | Owner decision 2. |
| A second signal ignored | A second signal exits at once | Owner decision 3. |
| One line on standard error | A line per step | The journal keeps one line a shutdown, and the tests read it whole. |
| `shut_down` in a module of its own | In `service.rs` | `service.rs` is not touched, and the module holds the bound and the words together. |
| Two stories | One | The two together measured 66.9 % of 65,536 B (§ Plan traps 2); US2 is also owner decision 4. |

## User Story 1 - The stop on a signal

Tests first: `stop_before_exit.rs` and `exit_on_signal.rs`. Then `MountQueue::close`;
`shutdown.rs` and its `mod` line; `serve`'s signals, serving and exit with tokio's
`signal` feature; the README; the two rows.

## User Story 2 - The binary with a moving mount

The dev-dependency line, then `exit_stops_the_mount.rs`. Its scenarios pass on arrival,
with US1 merged; on `main`'s node code all four fail (§ What was run and what was read).

## Verification (every story, before proposing)

- **The factory gates** (`ergane.yaml` at 7164e87f):
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

1. **The earlier specs have landed:** every spec in `depends_on_landed`. Met at 7164e87f.
2. **`main` is still as read.** Every citation here was read at 7164e87f on 2026-11-21.
   If `main` moves in a file § Project Structure names, or in a test § Shapes names,
   re-read them and re-run the validator.
3. **The owner decisions are settled and the owner approvals given** (spec § Owner
   decisions, § Owner approvals). The decisions: met on 2026-11-21, when the owner took
   the four recommended answers ("Yes, take them"). The approvals are given by the flip.
4. **The gate boundary gives a test a pseudo-terminal** (US2 only). Met on 2026-11-21.
   US2's tests open `/dev/ptmx` and a device under `/dev/pts`. The `test` gate runs
   inside the factory's boundary, a bubblewrap sandbox that mounts a `/dev` of its own
   (`--dev /dev`), which holds both. US2's four tests, as built on the probe, passed
   inside bubblewrap with the boundary's `--dev /dev`, `--proc /proc` and
   `--unshare-pid`, though not with its whole argument list. If the factory's own run
   still fails, owner decision 4 is answered (b).
5. **Probe builds.** Not met: no story has been built from this text.
6. **No operator-staged files are needed.**

## Outside the factory

Written here and not run.

**What the Pi's power-loss service may rely on**, once the Pi runs a node built with
044: `systemctl poweroff` stops `scope-node.service` with SIGTERM, and the node stops the
mount through its queue before it exits, within 8 s. It may not rely on a stop after a
crash, a `kill -9` or a cut of the Pi's power with the UPS spent; nor on the axes being
at rest when the node exits, since the node waits for the board's acknowledgement, not
for rest, and the mount decelerates on its own power. The UPS must hold the Pi through
the bound and the rest of the shutdown. The unit the Pi installs
(`~/.cache/finder-flash/pi-setup.sh`) has no `TimeoutStopSec=`, so systemd's 90 s
applies; adding `TimeoutStopSec=10` there is ops tooling, not this spec.

**The order of units at shutdown** (`systemd.unit(5)`): the inverse of their start
order. A console unit ordered `After=scope-node.service` is stopped first; its
heartbeats end, and systemd then sends the node SIGTERM once the console has exited, so
the node's own stop runs then, and the watchdog's, a second after the last heartbeat,
only if the node is still running. The node's unit, ordered `After=network.target
indiserver.service` on the Pi, is stopped before the INDI server and the network. Units
with no order between them stop at the same time. With `KillMode=control-group` the
`rpicam` subprocess gets SIGTERM with the node.

**The field check**, by the owner, with the node rebuilt from a commit with 044 and
installed, the mount powered and its axes free to move:

1. Start a slow move: Track on a target, or a GoTo of more than 10°.
2. Mid-move, run `sudo systemctl stop scope-node`. Both axes should stop within about
   2 s.
3. Run `journalctl -u scope-node -n 5`: the last line should be
   `scope-node: SIGTERM: the mount confirmed the stop`. Run
   `systemctl show -p Result scope-node`: it should read `Result=success`.
4. Run `sudo systemctl start scope-node`. Switch the mount off, then stop the node again
   at once: the stop should take no more than about 8 s, the line should say the mount
   did not confirm, and `Result=exit-code`.
5. Optionally, with the UPS fitted and a move under way, take the Pi's mains power away
   and let the power-loss service shut it down: the axes should stop.

An axis that keeps moving at step 2 means the stop did not reach the mount: use the
power switch, and report the journal.

**A new field kit's Pi binary is needed** once 044 lands: the node's binary changes, and
the Pi's current one does not stop the mount on a signal.

## Plan traps

They bind every story.

1. **Gate evidence or nothing.** The judge credits only gate results and the story's
   diff. *Check:* every Then names a committed test the `test` gate runs.
2. **One evidence budget.** *Check:* each story's diff stays under 64 KiB, measured with
   `git diff <base> -- . ':(exclude)crates/finder/web/dist' | wc -c`. Measured on the
   probe's commits: US1 about 29,100 B, 44.4 % of 65,536 B (the code and docs 10,098 B,
   its `Cargo.toml` hunk about 890 B, `stop_before_exit.rs` 13,149 B,
   `exit_on_signal.rs` 4,978 B); US2 about 14,900 B, 22.7 % (`exit_stops_the_mount.rs`
   14,286 B, its `Cargo.toml` hunk about 610 B). At the 1.35 times a factory build has
   run over its probe on earlier specs, US1 would be about 39,300 B, 60.0 %.
3. **A spec fixed mid-epic needs a relaunch.** *Check:* no builder edits
   `specs/044-node-stops-mount-before-exit/` or `.specify/`.
4. **An earlier story's test file is an existing test.** *Check:* US2 writes
   `exit_stops_the_mount.rs` and neither of US1's files; it copies the process helpers.
5. **Paused time cannot run a child process.** *Check:* the binary's tests are
   `#[tokio::test]`, never `start_paused`; the in-process ones are `start_paused`.
6. **The wire must be the test's alone.** *Check:* the node rig stops frames and beats
   every 200 ms; US2's client beats every 250 ms, or the watchdog refuses its moves.
7. **An oracle copied from the code proves nothing.** *Check:* the tests write 6,000,
   1,500, 1,200 and 8,000 ms, the four lines and the statuses as literals; none reads
   `STOP_BOUND` or formats `Shutdown` to get an expected value.
8. **A negative assertion passes when nothing happens.** *Check:* "no `:K` or `:L`" in
   US1-S8 stands beside its `:e1`; "no motion command" beside the exact `:K` list;
   "refused" beside `Confirmed` or `TimedOut`.
9. **The exit must not wait.** *Check:* `std::process::exit` after the line; awaiting the
   server fails US2-S1's 1 s.
10. **Signals only to the test's own child.** *Check:* `kill` is given the child's own
    pid from `Child::id`.
11. **The emulator's replies come back through the relay.** *Check:* the relay writes
    `discard_input()` to the master after every write to the emulator.
12. **An unused helper fails clippy.** *Check:* each test file holds only the helpers
    its own tests use.
13. **rustdoc denies warnings.** *Check:* no doc comment holds an angle bracket outside
    backticks.
14. **Each worktree builds on its own.** *Check:* no story sets `CARGO_TARGET_DIR`.
15. **The lockfile does not move.** *Check:* every cargo command runs with `--locked`,
    which fails if a feature line pulls in a crate.
16. **The watchdog is not changed.** *Check:* `crates/scope-node/src/mount/watchdog.rs`
    and `crates/scope-node/src/service.rs` are not in either diff.

## Risks

| Risk | Mitigation |
|---|---|
| The factory's gate boundary has no pseudo-terminal, and US2 cannot pass there. | Prerequisite 4 ran US2's tests in a sandbox with the boundary's `/dev`; if the factory's run still fails, owner decision 4 (b) drops US2. |
| A loaded host stretches the binary's wall-clock bounds: 1 s for an exit with nothing to wait for, 1,400 to 2,500 ms for US2-S3 and 9 s for US2-S4. | Each bound has its tolerance stated; the waits inside them are timer-driven. |
| US1 at a factory build's usual overrun reaches 60 % of the budget. | It stays well under the 64 KiB refusal (§ Plan traps 2); the test files are the bulk. |
| The node exits 1 on every stop while its mount is unplugged, and the unit reads failed. | Owner decision 2. |
| A console Stop pressed during the shutdown is refused, and the console says the mount may still move. | The node's own stop runs; its line in the journal says how it ended. |
| The real board takes `:K` differently from the emulator. | The field check. |

## Refinement Record

Drafted on 2026-11-21 by a plate-solver spec session against `main` at 7164e87f, from a
probe of the fault and a trial fix that day, a reading of the node's mount code, its
tests, 017's, 024's and 039's trios, systemd's man pages and the constitution. Writes
come from the task text. Shared writes name the merge edge that orders them, and the
story that last wrote the same file on `main`.

The four owner decisions of spec.md were settled on 2026-11-21 with the recommended
answers (spec § Owner decisions), and the record's `open_questions` are empty. This
record approves nothing: the owner approvals of spec.md are given by the flip.

```yaml
version: 1
reviewed_revision: 7164e87feb8a98a7a318e1c3ba3a7d5f706f44d8
stories:
  US1:
    tests:
      - path: crates/scope-node/tests/stop_before_exit.rs
        status: new
        disposition: add
        assertion: "a GoTo leg on both axes, 1 s under way: the shutdown answers Confirmed after 0 ms over exactly K1 and K2, each answered; a GoTo and a rate asked after it are refused ABORTED with mount command discarded by Stop; 2 s later both axes are at rest; no motion command after the mark; Confirmed reads the mount confirmed the stop"
        decision: "spec.md:US1-S1"
      - path: crates/scope-node/tests/stop_before_exit.rs
        status: new
        disposition: add
        assertion: "a rate move on both axes, 1 s under way: the shutdown answers Confirmed after 0 ms over exactly K1 and K2, each answered; later moves are refused; both axes rest 2 s later; no motion command after the mark"
        decision: "spec.md:US1-S2"
      - path: crates/scope-node/tests/stop_before_exit.rs
        status: new
        disposition: add
        assertion: "with five K1 replies dropped, a StopAll already running is carried through first and answers OK, and the shutdown's own stop follows: Confirmed 1,500 ms after the mark over K1 five times unanswered, then K2, L1, K1 and K2 answered"
        decision: "spec.md:US1-S3"
      - path: crates/scope-node/tests/stop_before_exit.rs
        status: new
        disposition: add
        assertion: "a board powered off under a rate: the shutdown answers NotConfirmed after 6,000 ms over K1, K2, L1 and L2 five times each, all unanswered; NotConfirmed reads the mount did not confirm the stop"
        decision: "spec.md:US1-S4"
      - path: crates/scope-node/tests/stop_before_exit.rs
        status: new
        disposition: add
        assertion: "a link lost after three failed reads of a powered-off board, the fourth refused mount not connected: the shutdown answers NotConfirmed after 1,200 ms, and its stop commands are K1, K2, L1 and L2 once each, all unanswered"
        decision: "spec.md:US1-S5"
      - path: crates/scope-node/tests/stop_before_exit.rs
        status: new
        disposition: add
        assertion: "a stand-in mount whose stop never answers: a GoTo and a rate asked 100 ms into the shutdown, and a GoTo after it, are refused; the shutdown answers TimedOut 8,000 ms after the mark; the stand-in recorded only goto Azimuth 1000; TimedOut reads the mount did not confirm the stop within 8 s"
        decision: "spec.md:US1-S6"
      - path: crates/scope-node/tests/stop_before_exit.rs
        status: new
        disposition: add
        assertion: "a node built with no mount: the shutdown answers NoMount after 0 ms, which reads no mount to stop"
        decision: "spec.md:US1-S7"
      - path: crates/scope-node/tests/stop_before_exit.rs
        status: new
        disposition: add
        assertion: "a mount never connected: the shutdown answers NotConfirmed after 0 ms; in the 3 s after it the link still sends e1, and no command starting K or L reaches the board"
        decision: "spec.md:US1-S8"
      - path: crates/scope-node/tests/exit_on_signal.rs
        status: new
        disposition: add
        assertion: "the binary with no mount table, sent SIGTERM and, as a second node, SIGINT, exits within 1 s with status 0 and the last line scope-node: SIGTERM: no mount to stop, or the same with SIGINT"
        decision: "spec.md:US1-S9"
      - path: crates/scope-node/tests/exit_on_signal.rs
        status: new
        disposition: add
        assertion: "the binary whose mount device does not exist, 200 ms after it listens, sent SIGTERM and, as a second node, SIGINT, exits within 1 s with status 1, its stderr holding mount not connected and its last line scope-node: SIGTERM: the mount did not confirm the stop, or the same with SIGINT"
        decision: "spec.md:US1-S10"
      - path: crates/scope-node/tests/mount_link.rs
        status: existing
        disposition: preserve
        assertion: "the binary keeps running 5 s with a missing mount device and is ended by SIGKILL; a disconnected service refuses moves and answers Stop at once"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/scope-node/tests/cli.rs
        status: existing
        disposition: preserve
        assertion: "check-config prints every setting with the token masked, and bad keys are refused naming the key"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/scope-node/tests/skywatcher_stop.rs
        status: existing
        disposition: preserve
        assertion: "017's halt on a connected board is K1, K2, then L after a silent K, five tries a command"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/scope-node/tests/mount_reset.rs
        status: existing
        disposition: preserve
        assertion: "Stop over a lost link reaches the kept stream within four reply timeouts and interrupts a connect"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/scope-node/tests/mount_step.rs
        status: existing
        disposition: preserve
        assertion: "039's Stop through stop_all on an open queue answers and sends K1 and K2 as before"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/simulator/tests/stop_and_watchdog.rs
        status: existing
        disposition: preserve
        assertion: "a MountQueue built directly still stops and refuses as before"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "StopAll answers OK only when the mount confirmed, and otherwise UNAVAILABLE with the mount did not confirm the stop (017 FR-011)"
      - "a StopAll already running is carried through to its end (017 FR-010)"
      - "Stop over a lost link goes once a command over the kept stream (024 FR-018)"
      - "the node starts and serves whether or not the mount answers (024 FR-001)"
      - "MountQueue::stop_all, QueueError, the Mount trait, the watchdog and every RPC's answer keep their shapes"
    checks:
      development:
        - "cargo nextest run -p scope-node --locked --test stop_before_exit"
        - "cargo nextest run -p scope-node --locked --test exit_on_signal"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs, a new field kit's Pi binary and the field check of plan.md § Outside the factory"
    writes:
      - crates/scope-node/Cargo.toml
      - crates/scope-node/README.md
      - crates/scope-node/src/lib.rs
      - crates/scope-node/src/main.rs
      - crates/scope-node/src/mount/queue.rs
      - crates/scope-node/src/shutdown.rs
      - crates/scope-node/tests/exit_on_signal.rs
      - crates/scope-node/tests/stop_before_exit.rs
      - docs/voice/03-safety.md
    no_write_reason: ""
    shared_writes:
      - path: crates/scope-node/Cargo.toml
        other_spec: 044-node-stops-mount-before-exit
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and adds its dev-dependency line after US1's tokio feature"
      - path: crates/scope-node/Cargo.toml
        other_spec: 030-imaging-camera-link
        other_story: US6
        disposition: merge_order
        reason: "044 depends_on_landed 030, whose US6 last changed the node's Cargo.toml (58aff47c)"
      - path: crates/scope-node/src/main.rs
        other_spec: 030-imaging-camera-link
        other_story: US8
        disposition: merge_order
        reason: "044 depends_on_landed 030, whose US8 last changed main.rs (49f8dcd2)"
      - path: crates/scope-node/src/lib.rs
        other_spec: 030-imaging-camera-link
        other_story: US1
        disposition: merge_order
        reason: "044 depends_on_landed 030, whose US1 last changed lib.rs (3849a0ff)"
      - path: crates/scope-node/src/mount/queue.rs
        other_spec: 017-console-stop-confirm
        other_story: US3
        disposition: merge_order
        reason: "044 depends_on_landed 017, whose US3 last changed queue.rs (b5146504)"
      - path: crates/scope-node/README.md
        other_spec: 039-mount-replies-in-step
        other_story: US2
        disposition: merge_order
        reason: "044 depends_on_landed 039, whose US2 last changed the node's README (33357d94)"
    open_questions: []
  US2:
    tests:
      - path: crates/scope-node/tests/exit_stops_the_mount.rs
        status: new
        disposition: add
        assertion: "the binary over the emulator behind a pseudo-terminal, a GoTo on azimuth and a rate on altitude under way, sent SIGTERM: the board says neither axis moves 2 s after the exit; its stop commands after the mark are exactly K1 and K2, each answered; no motion command; exit within 1 s with status 0 and the last line scope-node: SIGTERM: the mount confirmed the stop"
        decision: "spec.md:US2-S1"
      - path: crates/scope-node/tests/exit_stops_the_mount.rs
        status: new
        disposition: add
        assertion: "the binary over the emulator behind a pseudo-terminal, a GoTo on azimuth and a rate on altitude under way, sent SIGINT: the board says neither axis moves 2 s after the exit; its stop commands after the mark are exactly K1 and K2, each answered; no motion command; exit within 1 s with status 0 and the last line scope-node: SIGINT: the mount confirmed the stop"
        decision: "spec.md:US2-S2"
      - path: crates/scope-node/tests/exit_stops_the_mount.rs
        status: new
        disposition: add
        assertion: "with five K1 replies dropped, SIGTERM, SIGINT 200 ms later and a GoTo 200 ms after that: the GoTo fails; exit with status 0 between 1,400 and 2,500 ms after SIGTERM; exactly one line starts scope-node: SIG and it is the SIGTERM confirmed line; the stop commands begin K1 five times, K2, L1, and any after them start K; no motion command; both axes at rest"
        decision: "spec.md:US2-S3"
      - path: crates/scope-node/tests/exit_stops_the_mount.rs
        status: new
        disposition: add
        assertion: "the board powered off under the moves, then SIGTERM: exit within 9 s with status 1 and a last line starting scope-node: SIGTERM: the mount did not confirm the stop"
        decision: "spec.md:US2-S4"
      - path: crates/scope-node/tests/mount_link.rs
        status: existing
        disposition: preserve
        assertion: "the binary keeps running 5 s with a missing mount device and is ended by SIGKILL"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "US1's exit_on_signal.rs and stop_before_exit.rs pass unchanged: US2 writes neither"
      - "no file under crates/scope-node/src is written"
      - "Cargo.lock does not change"
    checks:
      development:
        - "cargo nextest run -p scope-node --locked --test exit_stops_the_mount"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - crates/scope-node/Cargo.toml
      - crates/scope-node/tests/exit_stops_the_mount.rs
    no_write_reason: ""
    shared_writes: []
    open_questions: []
```
