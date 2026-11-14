# Implementation Plan: A Chart Turned to the Sky

**Branch**: `021-chart-orientation` | **Date**: 2026-11-13, refined 2026-11-14 | **Spec**: [spec.md](spec.md)

This plan records every design decision for the feature. A story that would change one
escalates rather than re-deciding it.

## Summary

US1 adds a session setting, `chart_rotation_deg`, edited in Setup and kept in
`session.json`; the MCP tools refuse it. US2 turns the live views by it: the live chart
draws its frame inside one SVG group rotated counter-clockwise, in a `viewBox` sized to
the rotated frame, with labels laid out where they are seen and turned back upright. The
live photo and the aim-point photo turn the same way, the aim tap maps a click back
through the rotation, and the "Just off the frame" arrows turn with the chart. Uploads,
samples and an unset or zero rotation draw exactly as today.

## Technical Context

**Language/Version**: Rust 2021 for `crates/finder`; TypeScript with React 19 for the
page.

**Primary Dependencies**: nothing new.

**Storage**: one new optional field in `session.json` (US1).

**Testing**:
- US1: `crates/finder/tests/session_rotation.rs` (new) for the session and the MCP
  refusals, and `crates/finder/web/src/components/SetupRotation.test.tsx` (new) for
  Setup and the page's session read.
- US2: `crates/finder/web/src/labelsRotation.test.ts` (new) for the layout,
  `crates/finder/web/src/components/ChartRotation.test.tsx` (new) for the chart, the
  photos, the tap and the arrows, and `crates/finder/web/src/App.rotation.test.tsx` (new)
  for the page.

**Constraints**:
- `web/dist` is committed and must be the build of its source.
- Taps, the aim point and every pixel sent to the server stay in frame coordinates.

## Constitution Check

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Display only; the solver is untouched. |
| II. Never weaken a gate | No existing test, golden, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS` or `ergane.yaml` is edited. |
| III. Tests first, independent expectations | The oracles are hand-computed quarter turns, trigonometry on the frame's size, and round trips through the tap mapping. |
| IV. Critical paths | No `unsafe`, no dependency change, nothing under `proto/`. |
| V. Standalone Rust, honest licensing | Unaffected. |

## Project Structure

```text
crates/finder/
├── README.md                                   # US1: the field
├── src/session.rs                              # US1: chart_rotation_deg
├── src/mcp/tools.rs                            # US1: both tools refuse it
├── tests/session_rotation.rs                   # US1: new
└── web/
    ├── src/api.ts                              # US1: the session's field
    ├── src/useSession.ts                       # US1: edit type, read keeps it
    ├── src/components/SetupPage.tsx            # US1: Chart rotation section
    ├── src/components/SetupRotation.test.tsx   # US1: new
    ├── src/rotation.ts                         # US2: new, the geometry
    ├── src/labels.ts                           # US2: layout in the rotated view
    ├── src/labelsRotation.test.ts              # US2: new
    ├── src/components/Chart.tsx                # US2: rotated group, viewBox, edge labels
    ├── src/components/LivePreview.tsx          # US2: rotated photo
    ├── src/components/AimTap.tsx               # US2: rotated photo, inverse tap
    ├── src/components/NextTargets.tsx          # US2: rotated arrows
    ├── src/components/ChartRotation.test.tsx   # US2: new
    ├── src/App.tsx                             # US2: page aspect, LivePreview, NextTargets
    ├── src/App.rotation.test.tsx               # US2: new
    ├── src/index.css                           # US2: upright labels
    └── dist/                                   # both: rebuilt
DESIGN.md                                       # US1: Setup; US2: the rotated views
GLOSSARY.md                                     # US1: Chart rotation
```

## Interfaces

Line numbers are at 0a33c19 unless marked. § Prerequisites says which to re-read.

### US1: the session (`crates/finder/src/session.rs`)

- `SessionEdit` (`crates/finder/src/session.rs:351`) gains, last:

  ```rust
  /// Chart rotation in degrees counter-clockwise, stored within [0, 360), or explicit
  /// null to clear it.
  #[serde(default, deserialize_with = "present_nullable")]
  pub chart_rotation_deg: Option<Option<f64>>,
  ```

  `present_nullable` (`crates/finder/src/session.rs:372`) already gives absent → `None`,
  `null` → `Some(None)` and a number → `Some(Some(v))`. A string, boolean or array fails
  to deserialize, and `api::session_edit_error` (`crates/finder/src/api.rs:812` to
  `crates/finder/src/api.rs:819`) answers it 400 with a JSON `error`, with no new code.
- `SessionView` (`crates/finder/src/session.rs:513`) gains
  `#[serde(skip_serializing_if = "Option::is_none")] pub chart_rotation_deg: Option<f64>`
  after `aim` (`crates/finder/src/session.rs:550`), and the view sets it from `Values`
  beside `aim: self.aim` (`crates/finder/src/session.rs:619`).
- `Values` (`crates/finder/src/session.rs:557`) gains a plain
  `chart_rotation_deg: Option<f64>` after `aim` (`crates/finder/src/session.rs:572`).
  `Values` derives only `Clone`, so it takes no serde attribute. `Values::default()`
  (`crates/finder/src/session.rs:578`) sets it to `None`.
