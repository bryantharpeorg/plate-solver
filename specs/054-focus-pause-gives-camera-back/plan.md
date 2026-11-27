# Implementation Plan: The Focus Pause Gives the Camera Back

**Branch**: `054-focus-pause-gives-camera-back` | **Date**: 2026-11-27 |
**Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/054-focus-pause-gives-camera-back/spec.md`

## Summary

The imaging run's focus pause closes 033's focus aid on every answer, and the aid puts
the camera's settings back once, right after it aborts its own exposure. A camera
still stopping that exposure refuses them, the refusal is kept only as the aid's
`restore_error`, and the run, holding the camera again, finds the aid's focus region
and ends `camera_failed` with 041's words. The fix is in the aid's one put-back: the
scope node's refusal while the camera is `ready` is tried again once a second for up
to 5 s, the region a put-back could not take back is kept on the focus state with its
words, no later aid or check keeps it, and 041's refusal names it while it stands.
Then the record keeps the altitude it was read at (Track's target's, while Track is
tracking), and a reading is judged against the record carried to its own altitude by
the airmass to the power 0.6, the factor held between 1 and 1.30, so the field night's
6.7″ at 37° against 4.6″ at 74° reads `slightly_soft` and the run goes on. Last, the
Imaging page keeps its last picture until the next preview and says so, the Focus
block keeps its readings while the aid looks for its region again, and both lines that
compare with the record name the altitudes.

Four stories: the put-back and its words; the record's altitude and the carry; judging
at Track's altitude; the page. Each is under 60 % of 65,536 B on probe round 1's
commits with this fold's steps added, US1 the largest at 56.2 % (§ Plan traps 1). Two
general rules remove the side effects a narrower fix would bring
(spec.md § Owner decisions): **whatever the focus aid sets, it gives back before anyone
else holds the camera, or says why it could not, and no aid keeps what another aid
left; and a reading is compared with the record at one altitude, the reading's own, by
a carry that never makes the record stricter.**

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
f6d4f5fd, only while the camera is ready and the scope node refuses.

**Constraints**: no change to the scope node, the scope link's protocol, 037's step
traits or 041's region refusal (decision 24 stands); no existing test edited; MCP's
tool descriptions unchanged (pinned by tests). Keep out of 052's files
(`crates/star-detection/`, `crates/scope-node/src/detect.rs`) and 053's
(`crates/finder/src/{model,goto,live,session}.rs`): the altitude reads `SessionView`
through `Session::snapshot`, and none of them is written. 052, 053 and 055 share
other files with this spec (§ Prerequisites 2).

**Scale/Scope**: four stories, about 84 KB of diff in all on probe round 1 (about
86 KB written once with this fold's steps).

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Detection and the plate solver are untouched. The focus reading is 033's (ASTAP's `-analyse` on the scope's Pi, the simulated solver in the gates), unchanged. The carry rests on seeing theory, a star's size in seeing growing as the airmass to the power 3/5, which no local source gives (V below), applied only between 1 and 1.30 because the survey says seeing is not the whole size (`docs/research/2026-11-15-imaging-camera.md:220`): flagged as such in spec.md owner decision 2 and § Risks, and measured on the field night's own numbers (§ What was run). What rests on reading is listed in § What was run and what was read. |
| II. Never weaken a gate | No existing test, golden, fixture, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS` or `ergane.yaml` is edited. The only edits to existing test files are the `mod` lines of `crates/finder/tests/console/main.rs` (US1, US2) and `crates/finder/tests/motion/main.rs` (US3) (SC-002). The simulated imaging camera is f6d4f5fd's at its default settle of zero, so no existing caller sees a change (run: the 1,525 existing tests and the stories' 13 passed on the probe's tip, and again on probe round 1's tip after this fold, 06ec58d0, § What was run). 043's silent-node bound is kept by the rule that only the scope node's refusal (502) of a `ready` camera is tried again (the mutation that tries any refusal again fails 043's US1-S8 test). Owner decision 1 (b)'s first form would have edited 030's US6-S6 test's meaning; it is set aside. |
| III. Tests first, independent expectations | The oracles are the words of 041's refusal, of the simulator's refusal and of the scenarios; 031's own view of the settings and camera; the airmass written in each test as the secant of the zenith distance, the exponent 0.6 and the bounds 1 and 1.30 written there; 033's thresholds 1.10 and 1.30; the field night's numbers; the simulator's own altitude of the main telescope; and the views the page tests write. No test reads `RESTORE_PAUSE`, `RESTORE_WAIT`, `SEEING_EXPONENT` or `CARRY_CAP`, or calls `carry` for an expected value. Time comes from a paused tokio clock and the simulator's virtual clock. Every story's tests fail before it (run, § What was run). |
| IV. Read the critical paths twice | None is touched: no `Cargo.toml` or `Cargo.lock` change, no `unsafe`, nothing under `proto/`, `crates/math-core`, `crates/pattern-database` or `crates/database-generation`, no `build.rs` or `.cargo/`. `crates/simulator` is not a critical path; its change is owner approval 1. |
| V. Proven open source first, with honest licensing | Looked for, in local sources only (no download). **For the put-back:** the INDI adapter's own waits (`crates/scope-node/src/imaging/indi/mod.rs:40` to `crates/scope-node/src/imaging/indi/mod.rs:45`), whose `RETRY_PAUSE` of 1 s before an exposure is tried again is the pattern the retry follows, and its `ANSWER_WAIT`, which owner decision 1 (b) would have extended to the abort; no library is needed for a bounded retry of one call. Ekos, PHD2 and N.I.N.A. are applications, not libraries; that each capture there sets its own frame before it exposes (from memory, not read here) is the rule "no aid keeps what another aid left". **For the altitude:** the repository's survey of 2026-11-15, § 6 "Focus" (`docs/research/2026-11-15-imaging-camera.md:203`), with its sources on PHD2, SharpCap, Ekos and N.I.N.A.: each compares a reading with recent ones or the best since a reset (`docs/research/2026-11-15-imaging-camera.md:231`), and the survey says a reading cannot tell focus error from seeing (`docs/research/2026-11-15-imaging-camera.md:227`); none, as surveyed, carries a size across altitudes, and the survey gives no source for the airmass dependence. A search of the whole tree at f6d4f5fd, `reference-solutions/` (cedar-detect, cedar-solve) among it, finds no airmass anywhere. ASTAP's `-analyse`, installed as `astap_cli`, gives a half-flux diameter and a star count only. The console's own `sky::alt_az` (`crates/finder/src/sky.rs:258`) gives the geometric altitude, so nothing is adopted for it. The exponent 3/5 is seeing theory's (the Fried parameter's dependence on the zenith distance), cited from no local source: the owner's decision 2 accepts or refuses it. **For the page:** nothing to adopt; the kept picture is page state. |

## Project Structure

```text
crates/simulator/src/imaging.rs                         US1: with_abort_settle, STILL_STOPPING
crates/finder/src/imaging/focus/run.rs                  US1: RESTORE_PAUSE, RESTORE_WAIT, put_back, the region an aid left
                                                        US3: the altitude, the carried reference, the view's fields
crates/finder/src/imaging/mod.rs                        US1: Imaging::set forgets the region an aid left
crates/finder/src/imaging/run/console.rs                US1: the region refusal's second form
crates/finder/src/imaging/focus.rs                      US2: SEEING_EXPONENT, CARRY_CAP, carry, Altitudes, note_at, reference_at, altitudes, focus.json fields
crates/finder/README.md                                 US1: ## Focus and ## An imaging run, the put-back
                                                        US3: ## Focus, the unit and the altitude; ## An imaging run, the carried check
GLOSSARY.md                                             US3: Focus record
crates/finder/tests/console/imaging_run_focus_restore.rs  US1, new
crates/finder/tests/console/imaging_focus_carry.rs        US2, new
crates/finder/tests/console/main.rs                     US1, US2: one mod line each
crates/finder/tests/motion/imaging_focus_altitude.rs    US3, new
crates/finder/tests/motion/main.rs                      US3: one mod line
crates/finder/web/src/api.ts                            US4: alt_deg, carried
crates/finder/web/src/components/ImagingPage.tsx        US4: the kept picture and its line
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
are not `ImagingSettings::default()` and `now < refused_until` (refused while less than
the settle has passed; no scenario sits on the instant itself).

### `crates/finder/src/imaging/focus/run.rs`, the put-back (US1)

```rust
/// A put-back the scope node refuses while the camera is `ready` is tried
/// again after this long (054), as the scope node waits after its own aborts.
pub const RESTORE_PAUSE: Duration = Duration::from_secs(1);
/// No put-back is tried later than this after the first (054).
pub const RESTORE_WAIT: Duration = Duration::from_secs(5);

