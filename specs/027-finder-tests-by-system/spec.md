---
state: draft
depends_on_landed: ["012-targets-and-push-to", "013-live-finder", "014-goto", "015-tracking", "016-mcp", "017-console-stop-confirm", "018-live-chart-steady", "019-night-legibility", "020-live-chart-calm"]
# Drafted 2026-11-14 against `main` at 870b332 with the gate change of pull request 214
# applied, from the test-layer audit the owner asked for that day. Both stories were built
# in a throwaway worktree before this was written (plan.md § Refinement Record). It stays
# draft until the owner flips it to `ready` on Tharpebox, which gives the approvals listed
# under "Owner approvals" and dispatches it. The owner asked for it to run before 024, 025
# and 026, so that those are written against the layout it leaves.
---

# Feature Specification: Finder's Tests, Organized by System

**Feature Branch**: `027-finder-tests-by-system`

**Created**: 2026-11-14

**Status**: Draft

**Input**: A read-only audit of the test layer on 2026-11-14 found that `crates/finder`
holds 49 integration-test files, each its own test binary, filed one per spec story.
Six files pull in `crates/finder/tests/goto_api.rs` with a path attribute to reach its
simulator rig, and its six tests come along: they run in seven binaries. One file does the
same with `crates/finder/tests/backlash_sim.rs` and re-runs its five tests. Each binary
was about 289 MB in a default test build. The owner asked for the tests to be organized by
system. This spec moves finder's integration tests into four binaries by system, puts the
shared helpers in one folder that holds no test, and adds a test that keeps it so. It
changes where tests live. It changes no assertion.

## Clarifications

### Session 2026-11-14 (review of `main` at 870b332, and the probe)

- Q: What does this trio read besides itself? → A: Nothing. `spec.md` holds every
  requirement and scenario, and `plan.md` holds every design decision, the move of each
  file, the text of each header, the guard test and the alternatives rejected. A story
  that would change a decision escalates rather than re-deciding it.
- Q: What was found? → A: Four things, each read at 870b332 and built in the probe:
  1. Every top-level `.rs` file under `crates/finder/tests` is its own test binary:
     49 binaries for 261 tests.
  2. `crates/finder/tests/goto_api.rs` holds a simulator rig and six tests. The rig ends
     and the tests begin at `crates/finder/tests/goto_api.rs:328`. Six files include the
     whole file for the rig (for example `crates/finder/tests/goto_sim.rs:2` and
     `crates/finder/tests/track_api.rs:3`), so each of the six tests is compiled and run
     in seven binaries: 36 extra runs.
  3. `crates/finder/tests/backlash_sim.rs` holds a tracker rig and five tests, which begin
     at `crates/finder/tests/backlash_sim.rs:335`. `crates/finder/tests/backlash_cli.rs:3`
     includes the whole file: 5 extra runs.
  4. The helper files `crates/finder/tests/live/support.rs`,
     `crates/finder/tests/mcp/support.rs` and `crates/finder/tests/stop/support.rs` hold
     no tests and are included by 14, 6 and 3 files.
- Q: Why now? → A: Until pull request 214 the gate ran test binaries one after another,
  and the tests were the slow part. With that changed, compiling and linking 50 finder
  binaries is the larger share of a cold `test` gate on GitHub (253 s of 553 s on
  2026-11-14), and 024, 025 and 026 are each about to add test files in the old layout.
- Q: What does a story's evidence look like, when the story moves tests? → A: Two kinds,
  both in the diff. A committed test, `crates/finder/tests/layout/main.rs`, reads the
  source tree and asserts the rule; it fails before the story and passes after. And the
  diff itself: a moved file is a rename, and the judge can read that the only lines
  changed in it are the ones FR-003 and FR-007 allow.
- Q: Does a moved file stay a rename in the factory's diff? → A: Yes. The probe's US2
  moves 52 files; the factory's output check measured its diff at 39,683 bytes. US1
  measured 56,437 bytes. Both are under the 65,536-byte preferred size.
- Q: Does merging binaries change how a test behaves? → A: Two tests re-run their own
  binary with `--exact` and their bare name (`crates/finder/tests/targets_api.rs:114`
  and `crates/finder/tests/session_api.rs:178`). In a merged binary the bare name
  matches nothing, the child runs no test and exits 0, and the parent passes without
  having tested anything. FR-008 fixes both. No other test was found to depend on its
  binary: in the probe all 350 finder tests passed under `cargo nextest` (one process a
  test) and under `cargo test` (one process a binary).
