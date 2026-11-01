---
description: "Task list for 014-goto: GoTo"
---

# Tasks: GoTo

**Prerequisites** (plan.md § Prerequisites):
- 012 and 013 have landed (`depends_on_landed`), and this trio has been re-read against
  013 as landed;
- the owner approvals in spec.md are given (approval 1: termios through `rustix`);
- the repository is wired for the Ergane merge queue (met since 012);
- the coding agent can run cargo (met since ergane#896);
- `npm run e2e` waits its turn on one host-wide lock (met by #125, at 80cbd51);
- every CI job that builds `scope-link` installs protoc (met by #126, at 7f487e5).

**Rules for every story:**
- Never edit an existing test, golden, `scripts/ci/`, `.github/` or `CODEOWNERS`.
- New Rust tests go in new files under `crates/*/tests/`, or in-module.
- `crates/math-core` is never touched.
- No test opens a serial device or moves real hardware. Mount behaviour is proven on the
  simulated mount or the byte-level emulator, in paused virtual time.
- Read 013's modules before editing them, and keep to the names 013 actually landed.
- No step runs a tool outside the factory's gates (plan.md § Plan traps, trap 5).
- When the operator says a fix landed on `main`, run `git merge origin/main`; never
  cherry-pick or copy an operator's commit (trap 8).
- Node code reads time only through 013's `NodeClock` and waits only with `tokio::time`
  (trap 12).

## Phase 1: User Story 1 — the mount port and the simulated mount (Priority: P1)

### Tests first

- [ ] T001 [US1] In a new `crates/scope-node/src/mount/mod.rs`, write failing in-module
  tests of the `Mount` trait's contract on a fake two-axis mount: capabilities, goto to a
  position, a signed rate, and `stop_all`.
- [ ] T002 [US1] In a new `crates/simulator/tests/mount.rs`, write failing tests under
  `#[tokio::test(start_paused = true)]` against the simulated mount through the port:
  - capabilities, positions within one count, rates within 0.1%, and stopping (US1-S1);
  - backlash of 0.3° within 0.01° at axis position 0° from either side (US1-S2);
  - a 90° move never above 3.4°/s, arriving 27.6 s ± 0.1 s later (US1-S3);
  - a stop from 3.4°/s at rest 1.13 s ± 0.05 s later (US1-S4);
  - true altitudes 2° apart within 0.01° at axis azimuths 0° and 180° (US1-S5);
  - a 3° push by hand that moves the true direction 3° within 0.01° and leaves both axis
    positions unchanged (US1-S6);
  - a 0.5° slip per move: axis position 10°, true turn 9.5° within 0.01° (US1-S7);
  - a power cycle that resets both axis positions and leaves the true direction
    unchanged (US1-S8).

### Implementation

- [ ] T003 [US1] Implement the `Mount` trait, `Axis`, `AxisInfo` and `AxisState` in
  `crates/scope-node/src/mount/mod.rs`, and add `pub mod mount;` to
  `crates/scope-node/src/lib.rs`. Document every public item.
- [ ] T004 [US1] Implement the simulated mount in a new `crates/simulator/src/mount.rs`,
  and add `pub mod mount;` to `crates/simulator/src/lib.rs`:
  - two axes in virtual time, with a trapezoidal velocity profile (3°/s², 3.4°/s);
  - backlash as a 0.3° dead band;
  - a 1° base tilt by default, as a rotation of the mount's frame;
  - zero-point offsets from north and level.
- [ ] T005 [US1] Add the simulated mount's test hooks in `crates/simulator/src/mount.rs`:
  push by hand, slip per move, power-cycle, and the true eyepiece direction. Make 013's
  simulated telescope follow the mount's true direction while a mount is connected, and
  route 013's push by hand (`push_to` and `push_by` in `crates/simulator/src/controls.rs`)
  through the hook.
- [ ] T006 [US1] Run the `lint` and `test` gates. Confirm
  three things: `crates/scope-node/Cargo.toml` still depends on `star-detection` and
  `scope-link` only; `crates/math-core` is untouched; and every existing test is unchanged
  (US1-S9). Commit the green story (FR-001, FR-002, FR-003; US1-S1, US1-S2, US1-S3,
  US1-S4, US1-S5, US1-S6, US1-S7, US1-S8, US1-S9).

## Phase 2: User Story 2 — Stop wins, the watchdog, frames and the link (Priority: P1)

### Tests first

- [ ] T007 [US2] In a new `crates/simulator/tests/stop_and_watchdog.rs`, beside 013's
  `node.rs`, write failing paused-time tests on the real node service over the simulated
  mount, with any helper T007 to T009 share in a new
  `crates/simulator/tests/support/mod.rs` (plan.md § Decisions, "US2's tests live where
  their dependencies already are"):
  - a stop drops the goto queued behind another command, and a goto sent after the stop's
    reply is carried out (US2-S1), so the test fails if no goto is ever carried out;
  - with both axes moving, silent heartbeats lead to a stop command 1.0 ± 0.1 s after the
    last one (US2-S2);
  - after the watchdog's stop, resumed heartbeats leave the axis at rest until a new move
    is requested, and the new move is carried out (US2-S3).
- [ ] T008 [US2] In a new `crates/simulator/tests/frame_positions.rs`, write a failing
  test: a steady azimuth rate and a 200 ms exposure give start and end positions that
  differ by rate × exposure within two counts, on the node's clock (US2-S4).
- [ ] T009 [US2] In a new `crates/finder/tests/mount_calls.rs`, beside 013's `link.rs`,
  write failing tests over the in-process link. Add `scope-node` to the
  `[dev-dependencies]` of `crates/finder/Cargo.toml` (spec.md, owner approval 4), and
  update `Cargo.lock` by running cargo once without `--locked` (plan.md § Plan traps,
  trap 13):
  - a move with no token fails with UNAUTHENTICATED and no axis moves (US2-S5);
  - stop all with no token succeeds while both axes move, and both stop (US2-S6), with
    `ScopeNode`, `ScopeNodeMount` and `ScopeNodeStop` served together by
    `memory::serve_routes_on`;
  - describe and state through the link equal the simulator's counts per revolution, top
    rate, position, rate and moving flag exactly (US2-S7).

### Implementation

- [ ] T010 [US2] Implement the command queue with its epoch counter and the Stop bypass in
  a new `crates/scope-node/src/mount/queue.rs`. Stop raises the epoch, clears the queue
  and commands the stop at once, and older-epoch commands are dropped.
- [ ] T011 [US2] Implement the watchdog in a new `crates/scope-node/src/mount/watchdog.rs`:
  a 50 ms `tokio::time` tick, reading 013's `NodeClock`, that calls `stop_all` at 1 s since
  the last heartbeat and marks the link lost. Nothing resumes on its own. Declare `queue`
  and `watchdog` in `crates/scope-node/src/mount/mod.rs`.
- [ ] T012 [US2] In 013's capture loop, `crates/scope-node/src/capture.rs`, read both axes'
  positions at exposure start and end while a mount is connected.
- [ ] T013 [US2] In `crates/scope-link/proto/scope_node/v1/scope_node.proto`, add a new
  `service ScopeNodeMount` holding `DescribeMount`, `AxisState`, `GotoAxis` and
  `RateAxis`, a new `service ScopeNodeStop` holding only `StopAll`, and optional
  `axis_start` and `axis_end` to `StarFrame`, on new field numbers only and with no
  `[...]` in comments. Leave 013's `service ScopeNode` unchanged (plan.md § Decisions,
  "The mount calls are their own service"). In `crates/scope-link/tests/link.rs`, add
  `axis_start: None` and `axis_end: None` to `star_frame`'s literal and change nothing
  else in that file (US2-S8). Add `serve_routes_on` to `crates/scope-link/src/memory.rs`,
  keeping `serve_on`. Serve all three services from 013's
  `crates/scope-node/src/service.rs` over the queue, `ScopeNode` and `ScopeNodeMount` with
  013's `TokenInterceptor` and `ScopeNodeStop` with none, and from 013's `SimNode` in
  `crates/simulator/src/node.rs`. Add the matching calls to the console's link client,
  `crates/finder/src/link.rs`, through a `ScopeNodeMount` client on the link's channel
  and token.
- [ ] T014 [US2] Run the `lint` and `test` gates. Confirm that the proto change only
  adds, that 013's `service ScopeNode` is unchanged, and that every existing test in
  `crates/scope-link`, `crates/scope-node` and `crates/finder` is unchanged except
  `star_frame`'s two new fields (US2-S8), and that `Cargo.lock`'s diff is the one
  `scope-node` line in `finder`'s entry. Commit the green story (FR-004, FR-005,
  FR-006; US2-S1, US2-S2, US2-S3, US2-S4, US2-S5, US2-S6, US2-S7, US2-S8).

## Phase 3: User Story 3 — the Sky-Watcher adapter (Priority: P2)

### Tests first

- [ ] T015 [US3] In a new `crates/scope-node/src/mount/skywatcher/codec.rs`, write failing
  in-module tests:
  - 0, +1 and −1 encode to `000080`, `010080` and `FFFF7F` and decode back (US3-S1);
  - every 24-bit value round-trips (US3-S2).
- [ ] T016 [US3] In a new `crates/scope-node/src/mount/skywatcher/emulator.rs`, write
  failing in-module tests: each implemented command (`:e`, `:a`, `:b`, `:g`, `:f`, `:j`,
  `:q`, `:F`, `:W`, `:G`, `:S`, `:H`, `:M`, `:J`, `:I`, `:K`, `:L`) is answered `=data` or
  `!code` per the command set (US3-S3).
- [ ] T017 [US3] In a new `crates/scope-node/tests/skywatcher.rs`, write failing tests of
  the adapter against the emulator:
  - axis 1 to +0x123 sends `:S1230180\r` and reaches +0x123 (US3-S4);
  - a refused `:S` falls back to `:H` and `:M` and reaches the position (US3-S5);
  - `!2` leads to a stop, `:f` polled until stopped, and one resend (US3-S6);
  - +1°/s to −1°/s is stopped and reported stopped before the new direction (US3-S7);
  - an emulator only at 115200 is found there after 9600, with its version (US3-S8);
  - a single-wire echo is skipped (US3-S9);
  - firmware below 3.39 is accepted with a warning (US3-S10);
  - `:q` reporting auxiliary encoders leads to `:F3`, `:a`, `:b`, `:g` and `:W` (US3-S11);
  - the `:I` step period, fast mode only above 128× sidereal, and each rate held within
    0.1% (US3-S12);
  - an unacknowledged `:K` leads to `:L` on that axis (US3-S13);
  - the probe sequence `:e1`, `:a1`, `:b1`, `:g1`, `:q1010000`, `:j1` in order, and its
    report's fields (US3-S14).

  In a new `crates/scope-node/tests/mount_config.rs`, write failing CLI tests of
  `scope-node --check-config`: a `[mount]` table with `device = "/dev/ttyUSB0"` prints the
  device and exits 0; no `[mount]` table prints that no mount is configured and exits 0;
  an unknown key in `[mount]`, or no `device`, exits nonzero naming the key (US3-S16).

### Implementation

- [ ] T018 [US3] Implement the codec (24-bit little-endian hex, the 0x800000 offset,
  framing, `=`/`!` replies) in `crates/scope-node/src/mount/skywatcher/codec.rs`.
- [ ] T019 [US3] Implement the byte-level emulator in
  `crates/scope-node/src/mount/skywatcher/emulator.rs`, over an axis model like the
  simulator's. Give it modes for 115200 only, single-wire echo, injected `!2`, refused
  `:S`, an unacknowledged `:K` and old firmware.
- [ ] T020 [US3] Implement the transport in a new
  `crates/scope-node/src/mount/skywatcher/transport.rs`: echo skip, a 300 ms reply
  timeout, and up to 5 tries for queries and other commands safe to repeat.
- [ ] T021 [US3] Implement the protocol in a new
  `crates/scope-node/src/mount/skywatcher/protocol.rs` as a `Mount`:
  - connect: baud probe, `:F3`, `:a`, `:b`, `:g`, `:q`, `:W`, and the firmware warning;
  - rates as `:I` with fast mode;
  - goto with `:G`, `:S` and `:J`, and the `:H`/`:M` fallback;
  - reversing by stop and wait;
  - the `!2` retry;
  - stop by `:K`, then `:L`.
- [ ] T022 [US3] Add `rustix`'s `termios` and `fs` features (spec.md, owner approval 1)
  to `crates/scope-node/Cargo.toml`, and update `Cargo.lock` by running cargo once without
  `--locked` (plan.md § Plan traps, trap 13). Open the device in a new
  `crates/scope-node/src/mount/skywatcher/serial.rs`, the only untested shell, and declare
  the adapter's modules in `crates/scope-node/src/mount/skywatcher/mod.rs` and
  `crates/scope-node/src/mount/mod.rs`. Add the optional `[mount]` table (`device`) to
  013's `crates/scope-node/src/config.rs`, printed by `--check-config`. In
  `crates/scope-node/src/main.rs`, add `--probe-mount DEVICE` over a probe function that
  takes any byte stream, and drive the mount from the `[mount]` table when it is set.
- [ ] T023 [US3] Run the `lint` and `test` gates. Confirm that `rustix`'s two features are
  the only dependency change and that opening the device is the only code path no test
  reaches (US3-S15). Commit the green story (FR-007, FR-008, FR-009; US3-S1, US3-S2,
  US3-S3, US3-S4, US3-S5, US3-S6, US3-S7, US3-S8, US3-S9, US3-S10, US3-S11, US3-S12,
  US3-S13, US3-S14, US3-S15, US3-S16).

## Phase 4: User Story 4 — apparent altitude (Priority: P1)

### Tests first

- [ ] T024 [US4] In `crates/finder/src/ephemeris.rs`, write failing in-module tests:
  - Bennett at apparent 0° is 34.5′ ± 0.1′ (US4-S1);
  - Saemundsson at true 45° is 1.01′ ± 0.01′ (US4-S2);
  - adding then removing refraction from 0° to 90° in 0.1° steps returns within 0.1′
    (US4-S3);
  - Meeus 13.b's altitude stays 15.1249° within 0.01° (US4-S4).

### Implementation

- [ ] T025 [US4] Implement `apparent_from_true` (Saemundsson, Meeus 16.4) and
  `true_from_apparent` (Bennett, Meeus 16.3) for 1010 hPa and 10 °C in
  `crates/finder/src/ephemeris.rs`, with docs.
- [ ] T026 [US4] Run the `lint` and `test` gates. Confirm that
  `sky::alt_az` and `alt_az_matches_meeus_example_13b` are unchanged (US4-S4, US4-S5).
  Commit the green story (FR-010; US4-S1, US4-S2, US4-S3, US4-S4, US4-S5).

## Phase 5: User Story 5 — syncs and the pointing model (Priority: P1)

### Tests first

- [ ] T027 [US5] In a new `crates/finder/src/model.rs`, write failing in-module tests on
  synthetic syncs:
  - linear interpolation of the azimuth axis from count 1000 to 1060 gives 1030 (US5-S4);
  - IA 2°, IE −0.5°, AN 0.6° and AW −0.8° are each recovered within 1″ (US5-S5);
  - one sync with a 1° base tilt mispredicts by 2.00° ± 0.01° at azimuth 180° (US5-S6);
  - after the base tilt, 0.6° opens an epoch (IA and IE refitted, base tilt and fixed
    terms kept) and 0.4° does not (US5-S8);
  - 24 syncs over 200° × 40° recover CA 10′, NPAE 3′, TF 5′ and a 4° camera rotation
    within 1″ (US5-S9);
  - 19 syncs fit no fixed term and report how many more are needed (US5-S10);
  - a reset of the fixed terms leaves none until enough syncs again (US5-S13).
- [ ] T028 [US5] In a new `crates/finder/tests/model_sim.rs`, write failing paused-time
  tests of the simulator and the console over the in-process link:
  - a still mount with frames at 2 a second for 10 s keeps one sync, with axis positions
    within one count of the simulator's at mid-exposure (US5-S1);
  - a frame exposed at 2°/s is solved and charted but is no sync (US5-S2);
  - a frame uploaded to `/api/solve` with the mount connected becomes the latest pointing
    but is no sync (US5-S3);
  - one sync at azimuth 120° and a second at 250° open no epoch, and fit the base tilt
    within 1′ of the simulator's (US5-S7);
  - a restart at 05:30 UT on 2026-01-16 after syncs at 02:00 UT at 40° N, 75° W keeps the
    syncs, zero points and base tilt (US5-S11);
  - reading the session after about 17:10 UT clears them and keeps the fixed terms
    (US5-S12).

### Implementation

- [ ] T029 [US5] Implement the forward model in `crates/finder/src/model.rs`: counts to
  angles; IA and IE as exact offsets; the base tilt as an exact rotation; CA, NPAE and TF
  as TPoint small-angle terms; and the camera rotation through the parallactic angle. Add
  `pub mod model;` to `crates/finder/src/lib.rs`.
- [ ] T030 [US5] Implement the fit in `crates/finder/src/model.rs`: Levenberg–Marquardt on
  (ΔA·cos E, ΔE, Δroll) weighted by solve RMS, with the normal equations solved by
  Cholesky; the Newton inverse from the uncorrected guess; epochs and the 2.5°/0.5° slip
  thresholds; and the fixed terms' 20-sync, 180° × 30° rule with `needs`.
- [ ] T031 [US5] Implement sync intake in `crates/finder/src/model.rs` (the 0.2°/s rule,
  interpolation to mid-exposure, thinning to one per 2° per 2 minutes, refraction removed
  first). In 013's live loop, `crates/finder/src/live.rs`, hand each solved star frame
  with axis positions to it. Keep
  `/api/solve` uploads out of it.
- [ ] T032 [US5] In `crates/finder/src/session.rs`, keep the night's syncs, epochs and base
  tilt until the Sun's next upper transit after the first sync, across restarts, and the
  fixed terms until reset. Add `model` and `mount` to `GET /api/session` in
  `crates/finder/src/api.rs`, each left out of the body until it has a value, so
  `crates/finder/tests/session_api.rs` stays as it is (plan.md § Plan traps, trap 4).
- [ ] T033 [US5] Run the `lint` and `test` gates. Confirm that the
  fit adds no dependency, that `crates/math-core` is untouched, and that every existing
  `finder` test is unchanged (US5-S14). Commit the green story (FR-011, FR-012, FR-013,
  FR-014, FR-015; US5-S1, US5-S2, US5-S3, US5-S4, US5-S5, US5-S6, US5-S7, US5-S8, US5-S9,
  US5-S10, US5-S11, US5-S12, US5-S13, US5-S14).

## Phase 6: User Story 6 — GoTo (Priority: P1)

### Tests first

- [ ] T034 [US6] In a new `crates/finder/src/goto.rs`, write failing in-module planner
  tests:
  - 350° wound and a target 30° clockwise → anticlockwise 330° (US6-S1);
  - the same after a power cycle and a new epoch → anticlockwise 330° (US6-S2);
  - a 40 s slew planned to the place 40 s later, within 1 s (US6-S3).
- [ ] T035 [US6] In a new `crates/finder/tests/goto_sim.rs`, write failing paused-time
  tests from `--sim-start 2026-01-16T02:00:00Z` at 40° N, 75° W, minimum altitude 15°,
  sampling the simulator's true eyepiece direction every 0.1 s of virtual time:
  - a target at 16° keeps the true altitude at or above 15° (US6-S5);
  - the dusk slew stays 15° clear of the Sun and completes (US6-S7);
  - M42, M31, M45, M81 and M1 after a second sync each end within 1′ in at most 5
    corrections (US6-S8);
  - a 0.5° slip on every correction fails after 5 with the last error, axes stopped
    (US6-S9);
  - a 3° push by hand opens an epoch, and the GoTo still ends within 1′ (US6-S10);
  - a foreign-origin, bodiless `POST /api/stop` answers 200, commands the stop within
    0.1 s, leaves both axes at rest within 1.5 s, and reports the observer (US6-S11);
  - silent heartbeats stop the axes 1.0 ± 0.1 s later and report a lost link (US6-S12);
  - the Sun reaching −0.83° stops the axes within 1 s and reports the Sun guard (US6-S13);
  - a GoTo to M42 reports slewing, solving, each correction by number, and done with the
    final error in arcseconds (US6-S16).
- [ ] T036 [US6] In a new `crates/finder/tests/goto_api.rs`, write failing route tests:
  - a target at 15.2° → 409 naming the minimum altitude, and no axis moves (US6-S4);
  - the Sun at +5° → 409 "Sun is up", and no axis moves (US6-S6);
  - M8 at 02:00 → 409 below the horizon, and no axis moves (US6-S14);
  - no mount, or no sync → 409 naming each reason, and no axis moves (US6-S15);
  - a foreign `Origin` on `POST /api/goto` → 403, and no axis moves (US6-S17);
  - first, with the simulated mount connected, an accepted GoTo moves an axis, so "no axis
    moves" is asserted against a mount that can (plan.md § Plan traps, trap 11).

### Implementation

- [ ] T037 [US6] Implement the planner in `crates/finder/src/goto.rs`, and add
  `pub mod goto;` to `crates/finder/src/lib.rs`:
  - two arrival-time iterations, refraction added, and the model inverse;
  - the ±360° candidate within one revolution of the first sync's unwrapped azimuth;
  - the 0.5° overshoot and approach from below, and the minimum-altitude refusal;
  - the Sun-up refusal.
- [ ] T038 [US6] Implement the Sun guard on the path in `crates/finder/src/goto.rs`:
  simulate at 0.5° steps, try altitude first, azimuth first, then the dogleg through
  `max(current, target, Sun + 20°)`, and refuse if none keeps 15° clear.
- [ ] T039 [US6] Implement the state machine in `crates/finder/src/goto.rs`:
  `Plan → Slew → Settle → Solve → (Done | Correct → Slew …) | Failed | Halted`, with
  settle as both axes stopped plus 0.5 s, at most 5 corrections, the console's 1 s
  lost-link rule, the 50 ms Sun tick, and `stop_all` with the reason on every halt.
- [ ] T040 [US6] In `crates/finder/src/api.rs`, add `POST /api/goto` behind 012's
  same-origin check with its 409 reasons, and `POST /api/stop` with no body and no
  same-origin check. Add `goto` progress to `GET /api/session`. Document both routes in
  the crate docs and `crates/finder/README.md`.
- [ ] T041 [US6] Run the `lint` and `test` gates. Confirm that every
  existing test is unchanged and that no field or option turns off the Sun guard. Commit
  the green story (FR-016, FR-017, FR-018, FR-019, FR-020, FR-021; US6-S1, US6-S2, US6-S3,
  US6-S4, US6-S5, US6-S6, US6-S7, US6-S8, US6-S9, US6-S10, US6-S11, US6-S12, US6-S13,
  US6-S14, US6-S15, US6-S16, US6-S17).

## Phase 7: User Story 7 — the second sync and the model routes (Priority: P2)

### Tests first

- [ ] T042 [US7] In a new `crates/finder/tests/second_sync_sim.rs`, write a failing
  paused-time test: with one sync at azimuth 120° on the simulator (base 1° toward 130°),
  `POST /api/model/second-sync` slews at least 90° away in azimuth, takes a sync there,
  and fits the base tilt within 1′ of the simulator's (US7-S1).
- [ ] T043 [US7] In a new `crates/finder/tests/model_api.rs`, write failing route tests:
  - `POST /api/model/reset` for the night clears syncs, zero points and base tilt and
    keeps the fixed terms, and for the fixed terms clears them (US7-S2);
  - a foreign `Origin` on either route → 403, no axis moves, and the model is unchanged
    (US7-S3).

### Implementation

- [ ] T044 [US7] Implement the second sync in `crates/finder/src/goto.rs`: the nearest
  pointable spot at least 90° in azimuth from every sync of the night and outside the Sun
  guard, a GoTo with no corrections, then the sync.
- [ ] T045 [US7] In `crates/finder/src/api.rs`, add `POST /api/model/second-sync` and
  `POST /api/model/reset` (`{"what": "night" | "fixed"}`) behind 012's same-origin check.
  Document them in the crate docs and `crates/finder/README.md`.
- [ ] T046 [US7] Run the `lint` and `test` gates, with every existing test unchanged.
  Commit the green story (FR-022, FR-023; US7-S1, US7-S2, US7-S3).

## Phase 8: User Story 8 — GoTo and Stop on the page, and Setup's section (Priority: P2)

### Tests first

- [ ] T047 [US8] Record session fixtures from the simulator into
  `crates/finder/web/src/test/fixtures/` (a GoTo to M42 in each state, two syncs from base
  1° toward 130°, the second sync slewing). Write failing Vitest tests:
  - `src/components/GoToControl.test.tsx`: disabled with the reason for below the horizon,
    the Sun up and no sync, with Stop still shown while moving (US8-S1); the slew, each
    correction and "on target" with the final error (US8-S2);
  - `src/components/StopButton.test.tsx`: in Setup's top bar while the second sync slews,
    sending `POST /api/stop` (US8-S3); enabled and outside the sheet's dialog and any
    inert or `aria-hidden` subtree, with the sheet open or closed (US8-S4);
  - `src/components/MountSection.test.tsx`: "base tilt 1.0° toward 130°" within 0.1° and
    2°, with the RMS and "2 syncs" (US8-S5); link, board, second sync and both resets,
    each sending its route (US8-S6).

### Implementation

- [ ] T048 [US8] Add the session's `goto`, `model` and `mount` to
  `crates/finder/web/src/api.ts`. Implement `GoToControl` in
  `src/components/GuidanceBlock.tsx`, and `StopButton` in `src/components/TopBar.tsx`,
  which `src/App.tsx` renders above the console and Setup alike and hands the session's
  `mount` and `goto`, whenever the mount may be
  moving: never disabled, never inside a sheet, at least 44 px tall. Both render nothing
  when the session has no `mount` (plan.md § Plan traps, trap 9).
- [ ] T049 [US8] Implement `MountSection` on `src/components/SetupPage.tsx`.
- [ ] T050 [US8] Record GoTo, Stop and the mount and model section as built in
  `DESIGN.md` and `.impeccable/design.json` (US8-S7).
- [ ] T051 [US8] In `crates/finder/web`, run `npm ci`, `npm run typecheck`,
  `npm run format:check` and `npm test`, then `npm run build`. Commit the rebuilt
  `web/dist`, and confirm that `git status --porcelain -- dist` lists nothing, with
  `package.json` dependencies unchanged (US8-S8).
- [ ] T052 [US8] Run the `lint` and `test` gates, with every existing Vitest file
  unchanged and `e2e/session.spec.ts` green. Commit the green story (FR-024, FR-025,
  FR-026; US8-S1, US8-S2, US8-S3, US8-S4, US8-S5, US8-S6, US8-S7, US8-S8).

## Phase 9: User Story 9 — a night on the simulator, end to end (Priority: P3)

### Tests first

- [ ] T053 [US9] In a new `crates/finder/tests/night_sim.rs`, write the paused-time night
  from `--sim-start 2026-01-16T02:00:00Z` at 40° N, 75° W: first sync, second sync, five
  GoTos, a hand push and a GoTo, a Stop mid-slew, and a dropped link mid-slew. Every GoTo
  ends within 1′, and every halt leaves both axes at rest (US9-S1). It runs in the `test`
  gate's `cargo test --workspace --all-targets --all-features --locked`.
- [ ] T054 [US9] In a new `crates/finder/web/e2e/goto.spec.ts`, write the Playwright tests
  for 013's `simulate` project:
  - a GoTo to M42 from the page, with the block's progress and the target reticle within
    1′ of the aim point (US9-S2);
  - Stop mid-slew, with the axes stopped and the block saying the observer stopped it
    (US9-S3);
  - the second sync from Setup, with the base tilt appearing (US9-S4);
  - the night pixel check while GoTo runs (US9-S5);
  - Stop at least 44 px tall and the element at its centre, on the console and Setup
    (US9-S6).

### Implementation

- [ ] T055 [US9] In `crates/finder/web/playwright.config.ts`, add `e2e/goto.spec.ts` to
  013's `simulate` project's match and to the `default` project's ignore, and set
  `workers: 1` on the `simulate` project (owner approval 3; plan.md § Plan traps, trap
  15), changing nothing else (US9-S7).
- [ ] T056 [US9] Run the `lint` and `test` gates, whose `test` gate runs `CI=1 npm run e2e`
  in `crates/finder/web`, with every earlier end-to-end test unchanged and green. Commit
  the green story (FR-027, FR-028; US9-S1, US9-S2, US9-S3, US9-S4, US9-S5, US9-S6,
  US9-S7).

## Execution Order

The stories land in this order:

```text
US1 → US2 ─┬→ US3
           └─────┐
US4 ─────────────┴→ US5 → US6 → US7 → US8 → US9
```

Every arrow is a merge edge: the later story starts from a `main` that holds the earlier
one.
- **US4** runs beside US1 and US2, because it touches only
  `crates/finder/src/ephemeris.rs`.
- **US3** runs beside US5–US9, because it touches only `crates/scope-node` and
  `Cargo.lock`.
- **`[P]`** marks nothing here: every task within a story shares a file with another.
- **Out of scope for every story:** `crates/math-core`, `proto/`, `scripts/ci/`,
  `.github/`, `fixtures/`, existing tests, and Playwright servers and projects beyond
  owner approval 3's three lines.
- **Outside the factory, after landing:** the Impeccable review (after US8), and the
  maintainer's checks at the telescope with the musl cross-build (spec § Assumptions).
