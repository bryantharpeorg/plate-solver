---
state: draft
depends_on_landed: ["013-live-finder", "016-mcp", "030-imaging-camera-link", "031-imaging-frames-on-console", "032-imaging-solve-and-aim-point", "033-focus-reading-and-aid", "035-live-stack", "042-imaging-run-page-and-mcp", "050-tools-page-aligns-finder-camera", "051-tools-page-focuses-either-camera"]
# Drafted 2026-11-27 against `main` at f6d4f5fd, at the owner's ask of 2026-11-27 (the
# owner picked "Imaging live view" from the operator seat's options after a field night;
# the label and its description are the seat's words, not the owner's), from a probe of
# that day on the branch `probe/058` cut from f6d4f5fd: a three-story trial build, then
# re-cut on the same branch into the four stories below (tip cd55ef08; plan.md § What was
# run and what was read). Every `path:line` here was read at f6d4f5fd. None of the nine
# owner decisions is answered: each carries its recommendation, and the scenarios and the
# record are written to the recommendations. 013 is named for the simulator the rigs run
# on; 016 for MCP; 030 for the simulated imaging camera and the node's own copy; 031 for
# the imaging frame, its preview, the lease and the Imaging page; 032 for the imaging
# solve's refusal; 033 for the focus aid's refusal; 035, 042, 050 and 051 because each
# last changed a file this spec writes (plan.md § Refinement Record). 052, 053, 054 and
# 055 are drafted beside this spec and land before it, in that order; it is re-anchored
# on them before any flip.
---

# Feature Specification: Preview Frames Save Nothing

**Feature Branch**: `058-preview-frames-save-nothing`

**Created**: 2026-11-27

**Status**: Draft

**Input**: On a field night the focus aid refused the imaging camera's first stars,
donuts of about 80 px (`no_region`), until the owner had turned the focuser by eye; later
the owner refocused by eye on Saturn through a viewer the field driver built for the
night, and the next stack's stars were visibly tighter than the first stack's, which had
followed the focus aid (the field report, read). That viewer asked the console for one
imaging frame after another through `POST /api/imaging/frame`, which saves every frame
it takes: a 32 MB FITS file each time, about 380 files and 12 GB under the console's
frames folder by the end of the night. After that night the owner picked, from
the operator seat's options, the one the seat labelled "Imaging live view" and described
to them as "A live view of the imaging camera on the console page for focusing by eye,
using preview frames that aren't saved." Those are the seat's words; the owner's logged
words on the feature are none beyond that pick. Give the Imaging page a way to watch the
imaging camera frame after frame, the whole frame and its centre at full resolution, at the
camera's own pace, saving nothing.

## Clarifications

### Session 2026-11-27 (the probe on `main` at f6d4f5fd)

- Q: What does this trio read besides itself? → A: Nothing about the feature. `spec.md`
  holds every requirement and scenario, and `plan.md` every design decision, the
  interfaces, the shapes that existing tests pin and the alternatives rejected. The
  probe's branch and logs are not in the repository; what they showed is written here.
- Q: Why does every frame save today? → A: `POST /api/imaging/frame`
  (`crates/finder/src/imaging/routes.rs:31`) is "one frame, saved as FITS"
  (`crates/finder/src/imaging/routes.rs:190`), and `Imaging::take` says "Every frame
  `take` starts is saved to a file" (`crates/finder/src/imaging/mod.rs:741`): once the
  frame is whole the task saves it (`crates/finder/src/imaging/frames.rs:383`), with the
  node's own copy when `save_at_node` is on (`crates/finder/src/imaging/frames.rs:218`),
  and makes its preview in the same blocking step
  (`crates/finder/src/imaging/frames.rs:453`), kept among the last eight
  (`crates/finder/src/imaging/frames.rs:464`, `crates/finder/src/imaging/preview.rs:13`).
  The focus aid saves each of its frames for the imaging solver's star measure and
  removes the one before (`crates/finder/src/imaging/focus/run.rs:932`); nothing else
  takes a frame that is not kept.
- Q: How large is the preview, and can a star's focus be judged on it? → A: The preview
  is box-averaged by a whole factor until its longer side is at most 1280 pixels
  (`crates/finder/src/imaging/preview.rs:9`, `crates/finder/src/imaging/preview.rs:53`).
  A 4944 × 3284 frame of the imaging camera, a field night's own 32,477,760 B FITS file,
  gives a 1236 × 821 preview, each pixel 4 × 4 samples, a JPEG of 326,587 B (run). A
  focused star a few samples across is a pixel or two there, so the eye needs the
  centre at full resolution beside it: that frame's middle 800 × 800 samples, one pixel
  a 2 × 2 cell, are a 400 × 400 picture of 22,742 B (run).
- Q: What holds the imaging camera? → A: The lease (031 FR-028): the focus aid, darks
  and flats, the stack, the imaging run and the finder alignment each take it, and while
  it is out a single frame is refused with `the imaging camera is held by` and the
  holder's name (`crates/finder/src/imaging/mod.rs:975`). A single frame holds none (031
  FR-033): while one is in flight each holder's start is refused with
  `an exposure is in progress` (`crates/finder/src/imaging/focus/run.rs:382`,
  `crates/finder/src/imaging/stack/service.rs:411`,
  `crates/finder/src/imaging/run/console.rs:372`,
  `crates/finder/src/imaging/align.rs:237`,
  `crates/finder/src/imaging/calibration/service.rs:396`). A preview frame taken as a
  single frame keeps both rules unchanged (owner decision 1; run).
