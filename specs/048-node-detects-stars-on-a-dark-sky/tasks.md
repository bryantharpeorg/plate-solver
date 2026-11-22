---
description: "Task list for 048-node-detects-stars-on-a-dark-sky: The Node Detects Stars on a Dark Sky"
---

# Tasks: The Node Detects Stars on a Dark Sky

**Prerequisites** (plan.md § Prerequisites):
- every spec in `depends_on_landed` has landed on `main`: met at 28f436df;
- the owner decisions in spec.md are settled: met, all seven answered on 2026-11-22;
- the stories have been probe-built from this text: not met.

**Rules for every story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml`,
  `CODEOWNERS`, `ergane.yaml` or `.specify/`. If an existing test fails, stop and
  escalate with its exact output. `crates/simulator/tests/support/mod.rs` is an existing
  test file: the skies go in the new `crates/simulator/tests/dark/mod.rs`.
- Write only the files your story's tasks name; they are its `writes` in plan.md
  § Refinement Record. A story that needs another file escalates.
- `crates/finder`, `crates/scope-link`, `proto/`, `Cargo.toml`, `Cargo.lock`, the
  simulator's `src/`, and the node's `main.rs`, `capture.rs`, `camera.rs`, `exposure.rs`,
  `rpicam/`, `mount/` and `imaging/` are never touched.
- Draw the skies exactly as plan.md § The skies gives them. Write every expected output,
  message and print as a literal.
- No test uses a camera, a Pi or a network socket, and no test sends a signal. Run no
  tool the gates don't run, and do not set `CARGO_TARGET_DIR`.

## Phase 1: User Story 1 - Detection has a noise floor (Priority: P1)

### Tests first

- [ ] T001 [US1] Create `crates/star-detection/tests/noise_floor.rs` with one test for
  each of US1-S1 and US1-S2, under `#[test]`. They name `detect_stars_with_noise_floor`,
  which T005 adds; until then the file does not build.
- [ ] T002 [US1] Create `crates/simulator/tests/dark/mod.rs`, starting
  `#![allow(dead_code)]`, with the database, the field, the two skies, the random
  generator, the drawn stars, the frame, the true-star count and the console's solve, as
  plan.md § The skies and spec.md § User Scenarios give them. Its public items are
  exactly those plan.md § The skies lists, with those signatures: `WIDTH` and `HEIGHT`
  (`u32`), `Sky` (`sky`, `black`, `read`, `flux`, all `f64`; `Clone`, `Copy`), `DARK`,
  `CITY`, `Field` (`stars: Vec<RenderStar>`, `truth: TrueDirection`),
  `field() -> Field`, `peak(flux: f64, star: &RenderStar) -> f64`,
  `drawn(field: &Field, sky: Sky, scale: f64) -> Vec<RenderStar>`,
  `render(field: &Field, sky: Sky, scale: f64, seed: u64) -> Vec<u8>`,
  `frame(pixels: Vec<u8>) -> GrayFrame`, `floor(value: f64) -> DetectSettings`,
  `true_stars(centroids: &[Centroid], stars: &[RenderStar]) -> usize` and
  `solve(field: &Field, centroids: &[Centroid]) -> Option<f64>`. US2's test uses them
  as they are and does not edit the module.
- [ ] T003 [US1] Create `crates/simulator/tests/dark_sky.rs`, declaring `mod dark;`,
  with one test for each of US1-S3, US1-S4 and US1-S5, under `#[test]`. US1-S4 and
  US1-S5 name `noise_floor`, which T006 adds, so until then the file does not build. On
  `main`'s code the probe saw US1-S3, and US1-S5's solve without a floor, pass.
- [ ] T004 [US1] Create `crates/scope-node/tests/noise_floor.rs` with the config rig as
  spec.md § User Scenarios defines it, and one test for each of US1-S6 and US1-S7, under
  `#[test]`. Today `noise_floor` is an unknown key (`invalid noise_floor at line 5`).

### Implementation

- [ ] T005 [US1] In `crates/star-detection`, make `noise::NOISE_FLOOR` public, add
  `detect_stars_with_noise_floor` in `detect.rs` with `detect_stars` calling it, and
  re-export it from `lib.rs` (FR-001; plan.md § Interfaces).
- [ ] T006 [US1] In `crates/scope-node/src/detect.rs`, add `DetectSettings::noise_floor`
  and use it in `detect` (FR-002; plan.md § Interfaces).
- [ ] T007 [US1] In `crates/scope-node/src/config.rs`, add the floor's check, 0.2 to 5,
  to `validate` (FR-003; plan.md § Interfaces). Nothing else in the file changes.
- [ ] T008 [US1] In `crates/scope-node/README.md`, add `## Dark skies: the noise floor`
  before `## The mount link`, with 0.5 as the dark-site starting point, the floor left
  out in town, whose noise on the finder camera is not measured, or star counts compared
  with and without it, and, beside the floor, Setup's detection threshold (sigma) as the
  no-code option (FR-004; plan.md § Interfaces).
- [ ] T009 [US1] Run the `lint` and `test` gates with every existing test unchanged and
  green, `crates/star-detection/tests/parity.rs` and `crates/scope-node/tests/detect.rs`
  among them, then commit the green story (FR-001 to FR-004, FR-009; US1-S1, US1-S2,
  US1-S3, US1-S4, US1-S5, US1-S6, US1-S7).

## Phase 2: User Story 2 - Auto-exposure finds the stars on a dark sky (Priority: P1)

### Tests first

- [ ] T010 [US2] Create `crates/simulator/tests/dark_sky_exposure.rs`, declaring US1's
  `mod dark;`, with the sky camera (46.34 frames/s, the finder camera's listed ranges,
  gain 1 to 63.9375 and exposure 0.13 to 3066.985 ms, a first automatic frame of 200 ms),
  the node rig (`NodeService::new`, the node's own 200 ms frame interval) and the
  pairing settings as spec.md § User Scenarios defines them, and one test for each of
  US2-S1 and US2-S2, under `#[tokio::test(start_paused = true)]`. On today's
  `Configure`, US2-S1 passes and US2-S2 fails: pairing resets the floor, and the fourth
  frame falls to 53 ms. `dark/mod.rs` is US1's and does not change.

### Implementation

- [ ] T011 [US2] In `crates/scope-node/src/service.rs`, keep the node's own detection in
  `Shared` and take it as `from_wire`'s base in `configure` (FR-007; plan.md
  § Interfaces). Nothing else in the file changes.
- [ ] T012 [US2] In `crates/plate-solver/examples/solve_image.rs`, add `--noise-floor`
  to its arguments, its detection call, its usage line and its doc comment (FR-006;
  plan.md § Interfaces).
- [ ] T013 [US2] In `crates/scope-node/README.md`, add at the end of US1's section,
  after its Setup's detection threshold (sigma) paragraph, the paragraphs on the console
  and recordings, the 200 ms exposure cap with its fallback, and "Finding the value",
  with 200 ms frames at gains 16, 32 and the camera's top, and its live check, promising
  no star count (FR-005; plan.md § Interfaces).
- [ ] T014 [US2] Run the `lint` and `test` gates with every existing test unchanged and
  green, `crates/simulator/tests/node.rs` among them, then commit the green story
  (FR-005 to FR-009; US2-S1, US2-S2).

## Execution Order

```text
US1 → US2
```

- **US1** starts from a `main` that holds every spec of `depends_on_landed`.
- **US2** starts from a `main` holding US1: its test reads US1's `dark/mod.rs` and sets
  US1's floor, and it extends US1's README section.
- **`[P]`** marks nothing here: the implementation follows the tests, and the stories
  merge in order.
