# Implementation Plan: Target Search Finds Names of Several Words

**Branch**: `049-search-finds-whole-names` | **Date**: 2026-11-21 | **Spec**: [spec.md](spec.md)

This plan records every design decision for the feature. A story that would change one
escalates rather than re-deciding it.

## Summary

`Catalog::search` stops comparing the whole query with each word of a common name. A
private `match_words` makes the words of the query and of each name alike: lowercased,
without apostrophes (`'` and `’`), split at whitespace and hyphens. A common name
matches when each query word begins one of the name's words, in any order (spec owner
decisions 1 and 2), and a query with no word matches no name. A query of one word
matches at least the objects it did, and more only through a hyphen or an apostrophe in
the name or the query (`l'` finds what `l` finds); at the default limit of 20, `b` and
`p` each lose one of their first 20 to an object they gain. The exact id, the order by
magnitude and the limit are as they are. `targets::search`'s doc comment says the rule.
`GET /api/targets` and the MCP tool `search_targets` answer through the same function,
so both find "Orion Nebula" with no change of their own.

One story: the rule, `match_words`, its doc comment, nine unit tests after the last test
of the module that holds 012's search tests, and one test module at the MCP door that
asks the route and the tool alike.

## Technical Context

**Language/Version**: Rust, MSRV 1.96, stable in CI.

**Primary Dependencies**: none new. `Cargo.toml` and `Cargo.lock` do not change.

**Storage**: none.

**Testing**:
- The unit tests of `crates/finder/src/targets.rs` (US1-S1 to US1-S7, US1-S9 and
  US1-S10), nine new `#[test]` functions after the module's last one, with the extract
  the module's tests already read (`EXTRACT`, `crates/finder/src/targets.rs:239`).
  US1-S4 parses a catalog of its own with the private `Catalog::parse`, as 012's
  `exact_ids_precede_brighter_name_matches_and_nonexistent_rows_are_excluded`
  (`crates/finder/src/targets.rs:427`) does, so these are unit tests.
- `crates/finder/tests/mcp/target_words.rs` (new), one `#[tokio::test]` through
  `crate::support::mcp` (US1-S8), and `mod target_words;` in
  `crates/finder/tests/mcp/main.rs`.
- No Vitest or Playwright test: the page is unchanged.

**Target Platform**: the finder console, on the Pi and under `--simulate`.

**Constraints**:
- `targets::search`, `targets::find`, `targets::all`, `Target` and `SearchResult` keep
  their shapes.
- `RUSTFLAGS="-D warnings"`, `--all-features` clippy, and rustdoc with warnings denied.

**Scale/Scope**: one story; one changed function and doc comment, one new test module,
one `mod` line.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Nothing touches the solver, star detection or `crates/math-core`. The expectations are read from the pinned OpenNGC extract, the catalog's own source, and from 012's rule as its FR-007 words it. |
| II. Never weaken a gate | No existing test, golden, fixture, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS` or `ergane.yaml` is edited. The new unit tests are new functions after the module's last; 012's search tests and 016's search-tool test pass unchanged (§ Shapes that existing tests pin). |
| III. Tests first, independent expectations | Every expected list is written in the test from the extract's rows, or computed by the test from the extract's text (US1-S5 to US1-S7), US1-S6's lower bound by 012's documented rule and its gains written as literals, never by calling `search`; `targets::find` and `targets::all`, unchanged by FR-003, name the object a row or id stands for. The tool is held to the route's own answer. Seven of the ten scenarios fail on `main`'s code; the three that pass there pin what must not change (§ What was run and what was read). |
| IV. Critical paths | None is touched: no `Cargo.toml` or `Cargo.lock` change, no `unsafe`, nothing under `proto/`, no `build.rs`. |
| V. Proven open source first, honest licensing | Considered by name and not tried: a full-text index (`tantivy`), fuzzy matchers (`nucleo`, `fuzzy-matcher`) and edit distance (`strsim`). Each is a new dependency for a rule of a few lines over 182 names, and fuzzy matching would change what a word of one finds, against spec owner decision 1 (e). No reused code, no new license. |

## Project Structure

```text
crates/finder/
├── src/targets.rs                           # US1: the rule in Catalog::search; match_words; search's doc comment; nine unit tests
└── tests/mcp/
    ├── main.rs                              # US1: mod target_words;
    └── target_words.rs                      # US1, new
