# Implementation Plan: Tracking

**Branch**: `015-tracking` | **Date**: 2026-10-28 | **Spec**: [spec.md](spec.md)

This plan records every design decision for the feature. A story that would change one
escalates rather than re-deciding it.

## Summary

Make the console follow the target with the mount's motors:
- feed-forward axis rates from the pointing model, sent every second;
- a proportional-integral correction from the finder camera's solves;
- backlash measured at the first reversal and taken up before later ones;
- a wait at the exit of the 2° zenith circle;
- a safe ending on Stop, a lost link, sunrise or a target too low.

Add the alt-az rates, the field rotation and an imaging camera's longest exposure to
`finder::ephemeris`. Give the simulator a rate error, a backlash setting and a truth hook.
Expose Track and the imaging camera through the session and on the page.

Everything is proven on 014's simulated mount in paused virtual time. `crates/math-core`
is untouched. No existing test changes.

## Technical Context

**Language/Version**: Rust, MSRV 1.96 (`rust-version`), stable in CI. TypeScript with
React 19 for the page.

**Primary Dependencies**:
- already in the workspace: tokio (paused time), axum and serde;
- the simulator (`crates/simulator`, from 013 and 014);
- the page: Vite, Vitest and Playwright.

No new crate and no new npm package is expected. The PI controller, the forecast and the
backlash logic are a few hundred lines of plain Rust. A new crate would be a dependency
change on a critical path: it must build on 1.96, pass `cargo-deny` and `cargo-machete`,
and be justified in its story.

**Storage**: in the session's state file, held by the binary only:
- the measured backlash per axis, beside 014's fixed terms;
- the imaging camera.

Tracking itself is never persisted: it is off at every start.

**Testing**:
- Rust integration tests in new files under `crates/finder/tests/` and
  `crates/simulator/tests/`. The console and the simulator run in one paused-time tokio
  runtime over the in-process link, as 014's GoTo tests do.
- Vitest and Playwright in `crates/finder/web`.

**Target Platform**: the `finder` binary on desktop Linux, WSL and Raspberry Pi, driving
014's scope node and mount. The page runs on Android phones and laptops.

**Constraints**:
- `RUSTFLAGS="-D warnings"`, `--all-features` clippy and rustdoc, and the 80% coverage
  floor, which counts Rust only.
- Long runs must stay cheap: an hour of tracking in under 30 s of wall time.
- `web/dist` is committed and must be the build of its source.
- The night mode is red only.

**Scale/Scope**: one mount, one target at a time, rate updates at 1 Hz, and solves at
0.2–2 Hz.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Nothing here touches the solver or `crates/math-core`. The rates are checked against hand-computed values at φ 40°, δ 20° (Meeus ch. 13 and 14), and tracking against the simulator's own truth. |
| II. Never weaken a gate | No existing test, golden, `scripts/ci/`, `ci.yml` or `CODEOWNERS` is edited. New tests go in new files. The 1% rate-error scenario and the Playwright file are owner approvals (spec § Owner approvals). |
| III. Tests first, independent expectations | Every scenario names its oracle: a worked value at φ 40°, δ 20°, the simulator's true eyepiece direction, its configured backlash and zero points, or a fixed virtual clock. |
| IV. Critical paths | `src/api.rs` gains one route on the HTTP surface, not `proto/`. No `unsafe`, no database-format change, and no dependency change is planned. The scope node and the link are untouched. |
| V. Standalone Rust, honest licensing | No new data or third-party code. |

## Project Structure

```text
crates/finder/
├── README.md                 # US7: POST /api/track, imaging_camera, Stop ends tracking
├── src/
│   ├── ephemeris.rs          # US1: alt-az rates, field rotation, longest exposure
│   │                         # (012's; 014's US4 adds refraction)
│   ├── lib.rs                # US3: `pub mod tracking;`
│   ├── tracking.rs           # US3 (new): feed-forward, ending; US4: PI;
│   │                         # US5: backlash measure and take-up; US6: keyhole
│   ├── model.rs              # 014's US5. US2: test hook to load known terms;
│   │                         # US5: syncs against the approach direction
│   ├── session.rs            # US5: backlash kept with the fixed terms, cleared on reset;
│   │                         # US7: Track state and arming, imaging camera, readout
│   ├── live.rs               # 013's US6 (014's US5 adds syncs). US7: live solves
│   │                         # handed to the tracker while tracking
│   └── api.rs                # US5: `backlash` in the session; US7: POST /api/track,
│                             # imaging_camera in PUT /api/session, `track`, and
│                             # POST /api/stop ending tracking
├── tests/
│   ├── rates.rs              # US1 (new)
│   ├── sim_truth.rs          # US2 (new)
│   ├── tracking_sim.rs       # US3 (new)
│   ├── tracking_loop.rs      # US4 (new)
│   ├── backlash_sim.rs       # US5 (new)
│   ├── backlash_cli.rs       # US5 (new)
│   ├── keyhole_sim.rs        # US6 (new)
│   ├── track_api.rs          # US7 (new)
│   ├── track_cli.rs          # US7 (new)
│   └── tracking_night.rs     # US9 (new)
└── web/
    ├── src/api.ts                                        # US8: postTrack, the new fields
    ├── src/components/TrackControl.tsx (+ test)          # US8 (in the guidance block)
    ├── src/components/ImagingCameraSection.tsx (+ test)  # US8 (on SetupPage)
    ├── src/components/GuidanceBlock.tsx, SetupPage.tsx   # US8: mount the two parts
    ├── src/test/fixtures/                                # US8: recorded sessions
    ├── dist/                                             # US8: rebuilt
    ├── e2e/tracking.spec.ts                              # US9 (new, `simulate` project)
    └── playwright.config.ts                              # US9: owner approval 2's lines
crates/simulator/
├── src/mount.rs              # 014's US1. US2: rate error and backlash settings,
│                             # truth accessor
└── tests/rate_and_backlash.rs  # US2 (new)
DESIGN.md, .impeccable/design.json                         # US8
```

The simulated mount's module and the pointing model's module are 014's, at the paths
above (014 plan.md, § Project Structure). They are re-read against 014 as landed before
the flip (§ Prerequisites, item 1). No story touches `crates/scope-node`,
`crates/scope-link` or `crates/math-core`, and none adds a crate.

## Interfaces

**`finder::ephemeris`** (US1) adds:

```text
altaz_rates(lat_deg, dec_deg, hour_angle_deg) -> AltAzRates {
    alt_arcsec_per_s, az_arcsec_per_s, field_rotation_deg_per_h }
longest_exposure(sensor_w_mm, sensor_h_mm, pitch_um,
                 field_rotation_rad_per_s, smear_px) -> Exposure { Seconds(f64) | NoLimit }
```

