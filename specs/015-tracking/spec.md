---
state: ready
depends_on_landed: ["012-targets-and-push-to", "013-live-finder", "014-goto"]
# Converted 2026-10-28 from the reviewed OpenSpec change feat-15 (#102). This trio is
# now the feature's only specification. It stays draft until the owner has reviewed
# it and given the approvals listed under "Owner approvals". Flipping it to `ready`
# is the act that dispatches it.
# Refined 2026-10-31 against main at 1976dff (013's US1 to US4, US8 and US9 landed) and
# 014's refined trio at cab5143 (plan.md § Plan traps and § Refinement Record). It stays
# draft until 013 and 014 have landed and the prerequisites in plan.md are met; the paths
# marked "013's" and "014's" are re-read against them as landed then.
# Flipped to ready 2026-11-01 by the owner, before 014 landed (014's US1 and US4 on main,
# US2 running); depends_on_landed holds dispatch until 014 lands. Re-read against 014 so
# far: US1's MountConfig already has backlash_deg, one value for both axes, so US2 adds
# only rate_error (FR-004, T007; plan.md § Plan traps, trap 16). The rest of the re-read
# follows 014's stories as they land (plan.md § Prerequisites, 1).
---

# Feature Specification: Tracking

**Feature Branch**: `015-tracking`

**Created**: 2026-10-28

**Status**: Draft

**Input**: After a GoTo (014) the target sits at the aim point, then drifts out of the
eyepiece within a minute or two. An alt-az mount must turn both axes at changing rates to
follow the sky, and the mount's board runs only constant rates. This feature makes the
console follow the target:

- feed-forward axis rates from the pointing model, sent at least once a second;
- corrected by proportional-integral control from the finder camera's solves;
- backlash taken up before an axis reverses;
- a pause through the 2° circle round the zenith;
- a safe ending on Stop, a lost link, sunrise or a target sinking too low.

It is for visual use, pushed far enough toward imaging to report the field rotation and
the longest exposure an imaging camera allows. It needs no change to the scope node or
the link: tracking uses 014's move-at-rate and stop.

## Clarifications

### Session 2026-10-28

- Q: What does a story read besides this trio? → A: Nothing about the feature. The trio is
  self-contained: `spec.md` holds every requirement and scenario, and `plan.md` holds
  every design decision, the interfaces, and the alternatives rejected. A story that
  would change a decision escalates rather than re-deciding it.
- Q: How is the change sliced for the factory? → A: Into nine stories, each a vertical
  slice that lands its own tests:
  - the alt-az rates, field rotation and the longest exposure;
  - the simulator's rate error, backlash setting and truth hook;
  - feed-forward tracking and its safe ending;
  - the closed loop on the finder camera's solves;
  - backlash at reversals, kept with the fixed terms;
  - the zenith keyhole;
  - Track and the imaging camera in the session;
  - the page;
  - the end-to-end runs.
- Q: Does any story move real hardware? → A: No. Every tracking behaviour is proven on
  014's simulated mount in virtual time. The checklist at the telescope is the
  maintainer's, outside the factory (see Assumptions).
- Q: How do long tracking runs stay fast enough for the coverage job? → A: They feed the
  tracker the simulator's true eyepiece direction plus Gaussian noise (σ 3″) as its
  pointings, through the same entry point a solve takes after the solver. One short run on
  rendered, solved frames proves that the two paths agree.
- Q: How does Impeccable take part? → A: `DESIGN.md`, `.impeccable/design.json` and the
  surface briefs under `.impeccable/surfaces/` are the design system the page story builds
  to. The Impeccable review itself is outside the factory, because a "ship verdict" is not
  evidence the judge can see in a diff. US8 records the new parts in `DESIGN.md` as built,
  and the maintainer runs the review after US8 lands.
- Q: Who approves the changes to tested behaviour? → A: The owner, by reviewing this spec
  and flipping it to `ready`. The approvals are listed below so that the flip is an
  informed act.

### Session 2026-10-31 (refinement against main, 013 and 014)

