---
description: "Task list for 034-dark-and-flat-frames: Dark and Flat Frames"
---

# Tasks: Dark and Flat Frames

**Prerequisites** (plan.md § Prerequisites):
- `030-imaging-camera-link`, `031-imaging-frames-on-console`,
  `032-imaging-solve-and-aim-point` and `033-focus-reading-and-aid` have landed on
  `main`: not met, all four are drafts;
- 031's trio carries the imaging camera's lease (031 US9: `Imaging::lease`,
  `Imaging::holder`, `Lease::set`, `Lease::take`) and `FrameView::set_temp_c` in its
  plan.md as well as its spec.md (plan.md § Prerequisites 11): not met on `main`, where
  no trio of this series is committed yet; met in this worktree, whose 031 plan.md now
  carries both;
- this trio has been re-anchored against `main` after they landed, every plain path of a
  file they create given its line (plan.md § Prerequisites 2): not met;
- 033's draft has been read for the four points of plan.md § Prerequisites 5: met;
- the owner decisions in spec.md are settled and its owner approvals given: not met,
  but for decisions 17 and 23, which the plan document's amendments 2 and 1 settle, and
  7, which the owner settled on 2026-11-17 and which is written in;
- every story has been probe-built: not met;
- the plan document's rewrite of `GLOSSARY.md` is on `main`: not met;
- the scratch-folder helper `crate::support::Scratch` is on `main`: met, at
  `crates/finder/tests/support/mod.rs:12`;
- no operator-staged file is needed: met.

