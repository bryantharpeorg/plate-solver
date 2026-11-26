# Implementation Plan: The Tools Page Aligns the Finder Camera

**Branch**: `050-tools-page-aligns-finder-camera` | **Date**: 2026-11-25 |
**Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/050-tools-page-aligns-finder-camera/spec.md`

## Summary

The console gains a Tools page and, as its first block, the finder alignment. While it
runs it holds 031's lease on the imaging camera and takes one whole-sensor imaging frame
after another, 500 ms each unless asked otherwise. Each frame is paired with the finder
fix nearest the middle of its exposure, solved with one hint (that fix's centre, a 10°
radius and the field height 032 would give), and placed in the fix's frame: pixels,
degrees right and up along the finder camera's own frame, the angle in all, on the frame
or not, in its middle or not. The frame says Done once the centre has been in the middle
for two frames in a row. Every solve is handed to 032's aim point through
`Imaging::note_solve`, its FR-037, and 032's word for that frame is shown beside the
place. A start is refused while Track is on or without a field height, and Track turned
on ends a run. It moves nothing; the owner turns the ring screws. The stop puts the
camera's settings back.

Five stories: the place off the frame and the simulator change that lets a test point
the imaging camera there; the run, its start route, view and refusals; the stop with the
settings put back and only the last frame kept, Done, and the refusals for Track and a
missing field height; the two MCP tools, the README and the glossary; and the Tools
page with Setup's link. The owner chose the live alignment (option A); the seat split
it into these five for size (§ Plan traps 2).

## Technical Context

**Language/Version**: Rust, MSRV 1.96, stable in CI; TypeScript and React for the page,
as built today.

**Primary Dependencies**: none new. `Cargo.toml`, `Cargo.lock`, `package.json` and
`package-lock.json` do not change.

**Storage**: the alignment's imaging frames and solve sidecars under the imaging folder,
only the last kept (FR-005). Nothing is stored in `session.json`.

**Testing**:
- `crates/finder/tests/console/imaging_align_centre.rs` (new, US1): `centre_of` on
  centre frames of the test's own making, and the simulator pointed off the frame
  (US1-S1 to US1-S5), under `#[test]`.
- `crates/finder/tests/console/imaging_align.rs` (new, US2): the alignment rig with its
  `pub struct Setup` (`live`, `solver`, `imaging`, `focal_mm`, `interval_ms`, `folder`;
  the default is the rig; `interval_ms` and `folder` are set only by US3's tests, and
  `rig` reads both), its `pub struct Rig { pub app: Router, pub sim: Simulator, .. }`
  and `pub fn rig(e: (f64, f64), setup: Setup) -> Rig`, its helpers `call`,
  `wait_until`, `ready`, `last_after`, `start`, `oracle` and `assert_centre`, all `pub`,
  and US2-S1 to US2-S6 under `#[tokio::test(start_paused = true)]`. A private `Rig`
  fails `private_interfaces` under `-D warnings`, and a `pub` field no US2 test reads
  fails `dead_code`, so the rig exposes no imaging folder: US3-S1 hands it a folder of
  the test's own through `Setup::folder` and lists that.
- `crates/finder/tests/console/imaging_align_stop.rs` (new, US3): US3-S1 and US3-S2,
  over US2's helpers, used and not edited.
- `crates/finder/tests/console/imaging_align_aim.rs` (new, US3): US3-S3 to US3-S7, over
  US2's helpers, used and not edited.
