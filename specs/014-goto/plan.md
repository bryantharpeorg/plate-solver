# Implementation Plan: GoTo

**Branch**: `014-goto` | **Date**: 2026-10-28, refined 2026-10-31 | **Spec**: [spec.md](spec.md)

This plan records every design decision for the feature. A story that would change one
escalates rather than re-deciding it.

## Summary

The work runs from the node to the page:

1. Give the scope node a mount port, with a simulated mount and a Sky-Watcher adapter
   behind it.
2. Make Stop win on the node, and add a heartbeat watchdog.
3. Put axis positions on star frames, and carry the mount over the scope link.
4. In the console, add apparent altitude.
5. Add a pointing model fitted from syncs, and a closed-loop GoTo that plans through the
   model and the Sun guard and corrects to 1′.
6. Add the one-tap second sync and the model routes.
7. Add GoTo and Stop on the page, and Setup's mount and model section.

Everything is proven on the simulator in paused virtual time, and the adapter against a
byte-level emulator. `crates/math-core` is untouched, and no existing test changes.

## Technical Context

**Language/Version**: Rust, MSRV 1.96 (`rust-version`), stable in CI. TypeScript with
React 19 for the page.

**Primary Dependencies**:
- already present after 013: tokio, tonic and prost in `crates/scope-link`, axum in
  `finder`, and `rustix` with its `time` feature in `crates/scope-node`;
- the page: Vite, Vitest and Playwright.

The serial code is termios through `rustix`'s `termios` and `fs` features (owner
approval 1). `rustix` is already locked, so no crate is added. The model's fit is written in-crate, with no linear-algebra dependency. No npm
package is added.

**Storage**: the session's state file from 012. It gains the night's syncs, zero points
and base tilt, kept until the next local noon, and the fixed terms, kept until reset.

**Testing**:
- `cargo test --workspace --locked` runs:
  - unit tests in-module;
  - integration tests in new files under `crates/scope-node/tests/`,
    `crates/scope-link/tests/`, `crates/simulator/tests/` and `crates/finder/tests/`.
- Simulator tests use `#[tokio::test(start_paused = true)]` with 013's `VirtualClock` and
  in-process `tokio::io::duplex` link (`scope_link::memory`). Node code reads time only
  through 013's `NodeClock` (§ Plan traps, trap 12).
