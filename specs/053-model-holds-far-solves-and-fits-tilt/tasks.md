---
description: "Task list for 053-model-holds-far-solves-and-fits-tilt: The Pointing Model Holds Far Solves, and the Second Sync Fits the Tilt"
---

# Tasks: The Pointing Model Holds Far Solves, and the Second Sync Fits the Tilt

**Prerequisites** (plan.md § Prerequisites, each with its evidence there):
- every spec in `depends_on_landed` has landed on `main`: met at f6d4f5fd;
- `main` is still as read: met at f6d4f5fd, the draft's revision;
- the owner decisions in spec.md are settled: not met, all six open;
- every story has been probe-built from this text: not met;
- no operator-staged file is needed: met.

**Rules for every story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml`,
  `CODEOWNERS`, `ergane.yaml` or `.specify/`. If an existing test fails, stop and
  escalate with its exact output. A test file an earlier story of this spec created is
  an existing test. The exceptions: US1's one line of
  `slips_refit_only_epoch_zero_points_at_the_half_degree_threshold` in
  `crates/finder/src/model.rs` (spec owner approval 1), and each story's `mod` line in
  `crates/finder/tests/motion/model/mod.rs`, `crates/finder/tests/motion/goto/mod.rs`
  or `crates/finder/tests/console/main.rs`, each in its alphabetical place.
- Write only the files your story's tasks name; they are its `writes` in plan.md
  § Refinement Record. A story that needs another file escalates.
- `crates/finder/src/live.rs`, the tracker, `crates/finder/src/pushto.rs`,
  `crates/finder/src/mcp/`, `crates/finder/web/`, `crates/star-detection/`,
  `crates/scope-node/`, `crates/scope-link/`, `crates/simulator/`, `proto/`, the plate
  solver's crates, the databases and `crates/finder/src/imaging/` are never touched, and
  nothing is added or changed under `crates/finder/tests/support/`. In
  `crates/finder/src/session.rs`, only `record_sync` changes.
- Copy the words of spec.md's FRs and of plan.md § Interfaces exactly, the README's and
  the glossary's among them. Say "finder camera", never "finder" alone, in every
  message.
- No test downloads anything, and no test writes outside `CARGO_TARGET_TMPDIR`. Run no
  tool the gates don't run, and do not set `CARGO_TARGET_DIR`.
- The results said here before each story's implementation are the probe's
  (plan.md § What was run and what was read), where it ran them.

## Phase 1: User Story 1 - The pointing model takes only the solves it can vouch for (Priority: P1)

### Tests first

- [ ] T001 [US1] Create `crates/finder/tests/motion/model/intake.rs` with the mount's
  geometry and the chord distance of spec.md § User Scenarios written in the file, one
  test for each of US1-S1 to US1-S6 under `#[test]` and one for US1-S7 under
  `#[tokio::test]`, and add `mod intake;` to `crates/finder/tests/motion/model/mod.rs`
  in its alphabetical place. Before T002 and T003 the motion binary does not build: it
  reads `Model::candidate` and `ModelView::candidate` (read, not run).

### Implementation

- [ ] T002 [US1] In `crates/finder/src/model.rs`, add `MAX_BASE_TILT_DEG`, `SLIP_DEG`,
  `Sync::axis_az_deg`, `Model::candidate` (`#[serde(skip)]`), `ModelView::candidate`
  and `CandidateView`; split `add_sync` into its checks, the rule and `take`, with
  `vouches` and `tilt_spot`, the base-tilt test on `axis_az_deg`; clear the candidate
  in `reset_night` and fill it in `view`, each as plan.md § Interfaces gives it
  (FR-001 to FR-003, FR-005).
- [ ] T003 [US1] In `crates/finder/src/model.rs`'s own tests, replace the line
  `model.add_sync(sync.clone());` of
  `slips_refit_only_epoch_zero_points_at_the_half_degree_threshold` with plan.md
  § Interfaces' six lines and its comment (spec owner approval 1; US1-S8). On today's
  `add_sync` the 0.6° sync answers true, so the new assertion fails (read, not run).
- [ ] T004 [US1] In `crates/finder/src/session.rs`'s `record_sync`, keep the model's
  candidate when `add_sync` refuses, without writing the state file, as plan.md
  § Interfaces gives it (FR-004).