- `crates/finder/tests/motion/imaging_align_track.rs` (new, US3): US3-S8 and US3-S9
  with the Track rig, in the motion binary because Track moves the mount (the console
  binary's doc says no mount moves there).
- `crates/finder/tests/mcp/imaging_align.rs` (new, US4): US4-S1, with its own rig as the
  MCP binary's tests build theirs, and US4-S2, the README read as
  `crates/finder/tests/console/imaging_run_docs.rs` reads it, under `#[test]`.
- `crates/finder/web/src/components/AlignmentBlock.test.tsx` and `ToolsPage.test.tsx`
  (new, US5): US5-S1 to US5-S6 under Vitest; `crates/finder/web/e2e/tools.spec.ts`
  (new, US5): US5-S7 under Playwright's simulate project.

**Target Platform**: the console on the Pi or a desktop, the page in a phone's browser.

**Constraints**:
- 031's lease, frames and words, 032's solve, hint, relation and rules, and the focus aid
  are used and not changed (FR-009, FR-015).
- `RUSTFLAGS="-D warnings"`, `--all-features` clippy, and rustdoc with warnings denied.
- No test uses a camera, a mount, ASTAP or the network.

**Scale/Scope**: five stories; one new source module and four changed Rust files, one
simulator line, six new Rust test files, two new page components and their tests, one
browser test, the README section and seven glossary entries.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Detection and the plate solver are untouched; the finder camera's solves are the live loop's own. The place is gnomonic arithmetic on 032's frame, checked against the simulator's truth; what rests on reading (a camera not ready, an exposure in flight, a failed frame, a solve from the Imaging page during a run) is listed in § What was run and what was read. |
| II. Never weaken a gate | No existing test, golden, fixture, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS` or `ergane.yaml` is edited. `playwright.config.ts` gains two list entries, an owner approval as 035's were. The only edits to existing test files are three `mod` lines (SC-002). The simulator's relaxed assert widens what a test may ask of it and changes no existing test's observation (run: the workspace's 1,490 existing tests passed on round 1's tip and on the re-cut's, beside the alignment's own). |
| III. Tests first, independent expectations | The oracles are the simulator's eyepiece pixel and the pinhole arithmetic of its finder camera's optics, 1456 × 1088 and 12° wide, the centre frame's own geometry, the drawing's own arithmetic, 031's and 032's words, and the scenarios' words. No test calls `centre_of` or reads the module's constants for an expected value. Time comes from a paused tokio clock and the simulator's virtual clock. |
| IV. Critical paths | None is touched: no `Cargo.toml` or `Cargo.lock` change, no `unsafe`, nothing under `proto/`, `crates/math-core`, `crates/pattern-database` or `crates/database-generation`, no `build.rs`. `crates/simulator` is not a critical path; its one change is owner approval 1. |
| V. Proven open source first, honest licensing | The imaging solve is ASTAP's command-line solver behind 032's port, as today, and the gates reach it through the simulated adapter. Looked for, in local sources only: the repository's survey of 2026-11-15, § 5 "Relating two cameras" (`docs/research/2026-11-15-imaging-camera.md:179`), with its sources on StarSense, the Nexus eFinder, PiFinder and Cedar, SharpCap, CCDciel, KStars and N.I.N.A.; the vendored `reference-solutions/` (cedar-detect, cedar-solve); and the installed `astap-cli` package's documents. What exists stores a two-number offset set once by a person who centres a star (`docs/research/2026-11-15-imaging-camera.md:181`), or solves with the imaging camera alone (`docs/research/2026-11-15-imaging-camera.md:186`); none re-measures by itself (`docs/research/2026-11-15-imaging-camera.md:187`), and none pairs two cameras' solves live to guide a physical turn of a finder's screws. The cynical review's own search agreed. What this needs is 032's pairing and projection, which exist here, so nothing is adopted. No copied source, no new licence. |

## Project Structure

```text
crates/simulator/src/optics.rs                  # US1: eyepiece_ray asserts a finite pixel
crates/finder/
├── README.md                                   # US4: rows, MCP paragraphs, ## Finder alignment
├── src/
│   ├── imaging/align.rs                        # US1 new: the place; US2: the run; US3: stop, aim, done, refusals
│   ├── imaging/mod.rs                          # US1: pub mod align; US2: Held's align, ImagingView's align, Drop
│   ├── imaging/routes.rs                       # US2: POST /api/imaging/align; US3: DELETE
│   └── mcp/tools.rs                            # US4: get_finder_alignment, set_finder_alignment
├── tests/
│   ├── console/imaging_align_centre.rs         # US1, new
│   ├── console/imaging_align.rs                # US2, new
│   ├── console/imaging_align_stop.rs           # US3, new
│   ├── console/imaging_align_aim.rs            # US3, new
│   ├── console/main.rs                         # US1, US2: one mod line each; US3: two
│   ├── motion/imaging_align_track.rs           # US3, new
│   ├── motion/main.rs                          # US3: one mod line
│   ├── mcp/imaging_align.rs                    # US4, new
│   └── mcp/main.rs                             # US4: one mod line
└── web/
    ├── playwright.config.ts                    # US5: two list entries
    ├── e2e/tools.spec.ts                       # US5, new
    ├── dist/                                   # US5: rebuilt
    └── src/
        ├── api.ts                              # US5: the alignment's types and calls
        ├── App.tsx                             # US5: /tools
        └── components/
            ├── SetupPage.tsx                   # US5: the Tools link
            ├── ToolsPage.tsx                   # US5, new
            ├── ToolsPage.test.tsx              # US5, new
            ├── AlignmentBlock.tsx              # US5, new
            └── AlignmentBlock.test.tsx         # US5, new
GLOSSARY.md                                     # US4: Finder alignment; Lease; Purpose label; four Avoid lines
```

## Interfaces

The code as it ran: round 1's (`probe/050-r1` at 9709494c) re-cut for five stories
(`probe/050-r1-split` at 2cb196a6). Bodies not given here are the builder's, to these
words and the FRs.

### `crates/simulator/src/optics.rs` (US1)

`Optics::eyepiece_ray` (`crates/simulator/src/optics.rs:50`): its assert
(`crates/simulator/src/optics.rs:52`) becomes

```rust
        // Any finite pixel: off the frame is an eyepiece the camera does not see (050).
        assert!(x.is_finite() && y.is_finite());
```

### `crates/finder/src/imaging/align.rs` (US1 new; US2 and US3 add)

```rust
//! The finder alignment (050): imaging frames under the imaging camera's
//! lease, each solved and paired with the finder camera's live solve nearest
//! its middle, and where the imaging camera's centre lies from the finder
//! camera's centre, as the finder camera sees it, off its frame too. It moves
//! nothing. Every solve is offered to 032 through `Imaging::note_solve`.

/// The name the finder alignment holds the imaging camera's lease under. (US2)
pub const HOLDER: &str = "the finder alignment";
/// The purpose label of every imaging frame it takes. (US2)
pub const PURPOSE: &str = "align";
/// Its exposure when the request names none: with Track off a star at the
/// celestial equator trails about 7 px of ASTAP's prepared copy in it. (US2)
pub const ALIGN_EXPOSURE_MS: f64 = 500.0;
/// A frame is paired with the finder fix nearest the middle of its exposure,
/// no more than this many seconds from it. (US2)
pub const PAIR_S: f64 = 5.0;
/// Every hint names the paired fix's centre, with this radius. (US2)
pub const SEARCH_RADIUS_DEG: f64 = 10.0;
/// The middle of the finder camera's frame: within this share of its width,
/// and of its height, of its centre. (US1)
pub const MIDDLE_SHARE: f64 = 0.25;
/// Done once this many frames in a row put the centre in the middle. (US3)
pub const DONE_FRAMES: u64 = 2;
/// The words of a start refused, and of a run ended, while Track is on. (US3)
const TRACK_ON: &str = "Track is on; turn it off while the finder camera's screws are turned";

/// Where the imaging camera's centre lies in the finder camera's frame. (US1)
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Centre {
    /// The finder camera's pixels on the pinhole model, outside the frame
    /// when it falls outside.
    pub x: f64,
    pub y: f64,
    pub finder_width: f64,
    pub finder_height: f64,
    /// Right of the frame's centre along its rows (left when negative), and up
    /// along its columns (down when negative).
    pub right_deg: f64,
    pub up_deg: f64,
    /// The two centres' separation on the sky.
    pub separation_deg: f64,
    /// 032's own test (its FR-020): the place, distortion included, inside the frame.
    pub on_frame: bool,
    /// On the frame and within `MIDDLE_SHARE` of its width and height of its centre.
    pub middle: bool,
}

/// The imaging centre `ra_deg`, `dec_deg` in the fix's frame; `None` behind
/// the finder camera. (US1)
pub fn centre_of(fix: &FinderFix, ra_deg: f64, dec_deg: f64) -> Option<Centre> {
    let centre = UnitVector::from_radec(ra_deg.to_radians(), dec_deg.to_radians());
    let frame = &fix.frame;
    let (x, y) = frame.project_pinhole(centre)?;
    let (w, h) = frame.size();
    let focal = (w / 2.0) / (frame.fov_deg().to_radians() / 2.0).tan();
    let on_frame = frame
        .project(centre)
        .is_some_and(|place| frame.contains(place, 0.0));
    Some(Centre {
        x,
        y,
        finder_width: w,
        finder_height: h,
        right_deg: ((x - w / 2.0) / focal).atan().to_degrees(),
        up_deg: ((h / 2.0 - y) / focal).atan().to_degrees(),
        separation_deg: angular_distance(frame.center, centre).to_degrees(),
        on_frame,
        middle: on_frame
            && (x - w / 2.0).abs() <= MIDDLE_SHARE * w
            && (y - h / 2.0).abs() <= MIDDLE_SHARE * h,
    })
}

/// The body of `POST /api/imaging/align`. (US2)
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlignAsk {
    #[serde(default)]
    pub exposure_ms: Option<f64>,
}

/// The run's last imaging frame. (US2; US3 adds `aim` and `done`)
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AlignLast {
    pub frame_id: u64,
    /// The middle of the exposure, console Unix seconds.
    pub at: f64,
    /// `measured`, `not_solved` or `no_live_solve`.
    pub outcome: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub centre: Option<Centre>,
    /// For a frame that did not solve: how many frames in a row, this one
    /// included, did not solve.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unsolved: Option<u64>,
    /// 032's `relation.last.outcome` for this frame. (US3)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aim: Option<String>,
    /// This frame and the `DONE_FRAMES` - 1 before it each put the centre in
    /// the middle. (US3)
    pub done: bool,
}

/// `align` in the imaging view: the latest run, running or ended. (US2)
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AlignView {
    /// `running` or `ended`.
    pub state: &'static str,
    pub since: f64,
    pub exposure_ms: f64,
    pub frames: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last: Option<AlignLast>,
    /// `stopped` or `failed`, once ended.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// What the imaging state holds of the finder alignment. (US2)
#[derive(Debug, Default)]
pub(in crate::imaging) struct State {
    run: Option<Run>,          // view, task, the count in a row unsolved (US2); the
                               // stop asked, the previous saved frame and the count
                               // in a row in the middle (US3)
    ended: Arc<Notify>,        // notified once a run has ended (US3)
}
impl State {
    pub(in crate::imaging) fn abort_task(&mut self);   // for Imaging's Drop
}
pub(in crate::imaging) fn view(held: &Held) -> Option<AlignView>;

/// 031's words for a camera that is not `ready`, copied from `Imaging::set`. (US2)
fn state_refusal(held: &Held) -> Refusal;

/// Whether the session's `track` reads `wanted` true, read with no lock held. (US3)
fn track_on(imaging: &Imaging) -> bool {
    let session = imaging.state.lock().expect("imaging state").session.clone();
    session.is_some_and(|session| {
        session.0.snapshot(imaging.clock.now()).track.is_some_and(|track| track.wanted)
    })
}

impl Imaging {
    /// FR-004's refusals in order, US3's two among them, the lease last; then the task. (US2)
    pub async fn align_start(self: &Arc<Self>, ask: AlignAsk) -> Result<ImagingView, Refusal>;
    /// FR-007: mark the stop, `self.abort()`, wait on `State::ended` until the
    /// run has ended; with nothing running, the view. (US3)
    pub async fn align_stop(&self) -> ImagingView;
    /// Whether a run is running. (US3)
    pub fn align_running(&self) -> bool;
}
```

`align_start` reads `track_on(self)` first, with no lock held; then its checks, under
one lock, then the lease outside it:

- `held.state != "ready"`: 031's words for the state, copied into `align.rs` as a
  private `state_refusal(held: &Held) -> Refusal` from
  `crates/finder/src/imaging/mod.rs:591` to `crates/finder/src/imaging/mod.rs:601`,
  since they live inside `Imaging::set` and cannot be called from `align.rs`;
- no solver: `this console has no imaging solver`;
- `held.align` running: `the finder alignment is already running`;
- Track on: `TRACK_ON` (US3);
- `held.relation.height_inputs() == (None, None)`: `the imaging camera's field height is
  unknown: start the console with --focal-length-mm, or solve an imaging frame on the
  Imaging page` (US3);
- `held.relation.fixes().last()` older than `solving::HINT_AGE_S` or absent:
  `the finder camera has no live solve`;
- `held.lease.is_none()` and `held.frame`'s state `exposing` or `receiving`:
  `an exposure is in progress` (as `crates/finder/src/imaging/mod.rs:615`);
- `exposure_ms` not finite or outside `held.camera`'s range: 400,
  `exposure_ms {N} is outside the camera's range {min}–{max} ms`;
- `self.lease(HOLDER)?`.

The task, `run(Weak<Imaging>, Lease, Arc<dyn ImagingSolver>, exposure_ms)`: in US2,
the steps, then drop the lease and mark the view `ended`, `stopped` or `failed` with
the error. US3 wraps the steps: first remember the exposure, the ROI (`None` when it is
the whole sensor) and `save_at_node`, and after them put the remembered settings back
with `lease.set`, before the lease is dropped, and notify `State::ended` once the view
is marked. The steps: `lease.set` of the exposure, `roi: Some(None)` and
`save_at_node: Some(false)`; then one frame after another (`lease.take(PURPOSE)`, then
`imaging.ended(id)`; `aborted` ends the steps with no error, `failed` with the frame's
error; `frames` counted), a `SolveFrame` of it, and the lines below. US3 adds, before
each frame, the stop's check (a stop asked ends the steps with no error, as it does
after `imaging.ended`) and Track's (`if track_on(imaging) { return
Some(TRACK_ON.into()); }`), and, once a frame is counted, the previous saved frame's
`.fits` and `.solve.json` removed unless 032's `solve` is `solving` it:

```rust
        // The finder fix nearest the middle of the exposure.
        let fix = held.relation.fixes().into_iter()
            .filter(|fix| (fix.time - middle).abs() <= PAIR_S)
            .min_by(|a, b| (a.time - middle).abs().total_cmp(&(b.time - middle).abs()));
        // none: note(imaging, frame.id, middle, Err(None)); continue
        // 032's field height; the place and radius the alignment's own.
        let mut hint = imaging.hint(&solve_frame);        // never with the lock held
        let (ra, dec) = fix.frame.center.to_radec();
        hint.source = HintSource::Pointing;
        hint.position = Some((ra.to_degrees(), dec.to_degrees()));
        hint.radius_deg = SEARCH_RADIUS_DEG;
        // solving::solve_frame(solver, clock, solving::Ask { root, file, frame_id,
        //     frame: solve_frame.clone(), hint, requested_at: clock.now() }).await.outcome
        // Ok(solve): imaging.note_solve(&solve_frame, frame.id, &solve);   (US3)
        //            note(.., Ok(centre_of(&fix, solve.ra_deg, solve.dec_deg)))
        // Err(error): note(.., Err(Some(error.message())))
