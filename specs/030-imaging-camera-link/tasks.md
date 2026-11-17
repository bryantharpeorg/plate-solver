---
description: "Task list for 030-imaging-camera-link: The Imaging Camera on the Scope Link"
---

# Tasks: The Imaging Camera on the Scope Link

**Prerequisites** (plan.md § Prerequisites):
- `025-frame-capture` and `028-finder-tests-do-less` have landed on `main`: met (028
  completely at 49bb210);
- this trio has been re-anchored to `main`: met at 49bb210 (plan.md § Prerequisites 2
  lists the two citations corrected); done again if `main` moves before the flip;
- the owner decisions in spec.md, thirteen of them, are settled and its owner
  approvals given: not met (3, 8, 12 and the white balance of 13 are settled by the plan
  document's amendments; nine are open);
- the saved capture `fixtures/scope-node/indi/ccd-simulator.script` and its
  `ccd-simulator.expect.json` are recorded and committed by the owner: not met;
- the seven stories are probe-built, US2 and US4 first, and the trio refined from what
  they find: not met;
- the plan document, ADR-0002, the changed `GLOSSARY.md` (with **Purpose label** added,
  `probe` and `run` among its labels) and the research note are on `main`: not met
  (all four are in the drafting worktree, uncommitted);
- the plan document's § Interfaces shared across the specs agrees with the settled
  decisions: not met on `main`, where the plan document is not yet; met in the drafting
  worktree, where it carries `save` on `ExposeRequest` and `[imaging] save_dir` as this
  trio has them (owner decision 9, still open).

**Rules for every story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml`,
  `CODEOWNERS`, `ergane.yaml`, `.specify/` or `playwright.config.ts`. If an existing
  test fails, stop and escalate with its exact output. Do not edit it. This trio names
  no exception: every test it adds is a new file.
- New tests go in new files: the ones your story's tasks name, each a file of its own
  under `crates/scope-node/tests` or `crates/simulator/tests`. A test file holds only
  the setup its own tests use, since an unused helper fails `clippy -D warnings`; the
  one module two test files share, `crates/scope-node/tests/support/mod.rs`, starts
  `#![allow(dead_code)]` for that reason.
- Keep a test's files in a scratch folder as spec.md's shared setup says (**Scratch
  folders**): a drop guard of the file's own under `CARGO_TARGET_TMPDIR`. Never build a
  path from `CARGO_MANIFEST_DIR` and `../../target`.
- Write only the files your story's tasks name; they are its `writes` in plan.md
  § Refinement Record. A story that needs another file escalates.
- Tests compute their expected values themselves (plan.md § Plan traps 6): the sample
  rule, the worked examples, FITS files read with the test's own code, INDI through the
  stand-in server. Tolerances are the ones a scenario states and no wider.
- End every scope-link message literal and every `ImagingFrame` literal with
  `..Default::default()` (plan.md § Plan traps 15).
- No `unsafe`. Add no dependency and change no `Cargo.toml` or `Cargo.lock`, except US4,
  which adds `quick-xml`, and US5, which adds `base64`, each exactly as spec.md owner
  approval 3 gives it. A crate is declared only by the story whose code calls it: the
  merge queue refuses an unused dependency (plan.md § Plan traps 19).
- Name no camera model in product code: sizes, ranges, the Bayer pattern and the cooler
  come from the camera. Use `GLOSSARY.md`'s words: imaging frame (never "frame" alone
  in a message), imaging camera, camera port, camera adapter, scope node, console.
- Keep the story's diff under 64 KiB, measured as plan.md § Plan traps 3 says. A story
  that cannot escalates; it does not drop a scenario.
- Every code citation is `path:line` as read at 49bb210. If a cited line has moved,
  re-read it rather than guess. A file this spec creates is named without a line, with
  the story that creates it.
- A time a scenario states is exact. An INDI scenario's time is counted from the call
  it names on a camera that is already connected, unless it says it times the first
  call (plan.md § Plan traps 20).
- Do not edit `specs/030-imaging-camera-link/`.
- Run no tool the gates don't run: no `cargo deny`, `cargo machete`, Rust 1.96, INDI
  program or `ergane`.

## Phase 1: User Story 1 — The imaging side of the camera port (Priority: P1)

