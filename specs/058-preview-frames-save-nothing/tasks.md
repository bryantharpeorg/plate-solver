---
description: "Task list for 058-preview-frames-save-nothing: Preview Frames Save Nothing"
---

# Tasks: Preview Frames Save Nothing

**Prerequisites** (plan.md § Prerequisites, each with its evidence there):
- every spec in `depends_on_landed` has landed on `main`: met at e7d6d094;
- 052 to 055 have landed and this spec is re-anchored on them: met at e7d6d094, 055
  having landed at 936b140d;
- the owner decisions in spec.md are answered and the approvals given: met, the
  owner's "Flip 058, all recs" of 2026-11-28 and their yes to approval 9;
- a cynical review of this trio, folded: met, round 1 folded on 2026-11-27;
- every story has been probe-built from this text: met, probe round 1 on `probe/058-r1`
  (tip 0e2b083c: 0 blocking, 11 clarity), folded with its proofs (tip 61da6c7e);
- the page half has run once: met (`CI=1 npm run e2e`, 35 of 35 at US4's tip and the
  final tip, and on probe round 1's fold's tip 61da6c7e), and 39 of 39 on the
  replay onto 936b140d;
- no operator-staged file is needed: met.

**Rules for every story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml`,
  `CODEOWNERS`, `ergane.yaml` or `.specify/`. If an existing test fails, stop and
  escalate with its exact output. A test file an earlier story of this spec created is
  an existing test. The exceptions: each story's `mod` lines in
  `crates/finder/tests/console/main.rs` or `crates/finder/tests/mcp/main.rs`, each in
  its alphabetical place and naming a module of lower-case letters and `_` only, and
  US4's two list entries in `crates/finder/web/playwright.config.ts` (spec.md owner
  approval 6), and US2's one line in 055's test helper `held` in
  `crates/finder/src/nudge.rs` (spec.md owner approval 9).
- Write only the files your story's tasks name; they are its `writes` in plan.md
  § Refinement Record. A story that needs another file escalates.
- `crates/scope-node`, `crates/scope-link`, `proto/`, `crates/simulator`, the plate
  solver's crates and the databases are never touched; in `crates/finder/src/imaging/`,
  only `frames.rs`, `mod.rs`, `preview.rs` and `routes.rs` change, and the focus aid,
  calibration, stack, run, alignment and solving modules are not edited. Nothing is
  added under `crates/finder/tests/support/`.
- Copy the words of spec.md's FRs and of plan.md § Interfaces exactly, the README's and
  the glossary's among them. Say "preview frame" and "imaging frame", never "live view",
  in every message and label.
- No test downloads anything, and no test writes outside `CARGO_TARGET_TMPDIR`. Run no
  tool the gates don't run, and do not set `CARGO_TARGET_DIR`.
- The results said here before each story's implementation are the probe's
  (plan.md § What was run and what was read), where it ran them.
- Read a fixed-size pair of bytes with `as_chunks::<2>()`: the factory's clippy refuses
  `chunks_exact(2)` (plan.md § Plan traps 22).

## Phase 1: User Story 1 - The console takes a preview frame and saves nothing (Priority: P1)

### Tests first

- [ ] T001 [US1] Create `crates/finder/tests/console/imaging_preview_frames.rs` with the
  preview rig of spec.md § User Scenarios (its own `Transport::connector` over a fresh
  `tokio::io::duplex` serving `node.service().routes()`, plan.md § Plan traps 5, the
  node's folder N set through `node.service().with_imaging_save_dir(N)`, which the
  `SimNode`'s service shares, and a `SimulatedSolver` over the simulator), helpers that
  send a request and read its JSON, read the view every 100 ms until a condition holds
  (within 120 s of paused time) and list a folder, one test for each of US1-S1, US1-S2
  and US1-S3 under `#[tokio::test(start_paused = true)]`, and one for US1-S4 under
  `#[test]`, reading ADR 0002 from `../../docs/adr/` beside `env!("CARGO_MANIFEST_DIR")`,
  and add `mod imaging_preview_frames;` to `crates/finder/tests/console/main.rs` after
  `mod imaging_preview;`. On f6d4f5fd's source there is no route
  `/api/imaging/preview`, so the `POST`s answer no JSON and the first three fail (run),
  and f6d4f5fd's ADR has no line of 058, so US1-S4 fails (run, as a mutation).

### Implementation

