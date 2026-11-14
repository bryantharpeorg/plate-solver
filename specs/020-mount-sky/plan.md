# Implementation Plan: A Drawn Sky on the Real Mount

**Branch**: `020-mount-sky` | **Date**: 2026-11-14 | **Spec**: [spec.md](spec.md)

This plan records every design decision for the feature. A story that would change one
escalates rather than re-deciding it.

## Summary

Give `LinkManager` an optional frame filter, applied to each node frame before it is
published. Under `finder --mount-sky`, the filter is a new `MountSky`: it fixes the zero
counts on the first usable frame, turns the frame's mid-exposure counts into an
azimuth and altitude, and replaces the frame's centroids with the catalog stars there,
drawn by the simulator's `field_stars`. Everything downstream (live solving, syncs, the
model, GoTo, tracking, MCP) runs unchanged on those frames, and every motor command goes
to the real mount. US2 adds a night clock behind two flags. US3 adds a banner.

## Technical Context

**Language/Version**: Rust (MSRV 1.96) for US1 and US2; TypeScript with React 19 for US3.

**Primary Dependencies**: nothing new. `finder` already depends on `simulator`.

**Storage**: `mount-sky/session.json` under the default state folder (FR-002). Nothing
about the drawn sky is persisted: zero counts live for the process.

**Testing**:
- US1: `crates/finder/tests/mount_sky.rs` (new), in-process and against the built binary.
- US2: `crates/finder/tests/mount_sky_night.rs` (new), the same way.
- US3: `crates/finder/web/src/components/MountSkyBanner.test.tsx` (new), Vitest.
- No Playwright test: the e2e projects run `--simulate`, which this spec doesn't touch.

**Target Platform**: the `finder` console, on the Surface (WSL2) or any desk machine.

**Constraints**:
- No change under `crates/scope-node`, `crates/scope-link` or `crates/simulator`.
- `web/dist` is committed and must be the build of its source (US3).
- Night mode's red-only rule (US3).

**Scale/Scope**: one new module, one filter seam, three flags, one banner.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Nothing touches the solver or `crates/math-core`. |
| II. Never weaken a gate | No existing test, golden, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS` or `ergane.yaml` is edited. New tests go in new files. |
| III. Tests first, independent expectations | The oracles are the truth formula of spec § User Scenarios, computed in each test, and the simulator's sky maths. US2's night clock is allowed by the amendment made before flipping (§ Prerequisites). |
| IV. Critical paths | No `unsafe`, no dependency change, no database-format change, no proto change. |
| V. Standalone Rust, honest licensing | Unaffected. |

## Project Structure

```text
crates/finder/
├── src/mount_sky.rs                    # US1: new; US2: night_utc
├── src/link.rs                         # US1: FrameFilter, start_with_filter
├── src/api.rs                          # US1: with_mount_sky, mount_sky field; US2: with_night_clock
├── src/session.rs                      # US1: mount_sky_terms
├── src/lib.rs                          # US1: pub mod mount_sky
├── src/main.rs                         # US1: --mount-sky, --tube-alt; US2: --mount-sky-night, --covers-on
├── README.md                           # US1, US2: flags and a section
├── tests/mount_sky.rs                  # US1: new
├── tests/mount_sky_night.rs            # US2: new
└── web/
    ├── src/api.ts                      # US3: MountSky type
    ├── src/App.tsx                     # US3: banner placement
    ├── src/components/MountSkyBanner.tsx       # US3: new
    ├── src/components/MountSkyBanner.test.tsx  # US3: new
    └── dist/                           # US3: rebuilt
GLOSSARY.md                             # US2: Sun guard entry
DESIGN.md                               # US3: the banner
```

## Interfaces

### The filter seam (`crates/finder/src/link.rs`)

```rust
/// Replaces each node frame before anything in the console reads it (020).
pub trait FrameFilter: Send + Sync {
    /// `offset_s` is the link's node-to-console clock offset, when known: console time
    /// is node time minus it.
    fn filter(&self, frame: proto::StarFrame, offset_s: Option<f64>) -> proto::StarFrame;
}

