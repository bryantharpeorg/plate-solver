# Implementation Plan: Preview Frames Save Nothing

**Branch**: `058-preview-frames-save-nothing` | **Date**: 2026-11-27 |
**Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/058-preview-frames-save-nothing/spec.md`

## Summary

A **preview frame** is one imaging frame taken as 031's single frame is, at the settings
in force and with its refusals and words, but never saved: when its last byte is in
hand the console makes its preview, as 031 makes a saved frame's, and a crop of its
centre at full resolution, keeps both in memory among the last eight, and ends the frame
`shown`. Nothing is written under the imaging folder, and the node is asked with `save`
false. `POST /api/imaging/preview` takes one, `GET /api/imaging/frames/{id}/crop.jpg`
answers its crop, MCP's `take_preview_frame` takes one and answers both pictures, and
the Imaging page's block `Preview frames` asks for the next as soon as the last is
shown, until Stop, leaving the page, hiding it, a refusal or a frame that did not end
shown.

One general rule removes every side effect the owner would otherwise approve one by
one: **a preview frame is a single imaging frame that the console does not save; every
other rule is a single frame's** (spec.md § Owner decisions). It holds no lease, so it
needs no holder's name and no time limit; it takes the settings in force, so nothing is
remembered or put back; it is aborted, refused and refuses others as any single frame,
so 031's, 032's and 033's words serve unchanged; and the loop is the page's, so leaving
the page ends it with no rule on the console.

Four stories: the preview frame; its crop; MCP, the README and the glossary; the page.
Each is at most 34.3 % of 65,536 B on the probe's re-cut with its fold and at most
46.3 % at the factory's 1.35 times (§ Plan traps 2).

## Technical Context

**Language/Version**: Rust, MSRV 1.96, stable in CI; TypeScript and React for the page,
as built today.

**Primary Dependencies**: none new. `Cargo.toml`, `Cargo.lock`, `package.json` and
`package-lock.json` do not change. The crop uses 031's `preview::rgb` and
`preview::jpeg` (the `image` crate's JPEG encoder, already a dependency).

**Storage**: none on disk. In memory, beside 031's eight previews, at most eight crops:
on the simulated imaging camera a crop's JPEG is a mean of 84,608 B at 200 ms and 30,705 B
at 1000 ms (run); on a field night's frame of the real camera 22,742 B (run).

**Testing**:
- `crates/finder/tests/console/imaging_preview_frames.rs` (new, US1): its own preview
  rig, US1-S1 to US1-S3 under `#[tokio::test(start_paused = true)]`.
- `crates/finder/tests/console/imaging_preview_crop.rs` (new, US2): US2-S1 under
  `#[test]` on worked frames; US2-S2 on its own crop rig under
  `#[tokio::test(start_paused = true)]`.
- `crates/finder/tests/mcp/imaging_preview_frames.rs` (new, US3): its own MCP rig,
  US3-S1 under `#[tokio::test(start_paused = true)]` and US3-S2 under `#[test]`.
- `crates/finder/web/src/components/PreviewBlock.test.tsx` (new, US4): US4-S1 to US4-S5
  under Vitest with real timers; `crates/finder/web/e2e/imaging-preview.spec.ts` (new,
  US4): US4-S6 under Playwright's simulate project.

**Target Platform**: the console beside the scope node, as the field kit is built, and
the page in a phone's browser. A preview frame crosses the imaging connection whole, as
any imaging frame: 32.5 MB at full resolution and 16 bits, a quarter of that binned
2 × 2 (`docs/research/2026-11-15-imaging-camera.md:386`), so on the real camera the
cadence is the camera's and the link's, not the console's (read, not run).

**Constraints**:
- 031's `take` keeps its behaviour for every frame taken to be kept: a saved frame's
  file, cards, preview, node copy and words do not change (FR-005).
- The lease, the focus aid, darks and flats, the stack, the imaging run and the finder
  alignment are used and not changed: their own checks of a frame in flight already
  refuse a start while a preview frame is in flight (FR-004).