A form taking altitude and azimuth is used for the "Due east" case. The corner radius is
public for its test.

**`finder::tracking`** (US3) holds a `Tracker` driven by the console's clock:
- `start(target)`, `stop(reason)` and `state()`;
- a pointing entry point that takes the error between the target and the aim point. A
  solve feeds it after the solver, and a test feeds it truth plus noise.

`state()` reports:
- the state: `off`, `tracking` or `keyhole_wait` (US6 adds the last);
- `ended_reason`: `stopped`, `link_lost`, `sun_up`, `too_low`, or the GoTo's reason;
- the error samples for the RMS;
- the tracking offset per axis.

The tracker drives the mount through 014's link calls (`RateAxis`, `StopAll`) and reads
the link's state from 013's link client. `armed` is not a tracker state: the session
holds it while Track waits for a GoTo, and calls `start` when the GoTo ends (US7).

**`POST /api/track`** (US7) takes `{"on": true | false}` and answers 200 with the session.
- It answers 409 with `{"error": <reason>}` when there is no mount connected, no sync
  tonight, the Sun is up, or the target is not pointable.
- It is subject to 012's same-origin rule.

**`POST /api/stop`** (014's US6) keeps its body, answer and missing same-origin check.
From US7 it also calls `Tracker::stop` with `stopped` and clears the armed state, so the
next second's rates never restart the axes.

**The session** gains three fields, each left out of the body until it has a value, as
013's and 014's are (§ Plan traps, trap 4):
- `track?: {wanted, state, target?, ended_reason?, readout?: {rms_arcsec,
  field_rotation_deg_per_min, longest_exposure_s?: number | "no limit"}}`, where `state`
  is the tracker's or `armed`. It appears once Track is first turned on in a run of the
  binary, and is never persisted;
- `imaging_camera?: {width_mm, height_mm, pitch_um}`, set through `PUT /api/session` with
  012's checked-edit rules (each value more than 0, at most 100 mm or 50 µm). An explicit
  `null` clears it, as it clears the site and the target;
- `backlash?: {alt_deg?, az_deg?}`, each axis left out until it is measured. It is kept
  in the state file beside 014's fixed terms and cleared with them.

**The simulator** (US2) gains `rate_error` (a fraction, default 0) and a per-axis
`backlash_deg` (default 0.3), on 014's simulated mount in `crates/simulator/src/mount.rs`.
It also gains a truth accessor (base tilt and zero points) that a test can hand to the
pointing model's load hook in `crates/finder/src/model.rs`.

## Shapes that existing tests pin

These are not changed by any story:
- **012–014 tests:** every Rust and Vitest test, and every Playwright test in the default,
  `session` and `simulate` projects.
- **A fresh session's whole body:** `crates/finder/tests/session_api.rs` compares a fresh
  read's whole body with 012's fields, so `track`, `imaging_camera` and `backlash` are
  left out until they have a value, as 014's `goto`, `model` and `mount` are. 012's
  checked edits (`deny_unknown_fields`, null clears the site or target) keep their
  answers; `imaging_camera` is one more checked field.
- **Playwright:** the servers and projects are unchanged, except by owner approval 2: the
  `simulate` project's match and the `default` project's ignore gain
  `e2e/tracking.spec.ts`, and the `simulate` project runs one file at a time.
- **The simulator's defaults:** a rate error of 0 and backlash of 0.3° per axis are the
  defaults, so 014's simulator, GoTo and model tests stand unchanged.
- **014's mount path:** GoTo's approach-from-below, the slip thresholds (0.5° once the
  base tilt is fitted, 2.5° before) and the rule that every solve is a sync.
- **Geometry:** `sky::alt_az` stays geometric (the Meeus 13.b test). Tracking's ending
  judges geometric altitude, as "pointable" does.
- **Stop:** `POST /api/stop` takes no body and skips the same-origin check. It wins over
  tracking: from US7 it also ends tracking.
- **014's sync rule:** a frame is a sync when both axes move at no more than 0.2°/s during
  it. Tracking rates stay under that outside the 2° zenith circle (about 0.1°/s in
  azimuth at its edge), so tracking frames are syncs; the take-up at 0.5°/s and the
  keyhole slew are not.

## Decisions and the alternatives rejected

**The rates.** All are checked by hand at φ 40°, δ 20°:
- dh/dt = −ω cos φ cos δ sin H / cos h;
- dA/dt = ω (sin φ − sin δ sin h) / cos² h;
- dq/dt = −ω cos φ cos A / cos h, with A measured from north through east.

Meeus's own formula uses azimuth from the south, hence the sign. dq/dt is +33.69°/h at
φ 40°, δ 20°, transit. These rates serve the readout, the keyhole forecast and the tests'
oracles, not the feed-forward.

**Feed-forward from the model, sent every second.** Each second, the tracker:
1. takes the target's apparent place now and 1 s ahead (014's refraction);
2. inverts the model for both, to axis positions;
3. sends each axis the rate that covers the difference in that second, plus the
   correction.

That carries every model term (base tilt, CA, NPAE, TF) into the rates.
- *Rejected:* plain dh/dt and dA/dt, which miss the model's terms.

**The closed loop: PI on the error, resolved into the axes.** Following Wallace (2002):
1. rotate the error by the parallactic angle into cross-elevation and elevation;
2. divide the azimuth part by cos E.

The control constants:
- a proportional gain of 0.6 on each axis, applied as a rate over the next interval;
- a slow integral, time constant about 60 s, held as a tracking offset per axis and
  cleared when tracking ends;
- errors under 10″ ignored. Inside the deadband the integral keeps applying and is not
  updated.
- the integral clamped to 5% of the sidereal rate. That is well above the 1% error the
  loop is designed for, and low enough that a fault cannot drive the axes away.

The integral is kept apart from IA and IE because every tracking solve is also a sync, and
the zero-point refit would undo anything folded into them. The gains are fixed constants
in code, tuned on the simulator, not settings. The on-sky checklist confirms or retunes
them.
- *Rejected:* a model refit only, with no PI. Refits need spread syncs, and tracking gives
  syncs along one track.

**Backlash at reversals.** The altitude axis reverses at every transit. The azimuth axis
reverses at greatest elongation for any target with δ > φ, where cos H = tan φ / tan δ
and sin h = sin φ / sin δ; at φ 40° that includes M81, M82, M101 and Capella. For M81
(δ +69.07°) that is H ≈ 71.3° at h ≈ 43.5°.
- **Measuring.** At an axis's first reversal in the mount's life, the closed loop sees
  the error jump along that axis. The size of the jump, settled over the next solves, is
  that axis's backlash. Later reversals refine it.