- `Values::edited` checks it after `min_alt_deg`
  (`crates/finder/src/session.rs:1264`), refusing a non-finite number with
  `SessionError::Invalid("chart_rotation_deg must be a finite number of degrees")`, and
  stores it after the target (`crates/finder/src/session.rs:1286`):

  ```rust
  if let Some(rotation) = edit.chart_rotation_deg {
      // `rem_euclid` rounds a tiny negative angle up to 360 and keeps -0: both are 0.
      next.chart_rotation_deg = rotation.map(|deg| {
          let deg = deg.rem_euclid(360.0);
          if deg == 0.0 || deg >= 360.0 {
              0.0
          } else {
              deg
          }
      });
  }
  ```

  JSON cannot carry a non-finite number: serde_json refuses `1e400` while parsing, which
  answers 400. The check is for callers that build a `SessionEdit` in Rust.
- `StoredSession` (`crates/finder/src/session.rs:1398`) gains, after `aim`
  (`crates/finder/src/session.rs:1413`),
  `#[serde(default, skip_serializing_if = "Option::is_none")] chart_rotation_deg: Option<f64>`,
  so a file without it reads as unset and an unset value writes nothing.
- `read_state`'s `SessionEdit` (`crates/finder/src/session.rs:1437`) passes
  `chart_rotation_deg: Some(stored.chart_rotation_deg)`, so a stored value is checked and
  normalised exactly as a `PUT` is. `write_state`'s `StoredSession`
  (`crates/finder/src/session.rs:1487`) writes `values.chart_rotation_deg`.
- Every other `SessionEdit` literal ends in `..Default::default()` or
  `..SessionEdit::default()` and needs nothing (`crates/finder/src/session.rs:1529`,
  `crates/finder/src/simulate.rs:94`, `crates/finder/src/simulate.rs:118`).

### US1: the MCP tools (`crates/finder/src/mcp/tools.rs`)

Both exhaustive patterns name the field, and both tools refuse it:
- `set_target` (`crates/finder/src/mcp/tools.rs:364`): `chart_rotation_deg` joins the
  pattern, and `|| chart_rotation_deg.is_some()` joins the condition that returns
  "set_target changes only target".
- `set_session` (`crates/finder/src/mcp/tools.rs:400`): the same, joining the condition
  that returns "set_session changes only site, min_alt_deg and target".

### US1: the page's session

- `ObservingSession` (`crates/finder/web/src/api.ts:230`) gains, after `aim`
  (`crates/finder/web/src/api.ts:248`):

  ```ts
  /** How far the live views are turned, degrees counter-clockwise; omitted until set. */
  chart_rotation_deg?: number;
  ```

- `SessionEdit` (`crates/finder/web/src/useSession.ts:6`) gains
  `chart_rotation_deg?: number | null;` (`null` clears it), and `request()` keeps it
  after `aim` (`crates/finder/web/src/useSession.ts:38`):
  `...(body?.chart_rotation_deg != null && { chart_rotation_deg: body.chart_rotation_deg }),`.
- `SetupPage` (`crates/finder/web/src/components/SetupPage.tsx`) gets a section after
  Minimum altitude (`crates/finder/web/src/components/SetupPage.tsx:150` to
  `crates/finder/web/src/components/SetupPage.tsx:174`), built like it:
  - state `rotation` (a string, `""` when unset) and `rotationMessage`, set from
    `session.chart_rotation_deg` in an effect like the one for `min_alt_deg`
    (`crates/finder/web/src/components/SetupPage.tsx:34`);
  - `<section aria-labelledby="setup-rotation-title">` titled "Chart rotation", with
    `<Field label="Chart rotation" unit="°" … placeholder="0" />`, the hint as a
    `quiet` paragraph, the message as `role="status"` only once there is one, and a
    submit button "Save chart rotation";
  - on save: an empty field saves `{ chart_rotation_deg: null }` with "Chart rotation
    cleared."; a value that `Number()` makes finite saves it with "Chart rotation
    saved."; anything else says "Chart rotation must be a number of degrees." and saves
    nothing. Saving goes through the page's `save` helper
    (`crates/finder/web/src/components/SetupPage.tsx:36`).

### US1: the words

- `GLOSSARY.md`, after **Aim point**:

  > **Chart rotation**:
  > How far the page turns every live view of a frame, in degrees counter-clockwise, set
  > in Setup to match how the finder camera sits in its holder. A display setting: marks,
  > taps and the aim point stay in the frame's own pixels.
  > _Avoid_: turn (the base's motion in azimuth), roll (the solve's angle to north)

- `crates/finder/README.md`, after the aim point's paragraph
  (`crates/finder/README.md:331` to `crates/finder/README.md:336`):

  > `PUT /api/session` also takes `chart_rotation_deg`, in degrees counter-clockwise,
  > stored within [0, 360); `null` clears it. Reads omit it until set. The page turns its
  > live chart and photos by it. The MCP tools do not change it.

- `DESIGN.md`, a new section after § Live chart and preview
  (`DESIGN.md:332` to `DESIGN.md:333`):

  > ### Chart rotation — built
  > Setup's Chart rotation section, after Minimum altitude, holds one ruled degrees field
  > and a Save chart rotation action. An empty field clears the setting.

### US2: the geometry (`crates/finder/web/src/rotation.ts`, new)