- Q: Why four binaries, and why these? → A: By what a test needs to run. `motion` drives
  the simulated mount in paused time. `console` calls the router in process and moves no
  mount. `mcp` goes through the MCP door. `cli` starts the built `finder` binary. A
  system inside a binary is a folder or a file (plan.md § Move map).
- Q: How many tests are there afterwards? → A: The same 261 finder integration tests,
  each compiled once, plus three in `layout`. The workspace's `test` gate ran 659 tests
  at 870b332; after US1 it runs 620 (41 extra runs gone, 2 added) and after US2 621.

## Owner approvals

Flipping this spec to `ready` approves the following. The constitution's rule II lets a
story edit an existing test only where its spec names the edit for the owner's approval;
these are those edits.

1. **Existing test files are moved and their header lines edited**, as FR-002, FR-003,
   FR-005, FR-006 and FR-007 say and no further. No assertion is changed or removed.
2. **Forty-one test runs stop.** The six tests of `goto_api.rs` stop running in
   `goto_refusal_api`, `goto_sim`, `model_api`, `second_sync_sim`, `track_api` and
   `track_min_alt`, and the five of `backlash_sim.rs` stop running in `backlash_cli`.
   Each of the eleven still runs once.
3. **Test binary names change.** Finder's 49 integration-test binaries become five:
   `cli`, `console`, `layout`, `mcp` and `motion`. A command in a landed spec that names
   an old binary, such as `cargo test -p finder --locked --test tracking_sim`, no longer
   resolves; plan.md § Move map gives the new filter for each.
4. **Two test bodies are edited** (FR-008): the two tests that re-run their own binary
   pass their full name and gain one assertion, that the child ran exactly one test.
5. **Two fixture recorders are edited** (FR-010, FR-011):
   `crates/finder/web/src/test/fixtures/record-sessions.rs` and
   `crates/finder/web/src/test/fixtures/record-tracking.rs`. Nothing compiles them; they
   are copied into `crates/finder/tests` by hand to re-record the page's fixtures.
6. **A new test binary, `layout`**, with three tests that read the source tree (FR-004,
   FR-009). After this spec, a new test file placed at the top level of
   `crates/finder/tests` fails it.
7. **Adding a finder test will mean editing a shared file.** Today a story adds a test by
   adding a file and touching nothing else. After this spec it adds a module file under
   one of the four binaries and one `mod` line to that binary's `main.rs` or to a
   folder's `mod.rs`, a file other stories also edit. Two stories that add neighbouring
   lines to the same file conflict unless one waits for the other to merge (plan.md
   § Risks).

## User Scenarios & Testing *(mandatory)*

Both stories' scenarios run in
`cargo nextest run --workspace --all-targets --all-features --locked`, the first command
of the `test` gate, which runs `crates/finder/tests/layout/main.rs` and every moved test.
A scenario that begins "Given the story's diff" is read from the diff.

### User Story 1 - A test runs in one binary (Priority: P1)

As the maintainer, I have every finder test compiled into one test binary, so a gate does
not spend time running the same test seven times and a failure is reported once.

**Why this priority**: It removes 41 runs for a small change, and US2 needs the helpers
out of the two test files before it can move them.

**Independent Test**:
`cargo nextest run --workspace --all-targets --all-features --locked` runs
`crates/finder/tests/layout/main.rs`.

**Acceptance Scenarios**:

1. **Given** the files under `crates/finder/tests`, **When** a committed test reads every
   `.rs` file there and follows each `#[path = "…"]` attribute that starts a line,
   **Then** it asserts that no included file has a line starting `#[test]` or
   `#[tokio::test`, that it read more than 40 files, and that it followed at least one
   attribute.
2. **Given** `crates/finder/tests/support`, **When** a committed test reads it, **Then**
   it asserts that `goto.rs` and `tracker.rs` are there and that no file under it has a
   line starting `#[test]` or `#[tokio::test`.
3. **Given** the story's diff, **When** it is read, **Then** in `goto_refusal_api.rs`,
   `goto_sim.rs`, `model_api.rs`, `second_sync_sim.rs`, `track_api.rs`,
   `track_min_alt.rs` and `backlash_cli.rs` the only changed line is the path attribute,
   which names `support/goto.rs` or `support/tracker.rs`.