impl LinkManager {
    /// `start` with every frame passed through `filter` first.
    pub fn start_with_filter(
        session: Arc<Session>,
        clock: Arc<dyn Clock>,
        transport: Transport,
        filter: Option<Arc<dyn FrameFilter>>,
    ) -> Self;
}
```

`LinkManager::start` becomes `start_with_filter(session, clock, transport, None)`. The
filter is threaded to `active`, where the frame arm becomes:

```rust
frame = stream.message() => {
    let Ok(Some(frame)) = frame else { return false; };
    let frame = match &filter {
        Some(filter) => filter.filter(frame, status.offset_s),
        None => frame,
    };
    status.last_frame_at = Some(clock.now());
    status.stars = Some(frame.centroids.len());
    frame_tx.send_replace(Some(frame));
    publish(session, status_tx, status);
}
```

Nothing else in `link.rs` changes.

### `MountSky` (`crates/finder/src/mount_sky.rs`, new)

```rust
//! A drawn sky on the real mount (020): catalog stars at the pointing the encoders give.

/// The finder's horizontal field, as measured on the sky on 2026-11-09.
pub const FOV_DEG: f64 = 13.0;
/// The azimuth the zero counts read as.
pub const ZERO_AZ_DEG: f64 = 180.0;
/// The most centroids a node frame carries.
pub const MAX_CENTROIDS: usize = 200;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MountSkyView {
    pub tube_alt_deg: f64,
    /// Counts `[azimuth, altitude]` that read as `ZERO_AZ_DEG` and `tube_alt_deg`;
    /// `null` until the first usable frame.
    pub zero_counts: Option<[i32; 2]>,
}

pub struct MountSky { /* db, session, tube_alt_deg, zero: Mutex<Option<[i32; 2]>> */ }

impl MountSky {
    pub fn new(db: Arc<PatternDatabase>, session: Arc<Session>, tube_alt_deg: f64) -> Self;
    pub fn view(&self) -> MountSkyView;
}