### Tests first

- [ ] T001 [US1] Create `crates/scope-node/tests/imaging_port.rs` with the test clock of
  spec.md § User Scenarios and failing tests for US1-S1, US1-S2, US1-S3, US1-S4, US1-S5, US1-S6 and US1-S7, each under
  `#[tokio::test(start_paused = true)]`, computing the sample rule itself.

### Implementation

- [ ] T002 [US1] Create `crates/scope-node/src/imaging/mod.rs` with `BayerPattern`,
  `Range`, `Roi` (deriving `Default`), `ImagingInfo`, `ImagingSettings`,
  `ImagingStatus`, `Exposure`, `ImagingFrame` (deriving `Default`, with its `roi`),
  `ImagingError`, `ImagingFuture`, the trait `ImagingCamera`, `check_settings` and
  `check_exposure`, with the rules, order and texts of plan.md § The port, and add
  `pub mod imaging;` to `crates/scope-node/src/lib.rs`.
- [ ] T003 [US1] Create `crates/scope-node/src/imaging/synthetic.rs` with
  `SyntheticImagingCamera` as plan.md § The stand-in camera gives it: the description,
  the settings in force, the sample rule, the cooler's rule, a frame's `roi` the region
  in force, and an exposure that a guard always ends.
- [ ] T004 [US1] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US1-S1, US1-S2, US1-S3, US1-S4, US1-S5, US1-S6 and US1-S7).

## Phase 2: User Story 2 — A simulated imaging camera along the main telescope's axis (Priority: P1)

### Tests first

- [ ] T005 [US2] Create `crates/simulator/tests/imaging.rs` with the sim rig and the
  centroid and second-moment helpers of spec.md § User Scenarios, and failing tests for
  US2-S1, US2-S2, US2-S3, US2-S4 and US2-S5.
- [ ] T006 [US2] Add failing tests to `crates/simulator/tests/imaging.rs` for US2-S6, US2-S7, US2-S8 and US2-S9.

### Implementation

- [ ] T007 [US2] In `crates/simulator/src/optics.rs`, add the imaging optics to the
  state and `imaging_optics`, `set_imaging_optics`, `imaging_truth`, `imaging_truth_at`
  and `imaging_rotation_at` (plan.md § The simulated imaging camera, **The attitude**).
  Make `SplitMix64::uniform` `pub` in `crates/simulator/src/noise.rs`.
- [ ] T008 [US2] Create `crates/simulator/src/imaging.rs` with the constants,
  `ImagingOptics`, `ImagingTruth`, `SimStar`, `deep_field` (**The field**) and
  `SimImagingCamera` (**A frame**, steps 1 to 10, with step 1's `roi`, step 9's rule
  for a sample outside the region and step 10's rounding), and export them from
  `crates/simulator/src/lib.rs`.
- [ ] T009 [US2] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US2-S1, US2-S2, US2-S3, US2-S4, US2-S5, US2-S6, US2-S7, US2-S8 and US2-S9).

## Phase 3: User Story 3 — The imaging service on the scope link (Priority: P1)

### Tests first

- [ ] T010 [US3] Create `crates/scope-node/tests/imaging_service.rs` with the node rig
  and failing tests for US3-S1, US3-S2, US3-S3, US3-S4, US3-S5, US3-S6 and US3-S7.
- [ ] T011 [US3] Create `crates/simulator/tests/imaging_link.rs` with failing tests for
  US3-S8, whose camera is the test's own and returns a pattern, not a render, and
  US3-S9, `from_cameras` included.

### Implementation

- [ ] T012 [US3] Append `ScopeNodeImaging` and its messages to
  `crates/scope-link/proto/scope_node/v1/scope_node.proto`, exactly as plan.md § The
  scope link gives them, `ImagingFrameHeader`'s `ImagingRoi roi = 17` included,
  changing nothing already there.
