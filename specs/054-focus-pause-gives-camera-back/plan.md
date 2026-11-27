# Implementation Plan: The Focus Pause Gives the Camera Back

**Branch**: `054-focus-pause-gives-camera-back` | **Date**: 2026-11-27 |
**Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/054-focus-pause-gives-camera-back/spec.md`

## Summary

The imaging run's focus pause closes 033's focus aid on every answer, and the aid puts
the camera's settings back once, right after it aborts its own exposure. A camera
still stopping that exposure refuses them, the refusal is kept only as the aid's
`restore_error`, and the run, holding the camera again, finds the aid's focus region
and ends `camera_failed` with 041's words. The fix is in the aid's one put-back: a
refusal while the camera is `ready` is tried again once a second for up to 5 s, and
041's refusal names a put-back that never took. Then the record keeps the altitude it
was read at (Track's target's, while Track is on), and a reading is judged against the
record carried to its own altitude by the airmass to the power 0.6, so the field
night's 6.7″ at 37° against 4.6″ at 74° reads `good`, not `clearly_out`. Last, the
Imaging page keeps its last picture until the next preview, the Focus block keeps its
readings while the aid looks for its region again, and both lines that compare with
the record name the altitudes.

Four stories: the put-back and its words; the record's altitude and the carry; judging
at Track's altitude; the page. Each is under 35 % of 65,536 B on the probe's commits
(§ Plan traps 1). Two general rules remove the side effects a narrower fix would bring
(spec.md § Owner decisions): **whatever the focus aid sets, it gives back before anyone
else holds the camera, or says why it could not; and a reading is compared with the
record at one altitude, the reading's own.**

## Technical Context

**Language/Version**: Rust, MSRV 1.96, stable in CI; TypeScript and React for the page,
built with Vite into the committed `crates/finder/web/dist`.

**Primary Dependencies**: none new. `tokio` (the put-back's pause), `serde` (the
optional `focus.json` fields), the console's own `sky::alt_az` (the altitude), Testing
Library and Vitest (the page).

**Storage**: `focus.json` in the imaging folder (033 FR-014) gains two optional fields;
its `format` stays `imaging-focus/1`.

**Testing**: `cargo nextest` (paused tokio time, the simulator and its virtual clock),
Vitest with `fetch` stubbed. No browser test is added; the existing Playwright suite
runs unchanged in the `test` gate.

**Target Platform**: the console (`finder`) and the page in a phone's browser.

**Project Type**: a Cargo workspace with a web page inside `crates/finder`.

**Performance Goals**: none new. The put-back waits at most about 5 s more than at
f6d4f5fd, only while the camera is ready and refusing.

**Constraints**: no change to the scope node, the scope link's protocol, 037's step
traits or 041's region refusal (decision 24 stands); no existing test edited; MCP's
tool descriptions unchanged (pinned by tests). Keep out of 052's files
(`crates/star-detection/`, `crates/scope-node/src/detect.rs`) and 053's
(`crates/finder/src/{model,goto,live,session}.rs`): the altitude reads
`SessionView` through `Session::snapshot`, and none of them is written.

**Scale/Scope**: four stories, about 69 KB of diff in all on the probe.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Detection and the plate solver are untouched. The focus reading is 033's (ASTAP's `-analyse` on the scope's Pi, the simulated solver in the gates), unchanged. The carry rests on seeing theory, a star's size in seeing growing as the airmass to the power 3/5, which no local source gives (V below): flagged as such in spec.md owner decision 2 and § Risks, and measured on the field night's own numbers (§ What was run). What rests on reading is listed in § What was run and what was read. |
| II. Never weaken a gate | No existing test, golden, fixture, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS` or `ergane.yaml` is edited. The only edits to existing test files are the `mod` lines of `crates/finder/tests/console/main.rs` (US1, US2) and `crates/finder/tests/motion/main.rs` (US3) (SC-002). The simulated imaging camera is f6d4f5fd's at its default settle of zero, so no existing caller sees a change (run: the 1,525 existing tests and the stories' 9 passed on the probe's tip, § What was run). 043's silent-node bound is kept by the rule that only a `ready` camera is tried again (the mutation that drops it fails 043's US1-S8 test). |
| III. Tests first, independent expectations | The oracles are the words of 041's refusal, of the simulator's refusal and of the scenarios; 031's own view of the settings and camera; the airmass written in each test as the secant of the zenith distance and the exponent 0.6 written there; 033's thresholds 1.10 and 1.30; the field night's numbers; the simulator's own altitude of the main telescope; and the views the page tests write. No test reads `RESTORE_PAUSE`, `RESTORE_WAIT` or `SEEING_EXPONENT`, or calls `carry` for an expected value. Time comes from a paused tokio clock and the simulator's virtual clock. Every story's tests fail before it (run, § What was run). |
| IV. Read the critical paths twice | None is touched: no `Cargo.toml` or `Cargo.lock` change, no `unsafe`, nothing under `proto/`, `crates/math-core`, `crates/pattern-database` or `crates/database-generation`, no `build.rs` or `.cargo/`. `crates/simulator` is not a critical path; its change is owner approval 1. |
| V. Proven open source first, with honest licensing | Looked for, in local sources only (no download). **For the put-back:** the INDI adapter's own waits (`crates/scope-node/src/imaging/indi/mod.rs:40` to `crates/scope-node/src/imaging/indi/mod.rs:45`), whose `RETRY_PAUSE` of 1 s before an exposure is tried again is the pattern the retry follows; no library is needed for a bounded retry of one call. **For the altitude:** the repository's survey of 2026-11-15, § 6 "Focus" (`docs/research/2026-11-15-imaging-camera.md:203`), with its sources on PHD2, SharpCap, Ekos and N.I.N.A.: each compares a reading with recent ones or the best since a reset (`docs/research/2026-11-15-imaging-camera.md:231`), and the survey says a reading cannot tell focus error from seeing (`docs/research/2026-11-15-imaging-camera.md:227`); none, as surveyed, carries a size across altitudes, and the survey gives no source for the airmass dependence. A search of the whole tree at f6d4f5fd, `reference-solutions/` (cedar-detect, cedar-solve) among it, finds no airmass anywhere. ASTAP's `-analyse`, installed as `astap_cli`, gives a half-flux diameter and a star count only. The console's own `sky::alt_az` (`crates/finder/src/sky.rs:258`) gives the altitude, so nothing is adopted for it. The exponent 3/5 is seeing theory's (the Fried parameter's dependence on the zenith distance), cited from no local source: the owner's decision 2 accepts or refuses it. **For the page:** nothing to adopt; the kept picture is page state. |

