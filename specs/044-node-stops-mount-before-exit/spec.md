---
state: draft
depends_on_landed: ["014-goto", "017-console-stop-confirm", "024-mount-link-recovery", "030-imaging-camera-link", "039-mount-replies-in-step"]
# Drafted 2026-11-21 against `main` at 7164e87f (7164e87feb8a98a7a318e1c3ba3a7d5f706f44d8),
# at the owner's ask the same day ("Yes, draft 044"), from a probe of that day on the
# branch `probe/044` cut from 7164e87f: tests that show the fault, run on `main`'s code,
# and a trial fix with which they pass, with both gates' Rust commands passing. Every
# `path:line` here was read at 7164e87f. The owner took the four recommended answers on
# 2026-11-21 ("Yes, take them"). The trio has not yet been built from its own text
# (plan.md § Prerequisites). 027's test
# layout is not named: no story writes a finder test. What rests on a run and what on
# reading is said where it is used (plan.md § What was run and what was read).
---

# Feature Specification: The Node Stops the Mount Before It Exits

**Feature Branch**: `044-node-stops-mount-before-exit`

**Created**: 2026-11-21

**Status**: Draft

**Input**: The finder is moving to a Pi behind a UPS, and a power-loss service will shut
the Pi down cleanly. Today the scope node has no signal handling: SIGTERM and SIGINT
end it at once, so `systemctl stop`, `poweroff`, a deploy's restart and Ctrl-C leave a
GoTo leg running to its end and a rate move carrying on, with nothing left to stop them.
Make the node, on SIGTERM or SIGINT, stop taking calls, stop the mount through the queue
it already has, wait a bounded time for the stop to be confirmed, say so on one line,
and exit.

## Clarifications

### Session 2026-11-21 (the probe on `main` at 7164e87f)

- Q: What does this trio read besides itself? → A: Nothing about the feature. `spec.md`
  holds every requirement and scenario, and `plan.md` every design decision, the
  interfaces, the shapes that existing tests pin and the alternatives rejected. A story
  that would change a decision escalates rather than re-deciding it. The probe's branch
  and notes are not in the repository; what they showed is written here.
