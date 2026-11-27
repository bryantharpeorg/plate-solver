# Implementation Plan: The Second Sync Fits the Base Tilt

**Branch**: `053-second-sync-fits-base-tilt` | **Date**: 2026-11-27 |
**Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/053-second-sync-fits-base-tilt/spec.md`

## Summary

The pointing model gains one rule: **before the base tilt is fitted, a sync a quarter turn
round on the mount's azimuth axis from a sync of the current zero-point epoch is judged by
the tilt it fits alone**: at most 6°, it joins the epoch and fits the tilt; above, it is a
slip. Base-tilt pairs are read on the axis, not in solved azimuth. Every other sync keeps
014's bounds (0.5° once the base tilt is fitted, 2.5° before), and 038's slip mark is taken
at once as today, at a tilt spot too, before its tilt is judged. No solve is held, and no
state is added.

The second sync looks for its spot first between 30° and 60° up, the altitudes searched as
today, and ends `failed` with `taken as a slip: no base tilt fitted` when it
leaves no base tilt. A live solve whose field lies more than 5 % outside the range of the
database in use is no sync. A GoTo and the second sync wait for a solve the model took,
name it (`goto.frame_id`, `goto.frame_time`), and a timeout after solves not taken names
the last. With these, on the simulator's tilted mount, the second sync fits a tilt of 0° to
5° in one epoch, within 1′, and the next GoTos' first errors fall from 7,533″ to 45,412″
(on `main`, for 2° to 5°) to under 120″.

Three stories: the model (US1), the second sync, the field and GoTo (US2), the words
(US3). The largest is US2 at 43.2 % of 65,536 B on the probe from this text with its fold
(§ Plan traps 2).

## Technical Context

**Language/Version**: Rust, MSRV 1.96, stable in CI.

**Primary Dependencies**: none new. `Cargo.toml` and `Cargo.lock` do not change.

**Storage**: none new; `session.json` keeps its shape. A refused sync writes nothing, as
today (`crates/finder/src/session.rs:1475`).

**Testing**:
- `crates/finder/tests/motion/model/tilt.rs` (new, US1): US1-S1 to US1-S8 under `#[test]`
  on the model rig.
- `crates/finder/tests/motion/goto/tilt.rs` (new, US2): its own `rig_with` and the tilt
  rig; US2-S1 as six tests (one a tilt, two directions each), US2-S2 and US2-S3 one each.
- `crates/finder/tests/motion/goto/field.rs` (new, US2): the Scale solver over
  `super::tilt::rig_with`; US2-S4 and US2-S5 one each. Every US2 test runs under
  `#[tokio::test(start_paused = true)]`.
- `crates/finder/tests/console/model_docs.rs` (new, US3): US3-S1 under `#[test]`.

**Target Platform**: the console beside the scope node, on the alt-az mount, with the
finder camera; nothing on the node, the page or the link changes.

**Constraints**:
- The live loop (`crates/finder/src/live.rs`), the tracker, 038's push rules, the solver,
  detection, the databases, the scope node, the simulator and the page are not changed.
- `crates/finder/src/session.rs` changes in `record_frame_sync` alone.
- 052 owns `crates/star-detection/` and `crates/scope-node/src/detect.rs`; 054 owns the
  imaging run's focus and run code and `ImagingPage.tsx`. This spec touches none of them.
- `RUSTFLAGS="-D warnings"`, `--all-features` clippy, and rustdoc with warnings denied.
- No test uses a camera, a mount, ASTAP or the network.

**Scale/Scope**: three stories; three changed library files (`model.rs`, `goto.rs`,
`session.rs`), four new test files, three `mod` lines, the README's GoTo and second-sync
paragraphs and one new paragraph, and two glossary entries.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Detection and the plate solver are untouched; the model reads the solve the live loop gives it, as today. The field check (FR-007) reads the solved field against the database's own stated range, after the solver and 047's choice have run, and refuses only the sync. |
| II. Never weaken a gate | No golden, fixture, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS`, `ergane.yaml` or existing test is edited; the only edits to existing test files are three `mod` lines (SC-002). Every other test of the workspace passed unchanged on the probe's tip (§ What was run). |
| III. Tests first, independent expectations | The oracle of every Rust scenario is a mount written fresh in each test file (zero points, in the model tests CA and NPAE, then Rodrigues' rotation by the base tilt), the chord distance written in the test, the catalog's M42, the simulator's true eyepiece direction, the test database's stated range, and a frame answered at 0.7 times its solved field. No test calls `finder::model::separation` or the fit for an expected value, or reads a constant of the code; `predict` is called only to measure the model's miss, and the support rig's `finish_with` and `error_arcsec` call `separation` only to measure the Sun distance and the true error, as in every GoTo test. Time comes from a paused tokio clock and the simulator's virtual clock. Each story's tests fail before it (§ What was run), but US1-S2, US1-S6, US1-S7 and US1-S8, which keep today's behaviour (a slip for a tilt over 6°, 038's mark, the current epoch, the short way round) and each bite under a mutation of the story's code (§ Mutations). |
| IV. Read the critical paths twice | None is touched: no `Cargo.toml` or `Cargo.lock` change, no `unsafe`, nothing under `proto/`, `crates/math-core`, `crates/pattern-database` or `crates/database-generation`, no `build.rs` or `.cargo/`. |
| V. Proven open source first, with honest licensing | Looked for, in local sources only (no download): the repository's survey of 2026-10-21 (`docs/research/2026-10-21-pointing-model-and-tracking.md:18`, § 1 and § 2), with its sources on TPoint, Taki's matrix method and INDI's alignment subsystem, Cedar, PiFinder, StarSense, N.I.N.A., Ekos, SharpCap, ASIAIR and SynScan; the vendored `reference-solutions/` (cedar-detect, cedar-solve); and the local crate cache. **TPoint** is the model's own source (IA, IE, AN, AW and the fixed terms) and a desktop program fitted from a list of observations by an operator, with no live intake to adopt. **INDI's alignment subsystem** (Taki's method) is a pure rotation that the survey finds leaves 32′ worst case for an IE of 0.25° (`docs/research/2026-10-21-pointing-model-and-tracking.md:59`), and a C++ library of a server this console does not run; not adopted. **N.I.N.A., Ekos and SharpCap** loop solve, sync and re-slew (`docs/research/2026-10-21-pointing-model-and-tracking.md:88`, `docs/research/2026-10-21-pointing-model-and-tracking.md:92`, `docs/research/2026-10-21-pointing-model-and-tracking.md:95`); the survey reads no rule in them for refusing a sync; their loop is already 014's. Solve-and-sync tools hold a solve to the configured image scale (general knowledge, not in the survey), which is FR-007. **SynScan**'s alignment advice, 15° to 60° up and 45° to 135° apart (`docs/research/2026-10-21-pointing-model-and-tracking.md:101`), is the source of the band; it is advice, not code. **cedar-solve** (tetra3) holds a match to a false-alarm probability, already applied (`crates/finder/src/solve.rs:26`), and has no pointing model. **The local crate cache** holds no pointing-model, alignment or robust-fitting crate. So the rule is written here: a dozen lines on the model's own fit, no number of its own but the 6° bound, the band and the 5 %. |

## Project Structure

```text
crates/finder/
├── README.md                                   # US3: GoTo and second-sync paragraphs, a new paragraph
├── src/
│   ├── model.rs                                # US1: the tilt-spot rule, axis pairs
│   ├── goto.rs                                 # US2: taken solves only, frame_id/frame_time, the reasons, the band
│   └── session.rs                              # US2: record_frame_sync's field check
└── tests/
    ├── motion/model/tilt.rs                    # US1, new
    ├── motion/model/mod.rs                     # US1: one mod line
    ├── motion/goto/tilt.rs                     # US2, new
    ├── motion/goto/field.rs                    # US2, new
    ├── motion/goto/mod.rs                      # US2: two mod lines
    ├── console/model_docs.rs                   # US3, new
    └── console/main.rs                         # US3: one mod line