**Rules for every story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml`,
  `CODEOWNERS`, `ergane.yaml`, `.specify/` or `crates/finder/web/playwright.config.ts`.
  If an existing test fails, 030's, 031's, 032's and 033's included, stop and escalate with its
  exact output. Do not edit it. The one exception is the `mod` line of each new finder
  test module, added in alphabetical order to the binary's `main.rs` your story's tasks
  name, which holds no test (spec owner approval 9). Change nothing else there.
- New tests go in new files: the ones your story's tasks name. A new finder test is a
  module file in the binary's folder, never a file at the top level of
  `crates/finder/tests` or in `crates/finder/tests/support`, either of which fails the
  `layout` test. It takes its scratch folder from `crate::support::Scratch`.
- Rigs are written once (spec.md § User Scenarios, **Rigs are written once**). Your
  story's first test task says which helpers your module declares `pub(crate)` and which
  it imports with `use crate::<module>::…`. Declare only what your own tests use, since
  an unused helper fails `clippy -D warnings`, and never edit an earlier story's test
  module to add one: if a helper you need is missing there, escalate.
- Write only the files your story's tasks name; they are its `writes` in plan.md
  § Refinement Record. A story that needs another file escalates.
- Tests compute their expected values themselves (plan.md § Plan traps 6): a master is
  read with a FITS reader of the tests' own and compared with the mean the test takes of
  the saved frames; V, D and the column pattern come from the formulas of spec.md's
  shared setup; masters for `calibrate` are built by hand. Real numbers are compared
  within 0.001 unless the scenario gives another tolerance.
- Keep the story's diff under 64 KiB, measured as plan.md § Plan traps 3 says.
- Every `path:line` was checked at 49bb210 and names code that exists there, but for
  the `GLOSSARY.md` lines, which are of the plan document's rewrite; a file named
  without a line is created by 030, 031, 032 or 033, or by an earlier story of this spec. If
  a cited line has moved, re-read it rather than guess.
- Add no dependency and change no `Cargo.toml` or `Cargo.lock`. Change nothing under
  `crates/scope-node` or `crates/scope-link`, and outside US1 nothing under
  `crates/simulator`. Add no `unsafe`.
- Use the words of `GLOSSARY.md` exactly: imaging frame, sub, dark, flat, master dark,
  master flat, calibration library, bad-pixel map, offset level, flat label, imaging
  angle, lease, purpose label. Never "frame" alone in words the observer reads. Say
  "a set of darks" or "a set of flats", never "calibration set", in every string, key
  and document. Name no camera model, focal length, axis or altitude in code.
- Hold no lock of the calibration module across an await or a call into `Imaging`
  (plan.md § Plan traps 10).
- Take the imaging camera only through 031's lease (plan.md § The camera's lease, § The
  set's lease): a set's request takes `imaging.lease(HOLDER)` last of its checks, and
  its task sets every exposure with `Lease::set`, takes every frame with `Lease::take`,
  puts the exposure back with `Lease::set`, drops the `Lease`, and only then writes the
  final state. Add no lock, flag or method of this spec's own that says a set is
  running, and add nothing to `Imaging` or its lease; if the lease is missing or differs
  from plan.md, escalate.
- Do not edit `specs/034-dark-and-flat-frames/`.
- Run no tool the gates don't run: no `cargo deny`, `cargo machete`, Rust 1.96 or
  `ergane`.

## Phase 1: User Story 1 — A simulated sensor with its own pattern, a cover and a flat panel (Priority: P1)

### Tests first

- [ ] T001 [US1] Create `crates/simulator/tests/imaging_calibration.rs` with the sim rig
  of spec.md § User Scenarios and helpers to read a 16-bit sample at (x, y) of a frame
  and to take a frame of a given exposure, gain and binning. Write failing tests for the
  defaults and the unchanged frame (US1-S1), the cover (US1-S2) and the hot pixels,
  binned ones included (US1-S3).
- [ ] T002 [US1] Add failing tests to `crates/simulator/tests/imaging_calibration.rs` for
  the column pattern (US1-S4), the flat panel with vignetting (US1-S5), and the dust
  shadow, the vignetted sky and the covered panel, each with noise off and gain 0
  (US1-S6), each expected value computed in the test from the formulas of the shared
  setup and checked against the worked numbers the scenario gives.

### Implementation

- [ ] T003 [US1] In `crates/simulator/src/imaging.rs`, add `HotPixel`, `SensorPattern`,
  `FlatPanel`, `DustShadow` and `LightPath` exactly as plan.md § The sensor pattern and
  the light path gives them. In `crates/simulator/src/optics.rs`, add the
  `imaging_bench` cell and the four methods with their asserts, leaving `Pointing` and
  `ImagingOptics` as they are. Export the five names from `crates/simulator/src/lib.rs`.
- [ ] T004 [US1] In `SimImagingCamera::expose` in `crates/simulator/src/imaging.rs`, read
  the pattern and the path at step 1 and add the cover, the panel, the V·D step, step 6b
  and step 8b as plan.md gives them, each skipped at its default, with 030's own
  arithmetic and its order untouched (plan.md § Plan traps 7).
- [ ] T005 [US1] Run the `lint` and `test` gates with every existing test unchanged and
  green, 030's `crates/simulator/tests/imaging.rs` included. Commit the green story
  (US1-S1, US1-S2, US1-S3, US1-S4, US1-S5 and US1-S6).

## Phase 2: User Story 2 — Calibrate a sub on its mosaic (Priority: P1)

### Tests first

- [ ] T006 [US2] Create `crates/finder/tests/console/calibration.rs`, the module
  `calibration` of finder's `console` test binary, and add `mod calibration;` after
  `mod api;` in `crates/finder/tests/console/main.rs`. Write failing tests, as plain
  `#[test]`s over hand-built mosaics, for `Mosaic::from_samples`, the colours and
  `percentile` (US2-S1), the order of dark and flat, with and without a map given
  (US2-S2), `normalise_flat` and a sub divided by it (US2-S3) and `repair` (US2-S4).
- [ ] T007 [US2] Add failing tests to `crates/finder/tests/console/calibration.rs` for
  hot pixels found in the sub (US2-S5), a flat under the floor (US2-S6), a master of the
  wrong length (US2-S7), and one frame from the sim rig with the simulator's patterns,
  calibrated with masters the test builds from the formulas (US2-S8).
- [ ] T008 [US2] Add failing tests to `crates/finder/tests/console/calibration.rs` for
  the third test of `find_hot_pixels`: stars 0.8 and 0.5 samples wide kept, two hot
  pixels that touch found, three left (US2-S9); for `lit_samples` on the five mosaics
  the scenario names (US2-S10); and for one simulated frame at binning 2 with H on the
  simulator and a star the test places with `with_stars` (US2-S11).

### Implementation