- Q: What happens today on SIGTERM or SIGINT? → A: The signal's default action ends the
  process. `main` is a `#[tokio::main]` (`crates/scope-node/src/main.rs:39`) whose
  `serve` (`crates/scope-node/src/main.rs:141`) awaits tonic's `serve(config.listen)`
  (`crates/scope-node/src/main.rs:190`) for good, and nothing in the node takes a signal.
  The watchdog (`crates/scope-node/src/mount/watchdog.rs:51`) lives in the same process
  and ends with it. The probe ran it on `main`'s code with the built binary and the
  emulator behind a pseudo-terminal (US2's rig): with a GoTo on azimuth and a rate on
  altitude under way, SIGTERM and SIGINT each ended the process by the signal, with no
  line, no `:K` or `:L` reached the board, and 2 s after the exit the board said both
  axes still moved. A node with no mount, and one whose device is missing, likewise died
  by the signal. `docs/voice/03-safety.md:177` already lists the case: "May keep
  moving".
- Q: What stop does the node already have? → A: `MountQueue::stop_all`
  (`crates/scope-node/src/mount/queue.rs:115`) invalidates queued and in-flight moves,
  then asks the adapter through `Mount::stop_confirmed`
  (`crates/scope-node/src/mount/mod.rs:91`) and answers whether it confirmed. The
  watchdog and the `StopAll` RPC (`crates/scope-node/src/service.rs:589`) both use it.
  Its `stopping` flag refuses moves only while a stop runs
  (`crates/scope-node/src/mount/queue.rs:95`, `crates/scope-node/src/mount/queue.rs:126`),
  so on its own a move asked just after it could still start. The fix closes the queue
  for good before that stop (FR-003).
- Q: What is the fix? → A: Three parts. `MountQueue::close` marks the queue closed and
  runs `stop_all`. `NodeService::shut_down` runs `close` within `STOP_BOUND` and says how
  it ended, or that there is no mount. `serve` takes SIGTERM and SIGINT before the
  listener binds, serves with tonic's `serve_with_shutdown`, and on the first signal
  ends the serving, runs `shut_down`, writes one line and exits. The stop goes through
  the queue and the `Mount` trait only; nothing in the Sky-Watcher adapter changes.
- Q: What does the stop look like on the wire? → A: 017's halt, unchanged: `:K1` then
  `:K2`, and `:L` on an axis whose `:K` was not acknowledged
  (`crates/scope-node/src/mount/skywatcher/protocol.rs:403`). Run on the emulator, in
  process and behind the binary: both axes acknowledged `:K1` and `:K2` and came to
  rest, from a GoTo leg on both axes, a rate move on both, and a GoTo with a rate
  (US1-S1, US1-S2, US2-S1).
- Q: What does the bound wait for? → A: The adapter's own answer. On a healthy board it
  comes at once on paused time (US1-S1). On a board that answers nothing it comes after
  6,000 ms on paused time: four commands of five tries at 300 ms each
  (`crates/scope-node/src/mount/skywatcher/transport.rs:6`,
  `crates/scope-node/src/mount/skywatcher/transport.rs:67`; US1-S4). A status read
  already in flight holds the adapter first, at most five tries more
  (`crates/scope-node/src/mount/skywatcher/protocol.rs:192`), which the binary's capture
  loop makes likely (`crates/scope-node/src/capture.rs:50`). Over a lost link the stop
  goes once a command over the kept stream, 024's FR-018, and answers after 1,200 ms
  (US1-S5). `STOP_BOUND` is the backstop for an adapter that never answers (US1-S6);
  its value is owner decision 1.
- Q: What does a call during the stop get? → A: Nothing that moves the mount. The queue
  is closed before the stop begins, so `GotoAxis` and `RateAxis` answer `ABORTED` with
  `mount command discarded by Stop` (`crates/scope-node/src/service.rs:503`), during the
  stop and after it (US1-S1, US1-S6). Over the network, once serving has ended, the probe
  saw a call on a client's open channel fail `UNAVAILABLE` and a new connection
  refused (one exploratory run; US2-S3 asserts only that the call fails and that no
  motion command reaches the board). In process, after the shutdown, `AxisState` and
  `StopAll` still answer, and that `StopAll` sends `:K1` and `:K2` again (one
  exploratory run).
- Q: Does this change 017 or 024? → A: No requirement's text changes. 017's FR-010 reads:
  "The node SHALL carry a `StopAll` it has started through to the end even if the caller
  goes away" (`specs/017-console-stop-confirm/spec.md:405`). A `StopAll` running when the
  signal comes is carried through first, and the shutdown's own stop follows (US1-S3). A
  `StopAll` begun after the shutdown's stop may be cut short by the exit, after the
  shutdown's own stop has reached the board; that is the one place the exit ends a stop.
  017's FR-011, `StopAll`'s answer, is unchanged. 024's FR-018 reads, in part: "After the
  link's first connect, while it is lost or initialising again, `StopAll` and the node's
  watchdog SHALL send the stop to both axes over the stream of the link's last successful
  connect" (`specs/024-mount-link-recovery/spec.md:707`): the shutdown takes that same
  path (US1-S5). 024's FR-001, that the node starts and serves whether or not the mount
  answers (`specs/024-mount-link-recovery/spec.md:648`), is unchanged.
- Q: What is a second signal during the stop? → A: Ignored in the trial fix: once the
  first is taken, the node reads no other, so the stop runs to its answer or the bound.
  Run behind the binary: SIGINT 200 ms after SIGTERM, during a stop held 1.5 s by five
  lost `:K1` replies, changed nothing (US2-S3). Owner decision 3.
- Q: What does the console see? → A: By reading, not run: the same as when the node is
  killed. The console's link ends on a failed heartbeat or frame stream, or 1 s of
  silence (`crates/finder/src/link.rs:32`), and a GoTo or Track then ends "halted by a
  lost link" (`crates/finder/src/goto.rs:813`). The console cannot tell a clean exit from
  a kill; only the axes differ. Its own Stop round, `STOP_WINDOW`
  (`crates/finder/src/link.rs:36`), finds no node to answer. Nothing in the console
  changes.
- Q: Does anything here know about alt-az mounts or Sky-Watcher? → A: No. The stop is
  the queue's and the `Mount` trait's, and the bound's rationale is the worst case of
  the one adapter there is today. An equatorial mount's adapter would be stopped the
  same way. No Sky-Watcher code, constant or name enters `main.rs`, `queue.rs` or the new
  module.
- Q: What rests on a run, and what on reading? → A: Every scenario here was run on the
  probe, with the trial fix of plan.md § Interfaces, three times each with the same
  outcome; the binary's ones fail on `main`'s code. The stories' own text has not been
  built. plan.md § What was run and what was read lists each claim. Nothing was run on
  a real serial device, the real board, a Pi or systemd.
- Q: What is left for later? → A: The items under "Out of scope" in § Assumptions.

## Owner decisions

The owner took each recommended answer on 2026-11-21 ("Yes, take them"), and the
scenarios name them. The alternatives are kept below for the record.

1. **The bound's value** (FR-007). *Settled 2026-11-21: the recommended answer.* Recommended: 8 s, with the unit's
   `TimeoutStopSec=10`. It covers the one adapter's worst case before its own answer, by
   reading: 6 s of stop commands on a board that answers nothing, behind at most 1.5 s
   of a status read in flight (Clarifications), which leaves 0.5 s. The bound then never
   cuts that adapter's `:K`/`:L` escalation short, and on a healthy mount the node exits
   at once, so the 8 s is paid only when the mount does not answer. Weighed against:
   - systemd's `TimeoutStopSec`, 90 s by default (`DefaultTimeoutStopSec=`, man
     `systemd-system.conf(5)`): 10 s leaves the SIGKILL 2 s after the bound;
   - the console's Stop round, `STOP_WINDOW` 2 s with `STOP_ATTEMPT` 200 ms a try
     (`crates/finder/src/link.rs:34`, `crates/finder/src/link.rs:36`): the node's stop
     already outlives a console that gives up, since `StopAll` runs in a task of its own
     (`crates/scope-node/src/service.rs:597`);
   - a UPS that gives the Pi limited time on battery: the bound is paid once per
     shutdown, and only when the mount is silent; how long the Pi's UPS holds is not
     known here;
   - instead, (b) 2 s, the console's own window: a node behind a silent mount exits
     within 2 s, but a stop held by a status read in flight may never send `:K2`, and no
     `:L` is ever sent; or (c) 5 s: `:K1` and `:K2` always reach the wire, and the `:L`
     escalation is cut short.
