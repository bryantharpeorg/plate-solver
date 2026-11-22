---
description: "Task list for 049-search-finds-whole-names: Target Search Finds Names of Several Words"
---

# Tasks: Target Search Finds Names of Several Words

**Prerequisites** (plan.md § Prerequisites):
- every spec in `depends_on_landed` has landed on `main`: 012, 016, 027 and 047, met at
  28f436df;
- the owner decisions in spec.md are settled: met, 1 (a) and 3 (a) on 2026-11-21 and
  decision 2's rule on 2026-11-22;
- the story has been probe-built from this text: met on 2026-11-22, in probe round 1;
- no operator-staged file is needed.

**Rules for every story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml`,
  `CODEOWNERS`, `ergane.yaml` or `.specify/`. If an existing test fails, stop and
  escalate with its exact output. The unit tests already in
  `crates/finder/src/targets.rs` are existing tests: add after them, change none.
- Write only the files your story's tasks name; they are its `writes` in plan.md
  § Refinement Record. A story that needs another file escalates.
- `crates/finder/src/api.rs`, `crates/finder/src/mcp/`, `crates/finder/data/`,
  `crates/finder/web/`, every other crate, `proto/`, `Cargo.toml` and `Cargo.lock` are
  never touched.
- Copy the empty-query check, the filter, `words`, `name_matches` and the doc comment
  from plan.md § Interface.
- Run no tool the gates don't run, and do not set `CARGO_TARGET_DIR`.

## Phase 1: User Story 1 - A name of several words finds its object (Priority: P1)

### Tests first

- [ ] T001 [US1] In `crates/finder/src/targets.rs`, after the tests module's last test,
  `every_source_object_is_reachable_by_its_raw_and_canonical_id`, add the one helper
  plan.md § Interface names, `ids`, and one `#[test]` for each of US1-S1 to US1-S7,
  US1-S9 and US1-S10, with the queries and expected ids spec.md gives. US1-S6 keeps the
  prefixes that hold a letter or a digit by its own check, not by calling `words`. On
  today's search US1-S1, US1-S3, US1-S4, US1-S5, US1-S6, US1-S9 and US1-S10 fail, and
  US1-S2 and US1-S7 pass, as the probe saw.
- [ ] T002 [US1] Create `crates/finder/tests/mcp/target_words.rs` with one
  `#[tokio::test]` for US1-S8 through `crate::support::mcp`, and add
  `mod target_words;` after `mod steering;` in `crates/finder/tests/mcp/main.rs`. On
  today's search it fails: the route answers an empty list for `Orion+Nebula`.

### Implementation

- [ ] T003 [US1] In `crates/finder/src/targets.rs`, replace `Catalog::search`'s
  empty-query check and filter and `targets::search`'s doc comment with plan.md
  § Interface's, and add `words` and `name_matches` before `fn catalog()` (FR-001,
  FR-002). Nothing else in the file's code changes.
- [ ] T004 [US1] Run the `lint` and `test` gates with every existing test unchanged and
  green, 012's search tests and `crates/finder/tests/mcp/session_tools.rs` among them,
  then commit the green story (FR-001 to FR-003; US1-S1, US1-S2, US1-S3, US1-S4,
  US1-S5, US1-S6, US1-S7, US1-S8, US1-S9, US1-S10).

## Execution Order

```text
US1
```

- **US1** starts from a `main` that holds every spec of `depends_on_landed`.
- **`[P]`** marks nothing here: the implementation follows the tests.
