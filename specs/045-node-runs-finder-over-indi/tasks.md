---
description: "Task list for 045-node-runs-finder-over-indi: The Node Runs the Finder Camera over INDI"
---

# Tasks: The Node Runs the Finder Camera over INDI

**Prerequisites** (plan.md § Prerequisites):
- every spec in `depends_on_landed` has landed on `main`: met at be7f3ff6;
- the owner decisions in spec.md are settled: met, the owner's answers of 2026-11-25
  (spec.md § Clarifications);
- every story has been probe-built from this text: not met; the drafting probe and the
  fold of review round 1 built both stories from the seat's own trial, not from this text;
- no operator-staged file is needed under owner decision 7's answer.

**Rules for every story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml`,
  `CODEOWNERS`, `ergane.yaml` or `.specify/`. If an existing test fails, stop and
  escalate with its exact output. `crates/scope-node/tests/support/mod.rs` is an existing
  test file: use it, never edit it. A test file an earlier story of this spec created is
  an existing test: US2 uses US1's `crates/scope-node/tests/finder_script/mod.rs` and
  never edits it.
- Write only the files your story's tasks name; they are its `writes` in plan.md
  § Refinement Record. A story that needs another file escalates.
- `crates/finder`, `crates/simulator`, `crates/scope-link`, `proto/`, `Cargo.toml`,
  `Cargo.lock`, and the node's `camera.rs`, `capture.rs`, `service.rs`, `exposure.rs`,
  `detect.rs`, `rpicam/`, `synthetic.rs`, `mount/` and every file under `imaging/` but
  `imaging/indi/mod.rs` are never touched.
- Copy the words of spec.md's FRs and scenarios and the README section of plan.md
  § Interfaces exactly. Say "finder camera", never "finder" alone, in every message
  (GLOSSARY.md).
- No test uses a camera, a Pi, a network beyond `127.0.0.1` or a real INDI server, and no
  test signals a process it did not start. Run no tool the gates don't run, and do not set
  `CARGO_TARGET_DIR`.
- US1's diff has about 160 B of room under 60 % of 65,536 B at the usual overrun
  (plan.md § Plan traps 2): keep to plan.md § Interfaces, and escalate rather than grow it.

## Phase 1: User Story 1 - The finder camera is described and framed over INDI (Priority: P1)

### Tests first

- [ ] T001 [US1] Create `crates/scope-node/tests/finder_script/mod.rs`, opening with
  `#![allow(dead_code)]`, with the finder camera's device `Finder CCD`, the test clock and
  reading(ms), the finder script, the gain step, the exposures, its own FITS writer, the
  worked 16-bit frame and the finder rig's camera of spec.md § User Scenarios, built with
  the stand-in's public builders through `crate::support`.
- [ ] T002 [US1] Create `crates/scope-node/tests/indi_finder.rs` with `mod finder_script;`
  and `mod support;`, and one test for each of US1-S1 to US1-S9, under
  `#[tokio::test(start_paused = true)]`. They name `scope_node::indi_finder::IndiFinder`,
  which T005 adds; until then the file does not build, as on the probe.

### Implementation

- [ ] T003 [US1] In `crates/scope-node/src/imaging/indi/mod.rs`, make the record of a
  `setNumberVector` take each `oneNumber`'s `min` and `max` as plan.md § Interfaces gives
  it (FR-001). Nothing else in the file changes.
- [ ] T004 [US1] Create `crates/scope-node/src/indi_finder.rs` with `FRAME_MS`, `PURPOSE`,
  `IndiFinder`, `IndiFinder::discover`, `gray` and the camera port's implementation, its
  `next_frame` passing on no frame at a gain it did not send, as plan.md § Interfaces
  gives them (FR-002 to FR-005).
- [ ] T005 [US1] Add `pub mod indi_finder;` to `crates/scope-node/src/lib.rs` in its
  alphabetical place (FR-002).
- [ ] T006 [US1] Run the `lint` and `test` gates with every existing test unchanged and
  green, `crates/scope-node/tests/indi.rs`, `indi_expose.rs` and `indi_settings.rs` among
  them, then commit the green story (FR-001 to FR-006; US1-S1, US1-S2, US1-S3, US1-S4,
  US1-S5, US1-S6, US1-S7, US1-S8, US1-S9).

## Phase 2: User Story 2 - The node runs the finder camera over INDI beside the imaging camera (Priority: P1)

### Tests first

- [ ] T007 [US2] Create `crates/scope-node/tests/indi_finder_node.rs` with
  `mod finder_script;` (US1's, used and not edited) and `mod support;`, the config head,
  the node rig, the flat frame, the 60 s bound on a service read and the service rig, and
  one test for each of US2-S1 to US2-S9: US2-S1 to US2-S5 under `#[test]`, US2-S6 under
  `#[tokio::test]` and US2-S7 to US2-S9 under `#[tokio::test(start_paused = true)]`. On
  US1 alone the file does not build, since `Adapter::Indi` and `Config::indi` do not
  exist (by reading; the probe built both stories together, and US1 alone without it).

### Implementation

- [ ] T008 [US2] In `crates/scope-node/src/config.rs`, add `Adapter::Indi`,
  `FinderIndiConfig` with `address`, `device`, `neutral`, `neutral_controls` and
  `printed`, `Config::indi`, `validate_indi` called from `validate`, and `indi` in
  `masked`, as plan.md § Interfaces gives them (FR-007, FR-008, FR-009). The `[imaging]`
  table's code and the file's own tests do not change.
- [ ] T009 [US2] In `crates/scope-node/src/main.rs`, print the `[indi]` table under
  `--check-config` and serve `IndiFinder::discover` for `Adapter::Indi`, with the table's
  neutral controls, as plan.md § Interfaces gives them (FR-009, FR-010).
- [ ] T010 [US2] In `crates/scope-node/README.md`, add `## The finder camera over INDI`
  before `## The imaging camera`, from plan.md § Interfaces (FR-011).
- [ ] T011 [US2] Run the `lint` and `test` gates with every existing test unchanged and
  green, `crates/scope-node/tests/cli.rs`, `imaging_config.rs` and US1's
  `indi_finder.rs` among them, then commit the green story (FR-007 to FR-012; US2-S1,
  US2-S2, US2-S3, US2-S4, US2-S5, US2-S6, US2-S7, US2-S8, US2-S9).

## Execution Order

```text
US1 -> US2
```

- **US1** starts from a `main` that holds every spec of `depends_on_landed`.
- **US2** starts once US1 has merged: its binary builds US1's adapter, its tests use
  US1's shared test module, and US2-S7 to US2-S9 drive the adapter inside the node's
  service.
- **`[P]`** marks nothing here: each story's implementation follows its tests.
