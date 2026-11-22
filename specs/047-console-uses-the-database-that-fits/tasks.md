---
description: "Task list for 047-console-uses-the-database-that-fits: The Console Uses the Database That Fits"
---

# Tasks: The Console Uses the Database That Fits

**Prerequisites** (plan.md § Prerequisites):
- every spec in `depends_on_landed` has landed on `main`: met at 425b9da0;
- the owner decisions in spec.md are settled: met on 2026-11-21, the recommended answers;
- every story has been probe-built from this text: not met;
- the page half has run once: not met;
- no operator-staged file is needed.

**Rules for every story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml`,
  `CODEOWNERS`, `ergane.yaml` or `.specify/`. If an existing test fails, stop and
  escalate with its exact output. A test file an earlier story of this spec created is
  an existing test; the one exception is US2's `mod field;` line in US1's
  `crates/finder/tests/console/databases/mod.rs`.
- Write only the files your story's tasks name; they are its `writes` in plan.md
  § Refinement Record. A story that needs another file escalates.
- `crates/pattern-database`, `crates/database-generation`, `crates/plate-solver`,
  `crates/simulator`, `crates/scope-node`, `crates/scope-link`, `proto/` and every file
  under `crates/finder/src/imaging/` are never touched, and nothing is added under
  `crates/finder/tests/support/`.
- Copy the words of spec.md's FRs and the README section of plan.md § Interfaces exactly.
- No test downloads anything, and no test writes outside `CARGO_TARGET_TMPDIR`. Run no
  tool the gates don't run, and do not set `CARGO_TARGET_DIR`.

## Phase 1: User Story 1 - The console holds several databases and solves uploads with each (Priority: P1)

### Tests first

- [ ] T001 [US1] Add `database-generation = { version = "0.1.0", path = "../database-generation" }`
  under `[dev-dependencies]` in `crates/finder/Cargo.toml` (FR-019), and let cargo add it
  to the finder's list in `Cargo.lock`; nothing else in the lockfile may change.
- [ ] T002 [US1] Create `crates/finder/tests/console/databases/mod.rs` with the test
  databases of plan.md § The test databases (`NARROW`, `MID`, `WIDE`, `databases`) and
  `mod several;`, and add `mod databases;` to `crates/finder/tests/console/main.rs` in
  its alphabetical place.
- [ ] T003 [US1] Create `crates/finder/tests/console/databases/several.rs` with one test
  for each of US1-S4, US1-S5, US1-S6 and US1-S7, under `#[tokio::test]`. They call
  `AppState::with_databases` and read `database` and `databases`, which T005 to T007 add.