/// Put the remembered settings back through the lease (FR-027; 054): the
/// scope node's refusal (502) while the camera is `ready` is followed by
/// another try `RESTORE_PAUSE` later, so long as that try starts within
/// `RESTORE_WAIT` of the first; any other refusal is the last at once.
/// Answers the last refusal's words, or none once a try takes.
async fn put_back(imaging: &Arc<Imaging>, lease: &Lease, remembered: &Remembered) -> Option<String> {
    let first = tokio::time::Instant::now();
    loop {
        let refusal = match lease.set(remembered.edit()).await {
            Ok(_) => return None,
            Err(refusal) => refusal,
        };
        let ready = imaging.state.lock().expect("imaging state").state == "ready";
        if !ready
            || refusal.status != axum::http::StatusCode::BAD_GATEWAY
            || first.elapsed() + RESTORE_PAUSE > RESTORE_WAIT
        {
            return Some(refusal.message);
        }
        tokio::time::sleep(RESTORE_PAUSE).await;
    }
}
```

The focus `State` (`crates/finder/src/imaging/focus/run.rs:188`) gains
`left: Option<(RoiView, String)>`, `None` in `State::new`, documented as "the region an
aid could not take back, and its put-back's words (054): kept here, not with the run, so
a later run does not lose it", and:

```rust
impl State {
    // `describe` (crates/finder/src/imaging/focus/run.rs:206) also sets `self.left = None`.
    /// The observer changed the settings (054): a region an aid left is now theirs.
    pub(in crate::imaging) fn forget_left(&mut self) { self.left = None; }
    /// The words of the region an aid could not take back, while that region is
    /// still the one in force (054).
    fn left_in(&self, roi: Option<RoiView>) -> Option<&str>; // Some only while `left`'s region == roi
}
impl Imaging {
    /// The words of a put-back that failed, while the region the aid could not
    /// take back is still in force (054); 041's region refusal names it.
    pub(in crate::imaging) fn focus_left(&self) -> Option<String>; // left_in(held.settings' roi)
}
```

`remember` (`crates/finder/src/imaging/focus/run.rs:719`) takes the settings' region as
whole also when `held.focus.left_in(Some(settings.roi))` is `Some`: an aid never keeps
an aid's leftovers. `finish` (`crates/finder/src/imaging/focus/run.rs:1047`) calls
`put_back` in place of its one `lease.set`
(`crates/finder/src/imaging/focus/run.rs:1058`); in the same critical section that sets
the lifecycle `ended` and `run.restore_error`
(`crates/finder/src/imaging/focus/run.rs:1072`), when settings were remembered, it sets
`held.focus.left` to the settings' region in force with the words when the put-back
failed, and to `None` when it took. `use std::time::Duration` and `RoiView` join the
imports. No lock is held across an await.

### `crates/finder/src/imaging/mod.rs` (US1)

`Imaging::set` (`crates/finder/src/imaging/mod.rs:584`), which `PUT
/api/imaging/settings` and MCP's `set_imaging_camera` call (the lease's `set` does not),
becomes `let view = self.set_as(None, edit).await?;`, then
`self.state.lock().expect("imaging state").focus.forget_left();` under the comment "A
region an aid left is the observer's once they change the settings (054).", then
`Ok(view)`.

### `crates/finder/src/imaging/run/console.rs` (US1)

In `ConsoleCamera::hold` (`crates/finder/src/imaging/run/console.rs:358`), the region
refusal (`crates/finder/src/imaging/run/console.rs:378`) becomes:

```rust
// A region the focus aid could not take back is named as that (054).
return Err(match imaging.focus_left() {
    Some(words) => format!("the focus aid could not put the imaging camera's settings back: {words}"),
    None => "the imaging camera has a region of interest set: clear it before an imaging run".into(),
});
```

The lease is given back on that path as before (the local `lease` is dropped).

### The README, US1's sentences (`crates/finder/README.md`)

In `## Focus`, the sentence ending `puts the settings back and releases the camera.` is
followed, in the same paragraph, by:

> A camera still stopping an aborted exposure can refuse the settings for a moment, so
> a put-back the scope node refuses while the camera is ready is tried again once a
> second for up to 5 s, and a stop answers that much later. One that never takes is the
> aid's `restore_error`, and the camera keeps the aid's settings until they are changed
> by hand: `Use the whole sensor` on the Focus block, or `PUT /api/imaging/settings`
> with `{"roi":null}`. Until then the region is named as the aid's, even after a later
> check, and the next aid or check puts the whole sensor back rather than keep it.

In `## An imaging run`, the sentence ending `the page shows its time left.` is followed,
in the same paragraph, by:

> Every answer, and a Stop, closes the aid, which puts the camera's settings back as
> `## Focus` says, and the run then holds the camera again. When the aid could not put
> them back, the run ends `camera_failed` with the camera's own words after
> `the focus aid could not put the imaging camera's settings back:`. Clear the region
> with `Use the whole sensor` and start again.

### `crates/finder/src/imaging/focus.rs` (US2)