- **Keeping.** The measurement belongs to the mount, so it is kept with the fixed terms
  and cleared when they are reset.
- **Syncs.** A GoTo's last approach is from below, so its syncs carry the backlash on the
  upward side. A sync taken while an axis last moved the other way, such as tracking down
  after transit, has that axis's measured backlash applied before fitting. It is held
  back until a backlash is measured. Otherwise up to 0.3° would bias IE without reaching
  the 0.5° slip threshold.
- **Taking it up.** Before a planned reversal of either axis, the tracker:
  1. stops the axis;
  2. drives it through the measured backlash at 0.5°/s;
  3. resumes the feed-forward rate.

  A reversal is planned when the axis's feed-forward rate changes sign within the
  30-minute forecast. Until a backlash is measured, the closed loop recovers within about
  15 s, relying on 014's stop-before-reverse.

**The zenith keyhole: wait at the exit.** The tracker forecasts the target's path for the
next 30 minutes. When the path enters the 2° circle round the zenith, the tracker:
1. stops at the entry;
2. slews ahead, through 014's planner with the Sun guard included, to where the path
   leaves the circle;
3. waits;
4. once the target's forecast position is outside the circle, solves, applies the error
   and resumes.

A target 1° from the zenith spends about 18 minutes in the 2° circle. The azimuth rate
there is only about 0.1°/s at 2°, but it changes fast, and the plan's rule keeps the loop
out of that region.
- *Rejected:* chasing the target through the circle.

**Ending.** The tracker checks four conditions on the same 50 ms tick that 014 uses for the
Sun during a GoTo, so the scenarios' 1 s bounds have margin:
- Stop (`Tracker::stop`, which `POST /api/stop` calls from US7);
- a lost link (013's 1 s rule; the node's watchdog also stops the axes);
- the Sun's centre above −0.83°;
- the target's geometric altitude below the minimum altitude.

Any of them stops both axes and records why. Once ended, the tracker sends no rate until
it is started again. The session reports the reason, and the page shows it.

**Track and GoTo together.** `POST /api/track {"on": true}` arms tracking for the
session's target:
- if a GoTo is running, tracking starts when it completes;
- if the target is more than 1′ from the aim point and no GoTo is running, a GoTo starts
  first;
- `{"on": false}` stops tracking and the axes, leaving the telescope where it is.

The armed state is the session's (`crates/finder/src/session.rs`): it watches 014's GoTo
state and calls `Tracker::start` when the GoTo is done, or records the GoTo's reason when
it fails or is halted. So US7 never edits `tracking.rs`, and US6 and US7 can run together.

**What feeds the tracker.** 013's live loop (`crates/finder/src/live.rs`) hands each live
solve's error between the target and the aim point to the tracker's pointing entry point
while tracking, from US7, beside 014's sync intake. Uploaded frames (`/api/solve`) never
reach it. US4's tests feed the entry point themselves.

**The readout.**
- The RMS error is taken over the last 60 s.
- The field rotation at the target comes from dq/dt, in °/min.
- With an imaging camera set, the longest exposure is t = 1 / (r · |dq/dt|), for 1 px of
  smear at the sensor's corner. Here r is the half-diagonal in pixels and dq/dt is in
  radians per second.
- "No limit" applies when |dq/dt| is below 1e-9 rad/s.

**The rate-error scenario (owner approval 1).** The earlier plan's "0.5% drifts more than
1′ in 10 min" is false: 0.5% of the 8,985″ that M42 moves in 10 minutes is about 45″. The
scenario uses 1% (about 90″).

**Tests: truth plus noise for long runs.** Rendering and solving a frame every 2 s for an
hour of virtual time would dominate the coverage job. Long tests instead feed the tracker
the simulator's true eyepiece direction plus Gaussian noise (σ 3″, seeded) as its
pointings, through the same entry point a solve uses after the solver. One 2-minute run
on rendered, solved frames proves that the two paths agree: the test renders the
simulator's frames, solves them with `finder::solve`, and hands each error to the entry
point, as the live loop does from US7. The oracles are the simulator's truth. The noise
is seeded and drawn in the test file, because the simulator's `SplitMix64` is private to
`crates/simulator`.
- *Rejected:* rendering every frame of the long runs.

**Out of scope.** Field derotation (the readout states the limit instead), guiding from
the imaging camera, and objects that move against the stars.

## User Story 1 — Alt-az rates and the longest exposure

Add `altaz_rates` and `longest_exposure`, with its corner radius, to
`crates/finder/src/ephemeris.rs`. Both take explicit inputs. The tests go in the new
`crates/finder/tests/rates.rs`, using the hand-checked values at φ 40°, δ 20°.

## User Story 2 — The simulator's settings and truth

In 014's simulated mount, `crates/simulator/src/mount.rs`:
- add `rate_error`, applied to every commanded rate;
- add the per-axis `backlash_deg`, replacing the fixed 0.3° with a setting of the same
  default;
- add a truth accessor for the base tilt and zero points.

In the pointing model (`crates/finder/src/model.rs`), add a public hook that loads known
base-tilt and zero-point terms, documented as being for tests and the simulator.

## User Story 3 — Feed-forward and ending

Add `crates/finder/src/tracking.rs`, with the `Tracker`, the 1 Hz feed-forward through the
model's inverse, and the 50 ms ending checks, and declare it in `crates/finder/src/lib.rs`.
Each end stops both axes through 014's `StopAll` and records the reason, and no rate is
sent after it. A lost link uses 013's 1 s rule. The tests drive the tracker directly:
Stop is `Tracker::stop` here, and `POST /api/stop` reaches it from US7.

## User Story 4 — The closed loop

In `crates/finder/src/tracking.rs`, add the PI correction:
- the parallactic rotation and the division by cos E;
- the gain of 0.6, the 60 s integral as the tracking offset, the 10″ deadband and the 5%
  clamp;
- the offset cleared on every end.

Add the truth-plus-noise pointing feed (seeded σ 3″) in `crates/finder/tests/tracking_loop.rs`,
and the 2-minute rendered-frame run beside it, which solves in the test (§ Decisions,
"Tests: truth plus noise").

## User Story 5 — Backlash

In `crates/finder/src/tracking.rs`:
- add the 30-minute reversal forecast from the feed-forward rate's sign;
- measure the backlash from the closed loop's settled jump at the first reversal;
- take it up at 0.5°/s before later reversals.

In `crates/finder/src/model.rs`, apply or hold back syncs taken against the approach
direction. In `crates/finder/src/session.rs`, keep the backlash with the fixed terms in
the state file, and clear it on 014's fixed-terms reset (`{"what": "fixed"}`). In
`crates/finder/src/api.rs`, add `backlash` to `GET /api/session`, left out until an axis
is measured.