2. **The exit status when the stop is not confirmed** (FR-006). *Settled 2026-11-21: the recommended answer.* Recommended: 1,
   for `NotConfirmed` and `TimedOut`; 0 for `Confirmed` and `NoMount`. systemd counts a
   service that exits 1 after its stop as failed (`SuccessExitStatus=`, man
   `systemd.service(5)`), so `systemctl status` and the journal show that the mount may
   still move, and a restart is still not prevented: `Restart=always` does not restart a
   unit stopped with `systemctl stop`. The cost: a node whose `[mount]` device is absent
   or never connected also exits 1 on every stop (US1-S8, US1-S10). Instead: (b) 0
   always, the line alone telling; or (c) 0 also for a mount that was never connected,
   which needs a mount-neutral way to tell "never connected" from "lost", that the
   `Mount` trait does not have (`crates/scope-node/src/mount/mod.rs:68`).
3. **A second signal during the stop** (FR-008). *Settled 2026-11-21: the recommended answer.* Recommended: ignored. The stop
   is the reason for waiting, and the bound already caps it; systemd sends one SIGTERM
   and then SIGKILL at `TimeoutStopSec`. Instead: (b) a second signal ends the node at
   once with its own line and status 1, as many servers treat a second Ctrl-C, which
   lets an impatient hand abandon the stop mid-way.
4. **The binary's own test with a moving mount** (US2, FR-014). *Settled 2026-11-21: the recommended answer.* Recommended: yes.
   US2 runs the built binary with the emulator behind a pseudo-terminal, the only test
   in which the shipped serial path stops a moving mount on a signal. It needs rustix's
   `pty` feature in `[dev-dependencies]`: a feature of a crate the node already uses, no
   new crate, and `Cargo.lock` does not change (run). Instead: (b) no US2: the stop is
   then held in process (US1-S1 to US1-S8) and the binary's signal path only with no
   mount to stop (US1-S9, US1-S10). The factory's gate boundary gives a test a
   pseudo-terminal: it mounts a `/dev` of its own, which holds `/dev/ptmx` and `/dev/pts`,
   and US2's four tests passed in such a sandbox (plan.md § Prerequisites 4).

## Owner approvals

Flipping this spec to `ready` approves the answer to every owner decision above, and the
following. Each is named in the FR that makes it. No existing test, golden or CI file
changes.

1. **tokio's `signal` feature** in `crates/scope-node/Cargo.toml` (FR-010; constitution
   Principle IV). It pulls in no new crate: `signal-hook-registry`, which it needs, is
   already in `Cargo.lock` through the `process` feature, and `Cargo.lock` does not
   change (run).
2. **A clean SIGTERM or SIGINT exits with a status** where today the process dies by the
   signal (FR-006). systemd counts death by SIGTERM as clean; under owner decision 2 an
   unconfirmed stop now shows the unit failed.
3. **New public API in the node's library**: `MountQueue::close`, the module
   `scope_node::shutdown` with `STOP_BOUND` and `Shutdown`, and
   `NodeService::shut_down` (FR-003, FR-004). No existing signature changes.
4. **rustix's `pty` feature in `[dev-dependencies]`**, if owner decision 4 is taken
   (FR-014; Principle IV). `Cargo.lock` does not change (run).
5. **`docs/voice/03-safety.md`'s row for a dying node is split in two** (FR-012): the row
   keeps a crash, a kill and a power cut, and a new row below it gives a clean stop.

## User Scenarios & Testing *(mandatory)*