```rust
/// Seeing widens a star as the airmass to this power (054): the record, read
/// at one altitude, is carried to a reading's altitude by it.
pub const SEEING_EXPONENT: f64 = 0.6;
/// The most the air may loosen the record (054): a low target is never let
/// off by more than this factor, so `clearly_out` stays reachable.
pub const CARRY_CAP: f64 = 1.30;

/// The record's size `px`, read at `from_alt_deg`, as it judges a reading at
/// `to_alt_deg` (054): times the ratio of the two airmasses, each the secant
/// of the zenith distance, to the power `SEEING_EXPONENT`, that factor held
/// between 1 and `CARRY_CAP`, so the record is never made stricter.
pub fn carry(px: f64, from_alt_deg: f64, to_alt_deg: f64) -> f64 {
    let airmass = |alt: f64| 1.0 / alt.to_radians().sin();
    let factor = (airmass(to_alt_deg) / airmass(from_alt_deg)).powf(SEEING_EXPONENT);
    px * factor.clamp(1.0, CARRY_CAP)
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
(`earlier_alt_deg` only while the earlier part exists). In `note_at`, sizes are compared
as read, as `note` compares them: the foreign rule clears both altitudes with the
record; a new night sets `tonight_alt = alt_deg`; at a rollover, when the old night's
record joins the earlier nights', `earlier_alt = tonight_alt` before `tonight_alt =
alt_deg`; a smaller reading of tonight sets `tonight_alt = alt_deg`; a note that changes
no size leaves both. `altitudes(now, scale)` answers `Altitudes::default()` at a foreign
scale; while tonight's night began at most `NIGHT_S` ago, `{tonight: tonight_alt,
earlier: earlier_alt}`; after it, `{tonight: None, earlier: earlier_alt when the earlier
record is at most tonight's size, else tonight_alt}`, as `record` shows the parts; with
no tonight, `{None, earlier_alt}`. `reference_at` is `reference` with
`altitudes(now, scale).of(of)`.

### `crates/finder/src/imaging/focus/run.rs`, the altitude (US3)

```rust
/// A size and when it was read (FR-029), and the altitude it was read at, when known (054).
pub struct SizeAt { pub value: f64, pub at: f64,
    #[serde(skip_serializing_if = "Option::is_none")] pub alt_deg: Option<f64> }

/// The last sample in the view (FR-025), and the altitude the frame was read at,
/// when known (054): `LastView` gains, after `stars`,
    #[serde(skip_serializing_if = "Option::is_none")] pub alt_deg: Option<f64>,

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
at, alt)`, and `note` sets `run.last`'s `alt_deg` to `alt`:

```rust
/// The altitude, at `at`, of the place Track holds (054): while the
/// session's Track is `tracking` and a site is set, its target's geometric
/// altitude then; otherwise, and at or below the horizon, none. The session
/// is read with the imaging state's lock released.
fn altitude(imaging: &Arc<Imaging>, at: f64) -> Option<f64> {
    let session = imaging.state.lock().expect("imaging state").session.clone()?;
    let view = session.0.snapshot(at);
    let track = view.track.filter(|track| track.state == "tracking")?;
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

> Seeing also grows with the air a star is seen through: the part of a star's size
> that seeing makes grows as the airmass to the power 0.6, the airmass being the secant
> of the zenith distance. While Track is tracking and a site is set, each reading
> carries the geometric altitude of Track's target at the middle of its exposure, and
> the record keeps the altitude of the reading that made it. A reading is then judged
> against the record carried to its own altitude: its size times the ratio of the two
> airmasses to the power 0.6, that factor never by less than 1 or more than 1.30, so
> the air never makes the record stricter and never lets a low target off by more than
> a slightly soft focus. A record of 4.6″ read at 74° judges a reading at 37° as a
> record of 5.98″ would. The record's own parts are compared with each other as read.
> With either altitude unknown (Track not tracking, no site, or a record made before
> the console kept altitudes) the record judges as it is. The record's parts show their
> `alt_deg`, the latest reading its `alt_deg`, and a carried reference shows `carried`:
> the record's own size `from`, its `from_alt_deg` and the reading's `alt_deg`.

In `## An imaging run`, after US1's sentence ending `and start again.`:

> The check is judged against the record carried to the target's altitude, so a record
> made high in the sky does not call a low target's stars out of focus for the air
> alone; the pause's words and the run's `focus.record` give that carried size.

### `GLOSSARY.md`, US3's words

**Focus record** (`GLOSSARY.md:229`) becomes: "The smallest settled reading this
imaging camera has given on this telescope: tonight's, and the earlier nights', each
with the altitude it was read at when known. A reading is judged against the smaller of
the two, or against tonight's alone once the focus aid has passed through focus tonight,
carried to the reading's altitude when both altitudes are known; never against an
absolute number." Its `_Avoid_` line is unchanged.

### The page (`crates/finder/web/src/`, US4)

- `api.ts`: `ImagingFocusSized` gains `alt_deg?: number`; `ImagingFocusAid.last` gains
  `alt_deg?: number`; `ImagingFocusAid.reference` gains `carried?: { from: number;
  from_alt_deg: number; alt_deg: number }`.
- `ImagingPage.tsx`: a state `kept: { id: number; src: string } | null`, set during
  render whenever `frame?.preview` is set and differs from `kept.src`; `picture =
  frame?.preview != null ? { id: frame.id, src: frame.preview } : kept`. The picture's
  `div`, its `img` (`src={picture.src}`, `alt={`Imaging frame ${picture.id}`}`, the same
  classes) and the night's hold button (`crates/finder/web/src/components/ImagingPage.tsx:280`)
  show while `picture != null` in place of `frame?.preview`
  (`crates/finder/web/src/components/ImagingPage.tsx:271`); right after the picture's
  `div`, while `picture != null && frame?.preview == null`, a
  `<p className="quiet">` with `Frame ${picture.id}, the last picture`.
- `FocusBlock.tsx`: `showReading = aid != null && (aid.state === "running" ||
  (aid.state === "finding" && aid.readings > 0) || done)`
  (`crates/finder/web/src/components/FocusBlock.tsx:184`); the record line writes each
  part through `sized(part)`: `${size(part.value)} at ${part.alt_deg.toFixed(0)}°` with
  an `alt_deg`, else `${size(part.value)} with no altitude` while `aid?.last?.alt_deg !=
  null`, else `size(part.value)`; `verdictWords`
  (`crates/finder/web/src/components/FocusBlock.tsx:83`) names the record as `record`:
  with `reference.carried`, `${size(carried.from)} at
  ${carried.from_alt_deg.toFixed(0)}°${of}, as it reads at
  ${carried.alt_deg.toFixed(0)}°: ${size(reference.value)}`, else
  `${size(reference.value)}${of}`, `of` being ` tonight` or ` from earlier nights`; the
  three sentences end `the focus record of ${record}.`, the percentage still against
  `reference.value`.
- No colour, class or token is added: the night view's red-only rule holds as it is.

## What was run and what was read

All on `probe/054`, cut from f6d4f5fd, and on `probe/054-r1`, cut from this trio at
a2a65162, with the factory's Rust toolchain, `--locked`, `npm ci --offline` and the
factory's Playwright browsers; never pushed. Logs and scripts in the operator's probe
folder, outside the repository; the review fold's are `fold-r1-NN`, probe round 1's
fold's `fold-probe-r1-NN`.

- **The repro (run).** 2bd35a98 adds the simulator's settle and the repro test. On
  main's console code, going on anyway mid-exposure with a settle of 1 s ends
  `camera_failed` with 041's words, `restore_error` REFUSED, the region the aid's tile.
- **The first draft's put-back (run).** 8ea29909: the retry and the named words. Review
  r1 then ran, on 6ca3d283: after `Check focus again` mid-exposure the second pause's
  aid ran on the left tile; the aid alone stopped mid-exposure, a check, then a run
  start answered 409 with 041's words.
- **The fold's put-back (run).** 261bcd88, bfc04796, 77b38efa, 965ec262: the retry
  only for the scope node's refusal (502) while ready, the region an aid left on the
  focus state, `remember` taking it as whole, `Imaging::set` forgetting it, `hold`
  naming it, and US1's seven tests. Fails before (fold-r1-14, on the tip): on main's
  `focus/run.rs`, `imaging/mod.rs`, `run/console.rs` and README, with the simulator
  knob, all seven fail. US1-S5 first passed there (fold-r1-11: main names no region, so
  041's words after the PUT were main's too); it now also asserts the aid's region named
  at a start before the PUT, and fails.
- **Decision 1 (b), trial (run, fold-r1-01).** 973565bf (set aside in cf547227): the
  INDI adapter holds a settings change after the port's abort until the device answers
  `CCD_ABORT_EXPOSURE`, within `ANSWER_WAIT`, and three tests on the scripted server
  (`crates/scope-node/tests/indi_abort_answer.rs`): a driver that answers the abort
  1.5 s late, the put-back's first vector sent at that answer and taken; one that never
  answers, sent at 5 s and taken; one that answers at once and refuses `CCD_FRAME`,
  refused all the same. Every existing scope-node INDI test passed; the first two
  tests fail on main's adapter. The form in which the abort and the exposure answer
  only at the device's answer fails 030's
  `abort_ends_the_exposure_and_the_rules_of_one_in_flight_hold`
  (`crates/scope-node/tests/indi_expose.rs:782`, at
  `crates/scope-node/tests/indi_expose.rs:865`, 5 s late).
- **Decision 1 (f), trial (run).** 5e5b5b84 (set aside in 2664feef), as spec.md owner
  decision 1 gives it.
- **The altitude (run).** 06c4521a, 88956243, d49459ee, 6ca3d283, then the fold's
  261bcd88: the capped one-sided carry, Track `tracking` only, `last.alt_deg`, US2 split
  into three tests. In the motion rig, Track stays `armed` for its GoTo for about 18 s
  of paused time, and a check made at once ends `done` within about 3 s, uncarried;
  under `tracking` M42 stands at about 42.95°, Track's target's altitude within 0.5° of
  the simulator's own; a record of 2.0 px at 74° is carried to 2.459 px (factor 1.229,
  under the cap) and a reading of 2.826 px judged `slightly_soft`, `clearly_out`
  uncarried. Fails before (fold-r1-03): US2's test binary does not build on main's
  `focus.rs` (no `focus::carry`, `Kept::note_at`, `Kept::reference_at`,
  `Kept::altitudes`), and on the first draft's uncapped carry US2-S1 fails; US3's three
  tests fail on US1's `focus/run.rs` and README.