```ts
// The live views' rotation: one angle from the session, applied alike to the live chart,
// the live photo and the aim-point photo. Display only: marks, taps and everything sent
// to the server stay in the frame's own pixels.

import type { ObservingSession } from "./api";

/** The rotation in degrees counter-clockwise, from the session; 0 when unset. */
export function rotationOf(session: ObservingSession | null | undefined): number {
  const deg = session?.chart_rotation_deg;
  if (deg === undefined || !Number.isFinite(deg)) return 0;
  return ((deg % 360) + 360) % 360;
}

/** Sine and cosine of `deg`, exact at quarter turns so a frame on its side keeps whole pixels. */
function sinCos(deg: number): [number, number] {
  const quarter = (((deg % 360) + 360) % 360) / 90;
  if (Number.isInteger(quarter)) return ([[0, 1], [1, 0], [0, -1], [-1, 0]] as const)[quarter] as [number, number];
  const a = (deg * Math.PI) / 180;
  return [Math.sin(a), Math.cos(a)];
}

/** The bounding box of a W×H frame rotated by `deg`. */
export function rotatedBox(W: number, H: number, deg: number): { width: number; height: number } {
  const [sin, cos] = sinCos(deg);
  return { width: W * Math.abs(cos) + H * Math.abs(sin), height: W * Math.abs(sin) + H * Math.abs(cos) };
}

/** The SVG transform that draws frame pixels rotated in the box, or undefined at 0. */
export function rotationTransform(W: number, H: number, deg: number): string | undefined {
  if (deg === 0) return undefined;
  const b = rotatedBox(W, H, deg);
  return `translate(${b.width / 2} ${b.height / 2}) rotate(${-deg}) translate(${-W / 2} ${-H / 2})`;
}

/** Where frame pixel (x, y) is drawn in the rotated box, in frame pixels from its top left. */
export function toView(W: number, H: number, deg: number, x: number, y: number): [number, number] {
  const b = rotatedBox(W, H, deg);
  const [sin, cos] = sinCos(deg);
  const [dx, dy] = [x - W / 2, y - H / 2];
  return [dx * cos + dy * sin + b.width / 2, -dx * sin + dy * cos + b.height / 2];
}

/** The frame pixel drawn at (x, y) of the rotated box: `toView`'s inverse. */
export function fromView(W: number, H: number, deg: number, x: number, y: number): [number, number] {
  const b = rotatedBox(W, H, deg);
  const [sin, cos] = sinCos(deg);
  const [dx, dy] = [x - b.width / 2, y - b.height / 2];
  return [dx * cos - dy * sin + W / 2, dx * sin + dy * cos + H / 2];
}

/** The frame pixel under CSS point (x, y) of a photo drawn rotated in `box`. */
export function boxToFrame(
  W: number,
  H: number,
  deg: number,
  box: { width: number; height: number },
  x: number,
  y: number,
): [number, number] {
  // Unrotated, this is the plain scaling the tap has always used, to the last bit.
  if (deg === 0) return [(x * W) / box.width, (y * H) / box.height];
  const b = rotatedBox(W, H, deg);
  return fromView(W, H, deg, (x * b.width) / box.width, (y * b.height) / box.height);
}
```

`toView` is SVG's `rotate(-deg)` about the frame's centre, moved to the box's centre, so it
agrees with `rotationTransform`. At 90° a 1296 × 972 frame's pixel (300, 200) is drawn at
(200, 996) of the 972 × 1296 box (US2-S5).

### US2: labels laid out where they are seen (`crates/finder/web/src/labels.ts`)

`LabelInput` (`crates/finder/web/src/labels.ts:30`) gains:

```ts
/** The live chart's rotation, degrees counter-clockwise; absent or 0 lays out in the frame's axes. */
rotation?: number;
```

`layoutLabels` (`crates/finder/web/src/labels.ts:143`) lays out in the rotated view, with
every mapping the identity when the rotation is 0 or absent, so today's layout is
unchanged to the bit:
- **Inputs into the view.** Each star, each detection and each object outline point
  (`outline`, `crates/finder/web/src/labels.ts:75`) goes through `toView`; an object's
  label is placed from `toView` of its centre. Each furniture box becomes the bounding box
  of its four corners through `toView`. `W` and `H` become `rotatedBox`'s width and
  height.
- **Reach.** An object's reach in view direction `(dx, dy)` is its reach
  (`crates/finder/web/src/labels.ts:61`) in the frame direction
  `(dx·cos − dy·sin, dx·sin + dy·cos)`.
- **The frame check.** `intoFrame` (`crates/finder/web/src/labels.ts:165`) takes a
  candidate's four corners back through `fromView`, works out the shortfall against the
  frame's 3u margin in frame pixels as it does today, `[fx, fy]`, and returns that shift
  turned into the view, `[fx·cos + fy·sin, −fx·sin + fy·cos]`. Moving the box by it moves
  its corners in the frame by exactly `[fx, fy]`.
- **Spots back in frame pixels.** For each placed label, with `c` its box's centre in the
  view and `f = fromView(c)`, the spot's `x` and `y` move by `f − c`, and each end of its
  leader goes through `fromView`. The spot's `box` stays as laid out, in the view. Drawn
  in the rotated group and turned back about its middle (the CSS rule below), the label's
  middle lands on `c`, upright.
- **The box's own anchor point**, which US2-S4 uses, is `x0`, `x1` or their middle for
  the anchors `start`, `end` and `middle`, at `y1 − 0.22 × size`, where `size` is
  `STAR_TEXT × u` for a star (`candidate`, `crates/finder/web/src/labels.ts:129` to
  `crates/finder/web/src/labels.ts:137`).