- [ ] T009 [US2] Create `crates/finder/src/imaging/calibration.rs` with the seven
  constants, `Mosaic`, `percentile`, `Level`, `levels`, `Masters`, `Report`,
  `SizeMismatch`, `calibrate`, `find_hot_pixels` with its three tests, `lit_samples`,
  `repair` and `normalise_flat`, exactly as plan.md § The arithmetic gives them, and add
  `pub mod calibration;` to `crates/finder/src/imaging/mod.rs`.
- [ ] T010 [US2] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US2-S1, US2-S2, US2-S3, US2-S4, US2-S5, US2-S6, US2-S7,
  US2-S8, US2-S9, US2-S10 and US2-S11).

## Phase 3: User Story 3 — A calibration library on the console's disk (Priority: P1)

### Tests first

- [ ] T011 [US3] Create `crates/finder/tests/console/calibration_library.rs`, the module
  `calibration_library` of `console`, with `mod calibration_library;` after
  `mod calibration;` in `crates/finder/tests/console/main.rs`, and a FITS reader of the
  tests' own for 16-bit and 32-bit real images, declared `pub(crate)` for US6 and US7 to
  import. Write failing tests, as plain `#[test]`s on a scratch folder, for real images
  (US3-S1), the library's files, cards, index and ids (US3-S2), and the matching of
  darks with `FrameKey::agrees` (US3-S3).
- [ ] T012 [US3] Add failing tests to
  `crates/finder/tests/console/calibration_library.rs` for the matching of flats on
  camera, size, origin, bits, pattern and binning alone, whatever their gain, offset and
  set temperature, and the turned-camera rule (US3-S4), `Library::calibrate`, its
  empty-library case repairing the hot sample included (US3-S5), and `remove` and an
  unreadable index (US3-S6).

### Implementation

- [ ] T013 [US3] In `crates/finder/src/imaging/fits.rs`, add `RealImage`, `encode_real`
  and `decode_real` beside 031's functions, reusing its card formatting and editing
  neither `encode` nor `decode` (plan.md § Real images).
- [ ] T014 [US3] Create `crates/finder/src/imaging/calibration/library.rs` with the
  constants, `Kind`, `MasterEntry` with `checked`, `FrameKey` with `from_image`,
  `from_view` (its set temperature from the frame's `set_temp_c`) and `agrees`,
  `Selection`, `Applied` and `Library` (`open`, `error`,
  `masters`, `new_id`, `add` with the `DARKTEST` card, `remove`, `select`, `calibrate`),
  exactly as plan.md § The library gives them, and add `pub mod library;` to
  `crates/finder/src/imaging/calibration.rs`. `select` carries the chosen dark's
  `bad_pixels` from the start; `add` makes no map (plan.md § Plan traps 9).
- [ ] T015 [US3] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US3-S1, US3-S2, US3-S3, US3-S4, US3-S5 and US3-S6).

## Phase 4: User Story 4 — The library in the console (Priority: P1)

### Tests first

- [ ] T016 [US4] Create `crates/finder/tests/console/calibration_console.rs`, the module
  `calibration_console` of `console`, with `mod calibration_console;` after
  `mod calibration;` and before `mod calibration_library;` in
  `crates/finder/tests/console/main.rs`. Give it the console rig of spec.md § User
  Scenarios, a request helper for `DELETE` and a foreign `Origin`, and a wait on a
  route's body, each declared `pub(crate)` for the later stories to import. Write
  failing tests for the two routes, the view, the session and the match (US4-S1), a
  console without imaging, the origin judged first, and one that is not ready (US4-S2),
  and the two frame keys with the frame's `set_temp_c`, and the match that stays
  through a dark, an exposure, a focus frame and a flat (US4-S3).

### Implementation

- [ ] T017 [US4] Create `crates/finder/src/imaging/calibration/service.rs` with
  `CalibrationView`, and `Calibration` with `new` (its third parameter written `_clock`
  and not kept), `library`, `view` with the key last matched, passing over frames whose
  purpose is `dark`, `flat` or 033's `PURPOSE`, `focus`, and the imaging angle read from
  the view's `relation`, and `remove` for masters (plan.md § The console's
  calibration state), and add `pub mod service;` to
  `crates/finder/src/imaging/calibration.rs`. In `crates/finder/src/imaging/mod.rs`, add
  `calibration` to `ImagingView`, and `root` and `attach_calibration` to `Imaging`,
  setting the part only on a `ready` view, from the finished view and outside the
  state's lock. In `crates/finder/src/api.rs`, build the calibration in `with_imaging`
  and add `AppState::calibration`.
