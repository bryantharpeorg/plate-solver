# Implementation Plan: Push-To in the Observer's Own Directions

**Branch**: `022-altaz-guidance` | **Date**: 2026-11-13, refined 2026-11-14 | **Spec**: [spec.md](spec.md)

This plan records every design decision for the feature. A story that would change one
escalates rather than re-deciding it.

## Summary

The guidance block leads with the push in the observer's words, `right 2.7° · down 1.4°`,
to a tenth of a degree, and prints the distance left on the sky under it, from a new
guidance field, `separation_deg`. The old turn-and-tilt line stays, set quiet. The live
chart draws a mark at the aim point. The session's pointing and the live event gain
`up_deg`, where the zenith points in the frame, and the chart draws up and right from it
at the aim point. The numbers the console computes do not change, and the chart is not
turned.

## Technical Context

**Language/Version**: Rust 2021 for `crates/finder`; TypeScript with React 19 for the
page.

**Primary Dependencies**: nothing new.

**Storage**: none. Nothing here is kept in `session.json`.

**Testing**:
- US1: `crates/finder/tests/pushto_separation.rs` (new) for the field, and
  `crates/finder/web/src/components/GuidancePush.test.tsx` (new) for the block.
- US2: `crates/finder/web/src/components/ChartAim.test.tsx` (new).
- US3: `crates/finder/tests/pointing_up.rs`,
  `crates/finder/web/src/components/ChartUp.test.tsx` and
  `crates/finder/web/src/useLiveUp.test.ts` (all new).

**Constraints**:
- `web/dist` is committed and must be the build of its source.
- The page does no sky maths (`crates/finder/web/src/components/GuidanceBlock.tsx:1`).
  Both new numbers come from the server.
- `turn_deg`, `tilt_deg` and the target's placement keep their meaning and their values.

## Constitution Check

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | The solver and `crates/math-core` are untouched. Both new fields are derived from a solved frame with functions that already exist. |
| II. Never weaken a gate | No existing test, golden, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS` or `ergane.yaml` is edited. New tests go in new files. |
| III. Tests first, independent expectations | The oracles are the haversine formula, the textbook parallactic angle against the solve report's own north, and bearings computed from SVG attributes. |
| IV. Critical paths | No `unsafe`, no dependency change, nothing under `proto/`. |
| V. Standalone Rust, honest licensing | Unaffected. |

## Project Structure

```text
crates/finder/
├── src/pushto.rs                             # US1: separation_deg; US3: up_deg()
├── src/session.rs                            # US3: LatestPointing.up_deg
├── src/live.rs                               # US3: LiveReport.up_deg
├── README.md                                 # US3: the live event's fields
├── tests/pushto_separation.rs                # US1: new
├── tests/pointing_up.rs                      # US3: new
└── web/
    ├── src/api.ts                            # US1/US3: wire fields
    ├── src/useLive.ts                        # US3: keep up_deg on the live pointing
    ├── src/components/GuidanceBlock.tsx      # US1: push line, away, quiet old line
    ├── src/components/Chart.tsx              # US2: AimMark; US3: UpMark
    ├── src/index.css                         # US2/US3: the marks' ink
    ├── src/components/GuidancePush.test.tsx  # US1: new
    ├── src/components/ChartAim.test.tsx      # US2: new
    ├── src/components/ChartUp.test.tsx       # US3: new
    ├── src/useLiveUp.test.ts                 # US3: new
    └── dist/                                 # rebuilt by every story
GLOSSARY.md                                   # US1: Turn
DESIGN.md                                     # US1: § Guidance block, § Typography; US2, US3: new sections after § Target reticle
```

## Interfaces

### US1: the field (`crates/finder/src/pushto.rs`)

- `Guidance` (`crates/finder/src/pushto.rs:64`) gains, after `tilt_deg`
  (`crates/finder/src/pushto.rs:84`):

  ```rust
  /// Angle on the sky from the pointing to the target, in degrees.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub separation_deg: Option<f64>,
  ```

- `guidance` builds the struct once (`crates/finder/src/pushto.rs:103` to
  `crates/finder/src/pushto.rs:113`); it gets `separation_deg: None` there. Where turn
  and tilt are set (`crates/finder/src/pushto.rs:131` to
  `crates/finder/src/pushto.rs:136`) it sets
  `angular_distance(a, b).to_degrees()`, with `a` the pointing's `ra_deg` and `dec_deg`
  and `b` the target's, each through `UnitVector::from_radec` in radians. Both are
  already imported (`crates/finder/src/pushto.rs:3`). Both positions are J2000, and the
  angle between two directions does not depend on the epoch. The `if let` there binds
  only `(alt, az)` (`pointing.and_then(|p| p.alt_deg.zip(p.az_deg))`), so bind the
  pointing as well:
  `if let Some((p, (alt, az))) = pointing.and_then(|p| Some((p, p.alt_deg.zip(p.az_deg)?)))`.
- `guidance_with_source` clears it with the other two
  (`crates/finder/src/pushto.rs:152` to `crates/finder/src/pushto.rs:156`).
- `Guidance` in `crates/finder/web/src/api.ts:44` gains `separation_deg?: number`.

### US1: the block (`crates/finder/web/src/components/GuidanceBlock.tsx`)

The fragment that prints directions (`crates/finder/web/src/components/GuidanceBlock.tsx:76`
to `crates/finder/web/src/components/GuidanceBlock.tsx:82`) becomes four lines, in this
order:

```tsx
<p className="text-2xl leading-tight font-semibold" data-push>
  {push(guidance.turn_deg, "right", "left")} · {push(guidance.tilt_deg, "up", "down")}
