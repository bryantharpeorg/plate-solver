---
state: draft
depends_on_landed: ["013-live-finder", "014-goto", "024-mount-link-recovery", "027-finder-tests-by-system", "038-track-follows-a-push", "047-console-uses-the-database-that-fits", "051-tools-page-focuses-either-camera"]
# Drafted 2026-11-27 against `main` at f6d4f5fd, at the owner's ask of that day ("Your
# rec. Proceed", to the operator seat's ranked list of a field night's defects), the
# second of four specs (052 detection keeps saturated stars, 053 this one, 054 the imaging
# run's focus pause, 055 nudge buttons). From a probe of that day on the branch
# `probe/053` cut from f6d4f5fd (tip 5b40ab27; plan.md § What was run and what was
# read). Every `path:line` here was read at f6d4f5fd. State: draft. The six owner
# decisions are open, each with a recommendation the scenarios are written to; the flip
# is the owner's. 014 is named because this spec changes its pointing model, GoTo and
# second sync (FR-011, FR-012, FR-013, FR-019, FR-022, quoted in Clarifications); 038
# because its slip mark stays the one explanation taken at once; 013 for the live loop
# that takes the syncs; 024, 027, 038, 047 and 051 because each last changed a file this
# spec writes (plan.md § Refinement Record).
---

# Feature Specification: The Pointing Model Holds Far Solves, and the Second Sync Fits the Tilt

**Feature Branch**: `053-model-holds-far-solves-and-fits-tilt`

**Created**: 2026-11-27

**Status**: Draft

**Input**: On a field night, on the alt-az mount with the finder camera about 1.6° wide,
the second sync turned the telescope 113° in azimuth at about 79° up, and the next GoTo,
to M42, landed about 5° off (a first error of 17,960″) and ended `failed`, "no frame
solved after settling"; the model then read 4 syncs at an RMS of 784″, and later 5 syncs
at 2,282″. Earlier GoTos to the Pleiades and M52 had first errors of about 5.0° and 5.7°.
The operator seat's triage (outside the repository) found, by reading the code and by a
Python copy of the model: no base tilt was ever fitted, because a second sync more than
2.5° from its one-sync prediction counts as a slip and opens a new zero-point epoch
(`crates/finder/src/model.rs:508`, `crates/finder/src/model.rs:515`); a base tilt of
about 3° reproduces the misses; the second sync's spot has no altitude cap
(`crates/finder/src/goto.rs:280`), and one sync near the zenith puts several times the
tilt into the azimuth zero. It also found that nothing stands between a live solve and a
sync (`crates/finder/src/live.rs:265`, `crates/finder/src/live.rs:274`,
`crates/finder/src/session.rs:1507`): a far-off solve is never refused, only opens an
epoch, and a GoTo corrects from `syncs.last()` (`crates/finder/src/goto.rs:703`). The
owner asked for a sync to be refused when it is far from the model's prediction and for
a false match not to be offered as a sync. Make the model take only the solves it can
vouch for, hold the rest until a second solve agrees, have the second sync measure the
base tilt from a better place, and have a GoTo correct only from a solve the model took.

## Clarifications

### Session 2026-11-27 (the probe on `main` at f6d4f5fd)

- Q: What does this trio read besides itself? → A: Nothing about the feature. `spec.md`
  holds every requirement and scenario, and `plan.md` every design decision, the
  interfaces, the shapes that existing tests pin and the alternatives rejected. The
  probe's branch, logs and the field night's triage are not in the repository; what they
  showed is written here.
- Q: How does a live solve become a sync today? → A: The live loop records the solve as
  the latest pointing and hands it to `Session::record_frame_sync`
  (`crates/finder/src/live.rs:265`, `crates/finder/src/live.rs:274`), which refuses it
  only while the telescope is moving by hand under Track (038,
  `crates/finder/src/session.rs:1516`), without a site or counts, or when
  `mid_exposure` finds an axis moving over 0.2°/s (`crates/finder/src/model.rs:258`);
  `Session::record_sync` calls `Model::add_sync` (`crates/finder/src/session.rs:1475`). `add_sync` refuses only invalid data and a
  backlash crossing (`crates/finder/src/model.rs:493`, `crates/finder/src/model.rs:496`).
  A sync more than 0.5° from the prediction, or 2.5° before the base tilt is fitted, is
  a slip (`crates/finder/src/model.rs:508`), which opens a new epoch fitted from that sync
  alone (`crates/finder/src/model.rs:515`, `crates/finder/src/model.rs:566`). The base
  tilt is fitted only from two syncs of one epoch at least 90° apart in solved azimuth
  (`crates/finder/src/model.rs:531`). Nothing checks the solved field size, the matched
  count or the database; the RMS only weights the fit (`crates/finder/src/live.rs:268`).
