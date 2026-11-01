---
description: "Task list for 015-tracking: Tracking"
---

# Tasks: Tracking

**Prerequisites** (plan.md § Prerequisites):
- 012, 013 and 014 have landed (`depends_on_landed`), and this trio has been re-read
  against 013 and 014 as landed;
- the owner approvals in spec.md are given;
- the repository is wired for the Ergane merge queue (met since 012);
- the coding agent can run cargo (met since ergane#896);
- `npm run e2e` waits its turn on one host-wide lock (met by #125, at 80cbd51);
- every CI job that builds `scope-link` installs protoc (met by #126, at 7f487e5).

**Rules for every story:**
- Never edit an existing test, golden, `scripts/ci/`, `.github/` or `CODEOWNERS`.
- New Rust tests go in new files under `crates/finder/tests/` or
  `crates/simulator/tests/`.
- `crates/math-core`, `crates/scope-node`, `crates/scope-link` and `proto/` are never
  touched, and no `Cargo.toml` or `Cargo.lock` changes (plan.md § Plan traps, trap 13).
- Every tracking test runs on the simulator in paused virtual time. No story drives real
  hardware.
- Read 013's and 014's modules before editing them, and keep to the names they actually
  landed.
- No step runs a tool outside the factory's gates (trap 5).
- When the operator says a fix landed on `main`, run `git merge origin/main`; never
  cherry-pick or copy an operator's commit (trap 8).
- Console code reads time only through `finder`'s `Clock` and waits only with
  `tokio::time` (trap 12).
- Measure the diff before proposing, with
  `git diff <base> -- . ':(exclude)crates/finder/web/dist' | wc -c`, and keep it under
  64 KiB (trap 3).

## Phase 1: User Story 1 — alt-az rates, field rotation and the longest exposure (Priority: P1)

### Tests first

- [ ] T001 [US1] Write failing tests in a new `crates/finder/tests/rates.rs`:
  - the rates at transit at φ 40°, δ 20° (US1-S1);
  - the rates at hour angle −30° (US1-S2);
  - the 23.5 × 15.6 mm, 3.76 µm sensor at transit: a corner radius of 3,751 px and
    1.63 s (US1-S3);
  - due east and due west, with no limit (US1-S4).

### Implementation

- [ ] T002 [US1] Add `altaz_rates` (dh/dt, dA/dt and dq/dt, with A from north through
  east) to `crates/finder/src/ephemeris.rs`, with doc comments that state the formulas
  and the sign convention.
- [ ] T003 [US1] Add `longest_exposure` and its corner radius to
  `crates/finder/src/ephemeris.rs`: t = 1 / (r · |dq/dt|), and "no limit" below
  1e-9 rad/s.
- [ ] T004 [US1] Run the `lint` and `test` gates. Confirm `crates/math-core` and the
  existing tests in `ephemeris.rs` and `sky.rs` are untouched (US1-S5). Commit the green story (FR-001, FR-002; US1-S1, US1-S2, US1-S3, US1-S4,
  US1-S5).

## Phase 2: User Story 2 — the simulator's rate error, backlash setting and known model (Priority: P1)

### Tests first

- [ ] T005 [US2] Write failing tests in a new
  `crates/simulator/tests/rate_and_backlash.rs`:
  - one per cent fast (US2-S1);
  - no backlash (US2-S2);
  - the defaults, a rate error of 0 and backlash of 0.3° (US2-S4).
- [ ] T006 [US2] Write a failing test in a new `crates/finder/tests/sim_truth.rs`: with
  the truth loaded, the model's prediction at axis altitude 45°, azimuth 120° matches the
  true eyepiece direction within 1″ (US2-S3).

### Implementation

- [ ] T007 [US2] In 014's simulated mount, `crates/simulator/src/mount.rs`:
  - add `rate_error` to `MountConfig`, default 0, applied to every commanded rate;
  - keep 014's `backlash_deg`, one value for both axes with a default of 0.3, which
    already makes the backlash a setting. Every existing `MountConfig` literal ends with
    `..`, so the new field changes no existing test (plan.md § Plan traps, trap 16);
  - add a truth accessor for the base tilt and zero points.
- [ ] T008 [US2] Add a public hook to 014's pointing model in `crates/finder/src/model.rs`
  that loads known base-tilt and zero-point terms, documented as being for tests and the
  simulator.
- [ ] T009 [US2] Run the `lint` and `test` gates. Confirm every existing test in
  `crates/simulator` and `crates/scope-node`, 014's included, is unchanged
  (US2-S5). Commit the green story (FR-003, FR-004, FR-005; US2-S1, US2-S2, US2-S3,
  US2-S4, US2-S5).

## Phase 3: User Story 3 — feed-forward tracking that ends safely (Priority: P1)

### Tests first

- [ ] T010 [US3] Write failing paused-time tests in a new
  `crates/finder/tests/tracking_sim.rs`, with the simulator and the console over the
  in-process link:
  - rates follow the sky for 10 minutes of M42, within 10″ (US3-S1);
  - setting below the minimum altitude (US3-S2);
  - `Tracker::stop` while tracking: both axes stop within 0.5 s, and no rate follows
    (US3-S3);
  - silent heartbeats while tracking, as in 014's US6-S12 (US3-S4);
  - sunrise (US3-S5).

### Implementation

- [ ] T011 [US3] Add `pub mod tracking;` to `crates/finder/src/lib.rs`. Create
  `crates/finder/src/tracking.rs` with the `Tracker` (`start`, `stop`, `state`) on the
  console's clock, driving the mount through 014's `RateAxis` and `StopAll` on 013's
  link client.
- [ ] T012 [US3] Implement feed-forward in `crates/finder/src/tracking.rs`. Each second,
  take the target's apparent place now and 1 s ahead, invert the model for both, and
  send each axis the rate that covers the difference.
- [ ] T013 [US3] Implement the 50 ms ending checks in `crates/finder/src/tracking.rs`:
  - Stop;
  - a lost link, by 013's 1 s rule;
  - the Sun's centre above −0.83°;
  - geometric altitude below the minimum.

  Each records `stopped`, `link_lost`, `sun_up` or `too_low`, stops both axes, and ends
  the 1 Hz loop, so no rate is sent after it (plan.md § Plan traps, trap 15).
- [ ] T014 [US3] Run the `lint` and `test` gates. Commit the green story (FR-006, FR-007;
  US3-S1, US3-S2, US3-S3, US3-S4, US3-S5).

## Phase 4: User Story 4 — closed by the finder camera (Priority: P1)

### Tests first

- [ ] T015 [US4] Write failing paused-time tests in a new
  `crates/finder/tests/tracking_loop.rs`, with a seeded truth-plus-noise pointing feed
  (σ 3″ every 2 s):
  - a mount that runs 1% fast: feed-forward alone drifts more than 1′, and the closed loop
    holds an RMS of at most 20″ (US4-S1);
  - inside the deadband (US4-S2);
  - rendered frames, solved in the test with `finder::solve`, agree with truth plus noise
    within 5″ RMS (US4-S3).

  The noise is drawn from a seeded generator in the test file; the simulator's
  `SplitMix64` is private to its crate.

### Implementation

- [ ] T016 [US4] Add the pointing entry point to `crates/finder/src/tracking.rs`. The
  solver's output and the truth-plus-noise feed both take this one path; the live loop
  joins it in US7.
- [ ] T017 [US4] Implement the PI correction in `crates/finder/src/tracking.rs`:
  - the parallactic rotation into cross-elevation and elevation, and the division by
    cos E;
  - a gain of 0.6, and a 60 s integral held as a per-axis tracking offset;
  - the 10″ deadband, inside which the integral keeps applying and is not updated;
  - a clamp at 5% of the sidereal rate, with the offset cleared on every end.
- [ ] T018 [US4] Run the `lint` and `test` gates. Commit the green story (FR-008; US4-S1,
  US4-S2, US4-S3).

## Phase 5: User Story 5 — backlash at reversals, measured once and kept (Priority: P1)

### Tests first

- [ ] T019 [US5] Write failing paused-time tests in a new
  `crates/finder/tests/backlash_sim.rs`, with 0.3° of backlash on each axis:
  - the first reversal measures the backlash (US5-S1);
  - later reversals take it up (US5-S2);
  - an azimuth reversal for M81 at greatest elongation (US5-S3);
  - syncs after a transit, asserting first that they were taken (US5-S5);
  - the backlash read, then cleared by `POST /api/model/reset` with
    `{"what": "fixed"}`, leaving no `backlash` field (US5-S6).
- [ ] T020 [US5] Write a failing test in a new `crates/finder/tests/backlash_cli.rs`: the
  binary restarted on the same `--state` folder the next day keeps the measured altitude
  backlash exactly (US5-S4).

### Implementation

- [ ] T021 [US5] In `crates/finder/src/tracking.rs`:
  - forecast reversals over 30 minutes from the feed-forward rate's sign;
  - at an axis's first reversal, measure the backlash from the closed loop's settled
    jump;
  - before later reversals, stop the axis, drive through the backlash at 0.5°/s, and
    resume.
- [ ] T022 [US5] In 014's `crates/finder/src/model.rs`, apply the measured backlash to
  syncs taken while an axis last moved against the GoTo's approach (from below). Hold
  such syncs back until that axis's backlash is measured.
- [ ] T023 [US5] In `crates/finder/src/session.rs`, keep the backlash per axis with the
  fixed terms in the state file, and clear it when the fixed terms are reset. In
  `crates/finder/src/api.rs`, add `backlash` to `GET /api/session`, each axis left out
  until measured and the field left out until one is, so
  `crates/finder/tests/session_api.rs` stays as it is (plan.md § Plan traps, trap 4).
- [ ] T024 [US5] Run the `lint` and `test` gates, and measure the diff (trap 3). Commit
  the green story (FR-009, FR-010, FR-011; US5-S1, US5-S2, US5-S3, US5-S4, US5-S5,
  US5-S6).

## Phase 6: User Story 6 — the zenith keyhole (Priority: P2)

### Tests first

- [ ] T025 [US6] Write a failing paused-time test in a new
  `crates/finder/tests/keyhole_sim.rs`: a target at δ = φ − 1° at φ 40°. Tracking pauses
  at the 2° circle, waits at the exit, and is back within 1′ within 30 s of the target
  leaving (US6-S1).

### Implementation

- [ ] T026 [US6] Add the 30-minute path forecast against the 2° zenith circle to
  `crates/finder/src/tracking.rs`, using US1's rates. Enter the `keyhole_wait` state at
  the circle's entry.
- [ ] T027 [US6] In `crates/finder/src/tracking.rs`, slew to the exit through 014's
  planner, with the Sun guard included, calling it as it is without changing it. Wait
  there, and once the forecast position is outside the circle, solve,
  apply the error and resume. A planner refusal ends tracking with that reason.
- [ ] T028 [US6] Run the `lint` and `test` gates. Commit the green story (FR-012;
  US6-S1).

## Phase 7: User Story 7 — Track and the imaging camera in the session (Priority: P2)

### Tests first

- [ ] T029 [US7] Write failing tests in a new `crates/finder/tests/track_api.rs`, through
  the router with the simulator in paused time:
  - on after a GoTo (US7-S1);
  - refused for M8 below the horizon (US7-S2);
  - Track on starts a GoTo (US7-S3);
  - Track off leaves the telescope where it is (US7-S4);
  - an impossible pixel pitch (US7-S6);
  - the tracking readout against FR-001 and FR-002 (US7-S7);
  - a foreign `Origin` on `POST /api/track` (US7-S8);
  - a foreign, bodiless `POST /api/stop` while tracking: 200, both axes at rest within
    1.5 s and for the next 10 s, and tracking ended as stopped (US7-S10);
  - first, with the simulated mount connected, an accepted Track moves an axis, so
    "nothing moves" is asserted against a mount that can (plan.md § Plan traps, trap 9).
- [ ] T030 [US7] Write failing tests in a new `crates/finder/tests/track_cli.rs`, running
  the built binary on one `--state` folder:
  - the imaging camera is kept across a restart, and `"imaging_camera": null` then
    clears it (US7-S5);
  - tracking that moved an axis before a restart is off after it (US7-S9).

### Implementation

- [ ] T031 [US7] In `crates/finder/src/api.rs`:
  - add `POST /api/track` (`{"on": bool}`), with 409 reasons for no mount, no sync, the
    Sun up or a target that is not pointable, and 012's same-origin rule;
  - add `imaging_camera` to `PUT /api/session`'s checked edits (each value more than 0,
    at most 100 mm or 50 µm; `null` clears it);
  - add `track` and `imaging_camera` to `GET /api/session`, each left out of the body
    until it has a value (plan.md § Plan traps, trap 4);
  - make 014's `POST /api/stop` also call `Tracker::stop` with `stopped` and clear the
    armed state, keeping its empty body, its 200 and its missing same-origin check.
- [ ] T032 [US7] In `crates/finder/src/session.rs`:
  - hold the Track state and the armed state;
  - start the tracker when a running GoTo is done, recording the GoTo's reason when it
    fails or is halted, or start a GoTo first when the target is more than 1′ off;
  - stop tracking and both axes on `{"on": false}`;
  - report `ended_reason`.

  Leave the tracker's own module to US6, which may be running beside this story: the
  armed state is the session's (plan.md § Decisions, "Track and GoTo together").
- [ ] T033 [US7] In `crates/finder/src/session.rs`, compute the readout from the tracker's
  samples and US1's functions: the RMS over 60 s, the field rotation in °/min, and the
  longest exposure or "no limit". Persist the imaging camera, and never persist
  tracking.
- [ ] T034 [US7] In 013's live loop, `crates/finder/src/live.rs`, hand each live solve's
  error between the target and the aim point to the tracker's pointing entry point while
  tracking, beside 014's sync intake. Keep `/api/solve` uploads out of it.
- [ ] T035 [US7] Document `POST /api/track`, `imaging_camera`, `track` and Stop ending
  tracking in the crate docs and in `crates/finder/README.md`. Run the `lint` and `test`
  gates, and measure the diff (trap 3). Commit the green story (FR-013, FR-014, FR-015,
  FR-016; US7-S1, US7-S2, US7-S3, US7-S4, US7-S5, US7-S6, US7-S7, US7-S8, US7-S9,
  US7-S10).

## Phase 8: User Story 8 — Track on the console, and Setup's imaging camera section (Priority: P2)

### Tests first

- [ ] T036 [US8] Write failing Vitest tests in a new
  `crates/finder/web/src/components/TrackControl.test.tsx`, on recorded sessions:
  - tracking ended because the target is too low (US8-S1);
  - the readout's rounding, and "no limit" (US8-S2);
  - no toggle without a mount (US8-S3).
- [ ] T037 [US8] Write failing Vitest tests in a new
  `crates/finder/web/src/components/ImagingCameraSection.test.tsx`:
  - a camera set on Setup, which puts the session and adds the longest exposure to the
    readout (US8-S4);
  - a pitch of zero, refused with the allowed range (US8-S5).

### Implementation

- [ ] T038 [US8] Add `postTrack` and the session's `track`, `imaging_camera` and
  `backlash` fields to `crates/finder/web/src/api.ts`, and the recorded sessions to
  `crates/finder/web/src/test/fixtures/`. Add
  `crates/finder/web/src/components/TrackControl.tsx`: a Switch per DESIGN.md, offered
  only with a mount connected and rendering nothing with no `mount` (plan.md § Plan
  traps, trap 10), posting `/api/track`. It reads out the error, field rotation and
  longest exposure, or why tracking ended. Mount it beside 014's GoTo in
  `crates/finder/web/src/components/GuidanceBlock.tsx`.
- [ ] T039 [US8] Add `crates/finder/web/src/components/ImagingCameraSection.tsx`, which
  puts `imaging_camera` with range checks, and mount it in
  `crates/finder/web/src/components/SetupPage.tsx`.
- [ ] T040 [US8] Record the Track toggle, the tracking readout and Setup's imaging camera
  section as built in `DESIGN.md` and `.impeccable/design.json` (US8-S6).
- [ ] T041 [US8] In `crates/finder/web`, run `npm run build` and commit the rebuilt
  `crates/finder/web/dist`. Run the `lint` and `test` gates; the `test` gate's
  `git status --porcelain -- dist` must list nothing, and its `CI=1 npm run e2e` must
  pass with `e2e/session.spec.ts` unchanged. Confirm `package.json` dependencies and every
  existing Vitest file are unchanged (US8-S7). Commit the green story (FR-017, FR-018,
  FR-019; US8-S1, US8-S2, US8-S3, US8-S4, US8-S5, US8-S6, US8-S7).

## Phase 9: User Story 9 — an hour of tracking end to end, and Track on the page (Priority: P3)

### Tests first

- [ ] T042 [US9] Write `crates/finder/tests/tracking_night.rs`: an hour of paused time on
  the simulator, with a 1% rate error and 0.3° of backlash. It covers a transit, a keyhole
  passage, then setting below the minimum altitude, and asserts its own `Instant::elapsed`
  under 30 s (US9-S1).
- [ ] T043 [US9] Write `crates/finder/web/e2e/tracking.spec.ts` for the `simulate`
  project. It runs serially, sets the site, the imaging camera and M42 first, and turns
  Track off at the end:
  - "Tracking M42", with the readout matching the session as rounded (US9-S2);
  - the night pixel check with tracking on, reusing 012's helper (US9-S3).

### Implementation

- [ ] T044 [US9] In `crates/finder/web/playwright.config.ts`, add `e2e/tracking.spec.ts`
  to the `simulate` project's match and to the `default` project's ignore, beside 013's
  `simulate.spec.ts` and 014's `goto.spec.ts`. If the `simulate` project does not yet
  run one file at a time, set `workers: 1` on it (owner approval 2; plan.md § Plan
  traps, trap 14). Change nothing else.
- [ ] T045 [US9] Run the `lint` and `test` gates; the `test` gate's `CI=1 npm run e2e`
  runs every earlier end-to-end test unchanged and green beside the new file (US9-S4).
  Commit the green story (FR-020; US9-S1, US9-S2, US9-S3, US9-S4).

## Execution Order

The stories land in this order:

```text
US1 ───────────────────────────────┬→ US6 ───────────────┐
US2 → US3 → US4 → US5 ─────────────┴→ US7 → US8 ─────────┴→ US9
```

US6 and US7 each depend on both US1 and US5. Every arrow is a merge edge: the later story
starts from a `main` that holds the earlier one.
- **US1** may run beside the US2 → US5 chain, because it touches only
  `crates/finder/src/ephemeris.rs` and a new test file.
- **US6 and US7** may run together once US5 has merged. US6 touches only `tracking.rs`;
  US7 touches only `session.rs`, `api.rs`, `live.rs`, the README and new test files.
- **`[P]`** marks nothing here: every task within a story shares a file with another.
- **Out of scope for every story:** `crates/math-core`, `crates/scope-node`,
  `crates/scope-link`, `proto/`, every `Cargo.toml` and `Cargo.lock`, `scripts/ci/`,
  `.github/` and existing tests.
