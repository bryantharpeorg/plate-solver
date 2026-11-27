---
description: "Task list for 053-second-sync-fits-base-tilt: The Second Sync Fits the Base Tilt"
---

# Tasks: The Second Sync Fits the Base Tilt

**Prerequisites** (plan.md § Prerequisites, each with its evidence there):
- every spec in `depends_on_landed` has landed on `main`: met at f6d4f5fd;
- `main` is still as read: met at f6d4f5fd, the draft's revision;
- the shared files are re-anchored on 052, which lands first: not met;
- the owner decisions in spec.md are settled: not met, all six open;
- every story has been probe-built from this text: not met;
- no operator-staged file is needed: met.

**Rules for every story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml`,
  `CODEOWNERS`, `ergane.yaml` or `.specify/`. If an existing test fails, stop and
  escalate with its exact output. A test file an earlier story of this spec created is
  an existing test. The one exception is each story's `mod` line in
  `crates/finder/tests/motion/model/mod.rs`, `crates/finder/tests/motion/goto/mod.rs`
  or `crates/finder/tests/console/main.rs`, each in its alphabetical place.
- Write only the files your story's tasks name; they are its `writes` in plan.md
  § Refinement Record. A story that needs another file escalates.
- `crates/finder/src/live.rs`, the tracker, `crates/finder/src/pushto.rs`,
  `crates/finder/src/mcp/`, `crates/finder/web/`, `crates/star-detection/`,
  `crates/scope-node/`, `crates/scope-link/`, `crates/simulator/`, `proto/`, the plate
  solver's crates, the databases and `crates/finder/src/imaging/` are never touched, and
  nothing is added or changed under `crates/finder/tests/support/`. In
  `crates/finder/src/session.rs`, only `record_frame_sync` changes.
- Copy the words of spec.md's FRs and of plan.md § Interfaces exactly, the README's and
  the glossary's among them. Say "finder camera", never "finder" alone, in every
  message.
- No test downloads anything, and no test writes outside `CARGO_TARGET_TMPDIR`. Run no
  tool the gates don't run, and do not set `CARGO_TARGET_DIR`.
- The results said here before each story's implementation are the probe's
  (plan.md § What was run and what was read, § Each story before it).

## Phase 1: User Story 1 - The second sync's solve fits the base tilt (Priority: P1)

### Tests first

- [ ] T001 [US1] Create `crates/finder/tests/motion/model/tilt.rs` with the mount of
  spec.md § User Scenarios written in the file (zero points, collimation and axis
  non-perpendicularity, then Rodrigues' rotation by the base tilt), the chord distance
  and the worst miss over its grid, one test for each of US1-S1 to US1-S5 under
  `#[test]`, and add `mod tilt;` to `crates/finder/tests/motion/model/mod.rs` in its
  alphabetical place. On `main`'s `model.rs`, US1-S1, US1-S3, US1-S4 and US1-S5 fail and
  US1-S2 passes (run).

### Implementation

- [ ] T002 [US1] In `crates/finder/src/model.rs`, add `MAX_BASE_TILT_DEG` and
  `Sync::axis_az_deg`; split `add_sync` after its checks into `fits`, `tilt_spot` and
  `take`, the base-tilt test reading `axis_az_deg`, and reword `Model::base_tilt`'s doc,
  each as plan.md § Interfaces gives it (FR-001 to FR-003).
- [ ] T003 [US1] Run the `lint` and `test` gates with every existing test unchanged and
  green, then commit the green story (FR-001 to FR-003; US1-S1, US1-S2, US1-S3, US1-S4,
  US1-S5).

## Phase 2: User Story 2 - The second sync keeps to the band and says when it fits no tilt, and a GoTo corrects only from a taken sync (Priority: P1)

### Tests first

