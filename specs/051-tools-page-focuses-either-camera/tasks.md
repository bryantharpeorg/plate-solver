---
description: "Task list for 051-tools-page-focuses-either-camera: The Tools Page Focuses Either Camera"
---

# Tasks: The Tools Page Focuses Either Camera

**Prerequisites** (plan.md § Prerequisites, each with its evidence there):
- every spec in `depends_on_landed` has landed on `main`: met at c75a6e76;
- the owner decisions in spec.md are settled: not met, all ten are open;
- every story has been probe-built from this text: not met; the probe of the draft and
  of its fold built the stories before the text;
- the page half has run once: met on the probe (`CI=1 npm run e2e`);
- no operator-staged file is needed: met;
- the real optic's night series: only if the owner picks decision 10 (a), before the
  flip; under (b), recommended, after the landing.

**Rules for every story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml`,
  `CODEOWNERS`, `ergane.yaml` or `.specify/`. If an existing test fails, stop and
  escalate with its exact output. A test file an earlier story of this spec created is
  an existing test. The exceptions: each story's `mod` lines in
  `crates/finder/tests/console/main.rs` or `crates/finder/tests/mcp/main.rs`, each in
  its alphabetical place, and US4's two list entries in
  `crates/finder/web/playwright.config.ts` (spec owner approval 6).
- Write only the files your story's tasks name; they are its `writes` in plan.md
  § Refinement Record. A story that needs another file escalates.
- `crates/finder/src/imaging/` (033's `focus.rs` and `focus/` among it),
  `crates/finder/src/recording.rs`, `crates/finder/src/link.rs`,
  `crates/finder/src/live.rs`, `crates/finder/src/session.rs`, `crates/scope-node`,
  `crates/scope-link`, `proto/`, the plate solver's crates, the databases,
  `crates/finder/web/src/components/FocusBlock.tsx` and `AlignmentBlock.tsx` are never
  touched, and nothing is added under `crates/finder/tests/support/`. In
  `crates/simulator`, only US1's three files change.
- Copy the words of spec.md's FRs and of plan.md § Interfaces exactly, the README's and
  the glossary's among them. Say "finder camera", never "finder" alone, in every
  message and label.
- No test downloads anything, and no test writes outside `CARGO_TARGET_TMPDIR`. Run no
  tool the gates don't run, and do not set `CARGO_TARGET_DIR`.
- The results said here before each story's implementation are the probe's
  (plan.md § What was run and what was read), where it ran them.

## Phase 1: User Story 1 - The console reads the finder camera's focus, and the simulator turns it (Priority: P1)

### Tests first

- [ ] T001 [US1] Create `crates/finder/tests/console/finder_focus_measure.rs` with one
  test for each of US1-S1, US1-S2, US1-S3, US1-S5 and US1-S6 under `#[test]`, and one
  for US1-S4 under `#[tokio::test(start_paused = true)]`, drawing the frames (each
  pixel the integral of the star over its square) and the simulated frames as spec.md
  § User Scenarios defines them, with the oracle 2·√(2 ln 2)·σ and the hash
  `0xb1d308b97a09eb40` written in the test, and add `mod finder_focus_measure;` to
  `crates/finder/tests/console/main.rs` in its alphabetical place. Before T002 and T003
  the binary does not build (no `finder::finder_focus`, no
  `Simulator::set_finder_focus_error`, no `render::render_with_sigma`; run).

### Implementation

- [ ] T002 [US1] In `crates/simulator/src/optics.rs`, add `Pointing`'s
  `finder_focus_error`, 0 in `Simulator::new`, and `Simulator::finder_focus_error` and
  `Simulator::set_finder_focus_error`; in `crates/simulator/src/render.rs`, add
  `render_with_sigma` and make `render` call it at `PSF_SIGMA_PX`; in
  `crates/simulator/src/camera.rs`, draw `SimCamera`'s frames with the focus error's
  width, each as plan.md § Interfaces gives it (FR-001).
