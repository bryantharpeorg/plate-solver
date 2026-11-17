# Implementation Plan: A Drawn Sky on the Real Mount

**Branch**: `026-mount-sky` | **Date**: 2026-11-14 | **Spec**: [spec.md](spec.md)

This plan records every design decision for the feature. A story that would change one
escalates rather than re-deciding it.

## Summary

Give `LinkManager` an optional frame filter, applied to each node frame before it is
published. Under `finder --mount-sky`, the filter is a new `MountSky`: it fixes the zero
counts on the first usable frame, turns the frame's mid-exposure counts into an
azimuth and altitude, and replaces the frame's centroids with the catalog stars there,
drawn by the simulator's `field_stars`. Everything downstream (live solving, syncs, the
model, GoTo, tracking, MCP) runs unchanged on those frames, and every motor command goes
to the real mount. When the node's reset count (spec 024) changes, `MountSky` stops
drawing for good. US2 adds a night clock behind two flags. US3 adds a banner, which
also forbids hand pushes.

## Technical Context

**Language/Version**: Rust (MSRV 1.96) for US1 and US2; TypeScript with React 19 for US3.

**Primary Dependencies**: nothing new. `finder` already depends on `simulator`. US1
reads spec 024's reset count (`AxisState.mount_resets`, `Mount::resets` and the
session's last-seen count), so 024 landed first and is in `depends_on_landed`
(§ Prerequisites).

**Storage**: `mount-sky/session.json` under the default state folder (FR-002). Nothing
about the drawn sky is persisted: zero counts live for the process.

**Testing**:
- US1 and US2: four new Rust test modules, one in each test binary a story's scenarios
  need, as constitution 1.2.0 asks of a new finder test:
  - `crates/finder/tests/motion/mount_sky.rs` (US1) for US1-S1 to US1-S5 and US1-S8,
    which move or read the simulated mount in paused time
    (`crates/finder/tests/motion/main.rs:1`), declared by `mod mount_sky;` after
    `mod mount_identity;` in `crates/finder/tests/motion/main.rs`. US1-S5 is the same
    rig with no mount, as `crates/finder/tests/motion/goto/refusals.rs:117` is.
  - `crates/finder/tests/cli/mount_sky.rs` (US1) for US1-S6 and US1-S7, which start the
    built `finder` (`crates/finder/tests/cli/main.rs:1`), declared by `mod mount_sky;`
    after `mod flags;` in `crates/finder/tests/cli/main.rs`.
  - `crates/finder/tests/cli/mount_sky_night.rs` (US2) for US2-S1 to US2-S3, declared by
    `mod mount_sky_night;` after US1's `mod mount_sky;` in
    `crates/finder/tests/cli/main.rs`.
  - `crates/finder/tests/motion/mount_sky_night.rs` (US2) for US2-S4, whose two consoles
    stand on US1's rig, declared by `mod mount_sky_night;` after US1's `mod mount_sky;`
    in `crates/finder/tests/motion/main.rs`.

  A binary's `main.rs` holds only its support include and `mod` lines
  (`crates/finder/tests/layout/main.rs:156`). The `layout` test fails a file at the top
  level of `crates/finder/tests` (`crates/finder/tests/layout/main.rs:109`) and a new
  file in `crates/finder/tests/support` (`crates/finder/tests/layout/main.rs:126`). Each
  module reaches shared helpers through `crate::support`
  (`use crate::support::mcp as support;` in process, `crate::support::Scratch` for every
  temporary folder), copies any private helper it needs from the test it is built like,
  and holds only the setup its own tests use, since an unused helper fails
  `clippy -D warnings`. The in-process modules copy `Rig::new`
  (`crates/finder/tests/support/mcp.rs:191`), whose recording mount is private, keep its
  solver's `timeout_ms: u64::MAX` (`crates/finder/tests/support/mcp.rs:122`), leave out
  its `set_aim` (`crates/finder/tests/support/mcp.rs:248`), since a live pointing sits
  at the aim and the setup's is the frame centre, and give the copied mount a
  `resets()` the test sets. `crates/finder/tests/motion/mount_identity.rs` is that copy
  with a settable reset count; copy it, keeping its `DB`, `NodeTime`, `Stars` and
  `Recorded`. The binary modules start `finder` as
  `crates/finder/tests/cli/simulate.rs:25` does, without its `--simulate` argument. A
  refusal test spawns the binary and fails, killing it, as soon as standard error shows
  a `finder console:` line, instead of waiting on `output()`. They run alone with
  `cargo nextest run -p finder --locked --test motion mount_sky::`,
  `cargo nextest run -p finder --locked --test cli mount_sky::`,
  `cargo nextest run -p finder --locked --test cli mount_sky_night::` and
  `cargo nextest run -p finder --locked --test motion mount_sky_night::`.
- US3: `crates/finder/web/src/components/MountSkyBanner.test.tsx` (new), Vitest. It
  renders `App` with `fetch` stubbed as `crates/finder/web/src/App.target.test.tsx:12`
  does: `/api/session` answers `crates/finder/web/src/test/targets.ts`'s `SESSION` with
  or without `mount_sky`, `/api/samples` answers `[]`, and the rest answer health data.
  It waits for the target's guidance region before reading the statuses; at `/setup`
  (US3-S1), which has no guidance region, it waits for the drawn-sky status itself.
