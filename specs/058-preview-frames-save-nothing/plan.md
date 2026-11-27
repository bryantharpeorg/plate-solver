# Implementation Plan: Preview Frames Save Nothing

**Branch**: `058-preview-frames-save-nothing` | **Date**: 2026-11-27 |
**Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/058-preview-frames-save-nothing/spec.md`

## Summary

A **preview frame** is one imaging frame taken as 031's single frame is, at the settings
in force and with its refusals and words, but never saved: when its last byte is in
hand the console makes its preview, as 031 makes a saved frame's, and its **crop**, the
200 × 200 cells around the frame's brightest 32 × 32-sample box, one pixel a cell; it
keeps both in memory among the last eight and ends the frame `shown`. Nothing is written
under the imaging folder, and the node is asked with `save` false.
`POST /api/imaging/preview` takes one, `GET /api/imaging/frames/{id}/crop.jpg` answers
its crop, MCP's `take_preview_frame` takes one and answers both pictures, and the Imaging
page's block `Preview frames` asks for the next as soon as the last is shown. It pauses
while the page is hidden and asks for the next when it is shown again; Stop lets the
frame in flight end, the block reading `Stopping` meanwhile; leaving the page, a refusal,
a request that does not reach the console, or a frame that failed or was aborted end it.

One general rule removes every side effect the owner would otherwise approve one by
one: **a preview frame is a single imaging frame that the console does not save; every
other rule is a single frame's** (spec.md § Owner decisions). It holds no lease, so it
needs no holder's name and no time limit; it takes the settings in force, so nothing is
remembered or put back by the console; it is aborted, refused and refuses others as any
single frame, so 031's, 032's and 033's words serve unchanged; and the loop is the
page's, so leaving the page ends it with no rule on the console. The rule's one cost is
that a holder can take the camera between two preview frames, and a run that takes the
camera back while one is in flight ends (spec.md owner decision 10); the block names a
holder that overtook it, and the README tells the observer to stop and wait first.

Four stories: the preview frame; its crop; MCP, the README, the glossary and ADR 0002's
line; the page. Each is at most 42.6 % of 65,536 B on the fold's probe and at most
57.6 % at the factory's 1.35 times (§ Plan traps 2).

## Technical Context

**Language/Version**: Rust, MSRV 1.96, stable in CI; TypeScript and React for the page,
as built today.

**Primary Dependencies**: none new. `Cargo.toml`, `Cargo.lock`, `package.json` and
`package-lock.json` do not change. The crop uses 031's `preview::rgb` and
`preview::jpeg` (the `image` crate's JPEG encoder, already a dependency).

**Storage**: none on disk. In memory, beside 031's eight previews, at most eight crops of
200 × 200 pixels: on a field night's frames of the real camera 2,214 B (a half-second sub) and
4,753 B (a 5 s sub), and on the synthetic Saturns from 2,120 to 2,422 B (run).

**Testing**:
- `crates/finder/tests/console/imaging_preview_frames.rs` (new, US1): its own preview
  rig, US1-S1 to US1-S3 under `#[tokio::test(start_paused = true)]`.
- `crates/finder/tests/console/imaging_preview_crop.rs` (new, US2): US2-S1 under
  `#[test]` on worked frames; US2-S2 on its own crop rig under
  `#[tokio::test(start_paused = true)]`.
- `crates/finder/tests/mcp/imaging_preview_frames.rs` (new, US3): its own MCP rig,
  US3-S1 under `#[tokio::test(start_paused = true)]` and US3-S2 under `#[test]`.
- `crates/finder/web/src/components/PreviewBlock.test.tsx` (new, US4): US4-S1 to US4-S5
  and US4-S7 under Vitest with real timers, each with its own bound of 15 s;
  `crates/finder/web/e2e/imaging-preview.spec.ts` (new, US4): US4-S6 under Playwright's
  simulate project.