- `crates/scope-node`, `crates/scope-link`, `proto/` and the simulator are not changed:
  the node is asked with `save` false, an existing field, and the purpose `preview`,
  which its rule of 1 to 16 lower-case letters accepts
  (`crates/scope-node/src/imaging/mod.rs:284`; run through the simulator's node, which
  serves the scope node's own `NodeService`).
- `RUSTFLAGS="-D warnings"`, `--all-features` clippy, and rustdoc with warnings denied.
- No test uses a camera, a mount, ASTAP or the network.

**Scale/Scope**: four stories; four changed Rust files under `crates/finder/src/imaging/`
and `crates/finder/src/mcp/tools.rs`; three new Rust test files; one new page component,
its test and one browser test; two changed page files; the README's section, rows and
paragraph; one glossary entry and one changed.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Detection, the plate solver and the imaging solver are untouched. The crop is 031's own colour and stretch over the centre's samples (FR-006); a preview frame is never solved (032's refusal, unchanged). |
| II. Never weaken a gate | No existing test, golden, fixture, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS` or `ergane.yaml` is edited. `playwright.config.ts` gains two list entries, an owner approval as 050's and 051's were. The only edits to existing test files are the `mod` lines of `crates/finder/tests/console/main.rs` and `crates/finder/tests/mcp/main.rs` (SC-002). Every existing test passed unchanged on the probe's tip (run: nextest 1,533 of 1,533, the 1,525 existing, the stories' 7 and the probe's own 1; Vitest 612 of 612; `CI=1 npm run e2e` 35 of 35). |
| III. Tests first, independent expectations | The oracles are the simulated imaging camera's sensor (1236 × 822 RGGB, `crates/simulator/src/imaging.rs:145`), the worked frames and their hand-placed cells, the folders' own listings, 031's, 032's and 033's words and the scenarios' words, and DESIGN.md's night ink. No test calls `crop` or `rgb` for an expected value or reads `CROP_SIDE`, `PREVIEW_SIDE`, `PREVIEWS_KEPT` or `POLL_MS`. Each story's tests fail before it (run, § What was run). Time comes from a paused tokio clock and the simulator's virtual clock. |
| IV. Read the critical paths twice | None is touched: no `Cargo.toml` or `Cargo.lock` change, no `unsafe`, nothing under `proto/`, `crates/math-core`, `crates/pattern-database`, `crates/database-generation`, `crates/scope-node` or `crates/scope-link`, no `build.rs` or `.cargo/`. |
| V. Proven open source first, with honest licensing | Looked for, in local sources only (no download): the repository's survey of 2026-11-15, the vendored `reference-solutions/`, the crates in the cargo registry's local copy, the console's own code and the field night's viewer, outside the repository. **The camera maker's video mode**: the maker's library has one beside its single exposures, which INDI and INDIGO use (`docs/research/2026-11-15-imaging-camera.md:42`), and one study saw its video stall on USB 3.0 (`docs/research/2026-11-15-imaging-camera.md:43`); the camera is reached through INDI's server (ADR 0002), whose properties in the survey offer no stream (`docs/research/2026-11-15-imaging-camera.md:74`, a format, and `docs/research/2026-11-15-imaging-camera.md:76`, where a frame goes), so a video mode would be a new node path and a proto change (IV): not adopted. **N.I.N.A.'s looped exposure and Ekos's 1 s loop** (`docs/research/2026-11-15-imaging-camera.md:233`) and **SharpCap's** focus measures for the Moon and planets (`docs/research/2026-11-15-imaging-camera.md:208`): desktop programs that own the camera, read for their method only; a loop of single exposures is the method kept here. **OpenLiveStacker**: GPL-3, an appliance that wants the camera (`docs/research/2026-11-15-imaging-camera.md:419`). **The `image` crate** (0.25.10, already a dependency): its `imageops::crop_imm` crops a decoded picture, so it would need the whole frame in full-resolution colour first, where a crop of the samples' rows before `rgb` keeps the mosaic's cells and costs an eighth of the preview's work (run); its JPEG encoder is reused through `preview::jpeg`. **`reference-solutions/`** (cedar-detect, cedar-solve): no frame viewer (searched for one). **The field night's viewer**, a script beside the field notes: one `POST /api/imaging/frame` at a time while the camera is idle, the console's preview shown; it is this design less the file. **The console's own `/api/live` stream** (`crates/finder/web/src/useLive.ts:22`), the finder camera's: a second kind of message on it, rejected in § Decisions. Nothing is adopted; the new code is the route, the frame's end without a file, the crop of rows, the tool and the block. |

## Project Structure

```text
crates/finder/
├── README.md                                  # US3: ## Preview frames, two rows, two edits, MCP paragraph
├── src/
│   ├── imaging/frames.rs                      # US1: keep, show; US2: crop in show, FrameView.crop
│   ├── imaging/mod.rs                         # US1: take_preview, take_as's keep, ended; US2: crops, crop()
│   ├── imaging/preview.rs                     # US2: CROP_SIDE, crop
│   ├── imaging/routes.rs                      # US1: POST /api/imaging/preview; US2: crop.jpg
│   └── mcp/tools.rs                           # US3: take_preview_frame
├── tests/
│   ├── console/imaging_preview_frames.rs      # US1, new
│   ├── console/imaging_preview_crop.rs        # US2, new
│   ├── console/main.rs                        # US1, US2: one mod line each
│   ├── mcp/imaging_preview_frames.rs          # US3, new
│   └── mcp/main.rs                            # US3: one mod line
└── web/
    ├── playwright.config.ts                   # US4: two list entries
    ├── e2e/imaging-preview.spec.ts            # US4, new
    ├── dist/                                  # US4: rebuilt
    └── src/
        ├── api.ts                             # US4: state shown, crop, postImagingPreview
        └── components/
            ├── ImagingPage.tsx                # US4: one import, one line
            ├── PreviewBlock.tsx               # US4, new
            └── PreviewBlock.test.tsx          # US4, new
GLOSSARY.md                                    # US3: Preview frame; Purpose label
```

## Interfaces

The code as it ran on `probe/058`'s re-cut (tip aacd977b, and its fold cd55ef08, a test
alone). Bodies not given here are the
builder's, to these words and the FRs.

### `crates/finder/src/imaging/frames.rs` (US1; US2 adds)

```rust
pub struct FrameView {
    // ...031's fields...
    /// `exposing`, `receiving`, `saved`, `shown` (a preview frame, 058),
    /// `failed` or `aborted`.
    pub state: &'static str,
    // US2: after `preview`
    /// `/api/imaging/frames/<id>/crop.jpg`, set when a preview frame is shown
    /// and its crop could be made (058).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crop: Option<String>,
}

pub(super) fn take(
    imaging: &Imaging,
    purpose: &str,
    keep: bool,                       // false: a preview frame
    held: impl FnOnce(&Held) -> Result<Option<String>, Refusal>,
) -> Result<FrameTask, Refusal>;     // FrameTask { save: keep && settings.save_at_node, keep, .. }
```

- `FrameTask` gains `keep: bool`; its `save`, the node's copy, is
  `keep && settings.save_at_node`.
- In `follow`, once the frame is whole and the stream dropped: when `!task.keep`,
  `show(task, samples).await` and the outcome `Saved` (the frame's state is already
  `shown`); otherwise 031's `save`, unchanged.
- `async fn show(task: &mut FrameTask, samples: Vec<u8>)`: on the blocking pool,
  `preview::jpeg(&preview::rgb(..))` and (US2) `preview::jpeg(&preview::crop(..))` of the
  same samples; then, under the state's lock once, each JPEG made pushed to
  `held.previews` (US2: and `held.crops`), each trimmed from the front to
  `preview::PREVIEWS_KEPT`, and the view's `preview` (US2: and `crop`) set to its route;
  then `state = "shown"` and `task.finish()`. Nothing is written.

### `crates/finder/src/imaging/mod.rs` (US1; US2 adds)

```rust
impl Imaging {
    /// Take one preview frame (058): `take`'s checks, refusals and words with
    /// the purpose `preview`, but the frame is shown and saved nowhere, at the
    /// console or at the node. It holds no lease, as a single frame holds none.
    pub async fn take_preview(&self) -> Result<ImagingView, Refusal> {
        self.take_as(None, "preview", false).await
    }
    // `take` and `Lease::take` call `take_as(.., true)`.
    async fn take_as(&self, grant: Option<u64>, purpose: &str, keep: bool)
        -> Result<ImagingView, Refusal>;
    // US2:
    /// The JPEG crop of the preview frame with this id, among the last
    /// `PREVIEWS_KEPT` shown, or none (058).
    pub fn crop(&self, id: u64) -> Option<Vec<u8>>;
}
```

- `Held` gains (US2) `crops: Vec<(u64, Vec<u8>)>`, empty by default; the doc of
  `previews` and of `Imaging::preview` says "saved or shown frames".
- `Imaging::ended`'s match of a frame that has ended gains `"shown"`:
  `"saved" | "shown" | "failed" | "aborted"`.
- The doc of `take` ("Every frame `take` starts is saved to a file.") stays true:
  `take` passes `keep` true.

### `crates/finder/src/imaging/preview.rs` (US2)

```rust
/// The crop's side, in picture pixels: the centre of a preview frame at full
/// resolution (058).
pub const CROP_SIDE: u32 = 400;

/// The centre of a frame at full resolution (058): `rgb` of the middle
/// `CROP_SIDE` × `CROP_SIDE` cells (2 × 2 samples of a mosaic, one sample of a
/// mono frame), or of as many as the frame holds, whose first column and row
/// are the frame's own centre less half the crop, rounded down to whole cells
/// so a mosaic keeps its pattern. A crop of at most 1280 cells a side is never
/// box-averaged, and it is stretched over its own samples.
pub fn crop(samples: &[u8], width: u32, height: u32, bits: u32, bayer: Option<&str>) -> Rgb {
    let cell = if mosaic(bayer).is_some() { 2 } else { 1 };
    let bytes = if bits == 8 { 1 } else { 2 };
    let w = (CROP_SIDE * cell).min(width / cell * cell);
    let h = (CROP_SIDE * cell).min(height / cell * cell);
    let x0 = (width - w) / 2 / cell * cell;
    let y0 = (height - h) / 2 / cell * cell;
    // rows y0..y0 + h, each from x0 for w samples, copied in order, then:
    rgb(&part, w, h, bits, bayer)
}
```

### `crates/finder/src/imaging/routes.rs` (US1; US2 adds)

```rust
.route("/api/imaging/preview", post(take_preview))               // after /api/imaging/frame
.route("/api/imaging/frames/{id}/crop.jpg", get(frame_crop))     // US2, after preview.jpg

/// `POST /api/imaging/preview` (058): no body; one preview frame, shown and
/// never saved, with `POST /api/imaging/frame`'s refusals and words.
async fn take_preview(State(state): State<AppState>, _write: SessionWrite)
    -> Result<Json<ImagingView>, ApiError>;
// no imaging: 409 "this console takes no imaging frames"; a refusal: its status and words

/// `GET /api/imaging/frames/{id}/crop.jpg` (058): the crop of one of the last
/// eight preview frames, as the preview route answers its picture.
async fn frame_crop(State(state): State<AppState>, Path(id): Path<u64>)
    -> Result<impl IntoResponse, ApiError>;
// 404 "the console no longer holds that crop"; image/jpeg; `Path<u64>` answers 400 for "abc"
```

### `crates/finder/src/mcp/tools.rs` (US3)

`take_preview_frame(&self, arguments: JsonObject) -> CallToolResult`, after
`take_imaging_frame`, with FR-009's description exactly and `output_schema =
object_schema()`. Any argument: the tool error `take_preview_frame takes no arguments`;
no imaging: `this console takes no imaging frames`; a refusal: its words; then
`imaging.ended(id).await`. `shown`: `CallToolResult::structured` of the imaging view with
that frame, and an image block of `image/jpeg` (base64) for `imaging.preview(id)` and
then `imaging.crop(id)`, each when kept; `aborted`: `the exposure was aborted`; failed:
its `error`.

### The README (`crates/finder/README.md`, US3)

After the finder camera focus aid's entries (`crates/finder/README.md:149`) and before
`### MCP under WSL2` (`crates/finder/README.md:159`):

```markdown
The Imaging page's preview frames have one tool:

- `take_preview_frame` takes no arguments, does what `POST /api/imaging/preview`
  does, waits until the frame is shown and answers the imaging view with the
  frame's colour JPEG preview and the crop of its centre as two image blocks.
  Nothing of it is saved. It is refused as `take_imaging_frame` is, with the
  same words, and an aborted or failed frame answers a tool error with its
  words.
```

The API table, before the preview's row (`crates/finder/README.md:298`):

```markdown
| `POST /api/imaging/preview` | No body. Take one preview frame, as `POST /api/imaging/frame` takes one and with its refusals and words, but save nothing of it, at the console or at the node: its `frame` has the purpose `preview` and ends `shown`, with its `preview` and `crop`. Same-origin writes only. |
| `GET /api/imaging/frames/{id}/crop.jpg` | The crop of the preview frame with this id, its centre at full resolution, `image/jpeg`, while it is among the last eight preview frames; 404 with `the console no longer holds that crop` otherwise, and 400 for an id that is not a number. |
```

The preview's row then begins "The colour JPEG preview of the saved or shown imaging
frame with this id, `image/jpeg`, while it is among the last eight given one;", the rest
as it is. In `## Imaging frames` (`crates/finder/README.md:775`), "kept for the last
eight saved frames" becomes "kept for the last eight frames given one (preview frames
among them)". Between `## Imaging frames`' last paragraph and `## Solving imaging frames`
(`crates/finder/README.md:801`):

```markdown
## Preview frames

A preview frame is an imaging frame taken only to be looked at: to focus the
imaging camera by eye on a bright star or a planet, or to frame a target. It is
taken at the settings in force, as `POST /api/imaging/frame` takes one, and is
refused with the same words, but nothing of it is saved: no file under the
imaging folder, no copy at the scope node whatever `save_at_node` says, and so
no solve. `POST /api/imaging/preview` takes one. Its `frame` in the view has
the purpose `preview` and ends `shown`, with its `preview` and its `crop`, or
`failed` or `aborted` as any frame does. Its id is the console's same count, so
the next saved frame's id comes after it.

The crop is the frame's centre at full resolution: the middle 400 × 400 cells,
a cell being a 2 × 2 cell of the mosaic or one sample of a mono frame (or as
many as the frame holds), its first column and row rounded down to a whole
cell, turned into colour and stretched as the preview is but over its own
samples and with no box average. `GET /api/imaging/frames/<id>/crop.jpg`
answers it as `image/jpeg` while the frame is among the last eight preview
frames, and 404 `the console no longer holds that crop` after; its preview is
kept among the last eight previews, as a saved frame's is.

On the Imaging page, **Preview frames** takes them one after another at the
settings above while the page is open and shown: **Start preview frames** asks
for the next as soon as the last is shown, and **Stop preview frames**, leaving
the page or hiding it (another tab, a phone's screen locked) ends it; a frame
in flight then ends by itself. The whole frame shows under **Imaging frame**,
and the crop in the block. A refusal or a failed frame stops it with its words.
Nothing holds the camera between two preview frames, so while one is in flight
a holder's start (the focus aid, darks or flats, the stack, an imaging run or
the finder alignment) is refused with `an exposure is in progress`: stop the
preview frames first. A shorter exposure, or 2 × 2 binning, gives a quicker
picture; the crop at 2 × 2 is of binned pixels.
```

### The glossary (`GLOSSARY.md`, US3)

Before **Sub** (`GLOSSARY.md:173`):

```markdown
**Preview frame**:
An imaging frame taken only to be looked at, while the observer focuses or frames by eye: the console makes its preview and a crop of its centre at full resolution, keeps them in memory, and saves nothing of it, at the console or at the scope node.
_Avoid_: live view, snapshot, test frame
```

**Purpose label** (`GLOSSARY.md:277`): "`sub`, `run` and `align`." becomes "`sub`, `run`,
`align` and `preview`."

### The page (`crates/finder/web/src/`, US4)

- `api.ts`: `ImagingFrame.state` gains `"shown"` after `"saved"`; `crop?: string` after
  `preview?`; `postImagingPreview(): Promise<ImagingView>`, a `POST` to
  `/api/imaging/preview` with no body, after `postImagingFrame`
  (`crates/finder/web/src/api.ts:774`).
- `components/PreviewBlock.tsx`: `export const POLL_MS = 250;` and
  `export function PreviewBlock()`, with no props. State: `on`, the last `shown` frame,
  `every` (seconds) and `alert`. While `on`, one effect runs the loop of FR-013 and
  FR-014: `postImagingPreview()`, then every `POLL_MS` `getImaging()` (a failed read is
  read again) until the view's frame is not that frame `exposing` or `receiving`; then
  another frame's id ends it with `another imaging frame was taken`, `aborted` with
  `the exposure was aborted`, any other state but `shown` with the frame's `error`, and
  a shown frame sets `every` from `performance.now()` between the last two and loops.
  A `visibilitychange` listener turns `on` off when `document.visibilityState` is
  `hidden`; the effect's cleanup ends the loop and removes the listener. A press of the
  button clears the alert, and a start clears `every`.
- The block's markup:

```tsx
<section aria-labelledby="imaging-preview-title" className="space-y-3">
  <h2 id="imaging-preview-title" className="section-title">Preview frames</h2>
  <div className="space-y-3 border-y border-rule py-4">
    <p className="quiet">{US4-S1's words}</p>
    <button type="button" className={on ? "button" : "button button-primary"}>
      {on ? "Stop preview frames" : "Start preview frames"}</button>
    {on && <p role="status">{FR-013's status}</p>}
    {alert && <p role="alert">{alert}</p>}
    {shown?.crop && (
      <div className="border border-ink bg-photo">
        <img src={shown.crop} alt={`Centre of preview frame ${shown.id} at full resolution`}
             className="photo block h-auto w-full" />
      </div>)}
  </div>
</section>
```

- `components/ImagingPage.tsx`: `import { PreviewBlock } from "./PreviewBlock";` among
  the block imports, and `<PreviewBlock />` right after the section `Imaging frame`'s
  closing tag (`crates/finder/web/src/components/ImagingPage.tsx:291`), inside the same
  fragment, so it shows while that section does.

## What was run and what was read

Logs are under the operator's probe folder, not in the repository; the rounds are listed
in order.

**Run, the first cut** (2026-11-27), on `probe/058`, cut from f6d4f5fd:

- **The commits.** 0837b62a (US1: the preview frame and its crop on the console),
  126820db (US2: MCP, the README, the glossary), 73708d65 (US3: the page, `dist`
  rebuilt) and f2bc0898 (the probe's own measure, not a story).
- **The gates** on f2bc0898: `lint` passing; nextest 1,531 of 1,532, the one failing the
  layout test over the probe's module name, which held digits (§ Plan traps 9); the
  browser test of the block alone passing.
- **The size.** Its US1 measured 34,545 B, 71.2 % at the factory's 1.35 times, over the
  60 % line; so the crop was cut into a story of its own.

**Run, the re-cut** (2026-11-27), on the same branch:

- **The commits.** d6e621a2 (the tree back to f6d4f5fd's, byte for byte), 94f7282e
  (US1), 91017cfc (US2), e42df433 (US3), a30b8ff2 (US4, `dist` rebuilt) and aacd977b (the
  probe's own measure of a real frame, not a story: a test that prints when it is given
  a file and passes otherwise), and the fold cd55ef08 (US1-S2's settings change refused
  while a preview frame is in flight).
- **The gates** on aacd977b, every one: `cargo fmt --check`, clippy with `-D warnings`,
  both `cargo doc` runs, nextest of the whole workspace (1,533 of 1,533), the doc tests,
  `npm ci --offline`, the typecheck, Prettier, Vitest (612 of 612 in 65 files), the
  build, the dist check (clean) and `CI=1 npm run e2e` (35 of 35, with
  `imaging-preview.spec.ts` and the specs after it, `imaging.spec.ts` among them).
- **The fold** on cd55ef08: `cargo fmt --check` and the console binary's `imaging_preview`
  tests, 9 of 9; the other gates were not run again on it.
- **Each story's tests fail before it.** US1's three on f6d4f5fd's source: no route, so
  each `POST` answers no JSON. US2's file on US1's source: the console binary does not
  build (no `preview::crop`). US3's two on US2's source: the tool is unknown, and the
  README has no paragraph. US4's Vitest file on US3's source: `./PreviewBlock` does not
  resolve. US4's browser test before US4: not run.
- **The cadence**, the page's loop run over HTTP against the release console with
  `--simulate` (a script reading every 250 ms, as the block does): 20 preview frames of
  200 ms a mean 1.8 times the exposure apart (1.26 to 2.67 times), each `shown`; 10 of
  1000 ms 1.27 times apart; 10 saved frames of 200 ms 1.54 times. Preview JPEGs a mean
  of 135,466 B at 200 ms and 66,540 B at 1000 ms; crops 84,608 B and 30,705 B.
- **What was written.** After 30 preview frames the console's state folder held its
  `session.json` alone; after 10 saved frames, 10 files and 20,361,600 B more under
  `imaging/frames/`.
- **A real frame.** One of a field night's subs of the imaging camera (32,477,760 B
  FITS, 4944 × 3284, RGGB, 16 bits) through `preview::rgb` and `preview::crop`: a
  1236 × 821 preview of 326,587 B and a 400 × 400 crop of 22,742 B; the crop's work
  0.124 of the preview's, and 031's save of the FITS file 0.314 of it.
- **By day and by night.** US4-S6 on the simulated console in a 412 by 915 viewport:
  the page no wider than the viewport both ways, the night screenshot with no pixel off
  night red, the crop drawn red at night.

**Read, not run:**

- the real camera's download over the scope link, a larger share of each frame than the
  simulator's (the field night's notes; the survey's sizes,
  `docs/research/2026-11-15-imaging-camera.md:386`);
- owner decision 1 (b), a live mode on the console: not built, its size an estimate;
- the INDI adapter's frame type for the purpose `preview`, `FRAME_LIGHT` as for any
  purpose but `dark` and `flat` (`crates/scope-node/src/imaging/indi/mod.rs:1524`);
- the refusals of darks and flats, the stack, the imaging run and the finder alignment
  while a preview frame is in flight (`crates/finder/src/imaging/calibration/service.rs:396`,
  `crates/finder/src/imaging/stack/service.rs:411`,
  `crates/finder/src/imaging/run/console.rs:372`,
  `crates/finder/src/imaging/align.rs:237`); the focus aid's is run (US1-S2);
- the calibration library's match of the latest saved frame
  (`crates/finder/src/imaging/calibration/library.rs:173`);
- 054's kept picture and 055's Nudge block on the Imaging page, in their drafts (§ Risks);
- a planet through the crop: the stretch is 031's, and the field night's viewer showed
  031's preview of the whole frame, through which the owner focused on Saturn (the field
  report).

## Read from the diff, not from a test

Each passes every scenario built the other way (the probe); the judge reads it in the
diff.

- FR-001's state check (409 with the state's words while the state is not `ready`) and
  the order of the checks beyond US1-S2's lease before frame in flight.