- **The carry's options (run, fold-r1-04).** The field night's numbers and the rig's
  under decision 2's options, as spec.md owner decision 2 gives them (its (b) shares,
  its record-altitude bounds and (d)'s 20.5° recomputed by hand in fold-probe-r1-08);
  under (a) the scenario readings are 1.070 (6.4″ at 38°), 1.221 (7.3″ at 37°) and
  1.421 (8.5″ at 37°) times the carried record of 5.98″, each at least 0.029 from 033's
  1.10 and 1.30.
- **ASTAP (run).** The local `astap_cli` CLI-2025.09.29, `-analyse 30`, on frames made
  for the probe: noise, bright noise, daylight and flat frames give `HFD_MEDIAN=99.0`
  and `STARS=0`, exit 0; a star field `HFD_MEDIAN=3.9`, `STARS=40`; saturated and
  all-zero frames `HFD_MEDIAN=99.0` with `STARS=29721` and `STARS=29119`. The field's own
  ASTAP's output for a starless frame: not run (not on this machine).
- **The page (run).** e74a5f0c, then the fold's 261bcd88 and 965ec262: the kept
  picture's line, the plainer verdict line and ` with no altitude`; the first draft's
  guard (no reading before the first, which main passes) joined US4-S2's test. Vitest
  (fold-r1-12): all four new tests fail on main's page (fold-r1-06: three of the five
  then on the first draft's); all pass after.
- **The mutations (run, fold-r1-05, 05b to 05d, 06, 12, 13).** Each caught: the
  simulator never refusing fails US1-S2 to S6; no retry (`RESTORE_WAIT` 0) fails US1-S1 and US1-S3; a
  retry of any refusal in any state fails 043's
  `a_focus_stop_on_a_silent_node_ends_and_reports_the_restore_failure`; `remember`
  keeping the left region fails US1-S4; the words unnamed fail US1-S2, S4, S5 and S6;
  `forget_left` doing nothing fails US1-S5; the carry uncapped and the exponent 0.5 fail
  US2-S1; Track `!= "off"` in place of `tracking`, no carry, and no `last.alt_deg` fail
  US3-S1; on the page, no kept picture and no line fail US4-S1, no readings while
  finding and readings while finding with none both fail US4-S2 (fold-r1-12), the old
  verdict words US4-S3, and no ` with no altitude` US4-S4.
  Read from the diff instead: the 502 rule alone and the `ready` rule alone each cover
  043's case without the other; FR-001's open end.
- **The gates (run, fold-r1-15).** Both gates on the final tip, 965ec262: `cargo fmt`,
  clippy, both `cargo doc`, nextest (1,538 of 1,538: 1,525 existing and the stories'
  13), the doc tests, and in `crates/finder/web` `npm ci --offline`, typecheck,
  Prettier, Vitest (611 of 611), the build, the dist check clean and `CI=1 npm run e2e`
  (34 of 34).