- [ ] T013 [US3] Create `crates/scope-node/src/imaging/service.rs` with `CHUNK_BYTES`,
  `ImagingHost` and its `new`, and the five calls, every `Expose` failure after the
  token check being the stream's first item and every header's `roi` the frame's own;
  and in `crates/scope-node/src/service.rs`
  add the `OnceLock`, `with_imaging`, `imaging_host`, the route and `ready`'s visibility
  (plan.md § The node's service). Add `pub mod service;` to
  `crates/scope-node/src/imaging/mod.rs`.
- [ ] T014 [US3] In `crates/simulator/src/node.rs`, attach a `SimImagingCamera` in `new`
  and `from_camera` and add `from_cameras`.
- [ ] T015 [US3] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US3-S1, US3-S2, US3-S3, US3-S4, US3-S5, US3-S6, US3-S7, US3-S8 and US3-S9).

## Phase 4: User Story 4 — An INDI camera described and configured (Priority: P1)

### Tests first

- [ ] T016 [US4] Create `crates/scope-node/tests/support/mod.rs`, its first line
  `#![allow(dead_code)]`, with the stand-in INDI server of plan.md § The script format:
  the script reader, its own scanner of the adapter's elements, the reader task that
  stamps each element as it arrives, `@ expect` with and without `after MS`,
  `@ send [after MS]`, `@ close`, `Scripts::new`, `connect`, `connections`, `finish`
  (which also fails on an element no `@ expect` took) and `fixture`.
- [ ] T017 [US4] Create `crates/scope-node/tests/indi.rs` with the colour-camera and
  mono-camera scripts written out from plan.md § The two worked scripts, each with its
  `@ expect after 200 enableBLOB text=Also`, and failing
  tests for US4-S1, US4-S2, US4-S3, US4-S4, US4-S5 and US4-S8, the last over the
  colour-camera script with its `CCD_CONTROLS` expectation and answer added. Create
  `crates/scope-node/tests/imaging_config.rs` with failing tests for US4-S6 and US4-S7,
  `neutral` included, its config files kept behind a drop guard under
  `CARGO_TARGET_TMPDIR`.

### Implementation

- [ ] T018 [US4] Add `quick-xml` to `crates/scope-node/Cargo.toml` exactly as spec.md
  owner approval 3 gives it, and commit the `Cargo.lock` it produces. Do not add
  `base64`: nothing in this story calls it, and US5 adds it (plan.md § Plan traps 19).
- [ ] T019 [US4] Create `crates/scope-node/src/imaging/indi/protocol.rs` (`Element`,
  `Reader`, the four writers) and `crates/scope-node/src/imaging/indi/mod.rs`
  (`NeutralControls`, `IndiCamera` with `new` and `tcp` taking the neutral controls,
  its one task, connecting with its step 7 for the neutral controls and the rule that a
  failed step fails the connection, `describe` with its refusal of a sample size other
  than 8 or 16 bits, `status` and `apply`), with the bytes, waits, order and texts of
  plan.md § The INDI adapter. Until US5, `expose` answers
  `Failed("exposures arrive with US5")` and `abort` answers `Ok`. Add `pub mod indi;` to
  `crates/scope-node/src/imaging/mod.rs`.
- [ ] T020 [US4] In `crates/scope-node/src/config.rs`, add `ImagingAdapter`,
  `ImagingConfig` with `adapter`, `address`, `device` and `neutral`, `address()`,
  `neutral_controls()` and `printed()`, the missing-adapter note and the validations in
  their order and with their texts. In `crates/scope-node/src/main.rs`, print the table
  under `--check-config` and attach the camera the table names with its neutral controls
  (plan.md § The config, § The binary). Add `## The imaging camera` to
  `crates/scope-node/README.md`, with its paragraph on white balance and the camera's
  other controls (plan.md § The README).
- [ ] T021 [US4] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US4-S1, US4-S2, US4-S3, US4-S4, US4-S5, US4-S6, US4-S7 and US4-S8).

## Phase 5: User Story 5 — Exposures through INDI (Priority: P1)

### Tests first

- [ ] T022 [US5] Create `crates/scope-node/tests/indi_expose.rs`, using
  `crates/scope-node/tests/support/mod.rs` unchanged, with a helper of its own that
  builds a FITS BLOB (plan.md § Reading a FITS BLOB, **The worked FITS frame**) and
  failing tests for US5-S1, US5-S2, US5-S3, US5-S4, US5-S5, US5-S6 and US5-S7, the scripts of US5-S3, US5-S4
  and US5-S6 listing every element the adapter sends with `@ expect after MS` at the
  times those scenarios state, and for US5-S8 against
  `fixtures/scope-node/indi/ccd-simulator.script` and
  `fixtures/scope-node/indi/ccd-simulator.expect.json`.