- Q: Where are the simulated mount and the pointing model? → A: 014 creates them, the
  simulated mount in `crates/simulator/src/mount.rs` (014's US1) and the model in
  `crates/finder/src/model.rs` (014's US5). US2 extends both there.
- Q: What do the new session fields read before they have a value? → A: They are left out
  of the body, as 013's and 014's are, so the whole-body test of a fresh session stays as
  it is. `track` appears once Track is first turned on in a run of the binary,
  `imaging_camera` once one is set, and `backlash` once an axis's backlash is measured.
- Q: Does Stop end tracking? → A: Yes. `POST /api/stop` stops both axes, as 014 made it,
  and also ends tracking as stopped by the observer, so the next second's rates never
  start the axes again (FR-007, US7-S10).
- Q: What feeds the tracker its pointings in the running console? → A: 013's live loop
  (`crates/finder/src/live.rs`), which hands each live solve's error at the aim point to
  the tracker while tracking, from US7. US4's tests feed the same entry point themselves.
- Q: Are tracking frames still syncs? → A: Yes. 014 takes a frame as a sync when both axes
  move at no more than 0.2°/s during it. Tracking rates stay under that outside the 2°
  zenith circle (about 0.1°/s in azimuth at its edge). The backlash take-up at 0.5°/s
  and the keyhole slew are not syncs, as a GoTo's slew is not.
- Q: How can US6 and US7 run together when both concern the tracker? → A: The armed state
  (Track on, waiting for a GoTo) is the session's, and the session starts the tracker
  when the GoTo ends. So US7 does not edit `crates/finder/src/tracking.rs`, and US6 edits
  nothing else of the console's.

## Owner approvals

Flipping this spec to `ready` approves the following. Each is named in the FR that makes
it. No existing test changes, and CI (`ci.yml`) is unchanged.

1. **The closed-loop scenario uses a 1% rate error, not 0.5%.** The earlier plan said
   "a 0.5% rate error drifts more than 1′ in 10 minutes". That does not hold: 0.5% of the
   8,985″ M42 moves in 10 minutes is about 45″. The scenario uses 1% (about 90″) instead
   (FR-008).
2. **Two lines of Playwright config, and the `simulate` project one file at a time.**
   013's `simulate` project matches only `e2e/simulate.spec.ts`, and its `default`
   project ignores that file by name (013 plan.md, User Story 10). 014's US9 adds
   `e2e/goto.spec.ts` to both. US9 here adds `e2e/tracking.spec.ts` to the `simulate`
   project's match and to the `default` project's ignore. The `simulate` project's files
   share one `finder --simulate` server, and each sets the session's target and moves the
   mount, so they must not run at once: the project runs one file at a time
   (`workers: 1` on the project, Playwright 1.52 and later). 014's US9 sets it (014's
   approval 3); if it is not set when US9 here runs, US9 sets it. No server, other
   project or existing test changes (FR-020). Re-read against 013's US10 as landed, and
   against 014's US9 once it lands.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Alt-az rates, field rotation and the longest exposure (Priority: P1)

As the tracker, the readout and the tests' oracles, I ask `finder::ephemeris` how fast a
position's altitude, azimuth and parallactic angle change at a site and time. As the
readout, I also ask how long an imaging camera may expose before field rotation smears
its corner by one pixel, or whether there is no limit.

**Why this priority**: The readout, the keyhole forecast and every tracking test's oracle
are these rates. A sign error in field rotation misstates every exposure limit.

**Independent Test**: `cargo test -p finder --locked --test rates` runs the new
integration test file `crates/finder/tests/rates.rs` against the public functions in
`crates/finder/src/ephemeris.rs`. Every input is explicit, so no clock or server is
needed.

**Acceptance Scenarios**:

1. **Given** latitude 40°, declination 20° and hour angle 0, **When** a test in
   `tests/rates.rs` computes the rates, **Then** it asserts:
   - an altitude rate of 0, within 0.01″/s;
   - an azimuth rate of 41.33″/s, within 0.02″/s;
   - a field rotation of +33.69°/h with the parallactic angle increasing, within 0.02°/h.
2. **Given** latitude 40°, declination 20° and hour angle −30°, two hours before transit,
   **When** a test computes the rates, **Then** it asserts an altitude rate of +10.07″/s
   and an azimuth rate of 18.45″/s, each within 0.02″/s.
3. **Given** a 23.5 × 15.6 mm sensor with 3.76 µm pixels at latitude 40°, declination 20°,
   hour angle 0, **When** a test computes the longest exposure for 1 px of smear, **Then**
   it asserts a corner radius of 3,751 px and an exposure of 1.63 s, within 0.01 s.
4. **Given** latitude 40° and a target at altitude 30°, azimuth 90° (due east), and again
   at azimuth 270°, **When** a test computes the longest exposure, **Then** both answers
   say that field rotation sets no limit (|dq/dt| below 1e-9 rad/s).
5. **Given** the story's diff, **When** it is read, **Then** `crates/math-core` is
   untouched, and every existing test in `crates/finder/src/ephemeris.rs` and
   `crates/finder/src/sky.rs` is unchanged.

---

### User Story 2 - The simulator's rate error, backlash setting and known model (Priority: P1)

As a tracking test, I set the simulated mount's axes to run 1% fast, set its backlash
(014's setting, one value for both axes, 0.3° by default) to any value including 0, and
load the simulator's true base tilt and zero points into the console's pointing model. A
test can then show that feed-forward alone drifts, and that the closed loop holds, against
a model it knows to be right.

**Why this priority**: Every later story's evidence runs on these three settings. Without
a rate error, the closed loop has nothing to correct. Without a backlash setting, a
reversal cannot be measured. Without the truth loaded, a feed-forward test would also
measure the model's fit.

**Independent Test**: Two sets of tests run:
- `cargo test -p simulator --locked` runs the new `crates/simulator/tests/rate_and_backlash.rs`
  against 014's simulated mount in `crates/simulator/src/mount.rs`;
- `cargo test -p finder --locked --test sim_truth` runs the new
  `crates/finder/tests/sim_truth.rs` against 014's pointing model in
  `crates/finder/src/model.rs`.

The simulator's existing tests, 013's and 014's, pass unchanged.

**Acceptance Scenarios**:

1. **Given** a rate error of 1%, **When** an axis is commanded at 0.01°/s for 100 s of
   virtual time, **Then** a test asserts that it turns 1.01°, within 0.001°.
2. **Given** an axis whose backlash is set to 0, **When** it goes up 1° and then down 1°,
   **Then** a test asserts that its true direction returns to where it started, within 1″.
3. **Given** the simulator's truth (the base 1° off level toward 130°, zero points
   offset), **When** a test loads it into the console's pointing model, **Then** it asserts
   that the model's prediction for axis altitude 45°, azimuth 120° equals the simulator's
   true eyepiece direction there, within 1″.
4. **Given** a simulated mount built with no settings, **When** a test reads them,
   **Then** the rate error is exactly 0 and the backlash is exactly 0.3° on both axes, as
   014's simulator tests assume.
5. **Given** the story's diff, **When** it is read, **Then** every existing test in
   `crates/simulator` and `crates/scope-node` is unchanged.

---

### User Story 3 - Feed-forward tracking that ends safely (Priority: P1)

As the console, I track the session's target from the moment tracking starts. Each second
I send each axis the rate that keeps the target's apparent place at the aim point,
through the pointing model. I stop both axes and record why on any of four events:
- Stop;
- a lost link;
- the Sun's centre rising above −0.83°;
- the target sinking below the minimum altitude.

**Why this priority**: Feed-forward is what tracking is, and the safe ending is what
makes it allowed to run unattended for even a minute.

**Independent Test**: `cargo test -p finder --locked --test tracking_sim` runs the new
`crates/finder/tests/tracking_sim.rs`. It drives `finder::tracking` against 014's
simulated mount, with the console and the simulator in one paused-time tokio runtime over
the in-process link, as 014's GoTo tests do.

**Acceptance Scenarios**:

1. **Given** a simulator with no rate error, its backlash set to 0, and its true base tilt
   and zero points loaded into the console's model, **When** M42 is tracked for 10 minutes
   of virtual time, **Then** a test asserts two things:
   - a new rate is sent to each axis at least once a second;
   - the true eyepiece stays within 10″ of M42 throughout.
2. **Given** a target tracked as it sets through a minimum altitude of 15°, **When** its
   geometric altitude reaches 15°, **Then** a test asserts that tracking ends with both
   axes stopped within 1 s, saying the target is too low.
3. **Given** a target being tracked, **When** the observer's Stop reaches the tracker (as
   `POST /api/stop` hands it on from US7), **Then** a test asserts that both axes stop
   within 0.5 s of virtual time, that no rate is sent to either axis after the stop, and
   that tracking reports the observer stopped it.
