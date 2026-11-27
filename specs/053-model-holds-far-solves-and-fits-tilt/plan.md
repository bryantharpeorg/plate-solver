# Implementation Plan: The Pointing Model Holds Far Solves, and the Second Sync Fits the Tilt

**Branch**: `053-model-holds-far-solves-and-fits-tilt` | **Date**: 2026-11-27 |
**Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/053-model-holds-far-solves-and-fits-tilt/spec.md`

## Summary

The pointing model gains one rule: **it takes at once only the solves it can vouch for,
and anything else waits for the next solve to agree with it.** It vouches for the
night's first sync, for a sync within its own slip bound of the prediction (014's 0.5°
once the base tilt is fitted and 2.5° before, unchanged), for the second sync when it
lies a quarter turn round on the mount's azimuth axis and fits a base tilt of at most
6°, and for the first solve after a push by hand (038's mark). Any other solve is held
as the model's `candidate`, in memory only. The next solve that the model, with the
candidate taken as a slip, would vouch for is taken as that slip and opens the new
zero-point epoch; a solve the model vouches for drops the candidate; any other replaces
it. A real slip repeats in the next solve and is taken one frame later than today; a bad
solve does not repeat and moves nothing.

GoTo and the second sync then wait for a solve the model took: the loop that today
corrects from the first solve after the settle passes over a held one and waits, within
the same 30 s, for one that is taken, and a timeout with a solve held names that frame.
The second sync's spot, which today has no altitude cap and is 90° away only on the sky
as a one-sync model predicts it, is also 90° away on the mount's azimuth axis, and is
looked for first between 30° and 60° up. With both, on the simulator's tilted mount, the
second sync fits a tilt of 0° to 5° in one epoch, within 1′, and the next GoTos' first
errors fall from 7,533″ to 45,412″ (on `main`, for 2° to 5°) to under 63″.

Three stories: the model (US1), GoTo and the second sync (US2), the words (US3). The
largest is US2 at 34.9 % of 65,536 B on the probe's code (§ Plan traps 2).

## Technical Context

**Language/Version**: Rust, MSRV 1.96, stable in CI.

**Primary Dependencies**: none new. `Cargo.toml` and `Cargo.lock` do not change.

**Storage**: none new. The candidate is `#[serde(skip)]` on `Model`, so `session.json`
keeps its shape; a refused sync writes nothing (today a refused sync writes nothing
either, `crates/finder/src/session.rs:1475`).

**Testing**:
- `crates/finder/tests/motion/model/intake.rs` (new, US1): US1-S1 to US1-S6 under
  `#[test]` on the model rig, US1-S7 under `#[tokio::test]` on the session rig.
- `crates/finder/src/model.rs`'s own `slips_refit_only_epoch_zero_points_at_the_half_degree_threshold`
  (US1-S8): one line replaced (spec owner approval 1).
- `crates/finder/tests/motion/goto/intake.rs` (new, US2): its own `Swap` solver,
  `rig_with` and the tilt rig; US2-S1 as six tests (one a tilt), US2-S2 to US2-S5 one
  each, all under `#[tokio::test(start_paused = true)]`.
- `crates/finder/tests/console/model_docs.rs` (new, US3): US3-S1 under `#[test]`.

**Target Platform**: the console beside the scope node, on the alt-az mount, with the
finder camera; nothing on the node, the page or the link changes.

**Constraints**:
- The live loop (`crates/finder/src/live.rs`), the tracker, 038's push rules, the
  solver, detection, the databases, the scope node and the page are not changed.
- `crates/finder/src/session.rs` changes in `record_sync` alone (two lines).
- 052 owns `crates/star-detection/` and `crates/scope-node/src/detect.rs`; 054 owns the
  imaging run's focus and run code and `ImagingPage.tsx`. This spec touches none of
  them.
- `RUSTFLAGS="-D warnings"`, `--all-features` clippy, and rustdoc with warnings denied.
- No test uses a camera, a mount, ASTAP or the network.