- Q: What does this change of 014, 024, 032, 038 and 050? → A: It changes five of 014's
  requirements and keeps the others:
  - 014 FR-011, "The console SHALL take as a sync each solved star frame from the scope
    node that carries axis positions, when both axes moved at no more than 0.2°/s during
    its exposure.": now each such frame the model vouches for (FR-001, FR-002).
  - 014 FR-012, "the base tilt (AN, AW) once it holds two syncs at least 90° apart in
    azimuth": now 90° on the mount's azimuth axis, within one epoch, and up to 6° (FR-001,
    FR-003).
  - 014 FR-013, "Once the base tilt is fitted, a sync more than 0.5° from the model's
    prediction SHALL open a new zero-point epoch. … Before then, a sync SHALL open one
    only when it is more than 2.5° from the prediction (0.5° plus twice a 1° base
    tilt).": the bounds stay, but such a sync is held until a second solve repeats it,
    and that second solve opens the epoch, keeping the base tilt and the fixed terms
    (FR-002); before the base tilt is fitted, a sync 90° away on the azimuth axis fits
    the tilt instead (FR-001).
  - 014 FR-019, "It SHALL then solve at the aim point and correct, at most 5 times, until
    the target is within 1′ of the aim point.": the solve it corrects from is one the
    model took as a sync (FR-006).
  - 014 FR-022, "the console SHALL slew to a pointable spot at least 90° in azimuth from
    the night's syncs and outside the Sun guard, solve there, and take the sync": also 90°
    on the azimuth axis from the current epoch's syncs, between 30° and 60° up when one
    has a safe path (FR-009).
  - 038 FR-007, "The next sync recorded SHALL open a new zero-point epoch whatever its
    distance from the model's prediction": kept as it is; the slip mark is the one
    explanation taken at once (FR-001).
  - 024 FR-014, the refusals "until the console records a sync from a frame whose
    exposure started after it saw the rise": kept; after a reset the counts change, so
    the first solve is held and the second, which repeats it, is the sync that ends the
    refusal, one frame later than today (Edge Cases).
  - 032's owner decision 3 rested on 014 FR-013 ("a sync more than 0.5° from the model's
    prediction … already opens a new zero-point epoch") for a moved aim point: it still
    does, one frame later.
  - 050 says nothing about syncs but that its rig waits until "the session's pointing
    model holds a sync" (`specs/050-tools-page-aligns-finder-camera/spec.md:404`):
    unchanged.
- Q: Did the probe reproduce the field night? → A: Yes, on the factory's rig: the
  simulator's mount carries an exact base tilt and zero points
  (`crates/simulator/src/mount.rs:18`, `crates/simulator/src/mount.rs:216`). With the
  first sync at 75° up and azimuth 122°, on `main` the second sync opened a second epoch
  and fitted no tilt for every tilt of 2° to 5° tried (toward 195° and 300°), and the
  next GoTos' first errors were 7,533″ to 45,412″ (a 3° tilt toward 195°: 17,196″ to a
  place 36° up at azimuth 127°, against the field night's 17,960″ to M42); at 0° and 1°
  the tilt was fitted and the first errors were 51″ to 62″ (run). The trial build of
  this spec fitted the tilt in one epoch, within 1′, at every tilt from 0° to 5° in both
  directions, and every first error was under 63″ (run; plan.md § What was run).
- Q: Why measure 90° on the azimuth axis? → A: The one-sync model's azimuth zero is off
  by several times the tilt when the sync is high, so a spot 90° away on the sky as that
  model predicts it can land under 90° away in solved azimuth, and then the second sync
  fits no tilt. In the probe's first trial, which kept today's rule on the sky with its
  allowance, a 5° tilt toward 45° or 90° from a first sync 75° up put the two solved
  azimuths 87.6° and 89.3° apart, and each second sync opened an epoch (run). The mount's
  own azimuth axis turns exactly as commanded whatever the model's error, and what fixes
  the base tilt is how far apart the two directions are in the mount's own frame. With
  the axis rule, 56 of 56 trial cases fitted the tilt in one epoch: 1°, 3° and 5° of
  tilt, eight directions each, from 45°, 75° and 82° up (1° from 75° only), the solved
  azimuths as little as 51.7° apart (run); at 8°, 8 of 8 fitted none.
- Q: Why between 30° and 60° up? → A: The simulator's mount has no collimation or
  axis-perpendicularity error, so on it two syncs 90° apart fit the tilt within 1′
  wherever the second one is (run: second syncs from 38° to 67° up in the axis trial). On a Python model of the two-sync fit written for the probe (outside
  the repository), with 0.25° of either unmodelled and a 3° tilt, a second sync at 79° up
  after a first at 75° leaves a worst error, over targets 20° to 80° up, of 0.85°
  (collimation) or 0.98° (non-perpendicularity), and one at 60°, 45° or 30° about half
  that, 0.44° to 0.58°; a first sync at 45° as well halves it again, 0.19° and 0.20°
  (run; plan.md § What was run). SynScan's own alignment advice is 15° to 60° up and 45°
  to 135° apart (`docs/research/2026-10-21-pointing-model-and-tracking.md:101`).
- Q: Does a false solve happen? → A: Rarely. A match passes a binomial false-alarm test of
  1e-5 over the database's pattern count (`crates/finder/src/solve.rs:26`); the triage's
  600 random subsets and orders of the field night's 69 centroids gave no match in either
  database (outside the repository). The field night's far reading, logged that night as a
  false solve, was most likely a true solve through a model 5° wrong: the correction
  slew that followed it left M42 0.3° from the centre (the triage). The rule here
  covers both: a solve the model cannot vouch for waits for a second, and a slip repeats in the next solve while a bad solve
  does not.