- FR-002's view of a shown frame beyond US1-S1's keys, and its preview pushed into the
  same eight as saved frames' (US2-S2 counts the crops' eight, not the previews').
- FR-007's crop left out of the view when it cannot be made (US2-S1 shows that no JPEG
  is made; no view holds such a frame).
- FR-008's route open to anyone while a lease is out.
- FR-009's tool errors for an aborted and a failed frame.
- FR-013's `receiving` state in the loop, and S measured on the page's own clock.
- FR-014's refused `POST` words other than US4-S2's, and the read retried after a
  failed `GET`.
- FR-015's classes, beyond US4-S6's night screenshot.
- The README's rows and section and the glossary's entries (FR-010, FR-011), beyond
  US3-S2's sentences and headings.

## Shapes that existing tests pin

- **031's frames.** `crates/finder/tests/console/imaging_frames.rs`,
  `imaging_preview.rs`, `imaging_lease.rs` and `imaging_abort_silent.rs` pass unchanged:
  `take` and `Lease::take` pass `keep` true, so a saved frame is 031's (run).
- **032's solve.** `crates/finder/tests/console/imaging_solve.rs` and
  `crates/finder/tests/mcp/imaging_solve.rs` pass unchanged (run).
- **033's aid.** `crates/finder/tests/console/imaging_focus_aid.rs` and the other
  `imaging_focus_*` files pass unchanged: the aid's frames are leased, kept and removed
  as before (run).