`sinCos` is exported from `rotation.ts` for this, or `labels.ts` takes `sin` and `cos`
from `toView` of unit vectors; either keeps the quarter turns exact.

### US2: the chart (`crates/finder/web/src/components/Chart.tsx`)

- `Chart` reads `const rotation = live ? rotationOf(session) : 0;` and
  `const b = rotatedBox(W, H, rotation);`.
- `u` uses `b.width` where it used `W` (`crates/finder/web/src/components/Chart.tsx:153`);
  `--chart-aspect` is `b.width / b.height`
  (`crates/finder/web/src/components/Chart.tsx:177`); the `viewBox` is
  `${-m} ${-m} ${b.width + 2 * m} ${b.height + 2 * m}`
  (`crates/finder/web/src/components/Chart.tsx:183`); `layoutLabels` gets `rotation`
  (`crates/finder/web/src/components/Chart.tsx:163`).
- **Unrotated** the SVG's children are today's, in today's order.
- **Rotated** the SVG has the extra class `chart-rotated` and the style
  `--label-rotation: ${rotation}deg`, and its children are the paper's edge
  (`crates/finder/web/src/components/Chart.tsx:194` to
  `crates/finder/web/src/components/Chart.tsx:202`), sized
  `b.width + 2 * m - 3 * u` by `b.height + 2 * m - 3 * u` and level, then one
  `<g transform={rotationTransform(W, H, rotation)}>` holding the `<defs>` and everything
  else, in today's order. Inside the group a turned edge would poke out of the `viewBox`
  (8.9 px a side at 30°).
- **Grid edge labels** (`crates/finder/web/src/components/Chart.tsx:309` to
  `crates/finder/web/src/components/Chart.tsx:326`): `Grid` gets `rotation`. Rotated,
  every edge label gets a `transform`, so the upright rule leaves it alone and it reads
  along its edge:

  ```ts
  const size = 11 * u;
  // The angle it is read at on screen, in [-180, 180): upside down past a quarter turn.
  const seen = ((((rotate - rotation) % 360) + 540) % 360) - 180;
  const middle = ty + (baseline === "hanging" ? 0.35 : -0.35) * size;
  const transform =
    rotation === 0
      ? rotate
        ? `rotate(${rotate} ${tx} ${ty})`
        : undefined
      : `rotate(${rotate} ${tx} ${ty})${Math.abs(seen) > 90 ? ` rotate(180 ${tx} ${middle})` : ""}`;
  ```

  At 90° only the left edge's labels flip; at 180° only the top and bottom edges'.

### US2: upright labels (`crates/finder/web/src/index.css`)

Beside `.ink-in` (`crates/finder/web/src/index.css:329`), inside `@layer components`:

```css
/* A rotated chart keeps its labels upright: each turns back about its own middle. */
.chart-rotated text:not([transform]) {
  transform-box: fill-box;
  transform-origin: center;
  rotate: var(--label-rotation);
}
```

It catches the star, object, compass, scale-bar, target, aim and up labels, none of which
has a `transform` of its own.

### US2: the photos and the arrows

- `LivePreview` (`crates/finder/web/src/components/LivePreview.tsx:2`) takes
  `{ seq, width?, height?, rotation? }` and exports:

  ```ts
  /** How a photo of a W×H frame sits rotated inside a box of the rotated frame's aspect. */
  export function rotatedPhoto(W: number, H: number, deg: number): { box: CSSProperties; photo: CSSProperties } {
    const b = rotatedBox(W, H, deg);
    return {
      box: { aspectRatio: b.width / b.height, position: "relative", overflow: "hidden" },
      photo: {
        position: "absolute",
        top: "50%",
        left: "50%",
        width: `${(W / b.width) * 100}%`,
        transform: `translate(-50%, -50%) rotate(${-deg}deg)`,
      },
    };
  }
  ```

  With a rotation other than 0 and both sizes known, the box `<div>` takes `box` and the
  `<img>` takes `photo`; otherwise neither has a `style`.
- `AimTap` (`crates/finder/web/src/components/AimTap.tsx`) reads
  `rotationOf(session)`; with a rotation other than 0 its button takes `rotatedPhoto`'s
  `box` and its `<img>` the `photo`. The click
  (`crates/finder/web/src/components/AimTap.tsx:57` to
  `crates/finder/web/src/components/AimTap.tsx:61`) keeps measuring the `<img>`'s
  bounding box and replaces the two scalings with
  `boxToFrame(preview.width, preview.height, rotation, rect, event.clientX - rect.left, event.clientY - rect.top)`.
- `DirectionArrow` (`crates/finder/web/src/components/NextTargets.tsx:163`) takes
  `rotation = 0` and draws `rotate(${degrees - rotation})`; its `aria-label` keeps
  `degrees`. `NextTargets` takes `rotation?: number` and passes it to the "Just off the
  frame" arrows (`crates/finder/web/src/components/NextTargets.tsx:40`). `TargetMark`'s
  arrow is inside the rotated group and passes none.
