---
description: "Task list for 031-imaging-frames-on-console: Imaging Frames on the Console"
---

# Tasks: Imaging Frames on the Console

**Prerequisites** (plan.md § Prerequisites):
- `025-frame-capture` and `028-finder-tests-do-less` have landed on `main`: met (028
  completely at 49bb210); `030-imaging-camera-link` has landed, all seven of its
  stories: not met;
- this trio has been re-anchored to `main` after 030 landed, with every name of plan.md
  § What this trio takes from 030 read there: not met (the citations of files that
  exist today are met at 49bb210; plan.md § Prerequisites 2 lists the two corrected);
- the owner decisions in spec.md, sixteen of them, are settled and its owner approvals
  given: not met (3, 4, 8 and the region of 16 are settled by the plan document's
  amendments; twelve are open);
- the six stories are probe-built, on top of 030's probe builds, and the trio refined
  from what they find: not met;
- the plan document, ADR-0002, the changed `GLOSSARY.md` (with **Preview** widened) and
  the research note are on `main`: not met (all four are in this worktree, uncommitted);
- no other draft that writes the same files (`026-mount-sky`, `021-chart-orientation`,
  `022-altaz-guidance`) is flipped beside this one without one naming the other in
  `depends_on_landed`: not met;
- the plan document's § Interfaces shared across the specs agrees with the settled
  decisions: met on this side of the link (it now also carries 030's `save` on
  `ExposeRequest`, whose owner decision 9 is still open);
- the camera's lease carries the names 033 to 037 call: met (US6; plan.md
  § Prerequisites 10).

**Rules for every story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml`,
  `CODEOWNERS`, `ergane.yaml` or `.specify/`. If an existing test fails, stop and
  escalate with its exact output. Do not edit it. The only exceptions are the ones
  spec.md owner approval 8 names: the `mod` line of each new finder test module, added
  in alphabetical order to the `main.rs` your story's tasks name, and US4's two list
  entries in `crates/finder/web/playwright.config.ts`, with the simulate project's
  `testMatch` list written one entry a line as Prettier then requires. Change nothing
  else there.
- New tests go in new files: the ones your story's tasks name. A new finder test is a
  module file in its binary's folder, never a file at the top level of
  `crates/finder/tests` or in `crates/finder/tests/support`. It holds only the setup its
  own tests use, since an unused helper fails `clippy -D warnings`.
- Keep a test's files in a scratch folder as spec.md's shared setup says (**Scratch
  folders**): `crate::support::Scratch` in a finder test module. Never build a path
  from `CARGO_MANIFEST_DIR` and `../../target`.
- Write only the files your story's tasks name; they are its `writes` in plan.md
  § Refinement Record. A story that needs another file escalates. Write nothing under
  `crates/scope-node`, `crates/scope-link` or `crates/simulator`: those are 030's. A
  story that finds one of 030's interfaces different on `main` from plan.md § What this
  trio takes from 030 escalates rather than changing either side.
- Tests compute their expected values themselves (plan.md § Plan traps 6): the sample
  rule, the worked examples, FITS files read with the test's own code. Tolerances are
  the ones a scenario states and no wider.
- Compare an imaging view's `settings` and `frame` key by key, never whole, and end
  every scope-link message literal and every `ImagingFrame` literal with
  `..Default::default()` (plan.md § Plan traps 9).
- No `unsafe`. Add no dependency and change no `Cargo.toml` or `Cargo.lock`.
- Name no camera model in product code: sizes, ranges, the Bayer pattern and the cooler
  come from the camera. Use `GLOSSARY.md`'s words: imaging frame (never "frame" alone
  in a message or on the page), imaging camera, scope node, console.
- Keep the story's diff under 64 KiB, measured as plan.md § Plan traps 3 says. A story
  that cannot escalates; it does not drop a scenario.
- Every code citation is `path:line` as read at 49bb210. If a cited line has moved,
  re-read it rather than guess. A file this spec or 030 creates is named without a
  line, with the story that creates it.
- A time a scenario states is exact.
- Do not edit `specs/031-imaging-frames-on-console/` or
  `specs/030-imaging-camera-link/`.
- Run no tool the gates don't run: no `cargo deny`, `cargo machete`, Rust 1.96 or
  `ergane`.

## Phase 1: User Story 1 — The console sees the imaging camera (Priority: P1)


### Tests first

- [ ] T001 [US1] Create `crates/finder/tests/console/imaging.rs`, the module `imaging` of
  `console`, with `mod imaging;` after `mod freshness;` in
  `crates/finder/tests/console/main.rs`, the console rig of spec.md § User Scenarios,
  its scratch folder from `crate::support::Scratch`,
  and failing tests for US1-S1, US1-S2, US1-S3, US1-S4, US1-S5, US1-S6, US1-S8, US1-S9 and US1-S10. Create
  `crates/finder/tests/cli/imaging.rs`, the module `imaging` of `cli`, with
  `mod imaging;` after `mod flags;` in `crates/finder/tests/cli/main.rs`, and a failing
  test for US1-S7.

### Implementation

- [ ] T002 [US1] In `crates/finder/src/link.rs`, add `ImagingClient`, keep the session
  and transport in `LinkManager`, add `connect_imaging` and make `Transport::connect`
  `pub(crate)`, changing neither `start`'s signature nor `run`'s parameters.
- [ ] T003 [US1] Create `crates/finder/src/imaging/mod.rs` with the views,
  `SettingsEdit`, `RoiEdit`, `Refusal`, `Imaging::start`, `view` and `set` and the task of plan.md
  § The console's imaging state, which waits on the link's `connected` flag alone and
  never on a change of the whole status, and `crates/finder/src/imaging/routes.rs` with
  `GET /api/imaging` and `PUT /api/imaging/settings`.
- [ ] T004 [US1] In `crates/finder/src/api.rs`, add `with_imaging`, `imaging`, the
  session's `imaging` field, the merged routes and the two visibilities. In
  `crates/finder/src/main.rs`, add `--imaging-dir` and start imaging. Add
  `pub mod imaging;` and the two route lines to `crates/finder/src/lib.rs`, and
  `## Imaging frames` and the two rows to `crates/finder/README.md`.
