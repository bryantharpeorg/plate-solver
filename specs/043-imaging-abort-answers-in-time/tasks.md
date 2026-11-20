---
description: "Task list for 043-imaging-abort-answers-in-time: An Imaging Abort Answers in Time"
---

# Tasks: An Imaging Abort Answers in Time

**Prerequisites** (plan.md § Prerequisites):
- every spec in `depends_on_landed` has landed on `main`: met at fc161a54;
- 040 has landed and is named in `depends_on_landed`, and every citation has been re-read
  on that `main`: not yet met, 040 is being built;
- the owner decisions in spec.md are settled: not yet met, both are open;
- no operator-staged file is needed.

**Rules for the story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml`,
  `CODEOWNERS`, `ergane.yaml` or `.specify/`. If an existing test fails, stop and
  escalate with its exact output. The one exception is T001's `mod` line (spec.md owner
  approval 1).
- Write only the files the tasks name; they are the story's `writes` in plan.md
  § Refinement Record. A story that needs another file escalates.
- The scope node, `crates/scope-link`, `crates/simulator`, `crates/finder/web`, and every
  file under `crates/finder/src` but `crates/finder/src/imaging/mod.rs` are never
  touched. No caller of `Imaging::abort` changes (plan.md § Plan traps 14).
- `calibration_console.rs` and `stack_loop.rs` are used through their `pub(crate)` items
  and never edited.
- Copy the README's three places and the doc comment from plan.md § Interfaces; no test
  reads them.
- No test moves real hardware. Run no tool the gates don't run, and do not set
  `CARGO_TARGET_DIR`.

## Phase 1: User Story 1 - An abort answers whether or not the node does, and so does every stop that awaits it (Priority: P1)

### Tests first

- [ ] T001 [US1] Create `crates/finder/tests/console/imaging_abort_silent.rs` with the
  silent rig and the slow rig as spec.md § User Scenarios defines them, `SlowAbort` and
  its node clock among them, and one test for each of US1-S1, US1-S2, US1-S3, US1-S4,
  US1-S5, US1-S6, US1-S7, US1-S8 and US1-S9. Add `mod imaging_abort_silent;` after
  `mod imaging;` in `crates/finder/tests/console/main.rs`. On today's `Imaging::abort`
  seven of the nine fail, as the probe saw: US1-S1, US1-S3, US1-S6, US1-S8 and US1-S9
  with no answer within 60 s, US1-S4 answering after 8 s and US1-S7 after 28.9 s;
  US1-S2 and US1-S5 pass before the fix and after.

### Implementation

- [ ] T002 [US1] In `crates/finder/src/imaging/mod.rs`, add `ABORT_WAIT` after
  `SET_WAIT`, bound the `Exposing` branch of `Imaging::abort` with it, and add the
  sentence to its doc comment (plan.md § Interfaces). No other line of the file changes.
- [ ] T003 [US1] In `crates/finder/README.md`, write the three places of plan.md
  § Interfaces: the row of `POST /api/imaging/abort`, the row of
  `DELETE /api/imaging/stack` and the paragraph on the abort (FR-005).
- [ ] T004 [US1] Run the `lint` and `test` gates with every existing test unchanged and
  green, then commit the green story (FR-001 to FR-005; US1-S1, US1-S2, US1-S3, US1-S4,
  US1-S5, US1-S6, US1-S7, US1-S8, US1-S9).

## Execution Order

```text
US1
```

- **US1** starts from a `main` that holds every spec of `depends_on_landed`.
- **`[P]`** marks nothing here: T002 and T003 follow T001's tests.