- [ ] T018 [US4] In `crates/finder/src/imaging/routes.rs`, add `get_calibration` and
  `delete_calibration` and their two routes, judged in the order plan.md § The routes
  gives. Add the two route lines to `crates/finder/src/lib.rs`, and the two rows and
  `## Calibration frames` to `crates/finder/README.md` (plan.md § The routes,
  § `crates/finder/README.md`).
- [ ] T019 [US4] Run the `lint` and `test` gates with every existing test unchanged and
  green, US3's included. Commit the green story (US4-S1, US4-S2 and US4-S3).

## Phase 5: User Story 5 — A bad-pixel map from each master dark (Priority: P2)

### Tests first

- [ ] T020 [US5] Create `crates/finder/tests/console/calibration_bad_pixels.rs`, the
  module `calibration_bad_pixels` of `console`, with `mod calibration_bad_pixels;`
  after `mod calibration;` in `crates/finder/tests/console/main.rs`. Write failing tests
  for the rule (US5-S1), a column pattern (US5-S2), the 1 percent limit and a sample
  that is not a number (US5-S3), the simulator's hot pixels found in a mean of eight
  covered frames (US5-S4), and the map kept, used and removed by the library, with a
  master only added keeping null (US5-S5).

### Implementation

- [ ] T021 [US5] In `crates/finder/src/imaging/calibration.rs`, add `BAD_PIXEL_SIGMAS`,
  `BAD_PIXEL_MAX_SHARE`, `BadPixels` and `bad_pixel_map`. In
  `crates/finder/src/imaging/calibration/library.rs`, add `BAD_PIXELS_FORMAT` and
  `attach_bad_pixels`, have `calibrate` read and use the map, and have `remove` delete
  its file. `add` makes no map, and `select` is not changed (plan.md § Plan traps 9).
- [ ] T022 [US5] Add the **Bad-pixel map** entry to `GLOSSARY.md` after the
  **Calibration library** entry, with the three lines of spec owner approval 6, and the
  map's paragraph, with the limits of the search in the sub, to `## Calibration frames`
  in `crates/finder/README.md`.
- [ ] T023 [US5] Run the `lint` and `test` gates with every existing test unchanged and
  green, US3's and US4's included. Commit the green story (US5-S1, US5-S2, US5-S3,
  US5-S4 and US5-S5).

## Phase 6: User Story 6 — Take a set of darks (Priority: P1)

### Tests first

- [ ] T024 [US6] Create `crates/finder/tests/console/calibration_darks.rs`, the module
  `calibration_darks` of `console`, with `mod calibration_darks;` after
  `mod calibration_console;` in `crates/finder/tests/console/main.rs`. Import the
  console rig, the request helper and the wait from `crate::calibration_console` and the
  FITS reader from `crate::calibration_library`. Write here, `pub(crate)` for US7, the
  scripted camera of spec.md § User Scenarios, with its level function, its frames'
  carried gain and sensor temperature and its nth `expose` answering `Failed`, and the
  scripted console rig. Write failing tests for a set under the cover, its master and
  `state.imaging().unwrap().holder()` before, during and after (US6-S1), the settings
  carried (US6-S2), the three refusals on the simulated camera (US6-S3) and the exact
  lines and `anyway` on the scripted camera (US6-S4).
- [ ] T025 [US6] Add failing tests to `crates/finder/tests/console/calibration_darks.rs`
  for the route's refusals and their order, `holder()` `None` where the single frame
  answers (US6-S5), stopping a set by `DELETE` and by an aborted exposure, with the
  lease free after (US6-S6), a failed frame, changed settings and a sensor away from its
  set temperature (US6-S7), the match after a set and after a second (US6-S8), and the
  lease: the set's holder name in the refusals of an imaging frame, a settings change and
  a second lease while a dark exposes, and a darks request answered by a lease of the
  test's own, `lease("probe")`, then accepted once it is dropped (US6-S9).

### Implementation