- Q: How does the Imaging page see a frame end? → A: It reads `GET /api/imaging` once a
  second (`crates/finder/web/src/useImaging.ts:17`) and draws the latest frame's
  preview while the frame has one (`crates/finder/web/src/components/ImagingPage.tsx:271`).
  The probe's loop reads every 250 ms while its frame is in flight and asks for the next
  as soon as one is shown: on the simulated console, 200 ms frames came a mean 1.8 times
  the exposure apart (from 1.26 to 2.67 times, over 20) and 1000 ms frames 1.27 times
  (over 10): the exposure, the wait for the next read and the console's own share (run).
  The real camera's frame also has its download over the scope link, a larger share
  than the simulator's (the field night's notes, read, not run).
- Q: What was written while previewing? → A: Nothing. After 30 preview frames the
  simulated console's state folder held its `session.json` alone, and after 10 saved
  frames of the same camera it held 20,361,600 B more under `imaging/frames/` (run).
- Q: What of the glossary's "live view"? → A: It is an _Avoid_ of **Preview** and of
  **Stack** (`GLOSSARY.md:159`, `GLOSSARY.md:203`). This spec names the frame a
  **Preview frame** and the block **Preview frames** (owner decision 9).

## Owner decisions

None is answered. Each carries its options, its evidence and a recommendation, and the
scenarios and the record are written to the recommendations. One general rule is behind
decisions 1, 2, 5 and 8: **a preview frame is a single imaging frame that the console
does not save; every other rule is a single frame's.** It takes the settings in force,
holds no lease, is refused and refuses others exactly as a single frame does, and is
ended by abort as a single frame is; so it needs no lease name, no remembered settings,
no put-back, no time limit and no end rule of its own, and the page's loop, not the
console, decides when the next is asked for. Its own parts are the route, the frame's
end without a file, the crop, the MCP tool and the page's block.

1. **A loop on the console, or a single frame the page repeats** (FR-001, FR-004,
   FR-013). Recommended (a): **a preview frame, one unsaved single frame per request,
   that the page asks for again as soon as the last is shown**:
   `POST /api/imaging/preview`. Evidence: built and run in the probe (four stories, the
   largest 22,476 B); 200 ms frames a mean 1.8 times the exposure apart on the
   simulated console, the exposure and the wait for the next 250 ms read (run); nothing
   written (run); leaving the page
   ends it with no rule on the console (run in the browser test and Vitest). Instead:
   (b) **a live mode on the console**, `POST` and `DELETE /api/imaging/live`, a task
   that holds the lease and takes frames back to back: one remote Stop for MCP and for a
   second page, each frame up to one 250 ms read sooner,
   and a holder's start refused with the live
   mode's name rather than `an exposure is in progress`; but it holds the camera until
   stopped, so it needs an end of its own (a time limit, or a keepalive the page sends),
   a lease name and a view object, and it is about twice (a)'s console code (an estimate;
   not built);
   (c) **a `save` key on `POST /api/imaging/frame`**: one route, but 031's route takes no
   body today, its existing tests send `null` with a JSON content type
   (`crates/finder/tests/console/imaging_preview.rs:237`,
   `crates/finder/tests/support/live.rs:98`) and the page sends no content
   type (`crates/finder/web/src/api.ts:774`), and MCP's `take_imaging_frame` answers
   `take_imaging_frame takes no arguments` (`crates/finder/src/mcp/tools.rs:745`).