- **Probe round 1 from this text alone (run).** `probe/054-r1`: eb6e4b93 and a269fdad
  (US1), 5a4e6197 and ee9d093d (US2), fe40a82f and 21ac01f8 (US3), d8ca23bb and
  abf7e226 (US4), each story's tests committed first and failing before it as tasks.md
  says, each passing after it the first time, with plan.md § Interfaces' code as
  written and only each story's `writes`; every gate passing on abf7e226 (nextest
  1,538 of 1,538, Vitest 611 of 611, `CI=1 npm run e2e` 34 of 34); every `path:line`
  of the trio true at f6d4f5fd. 0 blocking and 10 clarity defects, folded here. A
  scenario bit each of its mutations of the four stories but these: diff-only (§ Read
  from the diff), equivalent (`left` kept after a try that took; the picture keeping
  its first frame), under the tolerance (the session read at `now`, inside 0.5°), no
  rig (the earlier part showing tonight's altitude), and five bitten only by throwaway
  cases, now steps.
  In the motion rig, with the focal length this fold adds, Track's target stands at
  42.963° (the simulator's 42.963°), s is 2.191, the record of 2.0 px (4.382″) carries
  to 5.386″ (factor 1.229, 5.4 % under the cap), and the median 6.191″ is 1.149 times
  it, `slightly_soft` (4.5 % over 1.10, 11.6 % under 1.30), 1.413 uncarried.
- **Probe round 1's fold (run).** Three test commits on `probe/054-r1`, history kept:
  99efbc63 (US1-S3's camera described again, US1-S4's refused PUT, US1-S5's frame; the
  pause rig hands back its simulator), 1eebf008 (US2-S2's past night, US2-S3's foreign
  rule) and 06ec58d0 (the motion rig's focal length), the tip. Each new step passed and
  bit under its mutation (fold-probe-r1-05): the simulator changing its settings
  before it refuses fails US1-S3 and US1-S5; the camera described again keeping the
  naming, US1-S3; a refused PUT forgetting it, US1-S4; the foreign rule keeping the
  altitudes, US2-S3; a past night showing the earlier nights' altitude, US2-S2;
  `carried.from` in pixels, US3-S1. US1's seven still fail on main's sources with the
  simulator's knob (fold-probe-r1-10), US3's three on US2's (fold-probe-r1-11). The
  focused commands of every story (US1's 24 and the simulator's 98 three times, US2's
  7 and US3's 3 three times, the console's 59, Vitest 4 and 72) and every gate on
  06ec58d0 (fold-probe-r1-07): `cargo fmt`, clippy, both `cargo doc`, nextest 1,538 of
  1,538, the doc tests, `cargo deny` offline, `npm ci --offline`, typecheck, Prettier,
  Vitest 611 of 611, the build, the dist check clean and `CI=1 npm run e2e` 34 of 34.
  Every worked number of spec.md and plan.md checked by hand against the formula
  (fold-probe-r1-08).
- **Read, not run.** That the field camera refused the put-back (the field logs never
  read `restore_error`); whether the field's INDI driver answers the abort switch late
  or early, and that INDI hands a driver its messages in order (no INDI source on this
  machine); decision 3 (b)'s effect on 031's view and its tests; the edge cases marked
  so in spec.md.

## Read from the diff, not from a test

The choices probe round 1 found unseen (a refusal that changes nothing, the camera
described again, a refused settings change, the altitudes the foreign rule drops and a
past night keeps, `carried.from` in the view's unit) are steps of US1-S3 to US1-S5,
US2-S2, US2-S3 and US3-S1 now, and so are not listed here; nor are `Kept::note`'s and
`Kept::reference`'s signatures, `FocusCheck` and the checklist's 18 steps, which
existing tests pin (§ Shapes that existing tests pin).

- `finish` holds no lock across the put-back's await, and the release follows it; it
  sets `held.focus.left` in the critical section that sets the lifecycle `ended` (the
  judge reads `crates/finder/src/imaging/focus/run.rs`).
- A refusal other than the scope node's 502 is not tried again (no rig makes a 400).
- `altitude` clones the session out of the imaging state's lock before
  `Session::snapshot`.
- 037's five step traits are not changed.
- `altitude` answers none at or below 0°: no rig puts Track's target under the horizon.
- A check notes no reading in the record, with or without an altitude (`if !run.check`
  around `note_at`, as around `note` at f6d4f5fd).
- `SimImagingCamera::apply` never refuses `ImagingSettings::default()`.
- `GLOSSARY.md`'s Focus record is in the words of § Interfaces.
- No file of 052 or 053 is in any story's diff beyond the shared ones of
  § Prerequisites 2.

## Shapes that existing tests pin

- `crates/finder/tests/console/imaging_abort_silent.rs:372`: 043 US1-S8, a focus stop
  on a silent node ends `stopped` with `restore_error` `the scope node is not
  connected` within its bound: kept by the 502 and `ready` rules.
- `crates/finder/tests/console/imaging_focus_aid.rs:1024`: a lost link gives the aid a
  `restore_error`.
- `crates/finder/tests/motion/imaging_run.rs:313`: a start with the observer's region
  of interest keeps 041's words.
- `crates/finder/tests/console/imaging_run_focus.rs:505`: the run's `focus.record` is
  the aid's `reference.value`; with no altitudes in its rig, uncarried.
- `crates/finder/tests/console/imaging_focus_record.rs:61`: `Mark { px, at }` by
  literal, and `Kept::note` and `Kept::reference` as 033 gave them, which US2's tests
  also call: their signatures cannot change.
- `crates/finder/tests/console/imaging_run_focus.rs:16`: `FocusCheck`
  (`crates/finder/src/imaging/run.rs:320`) by literal, so it gains no field.
- `crates/finder/tests/mcp/imaging_focus.rs:242`, `crates/finder/tests/mcp/imaging_focus.rs:246`
  and `crates/finder/tests/mcp/imaging_run.rs:133`: the MCP tools' descriptions, word
  for word.
- `crates/finder/tests/console/imaging_run_docs.rs:95`: the field checklist's 18 steps
  (`crates/finder/README.md:1759`), so it gains none.
- `crates/scope-node/tests/indi_expose.rs:782`: 030 US6-S6, the abort and the exposure
  answering in the instant of the call; untouched, since owner decision 1 (b) is set
  aside.
- `crates/finder/web/src/components/FocusBlock.test.tsx:267` (`Focus record: 5.70″
  tonight · 5.41″ on earlier nights.`, parts with no `alt_deg` and a `last` with none):
  why ` with no altitude` is written only while `last` has an `alt_deg`.
- `crates/finder/web/src/components/FocusBlock.test.tsx`,
  `crates/finder/web/src/components/ImagingPage.test.tsx` and
  `crates/finder/web/e2e/imaging.spec.ts` (its night red-only check): unchanged and
  passing (run).
- No test builds `SizeAt`, `LastView` or `ReferenceView` by literal, so their new
  fields break none.

## Decisions and the alternatives rejected

| Decision | Chosen (recommended) | Rejected |
|---|---|---|
| 1. The put-back | (a) the aid's put-back retried for the scope node's refusal (502) while `ready`, 1 s apart, within 5 s; the region a failed one left kept on the focus state, named, never kept by a later aid | (b) the node waits for the abort's answer, with naming (trial-built: covers only a late answer); (c) (b) with a short retry; (d) the retry without the left region; (e) naming only; (f) the run re-applies its settings (trial-built); (g) a fixed node wait |
| 2. The altitude | (a) the record keeps its altitude, carried by airmass^0.6 to the reading's, the factor held between 1 and 1.30, Track's target's while tracking; records compared as read | (b) quadrature with a seeing share; (c) a note only; (d) the full carry; (e) a record per band; (f) looser thresholds |
| 3. Between previews | (a) the page keeps its last picture with `Frame N, the last picture`, the Focus block its readings while finding | (b) `last_preview` in the view; (c) a Focus-block picture; (d) leave it |
| 4. Starless wording | (a) not here; capture the field's ASTAP output first | (b) read output with neither line as too few stars |
| 5. The unit | (a) a README sentence | (b) the MCP descriptions too (pinned by tests) |
| The retry's pause | 1 s, the INDI adapter's `RETRY_PAUSE` | a shorter pause: more calls to a camera that is stopping |
| The retry's bound | 5 s, 043's `ABORT_WAIT` | longer: a stop on a slow camera answers later still |
| What is tried again | the scope node's refusal (502) while `ready` | any refusal: a 400 cannot change, and 043's 409 must answer at once |
| Where the left region lives | the focus state, beside the record | the aid's run (the first draft): the next run replaces it |
| The altitude's source | Track's target while `tracking`, from the session's site | while Track is not `off` (lends a GoTo's target to a telescope not yet on it); the live solve's `SessionView.pointing.pointing.alt_deg` (`crates/finder/src/session.rs:680`, `crates/finder/src/session.rs:597`), readable without writing 053's files, but held at the last solved frame's time and absent while the live solve fails |
| The cap | 1.30, 033's `slightly_soft` line | none (the full carry); a smaller cap (the field's 6.7″ would read `clearly_out` at a cap under about 1.12) |
| The pause sentence | unchanged words, naming the carried size as the record | a carry clause (needs a field on 037's `FocusCheck`) |

## User Story 1 - The put-back and its words

The simulator's settle, `put_back` with its two constants and its 502 rule, the region
an aid left on the focus state with `forget_left`, `left_in` and `focus_left`,
`remember`'s rule, `Imaging::set`'s forgetting, the region refusal's second form, the
README's US1 sentences and their test file. No change to the record or the verdict.

## User Story 2 - The record's altitude and the carry

`SEEING_EXPONENT`, `CARRY_CAP`, `carry`, `Altitudes`, `Kept::note_at`, `reference_at`,
`altitudes` and the optional `focus.json` fields, with their test file. Nothing calls
them yet but the tests.

## User Story 3 - Judging at Track's altitude

`altitude`, the carried reference in `note`, `note_at` with the altitude, the view's
three fields, the README's US3 sentences, `GLOSSARY.md`'s Focus record and their test
file, over US2's API.

## User Story 4 - The page

`api.ts`'s fields, the kept picture and its line, the readings while finding, the
altitudes' words, the test file and the rebuilt `dist`.

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
2. **`main` is still as read, and the sibling drafts are ordered.** Every citation here
   was read at f6d4f5fd. Not yet checked at a flip. 052, 053 and 055, drafted beside
   this spec, write files this spec writes: 052's and 053's `writes` hold
   `crates/finder/README.md`, `crates/finder/tests/console/main.rs` and `GLOSSARY.md`;
   `probe/055` writes `crates/finder/README.md`, `crates/finder/tests/motion/main.rs`,
   `GLOSSARY.md`, `crates/finder/web/src/api.ts` and `crates/finder/web/dist`. The four
   land in the order 052, 053, 054, 055, and the operator seat re-anchors each one's
   `shared_writes` (§ Refinement Record) on whatever has landed before its flip, then
   re-reads the files § Project Structure and § Shapes name and re-runs the validator.
   Waits: 052 and 053 have not landed, and the re-anchor on their landed `main` comes
   after; every citation was true at f6d4f5fd on probe round 1.
3. **The owner decisions are settled and the owner approvals given** (spec § Owner
   decisions, § Owner approvals). Waits for the owner at the flip: none answered, each
   with its recommendation, to which the scenarios and the record are written.
4. **A cynical review and a probe round from this text alone**, their defects folded.
   Met: review r1 folded (0 blockers, 6 majors, 10 minors); probe round 1 built the
   four stories from this text alone (`probe/054-r1`, tip abf7e226: 0 blocking, 10
   clarity) and is folded, its new steps run on 06ec58d0 (§ What was run).
5. **The page half has run once.** Met on probe round 1's tip after this fold:
   `CI=1 npm run e2e` 34 of 34 on 06ec58d0 (fold-probe-r1-07), as on 965ec262
   (fold-r1-15) and abf7e226.
6. **No operator-staged files are needed.** Met: no story reads a file outside the
   repository.

## Outside the factory

Written, not run. The owner's, on the next field night, with the imaging camera on the
main telescope, Track and the imaging run as on 2026-11-26, the console at this spec's
landing.

1. **Make a record high.** On a target well above 60°, with Track tracking, run the
   focus aid to a found focus. On the Focus block the record line now reads `Focus
   record: … at N° tonight.`; note the size and N.
2. **A run low.** Pick a target at least 25° lower and start an imaging run. At its
   focus check, note the verdict, the reading and the Focus block's verdict line, which
   now names `as it reads at A°: V`. If it pauses, note the pause sentence and whether
   the record it names is V (at most 1.30 times the record line's size).
3. **Go anyway.** Answer the pause with `Go anyway`. The run should go on to the stack,
   with the imaging camera on its whole sensor (Imaging page, settings). Read
   `restore_error` from `get_focus_reading` either way: it settles owner decision 1's
   cause, which no log has shown. If the run ends, note the ending's words: `the focus
   aid could not put the imaging camera's settings back: …` gives the camera's own
   refusal. Clear any region with `Use the whole sensor`.
4. **Watch the picture.** During the aid and the run, note whether the Imaging page's
   picture stays between previews with `Frame N, the last picture` under it, and the
   Focus block's readings while it says `Finding the focus region on a full imaging
   frame`.
5. **The starless wording (owner decision 4).** With the cap on, take one imaging frame
   and run a focus check; note its words. If it says `the imaging solver's measure could
   not be read`, keep the frame and the ASTAP output the scope node logged for it: the
   follow-up's evidence.
6. **The seeing share, and the design changes, in a follow-up spec, only if** two
   records at altitudes 20° or more apart with the same focus (the focuser untouched
   between them) show how much of the size grows with the airmass; or a check judged
   against a carried record calls a focus that the aid alone finds good `clearly_out`
   or the reverse, on two nights; or `Go anyway` still ends a run after a pause.

## Plan traps

1. **Story sizes.** Probe round 1 built each story from this text alone, each measured
   as `git diff <story base> -- . ':(exclude)crates/finder/web/dist' | wc -c` (US1 from
   a2a65162, each later story from the previous story's commit): US1 29,959 B (45.7 % of
   65,536 B), US2 14,305 B (21.8 %), US3 25,299 B (38.6 %), US4 14,101 B (21.5 %; its
   `dist` adds 697 B of binary-diff lines), 93.7 % to 100.7 % of the review fold's
   estimates. With this fold's commits' bytes added to their stories (fold-probe-r1-12):
   US1 36,812 B (56.2 %, 93.6 % of the 60 % line, 39,321 B), US2 15,905 B (24.3 %), US3
   26,790 B (40.9 %), US4 unchanged. Those bytes count each hunk's context; written
   once, with its test file whole, US1 is 31,285 B (47.7 %), US2 15,165 B, US3 25,422 B.
   At the factory's worst overrun seen, 1.35 times, US1 is 49,696 B (75.8 %; 42,235 B,
   64.4 %, written once), over the line; the others stay under (US3 55.2 %). The
   factory's builds of 050 and 051 ran 0.78 to 1.04 times their probes, which puts US1 at
   38,284 B at most (58.4 %; 49.6 % written once), and 051's US2 flipped whole at the
   same probe size (29,952 B). So US1 is kept whole. US1's test file is about half its
   diff (15.3 KB, about 5.3 KB of it the copied rig). **The split, if the owner prefers
   it at the flip:** a story after US1 takes US1-S4 to US1-S6 and US1-S3's last step
   (the camera described again), with the region on the focus state (`left`, `left_in`,
   `forget_left`, `describe`'s clearing, `remember`'s rule, `finish`'s setting of `left`,
   `Imaging::set`'s forgetting and `focus_left` answered from `left`), and US1-S3 then
   ends at the stop's answer. Split so, the new story writes its own test file with its own
   copy of the pause rig, one `mod` line, `focus/run.rs`, `imaging/mod.rs` and
   `run/console.rs`. Until it lands, `focus_left` answers the latest run's
   `restore_error` while the settings are not whole. It measures about 15 KB (S4 to S6
   and the step about 4.6 KB, the region code about 5 KB, the rig copy 5.3 KB), leaving
   US1 about 22 KB.
2. **`FocusCheck` gains no field.** Tests build it by literal
   (`crates/finder/src/imaging/run.rs:320`); the carry stays in the aid's view, and the
   pause sentence is unchanged in its words (FR-010).
3. **The 18-step checklist.** `imaging_run_docs.rs` counts its steps; the README's new
   words go in prose, never in the checklist.
4. **MCP descriptions are pinned.** The words reach MCP through the views it already
   returns; no tool's description changes.
5. **043's bound.** Only the scope node's refusal of a `ready` camera is tried again; a
   silent node's refusal is the last at once.
6. **Paused time.** Every new Rust test runs under `start_paused = true`; the put-back's
   pause is `tokio::time::sleep`, so the stop's 1 s and 5 s are exact in paused time.
   US1-S3's windows (0.9 s to 1.5 s at a settle of 0.5 s, 4.9 s to 6.0 s at 10 s) keep
   clear of the tries at 0, 1 and 2 s whether the settle's end is open or closed.
7. **The simulator's settle is per abort, on tokio's clock.** It uses
   `tokio::time::Instant`, so paused time moves it; a default apply is never refused.
8. **The pause rig is private.** `rig` in `crates/finder/tests/console/imaging_run_focus.rs:363`
   builds its own camera and is not `pub`, and `tests/support/` is closed to writes:
   copy its lines (about 90) into the new file with the settle added, as the probe did.
9. **The motion rig's sync and Track's GoTo.** US3's tests wait for a first sync before
   Track; without a sync Track refuses. Track's first GoTo keeps it `armed` for about
   18 s of paused time, so US3-S1's first check, made at once, ends while it is armed.
10. **The dist rebuild.** US4 commits `dist` as `npm run build` makes it; the gate checks
    `git status --porcelain -- dist` is clean.
11. **The kept picture is set during render.** `ImagingPage` sets `kept` while
    rendering when the preview changes, guarded by `kept?.src !== frame.preview`, so it
    renders at most once more.
12. **052's and 053's files.** None of their own files is written. The altitude reads
    the session through `HeldSession` and `Session::snapshot`, both 032's, and
    `session.rs` is not edited.
13. **A stale build after a mutation.** Restoring a file by moving a copy back gives it
    an older time than the last build; touch it, or cargo runs the mutated code (seen
    on the probe). A mutation that leaves a name unused fails to build under
    `-D warnings` rather than failing a test.

## Risks

| Risk | Effect | Mitigation |
|---|---|---|
| The record's size is part seeing, part optics, in an unmeasured share | Carried, it may still be too strict low down, or too lenient | The factor is held between 1 and 1.30; § Outside the factory step 6 measures the share |
| A record read low judges a high target as read | A soft focus high up can read `good` (6.5″ at 70° against 6.1″ read at 37°, 1.066) | The next record made high replaces it when smaller; the aid's own readings say |
| Below about 38° from a record made at 74° the cap holds | A low target pauses a little sooner than full seeing theory says | `clearly_out` stays reachable (7.77″ for 4.6″), which is the cap's purpose |
| The record is the smallest settled reading, so it reads low | Carried, it still sets a strict line (the field night's 6.7″ reads `slightly_soft`) | Unchanged from 033; the verdict compares, and the aid is beside it |
| The scope node refuses for another reason while ready | The stop answers up to about 5 s later | The refusal's own words still reach `restore_error` |
| The field camera's refusal is inferred, never seen | The fix may not be the field's cause | The named words and `restore_error` say the camera's own refusal on the next night (§ Outside the factory step 3) |
| The exponent rests on no local source | A wrong exponent mis-carries | Owner decision 2; a constant, one place to change, and the cap bounds it |
| US1 at 1.35 times is over 60 % (75.8 %; 64.4 % written once) | A builder overrun could crowd the judge's diff | Builds of 050 and 051 ran at most 1.04 times their probes (58.4 %); § Plan traps 1 names the split the owner may choose at the flip |

## Refinement Record

Drafted on 2026-11-27 by the plate-solver operator seat's spec session against `main` at
f6d4f5fd, from a probe of that day on `probe/054` (the repro on the factory's rig, a
trial build of the four stories with every scenario's test and the gates passing, each
story's tests failing before it, the mutations, decision 1 (d) of the first draft
trial-built and set aside, the verdict's candidates measured on the field night's
numbers, and ASTAP on starless frames), and a reading of 033's aid, put-back, record and
verdict and its Focus block, 031's lease and Imaging page, 037's and 041's run, focus
gate, pause and region refusal, 043's silent-node bound, the INDI adapter's abort, the
simulator's imaging camera, MCP's tools, the constitution and the survey of 2026-11-15.
Cynical review r1 folded the same day, with the fold's probe commits (the region an aid
left, the 502 rule, the capped one-sided carry, Track `tracking` only, the page's
words) and decision 1 (b) trial-built on the scripted INDI server and set aside.
Probe round 1 built the four stories from this text alone the same day (`probe/054-r1`,
tip abf7e226: 0 blocking, 10 clarity), folded with three test commits there (tip
06ec58d0, every gate passing): the steps of US1-S3 to US1-S5, US2-S2, US2-S3 and the
motion rig's focal length, in the assertions below. Writes
come from the task text. Shared writes name the spec story that last wrote the same file
on `main`; within the spec, each story's files written by an earlier one are ordered by
`depends_on_merged`; 052, 053 and 055 are re-anchored at the flip (§ Prerequisites 2).

None of the five owner decisions of spec.md is answered; this record is written to
their recommendations, and approves nothing. The flip is the owner's. No open question
is left: each choice is an owner decision with a recommendation.

```yaml
version: 1
reviewed_revision: f6d4f5fd482ccc9d5b634ef1aa5124c112769583
stories:
  US1:
    tests:
      - path: crates/finder/tests/console/imaging_run_focus_restore.rs
        status: new
        disposition: add
        assertion: "the pause rig (imaging_run_focus.rs's rig copied, console::camera and console::focus, SimImagingCamera with_abort_settle 0.5 s, served by support::mcp::app), no focus record: POST /api/imaging/run {} 200; at run.state focus_pause with focus.aid.state running, readings >= 1, frame.state exposing and frame.purpose focus (run.focus.verdict no_record, settings.roi not whole): POST /api/imaging/run/continue {anyway: true} 200; once run.state is stacking or ended: run.state stacking, settings.roi x 0 y 0 and the camera's width and height, focus.aid without restore_error"
        decision: "spec.md:US1-S1"
      - path: crates/finder/tests/console/imaging_run_focus_restore.rs
        status: new
        disposition: add
        assertion: "the pause rig with with_abort_settle 10 s, going on anyway mid-exposure as in US1-S1: run.state ended, run.ended.reason camera_failed; focus.aid.restore_error 'the scope node did not apply the settings: the camera refused the settings: it is still stopping an exposure'; run.ended.message 'Ended: the imaging camera failed: the focus aid could not put the imaging camera's settings back: the scope node did not apply the settings: the camera refused the settings: it is still stopping an exposure.'; MCP get_imaging_run result.structuredContent.run.ended.message the same message; MCP get_focus_reading result.structuredContent.aid.restore_error the same restore words"
        decision: "spec.md:US1-S2"
      - path: crates/finder/tests/console/imaging_run_focus_restore.rs
        status: new
        disposition: add
        assertion: "the pause rig at settle 0.5 s and a fresh one at 10 s, no record: POST /api/imaging/focus {tile: {col 1, row 1}} 200; at focus.aid.state running, frame.state exposing, frame.purpose focus, settings.roi not whole: DELETE /api/imaging/focus answers 200 with focus.aid.ended_reason stopped; at 0.5 s within 0.9 to 1.5 s of paused time, no restore_error, settings.roi whole; at 10 s within 4.9 to 6.0 s, restore_error 'the scope node did not apply the settings: the camera refused the settings: it is still stopping an exposure', settings.roi as while the aid ran; then the simulator unresponsive until state is not ready and responsive until ready: settings.roi not whole, POST /api/imaging/run {} answers 409 with error 'the imaging camera has a region of interest set: clear it before an imaging run'"
        decision: "spec.md:US1-S3"
      - path: crates/finder/tests/console/imaging_run_focus_restore.rs
        status: new
        disposition: add
        assertion: "the pause rig at settle 20 s, the aid alone on tile (1, 1) exposing: DELETE /api/imaging/focus 200 with restore_error REFUSED; a console check (POST /api/imaging/focus {check: true}, until focus.aid.check true and state ended) ends ended_reason failed with settings.roi as while the aid ran; PUT /api/imaging/settings {roi: null} answers 502; POST /api/imaging/run {} answers 409 with error 'the focus aid could not put the imaging camera's settings back: ' and REFUSED; 21 s after the DELETE a console check ends done with settings.roi whole; POST /api/imaging/run {} answers 200"
        decision: "spec.md:US1-S4"
      - path: crates/finder/tests/console/imaging_run_focus_restore.rs
        status: new
        disposition: add
        assertion: "the pause rig at settle 10 s, the aid alone exposing with settings.roi x 412 y 274 412 x 274: DELETE /api/imaging/focus 200 with restore_error REFUSED; 11 s after it POST /api/imaging/frame, and once that frame is saved its width 412 and height 274; POST /api/imaging/run {} answers 409 with error 'the focus aid could not put the imaging camera's settings back: ' and REFUSED; PUT /api/imaging/settings {roi: {x 412, y 274, width 412, height 274}} 200; POST /api/imaging/run {} again answers 409 with error 'the imaging camera has a region of interest set: clear it before an imaging run'"
        decision: "spec.md:US1-S5"
      - path: crates/finder/tests/console/imaging_run_focus_restore.rs
        status: new
        disposition: add
        assertion: "the pause rig at settle 20 s: ready; POST /api/imaging/run {} 200; at run.state focus_pause, focus.aid.state running, frame.state exposing, frame.purpose focus: POST /api/imaging/run/continue {anyway: false} 200; at run.state focus_pause again with focus.aid.check false, focus.aid.state ended and focus.aid.since later than the first pause's: settings.roi as at the first pause; POST continue {anyway: true} 200; at run.state ended: run.ended.reason camera_failed, run.ended.message 'Ended: the imaging camera failed: the focus aid could not put the imaging camera's settings back: ' and REFUSED and a full stop"
        decision: "spec.md:US1-S6"
      - path: crates/finder/tests/console/imaging_run_focus_restore.rs
        status: new
        disposition: add
        assertion: "crates/finder/README.md with line breaks read as spaces: ## Focus up to the next ## heading contains 'is tried again once a second for up to 5 s' and 'the next aid or check puts the whole sensor back'; ## An imaging run up to ### Field checklist contains '`the focus aid could not put the imaging camera's settings back:`'"
        decision: "spec.md:US1-S7"
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
      - "041's region refusal (owner decision 24) refuses a start or re-hold with a region in force; only its words change while a region an aid's put-back left is in force"
      - "SimImagingCamera at its default settle of zero is f6d4f5fd's camera"
      - "037's step traits and FocusCheck are unchanged"
      - "the scope node and its INDI adapter are unchanged"
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
      - crates/finder/src/imaging/mod.rs
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
      - path: crates/finder/src/imaging/mod.rs
        other_spec: 050-tools-page-aligns-finder-camera
        other_story: US2
        disposition: merge_order
        reason: "054 depends_on_landed 050, whose US2 last changed the imaging state (500c7574)"
      - path: crates/finder/src/imaging/run/console.rs
        other_spec: 041-imaging-run-on-the-rig
        other_story: US8
        disposition: merge_order
        reason: "054 depends_on_landed 041, whose US8 last changed the run's console parts (61b54a47)"
      - path: crates/finder/README.md
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US3
        disposition: merge_order
        reason: "054 depends_on_landed 051, whose US3 last changed the README (9424a371); re-anchored at the flip on 052 and 053 (plan.md Prerequisites 2)"
      - path: crates/finder/tests/console/main.rs
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US2
        disposition: merge_order
        reason: "054 depends_on_landed 051, whose US2 last changed console/main.rs (f9706cc0); re-anchored at the flip on 052 and 053 (plan.md Prerequisites 2)"
    open_questions: []
  US2:
    tests:
      - path: crates/finder/tests/console/imaging_focus_carry.rs
        status: new
        disposition: add
        assertion: "carried(P, F, A) = P min(1.30, max(1, (X(A)/X(F))^0.6)) with X(h) = 1/sin h: carry(4.6, 74, 74) == 4.6; carry(4.6, 74, 60) within 1e-12 of carried(4.6, 74, 60) and strictly between 4.6 and 4.6 x 1.30; carry(4.6, 74, 20) within 1e-12 of 4.6 x 1.30; carry(6.1, 37, 70) == 6.1; judge(Ok(6.7), Some(4.6)) and judge(Ok(6.4), Some(4.6)) ClearlyOut; judge(Ok(6.4), Some(carry(4.6, 74, 38))) Good; judge(Ok(7.3), Some(carry(4.6, 74, 37))) SlightlySoft; judge(Ok(8.5), Some(carry(4.6, 74, 37))) ClearlyOut"
        decision: "spec.md:US2-S1"
      - path: crates/finder/tests/console/imaging_focus_carry.rs
        status: new
        disposition: add
        assertion: "a Kept on a scratch focus.json described with the simulated camera, T 1768528800: note_at(3.0, T, None, false, Some(74)) true, reference_at(T+1, None) (3.0, Tonight, Some(74)), file record.tonight.alt_deg 74; note_at(2.9, T+60, .., Some(60)) true; note_at(3.1, T+120, .., Some(30)) false, compared as read; altitudes(T+121, None).tonight Some(60); a second Kept on the file reference_at(T+121, None) (2.9, Tonight, Some(60)); on the first Kept note_at(3.2, T+43260, .., None) true, reference_at(T+43261, None) (2.9, Earlier, Some(60)), file record.earlier_alt_deg 60 and no record.tonight.alt_deg; note(2.5, T+43300, None, false) true, reference_at(T+43301, None) (2.5, Tonight, None); a fresh Kept on focus-past.json after note_at(3.0, T, None, false, Some(74)): reference_at(T+43201, None) (3.0, Earlier, Some(74))"
        decision: "spec.md:US2-S2"
      - path: crates/finder/tests/console/imaging_focus_carry.rs
        status: new
        disposition: add
        assertion: "a scratch folder of its own, a fresh Kept on focus-scaled.json: note_at(3.0, T, Some(0.55), false, Some(74)) true; altitudes(T+1, Some(0.55)).tonight Some(74); altitudes(T+1, Some(1.1)) Altitudes::default(); note_at(5.0, T+60, Some(1.1), false, None) true; reference_at(T+61, Some(1.1)) (5.0, Tonight, None); a fresh Kept on focus-foreign.json: note_at(3.0, T, Some(0.55), false, Some(60)), note_at(3.5, T+43260, Some(0.55), false, Some(70)), then altitudes(T+43261, Some(0.55)).earlier Some(60); note_at(4.0, T+43300, Some(1.1), false, Some(50)), then altitudes(T+43301, Some(1.1)) Altitudes { tonight: Some(50), earlier: None }; 033's own focus.json written by hand there (format imaging-focus/1, the simulated camera block, no region, record.tonight px 3.3 at and since T-800, no altitudes): another Kept's reference_at(T, None) (3.3, Tonight, None) and reference(T, None) (3.3, Tonight)"
        decision: "spec.md:US2-S3"
      - path: crates/finder/tests/console/imaging_focus_record.rs
        status: existing
        disposition: preserve
        assertion: "033's record: Kept::note and Kept::reference with their signatures, Mark { px, at } by literal, nights, the passed mark and the foreign scale as 033 words them"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "Kept::note and Kept::reference keep their signatures and behaviour; judge and its thresholds are unchanged"
      - "records are compared as read; focus.json's format stays imaging-focus/1, and 033's files read as before"
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
        reason: "054 depends_on_landed 051, whose US2 last changed console/main.rs (f9706cc0); re-anchored at the flip on 052 and 053 (plan.md Prerequisites 2)"
    open_questions: []
  US3:
    tests:
      - path: crates/finder/tests/motion/imaging_focus_altitude.rs
        status: new
        disposition: add
        assertion: "the motion rig (SimMount, pushed to alt 50 az 120, target M42, with_focal_length_mm 1800 so that s, the view's arcsec_per_px, is about 2.19, a first sync, the aim at the eyepiece pixel) with focus.json by hand: record.tonight px 2.0 at START - 600 and alt_deg 74; POST /api/track {on: true} and at once a check: ended_reason done while track.state is still armed, focus.aid.reference without carried, focus.aid.last without alt_deg; then track.state tracking and 15 s, a check: ended_reason done; reference.carried.from_alt_deg 74, carried.alt_deg within 0.5 deg of sim.telescope().eyepiece.alt, carried.from within 1e-9 of 2.0 s; focus.aid.last.alt_deg equals carried.alt_deg; reference.value within 1e-9 relative of 2.0 s min(1.30, max(1, (X(alt)/X(74))^0.6)); verdict good at most 1.10, slightly_soft at most 1.30 times that, clearly_out beyond, and slightly_soft; median.value above 1.30 x 2.0 s; focus.record.tonight.alt_deg 74"
        decision: "spec.md:US3-S1"
      - path: crates/finder/tests/motion/imaging_focus_altitude.rs
        status: new
        disposition: add
        assertion: "the motion rig with no record, focus error 0.5: a check has no focus.aid.reference; POST /api/imaging/focus {} until focus.record.tonight is an object: no record.tonight.alt_deg; DELETE; Track on and 15 s, focus error 0, POST {find: true}: focus.record.tonight.alt_deg comes and is within 0.5 deg of sim.telescope().eyepiece.alt"
        decision: "spec.md:US3-S2"
      - path: crates/finder/tests/motion/imaging_focus_altitude.rs
        status: new
        disposition: add
        assertion: "crates/finder/README.md with line breaks read as spaces: ## Focus contains 'grows as the airmass to the power 0.6', 'judged against the record carried to its own altitude', 'never by less than 1 or more than 1.30' and 'the far-out limit is about 11″'; ## An imaging run up to ### Field checklist contains 'against the record carried to the target's altitude'"
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
      - "the run's pause sentence's words and FocusCheck are unchanged"
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
      - GLOSSARY.md
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
        reason: "054 depends_on_landed 051, whose US3 last changed the README (9424a371); re-anchored at the flip on 052 and 053 (plan.md Prerequisites 2)"
      - path: GLOSSARY.md
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US3
        disposition: merge_order
        reason: "054 depends_on_landed 051, whose US3 last changed GLOSSARY.md (9424a371); re-anchored at the flip on 052 and 053 (plan.md Prerequisites 2)"
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
        assertion: "ImagingPage with fetch stubbed: ready, frame 1 saved with preview /api/imaging/frames/1/preview.jpg: in the region Imaging frame the img Imaging frame 1 has that src; frame 2 exposing, no preview: once the status reads 'Exposing for 1000 ms' the img Imaging frame 1 is still there with the text 'Frame 1, the last picture'; frame 2 with preview /api/imaging/frames/2/preview.jpg: img Imaging frame 2 with that src, no img Imaging frame 1 and no text holding 'the last picture'"
        decision: "spec.md:US4-S1"
      - path: crates/finder/web/src/components/FocusView.test.tsx
        status: new
        disposition: add
        assertion: "FocusBlock, unit arcsec at 0.55, aid finding with readings 9 and median, last and best 6.7: in the region Focus the text '6.70″' and 'Finding the focus region on a full imaging frame'; rendered afresh with an aid finding with readings 0 and no median, last, best or history: the region Focus has no text 'Focus reading'"
        decision: "spec.md:US4-S2"
      - path: crates/finder/web/src/components/FocusView.test.tsx
        status: new
        disposition: add
        assertion: "FocusBlock, record.tonight value 4.6 alt_deg 74, aid running, median 6.7, last.alt_deg 37, verdict slightly_soft, reference value 5.98 of tonight carried {from 4.6, from_alt_deg 74, alt_deg 37}: in the region Focus the text 'Focus record: 4.60″ at 74° tonight.' and 'Slightly soft: 12% larger than the focus record of 4.60″ at 74° tonight, as it reads at 37°: 5.98″.'"
        decision: "spec.md:US4-S3"
      - path: crates/finder/web/src/components/FocusView.test.tsx
        status: new
        disposition: add
        assertion: "FocusBlock, record.tonight value 4.6 without alt_deg, aid running, median 6.7, last.alt_deg 37, verdict clearly_out, reference value 4.6 of tonight not carried: in the region Focus the text 'Focus record: 4.60″ with no altitude tonight.' and 'Clearly out: 46% larger than the focus record of 4.60″ tonight.'"
        decision: "spec.md:US4-S4"
      - path: crates/finder/web/src/components/FocusBlock.test.tsx
        status: existing
        disposition: preserve
        assertion: "033's Focus block words, with no alt_deg anywhere and no carried, unchanged"
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
        reason: "054 depends_on_landed 051, whose US4 last rebuilt dist (f6d4f5fd); 055 rebuilds it after this spec (plan.md Prerequisites 2)"
      - path: crates/finder/web/src/api.ts
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US4
        disposition: merge_order
        reason: "054 depends_on_landed 051, whose US4 last changed api.ts (f6d4f5fd); 055 changes it after this spec (plan.md Prerequisites 2)"
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