## User Story 6 — The zenith keyhole

In `crates/finder/src/tracking.rs`, add the 30-minute path forecast against the 2° circle.
At the circle's entry, stop, slew to the exit through 014's planner, wait, then solve and
resume. A Sun-guard refusal from the planner ends tracking with that reason.

## User Story 7 — Track and the imaging camera in the session

In `crates/finder/src/api.rs`:
- add `POST /api/track`, with its 409 reasons and the same-origin rule;
- add `imaging_camera` to `PUT /api/session`'s checked edits, `null` clearing it;
- add `track` and `imaging_camera` to `GET /api/session`, each left out until it has a
  value;
- make 014's `POST /api/stop` also end tracking, as stopped by the observer.

In `crates/finder/src/session.rs`:
- hold the Track state and the armed state, and start the tracker after a running GoTo,
  or start a GoTo first;
- compute the readout from the tracker's samples and US1's functions;
- persist the imaging camera, and never persist tracking.

In 013's live loop, `crates/finder/src/live.rs`, hand each live solve's error at the aim
point to the tracker while tracking. Document the route and the fields in the crate docs
and `crates/finder/README.md`. `tracking.rs` is not edited (§ Decisions, "Track and GoTo
together").

## User Story 8 — The page

**`TrackControl`.** It goes in the guidance block, beside 014's GoTo. It shows a Switch
per DESIGN.md, and only with a mount connected: with no `mount` in the session it renders
nothing, as on the `default` and `session` servers (§ Plan traps, trap 10). It reads out
the error, field rotation and longest exposure, rounded to 1″, 0.01°/min and 0.1 s or
"no limit", or why tracking ended.

**`ImagingCameraSection`.** It goes on the Setup page, using DESIGN.md's inputs. It puts
`imaging_camera`, and refuses values out of range with the allowed range.

`src/api.ts` gains `postTrack` and the session's new fields. The recorded sessions go in
`src/test/fixtures/`, inside the existing package. Record both parts in `DESIGN.md` and
`.impeccable/design.json`. Rebuild `web/dist`.

## User Story 9 — End to end

**`crates/finder/tests/tracking_night.rs`.** An hour of paused time on the simulator:
- a 1% rate error and 0.3° of backlash;
- a target chosen to transit, pass within 2° of the zenith and then set through the
  minimum altitude within the hour;
- the test asserts its own `Instant::elapsed` is under 30 s.

**`crates/finder/web/e2e/tracking.spec.ts`.** It goes in the `simulate` project and covers
two things:
- "Tracking M42", with the readout matching the session as rounded;
- the night pixel check with tracking on, reusing 012's helper.

It runs serially, sets the site, the imaging camera and M42 first, and turns Track off at
the end. **`playwright.config.ts`** changes only by owner approval 2: the `simulate`
project's match and the `default` project's ignore gain `e2e/tracking.spec.ts`, and the
`simulate` project gets `workers: 1` unless 014 has set it (§ Plan traps, trap 14). The
`test` gate already runs `CI=1 npm run e2e`, so the file is gate evidence with no change
to the gate.

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

1. **012, 013 and 014 have landed**, and this trio has been re-read against them as
   landed: every path marked "013's" or "014's" in § Project Structure, § Interfaces and
   § Shapes that existing tests pin, the names of 014's link calls, simulated-mount
   settings, model reset and session fields, and owner approval 2's Playwright lines.
   012: met (108a5a8). 013: not yet; on `main` at 1976dff, US1 to US4, US8 and US9 have
   landed. 014: not yet; refined at cab5143, still draft. `depends_on_landed` holds
   dispatch until then.
2. **The owner approvals are given** (spec § Owner approvals). Flipping to `ready` records
   them.
3. **The repository is wired for the Ergane merge queue.** Met since 012: `ergane.yaml`
   and `ergane-gates.yml` are on `main`, and 012 and 013 have run through the queue.
4. **The factory's coding agent can run cargo.** Met since ergane#896 (deployed
   2026-10-28). No story here changes `Cargo.lock`.
5. **Two end-to-end runs never meet.** Met: #125, at 80cbd51 (013 plan, Prerequisite 6).
   US8 and US9 run `CI=1 npm run e2e` through it.
6. **Every CI job that builds `scope-link` installs protoc.** Met: #126, at 7f487e5. 015
   changes no proto, but the `finder-page` job builds `finder`, which depends on
   `scope-link` since 013's US5.

No operator-staged data is needed. No story adds a file large enough to need staging, and
none goes under `crates/` outside an existing crate (§ Plan traps, trap 11).

## Plan traps

Lessons from building 012 and 013 on this factory, as 014's plan records them (014
plan.md § Plan traps), each with its mechanism and the check that keeps it from
recurring here. They bind every story.

1. **Gate evidence or nothing.** The judge credits only the gate results and the story's
   diff. A Then clause that names a check only CI runs fails correct work: 012's page
   scenarios did, until #118 put the `finder-page` checks in the `test` gate. *Check:*
   every Then clause names a committed test, a command a gate runs, or the diff. US8-S7
   names the `test` gate's `npm run build` and `git status --porcelain -- dist`, and
   US9-S1 names the `test` gate, not the coverage job, which only the queue runs.