GLOSSARY.md                                     # US3: Base tilt, Slip
```

## Interfaces

The code as it ran on `probe/053` (tip 764a47c0) and as probe round 1 rebuilt it from this
text on `probe/053-r1`, with this fold's commits (tip 8fac1157; § What was run lists the
commits). Bodies not given here are the builder's, to these words and the FRs.

### `crates/finder/src/model.rs` (US1)

```rust
/// The largest base tilt a second sync may fit before it is read as a slip, in
/// degrees (053).
const MAX_BASE_TILT_DEG: f64 = 6.0;

impl Sync {
    /// The azimuth axis's own direction in degrees, which no model error moves (053).
    pub(crate) fn axis_az_deg(&self) -> f64 {
        self.axis[0] * 360.0 / f64::from(self.counts_per_revolution[0])
    }
}
```

`add_sync` keeps its validity checks (`crates/finder/src/model.rs:493` to
`crates/finder/src/model.rs:507`) and then, in place of today's slip test
(`crates/finder/src/model.rs:508`):

```rust
let slip = self.slip_pending || !self.fits(&sync);
self.take(sync, slip);
true
```

038's mark is read first: with `slip_pending` set, the sync opens an epoch, at a tilt spot
too, whatever tilt it would fit (FR-001, US1-S6).

- `fn fits(&self, sync: &Sync) -> bool`: true when `predict` gives nothing (the night's
  first sync); before the base tilt is fitted, when `tilt_spot(sync)`, whether a clone
  that takes the sync (`take(sync.clone(), false)`) holds a base tilt of at most
  `MAX_BASE_TILT_DEG`, whatever the distance; otherwise whether the sync lies within
  0.5° of the prediction once the base tilt is fitted and 2.5° before, as today.
- `fn tilt_spot(&self, sync: &Sync) -> bool`: a sync of the current epoch
  (`self.epochs.len() - 1`) whose `axis_az_deg()` differs from the sync's by at least
  90°, through `wrap`.
- `fn take(&mut self, sync: Sync, slip: bool)`: today's body from the epoch push
  (`crates/finder/src/model.rs:515`) to the end of `add_sync`, less its final `true`, the
  base-tilt test reading `axis_az_deg()` in place of `az_deg`
  (`crates/finder/src/model.rs:535`).
- The doc of `Model::base_tilt` reads "Absent until two syncs in an epoch are at least
  90° apart on the azimuth axis."

### `crates/finder/src/session.rs` (US2)

In `record_frame_sync` (`crates/finder/src/session.rs:1507`), the lock that reads the
aim, the site and the counts also reads `values.field`'s range (the database in use's,
which `use_database` refreshed after the frame's tries), and after the site and counts
test:

```rust
// A solve of a field the database in use was not built for is no sync (053).
if range.is_some_and(|(min, max)| !(0.95 * min..=1.05 * max).contains(&solved.fov_deg())) {
    return Ok(false);
}
```

### `crates/finder/src/goto.rs` (US2)

```rust
/// The second sync's altitude band, degrees (053): searched first.
const SECOND_SYNC_BAND: [f64; 2] = [30.0, 60.0];