4. **Given** M42 being tracked, **When** the heartbeats between the console and the node
   fall silent, as in 014's US6-S12, **Then** a test asserts three things:
   - the node's watchdog stops both axes 1.0 ± 0.1 s after the last heartbeat;
   - tracking ends saying the link was lost;
   - it does not resume when the link returns.
5. **Given** a target being tracked, **When** the virtual clock passes the moment the
   Sun's centre reaches −0.83°, **Then** a test asserts that both axes are commanded to
   stop within 1 s of that moment, and that tracking ends saying the Sun is up.

---

### User Story 4 - Closed by the finder camera (Priority: P1)

As the console tracking a target, I correct each axis's rate from at least one solve
every 5 s. I use proportional-integral control on the error between the target and the
aim point, resolved into the two axes. I ignore errors under 10″, and inside that
deadband the held integral keeps applying but is not updated. A mount that runs 1% fast
then still holds the target.

**Why this priority**: Feed-forward drifts on any real mount. The finder camera's solves
are what keep the target in a high-power eyepiece.

**Independent Test**: `cargo test -p finder --locked --test tracking_loop` runs the new
`crates/finder/tests/tracking_loop.rs`, in paused time on the simulator:
- the long runs feed the tracker the simulator's truth plus 3″ of noise every 2 s;
- one 2-minute run uses rendered, solved frames.

**Acceptance Scenarios**:

1. **Given** a simulator whose axes run 1% faster than commanded, and pointings that are
   its truth plus 3″ of noise every 2 s, **When** M42 is tracked for 10 minutes of virtual
   time, first with feed-forward alone and then with the finder camera's corrections,
   **Then** a test asserts two things:
   - feed-forward alone drifts more than 1′ (about 90″);
   - with the corrections, the RMS error over the last 8 minutes is at most 20″.
2. **Given** a simulator with no rate error, a zero integral at the start, and every
   pointing's error under 10″, **When** M42 is tracked for 2 minutes of virtual time,
   **Then** a test asserts that each axis's commanded rate equals the feed-forward rate
   within 0.001″/s, and that the tracking offset stays 0.
3. **Given** M42 tracked for the same 2 minutes of virtual time on the simulator twice,
   once on rendered and solved frames and once on its truth plus 3″ of noise every 2 s,
   **When** the two runs' tracking errors are compared, **Then** a test asserts that they
   agree within 5″ RMS.

---

### User Story 5 - Backlash at reversals, measured once and kept (Priority: P1)

As the console tracking a target through its transit, or through greatest elongation
when its declination exceeds the latitude, I handle each axis reversal as follows:
- the first time an axis reverses on this mount, I measure its backlash;
- before every later reversal, I stop that axis and take the backlash up;
- I keep the measurement with the fixed terms, across restarts and nights;
- I correct the syncs taken while an axis last moved against the GoTo's approach, so they
  do not bias the zero points.

**Why this priority**: The simulator's 0.3° of backlash (1,080″) throws the target far out
of a high-power eyepiece at every transit. Uncorrected, it would also bias IE by up to
0.3° without reaching the 0.5° slip threshold.

**Independent Test**: Two sets of tests run in paused time on the simulator with 0.3° of
backlash on each axis:
- `cargo test -p finder --locked --test backlash_sim` runs the new
  `crates/finder/tests/backlash_sim.rs`;
- `cargo test -p finder --locked --test backlash_cli` runs the new
  `crates/finder/tests/backlash_cli.rs`, which restarts the built binary on one `--state`
  folder as the earlier CLI tests do.