impl FrameFilter for MountSky { /* below */ }
```

`filter(frame, offset_s)`:
1. **Counts.** Both `axis_start` and `axis_end` present: the mid counts are their
   per-axis mean as `f64`. Otherwise the frame is unusable.
2. **Terms.** `session.mount_sky_terms()` gives the site and counts per revolution.
   Missing either, or `offset_s`, makes the frame unusable. So does a width, height,
   exposure or gain that is not finite and above zero: `field_stars` asserts all four
   (`crates/simulator/src/render.rs:20`).
3. **Unusable.** Return the frame with `centroids` cleared and `preview_jpeg` `None`.
4. **Zero.** If no zero is held, store the mid counts rounded to `i32`.
5. **Pointing.** With `c` the counts per revolution and `z` the zero, build
   `Model::default()`, call `load_known_terms(0.0, 0.0, [ZERO_AZ_DEG − z[0] × 360 / c[0],
   tube_alt_deg − z[1] × 360 / c[1]])` and take `predict(mid, c)`. With no tilt and no
   fixed terms this is exactly the spec's truth formula, in the model's own sign
   convention (`crates/finder/src/model.rs:327`). Its altitude is apparent.
6. **Time.** `(frame.start_s + frame.end_s) / 2.0 − offset_s`.
7. **Stars.** `rotation = simulator::sky::camera_rotation(ephemeris::true_from_apparent(alt),
   az, simulator::Site { lat_deg, lon_deg }, time)`;
   `optics = Optics { width: frame.width, height: frame.height, fov_deg: FOV_DEG,
   eyepiece_px: (frame.width as f64 / 2.0, frame.height as f64 / 2.0) }`;
   `simulator::render::field_stars(&db, optics, rotation, frame.exposure_ms, frame.gain)`.
8. **Centroids.** Each star becomes `Centroid { x, y, brightness: 1e6 × 10^(−0.4 ×
   magnitude) }`, sorted by brightness, highest first, and cut to `MAX_CENTROIDS`.
   Replace `centroids`, set `preview_jpeg` to `None`, and return the frame.

The filter never panics on a frame: a `predict` of `None` makes the frame unusable.

### The session (`crates/finder/src/session.rs`)

```rust
impl Session {
    /// The site and counts per revolution a drawn sky needs (020).
    pub fn mount_sky_terms(&self) -> (Option<Site>, Option<[u32; 2]>);
}
```

It reads `values.site` and `values.counts_per_revolution` under the existing lock, as
`record_frame_sync` does (`crates/finder/src/session.rs:1061`).

### The console (`crates/finder/src/api.rs`)

`AppState` gains `mount_sky: Option<Arc<MountSky>>`, and:

```rust
/// The paired real node, with every frame's stars drawn from its encoders. Call before
/// `with_live`, in place of `with_link`.
pub fn with_mount_sky(mut self, transport: Transport, tube_alt_deg: f64) -> Self;
```

It builds `MountSky::new(self.db.clone(), self.session.clone(), tube_alt_deg)`, starts
`LinkManager::start_with_filter` with it, starts the `MountMonitor` as `with_link` does,
and keeps the `MountSky`. `SessionResponse` gains:

```rust
#[serde(skip_serializing_if = "Option::is_none")]
mount_sky: Option<crate::mount_sky::MountSkyView>,
```

filled from `state.mount_sky` in `session_response`.

US2 adds `night_utc: Option<String>` to `AppState`, and:

```rust
/// The night clock of `--mount-sky-night` (020 US2). Call before `with_mount_sky`.
pub fn with_night_clock(self, start_unix_s: f64, utc: impl Into<String>) -> Self;
```

which calls `with_clock(Arc::new(ConsoleClock(VirtualClock::new(start_unix_s))))` and
keeps `utc`. `with_mount_sky` then builds its `MountSky` with `.with_night_utc(utc)`.

### US2 in `MountSky`

```rust
impl MountSky {
    pub fn with_night_utc(self, utc: String) -> Self;
}
```

`MountSkyView` gains:

```rust
#[serde(skip_serializing_if = "Option::is_none")]
pub night_utc: Option<String>,
```

### The binary (`crates/finder/src/main.rs`)

US1:

```rust
/// Draw each frame's stars from the real mount's encoders instead of the camera.
#[arg(long, requires = "tube_alt", conflicts_with = "simulate")]
mount_sky: bool,
/// The tube's altitude now, in degrees from a level (0 to 90). With --mount-sky.
#[arg(long, requires = "mount_sky", value_parser = parse_tube_alt)]
tube_alt: Option<f64>,
```

`parse_tube_alt` accepts a finite number from 0 to 90 inclusive and otherwise answers
`--tube-alt requires degrees from 0 to 90`. Without `--state`, `--mount-sky` appends
`mount-sky` to the state folder, as `--simulate` appends `simulate`. The state is then
built with `with_mount_sky(Transport::Tcp, tube_alt)` in place of `with_link`, and the
console prints, before its address line:

```text
finder console: drawn sky: stars come from the mount's encoders, not the camera
```

US2:

```rust
/// Run the console on a night clock from this UTC, for daylight tests. Needs --covers-on.
#[arg(long, requires_all = ["mount_sky", "covers_on"], value_parser = parse_night)]
mount_sky_night: Option<Night>,
/// Confirm that the mirror's cover and the finder's cap are on. With --mount-sky-night.
#[arg(long, requires = "mount_sky_night")]
covers_on: bool,
```

`Night { unix_s: f64, utc: String }`. `parse_night` calls `finder::simulate::parse_start`
and keeps the argument text as `utc`; its error is
`--mount-sky-night requires YYYY-MM-DDTHH:MM:SSZ (optional fractional seconds)`. With a
night, `with_night_clock(night.unix_s, night.utc)` comes before `with_mount_sky`, and the
console also prints:

```text
finder console: night clock from <utc>: the Sun guard checks a pretend Sun; keep the mirror cover and the finder cap on
```

### `crates/finder/README.md`

US1 adds two rows after `--sim-start`:

```markdown
| `--mount-sky` | off | Draw each frame's stars from the real mount's encoders instead of the camera, for testing GoTo without a sky. Needs `--tube-alt`; not with `--simulate`. |
| `--tube-alt DEG` | | The tube's altitude when the console starts, from a level, 0 to 90. Requires `--mount-sky`. |
```

and a section `## Drawn sky on the real mount` after `## Simulated night and live
stream`, saying: what the mode does; that the first frame's counts read as azimuth 180°
and `--tube-alt`; that it keeps its session in `mount-sky/session.json` and the node is
paired there once; that every guard is unchanged; and that it can't test slip, gear
backlash after the encoder or the base's tilt. US2 adds two rows:

```markdown
| `--mount-sky-night UTC` | | Run the console on a night clock from `YYYY-MM-DDTHH:MM:SSZ`, so a daylight test can move the mount. The Sun guard then checks that clock's Sun. Requires `--mount-sky` and `--covers-on`. |
| `--covers-on` | off | Confirm that the mirror's cover and the finder's cap are on. Requires `--mount-sky-night`. |
```

and a paragraph in that section repeating the warning line.

### `GLOSSARY.md` (US2)

The **Sun guard** entry becomes:

> The console's standing refusal to point at the Sun or move the telescope near it, and
> to make any motorised move while the Sun is up. Always on, with no override from the
> page or an MCP client. Under `finder --mount-sky --mount-sky-night`, started with
> `--covers-on`, it checks the Sun of the night clock: the observer has confirmed the
> telescope is covered.

### The page (`crates/finder/web/src/`, US3)

`api.ts`:

```ts
export interface MountSky {
  tube_alt_deg: number;
  zero_counts: [number, number] | null;
  night_utc?: string;
}
```

and `mount_sky?: MountSky;` in `ObservingSession`.

`components/MountSkyBanner.tsx`:

```tsx
export function MountSkyBanner({ sky }: { sky: MountSky }) {
  return (
    <div role="status" className="page-width border-b border-rule px-4 py-2 text-sm text-ink sm:px-5 lg:px-10">
      <strong className="font-semibold">Drawn sky.</strong> These stars come from the
      mount's encoders, not seen by the camera.
      {sky.night_utc && (
        <> Night clock from {sky.night_utc}. Keep the mirror cover and the finder cap on.</>
      )}
    </div>
  );
}
```

The test reads the status's `textContent`, so the words in spec US3 must survive the
line breaks above: write each phrase the test asserts on one line of JSX.

`App.tsx`: right after `<TopBar … />`, before the session error,
`{session.mount_sky && <MountSkyBanner sky={session.mount_sky} />}`.

### `DESIGN.md` (US3)

Under the components, add:

> **Drawn sky banner.** Under `finder --mount-sky` a one-line status sits under the top
> bar on every page: "Drawn sky." in semibold, then that the stars come from the
> mount's encoders, and on a night clock that the covers must stay on. Ink on paper,
> with a rule below; no new token.

## Shapes that existing tests pin

These are not changed by any story:
- **`LinkManager::start`'s signature**, used by many tests through `with_link` and
  directly in `crates/finder/tests/link.rs`.
- **`GET /api/session` outside the mode.** `mount_sky` is skipped when absent, so every
  existing body is byte-for-byte what it was.
- **`--simulate` and `--sim-start`.** `crates/finder/tests/simulate_cli.rs` pins their
  refusals and the address line `finder console: http://`; the new lines print before
  that line and only in the new mode.