- No Playwright test: no e2e server runs `--mount-sky`
  (`crates/finder/web/playwright.config.ts:62`), so no e2e page has `mount_sky`.

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
| II. Never weaken a gate | No existing test, golden, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS` or `ergane.yaml` is edited. New tests are new modules; each adds one `mod` line to a binary's `main.rs`, which holds no test (`motion/main.rs`, `cli/main.rs`), as constitution 1.2.0 asks. |
| III. Tests first, independent expectations | The oracles are the truth formula of spec § User Scenarios, computed in each test, and the simulator's sky maths. US2's night clock is allowed by the amendment made before flipping, which takes the constitution to 1.4.0 (§ Prerequisites). |
| IV. Critical paths | No `unsafe`, no dependency change, no database-format change, no proto change. |
| V. Proven open source first, with honest licensing | **Looked for:** a program that draws catalog stars at a telescope's pointing, to stand in for the sky in front of a real mount. **Found and turned down:** *INDI's CCD Simulator* (`indi_simulator_ccd`, part of INDI core, LGPL-2.1-or-later) renders 16-bit pixel frames of Guide Star Catalog stars at the RA and Dec it snoops from an INDI mount device (`EQUATORIAL_EOD_COORD`). It does not fit cleanly. Our mount is driven by our own scope node and has no INDI driver, so a bridge of ours would have to turn the encoder counts into RA and Dec, which is the count-to-sky step this plan writes anyway. Its frames are pixels: to reach the console through our scope link they would have to enter the node as its camera (a scope-node change and a release to the Pi, out of scope) or be detected in the console, the pixel path § Decisions turns down. It draws no stars without the separate GSC catalogue program installed, and its alt-az field rotation was only on INDI's development branch, in no release, when read on 2026-11-15. *KStars with Ekos* (GPL-2.0-or-later) rehearses a night on INDI's simulators, so it brings the same limits. *Stellarium* (GPL-2.0-or-later) draws the sky for a screen, not a camera's frame at a set field and magnitude limit, so its pixels too would need detecting. *INDI's Telescope Simulator* simulates the mount, the part this mode keeps real. **Taken:** our own `simulator` crate's `field_stars` and `camera_rotation`, which `finder` already depends on, so the new code is the filter and the count-to-sky step. Nothing is copied in, linked or called out to, and no license changes. |

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
├── tests/motion/main.rs                # US1: mod mount_sky; US2: mod mount_sky_night
├── tests/motion/mount_sky.rs           # US1: new
├── tests/motion/mount_sky_night.rs     # US2: new
├── tests/cli/main.rs                   # US1: mod mount_sky; US2: mod mount_sky_night
├── tests/cli/mount_sky.rs              # US1: new
├── tests/cli/mount_sky_night.rs        # US2: new
└── web/
    ├── src/api.ts                      # US3: MountSky type
    ├── src/useSession.ts               # US3: request keeps mount_sky
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
/// Replaces each node frame before anything in the console reads it (026).
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
filter is threaded through `run`, which gains `#[allow(clippy::too_many_arguments)]` as
`active` has, to `active`, where the frame arm becomes:

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

Beyond that argument and that allow, nothing else in `link.rs` changes.

### `MountSky` (`crates/finder/src/mount_sky.rs`, new)

```rust
//! A drawn sky on the real mount (026): catalog stars at the pointing the encoders give.

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
    /// True once the node's reset count changed after the zero was fixed (FR-014).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub power_cycled: bool,
}

/// `zero` holds the zero counts and the reset count the session had seen when they
/// were fixed; `power_cycled` is never cleared.
pub struct MountSky {
    /* db, session, tube_alt_deg, zero: Mutex<Option<([i32; 2], u64)>>, power_cycled: AtomicBool */
}

impl MountSky {
    pub fn new(db: Arc<PatternDatabase>, session: Arc<Session>, tube_alt_deg: f64) -> Self;
    pub fn view(&self) -> MountSkyView;
}

impl FrameFilter for MountSky { /* below */ }
```

`filter(frame, offset_s)`:
1. **Stopped.** Once `power_cycled` is set (step 5), every frame is unusable.
2. **Counts.** Both `axis_start` and `axis_end` present: the mid counts are their
   per-axis mean as `f64`. Otherwise the frame is unusable.
3. **Terms.** `session.mount_sky_terms()` gives the site, the counts per revolution and
   the last-seen reset count. Missing any of them, or `offset_s`, or counts per
   revolution holding a 0, makes the frame unusable. So does a width or height of 0, or
   an exposure or gain that is not finite and above zero, which `field_stars` asserts
   (`crates/simulator/src/optics.rs:33`, `crates/simulator/src/render.rs:20`).
4. **Unusable.** Return the frame with `centroids` cleared and `preview_jpeg` `None`.
5. **Zero.** If no zero is held, store the mid counts rounded to `i32` with the reset
   count beside them. If one is held and the reset count differs from the one stored
   with it, set `power_cycled`, and the frame is unusable (FR-014). A count that
   changes back does not clear it.
6. **Pointing.** With `c` the counts per revolution and `z` the zero, build
   `Model::default()`, call `load_known_terms(0.0, 0.0, [ZERO_AZ_DEG − z[0] × 360 / c[0],
   tube_alt_deg − z[1] × 360 / c[1]])` and take `predict(mid, c)`. With no tilt and no
   fixed terms this is exactly the spec's truth formula, in the model's own sign
   convention (`crates/finder/src/model.rs:327`). Its altitude is apparent.
7. **Time.** `(frame.start_s + frame.end_s) / 2.0 − offset_s`.
8. **Stars.** `rotation = simulator::sky::camera_rotation(ephemeris::true_from_apparent(alt),
   az, simulator::Site { lat_deg, lon_deg }, time)`;
   `optics = Optics { width: frame.width, height: frame.height, fov_deg: FOV_DEG,
   eyepiece_px: (frame.width as f64 / 2.0, frame.height as f64 / 2.0) }`;
   `simulator::render::field_stars(&db, optics, rotation, frame.exposure_ms, frame.gain)`.
   This `rotation` is the camera's attitude on the sky. It is not 021's chart rotation,
   which only turns the page's view: the filter never reads `chart_rotation_deg`.
9. **Centroids.** Each star becomes `Centroid { x, y, brightness: 1e6 × 10^(−0.4 ×
   magnitude) }`, sorted by brightness, highest first, and cut to `MAX_CENTROIDS`.
   Replace `centroids`, set `preview_jpeg` to `None`, and return the frame.

