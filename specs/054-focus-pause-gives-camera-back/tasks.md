---
description: "Task list for 054-focus-pause-gives-camera-back: The Focus Pause Gives the Camera Back"
---

# Tasks: The Focus Pause Gives the Camera Back

**Prerequisites** (plan.md § Prerequisites, each with its evidence there):
- every spec in `depends_on_landed` has landed on `main`: met at f6d4f5fd;
- `main` is still as read: not yet checked at a flip;
- the owner decisions in spec.md are settled: not met, none of the five answered;
- a cynical review and a probe round from this text alone, folded: not met;
- the page half has run once: met on the probe (`CI=1 npm run e2e`, 34 of 34);
- no operator-staged file is needed: met.

**Rules for every story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml`,
  `CODEOWNERS`, `ergane.yaml` or `.specify/`. If an existing test fails, stop and
  escalate with its exact output. A test file an earlier story of this spec created is
  an existing test. The only exceptions are each story's `mod` line in
  `crates/finder/tests/console/main.rs` or `crates/finder/tests/motion/main.rs`, in its
  alphabetical place.
- Write only the files your story's tasks name; they are its `writes` in plan.md
  § Refinement Record. A story that needs another file escalates.
- Never touch `crates/star-detection/`, `crates/scope-node/`, `crates/scope-link`,
  `proto/`, the plate solver's crates, the databases,
  `crates/finder/src/{model,goto,live,session}.rs`, `crates/finder/src/imaging/run.rs`,
  `crates/finder/src/imaging/astap.rs`, the MCP tools' descriptions, or the README's
  field checklist; add nothing under `crates/finder/tests/support/`. In
  `crates/simulator`, only US1's `imaging.rs` changes.
- `FocusCheck` and 037's five step traits gain nothing; `Kept::note` and
  `Kept::reference` keep their signatures.
- Copy the words of spec.md's FRs and of plan.md § Interfaces exactly, the README's
  among them. Say "finder camera", never "finder" alone, in every message and label.
- No test downloads anything, and no test writes outside `CARGO_TARGET_TMPDIR`. Run no
  tool the gates don't run, and do not set `CARGO_TARGET_DIR`.
- The results said here before each story's implementation are the probe's
  (plan.md § What was run and what was read), where it ran them.

## Phase 1: User Story 1 - Going on from a focus pause gives the camera back whole (Priority: P1)

### Tests first

- [ ] T001 [US1] Create `crates/finder/tests/console/imaging_run_focus_restore.rs` with
  one test for each of US1-S1, US1-S2 and US1-S3 under
  `#[tokio::test(start_paused = true)]`, over the pause rig as spec.md § User Scenarios
  defines it, with STILL and REFUSED written in the test, and one for US1-S4 under
  `#[test]`; and add `mod imaging_run_focus_restore;` to
  `crates/finder/tests/console/main.rs` in its alphabetical place. Before T002 the
  binary does not build (no `SimImagingCamera::with_abort_settle`); with T002 alone, all
  four tests fail (run).

### Implementation

- [ ] T002 [US1] In `crates/simulator/src/imaging.rs`, add `STILL_STOPPING`,
  `SimImagingCamera::with_abort_settle`, `State::refused_until` and the refusal in
  `apply` and the settle in `abort`, as plan.md § Interfaces gives them (FR-001).
- [ ] T003 [US1] In `crates/finder/src/imaging/focus/run.rs`, add `RESTORE_PAUSE`,
  `RESTORE_WAIT` and `put_back`, and call it from `finish` in place of its one
  `lease.set` (FR-002); in `crates/finder/src/imaging/run/console.rs`, give
  `ConsoleCamera::hold`'s region refusal its second form (FR-003), each as plan.md
  § Interfaces gives it.
- [ ] T004 [US1] Add US1's sentences to `crates/finder/README.md`'s `## Focus` and
  `## An imaging run`, in the words of plan.md § Interfaces (FR-004).
- [ ] T005 [US1] Run the `lint` and `test` gates with every existing test unchanged and
  green, then commit the green story (FR-001 to FR-004; US1-S1, US1-S2, US1-S3, US1-S4).

## Phase 2: User Story 2 - The focus record keeps the altitude it was read at (Priority: P1)

### Tests first