- [ ] T026 [US6] In `crates/finder/src/imaging/calibration/service.rs`, add the six
  darks constants and `HOLDER`, `SetView` with its `skip_serializing_if` attributes,
  `DarksRequest`, the clock kept by `new`, the set's state with its stop flag read at the
  three places plan.md names, the frame helper with its five steps on the set's `Lease`,
  `take_darks` with its six refusals in plan.md's order, the frame-in-flight check only
  while `imaging.holder()` is `None` and `imaging.lease(HOLDER)` last, its refusal
  passed on unchanged, and its task owning the `Lease`, `set` in the view, and the stop
  step of `remove`, exactly as plan.md § The console's calibration state and § The
  set's lease give them: the offset frame first, the three tests with their texts and
  `not_dark`, `anyway`, the master as the mean with `checked`, `library.add` then
  `attach_bad_pixels`, and the ending (`Lease::set` of the exposure, the `Lease`
  dropped) before the final state is written.
- [ ] T027 [US6] In `crates/finder/src/imaging/routes.rs`, add `take_darks` and its
  route, judged in the order plan.md § The routes gives. Add the route's line to
  `crates/finder/src/lib.rs`, its row and paragraph to `crates/finder/README.md`, and
  the **Offset level** entry to `GLOSSARY.md` after the **Flat** entry, with the three
  lines of spec owner approval 6.
- [ ] T028 [US6] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US6-S1, US6-S2, US6-S3, US6-S4, US6-S5, US6-S6, US6-S7,
  US6-S8 and US6-S9).

## Phase 7: User Story 7 — Take a set of flats (Priority: P2)

### Tests first

- [ ] T029 [US7] Create `crates/finder/tests/console/calibration_flats.rs`, the module
  `calibration_flats` of `console`, with `mod calibration_flats;` after
  `mod calibration_darks;` in `crates/finder/tests/console/main.rs`. Import the scripted
  camera and the scripted console rig from `crate::calibration_darks`, the console rig,
  the request helper and the wait from `crate::calibration_console`, and the FITS reader
  from `crate::calibration_library`; a level function that changes when the test says
  the camera is covered is a closure over a flag of the test's own, not a new helper.
  Write failing tests for the two steps on the linear camera (US7-S1), the search's
  other paths and failures, the eighth exposure named (US7-S2), a flat outside the
  window, darks taken while lit and darks taken with `anyway` (US7-S3), and the route's
  refusals and the drop (US7-S4).
- [ ] T030 [US7] Add failing tests to `crates/finder/tests/console/calibration_flats.rs`
  for a master flat of simulated flats against V and D (US7-S5); on 032's solving rig,
  built here from the console rig with `with_imaging_solver`, the imaging angle kept
  from the view, warned of after the camera is turned, and gone after `DELETE /api/aim`
  (US7-S6); and a simulated sky frame calibrated with a master dark and a master flat
  the console made (US7-S7).

### Implementation