2. **Exposure, gain and binning while previewing** (FR-001). Recommended (a): **the
   settings in force**, set in the Imaging page's Settings, as any single frame takes
   them; nothing is remembered or put back. To change them the observer stops the
   preview frames, saves the settings and starts again: a settings change is refused
   while any frame is in flight (031 FR-004's `an exposure is in progress`; run). 2 × 2
   binning gives a quarter of the bytes and a quicker frame; the crop is then of binned
   pixels. Instead: (b) **the block's own exposure and gain**, set for the preview frames
   and put back after: the focus aid's remember-and-put-back, whose failure left a
   region set on a field night (054's subject); (c) **one fixed short exposure**: a planet
   wants tens of milliseconds and a star field a second or more; (d) **the block pausing
   between frames for a settings change**: smoother, but a second state shared between
   the Settings form and the block.
3. **The crop: its size and where it sits** (FR-006). Recommended (a): **the frame's
   centre, 400 × 400 cells at full resolution**, one picture pixel a 2 × 2 cell of the
   mosaic (one sample of a mono frame): on the imaging camera 800 × 800 samples, about
   7.3′ at the field night's 0.5475″ a sample; a JPEG of 22,742 B against the preview's
   326,587 B, made in about an eighth of the preview's own time (a ratio of 0.124, run).
   GoTo and Track put the target at the imaging camera's centre, and 055's nudge arrows
   move it there by hand. Instead: (b) **200 × 200 cells**: twice the size on a phone,
   3.6′ across, a target a little off centre falls out of it; (c) **a tap on the whole
   frame to place the crop**: a centre sent with each request, a second picture to keep
   in step with the tap, more code; (d) **the camera's region of interest as the crop**:
   far fewer bytes a frame, but no whole frame, and a setting changed and put back (2 (b)).
4. **The focus aid's star size beside it** (FR-012). Recommended (a): **none**: the aid
   and preview frames each need the camera and cannot run together, and a star size
   needs a saved file (033's measure is the imaging solver's on a FITS file,
   `crates/finder/src/imaging/focus/run.rs:932`), which is what this spec avoids; the
   Focus block sits below for a reading once the eye has done its part. Instead: (b)
   **the focus record as a line in the block**: cheap, from the view, but a record is a
   settled reading of the focus region, not of what the eye sees; (c) **a reading of each
   preview frame**: a file written and removed each frame, and the solver's time added to
   each.
5. **How it ends** (FR-014). Recommended (a): **Stop, leaving the page, hiding it
   (another tab, a phone's screen locked), a refusal, or a frame that did not end
   shown**; no time limit, and a frame in flight ends by itself. Instead: (b) **also a
   time limit**: a page left open and shown elsewhere keeps the camera busy, so each
   holder's start is refused with `an exposure is in progress` until someone presses
   Stop; (c) **Stop also aborts the frame in flight**: frees the camera sooner after a
   long exposure, but an abort ends whatever frame is in flight
   (`crates/finder/src/imaging/routes.rs:224`), which may by then be another's.
6. **Where on the page** (FR-012). Recommended (a): **a block `Preview frames` on the
   Imaging page right after the section `Imaging frame`**, whose own picture shows the
   whole frame (`crates/finder/web/src/components/ImagingPage.tsx:271`); the block shows
   the crop. Instead: (b) **inside `Imaging frame`, beside `Take imaging frame`**: one
   section fewer, but 054 rewrites that section's picture and 055 adds the Nudge block
   after the camera's sections;
   (c) **the Tools page**, beside the focus aid's block: away from the whole
   frame's picture.
7. **MCP** (FR-009). Recommended (a): **one tool, `take_preview_frame`**, which takes no
   arguments and answers the view with the preview and the crop as two images: an MCP
   client looking at the sky saves nothing either (the field night's viewer was an
   agent's, and filled 12 GB). Instead: (b) **none**: an MCP client calls
   `POST /api/imaging/preview` itself, as that viewer called the frame route; (c)
   **an argument on `take_imaging_frame`**: changes 031's tool and its words (1 (c)).
8. **"Save this one"** (FR-005). Recommended (a): **none new**: `Take imaging frame`
   in the section above saves the next frame at the same settings. Instead: (b) **keep
   the last preview frame's samples** (32,472,192 B on the imaging camera) to save on
   request: memory held while previewing, a route and its words.
9. **The name** (FR-011). Recommended (a): **Preview frame**, the block **Preview
   frames**: "live view" is an _Avoid_ of **Preview** and of **Stack**
   (`GLOSSARY.md:159`, `GLOSSARY.md:203`). Instead: (b) **live view**, the seat's label:
   two glossary entries change.

## Owner approvals

Flipping this spec to `ready` approves the answer to every owner decision above, and the
following. Each is named in the FR that makes it. No existing test, golden, fixture or CI
file changes.

1. **New public API in `crates/finder`'s library** (FR-001, FR-006, FR-007):
   `Imaging::take_preview`, `Imaging::crop`, `preview::crop`, `preview::CROP_SIDE` and
   the frame view's `crop`. No existing signature changes.
2. **New routes and a tool** (FR-001, FR-008, FR-009): `POST /api/imaging/preview`,
   `GET /api/imaging/frames/{id}/crop.jpg` and the MCP tool `take_preview_frame`.
3. **A sixth frame state, `shown`** (FR-002): 031 FR-009's list of states gains it.
4. **The glossary** (FR-011): the entry **Preview frame**, and `preview` among the
   **Purpose label**'s words.
5. **The finder README** (FR-010): `## Preview frames`, two rows of the API table, the
   preview row's and the preview paragraph's words for the frames given one, and a
   paragraph of MCP tools of its own. The pinned sentence
   `The imaging camera has sixteen tools of its own` (`crates/finder/README.md:88`)
   stays as it is.
6. **Two list entries in `crates/finder/web/playwright.config.ts`** (FR-016):
   `"**/imaging-preview.spec.ts"` in the default project's `testIgnore` and the simulate
   project's `testMatch`, each on a line of its own after `"**/imaging.spec.ts"`
   (`crates/finder/web/playwright.config.ts:42`,
   `crates/finder/web/playwright.config.ts:62`).
7. **The page** (FR-012): the Imaging page's block `Preview frames`.
8. **ADR 0002 is not edited**: its "The console saves every frame"
   (`docs/adr/0002-imaging-cameras-through-indi-frames-relayed-console-stacks.md:3`)
   stays the rule for every frame taken to be kept; the glossary's **Preview frame** names
   the one kind that is not.

## User Scenarios & Testing *(mandatory)*

Every scenario shares these definitions.

- **The gate commands.** Every Rust scenario runs in
  `cargo nextest run --workspace --all-targets --all-features --locked`, the `test`
  gate's first command, each test in its own process. The page's Vitest scenarios run in
  `npm test`, and the browser scenario in `CI=1 npm run e2e`, later commands of the same
  gate, in `crates/finder/web`.
- **The simulated imaging camera** is `SimImagingCamera`, 1236 × 822 samples, RGGB, 16
  bits (`crates/simulator/src/imaging.rs:145`, `crates/simulator/src/imaging.rs:146`):
  its preview is 618 × 411 and its crop 400 × 400 (the middle 800 × 800 samples, from
  column 218 and row 10).
- **The preview rig** is the simulator `crate::support::live::sim()` with a `SimMount`
  connected; a `SimNode` over it with the token `test-token` and a 500 ms frame
  interval, its imaging camera the simulated one, keeping its own copies in a folder N
  through `with_imaging_save_dir`, served over a fresh in-memory duplex for each
  connection with the node's routes; a `Session` paired with it by the same edit as
  031's preview rig (`crates/finder/tests/console/imaging_preview.rs:171`, its node,
  token `test-token` and test site); and `AppState::shared` over
  `crate::support::live::DB` with `with_session`,
  `with_clock` of the simulator's clock, `with_link`, `with_live()`,
  `with_imaging(R)` and `with_imaging_solver` of a `SimulatedSolver` over the
  simulator, served by `finder::router`. **The crop rig** is the same with no N and no
  solver. R and N are folders of a `crate::support::Scratch`. **Ready** means
  `GET /api/imaging` reads `state` `ready`. Views are read every 100 ms; the tests run
  under `#[tokio::test(start_paused = true)]`, and every state awaited comes within
  120 s of paused time. **The listing** of a folder is every file under it, as paths
  relative to it, sorted; a folder that does not exist lists none. **A foreign write**
  is a request whose `Host` header is localhost on port 8080 and whose `Origin` header is
  `http://elsewhere.example`.
- **The MCP rig** is the crop rig served through `crate::support::mcp::app` at
  `127.0.0.1`, the tools called with `tools/call`; **the bare MCP console** is
  `crate::support::mcp::state()`, with no link, served the same way.
- **A worked frame** is a frame of the test's own making, its samples row by row from
  the top-left, 16-bit little-endian or 8-bit; **a cell at (x, y)** is the four samples
  (x, y), (x + 1, y), (x, y + 1) and (x + 1, y + 1).
- **The page rig** is the component rendered by Testing Library with `fetch` stubbed.
  **The stub console** answers each `POST /api/imaging/preview` with a `ready` view whose
  `frame` is the next id from 1, `exposing`, purpose `preview`, exposure 200 ms; and
  each `GET /api/imaging` with a `ready` view whose `frame` is, on the first read after a
  `POST`, that frame `exposing`, and on later reads the frame the scenario names. A
  **shown frame N** is frame N `shown` with `preview` `/api/imaging/frames/N/preview.jpg`
  and `crop` `/api/imaging/frames/N/crop.jpg`.
- **The simulated console** is the simulate project's server of
  `crates/finder/web/playwright.config.ts`, the release `finder` with `--simulate`.
- **Oracles.** The simulated imaging camera's sensor; the worked frames and their
  hand-worked crops; the folders' own listings; 031's, 032's and 033's words; the
  scenarios' words; DESIGN.md's night ink. No expected value comes from the code under
  test: no test calls `crop` or `rgb` for an expected value or reads `CROP_SIDE`,
  `PREVIEW_SIDE`, `PREVIEWS_KEPT` or `POLL_MS`.

### User Story 1 - The console takes a preview frame and saves nothing (Priority: P1)

As the owner, I need the console to take an imaging frame that is shown and never
written anywhere, so that watching the imaging camera costs no disk.

**Why this priority**: Every other part shows or asks for these frames.

**Independent Test**: `crates/finder/tests/console/imaging_preview_frames.rs`, new. The
`test` gate's nextest command runs it; during development,
`cargo nextest run -p finder --locked --test console -E 'test(imaging_preview_frames)'`.

**Acceptance Scenarios**:

1. **Given** the preview rig, ready, with `PUT /api/imaging/settings`
   `{"exposure_ms": 200, "save_at_node": true}` answered 200, **When** a committed test
   sends `POST /api/imaging/preview`, reads the view until its frame is `shown`, gets
   `/api/imaging/frames/1/preview.jpg`, then sends `POST /api/imaging/frame` and reads
   until frame 2 is `saved`, **Then** it asserts that the `POST` answers 200 with frame 1,
   purpose `preview`, `exposing`; that the shown frame has `width` 1236, `height` 822 and
   `preview` `/api/imaging/frames/1/preview.jpg`, and none of `file`, `saved_at`,
   `node_file`, `node_save_error` and `error`; that the preview answers 200 with a JPEG
   of 618 × 411; that R and N then list nothing; and that after frame 2 R lists exactly
   `frames/2026-01-16/000002.fits` and N exactly the frame's `node_file`.
2. **Given** the preview rig, ready, with `exposure_ms` 2000, **When** a committed test
   takes `Imaging::lease("the stack")` and sends `POST /api/imaging/preview`, drops the
   lease and sends it again, then at once `POST /api/imaging/preview`,
   `POST /api/imaging/frame`, `PUT /api/imaging/settings` `{"exposure_ms": 100}` and
   `POST /api/imaging/focus` `{}`, then a foreign write of `POST /api/imaging/preview`,
   reads until the frame is `shown`, sends `POST /api/imaging/solve`, and sends
   `POST /api/imaging/preview` to `AppState::shared` alone, **Then** it asserts 409
   `the imaging camera is held by the stack`; 200; 409
   `an exposure is already in progress` twice; 409 `an exposure is in progress` for the
   settings and for the focus aid, with `Imaging::holder()` `None`; 403 for the foreign
   write; 409 `the latest imaging frame was not saved`; 409
   `this console takes no imaging frames`; and that R lists nothing.
3. **Given** the preview rig, ready, with `exposure_ms` 5000, **When** a committed test
   sends `POST /api/imaging/preview`, waits 1 s, sends `POST /api/imaging/abort`, reads
   until the frame is `aborted`, sets `exposure_ms` 100 and sends
   `POST /api/imaging/preview` again, reading until it is `shown`, **Then** it asserts
   that the aborted frame has no `preview`, that the second `POST` answers frame 2 and
   the shown frame `preview` `/api/imaging/frames/2/preview.jpg`, and that R lists
   nothing.

### User Story 2 - The centre of each preview frame at full resolution (Priority: P1)

As the owner, I need the centre of each preview frame at the camera's own resolution, so
that I can see a star or a planet sharpen as I turn the focuser.

**Why this priority**: The preview alone is too coarse to judge focus by eye.

**Independent Test**: `crates/finder/tests/console/imaging_preview_crop.rs`, new; during
development,
`cargo nextest run -p finder --locked --test console -E 'test(imaging_preview_crop)'`.

**Acceptance Scenarios**:

1. **Given** a worked 16-bit RGGB frame of 1000 × 900 samples of 100 with cells of 1000
   at (100, 50), (898, 848) and (98, 48) and the sample (500, 450) at 1000; a worked
   8-bit mono frame of 500 × 300 samples of 10 with the samples (50, 0), (449, 299) and
   (450, 299) at 200; a worked 16-bit RGGB frame of 1236 × 822 samples of 100 with a cell
   of 1000 at (218, 10); and a worked 16-bit RGGB frame of 64 × 1 samples of 100, **When**
   a committed test calls `preview::crop` on each and `preview::jpeg` on the last's,
   **Then** it asserts that the first is 400 × 400 and its only pixels that are not
   black are (0, 0) and (399, 399) white, (255, 255, 255), and (200, 200) red,
   (255, 0, 0); that the second is 400 × 300 and its only such pixels are (0, 0) and
   (399, 299), white; that the third is 400 × 400 and its only such pixel is (0, 0),
   white; and that the last gives no JPEG.
2. **Given** the crop rig, ready, with `exposure_ms` 100, **When** a committed test takes
   a preview frame until `shown`, gets its crop, takes a saved frame until `saved`, gets
   `/api/imaging/frames/2/crop.jpg`, takes eight more preview frames, each until `shown`,
   and then gets the crops of frames 1, 3 and 10 and `/api/imaging/frames/abc/crop.jpg`,
   **Then** it asserts that frame 1's view has `crop` `/api/imaging/frames/1/crop.jpg` and
   the route answers 200 with a JPEG of 400 × 400; that the saved frame has no `crop` and
   its crop answers 404; that frame 1's crop then answers 404 with the error
   `the console no longer holds that crop`; that frames 3 and 10 answer 200; and that
   `abc` answers 400.

### User Story 3 - MCP and the words (Priority: P2)

As the owner, I need an MCP client to look through the imaging camera the same way, and
the README and the glossary to say what a preview frame is.

**Why this priority**: The page works without them; an agent at the telescope and the
next reader need them.

**Independent Test**: `crates/finder/tests/mcp/imaging_preview_frames.rs`, new; during
development,
`cargo nextest run -p finder --locked --test mcp -E 'test(imaging_preview_frames)'`.

**Acceptance Scenarios**:

1. **Given** the bare MCP console and the MCP rig, ready, with `set_imaging_camera`
   `{"exposure_ms": 200}`, **When** a committed test calls `take_preview_frame` `{}` on
   the bare console, then on the rig `take_preview_frame` `{"x": 1}` and `{}`, takes
   `Imaging::lease("the stack")` and calls `{}` again, drops the lease, and lists the
   tools, **Then** it asserts the tool error `this console takes no imaging frames`; the
   tool error `take_preview_frame takes no arguments`; for `{}` no error, a
   `structuredContent` whose `frame` is `shown` with purpose `preview` and no `file`, and
   exactly two image blocks of `image/jpeg`, the first 618 × 411 and the second
   400 × 400; that R holds no `frames` folder; the tool error
   `the imaging camera is held by the stack`; and that `tools/list` holds
   `take_preview_frame` with the output schema `{"type": "object"}`.
2. **Given** `crates/finder/README.md` read as a file, **When** a committed test finds
   its paragraphs and headings, **Then** it asserts that it still holds
   `The imaging camera has sixteen tools of its own`; that
   `The Imaging page's preview frames have one tool:` comes after
   `The Tools page's focus aid for the finder camera has two tools:` and before
   `### MCP under WSL2`, with a list entry for `take_preview_frame` between them; and that
   `## Preview frames` comes after `## Imaging frames` and before
   `## Solving imaging frames`, and holds `nothing of it is saved`.

### User Story 4 - The Imaging page shows preview frames (Priority: P2)

As the owner, at the telescope with a phone, I start preview frames on the Imaging page,
watch the whole frame and its centre sharpen while I turn the focuser, and stop.

**Why this priority**: The routes and MCP work without the page; the page is how the
owner uses them at the telescope.

**Independent Test**: `crates/finder/web/src/components/PreviewBlock.test.tsx`, new,
under `npm test`, and `crates/finder/web/e2e/imaging-preview.spec.ts`, new, under
`CI=1 npm run e2e`; during development, `npm test -- PreviewBlock` and
`npx playwright test --project=simulate e2e/imaging-preview.spec.ts` in
`crates/finder/web`.

**Acceptance Scenarios**:

1. **Given** the page rig with `PreviewBlock` over the stub console whose later reads are
   each shown frame N, **When** a committed test presses `Start preview frames`, waits
   for frame 1's crop and then for frame 2, presses `Stop preview frames` and waits
   1.2 s, **Then** it asserts that the region `Preview frames` shows
   `Imaging frames one after another at the settings above, shown and never saved, to focus or frame by eye. The whole frame shows under Imaging frame, and its centre at full resolution here.`;
   that the status reads `Waiting for the first preview frame` right after the press;
   that the image `Centre of preview frame 1 at full resolution` has the source
   `/api/imaging/frames/1/crop.jpg` and the status then reads
   `Preview frame 1 · not saved`; that the status for frame 2 matches
   `Preview frame 2 · every <seconds to one decimal> s · not saved`; that every `POST`
   went to `/api/imaging/preview`; and that after the stop no `POST` came, the button
   reads `Start preview frames`, and no status or alert shows.
2. **Given** the page rig with `PreviewBlock` over a `fetch` answering every request 409
   `{"error": "the imaging camera is held by the stack"}`, **When** a committed test
   presses `Start preview frames`, **Then** it asserts that the alert reads exactly
   `Preview frames stopped: the imaging camera is held by the stack.`, that the button
   reads `Start preview frames`, and that `fetch` was called once.
3. **Given** the page rig with `PreviewBlock` over the stub console, three times, its
   later reads frame N `failed` with the error `the imaging frame stalled`, frame N
   `aborted`, and frame N + 1 `exposing` with purpose `frame`, **When** a committed test
   presses `Start preview frames` each time, **Then** it asserts that the alerts read
   exactly `Preview frames stopped: the imaging frame stalled.`,
   `Preview frames stopped: the exposure was aborted.` and
   `Preview frames stopped: another imaging frame was taken.`, each after one `POST`
   and with no image in the block.
4. **Given** the page rig with `PreviewBlock` over the stub console whose later reads are
   each shown frame N, **When** a committed test presses `Start preview frames`, waits
   for a crop, sets `document.visibilityState` to `hidden` and dispatches
   `visibilitychange`, waits 1.2 s, then renders it again with the state `visible`,
   presses `Start preview frames`, waits for a `POST`, unmounts it and waits 1.2 s,
   **Then** it asserts that the button reads `Start preview frames` at once after the
   event, and that no `POST` came in either 1.2 s.
5. **Given** the page rig with `ImagingPage` (day), `fetch` answering a `ready` view of
   the simulated imaging camera whose frame is shown frame 7, **When** a committed test
   finds the regions `Imaging frame` and `Preview frames`, **Then** it asserts that
   `Preview frames` follows `Imaging frame` in the document, that `Imaging frame` holds
   the image `Imaging frame 7` with the source `/api/imaging/frames/7/preview.jpg`, and
   that `Preview frames` holds the button `Start preview frames`.
6. **Given** the simulated console in a 412 by 915 viewport at a device scale of 2, day
   mode, **When** a committed Playwright test opens `/imaging`, saves an exposure of 200 in
   the region `Settings`, presses `Start preview frames` in the region `Preview frames`,
   waits for its crop, waits until the status's frame number has risen by at least 3,
   reads `GET /api/imaging`, takes a full-page screenshot, turns night mode on with the
   switch `Night mode: red and black`, takes another, presses `Stop preview frames`, and
   in its `finally` leaves the page and reads `GET /api/imaging` until no frame is in
   flight, **Then** it asserts that the crop
   `Centre of preview frame <N> at full resolution` shows within 15 s with a natural size
   of 400 × 400; that the frame number rises by 3 within
   20 s and the status then matches
   `Preview frame <N> · every <seconds to one decimal> s · not saved`; that the image
   `Imaging frame <N>` in the region `Imaging frame` has a natural width of 618; that the
   view's frame has purpose `preview` and no `file`; that the document's `scrollWidth` is
   at most 412 by day and by night; that the day screenshot holds more than 10,000 pixels
   off night red and the night one none, counted as
   `crates/finder/web/e2e/imaging.spec.ts` counts them (any green or blue, or red above
   226); that `Start preview frames` shows after the stop; and that 1.5 s after the
   `finally` the view's frame id has not moved.

### Edge Cases

- **A settings change while previewing** is refused with 031's
  `an exposure is in progress` while a frame is in flight (US1-S2): stop, save, start
  again (owner decision 2). The Settings form shows the refusal as its alert today.
- **A holder's start while previewing** (the focus aid, darks or flats, the stack, a run,
  the finder alignment) is refused with `an exposure is in progress` while a preview
  frame is in flight (US1-S2 for the aid; the others read, not run): stop the preview
  frames first. **A holder already running** refuses each preview frame with its name
  (US1-S2), and the block stops with those words (US4-S2).
- **A page left open and shown** keeps asking; nothing is written, but the camera is
  busy for every holder until someone presses Stop there (owner decision 5).
- **Two pages, or a page and an MCP client**, each ask in turn; the second's request
  while the first's frame is in flight is refused with
  `an exposure is already in progress` (US1-S2), and a page whose frame was overtaken
  stops with `another imaging frame was taken` (US4-S3).
- **A failed or aborted preview frame** stops the block with its words (US4-S3); the
  `Abort exposure` button of the section above aborts a preview frame as any frame
  (US1-S3).
- **The latest frame a preview frame**: `POST /api/imaging/solve` refuses it (US1-S2),
  and the calibration library's match of the latest saved frame
  (`crates/finder/src/imaging/calibration/library.rs:173`) reads none until a frame is
  saved again (read, not run).
- **Ids**: a preview frame takes the console's next id, so saved files' numbers skip
  the ids of preview frames between them (US1-S1, US1-S3); an id is never reused
  within a run (031 FR-011).
- **The previews of saved frames** share the last eight with preview frames, so a saved
  frame's preview is gone after eight preview frames (US2-S2 for the crops; the
  preview's ring is 031's, read).
- **A long exposure**: Stop lets the frame in flight end by itself (owner decision 5);
  `Abort exposure` ends it at once.
- **The console stops mid-preview**: nothing was written and nothing is resumed (read,
  not run).

## Requirements *(mandatory)*

### Functional Requirements

**The console's preview frame** (US1):

- **FR-001**: `POST /api/imaging/preview` SHALL, under the same-origin rule, take one
  imaging frame exactly as `POST /api/imaging/frame` takes one (031 FR-008: "ask the node
  for one exposure of the settings' `exposure_ms` with the purpose `frame`, the
  settings' forwarding rate and `save_at_node` … and answer 200 with the view at once"),
  with the same refusals, words and order (031 FR-008, FR-029: the state's, the lease's
  `the imaging camera is held by {holder}`, and `an exposure is already in progress`),
  but with the purpose `preview`, and SHALL answer 409
  `this console takes no imaging frames` on a console with no imaging.
  `Imaging::take_preview()` SHALL be that take for the library (owner decisions 1 and 2;
  owner approvals 1 and 2).
- **FR-002**: A preview frame SHALL never be saved: no file under the imaging folder, and
  the node SHALL be asked with `save` false whatever `save_at_node` says. When the frame
  is whole the console SHALL make its preview as 031 FR-017 makes a saved frame's, keep
  it among the last eight previews of 031 FR-018, the same eight as saved frames', and
  end the frame `shown` with its `preview` and none of `file`, `saved_at`, `node_file`
  and `node_save_error`. This changes 031 FR-009 ("`state` (one of `exposing`,
  `receiving`, `saved`, `failed`, `aborted`)"), which gains `shown` (owner approval 3),
  031 FR-017 ("The console SHALL make a preview of each saved frame"), which makes one of
  each shown frame too, and 031 FR-018 ("while it is among the last eight … A saved
  frame's view SHALL carry the route as `frame.preview`"), whose eight are those of saved
  and shown frames together, a shown frame's view carrying the route as well.
- **FR-003**: A preview frame's id SHALL be the console's same count (031 FR-011): it
  takes the next id, and the next frame of either kind the one after.
- **FR-004**: A preview frame SHALL hold no lease, as 031 FR-033 says of a single frame:
  while it is in flight a settings change and every single frame SHALL be refused as for
  any frame in flight (031 FR-004, FR-008), and each holder's own check of a frame in
  flight refuses its start, unchanged.
  `POST /api/imaging/abort` SHALL end it `aborted` with no preview, as
  any frame (031 FR-013), and `POST /api/imaging/solve` SHALL refuse a shown frame with
  032's `the latest imaging frame was not saved`, unchanged: a shown frame has no
  `file`, so it is one of 032 FR-009's cases "when there is no saved latest frame"
  (`crates/finder/src/imaging/solving.rs:558`).
- **FR-005**: Nothing else about an imaging frame SHALL change: `POST /api/imaging/frame`,
  `take_imaging_frame`, a saved frame's file, cards, preview and words, the lease and
  its words are 031's and 033's, and no "save this one" is added (owner decision 8).

**The crop** (US2):

- **FR-006**: `crates/finder`'s `imaging::preview` SHALL gain `CROP_SIDE` (400) and
  `pub fn crop(samples: &[u8], width: u32, height: u32, bits: u32, bayer: Option<&str>) -> Rgb`:
  with c = 2 for a mosaic `rgb` reads (RGGB, BGGR, GRBG or GBRG) and 1 otherwise, the
  crop's sides in samples are w = min(400·c, ⌊width/c⌋·c) and h = min(400·c,
  ⌊height/c⌋·c), its first column ⌊⌊(width − w)/2⌋/c⌋·c and its first row
  ⌊⌊(height − h)/2⌋/c⌋·c, and the answer SHALL be `rgb` of those samples alone, so one
  pixel a cell, with no box average for a crop of at most 1280 cells a side, and
  stretched over its own samples (owner decision 3; owner approval 1).
- **FR-007**: A shown preview frame SHALL also get its crop, encoded as 031 FR-017's
  JPEG of quality 85 in the same blocking step as its preview, kept among the last eight
  crops, and its view SHALL carry `crop`, `/api/imaging/frames/{id}/crop.jpg`; a crop
  that cannot be made is left out, as a preview is. A saved frame SHALL have no `crop`.
  `Imaging::crop(id)` SHALL answer a kept crop.
- **FR-008**: `GET /api/imaging/frames/{id}/crop.jpg` SHALL answer that crop as
  `image/jpeg` while it is among the last eight, 404 with
  `the console no longer holds that crop` otherwise, and 400 for an id that is not a
  number, open to anyone as 031 FR-031 keeps the preview route (owner approval 2).

**MCP and the words** (US3):

- **FR-009**: The MCP tool `take_preview_frame` SHALL take no arguments (else the tool
  error `take_preview_frame takes no arguments`), answer
  `this console takes no imaging frames` on a console with no imaging, take a preview
  frame as FR-001 does, wait until it has ended, and answer the imaging view with the
  frame and the preview and the crop as two `image/jpeg` blocks in that order; a refusal
  SHALL be a tool error with the route's words, an aborted frame
  `the exposure was aborted` and a failed one its error, as `take_imaging_frame` answers
  them (`crates/finder/src/mcp/tools.rs:740`). Its description SHALL be "Take one preview
  frame, as POST /api/imaging/preview does: an imaging frame at the settings in force
  that is shown and never saved, at the console or at the scope node. It waits until the
  frame is shown and returns it with its colour JPEG preview and the crop of its centre
  at full resolution." (owner decision 7; owner approval 2).
- **FR-010**: `crates/finder/README.md` SHALL gain `## Preview frames` after
  `## Imaging frames` and before `## Solving imaging frames`, with plan.md § The README's
  paragraphs; the API table's rows for `POST /api/imaging/preview` and
  `GET /api/imaging/frames/{id}/crop.jpg` before the preview's row, whose words and those
  of `## Imaging frames`' preview paragraph name the frames given one; and, after the
  finder camera focus aid's MCP paragraph and before `### MCP under WSL2`, the paragraph
  `The Imaging page's preview frames have one tool:` with its entry. The sentence
  `The imaging camera has sixteen tools of its own` SHALL stay as it is (owner
  approval 5).
- **FR-011**: `GLOSSARY.md` SHALL gain **Preview frame**, before **Sub**, with plan.md
  § The glossary's words, and the **Purpose label**'s list SHALL end
  "`run`, `align` and `preview`." (owner decision 9; owner approval 4).

**The page** (US4):

- **FR-012**: The Imaging page SHALL render, right after the section `Imaging frame`
  and while it does, the block `<PreviewBlock />`: a section named by its level-two
  heading `Preview frames`, holding the words of US4-S1, the button
  `Start preview frames` (`Stop preview frames` while running), a status, an alert and
  the crop (owner decisions 4 and 6; owner approval 7).
- **FR-013**: `Start preview frames` SHALL send `POST /api/imaging/preview`
  (`postImagingPreview` in `api.ts`), read `GET /api/imaging` every `POLL_MS` (250 ms)
  while the view's frame is that frame `exposing` or `receiving`, and once it is
  `shown` show its crop and status and send the next `POST` at once. The status SHALL
  read `Waiting for the first preview frame` until a frame is shown, then
  `Preview frame N · not saved`, and from the second frame on
  `Preview frame N · every S s · not saved`, S the seconds between the last two frames'
  ends seen by the page, to one decimal.
- **FR-014**: The loop SHALL end, sending nothing more, on `Stop preview frames`, when
  the block unmounts, and when `document.visibilityState` turns `hidden`; and with the
  alert `Preview frames stopped: <words>.` on a refused `POST` (the route's error), on
  a frame ended `failed` (its error), `aborted` (`the exposure was aborted`), or overtaken
  by another frame (`another imaging frame was taken`) (owner decision 5).
- **FR-015**: The crop SHALL be an image named `Centre of preview frame N at full
  resolution`, in a `div` of the classes `border border-ink bg-photo` and with the
  classes `photo block h-auto w-full`, as the frame's own picture is drawn, so that night
  mode reddens it (`crates/finder/web/src/index.css:118`). `api.ts`'s `ImagingFrame`
  SHALL gain the state `shown` and `crop?: string`.
- **FR-016**: `crates/finder/web/e2e/imaging-preview.spec.ts` SHALL hold US4-S6, with
  `"**/imaging-preview.spec.ts"` added to the default project's `testIgnore` and the
  simulate project's `testMatch` (owner approval 6); `crates/finder/web/dist` SHALL be
  rebuilt and committed. Nothing else on the page SHALL change: the Imaging page gains
  one import and one line, and every other block is untouched.

### Key Entities

- **Preview frame**: an imaging frame taken at the settings in force only to be looked
  at, ending `shown` with its preview and crop in memory, never saved at the console or
  at the scope node.
- **Crop**: the centre of a preview frame at full resolution, one picture pixel a cell.
- **The Preview frames block**: the Imaging page's block that asks for one preview frame
  after another while it is open and shown, and draws each one's crop.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario has a committed test that asserts it, and the
  `lint` and `test` gates pass.
- **SC-002**: No existing test, golden, fixture, `scripts/ci/` file, `.github/` file,
  `CODEOWNERS` entry or `ergane.yaml` is edited, and `playwright.config.ts` gains only its
  two list entries. Every existing test passes unchanged (plan.md § Shapes that existing
  tests pin). The only edits to existing test files are the `mod` lines of
  `crates/finder/tests/console/main.rs` and `crates/finder/tests/mcp/main.rs`.
- **SC-003**: Taking preview frames writes no file under the imaging folder or the
  node's save folder, with `save_at_node` on (US1-S1, US1-S2, US1-S3, US3-S1).
- **SC-004**: A preview frame is refused, and refuses others, exactly as a single frame
  is (US1-S2), and its crop is the frame's centre at full resolution (US2-S1, US2-S2).
- **SC-005**: On the simulated console, with 200 ms frames, the page's frame number
  rises by at least 3 within 20 s (US4-S6), and the page stops asking when stopped,
  hidden or left (US4-S1, US4-S4, US4-S6).

## Assumptions

- **Earlier features are in place.** Every spec in `depends_on_landed` has landed. Every
  citation here was read at f6d4f5fd. 052 to 055 land before this spec; it is
  re-anchored on their landed `main` before any flip (plan.md § Prerequisites).
- **The simulated imaging camera stands for the real one** in the gates: an RGGB
  mosaic of 16-bit samples, delivered whole after its exposure. The real camera's
  download of 32 MB a frame over the scope link, a larger share of each frame than the
  simulator's, is read from the field night's notes, not run.
- **Out of scope:**
  - the finder camera's live view, which it has on Setup;
  - a focus reading of planets, and any reading of preview frames (owner decision 4);
  - removing frames already saved;
  - centring a target (spec 057) and the nudge arrows (055);
  - a "save this one" beyond `Take imaging frame` (owner decision 8);
  - the first use at the telescope, which is the owner's (plan.md § Outside the factory).
- **The factory's gates and CI:** the gates are `lint` and `test`, copied in plan.md
  § Verification. CI's other jobs run only in the merge queue, so no scenario's Then
  clause rests on them.

## Work Graph

```yaml
US1:
  depends_on: []
  implements: [FR-001, FR-002, FR-003, FR-004, FR-005]
US2:
  depends_on: []
  depends_on_merged: [US1]
  implements: [FR-006, FR-007, FR-008]
US3:
  depends_on: []
  depends_on_merged: [US2]
  implements: [FR-009, FR-010, FR-011]
US4:
  depends_on: []
  depends_on_merged: [US2]
  implements: [FR-012, FR-013, FR-014, FR-015, FR-016]
```

Four stories, each under 60 % of 65,536 B (39,321 B) at 1.35 times its probe size, as the
probe's re-cut measures them (plan.md § Plan traps 2): the preview frame on the console;
its crop; MCP, the README and the glossary; the page. US2 merges after US1, since its crop
is made in US1's `show` step and its rig takes US1's frames; US3 and US4 each merge after
US2, since the tool answers the crop and the page draws it. US3 and US4 write no file in
common, so they may run side by side. Plain `depends_on` would order them without putting
the earlier code in the later story's base.