**Acceptance Scenarios**:

1. **Given** a target tracked through its transit on the simulator (0.3° of backlash)
   before any backlash has been measured on this mount, **When** the altitude axis
   reverses, **Then** a test asserts three things:
   - the axis stops before it reverses;
   - the target is back within 30″ of the aim point within 15 s of the reversal;
   - the backlash measured there is 0.3°, within 30″.
2. **Given** that measurement, **When** another target is tracked through its transit on
   the same simulated mount, **Then** a test asserts that the altitude axis stops, takes up
   the measured backlash and reverses, and that the target stays within 30″ of the aim
   point through the reversal.
3. **Given** the azimuth axis's backlash already measured, **When** M81 (Dec +69°) is
   tracked at latitude 40° through its greatest elongation (hour angle about 71.3°, at
   altitude about 43.5°, where its azimuth rate changes sign), **Then** a test asserts
   that the azimuth axis stops,
   takes up the measured backlash and reverses, and that the target stays within 30″ of
   the aim point through the reversal.
4. **Given** the altitude axis's backlash measured on the simulator, **When** the binary
   restarts on the same `--state` folder the next day, **Then** a test asserts that the
   session's `backlash` holds the same value for that axis exactly.
5. **Given** M42 tracked through its transit on the simulator, with 0.3° of altitude
   backlash measured at that reversal, **When** syncs are then taken while the altitude
   axis tracks down, **Then** a test asserts that no new zero-point epoch is opened, and
   that the fitted IE equals the simulator's true altitude zero point within 30″.
6. **Given** a measured backlash, **When** the fixed terms are reset through
   `POST /api/model/reset` with `{"what": "fixed"}`, **Then** a test asserts that the
   session's body has no `backlash` field. The test first reads the measured backlash in
   the same session, so the field's absence follows a value.

---

### User Story 6 - The zenith keyhole (Priority: P2)

As the console tracking a target whose path will come within 2° of the zenith, I handle
the keyhole in four steps:
1. I stop following it where it enters that circle.
2. I slew ahead, through 014's planner with its Sun guard, to where it will leave.
3. I wait there.
4. When it leaves, I solve and resume.

**Why this priority**: A target 1° from the zenith spends about 18 minutes in the 2°
circle, where the azimuth rate changes too fast to chase. Waiting at the exit is how
tracking survives it. It is P2 because it matters only for targets that pass near the
zenith.

**Independent Test**: `cargo test -p finder --locked --test keyhole_sim` runs the new
`crates/finder/tests/keyhole_sim.rs`, in paused time on the simulator.

**Acceptance Scenarios**:

1. **Given** a target at declination φ − 1° tracked at latitude φ = 40° through its
   transit, **When** its path crosses the 2° circle round the zenith, **Then** a test
   asserts three things:
   - tracking pauses where the target enters the circle;
   - the telescope waits where the target will leave;
   - within 30 s of the target leaving, the target is back within 1′ of the aim point.

---

### User Story 7 - Track and the imaging camera in the session (Priority: P2)

As a phone or laptop on the console, I turn Track on and off with `POST /api/track`. Turned
on, it does one of three things:
- if a GoTo is running, tracking starts when it ends;
- if the target is more than 1′ from the aim point, a GoTo is started first;
- if a mount, a sync, a set Sun or a pointable target is missing, it is refused, saying
  why.

I put the imaging camera's sensor size and pixel pitch into the session. The session then
reports the tracking state, why tracking last ended, and the readout: the RMS error, the
field rotation and the longest exposure.

**Why this priority**: This is how the page and later clients drive tracking. It is P2
because it consumes the tracker that US3 to US5 build.

**Independent Test**: Two sets of tests run:
- `cargo test -p finder --locked --test track_api` runs the new
  `crates/finder/tests/track_api.rs`, through the router with the simulator in paused
  time;
- `cargo test -p finder --locked --test track_cli` runs the new
  `crates/finder/tests/track_cli.rs`, which restarts the built binary on one `--state`
  folder.

**Acceptance Scenarios**:

1. **Given** Track turned on, **When** a GoTo to M42 completes on the simulator, **Then**
   a test asserts that tracking starts within 2 s of the end of the GoTo, and that the
   session reports it tracking M42.
2. **Given** M8 below the horizon as the target, **When** Track is turned on, **Then** a
   test asserts that the answer is 409 with a JSON `error` saying why, and that nothing
   moves.
3. **Given** M42 5° from the aim point and no GoTo running on the simulator, **When** Track
   is turned on, **Then** a test asserts that a GoTo to M42 starts, and that tracking
   starts within 2 s of its end.
4. **Given** M42 being tracked on the simulator, **When** Track is turned off, **Then** a
   test asserts that both axes stop within 0.5 s of virtual time, and that the telescope
   stays where it is (the simulator's true direction moves by less than 1′ after they
   stop).
5. **Given** a 23.5 × 15.6 mm sensor with 3.76 µm pixels put into the session, **When** the
   binary restarts on the same `--state` folder, **Then** a test asserts that the session
   holds the same three values exactly, and that once `"imaging_camera": null` is put,
   the body has no `imaging_camera` field.
6. **Given** a session, **When** a pixel pitch of 0 µm is put, **Then** a test asserts that
   the answer is 400 with a JSON `error`, and that nothing changes.