## Project Structure

```text
crates/simulator/src/imaging.rs                         US1: with_abort_settle, STILL_STOPPING
crates/finder/src/imaging/focus/run.rs                  US1: RESTORE_PAUSE, RESTORE_WAIT, put_back
                                                        US3: the altitude, the carried reference, the view's fields
crates/finder/src/imaging/run/console.rs                US1: the region refusal's second form
crates/finder/src/imaging/focus.rs                      US2: SEEING_EXPONENT, carry, Altitudes, note_at, reference_at, altitudes, focus.json fields
crates/finder/README.md                                 US1: ## Focus and ## An imaging run, the put-back
                                                        US3: ## Focus, the unit and the altitude; ## An imaging run, the carried check
crates/finder/tests/console/imaging_run_focus_restore.rs  US1, new
crates/finder/tests/console/imaging_focus_carry.rs        US2, new
crates/finder/tests/console/main.rs                     US1, US2: one mod line each
crates/finder/tests/motion/imaging_focus_altitude.rs    US3, new
crates/finder/tests/motion/main.rs                      US3: one mod line
crates/finder/web/src/api.ts                            US4: alt_deg, carried
crates/finder/web/src/components/ImagingPage.tsx        US4: the kept picture
crates/finder/web/src/components/FocusBlock.tsx         US4: readings while finding, the altitudes' words
crates/finder/web/src/components/FocusView.test.tsx     US4, new
crates/finder/web/dist/                                 US4: rebuilt
```

## Interfaces

### `crates/simulator/src/imaging.rs` (US1)

```rust
/// The words of a settings change refused while the camera is still stopping
/// an aborted exposure (054), as a real camera's driver refuses one.
pub const STILL_STOPPING: &str =
    "the camera refused the settings: it is still stopping an exposure";

impl SimImagingCamera {
    /// Refuse every settings change that names a setting, with
    /// [`STILL_STOPPING`], for `settle` after each abort of an exposure, as a
    /// real camera's driver can while it stops one (054). Zero, the default,
    /// refuses none.
    pub fn with_abort_settle(mut self, settle: Duration) -> Self;
}
```

`State` gains `refused_until: Option<tokio::time::Instant>`, `None` at start, and the
camera `abort_settle: Duration`, `Duration::ZERO` in its constructor. `abort`, when an
exposure is in flight, sets `refused_until` to now plus `abort_settle` unless it is
zero. `apply`, after its exposing check and before `check_settings`, answers
`Err(ImagingError::Failed(STILL_STOPPING.to_string()))` when the settings asked for
are not `ImagingSettings::default()` and `refused_until` is in the future.

### `crates/finder/src/imaging/focus/run.rs`, the put-back (US1)

```rust
/// A put-back the camera refuses while it is `ready` is tried again after
/// this long (054), as the scope node waits before it tries an exposure again.
pub const RESTORE_PAUSE: Duration = Duration::from_secs(1);
/// No put-back is tried later than this after the first (054).
pub const RESTORE_WAIT: Duration = Duration::from_secs(5);

/// Put the remembered settings back through the lease (FR-027; 054): a
/// refusal while the camera is `ready` is followed by another try
/// `RESTORE_PAUSE` later, so long as that try starts within `RESTORE_WAIT` of
/// the first. Answers the last refusal's words, or none once a try takes.
async fn put_back(imaging: &Arc<Imaging>, lease: &Lease, remembered: &Remembered) -> Option<String> {
    let first = tokio::time::Instant::now();
    loop {
        let refusal = match lease.set(remembered.edit()).await {
            Ok(_) => return None,
            Err(refusal) => refusal.message,
        };
        let ready = imaging.state.lock().expect("imaging state").state == "ready";
        if !ready || first.elapsed() + RESTORE_PAUSE > RESTORE_WAIT {
            return Some(refusal);
        }
        tokio::time::sleep(RESTORE_PAUSE).await;
    }
}
```

`finish` (`crates/finder/src/imaging/focus/run.rs:1047`) calls it in place of its one
`lease.set` (`crates/finder/src/imaging/focus/run.rs:1058`); everything after it, the
release and `run.restore_error` (`crates/finder/src/imaging/focus/run.rs:1072`), is
unchanged. `use std::time::Duration` joins the imports. No lock is held across an
await.

### `crates/finder/src/imaging/run/console.rs` (US1)

In `ConsoleCamera::hold` (`crates/finder/src/imaging/run/console.rs:358`), the region
refusal (`crates/finder/src/imaging/run/console.rs:378`) becomes:

```rust
// A region the focus aid could not take back is named as that (054).
let left = view.focus.and_then(|focus| focus.aid).and_then(|aid| aid.restore_error);
return Err(match left {
    Some(words) => format!("the focus aid could not put the imaging camera's settings back: {words}"),
    None => "the imaging camera has a region of interest set: clear it before an imaging run".into(),
});
```

The lease is given back on that path as before (the local `lease` is dropped).

### The README, US1's sentences (`crates/finder/README.md`)

In `## Focus`, the sentence ending `puts the settings back and releases the camera.` is
followed, in the same paragraph, by:

> A camera still stopping an aborted exposure can refuse the settings for a moment, so
> a put-back the camera refuses while it is ready is tried again once a second for up
> to 5 s. One that never takes is the aid's `restore_error`, and the camera keeps the
> aid's settings until they are changed by hand: `Use the whole sensor` on the Focus
> block, or `PUT /api/imaging/settings` with `{"roi":null}`.

In `## An imaging run`, the sentence ending `the page shows its time left.` is followed,
in the same paragraph, by:

> Every answer closes the aid, which puts the camera's settings back as `## Focus`
> says, and the run then holds the camera again. When the aid could not put them back,
> the run ends `camera_failed` with the camera's own words after
> `the focus aid could not put the imaging camera's settings back:`. Clear the region
> with `Use the whole sensor` and start again.

### `crates/finder/src/imaging/focus.rs` (US2)