```

## Interface

### `crates/finder/src/targets.rs` (US1)

In `Catalog::search` (`crates/finder/src/targets.rs:164`), the filter
(`crates/finder/src/targets.rs:173` to `:184`) becomes:

```rust
        // Each query word must begin a word of the same common name, in any order.
        let terms = match_words(&query);
        let mut matches: Vec<_> = self
            .objects
            .iter()
            .filter(|object| {
                exact.is_some_and(|exact| std::ptr::eq(*object, exact))
                    || (!terms.is_empty()
                        && object.common_names.iter().any(|name| {
                            let words = match_words(name);
                            terms.iter().all(|term| {
                                words.iter().any(|word| word.starts_with(term.as_str()))
                            })
                        }))
            })
            .collect();
```

Before `fn catalog()` (`crates/finder/src/targets.rs:201`):

```rust
/// The words a common name or a query is matched by: lowercased, without apostrophes
/// (straight or curly), split at whitespace and hyphens.
fn match_words(text: &str) -> Vec<String> {
    text.to_lowercase()
        .replace(['\'', '\u{2019}'], "")
        .split(|c: char| c.is_whitespace() || c == '-')
        .filter(|word| !word.is_empty())
        .map(str::to_owned)
        .collect()
}
```

The trim, the lowercase, the empty and zero-limit answers, the exact id (looked up from
the trimmed, lowercased query, before `match_words`), the sort and the truncation are
unchanged. The doc comment of `pub fn search`
(`crates/finder/src/targets.rs:222` to `:225`) becomes:

```rust
/// Exact ids first, then common names in brightness order.
///
/// A common name matches when each word of the query begins one of its words, in any
/// order. Words are split at spaces and hyphens, and apostrophes and case are ignored.
/// Objects without a magnitude come last; canonical id breaks equal-magnitude ties. At
/// most `limit` objects are returned, with no duplicates.
```

The unit tests follow the module's last test,
`every_source_object_is_reachable_by_its_raw_and_canonical_id`
(`crates/finder/src/targets.rs:454`), with one helper of their
own that maps a search's objects to their ids. US1-S6's queries are built from the
extract's `common_names` column split on commas and spaces, as 012's rule reads words,
each word lowercased and cut after each of its characters; its oracle is 012's rule on
the extract's rows, as a lower bound, and the gains spec.md lists, written as literals
in a `match` on the query.

### `crates/finder/tests/mcp/target_words.rs` (US1)

A module comment naming its oracles (the extract's common names, with the ids written in
the test, and the route's JSON), then one `#[tokio::test]` that builds the console router
with `support::mcp::app(support::mcp::state(), "127.0.0.1")` and, for each query of
US1-S8, sends `GET /api/targets?q=` with the query encoded as spec.md gives it through
`support::mcp::send`, reads it with `support::mcp::body`, and sends
`support::mcp::call("search_targets", json!({"q": query}))`. It uses only
`crate::support::mcp`, `axum`, and `serde_json`.

### `crates/finder/tests/mcp/main.rs` (US1)

`mod target_words;` after `mod steering;` (`crates/finder/tests/mcp/main.rs:20`).

Nothing else changes.

## What was run and what was read

**Run on 2026-11-21**, on the branch `probe/049` cut from `main` at b7e983a0, three
times each with the same outcome unless said. The probe's tip, 1b0c4799, holds the ten
scenarios' tests as spec.md gives them and the trial fix of § Interface, for owner
decisions 1 (a) and 2 (b); its earlier commits, up to de91586c, held the draft's
decision 2 (a), whitespace words only, and eight scenarios.

