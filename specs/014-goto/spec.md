---
state: draft
depends_on_landed: [012-targets-and-push-to, 013-live-finder]
# Converted 2026-10-28 from the reviewed OpenSpec change feat-14 (#102). This trio is
# now the feature's only specification. It stays draft until the owner has reviewed
# it and given the approvals listed under "Owner approvals". Flipping it to `ready`
# is the act that dispatches it.
# Refined 2026-10-31 against main at 2a2f9d2 (013's US1, US8 and US9 landed, US2 in the
# merge queue) and the factory's gates as 013 runs them (plan.md § Plan traps and
# § Refinement Record). It stays draft until 013 has landed and the prerequisites in
# plan.md are met; the paths marked "013's" are re-read against 013 as landed then.
---

# Feature Specification: GoTo

**Feature Branch**: `014-goto`

**Created**: 2026-10-28

**Status**: Draft

**Input**: Live solving (013) tells the console where the 16-inch Dobsonian points and
where the target is, but the observer still pushes the tube. The mount has motors, and by
ADR-0001 the scope node drives its two axes. This feature makes the console bring the
target to the aim point itself, with no hand controller and no star alignment:

- every solve while the mount is connected is a sync;
- a pointing model is fitted from the syncs;
- the console slews, solves and corrects until the target is within 1′.

It adds a mount port to the scope node, with a Sky-Watcher adapter and a simulated mount
behind it. It also adds Stop and a heartbeat watchdog, apparent altitude, the pointing
model, closed-loop GoTo, and GoTo and Stop on the page. Tracking after a GoTo is the next
feature (015).

## Clarifications

### Session 2026-10-28

- Q: What does a story read besides this trio? → A: Nothing about the feature. The trio is
  self-contained. `spec.md` holds every requirement and scenario. `plan.md` holds every
  design decision, the interfaces, the shapes existing tests pin, and the alternatives
  rejected. A story that would change a decision escalates rather than re-deciding it.
- Q: How is the feature sliced for the factory? → A: Into nine stories, each a vertical
  slice that lands its own tests:
  - the mount port and the simulated mount;
  - Stop, the watchdog, axis positions on frames and the mount on the link;
  - the Sky-Watcher adapter;
  - apparent altitude;
  - the pointing model;
  - GoTo;
  - the one-tap second sync and the model routes;
  - the page;
  - the end-to-end suite.

  The node side is built before the console's model, the model before GoTo, GoTo before
  the page, and the page before its end-to-end tests. The Sky-Watcher adapter runs beside
  the console stories, because nothing in the factory drives real hardware.
- Q: How is any of this proven without the telescope? → A: On the simulator, in virtual
  time. Every GoTo, sync and model behaviour is a Rust integration test that runs the
  simulator and the console in one paused-time tokio runtime, over the in-process link. A
  night of slews costs milliseconds of CPU, and the coverage floor (Rust only) counts it.
  The Sky-Watcher adapter is proven against a byte-level emulator of the mount. No story
  opens a serial device or moves real hardware.
- Q: What about the checks at the telescope? → A: They are outside the factory, because a
  filled-in checklist is not evidence the judge can see in a diff. After the spec lands,
  the maintainer works through these:
  - `scope-node --probe-mount` with the USB–TTL cable, recording the board, firmware and
    counts per revolution;
  - Stop and the watchdog with the tube unloaded;
  - the second sync;
  - GoTos to five bright targets, each ending within 1′ in the eyepiece.
- Q: How does Impeccable take part? → A: `DESIGN.md`, `.impeccable/design.json` and the
  surface briefs under `.impeccable/surfaces/` are the design system the page story builds
  to. The Impeccable review itself (shape → craft → finish review) is outside the factory,
  because a "ship verdict" is not evidence the judge can see in a diff. US8 records GoTo,
  Stop and the mount and model section in `DESIGN.md` as built, and the maintainer runs
  the review after US8 lands.
- Q: Who approves the new dependency and the open Stop? → A: The owner, by reviewing this
  spec and flipping it to `ready`. The approvals are listed below so that the flip is an
  informed act.

### Session 2026-10-31 (refinement against main and 013)

- Q: How is stop all exempt from the token, when 013's `TokenInterceptor` guards the whole
  `ScopeNode` service? → A: Stop all is the one call of a second service, `ScopeNodeStop`,
  in the same proto file and package, served beside `ScopeNode` with no interceptor.
  tonic's server interceptor sees only a request's metadata, never which call it is, so
  one interceptor cannot exempt one call (plan.md § Decisions).
- Q: How does the node on the telescope know it has a mount? → A: From an optional
  `[mount]` table in its config file, naming the serial device. Without it the node runs
  with no mount, and its frames carry no axis positions (FR-009, US3-S16).
- Q: Which serial code? → A: The owner chooses at the flip (owner approval 1). Since 013,
  `rustix` is already a dependency of `crates/scope-node`, so its `termios` feature is a
  path with no new crate, beside `serialport`.
- Q: What do the new session fields read before they have a value? → A: They are left out
  of the body, as 013's are, so the whole-body test of a fresh session stays as it is.

## Owner approvals

Flipping this spec to `ready` approves the following. No existing requirement is modified,
no existing test or `ci.yml` changes, and no Playwright server or project changes beyond
approval 3's two lines.