- `App` (`crates/finder/web/src/App.tsx`) computes
  `const rotation = showLive ? rotationOf(chartSession) : 0;` after `chartSession`
  (`crates/finder/web/src/App.tsx:141`); the page's aspect
  (`crates/finder/web/src/App.tsx:145` to `crates/finder/web/src/App.tsx:148`) comes from
  `rotatedBox(report.width, report.height, rotation)`, with `rotation` among the effect's
  dependencies; `LivePreview` (`crates/finder/web/src/App.tsx:241`) gets
  `width={live.report.width} height={live.report.height} rotation={rotation}`; and
  `NextTargets` (`crates/finder/web/src/App.tsx:259`) gets `rotation={rotation}`.

### US2: `DESIGN.md` § Chart rotation

Appended to the section US1 adds:

> The live chart, the live photo and the aim-point photo turn counter-clockwise by the
> session's chart rotation. The frame, the graticule, the compass and every mark turn
> together inside a level paper edge, and the compass stays true. Star and object labels
> are laid out where they are seen and read upright; edge labels read along their edges
> and never upside down. A tap on the turned aim-point photo maps back to the frame pixel
> under it. The "Just off the frame" arrows turn with the chart. Uploads and samples are
> drawn as captured.

## Shapes that existing tests pin

These are not changed by any story:
- **The unrotated chart.** Every existing `Chart`, `ChartLive`, `App` and e2e test renders
  without `chart_rotation_deg`, and FR-009 keeps their markup byte for byte. The probe
  compared the ORION2, VIRGO, ROOF, busy and live-with-target charts, `LivePreview` and
  the aim-point region against c1ed295 and found them identical.
- **The label layout.** `crates/finder/web/src/labels.test.ts` calls `layoutLabels`
  without `rotation`; every mapping is then the identity.
- **The aim tap.** `crates/finder/web/src/components/AimTap.test.tsx:52` mocks only the
  `<img>`'s box, and `crates/finder/web/src/components/AimTap.test.tsx:86` pins the exact
  body `{"frame_id":42,"x":700,"y":420}`. Unrotated, `boxToFrame` is today's formula.
- **The arrows' labels.** `crates/finder/web/src/components/NextTargets.test.tsx:42`
  finds them by `/from up on the frame/`, which they keep.
- **The session's fields.** `crates/finder/tests/session_cli.rs:144` to
  `crates/finder/tests/session_cli.rs:152` (a stored file of exactly three keys),
  `crates/finder/tests/session_api.rs:54` to `crates/finder/tests/session_api.rs:64` (a
  fresh read of exactly five keys), `crates/finder/tests/session_cli.rs:160` and
  `crates/finder/tests/mcp_session_tools.rs:232` (named fields only). An unset rotation
  is neither written nor read.
- **The MCP refusals.** `crates/finder/tests/mcp_session_tools.rs:185` and
  `crates/finder/tests/mcp_endpoint.rs:394` keep their messages.
- **Setup's single status.** `crates/finder/web/src/components/SetupPage.test.tsx:37`
  and the e2e checks that find one `status` stay true: the new section shows none until
  it is used.

## Decisions and the alternatives rejected

| Decision | Rejected | Why |
|---|---|---|
| Keep it in the session | `localStorage`, as the hotfix did | The setting describes the camera; the phone and the Surface should agree, and it should survive a browser change. |
| Any angle | Quarter turns only | The owner's camera sits at about 79°, and asked for degrees. |
| Rotate inside the SVG | A CSS transform on the whole figure | A CSS turn doesn't change the layout box, so the figure would overlap the page. |
| Wrap in a group only when rotated | Always wrap | Unrotated markup stays byte-identical (FR-009). Going between 0 and another angle redraws the marks once. |
| Paper edge outside the group | Inside it | Inside, its corners leave the `viewBox` at any angle but a quarter turn. |
| Lay labels out in the rotated view | Turn each label back in place | Turned back in place, at 90° two labels sat on their own star and three on other stars' rings. |
| Edge labels along their edges, flipped past a quarter turn | Upright like the rest | Upright, the top and bottom labels crossed the neatline by 3.5 to 7.4 px. |
| Live views only | Every chart | Only live frames are sure to come from the finder camera. |
| The MCP tools refuse it | Let `set_session` set it | The MCP tools' arguments are out of scope; refusing matches the other fields they don't change. |
| A stored value goes through `edited` | Copy it in | A hand-edited `session.json` gets the same check and normalisation as a `PUT`. |
| Positive is counter-clockwise | Clockwise | It matches the owner's words and the hotfix's chip. |
| Measure the tap against the `<img>` | The button | The existing tests mock only the `<img>`; the button's box is all zeros in jsdom, and it adds a 1 px border. |
| Two stories | One | One came to about 55 KiB in the probe before the label, edge and arrow fixes, against 64 KiB. |
| "Rotation" in the code | "Turn" | Turn is the base's motion in azimuth. |

## Verification (every story, before proposing)

- **The factory gates** (`ergane.yaml` at 0a33c19):
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

1. **The earlier specs have landed:** 012, 013, 016, 017, 018, 019, 020 and 022. All but
   020 and 022 are met at 0a33c19; `depends_on_landed` holds this spec until both land.