**Target Platform**: the console beside the scope node, as the field kit is built, and
the page in a phone's browser over plain HTTP. A preview frame crosses the imaging
connection whole, as any imaging frame: 32.5 MB for the whole sensor at 16 bits
(`docs/research/2026-11-15-imaging-camera.md:386`). On the field night the live stack's
whole frames came back to back a median 3.97 times their exposure apart at half a
second and 1.32 times at 5 s (spec.md § Clarifications, run from the saved subs), the
part beyond the exposure about the same in both: the camera's read-out, the link and the
console together, never split by any run. The console's share is not small: its preview
of a whole frame is about three times the work of the save a preview frame skips (run),
so on the camera a preview frame comes about as often as a saved one.

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
| II. Never weaken a gate | No existing test, golden, fixture, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS` or `ergane.yaml` is edited. `playwright.config.ts` gains two list entries, an owner approval as 050's and 051's were. The only edits to existing test files are the `mod` lines of `crates/finder/tests/console/main.rs` and `crates/finder/tests/mcp/main.rs` (SC-002). The README's count of the imaging camera's tools, which `crates/finder/tests/console/imaging_run_docs.rs:139` pins, keeps its words and gains a clause, so the test is not edited (spec.md owner approval 5). Every existing test passed unchanged at each story's tip and on the fold's final tip (run: nextest 1,528 to 1,533 of as many; Vitest 607 to 613 of as many; `CI=1 npm run e2e` 34 of 34 before US4 and 35 of 35 from it). |
| III. Tests first, independent expectations | The oracles are the simulated imaging camera's sensor (1236 × 822 RGGB, `crates/simulator/src/imaging.rs:145`), the worked frames and their hand-worked boxes and cells, the folders' own listings, 031's, 032's and 033's words and the scenarios' words, the README's and the ADR's text, and DESIGN.md's night ink. No test calls `crop` or `rgb` for an expected value or reads `CROP_SIDE`, `PREVIEW_SIDE`, `PREVIEWS_KEPT` or `POLL_MS`. Each story's new tests fail before it, and a mutation of each new or changed requirement is caught by a scenario (run, § What was run). Time comes from a paused tokio clock and the simulator's virtual clock in Rust, and real time within a bound in Vitest. |
| IV. Read the critical paths twice | None is touched: no `Cargo.toml` or `Cargo.lock` change, no `unsafe`, nothing under `proto/`, `crates/math-core`, `crates/pattern-database`, `crates/database-generation`, `crates/scope-node` or `crates/scope-link`, no `build.rs` or `.cargo/`. |
| V. Proven open source first, with honest licensing | Looked for, in local sources only (no download): the repository's survey of 2026-11-15 and the research notes behind it, the project's capture of INDI's CCD simulator (outside the repository), the vendored `reference-solutions/`, the crates in the cargo registry's local copy, the console's own code and the field night's viewer, outside the repository. **The camera maker's video mode**: the maker's library has one beside its single exposures, which INDI and INDIGO use for deep-sky frames (`docs/research/2026-11-15-imaging-camera.md:42`); the maker's planetary program uses video and its deep-sky one single frames (the research notes, read); one study saw its video stall on USB 3.0 (`docs/research/2026-11-15-imaging-camera.md:43`). **INDI's own stream**: the survey's property table lists the frame's format (`docs/research/2026-11-15-imaging-camera.md:74`) and where a frame goes (`docs/research/2026-11-15-imaging-camera.md:76`), not a stream, but INDI's CCD simulator, as the project captured it, offers `CCD_VIDEO_STREAM`, `STREAMING_EXPOSURE`, `CCD_STREAM_FRAME` (a subframe), `CCD_STREAM_ENCODER` with `RAW` and `MJPEG` and `LIMITS_PREVIEW_FPS`; whether `indi_asi_ccd` streams this camera is not known here. A stream would be a new node path and a `proto/` change (IV) and would amend ADR 0002's node that "does no pixel work": spec.md owner decision 1 (d), not adopted, not built. **N.I.N.A.'s looped exposure and Ekos's 1 s loop** (`docs/research/2026-11-15-imaging-camera.md:233`), the deep-sky focus tools, repeat single exposures, as this spec does; **SharpCap's** focus measures for the Moon and planets (`docs/research/2026-11-15-imaging-camera.md:208`): desktop programs that own the camera, read for their method only (owner decision 4 (d)). **OpenLiveStacker**: GPL-3, an appliance that wants the camera (`docs/research/2026-11-15-imaging-camera.md:419`). **The `image` crate** (0.25.10, already a dependency): its `imageops::crop_imm` crops a decoded picture, so it would need the whole frame in colour first, where a crop of the samples' rows before `rgb` keeps the mosaic's cells; its JPEG encoder is reused through `preview::jpeg`. **`reference-solutions/`** (cedar-detect, cedar-solve): no frame viewer (searched for one). The crop's box sum is a few lines over the samples the preview already reads. **The field night's viewer**, a script beside the field notes: one `POST /api/imaging/frame` at a time while the camera is idle, the console's preview shown; it is this design less the file. **The console's own `/api/live` stream** (`crates/finder/web/src/useLive.ts:22`), the finder camera's: a second kind of message on it, rejected in § Decisions. Nothing is adopted; the new code is the route, the frame's end without a file, the crop of rows around the brightest box, the tool and the block. |

## Project Structure

```text
crates/finder/
├── README.md                                  # US3: ## Preview frames, two rows, five edits, MCP paragraph
├── src/
│   ├── imaging/frames.rs                      # US1: keep, show; US2: crop in show, FrameView.crop
│   ├── imaging/mod.rs                         # US1: take_preview, take_as's keep, ended; US2: crops, crop()
│   ├── imaging/preview.rs                     # US2: CROP_SIDE, BOX_SIDE, brightest, crop
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
GLOSSARY.md                                    # US3: Preview frame, Crop; Purpose label
docs/adr/0002-imaging-cameras-through-indi-frames-relayed-console-stacks.md  # US3: one line
```

## Interfaces

The code as it ran on `probe/058`'s fold of review round 1 (§ What was run). Bodies not
given here are the builder's, to these words and the FRs.

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
/// The crop's side, in cells, one picture pixel a cell (058).
pub const CROP_SIDE: u32 = 200;
/// The side, in samples, of the boxes the crop's centre is found among (058).
const BOX_SIDE: u32 = 32;

/// The centre, in samples, of the frame's brightest box (058): of the whole
/// `BOX_SIDE` × `BOX_SIDE` boxes of samples from the frame's top-left, the one
/// whose samples sum the highest, the first in row order on a tie; the frame's
/// own centre when it holds no whole box. A box's sum is a box average, so one
/// hot sample cannot outweigh a star or a planet brighter than it.
fn brightest(samples: &[u8], width: u32, height: u32, bits: u32) -> (u32, u32);
// one pass over each band of BOX_SIDE rows, a u64 sum a box, reset each band

/// The crop of a preview frame (058): `rgb` of `CROP_SIDE` × `CROP_SIDE` cells
/// (2 × 2 samples of a mosaic, one sample of a mono frame), or of as many as
/// the frame holds, centred on the frame's brightest box and moved in to lie
/// within the frame's whole cells, its first column and row rounded down to a
/// whole cell so a mosaic keeps its pattern. It is never box-averaged, so one
/// picture pixel is one cell, and it is stretched over its own samples.
pub fn crop(samples: &[u8], width: u32, height: u32, bits: u32, bayer: Option<&str>) -> Rgb {
    let cell = if mosaic(bayer).is_some() { 2 } else { 1 };
    let bytes = if bits == 8 { 1 } else { 2 };
    let (whole_w, whole_h) = (width / cell * cell, height / cell * cell);
    let w = (CROP_SIDE * cell).min(whole_w);
    let h = (CROP_SIDE * cell).min(whole_h);
    let (x, y) = brightest(samples, width, height, bits);
    let x0 = x.saturating_sub(w / 2).min(whole_w - w) / cell * cell;
    let y0 = y.saturating_sub(h / 2).min(whole_h - h) / cell * cell;
    // rows y0..y0 + h, each from x0 for w samples, copied in order, then:
    rgb(&part, w, h, bits, bayer)
}
```