- [ ] T002 [US1] In `crates/finder/src/imaging/frames.rs`, give `frames::take` the
  argument `keep`, `FrameTask` the field `keep` and its `save` the value
  `keep && settings.save_at_node`; in `follow`, show a frame that is not kept with a new
  `show` instead of saving it; and add `show`, which makes the preview on the blocking
  pool, keeps it among the last `PREVIEWS_KEPT`, sets the view's `preview`, ends the
  frame `shown` and writes nothing; and add `shown` to `FrameView::state`'s doc (FR-002,
  FR-003; plan.md § Interfaces).
- [ ] T003 [US1] In `crates/finder/src/imaging/mod.rs`, add `Imaging::take_preview`,
  give `take_as` the argument `keep` (true from `take` and `Lease::take`), leave
  `Imaging::ended` as it is (`finish` already answers a shown frame through
  `held.ended`), and say "saved or shown" in the docs of `Held::previews` and
  `Imaging::preview`; in `crates/finder/src/imaging/routes.rs`, add
  `POST /api/imaging/preview` after `/api/imaging/frame` and its handler `take_preview`;
  and add ADR 0002's one line after its last, as plan.md § Interfaces gives it (FR-001,
  FR-002, FR-004, FR-005; plan.md § Interfaces).
- [ ] T004 [US1] Run the `lint` and `test` gates with every existing test unchanged and
  green, `imaging_frames.rs`, `imaging_preview.rs`, `imaging_lease.rs`,
  `imaging_focus_aid.rs` and `imaging_solve.rs` among them, then commit the green story
  (FR-001 to FR-005; US1-S1, US1-S2, US1-S3, US1-S4).

## Phase 2: User Story 2 - The brightest part of each preview frame, one pixel a cell (Priority: P1)

### Tests first

- [ ] T005 [US2] Create `crates/finder/tests/console/imaging_preview_crop.rs` with one
  test for US2-S1 under `#[test]`, building the worked frames of spec.md § User
  Scenarios (helpers for a cell and a patch at a place) and listing each crop's pixels
  that are not black, and the crop rig (the preview rig with no node folder and no
  solver) with one test for US2-S2 under `#[tokio::test(start_paused = true)]`, and add
  `mod imaging_preview_crop;` to `crates/finder/tests/console/main.rs` after
  `mod imaging_preview;`. On US1's source the console binary does not build: there is no
  `preview::crop` (run).

### Implementation

- [ ] T006 [US2] In `crates/finder/src/imaging/preview.rs`, add `CROP_SIDE` (200), the
  private `BOX_SIDE` (32) and `brightest`, and `crop` centred on the brightest box, with
  their docs, and say in the module's doc that a preview frame also gets a crop around
  its brightest box, one pixel a cell (FR-006; plan.md § Interfaces).
- [ ] T007 [US2] In `crates/finder/src/imaging/frames.rs`, add `FrameView::crop` after
  `preview`, `None` in `take`, and make `show` encode the crop beside the preview in its
  one blocking step, keep it among the last `PREVIEWS_KEPT` crops and set the view's
  `crop`; in `crates/finder/src/imaging/mod.rs`, add `Held::crops` and `Imaging::crop`;
  in `crates/finder/src/imaging/routes.rs`, add
  `GET /api/imaging/frames/{id}/crop.jpg` after the preview's route and its handler
  `frame_crop`; in `crates/finder/src/nudge.rs`, add `crop: None,` after `file: None,` in
  the test helper `held`'s `FrameView` (`crates/finder/src/nudge.rs:446`) and change
  nothing else there (FR-007, FR-008; spec.md owner approval 9; plan.md § Interfaces).
- [ ] T008 [US2] Run the `lint` and `test` gates with every existing test unchanged and
  green, US1's test file and `imaging_preview.rs` among them, then commit the green
  story (FR-006 to FR-008; US2-S1, US2-S2).

## Phase 3: User Story 3 - MCP and the words (Priority: P2)

### Tests first

- [ ] T009 [US3] Create `crates/finder/tests/mcp/imaging_preview_frames.rs` with its own
  rig, the crop rig served through `crate::support::mcp::app` at `127.0.0.1`, and the
  bare MCP console, one test for US3-S1 under `#[tokio::test(start_paused = true)]` and
  one for US3-S2 under `#[test]`, reading `crates/finder/README.md` from
  `env!("CARGO_MANIFEST_DIR")` as `crates/finder/tests/console/imaging_run_docs.rs`
  does, and `GLOSSARY.md` from `../../` beside it, reading the README's text with
  each run of whitespace as one space, and add `mod imaging_preview_frames;` to
  `crates/finder/tests/mcp/main.rs` after `mod imaging_focus;`. On US2 the tool is unknown and the README has no line
  `The Imaging page's preview frames have one tool:`, so both fail (run).