- Vitest and Playwright (013's `simulate` project) run in `crates/finder/web`.

**Target Platform**:
- the scope node on a Raspberry Pi 3B (`aarch64-unknown-linux-musl`), with a USB–TTL
  cable in the mount's hand-controller jack;
- the console on desktop Linux, WSL and the Pi;
- the page on Android phones and laptops.

**Constraints**:
- **Rust and the page:** `RUSTFLAGS="-D warnings"`, `--all-features` clippy and rustdoc,
  and the 80% coverage floor (Rust only). `web/dist` is committed and must be the build of
  its source, and night mode is red only.
- **Safety:** Stop needs no token and wins everywhere, and the Sun guard cannot be
  overridden.

**Scale/Scope**: one mount with two axes, a few hundred syncs a night, and at most
8 model parameters.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | The solver and `crates/math-core` are untouched. The simulator still uses math-core's camera model, unchanged. New numerics are checked against Meeus (refraction), synthetic TPoint terms recovered within 1″, and the simulator's known truth. |
| II. Never weaken a gate | No existing test, golden, `scripts/ci/`, `ci.yml` or `CODEOWNERS` is edited. New tests go in new files. The proto changes only add. |
| III. Tests first, independent expectations | Every scenario names its oracle: Meeus ch. 16, the motor-controller command set's byte strings, synthetic model terms, or the simulator's true eyepiece direction sampled in virtual time. |
| IV. Critical paths | One dependency change, owner-approved: two features of the `rustix` already locked. `scope_node.v1` gains calls and one service only. No `unsafe` is planned: `rustix`'s termios API is safe. |
| V. Standalone Rust, honest licensing | `rustix` is Apache-2.0 / MIT, in `deny.toml`'s allow-list. No reference source is copied. |

## Project Structure

```text
Cargo.lock                              # US3: only if rustix's new features add entries
crates/scope-node/
├── Cargo.toml                          # US3: rustix's termios and fs (approval 1)
├── src/lib.rs                          # US1: `pub mod mount;`
├── src/mount/mod.rs                    # US1: the Mount trait, AxisInfo, AxisState;
│                                       # US2, US3: their modules
├── src/mount/queue.rs                  # US2: the command queue, epoch counter, Stop bypass
├── src/mount/watchdog.rs               # US2: the 1 s heartbeat watchdog, 50 ms tick
├── src/mount/skywatcher/mod.rs         # US3: the adapter's modules
├── src/mount/skywatcher/codec.rs       # US3: 24-bit hex, framing, replies
├── src/mount/skywatcher/transport.rs   # US3: echo skip, 300 ms timeout, 5 tries
├── src/mount/skywatcher/protocol.rs    # US3: connect, rates, goto, reverse, !2, stop
├── src/mount/skywatcher/emulator.rs    # US3: the byte-level emulator of the mount
├── src/mount/skywatcher/serial.rs      # US3: opening the device, the only untested shell
├── src/capture.rs (013's capture loop) # US2: axis positions at exposure start and end
├── src/service.rs (013's NodeService)  # US2: the mount calls and ScopeNodeStop
├── src/config.rs (013's config)        # US3: the optional [mount] table
├── src/main.rs (013's)                 # US3: --probe-mount DEVICE; the mount from config
└── tests/{stop_and_watchdog,frame_positions,skywatcher,mount_config}.rs   # US2, US3
crates/scope-link/
├── proto/scope_node/v1/scope_node.proto   # US2: mount calls, ScopeNodeStop (adds only)
├── src/memory.rs (013's)                  # US2: serve_routes_on, both services on a duplex
└── tests/mount_calls.rs                   # US2
crates/simulator/
├── src/lib.rs                          # US1: `pub mod mount;`
├── src/mount.rs                        # US1: the simulated mount behind the port
├── src/controls.rs (013's push)        # US1: a push goes through the mount's hook
├── src/node.rs (013's SimNode)         # US2: SimNode serves the mount
└── tests/mount.rs                      # US1
crates/finder/
├── src/link.rs (013's link client)     # US2: the mount calls
├── src/ephemeris.rs                    # US4: apparent altitude (Saemundsson, Bennett)
├── src/lib.rs                          # US5, US6: `pub mod` lines only
├── src/model.rs                        # US5 (new): syncs, fit, epochs, fixed terms, inverse
├── src/session.rs                      # US5: the night's model state and the fixed terms
├── src/live.rs (013's live loop)       # US5: hand solved star frames to the model as syncs
├── src/goto.rs                         # US6 (new): planner and state machine; US7: second sync
├── src/api.rs                          # US5: session fields; US6: /api/goto, /api/stop;
│                                       # US7: /api/model/*
├── README.md                           # US6, US7: the new routes
└── tests/{model_sim,goto_sim,goto_api,second_sync_sim,model_api,night_sim}.rs
crates/finder/web/
├── src/api.ts                                # US8: the session's goto, model and mount
├── src/App.tsx                               # US8: hands Stop's state to TopBar
├── src/components/TopBar.tsx                 # US8: hosts Stop, on the console and Setup
├── src/components/GuidanceBlock.tsx          # US8: hosts GoTo
├── src/components/SetupPage.tsx              # US8: hosts the mount and model section
├── src/components/GoToControl.tsx (+ test)   # US8
├── src/components/StopButton.tsx (+ test)    # US8
├── src/components/MountSection.tsx (+ test)  # US8
├── src/test/fixtures/session-*.json          # US8: sessions recorded from the simulator
├── dist/                                     # US8: rebuilt
├── playwright.config.ts                      # US9: three lines (owner approval 3)
└── e2e/goto.spec.ts                          # US9 (013's simulate project)
DESIGN.md, .impeccable/design.json            # US8
```

Paths marked "013's" are modules 013 creates. On `main` at 2a2f9d2, `crates/scope-link`
and 013's page work (US8, US9) have landed; `crates/scope-node` (US2, in the merge queue
as #135, then US4), `crates/simulator` (US3, US4), `link.rs` (US5), `live.rs` (US6),
`--simulate` (US7) and the `simulate` project (US10) have not. A story reads each before
editing it and keeps to the names 013 actually landed, and the operator re-reads this
list against 013 at the flip (§ Prerequisites, item 1). `TopBar` already serves the
console and Setup alike: `App.tsx` renders it above both.

## Interfaces

**The mount port** lives in `crates/scope-node/src/mount/mod.rs`:

```rust
trait Mount {
    fn axes(&self) -> [AxisInfo; 2];                    // counts per revolution, top rate, can_goto
    async fn state(&self, axis: Axis) -> AxisState;     // position (counts), rate, moving
    async fn goto(&self, axis: Axis, position: i32);    // absolute, in counts
    async fn rate(&self, axis: Axis, deg_per_s: f64);   // signed
    async fn stop_all(&self);
}
```

- Positions are in the axis's own counts, and the console converts them with counts per
  revolution.
- Rates are signed degrees per second, and each adapter turns them into its own units.

**`scope_node.v1` gains these calls** (adds only):
- in `service ScopeNode`, behind 013's `TokenInterceptor` like every call there:
  - `DescribeMount` → two `AxisInfo`;
  - `AxisState(axis)`;
  - `GotoAxis(axis, position)`;
  - `RateAxis(axis, deg_per_s)`;
- a new `service ScopeNodeStop { rpc StopAll(StopAllRequest) returns (StopAllReply); }`,
  in the same file and package, served beside `ScopeNode` with no interceptor (§ Decisions,
  "Stop all is its own service").

`StarFrame` gains optional `axis_start` and `axis_end` (both axes' counts, read at
exposure start and end) on the next free field numbers: 11 and 12 on `main` at 2a2f9d2,
where `preview_jpeg` is 10. `ScopeNode`'s comment, "Every call requires the session's
bearer token", stays true.

**`scope-link`'s Rust surface** gains `memory::serve_routes_on(server_io, routes)`, which
serves a `tonic::service::Routes` (both services) on one duplex. 013's `serve_on` keeps
its signature.

**`GET /api/session` gains** three fields, each left out of the body until it has a
value, as 013's are (§ Plan traps, trap 4):
- `goto?: {state, correction, error_arcsec, reason}`, present while a GoTo or the second
  sync runs and after it ends, until the next one starts;
- `model?: {syncs, epoch, ia, ie, base_tilt: {deg, toward_deg}?, fixed?, rms_arcsec,
  needs}`, where `needs` is the count of further syncs the fixed terms want. Absent while
  the night has no sync and no fixed terms are kept;
- `mount?: {connected, board, firmware, moving}`. Absent while the node reports no mount.

**The routes:**

| Route | Body | Same-origin check | Answers |
|---|---|---|---|
| `POST /api/goto` | none | 012's | 200 when started; 409 with a JSON `error` naming the reason when the target is not pointable, there is no mount, there is no sync, the Sun is up, or the approach would go below the minimum altitude |
| `POST /api/stop` | none | none; any origin is accepted | 200 always, stopping both axes |
| `POST /api/model/second-sync` | none | 012's | starts the second sync |
| `POST /api/model/reset` | `{"what": "night" \| "fixed"}` | 012's | clears that part of the model |

## Shapes that existing tests pin

These are not changed by any story:
- **012's and 013's surfaces:** every test they landed passes unchanged. That covers
  `AppState::new`, `AppState::shared` and `router`, the same-origin check (with
  `/api/stop` added as its only exempt route), the session's existing fields, and 013's
  link, frame and simulator APIs.
- **A fresh session's whole body:** `crates/finder/tests/session_api.rs` compares a fresh
  read's whole body with 012's fields, so `goto`, `model` and `mount` are left out until
  they have a value.
- **013's link and node:** `TokenInterceptor`, `BearerToken`, `tokens_match` and
  `memory::{channel_pair, serve_on}` keep their signatures, and `ScopeNode` keeps the
  interceptor on every call. 013's config keys, `scope-node --check-config` and
  `crates/scope-node/tests/cli.rs` are unchanged; the `[mount]` table is new and optional.
- **013's end-to-end tests:** `e2e/simulate.spec.ts`, `e2e/live-off.spec.ts` and
  `crates/finder/tests/e2e_live.rs` pass unchanged.
- **Geometry:** `sky::alt_az` stays geometric, as `alt_az_matches_meeus_example_13b`
  pins. Refraction lives only in `finder::ephemeris`.
- **The proto:** `scope_node.v1`'s existing calls, messages and field numbers are kept.
  New ones only add, and proto comments hold no `[...]`, because rustdoc denies warnings.
- **Playwright:** the servers and projects are unchanged, except that the `simulate`
  project's match and the `default` project's ignore gain `e2e/goto.spec.ts`, and the
  `simulate` project gains `workers: 1` (owner approval 3; § Plan traps, trap 15).
- **Layering:** `crates/scope-node` still depends on `star-detection` and `scope-link`
  only, so the Pi never links the solver.

## Decisions and the alternatives rejected

**A port small enough that another mount is an adapter.** Positions are in counts and
rates in signed degrees per second. Nothing Sky-Watcher-specific crosses the port.

**The Sky-Watcher adapter, in three layers.**
- **Codec:** pure functions, tested exhaustively.
  - Values are 24-bit little-endian hex, with the 0x800000 offset on positions.
  - Commands are framed as `:` + command + axis + data + CR.
  - Replies are `=data` or `!code`.
- **Transport:** a byte stream, either the serial port or the in-memory emulator.
  - A command is written, and its echo, if any, is skipped, because a TTL cable on a
    single-wire bus hears itself.
  - The reply is read with a 300 ms timeout. Queries, and other commands safe to repeat,
    get up to 5 tries, as INDI does.
- **Protocol:**
  - **Connect:** probe `:e1` at 9600, then at 115200. Then send `:F3`, read `:a`, `:b`,
    `:g` and `:q` (auxiliary-encoder support), and send `:W` (auxiliary encoders off).
  - **Firmware:** below 3.39 is accepted with a warning that smooth variable-rate
    tracking may not work.
  - **Rates:** set as a step period, `:I` = timer frequency × 360 / (°/s) / counts per
    revolution, times the high-speed ratio in fast mode. Fast mode (`:G` codes 0 and 3) is
    used above 128× sidereal, as INDI does.
  - **Goto:** `:G` (goto mode), `:S` (absolute target), `:J`. If `:S` is refused, the
    adapter falls back to `:H` (relative) and `:M` (brake point).
  - **Reversing:** `:K`, then `:f` polled until stopped, then the new direction.
  - **`!2` (motor not stopped):** the same stop-and-wait, then the command once more.
  - **Stop:** `:K` on both axes, a decelerating stop, gentler than `:L` on a 16-inch tube.
    `:L` is sent only if `:K` is not acknowledged.

**Serial: termios through `rustix` (owner approval 1).**
- `rustix`'s `termios` and `fs` features need no new crate: 013 locked `rustix` 1.1.4 in
  `crates/scope-node` for the node's clock. The device is opened with `O_NOCTTY`, put in
  raw mode, and set to 9600 or 115200 baud, in a few lines of safe calls.
- `serialport` (`default-features = false`) was the alternative. It is a new crate, and
  its musl build was unproven. The owner chose `rustix` on 2026-10-31.

The adapter's interface sits behind `serial.rs`, and opening the device is the only
untested shell. The maintainer's cable is an FT232RL USB-to-RJ45 converter, which the
Pi's kernel drives as `/dev/ttyUSB*`; on the Pi, the `[mount]` table names it by its
`/dev/serial/by-id/` path, since `ttyUSB` numbers can change between boots.

**Stop all is its own service.** 013's `TokenInterceptor` is a tonic server interceptor,
installed on the generated `ScopeNodeServer`, and it sees only a request's metadata
(`Request<()>`), never which call it is. tonic sets `GrpcMethod` only in generated clients,
and it does not cross the wire (tonic 0.14.6). So one interceptor cannot exempt one call.
`StopAll` is the only call of `service ScopeNodeStop`, served beside `ScopeNode` on the
same server with no interceptor. The node's TCP server (013's `service.rs`) and 013's
`SimNode` add both services to one router, and `memory::serve_routes_on` serves both on
one duplex for tests. A stop needs no token, and every other call still does.

**The node's mount comes from its config.** 013's config file gains an optional `[mount]`
table with one key, `device`, the serial device. With it, `main.rs` opens the device and
drives the Sky-Watcher adapter behind the mount port; without it, the node has no mount,
and its frames carry no axis positions. 013's config refuses unknown keys, so US3 adds
the table to `config.rs`, and `--check-config` prints it. The node has no simulated mount
of its own: the simulated mount lives in `crates/simulator`, which depends on
`crates/scope-node`, not the other way.