```

`note` writes `last` under the lock, with the counts in a row:

```rust
    // US3: 032's word for this frame.
    let aim = held.relation.view(None, false)
        .and_then(|view| view.last)
        .filter(|last| last.frame_id == frame_id)
        .and_then(|last| serde_json::to_value(last.outcome).ok())
        .and_then(|value| value.as_str().map(str::to_string));
    // ...
    run.unsolved = if outcome == "not_solved" { run.unsolved + 1 } else { 0 };
    run.middle = if centre.is_some_and(|centre| centre.middle) { run.middle + 1 } else { 0 };  // US3
    run.view.last = Some(AlignLast {
        frame_id, at, outcome, error, centre,
        unsolved: (outcome == "not_solved").then_some(run.unsolved),
        aim,                                   // US3
        done: run.middle >= DONE_FRAMES,       // US3
    });
```

### `crates/finder/src/imaging/mod.rs` (US1; US2)

US1: `pub mod align;`. US2: `Held` gains `align: align::State`
(`align::State::default()` where `Held` is built); `ImagingView` gains
`#[serde(skip_serializing_if = "Option::is_none")] pub align: Option<align::AlignView>`,
`None` in the view of a console with no imaging state and `align::view(&held)` in every
other; `Imaging`'s `Drop` calls `held.align.abort_task()`.

### `crates/finder/src/imaging/routes.rs` (US2; US3)

US2: `.route("/api/imaging/align", post(post_align))` beside the focus aid's.
`post_align(State, SessionWrite, Result<Json<AlignAsk>, JsonRejection>)` takes its body
through `sim_json` (`crates/finder/src/api.rs:766`) as the focus aid's route does, so a
body that is not JSON answers 415; it answers 409 `this console takes no imaging frames`
without an imaging state, and maps a `Refusal` to
`ApiError::new(refusal.status, refusal.message)`. US3: the route gains
`.delete(delete_align)`; `delete_align(State, SessionWrite)` answers 409
`this console takes no imaging frames` without an imaging state, else
`Json(imaging.align_stop().await)`.

### `crates/finder/src/mcp/tools.rs` (US4)