Every scenario shares these definitions.

- **The gate command.** Every scenario runs in
  `cargo nextest run --workspace --all-targets --all-features --locked`, the `test`
  gate's first command.
- **The node rig** (US1-S1 to US1-S5, US1-S8) is built under
  `#[tokio::test(start_paused = true)]`:
  - a node clock that is a copy of the `Clock` in
    `crates/scope-node/tests/mount_reset.rs:23`;
  - one `Emulator` built with `EmulatorConfig::default()`: `Emulator::new` returns a
    stream, which is not used, and the `EmulatorHandle` called `handle`;
  - a `MountLink` whose opener returns `Ok(handle.reopen(clock.clone()))`;
  - 1 s passes, and `link.connected()` is true;
  - a `NodeService` built with `NodeService::with_controls_and_mount` from
    `SyntheticCamera::new(clock.clone())`, the clock, `"test-token"`,
    `Settings::default()`, a `NodeControls::default()` on which `stop_frames()` was
    called, so the capture loop's position reads stay off the wire, and `Some(link)`;
  - a spawned task that calls the service's `heartbeat` every 200 ms until the test
    ends, so the watchdog never stops the mount.
- **The powered-off rig** (US1-S8) is the node rig with `handle.power_off()` called
  before the link starts; after the 1 s, `link.connected()` is false.
- **A call** is the service's own handler called directly, as
  `crates/scope-node/tests/mount_reset.rs:168` calls `axis_state`. **Reading both axes**
  is `axis_state` for azimuth, then altitude, each answered, taking `moving`. **Refused**
  means the call fails with `ABORTED` and the message `mount command discarded by Stop`.
- **The shutdown** is `service.shut_down()` awaited. Its time is the paused Tokio time
  from the moment the scenario names to its answer, exact.
- **The exchanges after a mark** are `handle.exchanges()` from the index read at the
  mark, each a command and its reply, compared whole and in order. **Answered** is the
  reply `=\r`; **unanswered** is no reply. **A motion command** is one whose second
  character is `G`, `S`, `H`, `M`, `I` or `J`.
- **The stand-in mount** (US1-S6) is a `Mount` the test defines: two axes of 1,620,000
  counts, 3.4°/s and `can_goto`; `state` answers position 0, rate 0 and not moving;
  `goto` and `rate` record `goto <axis> <position>` or `rate <axis> <rate>`, with the
  axis as `Debug` writes it, and answer at once; `stop_all` and `stop_confirmed` never
  answer.
- **The binary** (US1-S9, US1-S10, US2) is the built `scope-node`,
  `env!("CARGO_BIN_EXE_scope-node")`, started with `--config` naming a file under
  `CARGO_TARGET_TMPDIR` that sets `listen` to a free `127.0.0.1` port, `token` to
  `'test-token'` and `adapter` to `'simulated'`, and for a mount a `[mount]` table with
  `device`; with standard input and output null, standard error piped and
  `kill_on_drop`, as `crates/scope-node/tests/mount_link.rs:160` starts one. It is waited
  on until a TCP connection to its port succeeds, for at most 10 s.
- **Sending SIGTERM** is running `kill -TERM <pid>`, the `kill` program, which must exit
  0; **sending SIGINT** is `kill -INT <pid>`. **Exits within t** means the child's exit,
  awaited for at most 5 s (15 s in US2-S4), comes less than t after the moment just
  before `kill` ran, on the wall clock. **Its last line** is the last line of its
  standard error, read after the exit.
- **The terminal rig** (US2) is a pseudo-terminal from `rustix::pty`: `openpt` with
  `RDWR`, `NOCTTY` and `CLOEXEC`, then `grantpt`, `unlockpt` and `ptsname`, whose name is
  the device. The test opens the device itself, with `RDWR`, `NOCTTY` and `CLOEXEC`,
  holds it open to the test's end and sets it raw with `termios::tcgetattr`,
  `make_raw` and `tcsetattr`. The master is made non-blocking with
  `rustix::io::ioctl_fionbio`. A spawned relay reads the master, writes what it read to
  an `Emulator` built with `EmulatorConfig::default()` on `SystemClock`, and writes the
  emulator's `discard_input()` back to the master. `handle` is that emulator's handle.
- **The board says an axis moves** when a stream from
  `handle.reopen(Arc::new(SystemClock))`, sent `:f<axis>\r`, answers five bytes ending
  in `\r` whose third is `1`.
- **Moving** (US2) means: a tonic `Channel` to the binary's address and a
  `ScopeNodeMountClient` over it with the `BearerToken` `test-token`; `describe_mount`
  read every 50 ms until it lists axes, for at most 10 s; a spawned task calling
  `heartbeat` on a `ScopeNodeClient` over the same channel every 250 ms until a call
  fails; `goto_axis(azimuth, 1000000)` and `rate_axis(altitude, 1.0)` answered OK; then
  1 s, after which the board says both axes move.