- [ ] T004 [US2] Create `crates/finder/tests/motion/goto/tilt.rs` with a `NodeTime` of
  its own over the simulator's clock, a public `rig_with` building
  `crate::support::goto::Rig`'s public fields over a `SimMount` of a given
  `MountConfig` and a given live solver, a public `latest_frame`, the tilt rig's zero
  points and the place of spec.md § User Scenarios, written in the file; one test for
  each tilt of US2-S1 (0°, 1°, 2°, 3°, 4° and 5°, both directions in each) and one for
  each of US2-S2 and US2-S3; and add `mod tilt;` to
  `crates/finder/tests/motion/goto/mod.rs` in its alphabetical place.
- [ ] T005 [US2] Create `crates/finder/tests/motion/goto/field.rs` with the `Scale`
  solver of spec.md § User Scenarios over `super::tilt::rig_with`, one test for each of
  US2-S4 and US2-S5, and add `mod field;` to `crates/finder/tests/motion/goto/mod.rs`
  in its alphabetical place. All of T004's and T005's tests run under
  `#[tokio::test(start_paused = true)]`; on US1 every one fails (run).

### Implementation

- [ ] T006 [US2] In `crates/finder/src/session.rs`'s `record_frame_sync`, read the
  database in use's range inside the existing lock and take no sync from a solve whose
  field lies outside it widened by 5 %, as plan.md § Interfaces gives it (FR-007).
- [ ] T007 [US2] In `crates/finder/src/goto.rs`, add `Progress::frame_id`,
  `Progress::frame_time` and `State::frame`, reset where a GoTo or second sync starts
  and copied by `update`; in the settle loop, pass over a report whose time no sync of
  the session's model holds, set `State::frame` from the report taken, fail on the 30 s
  timeout with FR-006's reason when one was passed over, and end a second sync that
  leaves no base tilt `failed` with FR-009's reason (FR-004 to FR-006, FR-009; plan.md
  § Interfaces).
- [ ] T008 [US2] In `Planner::second_sync`, add `SECOND_SYNC_BAND`, start the altitudes
  at the current altitude held to the band, and sort the spots inside the band first,
  keeping today's clearance, cone and sky test (FR-008; plan.md § Interfaces).
- [ ] T009 [US2] Run the `lint` and `test` gates with every existing test unchanged and
  green, US1's test file and `crates/finder/tests/motion/goto/second_sync.rs` among
  them, then commit the green story (FR-004 to FR-010; US2-S1, US2-S2, US2-S3, US2-S4,
  US2-S5).

## Phase 3: User Story 3 - The README and the glossary say how the second sync fits the tilt (Priority: P2)

### Tests first

- [ ] T010 [US3] Create `crates/finder/tests/console/model_docs.rs` with one test for
  US3-S1 under `#[test]`, reading `crates/finder/README.md` and `GLOSSARY.md` from
  `env!("CARGO_MANIFEST_DIR")` with every run of whitespace joined into one space, as
  `crates/finder/tests/console/imaging_run_docs.rs` reads the README, and add
  `mod model_docs;` to `crates/finder/tests/console/main.rs` in its alphabetical place.
  On US2 the README and the glossary lack every sentence, so it fails (run).

### Implementation

- [ ] T011 [US3] In `crates/finder/README.md`, change the GoTo and second-sync paragraphs
  and add the paragraph after the second sync's; in `GLOSSARY.md`, change **Base tilt**
  and **Slip**, each as plan.md § Interfaces gives it (FR-011 to FR-013).
- [ ] T012 [US3] Run the `lint` and `test` gates with every existing test unchanged and
  green, the earlier stories' test files and `imaging_run_docs.rs` among them, then
  commit the green story (FR-011 to FR-013; US3-S1).

## Execution Order

```text
US1 -> US2 -> US3
```

- **US1** starts from a `main` that holds every spec of `depends_on_landed`.
- **US2** starts once US1 has merged: its tilt rig needs US1's rule to fit a tilt over
  1° in one epoch.
- **US3** starts once US2 has merged: its words describe US2's `frame_id`, reasons and
  band.
- **`[P]`** marks nothing here: each story's implementation follows its tests.