```rust
/// Seeing widens a star as the airmass to this power (054): the record, read
/// at one altitude, is carried to a reading's altitude by it.
pub const SEEING_EXPONENT: f64 = 0.6;

/// The record's size `px`, read at `from_alt_deg`, as it would read at
/// `to_alt_deg` (054): times the ratio of the two airmasses, each the
/// secant of the zenith distance, to the power `SEEING_EXPONENT`.
pub fn carry(px: f64, from_alt_deg: f64, to_alt_deg: f64) -> f64 {
    let airmass = |alt: f64| 1.0 / alt.to_radians().sin();
    px * (airmass(to_alt_deg) / airmass(from_alt_deg)).powf(SEEING_EXPONENT)
}

/// The altitudes the two parts of the record were read at, when known (054).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Altitudes { pub tonight: Option<f64>, pub earlier: Option<f64> }
impl Altitudes { pub fn of(self, of: Of) -> Option<f64>; }

impl Kept {
    pub fn note(&mut self, px: f64, at: f64, scale: Option<f64>, passed: bool) -> bool; // = note_at(.., None)
    pub fn note_at(&mut self, px: f64, at: f64, scale: Option<f64>, passed: bool, alt_deg: Option<f64>) -> bool;
    pub fn reference_at(&self, now: f64, scale: Option<f64>) -> Option<(f64, Of, Option<f64>)>;
    pub fn altitudes(&self, now: f64, scale: Option<f64>) -> Altitudes;
}
```

`Kept` gains `tonight_alt` and `earlier_alt`, `None` in `new`, read in `describe` from
`StoredNight::alt_deg` and `StoredRecord::earlier_alt_deg`, both
`#[serde(default, skip_serializing_if = "Option::is_none")]`, and written by `save`
(`earlier_alt_deg` only while the earlier part exists). In `note_at`: the foreign rule
clears both with the record; a new night sets `tonight_alt = alt_deg`; at a rollover,
when the old night's record joins the earlier nights', `earlier_alt = tonight_alt`
before `tonight_alt = alt_deg`; a smaller reading of tonight sets `tonight_alt =
alt_deg`; a note that changes no size leaves both. `altitudes(now, scale)` answers
`Altitudes::default()` at a foreign scale; while tonight's night began at most
`NIGHT_S` ago, `{tonight: tonight_alt, earlier: earlier_alt}`; after it, `{tonight:
None, earlier: earlier_alt when the earlier record is at most tonight's size, else
tonight_alt}`, as `record` shows the parts; with no tonight, `{None, earlier_alt}`.
`reference_at` is `reference` with `altitudes(now, scale).of(of)`.

### `crates/finder/src/imaging/focus/run.rs`, the altitude (US3)

```rust
/// A size and when it was read (FR-029), and the altitude it was read at, when known (054).
pub struct SizeAt { pub value: f64, pub at: f64,
    #[serde(skip_serializing_if = "Option::is_none")] pub alt_deg: Option<f64> }

/// What a reading was judged against (FR-025): the record, carried to the
/// reading's altitude when both altitudes are known (054).
pub struct ReferenceView { pub value: f64, pub of: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")] pub carried: Option<CarriedView> }

/// The record a reference was carried from, and the two altitudes (054).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct CarriedView { pub from: f64, pub from_alt_deg: f64, pub alt_deg: f64 }
```

`Run` gains `carried: Option<(f64, f64, f64)>` (the record's own size in pixels, its
altitude, the reading's), `None` in `Run::new`, set and cleared beside `reference` in
each of `note`'s three arms. The steps loop computes `at` (the middle of the exposure,
as at f6d4f5fd) and `alt = altitude(imaging, at)` before `note(imaging, &read, frame.id,
at, alt)`:

```rust
/// The altitude, at `at`, of the place Track holds (054): while the session's
/// Track is on and a site is set, its target's altitude then; otherwise, and at
/// or below the horizon, none. The session is read with the imaging state's
/// lock released.
fn altitude(imaging: &Arc<Imaging>, at: f64) -> Option<f64> {
    let session = imaging.state.lock().expect("imaging state").session.clone()?;
    let view = session.0.snapshot(at);
    let track = view.track.filter(|track| track.state != "off")?;
    let site = view.site?;
    let (alt, _) = crate::sky::alt_az(track.target.ra_deg, track.target.dec_deg,
        &crate::sky::Observer { lat: site.lat_deg, lon: site.lon_deg, time: at });
    (alt > 0.0).then_some(alt)
}
```

`Held::session` is 032's `HeldSession` (`crates/finder/src/imaging/mod.rs:355`), whose
`Session::snapshot` (`crates/finder/src/session.rs:1400`) gives `track`
(`crates/finder/src/session.rs:660`) and `site` (`crates/finder/src/session.rs:674`).
In `note` (`crates/finder/src/imaging/focus/run.rs:963`), the reference
(`crates/finder/src/imaging/focus/run.rs:998`) becomes `kept.reference_at(now,
measured)`, carried when its altitude and `alt` are both known (`carried = (px,
from_alt, alt)`, the reference `(carry(px, from_alt, alt), of)`), and a settled reading
of the aid is noted with `kept.note_at(m, at, measured, found, alt)`. `view` fills
`SizeAt::alt_deg` from `kept.altitudes(now, measured)` and `ReferenceView::carried` from
`run.carried`, `from` times the shown scale `s` as every other size.

### The README, US3's sentences (`crates/finder/README.md`)

In `## Focus`, the sentence ending `beyond which its star sizes stop being
trustworthy.` is followed, in the same paragraph, by:

> Every size in the imaging view's `focus`, and in the imaging run's `focus`, is in the
> `unit` beside it: `arcsec`, or `px`, sensor pixels. At binning 1 and 0.55″ a sensor
> pixel, the far-out limit is about 11″.

After the paragraph that ends `never against a fixed number.`, a paragraph of its own:

> Seeing also grows with the air a star is seen through: a star's size grows as the
> airmass to the power 0.6, the airmass being the secant of the zenith distance. While
> Track is on and a site is set, each reading carries the altitude of Track's target at
> the middle of its exposure, and the record keeps the altitude of the reading that
> made it. A reading is then judged against the record carried to its own altitude: a
> record of 4.6″ read at 74° judges a reading at 37° as a record of 6.1″ would. With
> either altitude unknown (Track off, no site, or a record made before the console kept
> altitudes) the record judges as it is. The record's parts show their `alt_deg`, and a
> carried reference shows `carried`: the record's own size `from`, its `from_alt_deg`
> and the reading's `alt_deg`.

In `## An imaging run`, after US1's sentence ending `and start again.`:

> The check is judged against the record carried to the target's altitude, so a record
> made high in the sky does not call a low target's stars out of focus for the air
> alone.

### The page (`crates/finder/web/src/`, US4)

- `api.ts`: `ImagingFocusSized` gains `alt_deg?: number`; `ImagingFocusAid.reference`
  gains `carried?: { from: number; from_alt_deg: number; alt_deg: number }`.