- **MCP's frame tool.** `crates/finder/tests/mcp/imaging.rs` passes unchanged:
  `take_imaging_frame` is not edited (run).
- **The README's MCP count.** `crates/finder/tests/console/imaging_run_docs.rs:139`
  asserts "sixteen tools of its own" (`crates/finder/README.md:88`): the new tool is a
  paragraph of its own and that sentence is not touched (run).
- **050's and 051's README checks.** `crates/finder/tests/mcp/imaging_align.rs:202` and
  `crates/finder/tests/mcp/finder_focus.rs:211` find their paragraphs before
  `### MCP under WSL2`: the new paragraph follows the finder camera focus aid's entries
  (run).
- **The layout test.** `crates/finder/tests/layout/main.rs:155`: a test binary's
  `main.rs` holds only `mod` lines whose names are lower-case letters and `_` (run: it
  refused the first cut's probe module).
- **The Imaging page.** `ImagingPage.test.tsx` and `e2e/imaging.spec.ts` pass unchanged
  with the block (run).
- **The simulated console's state.** `imaging-preview.spec.ts` sorts before
  `imaging.spec.ts` in the simulate project (`-` before `.`); it saves an exposure of
  200 ms, takes only preview frames, leaves no frame in flight and nothing looping in
  its `finally`, so the specs after it see the server as before (run: 35 of 35).
- **`playwright.config.ts`.** The two entries put `imaging-preview.spec.ts` in the
  simulate project only (run).

## Decisions and the alternatives rejected

The nine owner decisions are in spec.md § Owner decisions, none answered. Below them:

| Decision | Chosen | Rejected |
|---|---|---|
| What a preview frame is | 031's frame task with `keep` false | a task of its own (a second copy of the header, chunk and abort rules) |
| Its end | the state `shown` | `saved` with no `file` (every reader of `saved` expects a file: the solve, the calibration library, the run and the stack); no state (the page could not tell it from a frame in flight) |
| The crop's samples | the centre's rows of the samples, then `rgb` (an eighth of the preview's work, run) | the whole frame in colour at full resolution, 2472 × 1642 pixels on the imaging camera, then a crop of it (not run); the crop made at the route on request (the samples held for it) |
| The crops kept | a ring of their own, eight, beside the previews | in the previews' ring (two kinds under one id) |
| How the page sees a frame | read `GET /api/imaging` every 250 ms while its frame is in flight | the `/api/live` stream (a second kind of message on the finder camera's stream); a JPEG stream of `multipart/x-mixed-replace` (a loop on the console, decision 1 (b)) |
| The whole frame | the section `Imaging frame`'s own picture, read once a second | a second picture of the whole frame in the block (twice the bytes a frame to the phone) |
| The story cut | four, the largest US1 at 46.3 % at the overrun (§ Plan traps 2) | three, with the crop in US1 (71.2 % at the overrun) |

## User Story 1 - The preview frame

`take`'s `keep`, `show`, `take_preview`, the route and the state `shown`, with their test
file. No crop.

## User Story 2 - The crop

`preview::crop`, the crop in `show`, the view's `crop`, the crops' ring, `Imaging::crop`
and the route, with their test file.

## User Story 3 - MCP and the words

The tool, the README and the glossary, and the README's check.

## User Story 4 - The page

`PreviewBlock`, its line in `ImagingPage`, `api.ts`, the browser test, the two config
entries and the rebuilt `dist`.

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

1. **The earlier specs have landed:** every spec in `depends_on_landed`. Met: each is on
   `main` at f6d4f5fd.
2. **052 to 055 have landed, and this spec is re-anchored on them.** Not met. 054 writes
   `crates/finder/src/imaging/mod.rs` (`Imaging::set`), `api.ts`, `ImagingPage.tsx`
   (the kept picture of the section `Imaging frame`), the README, the glossary, `dist`
   and `crates/finder/tests/console/main.rs`; 055 writes `ImagingPage.tsx` (the Nudge
   block after the camera's sections), `mcp/tools.rs`, `crates/finder/tests/mcp/main.rs`,
   `playwright.config.ts`, `dist`, the README and the glossary. Re-read every file of
   § Project Structure and every test of § Shapes on their landed `main`, move every
   `path:line`, name their stories in the record's `shared_writes`, add them to
   `depends_on_landed`, and run the validator and preflight again.
3. **The owner decisions are answered and the owner approvals given** (spec.md § Owner
   decisions, § Owner approvals). Not met: none is answered.
4. **A cynical review** of this trio, folded. Not met.
5. **Probe builds.** A probe round builds the stories from this text alone, and its
   defects are folded. Not met: the probe here was built before the text.
6. **The page half has run once.** Met on the probe: `CI=1 npm run e2e` 35 of 35.
7. **No operator-staged files are needed.** Met: no story reads a file outside the
   repository; the probe's measure of a real frame is not a story.

## Outside the factory

Written, not run. The owner's, on the next field night, with the imaging camera at the
main telescope's focuser and the console's page on a phone.

1. **Before.** On the console, note the free space of its disk and the size of its
   imaging folder (with the node's own save folder, if `save_at_node` is on), and the
   clock time.
2. **Point.** GoTo a bright star, or put a planet in the imaging camera's frame by hand
   (055's nudge arrows), with Track on so that it stays there.
3. **Set.** On the Imaging page, in Settings: for a star an exposure of about a second,
   for a planet tens of milliseconds, the gain as for the night's subs, the whole sensor,
   binning 1 (2 for a quicker picture, the crop then of binned pixels); save.
4. **Start.** Under **Preview frames**, press **Start preview frames**. The status reads
   `Preview frame N · every S s · not saved`; S is the cadence. If the target is not in
   the crop, the centre of the frame, move it there (055's nudge arrows) and watch the
   whole frame under **Imaging frame**.
5. **Focus by eye.** Turn the focuser a small step and wait two frames: a star's donut
   shrinks to a point and a planet's edge sharpens. Keep turning while it sharpens; when
   it softens, come back in smaller steps. Lock the focuser and watch that the picture
   stays sharp. To change the exposure, press **Stop preview frames**, save, start again.
6. **Ten minutes.** Keep it running for ten minutes, then press **Stop preview frames**.
   Note the clock time, the free space and the imaging folder's size again: they should
   be as in step 1, with no new file under `frames/` and none at the node.
7. **One saved frame.** Press **Take imaging frame**: one new file of the frame's size
   under `frames/`, as before.
8. **A reading.** On stars, start the focus aid (033) for a reading in pixels; a planet
   gives none (it measures stars).
9. **Note** the cadence seen, whether the crop was enough to judge focus, the minutes
   previewed, the disk before and after, and any refusal seen.

## Plan traps

They bind every story.

1. **Gate evidence or nothing.** The judge credits only gate results and the story's
   diff. *Check:* every Then names a committed test the `test` gate runs.
2. **One evidence budget.** *Check:* each story's whole diff stays under 60 % of
   65,536 B (39,321 B), measured with
   `git diff <base> -- . ':(exclude)crates/finder/web/dist' | wc -c` from the previous
   story's tip. The probe's re-cut: US1 21,678 + 798 B of its fold = 22,476 B (34.3 %),
   US2 18,699 B (28.5 %), US3 20,459 B (31.2 %) and US4 20,244 B (30.9 %), `dist` 697 B
   more. At the 1.35 times a factory build has run over its probe on earlier specs: US1
   about 30,300 B (46.3 %),
   US2 about 25,200 B (38.5 %), US3 about 27,600 B (42.1 %) and US4 about 27,300 B
   (41.7 %): every story under the 60 % line and far under the 128 KiB refusal.
3. **A spec fixed mid-epic needs a relaunch.** *Check:* no builder edits
   `specs/058-preview-frames-save-nothing/` or `.specify/`.
4. **An earlier story's test file is an existing test.** *Check:* US2 to US4 write no
   file of an earlier story's tests; each Rust test file builds its own rig.
5. **No new support file, and the shared transport has no imaging.**
   `crate::support::live`'s connector serves the node's `into_server()`
   (`crates/finder/tests/support/live.rs:52`), which carries no imaging service, so a rig
   on it never leaves `no_service`. *Check:* nothing new under
   `crates/finder/tests/support/`; each rig builds its own `Transport::connector` over a
   fresh `tokio::io::duplex` serving `node.service().routes()` with
   `scope_link::memory::serve_routes_on`, and uses `crate::support::live`'s `DB`,
   `START`, `sim()`, `ConsoleClock` and `request` as they are.
6. **An oracle copied from the code proves nothing.** *Check:* the tests write the
   sizes, places, colours, routes and words as literals; none calls `crop` or `rgb` for
   an expected value or reads the module's constants.
7. **A negative assertion passes when nothing happens.** *Check:* US1-S1's empty
   listings stand beside the shown frame's preview and the saved frame's two files after
   them; US1-S2's empty listing beside the shown frame; US3-S1's missing `frames` folder
   beside the two images; US4-S1's and US4-S4's "no `POST`" beside the `POST`s before;
   US4-S3's "no image" beside its alert; the night screenshot's 0 beside the day one's
   count; US4-S6's unmoved id beside the frames counted before.
8. **Paused time, and an abort's moment.** *Check:* every Rust test of this spec that
   awaits runs under `start_paused = true` with every wait bounded (120 s of paused
   time); US1-S3 sends its abort 1 s into a 5 s exposure, as 031's abort scenario
   waits, since an abort sent at once can find the frame already shown.
9. **A test binary's module names are letters and `_`.** *Check:* each new `mod` line
   in `crates/finder/tests/console/main.rs` and `crates/finder/tests/mcp/main.rs` names a
   module of lower-case letters and `_` only, in its alphabetical place
   (`crates/finder/tests/layout/main.rs:155`).
10. **The lock is never held across an await.** *Check:* `show` takes the state's lock
    once, after the blocking step, and drops it before `finish`.
11. **The dist is rebuilt.** *Check:* US4 commits `crates/finder/web/dist` after
    `npm run build`.
12. **An unused item fails clippy.** *Check:* each rig's imports are each used; the
    crop rig has no solver and no node folder.
13. **rustdoc denies warnings.** *Check:* no doc comment holds an angle bracket outside
    backticks (`/api/imaging/frames/<id>/crop.jpg` is in backticks).
14. **Each worktree builds on its own.** *Check:* no story sets `CARGO_TARGET_DIR`.
15. **The README's MCP count and 050's and 051's README order are pinned.** *Check:*
    US3 leaves "sixteen tools of its own" as it is and puts its paragraph after the
    finder camera focus aid's entries (US3-S2).
16. **The browser test runs on a shared server.** *Check:* US4-S6's locators are held to
    the regions `Preview frames`, `Imaging frame`, `Settings` and `Camera`, but the top
    bar's night switch; it leaves the page and waits until no frame is in flight in its
    `finally`; its waits stay well under the 60 s test timeout
    (`crates/finder/web/playwright.config.ts:29`).
17. **Vitest runs the loop on real time.** *Check:* US4-S1 to US4-S4 wait for each
    outcome with a bound (3 s for a frame, 1.2 s for "no `POST`"), and `afterEach` puts
    `document.visibilityState` back to `visible`.
18. **Playwright's project flag takes `=`.** *Check:* the development command is
    `npx playwright test --project=simulate e2e/imaging-preview.spec.ts`.
19. **The glossary's terms.** *Check:* every message, label and button says "preview
    frame" or "imaging frame", never "live view", and "imaging camera", never "camera"
    alone in a README sentence that could mean the finder camera.

## Risks

| Risk | Mitigation |
|---|---|
| On the real camera the download sets the cadence: a full 16-bit frame is 32.5 MB on the imaging connection (`docs/research/2026-11-15-imaging-camera.md:386`). | Binning 2 × 2 or a region of the sensor in Settings, a quarter of the bytes or fewer; the README says so; § Outside the factory step 4 notes the cadence seen (read, not run). |
| A preview frame in flight refuses every holder's start, and a page left open and shown keeps the camera busy (owner decision 5). | The words are the holders' own `an exposure is in progress`; the README says to stop the preview frames first; hiding the page ends them. |
| At f6d4f5fd the section `Imaging frame` draws its picture only while the latest frame has a preview (`crates/finder/web/src/components/ImagingPage.tsx:271`), so between two preview frames the whole frame blinks out; the block's crop does not, as it keeps the last shown. | 054's kept picture lands first and keeps the last picture until the next (read in 054's draft); the re-anchor checks it (§ Prerequisites 2). |
| 055's Nudge block and this block are each added near the end of the camera's sections in `ImagingPage.tsx` (`crates/finder/web/src/components/ImagingPage.tsx:291`), a textual neighbour. | One line each; the re-anchor moves this spec's line after 055's lands, and US4-S5 holds the order. |
| Preview frames push saved frames' previews out of the eight. | A saved frame's file stays; its preview is 031's convenience (spec.md § Edge Cases). |
| The stretch is 031's: a planet in the crop is drawn near white with little inside it (read from the rule, `crates/finder/src/imaging/preview.rs:51`). | Focus by eye reads the edge; the owner focused on Saturn through 031's preview (the field report, read); not run on a planet. |
| A story grows past 60 % in the factory. | The largest measures 34.3 % on the probe and about 46.3 % at the worst overrun seen (§ Plan traps 2). |

## Refinement Record

Drafted on 2026-11-27 by the plate-solver operator seat's spec session against `main` at
f6d4f5fd, from a probe of that day on `probe/058` (a first cut of three stories, then a
re-cut of four from f6d4f5fd's tree, with every scenario's test and both gates passing
on its tip, each story's tests failing before it, the cadence and the folders measured
on the simulated console, and a field night's frame of the imaging camera through the
preview and the crop), and a reading of 031's frames, preview, lease and Imaging page,
032's solve, 033's aid, the stack, run, calibration and alignment refusals, the scope
node's purpose rule, the simulator's imaging camera, MCP's tools, the field night's
notes and viewer, 054's and 055's drafts, the constitution, the glossary, the ADRs and
the survey of 2026-11-15. Writes come from the task text. Shared writes name the spec
story that last wrote the same file on `main`; within the spec, each story's files
written by an earlier one are ordered by `depends_on_merged`; 054 and 055 are
re-anchored at the flip (§ Prerequisites 2).

None of the nine owner decisions of spec.md is answered; this record is written to
their recommendations, and approves nothing. The flip is the owner's. No open question
is left: each choice is an owner decision with a recommendation.

```yaml
version: 1
reviewed_revision: f6d4f5fd482ccc9d5b634ef1aa5124c112769583
stories:
  US1:
    tests:
      - path: crates/finder/tests/console/imaging_preview_frames.rs
        status: new
        disposition: add
        assertion: "the preview rig, ready, PUT /api/imaging/settings exposure_ms 200 save_at_node true 200: POST /api/imaging/preview 200 with frame id 1, purpose preview, state exposing; read until shown: width 1236, height 822, preview /api/imaging/frames/1/preview.jpg, no file, saved_at, node_file, node_save_error or error; GET the preview 200, a JPEG of 618 x 411; R and N list nothing; POST /api/imaging/frame, read until frame 2 saved: R lists exactly frames/2026-01-16/000002.fits and N exactly the frame's node_file"
        decision: "spec.md:US1-S1"
      - path: crates/finder/tests/console/imaging_preview_frames.rs
        status: new
        disposition: add
        assertion: "the preview rig, ready, exposure_ms 2000: with Imaging::lease(the stack) out, POST /api/imaging/preview 409 the imaging camera is held by the stack; lease dropped: 200; then POST /api/imaging/preview and POST /api/imaging/frame each 409 an exposure is already in progress; PUT /api/imaging/settings exposure_ms 100 409 an exposure is in progress; POST /api/imaging/focus {} 409 an exposure is in progress, Imaging::holder() None; a foreign write (Host localhost:8080, Origin http://elsewhere.example) of POST /api/imaging/preview 403; read until shown: POST /api/imaging/solve 409 the latest imaging frame was not saved; POST /api/imaging/preview to AppState::shared alone 409 this console takes no imaging frames; R lists nothing"
        decision: "spec.md:US1-S2"
      - path: crates/finder/tests/console/imaging_preview_frames.rs
        status: new
        disposition: add
        assertion: "the preview rig, ready, exposure_ms 5000: POST /api/imaging/preview, 1 s later POST /api/imaging/abort 200, read until aborted: no preview; exposure_ms 100, POST /api/imaging/preview 200 with frame id 2, read until shown: preview /api/imaging/frames/2/preview.jpg; R lists nothing"
        decision: "spec.md:US1-S3"
      - path: crates/finder/tests/console/imaging_frames.rs
        status: existing
        disposition: preserve
        assertion: "031's frames: ids, files, cards, refusals and words of a saved frame, as 031 words them"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/console/imaging_lease.rs
        status: existing
        disposition: preserve
        assertion: "031's lease: one holder, its 409 to anyone else, and a single frame holding none"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/console/imaging_focus_aid.rs
        status: existing
        disposition: preserve
        assertion: "033's focus aid takes 031's lease, saves and removes its frames, and refuses a start while an unleased frame is in flight"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "Imaging::take and Lease::take save every frame they take, with the node's copy as save_at_node says (031 FR-008, FR-011, FR-030)"
      - "the lease, its words and each holder's check of a frame in flight are unchanged (031 FR-028, FR-029, FR-033), and a settings change in flight is refused as before (031 FR-004)"
      - "032's solve refuses a frame with no file with its words (032 FR-009)"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console -E 'test(imaging_preview_frames)'"
        - "cargo nextest run -p finder --locked --test console -E 'test(/imaging_(frames|lease|focus|preview|solve)/)'"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - crates/finder/src/imaging/frames.rs
      - crates/finder/src/imaging/mod.rs
      - crates/finder/src/imaging/routes.rs
      - crates/finder/tests/console/imaging_preview_frames.rs
      - crates/finder/tests/console/main.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/imaging/frames.rs
        other_spec: 035-live-stack
        other_story: US5
        disposition: merge_order
        reason: "058 depends_on_landed 035, whose US5 last changed frames.rs (a1c1b331)"
      - path: crates/finder/src/imaging/mod.rs
        other_spec: 050-tools-page-aligns-finder-camera
        other_story: US2
        disposition: merge_order
        reason: "058 depends_on_landed 050, whose US2 last changed imaging/mod.rs (500c7574)"
      - path: crates/finder/src/imaging/routes.rs
        other_spec: 050-tools-page-aligns-finder-camera
        other_story: US3
        disposition: merge_order
        reason: "058 depends_on_landed 050, whose US3 last changed imaging/routes.rs (6b70a4a2)"
      - path: crates/finder/tests/console/main.rs
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US2
        disposition: merge_order
        reason: "058 depends_on_landed 051, whose US2 last changed console/main.rs (f9706cc0)"
    open_questions: []
  US2:
    tests:
      - path: crates/finder/tests/console/imaging_preview_crop.rs
        status: new
        disposition: add
        assertion: "preview::crop of a worked 16-bit RGGB 1000 x 900 frame of samples 100 with cells of 1000 at (100, 50), (898, 848) and (98, 48) and the sample (500, 450) at 1000: 400 x 400, its only pixels not black (0, 0) and (399, 399) white 255 255 255 and (200, 200) red 255 0 0; of a worked 8-bit mono 500 x 300 frame of 10 with (50, 0), (449, 299) and (450, 299) at 200: 400 x 300, its only such pixels (0, 0) and (399, 299) white; of a worked 16-bit RGGB 1236 x 822 frame of 100 with a cell of 1000 at (218, 10): 400 x 400, its only such pixel (0, 0) white; of a worked 16-bit RGGB 64 x 1 frame: preview::jpeg gives none"
        decision: "spec.md:US2-S1"
      - path: crates/finder/tests/console/imaging_preview_crop.rs
        status: new
        disposition: add
        assertion: "the crop rig, ready, exposure_ms 100: a preview frame until shown has crop /api/imaging/frames/1/crop.jpg and the route 200 a JPEG of 400 x 400; a saved frame until saved has no crop and /api/imaging/frames/2/crop.jpg 404; after eight more preview frames, each until shown, frame 1's crop 404 with error the console no longer holds that crop, frames 3 and 10 200, /api/imaging/frames/abc/crop.jpg 400"
        decision: "spec.md:US2-S2"
      - path: crates/finder/tests/console/imaging_preview.rs
        status: existing
        disposition: preserve
        assertion: "031's preview of a saved frame: its size, colour, stretch, route, 404 and 400, and the last eight kept"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "preview::rgb and preview::jpeg and 031's preview of a saved frame are unchanged (031 FR-017, FR-018)"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console -E 'test(imaging_preview_crop)'"
        - "cargo nextest run -p finder --locked --test console -E 'test(/imaging_preview/)'"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - crates/finder/src/imaging/frames.rs
      - crates/finder/src/imaging/mod.rs
      - crates/finder/src/imaging/preview.rs
      - crates/finder/src/imaging/routes.rs
      - crates/finder/tests/console/imaging_preview_crop.rs
      - crates/finder/tests/console/main.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/imaging/preview.rs
        other_spec: 031-imaging-frames-on-console
        other_story: US6
        disposition: merge_order
        reason: "058 depends_on_landed 031, whose US6 last changed preview.rs (f7b58d53)"
    open_questions: []
  US3:
    tests:
      - path: crates/finder/tests/mcp/imaging_preview_frames.rs
        status: new
        disposition: add
        assertion: "take_preview_frame {} on the bare MCP console: tool error this console takes no imaging frames; the MCP rig, ready, set_imaging_camera exposure_ms 200: {x: 1} tool error take_preview_frame takes no arguments; {} no error, structuredContent.frame state shown, purpose preview, no file, exactly two image blocks of image/jpeg, the first 618 x 411 and the second 400 x 400; R holds no frames folder; with Imaging::lease(the stack) out, tool error the imaging camera is held by the stack; tools/list holds take_preview_frame with outputSchema {type: object}"
        decision: "spec.md:US3-S1"
      - path: crates/finder/tests/mcp/imaging_preview_frames.rs
        status: new
        disposition: add
        assertion: "crates/finder/README.md still holds The imaging camera has sixteen tools of its own; The Imaging page's preview frames have one tool: after The Tools page's focus aid for the finder camera has two tools: and before ### MCP under WSL2, with a list entry for take_preview_frame between them; ## Preview frames after ## Imaging frames and before ## Solving imaging frames, holding nothing of it is saved"
        decision: "spec.md:US3-S2"
      - path: crates/finder/tests/console/imaging_run_docs.rs
        status: existing
        disposition: preserve
        assertion: "the README's MCP paragraph holds sixteen tools of its own"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/mcp/finder_focus.rs
        status: existing
        disposition: preserve
        assertion: "051's finder camera focus aid tool paragraph comes before ### MCP under WSL2"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/mcp/imaging.rs
        status: existing
        disposition: preserve
        assertion: "take_imaging_frame's arguments, refusals, saved frame and its preview block, as 031 words them"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "take_imaging_frame and its description are unchanged; the README's sixteen tools sentence is unchanged"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test mcp -E 'test(imaging_preview_frames)'"
        - "cargo nextest run -p finder --locked --test console -E 'test(imaging_run_docs)'"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - GLOSSARY.md
      - crates/finder/README.md
      - crates/finder/src/mcp/tools.rs
      - crates/finder/tests/mcp/imaging_preview_frames.rs
      - crates/finder/tests/mcp/main.rs
    no_write_reason: ""
    shared_writes:
      - path: GLOSSARY.md
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US3
        disposition: merge_order
        reason: "058 depends_on_landed 051, whose US3 last changed GLOSSARY.md (9424a371)"
      - path: crates/finder/README.md
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US3
        disposition: merge_order
        reason: "058 depends_on_landed 051, whose US3 last changed the finder README (9424a371)"
      - path: crates/finder/src/mcp/tools.rs
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US3
        disposition: merge_order
        reason: "058 depends_on_landed 051, whose US3 last changed mcp/tools.rs (9424a371)"
      - path: crates/finder/tests/mcp/main.rs
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US3
        disposition: merge_order
        reason: "058 depends_on_landed 051, whose US3 last changed mcp/main.rs (9424a371)"
    open_questions: []
  US4:
    tests:
      - path: crates/finder/web/src/components/PreviewBlock.test.tsx
        status: new
        disposition: add
        assertion: "PreviewBlock over the stub console, later reads each shown frame N: the region Preview frames shows US4-S1's words; Start preview frames pressed: status Waiting for the first preview frame; image Centre of preview frame 1 at full resolution with src /api/imaging/frames/1/crop.jpg and status Preview frame 1 · not saved; status for frame 2 matches Preview frame 2 · every <s.s> s · not saved; every POST to /api/imaging/preview; Stop preview frames pressed, 1.2 s later no POST more, button Start preview frames, no status, no alert"
        decision: "spec.md:US4-S1"
      - path: crates/finder/web/src/components/PreviewBlock.test.tsx
        status: new
        disposition: add
        assertion: "PreviewBlock over a fetch answering 409 error the imaging camera is held by the stack: Start preview frames pressed: alert exactly Preview frames stopped: the imaging camera is held by the stack., button Start preview frames, fetch called once"
        decision: "spec.md:US4-S2"
      - path: crates/finder/web/src/components/PreviewBlock.test.tsx
        status: new
        disposition: add
        assertion: "PreviewBlock over the stub console, later reads frame N failed with error the imaging frame stalled, frame N aborted, and frame N + 1 exposing purpose frame: alerts exactly Preview frames stopped: the imaging frame stalled., Preview frames stopped: the exposure was aborted. and Preview frames stopped: another imaging frame was taken., each after one POST and with no image in the block"
        decision: "spec.md:US4-S3"
      - path: crates/finder/web/src/components/PreviewBlock.test.tsx
        status: new
        disposition: add
        assertion: "PreviewBlock over the stub console, later reads each shown frame N: started, a crop shown, document.visibilityState hidden and visibilitychange dispatched: button Start preview frames at once, no POST in 1.2 s; rendered again visible, started, a POST seen, unmounted: no POST in 1.2 s"
        decision: "spec.md:US4-S4"
      - path: crates/finder/web/src/components/PreviewBlock.test.tsx
        status: new
        disposition: add
        assertion: "ImagingPage (day), fetch answering a ready view of the simulated imaging camera with shown frame 7: the region Preview frames follows the region Imaging frame in the document; Imaging frame holds the image Imaging frame 7 with src /api/imaging/frames/7/preview.jpg; Preview frames holds the button Start preview frames"
        decision: "spec.md:US4-S5"
      - path: crates/finder/web/e2e/imaging-preview.spec.ts
        status: new
        disposition: add
        assertion: "the simulated console, 412 x 915 at device scale 2, day: /imaging, exposure 200 saved in Settings, Start preview frames in Preview frames: the crop Centre of preview frame <N> at full resolution within 15 s, natural size 400 x 400; the status's frame number up by 3 within 20 s, then Preview frame <N> · every <s.s> s · not saved; Imaging frame <N> in Imaging frame of natural width 618; GET /api/imaging frame purpose preview and no file; scrollWidth at most 412 by day and by night; the day full-page screenshot over 10,000 pixels off night red (any green or blue, or red above 226) and the night one 0 after the switch Night mode: red and black; Start preview frames after Stop preview frames; finally about:blank and no frame in flight, then the frame id unmoved 1.5 s later"
        decision: "spec.md:US4-S6"
      - path: crates/finder/web/src/components/ImagingPage.test.tsx
        status: existing
        disposition: preserve
        assertion: "031's Imaging page: states, settings, frame, picture and blocks, as before 058"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/e2e/imaging.spec.ts
        status: existing
        disposition: preserve
        assertion: "a frame taken from the Imaging page appears as a 618 x 411 picture, and the page is red at night but the held picture"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "every other block of the Imaging page is untouched; the page gains one import and one line"
      - "the simulated console's state after imaging-preview.spec.ts is as before it: no frame in flight, nothing looping"
    checks:
      development:
        - "npm test -- PreviewBlock"
        - "npx playwright test --project=simulate e2e/imaging-preview.spec.ts"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs and the night of plan.md § Outside the factory"
    writes:
      - crates/finder/web/dist
      - crates/finder/web/e2e/imaging-preview.spec.ts
      - crates/finder/web/playwright.config.ts
      - crates/finder/web/src/api.ts
      - crates/finder/web/src/components/ImagingPage.tsx
      - crates/finder/web/src/components/PreviewBlock.test.tsx
      - crates/finder/web/src/components/PreviewBlock.tsx
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/web/dist
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US4
        disposition: merge_order
        reason: "058 depends_on_landed 051, whose US4 last rebuilt dist (f6d4f5fd)"
      - path: crates/finder/web/playwright.config.ts
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US4
        disposition: merge_order
        reason: "058 depends_on_landed 051, whose US4 last changed playwright.config.ts (f6d4f5fd)"
      - path: crates/finder/web/src/api.ts
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US4
        disposition: merge_order
        reason: "058 depends_on_landed 051, whose US4 last changed api.ts (f6d4f5fd)"
      - path: crates/finder/web/src/components/ImagingPage.tsx
        other_spec: 042-imaging-run-page-and-mcp
        other_story: US9
        disposition: merge_order
        reason: "058 depends_on_landed 042, whose US9 last changed ImagingPage.tsx (5d8f5353)"
    open_questions: []
```