### Implementation

- [ ] T010 [US3] In `crates/finder/src/mcp/tools.rs`, add `take_preview_frame` after
  `take_imaging_frame`, with FR-009's description and answers, a frame of another id
  answered `the preview frame was overtaken by another imaging frame` (FR-009; plan.md
  § Interfaces).
- [ ] T011 [US3] In `crates/finder/README.md`, add the clause to the imaging camera's
  count, the MCP paragraph after the finder camera focus aid's entries, the two rows
  before the preview's row, the edits to the frame's states and keys, the preview row,
  the preview paragraph and **One holder**, and `## Preview frames` before
  `## Solving imaging frames`; in `GLOSSARY.md`, add **Preview frame** and **Crop**
  before **Sub** and `preview` to the **Purpose label**'s list; each as plan.md
  § Interfaces gives it (FR-010, FR-011).
- [ ] T012 [US3] Run the `lint` and `test` gates with every existing test unchanged and
  green, the earlier stories' test files, `imaging_run_docs.rs`,
  `crates/finder/tests/mcp/finder_focus.rs` and `crates/finder/tests/mcp/imaging.rs`
  among them, then commit the green story (FR-009 to FR-011; US3-S1, US3-S2).

## Phase 4: User Story 4 - The Imaging page shows preview frames (Priority: P2)

### Tests first

- [ ] T013 [US4] Create `crates/finder/web/src/components/PreviewBlock.test.tsx` with one
  test for each of US4-S1, US4-S2, US4-S3, US4-S4, US4-S5 and US4-S7, each `it` with
  its own timeout of 15 000 ms, stubbing `fetch` with the stub console of spec.md § User
  Scenarios as the page's existing tests stub it (one helper that also lets a scenario
  answer the n-th `POST` itself), and an `afterEach` that cleans up, unstubs the globals
  and puts `document.visibilityState` back to `visible`.
- [ ] T014 [US4] Create `crates/finder/web/e2e/imaging-preview.spec.ts` with a test for
  US4-S6, reading the section's picture once the block shows no status (plan.md § Plan
  traps 16), and its own copy of `imaging.spec.ts`'s off-red count, and add
  `"**/imaging-preview.spec.ts"` to the default project's `testIgnore` and the simulate
  project's `testMatch` in `crates/finder/web/playwright.config.ts`, each on a line of
  its own after `"**/imaging.spec.ts"` (spec.md owner approval 6). Before T015 and T016,
  `PreviewBlock.test.tsx` fails to resolve `./PreviewBlock` (run), and the browser test
  runs out its 60 s with no `Start preview frames` on the page (run).

### Implementation

- [ ] T015 [US4] In `crates/finder/web/src/api.ts`, add the state `shown`, `crop?` and
  `postImagingPreview`; create `components/PreviewBlock.tsx` with `POLL_MS` and
  `PreviewBlock`, its modes, its pause while hidden, its loop, ends, words and markup as
  FR-012 to FR-015 and plan.md § Interfaces give them.
- [ ] T016 [US4] In `components/ImagingPage.tsx`, import `PreviewBlock` and render
  `<PreviewBlock />` right after the section `Imaging frame`, inside the same fragment
  (FR-012). Run `npm run build` and commit `crates/finder/web/dist`.
- [ ] T017 [US4] Run the `lint` and `test` gates with every existing test unchanged and
  green, `ImagingPage.test.tsx` and `e2e/imaging.spec.ts` among them, then commit the
  green story (FR-012 to FR-016; US4-S1, US4-S2, US4-S3, US4-S4, US4-S5, US4-S6,
  US4-S7).

## Execution Order

```text
US1 -> US2 -> US3
          \-> US4
```

- **US1** starts from a `main` that holds every spec of `depends_on_landed`, and 052 to
  055.
- **US2** starts once US1 has merged: its crop is made in US1's `show`, and its rig
  takes US1's preview frames.
- **US3** starts once US2 has merged: its tool answers US2's crop, and its README
  describes it.
- **US4** starts once US2 has merged: the page draws US2's crop, and its browser test
  needs US1's route. US3 and US4 write no file in common and may run side by side.
- **`[P]`** marks nothing here: each story's implementation follows its tests.