1. **The serial device's code**, one of two, which the owner names at the flip (plan.md
   § Prerequisites, item 2):
   - `serialport` with `default-features = false` (MPL-2.0, in `deny.toml`'s allow-list,
     no libudev), a new crate in `crates/scope-node`; or
   - `rustix`'s `termios` and `fs` features. `rustix` is already a dependency of
     `crates/scope-node` since 013 (its `time` feature, for the node's clock), so this
     adds no crate.

   If `serialport` is chosen and does not build for `aarch64-unknown-linux-musl` at
   bring-up, `rustix`'s termios replaces it behind the same interface (FR-009).
2. **Stop is open to anyone.**
   - `POST /api/stop` skips the same-origin check that 012 put on every route that changes
     the session. 012 already left room for a route that only stops the telescope.
   - The link's stop-all call needs no token. It is its own service, `ScopeNodeStop`,
     served without the token interceptor.

   So anyone who can reach the console or the node can halt the telescope, and no one else
   can move it (FR-006, FR-020).
3. **Two lines of Playwright config.** 013's `simulate` project matches only
   `e2e/simulate.spec.ts`, and its `default` project ignores that file by name (013
   plan.md, User Story 10). US9 adds `e2e/goto.spec.ts` to the `simulate` project's match
   and to the `default` project's ignore. The `session` project matches only
   `e2e/session.spec.ts` and is unchanged. No server, project or existing test changes
   (FR-028). Re-read against 013's US10 as landed, at the flip.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - The mount port, and a simulated mount behind it (Priority: P1)

As the scope node, I drive a mount only through a mount port of two axes. Each axis does
four things:
- it reports its position and capabilities;
- it goes to a position;
- it moves at a signed rate;
- it stops.

As every later story, I have a simulated mount behind that port with what a real Dob has:
- a base 1° off level and axis zero points offset from north and level;
- 0.3° of backlash on each axis;
- a 3.4°/s top rate, with acceleration and deceleration of 3°/s².

I can push the tube by hand where the encoders miss it, make an axis slip, power-cycle the
mount, and read its true eyepiece direction in tests.

**Why this priority**: Every later story drives this port and proves itself against this
simulated mount. A mount port that leaks one mount's details makes the next mount a
rewrite rather than an adapter.

**Independent Test**: `cargo test -p scope-node --locked` runs the port's unit tests on a
fake axis. `cargo test -p simulator --locked` runs `crates/simulator/tests/mount.rs` under
`#[tokio::test(start_paused = true)]`, against the simulated mount through the port.

**Acceptance Scenarios**:

1. **Given** the simulated mount, **When** a test in `crates/simulator/tests/mount.rs`
   asks its axes for their capabilities through the port, sends them to positions, moves
   them at rates and stops them, **Then** it asserts four things for each axis:
   - it reports its counts per revolution and its top rate;
   - it reaches each position within one count;
   - it holds each rate within 0.1%;
   - it stops.
2. **Given** a simulated axis, **When** a test sends it from −1° up to 0°, on up to +1°,
   and back down to 0°, **Then** its axis position reads 0° both times it passes there,
   and the true directions at those two moments differ by 0.3° within 0.01°.
3. **Given** a simulated axis, **When** a test sends it 90° away, **Then** it never
   exceeds 3.4°/s, and it arrives 27.6 s ± 0.1 s of virtual time later (90/3.4 + 3.4/3,
   accelerating and slowing at 3°/s²).
4. **Given** a simulated axis turning at 3.4°/s, **When** a test tells it to stop,
   **Then** it is at rest 1.13 s later (±0.05 s), having slowed at 3°/s².
5. **Given** a simulated base tilted toward axis azimuth 0°, **When** a test points the
   tube at axis altitude 45° at axis azimuths 0° and 180°, **Then** the true altitudes
   differ by 2° within 0.01°.
6. **Given** the simulated mount connected, **When** a test pushes the tube 3° in azimuth
   by hand, **Then** the true direction moves 3° (within 0.01°) and both axis positions
   are unchanged.
7. **Given** a slip of 0.5° per move set on the azimuth axis, **When** a test sends it 10°
   away, **Then** its axis position reads 10° and its true direction has turned 9.5°
   (within 0.01°).
8. **Given** the simulated mount after some moves, **When** a test power-cycles it,
   **Then** both axis positions read the mount's power-on zero and the true direction is
   unchanged.
9. **Given** the story's diff, **When** it is read, **Then** three things hold:
   - the port is a trait in `crates/scope-node`, and the simulated mount implements it in
     `crates/simulator`;
   - `crates/scope-node` still depends on `star-detection` and `scope-link` only;
   - `crates/math-core` is untouched.

---

### User Story 2 - Stop wins, a watchdog stops a silent link, and the mount travels the link (Priority: P1)

As the scope node, I carry out a stop at once and drop every mount command that reached me
before it. When 1 s passes without the console's heartbeat, I stop both axes myself. While
a mount is connected, each star frame carries both axes' positions at the start and end of
its exposure.

As the scope link, I carry the mount port's calls. Every call needs the token except stop
all, so anyone can halt the telescope and no one else can move it.

**Why this priority**: A telescope that keeps moving when the observer says stop, or when
the laptop's Wi-Fi drops, is dangerous. Axis positions on frames are what turn a solve into
a sync.

**Independent Test**: `cargo test -p scope-node --locked` and
`cargo test -p scope-link --locked` run the new tests in paused virtual time:
- `crates/scope-node/tests/stop_and_watchdog.rs`;
- `crates/scope-node/tests/frame_positions.rs`;
- `crates/scope-link/tests/mount_calls.rs`.

They run the real node service over the simulated mount and the in-process link.

**Acceptance Scenarios**:

1. **Given** a goto queued behind another command on the simulated mount, **When** a stop
   arrives, **Then** a test asserts three things:
   - the axes stop;
   - the queued goto is not carried out;
   - a goto sent after the stop's reply is carried out.
2. **Given** both axes moving in virtual time, **When** the console's heartbeats stop,
   **Then** a test asserts both axes are commanded to stop 1.0 ± 0.1 s after the last
   heartbeat.
3. **Given** a rate the watchdog stopped, **When** heartbeats resume, **Then** a test
   asserts the axis stays at rest until a new move is requested, and that the new move is
   carried out.
4. **Given** the simulated azimuth axis turning at a steady rate, **When** a 200 ms
   exposure is taken, **Then** a test asserts that the frame's start and end positions
   differ by that rate times the exposure, within two counts. Both are read on the node's
   clock.
5. **Given** a client with no token, **When** it asks the node to move an axis, **Then**
   the call fails with UNAUTHENTICATED and no axis moves.
6. **Given** a client with no token and both axes moving, **When** it calls stop all,
   **Then** the call succeeds and both axes stop.
7. **Given** the console's link client and the simulated mount, **When** a test describes
   the axes and reads their state through the link, **Then** it gets each axis's counts
   per revolution and top rate, and its position, rate and moving flag, exactly as the
   simulator holds them.
8. **Given** the story's diff, **When** it is read, **Then** three things hold:
   - the changes to `crates/scope-link/proto/scope_node/v1/scope_node.proto` only add
     calls, messages and new field numbers;
   - its comments hold no `[...]`;
   - every existing test in `crates/scope-link` and `crates/scope-node` is unchanged.

---

### User Story 3 - The Sky-Watcher adapter, proven against a byte-level emulator (Priority: P2)

As the scope node on the real telescope, I drive the Sky-Watcher Flextube 400P SynScan
through its motor-controller (`:`) protocol, over a USB–TTL cable in the mount's
hand-controller jack:
- I find the baud rate and initialise the board;
- I switch the auxiliary encoders off;
- I set rates and go to positions;
- I stop an axis before reversing it;
- I retry a "motor not stopped" refusal once.

A byte-level emulator stands in for the mount in every test, and
`scope-node --probe-mount DEVICE` runs the first-test sequence at bring-up.

**Why this priority**: This is the only path to the real mount. It is P2 because nothing
in the factory drives real hardware: every console story runs on the simulated mount.

**Independent Test**: `cargo test -p scope-node --locked` runs three sets of tests:
- the codec's unit tests in `crates/scope-node/src/mount/skywatcher/codec.rs`;
- the emulator's own tests;
- `crates/scope-node/tests/skywatcher.rs`, which tests the adapter against the emulator.

No serial device is opened.

**Acceptance Scenarios**:

1. **Given** positions 0, +1 and −1, **When** a test encodes them, **Then** they are
   `000080`, `010080` and `FFFF7F`, and each decodes back to itself.
2. **Given** every 24-bit value, **When** a test encodes and decodes each, **Then** every
   one round-trips.
3. **Given** the emulator, **When** a test sends it each command it implements, **Then**
   it asserts each reply against the command set. The commands are `:e`, `:a`, `:b`, `:g`,
   `:f`, `:j`, `:q`, `:F`, `:W`, `:G`, `:S`, `:H`, `:M`, `:J`, `:I`, `:K` and `:L`, each
   answered `=data` or `!code`.
4. **Given** the emulator, **When** a test sends axis 1 to position +0x123, **Then** the
   emulator receives `:S1230180\r` among the goto's commands, and the axis reaches +0x123.
5. **Given** an emulator told to refuse `:S`, **When** a test sends an axis to a position,
   **Then** the adapter falls back to `:H` (relative) and `:M` (brake point), and the axis
   reaches the position.
6. **Given** the emulator answering a command with `!2`, **When** the adapter sends it,
   **Then** the adapter does three things:
   - it stops that axis;
   - it waits until `:f` reports it stopped;
   - it sends the command again, once.
7. **Given** an axis moving at +1°/s, **When** it is asked to move at −1°/s, **Then** the
   emulator sees it stopped and reported stopped before the new direction is set.
8. **Given** an emulator that answers only at 115200 baud, **When** the adapter connects,
   **Then** it finds the board there after no answer at 9600, and reports the board's
   version.
9. **Given** an emulator that echoes every byte like a single-wire bus, **When** the
   adapter sends commands, **Then** it skips its own echo and reads each reply correctly.
10. **Given** an emulator reporting motor firmware below 3.39, **When** the adapter
    connects, **Then** it accepts the board and reports a warning that smooth
    variable-rate tracking may not work.
11. **Given** an emulator whose `:q` reports auxiliary-encoder support, **When** the
    adapter connects, **Then** it sends `:F3`, reads `:a`, `:b` and `:g`, and switches the
    auxiliary encoders off with `:W`.
12. **Given** the emulator's timer frequency and counts per revolution, **When** a test
    asks for rates of 1°/s and 0.01°/s, **Then** it asserts three things:
    - the `:I` step period is timer frequency × 360 / rate / counts per revolution, times
      the high-speed ratio in fast mode;
    - fast mode (`:G` codes 0 and 3) is used only above 128× sidereal;
    - the emulated axis turns at each rate within 0.1%.
13. **Given** an emulator that does not acknowledge `:K`, **When** the adapter stops all,
    **Then** it sends `:K` on both axes and then `:L` on the axis that did not acknowledge.
14. **Given** the probe run against the emulator's byte stream, **When** a test calls the
    function behind `scope-node --probe-mount`, **Then** it asserts the commands go out in
    the order `:e1`, `:a1`, `:b1`, `:g1`, `:q1010000`, `:j1`, and that the printed report
    names the board version, counts per revolution, timer frequency, high-speed ratio,
    auxiliary-encoder support and axis position.
15. **Given** the story's diff, **When** it is read, **Then** `crates/scope-node/Cargo.toml`
    adds only the serial path the owner chose (owner approval 1): `serialport` with
    `default-features = false`, or `rustix`'s `termios` and `fs` features. Opening the
    device is the only code path no test reaches.
16. **Given** a config file whose `[mount]` table names `device = "/dev/ttyUSB0"`, **When**
    `scope-node --check-config` runs on it, **Then** a CLI test in
    `crates/scope-node/tests/mount_config.rs` asserts that it prints the mount's device and
    exits 0. **And** with no `[mount]` table it prints that no mount is configured and
    exits 0, and a `[mount]` table with an unknown key, or with no `device`, exits nonzero
    naming the key.

---

### User Story 4 - Apparent altitude: refraction added and removed (Priority: P1)

As the GoTo planner and the pointing model, I ask `finder::ephemeris` for two things:
- an apparent altitude from a true one, by Saemundsson;
- a true altitude from an apparent one, by Bennett.

Both use the standard atmosphere. The finder console's own altitude and azimuth stay
geometric.

**Why this priority**: Near the horizon, refraction lifts a star by more than half a
degree. A model fitted without removing it, or a slew planned without adding it, misses by
far more than 1′.

**Independent Test**: `cargo test -p finder --locked ephemeris` runs the new in-module
tests in `crates/finder/src/ephemeris.rs`. `sky.rs`'s
`alt_az_matches_meeus_example_13b` passes unchanged.

**Acceptance Scenarios**:

1. **Given** an apparent altitude of 0°, **When** a test computes Bennett's refraction,
   **Then** it is 34.5′ ± 0.1′.
2. **Given** a true altitude of 45°, **When** a test computes Saemundsson's refraction,
   **Then** it is 1.01′ ± 0.01′.
3. **Given** every true altitude from 0° to 90° in 0.1° steps, **When** a test adds
   refraction and then removes it, **Then** each comes back within 0.1′.
4. **Given** Meeus's example 13.b, **When** it is computed after this change, **Then** the
   altitude is still 15.1249° within 0.01°, with no refraction in it, and
   `alt_az_matches_meeus_example_13b` is unchanged.
5. **Given** the story's diff, **When** it is read, **Then** `sky::alt_az` is unchanged and
   refraction lives only in `finder::ephemeris`.

---

### User Story 5 - Every solve is a sync, and a pointing model is fitted from them (Priority: P1)

As the console with the mount connected, I take each solved star frame from the scope node
as a sync, when both axes were nearly still during its exposure. A sync is the apparent
altitude and azimuth at the aim point at mid-exposure, with the axis positions
interpolated to the same moment.

From the syncs I fit a pointing model:
- the zero points from one sync;
- the base tilt from two syncs at least 90° apart in azimuth;
- a new zero-point epoch when a sync shows a slip;
- the fixed terms from about 20 well-spread syncs.

The session keeps the night's model until the next local noon, and keeps the fixed terms
until they are reset.

**Why this priority**: The model replaces the hand controller's star alignment. Without
it, the console cannot turn a target into axis positions.

**Independent Test**: `cargo test -p finder --locked` runs two sets of tests:
- the unit tests in `crates/finder/src/model.rs`, which fit synthetic syncs from known
  terms;
- `crates/finder/tests/model_sim.rs`, which runs the simulator and the console in one
  paused-time runtime over the in-process link.

**Acceptance Scenarios**:

1. **Given** the simulated mount still, **When** frames solve at 2 a second for 10 s,
   **Then** a test asserts that one sync is kept, and that its axis positions equal the
   simulator's at mid-exposure within one count.
2. **Given** a frame exposed while an axis turns at 2°/s, **When** it solves, **Then** a
   test asserts it is solved and charted but not taken as a sync.
3. **Given** the mount connected, **When** a frame is uploaded to `/api/solve`, **Then** a
   test asserts it is solved and becomes the latest pointing, but is not taken as a sync.
4. **Given** a frame exposed while the azimuth axis tracks from count 1000 to count 1060,
   **When** it is taken as a sync, **Then** the sync's azimuth axis position is count 1030.
5. **Given** syncs made from synthetic axis positions with IA 2°, IE −0.5°, AN 0.6° and AW
   −0.8°, two of them 90° apart in azimuth, **When** the model is fitted, **Then** the
   fitted IA, IE, AN and AW each equal the synthetic value within 1″.
6. **Given** the following setup, **When** the model's error there is computed, **Then**
   it is 2.00° ± 0.01°, twice the base tilt, as the pointing-model research predicts:
   - a base tilted 1° toward azimuth 0°;
   - one sync, at azimuth 0°, altitude 45°;
   - a model predicting azimuth 180°, altitude 45°.
7. **Given** the simulator's base 1° off level toward azimuth 130°, with one sync held at
   azimuth 120°, **When** the second sync is taken at azimuth 250°, **Then** no new epoch
   is opened, and the base tilt is fitted within 1′ of the simulator's.
8. **Given** a model with its base tilt fitted, **When** a sync arrives 0.6° from the
   prediction, **Then** a new zero-point epoch opens, with IA and IE fitted again from that
   sync and the base tilt and fixed terms unchanged. **And** a sync 0.4° from the
   prediction opens none.
9. **Given** 24 synthetic syncs spanning 200° of azimuth and 40° of altitude that carry
   CA 10′, NPAE 3′, TF 5′ and the finder camera rotated 4° about its axis on the tube,
   besides the zero points and base tilt, **When** the model is fitted, **Then** CA, NPAE,
   TF and the camera's rotation on the tube are each fitted within 1″.
10. **Given** only 19 such syncs, **When** the model is fitted, **Then** no fixed term is
    fitted, and the model says how many more syncs it needs.
11. **Given** the site 40° N, 75° W and syncs made at 02:00 UT on 2026-01-16, **When** the
    binary restarts at 05:30 UT that night, past local midnight, **Then** the syncs, zero
    points and base tilt are as before the restart.
12. **Given** that session, **When** it is read after the Sun's next upper transit at the
    site (about 17:10 UT), **Then** the night's syncs, zero points and base tilt are gone,
    and the fixed terms remain.
13. **Given** a model holding fixed terms, **When** they are reset, **Then** the model
    holds none, and fits them again once it has enough syncs.
14. **Given** the story's diff, **When** it is read, **Then** three things hold:
    - the fit (Levenberg–Marquardt with Cholesky-solved normal equations) and the Newton
      inverse are written in-crate, with no new dependency;
    - `crates/math-core` is untouched;
    - every existing `finder` test is unchanged.

---

### User Story 6 - GoTo: plan, guard, slew, solve and correct to 1′, or halt (Priority: P1)

As the observer with a pointable target and a sync, I send `POST /api/goto`. Each slew is
planned in four steps:
- to the target's apparent place at the expected arrival time;
- through the pointing model;
- keeping the cable within one revolution of the night's first sync;
- approaching each axis from below, never under the minimum altitude, never near the Sun.

The console then slews at top speed, settles, solves at the aim point, and corrects at
most 5 times until the target is within 1′. Stop, a lost link or the Sun guard halt the
mount, and the session reports the GoTo's progress and how it ended.

**Why this priority**: This is the feature's purpose. Everything before it exists so that
this story can bring the target to the eyepiece safely.

**Independent Test**: `cargo test -p finder --locked` runs three sets of tests:
- the planner's unit tests in `crates/finder/src/goto.rs`;
- `crates/finder/tests/goto_sim.rs`, which runs the simulator (base 1° off level, zero
  points offset, 0.3° backlash) and the console in one paused-time runtime from
  `--sim-start 2026-01-16T02:00:00Z` at 40° N, 75° W with a minimum altitude of 15°, and
  samples the simulator's true eyepiece direction;
- `crates/finder/tests/goto_api.rs`, for the routes.

**Acceptance Scenarios**:

1. **Given** the azimuth axis turned 350° clockwise since the first sync and a target 30°
   clockwise of where the telescope points, **When** the slew is planned, **Then** it
   turns anticlockwise 330°, not clockwise 30°.
2. **Given** the following sequence, **When** the slew is planned, **Then** it turns
   anticlockwise 330°:
   - the telescope turned 350° clockwise since the first sync;
   - the simulated mount power-cycled (its counts reset), and a sync opening a new
     zero-point epoch;
   - a target 30° clockwise of where the telescope points.
3. **Given** a slew expected to take 40 s, **When** it is planned, **Then** it is planned
   to the target's apparent place 40 s after it starts, within 1 s of planning time.
4. **Given** a minimum altitude of 15°, **When** a GoTo is asked for a target at 15.2° on
   the simulator, **Then** it is refused with 409 and a JSON `error` saying the approach
   would go below the minimum altitude, and no axis moves.
5. **Given** a minimum altitude of 15°, **When** a GoTo is made to a target at 16° on the
   simulator, **Then** the simulator's true eyepiece altitude, sampled every 0.1 s of
   virtual time, never goes below 15°.
6. **Given** the Sun's centre 5° above the horizon, **When** a GoTo is asked for, **Then**
   it is refused with 409 and a JSON `error` saying the Sun is up, and no axis moves.
7. **Given** the following positions, **When** a GoTo is made, **Then** it completes, and
   the simulator's true eyepiece direction, sampled every 0.1 s of virtual time, never
   comes within 15° of the Sun:
   - the Sun 10° below the western horizon;
   - the telescope pointing 3° up and 30° south of the Sun's azimuth;
   - the target 40° up and 30° north of it.
8. **Given** a second sync made, **When** GoTos to M42, M31, M45, M81 and M1 are made in
   turn on the simulator, **Then** each ends with the simulator's true eyepiece direction
   within 1′ of its target, using at most 5 corrections.
9. **Given** the simulated mount made to slip 0.5° on every correction, **When** a GoTo is
   made, **Then** after 5 corrections it ends as failed, reporting the last error, and the
   axes are stopped.
10. **Given** the simulated tube pushed 3° by hand with the auxiliary encoders off, **When**
    a frame solves and a GoTo is then made, **Then** a new zero-point epoch is opened at
    that sync, and the GoTo still ends with the true eyepiece within 1′ of the target.
11. **Given** a slew under way, **When** `POST /api/stop` is sent with a foreign `Origin`
    and no body, **Then** a test asserts four things:
    - the answer is 200;
    - the node commands both simulated axes to stop within 0.1 s of virtual time;
    - both are at rest within 1.5 s;
    - the GoTo reports that the observer stopped it.
12. **Given** a slew under way, **When** the console stops sending heartbeats, **Then** the
    node commands both axes to stop 1.0 ± 0.1 s after the last heartbeat, and the console
    reports the GoTo halted by a lost link.
13. **Given** a slew under way, **When** the virtual clock passes the moment the Sun's
    centre reaches −0.83°, **Then** both axes are commanded to stop within 1 s of that
    moment, and the GoTo reports the Sun guard.
14. **Given** 40° N, 75° W at 2026-01-16 02:00 UT, **When** a GoTo to M8 is asked for,
    **Then** the answer is 409 saying M8 is below the horizon, and no axis moves.
15. **Given** no mount connected, or a mount connected but no sync yet that night,
    **When** a GoTo is asked for, **Then** each answer is 409 with a JSON `error` naming
    the reason, and no axis moves.
16. **Given** a GoTo to M42 running on the simulator, **When** a test reads the session
    throughout, **Then** it reports the GoTo in this order:
    - slewing;
    - solving;
    - each correction, by number;
    - done, with the final error in arcseconds.
17. **Given** a `POST /api/goto` that carries a foreign `Origin`, **When** it is sent,
    **Then** it is answered 403 with a JSON `error` by 012's same-origin rule, and no axis
    moves.

---

### User Story 7 - The one-tap second sync, and resetting the model (Priority: P2)

As the observer after the night's first sync, I tap once. The console slews to a pointable
spot at least 90° in azimuth from the night's syncs, outside the Sun guard, solves there
and takes the sync, so the base tilt can be fitted. I can also reset the night's model, or
the fixed terms, from the console.

**Why this priority**: Without a second sync, every GoTo carries up to twice the base
tilt's error until a correction removes it. One tap is the fastest way to a fitted base
tilt. It is P2 because it consumes US6's slew machinery.

**Independent Test**: `cargo test -p finder --locked` runs
`crates/finder/tests/second_sync_sim.rs`, which uses the simulator in paused virtual time,
and `crates/finder/tests/model_api.rs`, for the routes.

**Acceptance Scenarios**:

1. **Given** the simulator (base 1° off level toward azimuth 130°) holding one sync at
   azimuth 120°, **When** `POST /api/model/second-sync` is sent, **Then** a test asserts
   three things:
   - the mount slews to a pointable spot whose azimuth differs from 120° by at least 90°;
   - a sync is taken there;
   - the base tilt is fitted within 1′ of the simulator's.
2. **Given** a model holding the night's syncs and fixed terms, **When**
   `POST /api/model/reset` is sent for the night, and then for the fixed terms, **Then**
   the first clears the night's syncs, zero points and base tilt and keeps the fixed terms,
   and the second clears the fixed terms.
3. **Given** `POST /api/model/second-sync` or `POST /api/model/reset` with a foreign
   `Origin`, **When** each is sent, **Then** each is answered 403 with a JSON `error`, no
   axis moves, and the model is unchanged.

---

### User Story 8 - GoTo and Stop on the page, and Setup's mount and model section (Priority: P2)

As the observer, I see GoTo in the guidance block while a mount is connected. It is
enabled only when a GoTo would be accepted, and otherwise says why not. It shows the
slew, each correction and "on target" with the final error.

Whenever the mount may be moving, Stop sits in the top bar of every page, the console and
Setup alike:
- always enabled;
- at least 44 px tall;
- never covered by a sheet.

Setup gains a mount and model section with four parts:
- the mount's link and board;
- the base tilt ("base tilt 0.8° toward 130°");
- the model's RMS residual and its number of syncs;
- the one-tap second sync, and the resets.

**Why this priority**: The page is how the observer uses GoTo and Stop. It is P2 because
it consumes US6's and US7's API.

**Independent Test**: In `crates/finder/web`, run `npm run typecheck`,
`npm run format:check` and `npm test`:
- the new tests are `src/components/GoToControl.test.tsx`,
  `src/components/StopButton.test.tsx` and `src/components/MountSection.test.tsx`;
- every existing Vitest file is unchanged.

Then `npm run build` rebuilds `web/dist`, and `git status --porcelain -- dist` lists
nothing once it is committed: the `test` gate runs both.

**Acceptance Scenarios**:

1. **Given** each of three sessions, **When** the guidance block renders, **Then** GoTo is
   disabled and the block says why. Stop is still shown when the session says the mount
   may be moving. The sessions are:
   - one whose target is below the horizon;
   - one with the Sun up;
   - one with no sync yet.
2. **Given** a session recorded while a GoTo to M42 ran on the simulator, **When** the
   block renders each of its states, **Then** a test asserts it shows the slew, each
   correction by number, and "on target" with the final error.
3. **Given** a session in which the second sync is slewing, **When** the Setup page
   renders, **Then** Stop is in Setup's top bar, and tapping it sends `POST /api/stop`.
4. **Given** any session in which the mount may be moving, **When** the console or Setup
   renders, with or without the target sheet open, **Then** a test asserts that Stop is
   enabled and is not inside the sheet's dialog, nor in any inert or `aria-hidden`
   subtree.
5. **Given** a session recorded from the simulator (base 1° off level toward azimuth 130°)
   holding two syncs 90° apart, **When** Setup renders, **Then** it reads "base tilt 1.0°
   toward 130°" within 0.1° and 2° of direction, with the RMS and "2 syncs".
6. **Given** a session with a connected mount, **When** Setup renders, **Then** its mount
   and model section shows the mount's link and board, and offers the second sync and the
   two resets. Each control sends its route.
7. **Given** `DESIGN.md` and `.impeccable/design.json`, **When** they are read, **Then**
   they describe GoTo in the guidance block, Stop in the top bar, and the mount and model
   section as built.
8. **Given** the committed `web/dist`, **When** the `test` gate runs `npm run build` and
   then `git status --porcelain -- dist`, **Then** it lists nothing, so the committed build
   is the build of this source, and `package.json` dependencies are unchanged.

---

### User Story 9 - A night on the simulator, end to end (Priority: P3)

As the maintainer, I run one Rust test that plays a whole night on the simulator in paused
virtual time:
- the first sync, then the second sync;
- five GoTos;
- a hand push;
- a Stop;
- a dropped link.

I also run a Playwright flow on `finder --simulate` that taps GoTo and Stop and the second
sync, and checks the red-only night rule while GoTo runs.

**Why this priority**: These tests confirm, as a whole, what each earlier story proves
piece by piece. They come last because they exercise every story before them.

**Independent Test**: `cargo test -p finder --locked` runs
`crates/finder/tests/night_sim.rs`. In `crates/finder/web`, `npm run e2e` runs the new
`e2e/goto.spec.ts` in 013's `simulate` project, with every earlier end-to-end test
unchanged and green.

**Acceptance Scenarios**:

1. **Given** the simulator and the console in one paused-time runtime from
   `--sim-start 2026-01-16T02:00:00Z` at 40° N, 75° W, **When**
   `crates/finder/tests/night_sim.rs` runs the night in this order, **Then** it asserts
   that every GoTo ends within 1′ and that the stops leave both axes at rest. It runs in
   the `test` gate's `cargo test --workspace --all-targets --all-features --locked`. The
   night:
   1. the first sync, then the second sync;
   2. five GoTos;
   3. a hand push;
   4. a GoTo;
   5. a Stop mid-slew;
   6. a dropped link mid-slew.
2. **Given** M42 as the target on `finder --simulate`, **When** a committed Playwright test
   taps GoTo, **Then** it asserts that the block shows the slew, each correction and "on
   target" with the final error, and that the chart's target reticle ends within 1′ of the
   aim point.
3. **Given** a slew under way on `finder --simulate`, **When** a committed Playwright test
   taps Stop, **Then** it asserts that the simulated axes stop and that the block says the
   observer stopped the GoTo.
4. **Given** one sync held on `finder --simulate`, **When** a committed Playwright test
   taps the second sync on Setup, **Then** it asserts that the mount slews, a second sync
   is taken, and the base tilt appears.
5. **Given** a GoTo running in night mode, **When** a committed Playwright test runs the
   night pixel check on its screenshot, **Then** it asserts no pixel with any green or
   blue, and none redder than the ink beyond two levels of compositing rounding.
6. **Given** a slew under way on the console and on Setup, **When** a committed Playwright
   test measures Stop, **Then** it asserts that Stop's bounding box is at least 44 px tall,
   and that the element at its centre is Stop itself.
7. **Given** the story's diff, **When** it is read, **Then** every earlier end-to-end test
   and the Playwright servers and projects are unchanged, except that the `simulate`
   project's match and the `default` project's ignore gain `e2e/goto.spec.ts` (owner
   approval 3).

---

### Edge Cases

- **Stop at any moment.**
  - A stop that arrives while the adapter is mid-reply wins over the reply. The queued
    command's epoch is older, so it is dropped.
  - Stop during the second sync, or during a correction, halts exactly as during a slew.
- **A frame with no mount.** A frame from the scope node with no mount connected carries
  no axis positions and is never a sync. The console behaves as after 013.
- **A power cycle mid-night** shows up as a slip at the first sync after it, and opens a
  new zero-point epoch. The cable's wind is the telescope's azimuth unwrapped continuously
  across syncs and epochs, not the raw counts the power cycle reset.
- **Corrections near the limit.**
  - A correction obeys the minimum altitude and the approach from below, as the first
    slew does.
  - A correction that would go below the minimum altitude fails the GoTo with that reason.
- **The Sun guard.**
  - If no ordering of the axes and no dogleg keeps a slew 15° clear of the Sun, the slew
    is refused before any axis moves.
  - The Sun guard has no override in the API, the page or Setup.
- **Firmware and refusals.**
  - Motor firmware below 3.39 is accepted with a warning.
  - A `!` reply other than `!2` fails the command with its code, after the retries allowed
    for commands that are safe to repeat.

## Requirements *(mandatory)*

### Functional Requirements

**The mount port and the simulated mount** (US1):

- **FR-001**: The scope node SHALL drive a mount only through a mount port of two axes.
  Each axis SHALL:
  - report its axis position and capabilities (counts per revolution, top rate, and
    whether it can go to a position);
  - go to a position;
  - move at a signed rate;
  - stop.

  Every mount SHALL be reached through an adapter behind this port. **Refs**: ADR-0001;
  GLOSSARY.md (mount port, mount adapter, axis position).
- **FR-002**: The simulator SHALL have a mount behind the mount port with these
  properties:
  - a base 1° off level by default;
  - axis zero points offset from true north and level;
  - 0.3° of backlash on each axis;
  - a top rate of 3.4°/s;
  - acceleration and deceleration of 3°/s².

  It SHALL expose its true eyepiece direction to tests. **Refs**: GLOSSARY.md (simulator);
  `docs/research/2026-10-21-synscan-mount-control.md` §1;
  `docs/research/2026-10-21-pointing-model-and-tracking.md` §1.
- **FR-003**: With the mount connected, the simulator's push by hand SHALL move the true
  direction without changing the axis positions, as on a Dob with its auxiliary encoders
  off. A test SHALL be able to make each axis slip by a set angle on every move, and to
  power-cycle the mount so that its counts reset. **Refs**: GLOSSARY.md (simulator);
  `docs/research/2026-10-21-synscan-mount-control.md` §1 (Freedom Find).

**Stop, the watchdog, frames and the link** (US2):

- **FR-004**: A stop SHALL cancel any move in progress. It SHALL discard every mount
  command that reached the node before it and has not yet been carried out. A command that
  reaches the node after the stop has been answered SHALL be carried out normally. The
  scope node SHALL stop both axes when 1 s has passed without a heartbeat from the console.
  After that, a move SHALL need a fresh request once the link returns. **Refs**: GLOSSARY.md
  (scope node); 013's scope-link heartbeat.
- **FR-005**: While a mount is connected, each star frame SHALL carry both axes' positions,
  read at the start and at the end of its exposure, on the node's clock. **Refs**:
  GLOSSARY.md (sync, axis position).
- **FR-006**: The scope link SHALL carry the mount port's calls:
  - describe the axes (counts per revolution, top rate, and whether each can go to a
    position);
  - each axis's state (position, rate, and whether it is moving);
  - go to a position;
  - move at a rate;
  - stop all.

  Every call except stop all SHALL need the token. *(Owner approval 2.)* **Refs**:
  ADR-0001; GLOSSARY.md (mount port).

**The Sky-Watcher adapter** (US3):

- **FR-007**: The Sky-Watcher adapter SHALL speak the motor-controller protocol:
  - framing as `:`, command, axis, data, CR, with 24-bit values low byte first and
    positions offset by 0x800000;
  - on connect, probe 9600 then 115200 baud with `:e1`, initialise with `:F`, read `:e`,
    `:a`, `:b` and `:g`, and switch the auxiliary encoders off with `:W`;
  - stop an axis before reversing it.

  **Refs**: the Sky-Watcher motor controller command set;
  `docs/research/2026-10-21-synscan-mount-control.md` §3, §5.
- **FR-008**: The adapter SHALL behave as follows:
  - **Transport:** skip its own echo on a single-wire bus. Read each reply with a 300 ms
    timeout, and retry up to 5 times for queries and other commands safe to repeat.
  - **Firmware:** accept motor firmware below 3.39, with a warning.
  - **Rates:** set them as a `:I` step period, using fast mode only above 128× sidereal.
  - **Goto:** go to a position with `:G`, `:S` and `:J`, falling back to `:H` and `:M`
    when `:S` is refused.
  - **`!2`:** answer it by stopping, waiting for `:f` to report stopped, and sending the
    command once more.
  - **Stop:** stop with `:K` on both axes, and send `:L` when `:K` is not acknowledged.

  A byte-level emulator of the mount SHALL stand in for it in tests. **Refs**: the
  Sky-Watcher motor controller command set; INDI's SkyWatcher driver (retries, fast mode);
  `docs/research/2026-10-21-synscan-mount-control.md` §3–5.
- **FR-009**: The adapter SHALL open the serial device with the serial path the owner
  chose: `serialport` (`default-features = false`), or termios through `rustix`.
  *(Owner approval 1.)* `scope-node --probe-mount DEVICE` SHALL run the first-test
  sequence (`:e1`, `:a1`, `:b1`, `:g1`, `:q1010000`, `:j1`) and print its results, for
  bring-up. The node's config file SHALL take an optional `[mount]` table naming the
  serial `device`. With it, the node drives the mount through the Sky-Watcher adapter on
  that device; without it, the node runs with no mount. **Refs**:
  `docs/research/2026-10-21-synscan-mount-control.md` (first test); `deny.toml`; 013's
  FR-007 (the node's config).

**Apparent altitude** (US4):

- **FR-010**: The system SHALL add refraction to a true (geometric) altitude by
  Saemundsson's formula, and remove it from an apparent altitude by Bennett's, for the
  standard atmosphere (1010 hPa, 10 °C). It SHALL leave the finder console's altitude and
  azimuth geometric. **Refs**: Meeus, *Astronomical Algorithms* 2nd ed., ch. 16 (eqs.
  16.3, 16.4); `docs/research/2026-10-21-pointing-model-and-tracking.md` §4.

**The pointing model** (US5):

- **FR-011**: The console SHALL take as a sync each solved star frame from the scope node
  that carries axis positions, when both axes moved at no more than 0.2°/s during its
  exposure. The sync is the apparent altitude and azimuth at the aim point at
  mid-exposure, with the axis positions interpolated to the same moment. Syncs SHALL be
  thinned to one per 2° of sky per 2 minutes, and corrected or held back where another
  requirement says so. A frame uploaded by hand SHALL NOT be a sync. **Refs**: GLOSSARY.md
  (sync, axis position); `docs/research/2026-10-21-pointing-model-and-tracking.md` §1–2.
- **FR-012**: The model SHALL fit the azimuth and altitude zero points (IA, IE) from one
  sync, and the base tilt (AN, AW) once it holds two syncs at least 90° apart in azimuth.
  It SHALL use TPoint's alt-az terms, with refraction removed first. **Refs**: TPoint
  manual v18.20 §7.2; Taki (1989);
  `docs/research/2026-10-21-pointing-model-and-tracking.md` §1.
- **FR-013**: Once the base tilt is fitted, a sync more than 0.5° from the model's
  prediction SHALL open a new zero-point epoch. The zero points SHALL be fitted again from
  that sync and later ones, keeping the base tilt and the fixed terms. Before then, a sync
  SHALL open one only when it is more than 2.5° from the prediction (0.5° plus twice a 1°
  base tilt). **Refs**: `docs/research/2026-10-21-synscan-mount-control.md` §1, §5.
- **FR-014**: The model SHALL fit four fixed terms once it holds at least 20 syncs spanning
  180° of azimuth and 30° of altitude:
  - the collimation error (CA);
  - the axes' non-perpendicularity (NPAE);
  - the tube's flexure (TF);
  - the finder camera's mounting on the tube.

  It SHALL keep them across nights until they are reset. **Refs**: TPoint manual v18.20
  §4.1, §7.2; `docs/research/2026-10-21-pointing-model-and-tracking.md` §1.
- **FR-015**: The session SHALL keep the night's syncs, zero points and base tilt across
  restarts until the Sun's next upper transit at the site (local noon) after the night's
  first sync, and then clear them. It SHALL keep the fixed terms across nights until they
  are reset. **Refs**: GLOSSARY.md (observing session, pointing model); 012's next
  rising, transit and setting.

**GoTo** (US6):

- **FR-016**: The console SHALL plan each slew to the target's apparent place (refraction
  added) at the expected arrival time, through the pointing model. The axis positions
  SHALL keep the cable within one revolution either way of the night's first sync. The
  wind is measured as the telescope's azimuth unwrapped continuously across syncs and
  zero-point epochs. **Refs**: FR-010, FR-012;
  `docs/research/2026-10-21-synscan-mount-control.md` §1 (cable wrap).
- **FR-017**: Every point of a planned slew, including the last approach and each
  correction, SHALL stay at or above the minimum altitude. When the last approach from
  below would go under it, the GoTo SHALL be refused with the reason. **Refs**:
  GLOSSARY.md (minimum altitude); plan.md § User Story 6 (the approach from below).
- **FR-018**: The console SHALL NOT move the mount while the Sun's centre is above −0.83°
  (sunrise and sunset as Meeus defines them), nor along a path that comes within 15° of
  the Sun. It SHALL re-plan a path that would, by moving the axes one at a time in the
  other order or through a higher altitude. It SHALL refuse the slew when no path keeps
  clear. **Refs**: GLOSSARY.md (Sun guard); Meeus ch. 15 (h₀ = −0°50′ for the Sun); 012's
  Sun.
- **FR-019**: A GoTo SHALL slew at the mount's top speed and make its last approach on
  each axis from one fixed direction. It SHALL then solve at the aim point and correct, at
  most 5 times, until the target is within 1′ of the aim point. Otherwise it SHALL end and
  report that it failed, with the last error. **Refs**: GLOSSARY.md (GoTo, slew); the
  centring loops of N.I.N.A. and Ekos (pointing-model research §2).
- **FR-020**: Stop, a lost link and the Sun guard SHALL each leave both axes stopped, and
  the console SHALL report which one ended the motion. `POST /api/stop` SHALL be accepted
  from any origin, with no body, and SHALL stop both axes whatever else is under way.
  *(Owner approval 2.)* **Refs**: FR-004; GLOSSARY.md (Sun guard); 012's same-origin rule
  and its exception for a route that only stops the telescope.
- **FR-021**: `POST /api/goto` SHALL start a GoTo to the session's target and report its
  progress with the session. It SHALL answer 409 with a JSON `error` naming the reason
  when any of these holds:
  - the target is not pointable;
  - no mount is connected;
  - the night has no sync yet;
  - the Sun is up.

  It SHALL be subject to 012's same-origin rule. **Refs**: 012's pointable and same-origin
  writes.

**The second sync and the model routes** (US7):

- **FR-022**: On request, the console SHALL slew to a pointable spot at least 90° in
  azimuth from the night's syncs and outside the Sun guard, solve there, and take the
  sync, so the base tilt can be fitted. **Refs**: GLOSSARY.md (sync);
  `docs/research/2026-10-21-pointing-model-and-tracking.md` §5.
- **FR-023**: `POST /api/model/second-sync` SHALL request the second sync.
  `POST /api/model/reset` SHALL clear either the night's model or the fixed terms. Both
  SHALL be subject to 012's same-origin rule. **Refs**: FR-015, FR-022.

**The page** (US8):

- **FR-024**: While a mount is connected, the guidance block SHALL offer GoTo, enabled only
  when a GoTo would be accepted and otherwise saying why not, and SHALL show its progress
  and result. Whenever the mount may be moving, every page SHALL show Stop in its top bar:
  - always enabled;
  - at least 44 px tall;
  - never covered by a sheet or another control.

  **Refs**: FR-019 to FR-021; DESIGN.md (Buttons; touch targets); GLOSSARY.md (GoTo).
- **FR-025**: The Setup page SHALL have a mount and model section showing four things:
  - the mount's link and board;
  - the base tilt ("base tilt 0.8° toward 130°");
  - the model's RMS residual and its number of syncs;
  - the one-tap second sync, and resets for the night's model and for the fixed terms.

  **Refs**: FR-011 to FR-015, FR-022, FR-023.
- **FR-026**: `DESIGN.md` and `.impeccable/design.json` SHALL record GoTo in the guidance
  block, Stop in the top bar, and the mount and model section as built. **Refs**:
  DESIGN.md; `.impeccable/design.json`.

**End to end** (US9):

- **FR-027**: A Rust end-to-end test SHALL play a night on the simulator in paused virtual
  time:
  - the first sync and the second sync;
  - five GoTos;
  - a hand push followed by a GoTo;
  - a Stop;
  - a dropped link.

  It SHALL hold every GoTo to 1′ and every halt to both axes at rest. **Refs**: FR-011 to
  FR-023.
- **FR-028**: A Playwright flow in 013's `simulate` project SHALL prove five things on
  `finder --simulate`:
  - a GoTo from the page;
  - Stop from the page;
  - the second sync from Setup;
  - Stop's size and that nothing covers it;
  - the night pixel check with a GoTo running.

  *(Owner approval 3.)* **Refs**: FR-024, FR-025.

### Key Entities

- **Mount port**: two axes. Each has `AxisInfo` (counts per revolution, top rate,
  `can_goto`) and `AxisState` (position in counts, rate, moving). The operations are
  `goto`, `rate` (signed degrees per second) and `stop_all`.
- **Sync**: the apparent altitude and azimuth at the aim point at mid-exposure; both axis
  positions interpolated to that moment; the solve's roll and RMS; the time; and the
  zero-point epoch it belongs to.
- **Pointing model**:
  - per night: the base tilt (AN, AW) and the syncs;
  - per zero-point epoch: IA and IE;
  - kept across nights: the fixed terms (CA, NPAE, TF and the camera's rotation on the
    tube);
  - the RMS residual.
- **GoTo**:
  - its `state`: `planning`, `slewing`, `settling`, `solving`, `correcting`, `done`,
    `failed` or `halted`;
  - its `correction` number;
  - its `error_arcsec`;
  - its `reason`: for `failed` and `halted`, one of the observer, a lost link, the Sun
    guard, the minimum altitude or did not converge.
- **The cable's wind**: the telescope's azimuth, unwrapped continuously from the night's
  first sync.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario above has a committed test that asserts it.
  `cargo test --workspace --locked`, `npm test` and `npm run e2e` pass.
- **SC-002**: No existing test, golden, `scripts/ci/` file, `.github/workflows/ci.yml` or
  `CODEOWNERS` entry is edited by any story. The owner approvals are the only exceptions
  to existing rules.
- **SC-003**: Workspace line coverage stays at or above the 80% floor after every story.
  The simulator tests in virtual time are what carry GoTo, sync and the model into it.
- **SC-004**: On the simulator, five GoTos from a second sync each end within 1′, and Stop
  brings both axes to rest within 1.5 s of virtual time.
- **SC-005**: No story opens a serial device or moves real hardware, and no API field,
  page control or option turns off the Sun guard.

## Assumptions

- **These are inherited from 012 and 013 and not restated:**
  - the observing session, the console's clock, the same-origin rule, the Sun and
    rise/transit/set times (012);
  - the scope link with its token and heartbeat, the scope node service, the simulator
    with its virtual clock and in-process link, `finder --simulate` and `--sim-start`, the
    aim point, and the `simulate` Playwright project (013).
- **Deferred:** tracking after a GoTo (015), so after a GoTo the axes stop.
- **Out of scope:**
  - the hand controller's protocol, SynScan App, and the mount's own star alignment and
    GoTo, which ADR-0001 rejects;
  - field derotation;
  - altitude limits beyond the minimum altitude and the mechanical range the adapter
    reports.
- **Done by the maintainer at the telescope, outside the factory**, after the spec lands:
  - the checks listed in Clarifications;
  - a cross-build of `scope-node` for `aarch64-unknown-linux-musl`, which settles, if the
    owner chose `serialport`, whether it builds there or termios through `rustix` replaces
    it.

  Neither is evidence a diff can carry.
- **Done by the maintainer after US8 lands:** the Impeccable review of GoTo, Stop and the
  mount and model section.
- **Hardware facts that come from the mount, not the code:** the 400P's board, its motor
  firmware version and its counts per revolution. `--probe-mount` records them at
  bring-up.
- **The factory's gates and CI** (as of 3fc6a24; plan.md § Verification):
  - The `lint` gate runs `cargo fmt`, clippy and rustdoc. The `test` gate runs the
    workspace's tests and, in `crates/finder/web`, `npm ci`, `npm run typecheck`,
    `npm run format:check`, `npm test`, `npm run build`, the build-freshness check and
    `CI=1 npm run e2e`. Every story runs both.
  - The merge queue's CI also runs coverage, the cedar eval, `cargo-deny`, `cargo machete`,
    the secret scan and the MSRV build, which the sandbox cannot run (plan.md § Plan traps,
    trap 5). `cargo-deny` and the MSRV build check US3's serial code there.

## Work Graph

```yaml
US1:
  depends_on: []
  implements: [FR-001, FR-002, FR-003]
US2:
  depends_on: []
  depends_on_merged: [US1]
  implements: [FR-004, FR-005, FR-006]
US3:
  depends_on: []
  depends_on_merged: [US2]
  implements: [FR-007, FR-008, FR-009]
US4:
  depends_on: []
  implements: [FR-010]
US5:
  depends_on: []
  depends_on_merged: [US2, US4]
  implements: [FR-011, FR-012, FR-013, FR-014, FR-015]
US6:
  depends_on: []
  depends_on_merged: [US5]
  implements: [FR-016, FR-017, FR-018, FR-019, FR-020, FR-021]
US7:
  depends_on: []
  depends_on_merged: [US6]
  implements: [FR-022, FR-023]
US8:
  depends_on: []
  depends_on_merged: [US7]
  implements: [FR-024, FR-025, FR-026]
US9:
  depends_on: []
  depends_on_merged: [US8]
  implements: [FR-027, FR-028]
```

Every edge is a merge edge, because each dependent imports what its predecessor creates:

| Story | Builds on |
|---|---|
| US2 | US1: it queues commands to the mount port and watches the simulated mount |
| US3 | US2: it adds its adapter beside the port, queue and watchdog in `crates/scope-node/src/mount/` |
| US5 | US2: it reads the axis positions on star frames, through the mount calls |
| US5 | US4: it removes refraction before fitting |
| US6 | US5: it plans through the model's inverse |
| US7 | US6: it slews with US6's machinery |
| US8 | US7: it renders GoTo's progress and the model routes |
| US9 | US8: it drives the finished page |

Two branches run alongside each other:
- **US4** (`crates/finder/src/ephemeris.rs` only) runs beside US1 and US2, which touch
  only `crates/scope-node`, `crates/scope-link` and `crates/simulator`.
- **US3** runs beside the console chain (US5 to US9), once US2 has merged. US3 touches
  only `crates/scope-node`, its `Cargo.toml` and `Cargo.lock`; the console chain touches
  `crates/finder`, `DESIGN.md` and `.impeccable/`.

Every console story edits `crates/finder/src/api.rs` or the page, so the console stories
form a chain.