</p>
{guidance.separation_deg !== undefined && (
  <p className="quiet" data-away>{tenths(guidance.separation_deg).toFixed(1)}° away</p>
)}
<p className="quiet">turn … · tilt …</p>   {/* today's text, unchanged; was font-semibold */}
{ageText && <p className="quiet">Pointing {ageText} ago.</p>}
```

```ts
/** A magnitude to a tenth of a degree, rounded half up. */
const tenths = (degrees: number) => Math.round(Math.abs(degrees) * 10) / 10;

/** One axis of the push to a tenth of a degree. A reading that rounds to zero keeps the positive word. */
function push(degrees: number, positive: string, negative: string): string {
  const t = tenths(degrees);
  return `${degrees < 0 && t > 0 ? negative : positive} ${t.toFixed(1)}°`;
}
```

The magnitude is rounded, not the signed value, because `Math.round` sends −13.5 to −13
and 13.5 to 14. "Away" goes through the same `tenths`: `toFixed(1)` alone prints 0.35 as
"0.3". Nothing outside this fragment prints either new line, so a stale, down or refused
target shows neither. Prettier sets the `data-away` paragraph over three lines; run
`npm run format`.

### US1: the words

- `GLOSSARY.md:37` to `GLOSSARY.md:39` become:

  > **Turn**:
  > A direction of motion in azimuth: clockwise or anticlockwise, as seen from above. The
  > page prints a clockwise turn as "right" and an anticlockwise one as "left", as the
  > observer faces where the telescope points.
  > _Avoid_: pan

- `DESIGN.md` § Guidance block (`DESIGN.md:290`): replace the sentence "Pointable
  guidance gives the whole-degree turn and tilt and the latest pointing's age, or asks
  for a solved frame." with:

  > Pointable guidance leads with the push in the observer's own directions, "right 2.7°
  > · down 1.4°", in the Push style, to a tenth of a degree. Under it, in Body Quiet, are
  > the distance left on the sky ("2.8° away"), the whole-degree turn and tilt, and the
  > latest pointing's age. Without a pointing it asks for a solved frame.

- `DESIGN.md` § Typography › Hierarchy gains, after Headline (`DESIGN.md:215`):

  > - **Push** (600, 1.5rem, line-height 1.25): the push line in the guidance block, the
  >   largest text there.

### US2: the aim mark (`crates/finder/web/src/components/Chart.tsx`)

Directly after the furniture (`crates/finder/web/src/components/Chart.tsx:233`), inside
the same clipped group:

```tsx
{live && session?.aim && <AimMark x={session.aim.x} y={session.aim.y} W={W} u={u} />}
```

```tsx
function AimMark({ x, y, W, u }: { x: number; y: number; W: number; u: number }) {
  const r = 9 * u;
  const labelRight = x < W / 2;
  return (
    <g data-aim transform={`translate(${x} ${y})`} className="aim-mark pointer-events-none" strokeWidth={1.6 * u}>
      <title>Aim point: where the telescope looks</title>
      <circle className="aim-ring" r={r} />
      <circle className="aim-dot" r={1.6 * u} />
      <text x={(labelRight ? 1 : -1) * (r + 5 * u)} y={4 * u} textAnchor={labelRight ? "start" : "end"}
            fontSize={12 * u} strokeWidth={3 * u}>aim</text>
    </g>
  );
}
```

It is not gated on a solved frame: the aim point is a pixel of the camera. `live` is the
chart's own prop (`crates/finder/web/src/components/Chart.tsx:125`), so uploads and
samples never get it. Prettier sets the `<text>` element over several lines.

`ChartAim.test.tsx` and `ChartUp.test.tsx` render `Chart` with their own helper that
passes `live` and `session`; the helper in `crates/finder/web/src/components/Chart.test.tsx`
passes neither. `ORION2` is 507 × 507, so the scenarios use the aim (320.5, 190).

`crates/finder/web/src/index.css`, between the `.furniture` rule
(`crates/finder/web/src/index.css:436` to `crates/finder/web/src/index.css:439`) and
`.furniture text` (`crates/finder/web/src/index.css:440`), where the hotfix put them:

```css
.aim-mark .aim-ring { fill: none; stroke: var(--color-on-photo); }
.aim-mark .aim-dot { fill: var(--color-on-photo); stroke: none; }
.aim-mark text {
  fill: var(--color-on-photo);
  paint-order: stroke;
  stroke: var(--color-photo);
  stroke-linejoin: round;
  font-weight: 600;
}
[data-theme="night"] .aim-mark .aim-ring { stroke-width: 2.2px; vector-effect: non-scaling-stroke; }
[data-theme="night"] .aim-mark text { font-weight: 680; }
```

`DESIGN.md` gains, after § Target reticle:

> ### Aim mark — built
> On a live chart with an aim point set, a Frame Ink ring of 9 chart units with a centre
> dot marks the pixel the eyepiece is centred on, with the upright word "aim" toward the
> frame's middle, cased like a star name. It is session furniture, outside the legend,
> and it leaves the centre reticle where it is. At night the ring is 2.2px and
> non-scaling.

### US3: where up is (`crates/finder/src/pushto.rs`, `session.rs`, `live.rs`)

- `pushto.rs` gains:

  ```rust
  /// Where the zenith points in the frame at a J2000 position, degrees clockwise from the
  /// frame's up, in [0, 360). Right, the way azimuth grows, is a quarter turn clockwise
  /// from it.
  pub fn up_deg(frame: &Frame, ra_deg: f64, dec_deg: f64, observer: &Observer) -> Option<f64> {
      let at = frame.project(UnitVector::from_radec(ra_deg.to_radians(), dec_deg.to_radians()))?;
      let (ra, dec) = frame.unproject((at.0, at.1 - 0.01))?.to_radec();
      let (alt, az) = sky::alt_az(ra_deg, dec_deg, observer);
      let (up_alt, up_az) = sky::alt_az(ra.to_degrees(), dec.to_degrees(), observer);
      let right = ((up_az - az + 180.0).rem_euclid(360.0) - 180.0) * alt.to_radians().cos();
      // The frame's up lies this far clockwise of the zenith; the zenith lies as far the other way.
      let roll = right.atan2(up_alt - alt).to_degrees();
      Some((-roll).rem_euclid(360.0))
  }
  ```

  This is the pointing model's own method (`crates/finder/src/model.rs:278` to
  `crates/finder/src/model.rs:288`), turned to give the zenith's bearing in the frame;
  it equals the model's roll negated to under 1e-5°. The probe compiled it as written.
  Because the projection does not keep angles far off axis, it differs from north less
  the parallactic angle by up to about 0.25° 6° from the centre of a 12° frame, and by
  0.13° to 0.15° where the scenarios look.
  Going through `sky::alt_az` (`crates/finder/src/sky.rs:229`) for both rays takes in
  the parallactic angle and the precession of north. `Frame::project` and
  `Frame::unproject` are at `crates/finder/src/sky.rs:101` and
  `crates/finder/src/sky.rs:109`.
- `LatestPointing` (`crates/finder/src/session.rs:424`) gains:

  ```rust
  /// Where the zenith points in the frame at this pointing, degrees clockwise from up,
  /// when a site is set.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub up_deg: Option<f64>,
  ```

  The view builds it (`crates/finder/src/session.rs:614` to
  `crates/finder/src/session.rs:618`) from `self.site`, the pointing's own `frame`,
  `ra_deg`, `dec_deg` and `time`, through `pushto::up_deg`. **`Pointing`
  (`crates/finder/src/session.rs:461`) gets no new field** (§ Shapes that existing tests
  pin).
- `LiveReport` (`crates/finder/src/live.rs:53`) gains `up_deg: Option<f64>`, skipped when
  absent. The live loop already takes a session snapshot for guidance
  (`crates/finder/src/live.rs:243`); it keeps the snapshot and sends
  `pointing_seq.and(snapshot.pointing.and_then(|p| p.up_deg))`, so an unsolved frame
  carries none.

### US3: the page

- `crates/finder/web/src/api.ts`: `LiveReport` (line 130) and `SessionPointing`
  (line 146) each gain `up_deg?: number`.
- `crates/finder/web/src/useLive.ts`: the rebuilt live pointing
  (`crates/finder/web/src/useLive.ts:46` to `crates/finder/web/src/useLive.ts:55`) gains
  `...(solved.up_deg !== undefined && { up_deg: solved.up_deg })`. The key is absent,
  not undefined, when the event has none.
- `crates/finder/web/src/components/Chart.tsx`, after the aim mark, on the latest
  pointing's chart like the target mark (`crates/finder/web/src/components/Chart.tsx:234`
  to `crates/finder/web/src/components/Chart.tsx:241`), but needing no target. The test
  on `up_deg` is `!== undefined`, never truthiness, which would drop the mark at 0° and
  leave a stray `0` text node:

  ```tsx
  {solved &&
    session?.pointing &&
    session.pointing.up_deg !== undefined &&
    report.pointing_seq !== undefined &&
    report.pointing_seq === session.pointing.seq && (
      <UpMark at={live && session.aim ? session.aim : { x: W / 2, y: H / 2 }} up={session.pointing.up_deg} u={u} />
    )}
  ```

  ```tsx
  function UpMark({ at, up, u }: { at: { x: number; y: number }; up: number; u: number }) {
    const out = (bearing: number, r: number) => {
      const a = (bearing * Math.PI) / 180;
      return { x: at.x + Math.sin(a) * r, y: at.y - Math.cos(a) * r };
    };
    const label = { textAnchor: "middle", dominantBaseline: "middle", fontSize: 11 * u, strokeWidth: 3 * u } as const;
    return (
      <>
        <g data-up transform={`translate(${at.x} ${at.y}) rotate(${up})`} className="up-mark pointer-events-none" strokeWidth={1.3 * u}>
          <title>Up and right at the aim point</title>
          <path fill="none" d={`M0 ${-13 * u}V${-34 * u}M${-4 * u} ${-28 * u}L0 ${-34 * u}L${4 * u} ${-28 * u}M${13 * u} 0H${22 * u}`} />
        </g>
        <text data-up-label="up" className="up-label pointer-events-none" {...out(up, 43 * u)} {...label}>up</text>
        <text data-up-label="right" className="up-label pointer-events-none" {...out(up + 90, 31 * u)} {...label}>right</text>
      </>
    );
  }
  ```

  The labels sit outside the group, so they carry `pointer-events-none` themselves;
  without it they catch taps meant for the stars beneath them and clear the pick through
  the SVG's `onClick`. Prettier reflows the group, the path and both labels.

  In an image of the sky that is not mirrored, right is a quarter turn clockwise from up,
  as east is a quarter turn counter-clockwise from north
  (`crates/finder/web/src/components/Chart.tsx:532`). The arms start 13 units out, clear
  of the aim ring (9 units) and the centre reticle (11 units).
- `crates/finder/web/src/index.css`, after the aim mark's rules:

  ```css
  .up-mark { stroke: var(--color-on-photo); }
  .up-label {
    fill: var(--color-on-photo);
    paint-order: stroke;
    stroke: var(--color-photo);
    stroke-linejoin: round;
    font-weight: 600;
  }
  [data-theme="night"] .up-mark path { stroke-width: 1.8px; vector-effect: non-scaling-stroke; }
  [data-theme="night"] .up-label { font-weight: 680; }
  ```

- `crates/finder/README.md:380` lists the live event's fields; add `up_deg?` after
  `guidance?`, and say it is present when the session has a site.
- `DESIGN.md` gains, after § Aim mark:

  > ### Up mark — built
  > When the session has a site, the chart draws which way is up and which way is right
  > at the aim point (the frame's centre without one): two Frame Ink arms from 13 chart
  > units out, the longer toward the zenith with an arrowhead, the shorter toward growing
  > azimuth, with the upright words "up" and "right" beyond them. It is session
  > furniture, outside the legend, drawn only on the latest pointing's chart. The
  > compass's north and east stay as they are.

## Shapes that existing tests pin

These are not changed by any story:
- **The old line's words.** `crates/finder/web/src/components/GuidanceBlock.test.tsx:31`
  and `crates/finder/web/src/components/GuidanceBlock.test.tsx:41` find it by its exact text, `crates/finder/web/src/App.live.test.tsx:151` and
  `crates/finder/web/src/App.live.test.tsx:162` want it in the block's text, and `crates/finder/web/e2e/simulate.spec.ts:191`
  wants `/turn .*tilt /` in the guidance region. Its paragraph keeps its text and changes
  only its class.
- **No "turn " or "tilt " without directions.**
  `crates/finder/web/src/components/GuidanceBlock.test.tsx:16` and
  `crates/finder/web/src/App.live.test.tsx:157` reject either word when there are no
  directions. The new lines print only inside the directions fragment, and their words
  are "right", "left", "up", "down" and "away".
- **Ink only.** `crates/finder/web/src/components/GuidanceBlock.test.tsx:34` wants no
  class containing `icon-` or `object-` in the block.
- **`Pointing`'s fields.** A unit test builds `crate::session::Pointing` field by field
  (`crates/finder/src/pushto.rs:421` to `crates/finder/src/pushto.rs:429` at c1ed295;
  after US1 lands it sits at `crates/finder/src/pushto.rs:439` to
  `crates/finder/src/pushto.rs:447`). A new field
  on it would stop that test compiling. `up_deg` therefore lives on `LatestPointing`,
  which only `crates/finder/src/session.rs:614` builds. `Guidance` and `LiveReport` are
  each built in one place outside tests (`crates/finder/src/pushto.rs:103`,
  `crates/finder/src/live.rs:244`).
- **Five chips.** `crates/finder/web/src/components/Chart.test.tsx:23` to `crates/finder/web/src/components/Chart.test.tsx:29` list
  them with their counts. Neither mark is a layer.
- **One furniture group.** `crates/finder/web/e2e/night-legibility.spec.ts:118`, `crates/finder/web/e2e/night-legibility.spec.ts:149`
  and `crates/finder/web/e2e/night-legibility.spec.ts:150` read a style off `chart.locator(".furniture")`, which Playwright refuses if
  two elements match. Neither mark uses the class `furniture`, and neither is drawn
  inside that group.
- **The marks the tests count.** `.mark-matched`, `.mark-expected` and `.mark-detected`
  (`crates/finder/web/src/components/Chart.test.tsx:48` to `crates/finder/web/src/components/Chart.test.tsx:60`), and `.mark-label` and
  `.object-label` read first-of-kind by
  `crates/finder/web/e2e/night-legibility.spec.ts:115` and `crates/finder/web/e2e/night-legibility.spec.ts:116`. The new marks use
  their own classes.
- **The first quiet line.** `crates/finder/web/e2e/night-legibility.spec.ts:76` reads
  the page's first `.quiet` element. With a target set that may now be "away" or the old
  line; both carry the same class and take the same style.
- **The live hook's shapes.** `crates/finder/web/src/useLive.test.ts:89` and `crates/finder/web/src/useLive.test.ts:151`
  compare whole objects. Their events have no `up_deg`, so the rebuilt pointing has no
  such key. Neither looks at the rebuilt pointing itself; US3-S5 is what checks it.
- **One status, one Minimum altitude.** Not touched here; this spec adds nothing to Setup.
- **The target's placement.** `crates/finder/tests/pushto_api.rs:165` and `crates/finder/tests/pushto_api.rs:185` pin
  it to the solve report's marks. It is not touched.

## Decisions and the alternatives rejected

| Decision | Rejected | Why |
|---|---|---|
| Left and right count azimuth degrees | Degrees on the sky, `turn_deg × cos(altitude)` | Azimuth is exact for the mount at every altitude and is the field the tests check. The on-sky figure breaks down near the zenith. The owner chose azimuth on 2026-11-14. |
| Print the on-sky distance as one number, "away" | Left and right in both units | Two pairs of numbers that differ by a cosine would be read as a disagreement. One distance can be held against the eyepiece's field. |
| A tenth of a degree | Arcminutes below 1° | One unit all the way in. A tenth is 6′, inside the 14 mm eyepiece's 47′. |
| A zero reads "right" and "up" | Dropping the word at zero | The line keeps its shape, and the word does not flicker on the target. |
| Keep the old line, quiet | Deleting it | Four existing assertions pin its words, and no builder may edit a test. |
| The server computes `separation_deg` | The page, from turn, tilt and altitude | The page does no sky maths, and it has no altitude on a live pointing. |
| `up_deg` on `LatestPointing` | On `Pointing`, as 021's plan has it | An existing unit test builds `Pointing` field by field. |
| `up_deg` on the live event too | The session poll alone | The page rebuilds a live pointing from the event; a polled field would come and go. |
| The two-ray method | The zenith's J2000 direction and a new `Frame::toward_at` (021's plan) | It needs no reverse precession, and it is the method the pointing model already uses and tests. |
| An aim mark beside the reticle | Moving the reticle to the aim point | `furnitureGeometry` puts the reticle at the centre for the label layout and for uploads; the mark is the smaller change. |
| Marks drawn in the frame's coordinates, labels without `transform` | Marks drawn after any turn | They turn with the frame under 021, and its rule for upright labels then applies to them. |
| Up and right at the aim point | A second compass in a corner | The target's mark is read against the aim point; the arms belong where the push starts. |
| No line from the aim point to the target | Drawing the push as a line | The mount moves along an altitude circle and a vertical, not along that line. |
| No horizon grid | A sixth chip, or a grid switched by a setting | Five chips are pinned; it needs the reverse of `sky::alt_az`; and where the choice lives is its own decision (spec § Owner approvals, 8). |

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
- **CI in the merge queue** runs every required `ci.yml` job. The coverage floor runs
  only by hand since 2026-11-14.

## Prerequisites before this spec is flipped to `ready`

1. **The earlier specs have landed:** 013, 015, 016, 017, 018, 019 and 020. All but 020
   are met at c1ed295; `depends_on_landed` holds this spec until 020 lands.
2. **`main` is still as read, once 020 has landed.** Every citation here is at c1ed295.
   020 edits `crates/finder/web/src/components/Chart.tsx`,
   `crates/finder/web/src/api.ts` and `DESIGN.md`; re-read the lines cited in those three
   after it lands and correct them here.
3. **The owner approvals are given.** The unit for left and right and US3 were chosen on
   2026-11-14; flipping gives the rest.
4. **021 waits for this one.** It names `022-altaz-guidance` in `depends_on_landed`.
5. **The hotfix stays off `main`.** If it were committed first, T004 and T008 would start
   from its code, which already has the markup named here.
6. **The refinement probe has run** (§ Refinement Record).

## Plan traps

1. **Gate evidence or nothing.** *Check:* every Then clause names a committed test that
   a gate runs.
2. **A named command is read literally.** *Check:* commands are copied from
   § Verification.
3. **One evidence budget.** *Check:* each story keeps its diff under 64 KiB, excluding
   `crates/finder/web/dist`. The probe's stories ran 19, 9 and 26 KiB, server and page
   halves together.
4. **jsdom has no layout or SVG geometry.** *Check:* page tests assert text, attributes
   and bearings computed from `x` and `y` attributes, never `getCTM`, `getBBox` or real
   sizes.
5. **A spec fixed mid-epic needs a relaunch.** *Check:* no builder edits
   `specs/022-altaz-guidance/`.
6. **The bundle must be rebuilt.** *Check:* commit `crates/finder/web/dist`.
7. **Each worktree builds on its own.** *Check:* no story sets `CARGO_TARGET_DIR`.
8. **A new struct field can break a test that builds the struct.** *Check:* no field is
   added to `session::Pointing`; `cargo clippy --all-targets` compiles
   `crates/finder/src/pushto.rs`'s own tests.
9. **Rounding a signed number.** *Check:* `push` and "away" round the magnitude through
   `tenths`, and US1-S2, US1-S3 and US1-S4 pass.
10. **A truthiness test drops zero.** *Check:* the up mark's condition is
    `up_deg !== undefined`, and US3-S9 passes.
11. **A label outside a `pointer-events-none` group still takes taps.** *Check:* both
    `data-up-label` texts carry the class themselves (US3-S6).
12. **An absence-only scenario is green before the change.** *Check:* US1-S6, US1-S8,
    US1-S9, US2-S2, US3-S2 and US3-S8 each also assert the positive case.
13. **The fixture is square.** `ORION2` is 507 × 507. *Check:* page scenarios use an aim
    inside it, (320.5, 190).
14. **Formatting is the repository's.** Prettier reflows several of the plan's snippets.
    *Check:* run `npm run format`, then `npm run format:check`.

## Risks

| Risk | Mitigation |
|---|---|
| Azimuth degrees over-read the push high in the sky (twice at 60°). | "Away" is printed beside them; the owner chose the unit. |
| A star's label overlaps a mark. | The marks are drawn after the stars and keep cased labels. The label layout is not changed here; adding their boxes to it is a later refinement. |
| The up mark crowds the target's mark when the target is within about 40 chart units of the aim point. | That is when the target is in the eyepiece, and the numbers lead. The mark has no fill. |
| `up_deg` is ill-defined at the zenith. | `atan2` still returns a value; within a degree of the zenith the mark is not meaningful, and a Dobsonian cannot be pushed there by these numbers either. |
| The night e2e reads a different first `.quiet` element. | In that test no target is set, so its first `.quiet` is Survey's note before and after; with a target it would be "away" or the age line, both styled by `.quiet` itself. |
| The word "right" lands near the centre reticle. | With the night's geometry (aim 121 px from the centre, up 78°) it falls about 10 CSS px from the reticle on a 412 px phone. Both are thin ink; the owner sees it on the next night out. |
| The pointable block is taller on a phone. | About 66 px more with "away"; the legend may leave a 390 × 844 first screen. No e2e measures that state; the owner checks it (spec § Edge Cases). |
| The sign of `up_deg` on the real camera. | Checked on the simulator and against the model's roll; the owner confirms about 78° on the next night out (SC-004). |

## Refinement Record

Drafted on 2026-11-13 by the plate-solver operator against `main` at ce80f67, from the
owner's report at the lake site, three measurements of the camera's angle that night, and
the hotfix built there. On 2026-11-14 the plan was applied to throwaway worktrees of
c1ed295, tests first, with no existing test edited:

- **Server** (branch `probe/rust`, after 020 US1): US1 (`5f66075`) and US3 (`3c96ab2`).
  `separation_deg` matched the haversine on the read's own fields to 8.9e-16°; M8 sat at
  −66.6° altitude in the rig. `up_deg` compiled as written and stayed within 0.145° (S1)
  and 0.129° (S3) of north less the parallactic angle; US3-S4 was written on
  `crates/finder/tests/live/support.rs`, with sequence numbers matching on every event.
  `fmt` and `clippy -p finder --all-targets` passed; the full `cargo test -p finder` after
  US1 passed 391 tests; after US3 the lib tests and the `pointing_up`, `api`,
  `pushto_api`, `freshness_api`, `live_stream`, `live`, `pointing_api`, `e2e_live`,
  `mcp_frames`, `session_api` and `pushto_separation` binaries passed.
- **Page** (branch `probe/022-page`): US1 (`aa852b3`), US2 (`9ffdd3a`) and US3 (`0b10da6`).
  `npm test` went from 258 to 274 passing with every existing test unchanged;
  typecheck, `format:check` and build passed, and `dist` was clean after each rebuild.
  The probe found the out-of-frame aim, the tap-swallowing labels, the zero bearing, the
  two roundings and the fifth pin on the old line; this trio carries each fix.
- **Not run on the Surface:** `CI=1 npm run e2e` (no Playwright browsers), and the
  workspace-wide Cargo gates. The `test` gate runs both. The e2e tests most at risk are
  the night red-only checks with the aim and up marks drawn:
  `crates/finder/web/e2e/goto.spec.ts:206`, `crates/finder/web/e2e/tracking.spec.ts:135`
  and `crates/finder/web/e2e/simulate.spec.ts:274`, and the stale check at
  `crates/finder/web/e2e/simulate.spec.ts:191`.

Writes come from the task text. Shared writes name the story in this spec that writes the
same file next, and the landed story that last wrote it (020's US2 for the files 020
touches). Nothing in this record is an owner approval: those are in spec.md, and flipping
to `ready` gives them.

```yaml
version: 1
reviewed_revision: c1ed295734253c7d6121052b0a14beb2ee64bc17
stories:
  US1:
    tests:
      - path: crates/finder/tests/pushto_separation.rs
        status: new
        disposition: add
        assertion: "separation_deg equals the haversine angle between the read's pointing and target to 1e-6 deg; absent for M8 below the horizon and present again for M78; absent with no pointing; present on a 5.0 s live pointing and cleared with turn and tilt on a 5.1 s one"
        decision: "spec.md:US1-S7..S9"
      - path: crates/finder/web/src/components/GuidancePush.test.tsx
        status: new
        disposition: add
        assertion: "right 33.6 up 12.4; left 2.7 down 1.4; zero keeps right and up; 2.8 away and 0.4 away only with the field; push line before the quiet old line; neither line without directions, both with them"
        decision: "spec.md:US1-S1..S6"
      - path: crates/finder/web/src/components/GuidanceBlock.test.tsx
        status: existing
        disposition: preserve
        assertion: "the old line's exact words; no turn or tilt word without directions; no icon- or object- class"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/src/App.live.test.tsx
        status: existing
        disposition: preserve
        assertion: "the old line's words before and after a stale pointing"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/e2e/simulate.spec.ts
        status: existing
        disposition: preserve
        assertion: "guidance holds turn and tilt while live and drops them when stale"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/pushto_api.rs
        status: existing
        disposition: preserve
        assertion: "turn, tilt and the target's placement keep their values"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "turn_deg, tilt_deg and the target's placement keep their meaning and values (crates/finder/tests/pushto_api.rs:165, :185)"
      - "the old line's words, pinned by five assertions (spec.md Clarifications)"
      - "the page does no sky maths; both new numbers come from the server"
    checks:
      development:
        - "cargo test -p finder --locked --test pushto_separation"
        - "cargo test -p finder --locked --test pushto_api"
        - "npm test"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs: test and finder-page"
    writes:
      - DESIGN.md
      - GLOSSARY.md
      - crates/finder/src/pushto.rs
      - crates/finder/tests/pushto_separation.rs
      - crates/finder/web/dist
      - crates/finder/web/src/api.ts
      - crates/finder/web/src/components/GuidanceBlock.tsx
      - crates/finder/web/src/components/GuidancePush.test.tsx
    no_write_reason: ""
    shared_writes:
      - path: DESIGN.md
        other_spec: 022-altaz-guidance
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1 and adds its own section"
      - path: crates/finder/web/dist
        other_spec: 022-altaz-guidance
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1 and rebuilds the bundle"
      - path: crates/finder/src/pushto.rs
        other_spec: 022-altaz-guidance
        other_story: US3
        disposition: merge_order
        reason: "US3 depends_on_merged US1 and adds up_deg beside separation_deg"
      - path: crates/finder/web/src/api.ts
        other_spec: 022-altaz-guidance
        other_story: US3
        disposition: merge_order
        reason: "US3 depends_on_merged US1 and adds up_deg"
      - path: DESIGN.md
        other_spec: 020-live-chart-calm
        other_story: US2
        disposition: merge_order
        reason: "022 depends_on_landed 020, whose US2 last writes DESIGN.md"
      - path: crates/finder/web/dist
        other_spec: 020-live-chart-calm
        other_story: US2
        disposition: merge_order
        reason: "022 depends_on_landed 020, whose US2 last rebuilds the bundle"
      - path: crates/finder/web/src/api.ts
        other_spec: 020-live-chart-calm
        other_story: US2
        disposition: merge_order
        reason: "022 depends_on_landed 020, whose US2 last writes api.ts"
      - path: crates/finder/src/pushto.rs
        other_spec: 013-live-finder
        other_story: US6
        disposition: merge_order
        reason: "022 depends_on_landed 013, whose US6 last wrote pushto.rs"
      - path: crates/finder/web/src/components/GuidanceBlock.tsx
        other_spec: 016-mcp
        other_story: US7
        disposition: merge_order
        reason: "022 depends_on_landed 016, whose US7 last wrote GuidanceBlock.tsx"
    open_questions: []
  US2:
    tests:
      - path: crates/finder/web/src/components/ChartAim.test.tsx
        status: new
        disposition: add
        assertion: "one data-aim group at the aim pixel with title, two circles and an untransformed label anchored toward the middle, after the furniture; absent when not live or with no aim, present with both; five chips and the furniture unchanged; present on an unsolved live frame"
        decision: "spec.md:US2-S1..S4"
      - path: crates/finder/web/src/components/Chart.test.tsx
        status: existing
        disposition: preserve
        assertion: "five chips with their counts; the counted mark classes"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/e2e/night-legibility.spec.ts
        status: existing
        disposition: preserve
        assertion: "one .furniture group whose night style is read"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/e2e/goto.spec.ts
        status: existing
        disposition: preserve
        assertion: "the running GoTo's night view has no off-red pixel, with the aim mark drawn"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "the centre reticle, compass, scale bar, target mark and legend are unchanged (spec.md FR-006)"
      - "uploads and samples get no aim mark"
      - "red-only night: the mark uses --color-on-photo and --color-photo only"
    checks:
      development:
        - "npm test"
        - "npm run build"
        - "CI=1 npm run e2e"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs: finder-page"
    writes:
      - DESIGN.md
      - crates/finder/web/dist
      - crates/finder/web/src/index.css
      - crates/finder/web/src/components/Chart.tsx
      - crates/finder/web/src/components/ChartAim.test.tsx
    no_write_reason: ""
    shared_writes:
      - path: DESIGN.md
        other_spec: 022-altaz-guidance
        other_story: US3
        disposition: merge_order
        reason: "US3 depends_on_merged US2 and adds its section after the aim mark's"
      - path: crates/finder/web/dist
        other_spec: 022-altaz-guidance
        other_story: US3
        disposition: merge_order
        reason: "US3 depends_on_merged US2 and rebuilds the bundle"
      - path: crates/finder/web/src/index.css
        other_spec: 022-altaz-guidance
        other_story: US3
        disposition: merge_order
        reason: "US3 depends_on_merged US2 and adds its rules after the aim mark's"
      - path: crates/finder/web/src/components/Chart.tsx
        other_spec: 022-altaz-guidance
        other_story: US3
        disposition: merge_order
        reason: "US3 depends_on_merged US2 and draws the up mark after the aim mark"
      - path: crates/finder/web/src/components/Chart.tsx
        other_spec: 020-live-chart-calm
        other_story: US2
        disposition: merge_order
        reason: "022 depends_on_landed 020, whose US2 last writes Chart.tsx"
      - path: crates/finder/web/src/index.css
        other_spec: 019-night-legibility
        other_story: US1
        disposition: merge_order
        reason: "022 depends_on_landed 019, whose US1 last wrote index.css"
    open_questions: []
  US3:
    tests:
      - path: crates/finder/tests/pointing_up.rs
        status: new
        disposition: add
        assertion: "up_deg within 0.3 deg of north less the parallactic angle at the centre and at an aim point; absent without a site and present once one is set; the live event's equals the session's"
        decision: "spec.md:US3-S1..S4"
      - path: crates/finder/web/src/useLiveUp.test.ts
        status: new
        disposition: add
        assertion: "the live pointing keeps up_deg, and has no such key when the event has none"
        decision: "spec.md:US3-S5"
      - path: crates/finder/web/src/components/ChartUp.test.tsx
        status: new
        disposition: add
        assertion: "the group's translate and rotate; labels outside it, untransformed and pointer-events-none, at bearings 78 and 168 to 0.01 deg; at the centre without an aim or off a live chart; absent without up_deg or on another pointing's chart; drawn at up_deg 0 with no target"
        decision: "spec.md:US3-S6..S9"
      - path: crates/finder/src/pushto.rs
        status: existing
        disposition: preserve
        assertion: "the unit tests that build session::Pointing field by field still compile"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/src/useLive.test.ts
        status: existing
        disposition: preserve
        assertion: "whole-object comparisons of the live report and the session"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/e2e/tracking.spec.ts
        status: existing
        disposition: preserve
        assertion: "a tracked night view has no off-red pixel, with the up and aim marks drawn"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "session::Pointing gains no field (crates/finder/src/pushto.rs:421 at c1ed295)"
      - "the compass's north and east stay as they are"
      - "red-only night: the mark uses --color-on-photo and --color-photo only"
    checks:
      development:
        - "cargo test -p finder --locked --test pointing_up"
        - "cargo test -p finder --locked --lib"
        - "npm test"
        - "CI=1 npm run e2e"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs: test and finder-page"
    writes:
      - DESIGN.md
      - crates/finder/README.md
      - crates/finder/src/pushto.rs
      - crates/finder/src/session.rs
      - crates/finder/src/live.rs
      - crates/finder/tests/pointing_up.rs
      - crates/finder/web/dist
      - crates/finder/web/src/api.ts
      - crates/finder/web/src/useLive.ts
      - crates/finder/web/src/useLiveUp.test.ts
      - crates/finder/web/src/index.css
      - crates/finder/web/src/components/Chart.tsx
      - crates/finder/web/src/components/ChartUp.test.tsx
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/session.rs
        other_spec: 017-console-stop-confirm
        other_story: US2
        disposition: merge_order
        reason: "022 depends_on_landed 017, whose US2 last wrote session.rs"
      - path: crates/finder/src/live.rs
        other_spec: 015-tracking
        other_story: US7
        disposition: merge_order
        reason: "022 depends_on_landed 015, whose US7 last wrote live.rs"
      - path: crates/finder/README.md
        other_spec: 017-console-stop-confirm
        other_story: US4
        disposition: merge_order
        reason: "022 depends_on_landed 017, whose US4 last wrote the README"
      - path: crates/finder/web/src/useLive.ts
        other_spec: 013-live-finder
        other_story: US8
        disposition: merge_order
        reason: "022 depends_on_landed 013, whose US8 last wrote useLive.ts"
    open_questions: []
```