- [ ] T006 [US2] Create `crates/finder/tests/console/imaging_focus_carry.rs` with one
  test for each of US2-S1, US2-S2 and US2-S3 under `#[test]`, the airmass as the secant
  of the zenith distance and the exponent 0.6 written in the test, and add
  `mod imaging_focus_carry;` to `crates/finder/tests/console/main.rs` in its
  alphabetical place. Before T007 the binary does not build (no `focus::carry`,
  `Kept::note_at`, `Kept::reference_at`, `Kept::altitudes`; run).

### Implementation

- [ ] T007 [US2] In `crates/finder/src/imaging/focus.rs`, add `SEEING_EXPONENT`,
  `carry`, `Altitudes`, `Kept`'s two altitudes, `note_at` (with `note` calling it),
  `reference_at`, `altitudes`, and the optional `alt_deg` and `earlier_alt_deg` of
  `focus.json`, as plan.md § Interfaces gives them (FR-005 to FR-008).
- [ ] T008 [US2] Run the `lint` and `test` gates with every existing test unchanged and
  green, then commit the green story (FR-005 to FR-008; US2-S1, US2-S2, US2-S3).

## Phase 3: User Story 3 - The aid and the check judge at Track's altitude (Priority: P1)

### Tests first

- [ ] T009 [US3] Create `crates/finder/tests/motion/imaging_focus_altitude.rs` with one
  test for each of US3-S1 and US3-S2 under `#[tokio::test(start_paused = true)]`, over
  the motion rig as spec.md § User Scenarios defines it, with the record written by
  hand and the airmass and 0.6 written in the test, and one for US3-S3 under
  `#[test]`; and add `mod imaging_focus_altitude;` to
  `crates/finder/tests/motion/main.rs` in its alphabetical place. Before T010 and T011
  all three tests fail (run).

### Implementation

- [ ] T010 [US3] In `crates/finder/src/imaging/focus/run.rs`, add `altitude`, the
  carried reference in `note` with `Run::carried`, `note_at` with the reading's
  altitude, `SizeAt::alt_deg`, `ReferenceView::carried` and `CarriedView`, as plan.md
  § Interfaces gives them (FR-009 to FR-011).
- [ ] T011 [US3] Add US3's sentences to `crates/finder/README.md`'s `## Focus` and
  `## An imaging run`, in the words of plan.md § Interfaces (FR-012).
- [ ] T012 [US3] Run the `lint` and `test` gates with every existing test unchanged and
  green, then commit the green story (FR-009 to FR-012; US3-S1, US3-S2, US3-S3).

## Phase 4: User Story 4 - The Imaging page keeps its picture and readings, and names the altitudes (Priority: P2)

### Tests first

- [ ] T013 [US4] Create `crates/finder/web/src/components/FocusView.test.tsx` with one
  test for each of US4-S1, US4-S2, US4-S3 and US4-S4, over the page rig as spec.md § User Scenarios
  defines it, each locator held to its region. Before T014 and T015, US4-S1, US4-S2
  and US4-S4 fail and US4-S3 passes, its guard (run).

### Implementation

- [ ] T014 [US4] In `crates/finder/web/src/api.ts`, add `ImagingFocusSized.alt_deg`
  and the reference's `carried`; in `crates/finder/web/src/components/ImagingPage.tsx`,
  keep the last picture (FR-013), as plan.md § Interfaces gives them.
- [ ] T015 [US4] In `crates/finder/web/src/components/FocusBlock.tsx`, show the readings
  while the aid is `finding` with readings, and write the altitudes in the record and
  verdict lines (FR-014, FR-015), as plan.md § Interfaces gives them.
- [ ] T016 [US4] Rebuild `crates/finder/web/dist` with `npm run build`, run the `lint`
  and `test` gates with every existing test unchanged and green, then commit the green
  story with its `dist` (FR-013 to FR-015; US4-S1, US4-S2, US4-S3, US4-S4).

## Execution Order

```text
US1 -> US2 -> US3
US4
```

- **US1** starts from a `main` that holds every spec of `depends_on_landed`.
- **US2** starts once US1 has merged: the two stories' `mod` lines share
  `crates/finder/tests/console/main.rs`.
- **US3** starts once US2 has merged: it calls US2's `carry` and `note_at`, and writes
  `crates/finder/src/imaging/focus/run.rs` and the README after US1.
- **US4** shares no file with the others and calls none of their code; it may run
  beside them.
- **`[P]`** marks nothing here: each story's implementation follows its tests.