- `ImagingPage.tsx`: a state `kept: { id: number; src: string } | null`, set during
  render whenever `frame?.preview` is set and differs from `kept.src`; `picture =
  frame?.preview != null ? { id: frame.id, src: frame.preview } : kept`. The picture's
  `div`, its `img` (`src={picture.src}`, `alt={`Imaging frame ${picture.id}`}`, the same
  classes) and the night's hold button (`crates/finder/web/src/components/ImagingPage.tsx:280`)
  show while `picture != null` in place of `frame?.preview`
  (`crates/finder/web/src/components/ImagingPage.tsx:271`).
- `FocusBlock.tsx`: `showReading = aid != null && (aid.state === "running" ||
  (aid.state === "finding" && aid.readings > 0) || done)`
  (`crates/finder/web/src/components/FocusBlock.tsx:184`); the record line writes each
  part through `sized(part) = part.alt_deg != null ? `${size(part.value)} at
  ${part.alt_deg.toFixed(0)}°` : size(part.value)`; `verdictWords`'s `where` appends ``,
  as at ${carried.alt_deg.toFixed(0)}° from ${size(carried.from)} at
  ${carried.from_alt_deg.toFixed(0)}°`` when `reference.carried` is set
  (`crates/finder/web/src/components/FocusBlock.tsx:83`).
- No colour, class or token is added: the night view's red-only rule holds as it is.

## What was run and what was read

All on `probe/054`, cut from f6d4f5fd, with the factory's Rust toolchain, `--locked`,
`npm ci --offline` and the factory's Playwright browsers; never pushed. Logs and
scripts in the operator's probe folder, outside the repository.

- **The repro (run).** 2bd35a98 adds the simulator's settle and the repro test. On
  main's console code (log 01), going on anyway mid-exposure with a settle of 1 s ends
  `camera_failed` with 041's words, `restore_error` REFUSED, the region the aid's tile
  (x 412, y 0, 412 × 274 at that moment). Fails before (log 18): with only the simulator
  knob and the test, all four of US1's tests fail.
- **The put-back (run).** 8ea29909: the retry and the named words. 1 s settle: the run
  stacks on the whole sensor with no `restore_error`; 10 s: `camera_failed` with the
  named words, in the view and through MCP (logs 02, 10, 13). The aid stopped alone
  answers at 1.000 s and 5.000 s of paused time (log 13).
- **Decision 1 (d), trial (run).** 5e5b5b84 (set aside in 2664feef): main's single
  put-back, and the run's focus part re-applying the settings it saw when it opened
  the aid. Once: the run ends `camera_failed` (the re-apply refused too); five tries a
  second apart: the run stacks, but the aid's `restore_error` stays in the view, and
  the aid stopped alone answers at once with its region kept (log 20).
- **The altitude (run).** 06c4521a: the record's altitude, the carry and the carried
  reference; 88956243, d49459ee and 6ca3d283: the README check, stronger oracles, and
  the README checks reading line breaks as spaces. In the motion rig M42 stands at
  about 42.96° when the check reads, Track's target's altitude within 0.5° of the
  simulator's own; a record of 2.0 px at 74° is carried to 2.458 px and a reading of
  2.826 px judged `slightly_soft`, `clearly_out` uncarried (logs 05, 17, 21). Fails
  before (log 18): US2's test binary does not build on main's `focus.rs` (no
  `focus::carry`, `Kept::note_at`, `Kept::reference_at`, `Kept::altitudes`); US3's three
  tests fail on US1's `focus/run.rs` and main's README.
- **The verdict candidates (run, log 04).** The field night's numbers under each
  candidate of decision 2, the secant and Kasten and Young's airmass: as spec.md owner
  decision 2 gives them. The carry factor is 1.324 from 74° to 37°, 1.859 from 74° to
  20° and 1.516 from 90° to 30°.
- **ASTAP (run, log 06).** The local `astap_cli` CLI-2025.09.29, `-analyse 30`, on
  frames made for the probe: noise, bright noise, daylight and flat frames give
  `HFD_MEDIAN=99.0` and `STARS=0`, exit 0; a star field `HFD_MEDIAN=3.9`, `STARS=40`;
  saturated and all-zero frames `HFD_MEDIAN=99.0` with `STARS=29721` and `STARS=29119`.
  The field's own ASTAP's output for a starless frame: not run (not on this machine).
- **The page (run).** e74a5f0c: the page, the README and the MCP reads. Vitest: three
  of the four new tests fail on main's page (log 09); all four pass after, and the
  whole suite, 611 of 611 (logs 08, 11). The page gate (typecheck, Prettier, Vitest,
  build, the dist check clean) and `CI=1 npm run e2e`, 34 of 34, on the probe's page,
  which no later commit changed (logs 15, 16).
- **The mutations (run, log 14).** Each caught: no retry (`RESTORE_WAIT` 0) fails
  US1-S1 and US1-S3; a retry whatever the camera's state fails 043's
  `a_focus_stop_on_a_silent_node_ends_and_reports_the_restore_failure`; the words
  unnamed fail US1-S2; the exponent 0.5 fails US2-S1; an altitude without Track fails
  US3-S2; no carry fails US3-S1.
- **The gates (run).** `lint` and the Rust half of `test` on 88956243 (log 19) and on
  the final tip, 6ca3d283 (log 23): `cargo fmt`, clippy, both `cargo doc`, nextest
  (1,534 of 1,534 each time: 1,525 existing and the stories' 9) and the doc tests.
- **Read, not run.** That the field camera refused the put-back (the field logs never
  read `restore_error`); the INDI adapter's behaviour after an abort beyond its code
  (`crates/scope-node/src/imaging/indi/mod.rs:1102`); decision 3 (b)'s effect on 031's
  view and its tests; the edge cases marked so in spec.md.

## Read from the diff, not from a test

- `finish` holds no lock across the put-back's await, and the release follows it (the
  judge reads `crates/finder/src/imaging/focus/run.rs`).
- `altitude` clones the session out of the imaging state's lock before
  `Session::snapshot`.
- `Kept::note` and `Kept::reference` keep their signatures; `FocusCheck`
  (`crates/finder/src/imaging/run.rs:320`) and the five step traits are not changed.
- The README's field checklist (`crates/finder/README.md:1759`) gains no step.
- `altitude` answers none at or below 0°: no rig puts Track's target under the horizon.
- A check notes no reading in the record, with or without an altitude (`if !run.check`
  around `note_at`, as around `note` at f6d4f5fd).
- `SimImagingCamera::apply` never refuses `ImagingSettings::default()`.
- No file of 052 or 053 is in any story's diff.

## Shapes that existing tests pin

- `crates/finder/tests/console/imaging_abort_silent.rs:372`: 043 US1-S8, a focus stop
  on a silent node ends `stopped` with `restore_error` `the scope node is not
  connected` within its bound: kept by the `ready` rule.