pub struct Progress {
    // … every field as today, then:
    /// The star frame whose solve, taken as a sync, gave `error_arcsec` or ended
    /// the second sync (053).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame_id: Option<u64>,
    /// That frame's mid-exposure, console Unix seconds (053).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame_time: Option<f64>,
}
```

- `State` gains `frame: Option<(u64, f64)>`, set to `None` where a GoTo or second sync
  starts and copied into every `Progress` that `update` writes.
- `Planner::second_sync` (`crates/finder/src/goto.rs:265`) keeps today's clearance, cone,
  sky test and altitudes (`crates/finder/src/goto.rs:280`), and sorts the spots with those
  inside the band first, 30° and 60° themselves inside, then by today's distance. Today's
  altitudes already hold 30° and 60° (every 0.5° at or above the minimum altitude), so
  holding the first altitude tried to the band would change no spot inside it.
- The settle loop (`crates/finder/src/goto.rs:955` to `crates/finder/src/goto.rs:1011`):
  after a report's pointing matches its frame (`crates/finder/src/goto.rs:982`), when no
  sync of `self.session.model(report.time)` has the report's time, it records the
  report's frame id as passed over and goes on to the next report; otherwise it sets
  `State::frame` to the report's frame id and time and returns the error (a GoTo) or
  nothing (a second sync), as today. On the 30 s timeout it fails with
  `format!("frame {frame} solved but was not taken as a sync")` when a report was passed
  over, and `"no frame solved after settling"` when none was.
- After the solve is taken, a second sync (no target) whose session model, read at the
  console's time, holds no base tilt returns
  `Err("taken as a slip: no base tilt fitted".into())`, which `run` reports as `failed`.

### The README (`crates/finder/README.md`, US3)

In the GoTo paragraph (`crates/finder/README.md:577`), "then uses a solved exposure
starting after that wait as a sync." becomes FR-012's first sentence; the `goto` keys
become FR-012's second sentence, after "`GET /api/session` adds"; and after the
paragraph's last sentence, which ends "is pending.", FR-012's third sentence. In the
second-sync paragraph (`crates/finder/README.md:591`), after its first sentence, which
ends "before the base tilt is fitted.", FR-012's fourth sentence; after "and takes the
sync, with no correction slews.", FR-012's fifth sentence. After that paragraph, a
new one:

> Before the base tilt is fitted, a sync at least 90° on the azimuth axis from a sync
> of the current zero-point epoch joins that epoch and fits the base tilt when the tilt
> it fits is at most 6°, and is otherwise taken as a slip; the tilt alone decides it.
> Any other sync more than 2.5° from the model's prediction, or 0.5° once the base tilt
> is fitted, opens a new zero-point epoch, as does the first solve after a push by hand
> while Track runs, at a tilt spot too. A live solve whose field lies more than 5% outside
> the range of the database in use is not taken as a sync. Two syncs fit the base tilt
> and the zero points exactly, so a slip between them is taken as tilt: take the first
> sync between 30° and 60° up when the sky allows, and press the second sync at once,
> with the telescope untouched, before the first far GoTo. If a tree blocks the second
> sync's spot, a GoTo to a clear target at least 90° round on the azimuth axis fits the
> base tilt by the same rule.

### The glossary (`GLOSSARY.md`, US3)

- **Base tilt**: "How far, and toward which azimuth, the mount's base is off level
  (TPoint's AN and AW). Fitted once two syncs of one zero-point epoch lie at least 90°
  apart on the azimuth axis, up to 6°." (_Avoid_ unchanged.)
- **Slip**: "A sync far enough from the model's prediction that the axes must have moved
  without their readings following, as after a push by hand or a power cycle. Before the
  base tilt is fitted, a sync a quarter turn round on the azimuth axis is one only when
  the tilt it fits is over 6°." (_Avoid_ unchanged.)

## What was run and what was read

Logs are under the operator's probe folder, not in the repository; the draft's are
numbered 00 to 13, the fold's `fold-r1-01` to `fold-r1-18`, probe round 1's in its own
folder, and its fold's `fold-probe-r1-01` to `fold-probe-r1-13`.

**The commits**, on `probe/053`, cut from f6d4f5fd. The draft's: 2bee1989 and 8570c83f
(measurements on `main`'s code, not stories: `probe_tilts.rs` and a `Rig::with_mount`
helper in `crates/finder/tests/support/goto.rs`), 0bdd2ddf and 5b40ab27 (the draft's
stories with the hold). The fold's: 1ef0d298 (US1, the hold reverted), 1fe5c824 (US2),
ac2dba1a (US3), 3cfa6408 (US2's formatting), f0d5e17e and bcc52b51 (measurements, not
stories), 204f61c3 and 764a47c0 (US1-S1 in a second direction and US1-S5 a 5° slip, so
that FR-002's mutations and `main` fail them), c4d36f72 (US2: the planner's axis rule
removed, § Mutations) and 3f3165cc (US3: its words). The tip is 764a47c0.

**Probe round 1, from this text alone**, on `probe/053-r1`, cut from c9a29b2f (this trio
on f6d4f5fd): ec3162f3 and 42eec24a (US1, its tests and then the story), ab7f972a and
b4962257 (US2), f4087b9a and a78a3f7a (US3). Every story built from the text: 0 blocking
and 8 clarity defects, each folded here. **Its fold**, on top: ea235dfb (US1-S6 to
US1-S8), 3d2fc4e8 (US2: the band's clamp removed, and US2-S1's and US2-S5's new steps)
and 8fac1157 (US3: FR-012's fourth and seventh sentences). The tip is 8fac1157. Story
sizes: § Plan traps 2.

**Run:**

- **The field night, reproduced** (log 02, on `main`'s code): the tilt rig at
  (75°, 122°), tilts 0° to 5° toward 195° and 300°, the second sync and then GoTos to
  (36°, 127°) and (40°, 250°). At 0° and 1° the second sync fitted the tilt in one epoch
  and the first errors were 51″ to 62″. At 2° to 5° the second sync opened a second
  epoch and no tilt was fitted, and the first errors were 7,533″ to 45,412″; each GoTo
  still ended `done` after two or three corrections, and each opened further epochs (3
  or 4). A 3° tilt toward 195° gave 17,196″ to (36°, 127°), against the field night's
  17,960″ to M42.
- **A Python model of the two-sync fit** (log 01, outside the repository): the CA and
  NPAE rows of spec.md § Clarifications (why 30° to 60°).
- **The axis trials** (logs 03, 04): spec.md § Clarifications (why the azimuth axis).
- **The unmodelled terms on a written truth** (log fold-r1-01; US1-S4's truth): the rows
  of spec.md § Clarifications, the same as the review's run U on the model's code to
  0.01° for every single term.
- **The checks before a two-sync tilt is trusted** (log fold-r1-01): five pairs of syncs
  (first 77° up with the second at 47°, 62° or 81°; first at (30°, 40°) with the second
  at (140°, 50°); first at (300°, 45°) with the second at (40°, 55°)), each true with CA
  and NPAE of 0, 0.25° and 0.5°, and with slips of 1°, 2°, 3°, 5°, 8°, −3° in azimuth,
  1°, 3°, 5° in altitude and (−5°, 2°); the roll of each sync by finite construction of
  the tube's up, plus a fixed camera rotation of 7°. Results: spec.md owner decision 3,
  part 6. Every slipped pair was taken in one epoch but one (5° in altitude with the
  second sync at 81°, which fitted over 6°).
- **The roll check on the simulator** (log fold-r1-02): the tilt rig at (75°, 122°),
  tilts 0°, 1°, 3°, 5°, toward 195° and 300°: the two solves' roll difference stayed
  0.36° to 0.55°, the fitted model's was 0.00° level and 2.04° to 22.12° tilted.
- **Run P on this build** (log fold-r1-04): the same false field (another place's real
  stars) over 2 or 6 frames after the GoTo's slew: epochs 1 to 3, syncs 2 to 5, errors
  101,828″ then 114,453″ then 10″, `done` after two corrections, the same numbers as the
  review's run P on the hold; two alternating false fields over 4 frames: epochs 1 to 4,
  errors 200,538″ and 227,544″, then 45″, `done`.
- **The tree remedy** (log fold-r1-05): spec.md owner decision 3, part 3.
- **The story tests** (logs fold-r1-03 on bcc52b51, fold-r1-15 after the axis rule's
  removal): the 15 new motion tests, the five of `goto::second_sync` and US3's test
  passed.
- **Each story's tests fail before it** (log fold-r1-13): US1's file on `main`'s code,
  US2's files on US1's, US3's file on US2's: § Each story before it.
- **One mutation per changed FR** (log fold-r1-14): § Mutations.
- **The gates on the tip** (logs fold-r1-16 to fold-r1-18 on 764a47c0, and fold-r1-10
  to fold-r1-12 on bcc52b51 before the last four commits), with the gates'
  own `RUSTFLAGS` (§ Verification): `lint` passed (`cargo fmt --check`, clippy with
  `-D warnings`, both `cargo doc` runs); nextest 1,541 of 1,541, the probe's own ten
  measurement tests (`probe_tilts`) left out, and every doc test passed; the page half,
  though the page is untouched, since the browser tests drive this console:
  `npm ci --offline`, the typecheck, Prettier, Vitest (607 of 607), the build, the dist
  check (clean) and `CI=1 npm run e2e` (34 of 34) with the factory's own browsers, the
  same on both tips.
- **Probe round 1, from this text** (its own logs, on a78a3f7a): each story's tests
  failed before it as § Each story before it says and passed on three runs after; its
  mutations found the unpinned choices this fold pins (§ Mutations, § Read from the diff);
  every gate passed, nextest 1,541 of 1,541, Vitest 607 of 607, e2e 34 of 34.
- **The fold's new steps** (logs fold-probe-r1-01, -04 and -07): US1-S6 to US1-S8, US2-S1's
  and US2-S5's new clauses, US3-S1's new words, and the five tests of `goto::second_sync`,
  passed; each new step's code mutated once (logs fold-probe-r1-02, -05 and -08):
  § Mutations; each story's tests on the code before it (logs fold-probe-r1-03, -06 and
  -09): § Each story before it.
- **The gates on the tip 8fac1157** (logs fold-probe-r1-10 to -12), as on 764a47c0:
  `lint` passed; nextest 1,544 of 1,544 (round 1's 1,541 and US1's three new tests) and
  every doc test passed; `npm ci --offline`, the typecheck, Prettier, Vitest (607 of 607),
  the build, the dist check (clean) and `CI=1 npm run e2e` (34 of 34). `cargo deny
  --offline check` (log fold-probe-r1-13), not a gate: advisories, bans, licenses and
  sources ok, on its local copy of the advisories.

**Read, not run:**

- The real mount's terms beyond the simulator's: only the written truth and the Python
  model above.
- A moved aim point after a tap (032), and a console restart: unchanged paths.
- MCP's `get_session` carrying `goto.frame_id` and `goto.frame_time`: it returns
  `api::session_response` (`crates/finder/src/mcp/tools.rs:538`,
  `crates/finder/src/api.rs:884`), the view the session tests read.
- That the field night's far reading was a true solve through a wrong model, not a false
  match: the operator seat's triage, outside the repository.

## Read from the diff, not from a test

Each passes every scenario built the other way; the judge reads it in the diff.

- FR-001's "at most 6°" at exactly 6° (`<` in place of `<=`): no fit lands on it.
- FR-001's tilt spot whose taking fits no tilt at all, its partner thinned away (a quarter
  turn above about 88.6° up): taken as a slip, as § Interfaces' `fits` reads it.
- FR-004's reading of the session's model at the report's time, not the console's.
- FR-007's 5 % at the upper end: Scale's frames lie below the range.
- FR-008's "only when none of those has a safe path, the nearest outside it": no scenario
  blocks the band.
- FR-011's paragraphs beyond FR-012's sentences.

## Shapes that existing tests pin

- **The 0.5° slip test.** `crates/finder/src/model.rs:898`'s
  `slips_refit_only_epoch_zero_points_at_the_half_degree_threshold` adds a 0.6° sync once
  the base tilt is fitted and asserts a second epoch at once: unchanged, since FR-001 acts
  only before the base tilt is fitted.
- **The second sync on the sky.** `goto::second_sync::one_tap_slews_takes_a_sync_and_fits_the_base_tilt_without_corrections`
  asserts the second sync 90° from the first on the sky: today's sky rule stays.
- **Hand pushes and slips.** `crates/finder/tests/motion/goto/sim.rs:159`'s
  `hand_push_opens_an_epoch_and_goto_still_converges`,
  `goto::sim::repeated_half_degree_slips_fail_after_five_corrections_with_error`,
  `night::a_whole_night_syncs_gotos_hand_push_stop_and_dropped_link` and the
  `steering_*` and `tracking::push*` tests pass unchanged (run).
- **A mount reset.** `crates/finder/tests/motion/mount_identity.rs` passes unchanged
  (run).
- **The databases.** 047's tests of the database in use, including a field that lies in
  no range, pass unchanged: they read the choice, not a sync (run).
- **The session's JSON.** `goto.frame_id` and `goto.frame_time` are absent before a sync
  is taken, and no test compares the `goto` object whole (read); the page's types
  (`crates/finder/web/src/api.ts:854`) ignore the new keys (run: Vitest and e2e).
- **The imaging run's GoTo.** The `imaging_run*` tests pass unchanged (run).

## Decisions and the alternatives rejected

The six owner decisions, each answered with its recommendation on 2026-11-27, are in
spec.md § Owner decisions. Below them:

| Decision | Chosen | Rejected |
|---|---|---|
| Where the tilt rule lives | `Model::add_sync`, which every sync passes through (the live loop, the second sync, the GoTo) | the second sync's route alone (a GoTo serving as the tilt measurement would still slip) |
| What decides at a tilt spot | the tilt alone (US1-S3) | the tilt or the 2.5° bound (a 7° tilt low on its hinge lies within 2.5°) |
| How far apart for the tilt | 90° on the azimuth axis, within an epoch | 90° in solved azimuth, today's (a high first sync puts it under 90°: log 03) |
| The second sync's spot | today's sky rule, the band first | an axis rule in the planner too (before the tilt is fitted the model is offsets and small fixed terms, so the sky rule with its 2.5° allowance already puts the spot at least 90° on the axis from each sync: § Mutations); the band only (no 90° at all) |
| Which syncs the sky rule reads | every epoch's, as today (`goto::second_sync::impossible_azimuth_spread_is_refused` pins the refusal) | the current epoch's alone (the base tilt needs its pair within one epoch, but a night of spread slip epochs is rare and the refusal is today's; a later spec if the owner wants it) |
| Where the field check lives | `record_frame_sync`, the sync only | the live loop (no pointing either: a change to 047's lens-change path) |
| What a GoTo corrects from | a solve the model took | the first solve after the settle, today's (a refused solve's pointing would steer the error) |
| A second sync with no tilt | `failed`, its reason | `done` with the reason (the page reads "On target") |
| The story cut | three stories, US2 the largest at 43.2 % | four (the field check apart: a rig copied twice) |

## Mutations

One mutation of each changed FR's code, each run alone on the new tests (log
fold-r1-14: on bcc52b51, and FR-001's, FR-002's and the README's again on 764a47c0;
probe round 1's on a78a3f7a; its fold's, logs fold-probe-r1-02, -05 and -08, on the
fold's commits); the scenarios named fail, and every other new test passes:

- FR-001: the 6° bound raised to 9°: US1-S2, US1-S3, US2-S3. The tilt or the 2.5° bound
  deciding at a tilt spot: US1-S3. The tilt-spot rule removed (`main`'s intake): US1-S1,
  US1-S3, US1-S4, US1-S5 and US2-S1 at 2° to 5°. 038's mark ignored at a tilt spot:
  US1-S6. The tilt spot read from a sync of any epoch: US1-S7. The tilt spot read
  without `wrap`: US1-S7, US1-S8.
- FR-002: the base-tilt pairs read in solved azimuth, and, apart, the tilt spot read in
  solved azimuth: US1-S1 (5° toward 15°). Neither bit US1-S1 in one direction, whose
  syncs lay over 90° apart in solved azimuth as well, nor any GoTo scenario, since the
  sky rule puts the second sync over 90° from the first in solved azimuth on those rigs;
  so US1-S1 has a second direction. The base-tilt pairs read across epochs: US2-S3 (the
  live loop's next sync at the slipped spot joins the new epoch and, paired with the
  first sync, fits a tilt).
- FR-003, the 0.5° and 2.5° bounds swapped: US1-S7, US1-S8, and the existing
  `slips_refit_only_epoch_zero_points_at_the_half_degree_threshold`,
  `fixed_calibration_uses_fresh_syncs_after_reset_and_spans_epochs` and
  `goto::sim::hand_push_opens_an_epoch_and_goto_still_converges` (probe round 1).
- FR-004, every solve taken: US2-S4, US2-S5. A 300 s wait in place of the 30 s: US2-S5.
- FR-005, `State::frame` never set: US2-S1 (all six), US2-S4. Not reset where a GoTo
  starts: US2-S1 (all six).
- FR-006, today's reason on the timeout: US2-S5. The first report passed over named, not
  the last: US2-S5.
- FR-007, the field check removed: US2-S4, US2-S5.
- FR-008, the band-first sort removed: US2-S2. Round 1's build also held the first
  altitude tried to the band; with that alone removed every US2 test passed, since
  today's altitudes already hold the band's, so 3d2fc4e8 removed it (§ Interfaces).
- FR-009, the plain end removed: US2-S3.
- FR-012, "at most 6°" made "at most 7°" in the README: US3-S1. The fourth sentence with
  "starting from the current altitude held to that band" back, and the seventh without
  "at a tilt spot too": US3-S1.
- **The planner's axis rule** (review minor 17), removed: no new test failed, nor any of
  `goto::second_sync`, and the four spot sweeps (64 second syncs: tilts of 1°, 3°, 5°
  and 8°, eight directions each, from 45°, 75° and 82° up) gave the same spot, epochs and
  tilt, line for line, with and without it. Before the base tilt is fitted the model
  maps the axes to the sky by offsets and the fixed terms, which move it by far less
  than the allowance, so the sky rule's 90° and its 2.5° allowance already put the spot
  90° round on the axis; it is gone from the planner (c4d36f72)
  and kept in the model, which reads the axis itself (FR-002).

## Each story before it

Each story's new tests on the code before it, the earlier stories' code in place (log
fold-r1-13, on 764a47c0; probe round 1, as T001, T005 and T010 say; and its fold's, logs
fold-probe-r1-03, -06 and -09, on 8fac1157's test files):

- **US1** on `main`'s `model.rs`, `goto.rs` and `session.rs`: US1-S1, US1-S3, US1-S4 and
  US1-S5 fail; US1-S2 passes, since `main` already takes the 8° second sync as a slip,
  and bites under FR-001's 9° mutation; US1-S6, US1-S7 and US1-S8 pass, since `main`
  already takes 038's mark at once, opens no epoch at a sync near the newest epoch's and
  measures nothing the long way round, and each bites under its mutation (§ Mutations).
- **US2** on US1's code (`goto.rs` and `session.rs` at f6d4f5fd): all ten fail: US2-S1's
  six and US2-S4 on `frame_id` absent, US2-S2 on the altitude, US2-S3 and US2-S5 on a
  `done` where they assert `failed`.
- **US3** on US2's code (the README and the glossary at f6d4f5fd): US3-S1 fails.

## User Story 1 - The pointing model

The tilt-spot rule in `Model::add_sync`, `Sync::axis_az_deg`, the axis pairs, and the
model's test file.

## User Story 2 - The second sync, the field and GoTo

The band, the plain end, the field check in `record_frame_sync`, the settle loop's wait
for a taken solve, `frame_id` and `frame_time`, the reason after solves not taken, and
the two GoTo test files.

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
   on `main` at 01b2d0f5, 052 included.
2. **`main` is still as read.** Every citation here was read at f6d4f5fd. If `main`
   moves in a file § Project Structure names, or in a test § Shapes names, re-read them
   and re-run the validator. Met at the flip: `main` is 01b2d0f5, where 052 landed. Of
   the files named, 052 changed `crates/finder/README.md`, `GLOSSARY.md` and
   `crates/finder/tests/console/main.rs`; each was re-read there (the README's two
   citations moved by five lines), and the validator re-ran.
3. **The shared files are re-anchored.** 052, 054 and 055, drafted beside this spec, also
   write `crates/finder/README.md`, the console and motion test `main.rs` files and
   `GLOSSARY.md`. The four land in the order 052, 053, 054, 055, and the operator seat
   re-anchors each one's `shared_writes` on whatever has landed before its flip. Met at the
   flip: 052 landed at 01b2d0f5, and the record's entries for the README, the glossary and
   `crates/finder/tests/console/main.rs` name 052's US3 and US2.
4. **The owner decisions are settled and the owner approvals given** (spec § Owner
   decisions, § Owner approvals). Met: the owner answered all six on 2026-11-27, each with its
   recommendation, and the flip gives the approvals.
5. **Probe builds.** A probe round builds the stories from this text alone, and its
   defects are folded. Met: probe round 1 built all three from this text alone on
   `probe/053-r1` (tip a78a3f7a), 0 blocking and 8 clarity defects, each folded here and
   proved on the fold's commits (tip 8fac1157; § What was run).
6. **No operator-staged files are needed.** Met: no story reads a file outside the
   repository.

## Outside the factory

Written, not run. The owner's, on the next field night, on the alt-az mount with the
finder camera.

1. **After the first sync**, read `GET /api/session` (or MCP's `get_session`) and write
   down `model.epoch`, the altitude the telescope was at, and whether `model.base_tilt`
   is absent.
2. **Press the second sync at once**, the telescope untouched. When `goto.state` is
   `done` or `failed`, write down `goto.reason`, `goto.frame_id`, the session's `time`
   less `goto.frame_time`, the altitude it went to, `model.epoch` (expected unchanged),
   `model.base_tilt.deg` and `model.base_tilt.toward_deg`, and `model.rms_arcsec`.
3. **Two far GoTos.** Choose two targets more than 90° apart in azimuth and more than
   30° up, neither the second sync's place. For each, write down the first
   `goto.error_arcsec` after the slew, its `goto.frame_id`, the number of corrections,
   the end state and reason, and `model.epoch` after it.
4. **If a GoTo fails** with `frame <id> solved but was not taken as a sync`, write down
   `field.fov_deg` and `field.database`, and press GoTo again once.
5. **What would change the design**, in a follow-up spec: a second sync that fails as a
   slip on a base under 6° off level; a first error to either far target over 1,800″
   (half a degree) with the tilt fitted, which would point at a slip absorbed into it or
   at large unmodelled terms; or a field refused on a true solve.

## Plan traps

They bind every story.

1. **Gate evidence or nothing.** The judge credits only gate results and the story's
   diff. *Check:* every Then names a committed test the `test` gate runs.
2. **One evidence budget.** *Check:* each story's whole diff stays under 60 % of
   65,536 B (39,321 B), measured with `git diff <base> -- . | wc -c` from the previous
   story's tip. On probe round 1's code, built from this text, US1 from c9a29b2f and each
   later story from the one before: US1 13,781 B, US2 24,983 B, US3 9,665 B; with each
   fold commit's own diff added to its story (1,673 B, 3,310 B and 5,767 B): US1
   15,454 B (23.6 %), US2 28,293 B (43.2 %), US3 15,432 B (23.5 %). That sum is an upper
   bound: by path from c9a29b2f with the fold in place, 15,060 B, 25,312 B and 9,597 B.
   At the 1.35 times a factory build has run over its probe on earlier specs: US1 about
   20,900 B (31.8 %), US2 about 38,200 B (58.3 %), US3 about 20,800 B (31.8 %), each
   under the 60 % line, and far under the judge's 128 KiB evidence limit and its 256 KiB
   refusal (the validator's report).
3. **A spec fixed mid-epic needs a relaunch.** *Check:* no builder edits
   `specs/053-second-sync-fits-base-tilt/` or `.specify/`.
4. **An earlier story's test file is an existing test.** *Check:* US2 and US3 write no
   file of an earlier story's tests; US2's rig is its own, in `goto/tilt.rs`, and
   `goto/field.rs` uses it as `super::tilt::rig_with`.
5. **No support edit.** *Check:* nothing under `crates/finder/tests/support/` changes;
   US2's `rig_with` builds `crate::support::goto::Rig`'s public fields in its own file
   (the probe's `Rig::with_mount` was the probe's alone).
6. **The layout check reads module names.** `crates/finder/tests/layout/main.rs:155`
   allows lowercase letters and underscores only. *Check:* the new modules are `tilt`
   (twice), `field` and `model_docs`.
7. **An oracle copied from the code proves nothing.** *Check:* the tests write the
   mount's geometry, the chord and the bounds as literals; none calls the model's
   `separation` or fit for an expected value, or reads a constant of the code.
8. **A negative assertion passes when nothing happens.** *Check:* US2-S4's unchanged
   epochs stand beside the `field.fov_deg` under 9 read while it ran and the `frame_id`
   past the window; US2-S5's absent `error_arcsec` beside its reason; US1-S2's two
   epochs beside the prediction at the second sync; US2-S1's slewing reads without a
   `frame_id` beside its test's check that there is at least one, and the `frame_id` each
   GoTo then names.
9. **Paused time and the simulator's clock.** *Check:* every GoTo test runs under
   `start_paused = true` and every wait is bounded by `finish_with`'s 5,000 reads.
10. **nextest ends a test after 5 minutes** (`.config/nextest.toml:23`). *Check:* each
    of US2-S1's tilts is a test of its own, two directions each, as the probe's are.
11. **The live loop records first, GoTo reads after.** *Check:* the settle loop reads
    the session's model at the report's time after the pointing names the report's
    frame, as today's second sync does (`crates/finder/src/goto.rs:995`).
12. **The session lock is never held across an await.** *Check:* `record_frame_sync`
    reads the range inside its existing lock; `goto.rs` takes `self.session.model(…)`
    by value.
13. **Words: "finder camera", never "finder" alone,** in every message; the glossary's
    terms exactly (zero-point epoch, base tilt, slip, sync).

## Risks

| Risk | Mitigation |
|---|---|
| A slip during the second sync's own slew is taken as a wrong tilt, and kept: each later sync it misses by over 0.5° opens an epoch that refits only its zero points (`crates/finder/src/model.rs:566`); 3° to 8° slips leave worst misses of 1.4° to 9.4° (fold-r1-01, run S). | An accepted risk (owner decision 3, part 6); the README says to press the second sync at once, untouched; a GoTo's corrections take out the miss locally; a roll check is the recommended later spec. |
| The real mount's unmodelled terms leave more after two syncs than the simulator does: 0.5° of CA or NPAE leaves 0.71° to 1.04° worst with the second sync in the band, 0.5° of each 1.90° to 2.07° (log 01) or 1.50° to 1.86° (run U), with 36 % to 81 % of places over 0.5°. | The band roughly halves what a high second sync leaves; the GoTo corrects locally; the fixed terms are fitted once 20 syncs are spread; § Outside the factory step 5 says what would change the design. |
| A false solve of a right-sized field is taken and steers one correction the wrong way (run P: about 32°). | The false-alarm test makes it rare; the next true solve opens another epoch and the GoTo ends `done` (owner decision 2). |
| A true solve of a field outside the database's range by more than 5 % is refused as a sync. | 047 has then chosen the database whose range holds the field, so only a field in no database's range is refused; § Outside the factory step 4 records it. |
| A story grows past 60 % in the factory. | The largest, US2, is 43.2 % on probe round 1's code with its fold commit added (38.6 % by path) and about 58.3 % at the worst overrun seen. |
| A new RustSec advisory fails `cargo-deny` in the merge queue. | Not this spec's to answer: the advisory's ignore is the owner's. |

## Refinement Record

Drafted on 2026-11-27 by the plate-solver operator seat's spec session against `main` at
f6d4f5fd, and folded the same day from the cynical review's round 1 (0 blockers, 6
majors, 12 minors) and the seat's direction to re-centre the spec on the tilt fix. From
a probe on `probe/053` (the field night reproduced on the simulator's tilted mount, the
axis trials, the draft's hold built and then reverted, the checks before a two-sync tilt
measured on a written truth and on the simulator, run P on this build, the tree remedy,
the three stories built with their scenarios' tests, each story's tests on the code
before it, a mutation per changed FR, and every gate on the tip), and a reading of 014's
pointing model, GoTo and second sync, 013's live loop, 024's refusal after a reset, 038's
push rules, 047's databases, 032's and 050's uses of a sync, MCP's session tool, the
constitution, the glossary and the repository's pointing-model survey. Folded again from
probe round 1, the three stories built from this text alone on `probe/053-r1` (tip
a78a3f7a: 0 blocking, 8 clarity defects), each fold proved on that branch's commits
(tip 8fac1157: three new US1 steps, new clauses in US2-S1 and US2-S5, the band's clamp
removed, two README sentences, every gate). Writes come from the task text. Shared
writes name the spec story that last wrote the same file on `main`; within the spec,
each story's files written by an earlier one are ordered by `depends_on_merged`. 052,
054 and 055 write some of the same files and land around this spec; the seat re-anchored
these `shared_writes` on 052's landing, 01b2d0f5, before the flip (plan.md § Prerequisites 3).

The six owner decisions of spec.md are answered, each with its recommendation
(2026-11-27). This record, like the scenarios, is written to the recommendations. Its
`open_questions` are empty so that the validator reports the record's structure; the
decisions themselves are spec.md's. This record approves nothing; the owner flipped the
spec on 2026-11-27.

```yaml
version: 1
reviewed_revision: 01b2d0f5aa29b85b57c35482df55816120920d14
stories:
  US1:
    tests:
      - path: crates/finder/tests/motion/model/tilt.rs
        status: new
        disposition: add
        assertion: "model rig, syncs at axis (105, 77) at 0 s and (0, 45) at 120 s under zero points (17, -2), tilts 0, 1, 3 and 5 toward 195 and toward 15: at 5 toward 15 the geometry's two azimuths under 90 apart, both add_sync true, one epoch, base_tilt deg within 1 arcmin of the tilt and toward_deg within 1 arcmin of the direction above 0, predict at axis (250, 30) within 1 arcsec of the geometry"
        decision: "spec.md:US1-S1"
      - path: crates/finder/tests/motion/model/tilt.rs
        status: new
        disposition: add
        assertion: "model rig under 8 degrees toward 195, the syncs of US1-S1: both true, two epochs, no base tilt, predict at (0, 45) within 1 arcsec of the geometry"
        decision: "spec.md:US1-S2"
      - path: crates/finder/tests/motion/model/tilt.rs
        status: new
        disposition: add
        assertion: "model rig under 7 degrees toward 0, syncs at (120, 10) at 0 s and (210, 10) at 120 s: the test's own one-sync prediction of the second (axis plus the first sync's offsets) within 2.5 degrees of the geometry; both true, two epochs, no base tilt"
        decision: "spec.md:US1-S3"
      - path: crates/finder/tests/motion/model/tilt.rs
        status: new
        disposition: add
        assertion: "model rig under 3 degrees toward 195 with collimation 0.25 or non-perpendicularity 0.25, a sync at (105, 77) at 0 s and a second at 120 s at (0, 47) or (0, 81): both true, one epoch, base_tilt deg within 0.15 of 3, the worst miss over axis azimuths every 15 and altitudes 22 to 82 every 10 under 0.5 at (0, 47) and not under 0.5 at (0, 81)"
        decision: "spec.md:US1-S4"
      - path: crates/finder/tests/motion/model/tilt.rs
        status: new
        disposition: add
        assertion: "model rig under 1 degree toward 130, a sync at (30, 40) at 0 s and one at (140, 50) at 200 s under zero points (22, -2): the test's own one-sync prediction more than 2.5 degrees from the slipped geometry, both true, one epoch, a base tilt, the worst miss against the slipped geometry over 2 degrees"
        decision: "spec.md:US1-S5"
      - path: crates/finder/tests/motion/model/tilt.rs
        status: new
        disposition: add
        assertion: "model rig under 1 degree toward 130, a sync at (30, 40) at 0 s, Model::mark_slip, a sync at (140, 50) at 200 s: both true, two epochs, no base tilt"
        decision: "spec.md:US1-S6"
      - path: crates/finder/tests/motion/model/tilt.rs
        status: new
        disposition: add
        assertion: "model rig under 8 degrees toward 195, the syncs of US1-S2 and a third at (350, 45) at 240 s: the third true, still two epochs"
        decision: "spec.md:US1-S7"
      - path: crates/finder/tests/motion/model/tilt.rs
        status: new
        disposition: add
        assertion: "model rig under 1 degree toward 130, syncs at (350, 45) at 0 s and (20, 45) at 120 s: both true, one epoch, no base tilt"
        decision: "spec.md:US1-S8"
      - path: crates/finder/src/model.rs
        status: existing
        disposition: preserve
        assertion: "slips_refit_only_epoch_zero_points_at_the_half_degree_threshold: with the base tilt fitted, a 0.6 degree sync opens a second epoch at once, base tilt and fixed terms unchanged"
        decision: "plan.md:Shapes that existing tests pin"
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
      - "014's slip bounds 0.5 and 2.5 degrees away from a tilt spot, its thinning, its fit and its epochs, and 038's slip mark taken at once"
      - "session.json's shape"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test motion -E 'test(/^model::tilt::/)'"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs and the field night of plan.md § Outside the factory"
    writes:
      - crates/finder/src/model.rs
      - crates/finder/tests/motion/model/mod.rs
      - crates/finder/tests/motion/model/tilt.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/model.rs
        other_spec: 038-track-follows-a-push
        other_story: US1
        disposition: merge_order
        reason: "053 depends_on_landed 038, whose US1 last changed model.rs (ad09efeb)"
      - path: crates/finder/tests/motion/model/mod.rs
        other_spec: 027-finder-tests-by-system
        other_story: US2
        disposition: merge_order
        reason: "053 depends_on_landed 027, whose US2 last changed model/mod.rs (b8dceaa9)"
    open_questions: []
  US2:
    tests:
      - path: crates/finder/tests/motion/goto/tilt.rs
        status: new
        disposition: add
        assertion: "tilt rig at (75, 122), tilts 0, 1, 2, 3, 4 and 5 toward 195 and 300: POST /api/model/second-sync done, every read correction 0, two syncs, one epoch, base_tilt deg within 1 arcmin of the tilt; GoTos to (36, 127) and (40, 250) each done, first error_arcsec under 120, frame_id above the latest frame id and frame_time after the console time at its start; no slewing read at correction 0 with a frame_id; one epoch after both"
        decision: "spec.md:US2-S1"
      - path: crates/finder/tests/motion/goto/tilt.rs
        status: new
        disposition: add
        assertion: "level rig at (75, 122), (45, 122) and (20, 122): second sync done, true altitude within 0.2 of 60, 45 and 30, the azimuth axis turned at least 90 and the true azimuth changed at least 90, one epoch"
        decision: "spec.md:US2-S2"
      - path: crates/finder/tests/motion/goto/tilt.rs
        status: new
        disposition: add
        assertion: "tilt rig at (75, 122), 8 degrees toward 45: second sync failed with reason 'taken as a slip: no base tilt fitted', every read correction 0, two epochs, no base tilt"
        decision: "spec.md:US2-S3"
      - path: crates/finder/tests/motion/goto/field.rs
        status: new
        disposition: add
        assertion: "field rig (1 degree toward 130, zero points (120, 50), syncs at counts (4500, 4500) and (580000, 4500)), Scale's window the frames latest + 1 to latest + 4 at the first solving read: GoTo to M42 done, frame_id above latest + 4, the first error_arcsec read under 120, the final error_arcsec and Rig::error_arcsec each under 60, a Session::snapshot read in finish_with's sample with field.fov_deg under 9, epochs unchanged"
        decision: "spec.md:US2-S4"
      - path: crates/finder/tests/motion/goto/field.rs
        status: new
        disposition: add
        assertion: "field rig, Scale's window from the frame after the latest without an end at the first solving read: GoTo to M42 failed, reason 'frame <id> solved but was not taken as a sync' with the id at least the window's first frame and within 3 of the latest frame's id at the end, ended within 35 s of the first solving read, no error_arcsec, epochs unchanged"
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
      - "047's choice of the database in use; the live pointing of every solve"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test motion -E 'test(/^goto::tilt::/) | test(/^goto::field::/)'"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs and the field night of plan.md § Outside the factory"
    writes:
      - crates/finder/src/goto.rs
      - crates/finder/src/session.rs
      - crates/finder/tests/motion/goto/field.rs
      - crates/finder/tests/motion/goto/mod.rs
      - crates/finder/tests/motion/goto/tilt.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/goto.rs
        other_spec: 024-mount-link-recovery
        other_story: US3
        disposition: merge_order
        reason: "053 depends_on_landed 024, whose US3 last changed goto.rs (20839158)"
      - path: crates/finder/src/session.rs
        other_spec: 047-console-uses-the-database-that-fits
        other_story: US2
        disposition: merge_order
        reason: "053 depends_on_landed 047, whose US2 last changed session.rs (c15937e7)"
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
        assertion: "docs rig, crates/finder/README.md and GLOSSARY.md whitespace joined: the README holds FR-012's ten sentences and the glossary FR-013's two, word for word"
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
        other_spec: 052-node-keeps-saturated-stars
        other_story: US3
        disposition: merge_order
        reason: "053 depends_on_landed 052, whose US3 last changed the README (01b2d0f5)"
      - path: GLOSSARY.md
        other_spec: 052-node-keeps-saturated-stars
        other_story: US3
        disposition: merge_order
        reason: "053 depends_on_landed 052, whose US3 last changed the glossary (01b2d0f5)"
      - path: crates/finder/tests/console/main.rs
        other_spec: 052-node-keeps-saturated-stars
        other_story: US2
        disposition: merge_order
        reason: "053 depends_on_landed 052, whose US2 last changed console/main.rs (bf1fcd66)"
    open_questions: []
```