2. **`main` is still as read, once 020 and 022 have landed.** Every citation here is at
   0a33c19. 020 and 022 edit `crates/finder/web/src/components/Chart.tsx`,
   `crates/finder/web/src/App.tsx`, `crates/finder/web/src/api.ts`,
   `crates/finder/web/src/labels.ts`, `crates/finder/web/src/index.css`, `DESIGN.md`,
   `GLOSSARY.md`, `crates/finder/README.md` and `crates/finder/src/session.rs`; re-read
   the lines cited in those files after both land and correct them here. For
   `session.rs`, the probe's build of 022 moved them as follows; re-check them against
   the real merge:

   | Site | At 0a33c19 | After 022 (probe) |
   |---|---|---|
   | `SessionEdit` | 351 | 351 |
   | `present_nullable` | 372 | 372 |
   | `SessionView` | 513 | 517 |
   | `SessionView.aim` | 550 | 554 |
   | `Values` | 557 | 561 |
   | `Values.aim` | 572 | 576 |
   | `Values::default` | 578 | 582 |
   | the view's `aim: self.aim` | 619 | 635 |
   | `edited`'s `min_alt_deg` check | 1264 | 1280 |
   | `edited`'s target | 1286 | 1302 |
   | `#[serde(deny_unknown_fields)]` on `StoredSession` | 1397 | 1413 |
   | `StoredSession.aim` | 1413 | 1429 |
   | `read_state`'s `SessionEdit` | 1437 | 1453 |
   | `write_state`'s `StoredSession` | 1487 | 1503 |

3. **The owner approvals are given.** The horizon level was dropped on 2026-11-14;
   flipping gives the rest.
4. **The hotfix stays off `main`.** If it were committed first, US2 would also have to
   remove its chip and `finder.turn`.
5. **The refinement probe has run** (§ Refinement Record).

## Plan traps

1. **Gate evidence or nothing.** *Check:* every Then clause names a committed test that
   a gate runs.
2. **A named command is read literally.** *Check:* commands are copied from
   § Verification.
3. **One evidence budget.** *Check:* each story keeps its diff under 64 KiB, excluding
   `crates/finder/web/dist`. The probe's single story ran 13 KiB on the server and 41 KiB
   on the page, before the label, edge and arrow fixes.
4. **jsdom has no layout, no `@layer` and no `var()`.** *Check:* page tests assert
   attributes, inline styles, selector matches, the rule's text in `index.css` and mocked
   bounding boxes, never `getCTM`, `getBBox` or a computed `rotate`.
5. **A spec fixed mid-epic needs a relaunch.** *Check:* no builder edits
   `specs/021-chart-orientation/`.