The module's doc gains "A preview frame also gets a crop around its brightest box, one
pixel a cell (058)." US2-S1's worked frames, by hand: the first's patch fills the box
of columns 672 to 703 and rows 288 to 319, whose sum (420,800) beats the hot sample's box
(167,835), so the centre is (688, 304) and the crop starts at (488, 104); the second's
patch fills the box of columns 0 to 31 and rows 256 to 287 (14,080 against the hot
sample's 10,475), centre (16, 272), and the crop is moved in to start at (0, 100), its
rows 288 to 299 in no box; the third's two patches tie and the first in row order wins,
centre (1008, 112), the crop moved down to start at (808, 0). In each crop fewer than one
pixel in a thousand differs from the background, so the stretch's span floors at 1 and
any cell above the background is white, and a red sample alone red.

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

The imaging camera's count (`crates/finder/README.md:88`) becomes:

```markdown
The imaging camera has sixteen tools of its own, which answer what the page's
Imaging page does (a seventeenth, for its preview frames, follows the Tools
page's tools below):
```

After the finder camera focus aid's entries (`crates/finder/README.md:149`) and before
`### MCP under WSL2` (`crates/finder/README.md:159`):

```markdown
The Imaging page's preview frames have one tool:

- `take_preview_frame` takes no arguments, does what `POST /api/imaging/preview`
  does, waits until the frame is shown and answers the imaging view with the
  frame's colour JPEG preview and its crop as two image blocks. Nothing of it
  is saved. It is refused as `take_imaging_frame` is, with the same words, and
  an aborted or failed frame answers a tool error with its words.
```

The API table, before the preview's row (`crates/finder/README.md:298`):

```markdown
| `POST /api/imaging/preview` | No body. Take one preview frame, as `POST /api/imaging/frame` takes one and with its refusals and words, but save nothing of it, at the console or at the node: its `frame` has the purpose `preview` and ends `shown`, with its `preview` and `crop`. Same-origin writes only. |
| `GET /api/imaging/frames/{id}/crop.jpg` | The crop of the preview frame with this id, 200 × 200 cells around its brightest part, one pixel a cell, `image/jpeg`, while it is among the last eight preview frames; 404 with `the console no longer holds that crop` otherwise, and 400 for an id that is not a number. |
```

The preview's row then begins "The colour JPEG preview of the saved or shown imaging
frame with this id, `image/jpeg`, while it is among the last eight given one;", the rest
as it is. In `## Imaging frames`: the states (`crates/finder/README.md:725`) read
"`exposing`, `receiving`, `saved`, `shown` (a preview frame's end, below), `failed` and
`aborted`"; the keys' sentence ends "and when saved, `saved_at` and `file`; a shown
preview frame has no `file` but a `crop` (§ Preview frames)."; the preview paragraph
(`crates/finder/README.md:775`) begins "Each saved frame, and each shown preview frame,
is also given a small colour preview, in memory only and kept for the last eight frames
given one, its route shown as the frame's `preview`. It is made from the frame's samples
as they were received:", and its "while the frame is among the last eight saved"
(`crates/finder/README.md:786`) reads "among the last eight given one"; and **One
holder** (`crates/finder/README.md:790`) ends "…, as before. The Imaging page's
**Preview frames** are single frames too, taken one after another: nothing holds the
camera between two of them, so a holder can start in the gap (§ Preview frames)."
Between that paragraph and `## Solving imaging frames` (`crates/finder/README.md:801`):

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

The crop is 200 × 200 cells of the frame around its brightest part, one picture
pixel a cell, a cell being a 2 × 2 cell of the mosaic or one sample of a mono
frame (or as many as the frame holds). The frame is cut from its top-left into
whole boxes of 32 × 32 samples, and the crop is centred on the box whose
samples sum the highest, the first in row order on a tie and the frame's own
centre when it holds no whole box, then moved in to lie within the frame, its
first column and row rounded down to a whole cell. A box's sum outweighs one
hot sample, so the crop follows a planet that drifts across the frame or the
bright star being focused on. It is turned into colour and stretched as the
preview is, but over its own samples and with no box average.
`GET /api/imaging/frames/<id>/crop.jpg` answers it as `image/jpeg` while the
frame is among the last eight preview frames, and 404 `the console no longer
holds that crop` after; its preview is kept among the last eight previews, as a
saved frame's is. However short the exposure, a frame comes no sooner than the
camera's read-out, its download and the console's preview allow.

On the Imaging page, **Preview frames** takes them one after another at the
settings above: **Start preview frames** asks for the next as soon as the last
is shown. The whole frame shows under **Imaging frame**, and the crop in the
block, drawn at the block's width. Hiding the page (another tab, or a phone
whose screen locks itself) pauses them, and showing it again asks for the next:
lengthen the phone's auto-lock while focusing. **Stop preview frames** lets the
frame in flight end by itself, and the block reads `Stopping: the last preview
frame is still in flight` until it has; leaving the page ends them too. A
refusal or a failed frame stops them with its words.

Nothing holds the camera between two preview frames. While one is in flight a
holder's start (the focus aid, darks or flats, the stack, an imaging run or the
finder alignment) is refused with `an exposure is in progress`, and a holder
that starts in the gap stops the block with the holder's name. Before anything
else takes the imaging camera, stop the preview frames and wait until the block
no longer reads `Stopping`. This holds for an imaging run's **Go anyway** after
a focus pause: the run takes the camera back at once and ends `camera_failed`
if a preview frame is still in flight. The settings in force are the night's:
put back the exposure and gain the night's subs use before starting a stack or
a run.
```

### The glossary (`GLOSSARY.md`, US3)

Before **Sub** (`GLOSSARY.md:173`):

```markdown
**Preview frame**:
An imaging frame taken only to be looked at, while the observer focuses or frames by eye: the console makes its preview and its crop, keeps them in memory, and saves nothing of it, at the console or at the scope node.
_Avoid_: live view, snapshot, test frame