**Scale/Scope**: three stories; three changed library files (`model.rs`, `session.rs`,
`goto.rs`), three new test files, one replaced unit-test line, three `mod` lines, the
README's GoTo and second-sync paragraphs and one new paragraph, and three glossary
entries.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Detection and the plate solver are untouched; the model reads the solve the live loop gives it, as today. The rule adds no check on the solve itself (spec owner decision 4): the solver's own 5 % field bound (`crates/finder/src/live.rs:252`) and false-alarm test (`crates/finder/src/solve.rs:26`) are as they are. |
| II. Never weaken a gate | No golden, fixture, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS` or `ergane.yaml` is edited. One line of one existing unit test is replaced, an owner approval (spec owner approval 1): the test's 0.6° sync now needs a second solve to open its epoch, and every assertion it made stays. The only other edits to existing test files are three `mod` lines (SC-002). Every other test of the workspace passed unchanged on the probe's tip (§ What was run). |
| III. Tests first, independent expectations | The oracle of every Rust scenario is the simulator mount's geometry, written fresh in each test file (zero points added to the axis directions, then Rodrigues' rotation by the base tilt), the chord distance written in the test, the catalog's M42, the simulator's true eyepiece direction, and false solves made of another place's real star field. No test calls `finder::model::separation`, `predict` or the fit for an expected value, or reads `SLIP_DEG`, `MAX_BASE_TILT_DEG` or `SECOND_SYNC_BAND`; the docs test's oracle is the spec's sentences. Time comes from a paused tokio clock and the simulator's virtual clock. Each story's tests fail before it (§ What was run). |
| IV. Read the critical paths twice | None is touched: no `Cargo.toml` or `Cargo.lock` change, no `unsafe`, nothing under `proto/`, `crates/math-core`, `crates/pattern-database` or `crates/database-generation`, no `build.rs` or `.cargo/`. |
| V. Proven open source first, with honest licensing | Looked for, in local sources only (no download): the repository's survey of 2026-10-21 (`docs/research/2026-10-21-pointing-model-and-tracking.md:18`, § 1 and § 2), with its sources on TPoint, Taki's matrix method and INDI's alignment subsystem, Cedar, PiFinder, StarSense, N.I.N.A., Ekos, SharpCap, ASIAIR and SynScan; the vendored `reference-solutions/` (cedar-detect, cedar-solve); and the local crate cache. **TPoint** is the model's own source (IA, IE, AN, AW and the fixed terms) and a desktop program fitted from a list of observations by an operator, with no live intake to adopt. **INDI's alignment subsystem** (Taki's method) is a pure rotation that the survey finds leaves 32′ worst case for an IE of 0.25° (`docs/research/2026-10-21-pointing-model-and-tracking.md:59`), and a C++ library of a server this console does not run; not adopted. **N.I.N.A., Ekos and SharpCap** loop solve, sync and re-slew (`docs/research/2026-10-21-pointing-model-and-tracking.md:88`, `docs/research/2026-10-21-pointing-model-and-tracking.md:92`, `docs/research/2026-10-21-pointing-model-and-tracking.md:95`); the survey reads no rule in them for refusing a sync, so none is adopted; their loop is already 014's. **SynScan**'s alignment advice, 15° to 60° up and 45° to 135° apart (`docs/research/2026-10-21-pointing-model-and-tracking.md:101`), is the source of the second sync's band; it is advice, not code. **cedar-solve** (tetra3) holds a match to a false-alarm probability, already applied (`crates/finder/src/solve.rs:26`), and has no pointing model. **The local crate cache** holds no pointing-model, alignment or robust-fitting crate; `nalgebra`, a linear-algebra crate, is the nearest, and the model needs no more than it has. So the rule is written here: ten lines on the model's own prediction and fit, no number of its own but the 6° bound and the band. |

## Project Structure

```text
crates/finder/
├── README.md                                   # US3: GoTo and second-sync paragraphs, a new paragraph
├── src/
│   ├── model.rs                                # US1: the rule, the candidate, Sync::axis_az_deg; one test line
│   ├── session.rs                              # US1: record_sync keeps a held candidate
│   └── goto.rs                                 # US2: taken solves only, frame_id, the held failure, the spot
└── tests/
    ├── motion/model/intake.rs                  # US1, new
    ├── motion/model/mod.rs                     # US1: one mod line
    ├── motion/goto/intake.rs                   # US2, new
    ├── motion/goto/mod.rs                      # US2: one mod line
    ├── console/model_docs.rs                   # US3, new
    └── console/main.rs                         # US3: one mod line
GLOSSARY.md                                     # US3: Base tilt, Slip, Candidate
```

## Interfaces

The code as it ran on `probe/053` (tip 5b40ab27). Bodies not given here are the
builder's, to these words and the FRs.

### `crates/finder/src/model.rs` (US1)

```rust
/// The largest base tilt a sync may fit before it is read as a slip, in degrees (053).
pub const MAX_BASE_TILT_DEG: f64 = 6.0;
/// Sky distance from the prediction within which the model takes a sync at once,
/// once the base tilt is fitted and before, in degrees (014's FR-013).
pub const SLIP_DEG: [f64; 2] = [0.5, 2.5];

impl Sync {
    /// The azimuth axis's own direction in degrees, which no model error moves (053).
    pub fn axis_az_deg(&self) -> f64 {
        self.axis[0] * 360.0 / f64::from(self.counts_per_revolution[0])
    }
}

pub struct Model {
    // … every field as today, then:
    /// A solve the model could not vouch for, held until a second solve agrees
    /// with it (053). Run-only.
    #[serde(skip)]
    pub candidate: Option<Sync>,
    // … `motion` as today
}