**The byte-level emulator.** It implements the command set over an axis model the same as
the simulator's. It can be told to:
- answer only at 115200;
- echo like a single wire;
- inject `!2`;
- refuse `:S`;
- withhold `:K`'s acknowledgement;
- report old firmware.

The adapter's tests, and the oracle byte strings, run against it.

**`scope-node --probe-mount DEVICE`.** It runs the first-test sequence (`:e1`, `:a1`,
`:b1`, `:g1`, `:q1010000`, `:j1`) and prints the results, for bring-up. The sequence is a
function over any byte stream, so it is tested on the emulator.

**Stop wins on the node through an epoch counter, not a priority queue.** Mount commands
go through one queue to the adapter's task. Stop bypasses the queue: it raises an epoch
counter, clears the queue, and commands the stop at once. A command carrying an older
epoch is dropped when it reaches the adapter. That gives "carried out before any command
that arrived with or after it" without a race between two queues.

**The watchdog runs on the node's monotonic clock.**
- It reads 013's `NodeClock`, and ticks with `tokio::time`. It never calls `Instant::now()`
  or the system clock itself, so the simulator's `VirtualClock` drives it in paused-time
  tests (§ Plan traps, trap 12).
- A 50 ms tick compares now with the last heartbeat.
- At 1 s, it calls `stop_all` and marks the link lost.
- A move then needs a fresh request after the link returns. Nothing resumes on its own.

**The pointing model: rigorous where the terms are large, TPoint where they are small.**
The encoder's zero points are arbitrary, so IA can be anything. The forward model (axis
positions → apparent alt/az) runs in four steps:
1. axis angles from counts;
2. IA and IE as exact offsets;
3. the base tilt (AN, AW) as an exact rotation of the mount's frame;
4. CA, NPAE and TF as TPoint's small-angle terms (§7.2).

The finder camera's rotation on the tube adds the solve's roll as a third equation per
sync, through the parallactic angle (Meeus ch. 14).

**The fit is in-crate Levenberg–Marquardt, not a linear-algebra crate.** It works on the
residuals (ΔA·cos E, ΔE, Δroll), weighted by each solve's RMS. There are at most 8
parameters, and the normal equations are solved by Cholesky, so no dependency is needed.

**The inverse (target → axis positions)** is Newton's method on the forward model from
the uncorrected guess. It converges in 3–4 steps.

**Epochs and nights.**
- Each night has its own base tilt.
- Each zero-point epoch has its own IA and IE.
- The fixed terms are shared and kept.

The fixed terms are fitted from one night's syncs once that night has at least 20 spread
over 180° × 30°, and are refined on later nights that qualify. Syncs are not archived
across nights.

**Syncs.**
- Only star frames from the scope node carry axis positions. A frame uploaded by hand
  updates the pointing but is never a sync.
- A frame counts if each axis's rate during the exposure is at most 0.2°/s. That rejects
  slews (3.4°/s) and keeps tracking, whose azimuth rate reaches about 0.1°/s at the edge
  of 015's 2° zenith circle and is far lower elsewhere.
- The axis positions are interpolated linearly between the frame's start and end
  readings, to mid-exposure.
- Thinning keeps the newest sync when another lies within 2° and less than 2 minutes
  older.

**Slips: 0.5° after the base tilt is fitted, 2.5° before.** Before the base tilt is
fitted, the prediction is off by up to twice the tilt (2° for 1°). A 0.5° threshold would
make the second sync itself open an epoch, and with its own IA and IE the base tilt could
never be fitted. A mount power-cycled mid-night shows up the same way, at its first sync.

**The night's state ends at local noon.** It is the Sun's next upper transit at the site
after the night's first sync, from 012's rise/transit/set times. It is kept across
restarts until then.

**The cable's wind is the telescope's azimuth, not the counts.** It is unwrapped
continuously from the night's first sync, across syncs and epochs. Raw axis counts would
be reset by a power cycle.

**GoTo is a state machine in `finder::goto`:**
`Plan → Slew → Settle → Solve → (Done | Correct → Slew …) | Failed | Halted`.

**Plan.**
1. Take the target's place at the estimated arrival time, using two iterations of the
   slew-time estimate. Add refraction, and invert the model.
2. Choose the azimuth axis value among the ±360° candidates. It must keep the unwrapped
   azimuth within one revolution of the night's first sync, and be the nearest that does.
   That unwinds the cable when needed.
3. Make each axis's last approach from below: overshoot by 0.5°, more than any backlash a
   mount like this has (the simulator's is 0.3°), then approach upward. When the altitude
   overshoot would go under the minimum altitude, the GoTo is refused with that reason.

**The Sun guard on motion.**
- The path is simulated from the axes' rates and acceleration, at 0.5° steps.
- If any point comes within 15° of the Sun, the planner tries three alternatives in turn:
  altitude first; azimuth first; a dogleg through
  `max(current, target, Sun's altitude + 20°)`.
- If none keeps clear, the slew is refused.
- The Sun's altitude is also checked on a 50 ms tick during motion. Reaching −0.83°
  halts.

**Settle and Solve.** Wait until both axes report stopped, plus 0.5 s. The first frame
whose exposure starts after that is solved and taken as a sync.

**Correct.** The new sync refines IA and IE. If the error is 1′ or more, re-plan a small
move, still approaching from below. After 5 corrections the GoTo fails with the last
error.

**Halted.** Stop, a lost link (the console's own 1 s rule, matching the node's watchdog)
or the Sun guard. Each sends `stop_all` and records the reason.

**The one-tap second sync.** It picks the pointable spot nearest the current pointing
whose azimuth is at least 90° from every sync of the night and that lies outside the Sun
guard. It plans and slews to it as a GoTo with no corrections, then solves and takes the
sync.

**The page.**
- **GoTo:** the guidance block gains it, as a primary button when the target is pointable,
  with its progress and result.
- **Stop:** it sits in the top bar of every page, the console and Setup alike, whenever
  the session says the mount may be moving. It is never disabled, never under a sheet, and
  at least 44 px tall.
- **Setup:** it gains the mount and model section.

**The console never moves the mount unasked.** Only GoTo and the second sync move it here
(015 adds Track), so someone at the eyepiece is never surprised. Stop is always on screen
while the mount moves.

**Proof in virtual time, not on hardware.** Every GoTo, sync and model behaviour is a
Rust integration test that runs the simulator and the console in one paused-time tokio
runtime, over the in-process link. A night of slews takes milliseconds of CPU, and the
coverage job counts it. Playwright's `simulate` project adds one GoTo-and-Stop flow.

**Rejected alternatives:**
- the hand controller's protocol and SynScan App;
- the mount's own star alignment and GoTo (ADR-0001);
- `:L` as the normal stop, which is abrupt on a 16-inch tube;
- a linear-algebra crate for an 8-parameter fit;
- archiving syncs across nights;
- measuring the cable's wind in raw counts.

## User Story 1 — The mount port and the simulated mount

- **The port:** `crates/scope-node/src/mount/mod.rs` holds the `Mount` trait and its
  types, with unit tests on a fake axis.
- **The simulated mount:** `crates/simulator/src/mount.rs` implements the trait over two
  simulated axes in virtual time:
  - a trapezoidal velocity profile (3°/s², 3.4°/s top rate);
  - backlash as a 0.3° dead band between the axis position and the true angle;
  - a base tilt (1° by default) as a rotation of the mount's frame;
  - zero-point offsets.
- **Test hooks:** push by hand (true direction only, counts unchanged), slip per move,
  power-cycle (counts reset to the power-on zero), and the true eyepiece direction.
- **013's simulated telescope** follows the mount's true direction while a mount is
  connected, and 013's push by hand (`Simulator::push_to` and `push_by` in
  `crates/simulator/src/controls.rs`, which `POST /api/sim/telescope` calls) goes through
  the hook.

## User Story 2 — Stop, the watchdog, frames and the link

- **`mount/queue.rs`:** the single queue to the adapter's task, with the epoch counter
  and the Stop bypass.
- **`mount/watchdog.rs`:** the 50 ms `tokio::time` tick, reading `NodeClock`, which calls
  `stop_all` at 1 s and marks the link lost.
