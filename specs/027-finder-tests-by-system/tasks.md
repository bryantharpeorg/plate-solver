---
description: "Task list for 027-finder-tests-by-system: Finder's Tests, Organized by System"
---

# Tasks: Finder's Tests, Organized by System

**Prerequisites** (plan.md § Prerequisites):
- `012-targets-and-push-to` through `020-live-chart-calm` have landed on `main`;
- pull request 214 has merged, so the `test` gate's first command is
  `cargo nextest run --workspace --all-targets --all-features --locked`;
- the owner approvals in spec.md are given;
- no operator-staged file is needed.

**Rules for every story:**
- This spec moves existing test files and edits their header lines. Make only the edits
  plan.md § Interfaces names. Change no line inside a test function, except in the two
  tests FR-008 names. Add no test and remove none, except the three in `layout`.
- Move a file with `git mv`, so the diff shows a rename. Change nothing by hand that
  `cargo fmt --all` does not change for you.
- Write only the files your story's tasks name; they are its `writes` in plan.md
  § Refinement Record. A story that needs another file escalates.
- Never edit a `Cargo.toml`, `Cargo.lock`, a file under `crates/finder/src`, a golden,
  a fixture's data, `scripts/ci/`, `.github/`, `deny.toml`, `CODEOWNERS` or
  `ergane.yaml`. If the build needs one of them changed, stop and escalate with the
  exact output.
- Run no tool the gates don't run, except `cargo fmt --all` to apply the formatting that
  `cargo fmt --all --check` checks, and `git mv`. No `cargo deny`, `cargo machete`,
  Rust 1.96 or `ergane`.

## Phase 1: User Story 1 — A test runs in one binary (Priority: P1)

### Tests first

- [ ] T001 [US1] Create `crates/finder/tests/layout/main.rs` with the two tests
  `no_file_with_tests_is_included_by_another` (US1-S1) and
  `support_holds_helpers_and_no_tests` (US1-S2) and their helpers, exactly as plan.md
  § Interfaces "The guard" gives them down to and including the second test. Both fail
  before the next tasks.

### Implementation

- [ ] T002 [US1] Create `crates/finder/tests/support/goto.rs` from lines 1 to 327 of
  `crates/finder/tests/goto_api.rs`, with the five changes of plan.md § Interfaces "US1:
  the two helper files", and replace those lines in `crates/finder/tests/goto_api.rs`
  with the header that section gives, leaving its six tests unchanged (FR-002, US1-S4).
- [ ] T003 [US1] Create `crates/finder/tests/support/tracker.rs` from lines 1 to 334 of
  `crates/finder/tests/backlash_sim.rs` in the same way, and replace those lines in
  `crates/finder/tests/backlash_sim.rs` with its header, leaving its five tests
  unchanged (FR-002, US1-S4).
- [ ] T004 [US1] Change the path attribute, and nothing else, in
  `crates/finder/tests/goto_refusal_api.rs`, `crates/finder/tests/goto_sim.rs`,
  `crates/finder/tests/model_api.rs`, `crates/finder/tests/second_sync_sim.rs`,
  `crates/finder/tests/track_api.rs` and `crates/finder/tests/track_min_alt.rs` to
  `#[path = "support/goto.rs"]`, and in `crates/finder/tests/backlash_cli.rs` to
  `#[path = "support/tracker.rs"]` (FR-001, FR-003, US1-S3).
- [ ] T005 [US1] Change `#[path = "goto_api.rs"]` to `#[path = "support/goto.rs"]` in
  `crates/finder/web/src/test/fixtures/record-sessions.rs` and
  `crates/finder/web/src/test/fixtures/record-tracking.rs` (FR-010).
- [ ] T006 [US1] Run `cargo fmt --all`, then the `lint` and `test` gates, with every
  test function unchanged and green. Commit the green story.

## Phase 2: User Story 2 — Finder's tests are grouped by system (Priority: P2)

### Tests first

- [ ] T007 [US2] Append `code_lines` and `tests_are_grouped_into_one_binary_per_system`
  (US2-S1, US2-S2, US2-S3) to `crates/finder/tests/layout/main.rs`, exactly as plan.md
  § Interfaces "The guard" gives them, editing nothing above them (US2-S7). The new test fails
  before the next tasks.

### Implementation

- [ ] T008 [US2] `git mv` `crates/finder/tests/live/support.rs`,
  `crates/finder/tests/mcp/support.rs` and `crates/finder/tests/stop/support.rs` to
  `crates/finder/tests/support/live.rs`, `crates/finder/tests/support/mcp.rs` and
  `crates/finder/tests/support/stop.rs`, unchanged. Create
  `crates/finder/tests/support/mod.rs`. In `crates/finder/tests/support/goto.rs` and
  `crates/finder/tests/support/tracker.rs` replace the include of the live helpers with
  `pub use super::live as support;` (FR-006).
- [ ] T009 [US2] `git mv` each of the 49 test files under `crates/finder/tests` to the
  path plan.md § Move map gives, and in each make the edits of plan.md § Interfaces
  "US2: the edits inside a moved test file" and no others (FR-005, FR-007, US2-S6).
- [ ] T010 [US2] Create `main.rs` in `crates/finder/tests/motion`,
  `crates/finder/tests/console`, `crates/finder/tests/mcp` and `crates/finder/tests/cli`,
  and `mod.rs` in `motion/goto`, `motion/model`, `motion/stop`, `motion/tracking`,
  `console/live` and `console/session`, as plan.md § Interfaces "US2: the four binaries"
  says (FR-005).
- [ ] T011 [US2] In `crates/finder/tests/console/targets.rs` and
  `crates/finder/tests/console/session/edits.rs`, make the test that re-runs its own
  binary pass its full name and assert that the child ran one test, as plan.md
  § Interfaces "US2: the two tests that re-run their own binary" gives it (FR-008, US2-S4,
  US2-S5).
- [ ] T012 [US2] In `crates/finder/web/src/test/fixtures/record-sessions.rs` and
  `crates/finder/web/src/test/fixtures/record-tracking.rs`, include `support/mod.rs` and
  bind `support::goto` as `api`, as plan.md § Interfaces "US2: the two recorders" gives
  it (FR-011).
- [ ] T013 [US2] Run `cargo fmt --all`, then the `lint` and `test` gates, with every
  test function unchanged and green except the two of T011. Check that
  `git diff -M --name-status` against the story's base shows a rename for each of the 52
  moved files. Commit the green story.

## Execution Order

```text
US1 → US2 (US2 waits for US1 to merge)
```

- **US1** touches `goto_api.rs`, `backlash_sim.rs`, the seven files that include them,
  two new helper files, the new `layout` binary and the two recorders.
- **US2** moves every file under `crates/finder/tests`, including the ones US1 edited,
  so it waits for US1's merge.