- a first probe test on `main`'s code: `Orion Nebula`, `Great Orion Nebula`,
  `orion neb`, `Ring Nebula`, `Whirlpool Galaxy`, `Double Cluster` and
  `North America Nebula` each found nothing (all seven once; `Orion Nebula`, which the
  first version stopped at, twice);
- the tip's tests with `main`'s filter in place of the fix's: US1-S1, US1-S3, US1-S4,
  US1-S5, US1-S6, US1-S8 and US1-S9 fail; US1-S2, US1-S7 and US1-S10 pass, as they
  must, since `main` finds no name for any query of several words, finds an id with a
  space alone, and finds nothing for `-` or `'`; 012's ten unit tests of the module
  pass;
- the tip's tests with the fix: all ten pass, and the module's ten other unit tests
  pass unchanged;
- with the fix, both gates' Rust commands, once at 1b0c4799 (and before, under decision
  2 (a), at 40cd6f70, 0f28f0dd and de91586c): `lint`'s four and `test`'s two, every test
  of the workspace passing (1,433 under nextest at 1b0c4799), and the doctests;
- four mutations of the words at 1b0c4799, each once, with US1-S8: decision 2 (a)'s
  whitespace words (US1-S6, US1-S8 and US1-S9 fail); no guard for a query with no word
  (US1-S10 fails, alone); names split and stripped but not queries (US1-S5, US1-S6,
  US1-S8 and US1-S9 fail); only the straight apostrophe dropped (US1-S8 and US1-S9
  fail);