- **Oracles.** The emulator's own record of what the board was sent and answered, the
  board's own word on its axes, the stand-in's record, the process's exit status and
  standard error, the words in these scenarios and paused Tokio time. No expected value
  comes from the code under test: 6,000, 1,500, 1,200 and 8,000 ms, the four lines and
  the statuses are the scenarios' own. A time in paused Tokio time is exact; a wall-clock
  time is a bound with its tolerance stated.

### User Story 1 - On SIGTERM or SIGINT the node stops the mount before it exits (Priority: P1)

As the observer, I stop the node's service, restart it for a deploy, or shut the Pi
down, during a GoTo or with Track on, and the mount stops before the node goes.

**Why this priority**: Today every clean stop of the node leaves the mount moving with
nothing to stop it, and the Pi's power-loss service would do the same on every power
cut.

**Independent Test**: `crates/scope-node/tests/stop_before_exit.rs` (US1-S1 to US1-S8)
and `crates/scope-node/tests/exit_on_signal.rs` (US1-S9, US1-S10), both new. The `test`
gate's `cargo nextest run --workspace --all-targets --all-features --locked` runs them;
during development, `cargo nextest run -p scope-node --locked --test stop_before_exit`
and `cargo nextest run -p scope-node --locked --test exit_on_signal`.

**Acceptance Scenarios**:

1. **Given** the node rig with `goto_axis(azimuth, 1000000)` and
   `goto_axis(altitude, 300000)` answered OK and, 1 s later, both axes reading `moving`,
   **When** a committed test marks the exchanges, awaits the shutdown, calls
   `goto_axis(altitude, 50000)` and `rate_axis(azimuth, 0.5)`, waits 2 s and reads both
   axes, **Then** it asserts that the shutdown answers `Shutdown::Confirmed` after 0 ms;
   that the exchanges from the mark to that answer are exactly `:K1` and `:K2`, each
   answered; that both calls are refused; that both axes then read not `moving`; that no
   exchange after the mark is a motion command; and that `Shutdown::Confirmed` displays
   as `the mount confirmed the stop`.
2. **Given** the node rig with `rate_axis(azimuth, 1.0)` and `rate_axis(altitude, -1.0)`
   answered OK and, 1 s later, both axes reading `moving`, **When** a committed test does
   what US1-S1 does, **Then** it asserts what US1-S1 asserts.
3. **Given** the node rig with `rate_axis(azimuth, 1.0)` answered OK and
   `handle.drop_replies(b'K', 1, 5)`, **When** a committed test marks the exchanges,
   spawns a call of `stop_all`, 100 ms later awaits the shutdown and then awaits the
   spawned call, **Then** it asserts that the shutdown answers `Shutdown::Confirmed`
   1,500 ms after the mark; that the spawned call answers OK; and that the exchanges
   after the mark are exactly `:K1` five times unanswered, then `:K2`, `:L1`, `:K1` and
   `:K2`, each answered: the stop already running was carried through, and the
   shutdown's own followed.
4. **Given** the node rig with `rate_axis(azimuth, 1.0)` answered OK and then
   `handle.power_off()`, **When** a committed test marks the exchanges and awaits the
   shutdown, **Then** it asserts that the shutdown answers `Shutdown::NotConfirmed`
   after 6,000 ms; that the exchanges after the mark are exactly `:K1`, `:K2`, `:L1` and
   `:L2`, five times each in that order, all unanswered; and that
   `Shutdown::NotConfirmed` displays as `the mount did not confirm the stop`.
5. **Given** the node rig with `rate_axis(azimuth, 1.0)` answered OK, then
   `handle.power_off()` and three calls of `axis_state(azimuth)` answered, after which a
   fourth fails with the message `mount not connected`, **When** a committed test marks
   the exchanges and awaits the shutdown, **Then** it asserts that the shutdown answers
   `Shutdown::NotConfirmed` after 1,200 ms, and that the exchanges after the mark whose
   command starts `:K` or `:L` are exactly `:K1`, `:K2`, `:L1` and `:L2`, once each, all
   unanswered.
6. **Given** a `NodeService` built with `NodeService::with_mount` from
   `SyntheticCamera::new(clock.clone())`, the clock of the node rig, `"test-token"`,
   `Settings::default()` and the stand-in mount, the node rig's heartbeat task, 10 ms
   passed and `goto_axis(azimuth, 1000)` answered OK, **When** a committed test marks the
   time, spawns the shutdown, 100 ms later calls `goto_axis(altitude, 50000)` and
   `rate_axis(azimuth, 0.5)`, awaits the shutdown and then calls
   `goto_axis(altitude, 50000)` again, **Then** it asserts that all three calls are
   refused; that the shutdown answers `Shutdown::TimedOut` 8,000 ms after the mark; that
   the stand-in recorded exactly one move, `goto Azimuth 1000`; and that
   `Shutdown::TimedOut` displays as `the mount did not confirm the stop within 8 s`.