2. **A named command is read literally.** The judge refused 012's US3 docs scenario until
   the `lint` gate ran the exact command it named (#113, #114). *Check:* a scenario that
   names a command copies it from `ergane.yaml` verbatim.
3. **One evidence budget.** The story's diff and both gates' output share the judge's
   128 KiB, and the diff's share is 98,304 bytes. 013's US2 was shown whole at 85,558
   bytes on its failed second attempt and at 75,986 on its passing third. *Check:* a
   story keeps its diff under 64 KiB (`dist/` is `-diff`), measured before proposing with
   `git diff <base> -- . ':(exclude)crates/finder/web/dist' | wc -c`. US5 (four source
   files and two test files) and US7 (four source files, the README and two test files)
   are the largest; one that cannot fit escalates rather than dropping a test.
4. **An additive field still breaks a whole-body test.** 012's US5 added `guidance` to
   session reads, and the existing whole-body comparison stopped the story on a question
   (8a357f48cda5). *Check:* `track`, `imaging_camera` and `backlash` are left out of the
   session until they have a value (§ Interfaces), and
   `crates/finder/tests/session_api.rs` is unchanged.
5. **The sandbox has only the gates' tools.** The factory's toolchain is stable Rust with
   rustfmt and clippy, Node 24 and the seeded Chromium. It has no `cargo-machete`, no
   `cargo-deny`, no Rust 1.96 and no `ergane` CLI. 013's builders stopped to ask for
   `ergane` before handing off (questions ea1213db3c0b, 4e4fc164492d, c2f71dc3b09d and
   6c5af5610caf); the factory's verification runs that check itself. *Check:* no task
   runs a tool outside the gates, and no story adds a dependency, so deny, machete and
   msrv see nothing new.
6. **The gate's ports are the host's.** Every builder and gate shares the host's network,
   so each e2e run binds 127.0.0.1:8097–8099. US1 runs beside the US2 → US5 chain, and
   US6 beside US7. *Check:* every e2e run goes through `npm run e2e`, which waits its turn
   on one host-wide lock (#125); a test binds `127.0.0.1:0`, and only
   `playwright.config.ts` names a fixed port.
7. **Each worktree builds on its own** (#118). *Check:* no story sets `CARGO_TARGET_DIR`,
   and a test's temporary state goes under `target/` or the system's temporary folder.
8. **Fixes land on `main` first.** When the operator fixes a gate, a fixture or this spec
   mid-epic, the fix merges first and the story merges `origin/main`. 012's US3
   cherry-picked two gate fixes and spent a recovery cycle; 013's US2 copied the operator's
   fixture files instead of merging, so its branch carried a stale spec. *Check:* a story
   never cherry-picks or copies an operator's commit; it runs `git merge origin/main`.
9. **A negative assertion passes when nothing happens.** 013's US2-S5 compared two empty
   star lists on a starless frame (013 plan, trap 12). Here "nothing moves", "no rate is
   sent", "no epoch" and "no `backlash`" would pass on a mount that is not connected or a
   session that never measured. *Check:* each negative scenario also asserts its positive
   side in the same test: an accepted Track moves an axis of the connected simulated
   mount before a refused or restarted one is asserted to move nothing (US7-S2, S8, S9);
   rates were being sent before Stop (US3-S3, US7-S10); the syncs after the transit were
   taken (US5-S5); and the backlash was read before the reset clears it (US5-S6).
10. **The phone's first screen has 8 px to spare.** With a target, 012's legend ends at
    836 px of 844 at 390 × 844, so anything taller above it fails the `test` gate's e2e
    (013 plan, trap 10). Track makes the guidance block taller. *Check:* `TrackControl`
    renders nothing when the session has no `mount`, as on the `default` and `session`
    servers; US8 and US9 run `CI=1 npm run e2e` with `e2e/session.spec.ts` unchanged and
    green.
11. **A folder under `crates/` needs a `Cargo.toml`.** The workspace's members are
    `crates/*`, so a folder there without one fails every cargo command (013 plan, trap
    11). *Check:* 015 adds no crate; its fixtures go in
    `crates/finder/web/src/test/fixtures/`, inside an existing package.
12. **Console time is the session's clock, in paused tokio time.** The tracker runs on the
    console, so it reads time through `finder`'s `Clock` (013's `VirtualClock` under
    `--simulate` and in tests) and waits only with `tokio::time`. 013's node-side rule is
    the same (#133). *Check:* `tracking.rs` adds no `Instant::now()` or `SystemTime`; the
    only `std::time::Instant` is US9's own wall-time bound in `tracking_night.rs`, which
    paused time does not affect.
13. **`Cargo.lock` is updated by cargo, once, or not at all.** `--locked` refuses a needed
    update, and a hand edit cannot lock a crate (013's US2, question 995c4dfefc6c).
    *Check:* 015 plans no new crate or feature, so `Cargo.lock` and every `Cargo.toml`
    are unchanged; a story that finds it needs one escalates.
14. **One `finder --simulate` server serves the whole `simulate` project.** Playwright runs
    a project's files in parallel workers by default, and `simulate.spec.ts`,
    `goto.spec.ts` and `tracking.spec.ts` each set the session's target and move the
    simulated mount, so two at once break each other. *Check:* the `simulate` project
    runs one file at a time (`workers: 1`, owner approval 2), and `tracking.spec.ts`
    turns Track off before it ends.
15. **An ended tracker must stay quiet.** A 1 Hz loop that misses an end sends the next
    second's rate and restarts axes that Stop, the watchdog or the Sun guard just halted.
    *Check:* every end is recorded before `StopAll` is sent and no rate follows it
    (US3-S3); `POST /api/stop` ends tracking (US7-S10); a lost link never resumes
    (US3-S4).

## Risks

| Risk | Mitigation |
|---|---|
| Motor firmware below 3.39 reportedly tracks unevenly at variable rates. | 014 warns at connect. The closed loop absorbs small unevenness, and bring-up records the firmware. |
| Rate updates every second go over a 9600-baud single-wire link. | Each update is a few short commands per axis, well under the link's capacity. It is measured at bring-up, outside the factory. |
| The altitude axis reverses at transit. | Stop-before-reverse costs a fraction of a second, and backlash far more. The first reversal measures the backlash and later ones take it up ahead of time, so the error stays within the 30″ the scenario allows. A real mount's backlash may vary along the axis; the closed loop absorbs what the measurement misses. |
| Imaging at 1800 mm on an alt-az mount needs short subs and derotation when stacking. | The readout states the limit. Derotation is out of scope. |
| A rate error far beyond 1% (a slipping clutch, a wrong counts-per-turn). | 1% is the closed loop's design case, and the integral is clamped at 5%. Larger errors point to a fault that the model and the slip rule catch at the next sync. |
| The simulator's PI gains do not suit the real mount. | They are the starting point. The on-sky checklist confirms or retunes them as constants in code. |
| A story's diff is past what the judge is shown whole. US5 and US7 are the largest. | § Plan traps, trap 3: measured before proposing; a story that cannot fit escalates rather than dropping a test. |
| 013 or 014 lands with a name or shape other than § Interfaces assumes. | Prerequisite 1: the operator re-reads the trio against both as landed before the flip, and fixes it on `main` first. |
| The `simulate` project's e2e files meet on one server. | § Plan traps, trap 14: one file at a time, under owner approval 2. |

## Refinement Record

Reviewed on 2026-10-31 by the plate-solver operator against `main` at 1976dff (013's US1
to US4, US8 and US9 landed) and 014's refined trio at cab5143, and re-read against 013
and 014 as landed before the flip (§ Prerequisites, item 1). Writes come from the task
text. Shared writes name the direct merge edge that orders them inside 015, and the 012,
013 and 014 stories that last write the same file; orderings carried only by a chain
(US2 and US5 on `model.rs`, US3 and US5 on `tracking.rs`) follow its edges. Existing
tests named here are the ones on `main`; 013's and 014's later tests are kept under
`preserved_behavior`.

```yaml
version: 1
reviewed_revision: 1976dff1bc8a5ef28235db2c58f348232e6514f4
stories:
  US1:
    tests:
      - path: crates/finder/tests/rates.rs
        status: new
        disposition: add
        assertion: "at latitude 40 and declination 20: transit rates of 0 and 41.33 arcsec/s and +33.69 deg/h; at hour angle -30 deg, +10.07 and 18.45 arcsec/s; the 23.5 x 15.6 mm, 3.76 um sensor at transit gives a corner radius of 3,751 px and 1.63 s; due east and due west give no limit"
        decision: "spec.md:US1-S1"
      - path: crates/finder/src/ephemeris.rs
        status: existing
        disposition: preserve
        assertion: "012's Sun and rise, transit and set tests in ephemeris.rs, and 014's refraction tests once landed, pass unchanged"
        decision: "spec.md:US1-S5"
      - path: crates/finder/src/sky.rs
        status: existing
        disposition: preserve
        assertion: "alt_az_matches_meeus_example_13b passes unchanged, and sky::alt_az stays geometric"
        decision: "spec.md:US1-S5"
    preserved_behavior:
      - "crates/math-core is untouched (spec.md US1-S5)"
      - "the rates serve the readout, the keyhole forecast and the tests' oracles, not the feed-forward (plan.md Decisions)"
    checks:
      development:
        - "cargo test -p finder --locked --test rates"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: Bryan, through the merge queue"
    writes:
      - crates/finder/src/ephemeris.rs
      - crates/finder/tests/rates.rs
    shared_writes:
      - path: crates/finder/src/ephemeris.rs
        other_spec: 014-goto
        other_story: US4
        disposition: merge_order
        reason: "015 depends_on_landed 014, whose US4 adds refraction to ephemeris.rs"
    open_questions: []
  US2:
    tests:
      - path: crates/simulator/tests/rate_and_backlash.rs
        status: new
        disposition: add
        assertion: "a 1% rate error turns an axis commanded at 0.01 deg/s for 100 s through 1.01 deg within 0.001; with backlash 0, up 1 deg and down 1 deg returns within 1 arcsec; a mount built with no settings has a rate error of exactly 0 and backlash of exactly 0.3 deg per axis"
        decision: "spec.md:US2-S1"
      - path: crates/finder/tests/sim_truth.rs
        status: new
        disposition: add
        assertion: "with the simulator's base tilt (1 deg toward 130) and zero points loaded, the model predicts the true eyepiece direction at axis altitude 45, azimuth 120 within 1 arcsec"
        decision: "spec.md:US2-S3"
      - path: crates/simulator/tests/node.rs
        status: existing
        disposition: preserve
        assertion: "013's simulated node test passes unchanged"
        decision: "spec.md:US2-S5"
      - path: crates/simulator/tests/sky.rs
        status: existing
        disposition: preserve
        assertion: "013's simulated sky test passes unchanged"
        decision: "spec.md:US2-S5"
    preserved_behavior:
      - "014's crates/simulator/tests/mount.rs, model_sim.rs and goto_sim.rs pass unchanged with the defaults of 0 and 0.3 deg (spec.md US2-S4)"
      - "crates/scope-node is untouched (spec.md US2-S5)"
      - "the load hook is documented as being for tests and the simulator; the model's fit is unchanged (plan.md User Story 2)"
    checks:
      development:
        - "cargo test -p simulator --locked --test rate_and_backlash"
        - "cargo test -p finder --locked --test sim_truth"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: Bryan, through the merge queue"
    writes:
      - crates/simulator/src/mount.rs
      - crates/simulator/tests/rate_and_backlash.rs
      - crates/finder/src/model.rs
      - crates/finder/tests/sim_truth.rs
    shared_writes:
      - path: crates/simulator/src/mount.rs
        other_spec: 014-goto
        other_story: US1
        disposition: merge_order
        reason: "015 depends_on_landed 014, whose US1 creates the simulated mount"
      - path: crates/finder/src/model.rs
        other_spec: 014-goto
        other_story: US5
        disposition: merge_order
        reason: "015 depends_on_landed 014, whose US5 creates the pointing model"
    open_questions: []
  US3:
    tests:
      - path: crates/finder/tests/tracking_sim.rs
        status: new
        disposition: add
        assertion: "in paused time with backlash 0 and the truth loaded: a rate to each axis at least once a second and the true eyepiece within 10 arcsec of M42 for 10 minutes; at a geometric altitude of 15 deg tracking ends too_low with both axes stopped within 1 s; Tracker::stop stops both within 0.5 s with no rate after it and reports stopped; silent heartbeats stop both 1.0 +/- 0.1 s after the last, end link_lost and do not resume; the Sun at -0.83 deg commands the stop within 1 s and ends sun_up"
        decision: "spec.md:US3-S1"
      - path: crates/finder/tests/session_api.rs
        status: existing
        disposition: preserve
        assertion: "a fresh session's whole body is unchanged; US3 adds no session field"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "the scope node and the link are untouched; tracking uses 014's RateAxis and StopAll (spec.md Input)"
      - "an ended tracker sends no rate until started again (plan.md Plan traps, trap 15)"
    checks:
      development:
        - "cargo test -p finder --locked --test tracking_sim"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: Bryan, through the merge queue; outside the factory, tracking at the telescope (spec.md Assumptions)"
    writes:
      - crates/finder/src/lib.rs
      - crates/finder/src/tracking.rs
      - crates/finder/tests/tracking_sim.rs
    shared_writes:
      - path: crates/finder/src/tracking.rs
        other_spec: 015-tracking
        other_story: US4
        disposition: merge_order
        reason: "US4 depends_on_merged US3, and adds the PI correction"
      - path: crates/finder/src/lib.rs
        other_spec: 014-goto
        other_story: US6
        disposition: merge_order
        reason: "015 depends_on_landed 014, whose US6 last declares a module in it"
    open_questions: []
  US4:
    tests:
      - path: crates/finder/tests/tracking_loop.rs
        status: new
        disposition: add
        assertion: "with a 1% rate error and seeded truth plus 3 arcsec of noise every 2 s, feed-forward alone drifts more than 1 arcmin in 10 minutes and the closed loop holds an RMS of at most 20 arcsec over the last 8; with every error under 10 arcsec the commanded rates equal feed-forward within 0.001 arcsec/s and the offset stays 0; 2 minutes on rendered, solved frames agree with truth plus noise within 5 arcsec RMS"
        decision: "spec.md:US4-S1"
      - path: crates/finder/tests/session_api.rs
        status: existing
        disposition: preserve
        assertion: "a fresh session's whole body is unchanged; US4 adds no session field"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "US3's crates/finder/tests/tracking_sim.rs passes unchanged with the correction in place"
      - "the gains are constants in code, not settings (plan.md Decisions)"
    checks:
      development:
        - "cargo test -p finder --locked --test tracking_loop"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: Bryan, through the merge queue; the coverage floor counts the simulator tests (spec.md SC-003)"
    writes:
      - crates/finder/src/tracking.rs
      - crates/finder/tests/tracking_loop.rs
    shared_writes:
      - path: crates/finder/src/tracking.rs
        other_spec: 015-tracking
        other_story: US5
        disposition: merge_order
        reason: "US5 depends_on_merged US4, and adds the backlash measurement and take-up"
    open_questions: []
  US5:
    tests:
      - path: crates/finder/tests/backlash_sim.rs
        status: new
        disposition: add
        assertion: "with 0.3 deg of backlash: the first transit stops the altitude axis before it reverses, is back within 30 arcsec within 15 s and measures 0.3 deg within 30 arcsec; a later transit and M81's elongation stop, take it up and stay within 30 arcsec; syncs taken tracking down after the transit open no epoch and fit IE within 30 arcsec of the truth; after the backlash is read, a fixed-terms reset leaves no backlash field"
        decision: "spec.md:US5-S1"
      - path: crates/finder/tests/backlash_cli.rs
        status: new
        disposition: add
        assertion: "the binary restarted the next day on the same --state folder reports the same altitude backlash exactly"
        decision: "spec.md:US5-S4"
      - path: crates/finder/tests/session_api.rs
        status: existing
        disposition: preserve
        assertion: "a fresh session's whole body is unchanged: backlash is left out until an axis is measured"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/session_cli.rs
        status: existing
        disposition: preserve
        assertion: "012's restart tests pass unchanged with the backlash in the state file"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "014's slip thresholds and the rule that every solve is a sync are unchanged; only syncs taken against the approach direction are corrected or held back (spec.md FR-011)"
      - "014's POST /api/model/reset keeps its body and answers (plan.md Shapes that existing tests pin)"
    checks:
      development:
        - "cargo test -p finder --locked --test backlash_sim --test backlash_cli"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: Bryan, through the merge queue; outside the factory, a transit tracked at the telescope (spec.md Assumptions)"
    writes:
      - crates/finder/src/tracking.rs
      - crates/finder/src/model.rs
      - crates/finder/src/session.rs
      - crates/finder/src/api.rs
      - crates/finder/tests/backlash_sim.rs
      - crates/finder/tests/backlash_cli.rs
    shared_writes:
      - path: crates/finder/src/tracking.rs
        other_spec: 015-tracking
        other_story: US6
        disposition: merge_order
        reason: "US6 depends_on_merged US5, and adds the keyhole"
      - path: crates/finder/src/session.rs
        other_spec: 015-tracking
        other_story: US7
        disposition: merge_order
        reason: "US7 depends_on_merged US5, and adds Track and the imaging camera"
      - path: crates/finder/src/api.rs
        other_spec: 015-tracking
        other_story: US7
        disposition: merge_order
        reason: "US7 depends_on_merged US5, and adds POST /api/track"
      - path: crates/finder/src/model.rs
        other_spec: 014-goto
        other_story: US5
        disposition: merge_order
        reason: "015 depends_on_landed 014, whose US5 creates the pointing model"
      - path: crates/finder/src/session.rs
        other_spec: 014-goto
        other_story: US5
        disposition: merge_order
        reason: "015 depends_on_landed 014, whose US5 last adds session state"
      - path: crates/finder/src/api.rs
        other_spec: 014-goto
        other_story: US7
        disposition: merge_order
        reason: "015 depends_on_landed 014, whose US7 last adds routes"
    open_questions: []
  US6:
    tests:
      - path: crates/finder/tests/keyhole_sim.rs
        status: new
        disposition: add
        assertion: "a target at declination 39 deg at latitude 40 pauses where it enters the 2 deg zenith circle, waits where it will leave after a slew through 014's planner, and is back within 1 arcmin within 30 s of leaving"
        decision: "spec.md:US6-S1"
      - path: crates/finder/tests/session_api.rs
        status: existing
        disposition: preserve
        assertion: "a fresh session's whole body is unchanged; US6 adds no session field"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "014's planner and its Sun guard are used as they are; crates/finder/src/goto.rs is not edited (spec.md Edge Cases, the keyhole)"
      - "US5's crates/finder/tests/backlash_sim.rs passes unchanged"
    checks:
      development:
        - "cargo test -p finder --locked --test keyhole_sim"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: Bryan, through the merge queue"
    writes:
      - crates/finder/src/tracking.rs
      - crates/finder/tests/keyhole_sim.rs
    shared_writes: []
    open_questions: []
  US7:
    tests:
      - path: crates/finder/tests/track_api.rs
        status: new
        disposition: add
        assertion: "through the router in paused time, with the simulated mount connected and an accepted Track moving an axis first: Track on during a GoTo starts within 2 s of its end; M8 below the horizon answers 409 and moves nothing; M42 5 deg off starts a GoTo, then tracks; Track off stops both axes within 0.5 s and the true direction moves under 1 arcmin; a pitch of 0 answers 400 and changes nothing; the readout's field rotation and longest exposure equal FR-001 and FR-002 within 1e-9; a foreign Origin answers 403 and moves nothing; a foreign bodiless POST /api/stop answers 200, rests both axes within 1.5 s for 10 s and ends tracking as stopped"
        decision: "spec.md:US7-S1"
      - path: crates/finder/tests/track_cli.rs
        status: new
        disposition: add
        assertion: "the imaging camera is kept exactly across a restart on one --state folder and null then clears it; tracking that moved an axis before a restart is off after it, with nothing moving"
        decision: "spec.md:US7-S5"
      - path: crates/finder/tests/session_api.rs
        status: existing
        disposition: preserve
        assertion: "a fresh session's whole body is unchanged: track and imaging_camera are left out until they have a value; 012's checked edits still refuse their bad values"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/session_writes.rs
        status: existing
        disposition: preserve
        assertion: "012's same-origin rule still refuses foreign writes on every route but /api/stop"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "POST /api/stop keeps no body and no same-origin check, and answers 200 as 014 made it (plan.md Shapes that existing tests pin)"
      - "tracking is never persisted (spec.md FR-013)"
      - "crates/finder/src/tracking.rs is not edited; the armed state is the session's (plan.md Decisions)"
    checks:
      development:
        - "cargo test -p finder --locked --test track_api --test track_cli"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: Bryan, through the merge queue"
    writes:
      - crates/finder/src/api.rs
      - crates/finder/src/session.rs
      - crates/finder/src/live.rs
      - crates/finder/README.md
      - crates/finder/tests/track_api.rs
      - crates/finder/tests/track_cli.rs
    shared_writes:
      - path: crates/finder/src/api.rs
        other_spec: 014-goto
        other_story: US7
        disposition: merge_order
        reason: "015 depends_on_landed 014, whose US7 last adds routes"
      - path: crates/finder/src/session.rs
        other_spec: 014-goto
        other_story: US5
        disposition: merge_order
        reason: "015 depends_on_landed 014, whose US5 last adds session state"
      - path: crates/finder/src/live.rs
        other_spec: 014-goto
        other_story: US5
        disposition: merge_order
        reason: "015 depends_on_landed 014, whose US5 hands solved frames to the model"
      - path: crates/finder/README.md
        other_spec: 014-goto
        other_story: US7
        disposition: merge_order
        reason: "015 depends_on_landed 014, whose US7 last documents routes"
    open_questions: []
  US8:
    tests:
      - path: crates/finder/web/src/components/TrackControl.test.tsx
        status: new
        disposition: add
        assertion: "a recorded too-low end shows the toggle off and says the target is too low; the readout rounds to 1 arcsec, 0.01 deg/min and 0.1 s, or reads no limit; no toggle is offered without a mount"
        decision: "spec.md:US8-S1"
      - path: crates/finder/web/src/components/ImagingCameraSection.test.tsx
        status: new
        disposition: add
        assertion: "23.5, 15.6 and 3.76 put the session with those values and the readout gains the longest exposure; a pitch of 0 saves nothing and states the allowed range"
        decision: "spec.md:US8-S4"
      - path: crates/finder/web/src/components/GuidanceBlock.test.tsx
        status: existing
        disposition: preserve
        assertion: "the guidance block's 012 and 013 cases, and 014's once landed, pass unchanged; Track renders nothing without a mount"
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
      - "no npm package is added (spec.md US8-S7)"
      - "every existing Vitest file is unchanged (tasks.md T041)"
    checks:
      development:
        - "cd crates/finder/web && npm test -- src/components/TrackControl.test.tsx src/components/ImagingCameraSection.test.tsx"
        - "cd crates/finder/web && CI=1 npm run e2e -- --project session"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: Bryan; the maintainer's Impeccable review of the Track toggle, readout and imaging camera section after US8 lands (spec.md Assumptions)"
    writes:
      - DESIGN.md
      - .impeccable/design.json
      - crates/finder/web/dist
      - crates/finder/web/src/api.ts
      - crates/finder/web/src/components/GuidanceBlock.tsx
      - crates/finder/web/src/components/SetupPage.tsx
      - crates/finder/web/src/components/TrackControl.tsx
      - crates/finder/web/src/components/TrackControl.test.tsx
      - crates/finder/web/src/components/ImagingCameraSection.tsx
      - crates/finder/web/src/components/ImagingCameraSection.test.tsx
      - crates/finder/web/src/test/fixtures
    shared_writes:
      - path: DESIGN.md
        other_spec: 014-goto
        other_story: US8
        disposition: merge_order
        reason: "015 depends_on_landed 014, whose US8 last records its parts"
      - path: .impeccable/design.json
        other_spec: 014-goto
        other_story: US8
        disposition: merge_order
        reason: "015 depends_on_landed 014, whose US8 last records its parts"
      - path: crates/finder/web/dist
        other_spec: 014-goto
        other_story: US8
        disposition: merge_order
        reason: "015 depends_on_landed 014, whose US8 last rebuilds the bundle"
      - path: crates/finder/web/src/api.ts
        other_spec: 014-goto
        other_story: US8
        disposition: merge_order
        reason: "015 depends_on_landed 014, whose US8 last writes api.ts"
      - path: crates/finder/web/src/components/GuidanceBlock.tsx
        other_spec: 014-goto
        other_story: US8
        disposition: merge_order
        reason: "015 depends_on_landed 014, whose US8 puts GoTo in the guidance block"
      - path: crates/finder/web/src/components/SetupPage.tsx
        other_spec: 014-goto
        other_story: US8
        disposition: merge_order
        reason: "015 depends_on_landed 014, whose US8 adds the mount and model section"
      - path: crates/finder/web/src/test/fixtures
        other_spec: 014-goto
        other_story: US8
        disposition: merge_order
        reason: "015 depends_on_landed 014, whose US8 adds recorded mount sessions"
    open_questions: []
  US9:
    tests:
      - path: crates/finder/tests/tracking_night.rs
        status: new
        disposition: add
        assertion: "an hour of paused time with a 1% rate error and 0.3 deg of backlash: within 30 arcsec through the transit's reversal, back within 1 arcmin within 30 s of leaving the keyhole, too_low within 1 s of the minimum altitude, and its own Instant::elapsed under 30 s"
        decision: "spec.md:US9-S1"
      - path: crates/finder/web/e2e/tracking.spec.ts
        status: new
        disposition: add
        assertion: "on finder --simulate with a 23.5 x 15.6 mm, 3.76 um imaging camera and Track on for M42: the error, field rotation and longest exposure equal the session's values as rounded; the night pixel check passes while tracking"
        decision: "spec.md:US9-S2"
      - path: crates/finder/web/e2e/console.spec.ts
        status: existing
        disposition: preserve
        assertion: "every console test passes unchanged in the default project, which now also ignores tracking.spec.ts"
        decision: "spec.md:US9-S4"
      - path: crates/finder/web/e2e/session.spec.ts
        status: existing
        disposition: preserve
        assertion: "every session test passes unchanged in the session project"
        decision: "spec.md:US9-S4"
    preserved_behavior:
      - "013's e2e/simulate.spec.ts and 014's e2e/goto.spec.ts pass unchanged in the simulate project (plan.md Shapes that existing tests pin)"
      - "playwright.config.ts changes only by owner approval 2 (spec.md US9-S4)"
    checks:
      development:
        - "cargo test -p finder --locked --test tracking_night"
        - "cd crates/finder/web && CI=1 npm run e2e"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: Bryan; 8097-8099 free on the factory host while it builds (plan.md Plan traps, trap 6); the telescope checklist outside the factory"
    writes:
      - crates/finder/tests/tracking_night.rs
      - crates/finder/web/e2e/tracking.spec.ts
      - crates/finder/web/playwright.config.ts
    shared_writes:
      - path: crates/finder/web/playwright.config.ts
        other_spec: 014-goto
        other_story: US9
        disposition: merge_order
        reason: "015 depends_on_landed 014, whose US9 adds goto.spec.ts to the simulate project"
    open_questions: []
```