The filter never panics on a frame: a `predict` of `None` makes the frame unusable, and
step 3 refuses a count per revolution of 0 before step 6 divides by it, since
`load_known_terms` asserts finite zero points (`crates/finder/src/model.rs:354`). The
mount poll already drops a 0 count (`crates/finder/src/live.rs:306`), so that frame
cannot arise today.

### The session (`crates/finder/src/session.rs`)

```rust
impl Session {
    /// The site, counts per revolution and last-seen count of the node's mount resets
    /// that a drawn sky needs (026).
    pub fn mount_sky_terms(&self) -> (Option<Site>, Option<[u32; 2]>, Option<u64>);
}
```

It reads `values.site`, `values.counts_per_revolution` and `values.mount_resets`, the
last reset count a good mount poll saw, which spec 024 keeps in the session
(`crates/finder/src/session.rs:594`; none before the first poll), under the existing
lock, as `record_frame_sync` does (`crates/finder/src/session.rs:1133`). A good poll
sets that count and the counts per revolution together, and a failed poll clears only
the counts per revolution (`Session::set_mount`, `crates/finder/src/session.rs:1059`),
so the session holds a reset count whenever it holds counts per revolution.
`Session::mount_resets` (`crates/finder/src/session.rs:1055`) reads the count alone;
`mount_sky_terms` takes all three under one lock.

### The console (`crates/finder/src/api.rs`)

`AppState` gains `mount_sky: Option<Arc<MountSky>>`, and:

```rust
/// The paired real node, with every frame's stars drawn from its encoders. Call before
/// `with_live`, in place of `with_link`.
pub fn with_mount_sky(mut self, transport: Transport, tube_alt_deg: f64) -> Self;
```

It builds `MountSky::new(self.db.clone(), self.session.clone(), tube_alt_deg)`, starts
`LinkManager::start_with_filter` with it, keeps that link and starts the `MountMonitor`
as `with_link` does (`crates/finder/src/api.rs:222`), and keeps the `MountSky`. So
`with_live`, GoTo and 025's recorder read the filtered frames. `SessionResponse` gains:

```rust
#[serde(skip_serializing_if = "Option::is_none")]
mount_sky: Option<crate::mount_sky::MountSkyView>,
```

filled from `state.mount_sky` in `session_response`.

US2 adds `night_utc: Option<String>` to `AppState`, and:

```rust
/// The night clock of `--mount-sky-night` (026 US2). Call before `with_mount_sky`.
pub fn with_night_clock(self, start_unix_s: f64, utc: impl Into<String>) -> Self;
```

which calls
`with_clock(Arc::new(crate::simulate::ConsoleClock(simulator::VirtualClock::new(start_unix_s))))`
and keeps `utc`. `with_mount_sky` then builds its `MountSky` with `.with_night_utc(utc)`.

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

`#[derive(Clone)] struct Night { unix_s: f64, utc: String }`: clap's typed
`value_parser` needs `Clone`. `parse_night` calls `finder::simulate::parse_start`
and keeps the argument text as `utc`; its error is
`--mount-sky-night requires YYYY-MM-DDTHH:MM:SSZ (optional fractional seconds)`. With a
night, `with_night_clock(night.unix_s, night.utc)` comes before `with_mount_sky`, and the
console also prints, after the drawn-sky line and before its address line:

```text
finder console: night clock from <utc>: the Sun guard checks a pretend Sun; keep the mirror cover and the finder cap on
```

### `crates/finder/README.md`

US1 adds two rows after `--sim-start`:

```markdown
| `--mount-sky` | off | Draw each frame's stars from the real mount's encoders instead of the camera, for testing GoTo without a sky. Needs `--tube-alt`; not with `--simulate`. Do not push the tube by hand in this mode. |
| `--tube-alt DEG` | | The tube's altitude when the console starts, from a level, 0 to 90. Requires `--mount-sky`. |
```