- [ ] T003 [US1] Create `crates/finder/src/finder_focus.rs` with its module doc,
  `STARS_MEASURED`, `BOX_PX`, `SATURATED`, `THRESHOLD_NOISE`, `SUBPIXELS`,
  `APERTURE_MARGIN_PX` and `measure`, each star's half-flux diameter summed in
  sub-pixel circles as FR-002 gives it, saturation tested on the star's own pixels
  (FR-002; plan.md § Interfaces), and add `pub mod finder_focus;` to
  `crates/finder/src/lib.rs` after `pub mod ephemeris;`.
- [ ] T004 [US1] Run the `lint` and `test` gates with every existing test unchanged and
  green, then commit the green story (FR-001 to FR-003; US1-S1, US1-S2, US1-S3, US1-S4,
  US1-S5, US1-S6).

## Phase 2: User Story 2 - The finder camera's focus aid runs beside the live solve (Priority: P1)

### Tests first

- [ ] T005 [US2] Create `crates/finder/tests/console/finder_focus_aid.rs` with the focus
  rig of spec.md § User Scenarios, its session's `camera_settings` fixed at
  `{"exposure_ms": 500, "gain": 1}`, its
  `pub struct Rig { pub app: Router, pub sim: Simulator, pub node: Arc<SimNode> }` and
  `pub fn rig() -> Rig`, helpers that send a request and read its JSON, wait until a
  condition holds on `GET /api/finder-camera/focus` (every 100 ms, within 120 s of
  paused time), wait for live, and read the settled reading after n, and one test for
  each of US2-S1, US2-S2, US2-S3, US2-S4 and US2-S5 under
  `#[tokio::test(start_paused = true)]`, and add `mod finder_focus_aid;` to
  `crates/finder/tests/console/main.rs` in its alphabetical place. On US1 there is no
  route `/api/finder-camera/focus`, so all five fail: the `GET` answers no
  `{"focus": null}`, and the `POST`s' empty answers fail the JSON read (run).

### Implementation

- [ ] T006 [US2] In `crates/finder/src/finder_focus.rs`, add `RAW_WAIT`, `LastView`,
  `MedianView`, `FocusView`, `FocusAsk`, `Refusal`, `FinderFocus` with `view`, `start`
  (refusing a running aid, a missing node and an automatic exposure or gain, in that
  order) and `stop`, the run's task and loop over `LinkManager::raw` and
  `LinkManager::frames`, `last.detected` the paired star frame's centroid count, the
  run ended `failed` when a paired star frame's exposure or gain differs from the
  first's and when a measure panics, each measure in `spawn_blocking` and pushed to one
  033 `Aid`, and `routes()`, the `POST` starting under the session's camera settings
  (FR-004 to FR-008; plan.md § Interfaces).
- [ ] T007 [US2] In `crates/finder/src/api.rs`, add `AppState`'s `finder_focus` field,
  `Arc::default()` in `AppState::shared`, `AppState::finder_focus`, and merge
  `crate::finder_focus::routes()` after the imaging routes in `router` (FR-004).
- [ ] T008 [US2] Run the `lint` and `test` gates with every existing test unchanged and
  green, US1's test file among them, then commit the green story (FR-004 to FR-009;
  US2-S1, US2-S2, US2-S3, US2-S4, US2-S5).

## Phase 3: User Story 3 - MCP and the README carry the finder camera's focus aid (Priority: P2)

### Tests first

- [ ] T009 [US3] Create `crates/finder/tests/mcp/finder_focus.rs` with its own rig, the
  focus rig served with `/mcp` as the MCP binary's other tests serve theirs, its
  session's `camera_settings` fixed at `{"exposure_ms": 500, "gain": 1}`, and the
  bare console, one test for US3-S1 under `#[tokio::test(start_paused = true)]` and one
  for US3-S2 under `#[test]`, reading `crates/finder/README.md` from
  `env!("CARGO_MANIFEST_DIR")` as `crates/finder/tests/console/imaging_run_docs.rs`
  does, and add `mod finder_focus;` to `crates/finder/tests/mcp/main.rs` in its
  alphabetical place. On US2 the tools are unknown and the README has no line
  `The Tools page's focus aid for the finder camera has two tools:`, so both fail (run).