4. **Given** the story's diff, **When** it is read, **Then** `goto_api.rs` keeps its six
   test functions and `backlash_sim.rs` its five, and no line inside any test function
   is changed, added or removed.

---

### User Story 2 - Finder's tests are grouped by system (Priority: P2)

As the maintainer, I find a test by the system it tests: the mount's motion, the console's
routes, the MCP door or the command line. A new story adds a module to one of the four
binaries and no longer adds a binary.

**Why this priority**: It is the organization the owner asked for, and it takes finder
from 49 test binaries to five.

**Independent Test**:
`cargo nextest run --workspace --all-targets --all-features --locked` runs
`crates/finder/tests/layout/main.rs`, `crates/finder/tests/console/targets.rs` and
`crates/finder/tests/console/session/edits.rs`.

**Acceptance Scenarios**:

1. **Given** `crates/finder/tests`, **When** a committed test lists it, **Then** it
   asserts that every entry is a directory and that the entries are exactly `cli`,
   `console`, `layout`, `mcp`, `motion` and `support`.
2. **Given** `crates/finder/tests/support`, **When** a committed test lists it, **Then**
   it asserts that its entries are exactly `goto.rs`, `live.rs`, `mcp.rs`, `mod.rs`,
   `stop.rs` and `tracker.rs`.
3. **Given** `main.rs` in each of `cli`, `console`, `mcp` and `motion`, **When** a
   committed test reads its lines that are neither blank nor comments, **Then** it
   asserts that the first three are `#[allow(dead_code, unused_imports)]`,
   `#[path = "../support/mod.rs"]` and `mod support;`, and that every other line is one
   `mod name;` declaration, with at least one.
4. **Given** the `console` binary, **When**
   `targets::search_works_from_an_empty_working_directory` re-runs the binary, **Then**
   the committed test passes the child `--exact` with its own full name, built from
   `module_path!()`, and asserts that the child's output contains
   `test result: ok. 1 passed`.
5. **Given** the `console` binary, **When**
   `session::edits::library_without_a_state_file_leaves_home_and_xdg_empty` re-runs the
   binary, **Then** the committed test does the same and asserts the same.
6. **Given** the story's diff, **When** it is read, **Then** each of the 49 test files
   and the three helper files of plan.md § Move map is a rename to the path the map
   gives, and the only changed lines in a renamed file are those FR-006, FR-007 and
   FR-008 name.
7. **Given** US1's two committed tests in `crates/finder/tests/layout/main.rs`, **When**
   the story's diff is read, **Then** neither is edited.

---

### Edge Cases

- **A helper that includes a helper.** After US1, `support/goto.rs` and
  `support/tracker.rs` include `live/support.rs` by path. That is allowed: the rule is
  about files that hold tests.
- **The same helper compiled into several binaries.** Each of the four binaries compiles
  the whole `support` folder. Items a binary does not use are allowed for by the
  attribute on its `mod support;` line.
- **A file that needs a path relative to itself.** `cli.rs` reads
  `THIRD_PARTY_NOTICES.md` with `include_str!`; one folder deeper it needs one more
  `../` (FR-007).
- **A test file added at the top level later.** US2's test fails, naming the file.
- **The recorders.** Copying one into `crates/finder/tests` to re-record puts a file at
  the top level, and US2's test fails until it is removed again. That is the recorders'
  own instruction: remove the temporary file.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: A test function under `crates/finder/tests` SHALL be compiled into exactly
  one test binary. No `#[path]` attribute SHALL name a file that holds a test.
- **FR-002**: The helpers of `goto_api.rs` (everything above the comment that begins
  `// Every negative motion assertion`) SHALL move to
  `crates/finder/tests/support/goto.rs`, and the helpers of `backlash_sim.rs` (everything
  above its first test) to `crates/finder/tests/support/tracker.rs`, changed only as
  plan.md § Interfaces "US1" says: the path of the helper they include, a re-export of
  three names, `pub` on their items, and one `#![allow(dead_code)]`. The tests stay in
  `goto_api.rs` and `backlash_sim.rs` under the headers that section gives.