- [ ] T031 [US7] In `crates/finder/src/imaging/calibration/service.rs`, add the six flats
  constants, `FlatsRequest`, the flat keys of `SetView`, `take_flats` with its two
  steps, the search exactly as plan.md's pseudocode gives it with its five texts, its
  `last` and its rounding, the flats held with the set in `needs_darks` with the imaging
  angle read from `imaging.view()`'s `relation`, the second step with the three tests
  and `anyway` and its return to `needs_darks` on failure, the master flat through
  `normalise_flat` with `checked`, and the drop step of `remove`. Each step takes
  `imaging.lease(HOLDER)` last of its request's checks and its task owns the `Lease`
  through the ending; the first step's task drops it before writing `needs_darks`, so
  flats that wait hold none (plan.md § The set's lease). Change no key of `SetView` that
  US6's tests read. Have `take_darks` refuse while flats wait, at the fourth place of its
  order.
- [ ] T032 [US7] In `crates/finder/src/imaging/routes.rs`, add `take_flats` and its
  route. Add the route's line to `crates/finder/src/lib.rs`, its row and paragraph to
  `crates/finder/README.md`, and the **Flat label** entry to `GLOSSARY.md` after the
  **Offset level** entry, with the three lines of spec owner approval 6.
- [ ] T033 [US7] Run the `lint` and `test` gates with every existing test unchanged and
  green, `crates/finder/tests/console/calibration_darks.rs` included and not edited.
  Commit the green story (US7-S1, US7-S2, US7-S3, US7-S4, US7-S5, US7-S6 and US7-S7).

## Phase 8: User Story 8 — Three MCP tools (Priority: P2)

### Tests first

- [ ] T034 [US8] Create `crates/finder/tests/mcp/calibration.rs`, the module
  `calibration` of finder's `mcp` test binary, with `mod calibration;` after
  `mod axis_moves;` in `crates/finder/tests/mcp/main.rs`, and 031's stand-in console rig
  with MCP on, written in the module. Write failing tests for the tool list and its
  schemas' keys (US8-S1), `list_calibration` and `take_darks` (US8-S2), the refusals and
  `take_flats` (US8-S3), and a console without imaging and the token (US8-S4).

### Implementation

- [ ] T035 [US8] In `crates/finder/src/mcp/tools.rs`, add `DarksArguments`,
  `FlatsArguments` and the three tools after the imaging tools already there, each
  parsing its arguments with the route's request type so that the words are the same,
  and each answering at once on a console without a calibration (plan.md § The MCP
  tools). Add one line a tool to `## MCP clients` in `crates/finder/README.md`.
- [ ] T036 [US8] Run the `lint` and `test` gates with every existing test unchanged and
  green, `crates/finder/tests/mcp/motion.rs` included. Commit the green story (US8-S1,
  US8-S2, US8-S3 and US8-S4).

## Phase 9: User Story 9 — The Calibration block on the Imaging page (Priority: P2)

### Tests first

- [ ] T037 [US9] Create `crates/finder/web/src/components/CalibrationBlock.test.tsx` with
  failing tests for US9-S1, US9-S2, US9-S3, US9-S4, US9-S5 and US9-S6, stubbing `fetch`
  as the Imaging page's tests do and, for US9-S1 and US9-S6, rendering `ImagingPage`
  with 031's ready view of its US1-S1.

### Implementation

- [ ] T038 [US9] Add `CalibrationMaster`, `CalibrationMatch`, `CalibrationSet`,
  `ImagingCalibration`, the four calls and `calibration?` on `ImagingView` to
  `crates/finder/web/src/api.ts`; create
  `crates/finder/web/src/components/CalibrationBlock.tsx` with the words of plan.md
  § The page, each asserted phrase on one line of JSX and only existing classes, the two
  `anyway` buttons hanging on `not_dark` and never on an error's words; and render it in
  `crates/finder/web/src/components/ImagingPage.tsx` as the page's last block, only when
  the ready view carries `calibration`.
- [ ] T039 [US9] Add the Calibration block entry to `DESIGN.md` after the last Imaging
  page entry and its component to `.impeccable/design.json`. Run the `lint` and `test`
  gates with every existing Vitest and Playwright test unchanged and green, 031's
  `ImagingPage.test.tsx` and `e2e/imaging.spec.ts` included, commit the rebuilt
  `crates/finder/web/dist`, and commit the green story (US9-S1, US9-S2, US9-S3, US9-S4,
  US9-S5 and US9-S6).

## Execution Order

```text
US1 ──► US2 ──► US3 ──► US4 ──► US5 ──► US6 ──► US7 ──┬──► US8
                                                      └──► US9
```

- **US1** comes first: every later oracle is one of the simulator's patterns.
- **US2** adds the arithmetic, and **US3** the library on disk that calls it. US3 adds
  its `pub mod` line to US2's file and its `mod` line after US2's.
- **US4** puts the library in the console: its state, the view and the first two routes.
  It is a story of its own so that neither it nor US3 passes 64 KiB.
- **US5** adds the map to US2's file and US3's library, and its paragraph to the README
  section US4 writes, before the story that needs it.
- **US6** takes darks: it extends US4's state, routes, route list and README section,
  and attaches US5's map.
- **US7** takes flats with US6's runner, dark test and scripted camera.
- **US8** and **US9** both follow US7 and share no file, so they may run together: US8
  writes the MCP tools, their test module and the README; US9 writes only
  `crates/finder/web/`, `DESIGN.md` and `.impeccable/design.json`.
- **`[P]`** marks nothing here: every task shares a file with another in its story.