## Owner decisions

All six are open. Each has its options, its evidence and a recommendation, (a); the
scenarios and the record are written to the recommendations. One general rule is behind
decisions 1 to 4: **the model takes at once only what it can vouch for, and anything
else waits for the next solve to agree with it.** It vouches for a solve within its own
slip bound of its prediction (014's 0.5° and 2.5°, unchanged), for the second sync when
the tilt it fits is plausible, and for the first solve after a push by hand, which 038
already marks. So there is no second bound for intake, no rule for a mount reset, no
field-size or matched-count gate, no time window and no new setting: a slip repeats in the
next solve, and a bad solve does not.

1. **The agreement bound, and how a held solve is confirmed** (FR-001, FR-002). (a) **The
   model's own slip bound, 0.5° once the base tilt is fitted and 2.5° before; the held
   solve is confirmed by the next solve that the model, with the held one taken as a
   slip, would vouch for, and that second solve opens the epoch.** Evidence: no new
   number; on the rig a false solve 5° off is held and a true one drops it (US1-S3), two
   far solves that disagree are both refused (US1-S5), a 3° slip is taken one frame later
   (US1-S4), and every existing test passes unchanged but one line of one unit test,
   whose 0.6° sync now needs a second solve (owner approval 1; run: nextest 1,543 of
   1,543 on the probe's tip, plan.md § What was run).
   (b) **A gate of its own at 1°**, between 0.5° and 1° an epoch at once as today, a
   second solve within 5′ to confirm: no unit test changes, but two bounds and two kinds
   of slip. (c) **No gate**, as today.
2. **How a real slip is told from a bad solve** (FR-002). (a) **By one rule, persistence:
   the next solve repeats a slip and does not repeat a bad solve. A push by hand keeps
   038's mark and is taken at once; a mount reset needs nothing of its own**, as its next
   two solves agree (run: `crates/finder/tests/motion/mount_identity.rs` passed
   unchanged). (b) **Also mark a slip on a mount reset**, taken at once: one frame sooner,
   a change to 024's path. (c) **A time window** for the second solve: a number the rule
   does not need.
3. **The second sync's bound, its altitudes, and a blocked spot** (FR-001, FR-009). (a)
   **Before the base tilt is fitted, a sync at least 90° from one of the epoch's on the
   azimuth axis is vouched for when the tilt it fits is at most 6°, and joins the epoch;
   the spot is the nearest between 30° and 60° up as the model predicts it, from the
   current altitude held to that band, 90° on the azimuth axis from every sync of the
   epoch as well as today's rule on the sky; only when no spot in the band has a safe
   path does it take the nearest outside it. A spot blocked by a tree fails as today, "no
   frame solved after settling"; a GoTo to any clear target at least 90° round on the
   azimuth axis then fits the tilt by the same rule.** Evidence:
   6° is above every tilt tried (0° to 5°, fitted within 1′) and a tilt of 8° is held
   and taken as a slip (US1-S2, US2-S3); the band halves the error left by unmodelled
   terms (Clarifications). (b) **5°**: the top of the range tried, too close to it for a
   fit that lands at 5.00° (run: 4.99° to 5.00°). (c) **15° to 60°**, SynScan's own: more
   trees and more refraction low down.
4. **The field-size and matched-count bounds** (none). (a) **None**: the rule of decision
   1 holds a false solve far off, and one near the prediction does no harm; the solver
   already holds a solve to 5 % of the estimated field (`crates/finder/src/live.rs:252`)
   and to its false-alarm test (`crates/finder/src/solve.rs:26`). (b) **The field within
   5 % of the database in use and at least 8 matched stars**: two numbers and a
   difference from 047's choice of database, for a case decision 1 covers.
5. **What the page and the API show of a held solve, and of a failed GoTo** (FR-005,
   FR-007, FR-008). (a) **`model.candidate: {off_deg, time}` in `GET /api/session`, and
   so in MCP's `get_session` (`crates/finder/src/mcp/tools.rs:538`,
   `crates/finder/src/api.rs:884`); `goto.frame_id`, the frame whose sync gave the
   error; and a failure that names the held frame and its distance. No page change**: the
   page already shows a GoTo's reason (`crates/finder/web/src/components/GoToControl.tsx:19`)
   and "Pointing stale" for a live pointing over 5 s old
   (`crates/finder/src/pushto.rs:184`, `crates/finder/web/src/components/ChartHeader.tsx:73`).
   (b) **Also a line in Setup's mount and model section** for the held solve: a page
   story, its build and its browser test.
6. **A warning while the model rests on one sync above about 70°** (none). (a) **None**:
   the second sync's band halves what such a first sync leaves, the GoTo's corrections
   take out the rest locally, and the README says to take the first sync between 30° and
   60° up when the sky allows. (b) **`model.warning` and a Setup line**.

## Owner approvals

Flipping this spec to `ready` approves the answer to every owner decision above, and the
following. Each is named in the FR that makes it. No golden, fixture or CI file changes.

1. **One line of one existing test replaced** (FR-002): in `crates/finder/src/model.rs`'s
   own tests, `slips_refit_only_epoch_zero_points_at_the_half_degree_threshold`
   (`crates/finder/src/model.rs:898`) adds its sync without reading the answer
   (`crates/finder/src/model.rs:911`). That line becomes an assertion that `add_sync`
   answers true for the 0.4° sync and false for the 0.6° one, followed, for the 0.6°
   one, by an assertion that the epochs are still one and by the same sync 1 s later,
   asserted taken. Every assertion after it (the epoch count, the base tilt and fixed
   terms unchanged, the prediction at the sync within 1″ in azimuth and altitude, the
   altitude zero moved by more than 0.5°) stays as it is.
2. **New public API in `crates/finder`'s library** (FR-001 to FR-010):
   `finder::model::{MAX_BASE_TILT_DEG, SLIP_DEG, CandidateView}`, `Model::candidate`,
   `ModelView::candidate`, `Sync::axis_az_deg`, `finder::goto::SECOND_SYNC_BAND` and
   `Progress::frame_id`. No existing signature changes.
3. **Two new optional keys in `GET /api/session`** (FR-005, FR-008): `model.candidate`
   and `goto.frame_id`, each absent when there is none.
4. **Five of 014's requirements changed** as Clarifications quotes them: FR-011, FR-012,
   FR-013, FR-019 and FR-022.
5. **The glossary** (FR-013): **Base tilt** and **Slip** reworded, **Candidate** added.

## User Scenarios & Testing *(mandatory)*

Every scenario shares these definitions.

- **The gate command.** Every scenario runs in
  `cargo nextest run --workspace --all-targets --all-features --locked`, the `test`
  gate's first command, each test in its own process.
- **The mount's geometry**, the oracle of every Rust scenario, written fresh in each test
  file from `simulator::mount`'s definition of the true direction
  (`crates/simulator/src/mount.rs:216`): axis directions (azimuth a, altitude e, in
  degrees) plus zero points, then rotated by the base tilt t about the horizontal axis
  (−sin w, cos w, 0), w the azimuth it leans toward, with Rodrigues' formula. No
  expected value comes from the code under test: no test calls `finder::model`'s
  `separation`, `predict` or the fit for an expected value, or reads `SLIP_DEG`,
  `MAX_BASE_TILT_DEG` or `SECOND_SYNC_BAND`; each test writes its own great-circle
  distance by the chord.
- **The model rig** is `finder::model::Model::default()` fed `finder::model::Sync`s that
  the test builds: **a sync at (a, e) at time T** has the axis positions a and e at
  1,620,000 counts a revolution, the azimuth and altitude the mount's geometry gives them
  under zero points (17°, −2°) unless a scenario says otherwise, RMS 1″, no roll and epoch
  0; **with an error (Δa, Δe)** its azimuth and altitude are moved by that many degrees.
  **The fitted model** is the model rig after syncs at (30°, 40°) at 0 s and (140°, 50°)
  at 200 s under a tilt of 1° toward 130°.
- **The session rig** is a `Session::with_state_file` in a `crate::support::Scratch`
  folder with a site set, served by `finder::router` over
  `AppState::shared(crate::support::live::DB, Samples::default())` with `with_session`
  and `with_clock(FixedClock(450.0))`, as `crates/finder/tests/motion/model/routes.rs`
  builds its own.
- **The GoTo rig** is `crate::support::goto::Rig`'s fields built in the test file over a
  simulator on `crate::support::live::DB` at `crate::support::live::START`, a `SimMount`
  of the scenario's `MountConfig` (backlash 0.3°) connected to a `NodeService` with
  `crate::support::goto::Recorded`, a 500 ms frame interval, the session `Rig::new`
  edits (site, target M42, 200 ms at gain 1, the aim point (700, 420)) and the live solver
  **Swap**: it solves each frame on the paused runtime as `crate::support::goto::Stars`
  does, except that it keeps the star field of the next frame after the test asks, and
  answers each frame whose id lies in a window the test sets with the kept fields in
  turn, a false solve of another place's real stars. **The tilt rig at (alt, az)** is the
  GoTo rig over a tilt t toward w with zero points that put counts (4500, 4500), reached
  from rest at zero, at the true (alt, az): the test's own inverse rotation, the tube
  0.85° after half the backlash, as `crates/finder/tests/motion/goto/second_sync.rs`'s
  `initial_zero` computes it; moved to (4500, 4500), the night's model reset and the
  first sync awaited with `Rig::sync`. **The level rig** is the tilt rig with t = 0.
  **A place (alt, az)** is the catalog's M42 moved to that true horizontal place at the
  rig's time, as `crates/finder/tests/motion/goto/sim.rs`'s `horizontal_target` makes it.
  **The two places** are (36°, 127°) and (40°, 250°). The tests run under
  `#[tokio::test(start_paused = true)]`, and every GoTo and second sync is read with
  `Rig::finish_with(Some(15.0), true, …)`, which asserts at every read that the true
  altitude is at least 15° and the true direction at least 15° from the Sun, and that it
  ends within 5,000 reads.
- **The docs rig** is the README and the glossary read as text, every run of whitespace
  joined into one space, as `crates/finder/tests/console/imaging_run_docs.rs` reads the
  README.

### User Story 1 - The pointing model takes only the solves it can vouch for (Priority: P1)

As the owner, I need the pointing model to take a solve at once only when it lies where
the model expects it, or where only the base tilt the second sync is there to measure
can explain it, and to hold any other solve until the next one agrees, so that one bad
solve cannot move the model and a real slip still opens a new zero-point epoch.

**Why this priority**: The field night's misses come from the second sync taken as a
slip, and nothing stood between a solve and the model.

**Independent Test**: `crates/finder/tests/motion/model/intake.rs`, new, and the replaced
line in `crates/finder/src/model.rs`'s own tests. The `test` gate's nextest command
runs them; during development,
`cargo nextest run -p finder --locked -E 'test(/^model::intake::/) | test(slips_refit_only)'`.

**Acceptance Scenarios**:

1. **Given** the model rig with a sync at (105°, 77°) at 0 s and a sync at (0°, 45°) at
   120 s, under tilts of 0°, 1°, 3° and 5° toward 195°, **When** a committed test adds
   the two in turn for each tilt, **Then** it asserts that both `add_sync` calls answer
   true, that the model holds one epoch and a base tilt whose `deg()` is within 1′ of the
   tilt and, above 0°, whose `toward_deg()` is within 1′ of 195°, that `predict` at the
   axis direction (250°, 30°) lies within 1″ of the mount's geometry, and that
   `view().candidate` is `None`.
2. **Given** the model rig under a tilt of 8° toward 195° with the two syncs of US1-S1,
   **When** a committed test adds them and then the second again at 121 s, **Then** it
   asserts that the first answers true and the second false, that `view().candidate`
   then has `off_deg` above 2.5 and `time` 120, with one sync and one epoch, that the
   third answers true, and that the model then holds two epochs, no base tilt and no
   candidate.
3. **Given** the fitted model and a sync at (200°, 45°) at 400 s with an error (5°, 0°),
   **When** a committed test adds it and then a sync at (200°, 45°) at 401 s, **Then** it
   asserts that the first answers false and leaves the syncs and epochs equal to the
   fitted model's, that `view().candidate.off_deg` is within 1″ of the chord distance
   between the geometry's direction at (200°, 45°) and that direction 5° further in
   azimuth, that the second answers true, and that the model then holds one epoch, three
   syncs and no candidate.
4. **Given** the fitted model and syncs at (200°, 45°) at 400 s and at (201°, 46°) at
   410 s under zero points (20°, −2°), a slip of 3° in azimuth, **When** a committed test
   adds them in turn, **Then** it asserts that the first answers false and the second
   true, that the model holds two epochs and three syncs, the newest at 410 s, and that
   `predict` at the axis direction (230°, 35°) lies within 1″ of the slipped geometry.
5. **Given** the fitted model, a sync at (200°, 45°) at 400 s with an error (5°, 0°) and
   one at 401 s with an error (−7°, 3°), **When** a committed test adds them in turn,
   **Then** it asserts that both answer false, that the model holds two syncs and one
   epoch, and that `view().candidate.time` is 401.
6. **Given** the fitted model with `mark_slip()` called, a sync at (200°, 45°) at 400 s
   under zero points (20°, −2°), and then a sync at (210°, 45°) at 500 s under (17°, −2°),
   **When** a committed test adds the two and then calls `reset_night()`, **Then** it
   asserts that the first answers true with two epochs, that the second answers false
   with a candidate in `view()`, and that after the reset `view()` is `None` and
   `candidate` is `None`.
7. **Given** the session rig with the fitted model's two syncs recorded through
   `Session::record_sync`, **When** a committed test records the sync of US1-S3 with its
   error, reads `GET /api/session`, records the sync at (200°, 45°) at 401 s and reads it
   again, **Then** it asserts that the first record answers `Ok(false)` and leaves the
   state file's bytes as they were, that the first read has `model.syncs` 2,
   `model.candidate.time` 400 and `model.candidate.off_deg` within 1″ of US1-S3's
   distance, that the second record answers `Ok(true)`, and that the second read has
   `model.syncs` 3 and no `candidate` key.
8. **Given** `crates/finder/src/model.rs`'s own test
   `slips_refit_only_epoch_zero_points_at_the_half_degree_threshold`, its 24 spread syncs
   with fixed terms fitted and its sync at (180°, 50°) at 5000 s moved 0.4° and 0.6° in
   altitude, **When** that test adds each, and the 0.6° one again at 5001 s, **Then** it
   asserts that the 0.4° sync is taken into the one epoch, that the 0.6° sync is refused
   with one epoch, that its repeat is taken and opens a second epoch, that the base tilt
   and fixed terms are unchanged, that `predict` at the sync gives its azimuth and altitude
   each within 1″, and that the altitude zero moved by more than 0.5°.

### User Story 2 - GoTo and the second sync use only taken solves, and the second sync measures the tilt (Priority: P1)

As the owner, I need a GoTo to correct, and the second sync to finish, only from a solve
the model took, a failed GoTo to name the solve it waited on, and the second sync to go
where it can measure the base tilt: between 30° and 60° up and a quarter turn round on
the mount's own azimuth axis.

**Why this priority**: The model's rule is only as good as what GoTo corrects from, and
the second sync is how the tilt gets measured at all.

**Independent Test**: `crates/finder/tests/motion/goto/intake.rs`, new. The `test` gate's
nextest command runs it; during development,
`cargo nextest run -p finder --locked --test motion -E 'test(/^goto::intake::/)'`.

**Acceptance Scenarios**:

1. **Given** the tilt rig at (75°, 122°) for tilts of 0°, 1°, 2°, 3°, 4° and 5°, each
   toward 195° and toward 300°, **When** a committed test sends
   `POST /api/model/second-sync` and then starts a GoTo to each of the two places in
   turn, **Then** it asserts that the second sync ends `done` with every progress read at
   correction 0, that the model then holds two syncs, one epoch and a base tilt whose
   `deg()` is within 1′ of the tilt, that each GoTo ends `done` with its first
   `error_arcsec` under 120 and a `frame_id` above the latest frame's id when it was
   started, and that the model still holds one epoch after both.
2. **Given** the level rig at (75°, 122°), at (45°, 122°) and at (20°, 122°), **When** a
   committed test sends `POST /api/model/second-sync` on each, **Then** it asserts that
   each ends `done`, that the mount's true altitude is then within 0.2° of 60°, 45° and
   30° respectively, that the azimuth axis turned at least 90° and the true azimuth
   changed by at least 90°, and that the model holds one epoch.
3. **Given** the tilt rig at (75°, 122°) for a tilt of 8° toward 45°, **When** a
   committed test sends `POST /api/model/second-sync`, reading the session's
   `model.candidate` at every progress read, **Then** it asserts that the second sync
   ends `done`, that a candidate was read while it ran, and that the model then holds two
   epochs and no base tilt.
4. **Given** the GoTo rig over a tilt of 1° toward 130° with zero points (120°, 50°),
   moved to (4500, 4500) and then to (580000, 4500), the star field of a frame kept at
   each, so that the model holds one epoch and a base tilt, **When** a committed test
   starts a GoTo to M42 and, at the first progress read `solving`, sets Swap's window to
   the four frames after the latest, **Then** it asserts that the GoTo ends `done`, that
   its `frame_id` is at least the window's end, that its `error_arcsec` and the
   simulator's true error from M42 (`Rig::error_arcsec`) are each under 60, that a
   candidate was read while it ran, and that the model holds the same number of epochs as
   before.
5. **Given** the GoTo rig of US2-S4 with its two kept fields, **When** a committed test
   starts a GoTo to M42 and, at the first progress read `solving`, sets Swap's window
   from the frame after the latest without an end, **Then** it asserts that the GoTo ends
   `failed`, its `reason` starting `frame ` and ending
   `from the pointing model and no second solve agreed`, with no `error_arcsec`, and
   that the model holds the same number of epochs as before.

### User Story 3 - The README and the glossary say which solves the model takes (Priority: P2)

As the owner, I need the README and the glossary to say what the model takes, what it
holds, where the second sync goes and what a failed GoTo says, in the words the console
uses.

**Why this priority**: The night's use of the second sync and of a held solve depends on
knowing them; the code is already right without them.

**Independent Test**: `crates/finder/tests/console/model_docs.rs`, new. The `test` gate's
nextest command runs it; during development,
`cargo nextest run -p finder --locked --test console -E 'test(model_docs)'`.

**Acceptance Scenarios**:

1. **Given** the docs rig, **When** a committed test reads `crates/finder/README.md` and
   `GLOSSARY.md`, **Then** it asserts that the README holds each of FR-012's eight
   sentences and the glossary each of FR-013's three, word for word.

### Edge Cases

- **A mount reset** (the node's `mount_resets` rises, `crates/finder/src/session.rs:1445`):
  the counts change, so the first solve after it is held and the second, which repeats
  it, is taken as a slip and ends 024's refusal; one frame later than today. Run: every
  test of `crates/finder/tests/motion/mount_identity.rs` passed unchanged.
- **A push by hand with Track on**: 038 marks the slip (`crates/finder/src/session.rs:1302`)
  and the first solve after it is taken at once (US1-S6).
- **A push by hand with Track off**: the solves during the push disagree with one another
  and none is taken; once the telescope is still, two agree and the second opens the
  epoch. Run: `hand_push_opens_an_epoch_and_goto_still_converges`
  (`crates/finder/tests/motion/goto/sim.rs:159`) passed unchanged.
- **A moved aim point** (a tap, 032): a step within the bound is taken; a larger one is
  held, and the next solve repeats it and opens the epoch. Read, not run.
- **The same false solve twice in a row** would be taken: two frames' stars differ, and
  the false-alarm test makes even one false match rare (Clarifications). Read, not run;
  US2-S5 runs two false fields in turn, which never agree.
- **A tree at the second-sync spot**: no frame solves and it fails as today. A GoTo to
  any clear target at least 90° round on the azimuth axis from the first sync then
  serves: its first solve fits the tilt by FR-001's rule (US1-S1 holds the rule on the
  model rig; the GoTo route was not run for this). A push by hand does not, since it
  is a slip (038).
- **A GoTo before the second sync**: a first solve more than 2.5° off and under 90°
  round on the azimuth axis is held, and the next, at the same place, opens an epoch, as
  today but one frame later. Read, not run.
- **A base more than 6° off level**: the second sync is held and taken as a slip
  (US1-S2, US2-S3); the model has no base tilt, and the GoTos correct as before.
- **Track running**: the tracker takes live pointings, not syncs
  (`crates/finder/src/session.rs:1178`), and is unaffected. Run: every tracking and
  steering test passed unchanged.
- **The imaging run**: its GoTo and centring are this GoTo. Run: every `imaging_run*`
  test passed unchanged.
- **A console restart with a solve held**: the candidate is run-only and is lost; the
  next two solves agree or not as before. Read, not run.

## Requirements *(mandatory)*

### Functional Requirements

**The pointing model** (US1):

- **FR-001**: `Model::add_sync` SHALL take a qualified sync at once only when the model
  vouches for it: the night's first sync; a sync within `SLIP_DEG` of the model's
  prediction, 0.5° once the base tilt is fitted and 2.5° before; before the base tilt is
  fitted, a sync at least 90° on the azimuth axis from a sync of the current zero-point
  epoch whose taking would fit a base tilt of at most `MAX_BASE_TILT_DEG`, 6°, which
  joins that epoch and fits the tilt and never opens an epoch; and a sync while the
  model's slip mark is set (038's FR-007), which opens a new epoch. **Refs**: 014's
  FR-012 and FR-013; 038's FR-007; owner decisions 1 and 3.