pub struct ModelView {
    // … every field as today, then:
    /// A solve held until a second solve agrees with it (053).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate: Option<CandidateView>,
}
/// A held solve: its distance from the model's prediction and its time.
#[derive(Debug, Serialize)]
pub struct CandidateView {
    /// Sky distance from the model's prediction, degrees.
    pub off_deg: f64,
    /// Mid-exposure, console Unix seconds.
    pub time: f64,
}
```

`add_sync` keeps its validity checks (`crates/finder/src/model.rs:493` to `crates/finder/src/model.rs:507`) and
then, in place of today's slip test (`crates/finder/src/model.rs:508`):

```rust
if self.slip_pending || self.vouches(&sync) {
    self.candidate = None;
    let slip = self.slip_pending;
    self.take(sync, slip);
    return true;
}
// A second solve agrees when the model, with the held solve taken as the
// slip it would then be, vouches for it. It opens the epoch in its place.
if let Some(candidate) = self.candidate.take() {
    let mut trial = self.clone();
    trial.take(candidate, true);
    if trial.vouches(&sync) {
        self.take(sync, true);
        return true;
    }
}
self.candidate = Some(sync);
false
```

- `fn vouches(&self, sync: &Sync) -> bool`: true when `predict` gives nothing (the
  night's first sync); before the base tilt is fitted, when `tilt_spot(sync)`, the
  answer is whether a clone that takes the sync (`take(sync, false)`) holds a base tilt
  of at most `MAX_BASE_TILT_DEG`; otherwise whether the sync lies within
  `SLIP_DEG[usize::from(self.base_tilt.is_none())]` of the prediction.
- `fn tilt_spot(&self, sync: &Sync) -> bool`: a sync of the current epoch
  (`self.epochs.len() - 1`) whose `axis_az_deg()` differs from the sync's by at least
  90°, through `wrap`.
- `fn take(&mut self, sync: Sync, slip: bool)`: today's body from the epoch push
  (`crates/finder/src/model.rs:515`) to the end of `add_sync`, less its final `true`,
  with `slip` false on an empty model, and the base-tilt test reading `axis_az_deg()` in place of `az_deg`
  (`crates/finder/src/model.rs:535`).
- `reset_night` sets `candidate` to `None`; `view` fills `candidate` from the held sync:
  `off_deg` the `separation` of `predict` at its axis positions from its azimuth and
  altitude, `time` its time.
- The doc of `Model::base_tilt` reads "Absent until two syncs in an epoch are at least
  90° apart on the azimuth axis."

The unit test's line (`crates/finder/src/model.rs:911`), as owner approval 1 gives it:

```rust
// 053: a sync past the bound is held until a second solve repeats it.
assert_eq!(model.add_sync(sync.clone()), epochs == 1);
if epochs == 2 {
    assert_eq!(model.epochs.len(), 1);
    sync.time += 1.0;
    assert!(model.add_sync(sync.clone()));
}
```

### `crates/finder/src/session.rs` (US1)

In `record_sync` (`crates/finder/src/session.rs:1475`):

```rust
if !next.model.add_sync(sync) {
    // A held solve is run-only: kept without a state write (053).
    values.model.candidate = next.model.candidate.take();
    return Ok(false);
}
```

### `crates/finder/src/goto.rs` (US2)

```rust
/// The second sync's altitude band, degrees (053): searched first.
pub const SECOND_SYNC_BAND: [f64; 2] = [30.0, 60.0];