- **FR-003**: The six files that include `goto_api.rs` SHALL include `support/goto.rs`
  instead, and `backlash_cli.rs` SHALL include `support/tracker.rs`, by changing the
  path attribute and nothing else.
- **FR-004**: `crates/finder/tests/layout/main.rs` SHALL hold the two tests of US1-S1 and
  US1-S2, as plan.md § Interfaces "the guard" gives them.
- **FR-005**: Finder's integration tests SHALL be four binaries, `motion`, `console`,
  `mcp` and `cli`, each a folder under `crates/finder/tests` whose `main.rs` holds a doc
  comment, the support include and module declarations only. Every test file SHALL move
  to the path plan.md § Move map gives.
- **FR-006**: `live/support.rs`, `mcp/support.rs` and `stop/support.rs` SHALL move
  unchanged to `support/live.rs`, `support/mcp.rs` and `support/stop.rs`.
  `support/mod.rs` SHALL declare the five helper modules. `support/goto.rs` and
  `support/tracker.rs` SHALL reach the live helpers with
  `pub use super::live as support;`.
- **FR-007**: In a moved test file the only edits SHALL be: each
  `#[path = "…"]` and `mod name;` pair replaced by `use crate::support::helper as name;`,
  and one more `../` in a path given to `include_str!`. Nothing else in a moved file
  changes, except as FR-008 says.
- **FR-008**: The two tests that re-run their own binary SHALL pass `--exact` their full
  name, built with `module_path!()`, and SHALL assert that the child's output contains
  `test result: ok. 1 passed`.
- **FR-009**: `crates/finder/tests/layout/main.rs` SHALL gain the test of US2-S1, US2-S2
  and US2-S3, as plan.md § Interfaces "the guard" gives it. US1's two tests SHALL stay
  as they are.
- **FR-010**: After US1 the two fixture recorders SHALL include `support/goto.rs`.
- **FR-011**: After US2 the two fixture recorders SHALL include `support/mod.rs` and
  bind `support::goto` as `api`.

### Key Entities

- **Test binary**: what cargo builds from one top-level entry of a crate's `tests`
  folder: a `.rs` file, or a folder with a `main.rs`.
- **Helper**: test support code with no test in it: a rig, a clock, a request function.
- **Guard**: the `layout` binary's tests, which read the source tree and assert where
  tests live.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario above is asserted by a committed test or can be
  read from the story's diff. The `lint` and `test` gates pass.
- **SC-002**: No file outside `crates/finder/tests` is edited except the two fixture
  recorders. No `Cargo.toml`, `Cargo.lock`, file under `crates/finder/src`, golden,
  `scripts/ci/` file, workflow, `CODEOWNERS` entry or `ergane.yaml` is edited.
- **SC-003**: No test function is added or removed except the three in `layout`, and no
  line inside a test function changes except in the two tests FR-008 names.
- **SC-004**: After US2 the `test` gate's first command reports 621 tests where it
  reported 659 at 870b332, and finder builds five integration-test binaries where it
  built 49. This is checked by the owner from the gate's log, not by a gate.

## Assumptions

- **Earlier features are in place.** 012 through 020 have landed, and pull request 214
  has merged: the `test` gate's first command is the nextest line above. Every citation
  here was read at 870b332.
- **Out of scope:**
  - the other crates' tests, and the page's Vitest and Playwright tests;
  - merging the three simulator rigs, or the four copies of the database fixture, into
    one;
  - moving a test into `crates/finder/src`, splitting a test file by subject, or
    renaming a test function;
  - any test's assertions, simulated span or frame rate, including the three longest
    tests;
  - the constitution and the spec templates, which the maintainer amends in a pull
    request of their own once this has landed, so that later specs add a module to one
    of these binaries and name it as `--test binary module::`;
  - the text of landed specs, which keep the commands they were built with;
  - the drafts of 024, 025 and 026, which the operator refreshes against this layout
    after it lands.
- **The factory's gates and CI:** the gates are `lint` and `test`, copied in plan.md
  § Verification. CI's other jobs run only in the merge queue.

## Work Graph

```yaml
US1:
  depends_on: []
  implements: [FR-001, FR-002, FR-003, FR-004, FR-010]
US2:
  depends_on: []
  depends_on_merged: [US1]
  implements: [FR-005, FR-006, FR-007, FR-008, FR-009, FR-011]
```

US2 moves every file US1 edits, so it waits for US1 to merge.