- [ ] T004 [US1] Create `crates/finder/tests/cli/databases.rs` with the binary's helper
  (start it with the scenario's `--db` paths, read its start lines and `GET /api/health`)
  and one test for each of US1-S1, US1-S2 and US1-S3, under `#[test]`, and add
  `mod databases;` to `crates/finder/tests/cli/main.rs` in its alphabetical place. By
  reading, not run, today's binary refuses a second `--db` (clap), so US1-S1 and US1-S2
  fail on it.

### Implementation

- [ ] T005 [US1] Create `crates/finder/src/databases.rs` with `Database`, `Reason`'s
  `Only` and `First`, `DatabaseView`, `InUseView`, `Databases` with `new`, `one`,
  `first`, `list`, `views`, `in_use` and `solve_frame` (FR-005, FR-006; plan.md
  § Interfaces), and add `pub mod databases;` and the health line to
  `crates/finder/src/lib.rs`.
- [ ] T006 [US1] Add `SolveReport::database` in `crates/finder/src/solve.rs` (FR-005).
- [ ] T007 [US1] In `crates/finder/src/api.rs`, add `AppState`'s `databases`,
  `with_databases`, the health's `databases` and `solve_upload`'s tries (FR-004, FR-005,
  FR-006, FR-007, FR-008; plan.md § Interfaces).
- [ ] T008 [US1] In `crates/finder/src/main.rs`, make `--db` repeatable, load and name
  every database, print the start lines and build `Databases` (FR-001, FR-002, FR-003).
- [ ] T009 [US1] Run the `lint` and `test` gates with every existing test unchanged and
  green, then commit the green story (FR-001 to FR-008, FR-019, FR-020; US1-S1, US1-S2,
  US1-S3, US1-S4, US1-S5, US1-S6, US1-S7).

## Phase 2: User Story 2 - The live loop uses the database whose range holds the field (Priority: P1)

### Tests first

- [ ] T010 [US2] Create `crates/finder/tests/console/databases/field.rs` with the live
  rig of spec.md § User Scenarios and one test for each of US2-S1, US2-S2, US2-S3,
  US2-S4, US2-S5, US2-S6, US2-S7, US2-S8 and US2-S9, under
  `#[tokio::test(start_paused = true)]`, and add `mod field;` to
  `crates/finder/tests/console/databases/mod.rs`, its one changed line. They read the
  session's `field.database`, which T012 adds; until then they fail.

### Implementation

- [ ] T011 [US2] In `crates/finder/src/databases.rs`, add `Reason`'s `Solved` and
  `Field`, the estimate in `order`, `solved`, `solve_centroids` and `DatabasesSolver`
  (FR-009, FR-010; plan.md § Interfaces).
- [ ] T012 [US2] In `crates/finder/src/live.rs`, add `Range` and
  `Live::start_with_databases`, keeping `Live::start`; in `crates/finder/src/session.rs`,
  add `FieldView`'s and `LiveField`'s `database` and `databases`, `update` and
  `use_database`; in `crates/finder/src/api.rs`, make `with_live` and `with_live_solver`
  use them (FR-011, FR-012).
- [ ] T013 [US2] Run the `lint` and `test` gates with every existing test unchanged and
  green, US1's test files among them, then commit the green story (FR-009 to FR-012;
  US2-S1, US2-S2, US2-S3, US2-S4, US2-S5, US2-S6, US2-S7, US2-S8, US2-S9).

## Phase 3: User Story 3 - The page, MCP and the README say which database is in use (Priority: P2)

### Tests first

- [ ] T014 [US3] Create `crates/finder/web/src/components/FieldDatabases.test.tsx` with
  one test for each of US3-S1, US3-S2, US3-S3, US3-S4 and US3-S5, and
  `crates/finder/web/src/healthLine.test.ts` with one test for each of US3-S6 and US3-S7.
- [ ] T015 [US3] Create `crates/finder/tests/mcp/databases.rs` with one test for each of
  US3-S8, under `#[tokio::test(start_paused = true)]`, and US3-S9, under
  `#[tokio::test]`, and add `mod databases;` to `crates/finder/tests/mcp/main.rs` in its
  alphabetical place. They pass on arrival with US2 merged.

### Implementation

- [ ] T016 [US3] In `crates/finder/web/src/`, add the types to `api.ts`, create
  `healthLine.ts`, call it from `App.tsx`'s footer, and add the lines and alerts to
  `components/FieldWarning.tsx` (FR-013, FR-014, FR-015; plan.md § Interfaces). Run
  `npm run build` and commit `crates/finder/web/dist` (FR-017).
- [ ] T017 [US3] In `crates/finder/README.md`, change the `--db` and `GET /api/health`
  rows and add `## Several databases` before `## API`, from plan.md § Interfaces
  (FR-018).
- [ ] T018 [US3] Run the `lint` and `test` gates with every existing test unchanged and
  green, US1's and US2's test files among them, then commit the green story (FR-013 to
  FR-018; US3-S1, US3-S2, US3-S3, US3-S4, US3-S5, US3-S6, US3-S7, US3-S8, US3-S9).

## Execution Order

```text
US1 -> US2 -> US3
```

- **US1** starts from a `main` that holds every spec of `depends_on_landed`.
- **US2** starts once US1 has merged: it adds the choice to US1's module and its test
  module to US1's.
- **US3** starts once US2 has merged: the page and MCP read the keys US2 adds.
- **`[P]`** marks nothing here: each story's implementation follows its tests.