pub struct Progress {
    // … every field as today, then:
    /// The star frame whose solve, taken as a sync, gave `error_arcsec` (053).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame_id: Option<u64>,
}
```

- `State` gains `frame: Option<u64>`, set to `None` where a GoTo or second sync starts
  and copied into every `Progress` that `update` writes.
- `Planner::second_sync` (`crates/finder/src/goto.rs:265`) keeps today's clearance, cone
  and sky test, and adds: the azimuth axis directions of the current epoch's syncs
  (`axis_az_deg`), and for each spot (e, a) the test that `self.model.inverse(a,
  ephemeris::apparent_from_true(e), self.counts())` gives an azimuth count whose
  degrees lie at least 90° (less 10⁻⁹) from every one of them; the first altitude tried
  is `alt.clamp(SECOND_SYNC_BAND[0], SECOND_SYNC_BAND[1]).max(self.min_alt)` in place
  of `alt.max(self.min_alt)` (`crates/finder/src/goto.rs:280`); and the spots are
  sorted with those inside the band first, then by today's distance.
- The settle loop (`crates/finder/src/goto.rs:955` to `crates/finder/src/goto.rs:1011`): after a report's
  pointing matches its frame (`crates/finder/src/goto.rs:982`), when no sync of
  `self.session.model(report.time)` has the report's time, it records
  `(report.frame_id, candidate.off_deg)` if the model's candidate has that time, and
  goes on to the next report; otherwise it sets `State::frame` to the report's frame
  and returns the error (a GoTo) or nothing (a second sync), as today. On the 30 s
  timeout it fails with
  `format!("frame {frame} solved {off:.1}° from the pointing model and no second solve agreed")`
  when a solve was recorded, and `"no frame solved after settling"` when none was.

### The README (`crates/finder/README.md`, US3)

In the GoTo paragraph (`crates/finder/README.md:572`), "then uses a solved exposure
starting after that wait as a sync." becomes "then for a solved exposure starting after
that wait that the pointing model takes as a sync, and corrects from that sync alone.";
the `goto` keys gain `frame_id`, with "; `frame_id` is the star frame whose sync gave
`error_arcsec`."; and after the paragraph's last sentence, which ends "is pending.",
FR-012's third sentence. In the
second-sync paragraph (`crates/finder/README.md:586`), after "before the base tilt is
fitted", ", " and FR-012's fourth sentence, in place of the full stop. After that paragraph, a new one:

> The pointing model takes a live solve as a sync only when it can vouch for it: the
> night's first sync; a solve within 0.5° of the model's prediction, or within 2.5°
> before the base tilt is fitted; before the base tilt is fitted, a solve at least 90°
> on the azimuth axis from a sync of the current zero-point epoch that fits a base tilt
> of at most 6°, which joins that epoch and fits the tilt; and the first solve after a
> push by hand. Any other solve is held as the model's candidate, shown in
> `GET /api/session` as `model.candidate: {off_deg, time}`, its distance from the
> prediction in degrees and its mid-exposure time. The next solve that the model, with
> the candidate taken as a slip, would vouch for is taken as that slip, opening a new
> zero-point epoch, and the candidate is dropped: a slip repeats in the next solve, and
> a bad solve does not. A solve the model vouches for drops the candidate, and any
> other takes its place. The candidate is never written to the state file. A first sync
> near the zenith gives the second sync less to fit the base tilt against: take the
> first between 30° and 60° up when the sky allows.

### The glossary (`GLOSSARY.md`, US3)

- **Base tilt**: "How far, and toward which azimuth, the mount's base is off level
  (TPoint's AN and AW). Fitted once two syncs of one zero-point epoch lie at least 90°
  apart on the azimuth axis, up to 6°." (_Avoid_ unchanged.)
- **Slip**: "The axes moving without their readings following, as after a push by hand
  or a power cycle. A solve that a second solve repeats beyond the model's prediction,
  or the first after a push by hand, opens a new zero-point epoch." (_Avoid_
  unchanged.)
- **Candidate**, new, after **Slip**: "A live solve the pointing model cannot vouch for,
  held back from it until a second solve agrees with it, which is then taken as a slip;
  a solve the model vouches for drops it." _Avoid_: outlier, false solve (a candidate
  is often a true solve after a slip).

## What was run and what was read

Logs are under the operator's probe folder, not in the repository, numbered in the
order they ran.

**The commits**, on `probe/053`, cut from f6d4f5fd: 2bee1989 (the probe's measurement
on `main`'s code, not a story: a printing test and a `Rig::with_mount` helper in
`crates/finder/tests/support/goto.rs`), 8570c83f (the same, renamed `probe_tilts` for
the layout check, and the spot tables, not a story), 0bdd2ddf (US1) and 5b40ab27 (US2
and US3 together; their sizes are measured by path, § Plan traps 2).

**Run:**

- **The field night, reproduced** (log 02, on `main`'s code): the tilt rig at
  (75°, 122°), tilts 0° to 5° toward 195° and 300°, the second sync and then GoTos to
  (36°, 127°) and (40°, 250°). At 0° and 1° the second sync fitted the tilt in one epoch
  and the first errors were 51″ to 62″. At 2° to 5° the second sync opened a second
  epoch and no tilt was fitted, and the first errors were 7,533″ to 45,412″; each GoTo
  still ended `done` after two or three corrections, and each opened further epochs (3
  or 4). A 3° tilt toward 195° gave 17,196″ to (36°, 127°), against the field night's
  17,960″ to M42.
- **A Python model of the two-sync fit** (log 01, outside the repository): with exact
  geometry, any two syncs 90° apart fit the tilt exactly; with 0.25° of collimation or
  of non-perpendicularity unmodelled and a 3° tilt, the worst error over targets 20° to
  80° up after syncs at 75° and 79° was 0.85° and 0.98°, after 75° and 30°, 45° or 60°,
  0.44° to 0.58°, and after 45° and 45°, 0.19° and 0.20°.
- **The first trial** (log 03): the vouch rule with the second sync's spot 90° away on
  the sky only, its allowance taken at the spot's own altitude. At 5° toward 45° and
  90° from 75° up, the solved azimuths were 87.6° and 89.3° apart and the second sync
  opened an epoch. Its spot table met nextest's 5-minute limit.
- **The axis trial** (log 04): the axis rule added. 56 of 56 cases of 1°, 3° and 5°
  fitted the tilt in one epoch, from 45°, 75° and 82° up, eight directions each (1° from
  75° only), the solved azimuths as little as 51.7° apart; at 8°, 8 of 8 fitted none and
  opened a second epoch. With the axis rule alone (the sky rule dropped),
  `goto::second_sync::one_tap_slews_takes_a_sync_and_fits_the_base_tilt_without_corrections`
  failed its 90° on the sky, so the sky rule was kept as well.
- **The `finder` package's tests on the trial** (log 05): all but two passed: the unit
  test of owner approval 1 (before its line was replaced) and the layout check, which
  refused the probe's module name `probe053` (renamed `probe_tilts`).
- **The story tests** (log 06): every new test passed; the replaced unit test failed on
  the base tilt, because the confirming solve then refitted every term. The rule was
  changed so that the confirming solve itself opens the epoch (`take(sync, true)`),
  keeping the base tilt and fixed terms as 014's FR-013 says (log 07: the 24 tests of
  the model, the GoTo, the hand push and the night, all passed).
- **US1 alone** (log 08, 0bdd2ddf exported): the `finder` package's 1,028 tests,
  all passed.
- **Each story's tests fail before it.** US2's file on US1's code alone (log 09): 9 of
  its 10 tests fail, the six tilt cases and US2-S4 on `frame_id` absent, US2-S5 on its
  reason (`no frame solved after settling`) and US2-S2 on the altitude; US2-S3 passes on
  US1's code, since it is US1's rule through the route. US1's file on `main` and US3's
  on US2's code: read, not run (US1's reads `Model::candidate`, which `main` lacks, so
  the motion binary does not build; the README and glossary on US2's code lack every
  sentence of FR-012 and FR-013).
- **The gates on the tip** (logs 10 to 13, 5b40ab27), with the gates' own `RUSTFLAGS`
  (§ Verification): `lint` passed (`cargo fmt --check`, clippy with `-D warnings`, both
  `cargo doc` runs); nextest 1,543 of 1,543, the probe's own five measurement tests
  (`probe_tilts`) left out, and every doc test passed; the page half, though the page is
  untouched, since the browser tests drive this console: `npm ci --offline`, the
  typecheck, Prettier, Vitest (607 of 607), the build, the dist check (clean) and
  `CI=1 npm run e2e` (34 of 34, with the factory's own browsers; a first run without
  them could not start a browser, log 12).

**Read, not run:**

- The real mount's terms beyond the simulator's (collimation, non-perpendicularity,
  flexure): only the Python model above.
- A blocked second-sync spot, a moved aim point after a tap (032), a console restart
  with a solve held, and the same false solve twice: spec.md § Edge Cases.
- MCP's `get_session` carrying `model.candidate` and `goto.frame_id`: it returns
  `api::session_response` (`crates/finder/src/mcp/tools.rs:538`,
  `crates/finder/src/api.rs:884`), the view the session tests read.
- That the field night's far reading was a true solve through a wrong model, not a false
  match: the operator seat's triage, outside the repository.

## Read from the diff, not from a test

Each passes every scenario built the other way; the judge reads it in the diff.

- FR-002's "the RMS and slip mark are unchanged" on a refusal: US1-S3 compares the syncs
  and epochs only.
- FR-003's "within one epoch": every two-sync scenario has one epoch.
- FR-004's drop on expiry and on `POST /api/model/reset` with `{"what":"night"}`:
  `reset_night` is what both call (`crates/finder/src/model.rs:433`); US1-S6 calls it.
- FR-006's "within the same 30 s": US2-S4's held frames are four, within it.
- FR-007's "until the next GoTo or second sync starts".
- FR-008's one decimal place: US2-S5 checks the reason's start and end.
- FR-009's "only when none of those has a safe path, the nearest outside it": no
  scenario blocks the band.
- FR-011's paragraphs beyond FR-012's sentences.

## Shapes that existing tests pin

- **The 0.5° slip test.** `crates/finder/src/model.rs:898`'s
  `slips_refit_only_epoch_zero_points_at_the_half_degree_threshold` adds a 0.6° sync
  and asserts a second epoch: one line replaced (owner approval 1; US1-S8).
- **The second sync on the sky.** `goto::second_sync::one_tap_slews_takes_a_sync_and_fits_the_base_tilt_without_corrections`
  asserts the second sync 90° from the first on the sky: the sky rule is kept beside
  the axis rule (run: log 04 without it, log 11 with it).
- **Hand pushes and slips.** `crates/finder/tests/motion/goto/sim.rs:159`'s
  `hand_push_opens_an_epoch_and_goto_still_converges`,
  `goto::sim::repeated_half_degree_slips_fail_after_five_corrections_with_error`,
  `night::a_whole_night_syncs_gotos_hand_push_stop_and_dropped_link` and the
  `steering_*` and `tracking::push*` tests pass unchanged: a push with Track is marked
  (`crates/finder/src/session.rs:1302`), and one without it is taken once two solves
  agree (run).
- **A mount reset.** `crates/finder/tests/motion/mount_identity.rs` passes unchanged:
  the solve after a reset is held and the next one is taken, one frame later (run).
- **The session's JSON.** `model.candidate` and `goto.frame_id` are absent when none,
  and no test compares the `model` or `goto` objects whole (read); the page's types
  (`crates/finder/web/src/api.ts:854`) ignore the new keys.
- **The state file.** `crates/finder/tests/motion/model/routes.rs` writes and reads
  `session.json`: the candidate is skipped, so its shape is unchanged (run).
- **The imaging run's GoTo.** The `imaging_run*` tests pass unchanged (run).

## Decisions and the alternatives rejected

The six owner decisions, all open, are in spec.md § Owner decisions, each with its
recommendation. Below them:

| Decision | Chosen | Rejected |
|---|---|---|
| Where the rule lives | `Model::add_sync`, which every sync passes through (the live loop, the second sync, the GoTo) | the live loop (a second path to the model, for 038's push mark too); the GoTo (a sync the live loop took would already have moved the model) |
| What confirms a held solve | the next solve the model, with the held one taken as a slip, vouches for; that solve opens the epoch | refitting with both (it refits the base tilt and fixed terms, which 014's FR-013 keeps; run: log 06) |
| How far apart for the tilt | 90° on the azimuth axis, within an epoch | 90° in solved azimuth, today's (a high first sync puts it under 90°: log 03) |
| The second sync's spot | today's sky rule, plus the axis rule, the band first | the axis rule alone (`one_tap_slews_…` asserts 90° on the sky: log 04); a band only (log 03's slips at 5°) |
| What a GoTo corrects from | a solve the model took | the first solve after the settle, today's (a held solve would steer it) |
| What the page shows | nothing new (decision 5) | a Setup line (a page story for a case the GoTo's reason already names) |
| The story cut | three stories, US2 the largest at 34.9 % | two (the model with GoTo: 63.8 % before any overrun) |

## User Story 1 - The pointing model

The rule in `Model::add_sync`, the candidate, `Sync::axis_az_deg`, the session keeping
the candidate, the unit test's line, and the model's test file.

## User Story 2 - GoTo and the second sync

The settle loop's wait for a taken solve, `frame_id`, the held failure, the spot's axis
rule and band, and the GoTo test file. Its tests read US1's candidate.

## User Story 3 - The words

The README's paragraphs and the glossary's entries, and their test.

## Verification (every story, before proposing)

- **The factory gates** (`ergane.yaml` at f6d4f5fd):
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

1. **The earlier specs have landed:** every spec in `depends_on_landed`. Met: each is
   on `main` at f6d4f5fd.
2. **`main` is still as read.** Every citation here was read at f6d4f5fd. If `main`
   moves in a file § Project Structure names, or in a test § Shapes names, re-read them
   and re-run the validator. 052 and 054, drafted beside this one, write none of this
   spec's files.
3. **The owner decisions are settled and the owner approvals given** (spec § Owner
   decisions, § Owner approvals). Not met: all six are open.
4. **Probe builds.** A probe round builds the stories from this text alone, and its
   defects are folded. Not met: the draft's probe built them before this text.
5. **No operator-staged files are needed.** Met: no story reads a file outside the
   repository.

## Outside the factory

Written, not run. The owner's, on the next field night, on the alt-az mount with the
finder camera.

1. **After the first sync**, read `GET /api/session` (or MCP's `get_session`) and write
   down `model.epoch`, the altitude the telescope was at, and whether `model.base_tilt`
   is absent.
2. **Press the second sync.** When `goto.state` is `done`, write down `goto.frame_id`,
   the altitude it went to, `model.epoch` (expected unchanged), `model.base_tilt.deg`
   and `model.base_tilt.toward_deg`, `model.rms_arcsec` and whether `model.candidate`
   was ever present while it ran.
3. **Two far GoTos.** Choose two targets more than 90° apart in azimuth and more than
   30° up, neither the second sync's place. For each, write down the first
   `goto.error_arcsec` after the slew, its `goto.frame_id`, the number of corrections,
   the end state, and `model.epoch` after it.
4. **If a GoTo fails** with `frame <id> solved …`, write down the reason and
   `model.candidate`, and press GoTo again once.
5. **What would change the design**, in a follow-up spec: a second sync that opens an
   epoch on a base under 6° off level; a first error to either far target over 1,800″
   (half a degree) with the tilt fitted; or a GoTo failed with a held solve while the
   telescope stood still.

## Plan traps

They bind every story.

1. **Gate evidence or nothing.** The judge credits only gate results and the story's
   diff. *Check:* every Then names a committed test the `test` gate runs.
2. **One evidence budget.** *Check:* each story's whole diff stays under 60 % of
   65,536 B (39,321 B), measured with `git diff <base> -- . | wc -c` from the previous
   story's tip. On the probe's code, by path: US1 (`model.rs`, `session.rs`,
   `tests/motion/model/`) 18,970 B (28.9 %), US2 (`goto.rs`, `tests/motion/goto/`)
   22,866 B (34.9 %), US3 (the README, the glossary, `tests/console/`) 9,440 B (14.4 %).
   At the 1.35 times a factory build has run over its probe on earlier specs: US1 about
   25,600 B (39.1 %), US2 about 30,900 B (47.1 %), US3 about 12,700 B (19.4 %), each
   under the 60 % line and far under the 128 KiB refusal.
3. **A spec fixed mid-epic needs a relaunch.** *Check:* no builder edits
   `specs/053-model-holds-far-solves-and-fits-tilt/` or `.specify/`.
4. **An earlier story's test file is an existing test.** *Check:* US2 and US3 write no
   file of an earlier story's tests; US2's rig is its own, in its own file.
5. **No support edit.** *Check:* nothing under `crates/finder/tests/support/` changes;
   US2's `rig_with` builds `crate::support::goto::Rig`'s public fields in its own file
   (the probe's `Rig::with_mount` was the probe's alone).
6. **The layout check reads module names.** `crates/finder/tests/layout/main.rs:155`
   allows lowercase letters and underscores only. *Check:* the new modules are
   `intake` (twice) and `model_docs`.
7. **An oracle copied from the code proves nothing.** *Check:* the tests write the
   mount's geometry, the chord and the bounds as literals; none calls the model's
   `separation`, `predict` or fit for an expected value, or reads the new constants.
8. **A negative assertion passes when nothing happens.** *Check:* US1-S3's unchanged
   syncs stand beside its candidate's `off_deg`; US1-S5's two refusals beside the
   candidate's time 401; US2-S4's unchanged epochs beside the candidate read while it
   ran and the `frame_id` past the window; US2-S5's absent `error_arcsec` beside its
   reason.
9. **Paused time and the simulator's clock.** *Check:* every GoTo test runs under
   `start_paused = true` and every wait is bounded by `finish_with`'s 5,000 reads.
10. **nextest ends a test after 5 minutes** (`.config/nextest.toml:23`). *Check:* each
    of US2-S1's tilts is a test of its own, two directions each, as the probe's are.
11. **The live loop records first, GoTo reads after.** *Check:* the settle loop reads
    the session's model at the report's time after the pointing names the report's
    frame, as today's second sync does (`crates/finder/src/goto.rs:995`).
12. **The session lock is never held across an await.** *Check:* `record_sync` moves
    the candidate inside its existing lock; `goto.rs` takes `self.session.model(…)`
    by value.
13. **Words: "finder camera", never "finder" alone,** in every message; the glossary's
    terms exactly (zero-point epoch, base tilt, slip, sync, candidate).

## Risks

| Risk | Mitigation |
|---|---|
| A false solve at the second sync's spot that fits a tilt of at most 6° would be taken. | It must land 90° round on the axis and fit a plausible tilt at once; the false-alarm test makes even one false match rare (spec.md § Clarifications); the next GoTo's corrections and the night's later syncs refit the zero points. |
| The real mount's unmodelled terms (collimation, non-perpendicularity, flexure) leave more after two syncs than the simulator does. | The band halves it on the Python model; the GoTo corrects locally; § Outside the factory step 5 says what would change the design. |
| A real slip is taken one frame later than today. | One frame is a fraction of a GoTo's settle and solve; 024's refusal ends on that frame. |
| Two different false solves in a row hold the model's last epoch while the telescope moves on. | Each is refused; the next true solve within the bound drops the candidate (US1-S3), and a GoTo waits for it within its 30 s. |
| A story grows past 60 % in the factory. | The largest is 34.9 % on the probe's code and about 47.1 % at the worst overrun seen. |
| A new RustSec advisory fails `cargo-deny` in the merge queue. | Not this spec's to answer: the advisory's ignore is the owner's. |

## Refinement Record

Drafted on 2026-11-27 by the plate-solver operator seat's spec session against `main` at
f6d4f5fd, from a probe of that day on `probe/053` (the field night reproduced on the
simulator's tilted mount, two trials of the second sync's spot, a trial build of the
three stories with their scenarios' tests, US1 alone and US2's tests on US1's code, and
both Rust gates on the tip), and a reading of 014's pointing model, GoTo and second
sync, 013's live loop, 024's refusal after a reset, 038's push rules, 032's and 050's
uses of a sync, MCP's session tool, the constitution, the glossary and the repository's
pointing-model survey. Writes come from the task text. Shared writes name the spec story
that last wrote the same file on `main`; within the spec, each story's files written by
an earlier one are ordered by `depends_on_merged`.

The six owner decisions of spec.md are open, each with its recommendation; this record
is written to the recommendations. Its `open_questions` are empty so that the validator
reports the record's structure; the decisions themselves are spec.md's, and the flip,
which settles them, is the owner's. This record approves nothing.

```yaml
version: 1
reviewed_revision: f6d4f5fd482ccc9d5b634ef1aa5124c112769583
stories:
  US1:
    tests:
      - path: crates/finder/tests/motion/model/intake.rs
        status: new
        disposition: add
        assertion: "model rig, syncs at axis (105, 77) at 0 s and (0, 45) at 120 s under zero points (17, -2), tilts 0, 1, 3 and 5 toward 195: both add_sync true, one epoch, base_tilt deg within 1 arcmin of the tilt and toward_deg within 1 arcmin of 195 above 0, predict at axis (250, 30) within 1 arcsec of the geometry, no candidate"
        decision: "spec.md:US1-S1"
      - path: crates/finder/tests/motion/model/intake.rs
        status: new
        disposition: add
        assertion: "model rig under 8 degrees toward 195, the syncs of US1-S1 and the second again at 121 s: first true, second false with candidate off_deg above 2.5 and time 120, one sync and one epoch; the third true, two epochs, no base tilt, no candidate"
        decision: "spec.md:US1-S2"
      - path: crates/finder/tests/motion/model/intake.rs
        status: new
        disposition: add
        assertion: "fitted model (syncs at (30, 40) at 0 s and (140, 50) at 200 s, 1 degree toward 130), a sync at (200, 45) at 400 s with error (5, 0): false, syncs and epochs as fitted, candidate off_deg within 1 arcsec of the chord from the geometry's direction to it 5 degrees further in azimuth; a sync at (200, 45) at 401 s: true, one epoch, three syncs, no candidate"
        decision: "spec.md:US1-S3"
      - path: crates/finder/tests/motion/model/intake.rs
        status: new
        disposition: add
        assertion: "fitted model, syncs at (200, 45) at 400 s and (201, 46) at 410 s under zero points (20, -2): first false, second true, two epochs, three syncs, the newest at 410 s, predict at axis (230, 35) within 1 arcsec of the slipped geometry"
        decision: "spec.md:US1-S4"
      - path: crates/finder/tests/motion/model/intake.rs
        status: new
        disposition: add
        assertion: "fitted model, a sync at (200, 45) at 400 s with error (5, 0) and at 401 s with error (-7, 3): both false, two syncs, one epoch, candidate time 401"
        decision: "spec.md:US1-S5"
      - path: crates/finder/tests/motion/model/intake.rs
        status: new
        disposition: add
        assertion: "fitted model with mark_slip, a sync at (200, 45) at 400 s under zero points (20, -2): true, two epochs; a sync at (210, 45) at 500 s under (17, -2): false, a candidate in view; after reset_night: view None and candidate None"
        decision: "spec.md:US1-S6"
      - path: crates/finder/tests/motion/model/intake.rs
        status: new
        disposition: add
        assertion: "session rig with the fitted model's two syncs through record_sync, the US1-S3 sync at 400 s with error (5, 0): Ok(false), the state file's bytes unchanged; GET /api/session model.syncs 2, model.candidate.time 400, off_deg within 1 arcsec of US1-S3's; the sync at (200, 45) at 401 s: Ok(true); GET model.syncs 3 and no candidate key"
        decision: "spec.md:US1-S7"
      - path: crates/finder/src/model.rs
        status: existing
        disposition: replace
        assertion: "slips_refit_only_epoch_zero_points_at_the_half_degree_threshold, 24 spread syncs, a sync at (180, 50) at 5000 s moved 0.4 and 0.6 degrees in altitude: add_sync true for 0.4 and false for 0.6 with one epoch, the 0.6 sync again at 5001 s true; then two epochs, base tilt and fixed terms unchanged, predict at the sync within 1 arcsec in azimuth and altitude, the altitude zero moved by more than 0.5"
        decision: "spec.md:US1-S8"
      - path: crates/finder/tests/motion/mount_identity.rs
        status: existing
        disposition: preserve
        assertion: "after a mount reset the console refuses motion until a sync from a frame that started after the rise, as 024 words it"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/motion/goto/sim.rs
        status: existing
        disposition: preserve
        assertion: "a hand push opens an epoch and GoTo still converges; repeated half-degree slips fail after five corrections"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "014's slip bounds 0.5 and 2.5 degrees, its thinning, its fit and its epochs, and 038's slip mark taken at once"
      - "session.json's shape: the candidate is never written"
    checks:
      development:
        - "cargo nextest run -p finder --locked -E 'test(/^model::intake::/) | test(slips_refit_only)'"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs and the field night of plan.md § Outside the factory"
    writes:
      - crates/finder/src/model.rs
      - crates/finder/src/session.rs
      - crates/finder/tests/motion/model/intake.rs
      - crates/finder/tests/motion/model/mod.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/model.rs
        other_spec: 038-track-follows-a-push
        other_story: US1
        disposition: merge_order
        reason: "053 depends_on_landed 038, whose US1 last changed model.rs (ad09efeb)"
      - path: crates/finder/src/session.rs
        other_spec: 047-console-uses-the-database-that-fits
        other_story: US2
        disposition: merge_order
        reason: "053 depends_on_landed 047, whose US2 last changed session.rs (c15937e7)"
      - path: crates/finder/tests/motion/model/mod.rs
        other_spec: 027-finder-tests-by-system
        other_story: US2
        disposition: merge_order
        reason: "053 depends_on_landed 027, whose US2 last changed model/mod.rs (b8dceaa9)"
    open_questions: []
  US2:
    tests:
      - path: crates/finder/tests/motion/goto/intake.rs
        status: new
        disposition: add
        assertion: "tilt rig at (75, 122), tilts 0, 1, 2, 3, 4 and 5 toward 195 and 300: POST /api/model/second-sync done, every read correction 0, two syncs, one epoch, base_tilt deg within 1 arcmin of the tilt; GoTos to (36, 127) and (40, 250) each done, first error_arcsec under 120, frame_id above the latest frame id at its start; one epoch after both"
        decision: "spec.md:US2-S1"
      - path: crates/finder/tests/motion/goto/intake.rs
        status: new
        disposition: add
        assertion: "level rig at (75, 122), (45, 122) and (20, 122): second sync done, true altitude within 0.2 of 60, 45 and 30, the azimuth axis turned at least 90 and the true azimuth changed at least 90, one epoch"
        decision: "spec.md:US2-S2"
      - path: crates/finder/tests/motion/goto/intake.rs
        status: new
        disposition: add
        assertion: "tilt rig at (75, 122), 8 degrees toward 45: second sync done, a model.candidate read while it ran, two epochs, no base tilt"
        decision: "spec.md:US2-S3"
      - path: crates/finder/tests/motion/goto/intake.rs
        status: new
        disposition: add
        assertion: "GoTo rig, 1 degree toward 130, zero points (120, 50), syncs at counts (4500, 4500) and (580000, 4500) with their fields kept, a GoTo to M42 with Swap's window the four frames after the latest at the first solving read: done, frame_id at least the window's end, error_arcsec and Rig::error_arcsec each under 60, a candidate read, epochs unchanged"
        decision: "spec.md:US2-S4"
      - path: crates/finder/tests/motion/goto/intake.rs
        status: new
        disposition: add
        assertion: "the US2-S4 rig, a GoTo to M42 with Swap's window from the frame after the latest without an end: failed, reason starting 'frame ' and ending 'from the pointing model and no second solve agreed', no error_arcsec, epochs unchanged"
        decision: "spec.md:US2-S5"
      - path: crates/finder/tests/motion/goto/second_sync.rs
        status: existing
        disposition: preserve
        assertion: "one tap slews to a spot at least 90 degrees from the first sync on the sky, takes the sync and fits the base tilt without corrections"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/motion/night.rs
        status: existing
        disposition: preserve
        assertion: "a whole night of syncs, GoTos, a hand push, a stop and a dropped link"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "the settle of 0.5 s, the 30 s wait, five corrections to 1 arcmin, the states and the refusals of 014 and 024"
      - "the second sync's 90 degrees on the sky with its allowance, its Sun guard, cable-wind and approach checks"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test motion -E 'test(/^goto::intake::/)'"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs and the field night of plan.md § Outside the factory"
    writes:
      - crates/finder/src/goto.rs
      - crates/finder/tests/motion/goto/intake.rs
      - crates/finder/tests/motion/goto/mod.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/goto.rs
        other_spec: 024-mount-link-recovery
        other_story: US3
        disposition: merge_order
        reason: "053 depends_on_landed 024, whose US3 last changed goto.rs (20839158)"
      - path: crates/finder/tests/motion/goto/mod.rs
        other_spec: 027-finder-tests-by-system
        other_story: US2
        disposition: merge_order
        reason: "053 depends_on_landed 027, whose US2 last changed goto/mod.rs (b8dceaa9)"
    open_questions: []
  US3:
    tests:
      - path: crates/finder/tests/console/model_docs.rs
        status: new
        disposition: add
        assertion: "docs rig, crates/finder/README.md and GLOSSARY.md whitespace joined: the README holds FR-012's eight sentences and the glossary FR-013's three, word for word"
        decision: "spec.md:US3-S1"
      - path: crates/finder/tests/console/imaging_run_docs.rs
        status: existing
        disposition: preserve
        assertion: "the README's imaging run sections and its MCP count, as 041 and 042 word them"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "every README section and glossary entry this spec does not name"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console -E 'test(model_docs)'"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - GLOSSARY.md
      - crates/finder/README.md
      - crates/finder/tests/console/main.rs
      - crates/finder/tests/console/model_docs.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/README.md
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US3
        disposition: merge_order
        reason: "053 depends_on_landed 051, whose US3 last changed the README (9424a371)"
      - path: GLOSSARY.md
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US3
        disposition: merge_order
        reason: "053 depends_on_landed 051, whose US3 last changed the glossary (9424a371)"
      - path: crates/finder/tests/console/main.rs
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US2
        disposition: merge_order
        reason: "053 depends_on_landed 051, whose US2 last changed console/main.rs (f9706cc0)"
    open_questions: []
```