- `crates/finder/tests/console/imaging_focus_aid.rs:1024`: a lost link gives the aid a
  `restore_error`.
- `crates/finder/tests/motion/imaging_run.rs:313`: a start with the observer's region
  of interest keeps 041's words.
- `crates/finder/tests/console/imaging_run_focus.rs:505`: the run's `focus.record` is
  the aid's `reference.value`; with no altitudes in its rig, uncarried.
- `crates/finder/tests/console/imaging_focus_record.rs:61`: `Mark { px, at }` by
  literal, and `Kept::note` and `Kept::reference` as 033 gave them.
- `crates/finder/tests/mcp/imaging_focus.rs:242`, `crates/finder/tests/mcp/imaging_focus.rs:246`
  and `crates/finder/tests/mcp/imaging_run.rs:133`: the MCP tools' descriptions, word
  for word.
- `crates/finder/tests/console/imaging_run_docs.rs:95`: the field checklist's 18 steps.
- `crates/finder/web/src/components/FocusBlock.test.tsx`,
  `crates/finder/web/src/components/ImagingPage.test.tsx` and
  `crates/finder/web/e2e/imaging.spec.ts` (its night red-only check): unchanged and
  passing (run).
- No test builds `SizeAt` or `ReferenceView` by literal, so their new fields break none.

## Decisions and the alternatives rejected

| Decision | Chosen (recommended) | Rejected |
|---|---|---|
| 1. The put-back | (a) the aid's put-back retried while `ready`, 1 s apart, within 5 s; 041's refusal names a failed one | (b) retry only; (c) naming only; (d) the run re-applies its settings (trial-built); (e) the node waits after an abort |
| 2. The altitude | (a) the record keeps its altitude, carried by airmass^0.6 to the reading's, Track's target's | (b) a record per band; (c) a note only; (d) looser thresholds |
| 3. Between previews | (a) the page keeps its last picture, the Focus block its readings while finding | (b) `last_preview` in the view; (c) a Focus-block picture; (d) leave it |
| 4. Starless wording | (a) not here; capture the field's ASTAP output first | (b) read output with neither line as too few stars |
| 5. The unit | (a) a README sentence | (b) the MCP descriptions too (pinned by tests) |
| The retry's pause | 1 s, the INDI adapter's `RETRY_PAUSE` | a shorter pause: more calls to a camera that is stopping |
| The retry's bound | 5 s, 043's `ABORT_WAIT` | longer: a stop on a slow camera answers later still |
| The altitude's source | Track's target, from the session's site | the live solve or the mount (053's files), which Track already follows |
| The pause sentence | unchanged, naming the carried size as the record | a carry clause (needs a field on 037's `FocusCheck`) |

## User Story 1 - The put-back and its words

The simulator's settle, `put_back` with its two constants, the region refusal's second
form, the README's US1 sentences and their test file. No change to the record or the
verdict.

## User Story 2 - The record's altitude and the carry

`SEEING_EXPONENT`, `carry`, `Altitudes`, `Kept::note_at`, `reference_at`, `altitudes`
and the optional `focus.json` fields, with their test file. Nothing calls them yet but
the tests.

## User Story 3 - Judging at Track's altitude

`altitude`, the carried reference in `note`, `note_at` with the altitude, the view's
two fields, the README's US3 sentences and their test file, over US2's API.

## User Story 4 - The page

`api.ts`'s fields, the kept picture, the readings while finding, the altitudes' words,
the test file and the rebuilt `dist`.

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
2. **`main` is still as read.** Every citation here was read at f6d4f5fd. Not yet
   checked at a flip. If `main` moves in a file § Project Structure names, or in a test
   § Shapes names, re-read them and re-run the validator; 052 and 053, drafted beside
   this spec, write none of these files.
3. **The owner decisions are settled and the owner approvals given** (spec § Owner
   decisions, § Owner approvals). Not met: none answered.
4. **A cynical review and a probe round from this text alone**, their defects folded.
   Not met.
5. **The page half has run once.** Met on the probe: `CI=1 npm run e2e` 34 of 34.
6. **No operator-staged files are needed.** Met: no story reads a file outside the
   repository.

## Outside the factory

Written, not run. The owner's, on the next field night, with the imaging camera on the
main telescope, Track and the imaging run as on 2026-11-26, the console at this spec's
landing.

1. **Make a record high.** On a target well above 60°, with Track on, run the focus aid
   to a found focus. On the Focus block the record line now reads `Focus record: … at
   N° tonight.`; note the size and N.
2. **A run low.** Pick a target at least 25° lower and start an imaging run. At its
   focus check, note the verdict, the reading and the Focus block's verdict line, which
   now names `as at A° from S at F°`. If it pauses, note the pause sentence and whether
   the carried record it names is the record line's size carried from F° to A° (about
   1.32 times from 74° to 37°).