- **The capture loop:** 013's `capture.rs` reads both axes' positions at exposure start
  and end, onto `StarFrame`.
- **`scope_node.v1`:** gains the mount calls in `ScopeNode`, behind 013's interceptor, and
  `ScopeNodeStop` with `StopAll`, served with none. 013's `service.rs` serves both, over
  the queue, and 013's `SimNode` (`crates/simulator/src/node.rs`) serves them over the
  simulated mount. `scope_link::memory` gains `serve_routes_on`.
- **The console's link client:** 013's `crates/finder/src/link.rs` gains the matching
  calls.

## User Story 3 — The Sky-Watcher adapter

- **The adapter:** codec, transport, protocol and emulator under
  `crates/scope-node/src/mount/skywatcher/`, as above.
- **The emulator:** implements every command the adapter sends, over an axis model.
- **`serial.rs`:** opens the device with termios through `rustix` (approval 1), and is
  the only untested shell.
- **`--probe-mount`:** added in `src/main.rs`.
- **The node's `[mount]` table:** added to 013's `config.rs`; `main.rs` builds the adapter
  on that device, or no mount without the table (§ Decisions). A new
  `tests/mount_config.rs` runs `--check-config` on the three cases (US3-S16).
- **The dependency:** `rustix`'s `termios` and `fs` features go in
  `crates/scope-node/Cargo.toml`, with
  `Cargo.lock` updated by cargo (§ Plan traps, trap 13). `cargo-deny` and the MSRV build
  check it in the queue.

## User Story 4 — Apparent altitude

`finder::ephemeris` gains the two directions of refraction for the standard atmosphere
(1010 hPa, 10 °C):
- `apparent_from_true(h)`, by Saemundsson (Meeus 16.4);
- `true_from_apparent(h0)`, by Bennett (Meeus 16.3).

`sky::alt_az` is not touched.

## User Story 5 — The pointing model

**`finder::model`:**
- sync intake (the rate rule, interpolation to mid-exposure, thinning);
- the forward model (four steps);
- the in-crate Levenberg–Marquardt fit with Cholesky;
- the Newton inverse;
- epochs and slips;
- the fixed terms and their spread rule;
- `needs`, the count of further syncs the fixed terms want.

**The live loop** hands each solved star frame with axis positions to the model.
Refraction is removed first.

**`finder::session`** keeps the night's model (syncs, epochs and base tilt) until the next
local noon after the first sync, across restarts, and keeps the fixed terms until reset.

## User Story 6 — GoTo

**`finder::goto`:**
- the planner: arrival-time iteration, refraction, the model inverse, the
  one-revolution cable rule, the approach from below, the minimum altitude, the Sun-up
  refusal, and the path simulation with its three re-plans;
- the state machine, with settle, solve, at most 5 corrections, and the halts.

**`src/api.rs`:**
- `POST /api/goto`, behind 012's same-origin check, with its 409 reasons;
- `POST /api/stop`, which skips the check and needs no body;
- `goto` progress in `GET /api/session`.

**Documentation:** the routes are documented in the crate docs and README.

## User Story 7 — The second sync and the model routes

**The second sync:**
- the spot choice (pointable, at least 90° in azimuth from every sync of the night,
  outside the Sun guard);
- a GoTo with no corrections;
- then the sync.

**The model routes:** `POST /api/model/second-sync` and `POST /api/model/reset`, behind
012's same-origin check, documented in the crate docs and README.

## User Story 8 — The page

**New components:**
- `GoToControl` in the guidance block, enabled only when a GoTo would be accepted,
  otherwise showing the reason, with progress and result;
- `StopButton` in `TopBar` on every page whenever `mount.moving` or a GoTo or second
  sync is under way: never disabled, never inside a sheet, at least 44 px;
- `MountSection` on `SetupPage`: link and board, "base tilt X° toward Y°", RMS, sync
  count, the second sync and the two resets.

**Fixtures:** sessions recorded from the simulator (a GoTo to M42, two syncs) go in
`src/test/fixtures/`.

**Design records:** `DESIGN.md` and `.impeccable/design.json` gain the new parts as
built.

**Last step:** rebuild `web/dist`.

## User Story 9 — End to end

**`crates/finder/tests/night_sim.rs`** plays the night in paused time, in this order:
1. the first sync, then the second sync;
2. five GoTos;
3. a hand push, then a GoTo;
4. a Stop mid-slew;
5. a dropped link mid-slew.

**`e2e/goto.spec.ts`**, in 013's `simulate` project, covers:
- a GoTo from the page;
- Stop from the page;
- the second sync from Setup;
- Stop's size and that nothing covers it;
- the night pixel check with GoTo running.

The `simulate` project gains `workers: 1`, so this file and 013's `simulate.spec.ts`
never run at once against the one simulated telescope (owner approval 3; § Plan traps,
trap 15).

## Verification (every story, before proposing)

- **The factory gates** (`ergane.yaml`, as of 3fc6a24):
  - `lint` = `cargo fmt --all --check`,
    `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`,
    `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features --locked`
    and `RUSTDOCFLAGS="-D warnings" cargo doc -p finder --no-deps`;
  - `test` = `cargo test --workspace --all-targets --all-features --locked`,
    `cargo test --workspace --locked --doc`, and then, in `crates/finder/web`, `npm ci`,
    `npm run typecheck`, `npm run format:check`, `npm test`, `npm run build`,
    `git status --porcelain -- dist` (which must list nothing) and `CI=1 npm run e2e`.

  Every story runs both, Rust and page stories alike, and the judge sees their results.
- **CI in the merge queue** runs every `ci.yml` job. Some have no factory counterpart:
  coverage at 80%, the cedar e2e eval and canary, `cargo machete`, `cargo deny`, the
  secret scan, and the build on Rust 1.96 (`msrv`). The sandbox has neither cargo tool
  and builds on stable, so a story meets those checks by construction (§ Plan traps,
  trap 5). A story is not done until it would pass them.

## Prerequisites before this spec is flipped to `ready`

1. **012 and 013 have landed**, and this trio has been re-read against 013 as landed:
   every path marked "013's" in § Project Structure, § Interfaces, § Shapes that existing
   tests pin, owner approval 3's Playwright lines, and `StarFrame`'s next free field
   numbers. 012: met (108a5a8). 013: met; its last story, US10, landed at 0675455. The
   re-read found every cited 013 path where the trio says, and `StarFrame`'s highest
   field at 10, so 014 takes 11 and 12. It found one gap: 013's `simulate` project sets no
   `workers`, so Playwright would run `simulate.spec.ts` and `goto.spec.ts` at once.
   Approval 3 gained `workers: 1` in the flip's PR (§ Plan traps, trap 15).
2. **The owner approvals are given.** Approval 1 is settled: termios through `rustix`,
   chosen 2026-10-31 and written into FR-009, US3-S15, T022 and § User Story 3. The
   others (the `ScopeNodeStop` service, the `[mount]` table and the Playwright lines) were
   accepted the same day; flipping to `ready` records them all. Met: the owner flipped
   014 to `ready` on 2026-10-31.
3. **The repository is wired for the Ergane merge queue.** Met since 012: `ergane.yaml`
   and `ergane-gates.yml` are on `main`, and 012 and 013 have run through the queue.
4. **The factory's coding agent can run cargo.** Met since ergane#896 (deployed
   2026-10-28). US3 changes `Cargo.lock` only if `rustix`'s new features add entries.
5. **Two end-to-end runs never meet.** Met: #125, at 80cbd51 (013 plan, Prerequisite 6).
   US8 and US9 run `CI=1 npm run e2e` through it.
6. **Every CI job that builds `scope-link` installs protoc.** Met: #126, at 7f487e5. US2
   changes the proto, and the `finder-page` job builds `finder`, which depends on
   `scope-link` since 013's US5.

No story adds a file large enough to need staging by the operator. The session fixtures
and test vectors are small, and none goes under `crates/` outside an existing crate
(§ Plan traps, trap 10).

## Plan traps

Lessons from building 012 and 013 on this factory, each with its mechanism and the check
that keeps it from recurring. They bind every story. 013's plan holds the longer record
(013 plan.md § Plan traps).

1. **Gate evidence or nothing.** The judge credits only the gate results and the story's
   diff. A Then clause that names a check only CI runs fails correct work: 012's page
   scenarios did, until #118 put the `finder-page` checks in the `test` gate. *Check:*
   every Then clause names a committed test, a command a gate runs, or the diff. US8-S8
   names the `test` gate's build-freshness commands, and US9-S1 names the `test` gate,
   not the coverage job, which only the queue runs.