- **FR-002**: A qualified sync the model does not vouch for SHALL be held as the model's
  `candidate` and refused: `add_sync` answers false and the syncs, epochs, base tilt,
  fixed terms, RMS and slip mark are unchanged. A later sync the model vouches for SHALL
  drop the candidate. A later sync it does not vouch for SHALL be taken as a slip,
  opening a new epoch fitted as 014's FR-013 fits one, with the base tilt and fixed terms
  kept, and the candidate dropped, when the model with the candidate taken as a slip
  would vouch for it; otherwise it SHALL replace the candidate. **Refs**: owner decisions
  1 and 2; owner approval 1.
- **FR-003**: Two syncs SHALL count as 90° apart for the base tilt when their axis
  positions' azimuths, `Sync::axis_az_deg()` (the azimuth count times 360 over the counts
  a revolution), differ by at least 90°, within one epoch. **Refs**: 014's FR-012;
  Clarifications (why the azimuth axis).
- **FR-004**: The candidate SHALL be run-only: never written to the state file, and
  dropped by `reset_night`, so by expiry and by `POST /api/model/reset` with
  `{"what":"night"}`. `Session::record_sync` SHALL keep a candidate the model holds when it refuses a sync,
  without writing the state file. **Refs**: 014's FR-015.
- **FR-005**: The session's `model` view SHALL carry `candidate: {off_deg, time}` while
  one is held: its great-circle distance in degrees from the current model's prediction
  at its axis positions, and its mid-exposure time; absent otherwise. **Refs**: owner
  decision 5; owner approval 3.