3. **Go anyway.** Answer the pause with `Go anyway`. The run should go on to the stack,
   with the imaging camera on its whole sensor (Imaging page, settings). If it ends,
   note the ending's words: `the focus aid could not put the imaging camera's settings
   back: …` gives the camera's own refusal, which this spec only infers. Read
   `restore_error` from `get_focus_reading` either way, and clear any region with `Use
   the whole sensor`.
4. **Watch the picture.** During the aid and the run, note whether the Imaging page's
   picture stays between previews, and the Focus block's readings while it says
   `Finding the focus region on a full imaging frame`.
5. **The starless wording (owner decision 4).** With the cap on, take one imaging frame
   and run a focus check; note its words. If it says `the imaging solver's measure could
   not be read`, keep the frame and the ASTAP output the scope node logged for it: the
   follow-up's evidence.
6. **The design changes, in a follow-up spec, only if** a check judged against a carried
   record calls a focus that the aid alone finds good `clearly_out` or the reverse, at
   altitudes 20° or more apart, on two nights; or `Go anyway` still ends a run after a
   pause.

## Plan traps

1. **Story sizes.** Estimated from the probe's commits, each story's whole diff, file by
   file (the README and `focus/run.rs` split by hunk): US1 about 21.2 KB (32.3 % of
   65,536 B), US2 about 13.0 KB (19.9 %), US3 about 22.3 KB (34.1 %), US4 about 11.7 KB
   (17.9 %). At the factory's worst overrun seen, 1.35 times, the largest, US3, is about
   30.2 KB (46.0 %), under 60 % (39,321 B).
2. **`FocusCheck` gains no field.** Tests build it by literal
   (`crates/finder/src/imaging/run.rs:320`); the carry stays in the aid's view, and the
   pause sentence is unchanged (FR-010).
3. **The 18-step checklist.** `imaging_run_docs.rs` counts its steps; the README's new
   words go in prose, never in the checklist.
4. **MCP descriptions are pinned.** The words reach MCP through the views it already
   returns; no tool's description changes.
5. **043's bound.** Only a `ready` camera is tried again; a silent node's refusal is the
   last at once.
6. **Paused time.** Every new Rust test runs under `start_paused = true`; the put-back's
   pause is `tokio::time::sleep`, so the stop's 1 s and 5 s are exact in paused time
   (US1-S3 allows 0.9 s to 2.0 s and 4.9 s to 6.0 s).
7. **The simulator's settle is per abort, on tokio's clock.** It uses
   `tokio::time::Instant`, so paused time moves it; a default apply is never refused.
8. **The motion rig's sync.** US3's tests wait for a first sync before Track; without a
   sync Track refuses.
9. **The dist rebuild.** US4 commits `dist` as `npm run build` makes it; the gate checks
   `git status --porcelain -- dist` is clean.
10. **The kept picture is set during render.** `ImagingPage` sets `kept` while
    rendering when the preview changes, guarded by `kept?.src !== frame.preview`, so it
    renders at most once more.
11. **052's and 053's files.** None is written. The altitude reads the session through
    `HeldSession` and `Session::snapshot`, both 032's, and `session.rs` is not edited.
12. **A stale build after a mutation.** Restoring a file by moving a copy back gives it
    an older time than the last build; touch it, or cargo runs the mutated code (seen
    on the probe).

## Risks

| Risk | Effect | Mitigation |
|---|---|---|
| The carry overstates where optics, not seeing, set the star's size | A real focus error at a low altitude reads better than it is | The aid's own readings, judged by the record at the same altitude, still say; § Outside the factory step 6 |
| The carry grows fast near the horizon (1.859 times from 74° to 20°) | A reading nearly twice the record reads `good` at 20° | The owner images mostly above 30°; a floor on the altitude is a follow-up if the field says so |
| The record is the smallest settled reading, so it reads low | Carried, it still sets a strict line (the field night's 6.7″ sits on it) | Unchanged from 033; the verdict compares, and the aid is beside it |
| The camera refuses for another reason while ready | The stop answers up to about 5 s later | The refusal's own words still reach `restore_error` |
| The field camera's refusal is inferred | The fix may not be the field's cause | The named words say the camera's own refusal on the next night (§ Outside the factory step 3) |
| The exponent rests on no local source | A wrong exponent mis-carries | Owner decision 2; a constant, one place to change |

## Refinement Record

Drafted on 2026-11-27 by the plate-solver operator seat's spec session against `main` at
f6d4f5fd, from a probe of that day on `probe/054` (the repro on the factory's rig, a
trial build of the four stories with every scenario's test and the gates passing, each
story's tests failing before it, the mutations, decision 1 (d) trial-built and set
aside, the verdict's candidates measured on the field night's numbers, and ASTAP on
starless frames), and a reading of 033's aid, put-back, record and verdict and its
Focus block, 031's lease and Imaging page, 037's and 041's run, focus gate, pause and
region refusal, 043's silent-node bound, the INDI adapter's abort, the simulator's
imaging camera, MCP's tools, the constitution and the survey of 2026-11-15. Writes come
from the task text. Shared writes name the spec story that last wrote the same file on
`main`; within the spec, each story's files written by an earlier one are ordered by
`depends_on_merged`.

None of the five owner decisions of spec.md is answered; this record is written to
their recommendations, and approves nothing. The flip is the owner's.

```yaml
version: 1
reviewed_revision: f6d4f5fd482ccc9d5b634ef1aa5124c112769583
stories:
  US1:
    tests:
      - path: crates/finder/tests/console/imaging_run_focus_restore.rs
        status: new
        disposition: add
        assertion: "the pause rig (imaging_run_focus.rs's rig with console::camera and console::focus, SimImagingCamera with_abort_settle 1 s, served by support::mcp::app), no focus record: POST /api/imaging/run {} 200; at run.state focus_pause with focus.aid.state running, readings >= 1, frame.state exposing and frame.purpose focus (run.focus.verdict no_record, settings.roi not whole): POST /api/imaging/run/continue {anyway: true} 200; once run.state is stacking or ended: run.state stacking, settings.roi x 0 y 0 and the camera's width and height, focus.aid without restore_error"
        decision: "spec.md:US1-S1"
      - path: crates/finder/tests/console/imaging_run_focus_restore.rs
        status: new
        disposition: add
        assertion: "the pause rig with with_abort_settle 10 s, going on anyway mid-exposure as in US1-S1: run.state ended, run.ended.reason camera_failed; focus.aid.restore_error 'the scope node did not apply the settings: the camera refused the settings: it is still stopping an exposure'; run.ended.message 'Ended: the imaging camera failed: the focus aid could not put the imaging camera's settings back: the scope node did not apply the settings: the camera refused the settings: it is still stopping an exposure.'; MCP get_imaging_run result.structuredContent.run.ended.message the same message; MCP get_focus_reading result.structuredContent.aid.restore_error the same restore words"
        decision: "spec.md:US1-S2"
      - path: crates/finder/tests/console/imaging_run_focus_restore.rs
        status: new
        disposition: add
        assertion: "the pause rig at settle 1 s and a fresh one at 10 s, no record: POST /api/imaging/focus {tile: {col 1, row 1}} 200; at focus.aid.state running, frame.state exposing, frame.purpose focus, settings.roi not whole: DELETE /api/imaging/focus answers 200 with focus.aid.ended_reason stopped; at 1 s within 0.9 to 2.0 s of paused time, no restore_error, settings.roi whole; at 10 s within 4.9 to 6.0 s, restore_error 'the scope node did not apply the settings: the camera refused the settings: it is still stopping an exposure', settings.roi as while the aid ran"
        decision: "spec.md:US1-S3"
      - path: crates/finder/tests/console/imaging_run_focus_restore.rs
        status: new
        disposition: add
        assertion: "crates/finder/README.md with line breaks read as spaces: ## Focus up to the next ## heading contains 'is tried again once a second for up to 5 s'; ## An imaging run up to ### Field checklist contains '`the focus aid could not put the imaging camera's settings back:`'"
        decision: "spec.md:US1-S4"
      - path: crates/finder/tests/console/imaging_abort_silent.rs
        status: existing
        disposition: preserve
        assertion: "043 US1-S8: a focus stop on a silent scope node ends stopped with restore_error 'the scope node is not connected' within 043's bound"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/console/imaging_focus_aid.rs
        status: existing
        disposition: preserve
        assertion: "a lost link during the aid ends it failed with a non-empty restore_error and the lease free"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/motion/imaging_run.rs
        status: existing
        disposition: preserve
        assertion: "a start with the observer's region of interest x 0 y 0 412 x 274 is refused with 'the imaging camera has a region of interest set: clear it before an imaging run'"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/console/imaging_run_docs.rs
        status: existing
        disposition: preserve
        assertion: "the README's field checklist for the first imaging run has exactly steps 1 to 18"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "041's region refusal (owner decision 24) refuses a start or re-hold with a region in force; only its words change when the aid's put-back failed"
      - "SimImagingCamera at its default settle of zero is f6d4f5fd's camera"
      - "037's step traits and FocusCheck are unchanged"
    checks:
      development:
        - "cargo nextest run -p finder --locked --all-features --test console -E 'test(imaging_run_focus_restore) | test(imaging_abort_silent) | test(imaging_focus_aid)'"
        - "cargo nextest run -p simulator --locked"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - crates/simulator/src/imaging.rs
      - crates/finder/src/imaging/focus/run.rs
      - crates/finder/src/imaging/run/console.rs
      - crates/finder/README.md
      - crates/finder/tests/console/imaging_run_focus_restore.rs
      - crates/finder/tests/console/main.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/simulator/src/imaging.rs
        other_spec: 035-live-stack
        other_story: US1
        disposition: merge_order
        reason: "054 depends_on_landed 035, whose US1 last changed the simulated imaging camera (7063003f)"
      - path: crates/finder/src/imaging/focus/run.rs
        other_spec: 033-focus-reading-and-aid
        other_story: US8
        disposition: merge_order
        reason: "054 depends_on_landed 033, whose US8 last changed the aid's run (9e670694)"
      - path: crates/finder/src/imaging/run/console.rs
        other_spec: 041-imaging-run-on-the-rig
        other_story: US8
        disposition: merge_order
        reason: "054 depends_on_landed 041, whose US8 last changed the run's console parts (61b54a47)"
      - path: crates/finder/README.md
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US3
        disposition: merge_order
        reason: "054 depends_on_landed 051, whose US3 last changed the README (9424a371)"
      - path: crates/finder/tests/console/main.rs
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US2
        disposition: merge_order
        reason: "054 depends_on_landed 051, whose US2 last changed console/main.rs (f9706cc0)"
    open_questions: []
  US2:
    tests:
      - path: crates/finder/tests/console/imaging_focus_carry.rs
        status: new
        disposition: add
        assertion: "carry(4.6, 74.0, 37.0) within 1e-12 of 4.6 (X(37)/X(74))^0.6 with X(h) = 1/sin h, and within 5e-4 of 6.0924; carry(4.6, 74.0, 74.0) == 4.6; carry of that back from 37 to 74 within 1e-12 of 4.6; judge(Ok(6.7), Some(4.6)) and judge(Ok(6.4), Some(4.6)) ClearlyOut; judge(Ok(6.7), Some(carry(4.6, 74, 37))) and judge(Ok(6.4), Some(carry(4.6, 74, 38))) Good; judge(Ok(8.0), Some(carry(4.6, 74, 37))) ClearlyOut"
        decision: "spec.md:US2-S1"
      - path: crates/finder/tests/console/imaging_focus_carry.rs
        status: new
        disposition: add
        assertion: "a Kept on a scratch focus.json described with the simulated camera, T 1768528800: note_at(3.0, T, None, false, Some(74)) true, reference_at(T+1, None) (3.0, Tonight, Some(74)), file record.tonight.alt_deg 74; note_at(2.9, T+60, .., Some(60)) true; note_at(3.1, T+120, .., Some(30)) false; altitudes(T+121, None).tonight Some(60); a second Kept on the file reference_at(T+121, None) (2.9, Tonight, Some(60)); note_at(3.2, T+43260, .., None) true, reference_at(T+43261, None) (2.9, Earlier, Some(60)), file record.earlier_alt_deg 60 and no record.tonight.alt_deg; note(2.5, T+43300, None, false) true, reference_at(T+43301, None) (2.5, Tonight, None)"
        decision: "spec.md:US2-S2"
      - path: crates/finder/tests/console/imaging_focus_carry.rs
        status: new
        disposition: add
        assertion: "a fresh Kept: note_at(3.0, T, Some(0.55), false, Some(74)) true; altitudes(T+1, Some(0.55)).tonight Some(74); altitudes(T+1, Some(1.1)) Altitudes::default(); note_at(5.0, T+60, Some(1.1), false, None) true; reference_at(T+61, Some(1.1)) (5.0, Tonight, None); 033's own focus.json with record.tonight px 3.3 and no altitudes: reference_at(T, None) (3.3, Tonight, None) and reference(T, None) (3.3, Tonight)"
        decision: "spec.md:US2-S3"
      - path: crates/finder/tests/console/imaging_focus_record.rs
        status: existing
        disposition: preserve
        assertion: "033's record: Kept::note and Kept::reference with their signatures, Mark { px, at } by literal, nights, the passed mark and the foreign scale as 033 words them"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "Kept::note and Kept::reference keep their signatures and behaviour; judge and its thresholds are unchanged"
      - "focus.json's format stays imaging-focus/1, and 033's files read as before"
    checks:
      development:
        - "cargo nextest run -p finder --locked --all-features --test console -E 'test(imaging_focus_carry) | test(imaging_focus_record)'"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - crates/finder/src/imaging/focus.rs
      - crates/finder/tests/console/imaging_focus_carry.rs
      - crates/finder/tests/console/main.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/imaging/focus.rs
        other_spec: 033-focus-reading-and-aid
        other_story: US5
        disposition: merge_order
        reason: "054 depends_on_landed 033, whose US5 last changed focus.rs (c8d30d26)"
      - path: crates/finder/tests/console/main.rs
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US2
        disposition: merge_order
        reason: "054 depends_on_landed 051, whose US2 last changed console/main.rs (f9706cc0)"
    open_questions: []
  US3:
    tests:
      - path: crates/finder/tests/motion/imaging_focus_altitude.rs
        status: new
        disposition: add
        assertion: "the motion rig (SimMount, pushed to alt 50 az 120, target M42, a first sync, the aim at the eyepiece pixel) with focus.json by hand: record.tonight px 2.0 at START - 600 and alt_deg 74; Track on and 15 s; a check: focus.aid.ended_reason done; reference.carried.from_alt_deg 74, carried.alt_deg within 0.5 deg of sim.telescope().eyepiece.alt, carried.from within 1e-9 of 2.0 s; reference.value within 1e-9 relative of 2.0 s (X(alt)/X(74))^0.6; verdict good at most 1.10, slightly_soft at most 1.30 times that, clearly_out beyond, and slightly_soft; median.value above 1.30 x 2.0 s; focus.record.tonight.alt_deg 74"
        decision: "spec.md:US3-S1"
      - path: crates/finder/tests/motion/imaging_focus_altitude.rs
        status: new
        disposition: add
        assertion: "the motion rig with no record, focus error 0.5: a check has no focus.aid.reference; POST /api/imaging/focus {} until focus.record.tonight is an object: no record.tonight.alt_deg; DELETE; Track on and 15 s, focus error 0, POST {find: true}: focus.record.tonight.alt_deg comes and is within 0.5 deg of sim.telescope().eyepiece.alt"
        decision: "spec.md:US3-S2"
      - path: crates/finder/tests/motion/imaging_focus_altitude.rs
        status: new
        disposition: add
        assertion: "crates/finder/README.md with line breaks read as spaces: ## Focus contains 'a star's size grows as the airmass to the power 0.6', 'judged against the record carried to its own altitude' and 'the far-out limit is about 11″'; ## An imaging run up to ### Field checklist contains 'against the record carried to the target's altitude'"
        decision: "spec.md:US3-S3"
      - path: crates/finder/tests/console/imaging_run_focus.rs
        status: existing
        disposition: preserve
        assertion: "the run's focus.record equals the aid's reference.value, judged against a record with no altitude, as 041 words it"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/mcp/imaging_focus.rs
        status: existing
        disposition: preserve
        assertion: "the MCP focus tools' descriptions word for word"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "a reading or a record with no altitude is judged as at f6d4f5fd"
      - "the run's pause sentence and FocusCheck are unchanged"
      - "crates/finder/src/{model,goto,live,session}.rs are read through SessionView only"
    checks:
      development:
        - "cargo nextest run -p finder --locked --all-features --test motion -E 'test(imaging_focus_altitude)'"
        - "cargo nextest run -p finder --locked --all-features --test console -E 'test(imaging_run_focus) | test(imaging_focus)'"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - crates/finder/src/imaging/focus/run.rs
      - crates/finder/README.md
      - crates/finder/tests/motion/imaging_focus_altitude.rs
      - crates/finder/tests/motion/main.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/imaging/focus/run.rs
        other_spec: 033-focus-reading-and-aid
        other_story: US8
        disposition: merge_order
        reason: "054 depends_on_landed 033, whose US8 last changed the aid's run (9e670694)"
      - path: crates/finder/README.md
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US3
        disposition: merge_order
        reason: "054 depends_on_landed 051, whose US3 last changed the README (9424a371)"
      - path: crates/finder/tests/motion/main.rs
        other_spec: 050-tools-page-aligns-finder-camera
        other_story: US3
        disposition: merge_order
        reason: "054 depends_on_landed 050, whose US3 last changed motion/main.rs (6b70a4a2)"
    open_questions: []
  US4:
    tests:
      - path: crates/finder/web/src/components/FocusView.test.tsx
        status: new
        disposition: add
        assertion: "ImagingPage with fetch stubbed: ready, frame 1 saved with preview /api/imaging/frames/1/preview.jpg: in the region Imaging frame the img Imaging frame 1 has that src; frame 2 exposing, no preview: once the status reads 'Exposing for 1000 ms' the img Imaging frame 1 is still there; frame 2 with preview /api/imaging/frames/2/preview.jpg: img Imaging frame 2 with that src and no img Imaging frame 1"
        decision: "spec.md:US4-S1"
      - path: crates/finder/web/src/components/FocusView.test.tsx
        status: new
        disposition: add
        assertion: "FocusBlock, unit arcsec at 0.55, aid finding with readings 9 and median, last and best 6.7: in the region Focus the text '6.70″' and 'Finding the focus region on a full imaging frame'"
        decision: "spec.md:US4-S2"
      - path: crates/finder/web/src/components/FocusView.test.tsx
        status: new
        disposition: add
        assertion: "FocusBlock, aid finding with readings 0 and no median, last, best or history: the region Focus has no text 'Focus reading'"
        decision: "spec.md:US4-S3"
      - path: crates/finder/web/src/components/FocusView.test.tsx
        status: new
        disposition: add
        assertion: "FocusBlock, record.tonight value 4.6 alt_deg 74, aid running, median 6.7, verdict good, reference value 6.09 of tonight carried {from 4.6, from_alt_deg 74, alt_deg 37}: in the region Focus the text 'Focus record: 4.60″ at 74° tonight.' and 'Within 10% of the focus record of 6.09″ tonight, as at 37° from 4.60″ at 74°.'"
        decision: "spec.md:US4-S4"
      - path: crates/finder/web/src/components/FocusBlock.test.tsx
        status: existing
        disposition: preserve
        assertion: "033's Focus block words, with no alt_deg and no carried, unchanged"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/e2e/imaging.spec.ts
        status: existing
        disposition: preserve
        assertion: "the Imaging page's frame picture and its red-only night view in the browser"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "the page adds no colour, class or token; the night view stays red-only"
      - "dist is the build of its source"
    checks:
      development:
        - "npm test -- FocusView"
        - "npm test -- FocusBlock ImagingPage"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - crates/finder/web/dist
      - crates/finder/web/src/api.ts
      - crates/finder/web/src/components/FocusBlock.tsx
      - crates/finder/web/src/components/FocusView.test.tsx
      - crates/finder/web/src/components/ImagingPage.tsx
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/web/dist
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US4
        disposition: merge_order
        reason: "054 depends_on_landed 051, whose US4 last rebuilt dist (f6d4f5fd)"
      - path: crates/finder/web/src/api.ts
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US4
        disposition: merge_order
        reason: "054 depends_on_landed 051, whose US4 last changed api.ts (f6d4f5fd)"
      - path: crates/finder/web/src/components/FocusBlock.tsx
        other_spec: 033-focus-reading-and-aid
        other_story: US6
        disposition: merge_order
        reason: "054 depends_on_landed 033, whose US6 last changed the Focus block (71ee65fb)"
      - path: crates/finder/web/src/components/ImagingPage.tsx
        other_spec: 042-imaging-run-page-and-mcp
        other_story: US9
        disposition: merge_order
        reason: "054 depends_on_landed 042, whose US9 last changed the Imaging page (5d8f5353)"
    open_questions: []
```