- **The frame a real link publishes** without a filter: `crates/finder/tests/link.rs` and
  `crates/finder/tests/e2e_live.rs` read it unchanged.
- **The page without `mount_sky`**: no new element, so every Vitest and Playwright test
  of the page, and the night screenshots, are unchanged.

## Decisions and the alternatives rejected

| Decision | Rejected | Why |
|---|---|---|
| Replace centroids at the link | Replace them in `Live`'s solver | GoTo's settle wait reads the link's channel too, and the page's star count and preview would still show the camera's. |
| Replace centroids at the link | Draw on the scope node | Needs the database on the Pi, a node and proto change, and a release to the Pi. |
| Centroids from `field_stars` | Render pixels and detect them | Frames carry centroids; drawing pixels only to detect them again adds time and nothing a GoTo test needs. |
| The model's own `predict` with known terms | A hand-written count-to-angle formula | It can't disagree with the console's sign convention. |
| Zero at azimuth 180° and the measured altitude | Zero at the simulator's 45°/180° | The minimum altitude then protects the real tube. |
| Its own state folder | The night's session | A drawn-sky night must never leave syncs or zero points in the real one. |
| `--covers-on` at start | A confirmation on the page | It is given before anything can move, and the page may not be open when an MCP client drives. |
| A night clock behind two flags | Turning the Sun guard off | Night-time rules (darkness, targets, the guard's geometry) keep working on the drawn sky. |

## Verification (every story, before proposing)

- **The factory gates** (`ergane.yaml` at c1ed295):
  - `lint` = `cargo fmt --all --check`,
    `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`,
    `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features --locked`
    and `RUSTDOCFLAGS="-D warnings" cargo doc -p finder --no-deps`;
  - `test` = `cargo test --workspace --all-targets --all-features --locked`,
    `cargo test --workspace --locked --doc`, and then, in `crates/finder/web`,
    `npm ci --no-audit --no-fund`, `npm run typecheck`, `npm run format:check`,
    `npm test`, `npm run build`, `git status --porcelain -- dist | (! grep .)` and
    `CI=1 npm run e2e`.

  Every story runs both, and the judge sees their results.
- **CI in the merge queue** runs every `ci.yml` job, including the finder-page
  build-freshness check and the build on Rust 1.96. Coverage runs only by hand
  (constitution 1.1.0).

## Prerequisites before this spec is flipped to `ready`

1. **The earlier specs have landed:** `016-mcp`, `017-console-stop-confirm` and `018-live-chart-steady`. Met:
   017's last story landed at c1ed295.
2. **`main` is still as read.** Every citation here is at c1ed295. If `main` has moved in
   these files, re-read them and re-run the validator.
3. **The owner approvals are given** (spec § Owner approvals). Flipping to `ready`
   records them.
4. **Constitution III is amended in the flip PR**, so the dispatch freezes it. The last
   sentence of Principle III becomes:

   > A virtual clock is allowed only in tests, `finder --simulate`, and
   > `finder --mount-sky --mount-sky-night` started with `--covers-on`, where the
   > observer has confirmed the telescope is covered and the Sun guard checks the
   > virtual clock's Sun.

   The version goes from 1.1.0 to 1.2.0 and `Last Amended` to the flip's date.
5. **No operator-staged files are needed.**

## Plan traps

Lessons from building 012 to 019 on this factory, each with its mechanism and the check
that keeps it from recurring. They bind every story.

1. **Gate evidence or nothing.** The judge credits only gate results and the story's
   diff. *Check:* every Then clause names a committed test that the `test` gate runs.
2. **A named command is read literally.** *Check:* commands are copied from
   § Verification.
3. **One evidence budget.** *Check:* each story keeps its diff under 64 KiB, measured
   with `git diff <base> -- . ':(exclude)crates/finder/web/dist' | wc -c`.
4. **The sandbox has only the gates' tools.** *Check:* no task runs a tool outside them.
5. **A spec fixed mid-epic needs a relaunch.** The judge grades the spec as dispatched.
   *Check:* no builder edits `specs/020-mount-sky/` or `.specify/`.
6. **An oracle copied from the code proves nothing.** *Check:* the tests compute the
   truth formula and the target's horizontal position themselves, and never call
   `MountSky` or `Model` to get an expected value.
7. **A solve that agrees by accident.** If the node's own camera pointed where the drawn
   sky does, US1-S1 and S3 would pass with no filter at all. *Check:* the `SimMount`'s
   zero points of `[300, 70]` keep the node's sky more than 40° away, and US1-S1 also
   asserts the zero counts.
8. **A binary test that never reaches the mode.** *Check:* US1-S7 and US2-S3 read
   `mount_sky` from the running binary, not only its exit status.
9. **The bundle must be rebuilt.** *Check:* US3 commits `crates/finder/web/dist` after
   `npm run build`, so that the gate's `git status --porcelain -- dist` finds nothing.
10. **Each worktree builds on its own.** *Check:* no story sets `CARGO_TARGET_DIR`.

## Risks

| Risk | Mitigation |
|---|---|
| A drawn-sky pointing is taken for a real one. | The banner on every page, the start-up line, `mount_sky` in the session, and a separate state folder. |
| The observer forgets the covers in daylight. | `--covers-on` can't be skipped, and the warning line and banner repeat it. Physical safety still rests on the observer (owner approval 2). |
| The real tube hits a stop because the drawn altitude is off. | Zero at the measured altitude; README asks for a level check before long moves. |
| Rendering slows the link loop. | `field_stars` takes milliseconds per frame at a 1 s cadence; frames carry at most 200 stars. |

## Refinement Record

Drafted on 2026-11-14 by the plate-solver operator against `main` at c1ed295, from a
read-only map of the console's frame, mount and simulate paths. No code was probed:
the interfaces were read from the cited lines, and the risk that `field_stars` and the
model's `predict` don't line up is carried by US1-S1 and S3. Writes come from the task
text. Shared writes name the merge edge that orders them, and the 016, 017, 018 and 019
stories that last wrote the same file on `main`. Nothing in this record is an owner
approval: those are in spec.md, and flipping to `ready` gives them.

```yaml
version: 1
reviewed_revision: c1ed295734253c7d6121052b0a14beb2ee64bc17
stories:
  US1:
    tests:
      - path: crates/finder/tests/mount_sky.rs
        status: new
        disposition: add
        assertion: "the first usable frame fixes zero_counts at its mid counts, and the live pointing reads azimuth 180 and altitude 30 within 0.05 deg while the node's own camera points elsewhere"
        decision: "spec.md:US1-S1"
      - path: crates/finder/tests/mount_sky.rs
        status: new
        disposition: add
        assertion: "a filtered frame keeps id, times, size, exposure, gain and counts, drops its preview, and carries 20 to 200 centroids in non-increasing brightness"
        decision: "spec.md:US1-S2"
      - path: crates/finder/tests/mount_sky.rs
        status: new
        disposition: add
        assertion: "a GoTo to M42 ends done with the truth of the last frame's counts within 0.05 deg of M42; in the day it is refused with Sun is up and no motion command reaches the mount"
        decision: "spec.md:US1-S3"
      - path: crates/finder/tests/mount_sky.rs
        status: new
        disposition: add
        assertion: "a node without a mount gives frames with no centroids, no live pointing and null zero_counts; the binary refuses bad flag combinations and keeps its session in mount-sky/"
        decision: "spec.md:US1-S5"
      - path: crates/finder/tests/simulate_cli.rs
        status: existing
        disposition: preserve
        assertion: "--simulate and --sim-start refusals and the address line are unchanged"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/link.rs
        status: existing
        disposition: preserve
        assertion: "a link without a filter publishes the node's frames unchanged"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "every guard (Sun, minimum altitude, Stop, watchdog) behaves as without the mode (spec.md FR-007)"
      - "GET /api/session outside the mode is unchanged"
      - "--simulate is unchanged and refused with --mount-sky"
    checks:
      development:
        - "cargo test -p finder --test mount_sky --locked"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs, and a first run against the real board with the covers on"
    writes:
      - crates/finder/README.md
      - crates/finder/src/api.rs
      - crates/finder/src/lib.rs
      - crates/finder/src/link.rs
      - crates/finder/src/main.rs
      - crates/finder/src/mount_sky.rs
      - crates/finder/src/session.rs
      - crates/finder/tests/mount_sky.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/README.md
        other_spec: 020-mount-sky
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and adds its flags to US1's section"
      - path: crates/finder/src/api.rs
        other_spec: 020-mount-sky
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and adds with_night_clock"
      - path: crates/finder/src/main.rs
        other_spec: 020-mount-sky
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and adds the night flags"
      - path: crates/finder/src/mount_sky.rs
        other_spec: 020-mount-sky
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and adds night_utc"
      - path: crates/finder/README.md
        other_spec: 017-console-stop-confirm
        other_story: US4
        disposition: merge_order
        reason: "020 depends_on_landed 017, whose US4 last changed the README"
      - path: crates/finder/src/api.rs
        other_spec: 017-console-stop-confirm
        other_story: US1
        disposition: merge_order
        reason: "020 depends_on_landed 017, whose US1 last changed api.rs"
      - path: crates/finder/src/lib.rs
        other_spec: 017-console-stop-confirm
        other_story: US1
        disposition: merge_order
        reason: "020 depends_on_landed 017, whose US1 last changed lib.rs"
      - path: crates/finder/src/link.rs
        other_spec: 017-console-stop-confirm
        other_story: US1
        disposition: merge_order
        reason: "020 depends_on_landed 017, whose US1 last changed link.rs"
      - path: crates/finder/src/main.rs
        other_spec: 016-mcp
        other_story: US1
        disposition: merge_order
        reason: "020 depends_on_landed 016, whose US1 last changed main.rs"
      - path: crates/finder/src/session.rs
        other_spec: 017-console-stop-confirm
        other_story: US2
        disposition: merge_order
        reason: "020 depends_on_landed 017, whose US2 last changed session.rs"
    open_questions: []
  US2:
    tests:
      - path: crates/finder/tests/mount_sky_night.rs
        status: new
        disposition: add
        assertion: "--mount-sky-night without --covers-on, without --mount-sky, and --covers-on alone each exit non-zero before listening, the first naming --covers-on"
        decision: "spec.md:US2-S1"
      - path: crates/finder/tests/mount_sky_night.rs
        status: new
        disposition: add
        assertion: "with every flag the running binary reports night_utc and a session time within 60 s of the night start, and prints the pretend Sun warning"
        decision: "spec.md:US2-S3"
      - path: crates/finder/tests/mount_sky_night.rs
        status: new
        disposition: add
        assertion: "a console built with with_night_clock reports night_utc and one built without has no night_utc key"
        decision: "spec.md:US2-S4"
    preserved_behavior:
      - "without --mount-sky-night the console clock is the system clock (constitution III)"
      - "--simulate is unchanged"
    checks:
      development:
        - "cargo test -p finder --test mount_sky_night --locked"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - GLOSSARY.md
      - crates/finder/README.md
      - crates/finder/src/api.rs
      - crates/finder/src/main.rs
      - crates/finder/src/mount_sky.rs
      - crates/finder/tests/mount_sky_night.rs
    no_write_reason: ""
    shared_writes: []
    open_questions: []
  US3:
    tests:
      - path: crates/finder/web/src/components/MountSkyBanner.test.tsx
        status: new
        disposition: add
        assertion: "a session with mount_sky shows one status saying Drawn sky and not seen by the camera; with night_utc it adds the night clock and the covers; without mount_sky nothing says Drawn sky"
        decision: "spec.md:US3-S1"
      - path: crates/finder/web/e2e/simulate.spec.ts
        status: existing
        disposition: preserve
        assertion: "the simulated console's page, with no mount_sky, and its night screenshots are unchanged"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "the page without mount_sky renders exactly as before"
      - "red-only night mode: no colour or token changes"
    checks:
      development:
        - "npm test"
        - "npm run typecheck"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs: finder-page"
    writes:
      - DESIGN.md
      - crates/finder/web/dist
      - crates/finder/web/src/App.tsx
      - crates/finder/web/src/api.ts
      - crates/finder/web/src/components/MountSkyBanner.test.tsx
      - crates/finder/web/src/components/MountSkyBanner.tsx
    no_write_reason: ""
    shared_writes:
      - path: DESIGN.md
        other_spec: 017-console-stop-confirm
        other_story: US5
        disposition: merge_order
        reason: "020 depends_on_landed 017, whose US5 last changed DESIGN.md"
      - path: crates/finder/web/dist
        other_spec: 017-console-stop-confirm
        other_story: US5
        disposition: merge_order
        reason: "020 depends_on_landed 017, whose US5 last rebuilt the bundle"
      - path: crates/finder/web/src/App.tsx
        other_spec: 018-live-chart-steady
        other_story: US1
        disposition: merge_order
        reason: "020 depends_on_landed 018, whose US1 last changed App.tsx"
      - path: crates/finder/web/src/api.ts
        other_spec: 017-console-stop-confirm
        other_story: US5
        disposition: merge_order
        reason: "020 depends_on_landed 017, whose US5 last changed api.ts"
    open_questions: []
```