7. **Given** a `NodeService` built with `NodeService::new` from
   `SyntheticCamera::new(clock.clone())`, the clock of the node rig, `"test-token"` and
   `Settings::default()`, with no mount, **When** a committed test awaits the shutdown,
   **Then** it asserts that it answers `Shutdown::NoMount` after 0 ms, and that
   `Shutdown::NoMount` displays as `no mount to stop`.
8. **Given** the powered-off rig, **When** a committed test marks the exchanges, awaits
   the shutdown and waits 3 s, **Then** it asserts that the shutdown answers
   `Shutdown::NotConfirmed` after 0 ms, that the exchanges after the mark hold an `:e1`,
   the link still trying to connect, and that none of them starts `:K` or `:L`.
9. **Given** the binary with no `[mount]` table, **When** a committed test sends SIGTERM
   to one such node and SIGINT to another, **Then** it asserts that each exits within
   1 s with status 0 and that its last line is `scope-node: SIGTERM: no mount to stop`,
   or `scope-node: SIGINT: no mount to stop` for SIGINT.
10. **Given** the binary with a `[mount]` table whose `device` is a path under
    `CARGO_TARGET_TMPDIR` that does not exist, and 200 ms passed after it accepts
    connections, **When** a committed test sends SIGTERM to one such node and SIGINT to
    another, **Then** it asserts that each exits within 1 s with status 1, that its
    standard error holds `mount not connected`, and that its last line is
    `scope-node: SIGTERM: the mount did not confirm the stop`, or the same with
    `SIGINT` for SIGINT.

---

### User Story 2 - The built node stops a moving mount on a signal (Priority: P2)

As the owner, I trust the Pi's power-loss service because the shipped binary, given the
signal systemd sends, stops a moving mount over its serial device.

**Why this priority**: US1 holds the stop in process and the binary's signal path with
no mount to stop. Only this story runs both together, through the serial device the node
opens on the Pi. It adds tests and a dev-dependency feature, and no code (owner
decision 4).

**Independent Test**: `crates/scope-node/tests/exit_stops_the_mount.rs`, new. The `test`
gate's `cargo nextest run --workspace --all-targets --all-features --locked` runs it;
during development, `cargo nextest run -p scope-node --locked --test exit_stops_the_mount`.

**Acceptance Scenarios**:

1. **Given** the terminal rig and the binary with a `[mount]` table whose `device` is the
   terminal's device, moving, **When** a committed test marks the exchanges, sends
   SIGTERM, awaits the exit, waits 2 s and asks the board about both axes, **Then** it
   asserts that the board says neither axis moves; that the exchanges after the mark
   whose command starts `:K` or `:L` are exactly `:K1` and `:K2`, each answered; that no
   exchange after the mark is a motion command; and that the node exits within 1 s with
   status 0 and the last line `scope-node: SIGTERM: the mount confirmed the stop`.
2. **Given** the setup of US2-S1, **When** a committed test does what US2-S1 does with
   SIGINT, **Then** it asserts what US2-S1 asserts, with the last line
   `scope-node: SIGINT: the mount confirmed the stop`.
3. **Given** the setup of US2-S1 and `handle.drop_replies(b'K', 1, 5)`, **When** a
   committed test marks the exchanges, sends SIGTERM, 200 ms later sends SIGINT, 200 ms
   after that calls `goto_axis(altitude, 50000)` on its client, awaits the exit, waits
   2 s and asks the board about both axes, **Then** it asserts that the call fails; that
   the node exits with status 0 between 1,400 and 2,500 ms after SIGTERM was sent; that
   exactly one line of its standard error starts `scope-node: SIG`, and it is
   `scope-node: SIGTERM: the mount confirmed the stop`; that the exchanges after the
   mark whose command starts `:K` or `:L` begin with `:K1` five times, `:K2` and `:L1`,
   and that any after those start `:K`; that no exchange after the mark is a motion
   command; and that the board says neither axis moves.
4. **Given** the setup of US2-S1 and then `handle.power_off()`, **When** a committed test
   sends SIGTERM and awaits the exit for at most 15 s, **Then** it asserts that the node
   exits within 9 s with status 1, and that its last line starts
   `scope-node: SIGTERM: the mount did not confirm the stop`.

### Edge Cases