- [ ] T005 [US1] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US1-S1, US1-S2, US1-S3, US1-S4, US1-S5, US1-S6, US1-S7, US1-S8, US1-S9 and US1-S10).

## Phase 2: User Story 2 — Take an imaging frame and keep it as FITS (Priority: P1)


### Tests first

- [ ] T006 [US2] Create `crates/finder/tests/console/imaging_frames.rs`, the module
  `imaging_frames` of `console`, with `mod imaging_frames;` after `mod imaging;` in
  `crates/finder/tests/console/main.rs`, a FITS reader of its own and failing tests for
  US2-S1, US2-S2, US2-S3 and US2-S4.
- [ ] T007 [US2] Add failing tests to that module for US2-S5, US2-S6, US2-S7, US2-S9,
  US2-S10, US2-S11 and US2-S12, the last on a node rig given `with_imaging_save_dir` and
  reading both files with the module's own FITS reader. Create
  `crates/finder/tests/cli/imaging_frames.rs`, the module `imaging_frames` of `cli`,
  with `mod imaging_frames;` after `mod imaging;` in `crates/finder/tests/cli/main.rs`,
  and a failing test for US2-S8.

### Implementation

- [ ] T008 [US2] Create `crates/finder/src/imaging/fits.rs` with `CardValue`,
  `FitsImage`, `FitsError` and its `Display`, `encode`, `decode` with its checks and
  texts in their order, and `utc` (plan.md § FITS at the console).