- six mutations of decision 1's rule, under decision 2 (a), each once: five at 40cd6f70
  on the unit tests only (so US1-S8 was not run under them), and one at 0f28f0dd with
  US1-S8. The words in the name's order (US1-S1 and US1-S3 fail), any query word rather
  than each (US1-S1 to US1-S4 and US1-S7 fail), a substring of the name (US1-S2,
  US1-S3, US1-S4, US1-S6 and 012's
  `search_uses_name_word_prefixes_and_orders_by_magnitude_with_unknowns_last` fail),
  the words next to each other in order (US1-S1 and US1-S3 fail), and the words from
  all of an object's names together (US1-S2 and US1-S3 fail), at 40cd6f70; and, at
  0f28f0dd, each query word beginning a different name word, owner decision 1 (f)
  (US1-S3's `orion o` fails, alone; the other 17 tests, US1-S8 among them, pass).

**Computed from the extract** on 2026-11-21, from the draft to the owner's answers, by a
model of the rules written apart from the code (the probe's notes):
- 182 common names on 13,372 objects, counted once per row (175 distinct); 951 distinct
  prefixes of their whitespace words; with whitespace words, under each of owner
  decision 1's (a), (b) and (c), every one of the 951 finds the same objects as today,
  and every full name finds its object;
- with decision 2 (b)'s words: every one of the 951 finds at least the objects it found
  with no limit, the 11 of US1-S6 find more, and 44 of the 917 distinct prefixes of the
  new words (every name's words as decision 2 (b) splits them), which found nothing, now
  find an object; queries outside both sets, such as `tie-` or `cats-eye`, can too;
- after the owner's answers, on 2026-11-21, the limits: at 20, `b` loses UGC 5373 and
  `p` NGC 6357 to a gain that sorts ahead; at 10, `b`, `l`, `m` and `t` lose one; at any
  limit, `l'`, `le` and `me` get a new first answer; from 28 on, none of the 951 loses
  one;
- with decision 2 (b)'s words, also: US1-S1 to US1-S5 and US1-S7 answer as with
  whitespace words; splitting names but not queries loses 25 full names; and, with no
  guard, `-` and `'` match all 152 named objects;
- none of the 13,372 canonical ids or the 110 Messier ids written with a space (13,482
  queries) finds any name under (a), (b) or (c), with either decision 2 answer's words;
- of the 312 queries made of two of a name's words in its order (285 distinct), (a)
  and (b) answer alike for all but `cluster 3` (spec owner decision 1 (b));
- under (f), each query word beginning a different name word, all 182 full names find
  what (a) finds, and `orion o` finds nothing where (a) finds NGC 1976 and IC 434; of
  the 1,999 prefixes of the 175 distinct names typed a character at a time, (a) and
  (f) differ on 3, `chi persei c`, `coalsack c` and `small s`, where (a) adds C 14,
  C 99 and NGC 292;
- under (d), `barbell cork` finds NGC 650 and `cluster s` adds NGC 6705;
- under (a), `C 30` and `C 3`, which are no ids of the catalog, find NGC 2070, and
  NGC 2070 and NGC 1049; none of the 651 duplicate aliases written with a space finds a
  name;
- 5 names hold a hyphen and 20 an apostrophe; no whitespace word of a name starts
  `tie` or `cats`.

**Read, not run**:
- that the page sends the trimmed query as typed, a space as `+`
  (`crates/finder/web/src/api.ts:914`,
  `crates/finder/web/src/components/TargetSheet.tsx:57`), and that axum's `Query`
  decodes `+` as a space (US1-S8 sends `+` through the router, which is run);
- that a query of several words narrows as words are added (spec § Edge Cases);
- the hyphen and apostrophe cases of spec § Edge Cases, from the extract and the rule.

**Not run at all**: the page, `finder --simulate`, the page half of the `test` gate
(`npm ci` installs packages, and the story writes nothing the page is built from), and
the factory's gate boundary itself.

## Read from the diff, not from a test

- **The doc comment** of `targets::search` says the rule (FR-002).
- **Nothing else changes** (FR-003): the diff holds the three files of § Project
  Structure, and in `targets.rs` only the filter, `match_words`, the doc comment and the
  new tests. `match_words` is private.

## Shapes that existing tests pin

Every existing test passes unchanged with the story built (run on the probe). The files
beside the change, and what the story does to what they observe:

- **`crates/finder/src/targets.rs`'s ten unit tests.**
  `search_uses_name_word_prefixes_and_orders_by_magnitude_with_unknowns_last`
  (`crates/finder/src/targets.rs:384`) searches single words only (`NEBULA`'s 60
  objects and their order, `cork`, `orion`, `romeda`, `qqq`, a blank query, a zero
  limit, `M31`, `ngc0651`); none of them is a word decision 2 changes (`NEBULA` still
  finds 60, NGC 3372 first): unchanged.
  `exact_ids_precede_brighter_name_matches_and_nonexistent_rows_are_excluded`
  (`crates/finder/src/targets.rs:427`) searches `m31`, `ic1` and `ngc5` on its own
  catalog: single words, unchanged. The other eight test `find`, `all` and the parse.
- **`crates/finder/tests/console/targets.rs`**, 012's route tests: `m31`, `ngc0224`,
  `andromeda`, `qqq`, `nebula` with its limits, a missing, empty or blank `q`, a
  malformed limit, and a search from an empty working directory. Single words and
  refusals: unchanged.
- **`crates/finder/tests/mcp/session_tools.rs`**:
  `catalog_search_passes_through_route_arrays_as_text_and_targets`
  (`crates/finder/tests/mcp/session_tools.rs:59`) holds the tool to the route for
  `orion`, `nebula` with its limits and `qqq`, and the blank refusal: unchanged.
  `no_landed_tool_result_or_call_log_contains_the_node_token`
  (`crates/finder/tests/mcp/motion.rs:517`) calls `search_targets` with `orion`
  (`:533`): unchanged.
- **`crates/finder/tests/console/pushto.rs`**:
  `search_classes_use_the_session_and_console_clock`
  (`crates/finder/tests/console/pushto.rs:142`) searches the route for Messier ids,
  single words: unchanged.
- **The page's tests.** `crates/finder/web/src/components/TargetSheet.test.tsx` and
  `App.target.test.tsx` answer the search from a mocked fetch; the e2e tests
  (`crates/finder/web/e2e/goto.spec.ts:62`, `crates/finder/web/e2e/session.spec.ts:27`)
  type Messier ids, single words. None changes.

## Decisions and the alternatives rejected

| Decision | Rejected | Why |
|---|---|---|
| Each query word begins a word of one name, any order | In order; next to each other; across names; substring or fuzzy; distinct words | Spec owner decision 1. |
| Names and queries alike lose `'` and `’` and split at whitespace and `-` | Leave them inside words; split names only | Spec owner decision 2 (b); `’` and the no-word guard are the author's reading of it. Splitting names only would lose 25 full names (US1-S5). |
| A query with no word matches no name | Let `all` over no words match | `-` or `'` would answer every one of the 152 named objects (US1-S10, run as a mutation). |
| One private `match_words` for names and queries | Words kept in `Target` at parse | A field would change the shape of the public `Target`, which § Technical Context's Constraints keep, and its JSON unless skipped; 182 names make a split per search cheap. |
| Change only `Catalog::search` | A second search for queries with a space; a change in `api::search` | One rule serves both doors and every query; for one word it is today's, but for hyphens and apostrophes. |
| Unit tests in the module of 012's search tests, after its last test | A `console/` module for them | US1-S4 needs the private `Catalog::parse`, and the rest sit with the tests of the same function. |
| One `mcp/` module that asks the route and the tool | A `console/` module for the route and an `mcp/` module for the tool | The MCP app answers both, as `session_tools.rs` already compares them; one module, one `mod` line. |
| A new module | New cases in `session_tools.rs` or `console/targets.rs` | Existing tests are not edited (constitution II). |
| 047 in `depends_on_landed` | `concurrent` with 047's US3; dropping US1-S8 so the story need not wait (spec owner decision 3) | 047's US3 and this story each add a line to `crates/finder/tests/mcp/main.rs`; 047 was dispatched before this draft. Without US1-S8 no test sends a query of several words through the route and the tool. 042 is named because its US10 last changed that file on `main`. |
| One story | Two | About 21 % of 65,536 B (§ Plan traps 2). |

## User Story 1 - A name of several words finds its object

Tests first: the nine unit tests in `targets.rs` and `tests/mcp/target_words.rs` with its
`mod` line. Then the filter, `match_words` and the doc comment in `targets.rs`.

## Verification (every story, before proposing)

- **The factory gates** (`ergane.yaml` at b7e983a0):
  - `lint` = `cargo fmt --all --check`,
    `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`,
    `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features --locked`
    and `RUSTDOCFLAGS="-D warnings" cargo doc -p finder --no-deps`;
  - `test` = `cargo nextest run --workspace --all-targets --all-features --locked`,
    `cargo test --workspace --locked --doc`, and then, in `crates/finder/web`,
    `npm ci --no-audit --no-fund`, `npm run typecheck`, `npm run format:check`,
    `npm test`, `npm run build`, `git status --porcelain -- dist | (! grep .)` and
    `CI=1 npm run e2e`.

  The story runs both, and the judge sees their results.
- **CI in the merge queue** runs every required `ci.yml` job.

## Prerequisites before this spec is flipped to `ready`

1. **The earlier specs have landed:** every spec in `depends_on_landed`. 012, 016, 027
   and 042 met at b7e983a0; 047 not met: it was building at b7e983a0.
2. **`main` is still as read.** Every citation here was read at b7e983a0 on 2026-11-21.
   047 changes `crates/finder/src/api.rs`, `crates/finder/tests/mcp/main.rs` (its
   `mod databases;` moves `mod steering;` down a line) and
   `crates/finder/web/src/api.ts`, so once it lands, re-read the citations of those
   three files, and of every file § Project Structure or § Shapes names, and re-run the
   validator. 047's US1 landed during the draft (d3f8bd0f) and moved `api.rs`'s lines;
   `targets.rs`, `support/` and `mcp/main.rs` were untouched by it. Once 047's US3 has
   landed, it, not 042's US10, last wrote `mcp/main.rs`: drop the 042 entry of
   § Refinement Record's `shared_writes`, and 042 from `depends_on_landed`.
3. **The owner decisions are settled and the owner approvals given** (spec § Owner
   decisions, § Owner approvals). Decisions met on 2026-11-21: the owner answered 1 (a),
   2 (b) and 3 (a) ("1. Agree 2. No hyphens or apostrophes require. 3. Ok"), and the
   trio is written for them. The approvals are given by the flip, which is the
   owner's.
4. **Probe builds.** Not met: no builder has built the story from this text alone. The
   drafting probe built it with the trial fix (§ What was run and what was read).

## Outside the factory

Written here and not run.

**A look at the page**, by the owner, once 049 has landed, on `finder --simulate`:

1. Open the Target search and type `Orion Nebula`: NGC 1976 (M42) should be the first
   row, and the only one.
2. Type `Ring Nebula`, `Whirlpool Galaxy` and `Double Cluster`: M57, M51 and C 14.
3. Ask MCP `search_targets` with `{"q":"Great Orion Nebula"}`: NGC 1976.

**The voice design's C5.** Once 049 has landed, C5's row in
`docs/voice/02-tool-contracts.md:183` describes a change that is done; marking it so is
the owner's.

**The Pi.** The console's binary changes, so a Pi kit built after 049 lands carries the
fix.

## Plan traps

They bind every story.

1. **Gate evidence or nothing.** The judge credits only gate results and the story's
   diff. *Check:* every Then names a committed test the `test` gate runs.
2. **One evidence budget.** *Check:* the story's diff stays under 64 KiB, measured with
   `git diff <base> -- . ':(exclude)crates/finder/web/dist' | wc -c`. Measured on the
   drafting probe's net diff from b7e983a0 at 1b0c4799: 13,614 B, 20.8 % of 65,536 B
   (`targets.rs` 11,176 B, the two `mcp/` files 2,438 B). At the 1.35 times a factory
   build has run over its probe on earlier specs, about 18,400 B, 28.0 %.
3. **A spec fixed mid-epic needs a relaunch.** *Check:* no builder edits
   `specs/049-search-finds-whole-names/` or `.specify/`.
4. **An oracle copied from the code proves nothing.** *Check:* every expected list is a
   literal in the test, or computed in the test from `EXTRACT`'s text, with
   `targets::find` or `targets::all` naming the object a row or id stands for; no test
   calls `search` or `Catalog::search` for an expected value.
5. **A negative assertion passes when nothing happens.** US1-S2, US1-S7 and US1-S10
   pass on `main`'s code. *Check:* US1-S1, US1-S3 to US1-S6, US1-S8 and US1-S9 assert
   objects that are found, and fail on `main`.
6. **Existing tests are not edited.** *Check:* the diff of `targets.rs` below
   `#[cfg(test)]` only adds lines after the module's last test; `session_tools.rs` and
   `console/targets.rs` are not in the diff.
7. **An unused helper fails clippy.** *Check:* the module's helper is used by the tests
   that follow it; `target_words.rs` holds no helper it does not call.
8. **rustdoc denies warnings.** *Check:* no doc comment holds an angle bracket outside
   backticks.
9. **Each worktree builds on its own.** *Check:* no story sets `CARGO_TARGET_DIR`.
10. **The lockfile does not move.** *Check:* every cargo command runs with `--locked`.
11. **The page is not rebuilt.** *Check:* nothing under `crates/finder/web/` is in the
    diff.

## Risks

| Risk | Mitigation |
|---|---|
| A new OpenNGC extract changes the counts and lists. | US1-S5 and US1-S6 assert their counts, 182 and 951, so a new extract fails them by name; re-read the lists from it. |
| A keyboard types an apostrophe or a dash the rule does not know, such as `ʼ` (U+02BC) or an en dash. | Only `'`, `’` and `-` are handled (spec § Edge Cases): another mark stays part of a word, and such a query finds what it would today. Voice mode may add more. |
| A one-word answer surprises an observer, such as `mel` finding PGC 143, or `b` and `p` each losing one of their first 20 at the default limit. | Owner approval 2 names the change; US1-S6 lists every gain over today's 951 prefixes; the 44 of the 917 prefixes of the new words that now find an object, `mel` among them, and the limit effects were computed; of the 44 only `tie` is tested (US1-S9). |
| An id typed with a space gains a name match on a later extract. | US1-S7 fails by that id; US1-S4 holds that the exact id still comes first. |

## Refinement Record

Drafted on 2026-11-21 by a plate-solver spec session against `main` at b7e983a0, from a
probe of the fault and a trial fix that day, a reading of `crates/finder/src/targets.rs`,
its callers in `api.rs` and `mcp/tools.rs`, their tests, 012's and 016's trios, the
voice design's C5 and the constitution. Writes come from the task text. Shared writes
name the merge edge that orders them, and the story that last wrote the same file on
`main`, or that will once its epic has landed.

The owner answered spec.md's three owner decisions on 2026-11-21: 1 (a), 2 (b), 3 (a).
The record's `open_questions` are empty. This record approves nothing: the owner
approvals of spec.md are given by the flip.

```yaml
version: 1
reviewed_revision: b7e983a031b6be86f894c1bbdff2ee3b04ba6cd3
stories:
  US1:
    tests:
      - path: crates/finder/src/targets.rs
        status: new
        disposition: add
        assertion: "with a limit of 20, Orion Nebula, Great Orion Nebula, orion neb, ring NEBULA, Whirlpool Galaxy, Double Cluster, h & chi Persei, North America Nebula, spaced-out Ring Nebula, nebula orion, gr or ne and whirl gal give exactly NGC 1976, NGC 1976, NGC 1976, NGC 6720, NGC 5194, C 14, C 14, NGC 7000, NGC 6720, NGC 1976, NGC 1976 and NGC 5194"
        decision: "spec.md:US1-S1"
      - path: crates/finder/src/targets.rs
        status: new
        disposition: add
        assertion: "orion galaxy, andromeda nebula, barbell cork, rion nebula and orion nebulae each find nothing"
        decision: "spec.md:US1-S2"
      - path: crates/finder/src/targets.rs
        status: new
        disposition: add
        assertion: "cluster s gives Cl 399, NGC 6087, Mel 111; little nebula gives NGC 6818, NGC 650, NGC 6369, and with a limit of 2 NGC 6818, NGC 650; eagle nebula gives IC 4703, NGC 6611; orion o gives NGC 1976, IC 434"
        decision: "spec.md:US1-S3"
      - path: crates/finder/src/targets.rs
        status: new
        disposition: add
        assertion: "on a parsed catalog of five rows where NGC 1 is M31 at magnitude 15, M 31 gives exactly NGC 1, NGC 2 (M 31 Bright Galaxy, 1), NGC 3 (Galaxy M 31, no magnitude), not NGC 4 (Medium M31 Galaxy) or NGC 5 (M 3 Galaxy)"
        decision: "spec.md:US1-S4"
      - path: crates/finder/src/targets.rs
        status: new
        disposition: add
        assertion: "each of the extract's 182 common names counted per row, in capitals, with no limit, finds the object find gives for its row"
        decision: "spec.md:US1-S5"
      - path: crates/finder/src/targets.rs
        status: new
        disposition: add
        assertion: "with no limit, each of the 951 distinct lowercased prefixes of the extract's common-name words, split at spaces and commas, finds at least the rows with a common name having a whitespace-separated word that starts with it, and beyond those exactly: b and bu NGC 3132; fi ESO 270-017; l, m and me PGC 143; l' ESO 56-115, NGC 1980, IC 2944, NGC 6523, NGC 6618, NGC 6818, NGC 650, UGC 5470, PGC 143, NGC 6445, NGC 6369; l'e UGC 5470; le NGC 6705; p NGC 2537; t NGC 40; nothing for the rest"
        decision: "spec.md:US1-S6"
      - path: crates/finder/src/targets.rs
        status: new
        disposition: add
        assertion: "every canonical id, and every Messier id written M then a space then its number, finds exactly its object"
        decision: "spec.md:US1-S7"
      - path: crates/finder/tests/mcp/target_words.rs
        status: new
        disposition: add
        assertion: "on the console router, GET /api/targets with Orion+Nebula, Great+Orion+Nebula, Ring+Nebula, Whirlpool+Galaxy, Double+Cluster and Cat%E2%80%99s+Eye answers 200 with exactly NGC 1976, NGC 1976, NGC 6720, NGC 5194, C 14 and NGC 6543; search_targets with each query is not an error, its structuredContent is targets set to the route's answer, and its one text block is JSON equal to it"
        decision: "spec.md:US1-S8"
      - path: crates/finder/src/targets.rs
        status: new
        disposition: add
        assertion: "with a limit of 20, bow tie, Bow-Tie and tie give exactly NGC 40; cats eye, cat's eye and cat with U+2019 s eye give exactly NGC 6543; bear paw NGC 2537; eight burst NGC 3132; Wolf Lundmark Melotte and Wolf-Lundmark-Melotte PGC 143; stephans quintet HCG 92"
        decision: "spec.md:US1-S9"
      - path: crates/finder/src/targets.rs
        status: new
        disposition: add
        assertion: "-, ', --, U+2019 alone and ' - each find nothing"
        decision: "spec.md:US1-S10"
      - path: crates/finder/src/targets.rs
        status: existing
        disposition: preserve
        assertion: "NEBULA finds 60 objects brightest first with unknowns last; cork, orion, M31 and ngc0651 find their objects; romeda, qqq and a blank query find nothing; exact ids precede brighter name matches"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/console/targets.rs
        status: existing
        disposition: preserve
        assertion: "m31, ngc0224 and andromeda start with NGC 224; qqq is an empty list; a missing, empty or blank q and a malformed limit are 400; nebula's limits are 20, 1 and 50"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/mcp/session_tools.rs
        status: existing
        disposition: preserve
        assertion: "search_targets equals GET /api/targets for orion, nebula with its limits and qqq, and refuses a blank q with the route's text"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/mcp/motion.rs
        status: existing
        disposition: preserve
        assertion: "calling every listed tool, search_targets with orion among them, leaves the node token out of every tool result and the call log"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/console/pushto.rs
        status: existing
        disposition: preserve
        assertion: "m42, m8 and m75 at the route answer first with their Messier object, classed pointable, below_horizon and sun_guard, then too_low and no_site as the session changes"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "a query of one word matches at least the objects it matched before, and more only through a hyphen or an apostrophe in the name or the query (owner decision 2), before the limit is applied; the order is 012 FR-007's"
      - "exact ids first, then magnitude with no magnitude last, then canonical id; limit defaults to 20 and is clamped to 1 to 50; an empty q is 400"
      - "targets::find, targets::all, the catalog data, api.rs, mcp/ and the page are unchanged"
    checks:
      development:
        - "cargo nextest run -p finder --locked --lib targets::"
        - "cargo nextest run -p finder --locked --test mcp target_words::"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs and the look at the page of plan.md § Outside the factory"
    writes:
      - crates/finder/src/targets.rs
      - crates/finder/tests/mcp/main.rs
      - crates/finder/tests/mcp/target_words.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/targets.rs
        other_spec: 012-targets-and-push-to
        other_story: US5
        disposition: merge_order
        reason: "049 depends_on_landed 012, whose US5 last changed targets.rs (918b3d48)"
      - path: crates/finder/tests/mcp/main.rs
        other_spec: 042-imaging-run-page-and-mcp
        other_story: US10
        disposition: merge_order
        reason: "049 depends_on_landed 042, whose US10 last changed mcp/main.rs (c5f30038)"
      - path: crates/finder/tests/mcp/main.rs
        other_spec: 047-console-uses-the-database-that-fits
        other_story: US3
        disposition: merge_order
        reason: "049 depends_on_landed 047, whose US3 adds mod databases to mcp/main.rs"
    open_questions: []
```