- [ ] T005 [US1] Run the `lint` and `test` gates with every existing test unchanged and
  green but owner approval 1's line, then commit the green story (FR-001 to FR-005;
  US1-S1, US1-S2, US1-S3, US1-S4, US1-S5, US1-S6, US1-S7, US1-S8).

## Phase 2: User Story 2 - GoTo and the second sync use only taken solves, and the second sync measures the tilt (Priority: P1)

### Tests first

- [ ] T006 [US2] Create `crates/finder/tests/motion/goto/intake.rs` with the `Swap`
  solver, a `NodeTime` of its own over the simulator's clock, `rig_with` building
  `crate::support::goto::Rig`'s public fields over a `SimMount` of a given
  `MountConfig`, the two syncs of US2-S4, the tilt rig's zero points and the place of
  spec.md § User Scenarios, written in the file; one test for each tilt of US2-S1
  (0°, 1°, 2°, 3°, 4° and 5°, both directions in each) and one for each of US2-S2 to
  US2-S5, all under `#[tokio::test(start_paused = true)]`; and add `mod intake;` to
  `crates/finder/tests/motion/goto/mod.rs` in its alphabetical place. On US1 nine of
  the ten fail: the tilt cases and US2-S4 on `frame_id` absent, US2-S5 on its reason
  and US2-S2 on the altitude; US2-S3 passes, since it is US1's rule (run).

### Implementation

- [ ] T007 [US2] In `crates/finder/src/goto.rs`, add `SECOND_SYNC_BAND`,
  `Progress::frame_id` and `State::frame`, reset where a GoTo or second sync starts and
  copied by `update`; in the settle loop, pass over a report whose time no sync of the
  session's model holds, recording a held candidate's frame and distance, and fail on
  the 30 s timeout with FR-008's reason when one was recorded (FR-006 to FR-008;
  plan.md § Interfaces).
- [ ] T008 [US2] In `Planner::second_sync`, add the axis rule against the current
  epoch's syncs, start the altitudes at the current altitude held to the band, and sort
  the spots inside the band first, keeping today's clearance, cone and sky test
  (FR-009; plan.md § Interfaces).
- [ ] T009 [US2] Run the `lint` and `test` gates with every existing test unchanged and
  green, US1's test file and `crates/finder/tests/motion/goto/second_sync.rs` among
  them, then commit the green story (FR-006 to FR-010; US2-S1, US2-S2, US2-S3, US2-S4,
  US2-S5).

## Phase 3: User Story 3 - The README and the glossary say which solves the model takes (Priority: P2)

### Tests first

- [ ] T010 [US3] Create `crates/finder/tests/console/model_docs.rs` with one test for
  US3-S1 under `#[test]`, reading `crates/finder/README.md` and `GLOSSARY.md` from
  `env!("CARGO_MANIFEST_DIR")` with every run of whitespace joined into one space, as
  `crates/finder/tests/console/imaging_run_docs.rs` reads the README, and add
  `mod model_docs;` to `crates/finder/tests/console/main.rs` in its alphabetical place.
  On US2 the README and the glossary lack every sentence, so it fails (read, not run).

### Implementation

- [ ] T011 [US3] In `crates/finder/README.md`, change the GoTo and second-sync paragraphs
  and add the paragraph after the second sync's; in `GLOSSARY.md`, change **Base tilt**
  and **Slip** and add **Candidate** after **Slip**, each as plan.md § Interfaces gives
  it (FR-011 to FR-013).
- [ ] T012 [US3] Run the `lint` and `test` gates with every existing test unchanged and
  green, the earlier stories' test files and `imaging_run_docs.rs` among them, then
  commit the green story (FR-011 to FR-013; US3-S1).

## Execution Order

```text
US1 -> US2 -> US3
```

- **US1** starts from a `main` that holds every spec of `depends_on_landed`.
- **US2** starts once US1 has merged: its loop waits on US1's taken syncs and reads
  US1's candidate, and its spot reads `Sync::axis_az_deg`.
- **US3** starts once US2 has merged: its words describe US2's `frame_id`, failure and
  spot.
- **`[P]`** marks nothing here: each story's implementation follows its tests.