- [ ] T009 [US2] Create `crates/finder/src/imaging/frames.rs` with `FrameView` and its
  `roi`, `set_temp_c`, `received_at` and `saved_at`, the receiving task, which replaces
  `roi` with the header's when it has one, the id rule, the region and set temperature
  `take` puts in the frame and the serial it keeps for the cards, and the save that
  refuses a name already taken; and add `take`, `abort` and `ended` to
  `crates/finder/src/imaging/mod.rs` (plan.md § Receiving and saving, § FITS at the
  console).
- [ ] T010 [US2] Add `POST /api/imaging/frame` and `POST /api/imaging/abort` to
  `crates/finder/src/imaging/routes.rs`, their lines to `crates/finder/src/lib.rs`, and
  their rows, the `frame` object, the path, the cards and how to read the time a frame
  took to reach the console to `crates/finder/README.md`.
- [ ] T011 [US2] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US2-S1, US2-S2, US2-S3, US2-S4, US2-S5, US2-S6, US2-S7, US2-S8, US2-S9, US2-S10, US2-S11 and US2-S12).

## Phase 3: User Story 3 — A colour preview of each imaging frame (Priority: P2)


### Tests first

- [ ] T012 [US3] Create `crates/finder/tests/console/imaging_preview.rs`, the module
  `imaging_preview` of `console`, with `mod imaging_preview;` after
  `mod imaging_frames;` in `crates/finder/tests/console/main.rs`, and failing tests for
  US3-S1, US3-S2, US3-S3 and US3-S4.

### Implementation

- [ ] T013 [US3] Create `crates/finder/src/imaging/preview.rs` with `rgb` and `jpeg`
  (plan.md § The preview, steps 1 to 4). In `crates/finder/src/imaging/frames.rs` and
  `crates/finder/src/imaging/mod.rs`, make the preview when a frame is saved, keep the
  last eight, set `frame.preview` and add `Imaging::preview`.
- [ ] T014 [US3] Add `GET /api/imaging/frames/{id}/preview.jpg` to
  `crates/finder/src/imaging/routes.rs`, its line to `crates/finder/src/lib.rs` and its
  row and paragraph to `crates/finder/README.md`. Run the `lint` and `test` gates with
  every existing test unchanged and green. Commit the green story (US3-S1, US3-S2, US3-S3 and US3-S4).

## Phase 4: User Story 4 — The Imaging page, red at night with colour while held (Priority: P2)


### Tests first

- [ ] T015 [US4] Create `crates/finder/web/src/components/ImagingPage.test.tsx` with
  failing tests for US4-S1, US4-S2, US4-S3, US4-S4, US4-S5 and US4-S6, stubbing `fetch` and `EventSource` as the
  existing page tests do, using fake timers for the hold limit and the reads, and
  rendering `useColourHold` with `renderHook` for US4-S5's hook checks.
- [ ] T016 [US4] Create `crates/finder/web/e2e/imaging.spec.ts` with an off-red counter
  of its own and failing tests for US4-S7 and US4-S8, and add `"**/imaging.spec.ts"`
  to the default project's `testIgnore` and the simulate project's `testMatch` in
  `crates/finder/web/playwright.config.ts`, the `testMatch` list written one entry a
  line as `npm run format` leaves it, changing nothing else there.

### Implementation

- [ ] T017 [US4] Add the imaging types and calls to `crates/finder/web/src/api.ts`,
  create `crates/finder/web/src/useImaging.ts`, `crates/finder/web/src/useColourHold.ts`
  (`COLOUR_HOLD_MS`, `ColourHoldBind`, `useColourHold`, exactly as plan.md § The page
  gives them) and `crates/finder/web/src/components/ImagingPage.tsx` with the words of
  plan.md § The page, its picture's control taken from `useColourHold`, add the
  `/imaging` route to `crates/finder/web/src/App.tsx` and the `Imaging` link to
  `crates/finder/web/src/components/TopBar.tsx`.