### Implementation

- [ ] T023 [US5] Add `base64` to `crates/scope-node/Cargo.toml` exactly as spec.md
  owner approval 3 gives it, and commit the `Cargo.lock` change. Create
  `crates/scope-node/src/imaging/fits.rs` with `Picture` and
  `read`, and add `pub mod fits;` to `crates/scope-node/src/imaging/mod.rs`.
- [ ] T024 [US5] In `crates/scope-node/src/imaging/indi/mod.rs`, replace US4's
  placeholder with `expose` and `abort`: the frame type, `CCD_EXPOSURE`, the times, the
  frame's `roi` from `CCD_FRAME` as it stood when the exposure was asked for, the
  BLOB decoded with `base64` and read on the blocking pool, the failures, the one retry,
  which sends the frame type again, `Busy` for an `apply` that names a setting during an
  exposure, and the texts of plan.md § The INDI adapter. Add the failed-exposure
  paragraph to `crates/scope-node/README.md`.
- [ ] T025 [US5] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US5-S1, US5-S2, US5-S3, US5-S4, US5-S5, US5-S6, US5-S7 and US5-S8).

## Phase 6: User Story 6 — A copy of each frame kept at the scope node (Priority: P3)

### Tests first

- [ ] T026 [US6] Create `crates/scope-node/tests/imaging_save.rs` with a FITS reader of
  its own, its folders and files behind a drop guard under `CARGO_TARGET_TMPDIR`, and
  failing tests for US6-S1, US6-S2, US6-S3 and US6-S4.

### Implementation

- [ ] T027 [US6] Create `crates/scope-node/src/imaging/save.rs` with `write` (plan.md
  § The node's copy), its cards with `XORGSUBF` and `YORGSUBF` from the frame's `roi`,
  in the format that section writes out, and add `pub mod save;` to `crates/scope-node/src/imaging/mod.rs`.
  In `crates/scope-node/src/imaging/service.rs`, write the file before the header when
  `save` is true and a folder is set, and add `ImagingHost::set_save_dir`; in
  `crates/scope-node/src/service.rs`, add `with_imaging_save_dir`.
- [ ] T028 [US6] In `crates/scope-node/src/config.rs`, add `save_dir`, its validation
  and its printed line; in `crates/scope-node/src/main.rs`, pass it to the service. Add
  `save_dir` to `crates/scope-node/README.md`. Run the `lint` and `test` gates with
  every existing test unchanged and green. Commit the green story (US6-S1, US6-S2, US6-S3 and US6-S4).

## Phase 7: User Story 7 — Probe the camera from the scope node (Priority: P3)


### Tests first

- [ ] T029 [US7] Create `crates/scope-node/tests/imaging_probe.rs` with failing tests
  for US7-S1, US7-S2 and US7-S3, the last against the built binary as
  `crates/scope-node/tests/cli.rs` runs it, its config file behind a drop guard under
  `CARGO_TARGET_TMPDIR`.

### Implementation

- [ ] T030 [US7] Create `crates/scope-node/src/imaging/probe.rs` with `run` (plan.md
  § The probe) and add `pub mod probe;` to `crates/scope-node/src/imaging/mod.rs`. In
  `crates/scope-node/src/main.rs`, add `--probe-camera` and `--probe-exposure-ms`. Add
  the probe to `crates/scope-node/README.md`.
- [ ] T031 [US7] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US7-S1, US7-S2 and US7-S3).

## Execution Order

```text
US1 ──► US2 ──► US3 ──► US4 ──► US5 ──► US6 ──► US7
```

- **US1, US2, US3** come first, in that order: the port, the simulated camera that
  implements it, and the service that serves both.
- **US4 then US5** are the INDI adapter: described and configured, then exposures.
- **US6** follows US5: it extends US3's service and writes the node's config and binary
  after the adapter's two stories have.
- **US7** follows US6, which orders the node's binary, README and imaging module.
- The seven stories form one chain. The console's stories are 031's and start after
  this trio has landed.
- **`[P]`** marks nothing here: every task shares a file with another in its story.
