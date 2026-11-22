---
description: "Task list for 046-node-reads-pi-5-frames: The Node Reads the Pi 5's Camera Frames"
---

# Tasks: The Node Reads the Pi 5's Camera Frames

**Prerequisites** (plan.md § Prerequisites):
- every spec in `depends_on_landed` has landed on `main`: met at 425b9da0;
- the Pi 5 capture is on `main` in `fixtures/scope-node/pi5/`, staged by the operator
  under owner decision 2: not met;
- the owner decisions in spec.md are settled: not met, both open;
- the story has been probe-built from this text: not met.

**Rules for every story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml`,
  `CODEOWNERS`, `ergane.yaml` or `.specify/`. If an existing test fails, stop and
  escalate with its exact output. Nothing under `fixtures/` is written: the Pi 5 capture
  is the operator's.
- Write only the files your story's tasks name; they are its `writes` in plan.md
  § Refinement Record. A story that needs another file escalates.
- `crates/finder`, `crates/simulator`, `crates/scope-link`, `proto/`, `Cargo.toml`,
  `Cargo.lock`, and the node's `main.rs`, `capture.rs`, `service.rs`, `config.rs`,
  `rpicam/metadata.rs`, `mount/` and `imaging/` are never touched.
- No test uses a camera or a Pi, and no test signals a process it did not start. Run no
  tool the gates don't run, and do not set `CARGO_TARGET_DIR`.

## Phase 1: User Story 1 - The node frames the Pi 5's camera at its row stride (Priority: P1)

### Tests first

- [ ] T001 [US1] Create `crates/scope-node/tests/rpicam_node.rs` with the fake camera
  (its `rpicam-hello` and its `rpicam-vid`, which writes each frame's bytes and then
  its metadata object) and the node rig as spec.md § User Scenarios defines them, and
  one test for each of US1-S1, US1-S2, US1-S5 and US1-S9, under `#[tokio::test]`. On
  today's adapter US1-S2 passes and the other three fail: over the Pi 5 capture no raw
  frame comes and nothing is logged, as the probe saw.
- [ ] T002 [US1] Create `crates/scope-node/tests/rpicam_stride.rs` with its own copy of
  the fake `rpicam-vid`, the synthetic streams and the adapter rig, and one test for
  each of US1-S3, US1-S4, US1-S6, US1-S7 and US1-S8, under `#[tokio::test]` (US1-S3 and
  US1-S4 under `#[test]`). They name `Isp`, `listing::isp` and `with_isp`, which T003 to
  T005 add; until then the file does not build, as on the probe.

### Implementation

- [ ] T003 [US1] In `crates/scope-node/src/rpicam/yuv.rs`, add `Isp` and `Isp::stride`,
  route `stride` through `Isp::Vc4`, and add the frame count and the padding check to
  `YuvReader::next` (FR-001, FR-004; plan.md § Interfaces).
- [ ] T004 [US1] In `crates/scope-node/src/rpicam/listing.rs`, add `is_camera` and
  `listing::isp` (FR-002; plan.md § Interfaces). `parse` does not change.
- [ ] T005 [US1] In `crates/scope-node/src/rpicam/mod.rs`, add the `isp` field,
  `with_isp`, the ISP in `discover` and its stride in `start` (FR-003; plan.md
  § Interfaces). Nothing else in the file changes.
- [ ] T006 [US1] Run the `lint` and `test` gates with every existing test unchanged and
  green, `crates/scope-node/tests/rpicam.rs` among them, then commit the green story
  (FR-001 to FR-005; US1-S1, US1-S2, US1-S3, US1-S4, US1-S5, US1-S6, US1-S7, US1-S8,
  US1-S9).

## Execution Order

```text
US1
```

- **US1** starts from a `main` that holds every spec of `depends_on_landed` and the
  operator's Pi 5 capture.
- **`[P]`** marks nothing here: the implementation follows the tests.