- **A status read in flight when the stop begins.** The binary's capture loop reads both
  axes for every frame (`crates/scope-node/src/capture.rs:50`). On a board that has just
  gone silent such a read holds the adapter for up to five tries of 300 ms before the
  stop's own commands begin, so the adapter's answer comes 6 s or up to 7.5 s after the
  signal, within the 8 s bound. US2-S4 runs it and asserts the start of the line, which
  both the adapter's answer and the bound's would give; US1-S4 holds the 6,000 ms in
  paused time with the capture loop stopped.
- **The watchdog's own stop.** During a stop longer than 1 s the console's heartbeats
  have stopped, so the watchdog runs `stop_all` too, behind the shutdown's stop, and its
  `:K1` can reach the wire just before the exit. Run in US2-S3, which allows any `:K`
  after the shutdown's seven stop commands. The watchdog is not changed.
- **A `StopAll` asked after the shutdown's stop** runs behind it and may be cut short by
  the exit (Clarifications, 017's FR-010). Read, not run.
- **A signal before `serve`.** With the `rpicam` adapter, camera discovery runs before
  `serve` (`crates/scope-node/src/main.rs:81`); a signal then ends the node at once, as
  today. No mount link exists yet, so no move can be under way. Read, not run.
- **The one-shot modes.** `--check-config`, `--probe-mount` and `--probe-camera` take no
  signal and are unchanged; `--probe-mount` sends no motion. Read, not run.
- **SIGHUP and SIGQUIT** keep their default action and end the node without a stop, as
  today; one exploratory run sent SIGHUP. systemd stops a service with SIGTERM
  (`KillSignal=`, man `systemd.kill(5)`), and sends SIGHUP only with `SendSIGHUP=`.
- **An imaging exposure in progress.** The node does nothing with it and exits. With the
  simulated imaging camera the probe saw the exposure's stream end at the exit with an
  `UNKNOWN` h2 error (one exploratory run, no mount). By reading, the console then ends
  the frame `failed` with `the exposure failed: ` and that error
  (`crates/finder/src/imaging/frames.rs:318`), as for a killed node. With INDI the
  driver's exposure runs to its end in its own process, and the cooler is left as the
  driver has it. A frame the node is saving with `save` is written under a `.part` name
  and renamed when whole (`crates/scope-node/src/imaging/save.rs:31`), so an exit
  mid-write leaves a `.part` file, as a kill does. Out of scope (§ Assumptions).
- **A crash, `kill -9` or a power cut** still ends the node with no stop: the first row
  of FR-012 says so.

## Requirements *(mandatory)*

### Functional Requirements

**The stop on a signal** (US1):

- **FR-001**: On SIGTERM or SIGINT while `serve` runs, `scope-node` SHALL end its
  serving with tonic's `serve_with_shutdown`, run `NodeService::shut_down`, write one
  line to standard error (FR-005) and exit with the status of FR-006.
- **FR-002**: `serve` SHALL take both signals before it starts the mount link and before
  the listener binds, so a node that accepts a connection handles a signal from then on.
- **FR-003**: `MountQueue` SHALL gain `pub async fn close(&self) -> bool`, which marks
  the queue closed for good and then runs `stop_all` and answers what it answers. A move
  submitted to a closed queue SHALL fail with `QueueError::Cancelled`, so `GotoAxis` and
  `RateAxis` answer `ABORTED` with `mount command discarded by Stop`. `stop_all`, its
  callers and every other answer SHALL be as today.
- **FR-004**: A public module `scope_node::shutdown` SHALL hold `STOP_BOUND`, the enum
  `Shutdown` with `NoMount`, `Confirmed`, `NotConfirmed` and `TimedOut`, and
  `NodeService::shut_down(&self) -> Shutdown`. With no mount it SHALL answer `NoMount` at
  once. Otherwise it SHALL run `close` for at most `STOP_BOUND`, and answer `Confirmed`
  or `NotConfirmed` by its answer, or `TimedOut` when the bound runs out first.
- **FR-005**: The line SHALL be `scope-node: <signal>: <outcome>`, `<signal>` being
  `SIGTERM` or `SIGINT` and `<outcome>` the `Display` of `Shutdown`: `no mount to stop`,
  `the mount confirmed the stop`, `the mount did not confirm the stop`, and
  `the mount did not confirm the stop within 8 s`.
- **FR-006**: The node SHALL exit 0 after `NoMount` or `Confirmed`, and 1 after
  `NotConfirmed` or `TimedOut` (owner decision 2).
- **FR-007**: `STOP_BOUND` SHALL be 8 s (owner decision 1).
- **FR-008**: After the first SIGTERM or SIGINT, a later one SHALL change nothing: no
  second line, and the stop runs to its answer or the bound (owner decision 3).
- **FR-009**: The node SHALL exit once the line is written, without waiting for open
  streams, such as the console's frames, or for blocking tasks.
- **FR-010**: `crates/scope-node/Cargo.toml` SHALL add `signal` to tokio's features, and
  `Cargo.lock` SHALL NOT change (owner approval 1).
- **FR-011**: `crates/scope-node/README.md` § Config and systemd SHALL give the unit
  `TimeoutStopSec=10`, after `RestartSec=1`, and SHALL say, in a paragraph after the
  sentence on the `dialout` group, what the node does on SIGTERM or SIGINT, the four lines, the exit
  statuses, that a second signal changes nothing, what `TimeoutStopSec=10` leaves, and
  that a crash, `kill -9` or a power cut still ends the node without a stop (plan.md
  § Interfaces).
- **FR-012**: `docs/voice/03-safety.md`'s row "Scope node or Pi dies"
  (`docs/voice/03-safety.md:177`) SHALL name a crash, a kill or a power cut and keep its
  other cells, and a row SHALL follow it for a clean stop: the node stops the axes before
  it exits, waits up to 8 s, and its last line says whether the mount confirmed
  (owner approval 5; plan.md § Interfaces).
- **FR-013**: Nothing else SHALL change: the scope link's protocol, every file under
  `crates/finder`, `crates/scope-node/src/service.rs`, the watchdog, the `Mount` trait,
  everything under `crates/scope-node/src/mount/skywatcher/` and
  `crates/scope-node/src/imaging/`, `StopAll`'s answer, and the one-shot modes.

**The binary's own test** (US2):

- **FR-014**: `crates/scope-node/Cargo.toml`'s `[dev-dependencies]` SHALL gain rustix
  with the `pty` feature, and `Cargo.lock` SHALL NOT change (owner decision 4, owner
  approval 4). US2 SHALL write no file under `crates/scope-node/src`.

### Key Entities

- **The shutdown**: the node's one stop on SIGTERM or SIGINT, bounded by `STOP_BOUND`.
- **A closed queue**: a mount queue that refuses every move for good.
- **`Shutdown`**: how the stop ended, `NoMount`, `Confirmed`, `NotConfirmed` or
  `TimedOut`, and the words of the line.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario has a committed test that asserts it, and the
  `lint` and `test` gates pass.
- **SC-002**: No existing test, golden, `scripts/ci/` file, `.github/` file,
  `CODEOWNERS` entry, `ergane.yaml` or `playwright.config.ts` is edited. Every existing
  test passes unchanged (plan.md § Shapes that existing tests pin).
- **SC-003**: Given SIGTERM or SIGINT with a mount moving, the node sends the stop to
  both axes before it exits and leaves them at rest (US1-S1, US1-S2, US2-S1 to US2-S3),
  where today it sends nothing and both axes keep moving.
- **SC-004**: The node exits within 1 s when there is nothing to wait for (US1-S9,
  US1-S10, US2-S1, US2-S2) and within 9 s when the mount answers nothing (US2-S4), and
  no move starts once the stop has begun (US1-S1, US1-S6, US2-S3).

## Assumptions

- **Earlier features are in place.** Every spec in `depends_on_landed` has landed.
  Every citation here was read at 7164e87f.
- **The emulator stands for the board** for the stop's commands and their replies, as in
  017's and 024's tests. Whether a real board takes `:K` within the same exchange is the
  field check's.
- **The pseudo-terminal stands for the serial device.** The node sets it raw and its
  baud as it does a serial device; the baud has no effect on it.
- **How long the Pi's UPS holds the Pi is not known.** The bound is paid once per
  shutdown and only when the mount is silent.
- **Out of scope:**
  - an imaging exposure in progress at the signal: aborting it, saving a stack, and the
    cooler (Edge Cases);
  - telling the console that the node stopped the mount before it went: the console
    sees a lost link either way;
  - SIGHUP and SIGQUIT, and the one-shot modes;
  - a crash, `kill -9` or a power cut;
  - any change to the watchdog, the Sky-Watcher adapter or the scope link's protocol.
- **Outside the factory:** the Pi's unit, the power-loss service and the field check in
  plan.md § Outside the factory, by the owner. A new field kit's Pi binary is needed
  once 044 lands.
- **The factory's gates and CI:** the gates are `lint` and `test`, copied in plan.md
  § Verification. CI's other jobs run only in the merge queue, so no scenario's Then
  clause rests on them.

## Work Graph

```yaml
US1:
  depends_on: []
  implements: [FR-001, FR-002, FR-003, FR-004, FR-005, FR-006, FR-007, FR-008, FR-009, FR-010, FR-011, FR-012, FR-013]
US2:
  depends_on: []
  depends_on_merged: [US1]
  implements: [FR-014]
```

US2 merges after US1: it tests US1's binary, and both write `crates/scope-node/Cargo.toml`,
US1 its tokio line and US2 a line under `[dev-dependencies]`.