**GoTo and the second sync** (US2):

- **FR-006**: The second sync's solve and each GoTo's solve SHALL be one the model took:
  of the reports for frames that started after the settle, the loop SHALL pass over one
  whose time no sync of the session's model holds, and wait on within the same 30 s; the
  error it reports and the correction it plans come from the first it took, as 014's
  FR-019 corrects. **Refs**: 014's FR-019 and FR-022.
- **FR-007**: The `goto` progress SHALL carry `frame_id`, the id of the frame whose sync
  gave `error_arcsec`, or the second sync's frame, from the moment it is taken until the
  next GoTo or second sync starts; absent before. **Refs**: owner decision 5; owner
  approval 3.
- **FR-008**: When the 30 s pass after a frame's solve was held as the model's candidate
  and no later one was taken, the GoTo or second sync SHALL end `failed` with
  `frame <id> solved <x.x>° from the pointing model and no second solve agreed`, the
  candidate's `off_deg` to one decimal place; with no solve held, `no frame solved after
  settling` as today. **Refs**: owner decision 5.
- **FR-009**: The second sync's spot SHALL be at least 90° on the mount's azimuth axis,
  as the model's inverse places it, from every sync of the current epoch, as well as
  today's 90° on the sky with its allowance (`crates/finder/src/goto.rs:275`). It SHALL be
  the nearest in turn and tilt (the hypotenuse of the azimuth and altitude differences)
  among the spots whose altitude, as the model predicts it, lies within
  `SECOND_SYNC_BAND`, 30° to 60°, searched from the current altitude held to that band; and only when none of those has a safe path, the nearest outside it, as
  today. **Refs**: 014's FR-022; owner decision 3.
- **FR-010**: Nothing else in GoTo or the second sync SHALL change: the settle of 0.5 s,
  the 30 s wait, five corrections to 1′, the states and the refusals. **Refs**: 014's
  FR-017 to FR-023; 024's FR-014 and FR-015.

**The words** (US3):

- **FR-011**: `crates/finder/README.md`'s GoTo and second-sync paragraphs SHALL say what
  FR-001 to FR-009 do, in the words of plan.md § Interfaces.
- **FR-012**: The README SHALL hold these eight sentences, each word for word with its
  line breaks free:
  1. "then for a solved exposure starting after that wait that the pointing model takes
     as a sync, and corrects from that sync alone."
  2. "`GET /api/session` adds `goto: {state, correction, error_arcsec, reason, frame_id}`
     only after one starts, retaining its result until the next GoTo; `frame_id` is the
     star frame whose sync gave `error_arcsec`."
  3. "When 30 s pass after settling with a solve held as the model's candidate and none
     taken, the GoTo or second sync ends `failed` with `frame <id> solved <x.x>° from the
     pointing model and no second solve agreed`; with no solve at all, `no frame solved
     after settling`."
  4. "and at least 90° on the mount's azimuth axis from every sync of the current
     zero-point epoch. It looks first between 30° and 60° up, as the model predicts the
     altitude, starting from the current altitude held to that band, and only when no
     spot there has a safe path does it take the nearest outside it."
  5. "The pointing model takes a live solve as a sync only when it can vouch for it:"
  6. "Any other solve is held as the model's candidate, shown in `GET /api/session` as
     `model.candidate: {off_deg, time}`, its distance from the prediction in degrees and
     its mid-exposure time."
  7. "a slip repeats in the next solve, and a bad solve does not."
  8. "The candidate is never written to the state file."
- **FR-013**: `GLOSSARY.md` SHALL hold these three sentences, each word for word with
  its line breaks free, **Base tilt** and **Slip** reworded and **Candidate** added after
  **Slip** (owner approval 5):
  1. "Fitted once two syncs of one zero-point epoch lie at least 90° apart on the
     azimuth axis, up to 6°."
  2. "**Candidate**: A live solve the pointing model cannot vouch for, held back from it
     until a second solve agrees with it, which is then taken as a slip; a solve the model
     vouches for drops it."
  3. "A solve that a second solve repeats beyond the model's prediction, or the first
     after a push by hand, opens a new zero-point epoch."

### Key Entities

- **Candidate**: a live solve the pointing model cannot vouch for, held in memory until
  the next solve agrees with it (then taken as a slip) or the model vouches for a solve
  (then dropped); one at a time.
- **The model's vouching**: within the slip bound of the prediction; or, before the base
  tilt is fitted, 90° away on the azimuth axis with a tilt of at most 6°; or after a
  marked slip.
- **The second sync's spot**: 90° on the azimuth axis and on the sky from the epoch's
  syncs, 30° to 60° up as the model predicts it when one is reachable.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario has a committed test that asserts it, and the
  `lint` and `test` gates pass.
- **SC-002**: No golden, fixture, `scripts/ci/` file, `.github/` file, `CODEOWNERS`
  entry or `ergane.yaml` is edited. Every existing test passes unchanged but the one
  assertion of owner approval 1 (plan.md § Shapes that existing tests pin). The only
  other edits to existing test files are the `mod` lines of
  `crates/finder/tests/motion/model/mod.rs`, `crates/finder/tests/motion/goto/mod.rs`
  and `crates/finder/tests/console/main.rs`.
- **SC-003**: On a tilted base of 0° to 5°, the second sync fits the tilt within 1′ in
  one epoch and the next GoTos' first errors are under 120″ (US2-S1), where `main` left
  first errors of 7,533″ to 45,412″ for 2° to 5° (plan.md § What was run).
- **SC-004**: A false solve neither moves the model nor steers a GoTo (US1-S3, US1-S5,
  US2-S4, US2-S5), and a real slip still opens an epoch one solve later (US1-S4).

## Assumptions

- **Earlier features are in place.** Every spec in `depends_on_landed` has landed. Every
  citation here was read at f6d4f5fd.
- **The simulator's mount stands for the real one** in the gates: an exact base tilt,
  zero points and backlash, and no collimation, axis-perpendicularity or flexure error;
  what those do to the second sync is the Python copy's and the night's (plan.md
  § Outside the factory).
- **Out of scope:**
  - star detection and saturated stars (052), the solver's matching, the databases and
    the scope node;
  - the imaging run's focus pause (054) and nudge buttons (055);
  - 038's push thresholds, and the cause of the field night's `mount.resets`;
  - the page: it shows a GoTo's reason and a stale pointing already (owner decision 5);
  - MCP's code: `get_session` returns the session the API returns
    (`crates/finder/src/mcp/tools.rs:538`, `crates/finder/src/api.rs:884`).
- **The factory's gates and CI:** the gates are `lint` and `test`, copied in plan.md
  § Verification. CI's other jobs run only in the merge queue, so no scenario's Then
  clause rests on them.

## Work Graph

```yaml
US1:
  depends_on: []
  implements: [FR-001, FR-002, FR-003, FR-004, FR-005]
US2:
  depends_on: []
  depends_on_merged: [US1]
  implements: [FR-006, FR-007, FR-008, FR-009, FR-010]
US3:
  depends_on: []
  depends_on_merged: [US2]
  implements: [FR-011, FR-012, FR-013]
```

Three stories, each under 60 % of 65,536 B (39,321 B) as the probe's commits measure
them, the largest US2 at 34.9 %, and at the factory's worst overrun seen, 1.35 times,
still under it, US2 at about 47.1 % (plan.md § Plan traps 2): the model; GoTo and the
second sync; the words. Each merges after the one before (`depends_on_merged`): US2's
loop waits on US1's taken syncs and reads US1's candidate, and its spot reads US1's
`Sync::axis_az_deg`; US3's words describe US2's progress and spot. Plain `depends_on`
would order them without putting the earlier code in the later story's base.