**Crop** (of a preview frame):
The 200 × 200 cells of a preview frame around its brightest part, one picture pixel a 2 × 2 cell of the mosaic or one sample of a mono frame, for the eye to judge focus on.
_Avoid_: zoom, close-up, full-resolution preview, region of interest (that is the camera's)
```

**Purpose label** (`GLOSSARY.md:277`): "`sub`, `run` and `align`." becomes "`sub`, `run`,
`align` and `preview`." The **Raw frame**'s _Avoid_ of "full-resolution preview"
(`GLOSSARY.md:167`) stands: no word here says "full resolution".

### ADR 0002 (US3)

After its last line, a blank line and then the one line:

```markdown
Amended by spec 058: the console saves every frame taken to be kept; a preview frame is shown and saved nowhere.
```

### The page (`crates/finder/web/src/`, US4)

- `api.ts`: `ImagingFrame.state` gains `"shown"` after `"saved"`; `crop?: string` after
  `preview?`; `postImagingPreview(): Promise<ImagingView>`, a `POST` to
  `/api/imaging/preview` with no body, after `postImagingFrame`
  (`crates/finder/web/src/api.ts:774`).
- `components/PreviewBlock.tsx`: `export const POLL_MS = 250;` and
  `export function PreviewBlock()`, with no props. State: `mode` (`off`, `on` or
  `stopping`), `visible` (from `document.visibilityState`, kept by a
  `visibilitychange` listener of its own), the last `shown` frame, `every` (seconds),
  `alert` and `lost` (the last read failed). Two refs outlive a pause and a stop:
  `pending`, the id of the block's frame in flight, and `asking`, the `POST` while it is
  being answered. One effect, on `[mode, visible]`, runs while `mode` is not `off` and
  the page is visible: it waits for `asking`; with no `pending` it sets `mode` `off`
  when `stopping` and otherwise sends `postImagingPreview()` and keeps the frame's id in
  `pending`, a refusal ending the block with the route's words (an `ApiError`'s message)
  and a request that did not reach the console with `the console did not answer`; then
  every `POLL_MS` it reads `getImaging()`, a failed read setting `lost` and read again,
  until the view's frame is not that frame `exposing` or `receiving`; it clears
  `pending`; another frame's id (or none) loops to one more `POST`, `aborted` ends with
  `the exposure was aborted`, any other state but `shown` with the frame's `error`, and a
  shown frame sets `every` from `performance.now()` between the last two of this run of
  the effect, shows it and loops. The effect's cleanup only marks it stopped: a hidden
  page and `Stop` leave `pending` for the next run of the effect. `Start` clears the
  alert, the shown frame, `every` and `lost` and sets `mode` `on`; `Stop` sets it
  `stopping`.
- The block's markup:

```tsx
<section aria-labelledby="imaging-preview-title" className="space-y-3">
  <h2 id="imaging-preview-title" className="section-title">Preview frames</h2>
  <div className="space-y-3 border-y border-rule py-4">
    <p className="quiet">{US4-S1's words}</p>
    <button type="button" className={mode === "on" ? "button" : "button button-primary"}>
      {mode === "on" ? "Stop preview frames" : "Start preview frames"}</button>
    {mode !== "off" && <p role="status">{FR-013's status}</p>}
    {alert && <p role="alert">{alert}</p>}
    {shown?.crop && (
      <div className="border border-ink bg-photo">
        <img src={shown.crop} alt={`Crop of preview frame ${shown.id} around its brightest part`}
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

**Run, the fold of review round 1** (2026-11-27), on the same branch, on top of
cd55ef08:

- **The commits.** b3865b5b (the tree back to f6d4f5fd's, byte for byte), 2f1651fe (US1,
  the re-cut's with its fold, unchanged by the review), 76cdb8c7 and 213c38a2 (US2, the
  crop around the brightest box, and the fix of the factory clippy's lint on it, § Plan
  traps 22), d7054639 (US3), dbec07d4 (US4, `dist` rebuilt), 615549e1 (the probe's own
  measure of the two crops, not a story) and 37f8fe6b (US4's browser test reading the
  whole frame's picture once the block has stopped, § Plan traps 16).
- **The gates** at each story's tip and on the final tip, every one:
  `cargo fmt --check`, clippy with `-D warnings`, both `cargo doc` runs, nextest of the
  whole workspace, the doc tests, `npm ci --offline`, the typecheck, Prettier, Vitest,
  the build, the dist check and `CI=1 npm run e2e`. Every gate passed at 2f1651fe (US1),
  213c38a2 (US2), d7054639 (US3), dbec07d4 (US4) and 37f8fe6b (the final tip): nextest
  1,528, 1,530, 1,532, 1,532 and 1,533 of as many; Vitest 607 of 607 in 64 files at the
  first three and 613 of 613 in 65 at the last two; `CI=1 npm run e2e` 34 of 34 at the
  first three and 35 of 35, with `imaging-preview.spec.ts`, at the last two. At 615549e1
  every gate passed but the browser tests, 34 of 35: US4-S6's read of the section's
  picture while preview frames ran ran out its 5 s; run alone ten times as committed
  there it failed 4, and fifteen times as 37f8fe6b reads it, none (§ Plan traps 16).
  US2's first commit failed clippy (`chunks_exact_to_as_chunks`), fixed in the next.
- **Each story's new tests fail before it.** US1's tests are the re-cut's, unchanged
  (their failure on f6d4f5fd's source is the re-cut's, above). US2's file on US1's tip:
  the console binary does not build, `preview::crop` not found (4 errors). US3's two on
  US2's tip: the tool is not found (`-32602`), and the README lacks the count's clause.
  US4's Vitest file on US3's tip: `./PreviewBlock` does not resolve; against the draft's
  block, `api.ts` and page (a30b8ff2), US4-S1, US4-S3, US4-S4 and US4-S7 failed and
  US4-S2 and US4-S5 passed. US4's browser test on US3's tip, its spec and the config's
  two entries with no block: it ran out its 60 s, the page holding no
  `Start preview frames`.
- **Each new or changed requirement mutated once.** FR-006: the crop centred on the
  frame (the draft's rule), a box's brightest sample for its sum, a tie won by the last
  box and the draft's 400 cells each failed US2-S1 (the last also US2-S2). FR-009: the
  tool's crop block dropped, and its arguments taken, each failed US3-S1. FR-010: the
  count's seventeenth, the One holder sentence and the Stopping line each failed US3-S2;
  FR-011: ADR 0002's line dropped failed US3-S2. FR-012: the page without the block
  failed US4-S5. FR-013: a start keeping the shown frame and no Stopping line each
  failed US4-S1, and no `Waiting for the console` US4-S7. FR-014: hiding ending the loop
  (the draft's rule) failed US4-S4; an overtaken frame ending it with its own words,
  US4-S3; a `POST`'s failure in `fetch`'s words, US4-S7; a stop that reads nothing more,
  US4-S1. FR-015: the crop named for the centre failed US4-S1, US4-S4 and US4-S7.
  FR-016's check is the browser test, which failed before the block (above).
- **A run with a preview frame in flight**, on the probe's release console with
  `--simulate` (built from the re-cut's source; the fold changes none of US1's code): a
  run started on M42 paused for focus with the verdict `no_record`, its focus aid
  `running`; a preview frame then was refused
  `the imaging camera is held by the focus aid`; with the aid stopped, a 2000 ms preview
  frame `exposing` and `Go anyway` sent, the run ended `camera_failed`,
  `Ended: the imaging camera failed: an exposure is in progress.`; a second run,
  `Go anyway` sent once its preview frame was `shown`, went on to `solving`, and a
  preview frame asked for then was refused
  `the imaging camera is held by the imaging run`.
- **Pause and resume in a browser**, headless Chromium on the probe's release console at
  dbec07d4 with `--simulate`, exposures of 1000 ms, the page hidden as US4-S4 hides it:
  the frame in flight ended `shown` on the console while the page was hidden, and a
  saved frame asked for from outside then was answered 200, the camera free; the block
  sent no `POST` while hidden, and the page's own once-a-second read of the imaging view
  (031's `useImaging.ts`, which has no such pause) went on; shown again, the block found
  its frame overtaken, asked once more and showed the next crop 1.77 exposures after the
  page was shown. A second time, with the focus aid started while the page was hidden,
  the block's next `POST` was refused and it stopped with
  `Preview frames stopped: the imaging camera is held by the focus aid.` The console
  wrote the saved frame and the aid's, and no preview frame.
- **The two crops measured** (the probe's own test, through `preview::crop` and 031's
  `rgb` on the samples' rows, 3 runs each): on six synthetic Saturns (sharp, soft and
  dim at the centre; sharp and soft near the bottom edge; sharp near the top-left
  corner), (a) held Saturn on the three at the centre only and drew stretched noise
  elsewhere (a JPEG of 91,303 B), and (e) held it on all six, its centre 8 to 10 samples
  from Saturn's, a JPEG of 2,120 to 2,422 B; on two field subs of M42, (e) sat on two
  bright stars at the nebula's edge (half-second) and on the Trapezium (5 s), a JPEG of
  2,214 B and 4,753 B against (a)'s 22,742 B and 7,331 B, and the 5 s sub's first
  brightest cell, in a saturated star near its top edge, did not take the crop; the work
  of each with its JPEG, 0.114 to 0.121 of the preview's for (e) and 0.116 to 0.133 for
  (a).
- **The field night's cadence**, from the saved subs' `DATE-OBS` over consecutive ids:
  381 gaps of half-second subs, a median 3.97 times the exposure (3.82 to 4.28 from the
  10th to the 90th percentile), and 205 of 5 s subs, 1.32 times (1.31 to 1.34); the time
  beyond the exposure about the same for both.
- **The review's own runs**, kept: the console binary's imaging subset at cd55ef08, 84
  of 84; the draft's centre crop on three synthetic Saturns (the stretch draws the globe
  and rings near white, the limb and the ring gaps still sharp against soft); the page's
  loop against a focus aid start on the simulated console (the aid's start won in a gap
  and the loop stopped, overtaken); and the 63 `path:line` cites, all true.

**Read, not run:**

- the real camera's download over the scope link for a preview frame: its share of each
  frame is read only through the saved subs' cadence (the fold round, above), and a
  preview frame's cadence on the real camera is the field step's (§ Outside the
  factory);
- owner decisions 1 (b), a live mode on the console, and 1 (d), INDI's own stream at the
  node: not built, their sizes estimates; whether `indi_asi_ccd` streams this camera is
  not known here;
- owner decision 4 (d), a number from the crop in memory, and 10 (c), the run's re-hold
  waiting out a frame in flight: not built;
- the INDI adapter's frame type for the purpose `preview`, `FRAME_LIGHT` as for any
  purpose but `dark` and `flat` (`crates/scope-node/src/imaging/indi/mod.rs:1524`);
- the refusals of darks and flats, the stack and the finder alignment while a preview
  frame is in flight (`crates/finder/src/imaging/calibration/service.rs:396`,
  `crates/finder/src/imaging/stack/service.rs:411`,
  `crates/finder/src/imaging/align.rs:237`); the focus aid's is run (US1-S2), and the
  imaging run's hold (`crates/finder/src/imaging/run/console.rs:372`) is run in the fold
  round: its re-hold after `Go anyway` ended the run on a frame in flight with its
  words, and a preview frame was refused while the run held the camera;
- the calibration view's match of the last saved frame, which later frames leave intact
  (`crates/finder/src/imaging/calibration/service.rs:199`), while `FrameKey::from_view`
  reads a key only from a frame `saved` (`crates/finder/src/imaging/calibration/library.rs:173`);
- 054's kept picture, 055's Nudge block and 059's stack start on the Imaging page, in
  their drafts (§ Interplay with the specs drafted beside it);
- a real planet through the crop: the synthetic Saturns are run (the fold round,
  above), and the field night's viewer showed 031's preview of the whole frame, through
  which the owner focused on Saturn (the field report).

## Read from the diff, not from a test

Each passes every scenario built the other way (the probe); the judge reads it in the
diff.

- FR-001's state check (409 with the state's words while the state is not `ready`) and
  the order of the checks beyond US1-S2's lease before frame in flight.
- FR-002's view of a shown frame beyond US1-S1's keys, and its preview pushed into the
  same eight as saved frames' (US2-S2 counts the crops' eight, not the previews').
- FR-006's centre for a frame with no whole box (US2-S1's 64 × 1 frame makes no JPEG
  whatever its centre), and its tie rule beyond the third worked frame.
- FR-007's crop left out of the view when it cannot be made (US2-S1 shows that no JPEG
  is made; no view holds such a frame).
- FR-008's route open to anyone while a lease is out.
- FR-009's tool errors for an aborted and a failed frame.
- FR-013's `receiving` state in the loop, S measured on the page's own clock, and a
  press of `Start preview frames` while stopping.
- FR-014's refused `POST` words other than US4-S2's and US4-S3's, the frame shown while
  stopping, an overtaking frame whose next `POST` is answered (the loop goes on), and a
  frame in flight at the moment the page is hidden that is still in flight when it is
  shown again.
- FR-015's classes, beyond US4-S6's night screenshot.
- The README's rows, section and edits and the glossary's entries (FR-010, FR-011),
  beyond US3-S2's sentences and headings.

## Shapes that existing tests pin

- **031's frames.** `crates/finder/tests/console/imaging_frames.rs`,
  `imaging_preview.rs`, `imaging_lease.rs` and `imaging_abort_silent.rs` pass unchanged:
  `take` and `Lease::take` pass `keep` true, so a saved frame is 031's (run).
- **032's solve.** `crates/finder/tests/console/imaging_solve.rs` and
  `crates/finder/tests/mcp/imaging_solve.rs` pass unchanged (run). A preview frame is
  never solved (US1-S2), so spec 057's centring on the imaging camera needs saved
  frames; a tap on a preview frame's picture has no solve of its own.
- **033's aid.** `crates/finder/tests/console/imaging_focus_aid.rs` and the other
  `imaging_focus_*` files pass unchanged: the aid's frames are leased, kept and removed
  as before (run).
- **MCP's frame tool.** `crates/finder/tests/mcp/imaging.rs` passes unchanged:
  `take_imaging_frame` is not edited (run). Its last arm,
  `frame.error.expect("a failed frame carries its error")`
  (`crates/finder/src/mcp/tools.rs:783`), would panic if `Imaging::ended`'s answer for
  an older id, the last ended frame (`crates/finder/src/imaging/mod.rs:877`), were a
  shown preview frame: that needs a preview frame to start and end between a saved
  frame's end and the tool's waiter waking, so it is very unlikely, and FR-005 leaves
  the tool as it is; noted here, not guarded.
- **The README's MCP count.** `crates/finder/tests/console/imaging_run_docs.rs:139`
  asserts "sixteen tools of its own" (`crates/finder/README.md:88`): the sentence keeps
  those words and gains a clause naming the seventeenth, so the test passes unchanged
  and the sentence stays true (run).
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
  its `finally`, so the specs after it see the server as before (run: `CI=1 npm run e2e`, 35 of 35, at US4's tip and the final tip).
- **`playwright.config.ts`.** The two entries put `imaging-preview.spec.ts` in the
  simulate project only (run).

## Decisions and the alternatives rejected

The ten owner decisions are in spec.md § Owner decisions, none answered. Below them:

| Decision | Chosen | Rejected |
|---|---|---|
| What a preview frame is | 031's frame task with `keep` false | a task of its own (a second copy of the header, chunk and abort rules) |
| Its end | the state `shown` | `saved` with no `file` (every reader of `saved` expects a file: the solve, the calibration library, the run and the stack); no state (the page could not tell it from a frame in flight) |
| The crop's samples | the rows around the brightest box, then `rgb` (0.11 to 0.12 of the preview's work with the box sums and its JPEG, run) | the whole frame in colour at the camera's colour resolution, 2472 × 1642 pixels on the imaging camera, then a crop of it (not run); the crop made at the route on request (the samples held for it) |
| The crop's centre | the brightest 32 × 32-sample box, by its sum | the brightest single cell (one hot sample wins, US2-S1's frames, and saturated cells tie: on a 5 s field sub of M42 the first brightest cell lay in a saturated star near the top edge, far from the Trapezium's box, run); the preview's own pixels (their box varies with the frame's size, one cell on the simulated camera, and they are stretched to 8 bits, so a planet and a bright star tie at 255) |
| The crops kept | a ring of their own, eight, beside the previews | in the previews' ring (two kinds under one id) |
| How the page sees a frame | read `GET /api/imaging` every 250 ms while its frame is in flight | the `/api/live` stream (a second kind of message on the finder camera's stream); a JPEG stream of `multipart/x-mixed-replace` (a loop on the console, decision 1 (b)) |
| A hidden page | pause: no request, the frame in flight kept as the block's own, waited for when shown | end (decision 5 (a)); keep reading while hidden (a phone's browser may stop the page's timers anyway) |
| An overtaken frame | one more `POST`, whose refusal names the holder | stop with `another imaging frame was taken` (hides the holder); `another imaging frame was taken (<purpose>)` (a purpose label, not the holder's words) |
| The whole frame | the section `Imaging frame`'s own picture, read once a second | a second picture of the whole frame in the block (twice the bytes a frame to the phone) |
| The story cut | four, the largest US3 at 57.6 % at the overrun (§ Plan traps 2) | three, with the crop in US1 (71.2 % at the overrun on the first cut) |

## Interplay with the specs drafted beside it

- **054** keeps the section `Imaging frame`'s last picture. Until it lands that picture
  is drawn only while the latest frame has a preview
  (`crates/finder/web/src/components/ImagingPage.tsx:271`), so it blinks out on every
  preview frame; after it, 054's line for the last picture comes and goes each frame.
  The block's crop keeps the last shown frame either way.
- **055**'s Nudge block goes after the camera's sections, so the page reads Imaging
  frame, Preview frames, Nudge: the arrows sit under the crop, which suits a target the
  crop shows off centre (owner decision 3). Textual neighbours only.
- **057** centres a target on the imaging camera from solves; a preview frame is never
  solved, so 057 needs saved frames, and a tap on a preview frame's picture has no solve
  of its own (057 must use 055's frame up and right). The seat tells 057.
- **059**'s stack start is refused `an exposure is in progress` while a preview frame is
  in flight, and can win in the gap between two, when the block stops naming the stack.

## User Story 1 - The preview frame

`take`'s `keep`, `show`, `take_preview`, the route and the state `shown`, with their test
file. No crop.

## User Story 2 - The crop

`preview::crop` around the brightest box, the crop in `show`, the view's `crop`, the
crops' ring, `Imaging::crop` and the route, with their test file.

## User Story 3 - MCP and the words

The tool, the README's section, rows, paragraph and five edits, the glossary's two
entries, ADR 0002's line, and their check.

## User Story 4 - The page

`PreviewBlock` with its pause, stop and waits, its line in `ImagingPage`, `api.ts`, the
browser test, the two config entries and the rebuilt `dist`.

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
4. **A cynical review** of this trio, folded. Round 1 (0 blockers, 5 majors, 11 minors)
   folded on 2026-11-27, every finding in § What was run's fold round; whether a second
   round runs is the seat's call.
5. **Probe builds.** A probe round builds the stories from this text alone, and its
   defects are folded. Not met: the probe here was built beside the text.
6. **The page half has run once.** Met on the probe: `CI=1 npm run e2e` 35 of 35 at each
   story's tip from US4 on.
7. **No operator-staged files are needed.** Met: no story reads a file outside the
   repository; the probe's measure of a real frame is not a story.

## Outside the factory

Written, not run. The owner's, on the next field night, with the imaging camera at the
main telescope's focuser and the console's page on a phone.

1. **Before.** On the console, note the free space of its disk and the size of its
   imaging folder (with the node's own save folder, if `save_at_node` is on), and the
   clock time. On the phone, lengthen the auto-lock as far as it goes for the focusing:
   the page cannot keep the screen on, and a locked phone pauses the preview frames.
2. **Point.** GoTo a bright star, or put a planet in the imaging camera's frame by hand
   (055's nudge arrows), with Track on so that it stays there if it will.
3. **Set.** On the Imaging page, in Settings, note the night's exposure and gain, then
   set for a star an exposure of about a second, for a planet tens of milliseconds, the
   whole sensor and binning 1; save.
4. **Start.** Under **Preview frames**, press **Start preview frames**. The status reads
   `Preview frame N · every S s · not saved`; S is the cadence. The crop shows the
   frame's brightest part: if that is not the target (a brighter star, the Moon's edge),
   move the telescope until the target is the brightest thing in the frame, watching the
   whole frame under **Imaging frame**.
5. **Focus by eye.** Turn the focuser a small step and wait two frames: a star's donut
   shrinks to a point and a planet's edge sharpens. Keep turning while it sharpens; when
   it softens, come back in smaller steps. Lock the focuser and watch that the picture
   stays sharp. To change the exposure, press **Stop preview frames**, wait until the
   block no longer reads `Stopping`, save, start again.
6. **Lock and wake.** Let the phone lock itself once, wait, and wake it: the block goes
   on with the next frame, and no file was written meanwhile.
7. **Ten minutes.** Keep it running for ten minutes, then press **Stop preview frames**
   and wait until the block no longer reads `Stopping`. Note the clock time, the free
   space and the imaging folder's size again: they should be as in step 1, with no new
   file under `frames/` and none at the node.
8. **Put back.** In Settings, set the night's exposure and gain from step 3 back, and
   save.
9. **One saved frame.** Press **Take imaging frame**: one new file of the frame's size
   under `frames/`, as before.
10. **A reading.** On stars, start the focus aid (033) for a reading in pixels; a planet
    gives none (it measures stars).
11. **With a run.** If an imaging run pauses for focus, stop its focus aid, focus with
    preview frames as in step 5, press **Stop preview frames**, wait until the block no
    longer reads `Stopping`, then press **Go anyway**: the run goes on.
12. **Note** the cadence seen, whether the crop was enough to judge focus and whether it
    sat on the target, the minutes previewed, the disk before and after, and any refusal
    seen.

## Plan traps

They bind every story.

1. **Gate evidence or nothing.** The judge credits only gate results and the story's
   diff. *Check:* every Then names a committed test the `test` gate runs.
2. **One evidence budget.** *Check:* each story's whole diff stays under 60 % of
   65,536 B (39,321 B), measured with
   `git diff <base> -- . ':(exclude)crates/finder/web/dist' | wc -c` from the previous
   story's tip. The fold's stories: US1 21,866 B (33.4 %), US2 21,997 B (33.6 %), US3
   27,947 B (42.6 %) and US4 25,287 B (38.6 %), with its fix commit. At the 1.35 times a
   factory build has run over its probe on earlier specs: about 29,519 B (45.0 %),
   29,696 B (45.3 %), 37,728 B (57.6 %) and 34,137 B (52.1 %): every story under the
   60 % line and far under the 128 KiB refusal.
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
   them; US1-S2's empty listing beside the shown frame; US2-S1's black pixels beside the
   lit ones placed by hand, and the hot sample's absence beside the patch it lost to;
   US3-S1's missing `frames` folder beside the two images; US4-S1's and US4-S4's "no
   `POST`" and "no request" beside the requests before and after; US4-S1's "no image"
   after the second start beside the crop that follows; US4-S3's "no image" beside its
   alert; the night screenshot's 0 beside the day one's count; US4-S6's unmoved id
   beside the frames counted before.
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
    US3 keeps "sixteen tools of its own" word for word, adds only the clause of plan.md
    § The README after "Imaging page does", and puts its paragraph after the finder
    camera focus aid's entries (US3-S2).
16. **The browser test runs on a shared server.** *Check:* US4-S6's locators are held to
    the regions `Preview frames`, `Imaging frame`, `Settings` and `Camera`, but the top
    bar's night switch; it leaves the page and waits until no frame is in flight in its
    `finally`; its waits stay well under the 60 s test timeout
    (`crates/finder/web/playwright.config.ts:29`). It reads the section's picture only
    once the block shows no status: the section draws it only while the view's frame has
    a preview, so while preview frames run it comes and goes with each exposure (run:
    read while they ran, it ran out its 5 s at 615549e1 and in 4 of 10 runs alone; read
    once the block shows no status, 15 of 15).
17. **Vitest runs the loop on real time.** *Check:* US4-S1 to US4-S4 and US4-S7 wait
    for each outcome with a bound (3 s for a frame or a status, 1.2 s for "no `POST`"),
    each `it` carries its own timeout of 15 s (Vitest's default is 5 s, and US4-S1 waits
    up to about 13 s in all), and `afterEach` puts `document.visibilityState` back to
    `visible`.
18. **Playwright's project flag takes `=`.** *Check:* the development command is
    `npx playwright test --project=simulate e2e/imaging-preview.spec.ts`.
19. **The glossary's terms.** *Check:* every message, label and button says "preview
    frame" or "imaging frame", never "live view", and "imaging camera", never "camera"
    alone in a README sentence that could mean the finder camera; the crop is "around
    its brightest part" and "one pixel a cell", never "full resolution".
20. **The box sums cover whole boxes only.** *Check:* `brightest` sums only the whole
    32 × 32-sample boxes from the top-left, so the samples past the last whole box of a
    row or a column (US2-S1's second frame's rows 288 to 299) count for none, and a
    frame with no whole box is centred on itself.
21. **A pause keeps the block's frame.** *Check:* hiding the page or pressing Stop
    leaves the block's frame in flight, and its request while it is being answered, for
    the next run of the effect (US4-S1, US4-S4), so a shown page or a stopping block
    never asks for a frame while its own is in flight.
22. **The factory's clippy is newer than the MSRV.** *Check:* a fixed-size pair of
    bytes is read with `as_chunks::<2>()`, not `chunks_exact(2)`: the factory
    toolchain's clippy refuses the latter under `-D warnings`
    (`chunks_exact_to_as_chunks`; run, the fold's first US2 commit), and `as_chunks` is
    within the MSRV.

## Risks

| Risk | Mitigation |
|---|---|
| On the real camera the read-out, the download and the console's preview set the cadence: on the field night whole frames came about three half-second exposures beyond their exposure (spec.md § Clarifications, run), so at a planet's exposure a frame comes about as often as the night's half-second subs. | No speed advice (owner decision 2): binning shortens nothing on the cable, and a region of interest's gain is unmeasured while its price, a region left set and then stacked, is 054's subject; § Outside the factory notes the cadence seen. |
| A preview frame in flight refuses every holder's start, and a page left open and shown keeps the camera busy (owner decision 5). | The words are the holders' own `an exposure is in progress`; the README says to stop the preview frames and wait for the last; hiding the page pauses them and frees the camera. |
| An imaging run's `Go anyway` with a preview frame in flight ends the run `camera_failed` (run on the probe's console). | The README and the field steps say to stop and wait until the block no longer reads `Stopping` (owner decision 10 (b)); a run that waits it out is a later spec's (10 (c)). |
| A phone's auto-lock pauses the preview frames, and the page cannot keep the screen on over plain HTTP. | The block asks for the next frame when shown again (US4-S4); the README and § Outside the factory step 1 say to lengthen the auto-lock. |
| Something brighter than the target (the Moon's edge, a brighter star) takes the crop. | The whole frame under `Imaging frame` still shows the target; the observer frames the target alone (§ Outside the factory step 4). |
| At f6d4f5fd the section `Imaging frame` draws its picture only while the latest frame has a preview (`crates/finder/web/src/components/ImagingPage.tsx:271`), so between two preview frames the whole frame blinks out; the block's crop does not, as it keeps the last shown. | 054's kept picture lands first and keeps the last picture until the next (read in 054's draft); the re-anchor checks it (§ Prerequisites 2). |
| 055's Nudge block and this block are each added near the end of the camera's sections in `ImagingPage.tsx` (`crates/finder/web/src/components/ImagingPage.tsx:291`), a textual neighbour. | One line each; the re-anchor moves this spec's line after 055's lands, and US4-S5 holds the order. |
| Preview frames push saved frames' previews out of the eight. | A saved frame's file stays; its preview is 031's convenience (spec.md § Edge Cases). |
| The stretch is 031's: on a synthetic Saturn the globe and the rings come out near white, the bands' contrast gone, though the limb and the ring gaps still show sharp against soft (run by the review on the synthetic Saturns, `crates/finder/src/imaging/preview.rs:51`). | Focus by eye reads the edge; the owner focused on Saturn through 031's preview (the field report, read); a number from the crop is decision 4 (d). |
| A story grows past 60 % in the factory. | The largest, US3, measures 42.6 % on the probe and about 57.6 % at the worst overrun seen (§ Plan traps 2), 12,324 B of it the README's diff: a builder keeps to § The README's paragraphs. |

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
the survey of 2026-11-15. Folded the same day with the cynical review's round 1 (0
blockers, 5 majors, 11 minors), rebuilt on the same branch from f6d4f5fd's tree into
the same four stories, with both gates passing at each story's tip and on the final
tip, each new test failing on the story before it, a mutation of each new or changed
requirement caught by a scenario, the run with a preview frame in flight run on the
probe's release console, the crop measured on synthetic Saturns and field subs, and
the field night's cadence measured from its saved subs (§ What was run). Writes come
from the task text. Shared writes name the spec story that last wrote the same file on
`main`; ADR 0002 was last written by a spec flip, not a story, so it has none; within the
spec, each story's files written by an earlier one are ordered by `depends_on_merged`;
054 and 055 are re-anchored at the flip (§ Prerequisites 2).

None of the ten owner decisions of spec.md is answered; this record is written to
their recommendations, and approves nothing. The flip is the owner's. No open question
is left: each choice is an owner decision with a recommendation, so every story's
`open_questions` is empty.

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
        assertion: "preview::crop of a worked 16-bit RGGB 1000 x 900 frame of samples 100 with a patch of 20000 at (700, 300), the sample (100, 800) at 65535, cells of 1000 at (488, 104), (886, 502) and (486, 102) and the sample (588, 304) at 1000: 200 x 200, its only pixels not black (0, 0), (106, 98), (107, 98), (106, 99), (107, 99) and (199, 199) white and (50, 100) red; of a worked 8-bit mono 500 x 300 frame of 10 with a patch of 250 at (4, 260), the sample (400, 50) at 255 and the samples (0, 100), (199, 299) and (200, 299) at 200: 200 x 200, its only such pixels (0, 0), the 16 with x 4 to 7 and y 160 to 163, and (199, 199), white; of a worked 16-bit RGGB 1236 x 822 frame of 100 with patches of 20000 at (1000, 100) and (200, 700) and cells of 1000 at (808, 0) and (1206, 398): 200 x 200, its only such pixels (0, 0), (96, 50), (97, 50), (96, 51), (97, 51) and (199, 199), white; of a worked 16-bit RGGB 64 x 1 frame: preview::jpeg gives none"
        decision: "spec.md:US2-S1"
      - path: crates/finder/tests/console/imaging_preview_crop.rs
        status: new
        disposition: add
        assertion: "the crop rig, ready, exposure_ms 100: a preview frame until shown has crop /api/imaging/frames/1/crop.jpg and the route 200 a JPEG of 200 x 200; a saved frame until saved has no crop and /api/imaging/frames/2/crop.jpg 404; after eight more preview frames, each until shown, frame 1's crop 404 with error the console no longer holds that crop, frames 3 and 10 200, /api/imaging/frames/abc/crop.jpg 400"
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
        assertion: "take_preview_frame {} on the bare MCP console: tool error this console takes no imaging frames; the MCP rig, ready, set_imaging_camera exposure_ms 200: {x: 1} tool error take_preview_frame takes no arguments; {} no error, structuredContent.frame state shown, purpose preview, no file, exactly two image blocks of image/jpeg, the first 618 x 411 and the second 200 x 200; R holds no frames folder; with Imaging::lease(the stack) out, tool error the imaging camera is held by the stack; tools/list holds take_preview_frame with outputSchema {type: object}"
        decision: "spec.md:US3-S1"
      - path: crates/finder/tests/mcp/imaging_preview_frames.rs
        status: new
        disposition: add
        assertion: "crates/finder/README.md and ADR 0002 read as files, the README's text read with each run of whitespace as one space: it holds The imaging camera has sixteen tools of its own, which answer what the page's Imaging page does (a seventeenth, for its preview frames, follows the Tools page's tools below):; The Imaging page's preview frames have one tool: after The Tools page's focus aid for the finder camera has two tools: and before ### MCP under WSL2, with - `take_preview_frame` between them; it holds `exposing`, `receiving`, `saved`, `shown` (a preview frame's end, below), `failed` and `aborted`; ## Imaging frames, **One holder.**, ## Preview frames and ## Solving imaging frames in that order, The Imaging page's **Preview frames** are single frames too between the second and the third, nothing of it is saved and Stopping: the last preview frame is still in flight between the third and the fourth; the ADR holds Amended by spec 058: the console saves every frame taken to be kept; a preview frame is shown and saved nowhere."
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
      - "take_imaging_frame and its description are unchanged; the README's sixteen tools sentence keeps its words, gaining only a clause naming the seventeenth"
      - "ADR 0002's decision text is unchanged; it gains one line after its last"
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
      - docs/adr/0002-imaging-cameras-through-indi-frames-relayed-console-stacks.md
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
        assertion: "PreviewBlock over the stub console, later reads each shown frame N: the region Preview frames shows US4-S1's words; Start preview frames pressed: status Waiting for the first preview frame; image Crop of preview frame 1 around its brightest part with src /api/imaging/frames/1/crop.jpg and status Preview frame 1 · not saved; status for frame 2 matches Preview frame 2 · every <s.s> s · not saved; every POST to /api/imaging/preview; Stop preview frames pressed: status Stopping: the last preview frame is still in flight at once, no status within 3 s; then 1.2 s with no POST, button Start preview frames, no alert; Start preview frames pressed again: status Waiting for the first preview frame and no image at once, then the crop Crop of preview frame 4 around its brightest part and status Preview frame 4 · not saved"
        decision: "spec.md:US4-S1"
      - path: crates/finder/web/src/components/PreviewBlock.test.tsx
        status: new
        disposition: add
        assertion: "PreviewBlock over a fetch answering 409 error the imaging camera is held by the stack: Start preview frames pressed: alert exactly Preview frames stopped: the imaging camera is held by the stack., button Start preview frames, fetch called once"
        decision: "spec.md:US4-S2"
      - path: crates/finder/web/src/components/PreviewBlock.test.tsx
        status: new
        disposition: add
        assertion: "PreviewBlock over the stub console, later reads frame N failed with error the imaging frame stalled; frame N aborted; and frame N + 1 exposing purpose focus with the second POST answered 409 error the imaging camera is held by the focus aid: alerts exactly Preview frames stopped: the imaging frame stalled. after one POST, Preview frames stopped: the exposure was aborted. after one, and Preview frames stopped: the imaging camera is held by the focus aid. after two, each with no image in the block"
        decision: "spec.md:US4-S3"
      - path: crates/finder/web/src/components/PreviewBlock.test.tsx
        status: new
        disposition: add
        assertion: "PreviewBlock over the stub console, later reads each shown frame N: started, frame 1's crop shown, document.visibilityState hidden and visibilitychange dispatched: button Stop preview frames at once, no request in 1.2 s; visible and dispatched again: the image Crop of preview frame 2 around its brightest part within 3 s and exactly one POST more within 3 s; unmounted: no POST in 1.2 s"
        decision: "spec.md:US4-S4"
      - path: crates/finder/web/src/components/PreviewBlock.test.tsx
        status: new
        disposition: add
        assertion: "ImagingPage (day), fetch answering a ready view of the simulated imaging camera with shown frame 7: the region Preview frames follows the region Imaging frame in the document; Imaging frame holds the image Imaging frame 7 with src /api/imaging/frames/7/preview.jpg; Preview frames holds the button Start preview frames"
        decision: "spec.md:US4-S5"
      - path: crates/finder/web/e2e/imaging-preview.spec.ts
        status: new
        disposition: add
        assertion: "the simulated console, 412 x 915 at device scale 2, day: /imaging, exposure 200 saved in Settings, Start preview frames in Preview frames: the crop Crop of preview frame <N> around its brightest part within 15 s, natural size 200 x 200; the status's frame number up by 3 within 20 s, then Preview frame <N> · every <s.s> s · not saved; GET /api/imaging frame purpose preview and no file; scrollWidth at most 412 by day and by night; the day full-page screenshot over 10,000 pixels off night red (any green or blue, or red above 226) and the night one 0 after the switch Night mode: red and black; Start preview frames after Stop preview frames; once the block shows no status, an image Imaging frame <any number> in Imaging frame of natural width 618; finally about:blank and no frame in flight, then the frame id unmoved 1.5 s later"
        decision: "spec.md:US4-S6"
      - path: crates/finder/web/src/components/PreviewBlock.test.tsx
        status: new
        disposition: add
        assertion: "PreviewBlock over a fetch answering the first POST with frame 1 exposing, the first read frame 1 exposing, reads 2 to 5 not reaching the console (fetch rejects with a TypeError), later reads shown frame 1, and the second POST not reaching the console: Start preview frames pressed: status Waiting for the console within 3 s; then the alert exactly Preview frames stopped: the console did not answer.; the image Crop of preview frame 1 around its brightest part still shown, button Start preview frames; exactly two POSTs to /api/imaging/preview"
        decision: "spec.md:US4-S7"
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