6. **The bundle must be rebuilt.** *Check:* commit `crates/finder/web/dist`.
7. **Each worktree builds on its own.** *Check:* no story sets `CARGO_TARGET_DIR`.
8. **An exhaustive pattern stops the build.** *Check:* `crates/finder/src/mcp/tools.rs`
   names `chart_rotation_deg` in both patterns, and
   `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
   passes.
9. **`rem_euclid` is not quite [0, 360).** *Check:* US1-S1 sends `-1e-20` and `-0.0`.
10. **A square fixture hides a swapped box.** `ORION2` is 507 × 507. *Check:* the size
    scenarios use `WIDE`, 1296 × 972.
11. **Floating point at quarter turns.** *Check:* `sinCos` is exact there, and US2-S1
    compares the transform string exactly.
12. **An absence-only scenario is green before the change.** *Check:* US1-S2 sets 30
    first, US1-S4 asserts the new refusals' exact words, and US2-S8 is a regression
    guard beside scenarios that fail first.
13. **e2e state leaks between projects.** *Check:* any new e2e test that sets a rotation
    clears it with `null`; this spec adds none.
14. **Formatting is the repository's.** Prettier reflows several of the plan's snippets.
    *Check:* run `npm run format`, then `npm run format:check`.

## Risks

| Risk | Mitigation |
|---|---|
| A console from before this spec, given a `session.json` with the field, starts a fresh session. | Owner approval 1. Clear the setting before a downgrade; or delete the key from `session.json.bad` and rename it back. |
| CSS `rotate` on SVG text, `aspect-ratio` on a `<button>`, the clip path and a rotated photo's bounding box are not checked in a browser here. | Chromium on the Surface and Android supports them, and `CI=1 npm run e2e` runs Chromium; no e2e test sets a rotation, and the owner checks SC-004. |
| The fill box's middle differs from the laid-out box's by under 1 px. | The layout's text height is an estimate today too. |
| On a non-square frame at an angle that isn't a quarter turn, the chart draws smaller: at 30° a 1296 × 972 frame is drawn at about 0.73 of its unrotated size. | The owner sets about 79° or 90°; at 90° nothing shrinks but the aspect. |
| Going between 0 and another angle redraws every mark once. | It happens when the setting changes, not frame to frame. |
| The e2e tests that read Setup now see a second text field, "Chart rotation". | They find fields by exact name: `crates/finder/web/e2e/session.spec.ts:175` finds "Minimum altitude". |

## Refinement Record

Drafted on 2026-11-13 by the plate-solver operator against `main` at ce80f67, from the
owner's report at the lake site, a measurement of the belt's angle in the live frame, and
the hotfix used that night (quarter turns only, uncommitted). On 2026-11-14 the draft's
US1 was applied to throwaway worktrees of c1ed295, tests first, with no existing test
edited:

- **Server** (branch `probe/rust`, after 020 US1 and 022's server halves): `92397ed`.
  The three session scenarios failed for the right reason first (`unknown field
  chart_rotation_deg`) and then passed. The crate failed to compile until
  `crates/finder/src/mcp/tools.rs` named the field. The probe ran a console built before
  the change on a `session.json` holding the field and saw the fresh session and the
  `.bad` file the Owner approvals describe. `fmt`, `clippy -p finder --all-targets`,
  `cargo doc -p finder` and the lib, `session_rotation`, `session_cli`,
  `mcp_session_tools`, `api`, `session_api`, `mcp_endpoint` and `session_writes`
  binaries passed. The zero fix in § Interfaces was then built there too (`9ef1042`):
  before it US1-S1 failed with `-1e-20 -> {"chart_rotation_deg":360.0,…}`, after it all
  three session tests passed, and `clippy -p finder --all-targets` and `fmt` stayed
  clean.
- **Page** (branch `probe/021-page`, on a stand-in for 022's aim mark): `be4e49b`.
  `npm test` went from 258 to 280 passing with every existing test unchanged;
  typecheck, `format:check` and build passed, and `dist` was clean after the rebuild.
  It found the missing `useSession.ts` and `App.tsx`, the inverted tap scale, the inexact
  quarter turns, the paper edge outside the `viewBox`, the broken label layout, the
  upside-down edge labels and the unturned arrows; this trio carries each fix. The label
  layout in the rotated view, the edge-label flip and the arrows were not built.
- **Not run on the Surface:** `CI=1 npm run e2e` (no Playwright browsers), and the
  workspace-wide Cargo gates. The `test` gate runs both. The e2e tests nearest the change
  are `crates/finder/web/e2e/session.spec.ts:191` (Setup has no off-red pixel, now with
  one more section) and `crates/finder/web/e2e/night-legibility.spec.ts:35`, which reads
  the `viewBox`.

Writes come from the task text. Shared writes name the story in this spec that writes the
same file next, and the landed story that last wrote it (022's US3 for the files 022
touches, 020's US2 for `labels.ts` and `App.tsx`). Nothing in this record is an owner
approval: those are in spec.md, and flipping to `ready` gives them.

```yaml
version: 1
reviewed_revision: 0a33c1914547d327790fc0f1fc28b286729922e9
stories:
  US1:
    tests:
      - path: crates/finder/tests/session_rotation.rs
        status: new
        disposition: add
        assertion: "450, -90, 0, -1e-20 and -0.0 store 90, 270, 0, 0 and 0, each 0 positive; strings, booleans and arrays answer 400 and change nothing; a fresh read has no key; 90 survives a restart and null clears it from the read and the file; both MCP tools refuse it with their messages"
        decision: "spec.md:US1-S1..S4"
      - path: crates/finder/web/src/components/SetupRotation.test.tsx
        status: new
        disposition: add
        assertion: "the section sits after Minimum altitude and holds the session's value; saving sends the number, an empty field sends null, a word sends nothing, each with its message; the page's session read keeps the field and omits it when absent"
        decision: "spec.md:US1-S5..S7"
      - path: crates/finder/tests/session_cli.rs
        status: existing
        disposition: preserve
        assertion: "a stored file of exactly three keys; named fields survive a restart"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/session_api.rs
        status: existing
        disposition: preserve
        assertion: "a fresh read of exactly five keys"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/mcp_session_tools.rs
        status: existing
        disposition: preserve
        assertion: "set_session refuses the fields it doesn't change, with its message"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/src/components/SetupPage.test.tsx
        status: existing
        disposition: preserve
        assertion: "one status at a time on Setup"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "an unset rotation is neither written to session.json nor read (crates/finder/tests/session_cli.rs:144, crates/finder/tests/session_api.rs:54)"
      - "the MCP tools change only the fields they did, with the same refusals"
      - "every other session field keeps its validation and storage"
    checks:
      development:
        - "cargo test -p finder --locked --all-features --test session_rotation"
        - "cargo clippy -p finder --all-targets --all-features --locked -- -D warnings"
        - "npm test"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs: test and finder-page"
    writes:
      - DESIGN.md
      - GLOSSARY.md
      - crates/finder/README.md
      - crates/finder/src/session.rs
      - crates/finder/src/mcp/tools.rs
      - crates/finder/tests/session_rotation.rs
      - crates/finder/web/dist
      - crates/finder/web/src/api.ts
      - crates/finder/web/src/useSession.ts
      - crates/finder/web/src/components/SetupPage.tsx
      - crates/finder/web/src/components/SetupRotation.test.tsx
    no_write_reason: ""
    shared_writes:
      - path: DESIGN.md
        other_spec: 021-chart-orientation
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1 and appends to the section US1 adds"
      - path: crates/finder/web/dist
        other_spec: 021-chart-orientation
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1 and rebuilds the bundle"
      - path: DESIGN.md
        other_spec: 022-altaz-guidance
        other_story: US3
        disposition: merge_order
        reason: "021 depends_on_landed 022, whose US3 last writes DESIGN.md"
      - path: crates/finder/web/dist
        other_spec: 022-altaz-guidance
        other_story: US3
        disposition: merge_order
        reason: "021 depends_on_landed 022, whose US3 last rebuilds the bundle"
      - path: crates/finder/web/src/api.ts
        other_spec: 022-altaz-guidance
        other_story: US3
        disposition: merge_order
        reason: "021 depends_on_landed 022, whose US3 last writes api.ts"
      - path: crates/finder/src/session.rs
        other_spec: 022-altaz-guidance
        other_story: US3
        disposition: merge_order
        reason: "021 depends_on_landed 022, whose US3 last writes session.rs"
      - path: crates/finder/README.md
        other_spec: 022-altaz-guidance
        other_story: US3
        disposition: merge_order
        reason: "021 depends_on_landed 022, whose US3 last writes the README"
      - path: GLOSSARY.md
        other_spec: 022-altaz-guidance
        other_story: US1
        disposition: merge_order
        reason: "021 depends_on_landed 022, whose US1 last writes GLOSSARY.md"
      - path: crates/finder/src/mcp/tools.rs
        other_spec: 017-console-stop-confirm
        other_story: US1
        disposition: merge_order
        reason: "021 depends_on_landed 017, whose US1 last wrote mcp/tools.rs"
      - path: crates/finder/web/src/useSession.ts
        other_spec: 016-mcp
        other_story: US7
        disposition: merge_order
        reason: "021 depends_on_landed 016, whose US7 last wrote useSession.ts"
      - path: crates/finder/web/src/components/SetupPage.tsx
        other_spec: 016-mcp
        other_story: US7
        disposition: merge_order
        reason: "021 depends_on_landed 016, whose US7 last wrote SetupPage.tsx"
    open_questions: []
  US2:
    tests:
      - path: crates/finder/web/src/labelsRotation.test.ts
        status: new
        disposition: add
        assertion: "at 90 and 79 degrees no star label covers another label, a ring or a detection where they are drawn, each lies 3u inside the frame, and its middle lands on its box; at 0 the spots equal today's"
        decision: "spec.md:US2-S4"
      - path: crates/finder/web/src/components/ChartRotation.test.tsx
        status: new
        disposition: add
        assertion: "the 90 degree viewBox and exact transform with the paper outside the group; the 30 degree box and aspect; class, property, selector and CSS rule for upright labels, and edge labels flipped on the left at 90 and top and bottom at 180; the tap round trip at 0, 30 and 90; the live photo's box and turn; the arrows turned with their labels kept; unrotated, zero and upload charts identical to today's"
        decision: "spec.md:US2-S1..S3, US2-S5..S8"
      - path: crates/finder/web/src/App.rotation.test.tsx
        status: new
        disposition: add
        assertion: "a live page with rotation 90 draws the rotated group, sets the page aspect to 0.75 and boxes the live photo at 0.75"
        decision: "spec.md:US2-S9"
      - path: crates/finder/web/src/labels.test.ts
        status: existing
        disposition: preserve
        assertion: "today's label layout"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/src/components/AimTap.test.tsx
        status: existing
        disposition: preserve
        assertion: "an unrotated tap posts frame 42 at (700, 420)"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/src/components/NextTargets.test.tsx
        status: existing
        disposition: preserve
        assertion: "one arrow per nearby object, labelled from up on the frame"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/e2e/night-legibility.spec.ts
        status: existing
        disposition: preserve
        assertion: "the unrotated chart's viewBox and night styles"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "uploads, samples and an unset or zero rotation draw byte-identical markup (spec.md FR-009)"
      - "taps, the aim point and every pixel sent to the server stay in frame coordinates"
      - "the compass's north and east, the target mark and 022's aim and up marks keep their frame positions"
    checks:
      development:
        - "npm test"
        - "npm run typecheck"
        - "npm run build"
        - "CI=1 npm run e2e"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs: finder-page"
    writes:
      - DESIGN.md
      - crates/finder/web/dist
      - crates/finder/web/src/rotation.ts
      - crates/finder/web/src/labels.ts
      - crates/finder/web/src/labelsRotation.test.ts
      - crates/finder/web/src/index.css
      - crates/finder/web/src/App.tsx
      - crates/finder/web/src/App.rotation.test.tsx
      - crates/finder/web/src/components/Chart.tsx
      - crates/finder/web/src/components/LivePreview.tsx
      - crates/finder/web/src/components/AimTap.tsx
      - crates/finder/web/src/components/NextTargets.tsx
      - crates/finder/web/src/components/ChartRotation.test.tsx
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/web/src/components/Chart.tsx
        other_spec: 022-altaz-guidance
        other_story: US3
        disposition: merge_order
        reason: "021 depends_on_landed 022, whose US3 last writes Chart.tsx"
      - path: crates/finder/web/src/index.css
        other_spec: 022-altaz-guidance
        other_story: US3
        disposition: merge_order
        reason: "021 depends_on_landed 022, whose US3 last writes index.css"
      - path: crates/finder/web/src/labels.ts
        other_spec: 020-live-chart-calm
        other_story: US2
        disposition: merge_order
        reason: "021 depends_on_landed 020, whose US2 last writes labels.ts"
      - path: crates/finder/web/src/App.tsx
        other_spec: 020-live-chart-calm
        other_story: US2
        disposition: merge_order
        reason: "021 depends_on_landed 020, whose US2 last writes App.tsx"
      - path: crates/finder/web/src/components/LivePreview.tsx
        other_spec: 013-live-finder
        other_story: US8
        disposition: merge_order
        reason: "021 depends_on_landed 013, whose US8 last wrote LivePreview.tsx"
      - path: crates/finder/web/src/components/AimTap.tsx
        other_spec: 013-live-finder
        other_story: US9
        disposition: merge_order
        reason: "021 depends_on_landed 013, whose US9 last wrote AimTap.tsx"
      - path: crates/finder/web/src/components/NextTargets.tsx
        other_spec: 012-targets-and-push-to
        other_story: US7
        disposition: merge_order
        reason: "021 depends_on_landed 012, whose US7 last wrote NextTargets.tsx"
    open_questions: []
```