2. **A named command is read literally.** The judge refused 012's US3 docs scenario until
   the `lint` gate ran the exact command it named (#113, #114). *Check:* a scenario that
   names a command copies it from `ergane.yaml` verbatim.
3. **One evidence budget.** The story's diff and both gates' output share the judge's
   128 KiB, and `ergane build attempts` shows the diff's share as 98,304 bytes. 013's US2
   was shown whole at 85,558 bytes on its failed second attempt and at 75,986 on its
   passing third, the largest diffs this factory has judged. *Check:* a story keeps its
   diff under 64 KiB, `Cargo.lock` included (`dist/` is `-diff`), measured before proposing
   with `git diff <base> -- . ':(exclude)crates/finder/web/dist' | wc -c`. US3, US5 and US6
   are the largest; one that cannot fit escalates rather than dropping a test.
4. **An additive field still breaks a whole-body test.** 012's US5 added `guidance` to
   session reads, and the existing whole-body comparison stopped the story on a question
   (8a357f48cda5). *Check:* `goto`, `model` and `mount` are left out of the session until
   they have a value (§ Interfaces), and `crates/finder/tests/session_api.rs` is unchanged.
5. **The sandbox has only the gates' tools.** The factory's toolchain is stable Rust with
   rustfmt and clippy, Node 24 and the seeded Chromium. It has no `cargo-machete`, no
   `cargo-deny`, no Rust 1.96 and no `ergane` CLI. Every 013 builder stopped to ask for
   `ergane` before handing off (questions ea1213db3c0b, 4e4fc164492d, c2f71dc3b09d); the
   factory's verification runs that check itself. *Check:* no task runs a tool outside the
   gates. US3's serial code is used, and builds on Rust 1.96, because only the queue checks
   either.
6. **The gate's ports are the host's.** Every builder and gate shares the host's network,
   so each e2e run binds 127.0.0.1:8097–8099. Two stories are in flight at once: US4 beside
   US1 and US2, and US3 beside the console chain. *Check:* every e2e run goes through
   `npm run e2e`, which waits its turn on one host-wide lock (#125); a test binds
   `127.0.0.1:0`, and only `playwright.config.ts` names a fixed port.
7. **Each worktree builds on its own** (#118). *Check:* no story sets `CARGO_TARGET_DIR`,
   and a test's temporary state goes under `target/` or the system's temporary folder.
8. **Fixes land on `main` first.** When the operator fixes a gate, a fixture or this spec
   mid-epic, the fix merges first and the story merges `origin/main`. 012's US3
   cherry-picked two gate fixes and spent a recovery cycle; 013's US2 copied the operator's
   fixture files instead of merging, so its branch carried a stale spec. *Check:* a story
   never cherry-picks or copies an operator's commit; it runs `git merge origin/main`.
9. **The phone's first screen has 8 px to spare.** With a target, 012's legend ends at
   836 px of 844 at 390 × 844, so anything taller above it fails the `test` gate's e2e
   (013 plan, trap 10). GoTo makes the guidance block taller, and Stop adds to the top
   bar. *Check:* `GoToControl` and `StopButton` render nothing when the session has no
   `mount`, as on the `default` and `session` servers; US8 and US9 run `CI=1 npm run e2e`
   with `e2e/session.spec.ts` unchanged and green.
10. **A folder under `crates/` needs a `Cargo.toml`.** The workspace's members are
    `crates/*`, so a folder there without one fails every cargo command (013 plan, trap
    11). *Check:* 014 adds no crate; its fixtures go in `crates/finder/web/src/test/fixtures/`,
    inside an existing package.
11. **A negative assertion passes when nothing happens.** 013's US2-S5 compared two empty
    star lists on a starless frame (013 plan, trap 12). Here "no axis moves", "not a sync"
    and "no epoch" would pass on a mount that is not connected or a frame that never
    solved. *Check:* each negative scenario also asserts its positive side in the same
    test: the frame was solved (US5-S2, US5-S3), a later command is carried out (US2-S1),
    the route tests' simulated mount is connected and moves on an accepted GoTo before a
    refused one is asserted to move nothing (US6-S4, S6, S14, S15's no-sync case, S17;
    US7-S3), and the model held syncs before a refused reset is asserted to leave it as
    it was (US7-S3).
12. **The node's time comes from `NodeClock`.** std's `Instant` never exposes the clock's
    reading, so it cannot share `SensorTimestamp`'s epoch, and it does not follow paused
    tokio time (#133). *Check:* 014's node code reads time only through 013's `NodeClock`
    and waits only with `tokio::time`; no `Instant::now()`, `SystemTime` or
    `clock_gettime` call is added outside 013's `clock.rs`.
13. **`Cargo.lock` is updated by cargo, once.** `--locked` refuses a needed update, and a
    hand edit cannot lock a crate (013's US2, question 995c4dfefc6c). *Check:* US3 runs
    cargo once without `--locked` for `rustix`'s new features, never `cargo update`, and checks
    that the lock's diff only adds; every gate stays `--locked`.
14. **One interceptor cannot exempt one call.** A tonic server interceptor sees only
    metadata, never the method (§ Decisions). *Check:* `ScopeNode` keeps 013's
    `TokenInterceptor` on every call, `ScopeNodeStop` is served with none, and US2-S5 and
    US2-S6 run both over `memory::serve_routes_on`.
15. **One `finder --simulate` server serves the whole `simulate` project.** Playwright runs
    a project's files in parallel workers by default, and 013 set no `workers`.
    `simulate.spec.ts` and `goto.spec.ts` each set the session's target and move the
    simulated telescope, so two at once break each other. 013's `serial` mode orders the
    tests within one file only. *Check:* the `simulate` project has `workers: 1` (owner
    approval 3).

## Risks

| Risk | Mitigation |
|---|---|
| The cable into the mount's jack. Sky-Watcher's own USB adapter may enumerate as HID rather than serial, and RS-232 levels damage the jack. | Use a plain USB–TTL serial cable (FTDI, CP210x, CH340 or PL2303, all in the Pi's kernel) at TTL levels. Confirm with `--probe-mount` before any move (maintainer, at bring-up). |
| Motor firmware below 3.39 may track unevenly (015). | Warn at connect, and record the version at bring-up. |
| 1′ needs the aim point accurate to well under 1′. At 30″ a pixel, 1′ is 2 px. | Centre the calibration star at high power. A wider field on the real finder camera makes 1′ harder; bring-up records the scale, and the tolerance is revisited then if needed. |
| The mechanical limits: no published altitude range, the rocker and ground board limit low altitudes, and there is no azimuth hard stop, so a power cable to the ground can wrap. | GoTo targets are pointable (above the minimum altitude). The one-revolution rule unwinds the cable. The adapter reports any range limits it learns at bring-up. |
| Someone at the eyepiece when the tube moves. | The console never moves the mount unasked: only GoTo and the second sync move it here (015 adds Track). Stop is always on screen while it moves. |
| Freedom Find off means a push by hand is not counted. | Slips are detected at the next sync, and the GoTo still converges (US6 "pushed by hand"). |
| The serial code may not build for musl. | It is `rustix`, which the node already builds with for its clock; the maintainer's cross-build at bring-up confirms the `termios` feature. |
| A story's diff is past what the judge is shown whole. US3, US5 and US6 are the largest. | § Plan traps, trap 3: measured before proposing; a story that cannot fit escalates rather than dropping a test. |
| 013 lands with a name or shape other than § Interfaces assumes. | Prerequisite 1: the operator re-reads the trio against 013 as landed before the flip, and fixes it on `main` first. |

## Refinement Record

Reviewed on 2026-10-31 by the plate-solver operator against `main` at 2a2f9d2 (013's US1,
US8 and US9 landed) and 013's trio, and re-read against 013 as landed before the flip
(§ Prerequisites, item 1). Writes come from the task text; shared writes name the merge
edge that orders them, and the 012 and 013 stories that last write the same file.

```yaml
version: 1
reviewed_revision: 2a2f9d28f40d1d3b6c4b83c93a7f7d9db44e4f39
stories:
  US1:
    tests:
      - path: crates/scope-node/src/mount/mod.rs
        status: new
        disposition: add
        assertion: "a fake two-axis mount honours the Mount contract: capabilities, goto to a position, a signed rate, and stop_all"
        decision: "spec.md:FR-001"
      - path: crates/simulator/tests/mount.rs
        status: new
        disposition: add
        assertion: "in paused time the simulated mount reaches positions within one count and rates within 0.1%; shows 0.3 degrees of backlash within 0.01; a 90 degree move arrives 27.6 +/- 0.1 s later at no more than 3.4 deg/s; a stop from 3.4 deg/s rests 1.13 +/- 0.05 s later; a 1 degree tilt gives true altitudes 2 degrees apart; a push moves only the true direction; a 0.5 degree slip turns 9.5 of 10; a power cycle resets counts only"
        decision: "spec.md:US1-S1"
      - path: crates/scope-link/tests/link.rs
        status: existing
        disposition: preserve
        assertion: "013's link test passes unchanged; US1 does not touch crates/scope-link"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "crates/scope-node depends on star-detection and scope-link only (spec.md US1-S9)"
      - "crates/math-core is untouched (spec.md US1-S9)"
      - "013's simulator tests (crates/simulator/tests/sky.rs, node.rs) pass unchanged; a push still moves the telescope when no mount is connected"
    checks:
      development:
        - "cargo test -p scope-node --locked"
        - "cargo test -p simulator --locked --test mount"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: Bryan, through the merge queue's ci.yml jobs"
    writes:
      - crates/scope-node/src/lib.rs
      - crates/scope-node/src/mount/mod.rs
      - crates/simulator/src/lib.rs
      - crates/simulator/src/mount.rs
      - crates/simulator/src/controls.rs
      - crates/simulator/tests/mount.rs
    shared_writes:
      - path: crates/scope-node/src/mount/mod.rs
        other_spec: 014-goto
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and declares queue and watchdog"
      - path: crates/scope-node/src/lib.rs
        other_spec: 013-live-finder
        other_story: US4
        disposition: merge_order
        reason: "014 depends_on_landed 013, whose US4 last declares modules in it"
      - path: crates/simulator/src/lib.rs
        other_spec: 013-live-finder
        other_story: US4
        disposition: merge_order
        reason: "014 depends_on_landed 013, whose US4 declares node and controls"
      - path: crates/simulator/src/controls.rs
        other_spec: 013-live-finder
        other_story: US4
        disposition: merge_order
        reason: "014 depends_on_landed 013, whose US4 creates the push"
    open_questions: []
  US2:
    tests:
      - path: crates/scope-node/tests/stop_and_watchdog.rs
        status: new
        disposition: add
        assertion: "a stop drops the goto queued behind another command and a goto sent after its reply is carried out; silent heartbeats stop both axes 1.0 +/- 0.1 s after the last; after that stop the axis rests until a new move, which is carried out"
        decision: "spec.md:US2-S1"
      - path: crates/scope-node/tests/frame_positions.rs
        status: new
        disposition: add
        assertion: "a steady azimuth rate and a 200 ms exposure give start and end positions that differ by rate times exposure within two counts, on the node's clock"
        decision: "spec.md:US2-S4"
      - path: crates/scope-link/tests/mount_calls.rs
        status: new
        disposition: add
        assertion: "over memory::serve_routes_on, a move with no token is UNAUTHENTICATED and no axis moves; stop all with no token succeeds and both moving axes stop; describe and state equal the simulator's exactly"
        decision: "spec.md:US2-S5"
      - path: crates/scope-link/tests/link.rs
        status: existing
        disposition: preserve
        assertion: "no token and a wrong token are still UNAUTHENTICATED on ScopeNode, and frames still stream with the token; serve_on is unchanged"
        decision: "spec.md:US2-S8"
    preserved_behavior:
      - "the proto only adds calls, a service, messages and new field numbers, with no [ in comments (spec.md US2-S8)"
      - "ScopeNode keeps 013's TokenInterceptor on every call; only ScopeNodeStop has none (plan.md Plan traps, trap 14)"
      - "every existing test in crates/scope-link and crates/scope-node is unchanged (spec.md US2-S8)"
    checks:
      development:
        - "cargo test -p scope-node --locked"
        - "cargo test -p scope-link --locked"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: Bryan, through the merge queue; finder-page builds scope-link with protoc (#126)"
    writes:
      - crates/scope-node/src/mount/mod.rs
      - crates/scope-node/src/mount/queue.rs
      - crates/scope-node/src/mount/watchdog.rs
      - crates/scope-node/src/capture.rs
      - crates/scope-node/src/service.rs
      - crates/scope-node/tests/stop_and_watchdog.rs
      - crates/scope-node/tests/frame_positions.rs
      - crates/scope-link/proto/scope_node/v1/scope_node.proto
      - crates/scope-link/src/memory.rs
      - crates/scope-link/tests/mount_calls.rs
      - crates/simulator/src/node.rs
      - crates/finder/src/link.rs
    shared_writes:
      - path: crates/scope-node/src/mount/mod.rs
        other_spec: 014-goto
        other_story: US3
        disposition: merge_order
        reason: "US3 depends_on_merged US2, and declares skywatcher"
      - path: crates/scope-node/src/capture.rs
        other_spec: 013-live-finder
        other_story: US4
        disposition: merge_order
        reason: "014 depends_on_landed 013, whose US4 extends the capture loop"
      - path: crates/scope-node/src/service.rs
        other_spec: 013-live-finder
        other_story: US4
        disposition: merge_order
        reason: "014 depends_on_landed 013, whose US4 creates the node service"
      - path: crates/scope-link/proto/scope_node/v1/scope_node.proto
        other_spec: 013-live-finder
        other_story: US1
        disposition: merge_order
        reason: "014 depends_on_landed 013, whose US1 created the proto (67ad559)"
      - path: crates/scope-link/src/memory.rs
        other_spec: 013-live-finder
        other_story: US1
        disposition: merge_order
        reason: "014 depends_on_landed 013, whose US1 created memory.rs (67ad559)"
      - path: crates/simulator/src/node.rs
        other_spec: 013-live-finder
        other_story: US4
        disposition: merge_order
        reason: "014 depends_on_landed 013, whose US4 creates SimNode"
      - path: crates/finder/src/link.rs
        other_spec: 013-live-finder
        other_story: US5
        disposition: merge_order
        reason: "014 depends_on_landed 013, whose US5 creates the link client"
    open_questions: []
  US3:
    tests:
      - path: crates/scope-node/src/mount/skywatcher/codec.rs
        status: new
        disposition: add
        assertion: "0, +1 and -1 encode to 000080, 010080 and FFFF7F and decode back, and every 24-bit value round-trips"
        decision: "spec.md:US3-S1"
      - path: crates/scope-node/src/mount/skywatcher/emulator.rs
        status: new
        disposition: add
        assertion: "every implemented command is answered =data or !code per the command set"
        decision: "spec.md:US3-S3"
      - path: crates/scope-node/tests/skywatcher.rs
        status: new
        disposition: add
        assertion: "against the emulator: :S1230180 reaches +0x123; refused :S falls back to :H and :M; !2 stops, polls :f and resends once; a reversal waits for stop; 115200 is found after 9600; echo is skipped; old firmware warns; :q leads to :F3, :a, :b, :g and :W; :I step periods and fast mode above 128x sidereal hold rates within 0.1%; an unacknowledged :K leads to :L; the probe's order and report"
        decision: "spec.md:US3-S4"
      - path: crates/scope-node/tests/mount_config.rs
        status: new
        disposition: add
        assertion: "--check-config prints a [mount] device and exits 0, prints that no mount is configured without the table and exits 0, and exits nonzero naming an unknown [mount] key or a missing device"
        decision: "spec.md:US3-S16"
    preserved_behavior:
      - "013's config keys and crates/scope-node/tests/cli.rs pass unchanged; the [mount] table is optional (plan.md Shapes that existing tests pin)"
      - "rustix's termios and fs features are the only dependency change, and opening the device is the only untested code (spec.md US3-S15)"
      - "Cargo.lock keeps every existing entry at its version (plan.md Plan traps, trap 13)"
    checks:
      development:
        - "cargo test -p scope-node --locked"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: Bryan, through the merge queue's deny and msrv jobs; outside the factory, the musl cross-build and --probe-mount at the telescope (spec.md Assumptions)"
    writes:
      - Cargo.lock
      - crates/scope-node/Cargo.toml
      - crates/scope-node/src/mount/mod.rs
      - crates/scope-node/src/mount/skywatcher/mod.rs
      - crates/scope-node/src/mount/skywatcher/codec.rs
      - crates/scope-node/src/mount/skywatcher/transport.rs
      - crates/scope-node/src/mount/skywatcher/protocol.rs
      - crates/scope-node/src/mount/skywatcher/emulator.rs
      - crates/scope-node/src/mount/skywatcher/serial.rs
      - crates/scope-node/src/config.rs
      - crates/scope-node/src/main.rs
      - crates/scope-node/tests/skywatcher.rs
      - crates/scope-node/tests/mount_config.rs
    shared_writes:
      - path: Cargo.lock
        other_spec: 013-live-finder
        other_story: US7
        disposition: merge_order
        reason: "014 depends_on_landed 013, whose stories US1 to US7 change the lockfile"
      - path: crates/scope-node/Cargo.toml
        other_spec: 013-live-finder
        other_story: US4
        disposition: merge_order
        reason: "014 depends_on_landed 013, whose US4 last adds a dependency"
      - path: crates/scope-node/src/config.rs
        other_spec: 013-live-finder
        other_story: US2
        disposition: merge_order
        reason: "014 depends_on_landed 013, whose US2 creates the config"
      - path: crates/scope-node/src/main.rs
        other_spec: 013-live-finder
        other_story: US4
        disposition: merge_order
        reason: "014 depends_on_landed 013, whose US4 adds serving"
    open_questions: []
  US4:
    tests:
      - path: crates/finder/src/ephemeris.rs
        status: new
        disposition: add
        assertion: "Bennett at apparent 0 degrees is 34.5 +/- 0.1 arcmin; Saemundsson at true 45 degrees is 1.01 +/- 0.01 arcmin; adding then removing refraction from 0 to 90 degrees returns within 0.1 arcmin; Meeus 13.b stays 15.1249 degrees"
        decision: "spec.md:US4-S1"
      - path: crates/finder/src/sky.rs
        status: existing
        disposition: preserve
        assertion: "alt_az_matches_meeus_example_13b passes unchanged, and sky::alt_az stays geometric"
        decision: "spec.md:US4-S4"
    preserved_behavior:
      - "sky::alt_az is unchanged and refraction lives only in finder::ephemeris (spec.md US4-S5)"
      - "012's Sun and rise, transit and set times in ephemeris.rs answer as before"
    checks:
      development:
        - "cargo test -p finder --locked ephemeris"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: Bryan, through the merge queue"
    writes:
      - crates/finder/src/ephemeris.rs
    shared_writes:
      - path: crates/finder/src/ephemeris.rs
        other_spec: 012-targets-and-push-to
        other_story: US1
        disposition: merge_order
        reason: "014 depends_on_landed 012, whose US1 last wrote ephemeris.rs (7c7f2b4)"
    open_questions: []
  US5:
    tests:
      - path: crates/finder/src/model.rs
        status: new
        disposition: add
        assertion: "interpolation gives count 1030 between 1000 and 1060; IA, IE, AN and AW are recovered within 1 arcsec; one sync mispredicts 2.00 +/- 0.01 degrees opposite a 1 degree tilt; 0.6 degrees opens an epoch and 0.4 does not; 24 syncs recover CA, NPAE, TF and a 4 degree camera rotation within 1 arcsec; 19 fit no fixed term and report the need; a reset clears the fixed terms"
        decision: "spec.md:US5-S5"
      - path: crates/finder/tests/model_sim.rs
        status: new
        disposition: add
        assertion: "in paused time with the simulator: a still mount keeps one sync within one count; a 2 deg/s frame is solved and charted but no sync; an uploaded frame is solved and becomes the pointing but no sync; syncs at 120 and 250 degrees fit the tilt within 1 arcmin; a restart at 05:30 UT keeps the night; after about 17:10 UT the night is gone and the fixed terms stay"
        decision: "spec.md:US5-S1"
      - path: crates/finder/tests/session_api.rs
        status: existing
        disposition: preserve
        assertion: "a fresh session's whole body is still 012's fields; model and mount are left out until they have a value"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "the fit and inverse are in-crate with no new dependency, and crates/math-core is untouched (spec.md US5-S14)"
      - "every existing finder test is unchanged (spec.md US5-S14)"
    checks:
      development:
        - "cargo test -p finder --locked model"
        - "cargo test -p finder --locked --test model_sim"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: Bryan, through the merge queue; the coverage floor counts the simulator tests (spec.md SC-003)"
    writes:
      - crates/finder/src/lib.rs
      - crates/finder/src/model.rs
      - crates/finder/src/session.rs
      - crates/finder/src/live.rs
      - crates/finder/src/api.rs
      - crates/finder/tests/model_sim.rs
    shared_writes:
      - path: crates/finder/src/lib.rs
        other_spec: 014-goto
        other_story: US6
        disposition: merge_order
        reason: "US6 depends_on_merged US5, and declares goto"
      - path: crates/finder/src/api.rs
        other_spec: 014-goto
        other_story: US6
        disposition: merge_order
        reason: "US6 depends_on_merged US5, and adds the goto and stop routes"
      - path: crates/finder/src/lib.rs
        other_spec: 013-live-finder
        other_story: US7
        disposition: merge_order
        reason: "014 depends_on_landed 013, whose US7 last declares modules in it"
      - path: crates/finder/src/session.rs
        other_spec: 013-live-finder
        other_story: US6
        disposition: merge_order
        reason: "014 depends_on_landed 013, whose US6 last adds session state"
      - path: crates/finder/src/live.rs
        other_spec: 013-live-finder
        other_story: US6
        disposition: merge_order
        reason: "014 depends_on_landed 013, whose US6 creates the live loop"
      - path: crates/finder/src/api.rs
        other_spec: 013-live-finder
        other_story: US7
        disposition: merge_order
        reason: "014 depends_on_landed 013, whose US7 last adds routes"
    open_questions: []
  US6:
    tests:
      - path: crates/finder/src/goto.rs
        status: new
        disposition: add
        assertion: "350 degrees wound and a target 30 degrees clockwise plans anticlockwise 330, also after a power cycle and a new epoch; a 40 s slew is planned to the place 40 s later within 1 s"
        decision: "spec.md:US6-S1"
      - path: crates/finder/tests/goto_sim.rs
        status: new
        disposition: add
        assertion: "in paused time: the true altitude stays at or above 15 degrees; the dusk slew stays 15 degrees from the Sun; five GoTos end within 1 arcmin in at most 5 corrections; a slip on every correction fails after 5; a hand push still ends within 1 arcmin; Stop, a lost link and the Sun each halt both axes and report why; the session reports slewing, solving, each correction and done"
        decision: "spec.md:US6-S8"
      - path: crates/finder/tests/goto_api.rs
        status: new
        disposition: add
        assertion: "with the simulated mount connected and an accepted GoTo moving an axis first: 15.2 degrees, the Sun up, M8 below the horizon, no mount and no sync each answer 409 naming the reason and move no axis; a foreign Origin answers 403 and moves no axis"
        decision: "spec.md:US6-S4"
      - path: crates/finder/tests/session_writes.rs
        status: existing
        disposition: preserve
        assertion: "012's same-origin rule still refuses foreign writes on every route but /api/stop"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "no API field, page control or option turns off the Sun guard (spec.md SC-005)"
      - "every existing test is unchanged (tasks.md T041)"
    checks:
      development:
        - "cargo test -p finder --locked goto"
        - "cargo test -p finder --locked --test goto_sim --test goto_api"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: Bryan, through the merge queue; outside the factory, GoTos to five bright targets at the telescope (spec.md Clarifications)"
    writes:
      - crates/finder/src/lib.rs
      - crates/finder/src/goto.rs
      - crates/finder/src/api.rs
      - crates/finder/README.md
      - crates/finder/tests/goto_sim.rs
      - crates/finder/tests/goto_api.rs
    shared_writes:
      - path: crates/finder/src/goto.rs
        other_spec: 014-goto
        other_story: US7
        disposition: merge_order
        reason: "US7 depends_on_merged US6, and adds the second sync"
      - path: crates/finder/src/api.rs
        other_spec: 014-goto
        other_story: US7
        disposition: merge_order
        reason: "US7 depends_on_merged US6, and adds the model routes"
      - path: crates/finder/README.md
        other_spec: 014-goto
        other_story: US7
        disposition: merge_order
        reason: "US7 depends_on_merged US6, and documents the model routes"
      - path: crates/finder/README.md
        other_spec: 013-live-finder
        other_story: US7
        disposition: merge_order
        reason: "014 depends_on_landed 013, whose US7 last documents routes"
    open_questions: []
  US7:
    tests:
      - path: crates/finder/tests/second_sync_sim.rs
        status: new
        disposition: add
        assertion: "with one sync at 120 degrees and the base tilted toward 130, the second sync slews at least 90 degrees away, takes a sync, and fits the tilt within 1 arcmin"
        decision: "spec.md:US7-S1"
      - path: crates/finder/tests/model_api.rs
        status: new
        disposition: add
        assertion: "reset for the night clears syncs, zero points and tilt and keeps the fixed terms, and for the fixed terms clears them; a foreign Origin on either route answers 403, moves no axis and leaves the model as it was"
        decision: "spec.md:US7-S2"
    preserved_behavior:
      - "both routes sit behind 012's same-origin rule (spec.md FR-023)"
      - "every existing test is unchanged (tasks.md T046)"
    checks:
      development:
        - "cargo test -p finder --locked --test second_sync_sim --test model_api"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: Bryan, through the merge queue; outside the factory, the second sync at the telescope"
    writes:
      - crates/finder/src/goto.rs
      - crates/finder/src/api.rs
      - crates/finder/README.md
      - crates/finder/tests/second_sync_sim.rs
      - crates/finder/tests/model_api.rs
    shared_writes: []
    open_questions: []
  US8:
    tests:
      - path: crates/finder/web/src/components/GoToControl.test.tsx
        status: new
        disposition: add
        assertion: "GoTo is disabled with the reason for below the horizon, the Sun up and no sync, with Stop still shown while moving; a recorded GoTo shows the slew, each correction and on target with the final error"
        decision: "spec.md:US8-S1"
      - path: crates/finder/web/src/components/StopButton.test.tsx
        status: new
        disposition: add
        assertion: "Stop is in Setup's top bar while the second sync slews and sends POST /api/stop; it is enabled and outside the sheet's dialog and any inert or aria-hidden subtree; it renders nothing with no mount"
        decision: "spec.md:US8-S3"
      - path: crates/finder/web/src/components/MountSection.test.tsx
        status: new
        disposition: add
        assertion: "base tilt 1.0 degrees toward 130 within 0.1 and 2, with the RMS and 2 syncs; link, board, second sync and both resets, each sending its route"
        decision: "spec.md:US8-S5"
      - path: crates/finder/web/src/components/GuidanceBlock.test.tsx
        status: existing
        disposition: preserve
        assertion: "the guidance block's 012 and 013 cases pass unchanged; GoTo renders nothing without a mount"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/src/components/SetupPage.test.tsx
        status: existing
        disposition: preserve
        assertion: "Setup's existing cases pass unchanged with the new section in it"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/e2e/session.spec.ts
        status: existing
        disposition: preserve
        assertion: "the first screen at 390 x 844 still fits, with no mount on the session server, and Setup has no off-red pixel"
        decision: "plan.md:Plan traps"
    preserved_behavior:
      - "no npm package is added (spec.md US8-S8)"
      - "every existing Vitest file is unchanged (tasks.md T052)"
    checks:
      development:
        - "cd crates/finder/web && npm test -- src/components/GoToControl.test.tsx src/components/StopButton.test.tsx src/components/MountSection.test.tsx"
        - "cd crates/finder/web && CI=1 npm run e2e -- --project session"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: Bryan; the maintainer's Impeccable review of GoTo, Stop and the section after US8 lands (spec.md Assumptions)"
    writes:
      - DESIGN.md
      - .impeccable/design.json
      - crates/finder/web/dist
      - crates/finder/web/src/api.ts
      - crates/finder/web/src/App.tsx
      - crates/finder/web/src/components/TopBar.tsx
      - crates/finder/web/src/components/GuidanceBlock.tsx
      - crates/finder/web/src/components/SetupPage.tsx
      - crates/finder/web/src/components/GoToControl.tsx
      - crates/finder/web/src/components/GoToControl.test.tsx
      - crates/finder/web/src/components/StopButton.tsx
      - crates/finder/web/src/components/StopButton.test.tsx
      - crates/finder/web/src/components/MountSection.tsx
      - crates/finder/web/src/components/MountSection.test.tsx
      - crates/finder/web/src/test/fixtures
    shared_writes:
      - path: DESIGN.md
        other_spec: 013-live-finder
        other_story: US9
        disposition: merge_order
        reason: "014 depends_on_landed 013, whose US9 last recorded its parts (2a2f9d2)"
      - path: .impeccable/design.json
        other_spec: 013-live-finder
        other_story: US9
        disposition: merge_order
        reason: "014 depends_on_landed 013, whose US9 last recorded its parts (2a2f9d2)"
      - path: crates/finder/web/dist
        other_spec: 013-live-finder
        other_story: US9
        disposition: merge_order
        reason: "014 depends_on_landed 013, whose US9 last rebuilt the bundle (2a2f9d2)"
      - path: crates/finder/web/src/api.ts
        other_spec: 013-live-finder
        other_story: US9
        disposition: merge_order
        reason: "014 depends_on_landed 013, whose US9 last wrote api.ts (2a2f9d2)"
      - path: crates/finder/web/src/App.tsx
        other_spec: 013-live-finder
        other_story: US8
        disposition: merge_order
        reason: "014 depends_on_landed 013, whose US8 last wrote App.tsx (da78790)"
      - path: crates/finder/web/src/components/TopBar.tsx
        other_spec: 012-targets-and-push-to
        other_story: US7
        disposition: merge_order
        reason: "014 depends_on_landed 012, whose US7 last wrote TopBar.tsx (cf09362)"
      - path: crates/finder/web/src/components/GuidanceBlock.tsx
        other_spec: 013-live-finder
        other_story: US8
        disposition: merge_order
        reason: "014 depends_on_landed 013, whose US8 last wrote GuidanceBlock.tsx (da78790)"
      - path: crates/finder/web/src/components/SetupPage.tsx
        other_spec: 013-live-finder
        other_story: US9
        disposition: merge_order
        reason: "014 depends_on_landed 013, whose US9 last wrote SetupPage.tsx (2a2f9d2)"
    open_questions: []
  US9:
    tests:
      - path: crates/finder/tests/night_sim.rs
        status: new
        disposition: add
        assertion: "a paused-time night of first and second syncs, five GoTos, a hand push and a GoTo, a Stop and a dropped link: every GoTo within 1 arcmin and every halt at rest"
        decision: "spec.md:US9-S1"
      - path: crates/finder/web/e2e/goto.spec.ts
        status: new
        disposition: add
        assertion: "on finder --simulate: a GoTo to M42 ends with the reticle within 1 arcmin of the aim point; Stop halts the axes; the second sync from Setup shows the base tilt; the night pixel check passes while GoTo runs; Stop is at least 44 px and the element at its centre"
        decision: "spec.md:US9-S2"
      - path: crates/finder/web/e2e/console.spec.ts
        status: existing
        disposition: preserve
        assertion: "every console test passes unchanged in the default project, which now also ignores goto.spec.ts"
        decision: "spec.md:US9-S7"
      - path: crates/finder/web/e2e/session.spec.ts
        status: existing
        disposition: preserve
        assertion: "every session test passes unchanged in the session project"
        decision: "spec.md:US9-S7"
    preserved_behavior:
      - "013's e2e/simulate.spec.ts and e2e/live-off.spec.ts pass unchanged (plan.md Shapes that existing tests pin)"
      - "playwright.config.ts changes only by approval 3's three lines, including workers: 1 on the simulate project (spec.md US9-S7)"
    checks:
      development:
        - "cargo test -p finder --locked --test night_sim"
        - "cd crates/finder/web && CI=1 npm run e2e"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: Bryan; 8097-8099 free on the factory host while it builds (plan.md Plan traps, trap 6); the telescope checks outside the factory"
    writes:
      - crates/finder/tests/night_sim.rs
      - crates/finder/web/e2e/goto.spec.ts
      - crates/finder/web/playwright.config.ts
    shared_writes:
      - path: crates/finder/web/playwright.config.ts
        other_spec: 013-live-finder
        other_story: US10
        disposition: merge_order
        reason: "014 depends_on_landed 013, whose US10 adds the simulate project"
    open_questions: []
```