7. **Given** M42 tracked on the simulator with that imaging camera in the session, **When**
   the session is read, **Then** a test asserts three things about its tracking readout:
   - the RMS error over the last 60 s;
   - a field rotation in °/min equal to FR-001's rate at the target divided by 60, within
     1e-9;
   - a longest exposure equal to FR-002's answer, within 1e-9 s.
8. **Given** a `POST /api/track` that carries `Origin: http://elsewhere.example`, **When**
   it is sent, **Then** a test asserts that the answer is 403 with a JSON `error`, and that
   nothing moves.
9. **Given** tracking on, **When** the binary restarts on the same `--state` folder,
   **Then** a test asserts that tracking is off after the restart and that nothing moves.
10. **Given** M42 being tracked on the simulator, **When** a bodiless `POST /api/stop`
    from a foreign origin is sent, **Then** a test asserts that the answer is 200, that
    both axes are at rest within 1.5 s of virtual time and stay at rest for the next 10 s,
    and that the session reports tracking ended because the observer stopped it.

In US7-S2, US7-S8 and US7-S9, "nothing moves" is asserted only after the same test has
seen an accepted Track move an axis of the connected simulated mount (plan.md § Plan
traps, trap 9).

---

### User Story 8 - Track on the console, and Setup's imaging camera section (Priority: P2)

As the observer with a mount connected, the guidance block offers me a Track toggle. While
tracking, it reads out three values:
- the tracking error;
- the field rotation;
- with an imaging camera set, the longest exposure.

When tracking ends, it says why. On the Setup page, I set the imaging camera's sensor size
and pixel pitch, and the page refuses values out of range with a message.

**Why this priority**: This is the visible half of tracking. It is P2 because it consumes
US7's API.

**Independent Test**: The `test` gate's page checks run in `crates/finder/web`:
`npm run typecheck`, `npm run format:check` and `npm test`:
- the new tests are `src/components/TrackControl.test.tsx` and
  `src/components/ImagingCameraSection.test.tsx`;
- every existing Vitest file is unchanged.

Then the gate's `npm run build` and `git status --porcelain -- dist` list nothing for the
committed `web/dist`, and its `CI=1 npm run e2e` passes with `e2e/session.spec.ts`
unchanged.

**Acceptance Scenarios**:

1. **Given** a recorded session in which tracking ended because the target sank below the
   minimum altitude, **When** a Vitest test renders the guidance block, **Then** it asserts
   that the toggle is off and that the block says the target is too low.
2. **Given** a recorded tracking session carrying an RMS error, a field rotation and a
   longest exposure, **When** a Vitest test renders the guidance block, **Then** it asserts
   the readout rounded to the nearest arcsecond, 0.01°/min and 0.1 s. A session reporting
   no limit reads "no limit".
3. **Given** a recorded session with no mount connected, **When** a Vitest test renders
   the guidance block, **Then** it asserts that no Track toggle is offered.
4. **Given** the Setup page, **When** a Vitest test enters 23.5, 15.6 and 3.76 and saves,
   **Then** it asserts that the session is put with those three values, and that the
   guidance block's readout gains the longest exposure.
5. **Given** the Setup page, **When** a Vitest test enters a pitch of 0, **Then** it
   asserts that nothing is saved and that the page states the allowed range.
6. **Given** `DESIGN.md` and `.impeccable/design.json`, **When** they are read, **Then**
   they describe the Track toggle, the tracking readout and Setup's imaging camera section
   as built.
7. **Given** the rebuilt `web/dist`, **When** the `test` gate runs `npm run build` and
   then `git status --porcelain -- dist`, **Then** the second lists nothing, so the
   committed build is the build of this source. The story's diff leaves `package.json`'s
   dependencies unchanged, so no npm package was added.

---

### User Story 9 - An hour of tracking end to end, and Track on the page (Priority: P3)

As the maintainer, I run two end-to-end checks. The first is one Rust test of an hour of
tracking on the simulator in paused time: a 1% rate error, a transit, a keyhole passage,
then setting below the minimum altitude. It finishes in under 30 s of wall time, so the
coverage job stays fast. The second is a Playwright flow that turns Track on for M42 on
`finder --simulate` and runs the night pixel check with tracking on.

**Why this priority**: These tests confirm, in one run, what each story proved piece by
piece. They come last because they exercise every story before them.

**Independent Test**: Two checks run:
- `cargo test -p finder --locked --test tracking_night` runs the new
  `crates/finder/tests/tracking_night.rs`;
- in `crates/finder/web`, the `test` gate's `CI=1 npm run e2e` runs every earlier test
  unchanged and green, plus the new `e2e/tracking.spec.ts` in the `simulate` project.

**Acceptance Scenarios**:

1. **Given** the simulator with a 1% rate error and 0.3° of backlash, **When** a target is
   tracked for an hour of paused virtual time through a transit, a keyhole passage and
   then setting below the minimum altitude, **Then** the test asserts four things:
   - the target stays within 30″ of the aim point through the transit's reversal;
   - after leaving the keyhole, it is back within 1′ within 30 s;
   - tracking ends saying the target is too low, within 1 s of its geometric altitude
     reaching the minimum;
   - the test's own `Instant::elapsed` is under 30 s.

   It runs in the `test` gate's `cargo test --workspace --all-targets --all-features
   --locked`.
2. **Given** `finder --simulate` with a 23.5 × 15.6 mm, 3.76 µm imaging camera and Track
   on for M42, **When** a committed Playwright test in `e2e/tracking.spec.ts` reads the
   guidance block, **Then** it asserts that the error in arcseconds, the field rotation and
   the longest exposure each equal the session's values as rounded for display.
3. **Given** M42 tracked in night mode on `finder --simulate`, **When** a committed
   Playwright test runs the night pixel check on its screenshot, **Then** it asserts no
   pixel with any green or blue, and none redder than the ink beyond two levels of
   compositing rounding.
4. **Given** the story's diff, **When** it is read, **Then** every earlier end-to-end test
   file is unchanged. `playwright.config.ts` changes only by owner approval 2: it adds
   `e2e/tracking.spec.ts` to the `simulate` project's match and the `default` project's
   ignore, and sets the `simulate` project to one file at a time if 014 has not.

---

### Edge Cases

- **Starting and stopping:**
  - Track turned on while a GoTo is running starts tracking only when the GoTo completes.
    A GoTo that fails or is halted leaves tracking off, with the GoTo's reason.
  - Track turned off while a GoTo armed by Track is running stops the GoTo and both axes,
    as Stop does.
- **Ending:**
  - A lost link ends tracking for good. It does not resume when the link returns, because
    the observer must turn Track on again.
  - The integral (the tracking offset) is cleared whenever tracking ends, for any reason.
- **Backlash:**
  - A reversal forecast inside the 30-minute horizon before any backlash is measured is
    handled by the closed loop alone, which recovers within about 15 s. That reversal is
    the measurement.
  - A sync taken against the approach direction before that axis's backlash is measured
    is held back from the fit, not dropped.
- **The keyhole:** a target whose keyhole exit is within 15° of the Sun is not slewed to.
  014's planner refuses the slew, tracking ends saying why, and both axes stop.
- **The imaging camera:** a width or height of 0 or above 100 mm, or a pitch of 0 or above
  50 µm, is refused with 400 and changes nothing. Without an imaging camera, the readout
  has no longest exposure.

## Requirements *(mandatory)*

### Functional Requirements

**Rates and the longest exposure** (US1):

- **FR-001**: The system SHALL compute, for a position at a site and time, the rates of
  its altitude, its azimuth and its field rotation (the parallactic angle), from the
  spherical relations of Meeus ch. 13 and 14. The altitude and azimuth rates SHALL be in
  arcseconds per second, and the field rotation in degrees per hour, positive when the
  parallactic angle increases. **Refs**: Meeus, *Astronomical Algorithms* 2nd ed., ch. 13
  and 14; SLALIB `sla_ALTAZ`; `docs/research/2026-10-21-pointing-model-and-tracking.md` §3.
- **FR-002**: The system SHALL compute the longest exposure an imaging camera allows
  before field rotation smears the frame's corner by a given number of pixels, from its
  sensor size and pixel pitch. It SHALL say there is no limit where the field rotation is
  zero (|dq/dt| below 1e-9 rad/s). **Refs**: GLOSSARY.md (imaging camera);
  `docs/research/2026-10-21-pointing-model-and-tracking.md` §3.

**The simulator** (US2):

- **FR-003**: The simulated mount SHALL take a rate-error setting, zero by default. It is
  a fraction by which every axis runs faster than commanded, or slower when negative.
  **Refs**: GLOSSARY.md (simulator); FR-008.
- **FR-004**: The simulated mount SHALL take a backlash setting, 0.3° by default and
  settable to any value including 0. 014's US1 provides it as `MountConfig.backlash_deg`,
  one value for both axes. **Refs**: GLOSSARY.md (simulator); FR-009.
- **FR-005**: Tests SHALL be able to load the simulator's true base tilt and zero points
  into the console's pointing model. **Refs**: GLOSSARY.md (simulator, pointing model);
  014's pointing model ("Zero points and the base tilt").

**Feed-forward and ending** (US3):

- **FR-006**: While tracking, the console SHALL command each axis at the rate that keeps
  the target's apparent place at the aim point, through the pointing model. The rates
  SHALL be recomputed and sent at least once a second. **Refs**: FR-001; GLOSSARY.md
  (tracking, aim point); `docs/research/2026-10-21-synscan-mount-control.md` §3 (the host
  computes alt-az tracking).
- **FR-007**: Tracking SHALL end on any of the following, leaving both axes stopped, and
  the console SHALL report which:
  - Stop;
  - a lost link;
  - the Sun's centre rising above −0.83°;
  - the target's altitude (geometric, as pointable is judged) sinking below the minimum
    altitude.

  **Refs**: 014's GoTo ("Halts"); 012's push-to ("Pointable"); GLOSSARY.md (Sun guard,
  minimum altitude).

**The closed loop** (US4):

- **FR-008**: *(Owner approval 1.)* While tracking, the console SHALL correct the axis
  rates from at least one solve every 5 s. The correction SHALL be proportional-integral
  control on the error between the target and the aim point, resolved into the two axes,
  and it SHALL ignore errors under 10″. Inside that deadband, the held integral SHALL keep
  applying but SHALL NOT be updated. The closed loop's design case is a 1% rate error.
  **Refs**: `docs/research/2026-10-21-pointing-model-and-tracking.md` §3 (Wallace 2002;
  PHD2's algorithms).

**Backlash** (US5):

- **FR-009**: The console SHALL measure each axis's backlash at that axis's first reversal
  on this mount. Before each later reversal of either axis, forecast from its feed-forward
  rate changing sign, it SHALL stop that axis and take up the measured backlash. **Refs**:
  FR-004; `docs/research/2026-10-21-synscan-mount-control.md` §3.
- **FR-010**: The session SHALL keep each axis's measured backlash, with the fixed terms,
  across restarts and nights. It SHALL clear the backlash when the fixed terms are reset.
  **Refs**: FR-009; 014's observing session ("The model's state for the night").
- **FR-011**: A sync taken while an axis last moved opposite to a GoTo's last approach
  SHALL have that axis's measured backlash applied to its axis position before it is
  fitted. Until that axis's backlash is measured, such a sync SHALL be held back from the
  fit. **Refs**: 014's pointing model ("Every solve is a sync", "Slips"); FR-009.

**The zenith keyhole** (US6):

- **FR-012**: When the target's path will come within 2° of the zenith, the console SHALL
  do three things:
  - stop following the target at that circle;
  - slew ahead to where the target will leave the circle;
  - on the target's exit, solve and resume tracking.

  **Refs**: Borkowski (1987), via `docs/research/2026-10-21-pointing-model-and-tracking.md`
  §3.

**Track and the imaging camera in the session** (US7):

- **FR-013**: The session SHALL hold whether tracking is wanted, set by `POST /api/track`
  with `on` true or false, and SHALL report the tracking state and why tracking last
  ended. Turning Track on SHALL be answered 409 with a JSON `error` naming the reason,
  unless all four of these hold:
  - a mount is connected;
  - the night has a sync;
  - the Sun is down;
  - the target is pointable.

  `POST /api/track` SHALL be subject to the same-origin rule, like every route that
  changes the session. Tracking SHALL be off whenever the binary starts. **Refs**: 014's
  GoTo ("Pointable targets only"); 012's observing session ("Same-origin writes").
- **FR-014**: Turning Track on while the target is more than 1′ from the aim point and no
  GoTo is running SHALL start a GoTo first. Turning Track on while a GoTo is running SHALL
  start tracking when the GoTo completes. Turning Track off SHALL stop tracking and both
  axes, leaving the telescope where it is. `POST /api/stop` SHALL also end tracking, as
  stopped by the observer (FR-007). **Refs**: 014's GoTo ("Halts"); 014's FR-020.
- **FR-015**: The session SHALL hold the imaging camera's sensor width and height in
  millimetres and its pixel pitch in micrometres, kept across restarts. Each value SHALL
  be more than 0, and at most 100 mm or 50 µm respectively. A value out of range SHALL be
  refused with 400 and a JSON `error`, changing nothing. An explicit `null` SHALL clear
  the imaging camera, as it clears the site and the target. **Refs**: GLOSSARY.md (imaging
  camera); FR-002; 012's checked edits.
- **FR-016**: While tracking, the session SHALL report a readout of three values:
  - the RMS tracking error over the last 60 s, in arcseconds;
  - the field rotation at the target, from FR-001's rate, in °/min;
  - with an imaging camera set, FR-002's longest exposure for 1 px of smear at the
    sensor's corner, or "no limit".

  **Refs**: FR-001, FR-002, FR-015.

**The page** (US8):

- **FR-017**: While a mount is connected, the guidance block SHALL offer a Track toggle.
  While tracking, it SHALL read out the tracking error (RMS over the last minute, in
  arcseconds), the field rotation (°/min) and, with an imaging camera set, the longest
  exposure for 1 px of smear. These SHALL be rounded to the nearest arcsecond, 0.01°/min
  and 0.1 s, or read "no limit". When tracking ends, the block SHALL say why. Without a
  mount, no Track toggle SHALL be offered. **Refs**: FR-013, FR-016; DESIGN.md (Switch).
- **FR-018**: The Setup page SHALL have an imaging camera section. It SHALL set the
  sensor's width and height in millimetres and its pixel pitch in micrometres, refusing
  values out of range with a message. **Refs**: FR-015; DESIGN.md (Inputs / Fields).
- **FR-019**: `DESIGN.md` and `.impeccable/design.json` SHALL record the Track toggle, the
  tracking readout and Setup's imaging camera section as built. **Refs**: DESIGN.md;
  `.impeccable/design.json`.

**End to end** (US9):

- **FR-020**: *(Owner approval 2.)* The repository SHALL hold two end-to-end proofs:
  - a Rust test of an hour of tracking on the simulator in paused time (a 1% rate error, a
    transit, a keyhole passage, then setting below the minimum altitude) that bounds its
    own wall time to 30 s;
  - a Playwright Track flow in the `simulate` project, with the night pixel check while
    tracking.

  Every earlier test stays unchanged. **Refs**: FR-006 to FR-017.

### Key Entities

- **Tracking state**:
  - `wanted`;
  - a state: off, armed after a GoTo, tracking, or waiting at a keyhole exit;
  - the target;
  - why tracking last ended: `stopped`, `link_lost`, `sun_up`, `too_low`, or a GoTo's
    reason;
  - the readout: RMS error, field rotation, longest exposure or "no limit".
- **Tracking offset**: the integral of the closed loop, one value per axis. It is kept
  apart from the model's IA and IE, cleared when tracking ends, and clamped to 5% of the
  sidereal rate.
- **Measured backlash**: one value per axis, in degrees, left out until that axis is
  measured. It is kept with the fixed terms and cleared with them.
- **Imaging camera**: sensor width and height (mm) and pixel pitch (µm), left out until
  one is set. It is kept across restarts.
- **Simulator settings**: a rate error (a fraction, default 0), beside 014's backlash
  (degrees, default 0.3, one value for both axes).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario above has a committed test that asserts it. In
  that state the `lint` and `test` gates pass, the `test` gate running the workspace's
  cargo tests, `npm test` and `CI=1 npm run e2e`.
- **SC-002**: No story edits an existing test, golden, `scripts/ci/` file,
  `.github/workflows/ci.yml` or `CODEOWNERS` entry.
- **SC-003**: Workspace line coverage stays at or above the 80% floor after every story.
  The hour-long tracking test runs in paused time in under 30 s of wall time.
- **SC-004**: Every path that ends tracking stops both axes, and no API field, page
  control or option keeps the mount tracking with the Sun up or the target too low.

## Assumptions

- **Built on 012, 013 and 014.**
  - 012-targets-and-push-to: the session, the push-to classes, the ephemeris.
  - 013-live-finder: live solving and the live loop (`crates/finder/src/live.rs`, US6),
    the link client with its 1 s rule (`crates/finder/src/link.rs`, US5), the simulator
    and `finder --simulate` (US7), and the `simulate` Playwright project (US10).
  - 014-goto: the mount port's move-at-rate and stop (US1, US2), the simulated mount
    (`crates/simulator/src/mount.rs`, US1), apparent altitude (US4), the pointing model
    and its fixed terms (`crates/finder/src/model.rs`, US5), GoTo and its planner
    (`crates/finder/src/goto.rs`, US6), `POST /api/stop` (US6) and
    `POST /api/model/reset` (US7).

  All three have landed before this spec dispatches. On `main` at 1976dff, 012 has
  landed, 013's US1 to US4, US8 and US9 have landed, and 014 is a refined draft.
- **No operator-staged data.** No story adds a file over about 200 KiB.
- **Out of scope:**
  - field derotation (the readout says how long an exposure may be instead);
  - guiding from the imaging camera; flexure between the finder camera and the tube limits
    imaging-grade guiding anyway;
  - tracking objects that move against the stars (planets, the Moon), which are not
    targets yet.
- **Done by the maintainer, outside the factory:**
  - the Impeccable review of the Track toggle, readout and imaging camera section, after
    US8 lands;
  - the checklist at the telescope. It tracks three targets for 10 minutes each at medium
    power, one through transit. It notes the RMS readout and whether each target stays in
    the eyepiece, and it checks the field rotation readout against a timed photograph. The
    filled-in checklist, with logs, confirms or retunes the PI gains, which are constants
    in code, not settings.
  - the measurement at bring-up of rate updates on the 9600-baud link, and the mount's
    firmware version (firmware below 3.39 may track unevenly; 014 warns at connect).
- **The factory's gates and CI:**
  - The factory's gates are `lint` and `test` (`ergane.yaml`, as of 3fc6a24). `lint`
    runs fmt, clippy and both rustdoc builds. `test` runs the workspace's cargo tests,
    then the page checks in `crates/finder/web`: `npm ci`, `npm run typecheck`,
    `npm run format:check`, `npm test`, `npm run build`, `git status --porcelain -- dist`
    and `CI=1 npm run e2e`. Every story runs both, Rust and page stories alike.
  - The merge queue also runs every `ci.yml` job, among them the coverage floor, msrv,
    deny and machete, which have no factory counterpart (plan.md § Verification).

## Work Graph

```yaml
US1:
  depends_on: []
  implements: [FR-001, FR-002]