- [ ] T018 [US4] Make the four `DESIGN.md` edits with the words of spec.md owner
  approval 6, the exception naming an imaging frame's preview and a stack's picture,
  and plan.md § `DESIGN.md`, add the Imaging page to
  `.impeccable/design.json` under the class `ds-imaging-page`, and a sentence to
  `## Night mode` in
  `crates/finder/README.md`. Run the `lint` and `test` gates with every existing Vitest
  and Playwright test unchanged and green, commit the rebuilt `crates/finder/web/dist`,
  and commit the green story (US4-S1, US4-S2, US4-S3, US4-S4, US4-S5, US4-S6, US4-S7 and US4-S8).

## Phase 5: User Story 5 — Three MCP tools (Priority: P2)


### Tests first

- [ ] T019 [US5] Create `crates/finder/tests/mcp/imaging.rs`, the module `imaging` of
  `mcp`, with `mod imaging;` after `mod frames;` in `crates/finder/tests/mcp/main.rs`,
  and failing tests for US5-S1, US5-S2, US5-S3, US5-S4, US5-S5 and US5-S6.

### Implementation

- [ ] T020 [US5] Add `ImagingSettingsArguments` and `ImagingRoiArguments`, and
  `get_imaging_camera`, `set_imaging_camera` with its input schema and
  `take_imaging_frame`, to `crates/finder/src/mcp/tools.rs` with the descriptions,
  errors and image block of plan.md § The MCP tools, and list the tools in
  `## MCP clients` in `crates/finder/README.md`.
- [ ] T021 [US5] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US5-S1, US5-S2, US5-S3, US5-S4, US5-S5 and US5-S6).

## Phase 6: User Story 6 — One holder of the imaging camera (Priority: P2)


### Tests first

- [ ] T022 [US6] Create `crates/finder/tests/console/imaging_lease.rs`, the module
  `imaging_lease` of `console`, with `mod imaging_lease;` after `mod imaging_frames;`
  and before `mod imaging_preview;` in `crates/finder/tests/console/main.rs`, the
  stand-in console rig and the console of US1-S4 whose connector always fails, built as
  spec.md § User Scenarios builds them, its scratch folder from
  `crate::support::Scratch`, the MCP door through `crate::support::mcp`'s `app`, `call`,
  `send` and `body`, and failing tests for US6-S1, US6-S2, US6-S3, US6-S4, US6-S5
  and US6-S6.

### Implementation

- [ ] T023 [US6] In `crates/finder/src/imaging/mod.rs`, add `Lease` with its `Weak` and
  grant, `Imaging::lease`, `Imaging::holder`, `Lease::set`, `Lease::take` and `Drop for
  Lease`, the state's `lease` and `next_grant`, and turn `set` and `take` into wrappers
  of `set_as` and `take_as`, whose lease check sits after the state's and before the
  frame-in-flight check under the same lock (plan.md § The camera lease). Change nothing
  in `crates/finder/src/imaging/routes.rs` or `crates/finder/src/mcp/tools.rs`.
- [ ] T024 [US6] Add the one-holder paragraph, with the words of plan.md § The README,
  at the end of `## Imaging frames` in `crates/finder/README.md`.
- [ ] T025 [US6] Run the `lint` and `test` gates with every existing test unchanged and
  green, the tests of US1, US2 and US5 among them. Commit the green story (US6-S1,
  US6-S2, US6-S3, US6-S4, US6-S5 and US6-S6).

## Execution Order

```text
US1 ──► US2 ──► US3 ──► US4 ──► US5 ──► US6
```

- **None starts before `030-imaging-camera-link` has landed**, all seven of its stories:
  every story here calls its service or builds its cameras.
- **US1, US2, US3** are the console's own part, in that order: its state and settings,
  then imaging frames and their files, then the preview.
- **US4** follows US3, whose preview the page shows; **US5** follows US4, which orders
  their edits of `crates/finder/README.md`.
- **US6** follows US3 and US5, so it is the last writer of
  `crates/finder/src/imaging/mod.rs`, `crates/finder/tests/console/main.rs` and
  `crates/finder/README.md`.
- **`[P]`** marks nothing here: every task shares a file with another in its story.