and a section `## Drawn sky on the real mount` after `## Simulated night and live
stream` (before 025's `## Recording a night`), saying: what the mode does; that the
first frame's counts read as azimuth 180° and `--tube-alt`; that it keeps its session in
`mount-sky/session.json` and the node is paired there once; that every guard is
unchanged; that the tube must not be pushed by hand in this mode, because with the board's encoders off a push moves the tube and not
the counts, so the drawn sky stops matching the tube and nothing detects it; that after
a power cycle (spec 024's reset count) the drawn sky stops for good, and the console must
be restarted with a fresh `--tube-alt` and the night then reset; and that it can't test slip,
gear backlash after the encoder, the base's tilt or the 2 to 3% shortfall seen on the
sky in the real board's 1° moves. US2 adds two rows after US1's `--tube-alt DEG` row:

```markdown
| `--mount-sky-night UTC` | | Run the console on a night clock from `YYYY-MM-DDTHH:MM:SSZ`, so a daylight test can move the mount. The Sun guard then checks that clock's Sun. Requires `--mount-sky` and `--covers-on`. |
| `--covers-on` | off | Confirm that the mirror's cover and the finder's cap are on. Requires `--mount-sky-night`. |
```

and a paragraph in that section, before its last paragraph (what the drawn sky cannot
test), repeating the warning line.

### `GLOSSARY.md` (US2)

The **Sun guard** entry's line, `GLOSSARY.md:30`, gains at its end:

> Under `finder --mount-sky --mount-sky-night`, started with `--covers-on`, it checks
> the Sun of the night clock: the observer has confirmed the telescope is covered.

Its `_Avoid_` line is kept.

### The page (`crates/finder/web/src/`, US3)

`api.ts`:

```ts
export interface MountSky {
  tube_alt_deg: number;
  zero_counts: [number, number] | null;
  power_cycled?: boolean;
  night_utc?: string;
}
```

and `mount_sky?: MountSky;` in `ObservingSession`. On `main` the interface goes right
before `export interface ObservingSession`, after `McpStatus`, and the field is
`ObservingSession`'s last, after `field?`; 021's `chart_rotation_deg?` sits between
`aim?` and `field?`, above it.

`useSession.ts`: `request` keeps only the session fields it names
(`crates/finder/web/src/useSession.ts:29`). After `backlash`
(`crates/finder/web/src/useSession.ts:50`), still the last spread on `main` (021's
`chart_rotation_deg` spread sits after `aim`, earlier in the list), add
`...(body?.mount_sky != null && { mount_sky: body.mount_sky }),` as the new last line.

`components/MountSkyBanner.tsx`:

```tsx
export function MountSkyBanner({ sky }: { sky: MountSky }) {
  return (
    <div role="status" className="page-width border-b border-rule px-4 py-2 text-sm text-ink sm:px-5 lg:px-10">
      <strong className="font-semibold">Drawn sky.</strong> These stars come from the
      mount's encoders, not seen by the camera.
      No hand pushes: a push moves the tube but not the drawn sky.
      {sky.power_cycled && (
        <> The mount was power-cycled, so the drawn sky has stopped: restart the console with a fresh --tube-alt.</>
      )}
      {sky.night_utc && (
        <> Night clock from {sky.night_utc}. Keep the mirror cover and the finder cap on.</>
      )}
    </div>
  );
}
```

The test reads the status's `textContent`. JSX joins text lines with one space, so
prettier's line breaks keep each phrase. Never join two words of an asserted phrase
across an expression such as `{""}`. Run `npx prettier --write` on the new files before
`npm run format:check`.

`App.tsx`: right after `<TopBar … />`, before the session error,
`{session.mount_sky && <MountSkyBanner sky={session.mount_sky} />}`. That is outside
the page switch, so the banner shows on `/setup` too (US3-S1).

### `DESIGN.md` (US3)

At the end of `## Components`, after the `### Recording — built` entry (025's, still the
last there on `main`; 021's `### Chart rotation — built` sits higher, after
`### Live chart and preview — built`), add a `### Drawn sky banner — built` entry,
written as one paragraph line under its heading, as its neighbours Aim-point tap and
Recording are:

> Under `finder --mount-sky` a one-line status sits under the top bar on every page:
> "Drawn sky." in semibold, then that the stars come from the mount's encoders and that
> the tube must not be pushed by hand; after a power cycle, that the drawn sky has
> stopped and the console must be restarted with a fresh `--tube-alt`; and on a night
> clock, that the covers must stay on. Ink on paper, with a rule below; no new token.

## Shapes that existing tests pin

These are not changed by any story:
- **`LinkManager::start`'s signature**, used by many tests through `with_link` and
  directly in `crates/finder/tests/console/link.rs` and in `motion`'s `mount_calls` and
  `tracking` modules.
- **`GET /api/session` outside the mode.** `mount_sky` is skipped when absent, so every
  existing body is byte-for-byte what it was.
- **`--simulate` and `--sim-start`.** `crates/finder/tests/cli/simulate.rs` pins their
  refusals and the address line `finder console: http://`; the new lines print before
  that line and only in the new mode.
- **The frame a real link publishes** without a filter:
  `crates/finder/tests/console/link.rs` and
  `crates/finder/tests/console/live/whole_loop.rs` read it unchanged.
- **The test layout.** `crates/finder/tests/layout/main.rs` pins the five folders, the six
  files of `tests/support/` and that each binary's `main.rs` holds only its support
  include and `mod` lines: each new module adds one `mod` line and no other file.
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
| React to 024's reset count | A counts-jump guard of this spec's own | 024 already counts a jump beyond the top rate as a reset, and its count also catches a lost link and a cleared init bit. |
| Stop drawing for good on a reset | Re-zero from the counts after it | The counts after a power cycle are an unknown offset from the old ones; only a fresh `--tube-alt` measures the altitude again. |
| Latch the stop | Compare the count on each frame | A count that changed back would resume drawing from a zero the reset made stale. |
| Forbid hand pushes in words | Detect a push | With the board's encoders off a push changes nothing the console reads. |

## Verification (every story, before proposing)

- **The factory gates** (`ergane.yaml` at 2f6a777, gates unchanged since 20ce2fe;
  `test`'s Rust half has run under cargo-nextest since 2026-11-14):
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
- **CI in the merge queue** runs every `ci.yml` job, including the finder-page
  build-freshness check and the build on Rust 1.96. Coverage runs only by hand
  (constitution 1.1.0).

## Prerequisites before this spec is flipped to `ready`

1. **The earlier specs have landed:** `016-mcp`, `017-console-stop-confirm`,
   `018-live-chart-steady`, `020-live-chart-calm` and `024-mount-link-recovery`. Met:
   020-live-chart-calm's last story landed at f8dab49 and 024's at f0ea2d5. US1 needs
   024's `AxisState.mount_resets`, `Mount::resets`, the session's last-seen reset count
   and `mount.resets` in `GET /api/session` (FR-014, US1-S8). On 2026-11-17 024 joined
   `depends_on_landed`, § Interfaces' session and filter steps were re-read against its
   code, and the validator was re-run (spec.md § Clarifications, Session 2026-11-17).
2. **`main` is still as read.** Every citation here was read at c1ed295, re-read at
   f8dab49, re-read at 20ce2fe after 022, 024, 025, 027 and 028 had landed, and re-read
   at 2f6a777, where it is cited, after 021 had landed. A spec that lands before this
   one and edits a cited file means re-reading those citations and the shared writes,
   and re-running the validator.
3. **The owner approvals are given** (spec § Owner approvals). Flipping to `ready`
   records them.
4. **Constitution III is amended in the flip PR**, so the dispatch freezes it. The last
   sentence of Principle III, unchanged by 1.2.0 and 1.3.0
   (`.specify/memory/constitution.md:47`), reads:

   > A virtual clock is allowed only in tests and `finder --simulate`.

   It becomes:

   > A virtual clock is allowed only in tests, `finder --simulate`, and
   > `finder --mount-sky --mount-sky-night` started with `--covers-on`, where the
   > observer has confirmed the telescope is covered and the Sun guard checks the
   > virtual clock's Sun.

   The version goes from 1.3.0 to 1.4.0 (the next minor version, if another amendment
   lands first) and `Last Amended` to the flip's date.
5. **No operator-staged files are needed.**
6. **Files shared with 021 and 038 are ordered.** 021-chart-orientation and
   038-track-follows-a-push (a draft in pull request 238, not on `main`) write files
   this spec writes too. The 021 overlap is met: 021 landed on 2026-11-17, its US1 at
   158fb41 (#248) and its US2 at 2f6a777 (#249), and this trio was re-anchored to
   `main` at 2f6a777 (spec.md § Clarifications, Session 2026-11-17, re-anchored), with
   its citations in those files re-read, its write places named for `main` and the
   validator re-run. The overlap left is 038's, which the operator orders at the flip:

   | File | 026 | 021 (landed) | 038 |
   |---|---|---|---|
   | `crates/finder/README.md` | US1, US2 | US1 | US3 |
   | `GLOSSARY.md` | US2 | US1 | US1 |
   | `DESIGN.md` | US3 | US1, US2 | US3 |
   | `crates/finder/web/src/api.ts` | US3 | US1 | US3 |
   | `crates/finder/src/session.rs` | US1 | US1 | US1, US2 |
   | `crates/finder/web/dist` | US3 | US1, US2 | US3 |
   | `crates/finder/web/src/App.tsx` | US3 | US2 | |
   | `crates/finder/web/src/useSession.ts` | US3 | US1 | |
   | `crates/finder/tests/cli/main.rs` | US1, US2 | US1 | |

   Neither 021 nor 038 is in this spec's `depends_on_landed`, and this spec adds neither
   for ordering. The operator orders 026 and 038 at the flip; whichever lands later
   re-reads its citations in those files and its shared writes, and re-runs the
   validator. The imaging drafts 031 to 037 in the same pull request write several of
   these files too.
7. **The refinement probe has run.** Met on 2026-11-17, in one round: each story was
   built once, and the fixes are in this trio (§ Refinement Record, Round 1).

## Plan traps

Lessons from building 012 to 028 on this factory, each with its mechanism and the check
that keeps it from recurring. They bind every story.

1. **Gate evidence or nothing.** The judge credits only gate results and the story's
   diff. *Check:* every Then clause names a committed test that the `test` gate runs.
2. **A named command is read literally.** *Check:* commands are copied from
   § Verification.
3. **One evidence budget.** *Check:* each story keeps its diff under 64 KiB, measured
   with `git diff <base> -- . ':(exclude)crates/finder/web/dist' | wc -c`.
4. **The sandbox has only the gates' tools.** *Check:* no task runs a tool outside them.
5. **A spec fixed mid-epic needs a relaunch.** The judge grades the spec as dispatched.
   *Check:* no builder edits `specs/026-mount-sky/` or `.specify/`.
6. **An oracle copied from the code proves nothing.** *Check:* the tests compute the
   truth formula and the target's horizontal position themselves, and never call
   `MountSky` or `Model` to get an expected value.
7. **A solve that agrees by accident.** If the node's own camera pointed where the drawn
   sky does, US1-S1 and S3 would pass with no filter at all. *Check:* the `SimMount`'s
   zero points of `[300, 70]` keep the node's sky more than 40° away, and US1-S1 also
   asserts the zero counts. The `SimMount` starts at counts `[0, 0]`
   (`crates/simulator/src/mount.rs:85`), so the setup drives it to `[20000, 10000]`
   before the console is built, and a zero always fixed at `[0, 0]` fails US1-S1.
8. **A binary test that never reaches the mode.** *Check:* US1-S7 and US2-S3 read
   `mount_sky` from the running binary, not only its exit status.
9. **The bundle must be rebuilt.** *Check:* US3 commits `crates/finder/web/dist` after
   `npm run build`, so that the gate's `git status --porcelain -- dist` finds nothing.
10. **Each worktree builds on its own.** *Check:* no story sets `CARGO_TARGET_DIR`.
11. **A stop that only compares.** A filter that compared the reset count on each frame
    would draw again from a stale zero once the count read as before. *Check:* US1-S8
    sets the count back to 0, waits until `GET /api/session` omits `mount.resets`, and
    asserts the frames stay empty.
12. **A new top-level finder test fails the layout test, and one module lives in one
    binary.** Spec 027 groups finder's Rust tests into one binary per system, and its
    `layout` test refuses a file at the top level of `crates/finder/tests` or a new file
    in `tests/support/`. *Check:* each story adds the modules § Technical Context names,
    each with one `mod` line in its alphabetical place, and
    `cargo nextest run -p finder --locked --test layout` passes.
13. **A gate command is copied from `ergane.yaml`, never remembered.** The `test` gate's
    Rust half has been `cargo nextest run` since 2026-11-14. *Check:* § Verification and
    the spec's Independent Tests name
    `cargo nextest run --workspace --all-targets --all-features --locked`.
14. **A rig solver with a deadline fails on a busy machine.** Spec 028 gave every
    inline rig solver `timeout_ms: u64::MAX`, so virtual time cannot outrun a solve.
    *Check:* the rig the `motion` modules copy keeps it.
15. **A session field the page reads must pass through `useSession`.** `request()`
    copies only the fields it names. *Check:* the story that adds a field to
    `ObservingSession` adds it to `request` too, and its test renders `App` over a
    served session.

## Risks

| Risk | Mitigation |
|---|---|
| A drawn-sky pointing is taken for a real one. | The banner on every page, the start-up line, `mount_sky` in the session, and a separate state folder. |
| The observer forgets the covers in daylight. | `--covers-on` can't be skipped, and the warning line and banner repeat it. Physical safety still rests on the observer (owner approval 2). |
| The real tube hits a stop because the drawn altitude is off. | Zero at the measured altitude; README asks for a level check before long moves; a power cycle stops the drawn sky (FR-014). |
| A hand push makes the drawn sky part from the tube unseen. | The README and the banner forbid hand pushes (owner approval 5). Nothing detects one. |
| The first long slew on the real board misbehaves. | Only 1° position moves are proven on it (firmware 2.09). Stop, the watchdog and the observer at the power switch are the real ones. |
| Rendering slows the link loop. | `field_stars` takes milliseconds per frame at a 1 s cadence; frames carry at most 200 stars. |

## Refinement Record

Drafted on 2026-11-14 by the plate-solver operator against `main` at c1ed295, from a
read-only map of the console's frame, mount and simulate paths. No code was probed:
the interfaces were read from the cited lines, and the risk that `field_stars` and the
model's `predict` don't line up is carried by US1-S1 and S3. Writes come from the task
text. Shared writes name the merge edge that orders them, and the landed stories that
last wrote the same file on `main`. Nothing in this record is an owner
approval: those are in spec.md, and flipping to `ready` gives them.

Renumbered from 020 to 026 on 2026-11-14 by the plate-solver operator on the owner's
instruction, with the field findings of 13-14 November applied: the drawn sky stops on
spec 024's reset count (FR-014, US1-S8, US3-S4), hand pushes are forbidden in words, and
the real board's figures are recorded. `main` was merged at f8dab49 and every cited
`path:line` re-read there: none of the cited Rust, proto or constitution lines had
moved, since 020-live-chart-calm changed only `crates/finder/src/solve.rs`, the page and
`DESIGN.md`; the plan's places in `App.tsx`, `api.ts` and `DESIGN.md` are named, not
numbered, and still hold. The two new citations (`crates/simulator/src/mount.rs:12`,
`crates/scope-node/src/mount/skywatcher/protocol.rs:14`) were read there too.
`reviewed_revision` is therefore f8dab49. The shared writes were
re-derived from `git log` at f8dab49: 020-live-chart-calm's US2 is now the last writer
of `DESIGN.md`, `crates/finder/web/dist`, `App.tsx` and `api.ts`, so 020-live-chart-calm
joins `depends_on_landed`. 024 is not on `main`, so no 024 story can be named here yet;
when it lands, its US3 becomes the last writer of `api.rs` and `session.rs`
(§ Prerequisites 1 and 2).

**Refreshed on 2026-11-17 to `main` at 20ce2fe** by the plate-solver operator, after 024,
025, 027, 028 and 022 had landed and the constitution had gone to 1.3.0; `main` was
merged in, not rebased onto. Every `path:line` the trio held was re-read there by
content: 12 of 20 were unchanged and 8 had moved with their lines, none changing what
it cites. They are `crates/finder/src/link.rs` 435 → 486 (025 added the raw-frame
client), `crates/finder/src/goto.rs` 332 → 333, 711 → 716 and 811 → 816,
`crates/finder/src/session.rs` 768 → 810 and 1061 → 1123, and
`crates/finder/src/tracking.rs` 382 → 392 (024's reset count, 022's guidance), and the
test rig's `Rig::new`, line 188 of the old `mcp/support.rs`, now
`crates/finder/tests/support/mcp.rs:191` (027 moved the file). 024's code was read
against § Interfaces' session and filter steps: its names and shapes are the ones this
trio uses, and § Interfaces now cites them. 024 joins
`depends_on_landed`, and its US3 is the landed writer named for `api.rs`, `session.rs` and
`motion/main.rs`; 025 does not join, since this spec uses nothing it added. The planned
`tests/mount_sky.rs` and `tests/mount_sky_night.rs` would each fail the `layout` test,
so each became two modules (§ Technical Context), and US1's and US2's writes gain them and
the two `main.rs` files that declare them. The two `existing` tests that no longer
resolved now name their new paths, `crates/finder/tests/cli/simulate.rs` and
`crates/finder/tests/console/link.rs`. § Verification, the development checks and the
spec's Independent Tests name the `test` gate's `cargo nextest run`; the Principle III
amendment goes to 1.4.0; § Constitution Check answers Principle V; the README and
`DESIGN.md` places name 025's sections; and § Prerequisites 6 records the files shared
with the drafts 021 and 038. No scenario's Given, When or Then, no FR and no owner
approval changed. Nothing was probed in this refresh either: no story of this spec has
been built.

**Round 1 (probe builds), 2026-11-17.** Each story was built once, tests first and as
this trio tells a builder, in a throwaway worktree on c8f4408, with no existing test
edited and nothing pushed; the code is not kept. US1 (`probe/026-r1-us1`, `153ac24`):
8 tests, the `test` gate's `cargo nextest run` at 677 (669 before), a diff of 42,197 B,
declared writes only. US2 (`b54ce94`, on US1): 4 tests, 681, 20,595 B, declared writes
only. US3 (`8a6fffa`, on US2): 4 Vitest tests, the web gate and its e2e run (29) green,
7,443 B, plus one line of `crates/finder/web/src/useSession.ts` (8,029 B with it) that
was not a declared write and now is (D10, trap 15). The line-up of `field_stars` and
`predict` that the first paragraph left to US1-S1 and S3 held on the first build. The
fixes, D1 to D17, give the setup the target M42, no aim and counts away from zero, make
US1-S3 and S4 wait for a sync, make US1-S5, S6 and S8, US2-S2 and S4 and US3-S3 able to
fail, add `/setup` to US3-S1, kill a refused binary that starts listening, name
`useSession.ts`, the rig to copy and the Vitest rig, and correct the plan's text on
`run`'s allow, step 3's guards, the night clock's paths, the warning's, README's,
GLOSSARY's and `DESIGN.md`'s places and line forms, prettier and the e2e servers (spec.md
§ Clarifications, Session 2026-11-17, round-1 probe builds). Still caught by no test:
M05, the refraction step dropped (about 1.7′ at 30°, under the 0.05° tolerance), and
M22, start counts used in place of mid counts (no checked frame is taken while an axis
moves); M04, a zero always fixed at `[0, 0]`, and M08b, a preview kept on an unusable
frame, survived the probe and are answered by D3's and D5's fixes, which no probe has
run yet. Every citation the fixes add was read on this branch, whose code is `main` at
20ce2fe.

**Re-anchored on 2026-11-17 to `main` at 2f6a777** by the plate-solver operator, after
021-chart-orientation landed (US1 at 158fb41, #248; US2 at 2f6a777, #249); `main` was
merged in, not rebased onto. Between 20ce2fe and 2f6a777, outside `specs/`, only 021's
files and `ergane.yaml`'s roadmap widths changed, and the gates did not. Every
`path:line` the trio held was re-read at 2f6a777 by content: 8 had moved with 021's
added lines, none changing what it cites. In `crates/finder/src/session.rs` (021's
`chart_rotation_deg` fields) they are 514 → 518, 587 → 594, 810 → 820, 1045 → 1055,
1049 → 1059 and 1123 → 1133, and in `crates/finder/web/src/useSession.ts` (021's edit field and spread)
28 → 29 and 48 → 50. The citations into files that 020, 022, 024 and 025 touched had
not moved. The write places in 021's files were checked against `main` and now name it:
the `useSession.ts` spread still follows `backlash`, the last; `api.ts`'s `MountSky` goes
before `ObservingSession` and its field last, after `field?`; `DESIGN.md`'s entry still
follows `### Recording — built`, the last of `## Components`, and the line-form note
names its one-line neighbours, since 021's entry wraps; the `cli/main.rs` `mod` line,
the README's rows and section, the `GLOSSARY.md:30` line and the `App.tsx` banner place
hold as written. § Interfaces step 8 now says its `rotation` is not 021's chart
rotation. Nothing 021 built changes what the trio assumes: chart rotation turns only the
page's live views, the drawn frames are made at the link before anything reads them,
and the banner's scenarios read only its text. A test merge of the round-1 probe
branch onto 2f6a777 conflicted only in `crates/finder/web/dist`, which US3 rebuilds. The
021 overlap of § Prerequisites 6 is met. No scenario's Given, When or Then, no FR, no
owner approval and no Principle III text changed.

Shared writes name the story in this spec that writes the same file next and a landed
story in `depends_on_landed` that wrote it before; where a later landed spec not in
`depends_on_landed` wrote it last (025, 028, 022 or 021), the reason says so, and since it
is on `main` no order against it is left to keep. `crates/finder/tests/cli/main.rs` was
last written by 021's US1, and before it by 028's US3, 025's US1 and 027's US2, and
`GLOSSARY.md` by 021's US1, and before it by 022's US1 and 025's US1 and US2; none of
them is in `depends_on_landed`, and all are on `main`.

```yaml
version: 1
reviewed_revision: 2f6a777592b7ff2767e71317da90eb3376d179e0
stories:
  US1:
    tests:
      - path: crates/finder/tests/motion/mount_sky.rs
        status: new
        disposition: add
        assertion: "the first usable frame, at counts the setup drove away from zero, fixes zero_counts at its mid counts, and the live pointing reads azimuth 180 and altitude 30 within 0.05 deg while the node's own camera points elsewhere"
        decision: "spec.md:US1-S1"
      - path: crates/finder/tests/motion/mount_sky.rs
        status: new
        disposition: add
        assertion: "a filtered frame keeps id, times, size, exposure, gain and counts, drops its preview, and carries 20 to 200 centroids in non-increasing brightness"
        decision: "spec.md:US1-S2"
      - path: crates/finder/tests/motion/mount_sky.rs
        status: new
        disposition: add
        assertion: "once the model holds a sync, a GoTo to M42 ends done with the truth of the last frame's counts within 0.05 deg of M42; in the day, once a sync is held, it is refused with Sun is up and no motion command reaches the mount"
        decision: "spec.md:US1-S3..S4"
      - path: crates/finder/tests/motion/mount_sky.rs
        status: new
        disposition: add
        assertion: "a node without a mount gives frames with no centroids, no live pointing and mount_sky equal to tube_alt_deg 30 with null zero_counts"
        decision: "spec.md:US1-S5"
      - path: crates/finder/tests/cli/mount_sky.rs
        status: new
        disposition: add
        assertion: "the built binary refuses bad flag combinations before listening, never with clap's unexpected argument and with the range error for --tube-alt 95, and keeps its session in mount-sky/"
        decision: "spec.md:US1-S6..S7"
      - path: crates/finder/tests/motion/mount_sky.rs
        status: new
        disposition: add
        assertion: "once the session's reset count changes, every later frame, over more than the node's 5 s preview interval, has no centroids and no preview, power_cycled is true, zero_counts is unchanged and a GoTo is refused; a count set back to 0 and seen by the session does not resume drawing"
        decision: "spec.md:US1-S8"
      - path: crates/finder/tests/cli/simulate.rs
        status: existing
        disposition: preserve
        assertion: "--simulate and --sim-start refusals and the address line are unchanged"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/console/link.rs
        status: existing
        disposition: preserve
        assertion: "a link without a filter publishes the node's frames unchanged"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no file at the top level of crates/finder/tests or new file in its support folder, and each binary's main.rs holds only its support include and mod lines, with mount_sky added to motion's and cli's"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "every guard (Sun, minimum altitude, Stop, watchdog) behaves as without the mode (spec.md FR-007)"
      - "GET /api/session outside the mode is unchanged"
      - "--simulate is unchanged and refused with --mount-sky"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test motion mount_sky::"
        - "cargo nextest run -p finder --locked --test cli mount_sky::"
        - "cargo nextest run -p finder --locked --test layout"
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
      - crates/finder/tests/cli/main.rs
      - crates/finder/tests/cli/mount_sky.rs
      - crates/finder/tests/motion/main.rs
      - crates/finder/tests/motion/mount_sky.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/README.md
        other_spec: 026-mount-sky
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and adds its flags to US1's section"
      - path: crates/finder/src/api.rs
        other_spec: 026-mount-sky
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and adds with_night_clock"
      - path: crates/finder/src/main.rs
        other_spec: 026-mount-sky
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and adds the night flags"
      - path: crates/finder/src/mount_sky.rs
        other_spec: 026-mount-sky
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and adds night_utc"
      - path: crates/finder/tests/cli/main.rs
        other_spec: 026-mount-sky
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and adds mod mount_sky_night after US1's mod mount_sky"
      - path: crates/finder/tests/motion/main.rs
        other_spec: 026-mount-sky
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and adds mod mount_sky_night after US1's mod mount_sky"
      - path: crates/finder/README.md
        other_spec: 017-console-stop-confirm
        other_story: US4
        disposition: merge_order
        reason: "026 depends_on_landed 017, whose US4 changed the README; 025's US1 and US2, 022's US3 and 021's US1 wrote it later, and are on main"
      - path: crates/finder/src/api.rs
        other_spec: 024-mount-link-recovery
        other_story: US3
        disposition: merge_order
        reason: "026 depends_on_landed 024, whose US3 changed api.rs; 025's US1 and US2 wrote it later, and are on main"
      - path: crates/finder/src/lib.rs
        other_spec: 017-console-stop-confirm
        other_story: US1
        disposition: merge_order
        reason: "026 depends_on_landed 017, whose US1 changed lib.rs; 025's US1 and US2 wrote it later, and are on main"
      - path: crates/finder/src/link.rs
        other_spec: 017-console-stop-confirm
        other_story: US1
        disposition: merge_order
        reason: "026 depends_on_landed 017, whose US1 changed link.rs; 025's US2 wrote it later, and is on main"
      - path: crates/finder/src/main.rs
        other_spec: 016-mcp
        other_story: US1
        disposition: merge_order
        reason: "026 depends_on_landed 016, whose US1 changed main.rs; 025's US1 wrote it later, and is on main"
      - path: crates/finder/src/session.rs
        other_spec: 024-mount-link-recovery
        other_story: US3
        disposition: merge_order
        reason: "026 depends_on_landed 024, whose US3 changed session.rs; 028's US4, 022's US3 and 021's US1 wrote it later, and are on main"
      - path: crates/finder/tests/motion/main.rs
        other_spec: 024-mount-link-recovery
        other_story: US3
        disposition: merge_order
        reason: "026 depends_on_landed 024, whose US3 added mod mount_identity; 025's US2 wrote it later, and is on main"
    open_questions: []
  US2:
    tests:
      - path: crates/finder/tests/cli/mount_sky_night.rs
        status: new
        disposition: add
        assertion: "--mount-sky-night without --covers-on, without --mount-sky, and --covers-on alone each exit non-zero before listening, the first naming --covers-on and the others never with clap's unexpected argument"
        decision: "spec.md:US2-S1..S2"
      - path: crates/finder/tests/cli/mount_sky_night.rs
        status: new
        disposition: add
        assertion: "with every flag the running binary reports night_utc and a session time within 60 s of the night start, and prints the pretend Sun warning"
        decision: "spec.md:US2-S3"
      - path: crates/finder/tests/motion/mount_sky_night.rs
        status: new
        disposition: add
        assertion: "a day console built with with_night_clock in place of with_clock reports night_utc and a time within 1 s of the night, and one built without has no night_utc key and a time within 1 s of the day"
        decision: "spec.md:US2-S4"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "each binary's main.rs holds only its support include and mod lines, with mount_sky_night added to cli's and motion's"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "without --mount-sky-night the console clock is the system clock (constitution III)"
      - "--simulate is unchanged"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test cli mount_sky_night::"
        - "cargo nextest run -p finder --locked --test motion mount_sky_night::"
        - "cargo nextest run -p finder --locked --test layout"
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
      - crates/finder/tests/cli/main.rs
      - crates/finder/tests/cli/mount_sky_night.rs
      - crates/finder/tests/motion/main.rs
      - crates/finder/tests/motion/mount_sky_night.rs
    no_write_reason: ""
    shared_writes: []
    open_questions: []
  US3:
    tests:
      - path: crates/finder/web/src/components/MountSkyBanner.test.tsx
        status: new
        disposition: add
        assertion: "a session with mount_sky shows one status saying Drawn sky, not seen by the camera and No hand pushes, at / and at /setup; with night_utc it adds the night clock and the covers; with power_cycled it says the mount was power-cycled and to restart with a fresh --tube-alt; without mount_sky, once the target's guidance shows, nothing says Drawn sky"
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
      - crates/finder/web/src/useSession.ts
    no_write_reason: ""
    shared_writes:
      - path: DESIGN.md
        other_spec: 020-live-chart-calm
        other_story: US2
        disposition: merge_order
        reason: "026 depends_on_landed 020-live-chart-calm, whose US2 changed DESIGN.md; 025's US3, 022's US1 to US3 and 021's US1 and US2 wrote it later, and are on main"
      - path: crates/finder/web/dist
        other_spec: 020-live-chart-calm
        other_story: US2
        disposition: merge_order
        reason: "026 depends_on_landed 020-live-chart-calm, whose US2 rebuilt the bundle; 025's US3, 022's US1 to US3 and 021's US1 and US2 rebuilt it later, and are on main"
      - path: crates/finder/web/src/App.tsx
        other_spec: 020-live-chart-calm
        other_story: US2
        disposition: merge_order
        reason: "026 depends_on_landed 020-live-chart-calm, whose US2 changed App.tsx; 021's US2 wrote it later, and is on main"
      - path: crates/finder/web/src/api.ts
        other_spec: 020-live-chart-calm
        other_story: US2
        disposition: merge_order
        reason: "026 depends_on_landed 020-live-chart-calm, whose US2 changed api.ts; 025's US3, 022's US1 and US3 and 021's US1 wrote it later, and are on main"
      - path: crates/finder/web/src/useSession.ts
        other_spec: 016-mcp
        other_story: US7
        disposition: merge_order
        reason: "026 depends_on_landed 016, whose US7 changed useSession.ts; 025's US3 and 021's US1 wrote it later, and are on main"
    open_questions: []
```