US2:
  depends_on: []
  implements: [FR-003, FR-004, FR-005]
US3:
  depends_on: []
  depends_on_merged: [US2]
  implements: [FR-006, FR-007]
US4:
  depends_on: []
  depends_on_merged: [US3]
  implements: [FR-008]
US5:
  depends_on: []
  depends_on_merged: [US4]
  implements: [FR-009, FR-010, FR-011]
US6:
  depends_on: []
  depends_on_merged: [US1, US5]
  implements: [FR-012]
US7:
  depends_on: []
  depends_on_merged: [US1, US5]
  implements: [FR-013, FR-014, FR-015, FR-016]
US8:
  depends_on: []
  depends_on_merged: [US7]
  implements: [FR-017, FR-018, FR-019]
US9:
  depends_on: []
  depends_on_merged: [US6, US8]
  implements: [FR-020]
```

Every edge is a merge edge, because each dependent imports what its predecessor creates:

| Story | Builds on |
|---|---|
| US3 | US2: it sets the simulator's backlash to 0 and loads its truth into the model |
| US4 | US3: it extends `finder::tracking` with the correction |
| US5 | US4: it extends the closed loop, which sees the reversal's jump, and adds the measurement and the take-up |
| US6 | US1: its forecast reads the rates |
| US6 | US5: it extends `finder::tracking` again, after the backlash take-up |
| US7 | US1: its readout reads the rates and the longest exposure |
| US7 | US5: it extends `finder::session` and `src/api.rs` after US5's backlash fields |
| US8 | US7: it renders `/api/track` and the readout |
| US9 | US6: it drives the keyhole |
| US9 | US8: it drives the finished page |

Two pairs may run together:
- **US1 and US2.** US1 touches only `crates/finder/src/ephemeris.rs` and a new test file.
  US2 touches `crates/simulator` and the pointing model's test hook.
- **US6 and US7**, once US5 has merged. US6 touches only `crates/finder/src/tracking.rs`
  and a new test file; US7 touches `crates/finder/src/session.rs`, `src/api.rs`,
  `src/live.rs`, `crates/finder/README.md` and new test files, and never `tracking.rs`
  (the armed state is the session's).