`AlignArguments { on: bool, exposure_ms: Option<f64> }`, schema only, as the existing
schema-only argument structs are; `get_finder_alignment` (`read_only_hint = true`) and
`set_finder_alignment` as FR-013 words them, each reading `self.state.imaging()` first
(`this console takes no imaging frames`) and its arguments after, and answering
`json!({ "align": view.align })` (a `?` on `serde_json::to_value` does not compile there:
the tools' error is a `String`). Descriptions:

- `get_finder_alignment`: "Read the finder alignment: for its last imaging frame, where
  the imaging camera's centre lies from the finder camera's centre as the finder camera
  sees it (right_deg and up_deg along its frame, separation_deg on the sky, off its frame
  too), on_frame, middle, done once the centre has held in the middle for two frames in
  a row, and aim, the aim point's word for that frame. null before the first start."
- `set_finder_alignment`: "Start or stop the finder alignment, as POST and DELETE
  /api/imaging/align do: imaging frames under the imaging camera's lease, each solved and
  paired with the finder camera's live solve. It moves nothing: the observer turns the
  finder camera's ring screws, with Track off. Read get_finder_alignment every few
  seconds until done."

### The page (`crates/finder/web/src/`, US5)

- `api.ts`: `ImagingAlignCentre` (the nine keys of `Centre`), `ImagingAlign` (`state`,
  `since`, `exposure_ms`, `frames`, `last?` of `frame_id`, `at`, `outcome`, `error?`,
  `centre?`, `unsolved?`, `aim?`, `done`; `ended?`; `error?`), `postImagingAlign()`
  (POST, header `Content-Type: application/json`, body `"{}"`), `deleteImagingAlign()`
  (DELETE) and `ImagingView`'s `align?`.
- `App.tsx`: the path `/tools` renders
  `<ToolsPage session={session} onBack={() => navigate("/")} />`; `SetupPage` gets
  `onTools={() => navigate("/tools")}`; the frame picker's `onPick` navigates home from
  `/tools` as from `/setup` and `/imaging` (`crates/finder/web/src/App.tsx:308`).
- `components/SetupPage.tsx`: an optional `onTools` prop; under `Back to console` in the
  header, while `session.node && onTools`, `<p><a href="/tools" className="link">Tools</a></p>`
  with the same plain-click handling as `Back to console`.
- `components/ToolsPage.tsx`:
  `ToolsPage({ onBack }: { session: ObservingSession; onBack: () => void })` (the
  session is taken now for 051's block and read by no block of this spec); `useImaging()`;
  a header as Setup's (`border border-ink p-[3px]`, the `chart-title` heading `Tools`,
  `Back to console`), then `<AlignmentBlock view={view} refresh={refresh} />` whether or
  not `view` is read yet: each block waits for its own data.
- `components/AlignmentBlock.tsx`:
  - `export function centreLine(centre, done): string`: the place "The imaging camera
    points X° right (left) and Y° up (down) of the finder camera's centre, as the finder
    camera sees it: Z° in all" (one decimal each, `Math.abs` for the side); with `done`,
    `Done: the imaging camera's centre has held in the middle of the finder camera's frame. <place>.`;
    in the middle, `<place>, in the middle of the finder camera's frame. Done once the next frame holds it there.`;
    else `<place>, on (off) the finder camera's frame.`
  - `export function aimLine(aim): string`: for `measured` or `replaced`,
    `The aim point is measured from this frame (<aim>).`; else
    `The aim point is not measured from this frame (<aim>); a later imaging solve measures it.`
  - the not-solved line: `Imaging frame N did not solve: <error>.`, then, from
    `unsolved` 3 on,
    ` U frames in a row have not solved: the imaging camera may be more than 10° from the finder camera's centre, the sky may hold too few stars, or the field height may be wrong.`,
    then ` The next frame looks again.`
  - `Drawing`: an SVG 184 wide: the frame 160 wide at (12, 12), a 12 margin on every
    side, h = 160 · height / width, in the finder camera's aspect; its middle half
    dashed, a line from the centre to the imaging centre, a filled dot on the frame and a
    ring off it, all in `currentColor`.
    The point is held to the edge along the line from the centre: with
    dx = (x − width/2)·160/width and dy = (y − height/2)·h/height,
    t = min(1, 80/|dx|, (h/2)/|dy|) and the point (12 + 80 + t·dx, 12 + h/2 + t·dy).
  - `AlignmentBlock({ view, refresh })`, `view` possibly null: the section, labelled by
    its heading `Align the finder camera` (`section-title`), the line "Imaging frames
    show where the imaging camera points from the finder camera's centre. Turn Track
    off, then turn the finder camera's ring screws until it says Done. Nothing moves the
    telescope." (`quiet`); with no view yet `Reading the imaging camera.` (`quiet`) and
    no button, else the button (`button button-primary`); the status lines, the aim line
    (`quiet`) and the alerts of FR-019.

### The README (`crates/finder/README.md`, US4)

After the paragraph that ends "`list_calibration` answers" … (the MCP section, before
`### MCP under WSL2`):

```markdown
The Tools page's finder alignment has two tools:

- `get_finder_alignment` takes no arguments and reads `{"align": ...}`: the
  finder alignment's state and its last imaging frame, with where the imaging
  camera's centre lies from the finder camera's centre, whether it is done and
  the aim point's word for that frame; `align` is null before the first start.
- `set_finder_alignment` takes `on`, and with `on` true optionally `exposure_ms`;
  it starts or stops the finder alignment as `POST` and `DELETE /api/imaging/align`
  do and answers `{"align": ...}`, or the route's refusal as a tool error. It moves
  nothing: the observer turns the ring screws, with Track off, and an MCP client
  says which way.

On a console that takes no imaging frames both answer `this console takes no
imaging frames` at once, before reading their arguments.
```

After the `DELETE /api/imaging/focus` row of the API table:

```markdown
| `POST /api/imaging/align` | JSON `{}` or `{"exposure_ms": N}`. Start the finder alignment; answers 200 with the imaging view, whose `align` reports it, or 400, 409 or 415 with a JSON `error`. Same-origin writes only. |
| `DELETE /api/imaging/align` | No body. Stop the finder alignment, put the imaging settings back and answer 200 with the imaging view. Same-origin writes only. |
```

Before `## Calibration frames`:

```markdown
## Finder alignment

The Tools page, opened from Setup's `Tools` link while a scope node is paired,
holds utilities used once a setup. Its first is the finder alignment: while the
observer turns the finder camera's ring screws, it shows where the imaging
camera's centre lies from the finder camera's centre, as the finder camera sees
it, also when that is off the finder camera's frame, and says Done once that
centre has held in the middle of the frame for two frames in a row. It moves
nothing.

`POST /api/imaging/align` starts it, with the body `{}` or `{"exposure_ms": N}`.
It holds the imaging camera's lease as `the finder alignment`, remembers the
exposure, the region of interest and the save setting, and takes one
whole-sensor imaging frame after another at 500 ms or `exposure_ms`, under the
purpose label `align`, without a node copy; the binning stays as it is, and
2 × 2 sends a quarter of the bytes. Each frame's file and solve sidecar go once
the next frame is saved, but for the last, and not while a solve from the
Imaging page reads it. Each frame is paired with the finder camera's live solve
nearest the middle of its exposure, no more than 5 s from it; with none, its
`outcome` is `no_live_solve` and it is not solved. Otherwise it is solved with
a hint that names that live solve's centre, a 10° radius and the field height
an imaging solve would be given; a frame that does not solve says so with the
solver's words, and the next frame looks again. `align` in the imaging view
carries `state` (`running` or `ended`), `since`, `exposure_ms`, `frames`, `last`
and, once ended, `ended` (`stopped` or `failed`) and `error`. A measured frame's
`last.centre` carries `x` and `y` in the finder camera's pixels, outside the
frame when the centre is, `right_deg` and `up_deg` along the frame's own rows
and columns (the chart rotation is not applied), `separation_deg` on the sky,
`on_frame` and `middle` (within a quarter of the frame's width and height of its
centre); a frame that did not solve carries `last.unsolved`, how many frames in
a row have not. `last.done` is true once this frame and the one before it each
put the centre in the middle. Each solve is offered to the measured aim point as
`Imaging::note_solve` offers any, and `last.aim` is what that made of it,
`relation.last.outcome`'s word: `measured` or `replaced` when the aim point took
this frame's reading, and `set_aside`, `moving`, `no_live_pointing` or
`off_frame` when it did not, in which case a later imaging solve measures it.
`DELETE /api/imaging/align` stops it, puts the three settings back and frees
the camera; it waits for a solve in flight, which ASTAP's limit holds to 120 s.
It is refused while the camera is not ready, without an imaging solver, while
it already runs, while Track is on, without a field height, when the finder
camera has had no live solve for 30 s, during an exposure no lease holds, for an
exposure outside the camera's range, and while another procedure holds the
camera, with that procedure's name. Track turned on while it runs ends it,
`failed`.

Turn Track off first, and back on after: a turned screw moves the pointing the
console reads, and Track would follow it or drive the mount back. Point the
telescope at a field of stars both cameras solve. Give the imaging camera's
field height first: `--focal-length-mm`, or a scale kept by an imaging solve on
the Imaging page. After a change of optics, solve imaging frames on the Imaging
page until one solves before aligning: the alignment's frames that do not solve
count no misses, so they never move the field height to its next source, while
a solve on the Imaging page during a run counts as that page's own.
```

### The glossary (`GLOSSARY.md`, US4)

- **Purpose label**: "…`focus`, `dark`, `flat`, `sub`, `run` and `align`." (was "…`sub`
  and `run`.").
- **Lease**: "…the focus aid, a set of darks or flats, the stack, the imaging run or the
  finder alignment." (was "…the stack or the imaging run.").
- **Sync** (`GLOSSARY.md:57`): `_Avoid_: align, alignment (alone: the finder alignment
  turns the finder camera on the tube)`.
- **Pointing model** (`GLOSSARY.md:61`): `_Avoid_: star alignment, mount model,
  alignment (alone: see Finder alignment)`.
- **Zero-point epoch** (`GLOSSARY.md:73`): `_Avoid_: session, alignment (alone: see
  Finder alignment)`.
- **Relation** (`GLOSSARY.md:299`): `_Avoid_: alignment (the finder alignment is the
  procedure, not what is kept), calibration, offset`.
- New, before **Focus check**:

  ```markdown
  **Finder alignment**:
  Imaging frames taken while the observer turns the finder camera's ring screws, with Track off, each showing where the imaging camera's centre lies from the finder camera's centre, also off the finder camera's frame, until it holds in the middle of the frame.
  _Avoid_: relation (that is what the console keeps between the cameras), collimation, calibration
  ```

### What 051 adds to the Tools page

Spec 051's focus aid for the finder camera or the imaging camera adds a block below the
alignment's: its own component, given `session` (which `ToolsPage` takes from US5 on)
or `useImaging`'s view, waiting for its own data as `AlignmentBlock` does, a region named
by its own heading, and its browser checks scoped to that region. US5's files need no
rework for it: `ToolsPage.tsx` gains one line, and `tools.spec.ts`'s locators of the
alignment are held to the alignment's region, so a second status on the page breaks
none of them.

## What was run and what was read

**Run, round by round** (logs under the operator's probe folder, not in the
repository).

- **The draft's probe and fold round 1** (`probe/050`, cut from e2eee793): commits
  7df72ef5 (the console's trial), e35b5b2e (the page's trial), 89fedd14 (the draft's
  code split for four stories), 935a0699 (the cynical review's fold) and f351c4da (the
  field-height refusal's test moved to US3). On f351c4da: every scenario's test and
  both gates' commands (nextest 1,483 of 1,483 at that base); each new or changed FR's
  check mutated once, each mutation failing its scenario (FR-004's no-imaging 409,
  FR-005's exposure, sidecar and count in a row, FR-007's abort, FR-010's two frames,
  FR-011's two Track checks, FR-012, FR-013's order, and the page's drawing, causes
  line, view gate and night colours); Track's refusal and end in the motion binary; the
  field height refused with none and every frame `not_solved` at 2200 mm; the stop
  during a 60 s exposure within 5 s of paused time; the page by night with no pixel off
  night red, and its width at 412 px; ASTAP's command-line solver on the saved 0.5°
  frame (owner decisions 3 and 4, times as shares of the on-target solve at 10°):
  smeared 7, 15 and 30 px along its rows, a starless frame of its own noise at 10°, 30°
  and 180°, and a field height 20 % high at 10°, 30° and 180°; from the draft's
  commits, the off-frame place at the eyepiece pixels of US2-S1 to US2-S4 within 2 px
  and 0.02°, every solve handed to 032 with the imaging camera 6.6° and 8.5° out reading
  `off_frame`, ASTAP at hint offsets of 0° to about 130°, and the page in Playwright's
  Chromium at 360, 390 and 412 px wide against a Tools chip in the top bar.
- **Probe round 1** (`probe/050-r1`, cut from 0e21ddb7, `main` ea3d6a9b and the trio,
  tip 9709494c): a builder built the four stories of the trio before this fold from its
  text alone, tests first: 0 blocking and 15 clarity defects (050-R1 to 050-R15), each
  folded here, in spec.md or in tasks.md. Each story's tests failed before it; every
  gate `ergane.yaml` names passed after the last (nextest 1,510 of 1,510, the 1,490
  existing among them; Vitest 598 of 598; Playwright 33 of 33; the dist check clean).
  Each of its bite checks, one mutation of the code run against the scenarios, bit but
  for eight: seven claims no scenario read (050-R2 to R6, R14, R15, each then shown by a
  throwaway that passed on the code and failed under its mutation) and the solving
  frame's files removed anyway (below, § Read from the diff). Its sizes: US1 9,221 B
  (14.1 %), US2 40,814 B (62.3 %), US3 42,281 B (64.5 %), US4 26,081 B (39.8 %) and
  `dist` 697 B more.
- **This fold's re-cut** (`probe/050-r1-split`, cut from 0e21ddb7): round 1's code
  rebuilt as one commit per story of the five, by `git checkout <commit> -- <paths>`
  from `probe/050-r1` and hand edits where a piece moved inside a file, with the seven
  steps' tests added: US1 80338ccc, US2 570f44a6, US3 3006660b, US4 37e425c8 and US5
  2cb196a6. For each story, its tests failed before it as tasks.md says, and its
  focused commands passed after it (three times each; the browser test once alone and
  once in the gate), with `cargo fmt` and clippy clean (for US5, Prettier and the
  typecheck); its size, from the previous story's tip, is as § Plan traps 2 gives it.
  After US5, every gate `ergane.yaml` names passed: `cargo fmt --check`, clippy with
  `-D warnings`, both `cargo doc` runs, nextest 1,512 of 1,512 (the 1,490 existing and
  the alignment's 22), the doc tests, `npm ci --offline`, the typecheck, Prettier,
  Vitest 598 of 598 (the six new among them), the build, the dist check (clean) and
  `CI=1 npm run e2e` 33 of 33 (`tools.spec.ts` among them). Each of the seven new
  steps' code was mutated once on the last tip, and each step bit: 050-R4's distorted
  projection (US1-S3), R2's refusal removed (US2-S6), R3's whole sensor not set
  (US2-S1), R14's hint place dropped (US3-S6), R15's aim read from any frame (US3-S3),
  and R5's aim line and R6's first-frame status dropped (US5-S2).

**Read, not run:**

- that a pixel off the frame panics `eyepiece_ray` before US1's change (the assert's own
  condition, `crates/simulator/src/optics.rs:52`);
- that 032's own route solve, at its 3° radius, misses an imaging camera 3° to 10° out
  and walks its hint to the whole sky after misses (its radius, its sources and the
  simulated solver's refusal, read; ASTAP's miss at 12° with a 10° radius, run);
- the imaging run's, the stack's and the darks' and flats' refusals (031's FR-028); the
  focus aid's is run;
- the refusals of a camera that is not ready (031's words, copied);
- a failed exposure ending the run `failed`, and the console stopping mid-run;
- a solve from the Imaging page during a run, and the frame it holds kept;
- a stop waiting for a solve in flight (ASTAP's 120 s limit, read);
- a frame picked from the top bar while on `/tools`;
- trailed frames from the real imaging camera: the smear was made on a saved frame that
  holds fainter stars than a 500 ms frame will;
- how far a guide scope in a shoe or rings can sit off the main tube;
- the real cameras, the real mount and ASTAP on a live frame (§ Outside the factory).

## Read from the diff, not from a test

- The README's section, rows and paragraphs and the glossary's entries (FR-014), beyond
  US4-S2's sentence, paragraph and headings: the judge reads them in the diff. The
  existing docs test of `crates/finder/tests/console/imaging_run_docs.rs:139` still
  finds "sixteen tools of its own".
- FR-007's rule that a frame 032's own solve is `solving` keeps its files: the solving
  frame's files removed anyway survived every scenario in round 1, as said here. FR-008
  (nothing but the alignment's own frames written) beyond US3-S1's mount, settings and
  files checks.
- `ToolsPage`'s `session` prop (FR-017), which no block of this spec reads.

## Shapes that existing tests pin

- **`ImagingView`'s keys.** `align` is skipped while `None`, so every existing view
  assertion that reads a whole view passes unchanged (run: nextest 1,490 of 1,490
  existing on round 1's tip and on the re-cut's).
- **031's lease words** `the imaging camera is held by {holder}` and the focus aid's
  holder are used as they are; the existing lease tests pass unchanged (run).
- **032's relation.** `note_solve` is called as FR-037 offers it; 032's tests pass
  unchanged (run). The alignment's not-solved frames call nothing of 032's, so the
  misses and the field height's source are as before.
- **The simulator.** Every existing caller sets an eyepiece pixel inside the frame; the
  relaxed assert changes none of their observations (run).
- **The README's MCP count.** `crates/finder/tests/console/imaging_run_docs.rs:139`
  asserts "sixteen tools of its own" (`crates/finder/README.md:88`): the new tools are a
  paragraph of their own, and that sentence is not touched (run).
- **MCP's tool list.** No existing test counts the tools (run: the MCP binary's tests
  passed with two more, on both tips).
- **The motion binary.** One more `mod` line; its other tests pass unchanged (run).
- **The page.** Setup's header gains a link only with a scope node and an `onTools`
  prop; Setup's existing tests and the night-mode browser tests pass unchanged (run:
  Vitest 598 of 598 and Playwright 33 of 33 on round 1's tip and on the re-cut's).
- **The simulated console's state.** The browser test turns Track off and keeps an
  imaging scale on the shared simulate server, and its alignment measures the aim point;
  the specs that run after it in that project pass unchanged (run: 33 of 33, both
  tips).
- **`playwright.config.ts`.** The two entries put `tools.spec.ts` in the simulate project
  only; every other project's files are unchanged (run).

## Decisions and the alternatives rejected

The twelve owner decisions, all settled, are in spec.md § Owner decisions. Below
them:

| Decision | Chosen | Rejected |
|---|---|---|
| Where the run lives | `imaging/align.rs`, as the focus aid's run lives in `imaging/focus/run.rs` | inside `solving.rs` or `relation.rs` (032's modules, FR-015) |
| The key for the place | `centre` | `offset` (`GLOSSARY.md:299` avoids it) |
| What is kept on disk | the last frame and its sidecar | every frame (two a second at best while it runs) |
| Pairing | the fix nearest the exposure's middle within 5 s | 032's anchor inside the exposure and least-squares line (that is the aim reading's job; a short exposure from a slow finder camera would rarely pair) |
| Done's count | a run's count of frames in a row in the middle, any other frame starting it again | a window of the last frames (a second rule for the same word) |
| The Track check | before every frame, the session read with no lock held | a watch on the session (a second task to stop) |
| Where the Track tests live | the motion binary, whose tests move the mount | the console binary, whose doc says no mount moves there |
| The browser test's field height | a scale kept by one solve through the API first | `--focal-length-mm` in the simulate server's command (an edit to `playwright.config.ts` beyond its two entries), or `--simulate` giving one (a change to `main.rs` and to every simulate test's hints) |
| The start's live-solve check | the last fix within 30 s (`HINT_AGE_S`) | none (every frame would be `no_live_solve`) |
| The drawing | held to the frame's edge along the line from its centre, a ring off it | each axis held on its own (off a corner it points tens of degrees wrong), or a wider canvas |
| The story cut | five stories, each under 60 % of 65,536 B: the stop moved from the run to the story of Done, and MCP, the README and the glossary a story of their own (§ Plan traps 2) | the four of round 1, whose run (62.3 %) and Done (64.5 %) stories passed 60 % and grew with the new steps; a raised budget; a refusal moved instead of the stop (too few bytes) |
| Where the stop's tests find the imaging folder | `Setup::folder`, a folder of the test's own that `rig` reads, as it reads `interval_ms` | a `pub` field of the rig that no US2 test reads (`dead_code` under `-D warnings`), or a copy of the rig in US3's file |

## User Story 1 - The place off the frame

`centre_of` and the simulator's relaxed assert, with their test file. No route, no view.

## User Story 2 - The run

The start's route, the loop, the view, the refusals but Track's and the field
height's, and the count of frames in a row that do not solve, over US1's `centre_of`.
A failed frame ends the run; nothing else does until US3. The test file's helpers are
`pub`, for US3's tests.

## User Story 3 - The stop, Done and the night's refusals

The stop and its route, the settings remembered and put back at every end, only the
last frame kept (all three moved here from round 1's US2 for size, since US3-S1 and
US3-S2 are the only scenarios that read them), `aim` and `done` on `last`,
`note_solve` called, and the Track and field-height refusals. Its console tests use
US2's helpers and do not edit US2's file. It takes the field-height refusal from US2,
which measured 85 B over 60 % of the budget with it in the draft's round.

## User Story 4 - MCP and the words

The two tools, the README and the glossary, which round 1 built at 22,884 B of its US3,
and the README's check (US4-S2).

## User Story 5 - The page

The Tools page, the block, Setup's link, the browser test and the rebuilt `dist`.

## Verification (every story, before proposing)

- **The factory gates** (`ergane.yaml` at ea3d6a9b):
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
   on `main` at ea3d6a9b, the base of both probe rounds.
2. **`main` is still as read.** Every citation here was read at ea3d6a9b. If `main`
   moves in a file § Project Structure names, or in a test § Shapes names, re-read them
   and re-run the validator. Met: `main` was ea3d6a9b on 2026-11-26 when this fold ran
   the validator, and every `path:line` added or touched here was read there.
3. **The owner decisions are settled and the owner approvals given** (spec § Owner
   decisions, § Owner approvals). Met: the optic and the scope on 2026-11-25, and
   decisions 1 to 10 answered (a) on 2026-11-26 ("Agree with your pending qs btw"); the
   approvals are given by the flip, which is the owner's.
4. **Probe builds.** A probe round builds the stories from this text alone, and its
   defects are folded. Met: round 1 (`probe/050-r1`, tip 9709494c) built the four
   stories of the trio before this fold from its text alone, 0 blocking and 15 clarity,
   all folded; the five-story cut was then proven on round 1's code
   (`probe/050-r1-split`, tip 2cb196a6), each story under 60 % with its tests failing
   before it and every gate passing after US5 (§ What was run and what was read).
5. **The page half has run once.** Met on the probes: `npm test`, the typecheck,
   Prettier, the build, the dist check and `CI=1 npm run e2e`, on round 1's tip and on
   the re-cut's (§ What was run and what was read).
6. **No operator-staged files are needed.** Met: no story reads a file outside the
   repository, and both probe rounds ran from a clean clone with `npm ci --offline`.

## Outside the factory

Written, not run. The owner's, on the real rig, at night, with the SV165's objective in
front of the finder camera (about 1.72° by 1.29°, with the database built for its
field, 047).

1. **Before the first night.** Give the imaging camera's field height: start the console
   with `--focal-length-mm 1800`, or solve one imaging frame on the Imaging page first
   so that 032 keeps its scale. Without one the start is refused.
2. **A rough start.** By day or at dusk, sight along the guide scope at the object
   centred in the main telescope's eyepiece, or centre a bright star in the eyepiece at
   night and turn the rings until it shows in the finder camera's preview on Setup.
   Neither camera solves a daytime scene, so the tool itself is for the night.
3. **At night.** Turn Track off. Point at a field rich in stars well above the horizon.
   On Setup, check that the finder camera solves (`pointing.source` live); open
   **Tools**, **Start aligning**. Each line says which way and how far the imaging
   camera points from the finder camera's centre, as the finder camera sees it. Turn the
   ring screws so that the finder camera points that way: the imaging camera's centre
   moves toward the middle. A screw turned mid-frame costs the next frame. The field
   drifts while Track is off; move the telescope back to stars by hand when the frames
   stop solving.
4. **Lock it while it reads right.** This finder has no springs: when the line says the
   centre is in the middle, tighten the ring screws against each other with the
   alignment still running, and watch the next frames hold it there (Done stays) after
   the screws are locked. If tightening moves it, loosen, turn back and lock again. Only
   then **Stop aligning**, and turn Track back on.
5. **If a frame says "did not solve"** three times running: the line names the causes:
   the imaging camera more than 10° out (move the guide scope by eye first), too few
   stars (clouds, or a field too sparse), or a wrong field height (solve on the Imaging
   page to check it).
6. **The aim point.** The line beside the place says whether 032 took the frame's
   reading. If it never did (a slow finder camera, `no_live_pointing`), a later imaging
   solve measures it, as before.
7. **After a bump** of the finder camera: open Tools again; the alignment and 032's aim
   point follow.

## Plan traps

They bind every story.

1. **Gate evidence or nothing.** The judge credits only gate results and the story's
   diff. *Check:* every Then names a committed test the `test` gate runs.
2. **One evidence budget.** *Check:* each story's whole diff stays under 60 % of
   65,536 B (39,321 B), measured with
   `git diff <base> -- . ':(exclude)crates/finder/web/dist' | wc -c` from the previous
   story's tip, on code built from the trio. Round 1 built the trio's four stories at
   US1 9,221 B (14.1 %), US2 40,814 B (62.3 %), US3 42,281 B (64.5 %) and US4 26,081 B
   (39.8 %), `dist` 697 B more, so the scope was cut into five: the stop, the settings
   put back and the last-frame rule moved from the run to US3 (the only piece of US2
   that its own scenarios S7 and S8 alone read, and enough bytes, where a refusal was
   not), and MCP, the README and the glossary (round 1: 22,884 B) became US4. Round 1's
   code re-cut for the five, with the seven new steps: US1 9,765 B (14.9 %), US2
   33,821 B (51.6 %), US3 33,974 B (51.8 %), US4 24,049 B (36.7 %) and US5 26,662 B
   (40.7 %), `dist` 697 B more; all five 128,271 B (round 1's four: 111,840 B; the
   seven steps, the README's check, the stop's own file and one more story's diff
   context make the difference). At the 1.35 times a factory build has run over its
   probe on earlier specs, the largest is about 45,865 B (70.0 %, US3), under the
   128 KiB refusal.
3. **A spec fixed mid-epic needs a relaunch.** *Check:* no builder edits
   `specs/050-tools-page-aligns-finder-camera/` or `.specify/`.
4. **An earlier story's test file is an existing test.** *Check:* US2 to US5 write no
   file of an earlier story's tests; US3's console tests use US2's `pub` helpers, and
   US3-S1 hands the rig its folder through `Setup::folder`.
5. **No new support file.** *Check:* nothing new under `crates/finder/tests/support/`.
6. **An oracle copied from the code proves nothing.** *Check:* the tests write the
   pinhole arithmetic, the pixels, the drawing's arithmetic and the words as literals;
   none calls `centre_of` or reads `MIDDLE_SHARE`, `PAIR_S`, `SEARCH_RADIUS_DEG`,
   `ALIGN_EXPOSURE_MS` or `DONE_FRAMES` for an expected value.
7. **A negative assertion passes when nothing happens.** *Check:* "no `centre`" in
   US2-S3 and US2-S5 stands beside `outcome` and `state`; "`done` false" in US3-S3 and
   US3-S4 beside the frame that says true; "no `aim`" in US3-S3 beside its `not_solved`
   outcome; "no link" in US5-S6 beside the heading awaited; "no button" in US5-S3
   beside `Reading the imaging camera.`; the night screenshot's 0 beside the day one's
   count.
8. **The frame in flight at a turn was taken before it.** *Check:* US2-S4 and US3-S4 skip
   one measured frame after each `set_optics`, or read until the condition holds, as
   US3-S3 and US3-S6 do.
9. **The lock is never held across a call into the session.** *Check:* `Imaging::hint`
   and the Track check are called with no lock held, as `Imaging::hint`'s doc says
   (`crates/finder/src/imaging/solving.rs:628`).
10. **Paused time and the simulator's clock.** *Check:* every Rust test of this spec but
    US1's runs under `start_paused = true`, and every wait is bounded (120 s of paused
    time, 300 s in the motion binary, at most 20 frames read in a loop).
11. **The dist is rebuilt.** *Check:* US5 commits `crates/finder/web/dist` after
    `npm run build`.
12. **An unused helper fails clippy.** *Check:* US2's helpers are each used by US2's own
    tests, and `Setup`'s `interval_ms` and `folder`, which only US3's tests set, are read
    by `rig`; the rig has no `pub` field that no US2 test reads; US3's and US4's files
    hold only what they use.
13. **rustdoc denies warnings.** *Check:* no doc comment holds an angle bracket outside
    backticks.
14. **Each worktree builds on its own.** *Check:* no story sets `CARGO_TARGET_DIR`.
15. **The README's MCP count is pinned.** *Check:* US4 leaves "sixteen tools of its own"
    as it is and puts the two tools in their own paragraph (US4-S2).
16. **The browser test runs on a shared server.** *Check:* US5-S7 turns Track off and
    keeps a scale itself, and every locator of the alignment is held to the region
    `Align the finder camera`; the heading and the night switch are not, being the
    page's and the top bar's; its waits stay well under the 60 s test timeout
    (`crates/finder/web/playwright.config.ts:29`).
17. **Playwright's project flag takes `=`.** *Check:* the development command is
    `npx playwright test --project=simulate e2e/tools.spec.ts`; with a space the file
    name is read as a second project.

## Risks

| Risk | Mitigation |
|---|---|
| A 500 ms frame from the real imaging camera holds too few stars, or trails more than the smear tried. | `exposure_ms` names another; § Outside the factory step 5 names the causes. The smear's frame held fainter stars than a 500 ms frame will (read). |
| A failing frame costs about one solve; a stop waits for it. | The 10° radius bounds it (1.07 times an on-target solve, measured); ASTAP's 120 s limit holds every placed hint. |
| A solve from the Imaging page during a run counts 032's misses. | The README says so; the alignment keeps the frame that solve reads. |
| A story grows past 60 % in the factory. | The five measure 51.8 % (US3, 33,974 B) at most on round 1's code; at the usual overrun the largest stays well under the 128 KiB refusal (§ Plan traps 2). |
| The real finder camera's frame may be mirrored or rotated in its rings. | Right and up are along the frame as the camera sees it, which turns with the rings and not with the night; Setup's chart rotation is not applied (owner decision 2). |

## Refinement Record

Drafted on 2026-11-25 by the plate-solver operator seat's spec session against `main` at
e2eee793, from a probe of that day (a trial build of the four stories with every
scenario's test and both gates passing, ASTAP on a saved frame, and the page measured in
a browser), a reading of 031's lease and frames, 032's solve, hint and relation, 033's
focus aid, 038's push check, the simulator's optics, the page's navigation, MCP's tools,
the constitution, the glossary and the ADRs. Folded the same day, round 1, from a cynical
review, against `main` at ea3d6a9b, on the probe's commits 935a0699 and f351c4da (every
gate passing; ASTAP timed on starless, trailed and failing frames; each new or changed
FR's check mutated once). Folded again on 2026-11-26 from probe round 1
(`probe/050-r1`, tip 9709494c: four stories built from the trio alone, 0 blocking and
15 clarity) and the owner's answers, with the scope re-cut into five stories and proven
on round 1's code (`probe/050-r1-split`, tip 2cb196a6, every gate passing). Writes come
from the task text. Shared writes name the spec story that last wrote the same file on
`main`; within the spec, each story's files written by an earlier one are ordered by
`depends_on_merged`.

All twelve owner decisions of spec.md are settled, decisions 1 to 10 answered (a) on
2026-11-26; this record is written to those answers, and its `open_questions` are
empty. This record approves nothing; the flip is the owner's.

```yaml
version: 1
reviewed_revision: ea3d6a9b0db7fa26f81a0e90d8705ff1ffe8a7d7
stories:
  US1:
    tests:
      - path: crates/finder/tests/console/imaging_align_centre.rs
        status: new
        disposition: add
        assertion: "the centre frame at RA 100 Dec 20, centre_of for RA 100 Dec 21: x 728 and y 544 - f tan 1 deg within 1e-6 px, right_deg 0, up_deg 1, separation_deg 1 within 1e-9 deg, on_frame and middle true, finder_width 1456, finder_height 1088"
        decision: "spec.md:US1-S1"
      - path: crates/finder/tests/console/imaging_align_centre.rs
        status: new
        disposition: add
        assertion: "the centre frame at RA 100 Dec 20, centre_of for RA 100 Dec 25: y 544 - f tan 5 deg within 1e-6 px and below 0, up_deg 5 within 1e-9 deg, on_frame and middle false"
        decision: "spec.md:US1-S2"
      - path: crates/finder/tests/console/imaging_align_centre.rs
        status: new
        disposition: add
        assertion: "the centre frame at RA 100 Dec 0, centre_of for RA 108 and RA 96 at Dec 0: the first right_deg -8 and up_deg 0 within 1e-9 deg, x 728 - f tan 8 deg within 1e-6 px, on_frame false; the second right_deg and separation_deg 4 within 1e-9 deg, on_frame true, middle false; with distortion -0.2 at RA 100 Dec 0, centre_of for RA 108 Dec 0 has x 728 - f tan 8 deg within 1e-6 px"
        decision: "spec.md:US1-S3"
      - path: crates/finder/tests/console/imaging_align_centre.rs
        status: new
        disposition: add
        assertion: "the centre frame at RA 100 Dec 20, centre_of for RA 280 Dec -20, behind the camera: None"
        decision: "spec.md:US1-S4"
      - path: crates/finder/tests/console/imaging_align_centre.rs
        status: new
        disposition: add
        assertion: "a simulator pushed to alt 50 az 120, set_optics with eyepiece_px (-300, 420): no panic, and the angle between imaging_truth and telescope().camera is atan(hypot(1028, 124) / f) within 1e-6 deg"
        decision: "spec.md:US1-S5"
      - path: crates/finder/tests/console/imaging_relation.rs
        status: existing
        disposition: preserve
        assertion: "032's relation: aim readings from imaging solves, off_frame outside the finder camera's frame, the kept scale, the hint's sources and the misses, as 032 words them"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "every existing simulator caller's eyepiece pixel inside the frame observes what it did (the simulator's other code unchanged)"
      - "032's relation, solve and hint are used, not changed (032 FR-020, FR-037)"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console -E 'test(imaging_align_centre)'"
        - "cargo nextest run -p simulator --locked"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - crates/simulator/src/optics.rs
      - crates/finder/src/imaging/align.rs
      - crates/finder/src/imaging/mod.rs
      - crates/finder/tests/console/imaging_align_centre.rs
      - crates/finder/tests/console/main.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/simulator/src/optics.rs
        other_spec: 036-tracking-from-subs
        other_story: US1
        disposition: merge_order
        reason: "050 depends_on_landed 036, whose US1 last changed the simulator's optics (8b13dcaf)"
      - path: crates/finder/src/imaging/mod.rs
        other_spec: 037-imaging-run
        other_story: US3
        disposition: merge_order
        reason: "050 depends_on_landed 037, whose US3 last changed imaging/mod.rs (581be8c3)"
      - path: crates/finder/tests/console/main.rs
        other_spec: 047-console-uses-the-database-that-fits
        other_story: US1
        disposition: merge_order
        reason: "050 depends_on_landed 047, whose US1 last changed console/main.rs (d3f8bd0f)"
    open_questions: []
  US2:
    tests:
      - path: crates/finder/tests/console/imaging_align.rs
        status: new
        disposition: add
        assertion: "the alignment rig at (700, 420), ready, its region set by PUT /api/imaging/settings roi 0 0 618 410 (200): POST /api/imaging/align {} answers 200 with align exactly state running, since, exposure_ms 500.0, frames 0; the first measured frame's centre x and y within 2 px of 700 and 420, right_deg up_deg separation_deg within 0.02 deg of the pinhole oracle, finder_width 1456, finder_height 1088, on_frame true; the imaging view's frame purpose align, settings exposure_ms 500.0, roi 0 0 1236 822, save_at_node false"
        decision: "spec.md:US2-S1"
      - path: crates/finder/tests/console/imaging_align.rs
        status: new
        disposition: add
        assertion: "three alignment rigs at (-60, 420), (-300, 420) and (1800, 100), each ready and started: each first measured frame's x and y within 2 px of the eyepiece pixel, right_deg up_deg separation_deg within 0.02 deg of the pinhole oracle, on_frame false"
        decision: "spec.md:US2-S2"
      - path: crates/finder/tests/console/imaging_align.rs
        status: new
        disposition: add
        assertion: "the alignment rig at (-1100, 420), 14.8 deg out, and the rig at (700, 420) with focal length 2200 mm, each started and read until last.unsolved is 3: align.state running, frames 3, outcome not_solved, error exactly the imaging solver found no solution, no centre"
        decision: "spec.md:US2-S3"
      - path: crates/finder/tests/console/imaging_align.rs
        status: new
        disposition: add
        assertion: "the alignment rig at (-300, 420), started, its first measured frame read; a screw turned to (1800, 100), one measured frame skipped, the next within 2 px and 0.02 deg of its pinhole oracle with on_frame false; then (900, 700) likewise with on_frame true"
        decision: "spec.md:US2-S4"
      - path: crates/finder/tests/console/imaging_align.rs
        status: new
        disposition: add
        assertion: "the alignment rig at (700, 420), started, its first measured frame read; PUT /api/session camera_settings exposure_ms 1 gain 1 answers 200; a later frame's outcome is no_live_solve with no centre while align.state is running"
        decision: "spec.md:US2-S5"
      - path: crates/finder/tests/console/imaging_align.rs
        status: new
        disposition: add
        assertion: "POST /api/imaging/align: no imaging state 409 this console takes no imaging frames; without the live loop 409 the finder camera has no live solve; without a solver 409 this console has no imaging solver; with all, {} as text/plain 415, exposure_ms 1e9 400 exposure_ms 1000000000 is outside the camera's range 1–3600000 ms, x 1 400; after PUT /api/imaging/settings exposure_ms 60000 (200) and POST /api/imaging/frame (200), {} 409 an exposure is in progress; once that frame is saved, with the focus aid started 409 the imaging camera is held by the focus aid; after it stopped 200, then 409 the finder alignment is already running, and POST /api/imaging/focus and POST /api/imaging/frame each 409 the imaging camera is held by the finder alignment"
        decision: "spec.md:US2-S6"
      - path: crates/finder/tests/console/imaging_focus_aid.rs
        status: existing
        disposition: preserve
        assertion: "the focus aid takes 031's lease, remembers and puts back the settings, and refuses and is refused with the holder's name as 033 words it"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "031's lease, frames, settings and words are used as they are (031 FR-028 to FR-031)"
      - "032's solve_frame, Imaging::hint and the relation are called, not changed"
      - "the imaging view's other keys are unchanged; align is skipped while None"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console -E 'test(/^imaging_align::/)'"
        - "cargo nextest run -p finder --locked --test console -E 'test(imaging)'"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs and the night of plan.md § Outside the factory"
    writes:
      - crates/finder/src/imaging/align.rs
      - crates/finder/src/imaging/mod.rs
      - crates/finder/src/imaging/routes.rs
      - crates/finder/tests/console/imaging_align.rs
      - crates/finder/tests/console/main.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/imaging/routes.rs
        other_spec: 037-imaging-run
        other_story: US3
        disposition: merge_order
        reason: "050 depends_on_landed 037, whose US3 last changed imaging/routes.rs (581be8c3)"
      - path: crates/finder/src/imaging/mod.rs
        other_spec: 037-imaging-run
        other_story: US3
        disposition: merge_order
        reason: "050 depends_on_landed 037, whose US3 last changed imaging/mod.rs (581be8c3)"
      - path: crates/finder/tests/console/main.rs
        other_spec: 047-console-uses-the-database-that-fits
        other_story: US1
        disposition: merge_order
        reason: "050 depends_on_landed 047, whose US1 last changed console/main.rs (d3f8bd0f)"
    open_questions: []
  US3:
    tests:
      - path: crates/finder/tests/console/imaging_align_stop.rs
        status: new
        disposition: add
        assertion: "the alignment rig at (700, 420), its imaging folder the test's own: DELETE with nothing running 200 with no align; settings exposure 2000, roi 0 0 618 410, save_at_node true; started, three measured frames, DELETE 200 with align.state ended, ended stopped, no error; settings equal the settings answer; the mount's true direction unchanged; exactly two files under frames, one .fits and the .solve.json of the same name; POST /api/imaging/frame then 200"
        decision: "spec.md:US3-S1"
      - path: crates/finder/tests/console/imaging_align_stop.rs
        status: new
        disposition: add
        assertion: "the alignment rig at (700, 420), started with exposure_ms 60000; once the frame is purpose align and exposing, DELETE answers within 5 s of paused time, 200, align.ended stopped, align.frames 0, the frame's state aborted"
        decision: "spec.md:US3-S2"
      - path: crates/finder/tests/console/imaging_align_aim.rs
        status: new
        disposition: add
        assertion: "the alignment rig at (700, 420), started with exposure_ms 1000: the first measured frame centre.middle true, done false, aim measured; the next measured frame done true; GET /api/session aim.source measured, aim.x within 1.5 px of 700 and aim.y within 1.5 px of 420; after those two frames, the screw turned to (-1100, 420), the first not_solved frame has no aim"
        decision: "spec.md:US3-S3"
      - path: crates/finder/tests/console/imaging_align_aim.rs
        status: new
        disposition: add
        assertion: "the alignment rig at (700, 420), exposure_ms 1000, three measured frames; turned to (-300, 420): a frame with aim off_frame has done false; turned to (900, 700): the frames with centre.middle true, until aim replaced, are exactly set_aside done false, set_aside done true, replaced done true; the session's aim.source measured, aim.x within 1.5 px of 900 and aim.y within 1.5 px of 700"
        decision: "spec.md:US3-S4"
      - path: crates/finder/tests/console/imaging_align_aim.rs
        status: new
        disposition: add
        assertion: "the alignment rig at (700, 420) with a 3000 ms node frame interval, started with {}: within 20 measured frames one has aim no_live_pointing and done true, its centre x and y within 2 px of 700 and 420, right_deg up_deg separation_deg within 0.02 deg of the pinhole oracle, finder_width 1456, finder_height 1088, on_frame true"
        decision: "spec.md:US3-S5"
      - path: crates/finder/tests/console/imaging_align_aim.rs
        status: new
        disposition: add
        assertion: "the alignment rig at (1300, 544), started with {}, three measured frames read and the session's aim.source measured; the screw turned to (-300, 544): within 20 measured frames one has centre.on_frame false, its centre x and y within 2 px of -300 and 544, right_deg up_deg separation_deg within 0.02 deg of the pinhole oracle (13 deg from the aim point, 8.4 from the finder camera's centre)"
        decision: "spec.md:US3-S6"
      - path: crates/finder/tests/console/imaging_align_aim.rs
        status: new
        disposition: add
        assertion: "the alignment rig at (700, 420) with no focal length: POST /api/imaging/align {} answers 409 the imaging camera's field height is unknown: start the console with --focal-length-mm, or solve an imaging frame on the Imaging page"
        decision: "spec.md:US3-S7"
      - path: crates/finder/tests/motion/imaging_align_track.rs
        status: new
        disposition: add
        assertion: "the Track rig, ready: POST /api/track on true 200, session track.wanted true, then POST /api/imaging/align {} answers 409 Track is on; turn it off while the finder camera's screws are turned"
        decision: "spec.md:US3-S8"
      - path: crates/finder/tests/motion/imaging_align_track.rs
        status: new
        disposition: add
        assertion: "the Track rig, ready, settings read, the alignment started and align.frames 1; POST /api/track on true 200: align.state ended, ended failed, error exactly Track is on; turn it off while the finder camera's screws are turned; settings equal those read; after POST /api/track on false 200, POST /api/imaging/frame 200"
        decision: "spec.md:US3-S9"
      - path: crates/finder/tests/motion/imaging_track.rs
        status: existing
        disposition: preserve
        assertion: "an aim reading taken while Track is on is held back and applied at the first live solve after Track off, as 032's FR-036 words it"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "032's note_solve takes the reading as FR-037 says; the kept scale and the misses are unchanged by the alignment"
      - "Track, 038's push check and 032's FR-036 are read, not changed"
      - "US2's run, start route and refusals are kept; the stop and the put-back wrap them"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console --test motion -E 'test(imaging_align)'"
        - "cargo nextest run -p finder --locked --test console -E 'test(imaging)'"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs and the night of plan.md § Outside the factory"
    writes:
      - crates/finder/src/imaging/align.rs
      - crates/finder/src/imaging/routes.rs
      - crates/finder/tests/console/imaging_align_aim.rs
      - crates/finder/tests/console/imaging_align_stop.rs
      - crates/finder/tests/console/main.rs
      - crates/finder/tests/motion/imaging_align_track.rs
      - crates/finder/tests/motion/main.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/imaging/routes.rs
        other_spec: 037-imaging-run
        other_story: US3
        disposition: merge_order
        reason: "050 depends_on_landed 037, whose US3 last changed imaging/routes.rs (581be8c3)"
      - path: crates/finder/tests/console/main.rs
        other_spec: 047-console-uses-the-database-that-fits
        other_story: US1
        disposition: merge_order
        reason: "050 depends_on_landed 047, whose US1 last changed console/main.rs (d3f8bd0f)"
      - path: crates/finder/tests/motion/main.rs
        other_spec: 041-imaging-run-on-the-rig
        other_story: US8
        disposition: merge_order
        reason: "050 depends_on_landed 041, whose US8 last changed motion/main.rs (61b54a47)"
    open_questions: []
  US4:
    tests:
      - path: crates/finder/tests/mcp/imaging_align.rs
        status: new
        disposition: add
        assertion: "the bare MCP console: get_finder_alignment x 1 and set_finder_alignment {} each err this console takes no imaging frames; the MCP rig: get_finder_alignment {} answers align null; set_finder_alignment {} errs on must be true or false, on false with exposure_ms errs set_finder_alignment with on false takes no other argument, get_finder_alignment x 1 errs get_finder_alignment takes no arguments; on true answers align.state running, again errs the finder alignment is already running; the read's align.last.centre.x within 2 px of 700; on false answers align.ended stopped; tools/list holds both, get_finder_alignment readOnlyHint true"
        decision: "spec.md:US4-S1"
      - path: crates/finder/tests/mcp/imaging_align.rs
        status: new
        disposition: add
        assertion: "crates/finder/README.md read as a file: it still holds The imaging camera has sixteen tools of its own; the line The Tools page's finder alignment has two tools: comes after it and before ### MCP under WSL2, with a list entry each for get_finder_alignment and set_finder_alignment between them; ## Finder alignment comes before ## Calibration frames"
        decision: "spec.md:US4-S2"
      - path: crates/finder/tests/console/imaging_run_docs.rs
        status: existing
        disposition: preserve
        assertion: "the README's MCP section still says sixteen tools of its own for the imaging camera"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "the README's sentence The imaging camera has sixteen tools of its own is unchanged"
      - "the imaging camera's existing MCP tools and their words are unchanged"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test mcp -E 'test(imaging_align)'"
        - "cargo nextest run -p finder --locked --test console -E 'test(docs)'"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - GLOSSARY.md
      - crates/finder/README.md
      - crates/finder/src/mcp/tools.rs
      - crates/finder/tests/mcp/imaging_align.rs
      - crates/finder/tests/mcp/main.rs
    no_write_reason: ""
    shared_writes:
      - path: GLOSSARY.md
        other_spec: 036-tracking-from-subs
        other_story: US2
        disposition: merge_order
        reason: "050 depends_on_landed 036, whose US2 last changed the glossary (c24f88cf)"
      - path: crates/finder/README.md
        other_spec: 047-console-uses-the-database-that-fits
        other_story: US3
        disposition: merge_order
        reason: "050 depends_on_landed 047, whose US3 last changed the finder's README (28f436df)"
      - path: crates/finder/src/mcp/tools.rs
        other_spec: 042-imaging-run-page-and-mcp
        other_story: US10
        disposition: merge_order
        reason: "050 depends_on_landed 042, whose US10 last changed mcp/tools.rs (c5f30038)"
      - path: crates/finder/tests/mcp/main.rs
        other_spec: 049-search-finds-whole-names
        other_story: US1
        disposition: merge_order
        reason: "050 depends_on_landed 049, whose US1 last changed mcp/main.rs (ed652f26)"
    open_questions: []
  US5:
    tests:
      - path: crates/finder/web/src/components/AlignmentBlock.test.tsx
        status: new
        disposition: add
        assertion: "centreLine gives exactly: off the frame, The imaging camera points 6.5° left and 1.0° up ... 6.6° in all, off the finder camera's frame.; on it, 4.7° right and 3.8° down ... 6.0° in all, on the finder camera's frame.; in the middle, 1.4° right and 1.3° down ... 1.9° in all, in the middle of the finder camera's frame. Done once the next frame holds it there.; done, Done: the imaging camera's centre has held in the middle of the finder camera's frame. The imaging camera points 0.2° left and 1.0° up ... 1.1° in all.; aimLine replaced gives The aim point is measured from this frame (replaced). and no_live_pointing gives The aim point is not measured from this frame (no_live_pointing); a later imaging solve measures it."
        decision: "spec.md:US5-S1"
      - path: crates/finder/web/src/components/AlignmentBlock.test.tsx
        status: new
        disposition: add
        assertion: "a running alignment with frames 0 and no last: the status reads exactly Taking the first imaging frame.; then Imaging frame 7 did not solve: the imaging solver found no solution. The next frame looks again.; then frame 9 with unsolved 3 ... 3 frames in a row have not solved: the imaging camera may be more than 10° from the finder camera's centre, the sky may hold too few stars, or the field height may be wrong. The next frame looks again.; then The finder camera had no live solve for imaging frame 10.; over a measured frame 2 with aim replaced, the text The aim point is measured from this frame (replaced). is shown"
        decision: "spec.md:US5-S2"
      - path: crates/finder/web/src/components/AlignmentBlock.test.tsx
        status: new
        disposition: add
        assertion: "with no view: Reading the imaging camera. and no button; Start aligning sends POST /api/imaging/align with Content-Type application/json and body {} and refreshes; a measured centre at (-300, -2000) shows the image The imaging camera's centre off the finder camera's frame with its circle cy 12 and cx 12 + 160 (728 - 1028 544 / 2544) / 1456 within 1e-6; Stop aligning sends DELETE /api/imaging/align and refreshes"
        decision: "spec.md:US5-S3"
      - path: crates/finder/web/src/components/AlignmentBlock.test.tsx
        status: new
        disposition: add
        assertion: "an alignment ended failed with the error the frame failed: the alert reads exactly The finder alignment stopped: the frame failed. and the button reads Start aligning"
        decision: "spec.md:US5-S4"
      - path: crates/finder/web/src/components/ToolsPage.test.tsx
        status: new
        disposition: add
        assertion: "App at /setup with a paired scope node: the link Tools leads to /tools with the level-one heading Tools and the button Start aligning; Back to console leads to /"
        decision: "spec.md:US5-S5"
      - path: crates/finder/web/src/components/ToolsPage.test.tsx
        status: new
        disposition: add
        assertion: "App at /setup with no scope node: once the heading Setup shows, no link named Tools"
        decision: "spec.md:US5-S6"
      - path: crates/finder/web/e2e/tools.spec.ts
        status: new
        disposition: add
        assertion: "the simulated console at 412 by 915, day, Track off and a scale kept through the API: Setup's Tools link opens /tools with the heading Tools; after Start aligning in the region Align the finder camera its status contains Done: the imaging camera's centre has held in the middle within 20 s, its image The imaging camera's centre on the finder camera's frame is visible, scrollWidth is at most 412; the day screenshot more than 10000 pixels off night red, the night one 0; after Stop aligning, Start aligning shows within 15 s"
        decision: "spec.md:US5-S7"
      - path: crates/finder/web/src/components/SetupPage.test.tsx
        status: existing
        disposition: preserve
        assertion: "Setup's header, sections and texts are as before when no onTools is given or no scope node is paired"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "the console, Setup and Imaging pages and the top bar are unchanged but for Setup's link"
      - "playwright.config.ts gains only its two list entries (owner approval 6)"
      - "dist is the build of its source"
    checks:
      development:
        - "npm test -- AlignmentBlock ToolsPage"
        - "npx playwright test --project=simulate e2e/tools.spec.ts"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml finder-page job and the night of plan.md § Outside the factory"
    writes:
      - crates/finder/web/dist
      - crates/finder/web/e2e/tools.spec.ts
      - crates/finder/web/playwright.config.ts
      - crates/finder/web/src/App.tsx
      - crates/finder/web/src/api.ts
      - crates/finder/web/src/components/AlignmentBlock.test.tsx
      - crates/finder/web/src/components/AlignmentBlock.tsx
      - crates/finder/web/src/components/SetupPage.tsx
      - crates/finder/web/src/components/ToolsPage.test.tsx
      - crates/finder/web/src/components/ToolsPage.tsx
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/web/dist
        other_spec: 047-console-uses-the-database-that-fits
        other_story: US3
        disposition: merge_order
        reason: "050 depends_on_landed 047, whose US3 last rebuilt dist (28f436df)"
      - path: crates/finder/web/src/App.tsx
        other_spec: 047-console-uses-the-database-that-fits
        other_story: US3
        disposition: merge_order
        reason: "050 depends_on_landed 047, whose US3 last changed App.tsx (28f436df)"
      - path: crates/finder/web/src/api.ts
        other_spec: 047-console-uses-the-database-that-fits
        other_story: US3
        disposition: merge_order
        reason: "050 depends_on_landed 047, whose US3 last changed api.ts (28f436df)"
      - path: crates/finder/web/src/components/SetupPage.tsx
        other_spec: 021-chart-orientation
        other_story: US1
        disposition: merge_order
        reason: "050 depends_on_landed 021, whose US1 last changed SetupPage.tsx (158fb41e)"
      - path: crates/finder/web/playwright.config.ts
        other_spec: 035-live-stack
        other_story: US9
        disposition: merge_order
        reason: "050 depends_on_landed 035, whose US9 last changed playwright.config.ts (6e506ec6)"
    open_questions: []
```