### Implementation

- [ ] T010 [US3] In `crates/finder/src/mcp/tools.rs`, add `FinderFocusArguments`,
  `get_finder_focus` and `set_finder_focus` with plan.md § Interfaces' descriptions and
  checks, `set_finder_focus` starting under the session's camera settings as the route
  does (FR-010).
- [ ] T011 [US3] In `crates/finder/README.md`, add the MCP paragraph after the finder
  alignment's, the three rows after `DELETE /api/imaging/align`'s and
  `## Focusing the finder camera` before `## Calibration frames`; in `GLOSSARY.md`,
  change **Raw frame**, **Focus reading** and **Focus aid**, each as plan.md
  § Interfaces gives it (FR-011).
- [ ] T012 [US3] Run the `lint` and `test` gates with every existing test unchanged and
  green, the earlier stories' test files, `imaging_run_docs.rs` and
  `crates/finder/tests/mcp/imaging_align.rs` among them, then commit the green story
  (FR-010 to FR-012; US3-S1, US3-S2).

## Phase 4: User Story 4 - The Tools page's Focus block picks the camera (Priority: P2)

### Tests first

- [ ] T013 [US4] Create `crates/finder/web/src/components/FocusTool.test.tsx` with one
  test for each of US4-S1, US4-S2, US4-S3 and US4-S4, and
  `crates/finder/web/src/components/FocusToolPage.test.tsx` with one test for US4-S5,
  stubbing `fetch` as the page's existing tests do.
- [ ] T014 [US4] Create `crates/finder/web/e2e/tools-focus.spec.ts` with a test for
  US4-S6 and its own copy of `tools.spec.ts`'s off-red count, which first sends
  `PUT /api/session` with `{"camera_settings": {"exposure_ms": 200, "gain": 1}}` through
  `page.request` and in its `finally` sends `DELETE /api/finder-camera/focus` through
  `page.request`, and add `"**/tools-focus.spec.ts"` to the default project's `testIgnore`
  and the simulate project's `testMatch` in `crates/finder/web/playwright.config.ts`,
  each on a line of its own after `"**/tools.spec.ts"` (spec owner approval 6). Before
  T015 and T016, `FocusTool.test.tsx` fails to resolve `./FocusTool` and US4-S5 finds
  no region `Focus a camera` (run); US4-S6 times out on the button
  `Start the finder camera's focus aid` (read, not run).

### Implementation

- [ ] T015 [US4] In `crates/finder/web/src/`, add `FinderFocus`, `getFinderFocus`,
  `postFinderFocus` and `deleteFinderFocus` to `api.ts`, and create
  `components/FocusTool.tsx` with `finderStatus` and `FocusTool`: 033's words with
  "finder camera frames", the `too_few_stars` hint chosen by `last.detected`, the
  status only while running or after a stop, and `Stars detected <n>` under it
  (FR-013 to FR-015; plan.md § Interfaces).
- [ ] T016 [US4] In `components/ToolsPage.tsx`, render
  `<FocusTool view={view} refresh={refresh} />` after the alignment's block (FR-016).
  Run `npm run build` and commit `crates/finder/web/dist`.
- [ ] T017 [US4] Run the `lint` and `test` gates with every existing test unchanged and
  green, `FocusBlock.test.tsx`, `ToolsPage.test.tsx` and `e2e/tools.spec.ts` among them,
  then commit the green story (FR-013 to FR-017; US4-S1, US4-S2, US4-S3, US4-S4, US4-S5,
  US4-S6).

## Execution Order

```text
US1 -> US2 -> US3 -> US4
```

- **US1** starts from a `main` that holds every spec of `depends_on_landed`.
- **US2** starts once US1 has merged: its run calls US1's `measure`, and its tests turn
  US1's focuser.
- **US3** starts once US2 has merged: its tools start and stop US2's run.
- **US4** starts once US3 has merged: the page reads US2's view, and its browser test
  needs US2's routes.
- **`[P]`** marks nothing here: each story's implementation follows its tests.
